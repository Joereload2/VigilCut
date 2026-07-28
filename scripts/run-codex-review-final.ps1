param([switch]$DryRun)

$ErrorActionPreference = 'Stop'
$repo = Split-Path -Parent $PSScriptRoot
$handoffFile = Join-Path $repo 'docs\reviews\CODEX_TO_GROK.md'
$promptFile = Join-Path $PSScriptRoot 'codex-review-prompt.md'
$codexCmd = Join-Path $env:APPDATA 'npm\codex.cmd'
$runtime = Join-Path $env:LOCALAPPDATA 'VigilCut\agent-runtime'
$logs = Join-Path $runtime 'logs'
New-Item -ItemType Directory -Path $logs -Force | Out-Null

$lock = $null
try {
    $lock = [IO.File]::Open((Join-Path $runtime 'codex-review.lock'), 'OpenOrCreate', 'ReadWrite', 'None')
} catch { exit 0 }

try {
    $text = Get-Content -LiteralPath $handoffFile -Raw
    $grok = [regex]::Matches($text, '(?m)^\*\*Estado del scheduler de Grok:\s*(ACTIVO|DETENIDO)\*\*\s*$')
    $codex = [regex]::Matches($text, '(?m)^\*\*Estado del scheduler de Codex:\s*(ACTIVO|DETENIDO)\*\*\s*$')
    if ($grok.Count -ne 1 -or $codex.Count -lt 1) {
        throw "Campos invalidos: Grok=$($grok.Count), Codex=$($codex.Count)"
    }
    $grokState = $grok[0].Groups[1].Value
    $codexStates = @($codex | ForEach-Object { $_.Groups[1].Value } | Sort-Object -Unique)
    if ($codexStates.Count -ne 1 -or $codexStates[0] -ne $grokState) {
        throw "Estados desacoplados: Grok=$grokState, Codex=$($codexStates -join ',')"
    }
    if ($grokState -eq 'DETENIDO') { exit 0 }
    if (-not (Test-Path $codexCmd) -or -not (Test-Path $promptFile)) {
        throw 'Falta Codex o el prompt programado'
    }

    $stamp = Get-Date -Format 'yyyyMMdd-HHmmss'
    $log = Join-Path $logs "codex-review-$stamp.log"
    $final = Join-Path $logs "codex-review-$stamp-final.txt"
    if ($DryRun) {
        "DRY RUN OK Grok=$grokState Codex=$($codexStates[0])" | Set-Content $log
        exit 0
    }

    $ErrorActionPreference = 'Continue'
    Get-Content -LiteralPath $promptFile -Raw |
        & $codexCmd exec --cd $repo --sandbox workspace-write --color never `
            --output-last-message $final - 2>&1 |
        Tee-Object -FilePath $log
    $exitCode = $LASTEXITCODE
    $ErrorActionPreference = 'Stop'
    if ($exitCode -ne 0) { throw "Codex fallo con $exitCode. Log: $log" }
} finally {
    if ($null -ne $lock) { $lock.Dispose() }
}
