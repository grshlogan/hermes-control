#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(CDPATH= cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck disable=SC1091
source "${SCRIPT_DIR}/hermes-control-common.sh"

hc_require_root

served_model_name="${1:-}"
if [[ -z "$served_model_name" ]]; then
  echo "served model name is required" >&2
  exit 2
fi

mapfile -t pids < <(hc_vllm_find_pids "$served_model_name")
if [[ "${#pids[@]}" -eq 0 ]]; then
  hc_vllm_json "stopped" "No vLLM process was running." "$served_model_name"
  exit 0
fi

kill -TERM "${pids[@]}" 2>/dev/null || true

for _ in {1..20}; do
  mapfile -t remaining < <(hc_vllm_find_pids "$served_model_name")
  if [[ "${#remaining[@]}" -eq 0 ]]; then
    hc_vllm_json "stopped" "vLLM process was stopped." "$served_model_name"
    exit 0
  fi
  sleep 1
done

hc_vllm_json "stopping" "vLLM process did not exit after SIGTERM." "$served_model_name"
exit 1
