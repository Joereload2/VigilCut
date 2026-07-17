#!/usr/bin/env bash
# VigilCut production build (macOS / Linux)
set -euo pipefail
cd "$(dirname "$0")/.."

echo "==> VigilCut build"

command -v npm >/dev/null || { echo "npm not found"; exit 1; }
command -v cargo >/dev/null || { echo "cargo not found — install via https://rustup.rs"; exit 1; }

echo "==> npm install"
npm install

echo "==> setup FFmpeg sidecars"
npm run setup:ffmpeg || true

echo "==> tauri build"
npm run tauri:build

echo ""
echo "Build complete. Artifacts under src-tauri/target/release/bundle/"
