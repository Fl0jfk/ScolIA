#!/bin/bash
# Base de LABO locale uniquement (données fictives) : la recrée vide, relance le serveur
# (migrations, tâches de fond) puis amorce le labo. Écrit labo.json à côté du script.
# Prérequis : `service postgresql start`, rôle ent_ci/ci, serveur compilé par
#   cd ent-rust/web && cargo leptos build --bin-features labo
set -euo pipefail
cd "$(dirname "$0")"
WEB="$(cd ../../../../ent-rust/web && pwd)"
su postgres -c "psql -c 'DROP DATABASE IF EXISTS ent_ci_test_a WITH (FORCE)' -c 'CREATE DATABASE ent_ci_test_a OWNER ent_ci'" >/dev/null
pkill -f target/debug/ent-web-server || true; sleep 1
cd "$WEB"
ENT_DATABASE_URL=postgres://ent_ci:ci@127.0.0.1:5432/ent_ci_test_a ENT_ENV=lab ENT_ALLOW_LABO=1 ENT_TACHES_ELEVES=0 \
LEPTOS_OUTPUT_NAME=ent-web LEPTOS_SITE_ROOT=target/site LEPTOS_SITE_PKG_DIR=pkg LEPTOS_SITE_ADDR=127.0.0.1:8090 \
  nohup target/debug/ent-web-server > /tmp/ent-web-demo.log 2>&1 &
for i in $(seq 1 120); do curl -s -o /dev/null http://127.0.0.1:8090/web/connexion && break; sleep 1; done
cd - >/dev/null
curl -s -X POST -H 'content-type: application/json' -d '{}' http://127.0.0.1:8090/labo/bootstrap -o labo.json
echo "labo prêt : $(python3 -c "import json;print(json.load(open('labo.json'))['etablissement_id'])")"
