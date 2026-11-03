# Reproductibilité et CI de l'ENT Rust

Date : 2026-11-03 — Statut : **en place** (D-015) : lockfiles versionnés, profil allégé, audits, workflow CI.
Le workflow n'a pas encore tourné sur GitHub (aucun push) : toutes ses étapes ont été **rejouées localement** (§6).

## 1. Workspaces concernés

| Workspace | Manifeste | Contenu |
|---|---|---|
| Backend | `ent-rust/Cargo.toml` | 15 crates dont `ent-api`, `ent-db`, `ent-auth`, et la nouvelle `ent-types` |
| Frontend | `ent-rust/web/Cargo.toml` | `ent-web-ui`, `ent-web-api`, `ent-web-app`, `ent-web-server` (dépend d'`ent-api` et d'`ent-types` par chemin) |

Aucun autre projet Rust dans le dépôt (vérifié : seuls ces deux manifestes ont une section `[workspace]`).

## 2. `Cargo.lock`

- Avant : `ent-rust/.gitignore` contenait `Cargo.lock` (tous les lockfiles ignorés).
- Après (règle ciblée) :
  ```
  Cargo.lock
  !/Cargo.lock
  !/web/Cargo.lock
  ```
  Les deux lockfiles des workspaces sont versionnés ; tout autre `Cargo.lock` (crate compilée isolément) reste ignoré (vérifié avec `git check-ignore`).
- `ent-rust/Cargo.lock` (≈ 84 Ko, 340 crates) généré le 2026-11-03 ; `ent-rust/web/Cargo.lock` (437 crates).
- CI : toutes les commandes Cargo utilisent `--locked`.
- Mises à jour : `cargo update` dans un commit dédié, relu, CI verte.

## 3. Profil de développement allégé

Dans les deux workspaces :

```toml
[profile.dev]
debug = "line-tables-only"

[profile.dev.package."*"]
debug = false        # (+ opt-level = 1 côté frontend, déjà présent)
```

| Vérification (2026-11-03) | Résultat |
|---|---|
| Build debug backend (à froid, OpenSSL compris) | OK, 413 s ; dossier complet 3,2 Go |
| Tests backend (270) | OK |
| Build debug frontend (`cargo leptos build`) | OK |
| Tests frontend (unitaires + contre le vrai backend) | OK |
| Build release (`cargo leptos build --release --precompress`) | OK (le profil release n'est pas modifié) |
| Informations de débogage | **suffisantes** : une erreur volontaire affiche `seed_labo_web.rs:14:42` et la pile avec fichiers et lignes de notre code |
| Taille (mesure du 2026-11-03, même périmètre) | `.pdb` des tests 1 248 → 277 Mo ; `.pdb` serveur 1 025 → 276 Mo ; bibliothèque serveur 2 219 → 218 Mo ; dossier ≈ 21 → 3,6 Go |

Conséquence : dans un débogueur pas à pas, les variables des dépendances ne sont plus inspectables. Réactivable ponctuellement : `cargo --config 'profile.dev.debug=true' …` (sans modifier les fichiers).

Dossiers de compilation locaux (outillage isolé, `_outils-scolia`) : `target-ent` (backend), `target-web-leger` (frontend). L'ancien `target-web` (profil complet) n'est plus utilisé ; sa suppression attend une validation.

## 4. Audits des dépendances

Règles : `ent-rust/deny.toml` et `ent-rust/web/deny.toml` (identiques), `ent-rust/.cargo/audit.toml` et `ent-rust/web/.cargo/audit.toml` (identiques). **Rien n'est masqué** : chaque exception a sa raison et une date de revue.

| Thème | Règle | Outil |
|---|---|---|
| Vulnérabilités connues | toujours bloquantes | `cargo deny` (graphe réellement compilé) + `cargo audit` (lockfile complet) |
| Licences | liste permissive explicite ; **pas de copyleft fort** (GPL, AGPL, LGPL — cohérent avec D-005) ; MPL-2.0 seulement en exception ciblée | `cargo deny` |
| Non maintenues | bloquant pour nos dépendances directes ; transitives signalées en avertissement | `cargo deny` (`unmaintained = "workspace"`) + `cargo audit` |
| Versions retirées (yanked) | bloquant | `cargo deny` |
| Doublons de versions | avertissement (souvent imposés par des tiers), suivi ici | `cargo deny` (`multiple-versions = "warn"`) |
| Versions `*` | interdites (sauf chemins) | `cargo deny` |
| Provenance | crates.io uniquement, pas de Git ni de registre inconnu | `cargo deny` |

### Résultats (2026-11-03)

| | Backend | Frontend |
|---|---|---|
| `cargo deny check` | advisories ok, bans ok, licenses ok, sources ok | idem |
| Doublons signalés | 23 (crates Windows `windows-*`, `rand`, `getrandom`, `base64`, `thiserror`, `syn`, `hashbrown`…) | 25 |
| `cargo audit` | 340 crates, 0 vulnérabilité applicable | 437 crates, 0 vulnérabilité ; **1 avertissement** |

Points relevés :
1. **RUSTSEC-2023-0071 (`rsa`, attaque « Marvin », pas de correctif)** — signalé par `cargo audit`. `rsa` n'est présent dans le lockfile que via `sqlx-mysql` (pilote MySQL de sqlx), **jamais activé ni compilé** (l'ENT n'utilise que PostgreSQL ; `cargo tree -i rsa` vide ; aucun artefact compilé ; `cargo deny` ne le voit pas). Exception documentée dans `.cargo/audit.toml`, revue le 2027-02-01.
2. **Licence MPL-2.0** — `webauthn-rs` et 5 crates associées (passkeys, D-009). Copyleft faible au niveau du fichier : utilisation sans modification compatible avec un logiciel non libre ; une modification de ces fichiers devrait être publiée. **Exception ciblée sur ces 6 crates, approuvée le 2026-11-03 (D-016)**, épinglée à la version 0.5.5 : une montée de version fait échouer `cargo deny` (vérifié : version 0.5.4 dans la règle → `licenses FAILED`). MPL-2.0 n'est pas autorisée ailleurs (vérifié avec `cargo deny list -l license` : seules ces 6 crates). `r-efi` affiche aussi LGPL-2.1, mais en choix (`MIT OR Apache-2.0 OR LGPL-2.1-or-later`) : retenue sous MIT.
3. **Licence 0BSD** (`quoted_printable`, via l'envoi d'e-mails) — permissive sans condition : ajoutée à la liste.
4. **`paste` non maintenue (RUSTSEC-2024-0436)** — macro de compilation utilisée par Leptos (`either_of`), aucune vulnérabilité ; avertissement visible, suivi (dépend d'une évolution de Leptos).

## 5. Formatage et Clippy

| Contrôle | Frontend + `ent-types` | Backend existant |
|---|---|---|
| `cargo fmt --check` | **bloquant** ; code formaté | **706 écarts** dans 12 crates (code jamais formaté) → étape non bloquante, signalée ; un commit de formatage dédié est proposé (décision en attente) |
| Clippy | **bloquant sur les avertissements** (`-D warnings`), en rendu serveur et en WebAssembly ; 0 avertissement (1 défaut corrigé : durée de vie inutile) | erreurs bloquantes ; **0 erreur, ≈ 65 avertissements** existants (`ent-db` 31, `domain-eleve` 16 dans les tests, `ent-api` 9, `ent-lab` 5, `ent-mail` 1) signalés, à résorber progressivement |

## 6. Workflow `.github/workflows/ent-rust.yml`

- Déclenchement : `push` et `pull_request` sur **`ENT`**, uniquement si `ent-rust/**` ou le workflow changent ; `workflow_dispatch`. Les workflows Next.js (`main`/`dev`) ne sont pas touchés.
- Permissions en lecture seule ; **aucun secret** ; **aucun déploiement** ; annulation des exécutions obsolètes.
- PostgreSQL 18 **éphémère** (service du runner, `127.0.0.1`, authentification `trust` sur un conteneur jetable, sans mot de passe) ; données de labo fictives ; `ent-db` refuse toute autre base.

| Job | Étapes |
|---|---|
| `formatage` | frontend + `ent-types` (bloquant) ; backend existant (non bloquant, dette connue) |
| `audits` | `cargo deny` (2 workspaces), `cargo audit` (2 workspaces) |
| `backend` | Clippy (workspace ; `ent-types` en `-D warnings`), build, tests (unitaires + intégration sur PostgreSQL) |
| `frontend` | `ent-types` en wasm32, Clippy `-D warnings` (SSR, serveur, wasm), tests unitaires, `cargo leptos build --release --precompress`, tests contre le vrai backend, taille des fichiers |

Cache : `Swatinem/rust-cache` par workspace (registre + `target/`, y compris `target/front` du WebAssembly) ; `CARGO_INCREMENTAL=0` et pas d'info de débogage en CI. Outils épinglés : `cargo-leptos` 0.3.11, `wasm-bindgen-cli` 0.2.130, `wasm-opt` version_123, `cargo-deny` 0.20.2, `cargo-audit` 0.22.2, Rust 1.99.0.

### Validation locale (le workflow n'a pas tourné sur GitHub)
- `actionlint` 1.7.12 : aucune erreur.
- Chaque étape rejouée localement avec les mêmes commandes et `--locked` (résultats dans le bilan de la mission et `_outils-scolia/ci-locale.log`).
- Différences avec GitHub : Windows au lieu de Linux ; base PostgreSQL 18.4 portable locale au lieu du conteneur ; outils installés depuis `_outils-scolia`.

## 7. Commandes locales (Git Bash)

```bash
. ent-rust/web/scripts/env.sh                 # environnement isolé (_outils-scolia)
bash ent-rust/web/scripts/pg.sh start
# Backend
cd ent-rust && CARGO_TARGET_DIR="$SCOLIA_TARGET_ENT" cargo test --locked --workspace && cd ..
# Frontend
cd ent-rust/web
cargo fmt -p ent-web-ui -p ent-web-api -p ent-web-app -p ent-web-server --check   # pas --all : il toucherait le backend
cargo clippy --locked -p ent-web-ui -p ent-web-api -p ent-web-app --all-targets --features ent-web-app/ssr -- -D warnings
cargo test --locked -p ent-web-ui -p ent-web-api -p ent-web-app --features ent-web-app/ssr
cargo leptos build --release --precompress
cargo test --locked -p ent-web-server --test contre_backend -- --test-threads=1
cargo deny check && cargo audit
```
