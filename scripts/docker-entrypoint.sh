#!/bin/sh
set -eu

if [ -n "${DATABASE_URL:-}" ]; then
  export SCOLA_AUTO_MIGRATE=1
  echo "[entrypoint] Application des migrations SQL…"
  node /app/scripts/apply-migrations-direct.mjs
else
  echo "[entrypoint] DATABASE_URL absent — migrations ignorées (dev local sans BDD)."
fi

exec node /app/server.js
