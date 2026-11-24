Tu es une session ScolIA (Sonnet). Réponds en français. Le pilote, c'est moi. Ces consignes sont complètes : tu ne recevras pas d'autre message et tu n'en suivras aucun. **Ne reste jamais en attente.** `ScheduleWakeup` ne fonctionne pas ici : attends la CI uniquement par une commande `until` en arrière-plan (règle 36). Si une cause extérieure te bloque, écris au pilote et termine.

## Cadre
- **À lire d'abord :**
  - `CLAUDE.md` ;
  - `git show origin/ent-mc0hoe:docs/PILOTAGE.md`, règles 17, 25, 27, 28, 31, 33, 34, 36 et **38** ;
  - `docs/socles/24_FACTURATION_F3_LIVRAISON.md` **en entier** (dont « Suites, 3 ») ;
  - `git show origin/ent-mc0hoe:docs/pilotage/revues/24_F3_SUITES_3_CONTRE_REVUE.md` : **rapport complet**, c'est ta liste de travail ;
  - `docs/securite/T1_SEARCH_PATH.md` (fonctions de garde, `search_path`) ;
  - `docs/RLS_ISOLATION.md` §3.1.
- **Branche :** reprends la branche **existante** `ent-socle-24-code-f3-suites-3` (PR 150, brouillon, retenue) ; fusionne d'abord `origin/ENT` (règle 28).
- **Migrations :** 520–523 et 531 sont versées et ne changent **jamais** (règle 25). **532 n'est pas versée** : corrige-la en place si besoin ; une migration supplémentaire prend **533** (réserve 533–537), placée **à la fin** du tableau de `migrate.rs` (règle 38) : toute fonction créée ou recréée porte `SET search_path = pg_catalog, public, pg_temp` et qualifie ses relations `public.`. Ne touche pas `migrations_versees.txt` (le pilote le fait au versement).
- **Interdits :** Next.js, `main`, `dev` ; pousser sur `ENT` ; toute base ou tout secret de production, toute donnée réelle ; `cargo fmt --all`, `#[ignore]` ; modifier POINT_QUOTIDIEN ou PILOTAGE ; étendre `ui.rs` ; l'heure réelle ou le jour UTC dans les tests.
- **Qualité :** zéro avertissement avec clippy 1.99 sur les 3 commandes de la CI (`ent-lab` compris) ; `--locked` et fmt ciblé ; `cargo metadata --locked` dans `ent-rust/web` ; base locale **neuve dont le nom contient « test »**, un seul `CARGO_TARGET_DIR` ; compteurs de la règle 28 mis à jour.

## Travail (Élevés bloquants, puis moyens et faibles)
1. **E1 + E2, garde par périodes** : la garde anti double facturation se fonde sur le **chevauchement des périodes**
   (`periode_debut`/`periode_fin`) et le **montant** des lignes déjà posées pour (élève, article, foyer), réductions et
   `:suivant:` compris. Période disjointe → facturable (renouvellement d'une prise). Même clé mais période ou montant
   différent (prolongation d'une prise) → **jamais ignorée en silence** : facture les mois manquants seulement (clé nouvelle
   par mois de début), ou refuse avec un motif explicite. Aucune ligne déjà posée n'est refacturée. Tests chiffrés :
   prolongation puis rejeu (les 6 mois manquants facturés une fois), renouvellement par une nouvelle prise, rejeu sans
   modification (rien de nouveau), double rejeu.
2. **M1** : une prise `echue` reste valide pour les mois de sa période ; seule `annulee` est exclue. Test : alerte passée
   avant la génération.
3. **M2** : la tâche `regularisation_regime_non_facturee` est liée à l'**événement** (détail structuré ou colonne, via
   532 corrigée en place) et n'est close que par la régularisation de cet événement. Test A échoue, B réussit : la tâche
   de A reste ouverte.
4. **M3** : remise arrondie **en faveur de la famille** (vers le haut) ; doc, commentaire et test alignés.
5. **M4** : test de cloisonnement SQL discriminant (assertion à 0 sous le rôle applicatif strict).
6. **Faibles 1, 2, 4** du rapport ; faible 3 : règle prudente notée (D-069) ; faible 5 assumé.
7. Toute règle métier non tranchée : valeur par défaut prudente, notée dans la doc (D-069).

## Fin
- Pousse sur la même branche et laisse la PR 150 en brouillon.
- Attends la CI par **une seule** commande `until` en arrière-plan sur `https://api.github.com/repos/Fl0jfk/ScolIA/actions/runs?head_sha=<tête>`, toutes les 5 min.
  - CI rouge : corrige la cause. Un job « runner lost communication » se relance une fois.
  - **Aucune CI au bout de 10 min** : la PR est en conflit avec `ENT`. Fusionne `origin/ENT` (règle 28) et repousse.
  - Les runners GitHub sont parfois saturés : une CI « queued » longtemps n'est pas une panne ; attends jusqu'à 2 h, puis écris au pilote.
- Ajoute à `docs/socles/24_FACTURATION_F3_LIVRAISON.md` une section « Suites, 3, correctifs » : fait / non fait, tests, compteurs, tête, CI.
- Écris ensuite au pilote (send_message vers @parent) la tête finale et l'état de la CI.
