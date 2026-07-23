# Create VigilCut desktop launcher with branded icon (no console).
# Prefers release, then debug. Uses VBS silent launch when available.
# Usage: powershell -ExecutionPolicy Bypass -File scripts/create-launcher.ps1

$ErrorActionPreference = "Stop"
$Root = Split-Path $PSScriptRoot -Parent
$release = Join-Path $Root "src-tauri\target\release\vigilcut.exe"
$debug = Join-Path $Root "src-tauri\target\debug\vigilcut.exe"
$vbs = Join-Path $Root "Abrir-VigilCut.vbs"
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

# Prefer VBS as shortcut target so double-click never flashes a console
$target = if (Test-Path $vbs) { $vbs } else { $exe }
$workDir = if (Test-Path $vbs) { $Root } else { Split-Path $exe }

$desktop = [Environment]::GetFolderPath("Desktop")
$lnkDesktop = Join-Path $desktop "VigilCut.lnk"
$lnkProject = Join-Path $Root "VigilCut.lnk"

$w = New-Object -ComObject WScript.Shell
foreach ($path in @($lnkDesktop, $lnkProject)) {
    $s = $w.CreateShortcut($path)
    $s.TargetPath = $target
    $s.WorkingDirectory = $workDir
    $s.IconLocation = "$ico,0"
    $s.Description = "VigilCut Factory (sin terminal)"
    $s.WindowStyle = 1
    $s.Save()
    Write-Host "OK $path -> $target"
}

Write-Host "Launcher ready. Binary: $exe ($(Get-Item $exe).LastWriteTime)"
