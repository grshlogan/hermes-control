#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(CDPATH= cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck disable=SC1091
source "${SCRIPT_DIR}/hermes-control-common.sh"

hc_require_root

timeout="${1:-$HERMES_HEALTH_TIMEOUT_SECONDS}"
mode="${2:-ready}"

if [[ ! "$timeout" =~ ^[0-9]+$ ]]; then
  hc_fail "Health timeout must be a whole number of seconds."
fi

deadline=$((SECONDS + timeout))
while (( SECONDS <= deadline )); do
  if hc_health_ok; then
    hc_status_json "$mode" "Hermes health endpoint is ready."
    exit 0
  fi
  sleep 1
done

hc_status_json "unhealthy" "Hermes health endpoint did not become ready before timeout."
exit 1
