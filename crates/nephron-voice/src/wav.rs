//! Pure-Rust 16-bit mono PCM RIFF WAV encoder and base64 helper.

use base64::{engine::general_purpose::STANDARD, Engine};

/// Encodes signed 16-bit PCM samples as a standard RIFF/WAVE byte buffer.
pub fn encode_pcm16_wav(samples: &[i16], sample_rate: u32) -> Vec<u8> {
    let data_bytes = (samples.len() * 2) as u32;
    let riff_size = 36u32.saturating_add(data_bytes);
    let byte_rate = sample_rate * 2;
    let mut out = Vec::with_capacity(44 + data_bytes as usize);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&riff_size.to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes()); // PCM chunk size
    out.extend_from_slice(&1u16.to_le_bytes()); // PCM format
    out.extend_from_slice(&1u16.to_le_bytes()); // mono
    out.extend_from_slice(&sample_rate.to_le_bytes());
    out.extend_from_slice(&byte_rate.to_le_bytes());
    out.extend_from_slice(&2u16.to_le_bytes()); // block align
    out.extend_from_slice(&16u16.to_le_bytes()); // bits per sample
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data_bytes.to_le_bytes());
    for &s in samples {
        out.extend_from_slice(&s.to_le_bytes());
    }
    out
}

pub fn wav_to_base64(wav_bytes: &[u8]) -> String {
    STANDARD.encode(wav_bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_valid_wav_header() {
        let samples = vec![0i16, 1000, -1000, 500];
        let wav = encode_pcm16_wav(&samples, 16_000);
        assert_eq!(&wav[0..4], b"RIFF");
        assert_eq!(&wav[8..12], b"WAVE");
        assert_eq!(&wav[36..40], b"data");
        assert_eq!(wav.len(), 44 + 8);
        assert!(!wav_to_base64(&wav).is_empty());
    }
}
