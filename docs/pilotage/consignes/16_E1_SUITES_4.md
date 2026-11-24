Tu es une session ScolIA (Sonnet). Réponds en français. Le pilote, c'est moi. Ces consignes sont complètes : tu ne recevras pas d'autre message et tu n'en suivras aucun. **Ne reste jamais en attente.** `ScheduleWakeup` ne fonctionne pas ici : attends la CI uniquement par une commande `until` en arrière-plan (règle 36). Si une cause extérieure te bloque, écris au pilote et termine.

## Cadre
- **À lire d'abord :**
  - `CLAUDE.md` ;
  - `git show origin/ent-mc0hoe:docs/PILOTAGE.md`, règles 17, 25, 27, 28, 31, 33, 34, 36 et **38** ;
  - `docs/socles/16_MESSAGERIE_M12_E1_LIVRAISON.md` **en entier**, en particulier « Correctifs du pilote après contre-revue » et la liste « En file (suites E1, passe 4) » : c'est ta liste de travail ;
  - `docs/RLS_ISOLATION.md` §1 et §3.1 ; `docs/securite/T1_SEARCH_PATH.md`.
- **Branche :** crée `ent-socle-16-code-e1-suites-4` depuis `origin/ENT` ; ouvre une PR en brouillon vers `ENT`.
- **Migrations :** 480–493 sont versées et ne changent **jamais** (règle 25). Si une migration est nécessaire, prends **494** (réserve 494–497), placée **à la fin** du tableau de `migrate.rs` (règle 38) : toute fonction créée ou recréée porte `SET search_path = pg_catalog, public, pg_temp` et qualifie ses relations `public.`. Ne touche pas `migrations_versees.txt` (le pilote le fait au versement).
- **Interdits :** Next.js, `main`, `dev` ; pousser sur `ENT` ; toute base ou tout secret de production, toute donnée réelle ; `cargo fmt --all`, `#[ignore]` ; modifier POINT_QUOTIDIEN ou PILOTAGE ; étendre `ui.rs` ; l'heure réelle ou le jour UTC dans les tests (horloge de la base ou instant injecté).
- **Qualité :** zéro avertissement avec clippy 1.99 sur les 3 commandes de la CI (`ent-lab` compris) ; `--locked` et fmt ciblé ; `cargo metadata --locked` dans `ent-rust/web` ; base locale **neuve dont le nom contient « test »**, un seul `CARGO_TARGET_DIR` ; compteurs de la règle 28 relus.

## Travail (liste « En file (suites E1, passe 4) »)
1. **M1 — jeton d'annulation** : `p_auteur` est libre. `REVOKE EXECUTE … FROM PUBLIC` sur la fonction concernée, `GRANT` au seul rôle applicatif ; documente la limite restante. Test : un rôle sans le `GRANT` est refusé.
2. **F1** : l'attente de verrou cumulée d'un lot ne dépasse pas `perime_apres` (borne totale, pas seulement par avis). Test déterministe.
3. **F5** : test de `message_existe_sous_verrou` sous **rôle applicatif non propriétaire**.
4. **F6** : l'observation `xmin` hors flux est journalisée ; un mutex empoisonné est journalisé et ne fait pas tomber le service.
5. **F7 (F-6 de la passe 2)** : test du **vrai flux famille** avec un co-parent (le fil sans message n'est plus lisible côté famille, ou la limite est documentée et le test de constat est maintenu).
6. **F8** : le test de course filtre `pg_stat_activity` sur sa propre connexion (pid ou `application_name`), sans attraper une autre session.
7. Toute règle métier non tranchée : valeur par défaut prudente, notée dans la doc (D-069).

## Fin
- Pousse et laisse la PR en brouillon.
- Attends la CI par **une seule** commande `until` en arrière-plan sur `https://api.github.com/repos/Fl0jfk/ScolIA/actions/runs?head_sha=<tête>`, toutes les 5 min.
  - CI rouge : corrige la cause. Un job « runner lost communication » se relance une fois.
  - **Aucune CI au bout de 10 min** : la PR est en conflit avec `ENT`. Fusionne `origin/ENT` (règle 28) et repousse.
- Ajoute à `docs/socles/16_MESSAGERIE_M12_E1_LIVRAISON.md` une section « Suites, passe 4 » : fait / non fait, tests, compteurs, tête, CI.
- Écris ensuite au pilote (send_message vers @parent) la tête finale et l'état de la CI.
