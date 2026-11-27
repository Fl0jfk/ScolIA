# Contre-revue des suites F3 (PR 139, tête 42f14c32) — 2026-11-24

Relecteur `review-sonnet` sur worktree, lecture seule. M-B, M-C (test réellement concurrent, sans sommeil), M-D corrigés avec
tests discriminants ; M-A corrigé sur le point demandé (tarif mois par mois) mais avec les défauts ci-dessous. Règle 38
respectée (3 fonctions de déclencheur, `search_path`, `public.`, aucun SECURITY DEFINER, 531 en fin de tableau, rejeu
idempotent). Aucune assertion affaiblie (`tests_rls` : commentaire seul ; `f3r_m2` durci).

## Élevé
1. **Double facturation à la régénération** (`finances.rs`, `lignes_annuelles` + `inserer_lignes`) : une génération faite
   avec v1 seule (une ligne, clé de base) puis rejouée après la mise en vigueur de v2 crée le segment `:m<AAAA-MM>` en plus :
   janvier-juin facturés deux fois (1 980 € au lieu de 1 260 €). Le test « rejouer ne double rien » ne rejouait rien.
   → **Corrigé par le pilote (D-069)** : si un article de l'élève est déjà facturé pour l'année et que le découpage
   d'aujourd'hui produit une clé nouvelle, **rien n'est écrit** et l'anomalie « le tarif a changé depuis la génération
   précédente … complément ou avoir » est rendue. Test ajouté dans `f3r_m1` (élève facturé avant v2, régénéré après : refus,
   total inchangé) et vrai rejeu (même génération deux fois : 2 lignes, 1 260 €).

## Moyens
1. **Mois sans tarif applicable facturés zéro en silence** (première grille au 01/10 sans historique).
   → **Corrigé par le pilote** : un segment sans aucun tarif applicable rend l'anomalie « aucun tarif applicable à partir du … ».
   Limite : un trou propre à **un** article (autres articles tarifés) n'est pas détecté — en file, avec test dédié.
2. Test « contributeurs » non discriminant au retour en brouillon (la direction était déjà contributrice) : en file.
3. Doc : `Action::TOUTES` vaut **282** (et non 269) ; « rejeu sans doublon » n'était pas prouvé. → doc corrigée par le pilote.

## Faibles (en file)
a. Alerte du second regard : repli sur l'e-mail, servie à tout `FacturationLire` ; date « rentrée 2027 » en dur.
b. Publication d'une version close supplantée (v1 après v3) encore possible : restreindre à la plus récente applicable.
c. Arrondis par segment (±1 centime) et lignes coupées pour des articles inchangés : comparer les tarifs par contenu.
d. Une requête par mois et par élève : cache par jour distinct de vigueur.
e. Commentaire de l'API « année future déjà votée » inexact (seules `en_vigueur` et `close`).
f. Test « les simulations ne s'effacent pas » vert pour une autre raison (FK) : préciser l'erreur attendue.
