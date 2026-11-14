# Passe « personas » sur le socle 14 — Accueil, sorties et sûreté

**Date :** 2026-11-11 — **Statut :** proposition, **non décidée** (à relire par Florian) ; les recommandations s'appliquent par défaut (D-031), sauf ce qui engage l'établissement à l'extérieur (§8.2). Aucun code écrit ni modifié ; aucun accès à une base, à un secret ni à un service de production. Next.js lu seulement (module `accueil`), sans rien exécuter.
**Branche :** `ent-socle-14-conception` (même branche que la conception, partie de `origin/ent-mc0hoe`).
**Éprouve :** `docs/socles/14_ACCUEIL_SURETE.md` (§0 à §31), avec `CLAUDE.md`, `docs/DECISIONS.md` D-031, **D-034**, **D-035**, **D-036**, `docs/TRANSITION_T0_INVENTAIRE.md` (module Accueil, Q-T0-07), `04_ADMINISTRATIF.md` §8.8, `08_NOTIFICATIONS.md` §14, `34_SANTE.md`, `42_SORTIES_VOYAGES.md` §15.4 et §33. Forme reprise de `PERSONAS_42.md` et `PERSONAS_10.md`.
**Ajouts :** faits **dans** `14_ACCUEIL_SURETE.md`, au **nouveau §32** (« ajout personas ») ; rien n'est supprimé ; en cas de contradiction, le §32 l'emporte.
**Numérotation :** manques **M-1400 à M-1449** ; actions **ACS-330 à ACS-359** ; règles **R-AS21 à R-AS28** ; cas limites **CAS-87 à CAS-94** ; questions **Q-AS25 à Q-AS30** ; réglages **S-1411 à S-1414**.

---

## 0. À lire en premier : cinq bloquants et leur recommandation (appliquée par défaut, D-031)

La conception tient sur le fond (calcul serveur, socle 4 lu et non copié, minimisation, PPMS vivant, réglementaire à jour). En la faisant vivre par seize personas, cinq points bloquent l'usage réel de la rentrée 2027.

| # | Point | Pourquoi c'est bloquant | Recommandation (appliquée, D-031) | Lots |
|---|---|---|---|---|
| **A** | **Année de doublon : où vont les absences déclarées à l'accueil ?** Le 14 remplace Next.js accueil en **usage réel** (D-035), mais l'assiduité officielle reste **Charlemagne** en 2027-2028 (D-034). La conception envoie la déclaration au socle 12 de ScolIA, qui n'est pas l'outil officiel cette année-là. | Sans règle, la CPE ne voit pas dans Charlemagne l'appel de 7 h 45 reçu par l'accueil, ou le ressaisit sans savoir lequel est fait : absence non prévenue signalée à tort, ou appel perdu (P01, P03, P12). | **Mode doublon du 14** (R-AS21) : la déclaration reste saisie dans ScolIA (usage réel), **transmise au 12** ET présentée à la vie scolaire dans une file **« à reporter dans Charlemagne »** avec case « reporté » (qui, quand) ; export du jour ; la file se vide quand l'établissement bascule l'assiduité (rentrée 2028). Pas d'écriture vers Charlemagne. Régimes et remises, visiteurs, main courante, PPMS : **usage réel**, Charlemagne ne les porte pas [H, à confirmer Q-AS25]. | AS1 |
| **B** | **Le comptage en alerte suppose que chaque professeur a l'appli sur son téléphone personnel.** | Aucun texte n'oblige un personnel à utiliser son téléphone personnel [H] ; en attentat-intrusion, la consigne est le silence ; un remplaçant n'a pas l'appli ; les agents de la collectivité n'ont pas de compte (06, P25). Le « faire mieux » du socle s'effondre si 30 % des salles ne remontent rien (P09, P11). | **Trois voies de comptage équivalentes** (R-AS22) : (1) appli ; (2) **poste de la salle** (ordinateur de classe connecté à ScolIA : page de comptage en un clic) ; (3) **responsable de zone** qui pointe pour les salles de sa zone (par la porte, par téléphone interne, par papier) ; plus la liste papier du matin (S-1408). Le tableau de la cellule distingue « compté par l'adulte », « compté par le responsable de zone », « non remonté ». Jamais d'obligation d'appli personnelle. | AS10 |
| **C** | **Le contrôle de masse à 16 h 30 est infaisable par scan, et le téléphone est interdit au collège.** | 400 élèves en 10 minutes = 1,5 s par élève (P02) ; au collège, l'usage du téléphone par les élèves est interdit (C. éduc. L. 511-5) [V-mémoire, NV sur la version en vigueur] : le **QR dans l'appli** ne peut pas être la voie de base (P16). | **Contrôle par exception** (R-AS23) : à chaque sonnerie, l'AED reçoit la **liste négative** du créneau — les seuls élèves de son portail **qui ne peuvent pas sortir maintenant** (photo, raison) et ceux **attendus** (sortie exceptionnelle, remise) ; tous les autres sortent sans contrôle individuel ; le scan (carte) reste pour le doute. QR de l'appli : **lycée seulement**, désactivé par défaut au collège (S-1411). | AS5 |
| **D** | **Aucune analyse d'impact (AIPD) n'est prévue**, alors que le socle traite des mineurs, des personnes vulnérables (liste « attention particulière » du PPMS, aide à l'évacuation), un contrôle des allées et venues, des interdictions judiciaires et un registre de tiers. | Le DPO (P13) refuserait l'usage réel sans AIPD et sans base légale écrite pour le registre des visiteurs et le contrôle des sorties dans le **privé** (pas de mission de service public au sens de l'EPLE) [NV]. | **AIPD du socle 14** préparée par le pilote (modèle CNIL) avant AS4–AS6 en usage réel ; fiches de traitement ; base légale proposée (intérêt légitime de sûreté pour le privé, mission d'intérêt public pour le public) **à valider par le DPO** (§8.2). Détection d'usage anormal de la localisation (ACS-345). | AS14 (avancé avant AS4) |
| **E** | **Le parent visé par une « demande du titulaire » est humilié à chaque remise.** La règle R-AS3 exige l'accord d'un membre de la direction **à chaque remise** quand l'autre parent a seulement exprimé une demande, sans décision de justice. | Un père en résidence alternée qui vient tous les vendredis attend la direction à chaque fois, devant son fils (P05) ; la direction absente à 16 h 30 bloque la remise ; c'est une privation de fait d'un titulaire sur la seule parole de l'autre, contraire au §8.8 du socle 4. | **Examen unique** (R-AS24) : une `demande_titulaire` est examinée **une fois** par la direction sous 48 h (entretien, pièces) : **maintenue en alerte** (accord direction à chaque remise, motivé), **classée** (aucun effet à la remise), ou **transformée** (désaccord sur les sorties seules). En attendant, l'accord d'un **CPE ou d'un membre de la direction** suffit. La **résidence du jour** (socle 4) est affichée : « résidence chez le père cette semaine ». | AS4 |

**Défauts nouveaux de l'existant** : aucun au-delà de DR-01 à DR-13. **À signaler au socle 4** : la résidence du jour doit être exposée par `personnes::remise` (M-1418) ; le paramétrage `remise JSONB` (04 §8.8, ligne 683) est à normaliser (déjà au §31 du 14).

---

## 1. Méthode

**Démarche** : celle de `PERSONAS_42.md` §1 : seize personas réalistes, prises dans le tableau « acteurs » du §3 de la conception, couvrant le **privé sous contrat** (lycée pilote, OGEC) et le **public** (collège REP), les familles ordinaires et les cas particuliers, un **attaquant** (en fait sept attaques) et un **auditeur RGPD**. Pour chacune : sa journée ou son année, ce qui lui fait perdre du temps, ce qu'elle redoute, ce qui la ravirait ; manques, contradictions, cas limites.
**Grille transversale** posée à chaque persona : (1) **usage réel / doublon** — que fait-elle dans Next.js ou sur papier aujourd'hui, et en septembre 2027 ? (2) **minimisation** — que voit-elle qu'elle ne devrait pas, et l'inverse ? (3) **panne** — que se passe-t-il sans réseau ? (4) **IA** — que prépare-t-elle sans décider ?
**Gravité** : **bloquant** · **important** · **confort** · **différenciant**.
**Limites** : personas construites, pas interviewées ; pratiques de terrain [H] ; aucune donnée réelle lue.

---

## 2. Les personas en un tableau

| # | Persona | Établissement | Ce qu'elle éprouve surtout |
|---|---|---|---|
| P01 | **Fatou**, agent d'accueil et standard | Lycée pilote, privé sous contrat | Téléphone et portail à la fois, 16 h 30, relève du midi, poste partagé |
| P02 | **Yanis**, AED étudiant, portail principal | Lycée pilote | 400 élèves en dix minutes, carte oubliée, réseau faible au portail |
| P03 | **Mme Benali**, CPE | Collège public en éducation prioritaire | 600 coupons de rentrée, appels pour chaque cas, sorties anticipées, restrictions |
| P04 | **M. Roux**, chef d'établissement | Lycée pilote | PPMS unifié pour 2027-2028, exercices, alerte réelle, délégation |
| P05 | **M. Garnier**, père séparé, résidence alternée | Lycée pilote, fils en 2ᵈᵉ | « Demande » de la mère contre lui, remise du vendredi |
| P06 | **Mme Diallo**, assistante familiale (ASE) | Collège public | Enfant placé, parents avec droit de visite médiatisé, service gardien |
| P07 | **Hugo**, 18 ans, terminale ; **Sarah**, 17 ans, émancipée | Lycée pilote | Gérer ses sorties seul, ne pas informer ses parents |
| P08 | **Léo**, 6ᵉ ULIS en fauteuil, son AESH et son taxi ; **Inès**, PAI allergie | Collège public | Évacuation à l'étage, transport adapté quotidien, confinement long |
| P09 | **Mme Martin**, professeure, salle B204, alerte intrusion | Lycée pilote | Téléphone dans le sac, élèves réfugiés, remplaçante absente de l'EDT |
| P10 | **Sylvie**, secrétaire qui tient l'accueil le mercredi | Lycée pilote | Visiteurs, familles candidates, intervenants de soins, badges |
| P11 | **M. Petit**, gestionnaire et référent sûreté | Lycée pilote | Mallettes, agents de la collectivité, exercice de nuit à l'internat, commission de sécurité |
| P12 | **Florian**, administrateur, bascule de Next.js accueil | Lycée pilote | Date de bascule, doublon de l'assiduité, formation |
| P13 | **M. Blanc**, délégué à la protection des données | Organisme gestionnaire | Base légale, AIPD, durées, localisation, tablette perdue |
| P14 | **Les attaquants** | — | Faux parent, faux appel, QR prêté, AED curieux, fausse alerte, tablette volée, exfiltration du PPMS |
| P15 | **Mme Leroy**, mère d'une 6ᵉ, inquiète | Collège public | Veut savoir chaque soir que sa fille est sortie |
| P16 | **Inès et Chloé**, 4ᵉ | Collège public | Carte oubliée, téléphone interdit, cours annulé, sortie refusée |

---

## 3. Parcours et manques, persona par persona

### P01 — Fatou, accueil et standard (lycée pilote)
**Sa journée** : 7 h 30–8 h 15, quinze appels (« Léa est malade ») ; matinée, visiteurs et livreurs ; 12 h, relève par Sylvie ; 16 h 30, file de parents au guichet pendant que le téléphone sonne. Aujourd'hui elle saisit les absences dans Next.js et garde un classeur des autorisations.
**Ce qui la ravirait** : la saisie en dix secondes qu'elle a déjà, plus « qui peut chercher Léa aujourd'hui » sans classeur.
**Manques**
- **M-1400 (important)** : **relève de poste** — à 12 h, Sylvie prend la tablette : les remises préparées, les visiteurs présents, les déclarations « à confirmer » et la fiche de secours doivent rester visibles sans que Fatou reste connectée ; **changement d'utilisateur en un geste** (passkey) sur un poste partagé, sans perdre l'état du poste → ACS-330.
- **M-1401 (important)** : **numéro de l'appelant** — l'accueil voit le numéro qui appelle ; le saisir (ou le coller) propose la personne du dossier qui a ce numéro (`personnes::par_telephone`, résultat minimal : « numéro connu : mère de Léa et de Tom ») ; un numéro inconnu marque la déclaration « à confirmer » → ACS-331. Pas de couplage téléphonique automatique au départ (confort, plus tard).
- **M-1402 (important)** : **file d'attente au guichet** — plusieurs parents attendent : l'accueil prépare plusieurs remises en parallèle (onglets), l'élève est **appelé** depuis l'écran (message à la vie scolaire ou au professeur : « Léa est attendue à l'accueil »), l'heure d'appel est notée → ACS-332.
- **M-1403 (confort)** : **mode doublon** de la déclaration (bloquant A) → R-AS21.
**Contradiction** : ACS-119 imprime une liste du jour « sans interdiction », mais en panne la fiche de secours n'est que sur la tablette : si la tablette est en panne aussi, rien. → la fiche papier du jour est **proposée** (pas imposée) à la direction, sous enveloppe, détruite le soir (S-1412).

### P02 — Yanis, AED au portail (lycée pilote)
**Sa journée** : trois sonneries de sortie, pause méridienne, 16 h 30 ; réseau faible sous le porche.
**Peur** : laisser sortir un élève en retenue ; se faire insulter pour une carte oubliée.
**Manques**
- **M-1404 (bloquant C)** : **liste négative du créneau** → ACS-333, R-AS23.
- **M-1405 (important)** : **portail** comme lieu (5) : la liste négative est filtrée par portail ; un élève refusé au portail A ne sort pas par le B (la décision est la même, la liste aussi) → ACS-334.
- **M-1406 (important)** : **carte oubliée** : nom tapé + photo ; si refus récurrent (trois oublis en un mois) → information à la CPE, jamais sanction automatique.
- **M-1407 (confort)** : **précharge** de la liste négative et des décisions du jour à l'ouverture du poste, pour le réseau faible (déjà R-AS11, préciser la précharge par créneau).
**Cas limite** : CAS-87 — élève refusé qui sort en courant : « parti sans autorisation » (ACS-169) ; pas de poursuite (consigne de l'établissement).

### P03 — Mme Benali, CPE (collège public REP)
**Son année** : septembre, 600 coupons papier, un tiers non rendus ; chaque jour, vingt appels de l'accueil ; mars, le principal veut « priver de sortie » un élève comme punition.
**Manques**
- **M-1408 (important)** : **coupons papier en masse** : photo du coupon → l'IA **propose** la lecture (élève, options cochées, signataire), la CPE valide ligne par ligne (N2) ; sans IA, saisie en grille par classe → ACS-335.
- **M-1409 (important)** : **relance par classe** et **liste des non-rendus** au professeur principal (sans les options des autres) → ACS-336.
- **M-1410 (important)** : **remise obligatoire pour les 6ᵉ** en fin de journée est souvent un **choix de famille** (« mon enfant peut rentrer seul / doit être récupéré ») : ajouter une option standard `sortie_seul_fin_journee` dans le modèle collège, avec effet « remise obligatoire » si non cochée → précision de R-AS16.
- **M-1411 (important)** : **sortie anticipée au collège** : l'information des familles doit être **préalable** (carnet ou ENT) ; la conception le dit par S-1402 (« la veille ») ; ajouter : un avis envoyé **après 18 h la veille** ne vaut pas information préalable pour le lendemain matin [R] → R-AS25.
- **M-1412 (important)** : **restriction-sanction** : la conception refuse (Q-AS4) ; ajouter un **garde-fou** : une restriction posée par la direction au-delà de 15 jours ou répétée trois fois demande un motif éducatif écrit et est signalée à la CPE comme « à revoir » ; un lien vers la procédure du 13 est proposé.
**Ce qui la ravirait** : que les cas simples ne remontent plus (décision calculée, ACS-149 pour les majeurs seulement au lycée).

### P04 — M. Roux, chef d'établissement (lycée pilote)
**Son année** : PPMS unifié à adopter pour 2027-2028 ; deux exercices ; une alerte réelle peut arriver pendant une sortie au théâtre à 21 h.
**Manques**
- **M-1413 (important)** : **délégation de déclenchement** : quand il est absent, qui déclenche ? la conception dit « personnes désignées au PPMS » ; ajouter **l'ordre de suppléance** (1, 2, 3) et le **déclenchement par tout témoin** n'ouvre qu'un signal (ACS-266) — mais si **aucun membre de la direction ne confirme en 2 minutes**, le signal devient **alerte** sur la posture choisie par le témoin [R, cohérent avec « toute personne témoin… peut déclencher l'alarme »] → R-AS26, ACS-337.
- **M-1414 (important)** : **alerte hors temps scolaire** (« y compris si l'événement s'étend au-delà du temps scolaire ») : la liste des présents doit savoir qu'à 18 h il reste l'étude, l'internat, l'association sportive (43), une réunion de parents (19) → les sources du §14.2 s'étendent aux activités (43) et aux réunions (19) → ACS-338.
- **M-1415 (confort)** : **groupe scolaire** (école + collège + lycée sur un site) : un PPMS commun à plusieurs établissements ScolIA (S-1409) ; l'école relève du socle 50.
- **M-1416 (important)** : **bilan pour l'OGEC** et pour l'assureur : le bilan annuel (ACS-324) doit inclure la **conformité** (exercices faits, temps, dysfonctionnements corrigés) — déjà là ; ajouter une **preuve horodatée** (empreinte du compte rendu signé, socle 7).

### P05 — M. Garnier, père séparé (lycée pilote)
**Sa semaine** : résidence alternée ; il vient chercher son fils le vendredi à 16 h 30 pour l'emmener au train. La mère a écrit au secrétariat : « je ne veux pas qu'il vienne le chercher ».
**Manques**
- **M-1417 (bloquant E)** : examen unique de la demande → R-AS24, ACS-339.
- **M-1418 (important)** : **résidence du jour** affichée à la remise (socle 4) ; à exposer par `personnes::remise` → signalé au socle 4.
- **M-1419 (important)** : le père ne doit **jamais** savoir qu'une demande le vise (04 §8.8) : l'écran de remise ne s'affiche pas tourné vers le parent ; le libellé est neutre (« vérification habituelle »), pas « signalement » à voix haute → consigne d'écran (§24) [R].
**Cas limite** : CAS-88 — la mère vient le vendredi alors que c'est la semaine du père : remise permise (titulaire), information du père **non** automatique ; le logiciel n'arbitre pas.

### P06 — Mme Diallo, assistante familiale (ASE, collège public)
**Son quotidien** : elle vient chercher Kevin, confié à l'ASE ; les parents ont un droit de visite médiatisé ; l'adresse de Mme Diallo doit rester inconnue d'eux.
**Manques**
- **M-1420 (important)** : la remise à l'assistante familiale est **permanente** (lien « assistant familial », 4) ; les parents sont **interdits de remise** par décision (fondement `decision_ase`) — couvert ; manque : l'**avis de remise** ne doit pas partir aux parents (ils ont l'autorité mais pas la remise ni l'adresse) → règle : l'avis de remise va aux titulaires **ayant le droit `autoriser_sorties`** ; pour un enfant placé, au service gardien et à l'assistante familiale seulement (R-AS27).
- **M-1421 (important)** : **changement de lieu d'accueil** (ADM-80 du 4) : les sorties exceptionnelles futures dont la remise vise l'ancienne assistante familiale sont annulées, la CPE prévenue (étendre §6.4).
**Cas limite** : CAS-89 — le parent se présente le jour d'une visite médiatisée prévue au lycée : aucune remise ; visite organisée par le service (visiteur attendu, hôte = référent ASE).

### P07 — Hugo, 18 ans, et Sarah, 17 ans émancipée (lycée pilote)
**Manques**
- **M-1422 (important)** : **mineur émancipé** : il est capable comme un majeur pour les actes de la vie civile (C. civ. 413-6 [V-mémoire, NV]) ; pour les sorties, ScolIA le traite **comme un majeur** si l'émancipation est enregistrée (socle 2 / 4) → précision de R-AS16 et du calcul (CAS-90).
- **M-1423 (confort)** : Hugo veut que ses parents ne voient plus ses sorties : couvert (R-A24) ; ajouter l'**écran « qui voit quoi »** pour l'élève majeur (ACS-323 étendu).
- **M-1424 (important)** : **majeur protégé** (tutelle, curatelle) : rare ; le tuteur signe ; traité comme un titulaire par le socle 4 [NV] → CAS-91.

### P08 — Léo (6ᵉ ULIS, fauteuil), son AESH, son taxi ; Inès (PAI allergie) — collège public
**Manques**
- **M-1425 (important)** : **transport adapté permanent** (taxi mandaté par le département, chauffeur variable) : la remise vise un **organisme** (socle 4) avec chauffeurs **non nommés** ; pièce professionnelle exigée ; la conception ne prévoit qu'une remise ponctuelle au chauffeur (ACS-189) → **autorisation permanente à un organisme de transport** (lu au 4 ; à demander au 4 si absent) et remise quotidienne en un geste → ACS-340.
- **M-1426 (important)** : **évacuation sans ascenseur** : l'aidant désigné et l'**espace d'attente sécurisé** de l'étage (registre de sécurité : évacuation selon les types de handicap, R. 143-44 3°) : le PPMS du 14 doit nommer, pour chaque élève à aide, l'espace d'attente de chaque étage où il a cours → `ent_surete_aidant` + lien vers le lieu d'attente (lieu du 5) → ACS-341.
- **M-1427 (important)** : **confinement long** (3 h) : Inès a besoin de sa trousse ; la fiche PPMS santé de la zone (34, SAN-312) est **poussée au responsable de zone** au déclenchement (sans diagnostic : « trousse d'urgence pour 1 élève de votre zone, emplacement X ») → ACS-342.
- **M-1428 (important)** : l'**AESH** n'est pas dans l'EDT du professeur : la liste de comptage doit inclure l'AESH présente (6, 35) comme adulte de la salle.

### P09 — Mme Martin, professeure en B204 pendant une alerte intrusion
**Manques**
- **M-1429 (bloquant B)** : trois voies de comptage → R-AS22, ACS-343.
- **M-1430 (important)** : **remplaçante** : la séance modifiée par le 15 (suppléant) doit donner la liste au **suppléant**, pas au titulaire absent (lecture de l'EDT réel, 10) — préciser au §14.2 et tester (CAS-92).
- **M-1431 (important)** : **demi-groupes** et cours partagés (deux professeurs, deux salles) : la liste est par **séance et salle**, pas par classe.
- **M-1432 (important)** : **écran sombre, silence, une main** : l'écran d'alerte n'émet ni son ni vibration longue, luminosité minimale, gros boutons ; « RAS » en un geste ; aucun écran de confirmation supplémentaire pendant l'alerte (la passkey n'est demandée qu'au déclenchement) → précision §24.

### P10 — Sylvie, secrétaire à l'accueil le mercredi
**Manques**
- **M-1433 (important — minimisation)** : **intervenants de soins** (orthophoniste, kinésithérapeute qui voient un élève dans l'établissement) : le registre ne doit **jamais** lier une visite à un élève, et la qualité « soins » révélerait une santé : qualité « intervenant », hôte = **service** (vie scolaire, infirmerie), jamais l'élève → R-AS28 ; CAS-93.
- **M-1434 (important)** : **familles candidates** (portes ouvertes, rendez-vous d'inscription, socle 3) : pointées comme visiteurs ; ne créent **ni personne ni élève** ; purgées à 3 mois comme les autres visites (le 3 garde ses propres données de prospect).
- **M-1435 (confort)** : **étiquette** badge imprimable sur une petite imprimante thermique ; sinon badges numérotés.

### P11 — M. Petit, gestionnaire et référent sûreté
**Manques**
- **M-1436 (important)** : **agents sans compte** (collectivité, OGEC) : comptés par le responsable de zone, joints par SMS au numéro professionnel ou d'urgence déclaré au 6 ; la conception l'évoque (§3) mais aucune action ne l'assure → ACS-344.
- **M-1437 (important)** : **exercice de nuit à l'internat** (R 33 : exercices de nuit s'il y a des locaux de sommeil) : nature `incendie_nuit` existe ; ajouter au compteur de conformité **« exercice de nuit fait »** si l'établissement a un internat (41).
- **M-1438 (confort)** : **commission de sécurité** : export du registre (dates et temps d'évacuation de l'année, R. 143-44 5°) en un document.
- **M-1439 (important)** : **lieux modifiés** (travaux d'été) : une salle fermée (5) retire la salle des zones et le signale ; le PPMS « doit être révisé si les locaux changent » → rappel de révision automatique (ACS-262 déclenché).

### P12 — Florian, administrateur, bascule de Next.js accueil
**Manques**
- **M-1440 (bloquant A)** : mode doublon → R-AS21, ACS-346.
- **M-1441 (important)** : **date de bascule** : l'accueil saisit tous les jours ; recommandation : **bascule un lundi de rentrée de vacances** (Toussaint 2027 au plus tôt, après la rentrée et le premier exercice), Next.js accueil en lecture une semaine, puis arrêt par Florian (D-035) [R, Q-AS26].
- **M-1442 (important)** : **formation** : l'accueil change d'outil ; un **mode guidé** des premiers jours (aide contextuelle) et une fiche d'une page.

### P13 — M. Blanc, délégué à la protection des données
**Audit**
- **M-1443 (bloquant D)** : AIPD → ACS-347.
- **M-1444 (important)** : **base légale** du registre des visiteurs et du contrôle des sorties dans le privé : intérêt légitime (sûreté des personnes), avec balance des intérêts écrite ; dans le public, mission d'intérêt public ; à valider (§8.2).
- **M-1445 (important)** : **tablette d'accueil perdue ou volée** : la fiche de secours (interdictions de remise !) est la donnée la plus sensible du socle hors ligne ; exiger : chiffrement par clé d'appareil **déverrouillée par passkey** à chaque ouverture, effacement à distance au prochain contact (ACS-320), **expiration** à minuit même sans contact, et **fait de main courante** + information du DPO en cas de perte (violation potentielle, 72 h) → ACS-348.
- **M-1446 (important)** : **comptages d'exercice** : 1 mois nominatif (§21.3) : bien ; ajouter la **pseudonymisation** des comptes rendus (aucun nom d'élève dans le compte rendu signé, seulement des effectifs et des zones).
- **M-1447 (important)** : **droit d'accès d'un visiteur** et **droit d'opposition** : le registre est tenu pour la sûreté ; l'opposition ne s'applique pas tant qu'il est justifié [NV] ; procédure ACS-319 suffit.
**Satisfait** : passages ordinaires non journalisés ; aucune biométrie ; motifs scellés ; localisation motivée et journalisée.

### P14 — Les attaquants
| Attaque | Ce qui la bloque | Manque |
|---|---|---|
| **Père interdit qui appelle en se faisant passer pour la mère** (« son oncle viendra ») | rappel obligatoire sur un numéro **du dossier** (ADM-88) | si le numéro du dossier est périmé et que le père l'a récupéré : **M-1448** — le rappel affiche la **date de dernière vérification** du numéro (4) ; plus de 12 mois → confirmation par un second canal (message dans l'appli du titulaire) |
| **Élève qui imite un parent** pour déclarer son absence | numéro inconnu → « à confirmer » ; déclaration téléphonique **n'est pas un justificatif** (le 12 l'attend) | — |
| **QR prêté ou capturé** | QR dynamique 30 s lié à l'appareil ; photo à l'écran du contrôleur | — |
| **AED curieux** qui localise une élève à répétition | motif, journal | **M-1449** — détection : plus de 5 localisations du même élève par la même personne en 7 jours sans remise ni recherche → alerte à la direction (ACS-345) |
| **Fausse alerte** par un personnel | le signal (ACS-266) va à la direction, qui confirme ; l'escalade à 2 min (R-AS26) n'envoie qu'aux **personnels**, pas aux familles | les familles ne reçoivent l'alerte que sur déclenchement par une personne habilitée (R-AS26) |
| **Tablette volée** sur le comptoir | verrouillage à 60 s, passkey, expiration à minuit, effacement à distance (M-1445) | — |
| **Exfiltration du PPMS** (plans, codes, annuaire) | R-AS10, droits restreints, export journalisé ; filigrane nominatif sur les PDF exportés [R] | ajouté à ACS-252 et ACS-258 |
| **Compte parent compromis** qui inonde de demandes de sortie | passkey (D-009), limite de débit (5 demandes par jour et par élève), demande décidée par la CPE | — |

### P15 — Mme Leroy, mère d'une 6ᵉ, inquiète (collège public)
**Ce qu'elle veut** : « un message chaque soir quand ma fille sort ».
**Conflit** : R-AS8 (on ne journalise pas les sorties ordinaires). **Recommandation** : pas de traçage quotidien ; à la place, (a) l'**avis de fin de journée prévue** (« demain, Léa finit à 15 h, elle peut sortir à 15 h ») chaque fois qu'elle change, déjà porté par `edt.sortie_anticipee` ; (b) l'avis immédiat de toute **exception** (sortie exceptionnelle, remise, départ sans autorisation) ; (c) au collège, l'option « doit être récupérée » (M-1410) si elle le souhaite. Pas d'option « suivi de chaque sortie » [R, Q-AS27].

### P16 — Inès et Chloé, 4ᵉ (collège public)
**Manques**
- Téléphone interdit au collège → QR désactivé (bloquant C, S-1411).
- **Cours annulé à 14 h** : Chloé a l'option « fin de journée », Inès non ; Chloé voit « tu peux sortir à 15 h » seulement si la règle du collège le permet (fin de demi-journée) ; Inès voit « permanence jusqu'à 16 h » — **explication lisible pour un enfant de 13 ans** (vocabulaire simple, sans « option `sur_option_famille` ») → consigne de rédaction (§24).
- **CAS-94** — Chloé sort avec Inès en disant « elle a le droit » : la liste négative du portail montre Inès (photo) : refusée.

---

## 4. Vérification réglementaire complémentaire (2026-11-11)

| Point | Statut | Effet |
|---|---|---|
| Circulaire PPMS du 8 juin 2023 (BO n° 26) | [V] (sous-agent de la conception) | exercices annoncés, deux par an, REX au 1er avril, communication au 1er juillet ; repris |
| Téléphone des élèves au collège (C. éduc. L. 511-5) | **[NV]** : de mémoire, interdiction de l'usage du téléphone dans les écoles et collèges, sauf usages pédagogiques prévus par le règlement intérieur ; version et extension « portable en pause » à vérifier | QR désactivé par défaut au collège |
| Mineur émancipé (C. civ. 413-6) | **[NV]** de mémoire | traité comme majeur pour les sorties si l'émancipation est enregistrée |
| Registre de sécurité : évacuation selon les types de handicap (CCH R. 143-44 3°) | [V] (conception §5.2) | espaces d'attente sécurisés dans le PPMS |
| Violation de données (RGPD art. 33, 72 h) | [V-mémoire] | tablette perdue = incident à qualifier par le DPO |

---

## 5. Manques qui relèvent d'autres socles

| Manque | Socle | Demande |
|---|---|---|
| Résidence du jour exposée par `personnes::remise` | 4 | M-1418 |
| Autorisation permanente de remise à un **organisme de transport** (chauffeurs non nommés) | 4 | M-1425 |
| Date de dernière vérification d'un numéro de téléphone | 4 | M-1448 |
| `personnes::par_telephone(numero)` minimal | 4 | M-1401 |
| Mineur émancipé, majeur protégé | 2, 4 | M-1422, M-1424 |
| Activités et réunions en cours (présents hors temps scolaire) | 43, 19 | M-1414 |
| AESH présente dans la salle (adulte de la séance) | 35, 6, 10 | M-1428 |
| Normaliser `remise JSONB` du paramétrage | 4 | §31 du 14 |
| `ent_metier_event` hors RLS, motif en clair | 1 | DR-06 |

---

## 6. Synthèse

### 6.1 Manques par gravité
- **Bloquants (5)** : A (doublon, M-1403/M-1440), B (comptage sans téléphone personnel, M-1429), C (contrôle de masse, M-1404), D (AIPD, M-1443), E (examen unique, M-1417).
- **Importants (39)** : M-1400 à M-1402, M-1405, M-1406, M-1408 à M-1414, M-1416, M-1418 à M-1422, M-1424 à M-1428, M-1430 à M-1434, M-1436, M-1437, M-1439, M-1441, M-1442, M-1444 à M-1449.
- **Confort (5)** : M-1407, M-1415, M-1423, M-1435, M-1438.

### 6.2 Les dix ajouts les plus importants (versés au §32 de la conception)
1. Mode doublon : file « à reporter dans Charlemagne » (R-AS21, ACS-346).
2. Trois voies de comptage, aucune obligation d'appli personnelle (R-AS22, ACS-343).
3. Contrôle par exception : liste négative du créneau (R-AS23, ACS-333).
4. AIPD et bases légales avant l'usage réel (ACS-347).
5. Examen unique d'une demande de titulaire (R-AS24, ACS-339).
6. Relève de poste et changement d'utilisateur (ACS-330).
7. Transport adapté permanent (ACS-340).
8. Espaces d'attente et trousses poussées au responsable de zone (ACS-341, ACS-342).
9. Escalade d'un signal de témoin sans réponse de la direction (R-AS26, ACS-337).
10. Tablette perdue : passkey, expiration, effacement, incident (ACS-348).

### 6.3 Ce que la passe a montré
La conception pensait **l'outil** ; les personas ont montré **le débit** (16 h 30, 400 élèves), **le silence** (une alerte se compte sans bruit et sans téléphone personnel), **l'année de doublon** (usage réel du 14, assiduité officielle ailleurs) et **la dignité** (le père contrôlé chaque vendredi). Aucun principe n'est remis en cause ; quatre règles sont précisées.

---

## 7. Décisions

### 7.1 Appliquées par défaut (D-031)
| N° | Décision |
|---|---|
| P-1 | Mode doublon du 14 (bloquant A) : déclarations dans ScolIA + file « à reporter dans Charlemagne » |
| P-2 | Comptage à trois voies, pas d'obligation d'appli personnelle |
| P-3 | Contrôle par liste négative ; QR élève au lycée seulement par défaut |
| P-4 | AIPD préparée par le pilote avant AS4–AS6 en usage réel |
| P-5 | Examen unique d'une demande de titulaire sous 48 h |
| P-6 | Escalade d'un signal de témoin à 2 min vers une alerte **aux personnels** seulement |
| P-7 | Pas de suivi quotidien des sorties ordinaires pour les familles (Q-AS27) |
| P-8 | Avis de remise d'un enfant placé : service gardien et assistant familial seulement |
| P-9 | Mineur émancipé traité comme majeur pour les sorties (sous réserve [NV]) |
| P-10 | Bascule de Next.js accueil au plus tôt à la rentrée de Toussaint 2027 (Q-AS26, date finale de Florian) |

### 7.2 Soumis à Florian
| N° | Question |
|---|---|
| Q-AS25 | Charlemagne porte-t-il aujourd'hui les **régimes de sortie** du lycée pilote ? (si oui, le 14 est en doublon pour eux aussi) |
| Q-AS26 | Date d'arrêt de Next.js accueil |
| Q-AS28 | **DPO** : bases légales, AIPD, durées (rejoint Q-AS22) |
| Q-AS29 | Les salles ont-elles un **poste fixe** connecté (voie 2 du comptage) ? |
| Q-AS30 | Le lycée a-t-il des **élèves en transport adapté** et un internat (exercice de nuit) ? |

## 8 bis. Réglages ajoutés
| N° | Réglage | Défaut |
|---|---|---|
| S-1411 | QR de l'appli élève pour le contrôle de sortie | lycée : oui ; collège : non |
| S-1412 | Fiche papier de secours du jour proposée à la direction | non |
| S-1413 | Délai de confirmation d'un signal de témoin avant escalade aux personnels | 2 min |
| S-1414 | Seuil de détection de localisations répétées | 5 en 7 jours |

## 9. Faits, hypothèses, contradictions
- **Faits** : ceux de la conception (§2, §5) ; aucun fait de code nouveau.
- **Hypothèses** : pratiques des personas ; Charlemagne ne porte ni remises ni visiteurs ni PPMS [H, Q-AS25] ; postes fixes en salle [H, Q-AS29].
- **Contradictions levées** : R-AS3 (accord direction à chaque remise) **remplacée** par R-AS24 ; ACS-266 précisée par R-AS26 ; ACS-167 (QR) limitée par S-1411 ; ACS-119 (liste papier) complétée par S-1412.
