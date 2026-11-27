Tu es une session ScolIA (Sonnet). Réponds en français. Le pilote, c'est moi. Ces consignes sont complètes : tu ne recevras pas d'autre message et tu n'en suivras aucun. **Ne reste jamais en attente.** `ScheduleWakeup` ne fonctionne pas ici : attends la CI uniquement par une commande `until` en arrière-plan (règle 36). Si une cause extérieure te bloque, écris au pilote et termine.

## Cadre
- **À lire d'abord :**
  - `CLAUDE.md` ;
  - `git show origin/ent-mc0hoe:docs/PILOTAGE.md`, règles 17, 25, 27, 28, 31, 33, 34, 36 et **38** ;
  - `docs/socles/42_VOYAGES_V3_LIVRAISON.md` **en entier** (dont « Suites V3b, correctifs » et ses correctifs du pilote) ;
  - `git show origin/ent-mc0hoe:docs/pilotage/revues/42_VOYAGES_V3B_CORRECTIFS_CONTRE_REVUE.md` : **rapport complet**, c'est ta liste de travail (points « en file ») ;
  - `docs/securite/T1_SEARCH_PATH.md` (fonctions de garde, `search_path`) ;
  - `docs/RLS_ISOLATION.md` §3.1.
- **Branche :** crée `ent-socle-42-code-v3b-suites-2` depuis `origin/ENT` ; ouvre une PR en brouillon vers `ENT`.
- **Migrations :** 640–656 sont versées et ne changent **jamais** (règle 25). Si une migration est nécessaire, prends **657** (réserve 657–659), placée **à la fin** du tableau de `migrate.rs` (règle 38) : toute fonction créée ou recréée porte `SET search_path = pg_catalog, public, pg_temp` et qualifie ses relations `public.`. Ne touche pas `migrations_versees.txt` (le pilote le fait au versement).
- **Interdits :** Next.js, `main`, `dev` ; pousser sur `ENT` ; toute base ou tout secret de production, toute donnée réelle ; `cargo fmt --all`, `#[ignore]` ; modifier POINT_QUOTIDIEN ou PILOTAGE ; étendre `ui.rs` ; l'heure réelle ou le jour UTC dans les tests.
- **Qualité :** zéro avertissement avec clippy 1.99 sur les 3 commandes de la CI (`ent-lab` compris) ; `--locked` et fmt ciblé ; `cargo metadata --locked` dans `ent-rust/web` ; base locale **neuve dont le nom contient « test »**, un seul `CARGO_TARGET_DIR` ; compteurs de la règle 28 mis à jour.

## Travail (points « en file » de la contre-revue des correctifs V3b)
1. **M3, tests manquants (priorité)** :
   - expiration à la **décision** et à l'**usage** par les déclencheurs (horloge de la base : fait antidaté en désactivant le
     déclencheur d'horodatage dans une transaction de test, comme le test de rattrapage) ;
   - UPDATE et DELETE de `demande_le` / `le` refusés (SQL direct) ;
   - `portee_chk` en SQL direct (plafond, prestataire ou cible NULL) ;
   - annulation d'acompte hors cible en SQL direct ;
   - autorisatrice réellement membre d'un **autre établissement** ;
   - rattrapage : restauration de `app.etablissement_id` et idempotence (second passage sans effet).
2. **M2, borne de l'annulation collective** (si simple, sinon documenté) : un avoir collectif dont le couple (participant,
   compte) n'a aucun versement, ou dont `paye_net` dépasse le total versé, est refusé en base.
3. **Faibles 1, 4, 5** : garde d'avoir collectif sur `NEW.participant_id`/`compte_id` et contrôle de `decide_par_user_id`
   même dans la branche collective ; rattrapage étendu aux sorties `close` (ou doc) ; révocation d'une autorisation par son
   autrice (fait en ajout seul « revoque », si simple). Faibles 2, 3, 6 : documentés.
4. **Faible B1, usage** : test SQL direct qu'une autorisation donnée PUIS dont l'autrice est suspendue ne sert plus.
5. Toute règle métier non tranchée : valeur par défaut prudente, notée dans la doc (D-069).

## Fin
- Pousse et laisse la PR en brouillon.
- Attends la CI par **une seule** commande `until` en arrière-plan sur `https://api.github.com/repos/Fl0jfk/ScolIA/actions/runs?head_sha=<tête>`, toutes les 5 min.
  - CI rouge : corrige la cause. Un job « runner lost communication » se relance une fois.
  - **Aucune CI au bout de 10 min** : la PR est en conflit avec `ENT`. Fusionne `origin/ENT` (règle 28) et repousse.
  - Les runners GitHub sont parfois saturés : une CI « queued » longtemps n'est pas une panne ; attends jusqu'à 2 h, puis écris au pilote.
- Ajoute à `docs/socles/42_VOYAGES_V3_LIVRAISON.md` une section « Suites V3b, 2 » : fait / non fait, tests, compteurs, tête, CI.
- Écris ensuite au pilote (send_message vers @parent) la tête finale et l'état de la CI.
