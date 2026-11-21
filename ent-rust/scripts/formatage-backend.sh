#!/usr/bin/env bash
# Formatage général du backend Rust (`ent-rust/`, hors `web/`) — procédure de
# docs/FORMATAGE_BACKEND.md. NE PAS LANCER en mode `--appliquer` sans validation écrite.
#
#   bash ent-rust/scripts/formatage-backend.sh              # simulation (défaut) : ne modifie rien
#   bash ent-rust/scripts/formatage-backend.sh --appliquer  # formate, puis vérifie
#
# Le script ne commite rien et ne pousse rien.
set -euo pipefail

MODE="${1:---simulation}"
RACINE="$(git rev-parse --show-toplevel)"
cd "$RACINE/ent-rust"

echo "== Contrôles préalables"
BRANCHE="$(git rev-parse --abbrev-ref HEAD)"
[ "$BRANCHE" = "ENT" ] || { echo "ARRÊT : branche $BRANCHE (attendu : ENT)"; exit 1; }
if [ -n "$(git status --porcelain -- .)" ]; then
  echo "ARRÊT : ent-rust/ contient des modifications non commitées :"
  git status --short -- .
  exit 1
fi
command -v cargo >/dev/null || { echo "ARRÊT : cargo introuvable (charger ent-rust/web/scripts/env.sh)"; exit 1; }
rustfmt --version

# Liste des fichiers que rustfmt modifierait (workspace backend uniquement : il n'a
# aucune dépendance par chemin hors de `crates/`, donc `--all` ne sort pas du backend).
LISTE="$(cargo fmt --all --check -- -l 2>/dev/null | tr '\\' '/' | sed -E 's#.*/ent-rust/##' | sort || true)"
NB="$(printf '%s\n' "$LISTE" | grep -c . || true)"
echo "== $NB fichier(s) à formater"
printf '%s\n' "$LISTE" | cut -d/ -f1-2 | sort | uniq -c
HORS="$(printf '%s\n' "$LISTE" | grep -v -E '^crates/[^/]+/.+\.rs$' || true)"
[ -z "$HORS" ] || { echo "ARRÊT : fichiers hors crates/ du backend :"; echo "$HORS"; exit 1; }

if [ "$MODE" != "--appliquer" ]; then
  echo "== Simulation terminée : aucun fichier modifié."
  exit 0
fi

echo "== Formatage"
cargo fmt --all

echo "== Vérification : seuls des .rs de ent-rust/crates ont changé"
MODIFIES="$(git diff --name-only -- "$RACINE")"
HORS="$(printf '%s\n' "$MODIFIES" | grep -v -E '^ent-rust/crates/[^/]+/.+\.rs$' || true)"
[ -z "$HORS" ] || { echo "ARRÊT : fichiers inattendus modifiés :"; echo "$HORS"; exit 1; }
NB_MODIFIES="$(printf '%s\n' "$MODIFIES" | grep -c . || true)"
[ "$NB_MODIFIES" = "$NB" ] || { echo "ARRÊT : $NB_MODIFIES fichiers modifiés, $NB attendus"; exit 1; }
cargo fmt --all --check

echo "== Compilation, tests et Clippy (même résultat qu'avant attendu)"
cargo build --locked --workspace
cargo test --locked --workspace
cargo clippy --locked --workspace --all-targets 2>&1 | grep -E '^warning: `' || true

echo "== Terminé : relire 'git diff --stat', puis commiter (message type dans docs/FORMATAGE_BACKEND.md)."
