# AI Change Guide

This guide is for AI-assisted changes inside `E:\WSL\Hermres\hermes-control`.
It mirrors the handoff style used by the nearby AiVideoSRTGui project, but the
rules below are specific to the Hermes Rust control rewrite.

## Source Of Truth

- `AGENTS.md` is the operating guide for agent behavior, safety boundaries, and
  commit/push approval.
- `plan_rust_control_rewrite.md` is the phase plan and authority model.
- `config/*.toml` are the current runtime facts consumed by read-only status
  code.
- `crates/hermes-control-types/src/lib.rs` is the shared DTO and config contract.
- `docs/RECENT_CHANGES.md` records landed structural changes after each working
  conversation.

## Current Boundaries

- `hermes-control-daemon` will be the only state-mutating authority.
- `hermes-control-cli`, `hermes-control-bot`, and the future GUI are thin
  clients.
- Bot and GUI must call daemon API or named-pipe client code, not `wsl.exe`,
  PowerShell, shell scripts, or arbitrary subprocess APIs directly.
- External machine operations must be represented as typed Rust operations.
- Secrets stay behind `*_ref`, environment variables, or local secret stores.
  Do not commit raw provider keys, daemon bearer tokens, or Telegram tokens.
- Daemon bind defaults to `127.0.0.1`; LAN bind remains policy-controlled.
- The Hermes Control-owned vLLM runtime is `E:\WSL\Hermres\hermes-control\vLLM`.
  Treat `E:\WSL\vLLM\models` only as the external model-weight store.
- Tauri is planned for Phase 8 only. Before that, `hermes-control-gui` may hold
  boundary types or daemon-client helpers, not a real GUI authority surface.

## Change Pattern

1. Read the relevant files before editing.
2. Extend `hermes-control-types` first when an API/config/status contract changes.
3. Put config parsing, validation, read-only collection, and typed operation
   planning in `hermes-control-core`.
4. Keep CLI rendering in `hermes-control-cli`.
5. Keep Telegram parsing, allowlist checks, and daemon request mapping in
   `hermes-control-bot`.
6. Put daemon HTTP routes, state DB ownership, audit, confirmation, and operation
   locks in `hermes-control-daemon`.
7. Add tests at the crate boundary touched by the change.
8. Update `docs/RECENT_CHANGES.md` when a structural change lands.

## Safety Rules For New Features

- No raw shell endpoint.
- No broad "run command" abstraction exposed to clients.
- Destructive actions need dry-run summaries, confirmation policy, and audit
  records.
- Mutating operations must be serialized behind the daemon operation lock once
  Phase 3 starts.
- New CLI, bot, or GUI commands should start as daemon API requests, even if the
  daemon route is still a skeleton.
- Prefer redacted DTOs over passing raw local paths, tokens, or command lines to
  clients.

## Verification

Use the smallest useful check for the touched surface. For normal Rust changes:

```powershell
cd E:\WSL\Hermres\hermes-control
cargo fmt --all -- --check
cargo test --workspace
```

For build or dependency boundary changes, also run:

```powershell
cargo build --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

For docs-only changes, `git diff --check` is usually enough.

## Commit And Push

Do not commit or push automatically. If the change is worth preserving, suggest
a concise commit title and wait for explicit user approval before running
`git commit` or `git push`.

The current GitHub remote is:

```text
https://github.com/grshlogan/hermes-control
```

When the user approves a push, prefer the local proxy path already used on this
machine:

```powershell
git -c http.proxy=http://127.0.0.1:7890 -c https.proxy=http://127.0.0.1:7890 push origin main
```
