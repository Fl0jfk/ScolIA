# `ent-rust` — vrai ENT (cible)

Workspace Cargo du chantier **Rust partout + PostgreSQL**.

## Crates (amorce)

| Crate | Rôle |
|---|---|
| `ent-events` | Bus d'événements métier |
| `domain-eleve` | Socle élève : dates d'effet (périodes) / absences / events |
| `ent-brain` | Cerveau : events + écritures **directes** additionnelles (pas de copie d'état) |
| `ent-db` | Postgres local (`ent_*`) + garde anti-prod — effectif = `SELECT` daté sur périodes |
| `ent-auth` | Argon2id + tokens de session (hash SHA-256 en BDD) |
| `ent-api` | HTTP **axum** — register/login/session PG ; lit le noyau |

**Auth = cible prod** (pas un bricolage) : `POST /auth/register`, `/auth/login` → Bearer token opaque ; tenant = `ent_session.etablissement_id`.

**Continuité métier :** `docs/metier-*.md` etc. — pas de reset. Chantier hors prod ScolIA actuelle.

**Labo multi-personas :** `PgEnt::seed_labo_cast` + scénarios + runner `cargo run -p ent-lab` (journée autonome + rapport). Doc : [`../docs/ent-labo-personas.md`](../docs/ent-labo-personas.md).

**API + UI labo minimale :**
```bash
set ENT_DATABASE_URL=postgres://scola:scola_dev_pwd@127.0.0.1:5432/scola
cargo run -p ent-api
# → http://127.0.0.1:8088/  (boutons bootstrap / login / dossier)
# JSON : POST /auth/login { email, password, etablissement_id } → { token }
# Authorization: Bearer <token>
```

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
- Checklist métier : [`../docs/ent-metier-checklist.md`](../docs/ent-metier-checklist.md)

## Commandes

```bash
cd ent-rust
cargo test
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
