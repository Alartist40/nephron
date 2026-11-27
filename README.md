# Nephron

Local-first Rust GUI workspace for ARM64 SBCs (Orange Pi 6 Plus) and desktops — combining a **3D MindCorpus Glitter Brain**, **Cynapse & Ollama AI backends**, **Perplexica-style Web Search**, **offline Voice + `pocket-tts`**, **custom animated Desktop Pets**, **desktop-wide CUA Swarm Cursors**, and **full-stage Parametric CAD**.

> **Status:** Phases 1–7 implemented & verified (`16/16` workspace tests passing, zero clippy warnings). See [`PLAN.md`](PLAN.md) and [`GATES.md`](GATES.md).

## Features
- **3D MindCorpus Brain & Universal Dock (`nephron-app`):** Full-window interactive 3D cortical network (`#030407` obsidian canvas, 900 fast-quad glitter stars, 90 curved Bezier synaptic pathways, 60 FPS drag/orbit, click-to-cascade, clickable Dendrite SQLite memory nodes with pinning & `Clear Unpinned`) plus a universal chatbox that auto-routes chat, web research, and CAD from one prompt bar.
- **Dual AI Backends + Perplexica Web Search (`nephron-bridge`):** Switch between [`cynapse-harness`](https://github.com/Alartist40/cynapse-harness) (`:38265`) and **Ollama** (`:11434`, auto-discovers installed models and omits unsupported `reasoning_effort`), plus **Perplexica-style Web Search** (SearXNG JSON or DuckDuckGo HTML fallback with `[1]` source cards & URL scheme validation) and SQLite + FTS5 Dendrite memory with pinning.
- **Offline Pipelined Voice (`nephron-voice`):** ALSA `arecord`/`aplay` 16 kHz PCM capture + RMS VAD (`Ctrl+M`) + local portable `kyutai-labs/pocket-tts` Rust sidecar (`fantine` gentle female voice, offline `hf_cache`, sentence-by-sentence double-buffered playback pipeline adapted from `Alartist40/paraclea`).
- **Floating Desktop Pets (`nephron-pets`):** Independent always-on-top transparent companions (**Cat**, **Chiba Dog**, **Sheep**) synced to the active GUI backend & model, with 59 custom animated PNG frames (`idle`, `think`, `talk`, `scan`), drag-anywhere positioning, click-to-chat speech bubbles, and automatic `scan` reactions.
- **Desktop-Wide CUA Swarm (`nephron-swarm`):** Transparent click-through overlay + spring-interpolated subagent cursors executing safe OS-level actions (`gnome-screenshot`, `xdg-open`, `notify-send`, allowlisted checks).
- **Full-Stage Pure-Rust + OpenSCAD CAD (`nephron-cad`):** Interactive 3D STL orbit viewport (backface culling, wireframe toggle, auto-fit camera, live `W×D×H` mm readout), pure-Rust primitive/assembly builder (`box`, `cylinder`, `cone`, `tube`, `sphere`) with auto-repair loop, OpenSCAD fallback, one-click `Export STL` (`~/Exports/`), and `Slicer` handoff.

## Quick Start

```bash
./run.sh              # Portable launcher using local vendor/pocket-tts + offline hf_cache
./install.sh          # Release build (jobs=2 for 0-swap SBC safety), installs ~/.local/bin/nephron, runs doctor
nephron               # Launch GUI
nephron doctor        # Check RAM, Cynapse/Ollama, audio, pocket-tts, hf_cache, openscad
```
