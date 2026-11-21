# Passe « personas » sur le socle N1 — Demandes et flux de travail

**Date :** 2026-11-13 — **Statut :** proposition, **non décidée** (à relire par Florian) ; les recommandations s'appliquent par défaut (D-031), sauf ce qui engage l'établissement à l'extérieur (§8.2). Aucun code écrit ni modifié ; aucun accès à une base, à un secret ni à un service de production. Le code Next.js a été **lu seulement** (rien exécuté).
**Branche :** `ent-socle-N1-conception`, partie de `origin/ent-mc0hoe` (`699275d`).
**Éprouve :** `docs/socles/N1_DEMANDES.md` (§0 à §35), avec `CLAUDE.md`, `docs/DECISIONS.md` (D-031, D-036, D-051, D-057), `docs/PLAN_BASCULE_FEVRIER.md` (M04, M08) ; forme reprise de `PERSONAS_19.md`.
**Ajouts :** faits **directement** dans `N1_DEMANDES.md`, au **§36** (« ajout personas »), qui l'emporte en cas de contradiction ; rien n'est supprimé.
**Numérotation :** manques **MN1-001 à MN1-099** ; actions **DEM-360 à DEM-383** ; règles **R-DM47 à R-DM52** ; cas limites **CDM-57 à CDM-72** ; questions **Q-DM14 et suivantes**.

---

## 0. À lire en premier : trois bloquants et leur recommandation (appliquée par défaut, D-031)

| # | Point | Pourquoi c'est bloquant | Recommandation (appliquée par défaut) | Lots |
|---|---|---|---|---|
| **A** | **La personne visée lit la demande qui la vise.** Une salariée dépose une demande RH (C3) pour se plaindre du comportement de la gestionnaire RH ; la file RH a **un seul membre** : la gestionnaire. Même chose pour une question à la direction qui concerne le chef d'établissement, ou une demande de comptabilité qui conteste un remboursement fait par la comptable (P06). | La confidentialité par file (§14) protège contre les autres services, pas contre le service lui-même. Une salariée ne déposera plus rien ; pire, elle le fera et sera exposée. | **Exclusion pour conflit d'intérêts** (R-DM47) : sur les natures C1+, le demandeur peut indiquer « cette demande concerne une personne de ce service » (choix dans la liste des membres, sans motif) ; la personne est **exclue** de la demande (lecture, prise, supervision, escalade) ; si la file devient vide, la demande va à une **file de repli hors service** désignée pour chaque file confidentielle (organisme de gestion : président ou vice-président ; public : chef d'établissement, ou, s'il est lui-même visé, message « adressez-vous à l'autorité académique » et aucune demande créée). | N1-1, N1-2 |
| **B** | **Le guichet devient un canal de harcèlement.** Un parent dépose quinze demandes en une journée, avec des insultes, au secrétariat et à la comptabilité ; la limitation de débit (NJ-72) arrête un robot, pas une personne (P11). | Personnels exposés sans protection, alors que la messagerie (16 CM-19, MSG-226) a une restriction motivée ; l'information due à la famille doit continuer. | **Restriction de dépôt** (R-DM48), symétrique de celle du 16 : la direction peut, par décision motivée, datée et réexaminée, limiter une personne à N demandes par semaine ou au **dépôt par l'accueil seulement** ; les demandes en cours restent traitées ; les informations dues continuent ; la personne voit « vos demandes passent par l'accueil jusqu'au … » sans motif détaillé. | N1-2 |
| **C** | **La bascule envoie une avalanche.** Le jeudi d'ouverture, chaque demande ouverte reprise envoie « votre demande est désormais suivie dans ScolIA » (§29.2), et chaque file reçoit au même moment son résumé de « nouvelles demandes » (toutes les reprises) ; un personnel qui a 12 demandes reçoit 12 avis (P12). | Première impression de ScolIA ratée ; inondation des boîtes ; résumés faux (« 140 nouvelles demandes »). | **Avis de reprise unique** (R-DM49) : un seul avis par personne, regroupé, envoyé **le jour d'ouverture à 8 h** (pas pendant la reprise) ; les demandes reprises n'entrent **pas** dans les résumés « nouvelles » ; aucun avis à un demandeur non rapproché ; un bandeau dans « Mes demandes » explique la reprise. | N1-5 |

**Défauts Next.js nouveaux** relevés par la passe : aucun au-delà de NJ-80 à NJ-95 ; la passe confirme la gravité de **NJ-80** (commentaire par tout compte : parfait outil d'hameçonnage depuis l'adresse de l'établissement) et de **NJ-81** (une AED pourrait reprendre une demande RH déjà prise).

---

## 1. Méthode

Douze personas réalistes, prises dans le tableau des acteurs (§4 de la conception), couvrant le **privé sous contrat** (groupe scolaire pilote de Florian, plusieurs sites et cycles, organisme de gestion) et le **public** (EPLE), les familles ordinaires et les cas particuliers, un **attaquant** (trois variantes) et un **persona de bascule** (Florian). Pour chacune : sa journée ou son année, ce qu'elle fait aujourd'hui dans le Next.js, ses irritants, peurs et ce qui la ravirait, puis les **manques** et **contradictions** de la conception.

**Grille transversale** : (1) **parité** avec le Next.js (M04, M08) ; (2) **minimisation** : qu'apprend-elle qu'elle ne devrait pas, y compris par recoupement ? (3) **D-036** : ce qu'elle saisit est-il une colonne typée ou une table ? (4) le **mobile** ; (5) ce que l'**IA** prépare sans décider ; (6) la **bascule de février**.

**Gravité** : **bloquant** · **important** · **confort** · **différenciant**.

---

## 2. Les personas en un tableau

| # | Persona | Établissement | Rôle dans N1 | Ce qu'elle éprouve |
|---|---|---|---|---|
| P01 | Mme Lefèvre, secrétaire de direction | groupe privé, 3 sites | membre de la file de tri, du secrétariat, de la file « direction » | tri de rentrée, qualification, transferts, réponses types |
| P02 | M. Dubois, agent de maintenance | groupe privé, 2 sites | seul membre de la file maintenance | mobile, urgences, doublons, prestataires, absence |
| P03 | Mme Roche, professeure de lettres | groupe privé, 2 sites | demandeuse (reprographie, HSE, assistance) | copies à 22 h, copies récurrentes, atelier théâtre |
| P04 | M. Garnier, directeur adjoint | lycée privé | décideur et attestant HSE, valideur reprographie couleur | enveloppe, équité, service fait |
| P05 | Mme Benoît, comptable de l'organisme de gestion | groupe privé | file comptabilité | pièces exigées, remboursement de frais, contrôle |
| P06 | Mme Hamon, gestionnaire RH ; Mme Kaci, salariée | organisme de gestion | file RH confidentielle ; demandeuse | conflit d'intérêts, confidentialité |
| P07 | Mme Martin et M. Diallo, parents séparés (interdiction de contact) ; Mme Lopez, accueil | collège | demandeurs ; déposante pour le compte | séparation, téléphone, attestation |
| P08 | Hugo, 18 ans, terminale | lycée | demandeur majeur | relevé de notes pour Parcoursup, urgence, parents |
| P09 | Léna, 12 ans, 6ᵉ | collège | demandeuse mineure | carte d'élève perdue, parents payeurs |
| P10 | M. Petit, principal d'un collège public | EPLE | chef d'établissement | CRPA, retards, escalade, signature |
| P11 | Les attaquants | — | — | agent curieux, élève qui devine, parent qui harcèle |
| P12 | Florian, administrateur, semaine de la bascule | groupe privé | administrateur | reprise, paramétrage, avis |

---

## 3. Audit de parité avec Next.js

| Fonction Next.js (NXN) | Retrouvée dans la conception ? | Écart |
|---|---|---|
| NXN-01 dépôt personnel avec pièces | oui (DEM-104, DEM-105, DEM-110) | — |
| NXN-02 dépôt sans compte | **non repris** (D-051) ; DEM-106, DEM-319 | à expliquer aux personnels qui l'utilisaient pour des externes (prestataires ?) → **MN1-001** |
| NXN-03 page parents | §19 (comptes) | Q-DM11 : usage réel |
| NXN-04 routage IA | déterministe ; IA en proposition (N1-6) | à la bascule, plus de devinette par le texte : la **qualité du catalogue** compte (→ MN1-002) |
| NXN-05 indicateur direction | R-DM14, DEM-160 | — |
| NXN-06 corbeille | file de tri (§10.4) | tri **en masse** manquant (→ MN1-010) |
| NXN-07 file partagée | DEM-170, DEM-171 | — |
| NXN-08 relâcher, renvoyer, réassigner | DEM-172, DEM-176, DEM-177 | — |
| NXN-09 délégation | DEM-174 | — |
| NXN-10 transmission direction | DEM-178 | — |
| NXN-11 statuts et commentaires | §11.1, §12 | — |
| NXN-12 supervision globale et par métier | DEM-181 | — |
| NXN-13 organigramme paramétrable | §10.2 sur les missions du 6 | conversion des adresses en missions à préparer **avant** la bascule (→ MN1-090) |
| NXN-14 file RH forcée | `rh.demande_agent` | — |
| NXN-15 avis aux étapes | §16 | — |
| NXN-16 purge à 30 jours | conservation par nature | — (amélioration) |
| NXN-17 assistance | `numerique.assistance` | — |
| NXN-18 HSE | §17 | plus exigeant (service fait) : charge nouvelle pour l'adjoint (→ MN1-040) |
| NXN-19 photocopies | §18 | copies récurrentes et multi-classes (→ MN1-020, MN1-021) |

**Conclusion** : parité fonctionnelle atteinte ; trois écarts sont **voulus** (pas de dépôt public, pas de routage par l'IA à la bascule, service fait des HSE) et doivent être **annoncés** aux personnels avant février (→ MN1-091).

---

## 4. Parcours et manques, persona par persona

### P01 — Mme Lefèvre, secrétaire de direction d'un groupe scolaire privé (école, collège, lycée ; trois sites)

**Sa journée.** 8 h : la file de tri a 23 demandes du week-end (rentrée de septembre). Elle en requalifie la moitié (« assistance » qui est une panne, « question à la direction » qui est une demande d'attestation). 10 h : un parent au téléphone. 14 h : le chef lui demande « où en est la demande de M. X ? ». Aujourd'hui, dans le Next.js, elle travaille dans la corbeille, qu'elle partage avec tout le personnel.
**Irritants** : refaire le même geste 23 fois ; réécrire la même réponse ; ne pas savoir si le chef a déjà vu une demande « direction ».
**Peurs** : envoyer une demande RH à la mauvaise personne ; être « la corbeille » de l'établissement.
**Ravie si** : trier en masse, répondre avec un modèle, voir la file direction avec « vue par le chef le … ».

| Manque | Gravité | Traitement |
|---|---|---|
| MN1-010 | important | **Transfert et requalification en masse** depuis la file de tri : sélection, même file cible ou même nature, aperçu, un fait par demande → **DEM-372** |
| MN1-011 | important | « Mettre en attente de la direction » **sans transmettre** : la demande reste au secrétariat, le chef a une tâche de lecture → **DEM-373** |
| MN1-012 | confort | Réponses types **par nature** (pas seulement par file) : déjà possible par file ; préciser qu'un modèle peut être restreint à une nature → précision §36.5 |
| MN1-013 | important | Signature des attestations : le chef délègue la signature des attestations au secrétariat (habilitation du 6) ; la conception exige `peut_signer` sans dire comment la secrétaire l'obtient → **DEM-378**, CDM-68 |
| MN1-014 | confort | « Qui a déjà vu cette demande ? » dans la file (lecture interne) : visible des membres de la file seulement → précision §36.5 |

### P02 — M. Dubois, agent de maintenance, seul pour deux sites

**Sa journée.** Il ouvre sa file sur le téléphone à 7 h 30, dans la cour. Douze demandes : quatre ampoules, deux radiateurs, une fuite, des stores. Il travaille avec des gants. Aujourd'hui, dans le Next.js, il reçoit un e-mail par demande, avec toute la description.
**Irritants** : les demandes floues (« ça ne marche pas en B12 »), les doublons, les allers-retours entre bâtiments.
**Peurs** : passer à côté d'une fuite parmi les ampoules ; qu'on lui reproche une demande qui relève de la collectivité ou d'un prestataire.
**Ravi si** : sa journée est groupée par bâtiment, il clôt avec une photo, il dit « c'est pour le chauffagiste » en un geste.

| Manque | Gravité | Traitement |
|---|---|---|
| MN1-030 | important | **Vue « ma tournée »** : demandes regroupées par site, bâtiment et étage, urgences en tête → **DEM-369** |
| MN1-031 | important | **Photo avant / après** à la clôture d'une intervention (preuve pour le demandeur et la direction) → **DEM-370** |
| MN1-032 | important | « **Hors de notre ressort** » : prestataire sous contrat, collectivité (public), organisme de gestion : issue `transmise_hors_scolia` avec destinataire codé et texte au demandeur ; l'envoi qualifié vient avec le volet du 44 → **DEM-371**, CDM-64 |
| MN1-033 | **important** | **Seul membre absent une semaine** : la file de repli par défaut (secrétariat de direction) ne sait pas réparer ; il faut une file de repli **métier** (intendance, qui appelle un prestataire) → règle déjà permise (file de repli au choix) ; **préréglage** : la file de repli de la maintenance est l'intendance → §36.5, CDM-72 |
| MN1-034 | confort | Saisie vocale de la note de clôture (mobile) → N1-7 (transcription selon le 11) |

### P03 — Mme Roche, professeure de lettres sur deux sites, atelier théâtre

**Sa semaine.** Dimanche 22 h : elle dépose les copies de la semaine pour ses quatre classes (mêmes documents, effectifs différents). Mercredi : elle demande 10 HSE pour préparer la représentation de juin. Elle a oublié son mot de passe de l'ENT pendant les vacances.
**Irritants** : refaire chaque semaine la même demande de copies ; quatre demandes pour quatre classes ; savoir si c'est prêt.
**Peurs** : arriver en classe sans copies ; une HSE refusée sans explication.
**Ravie si** : une demande pour ses quatre classes, « refaire comme la semaine dernière », « prête » sur le téléphone.

| Manque | Gravité | Traitement |
|---|---|---|
| MN1-020 | important | **Une demande, plusieurs classes** : exemplaires par cible (déjà `ent_dem_repro_cible`) et **lieu de retrait par cible** (casier de la classe ou salle des professeurs) → **DEM-368** |
| MN1-021 | important | **Dupliquer** une demande (reprographie, fournitures) : nouvelles valeurs pré-remplies, **sans** reprendre les pièces d'un autre ni les valeurs sensibles → **DEM-366**, R-DM52 |
| MN1-022 | confort | **Demande récurrente** : un brouillon préparé chaque semaine et un rappel, **jamais** un dépôt automatique (les PDF changent) → **DEM-367**, R-DM52 |
| MN1-023 | important | HSE d'un **atelier sans séance à l'EDT** et sans élèves inscrits au 10 : comment l'adjoint atteste-t-il ? → **DEM-374**, **DEM-375**, R-DM50 |
| MN1-024 | confort | Assistance « mot de passe oublié » : la conception renvoie au parcours de récupération (R-DM3) ; préciser qu'un personnel sans accès dépose **par un collègue ou l'accueil** → précision §36.5 |
| MN1-025 | différenciant | Effectifs pré-remplis **moins les élèves absents de longue durée** (stage, hospitalisation) : non, l'effectif reste celui de la classe (pas de donnée de santé déduite) → refus justifié, §36.5 |

### P04 — M. Garnier, directeur adjoint d'un lycée privé : HSE et reprographie couleur

**Son année.** Septembre : il ne connaît pas encore l'enveloppe d'HSE (notifiée en octobre). Il reçoit 40 demandes d'HSE au premier trimestre. Chaque fin de mois, il atteste. Il valide aussi la reprographie couleur. Dans le Next.js, il accepte par e-mail ; il n'atteste rien.
**Irritants** : attester à l'aveugle ; tenir un tableur pour l'enveloppe ; arbitrer sans voir ce qu'un collègue a déjà eu.
**Peurs** : dépasser l'enveloppe ; attester des heures non faites ; être accusé de favoritisme.
**Ravi si** : il voit le prévisionnel et le consommé, il atteste par lot à partir de l'appel, il voit la répartition par discipline.

| Manque | Gravité | Traitement |
|---|---|---|
| MN1-040 | **important** | **Attestation fondée sur un indice** : séance créée au 10 avec les élèves → l'appel (12) pré-coche la séance ; sans élèves (préparation, réunion de projet) → **déclaration sur l'honneur** de l'enseignant et contrôle par **échantillon** ; l'attestation en masse **sans indice** exige conf. 2 → **DEM-374**, **DEM-375**, R-DM50, CDM-65, CDM-66 |
| MN1-041 | important | **Enveloppe prévisionnelle** avant notification (mouvement `prevision`, remplacé par la dotation) → **DEM-376**, CDM-67 |
| MN1-042 | important | **Équité** : répartition des HSE accordées par agent sur l'année, visible du seul décideur et du chef (pas des collègues) → **DEM-377** |
| MN1-043 | confort | Décider plusieurs HSE en une fois (même motif, même volume) → action groupée (DEM-190) étendue aux HSE, §36.5 |
| MN1-044 | confort | Validation couleur depuis le téléphone (carte d'action de l'à-faire) → déjà `dem.repro_valider` ; N1-7 |

### P05 — Mme Benoît, comptable de l'organisme de gestion

**Son mois.** Remboursements de frais des personnels (déplacements de sortie, achats avancés), demandes de facture des familles, demandes d'achat. Aujourd'hui : e-mails et tickets de caisse photographiés.
**Irritants** : les demandes sans pièce ; les montants faux ; les relances.
**Peurs** : rembourser deux fois ; un contrôle sans pièce.
**Ravie si** : pièces exigées dès le dépôt, montant typé, seuil de validation par la direction, export pour la compta.

| Manque | Gravité | Traitement |
|---|---|---|
| MN1-050 | important | **Pièces exigées par type** dans `comptabilite.demande_interne` (remboursement de frais : justificatif obligatoire) : déjà permis par la déclaration (`Pieces::Exigees`) ; à écrire dans le catalogue → §36.5 |
| MN1-051 | important | **Validation par la direction au-delà d'un seuil** pour un remboursement de frais (circuit `AvecValidation`), et R-DM40 (pas sa propre demande) → précision §36.5 |
| MN1-052 | important | **Doublon de remboursement** (même montant, même date, même personne) signalé → doublon §9.5 étendu aux montants, §36.5 |
| MN1-053 | confort | Export des demandes traitées du mois (montants, références) pour la saisie comptable en attendant le 48 → DEM-340 précisé |
| MN1-054 | — | Coordonnées bancaires d'un personnel : **jamais** dans une demande comptable (champ refusé) ; la demande de changement de RIB est `rh.demande_agent` et le RIB est tenu par le 25/49 → précision §36.5 |

### P06 — Mme Hamon, gestionnaire RH de l'organisme de gestion ; Mme Kaci, salariée

**La situation.** Mme Kaci veut se plaindre du comportement de Mme Hamon et demander une attestation employeur. La file RH (C3) n'a qu'un membre : Mme Hamon. Dans un petit établissement, la secrétaire de direction cumule souvent la RH.
**Peurs (Mme Kaci)** : que la personne visée lise sa plainte. **(Mme Hamon)** : être accusée sans trace.

| Manque | Gravité | Traitement |
|---|---|---|
| MN1-060 | **bloquant** | **Conflit d'intérêts** dans la file : bloquant A → **DEM-360**, **DEM-361**, R-DM47, CDM-57, CDM-58, CDM-59 |
| MN1-061 | important | Une **plainte** n'est pas une demande RH ordinaire : rappeler les voies (référent harcèlement de l'employeur, CSE, inspection du travail) sur la nature `rh.demande_agent` quand le motif « conflit, harcèlement » est choisi, et ne pas forcer le dépôt dans ScolIA → texte d'aide de la nature, §36.5 |
| MN1-062 | important | Cumul RH + direction : la supervision de la direction ne doit pas lui donner le contenu RH (R-DM15 tient) ; quand la même personne est membre de la file RH **et** directrice, c'est sa qualité de membre qui ouvre la lecture (tracée) → précision §36.5 |

### P07 — Mme Martin et M. Diallo, parents séparés (interdiction de contact) ; Mme Lopez, accueil

**La situation.** Mme Martin demande une attestation de scolarité pour la CAF depuis son compte. M. Diallo appelle l'accueil : « Je veux savoir ce que mon ex a demandé, c'est pour mon fils aussi. » Mme Lopez dépose aussi, pour une grand-mère sans numérique, une demande de duplicata.

| Manque | Gravité | Traitement |
|---|---|---|
| MN1-070 | **important** | L'accueil, en déposant pour le compte, voit-il les **demandes existantes** de la famille (doublons, §9.5) ? Oui dans la conception → fuite : M. Diallo apprendrait l'existence d'une demande de Mme Martin. **Correction** : la recherche de l'accueil ne montre **aucune** demande existante d'une autre personne ; le doublon n'est signalé qu'à la file → **DEM-380**, R-DM51, CDM-69 |
| MN1-071 | important | **Appelant non vérifié** : avant de déposer pour un appelant, l'accueil propose de **rappeler au numéro connu** (comme NTF-147 du 8) ; il ne confirme rien au téléphone → **DEM-379**, R-DM51 |
| MN1-072 | important | La grand-mère n'est pas une responsable : le dépôt pour son compte n'est possible que si elle est **personne de confiance** autorisée (socle 4) et pour les natures ouvertes à ce lien ; sinon le dépôt se fait au nom d'un responsable avec son accord → précision §36.5 |
| MN1-073 | confort | Accusé imprimé : ne porte ni l'adresse ni le nom de l'autre parent (R-DM17 tient) → test |

### P08 — Hugo, 18 ans, terminale

Il veut un **relevé de notes** pour une candidature hors Parcoursup qui ferme dans trois jours ; il ne veut pas que ses parents voient ses démarches.

| Manque | Gravité | Traitement |
|---|---|---|
| MN1-080 | important | **Urgence datée** pour une famille ou un élève (« j'en ai besoin avant le … ») : champ « pour le » sur les natures familles, qui ordonne la file sans changer le délai d'engagement → **§36.5** (urgence `haute` bornée par la nature), CDM-71 |
| MN1-081 | confort | « Le bulletin est déjà dans vos documents » (raccourci R-DM3) avant de déposer un duplicata → déjà DEM-312 ; test |
| MN1-082 | — | Ses parents ne voient pas sa demande (R-DM34) : tient |

### P09 — Léna, 12 ans, 6ᵉ : carte d'élève perdue

Elle n'a pas de photo à joindre ; la carte coûte 5 € ; le compte de sa mère n'est pas activé.

| Manque | Gravité | Traitement |
|---|---|---|
| MN1-085 | important | Dépôt **sans pièce** par un élève mineur : la vie scolaire complète (photo prise au bureau) → **DEM-381** |
| MN1-086 | important | **Paiement** avant remise : lien vers le 25 pour le payeur ; si aucun payeur actif, la vie scolaire décide (remise sans paiement ou attente) → **DEM-382**, CDM-70 |
| MN1-087 | confort | « Tes parents verront cette demande » (R-DM33) : tient |

### P10 — M. Petit, principal d'un collège public

Il veut tenir les règles du CRPA pour les demandes des familles, ne pas être inondé, voir ce qui est en retard, déléguer la signature des attestations à sa secrétaire.

| Manque | Gravité | Traitement |
|---|---|---|
| MN1-095 | important | **Décisions implicites à venir** sur 30 jours (EPLE) dans son tableau → **DEM-383** |
| MN1-096 | important | Délégation de signature (MN1-013) : même besoin → DEM-378 |
| MN1-097 | important | Demande d'une famille qui **le vise lui-même** (contestation de sa décision) : pas une demande N1 (recours : rectorat, tribunal administratif) ; la nature `direction.question` rappelle les voies de recours → CDM-58 |
| MN1-098 | confort | Résumé hebdomadaire de direction (volumes, retards, décisions implicites) plutôt que des avis → §36.5 |

### P11 — Les attaquants

| Variante | Tentative | Ce que fait la conception |
|---|---|---|
| **Agent curieux** (AED) | ouvre `/demandes/<uuid>` d'une demande RH trouvée dans un historique de navigateur ; tente de « prendre » une demande déjà prise d'une autre file | 404 neutre (R-DM10) ; prise refusée (NJ-81) ; tentative journalisée |
| **Élève qui devine** | essaie des numéros lisibles `2026-00001…` | les numéros lisibles ne sont **pas** des clés d'accès : l'adresse d'une demande est son UUID, et l'accès est contrôlé ; « Mes demandes » ne liste que les siennes |
| **Parent qui harcèle** | quinze demandes par jour avec insultes | **bloquant B** : restriction de dépôt (R-DM48, **DEM-362**, **DEM-363**, CDM-60) |
| **Compte compromis** | dépôts en masse, messages d'hameçonnage aux familles | limitation de débit ; un personnel ne peut écrire qu'aux demandeurs **de ses files** (R-DM10) ; aucun message vers une adresse libre (R-DM36) |

| Manque | Gravité | Traitement |
|---|---|---|
| MN1-099 | **bloquant** | restriction de dépôt (bloquant B) |
| MN1-001 | confort | Les personnels qui faisaient déposer des **externes** (prestataire, intervenant) par le formulaire public : dépôt pour le compte par le personnel qui les reçoit (DEM-106) ; à expliquer (MN1-091) |
| MN1-002 | important | Sans IA à la bascule, la recherche du catalogue repose sur les **mots-clés** : jeu d'essai de 200 phrases (objets anonymisés) pour la régler (§4, transverses) |

### P12 — Florian, administrateur, semaine de la bascule (20 février – 8 mars 2027)

Samedi 20 : lecture seule du Next.js ; reprise ; contrôle. Jeudi 25 : ouverture aux personnels.

| Manque | Gravité | Traitement |
|---|---|---|
| MN1-090 | **important** | La conversion des **adresses e-mail** de l'organigramme Next.js en **missions** du 6 demande un travail humain **avant** la bascule (sinon files vides au jour J) → rapport « adresses non retrouvées » dès la répétition de Noël ; files « à désignation provisoire » (source `agent`, datée) acceptées pour la bascule → §36.5 |
| MN1-091 | important | **Annoncer** aux personnels les trois changements voulus (pas de dépôt public, pas de routage par l'IA, service fait des HSE) : note préparée par N1-5 → §36.5 |
| MN1-092 | **bloquant** | Avalanche d'avis à l'ouverture : **bloquant C** → **DEM-364**, R-DM49, CDM-61 |
| MN1-093 | important | Pendant la lecture seule (5 jours), une demande urgente (fuite) n'a pas de canal : message d'indisponibilité avec consigne (« appelez l'accueil ») sur le Next.js — geste de Florian — et sur ScolIA avant ouverture → **DEM-365**, CDM-62 |
| MN1-094 | confort | Tableau de bord de reprise : comptes par statut et par file, demandes sans file, pièces non copiées → DEM-343 précisé |

### Manques transverses : mobile, IA, tests

- **Mobile** : la file maintenance et la file reprographie sont **d'abord** mobiles ; N1-7 est après la bascule, mais N1-4 doit fonctionner à 375 px dès février (tient : §27).
- **IA** : à la bascule, **aucune** IA dans N1 (routage déterministe) ; c'est cohérent avec NJ-87 et réduit le risque ; la qualité des **mots-clés** du catalogue devient décisive (MN1-002 : jeu de 200 phrases réelles anonymisées tirées des objets des demandes Next.js, **sans** leur contenu, pour tester la recherche — à faire par Florian ou sur l'export anonymisé).
- **Tests** : ajouter les pièges de la passe (conflit d'intérêts, restriction, accueil qui ne voit pas les doublons d'un autre, avis de reprise unique).

---

## 5. Vérification réglementaire

Aucune vérification en ligne dans cette session. La passe **confirme** que le §6.1 (CRPA) est **[NV]** et doit être vérifié avant N1-1 pour les établissements publics (accusé à mentions, L114-2, L114-5, décision implicite et ses exceptions). Elle ajoute deux points [NV] : (1) **référent harcèlement** et obligations de l'employeur (code du travail) pour les plaintes d'un salarié de l'organisme de gestion (MN1-061) ; (2) **délégation de signature** du chef d'établissement d'un EPLE (MN1-013) : forme et publicité de l'acte, à vérifier avec le socle 6 (`06_PERSONNELS.md` §8.9 la note déjà [NV]).

---

## 6. Manques qui relèvent d'autres socles

| Manque | Socle | Proposition |
|---|---|---|
| Port `heures::constater` et `heures::regulariser` (origine « demande N1 ») | 15 | ajouter au §6.2 du 15 et au lot A7 |
| Conversion « fil → demande » (MSG-176 proposé) | 16 | ajouter au §19.3 du 16 |
| Restriction de dépôt symétrique de MSG-226 | 16 | aligner les deux restrictions (même décision, deux effets) |
| Prise des natures N1 : `ent_afaire_prise` non utilisée | 29 | précision de l'arbitrage A4 |
| Habilitation de signature des attestations au secrétariat | 6 | `ent_habilitation_signature` (déjà conçue) : documents `attestation`, `certificat_scolarite` |
| Personne de confiance autorisée à déposer pour une famille | 4 | droit du lien `deposer_demandes` (à ajouter) |
| Volet technique des interventions, envoi qualifié aux prestataires | 44 | RM4 |
| Reprise des HSE : RHP-252 sans objet | 49 | à retirer ou réécrire |

---

## 7. Synthèse

### 7.1 Manques par gravité

| Gravité | Nombre | Numéros |
|---|---|---|
| bloquant | 3 | MN1-060, MN1-092, MN1-099 |
| important | 31 | MN1-002, MN1-010, 011, 013, 020, 021, 023, 030 à 033, 040 à 042, 050 à 052, 061, 062, 070 à 072, 080, 085, 086, 090, 091, 093, 095 à 097 |
| confort | 14 | MN1-001, 012, 014, 022, 024, 034, 043, 044, 053, 073, 081, 087, 094, 098 |
| règle confirmée (sans gravité) | 2 | MN1-054 (aucun RIB dans une demande), MN1-082 (parents d'un majeur) |
| différenciant / refusé | 1 | MN1-025 (refusé : pas de donnée de santé déduite) |
| **Total** | **51** | |

### 7.2 Ajouts dans la conception (`N1_DEMANDES.md` §36)

**24 actions** (DEM-360 à DEM-383), **6 règles** (R-DM47 à R-DM52), **16 cas limites** (CDM-57 à CDM-72), précisions de catalogue et de préréglages (§36.5), une migration de la réserve (3049 : exclusions et restrictions).

### 7.3 Les dix ajouts les plus importants

1. Exclusion pour conflit d'intérêts et file de repli hors service (R-DM47).
2. Restriction de dépôt motivée et réexaminée (R-DM48).
3. Avis de reprise unique, différé à l'ouverture (R-DM49).
4. Attestation HSE fondée sur un indice ou une déclaration sur l'honneur (R-DM50).
5. L'accueil ne voit aucune demande existante d'une autre personne ; rappel au numéro connu (R-DM51).
6. Transfert et requalification en masse (DEM-372).
7. Une demande de copies pour plusieurs classes, dupliquer, brouillon récurrent (DEM-366 à DEM-368).
8. Tournée de maintenance, photo avant/après, « hors de notre ressort » (DEM-369 à DEM-371).
9. Enveloppe prévisionnelle et équité des HSE (DEM-376, DEM-377).
10. Délégation de signature des attestations ; décisions implicites à venir (DEM-378, DEM-383).

### 7.4 Ce que la passe a montré sur la conception

La conception tenait bien la **séparation entre services** et la **minimisation vers l'extérieur** ; elle était faible sur la **séparation à l'intérieur d'un service** (conflit d'intérêts), sur la **protection des personnels** contre un usage abusif, et sur les **moments de masse** (rentrée, bascule). Les écarts voulus avec le Next.js doivent être expliqués aux personnels.

---

## 8. Décisions : appliquées par défaut (D-031) et soumises à Florian

### 8.1 Appliquées par défaut (D-031)

Les trois bloquants (A, B, C) et tous les ajouts du §7.2.

### 8.2 Soumis à Florian

| N° | Question | Recommandation |
|---|---|---|
| Q-DM14 | Qui est la **file de repli hors service** de la file RH de l'organisme de gestion (président, vice-président, expert extérieur) ? | le président de l'organisme de gestion, sans compte ScolIA nécessaire : la demande lui est adressée par un **lien nominatif** du socle 8, ou le demandeur est orienté vers lui |
| Q-DM15 | Faut-il annoncer aux personnels, avant février, la fin du dépôt public et le service fait des HSE ? Par qui ? | oui ; note préparée par N1-5, envoyée par la direction |
| Q-DM16 | Les prestataires extérieurs (chauffagiste, ascensoriste) doivent-ils un jour **voir** les demandes ? | non en V1 (envoi qualifié par le 44) |

---

## 8 bis. Réglages signalés (D-023)

| N° | Réglage | Défaut |
|---|---|---|
| SN1-14 | Plafond hebdomadaire d'une restriction de dépôt | 3 demandes |
| SN1-15 | Taux d'échantillon de contrôle des HSE sur déclaration sur l'honneur | 10 % |
| SN1-16 | Heure d'envoi de l'avis de reprise | 8 h le jour d'ouverture |

---

## 9. Faits, hypothèses, contradictions

**Faits** : lecture de la conception et du module Next.js cité. **Hypothèses** : toutes les journées des personas [H] ; existence d'un référent harcèlement chez l'organisme de gestion [H] ; pratique de la délégation de signature des attestations [H]. **Contradictions** : la conception (§9.5) montrait les doublons au déposant pour le compte (corrigé par R-DM51) ; la conception (§10.2) prévoyait une file de repli unique « secrétariat de direction » par défaut, inadaptée à la maintenance (corrigé §36.5).

---

## 10. Seconde passe (2026-11-13, après les précisions de Florian relayées par le pilote) — sur `N1_DEMANDES.md` §37

**Pourquoi** : Florian a précisé le fil directeur (« certitude d'être traité », « plus simple et mieux », porte unique « Écrire à l'établissement » près de la messagerie). La conception a été complétée (§37) ; cette seconde passe la met à l'épreuve avec six personas choisies pour **mordre** sur ces points. Ses corrections sont **dans** le §37 (cas limites CDM-79 à CDM-86, actions DEM-391, DEM-398, DEM-399, règle R-DM57 sur les mineurs) et au **§37.11**.

### P13 — Mme Ferreira, mère pressée et inquiète (fils en 5ᵉ)

**Sa situation.** 7 h 40, au travail dans une heure : son fils a perdu son carnet et sa carte de cantine, et la veille il est rentré triste « à cause de garçons ». Elle tape un seul message qui mêle tout. Elle renvoie le même texte deux fois parce que rien ne se passe à l'écran.
**Ce qu'elle redoute** : que personne ne lise ; qu'on traite la carte et pas le reste.
**Ce qui la ravirait** : savoir que c'est **vu**, par **qui**, et être **rappelée**.

| Manque | Gravité | Traitement |
|---|---|---|
| MN1-110 | **important** | Un texte qui mêle démarche et inquiétude : l'aiguillage doit proposer **les deux** (la carte → demande ; « garçons », « triste » ne sont pas des mots d'alerte forts → « écrire à la vie scolaire ») et accepter deux envois depuis un seul texte → CDM-73, §37.2 |
| MN1-111 | important | Double envoi par impatience → doublon montré avec « vu par la vie scolaire à 8 h 02 » → CDM-80, R-DM53 |
| MN1-112 | important | « **Je préfère être rappelée** » au numéro de la fiche → §37.11.1 |
| MN1-113 | confort | Accusé qui dit **quand** elle aura une réponse, pas seulement « sous 3 jours ouvrés » (« au plus tard jeudi ») → date calculée affichée, §37.1 |

### P14 — M. Haddad, père qui lit mal le français

**Sa situation.** Il veut une attestation de scolarité pour la caisse d'allocations familiales ; il écrit en arabe sur son téléphone, ou en français phonétique (« atestasion pour la caf »).
**Ce qu'il redoute** : se tromper, avoir honte au guichet.
**Ce qui le ravirait** : une image à toucher, une réponse qu'il comprend.

| Manque | Gravité | Traitement |
|---|---|---|
| MN1-120 | **important** | Mots-clés seulement en français correct : tolérance aux fautes (déjà §9.1) **et** catalogue traduit quand rien n'est trouvé → CDM-79, DEM-391 |
| MN1-121 | important | **Vignettes avec pictogramme** pour les huit démarches les plus fréquentes, dictée vocale native → §37.11.2 |
| MN1-122 | important | Réponse du secrétariat traduite (relue) et attestation produite **en français** (document officiel) avec une phrase traduite d'explication dans l'avis → DEM-209 précisé |
| MN1-123 | — | Aucune IA nécessaire pour l'aiguiller (R-DM57) : tient ; l'IA de traduction reste facultative et pseudonymisée |

### P15 — Inès, 13 ans, timide

**Sa situation.** Elle n'ose pas aller au bureau de la vie scolaire. Elle voudrait un casier, et elle voudrait dire qu'on se moque d'elle dans le bus. Ses parents ont un compte.
**Ce qu'elle redoute** : que ses parents ou ses camarades sachent ; qu'un adulte « en fasse une affaire ».
**Ce qui la ravirait** : écrire au calme, être sûre qu'un adulte précis l'a lu.

| Manque | Gravité | Traitement |
|---|---|---|
| MN1-130 | **bloquant** | La conception (§14.2.5) rend certaines demandes d'élève visibles des parents ; rien ne garantissait que le **passage par le canal protégé** (« parler à un adulte ») restait invisible des parents → **corrigé** §37.11.3 (jamais visible des responsables) ; appliqué par défaut (D-031) |
| MN1-131 | important | « Vue par » et prénom de l'adulte qui la prend : rassurant, **gardé** (R-DM53) ; pour le canal protégé, c'est le 36 qui répond (hors N1) |
| MN1-132 | important | Le casier (demande N1 visible des parents si payant) et la moquerie (canal protégé) partent **séparément** depuis le même texte, l'élève voyant pour chacun « tes parents verront / ne verront pas » → §37.11.3, CDM-74 |
| MN1-133 | — | Aucune IA sur son texte (mineure, mot d'alerte possible) : tient (R-DM57) |

### P02 bis — M. Dubois, agent de maintenance, téléphone en main

Il teste le **signalement par photo** (DEM-385) dans une salle sans QR et le « résolu ? » à la clôture.

| Manque | Gravité | Traitement |
|---|---|---|
| MN1-140 | important | Lieu proposé quand il n'a **pas de séance** (agent) : dernière salle choisie, puis liste des salles du bâtiment le plus proche de sa dernière intervention ; jamais la position du téléphone → CDM-85 |
| MN1-141 | important | « Non, toujours un problème » cliqué par un **appui** (pas le demandeur) : rouvre aussi (tout abonné peut dire que ce n'est pas réglé) → DEM-394 précisé |
| MN1-142 | confort | Il veut **voir** l'indicateur « résolu du premier coup » de sa file : oui pour les membres de la file (agrégé), jamais par agent → DEM-395 précisé |

### P01 bis — Mme Lefèvre, secrétaire débordée, avec la boîte de service et la file de demandes

Avant le §37, elle avait **deux** boîtes : les messages des familles (16) et les demandes (N1). Elle teste l'**Accueil du service**.

| Manque | Gravité | Traitement |
|---|---|---|
| MN1-150 | **bloquant** (levé) | Deux boîtes concurrentes : **levé** par le jumelage et l'écran unique (R-DM58, DEM-396, DEM-397) |
| MN1-151 | important | Filtrer par type, par famille ; « ce qui attend une réponse de nous » en premier ; classer d'un geste un simple « merci » → §37.11.4 |
| MN1-152 | important | Convertir sans ressaisir : le texte du message pré-remplit la demande (champ « précision »), le fil est rattaché → DEM-318 (tient) |

### P10 bis — M. Petit, principal, qui veut voir ce qui traîne

| Manque | Gravité | Traitement |
|---|---|---|
| MN1-160 | important | « Vues mais non prises » (on a lu, personne ne s'en occupe) : le signal le plus parlant du malaise d'un service → DEM-399 |
| MN1-161 | important | Ne pas transformer le tableau en surveillance des agents : aucun classement nominatif, effectifs < 5 masqués (R-DM58, DEM-395) |
| MN1-162 | confort | Lien du tableau vers la file, sans contenu C2+ (titres neutres) : tient |

### 10.1 Synthèse de la seconde passe

| Gravité | Nombre | Numéros |
|---|---|---|
| bloquant | 2 (dont 1 levé par le §37) | MN1-130 (appliqué par défaut), MN1-150 (levé) |
| important | 14 | MN1-110 à 112, 120 à 122, 131, 132, 140, 141, 151, 152, 160, 161 |
| confort | 3 | MN1-113, 142, 162 |
| règle confirmée | 2 | MN1-123, MN1-133 |

**Ce que la seconde passe a montré** : la porte unique tient si elle **n'oblige jamais** (R-DM55) et si elle **sépare** proprement, depuis un même texte, ce qui va en demande, en message et au canal protégé, avec pour chaque envoi ce que verront les parents. Le risque principal n'était pas l'aiguillage mais la **visibilité familiale du canal protégé** (MN1-130), corrigée.
