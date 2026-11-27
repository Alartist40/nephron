//! Pipelined sentence-by-sentence TTS synthesis & playback (adapted from `Alartist40/paraclea`).

use crate::{play_wav_bytes, PocketTtsClient};
use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    mpsc::{self, Sender},
    Arc, Mutex,
};
use std::thread;

/// Accumulates streaming LLM tokens and emits complete sentences as soon as punctuation arrives.
#[derive(Default)]
pub struct SentenceBuffer {
    buf: String,
}

impl SentenceBuffer {
    pub fn push(&mut self, delta: &str) -> Vec<String> {
        self.buf.push_str(delta);
        let mut out = Vec::new();
        while let Some(idx) = find_sentence_break(&self.buf) {
            let sentence = self.buf[..=idx].trim().to_string();
            self.buf.drain(..=idx);
            if !sentence.is_empty() {
                out.push(sentence);
            }
        }
        out
    }

    pub fn flush(&mut self) -> Option<String> {
        let rem = self.buf.trim().to_string();
        self.buf.clear();
        (!rem.is_empty()).then_some(rem)
    }
}

fn find_sentence_break(s: &str) -> Option<usize> {
    let mut chars = s.char_indices().peekable();
    while let Some((i, ch)) = chars.next() {
        if matches!(ch, '.' | '!' | '?') && i >= 16 {
            let next_ok = chars
                .peek()
                .map(|(_, c)| c.is_whitespace())
                .unwrap_or(false);
            if next_ok {
                return Some(i);
            }
        } else if ch == '\n' && i >= 12 {
            return Some(i);
        }
    }
    None
}

/// Decoupled 2-stage pipeline:
/// Stage 1 (Synthesizer) converts sentence N+1 to WAV while Stage 2 (Player) plays sentence N.
pub struct SpeechPipeline {
    tx: Sender<(u64, String)>,
    speaking: Arc<AtomicBool>,
    epoch: Arc<AtomicU64>,
    voice: Arc<Mutex<String>>,
}

impl SpeechPipeline {
    pub fn spawn(tts: PocketTtsClient, voice: String) -> Self {
        let (tx, rx) = mpsc::channel::<(u64, String)>();
        let (wav_tx, wav_rx) = mpsc::channel::<(u64, Vec<u8>)>();
        let speaking = Arc::new(AtomicBool::new(false));
        let epoch = Arc::new(AtomicU64::new(0));
        let voice_arc = Arc::new(Mutex::new(voice));
        let speaking_play = speaking.clone();
        let epoch_synth = epoch.clone();
        let epoch_play = epoch.clone();
        let voice_synth = voice_arc.clone();

        // Stage 1: Async-capable synthesis worker (synthesizes ahead of playback)
        thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("tts synth runtime");
            while let Ok((ep, sentence)) = rx.recv() {
                if ep != epoch_synth.load(Ordering::Relaxed) {
                    continue;
                }
                let v = voice_synth.lock().unwrap_or_else(|e| e.into_inner()).clone();
                if let Ok(wav) = rt.block_on(tts.synthesize(&sentence, Some(&v))) {
                    if !wav.is_empty()
                        && ep == epoch_synth.load(Ordering::Relaxed)
                        && wav_tx.send((ep, wav)).is_err()
                    {
                        break;
                    }
                }
            }
        });

        // Stage 2: Dedicated audio playback worker
        thread::spawn(move || {
            while let Ok((ep, wav)) = wav_rx.recv() {
                if ep != epoch_play.load(Ordering::Relaxed) {
                    continue;
                }
                speaking_play.store(true, Ordering::Relaxed);
                let _ = play_wav_bytes(&wav);
                speaking_play.store(false, Ordering::Relaxed);
            }
        });

        Self { tx, speaking, epoch, voice: voice_arc }
    }

    pub fn set_voice(&self, voice: String) {
        *self.voice.lock().unwrap_or_else(|e| e.into_inner()) = voice;
    }

    pub fn enqueue(&self, sentence: String) {
        if !sentence.trim().is_empty() {
            let ep = self.epoch.load(Ordering::Relaxed);
            let _ = self.tx.send((ep, sentence));
        }
    }

    /// Barge-in: immediately invalidates all queued sentences/audio.
    pub fn interrupt(&self) {
        self.epoch.fetch_add(1, Ordering::Relaxed);
        self.speaking.store(false, Ordering::Relaxed);
    }

    pub fn is_speaking(&self) -> bool {
        self.speaking.load(Ordering::Relaxed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_streaming_sentences_without_fragmenting_short_tokens() {
        let mut sb = SentenceBuffer::default();
        assert!(sb.push("Hello world, this is").is_empty());
        let done = sb.push(" a test. Next sentence");
        assert_eq!(done, vec!["Hello world, this is a test."]);
        assert_eq!(sb.flush().as_deref(), Some("Next sentence"));
    }
}
