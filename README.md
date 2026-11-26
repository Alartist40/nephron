# Nephron

Local-first Rust GUI workspace for ARM64 SBCs (Orange Pi 6 Plus) and desktops — combining a **3D MindCorpus Glitter Brain**, **Cynapse & Ollama AI backends**, **Perplexica-style Web Search**, **offline Voice + `pocket-tts`**, **custom animated Desktop Pets**, **desktop-wide CUA Swarm Cursors**, and **full-stage Parametric CAD**.

> **Status:** Phases 1–6 implemented & verified (`14/14` workspace tests passing, zero warnings). See [`PLAN.md`](PLAN.md) and [`GATES.md`](GATES.md).

## Features
- **3D MindCorpus Brain & OS Dock (`nephron-app`):** Full-window interactive 3D cortical network (`#030407` obsidian canvas, 2,000 glitter stars, 90 curved Bezier synaptic pathways, mouse-hover glow, click-to-cascade, live Dendrite SQLite memory nodes) with a minimalist bottom OS icon dock.
- **Dual AI Backends + Web Search (`nephron-bridge`):** Switch between [`cynapse-harness`](https://github.com/Alartist40/cynapse-harness) (`:38265`) and **Ollama** (`:11434`, auto-discovers installed models), plus **Perplexica-style Web Search** (SearXNG JSON or DuckDuckGo HTML fallback with `[1]` citations) and SQLite + FTS5 Dendrite memory.
- **Offline Voice (`nephron-voice`):** ALSA `arecord`/`aplay` 16 kHz PCM capture + RMS VAD + `LFM3-Omni-2B` audio prompts + `pocket-tts` CPU playback (`Ctrl+M`).
- **Floating Desktop Pets (`nephron-pets`):** Independent always-on-top transparent companions (**Cat**, **Chiba Dog**, **Sheep**) with 59 custom animated PNG frames (`idle`, `think`, `talk`, `scan`), drag-anywhere positioning, click-to-chat speech bubbles, and automatic `scan` reactions during agentic tasks.
- **Desktop-Wide CUA Swarm (`nephron-swarm`):** Transparent click-through overlay + spring-interpolated subagent cursors executing safe OS-level actions (`gnome-screenshot`, `xdg-open`, `notify-send`, allowlisted checks).
- **Full-Stage Parametric CAD (`nephron-cad`):** Replaces the Brain full-screen with an interactive 3D STL orbit viewport, OpenSCAD `.scad` editor/compiler, AI generation, and Dendrite BOM tracking.

## Quick Start

```bash
./install.sh          # Release build (jobs=2 for 0-swap SBC safety), installs ~/.local/bin/nephron, runs doctor
nephron               # Launch GUI
nephron doctor        # Check RAM, Cynapse/Ollama, audio, pocket-tts, openscad
```
