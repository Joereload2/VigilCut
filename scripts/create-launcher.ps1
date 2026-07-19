# Create VigilCut desktop launcher with branded icon.
# Usage: powershell -ExecutionPolicy Bypass -File scripts/create-launcher.ps1

$ErrorActionPreference = "Stop"
$Root = Split-Path $PSScriptRoot -Parent
$exe = Join-Path $Root "src-tauri\target\release\vigilcut.exe"
$ico = Join-Path $Root "branding\VigilCut.ico"
if (-not (Test-Path $ico)) {
    $ico = Join-Path $Root "src-tauri\icons\icon.ico"
}
if (-not (Test-Path $exe)) {
    throw "Cannot find release exe: $exe  (build first: npx tauri build --no-bundle)"
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
    Write-Host "OK $path"
}

$bat = Join-Path $Root "Abrir-VigilCut.bat"
$batLines = @(
    "@echo off",
    "start `"`" `"$exe`""
)
Set-Content -Path $bat -Value $batLines -Encoding ASCII
Write-Host "OK $bat"
Write-Host "Launcher ready on Desktop: VigilCut.lnk"
Write-Host "Icon: $ico"
