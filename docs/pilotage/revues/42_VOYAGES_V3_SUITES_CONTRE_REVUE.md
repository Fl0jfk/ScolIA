# Contre-revue des suites voyages V3 (PR 138, tête a0b31a5a) — 2026-11-24

Relecteur `review-sonnet` sur worktree, lecture seule. **Aucun Élevé.** M-1 à M-4 de la revue précédente
(`42_VOYAGES_V3_CONTRE_REVUE.md`) sont réellement corrigés, chacun avec un test discriminant. Règle 38 respectée (3 fonctions
de déclencheur avec `SET search_path`, relations `public.`, aucun SECURITY DEFINER, 655 en fin de tableau). RLS et registre de
`ent_sortie_argent_refus` cohérents.

## Moyens
1. **Remise de bordereau hors règle 3** (`voyages_argent_flux.rs`, `voyage_bordereau_remettre`) alors que 655 l'annonçait.
   → **Corrigé par le pilote** : `exiger_libre_des_participants` sur les participants des lignes du bordereau ; commentaire
   de 655 rectifié (contrôle applicatif seul pour la remise) ; test `m3_la_remise_d_un_bordereau_est_refusee_au_parent_et_au_payeur`.
   Hors périmètre (en file) : budget, devis, commandes fournisseur, réglages.
2. **Sortie « rentrée »** : les participants restés « parti » perdent leur retrait pour toutes les nuits, y compris passées
   (`voyages_argent_v3.rs:1045-1089`, test `ff_…`). Inatteignable aujourd'hui (aucune commande n'écrit « rentree »).
   **Bloquant pour V3b** : borner à l'instant de passage en « rentree » (colonne `rentree_le`), test avec nuits passées.
3. **Blocage « sortie entière » (acomptes, annulation collective)** compte aussi désistés et liste d'attente.
   **Décision D-069 du pilote** : on garde la règle stricte (l'intégrité prime sur la disponibilité ; un désisté peut encore
   recevoir un avoir). Contournement tracé par une seconde personne : en file, à concevoir avec V3b.
4. **Tests SQL directs manquants** (23514) : frais réel, décision d'avoir (INSERT et UPDATE), paiement d'acompte (UPDATE),
   `ent_sortie_issue_coherente`. En file (suites V3b).

## Faibles (en file)
1. Branche directe de l'attendu redondante avec le CTE `retraits`, plus discriminée par `tests_internat_i2` ; l'`EXISTS`
   accepte un lien terminé → supprimer la branche ou exiger `annule_le IS NULL`.
2. Seuil M-2 : fonds social déclaratif ; `exiger_seconde_personne_annulation` sur l'écriture seule (pas le cumul) ; demandes
   expirées comptées dans le cumul.
3. Verrou avant le droit sur la saisie et la remise de bordereau → **corrigé par le pilote** (`avant_verrou` en tête).
4. Traces de refus partielles (seul `MSG_REGLE3` tracé ; 23514 de course → 500) ; pas de limite de débit des traces.
5. Rattrapage `annulee_le` en 655 dépendant du mode RLS de la session de migration (à confirmer sur base peuplée).
6. Annulation d'acompte sans auteur en base (`annule_par_user_id` à ajouter).
7. Cosmétique : `ent_sortie_retrait_effectif` sans préfixe `public.` en `:92`.
