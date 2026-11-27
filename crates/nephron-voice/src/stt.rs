//! Local Whisper / Moonshine STT fallback so text-only Ollama models understand voice.

use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static STT_SEQ: AtomicU64 = AtomicU64::new(0);

pub fn has_local_stt() -> bool {
    resolve_stt_bin().is_some()
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

/// Attempts local WAV transcription; returns `None` if no local STT binary/model is present.
pub fn transcribe_wav_local(wav_bytes: &[u8]) -> Option<String> {
    let bin = resolve_stt_bin()?;
    let seq = STT_SEQ.fetch_add(1, Ordering::Relaxed);
    let tmp = std::env::temp_dir().join(format!("nephron_stt_{}_{}.wav", std::process::id(), seq));
    std::fs::write(&tmp, wav_bytes).ok()?;

    let out = if bin.to_string_lossy().contains("whisper-cli") {
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
    };
    let _ = std::fs::remove_file(&tmp);
    let text = String::from_utf8_lossy(&out?.stdout).trim().to_string();
    (!text.is_empty()).then_some(text)
}
