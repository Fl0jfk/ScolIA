# Socle 21 — lot V10 « pilotage et statistiques » : livraison

Date : 2026-11-28. Branche `claude/project-thread-6vmdbh`, base `origin/ENT` 06333a59. Conception : `docs/socles/21_EVALUATIONS.md` §17.12 (EVA-212 à EVA-218), §17.5 (EVA-153, EVA-156), EVA-279, §14.6 point 10 et règle R-V44.

**Niveau : implémenté et testé en local (PostgreSQL 16, feature `labo`). La CI est à lire sur la PR. Ce lot n'est pas validé par Florian et n'est pas prêt pour le pilote.**

## Pourquoi ce lot maintenant

V14, V6, V7 et V8 sont dans des PR ouvertes (701 à 704). V9 (mobile) dépend de V7, V11 (IA) de V6 et V8, V15 et V16 de V8 et du socle 23. V10 dépend de V8 dans le tableau des lots, mais seulement pour le tableau de fin de période (EVA-212), que V8 couvre déjà en partie par son pilotage de la saisie. Le reste de V10 (évaluations communes, statistiques, harmonisation, charge, retards) ne lit que les tables de V1 à V5, déjà sur ENT. Le lot part donc d'ENT et ne touche aucune table des PR ouvertes.

## Ce qui est livré (faits)

### Migration 705 `eva_v10_pilotage`

La migration est additive et rejouable. Elle est en fin de `MIGRATIONS` (règle 38). Aucune migration versée n'est modifiée et `MIGRATION_TAG` est inchangé (3808).

- **`eva_evaluation_commune`** (462, sans service jusqu'ici) reçoit `examen_blanc`, `modifie_par` et `modifie_le`.
- **Index unique partiel `eva_eval_commune_cours_uq`** : une seule évaluation non annulée par cours dans une même commune (CE-46).
- **`eva_consultation_pilotage`** : journal des consultations de statistiques (R-V44). Il garde qui a regardé, quelle vue, sur quelle période, commune ou niveau, et quand. Il ne garde aucune valeur calculée. Il est en ajout seul (déclencheur `ent_trg_eva_ajout_seul`, troncature interdite). Une contrainte lie la vue à sa cible. La RLS est posée (`tests_rls.rs` : +1).

### Service (`ent-db/src/evaluations_v10.rs`) et routes (`ent-api/src/evaluations_v10.rs`)

- **Évaluations communes** (CE-46) : la direction crée l'évaluation commune (libellé, date, barème, examen blanc) et y rattache des évaluations déjà créées par les professeurs. Le rattachement est refusé, tout ou rien, dans ces cas :
  - autre date ou autre barème ;
  - évaluation annulée ou de positionnement ;
  - deuxième évaluation d'un même cours ;
  - évaluation déjà dans une autre commune.

  Elle peut ensuite rattacher ou détacher des évaluations. La liste montre tout à la direction, et au professeur les communes où il a un groupe (marqué `mon_cours`).
- **Comparaison entre groupes** (EVA-156, EVA-214, R-V44). La direction voit chaque groupe et l'ensemble. L'ensemble est la réunion des séries, pas une moyenne de moyennes. Un professeur concerné voit ses groupes, et des autres seulement le nombre de groupes, l'effectif et la moyenne **anonyme**, celle-ci masquée sous le seuil d'effectif de l'établissement (jamais sous le plancher du moteur). Les groupes sont rangés par libellé de cours, jamais par valeur.
- **Écarts à l'examen blanc** (EVA-216, direction). Pour chaque élève noté à l'examen, on compare sa note à sa moyenne de contrôle continu : la moyenne du cours sur **sa** période à la date de l'examen, examen exclu, calculée par le moteur. Seuls les élèves qui ont les deux entrent dans le calcul. Le résultat donne, par groupe et pour l'ensemble, les deux moyennes et leur écart.
- **Statistiques de période** (EVA-213). La série est celle des **moyennes de cours des élèves** (`moyenne_cours` du moteur unique). Seuls comptent les résultats dont la période, résolue par élève (R-V29), est la période demandée.
  - La direction choisit l'axe : matière, classe, niveau ou professeur (identifiant du compte, le nom est résolu par l'écran).
  - Un professeur reçoit toujours l'axe « cours », réduit à ses cours, quel que soit l'axe demandé.
- **Répartition par nature** (EVA-215, direction). Pour chaque nature, on donne :
  - le nombre d'évaluations et de notes ;
  - la somme des coefficients et sa part dans la période ;
  - la série des moyennes par évaluation.
- **Harmonisation d'un niveau** (EVA-279, direction). Matière par matière, on donne la moyenne du niveau et, pour chaque classe, ses statistiques et son écart à la moyenne du niveau. Aucun nom de professeur n'apparaît, et les classes sont rangées par libellé.
- **Charge d'une classe** (EVA-153, EVA-217). Elle couvre les évaluations ouvertes qui concernent au moins un élève de la classe à leur date (`eva_grille_eleves`), sur une fenêtre de 120 jours au plus. Pour chaque semaine (du lundi), elle donne le nombre d'évaluations et les jours à deux évaluations ou plus. Aucune note n'apparaît. Elle est ouverte à la direction, au professeur principal et aux professeurs qui enseignent à la classe.
- **Évaluations non publiées** (EVA-218, direction). Ce sont les évaluations ouvertes et non publiées dont la date dépasse le délai. Ce délai est le délai maximal de publication de l'établissement (EVA-106), sinon 15 jours, et la direction peut aussi le choisir (de 1 à 120 jours). Pour chaque évaluation, on donne :
  - les résultats saisis ;
  - l'indicateur « en cours de correction » ;
  - l'indicateur de publication programmée ;
  - les comptes des professeurs du cours.
- **Journal des consultations** (R-V44, direction) : les 500 dernières lignes, filtrables par acteur. Les vues journalisées sont les statistiques, les natures, l'harmonisation, la comparaison d'une commune et les écarts à l'examen blanc.
- **Moteur** : `notes_ramenees` est extraite de `statistiques_evaluation`, qui l'appelle désormais. La comparaison réunit ainsi les séries par le même `classer` que la moyenne. Le comportement de `statistiques_evaluation` est inchangé (377 tests du moteur verts).
- **Événements** : `evaluations.commune_creee` et `evaluations.commune_modifiee`, sans `eleve_id` (R-V17).
- **12 chemins, 13 points d'entrée** (`GET` et `POST` sur `/eva/v10/communes`). Ils sont ajoutés à `routes_src.rs` (78 sources).
- **Registre des données** : `eva_consultation_pilotage`, donnée du membre du personnel (`perso`), interne, hors export.

## Décisions prises par défaut (D-069, à relire par Florian)

1. **Droits** : sans nouvelle `Action`, `Action::TOUTES` est inchangé.
   - La direction est reconnue à `periodes.verrouiller`, comme dans V8. Elle seule crée les évaluations communes et lit les statistiques de l'établissement, les natures, l'harmonisation, les écarts, les retards et le journal.
   - Le secrétariat, qui a `notes.lire`, ne voit pas ces vues : R-V44 les réserve à la direction.
2. Une évaluation commune exige **la même date et le même barème** pour toutes ses évaluations (« même sujet, même date, barème commun », §4.4 point 4). Un examen sur deux jours devra être modélisé en deux communes.
3. Rattacher ou détacher une évaluation incrémente sa version, par le déclencheur existant. Un professeur qui modifie cette évaluation au même moment reçoit un conflit de version et recharge. Ce n'est pas une perte de donnée.
4. La série des statistiques de période est celle des **moyennes de cours** par élève, pas des moyennes de matière. Un élève suivi dans deux cours d'une même matière compte deux fois sur l'axe matière. C'est plus simple à expliquer, et la moyenne de matière reste dans le relevé et le bulletin.
5. Pour la vue « professeur » de la comparaison, le seuil d'anonymat est le seuil d'effectif des statistiques de l'établissement (5 par défaut), jamais sous le plancher du moteur.
6. Une évaluation non publiée n'est signalée qu'après le délai, compté depuis la date de l'évaluation, et non depuis la saisie.
7. Le journal enregistre chaque lecture, sans regroupement. Une lecture refusée (403) n'en laisse aucune.

## Tests

- 4 scénarios en base (`ent-db/src/tests_evaluations_v10.rs`) :
  - règles de rattachement et comparaison direction ou professeur (seuil, liste, détachement, journal en ajout seul) ;
  - écarts examen blanc et contrôle continu ;
  - statistiques par axe, vue réduite du professeur, natures, harmonisation, droits ;
  - charge de classe et évaluations non publiées (délai par défaut, délai choisi, bornes, droits).
- 4 tests unitaires du service : semaines, bornes de liste, seuil d'anonymat et séries.
- 2 tests de types.
- Séparation : les 13 points sont refusés (403) aux parents (dont un autre foyer et un parent sans droits) et aux élèves. Les 64 tests de `separation` sont verts, dont l'inventaire « aucune route sans session ».
- Tests voisins verts en local : `tests_rls`, registre, `migrate`, V3, V8 (82 tests), moteur (377).

## Ce qui reste (non fait)

- Les écrans Leptos (tableaux de la direction, comparaison, charge). Ce lot ne livre que l'API.
- Le lien avec le socle 38 (organisation des examens : salles, convocations), qui n'est pas codé.
- Le tableau de fin de période de l'établissement (EVA-212). Il se compose de l'état de saisie de V8 classe par classe et sera fait avec les écrans, une fois V8 versé.

## Lien avec les PR ouvertes

Ce lot ne touche aucune table de 701 à 704. Il peut entrer en conflit textuel avec V8 aux endroits où chaque lot ajoute en fin de liste :
- fin de `migrate.rs` ;
- compteurs de `tests_rls.rs` et de `routes_src.rs` (79 sources si les deux sont versés) ;
- fin de `separation.rs` ;
- `PROJECT_STATUS.md` et `MIGRATIONS.md`.

Il suffit de garder les deux ajouts.
