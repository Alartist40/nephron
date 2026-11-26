//! `nephron doctor` diagnostics for Cynapse, audio, pocket-tts, OpenSCAD, and RAM.

use nephron_bridge::{CynapseClient, SystemTelemetry};
use std::process::Command;

pub async fn run_doctor(endpoint: &str) {
    println!("=== Nephron Doctor ===");
    let tel = SystemTelemetry::sample();
    println!(
        "[RAM]     used={} MiB / total={} MiB (available={} MiB)",
        tel.used_mib, tel.total_mib, tel.available_mib
    );

    let cynapse_ok = match CynapseClient::new(endpoint) {
        Ok(c) => c.health().await,
        Err(_) => false,
    };
    println!(
        "[Cynapse] {} -> {}",
        endpoint,
        if cynapse_ok { "ONLINE" } else { "OFFLINE (start cynapse-engine / llama-server on :38265)" }
    );

    check_bin("arecord", "ALSA mic capture");
    check_bin("aplay", "ALSA audio playback");
    check_bin("pocket-tts", "kyutai-labs/pocket-tts CPU worker");
    check_bin("openscad", "OpenSCAD offline STL compiler");
}

fn check_bin(name: &str, role: &str) {
    let found = Command::new("which")
        .arg(name)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    let status = if found { "OK" } else { "MISSING (optional)" };
    println!("[{name:<10}] {status} — {role}");
}
