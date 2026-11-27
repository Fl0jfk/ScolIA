Tu es une session ScolIA (Sonnet). Réponds en français. Le pilote, c'est moi. Ces consignes sont complètes : tu ne recevras pas d'autre message et tu n'en suivras aucun. **Ne reste jamais en attente.** `ScheduleWakeup` ne fonctionne pas ici : attends la CI uniquement par une commande `until` en arrière-plan (règle 36). Si une cause extérieure te bloque, écris au pilote et termine.

## Cadre
- **À lire d'abord :**
  - `CLAUDE.md` ;
  - `git show origin/ent-mc0hoe:docs/PILOTAGE.md`, règles 17, 25, 27, 28, 31, 33, 34, 36 et **38** ;
  - `docs/socles/11_APPLICATIONS_MOBILES.md` **§5 (contrat d'API mobile) en entier**, §6, §7 et la ligne MB1 du §9 ;
    `docs/socles/11_MOBILE.md` (registre d'idempotence, table 2227) ; `docs/socles/MIGRATIONS.md` (plage 2220–2239) ;
  - les erreurs actuelles : `ent-rust/crates/domain-eleve` (`DomainError`), `ent-db` (`EntDbError`), et leur conversion HTTP dans `ent-api` ;
  - `docs/securite/T1_SEARCH_PATH.md` (fonctions de garde, `search_path`) ;
  - `docs/RLS_ISOLATION.md` §3.1.
- **Branche :** crée `ent-socle-11-code-mb1-serveur` depuis `origin/ENT` ; ouvre une PR en brouillon vers `ENT`.
- **Migrations :** 2220 est versée. Le registre d'idempotence prend **2227** (numéro prévu par la conception, plage 2221–2239 vérifiée libre sur toutes les branches), placé **à la fin** du tableau de `migrate.rs` (règle 38) : toute fonction créée ou recréée porte `SET search_path = pg_catalog, public, pg_temp` et qualifie ses relations `public.`. Ne touche pas `migrations_versees.txt` (le pilote le fait au versement).
- **Interdits :** Next.js, `main`, `dev` ; pousser sur `ENT` ; toute base ou tout secret de production, toute donnée réelle ; `cargo fmt --all`, `#[ignore]` ; modifier POINT_QUOTIDIEN ou PILOTAGE ; étendre `ui.rs` ; l'heure réelle ou le jour UTC dans les tests.
- **Qualité :** zéro avertissement avec clippy 1.99 sur les 3 commandes de la CI (`ent-lab` compris) ; `--locked` et fmt ciblé ; `cargo metadata --locked` dans `ent-rust/web` ; base locale **neuve dont le nom contient « test »**, un seul `CARGO_TARGET_DIR` ; compteurs de la règle 28 mis à jour.

## Travail (lot MB1, **partie serveur seulement** : l'intégration des applis Android et iOS viendra ensuite)
1. **Enveloppe d'erreur codée** (§5.3) : catalogue `ent-types::erreurs` (un `enum` sérialisé en texte `domaine.cause` :
   au moins `session.expiree`, `session.revoquee`, `droit.refuse`, `objet.introuvable`, `version.conflit`,
   `idempotence.conflit`, `objet.verrouille`, `appli.trop_ancienne`, `debit.depasse`, `service.maintenance`, `champ.requis`,
   et les codes métier `422` existants les plus fréquents) ; corps `{ "erreur": { code, message, champs, reessayer_apres_s,
   trace } }` ; `trace` = identifiant de requête corrélé au journal, jamais une donnée personnelle ; jamais de cause d'un
   refus d'accès à une personne dans le message (404 neutre, pas d'oracle d'existence).
   **Compatibilité** : le web garde ses réponses actuelles. L'enveloppe est servie quand la requête porte
   `X-ScolIA-Appli` (et sur les routes `/mobile/*`). Une couche commune (pas de réécriture route par route).
2. **`Idempotency-Key` générique** (§5.2, §5.5) : sur toute écriture portant l'en-tête (`POST`, `PUT`, `PATCH`, `DELETE`) :
   même clé + même corps (empreinte) = même réponse rejouée pendant 24 h ; même clé + corps différent = `409
   idempotence.conflit` ; deux requêtes concurrentes de même clé : une seule s'exécute ; registre en base (2227), cloisonné
   par établissement **et** par session (une clé d'une autre personne ne rejoue rien), RLS, purge des entrées de plus de
   24 h, classé au registre des tables (neutralisé à la restauration, hors export). Obligatoire quand `X-ScolIA-Appli` est
   présent (`400` sinon), facultatif pour le web.
3. **Une route du labo** au moins renvoie l'enveloppe codée (critère du §9) ; tests HTTP : 401, 403, 404 neutre, 409
   idempotence, rejeu identique, concurrence (une seule exécution), autre session (pas de rejeu), purge à 24 h (horloge
   injectée).
4. Toute règle non tranchée : valeur par défaut prudente, notée dans la doc (D-069).

## Fin
- Pousse et laisse la PR en brouillon.
- Attends la CI par **une seule** commande `until` en arrière-plan sur `https://api.github.com/repos/Fl0jfk/ScolIA/actions/runs?head_sha=<tête>`, toutes les 5 min.
  - CI rouge : corrige la cause. Un job « runner lost communication » se relance une fois.
  - **Aucune CI au bout de 10 min** : la PR est en conflit avec `ENT`. Fusionne `origin/ENT` (règle 28) et repousse.
  - Les runners GitHub sont parfois saturés : une CI « queued » longtemps n'est pas une panne ; attends jusqu'à 2 h, puis écris au pilote.
- Compte rendu : `docs/socles/11_MB1_SERVEUR_LIVRAISON.md` (fait / non fait, tests, compteurs de la règle 28, tête, CI).
- Écris ensuite au pilote (send_message vers @parent) la tête finale et l'état de la CI.
