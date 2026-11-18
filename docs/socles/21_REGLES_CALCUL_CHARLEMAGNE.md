# Socle 21 — Règles de calcul « comme Charlemagne » (idée I-032, décision D-068)

**Date :** 2026-11-18 — **Statut :** document de calage, **rédigé par la session de code V1/V2** (branche `ent-socle-21-code-v1v2`). Aucune règle de Charlemagne n'a pu être lue ici : tout ce qui est écrit sur Charlemagne est une **hypothèse [H]**, à lever par Florian avec les questions de ce document. Aucune donnée réelle, aucun accès à Charlemagne ni à une base de production.

**But.** Florian comparera ScolIA à Charlemagne sur de vrais élèves (I-032). Les moyennes doivent tomber **exactement** comme dans Charlemagne, ou l'écart doit s'expliquer. Pour cela, chaque point où deux logiciels de notes peuvent diverger est ici : (1) ce que fait le moteur ScolIA (fait, vérifié dans le code et les tests) ; (2) ce que fait probablement Charlemagne **[H]** ; (3) **la question précise** qui lève le doute ; (4) le **réglage** qui permet de caler le moteur sans recoder.

**Conventions.** **[F]** fait vérifié dans le dépôt ; **[H]** hypothèse, rien n'a été lu de Charlemagne ; **[R]** recommandation. Les réglages sont des colonnes de `eva_reglage` (migration 461, un jeu par établissement et par année, historisé dans `eva_reglage_historique`) et des champs de `domain_eleve::evaluation_moteur::ReglesMoteur` (`crates/domain-eleve/src/evaluation_moteur.rs`). Poser les réglages : `PgEnt::eva_v1_definir_regles_for_actor` (droit `periodes.verrouiller`).

**Lecture.** Un **cas-test** accompagne chaque règle (section 3) : une mini-saisie à faire dans Charlemagne, avec le résultat attendu selon chaque hypothèse. **Quinze minutes de saisie de ces cas lèvent la plupart des doutes**, plus sûrement que n'importe quelle lecture de documentation.

---

## 1. Tableau des règles et de leur réglage

| # | Règle | Réglage ScolIA (champ Rust → colonne SQL) | Défaut ScolIA | Hypothèse Charlemagne **[H]** |
|---|---|---|---|---|
| C1 | Échelle des moyennes | `echelle_m` → `eva_reglage.echelle` | 20 | sur 20 |
| C2 | Barèmes différents (10, 20, 40) | `ponderation` → `ponderation` | `notes_ramenees` | notes ramenées à 20 puis pondérées |
| C3 | Arrondi **d'affichage** | `affichage` → `arrondi_precision`, `arrondi_mode` | centième, demi vers le haut | centième, demi vers le haut |
| C4 | Arrondi d'**étape** (note ramenée, sous-matière, matière, période) | `arrondis.*` → `arrondi_*_precision`, `arrondi_*_mode` | aucun (calcul exact) | **inconnu** : c'est le premier suspect d'un écart de 0,01 |
| C5 | Coefficients d'évaluation | `EvalCalcul.coef_m` → `eva_evaluation.coef` | 1, décimaux admis, 0 admis (ne pèse rien) | décimaux admis [H] |
| C6 | Coefficient de matière | `eva_coefficient_matiere` (classe > MEF > niveau > défaut, daté) | 1 sans ligne | par classe et par matière [H] |
| C7 | Absent, dispensé, non noté, à rattraper | statuts, **non réglables** (R-V5) | exclus de la moyenne | « Abs » exclu, parfois compté 0 selon le paramétrage [H] |
| C8 | Travail non rendu | `non_rendu` → `non_rendu` | `exclu` | exclu ou 0 selon l'établissement [H] |
| C9 | Notes bonus | `bonus` → `bonus`, `bonus_seuil` | `ignore` | points au-dessus de 10 ajoutés, ou note comptée normalement [H] |
| C10 | Note au-delà du barème / négative | `hors_bareme_autorise` par évaluation, `plafonner_a_l_echelle` | refusée à la saisie ; plafonnée à 20 si autorisée | acceptée ou refusée [H] |
| C11 | Sous-matières | `sous_matieres` → `sous_matieres`, `eva_coefficient_sous_matiere` | moyenne des moyennes de sous-matières | idem ou « toutes les notes à plat » [H] |
| C12 | Moyenne générale | `generale` → `generale`, `hors_moyenne_generale` | pondérée par les coefficients de matière | pondérée [H] ; calculée sur moyennes de matière arrondies ou non [H] |
| C13 | Moyenne annuelle d'une matière | `annuelle` → `methode_annuelle`, poids de période | moyenne des moyennes de période, poids égaux | moyenne des trois trimestres [H] |
| C14 | Moyenne générale annuelle | fonction `moyenne_annuelle_periodes` sur les générales de période, ou sur les matières annuelles | à partir des matières annuelles | moyenne des trois générales [H] |
| C15 | Moyenne de classe, minimum, maximum | `statistiques` | moyenne des moyennes d'élèves | moyenne des moyennes [H] ; parfois « de toutes les notes » |
| C16 | Médiane (effectif pair) | `mediane` → `mediane_paire` | moyenne des deux valeurs centrales | non affichée [H] |
| C17 | Seuil d'effectif des statistiques | `seuil_effectif_stats` → `seuil_effectif_stats` | 5 | aucun seuil [H] |
| C18 | Rang et ex æquo | **non calculé** (S-108, Q-EV5) | — | calculé, ex æquo en « 3, 3, 5 » ou « 3, 3, 4 » [H] |
| C19 | Moyenne en attente / nombre minimal de notes | statut `en_attente` (motif interne) | une seule note suffit | une seule note suffit [H] |
| C20 | « Compte si elle améliore », « ne compte pas » | `comptage` par évaluation | disponibles | « note facultative » ou coefficient 0 [H] |
| C21 | Élève arrivé en cours de période | public à la date (socle 5), `non_concerne` | moyenne sur ses seules évaluations | idem [H] |
| C22 | Matière sur plusieurs cours (changement de groupe) | pool des évaluations de tous les cours de la matière | toutes comptent | idem [H] |
| C23 | Précision d'export (LSU, LSL, Cyclades) | à poser par le socle 9 | deux décimales (LSU), jusqu'à 9 (Cyclades) [V mémento] | — |

---

## 2. Les règles, une par une

### C1 — Échelle
- **ScolIA [F]** : 20 par défaut ; 10 et 100 acceptés (`echelle_m`). Gelée pour l'année dès le premier arrêt de période (R-V21, déclencheur en base). En 3ᵉ, l'échelle 20 est imposée (R-03).
- **Charlemagne [H]** : sur 20.
- **Question à Florian** : *Sur quelle échelle Charlemagne affiche-t-il les moyennes de votre établissement (20 partout, ou 10/100 pour certaines classes) ?*

### C2 — Barèmes différents ramenés à 20
- **ScolIA [F]** : `notes_ramenees` — chaque note est ramenée à l'échelle (`note / barème × 20`), puis moyenne pondérée par les coefficients. Alternative disponible : `points_cumules` — Σ(note × coef) / Σ(barème × coef) × 20. **Les deux donnent le même résultat quand tous les barèmes sont égaux et divergent sinon.**
- **Charlemagne [H]** : notes ramenées à 20 (équivalent à `notes_ramenees`).
- **Question à Florian** : *Dans Charlemagne, une interrogation sur 10 (coef 1) et un devoir sur 20 (coef 1), avec 10 aux deux, donnent-ils 15,00 (note ramenée : 20 et 10) ou 13,33 (points cumulés : 20/30) ?* → cas-test **T1**.

### C3 — Arrondi d'affichage
- **ScolIA [F]** : au centième, **demi vers le haut** (12,345 → 12,35), à l'affichage et à l'export seulement. Précisions possibles : millième, centième, dixième, quart de point, demi-point, unité. Modes : demi haut, demi bas, demi vers le pair, supérieur, inférieur.
- **Charlemagne [H]** : centième, demi vers le haut.
- **Question à Florian** : *Une moyenne qui vaut exactement 12,345 s'affiche-t-elle 12,35 ou 12,34 dans Charlemagne ? Et 12,355 ?* → cas-test **T3** (12,35 = demi haut ; 12,34 = demi bas ou demi vers le pair ; 12,36 pour 12,355 distingue le demi vers le pair du demi bas).

### C4 — Arrondi d'étape (le suspect n° 1 d'un écart de 0,01)
- **ScolIA [F]** : **aucun** par défaut. Quatre étapes réglables : (a) **chaque note ramenée à 20** avant la pondération ; (b) la **moyenne de chaque sous-matière** avant d'être pondérée dans la matière ; (c) la **moyenne de matière** avant d'entrer dans la moyenne générale ; (d) la **moyenne de période** avant d'entrer dans la moyenne annuelle.
- **Charlemagne [H]** : plusieurs logiciels de notes arrondissent la moyenne de matière avant de calculer la générale (c) ; certains arrondissent aussi les notes ramenées (a). **Rien n'est vérifié.**
- **Questions à Florian** : *(1) La moyenne générale de Charlemagne est-elle calculée à partir des moyennes de matière **telles qu'affichées** (arrondies) ou des moyennes exactes ? (2) La moyenne annuelle part-elle des moyennes de trimestre affichées ou exactes ?* → cas-tests **T2** (étape a), **T10** (étape c) et **T11 bis** (étape d).
- **Comment un écart se verra** : écart de 0,01 (rarement 0,02) sur la générale, jamais sur une matière à une seule note.

### C5 — Coefficients d'évaluation
- **ScolIA [F]** : décimaux admis (NUMERIC(6,3)), de 0 à 100 ; **0 est admis et ne pèse rien** (l'évaluation reste visible, jamais comptée ; si tous les coefficients sont nuls, aucune moyenne).
- **Charlemagne [H]** : coefficients décimaux ; le 0 est une façon de « ne pas compter ».
- **Question à Florian** : *Charlemagne accepte-t-il un coefficient 0 ? un coefficient 0,5 ? Quel maximum ?*

### C6 — Coefficient de matière
- **ScolIA [F]** : par cible, la plus précise gagne (classe > MEF > niveau > défaut), **daté** (un changement ne touche pas une période arrêtée, R-V23) ; sans ligne : 1 ; une matière peut être « hors moyenne générale ».
- **Charlemagne [H]** : coefficient par classe et par matière (et par matière de groupe).
- **Question à Florian** : *Les coefficients de matière de Charlemagne sont-ils définis par classe, par niveau, ou par option ? Peut-on les changer en cours d'année, et le changement s'applique-t-il aux moyennes déjà calculées ?*

### C7 — Absent, dispensé, non noté, à rattraper
- **ScolIA [F]** : tous exclus de la moyenne (numérateur **et** dénominateur). **Aucun réglage ne convertit une absence en zéro** (R-V5, règle nationale, mémento eduscol 2025 : *« la note zéro ne peut pas être utilisée pour sanctionner l'absence non justifiée »*) ; la base refuse même `absent_compte_zero = true` (migration 460).
- **Charlemagne [H]** : « Abs » exclu par défaut ; mais **certains établissements** paramètrent « absent = 0 ». Si c'est le cas chez vous, **l'écart sera systématique et expliqué** : l'outil de comparaison calcule pour cela `diagnostic_absents_comptes_zero` (« que vaudrait la moyenne si les absents comptaient 0 ? »), **jamais** branché sur une écriture ni une publication.
- **Questions à Florian** : *(1) Dans Charlemagne, un élève absent à un DS reçoit-il « Abs » (exclu), 0, ou autre chose ? (2) Cela dépend-il de la justification de l'absence ? (3) Existe-t-il un réglage d'établissement pour cela ?* → cas-test **T4**.

### C8 — Travail non rendu
- **ScolIA [F]** : `non_rendu = exclu` par défaut (Q-EV1) ; `zero` possible par réglage : c'est le **seul zéro** que le moteur sait produire, distinct de l'absence.
- **Charlemagne [H]** : un devoir non rendu reçoit souvent 0 (« NR » = 0 ou exclu selon le paramétrage).
- **Question à Florian** : *Dans Charlemagne, un élève qui n'a pas rendu son devoir à la maison reçoit-il 0 ? une mention « NR » exclue de la moyenne ?* → cas-test **T5**.

### C9 — Notes bonus
- **ScolIA [F]** : une évaluation de comptage `bonus` est **ignorée** par défaut ; réglable : (i) `comme_normale` ; (ii) `points_au_dessus` d'un seuil (en millièmes de l'échelle) : Σ max(0, note ramenée − seuil) × coef / Σ coef des évaluations non bonus, **ajouté à la moyenne** (puis plafonné à l'échelle). **Cette formule est une proposition [R]**, non celle d'un logiciel connu.
- **Charlemagne [H]** : « note bonus » = points au-dessus de 10 ; ajoutés à la moyenne ou au total des points.
- **Questions à Florian** : *(1) Charlemagne a-t-il une notion de note bonus, ou est-ce un coefficient ? (2) Si oui, les points sont-ils ajoutés à la **moyenne** (plafonnée à 20 ?) ou au **total des points** ?* → cas-test **T8**.

### C10 — Note au-delà du barème, note négative
- **ScolIA [F]** : refusée à la saisie (« 23 sur 20 : vérifiez le barème ») sauf évaluation marquée « hors barème autorisé » ; une note au-delà, **déjà présente** (reprise de l'existant), est ignorée par le moteur comme par l'ancien et listée au rapport (`hors_bareme`). Moyenne plafonnée à l'échelle si le réglage `plafonner_a_l_echelle` (défaut oui).
- **Charlemagne [H]** : message d'avertissement mais acceptée, ou refusée.
- **Question à Florian** : *Peut-on saisir 22 sur 20 dans Charlemagne ? Si oui, la moyenne peut-elle dépasser 20 ?*

### C11 — Sous-matières
- **ScolIA [F]** : `moyenne_des_moyennes` — une moyenne par sous-matière, puis moyenne pondérée par le coefficient de sous-matière (défaut 1) ; les évaluations rattachées directement à la matière forment un groupe de coefficient 1 ; une sous-matière sans moyenne est ignorée. Alternative : `toutes_les_notes` (à plat, S-105).
- **Charlemagne [H]** : l'un ou l'autre ; le plus répandu est la **moyenne des moyennes de sous-matières, chacune pondérée**.
- **Question à Florian** : *Pour le français (grammaire, rédaction), la moyenne de français est-elle la moyenne des deux moyennes (pondérées par un coefficient de sous-matière), ou la moyenne de toutes les notes ensemble ?* → cas-test **T9**.

### C12 — Moyenne générale
- **ScolIA [F]** : Σ(moyenne de matière × coefficient de matière) / Σ coefficients des matières **qui ont une moyenne** ; matières « hors moyenne générale » exclues ; `generale = non_ponderee` met toutes les matières à poids égal. **Indicative : aucune décision n'en découle** (R-V20).
- **Charlemagne [H]** : pondérée ; calculée sur les moyennes de matière arrondies ou non (voir C4).
- **Questions à Florian** : *(1) La moyenne générale est-elle pondérée par les coefficients de matière ? (2) Les matières facultatives ou de vie de classe y entrent-elles ? (3) Que se passe-t-il quand une matière n'a pas encore de note (est-elle ignorée ou compte-t-elle 0) ?* → cas-test **T10**.

### C13 — Moyenne annuelle d'une matière
- **ScolIA [F]** : par défaut, **moyenne des moyennes de période à poids égaux**, une période sans moyenne étant ignorée ; seconde méthode : moyenne de **toutes les évaluations** de l'année (S-106). Poids de période réglables (`PeriodeMoyenne.poids_m`). Méthode **figée pour l'année** au premier arrêt.
- **Charlemagne [H]** : moyenne des trois trimestres (poids égaux), parfois pondérée par les périodes.
- **Question à Florian** : *La moyenne annuelle de Charlemagne est-elle la moyenne des moyennes de trimestre, ou la moyenne de toutes les notes de l'année ? Les trimestres ont-ils le même poids ?* → cas-test **T11**.

### C14 — Moyenne générale annuelle
- **ScolIA [F]** : deux chemins sont possibles et **donnent des résultats différents** : (i) moyenne générale calculée à partir des moyennes **annuelles de matière** ; (ii) moyenne des moyennes **générales de période**. Le moteur offre les deux (`moyenne_generale` sur les matières annuelles ; `moyenne_annuelle_periodes` sur les générales).
- **Charlemagne [H]** : (ii) moyenne des trois générales.
- **Question à Florian** : *La moyenne générale annuelle est-elle la moyenne des trois moyennes générales de trimestre, ou recalculée à partir des moyennes annuelles par matière ?* → cas-test **T11 bis**.

### C15 — Moyenne de classe, minimum, maximum
- **ScolIA [F]** : moyenne de classe = **moyenne des moyennes d'élèves** (pas de toutes les notes) ; minimum et maximum sur ces moyennes ; les élèves sans moyenne n'y figurent pas ; la statistique d'**une évaluation** porte sur les notes ramenées des élèves notés. Un groupe multi-classes a sa **moyenne du groupe** (R-V30).
- **Charlemagne [H]** : moyenne des moyennes d'élèves ; pour une évaluation, moyenne des notes.
- **Question à Florian** : *La « moyenne de classe » d'une matière dans Charlemagne est-elle la moyenne des moyennes des élèves, ou la moyenne de toutes les notes de la matière ? Un élève sans note y compte-t-il ?* → cas-test **T13**.

### C16 — Médiane
- **ScolIA [F]** : médiane des moyennes ; effectif pair : moyenne des deux valeurs centrales (réglable : inférieure, supérieure).
- **Charlemagne [H]** : pas de médiane.
- **Question à Florian** : *Charlemagne affiche-t-il une médiane ? Si oui, que donne-t-il pour 10, 12, 14, 20 ?* → cas-test **T12**.

### C17 — Seuil d'effectif des statistiques
- **ScolIA [F]** : aucune statistique montrée à une famille ou à un élève sous **5 élèves notés** (R-V12) ; réglable (1 à 100), jamais moins de 1. Le professeur et la direction voient toujours.
- **Charlemagne [H]** : aucun seuil.
- **Question à Florian** : *Un groupe d'option de 3 élèves affiche-t-il sa moyenne de classe aux familles dans Charlemagne ?* **Écart assumé** : ScolIA masque ; pour comparer, la direction regarde la statistique côté personnel.

### C18 — Rang
- **ScolIA [F]** : **non calculé** (décision S-108 / Q-EV5 : « jamais dans la première version »). Écart de parité signalé à Florian (S-126).
- **Charlemagne [H]** : rang affiché ; ex æquo « 3, 3, 5 » (compétition) ou « 3, 3, 4 » (dense).
- **Question à Florian** : *Souhaitez-vous le rang pour la comparaison ? (c'est un écart de fonction, pas de calcul) Et comment Charlemagne numérote-t-il les ex æquo ?*

### C19 — Moyenne en attente, nombre minimal de notes
- **ScolIA [F]** : une seule note suffit pour une moyenne ; la direction peut marquer une moyenne « en attente » (`eva_moyenne_statut`, motif interne) — c'est le « EA » de LSU. Un élève sans aucune note comptée n'a **aucune** moyenne (jamais 0).
- **Charlemagne [H]** : une seule note suffit ; « moyenne non significative » possible.
- **Question à Florian** : *Charlemagne a-t-il un nombre minimal de notes pour calculer une moyenne ?*

### C20 — « Compte si elle améliore », « ne compte pas »
- **ScolIA [F]** : deux variantes par évaluation. « Compte si elle améliore » : ajoutée dans l'ordre des notes ramenées décroissantes, retenue seulement si elle **fait monter** la moyenne (donc indépendante de l'ordre de saisie) ; sans moyenne de base, la première compte. « Ne compte pas » : publiée, jamais comptée.
- **Charlemagne [H]** : « note facultative » ou coefficient 0.
- **Question à Florian** : *Charlemagne a-t-il une note « facultative » ou « ne compte que si elle améliore » ? Comment est-elle comptée s'il y en a deux ?*

### C21 à C22 — Parcours de l'élève
- **ScolIA [F]** : un élève arrivé le 4 novembre n'est pas « absent » aux évaluations d'octobre : il n'était **pas concerné** (statut calculé, jamais saisi) ; un changement de groupe garde les notes des deux cours dans la moyenne de matière.
- **Charlemagne [H]** : l'élève arrivé en cours ne reçoit aucune note aux évaluations antérieures ; les notes d'une ancienne classe suivent (ou non) l'élève.
- **Question à Florian** : *Quand un élève change de classe en cours de trimestre, Charlemagne reprend-il ses notes de l'ancienne classe dans la moyenne de la nouvelle ?*

---

## 3. Cas-tests à saisir dans Charlemagne (15 minutes)

Chaque cas utilise **un élève fictif dans une classe de test** (jamais de données réelles : voir I-032 sur les données réelles hors chaîne de développement). Les résultats ScolIA sont ceux du moteur (tests `evaluation_moteur`) ; la dernière colonne dit ce que révèle un résultat différent.

| Cas | Saisie | ScolIA | Si Charlemagne donne… | Règle |
|---|---|---|---|---|
| **T1** | interro 10/10 coef 1 ; DS 10/20 coef 1 | **15,00** (notes ramenées) | 13,33 → points cumulés | C2 |
| **T2** | devoir 1/3 coef 1 ; devoir 7/20 coef 1 | **6,83** (exact : (6,6667 + 7) / 2) | 6,84 → chaque note ramenée est arrondie à 0,01 avant la moyenne | C4 (a) |
| **T3** | devoirs 12/20 et 12,69/20, coef 1 | **12,35** (demi haut) | 12,34 → demi bas ou demi vers le pair ; essayer aussi 12,355 | C3 |
| **T4** | DS 14/20 ; DS suivant « Abs » | **14,00** (absent exclu) | 7,00 → l'absence compte 0 (réglage d'établissement) | C7 |
| **T5** | DS 14/20 ; DM « non rendu » | **14,00** (exclu) | 7,00 → non rendu = 0 | C8 |
| **T6** | devoir 12/20 coef 0 ; devoir 15/20 coef 1 | **15,00** | 13,50 → le coefficient 0 est refusé ou traité comme 1 | C5 |
| **T7** | saisir 22/20 | refusé | accepté → moyenne pouvant dépasser 20 ? | C10 |
| **T8** | devoirs 12 et 14 (coef 1), plus une note bonus de 16 (seuil 10) | moyenne 13 ; avec `points_au_dessus` : **16** (+3 = 6 × 1 / 2) | 13,00 → bonus ignoré ; 14,00 → comptée comme normale ; autre → formule propre à Charlemagne à noter | C9 |
| **T9** | français : grammaire 10 et 12 ; rédaction 18 ; coefficients de sous-matière 1 et 3 | **16,25** ; coefficients 1 et 1 : **14,50** ; à plat : **13,33** | l'un de ces trois nombres identifie la méthode | C11 |
| **T10** | maths 12 (coef 3) ; français 8 (coef 1) | **11,00** (pondérée) ; non pondérée : 10,00 | 10,00 → générale non pondérée | C12 |
| **T10 bis** | deux matières à 10,04 et 10,06 (coef 1) | **10,05** (exact) ; avec arrondi de matière à l'unité : 10,00 | 10,10 ou 10,00 → arrondi des moyennes de matière avant la générale | C4 (c) |
| **T11** | trimestre 1 : une note 20 ; trimestre 2 : quatre notes 10 | **15,00** (moyenne des périodes) ; toutes évaluations : **12,00** | 12,00 → moyenne de toutes les notes de l'année | C13 |
| **T11 bis** | trimestre 1 : générale 10,04 ; trimestre 2 : 10,06 | **10,05** ; avec arrondi de période à l'unité : 10,00 | 10,00 → moyenne annuelle sur moyennes de période arrondies | C4 (d), C14 |
| **T12** | moyennes d'élèves 10, 12, 14, 20 | médiane **13** | autre valeur ou absence d'affichage | C16 |
| **T13** | élève A : une note à 20 ; élève B : quatre notes à 10 | moyenne de classe **15,00** (moyenne des moyennes) | 12,00 → moyenne de toutes les notes | C15 |

---

## 4. Protocole de rapprochement sur de vrais élèves (futur outil de comparaison)

1. **Export Charlemagne d'une période** (notes, coefficients, moyennes de matière et générale, moyenne de classe) — format à lire sur l'exemple, le moment venu (rien n'a été vu ici).
2. **Recalcul ScolIA** : `PgEnt::eva_v1_moyennes_eleve_periode` donne, pour chaque élève, la moyenne de chaque matière, la générale et **l'explication ligne à ligne** (`MoyenneCalculee::explication_fr`).
3. **Rapport d'écarts** (lot « comparaison », à venir) élève par élève, matière par matière, avec la **cause probable**, déduite de ces familles : arrondi d'étape (écart ≤ 0,02 sur une moyenne composée), coefficient (écart sur une matière), note non comptée (nombre d'évaluations comptées différent), absence comptée 0 (`diagnostic_absents_comptes_zero` réconcilie l'écart), barème (`points_cumules` réconcilie), sous-matière (la méthode alternative réconcilie), bonus.
4. **Calage** : poser les réglages de l'établissement jusqu'à zéro écart ; la page de réglage (V3) montre chaque règle avec un exemple chiffré ; les versions antérieures restent dans `eva_reglage_historique` pour expliquer un calcul passé.

## 5. Ce qui ne se règle pas (et pourquoi)

- **Absence = zéro** : interdit (R-V5, règle nationale). Une absence ne devient jamais une note ; si Charlemagne le fait, l'écart est **expliqué** par le diagnostic, pas reproduit.
- **Moyenne = décision** : aucune mention, aucun passage, aucune orientation n'est calculée par ScolIA (R-V20).
- **Rang** : pas de rang en première version (S-108).
- **Statistiques sous 5 élèves aux familles** : seuil de confidentialité (R-V12).

## 6. Questions à Florian, par ordre de rendement

Les neuf questions suivantes lèvent à elles seules l'essentiel du risque d'écart ; chacune se résout en saisissant le cas-test indiqué dans Charlemagne.

1. **T1** — barèmes différents : notes ramenées ou points cumulés (C2) ?
2. **T4** — l'absence compte-t-elle 0 dans votre établissement (C7) ?
3. **T5** — le non-rendu compte-t-il 0 (C8) ?
4. **T10 bis** — la moyenne générale part-elle des moyennes de matière arrondies (C4 c) ?
5. **T2** — chaque note ramenée est-elle arrondie avant la moyenne (C4 a) ?
6. **T3** — arrondi du demi : haut, bas, ou vers le pair (C3) ?
7. **T11 / T11 bis** — moyenne annuelle : moyenne des périodes ou de toutes les notes ; générale annuelle (C13, C14) ?
8. **T9** — sous-matières : moyenne des moyennes ou à plat (C11) ?
9. **T13** — moyenne de classe : des moyennes ou de toutes les notes (C15) ?

Les questions C5, C6, C8 à C10, C16 à C22 se posent au fil de la comparaison, quand un écart les rend utiles.
