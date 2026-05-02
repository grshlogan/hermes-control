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

hc_require_root() {
  if [[ "$(id -u)" != "0" ]]; then
    echo "Hermes Control WSL helpers must run as root." >&2
    exit 1
  fi
}

hc_prepare_dirs() {
  mkdir -p "$HERMES_LOG_DIR" "$HERMES_PID_DIR"
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
