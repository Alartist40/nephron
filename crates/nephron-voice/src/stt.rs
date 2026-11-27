//! Native `whisper.cpp` (`whisper-cli`) offline STT with fallback to local `faster-whisper`.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static STT_SEQ: AtomicU64 = AtomicU64::new(0);

pub fn has_local_stt() -> bool {
    resolve_native_whisper().is_some() || resolve_local_python_stt().is_some()
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

fn resolve_native_whisper() -> Option<(PathBuf, PathBuf)> {
    if let (Some(bin), Some(model)) = (
        std::env::var_os("NEPHRON_STT_BIN").map(PathBuf::from),
        std::env::var_os("NEPHRON_WHISPER_MODEL").map(PathBuf::from),
    ) {
        if bin.is_file() && model.is_file() {
            return Some((bin, model));
        }
    }
    let cwd = std::env::current_dir().ok()?;
    let bin = find_in_ancestors(&cwd, "vendor/whisper.cpp/build/bin/whisper-cli")?;
    let model = find_in_ancestors(&cwd, "vendor/whisper.cpp/models/ggml-tiny.en.bin")?;
    Some((bin, model))
}

fn resolve_local_python_stt() -> Option<(PathBuf, PathBuf)> {
    let cwd = std::env::current_dir().ok()?;
    let py = find_in_ancestors(&cwd, "vendor/stt/.venv/bin/python")?;
    let script = find_in_ancestors(&cwd, "vendor/stt/transcribe.py")?;
    Some((py, script))
}

/// Transcribes WAV bytes locally using native `whisper-cli` (zero Python) or `vendor/stt`.
pub fn transcribe_wav_local(wav_bytes: &[u8]) -> Option<String> {
    let seq = STT_SEQ.fetch_add(1, Ordering::Relaxed);
    let tmp = std::env::temp_dir().join(format!("nephron_stt_{}_{}.wav", std::process::id(), seq));
    std::fs::write(&tmp, wav_bytes).ok()?;

    let out = if let Some((bin, model)) = resolve_native_whisper() {
        Command::new(&bin)
            .args(["-m"])
            .arg(&model)
            .args(["-f"])
            .arg(&tmp)
            .args(["--no-timestamps", "-nt", "-t", "4"])
            .output()
            .ok()
    } else if let Some((py, script)) = resolve_local_python_stt() {
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
    } else {
        None
    };

    let _ = std::fs::remove_file(&tmp);
    let text = String::from_utf8_lossy(&out?.stdout).trim().to_string();
    (!text.is_empty()).then_some(text)
}
