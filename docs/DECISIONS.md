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
| Rôles et droits | Rôles **créés par chaque établissement** à partir d'un catalogue d'actions fourni par ScolIA, chaque action avec une portée ; cumul de rôles ; familles et élèves hors rôles libres (lien légal) | **Validé** | D-017 |
| Récupération de passkey | Par la personne elle-même, par e-mail, avec délai de sécurité et alertes | **Validé** | D-018 |
| Cible commerciale du socle 2 | **Public et privé ensemble** (ÉduConnect, GAR, interfaces nationales prévues dès le socle 2) | **Validé** | D-019 |
| Modèle personne / compte, absences | Personne distincte du compte pour les responsables ; absences à la séance avec dérivation en demi-journées | **Validé** | D-020 |
| Construction par socles | Socles profonds conçus avant d'être codés ; 1 sécurité (fait), 2 l'élève, 3 inscriptions et réinscriptions, 4 dossier administratif ; carte 5–51 proposée (`docs/SOCLES.md`) | **Validé** (ordre 1–4) ; carte 5–51 à valider | D-021 |
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

## D-017 — Rôles paramétrables par l'établissement, droits portés par les actions

- **Date :** 2026-11-05 — **Validateur :** Florian
- **Contexte :** les rôles sont aujourd'hui une liste fixe (`ent-auth/src/roles.rs` : Admin, Direction, Administratif, Cpe, Compta, Prof, Infirmerie, Psy, Parent, Eleve), un seul rôle par personne et par établissement, des droits codés en dur. Or les besoins varient : un professeur peut animer un atelier (théâtre…) et voir un groupe hors de ses classes ; un établissement peut vouloir un rôle Accueil, Surveillant, etc.
- **Décision :** ScolIA fournit une **coquille sécurisée**, l'établissement paramètre l'intérieur :
  - **catalogue d'actions** fixe, défini et contrôlé côté serveur par ScolIA (lire les notes, saisir une note, justifier une absence…) ;
  - chaque action accordée l'est avec une **portée** : établissement, mes classes (enseignement), mes groupes/ateliers (responsable), classes ou groupes désignés, moi-même ;
  - **rôles créés par l'établissement** (nom libre), modèles fournis (Professeur, CPE, Secrétariat, Comptabilité, Accueil, Direction…) copiables et modifiables ;
  - **cumul** : une personne peut avoir plusieurs rôles, droits additionnés (ex. « Professeur » + « Professeur de théâtre ») ;
  - **groupes et ateliers** créés par l'établissement dans le référentiel, avec un ou plusieurs responsables.
- **Garde-fous :** pas d'escalade (on ne donne pas un droit qu'on n'a pas) ; un administrateur indélébile ; modifications journalisées et appliquées aux sessions ; familles et élèves hors rôles libres (droits issus du lien légal et des droits cochés sur la fiche).
- **Impacts :** remplace les lots B5/B6 du socle 1 ; la règle « un professeur ne voit que ses élèves » devient la portée « mes classes » ; les rôles actuels deviennent les modèles par défaut (bascule sans changement de comportement).

## D-018 — Récupération de passkey par la personne, par e-mail

- **Date :** 2026-11-05 — **Validateur :** Florian
- **Contexte :** une personne rattachée à plusieurs établissements qui perd sa passkey ne peut être débloquée par aucune direction (isolation, revue socle 1 E4).
- **Décision :** récupération en libre-service : lien envoyé par e-mail, **délai de sécurité** (24 à 48 h) avant effet, alerte dans chaque établissement de la personne et à la personne, annulation possible pendant le délai. La récupération par la direction reste possible pour une personne d'un seul établissement.

## D-019 — Socle 2 : public et privé ensemble

- **Date :** 2026-11-05 — **Validateur :** Florian (question Q1 de `docs/SOCLE2_DOMAINES.md`)
- **Décision :** le socle 2 vise **en même temps** les établissements publics et privés (contre la recommandation « privé d'abord »).
- **Impacts :** l'interopérabilité publique (lot S2-15 : ÉduConnect, GAR, schéma directeur des ENT, ONDE/SIECLE/STS, Op@le selon vérification) n'est plus repoussée en fin de socle : à étudier et planifier en parallèle des premiers lots ; les modèles (référentiel, personnes, absences, évaluations) doivent rester compatibles avec les nomenclatures nationales. Les règles citées de mémoire dans la conception (formats, seuils) sont à vérifier sur les textes officiels avant chaque lot.

## D-020 — Personne distincte du compte ; absences à la séance

- **Date :** 2026-11-05 — **Validateur :** Florian (questions Q4 et Q6 de `docs/SOCLE2_DOMAINES.md`)
- **Décision :** (1) une **personne** (responsable, contact) existe indépendamment d'un **compte** de connexion : parent sans e-mail, parents séparés (D-014), élève majeur ; (2) les absences sont enregistrées **à la séance**, les demi-journées sont dérivées automatiquement (décompte réglementaire).
- **Impacts :** lots S2-1 (dossier élève et personnes) et S2-4 (vie scolaire complète) ; migrations additives avec reprise de l'existant.

## D-021 — Construction de l'ENT socle par socle ; ordre des quatre premiers socles

- **Date :** 2026-11-06 — **Validateur :** Florian (ordre des socles 1 à 4) ; reste de la carte : proposition.
- **Contexte :** la feuille de route du 2026-11-05 regroupait tous les domaines dans un « socle 2 » (`docs/SOCLE2_DOMAINES.md`, 26 domaines, lots S2-0 à S2-15) et les fonctions différenciantes dans un « socle 3 ».
- **Décision :** l'ENT se construit **socle par socle**, chaque socle étant un domaine entier, conçu en profondeur avant d'être codé, relié aux autres par la toile des événements. Ordre fixé : **1 sécurité** (fait), **2 l'élève**, **3 inscriptions et réinscriptions**, **4 dossier administratif** (foyers, responsables, parents séparés, coordonnées, identifiants officiels). Florian estime le total à 40–50 socles et en confie l'identification à Claude.
- **Proposition (non validée) :** carte de 51 socles en 12 étages, avec jalons (dont « pilote » après le socle 29) : `docs/SOCLES.md`. L'ancien « socle 3 » devient l'étage H (socles 30 à 33). Questions ouvertes : `SOCLES.md` §8 (ordre 5–51, paie, comptabilité, position de l'IA, premier degré, post-bac).
- **Impacts :** `docs/ROADMAP.md` suit la carte ; « socle 2 » désigne désormais l'élève (D-019 s'applique à tous les socles) ; `SOCLE2_DOMAINES.md` reste la référence des actions et événements ; chaque socle commence par un document `docs/socles/NN_<NOM>.md` (premier : `docs/socles/02_ELEVE.md`).
- **Risque signalé :** le socle 3 crée des responsables avant que le socle 4 n'approfondisse leur modèle ; règle : le socle 3 n'écrit les personnes que par les commandes du domaine « personnes » existant.
- **Méthode :** pour chaque socle, une réflexion de fond sur Opus, en autonomie (Florian ne liste pas les actions), écrite dans `docs/socles/` ; puis le code par une équipe Sonnet ; puis la relecture de Florian. On ne découpe pas en largeur sur plusieurs domaines à la fois.
- **Méthode complétée (Florian, 2026-11-07) :** cycle de chaque socle = (1) conception de fond sur Opus (tables, actions, règles, droits, écrans) ; (2) passe de personas sur la conception ; (3) code sur Sonnet ; (4) **recette par personas sur le logiciel réellement codé**, massive et dans tous les sens : chaque persona se connecte (données fictives de labo) et fait son vrai travail pour trouver les fonctions manquantes, basiques ou complexes, et **attaque** le système (un élève qui tente de voir un autre élève, un parent d'une autre famille, un professeur hors de ses groupes, un autre établissement…) ; chaque trouvaille devient un test automatique ou une correction ; (5) relecture de Florian. Un socle n'est « fini » qu'après la recette (4).
- **Lots en cours et arrêts du 2026-11-06 :** les lots S2-x en cours (EDT, évaluations, communication, finances, santé) sont terminés sans élargissement et serviront de base aux socles correspondants. Six sessions de conception lancées en largeur le même jour ont été arrêtées après quelques minutes (environ 18 $ consommés, rien de livré).

## D-022 — Exports officiels réservés aux administratifs ; portée du professeur au groupe

- **Date :** 2026-11-06 — **Validateur :** Florian
- **Décision :** (1) les exports officiels (LSL, LSU, Notanet, Parcoursup, ONDE…) sont faits par les administratifs (direction, secrétariat) ; **un professeur n'exporte jamais** : l'action d'export est retirée du modèle de rôle « professeur ». (2) Un professeur ne voit que les élèves des **groupes** qu'il enseigne à l'intérieur d'une classe (exemple : LV2 allemand et espagnol dans la même classe, le professeur d'allemand ne voit que le groupe allemand), pour toute donnée élève (appel, notes, appréciations, cahier de textes, vie scolaire), et non la classe entière.
- **Principe (Florian) :** les matières, groupes, options, services d'enseignement et leurs rattachements sont du **paramétrage de l'établissement**, librement créés et nommés par lui (aucune liste figée dans le code) ; le logiciel ne code que la règle générique « un professeur voit les élèves des groupes et services qui lui sont attribués ». Conception détaillée : socle 5 (structure de l'établissement, `docs/SOCLES.md`).
- **Précision (Florian, 2026-11-07) :** la portée au groupe vaut **partout, conseil de classe compris** : un professeur qui n'a qu'un groupe dans une classe ne voit, dans le dossier du conseil, que les élèves de son groupe. Un professeur peut enseigner à la fois à des **classes entières et à des groupes** : pour faire l'appel, saisir des notes, donner ou corriger un travail, il **choisit dans une seule liste de ses enseignements** (ses classes et ses groupes mélangés), sans distinguer les deux. Cela confirme l'objet unique « cours » proposé par `docs/socles/PERSONAS_05-06.md` (Q-S20, Q-P28) : une seule notion d'enseignement (classe entière ou groupe), une seule règle de visibilité.
- **Impacts :** modèles par défaut de `ent_auth` (D-017), `perimetre.rs` / `eleves_autorises` (portée par groupe et service d'enseignement), constat RF-04 de `REVUE_SECURITE_SOCLE1.md` ; tests de séparation à ajouter (professeur de groupe, classe mixte).

## D-023 — Automatismes et modèles prêts plutôt qu'une liberté de mise en forme totale

- **Date :** 2026-11-06 — **Validateur :** Florian (secrétariat de lycée, utilisateur de Charlemagne)
- **Constat (Florian) :** Charlemagne est « une usine à gaz » : beaucoup trop de réglages accumulés en vingt ans et jamais allégés. Exemple vécu : des bulletins conçus sous Excel à la livraison du logiciel, restés tels quels vingt ans ; des courriers (sanctions…) à construire de A à Z dans Word. La liberté totale produit des documents laids et figés, et du travail inutile.
- **Décision :** ScolIA privilégie **les automatismes et des modèles prêts à l'emploi** à la liberté de mise en forme. Pour les documents produits (bulletins, courriers de sanction, convocations, certificats, relevés, attestations…) : **3 ou 4 trames soignées par type**, au design moderne, l'établissement **choisit** la sienne et n'ajuste que l'identité (logo, couleurs, coordonnées, signature, quelques textes libres encadrés). Pas d'éditeur de mise en page de A à Z, pas de modèle Excel ou Word à construire soi-même. Le contenu se remplit automatiquement à partir des données.
- **Articulation avec D-022 :** la **liberté reste totale pour les données de l'établissement** (matières, groupes, options, rôles, services, motifs…), qui reflètent son organisation réelle. La retenue s'applique à la **présentation** et aux réglages qui n'apportent rien. Règle générale : chaque réglage doit justifier son existence ; par défaut, un comportement automatique et raisonnable.
- **Priorité de construction (Florian, même jour) :** d'abord une **couverture métier au moins équivalente à un ENT classique de type Charlemagne**, **plus** les actions réelles du terrain que ces logiciels n'ont pas prévues (exemple vécu : l'accueil qui saisit l'absence signalée par un parent au téléphone et la transmet directement à la vie scolaire, au lieu d'un Excel partagé). On ne coupe rien d'avance : les réglages jugés « usine à gaz » sont **signalés**, pas supprimés, et seront tranchés avec Florian lors des relectures. Les modèles prêts à l'emploi restent la règle pour la mise en forme des documents.
- **Impacts :** socles 7 (documents et signatures), 13 (discipline : courriers), 23 (bulletins), 3 (certificats, courriers d'admission) et tout socle qui produit un document ; chaque conception signale les réglages candidats à l'allègement, pour décision commune.

## D-024 — Cible de la première version : un Charlemagne complet, moderne, avec l'IA et le dossier élève au cœur

- **Date :** 2026-11-06 — **Validateur :** Florian
- **Décision :** la première version livrable de ScolIA est **un équivalent entièrement fonctionnel d'un ENT/logiciel de vie scolaire de type Charlemagne** (couverture métier au moins égale), avec trois différences dès la base : (1) **une interface plus soignée et moderne** ; (2) **le dossier élève au centre** de la navigation et du modèle (socle 2) ; (3) **l'IA intégrée au cœur** du système, pas ajoutée à la fin. Les fonctionnalités différenciantes de Florian viendront ensuite, sur cette base ; des allègements seront décidés avec lui après usage (D-023).
- **Conséquences sur le plan (`docs/SOCLES.md`) :** (a) une **matrice de couverture Charlemagne** (modules et fonctions publiques → socles), pour garantir la parité et repérer les trous ; (b) l'IA n'attend plus le socle 30 pour exister : son **architecture de cœur** (l'IA agit par les services métier avec exactement les droits de l'utilisateur, catalogue d'actions décrites, confirmation humaine des actions sensibles et des envois, journal, minimisation, fournisseur souverain à l'étude — Mistral prioritaire, CLAUDE.md) est conçue maintenant, et chaque socle expose ses actions et lectures à l'IA au moment où il est codé. Les écrans conversationnels restent au socle 30.

## D-025 — Tout le périmètre dans l'ENT Rust : paie, comptabilité, génération d'emploi du temps ; principe de l'IA de cœur

- **Date :** 2026-11-06 — **Validateur :** Florian (réponses à Q-B, Q-C, Q-F de `SOCLES.md` et Q-IA1 de `socles/IA_COEUR.md`)
- **Décision :** (1) **Paie** (socle 49) et **comptabilité d'organisme** (socle 48, OGEC pour le privé ; interface avec l'agent comptable pour le public) sont **construites dans l'ENT Rust**, et font partie de la cible « équivalent Charlemagne » (D-024) ; rien n'est ajouté à l'application Next.js de production. (2) La **construction automatique de l'emploi du temps** (génération sous contraintes) est dans le périmètre (socle 10 ou socle dédié, à placer dans la carte). (3) **Principe de l'IA de cœur validé** : l'IA n'a aucun droit propre, elle est cliente des services métier avec exactement les droits de l'utilisateur, par un registre de capacités que chaque socle déclare ; le « contrat IA » entre dans les critères de fin de chaque lot (`IA_COEUR.md`).
- **Principe général (Florian) :** « tout faut faire » — les trous relevés par `MATRICE_CHARLEMAGNE.md` entrent dans la carte ; on allège ensuite, ensemble, à l'usage (D-023).
- **Impacts :** `SOCLES.md` (socles 48, 49, génération d'EDT, nouveaux socles N1–N3 de la matrice à intégrer), `IA_COEUR.md` (Q-IA1 close), étude réglementaire paie (DSN, conventions collectives, paie des enseignants du privé) et comptabilité (plan comptable des associations, FEC) à mener lors de la conception de ces socles.

## D-026 — Architecture en briques : chaque domaine se branche sur le cœur par un kit standard

- **Date :** 2026-11-07 — **Validateur :** Florian (intuition « c'est une addition de briques »), confirmée par la revue du contradicteur du pilotage.
- **Constat :** chaque lot modifie les mêmes fichiers centraux (liste des migrations, `MIGRATION_TAG`, catalogue des actions `ent-auth/actions.rs`, `ent-db/service.rs` de plus de 18 000 lignes, routes, inventaire de séparation, `perimetre.rs`) : les lots parallèles se télescopent et l'intégration devient le goulot.
- **Décision :** un **cœur** stable (élève, personnes et foyers, structure, personnels, droits et périmètre, cloisonnement et RLS, événements, fichiers, journal, contrat IA) expose un **kit de branchement** documenté ; chaque domaine (infirmerie, notes, cantine, internat…) est une **brique** dans son propre crate et ses propres fichiers : ses migrations dans sa plage, ses actions déclarées chez lui, ses routes enregistrées par une seule ligne, ses tests d'isolation générés à partir de la déclaration de ses routes. `MIGRATION_TAG` et les listes globales sont calculés automatiquement. Un nouveau domaine reçoit le kit et la conception de son socle ; il ne touche pas aux fichiers du cœur.
- **Mise en œuvre :** conception du kit et du découpage (Opus), puis refonte par étapes après l'intégration des lots en cours, sans changer le comportement (tests et CI identiques avant et après).

## D-027 — Socle 7 (documents) : recommandations des personas adoptées par défaut

- **Date :** 2026-11-07 — **Décideur :** le pilote, au titre de la sécurité et de la minimisation (délégation de Florian sur les choix de sécurité) ; Florian peut revenir dessus à la relecture.
- **Décision :** les points A à H du §0 de `docs/socles/PERSONAS_07.md` sont adoptés tels que recommandés : (A) les dossiers des personnels sont un domaine restreint distinct des dossiers d'élèves ; (B) un certificat destiné à un tiers n'imprime que le niveau et la formation, jamais une division ou un dispositif révélateur (SEGPA, ULIS, UPE2A…) ; (C) un titulaire sans compte peut signer par lien nominatif à usage limité ; (D) le délai d'une procédure court à partir de la notification, chaque nature déclarant les modes qui valent notification ; (E) une facture ou une quittance ne se réémet pas, elle se compense (avoir) ; (F) l'entité émettrice est portée par famille de documents (établissement ou organisme de gestion) ; (G) l'envoi à une personne sans compte se fait par lien nominatif, jamais en pièce jointe ; (H) les pièces interdites aux familles ne s'appliquent pas aux pièces RH exigées par le droit du travail. Les modèles de courriers restent 3 ou 4 trames prêtes par type (D-023).

## D-028 — Socle 8 (notifications) : choix par défaut du pilote

- **Date :** 2026-11-08 — **Décideur :** le pilote ; Florian peut revenir dessus.
- **Décisions :** (Q-N3) fournisseur d'e-mail : **Scaleway TEM** (cohérent avec l'hébergement français et l'existant Next.js), OVH reste le fournisseur SMS (D-004) ; (Q-N9) « ne pas solliciter » : jamais aucune sollicitation (invitation, demande d'accord, relance, diffusion, sondage) ; pour les informations dues (absence, bulletin, convocation), la direction décide pour chaque personne (courrier ou coffre sans révéler l'établissement, ou suspension) ; à défaut de décision, une tâche bloquante est créée pour la direction, rien ne part en silence ; (Q-N1) les notifications sont inscrites dans la même transaction que le fait qui les provoque et envoyées après validation (file d'envoi).
- **En attente de Florian :** Q-N5 (SMS automatique aux familles pour les absences et les signalements de l'accueil, et délai avant envoi) et Q-N22 (SMS inclus dans l'abonnement ou facturés à l'établissement).

## D-029 — Ouverture au public avant 2028 : ÉduConnect avant le GAR (Florian, 2026-11-09)
- **Décision** : ScolIA vise aussi les collèges et lycées **publics**, avec une ouverture possible avant 2028 (réponse à Q-X47).
- **Conséquence** : le calendrier de l'annexe 4 de l'arrêté du 7 septembre 2026 (JORF du 23/09/2026) s'applique à ScolIA comme « service numérique outillant les missions de vie scolaire » : ÉduConnect et protocoles d'authentification ouverts sous 10 mois (vers juillet 2027), guichets Agents et GAR sous 22 mois, Annuaire fédérateur sous 34 mois (`docs/socles/VERIF_09_SOURCES.md`, lignes 1 à 4).
- **Ordre des lots du socle 9** : X8 (fédération / ÉduConnect) puis X9 passent **avant** X11–X13 (GAR et annuaires).
- **Démarches à anticiper** (hors code) : inscription éditeur (Pléiade) pour les spécifications SIECLE, ONDE, LSU, LSL ; raccordement ÉduConnect ; pour le GAR, demande portée par une collectivité ou une académie via la DANé (Q-X45 pour le privé isolé).
- Le privé sous contrat reste la cible du premier pilote (établissement de Florian) ; l'arrêté ne s'impose pas à lui mais les mêmes briques serviront.

## D-030 — ScolIA construit lui-même l'emploi du temps, avec l'IA (Florian, 2026-11-09)
- ScolIA gère **son propre** emploi du temps, sans logiciel d'EDT externe : saisie à la main, **génération automatique**, modifications en cours d'année (remplacements, salles, sorties, fermetures).
- **Aide à la création par l'IA** : l'objectif est le plus intelligent et le plus automatisé possible. Prise en compte des **vœux et contraintes des enseignants**, des **choix des élèves** (options, groupes, enseignements suivis du socle 5), des salles, de la grille, du calendrier ; propositions expliquées, scénarios comparés, la personne valide (l'IA n'a jamais plus de droits que l'utilisateur, CLAUDE.md).
- L'import des logiciels d'EDT du marché (EDT, UnDeuxTEMPS, aSc) et l'export Charlemagne ne servent qu'à la reprise et aux futurs clients ; ils passent **après** les lots de saisie, de génération et d'IA (socle 10, `10_EMPLOI_DU_TEMPS.md`).

