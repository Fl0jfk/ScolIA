# Passe « personas » sur le socle 12 — Appel et assiduité

**Date :** 2026-11-11 — **Statut :** proposition, **non décidée** (à relire par Florian) ; les bloquants sont tranchés selon leur recommandation (D-031). Aucun code écrit ni modifié ; aucun accès à une base, à un secret ni à un service de production ; Next.js lu en lecture seule par un sous-agent (inventaire du §2.4 de la conception), aucune modification.
**Branche :** `ent-socle-12-conception` (base `origin/ent-mc0hoe` `7467f2d`).
**Éprouve :** `docs/socles/12_ASSIDUITE.md` (lu en entier), avec `CLAUDE.md`, `docs/DECISIONS.md` (D-020, D-022, D-031, D-034, D-035, D-036), `docs/SOCLES.md` §12, `docs/VERIF_REGLEMENTAIRE.md`, les conceptions 08, 10, 16, 34, 42 et leurs passes personas (méthode et forme de `PERSONAS_10.md` et `PERSONAS_42.md` reprises).
**Ajouts :** faits **directement** dans `12_ASSIDUITE.md`, au **§29** (actions AS-200 à AS-251, règles R-AS33 à R-AS40, compléments de modèle, de lots et de migrations 2256–2257), et rappelés au §0 point 10 de la conception. **Rien n'est supprimé** ; une contradiction avec le texte d'origine est écrite au §29.6.
**Numérotation :** manques **M-2592 à M-2663** (à la suite de M-2591) ; réglages **S-483 à S-486** (à la suite de S-482 de la conception).

---

## 0. À lire en premier : quatre bloquants et leur recommandation (appliquée par défaut, D-031)

| # | Point | Pourquoi | Ce que la conception fait désormais | Gravité | Lots |
|---|---|---|---|---|---|
| A | **L'autre parent n'est jamais informé.** Avec l'avis « foyer du jour d'abord » (8 §4.1), en résidence alternée, le parent qui n'a pas l'enfant cette semaine ne sait rien des absences | Code civil 373-2-1 [V] : le parent, même sans exercice de l'autorité parentale, « doit être informé » ; les absences en font partie (guide éduscol, VERIF R11) ; c'est aussi une source de conflit (P07) | **R-AS33** : avis immédiat au foyer du jour ; **récapitulatif quotidien** à l'autre parent (U0, sans le texte du premier) ; AS-200 | **bloquant** | AS5 |
| B | **L'élève exclu de cours disparaît** entre la salle et la vie scolaire. La conception ne traite que l'exclusion-inclusion décidée à l'avance (socle 13) | Un élève mis à la porte à 10 h 05 n'est ni « absent » ni « localisé » ; s'il n'arrive pas, personne ne le sait : responsabilité de l'établissement (P03, P04) | **R-AS37** : l'exclusion ponctuelle de cours le **localise « vie scolaire, en attente »** ; arrivée confirmée par l'AED ; sinon alerte au bout de 10 min ; AS-208, AS-209 | **bloquant** | AS2, AS6 |
| C | **Le cache hors ligne d'un téléphone perdu** contient les listes nominatives du jour | Données de mineurs sur un appareil personnel (P01, P15, P16) ; la conception chiffre le cache mais ne demande pas le déverrouillage à l'ouverture | **R-AS38** : ouverture de l'appel hors ligne derrière le déverrouillage de l'appareil (biométrie ou code), effacement à distance à la révocation, liste réduite au jour ; AS-224 ; AIPD (P-9) | **bloquant** (avant AS10) | AS10 |
| D | **L'année de doublon** : Charlemagne reste le registre d'appel officiel ; les professeurs ne feront pas l'appel deux fois | D-034 ; sans réponse, l'assiduité de ScolIA sera vide ou fausse toute l'année (P06) | AS-232 (import quotidien depuis Charlemagne), AS-233 (écarts du jour) ; **décision de Florian** (P-5) | **bloquant** (décision) | AS15 |

Le reste (soins réguliers, étudiants post-bac, AESH, familles allophones, énumération par messages d'erreur, délégation de validation) est important ou différenciant et ne change pas le schéma des premiers lots, sauf la **situation récurrente** (R-AS36, migration 2256).

---

## 1. Méthode

Celle de `PERSONAS_10.md` §1 : 16 personas réalistes prises dans le tableau des acteurs (§3 de la conception), couvrant public et privé, collège, lycée général, lycée professionnel, post-bac ; pour chacune, sa journée, puis ses incidents (rentrée de vacances, grève, piscine, exclusion de cours, résidence alternée, téléphone perdu, contrôle). Chaque fois qu'elle voulait faire quelque chose que la conception ne prévoit pas, ou que c'était trop long, ambigu, risqué, le manque est noté. Deux personas **attaquent** (P15) et **auditent** (P16).

**Grille posée à chaque persona** : qui constate, qui décide, qui est informé ? ; ce que voit chacun (D-022, minimisation, santé) ; ce qui se révèle par un libellé, un nombre, une absence d'information ; l'élève individuel ; la famille réelle (séparée, allophone, sans smartphone) ; ce que change le hors ligne ; ce que l'IA ajoute sans décider ; ce que Pronote, École Directe, Charlemagne font déjà (parité) ; ce que dit le texte (§5 de la conception).

**Gravité** : **bloquant** (contournement dangereux, cas réel impossible, contradiction qui empêche de coder) ; **important** (cas fréquent sans solution propre, perte de temps forte, fonction du marché absente) ; **confort** ; **différenciant** (peu de logiciels lus le font [H]).

**Limites** : personas construites, non interviewées ; pratiques de terrain [H] ; la recherche de marché n'a pas pu lire les pages d'École Directe, de Charlemagne ni la documentation en ligne de Pronote (plaquette 2023 seulement).

---

## 2. Les personas en un tableau

| # | Persona | Établissement | Ce qu'elle éprouve surtout |
|---|---|---|---|
| P01 | **Mme Roche**, professeure d'EPS | Lycée pilote, privé sous contrat ; gymnase municipal sans réseau, piscine | Hors ligne, inaptitudes et dispenses, trajet vers l'installation |
| P02 | **M. Haddad**, contractuel remplaçant, puis co-intervenant en LP | Collège public et LP | Appel d'un collègue, co-intervention, oubli, correction |
| P03 | **Nadia**, CPE | Collège public en éducation prioritaire, 650 élèves | Lundi de rentrée de vacances, qualification, dossier, exclusion de cours |
| P04 | **Théo**, AED | Collège de P03 | File du matin, appels aux familles, permanence, élève exclu de cours |
| P05 | **Fatou**, agent d'accueil | Lycée pilote | Dix appels à 8 h, appelant inconnu, élève en sortie, mot papier |
| P06 | **Mme Duval**, cheffe d'établissement | Lycée pilote | Signalements, délégation, rapport au CA, contrôle, doublon avec Charlemagne |
| P07 | **M. et Mme Garnier**, séparés, résidence alternée | Lycée pilote, fils en 2ᵈᵉ | Qui est prévenu, deux motifs contradictoires, texte de l'autre |
| P08 | **Mme Nguyen**, mère allophone, sans smartphone | Collège de P03 | Comprendre l'avis, répondre sans application |
| P09 | **Hugo**, 18 ans, terminale ; **Sarah**, 20 ans, BTS, boursière | Lycée pilote (post-bac) | Majorité, qui est informé, bourse du CROUS |
| P10 | **Léo**, 5ᵉ ULIS, et **Mme Sanchez**, son AESH | Collège de P03 | Soins réguliers (SESSAD, orthophonie), taxi, AESH non prévenue |
| P11 | **Inès**, 4ᵉ, déléguée, PAI, souvent à l'infirmerie | Collège de P03 | Mission de déléguée, infirmerie et discrétion, notifications en cours |
| P12 | **M. Lopes**, DDFPT | Lycée professionnel, apprentis en mixité | PFMP, tuteur sans compte, employeurs |
| P13 | **Mme Petit**, infirmière (module santé désactivé, N-0) | Collège de P03 | Localiser sans motif, retour en cours, renvoi au domicile |
| P14 | **Mme Lambert**, professeure principale de 3ᵉ | Collège de P03 | Voir sa classe, préparer le conseil, alerter tôt |
| P15 | Trois **attaquants** : **Kevin**, élève de 3ᵉ ; **Mme Sorel**, mère d'une élève ; **M. Valin**, professeur d'un autre groupe ; plus un **téléphone volé** | Collège de P03 | Toutes les portes : déclarer pour soi, lire les autres, énumérer, rejouer un lot hors ligne |
| P16 | **M. Blanc**, délégué à la protection des données de l'OGEC, en **audit RGPD** | Lycée pilote | Registre, base légale, durées, santé indirecte, AIPD, droit d'accès, tiers |

---

## 3. Parcours et manques, persona par persona

Chaque tableau : le manque, pourquoi il compte, sa gravité, le socle concerné, l'ajout fait (identifiant au §29 de `12_ASSIDUITE.md`) ou le renvoi.

### P01 — Mme Roche, professeure d'EPS

**Journée.** 8 h : elle part avec la 2ᵈᵉ B au gymnase municipal (dix minutes à pied) ; pas de réseau dans la salle. 8 h 15 : appel sur son téléphone, hors ligne. Deux élèves inaptes partiels, un « mot des parents » pour Camille, une élève arrivée en retard à 8 h 25. 10 h : piscine avec la 2ᵈᵉ C ; un élève dispensé de présence reste au lycée.

| N° | Manque | Pourquoi | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-2592 | L'appel « au départ du lycée » (trajet vers l'installation) n'existe pas ; un élève qui ne part pas avec le groupe n'est constaté qu'à l'arrivée | responsabilité pendant le trajet | important | 12 | AS-220 |
| M-2593 | Poser la **dispense ponctuelle** de Camille **depuis l'appel** (mot des parents) | AS-162 existe mais hors de la feuille | confort | 12, 34 | AS-221 |
| M-2594 | Voir les **restrictions** d'une inaptitude partielle (« pas de course ») sur la feuille, pas seulement « dispensé » | sécurité de l'élève ; 34 §382 | important | 12, 34 | AS-222 |
| M-2595 | Savoir si son appel hors ligne **est parti**, et le renvoyer à la main | peur de perdre l'appel (irritant Pronote [V]) | important | 12, 11 | AS-223 |
| M-2596 | Ouvrir l'appel hors ligne sans que le téléphone soit **déverrouillé** par biométrie | téléphone posé sur un banc du gymnase | **bloquant** (C) | 11, 12 | R-AS38, AS-224 |
| M-2597 | Retard de 8 h 25 au gymnase : le billet d'entrée de la vie scolaire n'a pas de sens hors du lycée | la conception suppose le passage à la vie scolaire | confort | 12 | AS-107 suffit (retard saisi par le professeur) ; noté |

### P02 — M. Haddad, contractuel remplaçant puis co-intervenant

**Journée.** Il remplace Mme Ferrand depuis lundi ; la 4ᵉ A avait commencé l'appel à 8 h avec un AED avant son arrivée. Mardi, en LP, il co-intervient avec un professeur d'enseignement professionnel. Jeudi, il oublie deux appels.

| N° | Manque | Pourquoi | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-2598 | **Reprendre** un appel commencé par un autre (AED, titulaire) | CA-10 parle du titulaire, pas du cas inverse | important | 12 | AS-225 |
| M-2599 | En co-intervention, **savoir qui** a fait l'appel et quand | éviter deux appels contradictoires | confort | 12 | AS-226 |
| M-2600 | Demander une correction **depuis l'avis** « vous avez noté Léa absente à 8 h » quand elle arrive à 8 h 40 | geste naturel | confort | 12 | AS-227 |
| M-2601 | Un **rappel groupé** en fin de journée des appels non faits (« 2 appels à faire ») plutôt que des relances séparées | contractuel débordé | confort | 12, 8 | AS-228 |
| M-2602 | Le remplaçant ne connaît pas les **élèves** : trombinoscope utile, mais S-468 le désactive | appel juste | confort | 12 | S-468 : activable par l'établissement pour les remplaçants seulement (S-483) |

### P03 — Nadia, CPE, lundi de rentrée des vacances d'automne

**Journée.** 7 h 45 : 180 déclarations « malade » du week-end et 40 non joints. 10 h 05 : M. Durand met Yanis à la porte. 11 h : Yanis n'est jamais arrivé à la vie scolaire. 14 h : entretien avec une famille au seuil des quatre demi-journées.

| N° | Manque | Pourquoi | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-2603 | **Exclusion ponctuelle de cours** : l'élève quitte la salle, n'est attendu nulle part | sécurité, responsabilité | **bloquant** (B) | 12, 13 | R-AS37, AS-208, AS-209 |
| M-2604 | Qualifier en masse les « maladie » de la **rentrée** avec un filtre (motif, source, pas de pièce, durée ≤ 2 jours) | 180 gestes sinon | important | 12 | AS-213 |
| M-2605 | File du lundi **regroupée par famille** (fratrie : un appel, pas trois) | gain de temps | important | 12 | AS-214 |
| M-2606 | La **convocation** de l'élève à la vie scolaire visible à l'appel suivant (Pronote le fait [V]) | AS-109 sens inverse seulement | confort | 12 | AS-215 |
| M-2607 | Planifier l'**entretien** avec la famille depuis le dossier (rendez-vous, socle 19) | double saisie | confort | 12, 19 | AS-239 |
| M-2608 | Voir la **commission éducative** comme une étape du dossier, avec le document signé (13, 7) | déjà AS-175, mais l'état de la commission n'est pas relu | confort | 12, 13 | AS-175 précisé (§29.1) |

### P04 — Théo, AED

**Journée.** 8 h 30 : file du matin ; il appelle les familles ; une collègue rappelle la même mère cinq minutes après. 10 h : permanence de 30 élèves, dont 4 non prévus (professeur absent). 10 h 10 : Yanis (exclu de cours) n'arrive pas.

| N° | Manque | Pourquoi | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-2609 | **« Je prends »** sur une famille à appeler (verrou court visible des collègues) | doubles appels (irritant [V], lycée Hébert) | important | 12 | AS-210 |
| M-2610 | **Modèle de message** laissé et script court d'appel, en plusieurs langues | qualité, homogénéité | confort | 12 | AS-211 |
| M-2611 | Permanence : élèves **attendus** et élèves **venus en plus** (autre cours annulé, exclu) | effectif réel | important | 12, 10 | AS-212 |
| M-2612 | Alerte de **non-arrivée** d'un élève exclu de cours | voir M-2603 | **bloquant** (B) | 12 | AS-209 |
| M-2613 | L'AED ne doit pas voir les motifs **médicaux** des déclarations (§15.2 le dit pour les pièces, pas pour le texte) | minimisation | important | 12 | R-AS19 précisé : nature « maladie » visible, texte libre non (§29.2) |

### P05 — Fatou, agent d'accueil

**Journée.** 8 h–8 h 30 : dix appels. Un « oncle » inconnu. Une mère qui dit « il arrive à 10 h, rendez-vous dentiste ». Un père qui appelle pour un élève parti en sortie à Lyon. Un mot papier apporté par un petit frère.

| N° | Manque | Pourquoi | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-2614 | Voir, avant de déclarer, que l'élève est **en sortie** ou **déjà prévenu** | repris de Next.js | important | 12, 42 | AS-216 (écran ; AS-112 pour la règle) |
| M-2615 | Une **tâche de rappel** d'un responsable pour un « tiers non vérifié », visible de la vie scolaire | R-AS6 ne dit pas qui rappelle | important | 12, 4 | AS-217 |
| M-2616 | Déclarer un **retard annoncé** (« il arrive à 10 h ») : absence prévenue de 8 h à 10 h, puis présence attendue | cas quotidien | important | 12 | AS-218 |
| M-2617 | **Photographier** le mot papier et le joindre à l'épisode | fin du papier (G22) | confort | 12, 7 | AS-219 |
| M-2618 | L'accueil ne doit pas voir **la liste** des élèves en sortie, seulement l'élève qu'il recherche | minimisation | important | 12 | règle §15.2 confirmée (lecture unitaire journalisée) |

### P06 — Mme Duval, cheffe d'établissement

**Situation.** Elle part trois jours en formation pendant la fenêtre de signalement ; l'inspection demande le registre d'appel d'octobre ; en juin, rapport au CA ; en 2027-2028, doublon avec Charlemagne.

| N° | Manque | Pourquoi | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-2619 | **Délégation** de la validation des signalements à l'adjoint, datée (D-041 : comme les exports) | absence de la direction | important | 12, 1 | AS-229 |
| M-2620 | Tableau de bord direction : taux, appels non faits **agrégés**, signalements en cours | pilotage sans chasse à l'individu | important | 12, 47 | AS-230 |
| M-2621 | **Export du registre** d'un mois pour une inspection ou un contrôle, tracé | RA-01 | important | 12, 9 | AS-231 |
| M-2622 | **Doublon** : l'assiduité de ScolIA doit venir de Charlemagne chaque jour, ou d'un groupe pilote | sinon vide | **bloquant** (D) | 12, 9 | AS-232, AS-233 ; P-5 |
| M-2623 | Valider un signalement **depuis le téléphone** avec passkey | réactivité | confort | 12, 11 | AS-181 sur mobile (§29.4) |

### P07 — M. et Mme Garnier, séparés, résidence alternée

**Situation.** Semaine du père. Lundi, Lucas est absent : le père déclare « malade » ; la mère, qui ne sait rien, apprend l'absence par Lucas le mercredi ; jeudi, elle déclare « rendez-vous » pour vendredi ; le père conteste.

| N° | Manque | Pourquoi | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-2624 | La mère (pas le foyer du jour) n'est **jamais informée** | 373-2-1 [V] | **bloquant** (A) | 12, 8, 4 | R-AS33, AS-200 |
| M-2625 | Chacun voit **ce qu'il a déclaré** et que « l'autre responsable a déclaré : maladie », sans son texte | R-AS22 | important | 12 | AS-236 |
| M-2626 | Accusé de lecture : la vie scolaire **a vu** la déclaration | « mon mot a-t-il été reçu ? » | confort | 12 | AS-236 |
| M-2627 | Deux motifs contradictoires : la CPE voit les deux, **ne départage pas** par défaut entre les parents ; la qualification porte sur l'absence | conflit parental hors du rôle de l'établissement | important | 12 | règle §29.2 (note à R-AS5) |
| M-2628 | Le père demande que la mère ne voie plus les absences : impossible sans décision du juge | socle 4 (interdiction) | important | 4 | renvoi (déjà au socle 4) |

### P08 — Mme Nguyen, mère allophone, sans smartphone

| N° | Manque | Pourquoi | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-2629 | Avis et formulaire de motif **dans sa langue**, avec pictogrammes | comprendre « non justifiée » | important | 12, 8 | AS-234 |
| M-2630 | Répondre **par SMS** (« 1 = malade, 2 = rendez-vous ») ou par un lien sans compte | pas d'application | important | 12, 8 | AS-235 (SMS entrant : P-4) |
| M-2631 | Un courrier postal **traduit** en fin de relance | RA-04 | confort | 12, 7 | AS-234 |
| M-2632 | Être **appelée** par l'AED avec un interprète ou une phrase type dans sa langue | AS-211 | confort | 12 | AS-211 |

### P09 — Hugo (18 ans, terminale) et Sarah (20 ans, BTS, boursière du CROUS)

| N° | Manque | Pourquoi | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-2633 | Hugo voit **qui est informé** de ses absences | transparence (RGPD art. 13) | important | 12 | AS-201 |
| M-2634 | Hugo **demande** la suspension de l'information de ses parents ; décision de la direction, tracée | Q-AS9 | important | 12, 4 | AS-202 |
| M-2635 | Sarah, étudiante : ses parents ne sont **pas** des responsables ; la conception traite le post-bac comme un majeur de lycée | profil différent | important | 12, 51 | R-AS34, AS-203 |
| M-2636 | Avant toute liste au CROUS, un **justificatif est demandé** à l'étudiant, avec délai | fiche AMUE [V-ext] | important | 12, 26 | AS-204 |
| M-2637 | Hugo à 17 ans et 11 mois : la bascule le jour de ses 18 ans est **annoncée** à lui et à ses parents | CA-31 | confort | 12, 2 | AS-201 |

### P10 — Léo (5ᵉ ULIS) et Mme Sanchez (AESH)

| N° | Manque | Pourquoi | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-2638 | **Soins réguliers** (orthophonie chaque mardi 10 h, SESSAD jeudi après-midi) : une autorisation **récurrente**, pas une déclaration chaque semaine | situation fréquente en inclusion | important | 12, 35 | R-AS36, AS-205, migration 2256 |
| M-2639 | Libellé neutre « prise en charge extérieure » : jamais « orthophonie » ni « SESSAD » visible du professeur | révèle un handicap | important | 12, 35 | R-AS36 |
| M-2640 | L'AESH est **prévenue** de l'absence de Léo (déjà §14.3) **et** de son retour | organisation de sa journée | confort | 12, 8 | AS-206 |
| M-2641 | Le **taxi** adapté en retard : retards en lot par transporteur, sans compteur | cas quotidien | confort | 12, 43 | AS-207 |

### P11 — Inès (4ᵉ, déléguée, PAI)

| N° | Manque | Pourquoi | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-2642 | Ses passages fréquents à l'infirmerie ne doivent jamais devenir une **statistique** visible hors infirmerie et CPE | santé indirecte | important | 12, 34 | R-AS40 |
| M-2643 | Ses **missions de déléguée** apparaissent « instance » dans son espace et jamais au bulletin | RA-19 | confort | 12, 23 | AS-250 |
| M-2644 | Ne pas recevoir les avis destinés à l'élève **pendant ses cours** (téléphone interdit, L511-5 [V]) | 8 R-N66 | confort | 12, 8 | R-AS39, AS-249 |

### P12 — M. Lopes, DDFPT (lycée professionnel)

| N° | Manque | Pourquoi | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-2645 | Le **tuteur** en entreprise, sans compte, signale une absence par lien nominatif | sinon téléphone et papier | important | 12, 39 | AS-240 |
| M-2646 | **Récapitulatif d'assiduité** de la PFMP pour l'évaluation et l'attestation | L124-15 [V] (maladie : validation possible) | important | 12, 39 | AS-241 |
| M-2647 | Absence d'un **apprenti** en cours → information de l'employeur, sans motif médical | RA-31 | important | 12 | AS-242 (avec AS-189) |

### P13 — Mme Petit, infirmière (santé désactivée, N-0)

| N° | Manque | Pourquoi | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-2648 | **Localiser** depuis une recherche en deux gestes, sans ouvrir de module santé | N-0 (34 §223) | important | 12, 34 | AS-243 |
| M-2649 | **Fin de localisation** : « reparti en cours à 10 h 10 », avec un billet de retour pour le professeur | sinon l'élève reste « à l'infirmerie » | important | 12 | AS-244 |
| M-2650 | Renvoi au domicile **sans** module santé : absence des séances restantes, motif « santé » visible de la vie scolaire seule | 34 §349 en N-0 | important | 12, 34 | AS-147 précisé (§29.1) |

### P14 — Mme Lambert, professeure principale de 3ᵉ

| N° | Manque | Pourquoi | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-2651 | Vue **assiduité de sa classe** (demi-journées, retards, tendance), sans motifs | §15.2 le dit, aucune action | important | 12 | AS-237 |
| M-2652 | Assiduité préparée pour le **conseil de classe** (période arrêtée) | socle 23 | important | 12, 23 | AS-238 |
| M-2653 | Être **associée** au dossier (référente possible) sans tout voir | RA-12 | confort | 12 | AS-176 (déjà) ; noté |

### P15 — Les attaquants

**Kevin (3ᵉ)** veut effacer ses absences et voir celles de sa voisine. **Mme Sorel** veut la liste des absents de la classe « pour le covoiturage ». **M. Valin**, professeur d'allemand, veut savoir pourquoi un élève d'espagnol manque souvent. Un **téléphone de professeur volé** dans le gymnase.

| N° | Manque ou porte | Résultat contre la conception | Gravité | Ajout |
|---|---|---|---|---|
| M-2654 | Kevin se **déclare** absent ou se justifie | refusé (mineur, R-AS23) ; test | — | tests §25 (déjà) |
| M-2655 | Kevin **énumère** les identifiants : un corps invalide renvoie 422 avant le contrôle de droits (constat RP1) | fuite de la forme des requêtes et de l'existence | important | R-AS35 |
| M-2656 | Mme Sorel cherche les absents par l'agenda, la messagerie (« Léa était absente ? »), un avis groupé | aucun avis ne cite un autre élève ; regroupement par **foyer** seulement ; test | — | tests (déjà) |
| M-2657 | M. Valin lit l'historique ou le motif d'un élève d'un autre groupe | 403 (R-AS21, CA-69) ; journalisé | — | AS-251 (alerte sur série de refus) |
| M-2658 | Téléphone volé : cache d'appel lisible, **lot hors ligne falsifié** rejoué | R-AS32, R-AS38, révocation, droits relus, fenêtre S-469 | **bloquant** (C) | R-AS38, AS-224 |
| M-2659 | Un parent fait déclarer par l'IA « justifie toutes les absences du mois » dans un message | l'IA ne qualifie pas (§21) ; la carte proposée ne porte que sur une période explicite | — | jeu d'évaluation (déjà) |

### P16 — M. Blanc, DPO, en audit RGPD

| N° | Manque | Pourquoi | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-2660 | **Journal des consultations** sensibles (pièces médicales, dossiers, signalements) consultable par le DPO | accountability (art. 5.2) | important | 12, 27 | AS-245 |
| M-2661 | **Export du dossier d'assiduité** d'un élève pour une demande d'accès (art. 15), sans les notes d'un tiers | droit d'accès | important | 12, 27 | AS-246 |
| M-2662 | **Purge à échéance** avec rapport (justificatifs à 1 an, textes libres en fin d'année) | §15.6 sans action | important | 12, 27 | AS-247 |
| M-2663 | Données du **tiers déclarant** (« l'oncle ») : nom, qualité, numéro ; durée ; information | personne non usager | confort | 12, 4 | AS-248 |

Le DPO relève aussi, sans manque nouveau : **AIPD obligatoire** (mineurs, grande échelle, données de santé indirectes, appareil personnel hors ligne) — à mener avant le doublon (P-9 au §29.5) ; base légale des statistiques à fixer (P-6) ; la circulaire 2014-159 cite l'enregistrement électronique « dans le respect de la loi 78-17 » [V-ext].

---

## 4. Couverture du marché (D-023, D-024)

D'après la plaquette Pronote 2023 [V] et les guides d'établissements [V-ext] (sources au rapport de recherche, consultées le 2026-11-11) :

| Fonction du marché | Conception |
|---|---|
| Feuille d'appel complète (absences, retards calculés, motifs, infirmerie, convocations, observations) | AS-100 à AS-109, AS-140 |
| Appel après le cours, verrou paramétrable ~15 min après | §9.2 (jour même) ; correction tracée au lieu d'un verrou sec |
| Récapitulatif des appels non faits sur l'accueil du professeur | AS-100, AS-150, AS-228 |
| Rappel des dernières absences sur la feuille ; absence « non réglée » en rouge au cours suivant | §9.2 (« absent à la séance précédente », sans motif) ; état « à régulariser » (R-AS21) |
| Appel en sortie par les accompagnateurs | lot AS12 (composant), 42 |
| Passages à l'infirmerie, statistiques | AS-140, AS-243 ; **pas** de statistique hors infirmerie (R-AS40) |
| Absences en stage | AS-118, AS-240, AS-241 |
| Absences prévisionnelles des parents, acceptées ou refusées | AS-111, AS-125 |
| Justification en ligne (paramétrable) | AS-111, AS-126 |
| SMS et courriers types, relances, publipostage | §10.5, AS-129, AS-159 |
| Statistiques (classe, jour, heure, matière ; seuils mensuels dépassés) ; demi-journées brutes, calculées, bulletin | AS-170, AS-191, AS-192 |
| Imports d'absences de systèmes tiers | AS-199, AS-232 |
| Droit à la déconnexion | socle 8 |
| **Non documenté chez les éditeurs lus** : appel hors ligne, avis regroupé avec démenti, dossier R131-6 prérempli, profils académiques, retenue de bourse, exclusion de cours suivie | différenciants (AS10, §14, §13, R-AS37) |

---

## 5. Manques qui relèvent d'autres socles

| Manque | Socle | Renvoi |
|---|---|---|
| Restriction d'information d'un parent sur décision du juge (M-2628) | 4 | déjà couvert (interdictions) |
| Langue de la famille, SMS entrant, lien nominatif (M-2629, M-2630) | 8, 4 | natures et canaux |
| Rendez-vous d'entretien (M-2607) | 19 | port de prise de rendez-vous |
| Commission éducative et document signé (M-2608) | 13, 7 | instance et trame |
| Saisie hors ligne de l'appel dans les applis (M-2595, M-2596) | 11 | AS10 conjoint |
| Prise en charge extérieure et AESH (M-2638 à M-2640) | 35 | autorisation récurrente lue par le 35 |
| Tuteur et PFMP (M-2645, M-2646) | 39 | périodes, attestations |
| Bulletin et conseil (M-2652) | 23 | port `absences_periode` |
| Journal, purge, droit d'accès (M-2660 à M-2662) | 27 | outils transverses, contenu du 12 |

---

## 6. Synthèse

### 6.1 Manques par gravité

72 lignes (M-2592 à M-2663) : **6 manques bloquants** regroupés en quatre points (A : M-2624 ; B : M-2603, M-2612 ; C : M-2596, M-2658 ; D : M-2622), **40 importants**, **22 confort**, aucun différenciant nouveau (les différenciants sont déjà dans la conception), et **4 attaques sans défaut** (portes déjà fermées par la conception, à garder en tests : M-2654, M-2656, M-2657, M-2659).

### 6.2 Ajouts dans `12_ASSIDUITE.md` (§29)

52 actions (AS-200 à AS-251), 8 règles (R-AS33 à R-AS40), une table de situations récurrentes et une table de politique d'information des majeurs (migrations 2256, 2257), précisions sur AS-147, AS-175, AS-181 et R-AS19, place des ajouts dans les lots, 4 réglages (S-483 à S-486), 2 points pour Florian (P-9, P-10).

### 6.3 Les dix ajouts les plus importants

1. Récapitulatif quotidien à l'autre parent (R-AS33).
2. Exclusion ponctuelle de cours suivie jusqu'à l'arrivée (R-AS37).
3. Cache hors ligne derrière le déverrouillage, effacement à distance (R-AS38).
4. Import quotidien de Charlemagne et écarts pendant le doublon (AS-232, AS-233).
5. Autorisations récurrentes neutres pour les soins (R-AS36).
6. Profil post-bac : parents non destinataires, justificatif avant le CROUS (R-AS34, AS-204).
7. « Je prends » sur l'appel d'une famille (AS-210).
8. Retard annoncé par l'accueil (AS-218).
9. Réponses uniformes contre l'énumération (R-AS35).
10. Journal des consultations et droit d'accès pour le DPO (AS-245, AS-246).

### 6.4 Ce que la passe a montré sur la conception

Solide sur le modèle (constat, situation, épisode), le réglementaire et la correction ; elle pensait la famille comme un foyer, pas comme **deux** parents qui ne vivent pas ensemble ; elle pensait l'élève hors de la classe comme un départ **prévu**, pas comme une **exclusion de cours** improvisée ; elle traitait le hors ligne comme un problème de synchronisation, pas comme un **risque d'appareil perdu** ; elle traitait le post-bac comme un lycéen majeur.

---

## 6 bis. Réglages signalés (D-023)

| N° | Réglage | Pourquoi il est suspect | Proposition |
|---|---|---|---|
| S-483 | Trombinoscope activé pour les remplaçants seulement | sous-réglage de S-468 | garder une seule case « photo pour les remplaçants » |
| S-484 | Heure du récapitulatif à l'autre parent | un paramètre de plus | fixe : 18 h, avec le résumé du socle 8 |
| S-485 | Délai de non-arrivée après exclusion de cours | sans enjeu local | fixe : 10 min |
| S-486 | Revue des autorisations récurrentes | fréquence | fixe : chaque trimestre |

---

## 7. Faits, hypothèses, contradictions

**Faits [F]** : conception relue en entier ; constats Next.js du §2.4 de la conception (lecture seule) ; constat d'énumération par 422 relevé dans `docs/recette/RECETTE_PERSONAS_1.md` (RP1, `POST /absences/{id}/justifier`).
**Vérifié [V]** : Code civil 373-2-1 (VERIF R11) ; L511-5 (VERIF R7) ; L124-15 ; R131-5 à R131-7 (conception §5).
**Hypothèses [H]** : pratiques de terrain (exclusion de cours, appels de familles en double, taxi, soins réguliers sur le temps scolaire).
**Contradictions** : écrites au §29.6 de la conception (C-13 à C-15).
