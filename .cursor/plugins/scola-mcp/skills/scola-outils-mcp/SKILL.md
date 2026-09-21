---
name: scola-outils-mcp
description: Utiliser les MCP ScolIA (postgres, browser, fetch, mistral, scaleway) ou le repli CLI du repo quand le catalogue MCP de la session est vide.
---

# Outils ScolIA (MCP + repli)

## Ordre

1. Si `GetDynamicTools` liste `postgres` / `browser` / `fetch` / `mistral` / `scaleway` : les utiliser.
2. Sinon **ne pas inventer** l’état. Repli repo, sans protocole MCP :

| Besoin | Commande |
|--------|----------|
| Postgres | `bash .cursor/tools/psql.sh -c 'SELECT 1'` (URL = `DATABASE_URL` / `MCP_DATABASE_URL`) |
| Doc web | `node .cursor/tools/fetch.mjs <url>` |
| UI locale | `node .cursor/tools/browser.mjs http://localhost:3000` |
| Scaleway lecture | `bash .cursor/tools/scw.sh instance server list` si `SCW_*` présents |
| Diagnostic | `bash .cursor/tools/status.sh` |

## Secrets

Uniquement `process.env` / `.env.local` / `.cursor/mcp.local.env`. Jamais de secret en dur. Pas de mutation prod Scaleway.

## Limitation Cursor

Un Cloud Agent déjà lancé ne charge pas `.cursor/mcp.json`. `workspaceOpen` (plugin projet) ne s’exécute pas sur Cloud Agents. L’IDE charge `.cursor/mcp.json` + le plugin `.cursor/plugins/scola-mcp`.
