# Socle 21 — lot V11 « alertes précoces », partie déterministe : livraison

Date : 2026-11-28. Branche `claude/project-thread-bsie86`, base `origin/ENT` 06333a59. Conception : `docs/socles/21_EVALUATIONS.md` §13.3 (alertes précoces), §13.8 (exclusions écrites, « ce qui a changé »), §14.2, règles R-V20 et R-V40, réglage S-118 ; actions EVA-109, EVA-154, EVA-155, EVA-209, EVA-276 et, en partie, EVA-277.

**Niveau : implémenté et testé en local (PostgreSQL 16, feature `labo`). La CI est à lire sur la PR. Ce lot n'est pas validé par Florian et n'est pas prêt pour le pilote.**

## Pourquoi ce lot maintenant

V14, V6, V7, V8 et V10 sont dans des PR ouvertes (701 à 705). Restaient V9 (mobile, dépend de V7), V11 (IA), V12, V13, V15 et V16 (dépendent de V8 ou de V12). Le tableau des lots dit que **les alertes déterministes de V11 peuvent sortir avant l'étude juridique** (Q-EV8), et le complément de V11 dit la même chose de « ce qui a changé » (EVA-276). Ces deux parties ne lisent que les tables de V1 à V5, déjà sur ENT. Le lot part donc d'ENT et ne touche aucune table des PR ouvertes. **Aucune capacité à modèle de langage n'est livrée** : elles attendent l'étude juridique.

## Ce qui est livré (faits)

### Migration 706 `eva_v11_alertes`

Additive et rejouable, en fin de `MIGRATIONS` (règle 38). Aucune migration versée n'est modifiée, `migrations_versees.txt` est intact et `MIGRATION_TAG` est inchangé (3808). Aucune fonction, aucune colonne JSON (D-036).

- **`eva_alerte_suite`** : la suite humaine donnée à une situation (vue, entretien prévu, entretien fait, famille informée, cellule de veille, sans suite), avec une note interne de 500 caractères au plus, l'auteur et la date. Ajout seul (déclencheur `ent_trg_eva_ajout_seul`, troncature interdite). Clés étrangères vers l'élève et la période.
- **`eva_alerte_consultation`** : le journal des lectures des vues d'alertes (classe, cours, changements, suites). Il garde qui a regardé quoi et quand, jamais une valeur calculée. Ajout seul, une contrainte lie la vue à sa cible.
- RLS posée sur les deux tables (`tests_rls.rs` : +2).

Les alertes elles-mêmes ne sont **jamais stockées** : elles se recalculent à chaque lecture depuis les résultats, avec les règles de l'établissement.

### Les règles (§13.3, S-118)

Trois règles écrites, chacune expliquée par ses faits, sans score ni probabilité :

| Règle | Souple | Normal (défaut) | Strict |
|---|---|---|---|
| Baisse de la moyenne générale depuis la période précédente | 3 points | 2 points | 1,5 point |
| Matières strictement sous 8 | 4 | 3 | 2 |
| Absences et non-rendus aux évaluations de la période | 4 | 3 | 2 |

Les seuils de note sont sur 20 et suivent l'échelle de l'établissement (8 sur 20 = 4 sur 10). La position est lue dans `eva_reglage.seuils_alertes` (colonne de 461, jusqu'ici sans service). Les moyennes viennent du moteur unique (`moyennes_tx`, vue du personnel : toutes les évaluations ouvertes), comme le relevé et le bulletin.

**CE-70** : une absence à une évaluation n'est pas comptée quand le résultat porte la marque d'aménagement (V4) ou que l'élève a ce jour-là un aménagement de scolarité actif (`ent_amenagement`). La nature de l'aménagement n'est jamais dite, ni même son existence. Un travail non rendu compte toujours.

### Service (`ent-db/src/evaluations_v11.rs`) et routes (`ent-api/src/evaluations_v11.rs`)

7 chemins, 9 points d'entrée, tous réservés au personnel (la famille et l'élève reçoivent 403 avant toute lecture) :

| Route | Qui | Ce qu'elle rend |
|---|---|---|
| `GET /eva/v11/reglage-alertes?annee=` | tout le personnel | position et seuils, avec les seuils des trois positions |
| `PUT /eva/v11/reglage-alertes` | direction | choisit la position (possible après le gel des règles de calcul, comme le prévoit 461) |
| `GET /eva/v11/classes/{id}/alertes?periode=` | PP, CPE référent, direction | élèves en alerte, rangés par nom, règles et faits, moyenne générale et précédente, dernière suite posée ; effectif, avertissements, rappel |
| `GET /eva/v11/cours/{id}/alertes?periode=` | professeur du cours, direction | élèves du cours sous le seuil ou absents de façon répétée, et le nombre d'élèves sous 8 (« 4 élèves sous 8 dans ce groupe ») |
| `GET /eva/v11/eleves/{id}/changements?periode=` | PP, CPE référent, direction | « ce qui a changé depuis le dernier conseil » : moyenne générale et moyennes de matière avant et après, avec leur écart ; alertes nouvelles, levées et maintenues ; appréciations générale et de vie scolaire **validées** de la période précédente ; suites posées |
| `GET /eva/v11/eleves/{id}/suites?periode=` | PP, CPE référent, direction | toutes les suites de l'élève, les plus récentes d'abord |
| `POST /eva/v11/eleves/{id}/suites` | PP, CPE référent, direction | pose une suite (201) |
| `GET /eva/v11/ia/exclusions` | tout le personnel | les exclusions écrites de l'IA (R-V40) avec leur raison |
| `GET /eva/v11/consultations?du=&au=` | direction | le journal des consultations (un an au plus, 500 lignes) |

Les vues de classe et d'élève retirent l'élève protégé du socle 2 que l'acteur ne peut pas voir. Chaque lecture d'alertes (classe, cours, changements, suites) laisse une ligne au journal, dans la même transaction.

Les routes sont ajoutées à `routes_src.rs` (78 sources), donc à l'inventaire lu par la séparation, la recette de sécurité 3 et le contrôle « aucune route publique ». Un test de séparation couvre les 9 points d'entrée pour le parent, l'élève, le parent d'un autre foyer et le parent sans drapeaux.

### Registre et fusion

- Registre des données : `eva_alerte_suite` et `eva_alerte_consultation`, données internes du personnel (`perso`), hors export.
- `TABLES_ELEVE` (fusion de fiches, socle 2) : les deux tables restent sur la fiche où la ligne a été posée.

## Décisions prises par défaut (D-069, à relire par Florian)

1. **Droits** : aucune nouvelle `Action` (`Action::TOUTES` inchangé). Le catalogue n'a pas `evaluations.suivre_classe` ; sa portée est rendue ainsi :
   - la direction est reconnue à `periodes.verrouiller`, comme V8 et V10 ;
   - le professeur principal par le service (`ent_service_enseignement.professeur_principal`) ou par la mission `professeur_principal` sur la classe ;
   - le CPE **par la mission `cpe_referent` seulement**, sur la classe ou sur son niveau. Le rôle CPE seul ne suffit pas : c'est la lecture prudente de « CPE de référence » (minimisation) ;
   - un professeur voit les alertes de **ses** cours (`ent_prof_sur_cours`), jamais celles de la classe s'il n'est pas PP.
2. **Seuils des positions souple et stricte** : la conception ne donne que la position normale. Les valeurs du tableau ci-dessus sont un choix par défaut.
3. **Période précédente** : la période de la même périodisation, de la même année et de la même nature, qui finit avant le début de la période. La première période de l'année n'a pas de baisse (pas de comparaison d'une année sur l'autre), et la vue le dit par un avertissement.
4. **Matières comptées** : toutes les matières qui ont une moyenne, même hors moyenne générale. Une option faible reste un signal pour le PP.
5. **Absences** : `absent` et `a_rattraper` comptent comme absences, `non_rendu` comme non-rendu ; les évaluations de positionnement comptent aussi (l'absence est un fait, pas une note).
6. **Réglage** : changer la position exige que le jeu de règles de l'année existe déjà. Le service ne crée pas de règles par défaut à la place de la direction. Le changement passe par le déclencheur de 461 (version +1, `modifie_par`, ligne d'historique) ; l'historique de 461 n'a pas de colonne pour la position, qui n'y apparaît donc pas.
7. **« Ce qui a changé »** compare la période à la précédente (le dernier conseil se tient sur la période précédente). Il rappelle les appréciations générale et de vie scolaire **validées** de cette période, jamais un brouillon.
8. **Ordre des élèves** : par nom, jamais par nombre de règles (aucun classement). EVA-209 (« élèves à discuter ») est couvert par la vue de classe : la liste est motivée par ses faits.

## Ce qui n'est pas fait (reste à faire)

- **Cumul avec l'absentéisme** (socle 12) et les faits de vie scolaire (socle 13) « si la personne a le droit de les voir » : non branché. Les absences comptées ici sont celles **aux évaluations** seulement.
- **Avis `evaluations.alerte_eleve`** (socle 8, résumé hebdomadaire au PP et au CPE) : non fait.
- **Signalement à la cellule de veille** (EVA-277) : seule la trace « cellule de veille » est posée ; la transmission au socle 35 n'existe pas.
- **Fenêtre du conseil pour les professeurs** (EVA-276 : professeurs dans la fenêtre) : non ouverte, faute du socle 23 ; seuls PP, CPE référent et direction lisent les changements.
- **Capacités à modèle de langage** (propositions d'appréciations, relecture, synthèses, relecture des bulletins EVA-275) : après l'étude juridique (Q-EV8). La capacité `evaluations.alertes` du registre IA n'est pas déclarée.
- **Écrans Leptos** : aucun (API seule).
- **Résultats d'une fiche absorbée par fusion** dans la vue d'un cours : ils ne sont pas rattachés à l'élève conservé (la vue de classe et les changements, eux, passent par `eva_fiches_de_l_eleve`).

## Tests

- `ent-types` : seuils des trois positions, proportionnalité à l'échelle, listes fermées identiques à la base.
- `ent-db`, tests purs : aucune alerte sans fait ; baisse de 2 points exactement (et 1,999 sans alerte, hausse jamais) ; trois matières **strictement** sous 8, deux en position stricte ; cumul absences et non-rendus ; vue du cours ; aucun mot « risque », « score » ni « probabilité » dans les textes ; exclusions R-V40 complètes.
- `ent-db`, sur base (`tests_evaluations_v11.rs`, 3 scénarios) :
  - alertes de classe expliquées, CE-70 (absences sous aménagement non comptées et jamais dites), première période sans baisse, réglage lu par le professeur et changé par la direction seule, passage en strict, refus du professeur non PP, du CPE sans mission et de la famille, accès du CPE référent par mission de niveau, journal en ajout seul et réservé à la direction, fenêtre d'un an ;
  - alertes d'un cours pour son professeur seulement, nombre d'élèves sous 8 ;
  - « ce qui a changé » (écarts, alerte nouvelle, appréciation validée seule rappelée), suite posée puis visible dans la vue de classe, suite non supprimable, refus hors suivi, commentaire trop long, élève arrivé en cours d'année sans alerte inventée, exclusions de l'IA réservées au personnel.
- `ent-api` : séparation (9 points d'entrée, 403 pour la famille et l'élève) ; inventaire des routes.

## Compteurs (règle 28)

`tests_rls.rs` **+2** ; registre des données **+2** ; `TABLES_ELEVE` **+2** ; `Action::TOUTES` inchangé ; sources de routes **+1** (78 sur la base d'ENT ; 79 à 83 une fois V14, V6, V7, V8 et V10 versés) ; `migrate.rs` **+1** (706, après 495).

## Livraison

Branche `claude/project-thread-bsie86`, PR brouillon vers `ENT`. Pas de conflit de fichier attendu avec les PR 175 à 179, hors les listes partagées (`lib.rs`, `routes.rs`, `migrate.rs`, `routes_src.rs`, `separation.rs`, `registre_donnees.rs`, `tests_rls.rs`, `MIGRATIONS.md`, `PROJECT_STATUS.md`) que la fusion de chaîne résout en gardant les deux côtés.
