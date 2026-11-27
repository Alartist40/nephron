//! Local Whisper / faster-whisper STT fallback so text-only Ollama models understand voice.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static STT_SEQ: AtomicU64 = AtomicU64::new(0);

pub fn has_local_stt() -> bool {
    resolve_local_python_stt().is_some() || resolve_stt_bin().is_some()
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

fn resolve_local_python_stt() -> Option<(PathBuf, PathBuf)> {
    let cwd = std::env::current_dir().ok()?;
    let py = find_in_ancestors(&cwd, "vendor/stt/.venv/bin/python")?;
    let script = find_in_ancestors(&cwd, "vendor/stt/transcribe.py")?;
    Some((py, script))
}

fn resolve_stt_bin() -> Option<PathBuf> {
    if let Some(p) = std::env::var_os("NEPHRON_STT_BIN").map(PathBuf::from) {
        if p.is_file() {
            return Some(p);
        }
    }
    for bin in ["whisper-cli", "whisper", "faster-whisper"] {
        if Command::new("which")
            .arg(bin)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
        {
            return Some(PathBuf::from(bin));
        }
    }
    None
}

/// Transcribes WAV bytes locally using `vendor/stt` (`faster-whisper`) or system `whisper-cli`.
pub fn transcribe_wav_local(wav_bytes: &[u8]) -> Option<String> {
    let seq = STT_SEQ.fetch_add(1, Ordering::Relaxed);
    let tmp = std::env::temp_dir().join(format!("nephron_stt_{}_{}.wav", std::process::id(), seq));
    std::fs::write(&tmp, wav_bytes).ok()?;

    let out = if let Some((py, script)) = resolve_local_python_stt() {
        let mut cmd = Command::new(&py);
        cmd.arg(&script).arg(&tmp);
        if let Some(dir) = script.parent() {
            let cache = dir.join("hf_cache");
            if cache.is_dir() {
                cmd.env("HF_HOME", &cache);
                cmd.env("HF_HUB_OFFLINE", "1");
            }
        }
        cmd.output().ok()
    } else if let Some(bin) = resolve_stt_bin() {
        if bin.to_string_lossy().contains("whisper-cli") {
            let model = std::env::var("NEPHRON_WHISPER_MODEL").ok()?;
            Command::new(&bin)
                .args(["-m", &model, "-f"])
                .arg(&tmp)
                .args(["--no-timestamps", "-nt"])
                .output()
                .ok()
        } else {
            Command::new(&bin)
                .arg(&tmp)
                .args(["--output_format", "txt", "--output_dir"])
                .arg(std::env::temp_dir())
                .output()
                .ok()
        }
    } else {
        None
    };

    let _ = std::fs::remove_file(&tmp);
    let text = String::from_utf8_lossy(&out?.stdout).trim().to_string();
    (!text.is_empty()).then_some(text)
}
