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

## 13. Conventions issues de « Mon agenda » (socle 19, 2026-11-11)

- **Le serveur donne le jour et l'heure de l'établissement** (`ElementAgenda.jour`, `heure_debut`…, `EvenementDetail.jour`…) : l'écran n'a aucun fuseau à deviner et ne convertit pas d'instant UTC (le fuseau d'un établissement peut ne pas être Paris). Les écrans plus anciens qui utilisent `a_paris` gardent cette limite.
- **Une source absente est dite** : la vue renvoie `sources_indisponibles` et l'écran l'affiche (`Alerte` d'information), jamais un silence.
- **Publier à une audience** : aperçu (rien n'est écrit), puis confirmation avec l'**empreinte** de l'aperçu ; toute modification des choix invalide l'aperçu (même règle que la montée de classes).
- **Téléchargement iCal** : simple lien `GET` (cookie de session, aucun jeton dans la page).
- Une route `/web/agenda` existait déjà (« Travail à faire », cahier de textes) : « Mon agenda » est sur `/web/mon-agenda`.


## 14. Conventions issues de l'assiduité (socle 12, AS14, 2026-11-16)

- **Types** : l'assiduité vit dans `ent-types::assiduite` (JSON identique à `ent-db::assiduite_as2/as3/as5`, états en chaînes) et `ent-web-api::assiduite` ; aucun type d'`ent-db` n'est utilisé côté web.
- **Appel v2** : lecture pré-remplie par le serveur (`GET /appels/{id}/v2`) ; le professeur n'a pas de « brouillon serveur » : sa saisie est locale, et à « Appel fait » **seules les lignes modifiées** partent (`pointer_ligne`) avant la validation (`valider_appel_v2`). Un échec partiel n'enchaîne pas la validation. Une ligne `localise` / `attendu_ailleurs` est posée par le système : `choix = None` la laisse inchangée.
- **Libellés lisibles sans la couleur** : chaque état d'un élève a un libellé complet (`phrase_ligne`, lue par le lecteur d'écran : « Léa Martin, absente, prévenu ») ; la couleur ne fait que doubler.
- **Corrections** : version lue envoyée (`version_base`), conflit = message et rechargement ; « qui corrige, jusqu'à quand » est décidé par le serveur (intervenant le jour même, vie scolaire ensuite avec motif) : l'écran propose « Corriger » et traduit le refus.
- **Qualification** : jamais posée d'office ; version lue envoyée ; en masse, **aperçu puis confirmation avec l'empreinte** (même règle que la montée de classes), toute modification des choix invalide l'aperçu.
- **Lecture unitaire** (accueil, AS-216) : un élève choisi à la fois, jamais la liste du jour ; la page dit que la consultation est enregistrée.
- **Vue famille / élève** : libellé à liste fermée du serveur (`EpisodeVueFamilleDto.libelle`) ; jamais de note interne ; « Donner le motif » seulement si le lien permet de justifier, le serveur revérifie.
- **Tableaux étroits** : sous 640 px la file « à qualifier » (`asd-tableau`) passe en cartes empilées ; l'en-tête reste lisible par lecteur d'écran.
- **Mots réservés par `view!`** : une constante passée en attribut (`options=MAJUSCULES.iter()…`) est lue comme un composant (« syntaxe `X:` non supportée ») : calculer la valeur dans une variable avant la vue.
- **Un `Memo` plutôt qu'une fermeture** pour une valeur lue par plusieurs éléments de la même ligne : une fermeture `move` n'est pas `Copy`.

## 15. Conventions issues des écrans des absences des personnels (socle 15, A11a à A11c, 2026-11-16)

- **Un module, un client, une feuille de style** : `modules/absences_personnels/` (modèle pur + vues), `ent-web-api/src/absences_personnels.rs` (méthodes préfixées `absp_`, pour ne pas heurter les absences d'élèves), `style/absences-personnels.css` importée par une seule ligne d'`@import` dans `ent-web.css` (moins de conflits quand plusieurs écrans s'écrivent en parallèle).
- **Pas de `if … else` en attribut de `view!`** : `ton=if x { A } else { B }` n'est pas analysé ; calculer la valeur avant la vue (`let ton = …;`).
- **Fermeture réactive et `Vec` capturé** : une fermeture `move || …` qui passe un `Vec` *par valeur* à un composant devient `FnOnce` (« expected a closure that implements FnMut ») ; ranger le `Vec` dans un `StoredValue` et appeler `get_value()` dans la fermeture.
- **`lien_actif` fonctionne par préfixe** : deux entrées de menu dont l'une est le préfixe de l'autre s'allument ensemble. Garder des chemins disjoints (`/web/personnel/absences` et `/web/personnel/instruction-absences`, pas `…/absences/files`).
- **Aucune décision sur sa propre absence** : « est-ce la mienne ? » se décide par la liste de **mes** absences (portée `moi`), jamais par un nom ; tant qu'elle n'a pas répondu, aucune écriture n'est proposée. Le serveur refuse de toute façon (R-AP10).
- **Un titre de `Dialogue` est statique** : une boîte par action (`DialogueDecision` paramétrée par `spec_decision`, testée en natif), et un bouton « Retour » (le mot « Annuler » est réservé à l'action métier « Annuler l'absence »).
- **Lots et plans** : une écriture après l'autre, **arrêt sur 401, 403 ou 429** (`arret_sur_erreur`, testé), échecs listés avec le nom de la ligne, rechargement ensuite ; seules les lignes **cochées et affichées** partent, les sélections masquées par le filtre sont comptées à voix haute. Un lot de validation ne contient jamais de nature sensible (liste blanche `NATURES_LOT`).
- **Dates** : les instants d'absence arrivent en UTC (`DateTime<Utc>`, fin exclusive) : `jours_absence` / `libelle_periode` les affichent en heure de Paris ; une limite assumée tant que le DTO ne donne pas le jour de l'établissement (§13).
- **Minimisation à l'écran** : un écran « organisation » (ce matin) n'affiche que des DTO sans nature ; un test de structure parcourt le JSON du tableau. Aucun nom de fichier de pièce n'est affiché ; les consultations de pièces sont sans nom (le journal n'en donne pas).
- **Parcours navigateur** : voir `docs/socles/15_ABSENCES_A11_ABC.md` §5 (jeu de données par l'API de labo, Chromium + Playwright, axe-core 4.10, 375 px). Compilation locale : `CARGO_PROFILE_DEV_DEBUG=0` et un `CARGO_TARGET_DIR` dédié (le binaire de test avec `ent-api` dépasse 4 Go d'informations de débogage : erreur de liaison « relocation R_X86_64_32 out of range »).
- **Jour du navigateur sous la garde de session** : un contenu placé sous `GardePersonnel` n'est jamais rendu côté serveur (la session est chargée par le navigateur) ; il peut lire `aujourdhui()` à sa création. Un `Effect` qui pose le jour **après** le premier chargement d'une `LocalResource` n'a pas relancé la requête (constaté : « Date invalide » affiché jusqu'à un clic sur « Actualiser ») : lire le jour avant de créer la ressource.

## 16. Conventions issues du tableau de bord de la vie scolaire en panneaux (socle 12, AS14b-bis, 2026-11-16)

- **Un composant partagé par l'accueil, pas un module** : le tableau est monté sur l'accueil (`modules/accueil`) ; comme « un module ne dépend jamais d'un autre module », il vit dans `composants_metier/tableau_vs/` (`modele.rs` pur + `mod.rs` vues). Le serveur revérifie chaque droit, l'écran ne montre un bouton que si le moteur de droits (`use_droits_moteur`) le donne.
- **Regroupement et disposition = logique pure testée en natif** (`regrouper`, `moment`, `disposition`, `appliquer`) ; **l'heure vient du serveur** (`heure_locale` de la file, heure de l'établissement). `?heure=HH:MM` dans l'adresse simule l'heure **pour la disposition seulement** (essais, formation, parcours navigateur) : jamais une donnée ni un droit.
- **Une disposition épinglée est utilisable ou ignorée en bloc** (cinq panneaux, chacun une fois, comme le serveur) : un doute retombe sur l'automatique plutôt que de cacher des panneaux. Le bandeau « Urgent » n'est pas un panneau : ni rang, ni masquage, ni repli.
- **Rafraîchissement sans perte du clavier** : l'actualisation automatique (60 s) **saute son tour quand le focus est dans le tableau** ; les données gardées d'un chargement au suivant (`Donnees`) ne sont jamais effacées par un échec (message d'avertissement, pas d'écran vide).
- **Replier en mode automatique est local** (le temps de la page), **épinglé en mode choisi** : l'écran le dit dans « Disposition ». Les lignes d'une `<For>` ont pour clé **toute la ligne** (`format!("{e:?}")`) : un changement visible la redessine.
- **Pas de turbofish dans un attribut de `view!`** : `each=move || x.collect::<Vec<_>>()` casse l'analyse (« expected identifier, found keyword `move` ») ; calculer la fermeture avant la vue.
- **Écritures par `Ecriture::lancer`** (une à la fois, annonce `aria-live`, rechargement ensuite) ; une boîte de dialogue par action (titre statique), choix dans une **liste fermée** copiée du serveur et testée contre les constantes d'`ent-db` (`listes_fermees_alignees_sur_le_serveur`).
## 17. Conventions issues des écrans heures, RCD et réglages des absences (socle 15, A11d à A11f, 2026-11-16)

- **Mêmes règles que §15**, dans le même module (`modules/absences_personnels/`) : `heures.rs` (« Mes heures »), `etats_heures.rs` + `paie.rs` (heures du mois, état mensuel, paie de l'OGEC, DSN), `rcd.rs` (plan, volontaires, remontée), `parametrage.rs` (seuil, EDT de référence, circuit) ; leurs modèles purs `modele_heures.rs`, `modele_rcd.rs`, `modele_parametrage.rs` ; client `ent-web-api/src/absences_personnels_hrcd.rs` (méthodes `absp_*`). Aucun fichier commun touché hors une ligne de `mod`, trois `<Route>` supplémentaires et les entrées de menu.
- **Sections par `Segmente` + `match`** : chaque section a **sa propre `LocalResource`** (une section masquée ne charge rien) ; les sections réservées (`vivier`, `remontee`, `paie`, `dsn`) ne sont **pas proposées** sans le droit du moteur, et le `match` refuse aussi la valeur (jamais de section « cachée par le style »). Le serveur reste juge : un 403 est affiché par `ErreurChargement`.
- **Menu** : seules les entrées de **gestion** ont un lien de menu, selon le moteur (`heures.*` ou `paie.absences` → « Heures et états » ; `rcd.gerer` ou `rcd.remontee` → « Plan de remplacement » ; `absences.parametrer` → « Réglages des absences »). « Mes heures » et le volontariat RCD (ouverts à tout le personnel) s'atteignent par deux liens de « Mes remplacements » : cela évite de changer les listes attendues du menu de chaque profil (conflits avec les autres sessions). Chemins disjoints (§15 : `lien_actif` par préfixe).
- **Pas d'identifiant à l'écran** : un agent s'affiche par son nom (`charger_noms`, annuaire borné à 200), sinon « Agent non listé » (`Noms`, testé).
- **R-AP28 à l'écran** : la saisie d'une heure sans séance ne propose que `NATURES_SAISIE` (jamais le remplacement de courte durée ni le rattrapage dû) ; l'attestation ne porte que sur une séance que **le serveur** liste (« à attester ») ; une heure figée ou exportée ne se corrige pas, elle se **régularise** (`peut_corriger` / `peut_regulariser`, testés). Une durée se saisit « 1 h 30 », « 45 min », « 1:30 » (`analyser_duree`, 1 minute à 24 heures comme le serveur).
- **Un titre de `Dialogue` est statique** : correction et régularisation sont **deux instances** de `DialogueDuree` (paramètre `regul`), pas une boîte au titre changeant.
- **Export CSV** : `lancer_export` (commun.rs) appelle `absp_csv` (octets, trace d'accès écrite par le serveur) puis `enregistrer_telechargement` ; une erreur s'affiche, jamais en silence. Les exports ne contiennent jamais la nature d'une absence (testé côté serveur).
- **Pièges de compilation** : `Uuid::new_v4()` n'existe pas dans le navigateur (la fonctionnalité `v4` n'est pas compilée) : les tests du modèle utilisent un compteur (`identifiant()`) ; un booléen lu **dans** une fermeture `then(|| view! …)` après `h.etat` déplacé donne « borrow of moved value » : calculer `let corrigeable = …;` avant la vue ; clippy 1.99 exige `then_some` pour une valeur sans calcul (`regul.then_some("…")`) et `sort_by_key` pour un tri par clé.
- **Parcours navigateur** : `docs/socles/15_ABSENCES_A11_DEF_captures/parcours.mjs` (même méthode que §15 ; `AXE_PATH` désigne `axe.min.js`).

## 18. Conventions issues du dossier d'assiduité et des signalements (socle 12, AS14d et AS14e, 2026-11-16)

- **Un dossier = un élève, une année** : `/web/assiduite/eleves/:id` (cinq sections : résumé, absences, décompte, contacts et courriers, chronologie). L'année scolaire est calculée par l'écran (1er septembre au 31 août) : hypothèse signalée, l'année propre à l'établissement est au socle 5.
- **Aucune règle recalculée** : « compte », « non justifiée », l'explication de chaque demi-journée viennent du serveur (`DemiJourneeDto.raison` / `explication`, champs ajoutés avec `#[serde(default)]`). L'écran ne fait que regrouper par mois et situer par rapport au seuil (`EtatSeuil`). Seuil illisible : aucun « atteint » n'est affiché et l'écran le dit.
- **Une section refusée est dite** : 403 sur les contacts ou les rectifications → message d'information (« non accessibles »), jamais un silence ni une page d'erreur ; le serveur reste juge (motif médical, justificatifs, établissement).
- **Requalifier** : motif codé obligatoire (`MOTIFS_REQUALIFICATION`), version lue envoyée, conflit = message. Un mois arrêté n'est pas réécrit : la différence est une rectification (liste, « marquer comme vue »).
- **Décider un signalement** : proposer = `assiduite.signaler`, décider = `discipline.gerer` (le bouton suit le second, pas le premier). « Noter comme transmis » ne transmet rien : ScolIA n'envoie rien à l'académie (R-AS26). Ce qui manque (AS9 : passkey, export, trace de transmission, profils) est listé à l'écran (`MANQUE_AS9`), jamais simulé.
- **Filtrage côté écran des rectifications** : la route renvoie celles du mois pour tout l'établissement (droit `assiduite.signaler`) ; l'écran du dossier ne garde que l'élève. À remplacer par un filtre serveur si le volume l'exige.
- **Un module ne dépend pas d'un autre** : les signalements ont quitté `discipline_vs` pour `assiduite_vs::signalements` ; le modèle pur est dans `assiduite_vs::dossier_modele`.
- **`ton=if … {}` dans `view!`** est refusé par la macro : calculer la valeur avant la vue. Un `LocalResource` n'est pas `Send` : les passer en paramètre de composant est possible (`Copy`), les emprunter dans `view!` non.

## 19. Conventions issues de la liste et de la fiche hub des élèves (socle 2, E9a et E9b, 2026-11-17)

- **Un module, un client, une feuille de style** (comme §15) : `modules/eleves/` (`modele.rs` pur + `liste.rs` + `fiche.rs`), `ent-web-api/src/eleves.rs` (méthodes `eleves_*` / `eleve_*`), `style/eleves.css` (une ligne d'`@import`). Types : `ent-types::eleve_synthese` (`EleveListeDto`, `EleveLigneDto`, `EleveFicheDto`, `SyntheseDto`…), jamais de copie locale ; un test d'`ent-api` garde l'identité du JSON de `EleveDto` (serveur) et d'`EleveFicheDto` (client).
- **Le serveur décide de tout, l'écran explique** : portée de la liste (`portee: etablissement|perimetre`, dite à l'écran), masquage par groupe (`masques` avec message), tuiles masquées avec leur **raison** (`raison_masquage`, jamais le contenu). Un élève absent, d'un autre établissement ou protégé donne **le même écran** « Fiche introuvable » (jamais « refusé » : la différence serait une information).
- **Aucun lien mort** : les liens du contrat d'E8 (`/web/eleves/{id}/scolarite`…) pointent des écrans futurs ; `destination_tuile` ne reprend que ce qui existe (onglet de la fiche, `/web/infirmerie/eleves/{id}`), et un test parcourt `CONTRATS_TUILES` pour qu'une tuile ajoutée à E8 force une décision ici. Une tuile visible sans écran dit ce qu'elle contiendra (`corps_tuile_sans_ecran`).
- **Route partagée selon le profil** : `/web/eleves/:id` est la fiche hub du **personnel** ; les familles et les élèves gardent le dossier (`/web/eleves/:id/dossier`). L'aiguillage est dans `lib.rs` (`PageEleve`), jamais dans un module (un module ne dépend pas d'un autre).
- **Recherche instantanée sans inonder le serveur** : pause de frappe (250 ms) puis une requête ; **une seule lettre ne cherche rien** (liste complète, `recherche_a_envoyer`) ; un filtre qui change ramène à la première page ; le résultat précédent reste affiché (`aria-busy`, opacité) pendant le rechargement au lieu d'un squelette qui clignote ; les listes de classes se **cumulent** (le sélecteur ne rétrécit pas quand un filtre est posé).
- **Raccourci « / »** : déclaré (texte d'aide visible), inactif pendant une saisie ou avec Ctrl, ⌘, Alt (`ouvre_la_recherche`, testé) ; il met le focus par l'identifiant de l'élément (navigateur seulement).
- **Pas de photo dans la liste** (initiales seulement) : aucune requête d'image par ligne ; la photo n'est chargée que sur la fiche, par `<img src="/eleves/{id}/photo">` (même origine, cookie de session) avec **repli sur les initiales** en cas d'erreur (404 : absente, masquée par la protection, ou refusée).
- **Onglets** : `Segmente` (boutons radio natifs, flèches) synchronisé avec `?onglet=` par `use_query_map` / `use_navigate` ; un code inconnu retombe sur la synthèse (`Onglet::depuis_code`). Chaque section a **sa propre `LocalResource`** (§17) : une section fermée ne charge rien.
- **Historique** : la frise compare chaque version à la **précédente dans le temps** (`etapes_frise`) ; une correction ne se confond pas avec un changement d'état civil (libellé et ton) ; la date de naissance n'est jamais inventée (`date_naissance_affichee` : précision et statut dits). Une section refusée (403) est **dite** (« réservé au secrétariat et à la direction »), jamais une page d'erreur ni un silence.
- **Parcours navigateur** : `docs/socles/02_ELEVE_E9_captures/parcours.mjs` (jeu de données par l'API de labo, Chromium + Playwright, axe-core 4.10, 375 px) — voir le rapport de livraison.
