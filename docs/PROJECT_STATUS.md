# État du projet — à maintenir sur faits vérifiés

Date : 2026-11-04 (mise à jour du 2026-11-03). Branche : `ENT` (D-001).
Sources : audit en lecture seule du 2026-11-02 et prototype Leptos (`docs/LEPTOS_CALL_PROTOTYPE.md`).

| Domaine | État observé | Preuve (chemin) | Tests | Risques/questions |
|---|---|---|---|---|
| Backend Rust | **Implémenté, non déployé** : axum 0.8 + sqlx, ~268 routes, 90 migrations, nombreux domaines (élève, absences/appel, notes, EDT, VS, facturation…) | `ent-rust/crates/ent-api`, `ent-db`, `domain-eleve`, `ent-rust/migrations/` | 2026-11-03 : `cargo test --workspace` (14 crates : domain_eleve, ent_api, ent_auth, ent_bank, ent_brain, ent_db, ent_events, ent_mail, ent_object, ent_obs, ent_oidc, ent_push, ent_sms, ent_types) en local contre la base de test `ent_proto_test` avec `ENT_ENV=lab` : 276 passés, 0 échec, 0 ignoré ; relance après les correctifs du 2026-11-03 (verrouillage de la clôture d'appel, +3 tests), dans les conditions du job CI backend (`ENT_ENV=test`, sans dérogation) : **279 passés, 0 échec, 0 ignoré** ; le vert GitHub date du 1er run CI, antérieur aux commits locaux (la CI n’a pas encore tourné sur eux ; deux blocages CI corrigés le 2026-11-03, non poussés) ; **2026-11-04** (base de test locale, conditions CI `ENT_ENV=test`) : `cargo test --workspace --features ent-api/labo,ent-lab/labo` **344 réussis, 0 échec** ; sans `labo`, sous-ensemble `ent-api` + `ent-db` + `ent-object` (les tests qui exigent le labo y sont exclus par construction) : 51 réussis. Le total complet comparable aux 279 de la veille est celui avec `labo` (344 = 279 + tests de la vague 1) | `Cargo.lock` désormais versionné ; erreurs classées par mots du message (`map_db`) ; dette de formatage et Clippy |
| Frontend Rust | **Tranche 1 implémentée et testée en local (non validée pilote, non poussée)** : parcours enseignant Leptos 0.8 (connexion mot de passe + passkey, coquille, accueil Bento, emploi du temps semaine, appel complet, mobile) ; `ent-web-app` en `noyau/` + `composants_metier/` + `modules/` ; `ent-web-api` un service par domaine ; `ent-web-ui` enrichi ; types `ent-types` lot 1 ; ancienne UI JavaScript (`ui.rs`, `/app`) toujours en service, figée. Détail : section « Tranche 1 » ci-dessous | `ent-rust/web/`, `ent-rust/crates/ent-types`, `docs/FRONTEND_CIBLE.md`, `docs/DESIGN_SYSTEM_LEPTOS.md` | relance du 2026-11-03 (après correctifs passkey/déconnexion) : ent-web-ui 14, ent-web-api 5, ent-web-app 36, ent-web-server 24 (lib 2 + contre_backend 8 + droits_parite 12 + entetes_securite 2), ent-web-session 9 ; clippy 0 avertissement (ssr, wasm32) ; essai navigateur intégré (ordinateur, 375 px, 400 élèves) ; **2026-11-04** : web 89 réussis, WASM OK, clippy 0 erreur, ancienne interface vérifiée au navigateur sans régression bloquante | lecteur d'écran réel non testé ; passkey non testée au navigateur ; limites listées ci-dessous |
| Auth | **Implémenté** : Argon2id, sessions Postgres (jeton opaque Bearer), passkeys `webauthn-rs` (facultatives), SSO OIDC | `ent-rust/crates/ent-auth`, `ent-oidc`, `ent-api/src/auth.rs` | tests unitaires + parcours connexion/401/403 ; ent-api 30 tests, ent-db 45 (relance du 2026-11-03) | jeton en `localStorage` côté web (risque XSS atténué par la CSP du 2026-11-03) ; **prototype isolé de cookie HttpOnly testé** (`ent-web-session`, 9 tests + essai Chrome, `SESSION_WEB_COOKIE_ETUDE.md` §14), production non décidée ; passkey obligatoire pour tous à construire (D-009) ; vague 1 (2026-11-04) : rattachement d'un parent sans mot de passe public (secret aléatoire), champ `password` du rattachement et réinitialisation par la direction non traités (L3, risque élevé) |
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

**Limites restantes** : L3 non traité (élevé) ; journal (moyen) ; empreinte e-mail non salée et autres points faibles ; jeton de session en `localStorage` et `/app` sans CSP, inchangés (importants avant pilote). Liste complète : `SECURITY.md`.

**Lots non couverts par cette vague** : L3 (non traité). Les lots L5 (anti-abus), L7 (session cookie), L9 (configuration de production), L10 (exploitation) et L11 (SSO) ne sont pas commencés : prévus en vagues 2 et 3.

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
| Importante (avant pilote) | Jeton de session en `localStorage` (CSP en atténuation) ; bascule vers le cookie HttpOnly du prototype `ent-web-session` à planifier |
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
