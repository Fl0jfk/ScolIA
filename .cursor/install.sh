#!/usr/bin/env bash
# Bootstrap idempotent de l'environnement de développement Cloud Agent pour docslapro (ScolIA).
# - Installe PostgreSQL (si absent)
# - Installe les dépendances npm (npm ci)
# - Prépare le rôle/la base PostgreSQL locale
# - Génère .env.local (secret dev jetable) si absent
# - Synchronise le schéma Drizzle (drizzle-kit push)
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

PG_USER="scola"
PG_PASSWORD="scola_dev_pwd"
PG_DB="scola"

echo "[install] PostgreSQL…"
if ! command -v pg_ctlcluster >/dev/null 2>&1; then
  echo "[install] Installation de PostgreSQL via apt…"
  sudo apt-get update -qq
  sudo DEBIAN_FRONTEND=noninteractive apt-get install -y -qq postgresql postgresql-contrib
fi

# Démarre le cluster pour pouvoir créer rôle/base et pousser le schéma.
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
DEFAULT_TENANT_LABEL=Instance de développement
ENV
  echo "[install] .env.local généré."
else
  echo "[install] .env.local déjà présent — conservé."
fi

echo "[install] Synchronisation du schéma (drizzle-kit push)…"
# L'historique de migrations contient un renommage clerk_*→external_* incohérent
# sur base fraîche ; drizzle-kit push aligne la base sur db/schema.ts (source de vérité ORM).
npx --yes drizzle-kit push --force

echo "[install] Terminé."
