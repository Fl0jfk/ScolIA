# Passe « personas » sur le socle 41 — Internat

**Date :** 2026-11-11 — **Statut :** proposition, **non décidée** (à relire par Florian) ; les recommandations s'appliquent par défaut (D-031), sauf ce qui engage l'établissement à l'extérieur (§8.2). Aucun code écrit ni modifié ; aucun accès à une base, à un secret ni à un service de production. Le code Next.js a été **lu seulement**, sur une copie de `origin/main` extraite par `git archive` dans le dossier temporaire de la session (rien exécuté, rien construit, aucun `.env` lu).
**Branche :** `ent-socle-40-41-conception`, partie de `origin/ent-mc0hoe` (`7467f2d4`).
**Éprouve :** `docs/socles/41_INTERNAT.md` (§0 à §29), avec `CLAUDE.md`, `docs/DECISIONS.md` D-017, D-031, **D-034**, **D-035**, **D-036**, `docs/PRA_SAUVEGARDES.md` §18.4, `docs/TRANSITION_NEXT_RUST.md`, `docs/TRANSITION_T0_INVENTAIRE.md` (M11) ; forme reprise de `PERSONAS_42.md`.
**Ajouts :** faits **directement** dans `41_INTERNAT.md`, au **§30** (actions INT-250 à INT-269, règles R-IT42 à R-IT44, cas limites CIT-49 à CIT-62, questions Q-IT31 à Q-IT36, lots révisés) et au **§31** (faits, hypothèses, contradictions). Rien n'est supprimé.
**Numérotation :** manques **M-4101 à M-4160** (plage propre au socle 41).

---

## 0. À lire en premier : quatre bloquants et leur recommandation (appliquée par défaut, D-031)

La conception tient sur le fond : l'interne naît du régime, la présence est un fait en ajout seul, la santé passe par des consignes, la procédure « non retrouvé » est minutée. Vécue par dix-sept personas, elle révèle quatre bloquants.

| # | Point | Pourquoi c'est bloquant | Recommandation (appliquée par défaut) | Lots |
|---|---|---|---|---|
| **A** | **La nuit sans réseau est traitée comme une exception, alors que c'est le cas normal d'un internat** (bâtiments anciens, murs épais, Wi-Fi coupé la nuit par économie ou par règlement). Le §9.2 met l'appel hors ligne au lot I10, **après** I2 ; la liste de secours imprimée est un réglage (S-4110). | Un surveillant seul (P01) qui n'a pas de réseau au 3e étage ne peut ni pointer, ni voir les consignes, ni ouvrir un cas : la procédure du §9.3 tombe exactement quand elle sert. | **Hors ligne dès I2** (pas en I10) pour l'appel, la fiche de nuit et la procédure de cas ; I10 garde seulement le confort (synchronisation fine, conflits). **Impression de la liste des présents prévus obligatoire** (S-4110 n'est plus désactivable) ; **R-IT42**. | I2, I5 |
| **B** | **L'astreinte n'a pas de seconde ligne.** La procédure appelle « le CPE d'astreinte » (lu au 6) ; si le 6 n'a pas d'astreinte saisie, ou si le CPE ne répond pas, rien n'est prévu. | À 1 h du matin, sans réponse du CPE (P02), le surveillant n'a plus de consigne ; c'est le scénario des drames. | **Chaîne d'astreinte à deux niveaux au moins** (CPE puis direction, ou adjoint) déclarée par semaine ; **bascule automatique** au niveau suivant après 10 min sans accusé ; **refus de valider l'ouverture de la nuit** sans astreinte déclarée (alerte au responsable dès 17 h) ; **R-IT43**, INT-250 à INT-252. | I2, I6 |
| **C** | **Le « titulaire qui annonce une absence » est un point d'usurpation.** Un élève qui a le mot de passe d'un parent, ou un parent visé par une interdiction de remise, peut annoncer « ne dort pas ce soir » et organiser une sortie que l'autre parent ignore (P05, P15). | La conception prévient l'autre titulaire (§10.1) mais l'absence est **acceptée** sans contrôle : l'enfant disparaît des attendus, personne ne le cherche. | **L'annonce n'efface jamais l'attendu tant qu'un adulte n'a pas confirmé la remise** : l'élève est « attendu absent », il **doit être remis** (14) ou **parti seul avec autorisation annuelle** ; sinon, à l'heure de l'appel, il redevient attendu et le cas s'ouvre. Annonce par un titulaire faite depuis un appareil inconnu ou par un compte élève connecté au même appareil = **confirmation par passkey** ; avis **immédiat** à tous les titulaires. **R-IT44**, INT-253 à INT-255. | I3 |
| **D** | **La bascule d'été (Q-IT1) laisse un trou pour les autorisations annuelles et la campagne d'installation.** Les autorisations de sortie 2027-2028 sont signées en juin–juillet, la campagne d'installation ouvre fin juin : si ScolIA n'est pas encore officiel, elles sont recueillies dans Next.js (papier ou liens sans expiration, DI-05) puis à ressaisir. | Double saisie au pire moment de l'année ; risque de rentrée sans autorisations valides (P16). | **Les objets de l'année 2027-2028 naissent dans ScolIA** dès juin (séjours, autorisations du 18, campagne d'installation, répartition), même si les nuits de juin restent dans Next.js : la propriété est **par année scolaire** (R-IT37 précisée) ; INT-266. | I3, I8, I11 |

**Défauts Next.js ajoutés** (signalés à Florian, Next.js intouchable) : **DI-26** — les sorties de week-end typées portent un **motif en texte libre** sans statut ni auteur (`lib/internat-sorties-db.ts:17, 126`) [F], qui peut contenir un motif familial ou médical ; **DI-27** — aucune vérification du sexe ou de l'aile à l'affectation typée (`lib/internat-chambres-db.ts`, aucune occurrence de `sexe`) [F], confirmant DI-15.

---

## 1. Méthode
Celle de `PERSONAS_42.md` §1 : dix-sept personas réalistes (privé sous contrat — lycée pilote avec internat collège et lycée — et public EPLE), dont un **attaquant** (quatre visages) et un **auditeur RGPD** ; pour chacune : sa nuit ou son année, ses irritants, peurs, ce qui la ravirait ; puis les **manques** (M-41xx), **contradictions** et **cas limites**, versés au §30 du 41. Gravité : **B** bloquant, **H** haute, **M** moyenne, **F** faible. Vérification ponctuelle du code Next.js sur trois points (marque « activité » de l'appel JSON, sorties de week-end typées, installation) [F].

---

## 2. Les personas en un tableau
| # | Persona | Situation | Socles touchés |
|---|---|---|---|
| P01 | **Yanis, 23 ans, AED étudiant**, seul sur le bâtiment B (48 garçons de lycée) de 18 h à 7 h 30 | Réseau faible au 3e, première année | 41, 11, 34, 6 |
| P02 | **Mme Benali, CPE d'astreinte**, réveillée à 1 h 05 | Élève introuvable, deuxième appel sans réponse | 41, 6, 4, 14 |
| P03 | **M. Garnier, responsable d'internat**, fin août | 132 internes, 3 bâtiments, répartition et installation | 41, 3, 18, 19, 44 |
| P04 | **Mme Duval, cheffe d'établissement** | Exercice de nuit, passage de la commission de sécurité | 41, 14, 5 |
| P05 | **Famille Martin-Leroy**, séparée, garde alternée, interdiction de remise visant un oncle | Léa, 4e, interne | 41, 4, 14, 18 |
| P06 | **Famille Okafor**, à Lagos ; Daniel, 1re, reste le week-end | Correspondant à 40 km, décalage horaire | 41, 4, 5, 6, 43 |
| P07 | **Tom, 11 ans, 6e**, mal du pays | Pleure le soir, veut appeler sa mère | 41, 36, 16 |
| P08 | **Hugo, 19 ans, BTS**, interne majeur | Sorties du soir, travail | 41, 2, 18 |
| P09 | **Inès, 2de, en fauteuil**, AESH en journée | Chambre adaptée, évacuation | 41, 35, 34, 14 |
| P10 | **Sam, 1re, élève transgenre** accompagné | Chambre, nom d'usage, appel | 41, 2, 4 |
| P11 | **Mme Roche, infirmière** | Santé désactivée (pas d'HDS) pendant le doublon | 41, 34 |
| P12 | **Mme Lambert, comptable de l'OGEC** | Doublon 2027-2028, Charlemagne officiel | 41, 24, 26 |
| P13 | **M. Petit, accueil** le vendredi 16 h | 90 départs, voitures en double file | 41, 14, 4 |
| P14 | **M. Blanc, délégué à la protection des données** | Audit de l'internat | 41, 27 |
| P15 | **Les attaquants** : élève qui veut « découcher », oncle interdit, surveillant malveillant, robot sur l'installation | | 41, 1, 4, 18 |
| P16 | **Florian, administrateur**, juin–juillet 2027 | Bascule du module | 41, 11, 18, 19 |
| P17 | **Lycée public Jean-Moulin** (EPLE, 300 internes) | Collectivité, agent comptable, CA | 41, 24, 9 |

---

## 3. Audit de parité avec Next.js (fait de code)

### 3.1 Comportements vérifiés
- **Marque « activité »** de l'appel JSON : acceptée comme quatrième marque (`api/internat/roll-call/route.ts:38`) [F] ; reprise (INT-133, INT-134) ; l'interdiction de valider avec une activité en attente est reprise (R-IT16).
- **Sorties de week-end typées** : élève, dates, **motif libre**, sans statut ni auteur ni lien avec les titulaires [F] → DI-26 ; la conception les remplace par des sorties typées avec type, décideur, personne, faits (§10.2, INT-150 à INT-169) : parité **et** correction.
- **Installation** : réservation en attente valable **2 h** (`INSTALLATION_PENDING_TTL_MS`), jeton cherché en liste (`internat-installation-storage.ts:11, 114`) [F] ; la conception remplace le courriel de confirmation par la **connexion du titulaire** ou un lien nominatif (INT-222, INT-223) : la durée de 2 h n'a plus d'objet pour un titulaire connecté ; conservée pour le lien nominatif (M-4140).

### 3.2 Fonctions sans équivalent au 41 (ajoutées au §30)
| Next.js | Manque | Ajout |
|---|---|---|
| Destinataires des PDF d'appel **séparés collège / lycée** et « contacts » paramétrables | le 41 dit « direction concernée » sans dire comment on la déclare | INT-256 (destinataires par formation) |
| « Sous surveillance » + note sur la fiche de l'interne (NI-06) | non repris, sans explication | INT-257 : **vigilance** codée posée par le CPE, sans texte, visible de l'équipe de nuit, datée, révisée chaque semaine |
| Autorisations spéciales datées (NI-06) | partiellement couvertes par les sorties | INT-258 : **permissions particulières** datées (« peut rester en salle informatique jusqu'à 22 h ») |
| Analyse des chambres (surcapacité) (NI-03) | reprise en plan, pas en alerte | intégrée à INT-107 (alerte) |

### 3.3 Défauts de sécurité nouveaux
DI-26, DI-27 (§0).

---

## 4. Parcours et manques, persona par persona

### P01 — Yanis, AED, seul sur le bâtiment B
**Sa nuit.** 18 h : prise de service ; 18 h 30 étude ; 19 h 15 dîner ; 21 h 30 appel ; 22 h 30 extinction ; ronde à minuit ; réveil 6 h 45. Il a 48 élèves, son téléphone personnel (pas de téléphone de service), un réseau faible au 3e.
**Irritants.** Recopier l'appel ; ne pas savoir qui a le droit d'être dehors ; les élèves qui « sont à la douche ».
**Peurs.** Un élève introuvable ; un malaise ; être accusé d'avoir mal surveillé.
**Ravi par.** Un appel qui marche sans réseau ; un écran qui dit quoi faire ; une transmission pré-remplie.
| M | Gr. | Manque | Ajout |
|---|---|---|---|
| M-4101 | **B** | Hors ligne seulement en I10 (§0 A) | hors ligne dès I2, R-IT42 |
| M-4102 | H | **Téléphone personnel** : l'appli sur l'appareil privé d'un AED contient des données d'internes | appareil personnel accepté **seulement** avec la session courte du mode « service de nuit » : liste chiffrée, effacée à la fin du service, aucune photo en cache après la nuit ; **téléphone de service recommandé** (Q-IT31) |
| M-4103 | H | Marque « à la douche / aux sanitaires » absente : il pointe « absent », ce qui ouvre un cas inutile | marque transitoire **« vu, momentanément absent »** (délai 15 min avant cas) — INT-259 |
| M-4104 | M | Pas de **minuterie de ronde** (oubli de la ronde de minuit) | rappel de ronde (INT-260) |
| M-4105 | M | La transmission du matin demande trop de saisie | proposée automatiquement, il coche (INT-213 existant, précisé) |
| M-4106 | M | Seul sur le bâtiment : **qui prévenir s'il a lui-même un malaise** ? | bouton « surveillant en difficulté » (alerte critique à l'astreinte, sans élève) — INT-261 |

### P02 — Mme Benali, CPE d'astreinte, 1 h 05
Le téléphone sonne : Kevin, 2de, n'est pas dans sa chambre. Elle est chez elle, à 20 min.
| M | Gr. | Manque | Ajout |
|---|---|---|---|
| M-4107 | **B** | Pas de seconde ligne d'astreinte (§0 B) | chaîne d'astreinte, bascule, R-IT43 |
| M-4108 | H | Elle reçoit un appel, pas l'écran : **le cas doit lui être lisible à distance** (étapes faites, heure, dernières marques de l'élève, dernière sortie) | vue « cas » sur mobile pour l'astreinte (INT-250) |
| M-4109 | H | L'accusé de réception de l'astreinte n'est pas tracé | accusé dans l'appli ou par appel tracé à la main (INT-251) |
| M-4110 | M | Le numéro de la **gendarmerie / du commissariat de secteur** n'est nulle part | annuaire d'urgence de l'établissement (17, 15, 18, 112, brigade de secteur) au paramétrage (INT-262) |
| M-4111 | M | Après l'issue, aucun **compte rendu** structuré pour la direction | compte rendu de cas généré (chronologie des étapes), relu par le CPE (INT-263) |

### P03 — M. Garnier, responsable d'internat, fin août
| M | Gr. | Manque | Ajout |
|---|---|---|---|
| M-4112 | H | La répartition n'intègre pas **les vigilances** de l'année passée (deux élèves à séparer) | les séparations demandées au §8.3 deviennent des **contraintes reconduites** d'une année sur l'autre, révisables (CIT-49) |
| M-4113 | M | Il veut **imprimer les étiquettes de porte** (prénom seul) | INT-264 |
| M-4114 | M | Installation : les familles arrivent **en dehors** de leur créneau | accueil sans créneau possible, compteur du créneau en cours |
| M-4115 | M | Trousseau, caution, règlement : il n'a pas la **liste de ce qui manque** par interne | tableau de rentrée (règlement signé, autorisations, caution, état des lieux, consignes de nuit publiées) — INT-265 |

### P04 — Mme Duval, cheffe d'établissement
| M | Gr. | Manque | Ajout |
|---|---|---|---|
| M-4116 | H | La commission de sécurité demande **l'effectif hébergé maximal** par bâtiment et l'effectif réel moyen | export « effectifs de nuit » (max, moyenne, par bâtiment) avec les exercices (INT-197 étendu) |
| M-4117 | M | L'exercice de nuit doit être **inopiné** : le rappel ne doit pas être vu des surveillants | rappel visible de la direction et du responsable seulement |
| M-4118 | M | Elle ne veut être réveillée que pour U3, mais **informée le matin** de tout cas clos | résumé du matin (déjà §18.1) sans IA : liste déterministe des cas de la nuit |

### P05 — Famille Martin-Leroy (séparée, garde alternée, interdiction visant un oncle)
| M | Gr. | Manque | Ajout |
|---|---|---|---|
| M-4119 | **B** | Annonce d'absence = point d'usurpation (§0 C) | R-IT44, « attendu absent » jusqu'à la remise |
| M-4120 | H | Le week-end « du père » : le vendredi, c'est le père qui vient ; si la mère annonce qu'elle vient, conflit | le départ du vendredi propose **le titulaire du calendrier de partage** (4) ; un changement demandé par l'autre titulaire est **soumis au CPE** et les deux sont avisés |
| M-4121 | M | L'oncle interdit appelle l'internat en se présentant comme le père | consigne au surveillant : **aucune information par téléphone** sur un interne à un appelant non identifié ; affichée dans la fiche de nuit de tout élève ayant une interdiction (CIT-50) |

### P06 — Famille Okafor (Lagos) ; Daniel reste le week-end
| M | Gr. | Manque | Ajout |
|---|---|---|---|
| M-4122 | H | Internat ouvert le week-end : **encadrement de week-end** et repas (40) non déclarés tant que les nuits exceptionnelles ne sont pas posées | rythme 7 nuits avec **week-ends ouverts** déclarés au calendrier (5) ; besoins d'encadrement de week-end (INT-210) ; repas du week-end au 40 |
| M-4123 | M | Les avis partent à 3 h du matin heure de Lagos ou de Paris ? | les avis non urgents respectent le **fuseau du destinataire** (8) ; les urgents partent tout de suite |
| M-4124 | M | Le correspondant doit être joignable comme un titulaire en cas de cas « non retrouvé » le week-end | correspondant ajouté à l'étape « titulaires » quand l'élève est chez lui ou devait y aller |

### P07 — Tom, 11 ans, mal du pays
| M | Gr. | Manque | Ajout |
|---|---|---|---|
| M-4125 | H | Rien pour **le premier mois** des plus jeunes (appel quotidien aux parents autorisé, référent adulte) | **référent d'internat** nommé par élève (adulte de l'équipe), visible de la famille ; fait de liaison « a appelé sa famille » (INT-257) |
| M-4126 | M | Le bilan de la semaine des 6e devrait être plus fréquent au début | fréquence du bilan par niveau (S-4114 par formation) |
| M-4127 | M | Tom peut-il **signaler** quelque chose sans que les grands le voient ? | préoccupation (36) accessible depuis son appli, déjà prévue ; mise en avant dans l'écran élève |

### P08 — Hugo, 19 ans, BTS
| M | Gr. | Manque | Ajout |
|---|---|---|---|
| M-4128 | H | Le règlement autorise les majeurs à sortir **jusqu'à 22 h** sans autorisation : la conception traite la sortie comme une demande | type de sortie « libre majeur » (sans décision, pointage de retour seulement) |
| M-4129 | M | Hugo travaille le samedi : nuit du vendredi chez lui, retour le lundi 8 h directement en cours | retour attendu **en cours** (12) et non à l'internat ; pas de cas d'internat |
| M-4130 | M | Ses parents paient : doivent-ils voir ses absences d'internat ? | S-4115 « si à charge », avec **déclaration** de l'élève sur ce point au 4 (pas une supposition) |

### P09 — Inès, 2de, en fauteuil
| M | Gr. | Manque | Ajout |
|---|---|---|---|
| M-4131 | H | Évacuation : la **liste des présents** doit signaler les personnes **à assister** (sans motif) | marque fonctionnelle « assistance à l'évacuation » issue du 35 (code), sur la liste des présents et l'exercice (CIT-51) |
| M-4132 | M | Accessibilité des **sanitaires** de la zone, pas seulement de la chambre | équipement « sanitaires accessibles » sur la zone |
| M-4133 | M | AESH absente la nuit : qui aide au coucher ? | besoin d'**aide humaine de nuit** déclaré par le 35 → besoin d'encadrement (6) ; jamais de motif |

### P10 — Sam, élève transgenre accompagné
| M | Gr. | Manque | Ajout |
|---|---|---|---|
| M-4134 | H | Le contrôle R-IT11 lit le **sexe d'état civil** : pour un élève accompagné, cela révèle la situation à chaque affectation (refus à l'écran) | la dérogation s'applique **à l'élève** pour l'année (pas à chaque affectation) ; le refus n'affiche jamais « sexe » mais « affectation à valider par la direction » ; le nom d'usage du 2 partout (CIT-52) |
| M-4135 | M | Sanitaires : besoin d'un accès individuel | équipement « sanitaires individuels » de chambre ou de zone |

### P11 — Mme Roche, infirmière, santé désactivée
| M | Gr. | Manque | Ajout |
|---|---|---|---|
| M-4136 | H | Sans HDS, la nuit n'a **aucune** consigne : la conception renvoie au « classeur de l'infirmerie » sans dire **où** il est | paramètre « lieu du classeur des consignes de nuit » et « lieu des trousses », affichés sur la fiche de nuit (texte d'organisation, sans santé) (CIT-53) |
| M-4137 | M | Le mouvement de premiers secours (34) n'existe pas non plus sans N-1 | fait de liaison codé « premiers secours donnés, infirmière à informer » (sans texte médical), seul fait permis, transmis à l'infirmière (R-IT6 précisé) |

### P12 — Mme Lambert, comptable de l'OGEC, doublon
| M | Gr. | Manque | Ajout |
|---|---|---|---|
| M-4138 | M | L'état de comparaison doit être au **format de Charlemagne** (par trimestre, par élève) | export par trimestre et par élève, causes regroupées comme Charlemagne (INT-234 précisé) |
| M-4139 | F | Prime à l'internat : elle veut voir les internes boursiers sans voir l'échelon | au 26 ; le 41 n'expose rien |

### P13 — M. Petit, accueil, vendredi 16 h
| M | Gr. | Manque | Ajout |
|---|---|---|---|
| M-4140 | H | 90 départs en 30 min : la remise un par un est trop lente | **remise par lot** par personne qui récupère (fratries, covoiturage déclaré par les titulaires) et pointage rapide « parti seul autorisé » ; interdiction toujours bloquante (INT-267) |
| M-4141 | M | Un élève part en covoiturage avec le parent d'un autre interne | personne autorisée **désignée par les titulaires** au 4 (covoiturage) ; sinon refus |

### P14 — M. Blanc, délégué à la protection des données
| M | Gr. | Manque | Ajout |
|---|---|---|---|
| M-4142 | H | Le **journal des lectures** de la fiche de nuit est prévu, mais pas sa **revue** | revue trimestrielle des lectures inhabituelles (surveillant hors service, lecture massive) — INT-268 |
| M-4143 | H | La **liste imprimée** des présents (R-IT42) est une copie papier de données de mineurs | consignes de destruction (broyeur le matin), mention imprimée « à détruire le … », pas de consigne de santé imprimée (codes seuls) |
| M-4144 | M | L'AIPD de l'internat (données de mineurs, nuit, faits de comportement) n'est pas mentionnée | **AIPD** requise avant mise en service (avec le 27) (Q-IT32) |
| M-4145 | M | Durées §21.3 : la liaison « année scolaire » reste longue pour des faits de comportement | liaison : **3 mois** pour les faits non publiés, année pour les publiés [R, DPO] |
| M-4146 | M | Information des familles et des élèves (art. 13) : notice spécifique internat | notice générée avec le règlement (7) |

### P15 — Les attaquants
| Visage | Attaque | Parade |
|---|---|---|
| Élève qui « découche » | Se connecte au compte de sa mère sur son propre téléphone et annonce « ne dort pas ce soir » | R-IT44 : passkey sur appareil inconnu ; avis immédiat aux titulaires ; attendu absent jusqu'à remise (M-4119) |
| Oncle interdit | Appelle et se fait passer pour le père ; vient le vendredi | CIT-50 ; remise bloquante (R-IT23) |
| Surveillant malveillant | Consulte la fiche de nuit de tous les internes, exporte les présents, modifie un appel après coup pour couvrir une absence | lecture limitée au **service en cours** et au périmètre (R-IT30) ; journal + revue (M-4142) ; correction après validation **visible et notifiée** (R-IT17) ; export des présents réservé à `internat.securite` |
| Robot / lien d'installation | Réserve tous les créneaux ou en efface | titulaire connecté ; lien nominatif à usage unique ; POST ; limiteur côté serveur partagé (pas en mémoire, DI-05) |
| Ex-AED | Garde l'appli sur son téléphone après la fin du contrat | fin d'engagement du 6 → sessions révoquées, données locales effacées au prochain démarrage (M-4147, haute, INT-269) |

### P16 — Florian, juin–juillet 2027
| M | Gr. | Manque | Ajout |
|---|---|---|---|
| M-4148 | **B** | Trou des autorisations annuelles et de l'installation (§0 D) | propriété par année scolaire (R-IT37 précisée), INT-266 |
| M-4149 | H | Le parallèle de juin demande aux surveillants **deux appels** : ils ne le feront pas | parallèle **sans double saisie** : ScolIA calcule l'attendu et le compare à l'appel Next.js importé chaque matin (export fourni par Florian) ; seuls 2 ou 3 volontaires pointent dans ScolIA |
| M-4150 | M | Rapprochement des internes JSON par nom : **les homonymes** | liste de cas présentée au CPE (déjà §26) avec photo et classe |

### P17 — Lycée public Jean-Moulin (EPLE)
| M | Gr. | Manque | Ajout |
|---|---|---|---|
| M-4151 | H | Le **règlement de l'internat** est voté en CA : versions, date d'adoption, date d'effet | version du règlement (7) avec date de vote ; les réglages §28 indiquent la version du règlement qui les fonde |
| M-4152 | M | Logement de fonction : CPE logé par nécessité absolue de service = astreinte de fait | fonction « logé NAS » au 6, proposée comme premier niveau d'astreinte [NV sur le texte] |
| M-4153 | M | Remise d'ordre d'hébergement selon le règlement de la **région** | causes de non-service codées compatibles avec les cas de la collectivité (24) |
| M-4154 | M | 300 internes : un appel par bâtiment avec **plusieurs surveillants** et des listes | prévu (listes, R-IT31) ; performance : attendu calculé < 1 s |

### Manques transverses
| M | Gr. | Manque | Ajout |
|---|---|---|---|
| M-4155 | H | **Téléphones de service** absents du modèle (numéro joignable du surveillant de nuit) | numéro du poste de nuit au 6 (poste `internat_nuit`), affiché aux familles pour l'urgence |
| M-4156 | M | IA : le résumé de la nuit peut **reformuler** un fait de liaison en jugement | résumé limité aux **codes** et comptes, sans réécriture des textes (R-IT33 précisé) |
| M-4157 | M | Tests : aucun test « nuit entière » de bout en bout | scénario de recette « une nuit » (§27) : 48 élèves, 3 absences annoncées, 1 cas, 1 malaise, hors ligne |
| M-4158 | M | Contrainte d'exclusion des sorties : le §24 l'annonce sans la définir | `EXCLUDE (etablissement_id, eleve_id, tstzrange(debut, fin_prevue) WITH &&) WHERE statut IN ('autorisee','partie')` |
| M-4159 | F | Accessibilité de l'écran d'appel la nuit (mode sombre, luminosité faible) | mode nuit de l'écran d'appel |
| M-4160 | F | Le plan sans nom (S-4113) ne s'applique pas à l'export papier | étiquette « interne » sur les exports |

---

## 5. Vérification réglementaire (2026-11-11)
- Repris du §5 du 41 sans changement de fond. Points **[V]** : référentiel national (non-mixité, espace fermant à clé, encadrement local) ; prime à l'internat 2026-2027 ; fiche CNIL vidéo de 2025.
- Points **[V-copie]** à relire sur Légifrance avant d'être codés en dur : article **R 33** (exercices de nuit), circulaire 96-248, circulaire 2000-106, R. 442-1 (registre des internes, hors contrat), L. 911-5, L. 511-5, circulaire PPMS de 2023.
- Points **[NV]** : taux d'encadrement (aucun texte), liste nominative des présents la nuit, permanence infirmière de nuit, logement NAS et astreinte (M-4152), proposition de loi sur l'honorabilité, circulaire « téléphone au lycée » du 2 juillet 2026, doctrine CNIL sur les durées.
- **Ajout** : l'AIPD (RGPD art. 35) est probable pour l'internat (mineurs, données de comportement, surveillance de nuit) [R, à confirmer avec le DPO] (M-4144).

---

## 6. Manques qui relèvent d'autres socles
| Socle | Manque |
|---|---|
| 4 | Personne autorisée « covoiturage » et « correspondant d'internat » avec jours ; déclaration du majeur « à charge » (M-4130, M-4141) |
| 6 | **Chaîne d'astreinte** à deux niveaux, téléphone du poste de nuit, fin d'engagement → révocation des appareils (M-4107, M-4147, M-4155) |
| 8 | Avis au fuseau du destinataire (M-4123) |
| 11 | Mode « service de nuit » : session courte, effacement local, appareil personnel (M-4102) |
| 14 | Remise par lot du vendredi (M-4140) |
| 18 | Autorisations 2027-2028 recueillies dans ScolIA dès juin (M-4148) |
| 27 | AIPD de l'internat, revue des lectures (M-4142, M-4144) |
| 34 | Fait « premiers secours donnés » sans N-1 (M-4137) |
| 35 | Codes fonctionnels « assistance à l'évacuation », « aide humaine de nuit » (M-4131, M-4133) |
| 36 | Préoccupation mise en avant pour les plus jeunes (M-4127) |

---

## 7. Synthèse

### 7.1 Manques par gravité
| Gravité | Nombre | Manques |
|---|---|---|
| **B** bloquant | 4 | M-4101, M-4107, M-4119, M-4148 |
| **H** haute | 20 | M-4102, M-4103, M-4108, M-4109, M-4112, M-4116, M-4120, M-4122, M-4125, M-4128, M-4131, M-4134, M-4136, M-4140, M-4142, M-4143, M-4147, M-4149, M-4151, M-4155 (plus les défauts DI-26, DI-27) |
| **M** moyenne | 33 | les autres |
| **F** faible | 3 | M-4139, M-4159, M-4160 |

### 7.2 Ajouts dans la conception (`41_INTERNAT.md` §30)
Actions **INT-250 à INT-269** (20), règles **R-IT42 à R-IT44** (3), cas limites **CIT-49 à CIT-62** (14), questions **Q-IT31 à Q-IT36**, lots révisés (hors ligne en I2), §31 faits, hypothèses, contradictions.

### 7.3 Les dix ajouts les plus importants
1. **Hors ligne dès I2** et liste des présents imprimée obligatoire (R-IT42).
2. **Chaîne d'astreinte** à deux niveaux avec bascule automatique et refus d'ouvrir la nuit sans astreinte (R-IT43).
3. **Annonce d'absence ≠ disparition de l'attendu** : « attendu absent » jusqu'à la remise ; passkey sur appareil inconnu (R-IT44).
4. **Propriété par année scolaire** pour la bascule : 2027-2028 naît dans ScolIA dès juin.
5. Vue **cas** pour l'astreinte à distance, accusés tracés, compte rendu de cas.
6. Marque **« vu, momentanément absent »** pour éviter les faux cas.
7. **Vigilance** codée sans texte (reprise de « sous surveillance »).
8. **Remise par lot** du vendredi.
9. Évacuation : **assistance** signalée sans motif ; dérogation de chambre **à l'élève**, sans révéler la raison.
10. **Revue des lectures**, destruction de la liste papier, AIPD.

### 7.4 Ce que la passe a montré
La conception pensait la nuit comme une suite d'écrans ; la nuit réelle est faite de **réseau absent**, d'un **adulte seul** et d'une **chaîne d'appel** qui doit tenir. Les ajouts portent presque tous sur la **continuité** (hors ligne, astreinte, papier) et sur **l'usurpation** (annonces, téléphone, appareils).

---

## 8. Décisions

### 8.1 Appliquées par défaut (D-031)
| # | Décision |
|---|---|
| 1 | Hors ligne de l'appel, de la fiche de nuit et des cas en I2 |
| 2 | Impression de la liste des présents prévus obligatoire, avec mention de destruction |
| 3 | Chaîne d'astreinte à deux niveaux, bascule à 10 min |
| 4 | Ouverture de la nuit refusée sans astreinte déclarée (alerte dès 17 h) |
| 5 | Annonce d'absence : attendu absent jusqu'à la remise ; passkey sur appareil inconnu |
| 6 | Propriété des données par année scolaire pendant la bascule |
| 7 | Marque « vu, momentanément absent » (15 min) |
| 8 | Vigilance codée sans texte, révisée chaque semaine |
| 9 | Dérogation de chambre portée par l'élève pour l'année |
| 10 | Remise par lot ; covoiturage seulement avec personne désignée |
| 11 | Liaison non publiée conservée 3 mois (sous réserve du DPO) |
| 12 | Révocation des appareils à la fin de l'engagement |

### 8.2 Soumis à Florian
| Q | Question | Recommandation |
|---|---|---|
| Q-IT31 | Téléphones de service pour les surveillants de nuit (coût) | oui, un par bâtiment ; à défaut, appareil personnel en mode « service de nuit » |
| Q-IT32 | AIPD de l'internat avec le DPO | à mener avant la mise en service (printemps 2027) |
| Q-IT33 | Chaîne d'astreinte de l'établissement (qui, quels numéros) | à fournir par la direction |
| Q-IT34 | Parallèle de juin 2027 : export quotidien de l'appel Next.js (geste de Florian) | oui, deux semaines |
| Q-IT1, Q-IT2, Q-IT3 | (déjà au 41) bascule, protocole, données médicales | inchangées |

---

## 9. Faits, hypothèses, contradictions
- **[F]** Marque « activité » de l'appel JSON ; sorties de week-end typées à motif libre ; réservation d'installation en attente 2 h ; aucun contrôle de sexe à l'affectation typée.
- **[H]** Réseau faible la nuit dans la plupart des internats ; surveillant seul par bâtiment ; usage d'appareils personnels par les AED.
- **Contradictions** : voir le §31 du 41 (T0 « beaucoup de TODO » contre aucun TODO trouvé ; « 80 fichiers » contre environ 50 propres ; planning des surveillants placé au 41 par `SOCLES.md` et au 6 par la conception ; INT-05 à INT-07 de `SOCLE2_DOMAINES.md` qui mêlent sorties de l'établissement et internat).
