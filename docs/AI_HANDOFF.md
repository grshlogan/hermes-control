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
- Daemon route `/v1/models/{model_id}/action` accepts typed `ModelAction`
  requests and reuses the same confirmation/audit/executor pipeline.
- CLI `model <start|stop|restart|health|benchmark>` posts typed daemon model
  actions.
- WSL root helper package now includes vLLM start/stop/health/logs/benchmark
  scripts and appends `VLLM_*` defaults into existing runtime env files.

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

- `869971c feat: close out Phase 4 daemon CLI actions`
- `90a5a99 feat: add WSL root helper integration`
- `c95855c feat: add Hermes fixed-script execution boundary`
- `44a6eaa feat: expose execution status on confirm`
- `660a210 feat: wire allowlisted Windows executor`

## Current Phase

Phase 5 is in progress. Current local unpushed work:

- Initial vLLM runtime action planner, daemon route, CLI model action client, and
  WSL root vLLM helpers.
- The helpers have been installed into `/opt/hermes-control/bin`.
- `hermes-control-vllm-health.sh qwen36-mtp 1 ready` correctly returns
  `unhealthy` because the vLLM endpoint is not currently ready; no model start
  smoke was run to avoid occupying GPU unintentionally.
- Next Phase 5 increment should decide whether to actually start qwen36-mtp or
  AWQ in a controlled smoke, then add readiness polling/state persistence.

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
