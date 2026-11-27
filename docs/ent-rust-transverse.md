# ENT Rust — carte des répercussions transverses

> Réponse factuelle à « est-ce que tout est lié ? ». Document **extrait du code**
> (`crates/ent-db/src/service.rs`, `crates/ent-events`, `crates/ent-brain`,
> `crates/ent-lab`), pas déclaratif. Toute ligne « preuve » est vérifiable par un
> test L1/L2 ou par le labo.

## 1. Le socle : deux mécanismes, pas de silos

| Mécanisme | Où | Rôle |
|---|---|---|
| **Tables canoniques partagées** | migrations `ent_*` | Un fait n'a **qu'une** écriture (`ent_eleve`, `ent_absence`, `ent_repas_grille`…). Les écrans **lisent**, ils ne recopient pas. |
| **Bus d'événements métier** | `crates/ent-events` → `ent_metier_event` | Chaque geste publie un `MetierEvent` (`149` types déclarés). Consommé par le journal, le dossier élève (`events_for_eleve`) et le **Brain**. |
| **Brain (handlers)** | `crates/ent-brain` | `react(event) -> Vec<BrainAction>` puis application d'écritures **additionnelles** (pas de copie de `classe`/`statut`). |
| **Cascades de service** | `PgEnt::*` | Répercussions réelles entre domaines (notifications, fermetures, révocations) déclenchées par le geste socle. |
| **Spine de notification familles** | `PgEnt::notify_parents_{mail,push,sms}` | Toute **publication destinée à la famille** (bulletin, doc à signer, facture, bourse, conseil, carnet…) pousse une notification, pas seulement un événement interne. Outbox anti-doublon `(source, kind)`. Les **diffusions** multi-familles (annonces) utilisent `notify_parents_*_broadcast` : anti-doublon **par destinataire** (`source, kind, élève`). |

Mesure sur la base : **404 méthodes de service**, dont **139 (≈34 %)** touchent
≥ 2 domaines (écriture + effet). Presque toutes publient un événement → le
« lié » est structurel, pas optionnel.

## 2. Matrice par domaine → répercussions transverses

| Domaine (geste) | Répercussions effectives | Preuve |
|---|---|---|
| **Élève — création** | période de scolarité ouverte + event `eleve.created` | `create_eleve` |
| **Élève — bascule datée** (classe/régime/mef/site) | effectif daté, EDT élève, appels, dossier, event `scolarite.*_changed`, journal `*_change` | `change_attribut`, test `vie_etablissement_*`, labo *vie croisée changement de classe* |
| **Élève — sortie** (daté **ou** bascule de statut) | périodes closes, `eleve.sortie`, **grille cantine close**, **affectation internat close**, compte élève délié + **sessions révoquées**, docs à signer annulés, journal | `appliquer_repercussions_sortie`, tests `sortie_eleve_*` + `bascule_statut_sorti_*`, labo *vie croisée sortie* |
| **Appel — clôture** | absents → `ent_absence` (canonique) → seuil → signal VS | `clore_appel` + `react(APPEL_CLOSED)` |
| **Absence (déclarée / seuil)** | signal VS ouvert + **fil de messagerie famille** + mail + push parents (une fois par signal) | `apres_absence_ecrite` |
| **Justificatif déposé** | push staff (file VS) | `justifier_absence` |
| **Sanction prononcée** | event + **SMS famille** (points VS via types/commission) | `prononcer_sanction` |
| **Carnet (entrée)** | `fil` messagerie + mail + SMS + push parents | `creer_carnet_entree` |
| **Messagerie (ouverture/réponse)** | push aux destinataires (famille) | `ouvrir_fil_for_actor`, `repondre_fil_for_actor` |
| **Cantine — grille/bourse/badge** | passages facturables, facture badge, volumes cuisine, historique famille | `poser_grille_repas`, `scan_badge_repas_for_actor`, labo *cantine* |
| **Bourse — décision** | statut (demandée/acceptée/refusée) + déductibilité facture + **mail + push parents** | `decider_bourse`, test `facture_et_bourse_notifient_les_responsables` |
| **Notes — bulletin publié** | PDF stocké (S3) + journal famille + **mail + push parents** | `publier_bulletin_for_actor`, test `publications_famille_notifient_les_responsables` |
| **Conseil de classe — décision publiée** | mention + `visible_famille` + **mail + push parents** (uniquement si publiée) | `decider_dossier_conseil_for_actor`, test `decision_conseil_classe_publiee_notifie_la_famille` |
| **Continuum — réinscription** | mail + push parents, acompte facturé, docs exigés | `confirmer_reinscription_for_parent` |
| **Facturation — facture/échéance/prélèvement** | échéances, SEPA (mandat, prélèvement, rejet), **mail famille à l'émission**, **quittance PDF notifiée à l'encaissement**, **rappels → messagerie** | `creer_facture_*`, `emettre_facture_for_actor`, `encaisser_facture_for_actor`, `planifier_prelevement_*`, `lancer_rappels_echeances_for_actor` |
| **Internat** | chambre, affectation, présences, suivi **visible famille**, clôture à la sortie | `affecter_eleve_internat_for_actor`, `appliquer_repercussions_sortie` |
| **Infirmerie / Psy / PAI** | fiche santé, PDF, ordonnances (S3), courrier orientation **SMTP**, lecture famille cloisonnée | `creer_passage_infirmerie_*`, `envoyer_courrier_orientation_psy_for_actor` |
| **Docs à signer** | publication CPE → **mail + push parents**, signature parent, annulation à la sortie | `publier_doc_a_signer_*`, `signer_doc_a_signer_*` |
| **Annonce établissement** | diffusion direction → **mail + push de chaque famille ciblée** (anti-doublon par destinataire), ciblage classe, vue famille filtrée ; event `annonce.publiee` + journal | `publier_annonce_for_actor`, `list_annonces_famille_for_actor`, test `annonce_etablissement_notifie_les_familles_ciblees`, labo *annonces* |
| **Retard (appel → fait VS)** | pointé à l'appel (`LigneEtat::Retard`) ou saisi par la VS → **fait canonique persistant** (`ent_retard`), distinct de l'absence (n'alimente pas la file/les compteurs d'absences), idempotent par `(appel, élève)`, compteur annuel, vue famille cloisonnée ; event `vs.retard` | `declarer_retard_for_actor`, `list_retards_eleve_for_actor`, `count_retards_eleve_depuis` (appliqué à la clôture d'appel), test `retard_pointe_a_l_appel_devient_fait_vs_persistant`, labo *retards* |
| **Compétences / LSU** | référentiel socle **par établissement** (`ent_competence_item`) + niveau de maîtrise **saisi** par `(élève, item, période)` (`ent_competence_evaluation`) ; lecture famille cloisonnée, **export LSU sur évaluations réelles** (plus de devinette), **livret PDF notifié** ; event `note.competence` | `creer_competence_item_for_actor`, `evaluer_competence_for_actor`, `list_evaluations_eleve_for_actor`, `publier_livret_competences_for_actor`, tests `competences_lues_par_la_famille_et_export_lsu_reel`, `api_competences_gate_role_validation_et_isolation`, labo *compétences* |
| **RH — absence prof** | validation direction → calendrier + mail secrétariat + event | `valider_absence_rh_for_actor` |
| **Auth** | login/logout/reset **audités** (RGPD), révocation de sessions | `audit_acces`, `POST /auth/password` |
| **Auth — passkey (WebAuthn)** | 2ᵉ facteur **exclusivement passkey** (aucun TOTP) : dès qu'un compte possède une passkey, le mot de passe seul **n'ouvre plus de session** (`PasskeyRequired`, et `login_with_ip` interne refuse par `EntDbError::PasskeyRequis`) ; le challenge est **à usage unique**, expire en 5 min et lie compte + établissement ; compteur d'usage mis à jour (anti-clonage) ; connexion **sans mot de passe** possible ; récupération perte d'appareil par Direction/Admin ; enrôlement/suppression/connexions **audités** (`passkey_enrolement`, `passkey_suppression`, `connexion_passkey`, `passkey_reset_admin`) | `passkey_register_start_for_actor`, `passkey_register_finish_for_actor`, `passkey_login_start`, `passkey_login_finish`, `reset_passkeys_admin_for_actor`, `login_start_with_ip`, tests `passkeys_enrolement_2fa_passwordless_et_isolation` (L1), `api_passkeys_2fa_passwordless_et_isolation` (L2), labo *Passkeys* |
| **Auth — SSO OIDC** | fédération d'identité **par établissement** (`ent_oidc_provider`) : découverte `.well-known` **validée à la configuration** (endpoints stockés, pas ressaisis), autorisation **PKCE S256 + `state` + `nonce`**, `id_token` **RS256 seulement** vérifié contre le JWKS (`iss`/`aud`/`exp`/`nonce`), `state` **à usage unique** (10 min) ; rattachement par `sub` (`ent_oidc_identity`) puis, à la première connexion, par **e-mail d'un membre existant** ; **aucun provisioning implicite** (identité inconnue = refus) ; **le SSO ne contourne pas une passkey déjà enrôlée** (2ᵉ facteur toujours exigé) ; connexion **auditée** (`connexion_sso`) ; vue publique = slug + libellé seulement (ni issuer, ni client_id, ni secret) | `upsert_oidc_provider_for_actor`, `list_oidc_providers_public`, `oidc_start`, `oidc_complete`, `crate ent-oidc`, tests `sso_oidc_rattachement_et_refus_des_inconnus` (L1), `api_sso_oidc_connexion_et_isolation` (L2), labo *SSO OIDC* |
| **Cascade RGPD (sortie élève)** | compte délié + sessions coupées = plus d'accès après le départ | `appliquer_repercussions_sortie` |
| **Observabilité / PRA** | métriques Prometheus (`/metrics`, compteurs par **gabarit de route**, histogramme de latence, sondes base : pool, sessions, outbox, passkeys) protégées par jeton (`ENT_METRICS_TOKEN`) ou 404 ; journaux JSON optionnels ; `/health` et `/ready` porteurs du `migration_tag` ; **inventaire d'intégrité** par établissement (lignes + digest par table + empreinte de schéma, tables volatiles exclues) qui **détecte la dérive** — donc qui prouve une restauration — et qui **refuse** un inventaire d'un autre tenant ; sauvegardes `pg_dump` gardées par loopback | `PgEnt::sondes_metriques`, `inventaire_etablissement_pour_acteur`, `verifier_inventaire_etablissement_pour_acteur`, `GET /metrics`, `GET /pra/inventaire`, `POST /pra/inventaire/verifier`, tests `pra_inventaire_integrite_et_isolation` (L1), `api_metriques_et_comptage_par_gabarit` + `api_pra_inventaire_integrite_et_isolation` (L2), `scripts/pra-exercice.sh --autotest`, labo *observabilité/PRA* |

## 3. Cascades « de vie » (les chaînes qu'un ENT doit garantir)

```
Appel clôturé ──► absence (canonique) ──► seuil atteint ──► signal VS
                                          ├─ fil messagerie famille
                                          ├─ mail + push parents
                                          └─ commission (CPE, optionnel)
Appel clôturé ──► retard (canonique `ent_retard`) ──► compteur / vue famille (distinct de l'absence)

Carnet saisi ──► fil messagerie ──► mail + SMS + push parents
Sanction     ──► SMS famille (+ points VS)
Réinscription─► mail + push + acompte + docs exigés
Facture      ──► échéances ──► prélèvement SEPA ──► rejet ──► rappel (messagerie)
Élève sorti  ──► périodes closes + cantine close + internat clos + accès coupés + docs annulés
```

### 3.1 Spine « publication établissement → famille »

Toute **publication destinée à la famille** déclenche une notification réelle
(mail via outbox anti-doublon `(source, kind)`, push si appareils enregistrés).
Aucun écran ne « suppose » que la famille verra l'info : elle est poussée.

```
Bulletin publié       ──► PDF + mail + push parents
Doc à signer publié   ──► mail + push parents (action requise)
Facture émise         ──► mail famille (montant) puis échéances/prélèvement
Encaissement          ──► quittance PDF + mail famille
Bourse décidée        ──► mail + push parents
Conseil de classe     ──► mail + push parents … si la décision est `visible_famille`
Annonce établissement ──► mail + push de chaque famille ciblée (classe respectée)
```

## 4. Trous assumés (décisions produit / hors scope)

| Sujet | État | Raison |
|---|---|---|
| **Prorata facturation / tarif** à la bascule de classe ou au départ | **non implémenté** | **règle d'établissement** → paramétrage à décider (Florian). Le reste de la cascade facture fonctionne. |
| **Absence élève → cantine** (repas du jour) | **via dispense repas** explicite | choix du projet (`PA_ABSENCE_REPAS` → « déjà, via dispenses repas »), pas d'automatisme pour ne pas inventer de règle. |
| **Incident → alerte CPE / notif parents** | non automatique | relève d'une politique d'établissement. |
| **Intégrations live** SMTP / SMS / push / SEPA / S3 | **dry-run** par défaut, opt-in par env | pas d'envoi réel en test. |
| **Événements déclarés mais jamais émis** (`scolarite.opened/closed`, `foyer.changed`, `eleve.homonyme_recherche`) | vocabulaire posé, émission partielle | enrichissement continu ; non bloquant pour les chaînes ci-dessus. |
| **Éditeur d'emploi du temps** (CP-SAT) | greedy + contraintes soft | solveur exact hors scope. |
| **SAML 2.0** (fédération d'identité) | **non implémenté** | OIDC couvre Google Workspace, Microsoft Entra, Keycloak ; SAML reste à faire si un établissement l'exige. |
| **Provisioning SSO automatique** (création/scission de comptes depuis l'IdP) | **non** : rattachement d'un compte local existant | choix assumé — l'ENT ne crée pas de compte ni de rôle à la volée depuis une identité externe ; l'administrateur crée le compte, la première connexion SSO le rattache (par `sub`, ou par e-mail). |
| **Perte de **toutes** les passkeys** | récupération par Direction/Admin (`DELETE /auth/users/{id}/passkeys`) | pas de facteur de secours automatique : la récupération est humaine et tracée (choix assumé, pas de contournement silencieux du 2ᵉ facteur). |
| **Traces distribuées (OTel) et tableaux de bord** | **non livrés** | journaux corrélés (`x-request-id`) + métriques suffisent au diagnostic ; pas de dépendance lourde, pas de dashboard propre à un hébergeur. |
| **Métriques métier par établissement dans `/metrics`** | **non** | `/metrics` est un endpoint d'infra : séries globales, aucune cardinalité tenant. L'inventaire par établissement, lui, est cloisonné et gardé par rôle. |
| **Réplication logique / restauration cross-région automatisée** | **non** | la RDB Scaleway assure snapshots + PITR ; le dump logique est la ceinture + la preuve d'intégrité (`/pra/inventaire`). |

## 5. Vérifier soi-même

```bash
# Toute la suite (L0/L1/L2 + scénarios multi-personas)
cd ent-rust
ENT_DATABASE_URL=postgres://scola:scola_dev_pwd@127.0.0.1:5432/scola \
TEST_DATABASE_URL=$ENT_DATABASE_URL cargo test --workspace

# Labo : vie croisée + cascade de sortie observables, line-by-line
ENT_DATABASE_URL=$ENT_DATABASE_URL ENT_ENV=lab NEXT_PUBLIC_SCOLA_ENV=lab \
  cargo run -p ent-lab        # → "=== OK — exploration terminée ==="
```

Preuves testées récemment : `crates/ent-db/src/scenarios.rs`
(`sortie_eleve_revoque_compte_et_grille`, `bascule_statut_sorti_declenche_la_cascade_transverse`,
`publications_famille_notifient_les_responsables`,
`facture_et_bourse_notifient_les_responsables`,
`decision_conseil_classe_publiee_notifie_la_famille`,
`copilote_repond_et_respecte_les_roles`, `garde_login_par_ip`, `reset_mot_de_passe_par_jeton_usage_unique`,
`annonce_etablissement_notifie_les_familles_ciblees`,
`retard_pointe_a_l_appel_devient_fait_vs_persistant`,
`competences_lues_par_la_famille_et_export_lsu_reel`,
`passkeys_enrolement_2fa_passwordless_et_isolation`,
`sso_oidc_rattachement_et_refus_des_inconnus`,
`pra_inventaire_integrite_et_isolation`)
et `crates/ent-api/src/lib.rs` (L2 HTTP + rôles, `api_annonces_gate_role_validation_et_isolation`,
`api_retards_gate_role_validation_et_isolation`,
`api_competences_gate_role_validation_et_isolation`,
`api_passkeys_2fa_passwordless_et_isolation`,
`api_sso_oidc_connexion_et_isolation`,
`api_metriques_et_comptage_par_gabarit`,
`api_pra_inventaire_integrite_et_isolation`).

Garde-fous des sauvegardes (aucune base requise) :

```bash
cd ent-rust && bash scripts/pra-exercice.sh --autotest   # → "Autotest OK"
```

Observabilité / PRA : voir `docs/ent-rust-observabilite-pra.md` (politique
d'accès à `/metrics`, séries et seuils d'alerte, procédure inventaire →
sauvegarde → restauration → vérification).

Les parcours passkey utilisent un **authentificateur logiciel** (`SoftPasskey`,
`webauthn-authenticator-rs`, dev/labo uniquement) : enrôlement et assertion sont
de vrais échanges WebAuthn (CBOR, clé COSE, signature ES256) — pas un simulacre.
La RP est configurée par `ENT_RP_ID` / `ENT_RP_ORIGIN` / `ENT_RP_NAME`
(repli dev `localhost` / `http://localhost:3000`) ; en prod : domaine réel en HTTPS.

Les parcours SSO utilisent un **IdP en mémoire** (`crate ent-oidc`, module
`testing` : clé RSA éphémère, JWKS, découverte, `id_token` RSA-SHA256 signé) : le
code de production est exercé sans socket. Un cas d'erreur PKCE est joué
(`arm_code_challenge` vérifie `SHA256(verifier)`, comme un vrai IdP). Réglages :
`ENT_PUBLIC_BASE_URL` (défaut `http://localhost:8088` → `redirect_uri`),
`ENT_OIDC_TIMEOUT_SECS` (10), `ENT_OIDC_STATE_MINUTES` (10).

## 6. Journal des dernières livraisons transverses (root-cause)

- **Observabilité + sauvegardes/PRA** : le besoin n'était pas « ajouter des
  métriques » mais **pouvoir prouver** deux choses : qu'une instance dérive, et
  qu'une restauration est fidèle. Choix structurants : (1) le middleware lit
  `MatchedPath` (gabarit `/auth/passkeys/{id}`) et non le chemin brut — sinon
  chaque identifiant d'élève créerait une série Prometheus (explosion de
  cardinalité) ; (2) `/metrics` est **désactivé par défaut** (`404`) et activé par
  `ENT_METRICS_TOKEN` (Bearer, comparaison à temps constant) ou `ENT_METRICS=1`
  en local — un endpoint d'observabilité ouvert est une fuite d'infrastructure ;
  (3) l'inventaire d'intégrité **exclut les tables volatiles** (`ent_session`,
  `ent_passkey_challenge`, `ent_oidc_state`, `ent_login_guard`,
  `ent_password_reset`) car restaurer un état de sécurité éphémère rouvrirait des
  sessions révoquées ou relâcherait un verrou anti-bourrage ; (4) le digest est
  calculé sur un agrégat **ordonné** (`ORDER BY` sur les digests de ligne) sinon
  deux lectures de la même table donneraient deux empreintes ; (5) la portée de
  vérification est **scellée sur l'établissement de la session** — un inventaire
  d'un autre tenant est un `403`, pas une comparaison silencieuse ; (6) les
  scripts de sauvegarde reprennent la garde du dépôt (`apply-migrations-direct.mjs`) :
  toute URL non loopback est refusée sauf `ALLOW_PROD_BACKUP=1`. La preuve de bout
  en bout est dans les tests : dérive détectée ligne à ligne, cloisonnement,
  `--autotest` des garde-fous, `/metrics` 404/401/200.

- **SSO OIDC (fédération d'identité par établissement)** : l'ENT délègue
  l'authentification à l'annuaire de l'établissement (Google Workspace, Microsoft
  Entra, Keycloak…). Racine : plutôt que de ressaisir des URLs d'endpoints (source
  d'erreur silencieuse), la configuration est **validée par découverte réelle**
  (`.well-known/openid-configuration`) et les endpoints sont **stockés**
  (`ent_oidc_provider`) : la page de connexion ne dépend donc pas du réseau, seul
  l'échange de code et le JWKS sortent. Le flux est **code + PKCE S256 + `state`
  + `nonce`** ; l'`id_token` est vérifié **RS256 uniquement** (`none`/`HS256` et
  toute confusion d'algorithme sont refusés) contre le JWKS par `kid` — un `kid`
  inconnu est un refus, pas un repli. `state`/PKCE/nonce vivent en base
  (`ent_oidc_state`) et sont **consommés** (`DELETE … RETURNING`, 10 min) : un
  retour rejoué échoue. Le rattachement se fait par `sub` (`ent_oidc_identity`,
  stable même si l'e-mail change), puis à la **première** connexion par l'e-mail
  d'un **membre existant** de l'établissement ; une identité inconnue est
  **refusée** — pas de provisioning implicite, donc pas de rôle inventé. Enfin,
  le SSO **ne contourne pas** un 2ᵉ facteur déjà enrôlé : un compte à passkey
  reçoit `PasskeyRequired` après l'IdP (sinon il suffirait de choisir « SSO »
  pour esquiver sa passkey). Le `client_secret` n'est jamais renvoyé par l'API
  (vue publique = slug + libellé, vue gestion = issuer/client_id/redirect_uri).
  Config : Direction/Admin. UI : bouton SSO à la connexion + page de retour
  `/auth/oidc/callback` + section « SSO établissement » dans `/app/securite`.
- **Passkeys / WebAuthn (2FA sans TOTP)** : la connexion accepte un **2ᵉ facteur
  passkey** — et **uniquement** passkey (`Touch ID` / `Face ID` / Windows Hello /
  clé de sécurité). Racine corrigée côté auth : `PgAuthRepo::login_inner` a été
  scindé en `verify_credentials` (mot de passe + appartenance) puis
  `issue_session`, ce qui permet d'**insérer le 2ᵉ facteur entre les deux** sans
  dupliquer la garde anti-force-brute. Dès qu'un compte a ≥ 1 passkey,
  `login_start_with_ip` retourne `PasskeyRequired` **sans session**, et le chemin
  interne `login_with_ip` **refuse** (`PasskeyRequis`) : impossible de contourner
  le facteur. Les challenges vivent en base (`ent_passkey_challenge`), sont
  **consommés** (`DELETE … RETURNING` → usage unique, rejeu impossible), expirent
  en 5 min (`ENT_PASSKEY_CHALLENGE_MINUTES`) et lient compte **et** établissement.
  Les credentials (`ent_passkey_credential`) stockent la clé publique WebAuthn
  opaque en JSONB + un index unique sur `cred_id` (un appareil ne s'enrôle qu'une
  fois) ; le compteur d'usage est réécrit après chaque assertion (détection de
  clonage). Enrôlement/suppression/connexions journalisés (RGPD). Récupération
  perte d'appareil : Direction/Admin seulement. UI : écran `/app/securite`
  (+ `/famille/securite`, `/eleve/securite`), bouton passkey à la connexion.
- **Vocabulaire d'événements réservé** : `scolarite.opened` / `scolarite.closed`,
  `foyer.changed` et `eleve.homonyme_recherche` sont **déclarés mais non émis** :
  leur sémantique n'est pas tranchée côté produit (période vs année de scolarité ;
  règle de foyer ; audit de recherche). Ils sont annotés « réservé » — on
  n'invente **pas** de règle pour les brancher. À ouvrir avec Florian si besoin.
- **Retards (`ent_retard`)** : un retard pointé à l'appel (`LigneEtat::Retard`) ou
  saisi par la vie scolaire devient un **fait canonique persistant** — au lieu de
  disparaître à la clôture de l'appel. Distinct de l'absence : il **n'alimente
  pas** la file VS ni les compteurs d'absences. Idempotent par `(appel, élève)`,
  compteur annuel par élève, vue famille **cloisonnée** (parent non lié refusé).
  Event `vs.retard` + `AggregateKind::Retard`. HTTP `GET/POST
  /eleves/{id}/retards`, section retards dans la vue famille de l'enfant, labo
  *retards*. Couvert par `retard_pointe_a_l_appel_devient_fait_vs_persistant` (L1)
  et `api_retards_gate_role_validation_et_isolation` (L2).

- **Compétences / LSU (`ent_competence_item` + `ent_competence_evaluation`)** :
  l'export LSU reposait sur une **devinette** (domaine déduit de la matière,
  niveau deviné depuis le texte d'une appréciation). Remplacé par un vrai modèle :
  référentiel socle **par établissement** (éditable, seed 5 domaines × 2 items) et
  **niveau de maîtrise saisi** par `(élève, item, période)` — échelle nationale
  insuffisante/fragile/satisfaisante/très bonne (codes LSU 1-4). L'export lit les
  **évaluations réelles** (élève sans évaluation → ligne « non évalué », aucun
  niveau inventé). Le **livret de compétences** est un PDF déposé côté famille et
  **notifié** (mail + push) ; un livret sans évaluation est refusé. Gate de lecture
  = `actor_can_read_notes` **+ vérification d'appartenance de l'élève à
  l'établissement** (le staff d'un autre tenant reçoit 404, pas une liste vide).
  Event `note.competence` + `AggregateKind::Competence`. HTTP `GET/POST
  /competences`, `GET/POST /eleves/{id}/competences`,
  `POST /eleves/{id}/livret-competences`, écran staff `/app/competences` + section
  famille, labo *compétences*.

- **Annonces établissement** (`ent_annonce`) : nouveau module de communication
  direction → familles / personnels. Une annonce « familles », « tous » ou
  « classe » est **diffusée** à chaque famille ciblée (mail + push), avec un
  anti-doublon **par destinataire** (`count_by_source_kind_eleve`) — plusieurs
  familles partageant la même `source_id` ne se bloquent pas mutuellement. La
  vue famille respecte le ciblage classe. HTTP `POST/GET /annonces` +
  `GET /eleves/{id}/annonces`, UI `/app/annonces`. Couvert par
  `annonce_etablissement_notifie_les_familles_ciblees` (L1) et
  `api_annonces_gate_role_validation_et_isolation` (L2).
- **Spine « publication établissement → famille »** : publier un **bulletin**, un
  **document à signer**, émettre une **facture**, **encaisser un paiement
  (quittance PDF)**, décider une **bourse** ou une **décision de conseil de classe
  `visible_famille`** notifie désormais réellement les responsables (outbox mail
  anti-doublon `(source, kind)` + push si appareils enregistrés ; quittance en
  pièce jointe). Une publication « dans le portail » n'est plus supposée vue :
  elle est poussée. Couvert par `publications_famille_notifient_les_responsables`,
  `facture_et_bourse_notifient_les_responsables`,
  `decision_conseil_classe_publiee_notifie_la_famille`.
- **Cascade de sortie unifiée** : extraite dans `appliquer_repercussions_sortie`
  et branchée sur **les deux** portes (`sortir` daté **et** bascule de statut
  → `Sorti`). Ajout de la **fermeture d'affectation internat** + **clôture des
  périodes** sur la porte statut (elles n'existaient que sur `sortir`). Idempotente.
  Un seul état « sorti », un seul jeu de conséquences.
- **Garde brute-force par IP** (en plus du compte) : correction de la fenêtre
  (`window_start = now` à la création au lieu de `now − fenêtre`) ; faux compte
  neuf accumule désormais (régression couverte).
- **Audit RGPD** : login/logout/changement de mot de passe journalisés (`domaine=auth`).
