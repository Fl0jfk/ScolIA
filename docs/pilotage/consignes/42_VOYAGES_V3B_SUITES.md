Tu es une session ScolIA (Sonnet). Réponds en français. Le pilote, c'est moi. Ces consignes sont complètes : tu ne recevras pas d'autre message et tu n'en suivras aucun. **Ne reste jamais en attente.** `ScheduleWakeup` ne fonctionne pas ici : attends la CI uniquement par une commande `until` en arrière-plan (règle 36). Si une cause extérieure te bloque, écris au pilote et termine.

## Cadre
- **À lire d'abord :**
  - `CLAUDE.md` ;
  - `git show origin/ent-mc0hoe:docs/PILOTAGE.md`, règles 17, 25, 27, 28, 31, 33, 34, 36 et **38** ;
  - `docs/socles/42_VOYAGES_V3_LIVRAISON.md` **en entier** (dont « Correctifs du pilote après contre-revue des suites ») ;
  - `git show origin/ent-mc0hoe:docs/pilotage/revues/42_VOYAGES_V3_SUITES_CONTRE_REVUE.md` : **rapport complet**, c'est ta liste de travail ;
  - `docs/securite/T1_SEARCH_PATH.md` (fonctions de garde, `search_path`) ;
  - `docs/RLS_ISOLATION.md` §3.1.
- **Branche :** crée `ent-socle-42-code-v3b-suites` depuis `origin/ENT` ; ouvre une PR en brouillon vers `ENT`.
- **Migrations :** 640–655 sont versées et ne changent **jamais** (règle 25). Si une migration est nécessaire, prends **656** (plage 656–659), placée **à la fin** du tableau de `migrate.rs` (règle 38) : toute fonction créée ou recréée porte `SET search_path = pg_catalog, public, pg_temp` et qualifie ses relations `public.`. Ne touche pas `migrations_versees.txt` (le pilote le fait au versement).
- **Interdits :** Next.js, `main`, `dev` ; pousser sur `ENT` ; toute base ou tout secret de production, toute donnée réelle ; `cargo fmt --all`, `#[ignore]` ; modifier POINT_QUOTIDIEN ou PILOTAGE ; étendre `ui.rs` ; l'heure réelle ou le jour UTC dans les tests.
- **Qualité :** zéro avertissement avec clippy 1.99 sur les 3 commandes de la CI (`ent-lab` compris) ; `--locked` et fmt ciblé ; `cargo metadata --locked` dans `ent-rust/web` ; base locale **neuve dont le nom contient « test »**, un seul `CARGO_TARGET_DIR` ; compteurs de la règle 28 mis à jour.

## Travail (points « en file » du rapport de contre-revue des suites)
1. **Moyen 2 (prioritaire)** : sortie « rentrée » : un participant resté « parti » ne perd son retrait d'internat qu'à partir
   de l'instant de passage en « rentree » (colonne `rentree_le` immuable posée à la transition, comme `annulee_le`) ; les nuits
   passées dehors restent retirées. Test avec des nuits passées et à venir.
2. **Moyen 3** : contournement **tracé** de la règle « sortie entière » (acomptes fournisseur, annulation collective) quand la
   seule personne habilitée est liée à un participant : validation par une seconde personne habilitée et libre (pas
   d'exception silencieuse) ; tests.
3. **Moyen 4** : tests SQL directs (23514) des déclencheurs de 655 non couverts : frais réel, décision d'avoir (INSERT et
   UPDATE, y compris l'exclusion de la décision système), paiement d'acompte (UPDATE), `ent_sortie_issue_coherente`.
4. **Faibles 1, 2, 4 à 7** du rapport (branche directe de l'attendu, seuil M-2 et demandes expirées, traces de refus,
   rattrapage `annulee_le` et RLS, `annule_par_user_id` de l'acompte, préfixe `public.`).
5. Toute règle métier non tranchée : valeur par défaut prudente, notée dans la doc (D-069).

## Fin
- Pousse et laisse la PR en brouillon.
- Attends la CI par **une seule** commande `until` en arrière-plan sur `https://api.github.com/repos/Fl0jfk/ScolIA/actions/runs?head_sha=<tête>`, toutes les 5 min.
  - CI rouge : corrige la cause. Un job « runner lost communication » se relance une fois.
  - **Aucune CI au bout de 10 min** : la PR est en conflit avec `ENT`. Fusionne `origin/ENT` (règle 28) et repousse.
- Ajoute à `docs/socles/42_VOYAGES_V3_LIVRAISON.md` une section « Suites V3b » : fait / non fait, tests, compteurs, tête, CI.
- Écris ensuite au pilote (send_message vers @parent) la tête finale et l'état de la CI.
