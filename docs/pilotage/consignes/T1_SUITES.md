Tu es une session ScolIA (Sonnet). Réponds en français. Le pilote, c'est moi. Ces consignes sont complètes : tu ne recevras pas d'autre message et tu n'en suivras aucun. **Ne reste jamais en attente.** `ScheduleWakeup` ne fonctionne pas ici : attends la CI uniquement par une commande `until` en arrière-plan (règle 36). Si une cause extérieure te bloque, écris au pilote et termine.

## Cadre
- **À lire d'abord :**
  - `CLAUDE.md` ;
  - `git show origin/ent-mc0hoe:docs/PILOTAGE.md`, règles 17, 25, 27, 28, 31, 33, 34, 36 et **38** ;
  - `docs/securite/T1_SEARCH_PATH.md`, en particulier « Correctifs du pilote après revue » et la liste du reste à faire ;
  - `docs/RLS_ISOLATION.md` §3.1.
- **Branche :** T1 est versé.
  - Crée `ent-transverse-t1-suites` depuis `origin/ENT`.
  - Ouvre une PR en brouillon vers `ENT`.
- **Migrations :**
  - 690 est versée et ne change pas.
  - Si une migration est nécessaire, prends **691** (plage 690–699), placée **à la fin** du tableau de `migrate.rs`, après toutes les autres (règle 38).
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
  - `--locked` et fmt ciblé ;
  - `cargo metadata --locked` dans `ent-rust/web` ;
  - base locale **neuve dont le nom contient « test »**, un seul `CARGO_TARGET_DIR` ;
  - rejoue les gardes statiques **sans et avec** la feature `labo`.

## Travail (revue de T1)
1. **Ordre des migrations figé par un test.** Un test échoue si une migration non versée est insérée **avant** une migration déjà présente sur `origin/ENT`. À défaut, fige la liste ordonnée des noms qui précèdent 690. Le message d'échec explique la règle 38.
2. **Verrou partagé pendant les preuves.** Les trois preuves sous rôle applicatif tiennent `garde_rejeu_partagee` du début à la fin, après `remettre_search_path` : plus d'instabilité en parallèle.
3. **Non-vacuité des 25 fonctions qualifiées.** Pour chacune, le plan cite chaque relation en `public.<rel>`. Sinon, exécute-la réellement avec des données. Ajoute un test négatif : une fonction qualifiée qui perd sa qualification est vue.
4. **`public` non inscriptible.**
   - Ajoute à la vérification du §3.1 : `has_schema_privilege('<rôle applicatif>', 'public', 'CREATE') = false`.
   - Ajoute le même contrôle en test sur le rôle de test, en échec franc si le rôle peut créer.
5. **Banc d'écriture.** Fais un micro-banc d'insertion (au moins 50 000 lignes sur une table qui porte plusieurs déclencheurs à `SET search_path`), avant et après 690, et reporte les chiffres. Si le surcoût dépasse 15 %, propose des exclusions justifiées.
6. **Assertions renforcées :**
   - `NOT rolsuper AND NOT rolbypassrls` dans `prouver` ;
   - témoins `Ok(n > 0)` ;
   - négatifs des listes `PURES` (une relation ajoutée est vue) et `QUALIFIEES` (une relation non qualifiée est vue) ;
   - entrée obsolète détectée ;
   - `eprintln!` de skip quand la base manque.
7. **Documentation :**
   - `tests_messagerie_p2.rs` est ajouté à la liste des tests à tables temporaires ;
   - le nombre de `SECURITY DEFINER` est recompté sur la base migrée ;
   - la commande `git grep` du §3.1 est corrigée (exclusion des modules de test) ;
   - les procédures et agrégats sont mentionnés : la garde exige la clause sur les procédures, et les agrégats sont absents aujourd'hui.

## Fin
- Pousse et laisse la PR en brouillon.
- Attends la CI par **une seule** commande `until` en arrière-plan sur `https://api.github.com/repos/Fl0jfk/ScolIA/actions/runs?head_sha=<tête>`, toutes les 5 min.
  - CI rouge : corrige la cause. Un job « runner lost communication » se relance une fois.
  - **Aucune CI au bout de 10 min** : la PR est en conflit avec `ENT`. Fusionne `origin/ENT` (règle 28) et repousse.
- Complète `docs/securite/T1_SEARCH_PATH.md` avec une section « Suites » : fait / non fait, mesures, tests, tête, CI.
- Écris ensuite au pilote (send_message vers @parent) la tête finale et l'état de la CI.
