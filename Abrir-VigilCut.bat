@echo off
set "ROOT=%~dp0"
cd /d "%ROOT%"
echo Cerrando VigilCut antiguos...
taskkill /IM vigilcut.exe /F >nul 2>&1
timeout /t 1 /nobreak >nul
set "EXE=%ROOT%src-tauri\target\debug\vigilcut.exe"
if not exist "%EXE%" set "EXE=%ROOT%src-tauri\target\release\vigilcut.exe"
if not exist "%EXE%" (
  echo No hay ejecutable. Ejecuta: npm run dev:win
  pause
  exit /b 1
)
echo Abriendo: %EXE%
start "" "%EXE%"
