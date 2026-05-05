#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(CDPATH= cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck disable=SC1091
source "${SCRIPT_DIR}/hermes-control-common.sh"

hc_require_root
hc_prepare_dirs

mode="${1:-if-running}"
case "$mode" in
  if-running | force) ;;
  *)
    echo "usage: hermes-control-openwebui-refresh.sh <if-running|force>" >&2
    exit 2
    ;;
esac

mapfile -t pids < <(hc_openwebui_find_pids)
if [[ "${#pids[@]}" -eq 0 && "$mode" == "if-running" ]]; then
  rm -f "$OPENWEBUI_PID_FILE"
  hc_openwebui_json "open_webui_not_running" "Open WebUI is not running; refresh skipped."
  exit 0
fi

if [[ "${#pids[@]}" -gt 0 ]]; then
  kill -TERM "${pids[@]}" 2>/dev/null || true
  for _ in {1..20}; do
    mapfile -t remaining < <(hc_openwebui_find_pids)
    if [[ "${#remaining[@]}" -eq 0 ]]; then
      break
    fi
    sleep 1
  done
fi

mapfile -t remaining < <(hc_openwebui_find_pids)
if [[ "${#remaining[@]}" -gt 0 ]]; then
  hc_openwebui_json "open_webui_stopping" "Open WebUI process did not exit after SIGTERM."
  exit 1
fi

if [[ ! -x "$OPENWEBUI_VENV_BIN" ]]; then
  hc_openwebui_json "open_webui_error" "Open WebUI executable is not executable: ${OPENWEBUI_VENV_BIN}"
  exit 1
fi

hc_load_hermes_env

openwebui_base_url="${OPENWEBUI_OPENAI_BASE_URL:-$OPENWEBUI_HERMES_BASE_URL}"
openwebui_default_model="${OPENWEBUI_DEFAULT_MODEL:-hermes-agent}"
openwebui_api_key="${OPENWEBUI_OPENAI_API_KEY:-${API_SERVER_KEY:-}}"

if [[ -z "$openwebui_api_key" ]]; then
  hc_openwebui_json "open_webui_error" "Open WebUI API key source is missing."
  exit 1
fi

out_log="${HERMES_LOG_DIR}/open-webui.out.log"
err_log="${HERMES_LOG_DIR}/open-webui.err.log"

if command -v setsid >/dev/null 2>&1; then
  setsid env \
    DATA_DIR="$OPENWEBUI_DATA_DIR" \
    ENABLE_OLLAMA_API="false" \
    ENABLE_OPENAI_API="true" \
    OPENAI_API_BASE_URLS="$openwebui_base_url" \
    OPENAI_API_KEYS="$openwebui_api_key" \
    DEFAULT_MODELS="$openwebui_default_model" \
    RAG_OPENAI_API_BASE_URL="$openwebui_base_url" \
    RAG_OPENAI_API_KEY="$openwebui_api_key" \
    "$OPENWEBUI_VENV_BIN" serve --host "$OPENWEBUI_HOST" --port "$OPENWEBUI_PORT" \
    >>"$out_log" 2>>"$err_log" </dev/null &
else
  nohup env \
    DATA_DIR="$OPENWEBUI_DATA_DIR" \
    ENABLE_OLLAMA_API="false" \
    ENABLE_OPENAI_API="true" \
    OPENAI_API_BASE_URLS="$openwebui_base_url" \
    OPENAI_API_KEYS="$openwebui_api_key" \
    DEFAULT_MODELS="$openwebui_default_model" \
    RAG_OPENAI_API_BASE_URL="$openwebui_base_url" \
    RAG_OPENAI_API_KEY="$openwebui_api_key" \
    "$OPENWEBUI_VENV_BIN" serve --host "$OPENWEBUI_HOST" --port "$OPENWEBUI_PORT" \
    >>"$out_log" 2>>"$err_log" </dev/null &
fi

pid="$!"
printf '%s\n' "$pid" > "$OPENWEBUI_PID_FILE"
sleep "${OPENWEBUI_START_SETTLE_SECONDS:-1}"

if ! kill -0 "$pid" 2>/dev/null; then
  rm -f "$OPENWEBUI_PID_FILE"
  hc_openwebui_json "open_webui_error" "Open WebUI exited during startup. Check ${out_log} and ${err_log}."
  exit 1
fi

deadline=$((SECONDS + OPENWEBUI_HEALTH_TIMEOUT_SECONDS))
while (( SECONDS <= deadline )); do
  if hc_openwebui_health_ok; then
    hc_openwebui_json "open_webui_restarted" "Open WebUI was restarted and is healthy."
    exit 0
  fi
  sleep 1
done

hc_openwebui_json "open_webui_restarted_not_ready" "Open WebUI was restarted but did not become healthy before timeout."
