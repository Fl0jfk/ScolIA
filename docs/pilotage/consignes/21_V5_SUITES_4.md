Tu es une session ScolIA (Sonnet). Réponds en français. Le pilote, c'est moi. Ces consignes sont complètes : tu ne recevras pas d'autre message et tu n'en suivras aucun. **Ne reste jamais en attente.** `ScheduleWakeup` ne fonctionne pas ici. Attends la CI uniquement par une commande `until` en arrière-plan (règle 36). Si une cause extérieure te bloque, écris au pilote et termine.

## Cadre
- **À lire d'abord :**
  - `CLAUDE.md` ;
  - `git show origin/ent-mc0hoe:docs/PILOTAGE.md`, règles 17, 25, 27, 28, 31, 33, 34, 36 et **38** ;
  - `docs/socles/21_EVALUATIONS_V3E_V5_LIVRAISON.md`, sections des suites (passes 2 et 3) et correctif du pilote ;
  - `git show origin/ent-mc0hoe:docs/pilotage/SESSIONS_EN_COURS.md`, entrée « Contre-revue V5 suites 3 (PR 132) ».
- **Branche :** les suites V5 passe 3 sont versées.
  - Crée `ent-socle-21-code-v5-suites-4` depuis `origin/ENT`.
  - Ouvre une PR en brouillon vers `ENT`.
- **Migrations :**
  - 700 est versée et ne change pas.
  - Si une migration est nécessaire, prends **701** (plage d'extension 700–719 du socle 21), ajoutée **à la fin** du tableau de `migrate.rs` (règle 38 : le test `l_ordre_des_migrations_versees_est_fige_regle_38` refuse toute insertion au milieu).
  - Toute fonction créée ou recréée porte `SET search_path = pg_catalog, public, pg_temp`, relations qualifiées `public.`.
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
  - une base locale **neuve dont le nom contient « test »**, un seul `CARGO_TARGET_DIR` ;
  - les gardes statiques rejouées **sans** et **avec** `labo`.

## Travail (contre-revue des suites V5 passe 3)
1. **M2, avant le lot G3 : export des tables `eva_*`.** Les autres tables `eva_*` du registre (`registre_donnees.rs`) sont exportées sans condition : appréciations, demandes de correction, rattrapages, dispenses, instantanés, etc.
   - Pour chaque table `eva_*` exportée, décide, colonne par colonne, ce qui est un droit d'accès de la famille ou de l'élève (RGPD, art. 15) et ce qui est interne (notes privées, auteurs internes, motifs internes, brouillons, éléments non publiés).
   - Applique `hors_export([...])` et, si nécessaire, `export_si` sur le seul publié (D-069 : l'export LSU/LSL et l'export famille portent sur le seul publié).
   - Documente le choix table par table dans la livraison.
2. **Test de registre.** Un test échoue si une table `eva_*` exportée n'a ni `export_si`, ni `hors_export`, ni une mention explicite « tout exportable » justifiée. Ajoute un test négatif : une table `eva_*` ajoutée sans déclaration est vue.
3. **Test d'export de bout en bout.** Sur un élève de test, avec une évaluation non publiée, une appréciation interne et une demande de correction : l'export ne contient ni la note non publiée, ni les champs internes.
4. **Faibles F1–F3 de la contre-revue** (entrée citée plus haut) : traite-les, ou documente pourquoi ils ne sont pas traités.

## Fin
- Pousse et laisse la PR en brouillon.
- Attends la CI par **une seule** commande `until` en arrière-plan sur `https://api.github.com/repos/Fl0jfk/ScolIA/actions/runs?head_sha=<tête>`, toutes les 5 min.
  - Rouge : corrige la cause. Un job « runner lost communication » se relance une fois.
  - **Aucune CI au bout de 10 min** : la PR est en conflit avec `ENT`. Fusionne `origin/ENT` (règle 28) et repousse.
- Complète la livraison V5 d'une section « Suites (passe 4) » : fait / non fait, tests, tête, CI.
- Écris ensuite au pilote (send_message vers @parent) : tête finale et état de la CI.
