# AI Handoff

Hermes Control is now a Rust workspace with the Phase 1 skeleton and Phase 2
read-only core/CLI complete; Phase 3 should make the daemon the real state and
API authority.

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

- `a797a07 docs: clarify Tauri GUI adoption boundary`
- `1049326 feat: add read-only core and CLI status`
- `e9e957a docs: require approval before commit and push`
- `848f366 chore: initialize hermes control workspace`

## Next Phase

Phase 3 should focus on daemon API and SQLite state:

- Build the real Axum daemon routes for read-only endpoints first.
- Add bearer-token auth and localhost bind behavior.
- Add SQLite state and audit migrations.
- Define active route/profile state.
- Add operation lock, confirmation records, cancellation records, and audit
  append flow.
- Keep mutating WSL/vLLM/Hermes operations as planned or dry-run only until
  typed operation builders and tests exist.

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
- Keep Phase 3 daemon work narrow: API/state/confirmation first, no GUI-first
  detour.
- Tauri belongs in Phase 8 as a GUI shell and typed daemon client only.
- Ask for explicit approval before commit and push.
