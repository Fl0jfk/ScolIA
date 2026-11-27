Tu es une session ScolIA (Sonnet). Réponds en français. Le pilote, c'est moi. Ces consignes sont complètes : tu ne recevras pas d'autre message et tu n'en suivras aucun. **Ne reste jamais en attente.** `ScheduleWakeup` ne fonctionne pas ici : attends la CI uniquement par une commande `until` en arrière-plan (règle 36). Si une cause extérieure te bloque, écris au pilote et termine.

## Cadre
- **À lire d'abord :**
  - `CLAUDE.md` ;
  - `git show origin/ent-mc0hoe:docs/PILOTAGE.md`, règles 17, 25, 27, 28, 31, 33, 34, 36 et **38** ;
  - `docs/socles/03_INSCRIPTIONS_I5_LIVRAISON.md` **en entier**, en particulier « Correctifs du pilote après contre-revue » et la passe 6 ;
  - `git show origin/ent-mc0hoe:docs/pilotage/revues/03_I5_SUITES_6_CONTRE_REVUE.md` : **rapport complet**, c'est ta liste de travail (points « en file ») ;
  - `docs/securite/T1_SEARCH_PATH.md` (fonctions de garde, `search_path`) ;
  - `docs/RLS_ISOLATION.md` §3.1.
- **Branche :** crée `ent-socle-3-code-i5-suites-7` depuis `origin/ENT` ; ouvre une PR en brouillon vers `ENT`.
- **Migrations :** 313–317 sont versées et ne changent **jamais** (règle 25). Si une migration est nécessaire, prends **318**, placée **à la fin** du tableau de `migrate.rs` (règle 38) : toute fonction créée ou recréée porte `SET search_path = pg_catalog, public, pg_temp` et qualifie ses relations `public.`. Ne touche pas `migrations_versees.txt` (le pilote le fait au versement).
- **Interdits :** Next.js, `main`, `dev` ; pousser sur `ENT` ; toute base ou tout secret de production, toute donnée réelle ; `cargo fmt --all`, `#[ignore]` ; modifier POINT_QUOTIDIEN ou PILOTAGE ; étendre `ui.rs` ; l'heure réelle ou le jour UTC dans les tests.
- **Qualité :** zéro avertissement avec clippy 1.99 sur les 3 commandes de la CI (`ent-lab` compris) ; `--locked` et fmt ciblé ; `cargo metadata --locked` dans `ent-rust/web` ; base locale **neuve dont le nom contient « test »**, un seul `CARGO_TARGET_DIR` ; compteurs de la règle 28 mis à jour.

## Travail (points « en file » de la contre-revue de la passe 6)
1. **Moyen D (priorité)** : `ent_inscription_offre_du_dossier` ne doit plus être appelée ligne par ligne quand `offre_id` est
   NULL (déclencheurs par instruction : jointure ensembliste) ; la mesure M4 doit exercer la **fonction réelle** ; test
   multi-lignes portant sur **deux offres** (verrou exigé pour chacune, refus si l'un manque).
2. **Moyen C** (tests) : la garde de rejeu de 304 ne doit jamais laisser la sentinelle absente si un rejeu échoue
   (restauration garantie, test d'échec du rejeu puis présence de la sentinelle).
3. **Moyen A, suite** : vérification de santé au démarrage : `error!` journalisé (sans bloquer) si un déclencheur sentinelle
   n'est pas en `ENABLE ALWAYS` (`tgenabled <> 'A'`) ; test.
4. **Faibles 1 à 3** : rejeu de 313/314 sur un historique non chaîné (garde ou doc) ; repli de l'alerte F6 par le **droit**
   `InscriptionsGerer` (et non par rôle brut) ; échec ouvert hérité de 316 (dossier invisible → offre NULL) : documenté comme
   filet ou fermé si simple. Faible 4 : relecture des tests passe 4 modifiés, anomalies corrigées.
5. Toute règle métier non tranchée : valeur par défaut prudente, notée dans la doc (D-069).

## Fin
- Pousse et laisse la PR en brouillon.
- Attends la CI par **une seule** commande `until` en arrière-plan sur `https://api.github.com/repos/Fl0jfk/ScolIA/actions/runs?head_sha=<tête>`, toutes les 5 min.
  - CI rouge : corrige la cause. Un job « runner lost communication » se relance une fois.
  - **Aucune CI au bout de 10 min** : la PR est en conflit avec `ENT`. Fusionne `origin/ENT` (règle 28) et repousse.
- Ajoute à `docs/socles/03_INSCRIPTIONS_I5_LIVRAISON.md` une section « Suites, passe 7 » : fait / non fait, tests, compteurs, tête, CI.
- Écris ensuite au pilote (send_message vers @parent) la tête finale et l'état de la CI.
