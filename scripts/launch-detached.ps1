$exe = "C:\Users\jose\consul\Documentos\VigilCut\src-tauri\target\release\vigilcut.exe"
$wd = Split-Path $exe
Start-Process -FilePath $exe -WorkingDirectory $wd
