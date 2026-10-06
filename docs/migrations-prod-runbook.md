# Runbook — migrations PostgreSQL (prod / Scaleway)

## Contexte

- Les migrations vivent dans `drizzle/*.sql`, ordre = `drizzle/meta/_journal.json`.
- **Au démarrage du conteneur** : `scripts/docker-entrypoint.sh` exécute `scripts/apply-migrations-direct.mjs` puis `server.js`. Échec migration → processus quitte en erreur (pas de serveur HTTP).
- `drizzle-kit migrate` ne doit plus être utilisé en prod : avec des champs `when` en doublon dans le journal, il **ignore silencieusement** la deuxième migration d’un même timestamp.

## Incident oct. 2026 (LPNB)

Migrations appliquées manuellement en prod (schéma OK) mais **non journalisées** :

| Tag SQL | Remarque |
|---------|----------|
| `0050_eleve_core_socle` | table `passage`, socle accueil |
| `0052_socle_faits_administratifs` | idem socle |
| `0053_compta_etablissement` | compta établissement |
| `0054_infirmerie_bloc` | infirmerie |
| `0055_sante_pai` | santé PAI |
| `0056_sante_inaptitude_eps` | inaptitude EPS |
| `0057_famille_messaging` | messagerie famille |
| `0058_famille_messaging_pieces_matrice` | pièces matrice |

Déjà présentes en schéma avant fix : `0051_travel_participant_eleve_id`, `0055_scolia_conversations`, etc.

**Prochain déploiement** : le migrateur rejoue ces fichiers (SQL idempotent `IF NOT EXISTS` / `DO $$ … EXCEPTION`) puis insère le hash SHA256 dans `drizzle.__drizzle_migrations`. Aucun `DROP` / `DELETE` métier dans ces scripts.

## Vérification après déploiement (psql)

```sql
-- 1) Dernières migrations journalisées (hash = SHA256 du fichier .sql)
SELECT id, left(hash, 12) AS hash_prefix, created_at
FROM drizzle.__drizzle_migrations
ORDER BY id DESC
LIMIT 15;

-- 2) Présence des objets critiques accueil / socle
SELECT to_regclass('public.passage') AS passage,
       to_regclass('public.infirmerie_passage') AS infirmerie_passage;

-- 3) Tags attendus (hash ≠ nom de tag ; comparer au repo avec sha256sum drizzle/<tag>.sql)
-- Exemple local : sha256sum drizzle/0050_eleve_core_socle.sql
```

Comparer le hash affiché en base au SHA256 du fichier dans le commit déployé. Tous les tags listés dans `_journal.json` doivent avoir une ligne dont le hash correspond au fichier (ou être antérieurs au fix avec hash tag-only legacy — alors un redéploiement ajoute la ligne SHA256).

## CI locale

```bash
npm run db:validate-migrations
npm run test:drizzle-migrations
```

Job **`migrations-fresh-db`** (GitHub Actions) : Postgres vierge → `apply-migrations-direct.mjs` jusqu’au dernier tag du journal (inclut bootstrap `0002` / `0056` sans changer les hash prod).

## Migration manuelle d’urgence (hors pipeline)

Uniquement avec validation humaine :

```bash
ALLOW_PROD_MIGRATION=1 DATABASE_URL='…' node scripts/apply-migrations-direct.mjs
```

Ne pas utiliser `drizzle-kit migrate` sur la RDB prod.

## Preuve déploiement (HTTP)

Endpoint public (sans auth) :

```bash
curl -s "${NEXT_PUBLIC_APP_URL%/}/api/health"
# → {"ok":true,"gitSha":"<commit>","migrationTag":"0061_vs_absence_eleve_annulee"}
```

Le workflow GitHub **Deploy container** interroge `/api/health` jusqu’à ce que `gitSha` corresponde au commit poussé (timeout ~10 min).
