# Hermes Control Bot Process Boundary

`hermes-control-bot` is a Windows-hosted Telegram admin process. It is a thin
client for `hermes-control-daemon`; it does not own state and does not execute
system commands.

## Authority Boundary

- Runs on Windows, outside WSL.
- Receives Telegram messages through Teloxide long polling.
- Checks Telegram user and optional chat allowlists before any daemon call.
- Converts a Teloxide `HermesBotCommand` enum into typed daemon API requests.
- Stores Telegram polling offset in local SQLite so restarts resume cleanly.
- Writes a local redacted event log for startup, command-menu publication, and
  daemon request failures.
- Retries Telegram polling after transient API/network failures instead of
  exiting the bot process.
- Sends requests only to the configured local daemon URL.
- Never calls `powershell`, `wsl.exe`, `.ps1`, shell scripts, or arbitrary
  subprocess APIs.

The daemon remains the only process allowed to mutate Hermes, WSL, model, route,
state, audit, or confirmation records.

## Required Environment

```powershell
$env:TELOXIDE_TOKEN = "<telegram bot token>"
$env:HERMES_CONTROL_API_TOKEN = "<daemon bearer token>"
$env:HERMES_CONTROL_TELEGRAM_ALLOWED_USERS = "<telegram user id>[,<telegram user id>]"
```

Optional:

```powershell
$env:HERMES_CONTROL_DAEMON_URL = "http://127.0.0.1:18787"
$env:HERMES_CONTROL_TELEGRAM_ALLOWED_CHATS = "<telegram chat id>[,<telegram chat id>]"
$env:HERMES_CONTROL_BOT_STATE_DB = "state/bot.sqlite"
$env:HERMES_CONTROL_BOT_LOG_DIR = "logs/bot"
$env:HERMES_CONTROL_BOT_ID = "primary"
$env:HERMES_CONTROL_BOT_POLL_TIMEOUT_SECONDS = "30"
$env:HERMES_CONTROL_BOT_POLL_ERROR_RETRY_SECONDS = "5"
```

`HERMES_CONTROL_TELEGRAM_TOKEN` can be used instead of `TELOXIDE_TOKEN`.
`HERMES_ADMIN_ALLOWED_USERS` is accepted as a temporary migration alias.

## Local State

The bot owns only Telegram polling state. The default SQLite DB is:

```text
E:\WSL\Hermres\hermes-control\state\bot.sqlite
```

It contains `telegram_state(bot_id, update_offset, updated_at)`. Machine-control
state, confirmations, audit rows, Hermes, WSL, vLLM, and route state still live
behind `hermes-control-daemon`.

## Local Logs

The default redacted bot event log is:

```text
E:\WSL\Hermres\hermes-control\logs\bot\bot.log
```

This log is the local source for daemon `/v1/logs/bot` tailing. It must not
contain Telegram tokens, daemon bearer tokens, raw authorization headers, or API
keys.

## Development Startup

```powershell
cd E:\WSL\Hermres\hermes-control
cargo run -p hermes-control-bot
```

## Release Build

```powershell
cd E:\WSL\Hermres\hermes-control
cargo build -p hermes-control-bot --release
```

The produced binary is:

```text
E:\WSL\Hermres\hermes-control\target\release\hermes-control-bot.exe
```

Service installation is intentionally left for the later Windows service phase.
