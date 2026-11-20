# Socle 21 — lots V3c « revue de sécurité de V3b » et V4 « absences, dispenses, aménagements, rattrapages » : livraison

**Date :** 2026-11-20 — **Branche :** `ent-socle-21-code-v3c` (depuis `origin/ENT` 65daf461) — **PR en brouillon vers `ENT`** : voir la fin.
**Niveau :** *implémenté + testé en local* (PostgreSQL 16 local, base **neuve** dédiée, mode normal) ; **CI GitHub : voir la fin** ; **non validé par Florian ; non prêt pilote.**
**À lire avec :** `21_EVALUATIONS.md` (§8, §9, §10, §11 CE-25 à CE-38, §14, §18, §23), `21_EVALUATIONS_V3B_LIVRAISON.md`, `MIGRATIONS.md` (mise à jour du 2026-11-20).

## 0. Modèle de menace appliqué (règles générales, pas champ par champ)
| Règle | Où elle est tenue |
|---|---|
| 1. Une note non publiée n'existe pas pour la famille ni l'élève (ni valeur, statut, moyenne, statistique, événement, notification, agenda) | `/eva/v4/*` : personnel seulement (403 famille et élève, testé) ; **événements V4 neutres** (aucun `eleve_id`, que `evaluation_id` et `rattrapage_id`, clés comparées dans le test) ; avis « nouvelle note » seulement aux élèves qui ont une **note** ; agenda : évaluation annoncée seulement ; C5 : résultat publié seulement ; bulletin : statistiques sous le seuil masquées |
| 2. Un seul moteur ; absent, dispensé, annulé, non rendu ne valent jamais 0 sauf « seul 0 autorisé » | vue `ent_note` sans « 0 » (474) ; C5 montre le **statut** ; V4 écrit des statuts (`absent`, `dispense`, `a_rattraper`) ; le 0 n'existe que pour la nature `remplacement` à l'état `absent_non_justifie`, par `notes.corriger` avec confirmation, **et la base le refuse ailleurs** (`eva_rattrapage_zero_chk`) |
| 3. L'élève visible : toute lecture par élève passe par `exiger_eleve_visible` | ajoutée au **relevé** (antérieure) ; toutes les lectures et écritures V4 filtrent ou refusent les élèves protégés |
| 4. Tout ce qui change une note est attribué et historisé, sans contournement par un champ vide | `saisi_par` NULL refusé par la base (M1) ; la règle « période ouverte = professeur » vit dans `ecrire_resultat_tx` (aucune voie ne la contourne) ; V4 n'écrit un résultat que par `ecrire_resultat_tx` (versions, historique, événement) |

## 1. Partie 1 — V3c (constats vérifiés de la revue de V3b)
| Constat | Réalisation | Test |
|---|---|---|
| **M1** attestation `app.eva_correcteur` contournée si `saisi_par` NULL | migration 474 : `ent_trg_eva_resultat` exige un **auteur** et une attestation **non vide** égale à l'auteur | `v3c_m1_…` : INSERT / UPDATE sans auteur refusés, après l'arrêt : auteur NULL, attestation absente, attestation d'un autre auteur refusés ; attestation correcte acceptée |
| **M2** cahier C5 « 0 / 20 » pour absent / dispensé / annulé | C5 lit `eva_resultat_publie` **revérifié pour l'élève de la remise** ; nouveau champ `note_statut` (affiché « Résultat : absent »…) ; vue `ent_note` : `valeur` NULL hors statut `note` (474) ; l'écran Leptos du retour affiche le statut | `tests_cahier_c5` (statut `absent` affiché, liaison forgée vers l'élève d'un autre : rien) ; `v3c_m2_…` (vue) ; test de compatibilité des vues ajusté à ce nouvel écart assumé |
| **M3** agenda ignore `annoncee` et les restrictions de public | `evaluations_calendrier.rs` lit `eva_evaluation` : annoncée (ou publiée) + public restreint respecté ; repli historique conservé pour un cours sans service | `v3c_m3_l_agenda_…` ; fixture des tests du cahier : devoirs « annoncés » |
| **M3** (descente) période ouverte = professeur | dans `ecrire_resultat_tx` : un porteur de `notes.corriger` qui n'est pas du cours ne saisit pas en période ouverte, **par aucune voie** | `v3c_m3_la_direction_…` ; 4 tests existants ajustés (la direction écrivait en période ouverte : le professeur du cours écrit, la direction corrige après l'arrêt avec motif) |
| **M4** statistiques de classe du bulletin sans seuil | `CalculClasse::stats_*_montrables` : même seuil que les familles (`seuil_effectif_stats`, plancher 3, 5 par défaut) pour le bulletin et le livret ; **alerte** dans le rapport de génération pour toute évaluation de la période **jamais publiée** qui a des notes (`RapportBulletinsDto.alertes`, jamais dans le bulletin) | `v3c_m4_…` (seuils 2/3/4/5) ; e2e V3b (alerte « Brouillon ») ; un test de bulletin ajusté (sous le seuil, la correction d'un élève ne change plus la ligne des autres) |
| **M5** rejeu tolère `42601` sur tout ordre citant `ent_note` | `rejeu_tolere` : **liste fermée** (42809 : DDL qui cite la vue comme identifiant entier ; 55000 : l'appel exact `ent_rattacher_classes()` ; 42601 : l'ordre exact de 460, 601 caractères ; tout le reste est fatal) — liste établie en **traçant** le rejeu réel (28 ordres distincts, tous couverts) | `migrate::tests::le_rejeu_ne_tolere_que_les_ordres_exacts_attendus` + rejeu complet vert (`tests_d036`, `tests_dossier_personnes`) |
| `NULLS LAST` des vues | 474 | par construction (vues recréées) |
| `service_id` `LIMIT 1` | les vues ont déjà `ORDER BY created_at, id` ; ajout d'un `ORDER BY p.id` aux deux sous-requêtes de noms (conflit, historique) qui n'en avaient pas | — |
| pas d'avis aux élèves absents | consommateur de notifications : `n.statut = 'note'` | `v3c_la_publication_ne_previent_que_les_eleves_qui_ont_une_note` |
| moyennes partielles du professeur (UI JS) | **partiel** : les moyennes « live » et de classe calculées dans le navigateur (`ui.rs`) sont **supprimées** (le nombre de notes reste, la moyenne vient du moteur) ; la moyenne générale élève (`moyenneGeneraleSur20FromMatiereRows`, moyenne simple des matières) **n'est pas touchée** : voir 4 | — |
| journaux `passages_quotidiens` | `classe_erreur` : famille + SQLSTATE seulement, jamais le message (10 sites) | `tests_journaux` |
| interblocage du lot de cases | cases traitées dans l'ordre stable (évaluation, élève, position) ; la réponse reste dans l'ordre de la requête | `v3c_deux_lots_croises_…` (deux lots opposés en parallèle) |
| `exiger_eleve_visible` sur le relevé | ajoutée | `v3c_le_releve_applique_…` |

## 2. Partie 2 — V4
### 2.1 Modèle (migration 475, 4 tables avec RLS)
`eva_dispense_port` (port 34 **en simulation**, sans motif), `eva_aesh_indisponibilite_port` (simulation du socle 6), `eva_rattrapage` (convocations de rattrapage et de **remplacement**, machine d'états en base, une seule convocation en cours par élève et évaluation), `eva_rattrapage_historique` (ajout seul, alimenté par déclencheur). Ports en **lecture** : 12 (`ent_asd_absence_seance` + séance du cours ce jour-là), 35 (`ent_amenagement`, portée examen / les deux).

### 2.2 Ce que chaque acteur peut faire (14 routes `/eva/v4/*`, personnel seulement)
| Acteur | Actions |
|---|---|
| **Professeur** (titulaire, co-enseignant, suppléant en saisie) | voir les **propositions** d'absents et de dispensés de son évaluation (l'appel de la séance du cours propose, il confirme tout ou partie ; serveur recalcule, jamais d'écrasement d'une case remplie) ; voir les **incohérences** (appel « absent » mais copie notée : la saisie prévaut) ; voir les **inaptes partiels** (information, rien n'est pré-rempli) ; voir les **aménagements** de ses élèves (libellés fonctionnels) et l'alerte AESH indisponible ; planifier un **rattrapage** (individuel ou séance collective), le déplacer, l'annuler (motif), le **noter** ou constater l'absence au rattrapage (justifiée / non) ; **reconvoquer** après absence justifiée ; voir les absences répétées de son cours |
| **Remplaçant** | ce que fait un suppléant en saisie ; **ne modifie pas** ce qui relève du titulaire sans droit (règles V3 conservées) ; le titulaire de retour retrouve la main |
| **Direction** (`notes.corriger`) | déclarer / retirer une **dispense** ou inaptitude datée (sans motif) ; **convoquer à l'évaluation de remplacement** ; attribuer le **zéro** (seulement remplacement + absence non justifiée constatée, confirmation de la référence R-01, acte tracé) ; autoriser un rattrapage après l'arrêt (existant V1) ; absences répétées de l'établissement ; aménagements de l'évaluation |
| **Secrétariat** (`notes.lire`) | lit les convocations d'un cours et les dispenses d'un élève (sans motif) ; ne planifie ni ne saisit |
| **CPE / vie scolaire** | **non couvert par un droit propre** dans ce lot : l'incohérence d'appel et l'absence répétée sont servies aux droits ci-dessus ; leur transmission à la CPE est **à faire** (voir 4) |
| **AESH** | voit les aménagements d'évaluation **de ses élèves seulement** (date de l'évaluation), jamais notes, moyennes, plan (PAP, PPS) ni diagnostic |
| **Famille, élève** | **rien** (403 sur toutes les routes V4, testé) : l'affichage « à rattraper le … » des familles relève de V7 |

### 2.3 Cas limites traités
Absence justifiée **après coup** (la justification est relue au socle 12 à chaque lecture ; le résultat `absent` n'est jamais réécrit) ; l'appel arrive **après** la saisie (CE-30 : propositions apparues, rien d'écrasé) ; copie notée malgré l'appel (CE-31) ; évaluation **hors séance** (aucune proposition) ; **dispense partielle** (CE-34) ; inaptitude **à l'année** (CE-33 : pas de moyenne de la matière) ; dispense **datée** (hors plage : rien) ; élève **arrivé en cours de période / hors public** (non convocable) ; **période arrêtée** (convocation refusée sans autorisation de rattrapage, et rien d'écrit) ; **version périmée** (conflit) ; double convocation (refus) ; élève **protégé** (introuvable) ; autre établissement (introuvable) ; reconvocation tracée et liée (CE-28) ; non-note (UPE2A) convocable au remplacement (CE-38).

### 2.4 CE-25 à CE-38
| CE | Test |
|---|---|
| 25, 26, 30, 31 | `v4_ce25_26_…`, `v4_ce31_…` |
| 27 | `v4_ce27_…` (alerte à 3 absences sans justification ; la justifiée et la 2ᵉ non comptées) |
| 28, 29 | `v4_ce28_29_…` (reconvocation, zéro refusé tant que non « non justifié », refusé hors remplacement **par la base aussi**, attribué après l'arrêt par la direction, tracé) |
| 32, 33, 34, 36 | `v4_ce32_33_36_…`, `v4_ce33_34_…` |
| 35, 37 | `v4_ce35_37_…` (marque « aménagé » posée par la base, jamais dans la vue des publications ; AESH ; alerte) |
| 38 | `v4_ce38_…` |
| rattrapages (EVA-259) | `v4_rattrapage_collectif_…`, `v4_rattrapage_apres_l_arret_…` |
| isolation | `v4_une_evaluation_d_un_autre_etablissement_…` |

## 3. Tests et vérifications
Base locale PostgreSQL 16 **neuve** (`ent_v3c`), `CARGO_TARGET_DIR` propre, toolchain 1.99.0, `--locked`.
* **Nouveaux : 21** (8 V3c en base ou unitaires, 1 rejeu, 1 journaux, 11 V4) + C5 étendu + assertion d'alerte au test e2e ; **7 tests existants ajustés** pour les nouvelles règles (voir 1).
* Exécutés en local : `tests_evaluations_v3c`, `tests_evaluations_v4`, `tests_evaluations*` (dont V1, revue V1, V3), `tests_cahier`, `registre_donnees`, `tests_rls::`, `migrate::`, `fusion`, notifications : **verts** après correction (premier passage : 9 échecs, tous des attentes d'avant V3c : direction qui saisissait en période ouverte, agenda sans « annoncée », bulletin avec moyenne de classe sous le seuil, vue sans « 0 » ; un seul vrai défaut du code : une jointure sur la mauvaise table de remises dans C5, corrigé).
* La suite complète d'`ent-db` (≈ 2 436 tests) et les 12 tranches de RLS stricte relèvent de la **CI** (règle 36 : pas de vérification locale qui double une CI à l'arbre identique).
* Clippy (commandes de la CI) backend et frontend `-D warnings` : voir 5 ; `cargo fmt` **ciblé** (`ent-types`, paquets web ; `rustfmt` des 4 nouveaux fichiers du backend et des 2 fichiers de tests nouveaux) ; `cd ent-rust/web && cargo metadata --locked` : **vert** (aucune dépendance ajoutée).

## 4. Non fait, limites, à trancher
1. **Aucun écran Leptos V4** (propositions dans la grille, convocations, aménagements, dispenses) : le lot livre la couche base, le service et l'API (14 routes) ; les écrans sont le lot suivant. **Non parcouru dans un navigateur.**
2. **Famille et élève** : aucun affichage ni avis des convocations (V7). Les événements V4 sont neutres et **non consommés** par les notifications.
3. **CPE / vie scolaire** : l'incohérence d'appel (CE-31) et l'absence répétée (CE-27) sont **servies sur demande**, sans événement ni alerte poussée ; pas de droit propre à la CPE (à poser avec la matrice des droits du socle 12).
4. **UI JavaScript** : `moyenneGeneraleSur20FromMatiereRows` (moyenne simple des moyennes de matière, sans coefficient) **reste** ; elle devrait venir du moteur (route `/eva/*` ou `eva_v1_moyennes_famille`). Décision : retirer l'affichage de cette moyenne générale de `ui.rs` ou l'alimenter par le moteur (V12).
5. **Ports 34 et AESH en simulation** : tables saisies par la direction en attendant les socles 34 et 6 ; l'interface de remplacement est le point unique de lecture (`eva_dispense_a_la_date`).
6. **Port 12** : la proposition d'absent exige une **séance de l'EDT rattachée au cours** (`ent_edt_seance.cours_id`) ce jour-là ; une évaluation hors séance se saisit à la main (conforme à §4.4). Les demi-journées sans séance ne proposent rien.
7. **Non couverts de V4** : EVA-170 (moyenne « en attente »), EVA-171, EVA-176/177 (résultats externes, V5), EVA-261 et 266 à 268 (sujet adapté, partage AESH, conflits d'accompagnement), proposition automatique d'une évaluation dédiée (EVA-169), rappel d'échéance de rattrapage (seul le drapeau `echu` est calculé à la lecture), évaluation de remplacement « de fin d'année unique » (la direction en juge ; rien n'empêche deux convocations de nature remplacement pour un même élève sur deux évaluations).
8. **À trancher** : (a) la direction ne **saisit** plus en période ouverte (même pour noter un rattrapage ou attribuer un zéro, qui passent par l'écriture commune) : le zéro et la note d'un rattrapage en période ouverte reviennent au professeur du cours ; (b) la référence réglementaire de R-01 vient du mémento eduscol de décembre 2025 (§5 de la conception) : à faire valider ; (c) `Action::NotesCorriger` tient lieu de « `evaluations.saisir_tout` » (inexistant) pour la convocation au remplacement.

## 5. Compteurs (règle 28) et état
`tests_rls.rs` `… + 31 + 13 + 5 + 2 + 4` (**+4**) ; registre des données **+4** ; `TABLES_ELEVE` **+3** ; sources de routes **68 → 69** ; `Action::TOUTES` **inchangé** ; `MIGRATION_TAG` **inchangé** (3808) ; migrations **474–475** (461–473 intactes) ; documents tenus : `MIGRATIONS.md`, ce rapport.
**Tête et CI :** voir le message final au pilote (cette rubrique est complétée au dernier commit).
