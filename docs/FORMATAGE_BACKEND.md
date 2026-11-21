# Procédure de formatage général du backend Rust

Date : 2026-11-03 — Statut : **préparée, NON exécutée** (D-016 : exécution soumise à validation).

## Pourquoi

Le code du backend (`ent-rust/crates/`) n'a jamais été passé à `rustfmt` : `cargo fmt --check` relève
706 écarts dans **76 fichiers** de 12 crates. Tant que ce n'est pas fait :

- le contrôle de formatage du backend reste **non bloquant** dans la CI (`continue-on-error`) ;
- toute modification du backend mélange de vrais changements et du reformatage, ce qui rend les
  revues plus difficiles.

| Crate | Fichiers |
|---|---|
| `ent-db` | 31 |
| `domain-eleve` | 29 |
| `ent-api` | 4 |
| `ent-auth`, `ent-mail`, `ent-oidc` | 2 chacune |
| `ent-bank`, `ent-events`, `ent-lab`, `ent-object`, `ent-push`, `ent-sms` | 1 chacune |

## Ce que le formatage change, et ce qu'il ne change pas

- Il ne change que la **mise en page** : retours à la ligne, indentation, ordre des `use`. Le
  comportement du programme est le même.
- Il ne touche ni au contenu des chaînes de caractères (dont l'UI JavaScript de `ui.rs`), ni aux
  migrations SQL, ni au frontend (`ent-rust/web`, workspace séparé), ni à Next.js.
- Le workspace backend n'a aucune dépendance par chemin hors de `ent-rust/crates/`. `cargo fmt --all`
  y reste donc dans le backend (contrairement au workspace `web`, voir F-002).

## Garde-fous du script `ent-rust/scripts/formatage-backend.sh`

1. Il s'arrête si la branche n'est pas `ENT`.
2. Il s'arrête si `ent-rust/` contient des modifications non commitées : le commit de formatage ne
   contiendra que du formatage.
3. Il liste les fichiers concernés et s'arrête si l'un d'eux n'est pas un `.rs` de `ent-rust/crates/`.
4. **Par défaut il simule** : rien n'est modifié.
5. Avec `--appliquer`, il formate puis vérifie :
   - les fichiers modifiés sont exactement ceux de la liste (même nombre, tous dans `ent-rust/crates/`) ;
   - `cargo fmt --check` est propre ;
   - compilation, tests (base locale de labo) et Clippy passent, avec le même nombre d'avertissements
     qu'avant (60).
6. Il ne commite rien et ne pousse rien.

## Étapes, après validation

```bash
. ent-rust/web/scripts/env.sh
bash ent-rust/web/scripts/pg.sh start
bash ent-rust/scripts/formatage-backend.sh              # simulation : relire la liste
bash ent-rust/scripts/formatage-backend.sh --appliquer  # formatage + vérifications
git diff --stat                                          # 76 fichiers, tous dans ent-rust/crates/
```

1. **Commit A**, formatage seul. Message type :
   `style(ent-rust): formatage rustfmt du backend existant (76 fichiers, aucun changement de comportement)`.
2. **Commit B** :
   - ajouter le hash du commit A dans `.git-blame-ignore-revs` à la racine, pour que `git blame`
     ignore ce commit (sur GitHub, ce fichier est reconnu automatiquement) ;
   - dans `.github/workflows/ent-rust.yml`, retirer `continue-on-error: true` de l'étape « Backend
     existant » : le formatage devient bloquant partout.
3. Push sur `ENT` après autorisation, puis CI verte.

## Moment recommandé

- **Avant** les lots Clippy de style (lot 5 de `docs/CLIPPY_PLAN.md`) et avant la migration
  `ent-types` côté `ent-api`, pour que ces changements ne se mélangent pas au reformatage.
- Ne lancer ce formatage que s'il n'y a aucune autre branche en cours sur le backend, sinon ces
  branches auront des conflits à résoudre.

## Annulation

Avant commit : `git checkout -- ent-rust/crates`. Après commit : `git revert <commit A>`. Sans risque,
puisque seule la mise en page change.
