# Passe « personas » sur le socle 9 — Interopérabilité nationale et identité fédérée

**Date :** 2026-11-08 — **Statut :** proposition, **non décidée** (à relire par Florian). Aucun code écrit ni modifié ; aucun accès à une base, à un secret ni à un service de production ; l'application Next.js n'a pas été ouverte.
**Branche :** `ent-personas-socle-9` (base `ent-mc0hoe` `f900681`, qui porte `09_INTEROP.md` relu, §30).
**Éprouve :** `docs/socles/09_INTEROP.md` (lu en entier), avec `CLAUDE.md`, `docs/PILOTAGE.md`, `docs/DECISIONS.md` (D-009, D-011 à D-013, D-019, D-021 à D-028), `docs/MATRICE_CHARLEMAGNE.md` (§0, §1, lignes ELE-01, ELE-05, ELE-13, ELE-22, INS-16, EDT-04, NOT-09, NOT-10, NOT-16, ORI-02, PER-18, CPT-07, PIL-09, PIL-20, G6, G7, G9, G10, T-26, T-28), `docs/socles/PERSONAS_08.md` (méthode et forme reprises).
**Ajouts :** faits **directement** dans `09_INTEROP.md`, marqués **[ajout personas]** : statut ; §0 point 11 ; règles `R-X17` à `R-X28` (§6.12) ; flux `FX-09`, `FX-41` à `FX-43`, `FX-56`, `FX-66`, `FX-67`, `FX-75` à `FX-78`, `FX-88`, `FX-97` à `FX-99`, `FX-106` à `FX-108` (§9.12, nouvelle sous-partie) ; note sur le rattachement de rentrée (§10.6) ; cas limites `CX-47` à `CX-65` (§12) ; actions `ITP-119` à `ITP-154` (§13.13) ; réglages signalés `S-68` à `S-79` (§13, couverture) ; droits (§18.3) ; lots (§25.4) ; démarches `D-17` à `D-20` (§26) ; questions `Q-X33` à `Q-X44` (§27) ; contradictions `C10` à `C17` (§28). **Rien n'est supprimé** ; quand un ajout contredit le texte d'origine, une note [ajout personas] est posée à côté.
**Numérotation :** les manques continuent `PERSONAS_08.md` (qui s'arrête à M-404) : **M-405 à M-496**. Les réglages signalés continuent aussi : **S-68 à S-79**. Identifiants vérifiés libres avant écriture (`grep` sur `docs/`).

---

## 0. À lire en premier : ce qu'il faut trancher avant le premier lot de code

Aucun lot du socle 9 n'est commencé. Le moteur est juste (propositions, mémoire de la source, garde-fous, exports figés, rattachement par preuve forte). En le faisant vivre par une secrétaire de lycée privé qui quitte Charlemagne, une principale adjointe de collège public, un comptable d'OGEC, une DSDEN et un informaticien, **quatre points** apparaissent qui changent l'ordre des lots ou la promesse commerciale. Ils ne relèvent pas d'un détail d'écran : ce sont des **décisions de stratégie** à prendre avec Florian.

| # | Point | Pourquoi | Ce qu'il faut décider | Gravité | Lots |
|---|---|---|---|---|---|
| A | **La première année d'un lycée privé qui quitte Charlemagne n'a pas de chemin.** La conception prévoit la cohabitation pour la seule remontée BEE (Q-X14). Or LSU, LSL et Parcoursup passent **aussi** par un logiciel validé ; et Parcoursup exige les **bulletins de première** (année N-1) des élèves de terminale, que ScolIA n'a pas. Sans décision, l'établissement saisit ses notes deux fois (ScolIA et Charlemagne) ou ressaisit à la main dans les applications nationales. Rien ne dit non plus quel logiciel « fait foi » pendant la cohabitation (§9.10 dit « ScolIA », Q-X14 dit « le logiciel agréé remonte ») (P01, P02, P09) | **Q-X34** : stratégie de bascule par module pour l'an 1 (ce qui passe sur ScolIA, ce qui reste sur le logiciel agréé jusqu'aux validations) ; profil `prive_contrat_cohabitation` ; **journal « à reporter dans le logiciel agréé »** (FX-107, R-X24) ; **reprise structurée des évaluations de N-1** (FX-106) obligatoire pour un lycée | **bloquant** | X3, X7, X7 bis, socle 28 |
| B | **La rentrée ÉduConnect d'un collège public demande 1 300 validations à la main.** La relecture (obj. 2, §10.2) exige un second élément (code, ou validation du secrétariat) même quand une jointure officielle existe ; R-X11 (a) dit pourtant que la jointure est une preuve forte. À la rentrée, la liste « rattachements à confirmer » sera validée en masse sans être lue, ce qui annule la protection, y compris pour un parent dont le lien vient d'être suspendu (P03, P08) | **Q-X35** : confirmation **automatique** quand la jointure vient de la chaîne officielle complète (import AAF ou BEE confirmé de cet établissement + attribut signé du fournisseur) **et** que le lien du socle 4 est actif sans mesure ; validation humaine **seulement** pour les discordances ; **R-X19** : un rattachement révoqué pour un motif de protection bloque toute nouvelle jointure de la personne jusqu'à levée par la direction | **bloquant** | X8 |
| C | **Dans le public, ScolIA ne sera souvent pas « l'ENT ».** La conception suppose que ScolIA est le projet ENT (`idEnt`, médiacentre, exigences n° 6 et 15). Dans beaucoup de collèges et lycées publics, l'ENT est choisi par le département ou la région, et le logiciel de vie scolaire s'y raccorde par SSO [H, pratique courante, à confirmer par D-17]. ScolIA doit alors **accepter la connexion venue de l'ENT de la collectivité** (CAS, SAML ou OIDC, FX-99) et laisser le GAR au projet ENT ; l'accès à l'AAF d'un service de vie scolaire qui n'est pas l'ENT est **NV** (P03, P12) | **Q-X33** : deux positionnements à porter (ScolIA = ENT ; ScolIA = service de vie scolaire raccordé à l'ENT de la collectivité) ; démarche **D-17** ; lot **X8 bis** (SSO entrant depuis un ENT) avant X11–X12 pour le public | **bloquant** (pour la vente au public) | X8 bis, X11, X12, X13 |
| D | **Les prélèvements de septembre tombent si les mandats SEPA ne sont pas repris.** Un mandat repris sans sa référence unique (RUM), sa date de signature et sa séquence impose de faire re-signer 600 familles, ou fait rejeter la première remise [H, règles SEPA de mémoire, à vérifier auprès de la banque, D-20] (P07) | reprise des mandats (FX-76) et des soldes d'ouverture **dans le parcours de mise en service**, avant le premier pain.008 ; aucun changement de RUM sans procédure d'information des débiteurs | **bloquant** (privé) | X18, socles 25, 28 |

Suivent, **importants**, à intégrer aux premiers lots plutôt qu'après : garde-fou de volume qui se déclenche à **chaque** bascule de juillet (R-X20, X2c) ; dépôt manuel **déclaré** et export **périmé** signalé (R-X21, X7) ; attestation de remontée et journal scellé (R-X22, X7) ; révocation générale d'une personne en un geste et coupe-circuit d'un fournisseur (R-X25, X8) ; aperçu « ce qui part pour la première fois » avant tout envoi à un tiers (R-X17, X7, X17).

---

## 1. Méthode

**Démarche.** Celle de `PERSONAS_08.md` §1 : 16 personas réalistes, prises dans la consigne (secrétariat de lycée privé sous Charlemagne, direction, chef de travaux, CPE, professeur, comptable, famille, élève majeur, informaticien, DSDEN et rectorat, éditeur partenaire) et complétées pour couvrir le collège public sous ENT de collectivité, l'école privée (ONDE), le CDI et la protection des données. Pour chacune : **son année des échanges** (§4 de la conception : préparation de rentrée, bascule de juillet, rentrée, constat, campagnes nationales, clôtures), puis ses **incidents** (rejet incompréhensible, plateforme nationale en panne la veille, fichier de l'an dernier, doublons, reprise qui ressuscite des données, compte compromis, partenaire révoqué). Chaque fois qu'elle voulait faire quelque chose que la conception ne prévoit pas, ou que c'était trop long, ambigu, risqué ou frustrant, le manque est noté.

**Grille transversale** posée à chaque persona : d'où vient la donnée et qui fait foi ? ; que se passe-t-il si le fichier est faux, vieux, partiel, d'un autre établissement ? ; que se passe-t-il **pendant** la première année (bascule depuis Charlemagne) ? ; qui sait ce qui est parti, où, quand, et peut le prouver ? ; que voit la personne dont on parle ? ; que révèle un flux sortant (minimisation, personnes protégées) ? ; comment on révoque ? ; que se passe-t-il la veille d'une échéance quand rien ne marche ? ; « reviendrait-elle à son tableur ? ».

**Couverture Charlemagne** (D-023, D-024) : les lignes de la matrice qui touchent le socle 9 et les fonctions d'échange qu'un utilisateur de Charlemagne attend [H : la documentation de l'éditeur n'a pas pu être lue, même limite que la matrice §0] ont été relues une à une (§4).

**Gravité** (identique aux passes précédentes).
- **bloquant** : sans lui, l'établissement contourne de façon dangereuse (sécurité d'un enfant, fuite, pratique probablement illégale) ou ne peut pas traiter un cas réel ; ou la conception se contredit au point d'empêcher de coder.
- **important** : cas fréquent sans solution propre, perte de temps forte ou risque d'erreur ; ou fonction de Charlemagne absente.
- **confort** : améliore nettement le quotidien ; un contournement existe.
- **différenciant** : peu de logiciels du marché le font [H, comparaison de mémoire] ; argument de vente.

**Conventions.** Celles de `09_INTEROP.md` : [F] fait vérifié dans la conception ; [H] hypothèse ; **[NV]** règle, format ou procédure cités de mémoire, **interdit de coder dessus** avant vérification sur le texte officiel ; [R] recommandation. Numéros de socles : `SOCLES.md`.

**Limites.** Les personas sont construites, pas interviewées ; la vie du secrétariat de lycée privé s'appuie sur l'expérience de Florian rapportée par `D-023` et les passes précédentes. Les règles citées (effacement des sanctions, règles SEPA de reprise des mandats, enquête SIFA, plateforme des stages de seconde, heures supplémentaires des maîtres du privé, facturation électronique, FranceConnect pour le privé, raccordement d'un service de vie scolaire à un ENT de collectivité) le sont **de mémoire** : marquées [NV] ou [H]. Aucune page officielle n'a été consultée (le réseau de la session les refuse, `09_INTEROP.md` en-tête) ; **aucun sous-agent n'a été utilisé** : la passe ne demandait pas de recherche au-delà des documents lus.

---

## 2. Les personas en un tableau

| # | Persona | Établissement | Ce qu'elle éprouve surtout |
|---|---|---|---|
| P01 | **Sylvie**, secrétaire de scolarité (Charlemagne depuis douze ans) | Ensemble privé sous contrat, collège + lycée, deux UAI, 1 100 élèves | Bascule depuis Charlemagne, cohabitation BEE, constat de rentrée, rejets, doublons |
| P02 | **M. Arnaud**, chef d'établissement coordinateur | même ensemble | Ce qui est parti où, preuve, sincérité des effectifs, première année |
| P03 | **Mme Benali**, principale adjointe | Collège public, ENT du département, 650 élèves | ScolIA derrière l'ENT de la collectivité, ÉduConnect, bascule de juillet, Affelnet |
| P04 | **M. Costa**, chef de travaux (DDFPT) | Lycée professionnel privé avec unité de formation d'apprentis | PFMP, examens, apprentis, entreprises |
| P05 | **Karim**, CPE | Collège public de P03 | Restauration et badges, familles non activées, nouveaux responsables |
| P06 | **M. Petit**, professeur de mathématiques, responsable de l'emploi du temps (décharge) | Ensemble de P01 | Échanges avec le logiciel d'EDT, groupes, ses notes dans un tableur |
| P07 | **Mme Royer**, comptable de l'organisme de gestion (OGEC) | Ensemble de P01 | Mandats SEPA, relevés, soldes, expert-comptable, facturation électronique |
| P08 | **Famille Martin** (un enfant au collège public, un au lycée privé) ; **M. Dufour**, père sous interdiction de contact | Collèges de P03 et P01 | Rattachement, rectification, « à qui mes données ? », protection |
| P09 | **Hugo**, 18 ans, terminale | Lycée de P01 | Parcoursup, ses propres données, son nom d'usage |
| P10 | **Yann**, informaticien de l'ensemble | Ensemble de P01 | Google Workspace déjà en place, annuaire local, certificats, révocation |
| P11 | **Mme Garnier**, division de l'enseignement privé de la DSDEN | DSDEN | Constat de rentrée, heures des maîtres, listes ponctuelles |
| P12 | **M. Laurent**, DSI académique | Rectorat | Raccordements, convention AAF, incidents, qualification |
| P13 | **RestoBadge**, éditeur de restauration scolaire ; **un éditeur de logiciel de CDI** | Partenaires | API, bac à sable, consentement, purge |
| P14 | **Mme Fontaine**, DPO du réseau d'établissements | Réseau diocésain | Registre, minimisation, élèves protégés, copies locales |
| P15 | **Mme Perret**, directrice d'école privée | École de l'ensemble de P01 | ONDE bloqué, liaison école-collège |
| P16 | **Lucie**, professeure documentaliste | Lycée de P01 | GAR, médiacentre vide en septembre, logiciel du CDI, Pix |

---

## 3. Parcours et manques, persona par persona

Chaque tableau donne : le manque, pourquoi il compte, sa gravité, le socle concerné, et l'ajout fait (identifiant dans `09_INTEROP.md`) ou le socle où il est renvoyé.

### P01 — Sylvie, secrétaire de scolarité (ensemble privé, sous Charlemagne)

**Année.** *Mars–juin* : la direction signe ScolIA pour la rentrée ; Sylvie doit « tout reprendre » de Charlemagne : élèves, responsables, fratries, historique, factures. *Juillet* : réinscriptions, affectés, départs ; elle remonte encore les mouvements dans BEE par Charlemagne (agréé). *Septembre* : constat de rentrée, effectifs signés ; les chiffres de ScolIA et de BEE doivent coïncider. *Octobre* : trois élèves arrivés sont refusés par BEE « élève déjà scolarisé » ; un rejet porte un code qu'elle ne comprend pas. *Toute l'année* : les mêmes listes pour l'UGSEL, l'assurance, le photographe.
**Incidents.** Elle saisit un départ dans ScolIA et oublie de le reporter dans Charlemagne : BEE garde l'élève, l'établissement d'accueil ne peut pas l'inscrire. Un parent apparaît deux fois (« Jean-Marc Dupont » et « Jean Marc DUPONT ») car Charlemagne avait une fiche par enfant. Un élève parti l'an dernier revient : la reprise en crée un second.

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-405 | **Journal « à reporter dans le logiciel agréé »** pendant la cohabitation : chaque mouvement saisi dans ScolIA qui doit partir vers BEE (arrivée, départ, changement de division, d'option, d'identité) crée une ligne à pointer | §0 point A : sans lui, la cohabitation de Q-X14 est une double saisie sans filet ; les oublis bloquent d'autres établissements | **bloquant** | 9 | FX-107, R-X24, ITP-119 |
| M-406 | **Profil `prive_contrat_cohabitation`** : ScolIA source de saisie, logiciel agréé source de remontée, SIECLE source de retour | §9.10 (« ScolIA fait foi ») et Q-X14 (« le logiciel agréé remonte ») se contredisent pour la première année ; la mémoire de la source ne sait pas quelle famille de source est la référence | **bloquant** | 9, 2, 4 | R-X24, C11 |
| M-407 | **Concordance ScolIA ↔ BEE à une date** avant le constat de rentrée (inscrits ici absents de BEE, présents dans BEE et sortis ici, divisions différentes) | Le constat fonde le forfait d'externat et les moyens [S, `09_INTEROP.md` §4] ; une discordance découverte par la DSDEN coûte cher | important | 9 | ITP-120 |
| M-408 | **« Élève encore inscrit ailleurs »** : rejet BEE traduit en tâche avec l'UAI, le nom et les coordonnées **publiques** de l'établissement d'origine (annuaire des UAI), et un modèle de message | Cas classique de rentrée [H] ; aujourd'hui Sylvie cherche le numéro de l'autre établissement et appelle | important | 9 | CX-47, ITP-121 |
| M-409 | **Rappel « sortie à remonter »** pour nos propres départs, tant qu'elle n'est pas remontée (ou pointée dans le journal de cohabitation) | C'est notre oubli qui bloque l'établissement d'accueil ; D-013 interdit de le prévenir directement, mais rien n'empêche de ne pas oublier | important | 9 | ITP-119, R-X24 |
| M-410 | **Assistant de reprise** avec un **catalogue « repris / archivé en PDF / abandonné »** par type de donnée, validé par la direction (★) avant la reprise | FX-102 dit « formats exacts NV » et renvoie au socle 28 ; sans catalogue signé, on reprend tout ou rien, et personne ne sait ce qui a été laissé | important | 9, 28 | FX-102 (note), ITP-122, R-X23 |
| M-411 | **La reprise n'importe pas ce qui aurait dû disparaître** : sanctions dont le délai d'effacement est passé [V : R511-13 IV — avertissement à la fin de l'année, blâme et responsabilisation à la fin de l'année suivante, autres sauf exclusion définitive à la fin de la deuxième année suivante, toutes au terme de la scolarité dans le second degré ; VERIF R34], données de santé hors PAI en cours, coordonnées d'anciens responsables | Une reprise « complète » ressuscite des données que la loi ou le règlement voulait effacées ; c'est une faute du nouvel outil, pas de l'ancien | important | 9, 13, 34 | R-X23, CX-60 |
| M-412 | **Rejet national traduit** : catalogue des codes de rejet connus tenu par ScolIA (cause probable, écran de correction), et nature du rejet (données / format / plateforme nationale) | Le rejet « incompréhensible » est la première plainte (§1 de la conception) ; l'IA (`expliquer_rejet`) aide, mais un catalogue vérifié est plus sûr et se capitalise entre établissements | important | 9 | ITP-123 |
| M-413 | **Dépôt manuel déclaré** : quand le fichier est déposé à la main sur le site national, la secrétaire déclare le dépôt (date, compte rendu joint) ; relance si un export généré n'est jamais déclaré déposé avant l'échéance | Le cycle §8.2 (`depose`, `accuse`) suppose un canal automatique ; pour un dépôt manuel, ScolIA ne sait rien et l'état ment | important | 9 | R-X21, ITP-124 |
| M-414 | **Export périmé** : si les données ont changé depuis l'instantané (note corrigée, élève arrivé), avertissement avant la déclaration de dépôt, avec l'écart | Le fichier généré lundi est déposé mercredi après une correction de mardi : la remontée est fausse et personne ne le voit | important | 9 | R-X21, CX-59 |
| M-415 | **Effectifs à une date passée** (« au jour du constat »), figés | Le constat est arrêté à une date ; la saisie continue ensuite | important | 9, 2 | ITP-125 |
| M-416 | **Même responsable saisi deux fois** dans l'ancien logiciel (une fiche par enfant) : proposition de fusion groupée « même personne probable » au socle 4, jamais automatique | Défaut connu des logiciels anciens [H] ; sans traitement, deux comptes, deux factures, deux avis | important | 4, 9 | CX-48 |
| M-417 | **Élève qui revient** (sorti l'an dernier) : les candidats de rapprochement incluent les élèves sortis ; on rouvre une scolarisation, on ne crée pas un second élève | Le port `candidats` n'est pas dit inclure les sortis | important | 2, 9 | CX-49 |
| M-418 | **Jumeaux** : même date de naissance, mêmes responsables ; un prénom différent interdit la proposition « même élève » | Un rapprochement par identité trop large fusionnerait deux enfants | important | 2, 9 | CX-50 |
| M-419 | **Photos du photographe scolaire** : lot de fichiers + table de correspondance, en propositions au socle 2 | Charlemagne et la matrice (ELE-05) importent les photos en masse ; la conception n'a pas de connecteur | confort | 2, 9 | FX-108, ITP-148 |
| M-420 | **Licenciables UGSEL / UNSS** et **liste pour l'assurance scolaire collective** : exports de travail tracés, champs minimaux | Demandés chaque année [H] ; aujourd'hui extractions Excel envoyées par courriel | confort | 9 | FX-66, FX-67, ITP-150 |

### P02 — M. Arnaud, chef d'établissement coordinateur (ensemble privé)

**Année.** *Avril* : il signe la bascule ; il veut savoir ce que la première année coûtera à Sylvie. *Septembre* : il signe le constat ; la DSDEN contrôle parfois les effectifs. *Juin* : la veille de la fermeture de LSU, la plateforme nationale ne répond pas. *Toute l'année* : un parent demande ce qui a été transmis sur son fils ; le diocèse demande des chiffres.
**Incidents.** Un élève inscrit en juin n'est jamais venu, il est compté au constat. La passation collège → lycée de l'ensemble perd l'historique parce que les deux UAI ont été ouvertes comme deux établissements ScolIA.

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-421 | **Stratégie de bascule de l'année 1** : LSU, LSL et Parcoursup dépendent aussi d'un logiciel validé ; sans décision, double saisie des notes | §0 point A | **bloquant** | 9, 22, 23, 37, 28 | Q-X34 |
| M-422 | **Attestation de remontée** : PDF signé (socle 7) avec empreinte du fichier, nombre de personnes, destinataire, date, accusé ; **journal des remontées scellé** (chaîne d'empreintes horodatées) | En cas de contestation (DSDEN, famille), la preuve doit être lisible par un non-informaticien et inaltérable | important | 9, 7 | R-X22, ITP-126 |
| M-423 | **Contrôle de sincérité avant constat** : inscrits sans aucune présence relevée depuis la rentrée, départs annoncés non saisis, élèves en double UAI | Un effectif faux engage le chef d'établissement ; personne ne fait ce contrôle à la main sur 1 100 élèves | différenciant | 9, 12 | ITP-127 |
| M-424 | **Échéance en danger** : la plateforme nationale est indisponible ; preuve horodatée des tentatives, modèle de message à la DSDEN, alerte à la direction | Le jour de fermeture, la seule chose utile est la preuve qu'on a essayé à temps | important | 9 | CX-51, ITP-128 |
| M-425 | **Ensemble scolaire = un établissement ScolIA à plusieurs UAI** (recommandation) ; sinon D-013 interdit toute continuité collège → lycée | La conception traite deux UAI sous un tenant (CX-35) mais ne dit pas que c'est la règle ; une ouverture en deux tenants est irréversible en pratique | important | 1, 5, 9 | Q-X37 |
| M-426 | **Coût annoncé du premier import** : estimation des écarts à arbitrer et du temps, mode « classe par classe » | « 214 écarts » est dit après coup ; la direction planifie la rentrée avant | confort | 9 | ITP-04 (note), §25.4 |
| M-427 | **Délégation de la signature des remontées** à l'adjoint désigné pendant une absence, journalisée | Les échéances ne attendent pas le retour du chef d'établissement | confort | 1, 6 | renvoi (socle 1, délégations) |

### P03 — Mme Benali, principale adjointe (collège public, ENT du département)

**Année.** *Juin* : le département impose son ENT ; le collège choisit sa vie scolaire. *Juillet* : bascule de SIECLE vers N+1 ; 160 élèves de 3ᵉ disparaissent du fichier. *Août* : Affelnet, élèves affectés arrivés dans SIECLE avant leur dossier. *Septembre* : ÉduConnect, 1 300 responsables.
**Incidents.** Le garde-fou de volume bloque l'import de juillet « 25 % de disparitions » ; la secrétaire le lève sans lire. Le département demande que la connexion passe par son ENT.

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-428 | **ScolIA derrière l'ENT de la collectivité** : connexion entrante depuis l'ENT (CAS, SAML ou OIDC), GAR et médiacentre laissés à l'ENT, exigences n° 6 et 15 portées par le projet ENT [H] | §0 point C ; la conception ne prévoit que « ScolIA = ENT » | **bloquant** | 1, 9 | FX-99, Q-X33, X8 bis |
| M-429 | **Accès à l'AAF d'un service de vie scolaire qui n'est pas l'ENT** [NV] | Si seul l'ENT reçoit l'AAF, ScolIA dépend des exports déposés (FX-02) ou d'un relais par l'ENT | important | 9 | D-17, Q-X33 |
| M-430 | **Garde-fou de volume conscient de la bascule** : départs attendus (dernier niveau, départs déclarés, non-réinscrits du socle 3) exclus du calcul | Sinon le garde-fou se déclenche **chaque** juillet et devient un clic réflexe qui ne protège plus rien | important | 9 | R-X20, CX-52 |
| M-431 | **Rattachement de rentrée sans 1 300 validations** | §0 point B | **bloquant** | 1, 9 | Q-X35, §10.6 (note) |
| M-432 | **Élève affecté présent dans SIECLE et dossier `affecte` du socle 3** : rapprochement avec le dossier, pas un nouvel élève | Le port de correspondance du socle 3 n'est pas listé (§5.4 (a) ne cite que 2, 4, 5, 6) | important | 3, 9 | CX-53 |
| M-433 | **« Corrigé dans SIECLE, en attente du prochain import »** : état d'un écart persistant pointé par la secrétaire | Sinon la liste des écarts ne descend jamais entre deux imports et on recorrige deux fois | confort | 9 | ITP-151 |
| M-434 | **Élèves de N+1 encore sans division** dans SIECLE en juillet : propositions sans division, jamais rejet | Une division n'existe qu'à la fin d'août [H] | confort | 5, 9 | CX-52 (note) |

### P04 — M. Costa, chef de travaux (lycée professionnel privé avec apprentis)

**Année.** *Septembre* : conventions de PFMP, entreprises ; apprentis en mixité de parcours. *Décembre* : enquête sur les apprentis. *Printemps* : inscriptions aux examens, durées de PFMP à justifier. *Juin* : stages de seconde.

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-435 | **Durées de PFMP et positionnements** (réduction de durée) au pré-contrôle des inscriptions aux examens [NV] | Un candidat sans durée suffisante justifiée n'est pas présentable [NV] ; découvert trop tard | important | 38, 9 | FX-38 (note) |
| M-436 | **Livret scolaire du lycée professionnel** et contrôle en cours de formation [NV] | FX-34 ne parle que de la voie générale et technologique | important | 23, 9 | FX-34 (note) |
| M-437 | **Enquête annuelle sur les apprentis** (SIFA [H, NV]) | Obligation des unités de formation d'apprentis [H] ; Charlemagne la sert-il ? [H] | important | 9, 44 | FX-42 |
| M-438 | **Référentiel des entreprises** (base Sirene téléchargée, jamais interrogée en ligne avec une donnée d'élève) | Conventions de stage : raison sociale, adresse, SIRET saisis à la main ; un entrepreneur individuel est une personne : minimisation | confort | 9, 44 | FX-56 |
| M-439 | **Stages de seconde** sur la plateforme nationale [H, NV] | Fonction récente, flux inconnu | confort | 44 | FX-43 |
| M-440 | **Taxe d'apprentissage** : UAI et coordonnées de l'établissement à jour sur la plateforme nationale [H] ; aucun flux nominatif | Simple veille, mais une UAI fausse fait perdre des fonds [H] | confort | 48 | §9.12 (note FX-56) |

### P05 — Karim, CPE (collège public)

**Année.** *Septembre* : badges de cantine, familles qui n'arrivent pas à se connecter. *Novembre* : une mère ajoute son nouveau conjoint comme responsable dans SIECLE (code « autre membre de la famille »). *Toute l'année* : passages à la cantine, signalements.

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-441 | **Passages de restauration et badges** (import des passages du logiciel de cantine, export des ayants droit) | Module « Passage » de Charlemagne [F, matrice §1] ; FX-95 le renvoie à « après le pilote » | important | 40, 9 | FX-88, ITP-149 |
| M-442 | **Notice ÉduConnect par famille non activée** (avec code de rattachement s'il y a lieu) et suivi des relances | ITP-35 et ITP-76 listent ; rien n'aide à agir sur la liste | confort | 9, 8 | ITP-129 |
| M-443 | **Nouveau responsable déclaré dans SIECLE avec un code de parenté vague** : jamais de lien d'autorité déduit ; proposition au socle 4 avec le code source affiché | Un beau-parent n'a pas l'autorité parentale [H, droit commun] ; un import ne doit pas la lui donner | important | 4, 9 | CX-54 |
| M-444 | **Signalements nationaux** (faits graves, absentéisme) : aide au pré-remplissage, jamais d'envoi automatique [H] | Double saisie fréquente ; mais flux non documenté | confort | 12, 13 | renvoi (socles 12, 13) |

### P06 — M. Petit, professeur responsable de l'emploi du temps

**Année.** *Juin* : services prévisionnels ; *août* : structure vers le logiciel d'EDT, retour des créneaux ; *septembre* : groupes renommés par STS. *Toute l'année* : ses notes dans un tableur qu'il veut verser dans ses devoirs.

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-445 | **Export de structure par un professeur responsable de l'EDT** : D-022 retire tout export du modèle « professeur » ; ITP-62 exige `echanges.exporter` (marqueur `export`) | Dans le privé, l'EDT est souvent fait par un professeur déchargé [H] ; il ne peut plus faire son travail, ou on lui donne un droit d'export trop large | important | 9, 1 | `echanges.exporter_structure` (§18.3), Q-X38, C13 |
| M-446 | **Reprise de la correspondance des groupes de N-1** pour N (codes annuels, §6.5) : proposition « même groupe que l'an dernier » | Sans elle, chaque rentrée refait tout le rapprochement des groupes | important | 9, 5 | ITP-142 |
| M-447 | **Import de ses notes depuis un tableur** dans ses propres devoirs, par le socle des notes, à sa portée | Ne relève pas de `echanges.importer` (établissement) ; doit exister au socle 21 avec le moteur de lecture | confort | 21, 9 | R-X27 |
| M-448 | **Résultats de certification Pix** dans les livrets [NV] | Attendu en 3ᵉ et terminale [H] | confort | 22, 23 | renvoi |

### P07 — Mme Royer, comptable de l'organisme de gestion

**Année.** *Août* : reprise des familles, des soldes, des mandats ; *septembre* : première remise de prélèvements ; *mensuel* : relevés, rejets, écritures pour l'expert-comptable ; *décembre* : paie, déclarations sociales.
**Incidents.** La banque livre ses relevés en format CFONB, pas en camt. Un mandat repris n'a pas de date de signature.

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-449 | **Reprise des mandats SEPA** (RUM, date de signature, séquence, identifiant créancier) | §0 point D | **bloquant** | 25, 9, 28 | FX-76, ITP-153, CX-61 |
| M-450 | **Soldes d'ouverture et échéanciers en cours** repris (impayés de N-1, avoirs) | Sans eux, la relance de septembre est fausse | important | 25, 9 | FX-106, ITP-153 |
| M-451 | **Relevés au format CFONB 120** [H : encore très répandu en France] | ITP-105 ne lit que camt ; sinon la comptable pointe à la main | important | 25, 9 | FX-75 |
| M-452 | **Canal EBICS** pour relevés et remises | Évite le téléchargement manuel ; plus tard | confort | 9 | FX-75 (note) |
| M-453 | **Facturation électronique** (réforme 2026–2027) et Chorus Pro pour les factures à une collectivité [NV : assujettissement d'un OGEC] | Obligation possible pour une partie des factures ; à vérifier avant le socle 48 | important | 48, 9 | FX-78, Q-X42 |
| M-454 | **Déclaration sociale nominative et comptes rendus** dans le catalogue du moteur (même cycle : pré-contrôle, dépôt, retour) | Le socle 49 (paie) aura le même besoin de retours bouclés ; le catalogue §9 l'ignore | important | 49, 9 | FX-77 |

### P08 — Famille Martin ; M. Dufour, père sous interdiction de contact

**Année (Martin).** Un enfant au collège public (ÉduConnect), un au lycée privé de P01 (sans ÉduConnect). Septembre : la mère corrige son adresse au guichet ; en novembre elle reçoit encore les courriers académiques à l'ancienne. Elle demande « à qui avez-vous transmis le dossier de ma fille ? ».
**Incident (Dufour).** Le juge a interdit le contact ; le lien est suspendu au socle 4. SIECLE le porte encore comme responsable légal. Il se reconnecte par ÉduConnect.

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-455 | **Rectification suivie jusqu'à la source**, visible par la famille : « corrigé ici le… ; à corriger dans SIECLE par l'établissement » ou « transmis à BEE le… » | Le droit de rectification ne s'arrête pas à ScolIA (§17) ; la famille ne voit rien et rappelle | important | 9, 4 | ITP-130 |
| M-456 | **« À qui mes données ont été transmises » en libre-service** (titulaire pour son enfant, élève majeur pour lui-même), sans valeurs | ITP-56 est réservé à la direction ; la demande arrive au secrétariat, qui la traite à la main | différenciant | 9, 27 | ITP-131, `echanges.mes_traces` |
| M-457 | **Rattachement par jointure bloqué après une révocation pour protection** | §0 point B ; R-X13 relit le lien à la connexion mais n'empêche pas la création d'un nouveau rattachement par la même jointure, ni sa validation en masse | important | 1, 9 | R-X19, CX-55 |
| M-458 | **Message clair « cet établissement n'utilise pas ÉduConnect »** quand un parent arrive du public | Il cherche le bouton, appelle le secrétariat | confort | 9 | ITP-83 (note) |
| M-459 | **Opposition au provisionnement tiers demandée par la famille** (formulaire), qui crée une tâche pour l'administrateur | ITP-92 n'a qu'une entrée administrateur | confort | 9, 27 | ITP-132 |
| M-460 | **Deux enfants, deux établissements ScolIA, un agenda** : deux abonnements, superposés par l'application mobile, jamais fusionnés côté serveur (D-013) | Demande naturelle ; la tentation serait un flux fusionné | confort | 9, 11 | CX-33 (note) |

### P09 — Hugo, 18 ans, terminale

**Année.** *Janvier–mars* : Parcoursup ; *mars* : il prend un prénom d'usage ; *juin* : bac, résultats.

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-461 | **Élève majeur maître de ses traces, de ses oppositions et de sa portabilité** | Le majeur exerce ses droits lui-même ; ITP-56, 58, 92 ne lui donnent aucune entrée | important | 9, 27 | ITP-131, ITP-132 |
| M-462 | **Voir avant la remontée ce qui partira** vers Parcoursup ou le LSL (ses moyennes, appréciations, sans les autres élèves) | Une erreur vue en avril est trop tard [H] | différenciant | 37, 23, 9 | ITP-133 |
| M-463 | **Changement de prénom ou de nom d'usage** : GAR et annuaires reçoivent l'usage ; le compte Google ou Microsoft est renommé sans perte (identifiant de service stable) ; exports nationaux à l'état civil (Q-X29) | Un compte recréé perd les documents ; un ancien prénom affiché blesse | important | 9, 2 | CX-56 |
| M-464 | **Résultats aux examens** rapatriés dans le dossier (attestations, statistiques) [NV] | Charlemagne publie des palmarès [H] ; aujourd'hui ressaisie | confort | 38, 9 | FX-09 |

### P10 — Yann, informaticien de l'ensemble

**Année.** *Été* : 900 comptes Google Workspace déjà créés à la main ; un annuaire Windows pour les postes ; un serveur de fichiers. *Septembre* : le certificat du canal SFTP expire pendant les vacances. *Mars* : un compte de personnel compromis.

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-465 | **Rapprocher un annuaire existant** (export Google ou Microsoft) **avant** tout provisionnement | Sinon ScolIA crée 900 doublons, ou Yann renonce au provisionnement | important | 9 | FX-98, ITP-143 |
| M-466 | **Annuaire local** (annuaire Windows, serveur pédagogique) : export CSV ou LDIF | Les élèves ouvrent une session sur un poste de l'établissement avec ces comptes [H] ; OneRoster ne le couvre pas | important | 9 | FX-97, ITP-144 |
| M-467 | **Expirations surveillées** (certificats, clés de l'IdP, secrets de canaux, conventions) : alertes J-30 et J-7 | Cause banale de panne de rentrée | important | 9, 1 | R-X26, ITP-145, CX-62 |
| M-468 | **Essayer un fournisseur SSO** sur une session d'essai avant activation ; un administrateur garde toujours une passkey locale | Une configuration fausse ferme l'accès à tous ; Q-X32 refuse un mode essai de données, pas un essai de connexion | important | 1, 9 | ITP-134, C16 |
| M-469 | **Révocation générale d'une personne en un geste** : sessions, rattachements, abonnements, jetons mobiles, comptes tiers, nouvelles assertions de l'IdP | Compte compromis ; aujourd'hui une action par brique | important | 1, 9 | R-X25, ITP-135, CX-64 |
| M-470 | **Coupe-circuit** d'un fournisseur fédéré ou d'un service tiers entier (incident chez lui) | Sans lui, on désactive établissement par établissement | important | 1, 9 | ITP-136 |
| M-471 | **Adresses réseau sortantes de la plateforme publiées** pour les pare-feu académiques et bancaires | Demandé par toute DSI [H] | confort | 9 | D-18 |
| M-472 | **Configuration des connecteurs exportable** (sans secrets) et journal des flux pour un audit | Audit, passation entre informaticiens | confort | 9 | ITP-137 |

### P11 — Mme Garnier, division de l'enseignement privé de la DSDEN

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-473 | **Heures supplémentaires et états de service des maîtres** (privé sous contrat), transmis au rectorat [NV : application et format] | La rémunération des maîtres dépend de ces états ; aujourd'hui tableur signé | important | 6, 49, 9 | FX-41, ITP-154 |
| M-474 | **Listes ponctuelles demandées par l'administration** : export daté, figé, minimisé, destinataire « DSDEN » tracé | Évite les extractions envoyées par courriel sans trace | confort | 9 | ITP-60 (note) |
| M-475 | **Fenêtres nationales respectées** : un dépôt hors fenêtre est signalé avant génération | Évite des dépôts inutiles et des appels | confort | 9 | ITP-152 (calendrier, M-491) |

### P12 — M. Laurent, DSI académique

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-476 | **Dossier technique de raccordement** prêt (architecture, hébergement, AIPD, adresses, contacts, procédure d'incident) | Toute convention le demande [H] ; sans lui, les démarches D-4 à D-7 s'allongent | important | 9 | D-18 |
| M-477 | **Notification d'incident** au responsable de traitement et à l'académie (fuite d'un lot, rattachement abusif) : périmètre tiré des traces et du journal | L'éditeur doit alerter vite [NV, délais RGPD] ; la conception journalise sans dire comment on s'en sert | important | 9, 1 | ITP-138 |
| M-478 | **Environnement de qualification ScolIA** avec données fictives, ouvert à la DSI | Les tests d'intégration se font des deux côtés | confort | 9 | §25.4 |

### P13 — RestoBadge (restauration) et un éditeur de logiciel de CDI

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-479 | **Bac à sable** à données fictives, documentation versionnée, **politique de dépréciation** annoncée | Sans eux, aucun partenaire n'investit ; chaque changement casse | important | 9 | FX-95 (note), §25.4 |
| M-480 | **Journal des lectures** du partenaire consultable par l'établissement (pas seulement les écritures) | Minimisation vérifiable ; le chef d'établissement a consenti, il doit voir | important | 9 | ITP-147 |
| M-481 | **Purge à la révocation** : demande signée, attestation attendue, relance | Révoquer l'accès ne supprime pas les données déjà copiées | important | 9 | R-X28, ITP-146, CX-63 |
| M-482 | **Nouveau champ ou nouvelle version = nouveau consentement** ; aperçu « ce qui part pour la première fois » | Un partenaire ou un connecteur qui élargit son périmètre en silence est la fuite la plus probable | important | 9 | R-X17 |
| M-483 | **Export des lecteurs pour le logiciel du CDI** (format du logiciel) | Le CDI importe aujourd'hui depuis SIECLE ou un tableur [H] | confort | 9 | FX-95 (note) |

### P14 — Mme Fontaine, DPO du réseau

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-484 | **Fiches du registre des traitements** générées par flux actif (finalité, catégories, destinataires, durée) | La notice publique existe (§17) ; le registre est une obligation distincte [H] | important | 9, 27 | ITP-139 |
| M-485 | **Élèves à confidentialité renforcée et identités protégées exclus par défaut** des annuaires sortants et des partenaires ; inclusion décidée par la direction, personne par personne | Un annuaire Google partagé du domaine rend un élève protégé trouvable ; la conception ne traite que l'adresse protégée (R-X15) | important | 9, 2 | R-X18, CX-57 |
| M-486 | **Copies locales des exports** : lien à usage unique de 15 minutes, avertissement « fichier nominatif », rappel | Les fichiers nationaux finissent sur des clés USB [H] | confort | 9, 7 | ITP-51 (note) |

### P15 — Mme Perret, directrice d'école privée

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-487 | **ONDE bloqué : aide à la saisie** (liste de travail, écarts suivis) en attendant | La directrice saisit dans ONDE seule, le soir [H] | confort | 50, 9 | FX-04 (note) |
| M-488 | **Liaison école → collège** dans le même ensemble (bilans de fin de cycle 3 [NV]) | Tenant multi-UAI (M-425) : la continuité est possible et attendue | important | 50, 22 | Q-X37 |

### P16 — Lucie, professeure documentaliste

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-489 | **Alimentation GAR conditionnée à la complétude** : pré-contrôle « élèves sans groupe, groupes sans enseignant », relance quotidienne jusqu'à la complétude | Médiacentre vide en septembre (§3 de la conception) ; l'alimentation part avant les groupes | important | 9, 5 | CX-41 (note) |
| M-490 | **Pix pour un établissement sans GAR** : import des élèves dans l'outil national [H] | Le privé hors GAR le fait à la main | confort | 9 | §9.12 (note FX-66) |

### Manques transverses

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-491 | **Calendrier national des campagnes** tenu par l'opérateur, avec une **couche académique** ; l'établissement signale un écart, il ne saisit pas | `ent_ech_echeance (source 'logiciel','etablissement')` laisse chaque secrétariat ressaisir les dates de son académie | important | 9 | ITP-152, Q-X44 |
| M-492 | **Dépôt automatique interrompu** (webservice Parcoursup coupé après 300 élèves) : état `depose_partiellement`, renvoi idempotent | Le cycle §8.2 ne connaît pas l'envoi partiel | important | 9 | CX-58, ITP-141 |
| M-493 | **Rejet non structuré** (compte rendu en PDF ou page) : rattachement assisté du rejet à l'enregistrement exporté | Tous les retours ne sont pas lisibles par une machine [H] | important | 9 | ITP-140, CX-65 |
| M-494 | **Reprise structurée des évaluations de N-1** (moyennes et appréciations par période et par matière) pour un lycée | §0 point A : Parcoursup et le LSL de terminale l'exigent la première année | **bloquant** | 9, 23, 37, 28 | FX-106 |
| M-495 | **Positionnement contractuel de la reprise** : le client demande lui-même à son ancien éditeur l'export complet de ses données (portabilité, réversibilité contractuelle) | Les formats de Charlemagne sont inconnus [H] ; l'établissement est responsable de traitement et peut l'exiger | important | 28, 9 | D-19 |
| M-496 | **Volume guard, rattachement, révocation : tests de recette** dans la liste §23 (bascule de juillet, rentrée ÉduConnect, révocation générale) | La recette personas (D-021) doit attaquer ces points précis | confort | 9 | §25.4 |

---

## 4. Couverture Charlemagne : échanges nationaux et identité (D-023, D-024)

Le contenu exact des fonctions d'échange de Charlemagne n'a pas pu être lu [H, même limite que la matrice §0]. La liste est celle qu'une secrétaire, une direction et une comptable habituées à Charlemagne attendent. « Bloqué » = bloqué par une démarche (§26), pas par la conception.

| Fonction attendue [H] | Avant la passe | Après la passe | Où |
|---|---|---|---|
| Import SIECLE (élèves, responsables, structures, nomenclatures) | conçu (X5, `Experimental`) | inchangé ; + rapprochement des affectés du socle 3, des sortis, jumeaux | FX-02, CX-49, 50, 53 |
| Import STS-web (personnels, services) | conçu (X6) | inchangé | FX-03 |
| Remontée BEE par le logiciel agréé | bloqué (D-3) ; cohabitation | + **journal à reporter**, profil de cohabitation, concordance avant constat, sorties à remonter | FX-107, R-X24, ITP-119–121 |
| LSU, LSL, Parcoursup, Affelnet, DNB | bloqué (D-2) ; pré-contrôle et aide à la saisie | + **stratégie de l'an 1 à trancher**, reprise de N-1, vérification par l'élève, export périmé | Q-X34, FX-106, ITP-133, R-X21 |
| Cyclades (inscriptions, PFMP), livret de la voie professionnelle | partiel | + durées de PFMP au pré-contrôle, LSL voie pro [NV] | FX-38, FX-34 (notes) |
| Résultats aux examens | absent | conçu [NV] | FX-09 |
| Échanges avec le logiciel d'EDT | conçu (X16) | + correspondance N-1 → N, export par le professeur responsable | ITP-142, `echanges.exporter_structure` |
| Prélèvements SEPA, relevés | conçu (camt, pain) | + **reprise des mandats**, CFONB 120, EBICS plus tard | FX-75, FX-76 |
| Export comptable, OGEC, UDOGEC, diocèse | conçu | + soldes d'ouverture, facturation électronique [NV], DSN au catalogue | FX-77, FX-78, FX-106 |
| Effectifs datés, constat | conçu (FX-62) | + effectifs à une date passée, contrôle de sincérité, attestation | ITP-125–127 |
| Cantine, badges (« Passage ») | renvoyé (FX-95 après pilote) | **conçu** en import de passages | FX-88 |
| UGSEL / UNSS, assurance scolaire | absent | exports de travail tracés | FX-66, FX-67 |
| Import de photos en masse (ELE-05) | absent du socle 9 | conçu | FX-108 |
| Reprise depuis Charlemagne (PIL-09) | « formats NV », socle 28 | + catalogue repris / archivé / abandonné, règles de conservation, démarche contractuelle | ITP-122, R-X23, D-19 |
| Comptes réseau des élèves (annuaire local) | absent | conçu | FX-97 |
| Extractions paramétrables (« requêteur ») | — | **hors socle 9** (socle 47) ; signalé | §5 |
| ÉduConnect, Agents, GAR (PIL-20) | bloqué (D-4 à D-6) | + positionnement derrière un ENT de collectivité, rattachement de rentrée, révocations | Q-X33, Q-X35, FX-99 |

**Verdict.** Sur le périmètre des échanges, la conception couvre tout ce que Charlemagne fait **hors agrément** et prévoit le reste ; ce qui manquait relève (1) de la **première année** (reprise, cohabitation), (2) des **petits flux de terrain** qui font la journée d'un secrétariat (cantine, photos, licences, assurance), (3) de la **preuve** (dépôt déclaré, attestation, journal scellé). L'équivalence avec Charlemagne **la première année** dépend de Q-X34 : sans validation LSU/LSL/Parcoursup et sans agrément BEE, ScolIA ne peut pas remplacer seul Charlemagne dans un lycée privé ; il peut le faire module par module.

---

## 5. Sécurité : vérifications demandées

| Thème | Ce que la conception fait bien [F] | Ce qui manquait | Ajout |
|---|---|---|---|
| **Minimisation des flux sortants** | champs déclarés par connecteur ; champs non déclarés non lus ; NUMEN jamais lu ; adresse protégée jamais en clair ; identifiants de service opaques ; aucun parent exporté par défaut vers les annuaires | (1) un **élargissement** (nouvelle version d'un connecteur, nouveau champ d'un partenaire) ne redemande rien ; (2) les **élèves protégés** (confidentialité renforcée, identité protégée) partent vers Google ou un partenaire comme les autres ; (3) les **copies locales** | R-X17, R-X18, ITP-51 (note) |
| **Comptes fédérés** | aucune création à la connexion ; rattachement par preuve forte ; message neutre ; guichet central sans session ; ticket lié à l'hôte et au navigateur | (1) contradiction §10.2 / R-X11 (a) qui pousse à la validation en masse (§0 B) ; (2) **nouvelle jointure** d'un parent révoqué ; (3) positionnement derrière un ENT de collectivité (le SSO entrant d'un ENT est une autre surface d'attaque, à durcir comme l'OIDC) | Q-X35, R-X19, FX-99 |
| **Révocation** | rattachement suspendu avec son lien, sessions révoquées (R-X13) ; partenaire révoqué immédiatement ; jetons iCal révocables | (1) **révocation générale** d'une personne compromise ; (2) **coupe-circuit** d'un fournisseur ou d'un service ; (3) **purge** chez un partenaire révoqué ; (4) **expirations** non surveillées | R-X25, R-X26, R-X28, ITP-135, 136, 146 |
| **Journal** | journal de chaque lot et export, inclusions, tentatives de connexion, journal de sécurité | (1) **journal des remontées scellé** (preuve inaltérable) ; (2) **lectures** des partenaires consultables ; (3) usage du journal pour une **notification d'incident** | R-X22, ITP-138, ITP-147 |
| **Reprise** | — (renvoyée au socle 28) | la reprise est le plus gros flux entrant de la vie d'un établissement et n'a pas de règle de minimisation | R-X23 |

Aucun point de la conception n'affaiblit le cloisonnement ; les ajouts ne créent **aucun** nouveau lecteur inter-établissements (la couche académique du calendrier est un référentiel global sans donnée personnelle).

---

## 6. Points où la conception contredit une décision (ou elle-même)

| # | Contradiction | Décision | Proposition |
|---|---|---|---|
| C10 | `SOCLES.md` §9 dit encore « parcours minimal vendable X0–X7 **sans aucun accès officiel** » ; `09_INTEROP.md` §0.9 et §25.2 ont été corrigés (jeux d'or réels nécessaires pour X5–X6) | — (cohérence documentaire) | note ajoutée dans `SOCLES.md`, rien supprimé |
| C11 | §9.10 « privé sous contrat : ScolIA fait foi » contre Q-X14 « cohabitation, le logiciel agréé remonte » | D-024 | profil `prive_contrat_cohabitation` (R-X24), Q-X34 |
| C12 | §10.2 (second élément obligatoire) contre R-X11 (a) (la jointure est une preuve forte) | D-013, D-017 | Q-X35 |
| C13 | ITP-62 (export vers le logiciel d'EDT, `echanges.exporter`, marqueur `export`) contre un professeur responsable de l'EDT | **D-022** | action `echanges.exporter_structure` sans donnée nominative d'élève, Q-X38 ; D-022 reste entière pour les exports officiels |
| C14 | Hypothèse implicite « ScolIA = projet ENT » (§11, X11, exigences n° 6 et 15) contre la pratique des ENT de collectivité dans le public [H] | D-019 (public dès le départ) | Q-X33, D-17 |
| C15 | FX-100 (dossier de transfert) ne dit pas ce qu'il contient ; D-027 (B) veut qu'un document destiné à un tiers ne révèle aucun dispositif | **D-027** | Q-X43 : l'établissement d'accueil n'est pas un tiers ordinaire, mais l'information sur un dispositif passe par la famille ou la notification officielle, jamais par défaut |
| C16 | Q-X32 (« pas de mode essai ») contre le besoin d'essayer une configuration SSO | — | compatible : ITP-134 est un essai de **connexion**, sans copie de données |
| C17 | Q-X11 (garde-fou à 5 % de disparitions) contre la bascule de juillet (25 à 35 % de départs normaux) | — | R-X20 |
| — | Q-X1 (dérogation à **D-009** pour la connexion fédérée) | **D-009** | déjà signalée par la conception ; **toujours à valider par Florian** ; la passe ne la tranche pas |

Aucune contradiction avec D-011, D-012, D-026 ou D-028. D-013 est respectée par tous les ajouts (aucune visibilité entre établissements ; l'élève « encore inscrit ailleurs » ne révèle que l'UAI, publique, donnée par le rejet national lui-même).

---

## 7. Manques qui relèvent d'autres socles

| Socle | Manques | Ce qu'il faudra prévoir |
|---|---|---|
| 1 Sécurité | M-427, M-431, M-457, M-468, M-469, M-470 | délégation de signature ; confirmation automatique sur chaîne officielle ; révocation générale ; coupe-circuit ; essai de fournisseur |
| 2 Élève | M-417, M-418, M-419, M-463, M-485 | candidats incluant les sortis ; règle des jumeaux ; photos en propositions ; identité protégée marquée pour les flux |
| 3 Inscriptions | M-432 | port de correspondance des dossiers `affectes` (exigence (a) du §5.4 élargie) |
| 4 Dossier administratif | M-416, M-443, M-455 | fusion groupée des doublons de responsables ; aucun lien d'autorité déduit d'un code vague ; état de la rectification |
| 5 Structure | M-434, M-446, M-489 | élèves sans division ; groupes annuels ; complétude des groupes pour le GAR |
| 6, 49 Personnels, paie | M-454, M-473 | DSN et retours ; états des heures des maîtres |
| 21 Notes | M-447 | import de notes à la portée du professeur |
| 22, 23, 37, 38 Livrets et examens | M-421, M-435, M-436, M-448, M-462, M-464, M-494 | stratégie de l'an 1 ; PFMP ; voie professionnelle ; Pix ; vérification par l'élève ; résultats ; reprise N-1 |
| 25 Paiements | M-449, M-450, M-451 | mandats, soldes, CFONB |
| 27 Droits des personnes | M-456, M-459, M-461, M-484 | traces en libre-service ; opposition ; registre |
| 28 Mise en service | M-410, M-411, M-421, M-494, M-495 | catalogue de reprise, règles de conservation, stratégie module par module, démarche auprès de l'ancien éditeur |
| 40 Restauration | M-441 | passages et ayants droit |
| 44 Stages, apprentis | M-435, M-437, M-438, M-439 | PFMP, enquête apprentis, entreprises, stages de seconde |
| 47 Pilotage | — | extractions paramétrables (hors socle 9, attendu de Charlemagne [H]) |
| 48 Comptabilité | M-453 | facturation électronique |
| 50 Premier degré | M-487, M-488 | aide à la saisie ONDE ; liaison école-collège |

---

## 8. Synthèse

### 8.1 Manques par gravité

| Gravité | Nombre |
|---|---|
| bloquant | 7 |
| important | 54 |
| confort | 28 |
| différenciant | 3 |
| **Total** | **92** (M-405 à M-496) |

Comptage fait par script sur les tableaux du §3. Les sept bloquants se regroupent dans les quatre points du §0 : **A** (M-405, M-406, M-421, M-494 : première année d'un lycée privé qui quitte Charlemagne), **B** (M-431 : rattachement de rentrée), **C** (M-428 : ScolIA derrière l'ENT de la collectivité), **D** (M-449 : mandats SEPA). Les trois différenciants : M-423 (contrôle de sincérité avant constat), M-456 (traces en libre-service), M-462 (l'élève vérifie ce qui partira).

**89** manques sont traités par un ajout dans `09_INTEROP.md` ; **3** sont renvoyés à leur socle sans ajout ici (M-427 délégation au socle 1, M-444 signalements aux socles 12 et 13, M-448 Pix aux socles 22 et 23) ; l'extraction paramétrable attendue de Charlemagne, sans numéro de manque, est renvoyée au socle 47.

### 8.2 Ajouts dans `09_INTEROP.md`

| Partie | Ajouts |
|---|---|
| En bref | §0 point 11 |
| Règles | **12** (R-X17 à R-X28), §6.12 |
| Flux | **19** fiches (FX-09, FX-41 à 43, FX-56, FX-66, 67, FX-75 à 78, FX-88, FX-97 à 99, FX-106 à 108), §9.12 |
| Identité fédérée | note de rentrée, §10.6 |
| Cas limites | **19** (CX-47 à CX-65), §12 |
| Actions | **36** (ITP-119 à ITP-154), §13.13 |
| Réglages signalés | **12** (S-68 à S-79), §13 |
| Droits | §18.3 (4 actions) |
| Lots | §25.4 (X7 bis, X8 bis, répartition) |
| Démarches | **4** (D-17 à D-20), §26 |
| Questions | **12** (Q-X33 à Q-X44), §27 |
| Contradictions | **8** (C10 à C17), §28 |

### 8.3 Les dix ajouts les plus importants

1. **Stratégie de l'année 1 et journal de cohabitation** (Q-X34, FX-107, R-X24 ; M-405, M-406, M-421). Le lycée privé quitte Charlemagne sans double saisie aveugle.
2. **Reprise structurée de N-1 et catalogue de reprise signé** (FX-106, ITP-122, R-X23 ; M-410, M-411, M-494). Parcoursup de terminale marche la première année ; la reprise ne ressuscite rien.
3. **Rattachement de rentrée automatique sur chaîne officielle, bloqué après protection** (Q-X35, R-X19 ; M-431, M-457). La protection tient même un 2 septembre.
4. **Deux positionnements dans le public** (Q-X33, FX-99, X8 bis ; M-428, M-429). ScolIA se vend aussi derrière l'ENT du département.
5. **Mandats SEPA et soldes repris** (FX-76, ITP-153 ; M-449, M-450). La première remise de septembre passe.
6. **Dépôt déclaré, export périmé, attestation et journal scellé** (R-X21, R-X22, ITP-124, 126 ; M-413, M-414, M-422). « Prouver ce qui est parti » devient un geste.
7. **Garde-fou de volume conscient de la bascule** (R-X20 ; M-430). Le garde-fou reste un garde-fou.
8. **Révocation générale, coupe-circuit, expirations, purge des partenaires** (R-X25, 26, 28 ; M-467, M-469, M-470, M-481).
9. **Minimisation qui couvre l'élargissement et les élèves protégés** (R-X17, R-X18 ; M-482, M-485).
10. **Les petits flux du secrétariat** (FX-66, 67, 88, 97, 98, 108 ; M-419, M-420, M-441, M-465, M-466). Ce qui fait revenir au tableur s'il manque.

Suivent de près : concordance BEE et contrôle de sincérité avant constat (ITP-120, 127), « élève encore inscrit ailleurs » (ITP-121), rejets traduits et rejets non structurés (ITP-123, 140), calendrier national à couche académique (ITP-152), traces et droits en libre-service pour les familles et le majeur (ITP-130 à 133).

### 8.4 Ce que la passe a montré sur la conception

- **Le moteur est pensé pour l'année de croisière ; la première année est sous-conçue.** Cohabitation, reprise, historique de N-1, mandats : c'est là que se gagne ou se perd un client qui quitte Charlemagne.
- **Les garde-fous doivent connaître le calendrier.** Un garde-fou qui sonne chaque juillet, une validation qui demande 1 300 clics un 2 septembre : les deux finissent en clic réflexe, donc en absence de protection.
- **Le dépôt est souvent humain.** La moitié des remontées se font par téléversement manuel [H] ; l'état d'un export doit pouvoir être déclaré et prouvé par la personne.
- **ScolIA n'est pas toujours l'ENT.** Dans le public, il faut savoir être un service raccordé.
- **La minimisation doit suivre le temps** : ce qui part aujourd'hui doit avoir été consenti pour ce périmètre, et les élèves protégés ne suivent pas la règle commune.
- **D-023 tient** : les ajouts sont des comportements par défaut ; douze réglages sont signalés avec une version automatique (§9).

---

## 9. Réglages signalés (D-023)

Liste à part, recopiée dans `09_INTEROP.md` §13 ; **rien n'est supprimé**. « Version automatique » = ce que fait le logiciel quand l'établissement ne touche à rien.

| # | Réglage | Pourquoi il pèse | Version automatique proposée |
|---|---|---|---|
| S-68 | **Existant** : règles d'application par connecteur × entité × **champ** × classe (`ent_ech_regle`) | une matrice que personne ne relit | par **famille de champs** (identité, coordonnées, scolarité), préréglée par le profil ; le champ seul en mode avancé replié |
| S-69 | **Existant** : `version_preferee` d'un connecteur | aucun établissement ne sait choisir une version de format | la plus récente acceptée ; champ retiré de l'écran |
| S-70 | **Existant** : seuils de volume réglables 5–30 % (ITP-41) | réglage sans enjeu local, abaissé pour « faire passer » | 15 % / 5 % fixes, conscients de la bascule (R-X20) |
| S-71 | **Existant** : politique de disparition (ITP-42) | deux valeurs, une seule juste par connecteur | fixée par le connecteur |
| S-72 | **Existant** : fenêtre horaire des imports automatiques (ITP-45) | sans enjeu local | nuit, fixe |
| S-73 | **Existant** : contenu des abonnements iCal (ITP-101 : salles, noms) | deux interrupteurs | défaut Q-X19, un seul interrupteur « afficher le professeur » |
| S-74 | **Existant** : échéances saisies par l'établissement | chaque secrétariat ressaisit les dates de son académie | couche académique tenue par l'opérateur ; l'établissement signale (ITP-152) |
| S-75 | **Existant** : choix manuel de l'encodage, choix du connecteur quand la détection hésite (ITP-02) | déjà signalé par la conception | détection seule ; choix seulement en cas d'ambiguïté réelle |
| S-76 | **Existant** : attributs transmis au GAR | déjà signalé ; imposés par le contrat | non réglables |
| S-77 | **Existant** : profil d'établissement modifiable par l'administrateur (ITP-44) | changement lourd de conséquences | posé à la mise en service, changé par l'opérateur sur demande ★ |
| S-78 | **Existant** : rattachement par e-mail professionnel autorisé | élargit la surface d'attaque | désactivé, non proposé hors personnels |
| S-79 | **Ajout** : délai de relance « export généré non déclaré déposé » (R-X21) | tentation de le rendre réglable | J+3 ou 3 jours avant l'échéance, fixe |

---

## 10. Faits, hypothèses, recommandations, questions

**Faits [F]** vérifiés dans la conception : cohabitation limitée à BEE (Q-X14) ; §9.10 « ScolIA fait foi » dans le privé ; second élément obligatoire au rattachement (§10.2, obj. 2) et jointure « preuve forte » (R-X11 a) ; garde-fou de 5 % de disparitions (Q-X11) ; cycle d'export `depose → accuse` sans dépôt déclaré (§8.2) ; `ent_ech_echeance` saisie par l'établissement ; ITP-56, 58, 92 réservées à la direction ou à l'administrateur ; ITP-62 sous `echanges.exporter` (marqueur `export`) ; ports listés §5.4 (a) sans le socle 3 ; FX-95 renvoyé après le pilote ; relevés limités à camt ; aucune fiche de flux pour la cantine, les photos, l'annuaire local, les mandats, la DSN ; `SOCLES.md` §9 « sans aucun accès officiel » ; module « Passage » de Charlemagne (matrice §1).

**Hypothèses [H]** : la vie des personas et les fréquences ; la pratique des ENT de collectivité dans le public et le raccordement des logiciels de vie scolaire par SSO ; le dépôt manuel majoritaire ; la persistance du format CFONB 120 ; les doublons de responsables dans les anciens logiciels ; les comptes Google créés à la main ; l'annuaire local des postes ; les demandes UGSEL, UNSS, assurance ; les fonctions de Charlemagne.

**Vérifié le 2026-11-09** : délais d'effacement des sanctions (R511-13 IV, `docs/VERIF_REGLEMENTAIRE.md` R34).

**À vérifier [NV]** (interdit de coder dessus) : accès d'un service de vie scolaire à l'AAF ; règles SEPA de reprise des mandats et de changement de RUM ; durées de PFMP et positionnements aux examens ; livret de la voie professionnelle ; enquête sur les apprentis ; plateforme des stages de seconde ; heures supplémentaires des maîtres du privé (application, format) ; assujettissement d'un OGEC à la facturation électronique ; Chorus Pro ; Pix dans les livrets ; résultats d'examens extraits de Cyclades ; liaison de fin de cycle 3 ; FranceConnect pour le privé (Q-X41) ; délais de notification d'un incident.

**Recommandations [R]** : celles du §0 et les règles R-X17 à R-X28 ; X7 bis (cohabitation) et X8 bis (SSO entrant depuis un ENT) comme lots nouveaux ; aucune autre création de lot.

**Questions** : Q-X33 à Q-X44 (`09_INTEROP.md` §27). **À trancher en premier : Q-X34, Q-X35, Q-X33**, puis la reprise des mandats (point D, sans question : c'est une exigence).

**Compléments de modèle** : les ajouts entrent dans les migrations prévues (3900, 3905, 3907, 3910, 3913) ou dans la réserve 3915–3989 ; **à attribuer dans `MIGRATIONS.md`** au moment du code (non fait ici, aucune migration n'est écrite).
