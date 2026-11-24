Tu es une session ScolIA (Sonnet). Réponds en français. Le pilote, c'est moi. Ces consignes sont complètes : tu ne recevras pas d'autre message et tu n'en suivras aucun. **Ne reste jamais en attente.** `ScheduleWakeup` ne fonctionne pas ici : attends la CI uniquement par une commande `until` en arrière-plan (règle 36). Si une cause extérieure te bloque, écris au pilote et termine.

## Cadre
- **À lire d'abord :**
  - `CLAUDE.md` ;
  - `git show origin/ent-mc0hoe:docs/PILOTAGE.md`, règles 17, 25, 27, 28, 31, 33, 34, 36 et **38** ;
  - `docs/socles/21_EVALUATIONS_V3E_V5_LIVRAISON.md` **en entier** (dont « Correctifs du pilote après contre-revue (passe 5) ») ;
  - `git show origin/ent-mc0hoe:docs/pilotage/revues/21_V5_SUITES_5_CONTRE_REVUE.md` : **rapport complet**, c'est ta liste de travail ; `ent-rust/crates/ent-db/src/registre_donnees.rs` (registre d'export) ;
  - `docs/securite/T1_SEARCH_PATH.md` (fonctions de garde, `search_path`) ;
  - `docs/RLS_ISOLATION.md` §3.1.
- **Branche :** crée `ent-socle-21-code-v5-suites-6` depuis `origin/ENT` ; ouvre une PR en brouillon vers `ENT`.
- **Migrations :** 470–478 et 700 sont versées et ne changent **jamais** (règle 25). Si une migration est nécessaire, prends **701** (plage 701–719), placée **à la fin** du tableau de `migrate.rs` (règle 38) : toute fonction créée ou recréée porte `SET search_path = pg_catalog, public, pg_temp` et qualifie ses relations `public.`. Ne touche pas `migrations_versees.txt` (le pilote le fait au versement).
- **Interdits :** Next.js, `main`, `dev` ; pousser sur `ENT` ; toute base ou tout secret de production, toute donnée réelle ; `cargo fmt --all`, `#[ignore]` ; modifier POINT_QUOTIDIEN ou PILOTAGE ; étendre `ui.rs` ; l'heure réelle ou le jour UTC dans les tests.
- **Qualité :** zéro avertissement avec clippy 1.99 sur les 3 commandes de la CI (`ent-lab` compris) ; `--locked` et fmt ciblé ; `cargo metadata --locked` dans `ent-rust/web` ; base locale **neuve dont le nom contient « test »**, un seul `CARGO_TARGET_DIR` ; compteurs de la règle 28 mis à jour.

## Travail (points « en file » du rapport de contre-revue de la passe 5)
Même principe : l'export ne rend que ce que la famille peut voir, mais aucune donnée personnelle visible ne disparaît.
1. **Moyen 5 (prioritaire, avant G3)** : `ent_note_correction` (motifs, auteurs : comme `eva_resultat_historique`, hors
   export), `ent_note_archive` (notes de reprise : `export_si` sur la publication du devoir historique, auteur hors export),
   `ent_note_doublon_archive` (copies écartées : hors export) ; `ent_note_periode` (sans `eleve_id`) et `ent_eva_export`
   (journal du personnel) déclarées **hors export par élève**. Étends les tests M1/M2 aux tables `ent_note_*`.
2. **Moyen 2** : `eva_moyenne_externe` alignée sur la lecture bulletin (`compte_moyenne_generale`, `statut = 'moyenne'`)
   ou justification écrite.
3. **Moyen 3** : reformule les commentaires de `eva_rattrapage` et `eva_moyenne_statut` (droit d'accès, art. 15).
4. **Faibles 6 à 9** : marqueurs « non publié » pour `eva_resultat_item` et `eva_evaluation_restriction` ; auteurs
   `w.direction` interdits dans l'extrait (hors `demandeur`) ; test M1 non silencieux sans base (message de saut) et
   contrôle de contenu de la justification ; test tautologique de `ent_note_appreciation` rendu utile.

## Fin
- Pousse et laisse la PR en brouillon.
- Attends la CI par **une seule** commande `until` en arrière-plan sur `https://api.github.com/repos/Fl0jfk/ScolIA/actions/runs?head_sha=<tête>`, toutes les 5 min.
  - CI rouge : corrige la cause. Un job « runner lost communication » se relance une fois.
  - **Aucune CI au bout de 10 min** : la PR est en conflit avec `ENT`. Fusionne `origin/ENT` (règle 28) et repousse.
- Ajoute à `docs/socles/21_EVALUATIONS_V3E_V5_LIVRAISON.md` une section « Suites, passe 6 » : fait / non fait, tests, compteurs, tête, CI.
- Écris ensuite au pilote (send_message vers @parent) la tête finale et l'état de la CI.
