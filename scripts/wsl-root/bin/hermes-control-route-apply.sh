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

if [[ -z "$profile_id" || -z "$provider_kind" || -z "$base_url" || -z "$model_id" ]]; then
  echo "usage: hermes-control-route-apply.sh <profile-id> <provider-kind> <base-url|auto-vllm> <model-id>" >&2
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

env_dir="$(dirname -- "$HERMES_ENV_FILE")"
mkdir -p "$env_dir"
touch "$HERMES_ENV_FILE"
chmod 0600 "$HERMES_ENV_FILE"

backup="${HERMES_ENV_FILE}.hermes-control-route.bak"
cp "$HERMES_ENV_FILE" "$backup"

tmp_file="$(mktemp "${HERMES_ENV_FILE}.tmp.XXXXXX")"
chmod 0600 "$tmp_file"

HERMES_ROUTE_ENV_FILE="$HERMES_ENV_FILE" \
HERMES_ROUTE_OUTPUT_FILE="$tmp_file" \
HERMES_ROUTE_PROFILE_ID="$profile_id" \
HERMES_ROUTE_PROVIDER_KIND="$provider_kind" \
HERMES_ROUTE_BASE_URL="$base_url" \
HERMES_ROUTE_MODEL_ID="$model_id" \
python3 - <<'PY'
import os
from pathlib import Path

env_path = Path(os.environ["HERMES_ROUTE_ENV_FILE"])
output_path = Path(os.environ["HERMES_ROUTE_OUTPUT_FILE"])
updates = {
    "HERMES_CONTROL_ACTIVE_PROFILE_ID": os.environ["HERMES_ROUTE_PROFILE_ID"],
    "HERMES_CONTROL_ACTIVE_PROVIDER_KIND": os.environ["HERMES_ROUTE_PROVIDER_KIND"],
    "LM_BASE_URL": os.environ["HERMES_ROUTE_BASE_URL"],
    "LM_MODEL": os.environ["HERMES_ROUTE_MODEL_ID"],
}

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

HERMES_ROUTE_PROFILE_ID="$profile_id" \
HERMES_ROUTE_PROVIDER_KIND="$provider_kind" \
HERMES_ROUTE_BASE_URL="$base_url" \
HERMES_ROUTE_MODEL_ID="$model_id" \
HERMES_ENV_FILE="$HERMES_ENV_FILE" \
python3 - <<'PY'
import json
import os

print(json.dumps({
    "state": "route_applied",
    "profile_id": os.environ["HERMES_ROUTE_PROFILE_ID"],
    "provider_kind": os.environ["HERMES_ROUTE_PROVIDER_KIND"],
    "base_url": os.environ["HERMES_ROUTE_BASE_URL"],
    "model_id": os.environ["HERMES_ROUTE_MODEL_ID"],
    "env_file": os.environ["HERMES_ENV_FILE"],
}, sort_keys=True))
PY
