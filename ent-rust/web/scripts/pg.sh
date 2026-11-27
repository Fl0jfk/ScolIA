#!/usr/bin/env bash
# PostgreSQL portable du prototype (aucun service Windows).
#   bash ent-rust/web/scripts/pg.sh start|stop|status|psql|reset
set -euo pipefail
. "$(dirname "${BASH_SOURCE[0]}")/env.sh"
DATA="$SCOLIA_OUTILS/pgdata"
LOG="$SCOLIA_OUTILS/pg.log"

case "${1:-status}" in
  start)
    # Sorties redirigées : sinon pg_ctl garde le terminal ouvert.
    pg_ctl -D "$DATA" -l "$LOG" -w start >/dev/null 2>&1 </dev/null
    pg_ctl -D "$DATA" status | head -1
    ;;
  stop)
    pg_ctl -D "$DATA" -m fast -w stop
    ;;
  status)
    pg_ctl -D "$DATA" status || true
    netstat -ano | grep -E ":${PGPORT}\s" || true
    ;;
  psql)
    PGPASSWORD="$(tr -d '\r\n' < "$SCOLIA_OUTILS/pg-ent_local.pwd")" \
      psql -h 127.0.0.1 -p "$PGPORT" -U ent_local -d ent_proto_test
    ;;
  reset)
    # Recrée la base de test vide (données de labo uniquement).
    export PGPASSWORD="$(tr -d '\r\n' < "$SCOLIA_OUTILS/pg-superuser.pwd")"
    psql -h 127.0.0.1 -p "$PGPORT" -U postgres -v ON_ERROR_STOP=1 -q \
      -c "DROP DATABASE IF EXISTS ent_proto_test WITH (FORCE);" \
      -c "CREATE DATABASE ent_proto_test OWNER ent_local;"
    echo "base ent_proto_test recréée"
    ;;
  *)
    echo "usage : pg.sh start|stop|status|psql|reset" >&2
    exit 2
    ;;
esac
