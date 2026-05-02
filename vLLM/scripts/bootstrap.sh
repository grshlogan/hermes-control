#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(CDPATH= cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=env.sh
source "$SCRIPT_DIR/env.sh"

LOG_FILE="${LOG_FILE:-${VLLM_LOG_DIR}/bootstrap-$(date +%Y%m%d-%H%M%S).log}"
VLLM_PIP_PACKAGE="${VLLM_PIP_PACKAGE:-vllm}"

if ! command -v python3 >/dev/null 2>&1; then
  echo "python3 is required to bootstrap vLLM." >&2
  exit 1
fi

if [[ ! -x "$VLLM_VENV/bin/python" ]]; then
  python3 -m venv --copies "$VLLM_VENV"
fi

VLLM_PYTHON="$VLLM_VENV/bin/python"

run_pip_direct_then_proxy() {
  local description="$1"
  shift

  echo "==> ${description} (direct first)" | tee -a "$LOG_FILE"
  if env -u http_proxy -u https_proxy -u HTTP_PROXY -u HTTPS_PROXY -u all_proxy -u ALL_PROXY \
     "$VLLM_PYTHON" -m pip "$@" 2>&1 | tee -a "$LOG_FILE"; then
    return 0
  fi

  if [[ -z "${VLLM_PROXY_FALLBACK:-}" ]]; then
    echo "Direct install failed and VLLM_PROXY_FALLBACK is not set." | tee -a "$LOG_FILE" >&2
    return 1
  fi

  echo "==> ${description} (fallback proxy: ${VLLM_PROXY_FALLBACK})" | tee -a "$LOG_FILE"
  HTTP_PROXY="$VLLM_PROXY_FALLBACK" \
  HTTPS_PROXY="$VLLM_PROXY_FALLBACK" \
  ALL_PROXY="$VLLM_PROXY_FALLBACK" \
  http_proxy="$VLLM_PROXY_FALLBACK" \
  https_proxy="$VLLM_PROXY_FALLBACK" \
  all_proxy="$VLLM_PROXY_FALLBACK" \
    "$VLLM_PYTHON" -m pip "$@" 2>&1 | tee -a "$LOG_FILE"
}

run_pip_direct_then_proxy "upgrade pip tooling" install --upgrade pip wheel "setuptools<81,>=77.0.3"
run_pip_direct_then_proxy "install ${VLLM_PIP_PACKAGE}" install "$VLLM_PIP_PACKAGE"

"$VLLM_PYTHON" - <<'PY' | tee -a "$LOG_FILE"
import importlib.metadata

print("vllm", importlib.metadata.version("vllm"))
PY

echo "vLLM bootstrap complete."
echo "workspace=${VLLM_WORKSPACE}"
echo "venv=${VLLM_VENV}"
echo "models=${VLLM_MODEL_ROOT}"
echo "log=${LOG_FILE}"
