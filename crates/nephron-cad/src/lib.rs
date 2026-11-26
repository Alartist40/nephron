//! Offline parametric OpenSCAD/STL builder, 3D orbit viewport, and BOM parser.

pub mod compiler;
pub mod stl;
pub mod viewport;

pub use compiler::{compile_scad_to_stl, parse_cad_response, sanitize_part_name, BomItem, CompileResult};
pub use stl::{parametric_box_stl, StlMesh, Triangle};
pub use viewport::CadViewport;

pub const CAD_SYSTEM_PROMPT: &str = "\
You are an offline parametric CAD engineer. Given a part description:
1. Output valid OpenSCAD code inside a ```openscad ... ``` block with top-level parameters.
2. Output a JSON BOM array inside a ```json ... ``` block: [{\"name\":\"...\",\"material\":\"PETG\",\"quantity\":1}].";
