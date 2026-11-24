# Socle 21 — lots V3e « revue de V3d » et V5 « parcours » : livraison

**Date :** 2026-11-22 — **Branche :** `ent-socle-21-code-v5` (depuis `origin/ENT` 02659006) — **PR en brouillon vers `ENT`** : voir la fin.
**Niveau :** *implémenté + testé en local* (PostgreSQL 16 local, base **neuve** `ent_v5_test`, toolchain 1.99.0, `--locked`, `ENT_ENV=test`) ; **CI GitHub : voir la fin** ; **non validé par Florian ; non prêt pilote.**

Faits, hypothèses et décisions sont séparés : les décisions prises sans question au pilote (règle 36 et D-069) sont dans la section 4, chacune avec sa valeur par défaut prudente.

## 1. Partie 1 — V3e (revue de V3d) : fait / non fait

| Point | Réalisation | Test |
|---|---|---|
| **1** motif du zéro et de la note après échéance effacé | migration **477** : `ent_trg_eva_resultat` garde le motif fourni avec l'écriture (`dernier_motif_correction := v_motif`, au lieu de « seulement en correction ») ; `motif_correction` reste une entrée remise à NULL à chaque écriture (un motif ancien ne sert jamais à l'écriture suivante) | `v3d_m4_m5_…` : la tautologie `motif.is_none_or(…)` est **remplacée** par `motif.is_some_and(contient « Absent sans justificatif » et « R-01 »)` ; `v3d_f4_…` étendu : le motif « Convocation reportée par la direction » est lisible au journal, hors correction |
| **2** M-5 hors séance | `eva_v4_attribuer_zero_for_actor` relit `eva_absence_justifiee_ce_jour` (demi-journée justifiée) sous le verrou de la convocation, comme l'assiduité de la séance | `v3e_item2_le_zero_hors_seance_relit_la_demi_journee_justifiee` (aucune séance du cours ce jour-là ; a bloqué, b passe) |
| **3** moyenne générale sans coefficients (bandeau CPE) | **retirée** de `ui.rs` : variable `moyResume`, calcul, suffixe « moyenne générale » du bloc et **deux tuiles « Moy. générale »** (bandeau et synthèse). Aucune extension de `ui.rs` | `grep moyResume` : aucune occurrence ; **l'interface n'a pas été rechargée dans un navigateur** |
| **4** F-4 : impasse après l'échéance en période ouverte | la voie : **prolongation d'échéance tracée**. Repousser une échéance dépassée est réservé à la direction (`notes.corriger`), avec un motif de 3 à 500 caractères ; le journal des convocations garde échéance avant, après, motif et auteur (migration 477) ; un déplacement qui ne change pas l'échéance ne porte aucun motif (jamais périmé) | `v3e_item4_prolongation_d_echeance_tracee_en_periode_ouverte` (l'impasse d'avant est reproduite : ni le professeur ni la direction ne notent ; puis professeur refusé, direction sans motif refusée, motif trop court refusé, prolongation tracée, puis le professeur note) |
| **5** « période du jour » : deux fonctions | **une seule** fonction, `ent_periode_choisir` (plus grand recouvrement, puis la plus précise, puis la plus ancienne : règle totale) ; `eva_periode_eleve_jour`, `ent_periode_de_note_pour_classe` et `eva_periode_dans` la lisent. **Chevauchement : non interdit en base, départage documenté** (voir 4.1). Test web : **dépendance au jour réel NON levée** (voir 3.1) | `v3e_item5_la_periode_d_un_jour_est_unique_et_les_lecteurs_s_accordent` (périodes qui se recouvrent : un jour n'appartient qu'à une seule, les trois lecteurs s'accordent) ; test web : **voir 3.1** |
| **6a** relevé par `Statistiques::pour_famille` | le relevé décide des statistiques montrées hors personnel par `pour_famille` (réglage, seuil **et plancher** d'effectif) au lieu de recalculer le seuil à la main | pas de test nouveau (le comportement des cas courants est couvert par `v3c_m4_…`, étendu en V3d) ; **le cas « seuil du réglage sous le plancher » n'a pas de test dédié** |
| **6b** ordre des contrôles de `charger_convocation` | une convocation **inconnue** répond « droit refusé » à qui ne porte pas `notes.corriger` (comme une convocation d'un autre cours) ; seule la direction apprend « introuvable » | `v3e_item6_une_convocation_inconnue_ne_se_distingue_pas_d_un_droit_refuse` ; **`v3d_m6_…` ajusté** (les trois appels du professeur d'un autre établissement attendent maintenant `Forbidden` ; les deux appels de la direction attendent toujours `NotFound`) |
| **6c** `LIMIT 500` appliqué trop tôt | la lecture des absences répétées lit **un candidat de plus que le plafond** ; au-delà, elle est **refusée** (« préciser le cours ou la période ») au lieu de rendre une liste tronquée ; la justification et la visibilité s'appliquent ensuite à des candidats tous examinés | `v3d_f5_…` (inchangé, vert) ; **le cas « plus de 500 candidats » n'est pas testé** (500 élèves à absences répétées à fabriquer) |
| **6d** table AESH et voie d'effacement RGPD | **documenté** (voir 4.3) : aucune suppression physique par construction ; la voie d'effacement n'existe pas encore dans le code (moteur de conservation du socle 27 : G6, non écrit) ; elle sera une fonction `SECURITY DEFINER` comme `ent_fin_purger_etablissement` (522) | — |

## 2. Partie 2 — V5 « parcours » : fait / non fait

### 2.1 Ce qui a changé dans le moteur (règle 6)

* **La note suit l'élève et la date.** Avant, une note comptait dans la période demandée si « la période de la classe qu'avait l'élève ce jour-là » était cette période : un élève qui passait d'une classe à trimestres à une classe à semestres **perdait** les notes de l'ancienne classe au bulletin du semestre. Maintenant, `eva_periode_dans(période, date)` range la note dans la période **du bulletin** (dans sa périodisation) : jamais perdue, jamais comptée deux fois (un jour n'appartient qu'à une seule période d'une périodisation). Lu par le moteur (`moyennes_tx`) et par le relevé.
* **Un bulletin, une classe (D15).** `eva_classe_rattachement` (une seule implémentation, SQL) dit la classe d'un élève pour une période ; l'effectif d'une classe ne compte l'élève **que dans sa classe de rattachement**. L'ancienne classe ne produit plus de bulletin pour lui (avant : deux bulletins concurrents). `eva_classe_eleve` (Rust) lit la même fonction.
* **CE-03** : un élève parti avant la fin de la période garde sa moyenne et **sort des statistiques** (`partis`, dans `eva_calculer_classe`).
* **Résultats externes (CE-02, CE-06, CE-07)** : tables de 466 enfin servies (aucun service Rust ne les lisait). Un résultat externe **ne compte pas par défaut** ; le compter dans UN cours est une décision tracée (auteur, date, motif) du professeur du cours (titulaire ou co-enseignant, **pas un suppléant**) ou de la direction ; la publier à la famille est **un second geste** ; après l'arrêt de la période de l'élève, **seule la direction** décide ; un retrait est motivé ; **rien ne se modifie ni ne se supprime** (garde en base). La famille voit les résultats **publiés, comptés, non retirés**, avec la mention neutre « évalué par un autre organisme », sans auteur ni motif.
* **Enseignements suivis ailleurs (CE-06, R-V38)** : la direction déclare un enseignement (CNED, conservatoire…) et saisit une moyenne par période ; si elle a déclaré que la moyenne externe vaut moyenne de matière, elle entre dans le moteur (même moteur) **sauf si l'élève a des notes de l'établissement dans la matière** (celles-ci l'emportent, avec un avertissement : jamais de double compte). Une correction de moyenne externe est journalisée en ajout seul.
* **Parcours** (`eva_v5_parcours_for_actor`) : pour le personnel (`notes.lire` sur l'élève, élève visible), classes traversées et classe de rattachement, « arrivé le », « parti le », enseignements suivis avec « abandonné le … » (option quittée, l'élève restant dans sa classe), évaluations de ses cours **concerné ou non à leur date** (jamais un zéro, jamais une absence), résultats externes. Aucun motif d'absence, aucune donnée de santé.
* **Routes** `/eva/v5/*` (10) : voir `ent-api/src/evaluations_v5.rs`. Seule `/eva/v5/eleves/{id}/externes-famille` est ouverte à la famille et à l'élève, sous le lien « voir les notes » ; les neuf autres leur répondent 403.

### 2.2 CE-01 à CE-24

| CE | Statut | Où |
|---|---|---|
| CE-01 arrivée tardive | **testé** : non concerné à la date (ni note ni absence écrites), moyenne sur ses seules évaluations, « arrivé le 4 novembre » au parcours | `v5_ce01_ce04_ce05_…` |
| CE-02 notes de l'établissement d'origine | **testé** : externe non compté par défaut, décision tracée, publication distincte, retrait motivé, droits (autre professeur refusé, cours d'une autre matière refusé, famille refusée), après l'arrêt direction seule | `v5_ce02_ce07_…`, `v5_ce02_apres_l_arret_…` |
| CE-03 élève parti | **testé** : garde sa moyenne, sort des statistiques (effectif 3 → 2), « parti le » ; **« bulletin de départ » sur demande : non ajouté** — la génération d'une classe inclut déjà les élèves partis dans la période (la direction la déclenche à la demande) ; le **dossier de transfert (FX-100)** relève d'un autre socle | `v5_ce03_…` |
| CE-04 une seule note | **testé** (moyenne calculée) ; **le signalement « 1 évaluation » et la marque « non significative » de la direction (R-02) ne sont pas écrits** (écran du conseil et bulletin : socle 23) | `v5_ce01_ce04_ce05_…` |
| CE-05 absent toute la période | **testé** : aucune moyenne, ni de matière ni générale ; **« évaluation de remplacement proposée » : couvert par V4** (CE-29, `v4_…`) | idem |
| CE-06 CNED, hôpital, domicile | **testé** : moyenne externe comptée si la direction l'a décidé, sans double compte ; **la mention « évalué par un autre organisme » est servie pour les résultats externes** (`mention`) ; **elle n'est pas imprimée au bulletin PDF** (le moteur nomme la ligne « Moyenne externe (évalué par un autre organisme) » dans l'explication) | `v5_ce06_…` |
| CE-07 échange à l'étranger | **testé** : exclusion temporaire du cours = non concerné (jamais un zéro), relevé de l'établissement d'accueil versé en externe sans toucher au calcul tant qu'il n'est pas compté | `v5_ce07_ce19_…` |
| CE-08 élève accueilli | **testé** : inclusion sans classe ; aucune moyenne au bulletin, aucune statistique (dans l'effectif d'aucune classe). **Écart à signaler** : sans classe, l'élève n'a pas de période et est **verrouillé** (M8, décision de V1) : le professeur ne peut pas écrire de note pour lui (test), seule la direction par une correction motivée ; ce n'est pas « aucune moyenne au bulletin » mais « pas de saisie ordinaire » | `v5_ce08_…` |
| CE-09 changement de classe | **testé** : mêmes périodes et **périodisations différentes** (trimestres → semestres) : les notes des deux classes comptent, une fois ; l'ancienne classe ne fait pas de bulletin pour lui ; l'origine de chaque note (classe du cours) est au parcours | `v5_ce09_ce10_d15_…`, `v5_ce09_changement_de_periodisation_…` |
| CE-10 changement la veille | **testé** (note du jour de la veille comptée une fois, évaluation de l'ancienne classe le lendemain : non concerné) ; **l'alerte au PP de la nouvelle classe et le droit maintenu des professeurs de l'ancienne classe pour les appréciations (R-V15) ne sont pas écrits** (appréciations : lot V6) | idem |
| CE-11 changement de groupe | **testé** : les deux cours alimentent la moyenne de matière ; chaque professeur n'écrit que dans son cours | `v5_ce11_ce12_…` |
| CE-12 groupes recomposés | **testé** (pas de cas particulier : public daté) | idem |
| CE-13 option abandonnée | **testé** : moyenne sur la période suivie, absente de la suivante, « abandonné le 2 novembre » au parcours ; un **départ de l'établissement n'est pas un abandon** (test) ; **la prise en compte dans la moyenne annuelle (§8.5, deux méthodes) n'est pas écrite ici** | `v5_ce13_ce15_…` |
| CE-14 spécialité changée | **couvert par la même mécanique que CE-13** (public daté par cours) ; **les exports officiels par période (NV) : non faits** | — |
| CE-15 spécialité abandonnée en fin de 1ʳᵉ | **idem** ; l'export LSL : non fait | — |
| CE-16, CE-17 double cursus, SEGPA | **testé** : élève d'une classe de référence (D) inclus dans un cours de la classe A : ses notes comptent, le bulletin est celui de **sa classe de référence** ; **la « rubrique pour le dispositif » et le calendrier visible de l'AESH ne sont pas écrits** | `v5_ce16_ce17_…` |
| CE-18 section sportive, CHAM | **non testé** : cours de groupe à public multi-classes déjà éprouvé par `ce66_…` (V1) ; « matière hors moyenne générale » : réglage de coefficient existant (461), non éprouvé ici | — |
| CE-19, CE-20 PFMP, apprenti | **CE-19 testé** (même mécanique que CE-07) ; **CE-20 (calendrier d'alternance) : non fait** (socle 5) ; CCF : non fait (NV) | `v5_ce07_ce19_…` |
| CE-21 exclusion temporaire le jour d'un devoir | **non fait** : la justification « exclusion » est **lue au socle 13**, qui n'est pas lu par `eva_absence_a_la_seance` ; l'absence reste « jamais un zéro » (V4, CE-25/26) | — |
| CE-22 fusion de deux fiches | **non fait en V5** : `eva_resultat_externe` reste « reste » (les résultats restent sur la fiche d'origine, lisibles via la fusion, comme pour `eva_resultat`) ; le **conflit « doublon à trancher »** n'est pas écrit | — |
| CE-23 élève décédé | **non fait** : gel des avis (socle 8, CN-13) | — |
| CE-24 majeur qui retire l'accès | **testé** : après le retrait du lien « voir les notes », la famille ne lit plus ni moyennes ni résultats externes (403) ; **« ce qui a déjà été remis reste remis »** : les bulletins déjà générés ne sont pas touchés (aucune écriture) mais **non testé** ; la date d'effet (« à partir de la date ») dépend du socle 4 | `v5_ce24_…` |

### 2.3 Droits (règle 5 : personnel → droit → visibilité → existence)

Déclarer un résultat externe : `notes.saisir` ou `notes.corriger` (établissement), **ou** enseigner un cours dont le public du jour compte l'élève, **ou** une portée désignée du socle 3 ; élève protégé : introuvable. Compter ou publier : professeur du cours (pas un suppléant) ou direction. Moyennes externes : direction. Lectures : `notes.lire` sur l'élève ; famille : `peut_voir_notes`. Une ligne inconnue répond « droit refusé » sauf à la direction.

## 3. Tests et vérifications

Base locale PostgreSQL 16 **neuve** (nom contenant « test » : `ent_v5_test`), un seul `CARGO_TARGET_DIR`, toolchain 1.99.0, `--locked`, `ENT_ENV=test`. Aucune heure réelle, aucun jour UTC dans les tests ajoutés (jour fixe 2031-10-20 du monde V1).
* **Nouveaux tests : 19** — `tests_evaluations_v5` : 14 (« quitte puis revient » ajouté ; CE-01/04/05, CE-02/07, CE-02 après l'arrêt, CE-06, CE-03, CE-09/10/D15, CE-09 périodisations différentes, CE-11/12, CE-13/15, CE-16/17, CE-08, CE-07/19, CE-24) ; `tests_evaluations_v4` : 4 (`v3e_item2`, `v3e_item4`, `v3e_item5`, `v3e_item6`) ; `ent-api/tests/separation.rs` : 1 (10 routes `/eva/v5/*` × 4 acteurs : 403) ; **étendus** : `v3d_m4_m5`, `v3d_f4` ; **ajusté** : `v3d_m6` (voir 1, point 6b).
* **Exécutés en local, verts :** `tests_evaluations_v5` (13/13), les quatre `v3e_*` et `v3d_m6` ; puis la suite `tests_evaluations*`, `tests_cahier*`, `tests_eleve_fusion`, `migrate::`, `registre_donnees`, `tests_rls::`, `tests_agenda*`, `tests_notifications*` : **304 tests, 302 verts au premier passage** ; les deux échecs : `v3d_m6` (attente à ajuster, voir ci-dessus, vert ensuite) et `e2_aucune_variable_de_session…` (**échec de l'amorçage du jeu d'essai, `Reessayer`, sans lien avec le lot ; vert à la relance**). **Séparation HTTP** : voir la fin.
* **Relèvent de la CI (règle 36)** : la suite complète d'`ent-db` (≈ 2 480 tests), les tranches de RLS stricte, la matrice de séparation complète, les tests du frontend hors `evaluations.rs`.
* **Clippy (commandes de la CI, `-D warnings`)** : `-p ent-db -p ent-types -p domain-eleve -p ent-api --all-targets` **0 avertissement** ; avec `--features ent-db/labo,ent-api/labo` **0**. `rustfmt` **ciblé** sur les fichiers nouveaux et sur `evaluations_v4.rs`, `tests_evaluations_v4.rs` (propres avant et après) ; `evaluations.rs`, `evaluations_v1.rs` et `separation.rs` ne sont **pas** reformatés (ils ne l'étaient déjà pas avant le lot, `cargo fmt --all` interdit).
* **Frontend** : clippy 0 avertissement (avec et sans `labo`), `cargo metadata --locked` vert ; aucun fichier de `web/` modifié.
* **Non testé, dit franchement :** (a) l'interface JavaScript (`ui.rs`) n'a pas été rechargée dans un navigateur ; (b) le cas « plus de 500 candidats » des absences répétées ; (c) le relevé sous le plancher d'effectif ; (d) l'impression PDF d'un bulletin d'élève muté ; (e) aucune route V5 n'a de test HTTP de succès (seul le 403 de la famille est éprouvé en HTTP ; les services le sont en base) ; (f) la concurrence de deux décisions « compter » (le `UPDATE … WHERE compte_dans_cours_id IS NULL` la tranche, non éprouvé en parallèle).

### 3.1 Test web `periode_complete_avec_le_client_du_frontend` — NON fait

Essayé : fixer la date de référence à « rentrée + 30 jours ». **Échec constaté en local** (`période arrêtée : seule la direction corrige…`) : les classes du labo sont déjà arrêtées par l'arrêt automatique selon le jour où la base est préparée ; une date fixe tombe donc dans une période arrêtée. Le changement a été **annulé** : le test est **inchangé** (il reste à l'horloge réelle, avec le départage de V3d). La cause réelle de l'intermittence (deux fonctions de période qui divergeaient) est, elle, corrigée en base et éprouvée par `v3e_item5_…`. Pour rendre ce test indépendant du jour, il faut rouvrir explicitement la période de la classe du test (service `eva_v1_changer_etat_periode_for_actor`) à la date choisie, ce qui demande d'exposer la période de l'élève à ce test web : **non fait**. Exécuté en local sur le code inchangé : voir ci-dessous la CI. Le frontend n'a par ailleurs pas été modifié (clippy frontend 0 avertissement, `cargo metadata --locked` vert) ; l'édition du test web est annulée, aucun fichier `web/` n'est modifié.

## 4. Décisions (valeurs par défaut prudentes, D-069)

1. **Chevauchement de périodes : départage, pas contrainte.** La contrainte d'exclusion de 335 ne vise que l'origine `service`. En poser une sur la périodisation historique romprait des données existantes que nous n'avons pas le droit de consulter (production). Le départage (recouvrement, puis la plus précise, puis la plus ancienne) est **total** et documenté (`MIGRATIONS.md`, 477). *À revoir* : quand une revue des périodes de production sera faite par le pilote, une contrainte pourra être posée.
2. **Une moyenne externe ne s'ajoute pas à des notes de l'établissement dans la même matière** : celles-ci l'emportent, avec avertissement. (Règle métier incertaine : R-V38 ne tranche pas ; valeur prudente = ne jamais compter deux fois.)
3. **Voie d'effacement RGPD.** Les tables d'ajout seul (`eva_resultat_historique`, `eva_rattrapage_historique`, `eva_dispense_port`, `eva_aesh_indisponibilite_port` depuis 476, `eva_moyenne_externe_historique`, `eva_resultat_externe`) **refusent UPDATE, DELETE et TRUNCATE** : c'est voulu (traçabilité). L'effacement d'un élève passera par l'**anonymisation de la fiche** (socle 2) et, pour l'effacement d'un établissement, par une fonction `SECURITY DEFINER` qui désactive les gardes le temps de sa transaction (modèle : `ent_fin_purger_etablissement`, 522). **Le moteur de conservation du socle 27 (G6) n'existe pas** : aucune durée n'est appliquée aujourd'hui ; les tables sont classées « données personnelles » au registre (`perso`). *Rien de ceci n'est implémenté par ce lot* : seule la documentation l'est.
4. **Famille et moyenne externe** : ~~la moyenne externe était servie à la famille dès sa saisie~~ — **corrigé par la revue (E1)** : une moyenne externe, comme un résultat externe, n'existe pour la famille et l'élève qu'après une **publication** explicite de la direction (`publiee_le`, `publiee_par`). Voir la section 7.
5. **Élève sans classe (CE-08)** : verrouillé (M8) ; décision de V1 conservée.

## 5. Compteurs (règle 28) et état

Recalculés à partir de la base commune `02659006` : **+1 table avec RLS** (`tests_rls.rs`) ; registre des données **+1** ; `TABLES_ELEVE` inchangé ; `Action::TOUTES` inchangé ; sources de routes **69 → 70** (`SOURCES` et `NOMS_SOURCES` de `routes_src.rs`) ; liste de `migrate.rs` **+1** (477) ; `MIGRATION_TAG` inchangé ; migrations 478–479 réservées ; plage non pleine (aucune plage nouvelle). Aucune migration versée modifiée. Aucune dépendance ajoutée (`cargo metadata --locked` dans `ent-rust/web` : voir la fin).

## 6. Fin : tête, CI

**Tête finale :** `15b277e1` sur `ent-socle-21-code-v5` ; **PR en brouillon n° 114 vers `ENT`**.
**CI :** à la clôture du lot, le run « ENT Rust » de cette tête était encore **en file d'attente** (`queued`, plusieurs dizaines de minutes) : **aucun résultat de CI à rapporter**. Le pilote lit le résultat sur la PR 114. Rien n'est affirmé vert côté CI.
**À surveiller en CI :** `periode_complete_avec_le_client_du_frontend` (non modifié, voir 3.1) ; la suite complète d'`ent-db` (non exécutée localement).

## 7. Correctifs de revue (revue du pilote, branche `ent-socle-21-code-v5`, PR 114)

**Niveau :** implémenté + testé en local (PostgreSQL 16, base neuve `ent_v5r_test`, toolchain 1.99.0, `--locked`, `ENT_ENV=test`). **CI : voir la fin. Non validé par Florian ; non prêt pilote.** La migration **477 (jamais versée) est modifiée sur place** ; aucune autre migration ne change ; 478–479 restent libres. `origin/ENT` fusionné : déjà à jour au départ (02659006).

### 7.1 Fait / non fait

| Point | Fait | Test |
|---|---|---|
| **E1** la moyenne externe entrait dans la moyenne de la famille sans publication | `eva_moyenne_externe.publiee_le/par` (477) ; `moyennes_tx` n'y lit que les moyennes publiées quand `publiees_seulement` ; service `eva_v5_publier_moyenne_externe_for_actor` (direction, une fois, seulement une `moyenne`) ; route `POST /eva/v5/moyennes-externes/{id}/publier` | `revue_e1_m3_m5_…` : famille avant/après publication (moyenne et service famille), professeur refusé, ligne inconnue (403 / 404 direction), double publication (conflit) |
| **M1** garde de période de `publier` et `retirer` | une fonction commune `eva_v5_garde_periode`, la même que pour `compter` : période de l'élève ouverte, sinon direction seule. `retirer` d'un résultat **non compté** (aucune moyenne touchée) reste à l'auteur | `revue_m1_…` (professeur refusé après l'arrêt, direction passe) |
| **M2** moindre privilège | `Droit { corriger, large }` : portée établissement/direction = tout ; sinon **cours et matières que le professeur enseigne** (`ent_prof_sur_cours`) pour `externes`, `enseignements-externes`, `parcours` (enseignements, évaluations, résultats externes) et pour déclarer (matière enseignée). `dispense` (moyenne externe et statut de résultat) et motifs (`compte_motif`, `motif_retrait`) servis à la direction seule | `revue_m2_…` (deux professeurs, direction, déclaration hors matière refusée) |
| **M3** correction d'une moyenne externe | motif 3–500 obligatoire quand une valeur change (service **et** déclencheur) ; geste sans changement = aucune écriture ni journal (`RETURN NULL`, avant : une ligne de journal portant l'ancien auteur) ; journal `evenement` (saisie / correction / publication), `motif`, `document_avant/apres`, auteur exact | `revue_e1_m3_m5_…` (motif absent/court/long, geste sans effet, journal complet) |
| **M4** moteur | présence testée sur les notes de l'établissement (et résultats externes comptés), jamais sur une autre moyenne externe ; **plusieurs moyennes externes d'une même matière dans la période du bulletin : aucune n'est comptée** (défaut prudent, la direction n'en garde qu'une) ; avertissements exacts qui nomment la matière | `revue_m4_…` |
| **M5** pièces | `eva_document_de_l_eleve` : la pièce doit exister dans `ent_doc_document` pour **cet établissement et cet élève** ; vérifiée par le service (message clair) et par les déclencheurs de `eva_resultat_externe` et `eva_moyenne_externe`. Clé composite : l'élève n'est pas dans la clé du socle 7, donc contrôle par fonction | `revue_e1_m3_m5_…` (pièce d'un autre élève, inconnue, INSERT direct refusé) |
| **F1** ordre des contrôles de `compter` | un cours inconnu répond « droit refusé » à qui ne porte pas `notes.corriger` (avant : « introuvable », oracle d'existence) | `revue_m2_…` |
| **F2** plafond des absences répétées | lecture **par lots** (pagination par élève) ; justification et visibilité s'appliquent à chaque lot ; refus seulement si la liste **filtrée** dépasse le plafond (ou 20 lots) | `revue_f2_…` (plafond réduit à 2 par `eva_v4_absences_repetees_plafonnees`) |
| **F3** période d'une moyenne externe | de l'année et de la périodisation d'une classe de l'élève, dans les dates de l'enseignement | `revue_f3_f4_…` |
| **F4** concurrence | `ON CONFLICT DO NOTHING` → conflit propre, jamais de 500 | `revue_f3_f4_…` (deux saisies simultanées) |
| **F5** HTTP | séparation entre établissements (A→B et B→A, 11 routes), portée du professeur, **succès de bout en bout** de `externes-famille` (déclarer, compter, publier, lecture famille sans motif ; non publié = rien) ; le test des 11 routes en 403 famille/élève inclut la nouvelle route | 3 tests ajoutés + 1 étendu dans `separation.rs` |
| **H1** période annuelle chevauchante | **vraie** : `eva_periode_dans` ne rendait jamais vrai pour une période qui englobe le trimestre choisi → bulletin annuel sans note. Corrigé : une période de nature `periode` qui englobe strictement la période choisie (trimestre/semestre) porte aussi la note ; deux trimestres qui se recouvrent gardent un seul propriétaire (V3e-5 inchangé). *Réserve* : en service, les périodes d'une périodisation ne se chevauchent pas (exclusion de 335) ; le cas ne concerne que les périodes `reprise` | `revue_h1_…` |
| **H2** fusion de fiches | **vraie** : un résultat externe compté restait sur la fiche absorbée et sortait de la moyenne. `eva_fiches_de_l_eleve` (élève + fiches absorbées) lue par le moteur, les lectures et la famille. **Non fait** : même rattachement pour `eva_resultat` ordinaires (limite connue de V3, registre de fusion) | `revue_h2_…` |
| **H3** reconvocation | **vraie** : constater une absence **justifiée** après l'échéance (sans motif, par le professeur) ouvrait la reconvocation, donc une nouvelle échéance sans la direction. Désormais direction + motif tracé ; de même, **annuler puis re-planifier** après l'échéance de l'annulée exige direction + motif (`PlanifierRattrapageBody.motif`, journal) | `revue_h3_…` (deux tests) |
| **H4** `GRANT` de la table historique | **fausse** : aucune migration du dépôt ne pose de `GRANT` ; les tables appartiennent au rôle de l'application, sous RLS forcée (`ent_rls_poser`), comme `eva_resultat_historique` | lecture des migrations ; RLS couverte par `tests_rls` |

**Décisions (D-069, valeurs prudentes)** : (1) plusieurs moyennes externes d'une matière sur la période : aucune comptée, avertissement, la direction tranche (non paramétrable pour l'instant) ; (2) la moyenne externe n'est jamais publiée par la saisie ; une correction garde l'état de publication, le motif trace ce que la famille voit ; (3) un professeur dont le droit vient d'une portée « désignée » (hors établissement) est traité comme le professeur : limité à ses cours.
**Non fait / limites** : interface `ui.rs` non touchée (aucune UI pour publier une moyenne externe) ; la « garde de période » d'une correction de moyenne externe est inhérente (direction seule) ; la lecture des matières d'un professeur se fait au jour de référence ; `eva_resultat` ordinaires non rattachés aux fiches absorbées.

### 7.2 Tests et compteurs
* **13 tests ajoutés** : `tests_evaluations_v5` 7, `tests_evaluations_v4` 3, `ent-api/tests/separation.rs` 3 (+ 1 étendu : 10 → 11 routes). Ajustés : le test CE-06 (motif de correction).
* **Locaux, verts** : 77/77 (`tests_evaluations_v5`, `revue_`, `v3e_`, `v3d_`) ; 174/174 (`tests_evaluations*`, `migrate::`, `registre_donnees`, `tests_rls::`, `tests_eleve_fusion`, `tests_d036`) ; séparation `evaluations_v5` (4/4) et inventaire des routes. **Relèvent de la CI** : la suite complète d'`ent-db` et la matrice de séparation complète.
* **Compteurs (règle 28)** : aucune table nouvelle, aucune action, `TABLES_ELEVE` inchangé ; sources de routes **70** (même fichier, 11 routes) ; liste de `migrate.rs` inchangée (477 déjà listée) ; `MIGRATION_TAG` inchangé.

### 7.3 Échec de CI 45008987285 (tranche frontend 2/6) : `periode_complete_avec_le_client_du_frontend`

**Symptôme.** La saisie du professeur revenait `issue: "refuse"`, « période arrêtée : seule la direction corrige, avec un motif écrit », au lieu de `"ecrit"`. Rejoué en local (base neuve `ent_v5_test`, 1.99.0, `--locked`) : même échec.

**Cause (deux défauts, un seul vrai).**
1. **Le test était faux (règle 33).** Il affectait l'élève à la classe avec `referentiel_affecter_eleve_classe(…, None)`, c'est-à-dire **à la date réelle du jour** (bornée à l'année), alors que les évaluations sont datées « rentrée + 30 jours » depuis V3e-5. Dès que le jour réel dépasse cette date, l'élève n'est pas dans la classe à la date de l'évaluation : `eva_periode_eleve` ne rend aucune période et `ecrire_resultat_tx` le verrouille comme un élève arrêté (M8, voulu). Le professeur a bien le droit sur le cours (il est titulaire) : **ce n'est pas un correctif de portée de la revue**, l'hypothèse du pilote sur H1-H3/M1-M5 est écartée. Le test dépendait donc du jour où la CI tourne. Corrigé : affectation à `annee.date_debut`.
2. **Le message était faux.** `eva_v3_une_case` traduisait tout `EntDbError::Forbidden` en « période arrêtée », quelle que soit la règle qui refusait (trois règles distinctes dans `ecrire_resultat_tx`). Ici la période était **ouverte** et le message disait le contraire.

**Correctif du code.** `CauseRefusEcriture` (`service.rs`, exportée) et la variante interne `EntDbError::RefusDroitEcriture(cause)` : `ecrire_resultat_cause_tx` (nouvelle, corps de l'ancienne `ecrire_resultat_tx`) rend la cause ; `ecrire_resultat_tx` devient un mince enrobage qui la rend en `Forbidden` : **aucun autre appelant ne change** (écriture simple, lots V4, HTTP 403 inchangé). Seule la saisie en lot V3 présente la cause, case par case :

| Cause | Règle | Message |
|---|---|---|
| `HorsDeVosCours` | ni titulaire/co-enseignant/suppléant en saisie, ni `notes.corriger` | « ce cours n'est pas l'un de vos cours : vous ne pouvez pas y saisir de résultat » |
| `PeriodeArretee` | période de la classe de l'élève arrêtée/fermée, sans droit de correction | « période arrêtée : seule la direction corrige, avec un motif écrit » |
| `EleveSansPeriode` | élève sans classe ni période à la date de l'évaluation (M8) | « cet élève n'a ni classe ni période à la date de l'évaluation : saisie impossible (vérifiez son affectation à la classe à cette date) » |
| `SaisieReserveeAuProfesseur` | période ouverte, porteur de `notes.corriger` hors du cours | « période ouverte : la saisie revient au professeur du cours ; la direction corrige après l'arrêt de la période, avec un motif écrit » |

Un `Forbidden` inattendu garde un message de droit (« droit insuffisant pour saisir ce résultat »), jamais « période arrêtée ».

**Tests.** `v3_le_message_d_un_refus_de_saisie_nomme_sa_vraie_cause` (élève sans classe à la date → `EleveSansPeriode`, jamais « période arrêtée » ; période réellement arrêtée → `PeriodeArretee`) ; `v3_ecriture_refus_de_droit_hors_de_vos_cours_et_saisie_reservee_au_professeur` (les deux autres causes, la voie simple rend toujours `Forbidden`, quatre messages distincts et seul `PeriodeArretee` dit « période arrêtée »). Les deux dernières causes sont une défense en profondeur sous le contrôle d'accès du lot : testées sur l'écriture elle-même.

**Vérifications locales (aucune CI avant le push).** `ent-web-server` + `ent-web-session`, toute la suite avec `labo` et `LEPTOS_OUTPUT_NAME=ent-web` (comme la CI) : **verte** (`evaluations` 2/2, `contre_backend` 9/9, `securite` 2/2, les autres cibles vertes ; aucun autre test rouge de même cause). `ent-db` : **176/176** (`tests_evaluations*`, `migrate::`, `registre_donnees`, `tests_rls::`, `tests_eleve_fusion`, `tests_d036`, dont les 2 nouveaux). Séparation HTTP `evaluations_v5` **4/4**. Clippy 1.99.0 `--all-targets` (backend avec labo ; frontend `-D warnings`) : **0 avertissement**. `rustfmt` ciblé sur le test web ; `cargo metadata --locked` dans `web/` : OK.

**Fusion d'`origin/ENT`** (5 commits : semis de labo relatif, `tests_semis_relatif`) : aucun conflit ; **compteurs (règle 28)** recalculés depuis la base commune 02659006 : aucune action, aucune table, aucune migration, `MIGRATION_TAG` inchangé ; seul ajout de ligne dans `lib.rs` côté ENT (`mod tests_semis_relatif`), indépendant des nôtres.

**Alerte RUSTSEC-2026-0592.** `origin/ENT` portait encore `url` 2.5.8 : correctif de la PR 121 appliqué à l'identique (`cargo +1.99.0 update -p url --precise 2.5.9` dans `ent-rust` et `ent-rust/web` : `url` 2.5.9, `form_urlencoded` 1.2.3, rien d'autre dans les deux `Cargo.lock`).

**Tête et CI :** tête du code et de la correction `947ceb8c` ; **CI « ENT Rust » verte** (run 45029868708 : tous les jobs réussis, un seul sauté par conception — la RLS stricte ne tourne que sur `ENT`, règle 27 ; aucun rejeu nécessaire). La PR 114 reste en brouillon. Niveau : implémenté + testé (local et CI) ; **non validé par Florian, non prêt pilote**. Le présent complément de documentation est poussé dans un commit suivant (documentation seule).

## 8. Suites de la contre-revue (branche `ent-socle-21-code-v5-suites`, depuis `origin/ENT` bfc95c00, PR 125 en brouillon)

**Niveau :** implémenté + testé en local (PostgreSQL 16, base neuve `ent_v5s_test`, toolchain 1.99.0, `--locked`, `ENT_ENV=test`). **CI : voir la fin. Non validé par Florian ; non prêt pilote.** La migration 477 (versée) n'est **pas** modifiée : tout le SQL est dans la nouvelle **478** `eva_v5_contre_revue_publication` (479 reste réservée).

### 8.1 Fait / non fait

| Point | Réalisation | Test |
|---|---|---|
| **M-A** (D-069) | une correction d'une moyenne externe **publiée** la remet en « non publiée » (déclencheur, `publiee_le/par` à NULL) : la famille ne voit plus rien jusqu'à la nouvelle publication ; événement « correction » avec `etait_publiee = true` et le motif. Route `POST /eva/v5/moyennes-externes/{id}/retirer-publication` (direction, motif 3–500, événement « retrait », journal) ; professeur : 403 ; ligne inconnue : droit refusé / introuvable (direction) ; double retrait : conflit | `contre_revue_ma_…` ; `revue_e1_…` adapté (la correction ne laisse plus la moyenne visible) |
| **M-B** | `moyennes_tx` : « notes d'établissement présentes » (ordinaires et résultats externes comptés) et « plusieurs moyennes externes » se décident sur **toutes** les lignes, publiées ou non ; seul l'affichage est filtré. Pour la famille, l'avertissement est neutre (« pas comptée pour le moment ») : ni nombre ni existence de ce qui n'est pas publié | `contre_revue_mb_…` (deux externes dont une publiée ; notes non publiées + externe publiée) |
| **M-C** | résultats ordinaires lus par `eva_fiches_de_l_eleve` (la note suit l'élève conservé) ; si la fiche conservée a un résultat pour la même évaluation, celui de l'absorbée ne compte pas ; annonce de `domain-eleve/src/fusion.rs` corrigée. **Fusion protégée** : une absorbée protégée vers une conservée non protégée est **introuvable** (aperçu et fusion) pour qui n'a pas G6 (`verrouiller_paire`) ; avec G6 elle est permise et la mesure suit la fiche conservée (test existant) | `contre_revue_mc_…` ; `fiche_absorbee_protegee_vers_conservee_non_protegee_refusee_sans_g6` (compte de comptabilité + rôle « fusionner » sans G6) |
| **M-D** | test HTTP `evaluations_v5_le_professeur_n_a_que_sa_portee_sur_les_routes` **réécrit avec données réelles** (résultat externe, enseignement externe, moyenne, dans une matière qu'il n'enseigne pas) et **absence vérifiée dans le corps** ; test d'un personnel à portée **désignée** sans cours ; test d'un résultat `dispense` dans le parcours (masqué au professeur, servi à la direction) | `contre_revue_md_…` ; test HTTP |
| **F-1** | parcours du professeur : segments de classe, rattachement, arrivée et départ **limités aux classes où il enseigne et à la période de ses cours** (segments bornés à cette fenêtre) | `contre_revue_f1_…` |
| **F-2** | un assistant ou un intervenant (saisie non effective) **ne déclare pas** de résultat externe (lecture seule conservée) ; un professeur ne déclare pas pour une **année précédente** (direction seule) | `contre_revue_f2_…` |
| **F-3** | `publier` d'une moyenne : seule une ligne `statut = 'moyenne'`, vérifié **en base** (déclencheur + contrainte) | `contre_revue_ma_…` (UPDATE direct refusé) |
| **F-5** | jour de référence fixe : tests HTTP touchés (`2020-10-10`), aucun `jour_paris()` ajouté | — |
| **F-6** | **la publication n'a pas d'écran** : API seule (résultat externe, moyenne externe, retrait). Ajoutée au reste à faire ci-dessous | — |

**Décisions (D-069, valeurs prudentes).** (1) Une correction d'une moyenne publiée **ne publie rien** : la direction republie après vérification (second geste). (2) **Personnel à portée désignée sans cours** (vie scolaire, CPE de niveau) : autorisé (200) mais **parcours vide** (aucun segment, enseignement, évaluation ni résultat externe) ; le parcours reste un outil pédagogique, la vie scolaire s'informe par ses propres vues ; à rouvrir si la vie scolaire demande les segments de classe. (3) Un cours de groupe sans classe ne justifie aucun segment pour un professeur.

**Reste à faire / limites.** Écrans (aucune UI) pour publier ou retirer une publication, corriger une moyenne externe, consulter le journal ; le journal des moyennes n'est lu par aucune route ; les évaluations du parcours (`eva_resultat`) ne suivent pas encore la fiche conservée (seul le moteur de moyennes le fait) ; deux fiches absorbées portant chacune un résultat pour la même évaluation comptent toutes deux ; le test du personnel désigné suppose l'affectation ouverte à la date réelle (affectation datée de 2020), car le droit désigné se calcule au jour réel de l'établissement ; clippy du frontend non relancé (aucun fichier de `web/` ni d'`ent-types` modifié ; `cargo metadata --locked` dans `web/` : vert).

### 8.2 Tests et compteurs
* **Nouveaux : 8** — `tests_evaluations_v5` : `contre_revue_ma`, `_mb`, `_mc`, `_f1`, `_f2`, `_md` (6) ; `tests_eleve_fusion` : 1 ; `separation.rs` : 1 réécrit (+ le test des 12 routes en 403 et celui de la séparation entre établissements étendus à la nouvelle route). **Ajusté :** `revue_e1_m3_m5_…`.
* **Locaux, verts :** `tests_evaluations_v5` 27/27 ; `tests_evaluations*`, `migrate::`, `registre_donnees`, `tests_rls::`, `tests_eleve_fusion`, `tests_d036` **183/183** ; séparation `evaluations_v5` **4/4**. Clippy 1.99.0 `-D warnings` : `--workspace --all-targets` (sans et avec `ent-db/labo,ent-api/labo`) et `-p ent-types` : **0 avertissement**. `rustfmt` ciblé sur les fichiers touchés ; `cargo fmt --all` non utilisé. **Relèvent de la CI :** suite complète d'`ent-db`, matrice de séparation complète, frontend.
* **Compteurs (règle 28)**, base `bfc95c00` : aucune table, aucune action, `TABLES_ELEVE` inchangé, registre inchangé, sources de routes 70 (12 routes dans `evaluations_v5.rs`), `migrate.rs` +1 (478), `MIGRATION_TAG` inchangé.

### 8.3 Tête et CI
Branche `ent-socle-21-code-v5-suites`, PR brouillon #125 vers `ENT`. Tête de code testée : `9ba9fd67` ; CI (run 45061808763) : verte (success). Le run précédent (6a848cfc) a été annulé par le push suivant. Les commits de documentation qui suivent ne changent pas le code.

## 9. Passe 2 (correctifs de la contre-revue, décisions D-069 du pilote ; branche `ent-socle-21-code-v5-suites`, depuis fe33a3ac + fusion d'`origin/ENT`)

**Niveau :** implémenté + testé en local (PostgreSQL 16, base neuve `ent_test_p2`, toolchain 1.99.0, `--locked`, `ENT_ENV=test`, un seul `CARGO_TARGET_DIR`). Non validé par Florian ; non prêt pilote. **478 n'est pas versée : elle est modifiée ; 479 (V13) et toute autre migration versée : intactes.**

### 9.1 Fait / non fait

| Point | Réalisation | Test |
|---|---|---|
| **M1** (oracle côté famille) | `moyennes_tx` : le moteur est le même pour tous, mais la famille et l'élève ne **voient que les lignes publiées** (notes d'établissement, moyennes externes, résultats externes). La moyenne externe publiée reste visible tant que rien de **publié** ne la remplace ; plus aucun avertissement côté famille sur ce qui n'est pas publié. Côté personnel : avertissement « la famille voit aujourd'hui la moyenne externe publiée ; la publication d'une note (ou d'une seconde moyenne externe) la masquera ». Remplace la décision de M-B (section 8). | `contre_revue_m1_…` : famille avant/après la saisie d'une note non publiée → vues **égales** et sans avertissement ; après publication de la note → moyenne externe masquée (la note compte) ; avertissement personnel présent ; cas de deux moyennes externes dont une publiée |
| **M2** (compare-et-écrit) | `publier` et `retirer-publication` portent la **version vue** (`vue` : statut, valeur, barème) comparée dans le `WHERE` ; écart → **409**, aucune écriture. Les deux corps JSON changent (`vue` obligatoire). | `contre_revue_m2_…` (A lit v1, B corrige en v2, la publication de A est refusée ; retrait sur version périmée refusé) ; HTTP : 409 dans `evaluations_v5_le_professeur_…` |
| **M3** (478) | contrainte `…_publication_statut_chk` posée **`NOT VALID`** ; lignes publiées hors « moyenne » (corrigées sous 477) **dépubliées** par 478 via le déclencheur (journal : « retrait », motif « correction antérieure à 478 ») ; contrainte validée ensuite ; ajoutée à `CONTRAINTES_A_VALIDER` (constat `tracing::error!` au démarrage si elle restait `NOT VALID`). Aucune valeur modifiée. | `contre_revue_m3_…` (geste rejoué, trace au journal) ; `tests_eleve_e9` (constat : liste vide, contrainte validée) |
| **Règle 33** | nouvelle `autoriser_eleve_le(…, jour)` ; `autoriser_eleve` la délègue avec le jour de l'établissement ; `eva_v5_droit_eleve` passe son `jour` (branche « portée désignée »). Gestes de direction sans jour métier (`SANS_JOUR`) : jour de l'établissement, comme avant. Le commentaire « aucune horloge » est vrai. | test CPE à portée désignée : jour fixé (2031-10-20), affectation **bornée explicitement** (2031-09-01 → 2031-12-31) |
| **F-1** | fenêtre **par classe** (`classe_id`, plus `classe_code`) et bornée par la période d'enseignement **réelle** du professeur dans la classe (présence de l'élève ∩ cours ∩ intervenance `du/au`), plus l'enveloppe de tous ses cours | `contre_revue_f1_la_fenetre_…` : un passage de la classe A avant l'enseignement du professeur dans A n'est pas vu, le suivant est tronqué à la date de début |
| **Tests faibles** | branche `sans` du test F-1 : refus exact (`Forbidden`) ; test HTTP : la direction saisit une moyenne réelle, la publie, **retire la publication (2xx)** ; le commentaire de `separation.rs` est donc vrai ; `contre_revue_mb` (devenu `m1`) asserte l'absence d'avertissement famille dans le cas b | — |
| **Retrait et `saisi_par`** | le retrait n'écrase plus `saisi_par` ; l'auteur du retrait est dans `retrait_par` / `dernier_retrait_par` (478) et dans le journal | `contre_revue_ma_…` (journal : auteur de chaque événement) |
| **Autres faibles** | **Non faits, volontairement :** (a) couplage `saisie = !NotesLire` dans `eva_v5_droit_eleve` : un paramètre explicite touche tous les appelants (hors périmètre de cette passe, aucun défaut) ; (b) lignes de journal antérieures avec `etait_publiee = false` : précision perdue, pas une erreur, rien à rejouer ; (c) usages de `jour_paris()` préexistants (`separation.rs` ~165, ~2417) : hors des tests touchés, à traiter par le lot qui les possède. | — |

### 9.2 Tests, compteurs
* **Nouveaux :** `contre_revue_m1` (remplace `_mb`), `contre_revue_m2`, `contre_revue_m3`, `contre_revue_f1_la_fenetre_…` ; adaptés : tous les appels de publication/retrait de moyenne (helpers `pub_moy`, `ret_moy`), `contre_revue_md` (jour fixé), `contre_revue_f1` (refus exact), `separation.rs` (corps `vue`, succès direction).
* **Compteurs (règle 28)** recalculés depuis la base commune : **0 table**, `tests_rls.rs` inchangé, registre des données inchangé, `TABLES_ELEVE` inchangé, `Action::TOUTES` inchangé, sources de routes 70 (12 routes), liste de `migrate.rs` +1 (478, en dernier), `CONTRAINTES_A_VALIDER` 2 → 3, `MIGRATION_TAG` inchangé. Fusion d'`origin/ENT` sans conflit.
* **Locaux, verts :** `tests_evaluations_v5` + `tests_eleve_e9` + `tests_eleve_fusion` + `migrate::` **62/62 au premier passage hors 2 de mes nouveaux tests, corrigés** (assertion trop stricte sur un avertissement de coefficient, intervention immuable à annuler puis recréer) ; puis `tests_evaluations*`, `tests_droits`, `droits`, `registre_donnees`, `tests_rls::`, `migrate::` **263/263** ; séparation `evaluations_v5` **4/4**. Clippy 1.99.0 : `ent-db`, `ent-api`, `ent-types` (`--tests`, `labo`) **0 avertissement** ; rustfmt ciblé sur les fichiers déjà conformes (`evaluations_v5.rs` ×2, tests, types) ; `cargo metadata --locked` dans `ent-rust/web` : OK.
* **Non exécuté en local :** les tests du frontend contre le backend (`ent-web-server`, `--features labo`, `evaluations.rs`, `grille_v3.rs`) : l'édition de liens du binaire échoue ici (`rust-lld` : relocations de `ring`, puis `bfd`/`gold` tués faute de mémoire). Ils sont laissés à la CI. Aucun code du frontend ne change ; seul `ent-types` gagne trois structures.

**Passe 2 — tête et CI.** Tête de code : `4965ab8a` (fusion d'`origin/ENT` a39f0892 ; règle 28 : 478 ajoute 0 table, `CONTRAINTES_A_VALIDER` = 3). CI « ENT Rust » : **verte** sur cette tête (run 45143473378). Les tests `ent-web-server` (evaluations, grille_v3) n'ont été exécutés que par la CI (édition de liens impossible localement faute de mémoire).

### Correctif du pilote après contre-revue de la passe 2 (2026-11-23)
- **Correctif (moyen 1 de la contre-revue)** :
  - 478 dépublie aussi une moyenne de statut « moyenne » corrigée **après** sa publication sous 477, d'après la date de la correction au journal (`eva_moyenne_externe_historique`, `evenement = 'correction'`, `le > publiee_le`).
  - Une moyenne republiée après sa correction n'est pas touchée, et un nouveau passage de la migration n'a aucun effet.
- **Tests** :
  - `contre_revue_m3_une_moyenne_corrigee_sous_477_est_depubliee` est nouveau.
  - Les deux tests M3 exécutent désormais **le texte même** de l'ordre de 478 (`include_str!`), au lieu d'une copie.
  - Résultat en local : suite `tests_evaluations_v5` et `migrate::`, 34 verts sur base neuve ; clippy `ent-db` labo sans avertissement.
- **Reste en file** (suites V5) :
  - `export_si` des tables d'évaluation (non publié jamais exporté, avant le lot G3) ;
  - test du retrait par un second compte (`saisi_par` inchangé) ;
  - contrôle positif du professeur dans `separation.rs` ;
  - `autoriser_eleve_le` en `pub(crate)` ;
  - dédoublonnage après fusions en chaîne (`DISTINCT ON`) et relevé aligné sur `eva_fiches_de_l_eleve` ;
  - export LSU/LSL calculé sur le seul publié (à trancher) ;
  - jeton de version complet (appréciation, pièce) ;
  - `search_path` des déclencheurs de 478 (couvert par le transverse T1).

## 10. Suites (passe 3) — branche `ent-socle-21-code-v5-suites-3`, depuis `origin/ENT`, décisions D-069 du pilote

**Niveau :** implémenté + testé en local (PostgreSQL 16, base neuve `ent_test_p3`, toolchain 1.99.0, `--locked`, `ENT_ENV=test`, un seul `CARGO_TARGET_DIR`). Non validé par Florian ; non prêt pilote. **Migration 700** (plage d'extension 700–719 ouverte par le pilote) ; 477 et 478 (versées) **intactes** ; 479 reste réservée à V13.

### 10.1 Fait / non fait

| Point | Réalisation | Test |
|---|---|---|
| **1. Export de portabilité** | `export_si` sur `eva_moyenne_externe` (publiée), `eva_resultat_externe` (publié, compté, non retiré : ce que lit la famille), `eva_resultat` (évaluation publiée, comme la vue `eva_resultat_publie`), `eva_resultat_item` (idem via le résultat). Les deux journaux `eva_moyenne_externe_historique` et `eva_resultat_historique` **sortent de l'export** (valeurs antérieures, motifs, auteurs). Colonnes `hors_export` : auteurs et motifs internes des deux tables externes. | `passe3_l_export_de_portabilite_ne_rend_que_le_publie` : la VRAIE condition SQL exécutée après note non publiée, résultat externe non compté / compté / publié / retiré, moyenne non publiée / publiée / corrigée |
| **2. Retrait par un second compte** | test seul (le code était déjà juste) | `passe3_le_retrait_par_un_second_compte_…` : `saisi_par` inchangé en base, `dernier_retrait_par` et `par` du journal = second compte |
| **3. Contrôle positif du professeur** | test de service à jour fixé (le cours du professeur du labo HTTP n'a pas de matière ; lui en donner une changerait un labo partagé) ; renvoi en commentaire dans `separation.rs` | `passe3_le_professeur_voit_un_resultat_externe_de_sa_matiere` |
| **4. `autoriser_eleve_le`** | `pub(crate)` ; commentaires corrigés : `droits_effectifs` lit encore le jour de l'établissement (attribution de rôle, cibles de périmètre, délégations) | — |
| **5. Fusions en chaîne** | `DISTINCT ON (evaluation_id)` dans `moyennes_tx` ; priorité à la fiche conservée, puis saisie la plus récente (`saisi_le DESC`, `id`). Le relevé suit `eva_fiches_de_l_eleve` avec la même règle. | `passe3_fusions_en_chaine_…` (A et B dans C : une seule ligne, moteur et relevé ; avec résultat de C : C l'emporte) |
| **6. Export LSU/LSL** | calculé sur le seul publié (`eva_calculer_classe(…, true)`) ; l'avertissement rendu au personnel ajoute « Les notes non publiées ne sont pas comptées dans cet export. » | `passe3_l_export_lsu_lsl_ne_compte_que_le_publie` : une note non publiée ne change pas l'export (LSU et LSL) |
| **7. Jeton de version** | colonne `version` (700) incrémentée par déclencheur à chaque écriture effective ; `vue` = `{version}` ; comparé dans le `WHERE` de publier et de retirer ; `MoyenneExterneDto.version` ; un client ne fixe jamais la version | `passe3_le_jeton_de_version_detecte_…` (A→B→A, appréciation seule, pièce seule, publication = écriture, version imposée par le déclencheur) ; HTTP : 409 sur jeton périmé |
| **8a. Fenêtre F-1** | intervalles **disjoints** (une fenêtre par intervenance, fusion seulement des intervalles qui se touchent) | `passe3_f1_la_fenetre_du_professeur_est_faite_d_intervalles_disjoints` |
| **8b. Cours sans ligne d'intervenance** | **documenté** (code) : sa fenêtre est celle de la présence de l'élève dans le cours, non bornée | — |
| **8c. `RAISE NOTICE` des lignes dépubliées** | **non fait** : la dépublication est dans 478, versée donc intouchable ; 700 ne dépublie rien | — |
| **8d. Auteur « système » du retrait de rattrapage** | **non fait** : le déclencheur de 478 exige un auteur (`retrait_par` non nul) ; la dépublication de rattrapage de 478 porte `saisi_par`, et 478 est versée | — |

**Limites restantes.** Les tables historiques `ent_note_*`, `eva_instantane_ligne`, `eva_moyenne_statut` et `eva_revision_demande` n'ont pas de règle `export_si` : à traiter au lot G3 avec l'assemblage de l'extrait. Les évaluations du parcours (`eva_resultat`) ne suivent pas encore la fiche conservée (seuls le moteur et le relevé le font). Toujours aucun écran pour publier ou retirer.

### 10.2 Tests, compteurs
* **Nouveaux : 7** (`passe3_*`, `tests_evaluations_v5.rs`) ; **adaptés :** `vue_moy`, `contre_revue_m2`, `separation.rs` (corps `vue`, jeton lu dans la réponse, 409 sur ancien jeton).
* **Locaux, verts :** `tests_evaluations*`, `migrate::`, `registre_donnees`, `tests_rls::`, `garde_search_path`, `tests_eleve_fusion`, `tests_eleve_e9`, `extrait_personne`, `droits` : **304/304** (un échec de semis parallèle `seed cast: Event(Infrastructure)`, vert au rejeu isolé).
* **Compteurs (règle 28)** : 0 table (`tests_rls.rs` inchangé), registre des tables inchangé (règles d'export seulement), `TABLES_ELEVE` et `Action::TOUTES` inchangés, sources de routes inchangées, `migrate.rs` +1 (700 après 690), `MIGRATION_TAG` inchangé.
* **Non exécuté en local :** `separation.rs` et les tests `ent-web-server` (édition de liens / mémoire), laissés à la CI.

### 10.3 Tête et CI
* **Tête du code :** `4f834e20ac72470d77ae90e6ac6fcadb98f83242` (PR brouillon #132, `ent-socle-21-code-v5-suites-3` → `ENT`).
* **CI « ENT Rust » sur cette tête : verte** (`completed/success`, y compris `separation.rs` et les tests `ent-web-server` non exécutés en local).
* Le commit de documentation qui inscrit cette section ne touche pas `ent-rust/**` : le filtre de chemins de la CI ne le relance pas ; la CI de référence reste celle de `4f834e20`.

## 11. Suites (passe 4) — branche `ent-socle-21-code-v5-suites-4`, depuis `origin/ENT` ae6e4fc1

**Niveau :** implémenté + testé en local (PostgreSQL 16, base neuve `ent_p4_test`, toolchain 1.99.0, `--locked`, `ENT_ENV=test`, un seul `CARGO_TARGET_DIR`). Non validé par Florian ; non prêt pilote. **Aucune migration** (règles d'export seulement, dans `registre_donnees.rs`).

### 11.1 Fait / non fait

| Point | Réalisation |
|---|---|
| **1. M2 : export des autres tables `eva_*`** | Décision table par table ci-dessous (droit d'accès de la famille ou de l'élève, RGPD art. 15 ; D-069 : seul le publié). |
| **2. Test de registre** | `toute_table_eva_exportee_est_declaree` : toute table `eva_*` / `ent_eva_*` exportée porte `export_si`, `hors_export`, ou une mention `tout_exportable("raison")` (nouveau champ `export_integral`, raison d'au moins 20 caractères). Négatif : `une_table_eva_exportee_sans_declaration_est_vue` (table nue et raison vide vues ; table déclarée, non exportée et hors domaine ignorées). |
| **3. Export de bout en bout** | `passe4_l_export_de_l_eleve_ne_contient_ni_note_non_publiee_ni_champ_interne` : sur un élève (note non publiée avec note privée, note publiée, brouillon d'appréciation, appréciation validée, demande de correction décidée), l'extrait est assemblé comme le fera G3 (`export_si`, retrait de `hors_export`, **pour toutes** les tables `eva_*` exportées ; chaque colonne retirée existe). Contrôle positif (note publiée, appréciation validée, demande de la famille) ; absents : évaluation non publiée, note privée, brouillon, auteurs et décideur internes. |
| **4. Faibles F1–F3** | **Non traités** : le texte de l'entrée du pilote (« Faibles F1-F3 en file ») ne les détaille pas et la revue n'est pas versée dans les documents accessibles ; les deviner serait inventer. Le pilote est prié de les détailler pour une prochaine passe. (Rappel : les 8c/8d de la passe 3 restent non faits, 478 étant versée.) |

**Choix table par table**

| Table | Choix |
|---|---|
| `ent_eva_export` | **Tout exportable**, raison écrite : journal des exports faits par la personne (format, classe, période, effectif, empreinte) ; aucune note, aucun élève nommé. |
| `eva_appreciation` | **Corrigé par le pilote (contre-revue, E1)** : `export_si` = la règle de la vue famille `ent_note_appreciation_v` (468, M5) : validée, type `cours` ou `generale`, appréciations closes pour la classe de l'élève (rang d'état de période ≥ 2). La version initiale (`x.etat = 'validee'`) aurait livré à une demande d'accès un texte que la famille ne voit pas encore. `hors_export` : `auteur`, `modifie_par`, `origine_appreciation_id`. `aide_ia` reste (transparence). |
| `eva_appreciation_historique` | **Retirée de l'export** : versions successives (brouillons, textes écartés) et éditeur de chacune ; interne. |
| `eva_correction_demande` | `hors_export` : `decide_par` (la demande, le motif et la décision sont ceux de la famille). |
| `eva_dispense_port` | `hors_export` : `saisi_par`, `retiree_par`. |
| `eva_enseignement_externe` | `hors_export` : `declare_par`, `referent` (tiers). |
| `eva_evaluation_restriction` | `export_si` : évaluation publiée (sinon elle révèle une évaluation non annoncée). |
| `eva_instantane_ligne` | **Retirée de l'export** : dérivé pris avant la publication, calculé sur des notes pas toutes publiées ; le bulletin figé (G3) porte ce que la famille a reçu. |
| `eva_moyenne_statut` | `hors_export` : `motif_interne`, `par`. |
| `eva_rattrapage` | `hors_export` : `cree_par`, `modifie_par`, `motif_annulation`, `dernier_motif_prolongation`, `zero_attribue_le`, `zero_attribue_par` (le zéro est une note non publiée). La convocation (date, lieu, modalité, échéance, état) sort. |
| `eva_rattrapage_autorise` | `hors_export` : `motif`, `par`. |
| `eva_rattrapage_historique` | `hors_export` : `par`, `motif`. |
| `eva_revision_demande` | **Tout exportable**, raison écrite : message de la famille et réponse qui lui a été adressée (`reponse` n'existe que si l'état est « repondue »). |

Les tables `ent_note_*` (hors préfixe `eva_`) ne sont pas dans le champ du test de registre ; elles restent à traiter au lot G3.

### 11.2 Tests, compteurs, vérifications
* **Nouveaux : 3** (2 de registre, 1 de bout en bout). Locaux, verts sur base neuve : `registre_donnees`, `tests_evaluations_v5`, `extrait_personne`, `tests_rls::`, `migrate::`, `garde_` : **124/124** avec `labo` ; gardes statiques et registre rejoués **sans** `labo` : 29/29.
* **Compteurs (règle 28)** : 0 table, 0 migration (`migrate.rs` et `MIGRATION_TAG` inchangés), `tests_rls.rs`, `TABLES_ELEVE`, `Action::TOUTES` inchangés ; le registre ne change que par des règles d'export et le champ `export_integral`.
* **Clippy 1.99.0 : 0 avertissement** sur les deux commandes de la CI (`--workspace --all-targets`, avec et sans `ent-api/labo,ent-lab/labo`, `-D warnings`) ; `cargo metadata --locked` dans `ent-rust/web` : OK. rustfmt : les fichiers touchés n'étaient pas conformes avant ce lot (aucun `cargo fmt --all`) ; lignes ajoutées écrites au format.

### 11.3 Tête et CI
* **Tête du code :** `8da928b0d51d3133416d1a4e17c4657ab798c5ed` (PR brouillon #136, `ent-socle-21-code-v5-suites-4` → `ENT`).
* **CI « ENT Rust » sur cette tête : verte** (run 45270353066, `completed/success`). Le commit de documentation qui inscrit cette section ne touche pas `ent-rust/**` : la CI de référence reste celle de `8da928b0`.

### Correctifs du pilote après contre-revue (passe 4)
- **E1 (élevé)** : `eva_appreciation` n'est exportée qu'aux conditions de la vue famille (validée, type `cours`/`generale`, appréciations closes) ; **E2** : le test de bout en bout, qui exigeait la sortie d'une appréciation validée en période ouverte, vérifie désormais son **absence** avant clôture, sa présence après `appreciations_closes`, et l'absence d'une appréciation `vie_scolaire` validée.
- **F1, F2** : `eva_resultat` retire aussi `derniere_ecriture_correction`, `source`, `version` ; `eva_resultat_externe` retire `compte_decide_le`.
- **En file (suites V5, passe 5)** : M1 (test de registre : `hors_export` seul n'est pas une décision pour une table à état), M2 (extrait de bout en bout : une ligne marquée par table exportée, échec si une table n'est pas filtrable, p. ex. `eva_moyenne_externe`), M3 (`eva_moyenne_statut` sans condition de publication), M4 (`eva_rattrapage_historique` exporté, incohérent avec le retrait de l'historique des appréciations), M5 (`eva_rattrapage*` sans condition d'état), F3 (déclarations externes et dispenses : communication à confirmer), **F4** (`ent_note_appreciation`, table de reprise, exportée sans condition : contourne E1 jusqu'à G3), réserve DPO sur l'historique d'une appréciation publiée puis modifiée.

## 12. Suites (passe 5) — branche `ent-socle-21-code-v5-suites-5`, depuis `origin/ENT`

**Niveau :** implémenté + testé en local (PostgreSQL 16, base neuve `ent_p5_test`, toolchain 1.99.0, `--locked`, `ENT_ENV=test`, un seul `CARGO_TARGET_DIR`). Non validé par Florian ; non prêt pilote. **Aucune migration** (règles d'export seulement, dans `registre_donnees.rs`) : 701 non consommée, `migrations_versees.txt` intact.

Principe : l'export ne rend **que ce que la famille peut déjà voir dans l'ENT** ; une table à état ne s'exporte jamais sans condition.

### 12.1 Fait / non fait

| Point | Réalisation |
|---|---|
| **M1** | Test `toute_table_eva_a_etat_exportee_a_une_condition_ou_une_justification` (sur la base) : toute table `eva_*` / `ent_eva_*` exportée qui porte une colonne d'état (`etat*`, `statut`, `publiee_le`, `retiree_le`, `annule_le`, `decide_le`, `valide_le`…) doit avoir `export_si` ou `tout_exportable(raison)` ; `hors_export` seul échoue, avec la liste. Garde non aveugle (tables d'état connues reconnues). Négatif : `hors_export_seul_sur_une_table_a_etat_est_vu`. Conséquence : `eva_correction_demande` (état, `hors_export` seul) reçoit une raison écrite (demande de la famille sur une note publiée). |
| **M2** | `extrait_eleve` : filtre par élève aussi via `enseignement_externe_id` et `rattrapage_id` ; une table exportée non filtrable **échoue** (plus de `continue`, sauf `TABLES_SANS_ELEVE` = `ent_eva_export`). Le test de bout en bout sème **une ligne marquée par table exportée** (restriction, item, revue, statut de moyenne, convocations, résultat externe, moyennes externes) et vérifie que chaque table exportée a rendu une ligne ; absents : convocation annulée, évaluation non publiée, moyenne non publiée, motifs et auteurs internes. |
| **M3** | `eva_moyenne_statut` : `export_si` = appréciations closes pour la classe de l'élève (même règle que `eva_appreciation`) ; avant, le statut révèle une moyenne en suspens. Testé avant/après clôture. |
| **M4** | `eva_rattrapage_historique` **retiré** de l'export (cohérent avec l'historique des appréciations). |
| **M5** | `eva_rattrapage` : `export_si` = état ≠ « annulée » et évaluation publiée. `eva_rattrapage_autorise` **retirée** (autorisation interne de la direction ; la famille reçoit la convocation). |
| **F3** | `eva_dispense_port` et `eva_enseignement_externe` **retirées** de l'export : aucune colonne ni lecture familiale ne prouve la communication (D-069, valeur prudente). À rouvrir quand l'ENT notifiera la décision (colonne `communiquee_le` + `export_si`). Ce que la famille voit d'un enseignement externe reste la moyenne externe publiée. |
| **F4** | `ent_note_appreciation` (reprise) **retirée** de l'export jusqu'à G3 (qui exportera le bulletin figé) ; test `la_table_de_reprise_des_appreciations_n_est_pas_exportee_sans_condition`. |
| **Réserve DPO** | Décision prudente retenue : l'historique d'une appréciation publiée puis modifiée **n'est pas exporté** (`eva_appreciation_historique` hors export) ; seule la version en vigueur, si elle est celle que la famille voit, sort. La personne qui demande l'historique (RGPD art. 15) relève d'une demande motivée traitée par le DPO, hors export automatique. À confirmer par le DPO. |

**Reste (non fait, signalé).** `ent_note_archive`, `ent_note_correction`, `ent_note_doublon_archive`, `ent_note_periode` restent exportées sans condition (hors du champ des tests `eva_*`) : à traiter au lot G3.

### 12.2 Tests, compteurs
* **Nouveaux : 4** de registre (M1 ×2, F4, M4/M5/F3) ; **étendu :** `passe4_l_export_de_l_eleve_…` (M2/M3/M5).
* **Compteurs (règle 28)** : 0 table, 0 migration, `tests_rls.rs`, `TABLES_ELEVE`, `Action::TOUTES` inchangés ; le registre ne change que par des règles d'export.

### 12.3 Tête et CI
* **Tête du code :** `4376238f` (PR brouillon #143, `ent-socle-21-code-v5-suites-5` → `ENT`). Local : 259/259 tests (`registre_donnees`, `tests_evaluations*`, `extrait_personne`, `tests_rls::`, `migrate::`, `garde_`, `tests_eleve_fusion`) ; clippy 1.99.0 `-D warnings` sur les deux commandes backend (avec `ent-lab`) : 0 avertissement ; `cargo metadata --locked` dans `web/` : OK.
* **CI « ENT Rust » sur cette tête : verte** (run 45356863579, `completed/success`). Le commit de documentation qui inscrit cette section ne touche pas `ent-rust/**`.
