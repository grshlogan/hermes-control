#!/usr/bin/env bash
set -euo pipefail

PROJECT_ROOT="$(CDPATH= cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)"
tmp_dir="$(mktemp -d)"
trap 'rm -rf "$tmp_dir"' EXIT

sandbox_bin="${tmp_dir}/bin"
env_file="${tmp_dir}/hermes.env"
data_dir="${tmp_dir}/open-webui-data"
db_file="${data_dir}/webui.db"
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

cat > "${sandbox_bin}/hermes-control-openwebui-sync.sh" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
printf '{"state":"open_webui_synced","db_file":"%s","backup_file":"%s"}\n' \
  "${OPENWEBUI_DB_FILE:?}" \
  "${OPENWEBUI_DB_FILE}.bak"
SH

cat > "${sandbox_bin}/hermes-control-openwebui-refresh.sh" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
printf '{"state":"open_webui_not_running","http_ready":false}\n'
SH

chmod +x "${sandbox_bin}/"*.sh

cat > "$env_file" <<'ENV'
DEEPSEEK_API_KEY=deepseek-secret-token
LM_API_KEY=old-secret-token
LM_BASE_URL=https://old-relay.example/v1
LM_MODEL=old-model
ENV
chmod 0600 "$env_file"
touch "$db_file"

output="$(
  HERMES_CONTROL_CONFIG_FILE="/dev/null" \
  HERMES_CONTROL_WORK_ROOT="$tmp_dir" \
  HERMES_ENV_FILE="$env_file" \
  API_TIMEOUT_MS="600000" \
  OPENWEBUI_DATA_DIR="$data_dir" \
  OPENWEBUI_DB_FILE="$db_file" \
  OPENWEBUI_BACKUP_DIR="${tmp_dir}/backups/open-webui" \
  "$sandbox_bin/hermes-control-route-apply.sh" \
    deepseek.api deepseek https://api.deepseek.com/v1 deepseek-chat DEEPSEEK_API_KEY
)"

ROUTE_APPLY_OUTPUT="$output" python3 - "$env_file" <<'PY'
import json
import os
import sys
from pathlib import Path

env_file = Path(sys.argv[1])
output = os.environ["ROUTE_APPLY_OUTPUT"]
payload = json.loads(output.strip().splitlines()[-1])

assert payload["state"] == "route_applied", payload
assert payload["provider_kind"] == "deepseek", payload
assert payload["secret_env_key"] == "DEEPSEEK_API_KEY", payload
assert "deepseek-secret-token" not in output, output

env_values = {}
for raw_line in env_file.read_text(encoding="utf-8").splitlines():
    if "=" in raw_line and not raw_line.lstrip().startswith("#"):
        key, value = raw_line.split("=", 1)
        env_values[key] = value

assert env_values["DEEPSEEK_API_KEY"] == "deepseek-secret-token", env_values
assert env_values["LM_API_KEY"] == "deepseek-secret-token", env_values
assert env_values["LM_BASE_URL"] == "https://api.deepseek.com/v1", env_values
assert env_values["LM_MODEL"] == "deepseek-chat", env_values
assert env_values["API_TIMEOUT_MS"] == "600000", env_values
assert "ANTHROPIC_AUTH_TOKEN" not in env_values, env_values
PY
