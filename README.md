# Nephron

Local-first Rust GUI workspace for ARM64 SBCs (Orange Pi 6 Plus) and desktops — combining **voice & text AI**, **original sprite-sheet desktop pets**, **multi-agent swarm cursors**, and **offline parametric CAD** on top of [`cynapse-harness`](https://github.com/Alartist40/cynapse-harness).

> **Status:** Planning & architecture phase. See [`PLAN.md`](PLAN.md) for the complete system design, referenced repositories, and step-by-step build roadmap.

## Core Pillars
- **Dual Input (Voice or Text):** Talk locally via `LFM3-Omni-2B` / `LFM3-Audio-2B` + [`pocket-tts`](https://github.com/kyutai-labs/pocket-tts) (CPU, MIT) or type directly to `MiniCPM5-1B`, `Glia-1.7B-v1.5`, and `Ornith-9B`.
- **Desktop Pets (`nephron-pets`):** Original click-reactive sprite-sheet companions (Dog, Cat, Capybara) with speech bubbles and voice responses.
- **Swarm Cursors (`nephron-swarm`):** Live multi-cursor canvas visualization (inspired by [`swarm-cursors`](https://github.com/DhvanilPatel/swarm-cursors)) showing concurrent subagents reading, editing, and building in real time.
- **Offline CAD Builder (`nephron-cad`):** Local voice/text-to-parametric CAD (OpenSCAD / `truck`) with `.stl` 3D preview and parts/BOM tracking in Dendrite memory (inspired by [`ada-jarvis-public`](https://github.com/nazirlouis/ada-jarvis-public), rewritten in pure Rust).
