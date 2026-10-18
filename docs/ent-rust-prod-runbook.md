# ENT Rust — runbook production (`ent-api`)

Service : binaire `ent-api` (workspace `ent-rust/`), HTTP **axum**, auth **100 % Rust**
(Argon2id + sessions Postgres). Base : PostgreSQL `ent_*`, migrations **embarquées
dans le binaire** (`include_str!`) et appliquées au démarrage.

> Règle absolue : en local / CI, `ENT_DATABASE_URL` **doit** pointer sur
> `127.0.0.1` / `localhost` / `::1`. `PgEnt::connect` refuse tout autre hôte
> (test unitaire dédié). Aucune migration / seed contre la RDB prod ScolIA.

---

## Démarrage

```bash
# Conteneur (voir ent-rust/Dockerfile)
docker build --build-arg GIT_SHA="$(git rev-parse HEAD)" -t ent-api:local ent-rust
docker run --rm -p 8088:8088 \
  -e ENT_DATABASE_URL='postgres://user:pass@host:5432/db' \
  -e ENT_API_BIND='0.0.0.0:8088' \
  -e ENT_ENV=prod \
  ent-api:local
```

Le `GIT_SHA` est injecté au build et exposé par `GET /health` et `GET /ready`
(`{ git_sha, version, … }`) → vérification du déploiement.

Ou directement le binaire :

```bash
cd ent-rust
ENT_DATABASE_URL='postgres://…' ENT_API_BIND='0.0.0.0:8088' ENT_ENV=prod \
  ./target/release/ent-api
```

Au boot : migration (verrou `pg_advisory_lock`, une seule instance à la fois),
purge des sessions expirées, puis `install_brain_p0`. Arrêt : `SIGTERM` / `SIGINT`
→ **fermeture gracieuse** (les requêtes en cours se terminent).

---

## Variables d'environnement

| Variable | Défaut | Rôle |
|---|---|---|
| `ENT_DATABASE_URL` | — (requis) | Postgres `ent_*`. Local uniquement hors prod. |
| `ENT_API_BIND` | `127.0.0.1:8088` | Adresse d'écoute. En conteneur : `0.0.0.0:8088`. |
| `ENT_ENV` | *(vide)* | `prod` / `production` → ferme les portes de dev (voir ci-dessous). |
| `ENT_ALLOW_REGISTRATION` | — | `1` → réautorise `POST /auth/register` même en prod (provisioning contrôlé). |
| `ENT_ALLOW_LABO` | — | `1` → réautorise `POST /labo/bootstrap` même en prod (démo). |
| `ENT_SESSION_TTL_DAYS` | `14` | Durée de vie d'une session. |
| `ENT_PUBLIC_BASE_URL` | `http://localhost:8088` | Base des liens publics : e-mails (ex. réinitialisation de mot de passe) **et `redirect_uri` par défaut du SSO** (`{base}/auth/oidc/callback`). En prod : l'URL réelle en HTTPS. |
| `ENT_WORKER_STACK_MB` | `16` | Taille de pile des workers tokio (≥ 2). Élargir si des chaînes async très profondes débordent (seed labo en debug). |
| `ENT_LOGIN_MAX_FAILED` | `5` | Échecs avant verrouillage **par compte**. |
| `ENT_LOGIN_MAX_FAILED_IP` | `50` | Échecs avant verrouillage **par IP** (plus haut : une école partage une IP via NAT). |
| `ENT_LOGIN_WINDOW_MINUTES` | `15` | Fenêtre glissante de comptage. |
| `ENT_LOGIN_LOCK_MINUTES` | `15` | Durée du verrou. |
| `ENT_TRUST_PROXY` | *(vide)* | `1` → lit l'IP cliente dans `x-forwarded-for` / `x-real-ip` (garde par IP). À activer **uniquement** derrière un proxy de confiance. |
| `ENT_MAX_BODY_BYTES` | `26214400` (25 Mio) | Taille max du corps de requête. |
| `ENT_REQUEST_TIMEOUT_SECS` | `120` | Délai max d'une requête (`0` = désactivé) → HTTP 408. |
| `ENT_CORS_ORIGINS` | *(vide)* | Liste d'origines (virgules) ou `*`. Vide → same-origin uniquement. |
| `RUST_LOG` | `info` | Filtre de logs (`tracing_subscriber`). |
| `ENT_OBJECT_DIR` / `ENT_OBJECT_S3_HTTP` / `ENT_MAIL_LIVE` / `ENT_SMS_LIVE` / `ENT_PUSH_LIVE` / `ENT_BANK_LIVE` | *(dry-run)* | Sorties live optionnelles (S3 / SMTP / SMS / push / banque). |
| `ENT_RP_ID` / `ENT_RP_ORIGIN` / `ENT_RP_NAME` | dev `localhost` / `http://localhost:3000` / `ScolIA ENT` | Identité du *relying party* WebAuthn pour les **passkeys**. En prod : domaine réel **en HTTPS** (sinon le navigateur refuse WebAuthn). |
| `ENT_PASSKEY_CHALLENGE_MINUTES` | `5` | Durée de vie d'un challenge passkey (à usage unique). |
| `ENT_OIDC_TIMEOUT_SECS` | `10` | Délai réseau par appel d'IdP (découverte, échange de code, JWKS). |
| `ENT_OIDC_STATE_MINUTES` | `10` | Durée de vie d'un parcours SSO (`state`/PKCE/nonce, à usage unique). |
| `ENT_METRICS_TOKEN` | *(vide)* | Jeton Bearer exigé par `GET /metrics`. **Recommandé en prod.** Sans jeton et sans `ENT_METRICS` → `404`. |
| `ENT_METRICS` | *(vide)* | `1` → `/metrics` en accès libre (scrape **local** uniquement, jamais exposé à Internet). |
| `ENT_LOG_FORMAT` | `text` | `json` → journaux structurés (collecteur Loki / Cockpit) ; sinon texte lisible. |
| `ENT_BACKUP_DIR` | `ent-rust/backups` | Répertoire des sauvegardes logiques (`scripts/sauvegarde-postgres.sh`). |
| `ENT_BACKUP_KEEP` | `14` | Nombre de sauvegardes conservées (rétention glissante). |
| `ENT_PG_BIN` | *(PATH)* | Répertoire des binaires PostgreSQL (`pg_dump`, `pg_restore`, `psql`…). |
| `ALLOW_PROD_BACKUP` | *(vide)* | `1` → autorise les scripts de sauvegarde/restauration sur une base **non loopback** (validation humaine explicite). |
| `CONFIRM_RESTORE` | *(vide)* | `oui` → exécute la restauration destructive (`scripts/restauration-postgres.sh`). |

---

## Santé & supervision

| Endpoint | Sens | Contenu |
|---|---|---|
| `GET /health` | *liveness* (pas d'I/O) | `{ ok, service, version, git_sha, migration_tag }` |
| `GET /ready` | *readiness* | ping Postgres + `db: "up"` et `migration_tag` ; `503` si la base ne répond pas |
| `GET /metrics` | métriques Prometheus (texte) | 404 par défaut, 401 sans jeton si `ENT_METRICS_TOKEN`, 200 sinon |

- `x-request-id` est présent sur **chaque** réponse (généré si absent) ; il
  apparaît aussi dans les logs de requête → corrélation log ↔ réponse.
- En-têtes de sécurité posés globalement : `x-content-type-options: nosniff`,
  `x-frame-options: SAMEORIGIN`.
- Un panic dans un handler est capturé (le process ne tombe pas).
- Docker `HEALTHCHECK` appelle `GET /health`.

### Métriques (`/metrics`)

Séries publiées : `ent_http_requetes_total{methode,route,statut}` (la route est
le **gabarit** `/auth/passkeys/{id}`, pas le chemin brut),
`ent_http_duree_secondes` (histogramme), `ent_http_requetes_en_cours`,
`ent_db_pool_connexions{etat}`, `ent_sessions_actives`, `ent_mails_outbox{statut}`,
`ent_passkeys_total`, `ent_eleves_total`, `ent_etablissements_total`,
`ent_sondes_erreur`, `ent_info{version,git_sha,migration_tag}`. Aucune donnée
nominative. Alertes suggérées et détail : `docs/ent-rust-observabilite-pra.md`.

```bash
curl -s -H "Authorization: Bearer $ENT_METRICS_TOKEN" https://ent.example/metrics | head
```

### PRA (sauvegardes)

| Étape | Commande / route |
|---|---|
| Inventaire de référence | `GET /pra/inventaire` (Direction/Admin) → `inventaire.json` |
| Sauvegarde logique | `DATABASE_URL=… bash scripts/sauvegarde-postgres.sh` (loopback) |
| Restauration | `CONFIRM_RESTORE=oui DATABASE_URL=… bash scripts/restauration-postgres.sh --depuis <ent.dump> --base <cible>` |
| Preuve de fidélité | `POST /pra/inventaire/verifier` (corps = `inventaire.json`) → `{ conforme: true }` |
| Exercice complet | `bash scripts/pra-exercice.sh` (dump → base jetable → restauration → volumétries → RTO) |
| Garde-fous seuls | `bash scripts/pra-exercice.sh --autotest` → `Autotest OK` |

RPO = PITR de la RDB Scaleway (minutes) ; le dump logique est la ceinture et
bretelles **et** la source de la preuve d'intégrité. Une URL non loopback est
refusée sans `ALLOW_PROD_BACKUP=1`. Procédure complète :
`docs/ent-rust-observabilite-pra.md`.

---

## Sécurité

### Profil prod (`ENT_ENV=prod`)

Fermé par défaut, réouvrable par dérogation explicite :

- `POST /auth/register` → **403** sauf `ENT_ALLOW_REGISTRATION=1`
  (l'inscription libre n'a pas sa place en prod ; les comptes sont provisionnés).
- `POST /labo/bootstrap` → **403** sauf `ENT_ALLOW_LABO=1`.

### Anti-force brute (login)

Deux verrous complémentaires, compteurs **en base** (`ent_login_guard`), donc
partagés entre instances :

- **par compte** : après `ENT_LOGIN_MAX_FAILED` échecs dans la fenêtre → verrou
  `ENT_LOGIN_LOCK_MINUTES` → HTTP **429** (même avec le bon mot de passe) ;
- **par IP** : après `ENT_LOGIN_MAX_FAILED_IP` échecs → même verrou. Freine le
  bourrage d'identifiants réparti sur de nombreux comptes. Le seuil est plus
  haut (NAT d'établissement) et un succès ne rend **pas** son budget à l'IP.
  Clé préfixée `ip:` dans la même table.

L'IP cliente n'est lue que si `ENT_TRUST_PROXY=1` (en-têtes `x-forwarded-for`,
repli `x-real-ip`, valeur bornée à 45 caractères) : sans proxy de confiance, un
client pourrait falsifier ces en-têtes, donc la garde par IP est **désactivée**
par défaut.

- e-mail inconnu : un hash factice est vérifié pour lisser le temps de réponse
  (anti-énumération) ; les compteurs inactifs > 1 jour sont purgés.

### Sessions

- Token opaque ; seul le SHA-256 est stocké (`ent_session`).
- TTL configurable ; purge des expirées au démarrage.
- `POST /auth/logout` révoque la session courante.
- `POST /auth/password` (authentifié) change le mot de passe et **révoque les
  autres sessions** de l'utilisateur (la session appelante reste valide).

### Réinitialisation de mot de passe (libre-service)

- `POST /auth/forgot-password` `{ email }` — public. Réponse **toujours 202**,
  qu'un compte existe ou non (anti-énumération). Si un compte correspond, un
  jeton à usage unique (TTL 30 min, hash SHA-256 en base `ent_password_reset`)
  est créé et un lien est envoyé par e-mail (dry-run ou SMTP live) :
  `{ENT_PUBLIC_BASE_URL}/reset-mot-de-passe?token=…`.
- `POST /auth/reset-password` `{ token, new_password }` — public. Consomme le
  jeton (atomique, non rejouable), applique la politique de mot de passe et
  **révoque toutes les sessions** de l'utilisateur. Jeton invalide / expiré ou
  mot de passe trop court → **400**.
- Pages web publiques : `/mot-de-passe-oublie` et `/reset-mot-de-passe?token=…`.
- Les jetons expirés / consommés sont purgés au démarrage ; l'action est
  journalisée (`domaine=auth`).

### Audit d'accès (RGPD)

Chaque connexion / déconnexion / changement de mot de passe écrit une entrée
`ent_journal_modif` avec `domaine = 'auth'` (`visible_famille = false`), traçant
l'utilisateur et son rôle. Aucun journal n'est écrit sur un simple échec
(dérivable de `ent_login_guard`), pour éviter l'amplification d'écriture.

Lecture : `GET /audit/journal?domaine=auth&limit=100` — staff habilité
(`can_manage_eleves` ou `can_read_all_eleves`).

### Copilote (assistant lecture seule)

`POST /copilote/ask` `{ question }` — personnel uniquement. Réponse **calculée**
(intentions déterministes : effectif, absences non justifiées, factures non
soldeées, élèves sans compte, alertes CPE) sur les données de l'établissement de
la **session**. Aucune écriture.

- `GET /copilote/capacites` → liste des questions supportées (aide UI).
- Chaque intention revérifie la **capacité du rôle** : une question hors droits
  renvoie `{ refusee: true }` (HTTP 200), pas une fuite. Parent / élève → **403**.
- Cloisonnement inter-tenant : tout comptage est filtré par `etablissement_id`
  (test négatif sur un établissement inexistant → compteurs à 0).

### Provisionnement des comptes personnel

L'inscription libre étant fermée en prod, les comptes **staff** sont ouverts via :

`POST /etablissement/personnel` `{ email, role, display_name? }` — session requise.

- Rôles autorisés : **Direction** et **Admin**.
- Seul un **Admin** peut créer un **Admin** ou une **Direction**.
- Un mot de passe temporaire est généré (≥ 20 car.) et renvoyé **une seule fois**
  ; l'utilisateur le change ensuite via `POST /auth/password`.
- Rôle cible = staff uniquement (`parent` / `eleve` refusés) ; doublon d'e-mail → 409.
- Journalisé (`domaine=etablissement`, action `personnel_provisionne`).

Réinitialisation par un tiers habilité :
`POST /etablissement/personnel/{id}/reset-mot-de-passe` (Direction / Admin) →
nouveau mot de passe temporaire **et révocation de toutes les sessions** de la
cible. Mêmes règles de rôle que le provisionnement ; l'auto-réinitialisation est
refusée (utiliser `POST /auth/password`).

Les comptes **élève** ont leur propre flux (`POST /eleves/{id}/compte/inviter`,
`POST /eleves/comptes/provisionner-lot`).

### Multi-tenant

- Le `etablissement_id` provient **exclusivement** de la session
  (`ent_session`), jamais du corps de requête (sauf `register` / `login`,
  contrôlés par l'appartenance).
- Toute table métier est filtrée par `etablissement_id`.

---

## Exploitation

- **Migrations** : idempotentes, rejouées à chaque boot sous verrou advisory.
  Une migration cassée bloque le démarrage (échec visible = pas de service à
  moitié migré).
- **Sauvegardes** : responsabilité de l'infra Postgres (snapshot RDB). Le binaire
  ne les gère pas.
- **Rotation** : changer `ENT_DATABASE_URL`, redémarrer (arrêt gracieux).
- **Logs** : `tracing` JSON-friendly via `RUST_LOG`. Chaque requête : méthode,
  chemin, statut, latence.

---

## Reste à décider / hors périmètre `ent-api`

- **Prorata facturation** au changement de classe / sortie : règle
  **d'établissement**, pas une décision SaaS — à trancher par Florian, puis à
  porter comme paramètre d'établissement.
- **« Mot de passe oublié »** : **fait** — libre-service par jeton tokenisé
  (e-mail dry-run ou SMTP live), voir §Sécurité. L'activation d'envoi réel
  requiert `ENT_MAIL_LIVE=1` + config SMTP.
- **Limitation par IP** : le verrou actuel est par compte (`ent_login_guard`) ;
  un quota global par IP serait un plus (proxy / connect-info).
- **Intégrations live** (SMTP / SMS / push / banque / S3) : câblées mais en
  **dry-run** par défaut — activation par variables d'environnement.
- **Clients natifs** (SwiftUI / Kotlin) et client web dédié : hors workspace
  `ent-rust` (ils consomment cette API).
- **Traces distribuées / tableaux de bord** : non livrés — journaux corrélés
  (`x-request-id`) + métriques suffisent ; le dashboard est propre à l'hébergeur.
- **Chiffrement des dumps au repos** : non implémenté — à assurer par le volume /
  bucket chiffré (ou une enveloppe `age`/`gpg` autour de `pg_dump`).
- **Restauration cross-région automatisée** : hors périmètre tant que la RDB
  Scaleway assure snapshots + PITR.
