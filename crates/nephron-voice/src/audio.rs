//! ALSA `arecord`/`aplay` PCM capture and WAV playback (zero C-header deps).

use crate::{VadConfig, VadDetector, VadState};
use anyhow::Result;
use std::io::{Read, Write};
use std::process::{Child, Command, Stdio};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::thread::{self, JoinHandle};

pub const SAMPLE_RATE: u32 = 16_000;

pub struct AudioCapture {
    running: Arc<AtomicBool>,
    samples: Arc<Mutex<Vec<i16>>>,
    rms: Arc<Mutex<f32>>,
    vad_state: Arc<Mutex<VadState>>,
    handle: Option<JoinHandle<()>>,
}

impl AudioCapture {
    pub fn start() -> Result<Self> {
        let mut child = Command::new("arecord")
            .args(["-q", "-f", "S16_LE", "-r", "16000", "-c", "1", "-t", "raw"])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()?;
        let stdout = child.stdout.take().expect("arecord stdout");
        let running = Arc::new(AtomicBool::new(true));
        let samples = Arc::new(Mutex::new(Vec::new()));
        let rms = Arc::new(Mutex::new(0.0f32));
        let vad_state = Arc::new(Mutex::new(VadState::Silence));

        let (r2, s2, rms2, v2) = (running.clone(), samples.clone(), rms.clone(), vad_state.clone());
        let handle = thread::spawn(move || read_loop(child, stdout, r2, s2, rms2, v2));
        Ok(Self {
            running,
            samples,
            rms,
            vad_state,
            handle: Some(handle),
        })
    }

    pub fn current_rms(&self) -> f32 {
        *self.rms.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn current_vad(&self) -> VadState {
        *self.vad_state.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn stop(mut self) -> Vec<i16> {
        self.running.store(false, Ordering::SeqCst);
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
        std::mem::take(&mut *self.samples.lock().unwrap_or_else(|e| e.into_inner()))
    }
}

fn read_loop(
    mut child: Child,
    mut stdout: impl Read,
    running: Arc<AtomicBool>,
    samples: Arc<Mutex<Vec<i16>>>,
    rms: Arc<Mutex<f32>>,
    vad_state: Arc<Mutex<VadState>>,
) {
    let mut buf = [0u8; 640]; // 20ms @ 16kHz mono S16_LE
    let mut vad = VadDetector::new(VadConfig::default());
    while running.load(Ordering::SeqCst) {
        match stdout.read_exact(&mut buf) {
            Ok(()) => {
                let frame: Vec<i16> = buf
                    .chunks_exact(2)
                    .map(|c| i16::from_le_bytes([c[0], c[1]]))
                    .collect();
                let st = vad.process(&frame);
                *rms.lock().unwrap_or_else(|e| e.into_inner()) = vad.last_rms;
                *vad_state.lock().unwrap_or_else(|e| e.into_inner()) = st;
                samples.lock().unwrap_or_else(|e| e.into_inner()).extend_from_slice(&frame);
            }
            Err(_) => break,
        }
    }
    let _ = child.kill();
    let _ = child.wait();
}

pub fn play_wav_bytes(wav: &[u8]) -> Result<()> {
    let mut child = Command::new("aplay")
        .args(["-q", "-"])
        .stdin(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()?;
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(wav)?;
    }
    let _ = child.wait();
    Ok(())
}
