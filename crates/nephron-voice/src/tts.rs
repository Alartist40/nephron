//! Local CPU `kyutai-labs/pocket-tts` client (HTTP sidecar or CLI fallback).

use anyhow::{bail, Result};
use serde::Serialize;
use std::process::Command;
use std::time::Duration;

pub const DEFAULT_TTS_URL: &str = "http://127.0.0.1:8000";

#[derive(Clone)]
pub struct PocketTtsClient {
    base_url: String,
    http: reqwest::Client,
}

#[derive(Serialize)]
struct TtsRequest<'a> {
    text: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    voice: Option<&'a str>,
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
        let url = format!("{}/tts", self.base_url);
        let body = TtsRequest { text, voice };
        if let Ok(resp) = self.http.post(url).json(&body).send().await {
            if resp.status().is_success() {
                return Ok(resp.bytes().await?.to_vec());
            }
        }
        synthesize_cli(text)
    }
}

fn synthesize_cli(text: &str) -> Result<Vec<u8>> {
    let tmp = std::env::temp_dir().join("nephron_tts.wav");
    let status = Command::new("pocket-tts")
        .args(["generate", "--text", text, "--output-path"])
        .arg(&tmp)
        .status()?;
    if !status.success() {
        bail!("pocket-tts exited with {status}");
    }
    let bytes = std::fs::read(&tmp)?;
    let _ = std::fs::remove_file(&tmp);
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_tts_url() {
        assert!(PocketTtsClient::new("file:///etc/passwd").is_err());
        assert!(PocketTtsClient::new(DEFAULT_TTS_URL).is_ok());
    }
}
