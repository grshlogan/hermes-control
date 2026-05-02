# WSL2 and Hermes Provisioning Plan

> Project: Hermes Control
> Purpose: make the local WSL2 + Hermes + Open WebUI + vLLM stack reproducible,
> inspectable, and safe enough for open-source users.
> Date: 2026-05-03

This plan is separate from `plan_rust_control_rewrite.md` on purpose. The main
plan owns the Rust control daemon, clients, and model runtime manager. This plan
owns the local machine provisioning contract: how a fresh or existing Windows
machine becomes a reliable Hermes Control host.

---

## 1. Product Boundary

Hermes Control may assist, repair, and eventually automate the local runtime
stack, but it must not silently take ownership of a user's whole machine.

Hermes Control should manage:

- WSL2 distro discovery and readiness checks.
- Product-owned WSL root helper installation under `/opt/hermes-control/bin`.
- Hermes process start/stop/restart/health through fixed root helpers.
- Hermes local model provider configuration for `custom:vllm`.
- Open WebUI backend configuration when Open WebUI is present.
- Project-owned vLLM runtime under `E:\WSL\Hermres\hermes-control\vLLM`.
- Model readiness, logs, health checks, and call-chain validation.

Hermes Control should not silently do:

- `wsl --unregister`.
- Windows feature installation or distro import without an explicit wizard step.
- Overwrite `~/.hermes/config.yaml`, `~/.hermes/.env`, or Open WebUI database
  without backup.
- Delete model weights under `E:\WSL\vLLM\models`.
- Expose arbitrary shell execution through daemon, GUI, CLI, or bot.

---

## 2. Supported Modes

### Adopt Existing

Use when the user already has WSL2, Hermes, Open WebUI, and model weights.

Expected behavior:

1. Detect WSL distro and confirm it is the selected Hermes distro.
2. Detect Hermes home, executable, config, env file, health URL, and gateway
   port.
3. Detect Open WebUI process/data directory when present.
4. Detect model store, defaulting to `E:\WSL\vLLM\models`.
5. Install or refresh `/opt/hermes-control/bin` helpers.
6. Back up Hermes/Open WebUI config before any mutation.
7. Configure Hermes `custom:vllm` only after vLLM readiness passes.
8. Verify vLLM -> Hermes -> Open WebUI call chain.

### Repair Install

Use when the stack exists but part of it is broken.

May repair:

- Missing or stale `/opt/hermes-control/bin` helpers.
- Missing `/etc/hermes-control/runtime.env` values.
- Project-owned vLLM venv under
  `E:\WSL\Hermres\hermes-control\vLLM\.venv`.
- vLLM cache/log/temp directories under the project-owned runtime.
- Hermes local vLLM provider URL and `NO_PROXY` entries.
- Open WebUI OpenAI backend URL when the user approves.

Must preserve:

- `E:\WSL\vLLM\models`.
- Hermes credentials and auth files.
- Open WebUI user database and secret key.
- Existing user provider profiles unless explicitly switched.

### Fresh Install

Use when the user wants a guided setup on a new machine.

Fresh install should be explicit and step-based:

1. Confirm Windows WSL2 availability.
2. Confirm or create a target WSL distro.
3. Install WSL prerequisites inside the distro.
4. Install Hermes.
5. Install Open WebUI if requested.
6. Install Hermes Control WSL root helpers.
7. Bootstrap project-owned vLLM runtime.
8. Ask user to place model weights under the model store.
9. Start vLLM profile and verify.
10. Configure Hermes local provider and verify.
11. Configure Open WebUI backend and verify.
12. Generate an installation report.

Fresh install must present a dry-run summary before mutating the system.

---

## 3. Filesystem Contract

Windows project root:

```text
E:\WSL\Hermres\hermes-control
```

Project-owned vLLM runtime:

```text
E:\WSL\Hermres\hermes-control\vLLM
```

External model-weight store:

```text
E:\WSL\vLLM\models
```

WSL project root:

```text
/mnt/e/WSL/Hermres/hermes-control
```

WSL Hermes home:

```text
/root/.hermes
```

WSL Hermes work root:

```text
/root/Hermres
```

WSL root helper install prefix:

```text
/opt/hermes-control/bin
```

WSL runtime config:

```text
/etc/hermes-control/runtime.env
```

Rules:

- Project venv/cache/logs/scripts belong under the project-owned `vLLM`
  directory.
- Model weights belong under `E:\WSL\vLLM\models` and are not deleted by repair
  flows.
- vLLM Unix sockets and temporary files may use WSL `/tmp` because DrvFS paths
  can reject Unix sockets.
- All Windows-to-WSL privileged actions go through fixed helper filenames.

---

## 4. WSL2 Provisioning Flow

### Detection

The control daemon should collect:

- `wsl.exe --list --verbose`
- selected distro name
- distro state
- WSL version
- root command availability
- `bash`, `python3`, `curl`, `coreutils`, `install`, `sed`, `awk`
- GPU visibility
- current WSL IP addresses
- proxy environment variables
- systemd availability where relevant

### Minimum WSL Packages

Ubuntu prerequisites:

```bash
apt-get update
apt-get install -y curl python3 python3-venv coreutils sed gawk ca-certificates
```

Useful diagnostics:

```bash
hostname -I
python3 --version
curl --version
```

GPU diagnostics:

```powershell
nvidia-smi
```

WSL GPU diagnostics may fail independently of Windows `nvidia-smi`; the
installer should report both and avoid treating one noisy probe as final proof.

---

## 5. Root Helper Contract

Root helpers are product-owned shell scripts installed to:

```text
/opt/hermes-control/bin
```

Current helper groups:

- `hermes-control-start.sh`
- `hermes-control-stop.sh`
- `hermes-control-restart.sh`
- `hermes-control-kill.sh`
- `hermes-control-health.sh`
- `hermes-control-status.sh`
- `hermes-control-vllm-start.sh`
- `hermes-control-vllm-stop.sh`
- `hermes-control-vllm-health.sh`
- `hermes-control-vllm-logs.sh`
- `hermes-control-vllm-bootstrap.sh`

Rules:

- Helpers must run as root.
- Helpers must emit JSON for machine-readable state when possible.
- Helpers must not read arbitrary user-provided script paths.
- Helpers must bypass proxies for local health checks.
- Helpers must resolve vLLM client endpoint at runtime when configured as
  `auto`.
- Helpers must never delete model weights.

Install or refresh:

```powershell
wsl.exe -d Ubuntu-Hermes-Codex -u root --exec bash -lc "cd /mnt/e/WSL/Hermres/hermes-control && bash scripts/wsl-root/install.sh"
```

---

## 6. vLLM Provisioning Flow

### Bootstrap

Bootstrap project-owned runtime:

```powershell
wsl.exe -d Ubuntu-Hermes-Codex -u root --exec /opt/hermes-control/bin/hermes-control-vllm-bootstrap.sh qwen36-mtp
```

Expected results:

- `E:\WSL\Hermres\hermes-control\vLLM\.venv` exists.
- `vllm` command is available inside that venv.
- vLLM logs are written under
  `E:\WSL\Hermres\hermes-control\vLLM\logs`.
- Dependency install tries direct network first and proxy fallback second.

### Model Start

Start MTP model:

```powershell
wsl.exe -d Ubuntu-Hermes-Codex -u root --exec /opt/hermes-control/bin/hermes-control-vllm-start.sh qwen36-mtp
```

Readiness:

```powershell
wsl.exe -d Ubuntu-Hermes-Codex -u root --exec /opt/hermes-control/bin/hermes-control-vllm-health.sh qwen36-mtp 600 ready
```

Expected readiness JSON:

```json
{
  "ready": true,
  "served_model_name": "qwen36-mtp",
  "models_endpoint": "http://<wsl-primary-ip>:18080/v1/models"
}
```

The resolved `models_endpoint` is the source of truth for local Hermes/Open
WebUI routing.

### Completeness Checks

The software should validate:

- `/v1/models` returns the served model id.
- model root matches the expected model store.
- max model length is recorded.
- `/v1/chat/completions` returns a short deterministic response.
- logs show MTP architecture and speculative config when MTP is expected.
- no fatal CUDA/NCCL/OOM pattern exists in the latest log tail.

---

## 7. Hermes Provisioning Flow

### Detection

Detect:

- Hermes executable path.
- Hermes health URL.
- Hermes gateway port.
- `~/.hermes/config.yaml`.
- `~/.hermes/.env`.
- active model provider.
- `custom_providers` entries.
- `NO_PROXY` values.

### Backup

Before mutation:

```text
/root/.hermes/backups/hermes-control-<timestamp>/config.yaml
/root/.hermes/backups/hermes-control-<timestamp>/.env
```

### Configure Local vLLM Provider

When vLLM is ready, Hermes Control should ensure:

```yaml
model:
  provider: custom:vllm
  default: qwen36-mtp
  base_url: http://<wsl-primary-ip>:18080/v1
  api_mode: chat_completions
  context_length: 90000

custom_providers:
  - name: vllm
    base_url: http://<wsl-primary-ip>:18080/v1
    model: qwen36-mtp
    api_mode: chat_completions
    models:
      qwen36-mtp:
        context_length: 90000
```

The exact key environment should use the existing local relay key when present.
Do not create or print secrets in logs.

### Configure Proxy Bypass

Hermes `.env` should include the vLLM endpoint host:

```text
NO_PROXY=localhost,127.0.0.1,::1,<wsl-primary-ip>,10.*
no_proxy=localhost,127.0.0.1,::1,<wsl-primary-ip>,10.*
```

### Restart and Verify

```powershell
wsl.exe -d Ubuntu-Hermes-Codex -u root --exec /opt/hermes-control/bin/hermes-control-restart.sh
wsl.exe -d Ubuntu-Hermes-Codex -u root --exec /opt/hermes-control/bin/hermes-control-health.sh 60 ready
```

Hermes call-chain smoke:

```text
POST http://127.0.0.1:8642/v1/chat/completions
model: hermes-agent
expected assistant content: OK
```

---

## 8. Open WebUI Provisioning Flow

### Detection

Detect:

- Open WebUI process.
- Open WebUI data directory.
- `webui.db`.
- `.webui_secret_key`.
- configured OpenAI base URLs and API keys.

### Preferred Backend

Open WebUI should talk to Hermes gateway, not directly to vLLM, by default:

```text
OPENAI_API_BASE_URLS=http://127.0.0.1:8642/v1
DEFAULT_MODELS=hermes-agent
```

Reason:

- Hermes stays the route owner.
- Switching external/local providers remains centralized.
- Open WebUI does not need to know vLLM internals.

Direct-to-vLLM mode may be offered as an advanced diagnostic profile.

### Verification

Open WebUI should pass:

```text
GET  /openai/models
POST /openai/chat/completions
```

Expected:

- `hermes-agent` appears in models.
- chat completion returns `OK`.

The provisioning tool should avoid printing JWTs, API keys, or Open WebUI
secret key values.

---

## 9. End-to-End Validation Ladder

Validation should run in this order:

1. Windows WSL state check.
2. WSL root helper install check.
3. Hermes health check.
4. vLLM bootstrap check.
5. vLLM start check.
6. vLLM `/v1/models` check.
7. vLLM chat completion check.
8. Hermes `custom:vllm` config check.
9. Hermes restart and health check.
10. Hermes chat completion check.
11. Open WebUI config check.
12. Open WebUI models check.
13. Open WebUI chat completion check.
14. Installation report.

Each step should record:

- status
- command or API route used
- redacted output summary
- log file path
- suggested repair action

---

## 10. Daemon and CLI Work Items

Future typed actions:

- `provision inspect`
- `provision adopt`
- `provision repair`
- `provision fresh`
- `provision backup`
- `provision report`
- `hermes route local-vllm`
- `openwebui configure-hermes`
- `stack verify`

Daemon DTOs should live in `hermes-control-types`.

State should record:

- selected WSL distro
- resolved WSL primary IP
- vLLM endpoint
- Hermes config backup id
- Open WebUI config backup id
- last successful stack verification time

All mutating provisioning actions require dry-run previews and audit events.

---

## 11. Completion Signal

This provisioning plan is complete when:

- A new user can follow README and prepare only model weights manually.
- Hermes Control can detect whether the machine is in Adopt Existing, Repair
  Install, or Fresh Install state.
- Product-owned root helpers can be installed or refreshed safely.
- vLLM can be bootstrapped, started, health-checked, and called.
- Hermes can be configured to local vLLM and called.
- Open WebUI can be configured through Hermes and called.
- All config mutations create backups.
- All high-risk operations require explicit confirmation.
- A final report explains exactly what was changed and how to undo it.

---

## 12. Current Machine Snapshot

As of 2026-05-03 on the current development machine:

- WSL distro: `Ubuntu-Hermes-Codex`
- Hermes health: `http://127.0.0.1:8642/health`
- Open WebUI: `http://127.0.0.1:3000`
- vLLM served model: `qwen36-mtp`
- vLLM callable endpoint observed:
  `http://10.2.176.55:18080/v1`
- Verified chains:
  - vLLM direct chat completion returned `OK`.
  - Hermes gateway chat completion returned `OK`.
  - Open WebUI OpenAI route returned `OK`.

This snapshot is not a portable constant. Future setup code must resolve WSL IP
and endpoint facts at runtime.
