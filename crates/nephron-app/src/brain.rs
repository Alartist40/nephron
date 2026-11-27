//! Fast, pinprick-glitter 3D Brain (MindCorpus style) with mouse-hover glow.

use eframe::egui::{self, Color32, Pos2, Rect, Stroke};
use nephron_bridge::MemoryRecord;
use std::f32::consts::{PI, TAU};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrainActivity {
    Idle,
    Listening,
    Thinking,
    Speaking,
}

#[derive(Clone, Copy)]
pub struct BrainPalette {
    pub primary: [f32; 3],
    pub secondary: [f32; 3],
    pub accent: [f32; 3],
    pub pulse: [f32; 3],
}

pub const PALETTES: [BrainPalette; 3] = [
    BrainPalette {
        primary: [56.0, 189.0, 248.0],
        secondary: [129.0, 140.0, 248.0],
        accent: [235.0, 248.0, 255.0],
        pulse: [165.0, 232.0, 255.0],
    },
    BrainPalette {
        primary: [192.0, 132.0, 252.0],
        secondary: [56.0, 189.0, 248.0],
        accent: [248.0, 224.0, 255.0],
        pulse: [232.0, 150.0, 255.0],
    },
    BrainPalette {
        primary: [52.0, 211.0, 153.0],
        secondary: [251.0, 191.0, 36.0],
        accent: [254.0, 248.0, 220.0],
        pulse: [130.0, 240.0, 200.0],
    },
];

struct CorticalNode {
    pos: [f32; 3],
    size: f32,
    phase: f32,
    is_hub: bool,
    mix_ratio: f32,
    label: Option<String>,
}

struct SynapticPathway {
    pts: [[f32; 3]; 8],
    progress: f32,
    speed: f32,
    active: bool,
    cooldown: f32,
}

pub struct BrainAtlas {
    nodes: Vec<CorticalNode>,
    pathways: Vec<SynapticPathway>,
    dust: Vec<[f32; 3]>,
    pub yaw: f32,
    pub pitch: f32,
    pub zoom: f32,
    pub auto_rotate: bool,
    pub palette_idx: usize,
    pub pulse_speed: f32,
    pub glitter: f32,
    time: f32,
    energy: f32,
    pub hovered_label: Option<String>,
}

impl Default for BrainAtlas {
    fn default() -> Self {
        let mut atlas = Self {
            nodes: Vec::new(),
            pathways: Vec::new(),
            dust: Vec::new(),
            yaw: -0.35,
            pitch: 0.08,
            zoom: 1.0,
            auto_rotate: true,
            palette_idx: 0,
            pulse_speed: 1.0,
            glitter: 1.0,
            time: 0.0,
            energy: 0.0,
            hovered_label: None,
        };
        atlas.build_anatomy(&[]);
        atlas
    }
}

fn hash01(seed: usize, salt: usize) -> f32 {
    let mut x = (seed as u32).wrapping_mul(747_796_405).wrapping_add(salt as u32 * 2_891_336_453);
    x = ((x >> ((x >> 28).wrapping_add(4))) ^ x).wrapping_mul(277_803_737);
    ((x ^ (x >> 22)) & 0x00ff_ffff) as f32 / 16_777_216.0
}

impl BrainAtlas {
    pub fn build_anatomy(&mut self, memories: &[MemoryRecord]) {
        self.nodes.clear();
        self.pathways.clear();
        self.dust.clear();

        // 900 fine pinprick cortical stars (fast 2-tri quads)
        let count = 900usize;
        for i in 0..count {
            let hemi = if i % 2 == 0 { 1.0f32 } else { -1.0f32 };
            let theta = hash01(i, 1) * TAU;
            let phi = (2.0 * hash01(i, 2) - 1.0).clamp(-1.0, 1.0).acos();
            let is_surface = hash01(i, 3) > 0.18;
            let r_base = if is_surface { 0.86 + hash01(i, 4) * 0.14 } else { 0.25 + hash01(i, 4) * 0.6 };

            let mut x = phi.sin() * theta.cos() * 1.45;
            let mut y = phi.cos() * 1.32;
            let mut z = phi.sin() * theta.sin() * 1.95;

            if y < -0.2 { y *= 0.62; }
            if z > 0.4 {
                x *= 1.0 - (z - 0.4) * 0.16;
                y *= 1.0 - (z - 0.4) * 0.10;
            }
            if y < 0.05 && y > -0.75 && z > -0.6 && z < 0.65 {
                x += x.signum() * 0.24 * (((z + 0.6) / 1.25) * PI).sin();
                y -= 0.14;
            }
            if z < -0.65 && y < -0.15 {
                x *= 0.78;
                y -= 0.25;
                z *= 0.92;
            }

            let fold_freq = 6.5;
            let folding = (x * fold_freq + z * 4.0).sin() * (y * fold_freq - z * 3.5).cos() * 0.065;
            let radial = r_base + if is_surface { folding } else { 0.0 };
            x *= radial;
            y *= radial;
            z *= radial;

            let medial_gap = 0.08 + 0.06 * y.max(0.0);
            x = x.abs() * hemi + hemi * medial_gap;

            let is_hub = hash01(i, 5) > 0.96;
            let size = if is_hub { 1.6 + hash01(i, 6) * 1.1 } else { 0.65 + hash01(i, 6) * 0.65 };
            self.nodes.push(CorticalNode {
                pos: [x, -y, z],
                size,
                phase: hash01(i, 7) * TAU,
                is_hub,
                mix_ratio: ((z + 2.0) / 4.0).clamp(0.0, 1.0),
                label: None,
            });
        }

        for (idx, mem) in memories.iter().take(40).enumerate() {
            if let Some(n) = self.nodes.get_mut((idx * 47) % count) {
                n.is_hub = true;
                n.size = 2.4;
                n.label = Some(format!("[{}] {}", mem.kind, mem.content));
            }
        }

        // 90 delicate curved Bezier synaptic pathways (8 points each = fast & smooth)
        for i in 0..90 {
            let idx_a = (hash01(i, 10) * count as f32) as usize % count;
            let mut idx_b = (hash01(i, 11) * count as f32) as usize % count;
            let want_cross = i % 4 == 0;
            let a = self.nodes[idx_a].pos;
            for attempt in 0..20 {
                let cand = (hash01(i * 31 + attempt, 12) * count as f32) as usize % count;
                let b = self.nodes[cand].pos;
                let d = ((a[0]-b[0]).powi(2) + (a[1]-b[1]).powi(2) + (a[2]-b[2]).powi(2)).sqrt();
                let same_hemi = a[0].signum() == b[0].signum();
                if (!want_cross && same_hemi && d > 0.45 && d < 1.55)
                    || (want_cross && !same_hemi && d > 0.5 && d < 1.8)
                {
                    idx_b = cand;
                    break;
                }
            }
            let b = self.nodes[idx_b].pos;
            let mid_scale = if want_cross { 0.55 } else { 0.82 };
            let mid = [(a[0] + b[0]) * 0.5 * mid_scale, (a[1] + b[1]) * 0.5 * mid_scale, (a[2] + b[2]) * 0.5 * mid_scale];
            let mut pts = [[0.0f32; 3]; 8];
            for (s, pt) in pts.iter_mut().enumerate() {
                let t = s as f32 / 7.0;
                let u = 1.0 - t;
                for k in 0..3 {
                    pt[k] = u * u * a[k] + 2.0 * u * t * mid[k] + t * t * b[k];
                }
            }
            self.pathways.push(SynapticPathway {
                pts,
                progress: hash01(i, 13),
                speed: 0.25 + hash01(i, 14) * 0.55,
                active: hash01(i, 15) > 0.3,
                cooldown: hash01(i, 16) * 2.0,
            });
        }

        for i in 0..140 {
            let r = 3.2 + hash01(i, 20) * 3.5;
            let th = hash01(i, 21) * TAU;
            let ph = (2.0 * hash01(i, 22) - 1.0).acos();
            self.dust.push([r * ph.sin() * th.cos(), r * ph.cos(), r * ph.sin() * th.sin()]);
        }
    }

    pub fn trigger_cascade(&mut self) {
        for (i, p) in self.pathways.iter_mut().enumerate() {
            if hash01(i, 99) > 0.3 {
                p.active = true;
                p.progress = hash01(i, 98) * 0.2;
                p.speed = 0.75 + hash01(i, 97) * 0.5;
            }
        }
    }

    pub fn paint(&mut self, ui: &mut egui::Ui, rect: Rect, dt: f32, activity: BrainActivity, raw_energy: f32) -> (Option<String>, bool) {
        let resp = ui.interact(rect, ui.id().with("brain_bg"), egui::Sense::click_and_drag());
        let mut interacting = false;
        if resp.dragged() {
            let d = resp.drag_delta();
            self.yaw += d.x * 0.006;
            self.pitch = (self.pitch + d.y * 0.006).clamp(-1.35, 1.35);
            interacting = true;
        } else if self.auto_rotate {
            self.yaw += dt * (0.14 + self.energy * 0.25);
        }
        let clicked_label = if resp.clicked() {
            self.trigger_cascade();
            self.hovered_label.clone()
        } else {
            None
        };
        let scroll = ui.input(|i| i.smooth_scroll_delta.y);
        if resp.hovered() && scroll.abs() > 0.0 {
            self.zoom = (self.zoom * (1.0 + scroll * 0.002)).clamp(0.5, 2.5);
            interacting = true;
        }

        self.energy += (raw_energy.clamp(0.0, 1.0) - self.energy) * (1.0 - (-dt / 0.14).exp());
        let act_mul = match activity {
            BrainActivity::Idle => 1.0,
            BrainActivity::Listening => 1.6 + self.energy,
            BrainActivity::Thinking => 2.0,
            BrainActivity::Speaking => 2.2 + self.energy,
        };
        self.time += dt * act_mul;

        let painter = ui.painter_at(rect);
        let pal = PALETTES[self.palette_idx % PALETTES.len()];
        let center = rect.center();
        let scale = rect.width().min(rect.height()) * 0.195 * self.zoom;
        let (cy, sy) = (self.yaw.cos(), self.yaw.sin());
        let (cp, sp) = (self.pitch.cos(), self.pitch.sin());
        let project = |[x, y, z]: [f32; 3]| -> (Pos2, f32) {
            let rx = x * cy - z * sy;
            let rz0 = x * sy + z * cy;
            let ry = y * cp - rz0 * sp;
            let rz = y * sp + rz0 * cp;
            let persp = 6.8 / (6.8 - rz).max(1.0);
            (Pos2::new(center.x + rx * scale * persp, center.y + ry * scale * persp), ((rz + 2.0) / 4.0).clamp(0.05, 1.0))
        };

        // 1. Deep-space dust (fast 1.5px squares)
        for d in &self.dust {
            let (p, _) = project(*d);
            painter.rect_filled(Rect::from_center_size(p, egui::vec2(1.4, 1.4)), 0.0, Color32::from_rgba_unmultiplied(148, 163, 184, 42));
        }

        // 2. Whisper-thin curved Bezier synaptic pathways + bright traveling pulse heads
        for p in &mut self.pathways {
            if p.active {
                p.progress += dt * p.speed * self.pulse_speed * act_mul;
                if p.progress >= 1.25 {
                    p.active = false;
                    p.cooldown = 0.6;
                }
            } else {
                p.cooldown -= dt * self.pulse_speed;
                if p.cooldown <= 0.0 {
                    p.active = true;
                    p.progress = 0.0;
                }
            }

            let mut prev = project(p.pts[0]).0;
            for s in 1..8 {
                let cur = project(p.pts[s]).0;
                let seg_t = s as f32 / 7.0;
                let mut alpha = 0.055f32;
                if p.active {
                    let dist = (seg_t - p.progress).abs();
                    if dist < 0.24 {
                        alpha += (1.0 - dist / 0.24) * 0.55;
                    }
                }
                let col = lerp_rgb(pal.secondary, pal.primary, seg_t, (alpha * 255.0) as u8);
                painter.line_segment([prev, cur], Stroke::new(if alpha > 0.16 { 1.1_f32 } else { 0.55_f32 }, col));
                prev = cur;
            }

            if p.active && p.progress >= 0.0 && p.progress <= 1.0 {
                let idx = (p.progress * 7.0).floor() as usize;
                let frac = p.progress * 7.0 - idx as f32;
                let p0 = project(p.pts[idx.min(7)]).0;
                let p1 = project(p.pts[(idx + 1).min(7)]).0;
                let head = p0.lerp(p1, frac);
                painter.circle_filled(head, 3.0, Color32::from_rgba_unmultiplied(pal.pulse[0] as u8, pal.pulse[1] as u8, pal.pulse[2] as u8, 55));
                painter.rect_filled(Rect::from_center_size(head, egui::vec2(2.2, 2.2)), 1.0, Color32::from_rgb(240, 250, 255));
            }
        }

        // 3. Fast pinprick glitter stars + live mouse-hover glow
        let pointer = ui.input(|i| i.pointer.hover_pos());
        self.hovered_label = None;
        for n in &self.nodes {
            let (p, depth) = project(n.pos);
            if !rect.contains(p) {
                continue;
            }
            let sparkle = 0.55 + 0.45 * (self.time * 2.8 + n.phase * 3.0).sin();
            let flash = if (self.time * 1.9 + n.phase * 7.0).sin() > 0.85 {
                (self.time * 1.9 + n.phase * 7.0).sin().powi(6) * 1.6
            } else {
                0.0
            };
            let hover_boost = pointer
                .filter(|ptr| (p.x - ptr.x).abs() < 72.0 && (p.y - ptr.y).abs() < 72.0)
                .map(|ptr| (1.0 - p.distance(ptr) / 72.0).max(0.0).powi(2) * 1.6)
                .unwrap_or(0.0);

            let alpha = ((0.22 + sparkle * 0.35 + flash * 0.45 + hover_boost * 0.55) * depth * self.glitter).clamp(0.06, 1.0);
            let sz = n.size * (0.65 + depth * 0.4 + flash * 0.35 + hover_boost * 0.85) * self.glitter.min(1.4);

            let mut c = lerp_rgb(pal.primary, pal.secondary, n.mix_ratio, (alpha * 255.0) as u8);
            if n.is_hub || flash > 0.6 || hover_boost > 0.25 {
                let k = (if n.is_hub { 0.65 } else { 0.0 } + flash * 0.35 + hover_boost * 0.55).min(1.0);
                c = lerp_rgb([c.r() as f32, c.g() as f32, c.b() as f32], pal.accent, k, (alpha * 255.0) as u8);
            }
            if hover_boost > 0.3 || (n.is_hub && flash > 0.8) {
                painter.circle_filled(p, sz * 2.4, Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), (alpha * 42.0) as u8));
            }
            painter.rect_filled(Rect::from_center_size(p, egui::vec2(sz, sz)), 0.0, c);

            if let (Some(lbl), Some(ptr)) = (&n.label, pointer) {
                if (p.x - ptr.x).abs() < 14.0 && (p.y - ptr.y).abs() < 14.0 && p.distance(ptr) < 14.0 {
                    self.hovered_label = Some(lbl.clone());
                }
            }
        }

        if let Some(lbl) = &self.hovered_label {
            painter.text(
                Pos2::new(rect.center().x, rect.top() + 26.0),
                egui::Align2::CENTER_TOP,
                lbl,
                egui::FontId::proportional(13.0),
                Color32::from_rgb(224, 242, 254),
            );
        }
        (clicked_label, interacting)
    }
}

fn lerp_rgb(a: [f32; 3], b: [f32; 3], t: f32, alpha: u8) -> Color32 {
    let k = t.clamp(0.0, 1.0);
    Color32::from_rgba_unmultiplied(
        (a[0] + (b[0] - a[0]) * k) as u8,
        (a[1] + (b[1] - a[1]) * k) as u8,
        (a[2] + (b[2] - a[2]) * k) as u8,
        alpha,
    )
}
