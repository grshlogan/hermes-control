# Recent Changes

This document records landed structural changes after each working
conversation. Keep entries short, factual, and ordered by time. Do not record
unimplemented ideas here.

## 2026-05-02: Host Rust toolchain prepared

- Installed the Windows Rust build environment on this machine.
- Verified `rustc` and `cargo` were available.
- Verified a small Cargo hello-world build/run before creating the project.

## 2026-05-02: Rust control workspace initialized

- Created the `E:\WSL\Hermres\hermes-control` Rust workspace.
- Added crates for `types`, `core`, `daemon`, `cli`, `bot`, `gui`, and
  `testkit`.
- Added initial TOML config templates under `config/`.
- Added `deny.toml`, `.gitignore`, `.gitattributes`, and workspace dependency
  wiring.
- Added basic config schema and CLI help tests.
- Landed in commit `848f366 chore: initialize hermes control workspace`.

## 2026-05-02: Bot independent subprocess boundary added

- Added `hermes-control-bot` as a Windows-hosted Teloxide thin client.
- Added environment-driven bot config, Telegram user/chat allowlist checks, and
  command-to-daemon request planning.
- Added tests proving bot commands map to typed daemon API requests instead of
  raw shell/process execution.
- Added `docs/bot-process-boundary.md`.

## 2026-05-02: Project agent guide created

- Added `AGENTS.md` under `hermes-control`.
- Adapted only the reusable project-operation parts from local example agent
  docs.
- Documented repository boundaries, Rust rules, legacy handling, bot boundary,
  and Simplified Chinese response preference.
- Added the rule that commit/push requires explicit user approval.
- Approval rule landed in commit
  `e9e957a docs: require approval before commit and push`.

## 2026-05-02: Local git and private GitHub remote established

- Initialized the local git repository on branch `main`.
- Added remote `origin` at `https://github.com/grshlogan/hermes-control`.
- Pushed the first baseline to the private GitHub repository.
- Recorded the local proxy push path for later pushes.

## 2026-05-02: Phase 1 completed

- Workspace skeleton, crate boundaries, config templates, and first tests were
  in place.
- The bot process boundary existed early as part of the crate layout, but the
  bot remained a daemon client rather than an authority surface.
- Verification at the time included workspace fmt/test/build checks.

## 2026-05-02: Phase 2 read-only core and CLI completed

- Implemented `load_config_dir`, control config validation, provider config
  loading, and model runtime config loading in `hermes-control-core`.
- Added typed fixed WSL list command spec and parser for
  `wsl.exe --list --verbose`.
- Added Hermes health endpoint checks and vLLM `/v1/models` readiness parsing.
- Added read-only status DTOs in `hermes-control-types`.
- Added CLI commands for `status`, `health`, `providers`, `models`,
  `wsl status`, and `model status`, including `--json`.
- Last smoke snapshot reported WSL running, Hermes health unavailable, vLLM
  models not ready, and overall `Degraded`.
- Landed in commit `1049326 feat: add read-only core and CLI status`.

## 2026-05-02: Tauri GUI boundary clarified in the plan

- Updated `plan_rust_control_rewrite.md` to include Tauri v2 in Phase 8.
- Clarified that Tauri is a GUI shell and typed daemon client, not a machine
  authority.
- Documented that Tauri capabilities/permissions must be narrow and must not
  expose broad filesystem, shell, or process authority.
- Landed in commit `a797a07 docs: clarify Tauri GUI adoption boundary`.

## 2026-05-02: AI handoff documents added

- Added this `docs/RECENT_CHANGES.md` change log.
- Added `docs/AI_CHANGE_GUIDE.md` for AI/dev modification rules.
- Added `docs/AI_HANDOFF.md` with Phase 1/2 report and Phase 3 handoff.
- Added `docs/APP_CODE_MAP.md` with crate responsibility map and task routing.
