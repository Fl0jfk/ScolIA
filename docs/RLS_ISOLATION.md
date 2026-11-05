# Isolation des établissements en base : Row-Level Security (socle 1, lot C4)

Statut : **pilote implémenté + testé en local** (2026-11-05, branche `ent-socle1-rls`). Non activé par défaut. Ni déployé, ni validé, ni prêt production.
Vocabulaire : *fait* (vérifié dans le code ou en test), *décision de conception* (à valider), *hypothèse*.

## 1. Pourquoi
Faits (audit du 2026-11-05) : l'isolation ne repose que sur le filtre `etablissement_id` écrit dans chaque requête (533 usages directs de `self.pool`, 57 repos). Un filtre oublié, une jointure laxiste ou une requête copiée suffisent à faire fuiter des données d'un autre établissement. La RLS ajoute une barrière **dans PostgreSQL** : même une requête sans filtre ne peut lire ou écrire que l'établissement de la transaction.

Ce que la RLS ne couvre pas (à ne pas oublier) :
- une injection SQL ou un code malveillant qui peut exécuter du SQL arbitraire peut aussi poser `app.etablissement_id` à une autre valeur : la RLS protège des **oublis de filtre**, pas d'un attaquant qui contrôle le SQL ;
- le choix de l'établissement reste applicatif (session, adresse D-012) : la RLS ne fait que le faire respecter jusqu'aux lignes ;
- les droits d'un rôle à l'intérieur d'un établissement (prof, parent…) restent applicatifs (lot B).

## 2. Conception

### 2.1 Deux rôles PostgreSQL (décision de conception)
| Rôle | Rôle | Droits |
|---|---|---|
| propriétaire / migration (`ent_owner`) | exécute les migrations (DDL), possède les tables | tout sur son schéma ; **n'est pas** utilisé par l'application en fonctionnement |
| applicatif (`ent_app`) | utilisé par `ent-api` | `SELECT/INSERT/UPDATE/DELETE` sur les tables ; ni propriétaire, ni `SUPERUSER`, ni `BYPASSRLS`, ni DDL ; aucun droit de désactiver un trigger |

Pourquoi : un propriétaire de table peut `ALTER TABLE … DISABLE ROW LEVEL SECURITY`, `DROP POLICY` ou `DISABLE TRIGGER` (journal en ajout seul, migration 091). Seul un rôle non propriétaire est réellement lié par ces garde-fous. Faits : aujourd'hui l'application rejoue les migrations au démarrage avec le même rôle, donc propriétaire.

Variables (implémenté) :
- `ENT_DATABASE_MIGRATION_URL` (facultative) : URL du rôle propriétaire ; `PgEnt::migrate()` ouvre une connexion dédiée, applique les migrations puis la ferme. Absente ou vide : comportement actuel (migrations via `ENT_DATABASE_URL`). Même garde-fou « hôte local » que `ENT_DATABASE_URL`.
- `ENT_DATABASE_URL` : rôle applicatif.

### 2.2 Variable de session et transaction
- `app.etablissement_id` (UUID) posé par `SELECT set_config('app.etablissement_id', $1, true)` : le troisième argument `true` limite la valeur à **la transaction** ; à `COMMIT`/`ROLLBACK` la connexion revient au pool sans valeur résiduelle (testé). `SET` de session est proscrit (fuite d'un établissement vers la requête suivante via le pool).
- Adaptateur Rust `TenantTx` (`ent-db/src/tenant_tx.rs`) : `TenantTx::begin(&pool, etablissement_id)` ouvre la transaction, pose la variable, se déréférence en `PgConnection` (on écrit `.execute(&mut *tx)`), `commit()` explicite, rollback à la destruction. L'établissement vient de la session (lot C3, type `Tenant`, à faire) ; aujourd'hui des paramètres déjà présents dans les signatures des repos.

### 2.3 Politiques (migration `100_rls_pilote.sql`)
Pour chaque table pilote : `ENABLE ROW LEVEL SECURITY`, `FORCE ROW LEVEL SECURITY` (la politique s'applique aussi au propriétaire ; seuls superutilisateur et `BYPASSRLS` y échappent), politique unique `ent_rls_etablissement` `FOR ALL` avec `USING` et `WITH CHECK` identiques :

```
etablissement_id = (SELECT ent_rls_etablissement())
OR ((SELECT ent_rls_etablissement()) IS NULL AND NOT (SELECT ent_rls_strict()))
```
- `ent_rls_etablissement()` : `nullif(current_setting('app.etablissement_id', true), '')::uuid` ; `ent_rls_strict()` : `app.rls_strict ∈ {on, 1, true}`. Fonctions `STABLE`, appelées dans un `(SELECT …)` pour n'être évaluées qu'une fois par requête (InitPlan) et non par ligne.
- Variable posée → seules les lignes de cet établissement (lecture, mise à jour, suppression) ; une insertion/mise à jour vers un autre établissement échoue (`42501`, `WITH CHECK`).
- Variable absente et mode **non strict** → permissif (comportement actuel : rien ne casse tant que tous les accès ne sont pas migrés).
- Variable absente et mode **strict** → 0 ligne en lecture, écriture refusée (fail-closed).

### 2.4 Modes
| `ENT_RLS_MODE` (côté application, lu une fois) | Effet de `TenantTx` | Effet en base |
|---|---|---|
| `off` (défaut, ou absent / inconnu) | ne pose rien | politiques permissives : aucun changement de comportement |
| `permissif` | pose `app.etablissement_id` | les accès migrés sont filtrés par la base ; un accès non migré reste permis |
| `strict` | pose aussi `app.rls_strict = on` pour la transaction | idem, et un accès sans établissement voit 0 ligne |

Recommandation pour la production (décision de conception) : le mode strict **au niveau du rôle** — `ALTER ROLE ent_app SET app.rls_strict = 'on'` — afin qu'un accès qui oublie `TenantTx` échoue de façon visible au lieu de rester permissif sans bruit. `ENT_RLS_MODE=strict` suffit pour les essais ou si le réglage de rôle est impossible. Attention : avec `ALTER ROLE … SET`, `ENT_RLS_MODE=off` (rien posé) rend alors les tables pilotes illisibles ; ne l'activer qu'avec `permissif` ou `strict`.

### 2.5 Tables pilotes (22, migration 100)
- santé : `ent_fiche_sante`, `ent_pai`, `ent_pai_piece`, `ent_infirmerie_passage` ;
- psychologue : `ent_psy_entretien`, `ent_psy_rdv`, `ent_psy_dossier` ;
- vie scolaire / discipline : `ent_incident_type`, `ent_sanction_type`, `ent_sanction_motif`, `ent_incident`, `ent_sanction`, `ent_vs_commission`, `ent_vs_commission_dossier` ;
- facturation : `ent_fac_facture`, `ent_fac_ligne`, `ent_fac_encaissement`, `ent_fac_echeance`, `ent_fac_quittance`, `ent_fac_sepa_mandat`, `ent_fac_sepa_prelevement`, `ent_fac_rappel`.

Repos migrés vers `TenantTx` (toutes leurs méthodes, une transaction par appel) : `PgInfirmerieRepo`, `PgPsyRepo`, `PgSanctionRepo`, `PgFacRepo` (`ent-db/src/{infirmerie,psy,sanction,fac}_repo.rs`). Les méthodes qui ouvraient déjà une transaction (`insert_facture`, `insert_encaissement_and_maybe_solde`, `insert_echeances`…) utilisent maintenant `TenantTx` ; `insert_echeances` ne fait plus rien (pas même une transaction) sur une liste vide. Contrôle : aucun autre fichier de `ent-rust/` ne référence ces tables.

Les clés étrangères (ex. `eleve_id → ent_eleve`) sont vérifiées par PostgreSQL sans RLS : elles ne fuient pas de ligne mais peuvent révéler l'existence d'un identifiant (erreur de contrainte). Le lot C5 (FK composites) traite cet angle.

### 2.6 Tables exclues, et pourquoi
Lues **avant** de connaître l'établissement, donc incompatibles avec une politique par établissement :
- identité et accès : `ent_user`, `ent_membership`, `ent_session`, `ent_passkey_credential`, `ent_passkey_challenge`, `ent_oidc_provider` / `ent_oidc_identity` / `ent_oidc_state`, `ent_password_reset`, `ent_rate_limit` (limitation de débit), `ent_etablissement` et `ent_etablissement_domaine` (résolution de l'hôte, D-012) ;
- le journal `ent_journal_modif` : protégé par son trigger « ajout seul » et par les droits du rôle, pas par la RLS (un événement concerne parfois un acteur sans établissement) ;
- tables de référence globales sans `etablissement_id`.
Pour ces tables l'isolation reste applicative ; le rôle applicatif est limité à ses droits DML.

### 2.7 Workers et tâches transverses
- Principe : un worker (relances de facture, envoi d'e-mails/SMS, purges) **traite un établissement à la fois** : il liste les établissements via une table exclue, puis ouvre un `TenantTx` par établissement. Aucun rôle « tout voir ».
- Si un traitement transverse légitime doit lire plusieurs établissements (statistiques de plateforme, migration de données), il passe par le rôle **propriétaire/migration** hors application (jamais via un `BYPASSRLS` donné à `ent_app`), de façon journalisée.
- Les tâches actuelles sur les 22 tables ne sont lancées que par une requête qui porte son établissement : aucun worker transverse à adapter dans le pilote (fait, vérifié par recherche dans `ent-rust/`).

### 2.8 Plan d'extension table par table
1. Tant que le pilote tourne en `permissif`/`strict` en environnement de test : mesurer (cf. 2.9) et corriger les accès qui oublient `TenantTx` (ils échouent en strict).
2. Vague 2 (données élèves sensibles) : `ent_eleve_document` / `ent_document_grant`, `ent_absence*`, `ent_note*`, messagerie (`ent_msg*`), `ent_carnet_entree`, `ent_conseil_classe_dossier`, RH (`ent_absence_rh`) — liste à confirmer table par table.
3. Vague 3 : le reste des tables avec `etablissement_id` (EDT, appel, inscriptions, repas, internat…), table par table via la même fonction `DO $$` de la migration (ajouter le nom dans la liste).
4. Pour chaque vague : migrer le repo (un `TenantTx` par méthode ou par cas d'usage), ajouter le test strict (lecture, mise à jour, insertion refusée), vérifier le plan d'exécution des requêtes chaudes.
5. Quand toute table de domaine est couverte : `app.rls_strict = on` au niveau du rôle, `ENT_RLS_MODE=strict` en production, suppression du repli permissif (une migration remplace la politique par la version stricte seule).
6. Étapes liées : C3 (type `Tenant` produit uniquement par la session et exigé par `TenantTx::begin`) et C5 (FK composites).

### 2.9 Performance (hypothèses à mesurer)
- Coût fixe par appel migré : `BEGIN`, `set_config`, `COMMIT` = environ 2 à 3 allers-retours réseau en plus (en `off` : `BEGIN`/`COMMIT` seulement). Sur une base distante à 1–2 ms, 3–6 ms par appel ; les méthodes déjà transactionnelles ne paient que `set_config`. Hypothèse : sans effet visible pour les usages actuels (pilote) ; à mesurer avant de généraliser (`pgbench` ou test de charge sur `/vie-scolaire` et la facturation).
- Coût par requête : le prédicat `etablissement_id = <constante>` est de la même forme que le filtre applicatif déjà présent et utilise les mêmes index (`etablissement_id` en tête sur les tables pilotes). Les fonctions sont `STABLE` et enveloppées dans `(SELECT …)` pour n'être évaluées qu'une fois par requête.
- Pool : une transaction par méthode garde une connexion un peu plus longtemps ; pool actuel de 5 connexions. À surveiller si des méthodes migrées appellent des services lents à l'intérieur d'une transaction (non fait dans le pilote : pas d'appel externe dans une transaction).
- Risque de plan : `current_setting` n'est pas une constante pour le planificateur (le plan générique d'une requête préparée ne peut pas utiliser la valeur). Sans effet sur des filtres déjà présents.

## 3. Création du rôle applicatif (script d'exemple)
**Exemple, à adapter ; ne jamais exécuter sur une base réelle sans revue.** Mots de passe fictifs. À exécuter par un administrateur, une fois la base créée et les migrations appliquées au moins une fois par le propriétaire.

```sql
-- 1. Propriétaire (migrations) : possède les objets.
CREATE ROLE ent_owner LOGIN PASSWORD '<secret-migration>' NOSUPERUSER NOBYPASSRLS;
-- (la base et le schéma public lui appartiennent : ALTER DATABASE ... OWNER TO ent_owner ; ALTER SCHEMA public OWNER TO ent_owner;)

-- 2. Applicatif : droits minimaux.
CREATE ROLE ent_app LOGIN PASSWORD '<secret-appli>'
    NOSUPERUSER NOBYPASSRLS NOCREATEDB NOCREATEROLE NOINHERIT;
GRANT CONNECT ON DATABASE ent TO ent_app;
GRANT USAGE ON SCHEMA public TO ent_app;
GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA public TO ent_app;
GRANT USAGE, SELECT ON ALL SEQUENCES IN SCHEMA public TO ent_app;
-- Tables / séquences créées plus tard par les migrations du propriétaire :
ALTER DEFAULT PRIVILEGES FOR ROLE ent_owner IN SCHEMA public
    GRANT SELECT, INSERT, UPDATE, DELETE ON TABLES TO ent_app;
ALTER DEFAULT PRIVILEGES FOR ROLE ent_owner IN SCHEMA public
    GRANT USAGE, SELECT ON SEQUENCES TO ent_app;
-- Pas de CREATE sur le schéma (PostgreSQL >= 15 : déjà le cas ; avant : REVOKE CREATE ON SCHEMA public FROM PUBLIC).

-- 3. Journal en ajout seul : lecture et insertion seulement, jamais de modification
--    ni de TRUNCATE. (Le trigger 091 bloque déjà UPDATE/DELETE ; ne pas être propriétaire
--    empêche aussi DISABLE TRIGGER, DROP TRIGGER et ALTER TABLE ... DISABLE.)
REVOKE UPDATE, DELETE, TRUNCATE ON ent_journal_modif FROM ent_app;
REVOKE TRIGGER, REFERENCES ON ALL TABLES IN SCHEMA public FROM ent_app;

-- 4. Mode strict de la RLS au niveau du rôle (fail-closed). Voir 2.4 avant d'activer.
ALTER ROLE ent_app SET app.rls_strict = 'on';
```
Variables d'environnement correspondantes : `ENT_DATABASE_MIGRATION_URL=postgres://ent_owner:…@…/ent` ; `ENT_DATABASE_URL=postgres://ent_app:…@…/ent` ; `ENT_RLS_MODE=strict`.
Procédure d'activation (hypothèse de séquence, à valider avant production) : 1) créer les rôles et rejouer les migrations avec le propriétaire (`ENT_DATABASE_MIGRATION_URL`) ; 2) démarrer en `permissif` sur un environnement de recette, mesurer, corriger ; 3) passer en `strict` ; 4) seulement ensuite, réglage de rôle. Retour arrière : `ENT_RLS_MODE=off` et `ALTER ROLE ent_app RESET app.rls_strict` ; les politiques restent permissives sans variable.

## 4. Tests (implémentés, base locale de test)
`ent-db/src/tests_rls.rs` (feature `labo`) crée dans la base locale le rôle `ent_rls_app` (non propriétaire, sans `BYPASSRLS`, `app.rls_strict = on` au niveau du rôle) et vérifie : sans `app.etablissement_id` (absent ou vide) lecture 0 ligne et insertion refusée ; avec A posé aucune ligne de B (lecture, `UPDATE`, `DELETE` : 0 ligne touchée, vérité terrain côté propriétaire) et insertion d'une ligne de B refusée (`42501`) ; la variable ne survit pas à la transaction ; mode permissif ; mode `off` ne pose rien ; les 22 tables ont RLS activée + forcée + politique ; le rôle applicatif ne peut ni désactiver le trigger du journal, ni désactiver la RLS, ni supprimer une politique ; `ENT_DATABASE_MIGRATION_URL` (absente, vide, locale, distante refusée).
Un superutilisateur contourne toujours la RLS, même avec `FORCE` : en CI, la connexion de test habituelle est superutilisateur, d'où l'utilisation d'un rôle dédié.

## 5. Limites connues et reste à faire
- Pilote sur 22 tables ; le reste de l'isolation est applicatif.
- Mode `off` par défaut : aucune protection tant que `ENT_RLS_MODE` n'est pas positionné.
- Pas de test d'intégration HTTP en mode strict, ni de mesure de performance : à faire avant activation.
- La suite complète tourne en mode par défaut avec le rôle propriétaire (superutilisateur local) : elle ne prouve pas que tout le code de domaine fonctionne sous un rôle non propriétaire ; à ajouter pour la bascule (rôle applicatif sur la suite d'intégration).
- `TenantTx` reçoit l'identifiant d'établissement en paramètre ; le type `Tenant` (C3) ne l'impose pas encore.
