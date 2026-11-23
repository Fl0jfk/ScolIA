# Contre-revue des suites V5 (PR 125, tête fe33a3ac) — revue indépendante du pilote

Numéros de ligne : ceux de fe33a3ac ; les relire sur la base actuelle.

**Contre-revue des suites V5 (socle 21, migration 478) : aucun défaut Élevé, 3 Moyens, 5 Faibles**

Lecture seule du patch et de la copie `/tmp/claude-0/-home-user-ScolIA/11355a13-49ee-5b3e-ab9a-53b200277b90/scratchpad/v5s`. Rien n'a été exécuté, ni tests ni migration : tout vient de la lecture du code. Les chemins relatifs sont sous `ent-rust/crates/` (tests dans `ent-db/src/` et `ent-api/tests/`).

## Élevé
Aucun défaut confirmé. Tous les chemins famille et élève passent par `moyennes_tx`, qui est le seul point de filtrage de la publication :
- Appelants de `moyennes_tx` : `ent-db/src/service.rs:7616`, `evaluations_v1.rs:1192/1218/1235` et `evaluations.rs:907` (classe, bulletin, conseil).
- `externes-famille` (`ent-db/src/evaluations_v5.rs:805`) ne lit que les résultats externes `publiee_le IS NOT NULL`, jamais les moyennes.
- Le parcours est réservé au personnel.

## Moyen

**1. (confirmé) M-B : une moyenne externe publiée qui disparaît pour la famille trahit l'existence d'une note non publiée.** `ent-db/src/evaluations_v1.rs:1587-1612` (décision) et `1651-1662` (affichage).
- Scénario : la famille voit la moyenne externe publiée de la matière M. Le professeur saisit une première note d'établissement non publiée dans M. La moyenne externe disparaît pour la famille.
- Ce que la famille voit exactement : la matière sans moyenne, plus l'avertissement « la moyenne externe n'est pas comptée pour le moment ». Elle n'apprend ni valeur, ni nombre, ni nature de ce qui bloque, mais c'est un oracle binaire. Le test `contre_revue_mb` l'affirme lui-même (« la note non publiée … empêche la moyenne externe de compter »).
- Même oracle pour une seconde moyenne externe non publiée (`1664-1676`) et pour un résultat externe compté mais non publié (`1596-1603`, pas de filtre de publication).
- La règle « une note non publiée n'existe pas pour la famille » et « un seul moteur » se contredisent ici. À trancher et écrire dans D-069.
- Correctif possible : pour la famille, décider sur les seules lignes publiées (règle 1), et prévenir le personnel que la publication future de la note masquera la moyenne externe. À défaut, accepter et documenter l'oracle, et au minimum ne pas émettre d'avertissement côté famille.

**2. (confirmé) `publier` et `retirer-publication` ne comparent pas ce que la direction a vu.** `ent-db/src/evaluations_v5.rs:1205-1208` et `1243-1245`.
- La direction A lit v1 et clique « publier ». Entre-temps B corrige en v2, ce qui dépublie la ligne. Le geste de A publie v2 sans qu'elle l'ait vue. La règle compare-et-écrit n'est pas respectée.
- Correctif : ajouter au corps `valeur_m`/`statut` (ou un jeton de version) et comparer dans le `WHERE`, avec un conflit en cas d'écart.

**3. (risque potentiel) Migration 478 : la contrainte `eva_moyenne_externe_publication_statut_chk` peut faire échouer la migration.** `ent-rust/migrations/478_eva_v5_contre_revue_publication.sql:1334-1336`.
- Sous 477, une correction gardait la publication (trigger de 477, lignes 464-481). Une ligne publiée puis corrigée en `en_attente` ou `dispense` existe peut-être déjà dans un environnement ayant 477. `ADD CONSTRAINT` sans `NOT VALID` échouerait.
- Dans le même cas, ces lignes déjà corrigées restent visibles de la famille avec une valeur non republiée : 478 ne les dépublie pas, par choix « sans toucher aux données versées ».
- À vérifier : existe-t-il un environnement avec des données sous 477, hors CI ? Si oui, `ADD CONSTRAINT … NOT VALID` et un constat des lignes concernées.

## Faible
- **F-1 : classes comparées par code, fenêtre en enveloppe.** `ent-db/src/evaluations_v5.rs:1444-1457`.
  - Segments et rattachement sont filtrés par `classe_code` (chaîne) et non par `classe_id`.
  - La fenêtre est l'enveloppe de tous les cours du professeur, et `du` est la présence de l'élève, pas la période d'enseignement du professeur. Un segment de la classe A hors de ses dates mais dans l'enveloppe est servi.
  - Recommandation : fenêtre par classe, avec `classe_id`.
- **Droit désigné lié à l'horloge réelle.** Le commentaire `evaluations_v5.rs:141-146` promet « aucune horloge », mais la branche `autoriser_eleve` (lignes 186-193) ne reçoit pas `jour`. À corriger dans le code de production ou à documenter.
- **M-D : test CPE désigné dépendant de la date réelle** (détail plus bas).
- **Tests faibles ou surdéclarés.**
  - Dans le test F-1 (`ent-db/src/tests_evaluations_v5.rs`), la branche `sans` accepte `Forbidden` ou une réponse vide : aucune assertion réelle.
  - Le commentaire de `ent-api/tests/separation.rs` (~3167) annonce « la direction voit et retire la publication », mais le code ne le fait pas.
  - Pas de test HTTP de succès de `retirer-publication`, seulement la matrice des 403.
  - `contre_revue_mb` n'asserte pas l'avertissement famille dans le cas b.
- **Le retrait écrase `saisi_par`.** `ent-db/src/evaluations_v5.rs:1244` : la ligne perd l'auteur de la valeur, que seul le journal garde. Les lignes de journal antérieures ont `etait_publiee = false` par défaut (précision perdue, pas une erreur).

## Cas demandés

**M-A (correction d'une moyenne externe publiée)** : conforme, vérifié en base et dans le service.
- Base (`478:1405-1416`) : la correction force `publiee_le` et `publiee_par` à NULL, y compris si un UPDATE direct les laisse inchangés. Motif de 3 à 500 caractères exigé. On ne corrige pas et on ne publie pas dans le même geste.
- Journal : l'événement « correction » porte `etait_publiee = OLD.publiee_le IS NOT NULL` (`478:1444`). Le retrait est un événement distinct, avec motif.
- Service : la route de retrait passe par `eva_v5_charger_moyenne` (staff, puis `NotesCorriger`, puis visibilité de l'élève). Motif exigé, 403 au professeur, conflit au double retrait, ligne inconnue en 403 ou introuvable selon le droit.
- Moteur, relevé, bulletin, parcours (personnel seul) et `externes-famille` : aucun accès famille à une moyenne dépubliée.

**M-C (fusion de fiches)**
- Vérifié : le moteur lit `eva_fiches_de_l_eleve` ; la fiche conservée l'emporte sur l'absorbée pour la même évaluation ; l'annonce de `domain-eleve/src/fusion.rs` est corrigée.
- Vérifié : le refus « fusion protégée vers non protégée sans G6 » n'est pas du code nouveau. `ent-db/src/eleve_fusion.rs:624-629` existait ; le patch ajoute seulement le test.
- Parcours qui ne suit pas la fiche conservée : pas de risque d'exposition. Le parcours est réservé au personnel déjà autorisé sur l'élève. Le risque est une incohérence entre la moyenne (qui compte la note) et le détail du parcours (qui ne la montre pas). C'est une dette de cohérence, pas un défaut de sécurité.
- Hypothèse non vérifiée : pas de doublon dans les statistiques de classe, car les affectations sont déplacées vers la conservée (`domain-eleve/src/fusion.rs:67`).
- Limite connue : deux absorbées portant chacune un résultat pour la même évaluation comptent toutes deux.

**M-D : le test CPE à portée désignée dépend de la date réelle.**
- Fichier : `ent-db/src/tests_evaluations_v5.rs`, test `contre_revue_md_dispense_et_portee_designee_sans_cours_dans_le_parcours`.
- Lignes 2898 (`UPDATE ent_affectation SET date_debut = '2020-09-01'`), 2935 et 2948 (parcours à `jour()` fixé au 2031-10-20, alors que la portée désignée est évaluée à la date réelle par `autoriser_eleve`).
- Le test passe tant que l'affectation du labo est ouverte à la date réelle. Il casse si elle a une `date_fin`, ou si l'horloge sort de la classe ou de l'année.
- Correctif préféré : passer `jour` à `autoriser_eleve` en production, pour un test réellement déterministe. À défaut : poser `date_fin = NULL` en plus de la date de début, ou utiliser un jour relatif à `ent_aujourdhui()` en SQL comme le test de fusion.
- Le test `dispense` est correct et déterministe.

**F-2 (assistant ou intervenant sans déclaration)** : conforme.
- `saisie = !actions.contains(NotesLire)` (`ent-db/src/evaluations_v5.rs:170`) est transmis à `ent_public_cours_jour` et à `eva_v5_enseigne`.
- Couplage fragile : un paramètre explicite serait plus sûr.
- Le refus d'une année précédente pour le professeur est en `458-498`, la direction en est exemptée.

**F-3 (`publier` contrôle `statut = 'moyenne'`)** : conforme, dans le service (`1207`), le trigger (`478:1385`) et la contrainte. Le test refuse un UPDATE direct. Seule réserve : le point Moyen 3.

**F-5 (tests sans jour réel)** : partiel. Les tests HTTP touchés utilisent `2020-10-10` en dur et aucun `jour_paris()` n'est ajouté. Restent des usages préexistants (`ent-api/tests/separation.rs:165` et `2417`) et le test CPE.

**F-6 (publication sans écran)** : exact, c'est une API seule. Cohérent avec le reste à faire du document.

**Migration 478** : rejouable, sans modification de données, sans nouvelle table.
- 477 n'est pas dans le diff : `ent-rust/migrations/477_eva_v3e_v5_parcours.sql` inchangée (vérifié par la liste des fichiers du patch).
- Rejouable : `ADD COLUMN IF NOT EXISTS`, `DROP CONSTRAINT IF EXISTS` puis `ADD`, `CREATE OR REPLACE FUNCTION`. Le nom `eva_moyenne_externe_historique_evenement_check` suit la convention PostgreSQL d'un `CHECK` de colonne.
- RLS inchangée : pas de nouvelle table, `ent_rls_poser` de 477 déjà posé. Triggers non `SECURITY DEFINER`. Journal toujours en ajout seul.
- Seul point : le risque de la contrainte (Moyen 3).

## Conforme
- Chemin unique de calcul pour toutes les audiences (`moyennes_tx`).
- Parité des filtres entre la requête « toutes lignes » (`evaluations_v1.rs:1587-1612`) et `par_matiere` (même `ev.etat`, même `eva_periode_dans`, résultats externes comptés non retirés).
- Ordre de vérification du retrait : staff, droit `NotesCorriger`, visibilité, existence. Aucune fuite d'existence au professeur.
- Journal en ajout seul, auteur exact, motif obligatoire pour correction et retrait.
- Séparation entre établissements et matrice des 12 routes en 403 à la famille et à l'élève, étendues à la nouvelle route.
- Compteurs et registres cohérents avec l'absence de nouvelle table.

