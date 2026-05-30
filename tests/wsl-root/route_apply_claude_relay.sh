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
ANTHROPIC_AUTH_TOKEN=relay-secret-token
ANTHROPIC_BASE_URL=https://old-relay.example/
ANTHROPIC_MODEL=old-claude
ENV
chmod 0600 "$env_file"
touch "$db_file"

output="$(
  HERMES_CONTROL_CONFIG_FILE="/dev/null" \
  HERMES_CONTROL_WORK_ROOT="$tmp_dir" \
  HERMES_ENV_FILE="$env_file" \
  ANTHROPIC_DEFAULT_SONNET_MODEL="claude-sonnet-4-6" \
  ANTHROPIC_DEFAULT_HAIKU_MODEL="claude-haiku-4-5" \
  ANTHROPIC_DEFAULT_OPUS_MODEL="claude-opus-4-7" \
  API_TIMEOUT_MS="600000" \
  HTTPS_PROXY="http://127.0.0.1:7890" \
  NO_PROXY="127.0.0.1,localhost" \
  effortLevel="high" \
  OPENWEBUI_DATA_DIR="$data_dir" \
  OPENWEBUI_DB_FILE="$db_file" \
  OPENWEBUI_BACKUP_DIR="${tmp_dir}/backups/open-webui" \
  "$sandbox_bin/hermes-control-route-apply.sh" \
    external.api-relay claude https://api-relay.example.com/ claude-sonnet-4-6 ANTHROPIC_AUTH_TOKEN
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
assert payload["provider_kind"] == "claude", payload
assert payload["secret_env_key"] == "ANTHROPIC_AUTH_TOKEN", payload
assert "relay-secret-token" not in output, output

env_values = {}
for raw_line in env_file.read_text(encoding="utf-8").splitlines():
    if "=" in raw_line and not raw_line.lstrip().startswith("#"):
        key, value = raw_line.split("=", 1)
        env_values[key] = value

assert env_values["ANTHROPIC_AUTH_TOKEN"] == "relay-secret-token", env_values
assert env_values["ANTHROPIC_BASE_URL"] == "https://api-relay.example.com/", env_values
assert env_values["ANTHROPIC_MODEL"] == "claude-sonnet-4-6", env_values
assert env_values["ANTHROPIC_DEFAULT_SONNET_MODEL"] == "claude-sonnet-4-6", env_values
assert env_values["ANTHROPIC_DEFAULT_HAIKU_MODEL"] == "claude-haiku-4-5", env_values
assert env_values["ANTHROPIC_DEFAULT_OPUS_MODEL"] == "claude-opus-4-7", env_values
assert env_values["API_TIMEOUT_MS"] == "600000", env_values
assert env_values["HTTPS_PROXY"] == "http://127.0.0.1:7890", env_values
no_proxy_values = {
    value.strip()
    for value in env_values["NO_PROXY"].split(",")
    if value.strip()
}
assert {"127.0.0.1", "localhost"}.issubset(no_proxy_values), env_values
assert env_values["effortLevel"] == "high", env_values
assert env_values["LM_BASE_URL"] == "https://api-relay.example.com/", env_values
assert env_values["LM_MODEL"] == "claude-sonnet-4-6", env_values
PY
