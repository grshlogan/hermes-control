# AI Handoff

Hermes Control is now a Rust workspace with Phase 1 and Phase 2 complete. Phase
3 has authenticated read-only daemon API and SQLite state/audit foundations;
Phase 4 has typed WSL/Hermes operation planning, dry-run previews,
destructive-action confirmation records, confirm/cancel endpoints, and a pending
operation lock. Confirmed operations now flow through an injectable executor
abstraction. The daemon binary wires an allowlisted Windows command executor for
confirmed operations; library/test router defaults can still use no-op or fake
executors.

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

Phase 4 has started:

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
- Hermes restart/stop/kill plans now produce fixed WSL script previews under
  the configured WSL user's `$HOME/Hermres`: `restart-services.sh`,
  `stop-services.sh`, `kill-stuck-services.sh`, and `health-check.sh 30 ready`.
- Confirmed Hermes destructive operations can now flow to the Windows executor
  through that fixed script allowlist. This requires the Linux-side helper
  scripts to exist for the configured WSL user.
- The daemon binary uses `WindowsCommandExecutor`; tests use fake or no-op
  executors.
- Failed executor outcomes are stored as `failed` operation state and release
  the pending operation lock for a later retry.
- Non-confirming wake actions still need an immediate execution path; today they
  return a typed plan rather than running directly.

## Current Runtime Observation

The last CLI smoke check observed:

- WSL distro `Ubuntu-Hermes-Codex` was running as WSL2.
- Hermes health at `http://127.0.0.1:18000/health` was unavailable.
- Configured vLLM model endpoints were not ready.
- Overall status was `Degraded`.

Treat this as a snapshot, not a permanent fact; rerun CLI status before making
runtime claims.

## Current Commit Baseline

Latest pushed commits:

- `44a6eaa feat: expose execution status on confirm`
- `660a210 feat: wire allowlisted Windows executor`
- `87ac3dc feat: add injectable operation executor`
- `c865ed5 feat: add confirmation lifecycle and operation lock`
- `3e4f2f1 feat: add phase4 typed operation previews`

## Current Phase

Phase 4 remaining work should stay focused on safe execution after the typed
planning layer:

- Extend real execution beyond the current WSL allowlist only through typed
  builders and focused tests.
- Add immediate execution/state handling for normal mutating wake actions.
- Keep any further WSL/Hermes real execution behind typed builders, audit,
  confirmation when required, and the operation lock.
- Move CLI mutating commands to daemon API calls after executor behavior is
  covered.

## Useful Commands

```powershell
cd E:\WSL\Hermres\hermes-control
cargo fmt --all -- --check
cargo test --workspace
cargo build --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

Read-only CLI smoke:

```powershell
cargo run -p hermes-control-cli -- status
cargo run -p hermes-control-cli -- status --json
cargo run -p hermes-control-cli -- providers
cargo run -p hermes-control-cli -- models
```

## Handoff Notes

- Keep `docs/RECENT_CHANGES.md` updated after each landed conversation-level
  change.
- Keep Phase 4 narrow: typed WSL/Hermes ops first, no vLLM Phase 5 work yet.
- Tauri belongs in Phase 8 as a GUI shell and typed daemon client only.
- Ask for explicit approval before commit and push.
