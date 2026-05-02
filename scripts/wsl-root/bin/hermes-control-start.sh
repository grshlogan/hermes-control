#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(CDPATH= cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck disable=SC1091
source "${SCRIPT_DIR}/hermes-control-common.sh"

hc_require_root
hc_prepare_dirs

if hc_health_ok; then
  hc_status_json "running" "Hermes health endpoint is already ready."
  exit 0
fi

if [[ ! -x "$HERMES_VENV_BIN" ]]; then
  hc_fail "Hermes executable is not executable: ${HERMES_VENV_BIN}"
fi

if [[ ! -d "$HERMES_AGENT_ROOT" ]]; then
  hc_fail "Hermes agent root does not exist: ${HERMES_AGENT_ROOT}"
fi

hc_load_hermes_env

cd "$HERMES_AGENT_ROOT"
out_log="${HERMES_LOG_DIR}/hermes-gateway.out.log"
err_log="${HERMES_LOG_DIR}/hermes-gateway.err.log"

if command -v setsid >/dev/null 2>&1; then
  setsid "$HERMES_VENV_BIN" gateway run --replace >>"$out_log" 2>>"$err_log" </dev/null &
else
  nohup "$HERMES_VENV_BIN" gateway run --replace >>"$out_log" 2>>"$err_log" </dev/null &
fi

pid="$!"
printf '%s\n' "$pid" > "$HERMES_PID_FILE"
sleep "${HERMES_START_SETTLE_SECONDS:-2}"

if ! kill -0 "$pid" 2>/dev/null; then
  hc_fail "Hermes gateway exited during startup. Check ${out_log} and ${err_log}."
fi

hc_status_json "started" "Hermes gateway process was started."
