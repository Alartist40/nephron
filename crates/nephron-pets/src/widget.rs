//! Minimalist floating desktop pet with cached PNG frames & speech bubbles.

use crate::atlas::{decode_frame, frames_for};
use crate::{PetCompanion, PetMood, PetSpecies};
use eframe::egui::{self, Color32, Frame, Stroke, TextureHandle, TextureOptions, ViewportCommand};
use std::collections::HashMap;
use std::time::Duration;

#[derive(Default)]
pub struct PetRenderer {
    cache: HashMap<(PetSpecies, PetMood, usize), TextureHandle>,
    pub pet_prompt: String,
}

impl PetRenderer {
    /// Draws the floating pet sprite + optional reply/input speech bubbles.
    pub fn ui(&mut self, ui: &mut egui::Ui, pet: &mut PetCompanion) -> Option<(PetSpecies, String)> {
        let mut submitted = None;
        let dt = ui.input(|i| i.stable_dt).min(0.1);
        pet.tick(dt);
        ui.ctx().request_repaint_after(Duration::from_millis(60));

        // 1. Separate reply speech bubble above the pet
        if let Some(msg) = &pet.bubble {
            Frame::NONE
                .fill(Color32::from_rgba_unmultiplied(10, 14, 24, 232))
                .stroke(Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(56, 189, 248, 120)))
                .corner_radius(14.0)
                .inner_margin(egui::Margin::symmetric(10, 6))
                .show(ui, |ui| {
                    ui.colored_label(Color32::from_rgb(226, 232, 240), msg);
                });
            ui.add_space(4.0);
        }

        // 2. Cached PNG texture for (species, mood, frame)
        let count = frames_for(pet.species, pet.mood).len().max(1);
        let idx = pet.frame % count;
        let key = (pet.species, pet.mood, idx);
        let tex = self.cache.entry(key).or_insert_with(|| {
            let img = decode_frame(pet.species, pet.mood, idx);
            ui.ctx().load_texture(
                format!("pet_{:?}_{:?}_{}", pet.species, pet.mood, idx),
                img,
                TextureOptions::LINEAR,
            )
        });

        let tex_size = tex.size_vec2();
        let max_dim = 128.0;
        let scale = (max_dim / tex_size.x.max(tex_size.y).max(1.0)).min(1.0);
        let draw_size = tex_size * scale;
        let (rect, resp) = ui.allocate_exact_size(draw_size, egui::Sense::click_and_drag());
        if resp.drag_started() {
            ui.ctx().send_viewport_cmd(ViewportCommand::StartDrag);
        } else if resp.clicked() {
            pet.show_input = !pet.show_input;
        }
        resp.context_menu(|ui| {
            for sp in PetSpecies::ALL {
                if ui.selectable_label(pet.species == sp, sp.name()).clicked() {
                    pet.species = sp;
                    pet.set_mood(PetMood::Idle);
                    pet.say(sp.greet(), 3.0);
                    ui.close_menu();
                }
            }
            ui.separator();
            for (m, lbl) in [(PetMood::Idle, "Idle"), (PetMood::Scan, "Scan"), (PetMood::Think, "Think")] {
                if ui.button(lbl).clicked() {
                    pet.set_mood(m);
                    ui.close_menu();
                }
            }
        });

        let uv = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
        ui.painter().image(tex.id(), rect, uv, Color32::WHITE);

        // 3. Separate input speech bubble (visible on click)
        if pet.show_input {
            ui.add_space(4.0);
            Frame::NONE
                .fill(Color32::from_rgba_unmultiplied(10, 14, 24, 232))
                .stroke(Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(56, 189, 248, 120)))
                .corner_radius(14.0)
                .inner_margin(egui::Margin::symmetric(8, 5))
                .show(ui, |ui| {
                    let edit = ui.add_sized(
                        [185.0, 22.0],
                        egui::TextEdit::singleline(&mut self.pet_prompt)
                            .hint_text(format!("Ask {}...", pet.species.name())),
                    );
                    if edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        let q = std::mem::take(&mut self.pet_prompt);
                        if !q.trim().is_empty() {
                            pet.set_mood(PetMood::Think);
                            pet.min_mood_ttl = 1.2;
                            pet.say("Thinking...", 6.0);
                            pet.show_input = false;
                            submitted = Some((pet.species, q));
                        }
                    }
                });
        }

        submitted
    }
}
