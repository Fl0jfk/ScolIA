#!/usr/bin/env bash
#
# Exercice de reprise d'activité (PRA) sur PostgreSQL, exécutable en labo :
#
#   1. inventaire d'intégrité de référence (l'ENT, avant le dump) — optionnel,
#      si une API est joignable ;
#   2. dump logique de la base source (pg_dump format custom) ;
#   3. création d'une base **jetable** et restauration du dump ;
#   4. comparaison des volumétries table par table (psql) ;
#   5. restauration de l'inventaire d'intégrité par l'ENT (optionnel) ;
#   6. suppression de la base jetable, sauf KEEP_SCRATCH=1 ;
#   7. rapport : durées mesurées (RTO observé), RPO = âge du dump.
#
# Variables :
#   DATABASE_URL | ENT_DATABASE_URL   base source (loopback par défaut)
#   ENT_PRA_BASE_JETABLE              défaut : ent_pra_exercice
#   ENT_PG_BIN                        répertoire des binaires PostgreSQL
#   ENT_PRA_API + ENT_PRA_JETON       API ENT pour l'inventaire d'intégrité
#   ALLOW_PROD_BACKUP=1               exercice sur une base distante (humain)
#
# `--autotest` valide les garde-fous (URL locale, masquage, rétention,
# remplacement de base) **sans base ni binaire PostgreSQL** : utilisable en CI.
set -euo pipefail

ici="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=pra-lib.sh
. "${ici}/pra-lib.sh"

autotest() {
    local echecs=0
    verifier() {
        local libelle="$1" attendu="$2" obtenu="$3"
        if [ "$attendu" = "$obtenu" ]; then
            printf '  ok   %s\n' "$libelle"
        else
            printf '  ECHEC %s : attendu « %s », obtenu « %s »\n' "$libelle" "$attendu" "$obtenu"
            echecs=$((echecs + 1))
        fi
    }
    est_locale() {
        if pra_est_locale "$1"; then printf 'oui'; else printf 'non'; fi
    }

    printf 'Autotest des garde-fous PRA\n'
    verifier "hôte 127.0.0.1" "127.0.0.1" \
        "$(pra_hote_de_url 'postgres://u:p@127.0.0.1:5432/scola')"
    verifier "hôte avec @ dans le mot de passe" "10.0.0.9" \
        "$(pra_hote_de_url 'postgres://u:p@ss@10.0.0.9:5432/scola')"
    verifier "hôte IPv6 entre crochets" "[::1]" \
        "$(pra_hote_de_url 'postgres://u:p@[::1]:5432/scola')"
    verifier "loopback 127.0.0.1" "oui" "$(est_locale 'postgres://u:p@127.0.0.1:5432/scola')"
    verifier "loopback localhost" "oui" "$(est_locale 'postgres://scola@localhost/scola')"
    verifier "loopback IPv6" "oui" "$(est_locale 'postgres://u:p@[::1]:5432/scola')"
    verifier "RDB distante refusée" "non" \
        "$(est_locale 'postgres://u:p@51.158.57.80:20566/scola')"
    verifier "hôte cloud refusé" "non" \
        "$(est_locale 'postgres://u:p@db.scaleway.example/scola')"
    verifier "URL masquée sans mot de passe" \
        "postgres://***:***@127.0.0.1:5432/scola" \
        "$(pra_masquer_url 'postgres://scola:secret@127.0.0.1:5432/scola')"
    verifier "base remplacée (identifiants gardés)" \
        "postgres://u:p@127.0.0.1:5432/jetable" \
        "$(pra_url_avec_base 'postgres://u:p@127.0.0.1:5432/scola' jetable)"
    verifier "base remplacée avec paramètres" \
        "postgres://u:p@127.0.0.1:5432/jetable?sslmode=disable" \
        "$(pra_url_avec_base 'postgres://u:p@127.0.0.1:5432/scola?sslmode=disable' jetable)"
    verifier "base ajoutée quand l'URL n'en a pas" \
        "postgres://u:p@127.0.0.1:5432/jetable" \
        "$(pra_url_avec_base 'postgres://u:p@127.0.0.1:5432' jetable)"

    local racine
    racine="$(mktemp -d)"
    mkdir -p "${racine}/20250101T000000Z" "${racine}/20250201T000000Z" "${racine}/20250301T000000Z"
    verifier "rétention garde les N plus récents" "1" "$(pra_a_purger "$racine" 2 | wc -l | tr -d ' ')"
    verifier "rétention : le plus ancien est purgé" "20250101T000000Z" \
        "$(pra_a_purger "$racine" 2 | xargs -n1 basename)"
    verifier "rétention large ne purge rien" "0" "$(pra_a_purger "$racine" 99 | wc -l | tr -d ' ')"
    rm -rf "$racine"

    # Garde-fou hors loopback : doit refuser faute d'ALLOW_PROD_BACKUP.
    local code=0
    ( pra_exiger_locale 'postgres://u:p@51.158.57.80:20566/scola' ) >/dev/null 2>&1 || code=$?
    verifier "URL distante refusée (code 2)" "2" "$code"

    if [ "$echecs" -eq 0 ]; then
        printf 'Autotest OK\n'
        return 0
    fi
    printf 'Autotest en échec : %s\n' "$echecs" >&2
    return 1
}

if [ "${1:-}" = "--autotest" ]; then
    autotest
    exit $?
fi

url="$(pra_url_base)"
pra_exiger_locale "$url"
pg_dump_bin="$(pra_exiger_binaire pg_dump)"
pg_restore_bin="$(pra_exiger_binaire pg_restore)"
psql_bin="$(pra_exiger_binaire psql)"
createdb_bin="$(pra_exiger_binaire createdb)"
dropdb_bin="$(pra_exiger_binaire dropdb)"

base_jetable="${ENT_PRA_BASE_JETABLE:-ent_pra_exercice}"
attendu="${ENT_PRA_BASE_ATTENDUE:-}"
travail="$(mktemp -d)"
dump="${travail}/ent-pra.dump"
inventaire="${travail}/inventaire.json"
nettoyer() {
    if [ "${KEEP_SCRATCH:-}" = "1" ]; then
        printf 'Base jetable conservée : %s (KEEP_SCRATCH=1)\n' "$base_jetable"
    else
        "$dropdb_bin" --if-exists --force "$base_jetable" >/dev/null 2>&1 || true
    fi
    rm -rf "$travail"
}
trap nettoyer EXIT

# 1) Inventaire de référence (si l'API répond) — pour la preuve ligne à ligne.
api="${ENT_PRA_API:-}"
api_restauree="${ENT_PRA_API_RESTAUREE:-}"
if [ -n "$api" ] && [ -n "${ENT_PRA_JETON:-}" ]; then
    if curl -sS -m 30 -H "Authorization: Bearer ${ENT_PRA_JETON}" \
        "${api}/pra/inventaire" -o "$inventaire"; then
        printf 'Inventaire de référence capté (%s octets).\n' "$(wc -c < "$inventaire" | tr -d ' ')"
    else
        printf 'AVERTISSEMENT : inventaire de référence non capté — poursuite.\n' >&2
    fi
else
    rm -f "$inventaire"
fi

# 2) Dump de la base source.
printf '1/4 Dump de %s\n' "$(pra_masquer_url "$url")"
t0="$(date +%s)"
"$pg_dump_bin" --format=custom --compress=9 --no-owner --no-privileges --file "$dump" "$url"
t_dump=$(( $(date +%s) - t0 ))

# 3) Base jetable + restauration.
printf '2/4 Base jetable %s\n' "$base_jetable"
"$dropdb_bin" --if-exists --force "$base_jetable" >/dev/null 2>&1 || true
"$createdb_bin" "$base_jetable"
url_jetable="$(pra_url_avec_base "$url" "$base_jetable")"
t1="$(date +%s)"
"$pg_restore_bin" --no-owner --no-privileges --dbname "$url_jetable" "$dump"
t_restore=$(( $(date +%s) - t1 ))

# 4) Comparaison des volumétries (preuve minimale sans API).
printf '3/4 Comparaison des volumétries\n'
requete="SELECT table_name FROM information_schema.tables WHERE table_schema='public' ORDER BY table_name"
ecarts=0
while IFS= read -r table; do
    [ -n "$table" ] || continue
    n1="$("$psql_bin" -At -c "SELECT count(*) FROM public.\"${table}\"" "$url")"
    n2="$("$psql_bin" -At -c "SELECT count(*) FROM public.\"${table}\"" "$url_jetable")"
    if [ "$n1" != "$n2" ]; then
        printf '  ECART %s : source %s, restaure %s\n' "$table" "$n1" "$n2"
        ecarts=$((ecarts + 1))
    fi
done < <("$psql_bin" -At -c "$requete" "$url")
printf '  %s\n' "$([ "$ecarts" -eq 0 ] && printf 'volumétries identiques' || printf "%s table(s) en écart" "$ecarts")"

# 5) Preuve ligne à ligne via l'ENT (si une API pointe la base restaurée).
if [ -s "$inventaire" ] && [ -n "$api_restauree" ] && [ -n "${ENT_PRA_JETON:-}" ]; then
    if curl -sS -m 120 -X POST -H "Content-Type: application/json" \
        -H "Authorization: Bearer ${ENT_PRA_JETON}" \
        --data-binary "@${inventaire}" "${api_restauree}/pra/inventaire/verifier" \
        -o "${travail}/rapport.json"; then
        printf '  rapport d’intégrité : %s\n' "$(head -c 400 "${travail}/rapport.json")"
    else
        printf 'AVERTISSEMENT : vérification d’intégrité non concluante.\n' >&2
    fi
fi

# 6) Rapport.
printf '4/4 Rapport PRA\n'
printf '  dump              : %ss (%s octets)\n' "$t_dump" "$(wc -c < "$dump" | tr -d ' ')"
printf '  restauration      : %ss\n' "$t_restore"
printf '  RTO observé       : %ss (dump + restauration, hors décision humaine)\n' "$((t_dump + t_restore))"
printf '  RPO               : âge de la dernière sauvegarde conservée\n'
if [ "$ecarts" -ne 0 ]; then
    printf 'EXERCICE EN ÉCHEC : %s table(s) en écart de volumétrie.\n' "$ecarts" >&2
    exit 4
fi
if [ -n "$attendu" ] && [ "$ecarts" -eq 0 ]; then
    printf '  (base attendue : %s)\n' "$attendu"
fi
printf 'Exercice PRA terminé.\n'
