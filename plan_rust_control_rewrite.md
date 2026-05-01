# Hermes Rust Control Rewrite Plan v2

> Codename: **Hermes Control Center**
> Tone: **safe Rust control core + premium Microsoft/Apple-style GUI shell**
> Date: 2026-05-01

This document is a clean-slate reconstruction route for Codex. It treats the old Python/PowerShell control chain as a reference sample only. The final production system must be Rust-centered and must not depend on the old bot or free-form shell scripts.

---

## 0. Ground Truth From Current Files

Current vLLM workspace state:

- vLLM venv: `/opt/vllm-qwen36`
- vLLM workspace: `E:\WSL\vLLM`
- model directory: `E:\WSL\vLLM\models`
- log directory: `E:\WSL\vLLM\logs`
- cache directory: `E:\WSL\vLLM\cache`
- Windows-side OpenAI-compatible endpoint: `http://127.0.0.1:18080/v1`
- local served model names:
  - `qwen36-awq-int4`
  - `qwen36-mtp`
- stable tested mode: 90000-token MTP without CPU offload
- experimental mode: 128000-token MTP with `CUDA_VISIBLE_DEVICES=1,0` and `CPU_OFFLOAD_GB=2`, slower and less stable

Current v1 plan direction:

- `hermes-agent` remains the conversation runtime.
- Telegram bot, Windows GUI, and CLI call the same Rust control daemon.
- Bot/GUI must not directly execute arbitrary shell.
- vLLM workspace should become a managed local model runtime.

V2 keeps these principles but changes the migration posture from “compatibility-first” to **clean rebuild with legacy as read-only behavior reference**.

---

## 1. Product Meaning

Hermes Control Center is not a chat client. It is the local command tower for Hermes.

It controls four things:

1. **AI route ownership**
   - Which AI Hermes is currently attached to.
   - External API, local vLLM, Codex, Claude, DeepSeek, OpenAI-compatible endpoint, LM Studio, or disabled/safe mode.

2. **WSL2 subsystem lifecycle**
   - Detect, wake, stop, restart, and inspect the WSL distro used by Hermes.
   - Keep daemon/bot alive on Windows even when WSL is down.

3. **Local GUI management panel**
   - High-quality Windows desktop control center.
   - Microsoft/Apple-inspired visual design.
   - Safe confirmation UX for destructive operations.

4. **Local model optimization and runtime control**
   - vLLM backend for Qwen3.6.
   - MTP speculative decoding profile for latency-oriented work.
   - AWQ INT4 profile for stable low-memory fallback.
   - Health checks, logs, profile readiness, and benchmark history.

---

## 2. Recommended Design Baseline

Use this baseline unless explicitly overridden:

```text
Control core:          safety-lightweight, all Rust, typed actions only
Bot/CLI:               thin clients, no direct process execution
GUI:                   advanced beautiful, Tauri v2 + Fluent 2 inspired frontend
Emergency fallback UI: optional tiny eframe/egui rescue panel after v1
Legacy code:           read-only reference, not production dependency
```

This resolves the “safe lightweight vs advanced beautiful” question as:

> The daemon and command core must be safe and lightweight. The GUI can be advanced and beautiful because it sits above a narrow, typed API and has no raw system authority.

If forced to choose a single visible product direction, choose **advanced beautiful**. Your UI standard is high; an egui-only main panel will be faster to build but harder to make feel like Microsoft Settings, Apple System Settings, or a polished control center.

---

## 3. Non-Negotiable Principles

1. **One source of truth**
   - Only `hermes-control-daemon` mutates state.
   - CLI, Telegram bot, and GUI call daemon APIs.

2. **No arbitrary shell**
   - There is no endpoint like `run_command` or `exec`.
   - Every operation is a Rust enum and maps to a fixed command builder.

3. **Windows daemon survives WSL failure**
   - Bot and GUI stay alive when WSL is stopped, broken, or restarting.

4. **Local-only control plane**
   - Bind to `127.0.0.1` by default.
   - Named pipe can be added later for GUI/CLI.
   - No LAN exposure unless explicitly configured.

5. **Secrets never live in profile JSON**
   - Profile config stores `secret_ref`, not raw API keys.
   - Use Windows Credential Manager or DPAPI-backed encrypted local file.

6. **Dangerous actions require confirmation**
   - Kill Hermes, stop/restart WSL, stop/restart model, switch profile with restart, cleanup caches, and unregister-like WSL actions are all confirmed operations.

7. **Old house demolition rule**
   - `admin-controller` and `switch/*.ps1` are not production dependencies.
   - During rebuild they may be moved to `_legacy_reference/` or left untouched but never called by new code except explicitly marked compatibility tests.

---

## 4. Workspace Layout

```text
E:\WSL\Hermres\
  hermes-agent\                         # keep: conversation runtime
  hermes-control\                       # new Rust tower
    Cargo.toml
    crates\
      hermes-control-types\             # shared API DTOs and domain enums
      hermes-control-core\              # business logic, config, state, safety rules
      hermes-control-daemon\            # Windows resident daemon
      hermes-control-cli\               # local CLI thin client
      hermes-control-bot\               # teloxide admin bot thin client
      hermes-control-gui\               # Tauri shell / Rust backend bridge
      hermes-control-testkit\           # fake vLLM, fake WSL, fixtures
    config\
      control.toml
      providers.toml
      model-runtimes.toml
      ui-theme.toml
    state\
      state.sqlite
      audit.sqlite
      transient\
    logs\
      daemon\
      bot\
      gui\
    docs\
      architecture.md
      api.md
      threat-model.md
      ui-design-system.md
      codex-tasks.md
    _legacy_reference\                  # optional archive, never imported by production code
      README.md
```

`hermes-control-types` is separated so bot/GUI/CLI can depend on shared request/response types without importing process-control internals.

---

## 5. Crate Responsibilities

### 5.1 `hermes-control-types`

Contains stable data contracts:

```rust
pub enum AiProviderKind {
    OpenAiCompatible,
    AnthropicClaude,
    DeepSeek,
    Codex,
    LocalVllm,
    LmStudio,
    Disabled,
}

pub enum HermesAction {
    Wake,
    Stop,
    Restart,
    Kill,
}

pub enum WslAction {
    Wake,
    StopDistro,
    RestartDistro,
    ShutdownAll,
}

pub enum ModelAction {
    Start,
    Stop,
    Restart,
    Health,
    Logs,
    Benchmark,
}

pub enum RiskLevel {
    ReadOnly,
    NormalMutating,
    Destructive,
    Experimental,
}
```

Also contains:

- API DTOs
- error codes
- status structs
- audit event schemas
- confirmation request schemas

### 5.2 `hermes-control-core`

Owns domain logic:

- config loading and validation
- secret reference resolution
- operation lock
- confirmation manager
- audit logging
- AI route/profile switcher
- typed process command builders
- WSL controller
- vLLM runtime controller
- Hermes runtime controller
- log tailing
- health aggregation

### 5.3 `hermes-control-daemon`

Windows resident process:

- local API server
- single mutating-operation queue
- scheduler for health polling
- readiness watcher for vLLM and Hermes
- state persistence
- Windows service mode
- graceful shutdown

### 5.4 `hermes-control-cli`

Thin client:

```text
hermes-control status
hermes-control health
hermes-control providers
hermes-control route active
hermes-control route switch <profile-id>
hermes-control models
hermes-control model status <model-id>
hermes-control model start <model-id>
hermes-control wsl status
hermes-control wsl restart --confirm
hermes-control logs model <model-id> --tail 200
```

CLI should support `--json` for machine-readable output.

### 5.5 `hermes-control-bot`

Teloxide bot:

- Parses commands into typed API calls.
- Maintains allowlist by Telegram user ID and optional chat ID.
- Performs no direct system mutation.
- Uses daemon confirmation flow.
- Stores Telegram offset/dialogue state through daemon or a local SQLite bot table.

### 5.6 `hermes-control-gui`

Recommended: Tauri v2 GUI with a Rust command bridge that calls daemon API.

- Frontend can use React + Fluent UI React v9 or a small custom design-token implementation.
- Rust side only exposes typed commands.
- No frontend-side raw process execution.
- UI must support keyboard navigation, dark/light/system themes, and accessible contrast.

Optional later: `hermes-control-rescue-gui` with eframe/egui for a tiny no-web rescue panel.

---

## 6. Static Configuration Model

### 6.1 `control.toml`

```toml
[daemon]
bind = "127.0.0.1:18787"
api_token_ref = "hermes/control/api-token"
state_db = "state/state.sqlite"
audit_db = "state/audit.sqlite"
log_dir = "logs/daemon"
operation_timeout_seconds = 900

[wsl]
distro = "Ubuntu-Hermes-Codex"
default_user = "hermes"

[hermes]
agent_root = "E:\\WSL\\Hermres\\hermes-agent"
health_url = "http://127.0.0.1:18000/health"
logs = ["E:\\WSL\\Hermres\\hermes-agent\\logs"]

[policy]
require_confirm_for_destructive = true
allow_lan_bind = false
allow_raw_shell = false
redact_secrets = true
```

### 6.2 `providers.toml`

```toml
[[providers]]
id = "external.openai-compatible"
kind = "OpenAiCompatible"
display_name = "External OpenAI-compatible API"
base_url = "https://example.com/v1"
api_key_ref = "hermes/provider/external-openai-compatible"
models = ["gpt-like-coder", "reasoner"]

[[providers]]
id = "anthropic.claude"
kind = "AnthropicClaude"
display_name = "Claude"
api_key_ref = "hermes/provider/claude"

[[providers]]
id = "deepseek.api"
kind = "DeepSeek"
display_name = "DeepSeek API"
api_key_ref = "hermes/provider/deepseek"

[[providers]]
id = "local.vllm.qwen36-mtp"
kind = "LocalVllm"
display_name = "Qwen3.6 MTP via vLLM"
base_url = "http://127.0.0.1:18080/v1"
model_runtime = "vllm-local"
served_model_name = "qwen36-mtp"
```

### 6.3 `model-runtimes.toml`

```toml
[[runtimes]]
id = "vllm-local"
kind = "Vllm"
workspace = "E:\\WSL\\vLLM"
wsl_distro = "Ubuntu-Hermes-Codex"
endpoint = "http://127.0.0.1:18080/v1"
models_endpoint = "http://127.0.0.1:18080/v1/models"
log_dir = "E:\\WSL\\vLLM\\logs"

[[runtimes.variants]]
id = "qwen36-awq-int4"
served_model_name = "qwen36-awq-int4"
mode = "stable"
max_model_len = 90000
start = { kind = "wsl_script", script = "/mnt/e/WSL/vLLM/scripts/serve-qwen36-awq-int4.sh" }
stop = { kind = "process_match", served_model_name = "qwen36-awq-int4" }
profiles = ["vllm.qwen36-awq-int4"]

[[runtimes.variants]]
id = "qwen36-mtp"
served_model_name = "qwen36-mtp"
mode = "latency"
max_model_len = 90000
speculative_method = "mtp"
num_speculative_tokens = 2
start = { kind = "wsl_script", script = "/mnt/e/WSL/vLLM/scripts/serve-qwen36-mtp.sh" }
stop = { kind = "process_match", served_model_name = "qwen36-mtp" }
profiles = ["vllm.qwen36-mtp"]

[[runtimes.variants]]
id = "qwen36-mtp-128k-experimental"
served_model_name = "qwen36-mtp"
mode = "experimental"
max_model_len = 128000
cpu_offload_gb = 2
cuda_visible_devices = "1,0"
requires_explicit_confirm = true
profiles = ["vllm.qwen36-mtp.128k"]
```

Final Rust code should not call `.ps1` scripts. It may call fixed WSL scripts or directly invoke `wsl.exe` with fixed arguments.

---

## 7. Daemon API Shape

Prefer versioned API:

```text
GET  /v1/status
GET  /v1/health
GET  /v1/audit?limit=100
GET  /v1/providers
GET  /v1/route/active
POST /v1/route/switch

GET  /v1/hermes/status
POST /v1/hermes/action

GET  /v1/wsl/status
POST /v1/wsl/action

GET  /v1/models
GET  /v1/models/{id}
POST /v1/models/{id}/action
GET  /v1/models/{id}/logs?tail=200
POST /v1/models/{id}/benchmark

GET  /v1/logs/{target}?tail=200
POST /v1/confirm
POST /v1/cancel
```

All mutating endpoints accept a typed body:

```json
{
  "requester": {
    "channel": "gui|cli|telegram",
    "user_id": "..."
  },
  "action": "Restart",
  "reason": "manual recovery",
  "dry_run": false
}
```

If confirmation is required:

```json
{
  "status": "confirmation_required",
  "confirmation_id": "op_01...",
  "code_hint": "HERMES-7421",
  "expires_at": "...",
  "risk": "Destructive",
  "summary": "Restart WSL distro Ubuntu-Hermes-Codex and stop all in-distro processes."
}
```

---

## 8. Command Builder Rules

Never do this:

```rust
Command::new(user_supplied_program).args(user_supplied_args)
```

Always do this:

```rust
enum FixedProgram {
    WslExe,
    HermesAgentBinary,
}

struct WslCommandSpec {
    distro: KnownDistro,
    user: KnownWslUser,
    operation: KnownWslOperation,
}
```

Examples:

```text
WslOperation::ListVerbose
  -> wsl.exe --list --verbose

WslOperation::RunHermesHelper(Health)
  -> wsl.exe --distribution Ubuntu-Hermes-Codex --user hermes -- /opt/hermes-control/bin/health.sh

WslOperation::TerminateDistro
  -> wsl.exe --terminate Ubuntu-Hermes-Codex

WslOperation::ShutdownAll
  -> wsl.exe --shutdown
```

Only `KnownWslOperation` can create process arguments.

---

## 9. State and Audit

Use SQLite, not scattered JSON, for daemon-owned mutable state.

### 9.1 Tables

```sql
active_route(id, provider_id, model_id, switched_at, switched_by, reason)
operation_lock(id, op_id, acquired_at, expires_at)
pending_confirmations(id, requester_channel, requester_id, action_json, code_hash, expires_at)
model_state(model_id, status, pid, endpoint, last_ready_at, last_error)
telegram_state(bot_id, update_offset, updated_at)
audit_events(id, ts, requester_channel, requester_id, action, risk, result, redacted_summary)
bench_runs(id, model_id, ts, input_len, output_len, concurrency, tpot_ms, tokens_per_second, notes)
```

### 9.2 Audit Rules

Log:

- who requested
- channel
- action
- dry run summary
- confirmation result
- execution result
- duration
- error class

Never log:

- raw API keys
- bearer tokens
- Authorization headers
- full environment variables
- Telegram bot token
- model provider secret values

---

## 10. AI Route and Profile Switching

Introduce a first-class concept: **Route**.

A profile is not just a JSON blob. It is a complete route contract:

```rust
struct AiRouteProfile {
    id: String,
    provider_id: String,
    model_id: String,
    base_url: Option<Url>,
    runtime_requirement: Option<ModelRuntimeRequirement>,
    hermes_env_patch: RedactedEnvPatch,
    health_check: HealthCheckSpec,
    rollback: RollbackPolicy,
}
```

Switch algorithm:

1. Validate profile exists.
2. Validate provider secret is resolvable.
3. If local runtime is required, ensure runtime is running.
4. Wait for `/v1/models` to show the served model.
5. Apply Hermes profile/env patch atomically.
6. Restart or hot-reload Hermes according to profile policy.
7. Confirm Hermes health.
8. Mark route active.
9. If step 4-7 fail, rollback to last-known-good route.

---

## 11. vLLM Runtime Design

### 11.1 Modes

```text
qwen36-awq-int4
  Purpose: stable fallback, lower memory pressure, predictable recovery.
  Use when: long admin session, reliability first, after failed MTP launch.

qwen36-mtp
  Purpose: low-latency coding and single-user interactive work.
  Use when: Hermes is acting as local coding agent and concurrency is low.

qwen36-mtp-128k-experimental
  Purpose: exceptional long-context tasks.
  Use when: explicitly confirmed; accept slower and less stable behavior.
```

### 11.2 Health Check

Readiness is not “process exists”. Readiness is:

1. TCP endpoint accepts connections.
2. `GET /v1/models` succeeds.
3. Response includes the expected `served_model_name`.
4. Optional smoke chat/completion under small max tokens succeeds.
5. Last N log lines contain no fatal CUDA/vLLM error patterns.

### 11.3 Start Flow

```text
ModelAction::Start(qwen36-mtp)
  -> acquire operation lock
  -> verify WSL distro reachable
  -> verify workspace paths
  -> verify port 18080 not occupied by wrong model
  -> run fixed WSL start helper
  -> stream progress to audit/log subscribers
  -> poll /v1/models until ready or timeout
  -> persist model_state
```

### 11.4 Stop Flow

Stop should be graceful first, hard second:

1. Ask known vLLM process group to terminate.
2. Wait for endpoint to go away.
3. Kill only process IDs matching daemon-owned process metadata or safe vLLM command signature.
4. Never kill arbitrary Python processes by default.

### 11.5 Benchmarking

Add a controlled benchmark action:

```text
hermes-control model benchmark qwen36-mtp --input 2048 --output 512 --concurrency 1
hermes-control model benchmark qwen36-awq-int4 --input 2048 --output 512 --concurrency 4
```

Store results in `bench_runs`. GUI should show a small history chart so MTP changes are measurable rather than emotional.

---

## 12. Telegram Bot Design With Teloxide

The bot is a remote-control keyboard, not an executor.

Commands:

```text
/status
/health
/providers
/route
/switch <profile-id>
/models
/model <status|start|stop|restart|logs|benchmark> <model-id>
/hermes <wake|stop|restart|kill>
/wsl <status|wake|stop|restart>
/logs <hermes|daemon|bot|model> [id]
/audit [limit]
/confirm <code>
/cancel
```

Handler rules:

- Parse command with strongly typed command enum.
- Check Telegram allowlist before daemon call.
- Add requester metadata to daemon request.
- For confirmation: daemon creates code; bot only relays.
- Bot state persistence should survive bot restart.
- Bot process must run on Windows, not inside WSL, so WSL restart does not silence remote control.

---

## 13. GUI Direction

### 13.1 Recommended Main GUI Stack

```text
Tauri v2 shell
  Rust side: typed daemon client and window/tray integration
  Frontend: React + TypeScript + Fluent UI React v9 or custom Fluent tokens
  Styling: Fluent 2 tokens + Apple HIG-inspired layout restraint
```

Reason:

- Better chance of achieving Microsoft/Apple-level polish.
- Mature sidebar, drawer, command bar, modal, table, tooltip, skeleton, and theme token patterns.
- Web frontend gives smoother animation and typography control.
- Tauri still keeps privileged system authority in Rust, behind explicit commands.

### 13.2 eframe/egui Role

Do not make egui the main UI if premium UI is the priority. Use it later for an emergency rescue panel:

```text
hermes-rescue.exe
  Dashboard
  WSL status
  Restart daemon
  Start qwen36-awq-int4
  Switch to last-known-good route
```

### 13.3 Visual Language

Name: **Hermes Control Center**

Style:

- quiet, spacious, utility-first
- translucent/acrylic inspiration without heavy blur everywhere
- left navigation rail
- command bar on top
- cards for runtime health
- status chips for model/provider state
- drawer for settings and details
- destructive actions in clear red/orange risk zones
- tables for logs/audit with filters
- light/dark/system themes

Do not copy Apple or Microsoft visuals exactly. Use the design principles:

- clear hierarchy
- consistent spacing tokens
- platform-native typography feel
- reduced modal overload
- keyboard-first operations
- high contrast and readable logs

### 13.4 GUI Pages

```text
Dashboard
  - Global system health
  - Active AI route
  - Hermes runtime status
  - WSL status
  - Current local model
  - Last critical audit event

AI Route
  - Provider cards: External API / Claude / DeepSeek / Codex / Local vLLM
  - Active route badge
  - Switch route flow
  - Last-known-good route

Local Models
  - qwen36-awq-int4 card
  - qwen36-mtp card
  - qwen36-mtp-128k experimental card
  - Start/Stop/Restart/Benchmark
  - vLLM endpoint health
  - benchmark history

WSL
  - distro status
  - WSL version
  - wake/terminate/restart controls
  - helper health

Hermes Runtime
  - wake/stop/restart/kill
  - profile hot reload if supported
  - agent logs

Logs
  - daemon logs
  - Hermes logs
  - vLLM logs
  - bot logs
  - tail/follow/search/copy

Audit
  - operation timeline
  - requester/channel filters
  - confirmation results
  - failure details

Settings
  - Telegram admin allowlist
  - provider secret refs
  - daemon API token rotation
  - auto-start/service install
  - theme and density
```

### 13.5 Confirmation UX

For destructive actions, display a sheet:

```text
Action: Restart WSL distro Ubuntu-Hermes-Codex
Impact: Hermes runtime and vLLM process may stop.
Rollback: daemon stays alive; last-known-good route preserved.
Required input: type HERMES-7421
```

Use countdown expiration and show exact typed action, not vague “Are you sure?”.

---

## 14. Windows Service and Autostart

Preferred final mode:

- `hermes-control-daemon.exe` installed as Windows Service.
- `hermes-control-bot.exe` can be child service or separate service.
- GUI is normal user app with tray option.

Development mode:

- Run daemon from terminal.
- Add a Scheduled Task only after service behavior is stable.

Service behavior:

- report `StartPending` then `Running`
- accept stop control
- flush audit log on stop
- never block service control handler on long operations

---

## 15. Migration / Demolition Plan

### Phase 0 — Legacy Freeze and Behavior Inventory

Goal:

- Stop adding features to Python/PowerShell.
- Inventory current commands, profiles, and failure cases.
- Copy or move legacy control code into `_legacy_reference/`.
- Write a `LEGACY_DO_NOT_USE.md` marker.

Codex tasks:

- Create `docs/legacy-behavior-inventory.md`.
- List old commands and desired Rust replacements.
- Add CI grep rule: production crates must not reference `admin-controller`, `switch.ps1`, `ops.ps1`, or `model.ps1`.

Completion signal:

- Old code is reference-only.
- New code has no runtime dependency on it.

### Phase 1 — Rust Workspace Skeleton

Goal:

- Create workspace and crates.
- Add shared types and config schema.
- Add formatting/lint baseline.

Codex tasks:

- `cargo new` all crates.
- Add `serde`, `thiserror`, `tracing`, `tokio`, `reqwest`, `axum`, `clap` where needed.
- Add `deny.toml` or equivalent supply-chain policy.
- Add first domain enums and DTOs.

Completion signal:

- `cargo test --workspace` passes.
- `hermes-control-cli --help` works.

### Phase 2 — Read-Only Core and CLI

Goal:

- Read config.
- Read state.
- Report status without mutating the machine.

Codex tasks:

- Implement config loaders.
- Implement WSL status parser for `wsl.exe --list --verbose`.
- Implement vLLM endpoint status check.
- Implement log tail reader.
- Implement CLI `status`, `health`, `models`, `providers`.

Completion signal:

- CLI can show WSL, Hermes, and vLLM status.
- No start/stop/restart yet.

### Phase 3 — Daemon API and SQLite State

Goal:

- Make daemon the state authority.

Codex tasks:

- Add axum local API.
- Add bearer token middleware.
- Add SQLite migrations.
- Add operation lock.
- Add audit events.
- Add confirmation lifecycle.

Completion signal:

- CLI can call daemon instead of reading directly.
- daemon restart preserves active route and audit events.

### Phase 4 — Typed WSL and Hermes Runtime Ops

Goal:

- Replace old ops scripts with Rust command builders.

Codex tasks:

- Implement `WslController`.
- Implement `HermesRuntimeController`.
- Add wake/stop/restart/kill with confirmation policy.
- Add dry-run summaries.

Completion signal:

- WSL can be restarted while daemon survives.
- Hermes can be restarted and health-checked.

### Phase 5 — vLLM Runtime Manager

Goal:

- Make vLLM a first-class managed runtime.

Codex tasks:

- Implement runtime registry.
- Implement `qwen36-awq-int4` and `qwen36-mtp` variants.
- Implement start/stop/restart/status/logs.
- Implement `/v1/models` readiness polling.
- Implement failed-start cleanup.
- Implement benchmark action.

Completion signal:

- `hermes-control model start qwen36-mtp` reaches ready state.
- `hermes-control model logs qwen36-mtp` works.
- failed MTP start can fall back to AWQ profile.

### Phase 6 — Route Switcher

Goal:

- Control who Hermes talks to.

Codex tasks:

- Implement providers.
- Implement active route state.
- Implement last-known-good route.
- Implement switch to external API.
- Implement switch to Claude/DeepSeek/Codex profiles.
- Implement switch to local vLLM profile with model readiness gate.

Completion signal:

- Route switch never leaves Hermes in unknown provider state.
- Local vLLM route waits for model ready before switching.

### Phase 7 — Teloxide Bot

Goal:

- Replace Python Telegram bot.

Codex tasks:

- Use teloxide command enum.
- Implement allowlist.
- Implement daemon client.
- Implement confirmation flow.
- Implement logs/status/model/route commands.
- Store offset/dialogue state robustly.

Completion signal:

- Bot stays responsive after Hermes restart.
- Bot stays responsive after WSL restart.
- Bot never executes shell directly.

### Phase 8 — Premium GUI

Goal:

- Build high-quality local management panel.

Codex tasks:

- Create Tauri v2 app.
- Create design-token layer.
- Implement Dashboard, AI Route, Local Models, WSL, Hermes Runtime, Logs, Audit, Settings.
- Implement confirmation sheet.
- Implement tray icon.
- Implement theme switching.

Completion signal:

- GUI can perform all normal operations without Telegram.
- GUI status matches CLI and bot.
- GUI has no privileged raw execution API.

### Phase 9 — Windows Service and Installer

Goal:

- Make the control tower survive Windows reboot.

Codex tasks:

- Add Windows service mode to daemon.
- Add install/uninstall CLI commands.
- Add service recovery policy documentation.
- Add optional installer packaging.

Completion signal:

- Windows reboot restores daemon and bot.
- GUI can connect after login.

### Phase 10 — Legacy Deletion

Goal:

- Finish demolition.

Codex tasks:

- Remove old Python/PowerShell production paths.
- Keep only a compressed archive or docs snapshot if needed.
- Update README to Rust-only operations.
- Add final E2E checklist.

Completion signal:

- Daily use depends only on Rust binaries and vLLM/Hermes runtime.
- Grep confirms no production call to old scripts.

---

## 16. Security Test Checklist

- API rejects requests without token.
- API rejects LAN bind unless explicitly enabled.
- Telegram user not in allowlist gets no privileged response.
- Confirmation code expires.
- Confirmation code cannot be used by another requester.
- Two mutating actions cannot run concurrently.
- Secrets do not appear in logs, audit, panic output, or GUI copy text.
- Model/profile JSON cannot inject shell args.
- Log viewer cannot read arbitrary file paths.
- `wsl --unregister` is not implemented.
- Cleanup cache action requires explicit destructive confirmation.

---

## 17. E2E Test Checklist

1. Windows boot starts daemon.
2. CLI status works.
3. GUI status works.
4. Bot `/status` works.
5. Start `qwen36-awq-int4`.
6. Start `qwen36-mtp`.
7. Switch Hermes to local MTP profile.
8. Switch Hermes to external API profile.
9. Stop Hermes and wake it.
10. Restart WSL and verify daemon/bot stay alive.
11. Tail vLLM logs from GUI and bot.
12. Trigger one failed model start and verify rollback.
13. Rotate API token and verify old clients fail.
14. Reboot Windows and verify service recovery.

---

## 18. UI Quality Bar

A screen is not complete until:

- it has empty/loading/error/ready states
- status is visually obvious in 2 seconds
- destructive action is impossible to click accidentally
- text can be copied where useful
- logs are readable in dark mode
- primary command has keyboard shortcut where appropriate
- layout works at 1280x720 and 4K
- no raw secrets are visible
- high-DPI scaling is acceptable
- UI copy uses calm operator language, not developer noise

---

## 19. Codex Working Rules

Give Codex these rules before implementation:

1. Build from the Rust workspace outward.
2. Do not import old Python or PowerShell code into production crates.
3. Do not expose any raw command execution endpoint.
4. Add tests before each mutating operation.
5. Every external process invocation must go through a typed command builder.
6. Every destructive action must have a dry-run summary and confirmation policy.
7. Every state mutation must create an audit event.
8. All API DTOs live in `hermes-control-types`.
9. GUI and bot must be thin clients.
10. Prefer making the operation impossible at type level over checking strings at runtime.

---

## 20. Reference Sources Checked

- Teloxide framework repository and docs: `https://github.com/teloxide/teloxide`, `https://docs.rs/teloxide/latest/teloxide/`
- vLLM MTP documentation: `https://docs.vllm.ai/en/latest/features/speculative_decoding/mtp/`
- vLLM Qwen3.5/Qwen3.6 recipe: `https://docs.vllm.ai/projects/recipes/en/latest/Qwen/Qwen3.5.html`
- vLLM Qwen3-Next recipe: `https://docs.vllm.ai/projects/recipes/en/latest/Qwen/Qwen3-Next.html`
- vLLM OpenAI-compatible server docs: `https://docs.vllm.ai/` and mirrored current docs
- Microsoft WSL basic commands: `https://learn.microsoft.com/windows/wsl/basic-commands`
- Tauri v2 security model: `https://v2.tauri.app/security/`
- eframe docs: `https://docs.rs/eframe/latest/eframe/`
- Microsoft Fluent 2 design tokens and typography docs: `https://fluent2.microsoft.design/`
- Apple Human Interface Guidelines: `https://developer.apple.com/design/human-interface-guidelines/`
- Rust `windows-service` crate docs: `https://docs.rs/windows-service/latest/windows_service/`
