Tu es une session ScolIA (Sonnet). Réponds en français. Le pilote, c'est moi. Ces consignes sont complètes : tu ne recevras pas d'autre message et tu n'en suivras aucun. **Ne reste jamais en attente.** `ScheduleWakeup` ne fonctionne pas ici : attends la CI uniquement par une commande `until` en arrière-plan (règle 36). Si une cause extérieure te bloque, écris au pilote et termine.

## Cadre
- **À lire d'abord :**
  - `CLAUDE.md` ;
  - `git show origin/ent-mc0hoe:docs/PILOTAGE.md`, règles 17, 25, 27, 28, 31, 33, 34, 36 et **38** ;
  - `docs/socles/24_FACTURATION_F3_LIVRAISON.md` **en entier** (dont « Suites, 3 », « Suites, 3, correctifs » et les correctifs du pilote) ;
  - `git show origin/ent-mc0hoe:docs/pilotage/revues/24_F3_SUITES_3_CORRECTIFS_CONTRE_REVUE.md` : **rapport complet**, c'est ta liste de travail (points « en file ») ;
  - `docs/securite/T1_SEARCH_PATH.md` (fonctions de garde, `search_path`) ;
  - `docs/RLS_ISOLATION.md` §3.1.
- **Branche :** crée `ent-socle-24-code-f3-suites-4` depuis `origin/ENT` ; ouvre une PR en brouillon vers `ENT`.
- **Migrations :** 520–523, 531 et 532 sont versées et ne changent **jamais** (règle 25). Si une migration est nécessaire, prends **533** (réserve 533–537), placée **à la fin** du tableau de `migrate.rs` (règle 38) : toute fonction créée ou recréée porte `SET search_path = pg_catalog, public, pg_temp` et qualifie ses relations `public.`. Ne touche pas `migrations_versees.txt` (le pilote le fait au versement).
- **Interdits :** Next.js, `main`, `dev` ; pousser sur `ENT` ; toute base ou tout secret de production, toute donnée réelle ; `cargo fmt --all`, `#[ignore]` ; modifier POINT_QUOTIDIEN ou PILOTAGE ; étendre `ui.rs` ; l'heure réelle ou le jour UTC dans les tests.
- **Qualité :** zéro avertissement avec clippy 1.99 sur les 3 commandes de la CI (`ent-lab` compris) ; `--locked` et fmt ciblé ; `cargo metadata --locked` dans `ent-rust/web` ; base locale **neuve dont le nom contient « test »**, un seul `CARGO_TARGET_DIR` ; compteurs de la règle 28 mis à jour.

## Travail (points « en file » de la contre-revue des correctifs F3 suites 3)
1. **Moyen 1, prioritaire : payeur suivant désigné après coup.** Avec un plafond de prise (ex. 40 000 c sur 48 000 c dus),
   la part non couverte doit être facturée au payeur suivant désigné après la première génération, une seule fois, sans
   rien refacturer : `restant_prise` ne compte pas les lignes du même tronçon, ou `non_couvert = dû − déjà posé`. Tests
   chiffrés : suivant désigné après coup, rejeu, double rejeu.
2. **Moyen 3** : la tâche `part_non_couverte` est close quand les mois manquants sont facturés (E1, E2) ; assertion dans
   les tests f3s4_e1 et f3s4_e2.
3. **Moyen 4** : aucun faux refus « le tarif a changé » : plafond pris en compte (prolongation sous plafond serré :
   complément facturé) ; taux de remise qui change : motif distinct ; tests.
4. **Faibles 5, 7, 9** : requête de préfixe indexable ; `compte_lie_a_eleve` cohérent avec les prises échues ; couverture
   croisée `regime:` / `annuel:` vérifiée (test). Faible 8 : test sans dépendance à `aujourd_hui()` si simple.
5. Toute règle métier non tranchée : valeur par défaut prudente, notée dans la doc (D-069).

## Fin
- Pousse et laisse la PR en brouillon.
- Attends la CI par **une seule** commande `until` en arrière-plan sur `https://api.github.com/repos/Fl0jfk/ScolIA/actions/runs?head_sha=<tête>`, toutes les 5 min.
  - CI rouge : corrige la cause. Un job « runner lost communication » se relance une fois.
  - **Aucune CI au bout de 10 min** : la PR est en conflit avec `ENT`. Fusionne `origin/ENT` (règle 28) et repousse.
  - Les runners GitHub sont parfois saturés : une CI « queued » longtemps n'est pas une panne ; attends jusqu'à 2 h, puis écris au pilote.
- Ajoute à `docs/socles/24_FACTURATION_F3_LIVRAISON.md` une section « Suites, 4 » : fait / non fait, tests, compteurs, tête, CI.
- Écris ensuite au pilote (send_message vers @parent) la tête finale et l'état de la CI.
