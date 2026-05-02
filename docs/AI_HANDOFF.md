# AI Handoff

Hermes Control is now a Rust workspace with Phase 1 and Phase 2 complete. Phase
3 has authenticated read-only daemon API and SQLite state/audit foundations;
Phase 4 has typed WSL/Hermes operation planning, dry-run previews,
destructive-action confirmation records, confirm/cancel endpoints, and a pending
operation lock. Confirmed operations now flow through an injectable executor
abstraction. The daemon binary wires an allowlisted Windows command executor for
confirmed operations; library/test router defaults can still use no-op or fake
executors. Phase 5 has started with initial vLLM runtime action planning.

## Phase Report

Phase 1 is complete:

- Rust workspace created under `E:\WSL\Hermres\hermes-control`.
- Crates added for `types`, `core`, `daemon`, `cli`, `bot`, `gui`, and `testkit`.
- Config templates added under `config/`.
- Bot subprocess boundary documented and tested.
- Local git repo initialized and pushed to the private GitHub remote.

Phase 2 is complete:

- Config loading and validation are implemented in `hermes-control-core`.
- Read-only status collection checks WSL, Hermes health URL, vLLM models, and
  state/audit DB file presence.
- CLI read-only commands are implemented: `status`, `health`, `providers`,
  `models`, `wsl status`, `model status`, plus `--json`.
- Read-only DTOs live in `hermes-control-types`.
- Workspace build, tests, fmt, and clippy passed before the Phase 2 commit.

Phase 3 has started:

- `hermes-control-daemon` builds authenticated Axum routes for `/v1/status`,
  `/v1/health`, `/v1/providers`, `/v1/models`, `/v1/route/active`, and
  `/v1/audit`.
- Daemon routes require `Authorization: Bearer <token>`.
- Build-time daemon initialization creates SQLite state and audit databases.
- Initial state tables cover active route, operation state, and confirmations.
- Initial audit table covers append-only audit event summaries.

Phase 4 is complete:

- `hermes-control-core` exposes `WslController` and `HermesRuntimeController`
  plan builders.
- WSL restart/shutdown plans produce fixed `wsl.exe` command previews and
  require confirmation.
- Hermes destructive actions produce confirmation-required operation plans
  without raw shell commands.
- `hermes-control-daemon` accepts `/v1/wsl/action` and `/v1/hermes/action`.
- Dry-run action requests return typed command previews.
- Non-dry-run destructive actions create confirmation records and audit preview
  events.
- Pending confirmation records lock out a second mutating action until confirmed
  or cancelled.
- `/v1/confirm` marks the pending confirmation, passes the stored operation to
  an injected executor, and records the executor outcome.
- `/v1/confirm` now includes optional `execution_status` so clients can
  distinguish confirmation success from execution success/failure.
- `/v1/cancel` marks the pending confirmation and operation as cancelled.
- `WindowsCommandExecutor` validates all command previews before running
  anything and currently allows only fixed WSL command shapes:
  `wsl.exe --shutdown`, `wsl.exe --terminate <safe-distro>`, and
  `wsl.exe --distribution <safe-distro> --user <safe-user> --exec true`.
- Hermes restart/stop/kill/wake plans now produce fixed WSL root helper previews
  under `/opt/hermes-control/bin`: `hermes-control-restart.sh`,
  `hermes-control-stop.sh`, `hermes-control-kill.sh`,
  `hermes-control-start.sh`, and `hermes-control-health.sh 30 ready`.
- Confirmed Hermes destructive operations can now flow to the Windows executor
  through that fixed helper allowlist. This requires installing the WSL root
  helper package from `scripts/wsl-root/install.sh`.
- Non-confirming normal mutating actions such as WSL/Hermes wake now execute
  immediately through the injected executor, create `operation_state`, append
  audit events, and return executor status.
- The daemon binary uses `WindowsCommandExecutor`; tests use fake or no-op
  executors.
- Failed executor outcomes are stored as `failed` operation state and release
  the pending operation lock for a later retry.

Phase 5 has started:

- `ModelRuntimeController` builds typed vLLM operation plans from
  `config/model-runtimes.toml`.
- Model `Start` emits canonical WSL root helper commands for
  `hermes-control-vllm-start.sh <variant>` plus
  `hermes-control-vllm-health.sh <served-model> 180 ready`.
- Model `Stop` and `Restart` require confirmation; `Benchmark` is marked
  experimental and reserved for a later Phase 5 increment.
- Model `Install` runs the project-owned vLLM bootstrap helper through the same
  daemon/WSL root execution boundary.
- Daemon route `/v1/models/{model_id}/action` accepts typed `ModelAction`
  requests and reuses the same confirmation/audit/executor pipeline.
- CLI `model <start|stop|restart|health|benchmark>` posts typed daemon model
  actions.
- WSL root helper package now includes vLLM start/stop/health/logs/benchmark
  scripts and appends `VLLM_*` defaults into existing runtime env files.
- The software-owned vLLM runtime boundary is now
  `E:\WSL\Hermres\hermes-control\vLLM`; `E:\WSL\vLLM\models` is only the
  external model-weight store.
- Project runtime scripts now exist under `vLLM/scripts/` for environment setup,
  bootstrap/repair install, OpenAI-compatible serving, qwen36 MTP, and qwen36
  AWQ INT4 eager startup.

## Current Runtime Observation

The last real Phase 4 E2E smoke observed on May 2, 2026:

- WSL distro `Ubuntu-Hermes-Codex` could be stopped, woken, and restarted by
  daemon API while the Windows daemon stayed alive.
- Hermes restart, stop, kill, and wake all completed through daemon API.
- Final restored state was WSL `Running` and Hermes health
  `http://127.0.0.1:8642/health` returning HTTP 200.
- Config was corrected to `wsl.default_user = "root"` because the distro has no
  `hermes` Linux user and Hermes process control is a WSL root boundary.
- New product-owned helpers should be installed to `/opt/hermes-control/bin`;
  observed legacy files under `/root/Hermres` are not daemon targets anymore.
- Config was corrected to Hermes health URL `http://127.0.0.1:8642/health`.
- Configured vLLM model endpoints at `http://127.0.0.1:18080/v1/models` were
  not ready.
- Overall status was `Degraded`.

Treat this as a snapshot, not a permanent fact; rerun CLI status before making
runtime claims.

## Current Commit Baseline

Latest pushed commits:

- `99dac2a feat: start Phase 5 vLLM runtime actions`
- `869971c feat: close out Phase 4 daemon CLI actions`
- `90a5a99 feat: add WSL root helper integration`
- `c95855c feat: add Hermes fixed-script execution boundary`
- `44a6eaa feat: expose execution status on confirm`

## Current Phase

Phase 5 is in progress. Current local unpushed work:

- Project-owned vLLM runtime scaffold under
  `E:\WSL\Hermres\hermes-control\vLLM`.
- `config/model-runtimes.toml` points qwen36 AWQ and MTP start scripts and logs
  at that project runtime.
- WSL root installer/common defaults migrate `VLLM_*` runtime paths away from
  the old shared `E:\WSL\vLLM` workspace while preserving
  `E:\WSL\vLLM\models` as the model-weight store.
- `vLLM/scripts/bootstrap.sh` can create or repair the project venv with
  direct-first/fallback-proxy dependency installation. vLLM socket/temp files
  default to WSL `/tmp` for DrvFS compatibility; pip cache falls back there when
  DrvFS ownership makes pip disable the project cache.
- CLI `model install <model-id>` posts `ModelAction::Install`; dry-run previews
  are available before running the bootstrap.
- Telegram `/model install <model-id>` maps to the same daemon action.
- The bootstrap helper was run successfully on this machine and installed vLLM
  0.20.0 plus Torch 2.11.0 into
  `E:\WSL\Hermres\hermes-control\vLLM\.venv`.
- `qwen36-mtp` has been started from the project-owned vLLM runtime and verified
  live. The working endpoint on this WSL distro is the WSL primary IP
  (`http://10.2.176.55:18080/v1` during the 2026-05-03 smoke), not loopback.
- `hermes-control-vllm-health.sh qwen36-mtp 5 ready` now returns ready after
  fixing the `/v1/models` body parser and runtime endpoint resolution.
- Hermes `custom:vllm` and Open WebUI were both verified against the local MTP
  model through real chat completion calls that returned `OK`.
- Next Phase 5 increment should persist runtime readiness/state, reduce noisy
  vLLM environment warnings, and make the local route switch reproducible
  without hand-editing Hermes/Open WebUI config.

## Useful Commands

```powershell
cd E:\WSL\Hermres\hermes-control
cargo fmt --all -- --check
cargo test --workspace
cargo build --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

Install or refresh WSL root helpers:

```powershell
wsl.exe -d Ubuntu-Hermes-Codex -u root --exec bash -lc "cd /mnt/e/WSL/Hermres/hermes-control && bash scripts/wsl-root/install.sh"
wsl.exe -d Ubuntu-Hermes-Codex -u root --exec /opt/hermes-control/bin/hermes-control-status.sh
wsl.exe -d Ubuntu-Hermes-Codex -u root --exec bash -lc "cd /mnt/e/WSL/Hermres/hermes-control && bash vLLM/scripts/bootstrap.sh"
```

Read-only CLI smoke:

```powershell
cargo run -p hermes-control-cli -- status
cargo run -p hermes-control-cli -- status --json
cargo run -p hermes-control-cli -- providers
cargo run -p hermes-control-cli -- models
```

Phase 4 daemon CLI smoke:

```powershell
$env:HERMES_CONTROL_API_TOKEN = "phase4-token"
cargo run -p hermes-control-daemon
cargo run -p hermes-control-cli -- --api-token phase4-token hermes restart --dry-run --reason "smoke"
cargo run -p hermes-control-cli -- --api-token phase4-token hermes wake --reason "smoke"
cargo run -p hermes-control-cli -- --api-token phase4-token confirm HERMES-1234
```

Phase 5 model dry-run smoke:

```powershell
$env:HERMES_CONTROL_API_TOKEN = "phase5-token"
cargo run -p hermes-control-daemon
cargo run -p hermes-control-cli -- --api-token phase5-token model start qwen36-mtp --dry-run --reason "smoke"
wsl.exe -d Ubuntu-Hermes-Codex -u root --exec /opt/hermes-control/bin/hermes-control-vllm-health.sh qwen36-mtp 1 ready
```

## Handoff Notes

- Keep `docs/RECENT_CHANGES.md` updated after each landed conversation-level
  change.
- Phase 5 should stay focused on vLLM runtime control before route switching.
- Tauri belongs in Phase 8 as a GUI shell and typed daemon client only.
- Ask for explicit approval before commit and push.
