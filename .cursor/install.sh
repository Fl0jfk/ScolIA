#!/usr/bin/env bash
# Bootstrap idempotent Cloud Agent / dev local pour docslapro (ScolIA).
# - PostgreSQL + rôle/base
# - npm ci
# - .env.local + mcp.local.env
# - drizzle-kit push
# - outils MCP (uvx, Playwright Chromium, Scaleway CLI)
# - seed:dev
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

PG_USER="scola"
PG_PASSWORD="scola_dev_pwd"
PG_DB="scola"
export PATH="${HOME}/.local/bin:${HOME}/bin:/usr/local/bin:${PATH}"

echo "[install] PostgreSQL…"
if ! command -v pg_ctlcluster >/dev/null 2>&1; then
  echo "[install] Installation de PostgreSQL via apt…"
  sudo apt-get update -qq
  sudo DEBIAN_FRONTEND=noninteractive apt-get install -y -qq postgresql postgresql-contrib
fi

bash "$REPO_ROOT/.cursor/start.sh"

echo "[install] Rôle & base PostgreSQL…"
sudo -u postgres psql -v ON_ERROR_STOP=1 <<SQL
DO \$\$
BEGIN
  IF NOT EXISTS (SELECT FROM pg_roles WHERE rolname = '${PG_USER}') THEN
    CREATE ROLE ${PG_USER} LOGIN PASSWORD '${PG_PASSWORD}';
  END IF;
END\$\$;
SQL
if ! sudo -u postgres psql -tAc "SELECT 1 FROM pg_database WHERE datname='${PG_DB}'" | grep -q 1; then
  sudo -u postgres createdb -O "${PG_USER}" "${PG_DB}"
fi
sudo -u postgres psql -v ON_ERROR_STOP=1 \
  -c "ALTER DATABASE ${PG_DB} OWNER TO ${PG_USER};" \
  -c "GRANT ALL PRIVILEGES ON DATABASE ${PG_DB} TO ${PG_USER};"

echo "[install] Dépendances npm (npm ci)…"
npm ci

echo "[install] .env.local…"
if [ ! -f "$REPO_ROOT/.env.local" ]; then
  SECRET="$(node -e "console.log(require('crypto').randomBytes(32).toString('hex'))")"
  cat > "$REPO_ROOT/.env.local" <<ENV
# Développement local — généré par .cursor/install.sh (ne pas committer, secret jetable).
DATABASE_URL=postgresql://${PG_USER}:${PG_PASSWORD}@127.0.0.1:5432/${PG_DB}
BETTER_AUTH_SECRET=${SECRET}
BETTER_AUTH_URL=http://localhost:3000
NEXT_PUBLIC_APP_URL=http://localhost:3000
AUTH_PROVIDER=better-auth
ENT_CORE_DB=1
REQUIRE_EMAIL_VERIFICATION=false
BUCKET_NAME=scola-dev
DEFAULT_TENANT_SLUG=default
DEFAULT_TENANT_LABEL="Instance de développement"
ENV
  echo "[install] .env.local généré."
else
  echo "[install] .env.local déjà présent — conservé."
fi

echo "[install] .cursor/mcp.local.env…"
if [ ! -f "$REPO_ROOT/.cursor/mcp.local.env" ]; then
  DB_URL="$(node -e "
const fs=require('fs');
const t=fs.readFileSync('.env.local','utf8');
for (const line of t.split('\\n')) {
  const m=line.match(/^DATABASE_URL=(.*)$/);
  if (m) { let v=m[1].trim(); if((v.startsWith('\"')&&v.endsWith('\"'))||(v.startsWith(\"'\")&&v.endsWith(\"'\"))) v=v.slice(1,-1); process.stdout.write(v); break; }
}")"
  cat > "$REPO_ROOT/.cursor/mcp.local.env" <<ENV
# Généré par install.sh — gitignored. Ne pas committer.
MCP_DATABASE_URL=${DB_URL}
ENV
  echo "[install] mcp.local.env généré."
else
  echo "[install] mcp.local.env déjà présent — conservé."
fi

# Ne plus préchauffer mcp-server-fetch (stdio serveur) — uvx suffit au premier appel MCP.

echo "[install] Synchronisation du schéma (drizzle-kit push)…"
# L'historique de migrations contient un renommage clerk_*→external_* incohérent
# sur base fraîche ; drizzle-kit push aligne la base sur db/schema.ts (source de vérité ORM).
npx --yes drizzle-kit push --force

echo "[install] uv (uvx) pour MCP fetch…"
if ! command -v uvx >/dev/null 2>&1; then
  curl -LsSf https://astral.sh/uv/install.sh | sh
  export PATH="${HOME}/.local/bin:${PATH}"
fi

echo "[install] Playwright Chromium (MCP browser)…"
npx --yes playwright install --with-deps chromium >/dev/null 2>&1 || \
  npx --yes playwright install chromium || true

echo "[install] Scaleway CLI (optionnel, MCP scaleway)…"
if ! command -v scw >/dev/null 2>&1; then
  mkdir -p "${HOME}/bin"
  SCW_VERSION="$(curl -fsSL https://api.github.com/repos/scaleway/scaleway-cli/releases/latest | node -e "let d='';process.stdin.on('data',c=>d+=c);process.stdin.on('end',()=>{try{console.log(JSON.parse(d).tag_name||'')}catch{console.log('')}}")"
  if [ -n "$SCW_VERSION" ]; then
    SCW_VER_NUM="${SCW_VERSION#v}"
    curl -fsSL -o /tmp/scw \
      "https://github.com/scaleway/scaleway-cli/releases/download/${SCW_VERSION}/scaleway-cli_${SCW_VER_NUM}_linux_amd64" \
      && chmod +x /tmp/scw && mv /tmp/scw "${HOME}/bin/scw" \
      && echo "[install] scw ${SCW_VERSION} installé." \
      || echo "[install] scw : téléchargement échoué — MCP scaleway indisponible jusqu’à install manuelle."
  else
    echo "[install] scw : version introuvable — ignoré."
  fi
else
  echo "[install] scw déjà présent: $(command -v scw)"
fi

echo "[install] Seed développement…"
npm run seed:dev

echo "[install] Terminé."
