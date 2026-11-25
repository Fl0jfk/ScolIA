# Contre-revue — correctifs F3 suites 4 (PR 156, a57db1af + retouche du pilote) — 2026-11-25

Relecteur `review-sonnet`, lecture seule.

## Fermé
- **E-1** pour les cas testés : année N+1 facturée en entier après un changement en N ; changement en cours d'année : chaque mois facturé une fois ; avoir total ou partiel ; deux changements dans l'année (270 €, sans surfacturation).
- **M-1** : tâches closes seulement après la garde réussie (refus de la garde : tâche ouverte).
- **M-2** : plafond relevé → complément à l'organisme, sans double facturation ; plafond abaissé → tâche.
- **F-1, F-2, F-3** ; garde contre la double facturation intacte.
- **Retouche du pilote** (D-069) : les mois avant un changement de régime ne sont plus refacturés au nouveau régime par l'annuel (externe sept-oct puis demi-pension : 360 €, pas 450 €).

## Élevé, préexistant (sur ENT avant ce lot) — décision du pilote : **en file, lot de conception**
- L'annuel facture l'année au régime **courant** et déduit l'historique des lignes `regime:` : un changement **sans ligne** (en juin, ou facturation de régime en échec) puis un rejeu annuel facture l'année entière au nouveau régime (450 € au lieu de 0 €). Le lot ne l'aggrave pas ; remède structurel : **régime mois par mois lu dans l'historique des changements**.

## Moyens
2. Changement au 1er septembre : septembre perdu (45 €, prudence D-069) → même remède.
3. Potentiel : annuel rejoué avant la réaction de régime → double facturation (préexistant) → même remède (lecture symétrique).
4. Doc en contradiction avec le code → **corrigée par le pilote**.
5. Plafond relevé avec un payeur suivant déjà posé : la famille garde sa part, aucun signal → en file (tâche).

## Faibles
6. Hausse de tarif sur une prise plafonnée facturée à l'organisme sans validation : à valider (D-069 : noté).
7. Test « avoir total » artificiel (DP forcé) : cas réaliste à ajouter.
8. Avoir partiel arrondi au mois supérieur : au pire un mois non refacturé (prudent, documenté).
9. Internat ↔ demi-pension à tarifs différents : rejeu annuel bloqué par le refus de tarif jusqu'à régularisation (F4).

## Décision
Lot **versable** : il ferme l'élevé E-1 du rapport précédent et n'aggrave rien ; l'élevé restant est **antérieur** et structurel. Il ouvre le lot de conception « régime mois par mois » (premier lot du Projet, Opus puis Sonnet).
