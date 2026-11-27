#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$ROOT_DIR"

echo "Building nephron-app (release, jobs=2 for 0-swap SBC safety)..."
cargo build --release -j 2 -p nephron-app

if [ -d "$ROOT_DIR/vendor/pocket-tts/rust" ]; then
    echo "Building local pocket-tts (release)..."
    (cd "$ROOT_DIR/vendor/pocket-tts/rust" && cargo build --release -j 2)
fi

mkdir -p "$HOME/.local/bin"
install -m 755 target/release/nephron-app "$HOME/.local/bin/nephron"
if [ -f "$ROOT_DIR/vendor/pocket-tts/rust/target/release/pocket-tts" ]; then
    install -m 755 "$ROOT_DIR/vendor/pocket-tts/rust/target/release/pocket-tts" "$HOME/.local/bin/pocket-tts"
fi
echo "Installed: $HOME/.local/bin/nephron"

"$HOME/.local/bin/nephron" doctor
