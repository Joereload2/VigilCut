# VigilCut production build (Windows)
# Requires: Node 20+, Rust (rustup), FFmpeg sidecars

$ErrorActionPreference = "Stop"
Set-Location (Split-Path $PSScriptRoot -Parent)

Write-Host "==> VigilCut build" -ForegroundColor Cyan

if (-not (Get-Command npm -ErrorAction SilentlyContinue)) {
  throw "npm not found. Install Node.js 20+."
}

if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
  throw "cargo not found. Install Rust: https://rustup.rs"
}

Write-Host "==> npm install" -ForegroundColor Cyan
npm install

Write-Host "==> setup FFmpeg sidecars" -ForegroundColor Cyan
npm run setup:ffmpeg

Write-Host "==> tauri build" -ForegroundColor Cyan
npm run tauri:build

Write-Host "`nBuild complete. Artifacts under src-tauri/target/release/bundle/" -ForegroundColor Green
