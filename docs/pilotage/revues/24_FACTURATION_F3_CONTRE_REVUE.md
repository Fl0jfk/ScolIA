# Contre-revue des correctifs F3 (PR 118), rapport du relecteur

Rapport intégral de la contre-revue `review-sonnet` (copie de travail de l'époque : chemins abrégés `f3b/` = dépôt, numéros de ligne indicatifs, à revérifier sur ENT).


## MOYEN

**M-A. Le jour de référence est le jour de génération, pas la période facturée (vérifié dans le code ; conséquence métier = hypothèse).**
- `facturer_annee_eleve` et `facturer_changement_regime` passent `jour_de_l_etablissement` (FIN:1452, 1622) à `sql_tarif_applicable` (FIN:1032-1045).
- Scénario 1 : inscription validée en juin 2027 pour 2027-2028, première grille en vigueur au 01/09/2027. Aucune version n'est applicable en juin. Si l'année n'a pas de tarifs historiques, le résultat est « aucun tarif applicable » (FIN:1435) et l'élève n'est pas facturé. La date de vigueur est bornée à l'année scolaire (GR:2470), donc impossible d'avancer la vigueur à juin.
- Scénario 2 : élève entré en janvier avec `depuis` en septembre. Tous les mois dus, y compris septembre à décembre, sont facturés au tarif v2 de janvier.
- `tarifs_tx` (FIN:3665) utilise aussi « aujourd'hui » : les tarifs d'une année future votée, mais pas encore en vigueur, n'apparaissent pas dans la liste.
- À trancher avec Florian, soit : jour de référence = max(aujourd'hui, début de période) ; soit tarif choisi mois par mois ; soit autoriser une vigueur antérieure au 1er septembre pour la rentrée. Documenter le choix retenu.

**M-B. Deux versions à la même date de vigueur : le choix suit le numéro, pas l'ordre des décisions (code vérifié, atteignabilité étroite).**
- Le déclencheur (MIG:266-269) et le service (GR:2505) n'interdisent que `du <` à celle d'une version déjà appliquée ; l'égalité est permise.
- Le choix (FIN:1042, GR:3599) est `ORDER BY en_vigueur_du DESC, numero DESC`.
- Scénario : v3 est en vigueur au 01/01 ; v2, restée « proposée », est votée plus tard puis mise en vigueur au 01/01. v2 devient `en_vigueur` et v3 `close`, mais v3 (numéro plus élevé) reste appliquée.
- Correctif : refuser l'égalité de date avec une autre version déjà appliquée (`b.du <= a.2` et même contrôle dans le déclencheur), ou ajouter un horodatage de mise en vigueur pour trier.

**M-C. Course sur la désactivation du second regard (risque potentiel).**
- Le service lit « proposée » sans verrou (GR:1587-1600). Le déclencheur (MIG:42-53) lit sous READ COMMITTED : une proposition concurrente non validée est invisible.
- Une désactivation et une proposition simultanées peuvent donc passer, puis la direction vote seule.
- Correctif : `pg_advisory_xact_lock` d'établissement dans `proposer_grille_for_actor` et `definir_reglages_grilles_for_actor`, ou `FOR SHARE` sur les lignes `proposee`.

**M-D. Lacunes de preuve dans les tests (confirmé par lecture et recherche).**
- Aucun test n'asserte le refus de création d'article ni d'émission quand la qualification est exigée par défaut. `f3r_m7` (T3:2746-2835) prouve seulement le défaut de colonne, la ligne absente et l'alerte ; la fin du test compte les lignes puis fait un rollback sans vérifier de refus. Les messages de refus (FIN:2394, 3234) n'apparaissent dans aucun test.
- `f3r_m2` (T3:2349-2378) vérifie « en base » avec `.is_err()` seulement, sans SQLSTATE ni message : un échec pour une autre raison passerait.
- `f3r_m1` ne teste ni `tarifs_tx` à une date, ni l'égalité de date (M-B), ni une vigueur future (la veille reste sur la précédente), ni le cas M-A.

## FAIBLE

1. **Bind mort.** `ajouter_tarif_grille_for_actor` (GR:1842) ajoute un 13e `.bind(actor)` pour un INSERT à 12 placeholders (ajout de ce lot). Probablement toléré, car sqlx envoie les types dans Parse (hypothèse : non exécuté). Intention probable : `modifie_par` ; à supprimer ou à brancher.
2. **Alerte du second regard (GR:1184-1198).** Le rapport promet « qui, quand » ; le code n'affiche que la date, formatée en UTC (`d.format`), sans auteur. Le texte « établissement antérieur à la 523 » apparaît aussi pour un établissement postérieur qui aurait mis `exiger=false` ; ce basculement n'est journalisé que sous l'action `grille.reglages`.
3. **Publication d'une version `close`.** Une version close reste applicable jusqu'à la veille de la suivante mais ne se publie pas (GR:2715 et déclencheur MIG:292). Les familles ne peuvent pas voir la grille encore appliquée.
4. **Suppression d'une simulation (GR:2988-3044).** Elle détruit l'historique des calculs (F4) avec le seul droit `Simuler` ; le journal ne garde pas les produits, alors qu'une exportation « conseil » est déjà journalisée. Recommandé : droit plus fort ou propriétaire, et produits dans le détail du journal. Le DELETE sur `ent_fin_simulation_calcul` n'est pas bloqué en base (seul UPDATE l'est).
5. **Plafonds non sérialisés (GR:2833, 3084).** Count puis insert sans verrou : dépassement possible de 200 en concurrence (impact minime).
6. **Ordre des contrôles incohérent.** `voter_grille_au_jour` valide la référence et l'année avant l'existence (GR:2356-2363), alors que le futur est contrôlé après ; `corriger` et `retour_brouillon` valident le motif avant l'existence. Cohérent avec l'ordre personnel, droit, existence uniquement si la validation de corps est tolérée avant l'existence.
7. **Auteurs du second regard.** Les tranches n'ont pas d'auteur ; les tarifs supprimés en brouillon disparaissent de `auteurs_grille_tx` (GR:3459) ; `retour_brouillon` efface `propose_par` (un ancien proposant peut ensuite voter). Hors liste littérale de la règle 3 mais contournable.
8. **Version supplantée.** Le journal donne seulement le nombre de versions supplantées, pas leurs identifiants (GR:2561).
9. **Tarif versionné en brouillon.** `annee_scolaire` peut être modifiée en base sans contrôle vis-à-vis de la version (MIG:353-383). Le tarif devient simplement inapplicable.
10. **Continuité (règle 4).** Ne couvre que les articles actifs (`a.actif`, GR:3610 et 3632) ; la facturation « regime: » n'est pas comptée dans `facturees` (seulement `annuel:`).
11. **F8 : documentaire uniquement.** Un commentaire de migration ; je n'ai pas vérifié le chemin d'effacement des utilisateurs. Les FK sans cascade bloquent seulement le DELETE physique.
12. **Cosmétique.** Commentaire `tests_rls.rs` « + 7 » alors que l'assertion ajoute 8 tables (calcul compris).

## Réponses aux points demandés

- **Trou ou chevauchement entre clôture et vigueur (règle 2).** Pas de trou : l'ancienne version passe `close` immédiatement (GR:2510-2522) mais reste applicable jusqu'à la veille grâce à `en_vigueur_du <= jour` et au tri par date décroissante. Pas de chevauchement hormis M-B.
- **Index unique partiel (`WHERE statut='en_vigueur'`, MIG:156).** Inchangé : au plus une version `en_vigueur` par année ; les précédentes sont `close`. Le choix est déterministe (date décroissante puis numéro), sauf l'égalité de date (M-B).
- **Chemins qui choisissent un tarif.** Seuls deux lisent `ent_fin_tarif` : la génération annuelle et le changement de régime (FIN:1105, via `lignes_annuelles`) et `tarifs_tx` (FIN:3658). Les deux utilisent `sql_tarif_applicable`. Le simulateur travaille sur une version choisie explicitement (`base_version_id`), sans date. Les factures ponctuelles, les avoirs et l'état des contributions ne lisent pas `ent_fin_tarif` (recherche sur tout le dépôt) ; l'état agrège les lignes émises.
- **Point 8 (ff16f6ec « établissement antérieur »).** Pas un contournement. Le helper `etablissement_antérieur` (T3:215) est limité à l'identifiant de l'établissement de test (INSERT ou UPDATE sur cet `etablissement_id`), il n'affecte aucun établissement par défaut ni ne change le défaut de colonne. Les établissements créés après la 523 reçoivent `true`. Le seul effet est d'éteindre l'exigence dans le jeu d'essai concerné. La règle M7 reste prouvée dans `f3r_m7` pour le défaut de colonne et la ligne absente, mais pas pour le refus d'émission (M-D). Le test web (`ent-web-server/tests/finances.rs:125`) passe par le service avec le droit `parametrer` de la comptabilité : correct. Il utilise `Utc::now().timestamp_micros()` comme suffixe de code, pas comme jour : acceptable.

## Conforme (vérifié dans le code)

- **Règle 1, gel en base.**
  - Déclencheur de version : INSERT imposé en brouillon vierge, colonnes de transition listées et vides au préalable, identité figée, DELETE et TRUNCATE interdits, hors-transition limité à `publiee_le`.
  - Tarif : `grille_version_id` et `etablissement_id` figés, `FOR SHARE` sur INSERT, UPDATE et DELETE, TRUNCATE bloqué.
  - Tranches : OLD et NEW contrôlés, version et établissement figés, TRUNCATE bloqué.
- **Règle 2, bornes.** Vote ≤ jour de l'établissement (`ent_aujourdhui_etab`). Vigueur ≥ vote, dans l'année du 1er septembre au 31 août, jamais avant une version déjà appliquée ; CHECK `vigueur ≥ vote` en base.
- **Règle 3, second regard.** Contrôlé en service et en base (créateur, proposant, `created_by` et `modifie_par` des tarifs). Désactivation refusée pendant une proposition, en service et par déclencheur sur `ent_fin_parametres` (course : M-C). Elle exige `voter` en plus de `parametrer` et est journalisée.
- **Règle 4, continuité.** `article_oublie` et `tarif_perdu` sont bloquants, joués à la proposition et rejoués à la mise en vigueur (GR:2485) ; repli sur l'historique quand il n'y a pas de version précédente.
- **Règle 5, supplantées.** Les autres versions votées passent `supplantee` ; la transition `supplantee → en_vigueur` n'existe ni en service ni en base.
- **Règle 6, M7 (hors preuve de test, voir M-D).** Colonne `DEFAULT true` posée après l'INSERT des établissements existants (donc faux uniquement pour l'existant) ; ligne absente = `(true, true)` (GR:1166) ; ligne sans article = non qualifiée (GR:1222) ; migration relançable sans effet pervers (`ON CONFLICT DO NOTHING`, la colonne n'est pas recréée).
- **Règle 7.**
  - F1 : FK composite `simulation_regle(etablissement_id, article_id)`.
  - F2 : 23001 et 23P01 classés en conflit (409).
  - F3 : `FOR SHARE` dans la garde.
  - F4 : simulation non écrasée, chaque calcul ajoute une ligne d'historique.
  - F5 : `opt_texte` répond 422 au lieu de tronquer (tous les appelants adaptés) ; plafonds des familles types (200) et des simulations (200), `regles` ≤ 50, `effectifs` ≤ 200, 12 tranches, 10 enfants.
  - F6 : CSV, tabulation et retour chariot en tête neutralisés.
  - F7 : arrondi symétrique en pourcentage ; hausse générale en montant épargnant `bourse` et `reduction` en hausse de grille et en simulation, sauf article explicite.
- **Règle 9.**
  - Cloisonnement : toutes les requêtes filtrent `etablissement_id` ; RLS ENABLE et FORCE sur les 8 tables ; clés étrangères composites.
  - Ordre personnel, droit, existence : `fin_exiger` d'abord partout.
  - Les tests F3 injectent un jour fixe (2030-2031), `facturer_au` et `voter_grille_au_jour` ; les `now()` restants ne sont que des horodatages SQL, pas des jours.
- **Purge d'établissement.** La fonction étendue conserve les six tables de la 522 et ajoute les tables de scénarios.

## Réponses du pilote (D-069)
- **M-A** : le tarif se choisit **mois par mois**, selon le mois facturé (jour de référence = premier jour du mois dû), et non au jour de génération. La liste des tarifs (`tarifs_tx`) accepte une date (défaut : aujourd'hui) pour montrer une année future votée.
- **M-B** : refuser l'égalité de date de vigueur avec une version déjà appliquée (service et déclencheur).
- **M-C** : verrou consultatif d'établissement dans la proposition et dans le réglage du second regard.
