# App Code Map

This map explains where to work in the Hermes Control Rust workspace.

## Top Level

- `AGENTS.md`: project operating guide for AI agents and contributors.
- `plan_rust_control_rewrite.md`: rewrite plan, phase order, authority model,
  and Tauri boundary.
- `Cargo.toml`: workspace members and shared dependency policy.
- `config/control.toml`: daemon, WSL, Hermes health, log, and policy facts.
- `config/providers.toml`: AI provider and route-source facts.
- `config/model-runtimes.toml`: local vLLM runtime and variant facts.
- `docs/`: handoff notes, boundary docs, and change log.

## Crates

- `crates/hermes-control-types`
  - Shared DTOs, config structs, request structs, status structs, enums, and
    client/daemon contracts.
  - Change this first when a JSON/TOML/API shape changes.

- `crates/hermes-control-core`
  - Config parsing and validation.
  - Local read-only status collection.
  - WSL verbose-list parser and fixed `wsl.exe --list --verbose` command spec.
  - Phase 4 WSL/Hermes operation plan builders and dry-run command previews.
  - HTTP endpoint checks and vLLM `/v1/models` parsing.
  - Local log-tail helper.
  - Future home for executor abstractions shared by daemon and tests.

- `crates/hermes-control-daemon`
  - Axum daemon surface.
  - Authenticated read-only routes for status, health, providers, models, active
    route, and audit summaries.
  - SQLite state/audit initialization for active route, operation state,
    confirmations, and audit events.
  - WSL/Hermes action routes for dry-run previews and destructive-action
    confirmation records.
  - Still needs operation locking, confirmation/cancel execution flow, and real
    command executor behavior.

- `crates/hermes-control-cli`
  - Clap command definitions and CLI rendering.
  - Current commands are read-only and call core directly.
  - Later mutating commands should become daemon API calls.

- `crates/hermes-control-bot`
  - Windows-hosted Teloxide subprocess.
  - Environment-based config, allowlist checks, Telegram command parsing, daemon
    request planning, and daemon response formatting.
  - Must remain a thin daemon client.

- `crates/hermes-control-gui`
  - Future GUI boundary crate.
  - Currently only proves GUI channel and no raw process execution.
  - Real Tauri app belongs in Phase 8.

- `crates/hermes-control-testkit`
  - Shared test helpers and fixtures.
  - Currently has requester helpers; expand as daemon/core tests need fake WSL,
    fake vLLM, and fixture config builders.

## Tests

- `crates/hermes-control-core/tests/config_schema.rs`: config parse/validation
  contract.
- `crates/hermes-control-core/tests/read_only_core.rs`: WSL parser, vLLM model
  parsing, log tailing, and status behavior.
- `crates/hermes-control-cli/tests/help_contract.rs`: CLI help contract.
- `crates/hermes-control-cli/tests/read_only_commands.rs`: read-only CLI
  rendering behavior.
- `crates/hermes-control-bot/tests/bot_boundary.rs`: bot allowlist, command
  mapping, and no raw subprocess boundary.
- `crates/hermes-control-daemon/tests/phase3_api.rs`: daemon bearer auth,
  SQLite initialization, and read-only API route behavior.
- `crates/hermes-control-core/tests/phase4_operation_plans.rs`: WSL/Hermes typed
  operation planning and command-preview behavior.
- `crates/hermes-control-daemon/tests/phase4_actions.rs`: daemon dry-run action
  responses, confirmation records, and audit preview events.

## Where To Make Changes

- New config field: `types` first, then `core` parser/tests, then config file.
- New read-only status fact: `types` DTO, `core` collector, CLI renderer, daemon
  route once Phase 3 lands.
- New CLI command: `cli` parser/rendering, then daemon client path if mutating.
- New Telegram command: `bot` parser/planner tests, then daemon route contract.
- New daemon API: `types` request/response DTOs, `daemon` route, `core` behavior,
  integration tests.
- New WSL/vLLM/Hermes mutating operation: start with typed operation specs and
  dry-run summaries; daemon owns execution and audit.
- New WSL/Hermes executor behavior: write tests around the executor abstraction
  first, then connect it to confirmed daemon operations.
- GUI work: keep it as daemon-client GUI surface; do not give it machine-control
  authority.
