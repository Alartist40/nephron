//! Interface Studio Modal: Cortex Studio, HabitRPG, and Engine & TTS Settings.

use crate::a2ui::{A2uiAction, A2uiKeyValue, A2uiMetric, A2uiSection, A2uiSurface};
use eframe::egui::{self, Color32, Frame, Stroke};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum StudioPane {
    #[default]
    Learn,
    Habits,
    Engine,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CompactCol {
    Sources,
    #[default]
    Synthesis,
    Deck,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ArtifactStage {
    #[default]
    Empty,
    Flashcards,
    Quiz,
    Audio,
    MindMap,
    Slides,
    Report,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StudioSource {
    pub id: String,
    pub title: String,
    pub kind: String,
    pub active: bool,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StudioChatMsg {
    pub user: bool,
    pub text: String,
    pub citations: Vec<String>,
    #[serde(default)]
    pub surface: Option<A2uiSurface>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Flashcard {
    pub front: String,
    pub back: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuizQuestion {
    pub q: String,
    pub options: Vec<String>,
    pub answer: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DialogueTurn {
    pub speaker: String,
    pub line: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StudioState {
    pub sources: Vec<StudioSource>,
    pub chat: Vec<StudioChatMsg>,
    pub flashcards: Vec<Flashcard>,
    pub quiz: Vec<QuizQuestion>,
    pub audio_script: Vec<DialogueTurn>,
    pub mindmap_lines: Vec<String>,
    pub slides: Vec<String>,
    pub brief_title: String,
    #[serde(skip)]
    pub open: bool,
    #[serde(skip)]
    pub pane: StudioPane,
    #[serde(skip)]
    pub compact_col: CompactCol,
    #[serde(skip)]
    pub stage: ArtifactStage,
    #[serde(skip)]
    pub fc_idx: usize,
    #[serde(skip)]
    pub fc_flipped: bool,
    #[serde(skip)]
    pub quiz_idx: usize,
    #[serde(skip)]
    pub quiz_score: u32,
    #[serde(skip)]
    pub quiz_selected: Option<usize>,
    #[serde(skip)]
    pub quiz_feedback: String,
    #[serde(skip)]
    pub chat_input: String,
    #[serde(skip)]
    pub web_input: String,
    #[serde(skip)]
    pub adding_note: bool,
    #[serde(skip)]
    pub note_title: String,
    #[serde(skip)]
    pub note_body: String,
    #[serde(skip)]
    pub generating: bool,
}

impl Default for StudioState {
    fn default() -> Self {
        Self {
            sources: vec![
                StudioSource {
                    id: "src_1".into(),
                    title: "MindCorpus White Paper MC-W01".into(),
                    kind: "PDF".into(),
                    active: true,
                    content: "MindCorpus stores atomic memory nodes as dendritic stars in SQLite FTS5, projected into a 3D cortical atlas with lobe illumination.".into(),
                },
                StudioSource {
                    id: "src_2".into(),
                    title: "Nephron Local Stack Spec".into(),
                    kind: "NOTE".into(),
                    active: true,
                    content: "Pure-Rust Nephron workspace runs Cynapse/Ollama inference, whisper.cpp STT, pocket-tts offline speech, and SDF CSG CAD on SBCs like Orange Pi.".into(),
                },
            ],
            chat: Vec::new(),
            flashcards: vec![
                Flashcard {
                    front: "How does Nephron link Dendrite memory to the 3D Cortex?".into(),
                    back: "Each SQLite FTS5 memory record maps to a cortical star node; queries and pins trigger synaptic cascades.".into(),
                },
                Flashcard {
                    front: "Why does Nephron use SDF CSG in pure Rust?".into(),
                    back: "Signed Distance Fields + Marching Cubes compile multi-part assemblies locally with zero external OpenSCAD binary.".into(),
                },
                Flashcard {
                    front: "What makes the local voice pipeline privacy-preserving?".into(),
                    back: "Audio capture, whisper.cpp STT, and pocket-tts synthesis all execute locally on-device with no cloud upload.".into(),
                },
            ],
            quiz: vec![
                QuizQuestion {
                    q: "Which local store indexes Nephron's pinned memories and study nodes?".into(),
                    options: vec![
                        "SQLite FTS5 (Dendrite)".into(),
                        "External Redis cluster".into(),
                        "Browser IndexedDB".into(),
                    ],
                    answer: 0,
                },
                QuizQuestion {
                    q: "What happens when a HabitRPG Focus Quest completes?".into(),
                    options: vec![
                        "Nothing changes".into(),
                        "Awards +50 XP, Gold, and triggers a synaptic cascade".into(),
                        "Clears all memories".into(),
                    ],
                    answer: 1,
                },
            ],
            audio_script: vec![
                DialogueTurn {
                    speaker: "Host A".into(),
                    line: "Welcome to Cortex Studio. Your local sources are indexed directly into the 3D brain.".into(),
                },
                DialogueTurn {
                    speaker: "Host B".into(),
                    line: "And every quiz answer or focus quest levels up your Neural Scholar profile offline.".into(),
                },
            ],
            mindmap_lines: vec![
                "MindCorpus Cognitive Canvas".into(),
                "  ├─ 01 · Knowledge Vault (PDF / Notes / Web Search)".into(),
                "  ├─ 02 · Grounded Synthesis -> Atomic Brain Stars".into(),
                "  └─ 03 · Interactive Deck (Flashcards / Quiz / TTS Audio)".into(),
            ],
            slides: vec![
                "Slide 1: Local-First Knowledge Vault with SQLite FTS5".into(),
                "Slide 2: Grounded Synthesis & Socratic Mastery Checks".into(),
                "Slide 3: Gamified HabitRPG & Focus Boss Quests".into(),
            ],
            brief_title: "MindCorpus Executive Brief".into(),
            open: false,
            pane: StudioPane::Learn,
            compact_col: CompactCol::Synthesis,
            stage: ArtifactStage::Empty,
            fc_idx: 0,
            fc_flipped: false,
            quiz_idx: 0,
            quiz_score: 0,
            quiz_selected: None,
            quiz_feedback: "Pick an option".into(),
            chat_input: String::new(),
            web_input: String::new(),
            adding_note: false,
            note_title: String::new(),
            note_body: String::new(),
            generating: false,
        }
    }
}

impl StudioState {
    pub fn active_sources(&self) -> Vec<&StudioSource> {
        self.sources.iter().filter(|s| s.active).collect()
    }

    pub fn active_context_block(&self) -> String {
        let active = self.active_sources();
        if active.is_empty() {
            return "No active sources selected.".into();
        }
        active
            .iter()
            .enumerate()
            .map(|(i, s)| format!("[{}] {} ({}): {}", i + 1, s.title, s.kind, s.content))
            .collect::<Vec<_>>()
            .join("\n\n")
    }

    pub fn add_file_source(&mut self, path: &Path) -> bool {
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("document")
            .to_string();
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();
        let content = if ext == "pdf" {
            extract_pdf_text(path).unwrap_or_else(|| format!("PDF document attached: {name}"))
        } else {
            std::fs::read_to_string(path).unwrap_or_default()
        };
        if content.trim().is_empty() {
            return false;
        }
        let clipped: String = content.chars().take(4000).collect();
        self.sources.insert(
            0,
            StudioSource {
                id: format!("src_{}", self.sources.len() + 1),
                title: name,
                kind: if ext == "pdf" { "PDF" } else { "DOC" }.into(),
                active: true,
                content: clipped,
            },
        );
        true
    }
}

fn extract_pdf_text(path: &Path) -> Option<String> {
    let out = std::process::Command::new("pdftotext")
        .arg(path)
        .arg("-")
        .output()
        .ok()?;
    if out.status.success() {
        let txt = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if !txt.is_empty() {
            return Some(txt);
        }
    }
    None
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpgHabit {
    pub title: String,
    pub pos: u32,
    pub neg: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpgDaily {
    pub title: String,
    pub streak: u32,
    pub done: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpgTodo {
    pub title: String,
    pub difficulty: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpgState {
    pub level: u32,
    pub hp: i32,
    pub max_hp: i32,
    pub xp: i32,
    pub max_xp: i32,
    pub mp: i32,
    pub max_mp: i32,
    pub gold: i32,
    pub habits: Vec<RpgHabit>,
    pub dailies: Vec<RpgDaily>,
    pub todos: Vec<RpgTodo>,
    #[serde(skip)]
    pub new_habit: String,
    #[serde(skip)]
    pub new_daily: String,
    #[serde(skip)]
    pub new_todo: String,
    #[serde(skip)]
    pub toast: String,
    #[serde(skip)]
    pub toast_timer: f32,
}

impl Default for RpgState {
    fn default() -> Self {
        Self {
            level: 7,
            hp: 48,
            max_hp: 50,
            xp: 340,
            max_xp: 500,
            mp: 85,
            max_mp: 100,
            gold: 142,
            habits: vec![
                RpgHabit { title: "Synthesize a new paper or note".into(), pos: 6, neg: 0 },
                RpgHabit { title: "Deep focus without context switching".into(), pos: 11, neg: 2 },
                RpgHabit { title: "Passive doomscrolling".into(), pos: 0, neg: 3 },
            ],
            dailies: vec![
                RpgDaily { title: "Review 5 Cortex Flashcards".into(), streak: 14, done: true },
                RpgDaily { title: "25m Deep Study Quest".into(), streak: 9, done: false },
                RpgDaily { title: "Evening MindCorpus Journal Entry".into(), streak: 21, done: false },
            ],
            todos: vec![
                RpgTodo { title: "Master MindCorpus White Paper MC-W01".into(), difficulty: "Hard".into() },
                RpgTodo { title: "Verify ARM64 release build on Orange Pi".into(), difficulty: "Medium".into() },
            ],
            new_habit: String::new(),
            new_daily: String::new(),
            new_todo: String::new(),
            toast: String::new(),
            toast_timer: 0.0,
        }
    }
}

impl RpgState {
    pub fn scholar_title(&self) -> &'static str {
        match self.level {
            0..=4 => "Cortex Apprentice",
            5..=9 => "Cortex Mage",
            10..=19 => "Synapse Architect",
            _ => "MindCorpus Sage",
        }
    }

    pub fn award_xp(&mut self, xp_gain: i32, gold_gain: i32, msg: &str) -> bool {
        self.xp += xp_gain;
        self.gold += gold_gain;
        self.mp = (self.mp + 4).min(self.max_mp);
        let mut leveled = false;
        while self.xp >= self.max_xp {
            self.xp -= self.max_xp;
            self.level += 1;
            self.max_xp = (self.max_xp as f32 * 1.15) as i32;
            self.hp = self.max_hp;
            self.mp = self.max_mp;
            leveled = true;
        }
        self.toast = if leveled {
            format!("★ LEVEL UP! Lv {}", self.level)
        } else {
            msg.to_string()
        };
        self.toast_timer = 2.6;
        leveled
    }

    pub fn damage_hp(&mut self, dmg: i32, msg: &str) {
        self.hp = (self.hp - dmg).max(5);
        self.toast = msg.to_string();
        self.toast_timer = 2.2;
    }

    pub fn tick(&mut self, dt: f32) {
        if self.toast_timer > 0.0 {
            self.toast_timer = (self.toast_timer - dt).max(0.0);
            if self.toast_timer == 0.0 {
                self.toast.clear();
            }
        }
    }
}

pub fn bento_frame() -> Frame {
    Frame::NONE
        .fill(Color32::from_rgba_unmultiplied(16, 22, 36, 195))
        .stroke(Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(255, 255, 255, 24)))
        .corner_radius(16.0)
        .inner_margin(10.0)
}

pub fn draw_stat_bar(ui: &mut egui::Ui, label: &str, val: i32, max: i32, col: Color32) {
    Frame::NONE
        .fill(Color32::from_rgba_unmultiplied(6, 10, 18, 175))
        .stroke(Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(255, 255, 255, 22)))
        .corner_radius(12.0)
        .inner_margin(egui::Margin::symmetric(10, 6))
        .show(ui, |ui| {
            ui.set_min_width(98.0);
            ui.horizontal(|ui| {
                ui.small(egui::RichText::new(label).monospace().color(col));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.small(egui::RichText::new(format!("{val}/{max}")).monospace().color(Color32::from_rgb(203, 213, 225)));
                });
            });
            let frac = (val as f32 / max.max(1) as f32).clamp(0.0, 1.0);
            let (rect, _) = ui.allocate_exact_size(egui::vec2(96.0, 5.0), egui::Sense::hover());
            ui.painter().rect_filled(rect, 2.5, Color32::from_rgba_unmultiplied(255, 255, 255, 24));
            let fill = egui::Rect::from_min_size(rect.min, egui::vec2(rect.width() * frac, rect.height()));
            ui.painter().rect_filled(fill, 2.5, col);
        });
}

pub fn techart_tag(ui: &mut egui::Ui, text: &str, col: Color32) {
    Frame::NONE
        .fill(Color32::from_rgba_unmultiplied(col.r(), col.g(), col.b(), 22))
        .stroke(Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(col.r(), col.g(), col.b(), 65)))
        .corner_radius(99.0)
        .inner_margin(egui::Margin::symmetric(8, 2))
        .show(ui, |ui| {
            ui.label(egui::RichText::new(text).monospace().size(9.5).color(col));
        });
}

pub fn parse_studio_synthesis(raw: &str, studio: &mut StudioState) -> Vec<(String, String)> {
    let mut answer = String::new();
    let mut nodes: Vec<(String, String)> = Vec::new();
    let mut new_cards: Vec<Flashcard> = Vec::new();
    let mut new_quiz: Vec<QuizQuestion> = Vec::new();
    let mut new_audio: Vec<DialogueTurn> = Vec::new();
    let mut new_map: Vec<String> = Vec::new();
    let mut new_slides: Vec<String> = Vec::new();

    if let Some(json_str) = extract_json_block(raw) {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&json_str) {
            if let Some(a) = v.get("answer").and_then(|x| x.as_str()) {
                answer = a.to_string();
            }
            if let Some(arr) = v.get("nodes").and_then(|x| x.as_array()) {
                for item in arr {
                    let t = item.get("title").and_then(|x| x.as_str()).unwrap_or("").trim();
                    let b = item.get("body").and_then(|x| x.as_str()).unwrap_or("").trim();
                    if !t.is_empty() {
                        nodes.push((t.to_string(), b.to_string()));
                    }
                }
            }
            if let Some(arr) = v.get("flashcards").and_then(|x| x.as_array()) {
                for item in arr {
                    let f = item.get("front").and_then(|x| x.as_str()).unwrap_or("").trim();
                    let b = item.get("back").and_then(|x| x.as_str()).unwrap_or("").trim();
                    if !f.is_empty() && !b.is_empty() {
                        new_cards.push(Flashcard { front: f.to_string(), back: b.to_string() });
                    }
                }
            }
            if let Some(arr) = v.get("quiz").and_then(|x| x.as_array()) {
                for item in arr {
                    let q = item.get("q").and_then(|x| x.as_str()).unwrap_or("").trim();
                    let opts: Vec<String> = item
                        .get("options")
                        .and_then(|x| x.as_array())
                        .map(|o| o.iter().filter_map(|s| s.as_str().map(str::to_string)).collect())
                        .unwrap_or_default();
                    let ans = item.get("answer").and_then(|x| x.as_u64()).unwrap_or(0) as usize;
                    if !q.is_empty() && opts.len() >= 2 {
                        new_quiz.push(QuizQuestion {
                            q: q.to_string(),
                            options: opts,
                            answer: ans,
                        });
                    }
                }
            }
            if let Some(arr) = v.get("audioScript").and_then(|x| x.as_array()) {
                for item in arr {
                    let sp = item.get("speaker").and_then(|x| x.as_str()).unwrap_or("Host A");
                    let ln = item.get("line").and_then(|x| x.as_str()).unwrap_or("").trim();
                    if !ln.is_empty() {
                        new_audio.push(DialogueTurn { speaker: sp.to_string(), line: ln.to_string() });
                    }
                }
            }
            if let Some(arr) = v.get("mindmap").and_then(|x| x.as_array()) {
                new_map = arr.iter().filter_map(|s| s.as_str().map(str::to_string)).collect();
            }
            if let Some(arr) = v.get("slides").and_then(|x| x.as_array()) {
                new_slides = arr.iter().filter_map(|s| s.as_str().map(str::to_string)).collect();
            }
        }
    }

    if answer.trim().is_empty() {
        answer = raw.trim().to_string();
    }
    if !new_cards.is_empty() {
        studio.flashcards = new_cards;
        studio.fc_idx = 0;
        studio.fc_flipped = false;
    }
    if !new_quiz.is_empty() {
        studio.quiz = new_quiz;
        studio.quiz_idx = 0;
        studio.quiz_score = 0;
        studio.quiz_selected = None;
        studio.quiz_feedback = "Pick an option".into();
    }
    if !new_audio.is_empty() {
        studio.audio_script = new_audio;
    }
    if !new_map.is_empty() {
        studio.mindmap_lines = new_map;
    }
    if !new_slides.is_empty() {
        studio.slides = new_slides;
    }

    let cites: Vec<String> = studio.active_sources().iter().take(2).map(|s| s.title.clone()).collect();
    let surface = if !nodes.is_empty() {
        Some(A2uiSurface {
            title: "Synthesis Dashboard".into(),
            subtitle: Some(answer.clone()),
            badge: Some("A2UI".into()),
            metrics: vec![
                A2uiMetric {
                    label: "Brain Nodes".into(),
                    value: nodes.len().to_string(),
                    delta: Some("planted".into()),
                    tone: Some("amber".into()),
                },
                A2uiMetric {
                    label: "Flashcards".into(),
                    value: studio.flashcards.len().to_string(),
                    delta: Some("ready".into()),
                    tone: Some("sky".into()),
                },
                A2uiMetric {
                    label: "Quiz".into(),
                    value: studio.quiz.len().to_string(),
                    delta: Some("+XP".into()),
                    tone: Some("emerald".into()),
                },
            ],
            sections: vec![A2uiSection {
                title: "Extracted Memory Nodes".into(),
                items: nodes
                    .iter()
                    .map(|(k, v)| A2uiKeyValue {
                        key: k.clone(),
                        value: v.clone(),
                    })
                    .collect(),
            }],
            checklist: Vec::new(),
            table: None,
            actions: vec![
                A2uiAction {
                    label: "Open Flashcards".into(),
                    prompt: None,
                    kind: Some("stage:flashcards".into()),
                },
                A2uiAction {
                    label: "Take Quiz (+XP)".into(),
                    prompt: None,
                    kind: Some("stage:quiz".into()),
                },
                A2uiAction {
                    label: "Concept Tree".into(),
                    prompt: None,
                    kind: Some("stage:mindmap".into()),
                },
            ],
        })
    } else {
        A2uiSurface::parse_from_text(&answer).or_else(|| A2uiSurface::from_markdown_heuristic(&answer))
    };
    studio.chat.push(StudioChatMsg {
        user: false,
        text: answer,
        citations: cites,
        surface,
    });
    nodes
}

fn extract_json_block(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.starts_with('{') && trimmed.ends_with('}') {
        return Some(trimmed.to_string());
    }
    if let Some(start) = trimmed.find("```json") {
        let rest = &trimmed[start + 7..];
        if let Some(end) = rest.find("```") {
            return Some(rest[..end].trim().to_string());
        }
    }
    let start = trimmed.find('{')?;
    let end = trimmed.rfind('}')?;
    (end > start).then(|| trimmed[start..=end].to_string())
}

pub fn pick_file_dialog() -> Option<std::path::PathBuf> {
    if let Ok(out) = std::process::Command::new("zenity")
        .args([
            "--file-selection",
            "--title=Add Document to Knowledge Vault",
            "--file-filter=Study Files | *.pdf *.txt *.md *.csv *.json *.rs *.py",
        ])
        .output()
    {
        if out.status.success() {
            let p = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if !p.is_empty() {
                return Some(std::path::PathBuf::from(p));
            }
        }
    }
    if let Ok(out) = std::process::Command::new("kdialog")
        .args(["--getopenfilename", ".", "*.pdf *.txt *.md *.csv *.json"])
        .output()
    {
        if out.status.success() {
            let p = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if !p.is_empty() {
                return Some(std::path::PathBuf::from(p));
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rpg_awards_xp_and_levels_up() {
        let mut rpg = RpgState::default();
        let start_lv = rpg.level;
        let leveled = rpg.award_xp(200, 10, "quest");
        assert!(leveled);
        assert_eq!(rpg.level, start_lv + 1);
        assert_eq!(rpg.gold, 152);
    }

    #[test]
    fn parses_studio_json_and_updates_artifacts() {
        let mut st = StudioState::default();
        let raw = r#"```json
        {
          "answer": "Grounded summary.",
          "nodes": [{"title": "Node A", "body": "Detail A"}],
          "flashcards": [{"front": "Q1", "back": "A1"}],
          "quiz": [{"q": "Pick 1", "options": ["X", "Y"], "answer": 1}]
        }
        ```"#;
        let nodes = parse_studio_synthesis(raw, &mut st);
        assert_eq!(nodes.len(), 1);
        assert_eq!(st.flashcards.len(), 1);
        assert_eq!(st.quiz.len(), 1);
        assert_eq!(st.chat.last().unwrap().text, "Grounded summary.");
    }
}
