#!/usr/bin/env bash
# shellcheck shell=bash
#
# Garde-fous et utilitaires communs aux scripts de sauvegarde / PRA de l'ENT.
#
# Règle non négociable (même philosophie que `apply-migrations-direct.mjs`) :
# aucune opération destructrice ou de sauvegarde sur une base **distante**
# (RDB Scaleway) sans `ALLOW_PROD_BACKUP=1` posé explicitement par un humain.
# En labo / CI locale, l'hôte doit être loopback.
#
# Sourcer ce fichier, ne pas l'exécuter :
#   . "$(dirname "$0")/pra-lib.sh"

set -euo pipefail

# Hôte d'une URL PostgreSQL, robuste aux mots de passe contenant `@` ou `:`.
#   postgres://user:p@ss@host:5432/db?x=1  ->  host
pra_hote_de_url() {
    local reste="${1#*://}"
    case "$reste" in
        */*) reste="${reste%%/*}" ;;
    esac
    case "$reste" in
        *@*) reste="${reste##*@}" ;;
    esac
    case "$reste" in
        '['*':'*']'*) printf '%s]' "${reste%%]*}" ;;
        *) printf '%s' "${reste%%:*}" ;;
    esac
}

# Rédige l'URL pour les journaux (jamais de mot de passe affiché).
pra_masquer_url() {
    local url="$1"
    local reste="${url#*://}"
    local creds=""
    case "$reste" in
        *@*) creds="${reste%%@*}"; reste="${reste#*@}" ;;
    esac
    if [ -n "$creds" ]; then
        printf '%s://***:***@%s' "${url%%://*}" "$reste"
    else
        printf '%s' "$url"
    fi
}

# Même URL avec un autre nom de base (les identifiants sont conservés).
pra_url_avec_base() {
    local url="$1"
    local base="$2"
    local params="" sans="$url"
    case "$url" in
        *'?'*)
            params="?${url#*\?}"
            sans="${url%%\?*}"
            ;;
    esac
    local apres="${sans#*://}"
    case "$apres" in
        */*) printf '%s/%s%s' "${sans%/*}" "$base" "$params" ;;
        *) printf '%s/%s%s' "$sans" "$base" "$params" ;;
    esac
}

# 0 si l'URL pointe le loopback (base de dev / labo).
pra_est_locale() {
    local hote
    hote="$(pra_hote_de_url "$1")"
    case "$hote" in
        127.0.0.1 | localhost | ::1 | '[::1]') return 0 ;;
        *) return 1 ;;
    esac
}

# Refuse une URL non locale sauf `ALLOW_PROD_BACKUP=1`.
pra_exiger_locale() {
    local url="$1"
    if pra_est_locale "$url"; then
        return 0
    fi
    if [ "${ALLOW_PROD_BACKUP:-}" = "1" ]; then
        printf 'AVERTISSEMENT : base distante (%s) — ALLOW_PROD_BACKUP=1 assumé.\n' \
            "$(pra_masquer_url "$url")" >&2
        return 0
    fi
    printf "REFUS : l'URL n'est pas loopback (%s).\n" "$(pra_masquer_url "$url")" >&2
    printf 'Sauvegardes / restaurations sur une base distante = validation humaine\n' >&2
    printf 'explicite, puis ALLOW_PROD_BACKUP=1.\n' >&2
    return 2
}

# URL de base : `DATABASE_URL` puis `ENT_DATABASE_URL`.
pra_url_base() {
    local url="${DATABASE_URL:-${ENT_DATABASE_URL:-}}"
    if [ -z "$url" ]; then
        printf 'DATABASE_URL (ou ENT_DATABASE_URL) requis.\n' >&2
        return 1
    fi
    printf '%s' "$url"
}

# Emplacement des binaires PostgreSQL (pg_dump, pg_restore, psql, createdb…).
# `ENT_PG_BIN` permet de pointer une distribution embarquée (VM de dev).
pra_chemin_binaire() {
    local nom="$1"
    if [ -n "${ENT_PG_BIN:-}" ] && [ -x "${ENT_PG_BIN}/${nom}" ]; then
        printf '%s' "${ENT_PG_BIN}/${nom}"
        return 0
    fi
    command -v "$nom" 2>/dev/null
}

# Échoue avec un message actionnable si un binaire manque.
pra_exiger_binaire() {
    local nom="$1"
    local chemin
    chemin="$(pra_chemin_binaire "$nom" || true)"
    if [ -z "$chemin" ]; then
        printf 'Binaire %s introuvable.\n' "$nom" >&2
        printf 'Installer les utilitaires PostgreSQL (paquet postgresql-client) ou\n' >&2
        printf 'poser ENT_PG_BIN=/chemin/vers/bin.\n' >&2
        return 3
    fi
    printf '%s' "$chemin"
}

# Empreinte SHA256 portable (macOS `shasum`, Linux `sha256sum`).
pra_sha256() {
    local fichier="$1"
    if command -v sha256sum >/dev/null 2>&1; then
        sha256sum "$fichier" | awk '{print $1}'
    elif command -v shasum >/dev/null 2>&1; then
        shasum -a 256 "$fichier" | awk '{print $1}'
    else
        printf 'ni sha256sum ni shasum disponibles\n' >&2
        return 1
    fi
}

# Rétention : liste des répertoires de sauvegarde à supprimer (les plus
# anciens au-delà de `garder`). Les noms sont horodatés donc triables.
pra_a_purger() {
    local racine="$1"
    local garder="${2:-14}"
    [ -d "$racine" ] || return 0
    local -a dossiers=()
    while IFS= read -r ligne; do
        dossiers+=("$ligne")
    done < <(find "$racine" -mindepth 1 -maxdepth 1 -type d -name '20*' | sort)
    local total="${#dossiers[@]}"
    local trop=$((total - garder))
    local i=0
    while [ "$i" -lt "$trop" ] && [ "$i" -lt "$total" ]; do
        printf '%s\n' "${dossiers[$i]}"
        i=$((i + 1))
    done
}
