#!/usr/bin/env bash
# IDE / CLI : charge le plugin projet ScolIA (MCP + skill de repli).
# Cloud Agents : workspaceOpen n'est pas exécuté (limitation Cursor).
set -euo pipefail

# Drain stdin (payload workspaceOpen) — stdout = JSON pluginPaths uniquement.
cat >/dev/null || true

ROOT="${CURSOR_PROJECT_DIR:-}"
if [[ -z "$ROOT" ]]; then
  ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
fi

PLUGIN="${ROOT}/.cursor/plugins/scola-mcp"
if [[ ! -d "$PLUGIN" ]]; then
  echo "[scola-plugin] introuvable: ${PLUGIN}" >&2
  echo '{}'
  exit 0
fi

node -e 'process.stdout.write(JSON.stringify({pluginPaths:[process.argv[1]]})+"\n")' "$PLUGIN"
