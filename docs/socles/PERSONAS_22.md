# Passe « personas » sur le socle 22 — Compétences et livrets

**Date :** 2026-11-11 — **Statut :** proposition, **non décidée** (à relire par Florian) ; les recommandations des bloquants s'appliquent par défaut (D-031). Aucun code écrit ni modifié ; aucun accès à une base, à un secret ni à un service de production ; l'application Next.js n'a pas été ouverte (aucun module de compétences en usage).
**Branche :** `ent-socle-22-23-conception` (base `origin/ent-mc0hoe` `7467f2d`).
**Éprouve :** `docs/socles/22_COMPETENCES.md` (relu en entier), avec `21_EVALUATIONS.md` (§6.1, §9.4, §10, §14, §18, §29), `23_BULLETINS.md` (séance, publication), `DECISIONS.md` (D-022, D-023, D-031, D-032, D-034, D-036) ; forme reprise de `PERSONAS_21.md`.
**Ajouts :** faits **directement** dans `22_COMPETENCES.md`, au **§28 « ajout personas »** (et une ligne au §0) ; rien n'est supprimé ; quand un ajout contredit le texte d'origine, le §28 l'emporte et la contradiction est écrite au §28.8.
**Numérotation (préfixée par le socle, pour éviter toute collision avec les sessions parallèles du 2026-11-11) :** manques **M22-001 à M22-078** ; actions **CMP-240 à CMP-266** ; règles **R-C21 à R-C30** ; cas limites **CC-42 à CC-55** ; questions **Q-CP13 à Q-CP19** ; réglages **S22-09 à S22-12** ; points pour Florian **F22-6 à F22-8**.

---

## 0. À lire en premier : les trois bloquants et leur recommandation (appliquée par défaut, D-031)

| # | Point | Pourquoi c'est bloquant | Recommandation (appliquée par défaut, D-031) | Lots |
|---|---|---|---|---|
| **A** | **Le droit `recevoir_bulletins` n'existe pas encore dans le code** (`21_EVALUATIONS.md` §29 point 3 : le lot V0 a dû se replier sur `peut_voir_notes`) ; or R-C7 fait dépendre synthèses, bilan de fin de cycle et attestations de ce droit (P11, M22-051) | Sans lui, soit le bilan de fin de cycle part aux seuls liens `peut_voir_notes` — et **le parent sans exercice** (destinataire par défaut des choix importants, C. civ. 373-2-1) ne le reçoit pas —, soit on ouvre trop | **C6 ne publie rien à une famille tant que le lot A14 du socle 4 n'a pas livré `recevoir_bulletins`** ; d'ici là, repli identique au V0 (`peut_voir_notes`) **et** tâche automatique au secrétariat pour chaque lien sans `peut_voir_notes` qui aurait dû recevoir (courrier ou remise tracée). Règle R-C21 | C6 dépend de socle 4 A14 |
| **B** | **Une observation hors évaluation peut devenir un jugement de comportement** : les éléments du domaine 3 (« formation de la personne et du citoyen ») positionnés au fil de l'eau par observation, sans évaluation ni contexte (P16, P15, M22-064) | Risque de **profilage** d'un mineur sur son attitude (règlement IA art. 6 § 3 : le profilage reste à haut risque ; minimisation RGPD) ; tentation de fonder un niveau sur une sanction | (1) **aucune observation ne se fonde sur un fait disciplinaire** du socle 13 (aucun lien, aucune lecture) ; (2) **pas de texte libre** sur une observation (le schéma n'en a pas : confirmé) ; (3) les éléments du domaine 3 se positionnent **sur une activité** (projet, engagement, oral, travail de groupe) nommée dans l'observation ; (4) purge à fin de cycle + 1 an (déjà au §15.4). Règles R-C22, R-C23 | C2, C6 |
| **C** | **Le bilan de fin de cycle d'un élève suivi par deux équipes** (élève d'ULIS inclus en 3ᵉ ; élève de SEGPA ; élève changé de classe en mai) n'a pas de « PP » unique : la proposition (CMP-163) et l'arrêt en conseil (CMP-165) supposent **une** classe et **un** PP (P08, M22-031) | Le bilan ne peut être ni proposé ni arrêté proprement ; l'élève risque de n'avoir aucun bilan, ou deux | Le bilan est proposé et arrêté par la **classe de rattachement à la date du conseil** (comme R-V28 du 21) ; le **coordonnateur de dispositif** est **co-rédacteur de droit** (dépôt d'avis et proposition), invité au point de l'élève (BUL-127) ; un changement de classe après l'ouverture transfère le bilan **avec ses avis**. Règle R-C24 | C4, C7 |

**Trois décisions importantes** (ne changent pas le schéma mais doivent être tranchées avant C4 et C6) : visa du bilan par **chaque** titulaire ou par un seul (M22-052, Q-CP13 : un suffit, chacun peut viser) ; Pix et compétences du LSL au lycée pilote **en doublon** l'an 1 (M22-040, Q-CP15) ; arrêt des **référentiels modèles** fournis par ScolIA (F22-4, M22-012).

---

## 1. Méthode

Celle de `PERSONAS_21.md` §1 : dix-sept personas réalistes, collège et lycée, public et privé sous contrat, parcours ordinaires et particuliers, **dont un attaquant et un auditeur RGPD**. Chacune vit son année (rentrée, fil des périodes, fin de cycle, LSU, archives) et ses incidents ; chaque fois qu'elle ne peut pas faire ce qu'elle veut, ou que c'est long, ambigu, risqué ou frustrant, le manque est noté. Grille transversale : qui saisit, qui valide, qui voit (D-022, liens du socle 4) ; ce qu'un libellé révèle ; ce que change un événement de mi-période ; ce que l'IA ajoute **sans juger** ; ce que Charlemagne et Pronote font déjà [H].
**Gravité** : **bloquant** (contournement dangereux, cas réel impossible, conception contradictoire) ; **important** (cas fréquent sans solution propre, perte de temps forte, parité du marché) ; **confort** ; **différenciant**.
**Réglementaire** : les points vérifiés le 2026-11-11 (§5 de la conception, R-C01 à R-C16) suffisent ; aucune nouvelle recherche n'a changé un manque.

---

## 2. Les personas en un tableau

| # | Persona | Contexte | Ce qu'elle éprouve surtout |
|---|---|---|---|
| P01 | Mme Morel, professeure de mathématiques | collège public, 6ᵉ « sans notes », 4 classes | positionner sans double saisie ; synthèse |
| P02 | M. Diallo, professeur de français | collège, 3ᵉ (notes obligatoires) et 5ᵉ (les deux) | notes + éléments ; bilan de fin de cycle |
| P03 | Mme Klein, professeure principale de 3ᵉ | 28 élèves, deux arrivants dont un en janvier de l'étranger | dossier d'appui, proposition, arrêt |
| P04 | M. Roche, coordonnateur de sciences | conseil pédagogique, nouveaux programmes | référentiel, versions, correspondance |
| P05 | Mme Duval, cheffe d'établissement | collège public de 600 élèves | visa en lot, LSU, pilotage |
| P06 | Florian, secrétariat de scolarité | lycée privé sous contrat (pilote), Charlemagne l'an 1 | Pix, LSL, attestations, doublon |
| P07 | M. Garnier, professeur d'EPS | collège ; natation en 6ᵉ | ASSN, dispenses, composante « arts et corps » |
| P08 | Mme Petit, coordonnatrice ULIS | 12 élèves inclus dans 9 classes | éléments adaptés, double équipe |
| P09 | Mme Nguyen, enseignante UPE2A | élèves allophones arrivés en cours d'année | non évaluable, CECRL |
| P10 | M. Bernard, professeur de lycée professionnel | bac pro, PFMP, tuteurs en entreprise | référentiel de diplôme, validation |
| P11 | Famille Martin | parents séparés ; père sans `voir_notes`, mère titulaire ; une grand-mère tiers digne de confiance pour le cadet | droits du lien, visa |
| P12 | Léo, 11 ans, 6ᵉ | — | comprendre, auto-positionnement |
| P13 | Sarah, 18 ans, terminale | lycée pilote | Pix, CECRL, LSL |
| P14 | Mme Fontaine, professeure documentaliste | référente numérique | Pix, éducation aux médias |
| P15 | Kevin, 14 ans, 4ᵉ — **attaquant** | élève curieux et malveillant ; son grand frère majeur a un compte | fuites, usurpation, IA, rejeu, import |
| P16 | M. Lacroix, **DPO** de l'organisme gestionnaire (auditeur RGPD) | contrôle avant mise en service | base légale, minimisation, durées, journaux, IA |
| P17 | Thomas, 24 ans, ancien élève | demande son ASSR 2 pour le permis | duplicata |

---

## 3. Parcours et manques, persona par persona

### P01 — Mme Morel, mathématiques, 6ᵉ sans notes
Septembre : elle crée un devoir « mode positionnement » (21) et associe 4 éléments. Elle veut saisir au clavier élève par élève **et** voir l'énoncé de chaque élément sans ouvrir un panneau. En novembre, elle découvre que deux de ses quatre classes ont des arrêts différents (21 R-V29), sans problème. À l'arrêt, elle valide les synthèses en colonne, mais trois élèves arrivés tard ont « pas assez d'éléments ».
| Id | Manque | Gravité | Renvoi |
|---|---|---|---|
| M22-001 | Libellé court de l'élément visible en en-tête de colonne, libellé complet au survol, sans quitter la grille | confort | §20 |
| M22-002 | Saisie « même niveau pour toute la ligne » (un élève, tous les éléments de l'évaluation) et « pour toute la colonne » (tous les élèves, un élément) | important | CMP-123 |
| M22-003 | Pour un élève « pas assez d'éléments », proposer une **observation rapide** ciblée avant la clôture (liste des élèves concernés) | important | §7.3 |
| M22-004 | Réutiliser une **évaluation type** (mêmes éléments) d'une classe à l'autre et d'une année à l'autre | confort | CMP-120 |
| M22-005 | En classe sans notes, la famille voit des niveaux mais aucun repère de progression : afficher « en progrès / stable / en recul » **déterministe** par élément (comparaison des deux derniers positionnements), sans couleur seule | différenciant | §9.2 |

### P02 — M. Diallo, français, 3ᵉ et 5ᵉ
En 3ᵉ, les notes sont obligatoires ; il veut aussi positionner sans que cela ne compte dans la moyenne. En 5ᵉ (« les deux »), il hésite : la famille verra-t-elle la note **et** le niveau ? En juin, il dépose ses avis sur le bilan de fin de cycle de ses élèves de 3ᵉ.
| Id | Manque | Gravité | Renvoi |
|---|---|---|---|
| M22-006 | Dire clairement, à la création de l'évaluation, ce que la famille verra (note, niveaux, les deux) selon le mode du niveau | important | §9.1, CMP-111 |
| M22-007 | Proposer un niveau **par exercice** depuis le barème détaillé (21 §8.9) avec des seuils **du professeur**, mémorisés par évaluation type | important | CMP-121, CMP-124 |
| M22-008 | Avis sur une composante du bilan : pouvoir s'appuyer sur la **frise** de l'élève dans ses propres cours, sans voir les autres matières hors fenêtre | important | CMP-162 |
| M22-009 | Avertir quand un professeur positionne un élément d'**une autre discipline** que la sienne (erreur fréquente de rattachement) | confort | CMP-120 |

### P03 — Mme Klein, PP de 3ᵉ
Mai : 28 bilans à proposer. Deux arrivants : l'un a un LSU de son ancien collège (pièce PDF), l'autre arrive de l'étranger en janvier sans aucun bilan. La veille du conseil, trois composantes restent vides pour deux élèves.
| Id | Manque | Gravité | Renvoi |
|---|---|---|---|
| M22-010 | **Vue « bilan de classe »** : 28 élèves × 8 composantes, proposition déterministe et choix côte à côte, saisie au clavier | important | §20 |
| M22-011 | Lire la **pièce PDF** du LSU antérieur à côté du dossier d'appui (pas seulement « importé ») | important | CC-05, CMP-221 |
| M22-012 | Pour l'arrivant de l'étranger : liste des **composantes sans aucun élément d'appui** et demande ciblée aux collègues (« positionner l'élève sur la composante langues avant le 10 juin ») | important | CC-04 |
| M22-013 | **Synthèse du bilan** : proposition de **trame de phrases** (ses propres formulations, banque de phrases du 21 rangée par personne et établissement, R-V35) ; jamais un texte généré sur l'élève | confort | §8.2 |
| M22-014 | Rappel à J-7 du conseil des **composantes vides** par élève, adressé au PP et aux collègues concernés | important | §16 |
| M22-015 | Voir ce qui a **changé** dans un bilan entre la proposition et l'arrêt (qui a changé quoi en séance) | confort | CMP-165 |

### P04 — M. Roche, coordonnateur de sciences
Les programmes de cycle 4 évoluent à la rentrée 2027 [H]. Il veut une nouvelle version du référentiel de physique-chimie sans casser les positionnements des 4ᵉ et 3ᵉ commencés sous l'ancien.
| Id | Manque | Gravité | Renvoi |
|---|---|---|---|
| M22-016 | Une version de référentiel **par niveau d'entrée** : les 5ᵉ commencent la nouvelle, les 4ᵉ et 3ᵉ finissent l'ancienne (cohabitation de deux versions la même année) | **important** | R-C3, CC-14 |
| M22-017 | Rattacher un élément à **plusieurs** composantes (« communiquer un résultat » : langue française et méthodes) avec une **composante principale** | important | R-C2 |
| M22-018 | Partager un référentiel avec **plusieurs établissements** du même organisme (copie par geste, comme R-V35), jamais lecture croisée | confort | D-013 |
| M22-019 | Garder la **référence au programme officiel** (intitulé, cycle, numéro de repère) dans l'élément | confort | §6.1 |
| M22-020 | Limite douce de **taille de référentiel** par discipline et cycle (avertissement au-delà de 40 items) | confort | S22-02 |

### P05 — Mme Duval, cheffe d'établissement
Juin : 180 bilans de 3ᵉ à viser, LSU dans trois semaines. Elle veut viser en lot, mais seulement ceux qui ne posent pas de question.
| Id | Manque | Gravité | Renvoi |
|---|---|---|---|
| M22-021 | Visa en lot avec **filtres** (aucune composante non évaluable, aucune correction tardive, aucune demande d'explication ouverte) et liste des exclus | important | CMP-170 |
| M22-022 | **Signature scannée** du chef sur le document de bilan, et mention de l'adjoint quand il vise | confort | CMP-177 |
| M22-023 | Indicateurs de **cohérence d'équipe** anonymes (part de « très bonne maîtrise » par classe et composante), sans classement de professeurs | confort | CMP-228 |
| M22-024 | Calendrier LSU **paramétrable** (date de remontée académique) pour caler le pré-contrôle | important | CMP-222 |

### P06 — Florian, secrétariat du lycée pilote
L'an 1, Charlemagne produit le LSL et doit contenir Pix ; les élèves de terminale passent Pix en février. Les ASSR ne le concernent pas (lycée), mais des élèves de 2ᵈᵉ arrivent sans ASSR 2 et en ont besoin pour le permis.
| Id | Manque | Gravité | Renvoi |
|---|---|---|---|
| M22-025 | Pix en **doublon** l'an 1 : saisie dans ScolIA **et** report dans Charlemagne par le **journal à reporter** (R-X24) | important | §10, Q-CP15 |
| M22-026 | Import du **relevé Pix** par fichier de la plateforme (format [NV]) avec rapprochement INE | important | CMP-181 |
| M22-027 | Attestation **ASSR 2 manquante** chez un élève de lycée : orienter vers la voie hors scolaire (texte d'information prêt, sans conseil juridique) | confort | CMP-184 |
| M22-028 | Pré-contrôle **LSL compétences** même l'an 1 (aide au report vers Charlemagne) | confort | C8a |
| M22-029 | Attestations des **arrivants** : importer depuis le dossier d'admission (socle 3) les pièces fournies | important | CMP-186 |

### P07 — M. Garnier, EPS
Six séances de natation en 6ᵉ ; certains élèves sont dispensés (certificat), d'autres absents au test.
| Id | Manque | Gravité | Renvoi |
|---|---|---|---|
| M22-030 | ASSN : saisie **« au bord du bassin »** sur mobile, hors ligne, par liste | important | CMP-180, C10 |
| M22-031 | Distinguer **dispensé** (inapte, socle 34) et **non passé** (absent) pour une attestation ; relance automatique « à repasser » | important | CC-39 |
| M22-032 | Composante « langages des arts et du corps » d'un élève **dispensé toute l'année** d'EPS : l'écran du bilan le signale sans révéler le motif | important | CC-18 |

### P08 — Mme Petit, coordonnatrice ULIS
Douze élèves, neuf classes de référence. Elle positionne sur des éléments adaptés et veut participer au bilan de fin de cycle de ses deux élèves de 3ᵉ.
| Id | Manque | Gravité | Renvoi |
|---|---|---|---|
| M22-033 | **Bloquant C** : bilan d'un élève suivi par deux équipes (classe de référence + dispositif) | **bloquant** | §0 C, R-C24 |
| M22-034 | Vue « mes élèves » transversale à toutes leurs classes, pour tous leurs enseignements, sans voir les autres élèves des classes | important | §15.2 |
| M22-035 | Élément adapté **rattaché à un élément ordinaire** (même attendu, palier intermédiaire) pour que la synthèse de l'enseignement reste lisible | important | §12.3 |
| M22-036 | Le bulletin et le LSU ne disent **jamais** « adapté » à un tiers ; au bulletin famille, la mention reste celle du dispositif (35) | important | R-C19 |

### P09 — Mme Nguyen, UPE2A
Des élèves arrivent tout au long de l'année, sans scolarité antérieure en français.
| Id | Manque | Gravité | Renvoi |
|---|---|---|---|
| M22-037 | Positionnement **CECRL en français langue seconde** par activité langagière, dès l'arrivée, avec un **test d'entrée** daté | important | §12.3 |
| M22-038 | « Non évaluable » **daté et levé** automatiquement à la fin de la première année, avec rappel au professeur | important | CC-17 |
| M22-039 | Bilan périodique de l'élève allophone : rubrique « parcours UPE2A » au bulletin famille **seulement** | important | R-C19 |

### P10 — M. Bernard, lycée professionnel
Bac pro ; le référentiel du diplôme a changé en 2026 (arrêté du 06/05/2026 [V R-C12]) ; ses élèves partent en PFMP.
| Id | Manque | Gravité | Renvoi |
|---|---|---|---|
| M22-040 | **Version de référentiel par session d'examen** et par cohorte (les terminales gardent l'ancien) | important | CMP-106 |
| M22-041 | Grille de positionnement **partagée au tuteur** par lien nominatif, propositions **reçues** puis validées (socle 39) ; l'auteur affiché « tuteur, validé par M. Bernard » | important | CMP-135 |
| M22-042 | Lien **positionnements → situation de CCF** (socle 38) : liste des compétences « prêtes » par élève, jamais une note | confort | §12.2 |
| M22-043 | Attestations **à durée de validité** (SST, habilitation électrique) avec alerte avant les PFMP qui l'exigent | important | CMP-185 |

### P11 — Famille Martin
La mère est titulaire avec tous les droits ; le père, séparé, a l'autorité parentale mais la direction lui a retiré `voir_notes` au vu d'une décision ; il doit néanmoins recevoir les **bulletins**. Le cadet vit chez sa grand-mère, tiers digne de confiance.
| Id | Manque | Gravité | Renvoi |
|---|---|---|---|
| M22-051 | **Bloquant A** : `recevoir_bulletins` absent du code | **bloquant** | §0 A, R-C21 |
| M22-052 | Visa du bilan : **chacun** des titulaires peut viser ; le bilan est réputé visé dès le premier ; l'écran du personnel montre qui a visé, **jamais** l'écran de l'autre parent (R-V46) | important | CMP-172, Q-CP13 |
| M22-053 | Tiers digne de confiance : reçoit et vise comme le titulaire selon la décision (socle 4) | important | CC-33 |
| M22-054 | Le père sans `voir_notes` ne doit **pas** voir la frise des positionnements dans le bulletin en ligne (elle relève de `voir_notes`) : la vue « bilan » sépare ce qui suit chaque droit | important | R-C7 |
| M22-055 | Une explication demandée par un parent n'est jamais visible de l'autre | important | CMP-205, R-V46 |

### P12 — Léo, 6ᵉ
| Id | Manque | Gravité | Renvoi |
|---|---|---|---|
| M22-056 | Libellés famille **écrits pour un enfant de 11 ans** (vocabulaire, longueur) : contrôle de longueur et de lisibilité à la saisie du libellé | confort | CMP-104 |
| M22-057 | « Ce qu'il me reste à travailler » relié aux **exercices** du cahier de textes (socle 20) | différenciant | CMP-206 |
| M22-058 | Auto-positionnement **avant** l'évaluation (« je pense savoir ») puis **après** (comparaison personnelle) | confort | CMP-133 |

### P13 — Sarah, terminale, 18 ans
| Id | Manque | Gravité | Renvoi |
|---|---|---|---|
| M22-059 | Voir son **niveau CECRL** par activité langagière tel qu'il partira au LSL, avant le visa | important | §12.1 |
| M22-060 | Majeure : décider elle-même si ses parents voient positionnements et attestations (socle 4) | important | R-V14 |
| M22-061 | Télécharger une **attestation de niveau** de langue interne (document de l'établissement, jamais présenté comme certification officielle) | confort | §11 |

### P14 — Mme Fontaine, professeure documentaliste
| Id | Manque | Gravité | Renvoi |
|---|---|---|---|
| M22-062 | Positionner sur l'**éducation aux médias** (éléments rattachés au domaine 2 ou 3) sans être professeure d'un cours de la classe : **mission** de référente (socle 6) donnant la portée sur les élèves de ses séances | important | §15.1 |
| M22-063 | Suivre l'**avancement Pix** (inscrits, passés, certifiés) par classe sans voir les scores détaillés hors de sa mission | confort | CMP-184 |

### P15 — Kevin, 14 ans, 4ᵉ — attaquant
Il essaie : (1) de lire les positionnements d'un camarade en changeant l'identifiant dans l'URL de l'API ; (2) de faire croire qu'un auto-positionnement est officiel (capture d'écran) ; (3) de demander à l'assistant « explique le bilan de Léa » et « quels élèves sont fragiles en maths ? » ; (4) d'utiliser le compte de son frère majeur pour se faire passer pour un parent et viser son bilan ; (5) de rejouer une requête de visa ; (6) d'injecter une formule dans un commentaire d'auto-positionnement exporté ; (7) de dénigrer un camarade dans un auto-positionnement de groupe.
| Id | Manque | Gravité | Renvoi |
|---|---|---|---|
| M22-064 | **Bloquant B** : observations et domaine 3 | **bloquant** | §0 B, R-C22, R-C23 |
| M22-065 | Toute lecture par identifiant vérifie **l'élève de la session** (élève) ou **le lien** (famille) et répond 404 sans distinguer « inexistant » de « interdit » | important | §15.3 |
| M22-066 | Auto-positionnement **marqué** « auto-positionnement de l'élève » partout (y compris en filigrane d'export), jamais dans le même style que l'officiel | important | §9.1 |
| M22-067 | L'assistant refuse tout élève autre que soi et toute liste d'élèves pour un élève ou une famille (tests de refus §14.3 étendus) | important | §14.3 |
| M22-068 | **Visa** : seul un **lien titulaire** (ou tiers selon décision) peut viser ; un frère majeur sans lien ne peut rien ; jeton anti-rejeu (idempotence par bilan × personne) | important | CMP-172 |
| M22-069 | Aucun **texte libre** d'élève dans l'auto-positionnement (choix d'un niveau seulement) : supprime dénigrement et injection | important | CMP-133, R-C25 |
| M22-070 | Échappement des cellules dans **tout** export de travail (aide à la saisie LSU, import de référentiel) — déjà prévu, à tester avec des cas d'attaque | important | CMP-223 |

### P16 — M. Lacroix, DPO (auditeur RGPD)
Il demande : base légale, finalités, minimisation, durées, droits, sous-traitants, IA, AIPD.
| Id | Manque | Gravité | Renvoi |
|---|---|---|---|
| M22-071 | **Fiche de traitement** du socle (registre, socle 27) : finalité « suivi des acquis et livret scolaire », base légale **mission d'intérêt public** / obligation légale (D. 111-3, arrêté du 31/12/2015) [H juridique, à valider DPO], catégories, destinataires, durées | important | §15.4 |
| M22-072 | **AIPD** : les propositions déterministes de synthèse et les contrôles de cohérence évaluent des mineurs → à inclure dans l'AIPD du socle 21 (§13.8 du 21) | important | §14 |
| M22-073 | **Purge** effective à fin de cycle + 1 an des positionnements, observations, auto-positionnements, avis internes sur bilan, avec rapport ; **jamais** dans les sauvegardes au-delà de leur rétention (D-043) | important | §15.4 |
| M22-074 | **Droit d'accès** : export de toutes les données de compétences d'un élève, **avis internes compris** (comme R-V41 du 21 pour la note privée) — l'écran de saisie de l'avis le dit | important | CMP-230 |
| M22-075 | Journal des **consultations** de bilans hors de ses cours par le personnel, consultable par le DPO | confort | §15.3 |
| M22-076 | Aucune donnée de compétences dans les **journaux applicatifs** ni les noms d'objets (E14) : test automatique | confort | §21 |

### P17 — Thomas, ancien élève
| Id | Manque | Gravité | Renvoi |
|---|---|---|---|
| M22-077 | Demande de **duplicata d'attestation** par un ancien élève sans compte : formulaire public minimal (identité, date de naissance, année), vérification par le secrétariat, réponse par courrier ou lien nominatif | important | CMP-189 |
| M22-078 | Archives des attestations **après fusion ou fermeture** d'établissement (reprise par l'organisme) | confort | §15.4 |

### Manques transverses (IA, droits, parité)
| Id | Manque | Gravité |
|---|---|---|
| M22-044 | Libellés famille **traduits** (familles allophones), marqués « traduction automatique » | confort |
| M22-045 | **Accessibilité** : niveaux en mots, contraste, lecteur d'écran sur la frise | important |
| M22-046 | Parité Pronote [H] : « évaluation par compétences » dans le devoir, bilan de fin de cycle proposé, items « éléments signifiants » : couverts (CMP-120, §7.4) | — |
| M22-047 | Parité Charlemagne [H] : référentiels, LSU officiel : LSU officiel bloqué par D-2 (C8b) | important |
| M22-048 | Délai de **grâce en lecture** pour un professeur parti (socle 6, P37) jusqu'au conseil | confort |
| M22-049 | Statistiques d'usage des éléments **sans nominatif** pour le CDD (aider à alléger) | confort |
| M22-050 | **Mode démonstration** : référentiel modèle et données fictives pour la démo de décembre, jamais en production (CD5) | confort |

---

## 4. Couverture du marché [H]

| Fonction | Charlemagne | Pronote | ScolIA (après la passe) |
|---|---|---|---|
| Référentiels du socle, items disciplinaires | oui (module vérifié, `MATRICE_CHARLEMAGNE.md` NOT-08) | oui | oui, versionnés, libellés famille |
| Positionnement dans le devoir | [H] | oui | oui, même grille que les notes |
| Synthèse de période proposée | [H] | oui (calcul) | proposée **et expliquée**, validée |
| Bilan de fin de cycle | oui | oui | dossier d'appui de trois ans, arrêt en séance, trois visas |
| LSU officiel | oui | oui | **après D-2** ; pré-contrôle avant (trou T-28) |
| Attestations | [H] | [H] | oui, documents, duplicata 50 ans |
| Lycée professionnel, PFMP | [H] | partiel | tuteur → validation (39) |

---

## 5. Manques qui relèvent d'autres socles

| Manque | Socle |
|---|---|
| `recevoir_bulletins` (bloquant A) | 4 (lot A14) |
| Mission de référent numérique, de coordonnateur, de référent sécurité routière | 6 |
| Grille partagée au tuteur, lien nominatif | 39, 7 |
| Situations de CCF | 38 |
| Fiche de traitement, AIPD, purge | 27 |
| Exercices reliés à « ce qu'il me reste à travailler » | 20 |
| Formulaire public de duplicata | 7 |

---

## 6. Synthèse

### 6.1 Manques par gravité
- **Bloquants : 3** (M22-033, M22-051, M22-064), tranchés au §0 (appliqué par défaut, D-031).
- **Importants : 48** ; **confort : 24** ; **différenciants : 2** (M22-005, M22-057) ; **parité constatée : 1** (M22-046). Total : 78 identifiants, M22-001 à M22-078.

### 6.2 Ajouts dans `22_COMPETENCES.md` (§28)
Actions CMP-240 à CMP-266 ; règles R-C21 à R-C30 ; cas limites CC-42 à CC-55 ; questions Q-CP13 à Q-CP19 ; réglages S22-09 à S22-12 ; compléments de modèle (réserve 2451–2456) ; lots C2, C4, C6, C7, C9 complétés ; points F22-6 à F22-8.

### 6.3 Ce que la passe a montré
La conception tient sur l'essentiel (un seul modèle, propositions expliquées, rien sans publication, IA tenue à l'écart du jugement). Elle était trop **« une classe, un PP »** (bloquant C), dépendait d'un droit **pas encore codé** (bloquant A) et laissait une porte au **jugement de comportement** (bloquant B). Les autres manques sont surtout de la vitesse de saisie (M22-002, M22-010) et des versions de référentiel qui cohabitent (M22-016, M22-040).

---

## 7. Faits, hypothèses, contradictions

**Faits [F]** : `recevoir_bulletins` absent du code (`21_EVALUATIONS.md` §29 point 3, confirmé par l'audit du 2026-11-11 qui ne le trouve pas dans `actions.rs`).
**Hypothèses [H]** : nouveaux programmes de cycle 4 en 2027 (P04) ; format du relevé Pix ; parité Pronote et Charlemagne.
**Contradictions** :
- **C1** R-C7 (synthèses et bilans par `recevoir_bulletins`) contre l'état du code : **tranché** par R-C21 (repli et tâche au secrétariat).
- **C2** §6.1 (« un élément est rattaché à **une** composante », R-C2) contre M22-017 (rattachement multiple) : **tranché** par R-C26 (une composante **principale**, d'autres secondaires, la synthèse du bilan ne lit que la principale).
- **C3** CMP-163 (« le PP renseigne ») contre les élèves à deux équipes : **tranché** par R-C24.
