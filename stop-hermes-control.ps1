param(
    [switch]$StopVllm,
    [switch]$StopHermes,
    [switch]$ShutdownWsl
)

$ErrorActionPreference = "Stop"

$Root = Split-Path -Parent $MyInvocation.MyCommand.Path
$StateDir = Join-Path $Root "state"
$DaemonPidFile = Join-Path $StateDir "hermes-control-daemon.pid"
$GuiPidFile = Join-Path $StateDir "hermes-control-gui.pid"

function Stop-PidFileProcess([string]$PidFile, [string]$Name) {
    if (-not (Test-Path -LiteralPath $PidFile)) {
        Write-Host "$Name PID file not found."
        return
    }

    $rawPid = (Get-Content -LiteralPath $PidFile -ErrorAction SilentlyContinue | Select-Object -First 1)
    if ($rawPid -notmatch "^\d+$") {
        Write-Warning "$Name PID file does not contain a valid PID: $PidFile"
        Remove-Item -LiteralPath $PidFile -Force -ErrorAction SilentlyContinue
        return
    }

    $process = Get-Process -Id ([int]$rawPid) -ErrorAction SilentlyContinue
    if ($process) {
        Write-Host "Stopping $Name PID $rawPid"
        Stop-Process -Id ([int]$rawPid) -Force
    } else {
        Write-Host "$Name PID $rawPid is not running."
    }

    Remove-Item -LiteralPath $PidFile -Force -ErrorAction SilentlyContinue
}

function Stop-PortOwner([int]$Port, [string[]]$AllowedNames) {
    $connections = Get-NetTCPConnection -LocalPort $Port -State Listen -ErrorAction SilentlyContinue
    foreach ($connection in $connections) {
        $pidValue = [int]$connection.OwningProcess
        $process = Get-Process -Id $pidValue -ErrorAction SilentlyContinue
        if ($process -and ($AllowedNames -contains $process.ProcessName)) {
            Write-Host "Stopping $($process.ProcessName) on port $Port PID $pidValue"
            Stop-Process -Id $pidValue -Force
        }
    }
}

Stop-PidFileProcess $GuiPidFile "GUI"
Stop-PidFileProcess $DaemonPidFile "daemon"

# Clean up common dev leftovers even if the PID files are missing.
Stop-PortOwner 5174 @("node")
Stop-PortOwner 18787 @("hermes-control-daemon")

if ($StopVllm) {
    Write-Host "Stopping vLLM qwen36-mtp via WSL root helper"
    wsl.exe -d Ubuntu-Hermes-Codex -u root --exec /opt/hermes-control/bin/hermes-control-vllm-stop.sh qwen36-mtp
}

if ($StopHermes) {
    Write-Host "Stopping Hermes via WSL root helper"
    wsl.exe -d Ubuntu-Hermes-Codex -u root --exec /opt/hermes-control/bin/hermes-control-stop.sh
}

if ($ShutdownWsl) {
    Write-Host "Shutting down all WSL distributions"
    wsl.exe --shutdown
}

Write-Host "Hermes Control local app processes stopped."
