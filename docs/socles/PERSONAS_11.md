# Passe « personas » sur le socle 11 — Applications mobiles (coquille commune)

**Date :** 2026-11-11 — **Statut :** proposition, **non décidée** (à relire par Florian) ; bloquants appliqués par défaut (D-031). Aucun code écrit ni modifié ; aucun accès à une base, à un secret ni à un service de production ; Next.js lu en lecture seule ciblée (`app/api/mobile/*`), rien modifié.
**Branche :** `ent-socle-11-conception` (base `ent-mc0hoe` `afeb8c4`).
**Éprouve :** `docs/socles/11_MOBILE.md` (lu en entier), avec `CLAUDE.md`, `docs/DECISIONS.md` (D-003, D-009, D-011, D-012, D-013, D-018, D-031, D-036), les attentes mobiles des socles 4, 8, 9, 12, 14, 29, 34, 41, 42, et la méthode de `PERSONAS_10.md` et `PERSONAS_42.md`.
**Ajouts :** faits dans `11_MOBILE.md` §32, marqués « ajout personas » : R-MO40 à R-MO46, MOB-256 à MOB-285, CMO-61 à CMO-74, S-1109, migrations 2231–2232, durées de conservation. Rien n'est supprimé ; les contradictions sont au §30 de la conception.
**Numérotation :** manques **M-1100 à M-1163** (plage propre au socle 11, comme M-1501 pour le 15 et M-2900 pour le 29).

---

## 0. À lire en premier : quatre bloquants

| # | Point | Pourquoi | Ce que la conception doit faire | Gravité | Lots |
|---|---|---|---|---|---|
| A | **Le téléphone familial partagé** (une mère, une collégienne, son petit frère sur le même Android) | Une installation = une liste de comptes ; sans séparation, le frère voit les notes et les messages de sa sœur, la mère ouvre le compte élève, et une notification de l'un s'affiche pour l'autre ; c'est l'équipement le plus courant des familles modestes [H] (P10, P08) | **Profils locaux** par compte dans l'appli, chacun avec son déverrouillage (passkey ou code propre au profil, en plus du verrou de l'appareil), caches et baux séparés, notifications étiquetées par profil sans contenu (R-MO40) | **bloquant** | MO4, MO5, MO7 |
| B | **Un lien ou un QR de rattachement piégé** (QR collé sur l'affiche de rentrée, SMS « activez votre compte » vers `lpnb-scolia.fr`) | Le premier geste d'une famille est de scanner ; une page d'hameçonnage peut demander un mot de passe ou lancer une fausse « activation » ; les établissements impriment des milliers de QR [H] (P02, P17) | l'appli ne fait confiance qu'au **guichet central** pour le nom, la commune et le logo ; elle ne demande **jamais** de passkey ni de mot de passe dans une page web ouverte depuis un QR ; QR imprimés avec un **code court de vérification** ; signalement d'un QR suspect (R-MO41) | **bloquant** | MO3 |
| C | **Le parent interdit garde l'appli** après la décision du juge | Le bail famille de 24 h laisse l'EDT du jour et l'à-faire au parent visé ; un appareil hors réseau garde tout jusqu'à l'expiration ; la coupure ne doit rien révéler (P08) | à la pose de l'interdiction : ordre de purge immédiat, notification silencieuse, coupure au prochain contact **en moins d'une minute** si joignable ; profils famille plafonnés à **la journée en cours** ; message neutre ; un nouveau code de rattachement est refusé (R-MO42) | **bloquant** | MO7 ; socle 4 |
| D | **Le téléphone de l'encadrant volé à l'étranger** avec les fiches d'urgence du voyage | 34 et 42 mettent des fiches C3 sur ce téléphone ; l'encadrant est seul, sans ordinateur ; le voyage continue (P12) | la **direction** (depuis la France) déclare la perte pour lui, ordonne la purge des baux de sortie, **émet le profil de sortie sur l'appareil d'un autre encadrant** ; compte rendu d'incident au DPO prérempli avec l'inventaire (R-MO43) | **bloquant** | MO7, MO14 ; socles 34, 42 |

Trois points importants changent aussi le kit : **le téléphone personnel n'est jamais obligatoire** pour un personnel (R-MO44) ; **le contexte** (établissement, casquette, profil) est affiché partout, notifications comprises (R-MO45) ; **les fonctions d'urgence ne sont jamais bloquées** par une version minimale ou une purge de confort (R-MO46).

---

## 1. Méthode

Démarche de `PERSONAS_10.md` §1 : 17 personas réalistes, public et privé, collège, lycée, lycée professionnel, internat, établissement rural ; pour chacune, sa journée et son année **avec le téléphone**, puis ses incidents (réseau absent, appareil perdu, partagé, ancien, sans verrou, langue, handicap, interdiction, voyage, attaque). Grille posée à chacune : *à qui appartient l'appareil ? qui d'autre y a accès ? que reste-t-il dessus s'il disparaît ? que voit l'écran verrouillé ? que se passe-t-il sans réseau ? que se passe-t-il quand ses droits changent ?*

Gravité : **bloquant** (fuite, impossibilité de traiter un cas réel ou contradiction qui empêche de coder) ; **important** (cas fréquent sans solution propre) ; **confort** ; **différenciant** [H, comparaison de mémoire avec les applis du marché].

Limites : personas construites, non interviewées ; comportements techniques des systèmes (passkeys sans compte Google, notifications silencieuses) notés [H] et à vérifier sur appareils.

---

## 2. Les personas

| # | Persona | Établissement | Ce qu'elle éprouve |
|---|---|---|---|
| P01 | **M. Durand**, professeur d'EPS | collège rural public, gymnase municipal sans réseau | appel hors ligne toute la matinée, froid, téléphone personnel |
| P02 | **Sylvie**, secrétaire de scolarité | lycée privé sous contrat (pilote) | rattachements de la rentrée, familles bloquées, QR |
| P03 | **Mme Benali**, professeure de maths partagée sur deux établissements, mère d'un élève dans l'un | collège public + lycée privé | casquettes, deux établissements, plages calmes différentes |
| P04 | **Théo**, AED | collège en éducation prioritaire | téléphone de service partagé, billets, localisation |
| P05 | **Fatou**, agent d'accueil | lycée public | tablette partagée, panne, fiche de secours |
| P06 | **M. Roux**, principal adjoint | collège public | alerte PPMS depuis la cour, version d'appli |
| P07 | **Mme Martin**, mère séparée | deux enfants dans deux établissements, Android ancien | appareil non supporté, l'autre parent, justification dans le métro |
| P08 | **M. Garcia**, père visé par une interdiction de communication | collège | **attaquant « légitime »** : garde l'appli, réessaie, passe par le téléphone de l'enfant |
| P09 | **Mme Haddad**, grand-mère responsable, arabophone | collège | pas de verrou, pas de compte Google, aide de l'accueil |
| P10 | **Inès**, 11 ans, 6ᵉ | collège | téléphone familial partagé avec sa mère et son frère |
| P11 | **Hugo**, 18 ans, terminale | lycée | majorité, fin de scolarité |
| P12 | **M. Lefèvre**, encadrant d'un voyage à Rome | lycée | téléphone volé, fiches d'urgence, itinérance, fuseau |
| P13 | **Mme Petit**, professeure malvoyante | lycée | VoiceOver, grandes tailles |
| P14 | **M. Nguyen**, référent numérique | ensemble scolaire privé, 60 iPad | flotte, iPad de classe, mises à jour |
| P15 | **Karim**, surveillant d'internat | lycée professionnel avec internat | appel de nuit, téléphone de service, urgence de nuit |
| P16 | **Mme Leroy**, DPO | réseau d'établissements privés | **auditrice RGPD** : inventaire, AIPD, preuves, incidents |
| P17 | **Lucas** (élève de 4ᵉ) et **« X »** (attaquant technique) | collège ; Internet | **attaquants** : téléphone du professeur, capture, lot rejoué, appli contrefaite |

---

## 3. Parcours et manques

### P01 — M. Durand, professeur d'EPS (collège rural)
*Journée.* 8 h : gymnase municipal, aucun réseau ; quatre classes enchaînées jusqu'à 12 h ; il fait l'appel avec des gants, un élève arrive en retard pendant l'appel ; il ne revient au collège qu'à 12 h 15. Il refuse que les données de ses élèves soient sur son téléphone personnel.
| ID | Manque | Pourquoi | Gravité | Ajout |
|---|---|---|---|---|
| M-1100 | État des **envois en attente** visible depuis Aujourd'hui (pas seulement dans l'écran d'appel) | quatre appels non envoyés : il doit savoir, d'un coup d'œil, que rien n'est perdu | important | MOB-256 |
| M-1101 | **Appel par exceptions** (« tous présents » puis absents) et gros boutons en extérieur | gants, froid, 30 élèves : dix secondes, pas trente gestes | confort | MOB-257 (composant du 12) |
| M-1102 | **Retard** saisi hors ligne pendant l'appel | élève arrivé à 8 h 07 ; opération distincte, horodatée | important | CMO-61 |
| M-1103 | **Faire l'appel sans téléphone personnel** (tablette de salle, poste partagé du gymnase, web) | on ne peut pas imposer son téléphone personnel à un agent [H, droit du travail et charte] | important | R-MO44, MOB-258 |

### P02 — Sylvie, secrétaire de scolarité (lycée privé, pilote)
*Rentrée.* 1 200 familles à rattacher ; 80 appellent ; une affiche avec QR à l'entrée ; un parent sans e-mail ; un SMS frauduleux circule.
| ID | Manque | Pourquoi | Gravité | Ajout |
|---|---|---|---|---|
| M-1104 | **QR ou lien piégé** : rien ne permet à la famille de reconnaître le vrai | §0 point B | **bloquant** | R-MO41, MOB-259 (QR avec code court de vérification), MOB-112 |
| M-1105 | **Fiche d'état de connexion** d'une personne pour le support (rattachée ? passkey ? appareil ? notifications bloquées ? version ?) sans accès à ses données | « l'appli ne marche pas » : dix minutes par appel aujourd'hui [H] | important | MOB-260 |
| M-1106 | **Lettres de rattachement** par famille, dans sa langue, avec le QR et le code | une feuille par famille, pas une affiche commune | confort | MOB-261 |
| M-1107 | **Responsable sans e-mail** (D-020) : activation et récupération | D-018 récupère par e-mail ; sans e-mail : par l'établissement seulement | important | CMO-62 |

### P03 — Mme Benali, professeure partagée et mère d'élève
*Semaine.* Lundi et jeudi au collège public, mardi et vendredi au lycée privé, où son fils est en 1ʳᵉ ; elle n'est pas professeure de sa classe… sauf en option maths expertes.
| ID | Manque | Pourquoi | Gravité | Ajout |
|---|---|---|---|---|
| M-1108 | **Contexte affiché** partout, notifications comprises (« Lycée X · Mère de Yanis ») | une notification « nouveau message » sans contexte : elle ne sait pas si c'est un parent d'élève ou le CPE de son fils | important | R-MO45 |
| M-1109 | Professeure **de son propre enfant** (option) : la casquette professeure voit son fils comme les autres élèves ; conflit d'intérêts | socle 4 (270, `conflit_interet`) ; le mobile ne doit pas offrir un raccourci entre casquettes | important | CMO-63 |
| M-1110 | **Plages calmes différentes** (public : arrêté ; privé : réglage) dans la même appli | l'appli ne doit pas appliquer la règle de l'un à l'autre | confort | CMO-64 |
| M-1111 | **Déclarer la perte** dans tous ses établissements en un geste | trois établissements, trois démarches sinon ; sans croisement serveur | important | MOB-262 |

### P04 — Théo, AED (collège REP)
*Journée.* Il prend le téléphone de service à 7 h 45, distribue des billets de retard dans la cour (réseau faible), cherche un élève à 10 h.
| ID | Manque | Pourquoi | Gravité | Ajout |
|---|---|---|---|---|
| M-1112 | **Qui a le téléphone de service** à 10 h 12 ? | incident, contestation d'un billet | important | MOB-216 (existant, confirmé) |
| M-1113 | Fermeture de session **à la fin du service** (planning du socle 6), pas seulement à l'inactivité | le téléphone passe à l'AED suivant | confort | S-1106 (précisé) |
| M-1114 | **Billets de retard** hors ligne | réseau faible dans la cour | important | opération déclarée par le 12 (MOB-181) |
| M-1115 | **Localiser un élève** hors ligne : impossible et c'est voulu (lecture unitaire journalisée) ; le dire | sinon il croit à une panne | confort | CMO-65 |

### P05 — Fatou, agent d'accueil (lycée public)
| ID | Manque | Pourquoi | Gravité | Ajout |
|---|---|---|---|---|
| M-1116 | Connexion au poste par **clé de sécurité / badge NFC** (pas de passkey sur téléphone personnel) | R-MO44 ; agents sans smartphone [H] | important | MOB-263 |
| M-1117 | Fiche de secours **mise à jour en moins d'une minute** après une interdiction, sinon « non vérifiée depuis » | R-A10 du socle 4 | important | MOB-206 (existant, confirmé) |
| M-1118 | **Rideau** : masquer l'écran d'un geste quand un visiteur se penche | interdictions en tête de liste | confort | MOB-264 |

### P06 — M. Roux, principal adjoint
| ID | Manque | Pourquoi | Gravité | Ajout |
|---|---|---|---|---|
| M-1119 | Déclencher l'alerte depuis une **zone blanche** de la cour | le 14 suppose le réseau ; repli : wifi, autre personne habilitée, procédure papier | important | CMO-66 |
| M-1120 | Alerte et comptage **jamais bloqués** par une version minimale ou une purge de confort | le pire moment pour « mettez à jour » | **important** | R-MO46 |
| M-1121 | Exercice PPMS : comptage hors ligne sur les téléphones des professeurs **et** des tablettes de salle | professeurs sans appli (R-MO44) | confort | MOB-258 |

### P07 — Mme Martin, mère séparée, deux établissements, Android 8
| ID | Manque | Pourquoi | Gravité | Ajout |
|---|---|---|---|---|
| M-1122 | Appareil **non supporté** : orientation vers le web avec un lien de connexion rapide | sinon « l'appli ne marche pas » | important | MOB-265 |
| M-1123 | **L'autre parent** ne voit jamais ses appareils, ses connexions, ni qu'elle a l'appli | minimisation entre foyers (socle 4) | important | CMO-67 |
| M-1124 | Préparer une **justification** dans le métro, envoyée **après relecture** au retour du réseau | R-PO20, R-MO21 | confort | MOB-266 |
| M-1125 | Deux établissements, **un seul** adopte ScolIA : rien sur l'autre | CMO-08 (existant, confirmé) | confort | — |

### P08 — M. Garcia, père visé par une interdiction (attaquant « légitime »)
| ID | Manque | Pourquoi | Gravité | Ajout |
|---|---|---|---|---|
| M-1126 | Il garde l'appli ouverte après la décision ; cache 24 h | §0 point C | **bloquant** | R-MO42 |
| M-1127 | Message de coupure **neutre** (jamais « décision de justice ») | protection de la mère et de l'enfant | important | R-MO42 |
| M-1128 | Il redemande un **code de rattachement** à l'accueil | le code doit être refusé par le socle 4/9, sans que l'agent ait à savoir pourquoi | important | CMO-68 |
| M-1129 | Il ouvre le **compte de son fils** sur le téléphone de l'enfant (il connaît le code de l'appareil) | profil élève avec déverrouillage propre | important | R-MO40, MOB-267 |

### P09 — Mme Haddad, grand-mère responsable, arabophone
| ID | Manque | Pourquoi | Gravité | Ajout |
|---|---|---|---|---|
| M-1130 | Téléphone **sans verrou** : R-MO9 la bloque ; il faut la **guider** pour en activer un | sinon exclusion | important | MOB-268 |
| M-1131 | Appli en **arabe**, droite-gauche | existant (MOB-222, 223) | confort | — |
| M-1132 | **Pas de compte Google** sur le téléphone : passkey impossible [H] | repli du socle 1 (Q-MO2) | important | CMO-69 |
| M-1133 | **Accompagnement** par l'accueil sans que l'agent prenne le téléphone ni voie ses données | dignité, confidentialité | différenciant | MOB-269 |

### P10 — Inès, 11 ans, 6ᵉ (téléphone familial)
| ID | Manque | Pourquoi | Gravité | Ajout |
|---|---|---|---|---|
| M-1134 | Un téléphone, trois comptes (mère, Inès, frère) | §0 point A | **bloquant** | R-MO40, 2232 |
| M-1135 | Notifications sur le téléphone partagé : **quel profil** ? sans contenu | sinon le frère lit « nouvelle note » d'Inès | important | R-MO45 |
| M-1136 | Rien pendant les cours | R-MO22 (existant, confirmé) | confort | — |
| M-1137 | **Élève sans téléphone** : tout au web, carte imprimée, aucune pénalité | égalité | important | CMO-70 |

### P11 — Hugo, 18 ans, terminale
| ID | Manque | Pourquoi | Gravité | Ajout |
|---|---|---|---|---|
| M-1138 | **Parcours guidé** le jour de sa majorité (ce que ses parents voient désormais, ce qu'il peut régler) | socle 4 décide, l'appli explique | différenciant | MOB-270 |
| M-1139 | Ses absences ne doivent plus prévenir ses parents **s'il l'a choisi** et si le règlement le permet | socles 4 et 8 | important | renvoyé aux socles 4 et 8 |
| M-1140 | **Fin de scolarité** : export de ses données avant la fermeture (3 mois) | droit à la portabilité | confort | CMO-71 |

### P12 — M. Lefèvre, encadrant d'un voyage à Rome
| ID | Manque | Pourquoi | Gravité | Ajout |
|---|---|---|---|---|
| M-1141 | Téléphone volé avec les fiches d'urgence | §0 point D | **bloquant** | R-MO43, MOB-271 |
| M-1142 | Passer le profil de sortie sur le **téléphone d'un collègue** | le voyage continue | important | MOB-272 |
| M-1143 | **N'envoyer qu'en wifi** (itinérance) | coût des données | confort | MOB-273 |
| M-1144 | **Fuseau** du lieu du voyage | heures affichées dans le fuseau de l'établissement avec l'heure locale en regard | confort | CMO-72 |

### P13 — Mme Petit, professeure malvoyante
| ID | Manque | Pourquoi | Gravité | Ajout |
|---|---|---|---|---|
| M-1145 | Grille d'appel au lecteur d'écran avec **actions personnalisées** (rotor : présent, absent, retard) | trente élèves au balayage, c'est trop long | important | MOB-274 |
| M-1146 | État « enregistré sur le téléphone » **annoncé** | sinon elle ne sait pas si c'est envoyé | important | MOB-256 |
| M-1147 | Taille de texte maximale : la grille ne doit pas se casser | test | confort | CMO-73 |

### P14 — M. Nguyen, référent numérique (60 iPad)
| ID | Manque | Pourquoi | Gravité | Ajout |
|---|---|---|---|---|
| M-1148 | **Session élève par QR personnel** imprimé (pas de passkey sur l'iPad de classe) | élèves de 6ᵉ sans téléphone | important | MOB-275 |
| M-1149 | **Fenêtre de mise à jour** de l'appli sur la flotte (hors cours) | une mise à jour en pleine séance | confort | MOB-276 |
| M-1150 | iPad perdu de la flotte : inventaire et purge comme un téléphone | §12.7 | confort | MOB-196 (existant, confirmé) |

### P15 — Karim, surveillant d'internat
| ID | Manque | Pourquoi | Gravité | Ajout |
|---|---|---|---|---|
| M-1151 | Appel de nuit hors ligne, effacé à validation + 12 h | 41 (existant, confirmé) | confort | — |
| M-1152 | Passation du téléphone de service à 23 h | journal du poste | confort | MOB-216 |
| M-1153 | **Fiche d'urgence des internes** la nuit (allergie, traitement), avec les mêmes garde-fous que la sortie | urgence de nuit sans infirmière [H] ; aujourd'hui seul `sortie.urgence` est permis | important | MOB-277 (profil `internat.urgence`, à déclarer par 34 et 41) |

### P16 — Mme Leroy, DPO (auditrice RGPD)
*Elle prépare l'AIPD et vérifie les preuves.*
| ID | Manque | Pourquoi | Gravité | Ajout |
|---|---|---|---|---|
| M-1154 | **Statistiques des baux** pour l'AIPD (appareils portant du C2+, durées réelles, purges non confirmées) | mesurer le risque, pas le supposer | important | MOB-278 |
| M-1155 | **Durées de conservation** de chaque table du socle | registre | important | §32.5 |
| M-1156 | **Preuve** de l'absence de SDK tiers (SBOM publiée, test CI qui refuse une dépendance non listée) | contrôle CNIL des applis | important | MOB-279 |
| M-1157 | **Notice** « ce qui peut se trouver sur votre téléphone » par profil | information (RGPD art. 13) | important | MOB-280 |
| M-1158 | Incident : **échéance des 72 h** affichée | art. 33 | important | MOB-281 |

### P17 — Lucas (élève de 4ᵉ) et « X » (attaquant technique)
*Lucas* : pendant l'appel, le professeur pose son téléphone déverrouillé sur le bureau pour aider un élève. *X* : rejoue un lot capturé, publie une fausse appli « ScolIA Plus », se déclare « mobile ».
| ID | Manque | Pourquoi | Gravité | Ajout |
|---|---|---|---|---|
| M-1159 | **Verrou court** sur les écrans de saisie quand le téléphone est posé (2 min d'inactivité pendant une séance) | Lucas met tout le monde présent ou lit la liste | important | MOB-282, S-1109 |
| M-1160 | **Photo de l'écran** d'appel par un autre téléphone | impossible à empêcher ; minimiser ce qui est affiché (pas de motif, pas de santé) | confort | CMO-74 |
| M-1161 | Script qui se déclare « mobile » | DS2, R-MO3 (existant, confirmé) | important | — |
| M-1162 | **Lot capturé et rejoué** | signature horodatée + idempotence (existant) ; test d'attaque | important | §28.3 |
| M-1163 | **Appli contrefaite** sur un magasin tiers ; appareils non attestés | une seule fiche officielle, signalement, vue des appareils non attestés, verrouillage d'urgence de toutes les sessions mobiles d'un établissement | important | MOB-283, MOB-284, MOB-285 |

---

## 4. Manques qui relèvent d'autres socles
- **Socle 1** : repli sans passkey (Q-MO2, M-1132) ; récupération sans e-mail (M-1107) ; clé de sécurité NFC pour les postes (M-1116).
- **Socle 4** : refus d'un code de rattachement pour un responsable interdit (M-1128) ; choix de l'élève majeur sur l'information de ses parents (M-1139) ; invisibilité des appareils entre foyers (M-1123).
- **Socle 6** : fin de service pour fermer la session du téléphone de service (M-1113) ; règle « téléphone personnel jamais obligatoire » dans la charte (M-1103).
- **Socle 8** : notifications étiquetées par profil local (M-1135) ; plages calmes par établissement (M-1110).
- **Socles 12, 14, 41, 42, 34** : appel par exceptions (M-1101), retard hors ligne (M-1102), alerte en zone blanche (M-1119), fiche d'urgence d'internat (M-1153), transfert du profil de sortie (M-1142).
- **Socle 27** : notice par profil hors ligne (M-1157), échéance des 72 h (M-1158).

---

## 5. Synthèse

### 5.1 Manques par gravité
| Gravité | Nombre | IDs |
|---|---|---|
| bloquant | 4 | M-1104, M-1126, M-1134, M-1141 |
| important | 38 | M-1100, 1102, 1103, 1105, 1107, 1108, 1109, 1111, 1112, 1114, 1116, 1117, 1119, 1120, 1122, 1123, 1127, 1128, 1129, 1130, 1132, 1135, 1137, 1139, 1142, 1145, 1146, 1148, 1153, 1154, 1155, 1156, 1157, 1158, 1159, 1161, 1162, 1163 |
| confort | 20 | M-1101, 1106, 1110, 1113, 1115, 1118, 1121, 1124, 1125, 1131, 1136, 1140, 1143, 1144, 1147, 1149, 1150, 1151, 1152, 1160 |
| différenciant | 2 | M-1133, M-1138 |

Total : 4 + 38 + 20 + 2 = **64**. Huit manques confirment la conception sans ajout (M-1112, 1117, 1125, 1131, 1136, 1150, 1151, 1161).

### 5.2 Ce que la passe a montré
1. La conception pensait **un appareil = une personne** ; le terrain a des **téléphones partagés** (famille) et des **personnes sans téléphone** (élèves, agents) : les deux doivent être des cas de première classe (R-MO40, R-MO44).
2. Le **premier geste** (scanner un QR) est le moment le plus exposé à l'hameçonnage (R-MO41).
3. Les **décisions qui coupent un accès** (interdiction, vol) doivent agir plus vite que le bail le plus court (R-MO42, R-MO43).
4. L'**urgence** passe avant les règles de confort (R-MO46).
5. Le DPO a besoin de **mesures**, pas de principes : les baux le permettent (MOB-278).

### 5.3 Réglage ajouté
S-1109 : verrou court des écrans de saisie pendant une séance (défaut 2 min), établissement.
