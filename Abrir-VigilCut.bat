@echo off
REM Launch latest VigilCut. Prefer debug (dev builds with newest features),
REM then release. Kills stale instances so you don't keep an old v1.0.0 UI.

set "ROOT=%~dp0"
cd /d "%ROOT%"

taskkill /IM vigilcut.exe /F >nul 2>&1

set "DEBUG_EXE=%ROOT%src-tauri\target\debug\vigilcut.exe"
set "RELEASE_EXE=%ROOT%src-tauri\target\release\vigilcut.exe"

if exist "%DEBUG_EXE%" (
  echo Opening debug build (newest features)...
  start "" "%DEBUG_EXE%"
  exit /b 0
)

if exist "%RELEASE_EXE%" (
  echo Opening release build...
  start "" "%RELEASE_EXE%"
  exit /b 0
)

echo No built exe found. Run: npm run dev:win
echo Or: npm run tauri:build
pause
