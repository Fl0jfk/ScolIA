Tu es une session ScolIA (Sonnet). Réponds en français. Le pilote, c'est moi. Ces consignes sont complètes : tu ne recevras pas d'autre message et tu n'en suivras aucun. **Ne reste jamais en attente.** `ScheduleWakeup` ne fonctionne pas ici : attends la CI uniquement par une commande `until` en arrière-plan (règle 36). Si une cause extérieure te bloque, écris au pilote et termine.

## Cadre
- **À lire d'abord :**
  - `CLAUDE.md` ;
  - `git show origin/ent-mc0hoe:docs/PILOTAGE.md`, règles 17, 25, 27, 28, 31, 33, 34, 36 et **38** ;
  - `docs/socles/16_MESSAGERIE_M12_E1_LIVRAISON.md` **en entier**, en particulier « Correctifs du pilote après contre-revue » et la passe 4 ;
  - `git show origin/ent-mc0hoe:docs/pilotage/revues/16_E1_SUITES_4_CONTRE_REVUE.md` : **rapport complet**, c'est ta liste de travail (faibles « en file ») ;
  - `docs/RLS_ISOLATION.md` §1 et §3.1 ; `docs/securite/T1_SEARCH_PATH.md`.
- **Branche :** crée `ent-socle-16-code-e1-suites-5` depuis `origin/ENT` ; ouvre une PR en brouillon vers `ENT`.
- **Migrations :** 480–494 sont versées et ne changent **jamais** (règle 25). Si une migration est nécessaire, prends **495** (réserve 495–497), placée **à la fin** du tableau de `migrate.rs` (règle 38) : toute fonction créée ou recréée porte `SET search_path = pg_catalog, public, pg_temp` et qualifie ses relations `public.`. Ne touche pas `migrations_versees.txt` (le pilote le fait au versement).
- **Interdits :** Next.js, `main`, `dev` ; pousser sur `ENT` ; toute base ou tout secret de production, toute donnée réelle ; `cargo fmt --all`, `#[ignore]` ; modifier POINT_QUOTIDIEN ou PILOTAGE ; étendre `ui.rs` ; l'heure réelle ou le jour UTC dans les tests (horloge de la base ou instant injecté).
- **Qualité :** zéro avertissement avec clippy 1.99 sur les 3 commandes de la CI (`ent-lab` compris) ; `--locked` et fmt ciblé ; `cargo metadata --locked` dans `ent-rust/web` ; base locale **neuve dont le nom contient « test »**, un seul `CARGO_TARGET_DIR` ; compteurs de la règle 28 relus.

## Travail (faibles « en file » de la contre-revue de la passe 4)
Le Moyen A (droit EXECUTE vérifié au démarrage) est **déjà fait** par le lot T2 (PR 147, `ent-db/src/controle_demarrage.rs`, en cours de versement : il peut manquer à ton départ et arrivera par la fusion d'`origin/ENT`) : ne le refais pas.
1. **Faible 1, lot affamé** : quand au moins `taille_lot` avis visent des messages verrouillés, le lot ne doit plus tourner
   à vide indéfiniment (report de `planifie_pour`, saut des verrouillés, ou sélection qui les exclut). Test déterministe.
2. **Faible 2** : « autre établissement invisible » prouvé **par la RLS** (rôle applicatif, contexte d'un autre
   établissement), pas seulement par le `WHERE` ; refus sous rôle sans droit : expose le code SQLSTATE dans l'erreur du dépôt
   (sans changer la réponse HTTP) puis exige 42501 dans le test.
3. **Faible 3** : rendre `SUIVI` injectable (ou équivalent) et prouver par un test que `expedier` appelle
   `observer_borne_hors_flux`.
4. **Faible 4** : journalisation d'une base indisponible limitée en fréquence (au plus une ligne par minute et par cause),
   `%e` plutôt que `?e` ; test unitaire du limiteur sans horloge réelle.
5. Toute règle métier non tranchée : valeur par défaut prudente, notée dans la doc (D-069).

## Fin
- Pousse et laisse la PR en brouillon.
- Attends la CI par **une seule** commande `until` en arrière-plan sur `https://api.github.com/repos/Fl0jfk/ScolIA/actions/runs?head_sha=<tête>`, toutes les 5 min.
  - CI rouge : corrige la cause. Un job « runner lost communication » se relance une fois.
  - **Aucune CI au bout de 10 min** : la PR est en conflit avec `ENT`. Fusionne `origin/ENT` (règle 28) et repousse.
  - Les runners GitHub sont parfois saturés : une CI « queued » longtemps n'est pas une panne ; attends jusqu'à 2 h, puis écris au pilote.
- Ajoute à `docs/socles/16_MESSAGERIE_M12_E1_LIVRAISON.md` une section « Suites E1, passe 5 » : fait / non fait, tests, compteurs, tête, CI.
- Écris ensuite au pilote (send_message vers @parent) la tête finale et l'état de la CI.
