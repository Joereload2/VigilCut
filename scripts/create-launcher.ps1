# Create VigilCut desktop launcher with branded icon.
# Prefers the newest built exe (release if present, else debug).
# Usage: powershell -ExecutionPolicy Bypass -File scripts/create-launcher.ps1

$ErrorActionPreference = "Stop"
$Root = Split-Path $PSScriptRoot -Parent
$release = Join-Path $Root "src-tauri\target\release\vigilcut.exe"
$debug = Join-Path $Root "src-tauri\target\debug\vigilcut.exe"
$ico = Join-Path $Root "branding\VigilCut.ico"
if (-not (Test-Path $ico)) {
    $ico = Join-Path $Root "src-tauri\icons\icon.ico"
}

$exe = $null
if (Test-Path $release) { $exe = $release }
elseif (Test-Path $debug) { $exe = $debug }
else {
    throw "Cannot find vigilcut.exe. Build first: npm run build ; cargo build --manifest-path src-tauri/Cargo.toml --release"
}

if (-not (Test-Path $ico)) {
    throw "Cannot find icon: $ico"
}

$desktop = [Environment]::GetFolderPath("Desktop")
$lnkDesktop = Join-Path $desktop "VigilCut.lnk"
$lnkProject = Join-Path $Root "VigilCut.lnk"

$w = New-Object -ComObject WScript.Shell
foreach ($path in @($lnkDesktop, $lnkProject)) {
    $s = $w.CreateShortcut($path)
    $s.TargetPath = $exe
    $s.WorkingDirectory = Split-Path $exe
    $s.IconLocation = "$ico,0"
    $s.Description = "VigilCut Factory - AI content engine"
    $s.WindowStyle = 1
    $s.Save()
    Write-Host "OK $path -> $exe"
}

$bat = Join-Path $Root "Abrir-VigilCut.bat"
$batLines = @(
    "@echo off",
    "set `"ROOT=%~dp0`"",
    "cd /d `"%ROOT%`"",
    "taskkill /IM vigilcut.exe /F >nul 2>&1",
    "timeout /t 1 /nobreak >nul",
    "if exist `"%ROOT%src-tauri\target\release\vigilcut.exe`" (",
    "  start `"`" `"%ROOT%src-tauri\target\release\vigilcut.exe`"",
    ") else if exist `"%ROOT%src-tauri\target\debug\vigilcut.exe`" (",
    "  start `"`" `"%ROOT%src-tauri\target\debug\vigilcut.exe`"",
    ") else (",
    "  echo No exe. Run: npm run dev:win",
    "  pause",
    ")"
)
Set-Content -Path $bat -Value $batLines -Encoding ASCII
Write-Host "OK $bat"
Write-Host "Launcher ready. Binary: $exe ($(Get-Item $exe).LastWriteTime)"
