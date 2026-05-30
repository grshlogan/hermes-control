#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(CDPATH= cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck disable=SC1091
source "${SCRIPT_DIR}/hermes-control-common.sh"

hc_require_root
hc_prepare_dirs

profile_id="${1:-}"
provider_kind="${2:-}"
base_url="${3:-}"
model_id="${4:-}"
secret_env_key="${5:-}"

if [[ -z "$profile_id" || -z "$provider_kind" || -z "$base_url" || -z "$model_id" || -z "$secret_env_key" ]]; then
  echo "usage: hermes-control-route-apply.sh <profile-id> <provider-kind> <base-url|auto-vllm> <model-id> <secret-env-key|none>" >&2
  exit 2
fi

if [[ ! "$profile_id" =~ ^[A-Za-z0-9_.-]+$ || ! "$model_id" =~ ^[A-Za-z0-9_.-]+$ ]]; then
  echo "profile-id and model-id must be safe identifiers" >&2
  exit 2
fi

case "$provider_kind" in
  openai-compatible | claude | deepseek | codex | local-vllm | lm-studio) ;;
  *)
    echo "unknown provider kind: ${provider_kind}" >&2
    exit 2
    ;;
esac

if [[ "$base_url" == "auto-vllm" ]]; then
  base_url="http://$(hc_resolve_vllm_client_host):${VLLM_PORT}/v1"
elif [[ ! "$base_url" =~ ^https?://[A-Za-z0-9_.:/-]+$ ]]; then
  echo "base-url must be http(s) and contain only safe route characters" >&2
  exit 2
fi

if [[ "$secret_env_key" != "none" && ! "$secret_env_key" =~ ^[A-Z][A-Z0-9_]*$ ]]; then
  echo "secret-env-key must be 'none' or an uppercase environment variable name" >&2
  exit 2
fi

env_dir="$(dirname -- "$HERMES_ENV_FILE")"
mkdir -p "$env_dir"
touch "$HERMES_ENV_FILE"
chmod 0600 "$HERMES_ENV_FILE"

backup="${HERMES_ENV_FILE}.hermes-control-route.bak"
cp "$HERMES_ENV_FILE" "$backup"

tmp_file="$(mktemp "${HERMES_ENV_FILE}.tmp.XXXXXX")"
chmod 0600 "$tmp_file"

restore_openwebui_backup() {
  local sync_json="${1:-}"
  local reason="${2:-route apply failure}"

  if [[ -z "$sync_json" ]]; then
    return 0
  fi

  HERMES_ROUTE_OPENWEBUI_SYNC_JSON="$sync_json" \
  HERMES_ROUTE_OPENWEBUI_RESTORE_REASON="$reason" \
  python3 - <<'PY'
import json
import os
import shutil
import sys
from pathlib import Path

try:
    payload = json.loads(os.environ["HERMES_ROUTE_OPENWEBUI_SYNC_JSON"])
except json.JSONDecodeError:
    sys.exit(0)

backup_file = payload.get("backup_file")
db_file = payload.get("db_file")
if not backup_file or not db_file:
    sys.exit(0)

backup_path = Path(backup_file)
db_path = Path(db_file)
if not backup_path.is_file():
    print(f"Open WebUI backup file is missing: {backup_path}", file=sys.stderr)
    sys.exit(1)

db_path.parent.mkdir(parents=True, exist_ok=True)
shutil.copy2(backup_path, db_path)
print(json.dumps({
    "state": "open_webui_restored",
    "reason": os.environ["HERMES_ROUTE_OPENWEBUI_RESTORE_REASON"],
    "db_file": str(db_path),
    "backup_file": str(backup_path),
}, sort_keys=True), file=sys.stderr)
PY
}

HERMES_ROUTE_ENV_FILE="$HERMES_ENV_FILE" \
HERMES_ROUTE_OUTPUT_FILE="$tmp_file" \
HERMES_ROUTE_PROFILE_ID="$profile_id" \
HERMES_ROUTE_PROVIDER_KIND="$provider_kind" \
HERMES_ROUTE_BASE_URL="$base_url" \
HERMES_ROUTE_MODEL_ID="$model_id" \
HERMES_ROUTE_SECRET_ENV_KEY="$secret_env_key" \
python3 - <<'PY'
import os
import sys
from pathlib import Path

env_path = Path(os.environ["HERMES_ROUTE_ENV_FILE"])
output_path = Path(os.environ["HERMES_ROUTE_OUTPUT_FILE"])
provider_kind = os.environ["HERMES_ROUTE_PROVIDER_KIND"]
secret_env_key = os.environ["HERMES_ROUTE_SECRET_ENV_KEY"]

values = {}
for raw_line in env_path.read_text(encoding="utf-8").splitlines():
    stripped = raw_line.strip()
    if not stripped or stripped.startswith("#") or "=" not in raw_line:
        continue
    key, value = raw_line.split("=", 1)
    values[key.strip()] = value

updates = {
    "HERMES_CONTROL_ACTIVE_PROFILE_ID": os.environ["HERMES_ROUTE_PROFILE_ID"],
    "HERMES_CONTROL_ACTIVE_PROVIDER_KIND": provider_kind,
    "LM_BASE_URL": os.environ["HERMES_ROUTE_BASE_URL"],
    "LM_MODEL": os.environ["HERMES_ROUTE_MODEL_ID"],
    "HERMES_CONTROL_ACTIVE_SECRET_ENV_KEY": secret_env_key,
    "OPENWEBUI_OPENAI_BASE_URL": "http://127.0.0.1:8642/v1",
    "OPENWEBUI_DEFAULT_MODEL": "hermes-agent",
}

if secret_env_key != "none":
    secret_value = values.get(secret_env_key) or os.environ.get(secret_env_key, "")
    if not secret_value:
        print(f"Required secret env key is missing or empty: {secret_env_key}", file=sys.stderr)
        sys.exit(1)
    if provider_kind in {"openai-compatible", "deepseek", "codex", "lm-studio"}:
        updates["LM_API_KEY"] = secret_value
    elif provider_kind == "claude":
        updates["ANTHROPIC_BASE_URL"] = os.environ["HERMES_ROUTE_BASE_URL"]
        updates["ANTHROPIC_AUTH_TOKEN"] = secret_value
        updates["ANTHROPIC_MODEL"] = os.environ["HERMES_ROUTE_MODEL_ID"]
        updates["ANTHROPIC_DEFAULT_SONNET_MODEL"] = os.environ.get("ANTHROPIC_DEFAULT_SONNET_MODEL", os.environ["HERMES_ROUTE_MODEL_ID"])
        updates["ANTHROPIC_DEFAULT_HAIKU_MODEL"] = os.environ.get("ANTHROPIC_DEFAULT_HAIKU_MODEL", os.environ["HERMES_ROUTE_MODEL_ID"])
        updates["ANTHROPIC_DEFAULT_OPUS_MODEL"] = os.environ.get("ANTHROPIC_DEFAULT_OPUS_MODEL", os.environ["HERMES_ROUTE_MODEL_ID"])

for key in ["API_TIMEOUT_MS", "HTTP_PROXY", "HTTPS_PROXY", "NO_PROXY", "effortLevel"]:
    env_value = os.environ.get(key)
    if env_value:
        updates[key] = env_value

seen = set()
lines = []
for raw_line in env_path.read_text(encoding="utf-8").splitlines():
    stripped = raw_line.strip()
    if not stripped or stripped.startswith("#") or "=" not in raw_line:
        lines.append(raw_line)
        continue
    key = raw_line.split("=", 1)[0].strip()
    if key in updates:
        lines.append(f"{key}={updates[key]}")
        seen.add(key)
    else:
        lines.append(raw_line)

for key, value in updates.items():
    if key not in seen:
        lines.append(f"{key}={value}")

output_path.write_text("\n".join(lines) + "\n", encoding="utf-8")
PY

mv "$tmp_file" "$HERMES_ENV_FILE"

if ! "${SCRIPT_DIR}/hermes-control-restart.sh"; then
  cp "$backup" "$HERMES_ENV_FILE"
  echo "Hermes restart failed; restored previous Hermes env file." >&2
  exit 1
fi

if ! "${SCRIPT_DIR}/hermes-control-health.sh" "${HERMES_ROUTE_HEALTH_TIMEOUT_SECONDS:-30}" ready; then
  cp "$backup" "$HERMES_ENV_FILE"
  "${SCRIPT_DIR}/hermes-control-restart.sh" >/dev/null 2>&1 || true
  echo "Hermes health check failed after route apply; restored previous Hermes env file." >&2
  exit 1
fi

openwebui_sync_json="$("${SCRIPT_DIR}/hermes-control-openwebui-sync.sh" "http://127.0.0.1:8642/v1" "hermes-agent" "API_SERVER_KEY")" || {
  cp "$backup" "$HERMES_ENV_FILE"
  "${SCRIPT_DIR}/hermes-control-restart.sh" >/dev/null 2>&1 || true
  echo "Open WebUI sync failed after route apply; restored previous Hermes env file." >&2
  exit 1
}

openwebui_refresh_json="$("${SCRIPT_DIR}/hermes-control-openwebui-refresh.sh" if-running)" || {
  restore_openwebui_backup "$openwebui_sync_json" "Open WebUI refresh failed after route apply." || true
  cp "$backup" "$HERMES_ENV_FILE"
  "${SCRIPT_DIR}/hermes-control-restart.sh" >/dev/null 2>&1 || true
  "${SCRIPT_DIR}/hermes-control-openwebui-refresh.sh" force >/dev/null 2>&1 || true
  echo "Open WebUI refresh failed after route apply; restored previous Hermes env file and Open WebUI database backup." >&2
  exit 1
}

HERMES_ROUTE_PROFILE_ID="$profile_id" \
HERMES_ROUTE_PROVIDER_KIND="$provider_kind" \
HERMES_ROUTE_BASE_URL="$base_url" \
HERMES_ROUTE_MODEL_ID="$model_id" \
HERMES_ROUTE_SECRET_ENV_KEY="$secret_env_key" \
HERMES_ROUTE_OPENWEBUI_SYNC_JSON="$openwebui_sync_json" \
HERMES_ROUTE_OPENWEBUI_REFRESH_JSON="$openwebui_refresh_json" \
HERMES_ENV_FILE="$HERMES_ENV_FILE" \
python3 - <<'PY'
import json
import os

try:
    open_webui_sync = json.loads(os.environ["HERMES_ROUTE_OPENWEBUI_SYNC_JSON"])
except json.JSONDecodeError:
    open_webui_sync = {"state": "open_webui_unknown"}

try:
    open_webui_refresh = json.loads(os.environ["HERMES_ROUTE_OPENWEBUI_REFRESH_JSON"])
except json.JSONDecodeError:
    open_webui_refresh = {"state": "open_webui_refresh_unknown"}

print(json.dumps({
    "state": "route_applied",
    "profile_id": os.environ["HERMES_ROUTE_PROFILE_ID"],
    "provider_kind": os.environ["HERMES_ROUTE_PROVIDER_KIND"],
    "base_url": os.environ["HERMES_ROUTE_BASE_URL"],
    "model_id": os.environ["HERMES_ROUTE_MODEL_ID"],
    "secret_env_key": os.environ["HERMES_ROUTE_SECRET_ENV_KEY"],
    "open_webui": {
        "sync": open_webui_sync,
        "refresh": open_webui_refresh,
    },
    "env_file": os.environ["HERMES_ENV_FILE"],
}, sort_keys=True))
PY
