# Passe « personas » sur le socle 10 — Emploi du temps

**Date :** 2026-11-09 — **Statut :** proposition, **non décidée** (à relire par Florian). Aucun code écrit ni modifié ; aucun accès à une base, à un secret ni à un service de production ; l'application Next.js n'a pas été ouverte ; aucun code de FET (AGPL-3.0) ni d'un autre logiciel n'a été lu ou repris.
**Branche :** `ent-personas-socle-10` (base `ent-mc0hoe` `c2bebb0`, qui porte `10_EMPLOI_DU_TEMPS.md`).
**Éprouve :** `docs/socles/10_EMPLOI_DU_TEMPS.md` (lu en entier), avec `CLAUDE.md`, `docs/PILOTAGE.md`, `docs/DECISIONS.md` (D-021 à D-030, en particulier **D-030** : ScolIA construit lui-même l'EDT, génération automatique avec IA, vœux des enseignants et choix des élèves ; **D-025**), `docs/MATRICE_CHARLEMAGNE.md` §3.6 (EDT-01 à EDT-17), `docs/socles/05_STRUCTURE.md` et `06_PERSONNELS.md` (parties utiles : barrettes, offre et choix, inclusions, AESH, services partagés, temps partiel, planning de service), `PERSONAS_08.md` (méthode et forme reprises).
**Fait du terrain** (Florian, 2026-11-09, rappelé au §13 de la conception) : l'établissement pilote construit aujourd'hui son EDT **dans Charlemagne**.
**Ajouts :** faits **directement** dans `10_EMPLOI_DU_TEMPS.md`, marqués **« ajout personas »** : §0 point 9 ; §5 (R-22 à R-33 et logiciels du marché) ; §6.2 (cinq ports) ; §8.3 bis (AESH) ; §8.4 (versions temporaires) ; §10 (CL-64 à CL-82) ; §11.2 (H-13 à H-17, G-15 à G-19) ; §11.5 ; §11.6 (travail à plusieurs) ; §11.7 (barrettes et choix des élèves) ; §11.8 (réparation) ; §11.9 (lycée professionnel) ; §12.5 (vœux) ; §14.4 (diffusion) ; §15.5 bis (pièges de droits) ; §16.8 (EDT-200 à EDT-238) ; §17 (R-E21 à R-E39) ; §18.3 ; §19 (note) ; §20.1 (IA) ; §21.6 (modèle) ; §23.3 (lots, dont T14 à T16) ; §25 (S-90 à S-99) ; §26 (Q-EDT17 à Q-EDT25) ; §27 (C-10 à C-18). **Rien n'est supprimé** ; quand un ajout contredit le texte d'origine, la contradiction est écrite au §27.
**Numérotation :** les manques continuent `PERSONAS_09.md` (qui s'arrête à M-496) : **M-497 et suivants**. Les réglages continuent le socle 10 (S-80 à S-89) : **S-90 à S-99**.

---

## 0. À lire en premier : ce qu'il faut trancher avant T1 et T8a

La conception est solide sur la journée réelle (séance stable, modification motivée, « demain », lots) et sur les principes du moteur (déterministe, explicable, reprenable, comparable). En la faisant vivre par celles et ceux qui construisent, subissent ou consultent l'EDT, trois points **bloquants** apparaissent, et trois décisions importantes changent l'ordre des lots.

| # | Point | Pourquoi | Ce que la conception doit faire | Gravité | Lots |
|---|---|---|---|---|---|
| A | **Les choix des élèves ne sont qu'une entrée figée du moteur.** Groupes et barrettes viennent tout faits du socle 5 (H-01, H-04) | D-030 demande de tenir compte des choix des élèves ; en lycée, les barrettes de spécialités **se conçoivent** à partir des combinaisons choisies, et les élèves se répartissent entre groupes **après** l'EDT ; EDT (Index) annonce des groupes de spécialités indépendants des classes et un contrôle des heures par élève [V] : sans cela, pas de parité (P01, P09) | §11.7 : proposer des barrettes (EDT-200), générer avec répartition (EDT-201), groupes compatibles pour un élève (EDT-214) ; le socle 5 reste propriétaire et applique (R-E21) | **bloquant** | T7, T8a, T8b ; Q-EDT17 |
| B | **L'explication donnée à un professeur peut nommer la contrainte d'un collègue.** « Votre mardi est pris parce que M. Z n'est disponible que ce jour-là » | Fuite sur un service partagé, un temps partiel (parfois thérapeutique), un vœu ; le format d'explication (JSONB) se fige au prototype (P06) | R-E31 : explications **typées** par source et visibilité dès T8a, filtre serveur pour l'écran, le retour de vœux et l'IA ; R-E23 : explication figée à la publication | **bloquant** | T8a, T8b, T12e |
| C | **Le modèle de version ne sait pas revenir au plan.** Une seule plage par version, exclusion entre versions publiées | Suppléant libre trois jours sur cinq pendant trois mois, périodes de PFMP, semaine de voyage, salle fermée trois semaines, cycles d'EPS : il faudrait recopier le plan de septembre en une troisième version (P14, P15, P16) | §8.4 : versions **de base** et **temporaires** (base, périmètre, plage bornée), résolution à la date, retour automatique (R-E39) ; dans le schéma **dès T1** | **bloquant** | T1, T2, T5 ; Q-EDT18 |
| D | **Le démarrage du pilote** : l'établissement fait son EDT dans Charlemagne | D-030 met les imports après la saisie et la génération ; un pilote qui démarre en cours d'année a besoin de son EDT **avant** (P03) | EDT-216 (saisie accélérée depuis un export Charlemagne ou à la main) placé en T1 si besoin | important | T1 ; Q-EDT19 |
| E | **L'AESH est absente** de la conception | Sa journée est l'EDT des élèves accompagnés ; une AESH mutualisée ne peut pas être dans deux salles ; elle doit être prévenue ; sa présence dans l'EDT de la classe révèle un handicap (P10) | §8.3 bis, EDT-226, H-16, R-E33 ; avec les socles 35 et 6 | important | T2, T4, T7 |
| F | **Six pièges de minimisation** dans des fonctions conçues | présence oui/non des collègues (absences répétées) ; localisation qui dit « infirmerie » ; AED qui lit l'EDT nominatif de tout élève ; groupe d'un seul élève visible dans l'EDT de classe ; codes de motif « décharge » (syndicale) et « temps partiel » (thérapeutique) ; AESH visible (P04, P05, P06, P10, P13) | R-E27, R-E29, R-E33, R-E34, R-E37, modèle AED (§15.5 bis) | important | T0 (mêmes routes que D2, D19), T4, T6 |

Le reste (réparation, lycée professionnel, ateliers à inscription, vœux en langage courant, écran du hall, accusés de lecture) est important ou différenciant, mais ne change pas le schéma des premiers lots.

---

## 1. Méthode

**Démarche.** Celle de `PERSONAS_08.md` §1 : 16 personas réalistes prises dans la consigne et dans le tableau « acteurs » du §3 de la conception, couvrant public et privé, collège et lycée général, technologique et professionnel, établissement rural à ramassage et ensemble scolaire. Pour chacune : son année (de la dotation de février à la clôture), puis ses **incidents** (absence longue, suppléant contraint, salle fermée, nouvel élève, changement d'option, grève, neige, voyage, contestation). Chaque fois qu'elle voulait faire quelque chose que la conception ne prévoit pas, ou que c'était trop long, ambigu, risqué ou frustrant, le manque est noté.

**Grille transversale** posée à chaque persona : qui construit, qui décide, qui subit ? ; ce que voit chacun (D-022, minimisation) ; ce qui se révèle par un nombre, un libellé, une absence ; l'élève **individuel** et pas seulement la classe ; le professeur **individuel** et l'équité ; ce que change une modification de mi-année ; à qui et quand on le dit ; ce que l'IA ajoute **sans** placer elle-même (D-030, §11.4) ; ce que Charlemagne et EDT/Pronote font déjà (parité, D-024).

**Couverture du marché** (D-023, D-024) : la ligne §3.6 de la matrice (EDT-01 à EDT-17) et les fonctions publiques de **Charlemagne Emploi du temps** et d'**EDT (Index Éducation)**, **lues** le 2026-11-09 sur les pages des éditeurs (§4 ; URL au §5 de la conception).

**Vérification réglementaire.** Un sous-agent de recherche a lu les textes sur Légifrance et sur l'Office des publications de l'UE (EUR-Lex refusait la lecture directe ; le BO sur education.gouv.fr répondait 403) : **9 points [V]**, 3 en partie, 3 non lus [NV]. Tout est versé au §5 de la conception (R-22 à R-33) ; les points qui changent un manque sont rappelés ici.

**Gravité** (identique aux passes précédentes).
- **bloquant** : sans lui, l'établissement contourne de façon dangereuse (fuite, décision injuste non contestable) ou ne peut pas traiter un cas réel ; ou la conception se contredit au point d'empêcher de coder.
- **important** : cas fréquent sans solution propre, perte de temps forte ou risque d'erreur ; ou fonction de Charlemagne / EDT absente.
- **confort** : améliore nettement le quotidien ; un contournement existe.
- **différenciant** : peu de logiciels du marché le font [H, comparaison avec les pages lues des éditeurs] ; argument de vente.

**Conventions.** [V] lu sur une source officielle (URL au §5 de la conception) ; [NV] non lu ; [H] hypothèse ; [R] recommandation.

**Limites.** Les personas sont construites, pas interviewées. Les pratiques de terrain (barrettes en juin, travail de juillet à domicile, écrans du hall, jours réservés des services partagés) sont des hypothèses fondées sur l'usage courant [H]. Les pages des éditeurs décrivent des fonctions, pas leur qualité.

---

## 2. Les personas en un tableau

| # | Persona | Établissement | Ce qu'elle éprouve surtout |
|---|---|---|---|
| P01 | **Mme Garnier**, adjointe chargée de l'EDT | Lycée général et technologique privé sous contrat, 1 100 élèves, 34 divisions, 95 intervenants | Barrettes de spécialités, vœux, génération de juillet, août et ses imprévus |
| P02 | **M. Roux**, chef d'établissement | Même lycée (ensemble scolaire collège + lycée) | Arbitrer les vœux, l'équité, la contestation, la décision humaine |
| P03 | **Sylvie**, secrétaire de scolarité sous Charlemagne depuis douze ans | Même lycée | Nouvel élève en octobre, changement d'option, rentrée imprimée, reprise de Charlemagne |
| P04 | **Nadia**, CPE | Collège public en éducation prioritaire, 650 élèves | Permanences, localisation, écran du hall, exclusion-inclusion |
| P05 | **Théo**, AED étudiant en master | Collège de P04 | Planning de permanence, appel de permanence, ce qu'il a le droit de voir |
| P06 | **Mme Bertin**, professeure de lettres à 80 % | Collège de P04 | Jours libérés, vœux, retour sur ses vœux, ce qu'on dit d'elle |
| P07 | **M. Okafor**, professeur de technologie en complément de service | Deux collèges à 25 km, l'un sous ScolIA | Jours réservés, trajet, chevauchements |
| P08 | **Mme Lambert**, professeure principale de 4ᵉ (maths) | Collège de P04 | Heure de vie de classe, charge de sa classe, informer les familles |
| P09 | **Inès**, élève de 1ʳᵉ aux options rares ; **Hugo**, 18 ans, terminale | Lycée de P01, élève transportée | Spécialité suivie ailleurs, car de 17 h 15, journées lourdes, majorité |
| P10 | **Léo**, élève de 5ᵉ en ULIS, et **Mme Sanchez**, son AESH mutualisée | Collège de P04 | Inclusions, SESSAD, AESH, taxi, discrétion |
| P11 | **Famille Martin**, parents séparés, trois enfants dans deux établissements | Collège de P04 et lycée de P01 | Ce qui change la journée, absence longue, fin de journée |
| P12 | **M. Dufour**, agent de maintenance ; **Mme Chen**, préparatrice de laboratoire | Lycée de P01 | Fermer une salle au bon moment ; préparer les TP |
| P13 | **Fatou**, agent d'accueil | Lycée public | « Où est-il ? », « Le professeur est-il là ? » |
| P14 | **M. Lopes**, DDFPT (ex-chef de travaux) | Lycée professionnel, bac pro MELEC et CAP, apprentis en mixité | Périodes, PFMP, ateliers, heures annuelles |
| P15 | **M. Vidal**, contractuel suppléant arrivé en novembre | Collège de P04 | Suppléance de trois mois avec contraintes, retour de la titulaire |
| P16 | **Mme Petit**, professeure d'EPS | Collège rural à ramassage | Installations municipales, cycles, verrous qui changent en septembre |

---

## 3. Parcours et manques, persona par persona

Chaque tableau donne : le manque, pourquoi il compte, sa gravité, le socle concerné, et l'ajout fait (identifiant dans `10_EMPLOI_DU_TEMPS.md`) ou le socle où il est renvoyé.

### P01 — Mme Garnier, adjointe chargée de l'EDT (lycée général et technologique privé)

**Année.** *Février* : dotation reçue ; elle veut savoir très tôt si la structure tient. *Avril* : campagne de vœux. *Mai* : les 2ᵈᵉˢ ont choisi leurs spécialités de 1ʳᵉ ; 47 triplettes différentes ; elle doit décider combien de barrettes et quels groupes ouvrir. *Juin* : répartition des services. *Juillet* : génération, chez elle, sur plusieurs jours, avec un professeur référent qui l'aide deux matinées par semaine. *Fin août* : une classe de 2ᵈᵉ ouverte (dotation ajustée), deux professeurs arrivés, un poste vacant. *Octobre* : un professeur conteste son EDT, explication à l'appui.
**Incidents.** Trois élèves MATHS-SES-LLCER italien sont bloqués par la barrette ; le lundi de Pâques, le lundi de Pentecôte et le 11 novembre font perdre cinq séances à l'option de 1 h placée le lundi ; le professeur référent et elle déplacent le même cours au même moment.

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-497 | **Proposer des barrettes** à partir des choix des élèves, élèves bloqués nommés avec l'alternative | §0 point A ; c'est la tâche la plus difficile de mai-juin en lycée [H] ; D-030 | **bloquant** | 10, 5, 3 | §11.7, EDT-200, R-E21, Q-EDT17 |
| M-498 | **Répartir les élèves** entre groupes d'un même enseignement pendant la génération | Deux groupes de SES dans deux barrettes : la répartition fait la faisabilité et l'équilibre | important | 10, 5 | §11.7 point 2, EDT-201 |
| M-499 | **Amorcer depuis l'EDT de l'an passé** (jours libérés, salles, créneaux EPS, habitudes) | Les professeurs jugent l'EDT par rapport à l'an passé ; repartir de zéro casse ce qui marchait | différenciant | 10 | EDT-202, G-15 |
| M-500 | **Travail à plusieurs** sur un scénario sans s'écraser ; commentaires | Adjointe + professeur référent + direction en lecture : pratique courante [H] | important | 10 | §11.6 note, EDT-203, R-E22, CL-79 |
| M-501 | **Assistant EDT** qui construit sans lire les commentaires des vœux | Le professeur référent est un collègue : il ne doit pas lire « garde de mes enfants » | confort | 10 | §15.5 bis, EDT-203, S-99 |
| M-502 | **Heures placées / dotation** par discipline, **HSA** par professeur | La dotation est un plafond ; deux HSA seulement imposables (R-04) | important | 10, 6 | EDT-204 |
| M-503 | **Simuler une autre grille** (séances de 50 min, récréation déplacée) | Débat récurrent en conseil pédagogique [H] ; aujourd'hui à l'aveugle | confort | 10, 5 | EDT-205 |
| M-504 | **Explication figée** à la publication | En octobre, après des modifications, le moteur ne donnerait plus la même raison qu'en juillet ; la contestation exige la raison d'origine | important | 10 | R-E23, CL-80 |
| M-505 | **Prix d'un vœu** avant arbitrage | « Ce mercredi libre coûte deux trous à la 4B » : l'arbitrage devient explicable | différenciant | 10 | EDT-206, §12.5 point 5 |
| M-506 | **File de calcul bornée** et réutilisation d'un calcul identique | Dix générations lancées par impatience saturent le serveur en juillet [H] | confort | 10 | R-E24 |
| M-507 | **Réparation** après les imprévus d'août (classe ouverte, professeur arrivé) sans régénérer | Régénérer en août casse les vœux satisfaits et la confiance ; D-030 | important | 10 | §11.8, EDT-207, lot T16 |
| M-508 | **Équité de qualité**, pas seulement de vœux ; rotation des créneaux mal aimés | Un professeur sans vœu peut hériter de tous les vendredis à 17 h | important | 10 | G-16, §12.5 point 6 |
| M-509 | **Séances perdues au calendrier** équilibrées | L'option de 1 h du lundi perd 5 séances sur 33 [H, calendrier 2026-2027] | différenciant | 10, 5 | G-17, CL-76, §11.5 |
| M-510 | **Planifier les conseils de classe** (présence des professeurs, salles) | Puzzle de décembre, mars et juin ; présence aux conseils (matrice EDT-17) | important | 10, 23 | EDT-208 |
| M-511 | **Ateliers à inscription** (AP au choix, clubs du midi) : choix classés, capacités, répartition | Ligne EDT-15 de la matrice (trou T-15) non traitée ; « choix des élèves » de D-030 | important | 10, 5, 43 | EDT-209, EDT-210, CL-81, lot T14, Q-EDT25 |
| M-512 | **Plage commune libre** (concertation des professeurs de 3ᵉ) comme contrainte | Une réunion d'équipe impossible à caser après coup | important | 10, 19 | H-13 |
| M-513 | **Sauvegarde continue** d'un travail long et à domicile | Perdre une journée de retouches à cause d'une coupure | confort | 10 | §11.6 note |

### P02 — M. Roux, chef d'établissement

**Année.** *Mars* : il présente l'organisation de la semaine (grille, mercredi après-midi) ; *avril* : il publie la règle des vœux ; *juillet* : il arbitre trois demandes d'indisponibilité non statutaires ; *septembre* : un professeur dénonce un « EDT de punition ». Il se demande si un moteur qui « décide » des services pose un problème juridique.

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-514 | **Décisions d'organisation du temps** tracées (objet, instance, date) | Dans le public, le CA fixe « les règles d'organisation de l'établissement » (R-28 [V]) ; la trace répond aux contestations | confort | 10, 7 | EDT-211, R-28 |
| M-515 | **Règle d'équité publiée** avant la campagne, indicateurs agrégés sans nom après | Une règle connue évite le soupçon de favoritisme | important | 10 | EDT-212, §12.5 point 6, S-97 |
| M-516 | **Comparer avec l'EDT de l'an passé** sur les indicateurs | « Est-il meilleur ? » est la première question du conseil pédagogique [H] | confort | 10 | §11.5 |
| M-517 | **Demander la révision** de son EDT, réponse tracée | RGPD art. 22 : contestation et intervention humaine (R-27 [V]) | important | 10 | EDT-213, CL-80 |
| M-518 | **Aucun trait personnel** dans le moteur ; évaluation documentée au titre du règlement sur l'IA | Annexe III 4 b) : attribuer des tâches sur des caractéristiques personnelles est à haut risque ; le profilage l'est toujours (R-26 [V]) | important | 10, 1 bis | R-E25, §12.5 point 9 |
| M-519 | **Parité minimale de juillet** écrite : ce que font Charlemagne et EDT doit exister au premier pilote | Charlemagne Emploi du temps annonce placement automatique, souhaits et contraintes, salles ; EDT, diagnostic expliqué et contrôle des heures par élève [V] | important | 10 | §5 (logiciels), §4 ci-dessous |
| M-520 | **Ancienneté** comme critère de priorité des vœux | Pratique locale possible [H] ; critère personnel et conflictuel | confort | 10 | Q-EDT20 (non recommandé) |

### P03 — Sylvie, secrétaire de scolarité (sous Charlemagne depuis douze ans)

**Année.** *Fin août* : 34 classes de listes et d'EDT à imprimer pour les familles sans internet ; vingt appels « quand verra-t-on l'EDT ? ». *Octobre* : un élève arrive de Lyon avec latin et allemand LV2 ; *novembre* : une élève abandonne l'espagnol pour l'italien. Elle se demande comment l'EDT de cette année, fait dans Charlemagne, arrivera dans ScolIA si le pilote démarre en janvier.

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-521 | **Groupes compatibles** pour un nouvel élève avec l'EDT publié | Aujourd'hui, elle teste à la main les groupes jusqu'à ne plus avoir de conflit [H] ; D-030 « nouvel élève » | important | 10, 5, 3 | EDT-214, port `groupes_compatibles` |
| M-522 | **Effets d'un changement d'option** montrés avant validation (conflit, groupe complet) | Le changement casse parfois un autre cours de l'élève | important | 10, 5 | EDT-214, §11.8 |
| M-523 | **Lot de rentrée imprimé** : l'EDT de chaque élève, avec ses groupes | Familles sans internet ; l'EDT de classe ne dit pas quel groupe suit l'élève | confort | 10, 7 | EDT-215 |
| M-524 | **Date de diffusion annoncée** aux familles | Évite les appels de fin août | confort | 10, 8 | §14.4 point 8 |
| M-525 | **Reprise de l'EDT de Charlemagne** si le pilote démarre en cours d'année | §0 point D ; D-030 place les imports après | important | 10, 9, 28 | EDT-216, Q-EDT19, C-17 |

### P04 — Nadia, CPE (collège en éducation prioritaire)

**Année.** *Chaque matin* : trois absences de professeurs, deux classes en permanence, des AED à placer. *Novembre* : un élève en exclusion-inclusion pendant trois jours. *Janvier* : une mère se présente à l'accueil ; l'élève est chez l'assistante sociale. *Mars* : elle découvre qu'un AED a consulté vingt fois la localisation de la même élève.

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-526 | **Besoins de surveillance par heure** fournis au planning des AED | Le planning de service (06 PRS-133) ne sait pas combien d'élèves arrivent en permanence | important | 10, 6, 12 | port `effectifs_attendus` |
| M-527 | **Écran du hall** des changements du jour, sans nom ni motif | Pratique répandue [H] ; sans cadre, l'écran affiche « M. X absent » | important | 10 | EDT-217, R-E26, Q-EDT22 |
| M-528 | **Exclusion-inclusion** : élève présent mais retiré de ses cours | Sans cela, l'appel le compte absent et « maintenant » l'envoie en cours | confort | 10, 13, 12 | CL-64 |
| M-529 | **Localisation sans révéler un service** de santé ou social | Dire « chez l'assistante sociale » à l'accueil devant un parent est une fuite | important | 10, 14, 34, 36 | R-E27, CL-78 |
| M-530 | **Relevé des lectures de localisation atypiques** | La journalisation ne protège que si quelqu'un la lit | important | 10 | EDT-218 |

### P05 — Théo, AED étudiant

**Année.** *Chaque jour* : deux heures de permanence, une heure de cour, la demi-pension ; il fait l'appel de permanence sur une feuille. *Le mardi* : il a cours à l'université. Il se demande pourquoi il peut ouvrir l'EDT de n'importe quel élève.

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-531 | **Liste des élèves attendus** en permanence par heure, pour l'appel | L'appel de permanence est une obligation de prise en charge ; la liste se calcule depuis les séances annulées ou non assurées | important | 10, 12 | EDT-219 |
| M-532 | **Indisponibilités d'étudiant** dans le planning de service des AED | Planning annualisé (R-25 [V]) ; ses cours sont fixes | confort | 6 | renvoyé (PRS-133) |
| M-533 | **AED sans lecture nominative** de tout élève : localisation journalisée et permanence seulement | Minimisation ; un AED n'a pas besoin de parcourir des EDT d'élèves | important | 10 | §15.5 bis, Q-EDT23, C-10 |

### P06 — Mme Bertin, professeure de lettres à 80 %

**Année.** *Mars* : elle demande son temps partiel (avant le 31 mars, R-23 [V]) avec mercredi et vendredi après-midi libérés ; *avril* : la campagne de vœux lui demande de les « saisir » ; *septembre* : on lui place 15 h pour un service de 14 h ; son vœu « pas de première heure le lundi » n'est pas satisfait, et l'explication cite le service partagé d'un collègue.

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-534 | **Jours de temps partiel = décision**, pas vœu ; hors budget | R-23 [V] : autorisation annuelle ; les saisir comme vœux mélange le droit et la préférence | important | 10, 6 | R-E28, §12.5 point 1, C-14 |
| M-535 | **Codes de motif non révélateurs** (thérapeutique, syndical) | Temps partiel thérapeutique (R-24) et appartenance syndicale (donnée sensible) | important | 10 | R-E29, §12.5 point 2 |
| M-536 | **Heures placées = service** à temps partiel | Heures entières (R-23 [V]) ; heures supplémentaires à temps partiel [NV, R-31] | important | 10, 6 | H-14, CL-82 |
| M-537 | **Vœux en langage courant** structurés par l'IA, raison privée non conservée | Plus simple et plus juste que neuf cases ; minimisation | différenciant | 10, 1 bis | EDT-220, R-E30, §20.1 |
| M-538 | **Explication filtrée** : jamais la contrainte nommée d'un collègue | §0 point B | **bloquant** | 10 | R-E31, §12.5 point 7 |

### P07 — M. Okafor, professeur en complément de service sur deux collèges

**Année.** *Avril* : les deux principaux se renvoient la balle : chacun attend l'EDT de l'autre. *Juillet* : le collège B (hors ScolIA) envoie son EDT en août. *Septembre* : il finit à 11 h 55 au collège B et commence à 13 h 30 ici, 25 km plus loin.

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-539 | **Jours réservés tôt** pour le service partagé, provisoires puis confirmés | Sans accord préalable, les deux EDT se bloquent mutuellement [H] | important | 10, 6 | EDT-221, §12.5 point 3 |
| M-540 | **Ses deux EDT ensemble** (lui seul) ; agenda de l'établissement hors ScolIA importé en plages occupées | D-013 ; il est le seul à voir le conflit | différenciant | 10, 6 | EDT-222 (PRS-141) |
| M-541 | **Temps de trajet** personnel autour des indisponibilités, sans lieu | H-09 ne couvre que les sites de l'établissement | important | 10 | EDT-223, CL-65 |

### P08 — Mme Lambert, professeure principale de 4ᵉ

**Année.** *Septembre* : l'heure de vie de classe tombe un jeudi à 17 h, une semaine sur quatre ; *novembre* : la semaine du 17, sa classe a trois contrôles le mardi et une sortie le jeudi ; *janvier* : une mère l'appelle pour un cours annulé dont elle n'a rien su.

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-542 | **Plage commune** classe + PP pour la vie de classe | 10 h par an au moins en collège (R-12 [V]) ; impossible après coup | confort | 10 | H-13 |
| M-543 | **Charge de la semaine** d'une classe (heures, évaluations, sorties) | « Charge des élèves » de D-030 ; la surcharge vient de l'addition des socles | important | 10, 20, 21, 42 | EDT-224, §20.1 |
| M-544 | **PP informée** des annulations de sa classe | Elle répond aux familles | confort | 10, 8 | §14.4 point 5 |

### P09 — Inès (1ʳᵉ, options rares) et Hugo (18 ans)

**Année d'Inès.** Spécialités maths, physique-chimie et LLCER italien, cette dernière suivie au lycée voisin par convention le mardi après-midi ; latin en option ; section européenne. Elle prend le car de 17 h 15. *Octobre* : on lui place le latin le jeudi de 17 h à 18 h. Ses journées les plus longues font 9 h ; l'indicateur de sa classe est « bon ».
**Hugo** a 18 ans en février ; ses parents reçoivent toujours les changements d'EDT.

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-545 | **Cours externe** (convention, autre établissement) à placement imposé, trajet | Spécialités rares mutualisées [H] ; sans cela, conflit permanent | important | 10, 5 | EDT-225, CL-66 |
| M-546 | **Contraintes de transport** des élèves (dernier car) | Rural : un cours après le car = élève absent ou parent qui fait 40 km [H] | important | 10, 43 | H-15, G-19, CL-67, S-95 |
| M-547 | **Charge individuelle** des élèves aux options cumulées | Les moyennes par classe cachent les élèves les plus chargés ; EDT contrôle les heures par élève [V] | important | 10 | G-18, §11.5, EDT-224 |
| M-548 | **Option à distance** quand rien ne se place | Mieux qu'un renoncement à l'option | confort | 10 | CL-68, EDT-225 |
| M-549 | **Majorité** : relecture des abonnements et notifications des parents | Le droit change à une date connue ; rien ne doit continuer par inertie | important | 10, 4 | R-E32, CL-69 |

### P10 — Léo (5ᵉ, ULIS) et Mme Sanchez, son AESH mutualisée

**Année.** Léo est inclus en maths et en EPS en 5B, suit le reste en ULIS, va au SESSAD chaque mardi de 10 h à 11 h, vient en taxi. Mme Sanchez accompagne aussi une élève de 4C. *Octobre* : la séance de maths de la 5B passe à 14 h, au moment où la 4C a français, où Mme Sanchez doit aussi être. *Toute l'année* : dans l'EDT de la 5B, les professeurs voient « Inclusion ULIS (1) ». *Février* : après une hospitalisation, Léo ne vient que le matin pendant trois semaines.

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-550 | **Planning d'AESH** dérivé des élèves accompagnés ; conflits | §0 point E ; l'AESH est absente de la conception | important | 10, 35, 6 | §8.3 bis, EDT-226, H-16 |
| M-551 | **AESH invisible** dans l'EDT de classe et des autres familles | Sa présence révèle un handicap | important | 10 | R-E33 |
| M-552 | **Prise en charge extérieure récurrente** (exclusion hebdomadaire, « absence prévue ») | Sans cela, l'appel le compte absent chaque mardi et la famille reçoit un avis | important | 10, 5, 12 | EDT-227, CL-70 |
| M-553 | **Transport adapté** signalé quand la première ou la dernière heure change | Le taxi vient quand même ; la famille doit le décommander | confort | 10, 8, 43 | CL-71 |
| M-554 | **Groupes révélateurs** affichés de façon neutre | « Inclusion ULIS (1) » désigne Léo à toute l'équipe de la 5B et à l'écran | important | 10, 5 | R-E34, CL-77, S-93 |
| M-555 | **Reprise progressive** temporaire (matinées) | Aujourd'hui, des « absences » saisies à la main pendant trois semaines [H] | confort | 10, 5 | CL-72 |

### P11 — Famille Martin (parents séparés, trois enfants, deux établissements)

**Année.** La mère travaille en horaires décalés ; le père a les enfants une semaine sur deux. *Novembre* : le professeur de physique de l'aînée est absent trois semaines : « cours non assuré » chaque lundi, sans autre information. *Toute l'année* : des notifications de changement de salle à 14 h qui ne changent rien pour eux, mais pas toujours celle qui dit que le cadet finit à 15 h.

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-556 | **État du remplacement** d'une absence longue, sans motif | La répétition sans explication nourrit les plaintes ; jamais le mot « absent » (R-11 [V]) | important | 10, 15, 8 | EDT-228, R-E35, §14.4 point 2 |
| M-557 | **Fin de journée réelle** mise en avant | La question des parents est « à quelle heure sort-il ? » | confort | 10, 14 | EDT-229 |
| M-558 | **Accessibilité** : couleurs de matières lisibles pour un daltonien, lecture d'écran | L'EDT est une grille colorée ; un élève malvoyant doit pouvoir le lire | confort | 10 | §19 note |
| M-559 | **Notifier ce qui change la journée**, résumer le reste | Moins de messages, plus d'attention ; parents en horaires décalés | important | 10, 8 | R-E36, §14.4 point 1, Q-EDT21, S-98 |
| M-560 | **iCal ≠ alerte** : un changement de dernière minute passe par une notification | Les agendas abonnés se rafraîchissent avec retard [H] | confort | 10, 9 | §14.4 point 7 |

### P12 — M. Dufour (maintenance) et Mme Chen (préparatrice de laboratoire)

**Année de M. Dufour.** Il doit refaire le sol de la salle 12, deux jours. Il appelle l'adjointe, qui cherche une semaine « où ça gêne le moins ».
**Année de Mme Chen.** Elle prépare chaque soir les TP du lendemain d'après un tableau papier ; elle apprend un déplacement de TP en voyant la salle vide.

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-561 | **Meilleure période** pour fermer un lieu | Choix fait au jugé aujourd'hui [H] | différenciant | 10, 5, 44 | EDT-230, port `periode_optimale` |
| M-562 | **Planning des TP** par laboratoire avec matériel demandé, changements notifiés | La préparation dépend de l'EDT ; sans liste d'élèves (portée « mes lieux ») | important | 10, 20, 44 | EDT-231, §15.5 bis |

### P13 — Fatou, agent d'accueil (lycée public)

**Année.** Un parent demande à voir M. Martin ; elle voit « absent » sur son écran, chaque lundi depuis un mois, et le dit.

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-563 | **Présence d'un personnel** en lecture unitaire, à l'instant, « disponible / non disponible », sans historique | L'historique révèle des absences répétées (et parfois une maladie) ; R-11 [V] | important | 10, 14 | R-E37, §15.5 bis, C-11 |

### P14 — M. Lopes, DDFPT (lycée professionnel)

**Année.** Bac pro MELEC et CAP, apprentis en mixité dans deux classes. *Juin* : il fixe les PFMP de six classes pour que l'atelier ne soit ni vide ni saturé ; *septembre* : le volume d'électrotechnique est dû sur l'année, pas chaque semaine ; *octobre* : pendant les PFMP d'une classe, ses professeurs doivent visiter les élèves en entreprise. L'atelier a 12 postes.

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-564 | **EDT par périodes**, heures dues sur l'année | PFMP de 16 à 20 semaines selon la spécialité (R-29 [V]) ; H-12 hebdomadaire ne tient pas | important | 10, 39 | §11.9, EDT-232, R-E38, C-13, lot T15 |
| M-565 | **Échelonner les PFMP** pour équilibrer les ateliers | L'atelier vide une semaine, saturé la suivante [H] | différenciant | 10, 39 | EDT-233 |
| M-566 | **Visites de suivi** sur les heures libérées | Obligation de suivi ; plages libérées sans usage tracé [H] | important | 10, 39 | EDT-234 |
| M-567 | **Capacité par postes** d'un atelier | Effectif > postes : sécurité et pédagogie | important | 10, 5 | H-17 |
| M-568 | **Mixité apprentis / scolaires** aux rythmes différents | Séances créées pour des absents, ou appel faux | important | 10, 5, 39 | CL-73 |
| M-569 | **Portée « mes formations »** pour le DDFPT | Il gère ateliers et périodes, pas tout l'EDT ; missions du DDFPT [NV, R-32] | important | 10 | §15.5 bis |

### P15 — M. Vidal, contractuel suppléant arrivé en novembre

**Année.** Il remplace Mme X trois mois ; il est libre lundi, mardi et jeudi seulement (autre complément ailleurs). L'adjointe doit replacer douze séances par semaine, puis tout remettre en février au retour de Mme X — qui revient finalement deux semaines plus tôt.

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-570 | **Version temporaire bornée** avec retour au plan | §0 point C | **bloquant** | 10 | §8.4, R-E39, CL-74, Q-EDT18, C-12 |
| M-571 | **Réparation** proposée par le moteur (moins de séances déplacées, personnes prévenues) | D-030 « réparation automatique après un changement » | important | 10, 15 | §11.8, EDT-207, lot T16 |
| M-572 | **Retour anticipé** de la titulaire, avec aperçu et message aux seules personnes touchées | Sans cela, double travail et messages inutiles | important | 10, 8 | R-E39 |

### P16 — Mme Petit, professeure d'EPS (collège rural)

**Année.** Piscine au premier trimestre, gymnase au deuxième, stade au troisième ; la commune revoit ses créneaux le 15 septembre.

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-573 | **Lieu par période** et verrous municipaux qui changent → réparation | Régénérer pour l'EPS casse tout le reste | important | 10, 5 | EDT-235, CL-75, §11.8 |

### Manques transverses : l'IA et les changements

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-574 | **Résumé « demain » par l'IA** avec propositions de réparation à valider | Le matin de trois absences, l'adjointe veut une proposition, pas une liste | différenciant | 10, 1 bis, 15 | EDT-236, §20.1 |
| M-575 | **Anomalies** d'un scénario signalées (même matière toujours en dernière heure, journée de 9 h d'un élève) | Ce que l'adjointe cherche à l'œil pendant des heures [H] | différenciant | 10, 1 bis | EDT-237, §20.1 |
| M-576 | **Message aux familles d'un lot** rédigé par l'IA, motif public seulement, envoi confirmé | Matin de neige : gagner dix minutes sans fuite | confort | 10, 8, 1 bis | §20.1 |
| M-577 | **Garde-fous de l'IA** : filtre d'explication, texte libre non conservé, fournisseur approuvé, jeu d'évaluation étendu | Les pièges de révélation passent aussi par l'IA (« qui bloque mon lundi ? ») | important | 10, 1 bis | §20.1, R-E30, R-E31 |
| M-578 | **Accusé de l'intervenant** d'un changement de moins de 24 h ; alerte sinon | « Personne n'a prévenu le remplaçant » est l'incident type d'un matin [H] | important | 10, 8 | EDT-238, §14.4 point 4, S-96 |

---

## 4. Couverture du marché : Charlemagne, EDT et Pronote (D-023, D-024)

Pages lues le 2026-11-09 (URL au §5 de la conception) : **Charlemagne Emploi du temps** [V] ; **EDT (Index Éducation)** et son cahier des nouveautés 2025 [V]. Le reste est la matrice §3.6.

| Fonction attendue | Source | Avant la passe | Après la passe | Où |
|---|---|---|---|---|
| Placement automatique ou assisté | Charlemagne [V], EDT [V] | conçu (§11) | inchangé | T8a, T8b |
| Souhaits et contraintes des enseignants, indisponibilités | Charlemagne [V], EDT [V] | conçu (§12) | + temps partiel hors vœux, codes non révélateurs, vœux en langage courant, prix d'un vœu, équité de qualité | §12.5 |
| Capacité, disponibilité, salles spécialisées | Charlemagne [V], EDT [V] | conçu | + capacité par postes | H-17 |
| Analyseur de contraintes, incohérences, résolutions proposées | EDT [V] | conçu (§11.3) | + barrettes, AESH | T7 |
| **Groupes de spécialités indépendants des classes** | EDT [V] | **absent** (groupes figés) | conçu | §11.7, EDT-200, 201 |
| **Contrôle des heures par élève** | EDT 2025 [V] | partiel (par classe) | conçu (charge individuelle) | G-18, §11.5 |
| Tableau des remplacements interactif, volontaires | EDT 2025 [V], matrice EDT-09 | socle 15 (créneaux RCD ici) | inchangé | 15 |
| Import STS | Charlemagne [V] | conçu (§13.3, après spécification) | inchangé | X16 bis |
| Consultation par professeur, classe, élève, salle (EDT-01) ; A/B (EDT-02) ; exceptions (EDT-03) ; iCal (EDT-05) ; « mon planning » (EDT-16) | matrice | conçu | + accusé, écran du hall | §14.4 |
| Annulation en masse (EDT-06) | matrice | conçu | inchangé | T3 |
| Rattrapage (EDT-11) | matrice | conçu (EDT-175) | inchangé | — |
| Information des familles d'une absence, sans motif (EDT-12) | matrice | conçu | + état du remplacement | R-E35 |
| **Enseignements transversaux** (EDT-15, trou T-15) | matrice | **absent** | conçu (ateliers à inscription) | EDT-209, 210, T14 |
| Présences des enseignants aux conseils (EDT-17) | matrice | socle 23 | + planification des conseils | EDT-208 |
| Écran d'affichage des changements | pratique [H] | absent | conçu | EDT-217 |
| EDT par périodes, PFMP (lycée professionnel) | pratique [H] ; R-29 [V] | absent (H-12 hebdomadaire) | conçu | §11.9, T15 |

**Ce qui rendrait ScolIA nettement meilleur** (différenciants retenus) : la réparation en un geste avec retour automatique (M-570 à M-572), le prix d'un vœu (M-505), l'amorce depuis l'an passé (M-499), les séances perdues au calendrier (M-509), les vœux en langage courant (M-537), les deux EDT d'un service partagé vus par le seul intéressé (M-540), la meilleure période pour fermer une salle (M-561), l'échelonnement des PFMP (M-565), le résumé « demain » et les anomalies par l'IA (M-574, M-575).

---

## 5. Manques qui relèvent d'autres socles

| Socle | Manques | Ce qu'il faudra prévoir |
|---|---|---|
| 3 Inscriptions | M-497, M-521 | Vœux de N+1 agrégés lisibles par le moteur ; inscription en cours d'année qui appelle `groupes_compatibles` |
| 4 Dossier administratif | M-549 | Événement et règle de la majorité pour les abonnements d'EDT |
| 5 Structure | M-497, M-498, M-521, M-522, M-545, M-552, M-554, M-555, M-568, M-573 | Appliquer barrettes et répartitions proposées (STR-58, STR-99) ; exclusions récurrentes ; cours externes ; drapeau « dispositif sensible » sur un groupe ; calendrier d'alternance par sous-public ; lieux par période |
| 6 Personnels | M-526, M-532, M-534, M-536, M-539, M-540 | Planning de service des AED (PRS-133) lecteur de `effectifs_attendus` ; décision de temps partiel publiée (`personnel.temps_partiel_decide`) ; jours réservés d'un service partagé |
| 8 Notifications | M-544, M-553, M-556, M-559, M-572, M-578 | Règle « ce qui change la journée » pour les familles ; accusé de l'intervenant |
| 12 Appel | M-528, M-531, M-552 | Appel de permanence depuis `effectifs_attendus` ; « absence prévue » pré-remplie |
| 13 Discipline | M-528 | Exclusion-inclusion émise vers l'EDT |
| 14 Accueil, sûreté | M-529, M-563 | Libellé « pris en charge par un service » ; présence unitaire |
| 15 Remplacements | M-556, M-571 | Événement d'état du remplacement ; réparation demandée depuis une absence longue |
| 20, 21 Cahier, évaluations | M-543, M-562 | Évaluations prévues lisibles pour la charge de la semaine ; matériel de TP |
| 23 Conseils | M-510 | Port de planification des conseils |
| 35 Accompagnement | M-550, M-551 | Plages d'accompagnement de chaque AESH |
| 39 Stages et PFMP | M-564, M-565, M-566, M-568 | Calendrier des PFMP par classe et par élève ; visites ; jours réalisés (allocation, R-29) |
| 43 Transports | M-546, M-553 | Publics transportés agrégés, heures du dernier car |
| 44 Réservations | M-561, M-562 | Réservation de matériel de TP ; fermeture planifiée |
| 1 bis IA | M-518, M-537, M-574 à M-577 | Évaluation art. 6 §4 ; jeu d'évaluation ; fournisseur approuvé |

---

## 6. Synthèse

### 6.1 Manques par gravité

| Gravité | Nombre |
|---|---|
| bloquant | 3 |
| important | 50 |
| confort | 20 |
| différenciant | 9 |
| **Total** | **82** (M-497 à M-578) |

Comptage fait par script sur les tableaux du §3. Les trois bloquants sont les points A à C du §0 : **M-497** (barrettes et choix des élèves), **M-538** (explication qui nomme la contrainte d'un collègue), **M-570** (version temporaire avec retour au plan). Les points D à F du §0 sont importants (M-525, M-550, et M-529, M-533, M-535, M-551, M-554, M-563).

**81** manques sont traités par un ajout dans `10_EMPLOI_DU_TEMPS.md` ; **1** est renvoyé (M-532 au socle 6) ; M-520 est une question (Q-EDT20) avec une recommandation négative.

### 6.2 Ajouts dans `10_EMPLOI_DU_TEMPS.md`

| Partie | Ajouts |
|---|---|
| Actions métier | **39** (EDT-200 à EDT-238) ; total du socle : 125 |
| Règles | **19** (R-E21 à R-E39) |
| Cas limites | **19** (CL-64 à CL-82) |
| Contraintes | **5** dures (H-13 à H-17), **5** souples (G-15 à G-19) |
| Réglementaire | **12** lignes (R-22 à R-33 : 9 [V], 3 [NV]) + logiciels du marché [V] |
| Ports | **5** (`effectifs_attendus`, `groupes_compatibles`, `proposer_barrettes`, `seances_accompagnees`, `periode_optimale`) |
| Sections | §8.3 bis (AESH), §8.4 (versions temporaires), §11.7 (barrettes), §11.8 (réparation), §11.9 (lycée professionnel), §12.5, §14.4, §15.5 bis, §20.1, §21.6, §23.3 |
| Lots | T14 (ateliers à inscription), T15 (lycée professionnel), T16 (réparation) ; répartition dans T0 à T12 |
| Événements | §18.3 (7 émis, 6 consommés) |
| Questions | **9** (Q-EDT17 à Q-EDT25) |
| Réglages signalés | **10** (S-90 à S-99) |
| Contradictions | **9** (C-10 à C-18) |

### 6.3 Les dix ajouts les plus importants

1. **Barrettes et répartition depuis les choix des élèves** (§11.7, EDT-200, 201, 214 ; M-497, M-498, M-521). La parité avec EDT et le cœur de D-030.
2. **Versions temporaires avec retour au plan** (§8.4, R-E39 ; M-570, M-572). À poser dans le schéma de T1.
3. **Explications typées et filtrées, figées à la publication** (R-E23, R-E31 ; M-538, M-504). Retour de vœux honnête sans fuite, contestation possible.
4. **Réparation automatique** (§11.8, EDT-207, lot T16 ; M-507, M-571, M-573). L'argument de toute l'année, pas seulement de juillet.
5. **AESH** (§8.3 bis, H-16, R-E33 ; M-550, M-551). Planning dérivé, conflits, discrétion.
6. **Six pièges de minimisation fermés** (R-E27, R-E29, R-E34, R-E37, modèle AED ; M-529, M-533, M-535, M-554, M-563). À traiter avec T0.
7. **Lycée professionnel** (§11.9, lot T15 ; M-564 à M-569). Périodes, heures annuelles, PFMP, ateliers.
8. **Équité de qualité, règle publiée, aucun trait personnel** (G-16, EDT-212, R-E25 ; M-508, M-515, M-518). Sécurité sociale et juridique (règlement sur l'IA, RGPD art. 22).
9. **Charge des élèves individuelle et de la semaine** (G-18, EDT-224 ; M-547, M-543). Les élèves aux options rares cessent d'être invisibles.
10. **Familles prévenues de ce qui change leur journée, état du remplacement, accusé de l'intervenant** (R-E35, R-E36, EDT-238 ; M-556, M-559, M-578).

### 6.4 Ce que la passe a montré sur la conception

- **Le moteur est bien pensé pour l'établissement, moins pour l'individu.** Classes, versions et contraintes sont justes ; l'élève aux options rares, l'AESH, le professeur en service partagé et l'élève transporté sont les cas où l'EDT « bon en moyenne » est mauvais pour quelqu'un.
- **D-030 tire plus loin que la génération.** « Choix des élèves » veut dire barrettes et ateliers ; « réparation » veut dire versions temporaires ; ce sont des changements de modèle, pas des écrans.
- **L'explication est une donnée personnelle.** Ce qui explique un placement dit souvent quelque chose d'un collègue ; elle doit être typée avant d'être affichée, et figée pour être opposable.
- **La minimisation doit couvrir les libellés et les états.** « Absent », « infirmerie », « Inclusion ULIS (1) », « décharge » sont des informations.
- **La parité du marché est vérifiable** : les pages de Charlemagne et d'EDT confirment placement automatique, souhaits et contraintes, groupes de spécialités, contrôle des heures par élève ; ScolIA les a désormais tous dans sa conception.
- **D-023 tient** : les ajouts sont des comportements par défaut ; les dix réglages nouveaux sont signalés avec une valeur fixe (§6 bis).

---

## 6 bis. Réglages signalés (D-023)

Liste recopiée dans `10_EMPLOI_DU_TEMPS.md` §25 ; **rien n'est supprimé**. « Version automatique » = ce que fait le logiciel quand l'établissement ne touche à rien.

| # | Réglage | Pourquoi il pèse | Version automatique proposée |
|---|---|---|---|
| S-90 | Vœux : 9 natures × priorité 1 à 5 × budget × plafond | quatre dimensions pour dire « je préfère » | « 3 vœux importants + 2 souhaits » |
| S-91 | Dure/souple **et** niveau sur chaque contrainte d'établissement | deux boutons par ligne | défaut par nature, replié |
| S-92 | Modes de publication accumulés | cinq notions pour « publier » | un écran « publier » guidé |
| S-93 | Seuil des groupes révélateurs | sans enjeu local | fixe : 3 |
| S-94 | Contenu de l'écran du hall | tentation d'un éditeur | fixe |
| S-95 | Fin des cours des transportés dure ou souple | technique | souple fort |
| S-96 | Délai d'alerte d'un changement non accusé | sans enjeu local | 1 h |
| S-97 | Règle d'équité paramétrable | conflit social | une règle, publiée |
| S-98 | Changements de milieu de journée notifiés aux familles | case par famille | règle fixe R-E36 |
| S-99 | Rôle « assistant EDT » | un rôle de plus | modèle fourni |

**Existant signalé** : aucun réglage existant du socle 10 n'est jugé « usine à gaz » au-delà de S-80 à S-89 déjà signalés par la conception.

---

## 7. Faits, hypothèses, contradictions

**Faits [F]** vérifiés dans la conception : groupes et alignements pris comme entrées (H-01, H-04, §11.1) ; une seule plage par version et exclusion entre versions publiées (§21.2) ; explication en JSONB non typée (§21.2) ; « présence oui/non » des collègues pour tout le personnel (§14.3, D19) ; aucune mention de l'AESH (recherche dans le document : zéro occurrence) ; ligne EDT-15 de la matrice sans action ; H-12 hebdomadaire ; temps partiel traité comme une indisponibilité saisie avec les vœux (§12.2).

**Vérifié le 2026-11-09** [V] (§5 de la conception) : R-22 (documentalistes), R-23 (temps partiel en heures entières, R911-7), R-24 (thérapeutique, CGFP), R-25 (AED), R-26 (règlement sur l'IA), R-27 (RGPD art. 22), R-28 (organisation du temps scolaire, R421-2, R421-20), R-29 (PFMP), R-30 (code du travail) ; pages de Charlemagne Emploi du temps, d'EDT et de FET.

**À vérifier [NV]** : heures supplémentaires d'un enseignant à temps partiel (R-31) ; missions du DDFPT (R-32) ; sections sportives (R-33) ; décret 2021-997 en entier ; instance qui fixe l'organisation du temps dans le privé sous contrat ; modifications du règlement sur l'IA en 2025-2026.

**Hypothèses [H]** : pratiques de terrain (barrettes en mai-juin, triplettes, travail de juillet à domicile, écrans du hall, jours réservés des services partagés, ancienneté locale), fréquences, rafraîchissement des agendas abonnés, calendrier des lundis fériés.

**Contradictions signalées** (toutes au §27 de la conception, aucune tranchée) : C-10 (AED et lecture nominative), C-11 (présence oui/non et R-11), C-12 (versions mono-plage), C-13 (H-12 et lycée professionnel), C-14 (temps partiel vœu ou décision), C-15 (EDT-15 non traitée), C-16 (R-08 remplacée par R-23), C-17 (ordre de D-030 et démarrage du pilote), C-18 (AESH absente).

Les compléments de modèle entrent dans la réserve de migrations du socle (4012–4020, repli 452–457) ; aucune plage nouvelle n'est demandée.
