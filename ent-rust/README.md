# `ent-rust` — vrai ENT (cible)

Workspace Cargo du chantier **Rust partout + PostgreSQL**.

## Crates (amorce)

| Crate | Rôle |
|---|---|
| `ent-events` | Bus d'événements métier |
| `domain-eleve` | Socle élève : dates d'effet (périodes) / absences / events |
| `ent-brain` | Cerveau : events + écritures **directes** additionnelles (pas de copie d'état) |
| `ent-db` | Postgres local (`ent_*`) + garde anti-prod — effectif = `SELECT` daté sur périodes |
| `ent-auth` | Argon2id, tokens de session (hash SHA-256 en BDD) + **passkeys WebAuthn** |
| `ent-oidc` | **SSO OIDC** : découverte, PKCE S256, vérification `id_token` RS256 (JWKS), transport injectable |
| `ent-obs` | **Métriques** Prometheus (compteurs / jauges / histogrammes atomiques), **sans dépendance externe** |
| `ent-mail` / `ent-sms` / `ent-push` | Passerelles de notification (dry-run par défaut, live opt-in par env) |
| `ent-bank` | Relevés / rapprochement SEPA (dry-run par défaut) |
| `ent-object` | Stockage objet S3 / disque (pièces jointes, PDF) |
| `ent-api` | HTTP **axum** — register/login/session PG ; lit le noyau |
| `ent-lab` | Runner labo : fait vivre le cast multi-personas et rapporte ce qui remonte |

**Auth = cible prod** (pas un bricolage) : `POST /auth/register`, `/auth/login` → Bearer token opaque ; tenant = `ent_session.etablissement_id`.

**2ᵉ facteur = passkey (WebAuthn) uniquement** — aucun TOTP, aucune application
d'authentification : `ent_passkey_credential` + challenge à usage unique ; dès
qu'un compte possède une passkey, le mot de passe seul n'ouvre plus de session
(et le chemin interne refuse). Connexion **sans mot de passe** possible,
récupération Direction/Admin. UI `/app/securite` (+ `/famille/securite`,
`/eleve/securite`).

**SSO OIDC par établissement** : annuaire Google Workspace / Microsoft Entra /
Keycloak ; configuration **validée par découverte réelle**, autorisation
PKCE S256 + `state` + `nonce`, `id_token` **RS256 seulement** vérifié contre le
JWKS, `state` à usage unique, rattachement par `sub` puis e-mail d'un membre
existant — **aucun provisioning implicite**, et le SSO **ne contourne pas** une
passkey déjà enrôlée. UI : bouton SSO à la connexion, page de retour
`/auth/oidc/callback`, section « SSO établissement » dans `/app/securite`.
Détails : [`../docs/ent-rust-transverse.md`](../docs/ent-rust-transverse.md).

**Continuité métier :** `docs/metier-*.md` etc. — pas de reset. Chantier hors prod ScolIA actuelle.

**Labo multi-personas :** `PgEnt::seed_labo_cast` + scénarios + runner `cargo run -p ent-lab` (journée autonome + rapport). Doc : [`../docs/ent-labo-personas.md`](../docs/ent-labo-personas.md).

**Vies croisées (cycle de vie) :** deux parcours de bout en bout sont joués + testés — *changement de classe en cours d'année* et *sortie élève* (statut + effet daté, révocation du compte face + sessions, clôture grilles repas, annulation docs à signer, journal `ent_journal_modif`). Les bascules datées (classe/régime/MEF/site/statut, identité) alimentent aussi le journal (`classe_change`, `statut_change`, …).

**API + UI labo minimale :**
```bash
set ENT_DATABASE_URL=postgres://scola:scola_dev_pwd@127.0.0.1:5432/scola
cargo run -p ent-api
# → http://127.0.0.1:8088/  (boutons bootstrap / login / dossier)
# JSON : POST /auth/login { email, password, etablissement_id } → { token }
# Authorization: Bearer <token>
```

**Durcissement prod (déjà en place) :** liveness `GET /health` + readiness
`GET /ready` (ping Postgres), `x-request-id` sur chaque réponse, en-têtes de
sécurité, limite de corps, timeout requête, CORS opt-in, arrêt gracieux
`SIGTERM`, anti-force brute (verrous Postgres partagés **par compte et par IP**), sessions à TTL
configurable, changement de mot de passe (révoque les autres sessions) et audit
d'accès (`ent_journal_modif` domaine `auth` + `GET /audit/journal`). En
`ENT_ENV=prod`, `POST /auth/register` et `POST /labo/bootstrap` sont fermés
(403) sauf dérogation explicite ; les comptes **staff** s'ouvrent alors via
`POST /etablissement/personnel` (Direction / Admin). Détails :
[`../docs/ent-rust-prod-runbook.md`](../docs/ent-rust-prod-runbook.md).

**Image de production :** `ent-rust/Dockerfile` (multi-étages, `rust` → `debian slim`,
migrations embarquées dans le binaire).

**Observabilité & PRA :** `GET /metrics` (format Prometheus) — **404 par défaut**,
activé par `ENT_METRICS_TOKEN` (Bearer, comparaison à temps constant) ou
`ENT_METRICS=1` pour un scrape local ; compteurs par **gabarit de route**
(`MatchedPath`, jamais d'identifiant brut), histogramme de latence, sondes base
(pool, sessions actives, outbox `dry_run`, passkeys, élèves) — aucune donnée
nominative. `/health` et `/ready` publient le `migration_tag`. Journaux JSON via
`ENT_LOG_FORMAT=json`. Côté sauvegardes, l'ENT fournit un **inventaire
d'intégrité** par établissement (`GET /pra/inventaire`, Direction/Admin) et sa
vérification (`POST /pra/inventaire/verifier`) : `count(*)` + digest **ordonné**
par table + empreinte de schéma → une restauration se **prouve**, et une dérive
est localisée (tables volatiles de sécurité exclues, portée scellée sur
l'établissement de la session). Scripts `scripts/sauvegarde-postgres.sh`,
`restauration-postgres.sh`, `pra-exercice.sh` (loopback obligatoire,
`--autotest` sans base). Détails :
[`../docs/ent-rust-observabilite-pra.md`](../docs/ent-rust-observabilite-pra.md).

**Mot de passe oublié (libre-service) :** `POST /auth/forgot-password { email }`
→ **202** systématique (anti-énumération) ; si le compte existe, un jeton à usage
unique (TTL 30 min) part par e-mail et `POST /auth/reset-password`
(`{ token, new_password }`) change le mot de passe en **révoquant toutes les
sessions**.
Pages publiques `/mot-de-passe-oublie` et `/reset-mot-de-passe`. Rappel :
les workers tokio ont une pile élargie (`ENT_WORKER_STACK_MB`, défaut 16 Mio).

**Copilote (assistant lecture seule) :** `POST /copilote/ask { question }`
(personnel) répond en français aux questions courantes — effectif (total / par
classe), absences non justifiées sur une fenêtre, factures non soldées, élèves
sans compte, alertes CPE. Intentions **déterministes** (pas de LLM requis),
chaque réponse recalculée sur l'établissement de la session, **capacité du rôle**
revérifiée par intention (refus explicite, jamais de fuite). Page staff
`/app/copilote` + `GET /copilote/capacites`. Détails :
[`../docs/ent-rust-quality.md`](../docs/ent-rust-quality.md) §8.

**UI (Tailwind v4 + Michelangelus) :** le HTML vit dans `crates/ent-api/src/ui.rs` ; le CSS est compilé à part puis servi sous `/static`.
```bash
cd crates/ent-api
npm install
npm run css:build          # → static/css/app.css
# optionnel : npm run css:watch
node ../../scripts/fetch-michelangelus.mjs   # fontes locales (EULA MS, non versionnées)
```

Docs :

- Vision : [`../docs/ent-vision-rust.md`](../docs/ent-vision-rust.md)
- Qualité / DoD : [`../docs/ent-rust-quality.md`](../docs/ent-rust-quality.md)
- **Carte transverse** (tout est-il lié ?) : [`../docs/ent-rust-transverse.md`](../docs/ent-rust-transverse.md)
- **Observabilité & PRA** (métriques, sauvegardes, preuve de restauration) : [`../docs/ent-rust-observabilite-pra.md`](../docs/ent-rust-observabilite-pra.md)
- Check-list prod (env, supervision, sécurité) : [`../docs/ent-rust-prod-runbook.md`](../docs/ent-rust-prod-runbook.md)
- Checklist métier : [`../docs/ent-metier-checklist.md`](../docs/ent-metier-checklist.md)

## Commandes

```bash
cd ent-rust
cargo test

# Garde-fous de sauvegarde (aucune base, aucun binaire PostgreSQL requis)
bash scripts/pra-exercice.sh --autotest            # → "Autotest OK"

# Exercice PRA complet en labo (Postgres local + pg_dump/pg_restore)
DATABASE_URL=postgres://scola:scola_dev_pwd@127.0.0.1:5432/scola \
  bash scripts/pra-exercice.sh
```

### Base de données — interdit prod

- **Jamais** la RDB prod / Scaleway / URL distante du `.env.local` ScolIA.
- L1 uniquement via `ENT_DATABASE_URL=postgres://…@127.0.0.1:5432/…`
- `PgEnt::connect` **refuse** tout hôte hors `127.0.0.1` / `localhost` / `::1` (test unitaire dédié).
- Tables préfixées `ent_*` pour ne pas mélanger avec le schéma ScolIA même en local.

```bash
# Exemple local (Postgres doit tourner sur ta machine)
set ENT_DATABASE_URL=postgres://scola:scola_dev_pwd@127.0.0.1:5432/scola
cargo test -p ent-db
```
