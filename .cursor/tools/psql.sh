#!/usr/bin/env bash
# Repli agents (sans protocole MCP) : psql sur DATABASE_URL / MCP_DATABASE_URL.
# Usage : .cursor/tools/psql.sh [-c SQL] [args psql…]
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
load_kv "$ROOT/.env"

URL="${MCP_DATABASE_URL:-${DATABASE_URL:-}}"
if [[ -z "$URL" ]]; then
  echo "[tools/psql] MCP_DATABASE_URL / DATABASE_URL manquant." >&2
  exit 1
fi

if [[ "$URL" == *"127.0.0.1"* || "$URL" == *"localhost"* ]]; then
  echo "[tools/psql] target=local" >&2
else
  echo "[tools/psql] target=remote" >&2
fi

exec psql "$URL" "$@"
