#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$ROOT_DIR"

export NEPHRON_POCKET_TTS_BIN="$ROOT_DIR/vendor/pocket-tts/rust/target/release/pocket-tts"
if [ -d "$ROOT_DIR/vendor/pocket-tts/hf_cache" ]; then
    export HF_HOME="$ROOT_DIR/vendor/pocket-tts/hf_cache"
    export HF_HUB_OFFLINE=1
fi

if [ ! -x "$ROOT_DIR/target/release/nephron-app" ]; then
    cargo build --release -j 2 -p nephron-app
fi

exec "$ROOT_DIR/target/release/nephron-app" "$@"
