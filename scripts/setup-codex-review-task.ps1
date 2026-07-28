param([switch]$Remove)

$ErrorActionPreference = 'Stop'
$taskName = 'VigilCut-Codex-Grok-Review'
$runner = Join-Path $PSScriptRoot 'run-codex-review-final.ps1'
$repo = Split-Path -Parent $PSScriptRoot

if ($Remove) {
    Unregister-ScheduledTask -TaskName $taskName -Confirm:$false -ErrorAction SilentlyContinue
    Write-Host "Tarea eliminada: $taskName"
    exit 0
}

if (-not (Test-Path -LiteralPath $runner)) {
    throw "No existe el ejecutor: $runner"
}

$action = New-ScheduledTaskAction `
    -Execute 'powershell.exe' `
    -Argument "-NoProfile -NonInteractive -ExecutionPolicy Bypass -File `"$runner`"" `
    -WorkingDirectory $repo

$triggers = @(
    New-ScheduledTaskTrigger -Daily -At '00:00'
    New-ScheduledTaskTrigger -Daily -At '04:00'
    New-ScheduledTaskTrigger -Daily -At '08:00'
    New-ScheduledTaskTrigger -Daily -At '12:00'
    New-ScheduledTaskTrigger -Daily -At '16:00'
    New-ScheduledTaskTrigger -Daily -At '20:00'
)

$principal = New-ScheduledTaskPrincipal `
    -UserId "$env:USERDOMAIN\$env:USERNAME" `
    -LogonType Interactive `
    -RunLevel Limited

$settings = New-ScheduledTaskSettingsSet `
    -StartWhenAvailable `
    -MultipleInstances IgnoreNew `
    -ExecutionTimeLimit (New-TimeSpan -Hours 3)

Register-ScheduledTask `
    -TaskName $taskName `
    -Action $action `
    -Trigger $triggers `
    -Principal $principal `
    -Settings $settings `
    -Description 'Revision acoplada Codex-Grok de VigilCut; requiere ambos estados ACTIVO.' `
    -Force | Out-Null

Write-Host "Tarea instalada: $taskName"
Get-ScheduledTaskInfo -TaskName $taskName |
    Select-Object LastRunTime, LastTaskResult, NextRunTime
