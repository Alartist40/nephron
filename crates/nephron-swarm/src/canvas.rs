//! `egui` overlay painter for labeled multi-agent swarm cursors.

use crate::{SwarmAction, SwarmCanvas};
use eframe::egui::{self, Color32, FontId, Pos2, Rect, Stroke};
use std::time::Duration;

pub fn draw_swarm_overlay(ui: &mut egui::Ui, rect: Rect, swarm: &mut SwarmCanvas) {
    let dt = ui.input(|i| i.stable_dt).min(0.05);
    swarm.tick(dt);
    if !swarm.cursors.is_empty() || swarm.demo_mode {
        ui.ctx().request_repaint_after(Duration::from_millis(16));
    }

    let painter = ui.painter_at(rect);
    for c in swarm.cursors.values() {
        let [r, g, b] = c.role.rgb();
        let color = Color32::from_rgb(r, g, b);
        let p = Pos2::new(
            rect.left() + c.pos[0] * rect.width(),
            rect.top() + c.pos[1] * rect.height(),
        );

        // Action visual feedback
        match c.action {
            SwarmAction::Read | SwarmAction::Search => {
                let box_rect = Rect::from_min_size(p + egui::vec2(8.0, 8.0), egui::vec2(64.0, 24.0));
                painter.rect_stroke(box_rect, 3.0, Stroke::new(1.5_f32, color), egui::StrokeKind::Outside);
            }
            SwarmAction::Edit | SwarmAction::CadBuild => {
                let dx = (c.phase.sin()) * 14.0;
                painter.line_segment(
                    [p + egui::vec2(6.0, 12.0), p + egui::vec2(26.0 + dx, 12.0)],
                    Stroke::new(2.0_f32, color),
                );
            }
            SwarmAction::Test => {
                let rad = 10.0 + c.phase.sin().abs() * 6.0;
                painter.circle_stroke(p, rad, Stroke::new(1.5_f32, color));
            }
            SwarmAction::Idle => {}
        }

        // Cursor pointer triangle + role/detail pill
        let tip = [p, p + egui::vec2(0.0, 14.0), p + egui::vec2(10.0, 10.0)];
        painter.add(egui::Shape::convex_polygon(tip.to_vec(), color, Stroke::NONE));
        let tag = if c.detail.is_empty() {
            c.role.label().to_string()
        } else {
            format!("{}: {}", c.role.label(), c.detail)
        };
        painter.text(
            p + egui::vec2(12.0, -2.0),
            egui::Align2::LEFT_TOP,
            tag,
            FontId::proportional(11.5),
            color,
        );
    }
}
