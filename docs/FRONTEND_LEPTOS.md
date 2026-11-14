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
| `ent-web-app` | application : `noyau/` (session, droits, navigation, coquille, erreurs, temps), `composants_metier/`, `modules/` (connexion, accueil, edt, appel) | un module par domaine, sans dépendre d'un autre module |
| `ent-web-server` | serveur : `ent_api::app(state)` + routes Leptos, même origine | aucune logique métier |

Dans chaque module métier :
- `modele.rs` : **logique d'écran pure**, sans Leptos ni réseau, testée en natif (filtres, sélection, calcul des changements à envoyer) ;
- vues séparées (`mod.rs`, `feuille.rs`…) qui ne font qu'afficher et relier les signaux au modèle.

**Aucune règle métier dans le frontend** : droits, inscriptions, effets d'une validation restent dans `ent-api` / `ent-db`. Le frontend s'adapte aux capacités réelles de l'API (ex. `CAPACITES_API_ACTUELLE`) au lieu de promettre une fonction absente.

## 3. Rendu

- Par défaut : **rendu serveur + hydratation** (structure et états de chargement immédiats).
- Îlots (`islands`) : pour les pages surtout statiques (portails famille / élève, consultation).
- La session est un cookie `HttpOnly` posé par le relais `ent-web-session` (A6, `SESSION_WEB_COOKIE_ETUDE.md` §15) : le WebAssembly n'a **aucun jeton**. Les données métier restent chargées **par le navigateur** (`LocalResource`) ; le serveur rend la coquille et des squelettes. Rendre des données côté serveur (le cookie est désormais lisible par le serveur) est une évolution possible, pas encore faite.
- Le rendu serveur et le navigateur doivent produire **exactement le même HTML initial** : pas de valeur dépendant de l'heure ou du navigateur dans le HTML rendu au premier passage (utiliser `prop:value`, des `Effect`, ou des ressources).

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
- Session : cookie `HttpOnly` + protection CSRF (`X-ENT-CSRF` ajouté par `ApiClient::relais()`). **Interdit** : lire ou écrire un jeton dans `localStorage`/`sessionStorage`/`document.cookie` (un test parcourt les sources et échoue). Toujours `session::client()` ; une 401 pendant l'usage → `reconnexion::signaler_session_perdue()` (fenêtre par-dessus la page), jamais une redirection qui ferait perdre la saisie ; `ErreurChargement` relance le chargement après reconnexion.
- CSP stricte à nonce par requête **en place** sur `/web` et `/pkg` (`ent-web-server`, `politique_csp`) ; ne couvre ni `/app` ni l'API. Voir § 11.

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

## 10. Compléments (2026-11-03)

- `#![recursion_limit = "512"]` est nécessaire **aussi dans `ent-web-server`** : le serveur calcule la disposition des types de vues de toute l'application (erreur « queries overflow the depth limit » apparue avec les composants du design system).
- Dans `view!`, ne pas réutiliser une variable déjà passée à un attribut (`for=id.clone()` puis `id=id` peut échouer selon l'ordre d'évaluation) : créer les copies **avant** la vue (`let pour = id.clone();`).
- CSS : le design system vit dans `ent-web-ui/style/ui.css`, importé par la feuille de l'application ; `style-bundle = true` produit un seul fichier.
- Notifications : `fournir_notifications()` à la racine de `App`, `<ZoneNotifications/>` **à l'intérieur** de `<Router>` (pas directement après les balises `leptos_meta`, voir §8).
- Types de l'API : utiliser `ent-types` (via `ent-web-api`), jamais une copie locale.
- **Ne jamais lancer `cargo fmt --all` dans `ent-rust/web`** : il reformate aussi les crates du backend (dépendances par chemin). Utiliser `cargo fmt -p ent-web-ui -p ent-web-api -p ent-web-app -p ent-web-server`.
- Avant de proposer un changement : formatage (commande ci-dessus), Clippy `-D warnings` (SSR, serveur, wasm), tests — comme la CI (`docs/CI_REPRODUCTIBILITE.md`).

## 11. Conventions issues de la tranche 1 (2026-11-03)

- **Environnement obligatoire** : toute compilation ou test passe par `. ent-rust/web/scripts/env.sh` (chaîne `_outils-scolia` : Rust 1.99 + cible `wasm32`, Strawberry Perl, cargo-leptos). La chaîne Rust système échoue (pas de `wasm32`, Perl MSYS sans module pour `openssl-src`).
- **Ressources** : ne pas lire une `Resource` hors d'un `<Suspense/>` (avertissement d'hydratation). Les données qui dépendent de la session (cookie, invisible du serveur pendant le rendu) vont dans une `LocalResource`. Exception connue : `PageConnexion` lit une `Resource` serveur pour rendre le nom d'établissement côté serveur.
- **CSP et nonce** : la politique interdit style et script en ligne. Pas d'attribut `style=` en ligne (utiliser des classes CSS), pas de `<script>` en ligne sans le nonce fourni par Leptos. Toute nouvelle origine (image, connexion) impose de modifier `politique_csp` et son test.
- **Adresses de retour** : `location.search` ne contient « ? » que s'il y a des paramètres ; reconstruire l'adresse avec le **chemin complet incluant « ? »** et passer par `retour_sur` (refuse redirections ouvertes, `..`, `\`, encodages, boucles).
- **Identifiants** : `Uuid::new_v4` exige la feature `v4`, non activée ; ne pas l'utiliser côté frontend.
- **Tests de rendu** : `pages_rendues` dépend du balisage ; tout changement de structure HTML impose de relire ces assertions.
- **Types sensibles** : implémenter `Debug` à la main pour les structures portant jeton ou mot de passe (masquage) ; borner les messages renvoyés par le serveur avant affichage ; aucun secret de session dans la page.
- **Écritures** : stopper l'envoi sur 401 / 403 / 429 au lieu d'enchaîner les requêtes.
- **Accessibilité** : déplacer le focus et annoncer (zone `aria-live`) à chaque changement de page.
- **Agents** : `explore-haiku` / `review-sonnet` sont restreints à `Read, Grep, Glob` ; les définitions sont lues au démarrage de la session, donc une modification n'agit qu'après redémarrage (à vérifier).

## 12. Conventions issues des écrans du socle 2 (2026-11-05)

- **Écritures** : passer par `noyau/ecriture.rs` (`Ecriture::lancer`, `RetourEcriture`) plutôt que de réécrire spawn_local / erreur / annonce dans chaque écran ; une écriture à la fois, 401 → reconnexion.
- **Droits** : `use_droits_moteur()` (affichage des boutons d'écriture et des entrées de menu) en plus de `use_droits()` (profil). Ne jamais bloquer une page sur l'absence de droit côté client : laisser le serveur répondre 403 (`ErreurChargement` affiche « accès refusé »). Tant que `/roles/mes-droits` n'a pas répondu, les boutons d'écriture sont masqués.
- **`Selection`** prend des options statiques : la construire dans une fermeture réactive une fois les données chargées ; une valeur vide (`""`) signifie « inchangé » ou « aucun ».
- **Listes éditables** (périodes, créneaux) : un `RwSignal` par champ dans une structure `Copy`, `<For key=…>` avec une clé numérique croissante ; envoi après contrôle local par le modèle de vue.
- **Dates du jour** : posées dans un `Effect` après l'hydratation (jamais dans le HTML du serveur) ; champs `date` / `month` / `time` natifs.
- **Aperçu avant écriture** : quand le serveur offre un aperçu (montée), n'appliquer que ce qui a été prévisualisé (toute modification des choix invalide l'aperçu) ; sinon décrire les effets dans la confirmation.
- **Tests contre le backend** : une base PostgreSQL locale de nom `*test*`, `ENT_DATABASE_URL`, `ENT_ENV=lab`, `ENT_ALLOW_LABO=1`, `LEPTOS_OUTPUT_NAME=ent-web`, `LEPTOS_SITE_PKG_DIR=pkg` ; les écritures du test ne doivent pas modifier l'année courante (utiliser une année de test lointaine, idempotente).

