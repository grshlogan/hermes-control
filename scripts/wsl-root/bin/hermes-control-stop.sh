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

kill -TERM "${pids[@]}" 2>/dev/null || true

for _ in {1..15}; do
  mapfile -t remaining < <(hc_find_pids)
  if [[ "${#remaining[@]}" -eq 0 ]]; then
    rm -f "$HERMES_PID_FILE"
    hc_status_json "stopped" "Hermes gateway process was stopped."
    exit 0
  fi
  sleep 1
done

hc_status_json "stopping" "Hermes gateway did not exit after SIGTERM."
exit 1
