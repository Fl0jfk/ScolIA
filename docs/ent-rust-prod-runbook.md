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
| `ENT_DATABASE_MIGRATION_URL` | — (facultatif) | URL du rôle **propriétaire** des tables, utilisée seulement pour les migrations (connexion dédiée, fermée ensuite). Absente : les migrations passent par `ENT_DATABASE_URL` (comportement historique). Si définie, `ENT_DATABASE_URL` doit être un rôle applicatif non propriétaire (`docs/RLS_ISOLATION.md`). Local uniquement hors prod. |
| `ENT_RLS_MODE` | `off` | RLS PostgreSQL pilote (C4) : `off` (rien posé, politiques permissives), `permissif` (pose `app.etablissement_id` dans les transactions des repos santé, psy, sanctions, facturation), `strict` (pose aussi `app.rls_strict`). Recommandé en prod à terme : `strict` + `ALTER ROLE ent_app SET app.rls_strict = 'on'`. Voir `docs/RLS_ISOLATION.md` avant activation. |
| `ENT_API_BIND` | `127.0.0.1:8088` | Adresse d'écoute. En conteneur : `0.0.0.0:8088`. |
| `ENT_ENV` | *(vide)* | Fail-closed : portes de dev ouvertes seulement pour `lab`/`labo`/`dev`/`local`/`test` ; vide, `prod`, `production` ou inconnu → fermées (voir ci-dessous). Mettre `prod` en production. |
| `ENT_ALLOW_REGISTRATION` | — | `1` → réautorise `POST /auth/register` même en prod (provisioning contrôlé). |
| `ENT_ALLOW_LABO` | — | `1` → réautorise `POST /labo/bootstrap` même en prod (démo). |
| `ENT_SESSION_TTL_DAYS` | `14` | Durée de vie d'une session. |
| `ENT_PUBLIC_BASE_URL` | `http://localhost:8088` | Base des liens publics : e-mails (ex. réinitialisation de mot de passe) **et `redirect_uri` par défaut du SSO** (`{base}/auth/oidc/callback`). En prod : l'URL réelle en HTTPS. |
| `ENT_WORKER_STACK_MB` | `16` | Taille de pile des workers tokio (≥ 2). Élargir si des chaînes async très profondes débordent (seed labo en debug). |
| `ENT_LOGIN_MAX_FAILED` | `5` | Échecs avant verrouillage du couple **(compte, IP)**. |
| `ENT_LOGIN_MAX_FAILED_IP` | `50` | Échecs avant verrouillage **par IP** (plus haut : une école partage une IP via NAT). |
| `ENT_LOGIN_WINDOW_MINUTES` | `15` | Fenêtre glissante de comptage. |
| `ENT_LOGIN_LOCK_MINUTES` | `15` | Durée du verrou. |
| `ENT_LOGIN_SLOWDOWN_AFTER` | `5` | Échecs par compte (toutes IP) avant ralentissement progressif. |
| `ENT_LOGIN_SLOWDOWN_BASE_MS` | `200` | Délai au premier échec au-delà du seuil, doublé à chaque échec suivant (`0` = désactivé). |
| `ENT_LOGIN_SLOWDOWN_MAX_MS` | `3000` | Plafond du délai. |
| `ENT_TRUST_PROXY` | *(vide)* | Proxys de confiance devant l'API. Vide / `0` : IP de la socket (défaut sûr). `1` ou `N` : `N` proxys, IP cliente = N-ième entrée **en partant de la droite** de `X-Forwarded-For`. Liste de CIDR (`10.0.0.0/8,fd00::/8`) : remontée de la chaîne de droite à gauche. À régler **exactement** selon l'infrastructure (voir « Limitation de débit »). |
| `ENT_RATE_LIMIT` | *(vide)* | `off` coupe toute la limitation de débit des routes d'authentification. |
| `ENT_RATE_<ROUTE>_IP` / `ENT_RATE_<ROUTE>_ID` | voir tableau ci-dessous | Quota `max/secondes` (`off` = pas de seau). `<ROUTE>` ∈ `LOGIN`, `FORGOT_PASSWORD`, `RESET_PASSWORD`, `PASSKEY_LOGIN_START`, `PASSKEY_LOGIN_FINISH`, `OIDC_START`, `PASSKEY_RECUP`, `PASSKEY_RECUP_CONFIRMER`, `PASSKEY_RECUP_ANNULER`, `PASSKEY_RECUP_FINALISER`. |
| `ENT_RECUPERATION_PASSKEY_DELAI_HEURES` | `24` | Délai de sécurité (heures, borné 1–168, valeur invalide → 24) entre la confirmation d'une récupération de passkey en libre-service et sa finalisation (D-018). Nécessite `ENT_PUBLIC_BASE_URL` et la messagerie. |
| `ENT_TRUST_PROXY` | *(vide)* | `1` → lit l'IP cliente dans `x-forwarded-for` / `x-real-ip` (garde par IP) **et** l'hôte dans `x-forwarded-host` (établissement de l'adresse, C2). À activer **uniquement** derrière un proxy de confiance qui réécrit ces en-têtes. |
| `ENT_TENANT_PAR_HOTE` | `souple` | Établissement déduit de l'adresse (D-012). `strict` : hôte inconnu → 404 (sauf `/health`, `/ready`, `/metrics`), `etablissement_id` du client ignoré, session d'un autre établissement → 401. `souple` : hôte connu → comme `strict`, sinon comportement antérieur (`etablissement_id` fourni par le client). **Production : `strict`** une fois les domaines déclarés. |
| `ENT_MAX_BODY_BYTES` | `26214400` (25 Mio) | Taille max du corps de requête. |
| `ENT_REQUEST_TIMEOUT_SECS` | `120` | Délai max d'une requête (`0` = désactivé) → HTTP 408. |
| `ENT_CORS_ORIGINS` | *(vide)* | Liste d'origines (virgules) ou `*`. Vide → same-origin uniquement. |
| `RUST_LOG` | `info` | Filtre de logs (`tracing_subscriber`). |
| `ENT_OBJECT_DIR` / `ENT_OBJECT_S3_HTTP` / `ENT_MAIL_LIVE` / `ENT_SMS_LIVE` / `ENT_PUSH_LIVE` / `ENT_BANK_LIVE` | *(dry-run)* | Sorties live optionnelles (S3 / SMTP / SMS / push / banque). |
| `ENT_OBJECT_SIGNING_KEY` | *(vide)* | Secret HMAC (≥ 32 octets, aléatoire) des liens courts de fichiers (`GET /fichiers/{jeton}`). **Obligatoire en prod** : sans lui, les routes `…/lien` répondent 503 (fail-closed). En `ENT_ENV` lab/labo/dev/local/test, une clé éphémère est générée. |
| `ENT_OBJECT_LINK_TTL_SECS` | `60` | Durée de vie d'un lien court (plafond 900). |
| `ENT_OBJECT_PUBLIC_BASE_URL` | *(vide)* | Préfixe absolu des liens locaux (ex. `https://ent.exemple.fr`) ; vide → URL relative `/fichiers/…`. |
| `ENT_OBJECT_PRESIGN_MODE` | `local` | `s3` → URL pré-signées SigV4 : `ENT_OBJECT_S3_BUCKET`, `ENT_OBJECT_S3_REGION` (`fr-par`), `ENT_OBJECT_S3_ACCESS_KEY`, `ENT_OBJECT_S3_SECRET_KEY` (secrets : coffre, jamais en dépôt), `ENT_OBJECT_S3_ENDPOINT` (défaut `https://s3.{région}.scw.cloud`), `ENT_OBJECT_S3_VIRTUAL_HOST=1` (virtual-host au lieu de path-style). Configuration incomplète → désactivé. |
| `ENT_RP_ID` / `ENT_RP_ORIGIN` / `ENT_RP_NAME` | dev `localhost` / `http://localhost:3000` / `ScolIA ENT` | Identité du *relying party* WebAuthn pour les **passkeys**. En prod : domaine réel **en HTTPS** (sinon le navigateur refuse WebAuthn). |
| `ENT_RP_ID` | dev `localhost` | Domaine du *relying party* WebAuthn (ex. `scolia.fr`, D-013). **Obligatoire hors labo/test** : domaine public (ni `localhost`, ni IP, ni nom sans point) ; sinon l'API **refuse de démarrer** (fail-closed). |
| `ENT_RP_ORIGIN` / `ENT_RP_ORIGINS` | dev `http://localhost:3000` | Origine(s) autorisées : `ENT_RP_ORIGIN` (une) et/ou `ENT_RP_ORIGINS` (liste séparée par des virgules). **Hors labo/test : au moins une origine `https`, toutes en `https`**, sous le `rp_id` (hôte égal ou sous-domaine), sans chemin ni requête. Les origines natives Android `android:apk-key-hash:<hash base64url>` sont acceptées en plus (au moins une origine web reste requise). iOS (associated domains) utilise l'origine `https` du `rp_id` : rien à ajouter. |
| `ENT_RP_ALLOW_SUBDOMAINS` | `0` | `1` : toute origine sous-domaine d'une origine autorisée est acceptée (ex. `https://lpnb.scolia.fr` pour `https://scolia.fr`, D-012). Par défaut les sous-domaines doivent être listés. Risque : prise de contrôle d'un sous-domaine = passkeys phishables sur celui-ci. |
| `ENT_RP_NAME` | `ScolIA ENT` | Libellé affiché par le navigateur à l'enrôlement. |
| `ENT_SECRETS_KEY` | *(labo/test : clé de dev publique)* | Clé AES-256-GCM des secrets au repos (secret client OIDC) : **32 octets en hex (64 car.) ou base64**. **Obligatoire hors labo/test** (absente/invalide → l'API refuse de démarrer). À conserver hors de la base et des sauvegardes de la base ; sa perte rend les secrets clients illisibles (à ressaisir). Pas de rotation automatique (format `enc:v1:`). **Sert aussi à chiffrer les champs sensibles** (IBAN/BIC/titulaire, fiche santé, dossier psy — D3) par une clé dérivée distincte : sa perte rend ces champs **illisibles et irrécupérables** (conserver la clé hors base, dans un coffre sauvegardé séparément des sauvegardes de la base). |
| `ENT_UPLOAD_MAX_DOCUMENT` | `10m` | Taille maximale (octets, ou suffixe `k`/`m`) d'un document déposé (documents élève, justificatifs, réinscription). Borné à [1 Kio, 64 Mio]. Le plafond de corps JSON des routes de dépôt est ajusté en conséquence. |
| `ENT_UPLOAD_MAX_PHOTO` | `5m` | Taille maximale d'une photo d'élève. |
| `ENT_UPLOAD_MAX_MESSAGE` | `256k` | Taille maximale d'une pièce jointe de messagerie (contenu encore stocké en base : ne pas relever sans migrer vers le magasin d'objets). |
| `ENT_UPLOAD_MAX_PAI` | `256k` | Taille maximale d'une pièce de PAI. |
| `ENT_OIDC_MAX_BYTES` | `1048576` | Taille maximale d'une réponse d'IdP (découverte, JWKS, tokens), min. 1024. |
| `ENT_PASSKEY_CHALLENGE_MINUTES` | `5` | Durée de vie d'un challenge passkey (à usage unique). |
| `ENT_OIDC_TIMEOUT_SECS` | `10` | Délai réseau par appel d'IdP (découverte, échange de code, JWKS). Hors labo/test, les URL d'IdP doivent être en **`https`** et ne pas viser d'adresse interne (loopback, privée, link-local dont métadonnées cloud, CGNAT) après résolution DNS ; redirections limitées à 3, revalidées. |
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

**L4 : sans la feature Cargo `labo` (image de production), `POST /auth/register` et `POST /labo/bootstrap` ne sont pas enregistrés (404) ; `ENT_ALLOW_REGISTRATION` / `ENT_ALLOW_LABO` sont sans effet. Les lignes ci-dessous ne valent que pour un binaire compilé avec `--features labo`.**

Fail-closed : les portes de dev ne s'ouvrent que si `ENT_ENV` (trim, insensible
à la casse) vaut explicitement `lab`, `labo`, `dev`, `local` ou `test`. `ENT_ENV`
absent, vide, `prod`, `production` ou inconnu → fermé. Réouvrable par dérogation
explicite (`=1`), y compris en prod :

- `POST /auth/register` → **403** sauf `ENT_ALLOW_REGISTRATION=1`
  (l'inscription libre n'a pas sa place en prod ; les comptes sont provisionnés).
- `POST /labo/bootstrap` → **403** sauf `ENT_ALLOW_LABO=1`.

### Anti-force brute (login)

Trois mécanismes complémentaires, compteurs **en base** (`ent_login_guard`),
donc partagés entre instances. Chaque tentative est comptée de façon atomique
**avant** la vérification du mot de passe (une seule requête `INSERT … ON
CONFLICT … DO UPDATE … RETURNING`), puis rendue en cas de succès : le compteur
reflète les échecs.

- **par couple (compte, IP)** : au-delà de `ENT_LOGIN_MAX_FAILED` échecs dans la
  fenêtre → verrou `ENT_LOGIN_LOCK_MINUTES` → HTTP **429** (même avec le bon mot
  de passe, *depuis cette IP*). Un tiers qui martèle un compte depuis son IP ne
  bloque donc que lui-même, pas la victime connectée depuis une autre IP.
  Clé `compte-ip:<e-mail>|<ip>` ;
- **par IP** : après `ENT_LOGIN_MAX_FAILED_IP` échecs → même verrou. Freine le
  bourrage d'identifiants réparti sur de nombreux comptes. Le seuil est plus
  haut (NAT d'établissement). Clé préfixée `ip:` ;
- **par compte, toutes IP confondues** : pas de blocage, un **ralentissement
  progressif** (`ENT_LOGIN_SLOWDOWN_*`) qui freine l'attaque distribuée.

La connexion par passkey n'est soumise à aucun de ces compteurs.
Mot de passe correct mais compte non membre de l'établissement : réponse
identique à des identifiants invalides (401 générique ; cause journalisée).
Bornes : e-mail ≤ 254 octets, mot de passe ≤ 256 caractères, refusés (400) avant
tout hachage.

### IP cliente fiable (`ENT_TRUST_PROXY`)

Le serveur est lancé avec `into_make_service_with_connect_info` : l'adresse de la
**socket** est toujours connue et sert par défaut. `X-Forwarded-For` n'est lu que
si `ENT_TRUST_PROXY` déclare des proxys de confiance, et c'est alors l'entrée de
**droite** (ajoutée par notre proxy) qui est retenue, jamais la première (écrite
par le client). Entrées invalides ou chaîne trop courte → repli sur la socket.
`X-Real-IP` n'est plus lu. En IPv6, la clé est le préfixe `/64`. Une valeur
`ENT_TRUST_PROXY` trop petite fait voir l'IP du proxy (tous les clients
partagent alors les mêmes quotas) ; trop grande, elle permet l'usurpation.

### Limitation de débit des routes d'authentification publiques

Seaux à jetons en base (`ent_rate_limit`, table volatile purgée au fil de l'eau),
une requête atomique par contrôle, partagés entre instances. Dépassement :
**429** + `Retry-After` (secondes), message neutre. Clés : (route, IP) et, pour
certaines routes, (route, identifiant normalisé en minuscules) ; empreintes
SHA-256 en base. Défauts (`max/secondes`) :

| Route | `_IP` | `_ID` |
|---|---|---|
| `/auth/login` | `120/60` | *(aucun)* |
| `/auth/forgot-password` | `10/900` | `3/3600` (e-mail) |
| `/auth/reset-password` | `20/900` | `10/900` (jeton) |
| `/auth/passkey/login/start` | `30/300` | *(aucun)* |
| `/auth/passkey/login/finish` | `60/300` | *(aucun)* |
| `/auth/oidc/{slug}/start` | `30/300` | *(aucun)* |
| `/auth/passkey/recuperation` | `5/900` | `2/3600` (e-mail) |
| `/auth/passkey/recuperation/{confirmer,annuler,finaliser}` | `20/900` | `10/900` (jeton) |

Pas de seau par identifiant sur la connexion ni le démarrage passkey : un tiers
épuiserait le seau de sa victime (déni de service). Si la base de limitation est
indisponible, la limitation s'ouvre (journal d'erreur) plutôt que de refuser.

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

### Récupération de passkey en libre-service (D-018)

- `POST /auth/passkey/recuperation` `{ email, etablissement_id }` — public, **202 toujours**. Si le compte appartient à l'établissement et a une passkey : e-mail avec lien `{ENT_PUBLIC_BASE_URL}/recuperation-passkey/confirmer?token=…` (valable 60 min).
- `POST /auth/passkey/recuperation/confirmer` `{ token }` → `200 { effective_at }` ; délai `ENT_RECUPERATION_PASSKEY_DELAI_HEURES` ; e-mail d'alerte avec liens `…/annuler?token=…` et `…/finaliser?token=…`. Alerte journalisée dans chaque établissement de la personne.
- `POST …/annuler` `{ token }` → 204 ; `POST …/finaliser` `{ token }` → 204 après le délai (409 avant), supprime les passkeys et révoque toutes les sessions. Jeton inconnu/rejoué/expiré → 400. Une connexion par passkey pendant le délai annule la demande.
- Après finalisation : connexion par mot de passe (réenrôlement si la passkey est obligatoire pour le rôle) ; sans mot de passe connu, passer par « mot de passe oublié ». Pages web des liens : à fournir côté Leptos.
- Table volatile `ent_recuperation_passkey` (migration 097), exclue des inventaires PRA ; purge des demandes closes > 30 j au démarrage.

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
  (`ent_session`), jamais du corps de requête.
- Routes de connexion (`/auth/login`, `/auth/passkey/login/start`,
  `/auth/oidc/public`, `/auth/oidc/{slug}/start`, `/auth/register` en labo) :
  l'établissement est celui de l'**adresse** (`ent_etablissement_domaine`,
  C2 / D-012) ; l'`etablissement_id` du client n'est lu qu'en mode `souple` et
  sur un hôte inconnu. `GET /auth/etablissement` renvoie la fiche publique
  (id, slug, nom) de l'établissement de l'adresse.
- Une session n'est acceptée que sur l'adresse de son établissement (sinon 401).
- Mise en service d'un établissement (rôle **Admin** de cet établissement,
  journalisé) : `PUT /etablissement/fiche` `{slug, nom}`, puis
  `POST /etablissement/domaines` `{domaine}` (normalisé : minuscules, sans
  port ni point final ; un domaine déjà servi par un autre établissement est
  refusé, 409). `GET` / `DELETE /etablissement/domaines/{domaine}`.
  Désactivation : `ent_etablissement.actif = false` (l'adresse ne résout plus).
- Ordre de bascule : déclarer fiche + domaines de chaque établissement en
  `souple`, vérifier `GET /auth/etablissement` sur chaque adresse, puis passer
  `ENT_TENANT_PAR_HOTE=strict`.
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


## Chiffrement des champs sensibles (D3) — exploitation
- **Au démarrage** l'API chiffre les valeurs encore en clair (IBAN/BIC/titulaire, fiche santé, dossier psy ; `ent_psy_entretien`, `ent_psy_rdv`) : message `champs sensibles rechiffrés : N`. L'opération est idempotente et reprenable ; en cas d'échec un avertissement est journalisé (`rechiffrement des champs sensibles impossible`) et l'API démarre quand même (les lectures d'héritage en clair restent possibles).
- **Rattrapage manuel** : redémarrer l'API (il n'y a pas de CLI séparée) ; la fonction `PgEnt::rechiffrer_champs_sensibles()` renvoie le détail par colonne.
- **Contrôle** (lecture seule, base de **test/préproduction** uniquement) : `SELECT count(*) FROM ent_fiche_sante WHERE note_confidentielle <> '' AND note_confidentielle NOT LIKE 'enc:v1:%'` doit renvoyer 0 (idem pour les autres colonnes de `CHAMPS_CHIFFRES`).
- **Avant la première mise en production** : générer `ENT_SECRETS_KEY` (`openssl rand -hex 32`), la stocker dans le coffre de secrets et planifier la rotation (non implémentée : voir `docs/SOCLE1_SECURITE.md`, limites D3).
