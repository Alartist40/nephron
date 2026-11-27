//! Standalone always-on-top overlay processes for Desktop Pet and Swarm Cursors.

use eframe::egui;
use nephron_bridge::{ChatMessage, ChatRole, CynapseClient, EffortLevel, StreamEvent};
use nephron_pets::{widget::PetRenderer, PetCompanion, PetMood, PetSpecies};
use nephron_swarm::{canvas::draw_swarm_overlay, SwarmCanvas};
use std::process::{Child, Command};
use std::time::Duration;
use tokio::runtime::Runtime;
use tokio::sync::mpsc;

#[derive(Default)]
pub struct OverlayChild {
    child: Option<Child>,
}

pub fn set_pet_mood_hint(mood: &str) {
    let p = std::env::temp_dir().join("nephron_pet_mood");
    let _ = std::fs::write(p, mood);
}

impl OverlayChild {
    pub fn set_active_args(&mut self, active: bool, args: &[&str]) {
        if active && self.child.is_none() {
            if let Ok(exe) = std::env::current_exe() {
                let mut cmd = Command::new(exe);
                cmd.args(args);
                if std::env::var_os("DISPLAY").is_some() {
                    cmd.env("WINIT_UNIX_BACKEND", "x11");
                }
                self.child = cmd.spawn().ok();
            }
        } else if !active {
            self.kill();
        }
    }

    pub fn set_active(&mut self, active: bool, flag: &str) {
        self.set_active_args(active, &[flag]);
    }

    pub fn kill(&mut self) {
        if let Some(mut c) = self.child.take() {
            let _ = c.kill();
            let _ = c.wait();
        }
    }
}

pub fn run_pet_only(rt: Runtime, client: CynapseClient, species: PetSpecies, model_id: String) -> eframe::Result<()> {
    let opts = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Nephron Pet")
            .with_transparent(true)
            .with_decorations(false)
            .with_always_on_top()
            .with_inner_size([260.0, 260.0]),
        ..Default::default()
    };
    let mut pet = PetCompanion {
        species,
        ..Default::default()
    };
    pet.say(species.greet(), 3.5);
    let app = PetOnlyApp {
        rt,
        client,
        model_id,
        pet,
        renderer: PetRenderer::default(),
        rx: None,
        buf: String::new(),
        last_hint_check: std::time::Instant::now(),
        last_hint: String::new(),
    };
    eframe::run_native("Nephron Pet", opts, Box::new(move |_| Ok(Box::new(app))))
}

struct PetOnlyApp {
    rt: Runtime,
    client: CynapseClient,
    model_id: String,
    pet: PetCompanion,
    renderer: PetRenderer,
    rx: Option<mpsc::UnboundedReceiver<StreamEvent>>,
    buf: String,
    last_hint_check: std::time::Instant,
    last_hint: String,
}

impl eframe::App for PetOnlyApp {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.0, 0.0, 0.0, 0.0]
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.rx.is_none() && self.pet.min_mood_ttl == 0.0 && self.last_hint_check.elapsed() >= Duration::from_millis(250) {
            self.last_hint_check = std::time::Instant::now();
            if let Ok(h) = std::fs::read_to_string(std::env::temp_dir().join("nephron_pet_mood")) {
                let h = h.trim().to_string();
                if h != self.last_hint {
                    self.last_hint = h.clone();
                    match h.as_str() {
                        "scan" => self.pet.set_mood(PetMood::Scan),
                        "think" => self.pet.set_mood(PetMood::Think),
                        "talk" => self.pet.set_mood(PetMood::Talk),
                        _ => self.pet.set_mood(PetMood::Idle),
                    }
                }
            }
        }
        let mut done = false;
        if let Some(rx) = &mut self.rx {
            while let Ok(ev) = rx.try_recv() {
                match ev {
                    StreamEvent::Delta(d) => {
                        self.buf.push_str(&d);
                        self.pet.set_mood(PetMood::Talk);
                        self.pet.say(self.buf.clone(), 6.0);
                    }
                    StreamEvent::Done { .. } => done = true,
                    StreamEvent::Error(e) => {
                        self.pet.say(format!("({e})"), 4.0);
                        done = true;
                    }
                }
            }
            ctx.request_repaint_after(Duration::from_millis(20));
        }
        if done {
            self.rx = None;
        }

        egui::CentralPanel::default().frame(egui::Frame::NONE).show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                if let Some((sp, q)) = self.renderer.ui(ui, &mut self.pet) {
                    self.ask(sp, q);
                }
            });
        });
    }
}

impl PetOnlyApp {
    fn ask(&mut self, sp: PetSpecies, prompt: String) {
        self.buf.clear();
        let persona = match sp {
            PetSpecies::Cat => "You are a witty, observant desktop Cat companion. Reply in 1 short sentence.",
            PetSpecies::Chiba => "You are a cheerful, loyal desktop Chiba Dog companion. Reply in 1 short sentence.",
            PetSpecies::Sheep => "You are a calm, dreamy desktop Sheep companion. Reply in 1 short sentence.",
        };
        let msgs = vec![
            ChatMessage { role: ChatRole::System, content: persona.into() },
            ChatMessage { role: ChatRole::User, content: prompt },
        ];
        let (tx, rx) = mpsc::unbounded_channel();
        self.rx = Some(rx);
        let client = self.client.clone();
        let model_id = self.model_id.clone();
        self.rt.spawn(async move {
            client.stream_chat_model(&model_id, EffortLevel::None, msgs, tx).await;
        });
    }
}

pub fn run_swarm_only() -> eframe::Result<()> {
    let opts = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Nephron Swarm")
            .with_transparent(true)
            .with_decorations(false)
            .with_always_on_top()
            .with_mouse_passthrough(true)
            .with_maximized(true),
        ..Default::default()
    };
    let mut swarm = SwarmCanvas::default();
    swarm.demo_mode = true;
    swarm.tick(0.016);
    eframe::run_native(
        "Nephron Swarm",
        opts,
        Box::new(move |_| Ok(Box::new(SwarmOnlyApp { swarm }))),
    )
}

struct SwarmOnlyApp {
    swarm: SwarmCanvas,
}

impl eframe::App for SwarmOnlyApp {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.0, 0.0, 0.0, 0.0]
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().frame(egui::Frame::NONE).show(ctx, |ui| {
            let rect = ui.max_rect();
            draw_swarm_overlay(ui, rect, &mut self.swarm);
        });
    }
}
