Tu es une session ScolIA (Sonnet). Réponds en français. Le pilote, c'est moi. Ces consignes sont complètes : tu ne recevras pas d'autre message et tu n'en suivras aucun. **Ne reste jamais en attente.** `ScheduleWakeup` ne fonctionne pas ici : attends la CI uniquement par une commande `until` en arrière-plan (règle 36). Si une cause extérieure te bloque, écris au pilote et termine.

## Cadre
- **À lire d'abord :**
  - `CLAUDE.md` ;
  - `git show origin/ent-mc0hoe:docs/PILOTAGE.md`, règles 17, 25, 27, 28, 31, 33, 34, 36 et **38** ;
  - `docs/socles/24_FACTURATION_F3_LIVRAISON.md` **en entier** (dont « Suites, 3 », « Suites, 3, correctifs » et les correctifs du pilote) ;
  - `git show origin/ent-mc0hoe:docs/pilotage/revues/24_F3_SUITES_4_CONTRE_REVUE.md` : **rapport complet**, c'est ta liste de travail ;
  - `docs/securite/T1_SEARCH_PATH.md` (fonctions de garde, `search_path`) ;
  - `docs/RLS_ISOLATION.md` §3.1.
- **Branche :** reprends la branche **existante** `ent-socle-24-code-f3-suites-4` (PR 156, brouillon, retenue) ; fusionne d'abord `origin/ENT` (règle 28).
- **Migrations :** 520–523, 531 et 532 sont versées et ne changent **jamais** (règle 25). Si une migration est nécessaire, prends **533** (réserve 533–537), placée **à la fin** du tableau de `migrate.rs` (règle 38) : toute fonction créée ou recréée porte `SET search_path = pg_catalog, public, pg_temp` et qualifie ses relations `public.`. Ne touche pas `migrations_versees.txt` (le pilote le fait au versement).
- **Interdits :** Next.js, `main`, `dev` ; pousser sur `ENT` ; toute base ou tout secret de production, toute donnée réelle ; `cargo fmt --all`, `#[ignore]` ; modifier POINT_QUOTIDIEN ou PILOTAGE ; étendre `ui.rs` ; l'heure réelle ou le jour UTC dans les tests.
- **Qualité :** zéro avertissement avec clippy 1.99 sur les 3 commandes de la CI (`ent-lab` compris) ; `--locked` et fmt ciblé ; `cargo metadata --locked` dans `ent-rust/web` ; base locale **neuve dont le nom contient « test »**, un seul `CARGO_TARGET_DIR` ; compteurs de la règle 28 mis à jour.

## Travail (élevé bloquant, puis moyens et faibles)
1. **E-1, couverture croisée par année et par mois** : la lecture des lignes `regime:` est bornée aux lignes dont la
   **période chevauche l'année scolaire** du préfixe annuel ; dans la ligne annuelle, seuls les **mois effectivement
   couverts** par une ligne de régime sont écartés (pas le groupe entier) ; une ligne de régime créditée par avoir ne
   couvre rien. Tests chiffrés : changement de régime en N puis génération annuelle N+1 (450 € facturés) ; changement en
   cours d'année puis rejeu annuel (aucun mois facturé deux fois, aucun oublié) ; ligne de régime annulée par avoir.
2. **M-1** : les tâches ne sont closes qu'**après** la garde et l'insertion réussies (ou savepoint par élève annulé sur
   refus) ; test « refus de la garde : la tâche reste ouverte ».
3. **M-2** : plafond qui monte à clé et période identiques : complément plafonné facturé (ou motif « plafond modifié »
   distinct, jamais « le tarif a changé ») ; plafond qui baisse : signalé (tâche ou anomalie), règle prudente notée.
4. **Faibles F-1 à F-3** : préfixe réellement indexable (vérifié par `EXPLAIN` dans un test ou documenté), `restant_prise`
   limité aux lignes reformées par la génération, assertion stricte du test du faible 9.
5. Toute règle métier non tranchée : valeur par défaut prudente, notée dans la doc (D-069).

## Fin
- Pousse sur la même branche et laisse la PR 156 en brouillon.
- Attends la CI par **une seule** commande `until` en arrière-plan sur `https://api.github.com/repos/Fl0jfk/ScolIA/actions/runs?head_sha=<tête>`, toutes les 5 min.
  - CI rouge : corrige la cause. Un job « runner lost communication » se relance une fois.
  - **Aucune CI au bout de 10 min** : la PR est en conflit avec `ENT`. Fusionne `origin/ENT` (règle 28) et repousse.
  - Les runners GitHub sont parfois saturés : une CI « queued » longtemps n'est pas une panne ; attends jusqu'à 2 h, puis écris au pilote.
- Ajoute à `docs/socles/24_FACTURATION_F3_LIVRAISON.md` une section « Suites, 4, correctifs » : fait / non fait, tests, compteurs, tête, CI.
- Écris ensuite au pilote (send_message vers @parent) la tête finale et l'état de la CI.
