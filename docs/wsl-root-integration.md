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
wsl.exe --distribution <safe-distro> --user root --exec /opt/hermes-control/bin/hermes-control-vllm-start-with-fallback.sh <primary-variant-id> <fallback-variant-id>
wsl.exe --distribution <safe-distro> --user root --exec /opt/hermes-control/bin/hermes-control-vllm-stop.sh <served-model-name>
wsl.exe --distribution <safe-distro> --user root --exec /opt/hermes-control/bin/hermes-control-vllm-health.sh <served-model-name> <seconds> ready
wsl.exe --distribution <safe-distro> --user root --exec /opt/hermes-control/bin/hermes-control-vllm-logs.sh <variant-id> <line-count>
wsl.exe --distribution <safe-distro> --user root --exec /opt/hermes-control/bin/hermes-control-vllm-benchmark.sh <variant-id>
wsl.exe --distribution <safe-distro> --user root --exec /opt/hermes-control/bin/hermes-control-vllm-bootstrap.sh <variant-id>
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
    hermes-control-vllm-start-with-fallback.sh
    hermes-control-vllm-stop.sh
    hermes-control-vllm-health.sh
    hermes-control-vllm-logs.sh
    hermes-control-vllm-benchmark.sh
    hermes-control-vllm-bootstrap.sh
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
VLLM_WORKSPACE=/mnt/e/WSL/Hermres/hermes-control/vLLM
VLLM_MODEL_ROOT=/mnt/e/WSL/vLLM/models
VLLM_PORT=18080
VLLM_CLIENT_HOST=auto
VLLM_MODELS_ENDPOINT=auto
VLLM_LOG_DIR=/mnt/e/WSL/Hermres/hermes-control/vLLM/logs
VLLM_START_QWEN36_MTP=/mnt/e/WSL/Hermres/hermes-control/vLLM/scripts/start-qwen36-mtp.sh
VLLM_START_QWEN36_AWQ_INT4=/mnt/e/WSL/Hermres/hermes-control/vLLM/scripts/start-qwen36-int4-eager.sh
VLLM_BOOTSTRAP_SCRIPT=/mnt/e/WSL/Hermres/hermes-control/vLLM/scripts/bootstrap.sh
```

On a fresh machine, set these paths to the actual Hermes installation before
starting Hermes through the daemon.

For vLLM, `VLLM_WORKSPACE` is the Hermes Control-owned runtime directory for
venv, cache, scripts, logs, and temp files. `VLLM_MODEL_ROOT` is the external
model-weight store and should not be deleted by installer repair flows.
`VLLM_CLIENT_HOST=auto` resolves the WSL primary IP at runtime and
`VLLM_MODELS_ENDPOINT=auto` becomes
`http://<wsl-primary-ip>:${VLLM_PORT}/v1/models`. This avoids relying on
`127.0.0.1` for vLLM on WSL builds where the server socket is visible but not
callable through loopback.

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

Bootstrap or repair the project-owned vLLM environment:

```powershell
wsl.exe -d Ubuntu-Hermes-Codex -u root --exec /opt/hermes-control/bin/hermes-control-vllm-bootstrap.sh qwen36-mtp
```

Start and verify the MTP model:

```powershell
wsl.exe -d Ubuntu-Hermes-Codex -u root --exec /opt/hermes-control/bin/hermes-control-vllm-start-with-fallback.sh qwen36-mtp qwen36-awq-int4
wsl.exe -d Ubuntu-Hermes-Codex -u root --exec /opt/hermes-control/bin/hermes-control-vllm-health.sh qwen36-mtp 600 ready
```

The health JSON includes the resolved `models_endpoint`. Use that endpoint for
Hermes local-provider configuration on the same WSL distro.

The fallback helper first tries the primary variant. If it exits early or does
not become ready before `VLLM_FALLBACK_PRIMARY_TIMEOUT_SECONDS`, it stops the
primary served model and starts the fallback variant, waiting up to
`VLLM_FALLBACK_SECONDARY_TIMEOUT_SECONDS`. Both defaults are 180 seconds.
