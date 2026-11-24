Tu es une session ScolIA (Sonnet). Réponds en français. Le pilote, c'est moi. Ces consignes sont complètes : tu ne recevras pas d'autre message et tu n'en suivras aucun. **Ne reste jamais en attente.** `ScheduleWakeup` ne fonctionne pas ici : attends la CI uniquement par une commande `until` en arrière-plan (règle 36). Si une cause extérieure te bloque, écris au pilote et termine.

## Cadre
- **À lire d'abord :**
  - `CLAUDE.md` ;
  - `git show origin/ent-mc0hoe:docs/PILOTAGE.md`, règles 17, 25, 27, 28, 31, 33, 34, 36 et **38** ;
  - `docs/socles/21_EVALUATIONS_V3E_V5_LIVRAISON.md` **en entier**, en particulier « Correctifs du pilote après contre-revue » et la ligne « En file (suites V5, passe 5) » : c'est ta liste de travail ; `ent-rust/crates/ent-db/src/registre_donnees.rs` (registre d'export) ;
  - `docs/securite/T1_SEARCH_PATH.md` (fonctions de garde, `search_path`) ;
  - `docs/RLS_ISOLATION.md` §3.1.
- **Branche :** crée `ent-socle-21-code-v5-suites-5` depuis `origin/ENT` ; ouvre une PR en brouillon vers `ENT`.
- **Migrations :** 470–478 et 700 sont versées et ne changent **jamais** (règle 25). Si une migration est nécessaire, prends **701** (plage 701–719), placée **à la fin** du tableau de `migrate.rs` (règle 38) : toute fonction créée ou recréée porte `SET search_path = pg_catalog, public, pg_temp` et qualifie ses relations `public.`. Ne touche pas `migrations_versees.txt` (le pilote le fait au versement).
- **Interdits :** Next.js, `main`, `dev` ; pousser sur `ENT` ; toute base ou tout secret de production, toute donnée réelle ; `cargo fmt --all`, `#[ignore]` ; modifier POINT_QUOTIDIEN ou PILOTAGE ; étendre `ui.rs` ; l'heure réelle ou le jour UTC dans les tests.
- **Qualité :** zéro avertissement avec clippy 1.99 sur les 3 commandes de la CI (`ent-lab` compris) ; `--locked` et fmt ciblé ; `cargo metadata --locked` dans `ent-rust/web` ; base locale **neuve dont le nom contient « test »**, un seul `CARGO_TARGET_DIR` ; compteurs de la règle 28 mis à jour.

## Travail (liste « En file (suites V5, passe 5) »)
Principe (déjà appliqué à `eva_appreciation` en passe 4) : l'export des données d'un élève ne rend **que ce que la famille
peut déjà voir dans l'ENT** ; une table à état ne s'exporte jamais sans condition.
1. **M1** : test de registre : `hors_export` seul n'est pas une décision pour une table à état (une table à état doit avoir
   `export_si(...)` ou une justification explicite) ; échec franc sinon.
2. **M2** : extrait de bout en bout : une ligne **marquée** par table exportée ; le test échoue si une table n'est pas
   filtrable (ex. `eva_moyenne_externe`).
3. **M3** : `eva_moyenne_statut` : condition de publication.
4. **M4** : `eva_rattrapage_historique` : retiré de l'export (cohérent avec le retrait de l'historique des appréciations) ou justifié.
5. **M5** : `eva_rattrapage*` : condition d'état.
6. **F3** : déclarations externes et dispenses : valeur par défaut prudente (D-069 : on n'exporte que ce qui est communiqué à la famille), notée.
7. **F4** : `ent_note_appreciation` (table de reprise) exportée sans condition : la condition ou le retrait, jusqu'à G3. Test.
8. Réserve DPO (historique d'une appréciation publiée puis modifiée) : documente la décision prudente retenue.

## Fin
- Pousse et laisse la PR en brouillon.
- Attends la CI par **une seule** commande `until` en arrière-plan sur `https://api.github.com/repos/Fl0jfk/ScolIA/actions/runs?head_sha=<tête>`, toutes les 5 min.
  - CI rouge : corrige la cause. Un job « runner lost communication » se relance une fois.
  - **Aucune CI au bout de 10 min** : la PR est en conflit avec `ENT`. Fusionne `origin/ENT` (règle 28) et repousse.
- Ajoute à `docs/socles/21_EVALUATIONS_V3E_V5_LIVRAISON.md` une section « Suites, passe 5 » : fait / non fait, tests, compteurs, tête, CI.
- Écris ensuite au pilote (send_message vers @parent) la tête finale et l'état de la CI.
