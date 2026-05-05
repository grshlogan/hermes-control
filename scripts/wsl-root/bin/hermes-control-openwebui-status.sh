#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(CDPATH= cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck disable=SC1091
source "${SCRIPT_DIR}/hermes-control-common.sh"

hc_require_root

mapfile -t pids < <(hc_openwebui_find_pids)
if [[ "${#pids[@]}" -eq 0 ]]; then
  rm -f "$OPENWEBUI_PID_FILE"
  hc_openwebui_json "open_webui_not_running" "No Open WebUI process is running."
else
  hc_openwebui_json "open_webui_running" "Open WebUI process is running."
fi
