Tu es une session ScolIA (Sonnet). Réponds en français. Le pilote, c'est moi. Ces consignes sont complètes : tu ne recevras pas d'autre message et tu n'en suivras aucun. **Ne reste jamais en attente.** `ScheduleWakeup` ne fonctionne pas ici : attends la CI uniquement par une commande `until` en arrière-plan (règle 36). Si une cause extérieure te bloque, écris au pilote et termine.

## Cadre
- **À lire d'abord :**
  - `CLAUDE.md` ;
  - `git show origin/ent-mc0hoe:docs/PILOTAGE.md`, règles 17, 25, 27, 28, 31, 33, 34, 36 et **38** ;
  - `docs/socles/24_FACTURATION_F3_LIVRAISON.md` **en entier** (dont « Suites, 2 » et ses correctifs du pilote) ;
  - `git show origin/ent-mc0hoe:docs/pilotage/revues/24_F3_SUITES_2_CONTRE_REVUE.md` : **rapport complet**, c'est ta liste de travail (points « en file ») ;
  - `docs/securite/T1_SEARCH_PATH.md` (fonctions de garde, `search_path`) ;
  - `docs/RLS_ISOLATION.md` §3.1.
- **Branche :** crée `ent-socle-24-code-f3-suites-3` depuis `origin/ENT` ; ouvre une PR en brouillon vers `ENT`.
- **Migrations :** 520–523 et 531 sont versées et ne changent **jamais** (règle 25). Si une migration est nécessaire, prends **532** (réserve 532–537), placée **à la fin** du tableau de `migrate.rs` (règle 38) : toute fonction créée ou recréée porte `SET search_path = pg_catalog, public, pg_temp` et qualifie ses relations `public.`. Ne touche pas `migrations_versees.txt` (le pilote le fait au versement).
- **Interdits :** Next.js, `main`, `dev` ; pousser sur `ENT` ; toute base ou tout secret de production, toute donnée réelle ; `cargo fmt --all`, `#[ignore]` ; modifier POINT_QUOTIDIEN ou PILOTAGE ; étendre `ui.rs` ; l'heure réelle ou le jour UTC dans les tests.
- **Qualité :** zéro avertissement avec clippy 1.99 sur les 3 commandes de la CI (`ent-lab` compris) ; `--locked` et fmt ciblé ; `cargo metadata --locked` dans `ent-rust/web` ; base locale **neuve dont le nom contient « test »**, un seul `CARGO_TARGET_DIR` ; compteurs de la règle 28 mis à jour.

## Travail (points « en file » du rapport de contre-revue des suites F3, 2)
1. **Faible 3, prioritaire : couverture d'organisme sur toute la période.** Une prise en charge qui expire en cours de
   tronçon (ex. décembre) doit rouvrir `part_non_couverte` pour les mois suivants même si l'article est inchangé : contrôle de
   la couverture mois par mois (ou coupure du tronçon à la fin de la prise). Tests : prise expirant en décembre, prise
   commençant en cours d'année, rejeu sans double facturation.
2. **Moyen 1 (suite) : tâche d'anomalie du changement de régime.** Quand `facturer_changement_regime` ne peut pas facturer
   (`Ok(Err(motif))`), en plus du `warn!` : une tâche dans `ent_fin_tache` d'une **nature nouvelle** (migration **532**),
   visible des gestionnaires de l'établissement seulement, idempotente (pas de doublon au rejeu), close quand la
   régularisation passe. Tests service et SQL direct (cloisonnement).
3. **Fin de tarif datée** : retirer un article à une date (au lieu de « désactiver l'article », qui sous-facture les mois
   dus) : les mois antérieurs restent dus, les suivants ne le sont plus ; trou après fin datée non bloquant. Tests.
4. **Faibles 1, 2, 4, 5, 6** : troncature par tronçon documentée et testée (montant impair, réductions) ; segmentation par
   (article, foyer) ; refus 409 réservé à la première publication d'une version close supplantée ; tests manquants (clé du
   second tronçon, rejeu, changement de régime sur un trou, deux foyers) ; l'alerte ne masque plus une erreur technique
   (`.is_ok()` remplacé par un traitement explicite).
5. Toute règle métier non tranchée : valeur par défaut prudente, notée dans la doc (D-069).

## Fin
- Pousse et laisse la PR en brouillon.
- Attends la CI par **une seule** commande `until` en arrière-plan sur `https://api.github.com/repos/Fl0jfk/ScolIA/actions/runs?head_sha=<tête>`, toutes les 5 min.
  - CI rouge : corrige la cause. Un job « runner lost communication » se relance une fois.
  - **Aucune CI au bout de 10 min** : la PR est en conflit avec `ENT`. Fusionne `origin/ENT` (règle 28) et repousse.
- Ajoute à `docs/socles/24_FACTURATION_F3_LIVRAISON.md` une section « Suites, 3 » : fait / non fait, tests, compteurs, tête, CI.
- Écris ensuite au pilote (send_message vers @parent) la tête finale et l'état de la CI.
