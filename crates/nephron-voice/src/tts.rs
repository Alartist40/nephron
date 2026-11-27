//! Local CPU `kyutai-labs/pocket-tts` client (HTTP sidecar or portable local CLI fallback).

use anyhow::{bail, Result};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

pub const DEFAULT_TTS_URL: &str = "http://127.0.0.1:8000";
/// Gentle female voice from Kyutai's pocket-tts VCTK catalog.
pub const DEFAULT_VOICE: &str = "fantine";
pub const FEMALE_VOICES: &[&str] = &["fantine", "cosette", "anna", "eve", "jane", "alba"];

static TEMP_SEQ: AtomicU64 = AtomicU64::new(0);

#[derive(Clone)]
pub struct PocketTtsClient {
    base_url: String,
    http: reqwest::Client,
}

impl PocketTtsClient {
    pub fn new(base_url: impl Into<String>) -> Result<Self> {
        let trimmed = base_url.into().trim().trim_end_matches('/').to_string();
        if !trimmed.starts_with("http://") && !trimmed.starts_with("https://") {
            bail!("invalid pocket-tts endpoint scheme: {trimmed}");
        }
        let http = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(2))
            .build()?;
        Ok(Self {
            base_url: trimmed,
            http,
        })
    }

    pub async fn synthesize(&self, text: &str, voice: Option<&str>) -> Result<Vec<u8>> {
        let cleaned = clean_spoken_text(text);
        if cleaned.is_empty() {
            return Ok(Vec::new());
        }
        let chosen = voice.filter(|v| !v.trim().is_empty()).unwrap_or(DEFAULT_VOICE);
        let url = format!("{}/tts", self.base_url);
        let params = [("text", cleaned.as_str()), ("voice_url", chosen)];
        if let Ok(resp) = self.http.post(&url).form(&params).send().await {
            if resp.status().is_success() {
                return Ok(resp.bytes().await?.to_vec());
            }
        }
        synthesize_cli(&cleaned, chosen)
    }
}

/// Strips markdown tokens so TTS speaks clean prose instead of "asterisk".
pub fn clean_spoken_text(raw: &str) -> String {
    let stripped = raw
        .replace("**", "")
        .replace(['*', '#', '`', '^', '~'], "");
    stripped
        .lines()
        .map(|l| {
            let t = l.trim_start();
            t.strip_prefix("- ")
                .or_else(|| t.strip_prefix("• "))
                .unwrap_or(l)
        })
        .collect::<Vec<_>>()
        .join(" ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// RAII handle for the in-memory `pocket-tts serve` HTTP sidecar.
#[derive(Default)]
pub struct PocketTtsSidecar {
    child: Option<Child>,
}

impl PocketTtsSidecar {
    pub fn spawn(port: u16, default_voice: &str) -> Self {
        let bin = resolve_pocket_tts_bin();
        let mut cmd = Command::new(&bin);
        cmd.args([
            "serve",
            "--host",
            "127.0.0.1",
            "--port",
            &port.to_string(),
            "--default-voice",
            default_voice,
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
        if let Some(cache) = resolve_hf_cache() {
            cmd.env("HF_HOME", &cache);
            cmd.env("HF_HUB_OFFLINE", "1");
        }
        Self {
            child: cmd.spawn().ok(),
        }
    }
}

impl Drop for PocketTtsSidecar {
    fn drop(&mut self) {
        if let Some(mut c) = self.child.take() {
            let _ = c.kill();
            let _ = c.wait();
        }
    }
}

pub fn resolve_pocket_tts_bin() -> PathBuf {
    if let Some(p) = std::env::var_os("NEPHRON_POCKET_TTS_BIN").map(PathBuf::from) {
        if p.is_file() {
            return p;
        }
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let sibling = dir.join("pocket-tts");
            if sibling.is_file() {
                return sibling;
            }
        }
    }
    if let Ok(cwd) = std::env::current_dir() {
        if let Some(found) = find_in_ancestors(&cwd, "vendor/pocket-tts/rust/target/release/pocket-tts") {
            return found;
        }
    }
    PathBuf::from("pocket-tts")
}

pub fn resolve_hf_cache() -> Option<PathBuf> {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let sibling = dir.join("hf_cache");
            if sibling.is_dir() {
                return Some(sibling);
            }
        }
    }
    std::env::current_dir()
        .ok()
        .and_then(|cwd| find_in_ancestors(&cwd, "vendor/pocket-tts/hf_cache"))
}

fn find_in_ancestors(start: &Path, rel: &str) -> Option<PathBuf> {
    let mut cur = Some(start);
    for _ in 0..4 {
        if let Some(dir) = cur {
            let cand = dir.join(rel);
            if cand.exists() {
                return Some(cand);
            }
            cur = dir.parent();
        }
    }
    None
}

fn synthesize_cli(text: &str, voice: &str) -> Result<Vec<u8>> {
    let seq = TEMP_SEQ.fetch_add(1, Ordering::Relaxed);
    let tmp = std::env::temp_dir().join(format!("nephron_tts_{}_{}.wav", std::process::id(), seq));
    let bin = resolve_pocket_tts_bin();
    let mut cmd = Command::new(&bin);
    cmd.args(["generate", "--voice", voice, "--text", text, "--output-path"])
        .arg(&tmp);
    if let Some(cache) = resolve_hf_cache() {
        cmd.env("HF_HOME", &cache);
        cmd.env("HF_HUB_OFFLINE", "1");
    }
    let status = cmd.status()?;
    if !status.success() {
        let _ = std::fs::remove_file(&tmp);
        bail!("pocket-tts ({}) exited with {status}", bin.display());
    }
    let bytes = std::fs::read(&tmp)?;
    let _ = std::fs::remove_file(&tmp);
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_tts_url_and_default_voice() {
        assert!(PocketTtsClient::new("file:///etc/passwd").is_err());
        assert!(PocketTtsClient::new(DEFAULT_TTS_URL).is_ok());
        assert_eq!(DEFAULT_VOICE, "fantine");
    }
}
