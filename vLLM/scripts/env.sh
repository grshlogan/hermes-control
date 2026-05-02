#!/usr/bin/env bash
set -euo pipefail

DEFAULT_VLLM_WORKSPACE="/mnt/e/WSL/Hermres/hermes-control/vLLM"
DEFAULT_VLLM_MODEL_ROOT="/mnt/e/WSL/vLLM/models"

export VLLM_WORKSPACE="${VLLM_WORKSPACE:-$DEFAULT_VLLM_WORKSPACE}"
export VLLM_MODEL_ROOT="${VLLM_MODEL_ROOT:-$DEFAULT_VLLM_MODEL_ROOT}"
export VLLM_VENV="${VLLM_VENV:-${VLLM_WORKSPACE}/.venv}"
export VLLM_LOG_DIR="${VLLM_LOG_DIR:-${VLLM_WORKSPACE}/logs}"
export VLLM_CACHE_DIR="${VLLM_CACHE_DIR:-${VLLM_WORKSPACE}/cache}"
export VLLM_DOWNLOAD_DIR="${VLLM_DOWNLOAD_DIR:-${VLLM_WORKSPACE}/downloads}"
export VLLM_TMPDIR="${VLLM_TMPDIR:-/tmp/hermes-control-vllm}"

mkdir -p \
  "$VLLM_WORKSPACE" \
  "$VLLM_MODEL_ROOT" \
  "$VLLM_LOG_DIR" \
  "$VLLM_CACHE_DIR/uv" \
  "$VLLM_CACHE_DIR/pip" \
  "$VLLM_CACHE_DIR/huggingface" \
  "$VLLM_CACHE_DIR/torch" \
  "$VLLM_CACHE_DIR/vllm" \
  "$VLLM_DOWNLOAD_DIR" \
  "$VLLM_TMPDIR"

export XDG_CACHE_HOME="$VLLM_CACHE_DIR"
export UV_CACHE_DIR="$VLLM_CACHE_DIR/uv"
export HF_HOME="$VLLM_CACHE_DIR/huggingface"
export HUGGINGFACE_HUB_CACHE="$HF_HOME/hub"
export TRANSFORMERS_CACHE="$HF_HOME/transformers"
export TORCH_HOME="$VLLM_CACHE_DIR/torch"
export VLLM_CACHE_ROOT="$VLLM_CACHE_DIR/vllm"
export UV_LINK_MODE="${UV_LINK_MODE:-copy}"
export HF_HUB_DISABLE_TELEMETRY="${HF_HUB_DISABLE_TELEMETRY:-1}"
export CUDA_DEVICE_ORDER="${CUDA_DEVICE_ORDER:-PCI_BUS_ID}"

# vLLM can create Unix sockets under TMPDIR. Keep the default on the WSL
# filesystem because DrvFS paths such as /mnt/e can reject Unix sockets.
export TMPDIR="$VLLM_TMPDIR"

_hc_vllm_pip_cache_dir() {
  local candidate="$VLLM_CACHE_DIR/pip"
  local candidate_uid
  candidate_uid="$(stat -c '%u' "$candidate" 2>/dev/null || true)"
  if [[ -n "$candidate_uid" && "$candidate_uid" == "$(id -u)" ]]; then
    printf '%s\n' "$candidate"
    return
  fi

  local fallback="${VLLM_PIP_CACHE_FALLBACK:-${TMPDIR}/pip-cache}"
  mkdir -p "$fallback"
  printf '%s\n' "$fallback"
}

export PIP_CACHE_DIR="${PIP_CACHE_DIR:-$(_hc_vllm_pip_cache_dir)}"

export NO_PROXY="${NO_PROXY:-localhost,127.0.0.1,::1}"
export no_proxy="${no_proxy:-$NO_PROXY}"

export VLLM_NET="${VLLM_NET:-auto}"
export VLLM_PROXY_FALLBACK="${VLLM_PROXY_FALLBACK:-http://127.0.0.1:7890}"
_saved_http_proxy="${http_proxy:-${HTTP_PROXY:-}}"
_saved_https_proxy="${https_proxy:-${HTTPS_PROXY:-$_saved_http_proxy}}"
_chosen_proxy="${VLLM_PROXY:-${_saved_https_proxy:-${VLLM_PROXY_FALLBACK:-}}}"

_hc_vllm_unset_proxy() {
  unset http_proxy https_proxy HTTP_PROXY HTTPS_PROXY all_proxy ALL_PROXY
}

_hc_vllm_export_proxy() {
  local proxy="$1"
  export http_proxy="$proxy"
  export https_proxy="$proxy"
  export HTTP_PROXY="$proxy"
  export HTTPS_PROXY="$proxy"
  export all_proxy="$proxy"
  export ALL_PROXY="$proxy"
}

case "$VLLM_NET" in
  direct)
    _hc_vllm_unset_proxy
    export VLLM_NET_RESOLVED=direct
    ;;
  proxy)
    if [[ -z "$_chosen_proxy" ]]; then
      echo "VLLM_NET=proxy requires VLLM_PROXY, VLLM_PROXY_FALLBACK, or proxy env vars." >&2
      exit 2
    fi
    _hc_vllm_export_proxy "$_chosen_proxy"
    export VLLM_NET_RESOLVED=proxy
    ;;
  auto)
    if command -v curl >/dev/null 2>&1 && \
       env -u http_proxy -u https_proxy -u HTTP_PROXY -u HTTPS_PROXY -u all_proxy -u ALL_PROXY \
       curl -I -L --max-time 6 --connect-timeout 4 -s -o /dev/null https://huggingface.co/; then
      _hc_vllm_unset_proxy
      export VLLM_NET_RESOLVED=direct
    elif [[ -n "$_chosen_proxy" ]]; then
      _hc_vllm_export_proxy "$_chosen_proxy"
      export VLLM_NET_RESOLVED=proxy
    else
      _hc_vllm_unset_proxy
      export VLLM_NET_RESOLVED=direct-no-proxy-available
    fi
    ;;
  *)
    echo "Unknown VLLM_NET='$VLLM_NET'. Use auto, direct, or proxy." >&2
    exit 2
    ;;
esac

if [[ -f "$VLLM_VENV/bin/activate" ]]; then
  # shellcheck source=/dev/null
  source "$VLLM_VENV/bin/activate"
fi
