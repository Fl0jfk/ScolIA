# ENT Rust — observabilité et PRA (sauvegardes / reprise)

> Ce doc répond à deux questions d'exploitant : « que voit-on quand ça tourne
> mal ? » et « comment prouve-t-on qu'une restauration est fidèle ? ».
> Tout ce qui est annoncé ici est **vérifiable** : tests L1/L2, labo, ou
> `--autotest` des scripts.

## 1. Observabilité : trois signaux, aucun PII

| Signal | Où | Contenu |
|---|---|---|
| **Liveness** | `GET /health` | `{ ok, service, version, git_sha, migration_tag }` — ne touche pas la base |
| **Readiness** | `GET /ready` | + `db: "up"` après un `SELECT 1` ; **503** si la base ne répond pas (retire l'instance du LB) |
| **Métriques** | `GET /metrics` | format texte Prometheus (`version=0.0.4`), registre `crate ent-obs` |
| **Journaux** | stdout | `tracing` : méthode, chemin, statut, latence, `x-request-id` ; `ENT_LOG_FORMAT=json` pour un collecteur |

`migration_tag` est la **dernière migration compilée** (`ent-db/src/migrate.rs`,
constante `MIGRATION_TAG`). Un test (`migration_tag_suit_la_derniere_migration`)
échoue si on ajoute un fichier `0NN_*.sql` sans bumper la constante : le tag ne
peut pas mentir.

### Accès à `/metrics`

| Environnement | Réglage | Comportement |
|---|---|---|
| Production | `ENT_METRICS_TOKEN=<secret>` | `Authorization: Bearer <secret>` exigé ; sinon **401** (+ `WWW-Authenticate`) |
| Poste / labo | `ENT_METRICS=1` | accès libre (scrape local uniquement) |
| Défaut | *(rien)* | **404** — aucune surface d'observabilité exposée |

Le jeton est comparé à **temps constant** (pas de fuite par le temps de réponse).
Aucune métrique ne contient de donnée nominative : uniquement des compteurs, des
durées et des volumétries agrégées.

### Ce qui est mesuré

| Métrique | Type | Sens |
|---|---|---|
| `ent_http_requetes_total{methode,route,statut}` | compteur | requêtes traitées |
| `ent_http_duree_secondes{methode,route}` | histogramme | latence (seaux 1 ms → 60 s, `_sum`/`_count`) |
| `ent_http_requetes_en_cours` | jauge | requêtes en vol |
| `ent_db_pool_connexions{etat="total\|inactives\|maximum"}` | jauge | saturation du pool Postgres |
| `ent_sessions_actives` | jauge | sessions valides (non révoquées, non expirées) |
| `ent_passkeys_total`, `ent_eleves_total`, `ent_etablissements_total` | jauge | volumétrie |
| `ent_mails_outbox{statut}` | jauge | `dry_run` = courrier jamais parti → **alerte principale** |
| `ent_sondes_erreur` | jauge | `1` si la collecte base a échoué au dernier scrape |
| `ent_info{version,git_sha,migration_tag}` | jauge | ce qui tourne réellement |

**Cardinalité bornée** : l'étiquette `route` vient de `MatchedPath` (le gabarit
`/auth/passkeys/{id}`), **jamais** du chemin brut — sinon chaque identifiant
d'élève créerait une série. C'est testé : une requête sur `/personnel/<uuid>`
produit `route="/personnel/{id}"` et l'UUID n'apparaît nulle part dans l'export.

### Alertes minimales suggérées (scrape toutes les 15 s)

| Condition | Interprétation |
|---|---|
| `ent_sondes_erreur == 1` | base injoignable depuis l'application |
| `ent_db_pool_connexions{etat="inactives"} == 0` **et** `total == maximum` (5 min) | pool saturé : requêtes lentes ou fuite de connexions |
| `ent_mails_outbox{statut="dry_run"}` croît (10 min) | SMTP non configuré / envoi coupé : les familles ne reçoivent rien |
| `rate(ent_http_requetes_total{statut=~"5.."}[5m]) > 0` | erreurs serveur |
| `histogram_quantile(0.95, …ent_http_duree_secondes…)` > 2 s | latence dégradée |
| `/ready` en 503 | l'instance doit sortir du load-balancer |

## 2. PRA : sauvegardes, et surtout **preuve de fidélité**

### 2.1 Ce qui sauvegarde quoi (couches)

| Couche | Mécanisme | RPO | RTO |
|---|---|---|---|
| **Infra** | snapshots + PITR de la RDB Scaleway (rétention gérée côté fournisseur) | quelques minutes | dizaines de minutes |
| **Logique (applicatif)** | `scripts/sauvegarde-postgres.sh` → `pg_dump --format=custom` + SHA256 + manifeste + rétention | selon la fréquence (quotidien recommandé) | quelques minutes (base scolaire) |
| **Applications** | image conteneur immuable (`GIT_SHA` injecté) + migrations au démarrage | — | redémarrage |
| **Objets (pièces, PDF, photos)** | bucket S3/Scaleway, versionné (`ent-object`) | selon le bucket | immédiat |

Le **RPO** réel est celui du PITR RDB (minutes) ; le dump logique est la ceinture
et bretelles + la preuve d'intégrité. Décider une restauration reste **humain**.

### 2.2 Le manifeste d'intégrité (`GET /pra/inventaire`)

Un dump qui s'écrit sans erreur ne prouve rien. L'ENT produit donc un
**inventaire d'intégrité** :

- **portée établissement** (HTTP, Direction/Admin) : uniquement les tables
  portant `etablissement_id`, filtrées sur **l'établissement de la session** ;
- **portée globale** (API de service `PgEnt::inventaire_global`, labo/opérateur) :
  toutes les tables métier, hors tables volatiles.

Par table : `count(*)` + `md5` de l'agrégat **ordonné** des digests de chaque
ligne sérialisée en JSON canonique (`jsonb`) — une seule valeur modifiée change
le digest. Plus une **empreinte de schéma** (catalogue `public` :
`table.colonne:type:not null`) : une migration non rejouée est détectée même si
les données sont identiques. Plus `migration_tag`.

**Tables volatiles exclues** (`TABLES_VOLATILES`) : `ent_session`,
`ent_passkey_challenge`, `ent_oidc_state`, `ent_login_guard`,
`ent_password_reset`. Ce sont des états de **sécurité éphémères** : les restaurer
depuis un dump ancien rouvrirait des sessions révoquées ou relâcherait un verrou
anti-bourrage. Hors périmètre, par conception.

### 2.3 Procédure de restauration prouvée

```text
1. GET  /pra/inventaire                      (Direction/Admin)  → inventaire.json
2. scripts/sauvegarde-postgres.sh                               → ent.dump + SHA256
3. restauration (scripts/restauration-postgres.sh, ou snapshot RDB)
4. démarrer une instance pointant la base restaurée
5. POST /pra/inventaire/verifier             (même établissement, corps = inventaire.json)
   → { conforme: true, tables_conformes: N }   preuve ligne à ligne
```

L'étape 5 **refuse** un inventaire dont la portée n'est pas l'établissement de la
session (`403`) : un manifeste d'un autre tenant ne peut pas servir de référence.

### 2.4 Scripts

| Script | Rôle |
|---|---|
| `scripts/pra-lib.sh` | garde-fous partagés : loopback obligatoire, masquage d'URL, rétention, binaires |
| `scripts/sauvegarde-postgres.sh` | dump compressé + SHA256 + `manifeste.txt` + rétention (`ENT_BACKUP_KEEP`, défaut 14) |
| `scripts/restauration-postgres.sh` | restauration `--clean --if-exists` avec `CONFIRM_RESTORE=oui` et vérification SHA256 |
| `scripts/pra-exercice.sh` | exercice complet (dump → base jetable → restauration → comparaison des volumétries → rapport RTO) + `--autotest` |

Garde-fou commun (même philosophie que `scripts/apply-migrations-direct.mjs`) :
**une URL non loopback est refusée** (`code 2`) sauf `ALLOW_PROD_BACKUP=1` posé
explicitement par un humain. Les mots de passe sont masqués dans tous les
journaux.

```bash
# Garde-fous (aucune base, aucun binaire requis) — utilisable en CI
bash scripts/pra-exercice.sh --autotest     # → "Autotest OK"

# Exercice complet en labo
DATABASE_URL=postgres://scola:scola_dev_pwd@127.0.0.1:5432/scola \
  bash scripts/pra-exercice.sh
```

## 3. Preuves automatisées

| Niveau | Test | Ce qu'il prouve |
|---|---|---|
| L0 (unitaire) | `cargo test -p ent-obs` (11 tests) | compteurs/jauges/histogrammes, étiquettes triées et échappées, seaux cumulés, `_sum`/`_count`, concurrence sans perte, rendu déterministe |
| L0 (unitaire) | `migrate::tests::migration_tag_suit_la_derniere_migration` | le tag de schéma publié par `/health` suit le dernier fichier de migration |
| L0 (shell) | `scripts/pra-exercice.sh --autotest` | loopback obligatoire (dont IPv6), refus de la RDB distante, masquage d'URL, remplacement de base, rétention |
| L1 (Postgres) | `pra_inventaire_integrite_et_isolation` | inventaire par établissement, conformité à vide, **dérive détectée ligne à ligne**, cloisonnement (les écritures d'un tenant ne bougent pas l'inventaire d'un autre), garde par rôle, refus d'une portée étrangère, inventaire global |
| L2 (HTTP) | `api_metriques_et_comptage_par_gabarit` | `/metrics` 404/401/200 selon la politique, jeton à temps constant, comptage **par gabarit**, histogramme, sondes base, `ent_info` |
| L2 (HTTP) | `api_pra_inventaire_integrite_et_isolation` | 401 anonyme / 403 prof & parent / 200 Direction, inventaire scellé sur l'établissement, dérive détectée après création d'un élève, nouvel inventaire conforme, refus d'un inventaire étranger |
| Labo | `explorer_observabilite_pra` | parcours complet joué en une passe (métriques, sondes, inventaire, dérive, rôle, global) |

## 4. Trous assumés

| Sujet | État | Raison |
|---|---|---|
| **Traces distribuées (OpenTelemetry)** | non | dépendance lourde ; les journaux corrélés par `x-request-id` + métriques couvrent le diagnostic « quelle route, quel statut, combien de temps ». |
| **Tableaux de bord livrés** (Grafana) | non | ce doc décrit les séries et les seuils d'alerte ; le tableau de bord est propre à l'hébergeur. |
| **Métriques métier par établissement** (`/metrics` est global) | non | éviter la cardinalité et toute fuite inter-tenant dans un endpoint d'infra. L'inventaire par établissement est, lui, **gardé par rôle et cloisonné**. |
| **`pg_dump`/`pg_restore` dans cette VM** | absents (le paquet embarqué ne fournit que `initdb`/`pg_ctl`/`postgres`) | les scripts exigent les utilitaires PostgreSQL et le disent (`code 3`) ; seule la partie applicative de la preuve (inventaire + dérive) est exécutée ici. |
| **Réplication logique / restore cross-région automatisé** | non | hors scope tant que la RDB Scaleway assure PITR + snapshots. |
| **Chiffrement des dumps au repos** | non (recommandé : volume chiffré / bucket chiffré, ou `age`/`gpg` en enveloppe) | à trancher avec l'hébergeur ; le SHA256 détecte l'altération, pas la lecture. |
