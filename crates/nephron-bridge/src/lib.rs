//! Bridge between Nephron GUI, `cynapse-engine` (:38265), and Dendrite SQLite memory.

pub mod client;
pub mod memory;
pub mod search;
pub mod telemetry;

use serde::{Deserialize, Serialize};

pub use client::{BackendKind, CynapseClient, OLLAMA_ENDPOINT};
pub use memory::{CadPart, MemoryRecord, MemoryStore};
pub use search::{format_citations_prompt, SearchResult, WebSearcher};
pub use telemetry::SystemTelemetry;

/// Supported local GGUF model profiles from the Nephron stack.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModelProfile {
    MiniCpm5_1B,
    Glia1_7B,
    Ornith9B,
    Lfm3Omni2B,
}

impl ModelProfile {
    pub const ALL: [Self; 4] = [
        Self::MiniCpm5_1B,
        Self::Glia1_7B,
        Self::Ornith9B,
        Self::Lfm3Omni2B,
    ];

    pub fn id(self) -> &'static str {
        match self {
            Self::MiniCpm5_1B => "MiniCPM5-1B",
            Self::Glia1_7B => "Glia-1.7B-v1.5",
            Self::Ornith9B => "Ornith-9B",
            Self::Lfm3Omni2B => "LFM3-Omni-2B",
        }
    }

    pub fn est_ram_gib(self) -> f32 {
        match self {
            Self::MiniCpm5_1B => 1.1,
            Self::Glia1_7B => 1.5,
            Self::Ornith9B => 6.2,
            Self::Lfm3Omni2B => 2.1,
        }
    }
}

/// Reasoning effort level forwarded to Cynapse (`/effort`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EffortLevel {
    None,
    Low,
    Med,
    High,
    Auto,
}

impl EffortLevel {
    pub const ALL: [Self; 5] = [Self::None, Self::Low, Self::Med, Self::High, Self::Auto];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Low => "low",
            Self::Med => "med",
            Self::High => "high",
            Self::Auto => "auto",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ChatRole {
    System,
    User,
    Assistant,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: ChatRole,
    pub content: String,
}

#[derive(Debug, Clone)]
pub enum StreamEvent {
    Delta(String),
    Done { tokens: usize, tok_per_sec: f32 },
    Error(String),
}
