//! OpenSCAD local compiler & LLM SCAD+BOM response parser.

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

pub fn sanitize_part_name(name: &str) -> String {
    let clean: String = name
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '_' || c == '-' { c } else { '_' })
        .collect();
    let trimmed = clean.trim_matches('_');
    if trimmed.is_empty() { "part".into() } else { trimmed.into() }
}

pub fn compile_scad_to_stl(scad_code: &str, out_dir: &Path, part_name: &str) -> Result<CompileResult> {
    std::fs::create_dir_all(out_dir)?;
    let safe = sanitize_part_name(part_name);
    let scad_path = out_dir.join(format!("{safe}.scad"));
    let stl_path = out_dir.join(format!("{safe}.stl"));
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
    Ok(CompileResult {
        mesh: parametric_box_stl(85.0, 56.0, 22.0),
        scad_path,
        stl_path,
        backend: "fallback-preview",
    })
}

pub fn parse_cad_response(text: &str) -> (Option<String>, Vec<BomItem>) {
    let scad = extract_fenced(text, "openscad").or_else(|| extract_fenced(text, "scad"));
    let bom = extract_fenced(text, "json")
        .and_then(|j| serde_json::from_str::<Vec<BomItem>>(&j).ok())
        .unwrap_or_default();
    (scad, bom)
}

fn extract_fenced(text: &str, lang: &str) -> Option<String> {
    let marker = format!("```{lang}");
    let start = text.find(&marker)? + marker.len();
    let rest = text[start..].strip_prefix('\n').unwrap_or(&text[start..]);
    let end = rest.find("```")?;
    Some(rest[..end].trim().to_string())
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
