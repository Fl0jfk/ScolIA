# Contre-revue — F3 suites 4 (PR 156, tête 418e9b75) — 2026-11-25

Relecteur `review-sonnet`, lecture seule, rien exécuté. **Verdict : après correctifs (1 élevé).** PR **retenue**.

## Vérifié
- Moyen 1 (payeur suivant après coup, plafond 40 000 c sur 48 000 c) : 8 000 c facturés une fois au suivant ; rejeu et double rejeu sans ligne nouvelle ; organisme non refacturé (test `f3s5_m1`).
- Moyen 4 : complément sous plafond serré facturé (180 € posés, plafond 400 €, 220 € ajoutés, 50 € au suivant) ; garde contre la double facturation intacte (ligne sans période refusée, tarif changé refusé, rejeu neutre).
- Échappement `LIKE` correct ; pas d'heure réelle dans les tests f3s5.

## Élevé
- **E-1, faible 9 : la couverture croisée `regime:` / `annuel:` sous-facture l'année suivante.** `finances.rs:1752-1773` lit toutes les lignes `regime:%` de l'élève et de l'article **sans filtre d'année** (l'article est le même d'une année à l'autre, la clé de régime ne porte pas l'année). Changement externe → demi-pension en novembre 2030 ; en septembre 2031, la ligne annuelle 2031-2032 de demi-pension (450 €) est ignorée (`continue`), sans tâche ni anomalie. Même dans l'année : ni la période ni l'avoir ne sont regardés (une ligne de régime créditée par avoir bloque des mois qu'elle ne couvrait pas).

## Moyens
- **M-1, tâche close avant la garde** : `resoudre_taches_periode` (`finances.rs:1476`, `:1501`) avant `garde_periodes` (`:1632`) ; si la garde refuse, la transaction est validée quand même (`Ok(Err)` avalé par `finances.rs:3613` et `finances_reactions.rs:69`) : part non facturée et plus aucune tâche.
- **M-2, plafond qui monte** à clé et période identiques : refusé avec le motif « le tarif a changé » (`:1822-1824`), génération de l'élève bloquée ; plafond qui baisse : sous-facturation prudente non signalée.

## Faibles
- F-1 : la requête « préfixe indexable » ne l'est probablement pas (motif issu d'un `unnest`, collation) : `EXPLAIN` et bornes de plage.
- F-2 : `restant_prise` exclut les lignes propres sans condition de période (prise annulée ou sortie du calcul).
- F-3 : test du faible 9 trop faible (`res` non contrôlé).
