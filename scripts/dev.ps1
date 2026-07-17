# VigilCut — one-shot dev launcher (Windows)
# Sets Cargo + VS Build Tools + FFmpeg PATH, then runs tauri dev.

$ErrorActionPreference = "Stop"
$Root = Split-Path $PSScriptRoot -Parent
Set-Location $Root

$cargoBin = Join-Path $env:USERPROFILE ".cargo\bin"
$machine = [Environment]::GetEnvironmentVariable("Path", "Machine")
$user = [Environment]::GetEnvironmentVariable("Path", "User")
$env:Path = "$cargoBin;$user;$machine"

$vswhere = "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe"
if (-not (Test-Path $vswhere)) {
    Write-Error "VS Build Tools not found. Install: winget install Microsoft.VisualStudio.2022.BuildTools"
}

$vsPath = & $vswhere -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
if (-not $vsPath) {
    Write-Error "MSVC C++ tools missing from Build Tools install."
}

$vcvars = Join-Path $vsPath "VC\Auxiliary\Build\vcvars64.bat"
Write-Host "==> VigilCut dev (vcvars + tauri)" -ForegroundColor Cyan
Write-Host "    VS: $vsPath"
Write-Host "    Project: $Root"

if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    Write-Error "cargo not found. Install Rust: https://rustup.rs"
}

if (-not (Get-Command ffmpeg -ErrorAction SilentlyContinue)) {
    Write-Warning "ffmpeg not on PATH — run: npm run setup:ffmpeg"
}

# Run tauri under the VS developer environment so link.exe is available
cmd /c "`"$vcvars`" && set PATH=$cargoBin;%PATH% && cd /d `"$Root`" && npm run tauri:dev"
