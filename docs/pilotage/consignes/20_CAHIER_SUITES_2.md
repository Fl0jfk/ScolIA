Tu es une session ScolIA (Sonnet). Réponds en français. Le pilote, c'est moi. Ces consignes sont complètes : tu ne recevras pas d'autre message et tu n'en suivras aucun. **Ne reste jamais en attente.** `ScheduleWakeup` ne fonctionne pas ici : attends la CI uniquement par une commande `until` en arrière-plan (règle 36). Si une cause extérieure te bloque, écris au pilote et termine.

## Cadre
- **À lire d'abord :**
  - `CLAUDE.md` ;
  - `git show origin/ent-mc0hoe:docs/PILOTAGE.md`, règles 17, 25, 27, 28, 31, 33, 34, 36 et **38** ;
  - `docs/socles/20_CAHIER_C8_C9_LIVRAISON.md`, en entier, dont les suites et leurs correctifs du pilote ;
  - `git show origin/ent-mc0hoe:docs/pilotage/revues/20_CAHIER_SUITES_CONTRE_REVUE.md` : **rapport complet**, c'est ta liste de travail (faibles « en file ») ;
  - `docs/securite/T1_SEARCH_PATH.md` (fonctions de garde, `search_path`) ;
  - `docs/RLS_ISOLATION.md` §3.1.
- **Branche :** crée `ent-socle-20-code-suites-2` depuis `origin/ENT` ; ouvre une PR en brouillon vers `ENT`.
- **Migrations :** 2400–2419 sont versées et ne changent **jamais** (règle 25). La plage 2400–2419 est épuisée : le pilote réserve au socle 20 la **plage d'extension 2420–2429** (vérifiée libre sur toutes les branches). Si une migration est nécessaire, prends **2420**, et inscris la réservation 2420–2429 dans `docs/socles/MIGRATIONS.md` (ligne du socle 20 : C14 et retrait des anciennes tables y prennent leurs numéros futurs) ; la migration est, placée **à la fin** du tableau de `migrate.rs` (règle 38) : toute fonction créée ou recréée porte `SET search_path = pg_catalog, public, pg_temp` et qualifie ses relations `public.`. Ne touche pas `migrations_versees.txt` (le pilote le fait au versement).
- **Interdits :** Next.js, `main`, `dev` ; pousser sur `ENT` ; toute base ou tout secret de production, toute donnée réelle ; `cargo fmt --all`, `#[ignore]` ; modifier POINT_QUOTIDIEN ou PILOTAGE ; étendre `ui.rs` ; l'heure réelle ou le jour UTC dans les tests.
- **Qualité :** zéro avertissement avec clippy 1.99 sur les 3 commandes de la CI (`ent-lab` compris) ; `--locked` et fmt ciblé ; `cargo metadata --locked` dans `ent-rust/web` ; base locale **neuve dont le nom contient « test »**, un seul `CARGO_TARGET_DIR` ; compteurs de la règle 28 mis à jour.

## Travail (faibles « en file » de la contre-revue des suites du cahier)
1. **Faible 8, gel** (décision D-069 du pilote : prudente) : `ent_cdt_echeance_eleve` et `ent_cdt_delai_demande` rejoignent le
   gel des années closes (même mécanisme que les autres tables filles : pas d'écriture sur une année gelée, fusion d'élève
   limitée aux années ouvertes). Migration **2420**. Tests SQL directs.
2. **Faible 3** : tests de déplacement d'une ligne vers ou depuis une année close (refusé dans les deux sens).
3. **Faibles 4 et 5** : test F3 qui exige le message, pas seulement 23514 ; messages des tables filles traduits en 409
   lisible (`ref_commun::db` comme `cahier_c5`).
4. **Faible 6** : branche `ent_app` du REVOKE exercée ; nom de rôle non codé en dur ; `rolname <> current_user`.
5. **Faible 7** : réservation 2420–2429 inscrite (voir Cadre) ; C14 et retrait des anciennes tables y reçoivent un numéro.
6. **Faibles 9 et 10** : le test F3 n'altère plus `ent_annee_scolaire` (verrou ACCESS EXCLUSIVE sur une table commune) ;
   F4 compare les comptes de requêtes par une borne, pas une égalité stricte.
7. Toute règle métier non tranchée : valeur par défaut prudente, notée dans la doc (D-069).

## Fin
- Pousse et laisse la PR en brouillon.
- Attends la CI par **une seule** commande `until` en arrière-plan sur `https://api.github.com/repos/Fl0jfk/ScolIA/actions/runs?head_sha=<tête>`, toutes les 5 min.
  - CI rouge : corrige la cause. Un job « runner lost communication » se relance une fois.
  - **Aucune CI au bout de 10 min** : la PR est en conflit avec `ENT`. Fusionne `origin/ENT` (règle 28) et repousse.
  - Les runners GitHub sont parfois saturés : une CI « queued » longtemps n'est pas une panne ; attends jusqu'à 2 h, puis écris au pilote.
- Ajoute à `docs/socles/20_CAHIER_C8_C9_LIVRAISON.md` une section « Suites, 2 » : fait / non fait, tests, compteurs, tête, CI.
- Écris ensuite au pilote (send_message vers @parent) la tête finale et l'état de la CI.
