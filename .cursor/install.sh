#!/usr/bin/env bash
# Bootstrap idempotent Cloud Agent / dev local pour docslapro (ScolIA).
# - PostgreSQL + rôle/base
# - npm ci
# - .env.local + mcp.local.env
# - drizzle-kit push
# - outils agents (uvx, Playwright Chromium, Scaleway CLI, github-mcp-server)
# - seed:dev
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

# shellcheck disable=SC1091
source "$REPO_ROOT/.cursor/ensure-agent-path.sh"

PG_USER="scola"
PG_PASSWORD="scola_dev_pwd"
PG_DB="scola"

echo "[install] PostgreSQL…"
if ! command -v pg_ctlcluster >/dev/null 2>&1; then
  echo "[install] Installation de PostgreSQL via apt…"
  sudo apt-get update -qq
  sudo DEBIAN_FRONTEND=noninteractive apt-get install -y -qq postgresql postgresql-contrib
fi
if ! command -v psql >/dev/null 2>&1; then
  sudo DEBIAN_FRONTEND=noninteractive apt-get install -y -qq postgresql-client || true
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

echo "[install] Synchronisation du schéma (drizzle-kit push)…"
# L'historique de migrations contient un renommage clerk_*→external_* incohérent
# sur base fraîche ; drizzle-kit push aligne la base sur db/schema.ts (source de vérité ORM).
npx --yes drizzle-kit push --force

install_github_mcp_server() {
  local dest="${HOME}/.local/bin/github-mcp-server"
  if [[ -x "$dest" ]] || command -v github-mcp-server >/dev/null 2>&1; then
    return 0
  fi
  mkdir -p "${HOME}/.local/bin" /tmp/github-mcp-unpack
  local json url
  json="$(curl -fsSL https://api.github.com/repos/github/github-mcp-server/releases/latest 2>/dev/null || true)"
  if [[ -z "$json" ]]; then
    echo "[install] github-mcp-server : API GitHub indisponible."
    return 0
  fi
  url="$(printf '%s' "$json" | node -e "
let d='';
process.stdin.on('data',c=>d+=c);
process.stdin.on('end',()=>{
  try {
    const j=JSON.parse(d);
    const a=(j.assets||[]).find(x=>/Linux_x86_64\\.tar\\.gz\$/.test(x.name||''));
    process.stdout.write(a && a.browser_download_url ? a.browser_download_url : '');
  } catch { process.stdout.write(''); }
});
")"
  if [[ -z "$url" ]]; then
    echo "[install] github-mcp-server : archive Linux_x86_64 introuvable."
    return 0
  fi
  if curl -fsSL -o /tmp/github-mcp-server.tar.gz "$url" \
    && tar -xzf /tmp/github-mcp-server.tar.gz -C /tmp/github-mcp-unpack \
    && find /tmp/github-mcp-unpack -type f -name 'github-mcp-server' -exec mv {} "$dest" \; \
    && chmod +x "$dest"; then
    echo "[install] github-mcp-server installé → ${dest}"
  else
    echo "[install] github-mcp-server : téléchargement/extraction échoué (optionnel)."
  fi
  rm -rf /tmp/github-mcp-server.tar.gz /tmp/github-mcp-unpack
}

echo "[install] uv (uvx) pour MCP fetch…"
if [[ ! -x "${HOME}/.local/bin/uvx" ]] && ! command -v uvx >/dev/null 2>&1; then
  curl -LsSf https://astral.sh/uv/install.sh | sh
fi
# shellcheck disable=SC1091
source "$REPO_ROOT/.cursor/ensure-agent-path.sh"
if command -v uvx >/dev/null 2>&1; then
  echo "[install] uvx: $(command -v uvx)"
else
  echo "[install] uvx toujours absent après install — fetch MCP/repli limité."
fi

echo "[install] Playwright Chromium (MCP browser + repli)…"
if npx --yes playwright install --with-deps chromium; then
  echo "[install] Playwright Chromium OK."
else
  echo "[install] Playwright --with-deps a échoué, tentative sans deps…"
  npx --yes playwright install chromium || echo "[install] Chromium non installé — browser repli limité."
fi

echo "[install] Scaleway CLI (optionnel, MCP scaleway)…"
if [[ ! -x "${HOME}/bin/scw" ]] && ! command -v scw >/dev/null 2>&1; then
  mkdir -p "${HOME}/bin"
  SCW_VERSION="$(curl -fsSL https://api.github.com/repos/scaleway/scaleway-cli/releases/latest 2>/dev/null \
    | node -e "let d='';process.stdin.on('data',c=>d+=c);process.stdin.on('end',()=>{try{console.log(JSON.parse(d).tag_name||'')}catch{console.log('')}})")" \
    || SCW_VERSION=""
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
fi
# shellcheck disable=SC1091
source "$REPO_ROOT/.cursor/ensure-agent-path.sh"
if command -v scw >/dev/null 2>&1; then
  echo "[install] scw: $(command -v scw)"
fi

echo "[install] github-mcp-server (optionnel)…"
install_github_mcp_server
# shellcheck disable=SC1091
source "$REPO_ROOT/.cursor/ensure-agent-path.sh"

PROFILE_LINE='export PATH="$HOME/.local/bin:$HOME/bin:/usr/local/bin:$PATH"'
for f in "${HOME}/.bashrc" "${HOME}/.profile"; do
  if [[ -f "$f" ]] && ! grep -Fq '.local/bin' "$f"; then
    printf '\n# ScolIA agent PATH\n%s\n' "$PROFILE_LINE" >> "$f"
  fi
done
if [[ -d /etc/profile.d ]] && [[ -w /etc/profile.d || -n "$(command -v sudo)" ]]; then
  if [[ ! -f /etc/profile.d/scola-agent-path.sh ]]; then
    if printf '%s\n' "$PROFILE_LINE" > /etc/profile.d/scola-agent-path.sh 2>/dev/null; then
      true
    else
      echo "$PROFILE_LINE" | sudo tee /etc/profile.d/scola-agent-path.sh >/dev/null 2>&1 || true
    fi
  fi
fi

echo "[install] Seed développement…"
npm run seed:dev

echo "[install] Terminé."
bash "$REPO_ROOT/.cursor/tools/status.sh" || true
