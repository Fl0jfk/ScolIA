#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
ENV_FILE="$ROOT/.cursor/mcp.local.env"
DOTENV_LOCAL="$ROOT/.env.local"

load_env_file() {
  local file="$1"
  [[ -f "$file" ]] || return 0
  set -a
  # shellcheck disable=SC1090
  source "$file"
  set +a
}

load_env_file "$ENV_FILE"
load_env_file "$DOTENV_LOCAL"

# Préférence : URL dédiée MCP (idéalement lecture seule). Repli : DATABASE_URL app.
if [[ -z "${MCP_DATABASE_URL:-}" && -n "${DATABASE_URL:-}" ]]; then
  MCP_DATABASE_URL="$DATABASE_URL"
fi

: "${MCP_DATABASE_URL:?Définis MCP_DATABASE_URL dans .cursor/mcp.local.env (ou DATABASE_URL dans .env.local)}"

exec npx -y @modelcontextprotocol/server-postgres@0.6.2 "$MCP_DATABASE_URL"
