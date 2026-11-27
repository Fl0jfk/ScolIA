# Contre-revue des suites voyages V3b (PR 145, tête 6c547800) — 2026-11-24

Relecteur `review-sonnet` sur worktree, lecture seule. **Aucun Élevé.** Contournement tracé contrôlé en base pour l'essentiel
(seconde personne distincte et libre via `ent_sortie_argent_decideur_interdit`, deux comptes d'un même foyer refusés, usage
unique, trace en ajout seul, consommation dans la transaction de l'écriture). `rentree_le` correct (nuits passées conservées,
à venir rétablies, retour d'état testé). Garde de 651 ni modifiée ni affaiblie. Règle 38 conforme (aucun SECURITY DEFINER).

**Décision du pilote : la PR 145 n'est pas versée avant correction des Moyens 1 à 3** (le contournement est un mécanisme
neuf sur l'argent des familles : pas d'autorisation « en blanc »).

## Moyens (bloquants pour le versement)
1. **Autorisation « en blanc »** : la demande n'est liée ni à un acompte précis ni à un montant ni à un prestataire ; la
   seconde personne autorise sans savoir quoi. → ajouter `cible_id` (payer, annuler), `montant_max_centimes` et
   `prestataire_id` (créer, payer), les montrer à l'autorisatrice, les vérifier dans `ent_sortie_contournement_valide`.
2. **Contrôle en base partiel** : l'expiration (30 jours), l'habilitation et l'appartenance à l'établissement de
   l'autorisatrice et l'horodatage (`le` fourni par l'appelant) ne sont qu'applicatifs ; l'annulation collective n'a aucun
   garde de base. → exiger en base `now() <= autorise.le + 30 j`, l'autorisatrice membre de l'établissement avec la
   capacité requise, `le` posé par `DEFAULT now()` ou déclencheur ; garde de base pour l'annulation collective.
3. **Rattrapage sous RLS stricte** : l'énumération `SELECT DISTINCT etablissement_id FROM public.ent_sortie` est elle-même
   filtrée (aucune ligne sans tenant) → la migration échoue fermée sur une base peuplée. → énumérer comme 2220
   (`ent_etablissement UNION ent_membership`) et restaurer la valeur précédente du réglage.

## Faibles
1. Fenêtre d'ouverture comptée depuis la demande et non l'autorisation : deux autorisations vivantes possibles.
2. Droit de l'autorisatrice non revérifié à l'usage (révocation entre-temps).
3. 403 (`contournements_lister`) contre 404 (`acomptes`) pour le même cas : harmoniser.
4. Motif libre (500 caractères) : consigne « aucun nom » à l'écran, durée de conservation à documenter (RGPD).
5. Au-delà de 30 refus par heure, plus aucune trace : compteur agrégé ou plafond par (personne, action).
6. Rattrapage `rentree_le = modifie_le` : approximation à documenter.
7. `rentree_le` fourni par SQL direct accepté tel quel : borner par `now()` et une tolérance.
8. Contrainte d'auteur d'annulation `NOT VALID` : acceptable (documenter).
9. Règle 3 fondée sur `user_id`, pas sur la personne physique (antérieur) : en file, transverse.
- Compteur annoncé `Action::TOUTES` 271 : **282** attendu, à corriger dans la doc.
