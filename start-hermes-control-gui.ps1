param(
    [string]$ApiToken = "phase8-dev-token",
    [string]$DaemonUrl = "http://127.0.0.1:18787",
    [string]$OperatorId = "desktop-gui",
    [switch]$Force
)

$ErrorActionPreference = "Stop"

$Root = Split-Path -Parent $MyInvocation.MyCommand.Path
$StartScript = Join-Path $Root "start-hermes-control.ps1"

if (-not (Test-Path -LiteralPath $StartScript)) {
    throw "Missing Hermes Control launcher: $StartScript"
}

Write-Host "Starting Hermes Control desktop GUI..."
if ($Force) {
    & $StartScript `
        -GuiMode Tauri `
        -ApiToken $ApiToken `
        -DaemonUrl $DaemonUrl `
        -OperatorId $OperatorId `
        -Force
} else {
    & $StartScript `
        -GuiMode Tauri `
        -ApiToken $ApiToken `
        -DaemonUrl $DaemonUrl `
        -OperatorId $OperatorId
}
