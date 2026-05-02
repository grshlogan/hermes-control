# AI Handoff

Hermes Control is now a Rust workspace with Phase 1 and Phase 2 complete. Phase
3 has authenticated read-only daemon API and SQLite state/audit foundations;
Phase 4 has typed WSL/Hermes operation planning, dry-run previews,
destructive-action confirmation records, confirm/cancel endpoints, and a pending
operation lock. Confirmed operations flow through an injectable executor
abstraction. Phase 5 basic vLLM runtime control is now closed out, including
project-owned vLLM provisioning, live qwen36 MTP validation, daemon-backed
`model logs`, and AWQ fallback start planning. Phase 6 has started with a
state-only route switch skeleton.

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

Phase 5 basic closeout is complete:

- `ModelRuntimeController` builds typed vLLM operation plans from
  `config/model-runtimes.toml`.
- Model `Start` for MTP variants emits the canonical
  `hermes-control-vllm-start-with-fallback.sh <primary> <fallback>` helper when
  a stable same-runtime fallback exists.
- Model `Stop` and `Restart` require confirmation; `Benchmark` is marked
  experimental and reserved for a later Phase 5 increment.
- Model `Install` runs the project-owned vLLM bootstrap helper through the same
  daemon/WSL root execution boundary.
- Daemon route `/v1/models/{model_id}/action` accepts typed `ModelAction`
  requests and reuses the same confirmation/audit/executor pipeline.
- CLI `model <install|start|stop|restart|health|logs|benchmark>` posts typed
  daemon model actions.
- Daemon executor stdout can now flow through optional `OperationResponse.output`
  so `model logs` renders helper log tails.
- WSL root helper package now includes vLLM start/stop/health/logs/benchmark
  scripts, a bootstrap helper, and a start-with-fallback helper.
- The software-owned vLLM runtime boundary is now
  `E:\WSL\Hermres\hermes-control\vLLM`; `E:\WSL\vLLM\models` is only the
  external model-weight store.
- Project runtime scripts now exist under `vLLM/scripts/` for environment setup,
  bootstrap/repair install, OpenAI-compatible serving, qwen36 MTP, and qwen36
  AWQ INT4 eager startup.
- Live qwen36 MTP, Hermes gateway, and Open WebUI calls were verified on
  2026-05-03.

Phase 6 has started:

- `POST /v1/route/switch` validates provider profile IDs, supports dry-run,
  persists active route state, records last-known-good route, and writes audit
  records.
- `GET /v1/route/active` now returns both `active_profile_id` and
  `last_known_good_profile_id`.
- CLI `route active` reads daemon route state; CLI
  `route switch <profile-id>` posts a typed `RouteSwitchRequest`.
- Local vLLM route switches are blocked unless the configured served model is
  ready.
- Hermes/Open WebUI config patching, Hermes reload/restart, and rollback after
  failed reload are still pending Phase 6 work.

## Current Runtime Observation

The last real runtime smoke observed on May 3, 2026:

- Config was corrected to `wsl.default_user = "root"` because the distro has no
  `hermes` Linux user and Hermes process control is a WSL root boundary.
- New product-owned helpers should be installed to `/opt/hermes-control/bin`;
  observed legacy files under `/root/Hermres` are not daemon targets anymore.
- Config was corrected to Hermes health URL `http://127.0.0.1:8642/health`.
- qwen36 MTP started from `E:\WSL\Hermres\hermes-control\vLLM` and returned
  `OK` through `/v1/chat/completions`.
- The working vLLM endpoint was the WSL primary IP during that smoke:
  `http://10.2.176.55:18080/v1`. Treat this as a snapshot.
- Hermes `custom:vllm` and Open WebUI's Hermes-backed route both returned `OK`
  through the local model.

Treat this as a snapshot, not a permanent fact; rerun CLI status before making
runtime claims.

## Current Commit Baseline

Latest pushed commits:

- `48d1b37 feat: add Phase 5 vLLM provisioning flow`
- `99dac2a feat: start Phase 5 vLLM runtime actions`
- `869971c feat: close out Phase 4 daemon CLI actions`
- `90a5a99 feat: add WSL root helper integration`
- `c95855c feat: add Hermes fixed-script execution boundary`

## Current Phase

Phase 6 has started. Current local unpushed work:

- `OperationResponse.output` and daemon stdout capture for log-style helpers.
- CLI `model logs <model-id>` daemon action support.
- WSL root `hermes-control-vllm-start-with-fallback.sh`.
- MTP start/restart plan fallback to stable AWQ variant.
- Daemon route switch endpoint and CLI route switch command.
- Route state stores active and last-known-good profile IDs.
- Local vLLM route readiness gate.

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
cargo run -p hermes-control-cli -- --api-token phase5-token model logs qwen36-mtp --reason "smoke"
wsl.exe -d Ubuntu-Hermes-Codex -u root --exec /opt/hermes-control/bin/hermes-control-vllm-health.sh qwen36-mtp 1 ready
```

Phase 6 route smoke:

```powershell
$env:HERMES_CONTROL_API_TOKEN = "phase6-token"
cargo run -p hermes-control-daemon
cargo run -p hermes-control-cli -- --api-token phase6-token route active
cargo run -p hermes-control-cli -- --api-token phase6-token route switch external.openai-compatible --dry-run --reason "smoke"
```

## Handoff Notes

- Keep `docs/RECENT_CHANGES.md` updated after each landed conversation-level
  change.
- Phase 6 should next implement Hermes provider config patch/reload and rollback
  around the already-persisted route state.
- Tauri belongs in Phase 8 as a GUI shell and typed daemon client only.
- Ask for explicit approval before commit and push.
