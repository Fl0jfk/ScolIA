#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
ENV_FILE="$ROOT/.cursor/mcp.local.env"
DOTENV_LOCAL="$ROOT/.env.local"

# Secrets injectés par le dashboard Cloud Agent : ne pas les écraser avec mcp.local.env.
PRESERVED_MCP_DATABASE_URL="${MCP_DATABASE_URL:-}"
PRESERVED_MISTRAL_API_KEY="${MISTRAL_API_KEY:-}"

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

if [[ -n "$PRESERVED_MCP_DATABASE_URL" ]]; then
  MCP_DATABASE_URL="$PRESERVED_MCP_DATABASE_URL"
fi
if [[ -n "$PRESERVED_MISTRAL_API_KEY" ]]; then
  MISTRAL_API_KEY="$PRESERVED_MISTRAL_API_KEY"
fi

# Préférence : URL dédiée MCP (idéalement lecture seule). Repli : DATABASE_URL app.
if [[ -z "${MCP_DATABASE_URL:-}" && -n "${DATABASE_URL:-}" ]]; then
  MCP_DATABASE_URL="$DATABASE_URL"
fi

: "${MCP_DATABASE_URL:?Définis MCP_DATABASE_URL (secret dashboard, .cursor/mcp.local.env, ou DATABASE_URL)}"

# Indice non sensible pour les logs agents
if [[ "$MCP_DATABASE_URL" == *"127.0.0.1"* || "$MCP_DATABASE_URL" == *"localhost"* ]]; then
  echo "[postgres-mcp] target=local" >&2
else
  echo "[postgres-mcp] target=remote" >&2
fi

exec npx -y @modelcontextprotocol/server-postgres@0.6.2 "$MCP_DATABASE_URL"
