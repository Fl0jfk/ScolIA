# Frontend Leptos — conventions et pièges

Référence obligatoire avant tout développement du frontend Leptos de l'ENT (D-008, D-010).
Base : prototype du module d'appel (`ent-rust/web/`, `docs/LEPTOS_CALL_PROTOTYPE.md`).

## 1. Versions

Versions **épinglées** avec `=` dans `ent-rust/web/Cargo.toml` (voir D-010) : `leptos` 0.8.23, `leptos_router` 0.8.17, `leptos_meta` 0.8.7, `leptos_axum` 0.8.10, `cargo-leptos` 0.3.11, `wasm-bindgen` 0.2.130, `wasm-opt` version_123, Rust 1.99.0.

- Écrire du code **Leptos 0.8** : `use leptos::prelude::*`, `signal()` / `RwSignal`, `Memo`, `Effect::new`, `LocalResource`, `Resource`, `#[component]`, `view!`, `leptos_router::components::{Router, Routes, Route}` + `path!`. Pas d'API 0.5/0.6 (`create_signal`, `cx: Scope`, `create_resource`…).
- Tout changement de version = mise à jour simultanée des 4 crates Leptos, de `cargo-leptos`, de `wasm-bindgen` (crate **et** CLI), puis tests complets. Passage en 0.9 : décision séparée.

## 2. Structure

| Crate (`ent-rust/web/crates/`) | Rôle | Règle |
|---|---|---|
| `ent-web-ui` | design system : composants génériques accessibles | aucun appel réseau, aucun métier |
| `ent-web-api` | client typé des routes JSON d'`ent-api` | aucun composant ; erreurs typées `ApiError` |
| `ent-web-app` | application : coquille, routes, modules métier | un module par domaine (`appel/`, …) |
| `ent-web-server` | serveur : `ent_api::app(state)` + routes Leptos, même origine | aucune logique métier |

Dans chaque module métier :
- `modele.rs` : **logique d'écran pure**, sans Leptos ni réseau, testée en natif (filtres, sélection, calcul des changements à envoyer) ;
- vues séparées (`mod.rs`, `feuille.rs`…) qui ne font qu'afficher et relier les signaux au modèle.

**Aucune règle métier dans le frontend** : droits, inscriptions, effets d'une validation restent dans `ent-api` / `ent-db`. Le frontend s'adapte aux capacités réelles de l'API (ex. `CAPACITES_API_ACTUELLE`) au lieu de promettre une fonction absente.

## 3. Rendu

- Par défaut : **rendu serveur + hydratation** (structure et états de chargement immédiats).
- Îlots (`islands`) : pour les pages surtout statiques (portails famille / élève, consultation).
- Tant que la session est un jeton Bearer en `localStorage`, les données métier sont chargées **par le navigateur** (`LocalResource`) ; le serveur rend la coquille et des squelettes. Après passage au cookie `HttpOnly` (étude `SESSION_WEB_COOKIE_ETUDE.md`), le serveur pourra rendre les données.
- Le rendu serveur et le navigateur doivent produire **exactement le même HTML initial** : pas de valeur dépendant de l'heure, du navigateur ou de `localStorage` dans le HTML rendu au premier passage (utiliser `prop:value`, des `Effect`, ou des ressources).

## 4. Réactivité et performances

- Un signal par élément modifiable d'une ligne (statut, sélection…), pas un seul `Vec` réactif : un changement ne redessine que sa ligne.
- Listes : `<For each=… key=…>` avec une clé stable.
- Données fixes dans `StoredValue`, valeurs dérivées dans `Memo`.
- Au-delà de quelques centaines de lignes sur poste modeste : pagination ou virtualisation (mesuré : 400 lignes ralenti ×4 → 0,4–0,6 s pour recréer la liste).

## 5. Accès à l'API

- Toujours via `ent-web-api` (jamais de `fetch` écrit à la main).
- Même origine : chemins relatifs, pas de CORS.
- Erreurs : `ApiError::message_utilisateur()` pour l'affichage ; `NonConnecte` → effacer la session et proposer la reconnexion.
- Écritures groupées : envoyer **seulement les changements** ; en cas d'échec partiel, ne pas enchaîner l'étape suivante (ex. ne pas clore l'appel) et lister les échecs.

## 6. Accessibilité (DESIGN_SYSTEM.md)

- Éléments natifs d'abord : `<button>`, `<input type=radio|checkbox>`, `<fieldset>/<legend>`, `<table>` + `<th scope>`, `<dialog>` (focus, Échap et retour de focus gérés par le navigateur).
- Chaque champ a un `<label for>` ou un `aria-label`; `lang="fr"`; lien d'évitement **dans la page**; focus visible; zone `aria-live` pour les changements (`Annonce`).
- Actions groupées : ne s'appliquent **qu'aux lignes sélectionnées et affichées** ; afficher le nombre de sélections masquées.
- Raccourcis clavier : déclarés (`aria-keyshortcuts`) et inactifs pendant la saisie de texte.
- Un audit automatique (axe-core) ne suffit pas : test avec lecteur d'écran avant de déclarer un écran accessible.

## 6 bis. Sécurité

- Jamais de `inner_html` (ni d'insertion HTML brute) avec des données venant d'un utilisateur ou de l'API : laisser Leptos échapper le texte.
- Aucun secret dans le WebAssembly ou le HTML : tout ce qui est envoyé au navigateur est public.
- Session : aujourd'hui jeton Bearer en `localStorage` (transitoire) ; cible cookie `HttpOnly` + protection CSRF (`docs/SESSION_WEB_COOKIE_ETUDE.md`). Ne pas multiplier les lectures de `localStorage` : passer par `session.rs`.
- CSP stricte visée : prévoir les nonces Leptos pour le script d'hydratation.

## 7. Compilation et environnement

- Outils et sorties dans `_outils-scolia` via `. ent-rust/web/scripts/env.sh` (aucune installation globale).
- `cargo leptos build` (debug), `cargo leptos build --release --precompress` (production : fichiers `.br`/`.gz` servis par `ServeDir::precompressed_*`).
- Les tests (`cargo test`) et `cargo-leptos` doivent compiler **avec les mêmes options** : garder `disable-erase-components = true` dans la configuration `[[workspace.metadata.leptos]]`.
- `wasm-bindgen` et `wasm-opt` doivent être dans le PATH (`_outils-scolia/bin`, `_outils-scolia/binaryen/bin`), sinon `cargo-leptos` les télécharge dans `AppData`.

## 8. Pièges rencontrés (à ne pas reproduire)

| Symptôme | Cause | Règle |
|---|---|---|
| Hydratation figée : « expected a marker node, but found … » sur `leptos_meta/stylesheet.rs` | élément HTML placé à la racine de `App` juste après `<Stylesheet>` / `<Title>` | à la racine de `App` : seulement les balises `leptos_meta` et `<Router>` ; tout le reste dans les pages |
| Même erreur d'hydratation après `cargo test` | `cargo-leptos` ajoute `--cfg erase_components` en debug, `cargo test` non : le serveur recompilé ne produit plus le même HTML | `disable-erase-components = true` |
| `queries overflow the depth limit!` (WebAssembly) | types de vues très imbriqués sans `erase_components` | `#![recursion_limit = "512"]` dans `ent-web-app` ; découper les gros composants (`.into_any()`) |
| `the name X is defined multiple times` sur une fonction serveur | `#[server] fn config_x` génère un type `ConfigX` | ne jamais nommer une structure comme la fonction serveur en PascalCase |
| `Several bin targets found` | crate serveur avec plusieurs binaires | `bin-target = "ent-web-server"` |
| WebAssembly de 1 Mo envoyé non compressé | `ServeDir` simple | `--precompress` + `precompressed_br().precompressed_gzip()` |
| Mesures incohérentes dans le navigateur de l'application | panneau masqué : animations et minuteries suspendues | mesurer dans Chrome sans fenêtre (profil séparé) |
| Réponses décompressées dans les navigateurs de ce poste (`X-Content-Encoding-Over-Network`) | intermédiaire du poste (logiciel de sécurité probable) | vérifier la compression avec `curl`, pas seulement dans le navigateur |

## 9. Tests attendus pour chaque écran

1. Modèle d'écran : tests unitaires natifs (dont un test par anomalie corrigée).
2. Contre le vrai backend (`ent-web-server/tests/`, base locale de labo) : rendu serveur, parcours principal, erreurs API, **permissions** (au moins un rôle refusé).
3. Navigateur : clavier, mobile 375 px, axe-core, mesures (affichage, clic, recherche) à processeur normal et ralenti ×4.
