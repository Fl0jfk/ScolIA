#!/usr/bin/env bash
#
# Restauration d'une sauvegarde logique ENT.
#
#   scripts/restauration-postgres.sh --depuis <ent.dump> --base <nom_base>
#
# La base cible est reconstruite (`--clean --if-exists`) : les objets présents
# sont supprimés avant recréation. Par sécurité :
#   - confirmation explicite obligatoire (CONFIRM_RESTORE=oui) ;
#   - loopback par défaut (`ALLOW_PROD_BACKUP=1` pour une base distante) ;
#   - l'empreinte SHA256 du dump est vérifiée si `<dump>.sha256` existe.
#
# Après restauration, vérifier la **fidélité ligne à ligne** :
#   POST /pra/inventaire/verifier (Direction / Admin) avec l'inventaire pris
#   avant la sauvegarde. Voir docs/ent-rust-observabilite-pra.md.
set -euo pipefail

ici="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=pra-lib.sh
. "${ici}/pra-lib.sh"

depuis=""
base=""
while [ "$#" -gt 0 ]; do
    case "$1" in
        --depuis)
            depuis="${2:-}"
            shift 2
            ;;
        --base)
            base="${2:-}"
            shift 2
            ;;
        -h | --help)
            printf 'usage: %s --depuis <ent.dump> --base <nom_base>\n' "$0"
            exit 0
            ;;
        *)
            printf 'option inconnue : %s\n' "$1" >&2
            exit 1
            ;;
    esac
done

if [ -z "$depuis" ] || [ -z "$base" ]; then
    printf 'usage: %s --depuis <ent.dump> --base <nom_base>\n' "$0" >&2
    exit 1
fi
if [ ! -f "$depuis" ]; then
    printf 'dump introuvable : %s\n' "$depuis" >&2
    exit 1
fi

url_source="$(pra_url_base)"
url_cible="$(pra_url_avec_base "$url_source" "$base")"
pra_exiger_locale "$url_cible"

if [ "${CONFIRM_RESTORE:-}" != "oui" ]; then
    printf 'REFUS : restauration destructive sur %s.\n' "$(pra_masquer_url "$url_cible")" >&2
    printf 'Relancer avec CONFIRM_RESTORE=oui après vérification de la cible.\n' >&2
    exit 2
fi

if [ -f "${depuis}.sha256" ]; then
    attendu="$(awk '{print $1}' "${depuis}.sha256")"
    obtenu="$(pra_sha256 "$depuis")"
    if [ "$attendu" != "$obtenu" ]; then
        printf 'REFUS : empreinte SHA256 du dump invalide (attendu %s, obtenu %s).\n' \
            "${attendu:0:12}…" "${obtenu:0:12}…" >&2
        exit 2
    fi
    printf 'Empreinte SHA256 vérifiée (%s…).\n' "${obtenu:0:12}"
fi

pg_restore_bin="$(pra_exiger_binaire pg_restore)"

printf 'Restauration %s -> %s\n' "$(basename "$depuis")" "$(pra_masquer_url "$url_cible")"
debut="$(date +%s)"
"$pg_restore_bin" \
    --clean \
    --if-exists \
    --no-owner \
    --no-privileges \
    --dbname "$url_cible" \
    "$depuis"
duree=$(( $(date +%s) - debut ))

printf 'Restauration terminée en %ss.\n' "$duree"
printf 'Vérifier la fidélité : POST /pra/inventaire/verifier avec l’inventaire pris avant le dump.\n'
