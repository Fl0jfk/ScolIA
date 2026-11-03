# Registre des décisions

## Vue d'ensemble

| Sujet | Orientation | Statut | Réf. |
|---|---|---|---|
| Branche de travail Rust | `ENT` (local + `origin/ENT`) | **Validé** | D-001 |
| Backend | Rust (axum, sqlx — existant dans `ent-rust/`) | **Validé** | D-002 |
| Frontend | Rust, comme le backend ; l'UI JS actuelle (`ent-api/src/ui.rs`) sera remplacée | **Validé** | D-002 |
| Framework frontend | **Leptos 0.8** (0.8.23), rendu serveur + hydratation / îlots, intégré à axum ; validé techniquement par le prototype d'appel ; pas de migration générale immédiate | **Validé** (socle confirmé) | D-008, D-010 |
| Types partagés front/back | Crate `ent-types` sans dépendance serveur ; lot 1 créé (appel, EDT, session), utilisé par le frontend ; **`ent-api` migré pour l'appel et l'EDT** (5 DTO), DTO d'authentification pas encore | Lot 1 appel/EDT migré (D-016) | D-015, D-016, `docs/ENT_TYPES_PROPOSITION.md` §8 |
| Reproductibilité et CI | `Cargo.lock` versionnés, profil de développement allégé, workflow `ent-rust.yml`, `cargo fmt` / Clippy / `cargo deny` / `cargo audit` | **Validé** (poussé sur `ENT` le 2026-11-03) ; MPL-2.0 approuvée (D-016) | D-015, D-016 |
| Session web | Cookie `HttpOnly` ; 2 h d'inactivité, 12 h maximum, **à condition** d'un parcours de reconnexion rapide (passkey en un geste, reconnexion sans quitter la page) | **Validé** (principe) | D-011 |
| Session mobile | Longue (plusieurs mois, prolongée à l'usage), déverrouillage biométrique, révocable à distance | **Validé** (principe) | D-011 |
| Adresse par établissement | Un sous-domaine par établissement (tenant), ex. `lpnb.scolia.fr` | **Validé** | D-012 |
| Identité | Une identité par personne au niveau de l'ENT, rattachée à plusieurs établissements ; passkey liée à `scolia.fr` ; aucune visibilité entre établissements | **Validé** (DPO à consulter) | D-013 |
| Comptes familles | Comptes parents uniquement ; préinscription pré-remplie à l'initiative du parent ; IBAN reproposé masqué à la validation d'inscription, nouveau mandat | **Validé** (DPO à consulter) | D-014 |
| DB | PostgreSQL | Cible | — |
| Hébergement/objet | Scaleway prioritaire | À valider | — |
| SMS | OVH | **Validé** | D-004 |
| Email | Existant Next : envoi Scaleway TEM, réception IMAP OVH (`docs/scaleway-email-polling.md`) ; Rust : `ent-mail` SMTP, sans fournisseur choisi | Ouvert | — |
| Auth | Conserver l'auth Rust maison (`ent-auth`, `ent-oidc` : Argon2id, sessions Postgres, passkeys via `webauthn-rs`, SSO OIDC) ; cookie de session sécurisé pour le web, jeton pour les applis mobiles | **Validé** | D-008, D-009 |
| Double authentification | Passkey obligatoire pour tous les comptes, à terme | **Validé** (exigence) | D-009 |
| Mobile | Une seule appli iOS + une Android pour tous les rôles, version « quotidienne » allégée | **Validé** | D-003 |
| IA | Mistral prioritaire à étudier | Sous garanties | — |
| Conversion de fichiers | Moteur maison Rust, briques libres non-AGPL, inspiré de convt | **Validé** | D-005 |
| Bureautique | Suite ArtCraft (Wordcraft, GridCraft, DeckCraft) intégrée en WebAssembly | **Validé** | D-006 |
| WASM | Bureautique ArtCraft (iframe) et îlots interactifs Leptos | **Validé** | D-006, D-008 |
| Calendrier | Sortie de l'ENT visée dans ~1 an (indicatif) | **Validé** (objectif) | D-007 |
| Next.js production | Intouchable | Règle absolue | — |
| DB production | Aucun accès même lecture | Règle absolue | — |

Pour chaque nouvelle décision : ID, date, sujet, contexte, options, décision, raisons, impacts, risques, validateur, fichiers concernés. Ne pas confondre hypothèse et décision validée.

---

## D-001 — Branche de travail Rust : `ENT`

- **Date :** 2026-11-02 — **Validateur :** Florian
- **Contexte :** les docs historiques nomment `feature/ent-rust` (`docs/ent-vision-rust.md`, `docs/ent-rust-quality.md`), branche inexistante. Le code Rust (`ent-rust/`) vit uniquement sur `ENT`.
- **Décision :** `ENT` est la branche autorisée pour l'ENT Rust. `main` = production Next.js, `dev` = intégration/labo Next.js.
- **Impacts :** mettre à jour les mentions `feature/ent-rust` dans les docs historiques. Aucun workflow GitHub ne se déclenche aujourd'hui sur `ENT`.
- **Fichiers :** `docs/ent-vision-rust.md`, `docs/ent-rust-quality.md`.

## D-002 — ENT entièrement en Rust, frontend compris

- **Date :** 2026-11-02 — **Validateur :** Florian
- **Contexte :** l'audit a montré que l'UI de l'ENT n'utilise aucun framework Rust : c'est une SPA JavaScript « vanilla » stockée dans une chaîne de 54 000 lignes (`ent-rust/crates/ent-api/src/ui.rs`), avec Tailwind v4. Les docs historiques annonçaient Leptos (`docs/ent-vision-rust.md` §1), jamais implémenté.
- **Options :** (a) garder l'UI JS actuelle et la découper ; (b) passer à un frontend Rust.
- **Décision :** (b). Front-end et back-end en Rust. L'approche JS de départ est considérée comme une erreur (voir `DECISIONS_AND_FAILURES.md`).
- **Reste ouvert :** choix du framework Rust (Leptos est la piste des docs historiques, non validée), rendu serveur et/ou WebAssembly côté navigateur, conservation de Tailwind v4 et de la PWA.
- **Impacts :** réécriture progressive de l'UI web ; l'API JSON existante (`ent-api/src/routes.rs`) reste la base commune web + mobile.
- **Risques :** volume de l'UI existante (famille, élève, staff) ; ne pas casser les parcours déjà couverts.
- **Contradiction à lever :** `CLAUDE.md` dit « conserver le framework déjà présent, ne pas choisir Leptos/Dioxus/Yew » et « WASM pas pour tout le frontend ». Ces consignes datent d'avant l'audit et sont à actualiser avec cette décision.

## D-003 — Une seule appli mobile pour tous les rôles

- **Date :** 2026-11-02 — **Validateur :** Florian
- **Contexte :** 4 prototypes existent (iOS/Android × famille/élève) dans `ent-rust/apps/`, sans fichiers de build ; pas d'appli professeur.
- **Décision :** une seule appli par plateforme (SwiftUI pour iOS, Kotlin/Jetpack Compose pour Android, pas Expo), ouverte à tous les rôles : élèves, familles, professeurs, administratifs, direction. Les écrans et actions dépendent des droits du rôle.
- **Périmètre :** version « quotidienne » allégée. Exemples : professeurs → appel, notes, espace de travail avec les élèves, visio ; tous → messagerie et actions simples. Les tâches complexes (séjours scolaires, etc.) restent sur le web.
- **Impacts :** fusionner les 4 prototypes en 2 applis ; ajouter les rôles personnel ; créer les projets de build (Gradle, Xcode).
- **Risques :** contrôle des droits obligatoirement côté serveur (l'appli ne fait qu'afficher ce que l'API autorise).

## D-004 — SMS : OVH

- **Date :** 2026-11-02 — **Validateur :** Florian
- **Contexte :** `ent-sms` n'a qu'un mode simulé (dry-run) et un fournisseur HTTP générique ; aucun fournisseur n'était choisi.
- **Décision :** OVH comme fournisseur SMS.
- **Impacts :** connecteur OVH à écrire dans `ent-rust/crates/ent-sms`.

## D-005 — Conversion de fichiers : moteur maison, briques libres

- **Date :** 2026-11-02 — **Validateur :** Florian
- **Contexte :** objectif RGPD — éviter que les fichiers d'élèves partent sur des services en ligne (iLovePDF…). convt (https://github.com/opencoredev/convt) fait ce travail, moteur en Rust, mais sous licence **AGPL-3.0-only** : l'intégrer imposerait de publier le code de l'ENT sous AGPL.
- **Options :** (1) moteur maison avec briques libres ; (2) convt en service séparé (obligations AGPL + validation juridique).
- **Décision :** (1). Conversion intégrée à l'ENT, exécutée sur nos serveurs, en Rust.
- **Règle de licence :** on peut **s'inspirer** de convt (architecture, choix des bibliothèques, isolation des conversions dans un processus séparé), mais **ne pas copier son code**, même partiellement, sous peine d'hériter de l'AGPL.
- **Briques pressenties (licences à vérifier au moment de l'intégration) :** crates `image`, `resvg`, `pdfium-render` ; outils externes FFmpeg (vidéo/audio) et LibreOffice (documents bureautiques, sortie PDF).
- **Impacts :** nouveau crate de conversion ; conversions isolées (processus séparé, limites de taille/temps) car les fichiers viennent des utilisateurs ; outils externes à installer dans l'image serveur.

## D-006 — Bureautique : suite ArtCraft en WebAssembly

- **Date :** 2026-11-02 — **Validateur :** Florian
- **Contexte :** Wordcraft (Word), GridCraft (Excel), DeckCraft (PowerPoint), organisation GitHub `storytold` (ArtCraft). Écrits en Rust, licence MIT ou Apache-2.0 (marques et logos exclus), builds navigateur en WebAssembly, lecture/écriture DOCX/XLSX/PPTX.
- **Décision :** intégrer la suite ArtCraft dans l'ENT web via WebAssembly.
- **Risques :** projets en pré-alpha (v0.4) ; fidélité limitée sur des documents complexes. Accepté vu le calendrier (D-007) : figer une version précise et la mettre à jour volontairement.
- **Impacts :** ne pas utiliser les marques/logos ArtCraft ; vérifier les fichiers LICENSE au moment de l'intégration.

## D-007 — Calendrier : sortie visée dans environ un an

- **Date :** 2026-11-02 — **Validateur :** Florian
- **Décision :** objectif indicatif de sortie de l'ENT Rust vers fin 2027 ; plus tôt si possible, sans en faire un engagement.
- **Impacts :** permet de miser sur des briques encore jeunes (D-006) et de faire la réécriture du frontend (D-002).

## D-008 — Framework frontend : Leptos

- **Date :** 2026-11-02 — **Validateur :** Florian
- **Contexte :** étude comparative (Leptos, Dioxus, Yew, Sycamore, rendu serveur askama, egui/Slint/Xilem) sur 14 critères : maturité, pérennité, documentation, travail avec Claude Code, formulaires, tableaux, état, routage, auth avec axum, appel de l'API, erreurs/chargement, accessibilité, performance sur postes modestes, mobile.
- **Options écartées :** Dioxus (API cassée à chaque version, orienté app dans le navigateur, quasi rien pour tableaux/formulaires) ; Yew (développement lent : 2 ans entre 0.21 et 0.22) ; Sycamore (un seul mainteneur, peu de code public) ; egui/Slint (rendu canvas, inaccessible) ; Xilem (expérimental) ; rendu serveur seul (écrans interactifs difficiles sans JavaScript htmx).
- **Décision :** Leptos. Pages rendues par le serveur Rust avec hydratation ; mode « îlots » là où peu d'interactivité suffit (portails famille/élève) ; intégration dans le serveur axum existant via `leptos_axum`.
- **Raisons :** seul framework combinant rendu serveur mûr, interactivité fine, intégration axum native, communauté active et volume de code récent ; tout en Rust, sans JavaScript applicatif ; léger pour les postes modestes.
- **Conditions :**
  - épingler une version précise (0.8.x, puis 0.9 une fois stable) et l'inscrire dans `CLAUDE.md` ;
  - construire notre propre design system (composants accessibles, tableaux, formulaires) — l'écosystème Leptos en propose peu ;
  - découper le frontend en plusieurs crates pour limiter les temps de compilation ;
  - **prototype avant généralisation** sur un module réel (appel ou grille de notes) : temps de compilation, taille du WebAssembly, accessibilité clavier/lecteur d'écran, tableau de 5 000 lignes, formulaire avec validation, iframe GridCraft. → **Fait** (module d'appel, 2026-11-02, voir D-010 et `docs/LEPTOS_CALL_PROTOTYPE.md`). Non couverts par le prototype : tableau de 5 000 lignes (testé jusqu'à 400), iframe GridCraft, lecteur d'écran réel.
- **Impacts sur l'existant :**
  - sortir les types échangés (`ent-api/src/dto.rs`, 247 types, dépendant aujourd'hui de `ent-db`/sqlx) dans une crate partagée sans dépendances serveur ;
  - auth web par cookie de session sécurisé (`HttpOnly`, `Secure`, `SameSite`) au lieu du jeton stocké dans le navigateur ; le jeton Bearer reste pour les applis mobiles ;
  - la nouvelle UI est servie par le même serveur axum, sous des routes distinctes ; remplacement module par module, sans migration massive ; `ui.rs` figé ;
  - bureautique ArtCraft : iframe même origine + messages (`postMessage`), indépendant du framework.
- **Risques :** pré-1.0 et un mainteneur principal ; temps de compilation ; Claude Code peut produire du code d'anciennes versions (exemples maison et version épinglée).

## D-009 — Double authentification par passkey pour tous

- **Date :** 2026-11-02 — **Validateur :** Florian
- **Contexte :** Next.js utilise Better-Auth. Il existe un projet Rust « inspiré » de Better-Auth (`better-auth` sur crates.io, organisation `better-auth-rs`) : non officiel, en alpha (1.0.0-alpha.5, oct. 2026), un seul mainteneur, schéma de base et API qui changent entre versions, aucune politique de sécurité publiée. L'ENT Rust a déjà sa propre auth (`ent-auth`) : Argon2id, sessions Postgres, passkeys via `webauthn-rs` 0.5.5, SSO OIDC, récupération par la direction.
- **Décision :**
  - conserver l'auth maison `ent-auth` / `ent-oidc` ; ne pas adopter `better-auth` (Rust) ;
  - à terme, **passkey obligatoire pour tous les comptes** (personnels, professeurs, familles, élèves) comme second facteur ou connexion sans mot de passe.
- **Existant :** la passkey est aujourd'hui facultative ; une fois enrôlée, le mot de passe seul ne suffit plus et le SSO ne la contourne pas.
- **À construire :**
  - enrôlement obligatoire à la première connexion, déploiement par vagues (personnels d'abord, puis familles et élèves) ;
  - plusieurs passkeys par compte (téléphone + ordinateur), passkeys synchronisées (iCloud, Google) acceptées ;
  - procédure de récupération en cas de perte (récupération humaine par la direction déjà prévue ; codes de secours à étudier) ;
  - cas des familles sans appareil compatible : solution de repli à décider ;
  - passkeys dans les applis mobiles (iOS, Android).
- **Point de sécurité à revoir :** `webauthn-rs` est utilisé avec l'option `danger-allow-state-serialisation` (état du challenge stocké en base) ; vérifier que cet état n'est jamais exposé au client et expire.

## D-010 — Leptos 0.8 confirmé comme socle frontend (prototype validé)

- **Date :** 2026-11-02 — **Validateur :** Florian
- **Contexte :** prototype du module d'appel (`ent-rust/web/`, `docs/LEPTOS_CALL_PROTOTYPE.md`) testé contre le vrai `ent-api` et une base locale de labo : écran complet fonctionnel, permissions intactes, backend non modifié, 15 tests automatisés verts, 0 violation axe-core, clic ≈ 17 ms pour 38 élèves (même processeur ralenti ×4), ≈ 324 Ko compressés au premier chargement.
- **Décision :** Leptos devient le socle frontend cible de l'ENT. **Validation technique, pas migration générale** : l'ancienne interface (`ui.rs`) reste en service et figée ; les modules seront reconstruits un par un, après les préalables ci-dessous.
- **Versions de référence (épinglées, `=` dans `ent-rust/web/Cargo.toml`) :**

  | Élément | Version |
  |---|---|
  | Rust | 1.99.0 + cible `wasm32-unknown-unknown` |
  | `leptos` / `leptos_router` / `leptos_meta` / `leptos_axum` | 0.8.23 / 0.8.17 / 0.8.7 / 0.8.10 |
  | `cargo-leptos` | 0.3.11 |
  | `wasm-bindgen` (crate + CLI) | 0.2.130 |
  | `wasm-opt` (binaryen) | version_123 |

  Passage à Leptos 0.9 (aujourd'hui en rc) : décision séparée, une fois la version stable publiée et testée sur le prototype.
- **Conventions et pièges :** `docs/FRONTEND_LEPTOS.md` (obligatoire avant tout développement Leptos).
- **Préalables avant de reconstruire d'autres modules :**
  1. décision sur la session web en cookie `HttpOnly` (`docs/SESSION_WEB_COOKIE_ETUDE.md`) ;
  2. crate `ent-types`, premier lot (`docs/ENT_TYPES_PROPOSITION.md`) ;
  3. `Cargo.lock` versionné, build Leptos et tests dans la CI ;
  4. test avec un vrai lecteur d'écran et sur un poste scolaire réel ;
  5. fondations du design system (tableau paginé/virtualisé, formulaires, CSP avec nonces).
- **Risques :** pré-1.0 et un mainteneur principal ; écosystème de composants pauvre (design system maison) ; temps de compilation du serveur qui embarque `ent-api` (20–30 s par modification en debug).

## D-011 — Durées de session : courtes sur le web, longues sur mobile, reconnexion rapide

- **Date :** 2026-11-02 — **Validateur :** Florian
- **Contexte :** étude `docs/SESSION_WEB_COOKIE_ETUDE.md`. Aujourd'hui : jeton valable 14 jours fixes, lisible par JavaScript. Postes partagés fréquents dans les établissements (salle des professeurs, CDI, accueil). Référence connue : École Directe impose des reconnexions fréquentes sur le web, pas sur l'appli.
- **Décision :**
  - **web** : session dans un cookie `HttpOnly` ; expiration après **2 h d'inactivité** (chaque action relance le compteur) et au plus **12 h** après la connexion ;
  - **applis mobiles** : session longue (plusieurs mois, prolongée à chaque utilisation), déverrouillage par Face ID / empreinte, révocable à distance (téléphone perdu) ;
  - **condition** : les durées courtes ne sont acceptables qu'avec un **parcours de reconnexion rapide et propre**.
- **Parcours de reconnexion attendu :**
  - passkey en un geste (Windows Hello, Touch ID, téléphone) ; adresse e-mail mémorisée ; pas de mot de passe ;
  - alerte quelques minutes avant l'expiration (« rester connecté » en un clic) ;
  - si la session expire, reconnexion **dans une fenêtre par-dessus la page** : on ne perd ni la page ni la saisie en cours ;
  - poste partagé : passkey du téléphone via QR code, ou SSO de l'établissement (Google / Microsoft) déjà présent dans `ent-oidc`.
- **Reste à arbitrer :** `SameSite=Lax` recommandé (un lien reçu par e-mail ouvre l'ENT connecté) ; durées éventuellement différenciées par rôle ; bascule de l'ancienne interface `/app` (plus tard).
- **Impacts :** évolutions `ent-auth` / `ent-api` (cookie, garde CSRF, durées par type de client, `last_seen_at`) ; parcours de connexion Leptos ; passkeys dans les applis (D-009). Aucune modification avant le détail du parcours et sa validation.

## D-012 — Un sous-domaine par établissement

- **Date :** 2026-11-02 — **Validateur :** Florian
- **Contexte :** l'application Next.js en production fonctionne déjà ainsi (ex. `lpnb.scolia.fr`, résolution du tenant par nom d'hôte). Le backend Rust, lui, **ne connaît pas encore de correspondance adresse → établissement** : pas de table des établissements avec leur domaine ; la connexion demande l'identifiant interne de l'établissement (`etablissement_id`) en clair, les pages publiques aussi (ex. fournisseurs SSO).
- **Décision :** l'ENT Rust est servi sur **un sous-domaine par établissement** ; l'établissement est déduit de l'adresse, jamais saisi par l'utilisateur.
- **Impacts :**
  - backend : table des établissements avec leur(s) domaine(s), résolution par l'en-tête `Host`, connexion sans `etablissement_id` à saisir ; contrôle que la session correspond bien au domaine utilisé ;
  - cookie `__Host-ent_session` : lié à l'adresse exacte, donc **une session par établissement**, sans fuite entre établissements ;
  - passkeys : liées à `scolia.fr` (voir D-013) ;
  - applis mobiles : rattachement à l'établissement par lien d'invitation (principal), QR code remis par l'établissement, recherche par nom ou ville en secours ; l'appli mémorise le ou les établissements.

## D-013 — Identité unique par personne, passkey liée à `scolia.fr`

- **Date :** 2026-11-02 — **Validateur :** Florian
- **Contexte :** le modèle Rust a déjà un compte par e-mail (`ent_user`, unique) et des accès par établissement (`ent_membership`), avec des sessions liées à un établissement. Les passkeys sont aujourd'hui rangées par établissement et la configuration WebAuthn n'accepte qu'une adresse.
- **Décision :**
  - une **identité par personne** au niveau de l'ENT, rattachée à un ou plusieurs établissements **avec sa confirmation** ; pas de fusion de comptes, pas de doublons ;
  - **passkey liée à `scolia.fr`** (`rp_id`) : une seule passkey pour tous les établissements, un seul domaine à déclarer dans l'appli mobile ;
  - **aucune visibilité entre établissements** : un établissement ignore qu'une famille est connue ailleurs ;
  - l'ENT connaît ces rattachements **uniquement** pour la connexion, l'absence de doublons et le pré-remplissage demandé par la personne ; pas de statistiques, de prospection ni de croisement ; pas de consultation par le personnel de l'ENT.
- **Règles de sécurité :** le serveur n'accepte que les adresses des établissements de l'ENT (liste d'origines WebAuthn) ; aucun contenu tiers ou modifiable par des tiers sous `scolia.fr`.
- **RGPD :** l'ENT devient responsable de traitement pour la couche « compte personne » ; conditions d'utilisation, notice, durées de conservation et analyse d'impact à faire valider par le DPO.
- **Impacts :** passkeys rattachées à la personne (et non plus à l'établissement), liste d'origines WebAuthn, rattachement d'une identité existante par invitation. Détail : `docs/IDENTITE_ET_PROFILS_FAMILLES.md`.

## D-014 — Comptes parents, préinscription pré-remplie, coordonnées bancaires

- **Date :** 2026-11-02 — **Validateur :** Florian
- **Décision :**
  - **comptes parents uniquement**, jamais de compte « famille » ; chaque parent a son espace, même dans un même foyer ;
  - parent du même foyer sans e-mail : présent sur la fiche famille de l'établissement, sans compte ; parents séparés : **un e-mail par parent obligatoire**, une fiche par parent ;
  - préinscription : vérification de l'e-mail (code) ou passkey **avant** tout affichage ; puis « utiliser mes informations ? » ; oui → formulaire pré-rempli et modifiable ; non → formulaire libre, même compte ; message clair : rien n'est transmis à un autre établissement sans accord ;
  - le nouvel établissement reçoit **sa propre copie**, et seulement ce qui figure dans le formulaire ;
  - second parent : e-mail indiqué par le premier, **invitation** envoyée à la validation du dossier pour confirmer l'adresse et créer (ou rattacher) son compte ;
  - **IBAN** : pas à la préinscription ; à la **validation de l'inscription**, si le parent a accepté de réutiliser son identité, proposition de reprendre ses coordonnées bancaires, affichées masquées (`FR76 •••• 1234`) ; **nouveau mandat SEPA** à signer pour chaque établissement.
- **Impacts :** vérification d'e-mail par code, profil personnel, parcours de préinscription, invitation du second parent, séparation des fiches, reprise d'IBAN tracée. Détail et règles : `docs/IDENTITE_ET_PROFILS_FAMILLES.md`. Réalisation en second temps (avec les modules préinscription / inscription).

## D-015 — Reproductibilité, CI et audits de l'ENT Rust

- **Date :** 2026-11-03 — **Validateur :** Florian (autorisation explicite de la mission « Reproductibilité et industrialisation »)
- **Décision et mise en place :**
  - `Cargo.lock` versionnés pour les deux workspaces (`ent-rust`, `ent-rust/web`) ; règle `.gitignore` ciblée ; CI en `--locked` ;
  - profil de développement allégé (`debug = "line-tables-only"`, dépendances sans info de débogage) dans les deux workspaces, après vérification debug / tests / release / traces ;
  - workflow `.github/workflows/ent-rust.yml` (branche `ENT` seulement) : formatage, Clippy, compilation backend et Leptos, tests unitaires et d'intégration sur PostgreSQL éphémère, audits ;
  - audits : `cargo deny` + `cargo audit`, règles documentées dans `deny.toml` / `.cargo/audit.toml` ;
  - crate `ent-types` (lot 1) créée et branchée sur le frontend ; `ent-api` non modifié.
- **Détails et résultats :** `docs/CI_REPRODUCTIBILITE.md`.
- **Restent à approuver :** exception de licence MPL-2.0 pour `webauthn-rs` ; commit de formatage du backend existant (706 écarts) ; plan de résorption des ≈ 65 avertissements Clippy du backend ; exception `cargo audit` RUSTSEC-2023-0071 (faux positif `rsa`) ; premier push sur `ENT` (déclenchera la CI).
- **Suites (2026-11-03) :** voir D-016 (MPL-2.0 approuvée, plan Clippy, push effectué, formatage et `rsa` encore en attente).

## D-016 — Suite de l'industrialisation : licences, Clippy, formatage, ent-types, cookie

- **Date :** 2026-11-03 — **Validateur :** Florian (message « Décisions techniques — Suite de l'industrialisation ENT »)
- **Approuvé :**
  - **MPL-2.0** : exception limitée aux 6 crates `webauthn-rs` (`webauthn-rs`, `-core`, `-proto`, `webauthn-attestation-ca`, `webauthn-authenticator-rs`, `base64urlsafedata`), **épinglée à la version 0.5.5** ; MPL-2.0 interdite ailleurs ; une montée de version fait échouer `cargo deny` et impose une relecture (vérifié par un test négatif) ;
  - **plan Clippy** détaillé, classé par gravité (`docs/CLIPPY_PLAN.md`) ; corrections par lots, jamais en une seule opération ;
  - **ent-types lot 1** côté `ent-api` : quelques DTO, contrat JSON inchangé, règles métier / routes / authentification inchangées, tests de compatibilité obligatoires ; périmètre exact présenté avant toute modification d'`ent-api` (`docs/ENT_TYPES_PROPOSITION.md` §8) ;
  - **suppression de `_outils-scolia/target-web`** (artefacts de compilation inutilisés, vérifié) ;
  - **push** des 4 commits de D-015 sur `ENT` ; aucun autre push sans autorisation.
- **Différé :**
  - **formatage général du backend** (706 écarts, 76 fichiers) : contrôle CI non bloquant maintenu ; procédure et script préparés (`docs/FORMATAGE_BACKEND.md`), **exécution soumise à validation** ;
  - **cookie HttpOnly** : principe de migration progressive validé, `SameSite=Lax` comme hypothèse de travail ; **aucune modification d'`ent-auth` / `ent-api`** ; d'abord un prototype isolé (connexion mot de passe et passkey, cookie HttpOnly/Secure, CSRF, révocation et expiration, déconnexion multi-onglets, coexistence Bearer mobile, coexistence `/app` et `/web`), résultats et risques présentés avant toute modification.
- **Toujours en attente :** exception `cargo audit` RUSTSEC-2023-0071 (faux positif `rsa`, non tranchée).
- **Réalisé le 2026-11-03 :** MPL-2.0 épinglée (`3e7ea701`) ; plan Clippy (`dd543579`) ; procédure de formatage, non exécutée (`d7716c28`) ; `target-web` supprimé (21 Go) ; puis, sur autorisation, trois commits locaux distincts : lot 1 `ent-types` dans `ent-api` (contrat JSON identique), prototype isolé de session par cookie (`ent-web-session`, pas prêt pour la production), correction de la CI (F-003). **Aucun push** sans autorisation explicite.
