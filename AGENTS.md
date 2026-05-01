# Hermes Control Agent Guide

This file is the project-level operating guide for agents working inside
`E:\WSL\Hermres\hermes-control`.

It is distilled from the reusable parts of the local examples under
`E:\AI\Person\AGENTS.md`, then adapted to the Hermes Rust control rewrite.
Do not apply unrelated persona, CTF, daily reminder, or other-project rules
from those examples to this repository.

## Project Scope

Hermes Control Center is the Rust control tower for Hermes. It owns the local
control plane for:

- AI route/profile ownership.
- Windows-resident daemon, CLI, Telegram bot, and future GUI clients.
- WSL2 lifecycle control.
- Local vLLM model runtime management.
- Confirmation, audit, health, and state persistence.

`hermes-control-daemon` is the only state-mutating authority. Bot, CLI, and GUI
are thin clients.

## Current Layout

```text
E:\WSL\Hermres\
  plan_rust_control_rewrite.md       # controlling rewrite plan
  admin-controller\                  # legacy Python reference only
  switch\                            # legacy PowerShell reference only
  hermes-agent\                      # conversation runtime, separate repo
  hermes-control\                    # new Rust control workspace
    Cargo.toml
    AGENTS.md
    crates\
      hermes-control-types\
      hermes-control-bot\
    docs\
      bot-process-boundary.md
```

The intended final workspace will grow toward the layout described in
`plan_rust_control_rewrite.md`.

## Non-Negotiable Boundaries

- Do not import or call legacy Python/PowerShell control code from production
  Rust crates.
- Do not expose raw command execution endpoints.
- Do not add bot, GUI, or CLI code that directly mutates the machine.
- Every external process invocation must eventually live behind a typed Rust
  command builder in the control core.
- Every destructive action must have a dry-run summary and confirmation policy.
- Every mutating daemon operation must produce an audit event.
- Secrets must be referenced by `*_ref` or environment/config indirection; never
  store raw provider keys, bearer tokens, or Telegram tokens in committed files.
- Bind daemon APIs to `127.0.0.1` by default.

## Rust Development Rules

- Read files before editing.
- Prefer existing crate boundaries and local patterns.
- Keep shared API DTOs in `hermes-control-types`.
- Use `tracing` for runtime logging, not `println!`, except in deliberate CLI
  output paths.
- Modify `Cargo.toml` only when the dependency or crate boundary is needed.
- Prefer Rust native async traits where available; avoid adding async-trait style
  crates unless there is a clear reason.
- Keep logic general enough to be explained by the domain model. Arbitrary
  thresholds, magic strings, or special cases need explicit justification.
- Add abstractions only when they reduce real complexity or enforce a safety
  boundary.

## Editing Discipline

- Use targeted edits. Do not rewrite whole files when a narrow change is enough.
- Preserve user or unrelated local changes.
- Do not run destructive git or filesystem operations unless explicitly asked.
- Do not commit unless the user explicitly asks.
- Use descriptive plan/document names. Major plan files should be specific, such
  as `plan_rust_control_rewrite.md`, not generic names like
  `implementation_plan.md`.
- Treat plans as living documents; update the specific section that changed.

## Evidence-Based Work

- Verify files, flags, APIs, and configuration before claiming they exist.
- Prefer `rg` for search when available; if blocked, use PowerShell native
  commands.
- For code explanations, anchor claims to actual files and line references.
- Summarize decisive command output instead of pasting long logs.
- When runtime behavior and source comments disagree, trust observed runtime
  behavior first.

## Test And Verification Policy

Run the smallest useful verification after changes:

```powershell
cd E:\WSL\Hermres\hermes-control
cargo fmt --all -- --check
cargo test --workspace
```

For dependency or build boundary changes, also run:

```powershell
cargo build --workspace
```

For narrow bot changes, this is a useful fast path:

```powershell
cargo test -p hermes-control-bot --test bot_boundary
```

If logic changed, tests should cover the behavior. For new mutating operations,
write tests before implementation when feasible.

## Bot Boundary

`hermes-control-bot` is a Windows-hosted Telegram process. It must remain a thin
daemon client:

- Check Telegram user allowlist and optional chat allowlist before daemon calls.
- Parse commands into typed daemon API requests.
- Relay confirmation codes to daemon `/v1/confirm`.
- Persist no machine-authoritative state of its own.
- Never call `powershell`, `wsl.exe`, `.ps1`, shell scripts, or arbitrary
  subprocess APIs.

See `docs/bot-process-boundary.md`.

## Legacy Handling

`admin-controller` and `switch` are behavior references during the rewrite.
They are not production dependencies for new Rust crates.

Before adding compatibility with legacy behavior:

- Read the old code only to understand user-visible behavior.
- Re-express the behavior as typed Rust DTOs, command builders, daemon API
  handlers, or tests.
- Do not shell out to `switch.ps1`, `ops.ps1`, `model.ps1`, or
  `admin-controller`.

## Documentation Style

Keep project docs practical and handoff-friendly:

- Current state.
- Active boundaries.
- Key files.
- Startup/build commands.
- Logs/state locations.
- Verification checklist.
- Next step.

Avoid copying personality prompts or unrelated project details into this
workspace.

## Response Style

Use Simplified Chinese by default when talking to the user unless they request
English. Keep code identifiers, commands, paths, logs, and API names in their
original language.
