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
start_script="$(hc_vllm_start_script_for_variant "$variant")"

if hc_vllm_health_ok "$served_model_name"; then
  hc_vllm_json "running" "vLLM model is already ready." "$served_model_name"
  exit 0
fi

if [[ ! -f "$start_script" ]]; then
  echo "vLLM start script does not exist: ${start_script}" >&2
  exit 1
fi

stamp="$(date +%Y%m%d-%H%M%S)"
stdout_log="${VLLM_LOG_DIR}/hermes-control-${variant}-${stamp}.out.log"
stderr_log="${VLLM_LOG_DIR}/hermes-control-${variant}-${stamp}.err.log"
pid_file="${VLLM_PID_DIR}/vllm-${variant}.pid"

if command -v setsid >/dev/null 2>&1; then
  setsid bash "$start_script" >>"$stdout_log" 2>>"$stderr_log" </dev/null &
else
  nohup bash "$start_script" >>"$stdout_log" 2>>"$stderr_log" </dev/null &
fi

pid="$!"
printf '%s\n' "$pid" > "$pid_file"
sleep "${VLLM_START_SETTLE_SECONDS:-2}"

if ! kill -0 "$pid" 2>/dev/null; then
  echo "vLLM exited during startup. Check ${stdout_log} and ${stderr_log}." >&2
  exit 1
fi

hc_vllm_json "started" "vLLM start command launched." "$served_model_name"
