#!/usr/bin/env bash
# Repli Scaleway CLI (lecture). Ne pas muter la prod.
# Usage : .cursor/tools/scw.sh <args scw…>
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
# shellcheck disable=SC1091
source "$ROOT/.cursor/ensure-agent-path.sh"

load_kv() {
  local file="$1"
  [[ -f "$file" ]] || return 0
  while IFS= read -r line || [[ -n "$line" ]]; do
    line="${line%$'\r'}"
    [[ -z "$line" || "$line" == \#* ]] && continue
    [[ "$line" != *=* ]] && continue
    local key="${line%%=*}"
    local value="${line#*=}"
    key="${key%"${key##*[![:space:]]}"}"
    value="${value#"${value%%[![:space:]]*}"}"
    value="${value%"${value##*[![:space:]]}"}"
    if [[ "${value:0:1}" == '"' && "${value: -1}" == '"' ]]; then
      value="${value:1:-1}"
    elif [[ "${value:0:1}" == "'" && "${value: -1}" == "'" ]]; then
      value="${value:1:-1}"
    fi
    if [[ -z "${!key:-}" ]]; then
      export "$key=$value"
    fi
  done < "$file"
}

load_kv "$ROOT/.cursor/mcp.local.env"
load_kv "$ROOT/.env.local"

if ! command -v scw >/dev/null 2>&1; then
  echo "[tools/scw] binaire scw introuvable — relance .cursor/install.sh." >&2
  exit 1
fi

missing=()
for k in SCW_ACCESS_KEY SCW_SECRET_KEY SCW_DEFAULT_ORGANIZATION_ID SCW_DEFAULT_PROJECT_ID; do
  if [[ -z "${!k:-}" ]]; then
    missing+=("$k")
  fi
done
if [[ ${#missing[@]} -gt 0 ]]; then
  echo "[tools/scw] variables manquantes: ${missing[*]}" >&2
  exit 1
fi

export SCW_DEFAULT_REGION="${SCW_DEFAULT_REGION:-fr-par}"
echo "[tools/scw] lecture seulement — pas de mutation prod." >&2
exec scw "$@"
