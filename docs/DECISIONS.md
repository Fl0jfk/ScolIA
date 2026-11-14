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
| Sauvegardes, PRA, portabilité | Deux destinations actives (Scaleway + OVHcloud), PITR, restauration et export par établissement, double validation des exports complets | **Proposé, à valider par Florian** | D-037 à D-046, `PRA_SAUVEGARDES.md`, `PORTABILITE_EXPORTS.md` |
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

## D-031 — Règle : les recommandations du pilote s'appliquent par défaut (Florian, 2026-11-09)
- Florian : « je suis toujours tes recommandations ». Les questions des conceptions, personas et recettes sont **tranchées selon la recommandation écrite** du pilote ou de la session, sans attendre une réponse, et notées « appliqué par défaut (D-031) ».
- Restent soumis à Florian avant d'agir : ce qui engage l'établissement à l'extérieur (démarches officielles, contrats, coûts, données réelles), ce qui touche une interdiction de CLAUDE.md, et ce qui contredit une décision existante. Le point quotidien liste les recommandations appliquées pour qu'il puisse revenir dessus.

## D-032 — Socle 21 Évaluations : recommandations appliquées (2026-11-09)
- **V0** (correctifs urgents de l'existant) avant tout autre lot du domaine — lancé.
- **Lycée pilote, an 1** (Q-EV11) : notes et bulletins restent dans Charlemagne l'an 1 tant que la validation éditeur LSL / Parcoursup n'est pas obtenue ; démarche éditeur **reportée** par Florian (vers mai 2027, une fois le logiciel crédible ; `A_FOURNIR_PAR_FLORIAN.md`, F-06). Les lots LSU/LSL/Parcoursup du socle 21 et les échanges officiels du socle 9 attendent ; le reste avance.
- **Publication** (Q-EV2) : explicite par le professeur ; tout le non-publié l'est automatiquement à l'arrêt des notes.
- **Fenêtre du conseil** (Q-EV6) : pendant la préparation et la tenue du conseil, un professeur voit toutes les matières des seuls élèves de ses groupes, journalisé.
- **IA** (Q-EV8) : propositions d'appréciations relues une à une, alertes déterministes, aucune note ni décision par l'IA ; étude juridique (règlement européen sur l'IA) avant la mise en service des fonctions à modèle de langage.

## D-033 — Socle 10 Emploi du temps : bloquants des personas tranchés selon leurs recommandations (2026-11-09)
- **A** : les choix des élèves servent à **construire** les barrettes et les groupes (pas une entrée figée), répartition des élèves après l'EDT possible.
- **B** : explications de l'IA **typées par source et visibilité**, filtrées côté serveur : jamais la contrainte d'un collègue nommée.
- **C** : versions **de base** et **temporaires** (stage, voyage, suppléant, salle fermée), retour automatique au plan.
- **D/E/F** : saisie accélérée possible tôt pour le pilote (EDT-216 en T1 si besoin) ; AESH intégrée (§8.3 bis) ; les six pièges de minimisation corrigés (R-E27 et suivantes).


## D-034 — Feuille de route vers la mise en service (Florian, 2026-11-10)
- **Début décembre 2026** : première démo du quotidien (élève et familles, inscriptions, structure et cours, EDT, appel et absences, notes et bulletins, messagerie, personnels).
- **Fin janvier – février 2027** : ENT complet en V1 — les 51 socles fonctionnels avec leurs lots essentiels, passés aux personas et validés par la CI, sans épaississement.
- **Février – août 2027** (environ 6 mois) : épaississement jusqu'à une profondeur digne d'un vrai ENT.
- **Année 2027-2028** : **année de doublon** dans l'établissement de Florian — ScolIA tourne en parallèle de Charlemagne toute l'année pour repérer manques, erreurs et améliorations ; Charlemagne reste l'outil officiel.  Le développement continue pendant le doublon : manques et améliorations relevés au fil de l'année sont corrigés et livrés au fur et à mesure (versions fréquentes, sans perturber le parallèle).
- **Rentrée 2028** : bascule envisagée si l'année de doublon est concluante (cohérent avec D-029, ouverture au public avant 2028 visée pour la commercialisation).
Préalables à l'année de doublon (à préparer au printemps 2027) : accord écrit de l'établissement avant toute donnée réelle (`MISE_EN_SERVICE_DONNEES.md` A8/A10), avis du DPO (durées de conservation, registre), hébergement en France (Scaleway) avec module santé désactivé tant qu'il n'y a pas d'hébergeur HDS, reprise de données depuis Charlemagne (F-01 à F-04), inscription éditeur et validation des interfaces utiles (F-06), décision sur la double saisie (ce qui est saisi dans les deux outils, ce qui est importé de Charlemagne). Finances : rester dans Charlemagne pendant le doublon, ScolIA calcule en parallèle sans prélever (recommandation socles 24–25).

## D-035 — L'add-on Next.js passe dans ScolIA et sert de porte d'entrée (Florian, 2026-11-10)
**Fait** : l'application Next.js en production est aujourd'hui un complément de Charlemagne dans l'établissement (inventaire : `MATRICE_CHARLEMAGNE.md` §4 — voyages et sorties, stages, internat, absences et RH du personnel, demandes, certificats, fiches de dialogue, photocopies, HSE, covoiturage, accueil et portes ouvertes, inscriptions et rendez-vous, cantine, facturation EasyTransac, messagerie…). Les personnels l'utilisent déjà.
**Décision** : ces fonctions sont reprises dans ScolIA (Rust), module par module, et deviennent la porte d'entrée : en venant y faire ce qu'ils font déjà, les utilisateurs découvrent le reste de l'ENT.
**Conséquences** (recommandations du pilote, D-031) :
1. Pendant l'année 2027-2028, deux régimes : **usage réel** pour les fonctions de l'add-on (Charlemagne ne les fait pas, ScolIA remplace Next.js) ; **doublon** pour le reste (Charlemagne reste officiel).
2. Les socles qui portent ces fonctions (notamment 6 personnels et RH, 14 accueil, 42 sorties et voyages, stages, internat, demandes, cantine) visent un niveau **production** pour la rentrée 2027, pas seulement V1 ; ils sont épaissis en priorité entre février et août 2027.
3. Remplacement **progressif** : Next.js reste en service tant que le module Rust équivalent n'a pas fait ses preuves ; pas de grand basculement. Next.js reste intouchable (CLAUDE.md) : le pilote ne le modifie ni ne l'arrête ; l'arrêt de chaque module Next.js est une décision et une action de Florian.
4. Données de l'add-on : la base de production reste inaccessible au pilote ; la reprise passera par des exports que Florian fournira (anonymisés pour les essais, réels seulement après l'accord écrit de l'établissement et l'avis du DPO, à obtenir au printemps 2027, plus tôt que prévu par D-034).
5. Rien de ce qui marche aujourd'hui dans Next.js ne doit être perdu : chaque conception de socle concerné garde sa revue « à ne pas perdre » (§2.4) et la recette compare avec Next.js.
**Précision (Florian, 2026-11-10)** : les modules de l'add-on (voyages et séjours scolaires, réservation de salles, etc.) sont **déjà fonctionnels et utilisés** ; on ne les réinvente pas. Méthode pour chacun :
1. **Partir des actions métier existantes, sans les copier telles quelles** (lecture seule et ciblée du code Next.js, autorisée par CLAUDE.md pour comprendre ; aucune modification) : les actions métier sont là et fonctionnent, on les garde ; mais le module est **réécrit en Rust** (axum, sqlx, Leptos), son **interface refaite** selon notre direction artistique (`DESIGN_SYSTEM.md`, `FRONTEND_CIBLE.md` : sobre, qualité Apple, tuiles Bento pour les tableaux de bord, tableaux denses pour la saisie), et on **repasse dessus** : bugs corrigés, améliorations utiles et finitions repérées en chemin (listées pour Florian quand elles changent un usage).
2. **La valeur ajoutée attendue est le branchement sur le cœur de ScolIA**, alors qu'aujourd'hui les données ont été importées une fois et ne bougent plus. Exemple du séjour scolaire : participants pris dans la base élèves (socle 2) et les classes et groupes (5), autorisations et responsables légaux du dossier administratif (4), payeurs et facturation, échéancier et encaissements dans les finances (24–25), budget et comptabilité du séjour, encadrants dans les personnels (6), signatures et pièces (7), informations aux familles (8, 16), séances concernées et appel pré-rempli « en sortie » (10, 12), santé et PAI des participants (34) avec partage minimal ; tout changement côté élève (départ, changement de classe, changement de responsable ou de payeur) se répercute automatiquement.
3. La conception de chaque socle concerné part donc de l'inventaire du module Next.js (« ce qu'il fait ») puis liste les branchements, plutôt que de repartir de zéro.
**Précision 2 (Florian, 2026-11-10)** : les modules Next.js n'ont pas tous la même maturité. Certains sont des **ébauches à peine commencées** (notamment comptabilité et paie, RH, pilotage de l'établissement) : ils **ne font pas foi** et sont refaits de zéro par la méthode des socles (conception, personas, code), leurs données étant probablement perdables. La méthode « reprendre les actions métier » ne vaut que pour les modules **en usage réel et fiables** (notamment voyages et séjours scolaires, réservation de salles). L'inventaire T0 classe chaque module selon sa maturité ; Florian confirme.

## D-036 — Plus aucun JSON qui sert de mémoire (Florian, 2026-11-10)
**Décision** : toutes les données métier sont stockées dans des **tables PostgreSQL normalisées** (colonnes typées, clés étrangères, contraintes, RLS). Interdits : fichiers JSON servant de base de données (sur disque ou en stockage objet), colonnes JSON/JSONB contenant des données métier qu'on lit, modifie ou recherche (dossier, droits, paramètres, règles, listes…).
**Reste permis** (recommandation du pilote, D-031 ; ce n'est pas de la « mémoire ») : JSON comme **format d'échange** de l'API ; **instantanés figés** d'historique ou d'audit (valeur avant/après, copie d'un bulletin publié), jamais relus comme source de vérité ; **état technique opaque** imposé par une bibliothèque (ex. état d'une passkey `webauthn-rs`) ; **charge utile transitoire** d'une file d'envoi. Chaque exception est listée et justifiée dans `docs/socles/MIGRATIONS.md`.
**Conséquences** :
1. Audit de l'existant Rust (environ 25 colonnes JSONB sur `ENT` au 2026-11-10, ex. `134_vs_discipline.dossier`, `172_delegation.droits`, `302…regle_relance`, `304…scolarite_actuelle`, `332…jours_ouvres` et `valeur`, `144_import_lot.donnees`, `250…resolutions`) : chaque colonne est classée « exception permise » ou « à normaliser », et les colonnes à normaliser reçoivent une migration de reprise vers des tables.
2. Toute nouvelle conception et tout nouveau lot respectent la règle ; la revue du pilote la vérifie (`grep -i jsonb` sur les migrations ajoutées).
3. Transition (D-035) : les modules Next.js qui stockent encore leurs données en JSON (fichiers ou objets S3) sont inventoriés en T0 ; leurs données sont reprises dans des tables, et un `pg_dump` seul ne suffit pas pour eux (export des fichiers JSON à prévoir).

## D-037 à D-046 — Sauvegardes, PRA et portabilité (proposées le 2026-11-10, **à valider par Florian**)
Issues de la mission `docs/missions/MISSION_PRA_SAUVEGARDES.md`. Détail et justification : `docs/PRA_SAUVEGARDES.md` et `docs/PORTABILITE_EXPORTS.md`. **Aucune n'est décidée** ; numéros provisoires si une autre session ajoute des décisions entre-temps. D-031 ne s'applique pas d'office : D-039, D-040 et D-043 engagent des coûts ou des contrats.

### À prendre maintenant (elles contraignent le modèle de données ou les lots en cours)

## D-037 — Un seul moteur d'extraction par établissement, et un registre de classement des tables (proposé, à valider par Florian)
- **Contexte** : une PITR PostgreSQL est globale ; restaurer ou exporter un établissement exige d'extraire ses lignes et ses fichiers proprement (identité partagée `ent_user` référencée par 163 FK, ~500 FK non composites, effets de bord en file).
- **Proposition** : crate `ent-extraction` unique pour la restauration sélective, les tests de restauration, les exports de portabilité et le format de manifeste de `ent-reprise` ; **registre de classement** de chaque table dans le code (portée, catégorie d'export, neutralisation à la restauration) avec un **test CI qui échoue** si une table n'est pas classée.
- **Impacts** : lot EXP-0 réalisable tout de suite ; chaque nouvelle migration classe ses tables.

## D-038 — Exigences de modèle de données pour la restaurabilité et la portabilité (proposé, à valider par Florian)
- **Proposition** : les exigences E1 à E14 de `PRA_SAUVEGARDES.md` §18.4 deviennent des règles de revue pour tous les lots : `etablissement_id NOT NULL` partout (tables filles comprises), FK composites pour les nouvelles tables, UUID applicatifs, pas de nouvelle table globale à données d'établissement, fichiers hors base immuables avec empreinte, horodatages, journal, migrations rejouables depuis tout état antérieur, files d'effets de bord déclarées, D-036, clés inventoriées avec identifiant, RLS.
- **Impacts** : à transmettre par le pilote aux sessions de code (consigne de lancement, `PILOTAGE.md` §2).

## D-039 — Hébergement de PostgreSQL compatible avec un PITR vers nos propres dépôts (proposé, à valider par Florian)
- **Faits (2026-11-10)** : la base managée Scaleway ne documente ni PITR ni accès aux WAL (sauvegarde quotidienne, 7 j) ; les outils PITR exigent `archive_command` ou la réplication physique.
- **Proposition** : (1) T1 et année de doublon : base managée Scaleway + `pg_dump` horaires chiffrés vers les deux destinations (RPO ≈ 1 h, acceptable car Charlemagne reste officiel) ; (2) **avant le premier usage réel** (modules de l'add-on, rentrée 2027, D-035) : PostgreSQL autogéré sur instance Scaleway + **pgBackRest** (MIT) avec `repo1` Scaleway et `repo2` OVHcloud, `archive_timeout` 60 s (RPO ≤ 5 min) — **sauf** si Scaleway confirme un PITR sur l'offre managée (Q-PRA1, à poser par Florian).
- **Risques** : exploitation d'un PostgreSQL autogéré (mises à jour, HA).

## D-040 — Deux destinations de sauvegarde actives et indépendantes, objets immuables (proposé, à valider par Florian)
- **Proposition** : Scaleway (organisation ou projet **séparé** de la production) et OVHcloud (compte distinct, Paris 3-AZ), en France, toutes deux versionnées avec **Object Lock** (GOVERNANCE 35 j au départ) ; réplication par notre worker (états par destination, réconciliation par manifestes), pas par les fonctions natives ; identifiants d'écriture sans droit de suppression ; **objets de production immuables** (nouvelle version = nouvelle clé, empreinte en base).
- **Conséquence immédiate** : lot de correction d'`ent-object` (écriture S3 signée, erreurs remontées, empreinte) et sortie des `BYTEA` (`ent_msg_piece`, `ent_pai_piece`).

## D-041 — Double validation des exports complets, réauthentification par passkey (proposé, à valider par Florian)
- **Proposition** : actions `donnees.export.initier`, `.approuver`, `.planifier`, `.sante` ; exports complets et domaines nominatifs à **deux personnes distinctes** (comptes, personnes D-020 et passkeys différents, contrôlés en base) ; session ouverte par passkey **et** réauthentification passkey de moins de 5 min (nouvelle colonne `ent_session.authentification_forte_le`) ; suppléance par `ent_delegation`, approbateur extérieur désigné à l'avance en cas d'absence ; aucun contournement par un administrateur technique (limite assumée : l'opérateur de la plateforme a l'accès infrastructure). Détail : `PORTABILITE_EXPORTS.md` §4–§6.

### Peuvent attendre

## D-042 — Clés : séquestre hors Scaleway, identifiant de clé, exercice annuel (proposé, à valider par Florian ; avant T1)
- Séquestre des phrases de passe pgBackRest, de `ENT_SECRETS_KEY` (et anciennes versions) et de la clé de signature des manifestes : Secret Manager + deux exemplaires hors ligne + un coffre hors Scaleway ; format chiffré `enc:v2:<id_cle>:` ; rotation annuelle ; exercice annuel de restauration depuis OVHcloud sans accès à Scaleway.

## D-043 — Rétentions des sauvegardes (proposé, à valider par Florian **et le DPO**)
- PITR 35 jours sur chaque destination ; 12 sauvegardes mensuelles ; effacement dans les sauvegardes à l'expiration de la rétention (à écrire dans la notice et les contrats).

## D-044 — Troisième copie froide immuable (proposé, à valider par Florian ; vers ~50 établissements)
- Copie mensuelle en OVHcloud Cold Archive (≈ 0,3 à 20 €/mois selon l'échelle), contre la perte simultanée des deux comptes ou un défaut de notre worker.

## D-045 — Pas d'infrastructure de reprise chaude au lancement (proposé, à valider par Florian)
- RTO réaliste « Scaleway perdu » : 8–24 h, ≈ 4 h après répétitions ; une reprise chaude chez OVHcloud (≈ 100–400 €/mois) n'est pas justifiée pour un établissement en doublon ; à réexaminer à la commercialisation ou sur exigence contractuelle.

## D-046 — Format d'export de portabilité (proposé, à valider par Florian ; avant le lot EXP-1)
- Archive avec `manifeste.json` signé, vue métier CSV, vue technique JSON Lines (format d'échange, compatible D-036), fichiers originaux, dictionnaire et `LISEZMOI` ; ZIP AES-256 ou `age` (Q-EXP1) ; rétention 7 jours ; bucket d'exports hors réplication.


## D-047 — Comptabilité et paie entièrement dans ScolIA : D-025 confirmée (Florian, 2026-11-11)
**Contexte** : les conceptions des socles 48 et 49 recommandaient un partage (ScolIA prépare ; logiciel du cabinet pour la tenue légale via FEC ; logiciel de paie pour le calcul et la DSN) — questions Q-CG1 et Q-RH1.
**Décision** : Florian **confirme D-025** : la comptabilité de l'organisme (tenue légale des comptes comprise) et la paie (calcul des bulletins et DSN comprises) sont construites dans ScolIA.
**Conséquences** : les conceptions 48 et 49 sont révisées en ce sens, avec les exigences propres à ces domaines (conformité de la DSN au cahier technique net-entreprises et tests de recette, mises à jour réglementaires annuelles et en cours d'année, piste d'audit fiable et inaltérabilité des écritures, FEC conforme à l'article A47 A-1 du LPF, archivage légal, facturation électronique en réception depuis le 1er septembre 2026 via une plateforme agréée, sécurité des données des salariés) ; l'export FEC reste disponible pour l'expert-comptable ; une phase de paie et de comptabilité en parallèle (« doublon ») précède toute bascule réelle, comme pour le reste (D-034).
**Mise en œuvre (2026-11-11, branche `ent-socle-48-49-revision`)** : `48_COMPTABILITE.md` §35 et `49_PAIE.md` §34 (révisions qui l'emportent), `PERSONAS_48.md` §8, `PERSONAS_49.md` §8 ; Q-CG1 et Q-RH1 closes ; bascule recommandée au 1-9-2028 au plus tôt pour la comptabilité (début d'exercice) et au **1-1-2029** pour la paie, sous critères chiffrés (K-CG, K-RH) ; points soumis à Florian : 48 §35.16, 49 §34.15.

## D-048 — Comptabilité et paie « light », limitées à un établissement scolaire (Florian, 2026-11-11)
**Contexte** : D-047 confirme la compta et la paie dans ScolIA ; il faut en fixer l'ampleur.
**Décision** : Florian ne veut pas d'un logiciel généraliste (type Sage Compta ou Sage Paie). Le périmètre se limite aux besoins d'un établissement scolaire : une seule convention collective, ou presque, à gérer ; une comptabilité d'organisme scolaire (pas de comptabilité multisectorielle, pas de fonctions d'éditeur comptable généraliste).
**Conséquences** : les conceptions 48 et 49 retirent tout ce qui ne sert pas à un établissement (multi-conventions, paramétrage générique de toutes les branches, consolidation de groupe, fiscalité d'entreprise commerciale, etc.). Les obligations légales qui s'appliquent quand même restent dues : DSN conforme, bulletin conforme, FEC, piste d'audit, réception des factures électroniques. **Hypothèse à confirmer par Florian** : la convention à couvrir est celle de l'enseignement privé non lucratif (EPNL, IDCC 3218) pour les salariés de l'organisme de gestion ; les enseignants sous contrat, payés par l'État, ne sont pas dans la paie ScolIA.

## D-049 — Bureautique ArtCraft retenue sans coédition en temps réel (Florian, 2026-11-11)
**Contexte** : la conception 32 (Q-ET2) notait qu'ArtCraft ne permet pas à deux personnes de modifier un document en même temps.
**Décision** : Florian garde ArtCraft (D-006) et accepte l'absence de coédition en temps réel ; il ne l'a jamais demandée.
**Conséquences** : Q-ET2 est close. Le modèle de la conception 32 s'applique : verrou d'édition (un rédacteur à la fois, « demander la main »), versions immuables, travail de groupe par parties ou à tour de rôle. Aucune autre suite n'est à étudier pour ce motif.

## D-050 — Comptabilité : réception et lecture automatique des factures fournisseurs (OCR + IA), à la manière de Zeendoc (Florian, 2026-11-11)
**Contexte** : l'établissement paie aujourd'hui un abonnement Zeendoc (gestion électronique de documents, lecture des factures) en plus de sa comptabilité.
**Décision** : le socle 48 comprend un « cloud factures ». On y dépose des factures scannées ou des PDF reçus. ScolIA les stocke, les lit par OCR et IA, puis propose : le fournisseur (rapproché du fichier fournisseurs), le numéro, les dates de facture et d'échéance, les montants HT, TVA et TTC, ainsi qu'un classement et une imputation comptable. Un humain valide toujours avant qu'une écriture ou un paiement ne parte. Objectif : pouvoir résilier Zeendoc.
**Conséquences et cadrage** :
- Les factures électroniques structurées (Factur-X, UBL, CII), reçues par la plateforme agréée (réception obligatoire depuis le 1-9-2026), sont lues **directement**, sans OCR. L'OCR et l'IA servent aux papiers et aux PDF simples.
- Moteur d'OCR : brique libre non-AGPL exécutée sur nos serveurs (comme D-005), à choisir après vérification de licence. IA : Mistral prioritaire sous réserve de garanties contractuelles réelles, sans fallback externe non autorisé ; l'IA n'a jamais plus de droits que l'utilisateur.
- Les factures sont stockées dans ent-object (objets immuables, empreinte SHA-256) ; la piste d'audit fiable et l'archivage légal (D-047) s'appliquent ; le champ lu par la machine est distingué du champ validé par l'humain.
- Reprise du stock Zeendoc à prévoir (export de l'établissement, avec son accord).
**Mise en œuvre (2026-11-11, branche `ent-socle-48-49-revision`)** : D-048 → `48_COMPTABILITE.md` §36 et `49_PAIE.md` §35 (question Q-RH41 sur la convention unique) ; D-050 → `48_COMPTABILITE.md` §37 (lots CG20 à CG22) et `PERSONAS_48.md` §9.

## D-051 — Défauts du Next.js en production : on les laisse, la version Rust les corrige tous (Florian, 2026-11-11)
**Contexte** : `ALERTES_NEXTJS_PROD.md` recense 79 défauts du Next.js en production. Plusieurs viennent de contournements faute de comptes parents : des parents identifiés par nom, prénom et date de naissance (stages, invitations, rendez-vous), des liens publics, etc.
**Décision** : Florian laisse le Next.js en production tel quel jusqu'à la bascule. **Chaque défaut du registre doit être corrigé dans la version Rust.** Les modules sont portés pour leurs **actions métier seulement** : aucun contournement d'authentification n'est repris. En Rust, parents et élèves ont de vrais comptes (D-009), donc chaque action se fait depuis le compte connecté, avec les droits contrôlés côté serveur.
**Conséquences** : tout lot de code qui porte un module Next.js indique, dans son rapport, les défauts NJ-xx du registre qu'il ferme, avec un test de non-régression pour chacun (règle 14 de PILOTAGE). Le pilote le vérifie avant de pousser sur ENT. Ce que le pilote ne fait pas : rien sur le Next.js. Ce qui reste : alerter le DPO de l'établissement reste un choix de Florian.

## D-052 — Périmètre V1 : écoles, collèges et lycées généraux et technologiques seulement (Florian, 2026-11-11)
**Contexte** : la conception 51 (post-bac, apprentissage) et quelques passages d'autres socles couvrent la voie professionnelle, les CFA et l'apprentissage.
**Décision** : ScolIA vise d'abord les **écoles, collèges et lycées généraux et technologiques** « classiques ». La voie professionnelle (PFMP, lycées professionnels), les CFA, l'apprentissage et le post-bac (BTS, CPGE) sont **hors périmètre pour l'instant** : Florian ne connaît pas ces établissements.
**Conséquences** : le socle 51 est mis **en attente** (conception gardée, aucun lot de code). Dans les autres socles, les parties propres à la voie pro ou à l'apprentissage ne sont pas codées ; les stages de collège et de lycée général (observation de 3e, stage de seconde) restent dans le périmètre (socle 39). Le premier degré (socle 50) reste dans le périmètre. L'habilitation SOLTéA ne concerne pas ScolIA (elle reste une démarche de l'établissement s'il le souhaite).

## D-053 — Correction des failles critiques et élevées du Next.js en production (Florian, 2026-11-11 ; modifie D-051)
**Contexte** : le dépôt GitHub est **public** ; le registre `ALERTES_NEXTJS_PROD.md` y décrit les failles avec leurs fichiers. Des robots parcourent les dépôts publics. D-051 laissait le Next.js tel quel.
**Décision** : Florian autorise la correction des failles **critiques et élevées** (NJ- de gravité critique ou élevée, environ 57) **dans le Next.js en production**, par petits lots. **Autorisation explicite et permanente pour ce chantier** : le pilote relit chaque lot, le pousse sur `main` (déploiement automatique), puis vérifie le déploiement (`/api/health`, run « Deploy container »). Les failles moyennes et faibles restent pour la version Rust (D-051 inchangée pour elles ; et toutes restent à corriger en Rust).
**Garde-fous** : code seulement ; **aucune migration ni modification du schéma de la base**, aucun accès à la base ni aux secrets de production, aucune nouvelle dépendance sans accord du pilote ; aucune correction qui change ce que font les familles ou le personnel (fermeture d'une page publique, coupure d'une IA, nouveau parcours) sans accord de Florian, demandé par le pilote ; une session de correction ne pousse jamais sur `main` elle-même ; les autres interdictions de CLAUDE.md sur Next.js restent en vigueur hors de ce chantier. Mission : `docs/missions/MISSION_NEXTJS_CORRECTIFS.md`.

## D-054 — Correctifs Next.js recentrés sur ce qui s'attaque sans connexion (Florian, 2026-11-11 ; précise D-053)
**Contexte** : les comptes connectés du Next.js sont aujourd'hui quelques personnels et professeurs de confiance ; plusieurs modules signalés (infirmerie, bien-être) sont des essais vides ou inutilisés ; élèves et parents n'ont pas de compte.
**Décision** : le chantier D-053 ne traite plus que les failles **exploitables de l'extérieur, sans connexion** : pages et API publiques, liens à jeton, e-mails entrants, webhooks et tâches planifiées, fichiers servis sans contrôle, secrets de repli visibles dans le dépôt public. Les corrections « internes » restantes (tableau a–o du 2026-11-11 : rôles entre personnels connectés, IA du pilotage, module santé vide, bien-être inutilisé…) **ne sont pas poussées** sur le Next.js ; elles restent dues dans la version Rust (D-051). Les corrections internes déjà en production restent en place.

## D-055 — Réponses de Florian du 2026-11-11 (après-midi)
- **SOLTéA / taxe d'apprentissage** : hors sujet (D-052) ; rien à faire.
- **Zeendoc (précise D-050)** : pas de reprise ni d'export Zeendoc à organiser ; on construit un **équivalent intégré** à la comptabilité ScolIA (une GED de factures qui *est* la compta, sans connecteur, là où Zeendoc a besoin d'un connecteur vers Charlemagne Comptabilité). Module **facultatif** : aucun établissement n'est poussé à y tout déposer. Q-CG40, Q-CG41, Q-CG46 sans objet tant qu'un établissement ne demande pas de reprise.
- **IA de lecture des factures (Q-CG39)** : **Mistral** (API), sous les conditions habituelles (pas de fallback externe, droits de l'utilisateur).
- **Vision all-in-one (rappel)** : à terme, **tout** ce qui sert à un établissement scolaire est dans ScolIA, au même endroit ; on évite les connecteurs vers des logiciels tiers.
- **Paie (modifie D-048)** : plusieurs conventions et régimes, pas une seule : établissements privés (sous contrat d'association et sous contrat simple) **et** établissements publics. À étudier : qui paie qui dans le public (État, collectivités, EPLE employeur des AED/AESH/contractuels…), ce qui reste à gérer en interne (RH, paie des contrats de droit public recrutés par l'établissement), la paie des maîtres sous contrat simple. Reste « light » au sens de D-048 : seulement ce qui concerne un établissement scolaire.
- **Applis mobiles** : comptes Apple et Google plus tard (coût) ; Florian dispose de Macs.
- **Exports Charlemagne, inscription éditeur DNE** : plus tard.
- **`BETTER_AUTH_SECRET`** défini en production ; `PHOTOCOPIES_READY_TOKEN_SECRET` non défini (le jeton photocopies retombe sur `BETTER_AUTH_SECRET`, comportement inchangé).

## D-056 — Adresse des établissements et groupes scolaires (question ouverte, complète D-012)
**Contexte** : D-012 a fixé un sous-domaine par établissement (ex. `lpnb.scolia.fr`). Florian préférerait le **code UAI (ex-RNE)** pour ne pas inventer de diminutif, mais un **groupe scolaire** (plusieurs UAI au même endroit, qui travaillent ensemble) doit fonctionner comme un tout.
**Conception faite (2026-11-11)** : `docs/socles/ORGANISME_ADRESSES.md`. Recommandation : le locataire devient le groupe scolaire ; l'UAI est un objet rattaché ; `<uai>.scolia.fr` renvoie vers l'adresse canonique du groupe ; Q-O1 à Q-O7 à trancher. Cadrage initial : séparer l'**organisme** (groupe scolaire / organisme de gestion, l'unité de travail et de cloisonnement) des **établissements UAI** qu'il porte ; adresses possibles par UAI, renvoyant toutes vers l'organisme ; effet sur le cloisonnement (`etablissement_id`), les cookies `__Host-`, les passkeys (D-013), les applis mobiles, l'existant `lpnb.scolia.fr`.

## D-057 — Bascule Next.js → Rust en une fois aux vacances d'hiver 2027 (Florian, 2026-11-11 ; modifie le calendrier de TRANSITION_NEXT_RUST.md)
**Décision** : pas de bascule partielle ni de double système en production. Le Next.js est **remplacé d'un coup** pendant les **vacances d'hiver (sam. 20 février → lun. 8 mars 2027)**, quand le Rust couvre tout ce que la production utilise. But : une transition **indolore, voire invisible**, pour les utilisateurs actuels. Florian n'est pas pressé : si le Rust n'est pas prêt, on attend les vacances suivantes (printemps, 17 avril → 3 mai 2027).
**Noël** : pas de mise en production de ScolIA ; on s'en sert pour la démo (D-034) et une **répétition de bascule** hors production (données de démonstration ou export anonymisé fourni par Florian).
**Base de production** : l'interdiction de CLAUDE.md reste en vigueur jusque-là. **Au moment du passage**, Florian donnera explicitement l'autorisation de lire, transférer et modifier la base PostgreSQL de production, **après une sauvegarde complète vérifiée** (restauration testée) prise avant toute opération.
**Conséquences** : priorité du code = porter tous les modules Next.js réellement utilisés (inventaire T0) avant le 20 février, en plus du cœur V1 ; outil de reprise rejouable et testé sur la répétition de Noël ; critères « prêt pour bascule » à écrire (fonctions couvertes, reprise vérifiée, retour arrière possible).

## D-058 — Réservation de salles : toutes les salles, y compris de classe, libérées selon l'EDT réel (Florian, 2026-11-11 ; précise le socle 19)
**Décision** : la réservation de lieux (socle 19, lots AG2–AG3) porte sur **toutes les salles** de l'établissement (référentiel du socle 5), **salles de classe comprises**, et non sur une liste à part. Une salle de classe est réservable **quand l'EDT réel la laisse libre** : créneau sans cours, ou cours supprimé (professeur absent sans remplaçant, classe en sortie ou en stage, journée banalisée, examen ailleurs…). La disponibilité se calcule à partir des ports du socle 10 (séances, modifications, remplacements) et du socle 15 (absences des personnels), jamais par une copie.
**Règles** : le cours prime. Si un remplaçant est nommé ou si le cours est rétabli, la réservation tombe ; le demandeur est prévenu avec le motif et des salles de rechange sont proposées. Une salle de classe libérée n'est proposée qu'après la décision d'absence ou de suppression (pas sur simple soupçon). Le responsable des salles (réglage par salle : réservable toujours, sur validation, jamais) garde la main. Le professeur titulaire de la salle peut être prévenu. L'affichage dit pourquoi la salle est libre (« cours de 4e B supprimé »), sans révéler le motif d'absence du professeur (donnée personnelle).
**Conséquences** : le « salle de classe non réservable par défaut » du Next.js (19 §2.4) devient un réglage par salle, pas une exclusion ; la conception 19 est complétée (§ ajout D-058) ; tests de bout en bout absence → cours supprimé → salle proposée → remplaçant nommé → réservation retirée.

## D-059 — Tout-en-un : aucun service externe nécessaire ; agenda interne à calques (Florian, 2026-11-11)
**Principe (rappel renforcé de D-055)** : un établissement qui adopte ScolIA ne doit avoir **besoin d'aucun service externe** pour gérer l'établissement : tout est intégré de base. Un connecteur externe peut exister en option, jamais comme prérequis.
**Agenda (socle 19)** : un agenda interne « type Google Agenda » pour chaque personne remplace Google Agenda (utilisé aujourd'hui par la direction pour les rendez-vous d'inscription, qui deviennent des créneaux natifs, déjà prévu Q-AG5).
- **Calques à cocher** : chacun choisit ce qu'il voit et les superpose : mes cours (EDT réel du socle 10, pour tout personnel qui a des séances à l'EDT, direction comprise ; aucune règle d'obligation à coder : le calque apparaît dès qu'une personne a des cours, quel que soit son rôle) ; mes rendez-vous ; mes réunions et convocations ; la **semaine de l'établissement** (la « feuille de semaine » de la direction : événements, sorties, examens, réunions, absences de classes, fermetures) ; agendas partagés (équipe, salle, service) ; échéances (conseils, bulletins, voyages).
- **Feuille de semaine** : la direction la compose dans ScolIA (le module Next.js « week-sheet » est repris), elle est publiée aux publics choisis, et elle apparaît comme calque dans l'agenda de chacun.
- Pas de double saisie : un cours, un voyage, un examen, un conseil apparaissent d'eux-mêmes (vue calculée par ports, AG1), jamais recopiés.

## D-060 — Audits approfondis par le modèle le plus puissant (Florian, 2026-11-11) — **suspendue le 2026-11-12** (quota : plus de session Fable sans accord de Florian, PILOTAGE règle 20)
Des revues complètes de sécurité et de qualité (failles, cloisonnement, droits, simplification, performance, refactorisation) sont confiées au modèle le plus puissant disponible (Fable), en lecture seule, avec liste classée de constats puis corrections par les sessions de code : **fin novembre 2026** (cœur sur ENT), **mi-février 2027** (tout le code, avant la bascule D-057), puis **quelques mois après la bascule**. **Complément (Florian, 2026-11-11, après l'audit n° 1)** : en plus de ces trois revues, un **audit Fable ciblé après chaque gros bloc versé sur ENT** (messagerie, emploi du temps, finances, etc.), calé juste après une remise à zéro des quotas ; chaque audit liste aussi ce qu'il n'a pas couvert, repris par le suivant. Elles s'ajoutent aux revues ciblées par lot et ne remplacent ni un test d'intrusion externe (avant ouverture à d'autres établissements) ni l'avis d'un juriste ou d'un DPO. Calées de préférence juste après une remise à zéro des quotas.

## D-061 — Socle N1 « Demandes » : complet dès le départ, familles comprises ; HSE en heures, validées par la direction (Florian, 2026-11-13)
**Familles et élèves (réponse à Q-DM2 et Q-DM11)** : la page « demande des parents » du Next.js n'a **jamais été ouverte** aux familles, elle n'est pas utilisée. Le Rust ne s'aligne donc pas sur elle : on construit **le module complet tout de suite**, natures familles et élèves comprises, comme s'il était en production, sans version réduite à rouvrir plus tard. La porte unique « Écrire à l'établissement » (I-008, `N1_DEMANDES.md` §37) en fait partie. Ce qui ne sert pas encore reste désactivable par établissement (réglage), mais existe et est testé.
**HSE (réponse à Q-DM6 et Q-DM10)** :
- l'enveloppe et toutes les saisies internes se comptent **en heures**, jamais en euros. Le calcul en euros, selon le type d'heure, est fait par l'application académique de déclaration, pas par ScolIA (nom de l'application à vérifier [NV] ; export en heures par type d'heure) ;
- circuit : le professeur demande des HSE (heures **déjà faites ou prévues**), la **direction valide**, et c'est tout. **Pas d'attestation séparée du service fait** : l'acceptation par la direction vaut confiance. Le circuit « autorisation → service fait attesté » de la conception est simplifié en conséquence (une seule étape de validation ; l'attestation devient au mieux un réglage facultatif, désactivé par défaut).
**File de repli quand la personne visée est dans la file RH (réponse à Q-DM14, précisée le même jour)** : **réglage par établissement**, explicité en clair à l'écran (« quand une demande vise un membre du service RH, elle lui est masquée et part à : … »). On choisit une ou plusieurs personnes ou rôles (attaché de gestion, chef d'établissement, adjoint, président de l'OGEC, autre). **Défaut : attaché de gestion et direction.** Choisir une personne extérieure à la vie interne (OGEC) affiche un avertissement (elle connaît moins le contexte et verra des informations RH). Garde-fous non réglables : la personne visée n'est **jamais** destinataire ; au moins un destinataire, sinon le réglage est refusé ; si un destinataire du repli est lui-même visé, la demande passe au suivant puis à la direction ; tout changement du réglage est journalisé ; le demandeur voit en mots simples à qui part sa demande.
**DPO (Q-DM1, Q-DM4)** : pas de DPO identifié à ce jour (constat de Florian). Les questions qui en dépendent restent ouvertes ; recommandation du pilote : désigner ou identifier un DPO avant la bascule, puisque la bascule transfère des données réelles (condition (d) du plan de bascule).
**Conséquences** : la recommandation « internes seulement en février » de `N1_DEMANDES.md` (Q-DM2) est **remplacée** ; les lots N1-0 à N1-5 incluent les natures familles et élèves ; N1-3 (HSE) est simplifié.

## D-062 — Dépendances : seulement ce qu'on ne peut pas coder à la main (Florian, 2026-11-14)
**Principe** : une dépendance externe n'entre dans l'ENT que si elle est indispensable **et** qu'on ne peut pas raisonnablement la coder à la main. Tout ce qui se code à la main se code à la main (moins de surface d'attaque, de licences à suivre et de mises à jour subies).
**Exception de sécurité** : la cryptographie (primitives, signature, chiffrement) ne se code **jamais** à la main ; on utilise des briques reconnues, de préférence celles **déjà présentes** dans le projet (`ring`, `rsa`, `der`, `sha2`, `webauthn-rs`). Le cas échéant, une petite brique de la même famille (RustCrypto), présentée à Florian avant ajout, avec licence vérifiée à la source.
**Application au socle 7, lot D1 (essai technique)** :
- génération de PDF : **moteur maison** (`domain-eleve/src/bulletin_pdf.rs` existe déjà, sans dépendance), étendu : accents (encodage WinAnsi), plusieurs pages, tableaux, logo JPEG inséré tel quel, numéros de page ; **Typst écarté** ;
- PDF/A : police libre embarquée en entier (sans découpage) par le moteur maison, en second temps ;
- QR code de vérification : maison ;
- client ClamAV (protocole INSTREAM) : maison ; ClamAV tourne comme service séparé ;
- cachet PAdES : structure CMS assemblée à la main avec `der`, signature par `rsa`/`ring` déjà présents ; validation par un validateur tiers en essai ;
- rendu de PDF en image (PDFium) : **non retenu pour la V1** (navigateurs et téléphones affichent les PDF) ; à reconsidérer seulement sur besoin réel.
**Défaut constaté** : le moteur maison actuel n'écrit que de l'ASCII (les caractères hors ASCII deviennent « ? », « œ » devient « o ») ; correction prioritaire (bulletins, factures, courriers) par l'encodage WinAnsi des polices standard.
**Conséquences** : la ligne D1 de `07_DOCUMENTS.md` est remplacée par cette décision ; toute nouvelle dépendance proposée par une session doit expliquer pourquoi elle ne peut pas être codée à la main.
