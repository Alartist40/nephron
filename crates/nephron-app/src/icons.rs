//! Crisp 1.7px vector icons drawn via egui::Painter (zero missing-glyph tofu).

use eframe::egui::{self, Color32, Pos2, Rect, Stroke};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IconKind {
    Mic,
    Studio,
    Bridge,
    Memory,
    Cad,
    Swarm,
    Cascade,
    Cortex,
    Close,
    Audio,
    Flashcards,
    Quiz,
    Tree,
    Slides,
    Brief,
    ArrowUpRight,
    SendUp,
    SendRight,
}

pub fn paint_icon(painter: &egui::Painter, c: Pos2, scale: f32, kind: IconKind, col: Color32) {
    let s = 7.5 * scale;
    let st = Stroke::new(1.65 * scale.clamp(0.85, 1.25), col);
    let p = |dx: f32, dy: f32| Pos2::new(c.x + dx * s, c.y + dy * s);

    match kind {
        IconKind::Mic => {
            painter.rect_stroke(
                Rect::from_center_size(p(0.0, -0.2), egui::vec2(0.55 * s, 0.95 * s)),
                0.28 * s,
                st,
                egui::StrokeKind::Middle,
            );
            let arc = [p(-0.55, -0.05), p(-0.45, 0.35), p(0.0, 0.52), p(0.45, 0.35), p(0.55, -0.05)];
            for w in arc.windows(2) {
                painter.line_segment([w[0], w[1]], st);
            }
            painter.line_segment([p(0.0, 0.52), p(0.0, 0.85)], st);
            painter.line_segment([p(-0.35, 0.85), p(0.35, 0.85)], st);
        }
        IconKind::Studio => {
            let r1 = Rect::from_min_max(p(-0.85, -0.85), p(-0.1, 0.05));
            let r2 = Rect::from_min_max(p(0.1, -0.85), p(0.85, -0.3));
            let r3 = Rect::from_min_max(p(-0.85, 0.25), p(-0.1, 0.85));
            let r4 = Rect::from_min_max(p(0.1, -0.1), p(0.85, 0.85));
            for r in [r1, r2, r3, r4] {
                painter.rect_stroke(r, 2.0, st, egui::StrokeKind::Middle);
            }
        }
        IconKind::Bridge => {
            painter.rect_stroke(
                Rect::from_center_size(c, egui::vec2(1.05 * s, 1.05 * s)),
                2.0,
                st,
                egui::StrokeKind::Middle,
            );
            painter.rect_stroke(
                Rect::from_center_size(c, egui::vec2(0.45 * s, 0.45 * s)),
                1.0,
                st,
                egui::StrokeKind::Middle,
            );
            for d in [-0.28_f32, 0.28] {
                painter.line_segment([p(d, -0.52), p(d, -0.85)], st);
                painter.line_segment([p(d, 0.52), p(d, 0.85)], st);
                painter.line_segment([p(-0.52, d), p(-0.85, d)], st);
                painter.line_segment([p(0.52, d), p(0.85, d)], st);
            }
        }
        IconKind::Memory => {
            let pts = [p(-0.52, 0.85), p(-0.52, -0.8), p(0.52, -0.8), p(0.52, 0.85), p(0.0, 0.42), p(-0.52, 0.85)];
            for w in pts.windows(2) {
                painter.line_segment([w[0], w[1]], st);
            }
        }
        IconKind::Cad => {
            let top = [p(0.0, -0.85), p(0.75, -0.4), p(0.0, 0.05), p(-0.75, -0.4), p(0.0, -0.85)];
            for w in top.windows(2) {
                painter.line_segment([w[0], w[1]], st);
            }
            painter.line_segment([p(-0.75, -0.4), p(-0.75, 0.45)], st);
            painter.line_segment([p(0.75, -0.4), p(0.75, 0.45)], st);
            painter.line_segment([p(0.0, 0.05), p(0.0, 0.88)], st);
            painter.line_segment([p(-0.75, 0.45), p(0.0, 0.88)], st);
            painter.line_segment([p(0.75, 0.45), p(0.0, 0.88)], st);
        }
        IconKind::Swarm => {
            let pts = [p(-0.55, -0.7), p(-0.1, 0.68), p(0.16, 0.16), p(0.72, -0.08), p(-0.55, -0.7)];
            for w in pts.windows(2) {
                painter.line_segment([w[0], w[1]], st);
            }
            painter.line_segment([p(0.16, 0.16), p(0.68, 0.68)], st);
        }
        IconKind::Cascade => {
            let pts = [p(0.15, -0.88), p(-0.6, 0.08), p(0.0, 0.08), p(-0.15, 0.88), p(0.6, -0.08), p(0.0, -0.08), p(0.15, -0.88)];
            for w in pts.windows(2) {
                painter.line_segment([w[0], w[1]], st);
            }
        }
        IconKind::Cortex => {
            for (x, cy) in [(-0.58_f32, 0.25_f32), (0.0, -0.28), (0.58, 0.2)] {
                painter.line_segment([p(x, -0.82), p(x, cy - 0.18)], st);
                painter.line_segment([p(x, cy + 0.18), p(x, 0.82)], st);
                painter.circle_stroke(p(x, cy), 0.18 * s, st);
            }
        }
        IconKind::Close => {
            painter.line_segment([p(-0.48, -0.48), p(0.48, 0.48)], st);
            painter.line_segment([p(0.48, -0.48), p(-0.48, 0.48)], st);
        }
        IconKind::Audio => {
            for (x, h) in [(-0.55_f32, 0.35_f32), (-0.18, 0.72), (0.18, 0.5), (0.55, 0.8)] {
                painter.line_segment([p(x, -h), p(x, h)], st);
            }
        }
        IconKind::Flashcards => {
            painter.rect_stroke(Rect::from_center_size(p(-0.12, 0.12), egui::vec2(1.05 * s, 0.85 * s)), 2.0, st, egui::StrokeKind::Middle);
            painter.line_segment([p(-0.4, -0.48), p(0.62, -0.48)], st);
            painter.line_segment([p(0.62, -0.48), p(0.62, 0.3)], st);
        }
        IconKind::Quiz => {
            painter.circle_stroke(c, 0.75 * s, st);
            painter.line_segment([p(0.0, -0.35), p(0.0, 0.12)], st);
            painter.circle_filled(p(0.0, 0.42), 1.3 * scale, col);
        }
        IconKind::Tree => {
            painter.circle_stroke(p(-0.5, 0.0), 0.22 * s, st);
            painter.circle_stroke(p(0.48, -0.48), 0.22 * s, st);
            painter.circle_stroke(p(0.48, 0.48), 0.22 * s, st);
            painter.line_segment([p(-0.28, -0.08), p(0.26, -0.42)], st);
            painter.line_segment([p(-0.28, 0.08), p(0.26, 0.42)], st);
        }
        IconKind::Slides => {
            painter.rect_stroke(Rect::from_center_size(c, egui::vec2(1.35 * s, 0.95 * s)), 2.0, st, egui::StrokeKind::Middle);
            painter.line_segment([p(-0.4, -0.15), p(0.4, -0.15)], st);
            painter.line_segment([p(-0.4, 0.18), p(0.15, 0.18)], st);
        }
        IconKind::Brief => {
            painter.rect_stroke(Rect::from_center_size(c, egui::vec2(1.05 * s, 1.35 * s)), 2.0, st, egui::StrokeKind::Middle);
            painter.line_segment([p(-0.28, -0.3), p(0.28, -0.3)], st);
            painter.line_segment([p(-0.28, 0.0), p(0.28, 0.0)], st);
            painter.line_segment([p(-0.28, 0.3), p(0.12, 0.3)], st);
        }
        IconKind::ArrowUpRight => {
            painter.line_segment([p(-0.45, 0.45), p(0.45, -0.45)], st);
            painter.line_segment([p(-0.1, -0.45), p(0.45, -0.45)], st);
            painter.line_segment([p(0.45, -0.45), p(0.45, 0.1)], st);
        }
        IconKind::SendUp => {
            painter.line_segment([p(0.0, 0.55), p(0.0, -0.55)], st);
            painter.line_segment([p(-0.4, -0.15), p(0.0, -0.55)], st);
            painter.line_segment([p(0.4, -0.15), p(0.0, -0.55)], st);
        }
        IconKind::SendRight => {
            painter.line_segment([p(-0.55, 0.0), p(0.55, 0.0)], st);
            painter.line_segment([p(0.15, -0.4), p(0.55, 0.0)], st);
            painter.line_segment([p(0.15, 0.4), p(0.55, 0.0)], st);
        }
    }
}

pub fn vector_icon_button(ui: &mut egui::Ui, id: &str, size: egui::Vec2, kind: IconKind, col: Color32, tip: &str) -> bool {
    let (rect, resp) = ui.allocate_exact_size(size, egui::Sense::click());
    let hov = ui.ctx().animate_bool(egui::Id::new(id).with("hov"), resp.hovered());
    let prs = ui.ctx().animate_bool(egui::Id::new(id).with("prs"), resp.is_pointer_button_down_on());
    if hov > 0.01 {
        ui.ctx().request_repaint();
    }
    let bg = Color32::from_rgba_unmultiplied(255, 255, 255, (14.0 + hov * 24.0) as u8);
    let br = Color32::from_rgba_unmultiplied(255, 255, 255, (28.0 + hov * 45.0) as u8);
    ui.painter().rect(rect, rect.height() * 0.5, bg, Stroke::new(1.0_f32, br), egui::StrokeKind::Middle);
    paint_icon(ui.painter(), rect.center(), 0.92 + hov * 0.08 - prs * 0.08, kind, col);
    resp.on_hover_text(tip).clicked()
}
