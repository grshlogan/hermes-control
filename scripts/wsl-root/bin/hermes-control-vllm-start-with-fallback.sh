#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(CDPATH= cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck disable=SC1091
source "${SCRIPT_DIR}/hermes-control-common.sh"

hc_require_root
hc_prepare_dirs

primary_variant="${1:-}"
fallback_variant="${2:-}"

if [[ -z "$primary_variant" || -z "$fallback_variant" ]]; then
  echo "usage: hermes-control-vllm-start-with-fallback.sh <primary-variant> <fallback-variant>" >&2
  exit 2
fi
if [[ "$primary_variant" == "$fallback_variant" ]]; then
  echo "primary and fallback variants must be different" >&2
  exit 2
fi

primary_served_model="$(hc_vllm_served_model_for_variant "$primary_variant")" || {
  echo "Unknown primary vLLM variant: ${primary_variant}" >&2
  exit 2
}
fallback_served_model="$(hc_vllm_served_model_for_variant "$fallback_variant")" || {
  echo "Unknown fallback vLLM variant: ${fallback_variant}" >&2
  exit 2
}

wait_for_vllm_ready() {
  local served_model_name="$1"
  local timeout="$2"
  local deadline=$((SECONDS + timeout))

  while (( SECONDS <= deadline )); do
    if hc_vllm_health_ok "$served_model_name"; then
      return 0
    fi
    sleep 1
  done

  return 1
}

start_variant() {
  local variant="$1"
  "${SCRIPT_DIR}/hermes-control-vllm-start.sh" "$variant"
}

primary_timeout="${VLLM_FALLBACK_PRIMARY_TIMEOUT_SECONDS:-180}"
fallback_timeout="${VLLM_FALLBACK_SECONDARY_TIMEOUT_SECONDS:-180}"

if hc_vllm_health_ok "$primary_served_model"; then
  hc_vllm_json "running" "Primary vLLM model is already ready." "$primary_served_model"
  exit 0
fi

if hc_vllm_health_ok "$fallback_served_model"; then
  hc_vllm_json "running" "Fallback vLLM model is already ready." "$fallback_served_model"
  exit 0
fi

if start_variant "$primary_variant"; then
  if wait_for_vllm_ready "$primary_served_model" "$primary_timeout"; then
    hc_vllm_json "ready" "Primary vLLM model became ready." "$primary_served_model"
    exit 0
  fi
else
  echo "Primary vLLM start failed for ${primary_variant}; attempting fallback ${fallback_variant}." >&2
fi

"${SCRIPT_DIR}/hermes-control-vllm-stop.sh" "$primary_served_model" >/dev/null 2>&1 || true

if ! start_variant "$fallback_variant"; then
  hc_vllm_json "unhealthy" "Fallback vLLM start failed after primary startup failure." "$fallback_served_model" >&2
  exit 1
fi

if wait_for_vllm_ready "$fallback_served_model" "$fallback_timeout"; then
  hc_vllm_json "ready" "Fallback vLLM model became ready after primary startup failure." "$fallback_served_model"
  exit 0
fi

hc_vllm_json "unhealthy" "Neither primary nor fallback vLLM model became ready before timeout." "$fallback_served_model" >&2
exit 1
