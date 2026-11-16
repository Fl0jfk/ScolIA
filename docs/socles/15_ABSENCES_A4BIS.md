# Socle 15 — lot A4bis : on organise sans attendre la validation

**Niveau atteint :** *implémenté et testé* (local ; CI : voir §6). **Non validé** par Florian, **pas prêt pilote**, rien déployé, aucune donnée de production touchée. Aucune migration.

## 1. Le défaut

`edt_remplacement::controler_absence_citee` exigeait `statut = 'validee'`. Un remplacement accepté sur une absence seulement déclarée ou signalée était refusé par l'emploi du temps ; la demande passait `annulee / echec`, la séance revenait « à traiter » et l'API répondait **409 « valeur déjà existante (conflit d'unicité) »** (`DomainError::Conflit`, sans message). Révélé par les écrans A11 (`15_ABSENCES_A11_ABC.md` §6). Contraire à §8.1 de la conception : *« impact calculé et visible (on organise même sans confirmation) »*.

## 2. Ce qui change

| Sujet | Avant | Après |
|---|---|---|
| États qui ouvrent une réponse (remplacer, annuler la séance) | `validee` seulement | **tous sauf `annulee`** : signalée, déclarée, en instruction, validée, **refusée** (`statut_ouvre_une_reponse`) |
| Refus de règle métier de l'emploi du temps (`edt_organiser_remplacement`, `edt_annuler_seance`) | `DomainError::Conflit` : « valeur déjà existante » | `EntDbError::Refus(Conflit, message)` : 409 **avec une raison lisible** |
| Acceptation d'une demande dont l'absence a été annulée | « la séance est déjà pourvue ou passée » (faux) | « cette demande n'a plus d'objet : l'absence a été annulée », demande close (cause `absence_annulee`) |
| Demande ouverte dont la séance n'est plus touchée (absence annulée, écourtée, plage retirée) | restait « envoyée » : le remplaçant pouvait l'accepter | close (cause `absence_annulee`, déjà permise par le CHECK de la migration 2305), journalisée |

Messages explicites : absence annulée ; absence qui ne concerne pas l'enseignant ou le jour ; séance déjà annulée ; cours déplacé ; remplaçant indisponible. Ils ne nomment **jamais** la nature, l'état de décision ni l'absence d'une autre personne (R-AP2 ; le refus « remplaçant indisponible » dit « cours, indisponibilité ou rôle inadapté », sans « absent »).

## 3. Décisions sur les cas limites (à valider)

Principe : conformément à §8.3 et CAP-35, **une réponse déjà posée n'est jamais levée toute seule** (le remplaçant a peut-être préparé) ; elle est signalée « à revoir » et l'adjoint décide.

| Cas | Décision | Séance / remplaçant | Heures (A7) |
|---|---|---|---|
| Absence **refusée** après remplacement | Le remplacement **reste** (§8.3 : impact inchangé ; l'agent n'était pas là, R-AP15). On peut aussi organiser sur une absence refusée. Rien à revoir. | inchangés | heure constatée si la séance est assurée (appel fait) |
| Absence **annulée** après remplacement | Remplacement **conservé**, impact `a_revoir = true`, demande retenue conservée. L'adjoint lève (`lever_reponse`) s'il le décide ; la levée révoque la délégation et rend la séance au titulaire. | remplaçant toujours désigné jusqu'à la levée | si la séance a lieu avec le remplaçant (appel fait) : heure due, c'est un fait ; si levée : **aucune** heure (CAP-35). Le moteur A7 ne constate que sur remplacement **actif** + séance terminée + appel |
| Absence annulée **avant** l'acceptation | La demande ouverte est close à l'annulation (cause `absence_annulee`) ; une acceptation tardive est refusée avec un message qui dit pourquoi ; rien n'est appliqué | — | aucune |
| Absence **écourtée** | Séances au-delà de la reprise : sans réponse → retirées de l'impact (ou `sans_objet`) ; **avec** réponse → conservées « à revoir » (proposé, pas levé seul) ; demande ouverte → close | remplacement conservé tant que l'adjoint ne lève pas | idem : suivent les faits |
| Absence **prolongée** | Nouvelles séances touchées, `a_traiter` ; elles se remplacent tout de suite, sans validation ; une séance redevenue touchée perd son « à revoir » | — | — |

Interdits maintenus : aucune levée automatique, aucune suppression (la levée archive, AR-15), aucune heure sans séance assurée (R-AP28).

## 4. Hors périmètre (signalé, non fait)

* **Prévenir le remplaçant** quand l'absence est annulée ou écourtée alors qu'il est désigné (§8.3 « le remplaçant est prévenu ») : dépend de la notification effective du socle 8, non branchée ; l'état « à revoir » est visible de l'adjoint, la demande fermée l'est du remplaçant dans « mes demandes ».
* Propositions de levée en un geste (« lever tout ce qui est à revoir ») : écran A11, hors backend.
* Réaction aux absences des **co-enseignants** et suppléances (A5, A6) : inchangées.

## 5. Tests (`tests_absences_personnels.rs`, bloc « LOT A4bis »)

| Test | Couvre |
|---|---|
| `a4bis_le_remplacement_accepte_est_applique_dans_chaque_etat_de_l_absence` | signalée, déclarée, en instruction, validée, refusée : sollicitation → acceptation → exception posée, demande `acceptee`, aucun échec, absence inchangée |
| `a4bis_annuler_la_seance_et_refus_explicites` | annulation de séance acceptée dans chaque état ; refus 409 **explicites** (absence annulée, remplaçant indisponible, séance déjà annulée) ; jamais `DomainError::Conflit` |
| `a4bis_absence_annulee_avant_l_acceptation` | demande close à l'annulation, acceptation tardive refusée avec message, rien appliqué |
| `a4bis_absence_refusee_apres_remplacement_le_remplacement_reste` | remplacement et heure du remplaçant conservés |
| `a4bis_absence_annulee_apres_remplacement_a_revoir_puis_levee` | « à revoir », rien levé seul ; levée → aucune heure |
| `a4bis_absence_ecourtee_et_prolongee` | écourtée (réponse conservée à revoir, demande ouverte close), prolongée (nouvelle séance remplacée sans validation) |
| `a0_ar11_…` (adapté) | l'ancien test exigeait « validée » : il vérifie maintenant absence annulée / autre agent refusés (messages), déclarée acceptée, autre établissement « introuvable » |

## 6. Vérifications

Voir la section « Résultats » ci-dessous (complétée après exécution).

**Résultats :** à compléter.
