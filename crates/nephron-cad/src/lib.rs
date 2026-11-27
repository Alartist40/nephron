//! Offline parametric OpenSCAD + pure-Rust CAD builder, 3D orbit viewport, and BOM parser.

pub mod compiler;
pub mod scene;
pub mod stl;
pub mod viewport;

pub use compiler::{
    cad_repair_prompt, compile_cad_bundle, compile_scad_to_stl, parse_cad_bundle,
    parse_cad_response, sanitize_part_name, BomItem, CompileResult, ParsedCad,
};
pub use scene::{CadDesign, CadPartSpec, CadPrimitive};
pub use stl::{parametric_box_stl, StlMesh, Triangle};
pub use viewport::CadViewport;

pub const CAD_SYSTEM_PROMPT: &str = "\
You are an offline parametric mechanical CAD engineer (mm units). Given a part or assembly request:
1. Build ONLY what the user requested (do NOT add extra base plates or unrelated shapes).
2. Output a ```json ... ``` block matching this schema so the pure-Rust CAD engine builds the 3D STL:
{\"name\":\"short_snake_name\",\"summary\":\"short note\",\"parts\":[{\"id\":\"p1\",\"name\":\"Part\",\"material\":\"PETG\",\"quantity\":1,\"primitives\":[{\"kind\":\"box\",\"size\":[w,d,h],\"pos\":[x,y,z],\"rot_deg\":[0,0,0]}]}]}
Supported kinds: \"box\" (size=[w,d,h]), \"cylinder\" (size=[r_bot,r_top,h]), \"cone\" (size=[r,0,h]), \"tube\" (size=[r_out,r_in,h]), \"sphere\" (size=[rx,ry,rz]).
Offset stacked parts along Z/X/Y using `pos` so primitives mate cleanly without overlapping.
3. Optionally also output matching OpenSCAD code inside a ```openscad ... ``` block.";
