# Passe « personas » sur le socle 32 — Espaces de travail et bureautique

**Date :** 2026-11-11 — **Statut :** proposition, **non décidée** ; recommandations appliquées par défaut (D-031) sauf ce qui engage l'établissement à l'extérieur (§6.2). Aucun code écrit ni modifié ; aucun accès à une base, à un secret ni à un service de production ; Next.js lu seulement.
**Branche :** `ent-socle-32-33-conception`.
**Éprouve :** `docs/socles/32_BUREAUTIQUE.md` (§0 à §29). **Ajouts** versés dans la conception au **§30** (« Ajouts personas »), qui l'emporte en cas de contradiction ; rien n'est supprimé.
**Numérotation :** manques **M-3201 à M-3234** ; actions **ETR-268 à ETR-299** ; règles **R-ET39 à R-ET44** ; cas limites **CET-53 à CET-72**.

---

## 0. À lire en premier : quatre bloquants (recommandations appliquées par défaut, D-031)

| # | Point | Pourquoi c'est bloquant | Recommandation appliquée | Où |
|---|---|---|---|---|
| **A** | Le **travail de groupe à rendre** n'a pas de chemin : la conception sépare l'espace (32) et la remise (20), mais un exposé fait à quatre dans un espace de groupe n'a aucune façon d'être « rendu » une seule fois pour quatre (M-3201) | Sinon chaque élève dépose la même chose, ou le professeur reçoit un partage qui ne vaut pas remise (pas d'horodatage, pas d'échéance, pas de retour) | **Rendu au nom du groupe** : un dépôt au 20 de la version figée, une remise par membre du groupe à la date (R-ET39, ETR-268) | §30.1 A |
| **B** | Un animateur qui met un dossier à la corbeille **emporte les productions des élèves** (M-3205) | Perte du travail d'un élève, impossible à justifier à une famille ; la corbeille de 30 jours ne suffit pas si l'élève ne sait pas | Les productions d'élèves non figées vont dans « **Retrouvés** », visibles de leurs auteurs (R-ET40, ETR-274) | §30.1 B |
| **C** | L'**attaquant** qui dépose un document piégé vise deux cibles nouvelles : l'éditeur WASM (même origine que l'ENT) et le moteur de conversion serveur (LibreOffice, PDFium : code C/C++ qui lit des fichiers hostiles) (M-3218) | Une évasion du moteur de conversion donne l'accès au réseau interne ; une iframe de même origine pilotée par un XSS donne l'accès à la session | `ent-convert` en **conteneur séparé, sans réseau, filtre d'appels système** ; iframe en `sandbox` restreint ; relations externes des fichiers OOXML retirées (R-ET41, CET-67, CET-68) | §30.1 C |
| **D** | L'**auditeur RGPD** demande comment répondre à un droit d'accès d'un élève : les données sont dispersées (contributions, casier, portfolio, commentaires, journal) (M-3224) | Le délai d'un mois (RGPD art. 12) n'est pas tenable à la main | **ETR-280** : rassembler les données d'une personne en lot pour la procédure du socle 27, sans les contributions d'autrui | §30.1 D |

---

## 1. Méthode
Démarche des passes précédentes (`PERSONAS_42.md` §1) : quatorze personas construites à partir du §3 de la conception, couvrant public et privé sous contrat, collège et lycée, familles ordinaires et particulières, un **attaquant** et un **auditeur RGPD**. Grille : (1) ce que la personne fait aujourd'hui (Next.js M19, clé USB, mail) ; (2) ce qu'elle retrouve dans ScolIA ; (3) ce qui manque, ce qui la gêne, ce qu'elle redoute ; (4) cas limites ; (5) D-036 et E1–E14. Gravité : **bloquant**, **important**, **confort**, **différenciant**.
**Limites** : personas construites, non interviewées ; pratiques de terrain [H] ; aucune donnée réelle lue.

## 2. Les personas
| # | Persona | Établissement | Ce qu'elle éprouve surtout |
|---|---|---|---|
| P01 | **Mme Arnaud**, professeure de lettres, 5 classes, utilise M19 tous les jours | lycée pilote, privé sous contrat | reprise de M19, rentrée, distribution personnalisée |
| P02 | **M. Kessler**, professeur de technologie, travaux de groupe à 4 | collège public | groupes, rendu commun, vieux ordinateurs |
| P03 | **Mme Ibrahim**, professeure principale de 3ᵉ, coordonne l'équipe | collège public | espace d'équipe, tableur partagé, verrou |
| P04 | **Léna**, 14 ans, 4ᵉ, timide | collège public | collectif, commentaires, portfolio |
| P05 | **Théo**, 16 ans, 2ᵈᵉ, malvoyant, lecteur d'écran | lycée pilote | accessibilité de l'éditeur |
| P06 | **Mme Ferreira (AESH)**, accompagne Théo et un élève de 6ᵉ | lycée pilote et collège | accès minimal, supports adaptés |
| P07 | **M. et Mme Roussel**, séparés ; Mme Roussel a une interdiction de remise prononcée (socle 4) | lycée pilote | vue famille |
| P08 | **Inès**, 18 ans, terminale | lycée pilote | majorité, portfolio, partage entre élèves |
| P09 | **Mme Vidal**, directrice | lycée pilote | instances, confidentialité, départs |
| P10 | **M. Lopez**, secrétaire de direction, part à la retraite en juin | lycée pilote | transfert au départ |
| P11 | **Mme Chen**, professeure documentaliste | collège public | droit d'auteur, ressources GAR |
| P12 | **M. Blanc**, délégué à la protection des données (auditeur RGPD) | lycée pilote | conservation, droit d'accès, sensibilité |
| P13 | **« Zed »**, élève de 1ʳᵉ qui teste les limites, et un attaquant extérieur | lycée pilote | contournements, fichiers piégés |
| P14 | **Florian**, administrateur, le jour de la bascule de M19 | lycée pilote | reprise, comparaison, retour arrière |

## 3. Parcours et manques, persona par persona

### P01 — Mme Arnaud, lettres, utilisatrice quotidienne de M19
Aujourd'hui : un dossier par classe dans son cloud personnel, des dossiers partagés avec deux collègues, envoi des textes aux élèves par la messagerie de Charlemagne.
- Rentrée : retrouve ses 5 espaces de cours **proposés**, reconduit sa structure (ETR-102) ; ✔.
- **M-3202 (important)** : elle veut **préparer** ses espaces fin août sans que les élèves voient l'ébauche → ETR-271, R-ET42.
- **M-3203 (important)** : corrigé visible **après** le devoir sur table → visibilité programmée ETR-272.
- Distribution personnalisée de 32 copies scannées « NOM_Prénom.pdf » : rapprochement proposé (ETR-201) ✔ ; homonymes (CET-30) ✔.
- **M-3204 (confort)** : récapitulatif hebdomadaire de l'activité de ses espaces → ETR-296.
- Reprise : ses 2 300 fichiers et 2 partages reviennent (§26) ; elle veut savoir **ce qui n'a pas été repris** (macros) → liste au propriétaire ✔ (§26 point 3).

### P02 — M. Kessler, technologie, groupes de 4
- Crée 8 groupes dans l'espace du cours (ETR-104) ✔.
- **M-3201 (bloquant A)** : rendu commun → R-ET39.
- **M-3206 (important)** : « qui a fait quoi » dans un document de groupe → ETR-270 (versions par auteur).
- **M-3207 (important)** : pas de coédition : il veut découper le dossier technique en parties attribuées → ETR-269.
- **M-3208 (important)** : salle informatique aux postes anciens, sans WebGPU → CET-58 : éditeur annoncé indisponible, aperçu et dépôt ; à mesurer au lot ET5 (taille de 9 à 11 Mo par appli).
- CET-53, CET-54 ajoutés (départ d'un membre la veille, rendu sans prévenir).

### P03 — Mme Ibrahim, professeure principale, espace d'équipe
- Espace d'équipe pédagogique de la 3ᵉ B ✔ ; tableur de suivi des élèves partagé.
- **M-3209 (important)** : le tableur contient des remarques sur la santé d'un élève (« PAI, voir infirmière ») → alerte de contenu sensible (R-ET18) ✔ ; mais elle demande **où mettre** l'information : renvoi vers le dossier de l'élève (socles 34, 35) — le message de l'alerte doit nommer le bon lieu (ajout au §15.5 : lien « ouvrir le dossier de l'élève »). Appliqué.
- Verrou : un collègue garde le tableur ouvert toute la soirée → expiration 30 min (R-ET31) ✔ ; « demander la main » ✔.
- **M-3210 (confort)** : voir l'espace « comme un élève » avant de l'ouvrir → ETR-275, R-ET43.

### P04 — Léna, 14 ans, 4ᵉ
- Retrouve ses espaces de cours, son casier ✔.
- Commentaire moqueur sous son exposé : signalement ✔ (ETR-168), masquage par l'animateur ✔.
- **M-3211 (important)** : elle veut **retirer** sa propre contribution regrettable → « retiré par l'auteur » (CET-38) ✔, mais le contenu reste visible de l'animateur : cela doit être dit à l'élève (« ton professeur peut encore le voir »). Appliqué au §9 point 4.
- Portfolio : verse son carnet de lecture ; choisit « famille » ✔.
- **M-3212 (important)** : un professeur peut-il lui **proposer** un versement sans l'imposer ? ✔ ETR-221 ; l'élève doit pouvoir refuser sans que le professeur le voie comme un refus nominatif ? Recommandation : le professeur voit « proposition non acceptée » après 30 jours, pas « refusée » (adouci, appliqué).

### P05 — Théo, malvoyant, lecteur d'écran
- **M-3213 (bloquant pour lui, important pour le socle)** : l'éditeur rendu en canvas est probablement inaccessible → R-ET30 (alternative) ✔ ; ajout : **préférences d'éditeur** (ETR-277 : ouvrir par défaut en aperçu accessible et téléchargement) ; demande de version adaptée (ETR-278).
- **M-3214 (important)** : aperçu PDF « balisé » : les PDF produits par `ent-convert` doivent être **PDF/UA** quand la source le permet (Typst, Krilla) ; à vérifier au lot ET5 (Q-ET3).
- **M-3215 (confort)** : texte alternatif des images déposées → ETR-288.

### P06 — Mme Ferreira, AESH
- Accès aux espaces des cours de Théo en lecture et dépôt dans son casier (R-ET14) ✔.
- **M-3216 (important)** : elle accompagne aussi un élève de 6ᵉ **dans un autre établissement** : deux rattachements, deux espaces séparés (D-013) ✔ ; rien ne fuit.
- **M-3217 (important)** : un accusé d'ouverture ne doit pas être posé quand c'est elle qui ouvre le casier de Théo ✔ (§13 point 3).

### P07 — M. et Mme Roussel, séparés ; interdiction de remise côté Mme Roussel
- L'interdiction de **remise** (socle 4, sortie de l'établissement) n'est pas une interdiction d'**information** : Mme Roussel garde la vue famille sauf retrait d'autorité ou interdiction de communication. **Précision ajoutée** à R-ET34 : la vue famille suit les droits d'**information** du socle 4, pas les interdictions de remise. Appliqué.
- Les deux parents voient la même chose (R-ET34) ✔ ; ni l'un ni l'autre ne voit ce que l'autre a ouvert (comme R-V46 du 21) — **ajout** : aucun accusé d'ouverture par un parent n'est visible de l'autre parent. Appliqué.

### P08 — Inès, 18 ans
- Majorité : la vue famille du portfolio s'arrête sauf accord (R-ET35) ✔.
- Partage entre élèves majeurs (S-3209, défaut fermé) ✔.
- Départ en juin : export du portfolio (ETR-227) ✔ ; **M-3219 (important)** : elle veut l'export **avant** le départ, pour Parcoursup en mars → déjà possible à tout moment ✔ ; sélection pour un dossier (ETR-229) ✔.

### P09 — Mme Vidal, directrice
- Espace du conseil pédagogique ; projet d'établissement en versions ; émission du document validé (ETR-190) ✔.
- **M-3220 (important)** : un contentieux (harcèlement) exige de **geler** un espace de classe → ETR-281, CET-48 ✔.
- **M-3221 (important)** : elle veut lire l'espace d'un cours pour vérifier une plainte de famille → elle se fait **ajouter comme lectrice**, ce que les membres voient (R-ET2, §20.2) ✔ ; pas d'accès discret. Confirmé.
- Espaces dormants (ETR-282) ✔.

### P10 — M. Lopez, secrétaire, départ en retraite
- **M-3222 (important)** : les modèles de courrier du secrétariat sont dans **son** espace personnel depuis 15 ans → parcours de départ (ETR-265, ETR-290) : il désigne un repreneur pour les éléments choisis ✔ ; s'il refuse, ce qui est dans son espace personnel lui appartient (export) et ce qui est dans l'espace de service reste (CET-65) ✔.
- **M-3223 (confort)** : alerte J-30 avant la fin de son lien ✔ (`espaces.depart_transfert`).

### P11 — Mme Chen, documentaliste
- Ressources GAR en liens profonds ✔ ; marquage « sous licence » (ETR-128) ✔.
- **M-3225 (important)** : elle veut **voir** toutes les ressources marquées « sous licence » de l'établissement pour vérifier les usages → ETR-299.
- **M-3226 (important)** : l'impression d'une ressource sous licence relève du CFC (papier) : ETR-287 permis, avec rappel.
- Accords sectoriels non vérifiés après 2020 : Q-ET10 maintenue.

### P12 — M. Blanc, DPO (auditeur RGPD)
- **M-3224 (bloquant D)** : droit d'accès → ETR-280.
- **M-3227 (important)** : durée de conservation des productions d'élèves : la conception propose un effacement un an après la sortie ; il demande que l'**export soit proposé au moins deux fois** (à la sortie, puis un mois avant l'effacement) → appliqué au §14 point 5.
- **M-3228 (important)** : les **dérivés** (aperçus, texte extrait pour la recherche) contiennent les mêmes données que l'original : ils s'effacent avec la version (registre : reconstructibles, effacés avec la source) ✔ ; ajout explicite à R-ET6 : « les dérivés suivent la version source ».
- **M-3229 (important)** : la liste de mots de l'alerte de contenu sensible est-elle un traitement de données de santé ? Non : elle **ne conserve** qu'une alerte (cible, liste, date), sans extrait ; le contenu n'est ni copié ni transmis (R-ET18) ✔.
- Sous-traitants : aucun hors l'hébergeur ; ArtCraft est un logiciel exécuté chez l'utilisateur, pas un service ✔.
- AIPD : les espaces entrent dans l'AIPD de l'ENT (Q-ET7).

### P13 — « Zed » et l'attaquant
- Zed renomme des dossiers pour écrire à une camarade (CET-36) ✔ ; ouvre l'URL de l'éditeur avec l'identifiant d'un autre document (CET-69) ✔ ; injecte un script dans un nom de fichier (CET-70) ✔ ; sature la file de conversion (CET-71) ✔.
- Attaquant extérieur : DOCX à relations externes (CET-67), ZIP renommé (CET-68), PDF piégé (désarmé par le socle 7) ; **M-3218 (bloquant C)** ✔.
- **M-3230 (important)** : envoi direct vers S3 avec taille signée : un client modifié peut-il réutiliser l'URL signée pour un **second** envoi ? → URL à usage unique de fait (clé d'objet unique, réservation consommée), objet supplémentaire rejeté à la réconciliation ✔ (§15.3).
- **M-3231 (important)** : lien externe fuité (CET-72) ✔.

### P14 — Florian, bascule de M19
- **M-3232 (important)** : comparaison par personne (fichiers, octets, partages) avant d'ouvrir ✔ (§26 point 5).
- **M-3233 (important)** : Next.js reste lisible après la bascule ; le gel en écriture est son geste ✔.
- **M-3234 (confort)** : message d'accueil à chaque personnel au premier passage (« votre espace a été repris ») ✔.

## 4. Manques qui relèvent d'autres socles
| Manque | Socle | Proposition |
|---|---|---|
| Remise au nom d'un groupe | 20 | accepter une remise multiple pointant une version figée (port `element_fige` + liste) |
| Droit d'accès rassemblé | 27 | procédure qui appelle ETR-280 |
| Versions adaptées (police, gros caractères) | 35 | aménagement « supports adaptés » qui ouvre le casier à l'AESH |
| Dossiers de continuité | 33 | ETR-284, ETR-285 |
| Alerte de contenu sensible → bon lieu | 34, 35 | lien vers le dossier de l'élève |

## 5. Vérification réglementaire
Les points du §5 de la conception ont été relus ; aucun changement, sauf la précision sur la vue famille (droits d'information du socle 4, P07).

## 6. Décisions
### 6.1 Appliquées par défaut (D-031)
Bloquants A à D ; préférences d'éditeur et alternative accessible ; « Retrouvés » ; préparation et programmation ; vue famille alignée sur les droits d'information ; aucun accusé d'un parent visible de l'autre ; export proposé deux fois ; dérivés effacés avec la source ; proposition de versement « non acceptée » plutôt que « refusée ».
### 6.2 Soumis à Florian
Q-ET1 (mise à jour de D-006), Q-ET2 (coédition), Q-ET3 (RGAA, PDF/UA), Q-ET7 (durées, DPO), Q-ET8 (usage réel et date de bascule de M19), Q-ET10 (accords sectoriels), Q-ET12 (coût du stockage).

## 7. Synthèse
34 manques (4 bloquants, 22 importants, 6 confort, 2 différenciants : ETR-269 parties de groupe, ETR-277 préférences) ; 32 actions, 6 règles et 20 cas limites ajoutés à la conception (§30) ; lots ET0, ET8 et ET11 élargis.
