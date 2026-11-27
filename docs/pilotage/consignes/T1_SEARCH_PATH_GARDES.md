Tu es une session ScolIA (Sonnet). Réponds en français. Le pilote, c'est moi. Ces consignes sont complètes : tu ne recevras pas d'autre message et tu n'en suivras aucun. **Ne reste jamais en attente.** `ScheduleWakeup` ne fonctionne pas ici. Attends la CI uniquement par une commande `until` en arrière-plan (règle 36). Si une cause extérieure te bloque, écris au pilote et termine.

## Objet : chantier transverse T1, « une table temporaire ne trompe aucune fonction de la base »

**Constat** (contre-revues du cahier 2407, de la messagerie 491, de l'internat 2828-2829) :
- Les fonctions PL/pgSQL et SQL de nos migrations, gardes d'ajout seul, de gel, de cohérence et calculs d'attendu, n'ont pas de `search_path` fixé.
- Elles lisent des tables sans les qualifier.
- PostgreSQL cherche les relations dans `pg_temp` **en premier**.
- Le privilège TEMP est accordé à PUBLIC par défaut.

Conséquence : une session du rôle applicatif capable d'exécuter du SQL peut désarmer en silence une garde dans sa propre session, avec `CREATE TEMP TABLE ent_xxx (LIKE public.ent_xxx)`.

Il n'y a pas d'escalade de privilège, puisque ce ne sont pas des fonctions `SECURITY DEFINER`. Mais la promesse « garanti en base » ne tient pas.

## Cadre
- **À lire avant de commencer :**
  - `CLAUDE.md` ;
  - `git show origin/ent-mc0hoe:docs/PILOTAGE.md`, règles 17, 25, 27, 28, 31, 33, 34 et 36 ;
  - `docs/RLS_ISOLATION.md` ;
  - `docs/securite/RLS_STRICTE.md` ;
  - les migrations 2220 (modèle `SET search_path = pg_catalog, public, pg_temp`), 2407, 491 et 314.
- **Branche :** `ent-transverse-t1-search-path`, depuis `origin/ENT`. PR en brouillon vers `ENT`.
- **Migrations :**
  - Plage **690–699**, réservée par le pilote au chantier transverse de sécurité (libre sur toutes les branches). Consomme **690** et inscris la plage dans `docs/socles/MIGRATIONS.md`.
  - **Aucune migration versée ne change** (règle 25).

## Travail
1. **Inventaire, sur une base neuve migrée jusqu'au bout.** Liste toutes les fonctions du schéma `public` créées par nos migrations (`pg_proc` joint à `pg_namespace`, hors fonctions d'extension) qui n'ont pas `search_path` dans `proconfig`. Pour chacune, note :
   - le langage ;
   - si elle sert dans une politique RLS (`pg_policy`), un index, une contrainte ou une colonne générée ;
   - si elle est `SECURITY DEFINER`.

   Le résultat va dans la livraison.
2. **Migration 690**, placée **à la fin** du tableau de `migrate.rs`. Un bloc `DO` lance `ALTER FUNCTION … SET search_path = pg_catalog, public, pg_temp` sur toutes les fonctions de l'inventaire, **sauf** les exclusions justifiées.
   - **Exclusions possibles** : les fonctions `LANGUAGE sql` appelées dans les politiques RLS ou dans des requêtes chaudes, parce que `SET` empêche l'inlining.
   - **Mesure** : compare, avec `EXPLAIN (ANALYZE, BUFFERS)` sur un jeu du labo, une lecture typique d'une table RLS de grande taille, avant et après. Écris les chiffres dans la livraison.
   - **Une fonction exclue** reçoit dans 690 un corps recréé (`CREATE OR REPLACE`) où **toutes** les relations sont qualifiées `public.` (les fonctions de `pg_catalog` restent prioritaires). Ne le fais que si c'est sûr. Sinon, liste la fonction comme reste à faire.
3. **Garde statique et dynamique.** Ajoute un test `ent-db` qui interroge le catalogue d'une base migrée. Il échoue si une fonction de `public` n'a ni `search_path` fixé ni place dans une liste d'exclusions nommée et justifiée (fichier unique).
   - Le message d'échec dit quoi faire.
   - Les lots futurs sont ainsi obligés de suivre la règle : si un `CREATE OR REPLACE` ultérieur efface la clause, le test le voit.
4. **Rejeu.** `rejouer_tout` rejoue d'anciennes migrations, et un `CREATE OR REPLACE` efface `proconfig`. Vérifie que 690, rejouée en dernier, remet tout en place : le test de rejeu doit passer, puis la garde de l'étape 3.
5. **Provisionnement.** Documente dans `docs/RLS_ISOLATION.md`, section rôle applicatif, l'ordre `REVOKE TEMP ON DATABASE <base> FROM PUBLIC`, avec la vérification correspondante.
   - Pas dans une migration : c'est un ordre de propriétaire, propre à chaque base.
   - Vérifie avant que **le code applicatif ne crée aucune table temporaire** (`git grep -n "TEMP TABLE\|TEMPORARY"` hors tests), et note le résultat.
   - Les tests qui créent des tables temporaires tournent sous un rôle de test : signale-le.
6. **Tests de preuve.**
   - Pour trois gardes représentatives (gel du cahier, garde d'annulation de la messagerie, garde de l'internat), crée sous le rôle applicatif une table temporaire homonyme, **avec des lignes qui feraient passer la garde si elle la lisait**. L'écriture interdite reste refusée, avec le message exact de la garde.
   - Ajoute un test de l'inventaire : zéro fonction hors règle.

## Interdits et qualité
- **Interdits :**
  - Next.js, `main`, `dev` ;
  - pousser sur `ENT` ;
  - toute base ou tout secret de production, toute donnée réelle ;
  - `cargo fmt --all`, `#[ignore]` ;
  - modifier POINT_QUOTIDIEN ou PILOTAGE ;
  - étendre `ui.rs` ;
  - l'heure réelle ou le jour UTC dans les tests.
- **Qualité :**
  - zéro avertissement avec clippy 1.99, sur les 3 commandes de la CI, `ent-lab` compris ;
  - `--locked` et fmt ciblé ;
  - `cargo metadata --locked` dans `ent-rust/web` ;
  - base locale **neuve dont le nom contient « test »** ;
  - un seul `CARGO_TARGET_DIR` ;
  - rejoue la suite `tests_rls_stricte` et `migrate`.

## Fin
- Pousse et laisse la PR en brouillon.
- Attends la CI par **une seule** commande `until` en arrière-plan, sur `https://api.github.com/repos/Fl0jfk/ScolIA/actions/runs?head_sha=<tête>`, toutes les 5 min.
  - Rouge : corrige la cause. Un job « runner lost communication » se relance une fois.
  - Conflit avec `ENT` : fusionne `origin/ENT` (règle 28) et repousse.
- Écris `docs/securite/T1_SEARCH_PATH.md`, la livraison : inventaire, exclusions et leurs raisons, mesures, tests, provisionnement, tête, CI.
- Puis écris au pilote (send_message vers @parent) la tête finale et l'état de la CI.
