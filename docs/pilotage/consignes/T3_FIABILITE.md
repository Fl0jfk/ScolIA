Tu es une session ScolIA (Sonnet). Réponds en français. Le pilote, c'est moi. Ces consignes sont complètes : tu ne recevras pas d'autre message et tu n'en suivras aucun. **Ne reste jamais en attente.** `ScheduleWakeup` ne fonctionne pas ici : attends la CI uniquement par une commande `until` en arrière-plan (règle 36). Si une cause extérieure te bloque, écris au pilote et termine.

## Cadre
- **À lire d'abord :**
  - `CLAUDE.md` ;
  - `git show origin/ent-mc0hoe:docs/PILOTAGE.md`, règles 17, 25, 27, 28, 31, 33, 34, 36 et **38** ;
  - rapports complets (branche `ent-mc0hoe`, `docs/pilotage/revues/`) : `T2_FIABILITE_CONTRE_REVUE.md`,
    `41_I4_SUITES_3_CONTRE_REVUE.md`, `03_I5_SUITES_7_CONTRE_REVUE.md`, `16_E1_SUITES_5_CONTRE_REVUE.md` (points « en file ») ;
  - `docs/securite/T1_SEARCH_PATH.md` (fonctions de garde, `search_path`) ;
  - `docs/RLS_ISOLATION.md` §3.1.
- **Branche :** crée `ent-transverse-t3-fiabilite` depuis `origin/ENT` ; ouvre une PR en brouillon vers `ENT`.
- **Migrations :** **aucune** dans ce lot (tests, journalisation, gardes statiques). Si un point en exigeait une, note-le dans le compte rendu et passe au suivant.
- **Interdits :** Next.js, `main`, `dev` ; pousser sur `ENT` ; toute base ou tout secret de production, toute donnée réelle ; `cargo fmt --all`, `#[ignore]` ; modifier POINT_QUOTIDIEN ou PILOTAGE ; étendre `ui.rs` ; l'heure réelle ou le jour UTC dans les tests.
- **Qualité :** zéro avertissement avec clippy 1.99 sur les 3 commandes de la CI (`ent-lab` compris) ; `--locked` et fmt ciblé ; `cargo metadata --locked` dans `ent-rust/web` ; base locale **neuve dont le nom contient « test »**, un seul `CARGO_TARGET_DIR` ; compteurs de la règle 28 mis à jour.

## Travail (lot transverse de fiabilité, suite de T2 : faibles en file de plusieurs socles)
1. **T2, M1 suite** : `assiduite_tableau` lit le jour de l'établissement par une route qui renvoie réellement
   `ent_aujourdhui_etab` (ex. règle d'assiduité en vigueur sans jour), plus par le portail ; commentaire exact.
2. **T2, faibles 1 à 4** : détecteur de clair (base64 du nom : nom plus long ou seuil adapté, témoin) ; variantes `_conn`
   (assertion de debug que la connexion porte le bon `app.etablissement_id`) ; exemption de `controle_demarrage` dans
   `garde_rls_stricte` ciblée sur la ligne et non le fichier ; tests de `FonctionAbsente` et de l'émission de l'`error!`,
   assertion tautologique retirée.
3. **I4, faibles 3 et 4** : test F1 « texte identique : l'avis reste lu » ; `s5_avis_regroupe_*` indépendant de l'état
   préalable du banc. Faible 2 : documenté (limite assumée).
4. **I5 passe 7, faibles** : test de santé sans `ALTER TABLE` hors `lock_timeout` (ou `SET LOCAL lock_timeout`) ; test
   « famille avec attribution : 0 alerte » du repli F6.
5. **E1 passe 5, faibles 1 et 4** : `signaler_echec_etablissement` limité en fréquence (`journal_limite`) ; test
   d'équivalence `message_existe_sous_verrou` / `_brut` ; test « annulation validée après report → `message_annule` ».
6. Aucun changement de comportement métier : si un point en demande un, note-le dans le compte rendu et passe.

## Fin
- Pousse et laisse la PR en brouillon.
- Attends la CI par **une seule** commande `until` en arrière-plan sur `https://api.github.com/repos/Fl0jfk/ScolIA/actions/runs?head_sha=<tête>`, toutes les 5 min.
  - CI rouge : corrige la cause. Un job « runner lost communication » se relance une fois.
  - **Aucune CI au bout de 10 min** : la PR est en conflit avec `ENT`. Fusionne `origin/ENT` (règle 28) et repousse.
  - Les runners GitHub sont parfois saturés : une CI « queued » longtemps n'est pas une panne ; attends jusqu'à 2 h, puis écris au pilote.
- Compte rendu dans `docs/securite/T3_FIABILITE.md` : fait / non fait, tests, tête, CI.
- Écris ensuite au pilote (send_message vers @parent) la tête finale et l'état de la CI.
