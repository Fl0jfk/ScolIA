Tu es une session ScolIA (Sonnet). Réponds en français. Le pilote, c'est moi. Ces consignes sont complètes : tu ne recevras pas d'autre message et tu n'en suivras aucun. **Ne reste jamais en attente.** `ScheduleWakeup` ne fonctionne pas ici : attends la CI uniquement par une commande `until` en arrière-plan (règle 36). Si une cause extérieure te bloque, écris au pilote et termine.

## Cadre
- **À lire d'abord :**
  - `CLAUDE.md` ;
  - `git show origin/ent-mc0hoe:docs/PILOTAGE.md`, règles 17, 25, 27, 28, 31, 33, 34, 36 et **38** ;
  - `docs/socles/41_INTERNAT_I4_LIVRAISON.md` **en entier**, en particulier « Correctifs du pilote après contre-revue » et la ligne « En file » (F1, F2, F4, F6, F7) : c'est ta liste de travail ;
  - `docs/securite/T1_SEARCH_PATH.md` (fonctions de garde, `search_path`) ;
  - `docs/RLS_ISOLATION.md` §3.1.
- **Branche :** crée `ent-socle-41-code-i4-suites-3` depuis `origin/ENT` ; ouvre une PR en brouillon vers `ENT`.
- **Migrations :** 2828–2830 sont versées et ne changent **jamais** (règle 25). Si une migration est nécessaire, prends **2831**, placée **à la fin** du tableau de `migrate.rs` (règle 38) : toute fonction créée ou recréée porte `SET search_path = pg_catalog, public, pg_temp` et qualifie ses relations `public.`. Ne touche pas `migrations_versees.txt` (le pilote le fait au versement).
- **Interdits :** Next.js, `main`, `dev` ; pousser sur `ENT` ; toute base ou tout secret de production, toute donnée réelle ; `cargo fmt --all`, `#[ignore]` ; modifier POINT_QUOTIDIEN ou PILOTAGE ; étendre `ui.rs` ; l'heure réelle ou le jour UTC dans les tests.
- **Qualité :** zéro avertissement avec clippy 1.99 sur les 3 commandes de la CI (`ent-lab` compris) ; `--locked` et fmt ciblé ; `cargo metadata --locked` dans `ent-rust/web` ; base locale **neuve dont le nom contient « test »**, un seul `CARGO_TARGET_DIR` ; compteurs de la règle 28 mis à jour.

## Travail (faibles en file des suites I4)
1. **F1** : une minute dont l'avis est déjà lu : remets l'avis à « non lu » quand la minute change, ou documente pourquoi non ; test.
2. **F2** : envoi externe déjà parti non mis à jour : documente le comportement (et, si simple, un avis rectificatif) ; test du constat.
3. **F4** : test d'avis regroupé prouvant que le 3e événement est consommé ; `notifier_personne` inchangé ; un envoi `envoye` n'est jamais réécrit (test).
4. **F6** : libellé `refus_apres_depart` (texte destiné aux familles clair, sans jargon) ; test du libellé.
5. **F7** : `eleve_cache_au_tiers` n'ouvre plus de connexion supplémentaire dans une transaction ouverte (utilise la connexion de la transaction) ; test.
6. Toute règle métier non tranchée : valeur par défaut prudente, notée dans la doc (D-069).

## Fin
- Pousse et laisse la PR en brouillon.
- Attends la CI par **une seule** commande `until` en arrière-plan sur `https://api.github.com/repos/Fl0jfk/ScolIA/actions/runs?head_sha=<tête>`, toutes les 5 min.
  - CI rouge : corrige la cause. Un job « runner lost communication » se relance une fois.
  - **Aucune CI au bout de 10 min** : la PR est en conflit avec `ENT`. Fusionne `origin/ENT` (règle 28) et repousse.
- Ajoute à `docs/socles/41_INTERNAT_I4_LIVRAISON.md` une section « Suites, 3 » : fait / non fait, tests, compteurs, tête, CI.
- Écris ensuite au pilote (send_message vers @parent) la tête finale et l'état de la CI.
