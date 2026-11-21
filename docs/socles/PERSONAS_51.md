# Passe « personas » sur le socle 51 — Post-bac et apprentissage

**Date :** 2026-11-11 — **Statut :** proposition, **non décidée** (à relire par Florian) ; recommandations appliquées par défaut (D-031) sauf ce qui engage l'établissement à l'extérieur (§7.2). Aucun code écrit ni modifié ; aucun accès à une base, à un secret ni à un service de production.
**Branche :** `ent-socle-50-51-conception`, partie de `origin/ent-mc0hoe`.
**Éprouve :** `docs/socles/51_POST_BAC_APPRENTISSAGE.md` (§0 à §26 et §28), avec `CLAUDE.md`, `docs/DECISIONS.md` (D-020, D-027, D-031, D-032, D-034, D-035, D-036, D-041) et les conceptions 4, 6, 7, 12, 21, 24 pour les attentes. Forme reprise de `PERSONAS_42.md`.
**Ajouts :** faits dans `51_POST_BAC_APPRENTISSAGE.md` au **§27** ; rien supprimé.
**Numérotation :** manques **M-5100 à M-5149** ; actions **PBA-264 à PBA-271** ; règles **R-PBA41 à R-PBA46** ; cas limites **CPB-63 à CPB-69** ; réglages **S-5113 à S-5115**.

---

## 0. À lire en premier : quatre bloquants et leur recommandation (appliquée par défaut, D-031)

| # | Point | Pourquoi c'est bloquant | Recommandation appliquée | Lot |
|---|---|---|---|---|
| **A** | Le parcours minimal commençait par le **contrat** ; or, dans le cas probable du lycée pilote (UFA rattachée à un CFA, M1), le CFA a déjà les contrats dans son logiciel (P01, P08) | On aurait construit d'abord ce que le CFA fait déjà, et laissé sans outil ce qui coûte vraiment au lycée : ressaisir les absences chez le CFA et prévenir les employeurs | **Export d'assiduité et d'heures vers le CFA** et **information de l'employeur** d'abord (PBA6 + PBA8), avec une **saisie minimale du contrat** (PBA-264) | PBA6, PBA8 |
| **B** | L'employeur recevait des absences que l'apprenti n'avait **pas pu voir** (P05, P13) | Une erreur d'appel envoyée au patron peut coûter un contrat ; c'est la peur n° 1 de l'apprenti | Récapitulatif **visible de l'apprenti 24 h avant** l'envoi, signalement d'erreur possible (PBA-265, R-PBA41, S-5113) | PBA6 |
| **C** | La liste CROUS était préparée et signée **sans que l'étudiant le sache** (P06, P15) | La procédure contradictoire exige que l'étudiant puisse s'expliquer avant la suspension ; une erreur coupe ses ressources | **Information préalable** de l'étudiant porté sur la liste, motif et délai (PBA-266, R-PBA42) | PBA5 |
| **D** | Le **lien employeur** sans compte est la principale surface d'attaque (P14) | Un lien transféré ou intercepté expose un mineur, ses absences, son livret | Lien nominatif + **code à usage unique** par ouverture, 30 jours, aucune liste d'apprentis, **journal des accès visible de l'apprenti** (PBA-267, R-PBA43, S-5114) | PBA7 |

---

## 1. Méthode

Quinze personas : coordination d'UFA, enseignants, colleur, vie scolaire, direction, secrétariat, gestionnaire du CFA partenaire, comptable de l'OGEC, apprenti majeur et mineur avec ses parents, étudiant boursier de BTS, étudiante de CPGE, employeur et maître d'apprentissage, **attaquants**, **auditeur RGPD**. Grille : (1) statut à la date et ce qu'il commande ; (2) qui reçoit quoi ; (3) ce qui sort vers un tiers et qui l'a validé ; (4) ce que fait l'IA ; (5) D-036. Gravité : bloquant · important · confort · différenciant. Personas construites, non interviewées [H] ; aucune donnée réelle.

---

## 2. Les personas en un tableau

| # | Persona | Contexte | Ce qu'elle éprouve surtout |
|---|---|---|---|
| P01 | **Mme Lopez**, coordinatrice de l'UFA (BTS MCO et NDRC en apprentissage) | Lycée privé, UFA rattachée au CFA de réseau (M1) | Ressaisie chez le CFA, employeurs, essais, ruptures |
| P02 | **M. Haddad**, professeur de BTS en classe mixte (12 scolaires, 10 apprentis) | Même lycée | Appel selon le rythme, CCF, stages |
| P03 | **Mme Klein**, professeure de mathématiques en MPSI et colleuse | CPGE du lycée | Colloscope, notes de colles |
| P04 | **M. Morel**, colleur vacataire extérieur (ingénieur) | CPGE | Sa liste du jour, ses heures |
| P05 | **Yanis**, 20 ans, BTS en apprentissage (repris de P18 de `PERSONAS_02-04.md`) | UFA | Ce qui part chez son patron |
| P06 | **Sarah**, 19 ans, BTS sous statut scolaire, boursière échelon 5 | Section BTS | Hospitalisation, peur de perdre la bourse |
| P07 | **Chloé**, 18 ans, MPSI, inscrite à l'université conventionnée | CPGE | Inscription parallèle, CVEC, crédits |
| P08 | **M. Durand**, gestionnaire au CFA de réseau (logiciel de type Ypareo) | CFA partenaire | Fichiers d'assiduité, certificats, OPCO |
| P09 | **Léo**, 16 ans, CAP en apprentissage, et **ses parents** | UFA (atelier) | Mineur, machines, information des parents |
| P10 | **Mme Petit**, maîtresse d'apprentissage dans une boulangerie | Employeur de Léo | Lien, livret, horaires |
| P11 | **M. Bernard**, DRH d'un magasin employeur de 4 apprentis | Employeur de Yanis | Récapitulatifs, ruptures |
| P12 | **Mme Fournier**, CPE post-bac | Lycée | Justificatifs, liste CROUS |
| P13 | **Mme Duval**, cheffe d'établissement | Lycée | Signer la liste CROUS, SOLTéA, conseil de perfectionnement |
| P14 | **Les attaquants** | — | Lien employeur transféré, employeur curieux, parent d'un majeur, étudiant AED |
| P15 | **M. Blanc**, délégué à la protection des données | Groupe | Salariés mineurs, aptitude, tiers employeurs |

---

## 3. Parcours et manques, persona par persona

### P01 — Mme Lopez, coordinatrice de l'UFA (M1)
**Son mois.** Le 1er : elle exporte les absences du mois du logiciel de vie scolaire et les ressaisit dans le portail du CFA ; elle appelle trois employeurs ; deux périodes d'essai finissent la semaine prochaine ; un apprenti n'a toujours pas d'employeur depuis septembre.
**Irritants** : la double saisie ; découvrir une rupture par l'apprenti lui-même ; les employeurs qui ne répondent pas.
**Ce qui la ravirait** : l'export mensuel au format du CFA en un geste ; les essais qui préviennent seuls ; la liste des apprentis sans employeur avec le délai restant.
**Manques**
- **M-5100 (bloquant → A)** : la saisie minimale du contrat doit suffire à l'appel et à l'essai (PBA-264) ; le reste vient du CFA.
- **M-5101 (important)** : « **apprentis sans employeur** » : liste, délai pour trouver un contrat [NV], actions d'accompagnement (PBA-269).
- **M-5102 (important)** : le **format** du CFA est imposé par son logiciel ; ScolIA doit produire un fichier au format du partenaire par un **adaptateur d'échange** (socle 9, X7), pas un format maison (CPB-69).
- **M-5103 (confort)** : bascule en masse au 1er septembre d'une classe entière en apprentissage (PBA-271), avec aperçu (R-PBA44).

### P02 — M. Haddad, professeur de BTS en classe mixte
**Manques**
- **M-5104 (important)** : à l'appel, voir d'un coup d'œil « en entreprise cette semaine » sans que ces élèves soient comptés absents (déjà R-PBA10) ; **et** voir qui revient lundi (prévoir le cours).
- **M-5105 (important)** : le **CCF** d'un apprenti absent pour son travail (employeur) : situation de remplacement (PBA-117) ; l'employeur est prévenu de la date de CCF (le récapitulatif inclut les **dates d'évaluation à venir**, sans notes).
- **M-5106 (confort)** : continuité pédagogique pendant les semaines en entreprise (cahier de textes, 20) : « ce que la classe a fait pendant votre absence » pour les apprentis au retour.

### P03 — Mme Klein, professeure et colleuse en MPSI
**Manques**
- **M-5107 (important)** : le colloscope doit gérer les **semaines de devoirs surveillés** (pas de colle) et les **vacances** (socle 5) ; et la **parité de rotation** (chaque trinôme voit chaque colleur).
- **M-5108 (confort)** : un trinôme dissous en cours d'année (départ d'une étudiante) : recomposition et reprise des rotations.

### P04 — M. Morel, colleur vacataire extérieur
**Manques**
- **M-5109 (important)** : il n'a souvent pas activé son compte ; **planning imprimable** (PBA-268) et saisie des notes sur papier ressaisies par la professeure ; heures comptées quand même.
- **M-5110 (important)** : ses **heures** ne doivent être vues que de lui, de la coordination et de la paie (R-PBA45).

### P05 — Yanis, 20 ans, BTS en apprentissage
**Sa peur** : un appel raté (il était aux toilettes) envoyé à son patron.
**Manques**
- **M-5111 (bloquant → B)** : aperçu 24 h avant envoi, signalement d'erreur (PBA-265, R-PBA41).
- **M-5112 (important)** : il veut voir **qui** a consulté son dossier côté employeur (R-PBA43, journal).
- **M-5113 (confort)** : il paie lui-même la restauration : facturation à lui (24), selon ses jours en centre.

### P06 — Sarah, 19 ans, BTS sous statut scolaire, boursière
**Son mois de mars.** Hospitalisée 3 semaines ; puis absente une semaine pour un deuil.
**Manques**
- **M-5114 (bloquant → C)** : elle doit savoir **avant** la signature qu'elle figure sur une liste (PBA-266, R-PBA42).
- **M-5115 (important)** : la règle « moins d'un mois » (R-PBA19) doit compter les absences **consécutives** ou **cumulées** sur le mois ? → à vérifier dans la circulaire 2026-2027 (Q-PBA43, [NV]) ; par défaut, **consécutives** (le texte parle de « déclarations d'absence de moins d'un mois », RA-29 du 12).
- **M-5116 (important)** : l'**échelon** de bourse ne doit être saisi nulle part (R-PBA46) ; elle veut quand même une **attestation d'assiduité** pour le CROUS en libre-service (PBA-161).

### P07 — Chloé, 18 ans, MPSI
**Manques**
- **M-5117 (important)** : l'attestation CVEC et l'inscription à l'université se font **en septembre-octobre** ; l'alerte de fin décembre est trop tard pour l'aider : rappel en octobre (S-5116 « rappel d'inscription parallèle », 15 octobre).
- **M-5118 (important)** : si elle se réoriente en février vers un BTS, l'**attestation descriptive** du S1 doit être produite avant son départ (PBA-142 déclenchée par la sortie).
- **M-5119 (confort)** : ses **notes de colles** consultables avec l'appréciation ; pas de moyenne « officielle » de colles si le professeur ne la publie pas.

### P08 — M. Durand, gestionnaire au CFA de réseau
**Ce qu'il attend du lycée** : chaque mois, par apprenti, les heures prévues, réalisées, les absences justifiées ou non ; à chaque semestre, les évaluations ; immédiatement, toute rupture apprise.
**Manques**
- **M-5120 (bloquant → A)** : fichier mensuel **figé** (instantané, l'export ne bouge plus une fois envoyé ; une correction = un rectificatif) (PBA-211).
- **M-5121 (important)** : une rupture apprise par le lycée doit être **transmise au CFA** le jour même (événement → export ponctuel).
- **M-5122 (important)** : les **certificats de réalisation** sont émis par le CFA (M1) à partir des heures du lycée : le lycée fournit les heures, **jamais** un certificat (R-PBA33 précisée : en M1, ScolIA ne signe pas de certificat).

### P09 — Léo, 16 ans, CAP en apprentissage, et ses parents
**Manques**
- **M-5123 (important)** : l'atelier de l'UFA utilise une **machine** soumise à travaux réglementés ; Léo n'a pas encore son avis d'aptitude : il est **retiré** de la séance (R-PBA25) et la séance de remplacement est proposée ; ses parents sont informés **qu'il manque l'avis**, sans contenu médical.
- **M-5124 (important)** : ses parents reçoivent le **même récapitulatif** que l'employeur (S-5115) ; ils ne voient pas le livret d'entreprise sauf si Léo le partage ? → pour un **mineur**, les titulaires voient le livret [R] (autorité parentale) ; ajouté à §22.2.
- **M-5125 (confort)** : à ses 18 ans, ses parents perdent les avis (CPB-05) : Léo est prévenu un mois avant et peut les désigner.

### P10 — Mme Petit, maîtresse d'apprentissage (boulangerie)
**Manques**
- **M-5126 (important)** : elle ouvre le lien sur son téléphone dans l'arrière-boutique : le code à usage unique par courriel est contraignant ; **SMS** possible (OVH, D-004) si elle a donné son mobile (S-5117 « code par SMS au maître d'apprentissage », défaut non : coût).
- **M-5127 (important)** : attester la **présence en entreprise** pour l'essai, en un geste chaque mois (PBA-270).
- **M-5128 (confort)** : le livret en **langage simple** (pas de codes de référentiel) ; l'IA peut reformuler la compétence (proposition, sans évaluer).

### P11 — M. Bernard, DRH d'un employeur de 4 apprentis
**Manques**
- **M-5129 (important)** : un seul lien **par apprenti** (pas de vue « tous mes apprentis ») : c'est voulu (R-PBA43) ; mais un **récapitulatif groupé** par courriel pour l'entreprise est demandé → refusé par défaut (un message par apprenti), réglage S-5118 « récapitulatif groupé par employeur » (défaut non).
- **M-5130 (important)** : il veut être prévenu des **dates de CCF et d'examen** (M-5105) : inclus dans le récapitulatif.

### P12 — Mme Fournier, CPE post-bac
**Manques**
- **M-5131 (important)** : la demande de justificatif doit partir **par une personne** (pas automatiquement au seuil) ; la liste des boursiers sous le seuil est un **outil de travail**, pas une décision.
- **M-5132 (important)** : les étudiants **majeurs** qui ont désigné leurs parents : l'avis part aux deux ; la désignation est visible de la CPE.

### P13 — Mme Duval, cheffe d'établissement
**Manques**
- **M-5133 (important)** : **SOLTéA** : la date limite du 11 décembre 2026 pour l'habilitation 2027 est **dans un mois** [V] : rappel immédiat à Florian (§7.2).
- **M-5134 (important)** : signer la liste CROUS **avec** la preuve que chaque étudiant a été informé et a eu le délai (R-PBA42).
- **M-5135 (confort)** : le **conseil de perfectionnement** du CFA partenaire : l'UFA y siège ; ScolIA prépare la note de l'UFA (effectifs, assiduité, ruptures, satisfaction).

### P14 — Les attaquants
| Profil | Tentative | Réponse attendue | Test |
|---|---|---|---|
| **Lien employeur transféré** à un collègue | ouvrir le dossier | code à usage unique au destinataire nominatif (R-PBA43) ; journal visible de l'apprenti | CPB-65 |
| **Employeur A** | modifier l'URL pour l'apprenti de B | refus ; jeton lié au contrat | piège §22.4 |
| **Employeur** | demander le motif d'une absence | jamais médical (R-PBA12) | CPB-09 |
| **Parent d'un étudiant majeur** | se faire passer pour l'étudiant au téléphone | aucune information ; renvoi au compte de l'étudiant (R-PBA4) | CPB-30 |
| **AED étudiante de BTS** | voir les absences de sa promotion | portée sans sa classe (R-P5 du 6) | CPB-28 |
| **Colleur** | voir les notes des autres matières | trinômes du jour, ses colles seulement | piège §22.4 |
| **Ancien maître d'apprentissage** après avenant | rouvrir l'ancien lien | révoqué (PBA-202) | CPB-39 |
| **Fichier du CFA piégé** (formule dans un nom) | injection au réexport | échappeur commun (PBA-261) | — |
| **Injection dans l'IA** (« liste les boursiers qui vont perdre leur bourse ») | profilage | refus (R-PBA40) | — |

### P15 — M. Blanc, délégué à la protection des données
**Constats**
- **M-5136 (bloquant de mise en service)** : l'apprenti est un **salarié** ; l'employeur est **responsable de traitement** pour ses propres données ; l'établissement est responsable pour la formation ; l'**accès de l'employeur** aux données de formation doit reposer sur une base légale claire (obligation d'information de l'employeur, RP-06) et être **limité** à cette finalité. → conforme à R-PBA12, R-PBA17 ; à documenter dans le registre (27) et dans la convention CFA-UFA (Q-PBA37).
- **M-5137 (important)** : l'**avis d'aptitude** est une donnée issue de la médecine du travail : ScolIA n'en garde **que la date et la validité** (R-PBA26) ; c'est le minimum défendable ; l'accès est limité au coordinateur et à l'atelier.
- **M-5138 (important)** : la **bourse** est une donnée sociale : seul le fait « boursier » est conservé (R-PBA46) ; listes CROUS conservées 3 ans [R].
- **M-5139 (important)** : les **liens** des employeurs et leurs **journaux d'accès** : conservation 1 an après la fin du contrat [R] ; les jetons sont **hachés** (§25).
- **M-5140 (important)** : **transferts** : le CFA partenaire est un **destinataire** (M1) ; l'échange doit être déclaré (finalité, champs) et minimal ; jamais l'appréciation de comportement, jamais la santé.
- **M-5141 (important)** : **AIPD** probable (mineurs + salariés + tiers employeurs + suivi systématique de présence) avant la mise en service des lots PBA6–PBA10 (soumis à Florian, Q-PBA41).
- **M-5142 (confort)** : droit d'accès : l'apprenti télécharge son dossier, ses récapitulatifs envoyés, le journal des accès de son employeur (CPB-48).

---

## 4. Vérification réglementaire (2026-11-11)

Vérifié pendant la conception (sources au §5 du 51) : L6211-2 [V], L6222-18 [V], R6222-21 [V-partiel], L6231-2 et L6231-3 [V-partiel], décret 2025-585 et R6332-25 [V-partiel], R4153-38 à R4153-52 [V], BTS 120 crédits et CCF [V-partiel], CPGE L612-3 et D612-19 et s. [V-partiel], bourses [V-partiel, circulaire 2026-2027 non relue], CVEC [V], SOLTéA [V]. **Ajouté par la passe** : la date limite SOLTéA du **11 décembre 2026** est imminente (M-5133). **Non vérifié** : règle de calcul « moins d'un mois » (M-5115, Q-PBA43), délai pour trouver un employeur, livret d'apprentissage obligatoire ou non, statut des colleurs.

---

## 5. Manques qui relèvent d'autres socles

| Manque | Socle | Attente |
|---|---|---|
| M-5102 adaptateur au format du CFA | 9 | adaptateur d'échange déclaré (X7) |
| M-5106 « ce que la classe a fait » au retour d'entreprise | 20 | vue de continuité par période d'absence prévue |
| M-5107 semaines sans colle | 5, 10 | calendrier : semaines de devoirs surveillés |
| M-5110 heures de colle | 49 | élément variable de paie |
| M-5126 code par SMS | 8 | nature d'envoi « code d'accès » par SMS (coût, D-028 Q-N22) |
| M-5136, M-5141 registre, AIPD | 27 | fiche de traitement « apprentissage » |

---

## 6. Synthèse

### 6.1 Manques par gravité
Bloquants : 5 (M-5100, M-5111, M-5114, M-5120 ; et le lien employeur, A à D) + 1 de mise en service (M-5136/M-5141). Importants : 31. Confort : 8.

### 6.2 Ajouts dans la conception (§27 du 51)
Bloquants A à D ; actions PBA-264 à PBA-271 ; règles R-PBA41 à R-PBA46 ; cas limites CPB-63 à CPB-69 ; réglages S-5113 à S-5115 ; parcours minimal réordonné (PBA6 et PBA8 avant PBA7). **Ajoutés ici, à reporter** (fait au §27.6 du 51) : S-5116 (rappel d'inscription parallèle au 15 octobre), S-5117 (code par SMS au maître d'apprentissage, défaut non), S-5118 (récapitulatif groupé par employeur, défaut non) ; récapitulatif employeur avec les **dates d'évaluation à venir** (M-5105, M-5130) ; export mensuel **figé** et rectificatif (M-5120) ; rupture transmise au CFA le jour même (M-5121) ; en M1, **aucun certificat de réalisation signé par ScolIA** (M-5122) ; titulaires d'un mineur : accès au livret (M-5124) ; attestation descriptive déclenchée par la sortie (M-5118).

### 6.3 Ce que la passe a montré
La conception était complète sur le droit et le modèle, mais elle pensait comme un **CFA** ; le lycée pilote est probablement une **UFA** dont le premier besoin est d'**alimenter** le CFA et de **parler juste** aux employeurs. Le cœur du socle, pour 2027, est donc le statut daté, le calendrier d'alternance, l'appel qui en tient compte, et les sorties vers le CFA et les employeurs, transparentes pour l'apprenti.

---

## 7. Décisions

### 7.1 Appliquées par défaut (D-031)
Bloquants A à D ; S-5113 = 24 h ; S-5114 = 30 jours ; S-5115 = oui ; S-5116 = 15 octobre ; S-5117 et S-5118 = non ; titulaires d'un mineur lisent le livret ; en M1, pas de certificat signé par ScolIA ; « moins d'un mois » compté en absences consécutives jusqu'à vérification.

### 7.2 Soumis à Florian
- **Q-PBA38 (urgent)** : habilitation **SOLTéA** pour 2027, dossier **avant le 11 décembre 2026** si l'établissement veut percevoir le solde en 2027.
- **Q-PBA1** : formations ouvertes, M1 ou M2, CFA partenaire et son logiciel.
- **Q-PBA37** : convention CFA-UFA (information de l'employeur, fichiers, format).
- **Q-PBA41** : AIPD et avis du DPO sur les données d'apprentis et l'accès des employeurs.
- **Q-PBA42** : rémunération des colleurs.

---

## 8. Faits, hypothèses, contradictions
- **Faits** : ceux de la conception.
- **Hypothèses** : M1 pour le lycée pilote ; pratiques des colles et des récapitulatifs.
- **Contradictions** : (1) le parcours minimal du §0 du 51 (PBA7 avant PBA8) contre le bloquant A : tranché au §27.5 ; (2) R-PBA33 (« certificat signé par la direction de l'organisme ») ne vaut qu'en M2 : précisé ; (3) §22.2 (titulaires d'un mineur sans livret) contre M-5124 : tranché, ils le lisent.
