#!/usr/bin/env bash
set -euo pipefail

INSTALL_PREFIX="/opt/hermes-control"
CONFIG_DIR="/etc/hermes-control"
RUNTIME_ENV="/etc/hermes-control/runtime.env"
INSTALL_PREFIX="${HERMES_CONTROL_INSTALL_PREFIX:-$INSTALL_PREFIX}"
CONFIG_DIR="${HERMES_CONTROL_CONFIG_DIR:-$CONFIG_DIR}"
RUNTIME_ENV="${CONFIG_DIR}/runtime.env"

if [[ "$(id -u)" != "0" ]]; then
  echo "hermes-control WSL helpers must be installed as root." >&2
  exit 1
fi

missing=()
for command_name in curl python3 paste install; do
  if ! command -v "$command_name" >/dev/null 2>&1; then
    missing+=("$command_name")
  fi
done

if [[ "${#missing[@]}" -gt 0 ]]; then
  echo "Missing required WSL commands: ${missing[*]}" >&2
  echo "On Ubuntu, install them with: apt-get update && apt-get install -y curl python3 coreutils" >&2
  exit 1
fi

SCRIPT_DIR="$(CDPATH= cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"

install -d -m 0755 "${INSTALL_PREFIX}/bin"
install -d -m 0755 "${CONFIG_DIR}"
install -d -m 0755 /var/log/hermes-control
install -d -m 0755 /run/hermes-control

for script in "${SCRIPT_DIR}/bin/"*.sh; do
  install -m 0755 "$script" "${INSTALL_PREFIX}/bin/$(basename "$script")"
done

if [[ ! -f "$RUNTIME_ENV" ]]; then
  cat > "$RUNTIME_ENV" <<'ENV'
# Hermes Control WSL root-side runtime config.
# Adjust these paths after installing Hermes on a fresh distro.
HERMES_CONTROL_WORK_ROOT=/root/Hermres
HERMES_AGENT_ROOT=/root/Hermres/hermes-agent
HERMES_VENV_BIN=/root/Hermres/hermes-agent/.venv-hermes/bin/hermes
HERMES_HEALTH_URL=http://127.0.0.1:8642/health
HERMES_HEALTH_TIMEOUT_SECONDS=30
HERMES_LOG_DIR=/root/Hermres/logs
HERMES_PID_DIR=/run/hermes-control
HERMES_PID_FILE=/run/hermes-control/hermes-gateway.pid
HERMES_ENV_FILE=/root/.hermes/.env
VLLM_WORKSPACE=/mnt/e/WSL/vLLM
VLLM_MODELS_ENDPOINT=http://127.0.0.1:18080/v1/models
VLLM_LOG_DIR=/mnt/e/WSL/vLLM/logs
VLLM_PID_DIR=/run/hermes-control
VLLM_START_QWEN36_MTP=/mnt/e/WSL/vLLM/scripts/start-qwen36-mtp.sh
VLLM_START_QWEN36_AWQ_INT4=/mnt/e/WSL/vLLM/scripts/start-qwen36-int4-eager.sh
ENV
  chmod 0644 "$RUNTIME_ENV"
fi

if ! grep -q '^VLLM_WORKSPACE=' "$RUNTIME_ENV"; then
  cat >> "$RUNTIME_ENV" <<'ENV'

# vLLM runtime defaults added by Hermes Control Phase 5.
VLLM_WORKSPACE=/mnt/e/WSL/vLLM
VLLM_MODELS_ENDPOINT=http://127.0.0.1:18080/v1/models
VLLM_LOG_DIR=/mnt/e/WSL/vLLM/logs
VLLM_PID_DIR=/run/hermes-control
VLLM_START_QWEN36_MTP=/mnt/e/WSL/vLLM/scripts/start-qwen36-mtp.sh
VLLM_START_QWEN36_AWQ_INT4=/mnt/e/WSL/vLLM/scripts/start-qwen36-int4-eager.sh
ENV
fi

echo "Installed Hermes Control WSL helpers to ${INSTALL_PREFIX}/bin"
echo "Runtime config: ${RUNTIME_ENV}"
