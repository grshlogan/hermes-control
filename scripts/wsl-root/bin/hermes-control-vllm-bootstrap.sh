#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(CDPATH= cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck disable=SC1091
source "${SCRIPT_DIR}/hermes-control-common.sh"

hc_require_root
hc_prepare_dirs

variant="${1:-}"
served_model_name="$(hc_vllm_served_model_for_variant "$variant")" || {
  echo "Unknown vLLM variant: ${variant}" >&2
  exit 2
}

if [[ ! -f "$VLLM_BOOTSTRAP_SCRIPT" ]]; then
  echo "vLLM bootstrap script does not exist: ${VLLM_BOOTSTRAP_SCRIPT}" >&2
  exit 1
fi

bash "$VLLM_BOOTSTRAP_SCRIPT"
hc_vllm_json "installed" "vLLM runtime bootstrap completed." "$served_model_name"
