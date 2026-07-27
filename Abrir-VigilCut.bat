@echo off
REM Always launch production release (embedded UI — no Vite/localhost).
set "ROOT=%~dp0"
cd /d "%ROOT%"
if exist "%ROOT%Abrir-VigilCut.vbs" (
  wscript.exe //nologo "%ROOT%Abrir-VigilCut.vbs"
  exit /b 0
)
taskkill /IM vigilcut.exe /F >nul 2>&1
if exist "%ROOT%src-tauri\target\release\vigilcut.exe" (
  start "" "%ROOT%src-tauri\target\release\vigilcut.exe"
  exit /b 0
)
echo No hay release. Ejecuta: npx tauri build --no-bundle
echo No uses target\debug sin Vite.
pause
