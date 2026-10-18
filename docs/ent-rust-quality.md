# ENT Rust — tests, vérifs, définition de « livré »

Branche de travail : `feature/ent-rust`.  
**Jamais** de validation contre la RDB / conteneur **prod**.

---

## 1. Pyramide de confiance

| Niveau | Quoi | Quand | Où |
|---|---|---|---|
| **L0 — Unitaire domaine** | Invariants, commandes, événements émis (sans I/O) | Chaque PR / chaque `cargo test` | crates `domain_*`, `ent-events` |
| **L1 — Intégration Postgres local** | Persistance + bus d’événements sur `127.0.0.1` | Avant merge vers `dev` si migration SQL | `ENT_DATABASE_URL` locale uniquement (jamais la RDB prod / `.env.local` distant) |
| **L2 — API** | HTTP (axum…) : auth, tenant, cas nominaux + erreurs | Quand le crate API existe | tests `#[tokio::test]` + éventuellement `cargo nextest` |
| **L3 — Front Rust** | Geste utilisateur observable (écran) | Quand UI existe | tests UI / e2e locaux |
| **L4 — Brain** | Handlers de répercussion (changement classe → domaines) | Dès le bus d’événements | tests de scénario « given event → expect side effects » |

Charlemagne / HFSQL = **référence de couverture**, pas oracle de tests automatiques.

---

## 2. Définition de livrable (DoD) — verticale

Un lot n’est « livré » que si :

1. **Code complet** (pas de TODO / stub qui prétend faire le métier).
2. **Annonce** `PRET:` ou `GROS_PRET:` au Pilote (voir `ent-rust-strategy.md`).
3. **Rôle Tests déclenché** dès « prêt » (pas optionnel).
4. **Tests L0 verts** (+ L1 si SQL) — nécessaires **mais pas suffisants**.
5. **Checklist sécurité multi-tenant** (section 2bis) OK.
6. **Geste / preuve observable** : commande domaine, curl API, ou parcours UI rejouable — pas « ça compile ».
7. **Événements métier** documentés si mutation socle élève.
8. **Scénario Brain** si impact cross-domaine.
9. **RGPD** : pas de nouvelle catégorie sensible sans oui Florian ; payloads minimisés.
10. Si `GROS_PRET:` : vague **Perf** et/ou **UI-Garant** / checklist parcours ; écarts corrigés ou tickets explicites.
11. **Pas d’accès prod** ; migrations local / labo seulement.
12. Doc à jour si le contrat change.

Tant que L2/L3 n’existent pas : crates + L0 (+ L1) + sécu + **preuve de comportement** (test d’intégration / scénario) = minimum « vrai ».

### 2bis. Checklist sécurité (chaque livrable données / API)

- [ ] Toute lecture/écriture filtrée par `etablissement_id`
- [ ] Test négatif cross-tenant (autre établissement → refus / NotFound)
- [ ] Events portent `etablissement_id`
- [ ] Pas de secret en dur
- [ ] Scripts test/migrate refusent une URL non locale
- [ ] (API) le client ne choisit pas le tenant librement sans session

---

## 3. Commandes garde-fou

```bash
cd ent-rust
cargo test
cargo test -p ent-obs            # registre de métriques (aucune base requise)
bash scripts/pra-exercice.sh --autotest   # garde-fous de sauvegarde (aucune base requise)
cargo fmt --check   # dès que rustfmt dispo
cargo clippy -- -D warnings   # dès que clippy dispo
```

Postgres local (L1) — même discipline que ScolIA :

- URL uniquement `127.0.0.1`
- Refus explicite d’URL distante Scaleway dans les scripts de test

---

## 4. Ce qu’on vérifie métier (exemples P0)

| Scénario | Attendu |
|---|---|
| Création élève | Snapshot cohérent + event `eleve.created` |
| Changement de classe | Event `scolarite.classe_changed` + hook Brain enregistré + journal `classe_change` |
| Changement de statut | Event `eleve.status_changed` + journal `statut_change` |
| Sortie élève (`sortir`) | Event `eleve.sortie` + `eleve.compte_revoque` (compte/sessions) + grilles repas closes + docs à signer annulés + journal `eleve_sorti` |
| Tenant manquant / mauvais `etablissement_id` | Erreur domaine, pas d’event |
| Conflit Postgres (unicité 23505) | Erreur `DomainError::Conflit` → HTTP **409** (pas 404) |
| Référence / contrainte Postgres (23503 / 23514 / 23502) | `ReferenceInconnue` / `ValeurInvalide` → HTTP **400** |

Les erreurs SQL remontent classées via `ent-db::pg_err` (jamais un « NotFound » fourre-tout pour un conflit).

---

## 5. Lien avec la validation ScolIA actuelle

Les règles workspace (MCP browser, seed admin…) s’appliquent surtout à l’**intranet Next**.  
Pour `feature/ent-rust`, la validation autonome = **cargo test (+ Postgres local)** jusqu’à ce qu’une UI Rust expose un geste utilisateur.

Quand le front Rust existe : même exigence « geste bout-en-bout local », toujours hors prod.

---

## 6. Durcissement runtime (`ent-api`)

Voir le runbook complet : [`ent-rust-prod-runbook.md`](ent-rust-prod-runbook.md).

| Sujet | État | Preuve |
|---|---|---|
| Liveness / readiness | `GET /health` (sans I/O) + `GET /ready` (ping Postgres → 503) | test `ready_checks_db_and_security_headers` |
| En-têtes sécurité | `nosniff` + `SAMEORIGIN` sur chaque réponse | idem |
| Corrélation logs | `x-request-id` généré / repris | idem |
| Panic handler | `CatchPanicLayer` (process stable) | couche globale |
| Anti-DoS corps | `RequestBodyLimitLayer` (25 Mio défaut, `ENT_MAX_BODY_BYTES`) | couche globale |
| Délai requête | `TimeoutLayer` → 408 (`ENT_REQUEST_TIMEOUT_SECS`) | couche globale |
| CORS | opt-in `ENT_CORS_ORIGINS` (défaut same-origin) | `cors_layer_from_env` |
| Arrêt gracieux | `SIGTERM` / `SIGINT` → fin des requêtes en cours | smoke `target/release/ent-api` |
| Portes de dev | `ENT_ENV=prod` ferme `register` / `labo/bootstrap` (403) | tests `gate_*` + smoke prod |
| Force brute | verrou Postgres partagé **par compte** (`ent_login_guard`) → 429 | test `login_locked_after_repeated_failures` |
| Force brute (IP) | verrou par **IP** (clé `ip:`, seuil `ENT_LOGIN_MAX_FAILED_IP`, actif si `ENT_TRUST_PROXY=1`) | tests `garde_login_par_ip`, `extraction_ip_cliente_selon_confiance_proxy` |
| Sessions | TTL configurable + purge boot + révocation autres sessions au changement de mdp | test `api_change_password_et_audit_connexion` |
| Audit d’accès | `ent_journal_modif` `domaine=auth` + `GET /audit/journal` (staff) | idem |
| Provisionnement staff | `POST /etablissement/personnel` (Direction/Admin ; Admin seul ⇒ Admin) | test `api_provisionne_personnel_garde_fous` |
| Réinit. mdp staff | `POST /etablissement/personnel/{id}/reset-mot-de-passe` (+ révocation sessions) | test `api_reinitialise_mot_de_passe_personnel` |
| Réinit. mdp libre-service | `POST /auth/forgot-password` (202 anti-énumération) + `/auth/reset-password` (jeton unique, révocation sessions) | tests `reset_mot_de_passe_par_jeton_usage_unique`, `api_forgot_reset_mot_de_passe_anti_enumeration` |
| Pile des workers | `ENT_WORKER_STACK_MB` (16 défaut) — chaînes async profondes | smoke `/labo/bootstrap` (sinon débordement en debug) |
| Image prod | `ent-rust/Dockerfile` multi-étages (rust → debian slim) | build `--release` local vert |

> **Correctif sécurité (2026-10)** — une ligne de garde `ent_login_guard`
> **neuve** démarrait sa fenêtre au *plancher* (`now − fenêtre`) au lieu de
> *maintenant* : le compteur retombait à 1 à chaque essai et le verrou n'était
> **jamais** atteint pour un compte ou une IP jamais vus (scénario typique du
> bourrage d'identifiants). La fenêtre démarre désormais à `now`. Couvert par
> `api_garde_login_compte_neuf_accumule` (compte neuf) et `garde_login_par_ip`
> (IP neuve, L1) + `api_garde_login_par_ip` (L2).

## 7. Smoke prod (binaire release, base locale)

```bash
cd ent-rust
export ENT_DATABASE_URL='postgres://…@127.0.0.1:5432/scola'
export ENT_API_BIND='127.0.0.1:8099' ENT_ENV=prod
./target/release/ent-api &
curl -s -i http://127.0.0.1:8099/health      # 200 + x-content-type-options + x-request-id
curl -s    http://127.0.0.1:8099/ready        # {"db":"up",…}
curl -so/dev/null -w '%{http_code}\n' -X POST http://127.0.0.1:8099/auth/register -d '{}' \
  -H 'content-type: application/json'         # 403 (inscription fermée en prod)
kill -TERM %1                                  # arrêt gracieux
```

## 8. Copilote (assistant lecture seule)

Assistant en langage naturel, **déterministe** (pas de LLM requis), branché sur
les tables centrales. Réponse calculée + données brutes (`donnees`) pour l'UI et
les tests.

| Question | Intention | Capacité requise |
|---|---|---|
| `combien d'élèves en 4B ?` / `effectif total` | `effectif` | `can_read_effectif` |
| `absences non justifiées [de la 4B] depuis 7 jours` | `absences_non_justifiees` | `can_lire_vs_absences` |
| `factures impayées ?` | `impayes` | `can_manage_facturation` |
| `élèves sans compte ?` | `sans_compte` | `can_manage_eleves` |
| `alertes CPE ouvertes ?` | `alertes_cpe` | `can_gerer_alertes_cpe` |
| `que peux-tu faire ?` | `capacites` | — (staff) |

Preuves :

- **L0** — `crates/ent-db/src/copilote.rs` : analyse d'intention (accents,
  classes `4B`/`2ND`, fenêtre `depuis N jours`), formatage euros (`format_eur`).
- **L1** — `scenarios::copilote_repond_et_respecte_les_roles` : effectif 4B = 3,
  absences seedées, refus impayés au prof, cloisonnement autre tenant → 0.
- **L2** — `api_copilote_ask_et_roles` : 200/400/403 selon rôle et corps ;
  `GET /copilote/capacites`.
- **Labo** — `explorer_copilote` joue 6 questions par rôle + contrôle du refus.
- **UI** — page `/app/copilote` (staff) avec exemples et table de détail.

## 9. Cohérence transverse (tout est-il lié ?)

Carte factuelle des répercussions entre domaines (fait → effet → preuve) :
**`docs/ent-rust-transverse.md`**.

Invariant vérifié : **un seul état produit les mêmes conséquences**, quelle que
soit la porte d'entrée. La cascade de sortie élève est factorisée dans
`PgEnt::appliquer_repercussions_sortie` et branchée sur `sortir` **et** sur la
bascule de statut `Sorti` (périodes closes, grille cantine close, **affectation
internat close**, compte élève délié + sessions révoquées, docs annulés). Elle est
idempotente (rejouer ne republie rien).

Preuves : `scenarios::sortie_eleve_revoque_compte_et_grille`,
`scenarios::bascule_statut_sorti_declenche_la_cascade_transverse`, labo
*vie croisée — un élève quitte l'établissement*.

### 9.1 Spine « publication établissement → famille »

Un ENT ne suppose jamais que la famille verra l'info : toute **publication
destinée à la famille** la **notifie** (outbox mail anti-doublon `(source, kind)`
+ push si appareils enregistrés). Couverts :

| Publication | Notification |
|---|---|
| Bulletin publié | mail + push parents |
| Document à signer publié | mail + push parents (action requise) |
| Facture émise | mail famille (montant) |
| Encaissement | quittance PDF en pièce jointe + mail famille |
| Bourse décidée | mail + push parents |
| Décision de conseil de classe `visible_famille` | mail + push parents |
| Annonce établissement (tous / familles / classe) | mail + push **de chaque famille ciblée** (anti-doublon par destinataire) |
| Carnet / sanction / absence / réinscription | messagerie + mail / SMS / push (déjà en place) |

Preuves : `scenarios::publications_famille_notifient_les_responsables`,
`scenarios::facture_et_bourse_notifient_les_responsables`,
`scenarios::decision_conseil_classe_publiee_notifie_la_famille`,
`scenarios::annonce_etablissement_notifie_les_familles_ciblees`, labo
*spine « publication établissement → famille »* + *annonces établissement →
familles*. Les intégrations live restent **dry-run par défaut**
(`ENT_MAIL_LIVE`, `ENT_SMS_LIVE`, `ENT_PUSH_LIVE`).

### 9.2 Fait VS canonique : les retards ne s'évaporent plus

Un retard pointé à l'appel était un simple **état de ligne**, perdu à la clôture
(sauf en rouvrant l'appel). Il est désormais écrit dans `ent_retard` — fait
canonique, **idempotent par `(appel, élève)`**, distinct de l'absence (aucun
impact sur la file / les compteurs d'absences), avec compteur annuel et vue
famille cloisonnée. Preuves : `scenarios::retard_pointe_a_l_appel_devient_fait_vs_persistant`
(L1), `api_retards_gate_role_validation_et_isolation` (L2), labo *retards*.

### 9.3 Compétences / LSU : le niveau de maîtrise se saisit, il ne se devine plus

L'export LSU déduisait un « domaine » de la matière et un « niveau » du texte d'une
appréciation — un artefact. Le module compétences introduit un vrai modèle : un
**référentiel socle par établissement** (`ent_competence_item`, éditable) et un
**niveau de maîtrise saisi** par `(élève, item, période)`
(`ent_competence_evaluation`) sur l'échelle nationale (insuffisante → très bonne,
codes LSU 1-4). L'export LSU lit ces **évaluations réelles** ; un élève non évalué
sort une ligne « non évalué » (aucun niveau inventé). Le **livret PDF** est déposé
côté famille et notifié ; un livret vide est refusé. La lecture famille réutilise
`actor_can_read_notes` et **vérifie l'appartenance de l'élève au tenant** (un staff
d'un autre établissement reçoit 404). Preuves :
`scenarios::competences_lues_par_la_famille_et_export_lsu_reel` (L1),
`api_competences_gate_role_validation_et_isolation` (L2), labo *compétences*.

### 9.4 Passkeys (WebAuthn) : un 2ᵉ facteur qu'on ne peut pas contourner

Le 2ᵉ facteur est **exclusivement** une passkey (`Touch ID` / `Face ID` / Windows
Hello / clé de sécurité). Pas de TOTP, pas d'application d'authentification : le
secret privé ne quitte jamais l'appareil, le serveur ne garde qu'une clé publique
et vérifie une signature asymétrique — donc rien à intercepter ni à recopier.

Le point sensible n'est pas la cryptographie (portée par `webauthn-rs`) mais la
**garde** : un 2ᵉ facteur qui peut être contourné ne sert à rien. Trois propriétés
sont tenues par construction :

1. **Pas de session avant la passkey.** `PgAuthRepo` a été scindé en
   `verify_credentials` (mot de passe + appartenance) puis `issue_session`, et le
   2ᵉ facteur s'insère **entre les deux** : `login_start_with_ip` renvoie
   `LoginOutcome::PasskeyRequired { token, options, .. }` *sans* session dès que le
   compte a ≥ 1 passkey.
2. **Le chemin interne refuse de contourner.** `PgEnt::login_with_ip` (utilisé par
   le seed et le provisioning) échoue avec `EntDbError::PasskeyRequis` plutôt que
   d'ouvrir la session — impossible de « sauter » le facteur par un appel oublié.
3. **Challenge à usage unique, lié et court.** Stocké en base
   (`ent_passkey_challenge`), il est **consommé** par `DELETE … RETURNING` (le
   rejeu échoue), expire en 5 min (`ENT_PASSKEY_CHALLENGE_MINUTES`) et porte le
   couple `(établissement, utilisateur)` ; finaliser un enrôlement depuis un autre
   établissement est refusé. `cred_id` est unique au monde : un même appareil ne
   s'enrôle pas deux fois. Le compteur d'usage est réécrit après chaque assertion
   (détection de clonage).

La **connexion sans mot de passe** est le même parcours, initié par
`passkey_login_start` (réponse générique si le compte n'existe pas / n'a pas de
passkey : pas d'énumération). La **récupération** après perte d'appareil est
humaine et tracée : `reset_passkeys_admin_for_actor` (Direction/Admin seulement)
retire les passkeys, le mot de passe redevient suffisant, puis l'utilisateur
réenrôle. Enrôlement, suppression, connexions et reset sont **audités** (RGPD).

Preuves : `scenarios::passkeys_enrolement_2fa_passwordless_et_isolation` (L1),
`api_passkeys_2fa_passwordless_et_isolation` (L2, vrai routeur axum), labo
*Passkeys*. Les tests pilotent un **authentificateur logiciel** (`SoftPasskey` de
`webauthn-authenticator-rs`, dev/labo uniquement) : enregistrement et assertion
sont de vrais échanges WebAuthn, pas des simulacres.

### 9.5 SSO OIDC : déléguer l'identité sans déléguer la décision d'accès

Un établissement qui a déjà un annuaire (Google Workspace, Microsoft Entra,
Keycloak…) ne veut pas gérer un mot de passe de plus. Le SSO délègue donc
**l'authentification** à l'IdP — mais l'ENT garde **la décision d'accès** :
c'est lui qui décide quel compte local entre, et avec quel rôle.

Quatre propriétés portent ce module :

1. **La configuration est prouvée, pas saisie.** Déclarer un fournisseur
   (Direction/Admin) déclenche une **découverte réelle** de l'IdP
   (`.well-known/openid-configuration`) ; les endpoints résolus sont stockés
   (`ent_oidc_provider`). Un issuer injoignable ou un document incomplet est un
   **refus explicite** (502/400) : pas de fournisseur à moitié configuré. Effet
   de bord utile — la page de connexion n'appelle plus le réseau.
2. **Le retour de l'IdP est vérifié de bout en bout.** Flux code + **PKCE S256**
   + `state` + `nonce`, `id_token` vérifié **RS256 uniquement** contre le JWKS
   (`kid` inconnu = refus ; `none`/`HS256` = refus), claims `iss`/`aud`/`exp`/
   `nonce` contrôlés. `state`+PKCE+nonce vivent en base (`ent_oidc_state`) et
   sont **consommés** (`DELETE … RETURNING`, TTL 10 min) : un retour rejoué
   échoue — c'est l'anti-CSRF du parcours.
3. **Aucun compte n'est inventé.** Le rattachement se fait par `sub`
   (`ent_oidc_identity` : stable même quand l'e-mail change), puis à la
   **première** connexion par l'e-mail d'un **membre existant** de
   l'établissement. Une identité inconnue est **refusée**
   (`EntDbError::OidcCompteInconnu`) : pas de provisioning implicite, donc pas
   de rôle ni d'établissement deviné. L'appartenance à l'établissement est
   re-vérifiée après résolution, et la session est **auditée** (`connexion_sso`).
4. **Le secret ne circule pas.** L'API ne renvoie jamais `client_secret` ; la
   vue publique (page de connexion) ne contient que `slug` + `libellé` — ni
   issuer, ni client_id.

Et une garde anti-contournement : **le SSO ne remplace pas un 2ᵉ facteur déjà
enrôlé**. Si le compte local possède ≥ 1 passkey, `oidc_complete` renvoie
`PasskeyRequired` (challenge à usage unique) au lieu d'une session : sinon il
suffirait de cliquer « SSO » pour esquiver sa passkey. Le SSO est un moyen
d'**authentifier**, pas un moyen de **désactiver** la politique de sécurité du
compte.

Le tout est testé sans réseau : `crate ent-oidc` embarque un **IdP en mémoire**
(clé RSA éphémère, JWKS, découverte, signature RSA-SHA256) et vérifie même le
PKCE comme un vrai IdP. Preuves : `scenarios::sso_oidc_rattachement_et_refus_des_inconnus`
(L1), `api_sso_oidc_connexion_et_isolation` (L2, vrai routeur axum), labo
*SSO OIDC*.


### 9.6 Observabilité et PRA : voir l'incident, prouver la restauration

Deux questions d'exploitant, traitées avec la même exigence de preuve :

1. **Que voit-on quand ça tourne mal ?** Trois signaux seulement, mais réels :
   `/health` (vif, `migration_tag` inclus — le tag ne peut pas mentir, un test
   échoue si un fichier de migration n'est pas répertorié), `/ready` (503 quand
   la base ne répond plus → l'instance sort du load-balancer), et `/metrics`
   (format Prometheus). Le registre (`crate ent-obs`, **zéro dépendance**) ne
   publie que des compteurs et des durées : requêtes par **gabarit de route**
   (`MatchedPath`, jamais le chemin brut — sinon chaque identifiant d'élève
   créerait une série), histogramme de latence, requêtes en vol, saturation du
   pool Postgres, sessions actives, outbox courrier par statut (`dry_run` =
   jamais parti : alerte principale), passkeys/élèves/établissements, et
   `ent_sondes_erreur` si la collecte base échoue. Aucune donnée nominative.
   `/metrics` est **désactivé par défaut** (404) : activé par `ENT_METRICS_TOKEN`
   (Bearer, comparaison à temps constant) en production, `ENT_METRICS=1` pour un
   scrape local. Journaux JSON (`ENT_LOG_FORMAT=json`) corrélés par
   `x-request-id`.

2. **Que prouve une sauvegarde ?** Rien, tant qu'on n'a pas prouvé que la base
   restaurée est **identique ligne à ligne**. D'où l'**inventaire d'intégrité**
   (`GET /pra/inventaire`, Direction/Admin) : par table portant
   `etablissement_id`, `count(*)` + `md5` de l'agrégat **ordonné** des digests de
   chaque ligne sérialisée en JSON canonique, plus une **empreinte de schéma**
   (catalogue) et le `migration_tag`. Après restauration,
   `POST /pra/inventaire/verifier` recalcule et compare
   (`{ conforme, ecarts[], tables_absentes[], tables_en_plus[] }`). Une seule
   valeur modifiée fait échouer la vérification **sur la bonne table** — ce qui
   prouve la fidélité, et ce qui détecte une restauration partielle.

Trois garde-fous portent la sécurité de ce module :

- **Cloisonnement** : la portée est **scellée sur l'établissement de la
  session** (jamais un paramètre d'appel) et un inventaire d'un autre
  établissement est **refusé** (`InventairePorteeEtrangere` → 403). Le test L1
  vérifie en plus qu'une écriture dans le tenant B ne bouge pas l'inventaire du
  tenant A.
- **Tables volatiles exclues** (`ent_session`, `ent_passkey_challenge`,
  `ent_oidc_state`, `ent_login_guard`, `ent_password_reset`) : restaurer un état
  de sécurité éphémère rouvrirait des sessions révoquées ou relâcherait un verrou
  anti-bourrage. Hors périmètre, par conception, et vérifié par test.
- **Opérations de sauvegarde gardées** : `scripts/sauvegarde-postgres.sh`,
  `restauration-postgres.sh`, `pra-exercice.sh` refusent toute URL non loopback
  (`code 2`) sans `ALLOW_PROD_BACKUP=1`, masquent les mots de passe, vérifient le
  SHA256 du dump et appliquent la rétention. La logique de garde est
  **auto-testée** (`bash scripts/pra-exercice.sh --autotest` → `Autotest OK`),
  donc exécutable sans binaire PostgreSQL.

Preuves : `cargo test -p ent-obs` (11 tests : seaux cumulés, `_sum`/`_count`,
étiquettes triées et échappées, concurrence sans perte, rendu déterministe),
`pra_inventaire_integrite_et_isolation` (L1 : conformité à vide, **dérive
détectée** puis retour à la conformité, cloisonnement, rôles, portée étrangère,
inventaire global), `api_metriques_et_comptage_par_gabarit` et
`api_pra_inventaire_integrite_et_isolation` (L2 : 404/401/200, gabarit
`/personnel/{id}` sans UUID dans l'export, 401/403/200 par rôle, dérive et
cloisonnement via HTTP), labo *observabilité/PRA*, et
`scripts/pra-exercice.sh --autotest`. Détail complet :
`docs/ent-rust-observabilite-pra.md`.
