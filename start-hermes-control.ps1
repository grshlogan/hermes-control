param(
    [ValidateSet("Web", "Tauri")]
    [string]$GuiMode = "Web",
    [string]$ApiToken = "phase8-dev-token",
    [string]$DaemonUrl = "http://127.0.0.1:18787",
    [int]$GuiPort = 5174,
    [string]$OperatorId = "browser-gui",
    [switch]$NoGui,
    [switch]$Force
)

$ErrorActionPreference = "Stop"

$Root = Split-Path -Parent $MyInvocation.MyCommand.Path
$StateDir = Join-Path $Root "state"
$LogDir = Join-Path $Root "logs\local-run"
$DaemonLogDir = Join-Path $Root "logs\daemon"
$GuiDir = Join-Path $Root "apps\hermes-control-gui"
$DaemonPidFile = Join-Path $StateDir "hermes-control-daemon.pid"
$GuiPidFile = Join-Path $StateDir "hermes-control-gui.pid"

function Ensure-Directory([string]$Path) {
    if (-not (Test-Path -LiteralPath $Path)) {
        New-Item -ItemType Directory -Path $Path | Out-Null
    }
}

function Test-TcpPort([int]$Port) {
    $connection = Get-NetTCPConnection -LocalPort $Port -ErrorAction SilentlyContinue |
        Where-Object { $_.State -eq "Listen" } |
        Select-Object -First 1
    return $null -ne $connection
}

function Stop-ExistingManagedProcess([string]$PidFile, [string]$Name) {
    if (-not (Test-Path -LiteralPath $PidFile)) {
        return
    }

    $rawPid = (Get-Content -LiteralPath $PidFile -ErrorAction SilentlyContinue | Select-Object -First 1)
    if ($rawPid -match "^\d+$") {
        $process = Get-Process -Id ([int]$rawPid) -ErrorAction SilentlyContinue
        if ($process) {
            Write-Host "Stopping existing $Name process PID $rawPid"
            Stop-Process -Id ([int]$rawPid) -Force
        }
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

function Stop-NamedProcess([string]$ProcessName) {
    $processes = Get-Process -Name $ProcessName -ErrorAction SilentlyContinue
    foreach ($process in $processes) {
        Write-Host "Stopping $($process.ProcessName) PID $($process.Id)"
        Stop-Process -Id $process.Id -Force
    }
}

function Wait-HttpOk([string]$Url, [hashtable]$Headers, [int]$TimeoutSeconds) {
    $deadline = (Get-Date).AddSeconds($TimeoutSeconds)
    do {
        try {
            Invoke-WebRequest -Uri $Url -Headers $Headers -UseBasicParsing -TimeoutSec 3 | Out-Null
            return $true
        } catch {
            Start-Sleep -Milliseconds 500
        }
    } while ((Get-Date) -lt $deadline)
    return $false
}

function Resolve-CommandPath([string]$CommandName) {
    return (Get-Command $CommandName -ErrorAction Stop).Source
}

Ensure-Directory $StateDir
Ensure-Directory $LogDir
Ensure-Directory $DaemonLogDir

if ($Force) {
    Stop-ExistingManagedProcess $DaemonPidFile "daemon"
    Stop-ExistingManagedProcess $GuiPidFile "GUI"
    Stop-NamedProcess "hermes-control-gui-tauri"
    Stop-PortOwner 5173 @("node")
    Stop-PortOwner 18787 @("hermes-control-daemon")
    Stop-PortOwner $GuiPort @("node")
}

$daemonPort = ([Uri]$DaemonUrl).Port
if (Test-TcpPort $daemonPort) {
    Write-Host "Daemon port $daemonPort is already listening; reusing existing daemon."
} else {
    $daemonExe = Join-Path $Root "target\debug\hermes-control-daemon.exe"
    $daemonOut = Join-Path $DaemonLogDir "local-run.out.log"
    $daemonErr = Join-Path $DaemonLogDir "local-run.err.log"

    $envBlock = @{
        HERMES_CONTROL_API_TOKEN = $ApiToken
        HERMES_CONTROL_CONFIG_DIR = (Join-Path $Root "config")
        RUST_LOG = "info"
    }

    if (Test-Path -LiteralPath $daemonExe) {
        $program = $daemonExe
        $arguments = @()
    } else {
        $program = "cargo"
        $arguments = @("run", "-p", "hermes-control-daemon")
    }

    Write-Host "Starting daemon on $DaemonUrl"
    $startInfo = @{
        FilePath = $program
        WorkingDirectory = $Root
        RedirectStandardOutput = $daemonOut
        RedirectStandardError = $daemonErr
        WindowStyle = "Hidden"
        PassThru = $true
    }
    if ($arguments.Count -gt 0) {
        $startInfo.ArgumentList = $arguments
    }

    foreach ($item in $envBlock.GetEnumerator()) {
        [Environment]::SetEnvironmentVariable($item.Key, $item.Value, "Process")
    }

    $daemonProcess = Start-Process @startInfo
    Set-Content -LiteralPath $DaemonPidFile -Value $daemonProcess.Id -Encoding ASCII
}

$headers = @{ Authorization = "Bearer $ApiToken" }
if (Wait-HttpOk "$DaemonUrl/v1/health" $headers 45) {
    Write-Host "Daemon ready: $DaemonUrl"
} else {
    Write-Warning "Daemon did not answer /v1/health within 45 seconds. Check logs\daemon\local-run.err.log."
}

if ($NoGui) {
    Write-Host "GUI disabled by -NoGui."
    exit 0
}

if ($GuiMode -eq "Web") {
    if (Test-TcpPort $GuiPort) {
        Write-Host "GUI port $GuiPort is already listening; open http://127.0.0.1:$GuiPort/"
        exit 0
    }

    $guiOut = Join-Path $LogDir "gui-web.out.log"
    $guiErr = Join-Path $LogDir "gui-web.err.log"
    $npmCmd = Resolve-CommandPath "npm.cmd"
    Write-Host "Starting Web GUI on http://127.0.0.1:$GuiPort/"
    $guiProcess = Start-Process `
        -FilePath $npmCmd `
        -ArgumentList @("run", "dev", "--", "--port", "$GuiPort") `
        -WorkingDirectory $GuiDir `
        -RedirectStandardOutput $guiOut `
        -RedirectStandardError $guiErr `
        -WindowStyle Hidden `
        -PassThru
    Set-Content -LiteralPath $GuiPidFile -Value $guiProcess.Id -Encoding ASCII
    Write-Host "Web GUI: http://127.0.0.1:$GuiPort/"
    exit 0
}

$env:HERMES_CONTROL_DAEMON_URL = $DaemonUrl
$env:HERMES_CONTROL_API_TOKEN = $ApiToken
$env:HERMES_CONTROL_GUI_OPERATOR_ID = $OperatorId
$guiTauriOut = Join-Path $LogDir "gui-tauri.out.log"
$guiTauriErr = Join-Path $LogDir "gui-tauri.err.log"
$npmCmd = Resolve-CommandPath "npm.cmd"
Write-Host "Starting Tauri GUI"
$tauriProcess = Start-Process `
    -FilePath $npmCmd `
    -ArgumentList @("run", "tauri", "dev") `
    -WorkingDirectory $GuiDir `
    -RedirectStandardOutput $guiTauriOut `
    -RedirectStandardError $guiTauriErr `
    -WindowStyle Hidden `
    -PassThru
Set-Content -LiteralPath $GuiPidFile -Value $tauriProcess.Id -Encoding ASCII
