#!/usr/bin/env bash
# Démarre le cluster PostgreSQL local (idempotent) à chaque boot de l'environnement.
set -euo pipefail

if ! command -v pg_lsclusters >/dev/null 2>&1; then
  echo "[start] PostgreSQL non installé — lancez d'abord .cursor/install.sh." >&2
  exit 0
fi

# Récupère version/cluster (ex. "16 main") ; démarre s'il n'est pas déjà online.
read -r PG_VER PG_CLUSTER < <(pg_lsclusters -h | awk 'NR==1 {print $1, $2}')
if [ -z "${PG_VER:-}" ]; then
  echo "[start] Aucun cluster PostgreSQL trouvé." >&2
  exit 0
fi

STATUS="$(pg_lsclusters -h | awk 'NR==1 {print $4}')"
if [ "$STATUS" != "online" ]; then
  echo "[start] Démarrage du cluster PostgreSQL ${PG_VER}/${PG_CLUSTER}…"
  sudo pg_ctlcluster "$PG_VER" "$PG_CLUSTER" start
else
  echo "[start] Cluster PostgreSQL ${PG_VER}/${PG_CLUSTER} déjà démarré."
fi

# Attente de disponibilité du socket.
for _ in $(seq 1 30); do
  if sudo -u postgres pg_isready -q; then
    echo "[start] PostgreSQL prêt."
    break
  fi
  sleep 1
done
