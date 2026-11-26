//! Interactive `egui` 3D orbit viewport for STL meshes (full-stage or embedded).

use crate::stl::{parametric_box_stl, StlMesh};
use eframe::egui::{self, Color32, Pos2, Rect, Stroke};

pub struct CadViewport {
    pub yaw: f32,
    pub pitch: f32,
    pub zoom: f32,
    pub mesh: StlMesh,
    pub backend_label: String,
}

impl Default for CadViewport {
    fn default() -> Self {
        Self {
            yaw: 0.65,
            pitch: 0.45,
            zoom: 3.2,
            mesh: parametric_box_stl(85.0, 56.0, 22.0),
            backend_label: "preview".into(),
        }
    }
}

impl CadViewport {
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        let (rect, _) = ui.allocate_exact_size(
            egui::vec2(ui.available_width().max(240.0), 220.0),
            egui::Sense::hover(),
        );
        self.ui_rect(ui, rect);
    }

    pub fn ui_rect(&mut self, ui: &mut egui::Ui, rect: Rect) {
        let resp = ui.interact(rect, ui.id().with("cad_orbit"), egui::Sense::drag());
        if resp.dragged() {
            let d = resp.drag_delta();
            self.yaw += d.x * 0.01;
            self.pitch = (self.pitch + d.y * 0.01).clamp(-1.4, 1.4);
        }
        let scroll = ui.input(|i| i.smooth_scroll_delta.y);
        if resp.hovered() && scroll.abs() > 0.0 {
            self.zoom = (self.zoom * (1.0 + scroll * 0.002)).clamp(0.5, 10.0);
        }

        let painter = ui.painter_at(rect);
        let (cy, sy) = (self.yaw.cos(), self.yaw.sin());
        let (cp, sp) = (self.pitch.cos(), self.pitch.sin());
        let center = rect.center();
        let proj = |x: f32, y: f32, z: f32| -> (Pos2, f32) {
            let rx = x * cy - y * sy;
            let ry = x * sy + y * cy;
            let rz = ry * sp + z * cp;
            let py = ry * cp - z * sp;
            (Pos2::new(center.x + rx * self.zoom, center.y - py * self.zoom), rz)
        };

        // Subtle CAD reference grid on Z = -15
        for g in (-60..=60).step_by(20) {
            let gf = g as f32;
            let (a, _) = proj(gf, -60.0, -15.0);
            let (b, _) = proj(gf, 60.0, -15.0);
            let (c, _) = proj(-60.0, gf, -15.0);
            let (d, _) = proj(60.0, gf, -15.0);
            let grid_col = Color32::from_rgba_unmultiplied(79, 217, 207, 22);
            painter.line_segment([a, b], Stroke::new(1.0_f32, grid_col));
            painter.line_segment([c, d], Stroke::new(1.0_f32, grid_col));
        }

        let mut projected: Vec<(f32, [Pos2; 3], Color32)> = self
            .mesh
            .triangles
            .iter()
            .map(|tri| {
                let mut z_sum = 0.0f32;
                let pts = tri.verts.map(|[x, y, z]| {
                    let (p, rz) = proj(x, y, z);
                    z_sum += rz;
                    p
                });
                let shade = ((tri.normal[2] * 0.35 + 0.65).clamp(0.35, 1.0) * 195.0) as u8;
                (z_sum / 3.0, pts, Color32::from_rgb(shade / 3, shade, 235))
            })
            .collect();

        projected.sort_by(|a, b| a.0.total_cmp(&b.0));
        for (_, pts, fill) in projected {
            painter.add(egui::Shape::convex_polygon(
                pts.to_vec(),
                fill.linear_multiply(0.70),
                Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(150, 235, 255, 130)),
            ));
        }

        painter.text(
            Pos2::new(rect.center().x, rect.top() + 18.0),
            egui::Align2::CENTER_TOP,
            format!("CAD Studio · {} tris ({}) — drag to orbit, scroll to zoom", self.mesh.triangles.len(), self.backend_label),
            egui::FontId::proportional(12.0),
            Color32::from_rgba_unmultiplied(185, 235, 240, 175),
        );
    }
}
