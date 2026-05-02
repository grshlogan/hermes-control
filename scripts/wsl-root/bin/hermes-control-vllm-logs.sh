#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(CDPATH= cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck disable=SC1091
source "${SCRIPT_DIR}/hermes-control-common.sh"

hc_require_root

variant="${1:-}"
line_count="${2:-200}"

if [[ -z "$variant" || ! "$line_count" =~ ^[0-9]+$ ]]; then
  echo "usage: hermes-control-vllm-logs.sh <variant> <line-count>" >&2
  exit 2
fi

latest_log="$(find "$VLLM_LOG_DIR" -maxdepth 1 -type f -name "*${variant}*.log*" -printf '%T@ %p\n' 2>/dev/null | sort -nr | awk 'NR == 1 {print $2}')"
if [[ -z "$latest_log" ]]; then
  echo "No log file found for ${variant} in ${VLLM_LOG_DIR}" >&2
  exit 1
fi

tail -n "$line_count" "$latest_log"
