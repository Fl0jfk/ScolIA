# Passe « personas » sur le socle 21 — Évaluations et notes (avec les attentes des socles 22 et 23)

**Date :** 2026-11-09 — **Statut :** proposition, **non décidée** (à relire par Florian) ; les recommandations des bloquants s'appliquent par défaut (D-031). Aucun code écrit ni modifié ; aucun accès à une base, à un secret ni à un service de production ; l'application Next.js n'a pas été ouverte.
**Branche :** `ent-personas-socle-21` (base `ent-mc0hoe` `ffda595`, qui porte `21_EVALUATIONS.md`).
**Éprouve :** `docs/socles/21_EVALUATIONS.md` (lu en entier), avec `CLAUDE.md`, `docs/PILOTAGE.md`, `docs/DECISIONS.md` (D-021 à D-033, en particulier **D-022**, **D-023**, **D-031**, **D-032**), `docs/MATRICE_CHARLEMAGNE.md` §3.7 (NOT-01 à NOT-19) et §5 (G9, G12, G28, G35), `docs/socles/09_INTEROP.md` (cohabitation : Q-X34, R-X24, FX-107), `docs/A_FOURNIR_PAR_FLORIAN.md` (F-06, F-07) ; forme reprise de `PERSONAS_10.md`.
**Fait de départ (D-032)** : au lycée pilote, **les notes et les bulletins restent dans Charlemagne l'an 1** ; la démarche éditeur est reportée vers mai 2027.
**Ajouts :** faits **directement** dans `21_EVALUATIONS.md`, marqués **« ajout personas »** : §0 point 10 ; §5 (R-25 à R-39) ; §6.3 ; §8.9 ; §9.4 ; §11.5 (CE-65 à CE-93) ; §12.3 ; §13.8 ; §14.6 ; §16 (cohabitation) ; §17.15 (EVA-233 à EVA-280) ; §18 (R-V29 à R-V47) ; §21.1 ; §23.3 (lots V14 à V16 et compléments de V1, V3, V4, V11) ; §25 (S-121 à S-131) ; §26 (Q-EV16 à Q-EV25) ; §27 (C6 à C13). **Rien n'est supprimé** ; quand un ajout contredit le texte d'origine, la contradiction est écrite au §27 de la conception.
**Numérotation :** les manques continuent `PERSONAS_10.md` (qui s'arrête à M-578) : **M-579 à M-668**. Réglages : **S-121 à S-131** (après S-120 du socle 21).

---

## 0. À lire en premier : les trois bloquants et leur recommandation (appliquée par défaut, D-031)

La conception est solide sur ce qui compte le plus : un moteur de calcul exact, les quatre gestes (saisir, publier, arrêter, figer), l'absent qui n'est jamais un zéro, le lot V0 qui ferme les fuites. En la faisant vivre par seize personas, **trois points bloquants** apparaissent. Les deux premiers changent le **schéma** du lot V1 ; le deuxième change **l'ordre** des lots pour le pilote.

| # | Point | Pourquoi c'est bloquant | Recommandation (appliquée par défaut, D-031) | Lots |
|---|---|---|---|---|
| **A** | **La période et le verrou sont portés par l'évaluation**, alors qu'un cours peut réunir des élèves de classes qui n'ont **ni la même date d'arrêt** (CE-41 : conseils étalés) **ni la même périodisation** (option de latin 2ᵈᵉ–1ʳᵉ : trimestres d'un côté, semestres de l'autre). `eva_evaluation.periode_id` (§21) et R-V1 supposent une seule période par évaluation (P02, M-587, M-588) | Un professeur de spécialité qui a des élèves de six classes ne peut pas saisir un devoir du 1ᵉʳ décembre : arrêté pour la 1ʳᵉ A le 28 novembre, ouvert pour la 1ʳᵉ D jusqu'au 5 décembre. Soit le logiciel refuse tout (blocage), soit il accepte tout (saisie après l'arrêt, instantané faux). C'est un défaut de modèle : il se fige en V1 | **La période et l'état se résolvent par élève** : classe de rattachement de l'élève à la date de l'évaluation → périodisation de cette classe → période → état de cette classe (R-V29). L'évaluation garde une **date** (et une période « indicative » pour l'écran du professeur, jamais utilisée pour un contrôle). La grille verrouille **cellule par cellule** ; la publication à l'arrêt, les rappels et la clôture des appréciations se font **par classe** (EVA-238, EVA-239). À poser **dans le schéma de V1** | **V1**, V2, V3, V8 ; Q-EV16 |
| **B** | **La première année du lycée pilote n'a pas de chemin outillé.** D-032 garde notes et bulletins dans Charlemagne l'an 1, mais la conception ne dit que « double saisie » (§16). Or ScolIA tient dès l'an 1 l'appel, les justifications et les liens familiaux (P06, M-611, M-612) | (1) Le bulletin Charlemagne imprime l'**assiduité** : sans export, le secrétariat ressaisit les absences de 1 100 élèves à chaque période. (2) **Les droits des parents séparés, privés d'accès, de l'élève majeur sont dans ScolIA**, pas dans Charlemagne : si Charlemagne (ou son portail) distribue les bulletins, il les envoie avec ses anciennes données, au mauvais parent dans certains cas. C'est une **fuite** probable dès le premier trimestre | **Lot V14 « cohabitation », juste après V0, avant V3** : (a) export de l'assiduité par élève et par période (demi-journées justifiées et non justifiées, retards) dans un tableur lisible par la saisie Charlemagne, et dans son format d'import s'il existe [NV, à lire sur l'exemple F-07] (EVA-233) ; (b) **dépôt en lot des bulletins PDF produits par Charlemagne** dans ScolIA, rapprochés par INE puis nom et date de naissance, jamais distribués au hasard (EVA-234, CE-89) ; (c) **distribution par ScolIA seule**, selon `recevoir_bulletins` de chaque lien, coffre ou courrier (EVA-235) ; (d) recommandation à l'établissement de **désactiver la diffusion des bulletins par le portail de l'éditeur** (CE-90) ; (e) moyennes de période importées **en lecture** pour l'historique, les alertes et la reprise de fin d'année (EVA-236) | **V14** (nouveau) ; Q-EV17 |
| **C** | **Une matière évaluée hors de l'établissement n'a pas de place.** Les résultats externes (§17.7, EVA-176–177) ne comptent qu'**à l'intérieur du cours d'un professeur** (`compte_dans_cours_id`). Un élève de 3ᵉ qui suit sa LV2 (russe) au CNED, une élève de 1ʳᵉ qui suit une spécialité dans un autre lycée, un élève de CHAM évalué par le conservatoire n'ont **aucun professeur** dans l'établissement pour cette matière (P15, M-652, M-654) | En 3ᵉ, une **moyenne annuelle chiffrée** est exigée pour chaque enseignement obligatoire (R-03 [V]) ; sans elle, le contrôle R-V22 **bloque la clôture** de l'année de toute la classe, et le LSU part incomplet ; au lycée, la moyenne de la spécialité suivie ailleurs entre au contrôle continu du baccalauréat (R-15) et au LSL. Le cas est réel, régulier, et la conception ne permet pas de le traiter | **Enseignement suivi à l'extérieur** déclaré par la direction ou le secrétariat (organisme, période, référent, pièce) ; une **moyenne de période externe** (et son appréciation transcrite) saisie par le secrétariat **tient lieu de moyenne de matière** quand la direction l'a déclaré (R-V38) ; source imprimée (« évalué par le CNED ») ; relance de l'organisme avant l'arrêt (EVA-240 à EVA-242). Rien ne passe automatiquement entre deux établissements ScolIA (D-013, CE-68). Tables dans **V1**, écrans en **V16** | **V1**, V16 ; Q-EV18 |

**Trois décisions importantes** (ne changent pas le schéma de V1 mais doivent être tranchées avant V7 et V15) : parent **sans exercice** de l'autorité parentale (destinataire par défaut des bulletins et des décisions d'orientation, R-V43, Q-EV19) ; **prénom d'usage** sur les documents internes (R-V36, Q-EV21) ; **aucune sanction au bulletin** et **aucune rétention pour impayé** (R-V32, R-V33, Q-EV20).

---

## 1. Méthode

**Démarche.** Celle de `PERSONAS_10.md` §1 : seize personas réalistes, prises dans la consigne du pilote et dans le tableau « acteurs » du §3 de la conception, couvrant collège et lycée, public et privé sous contrat (le lycée pilote), parcours ordinaires et particuliers. Pour chacune : son année (de la rentrée aux archives), puis ses **incidents** (arrêt des notes, conseil, correction, contestation, changement de situation familiale, départ). Chaque fois qu'elle voulait faire quelque chose que la conception ne prévoit pas, ou que c'était trop long, ambigu, risqué ou frustrant, le manque est noté.

**Grille transversale** posée à chaque persona : qui saisit, qui publie, qui décide ; ce que voit chacun (D-022, droits du lien du socle 4, minimisation) ; ce qui se révèle par un libellé, une rubrique, une absence ; l'élève **individuel** (parcours atypique) ; ce que change un événement de mi-période ; à qui et quand on le dit ; ce que l'IA ajoute **sans noter ni décider** (D-032) ; ce que Charlemagne et Pronote font déjà (parité, D-024).

**Couverture du marché.** La ligne §3.7 de la matrice (NOT-01 à NOT-19) et les fonctions publiques **lues le 2026-11-09** sur les pages des éditeurs : nouveautés Pronote 2026 et 2025, page « logiciel de gestion de la vie scolaire » (Index Éducation) ; pages EcoleDirecte et Charlemagne CFA/CFC (Aplim). Les pages Charlemagne lues **ne décrivent pas** le module de notes en détail : la parité Charlemagne repose sur l'expérience de Florian [H]. Détail au §4.

**Vérification réglementaire.** Un sous-agent de recherche a lu les textes sur Légifrance, eduscol et EUR-Lex ; les résultats sont versés au §5 de la conception (R-25 à R-39) avec leur statut [V] ou [NV] ; les points qui changent un manque sont rappelés ici.

**Gravité** (identique aux passes précédentes).
- **bloquant** : sans lui, l'établissement contourne de façon dangereuse (fuite, décision injuste non contestable) ou ne peut pas traiter un cas réel ; ou la conception se contredit au point d'empêcher de coder.
- **important** : cas fréquent sans solution propre, perte de temps forte ou risque d'erreur ; ou fonction de Charlemagne / Pronote absente.
- **confort** : améliore nettement le quotidien ; un contournement existe.
- **différenciant** : peu de logiciels du marché le font [H, comparaison avec les pages lues] ; argument de vente.

**Conventions.** [V] lu sur une source officielle (URL au §5 de la conception) ; [NV] non lu ; [H] hypothèse ; [R] recommandation ; [F] fait vérifié dans le dépôt.

**Limites.** Les personas sont construites, pas interviewées. Les pratiques de terrain (rattrapages collectifs du mercredi, relevés de mi-trimestre, remise en main propre des bulletins, demandes de duplicata d'anciens élèves, rétention de bulletins pour impayés dans certains établissements privés) sont des hypothèses fondées sur l'usage courant et sur l'expérience de Florian [H]. Les pages des éditeurs décrivent des fonctions, pas leur qualité.

---

## 2. Les personas en un tableau

| # | Persona | Établissement | Ce qu'elle éprouve surtout |
|---|---|---|---|
| P01 | **Mme Morel**, professeure de mathématiques | Collège public, 620 élèves ; groupes de besoin en 6ᵉ, deux 4ᵉ, une 3ᵉ | Saisie, copies, aménagements, rattrapages, l'IA qu'on voudrait voir corriger |
| P02 | **M. Haddad**, professeur de SES | Lycée général privé sous contrat (pilote), 1 100 élèves ; trois groupes de spécialité de 1ʳᵉ sur six classes, un groupe de terminale, deux 2ᵈᵉˢ, une option DGEMC 1ʳᵉ–terminale | Cours multi-classes, dates d'arrêt différentes, conseil de groupe, contrôle continu, Parcoursup |
| P03 | **Mme Ferrand**, professeure d'allemand en complément de service | Deux collèges publics, tous deux sous ScolIA | Deux établissements étanches, sa banque de phrases, ses soirs de conseil |
| P04 | **M. Lefèvre**, professeur principal de 3ᵉ (histoire-géographie) | Collège de P01 | Conseil, DNB, orientation, bilan de fin de cycle, rendez-vous familles |
| P05 | **Mme Duval**, cheffe d'établissement, et **M. Caron**, adjoint | Lycée de P02 | Arrêt, conseils présidés par délégation, harmonisation, publication, impayés |
| P06 | **Florian**, secrétariat de scolarité, édite les bulletins sous Charlemagne | Lycée de P02 | Cohabitation de l'an 1, impression et mise sous pli, duplicata, archives, transferts, guichet |
| P07 | **Mme Benali**, CPE | Collège de P01 | Vie scolaire au bulletin, assiduité, sanctions, décrochage |
| P08 | **Lucas**, 13 ans, élève de 4ᵉ | Collège de P01 | Comprendre, progresser, ne pas être pris de court |
| P09 | **Hugo**, élève de terminale, 18 ans le 10 décembre | Lycée de P02 | Majorité en cours de trimestre, destinataire, Parcoursup, avis pour l'examen |
| P10 | **Famille Garcia**, parents ensemble, deux enfants dans deux établissements ScolIA ; la mère lit mal le français | Collège de P01 et lycée de P02 | Comprendre, suivre deux enfants, ne pas être noyée d'avis |
| P11 | **M. Petit**, père séparé (exercice conjoint), autre ville | Collège de P01 | Son exemplaire, ses avis, rien sur l'autre parent |
| P12 | **Mme Roux** et deux variantes : parent **sans exercice** de l'autorité parentale ; parent dont l'autorité est **retirée** ; père sous **ordonnance de protection** qui ignore l'école de l'enfant | Collège de P01 | Ce qu'on doit lui dire, ce qu'on ne doit jamais lui dire, au guichet comme par courrier |
| P13 | **Mme Sanchez**, AESH mutualisée (trois élèves, deux classes) | Collège de P01 | Être prête le jour de l'évaluation, sans voir les notes |
| P14 | **Inès**, 2ᵈᵉ, dyslexique avec PAP ; **Léo**, 5ᵉ, ULIS | Lycée de P02 ; collège de P01 | Aménagements, rubrique « accompagnements », dossier Parcoursup, bulletin de la classe de référence |
| P15 | **Yanis**, 3ᵉ, LV2 russe au CNED ; **Clara**, 1ʳᵉ, spécialité arts plastiques suivie dans un autre lycée et horaires aménagés musique | Collège de P01 ; lycée de P02 | Matière sans professeur dans l'établissement, DNB, contrôle continu |
| P16 | **Emma**, déléguée de 2ᵈᵉ, et **Mme Laurent**, déléguée des parents | Lycée de P02 | Le conseil vu de la place des délégués : ce qu'on voit, ce qu'on rapporte |

---

## 3. Parcours et manques, persona par persona

Chaque tableau donne : le manque, pourquoi il compte, sa gravité, le socle concerné, et l'ajout fait (identifiant dans `21_EVALUATIONS.md`) ou le socle où il est renvoyé.

### P01 — Mme Morel, professeure de mathématiques (collège)

**Année.** *Septembre* : évaluations diagnostiques en 6ᵉ (groupes de besoin), DS d'une heure en 3ᵉ découpés en quatre exercices. *Octobre* : trois élèves de 4ᵉ ont un tiers-temps, une élève un sujet agrandi ; elle l'oublie une fois. *Novembre* : cinq absents à un DS, dont deux absents aussi au rattrapage qu'elle a improvisé sur son heure de permanence. *Décembre* : 140 appréciations.
**Incidents.** Un exercice mal énoncé (annulé pour tous) ; un QCM fait sur le cahier de textes dont elle voudrait verser les scores ; un collègue lui dit « l'IA peut corriger tes copies, non ? ».

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-579 | **Barème détaillé par exercice ou critère** (note = somme), réussite par exercice pour le professeur, détail montré à l'élève si elle le choisit | C'est ainsi que les professeurs corrigent ; l'exercice raté par toute la classe est l'information pédagogique la plus utile ; Charlemagne CFA/CFC annonce des « évaluations par critères » [V] | important | 21 | EVA-262 à EVA-264, §8.9, V3 |
| M-580 | **Rappel des aménagements de son public** à la création de l'évaluation (par type, sans diagnostic) et **sujet adapté** joint pour les élèves concernés | L'oubli d'un tiers-temps est la faute la plus fréquente et la plus mal vécue par les familles [H] ; PAP et PPS s'imposent aux évaluations (R-19 [V]) | important | 21, 35 | EVA-266, CE-85 |
| M-581 | **Rattrapage collectif** organisé par la vie scolaire (liste des élèves à rattraper toutes matières, salle, surveillant, sujets déposés, copies rendues au professeur) | Pratique courante (« rattrapages du mercredi ») [H] ; aujourd'hui chaque professeur improvise ; la conception ne connaît que le rattrapage individuel (EVA-134) | important | 21, 38 | EVA-259 à EVA-261, CE-81, V4 |
| M-582 | **Scores d'un QCM corrigé automatiquement** (socle 20) versés comme **résultats proposés**, validés par le professeur | EcoleDirecte relie QCM et carnet d'évaluations [V] ; ressaisir 30 scores est absurde | confort | 21, 20 | EVA-265, CE-82 |
| M-583 | **Exercice annulé** (énoncé faux) : note recalculée sur le barème réduit, proposée, historisée | Sans barème détaillé, le professeur recalcule à la main | confort | 21 | CE-83 |
| M-584 | **Refus explicite et expliqué** de la correction de copies et de la détection de fraude par l'IA | La demande viendra (« l'IA peut corriger ? ») ; évaluer les acquis et surveiller les élèves pendant une épreuve sont classés **à haut risque** (annexe III, point 3 b et d [V R-33]) ; D-032 : aucune note par l'IA | important | 21, 1 bis | R-V40, §13.8 |
| M-585 | **Dictée vocale** des appréciations (mobile) | Annoncée par Pronote (newsletter « dictée des appréciations », titre lu [V]) ; gain réel pour 140 appréciations | confort | 21, 11 | §12.3 |
| M-586 | Voir, pendant la rédaction des appréciations, les **éléments de ses propres séances** (oublis de matériel, travaux non faits) | Pronote 2026 l'annonce [V] ; l'appréciation s'appuie sur des faits | confort | 21, 12, 20 | §12.3 |

### P02 — M. Haddad, professeur de SES (lycée, spécialités)

**Année.** Trois groupes de spécialité de 1ʳᵉ mêlant des élèves de six classes (1ʳᵉ A à F) ; un groupe de terminale ; deux 2ᵈᵉˢ (enseignement commun, trimestres) ; une option DGEMC qui réunit des 1ʳᵉ et des terminales. Les conseils de 1ʳᵉ s'étalent du 28 novembre au 9 décembre ; les 2ᵈᵉˢ sont en trimestres, le cycle terminal en semestres.
**Incidents.** Un DS le 1ᵉʳ décembre : arrêté pour la 1ʳᵉ A, ouvert pour la 1ʳᵉ D ; une élève abandonne la spécialité en fin de 1ʳᵉ ; la fiche Avenir de Parcoursup en janvier ; six conseils où il ne peut pas aller.

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-587 | **Période et verrou par élève** dans un cours multi-classes aux dates d'arrêt différentes | §0 point A : sans cela, il ne peut pas saisir (ou saisit après l'arrêt) | **bloquant** | 21, 5 | R-V29, EVA-238, CE-65, Q-EV16, V1 |
| M-588 | Option **multi-niveaux** dont les classes n'ont pas la même périodisation (trimestres et semestres) | Même défaut de modèle, plus visible : la période elle-même diffère d'un élève à l'autre | **bloquant** | 21, 5 | R-V29, CE-66 |
| M-589 | **Moyenne « du groupe »** au bulletin pour un enseignement en groupe multi-classes (et non une « moyenne de classe » calculée sur 5 élèves de la 1ʳᵉ A) | Sinon la colonne de comparaison est fausse ou masquée (seuil de 5) pour toutes les spécialités | important | 21, 23 | R-V30 |
| M-590 | **Rappels et clôture des appréciations par classe** pour un groupe qui s'étale sur six conseils | Un seul rappel « avant le 28 » est faux pour cinq classes sur six | important | 21 | EVA-239 |
| M-591 | **Mode conseil de groupe** : passer d'un élève à l'autre de son groupe, toutes classes confondues, pendant chaque fenêtre de conseil | Pronote 2026 l'annonce [V] ; un professeur de spécialité prépare ses six conseils d'un coup | important | 21, 23 | §14.6, EVA-152 (précisé) |
| M-592 | Savoir, pour chacun des six conseils, si sa présence est attendue et pouvoir laisser une **note au président** pour ses élèves | Il ne peut pas assister à tout ; ses observations doivent arriver | confort | 23, 10 | renvoi socle 23 (avec EDT-208 du socle 10), §6.3 |
| M-593 | **Moyennes annuelles manquantes des enseignements du baccalauréat** contrôlées avant l'arrêt de la dernière période de 1ʳᵉ et de terminale (spécialité abandonnée en fin de 1ʳᵉ, EPS, enseignement scientifique) | Le contrôle continu repose sur les moyennes annuelles du livret (R-15, R-29) ; la conception ne contrôle que la 3ᵉ (R-V22) | important | 21, 38 | R-V45, EVA-280, CE-91 |
| M-594 | **Fiche Avenir et bulletins Parcoursup** alimentés par les appréciations annuelles, avec assemblage des appréciations de sous-matières | Pronote 2026 assemble les appréciations de sous-services pour Parcoursup [V] ; cf. R-17, R-31 | important | 23, 37 | §6.3, renvoi socles 23 et 37 |

### P03 — Mme Ferrand, professeure d'allemand sur deux collèges

**Année.** Douze heures au collège A, six au collège B, tous deux sous ScolIA ; une seule identité et une seule passkey (D-013), deux sessions (D-012). Elle a construit au collège A une banque de phrases de 200 formulations.
**Incidents.** Deux conseils le même mardi soir ; elle saisit une fois une note dans l'onglet du mauvais établissement ; l'IA lui propose au collège B une phrase rédigée l'an passé pour un élève du collège A.

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-595 | **Banque de phrases, préférences de style et exemples IA rangés par (personne, établissement)** ; copie vers l'autre établissement par **son** geste, sans noms d'élèves | Le modèle (`eva_banque_phrase user_id`) traverse les établissements, contrairement à D-013 (« aucune visibilité entre établissements ») ; le « style appris de ses appréciations passées » (§13.1) ferait lire au collège B des textes sur des élèves du collège A | important | 21, 1 | R-V35, EVA-243, §21.1 |
| M-596 | **Vue côte à côte** de ses deux établissements (calendrier des évaluations, conseils, rappels) **dans l'appli, sans croisement côté serveur** | Deux conseils le même soir : elle doit le voir d'un coup ; le serveur, lui, ne doit rien relier (D-013) | confort | 21, 11 | EVA-274 |
| M-597 | **Établissement toujours visible** (nom et couleur) dans l'en-tête de la grille et dans la confirmation de publication | Saisir dans le mauvais établissement est une erreur réelle de service partagé [H] | confort | 21 | §20 (note), EVA-274 |

### P04 — M. Lefèvre, professeur principal de 3ᵉ

**Année.** *Décembre* : premier conseil, vœux provisoires d'orientation. *Mars* : deuxième conseil, propositions provisoires. *Juin* : moyennes annuelles chiffrées pour le DNB (40 % de contrôle continu), bilan de fin de cycle, propositions d'orientation, rendez-vous avec les familles en désaccord.
**Incidents.** Yanis (P15) suit le russe au CNED ; un nouvel élève arrive en février de Belgique ; une famille conteste la mention ; les délégués des parents veulent un compte rendu.

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-598 | Appréciation **générale** et appréciation **du conseil** distinctes (§12.1) : dans la pratique c'est un seul texte, écrit par le PP pendant le conseil [H] | Deux champs pour la même chose ; l'un reste vide sur 30 bulletins | confort | 21, 23 | S-121 |
| M-599 | **Synthèse anonyme de la classe** (climat, résultats d'ensemble, points forts), validée par le président, pour le compte rendu des délégués des parents | Les délégués rapportent aux familles ; sans support, ils prennent des notes nominatives pendant le conseil [H] | confort | 23 | EVA-257 |
| M-600 | **Proposer un rendez-vous** à une famille depuis la fiche de l'élève au conseil (désaccord, élève à revoir) | Pronote 2026 propose des créneaux depuis la fiche navette [V] ; c'est la suite réelle du conseil | important | 23, 19 | §6.3, renvoi socle 19 |
| M-601 | Voir le **dernier bulletin de l'an passé** d'un élève redoublant ou nouveau (PP, direction, CPE), jamais ses notes détaillées ni ses mentions de comportement aux autres professeurs | Il ne connaît pas l'élève arrivé de Belgique ; il ne doit pas non plus l'étiqueter | important | 21, 23 | EVA-158 (précisé), CE-80 |
| M-602 | **Plusieurs mentions** pour un élève (encouragements et mise en garde comportement) | Pronote 2026 l'annonce [V] ; réalité des conseils | confort | 23 | §6.3 |

### P05 — Mme Duval, cheffe d'établissement, et M. Caron, adjoint

**Année.** Le projet d'évaluation est voté en septembre ; 34 conseils par trimestre, elle en préside la moitié, M. Caron et deux PP les autres. *Janvier* : la comptabilité de l'organisme de gestion lui demande de « bloquer le bulletin » de trois familles en impayé. *Mars* : un professeur s'inquiète de l'écart de moyennes entre les quatre 2ᵈᵉˢ.
**Incidents.** Elle tombe malade le jour d'un conseil ; un conseil est tenu par M. Caron ; elle veut que les bulletins de 2ᵈᵉ soient publiés le même soir, après la remise en main propre aux familles convoquées.

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-603 | **Délégation de la présidence** d'un conseil (adjoint, PP), pour un conseil ou une série, changeable le jour même | Le chef d'établissement ou son représentant préside (R-07, R-34 [V] ; la présidence par un « représentant » reste [NV]) ; aujourd'hui « présider » serait un droit d'établissement, trop large | important | 23 | EVA-256, CE-86 |
| M-604 | **Harmonisation** entre classes d'un niveau (écarts de moyennes par matière) **sans classement des professeurs**, statistiques par professeur réservées à la direction, journalisées | Utile au conseil pédagogique ; dangereux comme outil d'évaluation des personnes | important | 21 | EVA-279, R-V44 |
| M-605 | **Publication programmée des bulletins** par classe ou par niveau, et option « remis en main propre le …, en ligne le … » | Éviter que les familles de 2ᵈᵉ A comparent avant le conseil de 2ᵈᵉ B ; respecter la rencontre prévue [H] | important | 23 | EVA-247, CE-78 |
| M-606 | **Aucune rétention** de bulletin, relevé ou duplicata pour impayé | Pratique encore rencontrée dans le privé [H] ; le bulletin est une information due aux responsables (R-08 [V]) ; la conception n'en dit rien et l'outil le rendrait facile | important | 21, 23, 24 | R-V33, CE-77 |
| M-607 | **Procès-verbal** du conseil et liste des présents, archivés | Trace en cas de recours sur une décision ; durée d'utilité administrative à tenir (R-35) | important | 23, 7 | EVA-258 |
| M-608 | **« En attente » appliqué en masse** à une classe pour une matière (professeur absent tout le trimestre) | Pronote 2026 l'annonce (« non représentative » en masse) [V] | confort | 21 | EVA-171 (précisé), S-123 |
| M-609 | **Diplômes de félicitations** (trame prête, D-023) | Pronote 2026 [V] ; geste apprécié des familles | confort | 23 | §6.3 |
| M-610 | L'IA ne forme **aucun groupe d'élèves** (remédiation, niveau) et ne détecte pas la fraude | Annexe III point 3 b (« y compris pour orienter le processus d'apprentissage ») et 3 d [V R-33] | important | 21, 1 bis | R-V40 |

### P06 — Florian, secrétariat de scolarité (lycée pilote, Charlemagne)

**Année (an 1 de ScolIA, D-032).** ScolIA tient l'appel, les justifications, les familles ; Charlemagne garde notes et bulletins. *Décembre* : il doit faire imprimer les bulletins avec l'assiduité, les envoyer aux parents séparés, en poster 180. *Janvier* : trois anciens élèves demandent leurs bulletins de 2018 pour un concours. *Mars* : Parcoursup. *Juin* : archives de l'année.
**Incidents.** Un élève part en février pour Lyon ; une mère appelle au guichet pour « un double du bulletin » alors qu'une décision la prive d'accès ; un bulletin corrigé (v2) doit être réexpédié à ceux qui ont eu le papier.

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-611 | **Cohabitation an 1 : export de l'assiduité** par élève et par période pour le bulletin Charlemagne | §0 point B ; sinon ressaisie de 1 100 élèves à chaque période | **bloquant** | 21, 12, 9 | EVA-233, V14, Q-EV17 |
| M-612 | **Cohabitation : dépôt des bulletins Charlemagne** dans ScolIA, rapprochement, distribution selon les liens | §0 point B : les droits fins des liens sont dans ScolIA ; sinon fuite probable | **bloquant** | 21, 7, 4 | EVA-234, EVA-235, CE-89, CE-90 |
| M-613 | **Cohabitation : moyennes importées en lecture** (historique, alertes, reprise de fin d'année) | Sans elles, ScolIA ne peut ni alerter ni préparer la bascule de l'an 2 (FX-106) | important | 21, 9 | EVA-236 |
| M-614 | **Contrôle avant publication** des bulletins : bloquants (élève interdit de communication encore destinataire, bulletin sans moyenne ni mention « en attente », destinataire sans adresse) et avertissements (appréciation vide, texte tronqué, prénom d'un autre élève) | La direction redoute « un bulletin faux envoyé à 900 familles » (§3) ; la conception l'évoque sans le décrire | important | 23 | EVA-246 |
| M-615 | **Lot d'impression** trié par adresse, fratries regroupées dans une enveloppe, exclusions (interdiction, choix de l'élève majeur) appliquées, nombre d'envois et de pages | La mise sous pli est l'irritant n° 3 (§3) ; deux enfants, une adresse, deux enveloppes = double coût | important | 23, 7 | EVA-248, S-131 |
| M-616 | **Renvoi papier de la seule version 2** aux destinataires qui ont reçu la v1 sur papier | Réimprimer toute la classe pour une correction est le cas actuel [H] | confort | 23 | EVA-249 |
| M-617 | **Relevés de notes en lot** (classe, date) pour la rencontre parents-professeurs de mi-trimestre | Pratique courante [H] ; NOT-15 ne couvre que le bulletin | important | 21 | EVA-250 |
| M-618 | **Archives et duplicata** : retrouver un ancien élève (nom, naissance, INE, année), produire un duplicata daté avec mention | Demandes régulières d'anciens élèves (concours, dossiers à l'étranger) [H] ; le bulletin a une longue durée d'utilité (R-20, R-35) | important | 23, 7 | EVA-251, EVA-252, CE-79 |
| M-619 | **Accès de l'ancien élève** à son coffre de bulletins pendant un an après son départ | Évite la moitié des demandes de duplicata [H] | confort | 23, 7 | EVA-253, Q-EV22 |
| M-620 | **Registre annuel** (exemplaire établissement en PDF/A de tous les bulletins, par classe) et PV des conseils, versés aux archives | Charlemagne « garde tout » ; ScolIA doit le faire explicitement [H] | important | 23, 7 | EVA-254 |
| M-621 | **Dossier de transfert** à la sortie (bulletins de l'année et relevé de la période en cours, **tout publié et figé** au jour du départ) | L'établissement d'arrivée n'a sinon que des bulletins anciens ; FX-100 du socle 9 ne dit pas ce que contient le relevé | important | 21, 9 | EVA-255, R-V42, CE-88 |
| M-622 | **Bandeau de restriction** au moment de toute production manuelle (relevé, duplicata, transfert, réponse à une demande) | Au guichet, la secrétaire produit un document à la demande d'une personne : les droits du lien doivent être relus à l'instant | important | 21, 4 | EVA-244, R-V39, CE-74 |
| M-623 | **Envoi postal par prestataire** (Pronote propose Maileva [V]) | Gain réel pour 180 courriers ; mais c'est une dépendance externe à étudier (CLAUDE.md) | confort | 7, 8 | Q-EV24 |
| M-624 | **Statut scolaire ou apprenti** sur le bulletin (classes mixtes) | Pronote 2026 [V] | confort | 23 | §6.3 |
| M-625 | **Prénom d'usage** sur les documents internes, état civil sur les exports et documents à un tiers | Un bulletin au prénom de naissance pour un élève dont l'établissement a accepté le prénom d'usage blesse et expose (circulaire de 2021 sur l'identité de genre [NV R-36]) | important | 21, 2, 23 | R-V36, EVA-245, CE-75 |

### P07 — Mme Benali, CPE (collège)

**Année.** Elle rédige 300 appréciations de vie scolaire par trimestre, avec l'assiduité à côté ; elle anime la cellule de veille (groupe de prévention du décrochage).
**Incidents.** Les absences du bulletin ne correspondent pas à son tableau (la période court jusqu'au 4 décembre, l'arrêt était le 28 novembre) ; un avertissement de l'an passé est cité dans une appréciation ; un élève sportif de haut niveau déclenche l'alerte « absences aux évaluations ».

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-626 | **Bornes de l'assiduité** au bulletin : du premier jour de la période à la date d'arrêt effective de la classe, bornes imprimées | Source de contestations et d'appels [H] ; la conception ne fixe pas la borne | important | 21, 12, 23 | R-V31 |
| M-627 | **Aucune sanction disciplinaire** au bulletin ni dans l'instantané destiné aux familles | L'avertissement et les sanctions sont effacés du dossier selon des délais (R-11 [V]) ; un bulletin archivé des décennies les ferait survivre | important | 21, 13, 23 | R-V32, S-128 |
| M-628 | **Contrôle** « sanction ou fait disciplinaire ancien cité dans une appréciation » (mots « conseil de discipline », « exclusion », « avertissement ») | Même raison ; avertissement à l'auteur, comme pour la santé (R-V18) | confort | 21 | CE-93 |
| M-629 | **Signalement de décrochage** transmis à la cellule de veille (faits déterministes, expliqués, tracés ; décision humaine) | §13.3 s'arrête au PP et à la CPE ; la suite réelle est collective | important | 21, 12, 35 | EVA-277 |
| M-630 | L'alerte « absences répétées aux évaluations » **tient compte des aménagements de scolarité** (sportif de haut niveau, soins) sans en révéler la nature | Sinon fausse alerte d'« évitement » sur un élève protégé | confort | 21, 12, 35 | CE-70 |

### P08 — Lucas, 13 ans, élève de 4ᵉ

**Année.** Il regarde ses notes chaque soir sur l'appli ; il ne comprend pas pourquoi sa moyenne a baissé alors qu'il a eu 14.
**Incidents.** Sa copie est rendue, mais il ne la retrouve pas pour réviser ; il veut savoir quand arrive le bulletin.

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-631 | **Copie corrigée** (scan ou dépôt du socle 20) reliée au résultat | Réviser sur sa copie ; moins de « je l'ai perdue » | confort | 21, 20 | EVA-271 |
| M-632 | **Date prévue du conseil et du bulletin** visible de l'élève et de la famille | Question n° 1 au secrétariat en décembre [H] | confort | 21, 23 | EVA-269 |
| M-633 | **Simulateur** « si j'ai 12 au prochain DS » (élève seulement, calcul local, déterministe) | Demandé par les élèves [H] ; risque d'anxiété et de calcul faux côté famille | confort | 21 | EVA-270, S-130 |
| M-634 | **Accessibilité** du relevé (taille, contraste, lecteur d'écran, police adaptée pour les élèves dys) | Exigence générale du frontend ; plus forte sur les notes | confort | 21 | §20 (note) |

### P09 — Hugo, terminale, 18 ans le 10 décembre

**Année.** Les notes du 1ᵉʳ semestre s'arrêtent le 20 janvier ; il est majeur depuis le 10 décembre ; il habite seul depuis janvier ; ses parents paient. Parcoursup de janvier à mars.
**Incidents.** Il n'a pas encore répondu à « ce que vos parents continuent de recevoir » quand le bulletin part ; il ne comprend pas l'« avis pour l'examen » de son livret.

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-635 | **Destinataires calculés à la date de publication** (et non de génération) ; élève majeur destinataire principal ; choix non fait → défaut du socle 4, message à l'élève | Sinon le bulletin suit l'état d'il y a trois semaines | important | 21, 4, 23 | R-V47, CE-76 |
| M-636 | **Courrier à sa propre adresse** (foyer propre, R-A26 du socle 4) | Il a quitté le domicile ; le bulletin papier ne doit pas arriver chez ses parents sans son accord | confort | 4, 23 | CE-76 |
| M-637 | **Explication de l'avis pour l'examen** (LSL) et demande d'explication tracée | Un avis « doit faire ses preuves » le marque ; il doit pouvoir comprendre et demander | confort | 23 | §6.3 |

### P10 — Famille Garcia (deux enfants, deux établissements)

**Année.** Lucas au collège A, sa sœur au lycée B, tous deux sous ScolIA ; la mère lit difficilement le français.
**Incidents.** Quarante avis en une semaine de décembre ; elle ne comprend pas « mise en garde comportement ».

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-638 | **Résumé hebdomadaire** des deux enfants (faits déterministes, reformulation et traduction par l'IA facultatives) | G26 : trop de notifications ; la mère veut une vue d'ensemble | confort | 21, 8 | EVA-278 |
| M-639 | **Vue côte à côte** des deux établissements dans l'appli, sans croisement côté serveur | D-013 : le serveur ne relie pas ; l'appli peut afficher les deux sessions | confort | 11 | EVA-274 (familles) |
| M-640 | **Glossaire** des mentions, statuts et statistiques (« mise en garde », « en attente », « dispensé », « moyenne du groupe ») accessible depuis le bulletin et le relevé | Les mots du bulletin ne sont pas ceux des familles | confort | 21, 23 | §12.3 (note), §6.3 |

### P11 — M. Petit, père séparé (exercice conjoint)

**Année.** Il a `voir_notes` et `recevoir_bulletins` ; la mère aussi. Il veut son exemplaire, rien de plus, rien de moins.
**Incidents.** Il demande la révision d'une note ; il se demande si la mère « a vu » le bulletin.

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-641 | **Les appréciations suivent `recevoir_bulletins`**, pas `voir_notes` | La conception distingue les deux droits mais ne dit pas lequel ouvre les appréciations et les décisions | important | 21, 4 | R-V34 |
| M-642 | **Un parent ne voit jamais l'activité de l'autre** (lecture, accusé, demande de révision, réponse du professeur) | Pronote montre au personnel « si l'un des deux parents a répondu » [V] : légitime côté établissement, jamais côté parent ; source de conflits | important | 21, 4, 8 | R-V46 |

### P12 — Mme Roux et deux variantes : parents dont l'accès est limité

**Situations.** (a) Mme Roux **n'a pas l'exercice** de l'autorité parentale (jugement) mais ne l'a pas perdue ; (b) un père dont l'autorité parentale a été **retirée** ; (c) un père sous **ordonnance de protection**, qui ignore l'établissement de l'enfant (adresse dissimulée).
**Incidents.** (a) demande les bulletins et la décision d'orientation ; (b) téléphone au secrétariat ; (c) une relance d'impayé, un avis par courriel ou un duplicata révéleraient l'établissement.

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-643 | **Défaut pour le parent sans exercice** de l'autorité parentale : bulletins et décisions d'orientation **oui**, notes au fil de l'eau **non**, sauf décision contraire de la direction ou de la justice | Le parent qui n'a pas l'exercice garde le droit de surveiller l'éducation et doit être informé des choix importants (R-25 [V, §5]) ; « selon la décision de la direction » sans défaut produit l'oubli ou l'excès | important | 4, 21, 23 | R-V43, Q-EV19 |
| M-644 | **Ordonnance de protection, école cachée** : aucun envoi qui révèle l'établissement (bulletin papier, duplicata, réponse à une demande, avis) | Le bulletin porte l'établissement en en-tête : l'envoyer révèle ce que la justice cache ; Q-N9 du socle 8 couvre les avis, pas les documents | important | 21, 4, 8 | CE-73, R-V39 |
| M-645 | **Retrait** de l'autorité parentale : aucun document, au guichet comme en ligne ; la secrétaire voit pourquoi elle refuse, sans le dire | Le guichet est le point faible : la demande arrive par téléphone | important | 4, 21 | EVA-244, CE-74 |

### P13 — Mme Sanchez, AESH mutualisée

**Année.** Elle accompagne trois élèves dans deux classes ; elle lit les consignes à l'un, reformule pour l'autre, sert de secrétaire à la troisième.
**Incidents.** Deux de ses élèves ont une évaluation le jeudi à 10 h dans deux salles ; un sujet agrandi n'est pas prêt ; son accompagnement de Léo s'arrête en mars.

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-646 | **Sujet (adapté) partagé avec l'AESH** avant l'évaluation, sur le choix du professeur | Pour lire ou reformuler, elle doit l'avoir vu ; elle ne voit pas les notes (Q-EV9) | important | 21, 35 | EVA-267 |
| M-647 | **Alerte « deux évaluations simultanées »** d'élèves qu'elle accompagne | Elle ne peut pas être dans deux salles ; le professeur doit le savoir la veille | important | 21, 35, 6 | EVA-268, CE-84 |
| M-648 | **Fin d'accompagnement** : son accès au calendrier de l'élève s'arrête le jour même | Droits datés (§14.5) à appliquer aussi à l'AESH | confort | 21, 35 | §14.6 |

### P14 — Inès (2ᵈᵉ, PAP) et Léo (5ᵉ, ULIS)

**Année.** Inès a un tiers-temps et un sujet en police adaptée ; ses parents ne veulent pas que son PAP figure « partout ». Léo est inclus en 5ᵉ B pour quatre matières ; le coordonnateur ULIS l'évalue sur les objectifs de son PPS.
**Incidents.** Le conseil projette une « mini-fiche » avec les aménagements devant les délégués ; le dossier Parcoursup d'Inès, plus tard, porterait la rubrique « accompagnements ».

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-649 | **Rubrique « accompagnements » jamais transmise à un tiers** (Parcoursup, dossier d'admission, relevé pour un autre établissement) ; seulement à la famille et au LSU officiel | D-027 B ; C5 de la conception l'évoque pour le dossier d'admission, pas pour Parcoursup ni les relevés | important | 21, 23, 37 | R-V37 |
| M-650 | **Aménagements visibles en vue complète du conseil seulement**, jamais en vue « séance » devant les délégués | Pronote 2026 affiche une mini-fiche avec les aménagements en mode conseil [V] ; la vue séance de la conception les exclut (§14.4), à garder explicitement | important | 23 | §14.6 |
| M-651 | **Appréciation du coordonnateur ULIS** (type « dispositif ») au bulletin de la classe de référence, positionnements sur les objectifs du PPS | CE-16 prévoit « une rubrique pour le dispositif » sans auteur ni type d'appréciation | important | 21, 22, 23 | §12.3, CE-16 (précisé) |

### P15 — Yanis (3ᵉ, LV2 au CNED) et Clara (1ʳᵉ, spécialité dans un autre lycée, horaires aménagés musique)

**Année.** Le russe n'est pas enseigné au collège : Yanis le suit au CNED, qui envoie ses résultats au fil de l'année. Clara suit la spécialité arts plastiques dans le lycée voisin (convention) et la formation musicale au conservatoire.
**Incidents.** En juin, il manque la moyenne annuelle de russe : la 3ᵉ ne peut pas être close (R-V22) ; le lycée voisin est lui aussi sous ScolIA.

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-652 | **Enseignement suivi à l'extérieur** et **moyenne de période externe** valant moyenne de matière | §0 point C | **bloquant** | 21, 5 | R-V38, EVA-240, EVA-241, CE-67, Q-EV18, V1, V16 |
| M-653 | **Relance de l'organisme externe** avant l'arrêt (tâche, message préparé) | Les résultats du CNED arrivent tard [H] | confort | 21 | EVA-242 |
| M-654 | **Spécialité suivie dans un autre établissement ScolIA** : aucun passage automatique (D-013), transmission par document ou export explicite, saisie en moyenne externe | Deux établissements du même logiciel ne doivent pas se lire ; la convention ne l'autorise pas d'office | important | 21, 9 | CE-68 |
| M-655 | **Appréciation externe transcrite** (conservatoire, CHAM) avec sa source | Le bulletin des horaires aménagés porte l'appréciation de la formation musicale [H] | confort | 21, 23 | CE-69 |
| M-656 | **Périodes de l'organisme externe** différentes : rattachement à la période de l'établissement par date de résultat | Le CNED ne suit pas les trimestres du collège [H] | confort | 21 | CE-67 |

### P16 — Emma, déléguée de 2ᵈᵉ, et Mme Laurent, déléguée des parents

**Année.** Trois conseils ; Emma prépare un « pré-conseil » avec la classe ; Mme Laurent écrit un compte rendu pour les familles.
**Incidents.** Le conseil discute du cas d'Emma elle-même ; un professeur parle de la santé d'un élève.

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-657 | Geste **« délégués sortis »** : bascule en vue complète le temps d'un cas, retour en vue séance, tracé | La conception permet la bascule (§14.4), pas le retour ni la trace | confort | 23 | CE-87 |
| M-658 | **Rappel de discrétion** à l'ouverture de la vue séance | Les délégués entendent des choses sensibles ; un rappel écrit et sobre [H, R-34] | confort | 23 | §14.6 |

### Manques transverses : l'IA, les droits, la parité

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-659 | **Relecture IA des bulletins** avant publication (prénom ou pronom incohérent avec le prénom d'usage, appréciation qui cite un autre élève, texte tronqué, ton) : liste de remarques, jamais une modification | Les éditeurs lus n'annoncent pas d'IA [V] ; c'est le contrôle que le secrétariat fait aujourd'hui à l'œil | différenciant | 23, 21 | EVA-275 |
| M-660 | **Exemples de style** de l'IA pseudonymisés, sur consentement du professeur, rangés par établissement | §13.1 « appris de ses appréciations passées » : ces textes parlent d'autres élèves, souvent nommés | important | 21, 1 bis | R-V35 |
| M-661 | **« Ce qui a changé depuis le dernier conseil »** (déterministe) dans le dossier de chaque élève | Le conseil se répète d'une période à l'autre ; l'évolution est ce qu'on discute | différenciant | 21, 23 | §13.4 (ajout) |
| M-662 | **Synthèse agrégée d'une évaluation** pour le professeur (exercices les moins réussis), sans classement d'élèves | Utile ; mais touche l'« évaluation des acquis » : sous étude juridique | différenciant | 21, 1 bis | §13.8 |
| M-663 | **Note privée** du professeur présentée comme donnée de l'élève (droit d'accès) | Une note « privée » reste communicable sur demande (R-37 [V]) ; le professeur doit le savoir avant d'écrire | confort | 21 | R-V41 |
| M-664 | **Droit d'accès** : export de toutes les données d'évaluation d'un élève (résultats, historique, appréciations, notes privées) pour la réponse du DPO | RGPD art. 15 (R-37 [V]) ; aujourd'hui rien n'est prévu | important | 21, 1 | EVA-273 |
| M-665 | **Pas de note de vie scolaire ou de conduite** comptée dans une moyenne | Demande de certains établissements privés [H] ; contraire à l'esprit de R-02 (la note n'est pas une sanction) | confort | 21 | S-127, CE-92 |
| M-666 | **Rang** de l'élève (Pronote et Charlemagne le proposent [H]) | Écart de parité assumé par Q-EV5 (« jamais ») ; à rappeler à Florian | confort | 21 | S-126 |
| M-667 | **Dupliquer plusieurs évaluations** en une opération (de l'an passé, d'un cours à l'autre) | Pronote 2026 [V] | confort | 21 | EVA-125 (précisé) |
| M-668 | **Statistiques de l'évaluation commune** visibles des seuls professeurs concernés **pour leurs groupes**, comparaison anonyme des autres groupes | Comparer deux collègues nommément sur un bac blanc crée des tensions ; EVA-214 ne le précise pas | confort | 21 | R-V44 |

---

## 4. Couverture du marché : Charlemagne et Pronote (D-023, D-024)

| Fonction | Source | Dans la conception avant la passe | Après la passe | Où |
|---|---|---|---|---|
| Saisie en grille, absents de l'appel pré-remplis | Pronote 2026 [V] ; NOT-01 | conçu | conçu | EVA-128, 132 |
| Évaluations par critères, barèmes personnalisés | Charlemagne CFA/CFC [V] | absent | conçu | EVA-262 à 264 |
| QCM reliés au carnet d'évaluations | EcoleDirecte [V] | absent | conçu | EVA-265 |
| Duplication de plusieurs évaluations | Pronote 2026 [V] | une à la fois | conçu | EVA-125 |
| « Non représentative » en masse | Pronote 2026 [V] | un élève à la fois | conçu | EVA-171 |
| Plusieurs mentions pour un élève | Pronote 2026 [V] | absent | attente posée au 23 | §6.3 |
| Appréciations : filtre par groupe, éléments d'appel à côté, dictée | Pronote 2026 [V], newsletter [V titre] | en partie | conçu | §12.3, EVA-195 |
| Mode conseil de groupe | Pronote 2026 [V] | absent | conçu | §14.6 |
| Mini-fiche d'aménagements en mode conseil | Pronote 2026 [V] | exclus de la vue séance | **gardé exclus** de la vue séance, visible en vue complète | §14.6 |
| Statut scolaire / apprenti au bulletin | Pronote 2026 [V] | absent | attente posée au 23 | §6.3 |
| Diplômes de mentions | Pronote 2026 [V] | absent | attente posée au 23 | §6.3 |
| Appréciations de sous-services assemblées pour Parcoursup ; bilan par élève pour le PP | Pronote 2026 [V] | absent | attente posée aux 23, 37 | §6.3 |
| Fiche navette, créneaux de rendez-vous | Pronote 2026 [V] | absent (fiches de dialogue au 23) | attente posée aux 23, 19 | §6.3 |
| Accusé de réception par parent, visible du personnel | Pronote 2026 [V] | accusé (socle 7) | + jamais visible de l'autre parent | R-V46 |
| Envoi postal des bulletins par prestataire | Pronote (Maileva) [V] | absent | question | Q-EV24 |
| Signature électronique des bulletins | Pronote [V] ; EcoleDirecte [V] | socle 7 | inchangé | socle 7 |
| Récapitulatif des moyennes exportées vers LSU ; export même sans appréciations du dernier trimestre | Pronote 2025 [V] | pré-contrôle LSU (22) | inchangé | §16 |
| Rangs | NOT-04 [H] | refusé (Q-EV5) | **écart assumé**, signalé | S-126 |
| Relevés de notes, archives des bulletins, duplicata | expérience de Florian [H] | relevé individuel | conçu (lots, archives, duplicata) | EVA-250 à 254 |
| IA | aucune fonction d'IA annoncée sur les pages lues de Pronote et d'Aplim [V] (titre « QCM et IA », avril 2026, contenu non lu) | conçu (§13) | + exclusions explicites, relecture des bulletins | R-V40, EVA-275 |

**Ce qui rendrait ScolIA nettement meilleur** (différenciants retenus) : la moyenne expliquée (déjà conçue), la relecture IA des bulletins (M-659), « ce qui a changé depuis le dernier conseil » (M-661), la synthèse d'évaluation par exercice (M-579, M-662), le rattrapage collectif outillé (M-581), le lot d'impression par adresse (M-615), la distribution exacte des bulletins de Charlemagne pendant la cohabitation (M-612) — l'an 1 du pilote devient une démonstration plutôt qu'une contrainte.

---

## 5. Manques qui relèvent d'autres socles

| Socle | Manques | Ce qu'il faudra prévoir |
|---|---|---|
| 2 L'élève | M-625 | Prénom d'usage décidé et daté, lu par les documents internes |
| 4 Dossier administratif | M-635, M-636, M-641, M-643, M-644, M-645 | Défaut du parent sans exercice ; foyer propre de l'élève majeur ; drapeau « établissement à ne pas révéler » lu par les documents |
| 5 Structure | M-587, M-588, M-652 | Port « période de l'élève à une date » ; enseignement suivi à l'extérieur rattaché à l'offre |
| 7 Documents | M-614 à M-621 | Lot d'impression par adresse, duplicata, registre PDF/A, accès d'un ancien élève |
| 8 Notifications | M-638, M-642, M-644 | Résumé hebdomadaire multi-enfants ; aucune activité d'un parent visible de l'autre ; documents qui ne révèlent pas l'établissement |
| 11 Mobile | M-585, M-596, M-639 | Dictée vocale ; agrégation côté client de plusieurs établissements |
| 12 Appel | M-611, M-626 | Assiduité d'une période bornée, exportable |
| 13 Discipline | M-627, M-628 | Aucune sanction vers le bulletin ; vocabulaire pour le contrôle des appréciations |
| 19 Rendez-vous | M-600 | Créneaux proposés depuis le conseil |
| 20 Cahier de textes | M-582, M-631 | Scores de QCM et copies reliés aux résultats |
| 23 Conseils et bulletins | M-589 à M-609, M-614 à M-621, M-650, M-651, M-657, M-658 | Voir §6.3 de la conception |
| 24–25 Finances | M-606 | Aucun accès à la publication des documents d'évaluation |
| 35 Accompagnement | M-580, M-646, M-647, M-648 | Port des AESH par élève et par créneau ; fin d'accompagnement datée |
| 37 Orientation, Parcoursup | M-594, M-649 | Fiche Avenir depuis les appréciations annuelles ; aucune rubrique « accompagnements » |
| 38 Examens | M-581, M-593 | Rattrapages collectifs (salles, surveillance) ; moyennes manquantes du baccalauréat |
| 1 bis IA | M-584, M-610, M-659 à M-662 | Exclusions de l'annexe III ; jeu d'évaluation des relectures |

---

## 6. Synthèse

### 6.1 Manques par gravité

| Gravité | Nombre |
|---|---|
| bloquant | 5 (trois points : A = M-587, M-588 ; B = M-611, M-612 ; C = M-652) |
| important | 44 |
| confort | 38 |
| différenciant | 3 |
| **Total** | **90** (M-579 à M-668) |

**89** manques sont traités par un ajout dans `21_EVALUATIONS.md` (dont 14 sous forme d'« attente posée » aux socles 19, 23, 37) ; **1** est une question sans recommandation positive (M-623, prestataire postal, Q-EV24).

### 6.2 Ajouts dans `21_EVALUATIONS.md`

| Partie | Ajouts |
|---|---|
| Actions métier | **48** (EVA-233 à EVA-280) ; total du socle : 181 |
| Règles | **19** (R-V29 à R-V47) |
| Cas limites | **29** (CE-65 à CE-93) |
| Réglementaire | **15** lignes (R-25 à R-39 : 7 [V], 4 [V] en partie, 3 [V*] lus par un relais officiel, 1 [NV] ; les points non lus sont signalés dans chaque ligne) |
| Sections | §6.3 (attentes nouvelles aux socles 23, 37, 19), §8.9 (barème détaillé), §9.4 (période par élève), §11.5, §12.3, §13.8 (exclusions de l'IA), §14.6 (pièges de droits), §16 (cohabitation), §21.1 (modèle), §23.3 (lots) |
| Lots | V14 (cohabitation an 1), V15 (secrétariat : contrôle, impression, relevés, archives), V16 (enseignements externes, transferts) ; compléments de V1, V3, V4, V11 |
| Questions | **10** (Q-EV16 à Q-EV25) |
| Réglages signalés | **11** (S-121 à S-131) |
| Contradictions | **8** (C6 à C13) |

### 6.3 Les dix ajouts les plus importants

1. **Période et verrou par élève** (R-V29, §9.4 ; M-587, M-588). À poser dans le schéma de V1.
2. **Cohabitation de l'an 1 outillée** (§16, lot V14 ; M-611 à M-613). L'an 1 du pilote sans ressaisie ni fuite.
3. **Enseignement suivi à l'extérieur** (R-V38, EVA-240 à 242 ; M-652 à M-656). CNED, convention, conservatoire.
4. **Droits des liens précisés** : appréciations sur `recevoir_bulletins`, parent sans exercice, activité d'un parent invisible de l'autre, destinataires à la date de publication (R-V34, R-V43, R-V46, R-V47 ; M-635, M-641 à M-643).
5. **Le guichet relit les droits** (R-V39, EVA-244 ; M-622, M-644, M-645).
6. **Secrétariat outillé** : contrôle avant publication, lot d'impression par adresse, relevés en lot, duplicata, archives, transferts (lot V15 ; M-614 à M-621).
7. **Bulletin juste** : assiduité bornée, aucune sanction, moyenne du groupe, prénom d'usage, aucune rétention pour impayé (R-V30 à R-V33, R-V36 ; M-589, M-606, M-625 à M-627).
8. **Aménagements** : rappel au professeur, sujet adapté, AESH prête, rubrique jamais transmise aux tiers (EVA-266 à 268, R-V37 ; M-580, M-646, M-647, M-649).
9. **Exclusions de l'IA écrites** : pas de correction, pas de détection de fraude, pas de groupes d'élèves ; exemples de style pseudonymisés et par établissement (R-V35, R-V40 ; M-584, M-610, M-660).
10. **Évaluations détaillées et rattrapages collectifs** (EVA-259 à 265 ; M-579, M-581, M-582).

### 6.4 Ce que la passe a montré sur la conception

- **La conception pense la classe ; le lycée pense le groupe.** Les règles « par classe » (date d'arrêt, état, moyenne de comparaison, rappels) sont justes pour une classe et fausses pour un groupe de spécialité à six classes. La seule règle robuste est **par élève** (R-V29).
- **D-032 n'est pas un report, c'est un mode de fonctionnement.** Garder notes et bulletins dans Charlemagne l'an 1 exige des échanges outillés **dans les deux sens** et fait de ScolIA le **distributeur** des bulletins, parce que les droits fins des familles sont chez lui.
- **Le bulletin est un document qui dure.** Ce qu'on y imprime (sanction, aménagement, prénom, adresse, établissement) survit des décennies et circule ; chaque rubrique doit être pensée pour le destinataire **et** pour l'archive.
- **Les droits du lien ne suffisent pas au guichet.** Les fuites les plus probables ne passent pas par une route, mais par une personne qui produit un document à la demande d'une autre.
- **L'IA doit être cadrée par ce qu'elle ne fait pas.** Les demandes viendront des professeurs eux-mêmes (« corrige mes copies ») : les exclusions doivent être écrites, expliquées et visibles.
- **D-023 tient**, avec onze réglages signalés dont quatre de la conception jugés redondants (S-121 à S-124).

---

## 6 bis. Réglages signalés (D-023)

Liste recopiée dans `21_EVALUATIONS.md` §25 ; **rien n'est supprimé**. « Version automatique » = ce que fait le logiciel quand l'établissement ne touche à rien.

| # | Réglage | Pourquoi il pèse | Version automatique proposée |
|---|---|---|---|
| S-121 | Six types d'appréciations (cours, générale, vie scolaire, du conseil, annuelle, intermédiaire) | « générale » et « du conseil » sont un seul texte dans la pratique [H] | une **appréciation générale (du conseil)** ; l'appréciation du conseil séparée activable |
| S-122 | Neuf statuts de résultat, dont `a_rattraper` distinct d'`absent` et `non_rendu` | deux codes pour « absent, à rattraper » | `a_rattraper` devient un **attribut** (date de rattrapage) d'`absent` ou `non_rendu` ; codes clavier inchangés |
| S-123 | `non_significative` et `en_attente` pour un seul affichage | deux notions, une seule au bulletin et au LSU (EA) | une seule : **« en attente »**, motif interne |
| S-124 | Publication : trois modes, délai maximal, rappel à 10 jours, retrait à 24 h | cinq notions pour « publier » | un écran **« publier »** guidé ; défaut du professeur |
| S-125 | Décalage de publication entre élève et parents | réglage tentant, sans enjeu réglementaire | **ne pas faire** : même moment pour tous |
| S-126 | Rang de l'élève | parité du marché [H] | **désactivé** (Q-EV5) ; écart assumé, signalé |
| S-127 | Note de vie scolaire ou de conduite | usage ancien [H] | **ne pas faire** |
| S-128 | Sanctions imprimées au bulletin | usage [H] | **ne pas faire** (R-V32) |
| S-129 | Banque commune d'appréciations | Pronote propose une bibliothèque d'appréciations [V titre] | inchangé : S-116 (banque personnelle seulement) |
| S-130 | Simulateur de moyenne | demande des élèves [H] | **élève seulement**, calcul local, jamais côté famille |
| S-131 | Mise en forme du lot d'impression (tri, recto-verso, page d'adresse) | tentation d'un éditeur de lot | fixe : tri par adresse, fratries regroupées, recto-verso |

**Existant signalé** : `ent_eva_regle.arrondi = 'demi_point'` (S-107) et `absent_compte_zero` (S-100) sont déjà signalés par la conception ; rien de plus dans l'existant.

---

## 7. Faits, hypothèses, contradictions

**Faits [F]** vérifiés dans la conception : `eva_evaluation.periode_id` unique (§21) et R-V1 (« une période de la périodisation de sa classe ») ; date d'arrêt effective **par classe** (§9.1, CE-41) ; résultats externes comptés dans un cours (`compte_dans_cours_id`, §21) ; `eva_banque_phrase user_id` (§21) ; style « appris de ses appréciations passées » (§13.1) ; « double saisie » pour l'an 1 (§16) ; aucune règle sur l'assiduité imprimée, les sanctions au bulletin, la rétention, le prénom d'usage, le parent sans exercice (recherche dans le document : zéro occurrence pour « prénom d'usage », « impayé », « 373-2 »).

**Vérifié le 2026-11-09** [V] : fonctions des éditeurs lues (§4) ; textes du §5 de la conception (R-25 à R-39) avec leur statut propre : C. civ. 373-2-1, 378, 380, 414 ; D. 331-32 ; R. 421-51 ; R. 426-2 ; arrêté du 16/07/2018 et arrêté sur le contrôle continu (art. 12) ; arrêté du 4/03/2020 (LSL) ; annexe III de l'arrêté du 31/12/2015 ; guide eduscol du lycée (non-rendus signalés au bulletin) ; fiche Avenir 2026 ; instruction d'archives de 2005 ; relais officiels [V*] : circulaire 2006-137, règlement IA (version consolidée de la Commission), RGPD (copie CNIL).

**À vérifier [NV]** : circulaire de 2021 sur le prénom d'usage (R-36) ; présidence du conseil par un représentant et discrétion des délégués (R-34) ; qui évalue en scolarité partagée avec le CNED (R-39) ; annexes GT du LSL ; format d'import de l'assiduité dans Charlemagne (sur l'exemple F-07) ; usages de rétention des bulletins dans le privé ; composition et discrétion du conseil dans le privé sous contrat ; durées précises d'archivage des bulletins (R-35) ; points [NV] du §5 de la conception.

**Hypothèses [H]** : pratiques de terrain (rattrapages collectifs, relevés de mi-trimestre, remise en main propre, demandes de duplicata, rétention pour impayé, appréciation générale = appréciation du conseil) ; rangs chez Charlemagne et Pronote ; délais d'envoi du CNED.

**Contradictions signalées** (toutes au §27 de la conception, tranchées par défaut selon la recommandation, D-031) : C6 (R-V1 et la période par élève), C7 (« double saisie » du §16 et la cohabitation outillée), C8 (résultats externes dans un cours et moyenne externe), C9 (`eva_banque_phrase user_id` et D-013), C10 (« selon la décision de la direction » et le défaut du parent sans exercice), C11 (rubrique « accompagnements » et Parcoursup), C12 (six types d'appréciations et S-121), C13 (« échelle à 4 niveaux » du §6.1 absente de l'annexe III relue, R-38).

Les compléments de modèle entrent dans la réserve de migrations du socle (5108–5189, repli 468–477) ; aucune plage nouvelle n'est demandée.
