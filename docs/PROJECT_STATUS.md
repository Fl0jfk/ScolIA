# État du projet — à maintenir sur faits vérifiés

Date : 2026-11-02. Branche : `ENT` (D-001).
Sources : audit en lecture seule du 2026-11-02 et prototype Leptos (`docs/LEPTOS_CALL_PROTOTYPE.md`).

| Domaine | État observé | Preuve (chemin) | Tests | Risques/questions |
|---|---|---|---|---|
| Backend Rust | **Implémenté, non déployé** : axum 0.8 + sqlx, ~268 routes, 90 migrations, nombreux domaines (élève, absences/appel, notes, EDT, VS, facturation…) | `ent-rust/crates/ent-api`, `ent-db`, `domain-eleve`, `ent-rust/migrations/` | ~270 tests Rust (non lancés dans la CI) ; routes d'appel testées contre une base locale par le prototype | aucune CI Rust ; `Cargo.lock` non versionné ; erreurs classées par mots du message (`map_db`) |
| Frontend Rust | **Prototype validé** (D-010) : module d'appel en Leptos 0.8, même origine qu'`ent-api` ; ancienne UI JavaScript (`ui.rs`, 54 000 lignes) toujours en service, figée | `ent-rust/web/` (non suivi par Git), `docs/LEPTOS_CALL_PROTOTYPE.md` | 11 unitaires + 4 contre le vrai backend, verts ; axe-core 0 violation | pas encore dans la CI ; lecteur d'écran réel non testé ; design system à construire |
| Auth | **Implémenté** : Argon2id, sessions Postgres (jeton opaque Bearer), passkeys `webauthn-rs` (facultatives), SSO OIDC | `ent-rust/crates/ent-auth`, `ent-oidc`, `ent-api/src/auth.rs` | tests unitaires + parcours connexion/401/403 du prototype | jeton en `localStorage` côté web (étude cookie : `SESSION_WEB_COOKIE_ETUDE.md`) ; passkey obligatoire pour tous à construire (D-009) |
| PostgreSQL/migrations | **Implémenté** : migrations embarquées, appliquées au démarrage ; refus de tout hôte non local | `ent-db/src/migrate.rs`, `url_guard.rs` | migrations appliquées sur une base locale vierge (PostgreSQL 18.4) par le prototype | pas de RLS ; levée du verrou « local uniquement » à prévoir pour un déploiement |
| Stockage fichiers | **Partiel** : disque local réel ; mode « S3 » sans TLS, sans signature, sans URL pré-signée | `ent-rust/crates/ent-object` | non testé | non prêt pour Scaleway Object Storage |
| Email/SMS | **Partiel** : SMTP réel opt-in (`lettre`), SMS/push/banque en mode simulé ; fournisseur SMS décidé (OVH, D-004) non intégré | `ent-mail`, `ent-sms`, `ent-push`, `ent-bank` | non testé | connecteur OVH à écrire |
| Android/iOS | **Prototype non compilable** : 4 applis (famille/élève × iOS/Android), sans fichiers de build ; une seule appli par plateforme décidée (D-003) | `ent-rust/apps/` | aucun | fusion des 4 prototypes, projets Gradle/Xcode à créer |
| IA/conversion | **Absent côté Rust** : copilote à règles fixes, sans modèle de langage ; conversion de fichiers décidée (D-005) non commencée | `ent-db/src/copilote.rs`, `ent-brain` | — | choix du fournisseur LLM (Mistral à étudier) |
| CI/tests | **Next.js seulement** : workflows sur `main`/`dev` ; rien pour Rust ni pour `ENT` | `.github/workflows/` | — | ajouter build + tests Rust et Leptos sur `ENT` |

Pour chaque domaine fonctionnel, préciser absent / prototype / partiel / implémenté / testé / validé / prêt pilote. Ne jamais remplir par supposition. Next.js prod reste intouchable ; DB prod inaccessible.
