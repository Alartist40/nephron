//! Offline voice loop: PCM/WAV, energy VAD, ALSA capture/playback, and `pocket-tts`.

pub mod audio;
pub mod pipeline;
pub mod tts;
pub mod vad;
pub mod wav;

pub use audio::{play_wav_bytes, AudioCapture};
pub use pipeline::{SentenceBuffer, SpeechPipeline};
pub use tts::{PocketTtsClient, PocketTtsSidecar};
pub use vad::{VadConfig, VadDetector, VadState};
pub use wav::{encode_pcm16_wav, wav_to_base64};
