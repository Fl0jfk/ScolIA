# Contre-revue des suites I4, 3 (PR 142, tête f9cc15b2) — 2026-11-24

Relecteur `review-sonnet` sur worktree, lecture seule. **Aucun Élevé.** F7 (moteur de droits) est **équivalent** à l'ancien
comportement : mêmes SQL, mêmes filtres `etablissement_id`, contexte tenant posé par `TenantTx::begin(pool, etab)`, READ
COMMITTED inchangé, SELECT seulement (aucun verrou nouveau), pool moins sollicité. F1, F2, F4 faits et testés ; assertions de
`tests_internat_i4_suites2` renforcées. Appels de `service.rs` hors transaction : pas de problème.

## Moyens
1. **F7 incomplet (antérieur au lot)** : `exiger_eleve_visible` (`vie.rs` 654, 1687, 1804, 2193) et `a_action_etablissement`
   dans `groupe_appel_exiger` (`groupes.rs` 728-745, appelé en 2098, 2352, 2437, 2605) prennent encore une connexion du pool
   dans une transaction ouverte → interblocage possible sous charge (pool de taille N, N requêtes). **En file** : variantes
   `_conn` + test sur un pool d'une connexion.
2. **F6 défini, non branché** (aucune surface famille). → doc corrigée par le pilote.

## Faibles
1. `eleve_cache_au_tiers_conn` `pub` sans contrôle tenant → **corrigé par le pilote** (`pub(crate)`, contrat documenté).
2. F2 : le 3e élève n'est signalé que dans l'avis ENT (aucun e-mail ni push complémentaire) : limite assumée ; piste : un rectificatif par minute.
3. Test F1 : la contrepartie « texte identique : l'avis reste lu » n'est pas testée.
4. `s5_avis_regroupe_*` dépend de l'état préalable du banc (envoi déjà `envoye`).
