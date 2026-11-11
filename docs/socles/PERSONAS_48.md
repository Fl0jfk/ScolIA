# Passe « personas » sur le socle 48 — Comptabilité, budget et achats

**Date :** 2026-11-11 — **Statut :** proposition, **non décidée** (à relire par Florian) ; recommandations appliquées par défaut (D-031) sauf ce qui engage l'établissement à l'extérieur ou contredit une décision (§6.2). Aucun code écrit ni modifié ; aucun accès à une base, à un secret ni à un service de production.
**Branche :** `ent-socle-48-49-conception`. **Éprouve :** `docs/socles/48_COMPTABILITE.md` (lu en entier), avec `CLAUDE.md`, D-025, D-031, D-034, D-035, D-036, `24_FINANCES.md` (F14 cohabitation), `49_PAIE.md`.
**Ajouts :** faits dans `48_COMPTABILITE.md` **§33** (« ajout personas »). Rien n'est supprimé.
**Numérotation :** manques **M-4800 à M-4899** ; actions **CGA-300 à CGA-329** ; règles **R-CG43 à R-CG54** ; cas limites **CCG-61 à CCG-78** ; questions **Q-CG22 à Q-CG27** ; réglages **S-4823 à S-4832**.

Gravité : **bloquant** · **important** · **confort** · **différenciant**.

---

## 0. À lire en premier : quatre bloquants et leur recommandation (appliquée par défaut, D-031)

| # | Point | Pourquoi c'est bloquant | Recommandation appliquée | Lots |
|---|---|---|---|---|
| **A** | **Pendant le doublon, il n'y aura pas de ventes dans ScolIA.** D-034 garde la facturation des familles dans Charlemagne ; le 24 n'émet rien (F14 « cohabitation »). Le moteur d'écritures en « mode parallèle » (R-CG35) n'aurait donc que des achats, et la **banque** serait mouvementée par deux outils (encaissements des familles dans Charlemagne, paiements fournisseurs dans ScolIA si les achats sont en réel, Q-CG11). | Soit les achats sont saisis deux fois (ScolIA et outil actuel), soit le cabinet reçoit deux flux pour une même banque sans règle ; le budget « réalisé » de ScolIA ignore les produits. | **Deux régimes explicites** : (1) **achats et budget en réel** dans ScolIA dès qu'ils ont fait leurs preuves (ils ne sont pas tenus correctement ailleurs [H]) ; leurs écritures (achats, paiements fournisseurs) sont **exportées en réel** au cabinet dans des **journaux propres à ScolIA** (`HA-S`, `BQ-S`, R-CG43) ; (2) **le rapprochement bancaire reste dans l'outil actuel** pendant le doublon (le module trésorerie de ScolIA tourne en parallèle) ; (3) le **réalisé des produits** est lu dans la **balance mensuelle importée du cabinet** (CGA-232 étendu, CGA-300) pour que le budget soit complet. | CG2, CG8, nouveau CG8b |
| **B** | **Le cabinet ne veut pas 5 000 écritures de ventes nominatives.** L'expert-comptable (P04) tient le dossier avec son plan (comptes à 6 chiffres), ses journaux, et souvent un **411 collectif** sans auxiliaire par famille. | Un export « au détail » est refusé ou ressaisi ; la correspondance de comptes devient manuelle. | Réglage d'export **détaillé** ou **centralisé** par journal (centralisation journalière ou mensuelle par compte et par section, la pièce restant consultable dans ScolIA, R-CG44) ; **table de correspondance de comptes** ScolIA ↔ cabinet (longueur, comptes, journaux) appliquée à l'export (CGA-302) ; auxiliaires familles **optionnels** (S-4824). | CG8 |
| **C** | **La séparation des tâches est impossible dans une petite structure.** Un comptable et un directeur ; le directeur demande lui-même des achats (déplacements, formation) ; en son absence, aucun second valideur. | Le logiciel bloque ou, pire, on contourne (mot de passe partagé). | **Chaîne de suppléance obligatoire** à la mise en service : au moins deux valideurs de lots et d'IBAN parmi direction, président, trésorier (R-CG45) ; les **demandes et dépenses du directeur** sont validées par le président ou le trésorier (R-CG46) ; mode **« structure réduite »** (S-4825) : quand une seconde personne n'existe pas, la validation simple est permise **avec revue a posteriori** mensuelle obligatoire par le trésorier (liste à signer, CGA-305). | CG3, CG5, CG7 |
| **D** | **Fraude interne par IBAN partagé.** Un fournisseur fictif créé avec l'IBAN d'un salarié (49) ou d'un membre de l'équipe ; aucune règle du 48 ne le voit, car les IBAN sont chiffrés et cloisonnés par socle. | C'est le schéma de détournement le plus simple dans une association [H]. | **Empreinte HMAC** de chaque IBAN (clé dédiée, E11) stockée à côté du chiffré, dans le 48 (fournisseurs, notes de frais), le 49 (salariés) et le 25 (familles) : une **collision** entre un IBAN de fournisseur et un IBAN de salarié, de personnel ou de payeur déclenche une alerte à la direction et au trésorier **sans révéler** l'IBAN ni la personne au comptable (R-CG47, CGA-308). | CG5, 49 RH1 |

---

## 1. Méthode

Quinze personas (privé : lycée pilote géré par un OGEC ; public : un EPLE), choisies dans le tableau des acteurs du §4 de la conception, plus un persona attaquant (P14) et un auditeur RGPD (P13). Pour chacune : son mois type, ce qu'elle fait, ce qui coince dans la conception, les manques (M-48xx) et leur traitement. Grille posée à chaque persona : **doublon** (que se passe-t-il en 2027-2028 ?), **séparation des tâches**, **minimisation**, **D-036** (tout est-il en colonnes ?), **IA** (propose-t-elle sans décider ?).

---

## 2. Les personas

| # | Persona | Contexte |
|---|---|---|
| P01 | Mme Lambert, comptable de l'OGEC, seule au service | lycée pilote, 900 élèves, Charlemagne et un tableur aujourd'hui [H] |
| P02 | M. Duval, chef d'établissement | arbitre le budget, demande lui-même des achats |
| P03 | M. Garnier, trésorier bénévole de l'OGEC, retraité, souvent en voyage | second valideur des virements |
| P04 | Mme Roux, expert-comptable du cabinet qui tient le dossier | 120 dossiers, son propre logiciel |
| P05 | Mme Diallo, coordinatrice de SVT | demandes de matériel, devis en ligne |
| P06 | M. Petit, chef cuisinier | denrées chaque jour, mercuriale, factures récapitulatives |
| P07 | Mme Benali, accueil | colis, factures papier |
| P08 | M. Lemaire, organisateur d'un séjour à Londres | avances en livres sterling |
| P09 | M. Haddad, adjoint gestionnaire d'un collège public | Op@le, commande publique |
| P10 | Mme Ferrand, présidente de l'OGEC | vote, responsabilité, conflits d'intérêts |
| P11 | Transports Horizon (fictif), fournisseur | attend son paiement |
| P12 | M. Blanc, commissaire aux comptes | certification, piste d'audit |
| P13 | Mme Moreau, déléguée à la protection des données | auditrice RGPD |
| P14 | Les attaquants | fraude au président, IBAN, initié, facture fictive, restauration |
| P15 | Florian, administrateur, à la veille de la rentrée 2027 | régime du doublon |

---

## 3. Parcours et manques, persona par persona

### P01 — Mme Lambert, comptable seule
Son mois : factures fournisseurs (60 à 120), deux lots de virements, rapprochement bancaire, caisse, relances, export au cabinet, questions des professeurs.
- **M-4800 (important)** : la conception suppose qu'elle **importe** des fournisseurs depuis l'outil actuel (CGA-286) ; elle n'a souvent qu'un tableau ou les fiches de Charlemagne. → import CSV avec **mapping de colonnes** et rapport d'anomalies (SIREN invalide, doublons) ; IBAN importés « en attente » (déjà R-CG26) : la **validation en masse** de 200 IBAN repris est irréaliste par contre-appel. → **reprise attestée** : les IBAN déjà payés sans incident dans les 12 derniers mois par l'outil d'origine peuvent être validés par **attestation de la direction** (liste signée), sans contre-appel individuel ; les autres suivent R-CG26 (R-CG48, CGA-310).
- **M-4801 (important)** : pas de **vue « que faire aujourd'hui »** priorisée par échéance de paiement et pénalités. → la file de travail (CGA-282) trie par **date limite** (échéance fournisseur, préavis de contrat, J+5 de la paie au 49) et montre le montant en jeu.
- **M-4802 (confort)** : modèles d'écritures récurrentes : il manque la **génération par échéancier de contrat** (CGA-163) avec la facture attendue (« la facture de la photocopieuse de novembre n'est pas arrivée »). → alerte « facture attendue non reçue » à J+15 (CGA-311).
- **M-4803 (différenciant)** : continuité : en cas d'absence, son suppléant (cabinet) doit pouvoir **reprendre la file** sans tout réapprendre. → « carnet de procédures » par type de tâche, rédigé une fois (texte simple), lié à chaque tâche (CGA-312).
- Doublon : voir bloquant A.

### P02 — M. Duval, chef d'établissement
- **M-4804 (bloquant, → C)** : ses propres demandes (formation, déplacement) seraient validées… par lui. → R-CG46.
- **M-4805 (important)** : il veut voir le budget **par grandes masses** et par **activité** en une tuile, avec la projection de fin d'année et le point bas de trésorerie (déjà CGA-142, CGA-218) ; il manque l'**explication** des écarts sans jargon. → l'IA « explique un écart » (CGA-273) en phrases simples, avec les 5 pièces qui pèsent.
- **M-4806 (important)** : validation sur **mobile** avec la pièce : la conception le dit ; il manque la règle « pas de validation sans ouverture de la pièce » pour les montants au-delà du seuil direction. → R-CG49 (la pièce doit avoir été affichée).
- **M-4807 (confort)** : budget voté en CA de l'OGEC **et** présenté à l'**APEL** ou au conseil d'établissement : deux présentations. → trames de présentation multiples (socle 7), contenu identique (CGA-130).

### P03 — M. Garnier, trésorier bénévole
- **M-4808 (bloquant, → C)** : il est le seul second valideur et voyage ; les lots attendent. → chaîne de suppléance (R-CG45) ; **calendrier de lots** (deux par mois, S-4826) annoncé aux valideurs ; validation possible depuis l'étranger avec passkey (D-009).
- **M-4809 (important)** : il valide un lot sans comprendre une ligne. → chaque ligne du lot montre fournisseur, objet, demande d'origine, commande, réception, et **nouveauté** du fournisseur (créé ou IBAN modifié récemment, marqué).
- **M-4810 (confort)** : il veut un **rapport mensuel** court (trésorerie, budget, anomalies). → CGA-257, envoyé dans son espace, jamais par pièce jointe.

### P04 — Mme Roux, expert-comptable
- **M-4811 (bloquant, → B)** : centralisation et correspondance de comptes. → R-CG44, CGA-302.
- **M-4812 (important)** : elle veut que ses **écritures d'inventaire** reviennent dans ScolIA (déjà CGA-232) mais aussi que ScolIA **n'écrase pas** ses ajustements par un réexport. → les écritures importées du cabinet ne sont **jamais réexportées** (journal `cabinet` exclu des exports, R-CG50).
- **M-4813 (important)** : elle a besoin d'une **date de verrou** : après sa révision, plus aucune écriture ne doit entrer dans l'exercice. → état d'exercice « en clôture » verrouillé par le cabinet ou le comptable (déjà `ent_cg_exercice.etat`), toute écriture tardive va à N+1 avec alerte (CCG-61).
- **M-4814 (confort)** : accès à toutes les **pièces** d'un mois en une archive. → CGA-313 (archive des pièces d'une période, avec index).
- **M-4815 (important)** : la qualification du forfait d'externat et les comptes de la nomenclature UROGEC sont **à elle** de trancher : la conception le renvoie déjà (Q-CG3, Q-CG7).

### P05 — Mme Diallo, coordinatrice de SVT
- **M-4816 (important)** : les commandes de discipline sont **collectives** (chaque collègue ajoute ses besoins, elle regroupe). → **demande partagée** : brouillon commun d'une discipline, chaque membre ajoute ses lignes, la coordinatrice soumet (CGA-314).
- **M-4817 (important)** : le devis vient d'un **panier** du site du fournisseur (fichier ou lien). → import d'un panier (CSV, PDF de devis lu par l'IA) en lignes de demande (CGA-315).
- **M-4818 (confort)** : réception quand elle n'est pas là : l'accueil réceptionne, mais ne sait pas ce qui est conforme (« des lames de microscope, combien ? »). → l'accueil réceptionne le **colis** (nombre, état), le demandeur confirme la **conformité** (deux temps, CGA-316).
- **M-4819 (confort)** : voir ce que l'enveloppe a déjà financé cette année (liste des commandes). → déjà CGA-136, ajouter la liste.

### P06 — M. Petit, chef cuisinier
- **M-4820 (bloquant pour la restauration)** : les denrées ne suivent pas le schéma demande → commande → réception → facture : **commandes quotidiennes par téléphone**, prix variables (**mercuriale**), **une facture récapitulative** mensuelle pour 20 livraisons, avoirs pour produits refusés. → (1) **marché de denrées** : fournisseur + liste de produits + prix de la semaine (mercuriale importée ou saisie), commande quotidienne rapide (mobile, quantités) ; (2) chaque livraison = réception avec écarts ; (3) **facture récapitulative** rapprochée de **plusieurs** réceptions (CGA-317, R-CG51) ; (4) engagement mensuel sur l'enveloppe restauration (pas une commande par jour).
- **M-4821 (important)** : traçabilité sanitaire (lots, températures) : elle est au socle 40 ; la réception du 48 doit **renvoyer** au contrôle du 40 sans le dupliquer (lien de la réception au contrôle à réception du 40).
- **M-4822 (confort)** : coût matière par repas en direct (CGA-242) : demande du chef pour piloter les menus.

### P07 — Mme Benali, accueil
- **M-4823 (important)** : colis sans référence de commande. → recherche par fournisseur et date parmi les commandes ouvertes ; à défaut, « colis non identifié » (photo) en file du comptable (CGA-318).
- **M-4824 (confort)** : factures papier : la photo du téléphone suffit (CGA-180) ; l'original papier : conservé ou non ? → pour une facture reçue sur papier, la **copie numérique fidèle** peut tenir lieu d'original sous conditions [NV : CGI 289 VII, décret de 2016 sur la numérisation] ; par défaut, **original conservé** en classeur numéroté, la cote inscrite dans ScolIA (CGA-319) ; question au cabinet (Q-CG22).

### P08 — M. Lemaire, séjour à Londres
- **M-4825 (important)** : avances et dépenses en **livres sterling** ; la conception ignore les devises. → montant en devise et taux sur les lignes de paiement et de régie d'avances (`Montantdevise`, `Idevise` du FEC) ; conversion au taux du jour de l'opération ou du change réel (justificatif du bureau de change) ; écart de change en 666 / 766 (R-CG52, CGA-320).
- **M-4826 (important)** : dépenses par carte sur place (métro, entrées) : justificatifs photographiés **dans la journée** depuis le mobile, hors ligne puis synchronisés (CGA-187 étendu) ; liste des dépenses du séjour visible en direct (réalisé du 42).
- **M-4827 (confort)** : le séjour est clos au 42 ; la régie d'avances doit être **reconstituée ou soldée** avant la clôture du séjour (tâche bloquante du 42 → 48).

### P09 — M. Haddad, adjoint gestionnaire (public)
- **M-4828 (important)** : la conception fournit un compteur de seuils par « famille homogène » ; dans les EPLE, la nomenclature d'achat est **propre à l'établissement ou à l'académie** [H]. → nomenclature importable (CSV) et rattachement d'une famille à une catégorie d'achat (déjà `famille_ccp_code`) ; aucune nomenclature imposée.
- **M-4829 (important)** : il veut que la **demande validée** porte tout ce qu'il faut pour l'engagement Op@le (service, domaine, activité [NV]) pour éviter les allers-retours. → champs d'imputation **publique** libres mais paramétrés (codes Op@le saisis comme référentiel local) sur la demande (CGA-321).
- **M-4830 (confort)** : le constat de service fait dans ScolIA est la pièce jointe à l'Op@le ; export PDF par commande (CGA-322).

### P10 — Mme Ferrand, présidente de l'OGEC
- **M-4831 (important)** : **conventions réglementées** : un fournisseur lié à un administrateur ou à un dirigeant (son entreprise, celle d'un parent) ; dans une association recevant des subventions au-delà du seuil, le commissaire aux comptes fait un rapport sur ces conventions [NV : C. com. L612-5]. → **déclaration d'intérêts** des administrateurs et dirigeants (liste de SIREN ou de raisons sociales), contrôle à la création d'un fournisseur et à chaque commande, liste annuelle pour le CAC (R-CG53, CGA-323).
- **M-4832 (important)** : elle signe le budget ; elle veut l'**historique** des versions et des votes (déjà CG2).
- **M-4833 (confort)** : délégations de signature et de pouvoir du bureau au chef d'établissement : pièce au socle 7 et date de fin (lien avec les seuils).

### P11 — Transports Horizon, fournisseur
- **M-4834 (confort)** : savoir **quand** il sera payé et **ce qui** est payé (une facture, plusieurs). → **avis de paiement** au fournisseur après la remise du lot (courriel au contact de facturation, N2 : envoi automatique réglable, S-4827, CGA-324) ; aucun autre détail.
- **M-4835 (important)** : facture rejetée pour « référence de commande manquante » : il faut l'en informer. → motif de litige envoyé au fournisseur (N3) et, si la plateforme agréée le permet, statut « en litige » (CGA-204).

### P12 — M. Blanc, commissaire aux comptes
- **M-4836 (important)** : il veut prouver l'**intangibilité** et la **séparation des tâches**. → (1) rapport de **chaînage** (CGA-228) lisible : premier et dernier numéro, empreintes, résultat du contrôle ; (2) **rapport des validations** : pour chaque lot, facture, IBAN, OD au-delà du seuil, qui a préparé et qui a validé, et les exceptions « structure réduite » (CGA-325).
- **M-4837 (important)** : **circularisation** des fournisseurs et des banques : liste des soldes fournisseurs à une date et coordonnées de contact (CGA-237 étendu).
- **M-4838 (confort)** : accès temporaire : il veut aussi les **budgets votés** et les décisions de dépassement. → inclus dans l'accès CAC (CGA-267).

### P13 — Mme Moreau, DPO (auditrice RGPD)
- **M-4839 (important)** : des **fournisseurs sont des personnes physiques** (auto-entrepreneurs, intervenants, artisans) : leurs données (nom, adresse, IBAN, SIREN personnel) sont des données personnelles ; droit d'accès, durée. → type de fournisseur « personne physique » : mêmes protections que le 49 pour l'IBAN ; mention d'information à la création ; durée alignée sur les pièces (10 ans), le reste archivé à 2 ans sans mouvement (déjà §23.4).
- **M-4840 (important)** : **notes de frais** : les justificatifs (restaurant, hôtel) peuvent révéler des informations personnelles (avec qui, où). → accès limité au comptable et au valideur ; pas de liste nominative des notes de frais pour la direction au-delà du total par personne (R-CG54).
- **M-4841 (important)** : l'**IA** lit des factures : quelles données partent, où ? → déjà R-CG40 et §20.2 ; exiger la **liste des sous-traitants** du fournisseur d'IA et l'absence de conservation (contrat, `IA_COEUR.md`) avant la mise en service de la lecture par modèle de langage ; la lecture des factures **structurées** (Factur-X, UBL) se fait sans IA (déterministe).
- **M-4842 (important)** : le **cabinet** reçoit des données : rôle RGPD (sous-traitant pour la tenue, responsable distinct pour ses missions légales [NV]) : à préciser par contrat (Q-CG23).
- **M-4843 (confort)** : registre des traitements : la conception doit produire la **fiche de traitement** du domaine (finalités : comptabilité, achats, trésorerie ; bases légales : obligation légale, contrat ; destinataires ; durées). → CGA-326 (fiche générée depuis le registre E12 et le §23.4, socle 27).
- **M-4844 (important)** : la **table de correspondance tiers familles** (R-CG21) : qui la voit ? → `facturation.lire` seulement, consultation journalisée ; jamais dans l'accès du cabinet sans décision tracée (déjà §23.2) ; à tester.

### P14 — Les attaquants
| Attaque | Ce qui l'arrête dans la conception | Manque |
|---|---|---|
| **Fraude au président** : courriel « urgent, confidentiel » du faux directeur demandant un virement | aucun paiement hors lot ; lot à double validation ; fournisseur nouveau marqué | **M-4845 (important)** : sensibilisation dans l'écran : bandeau « un paiement urgent demandé par courriel est le signe d'une fraude » sur toute création de fournisseur ou de lot hors calendrier (CGA-327) |
| **Changement d'IBAN** par faux courrier du fournisseur | R-CG26 (contre-appel au numéro connu, second valideur, carence) | — |
| **Initié** : comptable crée un fournisseur avec son IBAN | R-CG27 (second valideur pour fournisseur récent) | **bloquant D** : collision d'IBAN (R-CG47) |
| **Facture fictive** déposée par la plateforme ou par courriel | rapprochement 3 voies (pas de commande → pas de paiement automatique) ; doublons | **M-4846 (important)** : adresse du courriel dédié aux factures : n'accepter que des expéditeurs connus des fournisseurs, sinon file de quarantaine (CGA-328) ; analyse antivirale et neutralisation des PDF (socle 7) |
| **Manipulation d'un export** après génération | empreinte, fichier conservé | — |
| **Professeur** qui énumère les enveloppes ou les factures | R-CG18, tests §23.3 | — |
| **Rejeu d'un lot** de virements après restauration | neutralisation E9 (CGA-296) ; identifiant unique de message (CCG-44) | **M-4847 (important)** : un lot « généré » restauré revient en état « à vérifier », jamais « à remettre » (R-CG28 renforcée) |
| **Compte du trésorier compromis** | passkey (D-009), réauthentification | **M-4848 (confort)** : avis à tous les valideurs à chaque validation de lot (« lot n° … validé par … ») pour qu'une validation inattendue se voie |

### P15 — Florian, veille de la rentrée 2027
- **M-4849 (bloquant, → A)** : il faut un **tableau de régime** par fonction (réel / parallèle / hors ScolIA) pour l'année de doublon, signé par l'établissement. → §33.1 de la conception.
- **M-4850 (important)** : la **non-conformité de réception des factures électroniques** (CCG-60) doit être levée **avant** l'année de doublon, indépendamment de ScolIA (Q-CG4).

---

## 4. Manques qui relèvent d'autres socles

| Manque | Socle | Proposition |
|---|---|---|
| Empreinte HMAC des IBAN des salariés et des payeurs (bloquant D) | 49, 25 | même clé, même calcul ; aucune lecture croisée en clair |
| Réception de denrées et contrôle sanitaire | 40 | lien de la réception du 48 vers le contrôle à réception du 40 |
| Clôture du séjour conditionnée au solde de la régie d'avances | 42 | tâche bloquante |
| Déclaration d'intérêts des administrateurs | 46 (instances) | registre des intérêts tenu au 46, lu par le 48 |
| Fiche de traitement générée | 27 | gabarit commun |
| Archive des pièces d'une période | 7 | export d'un lot de pièces |

---

## 5. Synthèse

### 5.1 Manques par gravité
- **Bloquants (4)** : A (doublon, M-4849), B (cabinet, M-4811), C (séparation des tâches, M-4804, M-4808), D (collision d'IBAN) ; plus **M-4820** (denrées, bloquant pour la restauration).
- **Importants (27)** : M-4800, 4801, 4805, 4806, 4809, 4812, 4813, 4815, 4816, 4817, 4821, 4823, 4825, 4826, 4828, 4829, 4831, 4832, 4835, 4836, 4837, 4839, 4840, 4841, 4842, 4844, 4845, 4846, 4847, 4850.
- **Confort et différenciants** : le reste.

### 5.2 Les dix ajouts les plus importants
1. Régimes du doublon et journaux propres à ScolIA (A).
2. Export centralisé ou détaillé, correspondance de comptes (B).
3. Chaîne de suppléance et mode « structure réduite » avec revue a posteriori (C).
4. Collision d'IBAN par empreinte, entre socles (D).
5. Marché de denrées, commande quotidienne, facture récapitulative (P06).
6. Reprise attestée des IBAN fournisseurs (P01).
7. Devises sur la régie d'avances et les paiements (P08).
8. Déclaration d'intérêts et conventions réglementées (P10).
9. Rapport des validations et du chaînage pour le CAC (P12).
10. Quarantaine des factures d'expéditeurs inconnus ; bandeau anti-fraude (P14).

### 5.3 Ce que la passe a montré
La conception est solide sur le moteur d'écritures et les garde-fous ; elle sous-estimait **le régime de transition** (ventes ailleurs pendant un an), **la petite taille** des structures (deux personnes pour séparer des tâches) et **la restauration**, dont les achats ne ressemblent pas aux achats pédagogiques.

---

## 6. Décisions

### 6.1 Appliquées par défaut (D-031)
| # | Question | Décision |
|---|---|---|
| Q-CG24 | Journaux propres à ScolIA pendant le doublon | Oui (R-CG43). |
| Q-CG25 | Export centralisé par défaut ? | **Centralisé par jour** pour les ventes et les encaissements des familles, **détaillé** pour les achats (S-4823). |
| Q-CG26 | Structure réduite | Permise avec revue a posteriori mensuelle (S-4825). |
| Q-CG27 | Collision d'IBAN | Empreinte HMAC, alerte direction et trésorier (R-CG47). |

### 6.2 Soumis à Florian
| # | Question | Recommandation |
|---|---|---|
| Q-CG22 | Conservation des originaux papier des factures (numérisation fidèle) | demander au cabinet ; original conservé par défaut |
| Q-CG23 | Contrat avec le cabinet : rôle RGPD, accès, données transmises | DPO et cabinet |
| Q-CG4 (rappel) | Plateforme agréée : **non-conformité actuelle possible** | à lever avant le doublon |
| Q-CG11 (précisé) | Achats et budget en **réel** pendant le doublon, journaux `HA-S` et `BQ-S` | oui, après trois mois de preuves |

---

## 7. Faits, hypothèses, contradictions
**Faits** : la conception relue ; aucun code exécuté. **Hypothèses [H]** : pratiques des cuisines scolaires (mercuriale, factures récapitulatives), taille des bureaux d'OGEC, préférences des cabinets. **Contradiction** : R-CG35 (mode parallèle pour toutes les écritures) est **remplacée** pour les achats par le bloquant A (§33 de la conception l'emporte).
