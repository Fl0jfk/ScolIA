# Plan de correction des avertissements Clippy du backend

Date : 2026-11-03 — Statut : **plan approuvé dans son principe ; aucune correction faite**.
Source : `cargo clippy --locked --workspace --all-targets` (Rust 1.99.0) sur `ent-rust/`,
liste dédupliquée dans `_outils-scolia/clippy-backend-liste.tsv` (hors dépôt).

**Total : 60 avertissements distincts**, 0 erreur. (Le chiffre « ≈ 65 » annoncé auparavant comptait
deux fois certains avertissements des tests de `domain-eleve`.)

| Crate | Nombre |
|---|---|
| `ent-db` | 31 |
| `domain-eleve` | 14 (12 dans des tests) |
| `ent-api` | 9 |
| `ent-lab` (outil de labo) | 5 |
| `ent-mail` | 1 |

## Règles du chantier

- **Jamais les 60 d'un coup** : un lot = une catégorie ou une crate, un commit, CI verte.
- Pour chaque lot : relire chaque site, corriger, `cargo test --workspace`, Clippy relancé
  (le compteur doit baisser exactement du nombre traité).
- Pas de `#[allow(...)]` global. Un `allow` local n'est admis qu'avec un commentaire qui explique
  pourquoi le code est voulu.
- Les lots qui touchent un **contrôle d'accès** ou un **calcul d'argent** demandent d'abord un test
  qui fige le comportement actuel (refus et acceptations), puis la correction.
- Quand le compteur d'une crate atteint 0, la CI passe cette crate en `-D warnings` (bloquant).

## 1. Sécurité et erreurs potentielles — priorité 1

Aucun bogue avéré. Ces sites demandent pourtant un examen humain, parce qu'ils portent sur
l'authentification, les droits ou des montants.

| Site | Lint | Constat | Action proposée |
|---|---|---|---|
| `ent-api/src/routes.rs:8742` (`map_auth`) | `if_same_then_else` | Deux branches identiques (400) : sans effet aujourd'hui. Mais tout le mapping erreur d'auth → code HTTP repose sur **le texte des messages** (`contains("déjà")`, `"refusé"`…). Reformuler un message change silencieusement un 401 en 403 ou 400. | Lot 1a : fusionner les deux branches (comportement identique) **et** ajouter un test par cas (429, 409, 400, 403, 401). La vraie correction (erreurs typées au lieu du texte) est une décision à part, liée au point `map_db` déjà connu. |
| `ent-db/src/service.rs:10910` | `collapsible_if` | Contrôle d'accès : un parent ou un élève ne voit pas un champ non visible pour les familles. | Lot 1b : test de refus (parent, élève) et d'acceptation (personnel) **avant** de fusionner les `if`. |
| `ent-db/src/service.rs:12756` | `collapsible_if` | Contrôle d'accès : un parent doit avoir un lien avec l'élève. | Lot 1b : même méthode. |
| `ent-db/src/service.rs:5372` | `match_like_matches_macro` | Décide quels créneaux un professeur voit le jour d'un remplacement (accès à l'appel). | Lot 1b : test « titulaire remplacé ne voit plus le créneau / remplaçant le voit », puis `matches!`. |
| `ent-db/src/seed.rs:1493` | `inconsistent_digit_grouping` | `850_50` centimes : **valeur vérifiée correcte** (85 050 centimes = 850,50 €), groupement « euros_centimes » voulu. Données de labo uniquement. | Lot 1c : écrire `85_050` avec le commentaire `// 850,50 €`, pour éviter qu'un lecteur se trompe d'un facteur 100. |
| `ent-mail/src/config.rs:148` | `if_same_then_else` | Gmail et Scaleway TEM renvoient tous deux le port 587 (STARTTLS) : voulu. | Lot 1c : réunir les deux conditions avec un commentaire. |

## 2. Robustesse — priorité 2

| Site | Lint | Action |
|---|---|---|
| `ent-db/src/service.rs:15283` (rapprochement bancaire) | `field_reassign_with_default` | Initialiser `lines` dans la construction : un champ ajouté plus tard ne sera plus oublié. |
| `domain-eleve/src/info_comp.rs:142` | `collapsible_match` | Validation des dates des informations complémentaires : intégrer la garde au `match`. Les tests existants couvrent ce cas, à vérifier avant. |
| `domain-eleve/src/salle.rs:43` | `manual_range_contains` | `!(0..=500).contains(&c)` : bornes plus lisibles. Test de bornes 0, 500 et 501. |

## 3. Performance — priorité 3 (impact négligeable)

| Sites | Lint | Remarque |
|---|---|---|
| `ent-db/src/seed.rs` (10), `continuum_repo.rs:202` (1) | `manual_contains` | `iter().any(|a| *a == x)` → `contains(&x)`. Le seul site hors seed est `continuum_repo.rs` (petite liste d'années). |
| `domain-eleve/src/repas.rs` (10), `facturation_sepa.rs:643` (1) | `cloned_ref_to_slice_refs` | **Tous dans des tests** : `&[x.clone()]` → `std::slice::from_ref(&x)`. |
| `ent-lab/src/main.rs` (3) | `useless_conversion` | `String` converti en `String`. Outil de labo. |
| `ent-api/src/routes.rs:3469` | `needless_borrow` | Référence de référence. |

## 4. Maintenabilité — priorité 4

| Sites | Lint | Remarque |
|---|---|---|
| 14 fonctions (`ent-db/src/service.rs` ×11, `edt_repo.rs:184`, `seed.rs:843`, `domain-eleve/src/sortie.rs:454`) | `too_many_arguments` | 8 paramètres ou plus. Regrouper en structures de commande (`…Cmd`), **au fil des modules migrés**, jamais en bloc : chaque changement de signature touche les appelants. |
| `ent-db/src/auth_repo.rs:930`, `:968` | `type_complexity` | Tuple de 5 colonnes SQL : un `#[derive(sqlx::FromRow)]` nommé. Code d'authentification : relu, avec tests. |

## 5. Style — priorité 5

| Sites | Lint |
|---|---|
| `ent-api/src/routes.rs` ×6 (5124, 5545, 5574, 5771, 5800, 5836) | `needless_question_mark` (`Ok(x?)` → `x`) |
| `ent-lab/src/main.rs:4147`, `:4354` | `useless_borrows_in_formatting` |
| `ent-api/src/routes.rs:1384` | `collapsible_str_replace` (`replace(['/', '\\'], "_")`). Le nom de fichier est de toute façon réassaini dans `attachment_response_dyn`. |

## Ordre proposé des lots

| Lot | Contenu | Nombre | Prérequis |
|---|---|---|---|
| 1a | `map_auth` + tests de statut | 1 | — |
| 1b | contrôles d'accès (service.rs 10910, 12756, 5372) | 3 | tests de refus d'abord |
| 1c | montant de seed, port SMTP | 2 | — |
| 2 | robustesse | 3 | — |
| 3 | performance (tests, seed, labo) | 26 | — |
| 4a | `type_complexity` (auth_repo) | 2 | relecture |
| 5 | style | 9 | après le commit de formatage, pour éviter les conflits |
| 4b | `too_many_arguments` | 14 | au fil de la migration des modules |

Après les lots 1 à 3 et 5 : `ent-lab`, `ent-mail` et `domain-eleve` sont à 0 et peuvent passer en
bloquant. `ent-db` et `ent-api` attendent le lot 4b.
