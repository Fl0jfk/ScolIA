Tu es une session ScolIA (Sonnet). Réponds en français. Le pilote, c'est moi. Ces consignes sont complètes : tu ne recevras pas d'autre message et tu n'en suivras aucun. **Ne reste jamais en attente.** `ScheduleWakeup` ne fonctionne pas ici : attends la CI uniquement par une commande `until` en arrière-plan (règle 36). Si une cause extérieure te bloque, écris au pilote et termine.

## Cadre
- **À lire d'abord :**
  - `CLAUDE.md` ;
  - `git show origin/ent-mc0hoe:docs/PILOTAGE.md`, règles 17, 25, 27, 28, 31, 33, 34, 36 et **38** ;
  - `docs/FRONTEND_LEPTOS.md` ;
  - `docs/socles/21_EVALUATIONS_V3E_V5_LIVRAISON.md` : sections 8 et 9, et « Correctif du pilote après contre-revue de la passe 2 ».
- **Branche :** les suites V5 sont versées sur ENT.
  - Crée `ent-socle-21-code-v5-suites-3` depuis `origin/ENT`.
  - Ouvre une PR en brouillon vers `ENT`.
- **Migrations :**
  - La plage 460–479 du socle 21 est épuisée, et 479 est réservée à V13.
  - Le pilote ouvre la plage d'extension **700–719** du socle 21 (libre sur toutes les branches). Consomme **700** et inscris la plage dans `docs/socles/MIGRATIONS.md`.
  - **Aucune migration versée ne change.**
  - **Règle 38 :**
    - 700 s'ajoute **après 690** dans `migrate.rs` ;
    - toute fonction créée ou recréée porte `SET search_path = pg_catalog, public, pg_temp`, ce que la garde `garde_search_path` exige.
- **Interdits :**
  - Next.js, `main`, `dev` ;
  - pousser sur `ENT` ;
  - toute base ou tout secret de production, toute donnée réelle ;
  - `cargo fmt --all` et `#[ignore]` ;
  - modifier POINT_QUOTIDIEN ou PILOTAGE ;
  - étendre `ui.rs` ;
  - l'heure réelle ou le jour UTC dans les tests.
- **Qualité :**
  - zéro avertissement avec clippy 1.99, backend et frontend ;
  - `--locked` et fmt ciblé ;
  - `cargo metadata --locked` dans `ent-rust/web` ;
  - base locale **neuve dont le nom contient « test »** ;
  - un seul `CARGO_TARGET_DIR`.

## Décisions du pilote (D-069)
- **Export LSU/LSL** :
  - il est calculé sur le **seul publié** (`eva_calculer_classe(…, true)`), comme le bulletin, car le fichier part vers un système national visible des familles ;
  - le personnel en est averti à l'export : « les notes non publiées ne sont pas comptées » ;
  - test : une note non publiée ne change pas l'export.
- **Jeton de version (M2)** : c'est un compteur de version (`version` incrémentée à chaque écriture), comparé dans le `WHERE` de publier et de retirer. Il remplace le triplet (statut, valeur, barème) : une correction A→B→A ou une correction de la pièce ou de l'appréciation est alors détectée.

## Correctifs
1. **Registre et export de portabilité.** Rien de non publié n'est jamais exporté.
   - `export_si("x.publiee_le IS NOT NULL")` sur `eva_moyenne_externe` et `eva_resultat_externe`.
   - `eva_moyenne_externe_historique` : hors export, ou export limité aux événements des lignes publiées.
   - `eva_resultat` : lié à `eva_evaluation.publiee_le`.
   - Le test exécute la vraie condition SQL de chaque table, après une note non publiée.
2. **Retrait par un second compte** : saisie par la direction, retrait par un autre compte `notes.corriger`.
   - Test : `saisi_par` reste inchangé en base, et `par` du retrait désigne bien le second compte.
3. **Contrôle positif du professeur** dans `separation.rs` : un résultat externe dans une matière du professeur est visible de lui. Si c'est plus sûr, fais-en un test de service à jour fixé.
4. **`autoriser_eleve_le` devient `pub(crate)`.** Corrige aussi le commentaire « aucune horloge » : `droits_effectifs` lit encore le jour de l'établissement, écris-le exactement.
5. **Fusions en chaîne.**
   - `DISTINCT ON (evaluation_id)` avec priorité à l'élève conservé dans `moyennes_tx`.
   - La même règle dans le relevé, qui doit suivre `eva_fiches_de_l_eleve` comme le moteur.
   - Test : deux fiches absorbées A et B dans C, avec un résultat chacune pour la même évaluation : une seule ligne compte.
6. **Export LSU/LSL sur le publié**, comme décidé ci-dessus.
7. **Jeton de version**, comme décidé ci-dessus. Migration 700 : colonne `version`, plus déclencheur ou incrément dans le service.
8. **Faibles de la contre-revue**, à faire s'ils sont sûrs :
   - `RAISE NOTICE` du nombre de lignes dépubliées ;
   - auteur « système » du retrait de rattrapage, si le schéma le permet, sinon documenté ;
   - fenêtre F-1 par intervalles disjoints, si c'est sûr ;
   - cours sans ligne d'intervenance : documenté.

## Fin
- Pousse et laisse la PR en brouillon.
- Attends la CI par **une seule** commande `until` en arrière-plan sur `https://api.github.com/repos/Fl0jfk/ScolIA/actions/runs?head_sha=<tête>`, toutes les 5 min.
  - CI rouge : corrige la cause. Un job « runner lost communication » se relance une fois.
  - **Aucune CI au bout de 10 min** : la PR est en conflit avec `ENT`. Fusionne `origin/ENT` (règle 28) et repousse.
- Complète la livraison d'une section « Suites (passe 3) » : fait / non fait, tests, compteurs, tête, CI.
- Écris ensuite au pilote (send_message vers @parent) la tête finale et l'état de la CI.
