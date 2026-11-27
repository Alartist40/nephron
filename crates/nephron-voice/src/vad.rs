//! Lightweight RMS energy Voice Activity Detector (VAD).

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VadState {
    Silence,
    Speech,
}

#[derive(Debug, Clone, Copy)]
pub struct VadConfig {
    pub threshold_rms: f32,
    pub hangover_frames: usize,
}

impl Default for VadConfig {
    fn default() -> Self {
        Self {
            threshold_rms: 0.008,
            hangover_frames: 25,
        }
    }
}

pub struct VadDetector {
    cfg: VadConfig,
    hangover: usize,
    pub last_rms: f32,
}

impl VadDetector {
    pub fn new(cfg: VadConfig) -> Self {
        Self {
            cfg,
            hangover: 0,
            last_rms: 0.0,
        }
    }

    pub fn process(&mut self, frame: &[i16]) -> VadState {
        let rms = frame_rms(frame);
        self.last_rms = rms;
        if rms >= self.cfg.threshold_rms {
            self.hangover = self.cfg.hangover_frames;
            VadState::Speech
        } else if self.hangover > 0 {
            self.hangover -= 1;
            VadState::Speech
        } else {
            VadState::Silence
        }
    }
}

pub fn frame_rms(frame: &[i16]) -> f32 {
    if frame.is_empty() {
        return 0.0;
    }
    let sum_sq: f64 = frame
        .iter()
        .map(|&s| {
            let norm = f64::from(s) / 32768.0;
            norm * norm
        })
        .sum();
    (sum_sq / frame.len() as f64).sqrt() as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_speech_and_hangover() {
        let mut vad = VadDetector::new(VadConfig {
            threshold_rms: 0.05,
            hangover_frames: 2,
        });
        let silence = vec![0i16; 320];
        let loud = vec![8000i16; 320];
        assert_eq!(vad.process(&silence), VadState::Silence);
        assert_eq!(vad.process(&loud), VadState::Speech);
        assert_eq!(vad.process(&silence), VadState::Speech); // hangover 1
        assert_eq!(vad.process(&silence), VadState::Speech); // hangover 2
        assert_eq!(vad.process(&silence), VadState::Silence);
    }
}
