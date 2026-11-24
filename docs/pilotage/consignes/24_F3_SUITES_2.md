Tu es une session ScolIA (Sonnet). Réponds en français. Le pilote, c'est moi. Ces consignes sont complètes : tu ne recevras pas d'autre message et tu n'en suivras aucun. **Ne reste jamais en attente.** `ScheduleWakeup` ne fonctionne pas ici : attends la CI uniquement par une commande `until` en arrière-plan (règle 36). Si une cause extérieure te bloque, écris au pilote et termine.

## Cadre
- **À lire d'abord :**
  - `CLAUDE.md` ;
  - `git show origin/ent-mc0hoe:docs/PILOTAGE.md`, règles 17, 25, 27, 28, 31, 33, 34, 36 et **38** ;
  - `docs/socles/24_FACTURATION_F3_LIVRAISON.md` **en entier** (dont « Correctifs du pilote après contre-revue des suites ») ;
  - `git show origin/ent-mc0hoe:docs/pilotage/revues/24_FACTURATION_F3_SUITES_CONTRE_REVUE.md` : **rapport complet**, c'est ta liste de travail ;
  - `docs/securite/T1_SEARCH_PATH.md` (fonctions de garde, `search_path`) ;
  - `docs/RLS_ISOLATION.md` §3.1.
- **Branche :** crée `ent-socle-24-code-f3-suites-2` depuis `origin/ENT` ; ouvre une PR en brouillon vers `ENT`.
- **Migrations :** 520–523 et 531 sont versées et ne changent **jamais** (règle 25). Si une migration est nécessaire, prends **532** (réserve 532–537), placée **à la fin** du tableau de `migrate.rs` (règle 38) : toute fonction créée ou recréée porte `SET search_path = pg_catalog, public, pg_temp` et qualifie ses relations `public.`. Ne touche pas `migrations_versees.txt` (le pilote le fait au versement).
- **Interdits :** Next.js, `main`, `dev` ; pousser sur `ENT` ; toute base ou tout secret de production, toute donnée réelle ; `cargo fmt --all`, `#[ignore]` ; modifier POINT_QUOTIDIEN ou PILOTAGE ; étendre `ui.rs` ; l'heure réelle ou le jour UTC dans les tests.
- **Qualité :** zéro avertissement avec clippy 1.99 sur les 3 commandes de la CI (`ent-lab` compris) ; `--locked` et fmt ciblé ; `cargo metadata --locked` dans `ent-rust/web` ; base locale **neuve dont le nom contient « test »**, un seul `CARGO_TARGET_DIR` ; compteurs de la règle 28 mis à jour.

## Travail (points « en file » du rapport de contre-revue des suites F3)
1. **Moyen 1 (suite)** : un mois dû sans tarif applicable **pour un article donné** (alors que d'autres articles sont
   tarifés) n'est jamais gratuit en silence : anomalie bloquante ; test dédié (première grille au 01/10 sans historique, et
   trou propre à un article).
2. **Moyen 2** : test « contributeurs » discriminant au retour en brouillon (un proposant qui n'a jamais touché un tarif
   reste exclu du vote, en service et en SQL direct).
3. **Faibles a à f** du rapport : alerte du second regard (pas d'e-mail en repli, destinataires restreints, pas de date en
   dur) ; publication réservée à la version applicable la plus récente ; comparaison des tarifs par **contenu** entre
   versions (pas de segment ni d'écart d'arrondi pour un article inchangé) ; cache par jour distinct de vigueur ; commentaire
   de l'API corrigé ; test « les simulations ne s'effacent pas » qui vérifie la bonne erreur.
4. Toute règle métier non tranchée : valeur par défaut prudente, notée dans la doc (D-069).

## Fin
- Pousse et laisse la PR en brouillon.
- Attends la CI par **une seule** commande `until` en arrière-plan sur `https://api.github.com/repos/Fl0jfk/ScolIA/actions/runs?head_sha=<tête>`, toutes les 5 min.
  - CI rouge : corrige la cause. Un job « runner lost communication » se relance une fois.
  - **Aucune CI au bout de 10 min** : la PR est en conflit avec `ENT`. Fusionne `origin/ENT` (règle 28) et repousse.
- Ajoute à `docs/socles/24_FACTURATION_F3_LIVRAISON.md` une section « Suites, 2 » : fait / non fait, tests, compteurs, tête, CI.
- Écris ensuite au pilote (send_message vers @parent) la tête finale et l'état de la CI.
