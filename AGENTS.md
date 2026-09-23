# AGENTS.md — guide opérationnel pour agents (Cursor Cloud / IDE)

Repo **docslapro / ScolIA** : ENT / intranet scolaire (Next.js App Router, Drizzle, PostgreSQL, Better-Auth).

## Value Gate — valeur fonctionnelle (obligatoire)

Deux validations **distinctes** après chaque lot **important**. Aucune ne remplace l’autre.

| | |
|--|--|
| **Technical validation** | Code, tests, sécurité, multi-tenant, self-healing UI/BDD (`scola-validation-autonome`). |
| **Functional value validation** | Un **scénario métier réel** fonctionne de bout en bout, **observable** (pas un rapport de tests). |

**N’est pas** une preuve de valeur : test vert, architecture, table, API, event store, tool Brain.

Développer en **vertical slice** : UI/API → métier → persistance → événements si besoin → conséquences inter-domaines → permissions → tenant → tests → **résultat observable**.

Si beaucoup d’infra et peu de geste utilisateur : le dire clairement (« fondation OK, pas encore démontrable ») — ne pas cocher le lot comme terminé produit.

Jalon à remplir (détail : `.cursor/rules/scola-value-gate.mdc`) :

```
VALUE GATE
→ scénario métier démontrable
→ résultat attendu
→ résultat réellement obtenu
→ domaines traversés
→ éléments encore manquants
→ limites connues
→ verdict : VALEUR | PARTIEL | INFRA SEULE
```

## Git — production protégée

| Branche | Rôle |
|---------|------|
| **`main`** | Production. Un push déclenche `.github/workflows/deploy-scaleway.yml` (conteneur Scaleway réel). |
| **`dev`** | Intégration architecture / lots métier. Branche de travail des agents. |
| `feature/…` | Optionnel, à partir de `dev`, puis merge dans `dev`. |

**Jamais** de `git push origin main` pour un lot métier. Passage prod = validation humaine puis merge `dev` → `main`.

Tests = Postgres **local** `127.0.0.1` (`install.sh`). Pas de migration / seed / wipe sur la RDB Scaleway. `scripts/apply-migrations-direct.mjs` refuse une URL non locale sauf `ALLOW_PROD_MIGRATION=1` (validation humaine obligatoire).

## Labo Florian (hors prod)

Voir `docs/labo-florian.md`. Conteneur Scaleway **séparé** + Postgres labo + seed Leo. Workflow `.github/workflows/deploy-lab.yml` sur push `dev` (tags `:lab`, jamais `:latest`).

| Action | Commande |
|--------|----------|
| Bootstrap labo (schéma + seed) | `SCOLA_ENV=lab ALLOW_LAB_MIGRATION=1 DATABASE_URL=… npm run seed:labo` |
| TOTP admin seed | `npm run seed:dev:totp` |

Runtime labo : `SCOLA_ENV=lab` / `NEXT_PUBLIC_SCOLA_ENV=lab` → bandeau ambre. **Interdit** : pointer le labo sur la RDB prod ou `SCW_CONTAINER_ID` prod.

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
| Valkey local (optionnel) | `docker run -d --name scolia-valkey -p 6379:6379 valkey/valkey:8` puis `VALKEY_URL=redis://127.0.0.1:6379` |
| Collabora CODE (bureautique) | Local : `docker compose -f docker-compose.collabora.yml up -d` + `COLLABORA_URL` / `WOPI_HOST`. Prod Scaleway : container `collabora-code` (`COLLABORA_URL=https://containerscolia1c9956df-collabora-code.functions.fnc.fr-par.scw.cloud`, `WOPI_HOST=https://lpnb.scolia.fr` — hôte tenant TLS valide, pas l’apex) |

## Compte de test local (après `seed:dev`)

| Champ | Valeur |
|-------|--------|
| URL | http://localhost:3000/auth/sign-in?dev_tenant=default |
| E-mail | `admin@localhost.dev` |
| Mot de passe | `DevLocalPass1!` |
| TOTP | secret `DEVLOCALTOTPSECRET00000000000001` — générer le code via `npm run seed:dev:totp` |

Compte **parent** (portail `/famille`, absences / justifs) :

| Champ | Valeur |
|-------|--------|
| E-mail | `parent@localhost.dev` |
| Mot de passe | `DevParentPass1!` |
| MFA | non (démo locale) |
| Enfant seed | Léo JUSTIF (classe 4B) |

Ces comptes sont locaux uniquement. Ne jamais utiliser ces identifiants en production.

Tenant local mono-instance : slug `default`, cookie/query `dev_tenant` (voir `app/lib/local-dev.ts`).

## Base de données — stratégie migrations

Incohérence connue dans l’historique Drizzle : `0000_initial.sql` est déjà au schéma post-renommage (`external_user_id` / `auth_user_mapping`), alors que `0002_rename_clerk_ids.sql` tente encore de renommer des colonnes `clerk_*`. Un `drizzle-kit migrate` sur base **vierge** échoue.

| Contexte | Méthode |
|----------|---------|
| **Dev / Cloud Agent (base locale)** | `npx drizzle-kit push --force` — **uniquement** si `DATABASE_URL` = `127.0.0.1` |
| **Prod / Scaleway** | `node scripts/apply-migrations-direct.mjs` **interdit** sans validation humaine + `ALLOW_PROD_MIGRATION=1` |

Ne pas « corriger » `0002` à la légère : la prod repose sur le backfill. Documenter tout changement de stratégie ici.

Postgres local Cloud Agent (si `install.sh`) :

- URL : `postgresql://scola:scola_dev_pwd@127.0.0.1:5432/scola`
- Variables dans `.env.local` (gitignored), généré par `.cursor/install.sh`

## MCP et outils agents

Florian n’a **rien à coller** (pas de JSON dashboard, pas d’Authenticate). Secrets uniquement via `process.env` / `.env.local` / `.cursor/mcp.local.env` (gitignored).

| Surface | Ce qui est branché |
|---------|-------------------|
| **IDE / Project** | `.cursor/mcp.json` (stdio via wrappers `node` + `${workspaceFolder}`) + plugin `.cursor/plugins/scola-mcp` chargé par le hook `workspaceOpen` |
| **Cloud Agent (cette VM)** | Binaires installés par `install.sh` (`uvx`, `scw`, Playwright Chromium, `psql`). Catalogue MCP Cloud = dashboard Cursor seulement |
| **Repli tous agents** | `.cursor/tools/` — `psql.sh`, `fetch.mjs`, `browser.mjs`, `scw.sh`, `status.sh` (pas de protocole MCP) |

| Serveur | Rôle | Prérequis |
|---------|------|-----------|
| **postgres** | Lire schéma / données | `MCP_DATABASE_URL` ou `DATABASE_URL` |
| **browser** | Playwright UI locale | Chromium (`install.sh`) |
| **fetch** | Doc officielle | `uvx` (`install.sh` → `/usr/local/bin/uvx`) |
| **mistral** | OCR / vision | `MISTRAL_API_KEY` si présent, sinon le wrapper échoue clairement |
| **scaleway** | Infra lecture | `scw` + `SCW_*` si présents ; **pas de mutation prod** |
| **github** | Optionnel | `GITHUB_TOKEN` / PAT + binaire `github-mcp-server` ; sinon ignoré |

**Limitation Cursor :** un Cloud Agent déjà lancé ne voit pas les MCP de l’IDE (`.cursor/mcp.json` / plugin projet) ; `workspaceOpen` ne s’exécute pas sur Cloud Agents. Un nouvel agent IDE charge le plugin ; un Cloud Agent utilise le repli `.cursor/tools/` tant que le dashboard équipe n’a pas le MCP (hors de portée repo).

Si un MCP n’est pas dans le catalogue de la session : le dire, utiliser le repli, **ne pas inventer** l’état.

```bash
bash .cursor/tools/status.sh
bash .cursor/tools/psql.sh -c 'SELECT 1'
node .cursor/tools/fetch.mjs https://example.com
node .cursor/tools/browser.mjs http://localhost:3000
```

## Multi-tenant & sécurité (rappel)

- Toute table métier : `etablissement_id` + filtre session.
- Auth cible : **Better-Auth uniquement** (pas NextAuth / Clerk).
- Secrets : uniquement `process.env` / dashboard Secrets — jamais committer `.env.local`.

## Absences accueil & registre VS

La page **Absence accueil** (`/accueil/absences`, module `accueil-absences`) enregistre le signal jour J (élèves → `vs_absence_eleve`, profs / OGEC → table `absence` RH). **Registre VS cible = ScolIA** (`vs_absence_eleve`, appels, sanctions, carnet). Charlemagne = **immigration only** (import / migration) — le pont runtime `app/lib/absences-sync/port.ts` reste `noop` volontairement. Les absences profs saisies à l’accueil passent par la validation direction, puis calendrier + mail secrétariat (déclaration rectorat), comme le circuit RH classique.

## RDV inscriptions (Google Agenda)

Module `rdv-inscription` : pages publiques `/rdv-inscription/[direction]` (une par direction) listant les créneaux Google dont le titre contient le motif configuré (défaut « rendez-vous inscription »). Paramétrage : `/etablissement/rdv-inscription`. OAuth compte technique (`GOOGLE_CLIENT_ID` / `GOOGLE_CLIENT_SECRET`) ; chaque directrice partage son agenda avec ce compte. Migrations : `drizzle/0042_rdv_inscription.sql` … `0046_rdv_inscription_eleve_reconfirm.sql`.

Flux parent : e-mail d’abord → matching protégé (pool = enfants liés au contact) → confirmation interactive → niveau → créneau → double opt-in mail → lien dossier `/eleves/dossier/[id]/inscription` dans Google Agenda. Relance J-7 « toujours OK ? » via `POST /api/rdv-inscription/reconfirm-cron` (`RDV_INSCRIPTION_CRON_SECRET` ou `TRAVELS_CRON_SECRET`) — le silence ne supprime pas le RDV.

## Hors scope sans confirmation explicite

- Mutations prod Scaleway (RDB, buckets, containers)
- `git push origin main` / merge vers `main`
- `scripts/apply-migrations-direct.mjs` ou `drizzle-kit push` contre une URL non locale
- Envoi d’e-mails réels (SMTP)
- Import massif SIECLE / données élèves réelles

## Cursor Cloud specific instructions

### Boot

1. `.cursor/start.sh` démarre PostgreSQL et rattache `uvx` / `scw` à `/usr/local/bin`.
2. Terminal `next-dev` : `npm run dev` (port **3000**).
3. Si besoin : `npm run seed:dev` (idempotent).
4. Diagnostic outils : `bash .cursor/tools/status.sh`.

### Validation minimale d’un changement UI / auth

1. `curl -s -o /dev/null -w "%{http_code}" http://localhost:3000/` → 200
2. Ouvrir `/auth/sign-in?dev_tenant=default`
3. Login seed + TOTP (`npm run seed:dev:totp`)
4. Vérifier absence d’erreurs console / 500
5. Si mutation BDD : contrôler via MCP postgres **ou** `bash .cursor/tools/psql.sh`

### Secrets optionnels (dashboard)

Non requis pour booter localement. Utile pour OCR / S3 / MCP Scaleway / cache Valkey :

- `MISTRAL_API_KEY`
- `SCW_ACCESS_KEY`, `SCW_SECRET_KEY`, `SCW_DEFAULT_ORGANIZATION_ID`, `SCW_DEFAULT_PROJECT_ID`, `SCW_DEFAULT_REGION`
- `MCP_DATABASE_URL` — **ne pas** y mettre l’URL RDB prod pour tester ; cette VM = `127.0.0.1`
- `VALKEY_URL` (ou `REDIS_URL`) — cache partagé auth / messagerie / dossiers / dashboard. Sans URL, l’app tourne avec repli mémoire + Postgres.

### Fichiers env Cloud

- Install : `bash .cursor/install.sh`
- Start : `bash .cursor/start.sh`
- Config : `.cursor/environment.json`

<!-- BEGIN:nextjs-agent-rules -->

# This is NOT the Next.js you know

This version has breaking changes — APIs, conventions, and file structure may all differ from your training data. Read the relevant guide in `node_modules/next/dist/docs/` (resolved from this file's directory; in monorepos the `next` package may not be visible from the repo root) before writing any code. Heed deprecation notices.

This block is written and re-added by `next dev` — verify at `node_modules/next/dist/server/lib/generate-agent-files.js`. Removing it from a diff only re-creates the uncommitted change; committing it with your work keeps the tree clean.

<!-- END:nextjs-agent-rules -->
