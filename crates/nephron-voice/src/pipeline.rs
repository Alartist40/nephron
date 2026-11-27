//! Pipelined sentence-by-sentence TTS synthesis & playback (adapted from `Alartist40/paraclea`).

use crate::{play_wav_bytes, PocketTtsClient};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc::{self, Sender},
    Arc,
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
    tx: Sender<String>,
    speaking: Arc<AtomicBool>,
}

impl SpeechPipeline {
    pub fn spawn(tts: PocketTtsClient, voice: String) -> Self {
        let (tx, rx) = mpsc::channel::<String>();
        let (wav_tx, wav_rx) = mpsc::channel::<Vec<u8>>();
        let speaking = Arc::new(AtomicBool::new(false));
        let speaking_play = speaking.clone();

        // Stage 1: Async-capable synthesis worker (synthesizes ahead of playback)
        thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("tts synth runtime");
            while let Ok(sentence) = rx.recv() {
                if let Ok(wav) = rt.block_on(tts.synthesize(&sentence, Some(&voice))) {
                    if !wav.is_empty() && wav_tx.send(wav).is_err() {
                        break;
                    }
                }
            }
        });

        // Stage 2: Dedicated audio playback worker
        thread::spawn(move || {
            while let Ok(wav) = wav_rx.recv() {
                speaking_play.store(true, Ordering::Relaxed);
                let _ = play_wav_bytes(&wav);
                speaking_play.store(false, Ordering::Relaxed);
            }
        });

        Self { tx, speaking }
    }

    pub fn enqueue(&self, sentence: String) {
        if !sentence.trim().is_empty() {
            let _ = self.tx.send(sentence);
        }
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
