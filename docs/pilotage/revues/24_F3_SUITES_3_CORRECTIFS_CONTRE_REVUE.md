# Contre-revue — correctifs F3 suites 3 (PR 150, tête a4fe5bec) — 2026-11-25

Relecteur `review-sonnet`, lecture seule. **Les deux élevés sont fermés**, scénarios chiffrés :
- E1 (prolongation) : 18 000 c puis tronçon de 10 mois ; les 6 mois manquants posés une fois sous `:c2031-01:{foyer}` (27 000 c) ; rejeu et double rejeu sans ligne nouvelle.
- E2 (renouvellement) : `:m2031-01` posé, 18 000 + 27 000 = 45 000 c ; rejeu inchangé. Tarif réellement changé sur des mois facturés : refus explicite.
- M1, M2, M3, M4 de la revue précédente fermés ; faibles 1, 2, 4 traités ; 532 modifiée en place sans risque (non versée).

## Moyens
1. **Payeur suivant désigné après la 1re génération** : `restant_prise` compte aussi le principal déjà posé ; avec plafond (40 000 c sur 48 000 c), les 8 000 c non couverts ne sont jamais facturés au suivant désigné ensuite, sans alerte. → **En file (F3 suites 4, prioritaire)**.
2. **Ligne ancienne sans période** : une clé nouvelle était jugée « disjointe » face à elle (double facturation possible). → **Corrigé par le pilote** : refus explicite (« déjà facturé sans période ») ; test `f3s4_pilote_ligne_ancienne_sans_periode_jamais_doublee`.
3. **Tâche `part_non_couverte` jamais close** après la facturation des mois manquants (E1/E2). → En file (F3 suites 4).
4. **Faux refus « le tarif a changé »** : prolongation sous plafond serré ; taux de remise qui monte à clé et période identiques ; motifs à distinguer. → En file (F3 suites 4).

## Faibles (en file)
5. `left(source_cle, …) = prefixe` n'utilise pas l'index : `LIKE prefixe || '%'` ou intervalle.
6. Une base persistante ayant appliqué l'ancienne 532 doit être recréée (empreinte) ; sans effet en production.
7. `compte_lie_a_eleve` exige `pc.statut = 'active'` : incohérent avec M1 (prise échue).
8. Test E1 dépend de `aujourd_hui()` (valable jusqu'au 30/06/2031).
9. Préfixes `regime:{evt}` et `annuel:` disjoints : couverture croisée non vue au rejeu annuel après un changement de régime (à vérifier).
