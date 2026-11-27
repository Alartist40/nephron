//! Pure-Rust parametric CAD assembly schema & mesh builder (inspired by `ada-jarvis-public`).

use crate::compiler::BomItem;
use crate::stl::{parametric_box_stl, StlMesh, Triangle};
use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use std::f32::consts::TAU;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CadDesign {
    pub name: String,
    #[serde(default)]
    pub summary: String,
    pub parts: Vec<CadPartSpec>,
    #[serde(default)]
    pub bom: Vec<BomItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CadPartSpec {
    pub id: String,
    pub name: String,
    #[serde(default = "default_material")]
    pub material: String,
    #[serde(default = "default_qty")]
    pub quantity: i32,
    pub primitives: Vec<CadPrimitive>,
}

fn default_material() -> String {
    "PETG".into()
}
fn default_qty() -> i32 {
    1
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CadPrimitive {
    pub kind: String,
    /// Dimensions in mm: box=[w,d,h], pyramid=[w,d,h], cylinder=[r_bot,r_top,h], cone=[r,0,h], tube=[r_out,r_in,h], sphere=[rx,ry,rz]
    pub size: [f32; 3],
    #[serde(default)]
    pub pos: [f32; 3],
    #[serde(default)]
    pub rot_deg: [f32; 3],
    #[serde(default = "default_op")]
    pub op: String,
}

fn default_op() -> String {
    "add".into()
}

impl CadDesign {
    pub fn validate(&self) -> Result<()> {
        if self.parts.is_empty() || self.parts.len() > 16 {
            bail!("CAD design must contain 1..=16 parts");
        }
        for part in &self.parts {
            if part.primitives.is_empty() {
                bail!("part '{}' has no primitives", part.id);
            }
            for p in &part.primitives {
                if !matches!(p.kind.as_str(), "box" | "pyramid" | "cylinder" | "cone" | "tube" | "sphere") {
                    bail!("unsupported primitive kind '{}' in '{}'", p.kind, part.id);
                }
                if p.size.iter().any(|&v| !v.is_finite() || !(0.0..=2000.0).contains(&v)) {
                    bail!("primitive size out of bounds (0..=2000mm) in '{}'", part.id);
                }
                if p.pos.iter().any(|&v| !v.is_finite() || v.abs() > 5000.0) {
                    bail!("primitive pos out of bounds in '{}'", part.id);
                }
            }
        }
        Ok(())
    }

    pub fn build_mesh(&self) -> Result<StlMesh> {
        self.validate()?;
        let has_subtract = self
            .parts
            .iter()
            .any(|p| p.primitives.iter().any(|pr| pr.op == "subtract"));
        if has_subtract {
            return build_sdf_csg_mesh(&self.parts);
        }
        let mut out = StlMesh::default();
        for part in &self.parts {
            for prim in &part.primitives {
                let mut m = build_primitive(prim)?;
                transform_mesh(&mut m, prim.pos, prim.rot_deg);
                out.merge(m);
            }
        }
        if out.triangles.is_empty() {
            bail!("generated CAD mesh is empty");
        }
        Ok(out)
    }

    pub fn effective_bom(&self) -> Vec<BomItem> {
        if !self.bom.is_empty() {
            return self.bom.clone();
        }
        self.parts
            .iter()
            .map(|p| BomItem {
                name: p.name.clone(),
                material: p.material.clone(),
                quantity: p.quantity.max(1),
            })
            .collect()
    }
}

fn build_primitive(p: &CadPrimitive) -> Result<StlMesh> {
    let [a, b, c] = p.size;
    match p.kind.as_str() {
        "box" => {
            if a <= 0.0 || b <= 0.0 || c <= 0.0 {
                bail!("box requires positive [w, d, h]");
            }
            Ok(parametric_box_stl(a, b, c))
        }
        "pyramid" => Ok(pyramid_mesh(a.max(1.0), b.max(1.0), c.max(1.0))),
        "cylinder" => Ok(cylinder_mesh(a.max(0.1), b.max(0.1), c.max(0.1), 24)),
        "cone" => Ok(cylinder_mesh(a.max(0.1), 0.0, c.max(0.1), 24)),
        "tube" => Ok(tube_mesh(a.max(0.2), b.clamp(0.05, a.max(0.2) - 0.05), c.max(0.1), 24)),
        "sphere" => Ok(sphere_mesh(a.max(0.1), b.max(0.1), c.max(0.1), 12, 20)),
        other => bail!("unknown primitive '{other}'"),
    }
}

fn push_tri(tris: &mut Vec<Triangle>, a: [f32; 3], b: [f32; 3], c: [f32; 3]) {
    let u = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
    let v = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
    let n = [
        u[1] * v[2] - u[2] * v[1],
        u[2] * v[0] - u[0] * v[2],
        u[0] * v[1] - u[1] * v[0],
    ];
    let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt().max(1e-6);
    tris.push(Triangle {
        normal: [n[0] / len, n[1] / len, n[2] / len],
        verts: [a, b, c],
    });
}

fn pyramid_mesh(w: f32, d: f32, h: f32) -> StlMesh {
    let (hx, hy, hz) = (w * 0.5, d * 0.5, h * 0.5);
    let b0 = [-hx, -hy, -hz];
    let b1 = [hx, -hy, -hz];
    let b2 = [hx, hy, -hz];
    let b3 = [-hx, hy, -hz];
    let apex = [0.0, 0.0, hz];
    let mut tris = Vec::with_capacity(6);
    push_tri(&mut tris, b0, b2, b1);
    push_tri(&mut tris, b0, b3, b2);
    push_tri(&mut tris, b0, b1, apex);
    push_tri(&mut tris, b1, b2, apex);
    push_tri(&mut tris, b2, b3, apex);
    push_tri(&mut tris, b3, b0, apex);
    StlMesh { triangles: tris }
}

fn cylinder_mesh(r_bot: f32, r_top: f32, h: f32, segs: usize) -> StlMesh {
    let hz = h * 0.5;
    let mut tris = Vec::with_capacity(segs * 4);
    for i in 0..segs {
        let t0 = (i as f32 / segs as f32) * TAU;
        let t1 = ((i + 1) as f32 / segs as f32) * TAU;
        let (c0, s0, c1, s1) = (t0.cos(), t0.sin(), t1.cos(), t1.sin());
        let b0 = [r_bot * c0, r_bot * s0, -hz];
        let b1 = [r_bot * c1, r_bot * s1, -hz];
        let u0 = [r_top * c0, r_top * s0, hz];
        let u1 = [r_top * c1, r_top * s1, hz];
        push_tri(&mut tris, [0.0, 0.0, -hz], b1, b0);
        if r_top > 0.0 {
            push_tri(&mut tris, [0.0, 0.0, hz], u0, u1);
            push_tri(&mut tris, b0, b1, u1);
            push_tri(&mut tris, b0, u1, u0);
        } else {
            push_tri(&mut tris, b0, b1, [0.0, 0.0, hz]);
        }
    }
    StlMesh { triangles: tris }
}

fn tube_mesh(r_out: f32, r_in: f32, h: f32, segs: usize) -> StlMesh {
    let hz = h * 0.5;
    let mut tris = Vec::with_capacity(segs * 8);
    for i in 0..segs {
        let t0 = (i as f32 / segs as f32) * TAU;
        let t1 = ((i + 1) as f32 / segs as f32) * TAU;
        let (c0, s0, c1, s1) = (t0.cos(), t0.sin(), t1.cos(), t1.sin());
        let ob0 = [r_out * c0, r_out * s0, -hz];
        let ob1 = [r_out * c1, r_out * s1, -hz];
        let ot0 = [r_out * c0, r_out * s0, hz];
        let ot1 = [r_out * c1, r_out * s1, hz];
        let ib0 = [r_in * c0, r_in * s0, -hz];
        let ib1 = [r_in * c1, r_in * s1, -hz];
        let it0 = [r_in * c0, r_in * s0, hz];
        let it1 = [r_in * c1, r_in * s1, hz];
        // Outer wall, inner wall, top ring, bottom ring
        push_tri(&mut tris, ob0, ob1, ot1);
        push_tri(&mut tris, ob0, ot1, ot0);
        push_tri(&mut tris, ib1, ib0, it0);
        push_tri(&mut tris, ib1, it0, it1);
        push_tri(&mut tris, ot0, ot1, it1);
        push_tri(&mut tris, ot0, it1, it0);
        push_tri(&mut tris, ob1, ob0, ib0);
        push_tri(&mut tris, ob1, ib0, ib1);
    }
    StlMesh { triangles: tris }
}

fn sphere_mesh(rx: f32, ry: f32, rz: f32, rings: usize, segs: usize) -> StlMesh {
    let mut tris = Vec::with_capacity(rings * segs * 2);
    let pt = |ri: usize, si: usize| -> [f32; 3] {
        let phi = (ri as f32 / rings as f32) * std::f32::consts::PI;
        let theta = (si as f32 / segs as f32) * TAU;
        [
            rx * phi.sin() * theta.cos(),
            ry * phi.sin() * theta.sin(),
            rz * phi.cos(),
        ]
    };
    for r in 0..rings {
        for s in 0..segs {
            let (p00, p10, p01, p11) = (pt(r, s), pt(r + 1, s), pt(r, s + 1), pt(r + 1, s + 1));
            if r > 0 {
                push_tri(&mut tris, p00, p10, p01);
            }
            if r + 1 < rings {
                push_tri(&mut tris, p10, p11, p01);
            }
        }
    }
    StlMesh { triangles: tris }
}

fn transform_mesh(mesh: &mut StlMesh, pos: [f32; 3], rot_deg: [f32; 3]) {
    let (rx, ry, rz) = (
        rot_deg[0].to_radians(),
        rot_deg[1].to_radians(),
        rot_deg[2].to_radians(),
    );
    let (sx, cx) = rx.sin_cos();
    let (sy, cy) = ry.sin_cos();
    let (sz, cz) = rz.sin_cos();
    let rot = |v: [f32; 3]| -> [f32; 3] {
        let y1 = v[1] * cx - v[2] * sx;
        let z1 = v[1] * sx + v[2] * cx;
        let x2 = v[0] * cy + z1 * sy;
        let z2 = -v[0] * sy + z1 * cy;
        [x2 * cz - y1 * sz, x2 * sz + y1 * cz, z2]
    };
    for tri in &mut mesh.triangles {
        tri.normal = rot(tri.normal);
        for v in &mut tri.verts {
            let r = rot(*v);
            *v = [r[0] + pos[0], r[1] + pos[1], r[2] + pos[2]];
        }
    }
}

/// Pure-Rust voxel SDF boolean CSG mesher for `op: "add"` and `op: "subtract"` primitives.
fn build_sdf_csg_mesh(parts: &[CadPartSpec]) -> Result<StlMesh> {
    let prims: Vec<&CadPrimitive> = parts.iter().flat_map(|p| p.primitives.iter()).collect();
    let (mut mn, mut mx) = ([f32::MAX; 3], [f32::MIN; 3]);
    for p in &prims {
        if p.op == "subtract" {
            continue;
        }
        let r = p.size[0].max(p.size[1]).max(p.size[2]);
        for k in 0..3 {
            mn[k] = mn[k].min(p.pos[k] - r);
            mx[k] = mx[k].max(p.pos[k] + r);
        }
    }
    for k in 0..3 {
        mn[k] -= 2.0;
        mx[k] += 2.0;
    }
    const N: usize = 28;
    let step = [
        (mx[0] - mn[0]) / N as f32,
        (mx[1] - mn[1]) / N as f32,
        (mx[2] - mn[2]) / N as f32,
    ];
    let solid = |ix: isize, iy: isize, iz: isize| -> bool {
        if ix < 0 || iy < 0 || iz < 0 || ix >= N as isize || iy >= N as isize || iz >= N as isize {
            return false;
        }
        let pt = [
            mn[0] + (ix as f32 + 0.5) * step[0],
            mn[1] + (iy as f32 + 0.5) * step[1],
            mn[2] + (iz as f32 + 0.5) * step[2],
        ];
        let mut d = f32::MAX;
        for p in &prims {
            if p.op != "subtract" {
                d = d.min(eval_prim_sdf(p, pt));
            }
        }
        for p in &prims {
            if p.op == "subtract" {
                d = d.max(-eval_prim_sdf(p, pt));
            }
        }
        d <= 0.0
    };

    let mut tris = Vec::new();
    let dirs: [([isize; 3], [[f32; 3]; 4]); 6] = [
        ([1, 0, 0], [[1.0, 0.0, 0.0], [1.0, 1.0, 0.0], [1.0, 1.0, 1.0], [1.0, 0.0, 1.0]]),
        ([-1, 0, 0], [[0.0, 0.0, 0.0], [0.0, 0.0, 1.0], [0.0, 1.0, 1.0], [0.0, 1.0, 0.0]]),
        ([0, 1, 0], [[0.0, 1.0, 0.0], [0.0, 1.0, 1.0], [1.0, 1.0, 1.0], [1.0, 1.0, 0.0]]),
        ([0, -1, 0], [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [1.0, 0.0, 1.0], [0.0, 0.0, 1.0]]),
        ([0, 0, 1], [[0.0, 0.0, 1.0], [1.0, 0.0, 1.0], [1.0, 1.0, 1.0], [0.0, 1.0, 1.0]]),
        ([0, 0, -1], [[0.0, 0.0, 0.0], [0.0, 1.0, 0.0], [1.0, 1.0, 0.0], [1.0, 0.0, 0.0]]),
    ];
    for ix in 0..N as isize {
        for iy in 0..N as isize {
            for iz in 0..N as isize {
                if !solid(ix, iy, iz) {
                    continue;
                }
                for (dn, q) in &dirs {
                    if !solid(ix + dn[0], iy + dn[1], iz + dn[2]) {
                        let v = |c: [f32; 3]| -> [f32; 3] {
                            [
                                mn[0] + (ix as f32 + c[0]) * step[0],
                                mn[1] + (iy as f32 + c[1]) * step[1],
                                mn[2] + (iz as f32 + c[2]) * step[2],
                            ]
                        };
                        push_tri(&mut tris, v(q[0]), v(q[1]), v(q[2]));
                        push_tri(&mut tris, v(q[0]), v(q[2]), v(q[3]));
                    }
                }
            }
        }
    }
    if tris.is_empty() {
        bail!("CSG subtraction removed entire solid");
    }
    Ok(StlMesh { triangles: tris })
}

fn eval_prim_sdf(p: &CadPrimitive, pt: [f32; 3]) -> f32 {
    let q = [pt[0] - p.pos[0], pt[1] - p.pos[1], pt[2] - p.pos[2]];
    let [a, b, c] = p.size;
    match p.kind.as_str() {
        "cylinder" | "cone" => {
            let r = a.max(b).max(0.1);
            let d_xy = (q[0] * q[0] + q[1] * q[1]).sqrt() - r;
            let d_z = q[2].abs() - c * 0.5;
            d_xy.max(d_z)
        }
        "sphere" => {
            let r = a.max(b).max(c).max(0.1);
            (q[0] * q[0] + q[1] * q[1] + q[2] * q[2]).sqrt() - r
        }
        _ => {
            let dx = q[0].abs() - a * 0.5;
            let dy = q[1].abs() - b * 0.5;
            let dz = q[2].abs() - c * 0.5;
            dx.max(dy).max(dz)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_multipart_assembly_and_measures_bounds() {
        let json = r#"{
            "name": "servo_mount",
            "summary": "L-bracket with bushing",
            "parts": [
                {
                    "id": "base",
                    "name": "Base Plate",
                    "material": "PETG",
                    "quantity": 1,
                    "primitives": [
                        { "kind": "box", "size": [60, 40, 6], "pos": [0, 0, 0] },
                        { "kind": "tube", "size": [10, 5, 16], "pos": [0, 0, 11] }
                    ]
                }
            ]
        }"#;
        let design: CadDesign = serde_json::from_str(json).unwrap();
        let mesh = design.build_mesh().unwrap();
        assert!(mesh.triangles.len() > 100);
        let dims = mesh.dimensions();
        assert!((dims[0] - 60.0).abs() < 0.5);
        assert!((dims[1] - 40.0).abs() < 0.5);
        assert!((dims[2] - 22.0).abs() < 0.5);
    }
}
