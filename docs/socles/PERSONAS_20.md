# Passe « personas » sur le socle 20 — Cahier de textes et travail personnel

**Date :** 2026-11-11 — **Statut :** proposition, **non décidée** (à relire par Florian) ; les recommandations s'appliquent par défaut (D-031), sauf ce qui engage l'établissement à l'extérieur (§8.2). Aucun code écrit ni modifié ; aucun accès à une base, à un secret ni à un service de production. Le code Next.js a été **lu seulement**, de façon ciblée (module `cahier_texte`).
**Branche :** `ent-socle-20-conception`, partie de `origin/ent-mc0hoe` (`7467f2d`).
**Éprouve :** `docs/socles/20_CAHIER_DE_TEXTES.md` (écrit dans la même session, relu en entier), avec `CLAUDE.md`, `docs/DECISIONS.md` (D-022, D-031, D-032, D-034, D-035, D-036), `10_EMPLOI_DU_TEMPS.md` (séance stable, AESH, PFMP), `06_PERSONNELS.md` (interventions, suppléance, Q-P5), `21_EVALUATIONS.md` (EVA-114, EVA-117, EVA-265, Q-EV1), `16_MESSAGERIE.md` (R-18), `08_NOTIFICATIONS.md`, `34_SANTE.md` ; forme reprise de `PERSONAS_10.md`, `PERSONAS_21.md`, `PERSONAS_42.md`.
**Ajouts :** faits **directement** dans `20_CAHIER_DE_TEXTES.md`, au **§32 « ajout personas »**, plus des corrections ponctuelles marquées « ajout personas » (§0, §10.3, §19.2, §28). **Rien n'est supprimé** ; les contradictions sont écrites au §30 de la conception.
**Numérotation :** pour éviter toute collision avec les sessions parallèles du 2026-11-11, les identifiants de cette passe portent le préfixe du socle : manques **M20-01 à M20-78** ; actions **CDT-250 à CDT-289** ; règles **R-C29 à R-C44** ; cas limites **CC-68 à CC-90** ; questions **Q-CDT13 à Q-CDT20** ; réglages **S20-12 à S20-15**.

---

## 0. À lire en premier : trois bloquants et leur recommandation (appliquée par défaut, D-031)

| # | Point | Pourquoi c'est bloquant | Recommandation (appliquée par défaut) | Lots |
|---|---|---|---|---|
| **A** | **La plage calme est devenue une obligation datée.** L'arrêté du 7-9-2026 (JORF du 23-9-2026, § 4.4, critères 30-34) impose aux ENT de ne diffuser **aucun devoir** entre 20 h et 7 h en semaine et du vendredi 20 h au lundi 7 h, avec un message de rappel, sous **dix mois** : vers **fin juillet 2027**. Le code actuel publie tout dès l'enregistrement (C-D13) et la conception place la plage calme en C2. | L'année de doublon commence en septembre 2027 ; si le cahier y est en usage réel (Q-CDT2), ScolIA serait **non conforme dès le premier jour** ; une professeure qui saisit le dimanche soir (P01) déclencherait des avis à 21 h chez 150 familles. | **C2a** sorti du lot C2 et placé **juste après C0** : `visible_a` calculé côté serveur (plage nationale, jours fériés et vacances du calendrier du socle 5), avis différés, message de rappel à l'auteur, levée par la direction tracée ; test d'or « saisie dimanche 21 h → visible et avis lundi 7 h ». Livrable avant la démo de décembre si possible (petit). | C2a |
| **B** | **Une modification « mineure, sans avis » efface la valeur juridique.** Le §9.1 permet au professeur de modifier une consigne publiée sans prévenir. Le cahier est un **document officiel à valeur juridique** (circulaire 2010-136) ; en cas de litige (P09, P05), une consigne changée la veille d'une évaluation sans trace visible est indéfendable pour l'établissement comme pour le professeur. | Un parent qui conteste « le travail n'était pas celui-là » doit pouvoir lire la version qu'il a vue ; un professeur accusé à tort doit pouvoir montrer la sienne. | **R-C29** : toute modification d'une entrée ou d'un travail **publié** est visible de tous ses lecteurs (« modifié le 12/03 à 18 h 04 », versions consultables par l'élève et ses responsables) ; seul l'**avis** peut être omis pour une correction mineure ; une modification de consigne, d'échéance, de documents ou de mode de rendu **dans les 24 h avant l'échéance** envoie toujours un avis et propose un délai (R-C30). | C2 |
| **C** | **Le corrigé des exercices ne doit jamais quitter le serveur avant la correction.** La conception (§15) décrit des exercices auto-corrigés mais ne dit pas où vit le corrigé. Dans la plupart des outils grand public, les bonnes réponses sont dans la page (P16). | Un élève qui lit la réponse du réseau triche sans effort ; tous les scores proposés deviennent faux ; le professeur verse au socle 21 des scores sans valeur. | **R-C31** : les réponses attendues, la propriété `correct` des choix et les explications ne sont **jamais** envoyées au navigateur ni à l'appli avant que la correction ne soit visible selon le réglage de l'exercice ; correction **côté serveur** à chaque réponse ; tentatives comptées côté serveur sous verrou ; test de sécurité qui inspecte les charges utiles. | C10 |

**Corrections de fond apportées à la conception** (détail §7.2) : copies déposées et copie **annotée** distinguées pour les familles (S20-08 tranché) ; aménagements **appliqués par défaut** à chaque nouveau travail ; **accès tiers en lecture** généralisé (inspection, enseignants du SAPAD ou de l'hôpital) ; **lecture préparatoire** du cahier par un suppléant nommé avant sa prise de poste ; **projets pluriannuels** (chef-d'œuvre, LP) ; **visa d'inspection** (la circulaire prévoit que direction **et** inspection visent) ; **traduction** des consignes pour les familles allophones ; **fiche papier** de la semaine pour les familles sans numérique ; **déclaration d'usage de l'IA** au dépôt ; **AIPD** du socle à faire avec le DPO.

---

## 1. Méthode

**Démarche.** Celle de `PERSONAS_21.md` §1 et `PERSONAS_42.md` §1 : dix-sept personas réalistes, tirées du §3 de la conception, couvrant collège et lycée, général et professionnel, public (EPLE) et privé sous contrat (lycée pilote, D-024), les familles ordinaires et les cas particuliers, un **attaquant** (P16, plusieurs profils) et un **auditeur RGPD** (P15). Pour chacune : sa semaine ou son année, ses irritants, ses peurs, ce qui la ravirait ; les **manques** et **contradictions** ; les cas limites ; les exigences nouvelles.

**Grille transversale** posée à chaque persona : (1) **date** — que voit-elle, à quelle date, après un changement de groupe, de professeur, d'année ? (2) **portée** — qui d'autre lit ce qu'elle écrit, et qu'est-ce qui n'a pas à être lu ? (3) **trace** — en cas de litige, que peut-on prouver ? (4) **charge** — ce que la règle fait de sa semaine ; (5) **IA** — ce qu'elle prépare, ce qu'elle ne doit jamais faire ; (6) **D-036** — ce qu'elle saisit est-il une colonne, une table, ou un JSON caché ?

**Vérification réglementaire.** Le §5 de la conception (24 règles, vérifiées le 2026-11-11) sert de référence ; aucune source nouvelle n'a été ajoutée par cette passe, sauf la lecture de l'arrêté du 7-9-2026 pour les bloquants A et la déclaration d'usage de l'IA (§5).

**Gravité** (identique aux passes précédentes) : **bloquant** · **important** · **confort** · **différenciant**.

**Limites.** Les personas sont construites, pas interviewées [H]. Les pratiques citées (saisie du dimanche, Devoirs faits en étude, chef-d'œuvre, SAPAD) sont courantes ; leurs volumes sont des ordres de grandeur.

---

## 2. Les personas en un tableau

| # | Persona | Établissement | Angle principal |
|---|---|---|---|
| P01 | **Mme Martin**, lettres, cinq classes de collège, saisit le dimanche soir | collège public | plage calme, préparation, copie de séance, cahier individuel papier |
| P02 | **M. Okafor**, SVT, spécialité de terminale (4 classes mêlées) et seconde | lycée pilote (privé sous contrat) | charge par élève, DS du samedi, épreuves du bac |
| P03 | **Mme Chen**, suppléante de physique du 12 janvier au 3 avril | lycée pilote | reprise d'un cours, copies en attente, lecture avant la prise de poste |
| P04 | **M. Girard**, remplaçant d'une heure (RCD) et co-intervenant en enseignement scientifique | lycée public | portée d'une séance, preuve de service, co-intervention |
| P05 | **Mme Duval**, cheffe d'établissement, et M. Faure, adjoint | lycée pilote | visa, tenue, litige, crainte d'une surveillance des professeurs |
| P06 | **M. Robert**, IA-IPR de physique-chimie | académie | inspection, année précédente, visa d'inspection |
| P07 | **Mme Benali**, CPE, et **Yanis**, AED de Devoirs faits et de l'étude de l'internat | collège avec internat | agenda en étude, plage calme à l'internat, « fait en étude » |
| P08 | **Lina**, 4ᵉ, dyslexique (PAP), et **Mme Diallo**, AESH mutualisée sur deux élèves | collège public | variantes, aménagements par défaut, AESH |
| P09 | **M. et Mme Garnier**, séparés ; le père n'a pas l'exercice de l'autorité parentale | collège public | égalité d'information, copies, litige sur une consigne |
| P10 | **Hugo**, 18 ans, terminale | lycée pilote | majeur, IA autorisée, portfolio |
| P11 | **Noah**, seconde, hospitalisé six semaines (SAPAD), puis exclu trois jours | lycée public | continuité, enseignants extérieurs, exclusion temporaire |
| P12 | **Inès**, bac pro, PFMP de quatre semaines, chef-d'œuvre sur deux ans | lycée professionnel | PFMP, co-intervention, projet pluriannuel |
| P13 | **M. Lefèvre**, professeur principal de 3ᵉ, brevet blanc | collège public | calendrier de classe, semaines protégées |
| P14 | **Mme Ndiaye**, mère allophone, un téléphone, peu à l'aise à l'écrit | collège REP | traduction, fiche papier, avis compréhensibles |
| P15 | **M. Blanc**, délégué à la protection des données | lycée pilote (OGEC) | conservation, minimisation, AIPD, IA, journalisation |
| P16 | **Les attaquants** : Tom (élève de 3ᵉ curieux), un parent dont le lien est suspendu, un ancien professeur | — | IDOR, XSS, fichiers piégés, triche aux exercices, jetons |
| P17 | **Florian**, administrateur, rentrée 2027 | lycée pilote | usage réel ou doublon, conformité, reprise |

---

## 3. Parité et existant

- **Next.js** [F] : module **ébauche** (`app/api/cahier-texte/route.ts`, `db/schema-socle-faits.ts:373-399`), sans contrôle de rôle à l'écriture, famille par classe texte. Rien à reprendre au titre de D-035 ; **aucune fonction utilisée à préserver** n'a été trouvée (pas de rendu en ligne, pas de pièces, pas de charge). Défaut à signaler à Florian : **NX-CDT-1** (tout utilisateur connecté de l'intranet crée, modifie ou masque n'importe quelle entrée).
- **Charlemagne** : pas de module cahier de textes dans la matrice (`MATRICE_CHARLEMAGNE.md` §3.8, sources E, P, N, X seulement) ; l'établissement pilote tient donc son cahier ailleurs (Q-CDT2).
- **Marché** (de mémoire [H], non vérifié au détail) : Pronote et École Directe proposent cahier par séance, travail à faire, rendu en ligne, QCM, charge de travail ; leurs lacunes connues côté terrain : modifications silencieuses, avis tardifs, charge par classe et non par élève, peu d'outils pour l'AESH, corrigés de QCM lisibles par les élèves avertis [H].

---

## 4. Parcours et manques, persona par persona

### P01 — Mme Martin, lettres, cinq classes de collège

**Sa semaine.** 22 heures de cours : deux 6ᵉ, deux 5ᵉ, une 3ᵉ. Elle prépare le dimanche entre 20 h et 23 h ; elle copie la même séance sur ses deux 5ᵉ ; elle photographie le tableau en fin d'heure ; ses élèves notent aussi le travail sur leur agenda papier (R-05).
**Irritants.** Les avis qui partent pendant qu'elle saisit ; recopier ; devoir choisir une date pour un travail « pour la prochaine fois ». **Peurs.** Qu'un parent lui reproche un devoir publié à 22 h ; qu'une photo du tableau montre un élève. **Ce qui la ravirait.** Préparer le dimanche, que tout parte lundi 7 h, et voir écrit « visible lundi à 7 h ».

| N° | Manque | Gravité | Où |
|---|---|---|---|
| M20-01 | **Plage calme obligatoire** avec rappel à l'auteur, conforme à l'arrêté du 7-9-2026 avant juillet 2027 | **bloquant** (A) | C2a ; R-C5 |
| M20-02 | « **Pour la prochaine séance** » et « **pour dans deux séances** » comme choix d'échéance par défaut, sans regarder un calendrier | important | §9.1 (déjà prévu) ; CDT-250 : raccourcis |
| M20-03 | **Dupliquer en lien** : une séance copiée sur deux 5ᵉ peut être modifiée une fois pour les deux (« modifier aussi dans 5ᵉ B ? ») | confort | CDT-251 |
| M20-04 | **Floutage proposé** des visages sur la photo du tableau (traitement local, sans IA distante) ou avertissement seul | confort | CDT-252 ; R-C26 |
| M20-05 | Les élèves sans téléphone notent sur papier : le professeur veut **projeter** le travail de la séance en fin d'heure (vue « tableau » en grand caractère) | confort | CDT-253 |
| M20-06 | Saisie hors ligne au collège (réseau faible) : brouillon local sur le web aussi, pas seulement sur l'appli | important | C2, C11 |

### P02 — M. Okafor, SVT, spécialité de terminale et seconde

**Sa semaine.** Un groupe de spécialité de 28 élèves issus de quatre classes ; un DS de trois heures le **samedi matin** (hors séance de son cours, organisé par la vie scolaire) ; une ECE au printemps.
**Irritants.** Le bandeau de charge d'une classe ne veut rien dire pour son groupe ; le DS du samedi n'est la séance d'aucun cours. **Peurs.** Mettre un DS le même jour qu'un DS de mathématiques qu'il ne voit pas. **Ravi si.** Il voit « 9 de vos élèves ont déjà une évaluation ce samedi ».

| N° | Manque | Gravité | Où |
|---|---|---|---|
| M20-07 | Les **devoirs surveillés hors séance** (samedi matin, plages communes) ne sont pas des séances : ils doivent exister comme **évaluations du socle 21 avec une plage horaire** (et une séance ajoutée du socle 10) et compter dans la charge | important | §11.1 ; port 21 ; CC-68 |
| M20-08 | Les **épreuves du baccalauréat** et des examens blancs (socle 38) dans l'agenda et dans la charge, avec **semaines protégées** | important | §11 ; CDT-254 |
| M20-09 | La charge doit compter le **travail long** réparti (exposé à préparer sur 3 semaines) par jour, pas seulement le jour de l'échéance | confort | §11.1 (durée répartie, R-C32) |
| M20-10 | Distinguer « **évaluation annoncée** » (socle 21) et « travail à rendre » dans le bandeau (deux nombres) — déjà prévu, à garder lisible sur mobile | confort | écran 2 |

### P03 — Mme Chen, suppléante de physique, 12 janvier – 3 avril

**Son arrivée.** Nommée le jeudi 8 janvier pour le lundi 12 ; elle veut préparer le week-end. Le titulaire, en congé de maladie, a laissé 34 copies **déposées non corrigées** et une progression **privée**.
**Irritants.** Arriver sans rien voir ; ne pas savoir quelles copies sont à elle. **Peurs.** Corriger contre la progression du titulaire ; voir des aménagements trop tard. **Ravie si.** Un écran « où en est le cours » dès sa nomination.

| N° | Manque | Gravité | Où |
|---|---|---|---|
| M20-11 | **Lecture préparatoire** du cahier (entrées, travaux, aménagements utiles) dès la **nomination** de la suppléance, avant sa date de début, sans saisie ni lecture des copies | important | R-C33 ; CDT-255 ; Q-CDT13 (à porter au socle 6) |
| M20-12 | **Copies en attente à la prise de poste** : liste explicite, choix par la direction ou la suppléante « je les corrige / le titulaire les corrigera / rendues sans correction » ; jamais de correction par un titulaire en congé de maladie | important | CDT-256 ; CC-69 |
| M20-13 | **Progression privée** du titulaire : la direction peut demander au titulaire (s'il est joignable) de la partager ; jamais d'ouverture forcée (création personnelle) | confort | §16 ; CDT-257 |
| M20-14 | **Écran d'accueil** de suppléance : dernière séance, prochaine échéance, travaux en cours, élèves avec aménagements (pastilles), PP, collègue de la discipline | important | CDT-202 enrichi |

### P04 — M. Girard, remplaçant d'une heure et co-intervenant

**Sa journée.** Remplacement de courte durée en 1ʳᵉ ; l'après-midi, co-intervention en enseignement scientifique avec une collègue de SVT.
**Irritants.** Ne pas savoir ce que la classe a fait ; écrire dans le cahier d'un autre. **Peurs.** Donner un travail que le titulaire annulera. **Ravi si.** Son heure assurée est prouvée sans papier.

| N° | Manque | Gravité | Où |
|---|---|---|---|
| M20-15 | Un travail donné par le remplaçant pour une séance du titulaire : le titulaire est **prévenu** et peut le garder, le modifier ou le retirer (avis aux élèves) | important | CDT-258 |
| M20-16 | **Preuve de service** : l'entrée de séance signée par le remplaçant est lue par le socle 15 (remplacement effectué) et le socle 49 (heures), sans nouvelle saisie | différenciant | port `cahier::seance_renseignee_par` ; §6.2 |
| M20-17 | **Co-intervention** : une séance, deux intervenants **de disciplines différentes** sur un même cours (enseignement scientifique) ou sur deux cours (co-intervention en LP) : les deux écrivent, chacun signe, un seul travail commun possible | important | §8.1, CC-16 ; CDT-125 |

### P05 — Mme Duval, cheffe d'établissement, et M. Faure, adjoint

**Leur trimestre.** 62 cours ; visa à chaque fin de trimestre ; un parent conteste un devoir « jamais donné » ; les représentants des professeurs ont demandé au conseil pédagogique comment le « taux de tenue » serait utilisé.
**Irritants.** Ouvrir 62 cahiers. **Peurs.** Être accusée de flicage ; un litige sans trace. **Ravie si.** Viser par échantillon, montrer la version vue par le parent.

| N° | Manque | Gravité | Où |
|---|---|---|---|
| M20-18 | **Visa par échantillon** : la direction vise une classe entière à partir d'un échantillon tiré au hasard d'entrées par cours, avec la liste de ce qui a été lu | confort | CDT-259 |
| M20-19 | **Transparence du tableau de tenue** : chaque professeur voit **son** taux et les mêmes chiffres que la direction ; aucun classement ; aucun export nominatif hors litige ; la règle d'usage est affichée (charte proposée au conseil pédagogique) | important | R-C34 ; CDT-260 |
| M20-20 | **Version vue par un lecteur à une date** : en litige, la direction affiche la version d'un travail **telle qu'elle était visible** le 12 mars à 18 h | important | bloquant B ; CDT-261 |
| M20-21 | Le **retrait** d'un contenu par la direction (C-D10) doit pouvoir être **contesté** par l'auteur (réponse écrite, gardée) | confort | CDT-262 |

### P06 — M. Robert, IA-IPR de physique-chimie

**Sa visite.** Rendez-vous de carrière de M. Okafor en mars ; il veut le cahier de cette année et de l'an passé (année close) ; il vise d'habitude le cahier.
**Irritants.** Identifiants partagés, PDF hétérogènes. **Peurs.** Recevoir des données d'élèves. **Ravi si.** Lecture directe, bornée, avec possibilité de viser.

| N° | Manque | Gravité | Où |
|---|---|---|---|
| M20-22 | **Visa d'inspection** : la circulaire prévoit que direction **et** inspection visent (R-02) ; l'inspecteur appose un visa daté, avec observation adressée au professeur | important | CDT-263 ; §14.2 |
| M20-23 | L'accès d'inspection couvre **l'année close** (cahier lisible un an) et l'**archive** quand elle est demandée | important | CDT-215 enrichi |
| M20-24 | L'export pour l'inspection inclut les documents joints du professeur (l'inspection fait partie du public autorisé par la circulaire) mais **jamais** les productions d'élèves | confort | §14.2 |

### P07 — Mme Benali, CPE, et Yanis, AED (Devoirs faits, étude de l'internat)

**Leur soirée.** Devoirs faits en 6ᵉ de 16 h à 17 h ; étude des internes de 18 h à 19 h 30 puis de 20 h 15 à 21 h.
**Irritants.** « J'ai rien » ; ne pas savoir ce qui est attendu demain. **Peurs.** Lire des copies ou des notes. **Ravis si.** La liste du groupe d'étude avec le travail du lendemain de chaque élève, et pouvoir noter « fait en étude ».

| N° | Manque | Gravité | Où |
|---|---|---|---|
| M20-25 | **Vue du groupe d'aide aux devoirs** (Devoirs faits, étude, internat) : les élèves du groupe (socle 5) et leurs échéances du lendemain et du surlendemain, consignes seulement | important | CDT-264 ; portée « mes groupes » |
| M20-26 | **« Travaillé en étude »** : l'encadrant note qu'un élève a travaillé sur tel travail (fait / commencé / besoin d'aide) ; visible du professeur du cours et du PP ; visible de l'élève ; **pas** des familles par défaut | important | CDT-265 ; R-C35 |
| M20-27 | La plage calme **n'empêche pas la consultation** à l'internat après 20 h (rappel) : seule la diffusion est différée ; un travail publié à 20 h 30 n'apparaît pas à l'étude du soir | confort | R-C5 (rappel) |

### P08 — Lina, 4ᵉ (PAP), et Mme Diallo, AESH mutualisée

**Sa semaine.** Lina lit lentement ; son PAP prévoit des consignes courtes, une police adaptée, un temps majoré d'un tiers, moins d'exercices. Mme Diallo accompagne Lina et un élève de 5ᵉ.
**Irritants.** Les professeurs oublient l'aménagement ; la variante arrive le lendemain. **Peurs.** Que les autres voient qu'elle a « un autre devoir ». **Ravie si.** Elle a sa version sans rien demander.

| N° | Manque | Gravité | Où |
|---|---|---|---|
| M20-28 | **Aménagements appliqués par défaut** à chaque nouveau travail (délai majoré, formats, police) avec une pastille « 2 élèves : aménagements appliqués », que le professeur peut lever pour ce travail | important | R-C36 ; CDT-266 |
| M20-29 | **Lecture à voix haute** et police adaptée côté élève sans action du professeur (accessibilité de l'agenda, du cahier, de la consigne) | important | CDT-175 enrichi ; socle 11 |
| M20-30 | Une variante attribuée **après** la publication ne change pas l'échéance déjà passée en retard : statut recalculé (comme CC-41) | confort | CC-70 |
| M20-31 | L'AESH mutualisée voit **les seuls** élèves accompagnés **ce jour-là** ; sa note d'accompagnement est visible des professeurs du cours et du PP, **jamais** des autres AESH | important | §13.3 ; CC-71 |
| M20-32 | Un temps majoré **sur un exercice chronométré** est appliqué seul (déjà CC-65) ; et le **nombre d'exercices allégé** de la variante est respecté par l'exercice (questions facultatives) | confort | §15 ; CDT-267 |

### P09 — M. et Mme Garnier, séparés

**Leur situation.** Garde chez la mère ; le père n'a pas l'exercice de l'autorité parentale mais un droit d'information ouvert par le socle 4 (C. civ. 373-2-1). La mère conteste une consigne « changée la veille ».
**Irritants.** Ne pas recevoir les mêmes choses. **Peurs** (père) : être oublié ; (mère) : que le père voie des documents qu'elle n'a pas choisi de partager. **Ravis si.** Même information, et la preuve de ce qui était écrit.

| N° | Manque | Gravité | Où |
|---|---|---|---|
| M20-33 | **Versions visibles** des lecteurs (bloquant B) : la mère lit « modifié le 11/03 à 21 h 40 » et l'ancienne version | **bloquant** (B) | R-C29 |
| M20-34 | **S20-08 tranché** : les responsables habilités voient la **copie annotée rendue** (retour) — c'est déjà l'objet du suivi — mais **pas** les dépôts en cours de l'élève avant le retour (espace de travail de l'élève) ; pour un retour porteur de note, droit « voir les notes » | important | §10.3, §19.2 (ajout personas) ; R-C37 |
| M20-35 | Le parent **informé sans exercice** de l'autorité parentale voit cahier, agenda, statuts et retours comme l'autre parent, sauf restriction **motivée** enregistrée au socle 4 (D111-4) | important | R-C7 ; CC-48 corrigé |
| M20-36 | **Résumé hebdomadaire** par responsable, dans la langue choisie par chacun (socle 8) | confort | CDT-180 |

### P10 — Hugo, 18 ans, terminale

**Son année.** Il a fermé l'accès de ses parents au cahier. Il utilise l'IA pour relire ses dissertations ; un professeur l'autorise « en relecture », un autre non.
**Irritants.** Ne pas savoir ce qui est permis. **Peurs.** Être accusé de fraude. **Ravi si.** La règle est écrite sur chaque devoir et il peut déclarer ce qu'il a fait.

| N° | Manque | Gravité | Où |
|---|---|---|---|
| M20-37 | **Libellés explicites** des niveaux d'IA autorisée, avec un texte court affiché sur la consigne (« vous pouvez utiliser une IA pour relire l'orthographe, pas pour rédiger ») et un texte libre du professeur | important | R-C27 ; CDT-268 |
| M20-38 | **Déclaration d'usage** au dépôt quand l'IA est autorisée (« j'ai utilisé une IA pour… », facultative, visible du professeur seulement) ; jamais un outil de détection | différenciant | CDT-269 ; R-C38 |
| M20-39 | **Portfolio** de fin de lycée : export de ses productions et retours avant la purge (déjà CDT-174), avec rappel en juin de terminale | confort | CDT-270 |

### P11 — Noah, seconde : SAPAD six semaines, puis exclusion temporaire de trois jours

**Son trimestre.** Hospitalisé en novembre, suivi par deux enseignants du **SAPAD** (non personnels de l'établissement) ; en février, exclu trois jours (socle 13).
**Irritants.** Les enseignants du SAPAD reçoivent des photocopies. **Peurs** (famille) : qu'on raconte sa maladie. **Ravie si** (famille) : il garde le fil sans rien expliquer.

| N° | Manque | Gravité | Où |
|---|---|---|---|
| M20-40 | **Accès tiers en lecture pour un élève** : un enseignant extérieur (SAPAD, école à l'hôpital, CNED) lit le cahier et l'agenda **de cet élève** et dépose ses retours dans un espace dédié, accordé par la direction avec l'**accord des responsables**, borné, journalisé ; l'accès d'inspection devient un cas de ce mécanisme général | important | §14.2 généralisé ; CDT-271 ; table `ent_cdt_acces_tiers` |
| M20-41 | La **continuité** ne porte **aucun motif** ; le bandeau dit « élève en continuité à distance » (déjà §12.1) ; les avis aux professeurs aussi | important | R-C8 |
| M20-42 | **Exclusion temporaire** : le socle 13 envoie la demande de travaux aux professeurs **avec la période**, jamais le motif de la sanction ; le cahier collecte les travaux et les rend visibles de l'élève et de ses responsables | important | CDT-208 ; R-C39 |

### P12 — Inès, bac pro : PFMP et chef-d'œuvre

**Son année.** Quatre semaines de PFMP en novembre ; co-intervention français–atelier ; chef-d'œuvre commencé en 1ʳᵉ, présenté en terminale.
**Irritants.** Recevoir des devoirs pendant le stage. **Peurs.** Perdre le travail de 1ʳᵉ au changement d'année. **Ravie si.** Son chef-d'œuvre la suit.

| N° | Manque | Gravité | Où |
|---|---|---|---|
| M20-43 | **Aucune échéance pendant une PFMP** de l'élève (période du socle 10 §11.9) : refus serveur pour elle, ou échéance individuelle reportée au retour, proposée | important | R-C40 ; CC-72 |
| M20-44 | **Projet pluriannuel** (chef-d'œuvre, projets longs) : un projet relie des travaux sur deux années ; les dépôts de l'année 1 restent lisibles en année 2 par l'élève et par l'intervenant en saisie sur le cours de l'année 2 du même élève ; la purge suit la fin du projet | important | CDT-272 ; table `ent_cdt_projet` ; R-C41 |
| M20-45 | Rapport de PFMP rendu **au retour** : travail du cahier (socle 20) ou pièce du socle des stages (39) ? → le **rapport** est un travail du cahier ; la **convention et l'évaluation du tuteur** restent au socle des stages | confort | §6.1 (ajout) |

### P13 — M. Lefèvre, professeur principal de 3ᵉ

**Son trimestre.** Brevet blanc en janvier ; il veut une semaine sans nouvelle évaluation avant ; il suit trois élèves qui ne rendent rien.
**Ravi si.** Il pose une « semaine protégée » et l'équipe le voit au moment de donner.

| N° | Manque | Gravité | Où |
|---|---|---|---|
| M20-46 | **Périodes de vigilance** posées par le PP ou la direction sur une classe ou un niveau (repère abaissé d'un cran, message affiché au professeur qui donne une évaluation) | confort | CDT-273 ; S20-12 |
| M20-47 | **Élèves qui ne rendent rien** : vue du PP sur sa classe, toutes matières (il est de l'équipe), faits seulement, avec lien vers le fil d'équipe (socle 16) | important | CDT-158 (déjà) ; R-C15 précisé |
| M20-48 | Le **conseil de classe** lit les non-rendus de la période (port déjà prévu) **en nombres**, jamais les copies | confort | §6.2 |

### P14 — Mme Ndiaye, mère allophone, un téléphone

**Sa semaine.** Elle reçoit des avis qu'elle ne comprend pas toujours ; son fils lui dit « j'ai rien ».
**Ravie si.** Elle lit la semaine dans sa langue et la reçoit aussi sur papier.

| N° | Manque | Gravité | Où |
|---|---|---|---|
| M20-49 | **Traduction des consignes** pour les familles (bouton « traduire » ; traduction automatique marquée comme telle ; le texte du professeur ne contient aucune donnée d'élève, R-C26) ; fournisseur conforme au cœur IA, jamais grand public | important | CDT-274 ; §17 |
| M20-50 | **Fiche papier hebdomadaire** de l'enfant (agenda de la semaine), imprimable par l'établissement ou la famille, pour les familles sans numérique ou les élèves sans téléphone | important | CDT-275 |
| M20-51 | **Avis en langage simple**, pictogrammes, sans sigle (« DS », « DM ») | confort | §22.3 |

### P15 — M. Blanc, délégué à la protection des données

**Son audit.** Il lit le §24.3, le §19 et le §17 ; il pose ses questions.

| N° | Constat | Gravité | Où |
|---|---|---|---|
| M20-52 | Une **AIPD** (analyse d'impact) est nécessaire : traitement à grande échelle de données de **mineurs** (copies, retours, tentatives, statuts), avec IA ; à faire avant l'usage réel | important | Q-CDT14 (soumis : DPO) |
| M20-53 | Le **« vu »** est-il nécessaire ? Le seul usage (avis « modifié après lecture ») disparaît avec le bloquant B (toute modification est visible et, si utile, avisée à tous) : **supprimer** la table `ent_cdt_vu` | important | R-C42 ; §24.1 (ajout personas : table retirée) |
| M20-54 | Le **journal de lecture de l'agenda** par la vie scolaire (`agenda_vie_scolaire`) : proportionné seulement pour une lecture **hors** groupe d'étude ; journal limité à 1 an | confort | §24.1, CDT-154 |
| M20-55 | Les **statistiques d'exercices** ne doivent pas devenir un **profil** de l'élève dans le temps (pas de courbe de réussite par élève sur l'année, pas de score agrégé) : statistiques par exercice seulement | important | R-C43 |
| M20-56 | Les **dépôts vidéo** (oral filmé) contiennent l'image du mineur : format désactivé par défaut, ouvert par travail, conservation la plus courte (fin de l'année) | important | S20-13 ; §24.3 |
| M20-57 | Une copie **signalée** (contenu préoccupant) part au dossier du socle 13 ou 36 : la copie **n'est pas** gardée plus longtemps au socle 20 ; c'est le dossier de destination qui fixe sa durée | confort | §10.4 précisé |
| M20-58 | Les **avis** sur l'écran verrouillé ne montrent ni le statut « non rendu » ni le contenu d'un retour | important | §19.3 (déjà) ; test |
| M20-59 | **Registre** : finalités séparées (cahier officiel ; travail et rendu ; exercices ; IA) avec leurs durées | confort | §24.4 |
| M20-60 | **Information des personnes** : notice pour élèves et familles (ce qui est vu par qui, durées, IA) et pour les professeurs (tableau de tenue, visa, inspection) | important | CDT-276 |

### P16 — Les attaquants

**Tom** (3ᵉ) : veut la copie de sa voisine, le corrigé du QCM, et une remise « à l'heure » faite en retard. **Un parent suspendu** par décision de justice : veut continuer à suivre son fils. **Un ancien professeur** de la classe : veut lire les copies de l'année.

| N° | Attaque | Défense attendue | Gravité | Où |
|---|---|---|---|---|
| M20-61 | **Corrigé dans la page** ou dans la réponse de l'API | R-C31 (bloquant C) ; correction serveur ; test des charges utiles | **bloquant** (C) | C10 |
| M20-62 | **Tentatives parallèles** pour contourner le nombre de tentatives | compteur et verrou serveur ; une tentative ouverte à la fois (CC-67) | important | C10 |
| M20-63 | **IDOR** sur `remise`, `depot`, `piece`, `retour`, `tentative`, `variante`, `groupe`, `acces` | contrôle d'appartenance cours + élève + établissement à chaque route ; tests générés à partir de la matrice | important | §27 |
| M20-64 | **XSS** par le balisage maison, un nom de fichier, un lien `javascript:`, un SVG | aucun HTML stocké ; échappement au rendu ; https seulement ; SVG refusé en dépôt d'élève et converti en image pour les ressources | important | R-C22 ; CC-73 |
| M20-65 | **Horodatage** forgé côté client | heure serveur (R-C10) ; aucune heure acceptée du client | important | — |
| M20-66 | **Saturation** : 1 000 dépôts ou 2 Go sur une remise | plafond de dépôts par remise (20) et de taille cumulée (S20-14), limitation de débit | important | R-C44 |
| M20-67 | **Jeton d'abonnement** au calendrier ou lien signé réutilisé après la suspension du lien parental | jetons révoqués à la suspension, au départ, à la fin d'une intervention ; liens signés liés à la session (07 §17.3) | important | CC-74 |
| M20-68 | **Ancien professeur** qui garde un onglet ouvert | portée recalculée à chaque requête (R-C2), sessions non concernées, test J+8 | important | C0 |
| M20-69 | **Archive de téléchargement** groupé : noms de fichiers piégés (chemins, caractères de contrôle) | noms reconstruits par le serveur (« NOM Prénom – titre – n° »), jamais le nom d'origine dans un chemin | important | CDT-185 |
| M20-70 | **Énumération des élèves** d'un autre groupe par le port de charge avec un identifiant de cours forgé | le port n'accepte que les cours de l'appelant ; nombres seulement ; seuil de 5 élèves sous lequel la distribution est masquée | important | R-C15 |
| M20-71 | **Lien externe** imitant la page de connexion | avertissement de sortie, liste de blocage, aucun lien dans les avis | confort | CC-45 |
| M20-72 | **Message de harcèlement** déposé comme « question sur le travail » ou dans un dépôt de groupe | modération du socle 16 ; signalement depuis la copie (CDT-199) | confort | — |

### P17 — Florian, administrateur, rentrée 2027

**Sa question.** Le lycée pilote tient-il son cahier dans ScolIA dès septembre 2027 ?
**Ce qu'il découvre.** Charlemagne n'a pas de cahier ; Next.js n'a qu'une ébauche ; l'arrêté du 7-9-2026 impose la plage calme avant fin juillet 2027 ; le cahier est un document officiel conservé six ans au total.

| N° | Manque | Gravité | Où |
|---|---|---|---|
| M20-73 | **Décision d'usage réel** (Q-CDT2) à prendre avant mars 2027, pour savoir si C0–C8, C2a, C11 visent le niveau production | important | §8.2 (soumis) |
| M20-74 | **Démarrage propre** à la rentrée : aucun import de cahier d'un autre outil en cours d'année ; import des **progressions** des professeurs depuis leurs fichiers (Word, PDF) par le moteur de conversion (D-005), en brouillon | confort | CDT-277 |
| M20-75 | **Charte d'usage** (tableau de tenue, visa, inspection, plage calme, IA) présentée au conseil pédagogique et au conseil d'administration (règlement intérieur, D111-5) | important | §8.2 (soumis) |
| M20-76 | **Recette** : comparer, pendant le premier mois, la tenue du cahier dans ScolIA et dans l'ancien outil (si l'établissement en a un) | confort | §27 |

### Manques transverses : mobile, tests, performance

| N° | Manque | Gravité | Où |
|---|---|---|---|
| M20-77 | **Mobile d'abord pour l'élève** : dépôt photo → PDF en trois gestes, accusé lisible hors ligne, agenda hors ligne | important | C11 |
| M20-78 | **Performance** : agenda d'un élève en < 300 ms (40 cours, 400 travaux/an) ; tableau de suivi en < 500 ms pour 35 élèves | confort | §27 |

---

## 5. Vérification réglementaire (2026-11-11)

La passe n'a pas ajouté de source. Elle s'appuie sur le §5 de la conception, en particulier : **R-03** (tableau de charge obligatoire), **R-06** (un an actif puis cinq ans d'archive), **R-14** (plage calme opposable, dix mois), **R-15** (IA à partir de la 4ᵉ, usage non autorisé = fraude, accès anonymisé), **R-16** (règlement sur l'IA, haut risque pour l'évaluation des acquis), **R-21** (information égale des deux parents, refus motivé). Restent **non vérifiés** et donc des recommandations : accès de l'AESH, durée des copies d'élèves, régime de l'élève majeur, SAPAD (aucun texte lu sur l'accès des enseignants du SAPAD à l'ENT, [NV]).

---

## 6. Manques qui relèvent d'autres socles

| Socle | Manques | Demande |
|---|---|---|
| 6 Personnels | M20-11, M20-17 | lecture préparatoire d'une suppléance nommée (avant `du`) ; intervenants `assistant` / `intervenant` avec droit de contribution (Q-CDT9) |
| 10 EDT | M20-07, M20-43 | séance ajoutée pour un DS hors cours ; période de PFMP lisible par élève |
| 13 Discipline | M20-42 | demande de travaux pour une exclusion temporaire, sans motif |
| 15 Remplacements | M20-16 | lire l'entrée signée du remplaçant comme preuve de service |
| 21 Évaluations | M20-07, M20-10 | évaluation avec plage horaire hors séance, comptée dans la charge |
| 35 Accompagnement | M20-28, M20-31 | port `amenagements_utiles` et application par défaut ; AESH mutualisée à la date |
| 38 Examens | M20-08 | épreuves dans l'agenda et la charge ; semaines protégées |
| 39 Stages | M20-45 | frontière rapport (20) / convention et évaluation (39) |
| 8 Notifications | M20-36, M20-51 | résumé hebdomadaire par responsable et par langue ; langage simple |
| 11 Mobile | M20-77 | dépôt photo, hors ligne |
| 1 Socle de sécurité | M20-67 | révocation des jetons aux changements de droits |

---

## 7. Synthèse

### 7.1 Manques par gravité
| Gravité | Nombre | Numéros |
|---|---|---|
| bloquant | 3 (en trois points) | M20-01 (A), M20-33 (B), M20-61 (C) |
| important | 46 | M20-06, 07, 08, 11, 12, 14, 15, 17, 19, 20, 22, 23, 25, 26, 28, 29, 31, 34, 35, 37, 40, 41, 42, 43, 44, 47, 49, 50, 52, 53, 55, 56, 58, 60, 62–70, 73, 75, 77 |
| confort | 27 | les autres |
| différenciant | 2 | M20-16, M20-38 |

### 7.2 Ajouts dans la conception (`20_CAHIER_DE_TEXTES.md`, §32)
- **§32.1** bloquants A, B, C et lot **C2a** ; **§32.2** actions CDT-250 à CDT-289 ; **§32.3** règles R-C29 à R-C44 ; **§32.4** cas limites CC-68 à CC-90 ; **§32.5** modèle : `ent_cdt_acces_tiers` (remplace `ent_cdt_acces_inspection`), `ent_cdt_aide_etude`, `ent_cdt_projet`, `ent_cdt_declaration_ia`, `ent_cdt_periode_vigilance`, retrait de `ent_cdt_vu` ; **§32.6** lots modifiés ; **§32.7** réglages S20-12 à S20-15 ; **§32.8** questions Q-CDT13 à Q-CDT20.
- Corrections ponctuelles : §0 (point « ajout personas ») ; §10.3 et §19.2 (copie annotée lisible des responsables habilités) ; §28 (S20-08 tranché, S20-03 fixe).

### 7.3 Les dix ajouts les plus importants
1. Plage calme livrée tôt (C2a), conforme à l'arrêté du 7-9-2026 (A).
2. Toute modification publiée visible de tous les lecteurs, versions consultables (B).
3. Corrigés jamais envoyés au client (C).
4. Aménagements appliqués par défaut, pastille levable.
5. Accès tiers en lecture (inspection, SAPAD, hôpital, CNED), avec accord des responsables pour un élève.
6. Lecture préparatoire et reprise des copies en attente pour le suppléant.
7. Projets pluriannuels.
8. « Travaillé en étude » pour Devoirs faits et l'internat.
9. Traduction et fiche papier pour les familles éloignées du numérique.
10. Suppression du « vu » et AIPD.

### 7.4 Ce que la passe a montré sur la conception
La conception tenait la sécurité et la continuité ; elle sous-estimait trois choses : la **valeur juridique** du document (une version vue doit pouvoir être montrée), le **calendrier réglementaire** (plage calme opposable en 2027), et les **adultes hors du cours** qui aident l'élève (étude, AESH, SAPAD), qui ont besoin de lire sans voir les copies.

---

## 8. Décisions : appliquées par défaut (D-031) et soumises à Florian

### 8.1 Appliquées par défaut (D-031)
1. Bloquants A, B, C selon leur recommandation (§0).
2. S20-08 : copie annotée rendue lisible des responsables habilités ; dépôts en cours non lisibles.
3. Suppression du « vu » (`ent_cdt_vu`) ; Q-CDT11 devient « non ».
4. Accès tiers généralisé ; export figé en V1 pour l'inspection (Q-CDT10 inchangé), accès direct en C8.
5. Aménagements appliqués par défaut.
6. Visa d'inspection.
7. « Travaillé en étude » non visible des familles par défaut.
8. Aucune échéance pendant une PFMP de l'élève.
9. Déclaration d'usage de l'IA facultative, jamais d'outil de détection.
10. Statistiques d'exercices par exercice seulement, jamais de profil d'élève.
11. Vidéo désactivée par défaut dans les dépôts.

### 8.2 Soumis à Florian (engagent l'établissement à l'extérieur, ou faits que seul Florian connaît)
| N° | Point | Recommandation |
|---|---|---|
| Q-CDT2 | Cahier en **usage réel** pendant 2027-2028 ? Quel outil aujourd'hui ? | usage réel, si aucun autre cahier numérique officiel |
| Q-CDT3 | Durées de conservation des copies (aucun texte) | fin de l'année scolaire suivante ; **DPO** |
| Q-CDT14 | **AIPD** du socle 20 | à mener avec le DPO avant l'usage réel |
| Q-CDT15 | **Charte d'usage** au conseil pédagogique et au conseil d'administration ; mention au règlement intérieur (D111-5) | oui, avant l'usage réel |
| Q-CDT16 | Accès des enseignants du **SAPAD** et de l'hôpital : convention avec le service académique | à demander par l'établissement |
| Q-CDT17 | Fournisseur de **traduction** des consignes (cœur IA) | même fournisseur que le cœur IA, contrat européen ; Mistral à étudier (CLAUDE.md) |
| Q-CDT1 | Données du module Next.js à reprendre ? | non (ébauche) ; confirmation |
| NX-CDT-1 | Défaut Next.js : écriture sans contrôle de rôle | signalé ; Next.js intouchable, action de Florian |

---

## 8 bis. Réglages signalés (D-023)

| Réglage | Défaut | Commentaire |
|---|---|---|
| S20-12 Périodes de vigilance | aucune | posées ponctuellement par le PP ou la direction ; pas un réglage permanent |
| S20-13 Dépôts vidéo | désactivés | ouverts par travail |
| S20-14 Plafond par remise | 20 dépôts, 100 Mo cumulés | technique ; non affiché aux professeurs |
| S20-15 « Travaillé en étude » visible des familles | non | à ouvrir seulement si l'établissement le demande |
| S20-03 | — | **retiré** (imposé par la circulaire : fixe) |

---

## 9. Faits, hypothèses, contradictions

**Faits [F]** : §3 (Next.js lu, lignes citées ; matrice Charlemagne). **Vérifié [V]** : §5 de la conception. **Hypothèses [H]** : pratiques de terrain des personas, volumes, performances visées.
**Contradictions relevées** (écrites au §30 de la conception) : (1) §9.1 « modification mineure sans avis » contre la valeur juridique (R-01) : résolue par R-C29 ; (2) §19.2 « fichiers si S20-08 » contre §10.3 « retour visible » : résolue par R-C37 ; (3) Q-CDT11 (« vu » gardé) contre la minimisation : renversée ; (4) §14.2 (inspection lit, ne vise pas) contre R-02 (l'inspection vise) : corrigée ; (5) socle 6 R-P3 (droits à la prise de poste) contre M20-11 (lecture avant) : lecture seule anticipée proposée au socle 6.
