$exe = "C:\Users\jose\consul\Documentos\VigilCut\src-tauri\target\release\vigilcut.exe"
$wd = Split-Path $exe
Get-Process -Name vigilcut -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
Start-Sleep -Milliseconds 500
Start-Process -FilePath $exe -WorkingDirectory $wd
