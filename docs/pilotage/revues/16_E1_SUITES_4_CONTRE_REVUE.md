# Contre-revue des suites E1, passe 4 (PR 141, tête da2e54cf) — 2026-11-24

Relecteur `review-sonnet` sur worktree, lecture seule. **Aucun Élevé.** M1, F1, F5, F6, F7, F8 traités ; `p4_m1` solide
(ACL PUBLIC, `has_function_privilege`, appel réel refusé, témoin sous `ent_app`). Règle 38 : aucune fonction recréée ; 494
en fin de tableau ; rejeu idempotent. Aucune assertion affaiblie dans `tests_messagerie_p3` (sonde F8 par `pg_blocking_pids`).

## Moyens
- **A (risque)** : le `GRANT EXECUTE` de 494 n'est joué qu'une fois ; un rôle applicatif créé ou refait après la migration
  n'a pas le droit → toute annulation de différé échoue en 42501 (fermé, mais panne silencieuse). Le script d'exemple de
  `RLS_ISOLATION.md` porte le GRANT. **En file** : vérification au démarrage (`has_function_privilege`) avec alerte.
  Risque lié : un futur `DROP` + `CREATE` de la fonction rendrait EXECUTE à PUBLIC (rattrapé par `p4_m1`).
- **B (risque)** : le budget de verrou égalait `perime_apres` sans compter les envois ni les étapes antérieures ; avec deux
  instances, un avis en cours pouvait être repris (perte ou double envoi). → **Corrigé par le pilote** : budget =
  `perime_apres / 2`.

## Faibles (en file)
1. Lot affamé si ≥ `taille_lot` avis visent des messages verrouillés (les reportés gardent leur `planifie_pour`).
2. F5 : « autre établissement invisible » prouvé par le `WHERE`, pas par la RLS ; refus sous rôle sans droit en `is_err()`
   seul (le dépôt rend « Infrastructure » : exposer le code avant de l'exiger).
3. F6 : aucun test ne prouve que `expedier` appelle `observer_borne_hors_flux` (`SUIVI` global non injectable).
4. F6 : journalisation `error = ?e` répétée toutes les 5 s si la base est indisponible (`%e` ou limite de fréquence).
5. Doc : répétition « 494, 494 » → corrigée par le pilote.
