Tu es une session ScolIA (Sonnet). Réponds en français. Le pilote, c'est moi. Ces consignes sont complètes : tu ne recevras pas d'autre message et tu n'en suivras aucun. **Ne reste jamais en attente.** `ScheduleWakeup` ne fonctionne pas ici. Attends la CI uniquement par une commande `until` en arrière-plan (règle 36). Si une cause extérieure te bloque, écris au pilote et termine.

## Cadre
- **À lire d'abord :**
  - `CLAUDE.md` ;
  - `git show origin/ent-mc0hoe:docs/PILOTAGE.md`, règles 17, 25, 27, 28, 31, 33, 34, 36 et **38** ;
  - `docs/socles/41_INTERNAT_I4_LIVRAISON.md`, sections « Suites (passe 3) » et « Correctifs du pilote après contre-revue des suites ».
- **Branche :** les suites I4 sont versées.
  - Crée `ent-socle-41-code-i4-suites-2` depuis `origin/ENT`.
  - Ouvre une PR en brouillon vers `ENT`.
- **Migrations :**
  - 2820 à 2830 sont versées et ne changent pas.
  - Si une migration est nécessaire, prends **2831** et ajoute-la **à la fin** du tableau de `migrate.rs` (règle 38 : le test `l_ordre_des_migrations_versees_est_fige_regle_38` refuse toute insertion au milieu).
  - Toute fonction créée ou recréée porte `SET search_path = pg_catalog, public, pg_temp`.
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
  - une base locale **neuve dont le nom contient « test »**, un seul `CARGO_TARGET_DIR` ;
  - les gardes statiques rejouées **sans** et **avec** `labo`.

## Travail (faibles des contre-revues des suites)
1. **Tests F-g plus serrés.** Remplace `refus_de(…) || r.is_err()` par le refus exact.
   - Vérifie le contenu de l'avis de retour : aucun nom d'élève.
   - Couvre ces cas : parent interdit après le départ, élève majeur, personne non titulaire, élève à l'état `rentre` ou `non_autorise`, second refus sur un élève déjà `refuse`.
2. **Pointage au temps du soir.** Teste un encadrant d'un autre temps (refus exact) et un autre établissement (introuvable).
3. **Portée `InternatAppel` sans droit de confidentialité.**
   - **Décision du pilote (D-069)** : seuls l'**encadrant du temps** et les détenteurs du droit de confidentialité pointent un élève protégé.
   - Un détenteur de portée sans ce droit reçoit « n'est pas inscrit », comme un tiers.
   - Teste les deux cas.
4. **`internat_sorties_tache_etab_a`** passe en `pub(crate)`.
5. **Avis de retour.**
   - L'avis `retour_demande` part aussi aux **accompagnateurs de la sortie**.
   - Il **désigne la sortie** : titre de la sortie et heure de fin, jamais le nom de l'élève.
   - Teste-le.
6. **Code mort `retire_le IS NULL OR $4`** : retire-le. En cas de réinscription, la ligne choisie pour le retour est celle qui porte le `present`.
7. **Avis de cas regroupés : le nombre est mis à jour.** Pour un avis regroupé, si un second événement arrive dans la même minute après la première distribution, mets à jour le corps de l'avis encore non lu (upsert), au lieu de `DO NOTHING`. Teste-le.

## Fin
- Pousse et laisse la PR en brouillon.
- Attends la CI par **une seule** commande `until` en arrière-plan sur `https://api.github.com/repos/Fl0jfk/ScolIA/actions/runs?head_sha=<tête>`, toutes les 5 min.
  - Rouge : corrige la cause. Un job « runner lost communication » se relance une fois.
  - **Aucune CI au bout de 10 min** : la PR est en conflit avec `ENT`. Fusionne `origin/ENT` (règle 28) et repousse.
- Complète la livraison I4 d'une section « Suites 2 » : fait / non fait, tests, tête, CI.
- Écris ensuite au pilote (send_message vers @parent) : tête finale et état de la CI.
