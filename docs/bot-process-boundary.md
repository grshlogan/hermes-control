# Hermes Control Bot Process Boundary

`hermes-control-bot` is a Windows-hosted Telegram admin process. It is a thin
client for `hermes-control-daemon`; it does not own state and does not execute
system commands.

## Authority Boundary

- Runs on Windows, outside WSL.
- Receives Telegram messages through Teloxide.
- Checks Telegram user and optional chat allowlists before any daemon call.
- Converts commands into typed daemon API requests.
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
```

`HERMES_CONTROL_TELEGRAM_TOKEN` can be used instead of `TELOXIDE_TOKEN`.
`HERMES_ADMIN_ALLOWED_USERS` is accepted as a temporary migration alias.

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
