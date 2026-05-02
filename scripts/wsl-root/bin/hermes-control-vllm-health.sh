#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(CDPATH= cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck disable=SC1091
source "${SCRIPT_DIR}/hermes-control-common.sh"

hc_require_root

served_model_name="${1:-}"
timeout="${2:-30}"
mode="${3:-ready}"

if [[ -z "$served_model_name" ]]; then
  echo "served model name is required" >&2
  exit 2
fi
if [[ ! "$timeout" =~ ^[0-9]+$ ]]; then
  echo "timeout must be a whole number of seconds" >&2
  exit 2
fi

deadline=$((SECONDS + timeout))
while (( SECONDS <= deadline )); do
  if hc_vllm_health_ok "$served_model_name"; then
    hc_vllm_json "$mode" "vLLM served model is ready." "$served_model_name"
    exit 0
  fi
  sleep 1
done

hc_vllm_json "unhealthy" "vLLM served model did not become ready before timeout." "$served_model_name"
exit 1
