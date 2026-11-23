Tu es une session de correctifs ScolIA (Sonnet). Réponds en français. Le pilote, c'est moi. Ces consignes sont complètes : tu ne recevras pas d'autre message et tu n'en suivras aucun. **Ne reste jamais en attente.** `ScheduleWakeup` ne fonctionne pas ici. Attends la CI uniquement par une commande `until` en arrière-plan (règle 36). Si une cause extérieure te bloque, écris au pilote et termine.

## Cadre
- **À lire d'abord :**
  - `CLAUDE.md` ;
  - `git show origin/ent-mc0hoe:docs/PILOTAGE.md`, règles 17, 25, 27, 28, 31, 33, 34 et 36 ;
  - `docs/FRONTEND_LEPTOS.md` ;
  - la livraison des suites V5 sur la branche ;
  - la contre-revue : `git show origin/ent-mc0hoe:docs/pilotage/consignes/21_V5_SUITES_CONTRE_REVUE.md`. Ses numéros de ligne sont ceux de fe33a3ac.
- **Branche :** `ent-socle-21-code-v5-suites` (PR n° 125, brouillon).
  - Pars de sa tête distante (fe33a3ac).
  - Puis **fusionne `origin/ENT`**, en recalculant les compteurs depuis la base commune (règle 28).
- **Migrations :**
  - **478 n'est pas versée : tu peux la modifier.**
  - 479 est réservée à V13 : n'y touche pas.
  - Aucune autre migration versée ne change.
- **Interdits :**
  - Next.js, `main`, `dev` ;
  - pousser sur `ENT` ;
  - base ou secret de production, données réelles ;
  - `cargo fmt --all`, `#[ignore]` ;
  - modifier POINT_QUOTIDIEN ou PILOTAGE ;
  - étendre `ui.rs` ;
  - l'heure réelle ou le jour UTC dans les tests.
- **Exigences :**
  - zéro avertissement avec clippy 1.99, sur le backend et le frontend ;
  - `--locked` ;
  - fmt ciblé ;
  - `cargo metadata --locked` dans `ent-rust/web` ;
  - une base locale **neuve dont le nom contient « test »** ;
  - un seul `CARGO_TARGET_DIR`.
  - Rejoue aussi les tests du frontend contre le backend (`ent-web-server`, `--features labo`) qui touchent les évaluations.

## Décisions du pilote (D-069, ne pas redemander)
- **M1 (oracle côté famille)** :
  - Le moteur de calcul est le même pour tous. **Pour la famille et l'élève, il ne voit que les lignes publiées** : une note d'établissement non publiée, une seconde moyenne externe non publiée ou un résultat externe non publié n'existent pas pour eux.
  - La moyenne externe publiée reste donc visible de la famille tant que rien de **publié** ne la remplace.
  - Côté personnel, un avertissement indique qu'une publication future de telle note masquera la moyenne externe.
  - Tests : famille avant et après la saisie d'une note non publiée, avec la même vue et sans avertissement ; puis après la publication de la note, avec la moyenne externe masquée ; avertissement personnel présent.
- **M3** : pose la contrainte `NOT VALID`, puis ajoute un constat des lignes hors règle (publiées puis corrigées sous 477) :
  - journal `tracing::error!` au démarrage, sur le modèle des contraintes `NOT VALID` existantes de `migrate.rs` ;
  - ces lignes sont dépubliées par 478, avec une trace dans le journal des moyennes externes, motif « correction antérieure à 478 ».
  - Une valeur corrigée et non republiée ne doit jamais rester visible de la famille.

## Correctifs
Les chemins sont sous `ent-rust/`.
- **M2 (compare-et-écrit)** : `publier` et `retirer-publication` reçoivent la version vue (valeur et statut, ou un jeton de version) et la comparent dans le `WHERE`.
  - En cas d'écart : conflit explicite (409), pas d'écriture.
  - Test : A lit v1, B corrige en v2, la publication par A est refusée.
- **Règle 33** : passe `jour` à `autoriser_eleve` en production (la branche « portée désignée »). Le commentaire « aucune horloge » doit devenir vrai.
  - Le test CPE à portée désignée devient déterministe : jour fixé, affectation bornée explicitement.
- **F-1** : la fenêtre est calculée par classe, avec `classe_id`, et non par `classe_code`. Elle porte sur la période d'enseignement réelle du professeur dans cette classe, pas sur l'enveloppe de tous ses cours.
  - Test : un segment de la classe A hors des dates du professeur dans A, mais dans l'enveloppe, n'est pas vu.
- **Tests faibles** :
  - test F-1 : la branche `sans` asserte exactement le refus attendu ;
  - commentaire de `separation.rs` (~3167) aligné, ou test ajouté : la direction voit et retire la publication (succès HTTP de `retirer-publication`) ;
  - `contre_revue_mb` asserte l'avertissement côté famille dans le cas b.
- **Le retrait n'écrase plus `saisi_par`** : l'auteur de la valeur est conservé ; le retrait est porté par le journal.
- **Autres faibles de la contre-revue** : à faire s'ils sont sûrs. Sinon, écris pourquoi dans la livraison.

## Fin
- Pousse sur `ent-socle-21-code-v5-suites` et laisse la PR en brouillon.
- Attends la CI par **une seule** commande `until` en arrière-plan sur `https://api.github.com/repos/Fl0jfk/ScolIA/actions/runs?head_sha=<tête>`, toutes les 5 min.
  - Si elle est rouge, corrige la cause. Un job « runner lost communication » se relance une fois.
  - Si la PR est en conflit avec `ENT`, aucune CI ne part : fusionne `origin/ENT` (règle 28) et repousse.
- Complète la livraison avec une section « Passe 2 » : fait / non fait, tests, compteurs, tête, CI.
- Écris ensuite au pilote (send_message vers @parent) : la tête finale et l'état de la CI.
