#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(CDPATH= cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=env.sh
source "$SCRIPT_DIR/env.sh"

MODEL="${VLLM_MODEL:-}"
if [[ $# -gt 0 && "${1:0:2}" != "--" ]]; then
  MODEL="$1"
  shift
fi

if [[ -z "$MODEL" ]]; then
  echo "Set VLLM_MODEL or pass a model/path as the first argument." >&2
  exit 2
fi

if ! command -v vllm >/dev/null 2>&1; then
  echo "vllm command not found. Run ${SCRIPT_DIR}/bootstrap.sh first." >&2
  exit 1
fi

HOST="${HOST:-127.0.0.1}"
PORT="${PORT:-18080}"
TP="${TP:-1}"
GPU_MEMORY_UTILIZATION="${GPU_MEMORY_UTILIZATION:-0.86}"
DOWNLOAD_DIR="${DOWNLOAD_DIR:-$VLLM_MODEL_ROOT}"
SERVED_MODEL_NAME="${SERVED_MODEL_NAME:-$(basename "$MODEL")}"
LOG_FILE="${LOG_FILE:-$VLLM_LOG_DIR/vllm-serve-$(date +%Y%m%d-%H%M%S).log}"
CPU_OFFLOAD_GB="${CPU_OFFLOAD_GB:-0}"

args=(
  "$MODEL"
  --host "$HOST"
  --port "$PORT"
  --download-dir "$DOWNLOAD_DIR"
  --served-model-name "$SERVED_MODEL_NAME"
  --tensor-parallel-size "$TP"
  --gpu-memory-utilization "$GPU_MEMORY_UTILIZATION"
)

if [[ -n "${MAX_MODEL_LEN:-}" ]]; then
  args+=(--max-model-len "$MAX_MODEL_LEN")
fi

if [[ -n "${QUANTIZATION:-}" ]]; then
  args+=(--quantization "$QUANTIZATION")
fi

if [[ "$CPU_OFFLOAD_GB" != "0" ]]; then
  args+=(--cpu-offload-gb "$CPU_OFFLOAD_GB")
fi

if [[ "${ENFORCE_EAGER:-0}" == "1" || "${ENFORCE_EAGER:-}" == "true" ]]; then
  args+=(--enforce-eager)
fi

if [[ -n "${SPECULATIVE_CONFIG:-}" ]]; then
  args+=(--speculative-config "$SPECULATIVE_CONFIG")
fi

echo "workspace=$VLLM_WORKSPACE"
echo "venv=$VLLM_VENV"
echo "models=$VLLM_MODEL_ROOT"
echo "network=${VLLM_NET_RESOLVED:-$VLLM_NET}"
echo "log=$LOG_FILE"
echo "model=$MODEL"
echo "served_model_name=$SERVED_MODEL_NAME"
echo "host=$HOST port=$PORT tp=$TP gpu_memory_utilization=$GPU_MEMORY_UTILIZATION cpu_offload_gb=$CPU_OFFLOAD_GB"

vllm serve "${args[@]}" "$@" 2>&1 | tee -a "$LOG_FILE"
