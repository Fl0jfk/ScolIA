Tu es une session ScolIA (Sonnet). Réponds en français. Le pilote, c'est moi. Ces consignes sont complètes : tu ne recevras pas d'autre message et tu n'en suivras aucun. **Ne reste jamais en attente.** `ScheduleWakeup` ne fonctionne pas ici : attends la CI uniquement par une commande `until` en arrière-plan (règle 36). Si une cause extérieure te bloque, écris au pilote et termine.

## Cadre
- **À lire d'abord :**
  - `CLAUDE.md` ;
  - `git show origin/ent-mc0hoe:docs/PILOTAGE.md`, règles 17, 25, 27, 28, 31, 33, 34, 36 et **38** ;
  - `docs/socles/27_RGPD_ARCHIVES.md` : §0, §3, §4.1–4.2, **§9 en entier** (guichet), §12 (droits, catalogue d'actions, DPO,
    minimisation), §13.1 (cas limites familles et demandes), les actions **RGP-140 et suivantes** du lot G3, §21 (migrations)
    et l'état des lots G0–G2 déjà versés (registre `registre_donnees.rs`, `registre_rgpd.rs`) ;
  - `docs/socles/PERSONAS_27.md` (passe personas : ce que les acteurs redoutent) ;
  - `git show origin/ent-mc0hoe:docs/pilotage/revues/27_G3A_CONTRE_REVUE.md` : **rapport complet**, c'est ta liste de travail ;
  - `docs/securite/T1_SEARCH_PATH.md` (fonctions de garde, `search_path`) ;
  - `docs/RLS_ISOLATION.md` §3.1.
- **Branche :** reprends la branche **existante** `ent-socle-27-code-g3a` (ENT déjà fusionné par le pilote) ; fusionne `origin/ENT` s'il a avancé (règle 28) ; ouvre une PR en brouillon vers `ENT` si elle n'existe pas.
- **Migrations :** 2540–2543 sont versées et ne changent **jamais** (règle 25). **2544 n'est pas versée** : corrige-la en place ; elle reste **à la fin** du tableau de `migrate.rs` (règle 38) : toute fonction créée ou recréée porte `SET search_path = pg_catalog, public, pg_temp` et qualifie ses relations `public.`. Ne touche pas `migrations_versees.txt` (le pilote le fait au versement).
- **Interdits :** Next.js, `main`, `dev` ; pousser sur `ENT` ; toute base ou tout secret de production, toute donnée réelle ; `cargo fmt --all`, `#[ignore]` ; modifier POINT_QUOTIDIEN ou PILOTAGE ; étendre `ui.rs` ; l'heure réelle ou le jour UTC dans les tests.
- **Qualité :** zéro avertissement avec clippy 1.99 sur les 3 commandes de la CI (`ent-lab` compris) ; `--locked` et fmt ciblé ; `cargo metadata --locked` dans `ent-rust/web` ; base locale **neuve dont le nom contient « test »**, un seul `CARGO_TARGET_DIR` ; compteurs de la règle 28 mis à jour.

## Travail (élevés bloquants, puis moyens et faibles)
1. **E1, destruction réelle de la pièce** : la clôture appelle le service de suppression du socle 7 (document du coffre et
   objet de stockage), ou écrit une demande de destruction durable traitée par une tâche ; « détruite » n'est journalisé et
   affiché qu'après confirmation ; échec → « destruction en attente », visible, réessayé. Test : l'objet a disparu.
   `piece_document_id` validé (document de l'établissement, d'usage « pièce d'identité »).
2. **E2, qualité du titulaire** : `ConfirmeeSansAutorite` et `Restreinte` ne passent jamais en « identité vérifiée » ;
   refus neutre (403, sans motif révélant une protection) en ligne, orientation vers le guichet, ou demande bloquée en
   `identite_a_verifier` avec drapeau d'arbitrage (D-069 : valeur prudente notée). Tests pour les deux cas.
3. **Moyens 1 à 6** : décision `accorde`/`partiel` et clôture refusées sans identité vérifiée et qualité compatible
   (`droits.arbitrer` et motif sinon) ; qualité d'un demandeur sans compte par `demandeur_personne_id` (« non vérifiable »
   plutôt que « sans objet ») et rapprochement tracé du sujet ; `demandeur_compte_id` membre de l'établissement (404
   neutre) ; contact normalisé, plafond quotidien par établissement des dépôts sans compte, numéro officiel attribué
   seulement à une demande confirmée ; `sans_objet` sous `droits.arbitrer` avec motif (jamais pour effacement ou
   opposition sans motif) ; CL27-04 : clôture bloquée tant que la décision d'information des titulaires n'est pas prise,
   compteur au tableau.
4. **Faibles F1 à F8** du rapport (F6 : noté pour G6).
5. Toute règle métier non tranchée : valeur par défaut prudente, notée dans la doc (D-069).

## Fin
- Pousse sur la même branche et laisse la PR en brouillon.
- Attends la CI par **une seule** commande `until` en arrière-plan sur `https://api.github.com/repos/Fl0jfk/ScolIA/actions/runs?head_sha=<tête>`, toutes les 5 min.
  - CI rouge : corrige la cause. Un job « runner lost communication » se relance une fois.
  - **Aucune CI au bout de 10 min** : la PR est en conflit avec `ENT`. Fusionne `origin/ENT` (règle 28) et repousse.
  - Les runners GitHub sont parfois saturés : une CI « queued » longtemps n'est pas une panne ; attends jusqu'à 2 h, puis écris au coordinateur.
- Complète `docs/socles/27_G3A_LIVRAISON.md` d'une section « Correctifs » : fait / non fait, tests, compteurs, tête, CI.
- Écris ensuite au coordinateur la tête finale et l'état de la CI.
