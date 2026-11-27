//! Kinetic Neural Canvas UI — MindCorpus 3D Brain, Kinetic Pill Dock, Living Bubble Head & CAD Studio.

use crate::brain::{BrainActivity, BrainAtlas};
use crate::icons::{paint_icon, IconKind};
use crate::overlays::{set_pet_mood_hint, OverlayChild};
use crate::studio::{parse_studio_synthesis, RpgState, StudioPane, StudioSource, StudioState};
use base64::Engine;
use eframe::egui::{self, Align2, Color32, Frame, Pos2, Rect, Stroke};
use nephron_bridge::{
    format_citations_prompt, BackendKind, CadPart, ChatMessage, ChatRole, CynapseClient,
    EffortLevel, MemoryRecord, MemoryStore, ModelProfile, SearchResult, StreamEvent,
    SystemTelemetry, WebSearcher,
};
use nephron_cad::{
    cad_repair_prompt, compile_cad_bundle, parse_cad_bundle, sanitize_part_name, CadDesign,
    CadViewport, CvGestureTracker, StlMesh, CAD_SYSTEM_PROMPT,
};
use nephron_pets::PetSpecies;
use nephron_swarm::actions::{execute_action, ComputerAction};
use nephron_voice::{
    audio::SAMPLE_RATE, encode_pcm16_wav, transcribe_wav_local,
    tts::{DEFAULT_TTS_URL, DEFAULT_VOICE, FEMALE_VOICES},
    wav_to_base64, AudioCapture, PocketTtsClient, PocketTtsSidecar, SentenceBuffer, SpeechPipeline,
    VadState,
};
use std::path::PathBuf;
use std::time::{Duration, Instant};
use tokio::runtime::Runtime;
use tokio::sync::mpsc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DrawerTab {
    Bridge,
    Memory,
    Cad,
    Agents,
    Cortex,
}

pub struct NephronApp {
    pub(crate) rt: Runtime,
    pub(crate) client: CynapseClient,
    pub(crate) backend_kind: BackendKind,
    pub(crate) ollama_models: Vec<String>,
    pub(crate) selected_ollama: String,
    ollama_rx: Option<mpsc::UnboundedReceiver<Vec<String>>>,
    pub(crate) store: MemoryStore,
    pub(crate) model: ModelProfile,
    pub(crate) effort: EffortLevel,
    active_tab: Option<DrawerTab>,
    bubble_open: bool,
    pub(crate) web_mode: bool,
    pub(crate) inject_pinned: bool,
    pub(crate) auto_vad: bool,
    vad_was_speech: bool,
    pet_active: bool,
    swarm_active: bool,
    pill_x: Option<f32>,
    pill_vx: f32,
    messages: Vec<ChatMessage>,
    input: String,
    memory_query: String,
    pub(crate) memory_hits: Vec<MemoryRecord>,
    streaming: bool,
    stream_rx: Option<mpsc::UnboundedReceiver<StreamEvent>>,
    health_rx: Option<mpsc::UnboundedReceiver<bool>>,
    daemon_online: bool,
    last_tok_per_sec: f32,
    telemetry: SystemTelemetry,
    last_poll: Instant,
    _tts_sidecar: PocketTtsSidecar,
    pub(crate) speech: SpeechPipeline,
    sentence_buf: SentenceBuffer,
    pub(crate) tts_voice: String,
    pub(crate) auto_tts: bool,
    cmd_palette_open: bool,
    cmd_query: String,
    last_export_path: Option<PathBuf>,
    capture: Option<AudioCapture>,
    pet_species: PetSpecies,
    last_pet_mood: &'static str,
    pet_proc: OverlayChild,
    swarm_proc: OverlayChild,
    cad_viewport: CadViewport,
    cad_part_name: String,
    cad_scad: String,
    cad_design: Option<CadDesign>,
    cad_parts: Vec<CadPart>,
    cad_rx: Option<mpsc::UnboundedReceiver<StreamEvent>>,
    cad_buf: String,
    cad_repairs: u8,
    cad_prompt: String,
    cad_explode: f32,
    cad_history: Vec<CadDesign>,
    cad_hist_idx: usize,
    cv_gesture: Option<CvGestureTracker>,
    pub(crate) pomo_end: Option<Instant>,
    pub(crate) show_hw_hud: bool,
    pub(crate) brain: BrainAtlas,
    pub(crate) searcher: WebSearcher,
    web_sources: Vec<SearchResult>,
    web_rx: Option<mpsc::UnboundedReceiver<(String, Vec<SearchResult>)>>,
    stt_rx: Option<mpsc::UnboundedReceiver<(Option<String>, Vec<u8>)>>,
    ui_time: f32,
    pub(crate) studio: StudioState,
    pub(crate) rpg: RpgState,
    pub(crate) studio_rx: Option<mpsc::UnboundedReceiver<StreamEvent>>,
    pub(crate) studio_buf: String,
    pub(crate) studio_web_rx: Option<mpsc::UnboundedReceiver<(String, Vec<SearchResult>)>>,
    pub(crate) studio_file_rx: Option<mpsc::UnboundedReceiver<Option<PathBuf>>>,
}

impl NephronApp {
    pub fn new(rt: Runtime, client: CynapseClient, store: MemoryStore) -> Self {
        let saved_backend = store.get_setting("backend");
        let backend_kind = if saved_backend.as_deref() == Some("ollama") {
            BackendKind::Ollama
        } else {
            BackendKind::Cynapse
        };
        let client = if backend_kind == BackendKind::Ollama {
            CynapseClient::with_kind(BackendKind::Ollama.default_url(), BackendKind::Ollama).unwrap_or(client)
        } else {
            client
        };
        let selected_ollama = store.get_setting("ollama_model").unwrap_or_else(|| "llama3.2".into());
        let tts_voice = store.get_setting("tts_voice").unwrap_or_else(|| DEFAULT_VOICE.into());
        let auto_tts = store.get_setting("auto_tts").map(|v| v != "0").unwrap_or(true);
        let auto_vad = store.get_setting("auto_vad").map(|v| v != "0").unwrap_or(true);
        let inject_pinned = store.get_setting("inject_pinned").map(|v| v != "0").unwrap_or(true);
        let tts_url = std::env::var("NEPHRON_TTS_URL").unwrap_or_else(|_| DEFAULT_TTS_URL.into());
        let _tts_sidecar = PocketTtsSidecar::spawn(8000, &tts_voice);
        let tts = PocketTtsClient::new(tts_url).expect("valid pocket-tts url");
        let speech = SpeechPipeline::spawn(tts, tts_voice.clone());
        let cad_parts = store.list_parts().unwrap_or_default();
        let cad_history: Vec<CadDesign> = store
            .list_cad_revisions(16)
            .into_iter()
            .filter_map(|(_, j)| serde_json::from_str::<CadDesign>(&j).ok())
            .collect();
        let memories = store.recent_memories(40).unwrap_or_default();
        let mut brain = BrainAtlas::default();
        if let Some(p) = store.get_setting("palette_idx").and_then(|s| s.parse().ok()) {
            brain.palette_idx = p;
        }
        brain.build_anatomy(&memories);
        let studio: StudioState = store
            .get_setting("studio_state")
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
        let rpg: RpgState = store
            .get_setting("rpg_state")
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
        let mut app = Self {
            rt,
            client,
            backend_kind,
            ollama_models: Vec::new(),
            selected_ollama,
            ollama_rx: None,
            store,
            model: ModelProfile::MiniCpm5_1B,
            effort: EffortLevel::Auto,
            active_tab: None,
            bubble_open: true,
            web_mode: false,
            inject_pinned,
            auto_vad,
            vad_was_speech: false,
            pet_active: false,
            swarm_active: false,
            pill_x: None,
            pill_vx: 0.0,
            messages: vec![ChatMessage {
                role: ChatRole::Assistant,
                content: "Nephron neural workspace ready.".into(),
            }],
            input: String::new(),
            memory_query: String::new(),
            memory_hits: memories,
            streaming: false,
            stream_rx: None,
            health_rx: None,
            daemon_online: false,
            last_tok_per_sec: 0.0,
            telemetry: SystemTelemetry::sample(),
            last_poll: Instant::now() - Duration::from_secs(10),
            _tts_sidecar,
            speech,
            sentence_buf: SentenceBuffer::default(),
            tts_voice,
            auto_tts,
            cmd_palette_open: false,
            cmd_query: String::new(),
            last_export_path: None,
            capture: None,
            pet_species: PetSpecies::Chiba,
            last_pet_mood: "",
            pet_proc: OverlayChild::default(),
            swarm_proc: OverlayChild::default(),
            cad_viewport: CadViewport::default(),
            cad_part_name: "opi6_enclosure".into(),
            cad_scad: "cube([85, 56, 22], center=true);".into(),
            cad_design: None,
            cad_parts,
            cad_rx: None,
            cad_buf: String::new(),
            cad_repairs: 0,
            cad_prompt: String::new(),
            cad_explode: 0.0,
            cad_history,
            cad_hist_idx: 0,
            cv_gesture: None,
            pomo_end: None,
            show_hw_hud: false,
            brain,
            searcher: WebSearcher::default(),
            web_sources: Vec::new(),
            web_rx: None,
            stt_rx: None,
            ui_time: 0.0,
            studio,
            rpg,
            studio_rx: None,
            studio_buf: String::new(),
            studio_web_rx: None,
            studio_file_rx: None,
        };
        app.poll_system();
        if app.backend_kind == BackendKind::Ollama {
            app.refresh_ollama();
        }
        set_pet_mood_hint("idle");
        app
    }

    pub(crate) fn save_settings(&self) {
        let _ = self.store.set_setting(
            "backend",
            match self.backend_kind {
                BackendKind::Cynapse => "cynapse",
                BackendKind::Ollama => "ollama",
            },
        );
        let _ = self.store.set_setting("ollama_model", &self.selected_ollama);
        let _ = self.store.set_setting("tts_voice", &self.tts_voice);
        let _ = self.store.set_setting("auto_tts", if self.auto_tts { "1" } else { "0" });
        let _ = self.store.set_setting("auto_vad", if self.auto_vad { "1" } else { "0" });
        let _ = self.store.set_setting("inject_pinned", if self.inject_pinned { "1" } else { "0" });
        let _ = self.store.set_setting("palette_idx", &self.brain.palette_idx.to_string());
    }

    pub(crate) fn switch_backend(&mut self, kind: BackendKind) {
        self.backend_kind = kind;
        if let Ok(c) = CynapseClient::with_kind(kind.default_url(), kind) {
            self.client = c;
            self.poll_system();
            if kind == BackendKind::Ollama {
                self.refresh_ollama();
            }
            if self.pet_active {
                self.pet_proc.kill();
            }
            self.save_settings();
        }
    }

    pub(crate) fn refresh_ollama(&mut self) {
        let client = self.client.clone();
        let (tx, rx) = mpsc::unbounded_channel();
        self.ollama_rx = Some(rx);
        self.rt.spawn(async move {
            let _ = tx.send(client.list_ollama_models().await);
        });
    }

    pub(crate) fn active_model_id(&self) -> String {
        match self.backend_kind {
            BackendKind::Cynapse => self.model.id().to_string(),
            BackendKind::Ollama => self.selected_ollama.clone(),
        }
    }

    fn poll_system(&mut self) {
        self.telemetry = SystemTelemetry::sample();
        let client = self.client.clone();
        let (tx, rx) = mpsc::unbounded_channel();
        self.health_rx = Some(rx);
        self.rt.spawn(async move {
            let _ = tx.send(client.health().await);
        });
        self.last_poll = Instant::now();
    }

    pub(crate) fn sync_brain(&mut self) {
        let mems = self.store.recent_memories(40).unwrap_or_default();
        self.brain.build_anatomy(&mems);
        self.memory_hits = mems;
    }

    fn capture_and_describe_screen(&mut self, question: String) {
        if self.streaming {
            return;
        }
        let shot = std::env::temp_dir().join(format!("nephron_vision_{}.png", std::process::id()));
        let cap_ok = execute_action(&ComputerAction::CaptureScreen(shot.clone())).is_ok();
        let bytes = if cap_ok { std::fs::read(&shot).ok() } else { None };
        // Privacy-first: delete local screenshot immediately after reading into memory
        let _ = std::fs::remove_file(&shot);
        self.bubble_open = true;
        self.messages.push(ChatMessage { role: ChatRole::User, content: question.clone() });
        let Some(png) = bytes else {
            self.messages.push(ChatMessage {
                role: ChatRole::Assistant,
                content: "Screen capture failed (install gnome-screenshot or xfce4-screenshooter).".into(),
            });
            return;
        };
        let b64 = base64::engine::general_purpose::STANDARD.encode(&png);
        self.messages.push(ChatMessage { role: ChatRole::Assistant, content: String::new() });
        let (tx, rx) = mpsc::unbounded_channel();
        self.stream_rx = Some(rx);
        self.streaming = true;
        let client = self.client.clone();
        let model_id = self.active_model_id();
        self.rt.spawn(async move {
            client.stream_vision_prompt_model(&model_id, &question, b64, tx).await;
        });
    }

    fn run_cua_workflow(&mut self) {
        self.bubble_open = true;
        let steps: [(&str, ComputerAction); 3] = [
            ("Git Status", ComputerAction::AllowlistedCheck { bin: "git", args: vec!["status".into(), "-s".into()] }),
            ("Memory", ComputerAction::AllowlistedCheck { bin: "free", args: vec!["-h".into()] }),
            ("Notify", ComputerAction::Notify { title: "Nephron CUA".into(), body: "Autonomous workflow checks complete.".into() }),
        ];
        let mut log = Vec::new();
        for (label, act) in steps {
            let res = execute_action(&act).unwrap_or_else(|e| format!("err: {e}"));
            log.push(format!("• {label}: {res}"));
        }
        self.messages.push(ChatMessage {
            role: ChatRole::Assistant,
            content: format!("CUA Workflow complete:\n{}", log.join("\n")),
        });
    }

    fn send_prompt(&mut self) {
        let prompt = self.input.trim().to_string();
        if prompt.is_empty() || self.streaming {
            return;
        }
        self.input.clear();
        self.dispatch_prompt(prompt);
    }

    fn dispatch_prompt(&mut self, prompt: String) {
        let lower = prompt.to_lowercase();
        if lower.contains("on my screen") || lower.contains("look at my screen") || lower == "screen" {
            self.capture_and_describe_screen(prompt);
            return;
        }
        if lower.starts_with("workflow") {
            self.run_cua_workflow();
            return;
        }
        if lower.starts_with("cua:") || lower == "open terminal" || lower == "take screenshot" {
            self.bubble_open = true;
            let res = if lower.contains("terminal") {
                execute_action(&ComputerAction::LaunchApp("gnome-terminal".into()))
            } else if lower.contains("screenshot") {
                execute_action(&ComputerAction::CaptureScreen(std::env::temp_dir().join("nephron_cua.png")))
            } else {
                execute_action(&ComputerAction::Notify {
                    title: "Nephron CUA".into(),
                    body: prompt.trim_start_matches("cua:").trim().into(),
                })
            };
            let msg = res.unwrap_or_else(|e| format!("CUA error: {e}"));
            self.messages.push(ChatMessage { role: ChatRole::User, content: prompt });
            self.messages.push(ChatMessage { role: ChatRole::Assistant, content: msg });
            return;
        }
        let web_intent = self.web_mode
            || lower.starts_with("search ")
            || lower.starts_with("research ")
            || lower.starts_with("look up ")
            || lower.contains("latest news");
        let cad_intent = !web_intent
            && ((self.active_tab == Some(DrawerTab::Cad))
                || lower.contains("build ")
                || lower.contains("design ")
                || lower.contains("cad")
                || lower.contains("3d model")
                || lower.contains("3d print")
                || lower.contains("make a 3d")
                || lower.contains("create a 3d"));
        if cad_intent {
            self.active_tab = Some(DrawerTab::Cad);
            self.bubble_open = true;
            let _ = self.store.save_memory("cad", &prompt);
            self.sync_brain();
            self.messages.push(ChatMessage { role: ChatRole::User, content: prompt.clone() });
            self.generate_cad(prompt);
            return;
        }
        let _ = self.store.save_memory("user", &prompt);
        self.sync_brain();
        self.bubble_open = true;
        self.messages.push(ChatMessage { role: ChatRole::User, content: prompt.clone() });
        self.messages.push(ChatMessage { role: ChatRole::Assistant, content: String::new() });

        if web_intent {
            if let Some(last) = self.messages.last_mut() {
                last.content = "Searching the web...".into();
            }
            let (wtx, wrx) = mpsc::unbounded_channel();
            self.web_rx = Some(wrx);
            self.streaming = true;
            let searcher = self.searcher.clone();
            self.rt.spawn(async move {
                let hits = searcher.search(&prompt, 5).await.unwrap_or_default();
                let _ = wtx.send((prompt, hits));
            });
            return;
        }
        self.start_chat_stream(None);
    }

    fn start_chat_stream(&mut self, web_ctx: Option<String>) {
        let (tx, rx) = mpsc::unbounded_channel();
        self.stream_rx = Some(rx);
        self.streaming = true;
        let client = self.client.clone();
        let model_id = self.active_model_id();
        let effort = self.effort;
        let mut history = self.messages[..self.messages.len() - 1].to_vec();
        if self.inject_pinned {
            let pinned_lines: Vec<String> = self
                .memory_hits
                .iter()
                .filter(|m| m.pinned)
                .take(6)
                .map(|m| format!("- [{}] {}", m.kind, m.content))
                .collect();
            if !pinned_lines.is_empty() {
                history.insert(
                    0,
                    ChatMessage {
                        role: ChatRole::System,
                        content: format!("Pinned user memory:\n{}", pinned_lines.join("\n")),
                    },
                );
            }
        }
        if let Some(c) = web_ctx {
            history.insert(history.len().saturating_sub(1), ChatMessage { role: ChatRole::System, content: c });
        }
        self.rt.spawn(async move {
            client.stream_chat_model(&model_id, effort, history, tx).await;
        });
    }

    fn toggle_mic(&mut self) {
        if let Some(cap) = self.capture.take() {
            self.vad_was_speech = false;
            let samples = cap.stop();
            if !samples.is_empty() && !self.streaming && self.stt_rx.is_none() {
                let wav = encode_pcm16_wav(&samples, SAMPLE_RATE);
                self.bubble_open = true;
                self.messages.push(ChatMessage {
                    role: ChatRole::Assistant,
                    content: "Transcribing voice...".into(),
                });
                let (tx, rx) = mpsc::unbounded_channel();
                self.stt_rx = Some(rx);
                self.rt.spawn(async move {
                    let wav_clone = wav.clone();
                    let text = tokio::task::spawn_blocking(move || transcribe_wav_local(&wav_clone))
                        .await
                        .ok()
                        .flatten();
                    let _ = tx.send((text, wav));
                });
            }
        } else if !self.streaming && self.stt_rx.is_none() {
            self.speech.interrupt();
            if let Ok(cap) = AudioCapture::start() {
                self.capture = Some(cap);
            }
        }
    }

    fn compile_cad(&mut self) {
        let dir = std::env::temp_dir().join("nephron_cad");
        if let Ok(res) = compile_cad_bundle(
            &self.cad_scad,
            self.cad_design.as_ref(),
            &dir,
            &self.cad_part_name,
        ) {
            let tris = res.mesh.triangles.len();
            let dims = res.mesh.dimensions();
            self.cad_viewport.mesh = res.mesh;
            self.cad_viewport.backend_label = res.backend.into();
            let path_str = res.stl_path.display().to_string();
            self.last_export_path = Some(res.stl_path);
            let _ = self.store.save_part(&self.cad_part_name, "PETG", 1, &path_str);
            if let Some(d) = &self.cad_design {
                if let Ok(j) = serde_json::to_string(d) {
                    let _ = self.store.save_cad_revision(&self.cad_part_name, &j);
                    self.cad_history.insert(0, d.clone());
                    self.cad_hist_idx = 0;
                }
            }
            self.cad_parts = self.store.list_parts().unwrap_or_default();
            let summary = format!(
                "Built {} ({tris} tris, {:.0}×{:.0}×{:.0} mm via {}).",
                self.cad_part_name.replace('_', " "),
                dims[0],
                dims[1],
                dims[2],
                self.cad_viewport.backend_label
            );
            self.messages.push(ChatMessage { role: ChatRole::Assistant, content: summary });
            if self.auto_tts {
                self.speech.enqueue(format!(
                    "Built {}, {} by {} by {} millimeters.",
                    self.cad_part_name.replace('_', " "),
                    dims[0].round() as i32,
                    dims[1].round() as i32,
                    dims[2].round() as i32
                ));
            }
        }
    }

    fn export_stl(&mut self) {
        let base = std::env::var_os("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir)
            .join("Exports");
        let _ = std::fs::create_dir_all(&base);
        let safe = sanitize_part_name(&self.cad_part_name);
        let out = base.join(format!("{safe}.stl"));
        if std::fs::write(&out, self.cad_viewport.mesh.to_binary_stl()).is_ok() {
            self.last_export_path = Some(out.clone());
            let _ = execute_action(&ComputerAction::Notify {
                title: "Nephron CAD".into(),
                body: format!("Exported {}", out.display()),
            });
        }
    }

    fn generate_cad(&mut self, desc: String) {
        if self.cad_rx.is_some() || desc.trim().is_empty() {
            return;
        }
        self.cad_repairs = 0;
        self.cad_prompt = desc.to_lowercase();
        self.cad_design = None;
        self.cad_scad.clear();
        self.cad_part_name = sanitize_part_name(&desc).chars().take(28).collect::<String>().to_lowercase();
        self.cad_viewport.backend_label = "designing...".into();
        self.messages.push(ChatMessage {
            role: ChatRole::Assistant,
            content: format!("Designing 3D model for \"{desc}\"..."),
        });
        if self.auto_tts {
            self.speech.enqueue("Designing your 3D model now.".into());
        }
        self.spawn_cad_request(desc);
    }

    fn spawn_cad_request(&mut self, prompt: String) {
        self.cad_buf.clear();
        let msgs = vec![
            ChatMessage { role: ChatRole::System, content: CAD_SYSTEM_PROMPT.into() },
            ChatMessage { role: ChatRole::User, content: prompt },
        ];
        let (tx, rx) = mpsc::unbounded_channel();
        self.cad_rx = Some(rx);
        let client = self.client.clone();
        let model_id = self.active_model_id();
        self.rt.spawn(async move {
            client.stream_chat_model(&model_id, EffortLevel::Med, msgs, tx).await;
        });
    }

    fn drain_events(&mut self, ctx: &egui::Context) {
        if let Some(rx) = &mut self.health_rx {
            if let Ok(ok) = rx.try_recv() {
                self.daemon_online = ok;
                self.health_rx = None;
            }
        }
        if let Some(rx) = &mut self.ollama_rx {
            if let Ok(list) = rx.try_recv() {
                if let Some(first) = list.first() {
                    self.selected_ollama = first.clone();
                }
                self.ollama_models = list;
                self.ollama_rx = None;
            }
        }
        let mut finished = false;
        if let Some(rx) = &mut self.stream_rx {
            while let Ok(ev) = rx.try_recv() {
                match ev {
                    StreamEvent::Delta(d) => {
                        if let Some(last) = self.messages.last_mut() {
                            last.content.push_str(&d);
                        }
                        if self.auto_tts {
                            for sentence in self.sentence_buf.push(&d) {
                                self.speech.enqueue(sentence);
                            }
                        }
                    }
                    StreamEvent::Done { tok_per_sec, .. } => {
                        self.last_tok_per_sec = tok_per_sec;
                        if let Some(last) = self.messages.last() {
                            let _ = self.store.save_memory("assistant", &last.content);
                        }
                        if self.auto_tts {
                            if let Some(rem) = self.sentence_buf.flush() {
                                self.speech.enqueue(rem);
                            }
                        }
                        finished = true;
                    }
                    StreamEvent::Error(err) => {
                        if let Some(last) = self.messages.last_mut() {
                            last.content = format!("[backend error] {err}");
                        }
                        let _ = self.sentence_buf.flush();
                        finished = true;
                    }
                }
            }
            ctx.request_repaint_after(Duration::from_millis(20));
        }
        if let Some(rx) = &mut self.stt_rx {
            if let Ok((maybe_text, wav)) = rx.try_recv() {
                self.stt_rx = None;
                if matches!(self.messages.last(), Some(m) if m.content == "Transcribing voice...") {
                    self.messages.pop();
                }
                if let Some(text) = maybe_text {
                    let _ = self.store.save_memory("voice", &text);
                    self.sync_brain();
                    self.dispatch_prompt(text);
                } else if self.backend_kind == BackendKind::Cynapse {
                    let b64 = wav_to_base64(&wav);
                    let _ = self.store.save_memory("voice", "[voice audio prompt]");
                    self.sync_brain();
                    self.messages.push(ChatMessage { role: ChatRole::User, content: "[Voice prompt]".into() });
                    self.messages.push(ChatMessage { role: ChatRole::Assistant, content: String::new() });
                    let (tx, srx) = mpsc::unbounded_channel();
                    self.stream_rx = Some(srx);
                    self.streaming = true;
                    let client = self.client.clone();
                    let effort = self.effort;
                    self.rt.spawn(async move {
                        client.stream_audio_prompt(ModelProfile::Lfm3Omni2B, effort, b64, tx).await;
                    });
                } else {
                    self.messages.push(ChatMessage {
                        role: ChatRole::Assistant,
                        content: "Didn't catch clear speech — try speaking a bit closer to the mic.".into(),
                    });
                }
            } else {
                ctx.request_repaint_after(Duration::from_millis(20));
            }
        }
        if let Some(rx) = &mut self.web_rx {
            if let Ok((q, hits)) = rx.try_recv() {
                self.web_sources = hits.clone();
                if let Some(last) = self.messages.last_mut() {
                    last.content.clear();
                }
                let c = if hits.is_empty() { None } else { Some(format_citations_prompt(&q, &hits)) };
                self.web_rx = None;
                self.start_chat_stream(c);
            } else {
                ctx.request_repaint_after(Duration::from_millis(20));
            }
        }
        if finished {
            self.streaming = false;
            self.stream_rx = None;
            self.sync_brain();
        }
        let mut cad_done = false;
        let mut should_compile = false;
        let mut repair_prompt: Option<String> = None;
        if let Some(rx) = &mut self.cad_rx {
            while let Ok(ev) = rx.try_recv() {
                match ev {
                    StreamEvent::Delta(d) => self.cad_buf.push_str(&d),
                    StreamEvent::Done { .. } => {
                        let parsed = parse_cad_bundle(&self.cad_buf);
                        let valid_err = match &parsed.design {
                            Some(d) => {
                                if let Err(e) = d.validate() {
                                    Some(e.to_string())
                                } else {
                                    let only_boxes = d.parts.iter().all(|p| p.primitives.iter().all(|pr| pr.kind == "box"));
                                    if only_boxes && self.cad_prompt.contains("pyramid") {
                                        Some("user requested a pyramid: use kind=\"pyramid\" with size=[40,40,40], NOT box".into())
                                    } else if only_boxes && self.cad_prompt.contains("cone") {
                                        Some("user requested a cone: use kind=\"cone\" with size=[20,0,40], NOT box".into())
                                    } else if only_boxes && self.cad_prompt.contains("sphere") {
                                        Some("user requested a sphere: use kind=\"sphere\" with size=[20,20,20], NOT box".into())
                                    } else {
                                        None
                                    }
                                }
                            }
                            None if parsed.scad.is_none() => Some("missing CadDesign JSON".into()),
                            None => None,
                        };
                        if let Some(err) = valid_err {
                            if self.cad_repairs < 2 {
                                self.cad_repairs += 1;
                                self.cad_viewport.backend_label = format!("refining ({}/2)...", self.cad_repairs);
                                repair_prompt = Some(cad_repair_prompt(&err));
                                cad_done = true;
                                break;
                            }
                        }
                        if let Some(code) = parsed.scad {
                            self.cad_scad = code;
                        }
                        if let Some(d) = &parsed.design {
                            if !d.name.trim().is_empty() {
                                self.cad_part_name = sanitize_part_name(&d.name);
                            }
                        }
                        self.cad_design = parsed.design;
                        for item in parsed.bom {
                            let _ = self.store.save_part(&item.name, &item.material, item.quantity, &self.cad_part_name);
                        }
                        should_compile = true;
                        cad_done = true;
                    }
                    StreamEvent::Error(e) => {
                        self.cad_viewport.backend_label = format!("err: {e}");
                        cad_done = true;
                    }
                }
            }
            ctx.request_repaint_after(Duration::from_millis(20));
        }
        if cad_done {
            self.cad_rx = None;
        }
        if let Some(rp) = repair_prompt {
            self.spawn_cad_request(rp);
        } else if should_compile {
            self.compile_cad();
        }
        let mut studio_done = false;
        if let Some(rx) = &mut self.studio_rx {
            while let Ok(ev) = rx.try_recv() {
                match ev {
                    StreamEvent::Delta(d) => self.studio_buf.push_str(&d),
                    StreamEvent::Done { .. } => {
                        let nodes = parse_studio_synthesis(&self.studio_buf, &mut self.studio);
                        for (t, b) in nodes {
                            let _ = self.store.save_memory("study", &format!("{t}: {b}"));
                        }
                        studio_done = true;
                    }
                    StreamEvent::Error(e) => {
                        self.studio.chat.push(crate::studio::StudioChatMsg {
                            user: false,
                            text: format!("[backend error] {e}"),
                            citations: Vec::new(),
                            surface: None,
                        });
                        studio_done = true;
                    }
                }
            }
            ctx.request_repaint_after(Duration::from_millis(20));
        }
        if studio_done {
            self.studio.generating = false;
            self.studio_rx = None;
            self.sync_brain();
            self.save_studio_and_rpg();
        }
        if let Some(rx) = &mut self.studio_web_rx {
            if let Ok((q, hits)) = rx.try_recv() {
                self.studio_web_rx = None;
                let summary = if hits.is_empty() {
                    format!("No web hits for \"{q}\".")
                } else {
                    hits.iter()
                        .map(|h| format!("• {} ({}): {}", h.title, h.url, h.snippet))
                        .collect::<Vec<_>>()
                        .join("\n")
                };
                self.studio.sources.insert(
                    0,
                    StudioSource {
                        id: format!("web_{}", self.studio.sources.len() + 1),
                        title: format!("Web: {q}"),
                        kind: "SEARCH".into(),
                        active: true,
                        content: summary,
                    },
                );
                self.save_studio_and_rpg();
                self.trigger_studio_synthesis(format!("Research \"{q}\" and add findings to our notebook."));
            } else {
                ctx.request_repaint_after(Duration::from_millis(20));
            }
        }
        if let Some(rx) = &mut self.studio_file_rx {
            if let Ok(maybe_path) = rx.try_recv() {
                self.studio_file_rx = None;
                if let Some(path) = maybe_path {
                    if self.studio.add_file_source(&path) {
                        self.save_studio_and_rpg();
                        self.brain.trigger_cascade();
                    }
                }
            } else {
                ctx.request_repaint_after(Duration::from_millis(40));
            }
        }
    }
}

fn kinetic_glass() -> Frame {
    Frame::NONE
        .fill(Color32::from_rgba_unmultiplied(10, 14, 24, 212))
        .stroke(Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(255, 255, 255, 28)))
        .corner_radius(22.0)
        .inner_margin(16.0)
}

fn kinetic_dock_frame() -> Frame {
    Frame::NONE
        .fill(Color32::from_rgba_unmultiplied(11, 15, 25, 190))
        .stroke(Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(255, 255, 255, 28)))
        .corner_radius(26.0)
        .inner_margin(egui::Margin::symmetric(8, 6))
}

fn kinetic_icon(ui: &mut egui::Ui, id: &str, active: bool, kind: IconKind, tip: &str, tint: Color32) -> (bool, bool, Pos2) {
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(40.0, 40.0), egui::Sense::click());
    let hovered = resp.hovered();
    let hover_t = ui.ctx().animate_bool(egui::Id::new(id).with("hov"), hovered);
    let press_t = ui.ctx().animate_bool(egui::Id::new(id).with("prs"), resp.is_pointer_button_down_on());
    let lift = hover_t * 3.5 - press_t * 1.5;
    let scale = 1.0 + hover_t * 0.14 - press_t * 0.10;
    let col = if active || hover_t > 0.2 {
        tint
    } else {
        Color32::from_rgba_unmultiplied(tint.r(), tint.g(), tint.b(), 195)
    };
    if hover_t > 0.02 && !active {
        ui.painter().circle_filled(
            rect.center(),
            18.0 + hover_t * 1.5,
            Color32::from_rgba_unmultiplied(255, 255, 255, (hover_t * 24.0) as u8),
        );
    }
    paint_icon(ui.painter(), Pos2::new(rect.center().x, rect.center().y - lift), scale, kind, col);
    let clicked = resp.on_hover_text(tip).clicked();
    (clicked, hovered, rect.center())
}

impl eframe::App for NephronApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let dt = ctx.input(|i| i.stable_dt).min(0.05);
        self.ui_time += dt;
        self.rpg.tick(dt);
        if self.last_poll.elapsed() >= Duration::from_secs(3) {
            self.poll_system();
        }
        if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::M)) {
            self.toggle_mic();
        }
        if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::K)) {
            self.cmd_palette_open = !self.cmd_palette_open;
            self.cmd_query.clear();
        }
        if ctx.input(|i| i.modifiers.ctrl && i.modifiers.shift && i.key_pressed(egui::Key::L)) {
            self.studio.open = !self.studio.open;
            if self.studio.open {
                self.active_tab = None;
            }
        }
        if ctx.input(|i| i.modifiers.ctrl && i.modifiers.shift && i.key_pressed(egui::Key::S)) {
            self.capture_and_describe_screen("What is on my screen right now?".into());
        }
        // Pomodoro / Focus Boss Quest completion check
        if let Some(end) = self.pomo_end {
            if Instant::now() >= end {
                self.pomo_end = None;
                self.rpg.award_xp(50, 15, "+50 XP · Focus Quest!");
                self.save_studio_and_rpg();
                self.brain.trigger_cascade();
                self.bubble_open = true;
                self.messages.push(ChatMessage {
                    role: ChatRole::Assistant,
                    content: "Focus Quest complete (+50 XP)! Time to stretch and take a short break.".into(),
                });
                let _ = execute_action(&ComputerAction::Notify {
                    title: "Nephron Focus Quest".into(),
                    body: "25m session done (+50 XP) — take a short break!".into(),
                });
                if self.auto_tts {
                    self.speech.enqueue("Focus quest complete. Time to stretch and take a short break.".into());
                }
            }
        }
        // File-drop intelligence (.stl, .scad, PDF/study files)
        let dropped = ctx.input(|i| i.raw.dropped_files.clone());
        for f in dropped {
            if let Some(path) = f.path {
                let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
                if self.studio.open && matches!(ext.as_str(), "pdf" | "txt" | "md" | "csv" | "json" | "rs" | "py") {
                    if self.studio.add_file_source(&path) {
                        self.save_studio_and_rpg();
                        self.brain.trigger_cascade();
                    }
                    continue;
                }
                if ext == "stl" {
                    if let Ok(bytes) = std::fs::read(&path) {
                        if let Ok(mesh) = StlMesh::parse(&bytes) {
                            self.cad_viewport.mesh = mesh;
                            self.cad_viewport.backend_label = "dropped-stl".into();
                            self.active_tab = Some(DrawerTab::Cad);
                        }
                    }
                } else if ext == "scad" {
                    if let Ok(code) = std::fs::read_to_string(&path) {
                        self.cad_scad = code;
                        self.cad_design = None;
                        self.active_tab = Some(DrawerTab::Cad);
                        self.compile_cad();
                    }
                } else if let Ok(txt) = std::fs::read_to_string(&path) {
                    let snippet: String = txt.chars().take(3500).collect();
                    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("file");
                    let _ = self.store.save_memory("user", &format!("[file:{name}] {}", snippet.chars().take(120).collect::<String>()));
                    self.sync_brain();
                    self.dispatch_prompt(format!("Summarize this dropped file ({name}):\n{snippet}"));
                }
            }
        }
        // Auto-VAD hands-free turn-taking & Barge-In interrupt
        if let Some(cap) = &self.capture {
            let vst = cap.current_vad();
            if vst == VadState::Speech {
                self.vad_was_speech = true;
                if self.speech.is_speaking() {
                    self.speech.interrupt();
                }
            } else if self.auto_vad && self.vad_was_speech && vst == VadState::Silence {
                self.toggle_mic();
            }
        }
        if let Some(cv) = &self.cv_gesture {
            let d = cv.take_delta();
            if d.active {
                self.cad_viewport.yaw += d.d_yaw;
                self.cad_viewport.pitch = (self.cad_viewport.pitch + d.d_pitch).clamp(-1.4, 1.4);
                self.cad_viewport.zoom = (self.cad_viewport.zoom + d.d_zoom).clamp(0.5, 10.0);
                ctx.request_repaint_after(Duration::from_millis(16));
            }
        }
        self.drain_events(ctx);

        let act = if self.capture.is_some() {
            BrainActivity::Listening
        } else if self.speech.is_speaking() || (self.streaming && self.messages.last().map(|m| !m.content.is_empty()).unwrap_or(false)) {
            BrainActivity::Speaking
        } else if self.streaming || self.stt_rx.is_some() || self.cad_rx.is_some() || self.web_rx.is_some() || !self.input.is_empty() {
            BrainActivity::Thinking
        } else {
            BrainActivity::Idle
        };

        // Full-window stage: 3D CAD Studio OR MindCorpus Prismatic Brain
        egui::CentralPanel::default()
            .frame(Frame::NONE.fill(Color32::from_rgb(3, 4, 7)))
            .show(ctx, |ui| {
                let rect = ui.max_rect();
                if self.active_tab == Some(DrawerTab::Cad) {
                    self.cad_viewport.ui_rect(ui, rect);
                } else {
                    let energy = if let Some(cap) = &self.capture {
                        (cap.current_rms() * 6.5).clamp(0.1, 1.0)
                    } else {
                        match act {
                            BrainActivity::Idle => 0.08,
                            BrainActivity::Listening => 0.55,
                            BrainActivity::Thinking => 0.75,
                            BrainActivity::Speaking => 0.85,
                        }
                    };
                    let (clicked_mem, interacting) = self.brain.paint(ui, rect, dt, act, energy);
                    if let Some(mem) = clicked_mem {
                        self.active_tab = Some(DrawerTab::Memory);
                        self.memory_query = mem.clone();
                        self.memory_hits = self.store.search_fts(&mem, 12).unwrap_or_default();
                        if self.input.trim().is_empty() {
                            self.input = mem;
                        }
                    }
                    if interacting {
                        ctx.request_repaint();
                    } else {
                        let fast = self.capture.is_some()
                            || self.streaming
                            || self.cad_rx.is_some()
                            || self.web_rx.is_some()
                            || self.speech.is_speaking()
                            || self.brain.auto_rotate
                            || !self.input.is_empty();
                        ctx.request_repaint_after(Duration::from_millis(if fast { 33 } else { 150 }));
                    }
                }
            });

        // 1. Whisper-Quiet Top-Left Backend Toggle Pill (click to toggle Cynapse / Ollama)
        egui::Area::new(egui::Id::new("whisper_backend_dot"))
            .anchor(Align2::LEFT_TOP, [18.0, 16.0])
            .show(ctx, |ui| {
                let dot_col = if self.daemon_online {
                    Color32::from_rgb(56, 189, 248)
                } else {
                    Color32::from_rgb(251, 146, 60)
                };
                let label = match self.backend_kind {
                    BackendKind::Cynapse => "cynapse",
                    BackendKind::Ollama => "ollama",
                };
                let btn = egui::Button::new(
                    egui::RichText::new(format!("   {label}"))
                        .size(11.5)
                        .monospace()
                        .color(Color32::from_rgb(203, 213, 225)),
                )
                .fill(Color32::from_rgba_unmultiplied(11, 15, 25, 190))
                .stroke(Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(255, 255, 255, 28)))
                .corner_radius(16.0)
                .min_size(egui::vec2(88.0, 28.0));
                let tip = format!(
                    "Click to switch Cynapse / Ollama\nModel: {}\nRAM free: {:.1} GiB",
                    self.active_model_id(),
                    self.telemetry.available_mib as f32 / 1024.0
                );
                let resp = ui.add(btn).on_hover_text(tip);
                ui.painter().circle_filled(
                    Pos2::new(resp.rect.left() + 14.0, resp.rect.center().y),
                    3.2,
                    dot_col,
                );
                if resp.clicked() {
                    let next = match self.backend_kind {
                        BackendKind::Cynapse => BackendKind::Ollama,
                        BackendKind::Ollama => BackendKind::Cynapse,
                    };
                    self.switch_backend(next);
                }
            });

        // 2. Single Centered Kinetic Mini-Drawer (opens smoothly above the bottom navbar)
        if let Some(tab) = self.active_tab {
            egui::Area::new(egui::Id::new("kinetic_drawer"))
                .anchor(Align2::CENTER_BOTTOM, [0.0, -78.0])
                .show(ctx, |ui| {
                    kinetic_glass().show(ui, |ui| {
                        ui.set_min_width(285.0);
                        ui.set_max_width(285.0);
                        match tab {
                            DrawerTab::Bridge => {
                                ui.horizontal(|ui| {
                                    if ui.selectable_label(self.backend_kind == BackendKind::Cynapse, "cynapse").clicked() {
                                        self.switch_backend(BackendKind::Cynapse);
                                    }
                                    if ui.selectable_label(self.backend_kind == BackendKind::Ollama, "ollama").clicked() {
                                        self.switch_backend(BackendKind::Ollama);
                                    }
                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                        if ui.checkbox(&mut self.auto_tts, "tts").changed() {
                                            self.save_settings();
                                        }
                                        if ui.checkbox(&mut self.auto_vad, "vad").changed() {
                                            self.save_settings();
                                        }
                                    });
                                });
                                ui.horizontal(|ui| {
                                    if self.backend_kind == BackendKind::Cynapse {
                                        egui::ComboBox::from_id_salt("cyn_m").selected_text(self.model.id()).show_ui(ui, |ui| {
                                            for m in ModelProfile::ALL {
                                                ui.selectable_value(&mut self.model, m, m.id());
                                            }
                                        });
                                    } else {
                                        egui::ComboBox::from_id_salt("ol_m").selected_text(&self.selected_ollama).show_ui(ui, |ui| {
                                            for m in &self.ollama_models {
                                                if ui.selectable_value(&mut self.selected_ollama, m.clone(), m).clicked() {
                                                    self.save_settings();
                                                }
                                            }
                                        });
                                        if ui.small_button("↻").clicked() {
                                            self.refresh_ollama();
                                        }
                                    }
                                    egui::ComboBox::from_id_salt("eff").selected_text(self.effort.as_str()).show_ui(ui, |ui| {
                                        for e in EffortLevel::ALL {
                                            ui.selectable_value(&mut self.effort, e, e.as_str());
                                        }
                                    });
                                });
                                ui.horizontal(|ui| {
                                    ui.small("voice");
                                    egui::ComboBox::from_id_salt("tts_v").selected_text(&self.tts_voice).show_ui(ui, |ui| {
                                        for &v in FEMALE_VOICES {
                                            if ui.selectable_value(&mut self.tts_voice, v.to_string(), v).clicked() {
                                                self.speech.set_voice(v.to_string());
                                                self.save_settings();
                                            }
                                        }
                                    });
                                });
                            }
                            DrawerTab::Memory => {
                                ui.horizontal(|ui| {
                                    ui.small(egui::RichText::new("dendrites").color(Color32::from_rgb(252, 211, 77)));
                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                        if ui.small_button("clear unpinned").clicked() {
                                            let _ = self.store.clear_unpinned();
                                            self.sync_brain();
                                        }
                                        ui.checkbox(&mut self.inject_pinned, "pin ctx");
                                    });
                                });
                                ui.horizontal(|ui| {
                                    let r = ui.add_sized([200.0, 22.0], egui::TextEdit::singleline(&mut self.memory_query).hint_text("search memory..."));
                                    if (r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter))) || ui.small_button("go").clicked() {
                                        self.memory_hits = self.store.search_fts(&self.memory_query, 12).unwrap_or_default();
                                    }
                                });
                                let mut toggle_pin: Option<(i64, bool)> = None;
                                egui::ScrollArea::vertical().max_height(150.0).show(ui, |ui| {
                                    ui.set_max_width(270.0);
                                    for h in &self.memory_hits {
                                        ui.horizontal(|ui| {
                                            if ui.small_button(if h.pinned { "★" } else { "☆" }).clicked() {
                                                toggle_pin = Some((h.id, !h.pinned));
                                            }
                                            let preview: String = h.content.chars().take(90).collect();
                                            ui.add(egui::Label::new(egui::RichText::new(format!("[{}] {}", h.kind, preview)).size(11.0)).wrap());
                                        });
                                        ui.separator();
                                    }
                                });
                                if let Some((id, pinned)) = toggle_pin {
                                    let _ = self.store.pin_memory(id, pinned);
                                    self.sync_brain();
                                }
                            }
                            DrawerTab::Cad => {
                                ui.horizontal(|ui| {
                                    ui.add_sized([100.0, 22.0], egui::TextEdit::singleline(&mut self.cad_part_name));
                                    if ui.small_button("Compile").clicked() { self.compile_cad(); }
                                    if ui.small_button("Gen AI").clicked() {
                                        let d = if self.input.trim().is_empty() { self.cad_part_name.clone() } else { self.input.clone() };
                                        self.generate_cad(d);
                                    }
                                    if !self.cad_history.is_empty() {
                                        if ui.small_button("◀").clicked() && self.cad_hist_idx + 1 < self.cad_history.len() {
                                            self.cad_hist_idx += 1;
                                            let d = self.cad_history[self.cad_hist_idx].clone();
                                            if let Ok(m) = d.build_mesh_exploded(self.cad_explode) {
                                                self.cad_viewport.mesh = m;
                                                self.cad_part_name = sanitize_part_name(&d.name);
                                                self.cad_design = Some(d);
                                            }
                                        }
                                        if ui.small_button("▶").clicked() && self.cad_hist_idx > 0 {
                                            self.cad_hist_idx -= 1;
                                            let d = self.cad_history[self.cad_hist_idx].clone();
                                            if let Ok(m) = d.build_mesh_exploded(self.cad_explode) {
                                                self.cad_viewport.mesh = m;
                                                self.cad_part_name = sanitize_part_name(&d.name);
                                                self.cad_design = Some(d);
                                            }
                                        }
                                    }
                                });
                                ui.horizontal(|ui| {
                                    ui.checkbox(&mut self.cad_viewport.wireframe, "wire");
                                    let cv_on = self.cv_gesture.is_some();
                                    if ui.selectable_label(cv_on, "cv hand").clicked() {
                                        if cv_on {
                                            self.cv_gesture = None;
                                        } else {
                                            self.cv_gesture = CvGestureTracker::start("/dev/video0").ok();
                                        }
                                    }
                                    if ui.small_button("export stl").clicked() { self.export_stl(); }
                                    if let Some(p) = &self.last_export_path {
                                        if ui.small_button("slicer").clicked() {
                                            let _ = execute_action(&ComputerAction::OpenPath(p.clone()));
                                        }
                                    }
                                });
                                if let Some(d) = &mut self.cad_design {
                                    if let Some(prim) = d.parts.first_mut().and_then(|p| p.primitives.first_mut()) {
                                        let mut changed = false;
                                        ui.horizontal(|ui| {
                                            changed |= ui.add(egui::DragValue::new(&mut prim.size[0]).range(1.0..=500.0).prefix("W ")).changed();
                                            changed |= ui.add(egui::DragValue::new(&mut prim.size[1]).range(1.0..=500.0).prefix("D ")).changed();
                                            changed |= ui.add(egui::DragValue::new(&mut prim.size[2]).range(1.0..=500.0).prefix("H ")).changed();
                                        });
                                        if d.parts.len() > 1 {
                                            changed |= ui.add(egui::Slider::new(&mut self.cad_explode, 0.0..=1.0).text("explode")).changed();
                                        }
                                        if changed {
                                            if let Ok(m) = d.build_mesh_exploded(self.cad_explode) {
                                                self.cad_viewport.mesh = m;
                                            }
                                        }
                                    }
                                }
                                ui.add(egui::TextEdit::multiline(&mut self.cad_scad).desired_rows(3).code_editor());
                                for p in self.cad_parts.iter().take(2) {
                                    ui.small(format!("{} x{} ({})", p.name, p.quantity, p.material));
                                }
                            }
                            DrawerTab::Agents => {
                                ui.horizontal(|ui| {
                                    for (sp, ico) in [(PetSpecies::Cat, "🐱"), (PetSpecies::Chiba, "🐕"), (PetSpecies::Sheep, "🐑")] {
                                        if ui.selectable_label(self.pet_species == sp && self.pet_active, ico).clicked() {
                                            self.pet_species = sp;
                                            self.pet_active = true;
                                            self.pet_proc.kill();
                                        }
                                    }
                                    ui.separator();
                                    if ui.small_button(if self.pet_active { "hide pet" } else { "pet" }).clicked() {
                                        self.pet_active = !self.pet_active;
                                    }
                                    if ui.selectable_label(self.swarm_active, "swarm").clicked() {
                                        self.swarm_active = !self.swarm_active;
                                    }
                                });
                                ui.horizontal(|ui| {
                                    if ui.small_button("⚡ shot").clicked() {
                                        let _ = execute_action(&ComputerAction::CaptureScreen(std::env::temp_dir().join("nephron_cua.png")));
                                    }
                                    if ui.small_button("⚡ term").clicked() {
                                        let _ = execute_action(&ComputerAction::LaunchApp("gnome-terminal".into()));
                                    }
                                    if ui.small_button("⚡ check").clicked() {
                                        if let Ok(out) = execute_action(&ComputerAction::AllowlistedCheck { bin: "uname", args: vec!["-a".into()] }) {
                                            self.messages.push(ChatMessage { role: ChatRole::Assistant, content: out });
                                        }
                                    }
                                    if ui.small_button("⚡ flow").clicked() {
                                        self.run_cua_workflow();
                                    }
                                    let pomo_lbl = if let Some(end) = self.pomo_end {
                                        let rem = end.saturating_duration_since(Instant::now()).as_secs() / 60;
                                        format!("⏱ {rem}m")
                                    } else {
                                        "⏱ 25m".into()
                                    };
                                    if ui.selectable_label(self.pomo_end.is_some(), pomo_lbl).clicked() {
                                        self.pomo_end = if self.pomo_end.is_some() {
                                            None
                                        } else {
                                            Some(Instant::now() + Duration::from_secs(25 * 60))
                                        };
                                    }
                                });
                            }
                            DrawerTab::Cortex => {
                                ui.horizontal(|ui| {
                                    if ui.selectable_label(self.brain.auto_rotate, "spin").clicked() {
                                        self.brain.auto_rotate = !self.brain.auto_rotate;
                                    }
                                    if ui.small_button("hue").clicked() {
                                        self.brain.palette_idx = (self.brain.palette_idx + 1) % 3;
                                        self.brain.trigger_cascade();
                                    }
                                    ui.checkbox(&mut self.web_mode, "web");
                                    ui.checkbox(&mut self.show_hw_hud, "hw");
                                });
                                ui.add(egui::Slider::new(&mut self.brain.pulse_speed, 0.3..=2.5).text("pulse"));
                                ui.add(egui::Slider::new(&mut self.brain.glitter, 0.4..=1.8).text("glitter"));
                            }
                        }
                    });
                });
        }

        // 3. Center-Bottom Kinetic Icon Navbar with Sliding Magnetic Pill
        egui::Area::new(egui::Id::new("kinetic_navbar"))
            .anchor(Align2::CENTER_BOTTOM, [0.0, -18.0])
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                kinetic_dock_frame().show(ui, |ui| {
                    let pill_bg_idx = ui.painter().add(egui::Shape::Noop);
                    let pill_ring_idx = ui.painter().add(egui::Shape::Noop);
                    let mut pill_target: Option<Pos2> = None;
                    let mut any_hov = false;
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 4.0;
                        let mic_active = self.capture.is_some();
                        let (mic_clk, mic_hov, mic_c) = kinetic_icon(ui, "nav_mic", mic_active, IconKind::Mic, "Voice Mic (Ctrl+M)", Color32::from_rgb(253, 164, 175));
                        any_hov |= mic_hov;
                        if mic_active {
                            let r = 17.0 + ((self.ui_time * 3.0).sin() * 0.5 + 0.5) * 6.0;
                            ui.painter().circle_stroke(mic_c, r, Stroke::new(1.5_f32, Color32::from_rgba_unmultiplied(251, 113, 133, 160)));
                        }
                        if mic_clk {
                            self.toggle_mic();
                        }

                        let (studio_clk, studio_hov, studio_c) = kinetic_icon(
                            ui,
                            "nav_studio",
                            self.studio.open,
                            IconKind::Studio,
                            "Interface Studio (Ctrl+Shift+L)",
                            Color32::from_rgb(165, 180, 252),
                        );
                        any_hov |= studio_hov;
                        if self.studio.open {
                            pill_target = Some(studio_c);
                        }
                        if studio_clk {
                            self.studio.open = !self.studio.open;
                            if self.studio.open {
                                self.active_tab = None;
                                self.brain.trigger_cascade();
                            }
                        }

                        ui.separator();

                        let tabs = [
                            (DrawerTab::Bridge, "nav_bridge", IconKind::Bridge, "AI Bridge", Color32::from_rgb(125, 211, 252)),
                            (DrawerTab::Memory, "nav_mem", IconKind::Memory, "Dendrite Memory", Color32::from_rgb(252, 211, 77)),
                            (DrawerTab::Cad, "nav_cad", IconKind::Cad, "Parametric CAD", Color32::from_rgb(110, 231, 183)),
                            (DrawerTab::Agents, "nav_agents", IconKind::Swarm, "Pets & Swarm", Color32::from_rgb(216, 180, 254)),
                            (DrawerTab::Cortex, "nav_cortex", IconKind::Cortex, "Cortex Visuals", Color32::from_rgb(203, 213, 225)),
                        ];
                        for (tab, id, kind, tip, col) in tabs {
                            let is_active = self.active_tab == Some(tab);
                            let (clk, hov, c) = kinetic_icon(ui, id, is_active, kind, tip, col);
                            any_hov |= hov;
                            if is_active {
                                pill_target = Some(c);
                            }
                            if clk {
                                self.studio.open = false;
                                self.active_tab = if is_active { None } else { Some(tab) };
                            }
                        }

                        let (burst_clk, burst_hov, _) = kinetic_icon(ui, "nav_burst", false, IconKind::Cascade, "Synaptic Cascade", Color32::from_rgb(103, 232, 249));
                        any_hov |= burst_hov;
                        if burst_clk {
                            self.brain.trigger_cascade();
                        }
                    });

                    if let Some(target_c) = pill_target {
                        let x = self.pill_x.get_or_insert(target_c.x);
                        let dx = target_c.x - *x;
                        // Damped spring (matches cubic-bezier(0.34, 1.56, 0.64, 1) overshoot)
                        self.pill_vx += dx * 280.0 * dt;
                        self.pill_vx *= (-16.0 * dt).exp();
                        *x += self.pill_vx * dt;
                        if dx.abs() > 0.25 || self.pill_vx.abs() > 1.0 {
                            ctx.request_repaint();
                        }
                        ui.painter().set(
                            pill_bg_idx,
                            egui::Shape::circle_filled(
                                Pos2::new(*x, target_c.y),
                                19.0,
                                Color32::from_rgba_unmultiplied(255, 255, 255, 30),
                            ),
                        );
                        ui.painter().set(
                            pill_ring_idx,
                            egui::Shape::circle_stroke(
                                Pos2::new(*x, target_c.y),
                                19.0,
                                Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(255, 255, 255, 58)),
                            ),
                        );
                    } else {
                        self.pill_x = None;
                        self.pill_vx = 0.0;
                    }
                    if any_hov {
                        ctx.request_repaint();
                    }
                });
            });

        // 4. Bottom-Right Living Bubble Head Companion + Expandable Universal Chat Popover
        if self.bubble_open && !self.studio.open {
            egui::Area::new(egui::Id::new("bubble_popover"))
                .anchor(Align2::RIGHT_BOTTOM, [-20.0, -86.0])
                .show(ctx, |ui| {
                    kinetic_glass().show(ui, |ui| {
                        ui.set_width(340.0);
                        ui.spacing_mut().item_spacing.y = 6.0;
                        let mut ctx_action: Option<(u8, String)> = None;
                        let mut a2ui_prompt: Option<String> = None;
                        egui::ScrollArea::vertical().max_height(240.0).stick_to_bottom(true).show(ui, |ui| {
                            for m in &self.messages {
                                let (who, col) = match m.role {
                                    ChatRole::User => ("You", Color32::from_rgb(125, 211, 252)),
                                    ChatRole::Assistant => ("Nephron", Color32::from_rgb(165, 180, 252)),
                                    ChatRole::System => ("System", Color32::from_rgb(148, 163, 184)),
                                };
                                ui.colored_label(col, egui::RichText::new(who).strong().size(11.0));
                                let parsed_ui = if m.role == ChatRole::Assistant && !self.streaming {
                                    crate::a2ui::A2uiSurface::parse_from_text(&m.content)
                                        .or_else(|| crate::a2ui::A2uiSurface::from_markdown_heuristic(&m.content))
                                } else {
                                    None
                                };
                                if let Some(mut surface) = parsed_ui {
                                    if let Some(act) = crate::a2ui::draw_a2ui_surface(ui, 316.0, &mut surface) {
                                        match act.kind.as_deref() {
                                            Some("cad") => ctx_action = Some((2, act.prompt.unwrap_or_else(|| m.content.clone()))),
                                            Some("pin") => ctx_action = Some((1, m.content.clone())),
                                            _ => a2ui_prompt = act.prompt,
                                        }
                                    }
                                } else {
                                    let lbl = ui.add(egui::Label::new(egui::RichText::new(&m.content).size(12.0)).wrap().sense(egui::Sense::click()));
                                    lbl.context_menu(|ui| {
                                        if ui.button("Copy").clicked() {
                                            ctx_action = Some((0, m.content.clone()));
                                            ui.close_menu();
                                        }
                                        if ui.button("Pin to Brain").clicked() {
                                            ctx_action = Some((1, m.content.clone()));
                                            ui.close_menu();
                                        }
                                        if ui.button("Send to CAD").clicked() {
                                            ctx_action = Some((2, m.content.clone()));
                                            ui.close_menu();
                                        }
                                    });
                                }
                                ui.add_space(6.0);
                            }
                            if !self.web_sources.is_empty() {
                                ui.separator();
                                ui.horizontal(|ui| {
                                    ui.small("Sources");
                                    if ui.small_button("clear").clicked() {
                                        self.web_sources.clear();
                                    }
                                });
                                let mut save_source: Option<(String, String)> = None;
                                for row in self.web_sources.chunks(2) {
                                    ui.horizontal(|ui| {
                                        for (j, s) in row.iter().enumerate() {
                                            let idx = self.web_sources.iter().position(|x| std::ptr::eq(x, s)).unwrap_or(j);
                                            let domain = s.url.split("//").nth(1).and_then(|r| r.split('/').next()).unwrap_or("web").trim_start_matches("www.");
                                            let short: String = s.title.chars().take(18).collect();
                                            Frame::NONE
                                                .fill(Color32::from_rgba_unmultiplied(24, 32, 48, 220))
                                                .stroke(Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(125, 211, 252, 45)))
                                                .corner_radius(8.0)
                                                .inner_margin(egui::Margin::symmetric(8, 5))
                                                .show(ui, |ui| {
                                                    ui.set_min_width(134.0);
                                                    ui.set_max_width(134.0);
                                                    ui.vertical(|ui| {
                                                        ui.horizontal(|ui| {
                                                            ui.hyperlink_to(egui::RichText::new(format!("[{}] {}", idx + 1, short)).size(11.0), &s.url)
                                                                .on_hover_text(format!("{}\n{}", s.title, s.snippet));
                                                            if ui.small_button("★").on_hover_text("Pin to Dendrite").clicked() {
                                                                save_source = Some((s.title.clone(), s.url.clone()));
                                                            }
                                                        });
                                                        ui.small(egui::RichText::new(domain).color(Color32::from_rgb(148, 163, 184)).size(9.5));
                                                    });
                                                });
                                        }
                                    });
                                    ui.add_space(4.0);
                                }
                                if let Some((title, url)) = save_source {
                                    if let Ok(id) = self.store.save_memory("web", &format!("{title} — {url}")) {
                                        let _ = self.store.pin_memory(id, true);
                                        self.sync_brain();
                                    }
                                }
                            }
                        });
                        ui.add_space(4.0);
                        if let Some((act_id, txt)) = ctx_action {
                            match act_id {
                                0 => ctx.copy_text(txt),
                                1 => {
                                    if let Ok(id) = self.store.save_memory("user", &txt) {
                                        let _ = self.store.pin_memory(id, true);
                                        self.sync_brain();
                                    }
                                }
                                _ => {
                                    self.active_tab = Some(DrawerTab::Cad);
                                    self.generate_cad(txt);
                                }
                            }
                        }
                        if let Some(p) = a2ui_prompt {
                            self.dispatch_prompt(p);
                        }
                        Frame::NONE
                            .fill(Color32::from_rgba_unmultiplied(255, 255, 255, 10))
                            .stroke(Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(255, 255, 255, 28)))
                            .corner_radius(99.0)
                            .inner_margin(egui::Margin::symmetric(12, 4))
                            .show(ui, |ui| {
                                ui.set_min_size(egui::vec2(316.0, 28.0));
                                ui.set_max_size(egui::vec2(316.0, 28.0));
                                ui.horizontal(|ui| {
                                    let resp = ui.add_sized(
                                        [274.0, 24.0],
                                        egui::TextEdit::singleline(&mut self.input)
                                            .frame(false)
                                            .hint_text("Ask, search, or build CAD..."),
                                    );
                                    let enter = resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                                    let send = crate::icons::vector_icon_button(
                                        ui,
                                        "bubble_send",
                                        egui::vec2(26.0, 26.0),
                                        IconKind::SendRight,
                                        Color32::from_rgb(186, 230, 253),
                                        "Send",
                                    );
                                    if !self.streaming && (send || enter) && !self.input.trim().is_empty() {
                                        self.send_prompt();
                                    }
                                });
                            });
                    });
                });
        }

        egui::Area::new(egui::Id::new("bubble_head_orb"))
            .anchor(Align2::RIGHT_BOTTOM, [-20.0, -18.0])
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    let status_txt = match act {
                        BrainActivity::Listening => Some("listening..."),
                        BrainActivity::Thinking => Some("thinking..."),
                        BrainActivity::Speaking => Some("speaking..."),
                        BrainActivity::Idle => None,
                    };
                    if let Some(st) = status_txt {
                        Frame::NONE
                            .fill(Color32::from_rgba_unmultiplied(10, 14, 24, 195))
                            .stroke(Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(255, 255, 255, 28)))
                            .corner_radius(14.0)
                            .inner_margin(egui::Margin::symmetric(10, 5))
                            .show(ui, |ui| {
                                ui.small(egui::RichText::new(st).monospace().color(Color32::from_rgb(203, 213, 225)));
                            });
                    }

                    let (rect, resp) = ui.allocate_exact_size(egui::vec2(52.0, 52.0), egui::Sense::click());
                    let hov = ui.ctx().animate_bool(egui::Id::new("orb_hov"), resp.hovered());
                    let bob = (self.ui_time * 2.2).sin() * 2.5;
                    let c = Pos2::new(rect.center().x, rect.center().y + bob);
                    let r = 24.0 + hov * 2.0;
                    let ring_col = if self.capture.is_some() {
                        Color32::from_rgba_unmultiplied(251, 113, 133, 180)
                    } else {
                        Color32::from_rgba_unmultiplied(56, 189, 248, (90.0 + hov * 90.0) as u8)
                    };
                    ui.painter().circle_filled(c, r, Color32::from_rgba_unmultiplied(15, 23, 42, 225));
                    ui.painter().circle_stroke(c, r, Stroke::new(1.5_f32, ring_col));

                    if let Some(cap) = &self.capture {
                        let rms = (cap.current_rms() * 8.0).clamp(0.0, 1.0);
                        ui.painter().circle_stroke(c, r + 3.0 + rms * 14.0, Stroke::new(1.8_f32, Color32::from_rgba_unmultiplied(251, 113, 133, 140)));
                        for k in 0..4 {
                            let h = 6.0 + ((self.ui_time * 8.0 + k as f32 * 1.3).sin() * 0.5 + 0.5) * 10.0;
                            let x = c.x - 9.0 + k as f32 * 6.0;
                            ui.painter().line_segment([Pos2::new(x, c.y - h * 0.5), Pos2::new(x, c.y + h * 0.5)], Stroke::new(2.2_f32, Color32::from_rgb(253, 164, 175)));
                        }
                    } else {
                        let blink = if (self.ui_time % 4.6) > 4.45 { 0.3 } else { 3.0 };
                        let eye_col = Color32::from_rgb(186, 230, 253);
                        ui.painter().rect_filled(Rect::from_center_size(Pos2::new(c.x - 6.5, c.y - 3.0), egui::vec2(3.6, blink * 2.0)), 1.5, eye_col);
                        ui.painter().rect_filled(Rect::from_center_size(Pos2::new(c.x + 6.5, c.y - 3.0), egui::vec2(3.6, blink * 2.0)), 1.5, eye_col);
                        let mouth_y = c.y + 6.0 + if act == BrainActivity::Speaking { (self.ui_time * 14.0).sin().abs() * 2.5 } else { 0.0 };
                        ui.painter().line_segment([Pos2::new(c.x - 5.0, c.y + 5.5), Pos2::new(c.x, mouth_y)], Stroke::new(1.8_f32, eye_col));
                        ui.painter().line_segment([Pos2::new(c.x, mouth_y), Pos2::new(c.x + 5.0, c.y + 5.5)], Stroke::new(1.8_f32, eye_col));
                    }
                    if resp.on_hover_text("Toggle Chat / Prompt").clicked() {
                        self.bubble_open = !self.bubble_open;
                    }
                });
            });

        self.draw_studio_modal(ctx);

        if self.cmd_palette_open {
            egui::Area::new(egui::Id::new("cmd_palette"))
                .anchor(Align2::CENTER_CENTER, [0.0, -40.0])
                .order(egui::Order::Foreground)
                .show(ctx, |ui| {
                    kinetic_glass().show(ui, |ui| {
                        ui.set_min_width(300.0);
                        ui.set_max_width(300.0);
                        let r = ui.add_sized([280.0, 26.0], egui::TextEdit::singleline(&mut self.cmd_query).hint_text("Command (Ctrl+K)..."));
                        r.request_focus();
                        if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                            self.cmd_palette_open = false;
                        }
                        let q = self.cmd_query.to_lowercase();
                        let cmds = [
                            ("Toggle Mic (Ctrl+M)", 0u8),
                            ("Switch Cynapse / Ollama", 1),
                            ("Screen Vision (Ctrl+Shift+S)", 2),
                            ("Open CAD Studio", 3),
                            ("Export STL", 4),
                            ("Toggle Pet Companion", 5),
                            ("Toggle Swarm Cursors", 6),
                            ("Run CUA Workflow", 7),
                            ("Synaptic Cascade", 8),
                            ("Clear Unpinned Memory", 9),
                            ("Toggle Hardware HUD", 10),
                            ("Open Cortex Studio (Ctrl+Shift+L)", 11),
                            ("Open Habits RPG", 12),
                            ("Open Engine & TTS Settings", 13),
                        ];
                        let enter = ui.input(|i| i.key_pressed(egui::Key::Enter));
                        let mut picked: Option<u8> = None;
                        for (label, id) in cmds {
                            if (q.is_empty() || label.to_lowercase().contains(&q))
                                && (ui.selectable_label(false, label).clicked() || (enter && picked.is_none()))
                            {
                                picked = Some(id);
                            }
                        }
                        if let Some(id) = picked {
                            self.cmd_palette_open = false;
                            match id {
                                0 => self.toggle_mic(),
                                1 => {
                                    let next = if self.backend_kind == BackendKind::Cynapse { BackendKind::Ollama } else { BackendKind::Cynapse };
                                    self.switch_backend(next);
                                }
                                2 => self.capture_and_describe_screen("What is on my screen right now?".into()),
                                3 => self.active_tab = Some(DrawerTab::Cad),
                                4 => self.export_stl(),
                                5 => self.pet_active = !self.pet_active,
                                6 => self.swarm_active = !self.swarm_active,
                                7 => self.run_cua_workflow(),
                                8 => self.brain.trigger_cascade(),
                                9 => {
                                    let _ = self.store.clear_unpinned();
                                    self.sync_brain();
                                }
                                11 => {
                                    self.studio.open = true;
                                    self.studio.pane = StudioPane::Learn;
                                    self.active_tab = None;
                                }
                                12 => {
                                    self.studio.open = true;
                                    self.studio.pane = StudioPane::Habits;
                                    self.active_tab = None;
                                }
                                13 => {
                                    self.studio.open = true;
                                    self.studio.pane = StudioPane::Engine;
                                    self.active_tab = None;
                                }
                                _ => self.show_hw_hud = !self.show_hw_hud,
                            }
                        }
                    });
                });
        }

        if self.show_hw_hud {
            egui::Area::new(egui::Id::new("hw_telemetry_hud"))
                .anchor(Align2::RIGHT_TOP, [-18.0, 16.0])
                .show(ctx, |ui| {
                    Frame::NONE
                        .fill(Color32::from_rgba_unmultiplied(11, 15, 25, 195))
                        .stroke(Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(255, 255, 255, 28)))
                        .corner_radius(16.0)
                        .inner_margin(egui::Margin::symmetric(12, 6))
                        .show(ui, |ui| {
                            let temp = self
                                .telemetry
                                .cpu_temp_c
                                .map(|c| format!(" · {c:.0}°C"))
                                .unwrap_or_default();
                            ui.small(
                                egui::RichText::new(format!(
                                    "RAM {}/{} MiB{temp} · {:.1} tok/s",
                                    self.telemetry.used_mib, self.telemetry.total_mib, self.last_tok_per_sec
                                ))
                                .monospace()
                                .color(Color32::from_rgb(148, 163, 184)),
                            );
                        });
                });
        }

        // Overlay subprocesses (Pet & Swarm)
        let sp_arg = match self.pet_species {
            PetSpecies::Cat => "cat",
            PetSpecies::Chiba => "chiba",
            PetSpecies::Sheep => "sheep",
        };
        let backend_arg = match self.backend_kind {
            BackendKind::Cynapse => "cynapse",
            BackendKind::Ollama => "ollama",
        };
        let pet_url = self.client.base_url().to_string();
        let pet_model = self.active_model_id();
        self.pet_proc.set_active_args(
            self.pet_active,
            &["--pet-only", sp_arg, backend_arg, &pet_url, &pet_model],
        );
        self.swarm_proc.set_active(self.swarm_active, "--swarm-only");
        if self.pet_active {
            let mood = if self.speech.is_speaking() {
                "talk"
            } else if self.web_rx.is_some() || self.cad_rx.is_some() {
                "scan"
            } else if self.streaming {
                "think"
            } else {
                "idle"
            };
            if mood != self.last_pet_mood {
                self.last_pet_mood = mood;
                set_pet_mood_hint(mood);
            }
        }
    }
}
