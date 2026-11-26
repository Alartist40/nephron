//! Obsidian MindCorpus OS Dock theme (`#030407` canvas + frosted glass dock).

use eframe::egui::{self, Color32, CornerRadius, Stroke, Visuals};

pub fn apply_hud_theme(ctx: &egui::Context) {
    let mut v = Visuals::dark();
    v.panel_fill = Color32::from_rgb(3, 4, 7);
    v.window_fill = Color32::from_rgba_unmultiplied(10, 14, 24, 210);
    v.extreme_bg_color = Color32::from_rgba_unmultiplied(6, 9, 16, 220);
    v.faint_bg_color = Color32::from_rgba_unmultiplied(255, 255, 255, 10);

    let r = CornerRadius::same(10);
    v.widgets.noninteractive.bg_stroke = Stroke::NONE;
    v.widgets.noninteractive.corner_radius = r;
    v.widgets.inactive.bg_fill = Color32::from_rgba_unmultiplied(255, 255, 255, 12);
    v.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(255, 255, 255, 24));
    v.widgets.inactive.corner_radius = r;
    v.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, Color32::from_rgb(226, 232, 240));

    v.widgets.hovered.bg_fill = Color32::from_rgba_unmultiplied(56, 189, 248, 36);
    v.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(56, 189, 248, 90));
    v.widgets.hovered.corner_radius = r;

    v.widgets.active.bg_fill = Color32::from_rgba_unmultiplied(56, 189, 248, 55);
    v.widgets.active.bg_stroke = Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(56, 189, 248, 120));
    v.widgets.active.corner_radius = r;

    v.selection.bg_fill = Color32::from_rgba_unmultiplied(56, 189, 248, 60);
    v.selection.stroke = Stroke::NONE;
    ctx.set_visuals(v);
}
