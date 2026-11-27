Tu es une session ScolIA (Sonnet). Réponds en français. Le pilote, c'est moi. Ces consignes sont complètes : tu ne recevras pas d'autre message et tu n'en suivras aucun. **Ne reste jamais en attente.** `ScheduleWakeup` ne fonctionne pas ici : attends la CI uniquement par une commande `until` en arrière-plan (règle 36). Si une cause extérieure te bloque, écris au pilote et termine.

## Cadre
- **À lire d'abord :**
  - `CLAUDE.md` ;
  - `git show origin/ent-mc0hoe:docs/PILOTAGE.md`, règles 17, 25, 27, 28, 31, 33, 34, 36 et **38** ;
  - `docs/socles/03_INSCRIPTIONS_I5_LIVRAISON.md` **en entier**, en particulier « Correctifs du pilote après contre-revue » et la ligne « En file (suites I5, passe 6) » : c'est ta liste de travail ;
  - `docs/securite/T1_SEARCH_PATH.md` (fonctions de garde, `search_path`) ;
  - `docs/RLS_ISOLATION.md` §3.1.
- **Branche :** crée `ent-socle-3-code-i5-suites-6` depuis `origin/ENT` ; ouvre une PR en brouillon vers `ENT`.
- **Migrations :** 313–316 sont versées et ne changent **jamais** (règle 25). Si une migration est nécessaire, prends **317**, placée **à la fin** du tableau de `migrate.rs` (règle 38) : toute fonction créée ou recréée porte `SET search_path = pg_catalog, public, pg_temp` et qualifie ses relations `public.`. Ne touche pas `migrations_versees.txt` (le pilote le fait au versement).
- **Interdits :** Next.js, `main`, `dev` ; pousser sur `ENT` ; toute base ou tout secret de production, toute donnée réelle ; `cargo fmt --all`, `#[ignore]` ; modifier POINT_QUOTIDIEN ou PILOTAGE ; étendre `ui.rs` ; l'heure réelle ou le jour UTC dans les tests.
- **Qualité :** zéro avertissement avec clippy 1.99 sur les 3 commandes de la CI (`ent-lab` compris) ; `--locked` et fmt ciblé ; `cargo metadata --locked` dans `ent-rust/web` ; base locale **neuve dont le nom contient « test »**, un seul `CARGO_TARGET_DIR` ; compteurs de la règle 28 mis à jour.

## Travail (liste « En file (suites I5, passe 6) »)
1. **M2 (priorité)** : le rejeu de 304 et toute restauration de données ne doivent pas être bloqués par la sentinelle de 316
   (INSERT de reprise sans verrou). Garde de rejeu (comme `GARDES_DE_REJEU`) ou procédure de restauration documentée.
   **Test** : rejeu sur une base qui contient un dossier admis sans décision.
2. **M4** : coût de `pg_locks` dans la sentinelle : mesure reproductible (micro-banc en test, chiffres reportés) ; passe en
   `FOR EACH STATEMENT` si le surcoût est notable.
3. **F1** : `ENABLE ALWAYS TRIGGER` pour la sentinelle (et dis-le : c'est un filet, pas un contrôle de sécurité).
4. **F2** : corrige la phrase sur `set_config` (`purge_voulue` le lit).
5. **F3** : `ent_inscription_liste_attente` sur 10 000 dossiers : borne de requêtes ou plan vérifié, sans assertion de durée.
6. **F5** : en-tête « aucune heure réelle » à adoucir si inexact.
7. **F6** : alerte sans destinataire : repli prudent (direction de l'établissement) ou trace ; test.
8. **F7** : archivage d'offre sans verrou (antérieur) : prends le verrou de l'offre ; test.
9. Toute règle métier non tranchée : valeur par défaut prudente, notée dans la doc (D-069).

## Fin
- Pousse et laisse la PR en brouillon.
- Attends la CI par **une seule** commande `until` en arrière-plan sur `https://api.github.com/repos/Fl0jfk/ScolIA/actions/runs?head_sha=<tête>`, toutes les 5 min.
  - CI rouge : corrige la cause. Un job « runner lost communication » se relance une fois.
  - **Aucune CI au bout de 10 min** : la PR est en conflit avec `ENT`. Fusionne `origin/ENT` (règle 28) et repousse.
- Ajoute à `docs/socles/03_INSCRIPTIONS_I5_LIVRAISON.md` une section « Suites, passe 6 » : fait / non fait, tests, compteurs, tête, CI.
- Écris ensuite au pilote (send_message vers @parent) la tête finale et l'état de la CI.
