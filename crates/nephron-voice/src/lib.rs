//! Offline voice loop: PCM/WAV, energy VAD, ALSA capture/playback, and `pocket-tts`.

pub mod audio;
pub mod tts;
pub mod vad;
pub mod wav;

pub use audio::{AudioCapture, play_wav_bytes};
pub use tts::PocketTtsClient;
pub use vad::{VadConfig, VadDetector, VadState};
pub use wav::{encode_pcm16_wav, wav_to_base64};
