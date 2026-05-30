#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(CDPATH= cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck disable=SC1091
source "${SCRIPT_DIR}/hermes-control-common.sh"

hc_require_root
hc_prepare_dirs

mapfile -t pids < <(hc_openwebui_find_pids)
if [[ "${#pids[@]}" -eq 0 ]]; then
  rm -f "$OPENWEBUI_PID_FILE"
  hc_openwebui_json "open_webui_not_running" "No Open WebUI process is running."
  exit 0
fi

kill -TERM "${pids[@]}" 2>/dev/null || true
for _ in {1..20}; do
  mapfile -t remaining < <(hc_openwebui_find_pids)
  if [[ "${#remaining[@]}" -eq 0 ]]; then
    rm -f "$OPENWEBUI_PID_FILE"
    hc_openwebui_json "open_webui_stopped" "Open WebUI process stopped."
    exit 0
  fi
  sleep 1
done

hc_openwebui_json "open_webui_stopping" "Open WebUI process did not exit after SIGTERM."
exit 1
