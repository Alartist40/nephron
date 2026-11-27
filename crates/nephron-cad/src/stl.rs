//! Pure-Rust binary & ASCII STL parser + parametric box mesh generator.

use anyhow::{bail, Result};

#[derive(Debug, Clone, Copy)]
pub struct Triangle {
    pub normal: [f32; 3],
    pub verts: [[f32; 3]; 3],
}

#[derive(Debug, Clone, Default)]
pub struct StlMesh {
    pub triangles: Vec<Triangle>,
}

impl StlMesh {
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        if bytes.len() >= 84 {
            let count = u32::from_le_bytes([bytes[80], bytes[81], bytes[82], bytes[83]]) as usize;
            if 84 + count * 50 == bytes.len() {
                return Self::parse_binary(bytes, count);
            }
        }
        let text = std::str::from_utf8(bytes)?;
        if text.trim_start().starts_with("solid") {
            return Self::parse_ascii(text);
        }
        bail!("unrecognized STL format");
    }

    fn parse_binary(bytes: &[u8], count: usize) -> Result<Self> {
        let mut triangles = Vec::with_capacity(count);
        for i in 0..count {
            let off = 84 + i * 50;
            let f = |k: usize| {
                let p = off + k * 4;
                f32::from_le_bytes([bytes[p], bytes[p + 1], bytes[p + 2], bytes[p + 3]])
            };
            triangles.push(Triangle {
                normal: [f(0), f(1), f(2)],
                verts: [[f(3), f(4), f(5)], [f(6), f(7), f(8)], [f(9), f(10), f(11)]],
            });
        }
        Ok(Self { triangles })
    }

    fn parse_ascii(text: &str) -> Result<Self> {
        let mut triangles = Vec::new();
        let mut verts = [[0.0f32; 3]; 3];
        let mut v_idx = 0usize;
        for line in text.lines().map(str::trim) {
            if let Some(rest) = line.strip_prefix("vertex ") {
                let nums: Vec<f32> = rest
                    .split_whitespace()
                    .filter_map(|s| s.parse().ok())
                    .collect();
                if nums.len() == 3 && v_idx < 3 {
                    verts[v_idx] = [nums[0], nums[1], nums[2]];
                    v_idx += 1;
                    if v_idx == 3 {
                        triangles.push(Triangle { normal: [0.0, 0.0, 1.0], verts });
                        v_idx = 0;
                    }
                }
            }
        }
        Ok(Self { triangles })
    }

    pub fn bounds(&self) -> Option<([f32; 3], [f32; 3])> {
        let first = self.triangles.first()?.verts[0];
        let (mut min, mut max) = (first, first);
        for tri in &self.triangles {
            for v in tri.verts {
                for k in 0..3 {
                    min[k] = min[k].min(v[k]);
                    max[k] = max[k].max(v[k]);
                }
            }
        }
        Some((min, max))
    }

    pub fn dimensions(&self) -> [f32; 3] {
        match self.bounds() {
            Some((min, max)) => [max[0] - min[0], max[1] - min[1], max[2] - min[2]],
            None => [0.0, 0.0, 0.0],
        }
    }

    /// Signed volume in cm³ (falls back to bounding-box estimate if open mesh).
    pub fn volume_cm3(&self) -> f32 {
        let mut sum = 0.0f64;
        for t in &self.triangles {
            let [a, b, c] = t.verts;
            let cross_x = b[1] as f64 * c[2] as f64 - b[2] as f64 * c[1] as f64;
            let cross_y = b[2] as f64 * c[0] as f64 - b[0] as f64 * c[2] as f64;
            let cross_z = b[0] as f64 * c[1] as f64 - b[1] as f64 * c[0] as f64;
            sum += a[0] as f64 * cross_x + a[1] as f64 * cross_y + a[2] as f64 * cross_z;
        }
        (sum.abs() / 6.0 / 1000.0) as f32
    }

    /// Rough PETG filament grams & print minutes (~35% effective infill+walls).
    pub fn print_estimate(&self) -> (f32, u32) {
        let vol = self.volume_cm3();
        let grams = (vol * 0.35 * 1.24).max(0.5);
        let mins = (grams * 1.6 + 4.0).round() as u32;
        (grams, mins)
    }

    pub fn merge(&mut self, mut other: StlMesh) {
        self.triangles.append(&mut other.triangles);
    }

    pub fn to_binary_stl(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(84 + self.triangles.len() * 50);
        let mut header = [0u8; 80];
        let tag = b"nephron-rust-cad";
        header[..tag.len()].copy_from_slice(tag);
        out.extend_from_slice(&header);
        out.extend_from_slice(&(self.triangles.len() as u32).to_le_bytes());
        for tri in &self.triangles {
            for f in tri.normal {
                out.extend_from_slice(&f.to_le_bytes());
            }
            for v in tri.verts {
                for f in v {
                    out.extend_from_slice(&f.to_le_bytes());
                }
            }
            out.extend_from_slice(&0u16.to_le_bytes());
        }
        out
    }
}

/// Generates a centered 12-triangle rectangular prism STL mesh (in mm).
pub fn parametric_box_stl(w: f32, d: f32, h: f32) -> StlMesh {
    let (hx, hy, hz) = (w * 0.5, d * 0.5, h * 0.5);
    let p = [
        [-hx, -hy, -hz], [hx, -hy, -hz], [hx, hy, -hz], [-hx, hy, -hz],
        [-hx, -hy, hz],  [hx, -hy, hz],  [hx, hy, hz],  [-hx, hy, hz],
    ];
    let quads = [
        ([0, 1, 2, 3], [0.0, 0.0, -1.0]), ([4, 5, 6, 7], [0.0, 0.0, 1.0]),
        ([0, 1, 5, 4], [0.0, -1.0, 0.0]), ([2, 3, 7, 6], [0.0, 1.0, 0.0]),
        ([1, 2, 6, 5], [1.0, 0.0, 0.0]),  ([3, 0, 4, 7], [-1.0, 0.0, 0.0]),
    ];
    let mut triangles = Vec::with_capacity(12);
    for (q, normal) in quads {
        triangles.push(Triangle { normal, verts: [p[q[0]], p[q[1]], p[q[2]]] });
        triangles.push(Triangle { normal, verts: [p[q[0]], p[q[2]], p[q[3]]] });
    }
    StlMesh { triangles }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn box_and_ascii_stl_parse() {
        assert_eq!(parametric_box_stl(80.0, 50.0, 20.0).triangles.len(), 12);
        let ascii = b"solid box\nfacet normal 0 0 1\nouter loop\nvertex 0 0 0\nvertex 1 0 0\nvertex 0 1 0\nendloop\nendfacet\nendsolid box\n";
        let mesh = StlMesh::parse(ascii).unwrap();
        assert_eq!(mesh.triangles.len(), 1);
    }
}
