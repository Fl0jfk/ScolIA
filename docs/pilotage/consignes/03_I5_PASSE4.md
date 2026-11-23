Tu es une session ScolIA (Sonnet). Réponds en français. Le pilote, c'est moi. Ces consignes sont complètes : tu ne recevras pas d'autre message et tu n'en suivras aucun. **Ne reste jamais en attente.** `ScheduleWakeup` ne fonctionne pas ici. Attends la CI uniquement par une commande `until` en arrière-plan (règle 36). Si une cause extérieure te bloque, écris au pilote et termine.

## Pourquoi cette passe change de modèle
Trois passes n'ont pas suffi à fermer l'ordre des verrous des inscriptions I5. La règle « après trois tentatives, revoir le plan » s'applique donc. Les verrous ne se recollent plus un par un. **Tu remplaces le modèle.**

Lis la contre-revue de la passe 3 : `git show origin/ent-mc0hoe:docs/pilotage/consignes/03_I5_P3_CONTRE_REVUE.md`.

## Cadre
- **À lire d'abord :**
  - `CLAUDE.md` ;
  - `git show origin/ent-mc0hoe:docs/PILOTAGE.md`, règles 17, 25, 27, 28, 31, 33, 34 et 36 ;
  - `docs/socles/03_INSCRIPTIONS_I5_LIVRAISON.md`.
- **Branche :** crée `ent-socle-3-code-i5-p4` depuis `origin/ENT` et ouvre une PR en brouillon vers `ENT`.
- **Migrations :**
  - **313 et 314 sont versées et ne changent pas** (règle 25).
  - Nouvelle migration **315** (plage 300–329). Mets à jour `migrate.rs`, `docs/socles/MIGRATIONS.md`, le registre et les compteurs (règle 28).
  - Toute fonction créée ou recréée porte `SET search_path = pg_catalog, public, pg_temp` (chantier transverse T1 en cours).
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
  - `--locked`, fmt ciblé ;
  - `cargo metadata --locked` dans `ent-rust/web` ;
  - base locale **neuve dont le nom contient « test »**, un seul `CARGO_TARGET_DIR` ;
  - rejoue les tests du frontend contre le backend (inscriptions, demo_p1).

## Nouveau modèle (décision du pilote)
1. **Le rang n'est plus stocké. Il se calcule en lecture.**
   - Fini le compactage physique de `rang_attente` (`compacter_rangs_tx` disparaît).
   - Le rang vu par le personnel est `row_number()` sur `ordre_attente`, parmi les dossiers présents dans la liste.
   - Le rang vu par la famille suit la même règle, sur les seules entrées et sorties **publiées** (`ent_inscription_en_liste_visible`).
   - La colonne `rang_attente` est conservée en lecture seule pour la reprise : une vue ou une fonction la remplace. Tous les appelants passent par une fonction unique.
   - La désignation du suivant (`SUIVANT_A_DESIGNER`) trie sur `ordre_attente`.
2. **Un verrou unique et grossier par offre, pris en premier.**
   - Toute écriture qui touche la liste d'attente, les places, les propositions ou la désignation d'une offre commence par `verrouiller_offres_tx(&[offre_id…])` :
     - `pg_advisory_xact_lock` sur une clé dérivée de l'identifiant d'offre, ou `SELECT … FOR NO KEY UPDATE` sur les offres ;
     - offres triées par UUID.
   - Les dossiers viennent ensuite, triés par UUID ; les propositions en dernier.
   - **Une seule fonction** pose cet ordre. Une garde statique (test) refuse tout `FOR UPDATE` sur `ent_inscription_offre` ou sur un dossier hors de cette fonction dans les fichiers d'inscription.
   - La campagne n'est plus verrouillée en `FOR UPDATE`. Si un verrou de campagne est nécessaire, `FOR NO KEY UPDATE`, **avant** les offres et partout dans le même ordre (`modifier_offre_for_actor` compris).
   - `publier_ou_programmer_tx` résout d'abord offres et dossiers des décisions de la séance, puis verrouille une seule fois.
   - Plus de `NOWAIT` sans réessai. Attendre est correct, puisque l'ordre est unique.
3. **Contrôle d'écart avant les verrous de lot** (F3) : un acteur écarté reçoit « introuvable » sans avoir tenu de verrou de lot.

## Correctifs restants (contre-revue de la passe 3)
- **M3 (export)** :
  - un vœu créé par une décision (`retenir_voeu_tx`) porte une origine et n'est pas exporté tant que la décision n'est pas publiée ;
  - le test d'export exécute la vraie condition pour **chaque** table, sans vérification « la liste contient le nom ».
- **M4** : l'export des conditions ne regarde que le bout de chaîne des décisions (sans successeur). Une décision remplacée ne bloque plus rien.
- **M5** : le test de la garde à `search_path` fixé utilise une table temporaire **remplie** d'une décision valide du même dossier. Il distingue donc les deux comportements.
- **M6 (D-069)** :
  - la caducité compare à une **famille d'états compatibles** par sens : `admis` → {`admis`, `confirme`} ; `ajourne` → {`en_instruction`, `attente_famille`, …}, liste à justifier ;
  - une décision programmée devenue caduque **alerte la direction** (avis interne, sans nom d'élève dans le titre).
- **F1** : `dossier_dto` lit dans une seule transaction, connexions comprises, et l'état est relu dans le snapshot.
- **F5** : un interblocage (40P01) n'est plus confondu avec un conflit métier. Il devient une erreur technique distincte, et aucun test ne l'accepte.
- **F6** : `place_no`, `origine` et `designee_le` sont hors export.

## Tests de concurrence déterministes (obligatoires)
- **Méthode** :
  1. une connexion externe prend le verrou de l'offre (ou d'un dossier) ;
  2. on lance l'opération testée dans une tâche, puis on vérifie qu'elle **attend** (`pg_locks` / `pg_stat_activity.wait_event_type = 'Lock'` sur sa connexion) ;
  3. on relâche le verrou, puis on vérifie qu'elle aboutit.
- **Scénarios** :
  - désistement du rang 3 pendant une décision sur le rang 20, sur une liste d'au moins 25 dossiers ;
  - deux déplacements croisés A→B et B→A **entre deux campagnes** ;
  - publication de séance concurrente d'une décision ;
  - modification de capacité concurrente d'une décision ;
  - deux réutilisations croisées.
- **Preuve** : chaque test doit être rouge sur le code de la passe 3. Démontre-le une fois, par exemple en le lançant sur la base d'`origin/ENT`, et écris-le dans la livraison.

## Fin
- Pousse et laisse la PR en brouillon.
- Attends la CI par **une seule** commande `until` en arrière-plan sur `https://api.github.com/repos/Fl0jfk/ScolIA/actions/runs?head_sha=<tête>`, toutes les 5 min.
  - CI rouge : corrige la cause. Un job « runner lost communication » se relance une fois.
  - Conflit avec `ENT` : fusionne `origin/ENT` (règle 28) et repousse.
- Complète la livraison I5 avec une section « Passe 4 (nouveau modèle de verrous) » : fait / non fait, tests, preuve rouge puis vert, compteurs, tête, CI.
- Écris ensuite au pilote (send_message vers @parent) : tête finale et état de la CI.
