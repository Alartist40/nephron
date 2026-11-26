#!/usr/bin/env bash
set -euo pipefail

echo "Building nephron-app (release, jobs=2 for 0-swap SBC safety)..."
cargo build --release -p nephron-app

mkdir -p "$HOME/.local/bin"
install -m 755 target/release/nephron-app "$HOME/.local/bin/nephron"
echo "Installed: $HOME/.local/bin/nephron"

"$HOME/.local/bin/nephron" doctor
