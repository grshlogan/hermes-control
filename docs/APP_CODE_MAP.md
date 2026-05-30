# App Code Map

This map explains where to work in the Hermes Control Rust workspace.

## Top Level

- `AGENTS.md`: project operating guide for AI agents and contributors.
- `start-hermes-control.ps1`: root Windows convenience launcher for the local
  daemon and GUI. It writes PID files under `state/`, daemon stdout/stderr under
  `logs/daemon/`, and GUI/Vite stdout/stderr under `logs/local-run/`.
- `stop-hermes-control.ps1`: root Windows convenience stopper for the local
  daemon and GUI. It only stops WSL, Hermes, or vLLM when explicit switches are
  passed.
- `plan_rust_control_rewrite.md`: rewrite plan, phase order, authority model,
  and Tauri boundary.
- `Cargo.toml`: workspace members and shared dependency policy.
- `apps/hermes-control-gui`: Tauri v2 + React/TypeScript desktop app scaffold.
  `src-tauri` is excluded from the root workspace so the normal Rust workspace
  loop remains fast; validate it with its own manifest path.
- `config/control.toml`: daemon, WSL, Hermes health, log, and policy facts.
  Current machine facts: WSL default user is `root`; Hermes health is
  `http://127.0.0.1:8642/health`.
- `config/providers.toml`: AI provider and route-source facts. The first
  third-party Anthropic/Claude relay entry is `external.api-relay`; its URL,
  model list, and secret ref are config facts, while raw API keys remain outside
  the repo and daemon payloads. Provider work should treat a remote API as a
  provider site plus account binding plus route/model choice; a token belongs to
  the account binding and must be referenced by env key or secret ref only once.
- `config/model-runtimes.toml`: local vLLM runtime and variant facts.
  Current machine facts: qwen36 MTP and AWQ variants run through WSL distro
  `Ubuntu-Hermes-Codex` and expose `/v1/models` on
  `http://127.0.0.1:18080`.
- `docs/`: handoff notes, boundary docs, and change log.
- `vLLM/`: Hermes Control-owned vLLM runtime scaffold. Scripts are tracked;
  `.venv`, cache, logs, downloads, and accidental local models are ignored.
  Default model weights live inside the WSL2 rootfs at `/root/Hermres/models`
  to avoid `/mnt/e` 9P checkpoint loading overhead. The old
  `E:\WSL\vLLM\models` path is only a migration source or backup.
  vLLM socket/temp files default to WSL `/tmp` for DrvFS compatibility; pip
  cache may also fall back there when DrvFS ownership is incompatible with pip.

## Crates

- `crates/hermes-control-types`
  - Shared DTOs, config structs, request structs, status structs, enums, and
    client/daemon contracts.
  - `ConfirmationLifecycleResponse.execution_status` reports confirmed
    operation execution outcome when confirmation also triggers execution.
  - `OperationResponse.output` carries optional captured helper stdout for
    read-only operator output such as model logs.
  - `ActiveRouteStatus` carries active and last-known-good route profile IDs.
  - Change this first when a JSON/TOML/API shape changes.

- `crates/hermes-control-core`
  - Config parsing and validation.
  - Local read-only status collection.
  - WSL verbose-list parser and fixed `wsl.exe --list --verbose` command spec.
  - Phase 4 WSL/Hermes operation plan builders and dry-run command previews.
  - Phase 5 vLLM model runtime operation planner for canonical
    `/opt/hermes-control/bin/hermes-control-vllm-*.sh` helpers.
  - MTP model start/restart planning can use the canonical
    `hermes-control-vllm-start-with-fallback.sh` helper when a stable fallback
    variant exists in the same runtime.
  - Hermes WSL root helper previews for `/opt/hermes-control/bin`
    `hermes-control-start.sh`, `hermes-control-stop.sh`,
    `hermes-control-restart.sh`, `hermes-control-kill.sh`, and
    `hermes-control-health.sh 30 ready`.
  - HTTP endpoint checks and vLLM `/v1/models` parsing.
  - Local log-tail helper.
  - Future home for executor abstractions shared by daemon and tests.

- `crates/hermes-control-daemon`
  - Axum daemon surface.
  - Authenticated read-only routes for status, health, providers, models,
    individual model status, active route, daemon/bot/Hermes/vLLM log tails, and
    audit summaries.
  - SQLite state/audit initialization for active route, operation state,
    confirmations, and audit events.
  - WSL/Hermes action routes for dry-run previews and destructive-action
    confirmation records.
  - Initial model action route:
    `/v1/models/{model_id}/action` for typed vLLM start/stop/restart/health/logs
    /benchmark plans.
  - Phase 6 route switch route: `/v1/route/switch` validates provider IDs,
    stores active/last-known-good route state, audits the switch, and gates
    local vLLM profiles on readiness.
  - Phase 6 rollback route: `/v1/route/rollback` reads last-known-good state and
    replays that profile through the same helper before updating active route.
  - Route switch execution now plans the fixed WSL root
    `hermes-control-route-apply.sh` helper and writes active route state only
    after the helper succeeds.
  - Provider `api_key_ref` values are resolved only to controlled Hermes env key
    names for route application; raw secret values stay out of daemon requests,
    responses, command previews, and audit rows.
  - Confirmation/cancel endpoints and pending operation lock.
  - Confirm responses expose executor outcome status, while failed outcomes are
    stored in operation state and release the lock.
  - Normal mutating actions that do not require confirmation execute
    immediately through the injected executor and still create operation state
    plus audit events.
  - Injectable `OperationExecutor`; `build_router()` defaults to no-op for safe
    library/test usage.
  - Daemon binary wires `WindowsCommandExecutor`, which executes only
    allowlisted WSL command-preview shapes after confirmation.
  - `WindowsCommandExecutor` captures successful command stdout and exposes it
    through `OperationResponse.output`.
  - Hermes destructive operations and wake operations now reach the executor
    through fixed WSL script previews.
  - HTTP request/response tracing is enabled at info level so the daemon log
    target has visible runtime activity.

- `scripts/wsl-root`
  - Product-owned WSL root helper package.
  - `install.sh` installs helpers to `/opt/hermes-control/bin` and creates
    `/etc/hermes-control/runtime.env`.
  - Helpers start, stop, restart, kill, health-check, and status-check the
    Hermes gateway without relying on legacy `/root/Hermres/*.sh` scripts.
  - `hermes-control-route-apply.sh` atomically patches non-secret Hermes route
    env keys, restarts Hermes, health-checks it, and restores the previous env
    file if the apply step fails.
  - The route helper validates the referenced secret env key inside WSL/Hermes
    env scope and copies it to the provider-family runtime key locally.
  - `hermes-control-openwebui-sync.sh` backs up Open WebUI `webui.db` and points
    its OpenAI backend/default model at Hermes gateway without printing API
    keys.
  - `hermes-control-openwebui-status.sh` and
    `hermes-control-openwebui-refresh.sh` inspect Open WebUI process state and
    restart it with Hermes gateway env only when it is already running.
  - Route apply restores the Open WebUI DB backup and previous Hermes env if
    Open WebUI refresh fails after sync.
  - vLLM helpers start/stop/health/log/benchmark fixed model runtime operations
    through the same root-side package. Benchmark is a reserved helper.
  - `hermes-control-vllm-start-with-fallback.sh` tries a primary MTP variant and
    falls back to a stable AWQ variant if the primary does not become healthy.
  - `hermes-control-vllm-bootstrap.sh` runs the project-owned vLLM bootstrap
    script for daemon-triggered install/repair.
  - `install.sh` refreshes `VLLM_*` defaults in `/etc/hermes-control/runtime.env`
    so stale old workspace paths migrate to the project-owned `vLLM/` runtime.

- `vLLM/scripts`
  - `env.sh`: project runtime environment, cache/log/temp defaults, external
    model store, and direct-first/fallback-proxy network policy.
  - `bootstrap.sh`: creates or repairs the project-owned Python venv and installs
    vLLM.
  - `serve-openai.sh`: shared OpenAI-compatible vLLM launcher.
  - `start-qwen36-mtp.sh` and `start-qwen36-int4-eager.sh`: fixed variant entry
    scripts consumed by WSL root helpers.

- `crates/hermes-control-cli`
  - Clap command definitions and CLI rendering.
  - Read-only status/providers/models commands still call core directly.
  - Phase 4 mutating commands call daemon APIs with bearer auth:
    `hermes <wake|stop|restart|kill>`,
    `wsl <wake|stop|restart|shutdown-all>`, `confirm <code>`, and `cancel`.
  - Phase 5 `model <install|start|stop|restart|health|logs|benchmark>` commands
    call daemon model action APIs.
  - Phase 6 `route active` reads daemon route state,
    `route switch <profile-id>` posts typed route switch requests, and
    `route rollback` posts typed last-known-good rollback requests.

- `crates/hermes-control-bot`
  - Windows-hosted Teloxide subprocess.
  - Environment-based config, allowlist checks, Teloxide `HermesBotCommand`
    parsing, daemon request planning, and daemon response formatting.
  - `/model install <model-id>` maps to typed `ModelAction::Install`.
  - `BotStateStore` persists Telegram long-polling offset in local SQLite
    `telegram_state` so bot restarts do not replay old updates.
  - `BotEventLog` appends redacted runtime events to `logs/bot/bot.log`, which
    is tailed by daemon `/v1/logs/bot`.
  - Runtime loop logs and retries Telegram polling failures, and logs
    message-send failures before continuing.
  - Must remain a thin daemon client.

- `crates/hermes-control-gui`
  - GUI Rust boundary crate shared by the Tauri app.
  - Provides `GuiConfig`, redacted `GuiConnectionSummary`, `GuiDaemonClient`,
    `GuiDashboardSnapshot`, GUI requester helpers, route switch/rollback
    preview and execute request builders, model action preview and execute
    request builders, daemon confirm/cancel request builders, WSL/Hermes
    runtime action preview and execute request builders, safe log tail target
    helpers, and the Tauri capability contract.
  - Must stay a daemon client; no raw WSL/Hermes/vLLM process control belongs
    here.

- `apps/hermes-control-gui`
  - Tauri v2 app shell in `src-tauri`.
  - React/TypeScript front end in `src`.
  - `src-tauri/capabilities/default.json` grants only `core:default`; do not
    add `shell:`, `fs:`, or process permissions without a new security review.
  - Current front end is an operations dashboard with Dashboard, AI Route, Local
    Models, Runtime, Logs, Audit, and Settings surfaces.
  - Route and Logs surfaces currently support daemon dry-run previews,
    switch/rollback execution, daemon confirmation confirm/cancel, and
    daemon-owned log tailing.
  - Local Models supports selected model/action preview and run controls for
    install/start/stop/restart/health/logs/benchmark.
  - Local Models reads `model_root` from daemon model summaries and displays
    the active WSL-native model directory; the GUI must not hard-code model
    storage paths.
  - Runtime supports WSL and Hermes action preview/run controls through daemon
    APIs and the shared confirmation sheet.
  - Logs supports daemon-owned target selection, bounded tail size, and
    client-side loaded-line filtering.
  - Audit supports client-side risk/requester/query filtering over the daemon
    audit summary.
  - Settings supports browser-preview daemon URL/token/operator localStorage,
    Tauri desktop environment summaries, redacted token status, and daemon
    connection testing.
  - `src/lib/i18n.ts` provides the Chinese-first translation dictionary,
    language normalization, and English fallback option. UI labels are
    localized, but typed daemon action IDs remain unchanged.

- `crates/hermes-control-testkit`
  - Shared test helpers and fixtures.
  - Currently has requester helpers; expand as daemon/core tests need fake WSL,
    fake vLLM, and fixture config builders.

## Tests

- `crates/hermes-control-core/tests/config_schema.rs`: config parse/validation
  contract.
- `crates/hermes-control-core/tests/phase4_wsl_install_assets.rs`: WSL root
  helper install asset contract.
- `crates/hermes-control-core/tests/phase5_model_runtime_plans.rs`: vLLM model
  runtime operation-plan contract.
- `crates/hermes-control-core/tests/phase5_vllm_project_runtime_assets.rs`:
  project-owned vLLM runtime path, script assets, and external model-store
  contract, plus WSL-primary-IP endpoint resolution and vLLM health response
  parsing. It also locks the WSL health-helper endpoint override contract so
  daemon-selected model endpoints survive `/etc/hermes-control/runtime.env`.
- `crates/hermes-control-core/tests/read_only_core.rs`: WSL parser, vLLM model
  parsing, WSL helper health command construction, log tailing, and status
  behavior.
- `crates/hermes-control-cli/tests/help_contract.rs`: CLI help contract.
- `crates/hermes-control-cli/tests/read_only_commands.rs`: read-only CLI
  rendering behavior.
- `crates/hermes-control-cli/tests/daemon_commands.rs`: CLI mutating daemon API
  request contract, including model logs and route switch.
- `crates/hermes-control-bot/tests/bot_boundary.rs`: bot allowlist, Teloxide
  command enum parsing, command mapping, offset persistence, redacted bot event
  logs, runtime config parsing, and no raw subprocess boundary.
- `crates/hermes-control-gui/tests/phase8_gui_boundary.rs`: GUI daemon-client
  config, GUI requester shape, Tauri capability safety, and no raw
  shell/filesystem/process boundary, plus route switch/rollback execution,
  model action execution, WSL/Hermes runtime action execution, confirmation
  lifecycle, safe log target contracts including `vllm`, and redacted
  connection summary behavior.
- `apps/hermes-control-gui/src/lib/viewModel.test.ts`: front-end dashboard view
  model, route option flags, log target bounds, control-surface navigation, and
  daemon confirmation prompt rendering, plus Local Models and Runtime action
  options, Settings connection summary rendering, log filtering, and audit
  filtering.
- `apps/hermes-control-gui/src/lib/i18n.test.ts`: Chinese default language,
  English language option, language normalization, and localized action option
  rendering without changing typed action IDs.
- `crates/hermes-control-daemon/tests/phase3_api.rs`: daemon bearer auth,
  local GUI CORS preflight, SQLite initialization, read-only API route behavior,
  and daemon log tailing.
- `crates/hermes-control-core/tests/phase4_operation_plans.rs`: WSL/Hermes typed
  operation planning, fixed WSL command previews, and Hermes script preview
  behavior.
- `crates/hermes-control-daemon/tests/phase4_actions.rs`: daemon dry-run action
  responses, confirmation records, audit preview events, confirm/cancel, and
  operation-lock release behavior, plus injected executor dispatch after
  confirmation, failed execution outcome reporting, Hermes fixed-script
  previews, initial vLLM action previews, immediate execution for normal
  mutating actions, Windows command allowlist enforcement, and stdout capture.
- `crates/hermes-control-daemon/tests/phase6_route_switch.rs`: Phase 6 route
  switch/rollback command previews, state, last-known-good tracking, audit,
  local vLLM readiness gate, and "do not update active route after apply
  failure" behavior.
- `tests/wsl-root/openwebui_sync.sh`: WSL helper smoke test for Open WebUI
  persistent config sync and secret redaction.
- `tests/wsl-root/openwebui_refresh.sh`: WSL helper smoke test for Open WebUI
  if-running refresh, env handoff, and secret redaction.
- `tests/wsl-root/route_apply_openwebui_rollback.sh`: WSL helper smoke test for
  route apply failure recovery after Open WebUI refresh errors.

## Where To Make Changes

- New config field: `types` first, then `core` parser/tests, then config file.
- New provider import field: add the typed config/import DTO, add a parser or
  normalization test first, then wire daemon/helper/GUI behavior. JSON import
  must reject raw API key values and preserve only env keys or secret refs.
- New read-only status fact: `types` DTO, `core` collector, CLI renderer, daemon
  route once Phase 3 lands.
- New CLI command: `cli` parser/rendering, then daemon client path if mutating.
- New Telegram command: `bot` parser/planner tests, then daemon route contract.
- New daemon API: `types` request/response DTOs, `daemon` route, `core` behavior,
  integration tests.
- New WSL/vLLM/Hermes mutating operation: start with typed operation specs and
  dry-run summaries; daemon owns execution and audit.
- New WSL/Hermes executor behavior: write tests around the executor abstraction
  first, then connect it to confirmed daemon operations. Real process execution
  must pass through the daemon allowlist.
- GUI work: keep it as daemon-client GUI surface; do not give it machine-control
  authority.
