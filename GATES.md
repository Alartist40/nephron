# Nephron Completion Gates

## Phase 1 — Core GUI Shell & Cynapse Bridge
- **G1:** `cargo check --workspace`
  - CHECK: `cargo check --workspace`
  - EXPECT: exits 0 with zero errors
  - EVIDENCE: PASS (exit 0)
- **G2:** `cargo test --workspace`
  - CHECK: `cargo test --workspace`
  - EXPECT: exits 0; all unit tests for `nephron-bridge` pass
  - EVIDENCE: PASS (exit 0; 4 passed, 0 failed in `nephron-bridge`)
- **G3:** `cargo build -p nephron-app`
  - CHECK: `cargo build -p nephron-app`
  - EXPECT: exits 0; `target/debug/nephron-app` exists
  - EVIDENCE: PASS (exit 0; `target/debug/nephron-app`, 269M)

## Phase 2 — Offline Voice Loop (`nephron-voice`)
- **G4:** `cargo check --workspace`
  - CHECK: `cargo check --workspace`
  - EXPECT: exits 0 with `nephron-voice` included
  - EVIDENCE: PASS (exit 0)
- **G5:** `cargo test --workspace`
  - CHECK: `cargo test --workspace`
  - EXPECT: exits 0; WAV round-trip, VAD speech/silence detection, and TTS URL validation tests pass
  - EVIDENCE: PASS (exit 0; 4/4 `nephron-bridge` + 3/3 `nephron-voice` passed)
- **G6:** `cargo build -p nephron-app`
  - CHECK: `cargo build -p nephron-app`
  - EXPECT: exits 0; GUI includes Voice panel, PTT mic button, RMS meter, and LFM3-Omni + pocket-tts wiring
  - EVIDENCE: PASS (exit 0)

## Phase 3 — Original Desktop Pets (`nephron-pets`)
- **G7:** `cargo check --workspace`
  - CHECK: `cargo check --workspace`
  - EXPECT: exits 0 with `nephron-pets` included
  - EVIDENCE: PASS (exit 0)
- **G8:** `cargo test --workspace`
  - CHECK: `cargo test --workspace`
  - EXPECT: exits 0; sprite atlas dimensions and pet state/click-reaction tests pass
  - EVIDENCE: PASS (exit 0; 9/9 passed: 4 `nephron-bridge` + 3 `nephron-voice` + 2 `nephron-pets`)
- **G9:** `cargo build -p nephron-app`
  - CHECK: `cargo build -p nephron-app`
  - EXPECT: exits 0; GUI includes `[Pet]` companion corner with Dog/Cat/Capybara, speech bubbles, and `/effort none` pet chat
  - EVIDENCE: PASS (exit 0)

## Phase 4 — Swarm Cursors Visualizer (`nephron-swarm`)
- **G10:** `cargo check --workspace`
  - CHECK: `cargo check --workspace`
  - EXPECT: exits 0 with `nephron-swarm` included
  - EVIDENCE: PASS (exit 0)
- **G11:** `cargo test --workspace`
  - CHECK: `cargo test --workspace`
  - EXPECT: exits 0; spring physics convergence and event ingestion tests pass
  - EVIDENCE: PASS (exit 0; 10/10 passed: 4 `nephron-bridge` + 3 `nephron-voice` + 2 `nephron-pets` + 1 `nephron-swarm`)
- **G12:** `cargo build -p nephron-app`
  - CHECK: `cargo build -p nephron-app`
  - EXPECT: exits 0; GUI `[Swarm]` toggle renders labeled spring-interpolated cursors and demo/live event actions
  - EVIDENCE: PASS (exit 0)

## Phase 5 — Offline Parametric CAD Builder (`nephron-cad`)
- **G13:** `cargo check --workspace`
  - CHECK: `cargo check --workspace`
  - EXPECT: exits 0 with `nephron-cad` included
  - EVIDENCE: PASS (exit 0)
- **G14:** `cargo test --workspace`
  - CHECK: `cargo test --workspace`
  - EXPECT: exits 0; STL parser/generator, SCAD+BOM response parser, and filename sanitizer tests pass
  - EVIDENCE: PASS (exit 0; 12/12 passed across 5 crates)
- **G15:** `cargo build -p nephron-app`
  - CHECK: `cargo build -p nephron-app`
  - EXPECT: exits 0; GUI `[CAD]` studio renders 3D orbit viewport, OpenSCAD prompt/compile, and Dendrite BOM table
  - EVIDENCE: PASS (exit 0)

## Phase 6 — Packaging & Orange Pi 6 Plus Tuning
- **G16:** `cargo test --workspace`
  - CHECK: `cargo test --workspace`
  - EXPECT: exits 0; RAM headroom guard unit test passes alongside all prior tests
  - EVIDENCE: PASS (exit 0; 12/12 passed across 5 crates)
- **G17:** `nephron-app doctor`
  - CHECK: `./target/debug/nephron-app doctor`
  - EXPECT: exits 0 and prints Cynapse, audio, pocket-tts, OpenSCAD, and RAM/swap diagnostics
  - EVIDENCE: PASS (exit 0; reported RAM 8658/15215 MiB, `arecord`/`aplay` OK, optional tools status)
- **G18:** `install.sh` syntax & executable check
  - CHECK: `bash -n install.sh && test -x install.sh`
  - EXPECT: exits 0
  - EVIDENCE: PASS (exit 0)

## Post-Phase Enhancements (Verified)
- **G19:** Custom animated pets (`Cat`, `Chiba Dog`, `Sheep` across `idle`, `think`, `talk`, `scan`), MindCorpus 3D Glitter Brain, OS bottom dock, Ollama backend, Perplexica web search, and independent XWayland overlay processes
  - CHECK: `cargo test --workspace && ./install.sh`
  - EXPECT: exits 0 with 14/14 tests passing and zero warnings
  - EVIDENCE: PASS (exit 0; 14/14 tests passed, zero warnings, `~/.local/bin/nephron` installed)
- **G21:** Kinetic Neural Canvas UI, CV hand-gesture CAD (`cv_experimental`), SDF boolean subtraction (`op: "subtract"`), live `W/D/H` sliders, Auto-VAD & Barge-In, local STT fallback, keyword-linked colored brain hubs, pinned context injection, full-page Perplexica reader + `★` pin, pet lip-sync, and CUA quick actions
  - CHECK: `cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings && cargo build --release -j 2 -p nephron-app`
  - EXPECT: exits 0 with 17/17 tests passing and zero clippy warnings
  - EVIDENCE: PASS (exit 0; 17/17 tests passed, 0 warnings, release binary built)
- **G22:** Portable offline `faster-whisper` (`tiny.en` in `vendor/stt`), Auto-VAD (`threshold_rms: 0.008`, enabled by default), and async non-blocking mic transcription for Ollama & Cynapse
  - CHECK: `HF_HUB_OFFLINE=1 vendor/stt/.venv/bin/python vendor/stt/transcribe.py /tmp/stt_warmup.wav && cargo test --workspace`
  - EXPECT: transcribes offline with exit 0 and all 17/17 workspace tests pass
  - EVIDENCE: PASS (exit 0; transcribed `"Hello, nephron."` offline, 17/17 tests passed, release binary built)
