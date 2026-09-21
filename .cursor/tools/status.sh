#!/usr/bin/env bash
# État des outils agents (MCP IDE vs repli CLI Cloud).
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
# shellcheck disable=SC1091
source "$ROOT/.cursor/ensure-agent-path.sh"

ok() { printf '  %-18s OK  %s\n' "$1" "$2"; }
ko() { printf '  %-18s --  %s\n' "$1" "$2"; }

echo "ScolIA outils agents"
echo "PATH uvx=$(command -v uvx 2>/dev/null || echo absent) scw=$(command -v scw 2>/dev/null || echo absent) psql=$(command -v psql 2>/dev/null || echo absent)"

if command -v psql >/dev/null 2>&1; then
  ok psql "$(command -v psql)"
else
  ko psql "installe postgresql-client"
fi

if [[ -n "${DATABASE_URL:-}${MCP_DATABASE_URL:-}" ]]; then
  ok DATABASE_URL "présent (valeur masquée)"
elif [[ -f "$ROOT/.env.local" ]] && grep -q '^DATABASE_URL=' "$ROOT/.env.local"; then
  ok DATABASE_URL ".env.local"
else
  ko DATABASE_URL "absent"
fi

if command -v uvx >/dev/null 2>&1; then
  ok uvx "$(command -v uvx)"
else
  ko uvx "relance install.sh"
fi

if command -v scw >/dev/null 2>&1; then
  ok scw "$(command -v scw)"
else
  ko scw "relance install.sh"
fi

if command -v github-mcp-server >/dev/null 2>&1; then
  ok github-mcp "$(command -v github-mcp-server)"
else
  ko github-mcp "optionnel"
fi

if [[ -n "${MISTRAL_API_KEY:-}" ]]; then
  ok MISTRAL_API_KEY "présent"
else
  ko MISTRAL_API_KEY "optionnel OCR"
fi

echo
echo "Repli CLI (Cloud Agents sans MCP dashboard) :"
echo "  bash .cursor/tools/psql.sh -c 'SELECT 1'"
echo "  node .cursor/tools/fetch.mjs https://example.com"
echo "  node .cursor/tools/browser.mjs http://localhost:3000"
echo "  bash .cursor/tools/status.sh"
