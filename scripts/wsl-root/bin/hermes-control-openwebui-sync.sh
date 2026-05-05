#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(CDPATH= cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck disable=SC1091
source "${SCRIPT_DIR}/hermes-control-common.sh"

hc_require_root
hc_prepare_dirs

base_url="${1:-$OPENWEBUI_HERMES_BASE_URL}"
default_model="${2:-$OPENWEBUI_DEFAULT_MODEL}"
api_key_env="${3:-$OPENWEBUI_OPENAI_API_KEY_ENV}"

if [[ ! "$base_url" =~ ^https?://[A-Za-z0-9_.:/-]+$ ]]; then
  echo "Open WebUI base-url must be http(s) and contain only safe route characters" >&2
  exit 2
fi

if [[ ! "$default_model" =~ ^[A-Za-z0-9_.-]+$ ]]; then
  echo "Open WebUI default model must be a safe identifier" >&2
  exit 2
fi

if [[ "$api_key_env" != "none" && ! "$api_key_env" =~ ^[A-Z][A-Z0-9_]*$ ]]; then
  echo "Open WebUI API key env must be 'none' or an uppercase environment variable name" >&2
  exit 2
fi

if [[ ! -f "$OPENWEBUI_DB_FILE" ]]; then
  OPENWEBUI_SYNC_STATE="open_webui_skipped" \
  OPENWEBUI_SYNC_DETAIL="Open WebUI database not found." \
  OPENWEBUI_SYNC_DB_FILE="$OPENWEBUI_DB_FILE" \
  python3 - <<'PY'
import json
import os

print(json.dumps({
    "state": os.environ["OPENWEBUI_SYNC_STATE"],
    "detail": os.environ["OPENWEBUI_SYNC_DETAIL"],
    "db_file": os.environ["OPENWEBUI_SYNC_DB_FILE"],
}, sort_keys=True))
PY
  exit 0
fi

HERMES_ENV_FILE="$HERMES_ENV_FILE" \
OPENWEBUI_DB_FILE="$OPENWEBUI_DB_FILE" \
OPENWEBUI_BACKUP_DIR="$OPENWEBUI_BACKUP_DIR" \
OPENWEBUI_ROUTE_BASE_URL="$base_url" \
OPENWEBUI_ROUTE_DEFAULT_MODEL="$default_model" \
OPENWEBUI_ROUTE_API_KEY_ENV="$api_key_env" \
python3 - <<'PY'
import json
import os
import sqlite3
import sys
import time
from pathlib import Path

env_file = Path(os.environ["HERMES_ENV_FILE"])
db_file = Path(os.environ["OPENWEBUI_DB_FILE"])
backup_dir = Path(os.environ["OPENWEBUI_BACKUP_DIR"])
base_url = os.environ["OPENWEBUI_ROUTE_BASE_URL"]
default_model = os.environ["OPENWEBUI_ROUTE_DEFAULT_MODEL"]
api_key_env = os.environ["OPENWEBUI_ROUTE_API_KEY_ENV"]

values = {}
if env_file.exists():
    for raw_line in env_file.read_text(encoding="utf-8").splitlines():
        stripped = raw_line.strip()
        if not stripped or stripped.startswith("#") or "=" not in raw_line:
            continue
        key, value = raw_line.split("=", 1)
        values[key.strip()] = value

api_key = ""
if api_key_env != "none":
    api_key = values.get(api_key_env) or os.environ.get(api_key_env, "")
    if not api_key:
        print(f"Required Open WebUI API key env is missing or empty: {api_key_env}", file=sys.stderr)
        sys.exit(1)

backup_dir.mkdir(parents=True, exist_ok=True)
backup_file = backup_dir / f"webui.db.hermes-control-openwebui.{time.time_ns()}.bak"

connection = sqlite3.connect(db_file)
try:
    backup_connection = sqlite3.connect(backup_file)
    try:
        connection.backup(backup_connection)
    finally:
        backup_connection.close()

    connection.execute(
        """
        CREATE TABLE IF NOT EXISTS config (
            id INTEGER PRIMARY KEY,
            data JSON NOT NULL,
            version INTEGER NOT NULL DEFAULT 0,
            created_at DATETIME,
            updated_at DATETIME
        )
        """
    )
    row = connection.execute(
        "SELECT id, data FROM config ORDER BY id DESC LIMIT 1"
    ).fetchone()
    if row is None:
        config_id = 1
        data = {"version": 0}
        connection.execute(
            "INSERT INTO config (id, data, version, created_at, updated_at) VALUES (?, ?, 0, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
            (config_id, json.dumps(data, ensure_ascii=False)),
        )
    else:
        config_id = row[0]
        try:
            data = json.loads(row[1]) if row[1] else {"version": 0}
        except json.JSONDecodeError:
            print("Open WebUI config.data is not valid JSON.", file=sys.stderr)
            sys.exit(1)

    if not isinstance(data, dict):
        print("Open WebUI config.data must be a JSON object.", file=sys.stderr)
        sys.exit(1)

    data.setdefault("version", 0)
    openai = data.setdefault("openai", {})
    openai["enable"] = True
    openai["api_base_urls"] = [base_url]
    openai["api_keys"] = [api_key]
    openai["api_configs"] = {"0": {}}
    ui = data.setdefault("ui", {})
    ui["default_models"] = default_model

    columns = {
        row[1]
        for row in connection.execute("PRAGMA table_info(config)").fetchall()
    }
    if "updated_at" in columns:
        connection.execute(
            "UPDATE config SET data = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
            (json.dumps(data, ensure_ascii=False), config_id),
        )
    else:
        connection.execute(
            "UPDATE config SET data = ? WHERE id = ?",
            (json.dumps(data, ensure_ascii=False), config_id),
        )
    connection.commit()
finally:
    connection.close()

print(json.dumps({
    "state": "open_webui_synced",
    "base_url": base_url,
    "default_model": default_model,
    "api_key_env": api_key_env,
    "db_file": str(db_file),
    "backup_file": str(backup_file),
}, sort_keys=True))
PY
