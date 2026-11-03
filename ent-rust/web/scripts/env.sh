# Environnement du prototype Leptos — à sourcer dans Git Bash :
#   . ent-rust/web/scripts/env.sh
# Ne modifie rien de global : variables valables pour le terminal courant uniquement.
# Tout l'outillage vit dans le dossier dédié `_outils-scolia` (à côté du dépôt).

WEB_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
REPO_DIR="$(cd "$WEB_DIR/../.." && pwd)"
export SCOLIA_OUTILS="${SCOLIA_OUTILS:-$(cd "$REPO_DIR/.." && pwd)/_outils-scolia}"

# Chaîne Rust dédiée (1.99.0 + cible wasm32), cache cargo et sorties de compilation.
export RUSTUP_HOME="$SCOLIA_OUTILS/rustup"
export CARGO_HOME="$SCOLIA_OUTILS/cargo"
export RUSTUP_TOOLCHAIN="1.99.0"
# Compilations du frontend (profil allégé). L'ancien dossier `target-web` (profil complet)
# n'est plus utilisé ; sa suppression attend une validation.
export CARGO_TARGET_DIR="$SCOLIA_OUTILS/target-web-leger"
# Compilations du workspace principal (backend), séparées.
export SCOLIA_TARGET_ENT="$SCOLIA_OUTILS/target-ent"

# cargo-leptos, Perl portable (compilation d'OpenSSL pour webauthn-rs), PostgreSQL portable.
# Seul perl/perl/bin est ajouté (pas perl/c/bin, pour ne pas masquer le compilateur MSVC).
# bin/ contient aussi wasm-bindgen, et binaryen/bin wasm-opt : cargo-leptos les trouve
# dans le PATH et ne télécharge rien dans AppData.
export PATH="$SCOLIA_OUTILS/bin:$SCOLIA_OUTILS/binaryen/bin:$SCOLIA_OUTILS/perl/perl/bin:$SCOLIA_OUTILS/pgsql/bin:$HOME/.cargo/bin:$PATH"

# Base de test locale (127.0.0.1 uniquement, données de labo fictives).
export PGPORT=54329
export PGHOST=127.0.0.1
ENT_LOCAL_PWD="$(tr -d '\r\n' < "$SCOLIA_OUTILS/pg-ent_local.pwd")"
export ENT_DATABASE_URL="postgres://ent_local:${ENT_LOCAL_PWD}@127.0.0.1:54329/ent_proto_test"
unset ENT_LOCAL_PWD

# Serveur du prototype : environnement labo (jamais prod), écoute locale.
export ENT_ENV="lab"
export ENT_ALLOW_LABO="1"
export ENT_API_BIND="127.0.0.1:8090"
export RUST_MIN_STACK="16777216"
export LEPTOS_OUTPUT_NAME="ent-web"
export LEPTOS_SITE_ROOT="$WEB_DIR/target/site"
export LEPTOS_SITE_PKG_DIR="pkg"
export LEPTOS_SITE_ADDR="127.0.0.1:8090"
