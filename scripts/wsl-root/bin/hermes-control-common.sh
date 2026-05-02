#!/usr/bin/env bash
set -euo pipefail

HERMES_CONTROL_CONFIG_FILE="${HERMES_CONTROL_CONFIG_FILE:-/etc/hermes-control/runtime.env}"
if [[ -f "$HERMES_CONTROL_CONFIG_FILE" ]]; then
  set -a
  # shellcheck disable=SC1090
  source "$HERMES_CONTROL_CONFIG_FILE"
  set +a
fi

: "${HERMES_CONTROL_WORK_ROOT:=/root/Hermres}"
: "${HERMES_AGENT_ROOT:=${HERMES_CONTROL_WORK_ROOT}/hermes-agent}"
: "${HERMES_VENV_BIN:=${HERMES_AGENT_ROOT}/.venv-hermes/bin/hermes}"
: "${HERMES_HEALTH_URL:=http://127.0.0.1:8642/health}"
: "${HERMES_HEALTH_TIMEOUT_SECONDS:=30}"
: "${HERMES_LOG_DIR:=${HERMES_CONTROL_WORK_ROOT}/logs}"
: "${HERMES_PID_DIR:=/run/hermes-control}"
: "${HERMES_PID_FILE:=${HERMES_PID_DIR}/hermes-gateway.pid}"
: "${HERMES_ENV_FILE:=${HOME}/.hermes/.env}"
: "${VLLM_WORKSPACE:=/mnt/e/WSL/vLLM}"
: "${VLLM_MODELS_ENDPOINT:=http://127.0.0.1:18080/v1/models}"
: "${VLLM_LOG_DIR:=${VLLM_WORKSPACE}/logs}"
: "${VLLM_PID_DIR:=/run/hermes-control}"
: "${VLLM_START_QWEN36_MTP:=${VLLM_WORKSPACE}/scripts/start-qwen36-mtp.sh}"
: "${VLLM_START_QWEN36_AWQ_INT4:=${VLLM_WORKSPACE}/scripts/start-qwen36-int4-eager.sh}"

hc_require_root() {
  if [[ "$(id -u)" != "0" ]]; then
    echo "Hermes Control WSL helpers must run as root." >&2
    exit 1
  fi
}

hc_prepare_dirs() {
  mkdir -p "$HERMES_LOG_DIR" "$HERMES_PID_DIR" "$VLLM_LOG_DIR" "$VLLM_PID_DIR"
}

hc_load_hermes_env() {
  if [[ -f "$HERMES_ENV_FILE" ]]; then
    set -a
    # shellcheck disable=SC1090
    source "$HERMES_ENV_FILE"
    set +a
  fi
}

hc_health_ok() {
  curl -fsS --max-time 2 "$HERMES_HEALTH_URL" >/dev/null 2>&1
}

hc_find_pids() {
  python3 - "$HERMES_VENV_BIN" "$HERMES_PID_FILE" <<'PY'
import os
import sys

needle = sys.argv[1]
pid_file = sys.argv[2]
self_pids = {os.getpid(), os.getppid()}
pids = set()

def add_pid(value):
    try:
        pid = int(value)
    except (TypeError, ValueError):
        return
    if pid in self_pids:
        return
    if os.path.exists(f"/proc/{pid}"):
        pids.add(pid)

try:
    with open(pid_file, "r", encoding="utf-8") as handle:
        add_pid(handle.read().strip())
except OSError:
    pass

try:
    proc_entries = os.listdir("/proc")
except OSError:
    proc_entries = []

for entry in proc_entries:
    if not entry.isdigit():
        continue
    try:
        with open(f"/proc/{entry}/cmdline", "rb") as handle:
            cmdline = handle.read().replace(b"\0", b" ").decode("utf-8", "ignore").strip()
    except OSError:
        continue
    if needle in cmdline and " gateway run" in cmdline:
        add_pid(entry)

for pid in sorted(pids):
    print(pid)
PY
}

hc_status_json() {
  local state="$1"
  local detail="${2:-}"
  local health="false"
  if hc_health_ok; then
    health="true"
  fi

  local pids
  pids="$(hc_find_pids | paste -sd, -)"

  HERMES_CONTROL_STATE="$state" \
  HERMES_CONTROL_DETAIL="$detail" \
  HERMES_CONTROL_HEALTH="$health" \
  HERMES_CONTROL_PIDS="$pids" \
  HERMES_HEALTH_URL="$HERMES_HEALTH_URL" \
  HERMES_VENV_BIN="$HERMES_VENV_BIN" \
  python3 - <<'PY'
import json
import os

pids_text = os.environ.get("HERMES_CONTROL_PIDS", "")
pids = [int(pid) for pid in pids_text.split(",") if pid]
print(json.dumps({
    "state": os.environ["HERMES_CONTROL_STATE"],
    "detail": os.environ.get("HERMES_CONTROL_DETAIL", ""),
    "health_url": os.environ["HERMES_HEALTH_URL"],
    "http_ready": os.environ["HERMES_CONTROL_HEALTH"] == "true",
    "pids": pids,
    "executable": os.environ["HERMES_VENV_BIN"],
}, sort_keys=True))
PY
}

hc_fail() {
  local detail="$1"
  hc_status_json "error" "$detail" >&2
  exit 1
}

hc_vllm_served_model_for_variant() {
  case "${1:-}" in
    qwen36-mtp) printf '%s\n' "qwen36-mtp" ;;
    qwen36-awq-int4) printf '%s\n' "qwen36-awq-int4" ;;
    *) return 1 ;;
  esac
}

hc_vllm_start_script_for_variant() {
  case "${1:-}" in
    qwen36-mtp) printf '%s\n' "$VLLM_START_QWEN36_MTP" ;;
    qwen36-awq-int4) printf '%s\n' "$VLLM_START_QWEN36_AWQ_INT4" ;;
    *) return 1 ;;
  esac
}

hc_vllm_health_ok() {
  local served_model_name="$1"
  local body
  body="$(curl -fsS --max-time "${VLLM_HEALTH_CURL_TIMEOUT_SECONDS:-1}" "$VLLM_MODELS_ENDPOINT" 2>/dev/null)" || return 1
  printf '%s' "$body" | python3 - "$served_model_name" <<'PY'
import json
import sys

served_model_name = sys.argv[1]
try:
    payload = json.load(sys.stdin)
except Exception:
    sys.exit(1)

for item in payload.get("data", []):
    if item.get("id") == served_model_name:
        sys.exit(0)
sys.exit(1)
PY
}

hc_vllm_find_pids() {
  local served_model_name="$1"
  python3 - "$served_model_name" <<'PY'
import os
import sys

served_model_name = sys.argv[1]
self_pids = {os.getpid(), os.getppid()}
pids = set()

for entry in os.listdir("/proc"):
    if not entry.isdigit():
        continue
    pid = int(entry)
    if pid in self_pids:
        continue
    try:
        with open(f"/proc/{entry}/cmdline", "rb") as handle:
            cmdline = handle.read().replace(b"\0", b" ").decode("utf-8", "ignore").strip()
    except OSError:
        continue
    if served_model_name in cmdline and ("vllm" in cmdline or "serve-openai" in cmdline):
        pids.add(pid)

for pid in sorted(pids):
    print(pid)
PY
}

hc_vllm_json() {
  local state="$1"
  local detail="${2:-}"
  local served_model_name="${3:-}"
  local ready="false"
  if [[ -n "$served_model_name" ]] && hc_vllm_health_ok "$served_model_name"; then
    ready="true"
  fi

  VLLM_CONTROL_STATE="$state" \
  VLLM_CONTROL_DETAIL="$detail" \
  VLLM_SERVED_MODEL_NAME="$served_model_name" \
  VLLM_MODELS_ENDPOINT="$VLLM_MODELS_ENDPOINT" \
  VLLM_READY="$ready" \
  python3 - <<'PY'
import json
import os

print(json.dumps({
    "state": os.environ["VLLM_CONTROL_STATE"],
    "detail": os.environ.get("VLLM_CONTROL_DETAIL", ""),
    "served_model_name": os.environ.get("VLLM_SERVED_MODEL_NAME", ""),
    "models_endpoint": os.environ["VLLM_MODELS_ENDPOINT"],
    "ready": os.environ["VLLM_READY"] == "true",
}, sort_keys=True))
PY
}
