# Contre-revue des suites I5, passe 6 (PR 144, tête 0113e4ce) — 2026-11-24

Relecteur `review-sonnet` sur worktree, lecture seule. **Aucun Élevé.** Sentinelle par instruction équivalente à 316
(ancienne ET nouvelle offre, purge exemptée, 55000, 7 déclencheurs) ; règle 38 respectée ; F7 sans interblocage (ordre unique
campagne puis offre) ; course dépôt/archivage fermée par le verrou de campagne ; aucune assertion affaiblie trouvée.

## Moyens
- **A** : `pg_restore --disable-triggers` remet les déclencheurs à « origine » et fait perdre `ENABLE ALWAYS` → **doc du pilote** :
  rétablir `ALWAYS` et contrôler `tgenabled = 'A'` après toute restauration. (Vérification de santé au démarrage : en file.)
- **B** : réplication logique : le worker `replica` serait bloqué par une sentinelle `ALWAYS` → **doc du pilote** : déclencheurs
  `DISABLE` sur un abonné ; à reprendre dans la procédure PRA.
- **C** (tests seulement) : la garde de rejeu de 304 peut laisser la sentinelle absente si un rejeu échoue → en file.
- **D** : `ent_inscription_offre_du_dossier` encore appelée par ligne quand `offre_id` est NULL ; la mesure M4 n'exerce pas
  la fonction réelle ; pas de test multi-lignes et deux offres → en file.

## Faibles (en file)
1. Rejeu de 313/314 sur un historique non chaîné : passe par la sentinelle (seule 304 est gardée).
2. F6 : repli de l'alerte par rôle brut (`admin`, `administratif`) au lieu du droit `InscriptionsGerer` (corps sans nom d'élève : pas de fuite).
3. Échec ouvert hérité de 316 (dossier invisible → offre NULL → pas de verrou exigé) : cohérent avec « filet ».
4. Tests passe4 et i5 modifiés non relus en détail.
