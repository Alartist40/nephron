//! Rendering and async synthesis wiring for the Interface Studio Modal.

use crate::a2ui::{draw_a2ui_surface, A2uiAction};
use crate::app::NephronApp;
use crate::icons::{paint_icon, vector_icon_button, IconKind};
use crate::studio::{
    bento_frame, draw_stat_bar, pick_file_dialog, techart_tag, ArtifactStage, CompactCol,
    RpgDaily, RpgHabit, RpgTodo, StudioPane, StudioSource,
};
use eframe::egui::{self, Color32, Frame, Pos2, Rect, Stroke};
use nephron_bridge::{BackendKind, ChatMessage, ChatRole, EffortLevel, ModelProfile};
use nephron_voice::tts::FEMALE_VOICES;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

fn bento_starter_card(
    ui: &mut egui::Ui,
    id: &str,
    size: egui::Vec2,
    num: &str,
    title: &str,
    col: Color32,
) -> bool {
    let (rect, resp) = ui.allocate_exact_size(size, egui::Sense::click());
    let hov = ui.ctx().animate_bool(egui::Id::new(id).with("hov"), resp.hovered());
    let prs = ui.ctx().animate_bool(egui::Id::new(id).with("prs"), resp.is_pointer_button_down_on());
    if hov > 0.01 {
        ui.ctx().request_repaint();
    }
    let lift = hov * 3.0 - prs * 1.5;
    let r = rect.translate(egui::vec2(0.0, -lift));
    let fill = Color32::from_rgba_unmultiplied(255, 255, 255, (11.0 + hov * 12.0) as u8);
    let br = Color32::from_rgba_unmultiplied(255, 255, 255, (24.0 + hov * 38.0) as u8);
    ui.painter().rect(r, 18.0, fill, Stroke::new(1.0_f32, br), egui::StrokeKind::Middle);

    // Top-left tag pill
    let tag_rect = Rect::from_min_size(r.min + egui::vec2(10.0, 10.0), egui::vec2(28.0, 17.0));
    ui.painter().rect(
        tag_rect,
        9.0,
        Color32::from_rgba_unmultiplied(col.r(), col.g(), col.b(), 24),
        Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(col.r(), col.g(), col.b(), 70)),
        egui::StrokeKind::Middle,
    );
    ui.painter().text(
        tag_rect.center(),
        egui::Align2::CENTER_CENTER,
        num,
        egui::FontId::monospace(9.5),
        col,
    );
    paint_icon(
        ui.painter(),
        Pos2::new(r.right() - 16.0, r.top() + 18.0),
        0.75,
        IconKind::ArrowUpRight,
        Color32::from_rgba_unmultiplied(148, 163, 184, (150.0 + hov * 105.0) as u8),
    );
    let galley = ui.painter().layout(
        title.to_string(),
        egui::FontId::proportional(11.0),
        Color32::from_rgb(226, 232, 240),
        size.x - 20.0,
    );
    ui.painter().galley(Pos2::new(r.left() + 10.0, r.top() + 34.0), galley, Color32::WHITE);
    resp.clicked()
}

#[allow(clippy::too_many_arguments)]
fn bento_deck_tile(
    ui: &mut egui::Ui,
    id: &str,
    size: egui::Vec2,
    active: bool,
    kind: IconKind,
    tag: &str,
    label: &str,
    col: Color32,
) -> bool {
    let (rect, resp) = ui.allocate_exact_size(size, egui::Sense::click());
    let hov = ui.ctx().animate_bool(egui::Id::new(id).with("hov"), resp.hovered());
    let prs = ui.ctx().animate_bool(egui::Id::new(id).with("prs"), resp.is_pointer_button_down_on());
    if hov > 0.01 {
        ui.ctx().request_repaint();
    }
    let lift = hov * 3.0 - prs * 1.5;
    let r = rect.translate(egui::vec2(0.0, -lift));
    let fill = if active {
        Color32::from_rgba_unmultiplied(col.r(), col.g(), col.b(), 26)
    } else {
        Color32::from_rgba_unmultiplied(255, 255, 255, (11.0 + hov * 13.0) as u8)
    };
    let br = if active {
        Color32::from_rgba_unmultiplied(col.r(), col.g(), col.b(), 145)
    } else {
        Color32::from_rgba_unmultiplied(255, 255, 255, (24.0 + hov * 38.0) as u8)
    };
    ui.painter().rect(r, 18.0, fill, Stroke::new(1.0_f32, br), egui::StrokeKind::Middle);
    paint_icon(ui.painter(), Pos2::new(r.left() + 17.0, r.top() + 18.0), 0.9, kind, col);
    let tag_rect = Rect::from_min_size(Pos2::new(r.right() - 40.0, r.top() + 10.0), egui::vec2(31.0, 16.0));
    ui.painter().rect(
        tag_rect,
        8.0,
        Color32::from_rgba_unmultiplied(col.r(), col.g(), col.b(), 22),
        Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(col.r(), col.g(), col.b(), 65)),
        egui::StrokeKind::Middle,
    );
    ui.painter().text(tag_rect.center(), egui::Align2::CENTER_CENTER, tag, egui::FontId::monospace(8.5), col);
    ui.painter().text(
        Pos2::new(r.left() + 11.0, r.bottom() - 14.0),
        egui::Align2::LEFT_CENTER,
        label,
        egui::FontId::proportional(11.0),
        Color32::from_rgb(226, 232, 240),
    );
    resp.clicked()
}

impl NephronApp {
    pub(crate) fn save_studio_and_rpg(&self) {
        if let Ok(j) = serde_json::to_string(&self.studio) {
            let _ = self.store.set_setting("studio_state", &j);
        }
        if let Ok(j) = serde_json::to_string(&self.rpg) {
            let _ = self.store.set_setting("rpg_state", &j);
        }
    }

    pub(crate) fn open_studio_file_picker(&mut self) {
        if self.studio_file_rx.is_some() {
            return;
        }
        let (tx, rx) = mpsc::unbounded_channel();
        self.studio_file_rx = Some(rx);
        std::thread::spawn(move || {
            let _ = tx.send(pick_file_dialog());
        });
    }

    pub(crate) fn trigger_studio_synthesis(&mut self, instruction: String) {
        if self.studio.generating {
            return;
        }
        self.studio.generating = true;
        self.studio_buf.clear();
        let context_block = self.studio.active_context_block();
        let prompt = format!(
            "You are Nephron's DeepTutor Cortex Studio engine. Ground your response strictly in the active sources below.\n\
            Return ONLY valid JSON with this schema:\n\
            {{\"answer\":\"concise grounded answer\",\"nodes\":[{{\"title\":\"short title\",\"body\":\"1-sentence fact\"}}],\
            \"flashcards\":[{{\"front\":\"question\",\"back\":\"answer\"}}],\
            \"quiz\":[{{\"q\":\"question\",\"options\":[\"A\",\"B\",\"C\"],\"answer\":0}}],\
            \"audioScript\":[{{\"speaker\":\"Host A\",\"line\":\"...\"}},{{\"speaker\":\"Host B\",\"line\":\"...\"}}],\
            \"mindmap\":[\"Root\",\"  ├─ Branch 1\",\"  └─ Branch 2\"],\
            \"slides\":[\"Slide 1: ...\",\"Slide 2: ...\",\"Slide 3: ...\"]}}\n\n\
            Active Sources:\n{context_block}\n\nTask: {instruction}"
        );
        let (tx, rx) = mpsc::unbounded_channel();
        self.studio_rx = Some(rx);
        let client = self.client.clone();
        let model = self.active_model_id();
        let effort = self.effort;
        let msgs = vec![ChatMessage {
            role: ChatRole::User,
            content: prompt,
        }];
        self.rt.spawn(async move {
            client.stream_chat_model(&model, effort, msgs, tx).await;
        });
        self.brain.trigger_cascade();
    }

    pub(crate) fn draw_studio_modal(&mut self, ctx: &egui::Context) {
        if !self.studio.open {
            return;
        }
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.studio.open = false;
            return;
        }

        let screen = ctx.screen_rect();
        egui::Area::new(egui::Id::new("studio_backdrop"))
            .fixed_pos(screen.min)
            .order(egui::Order::Middle)
            .show(ctx, |ui| {
                let (r, resp) = ui.allocate_exact_size(screen.size(), egui::Sense::click());
                ui.painter().rect_filled(r, 0.0, Color32::from_rgba_unmultiplied(3, 6, 12, 175));
                if resp.clicked() {
                    self.studio.open = false;
                }
            });

        let modal_w = (screen.width() - 48.0).clamp(360.0, 1240.0);
        let modal_h = (screen.height() - 116.0).clamp(380.0, 740.0);
        let modal_pos = Pos2::new((screen.width() - modal_w) * 0.5, 22.0);
        let compact = modal_w < 880.0;

        egui::Area::new(egui::Id::new("studio_interface_modal"))
            .fixed_pos(modal_pos)
            .order(egui::Order::Middle)
            .show(ctx, |ui| {
                let outer_rect = Rect::from_min_size(modal_pos, egui::vec2(modal_w, modal_h));
                ui.set_clip_rect(outer_rect);
                Frame::NONE
                    .fill(Color32::from_rgba_unmultiplied(10, 14, 24, 244))
                    .stroke(Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(255, 255, 255, 36)))
                    .corner_radius(24.0)
                    .inner_margin(0.0)
                    .show(ui, |ui| {
                        ui.allocate_ui_with_layout(
                            egui::vec2(modal_w, modal_h),
                            egui::Layout::top_down(egui::Align::LEFT),
                            |ui| {
                                ui.set_min_size(egui::vec2(modal_w, modal_h));
                                ui.set_max_size(egui::vec2(modal_w, modal_h));

                                // Top Studio Mode Strip
                                Frame::NONE
                                    .fill(Color32::from_rgba_unmultiplied(6, 9, 16, 190))
                                    .stroke(Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(255, 255, 255, 20)))
                                    .corner_radius(egui::CornerRadius { nw: 24, ne: 24, sw: 0, se: 0 })
                                    .inner_margin(egui::Margin::symmetric(16, 10))
                                    .show(ui, |ui| {
                                        ui.set_width(modal_w - 32.0);
                                        ui.horizontal(|ui| {
                                            for (pane, lbl) in [
                                                (StudioPane::Learn, "Cortex Studio"),
                                                (StudioPane::Habits, "Habits RPG"),
                                                (StudioPane::Engine, "Engine & TTS"),
                                            ] {
                                                let active = self.studio.pane == pane;
                                                let fill = if active {
                                                    Color32::from_rgba_unmultiplied(56, 189, 248, 52)
                                                } else {
                                                    Color32::from_rgba_unmultiplied(255, 255, 255, 10)
                                                };
                                                let stroke = if active {
                                                    Color32::from_rgba_unmultiplied(56, 189, 248, 125)
                                                } else {
                                                    Color32::from_rgba_unmultiplied(255, 255, 255, 24)
                                                };
                                                let txt_col = if active {
                                                    Color32::from_rgb(186, 230, 253)
                                                } else {
                                                    Color32::from_rgb(148, 163, 184)
                                                };
                                                let btn = egui::Button::new(egui::RichText::new(lbl).size(12.0).color(txt_col))
                                                    .fill(fill)
                                                    .stroke(Stroke::new(1.0_f32, stroke))
                                                    .corner_radius(99.0);
                                                if ui.add(btn).clicked() {
                                                    self.studio.pane = pane;
                                                }
                                            }

                                            if compact && self.studio.pane == StudioPane::Learn {
                                                ui.separator();
                                                for (col, lbl) in [
                                                    (CompactCol::Sources, "Sources"),
                                                    (CompactCol::Synthesis, "Synthesis"),
                                                    (CompactCol::Deck, "Deck"),
                                                ] {
                                                    if ui.selectable_label(self.studio.compact_col == col, lbl).clicked() {
                                                        self.studio.compact_col = col;
                                                    }
                                                }
                                            }

                                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                if vector_icon_button(
                                                    ui,
                                                    "close_studio_x",
                                                    egui::vec2(28.0, 28.0),
                                                    IconKind::Close,
                                                    Color32::from_rgb(203, 213, 225),
                                                    "Close Studio (Esc)",
                                                ) {
                                                    self.studio.open = false;
                                                }
                                                techart_tag(
                                                    ui,
                                                    &format!("{} nodes", self.memory_hits.len()),
                                                    Color32::from_rgb(252, 211, 77),
                                                );
                                                techart_tag(
                                                    ui,
                                                    &format!("Lv {}", self.rpg.level),
                                                    Color32::from_rgb(192, 132, 252),
                                                );
                                            });
                                        });
                                    });

                                let body_h = (modal_h - 52.0).max(260.0);
                                ui.allocate_ui_with_layout(
                                    egui::vec2(modal_w, body_h),
                                    egui::Layout::top_down(egui::Align::LEFT),
                                    |ui| {
                                        match self.studio.pane {
                                            StudioPane::Learn => self.draw_pane_learn(ui, modal_w, body_h, compact),
                                            StudioPane::Habits => self.draw_pane_habits(ui, modal_w, body_h),
                                            StudioPane::Engine => self.draw_pane_engine(ui, modal_w, body_h),
                                        }
                                    },
                                );
                            },
                        );
                    });
            });
    }

    fn draw_pane_learn(&mut self, ui: &mut egui::Ui, w: f32, h: f32, compact: bool) {
        if compact {
            match self.studio.compact_col {
                CompactCol::Sources => self.draw_learn_sources(ui, w, h),
                CompactCol::Synthesis => self.draw_learn_synthesis(ui, w, h),
                CompactCol::Deck => self.draw_learn_deck(ui, w, h),
            }
            return;
        }
        let col1_w = (w * 0.25).floor();
        let col2_w = (w * 0.42).floor();
        let col3_w = (w - col1_w - col2_w).max(240.0);
        ui.horizontal_top(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            self.draw_learn_sources(ui, col1_w, h);
            self.draw_learn_synthesis(ui, col2_w, h);
            self.draw_learn_deck(ui, col3_w, h);
        });
    }

    fn draw_learn_sources(&mut self, ui: &mut egui::Ui, col_w: f32, col_h: f32) {
        let inner_w = (col_w - 32.0).max(160.0);
        let inner_h = (col_h - 32.0).max(180.0);
        Frame::NONE
            .fill(Color32::from_rgba_unmultiplied(255, 255, 255, 5))
            .stroke(Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(255, 255, 255, 16)))
            .corner_radius(egui::CornerRadius { nw: 0, ne: 0, sw: 24, se: 0 })
            .inner_margin(16.0)
            .show(ui, |ui| {
                ui.allocate_ui_with_layout(
                    egui::vec2(inner_w, inner_h),
                    egui::Layout::top_down(egui::Align::LEFT),
                    |ui| {
                        ui.set_min_size(egui::vec2(inner_w, inner_h));
                        ui.set_clip_rect(ui.max_rect());
                        let active_count = self.studio.active_sources().len();
                        ui.horizontal(|ui| {
                            ui.set_width(inner_w);
                            ui.label(egui::RichText::new("01 · Knowledge Vault").size(12.5).strong().color(Color32::from_rgb(226, 232, 240)));
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                techart_tag(ui, &format!("{active_count} active"), Color32::from_rgb(125, 211, 252));
                            });
                        });
                        ui.add_space(10.0);

                        ui.horizontal(|ui| {
                            let half = (inner_w - 8.0) * 0.5;
                            if ui
                                .add_sized([half, 30.0], egui::Button::new("+ Drop PDF").corner_radius(14.0))
                                .on_hover_text("Attach PDF, Markdown, or text file (or drag & drop anywhere)")
                                .clicked()
                            {
                                self.open_studio_file_picker();
                            }
                            if ui
                                .add_sized([half, 30.0], egui::Button::new("+ Add Note").corner_radius(14.0))
                                .clicked()
                            {
                                self.studio.adding_note = !self.studio.adding_note;
                            }
                        });

                        if self.studio.adding_note {
                            ui.add_space(6.0);
                            bento_frame().show(ui, |ui| {
                                ui.vertical(|ui| {
                                    ui.add_sized([inner_w - 22.0, 22.0], egui::TextEdit::singleline(&mut self.studio.note_title).hint_text("Note title..."));
                                    ui.add_sized([inner_w - 22.0, 50.0], egui::TextEdit::multiline(&mut self.studio.note_body).hint_text("Paste study note..."));
                                    ui.horizontal(|ui| {
                                        if ui.small_button("Save Note").clicked() && !self.studio.note_body.trim().is_empty() {
                                            let title = if self.studio.note_title.trim().is_empty() {
                                                "Quick Study Note".into()
                                            } else {
                                                self.studio.note_title.trim().to_string()
                                            };
                                            let body = self.studio.note_body.trim().to_string();
                                            let _ = self.store.save_memory("note", &format!("{title}: {body}"));
                                            self.sync_brain();
                                            self.studio.sources.insert(
                                                0,
                                                StudioSource {
                                                    id: format!("note_{}", self.studio.sources.len() + 1),
                                                    title,
                                                    kind: "NOTE".into(),
                                                    active: true,
                                                    content: body,
                                                },
                                            );
                                            self.studio.note_title.clear();
                                            self.studio.note_body.clear();
                                            self.studio.adding_note = false;
                                            self.save_studio_and_rpg();
                                        }
                                        if ui.small_button("Cancel").clicked() {
                                            self.studio.adding_note = false;
                                        }
                                    });
                                });
                            });
                        }

                        ui.add_space(10.0);
                        Frame::NONE
                            .fill(Color32::from_rgba_unmultiplied(255, 255, 255, 10))
                            .stroke(Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(255, 255, 255, 26)))
                            .corner_radius(16.0)
                            .inner_margin(egui::Margin::symmetric(10, 4))
                            .show(ui, |ui| {
                                ui.set_min_size(egui::vec2(inner_w - 22.0, 28.0));
                                ui.set_max_size(egui::vec2(inner_w - 22.0, 28.0));
                                ui.horizontal(|ui| {
                                    let r = ui.add_sized(
                                        [(inner_w - 58.0).max(60.0), 24.0],
                                        egui::TextEdit::singleline(&mut self.studio.web_input)
                                            .frame(false)
                                            .hint_text("Discover web sources..."),
                                    );
                                    let enter = r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                                    let send = vector_icon_button(ui, "vault_web_go", egui::vec2(26.0, 26.0), IconKind::SendRight, Color32::from_rgb(186, 230, 253), "Search web sources");
                                    if (send || enter) && !self.studio.web_input.trim().is_empty() {
                                        let q = self.studio.web_input.trim().to_string();
                                        self.studio.web_input.clear();
                                        let searcher = self.searcher.clone();
                                        let (tx, rx) = mpsc::unbounded_channel();
                                        self.studio_web_rx = Some(rx);
                                        self.rt.spawn(async move {
                                            let hits = searcher.search(&q, 4).await.unwrap_or_default();
                                            let _ = tx.send((q, hits));
                                        });
                                    }
                                });
                            });

                        ui.add_space(12.0);
                        let mut remove_idx: Option<usize> = None;
                        let mut changed = false;
                        egui::ScrollArea::vertical().id_salt("vault_scroll").show(ui, |ui| {
                            for (idx, src) in self.studio.sources.iter_mut().enumerate() {
                                let stroke = if src.active {
                                    Color32::from_rgba_unmultiplied(56, 189, 248, 85)
                                } else {
                                    Color32::from_rgba_unmultiplied(255, 255, 255, 18)
                                };
                                Frame::NONE
                                    .fill(Color32::from_rgba_unmultiplied(15, 22, 36, if src.active { 210 } else { 110 }))
                                    .stroke(Stroke::new(1.0_f32, stroke))
                                    .corner_radius(12.0)
                                    .inner_margin(8.0)
                                    .show(ui, |ui| {
                                        ui.set_width(inner_w - 18.0);
                                        ui.horizontal(|ui| {
                                            if ui.checkbox(&mut src.active, "").changed() {
                                                changed = true;
                                            }
                                            ui.vertical(|ui| {
                                                ui.add(egui::Label::new(egui::RichText::new(&src.title).size(11.5).color(Color32::from_rgb(241, 245, 249))).wrap());
                                                ui.small(egui::RichText::new(&src.kind).monospace().color(Color32::from_rgb(125, 211, 252)));
                                            });
                                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                if ui.small_button("×").clicked() {
                                                    remove_idx = Some(idx);
                                                }
                                            });
                                        });
                                    });
                                ui.add_space(5.0);
                            }
                        });
                        if let Some(i) = remove_idx {
                            self.studio.sources.remove(i);
                            changed = true;
                        }
                        if changed {
                            self.save_studio_and_rpg();
                        }
                    },
                );
            });
    }

    fn draw_learn_synthesis(&mut self, ui: &mut egui::Ui, col_w: f32, col_h: f32) {
        let inner_w = (col_w - 28.0).max(220.0);
        let inner_h = (col_h - 28.0).max(200.0);
        Frame::NONE
            .fill(Color32::from_rgba_unmultiplied(4, 7, 14, 145))
            .inner_margin(14.0)
            .show(ui, |ui| {
                ui.allocate_ui_with_layout(
                    egui::vec2(inner_w, inner_h),
                    egui::Layout::top_down(egui::Align::LEFT),
                    |ui| {
                        ui.set_min_size(egui::vec2(inner_w, inner_h));
                        ui.set_clip_rect(ui.max_rect());
                        let scroll_h = (inner_h - 52.0).max(150.0);
                        let mut starter_prompt: Option<String> = None;

                        ui.allocate_ui_with_layout(
                            egui::vec2(inner_w, scroll_h),
                            egui::Layout::top_down(egui::Align::LEFT),
                            |ui| {
                                ui.set_min_size(egui::vec2(inner_w, scroll_h));
                                egui::ScrollArea::vertical()
                                    .id_salt("studio_synth_scroll")
                                    .max_height(scroll_h)
                                    .stick_to_bottom(true)
                                    .show(ui, |ui| {
                                        ui.set_width(inner_w - 8.0);
                                        ui.horizontal(|ui| {
                                            techart_tag(ui, "SYNTHESIS ENGINE", Color32::from_rgb(110, 231, 183));
                                            techart_tag(ui, "3D CORTEX SYNC", Color32::from_rgb(148, 163, 184));
                                        });
                                        ui.add_space(6.0);
                                        ui.label(egui::RichText::new("MindCorpus Cognitive Canvas").size(18.0).color(Color32::WHITE));
                                        ui.add(
                                            egui::Label::new(
                                                egui::RichText::new("Distill PDFs, notes, and web discoveries into living cortical stars—or launch an interactive deck on the right.")
                                                    .size(11.5)
                                                    .color(Color32::from_rgb(148, 163, 184)),
                                            )
                                            .wrap(),
                                        );
                                        ui.add_space(10.0);

                                        let card_w = ((inner_w - 20.0) / 3.0).max(90.0);
                                        ui.horizontal(|ui| {
                                            ui.spacing_mut().item_spacing.x = 6.0;
                                            for (idx, (num, title, prompt, col)) in [
                                                (
                                                    "01",
                                                    "First-Principles Breakdown",
                                                    "Break down the core concepts and first principles from my active sources.",
                                                    Color32::from_rgb(125, 211, 252),
                                                ),
                                                (
                                                    "02",
                                                    "Curriculum & Blind Spots",
                                                    "Construct a structured study curriculum with key mental models and blind spots.",
                                                    Color32::from_rgb(252, 211, 77),
                                                ),
                                                (
                                                    "03",
                                                    "Socratic Mastery Check",
                                                    "Challenge me with Socratic questions on the hardest ideas in my sources.",
                                                    Color32::from_rgb(110, 231, 183),
                                                ),
                                            ]
                                            .into_iter()
                                            .enumerate()
                                            {
                                                if bento_starter_card(
                                                    ui,
                                                    &format!("starter_{idx}"),
                                                    egui::vec2(card_w, 76.0),
                                                    num,
                                                    title,
                                                    col,
                                                ) {
                                                    starter_prompt = Some(prompt.to_string());
                                                }
                                            }
                                        });

                                        ui.add_space(12.0);
                                        let mut a2ui_act: Option<A2uiAction> = None;
                                        for m in &mut self.studio.chat {
                                            if let Some(surface) = &mut m.surface {
                                                if let Some(act) = draw_a2ui_surface(ui, inner_w - 12.0, surface) {
                                                    a2ui_act = Some(act);
                                                }
                                            } else {
                                                let bg = if m.user {
                                                    Color32::from_rgba_unmultiplied(14, 165, 233, 45)
                                                } else {
                                                    Color32::from_rgba_unmultiplied(255, 255, 255, 12)
                                                };
                                                Frame::NONE
                                                    .fill(bg)
                                                    .stroke(Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(255, 255, 255, 22)))
                                                    .corner_radius(14.0)
                                                    .inner_margin(10.0)
                                                    .show(ui, |ui| {
                                                        ui.vertical(|ui| {
                                                            ui.set_width(inner_w - 32.0);
                                                            ui.add(egui::Label::new(egui::RichText::new(&m.text).size(12.0).color(Color32::from_rgb(241, 245, 249))).wrap());
                                                            if !m.citations.is_empty() {
                                                                ui.add_space(4.0);
                                                                ui.small(
                                                                    egui::RichText::new(format!("Sources: {}", m.citations.join(" · ")))
                                                                        .color(Color32::from_rgb(125, 211, 252)),
                                                                );
                                                            }
                                                        });
                                                    });
                                            }
                                            ui.add_space(8.0);
                                        }
                                        if let Some(act) = a2ui_act {
                                            match act.kind.as_deref() {
                                                Some("stage:flashcards") => self.studio.stage = ArtifactStage::Flashcards,
                                                Some("stage:quiz") => self.studio.stage = ArtifactStage::Quiz,
                                                Some("stage:mindmap") => self.studio.stage = ArtifactStage::MindMap,
                                                _ => {
                                                    if let Some(p) = act.prompt {
                                                        starter_prompt = Some(p);
                                                    }
                                                }
                                            }
                                        }
                                        if self.studio.generating {
                                            ui.small(egui::RichText::new("Synthesizing across active sources...").monospace().color(Color32::from_rgb(125, 211, 252)));
                                        }
                                    });
                            },
                        );

                        ui.add_space(6.0);
                        let mut submit_prompt = starter_prompt;
                        Frame::NONE
                            .fill(Color32::from_rgba_unmultiplied(255, 255, 255, 12))
                            .stroke(Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(255, 255, 255, 32)))
                            .corner_radius(99.0)
                            .inner_margin(egui::Margin::symmetric(14, 5))
                            .show(ui, |ui| {
                                ui.set_min_size(egui::vec2(inner_w - 28.0, 30.0));
                                ui.set_max_size(egui::vec2(inner_w - 28.0, 30.0));
                                ui.horizontal(|ui| {
                                    ui.spacing_mut().item_spacing.x = 6.0;
                                    let r = ui.add_sized(
                                        [(inner_w - 118.0).max(80.0), 26.0],
                                        egui::TextEdit::singleline(&mut self.studio.chat_input)
                                            .frame(false)
                                            .hint_text("Ask or synthesize across sources..."),
                                    );
                                    let enter = r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                                    ui.add_sized(
                                        [42.0, 20.0],
                                        egui::Label::new(
                                            egui::RichText::new(format!("{} src", self.studio.active_sources().len()))
                                                .monospace()
                                                .size(10.5)
                                                .color(Color32::from_rgb(148, 163, 184)),
                                        ),
                                    );
                                    let send = vector_icon_button(
                                        ui,
                                        "studio_send_up",
                                        egui::vec2(28.0, 28.0),
                                        IconKind::SendUp,
                                        Color32::from_rgb(186, 230, 253),
                                        "Synthesize",
                                    );
                                    if !self.studio.generating && (send || enter) && !self.studio.chat_input.trim().is_empty() {
                                        submit_prompt = Some(self.studio.chat_input.trim().to_string());
                                        self.studio.chat_input.clear();
                                    }
                                });
                            });

                        if let Some(p) = submit_prompt {
                            self.studio.chat.push(crate::studio::StudioChatMsg {
                                user: true,
                                text: p.clone(),
                                citations: Vec::new(),
                                surface: None,
                            });
                            self.rpg.award_xp(15, 4, "+15 XP · Synthesis");
                            self.trigger_studio_synthesis(p);
                            self.save_studio_and_rpg();
                        }
                    },
                );
            });
    }

    fn draw_learn_deck(&mut self, ui: &mut egui::Ui, col_w: f32, col_h: f32) {
        let inner_w = (col_w - 32.0).max(200.0);
        let inner_h = (col_h - 32.0).max(200.0);
        Frame::NONE
            .fill(Color32::from_rgba_unmultiplied(255, 255, 255, 5))
            .stroke(Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(255, 255, 255, 16)))
            .corner_radius(egui::CornerRadius { nw: 0, ne: 0, sw: 0, se: 24 })
            .inner_margin(16.0)
            .show(ui, |ui| {
                ui.allocate_ui_with_layout(
                    egui::vec2(inner_w, inner_h),
                    egui::Layout::top_down(egui::Align::LEFT),
                    |ui| {
                        ui.set_min_size(egui::vec2(inner_w, inner_h));
                        ui.set_clip_rect(ui.max_rect());
                        ui.horizontal(|ui| {
                            ui.set_width(inner_w);
                            ui.label(egui::RichText::new("02 · Interactive Deck").size(12.5).strong().color(Color32::from_rgb(226, 232, 240)));
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                let st = if self.studio.generating { "synthesizing..." } else { "ready" };
                                techart_tag(ui, st, Color32::from_rgb(148, 163, 184));
                            });
                        });
                        ui.add_space(6.0);

                        let tiles = [
                            ("tile_audio", IconKind::Audio, "TTS", "Audio Dialogue", ArtifactStage::Audio, Color32::from_rgb(125, 211, 252)),
                            ("tile_cards", IconKind::Flashcards, "3D", "Flashcards", ArtifactStage::Flashcards, Color32::from_rgb(252, 211, 77)),
                            ("tile_quiz", IconKind::Quiz, "+XP", "Active Quiz", ArtifactStage::Quiz, Color32::from_rgb(103, 232, 249)),
                            ("tile_tree", IconKind::Tree, "MAP", "Concept Tree", ArtifactStage::MindMap, Color32::from_rgb(110, 231, 183)),
                            ("tile_slides", IconKind::Slides, "DECK", "Visual Slides", ArtifactStage::Slides, Color32::from_rgb(216, 180, 254)),
                            ("tile_brief", IconKind::Brief, "DOC", "Field Brief", ArtifactStage::Report, Color32::from_rgb(253, 164, 175)),
                        ];
                        let cols = if inner_w >= 300.0 { 3 } else { 2 };
                        let tile_w = ((inner_w - (cols - 1) as f32 * 6.0) / cols as f32).max(80.0);
                        for row in tiles.chunks(cols) {
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 6.0;
                                for (id, kind, tag, lbl, st, col) in row {
                                    if bento_deck_tile(
                                        ui,
                                        id,
                                        egui::vec2(tile_w, 58.0),
                                        self.studio.stage == *st,
                                        *kind,
                                        tag,
                                        lbl,
                                        *col,
                                    ) {
                                        self.studio.stage = *st;
                                    }
                                }
                            });
                            ui.add_space(6.0);
                        }

                        let tiles_h = if cols == 3 { 132.0 } else { 196.0 };
                        let stage_h = (inner_h - tiles_h - 30.0).max(150.0);
                        Frame::NONE
                            .fill(Color32::from_rgba_unmultiplied(4, 7, 14, 185))
                            .stroke(Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(255, 255, 255, 22)))
                            .corner_radius(18.0)
                            .inner_margin(12.0)
                            .show(ui, |ui| {
                                ui.allocate_ui_with_layout(
                                    egui::vec2(inner_w - 24.0, stage_h - 24.0),
                                    egui::Layout::top_down(egui::Align::LEFT),
                                    |ui| {
                                        ui.set_min_size(egui::vec2(inner_w - 24.0, stage_h - 24.0));
                                        match self.studio.stage {
                                            ArtifactStage::Empty => {
                                                ui.centered_and_justified(|ui| {
                                                    ui.add(
                                                        egui::Label::new(
                                                            egui::RichText::new("Select an artifact card above\nFlashcards, XP Quizzes, Audio Dialogue, or Concept Tree")
                                                                .size(11.5)
                                                                .color(Color32::from_rgb(148, 163, 184)),
                                                        )
                                                        .wrap(),
                                                    );
                                                });
                                            }
                                            ArtifactStage::Flashcards => {
                                                let total = self.studio.flashcards.len().max(1);
                                                let idx = self.studio.fc_idx % total;
                                                if let Some(c) = self.studio.flashcards.get(idx).cloned() {
                                                    let (tag, body, col) = if self.studio.fc_flipped {
                                                        ("INSIGHT", c.back, Color32::from_rgb(110, 231, 183))
                                                    } else {
                                                        ("PROMPT · TAP TO FLIP", c.front, Color32::from_rgb(252, 211, 77))
                                                    };
                                                    let resp = bento_frame()
                                                        .show(ui, |ui| {
                                                            ui.allocate_ui_with_layout(
                                                                egui::vec2(inner_w - 46.0, (stage_h - 86.0).max(80.0)),
                                                                egui::Layout::top_down(egui::Align::Center),
                                                                |ui| {
                                                                    techart_tag(ui, tag, col);
                                                                    ui.add_space(10.0);
                                                                    ui.add(egui::Label::new(egui::RichText::new(body).size(12.0).color(Color32::WHITE)).wrap());
                                                                },
                                                            );
                                                        })
                                                        .response;
                                                    if resp.interact(egui::Sense::click()).clicked() {
                                                        self.studio.fc_flipped = !self.studio.fc_flipped;
                                                    }
                                                    ui.add_space(6.0);
                                                    ui.horizontal(|ui| {
                                                        if ui.small_button("←").clicked() {
                                                            self.studio.fc_idx = (idx + total - 1) % total;
                                                            self.studio.fc_flipped = false;
                                                        }
                                                        ui.label(egui::RichText::new(format!("{} / {}", idx + 1, total)).monospace().size(11.0));
                                                        if ui.small_button("→").clicked() {
                                                            self.studio.fc_idx = (idx + 1) % total;
                                                            self.studio.fc_flipped = false;
                                                        }
                                                    });
                                                }
                                            }
                                            ArtifactStage::Quiz => {
                                                let total = self.studio.quiz.len().max(1);
                                                let idx = self.studio.quiz_idx % total;
                                                if let Some(q) = self.studio.quiz.get(idx).cloned() {
                                                    ui.horizontal(|ui| {
                                                        techart_tag(ui, &format!("{} / {}", idx + 1, total), Color32::from_rgb(103, 232, 249));
                                                        techart_tag(ui, &format!("score: {}", self.studio.quiz_score), Color32::from_rgb(110, 231, 183));
                                                    });
                                                    ui.add_space(6.0);
                                                    ui.add(egui::Label::new(egui::RichText::new(&q.q).size(12.0).strong().color(Color32::WHITE)).wrap());
                                                    ui.add_space(6.0);
                                                    let mut picked: Option<usize> = None;
                                                    for (i, opt) in q.options.iter().enumerate() {
                                                        let chosen = self.studio.quiz_selected == Some(i);
                                                        let col = if chosen && i == q.answer {
                                                            Color32::from_rgb(110, 231, 183)
                                                        } else if chosen {
                                                            Color32::from_rgb(253, 164, 175)
                                                        } else {
                                                            Color32::from_rgb(226, 232, 240)
                                                        };
                                                        if ui.add_sized([inner_w - 30.0, 26.0], egui::Button::new(egui::RichText::new(opt).size(11.0).color(col))).clicked()
                                                            && self.studio.quiz_selected.is_none()
                                                        {
                                                            picked = Some(i);
                                                        }
                                                    }
                                                    if let Some(i) = picked {
                                                        self.studio.quiz_selected = Some(i);
                                                        if i == q.answer {
                                                            self.studio.quiz_score += 1;
                                                            self.studio.quiz_feedback = "Correct! +20 XP · +5 Gold".into();
                                                            self.rpg.award_xp(20, 5, "+20 XP · Quiz");
                                                            self.brain.trigger_cascade();
                                                        } else {
                                                            self.studio.quiz_feedback = "Not quite — review & retry".into();
                                                        }
                                                        self.save_studio_and_rpg();
                                                    }
                                                    ui.add_space(6.0);
                                                    ui.horizontal(|ui| {
                                                        ui.small(egui::RichText::new(&self.studio.quiz_feedback).color(Color32::from_rgb(148, 163, 184)));
                                                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                            if ui.small_button("next →").clicked() {
                                                                self.studio.quiz_idx = (idx + 1) % total;
                                                                self.studio.quiz_selected = None;
                                                                self.studio.quiz_feedback = "Pick an option".into();
                                                            }
                                                        });
                                                    });
                                                }
                                            }
                                            ArtifactStage::Audio => {
                                                ui.horizontal(|ui| {
                                                    techart_tag(ui, "SYNTHETIC DIALOGUE", Color32::from_rgb(125, 211, 252));
                                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                        if ui.small_button("▶ play").clicked() {
                                                            for turn in &self.studio.audio_script {
                                                                self.speech.enqueue(format!("{}: {}", turn.speaker, turn.line));
                                                            }
                                                        }
                                                    });
                                                });
                                                ui.add_space(6.0);
                                                egui::ScrollArea::vertical().id_salt("audio_scr").show(ui, |ui| {
                                                    for turn in &self.studio.audio_script {
                                                        ui.add(
                                                            egui::Label::new(
                                                                egui::RichText::new(format!("{}: {}", turn.speaker, turn.line))
                                                                    .size(11.5)
                                                                    .color(Color32::from_rgb(226, 232, 240)),
                                                            )
                                                            .wrap(),
                                                        );
                                                        ui.add_space(4.0);
                                                    }
                                                });
                                            }
                                            ArtifactStage::MindMap | ArtifactStage::Slides | ArtifactStage::Report => {
                                                let (title, lines): (&str, Vec<String>) = match self.studio.stage {
                                                    ArtifactStage::MindMap => ("Concept Tree", self.studio.mindmap_lines.clone()),
                                                    ArtifactStage::Slides => ("Synthesis Slide Deck", self.studio.slides.clone()),
                                                    _ => (
                                                        &self.studio.brief_title,
                                                        std::iter::once(format!("Active Sources: {}", self.studio.active_sources().len()))
                                                            .chain(self.studio.slides.clone())
                                                            .collect(),
                                                    ),
                                                };
                                                ui.label(egui::RichText::new(title).size(12.5).strong().color(Color32::from_rgb(110, 231, 183)));
                                                ui.add_space(6.0);
                                                egui::ScrollArea::vertical().id_salt("doc_scr").show(ui, |ui| {
                                                    for ln in lines {
                                                        ui.add(egui::Label::new(egui::RichText::new(ln).size(11.5).color(Color32::from_rgb(226, 232, 240))).wrap());
                                                        ui.add_space(3.0);
                                                    }
                                                });
                                            }
                                        }
                                    },
                                );
                            });
                    },
                );
            });
    }

    fn draw_pane_habits(&mut self, ui: &mut egui::Ui, w: f32, h: f32) {
        let inner_w = (w - 32.0).max(300.0);
        let inner_h = (h - 32.0).max(220.0);
        Frame::NONE.inner_margin(16.0).show(ui, |ui| {
            ui.allocate_ui_with_layout(
                egui::vec2(inner_w, inner_h),
                egui::Layout::top_down(egui::Align::LEFT),
                |ui| {
                    // Character HUD Banner
                    Frame::NONE
                        .fill(Color32::from_rgba_unmultiplied(24, 24, 52, 210))
                        .stroke(Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(165, 180, 252, 65)))
                        .corner_radius(18.0)
                        .inner_margin(12.0)
                        .show(ui, |ui| {
                            ui.set_width(inner_w - 24.0);
                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new("🧙").size(28.0));
                                ui.vertical(|ui| {
                                    ui.horizontal(|ui| {
                                        ui.label(egui::RichText::new("Neural Scholar").size(14.0).strong().color(Color32::WHITE));
                                        techart_tag(ui, &format!("Lv {}", self.rpg.level), Color32::from_rgb(251, 191, 36));
                                        techart_tag(ui, self.rpg.scholar_title(), Color32::from_rgb(192, 132, 252));
                                    });
                                    ui.small(egui::RichText::new("Complete habits & study quests to evolve your 3D cortex").color(Color32::from_rgb(148, 163, 184)));
                                });
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    techart_tag(ui, &format!("GOLD {}", self.rpg.gold), Color32::from_rgb(253, 224, 71));
                                    draw_stat_bar(ui, "MP", self.rpg.mp, self.rpg.max_mp, Color32::from_rgb(56, 189, 248));
                                    draw_stat_bar(ui, "XP", self.rpg.xp, self.rpg.max_xp, Color32::from_rgb(251, 191, 36));
                                    draw_stat_bar(ui, "HP", self.rpg.hp, self.rpg.max_hp, Color32::from_rgb(251, 113, 133));
                                });
                            });
                        });

                    ui.add_space(12.0);
                    let col_w = ((inner_w - 20.0) / 3.0).max(180.0);
                    let col_h = (inner_h - 90.0).max(200.0);
                    let mut dirty = false;

                    ui.horizontal_top(|ui| {
                        ui.spacing_mut().item_spacing.x = 10.0;

                        // Column 1: Habits (+ / -)
                        bento_frame().show(ui, |ui| {
                            ui.allocate_ui_with_layout(
                                egui::vec2(col_w - 20.0, col_h - 20.0),
                                egui::Layout::top_down(egui::Align::LEFT),
                                |ui| {
                                    ui.horizontal(|ui| {
                                        ui.label(egui::RichText::new("Habits (+ / −)").size(12.5).strong().color(Color32::from_rgb(253, 230, 138)));
                                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                            ui.small("reinforce");
                                        });
                                    });
                                    ui.horizontal(|ui| {
                                        let r = ui.add_sized([col_w - 56.0, 24.0], egui::TextEdit::singleline(&mut self.rpg.new_habit).hint_text("Add a Habit..."));
                                        let enter = r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                                        if (ui.small_button("+").clicked() || enter) && !self.rpg.new_habit.trim().is_empty() {
                                            let t = self.rpg.new_habit.trim().to_string();
                                            self.rpg.new_habit.clear();
                                            self.rpg.habits.insert(0, RpgHabit { title: t, pos: 0, neg: 0 });
                                            dirty = true;
                                        }
                                    });
                                    egui::ScrollArea::vertical().id_salt("rpg_hb").show(ui, |ui| {
                                        let mut delta: Option<(usize, bool)> = None;
                                        for (i, h) in self.rpg.habits.iter().enumerate() {
                                            Frame::NONE
                                                .fill(Color32::from_rgba_unmultiplied(6, 10, 18, 165))
                                                .stroke(Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(255, 255, 255, 18)))
                                                .corner_radius(12.0)
                                                .inner_margin(8.0)
                                                .show(ui, |ui| {
                                                    ui.set_width(col_w - 38.0);
                                                    ui.horizontal(|ui| {
                                                        if ui.small_button(egui::RichText::new("+").color(Color32::from_rgb(110, 231, 183))).clicked() {
                                                            delta = Some((i, true));
                                                        }
                                                        ui.vertical(|ui| {
                                                            ui.add(egui::Label::new(egui::RichText::new(&h.title).size(11.5).color(Color32::from_rgb(226, 232, 240))).wrap());
                                                            ui.small(format!("+{} | −{}", h.pos, h.neg));
                                                        });
                                                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                            if ui.small_button(egui::RichText::new("−").color(Color32::from_rgb(253, 164, 175))).clicked() {
                                                                delta = Some((i, false));
                                                            }
                                                        });
                                                    });
                                                });
                                            ui.add_space(4.0);
                                        }
                                        if let Some((idx, pos)) = delta {
                                            if pos {
                                                self.rpg.habits[idx].pos += 1;
                                                self.rpg.award_xp(12, 3, "+12 XP · +3 Gold");
                                                self.brain.trigger_cascade();
                                            } else {
                                                self.rpg.habits[idx].neg += 1;
                                                self.rpg.damage_hp(5, "−5 HP");
                                            }
                                            dirty = true;
                                        }
                                    });
                                },
                            );
                        });

                        // Column 2: Dailies
                        bento_frame().show(ui, |ui| {
                            ui.allocate_ui_with_layout(
                                egui::vec2(col_w - 20.0, col_h - 20.0),
                                egui::Layout::top_down(egui::Align::LEFT),
                                |ui| {
                                    ui.horizontal(|ui| {
                                        ui.label(egui::RichText::new("Dailies").size(12.5).strong().color(Color32::from_rgb(186, 230, 253)));
                                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                            ui.small("streaks");
                                        });
                                    });
                                    ui.horizontal(|ui| {
                                        let r = ui.add_sized([col_w - 56.0, 24.0], egui::TextEdit::singleline(&mut self.rpg.new_daily).hint_text("Add a Daily..."));
                                        let enter = r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                                        if (ui.small_button("+").clicked() || enter) && !self.rpg.new_daily.trim().is_empty() {
                                            let t = self.rpg.new_daily.trim().to_string();
                                            self.rpg.new_daily.clear();
                                            self.rpg.dailies.insert(0, RpgDaily { title: t, streak: 0, done: false });
                                            dirty = true;
                                        }
                                    });
                                    egui::ScrollArea::vertical().id_salt("rpg_dl").show(ui, |ui| {
                                        let mut toggled: Option<usize> = None;
                                        for (i, d) in self.rpg.dailies.iter().enumerate() {
                                            Frame::NONE
                                                .fill(Color32::from_rgba_unmultiplied(14, 165, 233, if d.done { 10 } else { 26 }))
                                                .stroke(Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(56, 189, 248, if d.done { 20 } else { 55 })))
                                                .corner_radius(12.0)
                                                .inner_margin(8.0)
                                                .show(ui, |ui| {
                                                    ui.set_width(col_w - 38.0);
                                                    ui.horizontal(|ui| {
                                                        let mut done = d.done;
                                                        if ui.checkbox(&mut done, &d.title).changed() {
                                                            toggled = Some(i);
                                                        }
                                                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                            ui.small(egui::RichText::new(format!("🔥{}", d.streak)).color(Color32::from_rgb(252, 211, 77)));
                                                        });
                                                    });
                                                });
                                            ui.add_space(4.0);
                                        }
                                        if let Some(i) = toggled {
                                            let d = &mut self.rpg.dailies[i];
                                            d.done = !d.done;
                                            if d.done {
                                                d.streak += 1;
                                                self.rpg.award_xp(20, 6, "+20 XP · Streak!");
                                                self.brain.trigger_cascade();
                                            } else {
                                                d.streak = d.streak.saturating_sub(1);
                                            }
                                            dirty = true;
                                        }
                                    });
                                },
                            );
                        });

                        // Column 3: To-Dos & Focus Boss Quest
                        bento_frame().show(ui, |ui| {
                            ui.allocate_ui_with_layout(
                                egui::vec2(col_w - 20.0, col_h - 20.0),
                                egui::Layout::top_down(egui::Align::LEFT),
                                |ui| {
                                    ui.horizontal(|ui| {
                                        ui.label(egui::RichText::new("To-Dos & Focus Quest").size(12.5).strong().color(Color32::from_rgb(167, 243, 208)));
                                        if !self.rpg.toast.is_empty() {
                                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                ui.small(egui::RichText::new(&self.rpg.toast).monospace().color(Color32::from_rgb(110, 231, 183)));
                                            });
                                        }
                                    });
                                    ui.horizontal(|ui| {
                                        let r = ui.add_sized([col_w - 56.0, 24.0], egui::TextEdit::singleline(&mut self.rpg.new_todo).hint_text("Add a Quest To-Do..."));
                                        let enter = r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                                        if (ui.small_button("+").clicked() || enter) && !self.rpg.new_todo.trim().is_empty() {
                                            let t = self.rpg.new_todo.trim().to_string();
                                            self.rpg.new_todo.clear();
                                            self.rpg.todos.insert(0, RpgTodo { title: t, difficulty: "Medium".into() });
                                            dirty = true;
                                        }
                                    });
                                    let mut done_idx: Option<usize> = None;
                                    egui::ScrollArea::vertical().id_salt("rpg_td").max_height(col_h - 125.0).show(ui, |ui| {
                                        for (i, td) in self.rpg.todos.iter().enumerate() {
                                            Frame::NONE
                                                .fill(Color32::from_rgba_unmultiplied(16, 185, 129, 22))
                                                .stroke(Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(52, 211, 153, 55)))
                                                .corner_radius(12.0)
                                                .inner_margin(8.0)
                                                .show(ui, |ui| {
                                                    ui.set_width(col_w - 38.0);
                                                    ui.horizontal(|ui| {
                                                        if ui.small_button("✓").clicked() {
                                                            done_idx = Some(i);
                                                        }
                                                        ui.add(egui::Label::new(egui::RichText::new(&td.title).size(11.5).color(Color32::from_rgb(226, 232, 240))).wrap());
                                                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                            ui.small(egui::RichText::new(&td.difficulty).color(Color32::from_rgb(110, 231, 183)));
                                                        });
                                                    });
                                                });
                                            ui.add_space(4.0);
                                        }
                                    });
                                    if let Some(i) = done_idx {
                                        self.rpg.todos.remove(i);
                                        self.rpg.award_xp(30, 10, "+30 XP · Quest Clear!");
                                        self.brain.trigger_cascade();
                                        dirty = true;
                                    }

                                    ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                                        Frame::NONE
                                            .fill(Color32::from_rgba_unmultiplied(168, 85, 247, 26))
                                            .stroke(Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(192, 132, 252, 65)))
                                            .corner_radius(14.0)
                                            .inner_margin(10.0)
                                            .show(ui, |ui| {
                                                ui.set_width(col_w - 38.0);
                                                ui.horizontal(|ui| {
                                                    let secs = self
                                                        .pomo_end
                                                        .map(|e| e.saturating_duration_since(Instant::now()).as_secs())
                                                        .unwrap_or(25 * 60);
                                                    ui.vertical(|ui| {
                                                        ui.small(egui::RichText::new("FOCUS QUEST (+50 XP)").monospace().color(Color32::from_rgb(216, 180, 254)));
                                                        ui.label(egui::RichText::new(format!("{:02}:{:02}", secs / 60, secs % 60)).monospace().size(16.0).color(Color32::WHITE));
                                                    });
                                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                        if ui.small_button("↺").clicked() {
                                                            self.pomo_end = None;
                                                        }
                                                        let lbl = if self.pomo_end.is_some() { "pause" } else { "battle" };
                                                        if ui.small_button(lbl).clicked() {
                                                            self.pomo_end = if self.pomo_end.is_some() {
                                                                None
                                                            } else {
                                                                Some(Instant::now() + Duration::from_secs(25 * 60))
                                                            };
                                                        }
                                                    });
                                                });
                                            });
                                    });
                                },
                            );
                        });
                    });

                    if dirty {
                        self.save_studio_and_rpg();
                    }
                },
            );
        });
    }

    fn draw_pane_engine(&mut self, ui: &mut egui::Ui, w: f32, h: f32) {
        let inner_w = (w - 36.0).max(300.0);
        Frame::NONE.inner_margin(18.0).show(ui, |ui| {
            let card_w = ((inner_w - 16.0) * 0.5).max(240.0);
            let card_h = (h - 56.0).max(220.0);
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = 14.0;

                // Card 1: Local Inference & Thinking Level
                bento_frame().show(ui, |ui| {
                    ui.allocate_ui_with_layout(
                        egui::vec2(card_w - 20.0, card_h - 20.0),
                        egui::Layout::top_down(egui::Align::LEFT),
                        |ui| {
                            ui.label(egui::RichText::new("Local Inference & Research Bridge").size(13.0).strong().color(Color32::WHITE));
                            ui.add_space(8.0);
                            ui.horizontal(|ui| {
                                let hw = (card_w - 30.0) * 0.5;
                                for (kind, lbl) in [(BackendKind::Cynapse, "cynapse :38265"), (BackendKind::Ollama, "ollama :11434")] {
                                    let active = self.backend_kind == kind;
                                    let btn = egui::Button::new(egui::RichText::new(lbl).monospace().size(11.5))
                                        .fill(if active { Color32::from_rgba_unmultiplied(56, 189, 248, 52) } else { Color32::from_rgba_unmultiplied(255, 255, 255, 12) })
                                        .corner_radius(12.0);
                                    if ui.add_sized([hw, 32.0], btn).clicked() {
                                        self.switch_backend(kind);
                                    }
                                }
                            });
                            ui.add_space(8.0);
                            ui.horizontal(|ui| {
                                ui.label("Model:");
                                if self.backend_kind == BackendKind::Cynapse {
                                    egui::ComboBox::from_id_salt("eng_cyn").selected_text(self.model.id()).show_ui(ui, |ui| {
                                        for m in ModelProfile::ALL {
                                            ui.selectable_value(&mut self.model, m, m.id());
                                        }
                                    });
                                } else {
                                    egui::ComboBox::from_id_salt("eng_ol").selected_text(&self.selected_ollama).show_ui(ui, |ui| {
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
                            });
                            ui.horizontal(|ui| {
                                ui.label("Thinking Level:");
                                egui::ComboBox::from_id_salt("eng_eff").selected_text(self.effort.as_str()).show_ui(ui, |ui| {
                                    for e in EffortLevel::ALL {
                                        ui.selectable_value(&mut self.effort, e, e.as_str());
                                    }
                                });
                            });
                            ui.add_space(6.0);
                            ui.horizontal(|ui| {
                                if ui.checkbox(&mut self.inject_pinned, "Pin Dendrite Context").changed() {
                                    self.save_settings();
                                }
                                ui.checkbox(&mut self.web_mode, "Live Web Research");
                            });
                        },
                    );
                });

                // Card 2: Offline Voice, TTS & Cortex Visuals
                bento_frame().show(ui, |ui| {
                    ui.allocate_ui_with_layout(
                        egui::vec2(card_w - 20.0, card_h - 20.0),
                        egui::Layout::top_down(egui::Align::LEFT),
                        |ui| {
                            ui.label(egui::RichText::new("Offline Voice, TTS & Cortex Tuning").size(13.0).strong().color(Color32::WHITE));
                            ui.add_space(8.0);
                            ui.horizontal(|ui| {
                                ui.label("TTS Persona:");
                                egui::ComboBox::from_id_salt("eng_tts").selected_text(&self.tts_voice).show_ui(ui, |ui| {
                                    for &v in FEMALE_VOICES {
                                        if ui.selectable_value(&mut self.tts_voice, v.to_string(), v).clicked() {
                                            self.speech.set_voice(v.to_string());
                                            self.save_settings();
                                        }
                                    }
                                });
                            });
                            ui.horizontal(|ui| {
                                if ui.checkbox(&mut self.auto_tts, "Auto-speak replies & cards").changed() {
                                    self.save_settings();
                                }
                                if ui.checkbox(&mut self.auto_vad, "Hands-free Auto-VAD").changed() {
                                    self.save_settings();
                                }
                            });
                            if ui.add_sized([card_w - 30.0, 30.0], egui::Button::new("Test Voice Synthesis").corner_radius(12.0)).clicked() {
                                self.speech.enqueue("Nephron cognitive study engine and voice pipeline online.".into());
                            }
                            ui.separator();
                            ui.horizontal(|ui| {
                                if ui.selectable_label(self.brain.auto_rotate, "Spin Cortex").clicked() {
                                    self.brain.auto_rotate = !self.brain.auto_rotate;
                                }
                                if ui.small_button("Cycle Hue").clicked() {
                                    self.brain.palette_idx = (self.brain.palette_idx + 1) % 3;
                                    self.brain.trigger_cascade();
                                    self.save_settings();
                                }
                                ui.checkbox(&mut self.show_hw_hud, "HW Telemetry HUD");
                            });
                            ui.add(egui::Slider::new(&mut self.brain.pulse_speed, 0.3..=2.5).text("Pulse Speed"));
                            ui.add(egui::Slider::new(&mut self.brain.glitter, 0.4..=1.8).text("Star Glitter"));
                        },
                    );
                });
            });
        });
    }
}
