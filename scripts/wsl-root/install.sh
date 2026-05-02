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

PROJECT_VLLM_WORKSPACE="/mnt/e/WSL/Hermres/hermes-control/vLLM"
EXTERNAL_VLLM_MODEL_ROOT="/mnt/e/WSL/vLLM/models"

missing=()
for command_name in curl python3 paste install sed; do
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
install -d -m 0755 "${PROJECT_VLLM_WORKSPACE}/logs"
install -d -m 0755 "${PROJECT_VLLM_WORKSPACE}/cache"
install -d -m 0755 "${PROJECT_VLLM_WORKSPACE}/downloads"
install -d -m 0755 "${PROJECT_VLLM_WORKSPACE}/tmp"
install -d -m 0755 "${EXTERNAL_VLLM_MODEL_ROOT}"

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
VLLM_WORKSPACE=/mnt/e/WSL/Hermres/hermes-control/vLLM
VLLM_MODEL_ROOT=/mnt/e/WSL/vLLM/models
VLLM_PORT=18080
VLLM_CLIENT_HOST=auto
VLLM_MODELS_ENDPOINT=auto
VLLM_LOG_DIR=/mnt/e/WSL/Hermres/hermes-control/vLLM/logs
VLLM_PID_DIR=/run/hermes-control
VLLM_START_QWEN36_MTP=/mnt/e/WSL/Hermres/hermes-control/vLLM/scripts/start-qwen36-mtp.sh
VLLM_START_QWEN36_AWQ_INT4=/mnt/e/WSL/Hermres/hermes-control/vLLM/scripts/start-qwen36-int4-eager.sh
VLLM_BOOTSTRAP_SCRIPT=/mnt/e/WSL/Hermres/hermes-control/vLLM/scripts/bootstrap.sh
ENV
  chmod 0644 "$RUNTIME_ENV"
fi

set_runtime_env() {
  local key="$1"
  local value="$2"
  local escaped_value
  escaped_value="$(printf '%s' "$value" | sed 's/[\/&]/\\&/g')"

  if grep -q "^${key}=" "$RUNTIME_ENV"; then
    sed -i "s/^${key}=.*/${key}=${escaped_value}/" "$RUNTIME_ENV"
  else
    printf '%s=%s\n' "$key" "$value" >> "$RUNTIME_ENV"
  fi
}

set_runtime_env "VLLM_WORKSPACE" "$PROJECT_VLLM_WORKSPACE"
set_runtime_env "VLLM_MODEL_ROOT" "$EXTERNAL_VLLM_MODEL_ROOT"
set_runtime_env "VLLM_PORT" "18080"
set_runtime_env "VLLM_CLIENT_HOST" "auto"
set_runtime_env "VLLM_MODELS_ENDPOINT" "auto"
set_runtime_env "VLLM_LOG_DIR" "${PROJECT_VLLM_WORKSPACE}/logs"
set_runtime_env "VLLM_PID_DIR" "/run/hermes-control"
set_runtime_env "VLLM_START_QWEN36_MTP" "${PROJECT_VLLM_WORKSPACE}/scripts/start-qwen36-mtp.sh"
set_runtime_env "VLLM_START_QWEN36_AWQ_INT4" "${PROJECT_VLLM_WORKSPACE}/scripts/start-qwen36-int4-eager.sh"
set_runtime_env "VLLM_BOOTSTRAP_SCRIPT" "${PROJECT_VLLM_WORKSPACE}/scripts/bootstrap.sh"

echo "Installed Hermes Control WSL helpers to ${INSTALL_PREFIX}/bin"
echo "Runtime config: ${RUNTIME_ENV}"
