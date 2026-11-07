# Passe « personas » sur le socle 7 — Documents, pièces et signatures

**Date :** 2026-11-07 — **Statut :** proposition, **non décidée** (à relire par Florian). Aucun code écrit ni modifié ; aucun accès à la base ni aux secrets de production ; l'application Next.js n'a pas été lue pour ce document.
**Branche :** `ent-personas-socle-7` (base `ent-mc0hoe` `e2848bc`, qui porte `07_DOCUMENTS.md`).
**Éprouve :** `docs/socles/07_DOCUMENTS.md` (lu en entier), avec `CLAUDE.md`, `VISION.md`, `DECISIONS.md` (D-017, D-019 à D-026, surtout D-023), `SOCLES.md`, `MATRICE_CHARLEMAGNE.md` (§1, §3.10), `docs/recette/RECETTE_PERSONAS_1.md` (lu sur la branche `ent-recette-1-correctifs`, absent de la base de cette branche : constats RP1-05, RP1-12, RP1-29), `PERSONAS_05-06.md` (méthode et format repris).
**Ajouts :** faits **directement** dans `07_DOCUMENTS.md`, marqués **[personas]** : cas limites `D71` à `D100` (§4.2), natures complémentaires (§4.1 bis), compléments de modèle (§8.14), actions `DCT-129` à `DCT-168` (§14.14), règles `R-D1` à `R-D38` (§14.15), réglages signalés par la passe (§15.3, liste à part, rien n'est supprimé), droits (§18.3), événements (§19.3), contrat IA (§20.5), écrans (§21, note), lots (§23, note), questions `Q-D31` à `Q-D44`. Quand un ajout contredit le texte d'origine, une note **[personas]** est posée à côté ; rien n'est supprimé.
**Numérotation :** les manques continuent `PERSONAS_05-06.md` (qui s'arrête à M-236) : **M-237 à M-333**. Les réglages signalés continuent aussi : **S-44 à S-55**.

---

## 0. À lire en premier : ce qu'il faut trancher avant le premier lot de code

Aucun lot du socle 7 n'est commencé (conception seulement). En faisant vivre la conception par celles et ceux qui délivrent, reçoivent, signent et gardent les documents, huit points apparaissent qui changent les **premiers** lots (D2, D3, D4, D6). Ils sont à trancher avant d'écrire les migrations 3700–3705.

| # | Point | Pourquoi | Ce que le lot doit faire | Gravité | Lots touchés |
|---|---|---|---|---|---|
| A | **Les dossiers des personnels ne sont pas des dossiers d'élèves.** `documents.lire` en portée « établissement » ouvre DCT-30 (« dossier d'une personne ou d'un agent ») à tout le secrétariat ; le modèle ne connaît que deux domaines restreints (`categorie_sante`, `categorie_justice`) | La secrétaire de scolarité lirait le diplôme, le contrat, l'arrêt maladie ou le titre de séjour de sa collègue (P09) ; les avis d'imposition des bourses seraient lus de tout le secrétariat (D50 le promet pourtant) | **R-D6** : un **domaine** par document (`scolarite`, `sante`, `justice`, `financier`, `rh`, `social`, `disciplinaire`) à la place des deux booléens ; action distincte `documents.personnels.lire` et `documents.financier.lire`. **Dans la migration 3701** | **bloquant** | D2, D5 |
| B | **Le certificat de scolarité trahit des informations sensibles.** Rien ne dit ce que le certificat imprime de la classe : le libellé de division (« 4ᵉ SEGPA », « 5ᵉ ULIS », « UPE2A ») ou le régime | Un certificat va à la CAF, au club de sport, à l'employeur d'un parent : il révèle une orientation adaptée ou un handicap (P01, P20) ; c'est la **première** nature codée (D3) | **R-D3** : certificats et attestations destinés à un tiers portent le **niveau et la formation**, jamais le libellé de division ni un dispositif, sauf nature dédiée demandée par la famille | **bloquant** | D3 |
| C | **Un titulaire sans compte ne peut rien signer en ligne.** Le signataire « externe » est réservé à « une personne majeure qui agit pour une organisation » (§17.7) ; un parent sans compte n'a que le papier (D22) | Le second parent séparé n'a souvent jamais activé son compte [H] ; pour toute nature « tous les titulaires » (voyage à l'étranger, contrat, PAI), le circuit reste bloqué et le papier revient en masse (P05, P19) | **R-D8**, **DCT-137** : lien personnel nominatif + code envoyé au canal **vérifié** du socle 4 (S1), qui propose d'activer le compte. Rôle `titulaire` avec lien dans `ent_doc_signataire` **dès 3705** | **bloquant** | D6 |
| D | **Le délai d'une procédure part de la notification, pas de l'émission.** La conception bloque une convocation dont le délai est trop court (D35) mais ne dit pas **quand** le destinataire est réputé notifié, ni comment la discipline le lit | Une exclusion prononcée deux jours ouvrables après une émission au coffre jamais ouverte risque l'annulation pour vice de procédure [NV, à vérifier] (P02) | **R-D10**, **DCT-156** : chaque nature déclare les modes qui valent notification ; `etat()` renvoie la date de notification **par destinataire** | **bloquant** | D3, D4, D8 (socle 13) |
| E | **Une facture ne se « réémet » pas.** Le principe 4 (« corriger = nouvelle version, l'ancienne remplacée ») s'applique à toutes les natures | Une facture émise ne se remplace pas : on émet un **avoir** [NV, à vérifier : règles de facturation] ; une quittance non plus (P04) | **R-D2** : `correction` déclarée par la nature (`NouvelleVersion` ou `DocumentCompensatoire`) dans `NatureDecl`, **dès D2** | **bloquant** | D2, D8 (socle 24) |
| F | **Deux personnes morales émettent des documents.** `ent_doc_identite` a une seule ligne par établissement | Dans le privé, les factures, contrats de travail et bulletins de paie sont émis par l'**organisme de gestion** (raison sociale, SIRET), les certificats par l'**établissement** (UAI) ; un CFA a son numéro de déclaration d'activité [NV] (P04) | **R-D13**, **DCT-164** : entité émettrice par famille de natures, avec son identité et ses séries | important | D2, D3 |
| G | **Envoyer un document à quelqu'un qui n'a pas de compte.** Q-D13 interdit à raison la pièce jointe ; mais rien ne remplace l'e-mail pour une famille sans compte, une organisation payeuse, un OPCO, un établissement d'accueil hors ScolIA | Sans canal prévu, le secrétariat enverra le PDF en pièce jointe depuis sa messagerie : c'est la fuite la plus probable du socle [H] (P01, P04, P07, P13, P19) | **R-D9**, **DCT-129** : envoi par lien nominatif à usage limité, avec code pour les documents C2 et plus, inscrit au registre de départ | important | D4 |
| H | **Le catalogue des pièces interdites vaut pour les familles, pas pour l'employeur.** Le §11.2 interdit titre de séjour, carte Vitale et casier judiciaire | L'OGEC **doit** vérifier l'autorisation de travail d'un salarié étranger et en garder copie [NV, à vérifier] ; la paie a besoin du numéro de sécurité sociale (P09) | **R-D5** : interdits limités aux élèves et familles ; pièces RH exigées par le droit du travail dans un catalogue RH (domaine `rh`) | important | D5 |

---

## 1. Méthode

**Démarche.** Celle de `PERSONAS_05-06.md` §1 : 24 personas réalistes et variées, choisies dans le tableau « acteurs » du socle 7 (§3) et dans la liste de la consigne, complétées pour couvrir public, privé sous contrat, hors contrat ; école, collège, lycée général, lycée professionnel, BTS et apprentissage ; métropole ; bureau, guichet, direction, terrain ; familles séparées, non francophones, protégées, placées ; élève mineur et majeur ; personnes extérieures (entreprise, médecin, contrôleur, archiviste, tiers vérificateur). Pour chacune : son année (rentrée, fin de trimestre, examens, départs, fin d'année), puis ses **incidents** (document erroné à annuler, signature contestée, pièce illisible, parent qui ne signe pas, demande RGPD, panne). Chaque fois qu'elle voulait faire quelque chose que la conception ne prévoit pas, ou que c'était trop long, ambigu, risqué ou frustrant, le manque est noté.

**Grille transversale** posée à chaque persona : qui le voit ? ; erreur et annulation ; contestation et preuve ; fraude ; personne sans compte ; langue et accessibilité ; parents séparés et adresse protégée ; majorité ; départ et ancien élève ; panne ; deux personnes morales ; durée de conservation ; « le ferait-elle encore dans Word ? » (D-023).

**Couverture Charlemagne** (D-023, D-024) : les lignes DOC-01 à DOC-11 de la matrice, et les fonctions documentaires qu'un utilisateur de Charlemagne attend [H : le détail des « éditions » de Charlemagne n'a pas pu être vérifié, même limite que la matrice §0], ont été relues ; celles qui manquaient sont au §4.

**Gravité** (identique aux passes 2–4 et 5–6).
- **bloquant** : sans lui, l'établissement contourne de façon dangereuse (sécurité d'un enfant, fuite d'une donnée, pratique probablement illégale) ou ne peut pas traiter un cas réel ; ou la conception se contredit au point d'empêcher de coder.
- **important** : cas fréquent sans solution propre, perte de temps forte ou risque d'erreur ; ou fonction de Charlemagne absente.
- **confort** : améliore nettement le quotidien ; un contournement existe.
- **différenciant** : peu de logiciels du marché le font [H, comparaison de mémoire] ; argument de vente.

**Conventions.** [H] hypothèse ; **[NV, à vérifier]** règle réglementaire citée de mémoire, à vérifier sur le texte officiel avant de coder ; [R] recommandation. Numéros de socles : `SOCLES.md`.

**Limites.** Les personas sont construites, pas interviewées. Toutes les règles juridiques citées (facturation, droit du travail, procédure disciplinaire, archives, langue des documents, apostille, apprentissage, bulletins de paie) le sont **de mémoire** : marquées [NV]. Légifrance n'a pas été consulté pour ce document. Aucun sous-agent n'a été utilisé : la passe ne demandait pas de recherche dans le dépôt au-delà des documents lus.

---

## 2. Les personas en un tableau

| # | Persona | Établissement | Ce qu'elle éprouve surtout |
|---|---|---|---|
| P01 | **Sylvie**, secrétaire de scolarité, sous Charlemagne depuis douze ans | Lycée privé sous contrat, 1 100 élèves dont deux BTS | Certificats de rentrée, formulaires tiers, envois, radiations, courriers types |
| P02 | **Rachid**, CPE | Collège public en éducation prioritaire, 600 élèves | Avis de punition, procédure contradictoire, conseil de discipline, recours |
| P03 | **Mme Garnier**, proviseure | Lycée professionnel public, 700 élèves | Parapheur, conventions la veille du départ, documents rédigés ailleurs |
| P04 | **Hélène**, comptable de l'organisme de gestion | Ensemble scolaire privé : école, collège, lycée, unité de formation par apprentissage | Factures, avoirs, payeurs tiers, deux personnes morales, contrats de travail |
| P05 | **Marc et Céline**, parents séparés de Lou (4ᵉ) et de Tom (CM2) | Lou au collège, Tom à l'école d'un autre établissement ScolIA | Autorisation de voyage à signer par les deux, Marc sans compte, contestation |
| P06 | **Mme Kovalenko** (ukrainienne, peu de français) ; **M. Diallo** (ne lit pas le français) | Collège public avec UPE2A | Convocations incomprises, signature au guichet avec interprète, pièces étrangères |
| P07 | **Inès**, 19 ans, BTS en apprentissage | Unité de formation par apprentissage du lycée de P01 | CROUS, CAF, employeur, opérateur de compétences, refus d'informer ses parents |
| P08 | **M. Lambert**, professeur référent des périodes en entreprise | Lycée professionnel de P03 | 90 conventions par an, entreprises qui imposent leur modèle, mineurs en atelier |
| P09 | **Julien**, contractuel arrivé le 13 novembre ; **Aïcha**, surveillante salariée de l'organisme, de nationalité étrangère | Collège public ; ensemble privé de P04 | Pièces d'arrivée, autorisation de travail, contrat, bulletins de paie |
| P10 | **M. Fabre**, inspecteur ; **Mme Ndiaye**, auditrice qualité de l'apprentissage ; le **commissaire aux comptes** de l'organisme | Hors contrat ; UFA de P04 ; organisme de P04 | Preuves, accès temporaire, dossier de contrôle, émargements |
| P11 | **Mme Roux**, archiviste du service départemental ; **Bernard**, adjoint gestionnaire chargé des archives | Lycée public | Élimination, archives papier, tri, preuve à long terme |
| P12 | **Fatou**, agent d'accueil | Lycée public | Guichet : dépôts, remises contre signature, demandes au téléphone |
| P13 | **Mme Perret**, directrice d'école (déchargée un jour par semaine) | École publique de cinq classes | Tout faire seule : radiations, transmissions, pièces de la mairie |
| P14 | **Léo**, 15 ans, élève de 3ᵉ | Collège de P02 | Stage d'observation, règlement, convocations, papiers perdus |
| P15 | **Clara**, infirmière ; le **Dr Morel**, médecin qui suit un élève | Collège public | PAI à quatre signatures, renouvellement, justificatifs médicaux |
| P16 | **M. Petit**, délégué à la protection des données mutualisé | Huit établissements privés d'un diocèse | Demandes d'accès, occultation, réquisitions, violation |
| P17 | **Thomas**, ancien élève (bac 2011) ; **Mme Benoît**, ancienne surveillante | Lycée de P01 | Attestations des années après, identité sans compte |
| P18 | **Les tiers vérificateurs** : agent de la CAF, consulat, préfecture, employeur | — | Vérifier vite, formulaires à faire remplir, original signé à la main |
| P19 | **Nadia**, éducatrice de l'aide sociale à l'enfance ; **Mme Robert**, assistante familiale | Collège de P02 | Enfant placé, service gardien, actes usuels, changements d'éducateur |
| P20 | **Mme Lefort**, mère protégée par une ordonnance (adresse protégée) | Lycée de P01 | Ce qu'un document laisse deviner |
| P21 | **Kenza**, victime de harcèlement ; **les parents de l'élève mis en cause** ; **les membres du conseil de discipline** | Collège de P02 | Noms cités dans un courrier, consultation du dossier |
| P22 | **Yann**, référent numérique et administrateur | Ensemble privé de P04 | Lot erroné de 300 documents, cachet indisponible, panne de rentrée |
| P23 | **M. Weber**, directeur | Établissement hors contrat international, 300 élèves | Relevés en anglais et en allemand, apostille, diplômes maison |
| P24 | **Mme Laurent**, cheffe d'établissement coordinatrice | Ensemble de P04 (trois unités, deux numéros UAI) | Identités, séries, signataires, organisme de gestion |

---

## 3. Parcours et manques, persona par persona

Chaque tableau donne : le manque, pourquoi il compte, sa gravité, le socle concerné, et l'ajout fait (identifiant dans `07_DOCUMENTS.md`) ou le socle ultérieur où il est renvoyé.

### P01 — Sylvie, secrétaire de scolarité (lycée privé sous contrat, sous Charlemagne)

**Année.** *Fin août* : 300 demandes de certificat la première semaine, dont un tiers par téléphone (« vous pouvez me l'envoyer par mail ? ») ; une dizaine de familles n'ont pas de compte actif. *Septembre* : les familles apportent des **formulaires d'autres organismes** à « remplir et tamponner » : attestation de la région pour le transport scolaire, imprimé de la mutuelle, formulaire de la caisse d'allocations pour un enfant de plus de 16 ans [NV], certificat du club de football, formulaire d'un consulat. *Octobre* : trois dossiers scolaires à transmettre à des établissements qui ne sont pas sous ScolIA ; la boîte e-mail officielle du lycée reçoit un exeat, un jugement envoyé par un avocat, un courrier du rectorat. *Toute l'année* : la même lettre revient vingt fois (« information aux familles des élèves de 1ʳᵉ sur les épreuves anticipées », « rappel de la fiche d'urgence », « courrier aux familles qui n'ont pas rendu l'attestation d'assurance ») : dans Charlemagne, un publipostage Word. *Juin–juillet* : 180 certificats de radiation ; une famille qui doit 1 200 € demande l'exeat ; la direction lui dit « pas d'exeat sans règlement ».
**Incidents.** Un certificat affiche « 2ᵈᵉ Arc-en-ciel » (le libellé de la division) : la CAF le refuse, ne reconnaissant pas la classe. Un autre, pour un élève de la section adaptée, affiche « 4ᵉ SEGPA » : la famille proteste, l'organisme n'avait pas à le savoir. Une mère dépose de sa propre initiative, sans qu'on le lui demande, la nouvelle ordonnance du juge : il n'y a pas d'endroit pour la recevoir.

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-237 | **Envoyer un document par lien sécurisé** à une personne sans compte (canal vérifié) ou à une organisation, sans pièce jointe | La demande « envoyez-le-moi par mail » est quotidienne ; sans canal prévu, le PDF part en pièce jointe depuis une messagerie personnelle (§0 point G) | important | 7 | DCT-129, R-D9, Q-D33 |
| M-238 | **Compléter un formulaire tiers** (région, caisse, mutuelle, consulat, club) : le déposer, l'émettre avec une page d'attestation (données, cachet, code de vérification), ou noter « complété à la main et tamponné » | Geste de guichet très fréquent [H] que ni la conception ni Charlemagne ne prévoient ; aujourd'hui aucune trace de ce qui a été attesté | important | 7 | DCT-130 |
| M-239 | **Courrier type de l'établissement** réutilisable : objet et corps enregistrés, quelques champs d'un catalogue court (prénom, nom, classe, date), sans mise en forme, utilisable en lot | Le catalogue de natures ne peut pas prévoir chaque lettre récurrente ; sans courrier type, le secrétariat retourne à Word, ce que D-023 veut éviter | important | 7 | DCT-131, R-D26, Q-D32, S-44 |
| M-240 | **Certificat neutre** : niveau et formation, jamais le libellé de division ni un dispositif (SEGPA, ULIS, UPE2A, classe relais), sauf nature dédiée demandée par la famille | §0 point B : révèle une orientation adaptée ou un handicap à un tiers ; le libellé fantaisie fait refuser le certificat | **bloquant** | 7 (+5) | R-D3 |
| M-241 | **Certificat de radiation jamais conditionné au paiement** ; la situation financière fait l'objet d'une nature distincte (attestation de situation de compte) du socle 24 | Retenir l'exeat pour une dette est probablement illicite [NV, à vérifier] et bloque l'inscription ailleurs | important | 7, 24 | R-D4, §4.1 bis |
| M-242 | **Transmettre le dossier scolaire** à un établissement hors ScolIA par lien vers son adresse officielle, au registre de départ | INS-74 suppose un envoi explicite sans dire lequel ; aujourd'hui papier ou pièce jointe | important | 7, 3 | DCT-129 |
| M-243 | **Recevoir les e-mails de la boîte officielle** dans le registre du courrier ou le bon dossier (pièces en quarantaine) | Jugements, exeats, courriers du rectorat arrivent par e-mail ; le registre de la conception ne connaît que le papier | différenciant | 7 (+8) | DCT-151, Q-D42 |
| M-244 | **Dépôt spontané** d'un document par une famille (sans demande), file « à qualifier » routée selon le type | Une ordonnance nouvelle, un certificat, un courrier n'attendent pas qu'on les demande ; DCT-45 ne connaît que les dépôts demandés | important | 7 | DCT-144, DCT-145 |
| M-245 | **Mention du libre-service** sur le document (« délivré en ligne sous l'autorité du chef d'établissement, vérifiable par code ») et règle sur la signature visuelle d'un document que personne n'a signé | Un certificat émis à 23 h en libre-service porterait la signature scannée du chef qui ne l'a pas vu [NV : portée juridique à vérifier] | confort | 7 | R-D18, Q-D36 |

### P02 — Rachid, CPE (collège public en éducation prioritaire)

**Année.** *Chaque semaine* : cinq à dix avis de punition ou de retenue, une ou deux convocations de familles. *Novembre* : un élève est mis en cause pour des violences ; la procédure contradictoire impose d'informer l'élève et ses représentants des faits et de leur laisser un délai pour présenter leur défense [NV : R421-10-1, au moins deux jours ouvrables, à vérifier]. Rachid rédige ; l'adjointe relit ; la principale signe. Le courrier part au coffre le vendredi à 17 h ; la mère ne l'ouvre que le lundi. *Décembre* : conseil de discipline ; la famille a le droit de consulter le dossier avant la séance [NV] ; les membres (dont deux parents élus et deux élèves) doivent pouvoir le lire. *Février* : la famille forme un recours ; le rectorat **réforme** la sanction (exclusion de huit jours ramenée à trois) [NV : procédure d'appel à vérifier] ; le tribunal administratif demande ensuite des pièces.
**Incidents.** Le courrier d'information cite en toutes lettres la victime : « a frappé Kenza B. » ; il part aux parents de l'élève mis en cause. Le jour du conseil, la mère dit n'avoir jamais été notifiée dans les délais.

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-246 | **Date de notification effective par destinataire**, déclarée par la nature (remise contre signature, avis de réception, ouverture du coffre, papier simple) et lue par la discipline | §0 point D : un délai calculé depuis l'émission expose à l'annulation de la sanction [NV] | **bloquant** | 7, 13 | R-D10, DCT-156 |
| M-247 | **Visa interne** avant signature (la CPE rédige, l'adjointe vise, la cheffe signe), à l'unité ou en lot, sur téléphone | DCT-26 ne permet que le renvoi ; le circuit réel a deux ou trois étapes [H] ; sans visa, la relecture se fait par e-mail | important | 7 | DCT-140 |
| M-248 | **Contrôle de contenu à l'aperçu** : nom d'un autre élève, mention de santé ou de dispositif dans un texte libre → avertissement à confirmer | La victime est nommée dans le courrier envoyé à la famille de l'auteur ; c'est une fuite sur un mineur, irréversible | important | 7, 13 | DCT-157, R-D11 |
| M-249 | **Consultation du dossier** par la famille et par les membres d'une instance : accès temporaire, lecture seule, filigrane nominatif, expiration, consultations listées | Droit de la défense [NV] ; aujourd'hui des photocopies qui circulent après la séance | important | 7, 13 | DCT-148, DCT-149, R-D19 |
| M-250 | **Sanction annulée ou réformée** (recours, appel) consommée : courriers marqués, coffre mis à jour, nouveau courrier proposé | §19.2 ne consomme que l'effacement (D36) ; une sanction annulée resterait affichée comme valable | important | 7, 13 | §19.3, D79 |
| M-251 | **Dossier de contentieux** : sélection des pièces, index, empreintes, dossiers de preuve, export scellé, gel automatique | Le tribunal demande les pièces de la procédure ; les rassembler à la main fait oublier l'accusé de remise | important | 7 | DCT-150 |
| M-252 | Convention de **mesure de responsabilisation** avec une structure partenaire (signataire externe) | Nature absente du catalogue ; la mesure se fait hors de l'établissement [NV] | confort | 13, 7 | §4.1 bis |

### P03 — Mme Garnier, proviseure (lycée professionnel public)

**Année.** *Chaque matin* : le parapheur sur son téléphone, dans le tramway. *Octobre et mars* : 120 conventions de période en entreprise la veille des départs ; elle veut une **synthèse** (anomalies, dates hors période, tuteur sans contact) plutôt que 120 PDF. *Toute l'année* : des documents qu'elle rédige ailleurs (réponse au rectorat, attestation pour la banque d'un agent, courrier à la mairie, lettre de soutien à un projet) et qu'elle veut signer avec la même passkey et le cachet. *Avril* : elle part six semaines ; le proviseur adjoint assure l'intérim : les documents doivent dire « le proviseur adjoint, chargé de l'intérim ».
**Incidents.** Elle valide par erreur un lot de 40 courriers mal datés ; dix minutes après, elle voudrait que rien ne soit parti. Un grand groupe hospitalier refuse la convention du lycée et impose la sienne.

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-253 | **Signer pour l'établissement un PDF rédigé ailleurs** (cachet, registre, vérification « authenticité seule ») | Une part réelle de ce que signe une direction ne sort d'aucune nature ; sans cela, signature scannée collée dans Word | important | 7 | DCT-139, R-D12 |
| M-254 | **Circuit de signature sur un document importé** (convention imposée par l'entreprise, contrat d'un partenaire) : page de signatures ajoutée, empreinte de la version importée | Les grandes entreprises et les administrations imposent souvent leur modèle [H] ; DCT-72 n'importe que des documents **déjà** signés | important | 7, 39 | DCT-138, R-D12 |
| M-255 | **Délai d'annulation** de quelques minutes après une signature par lot ou un lot de mises au coffre, avant toute notification | L'erreur de masse se voit juste après ; une fois notifiée, elle oblige à 40 annulations et 40 messages | différenciant | 7 | DCT-133, R-D15, Q-D37 |
| M-256 | **Qualité exacte du signataire** (titre accordé, intérim, délégation) lue au socle 6, jamais saisie | « Le Proviseur » sous la signature d'une proviseure ; un intérim non mentionné rend la signature contestable [NV] | confort | 7, 6 | R-D24 |
| M-257 | **Synthèse avant signature par lot** : anomalies détectées par les données de la nature, documents écartés et pourquoi, échantillon | DCT-25 « écarte les documents incomplets » sans dire ce que la direction voit ; 120 PDF ne se relisent pas | confort | 7 | R-D27 |
| M-258 | **Actes soumis à transmission** (actes du conseil d'administration transmis à l'autorité académique et à la collectivité) [NV] | Production et suivi de la transmission absents de toute conception | confort | 19, 9 | renvoyé (§5) |

### P04 — Hélène, comptable de l'organisme de gestion (ensemble privé avec apprentissage)

**Année.** *Septembre* : factures annuelles aux familles ; trois familles paient par un tiers (centre communal d'action sociale, employeur, service de l'aide sociale à l'enfance) ; deux parents séparés paient chacun la moitié. *Octobre* : une facture comporte une erreur de tarif : il faut un **avoir** et une nouvelle facture, pas une « nouvelle version ». *Novembre* : l'unité de formation par apprentissage facture les opérateurs de compétences ; ceux-ci exigent un certificat de réalisation et, bientôt, des factures électroniques structurées [NV : calendrier de la réforme de la facturation électronique entre professionnels, à vérifier]. *Janvier* : nouveaux contrats de travail (surveillants, agents) signés par le président de l'organisme, bénévole, qui n'est pas un personnel. *Mars* : reçus de dons à l'organisme ou à sa fondation [NV : régime et modèle à vérifier]. *Toute l'année* : relances, mises en demeure en recommandé.
**Incidents.** Les certificats de scolarité portaient le nom et le numéro de l'organisme au lieu de l'établissement ; les factures l'inverse. Un couple marié reçoit deux enveloppes identiques, deux affranchissements.

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-259 | **Correction par document compensatoire** (avoir) pour les natures financières ; numéro de facture jamais remplacé | §0 point E : appliquer la « nouvelle version » à une facture serait irrégulier [NV] | **bloquant** | 7, 24 | R-D2, Q-D43 |
| M-260 | **Entité émettrice** par famille de natures (établissement, organisme de gestion, organisme de formation), avec identité et séries propres | §0 point F : deux personnes morales, deux identités légales sur les documents | important | 7 (+5, 24, 49) | DCT-164, R-D13, Q-D35 |
| M-261 | **Payeur tiers organisation** : facture envoyée à l'adresse professionnelle du CCAS, de l'employeur, du service de l'ASE, par lien | Le coffre suppose une personne avec compte ; une organisation n'en a pas | important | 7, 24 | DCT-129 |
| M-262 | **Facture électronique structurée** (PDF/A-3 avec données intégrées) pour les destinataires professionnels | Le moteur (D1) ne vise que PDF/A-2 ; la réforme concerne au moins la facturation aux opérateurs de compétences et aux entreprises [NV] | important | 7, 24 | R-D20, Q-D39 |
| M-263 | **Signataire de l'organisme hors personnel** (président, trésorier, délégataire) avec un accès restreint au parapheur | Le contrat de travail est signé par l'employeur, pas par le chef d'établissement [NV] ; le président n'a pas de compte | important | 7, 6, 49 | DCT-163 |
| M-264 | **Reçu de don** de l'organisme ou de sa fondation, numéroté, au modèle réglementaire [NV] | Fonction de gestion courante dans le privé [H], absente | confort | 24, 48 | §4.1 bis |
| M-265 | **Un courrier par adresse** : un seul pli nominatif pour les titulaires qui vivent ensemble ; deux plis seulement si les adresses ou les droits diffèrent | §9.9 impose « un courrier par titulaire » : double affranchissement pour la majorité des familles | confort | 7 | R-D1 |

### P05 — Marc et Céline, parents séparés (autorité conjointe, résidence chez la mère)

**Année.** *Septembre* : Céline signe en ligne le règlement intérieur pour Lou ; elle doit refaire la même chose pour Tom, à l'école de l'autre établissement, et elle a deux coffres dans deux espaces. *Janvier* : voyage en Allemagne : l'autorisation de sortie du territoire et l'engagement financier demandent **les deux** titulaires [NV : acte non usuel]. Marc n'a jamais activé son compte ; il a donné son portable au secrétariat en 2024, vérifié. *Février* : Marc affirme n'avoir « jamais reçu » la demande ; Céline accuse le collège de favoriser le père. *Septembre suivant* : la rentrée demande, pour chaque enfant, cinq signatures, trois pièces, une fiche et deux formulaires, chacun dans un écran différent.
**Incidents.** Céline demande au guichet : « Vous avez besoin de la signature du père ? » La secrétaire ne sait pas dire pourquoi pour ce document et pas pour l'autre.

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-266 | **Lien personnel sans compte** pour un titulaire : consultation, signature S1 par code au canal vérifié, dépôt d'une pièce ; invitation à activer le compte | §0 point C : sans lui, toute règle « tous les titulaires » bloque dès qu'un parent n'a pas de compte | **bloquant** | 7, 4 | DCT-137, R-D8, Q-D31 |
| M-267 | **Historique des invitations et relances** d'un signataire (canal masqué, date, résultat d'envoi) au dossier de preuve | « Je n'ai jamais reçu » est la contestation la plus fréquente [H] ; le dossier de preuve ne trace que les signatures | important | 7, 8 | DCT-143 |
| M-268 | **Explication de la règle d'accord** à la famille et au guichet (« un seul titulaire suffit pour cet acte, parce que… ») | La règle vient de la nature ; personne ne sait l'expliquer, et le parent soupçonne un parti pris | confort | 7 | R-D28 |
| M-269 | **Signer en un geste un même document pour plusieurs enfants** (un acte, plusieurs circuits) | Trois enfants = trois fois le même règlement ; abandon en cours de route [H] | confort | 7 | DCT-141, R-D14 |
| M-270 | **Coffre unifié** des enfants scolarisés dans plusieurs établissements ScolIA (affichage seulement, cloisonnement conservé) | D-013 donne une identité unique ; deux coffres séparés en font perdre l'intérêt | différenciant | 7 (+11, 29) | DCT-158 |
| M-271 | **Parcours de rentrée groupé** : documents à signer, pièces, fiche de rentrée et formulaires de l'enfant dans une seule démarche guidée | C'est le premier contact de l'année avec le logiciel ; dispersé en cinq écrans, il produit des oublis et des appels | important | 7 (+4, 18, 29) | DCT-159 |

### P06 — Mme Kovalenko et M. Diallo, familles non francophones

**Année.** *Rentrée* : Mme Kovalenko reçoit au coffre une demande de pièces et une convocation à un entretien avec le professeur principal ; elle ne comprend pas le courrier, ne vient pas. *Novembre* : M. Diallo est convoqué pour une punition de son fils ; il ne lit pas le français ; un médiateur traduit au guichet ; il signe l'accusé sur la tablette. *Décembre* : l'acte de naissance des enfants est en ukrainien ; on lui demande une « traduction assermentée » qu'elle ne peut pas payer. *Février* : la famille est hébergée chez une compatriote : elle n'a pas de justificatif de domicile à son nom.
**Incidents.** Un an plus tard, la famille Diallo conteste avoir compris ce qu'elle a signé : rien au dossier de preuve ne dit qu'un interprète était présent.

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-272 | **Notice traduite** du texte fixe d'une nature (convocation, demande de pièces, courrier disciplinaire, documents à signer) dans la langue de la famille ; le français fait foi | Une convocation incomprise, c'est un rendez-vous manqué et une procédure fragile ; §9.4 renvoie les langues autres que l'anglais à « plus tard » | important | 7 | DCT-134, R-D17, Q-D34 |
| M-273 | **Assistance à la signature** au dossier de preuve : interprète (nom, langue), lecture à voix haute, personne de confiance | Sans cette mention, le consentement d'une personne qui ne lit pas le français est contestable [NV] | important | 7 | DCT-142 |
| M-274 | **Version simple** (facile à lire et à comprendre) des documents fixes les plus fréquents | Familles peu lettrées, parents en situation de handicap ; rien sur le marché [H] | différenciant | 7 | DCT-134 |
| M-275 | **Pièces en langue étrangère** acceptées sans traduction assermentée par défaut ; noms translittérés tels que l'état civil les porte | Exiger une traduction payante bloque une inscription sans nécessité [NV] | confort | 7, 2 | R-D29 |
| M-276 | **Attestation d'hébergement et domiciliation** (types de pièces), pièce d'une personne tierce (l'hébergeant), et règle : l'absence de justificatif de domicile ne bloque jamais une inscription dans le public [NV] | Familles hébergées, sans domicile, en hôtel social : cas fréquents en éducation prioritaire [H] | important | 7, 3, 4 | R-D22, R-D23, §11.2 |
| M-277 | **Espace famille dans la langue de la famille** (libellés de l'interface, pas les documents) | Même une notice traduite ne sert à rien si le bouton « Signer » n'est pas compris | confort | 11, 29 | renvoyé (§5) |

### P07 — Inès, 19 ans, BTS en apprentissage

**Année.** *Septembre* : attestation d'inscription pour le CROUS et pour l'aide au logement ; certificat pour son employeur. *Octobre* : elle demande que ses parents ne reçoivent plus rien (RP1-12). *Tous les mois* : l'opérateur de compétences demande, pour payer le centre de formation, un **certificat de réalisation** et les émargements des séances [NV] ; l'employeur veut une attestation de présence. *Mars* : avenant au contrat d'apprentissage, signé par l'employeur, l'apprentie et le centre [NV : formulaire réglementaire]. *Juin* : elle obtient son BTS et part ; deux ans plus tard, un employeur lui demande une attestation de formation.

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-278 | **Certificat de réalisation et attestation de présence** pour l'opérateur de compétences et l'employeur, envoyés à l'organisation | Le financement de l'apprentissage en dépend [NV] ; Charlemagne a un module pour les centres de formation (`ENT2_CFA_*`, matrice §1) | important | 39, 24, 7 | §4.1 bis, DCT-129 |
| M-279 | **Émargement électronique par séance**, signé par l'apprenant et le formateur, exportable pour un contrôle | Exigé pour la formation financée [NV] ; aussi pour les réunions et examens | important | 12, 39 (+7) | renvoyé (§5), §4.1 bis |
| M-280 | **Contrat et avenant d'apprentissage** (formulaire réglementaire importé) signés par l'employeur, l'apprenti et le centre | Document à plusieurs parties dont le modèle n'est pas à nous | confort | 39, 7 | DCT-138 |
| M-281 | **Export automatique du coffre au départ** (archive des bulletins, attestations, documents signés) remis à l'élève ou aux titulaires | Q-D17 donne un an d'accès puis « export proposé » ; un départ en juin ne pense pas à exporter | confort | 7 | DCT-168, R-D30 |

### P08 — M. Lambert, professeur référent des périodes en entreprise (lycée professionnel)

**Année.** *Septembre* : 30 élèves × 3 périodes = 90 conventions. *Octobre* : un hôpital impose sa propre convention (M-254). *Novembre* : deux élèves mineurs vont utiliser des machines dangereuses : il faut la déclaration de dérogation aux travaux réglementés et l'avis médical d'aptitude [NV]. *Décembre* : une élève part en Espagne (mobilité européenne) : convention et engagement de formation en anglais. *Janvier* : l'assurance de responsabilité civile d'une entreprise expire pendant la période. *Mai* : les attestations de période signées par les tuteurs doivent être au dossier d'examen [NV].

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-282 | **Pièces d'un mineur en atelier** : déclaration de dérogation aux travaux réglementés, avis médical d'aptitude [NV] (types de pièces, domaine adapté) | Sans elles, un mineur sur machine est en situation irrégulière [NV] ; le catalogue §11.2 ne les connaît pas | important | 39, 7 | §11.2 [personas] |
| M-283 | **Convention de mobilité internationale** bilingue (engagement de formation) | §9.4 limite le bilingue aux certificats et attestations | confort | 39, 7 | §4.1 bis |
| M-284 | **Alerte d'expiration** d'une pièce à date (assurance de l'entreprise, titre d'un agent, habilitation) avant l'échéance | DCT-56 ne traite que le renouvellement annuel ; une assurance qui expire en cours de stage passe inaperçue | confort | 7 | DCT-146 |
| M-285 | **Attestations de période requises pour l'examen** : tableau de ce qui manque par candidat, à la date limite du dossier d'examen | Une attestation manquante peut empêcher la validation [NV] | important | 38, 39 | renvoyé (§5) |

### P09 — Julien (contractuel, arrivé le 13 novembre) et Aïcha (surveillante salariée, de nationalité étrangère)

**Année.** *Julien* : la veille de sa prise de fonctions, il reçoit un lien pour déposer ses pièces (diplôme, relevé d'identité bancaire pour la paie au socle 49, attestation de sécurité sociale) ; il n'a pas encore de compte ; le jour même il signe la charte et le procès-verbal d'installation [NV]. *Aïcha* : son contrat de travail est signé par le président de l'organisme (M-263) ; l'employeur doit vérifier son autorisation de travail et en garder copie [NV, à vérifier] ; son titre expire en mars. *Tous les mois* : bulletins de paie mis à disposition ; une collègue refuse le format électronique [NV : droit d'opposition à vérifier]. *Juin* : fin du contrat d'Aïcha : certificat de travail, solde de tout compte, attestation pour l'assurance chômage [NV].
**Incidents.** La secrétaire de scolarité, en cherchant le dossier d'un élève homonyme, ouvre le dossier RH d'Aïcha et y lit son titre de séjour.

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-286 | **Domaine RH séparé** : dossiers des personnels lus seulement avec `documents.personnels.lire` ; idem pour les pièces financières des bourses | §0 point A : la portée « établissement » de `documents.lire` ouvre les dossiers RH à tout le secrétariat | **bloquant** | 7, 6 | R-D6, §18.3 |
| M-287 | **Pièces exigées par le droit du travail** (autorisation de travail, numéro de sécurité sociale sans copie de carte, honorabilité) dans un catalogue RH, malgré les interdits du §11.2 | §0 point H : l'interdit, écrit pour les familles, empêcherait l'employeur de remplir une obligation [NV] | important | 7, 6, 49 | R-D5 |
| M-288 | **Bulletins de paie au coffre de l'agent** : mise à disposition, disponibilité longue après le départ, opposition au format électronique [NV] | Le coffre « un an après le départ » (Q-D17) ne convient pas à un document que l'employeur doit tenir disponible très longtemps [NV] | important | 49, 7 | DCT-162, R-D25, Q-D40 |
| M-289 | **Natures de fin de contrat** (droit privé) : certificat de travail, reçu pour solde de tout compte, attestation employeur [NV] | Obligatoires à la fin d'un contrat [NV] ; le §4.1 ne cite que l'attestation d'emploi et l'état de service | important | 6, 49 | §4.1 bis |
| M-290 | **Espace sans compte pour un arrivant** (dépôt de pièces, signature de la charte) avant l'activation de son compte | PRS-117 prévoit un lien d'accueil ; le socle 7 n'a de lien sans compte que pour les signataires d'organisations | important | 7, 6 | DCT-137 |

### P10 — M. Fabre (inspecteur), Mme Ndiaye (auditrice qualité), le commissaire aux comptes

**Année.** *Mars* : contrôle d'un établissement hors contrat [NV : contenu du contrôle à vérifier] : M. Fabre demande le registre des inscriptions, les attestations d'assurance, les diplômes des enseignants, les documents remis aux familles. *Mai* : audit qualité de l'apprentissage : émargements, certificats de réalisation, conventions, réclamations des apprenants [NV]. *Juillet* : le commissaire aux comptes vérifie la continuité de la numérotation des factures et des avoirs.
**Incidents.** L'auditrice veut vérifier un document de 2027 dont le certificat du cachet a expiré : le lecteur PDF affiche « signature non valide ».

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-291 | **Accès temporaire d'un contrôleur** à une sélection de documents (lecture seule, filigrane, journal, expiration), jamais au dossier entier | Aujourd'hui une clé USB ou un compte prêté ; PRS-125 ouvre l'accès d'inspection sans dire ce qu'il voit des documents | important | 7, 6 | DCT-148, R-D19 |
| M-292 | **Dossier de contrôle** exporté (sélection, index, empreintes, preuves) | Même besoin que le contentieux (M-251) ; le registre seul ne suffit pas | important | 7 | DCT-150 |
| M-293 | **Preuve à long terme** : ancrage périodique des empreintes horodaté, cachet prolongeable, re-scellement avant l'expiration des certificats ou des algorithmes [H] | Un cachet non prolongé devient « non valide » quelques années après l'émission [H] ; le registre matricule se garde des décennies [NV] | important | 7 | DCT-155, R-D16 |

### P11 — Mme Roux (archiviste départementale) et Bernard (adjoint gestionnaire)

**Année.** *Février* : Bernard prépare la proposition d'élimination annuelle ; 80 % des archives du lycée sont **papier** (dossiers d'élèves avant ScolIA, registres, conventions signées à la main, pièces comptables), rangées en salle d'archives. *Mars* : Mme Roux demande un bordereau unique (papier et numérique), et pour une catégorie « à trier », de garder un échantillon selon une consigne (par exemple une année sur dix, ou les dossiers de certaines lettres) [NV : consigne de tri du service]. *Juin* : un lycée voisin ferme et ses archives sont transférées.

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-294 | **Inventaire des archives papier** (cote, localisation, contenu, catégorie, années, sort final) inclus dans les lots et les bordereaux | Sans lui, deux registres et deux bordereaux ; les originaux papier gardés au §10.6 et §14.3 ne sont localisés nulle part | important | 7 | DCT-152 |
| M-295 | **Tri par échantillonnage** : consigne définie pour une catégorie « trier », appliquée au lot | Le sort « trier » existe sans règle ; l'archiviste devrait trier à la main dans l'export | confort | 7 | DCT-153 |
| M-296 | **Transfert des archives** d'un établissement qui ferme ou fusionne (vers l'établissement repreneur ou le service d'archives) | Cas réel des restructurations [H] ; la réversibilité (D55) ne vise que le départ de ScolIA | confort | 7 | DCT-154 |

### P12 — Fatou, agent d'accueil (lycée public)

**Année.** *Chaque jour* : des familles déposent des papiers ; d'autres appellent pour un certificat ; des parents viennent chercher un document à remettre contre signature. *Octobre* : une grand-mère vient chercher le certificat de radiation de son petit-fils avec un mot de sa fille. *Novembre* : un père lui tend un jugement à scanner.
**Incidents.** Après avoir scanné le jugement, Fatou peut le relire dans l'historique de ses dépôts.

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-297 | **Dépôt aveugle** : le déposant voit qu'il a déposé (date, type) mais ne relit le contenu que s'il a le droit de lecture du domaine | L'accueil a `documents.deposer` sans avoir à lire les jugements qu'il numérise | important | 7 | R-D7 |
| M-298 | **Saisir une demande de document pour le compte d'une famille** (téléphone, guichet) | Exemple de D-023 : l'accueil qui transmet au lieu du tableur partagé ; DCT-17 n'est ouvert qu'aux familles | confort | 7 | DCT-136 |
| M-299 | **Remise à un tiers mandaté** : procuration écrite déposée, identité de la personne vue, remise contre signature | Fréquent pour les familles qui travaillent [H] ; DCT-89 ne connaît que le titulaire | confort | 7 | DCT-166 |
| M-300 | **Deux refus « illisible »** → proposer le dépôt au guichet ou la numérisation par l'accueil | La famille renvoie trois fois la même photo floue (D15) ; personne ne propose une autre voie | confort | 7 | R-D38 |

### P13 — Mme Perret, directrice d'école (déchargée un jour par semaine)

**Année.** *Juin* : douze certificats de radiation et transmissions de dossiers, dont quatre vers des écoles hors ScolIA. *Septembre* : elle reçoit de la mairie le certificat d'inscription de chaque nouvel élève [NV : dans le public, l'inscription est faite par la mairie]. *Toute l'année* : elle émet **et** signe ; aucun parapheur n'a de sens pour elle.

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-301 | **Émettre et signer en un geste** quand l'émetteur est le signataire habilité (pas de parapheur pour soi) | Sans cela, la directrice s'envoie ses propres documents au parapheur | confort | 7 | R-D32 |
| M-302 | **Certificat d'inscription délivré par la mairie** (type de pièce reçue, premier degré public) | Pièce obligatoire de l'admission en école publique [NV] ; absente du catalogue | confort | 7, 50 | §11.2 [personas] |

### P14 — Léo, 15 ans, élève de 3ᵉ

**Année.** *Septembre* : il prend connaissance du règlement et de la charte (S0, seul). *Décembre* : stage d'observation : convention co-signée par lui, ses parents, l'entreprise et le collège. *Mars* : il est convoqué à un entretien avec la CPE ; il voudrait voir la convocation dans son espace. *Juin* : il a perdu son attestation de stage.

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-303 | **Ce que voit l'élève mineur** dans son espace : documents qui le concernent directement (convocations, règlement, charte, attestations, convention), jamais les documents financiers, de santé ni de justice | Q-D21 ne traite que le libre-service du certificat ; le contenu du coffre du mineur n'est pas défini | confort | 7, 2 | R-D33 |

### P15 — Clara (infirmière) et le Dr Morel (médecin qui suit l'élève)

**Année.** *Septembre* : quinze PAI à **reconduire** à l'identique ; chaque reconduction refait tout le circuit. *Toute l'année* : des familles déposent des certificats médicaux pour justifier des absences.

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-304 | **Reconduire un document signé** (PAI, autorisation annuelle) : nouvelle version marquée « inchangé », circuit proposé avec les mêmes signataires | Le renouvellement annuel est le cas majoritaire [H] ; refaire tout décourage | confort | 7, 34 | DCT-167 |
| M-305 | **Justificatif d'absence** : type de pièce distinct, certificat médical non exigé hors cas réglementaires [NV], motif médical jamais lisible hors du service qui en a besoin | Sans type propre, les certificats médicaux d'absence entrent en C2 ordinaire au dossier | important | 7, 12 | R-D21, §11.2 |

### P16 — M. Petit, délégué à la protection des données (huit établissements)

**Année.** *Septembre* : il valide les règles de conservation de huit établissements, catégorie par catégorie. *Novembre* : un père demande l'accès à tous les documents concernant sa fille ; plusieurs, déposés par la mère, portent l'adresse protégée de celle-ci. *Février* : une réquisition de police demande la liste des documents d'un élève. *Avril* : un lot de bulletins est parti au mauvais parent ; il doit savoir **exactement** qui a ouvert quoi pour notifier la violation [NV : délai de 72 heures].

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-306 | **Occultation** (caviardage) d'une version de communication, l'original intact | Communiquer un dossier à un parent, à un contrôleur, à un juge sans révéler les données d'un tiers [NV] ; aucun outil sur le marché [H] | important | 7, 27 | DCT-147 |
| M-307 | **Valider le référentiel proposé en un écran** (liste explicite, validation globale ou par catégorie) | Huit établissements × quatorze catégories ; la validation une à une décourage | confort | 7, 27 | R-D35 |
| M-308 | **Réquisition ou demande d'une autorité** enregistrée (base invoquée, demandeur, décision, documents transmis) | D70 dit « action humaine » sans trace structurée | confort | 7, 27 | DCT-165 |
| M-309 | **Rapport de violation** : pour un lot ou un document envoyé à tort, la liste exacte des ouvertures et téléchargements | Indispensable pour notifier dans les délais [NV] ; DCT-128 propose, ne rapporte pas | important | 7, 27 | DCT-132 |

### P17 — Thomas (ancien élève, bac 2011) et Mme Benoît (ancienne surveillante)

**Année.** Thomas prépare son dossier de retraite et demande une attestation de scolarité de ses années de BTS [NV : usage pour un rachat de trimestres]. Son compte ScolIA n'existe plus. Mme Benoît a perdu son certificat de travail de 2028.

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-310 | **Vérifier l'identité d'un ancien élève sans compte** : modes admis (au guichet, par courrier avec pièce vue puis non conservée, par la récupération D-018 si l'identité ScolIA existe) | DCT-19 dit « identité vérifiée » sans dire comment ; à distance, le secrétariat demande une copie de pièce d'identité qu'il garde | confort | 7 | R-D36 |

### P18 — Les tiers vérificateurs (CAF, consulat, préfecture, employeur)

**Situations.** La préfecture demande les certificats de scolarité des enfants pour le titre de séjour d'un parent ; un consulat exige un **original signé à la main**, légalisé ou apostillé [NV : l'apostille porte sur une signature d'autorité connue] ; un employeur étranger lit la page de vérification sans parler français.

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-311 | **Original papier signé à la main** (version « papier original », signature manuscrite réelle constatée, registre) | Certaines autorités étrangères refusent un document électronique [H] ; la conception ne prévoit que la signature visuelle numérisée | confort | 7 | DCT-135 |
| M-312 | **Page de vérification en anglais** (et dans les langues des notices) | Le QR est fait pour des tiers, souvent étrangers pour les documents bilingues | confort | 7 | R-D37 |

### P19 — Nadia (éducatrice de l'ASE) et Mme Robert (assistante familiale)

**Année.** L'enfant est confié au service ; les parents gardent l'autorité mais l'adresse de l'enfant ne leur est pas communiquée. Trois éducateurs se succèdent dans l'année. L'assistante familiale demande un certificat et signe l'autorisation de sortie au musée [NV : actes usuels délégués selon le projet pour l'enfant].

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-313 | **Documents adressés au service** (adresse fonctionnelle de l'organisme), pas à l'éducateur du moment | Chaque changement d'éducateur casse la chaîne ; les documents restent dans le coffre de quelqu'un qui est parti | important | 7, 4 | DCT-129, R-D9 |
| M-314 | **Délégation d'actes usuels** à l'assistante familiale (signature des autorisations courantes, réception des certificats) déclarée par pièce | Sans elle, chaque sortie scolaire passe par le service [NV] | confort | 4, 7 | R-D8 (note), renvoyé (§5) |

### P20 — Mme Lefort, mère protégée (adresse protégée)

**Situations.** Le père a l'autorité ; il reçoit les bulletins. La CPE écrit au père un courrier simple qui mentionne « depuis votre déménagement à Annecy, Mme Lefort… ». Un lot de convocations est préparé pendant que l'ordonnance est déposée.

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-315 | **Contrôle des textes libres** contre les éléments de l'adresse protégée (ville, établissement d'origine, employeur) | D7 protège l'adresse des champs du document, pas ce qu'un agent écrit dans un texte libre | important | 7, 4 | DCT-157, R-D11 |

### P21 — Kenza (victime), les parents de l'élève mis en cause, les membres du conseil de discipline

Repris dans P02 (M-248, M-249). **Ajout propre** : après la séance, les membres élus gardent sur leur téléphone le dossier téléchargé.

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-316 | **Lecture sans téléchargement** pour les accès temporaires (aperçu rendu par le serveur, filigrane nominatif, aucun fichier remis) | Un dossier disciplinaire d'un mineur circulerait sans fin parmi des parents et des élèves élus | important | 7, 13 | DCT-148, R-D19 |

### P22 — Yann, référent numérique et administrateur

**Incidents.** *Rentrée* : 300 certificats émis en lot avec l'année scolaire précédente : il faut tout annuler et réémettre, prévenir les familles qui les ont ouverts. *Le 2 septembre à 8 h* : la connexion du lycée tombe ; dix familles attendent un certificat au guichet. *Le 3* : le service du cachet est indisponible une heure ; l'émission est bloquée (Q-D7) et les familles en libre-service voient une erreur.

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-317 | **Annuler et réémettre en lot** avec un motif unique, rapport des ouvertures, message proposé | DCT-13 : « ce qui est émis reste émis, annulable un par un » : 300 annulations à la main | important | 7 | DCT-132 |
| M-318 | **Émission de secours** (numéros de secours réservés, papier) puis régularisation au registre | Une panne de rentrée sans solution renvoie au tampon et au cahier, sans trace | confort | 7 | DCT-160 |
| M-319 | **File d'émission** quand le cachet est indisponible : la demande est enregistrée, l'émission se fait au rétablissement, le demandeur est prévenu | Le blocage strict de Q-D7 est juste, l'erreur affichée ne l'est pas | confort | 7 | DCT-161 |

### P23 — M. Weber, directeur d'un établissement hors contrat international

**Année.** Relevés de notes en anglais et en allemand ; diplômes maison ; dossiers d'admission vers des universités étrangères qui exigent un original papier ; mentions de l'établissement (« établissement privé hors contrat ») [NV].

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-320 | **Langues des documents au-delà du français et de l'anglais** pour les natures bilingues (allemand, espagnol, au moins) | Clientèle internationale ; §9.4 renvoie à « plus tard » sans critère | confort | 7 | R-D17 (note), Q-D34 |

### P24 — Mme Laurent, cheffe d'établissement coordinatrice (ensemble à deux UAI)

**Situations.** L'école et le collège ont chacun leur UAI ; l'organisme de gestion est commun ; une série de numéros « par établissement » (D43) ne dit pas si l'école et le collège partagent la série.

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-321 | **Séries de numéros par entité émettrice** (établissement ou organisme), et identité du document selon l'UAI de scolarisation | Deux UAI, une série : numérotation ambiguë pour un contrôleur ; deux séries pour l'organisme : factures non continues [NV] | important | 7, 5 | R-D13 |

### Manques transverses (plusieurs personas)

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-322 | **Contrat IA des actions ajoutées** (lectures, commandes, fermées) | Critère de fin de lot (D-025) | important | 7 | §20.5 |
| M-323 | **Événements consommés manquants** : sanction annulée ou réformée (13), inscription annulée (3), majorité proche (2, ELV-39), bulletin de paie émis (49) | §19.2 incomplet ; un consommateur absent n'échoue pas, il ne fait rien | confort | 7 | §19.3 |
| M-324 | **Natures de l'orientation** : fiche de dialogue signée par la famille, notification de décision, appel [NV] | Absentes du catalogue §4.1 ; signées chaque année par toutes les familles de 3ᵉ et de 2ᵈᵉ [NV] | important | 37, 23, 7 | §4.1 bis |
| M-325 | **Convocations aux épreuves en cours de formation** (lycée professionnel, BTS) avec accusé, et constat d'absence [NV] | Contrôles certificatifs organisés par l'établissement : convocation et preuve exigées [NV] | important | 38, 7 | §4.1 bis |
| M-326 | **Procès-verbal de conseil de classe** et **procès-verbal d'instance** (conseil de discipline, conseil d'administration) signés par le président et le secrétaire de séance | Natures absentes ; signées sur papier aujourd'hui | confort | 23, 19, 13, 7 | §4.1 bis |
| M-327 | **Confirmation d'inscription à un examen national** reçue, signée par le candidat ou la famille, rangée au coffre [NV] | Pièce retournée chaque année par tous les candidats [NV] | confort | 38, 7 | §11.2 [personas] |
| M-328 | **Carnet de correspondance imprimé** (Charlemagne) ou numérique avec signature | Fonction attendue (COM-07 de la matrice) ; le socle 7 fournit la signature S0 | confort | 16, 7 | renvoyé (§5) |
| M-329 | **Pièces d'un tiers** (l'hébergeant, le nouveau conjoint qui paie) : portée « personne tierce », jamais une personne du dossier, conservation courte | Le modèle n'a que trois portées (personne, foyer, élève) | confort | 7, 4 | R-D23 |
| M-330 | **Natures déclarées bilingues étendues** aux conventions de mobilité et aux relevés des établissements internationaux | Voir M-283, M-320 | confort | 7 | §9.4 [personas] |
| M-331 | **Un seul signataire par défaut** : par famille de pouvoir (DCT-98) **ou** par nature (`signataire_defaut_agent_id`) | Deux réglages pour la même chose ; contradiction du modèle | confort | 7 | S-48 |
| M-332 | **Lecture du dossier de preuve par les deux parents** quand un seul a signé (l'autre titulaire veut savoir qui a signé quoi) | La conception le rend téléchargeable « par chaque signataire » : le parent qui n'a pas signé n'y a pas accès | confort | 7 | R-D31 |
| M-333 | **Rapprochement des réglages proposés par cette passe** (délais, plafonds, langues) en valeurs fixes au départ | D-023 : chaque réglage justifie son existence | confort | 7 | S-55 |

---

## 4. Couverture documentaire de Charlemagne (D-023, D-024)

Le contenu exact des « éditions » de Charlemagne n'a pas pu être lu [H, même limite que la matrice §0]. La liste ci-dessous est celle qu'une secrétaire de scolarité, une CPE et une comptable attendent d'un logiciel de ce type ; « conçu » ne veut pas dire fait.

| Fonction attendue [H] | Avant la passe | Après la passe | Où |
|---|---|---|---|
| Certificat de scolarité, attestations, exeat | conçu | conçu, rendu **neutre** (R-D3), radiation jamais conditionnée (R-D4) | §4.1, D3 |
| Publipostage de courriers types (Word dans Charlemagne) | **remplacé** par les natures (D-023) ; aucun courrier local | **conçu** : courrier type de l'établissement, contenu seulement (DCT-131) | §14.14 |
| Courriers de vie scolaire et de discipline | conçu (socle 13) | conçu, avec notification effective, visa, contrôle de contenu, consultation du dossier | R-D10, DCT-140, 148, 157 |
| Envoi groupé d'e-mails avec pièces jointes | remplacé par le coffre | complété par l'envoi par lien pour qui n'a pas de coffre (DCT-129) | §14.14 |
| Factures, relances, attestations de paiement | conçu (socle 24 déclare) | conçu, avec avoir (R-D2), entité émettrice (R-D13), payeur tiers, facture structurée | §4.1 bis |
| Bulletins, relevés, livrets | conçu (socle 23 déclare) | inchangé | — |
| Conventions de stage (module « Entreprise ») | conçu | conçu, avec modèle imposé par l'entreprise (DCT-138), pièces des mineurs en atelier, mobilité | §11.2, §4.1 bis |
| Centres de formation d'apprentis (`ENT2_CFA_*`) | **absent** | certificats de réalisation, attestations de présence, contrats importés (§4.1 bis) ; émargement renvoyé au socle 12 | §4.1 bis, §5 |
| Taxe d'apprentissage (`ENT2_TAXE*`) | absent | **renvoyé** (socle 24) ; le régime actuel ne demande peut-être plus de reçu de l'établissement [NV] | §5 |
| Étiquettes, listes, trombinoscopes | conçu (DCT-10, 21 ; STR-111) | inchangé | — |
| GED : numérisation et rattachement à la fiche | conçu | conçu, avec dépôt spontané, dépôt aveugle, e-mails entrants | DCT-144, 151, R-D7 |
| Registre des entrées et sorties (registre matricule) | socle 2 | inchangé | — |
| Paie : bulletins, certificat de travail, attestations de fin de contrat | socle 49 (carte) | natures déclarées, coffre de l'agent, disponibilité longue | §4.1 bis, DCT-162 |
| Carnet de correspondance | socle 16 (carte) | renvoyé | §5 |
| Orientation (fiche de dialogue) | absent du socle 7 | natures déclarées par le socle 37 | §4.1 bis |
| Convocations aux examens blancs et épreuves en cours de formation | socle 38 (carte) | natures déclarées par le socle 38 | §4.1 bis |
| Procès-verbaux de conseils | absent | natures déclarées par les socles 23, 19, 13 | §4.1 bis |
| Signature de documents (École Directe) | conçu | conçu, avec parent sans compte (DCT-137), plusieurs enfants, assistance, historique des invitations | §14.14 |
| Archives papier, tri | absent (aucun logiciel du marché [H]) | conçu | DCT-152, 153 |

---

## 5. Manques qui relèvent d'autres socles

| Socle | Manques | Ce qu'il faudra prévoir |
|---|---|---|
| 4 Dossier administratif | M-266, M-313, M-314 | Canal vérifié de chaque titulaire lu par le lien personnel ; adresse fonctionnelle d'un organisme gardien ; délégation d'actes usuels avec pièce |
| 6 Personnels | M-263, M-286, M-287, M-290 | Signataire de l'organisme hors personnel ; secrétariat de direction comme modèle de rôle lecteur RH ; catalogue RH ; lien d'accueil branché sur DCT-137 |
| 11, 29 Mobile, portails | M-270, M-271, M-277 | Coffre unifié multi-établissements ; parcours de rentrée ; interface dans la langue de la famille |
| 12 Appel | M-279, M-305 | Émargement électronique par séance ; justificatifs d'absence |
| 13 Discipline | M-246, M-248, M-249, M-250, M-252 | Modes de notification valables par nature ; contenu des courriers sans nom de tiers ; consultation du dossier ; publication de l'annulation ou de la réforme ; nature de la mesure de responsabilisation |
| 16 Messagerie | M-328 | Carnet de correspondance |
| 19 Réunions et instances | M-258, M-326 | Actes du conseil d'administration et leur transmission ; procès-verbaux |
| 23 Bulletins | M-326 | Procès-verbal de conseil de classe |
| 24 Facturation | M-241, M-259, M-261, M-262, M-264 | Attestation de situation de compte ; avoir ; payeurs organisations ; facture structurée ; reçus de dons |
| 27 RGPD | M-306, M-307, M-308, M-309 | Occultation, validation du référentiel, réquisitions, rapport de violation (le socle 7 fournit les outils documentaires) ; demandes de communication de documents administratifs dans le public [NV] |
| 37 Orientation | M-324 | Fiche de dialogue et décisions |
| 38 Examens | M-285, M-325, M-327 | Attestations de période requises ; convocations aux épreuves en cours de formation ; confirmations d'inscription |
| 39 Stages | M-254, M-278, M-280, M-282, M-283 | Modèles imposés ; certificats de réalisation ; contrats d'apprentissage ; travaux réglementés ; mobilité |
| 49 Paie | M-288, M-289 | Bulletins, fin de contrat |
| 50 Premier degré | M-302 | Certificat d'inscription de la mairie |

---

## 6. Synthèse

### 6.1 Manques par gravité

| Gravité | Nombre |
|---|---|
| bloquant | 5 |
| important | 46 |
| confort | 42 |
| différenciant | 4 |
| **Total** | **97** (M-237 à M-333) |

Comptage fait par script sur les tableaux du §3. Les cinq bloquants correspondent aux points A à E du §0 : **M-286** (dossiers RH et pièces financières ouverts au secrétariat), **M-240** (certificat qui révèle un dispositif), **M-266** (titulaire sans compte qui ne peut pas signer), **M-246** (délai de procédure calculé depuis l'émission), **M-259** (facture « réémise » au lieu d'un avoir). Les points F, G et H du §0 sont **importants** (M-260 et M-321, M-237, M-287).

Les quatre différenciants : M-243 (e-mails entrants au registre), M-255 (délai d'annulation après une signature par lot), M-270 (coffre unifié multi-établissements), M-274 (version simple des documents).

**93** manques sont traités par un ajout dans `07_DOCUMENTS.md` (dont M-279 et M-314 en partie, le reste renvoyé) ; **4** sont renvoyés en entier à un autre socle (M-258, M-277, M-285, M-328). Les natures déclarées par d'autres socles (§4.1 bis de la conception : orientation, épreuves en cours de formation, procès-verbaux, apprentissage, fin de contrat) sont comptées comme ajouts : le socle 7 les rend, leur socle les déclare.

### 6.2 Ajouts dans `07_DOCUMENTS.md`

| Partie | Ajouts |
|---|---|
| Actions métier | **40** (DCT-129 à DCT-168) |
| Règles | **38** (R-D1 à R-D38) |
| Cas limites | **30** (D71 à D100) |
| Natures complémentaires | §4.1 bis (23 natures, déclarées par leur socle) |
| Types de pièces | §11.2 [personas] (12 types) |
| Modèle | §8.14 (domaine, entité émettrice, envois, invitations, visas, accès temporaires, courriers types, archives papier, ancrage, occultation, notification, assistance) |
| Droits | §18.3 (9 actions) |
| Événements | §19.3 (8 émis, 6 consommés) |
| Contrat IA | §20.5 |
| Questions | 14 (Q-D31 à Q-D44) |
| Réglages signalés | 12 (S-44 à S-55), §15.3 |

### 6.3 Les dix ajouts les plus importants

1. **Domaines restreints et dossiers des personnels séparés** (R-D6, §18.3 ; M-286). Sans eux, la première version ouvre les dossiers RH et les avis d'imposition à tout le secrétariat.
2. **Certificat neutre** (R-D3 ; M-240). La première nature codée ne doit jamais révéler une SEGPA, une ULIS ou une UPE2A à la CAF ou au club de sport.
3. **Lien personnel sans compte pour les titulaires et les arrivants** (DCT-137, R-D8 ; M-266, M-290). Le parent séparé qui n'a pas de compte peut signer ; les règles « tous les titulaires » deviennent praticables.
4. **Notification effective par destinataire** (R-D10, DCT-156 ; M-246). Les délais de la procédure disciplinaire partent du bon jour ; le socle 13 le lit au lieu de le deviner.
5. **Correction par avoir et entité émettrice** (R-D2, R-D13, DCT-164 ; M-259, M-260, M-321). Les factures restent régulières ; l'organisme de gestion et l'établissement ont chacun leur identité et leurs séries.
6. **Envoi par lien sécurisé** (DCT-129, R-D9 ; M-237, M-242, M-261, M-313). Remplace la pièce jointe pour les familles sans compte, les payeurs, les opérateurs de compétences, l'établissement d'accueil, le service gardien.
7. **Documents importés à signer et PDF libres signés par la direction** (DCT-138, DCT-139, R-D12 ; M-253, M-254). La convention imposée par l'entreprise, le courrier rédigé ailleurs : sans eux, la signature scannée revient.
8. **Visa interne, synthèse et délai d'annulation au parapheur** (DCT-140, DCT-133, R-D15, R-D27 ; M-247, M-255, M-257). Le circuit réel d'une direction, et le droit à l'erreur juste après une signature par lot.
9. **Accès temporaires, occultation et dossier de contrôle ou de contentieux** (DCT-147, 148, 150 ; M-249, M-291, M-292, M-306, M-316). Communiquer ce qu'il faut, à qui il faut, sans que le dossier circule.
10. **Notice traduite, version simple et assistance à la signature** (DCT-134, DCT-142, R-D17 ; M-272, M-273, M-274). Les familles non francophones comprennent ce qu'on leur demande ; le dossier de preuve dit qu'un interprète était là.

Suivent de près : le courrier type de l'établissement (DCT-131, M-239, à trancher : Q-D32), le formulaire tiers (DCT-130), le dépôt spontané et le dépôt aveugle (DCT-144, R-D7), l'annulation en lot avec rapport de violation (DCT-132), la preuve à long terme (DCT-155), l'inventaire des archives papier (DCT-152), les bulletins de paie au coffre (DCT-162), les natures de l'orientation et des épreuves en cours de formation (§4.1 bis).

### 6.4 Ce que la passe a montré sur la conception

- **La conception est solide sur ce qu'elle produit** (version figée, numéro, cachet, vérification, niveaux de signature, quarantaine) ; ses manques sont **à ses bords** : la personne sans compte, l'organisation destinataire, le document qui n'est pas à nous, le papier qui reste, la seconde personne morale.
- **Elle a été pensée pour l'élève, moins pour l'agent** : les dossiers des personnels partagent les droits des dossiers d'élèves, et le catalogue des interdits, juste pour les familles, contredit le droit du travail.
- **Un principe général rencontre une règle de métier** : « corriger = nouvelle version » est juste pour un certificat, faux pour une facture ; « un courrier par titulaire » est juste pour des parents séparés, coûteux pour un couple.
- **La preuve ne suffit pas, il faut la date** : le dossier de preuve dit qui a signé ; la procédure a besoin de savoir **quand** chacun a été notifié et **par quel moyen**.
- **D-023 tient, à une exception près** : le catalogue de natures ne couvrira jamais toutes les lettres récurrentes d'un établissement. Un courrier type de contenu seulement (sans mise en forme) est le prix pour que Word ne revienne pas : à trancher (Q-D32).

---

## 6 bis. Réglages signalés et documents prêts (D-023)

### 6 bis.1 Réglages candidats à l'allègement (signalés, non supprimés)

Même lecture que `PERSONAS_05-06.md` §6 bis.1 : « version automatique » = ce que fait le logiciel quand l'établissement ne touche à rien. Les réglages S-D1 à S-D12 de la conception (§15.2) restent tels quels ; la liste ci-dessous les complète. **Ne sont pas signalés** : les données de l'établissement (types de pièces activés, signataires, entités émettrices).

| # | Réglage | Où | Pourquoi il pèse | Version automatique proposée |
|---|---|---|---|---|
| S-44 | **Courriers types de l'établissement** (DCT-131) | §14.14 | Risque de retour à l'éditeur que D-023 refuse : des dizaines de lettres mal écrites, jamais revues | Plafond de 30 actifs, catalogue de 12 champs, revue annuelle proposée ; une nature livrée prime toujours |
| S-45 | Dérogation de style par nature (`style_code` de `ent_doc_nature_param`) | §8.2 | Déjà signalé S-D3 ; la passe confirme : aucune persona ne l'a demandé | Retirer |
| S-46 | Activation par établissement de **chaque** type de pièce (`ent_doc_type_piece_param.actif`) | §8.6 | Une liste à cocher que personne ne revoit | Tous les types actifs ; seul « autre » est surveillé ; les types santé suivent le module |
| S-47 | Langues proposées (`ent_doc_identite.langues`) | §8.2 | Un établissement ne sait pas d'avance quelles langues ses familles parleront | Toutes les langues livrées proposées ; la langue vient de la fiche de la personne |
| S-48 | Signataire par défaut **par nature** en plus de **par famille de pouvoir** | §8.2 / DCT-98 | Deux réglages pour la même chose, qui finiront par se contredire (M-331) | Par famille de pouvoir seulement |
| S-49 | Niveau de signature « monté » par nature (DCT-99) | §14.7 | Personne ne le règle en pratique [H] | Garder (il protège), mais masqué dans un écran avancé |
| S-50 | Durée des liens d'envoi, nombre de téléchargements, durée des accès temporaires | DCT-129, DCT-148 | Trois nombres sans enjeu local | Valeurs fixes : 7 jours, 3 téléchargements ; accès temporaire jusqu'à la fin de la séance + 2 jours |
| S-51 | Délai d'annulation après un lot | DCT-133 | Paramètre de confort | 10 minutes fixes |
| S-52 | Règles de mentions (dispositifs masqués, niveau affiché) | R-D3 | Tentation de rendre la règle réglable | **Pas de réglage** : la règle protège |
| S-53 | Validation des règles de conservation **catégorie par catégorie** | §13.1 | Écran long pour un DPO de plusieurs établissements | Validation globale du référentiel proposé, avec la liste explicite ; exceptions par catégorie |
| S-54 | Modes de notification valables par nature | R-D10 | Ne doit pas devenir un réglage d'établissement | Déclarés par la nature (code), jamais par l'établissement |
| S-55 | **Mes ajouts** : seuil de détection des noms à l'aperçu, nombre de refus « illisible » avant proposition du guichet, délai de l'alerte d'expiration, plafond de l'export automatique au départ | R-D11, R-D38, DCT-146, DCT-168 | Paramètres ajoutés par cette passe | **Valeurs fixes** au départ ; un réglage n'apparaît que si un pilote le demande |

### 6 bis.2 Documents : 3 ou 4 trames prêtes par type (D-023)

La conception applique déjà la règle par ses quatre styles déclinés pour chaque nature (§9.3). Les natures ajoutées par cette passe (§4.1 bis de la conception) entrent dans les mêmes familles de structure, **sans nouvelle mise en page** : attestation (attestation de situation de compte, certificat de réalisation, attestation de présence, certificat de travail, reçu de don, page d'attestation d'un formulaire tiers) ; lettre (notification de décision d'orientation, courrier type) ; convocation (épreuve en cours de formation) ; relevé (dossier de contrôle : index) ; contrat (convention de mobilité, convention de mesure de responsabilisation, fiche de dialogue) ; liste (procès-verbaux : feuille de présence) ; une famille nouvelle : **page de signatures ajoutée** à un document importé (R-D12).

---

## 7. Faits, hypothèses, contradictions

**Faits [F]** vérifiés dans la conception : `categorie_sante` et `categorie_justice` seuls domaines restreints (`07` §8.3) ; `documents.lire` en portée « établissement » pour le modèle Secrétariat et DCT-30 ouvert à ADF (§14.3, §18.2) ; signataire externe défini comme « personne majeure qui agit pour une organisation, jamais un élève » (§17.7) ; principe 4 « correction = nouvelle version » sans exception (§1) ; « un courrier par titulaire » (§9.9) ; interdits du catalogue sans distinction de public (§11.2) ; `ent_doc_identite` clé primaire `etablissement_id` (§8.2) ; signataire par défaut à la fois par nature (§8.2) et par famille de pouvoir (DCT-98) ; aucun mode de notification déclaré dans `NatureDecl` (§5.3) ; §19.2 sans événement d'annulation de sanction ; DCT-13 « annulable un par un » ; dossier de preuve téléchargeable « par chaque signataire » (§10.8) ; PDF/A-2 visé au lot D1 (§23).

**Hypothèses [H]** : la vie des personas, les fréquences (demandes par e-mail, conventions imposées par les entreprises, seconds parents sans compte, reconduction des PAI), le contenu des éditions de Charlemagne, les pratiques des autres logiciels.

**À vérifier [NV]** : délai et forme de la procédure contradictoire et de la convocation au conseil de discipline, point de départ des délais ; procédure d'appel des sanctions ; régularité de la rétention d'un certificat de radiation pour impayé ; règles de facturation (avoir, numérotation continue) et calendrier de la facture électronique entre professionnels ; certificat de réalisation et émargements de l'apprentissage ; contrat d'apprentissage ; obligations de l'employeur sur l'autorisation de travail d'un salarié étranger ; numéro de sécurité sociale pour la paie ; bulletins de paie électroniques, opposition et durée de disponibilité ; certificat de travail, reçu pour solde de tout compte, attestation employeur ; dérogation aux travaux réglementés des mineurs et avis médical ; attestations de période et dossier d'examen ; épreuves en cours de formation ; fiche de dialogue d'orientation ; confirmation d'inscription aux examens ; certificat d'inscription de la mairie dans le premier degré public ; inscription sans justificatif de domicile ; justificatifs d'absence et certificat médical ; langue des documents administratifs (loi n° 94-665) ; traduction assermentée ; apostille et légalisation ; actes usuels délégués à l'assistant familial ; notification d'une violation de données (72 heures) ; demandes de communication de documents administratifs ; actes du conseil d'administration ; reçus de dons ; régime de la taxe d'apprentissage ; contrôle des établissements hors contrat ; validité à long terme des cachets électroniques.

**Contradictions signalées** (toutes notées **[personas]** dans la conception, aucune tranchée) : domaines restreints contre dossiers des personnels (§0 A) ; principe 4 contre les factures (§0 E) ; signataire externe contre titulaire sans compte (§0 C) ; interdits du catalogue contre le droit du travail (§0 H) ; « un courrier par titulaire » contre le coût postal (R-D1) ; identité unique contre deux personnes morales (§0 F) ; signataire par défaut à deux niveaux (S-48) ; Q-D13 (jamais de pièce jointe) tenu, mais sans solution pour qui n'a pas de compte (§0 G). Les ajouts de modèle demandent des numéros de migration : ils entrent dans les migrations déjà prévues (3701, 3704, 3705, 3707, 3708) ou dans la réserve 3713–3789 ; **à attribuer dans `MIGRATIONS.md`** au moment du code (non fait ici, aucune migration n'est écrite).
