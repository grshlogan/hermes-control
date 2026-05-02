#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(CDPATH= cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck disable=SC1091
source "${SCRIPT_DIR}/hermes-control-common.sh"

hc_require_root

variant="${1:-}"
if [[ -z "$variant" ]]; then
  echo "variant is required" >&2
  exit 2
fi

echo "vLLM benchmark helper for ${variant} is reserved for the next Phase 5 increment." >&2
exit 2
