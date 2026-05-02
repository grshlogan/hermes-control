#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(CDPATH= cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=env.sh
source "$SCRIPT_DIR/env.sh"

MODEL="${MODEL:-${VLLM_MODEL_ROOT}/cyankiwi/Qwen3.6-27B-AWQ-INT4}"
STAMP="${STAMP:-$(date +%Y%m%d-%H%M%S)}"
LOG_FILE="${LOG_FILE:-${VLLM_LOG_DIR}/live-qwen36-awq-int4-eager-${STAMP}.log}"
RUNLOG="${RUNLOG:-${LOG_FILE}.run}"

export VLLM_MODEL="$MODEL"
DEFAULT_HOST="${DEFAULT_HOST:-$(hostname -I 2>/dev/null | awk '{print $1}')}"
DEFAULT_HOST="${DEFAULT_HOST:-127.0.0.1}"
export HOST="${HOST:-$DEFAULT_HOST}"
export PORT="${PORT:-18080}"
export SERVED_MODEL_NAME="${SERVED_MODEL_NAME:-qwen36-awq-int4}"
export MAX_MODEL_LEN="${MAX_MODEL_LEN:-90000}"
export TP="${TP:-2}"
export GPU_MEMORY_UTILIZATION="${GPU_MEMORY_UTILIZATION:-0.86}"
export CPU_OFFLOAD_GB="${CPU_OFFLOAD_GB:-0}"
export ENFORCE_EAGER="${ENFORCE_EAGER:-1}"
export QUANTIZATION="${QUANTIZATION:-compressed-tensors}"
export LOG_FILE
export CUDA_DEVICE_ORDER="${CUDA_DEVICE_ORDER:-PCI_BUS_ID}"
export PYTORCH_CUDA_ALLOC_CONF="${PYTORCH_CUDA_ALLOC_CONF:-expandable_segments:True}"

exec bash "$SCRIPT_DIR/serve-openai.sh" \
  --disable-custom-all-reduce \
  --generation-config vllm \
  --max-num-seqs "${MAX_NUM_SEQS:-1}" \
  --max-num-batched-tokens "${MAX_NUM_BATCHED_TOKENS:-4096}" \
  "$@" >>"$RUNLOG" 2>&1
