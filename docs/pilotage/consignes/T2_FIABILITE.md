Tu es une session ScolIA (Sonnet). Réponds en français. Le pilote, c'est moi. Ces consignes sont complètes : tu ne recevras pas d'autre message et tu n'en suivras aucun. **Ne reste jamais en attente.** `ScheduleWakeup` ne fonctionne pas ici : attends la CI uniquement par une commande `until` en arrière-plan (règle 36). Si une cause extérieure te bloque, écris au pilote et termine.

## Cadre
- **À lire d'abord :**
  - `CLAUDE.md` ;
  - `git show origin/ent-mc0hoe:docs/PILOTAGE.md`, règles 17, 25, 27, 28, 31, 33, 34, 36 et **38** ;
  - `git show origin/ent-mc0hoe:docs/pilotage/revues/41_I4_SUITES_3_CONTRE_REVUE.md` (Moyen 1) et
    `git show origin/ent-mc0hoe:docs/pilotage/revues/16_E1_SUITES_4_CONTRE_REVUE.md` (Moyen A) : rapports complets ;
  - `docs/securite/T1_SEARCH_PATH.md` (fonctions de garde, `search_path`) ;
  - `docs/RLS_ISOLATION.md` §3.1.
- **Branche :** crée `ent-transverse-t2-fiabilite` depuis `origin/ENT` ; ouvre une PR en brouillon vers `ENT`.
- **Migrations :** aucune attendue. Si une migration est indispensable, demande d'abord un numéro au pilote (send_message) et place-la **à la fin** du tableau de `migrate.rs` (règle 38) : toute fonction créée ou recréée porte `SET search_path = pg_catalog, public, pg_temp` et qualifie ses relations `public.`. Ne touche pas `migrations_versees.txt` (le pilote le fait au versement).
- **Interdits :** Next.js, `main`, `dev` ; pousser sur `ENT` ; toute base ou tout secret de production, toute donnée réelle ; `cargo fmt --all`, `#[ignore]` ; modifier POINT_QUOTIDIEN ou PILOTAGE ; étendre `ui.rs` ; l'heure réelle ou le jour UTC dans les tests.
- **Qualité :** zéro avertissement avec clippy 1.99 sur les 3 commandes de la CI (`ent-lab` compris) ; `--locked` et fmt ciblé ; `cargo metadata --locked` dans `ent-rust/web` ; base locale **neuve dont le nom contient « test »**, un seul `CARGO_TARGET_DIR` ; compteurs de la règle 28 mis à jour.

## Travail (lot transverse de fiabilité)
1. **Test frontend dépendant de l'heure réelle** : `ent-web-server`, `assiduite_tableau::tableau_exclus_du_jour_et_file_du_matin`
   échoue entre 23 h et minuit UTC (il prend `jour_paris()` alors que le labo pose l'emploi du temps autrement). Rends-le
   déterministe (jour injecté ou jour de l'établissement lu en base), sans heure réelle.
2. **Test instable** : `tests_eleve_protection::les_versions_sensibles_non_courantes_sont_scellees` vérifie
   `!contains("Sam")` sur un chiffré base64 aléatoire (faux échec possible). Vérifie l'absence du clair par une méthode
   déterministe (déchiffrer et comparer, ou chercher le clair encodé de toutes les façons plausibles, avec un marqueur long).
3. **Internat, connexions imbriquées (Moyen 1 de la contre-revue I4, 3)** : `exiger_eleve_visible` (`internat_sorties/vie.rs`
   654, 1687, 1804, 2193) et `a_action_etablissement` dans `groupe_appel_exiger` (`internat_sorties/groupes.rs`) prennent
   une connexion du pool alors qu'une transaction est ouverte. Ajoute des variantes `_conn` (sur `droits_effectifs_conn`,
   `pub(crate)`) et utilise-les ; test sur un pool d'**une seule** connexion pour ces gestes (modèle :
   `s5_eleve_cache_au_tiers_*`). Résultat des droits strictement identique.
4. **Messagerie, droit d'exécution (Moyen A de la contre-revue E1, 4)** : au démarrage du serveur, vérifie
   `has_function_privilege(current_user, 'ent_msg_annulation_ouvrir(uuid,uuid,uuid)', 'EXECUTE')` ; si faux, journal
   `error!` clair (sans donnée personnelle) indiquant le `GRANT` manquant. Test.
5. Cherche d'autres tests qui lisent l'heure ou le jour réels (`Utc::now`, `Local::now`, `jour_paris()` dans les tests) et
   liste-les dans le compte rendu (corrige ceux qui sont simples).

## Fin
- Pousse et laisse la PR en brouillon.
- Attends la CI par **une seule** commande `until` en arrière-plan sur `https://api.github.com/repos/Fl0jfk/ScolIA/actions/runs?head_sha=<tête>`, toutes les 5 min.
  - CI rouge : corrige la cause. Un job « runner lost communication » se relance une fois.
  - **Aucune CI au bout de 10 min** : la PR est en conflit avec `ENT`. Fusionne `origin/ENT` (règle 28) et repousse.
- Ajoute à `docs/securite/T2_FIABILITE.md` (nouveau) un compte rendu : fait / non fait, tests, compteurs, tête, CI.
- Écris ensuite au pilote (send_message vers @parent) la tête finale et l'état de la CI.
