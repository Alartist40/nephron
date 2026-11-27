//! HTTP/SSE client for `cynapse-engine` / `llama-server` on `:38265`.

use crate::{ChatMessage, EffortLevel, ModelProfile, StreamEvent};
use anyhow::{bail, Result};
use futures_util::StreamExt;
use serde::Serialize;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

pub const DEFAULT_ENDPOINT: &str = "http://127.0.0.1:38265";
pub const OLLAMA_ENDPOINT: &str = "http://127.0.0.1:11434";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendKind {
    Cynapse,
    Ollama,
}

impl BackendKind {
    pub fn default_url(self) -> &'static str {
        match self {
            Self::Cynapse => DEFAULT_ENDPOINT,
            Self::Ollama => OLLAMA_ENDPOINT,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Cynapse => "Cynapse",
            Self::Ollama => "Ollama",
        }
    }
}

#[derive(Clone)]
pub struct CynapseClient {
    base_url: String,
    kind: BackendKind,
    http: reqwest::Client,
}

#[derive(Serialize)]
struct ChatRequest<'a> {
    model: &'a str,
    messages: &'a [ChatMessage],
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    reasoning_effort: Option<&'a str>,
}

impl CynapseClient {
    pub fn new(base_url: impl Into<String>) -> Result<Self> {
        Self::with_kind(base_url, BackendKind::Cynapse)
    }

    pub fn with_kind(base_url: impl Into<String>, kind: BackendKind) -> Result<Self> {
        let raw = base_url.into();
        let trimmed = raw.trim().trim_end_matches('/').to_string();
        if !trimmed.starts_with("http://") && !trimmed.starts_with("https://") {
            bail!("invalid endpoint scheme: {trimmed}");
        }
        let http = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(3))
            .build()?;
        Ok(Self {
            base_url: trimmed,
            kind,
            http,
        })
    }

    pub fn kind(&self) -> BackendKind {
        self.kind
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    pub async fn health(&self) -> bool {
        let path = match self.kind {
            BackendKind::Cynapse => "/health",
            BackendKind::Ollama => "/api/tags",
        };
        let url = format!("{}{}", self.base_url, path);
        matches!(self.http.get(url).send().await, Ok(r) if r.status().is_success())
    }

    pub async fn list_ollama_models(&self) -> Vec<String> {
        let url = format!("{}/api/tags", self.base_url);
        let Ok(resp) = self.http.get(url).send().await else {
            return Vec::new();
        };
        let Ok(val) = resp.json::<serde_json::Value>().await else {
            return Vec::new();
        };
        val.get("models")
            .and_then(|m| m.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|x| x.get("name").and_then(|n| n.as_str()).map(str::to_string))
                    .collect()
            })
            .unwrap_or_default()
    }

    pub async fn stream_chat_model(
        &self,
        model_id: &str,
        effort: EffortLevel,
        messages: Vec<ChatMessage>,
        tx: mpsc::UnboundedSender<StreamEvent>,
    ) {
        let reasoning_effort = match self.kind {
            BackendKind::Cynapse => Some(effort.as_str()),
            BackendKind::Ollama => None,
        };
        let body = ChatRequest {
            model: model_id,
            messages: &messages,
            stream: true,
            reasoning_effort,
        };
        self.post_sse(&body, tx).await;
    }

    pub async fn stream_chat(
        &self,
        model: ModelProfile,
        effort: EffortLevel,
        messages: Vec<ChatMessage>,
        tx: mpsc::UnboundedSender<StreamEvent>,
    ) {
        self.stream_chat_model(model.id(), effort, messages, tx).await;
    }

    pub async fn stream_audio_prompt(
        &self,
        model: ModelProfile,
        effort: EffortLevel,
        wav_b64: String,
        tx: mpsc::UnboundedSender<StreamEvent>,
    ) {
        self.stream_audio_prompt_model(model.id(), effort, wav_b64, tx).await;
    }

    pub async fn stream_audio_prompt_model(
        &self,
        model_id: &str,
        effort: EffortLevel,
        wav_b64: String,
        tx: mpsc::UnboundedSender<StreamEvent>,
    ) {
        let mut body = serde_json::json!({
            "model": model_id,
            "stream": true,
            "messages": [{
                "role": "user",
                "content": [
                    { "type": "input_audio", "input_audio": { "data": wav_b64, "format": "wav" } },
                    { "type": "text", "text": "Respond concisely to the spoken request." }
                ]
            }]
        });
        if self.kind == BackendKind::Cynapse {
            body["reasoning_effort"] = serde_json::json!(effort.as_str());
        }
        self.post_sse(&body, tx).await;
    }

    async fn post_sse<T: Serialize>(&self, body: &T, tx: mpsc::UnboundedSender<StreamEvent>) {
        let started = Instant::now();
        let url = format!("{}/v1/chat/completions", self.base_url);
        let resp = match self.http.post(url).json(body).send().await {
            Ok(r) if r.status().is_success() => r,
            Ok(r) => {
                let status = r.status();
                let detail = r.text().await.unwrap_or_default();
                let short: String = detail.chars().take(180).collect();
                let msg = if short.is_empty() {
                    format!("HTTP {status}")
                } else {
                    format!("HTTP {status}: {short}")
                };
                let _ = tx.send(StreamEvent::Error(msg));
                return;
            }
            Err(e) => {
                let _ = tx.send(StreamEvent::Error(format!("Connection failed: {e}")));
                return;
            }
        };

        let mut stream = resp.bytes_stream();
        let mut buf = String::new();
        let mut tokens = 0usize;
        while let Some(chunk) = stream.next().await {
            let bytes = match chunk {
                Ok(b) => b,
                Err(e) => {
                    let _ = tx.send(StreamEvent::Error(format!("Stream read error: {e}")));
                    return;
                }
            };
            buf.push_str(&String::from_utf8_lossy(&bytes));
            while let Some(pos) = buf.find('\n') {
                let line = buf[..pos].trim().to_string();
                buf.drain(..=pos);
                if let Some(delta) = parse_sse_line(&line) {
                    tokens += 1;
                    let _ = tx.send(StreamEvent::Delta(delta));
                }
            }
        }
        let trailing = buf.trim();
        if !trailing.is_empty() {
            if let Some(delta) = parse_sse_line(trailing) {
                tokens += 1;
                let _ = tx.send(StreamEvent::Delta(delta));
            }
        }
        let elapsed = started.elapsed().as_secs_f32().max(0.001);
        let _ = tx.send(StreamEvent::Done {
            tokens,
            tok_per_sec: tokens as f32 / elapsed,
        });
    }
}

/// Parses a single `data: {...}` SSE line from an OpenAI-compatible stream.
pub fn parse_sse_line(line: &str) -> Option<String> {
    let payload = line.strip_prefix("data:")?.trim();
    if payload.is_empty() || payload == "[DONE]" {
        return None;
    }
    let val: serde_json::Value = serde_json::from_str(payload).ok()?;
    val.get("choices")?
        .get(0)?
        .get("delta")?
        .get("content")?
        .as_str()
        .map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_bad_scheme() {
        assert!(CynapseClient::new("ftp://127.0.0.1:38265").is_err());
        assert!(CynapseClient::new(DEFAULT_ENDPOINT).is_ok());
    }

    #[test]
    fn parses_sse_delta_and_done() {
        let line = r#"data: {"choices":[{"delta":{"content":"hello"}}]}"#;
        assert_eq!(parse_sse_line(line).as_deref(), Some("hello"));
        assert_eq!(parse_sse_line("data: [DONE]"), None);
        assert_eq!(parse_sse_line(": keepalive"), None);
    }
}
