' Silent launcher — production only (no localhost / Vite).
' Always opens release build with embedded UI.

Option Explicit
Dim fso, sh, root, releaseExe
Set fso = CreateObject("Scripting.FileSystemObject")
Set sh = CreateObject("WScript.Shell")
root = fso.GetParentFolderName(WScript.ScriptFullName)
releaseExe = root & "\src-tauri\target\release\vigilcut.exe"

On Error Resume Next
sh.Run "taskkill /IM vigilcut.exe /F", 0, True
On Error GoTo 0

If Not fso.FileExists(releaseExe) Then
  MsgBox "No hay build de producción." & vbCrLf & vbCrLf & _
    "Ejecuta en PowerShell:" & vbCrLf & _
    "  cd " & root & vbCrLf & _
    "  npx tauri build --no-bundle" & vbCrLf & vbCrLf & _
    "NO uses target\debug (requiere Vite en localhost:1420).", 48, "VigilCut"
  WScript.Quit 1
End If

sh.CurrentDirectory = fso.GetParentFolderName(releaseExe)
sh.Run """" & releaseExe & """", 1, False
