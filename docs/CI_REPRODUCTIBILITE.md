# Reproductibilité et CI de l'ENT Rust

Date : 2026-11-03 — Statut : **en place** (D-015) : lockfiles versionnés, profil allégé, audits, workflow CI.
Premier passage sur GitHub le 2026-11-03 (run 41599498046) : formatage, audits et backend **verts** ; frontend en échec (variable Leptos manquante, F-003), corrigé localement, **correction non poussée**.

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
5. **`chrono` non maintenu (RUSTSEC-2026-0543, publiée le 2026-11-17)** — ce n'est pas une faille. Elle bloquait `cargo deny` parce que `chrono` est une dépendance directe (`unmaintained = "workspace"`). Exception dans `ent-rust/deny.toml` et `ent-rust/web/deny.toml` (`ignore`, même texte ; le web a son propre fichier et son propre `Cargo.lock`), **revue le 2026-11-17** : `chrono` est utilisé dans environ 540 fichiers et par sqlx ; un remplacement par `jiff` ou `time` est à étudier après la bascule de février (décision du pilote, à valider par Florian).

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
| `frontend` | **synthèse** (nom et condition conservés ; règle 27) de cinq jobs parallèles, voir §8 |

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

## 8. Frontend en jobs parallèles (lot `ent-ci-frontend-rapide`, 2026-11-18)

**Résultat : 78 min 29 → 28 min 51 de bout en bout** (run 396 sur ENT → run 399 sur la PR 99, même contenu testé), sur runners `ubuntu-latest` gratuits, sans retirer ni affaiblir une vérification.

### Découpage

| Job | Contenu | Durée (run 399) |
|---|---|---|
| `frontend-lint` | `ent-types` wasm32, Clippy `-D warnings` (4 commandes, inchangées), tests unitaires | 11 min 02 |
| `frontend-build-wasm` | `cargo leptos build --release --precompress --frontend-only` ; contrôle de taille (`ls -l target/site/pkg`) + présence des `.wasm.br` / `.wasm.gz` | 8 min 35 |
| `frontend-build-serveur` | `cargo leptos build --release --server-only` | 21 min 50 |
| `frontend-archive` | une seule compilation des binaires de test (`-p ent-web-server -p ent-web-session --features ent-web-server/labo,ent-api/labo`), archive nextest, inventaire, tests de documentation | 20 min 58 |
| `frontend-tests` | 6 tranches `count:i/6`, **un fil et un PostgreSQL éphémère par tranche** | 3 à 7 min chacune (dont 1 min 30 à 4 min 40 de téléchargement de l'archive) |
| `frontend` | synthèse : vert seulement si les cinq autres le sont ; `if: always()` et `portee` | 3 s |

Les jobs de synthèse (`backend`, `veille-pilote`, `veille-pilote-pr`) n'ont pas changé : ils lisent toujours `frontend`.

### Avant / après (par étape)

| Étape | Run 396 (ENT, un seul job) | Run 399 (jobs parallèles) |
|---|---|---|
| Clippy | 6 min 06 | 7 min 38 (en parallèle) |
| Tests unitaires | 2 min 59 | 2 min 40 |
| Build release (front puis serveur) | 28 min 15 | wasm 8 min 13 ‖ serveur 21 min 23 |
| En-têtes de sécurité | 9 min 22 | inclus dans les tranches |
| Tests contre le vrai backend | 20 min 20 | tranches 1 min 32 à 3 min 37 (exécution) |
| Session par cookie | 9 min 55 | inclus dans les tranches |
| **Total du job / bout en bout** | **78 min 29** | **28 min 51** |

### Ce que la mesure a appris
1. **Le build release de 28 min était séquentiel par construction** : avec `--release --precompress`, cargo-leptos 0.3.11 interdit la parallélisation (`can_parallelize` faux dans `command/build.rs`) et compile le WebAssembly puis le serveur à la suite. Séparer les deux (`--frontend-only` / `--server-only`) les exécute en parallèle sur deux machines. Mêmes commandes de compilation, mêmes profils.
2. **Les tests contre le backend étaient dominés par la recompilation** : trois étapes recompilaient `ent-api` / `ent-web-server` avec des jeux de features différents (avec / sans `labo`). Une seule compilation avec `labo` pour tout, puis exécution en tranches : l'exécution pure ne prend que 1 à 4 min par tranche.
3. **`--test-threads=1`** : les tests partagent une base et ses données de labo. Il est conservé dans chaque tranche ; la parallélisation vient des tranches, chacune avec sa propre base, comme `tests-labo`. Aucune isolation interne n'a été changée.

### Limites et points à surveiller
- **Chemin critique** : `frontend-build-serveur` (21 min 50) et `frontend-archive` (20 min 58). Le temps total est donc fixé par la compilation de `ent-api`, pas par les tests. Sous 35 min avec de la marge, pas davantage visé ici.
- Les entêtes de sécurité (`entetes_securite`) et `droits_parite` tournent maintenant avec la feature `labo` compilée (avant : sans). Ils n'utilisent que `routeur_web` ; ils sont verts.
- Run 399 mesuré avec les caches `rust-cache` des nouveaux jobs **froids** (clés nouvelles) : les runs suivants devraient être plus courts ; à confirmer.
- Le téléchargement de l'archive coûte 1 à 5 min par tranche (variable) ; 6 tranches restent un bon compromis.
- Pistes non engagées (non nécessaires pour l'objectif) : profil CI plus léger pour l'archive de tests, cache de `wasm-opt`.

### Échec d'essai (consigné)
Premier essai (run 398) : les options `--lib-only` / `--bin-only` n'existent pas dans cargo-leptos 0.3.11 (noms réels : `--frontend-only` / `--server-only`, vérifiés dans les sources de la version épinglée). Corrigé dans le commit suivant ; aucun retour en arrière.
