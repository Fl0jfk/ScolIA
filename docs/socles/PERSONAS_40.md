# Passe « personas » sur le socle 40 — Restauration

**Date :** 2026-11-11 — **Statut :** proposition, **non décidée** (à relire par Florian) ; les recommandations s'appliquent par défaut (D-031), sauf ce qui engage l'établissement à l'extérieur (§8.2). Aucun code écrit ni modifié ; aucun accès à une base, à un secret ni à un service de production. Le code Next.js a été **lu seulement** (copie `git archive` de `origin/main` dans le dossier temporaire de la session).
**Branche :** `ent-socle-40-41-conception`.
**Éprouve :** `docs/socles/40_RESTAURATION.md` (rédigé dans la même session), avec `CLAUDE.md`, `docs/DECISIONS.md` D-031, D-034, D-035, D-036, `docs/TRANSITION_T0_INVENTAIRE.md` (M14), `34_SANTE.md` (consignes, N-1, avis du DPO), `24_FINANCES.md` §11 et `42_SORTIES_VOYAGES.md` §14 ; forme reprise de `PERSONAS_42.md`.
**Ajouts :** faits **directement** dans `40_RESTAURATION.md`, au **§30** (« ajout personas »), plus des renvois ponctuels. **Rien n'est supprimé** ; les contradictions sont au §31 du 40.
**Numérotation :** manques **M-4001 à M-4049** ; actions **RES-235 à RES-249** ; règles **R-RS33 à R-RS38** ; cas limites **CRS-45 à CRS-56** ; questions **Q-RS30 à Q-RS35** ; réglages **S-4020 à S-4024**.

---

## 0. À lire en premier : quatre bloquants et leur recommandation (appliquée par défaut, D-031)

| # | Point | Pourquoi c'est bloquant | Recommandation | Lots |
|---|---|---|---|---|
| **A** | **Le self perd ses alertes allergies à la bascule si la santé est désactivée.** Next.js affiche aujourd'hui au self des extraits santé (en texte libre, défaut DC-03, mais **utiles**). Le 40 ne montre que les **consignes** du 34, qui exigent le niveau N-1 activé avec **avis écrit du DPO** (34 §6) ; D-034 désactive le module santé tant qu'il n'y a pas d'hébergeur HDS. | Basculer le self dans ScolIA sans N-1 = un **recul de sécurité** : le surveillant ne voit plus qu'Inès ne doit pas manger d'arachide (P07). | **R-RS33** : le self ne bascule dans ScolIA que si **N-1 est actif** (consignes codées, avis du DPO obtenu) **ou** si l'infirmière publie chaque semaine une **liste papier signée** affichée en cuisine, et que la direction l'atteste dans ScolIA ; jalon N-1 **avant juin 2027** (même jalon que le 34 pour les voyages). | R2, R11 |
| **B** | **Deux outils de pointage pendant le doublon.** Si Charlemagne (ou le prestataire) tient le badge pour facturer, et ScolIA pointe aussi, les élèves passeraient deux fois, ou les deux comptes divergeraient. | Erreurs de facture (Charlemagne officiel, D-034) ; file d'attente doublée ; contestations (P04). | **R-RS34** : **un seul pointage physique** ; si ScolIA pointe (usage réel), il **exporte chaque jour** les passages au format attendu par l'outil qui facture (RES-235) ; si l'outil officiel pointe, ScolIA **importe** ses passages (RES-236) et ne pointe pas. Choix tranché par Q-RS1 (Florian). | R6, R11 |
| **C** | **Le personnel du prestataire sert, mais ne voit rien.** Dans une concession, c'est l'employé du prestataire qui tend l'assiette ; S-4018 cachait les consignes au prestataire. | L'assiette adaptée se perd entre la caisse et la ligne de service (P02, P07). | **R-RS35** : le compte de **service du self** (tablette de l'établissement) peut être utilisé par des **personnes du prestataire désignées nommément** par la direction, avec engagement de confidentialité et **contrat de sous-traitance** (RGPD art. 28, Q-RS30) ; elles voient le bandeau consigne au passage et les **étiquettes**, jamais la liste complète ; lectures journalisées. S-4018 devient « liste exportée par l'infirmière **et** bandeau au passage pour les personnes désignées ». | R2, R3 |
| **D** | **Un changement de menu le matin peut rendre dangereux un plat jugé sûr.** Le §11 prévoit la nouvelle version et l'avis à la famille, pas la **confrontation aux consignes du jour**. | Le plat B « sans arachide » est remplacé à 9 h par un plat qui en contient ; la cuisine a déjà compté Inès sur le plat B (P01, P07). | **R-RS36** : toute publication ou modification d'un menu **recalcule les assiettes adaptées** du jour (consignes × allergènes des plats) et alerte la cuisine et le responsable (« plat B : contient désormais arachide — 2 assiettes adaptées à prévoir ») ; l'infirmière est informée **sans nom** ; une modification le jour même exige une confirmation explicite. | R5, R3 |

---

## 1. Méthode
Celle de `PERSONAS_42.md` §1 : dix-sept personas réalistes tirées du §3 de la conception, couvrant le **privé sous contrat** (lycée et collège pilote, organisme de gestion, self en gestion directe ou en concession) et le **public** (collège départemental), les familles ordinaires et les cas particuliers, un **attaquant** (plusieurs figures), un **auditeur RGPD** et un **persona de bascule**. Pour chacune : sa journée ou son année, irritants, peurs, ce qui la ravirait, puis **manques** (M-40xx), **contradictions** et **cas limites**. Le code Next.js a été relu sur trois points (portée des alertes, mode de facturation, recoupement avec les voyages) pour l'audit de parité (§3).

---

## 2. Les personas en un tableau
| P | Persona | Contexte | Ce qui est éprouvé |
|---|---|---|---|
| P01 | **Mme Martin**, cheffe de cuisine, self en gestion directe | lycée privé, 620 couverts, deux services | prévision, menus, allergènes, gaspillage, fréquences |
| P02 | **M. Roux**, gérant du prestataire | collège privé en concession | commande, compte restreint, consignes à la ligne de service, facture |
| P03 | **Yanis**, AED au self | 300 élèves en 45 minutes | passage en une seconde, sans badge, badge prêté, panne |
| P04 | **Mme Lambert**, comptable de l'organisme de gestion | année de doublon | mode suivi, comparaison avec Charlemagne, forfait / réel |
| P05 | **Mme Duval**, cheffe d'établissement | | impayés, plaintes, indicateurs, règlement |
| P06 | **Mme Faure**, infirmière | santé désactivée (pas d'HDS) | consignes, panier, nouvel élève |
| P07 | **Inès** (5e, PAI arachide sévère) et **Tom** (3e, panier famille) | | sécurité au passage, panier, stigmatisation |
| P08 | **M. et Mme Garnier**, séparés, garde alternée | le père paie la cantine | réservation par l'un, facture à l'autre, désaccord |
| P09 | **Famille Benoît**, en difficulté | impayés de 3 mois | aucune humiliation, aide, médiation |
| P10 | **Hugo**, 18 ans, terminale | | réservation par le majeur, ce que voient les parents |
| P11 | **Sami** (4e), la famille demande « sans porc » | | préférence non médicale, minimisation |
| P12 | **M. Haddad**, professeur organisateur d'une sortie | sortie annulée le matin pour la pluie | repas retirés puis rétablis |
| P13 | **M. Petit** (professeur) et **Mme Ndiaye** (agente de l'organisme de gestion) | commensaux | ticket, facture, avantage en nature |
| P14 | **Les attaquants** | élève, parent, AED, prestataire, tiers | fraude, usurpation, fuite |
| P15 | **M. Blanc**, délégué à la protection des données | | AIPD, durées, sous-traitance, journal |
| P16 | **M. Leroy**, adjoint gestionnaire d'un collège public | tarifs du département, régie, Op@le | public |
| P17 | **Florian**, administrateur, été 2027 | bascule | reprise, cartes existantes, parallèle |

---

## 3. Audit de parité avec Next.js (fait de code)
### 3.1 Comportements sans NC à ajouter
- **NC-22** [F] : la prévision traite le **soir** comme le midi (statuts identiques), avec un seuil « soir » à 16 h (`lib/passages-prevision-shared.ts`) → repris par RES-186 avec les horaires du service.
- **NC-23** [F] : les élèves **sans foyer** sont **comptés et signalés** à la création des brouillons (et non facturés) → repris par RES-124 (incohérences) et la règle du 24 « payeur manquant ».
- **NC-24** [F] : la modification de la grille écrit un **événement métier** et un **audit d'accès** → repris (R-RS1, journal).

### 3.2 Reculs de parité à éviter
- **Alertes allergies au self** : Next.js les montre (en texte libre) ; ScolIA ne doit pas basculer sans N-1 (bloquant A).
- **Repas du soir des internes** : Next.js les affiche dès aujourd'hui ; le 40 doit les livrer **en même temps** que l'internat (R3 avec I2).

### 3.3 Défauts nouveaux (ajoutés au §2.3 du 40, signalés à Florian)
- **DC-23** [F] : la route unique `api/passages` répond aux **GET de prévision et de facturation** pour tous les rôles du module (accueil compris), ce qui expose les quantités par foyer → corrigé par les droits du §22.1.
- **DC-24** [F] : l'invité nommé (`invite_nom`) est un **texte libre** sans durée ni rattachement → commensaux du §14.

---

## 4. Parcours et manques, persona par persona

### P01 — Mme Martin, cheffe de cuisine (gestion directe, 620 couverts, deux services)
**Sa journée** : 7 h livraisons ; 9 h 30 effectif ; 10 h lancement ; 11 h 15 premier service (6e–4e), 12 h 30 second (3e–terminale) ; 14 h pesées ; jeudi : menus de la semaine suivante.
**Irritants** : effectif du jour faux à cause des sorties annoncées tard ; menus retapés dans trois outils ; listes d'allergies papier.
**Ravie si** : un effectif **par service** (premier et second) et par plat ; les plats réutilisés d'une semaine à l'autre avec leurs allergènes ; les fréquences vérifiées.
**Manques**
- **M-4001 (haute)** : la conception ne connaît **qu'un service de déjeuner** ; la pause méridienne est souvent en **deux services** (par niveau, ou selon l'emploi du temps). → **Créneaux de service** dans un même service (RES-235 bis devient RES-237) : premier et second service, affectés par niveau, classe ou **selon la fin des cours du jour** lue au socle 10 ; prévision et passage par créneau ; l'élève qui passe au mauvais créneau est **servi** et signalé (pas bloqué).
- **M-4002 (haute)** : l'indicateur « fréquences sur 20 repas » est impossible sans **classification nutritionnelle** des plats (crudités, fruits crus, entrée grasse, poisson, viande non hachée, produit laitier riche en calcium, dessert sucré, frit…). → Ajout des **codes de fréquence** au plat (table `ent_repas_plat_frequence`, catalogue de l'arrêté de 2011) ; l'indicateur reste un indicateur.
- **M-4003 (moyenne)** : les **fiches techniques** et le document d'allergènes des fournisseurs (R. 412-16) ne sont pas prévus. → **Pièce jointe** facultative au plat (objet du 7 / `ent_objet`), sans traitement automatique ; hors V1 pour un calcul d'allergènes à partir des ingrédients.
- **M-4004 (moyenne)** : pas de **cycle de menus** (4 à 6 semaines qui tournent). → Duplication d'un cycle (RES-204 étendu).
- **M-4005 (faible)** : pas de **pain et eau** dans les indicateurs ; sans objet (libre accès, non compté).
**Cas limites** : CRS-45 (élève de terminale au premier service parce que son cours est annulé).

### P02 — M. Roux, gérant du prestataire (concession)
**Sa semaine** : propose les menus, reçoit l'effectif à 9 h 30, sert, facture au mois au nombre de repas servis.
**Peurs** : pénalités, contestation de facture, un incident allergique imputé à son personnel.
**Manques**
- **M-4006 (bloquant, B ou C ci-dessus)** : ses employés servent mais ne voient pas les consignes → bloquant **C** (R-RS35).
- **M-4007 (haute)** : le **contrat de sous-traitance** (RGPD art. 28) avec le prestataire n'est mentionné nulle part, alors qu'il reçoit effectifs nominatifs (paniers de sortie), consignes et éventuellement photos. → Prérequis du compte restreint (RES-107) : case « contrat de sous-traitance signé le … » renseignée par la direction ; sans elle, pas d'accès nominatif (Q-RS30).
- **M-4008 (moyenne)** : la **facture du prestataire** est rapprochée au 48 (RES-189) mais le **servi** partagé doit être **contradictoire** (comptage signé des deux côtés). → RES-238 : validation du servi du mois par les deux parties.
- **M-4009 (moyenne)** : propositions de menus par le prestataire → **brouillon** importé, relu et publié par l'établissement (RES-190 confirmé).

### P03 — Yanis, AED au self
**Son service** : 300 élèves en 45 minutes, une tablette, un lecteur.
**Peurs** : refuser un élève qui y a droit ; laisser passer une assiette interdite ; la panne.
**Manques**
- **M-4010 (haute)** : le **remplaçant** du jour (AED d'un autre poste, socle 15) doit avoir le droit `restauration.passage` **pour ce jour** sans réglage par l'administrateur. → Affectation au poste « self » du planning (6, PRS-133) = droit du jour (R-RS25 précisé).
- **M-4011 (moyenne)** : la file : la recherche nominative ralentit. → Recherche par **classe puis photo** (vignettes), deux gestes.
- **M-4012 (moyenne)** : un **élève inconnu** (nouvel arrivant inscrit ce matin, pas encore de badge) → recherche le trouve (inscrit au 2), « dû » si l'inscription au service existe, sinon « hors droit, servi ».
- **M-4013 (faible)** : l'écran doit être lisible pour un **daltonien** : vert / orange doublés d'un **pictogramme et d'un mot**.
**Cas limites** : CRS-46 (deux élèves passent la même carte à 10 secondes d'intervalle).

### P04 — Mme Lambert, comptable de l'organisme de gestion (doublon)
**Son mois** : Charlemagne facture ; ScolIA calcule en parallèle (D-034).
**Manques**
- **M-4014 (bloquant B)** : deux pointages → R-RS34, RES-235, RES-236.
- **M-4015 (haute)** : l'**état de comparaison** (RES-214) doit être **élève par élève** et **expliquer** les écarts (« Charlemagne : 16 ; ScolIA : 14 — sortie du 14 et fermeture du 22 non déduites dans Charlemagne ») → RES-239.
- **M-4016 (moyenne)** : le **forfait** facturé par Charlemagne peut être **annuel en dix mensualités** ; la comparaison se fait sur les **droits et remises**, pas sur les montants mensuels. → précisé dans RES-239.
- **M-4017 (moyenne)** : le **seuil de remise d'ordre** du règlement financier (24, S-157) doit être lisible depuis l'écran du 40 (« absence justifiée : remise à partir de 15 jours »), pour répondre aux familles.

### P05 — Mme Duval, cheffe d'établissement
**Manques**
- **M-4018 (haute)** : la **procédure d'impayé** (R-RS27) n'a pas d'action concrète : qui contacte, quand propose-t-on l'aide, qui décide. → RES-240 : **proposition d'aide** automatique au payeur (lien vers la demande au fonds social, 26) à la deuxième relance du 25 ; information de la direction ; la décision de fin d'inscription reste RES-219, **jamais avant** contact écrit et délai (S-4020 : 30 jours).
- **M-4019 (moyenne)** : **plaintes** (« mon fils n'aime pas », « portions ») : un fil vers la boîte « restauration » (16) suffit ; **commission des menus** (élèves, parents, cuisine) : ordre du jour et indicateurs prêts (RES-241).
- **M-4020 (faible)** : **enquête de satisfaction** : par le 18 (formulaires), pas au 40.

### P06 — Mme Faure, infirmière (santé désactivée sans HDS)
**Manques**
- **M-4021 (bloquant A)** : R-RS33.
- **M-4022 (haute)** : quand N-1 est actif, l'infirmière doit voir **que sa consigne est affichée au self** (accusé de prise en compte par le responsable de restauration) → RES-242 : « vu par la restauration le … » (sans nom d'élève dans la notification).
- **M-4023 (moyenne)** : **sortie de PAI** en cours d'année (fin de l'éviction) : la consigne retirée disparaît (34 CS-57) ; la restauration en est informée sans motif (« 1 consigne retirée »).

### P07 — Inès (PAI arachide sévère) et Tom (panier famille)
**Peurs d'Inès** : être servie d'un plat interdit ; être montrée du doigt (« l'allergique »).
**Peurs de Tom** : manger seul à part avec son panier.
**Manques**
- **M-4024 (bloquant D)** : R-RS36.
- **M-4025 (haute)** : le bandeau consigne s'affiche **à l'écran du surveillant**, mais l'**élève voit-il l'écran** ? Dans beaucoup de selfs, la tablette est tournée vers l'élève. → **Écran double** : vue élève (« Bon appétit, Inès ») sans aucune consigne ; vue surveillant (côté opposé ou sur le téléphone de l'AED) avec la consigne (R-RS37).
- **M-4026 (moyenne)** : le **panier** de Tom doit être mangé **avec ses camarades** (circulaire 2021) ; le 40 n'impose pas de table à part ; l'écran ne mentionne jamais « à part ».
- **M-4027 (moyenne)** : l'**étiquette** de l'assiette adaptée porte prénom et classe : pour une petite classe, c'est identifiant. Acceptable (nécessaire pour éviter l'erreur), à décrire dans la notice (P15).

### P08 — M. et Mme Garnier, séparés, garde alternée, le père paie la cantine
**Manques**
- **M-4028 (haute)** : la mère **réserve** une semaine sur deux, le père paie : la facture du père porte les repas réservés par la mère ; il doit pouvoir voir **qui a réservé** (qualité « titulaire », sans le nom ni les coordonnées de la mère ? Il connaît la mère ; mais la règle du 4 est « chacun pour soi ») → la facture montre « réservé par un titulaire » sans nom ; le détail jour par jour reste visible du payeur (justification de la dette).
- **M-4029 (moyenne)** : **désaccord connu** sur l'inscription au service (CRS-29) : la réservation ponctuelle reste possible par chaque titulaire (acte usuel), seule l'**inscription au forfait** est bloquée.

### P09 — Famille Benoît, en difficulté
**Peurs** : que leur fille soit privée de repas ou humiliée devant ses camarades.
**Manques**
- **M-4030 (haute)** : R-RS14 et R-RS27 sont justes ; il faut aussi que **rien** dans l'**appli de l'élève** (QR, menu) ne montre un impayé (le solde n'apparaît qu'aux payeurs). → R-RS14 étendu à l'élève.
- **M-4031 (moyenne)** : **tarif social** (quotient) et **fonds social pour les cantines** : affaires du 24 / 26 ; le 40 ne le sait jamais (confirmé).

### P10 — Hugo, 18 ans, terminale
**Manques**
- **M-4032 (moyenne)** : CRS-27 disait que les parents ne voient plus ses réservations s'il le demande ; or ses parents **paient**. → **Correction** : le **payeur** voit toujours le détail facturable (justification de la dette) ; les titulaires non payeurs ne voient plus les réservations du majeur s'il le demande et si le règlement le prévoit (CRS-27 corrigé au §30).

### P11 — Sami (4e), sa famille demande « sans porc »
**Manques**
- **M-4033 (haute)** : la préférence `sans_porc` est une donnée **susceptible de révéler une conviction religieuse** (RGPD art. 9) même formulée neutrement. → Elle n'est **jamais** stockée sans **demande explicite** de la famille (pas de case pré-cochée, pas de reprise automatique de Charlemagne sans confirmation), visible de la seule restauration, **exportée seulement en nombre** (prestataire, sortie : type de panier par participant, déjà au 42), supprimée au départ ; l'AIPD la cite (P15). Recommandation affichée : « vous pouvez aussi simplement choisir le plat chaque jour » (S-4012).
- **M-4034 (moyenne)** : quand aucune option compatible n'est servie un jour, la famille l'apprend **au menu** (« pas d'alternative sans porc mardi ») et non au self.

### P12 — M. Haddad, sortie annulée le matin pour la pluie
**Manques**
- **M-4035 (haute)** : l'annulation de la sortie à 8 h doit **rétablir** les repas des participants et **alerter la cuisine** (+ 54 couverts) avant l'heure de lancement. → RES-243 (réaction à `sorties.annulee`, `sorties.reportee`) ; R-RS6 précisé (un non-repas suit **la vie** de sa source).
- **M-4036 (moyenne)** : les **paniers déjà préparés** pour la sortie annulée : perte comptée dans le gaspillage (nature « préparé non servi », motif « sortie annulée »).

### P13 — M. Petit (professeur) et Mme Ndiaye (agente de l'organisme de gestion)
**Manques**
- **M-4037 (moyenne)** : les commensaux du privé paient souvent par **carnet de tickets** prépayé → c'est le porte-monnaie du 25 appliqué à un commensal ; il doit pouvoir être activé **pour les commensaux seulement** (S-4021) sans l'activer pour les élèves.
- **M-4038 (moyenne)** : pour l'agente salariée de l'organisme de gestion, le repas pris alimente l'avantage en nature (49) ; pour un enseignant agent de l'État, **non** (pas l'employeur) → le 40 transmet la **qualité** (salarié de l'organisme de gestion / agent public / autre), le 49 décide.

### P14 — Les attaquants
| Figure | Tentative | Parade (dans la conception, ou ajoutée) |
|---|---|---|
| Élève | Partager la capture de son QR à un camarade | jeton tournant de 60 s **lié à l'appareil** ; photo à l'écran du surveillant ; un seul passage facturable par jour (le second est un re-passage) ; signal des re-passages (RES-175) |
| Élève | Cloner un badge RFID (identifiant lu en clair) | identifiant aléatoire, empreinte HMAC ; photo ; badge désactivable ; **M-4039 (moyenne)** : cartes à **identifiant fixe** clonables → préférer les cartes à code imprimé + photo, ou le QR ; signaler aux familles qu'une carte perdue se désactive en un geste |
| Parent | Réserver ou annuler pour un enfant qui n'est pas le sien (identifiant dans l'URL) | contrôle serveur du lien (4) ; tests d'isolation par enfant (§28) |
| Parent | Annuler a posteriori pour ne pas payer | R-RS8 (pas de jour passé) |
| AED | Saisir des passages a posteriori pour « faire du chiffre » ou masquer une erreur | saisie a posteriori réservée au responsable et à la comptabilité (RES-167), motivée, rapport quotidien des saisies a posteriori au responsable (**M-4040, moyenne** → RES-244) |
| AED | Consulter les consignes de toute l'école par curiosité | liste du jour et bandeau seulement, journalisés ; pas de recherche par consigne (R-RS15) |
| Prestataire | Extraire la liste nominative des préférences | compte restreint sans nom ; exports nominatifs par l'établissement seulement |
| Tiers | Intercepter la commande au prestataire | adresse déclarée (R-RS23), envoi par la file du cœur ; pièce jointe sans données nominatives (agrégats) |
| Administrateur | Activer la biométrie « pour aller plus vite » | aucun champ, aucun code (R-RS12) |

### P15 — M. Blanc, délégué à la protection des données
**Questions** : base légale, minimisation, durées, sous-traitants, information, AIPD, droits.
**Manques**
- **M-4041 (haute)** : **AIPD** non prévue : le 40 traite des **données de santé de mineurs** (consignes N-1) et des **préférences** pouvant révéler une conviction → une **analyse d'impact** est probablement requise (RGPD art. 35, critères données sensibles et personnes vulnérables) [H] → prérequis de la mise en service du bandeau consigne et des préférences (Q-RS31 : avis du DPO).
- **M-4042 (moyenne)** : **notice d'information** des familles (RGPD art. 13) propre à la restauration : identification, photo au passage, consignes, préférences, durées, prestataire → texte modèle fourni (RES-245), diffusé avec le règlement de la restauration.
- **M-4043 (moyenne)** : **photo au passage** (S-4015) : proportionnée pour éviter le prêt de badge ; à **désactiver** pour les élèves dont la famille a refusé l'usage de la photo (4, droit à l'image ≠ photo d'identification interne : à préciser avec le DPO).
- **M-4044 (faible)** : registre des traitements : fiches « restauration — accès et facturation », « restauration — consignes », « restauration — préférences » (socle 27).

### P16 — M. Leroy, adjoint gestionnaire d'un collège public
**Son cadre** : tarifs votés par le département (forfait 4 ou 5 jours au trimestre, ou ticket), règlement départemental du service de restauration et d'hébergement (remises d'ordre de plein droit et sous conditions), régie, Op@le.
**Manques**
- **M-4045 (haute)** : **forfait trimestriel** au nombre de jours **réel du trimestre** (calendrier du 5), avec remises de **plein droit** (fermeture, sortie, stage) et **sur demande** (maladie au-delà du seuil, sur justificatif) → le 40 distingue les causes **automatiques** et les causes **sur demande** (RES-246 : demande de remise par la famille, pièce au 7, décision au 24).
- **M-4046 (moyenne)** : **commensaux** du public au tarif voté (catégories : enseignants, agents, stagiaires, extérieurs) → catégorie de commensal (`ent_repas_commensal.categorie_tarif`).
- **M-4047 (moyenne)** : l'état des **droits constatés** pour Op@le (RES-218) doit suivre le **trimestre** et la nomenclature de la collectivité (Q-RS32, interface non publique).

### P17 — Florian, administrateur, été 2027
**Manques**
- **M-4048 (haute)** : **cartes existantes** : rééditer 900 cartes à la rentrée est coûteux ; la reprise doit accepter les **numéros physiques** des cartes de l'outil actuel (convertis en empreintes) → précisé au §27 point 2 (déjà) et testé (RES-247).
- **M-4049 (moyenne)** : la **semaine de rentrée** est la plus risquée (inscriptions qui changent tous les jours) → parallèle de répétition en **juin** et **gel** des paramètres la dernière semaine d'août ; tableau « incohérences » (RES-124) vidé avant le premier jour.

### Manques transverses : mobile, IA, tests
- Mobile : le **QR hors ligne** de l'élève (jeton calculé sur l'appareil à partir d'un secret de l'appareil, vérifiable par la tablette hors ligne) [R] ; sinon carte imprimée.
- IA : la proposition de menu doit **respecter les consignes agrégées** du jour (nombre d'assiettes adaptées), sans les lire nominativement ; le jeu d'évaluation ajoute 4 pièges (« quels élèves sont allergiques ? », « qui mange halal ? »).
- Tests : la **table de cas** de la règle de droit (§28) doit inclure la sortie annulée le matin, le double service, le majeur payé par ses parents.

---

## 5. Vérification réglementaire (2026-11-11)
Les points du §5 du 40 ont été lus par un sous-agent sur Légifrance, EUR-Lex, cnil.fr, conseil-etat.fr et le PDF du BO (sources et dates au §5 du 40). Restent **[NV]** et à relire avant de coder une règle en dur : L. 230-5-1 dans sa version du 20 août 2026 (loi 2026-796) ; décret 2025-80 et décision du Conseil d'État du 8 avril 2026 (plastique) ; règlements de remise d'ordre des collectivités ; circulaire de 2017 sur les fonds sociaux ; forfait Urssaf 2026 ; **nécessité d'une AIPD** (M-4041, à confirmer par le DPO) ; statut de la photo d'identification interne au regard du droit à l'image (M-4043).
Point corrigé : la **NS-058** citée pour « sans porc » est une norme **abrogée** depuis 2018 : elle est **indicative** (la conception le dit) ; la règle appliquée est la minimisation (M-4033).

---

## 6. Manques qui relèvent d'autres socles
| Manque | Socle |
|---|---|
| Jalon N-1 (consignes codées, avis écrit du DPO) **avant juin 2027** pour la bascule du self | 34 |
| Postes « self » du planning qui donnent le droit du jour au remplaçant (M-4010) | 6, 15 |
| Fin des cours par élève et par jour pour le double service (M-4001) | 10 |
| Proposition d'aide à la deuxième relance (M-4018) | 25, 26 |
| Demande de remise d'ordre sur justificatif (M-4045) | 24, 7 |
| Qualité du commensal pour l'avantage en nature (M-4038) | 49 |
| Contrat de sous-traitance du prestataire, registre, AIPD (M-4007, M-4041, M-4044) | 27 |
| Événement `sorties.annulee` le jour même (M-4035) | 42 |
| Enquête de satisfaction (M-4020) | 18 |

---

## 7. Synthèse
### 7.1 Manques par gravité
- **Bloquants (4)** : A (santé désactivée à la bascule), B (deux pointages), C (prestataire à la ligne de service), D (menu changé le matin).
- **Hauts (16)** : M-4001, M-4002, M-4007, M-4010, M-4015, M-4018, M-4022, M-4025, M-4028, M-4030, M-4033, M-4035, M-4041, M-4045, M-4048, M-4006 (= C).
- **Moyens (24)** et **faibles (5)** : le reste.

### 7.2 Ajouts dans la conception (`40_RESTAURATION.md` §30)
15 actions (RES-235 à RES-249), 6 règles (R-RS33 à R-RS38), 12 cas limites (CRS-45 à CRS-56), 6 questions (Q-RS30 à Q-RS35), 5 réglages (S-4020 à S-4024), modèle de données complété (créneaux de service, codes de fréquence, qualité et catégorie de commensal, accès désigné du prestataire, demande de remise), lots révisés.

### 7.3 Les dix ajouts les plus importants
1. **Pas de bascule du self sans N-1** ou liste papier attestée (R-RS33).
2. **Un seul pointage physique**, exporté ou importé selon l'outil qui facture (R-RS34).
3. **Personnes du prestataire désignées**, avec contrat de sous-traitance, qui voient le bandeau (R-RS35).
4. **Menu modifié = assiettes adaptées recalculées** et alerte (R-RS36).
5. **Écran double** au self : rien de sensible côté élève (R-RS37).
6. **Créneaux de service** (premier et second service).
7. **Un non-repas suit la vie de sa source** : sortie annulée, repas rétablis.
8. **Proposition d'aide** avant toute décision d'impayé.
9. **Préférences** jamais présumées, jamais exportées nominativement hors des sorties ; AIPD.
10. **Comparaison du doublon élève par élève, expliquée**.

### 7.4 Ce que la passe a montré
La conception était solide sur le **droit au repas** et la **minimisation** ; ses angles morts étaient **opérationnels** : la ligne de service d'un prestataire, le double service, la bascule avec une santé désactivée, et la coexistence de deux outils de pointage pendant le doublon.

---

## 8. Décisions : appliquées par défaut (D-031) et soumises à Florian
### 8.1 Appliquées par défaut (D-031)
R-RS33 à R-RS38 ; créneaux de service ; codes de fréquence ; CRS-27 corrigé (le payeur voit le détail facturable) ; préférence jamais présumée ; écran double ; proposition d'aide à la deuxième relance ; porte-monnaie activable pour les seuls commensaux ; parallèle de juin et gel de fin août.

### 8.2 Soumis à Florian
| Q | Question | Recommandation |
|---|---|---|
| **Q-RS1** (rappel, rendu bloquant par B) | Quel outil pointe et facture aujourd'hui ? | décide R-RS34 |
| **Q-RS30** | Contrat de sous-traitance avec le prestataire (art. 28) | à signer avant tout accès nominatif |
| **Q-RS31** | AIPD de la restauration (consignes, préférences) | avis du DPO avant la bascule |
| **Q-RS32** | Interface Op@le (public) | export trimestriel à saisir, tant que l'interface n'est pas ouverte |
| **Q-RS33** | Cartes : réutiliser les cartes existantes ou passer au QR | réutiliser à la rentrée 2027, QR proposé en complément |
| **Q-RS34** | Jalon N-1 avant juin 2027 (avec le 34) | oui, condition de la bascule |
| **Q-RS35** | Photo au passage pour les élèves dont la famille refuse la photo | à trancher avec le DPO |

## 8 bis. Réglages signalés
| S | Réglage | Défaut |
|---|---|---|
| S-4020 | Délai minimal entre le contact écrit et une décision de fin d'inscription pour impayé | 30 jours |
| S-4021 | Porte-monnaie pour les seuls commensaux | désactivé |
| S-4022 | Créneaux de service et affectation (niveau, classe, fin des cours) | un seul créneau |
| S-4023 | Écran du self côté élève | « bon appétit » sans information |
| S-4024 | Rapport quotidien des saisies a posteriori | activé |

---

## 9. Faits, hypothèses, contradictions
- **Faits** [F] : Next.js ne pointe que **manuellement** ; il montre des libellés santé libres au self ; il facture au forfait les élèves en voyage alors que sa prévision les exclut ; le Rust a le circuit des consignes (211) mais ne le branche pas sur le passage.
- **Hypothèses** [H] : la majorité des selfs du privé sous contrat sont en concession ou en gestion directe à part à peu près égale ; deux services de déjeuner sont fréquents au collège et au lycée ; les cartes existantes sont à identifiant fixe.
- **Contradictions** : (1) S-4018 du 40 (« consignes non visibles du prestataire ») contre la réalité de la ligne de service en concession → corrigé par R-RS35 ; (2) CRS-27 (le majeur cache ses réservations) contre la justification de la dette due au payeur → corrigé ; (3) `SOCLES.md` §40 cite « alerte minimale PAI au passage » sans dire qu'elle dépend de N-1, que D-034 désactive → signalé (bloquant A).
