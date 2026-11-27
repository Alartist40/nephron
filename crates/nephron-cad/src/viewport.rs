//! Interactive `egui` 3D orbit viewport for STL meshes (full-stage or embedded).

use crate::stl::{parametric_box_stl, StlMesh};
use eframe::egui::{self, Color32, Pos2, Rect, Stroke};

pub struct CadViewport {
    pub yaw: f32,
    pub pitch: f32,
    pub zoom: f32,
    pub mesh: StlMesh,
    pub backend_label: String,
    pub wireframe: bool,
    pub cull_backfaces: bool,
}

impl Default for CadViewport {
    fn default() -> Self {
        Self {
            yaw: 0.65,
            pitch: 0.45,
            zoom: 3.2,
            mesh: parametric_box_stl(85.0, 56.0, 22.0),
            backend_label: "preview".into(),
            wireframe: false,
            cull_backfaces: true,
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
        let dims = self.mesh.dimensions();
        let max_dim = dims[0].max(dims[1]).max(dims[2]).max(20.0);
        let fit_scale = (rect.width().min(rect.height()) * 0.42 / max_dim) * (self.zoom / 3.2);
        let mesh_center = self
            .mesh
            .bounds()
            .map(|(mn, mx)| [(mn[0] + mx[0]) * 0.5, (mn[1] + mx[1]) * 0.5, (mn[2] + mx[2]) * 0.5])
            .unwrap_or([0.0, 0.0, 0.0]);
        let proj = |x: f32, y: f32, z: f32| -> (Pos2, f32) {
            let cx = x - mesh_center[0];
            let cy_m = y - mesh_center[1];
            let cz = z - mesh_center[2];
            let rx = cx * cy - cy_m * sy;
            let ry = cx * sy + cy_m * cy;
            let rz = ry * sp + cz * cp;
            let py = ry * cp - cz * sp;
            (Pos2::new(center.x + rx * fit_scale, center.y - py * fit_scale), rz)
        };

        // Subtle CAD reference grid on bottom plane
        let grid_z = -max_dim * 0.55;
        let grid_span = (max_dim * 0.7).max(40.0);
        let step = (grid_span / 3.0).max(10.0);
        for i in -3..=3 {
            let gf = i as f32 * step;
            let (a, _) = proj(mesh_center[0] + gf, mesh_center[1] - grid_span, mesh_center[2] + grid_z);
            let (b, _) = proj(mesh_center[0] + gf, mesh_center[1] + grid_span, mesh_center[2] + grid_z);
            let (c, _) = proj(mesh_center[0] - grid_span, mesh_center[1] + gf, mesh_center[2] + grid_z);
            let (d, _) = proj(mesh_center[0] + grid_span, mesh_center[1] + gf, mesh_center[2] + grid_z);
            let grid_col = Color32::from_rgba_unmultiplied(79, 217, 207, 22);
            painter.line_segment([a, b], Stroke::new(1.0_f32, grid_col));
            painter.line_segment([c, d], Stroke::new(1.0_f32, grid_col));
        }

        let mut projected: Vec<(f32, [Pos2; 3], Color32)> = self
            .mesh
            .triangles
            .iter()
            .filter_map(|tri| {
                // Rotate normal into camera space; rz_n > 0 faces toward the viewer.
                let n = tri.normal;
                let n_ry = n[0] * sy + n[1] * cy;
                let n_rz = n_ry * sp + n[2] * cp;
                if self.cull_backfaces && !self.wireframe && n_rz < -0.05 {
                    return None;
                }
                let mut z_sum = 0.0f32;
                let pts = tri.verts.map(|[x, y, z]| {
                    let (p, rz) = proj(x, y, z);
                    z_sum += rz;
                    p
                });
                let shade = ((n_rz * 0.4 + 0.6).clamp(0.30, 1.0) * 195.0) as u8;
                Some((z_sum / 3.0, pts, Color32::from_rgb(shade / 3, shade, 235)))
            })
            .collect();

        projected.sort_by(|a, b| a.0.total_cmp(&b.0));
        let wire_stroke = Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(150, 235, 255, 130));
        for (_, pts, fill) in projected {
            if self.wireframe {
                painter.add(egui::Shape::closed_line(pts.to_vec(), wire_stroke));
            } else {
                painter.add(egui::Shape::convex_polygon(
                    pts.to_vec(),
                    fill.linear_multiply(0.70),
                    wire_stroke,
                ));
            }
        }

        let dims = self.mesh.dimensions();
        painter.text(
            Pos2::new(rect.center().x, rect.top() + 18.0),
            egui::Align2::CENTER_TOP,
            format!(
                "CAD Studio · {} tris · {:.1}×{:.1}×{:.1} mm ({}) — drag to orbit, scroll to zoom",
                self.mesh.triangles.len(),
                dims[0],
                dims[1],
                dims[2],
                self.backend_label
            ),
            egui::FontId::proportional(12.0),
            Color32::from_rgba_unmultiplied(185, 235, 240, 175),
        );
    }
}
