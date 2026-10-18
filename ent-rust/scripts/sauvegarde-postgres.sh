#!/usr/bin/env bash
#
# Sauvegarde logique PostgreSQL de l'ENT (format `custom` de pg_dump) :
#   1. dump compressé ;
#   2. empreinte SHA256 à côté du dump ;
#   3. manifeste lisible (horodatage, hôte masqué, taille, version pg_dump) ;
#   4. rétention glissante (`ENT_BACKUP_KEEP`, défaut 14).
#
# Le manifeste de **fidélité** (nombre de lignes + digest par table) est produit
# par l'ENT lui-même : `GET /pra/inventaire` (Direction / Admin) avant le dump,
# puis `POST /pra/inventaire/verifier` après restauration. Voir
# docs/ent-rust-observabilite-pra.md.
#
# Variables :
#   DATABASE_URL | ENT_DATABASE_URL  base source (loopback par défaut)
#   ENT_BACKUP_DIR                   défaut : ent-rust/backups
#   ENT_BACKUP_KEEP                  défaut : 14
#   ENT_PG_BIN                       répertoire des binaires PostgreSQL
#   ALLOW_PROD_BACKUP=1              autorise une base distante (humain)
set -euo pipefail

ici="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=pra-lib.sh
. "${ici}/pra-lib.sh"

url="$(pra_url_base)"
pra_exiger_locale "$url"
pg_dump_bin="$(pra_exiger_binaire pg_dump)"

racine="${ENT_BACKUP_DIR:-${ici}/../backups}"
garder="${ENT_BACKUP_KEEP:-14}"
horodatage="$(date -u +%Y%m%dT%H%M%SZ)"
dossier="${racine}/${horodatage}"

mkdir -p "$dossier"
dump="${dossier}/ent.dump"

printf 'Sauvegarde de %s\n' "$(pra_masquer_url "$url")"
printf '  -> %s\n' "$dump"

"$pg_dump_bin" \
    --format=custom \
    --compress=9 \
    --no-owner \
    --no-privileges \
    --file "$dump" \
    "$url"

sha="$(pra_sha256 "$dump")"
printf '%s  %s\n' "$sha" "$(basename "$dump")" > "${dump}.sha256"

taille="$(wc -c < "$dump" | tr -d ' ')"
git_sha="$(git -C "${ici}/.." rev-parse --short HEAD 2>/dev/null || echo dev)"

{
    printf 'ent-rust sauvegarde logique\n'
    printf 'cree_le=%s\n' "$horodatage"
    printf 'base=%s\n' "$(pra_masquer_url "$url")"
    printf 'git_sha=%s\n' "$git_sha"
    printf 'pg_dump=%s\n' "$("$pg_dump_bin" --version)"
    printf 'fichier=%s\n' "$(basename "$dump")"
    printf 'octets=%s\n' "$taille"
    printf 'sha256=%s\n' "$sha"
    printf 'restauration=%s\n' "scripts/restauration-postgres.sh --depuis ${dump} --base <cible>"
    printf 'fidelite=%s\n' "GET /pra/inventaire avant, POST /pra/inventaire/verifier après restauration"
} > "${dossier}/manifeste.txt"

# Rétention glissante.
while IFS= read -r ancien; do
    [ -n "$ancien" ] || continue
    printf '  purge (rétention %s) : %s\n' "$garder" "$(basename "$ancien")"
    rm -rf "$ancien"
done < <(pra_a_purger "$racine" "$garder")

printf 'Sauvegarde terminée : %s octets, sha256 %s\n' "$taille" "${sha:0:12}…"
printf 'Manifeste : %s\n' "${dossier}/manifeste.txt"
