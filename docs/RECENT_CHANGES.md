# Recent Changes

This document records landed structural changes after each working
conversation. Keep entries short, factual, and ordered by time. Do not record
unimplemented ideas here.

## 2026-05-31: Add DeepSeek official route apply coverage

- Added daemon route-switch dry-run coverage for `deepseek.official`, using
  `https://api.deepseek.com/v1`, `deepseek-chat`, and `DEEPSEEK_API_KEY`.
- Added a WSL helper sandbox test proving DeepSeek account env keys are copied
  to `LM_API_KEY` for Hermes runtime without printing raw token values.

## 2026-05-30: Harden native provider JSON import previews

- Extended native `{"providers":[...]}` JSON imports to normalize provider
  account secret references before previewing them.
- Kept account-pool imports reusable by accepting `env:...` and
  `secret_ref:...` references while rejecting raw token-looking `secret_ref`
  values.
- Extended env-style provider JSON imports beyond Claude relays to
  OpenAI-compatible, DeepSeek, Codex, and LM Studio families with provider
  account bindings.
- Added a WSL helper sandbox test for OpenAI-compatible route apply so account
  env keys are copied to `LM_API_KEY` without printing the token.
- Added daemon preview coverage so raw account secrets in native provider JSON
  return `400 Bad Request` instead of reaching the GUI preview payload.

## 2026-05-30: Add provider JSON import preview through daemon and GUI

- Added daemon `/v1/providers/import/preview` as a dry-run-only endpoint backed
  by the existing Claude/Anthropic relay JSON normalizer.
- Added GUI/Tauri client wiring and an AI Route page textarea so operators can
  preview provider/account JSON drafts without writing config.
- Kept the import preview non-sensitive: the UI displays provider type, base
  URL, default model, secret env key, and runtime env key names, not raw keys or
  stored secret refs.

## 2026-05-30: Add provider account bindings and Claude relay import schema

- Extended provider config with account bindings, default account/model fields,
  Claude default model aliases, and non-sensitive runtime env hints such as
  `API_TIMEOUT_MS`, proxy keys, and `effortLevel`.
- Added a Claude/Anthropic relay JSON importer that accepts env/secret
  references like `$env:ANTHROPIC_AUTH_TOKEN` and rejects raw API key values.
- Updated route switch previews so Claude relay helpers receive non-sensitive
  env patch keys while raw provider tokens remain referenced only through
  `secret_env_key`.
- Updated route apply to preserve separate Claude Sonnet/Haiku/Opus defaults
  and relay runtime env hints when applying the Hermes env patch.
- Extended CLI and GUI route/provider summaries to show provider type, default
  model, account binding, secret env key, and runtime env keys without exposing
  raw secrets.

## 2026-05-30: Move GUI info card and add Open WebUI runtime control

- Moved the always-visible GUI boundary card into a dedicated sidebar
  `Info`/`信息` page so routine control pages keep their full workspace width.
- Added Open WebUI as a third Runtime control group beside WSL and Hermes, with
  typed `Wake`, `Stop`, `Restart`, and `Status` actions.
- Added daemon `/v1/openwebui/action` and `/v1/openwebui/status` endpoints backed
  by fixed WSL root helper previews and execution allowlist checks.
- Added `hermes-control-openwebui-stop.sh` and locked it into the WSL helper
  install asset contract.
- Kept the Tauri GUI thin-client boundary intact: no shell, filesystem, or raw
  process permissions were added.

## 2026-05-19: Remove tuned vLLM preset and add API relay provider

- Removed the temporary `qwen36-mtp-tuned` preset from runtime config and WSL
  helper registration, leaving only the default MTP and AWQ INT4 variants.
- Made the WSL helper installer remove the obsolete
  `VLLM_START_QWEN36_MTP_TUNED` runtime env key during refresh.
- Added `external.api-relay` as the first Anthropic/Claude third-party relay
  provider in `config/providers.toml`, matching relay configs that use
  `ANTHROPIC_BASE_URL`, `ANTHROPIC_AUTH_TOKEN`, and Claude model env names.
- Kept raw relay API keys outside the repo and daemon payloads; route apply
  still resolves the provider secret ref to the controlled
  `ANTHROPIC_AUTH_TOKEN` env key inside WSL/Hermes scope.

## 2026-05-12: Add qwen36 MTP tuned vLLM profile

- Added an isolated `qwen36-mtp-tuned` vLLM variant and
  `vLLM/scripts/start-qwen36-mtp-tuned.sh` as an experimental launch profile.
- Kept the default `qwen36-mtp` startup path unchanged.
- Tuned profile borrows low-risk runtime flags only: spawn workers, conservative
  NCCL settings, prefix caching, and chunked prefill, while retaining TP=2,
  80000 max context, and 0.80 GPU memory utilization defaults.
- The tuned profile prefers GPU order `0,1` so the RTX 5080 can be the primary
  CUDA rank when display output is moved to the RTX 3080.
- Updated WSL root helper install/runtime mappings so daemon model actions can
  start and stop the tuned variant through the product-owned helper boundary.

## 2026-05-10: Clear stale daemon operation locks on restart

- Fixed a stale operation-lock bug where a daemon restart during a long-running
  model start could leave `operation_state.status = running` forever, causing
  GUI/Web model Stop requests to return HTTP 409.
- Daemon startup now marks inherited `running` operations as `failed` and writes
  an audit recovery record, while keeping pending confirmations locked until an
  operator confirms or cancels them.
- Improved GUI/Web 409 messages for model/runtime operations so operators see
  that a pending or running operation must be confirmed, cancelled, or retried.

## 2026-05-10: Move qwen36 weights into WSL2 native storage

- Copied `Qwen3.6-27B-AWQ-INT4` from the old Windows-backed
  `E:\WSL\vLLM\models` store into `/root/Hermres/models` inside
  `Ubuntu-Hermes-Codex`.
- Changed vLLM runtime defaults and WSL root helper install defaults to use
  `/root/Hermres/models` as `VLLM_MODEL_ROOT`.
- Added `model_root` to the daemon model-runtime summary and Local Models GUI
  cards so operators can see the active WSL-native model directory from the UI.
- Preserved the old Windows-backed model store as a migration source/backup;
  no model weights were deleted.
- Updated Phase 5 asset tests to lock the WSL2-native model root contract.

## 2026-05-07: Phase 8 local vLLM route readiness gate fixed

- Fixed the local-vLLM route switch gate so daemon readiness checks can use the
  WSL root helper when Windows cannot reach the vLLM WSL IP directly.
- Added a daemon-provided vLLM models endpoint override for the WSL health
  helper, and made `hermes-control-common.sh` preserve that override after
  sourcing `/etc/hermes-control/runtime.env`.
- Refreshed the installed WSL helpers under `/opt/hermes-control/bin`.
- Verified live `qwen36-mtp` readiness through `GET /v1/models` and confirmed
  `POST /v1/route/switch` completed for `local.vllm.qwen36-mtp`.
- Added regression tests proving bad local endpoints are still rejected and
  cannot accidentally borrow the machine's currently running vLLM instance.

## 2026-05-07: Root start and stop scripts

- Added `start-hermes-control.ps1` at the project root to start the Windows
  daemon and Web/Tauri GUI with local PID files and logs.
- Added `stop-hermes-control.ps1` at the project root to stop only the local
  hermes-control daemon/GUI by default, with explicit switches for vLLM, Hermes,
  or full WSL shutdown.
- Documented the root scripts in `README.md` and `docs/APP_CODE_MAP.md`.

## 2026-05-07: Phase 8 model startup feedback clarified

- Reproduced the Local Models `Start qwen36-mtp` path while vLLM/MTP was
  actually loading in WSL.
- Replaced the terse `Submitting start` status with a localized long-running
  startup message that names the model and explains that vLLM/MTP loading can
  take several minutes.
- Added a view-model regression test for Start/Restart progress copy.

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

## 2026-05-05: Phase 6 Hermes route apply boundary

- Added WSL root helper `hermes-control-route-apply.sh` for fixed Hermes route
  profile application.
- Route switch dry-runs now show the exact fixed `wsl.exe --exec
  /opt/hermes-control/bin/hermes-control-route-apply.sh ...` command preview.
- Non-dry-run route switches now execute the route-apply operation first and
  persist active route state only after the helper succeeds.
- The route helper writes only non-secret route env keys, restarts Hermes,
  checks health, and restores the previous Hermes env file on restart/health
  failure.
- The Windows command executor allowlist now accepts only this fixed route
  apply helper shape with constrained profile/provider/base-url/model args.

## 2026-05-05: Phase 6 provider secret-ref route boundary

- Extended route switch planning so `api_key_ref` is resolved to a controlled
  Hermes-side environment variable name, not to the raw secret value.
- Route apply command previews now include only the secret env key name, such as
  `LM_API_KEY`, or `none` for local vLLM profiles.
- Updated `hermes-control-route-apply.sh` to validate that the named secret env
  key exists inside the WSL/Hermes env scope before restarting Hermes.
- The helper copies the secret value only inside WSL into the Hermes runtime env
  variable expected by the selected provider family; the daemon never receives
  or logs the raw key.

## 2026-05-05: Phase 6 Open WebUI route sync boundary

- Added `hermes-control-openwebui-sync.sh` to back up Open WebUI `webui.db` and
  persist its OpenAI backend/default model through Hermes gateway.
- Route apply now writes Open WebUI route env hints, runs the sync helper after
  Hermes health succeeds, and rolls Hermes env back if the sync helper fails.
- Added a WSL-side smoke test proving Open WebUI config sync does not print raw
  API keys.
- Live Open WebUI process refresh/restart was left for the next Phase 6
  increment.

## 2026-05-05: Phase 6 Open WebUI if-running refresh

- Added `hermes-control-openwebui-status.sh` and
  `hermes-control-openwebui-refresh.sh` for product-owned Open WebUI process
  detection and controlled restart.
- Route apply now runs Open WebUI refresh after DB sync. If Open WebUI is not
  running, refresh is skipped; if it is running, it is restarted with Hermes
  gateway env.
- Added a WSL-side refresh smoke test proving the helper passes route env to the
  process without printing raw API keys.

## 2026-05-06: Phase 6 Open WebUI refresh-failure recovery

- Route apply now restores the Open WebUI database backup when refresh fails
  after DB sync.
- The same failure path restores the previous Hermes env, restarts Hermes, and
  attempts to restart Open WebUI with the restored route env.
- Added a WSL-side route apply recovery smoke test for this post-sync failure
  path.

## 2026-05-06: Phase 6 explicit route rollback closeout

- Added daemon `POST /v1/route/rollback` to replay the last-known-good provider
  through the same fixed route apply helper.
- Added CLI `hermes-control route rollback` and Telegram `/rollback` thin-client
  boundaries.
- Added daemon, CLI, and bot tests covering rollback request shape, dry-run
  preview, state mutation, and missing last-known-good rejection.

## 2026-05-06: Phase 7 Teloxide bot code closeout

- Replaced the bot's ad hoc command parsing core with a Teloxide
  `HermesBotCommand` enum while preserving the daemon thin-client boundary.
- Added local SQLite `BotStateStore` for Telegram polling offset persistence,
  aligned with Teloxide long-polling offset behavior and without importing
  legacy code.
- Changed the runtime bot loop to long-poll from the persisted offset and record
  the next offset before handling each update.
- Added `BotEventLog` so the bot writes redacted runtime events to
  `logs/bot/bot.log`.
- Added polling retry configuration and changed the bot runtime loop to log
  Telegram polling/message-send failures and continue running.
- Added daemon `/v1/logs/{target}` for read-only `daemon`, `bot`, and `hermes`
  log tailing so Telegram `/logs` has a real typed API target.
- Added bot tests for `/start`, command mentions, `/audit` defaulting, Teloxide
  enum parsing, offset persistence across restarts, redacted event logs, runtime
  config parsing, and daemon log tailing.

## 2026-05-06: Phase 8 Tauri GUI scaffold started

- Added `apps/hermes-control-gui` as a Tauri v2 + React/TypeScript desktop app
  scaffold.
- Added a quiet operations-dashboard first screen with Dashboard, AI Route,
  Local Models, Runtime, Logs, Audit, and Settings surfaces.
- Added `GuiConfig`, `GuiDaemonClient`, `GuiDashboardSnapshot`, GUI requester
  helpers, and route-switch request helpers to `crates/hermes-control-gui`.
- Added Tauri command boundary `gui_dashboard_snapshot`, which calls local
  daemon APIs instead of controlling WSL/Hermes/vLLM directly.
- Added route switch dry-run preview, route rollback dry-run preview, and
  daemon-owned log tail commands for `daemon`, `bot`, and `hermes`.
- Wired the Route surface to provider options/current/LKG context and the Logs
  surface to bounded log target selection.
- Added `src-tauri/capabilities/default.json` with only `core:default`, plus
  tests proving no broad `shell:`, `fs:`, or process authority is exposed.
- Added front-end view-model tests and Rust Phase8 GUI boundary tests.

## 2026-05-06: Phase 8 GUI authority tier clarified

- Updated the plan to define GUI as the highest client authority surface because
  it runs locally on the Windows desktop.
- Clarified asymmetric parity: GUI should cover normal bot operations and more
  local-only controls, while Telegram remains the narrower remote-control
  surface.
- Reaffirmed that higher GUI permission still means typed daemon APIs,
  confirmation, requester identity, and audit, not raw shell/filesystem/process
  access.

## 2026-05-06: Phase 8 route execution and confirmation bridge

- Added GUI daemon-client methods and Tauri commands for route switch execution,
  route rollback execution, daemon confirmation, and daemon cancellation.
- Added controlled Switch/Rollback buttons to the Route surface; daemon
  `confirmation_required` responses now render a confirmation sheet with
  confirm/cancel actions.
- Kept Tauri capabilities at `core:default`; the new GUI authority is expressed
  only as additional typed daemon verbs.
- Added Rust and TypeScript tests for GUI requester shape, execution command
  coverage, confirmation lifecycle requests, and confirmation prompt rendering.

## 2026-05-06: Phase 8 Local Models action controls

- Added GUI daemon-client methods and Tauri commands for model action preview
  and execution through `POST /v1/models/{model_id}/action`.
- Wired the Local Models surface with selected model/action controls for
  install, start, stop, restart, health, logs, and benchmark.
- Reused the daemon confirmation sheet for destructive or experimental model
  operations while keeping Tauri capabilities at `core:default`.
- Added Rust and TypeScript tests for model action requester shape, GUI command
  coverage, and model action option rendering.

## 2026-05-07: Phase 8 Runtime action controls

- Added GUI daemon-client methods and Tauri commands for WSL and Hermes runtime
  action preview/execution.
- Wired the Runtime surface with WSL wake/stop/restart/shutdown and Hermes
  wake/stop/restart/kill controls.
- Reused the daemon confirmation sheet for destructive runtime operations while
  keeping Tauri capabilities at `core:default`.
- Added Rust and TypeScript tests for runtime action requester shape, GUI
  command coverage, and runtime action option rendering.

## 2026-05-07: Phase 8 Settings, Logs, and Audit controls

- Added redacted GUI connection summaries for Tauri desktop mode so Settings can
  show daemon URL, operator ID, and token configured state without exposing the
  raw token to the renderer.
- Wired browser-preview Settings controls for daemon URL, API token, and
  operator ID through localStorage, plus a daemon connection test.
- Added bounded log tail size selection and client-side loaded-line filtering to
  the Logs surface.
- Added Audit filters for risk, requester, and query over daemon-provided audit
  summaries.
- Kept Tauri capabilities at `core:default`; these controls remain typed daemon
  client flows or local renderer-only filtering.

## 2026-05-07: Phase 8 Chinese-first GUI i18n

- Added `src/lib/i18n.ts` with Simplified Chinese as the default GUI language
  and English as an explicit Settings option.
- Added Settings language selection stored locally in the renderer.
- Localized the main navigation, dashboard, Settings, route/model/runtime/log
  and audit controls, confirmation UI, and common action/risk labels.
- Kept daemon-facing action IDs and typed request payloads unchanged; i18n only
  affects operator-facing labels.
- Added tests for default language, language normalization, English fallback
  option, localized action labels, and risk label translation.

## 2026-05-07: Phase 8 browser preview daemon CORS fixed

- Reproduced the browser-preview `Failed to fetch` path as a daemon CORS
  preflight failure: `OPTIONS /v1/status` from `http://localhost:5174` returned
  HTTP 405 before the fix.
- Added a local GUI CORS layer to the daemon for `localhost` and `127.0.0.1`
  Vite origins on ports `5173` and `5174`.
- Kept daemon bearer auth unchanged; successful browser requests still require
  the GUI API token to match `HERMES_CONTROL_API_TOKEN`.
- Added a daemon Phase 3 regression test covering the local GUI preflight
  headers.

## 2026-05-07: Phase 8 dashboard Hermes status card

- Replaced the top status strip's standalone State DB card with a Hermes status
  card so operators can see Hermes reachability without opening Runtime.
- Combined state DB and audit DB visibility into one Dashboard detail row.
- Added a front-end view-model test for the combined local state-store summary.

## 2026-05-07: Phase 8 runtime readiness and log output repair

- Fixed local vLLM readiness checks to fall back from configured Windows
  loopback endpoints to the current WSL primary IP when vLLM is only reachable
  inside WSL.
- Added daemon `/v1/logs/vllm` support using configured model runtime log
  directories, and exposed `vllm` in both browser GUI and Tauri GUI safe log
  target lists.
- Updated the root start script so daemon stdout/stderr is written under
  `logs/daemon`, the same target read by the GUI Logs page.
- Added info-level HTTP request/response tracing to the daemon so the daemon log
  target has visible runtime activity instead of an always-empty file.
- Verified the live daemon at `http://127.0.0.1:18787` returned vLLM log lines
  from `vLLM\logs` while the model itself remained stopped.
