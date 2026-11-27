# Nephron

Local-first Rust GUI workspace for ARM64 SBCs (Orange Pi 6 Plus) and desktops — combining a **3D MindCorpus Glitter Brain**, **Cynapse & Ollama AI backends**, **Perplexica-style Web Search**, **offline Voice + `pocket-tts`**, **custom animated Desktop Pets**, **desktop-wide CUA Swarm Cursors**, and **full-stage Parametric CAD**.

> **Status:** Phases 1–8 implemented & verified (`17/17` workspace tests passing, zero clippy warnings). See [`PLAN.md`](PLAN.md) and [`GATES.md`](GATES.md).

## Features
- **3D MindCorpus Kinetic Brain & Universal Dock (`nephron-app`):** Full-window interactive 3D cortical network (`#030407` obsidian canvas, 1,500 fast-quad glitter stars, gyri/sulci folding, 7-stop prismatic spectrum, cognitive lobe illumination, color-coded memory hubs + keyword-linked synaptic edges, pinned context injection, and `Clear Unpinned`) with a sliding kinetic icon dock and living Bubble Head orb.
- **Dual AI Backends + Full-Page Perplexica Research (`nephron-bridge`):** Toggle between [`cynapse-harness`](https://github.com/Alartist40/cynapse-harness) (`:38265`) and **Ollama** (`:11434`, auto-discovers models), plus **Perplexica-style Web Search** (SearXNG JSON / DuckDuckGo HTML + concurrent full-page article excerpt reader + 2-column source cards with one-click `★` pin to Dendrite).
- **Offline Pipelined Voice + Auto-VAD & STT (`nephron-voice`):** ALSA `arecord`/`aplay` 16 kHz PCM capture, hands-free Auto-VAD turn-taking, Barge-In speech interrupt, local Whisper/Moonshine STT fallback, and portable `kyutai-labs/pocket-tts` Rust sidecar (`fantine` female voice, offline `hf_cache`, sentence-by-sentence double-buffered playback).
- **Floating Desktop Pets (`nephron-pets`):** Independent always-on-top transparent companions (**Cat**, **Chiba Dog**, **Sheep**) synced to the active GUI backend & model, staying alive even when minimized/closed, with voice lip-sync (`talk` frames) and custom animated PNG sprites.
- **Desktop-Wide CUA Swarm (`nephron-swarm`):** Independent always-on-top click-through overlay with spring-interpolated subagent cursors + allowlisted OS-level CUA actions (`screenshot`, `gnome-terminal`, `notify-send`, `xdg-open`, system checks).
- **Full-Stage Pure-Rust + CV CAD (`nephron-cad`):** Interactive 3D STL orbit viewport (backface culling, wireframe toggle, auto-fit camera), pure-Rust primitive + SDF boolean CSG (`op: "subtract"`) builder (`box`, `pyramid`, `cylinder`, `cone`, `tube`, `sphere`), live parametric `W/D/H` sliders, `cv_experimental`-inspired `cv hand` webcam gesture control, auto-repair loop, `Export STL`, and `Slicer` handoff.

## Quick Start

```bash
./run.sh              # Portable launcher using local vendor/pocket-tts + offline hf_cache
./install.sh          # Release build (jobs=2 for 0-swap SBC safety), installs ~/.local/bin/nephron, runs doctor
nephron               # Launch GUI
nephron doctor        # Check RAM, Cynapse/Ollama, audio, pocket-tts, hf_cache, openscad
```
