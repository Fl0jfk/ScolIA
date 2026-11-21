# Passe « personas » sur le socle 15 — Absences des personnels et remplacements

**Date :** 2026-11-11 — **Statut :** proposition, **non décidée** (à relire par Florian) ; les recommandations s'appliquent par défaut (D-031), sauf ce qui engage l'établissement à l'extérieur (§8.2). Aucun code écrit ni modifié ; aucun accès à une base, à un secret ni à un service de production ; le code Next.js n'a été que **lu** (enquête de la conception).
**Branche :** `ent-socle-15-conception` (même branche que la conception).
**Éprouve :** `docs/socles/15_ABSENCES_PERSONNELS.md` (lu en entier), avec `CLAUDE.md`, D-020, D-022, D-028, D-031, D-034, D-035, D-036 ; forme reprise de `PERSONAS_10.md` et `PERSONAS_42.md`.
**Ajouts :** faits **dans la conception**, au **§32** « Ajout personas » (plus des renvois ponctuels). **Rien n'est supprimé** ; en cas de contradiction, le §32 de la conception l'emporte.
**Numérotation :** manques **M-1501 à M-1562** ; actions **ABP-272 à ABP-299** ; règles **R-AP45 à R-AP59** ; cas limites **CAP-73 à CAP-90** ; questions **Q-AP37 à Q-AP44** ; réglages **S-1518 à S-1521**.

---

## 0. À lire en premier : quatre bloquants et leur recommandation (appliquée par défaut, D-031)

La conception tient sur le fond : nature catégorisée au lieu du motif, trois niveaux (fait RH, impact, réponse), circuit par engagement, heures constatées et non déclarées. En la faisant vivre par seize personas, quatre bloquants apparaissent. Deux d'entre eux montrent que des conséquences **vers l'extérieur** (familles, sorties d'élèves) partent sur la foi de données qui ne sont pas encore sûres.

| # | Point | Pourquoi c'est bloquant | Recommandation (appliquée par défaut, D-031) | Lots |
|---|---|---|---|---|
| **A** | **L'impact et l'information des familles reposent sur l'emploi du temps de ScolIA, qui ne fait pas encore foi pendant le doublon** (P02, P09, P16). En 2027-2028, l'EDT officiel du lycée pilote peut rester celui de Charlemagne (Q-EDT19) ; celui de ScolIA est une copie, peut-être en retard d'une modification. | Une famille prévenue d'un « cours annulé » qui a en réalité lieu (ou l'inverse), ou une **sortie anticipée** calculée sur une séance qui n'existe plus : c'est précisément l'erreur que la circulaire 96-248 interdit (R-13 de la conception). | **Interrupteur « EDT de référence »** par établissement (R-AP45) : tant qu'il n'est pas posé par la direction, le socle 15 calcule l'impact et organise **en interne**, mais **aucune** notification aux familles ni aux élèves, **aucune** proposition de sortie anticipée. L'interrupteur se pose une fois, avec la date et l'auteur, après un contrôle « EDT à jour » du socle 10 (ABP-272). | A3, A4 |
| **B** | **Un appel anonyme à l'accueil peut annuler un cours** (P11, P15). « Bonjour, ici M. Roux, je suis malade » : l'accueil saisit une absence `signalee`, l'impact est calculé, les propositions s'appliquent, les familles sont prévenues, les externes sortent. Un élève qui imite un professeur obtient sa matinée. | Effet externe d'une donnée non authentifiée ; atteinte à la sécurité des mineurs (sortie). | **Signalement non confirmé = organisation interne seulement** (R-AP46) : tant que l'absence n'est confirmée ni par l'agent (son compte, sa passkey) ni par un rappel tracé de l'ADJ ou de la direction (ABP-273 : « j'ai rappelé M. Roux sur son numéro de la fiche »), rien ne part vers les familles ou les élèves et aucune sortie anticipée n'est proposée ; le remplacement interne peut être préparé. | A1, A4 |
| **C** | **Dans le public, l'établissement ne « valide » pas un congé de maladie** (P12, P13, P03). La conception reprend de Next.js une « validation par la direction » qui a un sens pour un salarié de l'OGEC, mais pour un fonctionnaire, un maître contractuel ou une **AESH** (employée par la DSDEN ou un employeur mutualisateur, coordonnée par le PIAL [V*]), la décision appartient à l'employeur extérieur ; l'établissement **constate**, **transmet** et **organise**. | Sinon, un chef d'établissement « refuse » une absence qu'il n'a pas le pouvoir de refuser, ou un état « validée » laisse croire que le rectorat a statué. | **Circuit externe** (R-AP47) : chaque nature et chaque employeur déclarent si la décision est **interne** (établissement, OGEC) ou **externe** (rectorat, DSDEN/PIAL) ; en externe, l'état `validee` est remplacé par `prise_en_compte` (établissement) puis, si l'information revient, `decision_employeur` (accordée, refusée, requalifiée, avec date et pièce) ; le refus n'est jamais proposé à l'établissement. | A1 |
| **D** | **Le port « présent / absent » permet à n'importe quel professeur de surveiller l'absentéisme de ses collègues** (P15, P14). La conception l'ouvre aux « personnels » (ABP-155) : en interrogeant chaque matin tous les agents, on reconstitue la liste des absents, leurs dates, leur fréquence. | Contredit R-AP38 (pas d'indicateur nominatif) et la recommandation CNIL d'accès limité (R-14) ; c'est une surveillance de fait. | **Présence à qui en a besoin** (R-AP48) : l'accueil et la vie scolaire voient tout le personnel ; un professeur voit seulement les agents avec qui il partage une séance, une équipe (socle 6) ou un rendez-vous ce jour-là ; limitation de débit et journal des consultations hors accueil ; jamais d'historique. | A0, A1 |

**Défaut de conception à corriger aussitôt** (sans être bloquant) : la conception affiche par défaut aux familles le **compte des séances non assurées par matière** (S-1510). Pour une matière à un seul professeur, c'est **le compte d'absences d'un enseignant nommé**. Recommandation appliquée : **désactivé par défaut**, et quand il est activé, compté par période et sur l'ensemble des matières, jamais par matière seule (R-AP49, M-1541).

---

## 1. Méthode

**Démarche.** Celle de `PERSONAS_10.md` §1 et `PERSONAS_42.md` §1 : seize personas réalistes, prises dans le tableau des acteurs (§3 de la conception), couvrant le **privé sous contrat** (lycée pilote : maîtres payés par l'État, salariés de l'OGEC) et le **public** (collège d'EPLE), les familles ordinaires et séparées, un attaquant à plusieurs visages, un auditeur RGPD, et un persona de bascule (Florian administrateur). Pour chacune : sa **journée** ou son **année**, ses **irritants**, **peurs** et ce qui la **ravirait** ; les **manques** et **contradictions** ; les **cas limites** ; les exigences nouvelles.

**Grille transversale** posée à chaque persona : (1) **parité** — ce qu'elle fait aujourd'hui dans Next.js M05, le retrouve-t-elle ? (2) **qui voit quoi** — la nature de l'absence ou un indice qui la révèle sort-il du cercle RH ? (3) **effet extérieur** — quelle donnée non sûre peut atteindre une famille, un élève, la paie, le rectorat ? (4) **le temps** — ce qui change à une date, en fin de mois, à la bascule ; (5) ce que l'**IA** prépare sans décider.

**Vérification réglementaire complémentaire (2026-11-11).** Deux recherches : circuit des absences des **AESH** (guides académiques de Normandie, de Lyon, de Bordeaux et syndicaux : l'AESH prévient l'établissement, adresse son arrêt à **l'employeur — DSDEN ou employeur mutualisateur** —, qui renvoie sa décision à l'AESH, à l'établissement et au **coordonnateur du PIAL** [V*, pas de texte national unique]) ; **analyse d'impact** : la liste CNIL des traitements soumis à AIPD (délibération n° 2018-327 du 11/10/2018) vise notamment les traitements RH impliquant une **surveillance individualisée** des salariés et les données sensibles de personnes vulnérables, dont des employés ; la liste des dispenses vise le seul **contrôle du temps de travail** [V* : résumés et copie de la liste ; texte intégral non relu].

**Gravité** : **bloquant** · **important** · **confort** · **différenciant**.

**Conventions.** [V] source officielle ; [V*] copie ou résumé d'une source officielle ; [F] fait de code ; [H] hypothèse ; [R] recommandation.

---

## 2. Les personas en un tableau

| N° | Persona | Établissement | Rôle | Angle |
|---|---|---|---|---|
| P01 | Mme Claire Martin, certifiée de mathématiques, deux enfants | lycée privé sous contrat (pilote) | professeure absente | déclarer malade à 6 h 45, arrêt, temps partiel thérapeutique, retour |
| P02 | M. Thomas Leroy, directeur adjoint | lycée pilote | organise les remplacements | la matinée, échanges de cours, doublon |
| P03 | Mme Sophie Bernard, secrétaire de direction | lycée pilote | instructrice (maîtres, rectorat) | prolongations en chaîne, transmissions |
| P04 | Mme Nadia Haddad, comptable de l'OGEC | lycée pilote | paie des salariés | variables du mois, DSN, maintien de salaire |
| P05 | Mme Julie Petit, surveillante (salariée OGEC), mère seule | lycée pilote | absente (enfant malade) | compteur annuel, poste à couvrir |
| P06 | Mme Karima Benali, CPE | lycée pilote | vie scolaire | permanence, sorties, AED absents |
| P07 | M. Lucas Garnier, professeur d'histoire, volontaire RCD | collège public | remplaçant | créneaux, refus légitime, équité, heures |
| P08 | M. Hugo Faure, maître délégué suppléant | lycée pilote | suppléant trois mois | arrivée, propre absence, cascade |
| P09 | M. et Mme Moreau, séparés ; Inès, 6ᵉ, transport scolaire | collège public | famille | qui est prévenu, quand ; car scolaire |
| P10 | Yanis, 18 ans, terminale, interne | lycée pilote | élève majeur | où aller, qui prévient |
| P11 | M. Didier Roche, accueil | lycée pilote | accueil | appels du matin, canular, familles qui questionnent |
| P12 | Mme Anne Duval, principale | collège public | cheffe d'établissement | plan RCD, CA, remontée, grève |
| P13 | Mme Fatou Diallo, AESH (PIAL) | collège public | AESH absente | employeur extérieur, élève accompagné |
| P14 | M. Paul Blanc, délégué à la protection des données | organisme | auditeur RGPD | minimisation, inférences, conservation, AIPD |
| P15 | Les attaquants | — | — | curieux, collusion, canular, injection, usurpation |
| P16 | Florian, administrateur | lycée pilote | bascule M05 → ScolIA | date, propriété, données, paie du mois |

---

## 3. Audit de parité avec Next.js (relecture de l'inventaire de la conception)

La conception reprend 58 fonctions (NX15-01 à NX15-58). La relecture par personas ajoute ou précise :

| Constat | Persona | Où | Suite |
|---|---|---|---|
| Next.js laisse **l'agent qui a saisi pour un collègue** suivre l'absence (prévu, mais perdu par le bug AD-07) : la conception le garde via `saisie_par_user_id`, mais ne dit pas **ce qu'il voit** | P03 | §21.1 | il voit l'organisation et l'étape, jamais la nature choisie ensuite par l'agent (R-AP50) |
| Le **suivi « prochaine étape »** de Next.js (`absences-suivi.ts`) est la fonction la plus utilisée par la secrétaire [H] : la conception le cite (ABP-172) sans écran | P03 | §25 | écran 4 enrichi : colonne « qui doit agir, depuis quand » (M-1513) |
| Next.js crée une absence **par jour** à l'import d'une convocation ; la conception une absence à plusieurs plages : la secrétaire perdrait la possibilité de valider un jour et pas un autre | P03 | NX15-05 | décision **par plage** possible pour les convocations (M-1514) |
| Les **trous dans le planning de l'agent** (`mon-planning`) existent dans Next.js | P01 | NX15-33 | « mon EDT » du socle 10 montre l'absence à l'agent lui-même (M-1503) |
| Aucun mail de **refus** dans Next.js : corrigé dans la conception (ABP-234) | P05 | — | — |

---

## 4. Parcours et manques, persona par persona

### P01 — Mme Claire Martin, professeure de mathématiques, malade un mardi

**Sa journée.** 6 h 40, fièvre. Elle ouvre l'appli : « Absente aujourd'hui », nature « Arrêt de travail ». Elle n'a pas encore vu le médecin ; elle prolongera peut-être. Elle voit ses quatre séances. « Déposer le travail » : elle n'a pas la force. Ensuite, toute la matinée, son téléphone vibre : la liste de ses séances qui se remplissent, les messages des collègues. Elle revient jeudi ; trois semaines plus tard, elle reprend à mi-temps thérapeutique pour deux mois. En février, sa fille est malade deux jours.
**Irritants.** Devoir prévoir du travail quand on est malade ; recevoir des notifications pendant l'arrêt ; ne pas savoir ce qui a été fait.
**Peurs.** Que les collègues ou les élèves sachent qu'elle est en mi-temps thérapeutique ; qu'on la croie « tire-au-flanc ».
**Ce qui la ravirait.** Une déclaration en trois touches ; le silence pendant l'arrêt ; au retour, « ce qui s'est passé » (ABP-159).

| N° | Manque | Gravité | Suite |
|---|---|---|---|
| M-1501 | **Silence pendant une absence**, même courte : aucune notification non urgente (relances, messages d'équipe, sollicitations) entre le début et la fin, sauf ce qui la concerne directement (décision, pièce demandée) ; résumé au retour | important | R-AP51 ; lien socle 8 (pause) et 16 (réponse d'absence) |
| M-1502 | « Déposer le travail » ne doit jamais être une **attente** : option « le remplaçant choisit, voici ma progression » (lien au cahier de textes et à la progression du service, socle 20) | important | ABP-274 |
| M-1503 | L'absence apparaît dans **son propre** emploi du temps (et seulement le sien) | confort | ABP-275 (socle 10) |
| M-1504 | **Temps partiel thérapeutique** : c'est une quotité temporaire au socle 6 (service réduit), pas une suite d'absences ; la conception le range en « absence discontinue » (CAP-06), ce qui crée une absence sensible ouverte deux mois et des « séances touchées » chaque semaine | important | R-AP52 : un temps partiel thérapeutique est une **modification datée de quotité** au socle 6 (ses séances sont replacées ou suppléées par le 10), jamais une absence ; l'absence ne sert qu'aux rendez-vous ponctuels |
| M-1505 | « Enfant malade » : la conception le fait apparaître dans les compteurs, mais une mère veut savoir **combien de jours il lui reste** dans l'année (règle de l'employeur ou de la convention) | important | compteur par nature (R-AP53, M-1520) |
| M-1506 | Elle veut **corriger elle-même** la nature avant décision (« ce n'était pas une maladie, c'était un rendez-vous médical » ) | confort | ABP-128 ouvert à l'agent avant décision |

### P02 — M. Thomas Leroy, directeur adjoint, 7 h 05

**Sa matinée.** Trois absents (Mme Martin, un professeur d'EPS en formation prévue, une surveillante). Huit séances à couvrir. Le tableau propose : deux remplacements par des volontaires, un autre cours, une continuité au CDI, quatre annulations. Mais ce qu'il fait le plus souvent, c'est un **échange** : M. Dumas, libre à 10 h, prend la classe de Mme Martin, et Mme Martin rendra à M. Dumas une heure le 4 décembre ; ou il **avance** le cours de 15 h de M. Petit à 10 h, puisque la classe est libre. Ses professeurs n'ont pas tous l'appli : il envoie des SMS. En 2027-2028, l'EDT officiel est encore celui de Charlemagne.
**Irritants.** Ressaisir dans deux outils ; les professeurs injoignables ; les familles qui appellent.
**Peurs.** Une classe sans adulte ; une famille prévenue à tort.
**Ce qui le ravirait.** Les échanges et les avancées proposés par le moteur ; tout valider en un geste ; voir qui a vu quoi.

| N° | Manque | Gravité | Suite |
|---|---|---|---|
| M-1507 | **EDT de référence** (bloquant A) | **bloquant** | R-AP45, ABP-272 |
| M-1508 | **Échange de séances** entre deux professeurs (« je prends ta classe aujourd'hui, tu prends la mienne le 4 ») : deux modifications liées du socle 10, la seconde à date future, avec accord des deux | important | ABP-276 ; type de réponse `echange` |
| M-1509 | **Avancer** un cours de la même classe (le professeur de 15 h vient à 10 h) : déplacement **dans la journée**, qui raccourcit la journée des élèves → information des familles si l'heure de sortie change | important | ABP-277 ; le moteur le propose (§11.2) |
| M-1510 | **Sollicitation par SMS** pour les professeurs sans appli (et en secours quand le push n'est pas lu en 5 min), lien d'acceptation à usage unique | important | ABP-278 ; coût SMS (D-028, Q-N22) |
| M-1511 | **Validation de la journée** : à 7 h 30, un seul geste « tout ce qui est proposé et coché part » — existe (ABP-168) ; manque l'**état final** visible à 8 h (qui a accusé réception) et le **journal de la matinée** | confort | écran 1 complété |
| M-1512 | Les **séances d'examen blanc**, de **devoir commun** : remplacer un surveillant, pas un enseignant ; candidats = tout personnel habilité à surveiller (AED compris) | important | type d'impact `surveillance` ; candidats élargis (R-AP54) |

### P03 — Mme Sophie Bernard, secrétaire de direction (maîtres du lycée)

**Son année.** Elle transmet les absences des maîtres au rectorat : arrêts initiaux, prolongations (parfois cinq de suite), passage en congé de longue maladie, reprise. Elle tient les états d'heures. Elle importe les convocations de jurys en juin.
**Irritants.** Les arrêts qui arrivent par mail, papier, photo ; les prolongations à rattacher ; le « qui doit agir ».
**Peurs.** Un délai de 48 h manqué sur une prolongation ; un arrêt perdu.

| N° | Manque | Gravité | Suite |
|---|---|---|---|
| M-1513 | **« Qui doit agir, depuis quand »** sur chaque ligne de sa file | important | écran 4 |
| M-1514 | Décision **par plage** pour une convocation de plusieurs jours | confort | ABP-124 étendu |
| M-1515 | **Une date de réception par arrêt**, pas par absence : chaque prolongation a sa pièce, sa date de réception et son échéance de 48 h ; la conception porte `arret_recu_le` sur l'absence seulement | important | modèle : `ent_absp_arret` (une ligne par arrêt initial ou de prolongation), §32.4 |
| M-1516 | **Bordereau de transmission** : grouper plusieurs absences dans un envoi daté au rectorat, avec accusé | important | ABP-279 |
| M-1517 | **Requalification par l'employeur** (le rectorat transforme un CMO en CLM rétroactivement) : à saisir comme décision externe, sans refaire l'organisation passée | important | bloquant C, R-AP47 |
| M-1518 | Ce que voit le **collègue qui a saisi** pour un autre (R-AP50) | important | règle |

### P04 — Mme Nadia Haddad, comptable de l'OGEC

**Son mois.** Les variables de paie partent le 20. Tout ce qui arrive après passe le mois suivant. Elle signale les arrêts en DSN sous 5 jours. Pour un arrêt maladie, la Sécurité sociale compte les **jours calendaires** (carence, indemnités journalières) ; la convention fixe le **maintien de salaire** selon l'ancienneté [H].
**Irritants.** Des absences validées après le 20 sans qu'elle le sache ; des libellés à recoder ; les doubles casquettes.
**Peurs.** Une DSN en retard ; une retenue indue.

| N° | Manque | Gravité | Suite |
|---|---|---|---|
| M-1519 | **Date de clôture des variables** du mois (réglage) : une absence connue après la clôture est marquée « mois suivant », jamais glissée en silence dans un mois déjà parti | important | S-1518, R-AP55 |
| M-1520 | **Compteurs annuels par nature** avec plafond (enfant malade, congés exceptionnels, congés payés lus au socle 49) : jours pris, restants, période de référence | important | R-AP53 ; `ent_absp_compteur` (§32.4) |
| M-1521 | **Jours calendaires** en plus des jours ouvrés et des heures pour un arrêt (carence, IJSS) | important | §14.4 complété |
| M-1522 | **Attestation de salaire / subrogation** : hors ScolIA (DSN, socle 49), mais elle veut savoir si l'OGEC **subroge** pour ce salarié | confort | champ au socle 49 ; rien au 15 |
| M-1523 | La comptabilité voit la partie OGEC d'un **double engagement** sans la partie État — confirmé (piège §23.4 n° 1) | — | test |

### P05 — Mme Julie Petit, surveillante (salariée de l'OGEC), mère seule

**Sa semaine.** Son fils a de la fièvre lundi : elle prévient à 7 h. Elle tient la cour de 10 h et la cantine de midi. En mars elle veut poser trois jours de congés payés pendant les vacances de printemps.
**Irritants.** Ne pas savoir si elle sera payée ; qu'on lui demande un certificat pour une demi-journée.
**Peurs.** Que ses collègues pensent qu'elle abuse ; que la cour soit sans surveillance.

| N° | Manque | Gravité | Suite |
|---|---|---|---|
| M-1524 | **Ce que deviendra sa paie** : afficher à l'agent, sans jargon, le traitement prévu (« maintenu », « à rattraper », « sans solde ») au moment de la décision | important | ABP-234 enrichi |
| M-1525 | **Pièce selon la durée** : la conception demande un certificat pour « enfant malade » toujours ; la règle de l'employeur peut en dispenser une courte absence | confort | `ent_absp_nature` : `piece_a_partir_de_jours` (§32.4) |
| M-1526 | **Poste couvert** : son poste de cour doit apparaître à la CPE avec les AED libres, comme les séances pour l'adjoint | important | A15, déjà prévu ; à **remonter** dans le parcours minimal de la rentrée 2027 (les surveillants de l'OGEC utilisent Next.js aujourd'hui) |
| M-1527 | Demande de **congé payé** pendant les vacances scolaires : aucun impact (pas de cours), mais un impact sur les postes de vacances (internat, accueil) s'il y en a | confort | CAP-80 |

### P06 — Mme Karima Benali, CPE

**Sa journée.** 8 h 05 : une classe de 2ⁿᵈᵉ dans le couloir ; personne ne l'a prévenue. 10 h : trente élèves en permanence, une salle de vingt places. 15 h : la dernière heure de la 1ʳᵉ B est annulée ; certains élèves sont externes autorisés, d'autres non, deux prennent le car de 17 h 30, et pour cinq élèves la dernière séance de la journée n'est **pas** celle-là (ils ont une option à 16 h).
**Irritants.** Les listes faites au crayon ; les élèves sortis à tort.
**Peurs.** Un mineur sorti sans autorisation.

| N° | Manque | Gravité | Suite |
|---|---|---|---|
| M-1528 | **La sortie anticipée se calcule par élève**, pas par classe : dernière séance **de l'élève** dans la demi-journée ou la journée (options, groupes) | important | R-AP56 (précise R-AP24) |
| M-1529 | **Élève qui prend un car scolaire** ou qui est **interne** : jamais proposé à la sortie (régime du socle 14) | important | CAP-75 |
| M-1530 | **Liste d'appel de permanence pré-remplie** avec les élèves des séances annulées de l'heure, salle et surveillant (socle 12, EDT-219) | important | lien explicite ABP-240 → appel de permanence |
| M-1531 | **Capacité des salles de permanence** : alerte quand l'effectif attendu dépasse la salle, proposition de deuxième salle ou du CDI | confort | ABP-280 |
| M-1532 | Elle **valide les absences des AED** (responsable fonctionnelle) : elle doit voir leur nature, et pas celle des professeurs — conforme à la conception (§23.2) | — | test |

### P07 — M. Lucas Garnier, professeur d'histoire, volontaire RCD (collège public)

**Son année.** Engagé pour 18 h de RCD ; créneau jeudi M4 et lundi M2. Il est sollicité trois fois en septembre, aucune en novembre. Un jeudi il a une réunion de parents : il voudrait rendre son créneau **cette semaine-là**.
**Irritants.** Être sollicité à 7 h 50 pour 8 h ; ne pas connaître les élèves ; ne pas voir ses heures.
**Peurs.** Être payé en retard, ou pas du tout ; être le « bouche-trou » attitré.

| N° | Manque | Gravité | Suite |
|---|---|---|---|
| M-1533 | **Indisponibilité ponctuelle** sur un créneau RCD (« pas ce jeudi », motif court) | important | ABP-281 (lu au 10 comme indisponibilité) |
| M-1534 | **Trombinoscope** de la classe dans « ma séance de remplacement » (photos des élèves, socle 2, si l'établissement les montre aux professeurs) | important | ABP-158 complété |
| M-1535 | **Équité visible** : sa propre position (« sollicité 3 fois, collègues : médiane 2 ») sans voir les autres nommément | confort | ABP-213 complété |
| M-1536 | **Délai de prévenance** affiché sur chaque sollicitation (« hors délai : vous pouvez refuser sans motif ») | important | R-AP36, écran 5 |
| M-1537 | Ses heures RCD rattachées à sa **mission Pacte** (volume consommé) ou en HSE selon son statut, visibles chaque mois | important | ABP-200 ; export |

### P08 — M. Hugo Faure, maître délégué, suppléant de Mme Lopez pour trois mois

**Son arrivée.** Nommé le jeudi, installé le lundi. En décembre, il est lui-même malade trois jours. En janvier, Mme Lopez revient deux semaines plus tôt.
| N° | Manque | Gravité | Suite |
|---|---|---|---|
| M-1538 | **Absence du suppléant** : impact sur les séances du service suppléé (il est en saisie) — conforme ; le circuit est celui de **son** engagement (État, maître délégué), pas celui du titulaire | important | test (CAP-76) |
| M-1539 | **Retour anticipé** de la titulaire : la fin de suppléance proposée (port du 6) **et** l'information du suppléant (sa fin de mission avance : conséquences sur son contrat, hors ScolIA) | important | ABP-282 |

### P09 — M. et Mme Moreau, séparés ; Inès, 6ᵉ, transport scolaire

**Leur matinée.** Inès part à 6 h 55 en car. Le cours de 8 h est annulé : la notification arrive à 7 h 12. La mère a la résidence ; le père a l'autorité parentale et veut être informé.
| N° | Manque | Gravité | Suite |
|---|---|---|---|
| M-1540 | **Heure utile d'information** : une annulation du premier cours décidée après le départ habituel de l'élève est **inutile** pour la famille ; la règle de la circulaire 96-248 (surveillance aux horaires habituels) s'applique : l'élève va en permanence ; le message dit « Inès est accueillie en permanence de 8 h à 9 h », pas « cours annulé » | important | R-AP57 : formulation selon que l'information arrive avant ou après l'heure de départ |
| M-1541 | Le **compte par matière** des séances non assurées révèle un professeur (défaut du §0) | important | R-AP49 |
| M-1542 | Les **deux parents** ayant droit reçoivent la même information au même moment (déjà socle 4) — confirmé | — | test |

### P10 — Yanis, 18 ans, terminale, interne

| N° | Manque | Gravité | Suite |
|---|---|---|---|
| M-1543 | **Élève majeur interne** : jamais de sortie anticipée de l'établissement (internat) mais liberté de CDI ou d'étude selon le règlement ; message « où aller » plutôt que « vous pouvez sortir » | confort | R-AP56 |
| M-1544 | Il veut savoir **la veille** au soir ce qui change le lendemain, en une notification | confort | résumé du soir (socle 10) |

### P11 — M. Didier Roche, accueil

**Sa matinée.** 7 h 20 : « Bonjour, ici M. Roux, je ne viendrai pas. » La voix est jeune. 7 h 40 : une mère : « Pourquoi Mme Martin est-elle absente depuis une semaine ? » 9 h : la cantinière appelle, elle est malade.
| N° | Manque | Gravité | Suite |
|---|---|---|---|
| M-1545 | **Canular** (bloquant B) | **bloquant** | R-AP46, ABP-273 |
| M-1546 | **Rappel au numéro de la fiche** proposé à l'accueil ou à l'ADJ (numéro professionnel ou personnel du socle 6, jamais affiché à l'accueil : bouton « appeler » qui compose sans montrer) | important | ABP-273 |
| M-1547 | **Phrase type** pour les familles qui demandent la raison (« nous ne communiquons pas les raisons d'une absence ») et pour la durée (« le remplacement est organisé ») | confort | écran accueil |
| M-1548 | L'absence de la **cantinière** (salariée OGEC, poste de service) se saisit de la même façon | — | conforme |

### P12 — Mme Anne Duval, principale de collège public

**Son année.** Plan RCD présenté au CA d'octobre ; compte rendu en février et en juin ; remontée chaque lundi ; grève en mars ; deux AESH absentes en janvier.
| N° | Manque | Gravité | Suite |
|---|---|---|---|
| M-1549 | **Circuit externe** des agents de l'État et des AESH (bloquant C) | **bloquant** | R-AP47 |
| M-1550 | **Trame du compte rendu au CA** : taux de remplacement, heures non assurées par discipline, modes de continuité, volontaires — prête à imprimer (D-023) | important | ABP-224 complété |
| M-1551 | **Remontée rejetée** par le service national : motif, correction, renvoi | important | ABP-283 |
| M-1552 | **Petit établissement** : la remontée par discipline × niveau × catégorie identifie un professeur (un seul enseignant d'allemand) ; obligation légale, mais à documenter au registre | confort | §5 R-02, note DPO |

### P13 — Mme Fatou Diallo, AESH du PIAL, absente deux jours

| N° | Manque | Gravité | Suite |
|---|---|---|---|
| M-1553 | Employeur **extérieur** (DSDEN ou employeur mutualisateur), coordonnateur du PIAL informé : l'établissement **prend en compte** et **informe**, il ne valide pas [V*] | **bloquant** (C) | R-AP47 |
| M-1554 | **Informer le coordonnateur du PIAL** (personne extérieure, sans compte) : export ou message tracé, sans nature | important | ABP-284 |
| M-1555 | AESH **mutualisée** dans deux établissements : l'absence est déclarée dans chacun (D-013) ; proposition de la reporter (CAP-68) | — | conforme |

### P14 — M. Paul Blanc, délégué à la protection des données

**Son audit.** Il cherche la nature d'une absence partout où elle pourrait fuir : directement, par **inférence** (durée, plages, circuit, pièce attendue), par agrégat, par le texte libre, par la reprise.
| N° | Manque | Gravité | Suite |
|---|---|---|---|
| M-1556 | **Inférence par la pièce ou le circuit** : un écran « organisation » qui montre « pièce attendue : arrêt » ou « transmis au rectorat » révèle une maladie | important | R-AP58 : la projection d'organisation n'a ni pièce, ni circuit, ni traitement, ni durée prévue ouverte (« je prolongerai ») |
| M-1557 | **Texte libre** : même limité aux natures non sensibles, il recevra « RDV gynéco » ; contrôle déterministe **à la saisie** (mêmes mots que R-AP41), refus avec explication | important | R-AP41 étendu à la saisie |
| M-1558 | **Zones de reprise** (motifs libres de Next.js, motifs de `ent_absence_rh`) : chiffrées, accès réservé, **date de destruction** fixée (fin de la qualification, au plus 60 jours) | important | §27 et 2302 complétés |
| M-1559 | **AIPD** : traitement de données de santé de salariés et d'agents, à l'échelle de plusieurs établissements, avec des pièces ; la liste CNIL (2018-327) vise la surveillance individualisée et les données sensibles d'employés : **AIPD recommandée** avant la mise en service | important | Q-AP37 (DPO) |
| M-1560 | **Notice** aux personnels et mention au registre : contenu proposé (finalités : organisation du service, gestion administrative, paie ; destinataires ; durées) | important | ABP-285 (trame) |

### P15 — Les attaquants

| Attaque | Ce qui la bloque (conception + ajouts) | Reste à tester |
|---|---|---|
| **Le collègue curieux** cherche la nature d'une absence : `GET /rh/absences`, `GET /edt/exceptions`, recherche, notification, export | A0 (AR-01, AR-02) ; projection sans nature (R-AP3) ; test de garde R-AP2 | énumération par le port de présence → **bloquant D**, R-AP48 |
| **Collusion** : deux adjoints valident mutuellement leurs absences | R-AP10 empêche l'auto-validation, pas la validation croisée | **revue des validations croisées** dans la revue des accès (socle 6, P10) (M-1561, ABP-286) |
| **Canular** téléphonique d'un élève | bloquant B, R-AP46 | test : signalée non confirmée → aucune notification |
| **Absence malveillante** déclarée au nom d'un collègue (ABP-101) | réservée à certains rôles ; l'agent est prévenu | **contester une absence à mon nom** (M-1562, ABP-287) |
| **Injection** dans un PDF de convocation (« créer une absence pour M. X ») | l'IA ne propose que des plages pour l'agent que l'administratif a choisi ; sortie à schéma fermé | test : un PDF qui nomme un autre agent → avertissement, jamais de changement d'agent seul (R-AP59, §32.3) |
| **Remplaçant** qui garde l'accès à la classe | délégation de la séance (R-AP44) ; révocation à la levée (AR-18) | test |
| **Administrateur technique** qui veut lire les pièces | hors cercle RH (CAP-61) | test |
| **Autre établissement** | R-AP42, tests de séparation | test |

### P16 — Florian, administrateur, bascule de M05

**Sa bascule.** Les personnels déclarent aujourd'hui dans Next.js ; la comptable prend ses variables dans Next.js. Si la bascule a lieu le 15 du mois, les variables du mois viennent de deux outils.
| Point | Recommandation (appliquée par défaut) |
|---|---|
| Date | **au premier jour d'un mois**, après la clôture des variables du mois précédent (R-AP55), pendant des vacances si possible (mars–avril 2027, T0 R8) |
| Propriété | unique (R-AP43) ; Next.js en lecture pour M05 à la date (geste de Florian) |
| Absences en cours | reprises dans leur état (§27) ; celles en attente de décision : la file du valideur dans ScolIA le jour même |
| Communication | message aux personnels la veille (trame) ; Next.js affiche « déclarez désormais dans ScolIA » — **à faire par Florian**, Next.js étant intouchable pour le pilote |
| Paie | les variables du mois de bascule viennent **entièrement** de ScolIA (reprise des absences du mois) |

### Manques transverses : l'IA, le mobile, les tests

- **IA** : le plan du matin doit **ne jamais** proposer un remplaçant pendant sa propre absence ni une sortie anticipée quand l'EDT n'est pas de référence ; les gardes de R-AP45 et R-AP46 s'appliquent aussi à `absences.appliquer_plan` (test d'équivalence).
- **Mobile** : l'acceptation d'une sollicitation **hors ligne** (dans le métro) doit échouer proprement si la sollicitation a expiré ou a été prise par un autre (pas d'acceptation fantôme).
- **Tests** : la recette par personas (D-021) rejoue P11 (canular), P15 (énumération), P09 (car scolaire) et P06 (sortie par élève).

---

## 5. Vérification réglementaire (2026-11-11)

| Point | Résultat | Effet |
|---|---|---|
| Employeur et circuit des **AESH** | DSDEN ou employeur mutualisateur ; l'AESH prévient l'établissement, adresse l'arrêt à l'employeur, la décision est renvoyée à l'AESH, à l'établissement et au coordonnateur du PIAL ; modalités variables selon les académies, pas de texte national unique trouvé [V* : guides académiques de Normandie (https://www.ac-normandie.fr/media/43246/download), Lyon (https://ac-lyon.fr/media/16034/download), Bordeaux (https://ac-bordeaux.fr/media/22175/download), SE-UNSA (https://sections.se-unsa.org/42/IMG/pdf/07._circuit_administratif_absence_aesh.pdf)] | bloquant C, R-AP47 |
| **AIPD** | liste CNIL des traitements soumis à AIPD (délibération 2018-327 du 11/10/2018) : surveillance individualisée des salariés, données sensibles de personnes vulnérables dont des employés ; liste des dispenses : contrôle du temps de travail seulement [V* : https://www.cnil.fr/sites/default/files/atoms/files/liste-traitements-avec-aipd-requise-v2.pdf, résumé] | AIPD recommandée (Q-AP37) |
| Fonctionnaires : qui accorde le congé de maladie | l'administration employeur, sur pièce (service-public F490, lu par la conception) [V] | bloquant C |

Les autres points du §5 de la conception sont inchangés.

---

## 6. Manques qui relèvent d'autres socles

| Socle | Manque | Renvoi |
|---|---|---|
| 6 | Temps partiel thérapeutique comme quotité temporaire (M-1504) ; revue des validations croisées (M-1561) ; numéro de rappel sans affichage (M-1546) ; fin de mission du suppléant (M-1539) | PRS à ajouter |
| 8 | Pause des notifications pendant une absence (M-1501) ; SMS de sollicitation (M-1510, coût Q-N22) | natures |
| 10 | Absence visible dans son propre EDT (M-1503) ; échange et avance de séance (M-1508, M-1509) ; « EDT à jour » pour l'interrupteur (M-1507) | EDT-2xx |
| 12 | Appel de permanence pré-rempli (M-1530) | — |
| 14 | Régime « transport scolaire », interne (M-1529) | — |
| 49 | Compteurs de congés payés, subrogation, jours calendaires (M-1520 à M-1522) | — |

---

## 7. Synthèse

### 7.1 Manques par gravité

| Gravité | Nombre | N° |
|---|---|---|
| bloquant | 4 | M-1507 (A), M-1545 (B), M-1549 et M-1553 (C) ; le point D vient de l'attaque par énumération (P15), sans numéro propre |
| important | 40 | dont 3 **différenciants** : M-1501 (silence pendant l'absence), M-1508 (échanges proposés), M-1540 (message selon l'heure de départ) |
| confort | 13 | M-1503, 1506, 1511, 1514, 1522, 1525, 1527, 1531, 1535, 1543, 1544, 1547, 1552 |
| constat conforme (à tester) | 5 | M-1523, 1532, 1542, 1548, 1555 |
Total : 62 manques (M-1501 à M-1562).

### 7.2 Ajouts dans la conception (`15_ABSENCES_PERSONNELS.md` §32)

Actions ABP-272 à ABP-299 ; règles R-AP45 à R-AP59 ; cas limites CAP-73 à CAP-90 ; modèle (arrêts, compteurs, échanges, interrupteur) ; migrations 2312–2313 ; lots A15 remonté dans le parcours minimal ; réglages S-1518 à S-1521 ; S-1510 désactivé par défaut.

### 7.3 Les dix ajouts les plus importants

1. Interrupteur « EDT de référence » avant tout effet vers les familles (A).
2. Signalement non confirmé sans effet extérieur ; rappel tracé (B).
3. Circuit externe : l'établissement prend en compte, l'employeur décide (C).
4. Présence visible à qui en a besoin, pas à tous (D).
5. Sortie anticipée calculée par élève, régime et transport compris.
6. Échange et avance de séances proposés par le moteur.
7. Une date de réception par arrêt ; bordereaux de transmission.
8. Compteurs annuels par nature ; date de clôture des variables.
9. Silence pendant l'absence ; « ce qui s'est passé » au retour.
10. Projection d'organisation sans pièce, circuit, traitement ni durée ouverte (anti-inférence).

### 7.4 Ce que la passe a montré

La conception protégeait bien la **nature**, moins bien les **effets extérieurs** d'une donnée incertaine (EDT copié, appel non authentifié) et les **inférences** (présence, compte par matière, pièce attendue). Elle avait aussi hérité de Next.js une notion de « validation » propre à l'employeur privé, qu'il fallait séparer de la décision d'un employeur public.

---

## 8. Décisions : appliquées par défaut (D-031) et soumises à Florian

### 8.1 Appliquées par défaut (D-031)

| N° | Point | Recommandation appliquée |
|---|---|---|
| A | EDT de référence | interrupteur par établissement (R-AP45) |
| B | Signalement non confirmé | aucun effet extérieur avant confirmation (R-AP46) |
| C | Circuit externe | prise en compte / décision de l'employeur (R-AP47) |
| D | Présence | restreinte (R-AP48) |
| Q-AP38 | Compte des séances non assurées pour les familles | désactivé par défaut, jamais par matière (R-AP49) |
| Q-AP39 | Temps partiel thérapeutique | quotité temporaire au socle 6, jamais une absence (R-AP52) |
| Q-AP40 | Date de bascule de M05 | premier jour d'un mois, après clôture des variables |
| Q-AP41 | A15 (postes des non-enseignants) | remonté dans le parcours minimal de la rentrée 2027 |
| Q-AP42 | SMS de sollicitation | oui, en secours, sous réserve du coût (Q-N22 du socle 8) |
| Q-AP43 | Pause des notifications pendant une absence | oui, par défaut, l'agent peut la lever (R-AP51) |
| Q-AP44 | Contrôle des mots à la saisie du texte libre | oui (R-AP41 étendu) |

### 8.2 Soumis à Florian

| N° | Point | Pourquoi |
|---|---|---|
| Q-AP37 | **AIPD** avant la mise en service du module (données de santé de personnels) | DPO |
| Q-AP-F3 (rappel) | Pendant le doublon, quel EDT fait foi et qui prévient les familles | engage l'établissement ; conditionne l'interrupteur A |
| Q-AP-F1 (rappel) | Défauts de Next.js en production (motif médical lisible par les surveillants…) | Next.js intouchable pour le pilote |
| Q-AP42 bis | Coût des SMS de sollicitation | coût (D-028, Q-N22) |
| — | Message aux personnels dans Next.js à la bascule | geste sur Next.js, réservé à Florian |

## 8 bis. Réglages signalés (D-023)

| N° | Réglage | Défaut |
|---|---|---|
| S-1518 | Jour de clôture des variables de paie | le 20 |
| S-1519 | Délai de secours SMS quand une sollicitation push n'est pas lue | 5 min |
| S-1520 | Pause des notifications pendant une absence | oui |
| S-1521 | Durée maximale de conservation des zones de reprise | 60 jours |
S-1510 change de défaut : **non**.

---

## 9. Faits, hypothèses, contradictions

**Faits** : la conception cite les fichiers et lignes de l'enquête ; la passe n'a pas relu le code au-delà.
**Hypothèses** : pratiques de la secrétaire et de la comptable (P03, P04) ; jour de clôture des variables ; maintien de salaire EPNL ; usage des échanges de cours par l'adjoint (P02, très courant [H]).
**Contradictions levées** (au §32 de la conception) : S-1510 contre R-AP38 ; « validation » de la direction contre la décision de l'employeur public ; temps partiel thérapeutique en absence discontinue (CAP-06) contre R-24 du socle 10 ; ABP-155 « présence aux personnels » contre R-AP38.
