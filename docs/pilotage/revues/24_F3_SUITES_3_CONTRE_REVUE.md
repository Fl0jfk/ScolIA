# Contre-revue — F3 suites 3 (PR 150, tête 4f86939a) — 2026-11-24

Relecteur `review-sonnet`, lecture seule, rien exécuté. **Verdict : après correctifs (2 élevés).** PR **retenue**.

## Élevés
- **E1, prolongation d'une prise en charge : mois jamais facturés, en silence.** Le premier tronçon garde la clé de base
  `annuel:{annee}:{eleve}:{article}:{foyer}` quelle que soit sa durée (`finances.rs:1332-1343`, `:1426`). Prise 01/09–31/12
  (restauration 450 €/an) : 1re génération Sep–Déc 18 000 c. Réactivation de la même prise avec `au = 30/06`
  (`facturation_comptes.rs:910`) : au rejeu, un seul tronçon de 10 mois sous la clé de base ; la clé existe, la garde ne
  regarde que les clés nouvelles (`:1622-1625`), `ON CONFLICT DO NOTHING` (`:550`) ignore la ligne : **27 000 c jamais
  facturés**, ni erreur ni alerte.
- **E2, renouvellement d'une prise (nouvelle prise Jan–Juin) : facturation refusée à tort** (« le tarif a changé ») : clé
  nouvelle `:m2031-01` refusée par la garde alors que la période est disjointe ; la tâche « part non couverte » reste ouverte
  sans issue.
- **Correctif attendu (E1 + E2)** : la garde se fonde sur le **chevauchement des périodes** (`periode_debut`/`periode_fin`)
  et le **montant** des lignes déjà posées pour (élève, article, foyer), y compris réductions et `:suivant:` : période
  disjointe → facturable ; même clé mais période ou montant différent → facturer le complément (mois manquants) ou refuser
  avec un motif explicite, jamais ignorer. Tests : prolongation puis rejeu, renouvellement, rejeu sans modification.

## Moyens
- **M1** : prise `echue` ignorée même pour les mois où elle était valide (`finances.rs:981-984` filtre `statut='active'`) :
  filtrer par période, n'exclure que `annulee`.
- **M2** : la tâche `regularisation_regime_non_facturee` est close par n'importe quel changement de régime réussi de l'élève
  (`finances.rs:1852`) : lier la tâche à l'événement ; test A-puis-B.
- **M3** : remise arrondie au centime inférieur → la famille peut payer jusqu'à 1 c de trop par tronçon réduit ; doc et
  commentaire « jamais un centime en trop » faux pour les remises : arrondir la remise vers le haut (en faveur de la famille)
  ou corriger doc/commentaire/test.
- **M4** : test de cloisonnement SQL non discriminant (`tests_finances_f3.rs:4856-4867`, `let _ = autre;`) : asserter 0
  sous le rôle applicatif strict.

## Faibles
1. `retire_a_partir_du` sans année : avertir à la proposition d'une grille qui tarifie un article retiré.
2. Tests de fin de tarif : retrait au 01/03 avec trou dès janvier (bloquant), tous les articles retirés.
3. Prise commençant le 15 du mois : mois non couvert (règle à confirmer, D-069 : prudente).
4. Plafond de prise consommé au rejeu : ligne `:suivant:` refusée (rejeu stérile) ; à vérifier en multi-tronçons.
5. Détail de tâche figé au premier motif (assumé).

## Conforme
Couverture mois par mois nominale, aucun chevauchement ; 532 en fin de tableau, contrainte de nature recréée avec les 9
natures existantes + 1, RLS de 522 inchangée, idempotence par index unique partiel ; fin de tarif datée correcte ;
409 limité à la première publication ; `.is_ok()` remplacé ; segmentation (article, foyer) correcte.
