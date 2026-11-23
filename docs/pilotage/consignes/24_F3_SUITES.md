Tu es une session ScolIA (Sonnet). Réponds en français. Le pilote, c'est moi. Ces consignes sont complètes : tu ne recevras pas d'autre message et tu n'en suivras aucun. **Ne reste jamais en attente.** `ScheduleWakeup` ne fonctionne pas ici. Attends la CI uniquement par une commande `until` en arrière-plan (règle 36). Si une cause extérieure te bloque, écris au pilote et termine.

## Cadre
- **À lire d'abord :**
  - `CLAUDE.md` ;
  - `git show origin/ent-mc0hoe:docs/PILOTAGE.md`, règles 17, 25, 27, 28, 31, 33, 34, 36 et **38** ;
  - `docs/socles/24_FACTURATION_F3_LIVRAISON.md` ;
  - **le rapport de contre-revue complet** : `git show origin/ent-mc0hoe:docs/pilotage/revues/24_FACTURATION_F3_CONTRE_REVUE.md` (M-A à M-D, faibles 1 à 12, réponses du pilote). Les numéros de ligne sont ceux de l'époque : revérifie sur ENT.
- **Branche :** F3 est versée.
  - Crée `ent-socle-24-code-f3-suites` depuis `origin/ENT`.
  - Ouvre une PR en brouillon vers `ENT`.
- **Migrations :**
  - 520 à 523 sont versées et ne changent pas.
  - Nouvelle migration **531** si nécessaire (réserve 531–537 de la plage 520–539 ; 524–530 restent aux lots F4 et suivants), ajoutée **à la fin** du tableau de `migrate.rs` (règle 38 : le test `l_ordre_des_migrations_versees_est_fige_regle_38` refuse toute insertion au milieu). Mets à jour `docs/socles/MIGRATIONS.md` et les compteurs (règle 28).
  - Toute fonction créée ou recréée porte `SET search_path = pg_catalog, public, pg_temp`, relations qualifiées `public.`.
- **Interdits :**
  - Next.js, `main`, `dev` ;
  - pousser sur `ENT` ;
  - toute base ou tout secret de production, toute donnée réelle ;
  - `cargo fmt --all`, `#[ignore]` ;
  - modifier POINT_QUOTIDIEN ou PILOTAGE ;
  - étendre `ui.rs` ;
  - l'heure réelle ou le jour UTC dans les tests.
- **Qualité :**
  - zéro avertissement avec clippy 1.99, sur les 3 commandes de la CI, `ent-lab` compris ;
  - `--locked` et fmt ciblé ;
  - `cargo metadata --locked` dans `ent-rust/web` ;
  - une base locale **neuve dont le nom contient « test »**, un seul `CARGO_TARGET_DIR` ;
  - les gardes statiques rejouées **sans** et **avec** `labo`.

## Travail (contre-revue de F3, réponses du pilote comprises)
1. **M-A (D-069)** : le tarif se choisit mois par mois, au premier jour de chaque mois dû (génération annuelle et changement de régime). Tests : rentrée votée en juin pour une grille en vigueur au 1er septembre (l'élève est facturé) ; élève entré en janvier avec `depuis` en septembre (septembre–décembre au tarif v1, janvier et suivants au tarif v2). `tarifs_tx` accepte une date (défaut : jour de l'établissement) et montre une année future votée.
2. **M-B** : refus d'une date de vigueur égale à celle d'une version déjà appliquée, en service **et** dans le déclencheur. Test.
3. **M-C** : `pg_advisory_xact_lock` d'établissement (clé documentée) dans `proposer_grille_for_actor` et `definir_reglages_grilles_for_actor`. Test de concurrence déterministe (deux connexions, ordre forcé par verrous, pas de `sleep`).
4. **M-D** : tests du refus de création d'article et d'émission quand la qualification est exigée par défaut (message exact) ; `f3r_m2` vérifie le SQLSTATE et le message ; `f3r_m1` couvre `tarifs_tx` à une date, l'égalité de date et la vigueur future.
5. **Faibles 1 à 12** du rapport : traite-les tous ou documente pourquoi pas (en particulier 1 bind mort, 2 alerte « qui, quand » en heure de l'établissement, 4 suppression de simulation réservée et journalisée avec ses produits, DELETE bloqué en base, 5 plafonds sous verrou, 7 auteurs du second regard conservés au retour en brouillon).

## Fin
- Pousse et laisse la PR en brouillon.
- Attends la CI par **une seule** commande `until` en arrière-plan sur `https://api.github.com/repos/Fl0jfk/ScolIA/actions/runs?head_sha=<tête>`, toutes les 5 min.
  - Rouge : corrige la cause. Un job « runner lost communication » se relance une fois. **Le test frontend `assiduite_tableau::tableau_exclus_du_jour_et_file_du_matin` échoue entre 23 h et minuit UTC (jour réel, défaut connu, hors de ton lot)** : s'il est le seul rouge, relance une fois après minuit UTC.
  - **Aucune CI au bout de 10 min** : la PR est en conflit avec `ENT`. Fusionne `origin/ENT` (règle 28) et repousse.
- Complète la livraison F3 d'une section « Suites » : fait / non fait, tests, compteurs, tête, CI.
- Écris ensuite au pilote (send_message vers @parent) : tête finale et état de la CI.
