' Silent launcher — no console window.
' Prefers release (standalone UI), then debug.

Option Explicit
Dim fso, sh, root, releaseExe, debugExe, exe, wsh
Set fso = CreateObject("Scripting.FileSystemObject")
Set sh = CreateObject("WScript.Shell")
root = fso.GetParentFolderName(WScript.ScriptFullName)
releaseExe = root & "\src-tauri\target\release\vigilcut.exe"
debugExe = root & "\src-tauri\target\debug\vigilcut.exe"

On Error Resume Next
sh.Run "taskkill /IM vigilcut.exe /F", 0, True
On Error GoTo 0

If fso.FileExists(releaseExe) Then
  exe = releaseExe
ElseIf fso.FileExists(debugExe) Then
  exe = debugExe
Else
  MsgBox "No hay ejecutable." & vbCrLf & "Ejecuta: npm run tauri:build" & vbCrLf & "o: npm run dev:win", 48, "VigilCut"
  WScript.Quit 1
End If

' 0 = hidden window for the process start helper; GUI app still shows.
sh.CurrentDirectory = fso.GetParentFolderName(exe)
sh.Run """" & exe & """", 1, False
