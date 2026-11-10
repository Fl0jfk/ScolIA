# Recette de sécurité n° 3 — cloisonnement multi-établissements, sur le logiciel réel

**Date :** 2026-11-09 — **Branche :** `ent-recette-securite-3` (base `ent-integration-lot10`) — **Pilote :** Opus, sous-agents Sonnet pour les relectures adverses.
**Statut :** recette faite ; **RS3-01 à RS3-04 corrigées + tests** (RS3-03 soldée par le lot F0 des finances, 2026-11-10), cloisonnement multi-établissements **vérifié tenant**. Correctifs de la session du 2026-11-09 à relire par Florian.

## Méthode

On joue des **attaquants réels** contre le **logiciel réel** : le routeur `ent_api::app` est appelé en process (`tower::ServiceExt::oneshot` — mêmes couches, mêmes extracteurs, mêmes gardes, même base que le serveur HTTP), sur un **PostgreSQL 16 local jetable** (`ent_ci_test`, rôle `ent_ci`), avec **deux établissements de labo A et B** créés par `POST /labo/bootstrap`. Aucun accès à la production, à Next.js, à `main`, `dev`, `ENT`, ni à un secret. Données de labo fictives uniquement.

Trois relectures adverses du code (lecture seule) ont cadré les vecteurs : (1) lot 7 évaluations + emploi du temps ; (2) finances/SEPA + communication ; (3) RLS, jobs de fond, santé/psy, personnels, recherche. Chaque trouvaille est rejouée contre le serveur réel et **figée en test automatique** dans `ent-rust/crates/ent-api/tests/recette_securite_3.rs` :
- test **vert** si le garde-fou tient ;
- pour une **faille ouverte**, un test `#[ignore = "FAILLE OUVERTE RS3-xx …"]` qui **échoue** tant que la faille existe (`cargo test … -- --ignored`).

Lancement :
```
service postgresql start
ENT_DATABASE_URL='postgres://ent_ci:ci@127.0.0.1:5432/ent_ci_test' ENT_ENV=test \
RUST_MIN_STACK=16777216 CARGO_PROFILE_DEV_DEBUG=0 CARGO_BUILD_JOBS=4 \
cargo test --locked -p ent-api --features ent-api/labo,ent-lab/labo \
  --test recette_securite_3 -- --test-threads=1
```
Pour la partie « relevé bancaire rejoué » de RS3-02, ajouter `ENT_BANK_DRY_RUN=1` (sinon cette partie est sautée avec un message). (La feature CI est `ent-api/labo,ent-lab/labo`. Le rôle `ent_ci` doit être superuser, comme l'image PostgreSQL de la CI, pour les tests qui créent un rôle RLS jetable.)

---

## Verdict (trois lignes)

1. **Le cloisonnement multi-établissements tient** : sur toutes les routes de l'API, un acteur de A (tout rôle, du compte désactivé au double membre) n'atteint **aucune** donnée de B — matrice générique verte, et aucune des trois relectures adverses n'a trouvé de fuite inter-établissements exploitable.
2. **Une faille élevée, intra-établissement, a été corrigée** : le chemin historique `/facturation/*` ouvrait factures, IBAN et mandats SEPA à un parent **non payeur** (parent séparé, tiers lié), jusqu'à lui permettre de détourner le prélèvement ; il est désormais réservé au parent payeur (RS3-01), comme le chemin `/finances/*`.
3. **Corrigés ensuite** : le double encaissement SEPA (RS3-02, migration 376), le prélèvement d'un foyer en paiement manuel (RS3-03, en partie) et la référence croisée `absence_rh_id` de `POST /edt/annulations` (RS3-04, migration 377).

### Failles critiques ou élevées

| Réf. | Gravité | Portée | Constat | Statut |
|---|---|---|---|---|
| **RS3-01** | **Élevée** | intra-établissement | Chemin historique `/facturation/*` : un parent **non payeur** lié au foyer lit factures, quittances, historique IBAN et mandats SEPA, et peut **créer/révoquer** le mandat de prélèvement (détournement du compte débité). `foyer_ids_parent` ne filtrait pas `est_payeur`. | **Corrigé + test RS3-01** |
| **RS3-02** | **Élevée** | intra-établissement | Double encaissement SEPA : `executer_prelevement_for_actor` encaisse **avant** le contrôle de statut, sans transaction englobante ; rejouer `POST /facturation/sepa/prelevements/{id}/executer` (ou le même relevé via `POST /facturation/banque/rapprocher`) encaisse deux fois. | **Corrigé + test RS3-02** (voir 2.2) |

Aucune faille **critique**. Aucune fuite **inter-établissements** exploitable (priorité absolue) n'a été trouvée.

---

## 1. Ce qui tient (vérifié sur le serveur réel)

### 1.1 Cloisonnement multi-établissements — priorité absolue

- **Matrice générique de lecture** (`matrice_cloisonnement_toutes_routes_get`) : l'inventaire des routes est **extrait du source** de `routes.rs` et des 16 autres fichiers de routeur (714 déclarations `.route`, 258 à paramètre) ; le test parcourt **toutes les routes GET à paramètre**, y injecte successivement **chaque identifiant connu de B** (établissement, élèves, foyers, document, fil, facture, absence, matière, appel, utilisateurs), avec **5 acteurs de A** (Direction, Admin, Prof, Parent, Élève). Aucune réponse ne révèle une donnée de B : refus (401/403/404), réponse vide, ou — si 2xx — aucun **autre** identifiant de B, aucun champ interdit (`storage_key`, `password_hash`, `argon2`), et indiscernable de la même route avec un UUID inexistant. **Toute route ajoutée au routeur est couverte automatiquement.**
- **Matrice générique d'écriture** (`matrice_cloisonnement_toutes_routes_ecritures_corps_vide`) : toutes les routes POST/PUT/PATCH/DELETE à paramètre, identifiant de B dans l'URL, corps vide, acteurs Direction et Admin de A → **aucun 2xx** sur une cible de B.
- Ces matrices **complètent** la matrice déclarative existante (`tests/separation.rs` : rôle × ressource × A/B, écritures comprises, et C6 « double membre » — personne membre de A **et** de B, connectée sur A, refusée sur les identifiants de B) et les tests d'hôte (`tests/tenant_hote.rs` : l'établissement est imposé par l'adresse ; `etablissement_id`/`X-Forwarded-Host` du client ignorés sans proxy de confiance ; session de A sur l'hôte de B → 401 ; hôte inconnu → 404 en strict).
- Les **trois relectures adverses** (lot 7, finances/communication, RLS/jobs/santé/personnels) n'ont trouvé **aucune** requête qui lise/écrive par identifiant sans filtre `etablissement_id`, aucun `etablissement_id` pris dans le corps/la requête du client, aucune injection SQL (les `format!` n'assemblent que des constantes ou des numéros de paramètres `$n` ; les entrées utilisateur passent par `bind`).

### 1.2 Comptes, sessions, jetons (tests verts)

- **Compte désactivé** (`rs3_compte_suspendu_puis_ferme_invalide_la_session`) : un accès **suspendu** ou **fermé** invalide immédiatement le jeton déjà ouvert (401) et refuse toute nouvelle connexion — `resolve_bearer` relit l'état du membership (`etat = 'ouvert'`, `ent_agent_acces_suspendu`) à chaque requête (socle 6 P0).
- **Session volée / logout** (`rs3_logout_invalide_le_jeton_vole`) : après `POST /auth/logout`, le jeton ne vaut plus rien (401).
- **Jeton mobile** (`rs3_jeton_mobile_de_a_ne_voit_rien_de_b`) : un jeton `client=mobile` de A porte l'établissement A comme un jeton web et est refusé de la même façon sur les cibles de B ; le type de client n'élargit pas le cloisonnement.

### 1.3 Facturation après correctif (RS3-01)

Un parent **non payeur** est désormais refusé sur `GET /facturation/factures/{id}`, `GET /foyers/{id}/iban-histo`, `GET /facturation/sepa/mandats`, `POST /facturation/sepa/mandats` (et révocation) — comme le chemin `/finances/*`. La payeuse légitime conserve l'accès (matrice `separation::domaine_factures` toujours verte).

---

## 2. Failles et défauts

### 2.1 RS3-01 — Parent non payeur et facturation (élevée) — **CORRIGÉ**

- **Scénario d'attaque (rejouable).** Un parent lié au foyer mais **non payeur** (parent séparé sans charge de paiement, tiers lié) appelle, via le chemin historique :
  - `GET /facturation/factures/{id}` / `/pdf`, `GET /facturation/quittances/{id}` → factures et quittances du foyer ;
  - `GET /foyers/{id}/iban-histo` → historique bancaire (titulaire, BIC) ;
  - `GET /facturation/sepa/mandats` → mandats du foyer ;
  - `POST /facturation/sepa/mandats/{id}/revoquer` puis `POST /facturation/sepa/mandats` avec **son** IBAN → **détournement du compte débité**.
- **Cause.** `PgEnt::foyer_ids_parent` (`ent-db/src/service.rs`) retournait **tous** les foyers d'un lien actif, sans filtrer `est_payeur` ; `actor_can_access_facture`, les mandats, l'historique IBAN, le quotient familial et la liste de factures s'appuient tous dessus. Le chemin neuf `/finances/*` (`espace_finances_famille`, `finances::payeurs`) filtre `est_payeur` : les deux chemins divergeaient.
- **Preuve.** Test `rs3_01_parent_non_payeur_sans_acces_a_la_facturation` : il bascule temporairement la payeuse en non-payeuse (SQL), vérifie le **refus** sur les quatre routes, puis restaure l'état. Sans le correctif, il échoue (`GET facture : 2xx (200)`, `iban-histo : données`, `sepa/mandats : mandat visible`).
- **Correctif.** `foyer_ids_parent` filtre désormais `l.est_payeur` (un seul point, tous les appelants financiers en héritent : factures, quittances, mandats, IBAN, quotient). Non-régression : suites `ent-db tests_finances` (6/6) et `ent-api separation` (factures, L1/L2) vertes.

### 2.2 RS3-02 — Double encaissement SEPA (élevée) — **CORRIGÉ**

- **Scénario.** `executer_prelevement_for_actor` (`ent-db/src/service.rs:19212`, route `POST /facturation/sepa/prelevements/{id}/executer`) exécute `encaisser_avec_engage` (encaissement + quittance + e-mail) **avant** `plan_executer_prelevement`, qui seul contrôle le statut (`Prevu`/`Exporte`). Les deux opérations ne sont **pas** dans une transaction commune. Rejouer la route sur le même prélèvement (ou rejouer le même relevé via `POST /facturation/banque/rapprocher` — `update_sepa_prelevement` n'a aucune garde de statut dans son `WHERE`) **encaisse plusieurs fois** la même échéance.
- **Correctif appliqué (2026-11-09).** `PgFacRepo::executer_sepa_atomique` (`fac_repo.rs`) fait tout en **une transaction** : `SELECT … FOR UPDATE` sur le prélèvement puis sur la facture (sérialise les exécutions concurrentes), **contrôle de statut avant toute écriture**, calcul des sommes sous verrou, encaissement, solde, quittance et passage à `execute`. Idempotence : un prélèvement déjà `execute` renvoie `(prélèvement, None)` — réponse 200 **sans effet** ; un prélèvement `rejete` est refusé. Journal et e-mail (`effets_encaissement`) partent **après** la validation, une seule fois. `update_sepa_prelevement` porte une garde de statut dans son `WHERE` (un prélèvement exécuté ou rejeté ne bouge plus : plus de rejet après exécution). Le rapprochement bancaire rejoué enregistre la ligne en `skip` sans compter d'exécution. **Garde-fou en base : migration `376_sepa_unicite_encaissement`** (index uniques sur `ent_fac_sepa_prelevement(encaissement_id)` et sur `ent_fac_encaissement(etablissement_id, reference)` pour le moyen `prelevement`). **Attention au déploiement** : la création de ces index échoue s'il existe déjà des doublons en base — à contrôler avant, sans les corriger automatiquement (données monétaires). Test : `rs3_02_double_encaissement_sepa_rejeu_et_concurrence` (4 appels concurrents, rejeu, relevé rejoué deux fois, doublon refusé par la base, rejet après exécution refusé).
- *Recommandation d'origine :* (1) Contrôler le statut du prélèvement **avant** l'encaissement ; (2) envelopper lecture + encaissement + mise à jour dans **une** transaction avec `SELECT … FOR UPDATE` sur la ligne de prélèvement (anti-TOCTOU) ; (3) garde de statut dans le `WHERE` de `update_sepa_prelevement` ; (4) idempotence du rapprochement par `end_to_end_id`. Test à ajouter : préparer un prélèvement (chemin `/finances/*`), appeler `executer` deux fois, vérifier **un seul** encaissement.

### 2.3 RS3-03 — Chemin SEPA historique : pré-notification et mode de paiement contournés (moyenne) — **PARTIELLEMENT CORRIGÉ**

- `GET /facturation/sepa/export` (`service.rs:18904`) exporte les prélèvements `statut = 'prevu'` **sans exiger** `prenotification_id IS NOT NULL` (le chemin neuf `/finances/*` l'exige) ; de plus c'est un **GET qui change l'état** des lignes en `exporte`. `POST /facturation/factures/{id}/sepa/prelever` ne vérifie pas `mode_paiement = manuel`. L'IBAN du créancier passe en **query string** (`cdtr_iban`, `cdtr_bic`, `cdtr_ics`) — hors des logs applicatifs (qui utilisent `MatchedPath`), mais exposé aux proxys/historique. **Corrigé le 2026-11-09 :** `planifier_prelevement_for_actor` refuse un foyer en paiement `manuel` (test `rs3_03_prelevement_historique_refuse_un_foyer_en_paiement_manuel`) ; la garde de statut ajoutée à `update_sepa_prelevement` (RS3-02) rend aussi l'export sans double marquage. **Soldé le 2026-11-10 (lot F0 des finances, branche `ent-socle-finances-f0`)** : l'export historique est un **`POST`** (`GET` → 405) ; le créancier est **lu côté serveur** (plus de `cdtr_*`, un champ inconnu est refusé) ; il n'exporte que les prélèvements **pré-notifiés** (garde en base : transition `prevu → exporte` refusée sans pré-notification), sur **mandat actif**, du chemin historique ; la planification historique remet la pré-notification (délai de l'établissement, payeur joignable exigé) et refuse une facture qui a des échéances. Tests : `ent-api/tests/recette_finances_f0.rs` (`f0_fd03_…`, `f0_r_p6_…`, `f0_fd04_…`, `f0_fd10_…`). L'UI JavaScript actuelle appelle le nouvel export en POST. **Recommandation d'origine (réalisée) :** aligner l'export historique sur `/finances/*` (pré-notification obligatoire, verbe POST, paramètres du créancier lus côté serveur), ou retirer le chemin historique.

### 2.4 RS3-04 — Référence croisée EDT A→B (faible, cloisonnement) — **CORRIGÉ**

- **Scénario (rejoué, 201 obtenu).** `POST /edt/annulations` (`edt_s2.rs:356`) accepte un `absence_rh_id` du **corps** sans vérifier qu'il appartient à l'établissement de la session : `edt_annuler_seance` → `ecrire_exception` l'insère tel quel (`edt_remplacement.rs`, clé étrangère **simple** `ent_absence_rh(id)`). Une exception d'EDT de A peut donc pointer vers une **absence RH de B**.
- **Impact.** Limité : il faut `edt.gerer` dans A ; un UUID existant renvoie 201, un UUID inexistant échoue sur la clé étrangère → **oracle d'existence d'UUID** ; la clé étrangère **bloque la suppression** de l'absence côté B (DoS mineur). Pas de fuite de contenu (les lectures filtrent par établissement).
- **Preuve.** Test `rs3_04_edt_annulation_absence_rh_d_un_autre_etablissement` : déclare une absence RH dans B, trouve un créneau de A, annule ce créneau en référençant l'absence de B → attend un refus (obtenait **201** avant le correctif ; test `#[ignore]` rendu normal).
- **Correctif appliqué (2026-11-09).** `edt_annuler_seance` vérifie que l'absence appartient à l'établissement de l'acteur (refus « introuvable », sans oracle d'UUID) ; **migration `377_edt_exception_absence_etab`** : index unique `(etablissement_id, id)` sur `ent_absence_rh` et clé étrangère composite `ent_edt_exception(etablissement_id, absence_rh_id)` créée `NOT VALID` (les lignes historiques ne sont pas revérifiées, toute nouvelle ligne l'est).
- **Correctif recommandé (petit).** Vérifier `absence_rh_id` par `etablissement_id` avant l'insertion (reprendre le contrôle de `edt_organiser_remplacement`, `edt_remplacement.rs:743`) et poser une **clé étrangère composite** `(etablissement_id, absence_rh_id)` (migration).

### 2.5 Autres constats signalés (moyens / faibles, intra-établissement)

| Réf. | Gravité | Constat | Recommandation |
|---|---|---|---|
| RS3-05 | Moyenne | `/personnel/search` ouvert à **tout** le personnel (`is_staff`) ; renvoie e-mails et rôles de tout le personnel de l'établissement ; `%` et `_` non échappés dans l'`ILIKE` → énumération. Pas de fuite inter-établissements. | Réserver à un droit (annuaire), échapper `%`/`_`, limiter les champs. |
| RS3-06 | Moyenne | Courrier d'orientation psy (`service.rs:16783`) envoyé à **tous** les responsables au lien actif, sans vérifier `signale_famille` ni les droits du lien (`peut_voir_*`, autorité parentale), alors que le PDF parent le vérifie. | Appliquer les mêmes filtres qu'au PDF parent. |
| RS3-07 | Moyenne | `expedier_envois` (`inscriptions_reactions.rs`) garde une transaction ouverte pendant l'envoi SMTP ; un échec après des envois réussis provoque des **doublons** d'invitation. | Sortir l'envoi de la transaction, journal d'état par envoi. |
| RS3-08 | Faible | Mandat SEPA créé avec `signe_at = now()` sans preuve de consentement du débiteur ; RUM choisi par le client. | Exiger une preuve de signature ; RUM serveur. |
| RS3-09 | Faible | `GET /eleves/{id}/edt/changements` renvoie l'**e-mail** du remplaçant aux familles quand `display_name` est vide (repli sur `u.email`). | Pour les non-staff, nom affiché seul. |

---

## 3. Défense en profondeur (constat, hors faille)

- **RLS PostgreSQL en mode `off` par défaut** (`ENT_RLS_MODE`, `tenant_tx.rs`) : aujourd'hui l'isolation repose sur la **discipline applicative** (vérifiée saine ci-dessus), pas sur la base. Les politiques existent sur les tables sensibles (santé, psy, sanctions, finances, élève, agents, structure) mais **pas** sur les tables de base (`ent_eleve_lien`, `ent_user`, `ent_membership`, notes, absences, messages, notifications, outbox, imports). Aucune fonction `SECURITY DEFINER` (aucun risque de contournement par fonction). **Avant pilote/production :** passer `ENT_RLS_MODE=strict` avec un rôle applicatif **ni propriétaire ni `BYPASSRLS`** et `ALTER ROLE … SET app.rls_strict='on'`, étendre la RLS aux tables de base, faire passer les jobs de fond (outbox, notifications, rappels) par `TenantTx`. C'est le constat **RF-20** de `REVUE_SECURITE_SOCLE1.md`, toujours ouvert.
- **Écrans `/web/*`** : non rejoués dans un navigateur ; ils appellent les **mêmes** routes de l'API backend que la matrice couvre, et `ent-web-server` a ses propres suites d'en-têtes de sécurité et de session cookie (`entetes_securite.rs`, `session_cookie_serveur.rs`, `securite_backend.rs`).

## 4. Couverture et limites

- **Couverture des routes** : automatique et auto-entretenue (inventaire extrait du source ; `rs3_inventaire_couvre_tous_les_fichiers_de_routes` échoue si un fichier de routeur n'est pas inventorié). Les deux matrices génériques couvrent lecture (GET à paramètre) et écriture (corps vide) sur **toutes** les routes.
- **Limites.** (1) Les écritures ne sont éprouvées qu'avec un **corps vide** : un corps complet portant un identifiant de B dans un sous-objet reste du ressort des specs déclaratives de `separation.rs` et des tests ciblés. (2) RS3-02 est rejouée de bout en bout sur le chemin historique ; le chemin `/finances/*` exécute déjà sous `FOR UPDATE` et profite de l'index unique de la migration 376. (3) La RLS n'est pas éprouvée en mode strict (déploiement). (4) Pièces jointes et URL pré-signées : vérifiées par relecture (clé de stockage serveur, HMAC court, `key_belongs_to_etab`) et par l'oracle `storage_key` des matrices, non par un essai de forge de lien d'un autre établissement.

## 5. Tests ajoutés

`ent-rust/crates/ent-api/tests/recette_securite_3.rs` :
- `matrice_cloisonnement_toutes_routes_get` — générique, lecture inter-établissements (vert) ;
- `matrice_cloisonnement_toutes_routes_ecritures_corps_vide` — générique, écriture inter-établissements (vert) ;
- `rs3_compte_suspendu_puis_ferme_invalide_la_session`, `rs3_logout_invalide_le_jeton_vole`, `rs3_jeton_mobile_de_a_ne_voit_rien_de_b` — comptes/sessions/jetons (verts) ;
- `rs3_01_parent_non_payeur_sans_acces_a_la_facturation` — RS3-01 corrigée (vert) ;
- `rs3_inventaire_couvre_tous_les_fichiers_de_routes` — invariant d'inventaire (vert) ;
- `rs3_02_double_encaissement_sepa_rejeu_et_concurrence` — RS3-02 corrigée (vert) ;
- `rs3_03_prelevement_historique_refuse_un_foyer_en_paiement_manuel` — RS3-03, volet mode de paiement (vert) ; les volets pré-notification, verbe et IBAN en URL sont couverts par `recette_finances_f0.rs` (lot F0) ;
- `rs3_04_edt_annulation_absence_rh_d_un_autre_etablissement` — RS3-04 corrigée (vert, plus d'`#[ignore]`).

Correctif : `ent-rust/crates/ent-db/src/service.rs` (`foyer_ids_parent` filtre `est_payeur`).

## 6. Vérifications des correctifs (2026-11-09)

Verts en local : `recette_securite_3` 10/10 (labo, `ENT_BANK_DRY_RUN=1`) ; `ent-db` sans labo (147 tests unitaires + `isolation_sql_statique`) ; `ent-db` avec labo, modules `tests_finances` (6), `tests_edt_s2` (10) et `migrate` ; clippy sans erreur sur `ent-db`, `ent-api`, `ent-lab` (sans labo). **Non exécutés** : la suite complète `ent-db`/`ent-api` avec labo (plus d'une heure en local), le scénario `ent-lab` de bout en bout (compilé seulement ; il consomme le nouveau type de retour de `executer_prelevement_for_actor`), et `cargo fmt` (les paquets ne sont pas formatés à l'origine : un `fmt` réécrirait des milliers de lignes hors sujet). À lancer par la CI GitHub.
