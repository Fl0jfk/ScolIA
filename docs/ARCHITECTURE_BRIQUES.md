# Architecture en briques de l'ENT ScolIA (D-026)

- **Date :** 2026-11-07 — **Auteur :** session de conception (Opus), branche `ent-architecture-briques`.
- **Statut :** **proposition, non décidée.** Le principe (D-026) est validé par Florian ; le découpage, le kit et le plan ci-dessous sont à relire par le contradicteur du pilotage puis par Florian (§11).
- **Prototype :** `ent-rust/crates/ent-kit/` (19 tests, non branché : aucun comportement changé), §9.
- **Relecture indépendante :** 15 objections (6 hautes) d'une revue `review-sonnet` le 2026-11-07, toutes intégrées (§13).
- **Marques :** **[F]** fait vérifié dans le code (chemin:ligne, état de `ent-integration-lot7` + documentation de `ent-mc0hoe`) ; **[R]** recommandation ; **[H]** hypothèse à vérifier ; **[Q]** question à trancher.

---

## 0. En bref

1. **Le problème est mesuré [F].** Les quatre branches de code en cours (E3–E4, A1, I1, D-022 bis) modifient **toutes les quatre** les cinq mêmes fichiers : `ent-db/src/migrate.rs`, `ent-auth/src/actions.rs`, `ent-db/src/service.rs`, `ent-db/src/lib.rs`, `ent-api/tests/separation.rs` ; trois d'entre elles touchent aussi `routes.rs`, `droits_repo.rs`, `dossier_personnes.rs`, `droits_ctx.rs`, `ent-lab/src/main.rs`, `ent-web-api/src/lib.rs`. Ce ne sont pas des conflits de métier, ce sont des **listes tenues à la main**.
2. **La cause est double [F].** (a) Des listes centrales : 146 migrations écrites une à une et un `MIGRATION_TAG` à la main ; un `enum Action` de 68 variantes qu'on complète en 6 endroits ; 552 routes dans 13 fichiers dont un de 12 000 lignes ; un inventaire de séparation qui relit le texte des sources ; trois listes d'écrans Leptos. (b) Tout le métier est une seule structure `PgEnt` étendue par **37 fichiers `impl PgEnt`** dans un seul crate `ent-db` de 115 000 lignes, où chaque domaine lit et parfois **écrit** les tables des autres (la facturation met à jour `ent_eleve`, les inscriptions écrivent dans 7 tables du noyau).
3. **La réponse [R]** : un **noyau** (élève, personnes et foyers, structure et cours, personnels, sécurité, plateforme : transactions d'établissement, outbox, fichiers, journal, registres, contrat IA) qui expose des **ports** stables, et des **briques** (un dossier, ses crates, ses migrations, ses actions, ses routes, ses écrans, ses tests) qui se branchent par un **kit** : des déclarations-données (`pub const BRIQUE`) que l'assemblage vérifie au démarrage et en test. Une brique ne modifie aucun fichier du noyau ; la seule ligne partagée est son inscription dans la liste d'assemblage, protégée par un test de complétude et une fusion git « union ». Attention : la sécurité « par construction » suppose que le RLS soit réellement appliqué aux transactions des briques, ce qui n'est **pas** le cas aujourd'hui (mode `Off` par défaut) : c'est un préalable de la première extraction (§7.3).
4. **Le chemin [R]** : dix étapes **sans changement de comportement** (même suite de tests, même CI, mêmes instantanés du catalogue, de l'inventaire des routes et des consommateurs avant et après), en commençant par ce qui supprime le plus de conflits : migrations découvertes (E1), actions déclarées (E2), routes déclarées et isolation générée (E3–E4), registres (E5) ; puis le kit (E6), l'extraction des domaines déjà présents (E7 : santé en pilote, puis finances, communication, évaluations, emploi du temps, cahier de textes, inscriptions), le web (E8), et enfin le découpage du reste d'`ent-db` en noyau (E9). Les changements **de comportement** que l'analyse révèle (suivi des migrations appliquées, RLS strict par défaut, tables sans RLS) sont listés à part et ne se mélangent pas à la refonte.
5. **Le prototype lève deux doutes [F]** : la liste découverte des migrations est **identique** à la liste manuelle (146 entrées, même ordre, même tag) ; une action réduite à son **identifiant** (le code) et déclarée comme **constante** sert de motif de `match`, donc l'`enum` peut devenir un catalogue déclaré par brique sans toucher les ≈ 460 usages `Action::X` (seuls les 4 `use Action::*` d'`ent-auth` sont à réécrire). Il révèle aussi **6 contraintes déjà redéfinies dans deux migrations** (le piège exact de F-009), désormais gelées par un test.

---

## 1. Ce que dit le code aujourd'hui [F]

### 1.1 Points de collision mesurés
Différence de chaque branche avec son point de départ sur la lignée du lot 7 (`git merge-base`), au 2026-11-07 :

| Branche | Base | Commits | Fichiers « centraux » touchés |
|---|---|---|---|
| `ent-socle-eleve-e3-e4` | `99a6e17` (lot 6) | 42 | `routes.rs` (+973), `actions.rs` (+239), `service.rs` (+224), `migrate.rs`, `separation.rs`, `droits_repo.rs`, `dossier_personnes.rs`, `ent-db/lib.rs`, `ent-events/lib.rs`, `ent-lab/main.rs`, `dto.rs`, `ent-api/main.rs` |
| `ent-socle-administratif-a1` | `99a6e17` | 34 | `actions.rs` (+439), `dossier_personnes.rs` (331 lignes changées), `roles.rs`, `migrate.rs`, `separation.rs`, `service.rs`, `droits_repo.rs`, `ent-db/lib.rs`, `ent-web-api/lib.rs`, `ent-web-app/lib.rs`, `navigation.rs` |
| `ent-socle-inscriptions-i1` | `99a6e17` | 56 | `routes.rs` (+247), `actions.rs` (+227), `service.rs` (+222), `migrate.rs`, `separation.rs`, `dossier_personnes.rs`, `droits_repo.rs`, `ent-db/lib.rs`, `ent-lab/main.rs`, `ent-web-api/lib.rs`, `ent-web-app/lib.rs`, `navigation.rs` |
| `ent-d022-bis-enseignements` | `9669cd8` (lot 7) | 28 | `perimetre.rs` (+92), `service.rs` (+227), `routes.rs`, `actions.rs`, `migrate.rs`, `separation.rs`, `ent-db/lib.rs`, `ent-lab/main.rs` (signatures `*_for_actor` changées : appels modifiés dans le labo) |

Fichiers touchés par les 4 branches : `service.rs`, `migrate.rs`, `ent-db/lib.rs`, `actions.rs`, `separation.rs`. Par 3 : `routes.rs`, `droits_repo.rs`, `droits_ctx.rs`, `dossier_personnes.rs`, `cahier.rs`, `ent-lab/main.rs`, `ent-web-api/lib.rs`, `referentiel.rs`, `dossier.rs` (api), `ent-types/dossier.rs`. Trois branches sur quatre partent du lot 6 et non du lot 7.

### 1.2 Migrations
- 146 fichiers `NNN_nom.sql` dans `ent-rust/migrations/` (001 à 255, par blocs non contigus).
- `ent-db/src/migrate.rs:10-157` : liste `MIGRATIONS` écrite à la main (`include_str!`) ; `:163` : `MIGRATION_TAG = "255_email_confirmation"` à la main ; test `:247-270` qui compare le **dernier** nom et le **nombre** de fichiers (pas l'égalité des ensembles).
- **Toutes les migrations sont rejouées à chaque démarrage** (`ent-api/src/lib.rs:238` → `service.rs:905-917` → `apply_socle_migrations`), sans table de suivi, sous un verrou consultatif (`pg_advisory_lock`, `migrate.rs:168-188`), **sans transaction** (instruction par instruction en autocommit, `:197-214`) ; le découpage ne gère que les blocs `$$` et les commentaires en début de ligne.
- Idempotence de fait : 225/225 `CREATE TABLE IF NOT EXISTS`, 137/137 `ADD COLUMN IF NOT EXISTS`, politiques RLS dans des blocs `DO` avec `DROP POLICY IF EXISTS`.
- **Six contraintes `CHECK` sont définies dans deux migrations** (mesuré par le prototype) : `ent_absence_statut_chk` (010, 011), `ent_absence_signal_statut_chk` (012, 035), `ent_fac_encaissement_moyen_chk` (025, 030), `ent_sortie_controle_source_chk` (062, 065), `ent_affectation_source_chk` (092, 110 — le cas de F-009), `ent_eleve_dispositif_kind_chk` (016, **212 : migration de la santé qui redéfinit une contrainte d'une table de l'élève**). Elles fonctionnent parce que la seconde est rejouée après la première ; une troisième redéfinition plus étroite rétrécirait silencieusement la règle.
- `MIGRATION_TAG` est exposé par `/health` et `/ready` (`routes.rs:1811`), lu par le PRA (`pra.rs:25`), les scénarios et l'observabilité.

### 1.3 Catalogue d'actions (D-017)
- `ent-auth/src/actions.rs` (912 lignes) : `enum Action` de **68 variantes** (`:13-163`) ; chaque nouvelle action se répète dans `TOUTES` (`:179-251`, liste à la main, non vérifiée par le compilateur), `code()`, `libelle()`, `portees_admises()` et `modele_par_defaut()` (`:513-746`, les modèles par profil).
- Le catalogue **n'est pas en base** : `ent_role_droit.action` stocke le code texte (`098_roles_parametrables.sql:22-30`) ; un code inconnu est ignoré à la lecture, « jamais élargi » (`droits_repo.rs:92-93`).
- Contrôle : `autoriser_eleve(etab, user, role_base, action, eleve)` (`droits_repo.rs:126`), `eleves_autorises(...) -> Option<HashSet<Uuid>>` (`:174`), `a_action_etablissement` (`:111`), enveloppe `staff_eleve` (`service.rs:8845`). 463 usages de `Action::X` dans 45 fichiers.
- Le périmètre du professeur (D-022) : `perimetre.rs` (150 lignes, fenêtre de remplacement de 14 jours `:26`), et `enseigne_a` dans `referentiel_repo.rs:979`.

### 1.4 Routes et isolation
- `ent-api/src/routes.rs` (12 014 lignes) : `router()` (`:120`) avec 287 `.route` directes, puis 12 sous-routeurs fusionnés (`:738-757`) ; 552 routes au total ; un seul `AppState` (`lib.rs:57-66`) ; extracteur `EntSession` (`auth.rs:13-26`) ; middleware `contexte_droits` (tâche locale des droits), `exiger_module` (santé), `resoudre_tenant`.
- Les méthodes métier prennent le triplet `(etablissement_id, user_id, role: &str)` en paramètres positionnels (`get_eleve_for_actor`, `service.rs:5794`) : ajouter un paramètre (D-022 bis) oblige à modifier tous les appelants, labo compris.
- `ent-api/tests/separation.rs` (1 842 lignes, 41 tests) + `separation/harness.rs` (12 acteurs, cibles liée / non liée / autre établissement) + `separation/routes_src.rs` : l'inventaire **relit le texte** de 13 fichiers listés à la main (une garde de complétude vérifie que tout fichier contenant `.route(` y figure, `separation.rs:508-523`) ; un chemin non littéral est ignoré sans erreur (`routes_src.rs:80-82`) ; les routes publiques sont une liste centrale `PUBLIQUES` (`separation.rs:466-502`) ; sans base de données, les tests **passent sans rien jouer** (`harness.rs:349-357`, `separation.rs:157`).
- Garde SQL statique `ent-db/tests/isolation_sql_statique.rs` : toute requête sur une table à `etablissement_id` doit filtrer dessus (exceptions nominatives).

### 1.5 Le métier dans `ent-db`
- `service.rs` : 19 082 lignes, `pub struct PgEnt` (`:613`), un seul `impl` (`:698`), **452 `pub async fn`**, 6 séparateurs de section.
- **37 fichiers** ouvrent un bloc `impl PgEnt` (santé, finances, évaluations, cahier, EDT, inscriptions, dossier, référentiel…). Un `impl` inhérent ne peut pas sortir de son crate : c'est le verrou technique principal de l'extraction (§7, E7).
- Les domaines lisent les tables du noyau en SQL direct (comptage des mentions de `ent_eleve`, `ent_eleve_lien`, `ent_personne*`, `ent_foyer*`, `ent_classe`, `ent_affectation`, `ent_groupe*`, `ent_membership`, `ent_user`) : cahier 20, finances 15 + SEPA 4, EDT 15, évaluations 15, santé 6, notifications 4, inscriptions 37. Et ils **écrivent** dans le noyau : `finances.rs:2806` et `:2880` (`UPDATE ent_eleve` régime, date de sortie), `inscriptions.rs:2148-3715` (11 écritures dans `ent_eleve`, `ent_personne`, `ent_foyer`, `ent_eleve_lien`, `ent_user`, `ent_membership`), état du lot 7 (I1 et A1 en corrigent une partie par les ports de `dossier_personnes_ports.rs`). Autres écritures dans des tables `ent_eleve_*` créées par le noyau : `imports.rs:508`, `:593` (`ent_eleve`, `ent_eleve_lien`) ; `accompagnement.rs:343-421` et `document_repo.rs:269` (`ent_eleve_dispositif`) ; `repas_repo.rs`, `continuum_repo.rs`, `evaluations_bulletin.rs:540-564` (`ent_eleve_document`) ; `edt_remplacement.rs` et `edt_masse.rs` écrivent `ent_edt_seance`, table créée par le référentiel (`110_referentiel_structure.sql:199`, référencée par le cahier `150`). **Le préfixe d'une table ne dit donc pas son propriétaire** : la propriété de chaque table `ent_eleve_*` et de la séance est à fixer avant les extractions (Q-B15, Q-B17). Les fichiers santé `sante_acc.rs`, `infirmerie_repo.rs`, `psy_repo.rs`, `module_sante.rs` et les évaluations hors bulletin n'écrivent dans aucune table du noyau.
- `domain-eleve` (≈18 000 lignes, sans sqlx ni axum) : règles pures `plan_*` et dépôts-traits ; son périmètre dépasse l'élève (exports, facturation, messagerie, PDF).
- `seed.rs` (13 836 lignes) et `ent-lab/src/main.rs` (11 069 lignes) sont deux autres listes centrales (amorçage du labo, scénarios).

### 1.6 Cloisonnement
- `tenant_tx.rs:56-88` : `TenantTx::begin(pool, etab)` pose `app.etablissement_id` (et `app.rls_strict` en mode strict) **dans la transaction**. `RlsMode` par défaut : **`Off`** (`:30`, variable `ENT_RLS_MODE`).
- 45 tables sous RLS (`100_rls_pilote.sql`, `206_fin_rls.sql`, `215_sante_rls.sql`), `FORCE ROW LEVEL SECURITY`, politique permissive si la variable est absente et le mode non strict.
- `service.rs` n'ouvre **aucune** `TenantTx` : ses requêtes passent par `self.pool` ; l'isolation y repose sur les filtres `WHERE etablissement_id` et la garde statique. Les domaines plus récents (santé, finances, sanctions, accompagnement) ouvrent une `TenantTx`.
- **Sans RLS** : `ent_eleve`, `ent_outbox`, `ent_metier_event`, `ent_event_traitement`, `ent_notification`, `ent_journal_modif` (aucune migration ne les active).

### 1.7 Événements, fichiers, journal, IA, tâches
- **Outbox** (`102_outbox.sql`, `ent-db/src/outbox.rs`) : trait `Publier` sur `PgConnection` (`:63-91`, donc dans la transaction du fait), causalité et profondeur ≤ 5 ; trait `Consommateur { nom, types, rejouable, traiter(tx, evt) }` (`:216-230`) ; idempotence par `ent_event_traitement (consommateur, event_id)` dans la transaction de l'effet (`:631-672`) ; ordre par agrégat ; reprise exponentielle. **9 consommateurs**, assemblés par `service.rs:1034-1050` à partir de trois fonctions tenues à la main (`reactions.rs:43`, `inscriptions_reactions.rs:37`, `finances_reactions.rs:34`) : (`d0.notifications`, `d5.relance_appel`, `d7.notifications`, `d4.notifications_edt`, `d6.evaluations`, `d1.affectation_inscription`, `d2.invitations_responsables`, `d8.accueil_inscription`, `d10.facturation`).
- **Un second chemin d'émission** hors outbox et hors transaction subsiste : `PgEventStore::publish` (`event_store.rs:28-67`).
- **Fichiers** : `ent-object` (`Presigner::issue`, durée 60 s par défaut, 900 s au plus, clé vérifiée par établissement `presign.rs:704-708`), émission `service.rs:17114` (`emettre_lien`), service `GET /fichiers/{jeton}`. Antivirus par défaut : `AucuneAnalyse`.
- **Journal** : `ent_journal_modif` en ajout seul (trigger, `091_journal_ajout_seul.sql`) ; `journal_repo.rs:19` ; `tracer_acces_sensible` (`service.rs:3705`).
- **IA** : aucun registre de capacités, aucun client de modèle ; un copilote déterministe (`copilote.rs`, `service.rs:18798`).
- **Tâches de fond** : lancées à la main dans `ent-api/src/main.rs:89-151` (distributeur d'outbox, envois d'inscription toutes les 5 s, balayage des délégations, expéditeur de notifications, rappels de rendez-vous).

### 1.8 Frontend Leptos
- Workspace séparé `ent-rust/web/` (son propre `Cargo.lock`), dépendances backend par chemin. `ent-web-app` ne dépend ni d'`ent-api` ni d'`ent-db` ; les données passent par le client typé `ent-web-api` et le relais de session en processus.
- Un nouveau module d'écrans touche **trois listes** : `modules/mod.rs:4-34`, `lib.rs:60-127` (≈ 65 `<Route>`), `noyau/navigation.rs:77-190` (entrées de menu sous condition de droits) **et** les tests de navigation par profil (`:228-334`, `:439-461`), plus `ent-web-api/src/lib.rs:18-105` pour le client. Une seule feuille de style globale. Un seul WebAssembly.
- La règle « un module ne dépend pas d'un autre » est déjà violée une fois (`recuperation` → `connexion`).

### 1.9 CI
`.github/workflows/ent-rust.yml` : formatage (bloquant sur web et `ent-types`), audits, backend (clippy, build sans labo + contrôle du binaire, tests sans labo, **tests avec labo sur un seul fil : plus de 55 min**, `timeout-minutes: 120`), frontend (75 min). Déclenché sur `ENT` ; lancé à la main sur les branches d'intégration.

---

## 2. Principes [R]

1. **Un domaine possède ses données** (règle 1 de `SOCLES.md`) : il est le seul à écrire ses tables, et le seul à écrire ses migrations sur ses tables.
2. **Le noyau ne dépend d'aucune brique.** Une brique dépend du noyau. Une brique ne dépend jamais du code d'une autre brique (au plus de son crate de **types**, §4.8).
3. **Tout ce qui est partagé est une donnée déclarée, pas une ligne ajoutée à une liste** : migrations découvertes, actions déclarées, routes déclarées, consommateurs déclarés, tâches déclarées, capacités IA déclarées, écrans et entrées de menu déclarés. L'assemblage les **vérifie** (doublons, plages, préfixes, droits absents) et échoue avant de démarrer.
4. **La sécurité est fournie par construction, pas par discipline** : une brique n'a pas le pool PostgreSQL, elle n'a qu'une transaction d'établissement **où le RLS est toujours posé** ; un `Acteur` ne se fabrique pas ; une route n'est pas assemblée sans clause d'accès, et le kit applique cette clause avant le gestionnaire ; les routes publiques sont une liste fermée de l'assemblage ; les tests d'isolation sont générés depuis les déclarations et échouent sans base en CI ; une capacité IA sans droit est refusée. Tant que ces conditions ne sont pas toutes réunies (§7.3), l'isolation reste celle d'aujourd'hui : filtres applicatifs et garde SQL.
5. **Le noyau évolue par ports additifs** : on ajoute une fonction, on n'en change pas la signature sans période de transition (§3.3).
6. **Refonte ≠ correction.** Une étape de refonte ne change aucun comportement ; un défaut découvert (RLS, suivi des migrations…) devient un lot à part, avec sa décision.

---

## 3. Le noyau

### 3.1 Ce qu'il contient

| Module du noyau | Socle propriétaire | Données | Ports principaux (lecture / commande) |
|---|---|---|---|
| **plateforme** | 1 | transactions d'établissement, registres (migrations, actions, routes, consommateurs, tâches, capacités, écrans), outbox, journal en ajout seul, stockage objet et liens pré-signés, réglages de module | `tx(acteur)`, `publier`, `journal`, `fichiers`, `modules.actif` |
| **sécurité** | 1 | comptes, sessions, passkeys, OIDC, établissements, appartenances, rôles et droits (D-017), délégations, réauthentification | `acteur` (résolu par la requête), `autoriser`, `exiger_assurance` |
| **périmètre** | 1 + 5 + 6 (D-022) | aucune donnée propre : règle unique de visibilité calculée à partir des cours (5), des missions et services (6) et des liens familiaux (4) | `autoriser_eleve`, `eleves_visibles`, `cours_de(acteur)` |
| **élève** | 2 | identité versionnée, identifiants, scolarisations, inscription annuelle, régime, capacité juridique, confidentialité, photo, propositions, fusion | `eleve::resume`, `eleve::scolarite`, `eleve::regime_au`, commandes `creer_eleve`, `ouvrir_scolarisation`, `enregistrer_inscription_annuelle`, `cloturer_scolarisation`, `changer_regime`, `reintegrer` |
| **personnes et foyers** | 4 | personnes, foyers, adresses, liens de responsabilité et leurs droits, contacts d'urgence, remises et interdictions, payeurs | les ports déjà conçus par `04_ADMINISTRATIF.md` §5.3 : `responsables_actifs`, `titulaires_autorite`, `droits_lien`, `destinataires`, `joignables`, `remise`, `foyer_du_jour`, `commune_residence`, `payeurs`, `adresse_postale`, `integrer_declaration`, `correspondances` |
| **structure** | 5 | années, périodes, calendrier, classes, groupes, enseignements, **cours** (objet unique D-022), **séances datées** (proposé, Q-B15 : aujourd'hui `ent_edt_seance` est créée par le référentiel et écrite par l'EDT et les remplacements), sites, salles | `annee_courante`, `periodes`, `cours_de_l_eleve`, `eleves_du_cours(cours, jour)`, `classe_de(eleve, jour)`, `jour_ouvre` |
| **personnels** | 6 | engagements, missions datées, services d'enseignement, équipes | `missions_de(acteur, jour)`, `intervenants_du_cours`, `equipe_pedagogique(classe)` |
| **contrat IA** | 1 bis (`IA_COEUR.md`) | journal IA, brouillons, quotas (lot IA-L0) | registre `Capacite`, tests de garde (§5.4 d'`IA_COEUR.md`) |
| **points d'extension de la fiche 360°** | 2 | aucune donnée : la fiche est une **vue assemblée** | `SectionFiche`, `SourceChronologie`, `Pastille`, `Signal` déclarés par les briques |

Les six premiers modules sont les « socles 1 à 6 moins le 3 ».

**Ordre interne du noyau (sans cycle) [R]** : plateforme → sécurité (comptes, établissements) → structure (années, classes, groupes, cours, séances ; aucune clé vers l'élève) → élève (identité, scolarités, inscriptions annuelles, **affectations élève ↔ groupe**, qui référencent la structure) → personnes et foyers (référencent l'élève ; la capacité juridique est lue par port) → personnels (référencent personnes et structure) → périmètre (aucune table : lit tout par ports). Les dépendances inverses (la structure qui compte des élèves, l'élève qui affiche ses responsables) passent par des **lectures** à l'exécution, jamais par des clés étrangères ni des dépendances de crate. Les plages de migration du noyau suivent cet ordre (§8.1) ; un test sur `cargo metadata` refuse tout cycle entre crates `noyau-*`. **Les inscriptions (socle 3) sont une brique** : elles appellent les commandes de l'élève et des personnes (règle déjà écrite dans `02_ELEVE.md` §4 et `04_ADMINISTRATIF.md` §5.2) [R, Q-B7].

### 3.2 Ce qu'il ne contient pas
- Aucune table, aucune règle, aucun écran d'un domaine métier : notes, appel et vie scolaire, cahier de textes, emploi du temps, discipline, santé, facturation, paiements, cantine, internat, messagerie, notifications (socle 8 : c'est une brique qui **consomme** les événements et appelle le port `destinataires`), documents et signatures (socle 7 : le noyau stocke les octets et signe les liens ; la qualification et la conservation sont une brique), interopérabilité, paie, comptabilité.
- Aucun client de fournisseur externe propre à un domaine (SEPA, SMS, courriel, modèle de langage) : ils vivent dans la brique qui les emploie ou dans un crate d'adaptateur.
- Aucune liste de briques : c'est l'assemblage (binaire) qui les connaît.
- Pas de mécanisme générique de « champs libres » ou d'entité-attribut-valeur : le modèle reste relationnel et normalisé (CLAUDE.md, D-026).

### 3.3 Comment il reste stable pendant que les socles 2 à 6 l'enrichissent
1. **Les briques ne voient que les ports**, pas les tables ni l'implémentation. Quand le socle 5 remplacera la portée « classe texte » par l'objet « cours », la signature `eleves_visibles(acteur, action, jour)` ne bougera pas : seule son implémentation change. C'est ce qui permet de coder D-022 bis **maintenant** en marquant l'implémentation provisoire (objection du contradicteur, `PILOTAGE.md` §6).
2. **Un dossier (et à terme un crate) par module du noyau** (`noyau/eleve`, `noyau/personnes`, `noyau/structure`, `noyau/personnels`, `noyau/securite`, `noyau/plateforme`) : un lot du socle 2 et un lot du socle 4 ne touchent plus les mêmes fichiers.
3. **Ports additifs.** On ajoute un port ; pour en changer un, on ajoute la nouvelle forme, on marque l'ancienne `#[deprecated]`, on migre les briques, on retire l'ancienne quand plus rien ne l'appelle (clippy le montre). Jamais de changement de signature dans le même lot qu'une fonctionnalité.
4. **Tests de contrat** pour chaque port : comportement attendu, cas de refus (autre établissement, hors portée, lien suspendu), écrits une fois dans le noyau et rejoués à chaque lot.
5. **Un seul lot à la fois modifie la surface du noyau** (les signatures des ports, le kit) ; le contradicteur relit chaque changement de surface. Les enrichissements internes (nouvelles colonnes, nouvelles tables du noyau) ne sont pas limités.
6. **Vues de lecture publiées** (§3.4) : colonnes ajoutées seulement ; un test fige la liste des colonnes de chaque vue.

### 3.4 Lire les données du noyau
Les domaines existants joignent massivement les tables du noyau (§1.5) ; interdire toute jointure obligerait à charger les élèves en mémoire pour chaque liste de notes. Règle proposée [R, Q-B4] :
- **Écriture** dans une table du noyau : **interdite** hors des commandes du noyau, sans exception (garde statique, §4.6).
- **Contrôle d'accès** : **toujours** par les ports (`autoriser_eleve`, `eleves_visibles`, `droits_lien`), jamais en relisant les colonnes `peut_*` ou les affectations.
- **Lecture pour affichage et jointure** : autorisée **uniquement sur des vues publiées** par le noyau (`noyau_eleve_v`, `noyau_scolarite_v`, `noyau_cours_eleve_v`, `noyau_responsable_v`…), colonnes minimisées, `etablissement_id` toujours présent, contrat de colonnes figé par test. Les tables elles-mêmes restent privées au noyau. La transition (les domaines actuels joignent les tables) se fait brique par brique à l'extraction ; une liste d'exceptions datées, décroissante, est tenue par la garde.

---

## 4. Le kit de branchement

### 4.1 Forme d'une brique
```
ent-rust/briques/<nom>/
  Cargo.toml                 # crate brique-<nom> ; [package.metadata.scolia] plages = ["6400-6499"]
  migrations/6400_<nom>_….sql
  src/lib.rs                 # pub const BRIQUE: Brique = …  + impl du trait Branchement
  src/actions.rs             # declarer_actions! { … } : constantes + DEFINITIONS
  src/modele.rs              # règles pures (testables sans base), dans l'esprit de domain-eleve
  src/depot.rs               # SQL de la brique, toujours dans une TenantTx
  src/service.rs             # commandes et lectures « pour un acteur »
  src/routes.rs              # ROUTES: &[RouteDecl] + routeur construit depuis elles
  src/evenements.rs          # types d'événements publiés, consommateurs
  src/ia.rs                  # capacités (contrat IA_COEUR §5)
  src/labo.rs                # amorçage du labo et scénarios ent-lab de la brique
  tests/                     # isolation générée, garde SQL, simulation sans écriture, métier
  types/                     # crate brique-<nom>-types : DTO partagés avec le web (compile en wasm)
ent-rust/web/briques/<nom>/  # crate brique-<nom>-web : écrans Leptos, client, entrées de menu
```
Deux dossiers parce que le web est un workspace séparé (D-015) [R, Q-B11].

### 4.2 Le contexte d'appel : `Acteur`
Le triplet positionnel `(etablissement_id, user_id, role)` devient une valeur résolue une fois par requête [R] :
```rust
pub struct Acteur {                 // champs PRIVÉS, accesseurs en lecture seule
    etablissement_id: Uuid,
    user_id: Uuid,
    personne_id: Option<Uuid>,
    profil: Profil,                 // personnel | famille | élève (familles et élèves hors rôles libres, D-017)
    assurance: Assurance,           // passkey récente ou non (actions ★, D-009)
    canal: Canal,                   // web (cookie) | mobile (Bearer, D-003) | ia { tache_id } | tache_de_fond
    session_id: Option<Uuid>,
}
```
Un `Acteur` ne se construit **que** dans le noyau : par l'extracteur de session (cookie du web ou jeton Bearer des applications mobiles, qui aboutissent au même `Acteur` comme aujourd'hui `EntSession`), ou par `noyau.acteur_pour_tache(personne, etab)` qui **recharge** les droits de la personne au moment de l'exécution et refuse une personne sans session ni appartenance valides (IA_COEUR §4.4). Une brique ne peut pas fabriquer un acteur d'un autre établissement.
Ajouter une information de contexte (comme le jour de référence de D-022 bis) ne change plus aucune signature.

### 4.3 Les ports du noyau qu'une brique peut appeler
Forme indicative (le lot K5 fixe les noms) ; toutes les lectures sont **bornées par l'acteur**, toutes les écritures prennent la **transaction en cours** :
```rust
pub struct Noyau { /* privé : pool, presigner, registres */ }

impl Noyau {
    // Transactions : la SEULE porte vers PostgreSQL pour une brique.
    // Pose TOUJOURS app.etablissement_id et app.rls_strict, quel que soit ENT_RLS_MODE (§7.3).
    pub async fn tx(&self, acteur: &Acteur) -> Result<TenantTx<'static>, Erreur>;
    // Tâches de fond : seulement pour une tâche déclarée (TacheDecl) et un motif de sa liste fermée ; journalisé.
    pub async fn tx_systeme(&self, tache: &JetonTache, etab: EtablissementId) -> Result<TenantTx<'static>, Erreur>;

    // Droits et périmètre (D-017, D-022)
    pub async fn autoriser(&self, a: &Acteur, action: Action) -> Result<(), Refus>;
    pub async fn autoriser_eleve(&self, a: &Acteur, action: Action, eleve: EleveId) -> Result<(), Refus>;
    pub async fn eleves_visibles(&self, a: &Acteur, action: Action, jour: NaiveDate) -> Result<Visibilite, Erreur>; // Tous | Ensemble(..)
    pub async fn exiger_assurance(&self, a: &Acteur, niveau: Assurance) -> Result<(), Refus>;

    // Structure, personnels, élève, personnes (lectures minimisées)
    pub fn structure(&self) -> &PortsStructure;   // cours_de(a, jour), eleves_du_cours(cours, jour), classe_de(eleve, jour), periodes(annee)
    pub fn eleves(&self) -> &PortsEleve;          // resume(a, eleve), scolarite_au(eleve, jour), regime_au(eleve, jour)
    pub fn personnes(&self) -> &PortsPersonnes;   // foyer_du_jour, payeurs(eleve, categorie, jour), joignables, destinataires, remise, droits_lien

    // Événements (outbox, dans la transaction du fait)
    pub async fn publier<E: Evenement>(&self, tx: &mut TenantTx<'_>, a: &Acteur, e: &E) -> Result<EventId, Erreur>;

    // Fichiers privés : dépôt analysé, lien pré-signé court après contrôle
    pub async fn deposer(&self, tx: &mut TenantTx<'_>, a: &Acteur, espace: &'static str, f: FichierRecu) -> Result<FichierId, Erreur>;
    pub fn lien(&self, a: &Acteur, fichier: &FichierAutorise, disposition: Disposition) -> Result<LienFichier, Erreur>;

    // Journal (ajout seul) et consultations sensibles
    pub async fn journaliser(&self, tx: &mut TenantTx<'_>, a: &Acteur, j: EntreeJournal) -> Result<(), Erreur>;
    pub async fn tracer_acces_sensible(&self, a: &Acteur, eleve: Option<EleveId>, domaine: &'static str, quoi: &'static str);

    // Modules activables (généralise `exiger_module` de la santé, migration 216)
    pub async fn module_actif(&self, etab: EtablissementId, brique: &'static str) -> Result<bool, Erreur>;
}
```
`FichierAutorise` (type témoin) n'a qu'un constructeur privé au noyau, appelé par `autoriser_eleve` sur le propriétaire du fichier : une brique ne peut pas émettre un lien sans contrôle (leçon F-008). De même `JetonTache` n'est remis par l'assemblage qu'aux tâches déclarées.

**Limite assumée** : une `TenantTx` donne accès à une connexion SQL ; une brique peut techniquement y écrire n'importe quelle requête. Ce qui l'en empêche : le RLS strict (une requête ne voit que son établissement), la garde SQL statique (filtre, tables possédées, vues publiées) et la revue. Le kit ne promet pas plus.

### 4.4 Ce qu'une brique déclare
Tout est **dans son crate** ; l'assemblage le lit.

| Élément | Déclaration | Vérifié par |
|---|---|---|
| Identité | `BRIQUE.nom` (`infirmerie`), socle, préfixes de routes, domaines d'actions, préfixe d'événements | assemblage : noms et préfixes uniques |
| Migrations | fichiers de `migrations/`, découverts par `build.rs` ; plages dans `Cargo.toml` | prototype `ent-kit::migrations` : plage, doublon, chevauchement ; `MIGRATION_TAG` calculé ; garde F-009 ; garde de propriété (§4.6) |
| Actions D-017 | `declarer_actions! { PASSAGES_LIRE = "infirmerie.passages.lire", "…", Portee::TOUTES, sensible; … }` : la macro produit les constantes (`Action` = identifiant) **et** la liste `DEFINITIONS` (libellé, portées, `sensible`, `export`) | `Catalogue::assembler` : forme du code, unicité, domaine possédé, portée non vide ; une action ne peut pas exister sans définition |
| Modèles de rôle par défaut | `ContributionModele { modele: "infirmerie", action, portee }` | portée admise ; **une action marquée `export` n'est accordée qu'aux modèles administratifs** (D-022), contrôlé par l'assemblage (prototype) |
| Routes | `pub const ROUTES: &[RouteDecl]` (méthode, chemin, accès, cible, assurance exigée, couches : limite de corps, limitation de débit) ; le routeur axum est **construit depuis** cette liste, et le kit pose devant chaque gestionnaire l'exigence de session, le pré-contrôle « au moins une des actions » (`a_action_quelconque`), l'assurance (actions ★) et le module activable | `ent-kit::routes::verifier` : préfixe, doublon (aux noms de paramètres près), accès explicite, paramètre de cible ; une route `Public` doit figurer dans la **liste fermée de l'assemblage** ; inventaire de séparation = cette liste. Le contrôle fin (portée, élève) reste dans le service et n'est prouvé que par l'isolation générée |
| Événements publiés | `struct PassageEnregistre` + `impl Evenement { const TYPE = "infirmerie.passage_enregistre"; const VERSION = 1; }` (charge utile minimisée, jamais de valeur d'identité, `02_ELEVE.md` §13) | préfixe de brique ; noms historiques gelés |
| Consommateurs | `impl Consommateur` (trait existant `outbox.rs:216`) ; nom stable `infirmerie.<effet>` ; `rejouable() = false` pour un effet externe | nom unique ; types consommés existants |
| Tâches de fond | `TacheDecl { nom, periode, motifs, executer: fn(&Noyau, JetonTache) -> BoxFuture }` | seule voie vers `tx_systeme` ; une tâche qui agit pour une personne obtient son acteur par `acteur_pour_tache` (droits rechargés, IA_COEUR §4.4) |
| Capacités IA | `pub const CAPACITES: &[Capacite]`, type défini par `ent-ia` comme prévu par `IA_COEUR.md` §4.2 (le kit ne fait que les recueillir) ; droits « tous requis » côté IA | tests de garde du cœur IA : droit non vide, simulation obligatoire, équivalence écran / IA, isolation, simulation sans écriture |
| Fiche 360° et chronologie | `SectionFiche { id, titre, sensibilite, charger }`, `SourceChronologie`, `Signal` ; les **pastilles** restent la projection `ent_eleve_indicateur` de `02_ELEVE.md`, alimentée par les événements de la brique (déclaration = type d'indicateur et événement source) | sensibilité déclarée ; chargement par port borné à l'acteur |
| Amorçage du labo | `fn amorcer_labo(monde) -> ObjetsIsolation` : un objet lié à l'élève lié, un à l'élève non lié, un dans l'établissement B | nécessaire aux tests d'isolation générés |
| Écrans Leptos | crate `brique-<nom>-web` : `#[component(transparent)] fn Routes()`, `pub const NAVIGATION: &[EntreeNav]` (chemin, libellé, groupe, ordre, condition sur les droits) | test de navigation par profil **de la brique** ; parité de la condition d'écran avec l'action serveur |
| Module activable | `BRIQUE.activable = Some(par_defaut)` | le noyau renvoie 404 pour toutes les routes d'un module inactif (généralise la migration 216) |

### 4.5 Les tests que le kit fournit à chaque brique
Une brique les obtient en écrivant une ligne par famille (`ent_kit_test::isolation!(brique_infirmerie::BRIQUE);` etc.) :
1. **Isolation générée** depuis `ROUTES` (prototype : `routes::cas_isolation`) : sans session → 401 ; personnel sans l'action → refus sans donnée ; action accordée hors portée → refus ; famille non liée → refus ou réponse indistincte ; identifiant de l'établissement B avec un jeton de A → indistinct ; oracle de fuite sur toute réponse 2xx. Les acteurs et le monde sont ceux du harnais actuel (`separation/harness.rs`), déplacé dans le kit.
2. **Garde SQL** : filtre `etablissement_id` (garde actuelle) + **aucune écriture hors de ses tables** + aucune lecture de table du noyau hors vues publiées (§3.4). Propriété des tables : déclarée par la brique (liste de ses tables) et recoupée avec les `CREATE TABLE`, pas déduite du préfixe (§1.5).
3. **RLS** : toute table de la brique portant `etablissement_id` a `relrowsecurity` et `relforcerowsecurity` (requête sur `pg_class`), la politique du noyau étant posée par `SELECT ent_rls_activer('<table>')` dans la migration ; **et** les tests de la brique tournent avec un rôle `NOBYPASSRLS` en mode strict (comme `tests_rls.rs`), pour prouver que la politique filtre.
4. **Migrations** : plage, numéro, garde F-009, rejeu deux fois de suite sans erreur ni changement de schéma (empreinte de schéma avant / après, déjà calculée par `pra.rs`).
5. **Contrat IA** : les tests de garde d'`IA_COEUR.md` §5.4.
6. **Catalogue** : instantané des actions et contributions de la brique (un changement de code d'action casse le test : il faut une migration de `ent_role_droit`).

### 4.6 Ce qu'une brique n'a pas le droit de faire (et la garde qui l'en empêche)
| Interdit | Garde |
|---|---|
| Ouvrir une connexion avec le pool ou une `TenantTx` sans acteur | le pool n'est pas exposé ; `tx_systeme` exige un motif et journalise |
| `INSERT` / `UPDATE` / `DELETE` sur une table qui n'est pas la sienne | garde SQL statique avec la carte « table → brique » déduite des `CREATE TABLE` |
| Migration qui modifie une table d'une autre brique ou du noyau (`ALTER TABLE`, contrainte, index, trigger) — cas de la 212 aujourd'hui | garde de migration (même carte) |
| Redéfinir une contrainte déjà définie | garde F-009 (prototype) |
| Dépendre du crate serveur d'une autre brique | test sur `cargo metadata` : une brique ne dépend que du noyau, du kit, de crates externes et de crates `*-types` |
| Publier un événement hors outbox (`PgEventStore::publish`) ou hors transaction | l'API n'est pas exposée aux briques |
| Lancer une tâche détachée qui appelle un service (`tokio::spawn`, `task::spawn`, `JoinSet`, `spawn_blocking`) | la seule voie vers une transaction hors requête est `TacheDecl` + `JetonTache` ; une tâche détachée n'a ni `Acteur` valide ni jeton ; garde textuelle en complément |
| Se déclarer route publique | liste fermée de l'assemblage (prototype : `RoutePubliqueNonAutorisee`) |
| Accorder un export à un professeur | marqueur `export` + contrôle de l'assemblage (prototype) |
| Décider d'un droit côté écran seulement | parité condition d'écran / action serveur ; isolation générée |

### 4.7 Comment les briques se parlent
1. **Par défaut : événements** (outbox) et, si la brique a besoin de données d'une autre pour ses propres écrans, une **projection** locale tenue par un consommateur (ex. : la cantine garde le régime de l'élève à jour en consommant `eleve.regime_changed`).
2. **Commandes vers le noyau** : toujours par ses ports (les inscriptions créent l'élève par `creer_eleve`).
3. **Lecture synchrone d'une autre brique** quand une projection serait absurde (le bulletin a besoin des absences de la période) [R, Q-B5] : la brique fournisseuse déclare une **lecture publiée** dont le type est dans son crate `-types` (trait `LecturePubliee` + DTO) ; l'implémentation est enregistrée dans le registre du noyau à l'assemblage ; la brique cliente l'appelle par `noyau.lecture::<AbsencesPeriode>()`. La cliente dépend du **contrat** (types), jamais du code. Si la brique fournisseuse est absente ou son module inactif, la lecture renvoie « non disponible » et la cliente l'affiche comme tel.

### 4.8 bis Correspondance avec les conceptions existantes
| Source | Ce qu'elle prévoit | Forme retenue ici |
|---|---|---|
| D-017 | portées établissement, mes classes, mes groupes, désignés, **moi-même** | les 4 premières existent dans le code (`Portee`) ; « moi » manque (Q-B16) |
| D-022 | export réservé aux administratifs, portée au groupe partout | marqueur `export` contrôlé par l'assemblage ; `eleves_visibles` unique |
| `IA_COEUR.md` §4.2 | `Capacite` et registre dans `ent-ia`, implémentations dans `ent-db/ia_repo.rs` | `Capacite` reste dans `ent-ia` ; chaque brique implémente ses capacités chez elle ; le registre est assemblé comme les autres |
| `IA_COEUR.md` §4.4 | tâches détachées : droits rechargés | `acteur_pour_tache` |
| `02_ELEVE.md` §4, §13 | commandes du socle 2 appelées par les autres ; pastilles projetées | ports `eleves` ; projection `ent_eleve_indicateur` conservée |
| `04_ADMINISTRATIF.md` §5.3 | ports à paramètres explicites, écritures avec `&mut TenantTx` | **repris tels quels** sous `noyau.personnes()` ; `droits_lien(personne, eleve, droit, date)` reste sans `Acteur` (c'est un contrôle, appelé par le noyau) ; seules les lectures « pour afficher » prennent l'`Acteur` |
| D-009, D-011 | passkey, actions ★, sessions web courtes / mobiles longues | `Acteur.assurance` ; `RouteDecl` porte l'assurance exigée |
| D-003 | mobile natif, jeton Bearer | même `Acteur`, `canal = mobile` ; les routes des briques servent web et mobile |

### 4.8 Ce qui reste partagé (et comment cela ne conflicte plus)
- `ent-rust/assemblage/src/briques.rs` : une ligne par brique (`brique_infirmerie::BRIQUE,`) ; `ent-rust/assemblage/Cargo.toml` : une ligne de dépendance ; idem `ent-web-app/src/briques.rs`. **Ordre alphabétique, une entrée par ligne**, `.gitattributes` `briques.rs merge=union` : deux branches qui ajoutent chacune une brique fusionnent sans conflit ; un test de complétude vérifie que chaque dossier de `briques/` est listé et que la liste ne contient pas de doublon [R, Q-B6].
- `docs/socles/MIGRATIONS.md` reste le **registre de réservation** des plages avant le lancement d'une session (règle 3 de `PILOTAGE.md`) ; le kit vérifie ensuite que les plages déclarées ne se chevauchent pas.

---

## 5. Mode d'emploi : créer la brique « infirmerie » (pour une session Sonnet)

> Contexte à lire d'abord : `CLAUDE.md`, ce document §4, la conception du socle concerné (« Santé et infirmerie » = socle 34 de `SOCLES.md` ; aujourd'hui son code est dans `ent-db/src/{sante_acc,infirmerie_repo,psy_repo,module_sante}.rs`, extrait en brique `sante` à l'étape E7 ; une session qui crée « infirmerie » après E7 **étend cette brique** plutôt que d'en créer une seconde, car un domaine d'actions n'appartient qu'à une brique — remplacer `infirmerie` par `sante` ci-dessous). Le mode d'emploi vaut pour toute brique nouvelle.

**0. Avant de coder**
- Vérifier que ta plage est réservée dans `docs/socles/MIGRATIONS.md` (sinon : arrêt, demande au pilote). Règle proposée : plage d'une brique `3000 + 100 × numéro de socle` (socle 34 → `6400–6499`, Q-B2).
- Lister, à partir de la conception du socle : tables, actions D-017, routes, événements émis et consommés, capacités IA, écrans. Tu ne crées rien qui ne soit dans la conception sans le signaler.

**1. Créer le squelette** (une commande du kit, lot K6 : `scripts/nouvelle-brique.sh infirmerie 34`) : dossiers §4.1, `Cargo.toml` avec la plage, `BRIQUE` vide qui compile, ligne ajoutée dans les deux `briques.rs` de l'assemblage. Lancer `cargo test -p brique-infirmerie` : les tests générés passent à vide.

**2. Migrations** : `migrations/6400_infirmerie_passages.sql`. Règles :
- `CREATE TABLE IF NOT EXISTS`, `ADD COLUMN IF NOT EXISTS` : la migration doit pouvoir être rejouée (tant que le suivi des migrations n'existe pas, Q-B10) ;
- toute table porte `etablissement_id UUID NOT NULL` et se termine par `SELECT ent_rls_activer('ent_infirmerie_passage');` ;
- clés étrangères **seulement vers des tables du noyau** (`ent_eleve`, `ent_personne`…) ou de ta brique ; jamais vers une autre brique ;
- une contrainte `CHECK` nommée s'écrit **toujours** `ALTER TABLE … DROP CONSTRAINT IF EXISTS x;` puis `ALTER TABLE … ADD CONSTRAINT x CHECK (…);`, jamais dans le `CREATE TABLE IF NOT EXISTS` (sur une base existante, la table existe : la contrainte du `CREATE` ne serait jamais mise à jour) ; elle n'est écrite que dans **une** migration ; pour l'élargir, modifier cette migration-là, en élargissant seulement (F-009 ; la garde vérifie les deux règles) ;
- limites du moteur actuel (`migrate.rs:197-238`) : pas de ligne commençant par `--` à l'intérieur d'un bloc `DO $$ … $$`, pas de `;` dans un littéral, seuls les délimiteurs `$$` sont reconnus ;
- rien sur une table qui n'est pas à toi (pas d'`ALTER TABLE ent_eleve`) : si le noyau doit changer, c'est une demande au propriétaire du socle.
Rien d'autre à faire : pas de liste à compléter, pas de `MIGRATION_TAG`.

**3. Actions** (`src/actions.rs`) :
```rust
ent_kit::declarer_actions! {
    PASSAGES_LIRE = "infirmerie.passages.lire", "Lire les passages à l'infirmerie", Portee::TOUTES, sensible;
    PASSAGES_SAISIR = "infirmerie.passages.saisir", "Enregistrer un passage", Portee::ETABLISSEMENT_SEULE, sensible;
    REGISTRE_EXPORTER = "infirmerie.registre.exporter", "Exporter le registre", Portee::ETABLISSEMENT_SEULE, sensible, export;
}
pub const MODELES: &[ContributionModele] = &[
    ContributionModele { modele: "infirmerie", action: PASSAGES_SAISIR, portee: Portee::Etablissement },
    ContributionModele { modele: "infirmerie", action: PASSAGES_LIRE,   portee: Portee::Etablissement },
];
```
Codes en `domaine.objet.verbe`, domaine = ta brique. Un code publié ne se renomme plus (il est stocké dans `ent_role_droit`). Toute action d'export porte `export` : l'assemblage refuse de l'accorder à un modèle non administratif (D-022).

**4. Règles pures** (`src/modele.rs`) : validations, calculs, transitions d'état, sans base ni réseau ; tests unitaires exhaustifs, cas limites de la conception.

**5. Dépôt et service** (`src/depot.rs`, `src/service.rs`) :
```rust
pub async fn enregistrer_passage(n: &Noyau, a: &Acteur, saisie: SaisiePassage) -> Result<PassageId, ErreurInfirmerie> {
    n.autoriser_eleve(a, PASSAGES_SAISIR, saisie.eleve_id).await?;         // 1. droit et portée
    let regles = modele::valider(&saisie)?;                                 // 2. règles pures
    let mut tx = n.tx(a).await?;                                            // 3. transaction de l'établissement (RLS)
    let id = depot::inserer_passage(&mut tx, a.etablissement_id, &regles).await?;
    n.journaliser(&mut tx, a, EntreeJournal::creation("infirmerie.passage", id)).await?;
    n.publier(&mut tx, a, &PassageEnregistre { passage_id: id, eleve_id: saisie.eleve_id }).await?; // 4. fait + journal + outbox : une transaction
    tx.commit().await?;
    Ok(id)
}
```
Pour une liste : `n.eleves_visibles(a, PASSAGES_LIRE, jour)` **dans la requête SQL** (filtre), pas après. Toute lecture de données sensibles appelle `tracer_acces_sensible`. Pour afficher nom et classe, joindre les **vues publiées** du noyau (`noyau_eleve_v`), jamais `ent_eleve`.

**6. Routes** (`src/routes.rs`) :
```rust
pub const ROUTES: &[RouteDecl] = &[
    RouteDecl { methode: Get,  chemin: "/infirmerie/eleves/{eleve_id}/passages",
                acces: Acces::Personnel { actions: &[PASSAGES_LIRE] }, cible: Cible::Eleve { parametre: "eleve_id" } },
    RouteDecl { methode: Post, chemin: "/infirmerie/passages",
                acces: Acces::Personnel { actions: &[PASSAGES_SAISIR] }, cible: Cible::Aucune },
];
pub fn routeur() -> Router<EtatNoyau> { kit::routeur(ROUTES, |r| match r.chemin { /* handler par route */ }) }
```
Le kit pose devant le gestionnaire la session, le pré-contrôle d'action, l'assurance et le module activable ; le gestionnaire extrait `Acteur`, appelle le service, mappe l'erreur ; le **service** fait le contrôle fin (portée, élève). Une route sans accès, hors de `/infirmerie`, dont le paramètre de cible manque, ou publique sans figurer dans la liste fermée de l'assemblage, ne passe pas l'assemblage. Pour une action ★, déclarer l'assurance forte dans la route **et** l'exiger dans le service.

**7. Événements et consommateurs** : publier `infirmerie.passage_enregistre` (charge utile : identifiants seulement). Pour prévenir la famille, ne pas envoyer toi-même : la brique notifications consomme l'événement. Un consommateur de ta brique : nom `infirmerie.<effet>`, idempotent par construction (le noyau marque `ent_event_traitement` dans la même transaction).

**8. Capacités IA** (`src/ia.rs`) : une fiche par lecture et par commande ouverte à l'IA (IA_COEUR §5.1–5.3) ; santé = sensibilité C3, servie seulement avec `ia.donnees_sensibles` ; commandes avec simulation et texte d'aperçu rédigé par toi.

**9. Fiche 360° et chronologie** : déclarer la pastille « passage récent » et la source de chronologie (titre neutre, sans motif médical).

**10. Labo** (`src/labo.rs`) : amorçage minimal (un passage pour l'élève lié, un pour le non lié, un dans l'établissement B) et un scénario `ent-lab` de la brique.

**11. Écrans** (`web/briques/infirmerie/`) : composants dans le style de `docs/FRONTEND_LEPTOS.md` (lire d'abord) ; `Routes()` sous `/web/infirmerie/*` ; `NAVIGATION` avec la condition `droits.a(PASSAGES_LIRE)` ; client typé à partir des DTO de `brique-infirmerie-types`. Aucun `fetch` à la main, aucun `inner_html`.

**12. Critère de fin du lot** : `cargo test -p brique-infirmerie` (métier, isolation générée, garde SQL, RLS, migrations rejouées, contrat IA) ; `cargo test -p assemblage` (catalogue et routes assemblés, complétude) ; build sans labo et contrôle du binaire ; clippy ; tests web de la brique ; **aucun fichier modifié hors `briques/infirmerie/`, `web/briques/infirmerie/` et les deux lignes d'assemblage** (vérifié par `git diff --stat`) ; `PROJECT_STATUS.md` à jour.

---

## 6. Découpage cible en crates [R]

```
                         ┌──────────────────────── assemblage (binaire ent-api) ──────────────────────┐
                         │ briques.rs : liste ; migrations, catalogue, routes, consommateurs, tâches  │
                         └───────┬───────────────────────────────┬────────────────────────────────────┘
                                 │                               │
        ┌──────────── briques/ (une par domaine) ────────┐       │
        │ brique-sante  brique-finances  brique-cahier …  │──┐    │
        │      │ (contrat seulement)                      │  │    │
        │ brique-*-types (DTO, wasm)                      │  │    │
        └──────────────────────────────────────────────────┘  │    │
                                                              ▼    ▼
   ┌──────────────────────────────── noyau ─────────────────────────────────────────────────┐
   │ ent-noyau (façade : Noyau, Acteur, ports)                                               │
   │   noyau-eleve · noyau-personnes · noyau-structure · noyau-personnels · noyau-perimetre  │
   │   noyau-securite (ent-auth, ent-oidc) · noyau-plateforme (TenantTx, outbox, journal,    │
   │   fichiers via ent-object, registres) · ent-kit (déclarations pures, sans dépendance)   │
   └─────────────────────────────────────────────────────────────────────────────────────────┘
     ent-types (DTO du noyau) · ent-events · domain-eleve (règles pures du noyau, recentré)
     adaptateurs : ent-mail · ent-sms · ent-push · ent-bank · ent-object · ent-obs
web/ : ent-web-ui · ent-web-api (client du noyau) · ent-web-noyau (coquille, navigation, droits)
       · web/briques/<nom> · ent-web-app (assemblage : briques.rs) · ent-web-server · ent-web-session
```
- **ent-kit** : le prototype, complété (Capacite, EntreeNav, TacheDecl, Evenement) ; aucune dépendance, compile en wasm (le web l'utilise pour la navigation).
- **Briques visées par l'extraction** (domaines déjà codés) : `sante` (santé, infirmerie, PAI, psy, aménagements, module activable), `finances` (facturation, SEPA, bourses), `communication` (S2-8), `evaluations` (évaluations, bulletins, conseils, livrets, LSU/LSL), `edt` (emploi du temps, remplacements), `cahier`, `inscriptions`, puis `vie-scolaire` (appel, absences, discipline), `notifications`, `rencontres`, `formulaires`, `campagnes`, `imports`.
- **domain-eleve** se recentre sur les règles pures du noyau ; ses modules de domaine (facturation, exports, messagerie…) migrent vers le `modele.rs` de la brique concernée.
- **Temps de compilation** [H, à mesurer à chaque étape] : un crate `ent-db` de 115 000 lignes recompile en entier à chaque modification ; découpé, une modification dans une brique ne recompile que la brique et l'assemblage. Les requêtes sqlx sont dynamiques (0 macro `query!`) : aucune base n'est nécessaire à la compilation. Le nombre de crates augmente le travail d'édition de liens ; critère : pas plus de 10 % de régression du temps de build propre de la CI, gain mesuré du build incrémental.

---

## 7. Plan de migration par étapes

**Règles communes à toutes les étapes**
- Elles commencent **après l'arrivée des lots 7 et 8 sur `ENT`**, chacune repart de la tête d'`ENT` et passe par la **file d'intégration unique** comme un lot (PILOTAGE §2 règle 5) : jamais en parallèle d'une autre intégration.
- **Aucun changement de comportement** : même liste de tests (aux chemins de module près), mêmes résultats, mêmes quatre jobs CI verts, et les **instantanés de référence** de l'étape E0 inchangés (sauf ceux que l'étape remplace par un calcul, qui doit donner la même valeur).
- Une étape = une branche courte (un à deux jours de travail Sonnet), fusionnée aussitôt validée, pour ne pas vieillir.
- Les corrections découvertes en route sont notées, pas faites (§7.3).

| Étape | Contenu | Conflits supprimés | Critère de fin |
|---|---|---|---|
| **E0 — Référence** | Instantanés commités, comparés **comme des ensembles** : `cargo test -- --list` par crate ; catalogue (codes, libellés, portées admises) et modèles par défaut (`(profil, action, portée)`) ; inventaire des routes (méthode, chemin, fichier) ; **couches par route** (limite de corps, débit, module, ordre des middlewares) ; consommateurs et types ; empreinte de schéma après migrations ; `MIGRATION_TAG`. Ajout de `.gitattributes`. | — | instantanés générés par un test (régénérables), CI verte |
| **E1 — Migrations découvertes** | `ent-db/build.rs` (code de `ent-kit::decouverte`) génère `MIGRATIONS` et `MIGRATION_TAG` ; suppression de la liste manuelle ; garde F-009 ; test d'égalité avec l'instantané E0. | `migrate.rs`, `MIGRATION_TAG` (touchés par 4/4 branches) | liste générée identique à l'instantané (noms, ordre, contenu) ; `/health` renvoie le même tag ; CI verte |
| **E2 — Actions déclarées** | `Action` devient un identifiant (`struct Action(&'static str)`, défini dans `ent-kit`, compilable en WebAssembly, ré-exporté par `ent-auth`) ; chaque variante devient une constante associée **du même nom** (`Action::NotesLire`) dans un bloc « historique » : les ≈ 460 usages compilent sans changement ; libellés, portées, sensibilité et export passent dans des `DEFINITIONS` par domaine (`declarer_actions!`), éclatées en `actions/<domaine>.rs` ; `TOUTES` (devient une tranche : adapter `roles.rs:61`, `tests_edt_s2.rs:1925`) et `modele_par_defaut` calculés. À réécrire : les 4 `use Action::*` d'`ent-auth`, `droits_parite.rs` du web. **Recul à compenser** : l'exhaustivité des `match` de `code()`/`libelle()` disparaît ; la macro (une constante ne peut exister sans définition) et l'instantané E0 la remplacent. | `actions.rs` (4/4), `roles.rs` | instantanés du catalogue et des modèles identiques (ensembles) ; tests de `actions.rs` et `tests_roles.rs` verts |
| **E3 — Routes déclarées** | Chaque fichier de routes expose `ROUTES: &[RouteDecl]` et construit son sous-routeur depuis cette liste ; **les couches restent posées par sous-routeur** comme aujourd'hui (aucun déplacement de middleware dans cette étape) ; l'inventaire de séparation est lu dans les déclarations ; l'ancien lecteur de texte est gardé **une étape** et doit donner le même ensemble ; `PUBLIQUES` devient la liste fermée de l'assemblage. `routes.rs` est découpé en fichiers par domaine (sans changer de crate). Le pré-contrôle d'action par le kit (§4.4) est **un changement de comportement** possible (un refus plus tôt) : il vient dans un lot à part, après E4. | `routes.rs` (3/4), `routes_src.rs`, inventaire de `separation.rs` | inventaire déclaré = inventaire texte = instantané E0 ; couches par route = instantané E0 ; 41 tests de séparation verts |
| **E4 — Isolation générée** | Le harnais devient un module du kit ; les cas générés (`cas_isolation`) s'ajoutent aux spécifications écrites, qui sont réparties par domaine (`tests/separation/<domaine>.rs`). **Préalable** : Q-B9 (échec sans base en CI), sans quoi les cas générés peuvent passer sans être joués. | `separation.rs` (4/4) | toutes les spécifications existantes présentes (comptage) ; cas générés joués et verts en CI |
| **E5 — Registres** | Consommateurs, tâches de fond, amorçage du labo et scénarios `ent-lab` déclarés par domaine et assemblés (fin des listes de `service.rs:1034`, `main.rs:89-151`, `seed.rs`, `ent-lab/main.rs`). | `service.rs` (consommateurs), `ent-api/main.rs`, `seed.rs`, `ent-lab/main.rs` (3/4) | mêmes 9 noms de consommateurs, mêmes types, mêmes tâches (périodes), même labo (scénarios identiques au journal près) |
| **E6 — Kit et façade du noyau** | `ent-kit` (issu du prototype) + `ent-noyau` : `Acteur`, `Noyau`, ports **implémentés en déléguant** aux fonctions existantes ; vues publiées du noyau (migration additive : des `CREATE VIEW`, aucun changement de table) ; `ent_rls_activer()`. Aucun code métier déplacé. | prépare E7 | tests identiques ; tests de contrat des ports verts ; documentation des ports |
| **E7 — Extraction des domaines** | **Préalables** : lot RLS (§7.3 : `Noyau::tx` pose toujours le RLS strict pour les tables de briques, tests en rôle `NOBYPASSRLS`) et propriété des tables fixée (Q-B15, Q-B17). Un lot par domaine, dans cet ordre : **santé** (pilote : `sante_acc`, `infirmerie_repo`, `psy_repo`, `module_sante` — déjà en `TenantTx`, sous RLS, module activable, **aucune écriture dans le noyau** ; l'accompagnement, qui écrit `ent_eleve_dispositif`, attend Q-B17), **finances** (ses écritures dans `ent_eleve` remplacées par les commandes du socle 2 dans le même lot, c'est donc le premier lot qui *corrige* : à faire valider), **communication**, **évaluations**, **EDT**, puis **cahier** (après D-022 bis, qui le touche), **inscriptions** (après I1). Pour chacun : migrations déplacées dans la brique (ordre global inchangé, §8.1), actions, routes, consommateurs, tests, écrans. Les méthodes `impl PgEnt` deviennent un **trait d'extension** de la brique (`pub trait SanteExt` implémenté pour `PgEnt`) : les appelants (labo, tests) ne changent que par un `use` ; la conversion vers `Noyau`/`Acteur` suit dans le même lot si elle ne change rien, sinon dans un lot suivant. Accès transitoire au pool par `#[doc(hidden)] fn pool_historique()`, tenu dans une liste d'exceptions décroissante. | les fichiers du domaine quittent `ent-db` ; `ent-db/lib.rs` (4/4) | pour chaque brique : mêmes tests (déplacés), mêmes instantanés, même inventaire ; `git diff --stat` montre des déplacements, pas de réécriture de SQL |
| **E8 — Web** | **Après W0** (preuve Leptos 0.8 : `Routes()` transparentes venant d'un autre crate, hydratation, taille WASM). Dans `ent-web-app` d'abord, **un module à la fois** (CLAUDE.md) : navigation et routes déclarées (fin des trois listes) ; puis un crate par brique sous `web/briques/` ; tests de navigation par brique ; chaque crate ajouté met à jour `web/Cargo.lock` dans le même lot. | `lib.rs`, `modules/mod.rs`, `navigation.rs`, `ent-web-api/lib.rs` | mêmes routes Leptos, mêmes entrées de menu par profil (instantané), même HTML serveur, taille du WebAssembly ±2 % |
| **E9 — Le noyau en modules** | Ce qui reste d'`ent-db` (`service.rs` et voisins) est réparti par module du noyau (§3.1), d'abord en dossiers, puis en crates si le temps de compilation le justifie. | `service.rs` (4/4) | tests identiques ; `service.rs` n'existe plus ou n'est plus qu'une façade |

### 7.1 Ne pas télescoper les branches en cours (E3–E4, A1, I1, D-022 bis)
1. **Rien ne démarre avant le lot 8 sur `ENT`.** Les quatre branches entrent ensuite dans la file, dans l'ordre du pilotage (E3-E4 → A1 → I1 → D-022 bis).
2. **E1 s'insère juste après le lot 8, avant E3-E4** [R, Q-B1] : c'est l'étape la plus petite et la plus rentable (4/4 branches touchent `migrate.rs`), et sa résolution est mécanique. **Recette pour chaque branche en cours, au moment de son intégration** : « dans `migrate.rs`, prendre la version d'`ENT` ; tes fichiers `.sql` sont découverts tout seuls ; supprimer ta modification de `MIGRATION_TAG` ». La recette est écrite dans la consigne de l'intégrateur, et vérifiée par le test d'égalité (la liste générée contient bien les nouveaux fichiers).
3. **E2 à E5 attendent la fin de la file actuelle** (après D-022 bis) : leurs conflits avec des branches qui ajoutent des actions et des routes ne sont pas mécaniques. Si la file prend du retard, E2 peut passer avant I1 et D-022 bis avec un **script de conversion** (`scripts/briques/e2-convertir-actions.py`, lot K2) qui transforme les variantes ajoutées par une branche en constantes ; le pilote tranche au moment voulu.
4. **Les extractions (E7) ne touchent jamais un domaine qu'une branche ouverte modifie.** Santé, finances, communication, évaluations, EDT : aucune des quatre branches n'y touche (vérifié §1.1) ; cahier (touché par 3 branches) après D-022 bis ; inscriptions et dossier après I1 et A1 ; élève, personnes, périmètre (noyau) en dernier (E9).
5. **Gel des fichiers centraux pendant une étape** : l'étape annonce les fichiers qu'elle déplace ; aucune session de code ne démarre sur ces fichiers tant qu'elle n'est pas sur `ENT`. Les nouvelles sessions de code partent de la tête d'`ENT` (règle existante).
6. **Nouvelles sessions lancées pendant la refonte** : si elles démarrent avant E6, elles codent « à l'ancienne » mais **dans des fichiers neufs** (un fichier par sujet, pas d'ajout dans `service.rs` ni `routes.rs`) ; après E6, elles reçoivent le §5.

### 7.2 Lots de code pour l'équipe Sonnet

| Lot | Étape | Contenu | Modèle | Dépend de |
|---|---|---|---|---|
| K0 | E0 | instantanés de référence, `.gitattributes`, test « instantanés à jour » | Sonnet | lot 8 sur ENT |
| K1 | E1 | `build.rs` d'`ent-db`, suppression de la liste, garde F-009, égalité avec E0 | Sonnet | K0 |
| K2 | E2 | `Action` en structure à constantes associées, `actions/<domaine>.rs`, script de conversion | Sonnet (+ revue `review-sonnet` : droits) | K1, fin de la file actuelle |
| K3 | E3 + E4 | `RouteDecl`, routeurs construits depuis les déclarations, découpage de `routes.rs`, harnais dans le kit, cas générés | Sonnet (+ revue sécurité) | K2 |
| K4 | E5 | registres des consommateurs, tâches, amorçages, scénarios | Sonnet | K3 |
| K5 | E6 | `ent-kit`, `ent-noyau` (Acteur, ports délégués, vues publiées, `ent_rls_activer`), tests de contrat | **Opus pour la surface des ports** (décision structurante), Sonnet pour l'implémentation | K4 |
| K6 | E6 | `scripts/nouvelle-brique.sh`, gabarit, test de complétude de l'assemblage | Sonnet | K5 |
| R1 | §7.3 | RLS appliqué aux transactions des briques (mode strict forcé pour leurs tables, rôle `NOBYPASSRLS` en test, RLS sur les 6 tables qui en manquent) — **changement de comportement, décision Q-B18** | Sonnet + revue sécurité Opus | K6 |
| B1 | E7 | brique `sante` (pilote, sans l'accompagnement) ; retour d'expérience écrit avant B2 | Sonnet | R1, Q-B15, Q-B17 |
| B2–B5 | E7 | `finances`, `communication`, `evaluations`, `edt` (un lot chacun, l'un après l'autre dans la file) | Sonnet | B1 |
| B6, B7 | E7 | `cahier` (après D-022 bis), `inscriptions` (après I1) | Sonnet | B5 |
| W0 | E8 | vérification Leptos 0.8 : `Routes()` transparentes depuis un autre crate, hydratation, taille WASM | Sonnet | K6 |
| W1, W2 | E8 | navigation déclarée dans `ent-web-app`, puis crates `web/briques/*` | Sonnet | W0 |
| N1… | E9 | répartition du reste d'`ent-db` par module du noyau (un module par lot) | Sonnet | B7 |

### 7.3 Hors refonte : changements de comportement révélés (lots séparés, décisions à part)
- **Suivi des migrations appliquées** (table `ent_migration_appliquee` avec empreinte, application dans une transaction par migration, refus d'une migration modifiée après application) : supprime le risque de rejeu F-009 à la racine [Q-B10].
- **RLS (lot R1, préalable de E7)** : `Noyau::tx` pose toujours le mode strict pour les briques ; activer la RLS sur `ent_eleve`, `ent_outbox`, `ent_metier_event`, `ent_event_traitement`, `ent_notification`, `ent_journal_modif` ; passer `service.rs` sur `TenantTx` (fait naturellement par E9) ; `ENT_RLS_MODE=strict` par défaut en pilote.
- Pré-contrôle d'action et d'assurance posé par le kit devant les gestionnaires (refus plus précoce, mêmes résultats attendus) : lot à part après E4.
- Retirer le chemin d'émission `PgEventStore::publish` hors transaction.
- Les 6 contraintes redéfinies : les ramener à une seule définition **après** le suivi des migrations (avant, la correction serait elle-même un rejeu risqué).
- Écritures de la facturation dans `ent_eleve` (régime, sortie) : à remplacer par les commandes du socle 2 (`changer_regime`, `cloturer_scolarisation`), au plus tard dans B2.
- Antivirus par défaut `AucuneAnalyse` (socle 7).

---

## 8. Risques et parades

### 8.1 Rejeu des migrations (F-009)
- **Ordre global** : déplacer un fichier d'un dossier à l'autre ne change rien tant que l'ordre d'application reste « par numéro » ; on garde donc **un seul ordre, numérique**, et on choisit les plages pour qu'il soit aussi l'ordre des dépendances [R, Q-B2, Q-B3] :
  - **< 1000 : historique**, appliqué dans son ordre actuel, quelle que soit la brique qui possède le fichier après extraction ;
  - **1000–2999 : noyau**, par module dans l'ordre de §3.1 (plateforme et sécurité 1000–1199, structure 1200–1399, élève 1400–1599, personnes et foyers 1600–1799, personnels 1800–1999, périmètre et IA 2000–2199, réserve 2200–2999) ;
  - **≥ 3000 : briques**, `3000 + 100 × numéro de socle` (socle 3 inscriptions → 3300, socle 34 santé → 6400, socle 51 → 8100).
  Ainsi toute migration du noyau précède toute migration de brique, et les inscriptions (socle 3) passent après les personnes : aucune clé étrangère vers une table pas encore créée. Le `MIGRATION_TAG` reste « le plus haut numéro », cohérent avec cet ordre (prototype). Une brique ne référence que le noyau ; le noyau ne référence jamais une brique — l'exception historique `ent_edt_seance → ent_edt_creneau` (`110_referentiel_structure.sql:199`) disparaît si la séance et le créneau sont rangés dans la structure (Q-B15).
  - Limite : avec un rejeu systématique, une nouvelle migration du noyau (ex. 1450) ajoutée après une brique (6400) s'applique sans problème ; avec le futur suivi des migrations (Q-B10), il faudra autoriser l'application « hors ordre » d'une migration de numéro inférieur, comme le permettent les outils usuels.
- **Rejeu** : chaque brique a un test « appliquer deux fois, même empreinte de schéma » ; la garde F-009 interdit une nouvelle redéfinition de contrainte ; la garde de propriété interdit à une brique de toucher une table du noyau (la 212 actuelle est l'exception historique gelée).
- **Numérotation** : passage à 4 chiffres pour les plages ≥ 1000 (`numero_de` du prototype n'accepte aujourd'hui que 3 chiffres, et son test fixe la plage historique à `1..=999` : à étendre dans K1) ; les plages déjà réservées < 1000 restent valables ; tri numérique (et non alphabétique) obligatoire, car `999_x` < `1000_y`.
- **Contraintes** : gabarit `DROP CONSTRAINT IF EXISTS` + `ADD CONSTRAINT` imposé et vérifié (§5 étape 2) ; la garde du prototype lit les lignes de SQL : elle ne voit ni les noms construits dans un bloc `DO` (`format('%I')`), ni les commentaires `/* */`, ni les index, triggers et fonctions redéfinis (`CREATE OR REPLACE`) : à compléter dans K1.
- **Échec à mi-parcours** (pas de transaction) : inchangé par la refonte ; traité par le suivi des migrations (§7.3).

### 8.2 Droits et RLS
- Le risque principal de la refonte est **de perdre un contrôle en déplaçant du code** (F-008). Parades : instantané du catalogue et des modèles (E0) ; tests de séparation inchangés + cas générés ; E7 déplace sans réécrire le SQL ; revue `review-sonnet` ciblée sur les droits à K2, K3 et B1.
- Dans le kit, une brique **ne peut pas** contourner : pas de pool, routes à accès obligatoire, liens de fichiers à type témoin, capacités IA à droit obligatoire.
- RLS : en mode `Off` par défaut, la RLS n'est aujourd'hui qu'un filet ; le test RLS par brique vérifie au moins que la politique **existe**, et la CI joue une suite en `ENT_RLS_MODE=strict` (à ajouter avec le lot RLS de §7.3).
- Familles : le contrôle unique `droits_lien` (socle 4) est un port ; aucune brique ne relit `ent_eleve_lien`.

### 8.3 Temps de compilation et CI
- Mesurer à chaque étape : build propre, build incrémental après modification d'un fichier de brique, durée des suites. Critère : ±10 % sur le build propre, gain sur l'incrémental.
- Gain attendu sur F-012 : une brique se teste seule (`cargo test -p brique-x`), en minutes, sur un conteneur qui redémarre ; la CI peut lancer un job par brique (matrice) en plus de la suite complète. La suite complète en un fil reste la référence jusqu'à ce que l'isolation des tests entre eux soit prouvée.
- Le workspace web a son propre `Cargo.lock` : chaque crate de brique web s'y ajoute ; garder les versions Leptos épinglées (CLAUDE.md).

### 8.4 Leptos
- [H] Composer des routes Leptos 0.8 venant de plusieurs crates (`#[component(transparent)]` renvoyant `impl MatchNestedRoutes + Clone`) : à prouver par W0 avant W2. Repli si cela ne tient pas : les écrans restent des modules d'`ent-web-app` (un dossier par brique), avec navigation et routes **déclarées** (la moitié du gain, sans crates séparés).
- Le WebAssembly reste unique : découper en crates ne réduit pas sa taille ; le découpage du WASM (chargement différé) est une question à part, plus tard.
- `#![recursion_limit = "512"]`, `disable-erase-components`, CSP sans style ni script en ligne : à reprendre dans chaque crate de brique web (gabarit K6).
- La violation existante `recuperation → connexion` est à lever dans W1 (la configuration de connexion passe dans `ent-web-noyau`).

### 8.5 Autres risques
- **Noyau-dieu** : tout tirer dans le noyau par facilité. Parade : la table §3.1 est la liste fermée ; toute addition au noyau passe par le contradicteur.
- **Renommages** : noms de consommateurs (`d0.notifications`…), types d'événements, codes d'action et chemins de routes sont des **identifiants publiés** : jamais renommés par la refonte (liste « historique gelée » dans l'assemblage).
- **Branches qui vieillissent** pendant la refonte : étapes courtes, file unique, recettes de résolution écrites (§7.1).
- **Coût** : une dizaine de lots Sonnet de refonte pure avant le premier gain sur les nouvelles briques ; E1 rapporte dès le lot suivant.

---

## 9. Prototype `ent-kit` (branche `ent-architecture-briques`)

- **Où** : `ent-rust/crates/ent-kit/` (membre du workspace, **aucune dépendance**, non utilisé par `ent-db` ni `ent-api` : aucun comportement modifié).
- **Contenu** : `decouverte.rs` (lecture du répertoire, partagée avec `build.rs`) ; `build.rs` (génère `MIGRATIONS_NOYAU` par `include_str!`) ; `migrations.rs` (plages, assemblage, tag calculé, garde F-009) ; `actions.rs` (actions-constantes, catalogue, contributions aux modèles) ; `routes.rs` (déclarations, contrôles, cas d'isolation) ; `brique.rs` (déclaration complète d'une brique, assemblage vérifié, exemple « infirmerie »).
- **Résultats** (`cargo test -p ent-kit`, Rust 1.97 local ; CI en 1.99) : **19 tests réussis** (14 unitaires, 4 contre le code existant dans `tests/existant.rs`, 1 exemple de documentation) :
  - la liste découverte est **égale** à la liste manuelle de `migrate.rs` (146 migrations, même ordre) et le tag calculé vaut `255_email_confirmation` ;
  - les 6 redéfinitions de contraintes existantes sont gelées ; une septième fait échouer le test ;
  - les 68 codes du catalogue actuel ont la forme exigée par le kit (`domaine.verbe`, uniques) ;
  - `Action` réduite à son code sert de motif de `match`, y compris sous un nom de constante associée identique à l'ancienne variante (`Action::NotesLire`), et l'égalité ne porte que sur le code ;
  - `declarer_actions!` produit constantes et définitions ensemble ; un export accordé au modèle `prof` est refusé (D-022) ; une route publique hors de la liste de l'assemblage est refusée ; deux routes identiques aux noms de paramètres près sont refusées.
- `cargo clippy -p ent-kit --all-targets -- -D warnings` : propre. `cargo fmt -p ent-kit` appliqué (pas `--all`).
- **Ce que le prototype ne prouve pas** : la construction du routeur axum depuis les déclarations et la pose des couches (K3), la composition des routes Leptos entre crates (W0), le gain de temps de compilation (à mesurer), le trait d'extension pour sortir un `impl PgEnt` (B1 ; [H] : les futures d'un `async fn` de trait implémenté pour un type concret restent `Send` pour axum), le RLS forcé (R1).
- **Limites connues du prototype** : numéros à 3 chiffres seulement ; garde F-009 par lignes (voir §8.1) ; le test des codes existants relit le texte d'`actions.rs` ; `cas_isolation` ne connaît pas l'assurance (actions ★) ni le rattachement d'un `Cible::Objet` à son élève ; les jokers `{*reste}` d'axum ne sont pas traités ; `build.rs` écrit des chemins absolus dans le code généré (le binaire ne change pas, mais le fichier généré dépend de la machine) ; un `.sql` au nom non conforme fait échouer la compilation (voulu), les fichiers cachés sont ignorés.
- **Attention** : le test d'égalité lit `ent-db/src/migrate.rs` ; il sera remplacé à l'étape E1 (la liste manuelle disparaît). Une branche qui ajoute une migration sans l'ajouter à `migrate.rs` le fait échouer — comme le test existant de `migrate.rs`.

---

## 10. Récapitulatif pour le pilote
1. Faire relire ce document par le contradicteur (objections dans `PILOTAGE.md` §6).
2. Faire trancher par Florian les questions §11 (au minimum Q-B1 à Q-B4).
3. Après le lot 8 sur `ENT` : K0 puis K1 dans la file ; recette E1 dans les consignes d'intégration de E3-E4, A1, I1, D-022 bis.
4. Après D-022 bis : K2 → K6, puis B1 (santé) et son retour d'expérience, puis le reste de §7.2.
5. Les nouvelles conceptions de socle (7 et suivants) peuvent dès maintenant décrire leur brique selon §4 (plage `3000 + 100 × socle` si Q-B2 est acceptée, préfixes, actions, routes, événements, capacités).

---

## 11. Questions à trancher

| # | Question | Recommandation |
|---|---|---|
| Q-B1 | Insérer E1 (migrations découvertes) juste après le lot 8, **avant** E3-E4, avec une recette de résolution pour les branches en cours ? | **Oui** : petite étape, conflit mécanique, supprime le conflit qui touche 4 branches sur 4. E2 et suivantes après D-022 bis. |
| Q-B2 | Numérotation à 4 chiffres : noyau 1000–2999 par module, briques `3000 + 100 × numéro de socle` ? | **Oui** (§8.1) : plus de négociation de plage, place pour 100 migrations par socle, et l'ordre numérique devient l'ordre des dépendances ; les plages < 1000 déjà réservées restent. |
| Q-B3 | Un seul ordre d'application, numérique (historique, puis noyau, puis briques par construction des plages) ? | **Oui** ; c'est ce que fait déjà le prototype. |
| Q-B4 | Les briques peuvent-elles **lire** le noyau en SQL ? | **Oui, seulement par des vues publiées**, jamais les tables ; écriture toujours par commande ; contrôle d'accès toujours par port. |
| Q-B5 | Lecture synchrone entre briques par « lectures publiées » (dépendance au seul crate de types) ? | **Oui**, par défaut événements et projections ; lecture publiée quand une projection serait absurde. |
| Q-B6 | Liste d'assemblage explicite (une ligne par brique, fusion « union », test de complétude) plutôt qu'un enregistrement automatique par l'éditeur de liens (`inventory`, `linkme`) ? | **Liste explicite** : auditable, sans magie de plateforme (WebAssembly), aucune dépendance nouvelle à analyser ; le coût est une ligne sans conflit. |
| Q-B7 | Inscriptions (socle 3) : brique ou noyau ? | **Brique** : elle appelle les commandes de l'élève et des personnes (déjà la règle de `02_ELEVE.md` et `04_ADMINISTRATIF.md`). |
| Q-B8 | Vie scolaire, appel, notifications, documents : briques ? | **Oui** ; le noyau ne garde que le stockage des fichiers et les ports `destinataires` / `joignables`. |
| Q-B9 | Les tests de séparation doivent-ils **échouer** quand la base manque en CI (aujourd'hui ils passent sans rien jouer) ? | **Oui en CI** (variable `ENT_EXIGER_BASE=1`), comportement local inchangé ; c'est un changement de test, à décider à part de la refonte. |
| Q-B10 | Introduire le suivi des migrations appliquées (fin du rejeu systématique) ? | **Oui, après E9**, lot dédié avec plan de bascule sur une base existante (marquer l'historique comme appliqué). |
| Q-B11 | Une brique = un dossier backend + un dossier dans `web/` (deux workspaces) ? | **Oui** : on garde la séparation des workspaces (D-015) ; repli §8.4 si la composition Leptos ne tient pas. |
| Q-B12 | Qui peut modifier la surface du noyau (ports, kit) ? | **Un seul lot à la fois**, relu par le contradicteur ; enrichissements internes libres. |
| Q-B13 | Conserver les noms historiques (`d0.notifications`…, `eleve.created`…) ? | **Oui**, gelés dans une liste ; les nouveaux noms suivent `<brique>.<effet>`. |
| Q-B14 | Le code existant de santé devient la brique `sante` ; une future « infirmerie » s'y ajoute plutôt que d'être une seconde brique ? | **Oui** (socle 34 « Santé et infirmerie ») : un domaine d'actions n'a qu'une brique. |
| Q-B15 | Les séances datées et les créneaux (`ent_edt_seance`, `ent_edt_creneau`) : noyau (structure) ou brique EDT ? | **Noyau (structure)** : l'appel, le cahier de textes, les remplacements et l'EDT s'y rattachent ; l'EDT et les remplacements les **modifient par commandes** du noyau. |
| Q-B16 | Ajouter la portée « moi-même » (D-017, `ia.*` d'IA_COEUR §5.5) au catalogue ? | **Oui**, dans un lot à part (nouvelle valeur, pas de changement des droits existants). |
| Q-B17 | Propriété des tables `ent_eleve_*` écrites par des domaines (`ent_eleve_dispositif` par l'accompagnement, `ent_eleve_document` par bulletins, repas, continuum, `ent_eleve_periode` par la facturation) ? | Le **noyau élève** garde `ent_eleve` et ses historiques (`ent_eleve_periode` : écrite par commande) ; `ent_eleve_dispositif` passe à la brique santé/accompagnement (socle 35) ; `ent_eleve_document` devient une commande du noyau « déposer au dossier » (socle 7 plus tard). Liste complète à dresser en K0. |
| Q-B18 | Lot R1 avant la première extraction : RLS strict forcé pour les tables de briques, RLS sur les 6 tables qui en manquent ? | **Oui** : sans lui, la « sécurité par construction » du kit n'est pas tenue. C'est un changement de comportement, donc une décision et un lot à part, avec revue de sécurité. |

---

## 12. Faits, hypothèses, contradictions

- **Faits** (§1, §9) : vérifiés dans le code de `ent-integration-lot7` et par `git diff` des branches en cours, le 2026-11-07.
- **Hypothèses** : composition des routes Leptos entre crates (W0) ; `Send` des futures d'un trait d'extension (B1) ; gain de compilation (à mesurer).
- **Contradictions et écarts relevés** : la documentation parle d'une « matrice 40/40 », le code compte 41 tests de séparation ; l'en-tête de `separation.rs` annonce des cas `#[ignore]` qui n'existent pas ; deux routes publiques en double dans `PUBLIQUES` ; `ent-types/lib.rs:12-13` dit qu'`ent-api` n'utilise pas encore ces types alors qu'il les importe ; `outbox.rs:17-18` cite `ent.etablissement_id` alors que le code pose `app.etablissement_id` ; `ent-web-session/Cargo.toml` se dit prototype alors qu'il est intégré au serveur ; le test de `migrate.rs` ne vérifie que le dernier nom et le nombre de fichiers ; le test d'`actions.rs` ne détecte pas une variante oubliée dans `TOUTES`. Aucune n'est corrigée ici (hors périmètre, aucun comportement changé).

---

## 13. Relecture indépendante (2026-11-07) et traitement
Revue `review-sonnet` en lecture seule : les faits du §1 vérifiés sont exacts, sauf trois imprécisions (corrigées : verrou `migrate.rs:168-188`, assemblage des consommateurs par trois fonctions, garde de complétude existante de l'inventaire). Objections et traitement :

| # | Objection | Traitement |
|---|---|---|
| 1 | « Sécurité par construction » non tenue : RLS `Off` par défaut, `TenantTx` donne une connexion libre | **Fait** : `Noyau::tx` pose toujours le RLS strict pour les briques, tests en `NOBYPASSRLS`, lot R1 préalable de E7 (Q-B18), limite assumée écrite (§4.3) |
| 2 | `Acteur`, `tx_systeme`, `FichierAutorise`, garde `tokio::spawn` contournables | **Fait** : champs privés, constructeurs réservés au noyau, `JetonTache`, `acteur_pour_tache` (§4.2–4.6) |
| 3 | `RouteDecl.acces` n'est qu'une métadonnée ; routes publiques auto-déclarées | **Fait** : pré-contrôle posé par le kit (lot à part, changement possible), liste fermée des routes publiques (prototype), Q-B9 préalable de E4 |
| 4 | Ordre des migrations faux avec `1000 + 100 × socle` ; séance du noyau dépendant de l'EDT | **Fait** : plages par couche (§8.1, Q-B2, Q-B3), question Q-B15 sur la séance |
| 5 | « Modifier la migration d'origine » sans effet si la contrainte est dans un `CREATE TABLE IF NOT EXISTS` | **Fait** : gabarit `DROP … IF EXISTS` + `ADD` imposé et vérifié (§5) |
| 6 | E2 : `use Action::*`, perte d'exhaustivité, égalité sur tous les champs, `Action` en double, wasm | **Fait** : `Action` = identifiant seul dans `ent-kit`, définitions par macro, sites à adapter listés (§7 E2) ; prototype refait et testé |
| 7 | D-022 « pas d'export au professeur » laissée aux briques | **Fait** : marqueur `export`, contrôle de l'assemblage, test du prototype |
| 8 | Noyau non acyclique | **Fait** : ordre interne publié (§3.1), contrôle de cycle sur `cargo metadata` |
| 9 | Écritures dans le noyau sous-estimées ; santé pas propre (accompagnement) | **Fait** : liste complétée (§1.5), pilote santé sans l'accompagnement, Q-B17 |
| 10 | E3 perdrait les couches axum | **Fait** : couches gardées par sous-routeur, instantané des couches en E0 |
| 11 | Écarts avec D-017 (« moi »), IA_COEUR (`Capacite` dans `ent-ia`, tâches), 02 (pastilles), 04 (ports) | **Fait** : table de correspondance §4.8 bis, Q-B16 |
| 12 | Imprécisions de faits | **Fait** (ci-dessus) |
| 13 | Limites du moteur de migrations (commentaires, `;`) | **Fait** : règles ajoutées au §5 |
| 14 | Limites du prototype non signalées | **Fait** : §9 ; doublons aux noms de paramètres près et cas « famille non liée » des listes corrigés dans le prototype |
| 15 | E8 en bloc, avant W0 ; mobile Bearer absent | **Fait** : W0 avant E8, un module à la fois ; `Acteur` commun cookie / Bearer (§4.2, §4.8 bis) |

