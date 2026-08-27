# AGENTS.md — guide opérationnel pour agents (Cursor Cloud / IDE)

Repo **docslapro / ScolIA** : ENT / intranet scolaire (Next.js App Router, Drizzle, PostgreSQL, Better-Auth).

## Commandes essentielles

| Action | Commande |
|--------|----------|
| Install deps | `npm ci` |
| Dev server | `npm run dev` → http://localhost:3000 |
| Seed local | `npm run seed:dev` |
| Code TOTP seed | `npm run seed:dev:totp` |
| Sync schéma (dev) | `npx drizzle-kit push --force` |
| Tests OCR match | `npm run test:ocr-match` |
| Lint | `npm run lint` |

## Compte de test local (après `seed:dev`)

| Champ | Valeur |
|-------|--------|
| URL | http://localhost:3000/auth/sign-in?dev_tenant=default |
| E-mail | `admin@localhost.dev` |
| Mot de passe | `DevLocalPass1!` |
| TOTP | secret `DEVLOCALTOTPSECRET00000000000001` — générer le code via `npm run seed:dev:totp` |

Ce compte est **orgAdmin** + rôle `admin`, MFA déjà activée. Ne jamais utiliser ces identifiants en production.

Tenant local mono-instance : slug `default`, cookie/query `dev_tenant` (voir `app/lib/local-dev.ts`).

## Base de données — stratégie migrations

Incohérence connue dans l’historique Drizzle : `0000_initial.sql` est déjà au schéma post-renommage (`external_user_id` / `auth_user_mapping`), alors que `0002_rename_clerk_ids.sql` tente encore de renommer des colonnes `clerk_*`. Un `drizzle-kit migrate` sur base **vierge** échoue.

| Contexte | Méthode |
|----------|---------|
| **Dev / Cloud Agent (base locale)** | `npx drizzle-kit push --force` (aligne sur `db/schema.ts`) |
| **Prod / Scaleway (déjà peuplée)** | `node scripts/apply-migrations-direct.mjs` (backfill jusqu’à `0013` puis apply) |

Ne pas « corriger » `0002` à la légère : la prod repose sur le backfill. Documenter tout changement de stratégie ici.

Postgres local Cloud Agent (si `install.sh`) :

- URL : `postgresql://scola:scola_dev_pwd@127.0.0.1:5432/scola`
- Variables dans `.env.local` (gitignored), généré par `.cursor/install.sh`

## MCP

Config IDE / projet : `.cursor/mcp.json` (chemins portables Linux/macOS). Wrapper Postgres : `.cursor/run-postgres-mcp.sh`.

| Serveur | Rôle | Prérequis |
|---------|------|-----------|
| **postgres** | Lire schéma / données | `DATABASE_URL` ou `MCP_DATABASE_URL` |
| **browser** | Playwright UI locale | Chromium (`npx playwright install chromium`) |
| **fetch** | Doc officielle | `uvx` (installé par `install.sh`) |
| **mistral** | OCR / vision | secret `MISTRAL_API_KEY` |
| **scaleway** | Infra | CLI `scw` + secrets `SCW_*` |

### Important — Cloud Agents

`.cursor/mcp.json` **n’est pas** chargé automatiquement par les Cloud Agents dans le catalogue d’outils. Les MCP cloud se configurent dans le **dashboard** :

1. [cursor.com/agents](https://cursor.com/agents) → menu **MCP** (ou Dashboard → Integrations & MCP pour une équipe)
2. Ajouter chaque serveur en **stdio** (mêmes commandes que `mcp.json`) ou HTTP si disponible
3. Y coller les secrets (`MISTRAL_API_KEY`, `MCP_DATABASE_URL`, `SCW_*`) dans l’env du MCP
4. Si `mcpServerAllowlist` est défini dans l’environnement, y autoriser les commandes `npx` / `uvx` / `bash` / `scw`

Sur une VM cloud, les binaires sont déjà préparés par `install.sh` (`uvx`, `scw`, Playwright). Sans enregistrement dashboard, l’agent ne voit que les MCP Cursor internes (`cursor-cloud`, etc.) — repli possible : appeler les serveurs en stdio manuellement / `psql` / `curl`.

Si un MCP n’est pas disponible : le signaler, ne pas inventer l’état.

## Multi-tenant & sécurité (rappel)

- Toute table métier : `etablissement_id` + filtre session.
- Auth cible : **Better-Auth uniquement** (pas NextAuth / Clerk).
- Secrets : uniquement `process.env` / dashboard Secrets — jamais committer `.env.local`.

## Hors scope sans confirmation explicite

- Mutations prod Scaleway (RDB, buckets, containers)
- Envoi d’e-mails réels (SMTP)
- Import massif SIECLE / données élèves réelles

## Cursor Cloud specific instructions

### Boot

1. `.cursor/start.sh` démarre PostgreSQL.
2. Terminal `next-dev` : `npm run dev` (port **3000**).
3. Si besoin : `npm run seed:dev` (idempotent).

### Validation minimale d’un changement UI / auth

1. `curl -s -o /dev/null -w "%{http_code}" http://localhost:3000/` → 200
2. Ouvrir `/auth/sign-in?dev_tenant=default`
3. Login seed + TOTP (`npm run seed:dev:totp`)
4. Vérifier absence d’erreurs console / 500
5. Si mutation BDD : contrôler via MCP postgres ou `psql`

### Secrets optionnels (dashboard)

Non requis pour booter localement. Utile pour OCR / S3 / MCP Scaleway :

- `MISTRAL_API_KEY`
- `SCW_ACCESS_KEY`, `SCW_SECRET_KEY`, `SCW_DEFAULT_ORGANIZATION_ID`, `SCW_DEFAULT_PROJECT_ID`, `SCW_DEFAULT_REGION`
- `MCP_DATABASE_URL` (Postgres Scaleway lecture seule, si tests contre la vraie base)

### Fichiers env Cloud

- Install : `bash .cursor/install.sh`
- Start : `bash .cursor/start.sh`
- Config : `.cursor/environment.json`
