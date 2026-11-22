# Semis de labo relatif à l'année scolaire (branche `ent-semis-labo-relatif`, base ENT 02659006)

Aucune migration, aucun changement de code de production (seul `seed.rs`, compilé avec `labo`, et un test).

## Cause
`seed_labo_cast` était déjà en partie relatif (`jour = today_utc()`), mais gardait deux dates en dur : la rentrée des élèves
(`2026-09-01`) et la scolarité N-1 (`"2025-2026"`). À partir du 2027-09-01 la rentrée est dans le passé de l'année
courante et le semis échoue. S'y ajoutait l'**année « préparation » parasite** : la période T0 de démo (fermée, finissant
la veille de la rentrée) tombe dans l'année précédente, que le référentiel crée alors **non courante, en préparation, avec
des copies des classes 4B et 5A** (constaté en base : 2025-2026 / préparation / classes 4B, 5A à côté de 2026-2027 courante).
C'est la source des classes en double et de la période « du jour » ambiguë.

## Changements (`ent-db/src/seed.rs`)
* `seed_labo_cast_au(jour)` : **jour de référence injectable** ; `seed_labo_cast()` l'appelle avec `today_utc()` (jour de
  l'établissement). Le jour est passé jusqu'à `seed_labo_journee_audit`.
* `entree = debut_annee_scolaire(jour)` ; scolarité historique = `annee_relative(annee_scolaire_de(jour), -1)`.
* Fin du semis : l'année antérieure à la rentrée est **close explicitement** (`cloturee_le`), donc plus « préparation ».
* Nuit d'internat de Neville : si le jour injecté diffère du jour réel, la nuit est le jour de référence (la règle « avant
  midi » de `nuit_courante_internat` lit l'horloge réelle et donnait `DateEffetAvantDebut` pour un jour futur).
* Test `tests_semis_relatif` : rejoue le semis aux 5 jours (2026-09-01, 2026-12-20, 2027-06-15, 2027-09-01, 2030-09-01) ;
  l'année du jour existe, aucune année antérieure ne reste en préparation.

## Preuve
| Vérification | Résultat |
|---|---|
| Semis seul aux 5 jours de référence (`tests_semis_relatif`) | ✔ (2027-09-01 échouait avant : `DateEffetAvantDebut`) |
| Sous-ensemble à la date réelle (22-11-2026) : `scenarios`, `tests_structure*`, `tests_evaluations*`, `tests_assiduite_as6`, `tests_referentiel*` | 251/251 ✔ (base neuve `ent_semis_test`) |
| Zéro avertissement rustc (`ent-db`, labo, tests) | ✔ ; clippy non relancé |
| **Suite à base de labo sous horloge simulée (rentrée, 20-12, 15-06, 2027-09-01, 2030-09-01)** | **NON FAITE** |

Raison : cette preuve exige `libfaketime` (binaire de test **et** PostgreSQL, comme `TESTS_JOUR_SEMAINE.md`) ; il n'est pas
installé dans ce conteneur et son installation (`apt-get`) a été refusée par le système de permissions. Le jour injecté
ne déplace pas l'horloge réelle (année « courante » du référentiel, `now()` SQL, `jour_paris()` des tests) : il ne remplace
pas la simulation d'horloge pour les tests.

## Correctif CI (run de 2273adb / ac64c88)
`tests_referentiel_s2::isolation_droits_et_import_mef` : `SELECT id FROM ent_classe … LIMIT 1` pouvait tomber sur une classe de
l'année close (`Conflit` au lieu de `NotFound`) — l'ambiguïté prévue. Recherches restreintes aux années non closes dans ce test
(3 requêtes), `tests_absences_personnels`, `tests_demandes_n1_3` et `tests_cahier` (mêmes motifs) ; assertions inchangées.

## Restes
1. Rejouer la suite (ou au moins `scenarios`, `tests_structure*`, `tests_evaluations*`, `tests_assiduite*`) sous
   `faketime` aux 5 dates ; corriger ce qui rougit sans affaiblir les assertions.
2. Point 4 (dates en dur dans les tests) : recensement seulement — fichiers les plus chargés en `2025-/2026-/2027-` :
   `tests_structure_s3` (107 occurrences), `tests_assiduite_as4` (56), `tests_internat_i2` (42), `tests_notifications_n3` (39),
   `ent-api/tests/separation.rs` (27), `tests_internat_i3` (23). Non audités un par un : à classer entre « borne
   volontaire » (années fixes créées par le test lui-même, sans dépendre de l'horloge) et « dépend du jour ».
   Aussi `cahier_c3.rs` / `edt_notifications.rs` (tests unitaires datés 2026-11-16, purs, sans horloge).
3. Les tests qui cherchent une classe par code sans filtrer l'année pourraient encore voir la classe de l'année close
   (4B, 5A) : l'année close est exclue du cas « courant » mais non vérifié test par test.
