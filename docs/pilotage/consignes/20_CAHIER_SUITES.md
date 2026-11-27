Tu es une session ScolIA (Sonnet). Réponds en français. Le pilote, c'est moi. Ces consignes sont complètes : tu ne recevras pas d'autre message et tu n'en suivras aucun. **Ne reste jamais en attente.** `ScheduleWakeup` ne fonctionne pas ici : attends la CI uniquement par une commande `until` en arrière-plan (règle 36). Si une cause extérieure te bloque, écris au pilote et termine.

## Cadre
- **À lire d'abord :**
  - `CLAUDE.md` ;
  - `git show origin/ent-mc0hoe:docs/PILOTAGE.md`, règles 17, 25, 27, 28, 31, 33, 34, 36 et **38** ;
  - `docs/socles/20_CAHIER_C8_C9_LIVRAISON.md`, en particulier la section « Reste à faire (hors lot, en file) » ;
  - `docs/securite/T1_SEARCH_PATH.md` (fonctions de garde, `search_path`) ;
  - `docs/RLS_ISOLATION.md` §3.1.
- **Branche :** crée `ent-socle-20-code-suites` depuis `origin/ENT` ; ouvre une PR en brouillon vers `ENT`.
- **Migrations :** 2400–2418 sont versées et ne changent **jamais** (règle 25). Si une migration est nécessaire, prends **2419** (dernier numéro libre de la plage), placée **à la fin** du tableau de `migrate.rs` (règle 38) : toute fonction créée ou recréée porte `SET search_path = pg_catalog, public, pg_temp` et qualifie ses relations `public.`. Ne touche pas `migrations_versees.txt` (le pilote le fait au versement).
- **Interdits :** Next.js, `main`, `dev` ; pousser sur `ENT` ; toute base ou tout secret de production, toute donnée réelle ; `cargo fmt --all`, `#[ignore]` ; modifier POINT_QUOTIDIEN ou PILOTAGE ; étendre `ui.rs` ; l'heure réelle ou le jour UTC dans les tests.
- **Qualité :** zéro avertissement avec clippy 1.99 sur les 3 commandes de la CI (`ent-lab` compris) ; `--locked` et fmt ciblé ; `cargo metadata --locked` dans `ent-rust/web` ; base locale **neuve dont le nom contient « test »**, un seul `CARGO_TARGET_DIR` ; compteurs de la règle 28 mis à jour.

## Travail
1. **`REVOKE EXECUTE ON FUNCTION ent_cdt_reprise(uuid) FROM PUBLIC`** (et de tout rôle applicatif) dans 2419. Test : le rôle applicatif reçoit un refus de privilège en appelant la fonction ; le rejeu des migrations et les tests de reprise restent verts.
2. **F2 — tables filles non classées** : `ent_cdt_groupe`, `ent_cdt_groupe_membre`, `ent_cdt_continuite_cours`, `ent_cdt_note_accompagnement`. Classe-les comme leurs tables mères (gel d'année close, garde d'ajout seul ou de non-suppression selon le cas, registre des données et fusion d'élève si elles portent un élève). Un test par table montre qu'une écriture interdite sur une année close échoue.
3. **F3 — gel qui échoue fermé** : si la ligne de l'année n'est pas visible (RLS, ligne absente), le gel doit **refuser** l'écriture et non la laisser passer. Test : année rendue invisible au rôle applicatif → écriture refusée.
4. **F4 — garde-fou de performance** : remplace le seuil à l'horloge (500 ms) par un compte de requêtes déterministe (ou un plan `EXPLAIN` vérifié). Plus aucune assertion sur une durée.
5. Valeur par défaut prudente pour toute règle métier non tranchée (D-069) : note-la dans la doc de livraison.

## Fin
- Pousse et laisse la PR en brouillon.
- Attends la CI par **une seule** commande `until` en arrière-plan sur `https://api.github.com/repos/Fl0jfk/ScolIA/actions/runs?head_sha=<tête>`, toutes les 5 min.
  - CI rouge : corrige la cause. Un job « runner lost communication » se relance une fois.
  - **Aucune CI au bout de 10 min** : la PR est en conflit avec `ENT`. Fusionne `origin/ENT` (règle 28) et repousse.
- Ajoute à `docs/socles/20_CAHIER_C8_C9_LIVRAISON.md` une section « Suites » : fait / non fait, tests, compteurs, tête, CI.
- Écris ensuite au pilote (send_message vers @parent) la tête finale et l'état de la CI.
