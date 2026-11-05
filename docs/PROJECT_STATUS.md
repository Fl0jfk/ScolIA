# État du projet — à maintenir sur faits vérifiés

Date : 2026-11-05 (mise à jour du 2026-11-04). Branche : `ENT` (D-001).
Sources : audit en lecture seule du 2026-11-02 et prototype Leptos (`docs/LEPTOS_CALL_PROTOTYPE.md`).

## Socle 2 — S2-1 dossier élève et personnes (2026-11-05)
Personne distincte du compte (`ent_personne`, reprise idempotente des liens), liens d'autorité (autorité, garde, dates, suspension) vérifiés en un point unique (vue `ent_eleve_lien_actif`), contacts d'urgence, payeurs, fusion réversible des personnes, 21 routes `/dossier/*`, fiche élève 360° Leptos `/web/eleves/{id}` avec chronologie. **Implémenté + testé en local** (branche `ent-socle2-dossier`, inclut S2-0), non validé pilote, non fusionné. Détail, écarts et reste à faire : `docs/SOCLE2_DOMAINES.md` §6.2.

## Socle 2 — S2-0 outbox transactionnelle (2026-11-05)
Outbox (`migrations/102_outbox.sql`, `ent-db/src/outbox.rs` : `tx.publier`, distributeur tokio, consommateurs idempotents, reprises, file des échecs, rejeu) et 3 réactions D5 → D0 (absence constatée, décision de justificatif, retard → notification des responsables `peut_voir_vs`) ; `clore_appel` et les décisions de justificatif publient dans la transaction du fait. **Implémenté + testé en local** (branche `ent-socle2-outbox`), non validé pilote, non fusionné. Détail, écarts et plan de migration des autres publications : `docs/SOCLE2_DOMAINES.md` §6.1.

## Socle 2 — S2-2 référentiel complet (2026-11-05)
Sites, niveaux/MEF (nomenclature nationale, import SIECLE/BCN), périodes, calendrier, grille M1…S4, séances datées générées, `classe_id` dans 9 tables (double écriture par triggers, rattachement idempotent), bascule d'année (préparer, reconduire, monter, ouvrir, clôturer), droits par le moteur (`referentiel.lire/gerer`), ~30 routes `/referentiel/*`, événements via l'outbox. **Implémenté + testé en local** (`ent-socle2-referentiel`, migrations 110-112), non validé pilote ; écrans Leptos non faits. Détail : `docs/SOCLE2_DOMAINES.md` §6.2.

## Socle 1 — écrans de sécurité du compte (2026-11-05)
Frontend Leptos : enrôlement obligatoire d'une passkey (`/web/securite/enrolement`), « Sécurité du compte » (`/web/securite`), politique de sécurité de l'établissement (`/web/direction/securite`). **Implémenté + testé en local** (branche `ent-socle1-ecrans-securite`), non validé pilote, non fusionné. Backend non modifié. Détail, limites et divergence `ent-types::MeDto` : `docs/SOCLE1_SECURITE.md` (« Écrans frontend ») et `docs/FRONTEND_CIBLE.md` § 8.

## Vague produit 1 (2026-11-04 → 05)

**Niveau atteint : implémenté + testé en local.** Non validé pilote, non prêt production. Commité et poussé sur `ENT` dans la foulée ; SHA à compléter par le coordinateur. CI GitHub sur ces commits : non vérifiée.

**Pilotage (depuis le 2026-11-04)** : priorité à la valeur fonctionnelle livrée ; les points de sécurité restent des bloquants réels. Cartographie fonctionnelle exploitable : `docs/CARTOGRAPHIE_ENT.md` (16 domaines, propriété des données, ordre de réalisation).

**Faits vérifiés (implémenté, testé en local)**
- L3 sécurité des comptes (terminé ; la mention « L3 non traité » de la vague 1 ci-dessous est antérieure) :
  - changement d'e-mail hiérarchisé : Administratif ne touche ni Admin, ni Direction, ni Administratif ; soi-même avec mot de passe actuel ;
  - réinitialisation d'un parent ou d'un élève par lien sans secret (202 ; 503 si l'envoi est impossible) ;
  - reset des passkeys par la direction, avec hiérarchie et acteur réel au journal ;
  - ajout ou suppression de passkey : mot de passe actuel exigé, plafond 10, nom ≤ 64 caractères ;
  - rattachement d'un responsable sans mot de passe, avec lien d'activation ;
  - jetons en attente invalidés ; ré-authentification sans toucher au compteur de connexion ;
  - refus si la cible appartient à un autre établissement.
  - Matrice de séparation (`ent-rust/crates/ent-api/tests/separation.rs`) : 30 tests verts (27 à la vague 1), plus aucun cas ignoré.
- Référentiel scolaire V1 (additif, migration 092) : années scolaires, classes (niveau en champ), groupes, affectations, services d'enseignement. Amorçage idempotent depuis les colonnes texte `classe` existantes ; changement de classe d'un élève synchronisé ; `enseigne_a(prof, élève, date)` exposée et testée mais non branchée sur les droits (décision de Florian) ; routes `/referentiel/*` ; écrans `/web/classes`, `/web/classes/:id`, `/web/mes-classes` ; feuille d'appel enrichie (libellé et effectif de la classe).
- Module Vie scolaire (Leptos) : CPE `/web/vie-scolaire` (tableau du jour, liste filtrable, détail, accepter / refuser / annuler avec confirmation, signaux d'absence répétée) ; famille et élève `/web/absences` (justification selon les droits du lien ; lecture seule pour l'élève) ; tuiles d'accueil.
- Module Messagerie (Leptos) : `/web/messages` (boîte, filtres, recherche) ; fil (lecture, réponse, pièces jointes, archivage) ; nouveau message avec confirmation (personnel → à propos d'un élève ; famille → école) ; compteur de non lus dans la navigation ; tuile d'accueil vers `/web/messages` (plus vers l'ancienne UI).

**Limites connues**
- Vie scolaire : pas de pièce jointe aux justificatifs ; liste serveur = absences à traiter du jour ; pas de liste des retards du jour ; classe et séance absentes du DTO ; un élève peut justifier ses propres absences via l'API.
- Messagerie : rôle de l'auteur non renvoyé par l'API ; boîte plafonnée à 50 fils ; pas de désarchivage.
- Référentiel : `enseigne_a` non branché sur les droits.
- Passkey : le test d'enrôlement du prototype `ent-web-session` (gelé) échoue depuis L3 (il n'envoie pas le mot de passe actuel) ; correctif de 2 lignes en attente d'autorisation.
- Compte global des suites de tests de cette vague : non relevé dans cette mise à jour.

**Décisions à soumettre (non décidées)**
1. Référentiel source unique des classes, avec `classe_id` à côté du texte.
2. Séparer personne et compte pour les responsables.
3. Règle de propriété des données et découpage progressif de `PgEnt`.
4. Événements typés, outbox et domaine Notifications unifié.
5. Périmètre du pilote et date de retrait de l'ancienne UI.

**En attente** : brancher la règle « un professeur ne voit que ses classes » (notes, absences, fils) sur `enseigne_a`.

| Domaine | État observé | Preuve (chemin) | Tests | Risques/questions |
|---|---|---|---|---|
| Backend Rust | **Implémenté, non déployé** : axum 0.8 + sqlx, ~268 routes, migrations jusqu'à 092 (référentiel scolaire V1), nombreux domaines (élève, absences/appel, notes, EDT, VS, facturation…) | `ent-rust/crates/ent-api`, `ent-db`, `domain-eleve`, `ent-rust/migrations/` | 2026-11-03 : `cargo test --workspace` (14 crates : domain_eleve, ent_api, ent_auth, ent_bank, ent_brain, ent_db, ent_events, ent_mail, ent_object, ent_obs, ent_oidc, ent_push, ent_sms, ent_types) en local contre la base de test `ent_proto_test` avec `ENT_ENV=lab` : 276 passés, 0 échec, 0 ignoré ; relance après les correctifs du 2026-11-03 (verrouillage de la clôture d'appel, +3 tests), dans les conditions du job CI backend (`ENT_ENV=test`, sans dérogation) : **279 passés, 0 échec, 0 ignoré** ; le vert GitHub date du 1er run CI, antérieur aux commits locaux (la CI n’a pas encore tourné sur eux ; deux blocages CI corrigés le 2026-11-03, non poussés) ; **2026-11-04** (base de test locale, conditions CI `ENT_ENV=test`) : `cargo test --workspace --features ent-api/labo,ent-lab/labo` **344 réussis, 0 échec** ; sans `labo`, sous-ensemble `ent-api` + `ent-db` + `ent-object` (les tests qui exigent le labo y sont exclus par construction) : 51 réussis. Le total complet comparable aux 279 de la veille est celui avec `labo` (344 = 279 + tests de la vague 1) | `Cargo.lock` désormais versionné ; erreurs classées par mots du message (`map_db`) ; dette de formatage et Clippy ; vague produit 1 : matrice de séparation 30 tests verts, compte global des suites non relevé dans cette mise à jour |
| Frontend Rust | **Tranche 1 implémentée et testée en local (non validée pilote, non poussée)** : parcours enseignant Leptos 0.8 (connexion mot de passe + passkey, coquille, accueil Bento, emploi du temps semaine, appel complet, mobile) ; `ent-web-app` en `noyau/` + `composants_metier/` + `modules/` ; `ent-web-api` un service par domaine ; `ent-web-ui` enrichi ; types `ent-types` lot 1 ; ancienne UI JavaScript (`ui.rs`, `/app`) toujours en service, figée. **Vague produit 1 (2026-11-04 → 05), implémentée et testée en local** : Vie scolaire (`/web/vie-scolaire`, `/web/absences`), messagerie (`/web/messages`), classes (`/web/classes`, `/web/mes-classes`) ; l'accueil renvoie vers `/web/messages`. Détail : sections « Vague produit 1 » et « Tranche 1 » ci-dessous | `ent-rust/web/`, `ent-rust/crates/ent-types`, `docs/FRONTEND_CIBLE.md`, `docs/DESIGN_SYSTEM_LEPTOS.md` | relance du 2026-11-03 (après correctifs passkey/déconnexion) : ent-web-ui 14, ent-web-api 5, ent-web-app 36, ent-web-server 24 (lib 2 + contre_backend 8 + droits_parite 12 + entetes_securite 2), ent-web-session 9 ; clippy 0 avertissement (ssr, wasm32) ; essai navigateur intégré (ordinateur, 375 px, 400 élèves) ; **2026-11-04** : web 89 réussis, WASM OK, clippy 0 erreur, ancienne interface vérifiée au navigateur sans régression bloquante | lecteur d'écran réel non testé ; passkey non testée au navigateur ; limites listées ci-dessous ; vague produit 1 : compte de tests non relevé dans cette mise à jour |
| Auth | **Implémenté** : Argon2id, sessions Postgres (jeton opaque Bearer), passkeys `webauthn-rs` (facultatives), SSO OIDC | `ent-rust/crates/ent-auth`, `ent-oidc`, `ent-api/src/auth.rs` | tests unitaires + parcours connexion/401/403 ; ent-api 30 tests, ent-db 45 (relance du 2026-11-03) | **A6 (2026-11-05) : le frontend Leptos n'a plus de jeton en `localStorage`** — cookie `HttpOnly` posé par le relais `ent-web-session` intégré à `ent-web-server` (la base applique inactivité 2 h / maximum 12 h, cookie ré-émis après enrôlement passkey, reconnexion rapide), testé en local, **essai navigateur réel à faire** (`SESSION_WEB_COOKIE_ETUDE.md` §15) ; l'ancienne UI `/app` (figée) garde son jeton en `localStorage` ; passkey obligatoire pour tous à construire (D-009) ; vague 1 (2026-11-04) : rattachement d'un parent sans mot de passe public (secret aléatoire) ; L3 traité en vague produit 1 (2026-11-05) : rattachement sans mot de passe, reset des passkeys par la direction, hiérarchie des e-mails, voir « Vague produit 1 » |
| PostgreSQL/migrations | **Implémenté** : migrations embarquées, appliquées au démarrage ; refus de tout hôte non local | `ent-db/src/migrate.rs`, `url_guard.rs` | migrations appliquées sur une base locale vierge (PostgreSQL 18.4) par le prototype | pas de RLS ; levée du verrou « local uniquement » à prévoir pour un déploiement |
| Stockage fichiers | **Partiel** : disque local réel ; mode « S3 » sans TLS, sans signature, sans URL pré-signée | `ent-rust/crates/ent-object` | vague 1 (2026-11-04, local) : clés de stockage jamais fournies par le client, contrôle d'appartenance au document et à l'établissement, `assert_safe_key` ; couvert par la suite backend du 2026-11-04 | non prêt pour Scaleway Object Storage (URL pré-signées toujours absentes) |
| Email/SMS | **Partiel** : SMTP réel opt-in (`lettre`), SMS/push/banque en mode simulé ; fournisseur SMS décidé (OVH, D-004) non intégré | `ent-mail`, `ent-sms`, `ent-push`, `ent-bank` | non testé | connecteur OVH à écrire |
| Android/iOS | **Prototype non compilable** : 4 applis (famille/élève × iOS/Android), sans fichiers de build ; une seule appli par plateforme décidée (D-003) | `ent-rust/apps/` | aucun | fusion des 4 prototypes, projets Gradle/Xcode à créer |
| IA/conversion | **Absent côté Rust** : copilote à règles fixes, sans modèle de langage ; conversion de fichiers décidée (D-005) non commencée | `ent-db/src/copilote.rs`, `ent-brain` | — | choix du fournisseur LLM (Mistral à étudier) |
| CI/tests | **Workflow `ent-rust.yml`** (D-015) : 1er run GitHub : formatage, audits, backend verts ; frontend en échec (F-003), **corrigé localement, non poussé** ; lockfiles versionnés ; audits `cargo deny` + `cargo audit` | `.github/workflows/ent-rust.yml`, `docs/CI_REPRODUCTIBILITE.md` ; contrôle « binaire de production sans labo » ajouté en CI et dans le Dockerfile (vague 1, non poussé) | backend : 279 tests verts en local le 2026-11-03 en conditions CI (voir ligne Backend) ; la CI GitHub elle-même n'a pas tourné sur ces changements ; frontend : ent-web-ui 14, ent-web-api 5, ent-web-app 36, ent-web-server 24, ent-web-session 9 ; ent-types : 4 tests | dette : 706 écarts de formatage (procédure prête, `FORMATAGE_BACKEND.md`) et 60 avertissements Clippy (plan : `CLIPPY_PLAN.md`) |

## Vague 1 du socle de confiance — tranche 2 (2026-11-04)

**Niveau atteint : implémenté + testé en local.** Non commitée à la date de rédaction ; commit local prévu sur `ENT`, non poussé. Non validé pilote, non prêt production. Détail sécurité, constats et limites : `SECURITY.md`, section « Vague 1 ».

**Faits vérifiés (implémenté)**
- L1 fichiers (faille critique corrigée) : clés de stockage jamais fournies par le client ; lecture seulement si la clé appartient au document et à son établissement ; `assert_safe_key` dans `ent-object`.
- L2 annuaire : `/eleves/search` et `/eleves/labels` réservés au personnel ; identité par niveau (Complet, Sante, Reduit) appliquée en un point (`PgEnt::niveau_identite`) ; recherche par badge réservée au niveau Complet ; foyers réservés à une liste de rôles.
- L4 labo hors production : feature Cargo `labo` désactivée par défaut ; sans elle, `/labo/bootstrap` et `/auth/register` répondent 404 ; binaire de production contrôlé en CI et dans le Dockerfile. Lancement local : `cargo leptos build --bin-features labo` ; tests avec `--features labo` ou `--features ent-api/labo`. Écran de provisionnement du personnel via `POST /etablissement/personnel`.
- L6 erreurs et journaux : erreur interne générique avec référence de requête (détail au journal seulement) ; `Cache-Control: no-store` sur l'API et `/web` ; traçabilité des consultations santé et psy, des téléchargements et des exports réglementaires ; journal `ent_journal_modif` en ajout seul (migration 091).
- L8 matrice de séparation : `ent-rust/crates/ent-api/tests/separation.rs`, 27 tests, plus de 1 700 cas.

**Validation (2026-11-04, base de test locale, conditions CI `ENT_ENV=test`)** : backend 344 réussis avec `labo` (suite complète), 51 sans `labo` (sous-ensemble ent-api + ent-db + ent-object) ; web 89 réussis ; WASM OK ; clippy 0 erreur ; ancienne interface vérifiée au navigateur sans régression bloquante ; revues Sonnet : 1 bloquant corrigé, aucun bloquant restant. Non fait : CI GitHub sur ces changements, test pilote.

**Limites restantes** : L3 traité depuis (voir « Vague produit 1 ») ; journal (moyen) ; empreinte e-mail non salée et autres points faibles ; jeton de session en `localStorage` **seulement pour l'ancienne UI `/app`** (figée, sans CSP) — le frontend Leptos n'en a plus (A6) ; importants avant pilote. Liste complète : `SECURITY.md`.

**Lots non couverts par cette vague** : L3 (non traité à la date de la vague ; traité depuis, voir « Vague produit 1 »). Les lots L5 (anti-abus), L7 (session cookie), L9 (configuration de production), L10 (exploitation) et L11 (SSO) ne sont pas commencés : prévus en vagues 2 et 3.

**Questions ouvertes (décisions en attente)**
1. Rôle de base PostgreSQL séparé pour les migrations (fin du contournement du trigger du journal).
2. Durée de conservation du journal.
3. HMAC avec secret serveur pour l'empreinte des e-mails (aujourd'hui SHA-256 non salé).
4. Besoins réels de la Compta et du CPE en régime et en badge.

Incidents d'intégration de cette vague : `DECISIONS_AND_FAILURES.md`, F-006.

## Tranche 1 — frontend enseignant (2026-11-03)

**Niveau atteint : implémenté + testé en local.** Non validé pilote, non prêt production, 4 commits locaux sur `ENT` (1b5b1b47 fix backend, 8a41bfba feat ent-web tranche 1, 0e6ee900 docs, 7aa66701 agents), non poussés.

**Faits vérifiés**
- Parcours : connexion (mot de passe + passkey via l'API WebAuthn du navigateur, `modules/connexion/webauthn.rs`), coquille (navigation, fil d'Ariane), accueil Bento (prochain cours, séances du jour, tuiles messages non lus et annonces), emploi du temps semaine, appel (présent/absent/retard, clavier, sélection multiple, confirmation, clôture, verrouillage après clôture), responsive mobile.
- Structure : `ent-web-app` = `noyau/` (session, droits, navigation, coquille, erreurs, temps), `composants_metier/`, `modules/` (connexion, accueil, edt, appel) ; `ent-web-api` = un service par domaine (auth, edt, appel, communication) ; `ent-web-ui` : avatar, coquille, icône, mise en page, segmenté, bouton Sombre ; feuille `ent-rust/web/style/ent-web.css` sur les jetons de `ui.css`.
- Décision de Florian (2026-11-03) : pour la tranche 1, messages et annonces = tuiles d'accueil seulement ; messagerie complète = module ultérieur. Le lien « Ouvrir la messagerie » mène encore à l'ancienne UI JS `/app`.
- Tests : voir la ligne « Frontend Rust » ; exécutés contre la base de TEST locale `ent_proto_test`, aucun test sauté.

**Corrections de sécurité faites pendant la session** (détail : `SECURITY.md`) : porte du labo / inscription libre fail-closed ; contrôle d'accès horizontal sur l'appel ; CSP stricte à nonce sur `/web` et `/pkg` ; gardes « base locale » des tests et du seed durcies ; durcissement du frontend (adresse de retour, synchro d'onglets, 401/403/429, `?semaine=`, `Debug`, messages bornés).

**Limites restantes**

| Gravité | Limite |
|---|---|
| Importante (avant pilote) | Jeton de session en `localStorage` **limité à l'ancienne UI `/app`** (frontend Leptos : cookie `HttpOnly`, A6, 2026-11-05) ; essai du cookie et de la reconnexion rapide dans un vrai navigateur à faire |
| Importante | `/app` (ancienne UI JS) partage l'origine et le jeton, sans CSP |
| Moyenne | Commentaire et statut « dispensé » non gérés par le backend |
| Moyenne | Consultation d'une feuille d'appel = `POST /appels` (écriture) |
| Moyenne | Clôture d'appel : les événements et actions Brain (absences, retards) sont publiés après le commit ; s'ils échouent, l'appel reste clos sans absences publiées et sans relance possible (outbox ou réconciliation à prévoir) |
| Faible | Aucun `lock_timeout` sur les verrous de l'appel (pointage / clôture) |
| Faible | Second facteur passkey : si l'e-mail est modifié pendant la signature, c'est le compte initial (dont le mot de passe a été saisi) qui s'ouvre ; le délai maximal de 5 s de la déconnexion n'a pas été exercé en navigateur réel (le serveur arrêté était détecté en ~2 s) ; il est couvert par un test unitaire |
| Moyenne | `config_connexion` expose `ENT_WEB_ETABLISSEMENT_ID` (à revoir avec D-012) |
| Moyenne | Passkey : parcours WebAuthn complet non testé au navigateur (pas d'authentificateur) ; couvert côté `ent-web-session` par un authentificateur logiciel ; erreurs WebAuthn regroupées |
| Faible | Date d'accueil calculée côté serveur puis client (décalage possible autour de minuit) |
| Faible | Touche Espace sur la case de sélection à tester sur Firefox / Safari |
| Faible | L'EDT n'indique pas les appels clos |
| Faible | Barre d'action mobile haute (~140 px) |
| Faible | Thème sombre incomplet (jetons de statut non redéfinis) |
| Faible | Backend non formaté globalement (`FORMATAGE_BACKEND.md` non exécutée) |
| Faible | `PageConnexion` lit encore une `Resource` serveur (choix conservé pour le rendu serveur du nom d'établissement) |

**Recommandations (non décidées)**
1. Bascule vers la session par cookie HttpOnly (`SESSION_WEB_COOKIE_ETUDE.md`).
2. CSP pour `/app`, ou retrait progressif de l'ancienne UI.
3. Commentaire / dispensé côté backend (à autoriser : touche `ent-db`).
4. Tranche 2 : périmètre à définir avec Florian.
5. Exécuter la procédure de formatage du backend.

**Questions ouvertes** : (a) `config_connexion` expose l'identifiant d'établissement : à trancher avec D-012 (éventuelle mise à jour de `DECISIONS.md`, non faite ici) ; (b) faut-il autoriser une route en lecture pour consulter un appel sans `POST` ?

Pour chaque domaine fonctionnel, préciser absent / prototype / partiel / implémenté / testé / validé / prêt pilote. Ne jamais remplir par supposition. Next.js prod reste intouchable ; DB prod inaccessible.
