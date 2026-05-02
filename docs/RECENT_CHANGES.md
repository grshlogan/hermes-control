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

## 2026-05-02: Phase 3 daemon API and state foundation started

- Clarified in `plan_rust_control_rewrite.md` that Hermes Control is both the
  vLLM/MTP local runtime manager and the Hermes/WSL/provider route control
  tower.
- Added authenticated daemon routes for `/v1/status`, `/v1/health`,
  `/v1/providers`, `/v1/models`, `/v1/route/active`, and `/v1/audit`.
- Added bearer-token route protection using `Authorization: Bearer <token>`.
- Added SQLite state DB initialization for active route, operation state, and
  confirmations.
- Added SQLite audit DB initialization for audit event summaries.
- Added daemon Phase 3 tests covering auth, database initialization, providers,
  active route, and model routes.
- Mutating operation execution remains intentionally unimplemented.

## 2026-05-02: Phase advancement verification and Phase 4 typed ops started

- Added Phase advancement self-verification rules to `AGENTS.md`.
- Added `WslController` and `HermesRuntimeController` planning APIs in
  `hermes-control-core`.
- WSL restart/shutdown plans now produce fixed `wsl.exe` command previews and
  require confirmation.
- Hermes destructive plans now require confirmation and intentionally expose no
  raw shell command.
- Added daemon `/v1/wsl/action` and `/v1/hermes/action` routes.
- Dry-run action requests return typed operation previews.
- Non-dry-run destructive requests create confirmation records and append audit
  preview events.
- Added Phase 4 tests for core operation plans and daemon action routes.
- Real WSL/Hermes process execution remains intentionally unimplemented until an
  executor abstraction, operation lock, and confirmation lifecycle are tested.

## 2026-05-02: Phase 4 confirmation lifecycle and operation lock added

- Added daemon `/v1/confirm` and `/v1/cancel` routes.
- Destructive non-dry-run WSL/Hermes actions now create an `operation_state`
  row alongside the pending confirmation.
- A pending confirmation now locks out a second mutating action with HTTP
  conflict until the operation is confirmed or cancelled.
- Confirmation marks the pending confirmation and operation as `confirmed` and
  appends an audit event.
- Cancellation marks the pending confirmation and operation as `cancelled` and
  appends an audit event.
- Expanded Phase 4 daemon tests to cover lock rejection, confirmation release,
  and cancellation release.
- Real WSL/Hermes process execution remains intentionally unimplemented.

## 2026-05-02: Phase 4 injectable executor path added

- Added `OperationExecutor`, `ExecutableOperation`, and `ExecutionOutcome` to
  the daemon crate.
- Added `build_router_with_executor()` so tests and future service wiring can
  inject an executor implementation.
- Confirmation now loads the stored pending operation, passes it to the injected
  executor, and records the executor outcome in `operation_state`.
- `operation_state` now stores operation summary and command preview JSON so
  confirmed operations can be executed from persisted state.
- Added a no-op default executor that completes operations without running
  system commands.
- Expanded Phase 4 daemon tests with a recording executor to verify confirm-time
  dispatch and completed operation status.
- Real WSL/Hermes process execution remains intentionally unwired.

## 2026-05-02: Phase 4 allowlisted Windows executor wired

- Added `CommandRunner`, `CommandOutput`, `WindowsProcessRunner`, and
  `WindowsCommandExecutor` in the daemon crate.
- `WindowsCommandExecutor` validates every command preview before running
  anything and currently allows only fixed WSL shutdown, terminate, and wake
  probe command shapes.
- The daemon binary now starts with `WindowsCommandExecutor`; tests and library
  router defaults can still use fake or no-op executors.
- Added Phase 4 daemon tests proving allowed WSL command previews execute and
  non-allowlisted programs or WSL argument shapes are rejected without running.
- Hermes runtime process execution remains intentionally unimplemented until its
  typed command builders and failure handling are covered.

## 2026-05-02: Phase 4 execution failure reporting tightened

- Added optional `execution_status` to `ConfirmationLifecycleResponse`.
- `/v1/confirm` now returns the executor outcome status alongside the
  confirmation lifecycle status.
- Added a daemon test proving failed execution is visible to clients, is stored
  as a failed operation, and releases the mutating operation lock for later
  retries.

## 2026-05-02: Phase 4 Hermes fixed-script execution boundary added

- `HermesRuntimeController` can now be constructed with WSL distro/user facts
  and emits fixed WSL command previews for Hermes restart, stop, kill, wake, and
  health-check steps.
- The daemon `/v1/hermes/action` path now builds Hermes plans with configured
  WSL facts, so confirmed destructive Hermes operations can reach the executor.
- `WindowsCommandExecutor` allowlist now accepts only the fixed Hermes helper
  scripts under the configured WSL user's `$HOME/Hermres` path and rejects
  unknown script names.
- Added core and daemon Phase 4 tests covering Hermes restart dry-run previews,
  confirm-time command dispatch, and fixed-script allowlist behavior.

## 2026-05-02: Phase 4 real WSL/Hermes E2E validation

- Corrected `config/control.toml` to match this machine: WSL user `root` and
  Hermes health URL `http://127.0.0.1:8642/health`.
- Added immediate executor dispatch for non-confirming normal mutating actions
  such as WSL/Hermes wake, including operation state and audit events.
- Verified through a real daemon API smoke that WSL can stop, wake, and restart
  while the Windows daemon stays alive.
- Verified through daemon API that Hermes can restart, stop/kill, and wake; the
  final restored state had WSL running and Hermes health returning HTTP 200.
- Observed vLLM model endpoint `http://127.0.0.1:18080/v1/models` still
  unavailable, which remains Phase 5 scope.
- Observed Linux `service-status.sh` process checks can report false positives
  because `pgrep -af` matches the probe command itself; health endpoint checks
  were used as the reliable stop/start signal.

## 2026-05-02: Phase 4 WSL root helper contract added

- Switched the Phase 4 Hermes operation boundary from legacy `$HOME/Hermres`
  scripts to product-owned WSL root helpers under `/opt/hermes-control/bin`.
- Added `scripts/wsl-root/install.sh` plus start/stop/restart/kill/health/status
  helpers and `/etc/hermes-control/runtime.env` defaults for fresh installs.
- Tightened the Windows executor allowlist so Hermes helpers must run as WSL
  `root` and match the canonical helper filenames.
- Added tests covering canonical helper previews, allowlist acceptance, and WSL
  install asset presence.

## 2026-05-02: Phase 4 CLI daemon closeout

- Added CLI daemon API support for `hermes <wake|stop|restart|kill>`,
  `wsl <wake|stop|restart|shutdown-all>`, `confirm <code>`, and `cancel`.
- Added global `--daemon-url` and `--api-token` options, with
  `HERMES_CONTROL_API_TOKEN` fallback for mutating calls.
- Added HTTP-level CLI tests proving mutating commands post typed JSON with
  bearer auth to `/v1/hermes/action`, `/v1/wsl/action`, and `/v1/confirm`.
- Smoke-tested CLI -> daemon -> WSL root helper execution with a confirmed
  Hermes restart; execution completed and Hermes health returned ready on
  `http://127.0.0.1:8642/health`.

## 2026-05-02: Phase 5 vLLM action planning started

- Added `ModelRuntimeController` with typed vLLM start/stop/restart/health/logs
  /benchmark operation plans.
- Added daemon `/v1/models/{model_id}/action` and `/v1/models/{model_id}` route
  support for model-specific control.
- Added CLI `model <start|stop|restart|health|benchmark>` daemon calls.
- Added canonical WSL root vLLM helpers under `/opt/hermes-control/bin` and
  installed them into the current WSL distro.
- Corrected static vLLM start script facts to current existing scripts:
  `start-qwen36-mtp.sh` and `start-qwen36-int4-eager.sh`.

## 2026-05-03: Phase 5 vLLM self-deployment requirement added

- Updated the plan to require future Hermes Control support for vLLM
  self-deployment/provisioning, not only adoption of an existing runtime.
- Defined adopt-existing, fresh-install, and repair-install modes.
- Documented install-test safety: runtime files may be recreated, but
  `E:\WSL\vLLM\models` must be preserved by default.
- Added network policy: prefer direct connectivity and use configured fallback
  proxy only after direct install/download attempts fail.

## 2026-05-03: Phase 5 project-owned vLLM runtime scaffold

- Clarified that `E:\WSL\vLLM\models` is only the model-weight store.
- Moved the software-owned vLLM runtime boundary to
  `E:\WSL\Hermres\hermes-control\vLLM`.
- Added project runtime scripts for env setup, bootstrap/repair install,
  OpenAI-compatible serve, qwen36 MTP start, and qwen36 AWQ INT4 eager start.
- Kept vLLM socket/temp defaults on WSL `/tmp` for DrvFS compatibility while
  keeping venv/cache/logs/scripts under the project-owned runtime.
- Made pip cache fall back to WSL `/tmp` when DrvFS ownership makes pip refuse
  the project cache directory.
- Updated `config/model-runtimes.toml` and WSL helper defaults so vLLM start
  scripts and logs point at the project-owned runtime.
- Made `scripts/wsl-root/install.sh` migrate old `VLLM_*` values in
  `/etc/hermes-control/runtime.env` instead of leaving stale old workspace
  paths behind.
- Added typed `ModelAction::Install`, CLI/Bot `model install <model-id>`, and a
  WSL root bootstrap helper so the daemon can trigger project-owned vLLM repair
  through the same allowlisted execution path.
- Added tests that protect the project-owned runtime path while preserving the
  external model store.
- Verified the bootstrap helper on this WSL distro. It created
  `E:\WSL\Hermres\hermes-control\vLLM\.venv` and installed vLLM 0.20.0 with
  Torch 2.11.0. The model endpoint remains not ready until a model is started.

## 2026-05-03: Phase 5 qwen36 MTP live validation

- Started `qwen36-mtp` from the project-owned vLLM runtime and verified
  `/v1/models` plus `/v1/chat/completions` with an `OK` response.
- Confirmed vLLM 0.20.0 loaded `Qwen3_5ForConditionalGeneration` with
  `Qwen3_5MTP`, `SpeculativeConfig(method='mtp', num_spec_tokens=2)`, tensor
  parallel size 2, and model weights from `E:\WSL\vLLM\models`.
- Found this WSL/vLLM server is callable on the WSL primary IP, not reliably on
  `127.0.0.1`; updated fixed start scripts and WSL helpers to resolve the WSL
  primary IP at runtime while preserving explicit overrides.
- Fixed `hermes-control-vllm-health.sh`: the previous heredoc-based parser
  discarded the `/v1/models` response body, so health stayed false even while
  vLLM returned HTTP 200.
- Updated this machine's Hermes `custom:vllm` provider to
  `http://10.2.176.55:18080/v1`, extended Hermes `NO_PROXY`, restarted Hermes,
  and verified Hermes gateway returned `OK` through the local model.
- Verified Open WebUI can call the local model through its Hermes gateway
  backend: `/openai/models` exposed `hermes-agent` and
  `/openai/chat/completions` returned `OK`.

## 2026-05-03: WSL2/Hermes provisioning plan and Chinese README

- Added root-level `plan_wsl2_hermes_provisioning.md` to separate WSL2,
  Hermes, Open WebUI, and vLLM provisioning from the main Rust rewrite plan.
- Defined Adopt Existing, Repair Install, and Fresh Install provisioning modes.
- Documented root helper, filesystem, vLLM, Hermes, Open WebUI, backup,
  validation, and completion contracts for future installer/assistant work.
- Added a Chinese root `README.md` covering project purpose, current status,
  directory map, WSL helper commands, vLLM commands, safety principles, and git
  workflow.
- Added a cross-reference from the main Rust rewrite plan to the new
  provisioning plan.

## 2026-05-03: Phase 5 closeout and Phase 6 route switch start

- Changed CLI `model logs <model-id>` from placeholder text to a typed daemon
  `ModelAction::Logs` request, with helper stdout rendered back to the operator.
- Added optional `OperationResponse.output` and daemon executor stdout capture
  so read-only helper output can flow through daemon clients.
- Added canonical WSL root helper
  `hermes-control-vllm-start-with-fallback.sh <primary> <fallback>`.
- Updated MTP start/restart plans to use the fallback helper when a stable
  same-runtime fallback variant such as `qwen36-awq-int4` exists.
- Started Phase 6 with daemon `POST /v1/route/switch`, active route persistence,
  last-known-good route tracking, audit records, CLI `route switch`, and local
  vLLM readiness gating.
- Route switching is currently state-only; Hermes/Open WebUI config patching,
  hot reload/restart, and rollback remain later Phase 6 work.
