//! Pure-Rust audio-reactive Jarvis geodesic orb (reconstructed from `ada-jarvis-public`).

use eframe::egui::{self, Color32, Pos2, Rect, Stroke};
use std::collections::HashSet;
use std::f32::consts::TAU;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrbState {
    Idle,
    Listening,
    Thinking,
    Speaking,
    Error,
}

impl OrbState {
    fn target_rgb(self) -> [f32; 3] {
        match self {
            Self::Idle => [20.0, 219.0, 232.0],      // cyan
            Self::Listening => [26.0, 245.0, 189.0], // mint-emerald
            Self::Thinking => [72.0, 150.0, 255.0],  // electric blue
            Self::Speaking => [110.0, 185.0, 255.0], // radiant violet-cyan
            Self::Error => [255.0, 125.0, 33.0],     // warm amber
        }
    }

    fn target_brightness(self) -> f32 {
        match self {
            Self::Idle => 0.95,
            Self::Listening => 1.15,
            Self::Thinking => 1.0,
            Self::Speaking => 1.25,
            Self::Error => 0.85,
        }
    }
}

pub struct JarvisOrb {
    verts: Vec<[f32; 3]>,
    edges: Vec<(usize, usize)>,
    seeds: Vec<[f32; 3]>,
    time: f32,
    energy: f32,
    rgb: [f32; 3],
    brightness: f32,
}

impl Default for JarvisOrb {
    fn default() -> Self {
        let (verts, edges) = geodesic();
        let seeds = (0..320)
            .map(|i| {
                let fi = i as f32;
                [
                    (fi * 2.399_963_2) % TAU,
                    ((fi * 0.754_877_7) % 1.0) * TAU,
                    ((fi * 0.569_840_3) % 1.0) * TAU,
                ]
            })
            .collect();
        Self {
            verts,
            edges,
            seeds,
            time: 0.0,
            energy: 0.0,
            rgb: OrbState::Idle.target_rgb(),
            brightness: 0.95,
        }
    }
}

impl JarvisOrb {
    pub fn paint(
        &mut self,
        painter: &egui::Painter,
        rect: Rect,
        dt: f32,
        state: OrbState,
        raw_energy: f32,
        listening: f32,
    ) {
        let smooth = 1.0 - (-dt / 0.12).exp();
        let target_e = raw_energy.clamp(0.0, 1.0);
        self.energy += (target_e - self.energy) * smooth;
        let trgb = state.target_rgb();
        for i in 0..3 {
            self.rgb[i] += (trgb[i] - self.rgb[i]) * (1.0 - (-dt / 0.25).exp());
        }
        self.brightness += (state.target_brightness() - self.brightness) * smooth;
        self.time += dt * (1.0 + self.energy * 0.85);

        let c = rect.center();
        let r = rect.width().min(rect.height()) * 0.34 * (1.0 + 0.02 * (self.time * 0.98).sin() + self.energy * 0.08);
        let e = self.energy;
        let t = self.time;
        let col = |a: f32| {
            let alpha = (a * self.brightness * 255.0).clamp(0.0, 255.0) as u8;
            Color32::from_rgba_unmultiplied(self.rgb[0] as u8, self.rgb[1] as u8, self.rgb[2] as u8, alpha)
        };

        // Radial atmosphere bloom
        for (scale, alpha) in [(1.35, 0.03), (1.05, 0.05), (0.65, 0.08)] {
            painter.circle_filled(c, r * scale, col(alpha + e * 0.04));
        }

        // Audio-reactive harmonic filament rings
        for band in 0..3 {
            let offset = (band as f32 - 1.0) * 0.018;
            let pts: Vec<Pos2> = (0..=96)
                .map(|i| {
                    let a = (i as f32 / 96.0) * TAU;
                    let rad = r * (0.94 + surface(a, t, e, listening) + offset);
                    Pos2::new(c.x + a.cos() * rad, c.y - a.sin() * rad)
                })
                .collect();
            painter.add(egui::Shape::line(pts, Stroke::new(1.2_f32, col(0.28 + e * 0.35))));
        }

        // Expanding pulse rings
        for i in 0..2 {
            let spread = (t * 0.14 + i as f32 * 0.5).fract();
            let rad = r * (0.42 + spread * 0.55);
            let alpha = (spread * std::f32::consts::PI).sin() * (0.10 + e * 0.14);
            painter.circle_stroke(c, rad, Stroke::new(1.0_f32, col(alpha)));
        }

        // Golden-angle particle halo
        for s in &self.seeds {
            let a = s[0] + t * 0.05;
            let tube = s[1] + t * 0.14;
            let rad = r * (0.95 + surface(a, t, e, listening) + tube.sin() * (0.035 + e * 0.025));
            let life = 0.45 + 0.45 * (s[2] + t * 0.3).sin();
            let pos = Pos2::new(c.x + a.cos() * rad, c.y - a.sin() * rad);
            painter.circle_filled(pos, 1.3 + e * 1.1, col(life * (0.35 + e * 0.35)));
        }

        // Rotating 3D geodesic core
        let (cy, sy) = ((t * 0.18).cos(), (t * 0.18).sin());
        let (cp, sp) = ((0.35 + (t * 0.11).sin() * 0.15).cos(), (0.35 + (t * 0.11).sin() * 0.15).sin());
        let core_r = r * (0.34 + e * 0.035);
        let proj: Vec<(Pos2, f32)> = self
            .verts
            .iter()
            .map(|&[x, y, z]| {
                let rx = x * cy - z * sy;
                let rz0 = x * sy + z * cy;
                let ry = y * cp - rz0 * sp;
                let rz = y * sp + rz0 * cp;
                let persp = 4.2 / (4.2 - rz);
                (Pos2::new(c.x + rx * core_r * persp, c.y - ry * core_r * persp), (rz + 1.0) * 0.5)
            })
            .collect();

        for &(a, b) in &self.edges {
            let depth = (proj[a].1 + proj[b].1) * 0.5;
            painter.line_segment([proj[a].0, proj[b].0], Stroke::new(0.9_f32, col(0.08 + depth * (0.32 + e * 0.25))));
        }
        for (p, depth) in proj {
            painter.circle_filled(p, 1.0 + depth * 0.9, col(0.15 + depth * 0.45));
        }
    }
}

fn surface(a: f32, t: f32, e: f32, listening: f32) -> f32 {
    let wave = (a * 3.0 + t * 0.57).sin() * 0.54
        + (a * 5.0 - t * 0.43).sin() * 0.30
        + (a * 8.0 + t * 0.29).sin() * 0.16;
    wave * (0.008 + e * 0.085 + listening * 0.025)
}

fn geodesic() -> (Vec<[f32; 3]>, Vec<(usize, usize)>) {
    let phi = (1.0 + 5.0f32.sqrt()) * 0.5;
    let norm = |p: [f32; 3]| -> [f32; 3] {
        let len = (p[0] * p[0] + p[1] * p[1] + p[2] * p[2]).sqrt();
        [p[0] / len, p[1] / len, p[2] / len]
    };
    let mut verts: Vec<[f32; 3]> = vec![
        [-1.0, phi, 0.0], [1.0, phi, 0.0], [-1.0, -phi, 0.0], [1.0, -phi, 0.0],
        [0.0, -1.0, phi], [0.0, 1.0, phi], [0.0, -1.0, -phi], [0.0, 1.0, -phi],
        [phi, 0.0, -1.0], [phi, 0.0, 1.0], [-phi, 0.0, -1.0], [-phi, 0.0, 1.0],
    ].into_iter().map(norm).collect();
    let mut faces = vec![
        [0, 11, 5], [0, 5, 1], [0, 1, 7], [0, 7, 10], [0, 10, 11],
        [1, 5, 9], [5, 11, 4], [11, 10, 2], [10, 7, 6], [7, 1, 8],
        [3, 9, 4], [3, 4, 2], [3, 2, 6], [3, 6, 8], [3, 8, 9],
        [4, 9, 5], [2, 4, 11], [6, 2, 10], [8, 6, 7], [9, 8, 1],
    ];
    for _ in 0..2 {
        let mut cache = std::collections::HashMap::new();
        let mut next = Vec::with_capacity(faces.len() * 4);
        let mut mid = |a: usize, b: usize| -> usize {
            let key = if a < b { (a, b) } else { (b, a) };
            *cache.entry(key).or_insert_with(|| {
                let idx = verts.len();
                verts.push(norm([
                    (verts[a][0] + verts[b][0]) * 0.5,
                    (verts[a][1] + verts[b][1]) * 0.5,
                    (verts[a][2] + verts[b][2]) * 0.5,
                ]));
                idx
            })
        };
        for [a, b, c] in faces {
            let (ab, bc, ca) = (mid(a, b), mid(b, c), mid(c, a));
            next.extend([[a, ab, ca], [b, bc, ab], [c, ca, bc], [ab, bc, ca]]);
        }
        faces = next;
    }
    let mut seen = HashSet::new();
    let mut edges = Vec::new();
    for f in faces {
        for i in 0..3 {
            let (a, b) = (f[i], f[(i + 1) % 3]);
            let key = if a < b { (a, b) } else { (b, a) };
            if seen.insert(key) {
                edges.push(key);
            }
        }
    }
    (verts, edges)
}
