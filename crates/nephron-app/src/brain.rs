//! 3D MindCorpus Prismatic Brain — gyri/sulci folding, 7-stop spectrum & regional cognitive lobe illumination.

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
struct Palette {
    prismatic: bool,
    primary: [f32; 3],
    secondary: [f32; 3],
    accent: [f32; 3],
    pulse: [f32; 3],
}

const PALETTES: [Palette; 3] = [
    Palette {
        prismatic: true,
        primary: [56.0, 189.0, 248.0],
        secondary: [192.0, 132.0, 252.0],
        accent: [255.0, 255.0, 255.0],
        pulse: [253.0, 224.0, 71.0],
    },
    Palette {
        prismatic: false,
        primary: [56.0, 189.0, 248.0],
        secondary: [129.0, 140.0, 248.0],
        accent: [224.0, 242.0, 254.0],
        pulse: [125.0, 211.0, 252.0],
    },
    Palette {
        prismatic: false,
        primary: [52.0, 211.0, 153.0],
        secondary: [251.0, 191.0, 36.0],
        accent: [254.0, 243.0, 199.0],
        pulse: [110.0, 231.0, 183.0],
    },
];

const MC_STRATA: [[f32; 3]; 7] = [
    [251.0, 113.0, 133.0], // coral
    [251.0, 146.0, 60.0],  // amber
    [250.0, 204.0, 21.0],  // gold
    [74.0, 222.0, 128.0],  // emerald
    [34.0, 211.0, 238.0],  // cyan
    [96.0, 165.0, 250.0],  // cobalt
    [192.0, 132.0, 252.0], // violet
];

fn sample_mc_spectrum(t: f32) -> [f32; 3] {
    let clamped = t.clamp(0.0, 0.9999);
    let scaled = clamped * (MC_STRATA.len() - 1) as f32;
    let idx = scaled.floor() as usize;
    let frac = scaled - idx as f32;
    let a = MC_STRATA[idx];
    let b = MC_STRATA[(idx + 1).min(MC_STRATA.len() - 1)];
    [
        a[0] + (b[0] - a[0]) * frac,
        a[1] + (b[1] - a[1]) * frac,
        a[2] + (b[2] - a[2]) * frac,
    ]
}

struct CorticalNode {
    pos: [f32; 3],
    size: f32,
    phase: f32,
    is_hub: bool,
    mix_ratio: f32,
    strata_t: f32,
    label: Option<String>,
    kind_rgb: Option<[f32; 3]>,
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
    cog_intensity: f32,
    speed_boost: f32,
    pub hovered_label: Option<String>,
}

impl Default for BrainAtlas {
    fn default() -> Self {
        let mut atlas = Self {
            nodes: Vec::new(),
            pathways: Vec::new(),
            dust: Vec::new(),
            yaw: 0.45,
            pitch: 0.14,
            zoom: 1.0,
            auto_rotate: true,
            palette_idx: 0,
            pulse_speed: 1.0,
            glitter: 1.0,
            time: 0.0,
            energy: 0.05,
            cog_intensity: 0.0,
            speed_boost: 1.0,
            hovered_label: None,
        };
        atlas.build_anatomy(&[]);
        atlas
    }
}

fn hash01(seed: usize, salt: usize) -> f32 {
    let mut x = (seed.wrapping_mul(747796405)).wrapping_add(salt.wrapping_mul(2891336453)) as u32;
    x = ((x >> ((x >> 28).wrapping_add(4))) ^ x).wrapping_mul(277803737);
    x = (x >> 22) ^ x;
    (x as f32) / (u32::MAX as f32)
}

impl BrainAtlas {
    pub fn build_anatomy(&mut self, memories: &[MemoryRecord]) {
        self.nodes.clear();
        self.pathways.clear();
        self.dust.clear();

        let count = 1500usize;
        for i in 0..count {
            let hemi = if i % 2 == 0 { 1.0f32 } else { -1.0f32 };
            let theta = hash01(i, 1) * TAU;
            let phi = (2.0 * hash01(i, 2) - 1.0).clamp(-1.0, 1.0).acos();
            let is_surface = hash01(i, 3) > 0.16;
            let r_base = if is_surface { 0.87 + hash01(i, 4) * 0.13 } else { 0.25 + hash01(i, 4) * 0.6 };

            let mut x = phi.sin() * theta.cos() * 1.45;
            let mut y = phi.cos() * 1.32;
            let mut z = phi.sin() * theta.sin() * 1.95;

            if y < -0.2 { y *= 0.62; }
            if z > 0.4 {
                x *= 1.0 - (z - 0.4) * 0.16;
                y *= 1.0 - (z - 0.4) * 0.10;
            }
            if y < 0.05 && y > -0.75 && z > -0.6 && z < 0.65 {
                x += x.signum() * 0.25 * (((z + 0.6) / 1.25) * PI).sin();
                y -= 0.14;
            }
            if z < -0.65 && y < -0.15 {
                x *= 0.78;
                y -= 0.26;
                z *= 0.92;
            }

            // Multi-harmonic gyri/sulci cortical folding
            let primary_fold = (x * 6.8 + z * 4.2).sin() * (y * 6.8 - z * 3.6).cos() * 0.068;
            let fine_sulci = (x * 13.5 - y * 11.0 + z * 9.0).sin() * 0.022;
            let radial = r_base + if is_surface { primary_fold + fine_sulci } else { 0.0 };
            x *= radial;
            y *= radial;
            z *= radial;

            let medial_gap = 0.085 + 0.065 * y.max(0.0);
            x = x.abs() * hemi + hemi * medial_gap;

            let is_hub = hash01(i, 5) > 0.925;
            let size = if is_hub { 2.6 + hash01(i, 6) * 2.0 } else { 0.85 + hash01(i, 6) * 1.35 };
            let strata_t = ((y + 1.1) * 0.34 + (z + 1.9) * 0.14 + (x * 2.5).sin() * 0.08).clamp(0.0, 0.9999);
            let (label, kind_rgb) = if is_hub && !memories.is_empty() {
                let m = &memories[i % memories.len()];
                let kcol = match m.kind.as_str() {
                    "cad" => [52.0, 211.0, 153.0],
                    "voice" => [251.0, 113.0, 133.0],
                    "web" => [192.0, 132.0, 252.0],
                    _ => [250.0, 204.0, 21.0],
                };
                (Some(m.content.chars().take(48).collect()), Some(kcol))
            } else {
                (None, None)
            };

            self.nodes.push(CorticalNode {
                pos: [x, y, z],
                size,
                phase: hash01(i, 7) * TAU,
                is_hub,
                mix_ratio: ((z + 2.0) / 4.0).clamp(0.0, 1.0),
                strata_t,
                label,
                kind_rgb,
            });
        }

        // Keyword-linked synaptic edges between memory hubs
        let hubs: Vec<(usize, String)> = self
            .nodes
            .iter()
            .enumerate()
            .filter_map(|(idx, n)| n.label.as_ref().map(|l| (idx, l.to_lowercase())))
            .take(24)
            .collect();
        for a_i in 0..hubs.len() {
            for b_i in (a_i + 1)..hubs.len() {
                let share_kw = hubs[a_i]
                    .1
                    .split(|c: char| !c.is_alphanumeric())
                    .any(|w| w.len() >= 4 && hubs[b_i].1.contains(w));
                if share_kw {
                    let a = self.nodes[hubs[a_i].0].pos;
                    let b = self.nodes[hubs[b_i].0].pos;
                    let mid = [(a[0] + b[0]) * 0.4, (a[1] + b[1]) * 0.4, (a[2] + b[2]) * 0.4];
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
                        progress: 0.2,
                        speed: 0.45,
                        active: true,
                        cooldown: 0.3,
                    });
                }
            }
        }

        for i in 0..90 {
            let idx_a = (hash01(i, 10) * count as f32) as usize % count;
            let a = self.nodes[idx_a].pos;
            let want_cross = hash01(i, 11) > 0.72;
            let mut idx_b = (idx_a + 13) % count;
            for attempt in 0..20 {
                let cand = (hash01(i * 31 + attempt, 12) * count as f32) as usize % count;
                let b = self.nodes[cand].pos;
                let d = ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt();
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

        for i in 0..120 {
            let r = 3.2 + hash01(i, 20) * 3.5;
            let th = hash01(i, 21) * TAU;
            let ph = (2.0 * hash01(i, 22) - 1.0).acos();
            self.dust.push([r * ph.sin() * th.cos(), r * ph.cos(), r * ph.sin() * th.sin()]);
        }
    }

    pub fn trigger_cascade(&mut self) {
        for p in &mut self.pathways {
            if !p.active {
                p.active = true;
                p.progress = 0.0;
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
            self.yaw += dt * (0.14 + self.energy * 0.22);
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

        // Cognitive State Engine: smooth lobe illumination & synaptic velocity
        let (target_int, target_boost, focus_a, col_a, focus_b, col_b) = match activity {
            BrainActivity::Idle => (0.0, 1.0, [0.0, 0.3, 0.8], [56.0, 189.0, 248.0], [0.0, -0.2, -0.8], [192.0, 132.0, 252.0]),
            BrainActivity::Listening => (0.85, 1.9, [1.05, -0.15, 0.25], [251.0, 113.0, 133.0], [-1.05, -0.15, 0.25], [251.0, 146.0, 60.0]),
            BrainActivity::Thinking => (1.35, 2.8, [0.0, 0.55, 1.25], [56.0, 189.0, 248.0], [0.0, 0.65, -0.65], [192.0, 132.0, 252.0]),
            BrainActivity::Speaking => (1.1, 2.1, [-0.85, 0.25, 0.85], [52.0, 211.0, 153.0], [0.75, 0.15, 0.75], [250.0, 204.0, 21.0]),
        };
        let lerp_k = 1.0 - (-dt * 4.5).exp();
        self.cog_intensity += (target_int - self.cog_intensity) * lerp_k;
        self.speed_boost += (target_boost - self.speed_boost) * lerp_k;
        self.energy += (raw_energy.clamp(0.0, 1.0) - self.energy) * (1.0 - (-dt / 0.14).exp());
        self.time += dt * (1.0 + self.energy * 0.8) * self.speed_boost;

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
            (Pos2::new(center.x + rx * scale * persp, center.y - ry * scale * persp), ((rz + 2.0) / 4.0).clamp(0.05, 1.0))
        };

        // 1. Deep-space dust (no solid disc behind brain so the cortical silhouette stays crisp)
        for d in &self.dust {
            let (p, _) = project(*d);
            if rect.contains(p) {
                painter.rect_filled(Rect::from_center_size(p, egui::vec2(1.4, 1.4)), 0.0, Color32::from_rgba_unmultiplied(148, 163, 184, 38));
            }
        }

        // 2. Curved Bezier synaptic pathways + traveling pulse heads
        let path_speed_mul = self.pulse_speed * self.speed_boost;
        for p in &mut self.pathways {
            if p.active {
                p.progress += dt * p.speed * path_speed_mul;
                if p.progress >= 1.25 {
                    p.active = false;
                    p.cooldown = 0.55;
                }
            } else {
                p.cooldown -= dt * path_speed_mul;
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
                let base_col = if pal.prismatic {
                    sample_mc_spectrum(seg_t)
                } else {
                    pal.primary
                };
                let col = lerp_rgb(pal.secondary, base_col, seg_t, (alpha * 255.0) as u8);
                painter.line_segment([prev, cur], Stroke::new(if alpha > 0.16 { 1.1_f32 } else { 0.55_f32 }, col));
                prev = cur;
            }

            if p.active && p.progress >= 0.0 && p.progress <= 1.0 {
                let idx = (p.progress * 7.0).floor() as usize;
                let frac = p.progress * 7.0 - idx as f32;
                let p0 = project(p.pts[idx.min(7)]).0;
                let p1 = project(p.pts[(idx + 1).min(7)]).0;
                let head = p0.lerp(p1, frac);
                painter.circle_filled(head, 3.0, Color32::from_rgba_unmultiplied(pal.pulse[0] as u8, pal.pulse[1] as u8, pal.pulse[2] as u8, 60));
                painter.rect_filled(Rect::from_center_size(head, egui::vec2(2.2, 2.2)), 0.0, Color32::from_rgb(240, 250, 255));
            }
        }

        // 3. Cortical glitter stars + regional cognitive lobe illumination
        let pointer = ui.input(|i| i.pointer.hover_pos());
        self.hovered_label = None;
        for n in &self.nodes {
            let (p, depth) = project(n.pos);
            if !rect.contains(p) {
                continue;
            }
            let sparkle = 0.55 + 0.45 * (self.time * 2.6 + n.phase * 3.0).sin();
            let flash = if (self.time * 1.8 + n.phase * 7.0).sin() > 0.86 {
                (self.time * 1.8 + n.phase * 7.0).sin().powi(6) * 1.5
            } else {
                0.0
            };
            let hover_boost = pointer
                .filter(|ptr| (p.x - ptr.x).abs() < 72.0 && (p.y - ptr.y).abs() < 72.0)
                .map(|ptr| (1.0 - p.distance(ptr) / 72.0).max(0.0).powi(2) * 1.6)
                .unwrap_or(0.0);

            let (reg_a, reg_b) = if self.cog_intensity > 0.02 {
                let da = dist3(n.pos, focus_a);
                let db = dist3(n.pos, focus_b);
                let wa = 0.65 + 0.35 * (self.time * 4.5 + n.pos[2] * 2.5).sin();
                let wb = 0.65 + 0.35 * (self.time * 3.8 - n.pos[0] * 2.5).cos();
                (
                    ((1.45 - da) / 1.45).clamp(0.0, 1.0) * self.cog_intensity * wa,
                    ((1.35 - db) / 1.35).clamp(0.0, 1.0) * self.cog_intensity * wb,
                )
            } else {
                (0.0, 0.0)
            };

            let alpha = ((0.24 + sparkle * 0.34 + flash * 0.42 + hover_boost * 0.5 + (reg_a + reg_b) * 0.35) * depth * self.glitter).clamp(0.06, 1.0);
            let sz = n.size * (0.65 + depth * 0.4 + flash * 0.3 + hover_boost * 0.8 + (reg_a + reg_b) * 0.35) * self.glitter.min(1.4);

            let mut rgb = if pal.prismatic {
                sample_mc_spectrum(n.strata_t)
            } else {
                lerp_rgb_raw(pal.primary, pal.secondary, n.mix_ratio)
            };
            if let Some(krgb) = n.kind_rgb {
                rgb = lerp_rgb_raw(rgb, krgb, 0.72);
            } else if n.is_hub || flash > 0.6 || hover_boost > 0.25 {
                let k = (if n.is_hub { 0.58 } else { 0.0 } + flash * 0.32 + hover_boost * 0.55).min(1.0);
                rgb = lerp_rgb_raw(rgb, pal.accent, k);
            }
            if reg_a > 0.05 {
                rgb = lerp_rgb_raw(rgb, col_a, (reg_a * 0.85).min(1.0));
            }
            if reg_b > 0.05 {
                rgb = lerp_rgb_raw(rgb, col_b, (reg_b * 0.80).min(1.0));
            }

            let c = Color32::from_rgba_unmultiplied(rgb[0] as u8, rgb[1] as u8, rgb[2] as u8, (alpha * 255.0) as u8);
            if hover_boost > 0.3 || (n.is_hub && (flash > 0.8 || reg_a + reg_b > 0.55)) {
                painter.circle_filled(p, sz * 2.2, Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), (alpha * 42.0) as u8));
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

fn dist3(a: [f32; 3], b: [f32; 3]) -> f32 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
}

fn lerp_rgb_raw(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    let k = t.clamp(0.0, 1.0);
    [a[0] + (b[0] - a[0]) * k, a[1] + (b[1] - a[1]) * k, a[2] + (b[2] - a[2]) * k]
}

fn lerp_rgb(a: [f32; 3], b: [f32; 3], t: f32, alpha: u8) -> Color32 {
    let r = lerp_rgb_raw(a, b, t);
    Color32::from_rgba_unmultiplied(r[0] as u8, r[1] as u8, r[2] as u8, alpha)
}
