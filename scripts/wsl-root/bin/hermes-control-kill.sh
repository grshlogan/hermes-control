#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(CDPATH= cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck disable=SC1091
source "${SCRIPT_DIR}/hermes-control-common.sh"

hc_require_root

mapfile -t pids < <(hc_find_pids)
if [[ "${#pids[@]}" -eq 0 ]]; then
  rm -f "$HERMES_PID_FILE"
  hc_status_json "stopped" "No Hermes gateway process was running."
  exit 0
fi

kill -KILL "${pids[@]}" 2>/dev/null || true
rm -f "$HERMES_PID_FILE"
hc_status_json "killed" "Hermes gateway process was killed."
