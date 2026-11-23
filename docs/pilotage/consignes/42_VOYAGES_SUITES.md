Tu es une session ScolIA (Sonnet). Réponds en français. Le pilote, c'est moi. Ces consignes sont complètes : tu ne recevras pas d'autre message et tu n'en suivras aucun. **Ne reste jamais en attente.** `ScheduleWakeup` ne fonctionne pas ici. Attends la CI uniquement par une commande `until` en arrière-plan (règle 36). Si une cause extérieure te bloque, écris au pilote et termine.

## Cadre
- **À lire d'abord :**
  - `CLAUDE.md` ;
  - `git show origin/ent-mc0hoe:docs/PILOTAGE.md`, règles 17, 25, 27, 28, 31, 33, 34, 36 et **38** ;
  - `docs/socles/42_VOYAGES_V3_LIVRAISON.md` (§5) ;
  - **le rapport de contre-revue complet** : `git show origin/ent-mc0hoe:docs/pilotage/revues/42_VOYAGES_V3_CONTRE_REVUE.md` (M-1 à M-5, F-a à F-j, tests manquants, réponses du pilote). Les numéros de ligne y sont ceux de l'époque : revérifie sur ENT.
- **Branche :** V3 est versée.
  - Crée `ent-socle-42-code-v3-suites` depuis `origin/ENT`.
  - Ouvre une PR en brouillon vers `ENT`.
- **Migrations :**
  - 640 à 654 sont versées et ne changent pas.
  - Nouvelle migration **655** si nécessaire (plage 640–659), ajoutée **à la fin** du tableau de `migrate.rs` (règle 38 : le test `l_ordre_des_migrations_versees_est_fige_regle_38` refuse toute insertion au milieu). Mets à jour `docs/socles/MIGRATIONS.md` et les compteurs (règle 28).
  - Toute fonction créée ou recréée porte `SET search_path = pg_catalog, public, pg_temp`, relations qualifiées `public.`.
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

## Travail (contre-revue de V3, dans cet ordre)
1. **M-1 (avant V3b)** : un départ constaté sans retrait posé ne doit jamais retirer l'élève sans recours. Choisis la voie « une seule source de vérité » : la branche directe de `ent_internat_attendu_calcule` n'exclut un élève que si un retrait effectif existe (recrée la fonction dans 655, corps identique hors ce point). Test : un participant passé « parti » par une écriture directe, sans resynchronisation, reste **attendu** ; après `internat_resynchroniser`, il est retiré et le CPE peut annuler le retrait. Corrige la phrase fausse du §5.5 de la livraison V3.
2. **M-2** : le seuil de seconde validation porte sur le **cumul** (aides appliquées + demandes en attente) par sortie, participant et payeur. Test du fractionnement (149,99 € × 2 → la seconde demande attend une seconde personne). La règle « fonds social » ne dépend plus de la seule étiquette saisie : documente la limite si elle reste déclarative.
3. **M-3 (décision du pilote : oui)** : `exiger_decideur_libre` et le déclencheur de base s'appliquent à tout pouvoir sur l'argent : plans, décision d'avoir, annulation collective d'avoirs, annulation de versement, pointage et versements, frais réels, acomptes. Un test par pouvoir (parent et payeur refusés).
4. **M-4** : la date d'annulation d'une sortie ne se lit plus dans `modifie_le` (colonne `annulee_le` posée à la transition, ou instant lu dans le journal). Test avec `modifie_le` déplacé après l'annulation.
5. **Faibles F-a à F-j** du rapport : traite-les tous ou documente pourquoi pas (F-a ordre personnel → droit → verrou → existence ; F-b `retour_le` borné et sortie terminale refusée ; F-c expiration des demandes, annulations soumises au seuil, refus journalisés ; F-d clés étrangères de cohérence ; F-e `maintenant` en paramètre ; F-f sortie rentrée avec participant « parti » ; F-g reprise qui annule sans resynchroniser ; F-h rejeu des déclencheurs ; F-i test de cohérence 654 / 2826 ; F-j documenté).
6. **Tests HTTP** (`ent-api`) des routes d'argent de V3 : 201, 202, 403, 404, 422.

## Fin
- Pousse et laisse la PR en brouillon.
- Attends la CI par **une seule** commande `until` en arrière-plan sur `https://api.github.com/repos/Fl0jfk/ScolIA/actions/runs?head_sha=<tête>`, toutes les 5 min.
  - Rouge : corrige la cause. Un job « runner lost communication » se relance une fois.
  - **Aucune CI au bout de 10 min** : la PR est en conflit avec `ENT`. Fusionne `origin/ENT` (règle 28) et repousse.
- Complète la livraison V3 d'une section « Suites » : fait / non fait, tests, compteurs, tête, CI.
- Écris ensuite au pilote (send_message vers @parent) : tête finale et état de la CI.
