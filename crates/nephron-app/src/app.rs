//! Full-window 3D Brain Atlas + `gods-eye-view` OS-style bottom command dock.

use crate::brain::{BrainActivity, BrainAtlas};
use crate::overlays::{set_pet_mood_hint, OverlayChild};
use eframe::egui::{self, Align2, Color32, Frame, Stroke};
use nephron_bridge::{
    format_citations_prompt, BackendKind, CadPart, ChatMessage, ChatRole, CynapseClient,
    EffortLevel, MemoryRecord, MemoryStore, ModelProfile, SearchResult, StreamEvent,
    SystemTelemetry, WebSearcher,
};
use nephron_cad::{
    cad_repair_prompt, compile_cad_bundle, parse_cad_bundle, sanitize_part_name, CadDesign,
    CadViewport, CAD_SYSTEM_PROMPT,
};
use nephron_pets::PetSpecies;
use nephron_swarm::actions::{execute_action, ComputerAction};
use nephron_voice::{
    audio::SAMPLE_RATE, encode_pcm16_wav,
    tts::{DEFAULT_TTS_URL, DEFAULT_VOICE},
    wav_to_base64, AudioCapture, PocketTtsClient, PocketTtsSidecar, SentenceBuffer, SpeechPipeline,
};
use std::path::PathBuf;
use std::time::{Duration, Instant};
use tokio::runtime::Runtime;
use tokio::sync::mpsc;

#[derive(Default)]
struct DockState {
    chat: bool,
    brain: bool,
    cad: bool,
    backend: bool,
    pet_menu: bool,
    pet: bool,
    swarm: bool,
    web: bool,
}

pub struct NephronApp {
    rt: Runtime,
    client: CynapseClient,
    backend_kind: BackendKind,
    ollama_models: Vec<String>,
    selected_ollama: String,
    ollama_rx: Option<mpsc::UnboundedReceiver<Vec<String>>>,
    store: MemoryStore,
    model: ModelProfile,
    effort: EffortLevel,
    dock: DockState,
    messages: Vec<ChatMessage>,
    input: String,
    memory_query: String,
    memory_hits: Vec<MemoryRecord>,
    streaming: bool,
    stream_rx: Option<mpsc::UnboundedReceiver<StreamEvent>>,
    health_rx: Option<mpsc::UnboundedReceiver<bool>>,
    daemon_online: bool,
    last_tok_per_sec: f32,
    telemetry: SystemTelemetry,
    last_poll: Instant,
    _tts_sidecar: PocketTtsSidecar,
    speech: SpeechPipeline,
    sentence_buf: SentenceBuffer,
    auto_tts: bool,
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
    brain: BrainAtlas,
    searcher: WebSearcher,
    web_sources: Vec<SearchResult>,
    web_rx: Option<mpsc::UnboundedReceiver<(String, Vec<SearchResult>)>>,
}

impl NephronApp {
    pub fn new(rt: Runtime, client: CynapseClient, store: MemoryStore) -> Self {
        let tts_url = std::env::var("NEPHRON_TTS_URL").unwrap_or_else(|_| DEFAULT_TTS_URL.into());
        let _tts_sidecar = PocketTtsSidecar::spawn(8000, DEFAULT_VOICE);
        let tts = PocketTtsClient::new(tts_url).expect("valid pocket-tts url");
        let speech = SpeechPipeline::spawn(tts.clone(), DEFAULT_VOICE.into());
        let cad_parts = store.list_parts().unwrap_or_default();
        let memories = store.recent_memories(40).unwrap_or_default();
        let mut brain = BrainAtlas::default();
        brain.build_anatomy(&memories);
        let mut app = Self {
            rt,
            client,
            backend_kind: BackendKind::Cynapse,
            ollama_models: Vec::new(),
            selected_ollama: "llama3.2".into(),
            ollama_rx: None,
            store,
            model: ModelProfile::MiniCpm5_1B,
            effort: EffortLevel::Auto,
            dock: DockState { chat: true, ..Default::default() },
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
            auto_tts: true,
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
            brain,
            searcher: WebSearcher::default(),
            web_sources: Vec::new(),
            web_rx: None,
        };
        app.poll_system();
        app
    }

    fn switch_backend(&mut self, kind: BackendKind) {
        self.backend_kind = kind;
        if let Ok(c) = CynapseClient::with_kind(kind.default_url(), kind) {
            self.client = c;
            self.poll_system();
            if kind == BackendKind::Ollama {
                self.refresh_ollama();
            }
            if self.dock.pet {
                self.pet_proc.kill();
            }
        }
    }

    fn refresh_ollama(&mut self) {
        let client = self.client.clone();
        let (tx, rx) = mpsc::unbounded_channel();
        self.ollama_rx = Some(rx);
        self.rt.spawn(async move {
            let _ = tx.send(client.list_ollama_models().await);
        });
    }

    fn active_model_id(&self) -> String {
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

    fn sync_brain(&mut self) {
        let mems = self.store.recent_memories(40).unwrap_or_default();
        self.brain.build_anatomy(&mems);
        self.memory_hits = mems;
    }

    fn send_prompt(&mut self) {
        let prompt = self.input.trim().to_string();
        if prompt.is_empty() || self.streaming {
            return;
        }
        self.input.clear();
        let lower = prompt.to_lowercase();
        let cad_intent = (self.dock.cad && !self.dock.web)
            || lower.starts_with("build ")
            || lower.starts_with("design ")
            || lower.starts_with("cad:")
            || lower.starts_with("3d model ")
            || lower.contains("3d print");
        if cad_intent {
            self.dock.cad = true;
            self.dock.chat = true;
            let _ = self.store.save_memory("cad", &prompt);
            self.sync_brain();
            self.messages.push(ChatMessage { role: ChatRole::User, content: prompt.clone() });
            self.generate_cad(prompt);
            return;
        }
        let web_intent = self.dock.web
            || lower.starts_with("search ")
            || lower.starts_with("research ")
            || lower.starts_with("look up ")
            || lower.contains("latest news");
        let _ = self.store.save_memory("user", &prompt);
        self.sync_brain();
        self.dock.chat = true;
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
        if let Some(c) = web_ctx {
            history.insert(history.len().saturating_sub(1), ChatMessage { role: ChatRole::System, content: c });
        }
        self.rt.spawn(async move {
            client.stream_chat_model(&model_id, effort, history, tx).await;
        });
    }

    fn toggle_mic(&mut self) {
        if let Some(cap) = self.capture.take() {
            let samples = cap.stop();
            if !samples.is_empty() && !self.streaming {
                let b64 = wav_to_base64(&encode_pcm16_wav(&samples, SAMPLE_RATE));
                let _ = self.store.save_memory("voice", "[voice audio prompt]");
                self.sync_brain();
                self.dock.chat = true;
                self.messages.push(ChatMessage { role: ChatRole::User, content: "[Voice prompt]".into() });
                self.messages.push(ChatMessage { role: ChatRole::Assistant, content: String::new() });
                let (tx, rx) = mpsc::unbounded_channel();
                self.stream_rx = Some(rx);
                self.streaming = true;
                let client = self.client.clone();
                let effort = self.effort;
                let model_id = match self.backend_kind {
                    BackendKind::Cynapse => ModelProfile::Lfm3Omni2B.id().to_string(),
                    BackendKind::Ollama => self.selected_ollama.clone(),
                };
                self.rt.spawn(async move {
                    client.stream_audio_prompt_model(&model_id, effort, b64, tx).await;
                });
            }
        } else if !self.streaming {
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
            self.cad_parts = self.store.list_parts().unwrap_or_default();
            let summary = format!(
                "Built {} ({tris} tris, {:.0}×{:.0}×{:.0} mm via {}).",
                self.cad_part_name.replace('_', " "),
                dims[0],
                dims[1],
                dims[2],
                self.cad_viewport.backend_label
            );
            self.messages.push(ChatMessage {
                role: ChatRole::Assistant,
                content: summary.clone(),
            });
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
        self.cad_design = None;
        self.cad_scad.clear();
        let slug: String = sanitize_part_name(&desc)
            .chars()
            .take(28)
            .collect();
        self.cad_part_name = slug.to_lowercase();
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
                            Some(d) => d.validate().err().map(|e| e.to_string()),
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
    }
}

fn hud_frame() -> Frame {
    Frame::NONE
        .fill(Color32::from_rgba_unmultiplied(10, 14, 24, 198))
        .stroke(Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(255, 255, 255, 28)))
        .corner_radius(16.0)
        .inner_margin(12.0)
}

fn dock_frame() -> Frame {
    Frame::NONE
        .fill(Color32::from_rgba_unmultiplied(12, 16, 28, 185))
        .stroke(Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(255, 255, 255, 30)))
        .corner_radius(18.0)
        .inner_margin(egui::Margin::symmetric(12, 8))
}

fn dock_icon(ui: &mut egui::Ui, active: bool, icon: &str, tip: &str, tint: Color32) -> bool {
    let fill = if active {
        Color32::from_rgba_unmultiplied(56, 189, 248, 48)
    } else {
        Color32::from_rgba_unmultiplied(255, 255, 255, 10)
    };
    let stroke = if active {
        Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(56, 189, 248, 110))
    } else {
        Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(255, 255, 255, 24))
    };
    let btn = egui::Button::new(egui::RichText::new(icon).size(18.0).color(tint))
        .fill(fill)
        .stroke(stroke)
        .corner_radius(12.0)
        .min_size(egui::vec2(42.0, 42.0));
    let resp = ui.add(btn).on_hover_text(tip);
    if active {
        let dot_pos = egui::pos2(resp.rect.center().x, resp.rect.bottom() + 3.0);
        ui.painter().circle_filled(dot_pos, 2.0, Color32::from_rgb(56, 189, 248));
    }
    resp.clicked()
}

impl eframe::App for NephronApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.last_poll.elapsed() >= Duration::from_secs(3) {
            self.poll_system();
        }
        if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::M)) {
            self.toggle_mic();
        }
        self.drain_events(ctx);

        // Full-window stage: 3D CAD Studio OR MindCorpus 3D Glitter Brain
        egui::CentralPanel::default()
            .frame(Frame::NONE.fill(Color32::from_rgb(3, 4, 7)))
            .show(ctx, |ui| {
                let rect = ui.max_rect();
                if self.dock.cad {
                    self.cad_viewport.ui_rect(ui, rect);
                } else {
                    let dt = ctx.input(|i| i.stable_dt).min(0.05);
                    let (act, energy) = if let Some(cap) = &self.capture {
                        (BrainActivity::Listening, (cap.current_rms() * 6.5).clamp(0.1, 1.0))
                    } else if self.streaming {
                        let empty = self.messages.last().map(|m| m.content.is_empty()).unwrap_or(true);
                        if empty { (BrainActivity::Thinking, 0.45) } else { (BrainActivity::Speaking, 0.7) }
                    } else if self.speech.is_speaking() {
                        (BrainActivity::Speaking, 0.75)
                    } else if !self.input.is_empty() {
                        (BrainActivity::Idle, 0.2)
                    } else {
                        (BrainActivity::Idle, 0.05)
                    };
                    let (clicked_mem, interacting) = self.brain.paint(ui, rect, dt, act, energy);
                    if let Some(mem) = clicked_mem {
                        self.dock.brain = true;
                        self.memory_query = mem.clone();
                        self.memory_hits = self.store.search_fts(&mem, 12).unwrap_or_default();
                        if self.input.trim().is_empty() {
                            self.input = mem;
                        }
                    }
                    if interacting {
                        ctx.request_repaint();
                    } else {
                        let active = self.capture.is_some()
                            || self.streaming
                            || self.cad_rx.is_some()
                            || self.web_rx.is_some()
                            || self.speech.is_speaking()
                            || self.brain.auto_rotate
                            || !self.input.is_empty();
                        let wait_ms = if active { 33 } else { 150 };
                        ctx.request_repaint_after(Duration::from_millis(wait_ms));
                    }
                }
            });

        // Top-left minimal borderless status pill
        egui::Area::new(egui::Id::new("hud_status"))
            .anchor(Align2::LEFT_TOP, [16.0, 14.0])
            .show(ctx, |ui| {
                hud_frame().show(ui, |ui| {
                    ui.horizontal(|ui| {
                        let dot = if self.daemon_online { Color32::from_rgb(85, 230, 196) } else { Color32::from_rgb(230, 140, 110) };
                        ui.colored_label(dot, "●");
                        ui.label(format!("{} · {} · {:.1} GiB free", self.backend_kind.label(), self.active_model_id(), self.telemetry.available_mib as f32 / 1024.0));
                    });
                });
            });

        // Floating HUD drawers (Chat, Brain Memory, CAD, Backend)
        if self.dock.chat {
            egui::Area::new(egui::Id::new("hud_chat"))
                .anchor(Align2::LEFT_BOTTOM, [14.0, -110.0])
                .show(ctx, |ui| {
                    hud_frame().show(ui, |ui| {
                        ui.set_max_width(420.0);
                        ui.set_max_height(280.0);
                        egui::ScrollArea::vertical().stick_to_bottom(true).show(ui, |ui| {
                            for m in &self.messages {
                                let who = match m.role {
                                    ChatRole::User => "You",
                                    ChatRole::Assistant => "Nephron",
                                    ChatRole::System => "System",
                                };
                                ui.strong(who);
                                ui.label(&m.content);
                                ui.add_space(4.0);
                            }
                            if !self.web_sources.is_empty() {
                                ui.separator();
                                ui.horizontal(|ui| {
                                    ui.small("Sources");
                                    if ui.small_button("Clear").clicked() {
                                        self.web_sources.clear();
                                    }
                                });
                                ui.horizontal_wrapped(|ui| {
                                    for (i, s) in self.web_sources.iter().enumerate() {
                                        let domain = s
                                            .url
                                            .split("//")
                                            .nth(1)
                                            .and_then(|rest| rest.split('/').next())
                                            .unwrap_or("web")
                                            .trim_start_matches("www.");
                                        let short_title: String = s.title.chars().take(28).collect();
                                        Frame::NONE
                                            .fill(Color32::from_rgba_unmultiplied(24, 32, 48, 220))
                                            .stroke(Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(125, 211, 252, 45)))
                                            .corner_radius(8.0)
                                            .inner_margin(egui::Margin::symmetric(8, 5))
                                            .show(ui, |ui| {
                                                ui.set_max_width(185.0);
                                                ui.hyperlink_to(
                                                    egui::RichText::new(format!("[{}] {}", i + 1, short_title)).size(11.5),
                                                    &s.url,
                                                )
                                                .on_hover_text(format!("{}\n{}", s.title, s.snippet));
                                                ui.small(egui::RichText::new(domain).color(Color32::from_rgb(148, 163, 184)).size(10.0));
                                            });
                                    }
                                });
                            }
                        });
                    });
                });
        }

        if self.dock.brain {
            egui::Area::new(egui::Id::new("hud_brain"))
                .anchor(Align2::RIGHT_TOP, [-14.0, 12.0])
                .show(ctx, |ui| {
                    hud_frame().show(ui, |ui| {
                        ui.set_width(270.0);
                        ui.set_max_height(320.0);
                        ui.horizontal(|ui| {
                            ui.strong("Dendrite Brain DB");
                            if ui.small_button("Clear Unpinned").clicked() {
                                let _ = self.store.clear_unpinned();
                                self.sync_brain();
                            }
                        });
                        ui.horizontal(|ui| {
                            if ui.text_edit_singleline(&mut self.memory_query).lost_focus()
                                && ui.input(|i| i.key_pressed(egui::Key::Enter))
                            {
                                self.memory_hits = self.store.search_fts(&self.memory_query, 12).unwrap_or_default();
                            }
                            if ui.button("Search").clicked() {
                                self.memory_hits = self.store.search_fts(&self.memory_query, 12).unwrap_or_default();
                            }
                        });
                        let mut toggle_pin: Option<(i64, bool)> = None;
                        egui::ScrollArea::vertical().show(ui, |ui| {
                            for h in &self.memory_hits {
                                ui.horizontal(|ui| {
                                    let pin_lbl = if h.pinned { "📌" } else { "·" };
                                    if ui.small_button(pin_lbl).clicked() {
                                        toggle_pin = Some((h.id, !h.pinned));
                                    }
                                    ui.small(format!("[{}] {}", h.kind, h.content));
                                });
                                ui.separator();
                            }
                        });
                        if let Some((id, pinned)) = toggle_pin {
                            let _ = self.store.pin_memory(id, pinned);
                            self.sync_brain();
                        }
                    });
                });
        }

        if self.dock.cad {
            egui::Area::new(egui::Id::new("hud_cad"))
                .anchor(Align2::RIGHT_BOTTOM, [-16.0, -110.0])
                .show(ctx, |ui| {
                    hud_frame().show(ui, |ui| {
                        ui.set_width(300.0);
                        ui.horizontal(|ui| {
                            ui.add_sized([110.0, 22.0], egui::TextEdit::singleline(&mut self.cad_part_name));
                            if ui.button("Compile").clicked() { self.compile_cad(); }
                            if ui.button("Gen AI").clicked() {
                                let d = if self.input.trim().is_empty() { self.cad_part_name.clone() } else { self.input.clone() };
                                self.generate_cad(d);
                            }
                        });
                        ui.horizontal(|ui| {
                            ui.checkbox(&mut self.cad_viewport.wireframe, "Wire");
                            if ui.button("Export STL").clicked() {
                                self.export_stl();
                            }
                            if let Some(p) = &self.last_export_path {
                                if ui.button("Slicer").clicked() {
                                    let _ = execute_action(&ComputerAction::OpenPath(p.clone()));
                                }
                            }
                        });
                        ui.add(egui::TextEdit::multiline(&mut self.cad_scad).desired_rows(3).code_editor());
                        for p in self.cad_parts.iter().take(3) {
                            ui.small(format!("{} x{} ({})", p.name, p.quantity, p.material));
                        }
                    });
                });
        }

        if self.dock.backend {
            egui::Area::new(egui::Id::new("hud_backend"))
                .anchor(Align2::CENTER_BOTTOM, [0.0, -115.0])
                .show(ctx, |ui| {
                    hud_frame().show(ui, |ui| {
                        ui.horizontal(|ui| {
                            if ui.selectable_label(self.backend_kind == BackendKind::Cynapse, "Cynapse :38265").clicked() {
                                self.switch_backend(BackendKind::Cynapse);
                            }
                            if ui.selectable_label(self.backend_kind == BackendKind::Ollama, "Ollama :11434").clicked() {
                                self.switch_backend(BackendKind::Ollama);
                            }
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
                                        ui.selectable_value(&mut self.selected_ollama, m.clone(), m);
                                    }
                                });
                                if ui.button("Refresh").clicked() { self.refresh_ollama(); }
                            }
                            egui::ComboBox::from_id_salt("eff").selected_text(self.effort.as_str()).show_ui(ui, |ui| {
                                for e in EffortLevel::ALL {
                                    ui.selectable_value(&mut self.effort, e, e.as_str());
                                }
                            });
                            ui.checkbox(&mut self.auto_tts, "TTS");
                        });
                    });
                });
        }

        if self.dock.pet_menu {
            egui::Area::new(egui::Id::new("hud_pet_picker"))
                .anchor(Align2::CENTER_BOTTOM, [0.0, -115.0])
                .show(ctx, |ui| {
                    hud_frame().show(ui, |ui| {
                        ui.horizontal(|ui| {
                            for sp in [PetSpecies::Cat, PetSpecies::Chiba, PetSpecies::Sheep] {
                                if ui.selectable_label(self.pet_species == sp, sp.name()).clicked() {
                                    self.pet_species = sp;
                                    self.dock.pet = true;
                                    self.pet_proc.kill();
                                }
                            }
                            ui.separator();
                            let lbl = if self.dock.pet { "Hide Pet" } else { "Spawn Pet" };
                            if ui.button(lbl).clicked() {
                                self.dock.pet = !self.dock.pet;
                            }
                        });
                    });
                });
        }

        // OS-style Bottom Application Dock (`mindcorpus_neural_canvas_3d_brain_explorer.html` layout)
        egui::Area::new(egui::Id::new("os_dock"))
            .anchor(Align2::CENTER_BOTTOM, [0.0, -16.0])
            .show(ctx, |ui| {
                dock_frame().show(ui, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.horizontal(|ui| {
                            let resp = ui.add_sized([390.0, 26.0], egui::TextEdit::singleline(&mut self.input).hint_text("Ask Nephron..."));
                            let enter = resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                            if (ui.add_enabled(!self.streaming, egui::Button::new("Send")).clicked() || enter) && !self.streaming {
                                self.send_prompt();
                            }
                        });
                        ui.add_space(6.0);
                        ui.horizontal(|ui| {
                            if dock_icon(ui, self.dock.chat, "💬", "Chat", Color32::from_rgb(226, 232, 240)) {
                                self.dock.chat = !self.dock.chat;
                            }
                            if dock_icon(ui, self.dock.brain, "◈", "Dendrite Brain DB", Color32::from_rgb(165, 180, 252)) {
                                self.dock.brain = !self.dock.brain;
                            }
                            let mic_active = self.capture.is_some();
                            if dock_icon(ui, mic_active, "◉", "Voice (Ctrl+M)", Color32::from_rgb(56, 189, 248)) {
                                self.toggle_mic();
                            }
                            if dock_icon(ui, self.dock.pet, "🐾", "Choose Desktop Pet (Cat / Chiba / Sheep)", Color32::from_rgb(110, 231, 183)) {
                                self.dock.pet_menu = !self.dock.pet_menu;
                                if self.dock.pet_menu { self.dock.backend = false; }
                            }
                            if dock_icon(ui, self.dock.swarm, "✨", "Desktop Swarm Cursors", Color32::from_rgb(251, 191, 36)) {
                                self.dock.swarm = !self.dock.swarm;
                            }
                            if self.dock.swarm && dock_icon(ui, false, "⚡", "Run CUA Capture & Notify", Color32::from_rgb(251, 191, 36)) {
                                let shot = std::env::temp_dir().join("nephron_cua_screen.png");
                                let _ = execute_action(&ComputerAction::CaptureScreen(shot));
                                let _ = execute_action(&ComputerAction::Notify {
                                    title: "Nephron CUA".into(),
                                    body: "Swarm desktop observation active".into(),
                                });
                            }
                            if dock_icon(ui, self.dock.cad, "📐", "3D CAD Studio", Color32::from_rgb(125, 211, 252)) {
                                self.dock.cad = !self.dock.cad;
                            }
                            if dock_icon(ui, self.dock.web, "🌐", "Perplexica Web Search", Color32::from_rgb(192, 132, 252)) {
                                self.dock.web = !self.dock.web;
                            }
                            if dock_icon(ui, false, "🎨", "Shift Brain Spectrum", Color32::from_rgb(216, 180, 254)) {
                                self.brain.palette_idx = (self.brain.palette_idx + 1) % 3;
                                self.brain.trigger_cascade();
                            }
                            if dock_icon(ui, self.dock.backend, "⚙", "Backend (Cynapse / Ollama)", Color32::from_rgb(148, 163, 184)) {
                                self.dock.backend = !self.dock.backend;
                                if self.dock.backend { self.dock.pet_menu = false; }
                            }
                        });
                    });
                });
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
                    self.dock.pet,
                    &["--pet-only", sp_arg, backend_arg, &pet_url, &pet_model],
                );
                self.swarm_proc.set_active(self.dock.swarm, "--swarm-only");
                if self.dock.pet {
                    let mood = if self.web_rx.is_some() || self.cad_rx.is_some() || self.dock.swarm {
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
            });
    }
}
