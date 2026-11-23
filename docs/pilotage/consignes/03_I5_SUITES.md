Tu es une session ScolIA (Sonnet). Réponds en français. Le pilote, c'est moi. Ces consignes sont complètes : tu ne recevras pas d'autre message et tu n'en suivras aucun. **Ne reste jamais en attente.** `ScheduleWakeup` ne fonctionne pas ici. Attends la CI uniquement par une commande `until` en arrière-plan (règle 36). Si une cause extérieure te bloque, écris au pilote et termine.

## Cadre
- **À lire d'abord :**
  - `CLAUDE.md` ;
  - `git show origin/ent-mc0hoe:docs/PILOTAGE.md`, règles 17, 25, 27, 28, 31, 33, 34, 36 et **38** ;
  - `docs/socles/03_INSCRIPTIONS_I5_LIVRAISON.md`, section de la passe 4 (nouveau modèle de verrous) et correctif du pilote ;
  - `git show origin/ent-mc0hoe:docs/pilotage/SESSIONS_EN_COURS.md`, entrée « Contre-revue I5 passe 4 (PR 130) ».
- **Branche :** I5 passe 4 est versée.
  - Crée `ent-socle-3-code-i5-suites` depuis `origin/ENT`.
  - Ouvre une PR en brouillon vers `ENT`.
- **Migrations :**
  - 313 à 315 sont versées et ne changent pas.
  - Nouvelle migration **316** si nécessaire (plage 310–329), ajoutée **à la fin** du tableau de `migrate.rs` (règle 38).
  - Toute fonction créée ou recréée porte `SET search_path = pg_catalog, public, pg_temp`, relations qualifiées `public.`. Aucune garde ne s'appuie sur `PG_CONTEXT` ni sur `set_config` falsifiable par le rôle applicatif.
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

## Travail (contre-revue de la passe 4)
1. **M-2 : sentinelle en base de l'ordre des verrous.** La garde statique est contournable. Ajoute dans 316 un déclencheur sur les tables de places, propositions et décisions qui exige que la transaction tienne le verrou consultatif de l'offre concernée (`pg_locks` filtré sur `locktype = 'advisory'`, la clé de l'offre et `pid = pg_backend_pid()`), sinon il lève une erreur nommée.
   - Mesure le surcoût par écriture (banc court) et reporte le chiffre.
   - Parcours tous les `inscriptions*.rs` : chaque écriture concernée prend le verrou d'offre avant. Les tests prouvent qu'une écriture hors verrou est refusée.
2. **M-3 : `repondre_accord`.** Il prend la demande avant l'offre, à rebours de l'ordre (offres → dossiers → propositions). Remets-le dans l'ordre et ajoute un test de concurrence déterministe contre une décision de séance.
3. **F-a / F-b** : les écritures de places et les écritures de masse hors verrou d'offre passent sous ce verrou (la sentinelle du point 1 doit les attraper).
4. **F-c** : la formule de rang dupliquée n'a plus qu'une seule source (fonction SQL ou module Rust unique), et un test compare les deux usages.
5. **F-e** : les directeurs partis (fin de fonction, compte désactivé) ne reçoivent plus les alertes. Test.
6. **Liste nominative** des tests de concurrence qui étaient verts sur l'ancien code, avec l'explication, dans la livraison.

## Fin
- Pousse et laisse la PR en brouillon.
- Attends la CI par **une seule** commande `until` en arrière-plan sur `https://api.github.com/repos/Fl0jfk/ScolIA/actions/runs?head_sha=<tête>`, toutes les 5 min.
  - Rouge : corrige la cause. Un job « runner lost communication » se relance une fois.
  - **Aucune CI au bout de 10 min** : la PR est en conflit avec `ENT`. Fusionne `origin/ENT` (règle 28) et repousse.
- Complète la livraison I5 d'une section « Suites (passe 5) » : fait / non fait, tests, mesures, tête, CI.
- Écris ensuite au pilote (send_message vers @parent) : tête finale et état de la CI.
