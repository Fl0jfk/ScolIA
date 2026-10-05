# Labo ScolIA pour Florian — hors production

Décision annuaire (§ Premier livrable visible) : **voir `dev`**, pas seulement l’annuaire. Pas Vercel. Conteneur Scaleway + **autre** Postgres + seed Leo. Jamais `main`, jamais la RDB prod.

## Objectif

URL dédiée où Florian joue une journée parent + staff (appel, notes, EDT, cahier, messagerie, carnet, sanctions, `/quotidien`) sur des données fictives.

## Une fois — console Scaleway (Florian)

1. **Postgres labo**
   - Créer une base (instance RDB dédiée **ou** database `scolia_lab` sur une instance hors prod).
   - Le hostname **ou** le nom de base **doit** contenir `lab` (garde-fou scripts).
   - Noter `DATABASE_URL` (SSL Scaleway).

2. **Conteneur Serverless**
   - Namespace existant `container-scolia` (ou équivalent).
   - Nouveau conteneur : nom `scolia-lab` (≠ `container-app` prod).
   - Port `8080`, même limites mémoire raisonnables que la prod.
   - Variables runtime (console) :

| Variable | Valeur |
|----------|--------|
| `DATABASE_URL` | URL Postgres **labo** |
| `SCOLA_ENV` | `lab` |
| `NEXT_PUBLIC_SCOLA_ENV` | `lab` |
| `BETTER_AUTH_URL` | URL publique du labo |
| `BETTER_AUTH_SECRET` | secret **distinct** de la prod |
| `AUTH_PROVIDER` | `better-auth` |
| `ENT_CORE_DB` | `1` |
| `NEXT_PUBLIC_APP_URL` | même URL publique |

3. **DNS / URL**
   - Soit l’URL Scaleway `https://…functions.fnc.fr-par.scw.cloud`
   - Soit un domaine `lab.scolia.fr` (CNAME) — recommandé.

4. **Secrets GitHub** (Settings → Actions → Secrets)

| Secret | Rôle |
|--------|------|
| `NEXT_PUBLIC_LAB_APP_URL` | URL publique labo |
| `SCW_LAB_CONTAINER_ID` | UUID du conteneur `scolia-lab` (**≠** `SCW_CONTAINER_ID` prod) |
| `SECRET_ACCESS_KEY` | déjà présent (registry + API) |

5. **Premier seed** (depuis une machine avec l’URL labo, pas la prod) :

```bash
export SCOLA_ENV=lab
export ALLOW_LAB_MIGRATION=1
export DATABASE_URL='postgresql://…/scolia_lab?sslmode=require'
npm run seed:labo
```

Comptes seed (fictifs) :

| Rôle | E-mail | Mot de passe |
|------|--------|--------------|
| Staff admin | `admin@localhost.dev` | `DevLocalPass1!` (+ TOTP `npm run seed:dev:totp`) |
| Parent | `parent@localhost.dev` | `DevParentPass1!` |
| Élève | Leo JUSTIF · classe 4B | — |

## Automatique ensuite

Chaque push sur **`dev`** → workflow **Deploy lab** (`.github/workflows/deploy-lab.yml`) :

- Build image tags `:lab` et `:lab-<sha>` (**jamais** `:latest` prod)
- Redeploy uniquement `SCW_LAB_CONTAINER_ID`
- Refus si `SCW_LAB_CONTAINER_ID` == `SCW_CONTAINER_ID`

Prod reste : push `main` → **Deploy container**.

## Parcours de démo (Value Gate)

1. Ouvrir l’URL labo → bandeau ambre « Labo ScolIA — hors production ».
2. Parent : `/auth/sign-in?dev_tenant=default` → `parent@localhost.dev` → hub `/quotidien` (EDT, notes, messages, cahier, sanctions).
3. Staff : `admin@localhost.dev` + TOTP → appel / notes / cahier / messagerie.

## Ce que ce n’est pas

- Pas un merge vers `main`.
- Pas Vercel / preview qui pointerait la base réelle.
- Pas les apps natives (SwiftUI / Compose) — horizon classique.
- Tant que l’URL labo n’est pas joignable : verdict produit **INFRA SEULE** (jauge ENT inchangée à 25 %).
