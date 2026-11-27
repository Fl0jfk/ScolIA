# Socle 11 — palier « notes eva » : saisie des notes sur mobile par la grille `/eva/*`

Branche `ent-socle-11-palier-notes-eva`, partie de la tête du palier MB2 (`ent-socle-11-palier-mb2`) et à rebaser sur `ENT` après le versement de MB2. Règle 39 de `PILOTAGE.md` : lots sans PR, contre-revue par lot, une seule PR et une seule CI à la fin. Origine : D-069 n° 1 de `11_MB2_LIVRAISON.md`, où la saisie mobile a été retirée parce qu'elle appelait `POST /eleves/{id}/notes`, une route fermée (410, V0 D1).

**Aucune migration, aucun `Cargo.toml` ni `Cargo.lock` modifié, aucun changement de route serveur.** Le palier ne touche pas au code de production Rust : seulement aux tests (`tests_contrat_mobile.rs`, test de vecteurs dans `ent-types`) et aux applis.

## 1. Conception

### Besoin
Un professeur veut saisir les notes d'un contrôle **depuis son téléphone**, en classe, en salle des profs ou dans le train : choisir le cours, créer l'évaluation, taper les notes et les codes (absent, dispensé, non rendu…), puis publier aux familles quand il est prêt. Il redoute trois choses : écraser la saisie d'un collègue (co-enseignement, suppléance), perdre une saisie sur un réseau faible, et publier par erreur. Le reste de la grille (modification du barème avec motif, annulation, import, historique, corrections après l'arrêt de la période) demande un grand écran et reste sur le web.

### Routes (toutes existantes, socle 21, lot V3)
| Route | Usage mobile |
|---|---|
| `GET /eva/enseignements` | cours du compte ; seuls ceux en `droit = saisie` sont proposés |
| `GET /eva/cours/{id}/periodes` | périodes ; celle du jour s'ouvre d'elle-même, une seule fois par cours |
| `GET /eva/grille` | évaluations, élèves, cases avec version, `peut_saisir`, `saisie_ouverte`, statistiques du moteur |
| `POST /eva/evaluations` | création : titre, date, barème, coefficient (le reste garde ses valeurs par défaut, publication explicite) |
| `POST /eva/saisie` | lot de cases, chacune avec la version lue ; une issue par case |
| `POST /eva/evaluations/{id}/publication` | `maintenant` (confirmation obligatoire) ou `retirer` (motif écrit, dans les 24 h) |

Restent sur le web : `PUT /eva/evaluations/{id}`, `annuler`, `en-cours-de-correction`, `historique`, `explication`, `sous-matieres`, `import/*`, la publication programmée et les corrections motivées après l'arrêt de la période.

### Règles
- **Le serveur reste juge.** L'appli copie seulement l'analyse du texte d'une case (`analyser_cellule`) pour l'afficher tout de suite. Kotlin et Swift rejouent `contrat/mobile/regles/analyse_cellule.json` au message près : 348 vecteurs écrits par le test Rust `contrat_mobile_analyse_cellule`. Aucune moyenne n'est calculée côté appli.
- **Pas d'écrasement.** Chaque case part avec la version lue. Si un autre auteur a écrit entre-temps, le serveur répond `conflit`. L'écran montre alors sa valeur et son auteur, et la case ne repart qu'après un choix explicite : « Garder la sienne » ou « Garder la mienne ». Un conflit sans état actuel bloque la case jusqu'à ce que la grille soit relue.
- **Seules les cases modifiées partent**, si elles sont comprises (note ou code) et ouvertes. Une case vidée n'efface rien (R-V6), et l'écran le dit. Une case dont la période est arrêtée est en lecture, avec un renvoi vers le web.
- **Réseau faible.** La clé d'intention est le contenu exact du lot : élève, texte et version, pas un hachage (D-069). Après un échec incertain, « Enregistrer » renvoie la même clé, et le serveur rejoue la première réponse. Une frappe faite pendant l'envoi est gardée.
- **Publication** : la confirmation dit que les familles verront les notes. Elle n'est possible que sans saisie en attente. Retirer exige un motif.
- **Module** : « Saisir des notes » apparaît pour le personnel qui a `notes.saisir` (`Capacites`). Le serveur ne rend que les cours du compte et refuse tout le reste.

## 2. Lots
| Lot | Contenu | Où |
|---|---|---|
| 1 (contrat) | Six routes dans `routes.txt` et le banc MB2, plus 9 fixtures (`eva_*`, dont un lot `inchange` et `refuse`, un conflit et la grille relue). Refus exacts (403 ou 404 codés) en lecture **et en écriture** pour un collègue du même établissement, un parent, un élève et un autre établissement. Vecteurs d'analyse partagés | `ent-api/src/tests_contrat_mobile.rs`, `ent-types/src/evaluations_v3.rs`, `ent-rust/contrat/mobile/` |
| 2 (cœurs) | Modèles `/eva`, méthodes du client, `SaisieNotes` (analyse et format), `BrouillonSaisie` (lot, issues, conflits, intention), module `SAISIE_NOTES` / `.saisieNotes` | `Evaluations.kt`, `Evaluations.swift`, `ApiClient.*`, `Capacites.*` |
| 3 (écrans) | Cours, période, liste des évaluations, création, saisie, conflit, publication. Test Robolectric de bout en bout (hors barème refusé localement, enregistrement, publication confirmée) | `ecrans/SaisieNotes.kt`, `ScolIAUI/SaisieNotes.swift` |

## 3. Règles non tranchées → valeur par défaut prudente (D-069)
1. **Saisie limitée aux cases ouvertes** : une correction après l'arrêt de la période (motif, `notes.corriger`) reste sur le web.
2. **Évaluation créée « explicite »** : jamais publiée automatiquement depuis le mobile ; la publication programmée reste sur le web.
3. **Module sur `notes.saisir`** : la direction sans ce droit ne voit pas l'écran (lecture des grilles sur le web).

## 4. Non fait (et pourquoi)
- `:app` Android et `ScolIAUI` ne sont pas compilés en local, faute de SDK Android et de Mac : `:app` est compilé et testé par la CI, et SwiftUI reste à passer sur un Mac.
- Il n'y a pas de saisie hors ligne différée : une saisie partie sans réseau doit être renvoyée par l'utilisateur, avec la même clé. Le brouillon tient en mémoire tant que l'écran reste ouvert. Les opérations durables viendront avec MB7.
- Commentaires de case, sous-matières, demi-groupes (`restreinte_a`), compétences : web.

## 5. Tests (local, 2026-11-27)
- Rust : banc MB2 avec `/eva` (fixtures, parité, refus d'écriture), vecteurs d'analyse : verts.
- Cœur Kotlin et cœur Swift : voir la PR (nombres à jour dans la livraison).

## 6. Contre-revues
- Lots 1 et 2 : aucun élevé confirmé. **E1** (saisie hors droit non prouvée) **corrigé** : refus exacts en écriture, dont un collègue du même établissement. **M1** (refus `is_client_error`) **corrigé** : 403 ou 404 exigés partout. **M2** (personas absents ignorés) **corrigé** pour `/eva`. **M3** (frappe pendant l'envoi) **corrigé**. **M4** (conflit sans état) **corrigé**. **M5** et **M6** (grille remplie, issues `inchange` et `refuse`) **corrigés**. **M7** (vecteurs) **corrigé**, de 156 à 348 vecteurs. **M8** (scalaires Swift, grille dégénérée) **corrigé**. **F3** (intention sur le lot envoyé) **corrigé**. F1, F2 et F4 à F8 sont sans impact ou conformes.
