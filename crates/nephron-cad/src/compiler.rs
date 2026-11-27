//! OpenSCAD & pure-Rust CAD compiler + LLM CAD/BOM response parser.

use crate::scene::CadDesign;
use crate::stl::{parametric_box_stl, StlMesh};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BomItem {
    pub name: String,
    pub material: String,
    pub quantity: i32,
}

pub struct CompileResult {
    pub mesh: StlMesh,
    pub scad_path: PathBuf,
    pub stl_path: PathBuf,
    pub backend: &'static str,
}

#[derive(Debug, Clone, Default)]
pub struct ParsedCad {
    pub design: Option<CadDesign>,
    pub scad: Option<String>,
    pub bom: Vec<BomItem>,
}

pub fn sanitize_part_name(name: &str) -> String {
    let clean: String = name
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '_' || c == '-' { c } else { '_' })
        .collect();
    let trimmed = clean.trim_matches('_');
    if trimmed.is_empty() { "part".into() } else { trimmed.into() }
}

pub fn compile_scad_to_stl(scad_code: &str, out_dir: &Path, part_name: &str) -> Result<CompileResult> {
    compile_cad_bundle(scad_code, None, out_dir, part_name)
}

pub fn compile_cad_bundle(
    scad_code: &str,
    design: Option<&CadDesign>,
    out_dir: &Path,
    part_name: &str,
) -> Result<CompileResult> {
    std::fs::create_dir_all(out_dir)?;
    let safe = sanitize_part_name(part_name);
    let scad_path = out_dir.join(format!("{safe}.scad"));
    let stl_path = out_dir.join(format!("{safe}.stl"));
    if !scad_code.trim().is_empty() {
        std::fs::write(&scad_path, scad_code)?;
        let ok = Command::new("openscad")
            .args(["-o"])
            .arg(&stl_path)
            .arg(&scad_path)
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        if ok {
            let bytes = std::fs::read(&stl_path)?;
            let mesh = StlMesh::parse(&bytes)?;
            return Ok(CompileResult { mesh, scad_path, stl_path, backend: "openscad" });
        }
    }
    if let Some(d) = design {
        if let Ok(mesh) = d.build_mesh() {
            let _ = std::fs::write(&stl_path, mesh.to_binary_stl());
            return Ok(CompileResult { mesh, scad_path, stl_path, backend: "rust-cad" });
        }
    }
    Ok(CompileResult {
        mesh: parametric_box_stl(85.0, 56.0, 22.0),
        scad_path,
        stl_path,
        backend: "fallback-preview",
    })
}

pub fn parse_cad_bundle(text: &str) -> ParsedCad {
    let scad = extract_fenced(text, "openscad").or_else(|| extract_fenced(text, "scad"));
    let json_raw = extract_fenced(text, "json").or_else(|| extract_raw_json(text));
    let mut design: Option<CadDesign> = None;
    let mut bom: Vec<BomItem> = Vec::new();
    if let Some(j) = json_raw {
        if let Ok(d) = serde_json::from_str::<CadDesign>(&j) {
            bom = d.effective_bom();
            design = Some(d);
        } else if let Ok(items) = serde_json::from_str::<Vec<BomItem>>(&j) {
            bom = items;
        }
    }
    ParsedCad { design, scad, bom }
}

pub fn parse_cad_response(text: &str) -> (Option<String>, Vec<BomItem>) {
    let parsed = parse_cad_bundle(text);
    (parsed.scad, parsed.bom)
}

pub fn cad_repair_prompt(err: &str) -> String {
    format!(
        "Your previous CAD output failed validation ({err}). \
         Return ONLY a valid ```json ... ``` CadDesign block with non-empty `parts` and `primitives` \
         using kinds box/cylinder/cone/tube/sphere and positive sizes in mm (0.1..=2000)."
    )
}

fn extract_fenced(text: &str, lang: &str) -> Option<String> {
    let marker = format!("```{lang}");
    let start = text.find(&marker)? + marker.len();
    let rest = text[start..].strip_prefix('\n').unwrap_or(&text[start..]);
    let end = rest.find("```")?;
    Some(rest[..end].trim().to_string())
}

fn extract_raw_json(text: &str) -> Option<String> {
    let start = text.find('{')?;
    let end = text.rfind('}')?;
    (end > start).then(|| text[start..=end].to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitizes_paths_and_parses_cad_response() {
        assert_eq!(sanitize_part_name("../../etc/passwd"), "etc_passwd");
        let resp = "```openscad\ncube([85, 56, 22]);\n```\n```json\n[{\"name\":\"Case\",\"material\":\"PETG\",\"quantity\":1}]\n```";
        let (scad, bom) = parse_cad_response(resp);
        assert_eq!(scad.as_deref(), Some("cube([85, 56, 22]);"));
        assert_eq!(bom.len(), 1);
        assert_eq!(bom[0].name, "Case");
    }
}
