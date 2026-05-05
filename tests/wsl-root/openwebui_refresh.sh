#!/usr/bin/env bash
set -euo pipefail

PROJECT_ROOT="$(CDPATH= cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)"
tmp_dir="$(mktemp -d)"
cleanup_pids=()
cleanup() {
  for pid in "${cleanup_pids[@]:-}"; do
    kill "$pid" >/dev/null 2>&1 || true
  done
  rm -rf "$tmp_dir"
}
trap cleanup EXIT

env_file="${tmp_dir}/hermes.env"
data_dir="${tmp_dir}/open-webui-data"
pid_file="${tmp_dir}/open-webui.pid"
capture_file="${tmp_dir}/open-webui-env.txt"
fake_bin="${tmp_dir}/open-webui"
mkdir -p "$data_dir"

cat > "$env_file" <<'ENV'
API_SERVER_KEY=local-hermes-secret
ENV
chmod 0600 "$env_file"

cat > "$fake_bin" <<'SH'
#!/usr/bin/env bash
key_state="missing"
if [[ -n "${OPENAI_API_KEYS:-}" ]]; then
  key_state="present"
fi
{
  printf 'base_url=%s\n' "${OPENAI_API_BASE_URLS:-}"
  printf 'default_model=%s\n' "${DEFAULT_MODELS:-}"
  printf 'key_state=%s\n' "$key_state"
} > "${OPENWEBUI_CAPTURE_FILE:?}"
sleep 300
SH
chmod +x "$fake_bin"

base_env=(
  HERMES_CONTROL_CONFIG_FILE="/dev/null"
  HERMES_CONTROL_WORK_ROOT="$tmp_dir"
  HERMES_ENV_FILE="$env_file"
  OPENWEBUI_DATA_DIR="$data_dir"
  OPENWEBUI_PID_FILE="$pid_file"
  OPENWEBUI_VENV_BIN="$fake_bin"
  OPENWEBUI_CAPTURE_FILE="$capture_file"
  OPENWEBUI_HEALTH_TIMEOUT_SECONDS="0"
  OPENWEBUI_PORT="39999"
)

skip_output="$(env "${base_env[@]}" "${PROJECT_ROOT}/scripts/wsl-root/bin/hermes-control-openwebui-refresh.sh" if-running)"
OPENWEBUI_REFRESH_OUTPUT="$skip_output" python3 - <<'PY'
import json
import os

payload = json.loads(os.environ["OPENWEBUI_REFRESH_OUTPUT"])
assert payload["state"] == "open_webui_not_running", payload
assert payload["http_ready"] is False, payload
PY

sleep 300 &
old_pid="$!"
cleanup_pids+=("$old_pid")
printf '%s\n' "$old_pid" > "$pid_file"

refresh_output="$(env "${base_env[@]}" "${PROJECT_ROOT}/scripts/wsl-root/bin/hermes-control-openwebui-refresh.sh" if-running)"
OPENWEBUI_REFRESH_OUTPUT="$refresh_output" python3 - "$old_pid" "$pid_file" "$capture_file" <<'PY'
import json
import os
import signal
import sys
import time
from pathlib import Path

old_pid = int(sys.argv[1])
pid_file = Path(sys.argv[2])
capture_file = Path(sys.argv[3])
payload = json.loads(os.environ["OPENWEBUI_REFRESH_OUTPUT"])
assert payload["state"] in {"open_webui_restarted", "open_webui_restarted_not_ready"}, payload
assert payload["http_ready"] is False, payload
assert payload["pids"], payload
assert "local-hermes-secret" not in os.environ["OPENWEBUI_REFRESH_OUTPUT"], payload

for _ in range(20):
    if capture_file.exists():
        break
    time.sleep(0.1)

assert capture_file.exists(), "fake Open WebUI did not capture startup env"
captured = capture_file.read_text(encoding="utf-8")
assert "base_url=http://127.0.0.1:8642/v1" in captured, captured
assert "default_model=hermes-agent" in captured, captured
assert "key_state=present" in captured, captured
assert "local-hermes-secret" not in captured, captured

new_pid = int(pid_file.read_text(encoding="utf-8").strip())
assert new_pid != old_pid, (old_pid, new_pid)
os.kill(new_pid, signal.SIGTERM)
PY
