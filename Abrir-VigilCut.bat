@echo off
REM Silent launch via VBS (no console). Falls back to start if VBS missing.
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
if exist "%ROOT%src-tauri\target\debug\vigilcut.exe" (
  start "" "%ROOT%src-tauri\target\debug\vigilcut.exe"
  exit /b 0
)
echo No hay ejecutable. npm run tauri:build
pause
