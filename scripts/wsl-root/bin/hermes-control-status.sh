#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(CDPATH= cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck disable=SC1091
source "${SCRIPT_DIR}/hermes-control-common.sh"

hc_require_root

if hc_health_ok; then
  hc_status_json "running" "Hermes health endpoint is ready."
else
  hc_status_json "not_ready" "Hermes health endpoint is not ready."
fi
