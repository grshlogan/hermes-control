# WSL Root Integration

This document defines the product-owned WSL2 root-side contract for Hermes
Control. It replaces inherited ad hoc `.sh` files with a small fixed helper
package that can be installed on a fresh WSL distro.

## Why Root Is Explicit

Hermes currently runs with root authority inside WSL2 because it needs privileged
access for WSL-to-Windows operations. Hermes Control therefore treats root as an
explicit integration boundary instead of pretending a normal Linux user can own
the Hermes lifecycle.

The Windows daemon still stays outside WSL. It calls `wsl.exe` with fixed
arguments and the WSL user `root`; root authority is only entered through the
installed helper scripts below.

## Installed Contract

The daemon may execute only these Hermes helper commands:

```text
wsl.exe --distribution <safe-distro> --user root --exec /opt/hermes-control/bin/hermes-control-start.sh
wsl.exe --distribution <safe-distro> --user root --exec /opt/hermes-control/bin/hermes-control-stop.sh
wsl.exe --distribution <safe-distro> --user root --exec /opt/hermes-control/bin/hermes-control-restart.sh
wsl.exe --distribution <safe-distro> --user root --exec /opt/hermes-control/bin/hermes-control-kill.sh
wsl.exe --distribution <safe-distro> --user root --exec /opt/hermes-control/bin/hermes-control-health.sh 30 ready
```

No legacy `/root/Hermres/*.sh` script is part of the daemon allowlist.

The daemon may also execute these vLLM helper shapes:

```text
wsl.exe --distribution <safe-distro> --user root --exec /opt/hermes-control/bin/hermes-control-vllm-start.sh <variant-id>
wsl.exe --distribution <safe-distro> --user root --exec /opt/hermes-control/bin/hermes-control-vllm-stop.sh <served-model-name>
wsl.exe --distribution <safe-distro> --user root --exec /opt/hermes-control/bin/hermes-control-vllm-health.sh <served-model-name> <seconds> ready
wsl.exe --distribution <safe-distro> --user root --exec /opt/hermes-control/bin/hermes-control-vllm-logs.sh <variant-id> <line-count>
wsl.exe --distribution <safe-distro> --user root --exec /opt/hermes-control/bin/hermes-control-vllm-benchmark.sh <variant-id>
```

The first Phase 5 benchmark helper is intentionally reserved and exits with a
clear message; real benchmark execution should land with benchmark storage.

## Source Layout

```text
scripts/wsl-root/
  install.sh
  bin/
    hermes-control-common.sh
    hermes-control-start.sh
    hermes-control-stop.sh
    hermes-control-restart.sh
    hermes-control-kill.sh
    hermes-control-health.sh
    hermes-control-status.sh
    hermes-control-vllm-start.sh
    hermes-control-vllm-stop.sh
    hermes-control-vllm-health.sh
    hermes-control-vllm-logs.sh
    hermes-control-vllm-benchmark.sh
```

`install.sh` copies these files into `/opt/hermes-control/bin` and creates the
runtime config file at `/etc/hermes-control/runtime.env`.

## Fresh Install Flow

WSL prerequisites:

```bash
apt-get update
apt-get install -y curl python3 coreutils
```

Run this from Windows PowerShell after cloning or unpacking `hermes-control`:

```powershell
wsl.exe -d Ubuntu-Hermes-Codex -u root --exec bash -lc "cd /mnt/e/WSL/Hermres/hermes-control && bash scripts/wsl-root/install.sh"
```

Then inspect and adjust the WSL runtime config:

```powershell
wsl.exe -d Ubuntu-Hermes-Codex -u root --exec cat /etc/hermes-control/runtime.env
```

Important fields:

```text
HERMES_CONTROL_WORK_ROOT=/root/Hermres
HERMES_AGENT_ROOT=/root/Hermres/hermes-agent
HERMES_VENV_BIN=/root/Hermres/hermes-agent/.venv-hermes/bin/hermes
HERMES_HEALTH_URL=http://127.0.0.1:8642/health
HERMES_LOG_DIR=/root/Hermres/logs
HERMES_PID_FILE=/run/hermes-control/hermes-gateway.pid
HERMES_ENV_FILE=/root/.hermes/.env
VLLM_WORKSPACE=/mnt/e/WSL/vLLM
VLLM_MODELS_ENDPOINT=http://127.0.0.1:18080/v1/models
VLLM_LOG_DIR=/mnt/e/WSL/vLLM/logs
VLLM_START_QWEN36_MTP=/mnt/e/WSL/vLLM/scripts/start-qwen36-mtp.sh
VLLM_START_QWEN36_AWQ_INT4=/mnt/e/WSL/vLLM/scripts/start-qwen36-int4-eager.sh
```

On a fresh machine, set these paths to the actual Hermes installation before
starting Hermes through the daemon.

## Verification

Check helper installation:

```powershell
wsl.exe -d Ubuntu-Hermes-Codex -u root --exec /opt/hermes-control/bin/hermes-control-status.sh
```

Start and health-check Hermes through the product-owned helpers:

```powershell
wsl.exe -d Ubuntu-Hermes-Codex -u root --exec /opt/hermes-control/bin/hermes-control-start.sh
wsl.exe -d Ubuntu-Hermes-Codex -u root --exec /opt/hermes-control/bin/hermes-control-health.sh 30 ready
```

The status and health helpers emit JSON. The daemon does not depend on legacy
process probes such as `service-status.sh`.

Check vLLM readiness without starting a model:

```powershell
wsl.exe -d Ubuntu-Hermes-Codex -u root --exec /opt/hermes-control/bin/hermes-control-vllm-health.sh qwen36-mtp 1 ready
```
