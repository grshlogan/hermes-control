#!/usr/bin/env bash
set -euo pipefail

PROJECT_ROOT="$(CDPATH= cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)"
tmp_dir="$(mktemp -d)"
trap 'rm -rf "$tmp_dir"' EXIT

sandbox_bin="${tmp_dir}/bin"
env_file="${tmp_dir}/hermes.env"
data_dir="${tmp_dir}/open-webui-data"
db_file="${data_dir}/webui.db"
refresh_calls="${tmp_dir}/refresh-calls.txt"
mkdir -p "$sandbox_bin" "$data_dir"
cp "${PROJECT_ROOT}/scripts/wsl-root/bin/"*.sh "$sandbox_bin/"

cat > "${sandbox_bin}/hermes-control-restart.sh" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
printf '{"state":"restarted"}\n'
SH

cat > "${sandbox_bin}/hermes-control-health.sh" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
printf '{"state":"ready","http_ready":true}\n'
SH

cat > "${sandbox_bin}/hermes-control-openwebui-refresh.sh" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "${1:-}" >> "${OPENWEBUI_REFRESH_CALLS_FILE:?}"
case "${1:-}" in
  if-running)
    printf '{"state":"open_webui_error","detail":"simulated refresh failure"}\n' >&2
    exit 1
    ;;
  force)
    printf '{"state":"open_webui_restarted","detail":"restored previous Open WebUI process"}\n'
    exit 0
    ;;
  *)
    exit 2
    ;;
esac
SH

chmod +x "${sandbox_bin}/"*.sh

cat > "$env_file" <<'ENV'
API_SERVER_KEY=local-hermes-secret
LM_BASE_URL=https://old.example/v1
LM_MODEL=old-model
HERMES_CONTROL_ACTIVE_PROFILE_ID=external.old
ENV
chmod 0600 "$env_file"

python3 - "$db_file" <<'PY'
import json
import sqlite3
import sys

connection = sqlite3.connect(sys.argv[1])
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
                "openai": {
                    "enable": True,
                    "api_base_urls": ["https://old.example/v1"],
                    "api_keys": ["old-key"],
                    "api_configs": {"0": {}},
                },
                "ui": {"default_models": "old-model"},
            }
        ),
    ),
)
connection.commit()
PY

set +e
output="$(
  HERMES_CONTROL_CONFIG_FILE="/dev/null" \
  HERMES_CONTROL_WORK_ROOT="$tmp_dir" \
  HERMES_ENV_FILE="$env_file" \
  OPENWEBUI_DATA_DIR="$data_dir" \
  OPENWEBUI_DB_FILE="$db_file" \
  OPENWEBUI_BACKUP_DIR="${tmp_dir}/backups/open-webui" \
  OPENWEBUI_REFRESH_CALLS_FILE="$refresh_calls" \
  "$sandbox_bin/hermes-control-route-apply.sh" \
    external.next openai-compatible https://new.example/v1 new-model API_SERVER_KEY \
    2>&1
)"
status="$?"
set -e

if [[ "$status" -eq 0 ]]; then
  echo "route apply should have failed after simulated Open WebUI refresh failure" >&2
  echo "$output" >&2
  exit 1
fi

ROUTE_APPLY_OUTPUT="$output" python3 - "$env_file" "$db_file" "$refresh_calls" <<'PY'
import json
import os
import sqlite3
import sys
from pathlib import Path

env_file = Path(sys.argv[1])
db_file = sys.argv[2]
refresh_calls = Path(sys.argv[3])
output = os.environ["ROUTE_APPLY_OUTPUT"]

assert "local-hermes-secret" not in output, output
env_values = {}
for raw_line in env_file.read_text(encoding="utf-8").splitlines():
    if "=" in raw_line and not raw_line.lstrip().startswith("#"):
        key, value = raw_line.split("=", 1)
        env_values[key] = value

assert env_values["LM_BASE_URL"] == "https://old.example/v1", env_values
assert env_values["LM_MODEL"] == "old-model", env_values
assert env_values["HERMES_CONTROL_ACTIVE_PROFILE_ID"] == "external.old", env_values

data = json.loads(sqlite3.connect(db_file).execute("SELECT data FROM config WHERE id = 1").fetchone()[0])
assert data["openai"]["api_base_urls"] == ["https://old.example/v1"], data
assert data["openai"]["api_keys"] == ["old-key"], data
assert data["ui"]["default_models"] == "old-model", data
assert refresh_calls.read_text(encoding="utf-8").splitlines() == ["if-running", "force"]
PY
