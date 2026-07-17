# VigilCut — compile + test backend + frontend check
$ErrorActionPreference = "Stop"
$Root = Split-Path $PSScriptRoot -Parent
Set-Location $Root

$cargoBin = Join-Path $env:USERPROFILE ".cargo\bin"
$machine = [Environment]::GetEnvironmentVariable("Path", "Machine")
$user = [Environment]::GetEnvironmentVariable("Path", "User")
$env:Path = "$cargoBin;$user;$machine"

$vsPath = & "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe" `
    -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 `
    -property installationPath

$vcvars = Join-Path $vsPath "VC\Auxiliary\Build\vcvars64.bat"

Write-Host "==> frontend check" -ForegroundColor Cyan
npm run check
npm run build

Write-Host "==> rust check + test" -ForegroundColor Cyan
cmd /c "`"$vcvars`" && set PATH=$cargoBin;%PATH% && cd /d `"$Root\src-tauri`" && cargo check && cargo test"

Write-Host "`nAll checks passed." -ForegroundColor Green
