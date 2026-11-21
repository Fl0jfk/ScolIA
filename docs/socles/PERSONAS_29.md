# Passe « personas » sur le socle 29 — Portails, « Aujourd'hui » et à-faire unifié

**Date :** 2026-11-11 — **Statut :** proposition, **non décidée** (à relire par Florian) ; les recommandations s'appliquent par défaut (D-031), sauf ce qui engage l'établissement à l'extérieur (§8.2). Aucun code écrit ni modifié ; aucun accès à une base, à un secret ni à un service de production ; Next.js lu seulement (`git show origin/main`, fichiers cités dans la conception §2.2).
**Branche :** `ent-socle-29-conception`, partie de `origin/ent-mc0hoe` (`7467f2d4`).
**Éprouve :** `docs/socles/29_PORTAILS.md` (lu en entier), avec `CLAUDE.md`, `docs/DECISIONS.md` D-013, D-017, D-023, D-028, D-031, **D-034**, **D-035**, **D-036**, la recherche réglementaire du §5 de la conception (arrêté du 7/09/2026), et les conceptions des socles qui déposent des tâches (recensement de la conception §6.2). Forme reprise de `PERSONAS_10.md` et `PERSONAS_42.md`.
**Contexte imposé :** l'accueil de chaque profil est la **page d'ouverture de la démo de décembre** (D-034) ; l'année 2027-2028 est une année de **doublon** avec Charlemagne, sauf les modules de l'add-on en usage réel (D-035).
**Ajouts :** faits **directement** dans `29_PORTAILS.md`, §29 « Compléments de la passe personas », marqués « ajout personas » ; **rien n'est supprimé**.
**Numérotation :** manques **M-2900 à M-2947** ; actions **POR-252 à POR-266** ; règles **R-PO33 à R-PO38** ; cas limites **CPO-73 à CPO-86** ; questions **Q-PO11 à Q-PO14** ; réglages **S-2912 à S-2914**.

---

## 0. À lire en premier : quatre bloquants et leur recommandation (appliquée par défaut, D-031)

| # | Point | Pourquoi c'est bloquant | Recommandation (appliquée par défaut) | Lots |
|---|---|---|---|---|
| **A** | **Le cache hors ligne du mobile survit à la suspension d'un lien.** La conception (§14.3) garde « la dernière version de l'accueil » lisible hors ligne. Un parent dont le lien vient d'être suspendu par une décision de justice (violences) garde, sur son téléphone, **l'emploi du temps du jour de l'enfant, ses salles, sa sortie de l'après-midi**, tant que l'appli ne recontacte pas le serveur — et il peut la tenir hors ligne volontairement (P15-c). | C'est une information de **localisation d'un enfant protégé**. Aucun autre défaut de la conception n'a cette gravité. | **R-PO33** : le cache hors ligne d'un responsable **ne contient jamais l'emploi du temps au-delà de la journée en cours**, expire au bout de **24 h** sans contact serveur, et est **effacé** au premier contact après une suspension, un retrait d'adhésion ou une révocation de session (envoi silencieux de purge par le socle 11, à l'arrivée du fait `lien.suspendu`) ; la révocation de toutes les sessions du parent est **proposée** au secrétariat lors de la saisie de la suspension (socle 4). | P8 (condition d'entrée), socles 4 et 11 |
| **B** | **Les compteurs multi-établissements du web (P9) peuvent révéler qu'une personne est inscrite ailleurs.** La conception autorise un appel du navigateur vers chaque sous-domaine `*.scolia.fr`, CORS « limité aux sous-domaines ». Si un sous-domaine d'établissement héberge un jour du contenu fourni par l'établissement (site public, socle N3, affiches), une page de cet établissement peut appeler `/portail/compteur` des autres sous-domaines avec les cookies du visiteur et apprendre **dans quels autres établissements il a une session**. | Contraire à D-013 et à l'arrêt « un établissement n'apprend jamais l'existence d'un autre » (04 : 632). | **R-PO34** : P9 ne passe **pas** par CORS entre sous-domaines. Les compteurs sont servis par le **guichet central** `connexion.scolia.fr` (socle 9), qui connaît déjà les adhésions de l'identité, sous forme d'une **liste signée et à durée courte** que le navigateur présente à chaque sous-domaine ; chaque établissement répond seulement s'il figure dans la liste ; **aucun contenu fourni par un établissement n'est servi depuis un sous-domaine qui porte des sessions** (le site public, s'il existe, vit sur un autre domaine). À défaut, P9 se limite au **sélecteur**. | P9, socle 9, futur N3 |
| **C** | **Le mode discret ne protège pas l'information qui doit, elle, rester visible.** À l'accueil (P06), l'écran est tourné vers les visiteurs ; le mode discret masque les noms. Mais lors d'une remise d'élève, l'**interdiction de remise** (décision de justice) doit sauter aux yeux de l'agent **sans** être lisible par la personne en face. La conception ne dit rien de cette tension. | Remettre un enfant à une personne interdite est le pire incident possible de l'accueil ; afficher le motif d'une interdiction devant un visiteur en est un autre. | **R-PO35** : en mode discret, une **alerte de remise** reste affichée sous forme neutre (« Attention : restriction de remise — vérifier avant toute sortie », pictogramme, couleur avec libellé), le détail n'apparaissant qu'après un geste de l'agent (appui long, confirmation) ; jamais le motif ni le nom de la personne interdite en clair à l'écran d'accueil ; source : `qui_peut_recuperer` du socle 4 (ADM-44), qui reste l'autorité. | P1 (tuile accueil), P7 |
| **D** | **L'accueil de la démo de décembre (P1) serait écrit deux fois.** P1 compose les tuiles « sans index », en appelant directement les routes existantes ; P2 ajoute l'index. Si P1 code ses tuiles en dur dans un gestionnaire `aujourdhui` (comme Next.js, NXP-03 : une fonction de 2 000 lignes), P2 devra tout refaire et chaque socle devra être rebranché. | C'est exactement le télescopage que D-026 veut supprimer, et la démo ne doit pas créer de dette au cœur. | **R-PO36** : dès P1, chaque tuile est une **déclaration au contrat de tuile** (§9.1) dans la brique de son socle (`rendre_tuile`), même si elle lit une route existante ; le composeur du 29 ne connaît **aucune** tuile par son nom. P1 livre donc le **contrat de tuile** et le composeur ; P2 ajoute le contrat de nature et l'index. | P1, kit D-026 |

---

## 1. Méthode

Seize personas réalistes, prises dans le tableau des acteurs de la conception (§3) et dans les cas difficiles des passes précédentes (parents séparés du socle 24–25, AESH du socle 10, accueil du socle 4, DPO du socle 42). Pour chacune : sa journée (ou son moment), ce qu'elle cherche sur l'accueil, ses irritants, ses peurs, ce qui la ravirait ; puis les **manques** (gravité : **B** bloquant, **I** important, **C** confort, **D** différenciant), les **contradictions** et les **cas limites**. Le persona attaquant (P15) éprouve les garde-fous du §15 ; le DPO (P14) les règles du §5 et du §22. Chaque manque renvoie à son ajout dans la conception (§29).

## 2. Les personas en un tableau

| # | Persona | Profil | Établissement | Ce qui est éprouvé |
|---|---|---|---|---|
| P01 | Mme Martin, mère de trois enfants | responsable, autorité conjointe, payeuse | collège pilote + lycée d'un autre établissement ScolIA | multi-enfants, multi-établissements, bus à 7 h 40 |
| P02 | M. Garnier, père séparé | autorité conjointe, payeur à 50 %, autre foyer | collège pilote | neutralité entre parents |
| P03 | Mme Benali, CPE | vie scolaire | lycée de 1 500 élèves | « maintenant », files, AED |
| P04 | M. Haddad, professeur partagé et PP | professeur, PP de 3ᵉ | collège A et lycée B | journée réunie, PP, projection |
| P05 | Mme Diallo, AESH | AESH, téléphone personnel | collège pilote | journée des élèves accompagnés |
| P06 | Mme Lefèvre, accueil | accueil, poste vu des visiteurs | lycée pilote | remise, appel d'un parent, mode discret |
| P07 | M. Roux, secrétariat de scolarité | secrétariat | lycée pilote, rentrée | volume, recherche, files |
| P08 | Mme Duval, cheffe d'établissement | direction | lycée pilote + école (autre établissement) | « ne peut pas attendre », supervision |
| P09 | Léa, 13 ans, 4ᵉ | élève | collège pilote, téléphone partagé avec sa sœur | élève mineure, discrétion |
| P10 | Hugo, 18 ans, terminale | élève majeur | lycée pilote | bascule de majorité |
| P11 | Famille Petit, allophone | responsable sans e-mail, téléphone d'entrée de gamme | collège pilote | langue, performance, sans compte |
| P12 | M. Perrin, père sans exercice de l'autorité | responsable, jugement | autre ville | information des choix importants |
| P13 | Mme Caron, infirmière | infirmerie | lycée pilote | santé hors de l'accueil des autres |
| P14 | M. Blanc, DPO | auditeur RGPD | OGEC et EPLE | minimisation, durées, droit d'accès |
| P15 | Les attaquants | élève curieux, parent d'un autre foyer, parent suspendu, agent indélicat, établissement malveillant | — | séparation, canaux cachés |
| P16 | Florian, administrateur | administrateur, démonstrateur | labo, 30 novembre 2026 au soir | démo de décembre |

---

## 3. Parcours et manques, persona par persona

### P01 — Mme Martin, trois enfants, deux établissements, dans le bus à 7 h 40

**Sa matinée.** Elle ouvre l'appli au feu rouge du bus. Elle veut savoir : la première heure de Tom (6ᵉ) a-t-elle lieu ? Ninon (3ᵉ) a-t-elle son brevet blanc aujourd'hui ? Y a-t-il quelque chose à signer pour Jules (lycée, autre établissement) ?
**Irritants** : trois applis l'an dernier ; une pastille rouge « 7 » qui n'était que des actualités. **Peurs** : que Tom attende devant la grille parce qu'un cours a été annulé la veille au soir. **Ravie si** : une seule page, trois couleurs, « Rien à signer ».

| # | Manque | Gravité | Ajout |
|---|---|---|---|
| M-2900 | **Plage calme et annulation du lendemain** : un cours de 8 h annulé à 21 h n'est visible qu'à 7 h (R-PO31) ; le bus de Tom part à 7 h 10. L'arrêté permet au chef d'établissement d'ajuster pour l'urgence, mais la conception ne dit pas quelles natures entrent dans cet ajustement. | **I** | R-PO37 : liste d'**ajustements préparés** proposée au chef (cours du lendemain avant 10 h annulé ou déplacé, fermeture, alerte) : visibles **en consultation** dès leur publication, **sans aucune notification** pendant la plage ; la liste est publiée aux familles (« ce que vous pouvez voir le soir ») ; lecture juriste (Q-PO1) |
| M-2901 | Sur le web, pas de page réunie des deux établissements (seulement un sélecteur, puis des compteurs) | C | assumé (R-PO17) ; l'accueil web indique « Voir les deux établissements ensemble : appli mobile » |
| M-2902 | Elle veut voir **l'heure de fin** de la journée de chaque enfant (pour l'organisation du soir) | C | tuile « Aujourd'hui de… » : première et dernière heure en tête (POR-252) |
| M-2903 | La pastille « 7 » de l'ancien outil comptait des actualités : elle veut savoir ce que compte la pastille | C | infobulle « 3 choses à faire » (R-PO4 déjà), texte explicatif dans l'aide |

### P02 — M. Garnier, père séparé, autorité conjointe, payeur à 50 %

**Son moment.** Semaine sans les enfants. Le dimanche soir, il ouvre l'à-faire : « Autoriser la sortie au Louvre de Tom ». Il commence à remplir, s'interrompt. Le lundi, la tâche a disparu.
**Peurs** : que son ex-conjointe sache ce qu'il a fait ou lu ; payer la part de l'autre. **Ravi si** : il n'apprend rien d'elle, elle rien de lui.

| # | Manque | Gravité | Ajout |
|---|---|---|---|
| M-2904 | **Canal temporel** : si la tâche disparaît à l'instant où l'autre parent signe, un parent qui rafraîchit sait **quand** l'autre a agi (et donc, par exemple, qu'elle était réveillée à 23 h 02 ou en ligne pendant un rendez-vous). R-V46 (21) et R-PO7 ne couvrent que le nom et la date affichés. | **I** | R-PO38 : une tâche close par **un autre responsable d'un autre foyer** reste affichée jusqu'au **prochain point de rafraîchissement neutre** (7 h le lendemain ou ouverture de session suivante, au plus tard 24 h) ; toute action dessus répond « Cette démarche est déjà traitée » (même code, même texte, même temps de réponse que pour une tâche close par l'établissement) |
| M-2905 | Son brouillon de réponse est perdu sans explication | C | message neutre à l'envoi (A11 déjà) ; brouillon gardé localement 7 jours pour sa propre trace |
| M-2906 | Il voudrait que l'à-faire distingue **ses** semaines de garde (calendrier de garde du 4, CN-69 du 8) | C | filtre « mes semaines » si le calendrier de garde est renseigné ; aucune donnée de l'autre (POR-253) |
| M-2907 | La tuile « Compte famille » affiche « solde » : il craint d'y voir la dette de l'autre | I (vérification) | confirmé par R-PO2 et 24 (CF-94) ; test ajouté (§25 : « part de l'autre payeur jamais visible ») |

### P03 — Mme Benali, CPE d'un lycée de 1 500 élèves, 8 h 10

**Sa matinée.** Elle veut les appels non faits **de la séance en cours** seulement, les absents non signalés, et parmi eux ceux qui l'inquiètent ; deux AED l'aident à trier les justificatifs.
**Irritants** : téléphoner aux professeurs ; relancer à tort un professeur dont le cours a été annulé. **Peurs** : rater l'absence inquiétante. **Ravie si** : « 3 appels non faits » est vrai, et elle relance en un geste.

| # | Manque | Gravité | Ajout |
|---|---|---|---|
| M-2908 | « Appels non faits » doit exclure les séances annulées, en sortie, en examen, et celles où le remplaçant a fait l'appel | I | la tuile est **déclarée par le 12**, qui calcule ; le 29 n'y ajoute rien (rappel au §29) |
| M-2909 | **Absences inquiétantes** : aucune tuile dédiée (premier jour d'absence d'un élève suivi, élève protégé absent sans signalement, absence après un incident) | **I** | tuile `vs.absences_inquietantes` **déclarée par le 12** (signal déterministe, explication, aucun motif d'accompagnement affiché ; le critère « suivi » vient des droits de la CPE, 36) — demande au socle 12 (§6 de cette passe) |
| M-2910 | **Files à deux niveaux** : les AED préparent les justificatifs (vérifient la pièce), la CPE décide ; la conception n'a qu'une prise en charge | **I** | file à **deux niveaux** déclarables (`preparer`, `decider`) : état `en_attente` « préparé par X, à décider », droits distincts (POR-254, POR-255) |
| M-2911 | Elle veut voir « où est … » pour dix élèves d'un coup (sortie du car) | C | renvoyé à l'écran du 12 (liste) ; l'accueil n'en fait qu'un |

### P04 — M. Haddad, professeur partagé entre un collège et un lycée, PP de 3ᵉ

**Sa journée.** 8 h–11 h au collège A, 13 h–17 h au lycée B. Il projette parfois son écran pour montrer le cahier de textes.
**Irritants** : deux applis, deux EDT ; les relances du PP par courriel. **Peurs** : projeter par erreur les absences d'un élève. **Ravi si** : une journée, deux couleurs.

| # | Manque | Gravité | Ajout |
|---|---|---|---|
| M-2912 | **Poste de classe partagé** (ordinateur de la salle, tableau interactif) : le mode discret manuel s'oublie | **I** | S-2912 « poste partagé » : un poste déclaré par l'établissement (socle 44 ou 1) ouvre toujours l'accueil en mode discret et ferme la session à la fin de la séance (POR-256) |
| M-2913 | PP : « appréciations manquantes **par collègue** » nomme des collègues — pression entre pairs, alors que le 10 interdit de nommer la contrainte d'un collègue (R-E31) | C | affiché **par matière** (comme la préparation du conseil du 23, journalisé) ; relance par le PP via la commande du 23, sans liste nominative sur l'accueil |
| M-2914 | Première connexion au second établissement sur l'appli : parcours non décrit | C | socle 11 (rattachement par lien ou QR) ; l'accueil propose « Ajouter un établissement » |
| M-2915 | Conflit horaire entre ses deux établissements : il veut pouvoir le **signaler** à l'adjoint, sans que l'appli le fasse à sa place | C | bouton « Signaler à … » qui ouvre un message pré-rempli **qu'il envoie lui-même** à un seul établissement (POR-257) |

### P05 — Mme Diallo, AESH, téléphone personnel

**Sa journée.** Elle accompagne Léo (5ᵉ) et Sami (4ᵉ) ; son emploi du temps suit le leur.
**Irritants** : arriver dans une salle vide. **Peurs** : lire sur son téléphone, dans le métro, un nom d'élève en clair. **Ravie si** : prévenue en premier d'un changement.

| # | Manque | Gravité | Ajout |
|---|---|---|---|
| M-2916 | Le travail à faire de ses élèves (utile pour les aider) n'est ouvert à aucun droit de l'AESH | I | demande aux socles 20 et 35 : lecture « travail de mes élèves accompagnés » par la mission (355) ; tuile déclarée par le 20 |
| M-2917 | Notification d'un changement de salle : nom de l'élève sur l'écran verrouillé | I (vérification) | R-PO30 confirmé ; texte « Changement pour un élève accompagné à 10 h » |
| M-2918 | Elle n'a pas de modèle de rôle par défaut dans la conception | C | modèle « AESH » ajouté aux tuiles par défaut (§29) |

### P06 — Mme Lefèvre, accueil, écran visible des visiteurs

**Sa matinée.** Un parent appelle : Tom est malade. Un grand-père vient chercher Ninon à 11 h. Un livreur attend.
**Irritants** : le tableur partagé avec la vie scolaire. **Peurs** : remettre Ninon à la mauvaise personne. **Ravie si** : « Déclarer une absence » en deux gestes, « qui peut venir chercher Ninon » en un.

| # | Manque | Gravité | Ajout |
|---|---|---|---|
| M-2919 | **Interdiction de remise en mode discret** (bloquant C) | **B** | R-PO35 |
| M-2920 | La palette « Déclarer une absence » doit permettre la recherche de l'élève **par le nom du parent** qui appelle (« Je suis Mme Martin ») | I | port de recherche du 4 ouvert à l'accueil en mode « retrouver l'élève depuis un responsable » : résultat = élèves liés, minimal (POR-258) ; la déclaration reste celle du 12 |
| M-2921 | Les visiteurs attendus doivent apparaître sans motif (« RDV CPE 10 h ») | I (vérification) | titres neutres (R-PO5) ; tuile déclarée par le 14 |

### P07 — M. Roux, secrétariat de scolarité, rentrée

**Sa semaine.** 600 dossiers, 300 tâches, des centaines de recherches.
**Irritants** : être bloqué par un plafond de recherche un jour de rentrée. **Peurs** : une échéance BEE ratée.

| # | Manque | Gravité | Ajout |
|---|---|---|---|
| M-2922 | Plafond de recherche fixe (S-2907) : la rentrée multiplie les recherches légitimes | I | plafond **par rôle et par période** (rentrée = ×3), déblocage en libre-service une fois par jour avec motif (journalisé), puis administrateur (POR-259) |
| M-2923 | L'échéance nationale (BEE, LSU) doit être une **tâche** avec compte à rebours, pas seulement une tuile | I | nature `ech.echeance_nationale` déclarée par le 9 (tâche collective du secrétariat, priorité montante) — demande au socle 9 |
| M-2924 | 300 tâches de la même nature rendent l'à-faire illisible | C | regroupement déjà (§8.6) ; la tuile de file renvoie au traitement en série du 3 |

### P08 — Mme Duval, cheffe d'un lycée et directrice d'une école (deux établissements)

**Sa journée.** Elle veut ce qui ne peut pas attendre, et déléguer durablement à son adjoint ce qui relève de lui.
**Peurs** : une décision bloquée en silence (D-028). **Ravie si** : « 0 décision en attente ».

| # | Manque | Gravité | Ajout |
|---|---|---|---|
| M-2925 | Délégation **permanente par nature** (« les conventions de stage, c'est l'adjoint ») : les délégations du 6 sont datées et par droits | I | demande au socle 6 : délégation sans date de fin par **nature de tâche**, révocable ; le 29 indexe chez le délégataire « pour Mme Duval » (POR-260) |
| M-2926 | **Supervision et surveillance des agents** : « tâches en retard par file » d'une file à une seule personne désigne cette personne ; c'est un contrôle de l'activité des agents | **I** | R-PO : supervision **sans classement de personnes** ; une file à moins de 3 membres n'affiche que « en retard : oui / non » ; information préalable des personnels dans la charte (socle 6) ; avis du DPO (Q-PO11) |
| M-2927 | Les deux établissements : son appli réunit les deux « ne peut pas attendre » | C (vérification) | R-PO17 : réunion sur l'appareil ; aucune vue serveur commune |

### P09 — Léa, 13 ans, 4ᵉ, téléphone partagé avec sa sœur

| # | Manque | Gravité | Ajout |
|---|---|---|---|
| M-2928 | Deux élèves sur le même téléphone : chacun son compte ; la conception ne dit rien du changement de personne sur l'appli | I | socle 11 : « changer de personne » avec passkey ; l'accueil ne montre jamais les deux personnes à la fois ; notifications neutres |
| M-2929 | La porte « Parler à un adulte » (vers le 36) : elle ne doit apparaître ni dans « fait récemment », ni dans l'historique de la palette, ni dans l'IA | **I** | R-PO : la tuile de la porte n'a ni compteur ni état ; l'action n'est jamais inscrite dans les états personnels ni dans la mémoire locale de la palette (POR-201 exclut cette entrée) |
| M-2930 | Notes « sur geste » : elle veut quand même savoir qu'une note **nouvelle** existe | C | « 1 note nouvelle — afficher » (sans valeur) ; réglable par l'élève |

### P10 — Hugo, 18 ans le 14 mars

| # | Manque | Gravité | Ajout |
|---|---|---|---|
| M-2931 | Le jour de ses 18 ans, ses parents perdent les tâches de décision en cours (sortie de mai à autoriser) sans explication | I | message neutre aux parents « Les démarches de Hugo sont désormais suivies par lui » **si** le 4 maintient l'information des parents, sinon rien ; Hugo reçoit les tâches avec leur historique (§8.8 déjà, précisé) |
| M-2932 | La tuile « Ce que voient vos parents » doit être **exacte** et dater ses changements | I | rendue par le socle 4 (source unique) ; journal visible de Hugo |

### P11 — Famille Petit, allophone, sans e-mail, téléphone d'entrée de gamme

| # | Manque | Gravité | Ajout |
|---|---|---|---|
| M-2933 | **Budget de poids** de l'accueil non fixé : une appli web Leptos avec WebAssembly peut peser plusieurs centaines de kilo-octets | **I** | §14.4 complété : accueil lisible **sans WebAssembly** (rendu serveur), premier affichage < 100 Ko transférés sur 3G, îlots hydratés seulement pour les cartes d'action |
| M-2934 | Les cartes d'action en ukrainien doivent garder l'**aperçu juridique** en français | I (vérification) | §14.2 confirmé ; aperçu bilingue |
| M-2935 | Sans e-mail ni compte : la famille ne voit rien, la vie scolaire doit le savoir | I (vérification) | CPO-18 confirmé ; tuile « familles à relancer autrement » (NTF-63) |

### P12 — M. Perrin, père sans exercice de l'autorité parentale

| # | Manque | Gravité | Ajout |
|---|---|---|---|
| M-2936 | **« Choix importants »** (373-2-1 C. civ.) : la conception l'informe « selon le lien » mais aucune liste ne dit **quels faits** sont des choix importants | **I** | catégorie d'information `choix_important` déclarable par les socles 3 (changement d'établissement, départ), 23 (décision de passage, redoublement), 37 (orientation), 13 (exclusion définitive), 42 (séjour à l'étranger), 35 (projet personnalisé) : tâche `information` avec accusé facultatif au parent sans exercice dont le lien prévoit l'information ; liste soumise au juriste (Q-PO12) (POR-261) |
| M-2937 | Il ne doit pas voir les boutons de décision, même grisés | C | R-PO8 (aucune tâche de décision) ; aucun bouton grisé |

### P13 — Mme Caron, infirmière

| # | Manque | Gravité | Ajout |
|---|---|---|---|
| M-2938 | Traitements à heure fixe (12 h) : la tâche doit **sonner** même si l'infirmière a mis son téléphone en plage calme | I | nature santé à urgence déclarée (socle 34), hors plage calme **pour les personnels habilités** seulement ; le 29 l'affiche en tête, le 8 la notifie |
| M-2939 | Module santé désactivé (D-034, pas d'hébergeur HDS) : son accueil ne doit pas être vide | C | tuile « Infirmerie (registre de passage minimal) » selon N-0 / N-1 du 34 |

### P14 — M. Blanc, DPO de l'OGEC et d'un EPLE

**Sa lecture.** Il cherche ce que l'accueil **garde** sur les personnes.

| # | Manque | Gravité | Ajout |
|---|---|---|---|
| M-2940 | **`vue_le` par tâche** est une trace de comportement de lecture des familles (« vous aviez vu la convocation ») ; pourrait être demandée dans un litige alors que la preuve de notification relève des socles 7 et 8 (D-027 D) | **I** | `ent_afaire_etat_personnel.vue_le` remplacé par un **booléen « nouveau / vu »** sans horodatage, effacé à la clôture ; une seule date globale par compte (`derniere_visite_accueil`) pour « ce qui a changé » ; aucune route ni export ne s'en sert comme preuve (R-PO : non probant) |
| M-2941 | **Rappels personnels et droit d'accès** : un rappel lié à un élève (« Tom — appeler la mère ») est une donnée sur Tom ; la règle « jamais visible d'un autre » empêche de le retrouver pour une demande d'accès de Tom (RGPD art. 15) | **I** | les rappels **liés à un élève** sont inclus dans l'extraction du droit d'accès de cet élève (procédure du socle 27, par le DPO) ; l'avertissement de création le dit ; sans lien, texte libre limité et avertissement renforcé |
| M-2942 | **AIPD** : l'accueil agrège des données de mineurs de tous les domaines ; la conception ne dit pas qu'il entre dans l'analyse d'impact | I | renvoi au socle 27 : le portail est inscrit comme composante de l'AIPD de l'ENT (Q-PO11) |
| M-2943 | Journal d'états 12 mois avec `par_compte_id` : traçabilité des agents (qui a traité quoi) — légitime, mais à annoncer | C | information des personnels (charte, socle 6) |
| M-2944 | Mesure d'audience dans le privé (pas de DNMA) : qui la fait, avec quoi ? | C (vérification) | R-PO28 et §5.5 : interne, agrégée, sans identifiant |

### P15 — Les attaquants

| Attaque | Résultat attendu | Manque éventuel |
|---|---|---|
| (a) Kévin, élève, appelle `/portail/tuiles/vie_scolaire?eleve_id=<camarade>` | refus : le contexte d'une tuile d'élève est **la session**, jamais un paramètre pour un élève ou une famille | M-2945 (**I**) : le contrat de tuile doit interdire un `eleve_id` fourni par le client sauf pour un responsable, revérifié par `liens_de` ; test de séparation généré (POR-262) |
| (b) Parent d'un autre foyer rejoue l'URL d'une carte d'action avec un autre `afaire_id` | refus du propriétaire (revérification) ; réponse identique à « tâche inexistante » | — |
| (c) Parent suspendu qui garde l'appli hors ligne | **bloquant A** (R-PO33) | M-2946 (**B**) |
| (d) Agent indélicat qui énumère les élèves par la recherche | compteur, ralentissement, tâche à l'administrateur ; temps de réponse identiques | — |
| (e) Établissement malveillant ou compromis qui sonde les autres établissements de ses visiteurs | **bloquant B** (R-PO34) | M-2947 (**B**) |
| (f) Lien piégé « exécute l'action » (`/web/a-faire?executer=…`) envoyé par message | aucune action à l'ouverture d'une adresse : toute action exige un geste dans la page et son aperçu (comme les pages de récupération du socle 1, `FRONTEND_CIBLE.md` §9) | R-PO (précisé §29) |
| (g) Titre de tâche avec balisage (nom d'élève « `<img onerror>` ») | échappement Leptos, gabarit typé, CSP à nonce | — |
| (h) Requête intersite (CSRF) sur une carte d'action | cookie `SameSite`, jeton anti-CSRF du socle 1, méthode non sûre seulement | — |

### P16 — Florian, administrateur, le 30 novembre 2026 au soir

| # | Constat | Gravité | Ajout |
|---|---|---|---|
| — | La démo passe par sept profils ; chaque accueil doit avoir des données de labo **réalistes** (tâches, changements, rendez-vous) | I | jeu de démonstration du socle 28 complété pour l'accueil (scénario « lundi 8 h ») (POR-263) |
| — | Le lien « Ouvrir l'interface actuelle » ne doit plus apparaître dans la démo | I | P1 supprime la tuile « Votre espace » pour les profils couverts |
| — | **Bloquant D** : P1 doit être écrit sur le contrat de tuile | **B** | R-PO36 |
| — | Il veut présenter l'accueil d'un parent sans se connecter en parent | C | « voir comme » fictif (POR-223) prêt dès P1 (POR-264) |

### Manques transverses : mobile, IA, tests

- **Mobile** : pastille et plage calme (la pastille d'icône ne doit pas augmenter pendant la plage pour une famille) → R-PO31 s'applique aussi à `compteur_a_faire` (POR-265).
- **IA** : « que dois-je faire » doit refuser de répondre pour un autre enfant que ceux du lien (test d'évaluation ajouté).
- **Tests** : neutralité temporelle (M-2904) ; purge du cache hors ligne (M-2946) ; absence de CORS entre sous-domaines (M-2947) ; poste partagé (M-2912).

---

## 4. Vérification réglementaire (2026-11-11)

Faite pour la conception (§5 de `29_PORTAILS.md`, sous-agent, sources officielles). Points utilisés par cette passe : **plage calme** de l'arrêté du 7/09/2026 (M-2900, M-2938, POR-265) ; **373-2-1 C. civ.** « informé des choix importants » [V] (M-2936) ; circulaire n° 2006-137 « deux interlocuteurs » [V] (P02) ; RGPD art. 15 et rôle du DPO (M-2941) ; mesure d'audience CNIL (M-2944). **Non vérifié** : encadrement de la supervision de l'activité des agents publics par un outil métier (M-2926) — à faire avec le DPO (Q-PO11) [NV].

---

## 5. Contradictions relevées

| # | Contradiction | Traitement |
|---|---|---|
| C-01 | §14.3 (hors ligne lisible) contre la protection d'un enfant (lien suspendu) | R-PO33 (bloquant A) |
| C-02 | §11.3 (compteurs web par CORS) contre D-013 | R-PO34 (bloquant B) |
| C-03 | §14.5 (mode discret masque tout) contre la sécurité de la remise | R-PO35 (bloquant C) |
| C-04 | §24 (P1 « sans index », lectures directes) contre D-026 | R-PO36 (bloquant D) |
| C-05 | R-PO31 (rien de nouveau visible pendant la plage) contre le besoin d'une famille de savoir le soir qu'un cours du lendemain 8 h est annulé | R-PO37 (ajustements préparés du chef, Q-PO1) |
| C-06 | §8.4 `vue_le` horodaté contre la minimisation (D-027 D : la preuve de notification est ailleurs) | M-2940 |
| C-07 | §8.12 « rappels jamais visibles d'un autre » contre le droit d'accès de l'élève cité | M-2941 |
| C-08 | §15.1 `afaire.superviser` contre la protection des agents (petite file = une personne) | M-2926 |

## 6. Manques qui relèvent d'autres socles (à leur transmettre)

| Manque | Socle | Demande |
|---|---|---|
| M-2908, M-2909 | 12 | tuiles « appels non faits » (exclusions) et « absences inquiétantes » (signal déterministe expliqué) |
| M-2916 | 20, 35 | lecture du travail des élèves accompagnés par l'AESH |
| M-2923 | 9 | nature « échéance nationale » |
| M-2925 | 6 | délégation permanente par nature |
| M-2926, M-2943 | 6 | charte : information des personnels sur la supervision et le journal |
| M-2928, M-2946 | 11 | changement de personne ; purge silencieuse ; expiration du cache |
| M-2931, M-2932 | 2, 4 | message de bascule de majorité ; « ce que voient vos parents » |
| M-2936 | 3, 13, 23, 35, 37, 42 | déclarer les faits « choix important » |
| M-2938 | 34, 8 | nature à urgence hors plage pour l'infirmerie |
| M-2941, M-2942 | 27 | rappels liés dans le droit d'accès ; AIPD |
| M-2947 | 9, N3 | liste signée des adhésions ; aucun contenu d'établissement sur un sous-domaine à sessions |
| bloquant A | 4 | proposer la révocation des sessions à la saisie d'une suspension |

---

## 7. Synthèse

### 7.1 Manques par gravité

48 manques (M-2900 à M-2947) : **3 bloquants numérotés** (M-2919, M-2946, M-2947) plus le **bloquant D**, constat d'architecture de P16 sans numéro, soit quatre points (A à D) ; **28 importants** (dont 6 « vérification » qui confirment une règle existante et ajoutent un test) ; **17 confort**. Trois manques sont aussi **différenciants** : la neutralité temporelle entre parents (M-2904), les ajustements préparés de la plage calme (M-2900) et la journée réunie du professeur partagé (M-2915, avec CPO-31).

### 7.2 Ajouts dans la conception (`29_PORTAILS.md` §29)

- Règles **R-PO33 à R-PO38** (cache hors ligne, compteurs multi-établissements, remise en mode discret, P1 sur le contrat de tuile, ajustements préparés de la plage calme, neutralité temporelle entre parents) et précisions de R-PO (supervision sans classement, porte « parler à un adulte », aucune action à l'ouverture d'une adresse, `vue_le` non probant).
- Actions **POR-252 à POR-266**.
- Cas limites **CPO-73 à CPO-86**.
- Modèle : `vue_le` → booléen ; `ent_portail_poste_partage` ; colonne `derniere_visite_accueil` ; deux niveaux de file.
- Lots : P1 livre le contrat de tuile et le composeur ; P8 a pour condition d'entrée R-PO33 ; P9 est réécrit (R-PO34).
- Questions **Q-PO11 à Q-PO14**, réglages **S-2912 à S-2914**.

### 7.3 Les dix ajouts les plus importants

1. Purge et expiration du cache hors ligne d'un responsable (R-PO33).
2. Compteurs multi-établissements par le guichet central, jamais par CORS (R-PO34).
3. Alerte de remise neutre mais visible en mode discret (R-PO35).
4. P1 écrit sur le contrat de tuile (R-PO36).
5. Ajustements préparés de la plage calme pour le chef d'établissement (R-PO37).
6. Neutralité **temporelle** entre parents séparés (R-PO38).
7. Information des « choix importants » au parent sans exercice (POR-261).
8. Files à deux niveaux, préparer / décider (POR-254, POR-255).
9. `vue_le` réduit à un booléen non probant ; rappels liés dans le droit d'accès.
10. Supervision des files sans classement de personnes.

### 7.4 Ce que la passe a montré sur la conception

La conception tient sur l'essentiel (le 29 ne possède rien, l'index est revérifié, les droits sont par enfant, la neutralité entre parents est posée). Ses faiblesses sont toutes aux **frontières de l'appareil** (cache hors ligne, poste partagé, écran vu par un tiers, sous-domaines) et dans le **temps** (moment de disparition d'une tâche, plage calme) : des canaux que les autres socles n'ont pas, parce que l'accueil est l'écran le plus regardé.

---

## 8. Décisions : appliquées par défaut (D-031) et soumises à Florian

### 8.1 Appliquées par défaut (D-031)

1. R-PO33 à R-PO38 (bloquants A à D, C-05, M-2904).
2. Files à deux niveaux ; supervision sans classement de personnes, file < 3 membres en « oui / non ».
3. `vue_le` remplacé par un booléen ; date de dernière visite globale par compte.
4. Rappels liés à un élève inclus dans son droit d'accès.
5. Catégorie « choix important » déclarable, information du parent sans exercice.
6. Poste partagé en mode discret et fermeture de session en fin de séance.
7. Budget de poids de l'accueil (lisible sans WebAssembly, < 100 Ko au premier affichage).
8. Les quinze recommandations de la conception (§27.1).

### 8.2 Soumis à Florian

| # | Question | Recommandation |
|---|---|---|
| Q-PO11 | **DPO** : supervision des files (activité des agents), journal d'états nominatif 12 mois, inscription du portail à l'AIPD de l'ENT | à valider avec le DPO au printemps 2027, avec Q-PO5 |
| Q-PO12 | **Juriste** : liste des « choix importants » à communiquer au parent sans exercice de l'autorité parentale | à faire valider avec Q-PO1 et Q-PO2 |
| Q-PO13 | **Établissement pilote** : y a-t-il des postes de classe partagés (tableaux interactifs, ordinateurs de salle) à déclarer ? | fait de terrain |
| Q-PO14 | **Démo de décembre** : quels sept profils montrer, et sur quel scénario (« lundi 8 h » proposé) ? | famille de trois enfants, élève, professeur, CPE, accueil, secrétariat, direction |
| (rappel) | Q-PO1 à Q-PO10 de la conception (§27.2) | |

## 8 bis. Réglages signalés (D-023)

| # | Réglage | Défaut |
|---|---|---|
| S-2912 | postes partagés déclarés | aucun ; mode discret et fin de session à la fin de la séance |
| S-2913 | ajustements préparés de la plage calme (liste) | cours du lendemain avant 10 h annulé ou déplacé, fermeture, alerte |
| S-2914 | point de rafraîchissement neutre (R-PO38) | 7 h ou ouverture de session suivante, au plus 24 h |

---

## 9. Faits, hypothèses, contradictions

**Faits [F]** : la conception lue en entier ; les renvois aux autres socles vérifiés dans leurs fichiers (recensement de la conception §6.2) ; `FRONTEND_CIBLE.md` §9 (pages de récupération qui n'agissent pas à l'ouverture). **Hypothèses [H]** : usages des personas (terrain), poids d'une appli Leptos avec WebAssembly (à mesurer, M-2933). **Contradictions** : §5 ci-dessus ; aucune ne contredit une décision validée de Florian.
