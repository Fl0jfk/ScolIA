# État du projet — à maintenir sur faits vérifiés

Date : 2026-11-03. Branche : `ENT` (D-001).
Sources : audit en lecture seule du 2026-11-02 et prototype Leptos (`docs/LEPTOS_CALL_PROTOTYPE.md`).

| Domaine | État observé | Preuve (chemin) | Tests | Risques/questions |
|---|---|---|---|---|
| Backend Rust | **Implémenté, non déployé** : axum 0.8 + sqlx, ~268 routes, 90 migrations, nombreux domaines (élève, absences/appel, notes, EDT, VS, facturation…) | `ent-rust/crates/ent-api`, `ent-db`, `domain-eleve`, `ent-rust/migrations/` | 274 tests verts en local (profil allégé, base locale) ; **verts sur GitHub** (1er run CI, 2026-11-03) | `Cargo.lock` désormais versionné ; erreurs classées par mots du message (`map_db`) ; dette de formatage et Clippy |
| Frontend Rust | **Tranche 1 implémentée et testée en local (non validée pilote, non poussée)** : parcours enseignant Leptos 0.8 (connexion mot de passe + passkey, coquille, accueil Bento, emploi du temps semaine, appel complet, mobile) ; `ent-web-app` en `noyau/` + `composants_metier/` + `modules/` ; `ent-web-api` un service par domaine ; `ent-web-ui` enrichi ; types `ent-types` lot 1 ; ancienne UI JavaScript (`ui.rs`, `/app`) toujours en service, figée. Détail : section « Tranche 1 » ci-dessous | `ent-rust/web/`, `ent-rust/crates/ent-types`, `docs/FRONTEND_CIBLE.md`, `docs/DESIGN_SYSTEM_LEPTOS.md` | relance du 2026-11-03 : ent-web-ui 14, ent-web-api 5, ent-web-app 28, ent-web-server 24 (lib 2 + contre_backend 8 + droits_parite 12 + entetes_securite 2), ent-web-session 9 ; clippy 0 avertissement (ssr, wasm32) ; essai navigateur intégré (ordinateur, 375 px, 400 élèves) | lecteur d'écran réel non testé ; passkey non testée au navigateur ; limites listées ci-dessous |
| Auth | **Implémenté** : Argon2id, sessions Postgres (jeton opaque Bearer), passkeys `webauthn-rs` (facultatives), SSO OIDC | `ent-rust/crates/ent-auth`, `ent-oidc`, `ent-api/src/auth.rs` | tests unitaires + parcours connexion/401/403 ; ent-api 30 tests, ent-db 42 (relance du 2026-11-03) | jeton en `localStorage` côté web (risque XSS atténué par la CSP du 2026-11-03) ; **prototype isolé de cookie HttpOnly testé** (`ent-web-session`, 9 tests + essai Chrome, `SESSION_WEB_COOKIE_ETUDE.md` §14), production non décidée ; passkey obligatoire pour tous à construire (D-009) |
| PostgreSQL/migrations | **Implémenté** : migrations embarquées, appliquées au démarrage ; refus de tout hôte non local | `ent-db/src/migrate.rs`, `url_guard.rs` | migrations appliquées sur une base locale vierge (PostgreSQL 18.4) par le prototype | pas de RLS ; levée du verrou « local uniquement » à prévoir pour un déploiement |
| Stockage fichiers | **Partiel** : disque local réel ; mode « S3 » sans TLS, sans signature, sans URL pré-signée | `ent-rust/crates/ent-object` | non testé | non prêt pour Scaleway Object Storage |
| Email/SMS | **Partiel** : SMTP réel opt-in (`lettre`), SMS/push/banque en mode simulé ; fournisseur SMS décidé (OVH, D-004) non intégré | `ent-mail`, `ent-sms`, `ent-push`, `ent-bank` | non testé | connecteur OVH à écrire |
| Android/iOS | **Prototype non compilable** : 4 applis (famille/élève × iOS/Android), sans fichiers de build ; une seule appli par plateforme décidée (D-003) | `ent-rust/apps/` | aucun | fusion des 4 prototypes, projets Gradle/Xcode à créer |
| IA/conversion | **Absent côté Rust** : copilote à règles fixes, sans modèle de langage ; conversion de fichiers décidée (D-005) non commencée | `ent-db/src/copilote.rs`, `ent-brain` | — | choix du fournisseur LLM (Mistral à étudier) |
| CI/tests | **Workflow `ent-rust.yml`** (D-015) : 1er run GitHub : formatage, audits, backend verts ; frontend en échec (F-003), **corrigé localement, non poussé** ; lockfiles versionnés ; audits `cargo deny` + `cargo audit` | `.github/workflows/ent-rust.yml`, `docs/CI_REPRODUCTIBILITE.md` | 270 tests backend + 29 frontend/ent-types verts en local | dette : 706 écarts de formatage (procédure prête, `FORMATAGE_BACKEND.md`) et 60 avertissements Clippy (plan : `CLIPPY_PLAN.md`) |

## Tranche 1 — frontend enseignant (2026-11-03)

**Niveau atteint : implémenté + testé en local.** Non validé pilote, non prêt production, rien poussé ni commité à ce jour.

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
