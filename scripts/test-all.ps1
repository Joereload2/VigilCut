# VigilCut test pyramid: unit -> smoke -> e2e
$ErrorActionPreference = "Stop"
$Root = Split-Path $PSScriptRoot -Parent
Set-Location $Root

$cargoBin = Join-Path $env:USERPROFILE ".cargo\bin"
$env:Path = "$cargoBin;$env:Path"

$ff = Join-Path $Root "src-tauri\binaries\ffmpeg.exe"
if (-not (Test-Path $ff)) {
    Write-Host "==> ffmpeg sidecar missing - running setup:ffmpeg"
    npm run setup:ffmpeg
}

Write-Host ""
Write-Host "==> UNIT (cargo test --lib)"
Push-Location (Join-Path $Root "src-tauri")
cargo test --lib
if ($LASTEXITCODE -ne 0) { Pop-Location; exit $LASTEXITCODE }

Write-Host ""
Write-Host "==> SMOKE (pipeline + synthetic media)"
cargo test --test smoke_pipeline -- --nocapture
if ($LASTEXITCODE -ne 0) { Pop-Location; exit $LASTEXITCODE }

Write-Host ""
Write-Host "==> E2E factory (export + artifacts + batch)"
cargo test --test e2e_factory -- --nocapture
if ($LASTEXITCODE -ne 0) { Pop-Location; exit $LASTEXITCODE }
Pop-Location

Write-Host ""
Write-Host "==> ALL LAYERS PASSED"
exit 0
