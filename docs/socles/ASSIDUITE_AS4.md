# Socle 12 — lot AS4 (code) : décompte versionné

**Date :** 2026-11-14 — **Branche :** `ent-socle-12-code-4` (depuis `origin/ENT` e0474cff, après AS0 à AS3) — **Niveau :** implémenté + testé en local (PostgreSQL 16) ; CI : voir `docs/PROJECT_STATUS.md` ; **non validé par Florian, pas prêt pilote.**
**Conception :** `12_ASSIDUITE.md` §13.1, §13.2, §22.6, R-AS15 à R-AS17, CA-25, CA-39, CA-48, CA-49, CA-67 (sur `origin/ent-mc0hoe`). Plage de migrations imposée **2246–2259** (vérifiée libre sur toutes les branches distantes, 2256 laissée à l'autorisation récurrente) ; consommée : **2246** ; `MIGRATION_TAG` **inchangé** (`3041_demandes_n1_0_droits`, plus haute du répertoire au tri numérique) ; aucune migration existante modifiée ; aucune colonne JSON (D-036).

## 1. Ce qui est livré

### Le décompte lit les épisodes (limite laissée par AS3)
- `assiduite_as4.rs` : `calculer_demi_journees` part des **épisodes** (`ent_asd_absence_seance` actives, épisode non retiré) et de leur **qualification** : réputée légitime ou excusée valable = justifiée ; non valable, sans motif, en attente = non justifiée jusqu'à décision. Chaque séance garde son état : un jour qui mêle épisodes valables et non valables n'est plus « refusé » en bloc (test `as4_jour_mixte_chaque_episode_garde_son_etat`). L'ancien statut `ent_absence.statut` n'est plus lu par le décompte (il reste projeté vers l'ancien circuit, pour l'ancienne famille).
- **Absences de l'ancien circuit sans séance** (route famille ou accueil à la journée, DA-14) : la fonction SQL idempotente `ent_asd_adopter_absences` leur donne un épisode et des demi-journées entières (lundi au vendredi), avec des identifiants déterministes comme la reprise de 2244 ; le calendrier retire ensuite vacances et fériés au calcul.

### Règle datée du jour, jamais rétroactive (R-AS15)
- La règle appliquée à un jour est **la dernière version dont `valide_du` ≤ ce jour** (`regle_a`), jamais celle d'aujourd'hui ; chaque demi-journée matérialisée porte `regle_valide_du` et `regle_id`. Les trois **définitions** de la demi-journée sont codées : une séance manquée, au moins N minutes (l'existant, vérifié équivalent par un test d'équivalence avec `deriver_demi_journees`), toute la demi-journée (minutes manquées ≥ minutes de cours attendues, d'après les appels validés ; ou absence « à la journée »).
- **Seuil de signalement** du mois : règle en vigueur le dernier jour déjà écoulé du mois (vivant), ou règle figée à l'arrêt.

### Calendrier, coupure, scolarisation, fuseau (R-AS17)
- **Calendrier du socle 5** (`structure::calendrier::jours`, par classe de l'élève) : une demi-journée sans cours ni accueil compte **0** (`sans_cours`) ; hors année scolaire : `hors_annee` ; mercredi après-midi sans cours du profil : l'après-midi ne compte pas. Les absences lues pendant des vacances sont **conservées avec leur raison** (jamais supprimées) mais comptent 0. **Repli signalé** (`calendrier_source = 'repli'`) : un établissement sans aucune année scolaire garde « lundi au vendredi ».
- **Coupure** : la grille du socle 5 (STR-35, `ent_grille_pour` pour le site et le niveau de la classe) prime ; **repli sur la règle** (`coupure_source`). 
- **Scolarisation** : hors des périodes de scolarisation de l'élève (entrée, sortie exclusive), rien ne compte (`hors_scolarisation`, DA-22).
- **Fuseau** : jour métier `ent_aujourdhui_etab(etab, now())` (arrêt du mois, consolidation) ; les horaires comparés sont des heures murales d'établissement (socle 10) ; la date UTC du serveur n'est utilisée nulle part. Aucun test ne dépend de l'heure réelle (jours passés fixes ; un seul test joue « aujourd'hui », avec un calendrier rendu ouvert tous les jours).

### Matérialisation, explication, arrêt, rectifications (migration 2246)
- **`ent_asd_demi_journee`** (recalculable, clé `(établissement, élève, jour, période)`) : minutes et séances manquées / non justifiées, minutes attendues, règle appliquée, définition, seuil, coupure et sa source, source du calendrier, `compte`, `non_justifiee`, **`raison` de liste fermée**. `recalculer_decompte` est **idempotent** : une ligne inchangée n'est pas réécrite (« recalculer tout ne change rien », compteur de lignes modifiées = 0, `calcule_le` identique).
- **Explication** (AS-170) : `expliquer` fabrique « mardi 08/09/2026 matin compte, non justifiée : 2 séances manquées (120 min), dont 2 non justifiées ; règle : au moins une séance manquée, version du 01/09/2026. » **à partir des seuls champs codés** (jamais motif ni note, R-AS19, R-AS22). Exposée par `GET /eleves/{id}/demi-journees` (champs `raison` et `explication`, mêmes droits qu'avant, famille comprise pour son enfant) et par `decompte_eleve_explique_for_actor`.
- **Arrêt du mois** (AS-172) : `ent_asd_mois_cloture` + `ent_asd_mois_arrete` (décompte figé par élève, règle et seuil, **ajout seul** par déclencheur) ; `arreter_mois` n'arrête qu'un mois **terminé au jour métier de l'établissement**, est idempotent, fige les versions de règle qui ont servi (`figee_le`, déjà gardé par le déclencheur de 2241) et émet `assiduite.mois_arrete` (aucun texte). Déclenché par la **direction** (`POST /assiduite/mois/{mois}/arreter`, action `assiduite.regle_gerer`) ou par la tâche système horaire `assiduite_arret_mois_tache` (trois derniers mois, au jour métier de chaque établissement).
- **Rectifications** (R-AS16) : `ent_asd_rectification` (avant, après, cause de liste fermée, référence, `touche_signalement`, « vue par » posée une fois, le reste en ajout seul). Elles s'inscrivent **d'elles-mêmes** après une qualification, requalification, qualification en masse, correction d'appel ou neutralisation d'une séance qui touche un mois déjà arrêté ; le mois figé ne bouge jamais ; le décompte lu pour un mois arrêté est le figé (`DecompteMois.arrete`). Lecture : `GET /assiduite/mois/{mois}/rectifications` (`assiduite.signaler`, CPE et direction).

### Droits par défaut (règle 21)
Aucun : aucune nouvelle action. L'arrêt du mois réutilise `assiduite.regle_gerer` (direction), la lecture des rectifications `assiduite.signaler`. `socle_12_assiduite` n'est pas modifiée.

## 2. Défauts Next.js fermés (règle 14, D-051)
**Aucun NJ-xx n'est rattaché à ce lot** dans `ALERTES_NEXTJS_PROD.md` (les NJ de l'assiduité — NJ-30, 31, 34, 35, 38, 39 — sont fermés par AS2 et AS3). Les défauts de l'existant que ce lot ferme sont des **DA** (défauts du code Rust précédent), chacun avec son test :

| DA | Défaut | Test |
|---|---|---|
| **DA-11** | la règle courante réécrit le passé (changer la coupure ou le seuil modifie tous les mois) | `as4_changement_de_regle_en_janvier_laisse_septembre_inchange`, `as4_arret_du_mois_fige_et_les_rectifications_sont_a_part` |
| **DA-12** (suite d'AS3) | un jour mixte se projetait « refusé » | `as4_jour_mixte_chaque_episode_garde_son_etat` |
| **DA-14** | reprise en demi-journées du lundi au vendredi sans vacances ni fériés ni mercredi après-midi | `as4_vacances_declarees_donnent_zero_demi_journee` |
| **DA-22** | décompte incluant des élèves sortis | `as4_un_eleve_hors_scolarisation_ne_compte_pas` |

## 3. Critères de fin (§24.1) — tests
- **« Tout recalculer ne change rien »** → `as4_recalculer_tout_ne_change_rien` (trois rejeux, puis un élève seul : 0 ligne écrite, 0 supprimée, instantané identique ; une vraie requalification ne change que sa ligne, et le retour à l'état initial redonne l'état initial).
- **Un changement de règle en janvier laisse septembre inchangé** → `as4_changement_de_regle_en_janvier_laisse_septembre_inchange` (45 minutes comptent en septembre sous la règle de 30 minutes et ne comptent pas en janvier sous celle de 120 ; septembre recalculé = septembre ; la règle de septembre est figée à l'arrêt et immuable, y compris par SQL ; seuil 4 pour septembre, 6 pour janvier).
- **Des vacances déclarées donnent 0 demi-journée** → `as4_vacances_declarees_donnent_zero_demi_journee` (vacances de la Toussaint du calendrier du socle 5 ; absence à la journée de l'ancien circuit et séance isolée : 0, raison `sans_cours` ; contrôle : le jour ouvré voisin compte ; hors année : `hors_annee`).
- Autres : `as4_la_coupure_de_la_grille_prime_sur_le_repli_de_la_regle`, `as4_le_jour_metier_est_celui_de_l_etablissement_pour_l_arret`, `as4_explication_et_droits_de_lecture`, `as4_requalifier_apres_l_arret_inscrit_la_rectification`, et 7 tests unitaires purs (`assiduite_as4::tests`, dont l'équivalence avec le calcul existant pour la définition « minutes »).

## 4. Exigences E1–E14 (règle 13)
`etablissement_id UUID NOT NULL` (différent de l'UUID nul) sur les 4 tables neuves ; clés étrangères composites vers `ent_eleve` et `ent_asd_mois_cloture` ; UUID applicatifs ; aucun `BYTEA` ; `calcule_le` / `arrete_le` / `cree_le` ; mois arrêté et clôture en ajout seul, rectification en ajout seul hors « vue par » (déclencheurs) ; migration rejouable ; aucune colonne JSON ; **registre des tables à jour** (4 lignes, liste triée) ; RLS par `ent_rls_poser` et `TenantTx` ; compte RLS de `tests_rls.rs` : +4 ; payloads d'événements = codes et nombres (R-AS25) ; aucune donnée d'établissement dans les journaux techniques.

## 5. Limites connues et reste à faire
- **Mois glissant** (`type_mois = 'glissant'`, profil académique) : stocké par AS1 mais **non appliqué** : le décompte reste civil (CA-67 : une absence à cheval est imputée à chaque mois civil). À faire avec le profil (AS9).
- **Retards** : `ent_asd_mois_arrete.retards` vaut 0 jusqu'à AS7 ; les compteurs de retards ne sont pas dans le décompte (VADE).
- **Décompte en jours** pour la retenue de bourse (R-AS31, `jours_injustifies`, socle 26) : lot AS11 (exports).
- **Calendrier par classe courante** : la cible du calendrier est la classe **actuelle** de l'élève (`ent_eleve.classe_id`), pas sa classe à la date du constat ; un élève qui change de classe voit ses jours anciens relus avec le calendrier de sa nouvelle classe. Idem pour la grille de coupure. À affiner quand l'affectation datée du socle 3 sera lue.
- **Définition « entière »** : les cours attendus viennent des **appels validés** du jour (type `cours`, séance horodatée) ; une séance sans appel validé n'est pas attendue. Les créneaux attendus du socle 10 ne sont pas lus (port d'EDT à brancher en AS10).
- **Plafond** : une absence ancienne de plus de 400 jours est tronquée à 400 jours à l'adoption ; un calcul ne couvre pas plus de 800 jours.
- **Rectification et signalement** : `touche_signalement` est posé (franchissement du seuil) et l'événement `assiduite.rectification` est émis ; **aucun consommateur** n'alerte encore la direction (AS5 notifications, AS9 signalement). Le détail d'un signalement déjà proposé (`discipline_vs`) n'est pas recalculé sur le mois figé.
- **Nuit de l'heure d'été (CA-49)** : le calcul ne manipule que des heures murales et des dates de jour métier ; aucun test dédié au passage d'heure n'est ajouté ici (les tests de fuseau de minuit existent côté jour métier).
- **Pas d'écran** (AS14) : les routes sont prêtes, les écrans Leptos ne sont pas faits ; pas de jeu d'IA (AS13).
- **Tâche système** `assiduite_arret_mois_tache` : branchée dans `ent-api` (passage horaire), jamais appelée par les tests (elle toucherait les établissements des autres tests) ; l'arrêt d'un établissement est testé par `arreter_mois_systeme`.
