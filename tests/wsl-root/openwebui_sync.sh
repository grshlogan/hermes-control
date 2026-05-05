#!/usr/bin/env bash
set -euo pipefail

PROJECT_ROOT="$(CDPATH= cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)"
tmp_dir="$(mktemp -d)"
trap 'rm -rf "$tmp_dir"' EXIT

env_file="${tmp_dir}/hermes.env"
data_dir="${tmp_dir}/open-webui-data"
db_file="${data_dir}/webui.db"
mkdir -p "$data_dir"

cat > "$env_file" <<'ENV'
API_SERVER_KEY=local-hermes-secret
LM_API_KEY=external-provider-secret
ENV
chmod 0600 "$env_file"

python3 - "$db_file" <<'PY'
import json
import sqlite3
import sys

db_file = sys.argv[1]
connection = sqlite3.connect(db_file)
connection.execute(
    """
    CREATE TABLE config (
        id INTEGER PRIMARY KEY,
        data JSON NOT NULL,
        version INTEGER NOT NULL DEFAULT 0,
        created_at DATETIME,
        updated_at DATETIME
    )
    """
)
connection.execute(
    "INSERT INTO config (id, data, version, created_at, updated_at) VALUES (1, ?, 0, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
    (
        json.dumps(
            {
                "version": 0,
                "ui": {"enable_signup": False},
                "openai": {
                    "enable": False,
                    "api_base_urls": ["https://old.example/v1"],
                    "api_keys": ["old-secret"],
                },
            }
        ),
    ),
)
connection.commit()
PY

output="$(
  HERMES_CONTROL_CONFIG_FILE="/dev/null" \
  HERMES_CONTROL_WORK_ROOT="$tmp_dir" \
  HERMES_ENV_FILE="$env_file" \
  OPENWEBUI_DATA_DIR="$data_dir" \
  OPENWEBUI_DB_FILE="$db_file" \
  "${PROJECT_ROOT}/scripts/wsl-root/bin/hermes-control-openwebui-sync.sh" \
    "http://127.0.0.1:8642/v1" \
    "hermes-agent" \
    "API_SERVER_KEY"
)"

OPENWEBUI_SYNC_OUTPUT="$output" python3 - "$db_file" "$tmp_dir" <<'PY'
import glob
import json
import os
import sqlite3
import sys

db_file = sys.argv[1]
tmp_dir = sys.argv[2]
output = os.environ["OPENWEBUI_SYNC_OUTPUT"]
payload = json.loads(output)
assert payload["state"] == "open_webui_synced", payload
assert payload["base_url"] == "http://127.0.0.1:8642/v1", payload
assert payload["default_model"] == "hermes-agent", payload
assert "local-hermes-secret" not in output, output
assert "external-provider-secret" not in output, output

connection = sqlite3.connect(db_file)
raw_data = connection.execute("SELECT data FROM config WHERE id = 1").fetchone()[0]
data = json.loads(raw_data)
assert data["openai"]["enable"] is True, data
assert data["openai"]["api_base_urls"] == ["http://127.0.0.1:8642/v1"], data
assert data["openai"]["api_keys"] == ["local-hermes-secret"], data
assert data["openai"]["api_configs"] == {"0": {}}, data
assert data["ui"]["default_models"] == "hermes-agent", data
backups = glob.glob(f"{tmp_dir}/backups/open-webui/*.bak")
assert backups, "expected an Open WebUI database backup"
PY
