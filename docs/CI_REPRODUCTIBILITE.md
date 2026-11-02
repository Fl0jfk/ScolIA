# Reproductibilité et CI de l'ENT Rust (proposition)

Date : 2026-11-02 — Statut : **proposition** ; aucun fichier suivi par Git n'est modifié, aucun workflow créé, aucun paramètre global de la machine changé.

## 1. Constat

- `ent-rust/.gitignore` contient `Cargo.lock` : **aucun fichier de verrouillage n'est versionné**, ni pour le workspace principal ni pour `ent-rust/web/`. Deux compilations à des dates différentes peuvent donc utiliser des versions de dépendances différentes (comportement et sécurité non reproductibles).
- **Aucune CI Rust** : les workflows (`ci.yml`, `deploy-*.yml`) ne concernent que Next.js, sur `main` / `dev`. Rien ne s'exécute sur `ENT`.
- Compilations de développement très lourdes : `target-web` a atteint 25 Go (détail dans `LEPTOS_CALL_PROTOTYPE.md` §8.4), dont l'essentiel en informations de débogage.

## 2. Versionner `Cargo.lock`

L'ENT est une **application** (pas une bibliothèque publiée) : la règle Cargo est de versionner le fichier de verrouillage.

Proposition :
1. retirer la ligne `Cargo.lock` de `ent-rust/.gitignore` ;
2. générer et versionner `ent-rust/Cargo.lock` (workspace principal) et `ent-rust/web/Cargo.lock` (prototype, déjà présent sur le disque) ;
3. compiler en CI avec `--locked` (échec si le fichier n'est pas à jour) ;
4. mises à jour volontaires : `cargo update` dans un commit dédié, relu, tests verts ; option : un outil d'alerte de vulnérabilités (`cargo audit` / `cargo deny`) en CI.

Conséquence : le premier `Cargo.lock` du workspace principal figera les versions **du jour de sa génération** (celles déjà résolues localement pour les tests du prototype).

## 3. Compilation de développement allégée

Proposition pour `ent-rust/web/Cargo.toml` (et à terme `ent-rust/Cargo.toml`) :

```toml
[profile.dev]
debug = "line-tables-only"     # numéros de ligne dans les traces, sans variables détaillées

[profile.dev.package."*"]
debug = false                  # aucune info de débogage pour les dépendances
```

Effets attendus : fichiers `.pdb` et bibliothèques beaucoup plus petits, édition de liens plus rapide ; les traces d'erreur gardent les numéros de ligne de notre code. Inconvénient : moins d'informations dans un débogueur pas à pas (réactivable ponctuellement).

Mesure réalisée (même périmètre que les tests du prototype, dossier séparé `_outils-scolia/target-mesure-debug`, options passées en ligne de commande, aucun fichier modifié) : voir §3 bis.

### 3 bis. Mesure

_(complétée ci-dessous)_

## 4. CI proposée

Fichier proposé : `.github/workflows/ent-rust.yml`, **distinct** des workflows Next.js, déclenché uniquement sur la branche `ENT` et les chemins `ent-rust/**`. Permissions en lecture seule, **aucun secret**, **aucun déploiement**.

```yaml
name: ENT Rust

on:
  push:
    branches: [ENT]
    paths: ["ent-rust/**", ".github/workflows/ent-rust.yml"]
  pull_request:
    branches: [ENT]
    paths: ["ent-rust/**", ".github/workflows/ent-rust.yml"]

permissions:
  contents: read

env:
  CARGO_TERM_COLOR: always
  CARGO_INCREMENTAL: "0"              # inutile en CI, alourdit le cache
  CARGO_PROFILE_DEV_DEBUG: "0"        # pas d'info de débogage en CI
  RUST_MIN_STACK: "16777216"          # même réglage qu'ent-api (ui.rs)

jobs:
  backend:
    runs-on: ubuntu-latest
    timeout-minutes: 45
    services:
      postgres:
        image: postgres:18
        env:
          POSTGRES_USER: ent_ci
          POSTGRES_PASSWORD: ent_ci       # base jetable du runner, pas un secret
          POSTGRES_DB: ent_ci
        ports: ["5432:5432"]
        options: >-
          --health-cmd "pg_isready -U ent_ci" --health-interval 5s --health-retries 20
    env:
      ENT_DATABASE_URL: postgres://ent_ci:ent_ci@127.0.0.1:5432/ent_ci
    defaults:
      run:
        working-directory: ent-rust
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@master
        with:
          toolchain: "1.99.0"
          components: clippy
      - uses: Swatinem/rust-cache@v2
        with:
          workspaces: "ent-rust -> target"
      - run: cargo build --locked --workspace
      - run: cargo test --locked --workspace
      - run: cargo clippy --locked --workspace     # d'abord informatif, bloquant plus tard

  web:
    runs-on: ubuntu-latest
    timeout-minutes: 60
    services:
      postgres:
        image: postgres:18
        env: { POSTGRES_USER: ent_ci, POSTGRES_PASSWORD: ent_ci, POSTGRES_DB: ent_ci }
        ports: ["5432:5432"]
        options: >-
          --health-cmd "pg_isready -U ent_ci" --health-interval 5s --health-retries 20
    env:
      ENT_DATABASE_URL: postgres://ent_ci:ent_ci@127.0.0.1:5432/ent_ci
      ENT_ENV: lab
      ENT_ALLOW_LABO: "1"
    defaults:
      run:
        working-directory: ent-rust/web
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@master
        with:
          toolchain: "1.99.0"
          targets: wasm32-unknown-unknown
      - uses: Swatinem/rust-cache@v2
        with:
          workspaces: "ent-rust/web -> target"   # inclut target/front (WebAssembly)
      - name: Outils Leptos (versions épinglées, mis en cache)
        uses: taiki-e/install-action@v2
        with:
          tool: cargo-leptos@0.3.11,wasm-bindgen-cli@0.2.130
      - name: wasm-opt (binaryen version_123)
        run: |
          curl -sSL https://github.com/WebAssembly/binaryen/releases/download/version_123/binaryen-version_123-x86_64-linux.tar.gz | tar xz
          echo "$PWD/binaryen-version_123/bin" >> "$GITHUB_PATH"
      - run: cargo test --locked -p ent-web-api -p ent-web-app --features ent-web-app/ssr
      - run: cargo leptos build --release --precompress
      - run: cargo test --locked -p ent-web-server --test contre_backend -- --test-threads=1
      - name: Taille des fichiers envoyés au navigateur
        run: ls -l target/site/pkg
```

### Stratégie de cache
- `Swatinem/rust-cache` : met en cache le registre Cargo et `target/` **par workspace**, avec une clé qui inclut le `Cargo.lock`, la version de Rust et le système. Les compilations WebAssembly de `cargo-leptos` (`target/front/…`) sont dans le même dossier, donc couvertes.
- `CARGO_INCREMENTAL=0` et pas d'info de débogage en CI : cache plus petit et plus rapide à restaurer.
- Outils (`cargo-leptos`, `wasm-bindgen-cli`) installés en binaires précompilés par version épinglée ; `wasm-opt` téléchargé à version fixe. **Les versions doivent rester alignées** avec D-010 (`wasm-bindgen` CLI = crate).
- Pas de cache des artefacts release : reconstruits à chaque exécution (évite de publier un binaire périmé).

### Points d'attention
- Sur Linux, la compilation d'OpenSSL (`webauthn-rs`) utilise le Perl et le compilateur du runner : pas de Strawberry Perl nécessaire.
- `ent-db` refuse toute base non locale : la base de service de CI est en `127.0.0.1`, donc acceptée.
- Durée estimée à froid : 15 à 25 min par job ; avec cache : quelques minutes.
- Le fichier de workflow doit exister sur la branche `ENT` pour s'y déclencher ; il n'affecte pas `main` / `dev`.

## 5. Commandes locales (Windows, environnement isolé)

```bash
. ent-rust/web/scripts/env.sh
bash ent-rust/web/scripts/pg.sh start
cd ent-rust/web
cargo test --locked -p ent-web-api -p ent-web-app --features ent-web-app/ssr
cargo leptos build --release --precompress
cargo test --locked -p ent-web-server --test contre_backend -- --test-threads=1
```

## 6. Décisions à prendre
1. Versionner les `Cargo.lock` (recommandé) — modification de `ent-rust/.gitignore`.
2. Adopter le profil de développement allégé (§3).
3. Créer le workflow `ent-rust.yml` (§4) ; `clippy` informatif puis bloquant.
4. Ajouter `cargo audit` / `cargo deny` (vulnérabilités, licences — utile aussi pour D-005 et D-006).
