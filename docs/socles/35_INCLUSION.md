# Socle 35 — Besoins éducatifs particuliers et inclusion

**Date :** 2026-11-11 — **Statut :** conception de fond, **non décidée** (à relire par Florian), rédigée **après** le code existant (migrations 016 et 212, `ent-db/src/accompagnement.rs`, droits `accompagnement.gerer` et `amenagements.lire`) pour le rattraper. Aucun code écrit ni modifié ; aucun accès à une base, à un secret ni à l'application Next.js en écriture.
**Branche :** `ent-socle-35-36-conception`, partie d'`origin/ent-mc0hoe` (`afeb8c4`).
**Passe personas :** `PERSONAS_35.md` ; ajouts au **§30**, qui l'emporte en cas de contradiction.
**À lire avec :** `CLAUDE.md` ; `docs/DECISIONS.md` (D-017, D-019, D-021, D-023, D-026, D-031, D-034, D-035, D-036, D-037 à D-046) ; `docs/SOCLES.md` §35 ; **`34_SANTE.md`** (frontière §7, secret, consignes, avis médical, R-S1, R-S48) et `PERSONAS_34.md` ; `02_ELEVE.md` (ELV-36 modalités de scolarisation, pastilles) ; `04_ADMINISTRATIF.md` (autorité parentale, majorité) ; `05_STRUCTURE` (inclusions ULIS) ; `06_PERSONNELS.md` (AESH P24, P58, mission « Accompagnement AESH », PRS-67) ; `10_EMPLOI_DU_TEMPS.md` §8.3 bis (planning dérivé de l'AESH, H-16) ; `12_ASSIDUITE.md` ; `13_DISCIPLINE.md` (pastille « besoins particuliers », CD-13) ; `14_ACCUEIL_SURETE.md` (aide à l'évacuation, CAS-70, ACS-255) ; `15_ABSENCES_PERSONNELS.md` (absence d'AESH) ; `19_AGENDA_RDV.md` (ESS, réunions restreintes) ; `20_CAHIER_DE_TEXTES.md` (faits d'aménagement, R-C36, AESH) ; `21_EVALUATIONS.md` (R-19) ; `IA_COEUR.md` ; `docs/PRA_SAUVEGARDES.md` §18.4 (E1–E14).
**Conventions.** **[F]** fait vérifié dans le dépôt (chemin et ligne) ; **[V]** règle lue sur une source officielle ; **[Ve]** extrait officiel vu (résultat de recherche, document académique reprenant le texte) sans lecture du texte entier ; **[NV]** non vérifié ; **[H]** hypothèse ; **[R]** recommandation. Numérotation propre : actions **INC-100** et suivantes ; défauts **DINC-01** ; règles **R-INC1** ; cas limites **CINC-01** ; questions **Q-INC1** ; réglages **S-2700 à S-2719** (numérotés dans la plage de migrations du socle, pour éviter les collisions entre sessions parallèles) ; lots **IN0** et suivants.
**Plage de migrations imposée :** **2700–2719**, vérifiée libre sur **les 134 branches distantes** le 2026-11-11 (aucun fichier `27[0-3]x_`, aucune mention dans un `MIGRATIONS.md`). Registre : `MIGRATIONS.md`.
**Méthode et limite.** Lecture des décisions, de la carte, du 34 et de sa passe personas, des conceptions qui attendent le 35 (02, 06, 10, 13, 14, 15, 19, 20, 21) ; lecture directe des migrations 016 et 212, d'`accompagnement.rs`, d'`actions.rs` et `roles.rs` (lignes citées). Recherche réglementaire **réduite** : les deux sous-agents de recherche ont été arrêtés par une limite d'usage ; la vérification a été refaite par recherches ciblées (résultats officiels vus en extrait, d'où beaucoup de **[Ve]**) et en reprenant les sources **[V]** du socle 34 (§5.7). Les points [Ve] et [NV] sont listés au §28 pour une relecture sur le texte entier.

Sommaire : §0 En bref · §1 Thèse · §2 Existant · §3 Acteurs · §4 L'année vue du référent · §5 Réglementaire · §6 Frontières · §7 Concepts · §8 Partage minimal · §9 Domaines · §10 Familles, mineur, majeur · §11 Cas limites · §12 IA · §13 Actions INC · §14 Règles · §15 Droits · §16 Conservation · §17 Événements · §18 Notifications · §19 Écrans et mobile · §20 Modèle de données · §21 Plan de migration · §22 Lots · §23 Migrations · §24 Tests · §25 Réglages · §26 Questions · §27 Décisions appliquées · §28 Faits, hypothèses, contradictions · §29 Priorités · §30 Ajouts personas.

---

## 0. En bref

1. **Un existant honnête mais à l'envers des besoins.** Le lot S2-11 a posé de bonnes bases — catalogue fermé de 14 aménagements sans diagnostic, portée cours / examen, comptes rendus d'équipe chiffrés, événements sans contenu — mais **le pilotage est confié à la direction et à l'administrateur seuls** (`accompagnement.gerer`, `actions.rs:1145-1147`), le **secrétariat, le CPE, l'infirmerie et le psychologue listent tous les élèves aménagés de l'établissement** (`amenagements.lire` à l'échelle de l'établissement, `actions.rs:1148-1157`, `accompagnement.rs:851-920`), **tout parent au lien actif** lit les aménagements sans contrôle d'autorité parentale ni de majorité (`accompagnement.rs:822`), et **l'AESH n'existe que comme un nom en texte** (`212:51`, `aesh_user_id` jamais écrit) : celle qui passe ses journées avec l'élève n'en voit rien. Un **lot IN0** corrige cela avant toute démonstration.
2. **La thèse (§1).** L'inclusion n'est pas un dossier médical : c'est **un engagement pédagogique daté**, pris par l'établissement envers un élève, et qui doit **arriver dans la classe** — au bon professeur, au bon cours, au bon devoir, au bon examen, à l'AESH du jour, au remplaçant du matin — **sans jamais dire pourquoi**. ScolIA porte donc trois choses : le **dispositif** (le cadre : PAP, PPS, PPRE, ULIS, UPE2A…, avec ses étapes, ses signatures, ses échéances), les **mesures** (ce que chaque adulte doit faire, codé, sans diagnostic) et l'**accompagnement humain** (AESH, tuteur, coordonnateur), et il garde la **cause** hors de portée (au socle 34 ou dans les pièces de la MDPH, au coffre).
3. **Ce que tranche le réglementaire [V/Ve]** (§5) : le **PAP** est mis en place par le chef d'établissement après **avis du médecin de l'Éducation nationale**, pour un trouble des apprentissages, révisé chaque année (D311-13) ; le **PPS** est **décidé hors de l'établissement** (MDPH, CDAPH), mis en œuvre par l'équipe et suivi au moins une fois par an par l'**équipe de suivi de la scolarisation** (ESS) réunie par l'**enseignant référent** (D351-5 à D351-10) ; le **PPRE** est pédagogique et ne relève pas de la santé (L311-3-1) ; l'**AESH** est rémunérée par l'État **y compris sur la pause méridienne** depuis la loi 2024-475 du 27/05/2024, public et privé sous contrat ; les **aménagements d'examens** passent par une **procédure simplifiée** quand l'élève a un PPS, un PAP ou un PAI et que les aménagements demandés sont ceux de sa scolarité (D351-27 et s., décret 2020-1523, circulaire du 8/12/2020), à demander **avant la date limite d'inscription** (dès la **4e pour le DNB**, en **2de pour le bac**) ; le **livret de parcours inclusif (LPI)**, application nationale (décret 2021-1246, arrêté du 30/09/2021), porte déjà PPRE, PAP, PAI, GEVA-Sco et volet PPS dans le public, **sans interconnexion prévue** avec les logiciels de vie scolaire ; les **pôles d'appui à la scolarité (PAS)** sont en expérimentation et leur **généralisation a été rejetée** par l'Assemblée nationale le 11/05/2026 : ScolIA ne les suppose pas, il nomme l'organisation de l'AESH de façon neutre (PIAL, PAS ou autre).
4. **Trois idées fortes [R]** : (a) **un aménagement est une instruction à l'adulte**, avec une **portée** (matière, type d'activité, examen) et une **date**, appliquée **par défaut** là où elle compte (devoir du cahier de textes, évaluation, convocation d'examen, sortie, évacuation), sans motif ; (b) **l'AESH est une personne de l'établissement** (socle 6) qui reçoit, pour les seuls élèves qu'elle accompagne et sur les seules plages prévues, l'emploi du temps, le cahier de textes, les aménagements et les consignes de sécurité — pas les notes, pas la santé, pas la discipline ; (c) **le dispositif suit l'élève dans le temps** : échéances visibles, révision préparée, transmission au collège ou au lycée d'accueil **avec accord**, et rien de conservé au-delà du nécessaire.
5. **Public et privé (D-019).** Dans le public, le LPI fait foi pour les documents officiels (PAP, document de mise en œuvre du PPS) : ScolIA ne double pas la saisie, il **reprend les mesures** et les fait vivre dans la classe ; dans le privé sous contrat, sans LPI [H, à confirmer], ScolIA porte aussi le **document**. Un réglage (S-2700) choisit « document dans ScolIA » ou « document dans le LPI (référence seulement) ».
6. **IA (§12, aligné sur R-S48 du 34)** : **aucune IA sur une donnée qui révèle un handicap ou un trouble** (PAP, PPS, ULIS, AESH, notification MDPH, aménagements liés à un PAP/PPS). L'IA peut seulement **adapter un support pédagogique sans aucune donnée d'élève** (« produis une version aérée de cette fiche ») et, pour le **PPRE** (pédagogique, non médical), **proposer** des objectifs que le professeur relit un à un.
7. **Le plan.** **151 actions** (119 de INC-100 à INC-283, numérotation à trous, puis 32 de INC-300 à INC-331 de la passe personas, §30), **48 cas limites**, **37 règles** (30 + 7 de la passe personas), **13 lots** : **IN0** (correctifs urgents), **IN1** (droits, cercles, journal), **IN2** (dispositifs et étapes), **IN3** (mesures et diffusion dans la classe), **IN4** (AESH), **IN5** (équipes, ESS, réunions), **IN6** (examens), **IN7** (PPRE et tutorat), **IN8** (ULIS, UPE2A, allophones, SEGPA), **IN9** (transitions et transmission), **IN10** (mobile), **IN11** (pilotage et conservation), **IN12** (RLS stricte, retraits). **Parcours minimal du pilote** : IN0, IN1, IN2, IN3, IN4 — l'appel, le cahier de textes et l'EDT en ont besoin dès la V1.
8. **Décisions appliquées par défaut (D-031)** : 22 (§27). **Soumis à Florian** : avis du DPO sur le classement « donnée de santé indirecte » et l'AIPD commune avec le 34 ; référents métier (un enseignant référent ou coordonnateur ULIS, une AESH) ; interface éventuelle avec le LPI (démarche officielle) ; choix de l'établissement pilote sur le document PAP.

---

## 1. Thèse : une promesse pédagogique, visible en classe, muette sur sa cause

1. **Le besoin de l'élève, pas son étiquette.** Ce que l'établissement doit savoir, c'est **ce qu'il faut faire** : « lui laisser un tiers de temps en plus », « lui fournir le cours imprimé en police agrandie », « ne pas l'interroger au tableau sans prévenir », « l'AESH l'accompagne en maths et en EPS ». Jamais « dyslexique », « TSA », « TDAH ». Le diagnostic reste au médecin, à la MDPH et à la famille (R-INC1).
2. **Une mesure qui n'arrive pas en classe n'existe pas.** La plainte n° 1 des familles et des AESH [H, terrain] : « le prof ne savait pas ». ScolIA **pousse** la mesure au bon endroit : liste d'appel (pastille sans motif), devoir du cahier de textes (délai majoré par défaut), sujet d'évaluation (version adaptée), remplaçant du jour, convocation d'examen blanc, sortie, plan d'évacuation (R-INC5).
3. **Le dispositif est un processus, pas une case.** Repérage → demande → avis → rédaction → signatures → mise en œuvre → suivi → révision → transmission ou fin. Chaque étape a un responsable, une échéance et une trace (R-INC3).
4. **Ceux qui accompagnent voient ce qu'il faut, pas plus.** L'AESH, le tuteur, le coordonnateur ULIS, le professeur principal, le professeur de la matière, l'enseignant référent, le CPE ont chacun **un périmètre** : un élève, un cours, une période (R-INC6).
5. **La famille est partenaire, l'élève est acteur.** Les titulaires de l'autorité parentale signent, lisent, demandent une révision ; l'élève (selon son âge) voit **ses** aménagements et peut dire ce qui marche ; l'élève **majeur** décide seul (R-INC8).
6. **Le public a son outil national.** Là où le LPI porte le document, ScolIA ne le recopie pas : il porte les **mesures** et la **logistique** (R-INC22).
7. **On n'oublie pas, on n'accumule pas non plus.** Les échéances sont tenues ; à la fin, ce qui doit suivre l'élève suit (avec accord), le reste est détruit (R-INC20).

---

## 2. Évaluation franche de l'existant

Code lu sur `origin/ent-mc0hoe` (`afeb8c4`), chemins depuis `ent-rust/`.

### 2.1 Ce qu'il y a [F]

| Élément | Où | Ce que ça fait |
|---|---|---|
| Dispositifs | `migrations/016_eleve_documents.sql:50-62`, `212_accompagnement.sql:8-21` | `ent_eleve_dispositif` : `kind` ∈ `pap, pps, pai, gevasco, ppre, ulis, autre`, **libellé libre** (120 car.), document lié, dates, statut `actif/clos`, échéance de révision (défaut : un an) |
| Aménagements | `212:23-41`, `accompagnement.rs:40-55` | `ent_amenagement` : **code d'un catalogue fermé de 14** (tiers-temps, police adaptée, sujets agrandis, premier rang, ordinateur, secrétaire, lecteur, salle séparée, pauses surveillées, consignes reformulées, calculatrice, polycopiés, oral, mise en page aérée), portée `cours / examen / les_deux`, dates, unicité d'un code actif par élève ; test qui refuse un nom de trouble dans le catalogue (`accompagnement.rs:1018-1030`) |
| AESH | `212:43-58`, `accompagnement.rs:562-611` | `ent_aesh_affectation` : **nom en texte** (`aesh_nom`), `aesh_user_id` **prévu mais jamais écrit**, heures hebdomadaires, dates |
| Réunions | `212:60-75`, `accompagnement.rs:613-716` | `ent_equipe_reunion` : `equipe_educative / equipe_suivi`, date, statut, **compte rendu chiffré** écrit une fois à la tenue (`statut = 'planifiee'` exigé) |
| Lectures | `accompagnement.rs:717-850` | dossier d'accompagnement (gestion ou `sante.lire`) ; aménagements d'un élève (professeur de l'élève, gestion, **parent lié**, élève) ; liste « mes aménagements » ; dispositifs à réviser |
| Droits | `actions.rs:216-221`, `:1143-1157` | `accompagnement.gerer` : **Admin, Direction** ; `amenagements.lire` : Admin, Direction, Administratif, CPE, Infirmerie, Psy à l'établissement, professeurs pour leurs élèves |
| Chiffrement | `champs_chiffres.rs:54` | `ent_equipe_reunion.compte_rendu` |
| RLS | `215_sante_rls.sql:9-10`, `580_rls_eleves_responsables.sql:44` | ENABLE + FORCE sur `ent_amenagement`, `ent_aesh_affectation`, `ent_equipe_reunion`, `ent_eleve_dispositif` |
| Registre E12 | `registre_donnees.rs:242, 255, 315, 352` | aménagements et AESH classés `sante` ; dispositifs `dossier_eleve` + santé ; **réunions classées `emploi_du_temps_structure` sans marque santé** |
| Mission AESH | `355_missions.sql:113` | type de mission « Accompagnement d'élèves (AESH) » (socle 6) |
| Chronologie | `dossier_personnes.rs:417, 455` | « Dispositif d'accompagnement », visible selon la section `dispositifs` |

### 2.2 Défauts (à corriger avant toute démonstration)

| # | Gravité | Défaut | Conséquence | Où [F] | Lot |
|---|---|---|---|---|---|
| **DINC-01** | **Élevée** | **Pilotage réservé à Admin et Direction** : ni le professeur principal, ni le coordonnateur ULIS, ni le référent de l'établissement, ni le CPE ne peuvent ouvrir un PPRE, préparer un PAP, planifier une ESS ; la direction et l'administrateur technique **lisent tous les comptes rendus** d'équipe | le travail réel se fait hors de l'outil ; l'administrateur technique lit des comptes rendus d'ESS qui citent souvent un diagnostic | `actions.rs:1145-1147`, `accompagnement.rs:267-275`, `:762-775` | IN0 |
| **DINC-02** | **Élevée** | **`amenagements.lire` à l'échelle de l'établissement** pour Administratif, CPE, Infirmerie, Psy (et Direction, Admin) ; `mes_aménagements` liste alors **tous** les élèves aménagés avec nom, classe et codes | le secrétariat imprime « tous les élèves à tiers-temps » (= une liste d'élèves à PAP/PPS) ; donnée de santé indirecte (C-184/20) sans besoin d'en connaître | `actions.rs:1148-1157`, `accompagnement.rs:851-920` | IN0 |
| **DINC-03** | **Élevée** | **Parent : tout lien actif suffit** (`lien_parent_eleve` lit `ent_eleve_lien_actif` sans autorité parentale, interdiction ni majorité) | un parent sans autorité, ou les parents d'un élève **majeur**, lisent ses aménagements | `accompagnement.rs:822`, `foyer_repo.rs:594-616` | IN0 |
| **DINC-04** | Élevée | **Libellé libre du dispositif** servi avec le code PAP / PPS dans `/eleves/{id}/dossier` (relevé SD-21 au 34) ; tout profil qui a la section `dispositifs` le lit (tout le personnel sauf professeurs, `service.rs:4755`) | « PPS — troubles autistiques » sous les yeux de l'accueil ou d'un AED | `service.rs:4550-4560`, `:4755`, `016:55` | IN0 |
| **DINC-05** | Moyenne | **`kind` mélange des natures** : `pai` (document médical du 34) et `gevasco` (document d'évaluation envoyé à la MDPH) sont rangés comme des « dispositifs » | double source du PAI (34 et 35) ; un GEVA-Sco apparaît comme un « dispositif actif » | `016:60-61`, `212:10-11`, `accompagnement.rs:67` | IN0 (gel), IN2 |
| **DINC-06** | Moyenne | **AESH sans compte** : l'affectation ne lie pas l'agent (socle 6) ; l'AESH ne voit ni l'EDT, ni le cahier de textes, ni les aménagements de l'élève ; nom en clair | l'AESH reçoit l'information par photocopie ; impossible de prévenir l'AESH d'un changement de salle ou d'une ESS | `212:47-48`, `accompagnement.rs:579-600` | IN4 |
| **DINC-07** | Moyenne | **Réunions sans participants ni convocation** ; aucune trace de la présence de la famille, de l'enseignant référent, de l'AESH ; table mal classée au registre (pas de marque santé) | une ESS sans la famille ou sans l'AESH ne se voit pas ; export de portabilité ou restauration traités comme une donnée ordinaire | `212:60-75`, `registre_donnees.rs:352` | IN0 (registre), IN5 |
| **DINC-08** | Moyenne | **Clés étrangères non composites** (`REFERENCES ent_eleve (id)`), `ON DELETE SET NULL` vers le dispositif | E2 non respecté ; un aménagement perd son cadre sans trace | `212:26-27, 46-47, 63-64` | IN0 |
| **DINC-09** | Moyenne | **Trace des lectures « fail-open » et incomplète** : écrite après la lecture, hors transaction ; `mes_amenagements` et `amenagements_eleve` non tracés | lecture d'une donnée de santé indirecte sans preuve | `accompagnement.rs:794-802`, `:812-920` | IN0 |
| **DINC-10** | Moyenne | **Révision sans historique** : la date est écrasée, la clôture retire tous les aménagements sans conserver la version signée ni le motif de fin | impossible de prouver ce qui était prévu quand un parent conteste (« le tiers-temps n'a pas été appliqué en mars ») | `accompagnement.rs:396-430` | IN2 |
| **DINC-11** | Faible | **Ouverture d'un PAP par l'infirmerie** (`sante.saisir`) alors que le PAP est mis en place par le chef d'établissement après avis du médecin ; aucune trace de l'avis | confusion des rôles ; PAP sans avis médical | `accompagnement.rs:308-313` | IN2 |
| **DINC-12** | Faible | **Catalogue trop court et à portée unique** : pas de matière, pas de type d'activité (écrit, oral, EPS, sortie), pas de valeur (tiers-temps de 33 %, « temps majoré » quantifié), rien pour l'évacuation, la sortie, le devoir à la maison | le professeur d'EPS reçoit « ordinateur » ; le cahier de textes ne sait pas quel délai appliquer | `accompagnement.rs:40-55` | IN3 |

### 2.3 Verdict
- **Garder** : le catalogue fermé sans diagnostic et son test ; la portée cours / examen ; le compte rendu chiffré écrit une fois ; les événements sans contenu ; la RLS posée.
- **Corriger tout de suite (IN0)** : DINC-01 à DINC-05, DINC-07 (registre), DINC-08, DINC-09.
- **Remplacer progressivement** : `ent_eleve_dispositif` par `ent_inc_dispositif` à **versions** et **étapes** ; `ent_amenagement` par `ent_inc_mesure` (catalogue étendu, portée par matière et activité, valeur) ; `ent_aesh_affectation` par `ent_inc_accompagnement` (agent du socle 6, notification, plages) ; `ent_equipe_reunion` par `ent_inc_reunion` (participants, convocation, relevé de décisions) ; le droit unique `accompagnement.gerer` par des droits fins (§15).
- **Reçu du socle 34** (MIG-9 du 34) : les tables 212 passent ici ; le **PAI reste au 34** (`34_SANTE.md` §28 point 2) ; le 34 fournit au 35 l'**avis médical** sans motif (port `sante::avis_medical`).

### 2.4 Next.js de production (lecture ciblée, sans modification)
L'inventaire T0 (`TRANSITION_T0_INVENTAIRE.md`) ne recense **aucun module d'inclusion** dans l'application Next.js [F, lecture de l'inventaire ; aucune lecture nouvelle du code] : pas de reprise d'actions, pas de données à migrer. Les séjours (socle 42) y portent des besoins d'AESH et de PAI de participants : leur reprise est traitée au 42 (`PERSONAS_42.md` P08) ; le 35 fournit le port `inclusion::besoins_sortie`.

---

## 3. Acteurs, besoins réels, irritants

| Acteur | Ce qu'il fait | Ce qui lui fait perdre du temps | Ce qu'il redoute | Ce qui le ravirait |
|---|---|---|---|---|
| **Chef d'établissement / adjoint** | met en place le PAP, signe, préside les équipes éducatives, organise les examens, répond aux familles | relances papier, PAP non revus à la rentrée, listes d'examens refaites chaque année | un recours parce qu'un aménagement d'examen n'a pas été demandé à temps ; une plainte « on ne m'a jamais convoqué » | tableau de rentrée « dispositifs à revoir », demandes d'examens prêtes à partir, relevé de conformité par élève |
| **Référent inclusion de l'établissement** (souvent CPE, adjoint ou professeur ; fonction du socle 6) [H] | tient la liste, prépare les ESS, fait le lien avec l'enseignant référent, les familles, les AESH | tableaux Excel parallèles ; « qui a quel aménagement cette année ? » | oublier un élève à la rentrée | un **tableau de bord du cycle** : étapes, échéances, pièces manquantes |
| **Enseignant référent** (extérieur, Éducation nationale, plusieurs établissements) | réunit l'ESS au moins une fois par an, rédige le GEVA-Sco de réexamen, assure le lien avec la MDPH | trouver une date avec tout le monde ; obtenir les bilans des professeurs | ESS sans les bons participants | **accès invité** à ses élèves seulement, convocation automatique, bilans des professeurs déjà collectés |
| **Professeur principal** | repère, propose un PPRE ou un PAP au conseil, fait le bilan avec la famille | PPRE rédigé dans Word puis perdu | « je ne savais pas qu'il avait un PAP » | proposition de PPRE depuis le conseil de classe ; vue des mesures de la classe |
| **Professeur de la matière** | applique les mesures, adapte les supports, évalue | retrouver l'information en début d'année ; refaire les adaptations | une famille qui dit « le tiers-temps n'a pas été donné » | la pastille dans l'appel, le délai appliqué par défaut au devoir, la variante de l'évaluation prête, un « fait / pas fait » en un geste |
| **Professeur d'EPS** | adapte les activités | confusion entre inaptitude médicale (34) et aménagement pédagogique (35) | un accident | mesures propres à l'EPS (« pas d'activité en hauteur » vient du 34, « consignes démontrées » du 35) |
| **Coordonnateur ULIS** | emploi du temps de chaque élève d'ULIS (temps en ULIS, inclusions), lien avec les professeurs d'accueil | inclusions à reconstruire à chaque changement d'EDT | un élève inclus dont le professeur n'est pas prévenu | l'EDT d'inclusion par élève, généré par l'EDT (10), avec la liste des professeurs à prévenir |
| **Enseignant d'UPE2A** | positionnement à l'arrivée, parcours d'inclusion progressive des élèves allophones | double saisie des tests de positionnement, absences de suivi d'une année sur l'autre | un élève « oublié » en UPE2A au-delà de la durée prévue | parcours daté, inclusion progressive par matière, sortie du dispositif préparée |
| **AESH** (individuelle, mutualisée, collective) | accompagne, prend des notes d'adaptation, participe à l'ESS | ne pas être prévenue d'un changement de salle, d'une sortie, d'une absence du professeur, d'une ESS ; tout savoir par l'élève | voir trop (les notes, la santé) ou pas assez ; être tenue responsable de ce qu'on ne lui a pas dit | **son planning du jour** (élèves, cours, salles, aménagements, consignes de sécurité), alerte des changements, convocation aux réunions de ses élèves |
| **CPE / vie scolaire** | suit l'assiduité, les incidents, la vie de l'élève | ne pas savoir qu'un élève a un besoin particulier lors d'un incident | sanctionner un comportement lié au trouble | la pastille « besoins particuliers » sans contenu sur la fiche de l'élève qu'il suit (13, R-DS7) |
| **Secrétariat** | inscrit, prépare les dossiers d'examen | recopie des aménagements dans les plateformes d'examens | une demande oubliée | liste de demandes d'examens par session, **sans motif**, sur son seul périmètre de saisie |
| **Infirmière / médecin (34)** | avis médical sur le PAP, demande d'aménagement d'examen hors procédure simplifiée | « avis demandé par mail » | voir son avis transformé en diagnostic dans un autre outil | une demande d'avis tracée, un avis **sans motif** retourné au 35 |
| **Psychologue (36)** | bilans, participation aux équipes | — | que son bilan circule | un lien « bilan transmis à la MDPH par la famille », jamais le bilan dans le 35 |
| **Familles** (dont séparées) | demandent, signent, se battent souvent pour obtenir l'application | répéter l'histoire à chaque rentrée ; ne pas savoir si c'est appliqué | que l'enfant soit étiqueté ; que l'autre parent soit tenu à l'écart ou au contraire informé contre l'accord du juge | voir ce qui est prévu, signer en ligne, être convoquée à temps, signaler qu'une mesure n'est pas appliquée |
| **Élève mineur** | vit avec les mesures | se faire remarquer (« pourquoi lui a plus de temps ? ») | être stigmatisé devant la classe | aménagement discret, appliqué sans qu'il doive le réclamer, pouvoir dire ce qui l'aide |
| **Élève majeur** | décide seul | que ses parents reçoivent encore tout | — | décider de partager ou non avec ses parents |
| **Remplaçant, suppléant** | assure une séance | ne rien savoir des élèves | mettre un élève en difficulté | les mesures de **la séance** dès sa désignation (R-S39 du 34, socle 15) |
| **Accueil, agents** | — | — | — | rien à voir, sauf aide à l'évacuation pour le responsable de zone (14) |
| **DPO** | registre, AIPD | données de santé indirectes dispersées | une liste d'élèves handicapés exportable | matrice de partage testée, journal, durées appliquées |

---

## 4. L'année vue du référent inclusion [H, à confirmer avec un référent réel]

| Période | Ce qui se passe | Ce que ScolIA fait |
|---|---|---|
| Mai–juin (N-1) | liaison école–collège, collège–lycée ; ESS de réexamen ; notifications MDPH qui arrivent | réception des **dossiers de liaison** (avec accord), préparation de la rentrée, besoins d'AESH remontés |
| Fin août | affectations d'AESH (PIAL / PAS / service départemental) | saisie ou import des affectations ; planning d'AESH proposé à partir de l'EDT (10) |
| 1re semaine | professeurs à informer, emplois du temps ULIS | **diffusion des mesures** aux équipes ; pastilles dans les appels ; tableau « mesures non lues » |
| Septembre–octobre | PAP à revoir, PPRE de rentrée, **demandes d'aménagements d'examens** (DNB en 4e/3e, bac en 2de/1re/terminale) | campagne d'examens : liste des candidats à procédure simplifiée, formulaires pré-remplis, échéances |
| Novembre–décembre | 1ers conseils : propositions de PPRE / PAP | proposition depuis le conseil (23) |
| Janvier–mars | ESS (calendrier de l'enseignant référent), GEVA-Sco de réexamen | convocations, collecte des bilans des professeurs, compte rendu |
| Avril–juin | examens blancs et réels, bilans de fin d'année, orientation (37) | convocations d'examens avec aménagements (38), bilans, transmission |
| Juillet | clôture | dispositifs clos ou reconduits, destruction programmée |

---

## 5. Ce qui est réglementaire (vérifié le 2026-11-11)

Recherche réduite (voir l'en-tête). Les sources [V] proviennent pour partie du socle 34 (§5.7, lues le 2026-11-10).

### 5.1 Plan d'accompagnement personnalisé (PAP)
- **C. éduc. D311-13** [Ve, `34_SANTE.md` §5.7 et `20_CAHIER_DE_TEXTES.md` R-11] : PAP pour l'élève dont les difficultés scolaires durables résultent d'un **trouble des apprentissages** ; **avis du médecin de l'Éducation nationale** ; mis en place par le chef d'établissement (ou le directeur d'école) ; **révisé chaque année** ; mesures pédagogiques ; proposé par le conseil de classe ou demandé par la famille.
- **Circulaire 2015-016 du 22/01/2015** (modèle national de PAP) [NV] : document type ; le PAP suit l'élève dans son changement d'établissement [NV].
- *Conséquence* : le PAP est un document **pédagogique** fondé sur un **avis médical** ; il n'énonce pas le trouble [H, le modèle national comporte une page d'avis du médecin : à vérifier] ; ScolIA stocke l'**avis** (favorable / défavorable, date, médecin) sans motif (port `sante::avis_medical` du 34).

### 5.2 Projet personnalisé de scolarisation (PPS), MDPH, ESS
- **C. éduc. L112-1, L112-2** [NV] ; **D351-5 à D351-7** [V via 34 §5.7] : PPS élaboré par l'**équipe pluridisciplinaire** de la MDPH, décidé par la **CDAPH** ; **D351-9** : PAI hors PPS ; **D351-10 à D351-12** [NV] : **équipe de suivi de la scolarisation** (ESS) réunie **au moins une fois par an** par l'**enseignant référent**, qui comprend les parents, l'élève et les professionnels concernés.
- **GEVA-Sco** (première demande, rédigé en équipe éducative ; réexamen, rédigé en ESS) [Ve, documents académiques et LPI].
- **Document de mise en œuvre du PPS** (arrêté du 6/02/2015) [NV] ; dans le public, versé et complété dans le **LPI** [Ve].
- *Conséquence* : le PPS **n'est pas décidé** dans l'établissement : ScolIA reçoit une **notification** (nature des décisions : AESH, matériel, orientation ULIS, aménagements, durée), met en œuvre, organise l'ESS ; la notification (qui peut mentionner un diagnostic) est une **pièce C3 au coffre** (Q-S12 du 34), lue d'un cercle restreint.

### 5.3 PPRE
- **C. éduc. L311-3-1, D311-12** [Ve] : programme personnalisé de réussite éducative, quand l'élève risque de ne pas maîtriser les connaissances et compétences attendues ; actions pédagogiques ciblées ; famille associée. *Conséquence* : **pas une donnée de santé** ; c'est un plan pédagogique ordinaire, à traiter comme tel (accès des professeurs de l'élève, IA permise en proposition).

### 5.4 ULIS, UPE2A, élèves allophones, SEGPA
- **ULIS** (circulaire 2015-129) [NV] : dispositif (pas une classe) ; l'élève est inscrit dans sa classe de référence et inclus ; orientation par la CDAPH ; coordonnateur.
- **UPE2A / EANA** (circulaire 2012-141) [NV] : positionnement à l'arrivée, inclusion progressive ; durée de soutien [NV].
- **SEGPA** (circulaire 2015-176) [NV] : orientation par la commission départementale (CDOEA).
- *Conséquence* : ScolIA modélise une **modalité de scolarisation** datée (socle 2, ELV-36) et un **parcours d'inclusion** par matière ; le statut d'allophone n'est pas une donnée de santé, mais peut révéler l'origine (donnée sensible au sens de l'art. 9 si elle révèle l'origine « raciale ou ethnique » [H]) : minimisation identique.

### 5.5 AESH et organisation
- **C. éduc. L917-1** [NV, texte non lu] : AESH recrutés par l'État ; **loi 2024-475 du 27/05/2024** [Ve] : l'État rémunère l'accompagnement **sur le temps scolaire et sur la pause méridienne**, écoles et établissements publics et **privés sous contrat** ; note de service du 24/07/2024 abrogée par un **décret du 14/02/2025** (numéro 2025-135 ou 2025-137 selon les sources : **à vérifier**) sans remettre en cause le principe [Ve, réponses ministérielles].
- **Organisation** : PIAL (loi 2019-791) [NV] ; **pôles d'appui à la scolarité (PAS)** expérimentés depuis la rentrée 2024 (circulaire du 4/07/2024) [Ve] ; **généralisation rejetée par l'Assemblée nationale le 11/05/2026** en nouvelle lecture d'une proposition de loi sur l'inclusion scolaire [Ve, presse spécialisée et questions parlementaires] — état du droit **mouvant** : ScolIA ne code aucun nom d'organisation en dur (référentiel `organisation_accompagnement` : PIAL, PAS, autre).
- **Discrétion** : l'AESH est tenue à la discrétion professionnelle [NV] ; *conséquence* : accès restreint à ses élèves, ses plages, sans santé ni discipline.

### 5.6 Aménagements d'examens
- **C. éduc. D351-27 et s.**, modifiés par le **décret 2020-1523 du 4/12/2020** ; **circulaire du 8/12/2020** (BO n° 47 du 10/12/2020) [Ve] : **procédure simplifiée** pour les candidats ayant un PPS, un PAP (trouble du neurodéveloppement) ou un PAI ayant fait l'objet d'un avis médical, quand les aménagements demandés sont **conformes à ceux de la scolarité** ; plan mis en place dès le cycle 4 (DNB) ou la 2de (autres examens) ; **procédure complète** sinon (avis d'un médecin désigné par la CDAPH) ; demande **avant la date limite d'inscription** ; possible **dès la 4e pour le DNB** et en 2de pour le bac ; aménagements accordés à un examen antérieur reconductibles dans les règles du nouvel examen [Ve].
- *Conséquence* : ScolIA prépare la **demande** (candidat, examen, aménagements de la scolarité **déjà en place**, procédure), l'établissement la dépose sur la plateforme académique ; ScolIA trace la **décision** reçue et la transmet au socle 38 (examens blancs et réels).

### 5.7 Livret de parcours inclusif (LPI)
- **Décret 2021-1246 du 29/09/2021** et **arrêté du 30/09/2021** [Ve] ; déployé nationalement le 31/01/2022 ; regroupe PPRE, PAP, PAI, document de mise en œuvre du PPS, GEVA-Sco ; décisions de la CDAPH versées automatiquement ; synthèse consultable par les parents ; **pas d'interconnexion prévue avec Pronote** (FAQ DGESCO de mai 2024) [Ve].
- *Conséquence* : dans le public, **pas de double saisie du document** ; ScolIA garde la **référence** (« PAP dans le LPI, version du … ») et les **mesures** utiles en classe ; une interface LPI serait une **démarche officielle** (Florian, Q-INC3).

### 5.8 Données, RGPD
- **CJUE C-184/20** [V via 34 §5.3] : PAP, PPS, ULIS, AESH, aménagements liés révèlent **indirectement** un état de santé ou un handicap : **données de santé** (art. 9). PPRE, UPE2A : non [H].
- **Délibération CNIL 2018-327** [V via 34] : AIPD pour l'accompagnement médico-social de personnes vulnérables : **AIPD** à mener avec celle du 34 [R].
- **Durées** : instruction de tri des archives des EPLE [NV] ; propositions §16, soumises au DPO.

### 5.9 Ce que les textes ne disent pas (à faire trancher)
1. Le PAP du **privé sous contrat** : médecin de l'Éducation nationale ou médecin de la famille ? [NV] → défaut ScolIA : « médecin désigné », champ libre de fonction (Q-INC5).
2. Le **désaccord entre titulaires** sur un PAP ou une ESS → même règle que le 34 (un titulaire suffit, l'autre informé, opposition enregistrée, pas de tranchage par ScolIA).
3. **Accès du professeur au document PAP entier** ou aux seules mesures ? → défaut : **mesures seulement** ; le document reste au référent, à la direction et à la famille (R-INC6).

---

## 6. Frontières

### 6.1 Qui possède quoi
| Donnée | 35 possède | Fournit / lit |
|---|---|---|
| Dispositifs PAP, PPS (mise en œuvre), PPRE, ULIS, UPE2A, SEGPA (modalité), tutorat, étapes, versions, signatures | **oui** | 2 (pastille), 23 (proposition depuis le conseil), 37 (orientation) |
| Mesures (aménagements) et leur portée | **oui** | 10, 12, 15, 20, 21, 38, 42, 14 (évacuation) lisent par port |
| Accompagnement humain (AESH, tuteur), plages, notification d'heures | **oui** | 6 (agent, mission), 10 (planning dérivé), 15 (absence d'AESH) |
| ESS, équipes éducatives, réunions de suivi (contenu) | **oui** | 19 (créneau, convocation restreinte) |
| PAI, inaptitudes EPS, consignes de santé, avis médical | 34 | 35 lit l'**avis** et l'**existence** |
| Notification MDPH, GEVA-Sco, bilans | pièces au coffre (7) sous domaine `inclusion` C3 | — |
| Bilans psychologiques | 36 | jamais dans le 35 |
| Aménagements d'épreuves accordés | 35 (demande et décision) | 38 (application) |
| Modalité de scolarisation (temps partagé, ULIS, UPE2A) | 2 (fait daté ELV-36) | 35 la crée par port |
| Inclusions ULIS dans les cours | 5 (STR-102) et 10 | 35 les demande |

### 6.2 Règles de frontière [R]
1. **Le 35 ne stocke jamais un diagnostic** ni un texte médical : codes, dates, avis sans motif, références de pièces.
2. **Le PAI reste au 34** ; le 35 ne garde que « PAI : oui » via `sante::pai_existe` pour le tableau de l'élève et la procédure simplifiée d'examen.
3. **Aucun socle n'écrit dans les tables `ent_inc_*`** ; ils appellent les ports (D-026).
4. **Le 19 ne voit que l'occupation** d'une réunion restreinte (titre « Réunion de suivi », sans élève pour les non-participants).

### 6.3 Ports exposés [R]
- `inclusion::mesures(eleve, contexte{seance|matiere|activite|examen|sortie|devoir}, date) -> [Mesure{code, libelle_instruction, valeur}]` — **sans source ni dispositif**.
- `inclusion::mesures_seance(seance) -> [(eleve, [Mesure])]` (appel, remplaçant, AESH).
- `inclusion::delai_devoir(eleve, matiere, date) -> Option<pourcentage>` (20, R-C36).
- `inclusion::variante_evaluation(eleve, evaluation) -> [Mesure]` (21).
- `inclusion::amenagements_epreuve(eleve, examen) -> Decision` (38).
- `inclusion::pastille(eleve, lecteur) -> {besoins_particuliers | aucune}` (2, 12, 13).
- `inclusion::accompagnements(eleve, date) -> [Accompagnement{agent, plages}]` et `inclusion::eleves_accompagnes(agent, date)` (6, 10, 15).
- `inclusion::besoins_sortie(sortie) -> [(eleve, [Mesure], accompagnant_requis)]` (42).
- `inclusion::aide_evacuation(eleve) -> {oui, aidant, suppleant}` (14).
- `inclusion::statistiques(etab, annee)` (47) — agrégats, petits effectifs masqués.

### 6.4 Ports consommés
`eleves`, `modalite_scolarisation` (2) ; `responsables`, `autorite`, `interdictions`, `majorite` (4) ; `classes`, `groupes`, `inclusions` (5) ; `agents`, `fonctions`, `missions` (6) ; `documents.deposer|signer|coffre` (7) ; `notifications` (8) ; `seances`, `seances_accompagnees` (10) ; `conseils` (23) ; `rendez_vous` (19) ; `sante::avis_medical`, `sante::pai_existe`, `sante::consignes` (34, lecture seule et seulement pour l'existence).

---

## 7. Concepts et vocabulaire [R]

| Terme | Sens | Remplace |
|---|---|---|
| **Dispositif** | cadre daté d'accompagnement d'un élève : `pap`, `pps`, `ppre`, `ulis`, `upe2a`, `segpa_egpa`, `tutorat`, `accompagnement_interne` (sans texte officiel), avec étapes, versions, signatures, échéance | `ent_eleve_dispositif` |
| **Étape** | moment du cycle : repérage, demande, avis, rédaction, signature, mise en œuvre, bilan, révision, clôture | — |
| **Mesure** | instruction à un adulte, code d'un catalogue fermé, avec **portée** (matières, activités, examen, sortie, devoir, évacuation), **valeur** éventuelle (« temps majoré : 33 % »), dates, dispositif d'origine caché | `ent_amenagement` |
| **Accompagnement** | une personne (AESH, tuteur, aide humaine collective d'ULIS) affectée à un élève, avec **plages** (matières, séances, pause méridienne), heures notifiées et heures réelles | `ent_aesh_affectation` |
| **Notification** | décision de la CDAPH reçue (référence, dates, catégories de décisions codées), pièce au coffre | — |
| **ESS / équipe éducative / réunion de suivi** | réunion autour d'un élève, avec participants, convocation, relevé de décisions, compte rendu chiffré | `ent_equipe_reunion` |
| **Pastille** | signal sans contenu « besoins particuliers » pour un adulte qui a l'élève | — |
| **Référent inclusion** | fonction du socle 6 qui pilote les dispositifs de l'établissement | `accompagnement.gerer` donné à la direction |
| **Cercle du dispositif** | personnes qui lisent le document : référent, direction, professeur principal, coordonnateur, enseignant référent invité, titulaires | — |

**Quatre distinctions** : (1) ce qu'il faut **faire** (mesure) ≠ ce que l'élève **a** (cause) ; (2) le **cadre** (dispositif) ≠ les **mesures** ; (3) ce qui est **décidé ailleurs** (CDAPH) ≠ ce qui est **décidé ici** (PAP, PPRE) ; (4) **pédagogique** (PPRE, UPE2A : non santé) ≠ **lié au handicap** (PAP, PPS, ULIS, AESH : santé indirecte).

---

## 8. Partage strictement minimal

### 8.1 Matrice [R]
| Information | Référent / direction | PP | Prof de l'élève | Coord. ULIS | AESH de l'élève | Ens. référent | CPE | Secrétariat | Remplaçant du jour | Infirmerie | Titulaires | Élève |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| Document PAP / PPS / version signée | ✔ | ✔ | — | ✔ (ULIS) | — | ✔ (PPS) | — | — | — | — | ✔ | ✔ (adapté à l'âge) |
| Nature du dispositif (« PAP ») | ✔ | ✔ | — (pastille seulement) | ✔ | ✔ | ✔ | pastille | — | — | ✔ (avis) | ✔ | ✔ |
| Mesures de **sa** matière / activité | ✔ | ✔ | ✔ | ✔ | ✔ | ✔ | — | — | ✔ (séance) | — | ✔ | ✔ |
| Mesures d'examen | ✔ | ✔ | ✔ (sa matière) | ✔ | ✔ | ✔ | — | **liste de session** (dossiers d'examen) | — | — | ✔ | ✔ |
| Accompagnement (qui, quand) | ✔ | ✔ | ✔ (ses séances) | ✔ | ✔ (le sien) | ✔ | ✔ | — | ✔ | — | ✔ | ✔ |
| Notification MDPH (pièce) | référent seulement | — | — | ✔ (ULIS) | — | ✔ | — | — | — | — | ✔ | majeur |
| Compte rendu d'ESS | participants | participants | — | participants | participant | ✔ | — | — | — | — | ✔ | ✔ (adapté) |
| Avis médical (sans motif) | ✔ | — | — | — | — | — | — | — | — | ✔ | ✔ | ✔ |
| PPRE (objectifs, bilan) | ✔ | ✔ | ✔ | — | — | — | ✔ | — | — | — | ✔ | ✔ |

### 8.2 Règles [R]
1. **Besoin d'en connaître, par séance et par matière** : un professeur voit les mesures de **ses** élèves **dans ses** matières et activités ; il ne voit pas la nature du dispositif (« PAP », « PPS »), seulement la pastille et les mesures (corrige DINC-02, DINC-04).
2. **Aucune liste établissement hors du référent et de la direction**, sauf la **liste de session d'examen** du secrétariat (candidats, épreuves, mesures, **sans** dispositif).
3. **Libellés en instructions** (« donner un tiers de temps supplémentaire », « fournir l'énoncé en police 14 ») : l'adulte sait quoi faire, l'élève n'est pas nommé par un trouble (R-INC2).
4. **Discrétion devant la classe** : aucune mesure affichée sur un écran projeté, une liste murale, un plan de classe imprimé (R-INC13).

---

## 9. Les domaines un par un [R]

### 9.1 Repérage et demande
Le professeur principal, le conseil de classe (23), un professeur, le CPE, l'infirmière, la famille ou l'élève majeur **signalent un besoin** (« difficultés durables en lecture », sans diagnostic) ; le référent ouvre un **dossier en préparation** avec la piste (PPRE, PAP, demande MDPH, aucun) ; la famille est **informée et associée** à chaque étape ; une demande de PAP déclenche la **demande d'avis** au médecin (34) ; une demande MDPH : ScolIA prépare l'**équipe éducative** et le GEVA-Sco de première demande (document hors ScolIA dans le public, LPI).

### 9.2 PAP
Avis du médecin reçu (favorable, date) → rédaction des **mesures** (catalogue, par matière) → signatures (chef, chaque titulaire pour soi, élève selon l'âge, médecin de l'avis) → **version figée** (empreinte) → diffusion aux professeurs → **bilan annuel** et révision (S-2701 : rappel à J-30 de l'échéance) → reconduction (nouvelle version) ou fin.

### 9.3 PPS
Réception de la **notification** (référence, dates, décisions codées : AESH individuelle / mutualisée, heures, matériel pédagogique adapté, orientation ULIS, SEGPA, établissement médico-social, transport, aménagements) → **document de mise en œuvre** (dans ScolIA ou référence LPI, S-2700) → mesures → ESS annuelle avec l'enseignant référent (invité) → GEVA-Sco de réexamen (pièce) → nouvelle notification. Alerte quand la notification **expire** dans moins de six mois sans ESS planifiée.

### 9.4 PPRE
Ouvert par le professeur principal ou une équipe : objectifs (compétences du socle 22), actions, durée (une période), bilan ; famille informée et associée ; peut être proposé depuis le conseil de classe ; **IA permise** en proposition d'objectifs (§12).

### 9.5 Mesures dans la classe
- **Appel (12)** : pastille sans motif ; « mesures de la séance » en un geste.
- **Cahier de textes (20)** : délai majoré par défaut, variante attribuée, levée possible sans motif, tracée (R-C36).
- **Évaluations (21)** : variante, temps majoré, marque « aménagé » sur le résultat ; évaluation neutralisée si dispense (R-19 du 21).
- **Remplacement (15)** : le remplaçant reçoit les mesures de la séance dès sa désignation.
- **Sorties (42)** : besoins (accompagnant requis, accessibilité) — jamais le dispositif.
- **Évacuation (14)** : « aide à l'évacuation » et aidant désigné.
- **Suivi d'application** : un professeur peut marquer « appliqué / impossible aujourd'hui » ; la famille peut **signaler une mesure non appliquée** ; le référent voit les signalements (pas de surveillance des professeurs : agrégat par mesure, pas de classement nominatif hors du référent, S-2706).

### 9.6 AESH
Affectation (agent du socle 6, mission « Accompagnement AESH »), type (individuelle, mutualisée, collective ULIS), **heures notifiées** et **plages** (matières, séances, pause méridienne — loi 2024-475), planning dérivé (10 §8.3 bis), contrôle des conflits (H-16) ; **absence de l'AESH** (15) : prévenir professeurs, vie scolaire, famille si le PPS le prévoit ; **notes d'accompagnement** (ce qui marche, sans diagnostic, visibles du référent et de l'enseignant référent) ; participation aux ESS ; **heures réelles** vs notifiées (indicateur pour la direction, sans évaluation de la personne).

### 9.7 Équipes, ESS, réunions
Convocation (19, nature restreinte), participants (famille, élève, enseignant référent, professeurs, AESH, CPE, infirmière, psychologue selon le cas), **présences**, **relevé de décisions codé** (mesures ajoutées / retirées, demandes MDPH, orientation), compte rendu chiffré (texte libre limité, **avertissement « pas de diagnostic »** et lint doux), diffusion aux participants ; la famille reçoit le relevé.

### 9.8 Examens
Campagne par session (DNB, bac, CAP, BTS ; dates de l'académie saisies) ; liste des **candidats éligibles à la procédure simplifiée** (dispositif en place dans le cycle) ; **demande** pré-remplie avec les mesures de la scolarité ; suivi : déposée, décision reçue, mesures accordées ; transmission au 38 ; alerte « demande non faite à J-15 de la date limite ».

### 9.9 ULIS, UPE2A, SEGPA, allophones
- **ULIS** : emploi du temps d'inclusion par élève (temps en ULIS / en classe), professeurs d'accueil prévenus, révision par période.
- **UPE2A** : positionnement (langue de scolarisation antérieure, niveau en français par compétences, sans nationalité), inclusion progressive par matière, durée, sortie du dispositif, liaison avec le CASNAV [NV] ; documents traduits pour la famille (langue préférée du socle 4).
- **SEGPA / EGPA** : modalité de scolarisation et orientation (37), sans contenu de commission.

### 9.10 Tutorat
Tuteur (professeur, AED, élève pair), objectifs, rencontres, bilan ; pas une donnée de santé ; utile aussi aux élèves à haut potentiel [NV, aucun dispositif formel trouvé].

### 9.11 Transitions et transmission
Changement de classe, d'établissement, de cycle : **dossier de liaison** proposé (dispositif, dernière version, mesures, accompagnement) transmis **avec accord** des titulaires ou de l'élève majeur (R-INC17) ; arrivée d'un élève avec PAP : reprise de la version reçue, nouvelle signature ; départ : rien n'est transmis automatiquement (socle 3 R-I20 : jamais santé ni suivi social).

### 9.12 Pilotage
Nombre de dispositifs par type, échéances, ESS tenues, demandes d'examens, heures d'AESH notifiées / réelles, signalements de non-application, **petits effectifs masqués** hors du référent ; export pour l'enquête ministérielle [NV] sans nom.

---

## 10. Familles, mineur, majeur [R]
1. **Titulaires de l'autorité parentale** : lisent le document, les mesures, les réunions ; signent **chacun pour soi** ; demandent une révision ; signalent une non-application ; reçoivent les convocations séparément (aucune adresse de l'autre).
2. **Parent sans autorité ou visé par une interdiction** : **rien** (corrige DINC-03).
3. **Désaccord** : un titulaire suffit à mettre en place un PAP ou un PPRE ; l'autre est informé et peut s'opposer par écrit ; ScolIA enregistre et transmet à la direction, ne tranche pas (aligné Q-S11 du 34).
4. **Élève mineur** : voit **ses** mesures dans un langage pour lui ; peut dire « cette mesure m'aide / me gêne » ; participe à l'ESS ; ne voit pas les notes d'AESH.
5. **Élève majeur** : à 18 ans, accès des titulaires fermé ; il signe seul ; il peut rouvrir un accès (portée : convocations, document, tout), révocable (aligné R-S13).
6. **Décès, départ** : gel, aucune relance, conservation §16.

---

## 11. Cas limites (CINC-01 à CINC-36 ; ajouts personas au §30)

| # | Cas | Réponse [R] |
|---|---|---|
| CINC-01 | PAP arrivé d'un autre collège en septembre | version reçue enregistrée comme « reprise », mesures actives après relecture du référent, nouvelle signature à la première révision |
| CINC-02 | Notification MDPH expirée, nouvelle pas encore reçue | les mesures restent appliquées jusqu'à la décision (droit acquis provisoire [H]) ; alerte référent ; AESH : selon la notification, jamais coupée par ScolIA |
| CINC-03 | AESH mutualisée pour trois élèves de deux classes à la même heure | conflit signalé (H-16) ; plages arbitrées par le référent ; jamais deux lieux à la fois |
| CINC-04 | AESH absente le matin | 15 → professeurs des séances, vie scolaire ; famille prévenue si le document le prévoit ; aucune information sur le motif de l'absence |
| CINC-05 | AESH qui accompagne aussi dans un autre établissement | une identité, une mission par établissement ; elle ne voit que les élèves de chaque établissement dans la casquette active |
| CINC-06 | Tiers-temps à l'écrit mais pas à l'oral | portée par activité (`ecrit`, `oral`, `tp`, `eps`) |
| CINC-07 | Le professeur juge la mesure impossible pour une évaluation (épreuve chronométrée) | « impossible aujourd'hui » + motif codé ; référent prévenu ; la famille voit le signalement, pas le nom du professeur hors réunion |
| CINC-08 | Élève qui refuse sa mesure devant ses camarades | l'élève peut demander de ne pas l'appliquer en classe ; tracé ; la mesure d'examen reste demandée |
| CINC-09 | Famille qui refuse le PAP proposé par le conseil | refus enregistré ; aucune mesure ; reproposition possible |
| CINC-10 | Parents séparés, l'un refuse l'ESS | convocation aux deux ; tenue possible ; refus consigné |
| CINC-11 | Parent visé par une interdiction de contact qui demande la convocation | refus, aucune information ; journal |
| CINC-12 | Élève de 17 ans et 11 mois avant l'ESS | convocations aux titulaires ; l'ESS après 18 ans : élève convoqué, titulaires seulement s'il l'accepte |
| CINC-13 | Élève en ULIS inclus en 5B pour deux cours, changement d'EDT | inclusions recalculées par le 10 ; professeurs d'accueil prévenus ; AESH prévenue |
| CINC-14 | Remplaçant qui n'a pas encore de compte | mesures de la séance imprimables sans nom de dispositif, remises par la vie scolaire, filigrane |
| CINC-15 | Examen blanc : salle séparée indisponible | 38 propose une salle ; sinon alerte au référent avant l'épreuve |
| CINC-16 | Demande d'aménagement d'examen hors délai | statut « tardive », procédure académique [NV] ; ScolIA ne la bloque pas |
| CINC-17 | Mesure « ordinateur » sans ordinateur disponible | lien avec le socle 44 (prêt) ; alerte |
| CINC-18 | Élève allophone arrivé en mars sans dossier | positionnement, inclusion progressive ; aucune nationalité stockée |
| CINC-19 | Élève allophone dont les parents ne lisent pas le français | convocations traduites (modèles validés), interprète noté comme participant |
| CINC-20 | PPS avec orientation vers un établissement médico-social, scolarisation partagée | modalité de scolarisation partagée (2) ; présences attendues seulement les jours prévus (12) |
| CINC-21 | Élève dispensé d'un enseignement par décision du recteur | 21 neutralise ; 10 retire la séance de son EDT ; pas de motif |
| CINC-22 | Élève en PPRE qui change de classe | PPRE suit, nouveau professeur principal prévenu |
| CINC-23 | Professeur qui demande « pourquoi » | l'écran répond « le cadre et la cause ne sont pas partagés ; adressez-vous au référent » |
| CINC-24 | Le chef veut la liste des élèves à AESH pour un rapport | agrégats seulement ; liste nominative au référent, tracée |
| CINC-25 | Contrôle de l'inspection qui demande les PAP | consultation par le référent en présence ; export nominatif seulement sur demande écrite, tracé (S-2711) |
| CINC-26 | Élève qui arrive avec un PAP du privé sans avis médical | mesures appliquées à titre provisoire (accompagnement interne) ; avis demandé |
| CINC-27 | Fusion d'élèves (doublon) | dispositifs jamais fusionnés automatiquement ; revue par le référent |
| CINC-28 | Erreur de saisie d'une mesure sur le mauvais élève | correction = nouvelle version avec motif ; lectures de l'erreur visibles du référent (journal) |
| CINC-29 | Restauration de sauvegarde après une clôture | journal des destructions rejoué (aligné R-S44) |
| CINC-30 | Élève en fauteuil, sortie au musée | 42 lit « accompagnant requis », « accessibilité » ; jamais « PPS » |
| CINC-31 | Professeur principal qui change en cours d'année | droits transférés à la date (mission du 6) |
| CINC-32 | Enseignant référent qui part | invitations révoquées ; successeur invité |
| CINC-33 | Élève majeur qui refuse que ses parents soient convoqués | respecté ; convocations à lui seul |
| CINC-34 | Pause méridienne : AESH sur le temps de repas | plage « pause méridienne » ; 40 lit qu'un accompagnant est présent, jamais pourquoi |
| CINC-35 | Élève à haut potentiel, accélération | dispositif `accompagnement_interne` + tutorat ; pas un handicap ; pas de pastille santé |
| CINC-36 | IA sollicitée par un professeur sur « comment aider Léo » | refus (« je n'ai pas accès aux besoins particuliers des élèves ») ; propose des ressources génériques |

---

## 12. Intelligence artificielle [R] (aligné `IA_COEUR.md` et R-S48 du 34)

1. **Aucune capacité IA ne lit** un dispositif PAP, PPS, ULIS, une notification, un accompagnement AESH, un compte rendu de réunion, ni une mesure **liée** à l'un d'eux (donnée de santé indirecte) ; les contextes IA de tous les socles filtrent les ports du 35 de nature `sante_indirecte` (R-INC19). Réponse type : « je n'ai pas accès aux besoins particuliers des élèves ; voici l'écran qui répond ».
2. **Permis (C0, sans donnée d'élève)** : `inclusion.adapter_support` — à partir d'un document fourni par le professeur, proposer une version aérée, agrandie, aux consignes reformulées ; aucune identité, aucun lien à un élève ; le professeur relit.
3. **Permis (PPRE, non santé)** : `inclusion.proposer_objectifs_ppre` — à partir des seules compétences et résultats que **le demandeur** voit déjà (21, 22), proposer des objectifs et actions ; chaque proposition relue et acceptée une à une ; jamais d'envoi automatique à la famille.
4. **Interdit** : déduire un trouble, suggérer un dispositif à partir de notes ou d'absences, classer des élèves « à risque ». Les **alertes déterministes** (échéance, demande d'examen manquante, mesure non lue) ne sont pas de l'IA.

---

## 13. Actions métier (INC-100 et suivantes)

Abréviations : REF référent inclusion ; DIR direction ; PP professeur principal ; PRF professeur ; PEPS professeur d'EPS ; CUL coordonnateur ULIS ; UPE enseignant d'UPE2A ; AESH ; ER enseignant référent (invité) ; CPE ; ADF secrétariat ; INF/MED (34) ; PAR titulaire ; ELV élève ; ELM élève majeur ; SYS système. Droits au §15.

### 13.1 Paramétrage, cercles, activation
| ID | Action | Qui · droit | Règles | Événement |
|---|---|---|---|---|
| INC-100 | Désigner le ou les référents inclusion (fonction du socle 6) | DIR · `inclusion.parametrer` | ne s'inscrit pas lui-même par défaut | `inclusion.referent_designe` |
| INC-101 | Choisir le mode du document PAP / PPS : dans ScolIA ou référence LPI (S-2700) | DIR · `inclusion.parametrer` | public : LPI par défaut | — |
| INC-102 | Régler les rappels d'échéance (S-2701) | REF · `inclusion.parametrer` | — | — |
| INC-103 | Inviter un enseignant référent (accès invité à ses élèves, durée) | REF · `inclusion.invites.gerer` | identité fédérée ou compte invité ; journal | `inclusion.invite_ajoute` |
| INC-104 | Révoquer un invité | REF, DIR · idem | immédiat | `inclusion.invite_revoque` |
| INC-105 | Désigner le coordonnateur ULIS, l'enseignant d'UPE2A | DIR · `inclusion.parametrer` | mission du 6 | — |
| INC-106 | Saisir l'organisation des AESH (PIAL, PAS, autre ; coordonnateur, contacts) | REF · `inclusion.parametrer` | aucun nom d'organisation en dur | — |
| INC-107 | Voir l'écran « ce que vous voyez, et pourquoi pas plus » | tous | — | — |
| INC-108 | Saisir le calendrier des sessions d'examens et dates limites | ADF, REF · `inclusion.examens.gerer` | — | — |

### 13.2 Repérage et demande
| ID | Action | Qui · droit | Règles | Événement |
|---|---|---|---|---|
| INC-110 | Signaler un besoin (sans diagnostic) | PRF, PP, CPE, INF, AESH · `inclusion.signaler` | texte court, lint « pas de diagnostic » ; visible du référent seul | `inclusion.besoin_signale` |
| INC-111 | Proposer un PPRE ou un PAP depuis le conseil de classe | PP, DIR · `inclusion.proposer` | port du 23 | `inclusion.proposition` |
| INC-112 | Demander un PAP (famille) ou une évaluation des besoins | PAR, ELM · lien | accusé ; référent prévenu | `inclusion.demande_famille` |
| INC-113 | Ouvrir un dossier en préparation | REF · `inclusion.dispositifs.gerer` | piste : PPRE, PAP, MDPH, aucune | `inclusion.dossier_ouvert` |
| INC-114 | Informer la famille et l'associer | REF · idem | chaque titulaire pour soi | `inclusion.famille_informee` |
| INC-115 | Demander l'avis du médecin (PAP, examen procédure complète) | REF · `inclusion.avis.demander` | port du 34 ; aucun motif transmis par écrit hors rendez-vous | `sante.avis_demande` |
| INC-116 | Recevoir l'avis (favorable / défavorable, date) | MED · `sante.avis.emettre` (34) | aucun motif dans le 35 | `inclusion.avis_recu` |
| INC-117 | Préparer l'équipe éducative de première demande MDPH | REF · `inclusion.reunions.gerer` | GEVA-Sco hors ScolIA (public) ou pièce | `inclusion.reunion_planifiee` |
| INC-118 | Clore un dossier en préparation sans suite | REF · `inclusion.dispositifs.gerer` | motif codé ; destruction programmée | `inclusion.dossier_clos` |
| INC-119 | Voir la file des besoins signalés et des demandes | REF · `inclusion.dispositifs.lire` | — | — |

### 13.3 PAP
| ID | Action | Qui · droit | Règles | Événement |
|---|---|---|---|---|
| INC-120 | Rédiger le PAP (mesures par matière et activité) | REF, PP · `inclusion.dispositifs.gerer` | avis favorable requis (R-INC4) | `inclusion.version_projet` |
| INC-121 | Recevoir les contributions des professeurs (mesures proposées pour leur matière) | PRF · `inclusion.contribuer` | sur demande du référent ; délai | `inclusion.contribution` |
| INC-122 | Soumettre aux signatures | REF · idem | S2 du socle 7 ; chaque titulaire pour soi | `inclusion.a_signer` |
| INC-123 | Signer (chef) | DIR · `inclusion.signer_chef` | voit mesures et cadre, pas de motif | `inclusion.signe` |
| INC-124 | Signer (titulaire, élève selon l'âge, majeur) | PAR, ELV, ELM · lien | version figée (empreinte) | `inclusion.signe` |
| INC-125 | Mettre en vigueur (diffusion des mesures) | SYS | dès la signature du chef et d'un titulaire | `inclusion.en_vigueur` |
| INC-126 | Réviser (nouvelle version) | REF · idem | ancienne version gardée | `inclusion.version_creee` |
| INC-127 | Reconduire à l'identique à la rentrée | REF · idem | relecture obligatoire ; signatures | `inclusion.reconduit` |
| INC-128 | Clore (motif codé : besoin couvert, départ, passage en PPS, refus) | REF · idem | mesures retirées à la date | `inclusion.clos` |
| INC-129 | Imprimer le PAP pour la famille (filigrane) | REF, PAR · idem / lien | — | — |
| INC-130 | Enregistrer un PAP tenu dans le LPI (référence, date, mesures utiles) | REF · idem | public ; aucun contenu recopié au-delà des mesures | `inclusion.reference_lpi` |

### 13.4 PPS
| ID | Action | Qui · droit | Règles | Événement |
|---|---|---|---|---|
| INC-135 | Enregistrer une notification de la CDAPH (référence, dates, décisions codées) | REF · `inclusion.notifications.gerer` | pièce au coffre C3 ; aucun diagnostic saisi | `inclusion.notification_recue` |
| INC-136 | Ouvrir le document de mise en œuvre (ou référence LPI) | REF, CUL · `inclusion.dispositifs.gerer` | S-2700 | `inclusion.version_projet` |
| INC-137 | Dériver les mesures et les besoins d'accompagnement de la notification | REF · idem | relecture ; aucun automatisme silencieux | `inclusion.mesures_derivees` |
| INC-138 | Planifier l'ESS annuelle avec l'enseignant référent | REF, ER · `inclusion.reunions.gerer` | 19 ; participants requis | `inclusion.reunion_planifiee` |
| INC-139 | Suivre l'échéance de la notification (alerte à 6 mois) | SYS | S-2701 | `inclusion.notification_expire` |
| INC-140 | Déposer le GEVA-Sco de réexamen (pièce) | ER, REF · idem | coffre C3 | `inclusion.piece_deposee` |
| INC-141 | Enregistrer un matériel pédagogique adapté notifié (lien au prêt, 44) | REF · idem | — | `inclusion.materiel` |
| INC-142 | Enregistrer un transport adapté notifié (lien au 43) | REF · idem | aucune donnée au transporteur au-delà de l'horaire | — |
| INC-143 | Enregistrer une scolarisation partagée (unité d'enseignement, IME) | REF · idem | modalité du 2 ; présences attendues (12) | `eleve.modalite` |
| INC-144 | Lire la notification (pièce) | REF, CUL, ER · `inclusion.notifications.lire` | journal ; jamais le professeur | — |

### 13.5 PPRE et tutorat
| ID | Action | Qui · droit | Règles | Événement |
|---|---|---|---|---|
| INC-150 | Ouvrir un PPRE (objectifs, actions, période) | PP, PRF · `inclusion.ppre.gerer@mes_classes` | famille informée | `inclusion.ppre_ouvert` |
| INC-151 | Faire proposer des objectifs par l'IA (§12.3) | PP · idem | relecture une à une | — |
| INC-152 | Associer les actions au cahier de textes (soutien, AP) | PRF · idem | 20 | — |
| INC-153 | Faire le bilan, prolonger, clore | PP · idem | bilan visible de la famille | `inclusion.ppre_bilan` |
| INC-154 | Désigner un tuteur (adulte ou pair) | PP, CPE · `inclusion.tutorat.gerer` | pair : accord des familles des deux élèves | `inclusion.tuteur_designe` |
| INC-155 | Noter une rencontre de tutorat (date, objectif, sans contenu sensible) | tuteur · idem | — | — |
| INC-156 | Clore un tutorat | PP · idem | — | — |

### 13.6 Mesures et diffusion
| ID | Action | Qui · droit | Règles | Événement |
|---|---|---|---|---|
| INC-160 | Définir une mesure (code, portée, valeur, dates) | REF · `inclusion.mesures.gerer` | catalogue fermé ; instruction | `inclusion.mesure_definie` |
| INC-161 | Retirer une mesure (motif codé) | REF · idem | historique | `inclusion.mesure_retiree` |
| INC-162 | Voir les mesures de la séance | PRF, AESH, remplaçant · `inclusion.mesures.lire@seance` | port `mesures_seance` | — |
| INC-163 | Voir les mesures de mes élèves (ma matière) | PRF · `inclusion.mesures.lire@mes_classes` | sans dispositif | — |
| INC-164 | Accuser lecture des mesures de rentrée | PRF · idem | relance après 7 jours (S-2702) | `inclusion.mesures_lues` |
| INC-165 | Marquer « appliquée / impossible aujourd'hui » (motif codé) | PRF · idem | facultatif ; agrégé | `inclusion.application_notee` |
| INC-166 | Lever une mesure pour un travail donné | PRF · idem | sans motif, tracé (R-C36) | — |
| INC-167 | Voir le tableau « mesures non lues » | REF · `inclusion.dispositifs.lire` | relancer | — |
| INC-168 | Signaler une mesure non appliquée | PAR, ELV, ELM, AESH · lien / `inclusion.signaler` | référent ; aucun nom de professeur affiché à la famille | `inclusion.non_application` |
| INC-169 | Traiter un signalement de non-application | REF · idem | réponse à la famille | `inclusion.non_application_traitee` |
| INC-170 | Pastille « besoins particuliers » sur la fiche et l'appel | SYS | sans contenu | — |
| INC-171 | Imprimer les mesures d'une séance pour un remplaçant sans compte | CPE · `inclusion.mesures.lire@seance` | filigrane, sans dispositif | — |
| INC-172 | Proposer une variante de document (IA C0) | PRF · `ia.utiliser` | aucune donnée d'élève | — |

### 13.7 AESH
| ID | Action | Qui · droit | Règles | Événement |
|---|---|---|---|---|
| INC-180 | Affecter un accompagnant (agent du 6, type, heures notifiées) | REF · `inclusion.accompagnement.gerer` | lien à la notification | `inclusion.accompagnement_affecte` |
| INC-181 | Définir les plages (matières, séances, pause méridienne) | REF, CUL · idem | conflits signalés (H-16) | `inclusion.plages_definies` |
| INC-182 | Importer les affectations du service d'organisation (fichier) | REF · idem | correspondance d'identités, rapport | — |
| INC-183 | Voir mon planning du jour | AESH · `inclusion.accompagnement.lire@moi` | élèves, séances, salles, mesures, consignes de sécurité (34 audience « AESH ») | — |
| INC-184 | Être prévenue d'un changement (salle, séance annulée, sortie, évaluation) | SYS | notifications 8 | — |
| INC-185 | Voir le cahier de textes et les documents des cours accompagnés | AESH · `cahier.lire@accompagnes` | 20 | — |
| INC-186 | Noter une observation d'accompagnement (ce qui aide) | AESH · `inclusion.notes_accompagnement.saisir` | lint ; référent et ER lecteurs | `inclusion.note_accompagnement` |
| INC-187 | Déclarer son absence ou être déclarée absente | AESH, ADF · socle 15 | prévenir 35 | `inclusion.accompagnement_non_assure` |
| INC-188 | Organiser un remplacement ponctuel d'AESH | REF · `inclusion.accompagnement.gerer` | temporaire, daté | `inclusion.accompagnement_temporaire` |
| INC-189 | Suivre heures notifiées / planifiées / réalisées | REF, DIR · `inclusion.pilotage.lire` | agrégé ; pas d'évaluation nominative de l'AESH | — |
| INC-190 | Mettre fin à un accompagnement (date, motif codé) | REF · idem | accès fermés le jour même | `inclusion.accompagnement_fin` |
| INC-191 | Voir les réunions de mes élèves accompagnés | AESH · idem | convocation | — |

### 13.8 Réunions (équipe éducative, ESS, suivi)
| ID | Action | Qui · droit | Règles | Événement |
|---|---|---|---|---|
| INC-200 | Planifier (nature, date, lieu ou visio 33) | REF, ER, DIR · `inclusion.reunions.gerer` | 19 : nature restreinte | `inclusion.reunion_planifiee` |
| INC-201 | Convoquer les participants (chacun pour soi) | SYS | délai (S-2703) ; traduction | `inclusion.convocation` |
| INC-202 | Collecter les bilans des professeurs avant l'ESS | REF · idem | formulaire court par matière | `inclusion.bilan_demande` |
| INC-203 | Saisir son bilan de matière | PRF · `inclusion.contribuer` | sans diagnostic | — |
| INC-204 | Noter les présences | REF · idem | — | — |
| INC-205 | Saisir le relevé de décisions codé | REF, ER · idem | mesures + / −, demandes MDPH | `inclusion.reunion_tenue` |
| INC-206 | Saisir le compte rendu (chiffré, une fois, compléments ajoutés) | REF, ER · idem | lint « pas de diagnostic » | — |
| INC-207 | Diffuser le relevé aux participants et à la famille | REF · idem | — | `inclusion.releve_diffuse` |
| INC-208 | Reporter, annuler (motif) | REF · idem | — | — |
| INC-209 | Voir l'historique des réunions d'un élève | cercle · `inclusion.reunions.lire` | journal | — |

### 13.9 Examens
| ID | Action | Qui · droit | Règles | Événement |
|---|---|---|---|---|
| INC-215 | Ouvrir une campagne (session, date limite) | REF, ADF · `inclusion.examens.gerer` | — | `inclusion.campagne_examens` |
| INC-216 | Lister les candidats éligibles à la procédure simplifiée | SYS | dispositif en place dans le cycle | — |
| INC-217 | Préparer la demande (pré-remplie des mesures de scolarité) | REF · idem | partie établissement | `inclusion.demande_examen_preparee` |
| INC-218 | Faire valider la demande par la famille / l'élève majeur | PAR, ELM · lien | signature | `inclusion.demande_examen_signee` |
| INC-219 | Marquer « déposée » sur la plateforme académique (date) | ADF · idem | pas d'interface au premier temps | `inclusion.demande_examen_deposee` |
| INC-220 | Enregistrer la décision (aménagements accordés) | ADF, REF · idem | pièce au coffre | `inclusion.decision_examen` |
| INC-221 | Transmettre au 38 (épreuves, salles, surveillances) | SYS | port `amenagements_epreuve` | — |
| INC-222 | Alerter « demande non faite » à J-15 | SYS | — | — |
| INC-223 | Appliquer aux examens blancs | SYS (38) | mesures d'examen de la scolarité | — |
| INC-224 | Reconduire une décision d'un examen antérieur | REF · idem | dans les règles de l'examen | — |

### 13.10 ULIS, UPE2A, SEGPA, allophones
| ID | Action | Qui · droit | Règles | Événement |
|---|---|---|---|---|
| INC-230 | Enregistrer l'orientation ULIS (notification) et la classe de référence | REF, CUL · `inclusion.dispositifs.gerer` | 2 et 5 | `inclusion.ulis_entree` |
| INC-231 | Construire l'emploi du temps d'inclusion | CUL · `inclusion.ulis.gerer` | port du 10 | `edt.inclusions` |
| INC-232 | Prévenir les professeurs d'accueil | SYS | — | — |
| INC-233 | Faire le bilan d'inclusion par période | CUL · idem | — | — |
| INC-234 | Enregistrer un élève allophone (positionnement par compétences, langue de scolarisation antérieure) | UPE · `inclusion.upe2a.gerer` | ni nationalité ni pays | `inclusion.upe2a_entree` |
| INC-235 | Planifier l'inclusion progressive par matière | UPE · idem | 10 | — |
| INC-236 | Sortir du dispositif (date, bilan) | UPE · idem | alerte à l'échéance | `inclusion.upe2a_sortie` |
| INC-237 | Envoyer les documents dans la langue de la famille (modèles validés) | REF, UPE · idem | jamais traduction automatique d'un document signé | — |
| INC-238 | Enregistrer une orientation SEGPA / EGPA (modalité) | REF · idem | sans contenu de commission | `eleve.modalite` |

### 13.11 Transitions, transmission, pilotage, conservation
| ID | Action | Qui · droit | Règles | Événement |
|---|---|---|---|---|
| INC-250 | Proposer le dossier de liaison au départ | REF · `inclusion.transmettre` | accord requis | `inclusion.liaison_proposee` |
| INC-251 | Recueillir l'accord de transmission | PAR, ELM · lien | chaque titulaire ; opposition possible | `inclusion.liaison_accord` |
| INC-252 | Transmettre au nouvel établissement (lien court, ou papier) | REF · idem | bordereau | `inclusion.liaison_transmise` |
| INC-253 | Recevoir un dossier de liaison | REF · idem | reprise CINC-01 | `inclusion.liaison_recue` |
| INC-254 | Préparer la liaison école–collège / collège–lycée (commission de juin) | REF · idem | liste des arrivants avec besoin, sans contenu avant accord | — |
| INC-260 | Voir le tableau de bord du cycle (étapes, échéances, pièces) | REF, DIR · `inclusion.dispositifs.lire` | — | — |
| INC-261 | Tableau de rentrée (à revoir, à signer, mesures non lues, AESH non affectées) | REF · idem | — | — |
| INC-262 | Statistiques anonymes (47) | DIR · `inclusion.pilotage.lire` | < 5 masqué | — |
| INC-263 | Relevé de conformité d'un élève (étapes, signatures, lectures, ESS) | REF, DIR · idem | sans contenu | `inclusion.releve_conformite` |
| INC-270 | Lire le journal des accès d'un dossier | REF, DPO · `inclusion.journal.lire` | — | — |
| INC-271 | Traiter une demande d'accès RGPD (titulaire, majeur) | REF, DPO · `inclusion.acces_droits.traiter` | export du dossier pour la personne | `inclusion.acces_traite` |
| INC-272 | Destruction à l'échéance (revue, procès-verbal sans contenu) | REF · `inclusion.conservation.gerer` | journal des destructions | `inclusion.donnees_detruites` |
| INC-273 | Rejouer les destructions après restauration | SYS | avant réouverture | `inclusion.destructions_rejouees` |
| INC-274 | Ouvrir l'accès des parents d'un élève majeur | ELM · lien | portée, révocable | `inclusion.acces_majeur` |
| INC-280 | Enregistrer l'opposition d'un titulaire | PAR · lien ; REF | transmise à la direction | `inclusion.opposition` |
| INC-281 | Gel en cas de décès ou de départ | SYS | aucune relance | — |
| INC-282 | Voir mes mesures expliquées (élève) | ELV · lien | langage adapté | — |
| INC-283 | Dire « cette mesure m'aide / me gêne » | ELV, ELM · lien | référent | `inclusion.retour_eleve` |

**Compte (conception)** : 119 actions numérotées de INC-100 à INC-283 ; **+ 32** de la passe personas (§30) = **151**.

---

## 14. Règles transverses (R-INC1 et suivantes) [R]

| # | Règle | Lot |
|---|---|---|
| R-INC1 | **Aucun diagnostic** dans une table du 35 ; contrôle en base (codes) ; lint doux sur les textes libres restants | IN0, IN3 |
| R-INC2 | **Catalogue en instructions, sans trouble** : test sur liste noire (étend `accompagnement.rs:1018`) | IN3 |
| R-INC3 | **Cycle à étapes** : chaque dispositif a une étape courante, un responsable, une échéance | IN2 |
| R-INC4 | **PAP seulement après avis favorable** du médecin (port 34) | IN2 |
| R-INC5 | **Mesure poussée au contexte** (appel, devoir, évaluation, remplaçant, sortie, évacuation) ; appliquée par défaut | IN3 |
| R-INC6 | **Professeur : mesures de ses élèves dans ses matières, sans dispositif** | IN0 |
| R-INC7 | **Aucune liste établissement** hors référent et direction ; liste de session d'examen sans dispositif pour le secrétariat | IN0 |
| R-INC8 | **Autorité parentale et majorité** : seuls les titulaires ; fermeture à 18 ans ; réouverture par l'élève | IN0 |
| R-INC9 | **Version signée = version figée** (empreinte, ajout seul) | IN2 |
| R-INC10 | **AESH** : agent du 6, périmètre = élèves accompagnés × plages × dates ; ni notes, ni santé, ni discipline | IN4 |
| R-INC11 | **Trace des lectures transactionnelle et fail-closed**, sans contenu | IN0, IN1 |
| R-INC12 | **Aucun contenu dans les événements, SMS, push, objets d'e-mail** (enveloppe neutre) | IN0 |
| R-INC13 | **Discrétion** : pas d'affichage projeté ni mural ; plan de classe imprimé sans mesure | IN3 |
| R-INC14 | **Invité extérieur** (enseignant référent) : élèves désignés, durée, révocation, journal | IN1 |
| R-INC15 | **Pas de rôle supérieur** : l'administrateur technique ne lit aucun dispositif ; le bris de glace du 2 n'ouvre pas le 35 | IN0 |
| R-INC16 | **Notification MDPH = pièce C3** au coffre, lue du référent, du coordonnateur ULIS, de l'enseignant référent | IN2 |
| R-INC17 | **Transmission avec accord** seulement ; jamais automatique | IN9 |
| R-INC18 | **Fusion d'élèves** : jamais automatique pour le 35 | IN0 |
| R-INC19 | **IA** : aucune donnée de santé indirecte du 35 ; PPRE en proposition relue ; C0 pour les supports | IN0 (garde), IN7 |
| R-INC20 | **Conservation bornée** (§16), destructions journalisées et rejouées | IN11 |
| R-INC21 | **Petits effectifs masqués** (< 5) hors du référent | IN11 |
| R-INC22 | **Public : pas de double saisie du LPI** ; référence + mesures | IN2 |
| R-INC23 | **Pas de surveillance des professeurs** : application agrégée par mesure ; nominatif au seul référent, jamais à la direction comme évaluation | IN3 |
| R-INC24 | **Pas de traduction automatique** d'un document signé ; modèles traduits validés | IN8 |
| R-INC25 | **UPE2A** : ni nationalité ni pays d'origine ; langue de scolarisation antérieure seulement | IN8 |
| R-INC26 | **Plages AESH sans conflit** de lieu ni d'heure (H-16 du 10) | IN4 |
| R-INC27 | **Demande d'examen** : alerte avant la date limite ; jamais bloquante | IN6 |
| R-INC28 | **Données de démonstration sans trouble nommé** | IN0 |
| R-INC29 | **Un seul service inclusion** : toute route (historique, Leptos, mobile, IA, labo) passe par les mêmes commandes ; garde de code | IN1 |
| R-INC30 | **Tests sur données fictives seulement** | toujours |

---

## 15. Droits et minimisation (D-017) [R]

| Droit | Contenu | Portées | Modèles par défaut |
|---|---|---|---|
| `inclusion.parametrer` | référents, mode LPI, rappels, organisation AESH | établissement | Direction |
| `inclusion.dispositifs.gerer` / `.lire` | dossiers, versions, étapes | établissement, mes élèves | Référent ; PP (mes classes, PPRE et contributions) ; CUL (ULIS) ; Direction (lecture) |
| `inclusion.signer_chef` | signature du chef | établissement | Direction |
| `inclusion.notifications.gerer` / `.lire` | notifications CDAPH (pièces) | établissement / désignés | Référent / CUL, ER |
| `inclusion.mesures.gerer` | définir, retirer | établissement | Référent, CUL |
| `inclusion.mesures.lire` | lire | séance, mes classes, accompagnés | Professeur, remplaçant, AESH |
| `inclusion.contribuer` | bilans, mesures proposées | mes classes | Professeur |
| `inclusion.signaler` | signaler un besoin, une non-application | mes classes | Professeur, CPE, AESH, INF |
| `inclusion.proposer` | proposition depuis le conseil | classe | PP, Direction |
| `inclusion.ppre.gerer` | PPRE | mes classes | PP, Professeur |
| `inclusion.tutorat.gerer` | tutorat | mes classes | PP, CPE |
| `inclusion.accompagnement.gerer` / `.lire` | AESH, plages | établissement / moi | Référent, CUL / AESH |
| `inclusion.notes_accompagnement.saisir` | observations | accompagnés | AESH |
| `inclusion.reunions.gerer` / `.lire` | réunions | établissement / participants | Référent, ER / participants |
| `inclusion.examens.gerer` | demandes d'examen | établissement | Référent, Secrétariat (liste de session, sans dispositif) |
| `inclusion.ulis.gerer`, `inclusion.upe2a.gerer` | dispositifs collectifs | dispositif | CUL, UPE |
| `inclusion.invites.gerer` | enseignant référent invité | établissement | Référent |
| `inclusion.transmettre` | liaison | établissement | Référent |
| `inclusion.pilotage.lire` | agrégats | établissement | Direction, Référent |
| `inclusion.journal.lire` | journal des accès | établissement | Référent, DPO |
| `inclusion.acces_droits.traiter`, `inclusion.conservation.gerer` | droits RGPD, destruction | établissement | Référent, DPO |

**Alias de transition** : `accompagnement.gerer` → `inclusion.dispositifs.gerer` (Référent ; **retiré d'Admin**, lecture seule pour Direction) ; `amenagements.lire` → `inclusion.mesures.lire` (portées « mes classes » et « séance » seulement ; **retiré** d'Administratif, CPE, Infirmerie, Psy à l'échelle de l'établissement).

**Pièges à tester** : (1) administrateur, accueil, AED : aucune route du 35 ; (2) secrétariat : liste de session d'examen seulement, sans dispositif ; (3) professeur : ses élèves, ses matières, sans nature de dispositif ; (4) AESH : ses élèves, ses plages, ni notes ni santé ; (5) parent sans autorité, parents d'un majeur, parent interdit : rien ; (6) `/eleves/{id}/dossier` : pas de libellé libre, pas de nature hors cercle ; (7) événements, SMS, e-mails : neutres ; (8) IA : refus ; (9) invité révoqué : rien, immédiatement ; (10) journal générique : n'expose pas l'existence d'un PAP hors cercle.

---

## 16. Conservation [R, durées soumises au DPO]

| Donnée | Durée proposée | Raison |
|---|---|---|
| Dispositif PAP / PPRE (versions, signatures) | fin du dispositif + **1 an**, ou départ de l'élève + 1 an ; dernière version transmise avec accord | preuve de ce qui était prévu ; minimisation |
| Mise en œuvre du PPS, ESS, relevés | fin de scolarité dans l'établissement + **1 an** | suivi pluriannuel |
| Notification CDAPH (pièce) | fin de validité + 1 an | — |
| Mesures, applications notées | année scolaire + 1 an | contestation |
| Accompagnement AESH (affectations, plages) | année + 1 an ; heures agrégées sans limite (anonymes) | — |
| Notes d'accompagnement de l'AESH | fin de l'accompagnement + 1 an | — |
| Demandes et décisions d'examen | session + **1 an** (recours) | [R] |
| Signalements de besoin sans suite | **1 an** | minimisation |
| Journal des accès | **1 an**, 3 ans si l'AIPD le justifie (aligné 34 §30.7) | CNIL 2021-122 via 34 |
| Statistiques | sans limite (anonymes) | — |

---

## 17. Événements (outbox) [R]
**Émis** (identifiants et états ; **aucune nature de dispositif dans la charge utile publique**, aucun code de mesure, aucun nom) : `inclusion.dossier_ouvert|clos`, `inclusion.besoin_signale`, `inclusion.proposition`, `inclusion.avis_recu`, `inclusion.version_projet|version_creee|a_signer|signe|en_vigueur|reconduit|clos`, `inclusion.notification_recue|notification_expire`, `inclusion.mesures_publiees` (contextes concernés : matières, séances), `inclusion.mesure_retiree`, `inclusion.mesures_lues`, `inclusion.non_application|non_application_traitee`, `inclusion.accompagnement_affecte|plages_definies|accompagnement_temporaire|accompagnement_non_assure|accompagnement_fin`, `inclusion.reunion_planifiee|convocation|reunion_tenue|releve_diffuse`, `inclusion.campagne_examens|demande_examen_*|decision_examen`, `inclusion.ulis_entree`, `inclusion.upe2a_entree|upe2a_sortie`, `inclusion.liaison_*`, `inclusion.opposition`, `inclusion.acces_majeur`, `inclusion.donnees_detruites|destructions_rejouees`.
**Consommateurs** : 10 (plages → planning AESH ; inclusions ULIS), 12 (pastille, présences de la scolarisation partagée), 20 et 21 (mesures), 15 (absence AESH), 38 (décisions d'examen), 42, 14, 19 (réunions restreintes), 8 (avis), 2 (pastille, chronologie **côté cercle**).
**Consommés** : `eleve.cree|majeur|decede|fusionne|sortie|classe_changee` (2) ; `lien.*`, `interdiction.*` (4) ; `agent.fonction_*`, `mission.*` (6) ; `seance.*`, `edt.version_publiee` (10) ; `conseil.proposition` (23) ; `sante.avis_medical` (34) ; `absence_personnel.*` (15).
**Historiques** : `eleve.dispositif_ajoute`, `accompagnement.*` (`accompagnement.rs:26-30`) renommés avec alias de lecture.

---

## 18. Notifications (natures du socle 8) [R]
| Nature | Urgence · sensibilité | Destinataires | Contenu externe |
|---|---|---|---|
| `inclusion.a_signer` | U1 · C3 | chaque signataire pour soi | « Un document est à signer » |
| `inclusion.convocation` | U1 · C3 | participants | « Vous êtes invité(e) à une réunion le {date} — ouvrez l'ENT » |
| `inclusion.mesures_nouvelles` | U1 · C2 | professeurs concernés, AESH | in-app seulement |
| `inclusion.accompagnement_non_assure` | U2 · C2 | professeurs des séances, VS, famille si prévu | « L'accompagnement de {prénom} n'est pas assuré ce matin » (in-app ; externe : enveloppe neutre) |
| `inclusion.echeance` | U0 · C2 | référent | in-app, résumé |
| `inclusion.demande_examen` | U1 · C3 | famille, référent | « Une demande est à valider » |
| `inclusion.non_application` | U1 · C2 | référent | in-app |
| `inclusion.changement_seance` | U2 · C1 | AESH | push « changement pour demain » sans nom d'élève |
| `inclusion.majorite` | U0 · C1 | élève majeur | in-app |

---

## 19. Écrans clés et mobile [R]
| Écran | Où | Ce qu'il montre |
|---|---|---|
| **Inclusion — tableau de bord** | `/web/inclusion` | cycle : à préparer, à signer, en vigueur, à revoir ; échéances ; ESS du mois ; demandes d'examens ; AESH ; non-applications (tuiles Bento) |
| **Dossier d'un élève** | `/web/inclusion/eleves/{id}` | étapes, versions, signatures, mesures par matière, accompagnement, réunions, pièces, journal |
| **Mesures de ma classe** | `/web/classes/{id}/mesures` | professeur : élèves, mesures de ses matières, accusé de lecture |
| **Planning AESH** | `/web/inclusion/mon-planning` | jour et semaine, élèves, salles, mesures, changements |
| **Campagne d'examens** | `/web/inclusion/examens/{session}` | candidats, procédure, état des demandes, date limite |
| **ULIS / UPE2A** | `/web/inclusion/dispositifs/{id}` | élèves, inclusions, bilans |
| **Réunion** | `/web/inclusion/reunions/{id}` | participants, présences, bilans, relevé, compte rendu |
| **Famille** | espace famille | document, mesures, convocations, signature, signaler une non-application, demander une révision |
| **Élève** | espace élève | mes mesures expliquées, « ça m'aide / ça me gêne », mes réunions |

**Mobile (D-003)** : AESH — planning du jour, changements, mesures, observation rapide, consignes de sécurité hors ligne pour une sortie (42) ; professeur — pastilles dans l'appel, mesures de la séance ; famille — signature, convocation, signalement ; élève — ses mesures. Écran sécurisé (capture bloquée sur Android, masqué dans le sélecteur), aucune copie locale du document.

---

## 20. Modèle de données cible (brique `inclusion`) [R]

Principe : nouvelles tables `ent_inc_*` ; reprise des tables 016/212 ; **toutes** avec `etablissement_id UUID NOT NULL` (E1), FK **composites** `(etablissement_id, x_id)` (E2), UUID applicatifs (E3), aucune table globale de données d'établissement (E4), pièces = objets `ent_objet` (E5, aucun `BYTEA`), `cree_le` / `modifie_le` (E6), journal `ent_journal_modif` pour les actions sensibles (E7), migrations additives (E8), files d'envoi déclarées neutralisées (E9), **aucun JSON ni tableau** (E10, D-036), clés inscrites (E11), registre E12, RLS (E13), journaux sans contenu (E14). Aucun `ON DELETE CASCADE` sur une donnée d'historique.

```
-- Références globales (E12 « globale_reference », produit) — sans donnée d'établissement
ref_inc_type_dispositif        (code, libelle, sante_indirecte BOOL, officiel BOOL)
ref_inc_mesure                 (code, libelle_instruction, unite NULL ('pourcent','minutes','points'), contextes_permis_code)
ref_inc_mesure_contexte        (mesure_code, contexte ('cours','ecrit','oral','tp','eps','devoir','evaluation','examen','sortie','evacuation','pause_meridienne'))
ref_inc_decision_cdaph         (code, libelle)       -- aesh_individuelle, aesh_mutualisee, materiel, ulis, segpa, ems, transport, amenagements
ref_inc_motif_fin, ref_inc_motif_impossible

-- Établissement
ent_inc_parametre              (etablissement_id, mode_document ('scolia','lpi'), rappel_jours, delai_convocation_jours, organisation_aesh_code, organisation_aesh_libelle)
ent_inc_referent               (etablissement_id, id, agent_id, du, au)
ent_inc_invite                 (etablissement_id, id, personne_id, qualite ('enseignant_referent','autre'), du, au, revoque_le, revoque_par)
ent_inc_invite_eleve           (etablissement_id, invite_id, eleve_id)
ent_inc_dossier                (etablissement_id, id, eleve_id, type_code, etape ('preparation','avis','redaction','signature','en_vigueur','revision','clos'),
                                ouvert_le, ouvert_par, echeance, clos_le, motif_fin_code, mode_document, reference_lpi NULL)
ent_inc_dossier_etape          (etablissement_id, id, dossier_id, etape, le, par)                       -- ajout seul
ent_inc_besoin_signale         (etablissement_id, id, eleve_id, signale_par, le, texte_chiffre, piste ('ppre','pap','mdph','aucune'), traite_le, dossier_id NULL)
ent_inc_avis                   (etablissement_id, id, dossier_id, demande_le, sens ('favorable','defavorable'), rendu_le, medecin_personne_id, sante_ref UUID NULL)  -- sans motif
ent_inc_version                (etablissement_id, id, dossier_id, numero, empreinte, cree_par, cree_le, en_vigueur_du, en_vigueur_au, document_objet_id NULL) -- ajout seul
ent_inc_signature              (etablissement_id, id, version_id, partie ('chef','titulaire','eleve','medecin','coordonnateur'), personne_id, signe_le, preuve_ref (socle 7), empreinte)
ent_inc_opposition             (etablissement_id, id, dossier_id, personne_id, le, document_objet_id NULL, leve_le)
ent_inc_notification           (etablissement_id, id, eleve_id, reference, decide_le, du, au, piece_objet_id)
ent_inc_notification_decision  (etablissement_id, notification_id, decision_code, heures_hebdo NULL)
ent_inc_mesure                 (etablissement_id, id, eleve_id, version_id NULL, dossier_id NULL, mesure_code, valeur NUMERIC NULL, du, au, retire_le, retire_par, motif_retrait_code)
ent_inc_mesure_portee          (etablissement_id, mesure_id, contexte, matiere_id NULL)
ent_inc_mesure_lecture         (etablissement_id, mesure_id, agent_id, lu_le)
ent_inc_application            (etablissement_id, id, mesure_id, seance_id NULL, evaluation_id NULL, agent_id, etat ('appliquee','impossible','levee'), motif_code NULL, le)
ent_inc_non_application        (etablissement_id, id, mesure_id, signale_par_personne_id, le, traite_le, reponse_chiffree)
ent_inc_accompagnement         (etablissement_id, id, eleve_id, agent_id, type ('individuelle','mutualisee','collective'), notification_id NULL, heures_notifiees, du, au, motif_fin_code, temporaire BOOL)
ent_inc_plage                  (etablissement_id, id, accompagnement_id, nature ('matiere','seance','pause_meridienne'), matiere_id NULL, seance_modele_id NULL, jour_semaine NULL, debut NULL, fin NULL)
ent_inc_note_accompagnement    (etablissement_id, id, accompagnement_id, le, par, texte_chiffre)       -- ajout seul
ent_inc_reunion                (etablissement_id, id, eleve_id, dossier_id NULL, nature ('equipe_educative','ess','suivi','bilan'), prevue_le, lieu, rdv_ref UUID NULL (socle 19), statut, annulee_motif)
ent_inc_reunion_participant    (etablissement_id, reunion_id, personne_id, qualite, convoque_le, present BOOL NULL)
ent_inc_reunion_bilan          (etablissement_id, id, reunion_id, matiere_id, auteur, texte_chiffre, le)
ent_inc_reunion_decision       (etablissement_id, id, reunion_id, decision ('mesure_ajoutee','mesure_retiree','demande_mdph','orientation','autre'), mesure_code NULL)
ent_inc_reunion_compte_rendu   (etablissement_id, id, reunion_id, texte_chiffre, par, le)             -- ajout seul (compléments)
ent_inc_session_examen         (etablissement_id, id, examen_code, session, date_limite)
ent_inc_demande_examen         (etablissement_id, id, session_id, eleve_id, procedure ('simplifiee','complete'), etat, prepare_le, signe_le, depose_le, decide_le, decision_objet_id NULL)
ent_inc_demande_examen_mesure  (etablissement_id, demande_id, mesure_code, valeur NULL, accordee BOOL NULL)
ent_inc_collectif              (etablissement_id, id, nature ('ulis','upe2a','segpa'), coordonnateur_agent_id, libelle)
ent_inc_collectif_eleve        (etablissement_id, id, collectif_id, eleve_id, du, au, classe_reference_id)
ent_inc_inclusion              (etablissement_id, id, collectif_eleve_id, matiere_id, du, au)          -- demande au 5/10
ent_inc_positionnement         (etablissement_id, id, eleve_id, le, langue_scolarisation_code, competence_code, niveau_code)
ent_inc_ppre                   (etablissement_id, id, dossier_id, periode_id)
ent_inc_ppre_objectif          (etablissement_id, id, ppre_id, competence_ref NULL, texte, propose_par_ia BOOL, accepte_par, accepte_le, bilan_code NULL)
ent_inc_tutorat                (etablissement_id, id, eleve_id, tuteur_personne_id, nature ('adulte','pair'), du, au, objectif)
ent_inc_tutorat_rencontre      (etablissement_id, id, tutorat_id, le, note_courte)
ent_inc_liaison                (etablissement_id, id, eleve_id, sens ('envoi','reception'), version_id NULL, accord_le, transmis_le, bordereau_objet_id)
ent_inc_acces_majeur           (etablissement_id, id, eleve_id, personne_id, portee ('convocations','document','tout'), du, revoque_le)
ent_inc_retour_eleve           (etablissement_id, id, mesure_id, le, sens ('aide','gene'))
ent_inc_acces                  (etablissement_id, id, personne_id, casquette, categorie, objet_id, contexte, le)  -- ajout seul, sans contenu
ent_inc_destruction            (etablissement_id, id, categorie, objet_id, le, pv_objet_id)          -- conservé hors rétention, rejoué
```

Chiffrement : `texte_chiffre`, `reponse_chiffree` avec la clé « inclusion » de l'établissement (`enc:v2:<id_cle>:`, E11), distincte de celle de la santé (34) et du suivi (36) [R].
Registre (E12) : `ent_inc_*` → portée `etablissement`, catégorie d'export **« particulière »** pour les tables de dispositifs, notifications, réunions, accompagnement et mesures (santé indirecte) ; `ref_inc_*` → `globale_reference` ; aucune file d'effet de bord propre (notifications par le 8).

---

## 21. Plan de migration depuis l'existant

| Étape | Quoi | Garantie |
|---|---|---|
| MIG-1 (IN0) | Droits : `accompagnement.gerer` retiré d'Admin (Direction : lecture et signature) ; `amenagements.lire` réduit à `mes_classes` et `seance` ; parent : contrôle d'autorité, d'interdiction et de majorité ; `/dossier` sans libellé ni nature hors cercle ; test `actions.rs` réécrit | aucune donnée modifiée |
| MIG-2 (IN0, 2700) | FK composites et `RESTRICT` sur 212 ; `ent_inc_acces` (journal transactionnel) ; reclassement de `ent_equipe_reunion` au registre ; `libelle` chiffré ; gel des `kind` `pai` et `gevasco` (CHECK refusant de nouvelles lignes) | additive ; reprise idempotente |
| MIG-3 (IN2) | `ent_eleve_dispositif` → `ent_inc_dossier` + version 1 « reprise » ; `pai` → laissé au 34 (rapprochement avec `ent_san_pai`) ; `gevasco` → pièce du dossier | vues de compatibilité pour `ui.rs` |
| MIG-4 (IN3) | `ent_amenagement` → `ent_inc_mesure` + portée `cours` ou `examen` | idem |
| MIG-5 (IN4) | `ent_aesh_affectation` → `ent_inc_accompagnement` : rapprochement nom → agent par le référent (écran de correspondance), jamais automatique | rapport |
| MIG-6 (IN5) | `ent_equipe_reunion` → `ent_inc_reunion` + `ent_inc_reunion_compte_rendu` | — |
| MIG-7 (IN12) | Retrait des tables 212 et des routes avec `ui.rs` (F-001), après la durée d'archive | 2719, sur décision (E8) |

---

## 22. Lots de code (équipe Sonnet)

| Lot | Contenu | Critères de fin | Dépend de |
|---|---|---|---|
| **IN0** correctifs urgents | DINC-01 à DINC-05, DINC-07 (registre), DINC-08, DINC-09 ; garde IA ; seed sans trouble | matrice §15 testée ; test « secrétariat sans liste » ; test « parent d'un majeur » ; test « IA sans 35 » | — |
| **IN1** droits, cercles, journal | droits fins, alias, référents, invités, journal, garde de service unique | aucune lecture sans trace ; invité révoqué = 403 | IN0, socle 6 |
| **IN2** dispositifs | dossiers, étapes, avis (34), versions, signatures (7), notifications, référence LPI | version signée immuable ; CINC-01, CINC-02, CINC-09 | IN1, 7, 34 (port avis) |
| **IN3** mesures dans la classe | catalogue étendu, portées, ports `mesures_*`, pastille, lectures, application, non-application | appel, cahier de textes et évaluation reçoivent les mesures ; CINC-06 à CINC-08 | IN2, 10, 12, 20, 21 |
| **IN4** AESH | accompagnement, plages, planning dérivé, absence, notes, heures | CINC-03 à CINC-05, CINC-34 ; AESH ne voit que ses élèves | IN3, 6, 10, 15 |
| **IN5** réunions | réunions, participants, convocations (19), bilans, relevé, compte rendu | CINC-10 à CINC-12 | IN2, 19 |
| **IN6** examens | sessions, demandes, décisions, port 38 | CINC-15, CINC-16 | IN3, 38 |
| **IN7** PPRE et tutorat | PPRE, objectifs (IA proposée), tutorat | relecture une à une | IN2, 22, 23 |
| **IN8** ULIS, UPE2A, SEGPA | collectifs, inclusions, positionnement, modèles traduits | CINC-13, CINC-18, CINC-19 | IN3, 5, 10 |
| **IN9** transitions | liaison, accord, réception | CINC-01 ; R-INC17 | IN2 |
| **IN10** mobile | AESH, professeur, famille, élève | écran sécurisé | IN3, IN4, socle 11 |
| **IN11** pilotage, conservation | tableaux, statistiques, droits RGPD, destructions rejouées | CINC-29 | IN2 |
| **IN12** RLS stricte et retraits | test strict ; retrait 212 | 2718, 2719 | tous |

**Parcours minimal (V1, fin janvier 2027)** : IN0, IN1, IN2, IN3, IN4 ; IN5 et IN6 pour la rentrée 2027 (année de doublon, D-034) ; IN8 pour un établissement avec ULIS ou UPE2A.

---

## 23. Migrations (plage imposée 2700–2719)

| N° | Lot | Contenu |
|---|---|---|
| **2700** | IN0 | FK composites et `RESTRICT` sur les tables 212 ; `libelle` déclaré chiffré ; CHECK gelant `pai`/`gevasco` pour les nouvelles lignes ; `ent_inc_acces` |
| **2701** | IN1 | `ref_inc_*` (références globales) ; `ent_inc_parametre`, `ent_inc_referent`, `ent_inc_invite`, `ent_inc_invite_eleve` |
| **2702** | IN2 | `ent_inc_dossier`, `_dossier_etape`, `_besoin_signale`, `_avis` |
| **2703** | IN2 | `ent_inc_version`, `_signature`, `_opposition` (déclencheurs d'ajout seul) |
| **2704** | IN2 | `ent_inc_notification`, `_notification_decision` |
| **2705** | IN3 | `ent_inc_mesure`, `_mesure_portee`, `_mesure_lecture`, `_application`, `_non_application`, `_retour_eleve` |
| **2706** | IN4 | `ent_inc_accompagnement`, `_plage`, `_note_accompagnement` |
| **2707** | IN5 | `ent_inc_reunion`, `_reunion_participant`, `_reunion_bilan`, `_reunion_decision`, `_reunion_compte_rendu` |
| **2708** | IN6 | `ent_inc_session_examen`, `_demande_examen`, `_demande_examen_mesure` |
| **2709** | IN7 | `ent_inc_ppre`, `_ppre_objectif`, `_tutorat`, `_tutorat_rencontre` |
| **2710** | IN8 | `ent_inc_collectif`, `_collectif_eleve`, `_inclusion`, `_positionnement` |
| **2711** | IN9, IN11 | `ent_inc_liaison`, `ent_inc_acces_majeur`, `ent_inc_destruction` |
| **2712** | IN2–IN5 | reprise depuis 016/212 (MIG-3 à MIG-6), idempotente |
| 2713–2717 | — | réserve (passe personas : §30.10) |
| **2718** | IN12 | RLS stricte (test) |
| **2719** | IN12 | retraits des tables 212 et colonnes héritées (**destructif, sur décision**, sauvegarde vérifiée, E8) |

**D-036** : aucune colonne JSON/JSONB, aucun tableau, aucun `BYTEA` ; contrôle de revue `grep -iE "json|bytea|\[\]"` sur 2700–2719.

---

## 24. Tests et recette
- Matrice de séparation (§15, dix pièges) en test d'API ; test sentinelle des événements (aucun code de mesure, aucune nature) ; test « IA sans 35 » ; test du catalogue (liste noire) ; test « version signée immuable » ; test « AESH hors plage = 403 » ; test « parents d'un majeur » ; test « invité révoqué » ; test de restauration (destructions rejouées) ; données fictives seulement (R-INC30).
- **Recette** avec un référent inclusion, une AESH et un professeur principal réels (Q-INC2) sur un cycle complet : PAP de bout en bout, ESS, campagne d'examen, rentrée.

---

## 25. Réglages (D-023 : signalés, non supprimés)
| # | Réglage | Défaut |
|---|---|---|
| S-2700 | document PAP / PPS dans ScolIA ou dans le LPI | public : LPI ; privé : ScolIA |
| S-2701 | rappels d'échéance (jours) | 30 |
| S-2702 | relance des mesures non lues (jours) | 7 |
| S-2703 | délai minimal de convocation (jours) | 15 [R] |
| S-2704 | signature de l'élève mineur (à partir de quel niveau) | 6e, sans valeur juridique, signe « a pris connaissance » |
| S-2705 | famille prévenue d'une absence d'AESH | si le document le prévoit |
| S-2706 | suivi d'application par les professeurs (« appliquée / impossible ») | activé, facultatif |
| S-2707 | IA de proposition pour le PPRE | activée |
| S-2708 | alerte de demande d'examen (jours avant la date limite) | 15 |

**Usine à gaz signalée** : S-2706 peut être ressenti comme une surveillance ; il reste facultatif et agrégé (R-INC23).

---

## 26. Questions (recommandations appliquées par défaut, D-031, sauf mention)
| # | Question | Recommandation |
|---|---|---|
| Q-INC1 | Lancer IN0 tout de suite ? | **Oui** (appliqué) |
| Q-INC2 | Référents métier (référent inclusion, AESH, enseignant référent) pour la relecture et la recette | **Soumis à Florian** (personnes réelles) |
| Q-INC3 | Interface avec le LPI | **Soumis à Florian** (démarche officielle) ; d'ici là, référence + mesures, sans interface |
| Q-INC4 | AIPD commune 34–35 et classement « santé indirecte » | **Soumis au DPO** ; appliqué par défaut : traité comme donnée de santé |
| Q-INC5 | PAP dans le privé : qui donne l'avis ? | « médecin désigné » ; **à confirmer par un juriste du privé** (via Florian) |
| Q-INC6 | Professeur : mesures sans nature du dispositif | **Oui** (appliqué) |
| Q-INC7 | Secrétariat : seulement la liste de session d'examen | **Oui** (appliqué) |
| Q-INC8 | AESH : planning, cahier de textes, mesures de ses élèves ; ni notes ni santé ni discipline | **Oui** (appliqué) |
| Q-INC9 | Un titulaire suffit à mettre en place, l'autre informé | **Oui** (appliqué) |
| Q-INC10 | Majorité : accès parental fermé à 18 ans | **Oui** (appliqué) |
| Q-INC11 | Enseignant référent en accès invité (identité fédérée ou compte invité) | **Oui** (appliqué) |
| Q-INC12 | Notification MDPH lue du référent, du coordonnateur ULIS et de l'enseignant référent seulement | **Oui** (appliqué) |
| Q-INC13 | IA : aucune donnée de santé indirecte ; PPRE proposé ; supports C0 | **Oui** (appliqué) |
| Q-INC14 | Ne pas coder PIAL / PAS en dur | **Oui** (appliqué) |
| Q-INC15 | Transmission seulement avec accord | **Oui** (appliqué) |
| Q-INC16 | Durées §16 | **Soumis au DPO** ; défauts appliqués |
| Q-INC17 | UPE2A sans nationalité ni pays | **Oui** (appliqué) |
| Q-INC18 | Suivi d'application facultatif et agrégé | **Oui** (appliqué) |
| Q-INC19 | Demandes d'examen sans interface académique au premier temps | **Oui** (appliqué) |
| Q-INC20 | Clé de chiffrement « inclusion » séparée | **Oui** (appliqué) ; inventaire E11 |
| Q-INC21 | Plage 2700–2719 | **Réservée** (imposée par le pilote) |
| Q-INC22 | Mesures d'examen appliquées aux examens blancs par défaut | **Oui** (appliqué) |

---

## 27. Décisions appliquées par défaut (liste pour le point quotidien)
Q-INC1, Q-INC6 à Q-INC15, Q-INC17 à Q-INC22 (19 questions), plus les défauts de durées (Q-INC16) et de classement (Q-INC4) en attendant le DPO. **Soumis** : Q-INC2, Q-INC3, Q-INC4 (DPO), Q-INC5 (juriste), Q-INC16 (DPO).

---

## 28. Faits, hypothèses, contradictions
**Faits [F]** : §2 (code lu sur `afeb8c4`) ; plages vérifiées sur 134 branches.
**Vérifié en extrait [Ve], à relire sur le texte entier** : D311-13 ; D351-10 à D351-12 ; loi 2024-475 et décret de février 2025 (numéro incertain) ; circulaire PAS du 4/07/2024 et rejet de la généralisation (11/05/2026) ; D351-27 et s., décret 2020-1523, circulaire du 8/12/2020 ; décret 2021-1246 et arrêté du 30/09/2021 (LPI).
**Non vérifié [NV]** : circulaires 2015-016, 2015-129, 2012-141, 2015-176 ; arrêté du 6/02/2015 ; L917-1 ; discrétion de l'AESH ; usage du LPI dans le privé ; instruction de tri.
**Hypothèses [H]** : année vue du référent (§4) ; maintien provisoire des mesures à l'expiration d'une notification (CINC-02) ; origine révélée par le statut d'allophone.
**Contradictions relevées** :
1. `21_EVALUATIONS.md` §6 dit « 35 possède PAP, PPS, PPRE, **PAI** » ; le 34 et ce document gardent le **PAI au 34** (déjà signalé au 34 §28 point 2 ; `21` reste à corriger).
2. `SOCLES.md` §35 range le PAI implicitement dans « aménagements » ; le 35 ne garde que son existence.
3. Le code range `pai` et `gevasco` parmi les dispositifs (DINC-05) : gelés en IN0.
4. `SOCLES.md` dit « élèves allophones [H] » : confirmé comme dispositif UPE2A (circulaire 2012-141 [NV]).
5. Le PAS est présenté par certains sites grand public comme généralisé à la rentrée 2026 : **contredit** par le vote du 11/05/2026 ; ScolIA reste neutre (Q-INC14).
6. `06_PERSONNELS.md` P24 évoque une « organisation en pôle [NV] » : même neutralité.

---

## 29. Les cinq décisions prioritaires
1. **Lancer IN0** : le secrétariat liste tous les élèves aménagés, les parents d'un majeur et les parents sans autorité lisent ses aménagements, l'administrateur lit les comptes rendus d'ESS. **Appliqué par défaut.**
2. **Des mesures sans cause, poussées dans la classe** (R-INC5, R-INC6). **Appliqué.**
3. **L'AESH, personne de l'établissement**, avec son planning et un périmètre strict (R-INC10). **Appliqué** ; dépend du socle 6.
4. **Pas de double saisie du LPI dans le public** ; interface éventuelle = démarche officielle. **Soumis à Florian.**
5. **AIPD et classement santé indirecte** avec le 34. **Soumis au DPO.**

---

## 30. Ajouts de la passe personas (2026-11-11, `PERSONAS_35.md`)

**Statut** : proposition ; ce paragraphe **l'emporte** sur le texte ci-dessus en cas de contradiction.

### 30.1 Actions ajoutées (INC-300 à INC-331)
| ID | Action | Qui · droit | Règles | Événement |
|---|---|---|---|---|
| INC-300 | Voir « ce que l'établissement sait de mon enfant » (dispositif, mesures, lecteurs par catégorie) | PAR, ELM · lien | R-INC8 | — |
| INC-301 | Demander une réunion | PAR, ELM · lien | réponse sous délai (S-2709) | `inclusion.reunion_demandee` |
| INC-302 | Choisir la langue des convocations | PAR · lien | modèles validés | — |
| INC-303 | Participer à distance à une ESS (visio, 33) | participants | lien nominatif | — |
| INC-304 | Déléguer la lecture à une personne de confiance (sans signature) | PAR · lien | révocable ; aucune santé | `inclusion.delegation` |
| INC-305 | Préparer la rentrée d'un arrivant d'école (liaison de juin) | REF · `inclusion.transmettre` | avec accord | — |
| INC-306 | Voir les élèves accompagnés d'une séance où je remplace l'AESH | AESH remplaçante · `inclusion.accompagnement.lire@moi` | temporaire | — |
| INC-307 | Recevoir le plan de la journée de sortie pour un élève accompagné (42) | AESH · idem | hors ligne, effacé au retour | — |
| INC-308 | Déclarer une indisponibilité de plage (formation, réunion) | AESH · idem | référent prévenu | `inclusion.plage_indisponible` |
| INC-309 | Voir mes heures planifiées vs notifiées | AESH · idem | pour elle-même | — |
| INC-310 | Signaler un problème d'accessibilité (salle, matériel) | AESH, PRF · `inclusion.signaler` | vers le 44 | — |
| INC-311 | Copier les mesures d'un dispositif dans un autre (PAP → PPS) | REF · `inclusion.mesures.gerer` | relecture | — |
| INC-312 | Clore un dossier à la sortie d'un élève (départ) en un geste | REF · idem | liaison proposée | — |
| INC-313 | Désigner un suppléant du référent pendant une absence | DIR · `inclusion.parametrer` | daté | — |
| INC-314 | Voir l'aide à l'évacuation des élèves d'une zone | responsable de zone · port 14 | sans dispositif | — |
| INC-315 | Exporter une liste de session d'examen au format attendu par l'académie | ADF · `inclusion.examens.gerer` | sans dispositif ; filigrane ; journal | `inclusion.export_session` |
| INC-316 | Marquer une épreuve d'examen blanc « mesure non appliquée » (38) | surveillant · port 38 | retour au référent | — |
| INC-317 | Rechercher un élève par mesure (référent seulement) | REF · `inclusion.dispositifs.lire` | journal | — |
| INC-318 | Produire le dossier d'avis du DPO (tables, matrice, durées, tests) | REF, DPO · `inclusion.journal.lire` | — | — |
| INC-319 | Évaluer le périmètre d'une fuite (lecteurs, exports) | DPO · idem | sans contenu | `inclusion.fuite_evaluee` |
| INC-320 | Bloquer un compte qui fait des lectures massives | SYS | seuil S-2710 | `inclusion.lecture_massive` |
| INC-321 | Revoir les invités expirés | REF · `inclusion.invites.gerer` | mensuel | — |
| INC-322 | Imprimer la fiche d'accueil d'une AESH arrivante (élèves, plages, contacts) | REF · idem | sans dispositif | — |
| INC-323 | Archiver une version LPI reçue en pièce (privé ou transfert) | REF · idem | coffre C3 | — |
| INC-324 | Neutraliser une évaluation pour un élève dispensé (port 21) | REF · idem | sans motif | — |
| INC-325 | Prévoir un accompagnement à la pause méridienne visible de la restauration (40) | REF · idem | « accompagnant présent » sans motif | — |
| INC-326 | Afficher à l'élève les mesures qu'il peut demander lui-même (temps, pause) | ELV · lien | — | — |
| INC-327 | Historique des mesures d'un élève (qui, quand, quoi) | REF, PAR · idem / lien | — | — |
| INC-328 | Demander la reconduction des aménagements d'examen d'une session antérieure | PAR, ELM · lien | — | — |
| INC-329 | Avertir quand un professeur arrivant n'a pas lu les mesures après sa première séance | SYS | — | — |
| INC-330 | Rendre les mesures disponibles pour l'emploi du temps (contraintes de salle : rez-de-chaussée) | SYS → 10 | contrainte sans motif | — |
| INC-331 | Retirer toutes les données d'un dossier ouvert par erreur (mauvais élève) | REF · `inclusion.conservation.gerer` | double validation, journal des destructions | `inclusion.donnees_detruites` |

### 30.2 Règles ajoutées
| # | Règle | Lot |
|---|---|---|
| R-INC31 | **Contraintes d'emploi du temps sans motif** (rez-de-chaussée, salle calme) transmises au 10 comme exigences de salle, jamais avec le nom d'un dispositif | IN3 |
| R-INC32 | **Remplaçante d'AESH** : accès temporaire calculé sur les plages remplacées seulement | IN4 |
| R-INC33 | **Export de session** : seul export nominatif permis hors référent ; filigrane ; journal ; destruction conseillée | IN6 |
| R-INC34 | **Lecture massive** (plus de S-2710 dossiers en une heure) : alerte au référent et au DPO, blocage après double seuil | IN1 |
| R-INC35 | **Personne de confiance** : lit convocations et mesures, ne signe pas, ne voit jamais la notification | IN9 |
| R-INC36 | **Restauration et cantine** : le 40 ne lit qu'« accompagnant présent » | IN4 |
| R-INC37 | **Remplaçant de professeur** : mesures de la séance au moment de sa désignation, retirées après la séance | IN3 |

### 30.3 Cas limites ajoutés (CINC-37 à CINC-48)
| # | Cas | Réponse |
|---|---|---|
| CINC-37 | AESH qui découvre un élève en détresse | parcours du 36 (« signaler une préoccupation ») ; rien n'est écrit au 35 |
| CINC-38 | Enseignant référent d'un autre département | invité ; identité fédérée ou compte invité ; durée |
| CINC-39 | Père qui obtient l'autorité parentale en cours d'année | accès ouvert à la date ; historique accessible ; signature proposée à la révision |
| CINC-40 | Élève accompagné par une AESH qui est aussi sa tante | déclaration de lien (socle 6) ; le référent décide ; aucune règle automatique |
| CINC-41 | Notes d'accompagnement qui citent un trouble | lint et rappel ; le référent peut demander une correction (ajout) |
| CINC-42 | Inspection qui demande les heures d'AESH | agrégats ; nominatif au référent |
| CINC-43 | Demande d'accès RGPD d'un parent qui veut voir les notes de l'AESH | communication des données le concernant l'enfant, sous réserve des droits d'autrui (notes descriptives, pas d'opinion sur des tiers) [R, DPO] |
| CINC-44 | Professeur qui recopie les mesures dans un tableur personnel | impression filigranée seulement ; rappel ; pas d'export de liste |
| CINC-45 | Élève en ULIS en voyage scolaire avec sa classe de référence | 42 lit besoins et accompagnant ; AESH encadrante |
| CINC-46 | Collège qui ferme l'UPE2A, élèves répartis | collectif clos, positionnements conservés, inclusions recalculées |
| CINC-47 | Démo de décembre 2026 | données fictives, aucun trouble nommé, aucun établissement réel |
| CINC-48 | Restauration à J-3 après la destruction d'un dossier ouvert par erreur | destruction rejouée (INC-273) |

### 30.4 Réglages ajoutés
S-2709 (délai de réponse à une demande de réunion : 15 jours), S-2710 (seuil de lecture massive : 30 dossiers par heure).

### 30.5 Lots révisés
**IN0** ajoute : test « secrétariat sans liste », test « IA sans 35 », garde de lecture massive (R-INC34, journalisation seulement) ; **IN3** : R-INC31, R-INC37 ; **IN4** : R-INC32, R-INC36 ; **IN6** : R-INC33 ; **IN9** : R-INC35. Migrations de la passe : **2713** (`ent_inc_delegation`, `ent_inc_plage_indisponibilite`), **2714** (`ent_inc_export_session` journal des exports), 2715–2717 réserve.

**Précisions sur des actions existantes** : INC-123 permet la **signature en série** des PAP de rentrée (chaque dossier ouvert au moins une fois, confirmation par dossier) ; INC-304 notifie **chaque titulaire** de la création d'une délégation (`PERSONAS_35.md` P15) ; R-INC37 : la portée « séance » **exclut les permanences et études**, où seules « aide à l'évacuation » et « accompagnant présent » sont servies (P14) ; R-INC10 : l'AESH n'a pas `dossier.lire` au-delà de l'identité, de la photo et de la classe (contre `06_PERSONNELS.md` §775, Q-INC24) ; la direction ne lit un compte rendu d'ESS que si elle y participe (Q-INC25, changement d'usage signalé).

**Compte final** : **151 actions** (119 + 32), **48 cas limites**, **37 règles**, **11 réglages**, **13 lots**, **26 questions** (Q-INC1 à Q-INC22, puis Q-INC23 à Q-INC26 au `PERSONAS_35.md` §6).
