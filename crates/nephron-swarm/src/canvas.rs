//! `egui` overlay painter for labeled multi-agent swarm cursors.

use crate::{SwarmAction, SwarmCanvas};
use eframe::egui::{self, Color32, FontId, Pos2, Rect, Stroke};
use std::time::Duration;

pub fn draw_swarm_overlay(ui: &mut egui::Ui, rect: Rect, swarm: &mut SwarmCanvas) {
    let dt = ui.input(|i| i.stable_dt).min(0.05);
    swarm.tick(dt);
    ui.ctx().request_repaint_after(Duration::from_millis(16));

    let painter = ui.painter_at(rect);
    for c in swarm.cursors.values() {
        let [r, g, b] = c.role.rgb();
        let color = Color32::from_rgb(r, g, b);
        let p = Pos2::new(
            rect.left() + c.pos[0] * rect.width(),
            rect.top() + c.pos[1] * rect.height(),
        );

        // Subtle pulse ring around cursor tip
        if !matches!(c.action, SwarmAction::Idle) {
            let rad = 7.0 + c.phase.sin().abs() * 4.5;
            painter.circle_stroke(
                p,
                rad,
                Stroke::new(1.2_f32, Color32::from_rgba_unmultiplied(r, g, b, 110)),
            );
        }

        // Crisp pointer triangle
        let tip = [p, p + egui::vec2(0.0, 14.0), p + egui::vec2(10.0, 10.0)];
        painter.add(egui::Shape::convex_polygon(tip.to_vec(), color, Stroke::NONE));

        // Clean glass pill badge offset below-right of cursor (never overlaps cursor or other elements)
        let tag = if c.detail.is_empty() {
            c.role.label().to_string()
        } else {
            format!("{} · {}", c.role.label(), c.detail)
        };
        let pill_pos = p + egui::vec2(12.0, 12.0);
        let text_galley = painter.layout_no_wrap(tag, FontId::proportional(11.0), Color32::from_rgb(241, 245, 249));
        let pill_rect = Rect::from_min_size(pill_pos, text_galley.size() + egui::vec2(12.0, 6.0));
        painter.rect_filled(pill_rect, 8.0, Color32::from_rgba_unmultiplied(10, 14, 24, 215));
        painter.rect_stroke(
            pill_rect,
            8.0,
            Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(r, g, b, 140)),
            egui::StrokeKind::Outside,
        );
        painter.galley(pill_pos + egui::vec2(6.0, 3.0), text_galley, Color32::WHITE);
    }
}
