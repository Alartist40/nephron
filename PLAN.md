# Nephron — Architecture & Step-by-Step Build Plan

**Target Hardware:** Orange Pi 6 Plus (`aarch64`, ARMv9.2-A, 14 GiB LPDDR5, 0 B swap) & Desktop Linux/macOS/Windows  
**Core Foundation:** [`Alartist40/cynapse-harness`](https://github.com/Alartist40/cynapse-harness) (`cynapse-engine`, `cynapse-memory` / Dendrite, `llama-server` daemon on `:38265`)  
**Language:** Pure Rust (`winit` + `eframe`/`egui` + `wgpu`) — zero Electron, zero Chromium kiosk, zero cloud API keys required.

---

## 1. Vision & User Experience

**Nephron** is a local-first, visual GUI workspace built on top of Cynapse's inference and memory engine. A user can:
1. **Choose Voice or Text seamlessly:** Press push-to-talk (or wake/toggle mic) for hands-free voice commands, or type directly in the prompt bar.
2. **Toggle Modular Tools on Demand:**
   - **Desktop Pets:** Original sprite-sheet companions (**Dog**, **Cat**, **Capybara**) that idle, walk, react to clicks, display speech bubbles, and speak responses aloud.
   - **Swarm Cursors:** Multi-colored labeled cursors animating live across the workspace/canvas whenever Cynapse dispatches parallel subagents.
   - **Offline CAD Studio:** Describe a part or enclosure by voice or text, generate parametric CAD locally, inspect the 3D mesh, and track parts/BOM in Dendrite SQLite memory.

---

## 2. Referenced Repositories & Model Stack

| Component | Source / Repo | Role in Nephron | License / Notes |
| :--- | :--- | :--- | :--- |
| **AI & Memory Ground** | [`Alartist40/cynapse-harness`](https://github.com/Alartist40/cynapse-harness) | `cynapse-engine` (`:38265`), Dendrite SQLite+FTS5 memory, `/sdd`, `/effort`, `/soup` | Clean-room Rust foundation |
| **SBC Voice Workspace** | [`nazirlouis/ada-jarvis-public`](https://github.com/nazirlouis/ada-jarvis-public) | Conceptual inspiration for voice-driven briefs, parts inventory, & CAD workflow | Rewritten cleanly in Rust (no Python/Chromium/Gemini cloud) |
| **CPU Text-to-Speech** | [`kyutai-labs/pocket-tts`](https://github.com/kyutai-labs/pocket-tts) | ~100M-param CPU TTS (~200ms first-audio, zero-shot voice cloning) | MIT License |
| **Swarm Visualization** | [`DhvanilPatel/swarm-cursors`](https://github.com/DhvanilPatel/swarm-cursors) | Inspiration for multi-cursor live build animations | Adapted for Rust subagent canvas |
| **Voice / Audio Model** | `LiquidAI/LFM3-Omni-2B-GGUF` / `LFM3-Audio-2B-GGUF` | Offline speech understanding & multimodal reasoning | Runs via `cynapse-engine` |
| **Fast Tool / Chat Core** | `openbmb/MiniCPM5-1B-GGUF` / `Glia-1.7B-v1.5` | Instant low-RAM replies, tool routing, & companion chat (`/effort none`) | Apache 2.0 / local GGUF |
| **Complex Code / CAD** | `Ornith-1.5-9B-Q4_K_M.gguf` | Multi-step parametric CAD & code generation | Runs on `Tier 1 Fast` (`llama-server`) |

---

## 3. Workspace Crate Architecture

Keep `cynapse-harness` lean and terminal-first; build Nephron as a modular Rust workspace that links `cynapse-engine` + `cynapse-memory` (or connects to `http://127.0.0.1:38265`):

```text
nephron/
├── Cargo.toml                  # Workspace manifest (jobs = 2 in .cargo/config.toml for SBC safety)
├── crates/
│   ├── nephron-app/            # Main GUI shell (eframe / egui + wgpu), mode toggles (Voice/Text/Tools)
│   ├── nephron-bridge/         # HTTP/SSE + Rust bridge to cynapse-engine (:38265) & Dendrite memory
│   ├── nephron-voice/          # Mic capture (cpal) + VAD + LFM3-Omni input + pocket-tts playback
│   ├── nephron-pets/           # Original sprite-sheet engine (Dog, Cat, Capybara) + click/talk states
│   ├── nephron-swarm/          # Multi-cursor spring physics & subagent event-stream visualizer
│   └── nephron-cad/            # Parametric OpenSCAD / truck B-rep generator + 3D STL preview + BOM
└── assets/
    ├── sprites/                # Original Dog, Cat, Capybara sprite sheets (PNG atlases + JSON frames)
    └── voices/                 # Reference .wav clips for pocket-tts zero-shot companion voices
```

---

## 4. Step-by-Step Build Guide

### Phase 1 — Core GUI Shell & Cynapse Bridge
1. Initialize the Rust workspace (`nephron-app` + `nephron-bridge`) with `.cargo/config.toml` (`jobs = 2`) for safe compilation on 14 GiB ARM64 SBCs.
2. Build the `eframe` / `egui` main window with:
   - Top bar: active model selector (`MiniCPM5-1B`, `Glia-1.7B-v1.5`, `Ornith-9B`, `LFM3-Omni-2B`), live RAM/tok-s telemetry, and `/effort` selector (`none|low|med|high|auto`).
   - Tool toggles: `[Voice]`, `[Pet]`, `[Swarm]`, `[CAD]`.
   - Center workspace + streaming chat pane connected to `cynapse-engine` (`http://127.0.0.1:38265`).
3. Wire `cynapse-memory` (Dendrite SQLite + FTS5) so project briefs, CAD parts, and companion conversations persist locally.

### Phase 2 — Offline Voice Loop (`LFM3-Omni` + `pocket-tts`)
1. Integrate `cpal` microphone capture + lightweight energy/Silero VAD in `nephron-voice`.
2. Route captured audio chunks to `LFM3-Omni-2B` / `LFM3-Audio-2B` via `cynapse-engine` for offline speech understanding.
3. Run `kyutai-labs/pocket-tts` as a local CPU TTS worker (sidecar over localhost/pipe initially, or `ort`/`candle` Rust bindings later) streaming `.wav`/PCM audio back through `rodio` / `cpal` (~200ms latency).
4. Add push-to-talk hotkey + GUI mic toggle so users can switch between voice and typing at any moment.

### Phase 3 — Original Desktop Pets (`nephron-pets`)
1. Create original sprite-sheet atlases for **Dog**, **Cat**, and **Capybara** (`idle`, `walk`, `listen`, `think`, `talk`, `happy/petted`).
2. Implement an optional transparent always-on-top overlay window (`winit` transparent + click-through outside sprite bounds) or docked companion corner inside `nephron-app`.
3. Wire click/hover events to pet reactions and route user messages/voice replies through a speech bubble + `pocket-tts` voice profile per animal.
4. Keep prompts fast and brief by defaulting pet chat to `/effort none` on `MiniCPM5-1B` or `Glia-1.7B-v1.5`.

### Phase 4 — Swarm Cursors Visualizer (`nephron-swarm`)
1. Subscribe to `cynapse` subagent / tool-call events (`read`, `edit`, `search`, `cad_build`, `test`).
2. Render smooth spring-interpolated labeled cursors (one color per subagent role, e.g., `Researcher`, `Builder`, `Reviewer`) on the workspace canvas / CAD preview.
3. Animate cursor actions (selection box, pen/node trace, typing indicator) as subagents execute in parallel, giving instant visual feedback for both daily work and demos.

### Phase 5 — Offline Parametric CAD Builder (`nephron-cad`)
1. **Generator:** Prompt `Ornith-9B` (or `Glia-1.7B-v1.5`) with a structured CAD system prompt to emit parametric **OpenSCAD (`.scad`)** code (or pure-Rust **`truck`** B-rep scripts) + a JSON Bill of Materials (BOM).
2. **Local Compiler:** Run `openscad -o out.stl part.scad` (or `truck` mesh tessellation) completely offline on ARM64 Linux.
3. **3D Viewport & Inventory:** Render the `.stl` mesh in an interactive `egui` + `wgpu`/`three-d` orbit viewport alongside an editable parts/BOM table saved to Dendrite SQLite.

### Phase 6 — Packaging & Orange Pi 6 Plus Tuning
1. Add `install.sh` + `doctor` check verifying `cynapse` daemon reachability, `openscad` (if enabled), `pocket-tts`, and audio devices.
2. Enforce `cynapse`'s RAM headroom guard so switching between `MiniCPM5-1B`, `LFM3-Omni-2B`, and `Ornith-9B` cleanly offloads the prior model on 0-swap SBCs.
