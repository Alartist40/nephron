//! Pure-Rust CV hand-gesture tracker via `ffmpeg` V4L2 gray stream (inspired by `Alartist40/cv_experimental`).

use std::io::Read;
use std::process::{Child, Command, Stdio};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::thread::{self, JoinHandle};

#[derive(Debug, Clone, Copy, Default)]
pub struct GestureDelta {
    pub d_yaw: f32,
    pub d_pitch: f32,
    pub d_zoom: f32,
    pub active: bool,
}

pub struct CvGestureTracker {
    running: Arc<AtomicBool>,
    delta: Arc<Mutex<GestureDelta>>,
    handle: Option<JoinHandle<()>>,
}

impl CvGestureTracker {
    pub fn start(device: &str) -> anyhow::Result<Self> {
        let mut child = Command::new("ffmpeg")
            .args([
                "-loglevel", "quiet",
                "-f", "v4l2",
                "-video_size", "160x120",
                "-framerate", "15",
                "-i", device,
                "-vf", "format=gray",
                "-f", "rawvideo",
                "-",
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()?;
        let stdout = child.stdout.take().ok_or_else(|| anyhow::anyhow!("no ffmpeg stdout"))?;
        let running = Arc::new(AtomicBool::new(true));
        let delta = Arc::new(Mutex::new(GestureDelta::default()));
        let (r2, d2) = (running.clone(), delta.clone());
        let handle = thread::spawn(move || track_loop(child, stdout, r2, d2));
        Ok(Self { running, delta, handle: Some(handle) })
    }

    pub fn take_delta(&self) -> GestureDelta {
        let mut g = self.delta.lock().unwrap_or_else(|e| e.into_inner());
        let out = *g;
        g.d_yaw = 0.0;
        g.d_pitch = 0.0;
        g.d_zoom = 0.0;
        out
    }

    pub fn stop(mut self) {
        self.running.store(false, Ordering::SeqCst);
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
    }
}

impl Drop for CvGestureTracker {
    fn drop(&mut self) {
        self.running.store(false, Ordering::SeqCst);
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
    }
}

fn track_loop(
    mut child: Child,
    mut stdout: impl Read,
    running: Arc<AtomicBool>,
    delta: Arc<Mutex<GestureDelta>>,
) {
    const W: usize = 160;
    const H: usize = 120;
    let mut prev = vec![0u8; W * H];
    let mut curr = vec![0u8; W * H];
    let mut last_centroid: Option<(f32, f32, f32)> = None;

    if stdout.read_exact(&mut prev).is_err() {
        let _ = child.kill();
        let _ = child.wait();
        return;
    }

    while running.load(Ordering::SeqCst) {
        if stdout.read_exact(&mut curr).is_err() {
            break;
        }
        let next = compute_motion_centroid(&prev, &curr, W, H);
        if let (Some((cx, cy, area)), Some((px, py, parea))) = (next, last_centroid) {
            let mut g = delta.lock().unwrap_or_else(|e| e.into_inner());
            g.d_yaw += (cx - px) * 4.5;
            g.d_pitch += (cy - py) * 3.5;
            g.d_zoom += (area - parea) * 6.0;
            g.active = true;
        } else {
            delta.lock().unwrap_or_else(|e| e.into_inner()).active = next.is_some();
        }
        last_centroid = next;
        std::mem::swap(&mut prev, &mut curr);
    }
    let _ = child.kill();
    let _ = child.wait();
}

pub fn compute_motion_centroid(prev: &[u8], curr: &[u8], w: usize, h: usize) -> Option<(f32, f32, f32)> {
    let (mut sx, mut sy, mut count) = (0.0f32, 0.0f32, 0usize);
    for y in 0..h {
        for x in 0..w {
            let i = y * w + x;
            if prev[i].abs_diff(curr[i]) > 24 {
                sx += x as f32 / w as f32;
                sy += y as f32 / h as f32;
                count += 1;
            }
        }
    }
    let ratio = count as f32 / (w * h) as f32;
    (ratio > 0.01 && ratio < 0.65).then(|| (sx / count as f32, sy / count as f32, ratio))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_motion_centroid_shift() {
        let prev = vec![0u8; 160 * 120];
        let mut curr = vec![0u8; 160 * 120];
        for y in 40..60 {
            for x in 80..100 {
                curr[y * 160 + x] = 200;
            }
        }
        let (cx, cy, area) = compute_motion_centroid(&prev, &curr, 160, 120).unwrap();
        assert!((cx - 0.56).abs() < 0.05);
        assert!((cy - 0.41).abs() < 0.05);
        assert!(area > 0.01);
    }
}
