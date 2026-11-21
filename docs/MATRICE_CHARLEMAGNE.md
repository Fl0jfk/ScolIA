# Matrice de couverture : Charlemagne, École Directe, Pronote, ENT usuels, application Next.js du porteur

**Date :** 2026-11-06 — **Branche :** `ent-matrice-charlemagne` — **Statut :** document de travail, **non décidé** (à relire par Florian). Aucun code. Rien compilé ni testé. Aucune base ni aucun secret de production consulté ; l'application Next.js n'a été **lue** qu'en lecture seule (`app/`, `app/api/`, quelques fichiers de `app/lib/`, `app/components/`), sans rien modifier.

**Pourquoi.** D-024 : la première version de ScolIA doit être au moins équivalente à Charlemagne, puis meilleure. Il faut une carte qui prouve qu'aucun module ni aucune fonction utile n'est oublié. Ce document rapproche (a) ce que font les logiciels du marché, (b) ce que fait déjà l'application Next.js du porteur (construite pour combler leurs manques), (c) la carte des socles (`docs/SOCLES.md`), et en tire la liste des **trous**.

Sommaire : §0 Lire ce document (méthode, limites) · §1 Ce qu'on sait de Charlemagne · §2 Légende · §3 La matrice (fonctions) · §4 Application Next.js : fonctions sans équivalent au marché · §5 Manques de terrain · §6 Registre des trous et où les ajouter · §7 Constats annexes · §8 Chiffres · §9 Sources.

---

## 0. Lire ce document : méthode et limites (à lire d'abord)

**Limite majeure, dite franchement.** Les sous-agents de recherche (Haiku) ont travaillé depuis une session dont le réseau **bloque la plupart des domaines** : `aplim.fr`, `ecoledirecte.com`, `index-education.com`, `education.gouv.fr`, `apps.apple.com`, des sites syndicaux, etc. Résultat :
- **Aucune page du site de l'éditeur de Charlemagne ni d'École Directe n'a pu être ouverte.** Aucune documentation ou aide en ligne de ces logiciels n'a été lue.
- Ce qui est dit de Charlemagne repose sur **(i)** des fiches de paquets de déploiement publiques (noms de modules, versions) et **(ii)** des extraits de moteurs de recherche. Les **noms des modules** sont donc assez solides ; les **fonctions détaillées** de chaque module ne le sont pas.
- Pour Pronote / ENT : extraits de recherche, guides d'établissements et d'académies, pour la plupart **non ouverts en entier**.
- Le recensement des manques de terrain (forums, avis) n'a pas atteint les forums d'enseignants, ni les avis des magasins d'applications : il repose surtout sur des protocoles d'établissements, des questions parlementaires, de la presse et des documentations d'éditeurs.
- **Conséquence : la matrice est une base de travail à compléter, pas un inventaire certifié.** La colonne « Vérif. » dit, ligne par ligne, ce qui est établi. Une **seconde passe est à faire** quand ces domaines seront autorisés (liste en §9.4). Ne pas conclure d'une absence de ligne qu'une fonction n'existe pas chez Charlemagne.

**Application Next.js.** Son inventaire a été fait par trois sous-agents Haiku en lecture seule (routes, types, libellés, chemins de fichiers). Je n'ai **pas relu moi-même chaque fichier** ; les chemins cités permettent de vérifier. Les permissions route par route n'ont pas été auditées.

**État des socles** : repris de `docs/SOCLES.md` (colonnes « Existant ») et des conceptions `docs/socles/02` à `06`. « Codé » ne veut **jamais** dire « validé pilote » (règle de la carte).

**Contradictions et trous documentaires relevés**
- `docs/ent-metier-checklist.md` §6 cite `docs/charlemagne-reference/` (catalogue de 659 tables, listes) : **ce dossier est absent de la branche** [F]. Les préfixes de tables cités par la checklist (`COM_`, `VS_`, `NO_`, `LSU_`, `INS_`, `FAC_`, `ADM_`, `PA_`, `TAB_`, `REC_`, `ENT2_`, `INF_`) sont repris ici comme **indices internes** (source : la checklist, elle-même issue d'un relevé local du porteur), non vérifiés à nouveau.
- Paie et comptabilité : la checklist les classe « DROP » ; la carte les garde en fin de parcours (Q-B, Q-C) ; **D-024 (équivalence avec Charlemagne) et les modules Paie et Comptabilité confirmés par les fiches de paquets rendent ce « DROP » à reconsidérer** (voir T-04, T-05).
- L'éditeur d'École Directe est donné tantôt comme Aplim, tantôt comme un autre éditeur selon les extraits : **à confirmer**. Le nom « Alcuin » ne correspond, dans les sources, qu'au répertoire d'installation ; « Charlemagne Host » n'a été trouvé nulle part.

---

## 1. Ce qu'on sait de Charlemagne

Éditeur : Aplim (Savoie), clientèle surtout enseignement privé catholique (chiffres de parc variables selon les sources : environ 3 000 établissements en 2020, plus de 5 500 selon une émission radio) [C2][C6].

**Modules (noms établis par des fiches de paquets de déploiement)** [C1] — niveau de preuve **V pour le nom, H pour le contenu** :

| Module | Contenu probable (H, à vérifier sur la documentation de l'éditeur) |
|---|---|
| Vie Scolaire | absences, retards, sanctions, appels, carnet, suivi |
| Passage | passages à la cantine / badge |
| Infirmerie | fiches santé, passages |
| Paie | paie du personnel et plannings (une presse locale évoque la paie « jusqu'aux plannings des enseignants ») [C5] |
| Emploi du temps | construction et consultation de l'emploi du temps |
| Compétences | livret de compétences, LSU |
| Comptabilité | comptabilité d'organisme de gestion ; citée pour les OGEC [C3] |
| Administratif | « Gestion administrative » : élèves, familles, inscriptions, facturation (H) |
| Entreprise | relations entreprises, stages (H) |
| Outils | utilitaires communs |
| Rectorat | échanges avec l'académie / le rectorat (H) |

Fonctions rapportées par des extraits : application mobile (contact administration, relevés de notes) ; paiement en ligne cantine, facturation, frais d'inscription (via l'espace famille de l'éditeur) [C2] ; hébergement du logiciel financé par un département dans un « plan » local [C4].

**Indices complémentaires (internes, H)** : préfixes `PA_PORTE_MONNAIE*`, `VS_PERMIS_*`, `REC_*` (échanges rectorat, ONDE, BE1D), `ENT2_CFA_*`, `ENT2_TAXE*` (checklist, §3.9, 3.4, 3.11, 3.12).

---

## 2. Légende

**Sources** : **C** Charlemagne · **E** École Directe · **P** Pronote / Index Éducation · **N** ENT usuels (SDET, Kosmos / MBN, ENT régionaux, Beneylu) · **X** application Next.js du porteur · **T** manque de terrain.

**Vérif.** (pour la colonne Sources) : **V** vérifié par une URL de fiche ou d'extrait (c'est un nom de module ou une fonction décrite) · **S** signalé par un extrait de recherche ou un tiers, page non ouverte · **H** supposé / déduit · **F** vu dans le code Next.js (chemin au §4, relevé par sous-agent, non relu en entier).

**État ScolIA** : **codé** (existant sur `ENT`, serveur, avec ancienne ou nouvelle interface ; pas « validé pilote ») · **partiel** (existant incomplet) · **branche** (code sur une branche non fusionnée) · **conçu** (document `docs/socles/NN`) · **carte** (le socle est nommé dans `SOCLES.md`, rien d'écrit sur cette fonction) · **TROU** (aucun socle ne la couvre : voir §6).

Dans la colonne « Trou » : un identifiant `T-xx` signale que la fonction est **à ajouter ou à préciser** (entièrement s'il s'agit d'un TROU, ou pour la partie manquante d'une fonction partiellement couverte).

---

## 3. La matrice

### 3.1 Structure, année, calendrier (socle 5)

| ID | Fonction | Sources | Socle | État | Trou |
|---|---|---|---|---|---|
| STR-01 | Établissement, sites, bâtiments, salles | C(H) P | 5 | codé | |
| STR-02 | Années scolaires, périodes, calendrier, vacances par zone | C(H) P | 5 | codé | |
| STR-03 | Grilles horaires nommées | C(H) P X(F) | 5 | codé | |
| STR-04 | Niveaux, MEF, formations, classes, groupes, ateliers | C(H) P | 5 | codé | |
| STR-05 | Matières, sous-matières, familles, coefficients par classe | C(H) X(F) | 5, 21 | codé | |
| STR-06 | Paramètres d'établissement, profil public / privé | C(H) | 5 | conçu | |
| STR-07 | Bascule d'année guidée (préparation, montée, ouverture, clôture) | C(H) P(S) T(G12) | 5 | conçu | |
| STR-08 | Contrôles avant clôture, réouverture limitée dans le temps | T(G12,G14) | 5 | conçu | |
| STR-09 | Annulation complète d'une bascule déjà faite (retour arrière) | T(G14,H) | 5 | partiel (réouverture seulement) | T-31 |
| STR-10 | Constitution des classes de rentrée (campagne, contraintes, scénarios, publication) | X(F) | 5 | conçu | |
| STR-11 | Souhaits des familles pour la constitution des classes (camarades, sans choix de professeur) | X(F) | 5 | conçu | |
| STR-12 | Contrainte « bilangue / option » par classe cible | X(F) | 5 | conçu | |
| STR-13 | Calendrier de l'établissement (fermetures, journées banalisées) | C(H) P | 5, 19 | codé | |
| STR-14 | Zones de vacances par site | X(F) | 5 | conçu | |

### 3.2 Élève, dossier, famille (socles 2, 4)

| ID | Fonction | Sources | Socle | État | Trou |
|---|---|---|---|---|---|
| ELE-01 | Identité de l'élève (état civil, INE, identifiants SIECLE / ONDE) | C(H) P E | 2 | partiel | |
| ELE-02 | Identité historisée (nom de naissance / d'usage) | T | 2 | conçu | |
| ELE-03 | Cycle de vie (prévu, inscrit, actif, sorti, archivé) | C(H) | 2 | conçu | |
| ELE-04 | Scolarité datée année par année (classe, groupes, options, régime) | C(H) | 2 | codé | |
| ELE-05 | Photos et trombinoscope, import de photos en masse | C(H) X(F) | 2 | codé | |
| ELE-06 | Fiche 360° et chronologie typée | X(F) | 2 | codé | |
| ELE-07 | Recherche d'élève, homonymes | C(H) X(F) | 2 | partiel | |
| ELE-08 | Fusion réversible de deux dossiers élèves | T | 2 | conçu | |
| ELE-09 | Passage à la majorité, recalcul des droits | T | 2 | conçu | |
| ELE-10 | Champs complémentaires paramétrables (informations complémentaires) | C(H) | 2 | codé | |
| ELE-11 | Journal des modifications du dossier | C(H) | 2, 27 | codé | |
| ELE-12 | Indicateur de complétude du dossier (« dossier vide / RAS ») | X(F) | 2, 4, 47 | conçu | |
| ELE-13 | Export complet d'un dossier en un geste (portabilité, droit d'accès, départ) | T(G10) | 2, 27 | carte | T-26 |
| ELE-14 | Responsables, liens d'autorité, contacts d'urgence, personnes autorisées | C(H) E | 4 | codé | |
| ELE-15 | Parents séparés : fiches et envois distincts, masquage des coordonnées de l'autre parent | T(G1,G2,G3) | 4, 16 | conçu | |
| ELE-16 | Suspension d'autorité sur décision de justice, pièce jointe | T | 4 | codé | |
| ELE-17 | Coordonnées historisées, changements proposés par la famille puis validés | T | 4 | conçu | |
| ELE-18 | Foyers, fratries, payeurs, répartition | C(H) | 4 | codé | |
| ELE-19 | Fusion de personnes | T | 4 | codé | |
| ELE-20 | Fiche de rentrée annuelle vérifiée en ligne, sans ressaisie | X(F) T(G6) | 4 | conçu | |
| ELE-21 | Accord des deux parents sur une démarche (conflit résolu par la direction) | X(F) | 4, 3 | conçu | |
| ELE-22 | Identifiants SIECLE des responsables à l'import | X(F) | 4, 9 | conçu | |
| ELE-23 | Photo élève / droit à l'image vérifié avant publication | T | 18, 2 | carte | |

### 3.3 Inscriptions, admission, rentrée (socle 3)

| ID | Fonction | Sources | Socle | État | Trou |
|---|---|---|---|---|---|
| INS-01 | Préinscription en ligne, pré-remplie | C(H) X(F) | 3 | branche | |
| INS-02 | Entonnoir de recrutement (demandes d'information, visites, taux de transformation) | X(F) | 3 | conçu | |
| INS-03 | Portes ouvertes : créneaux, confirmation, pointage à l'accueil, équipe par créneau | X(F) | 3 | conçu | |
| INS-04 | Rendez-vous d'inscription avec réservation provisoire, reconfirmation, changement de créneau | X(F) | 3 | conçu | |
| INS-05 | Agenda de direction externe (OAuth Google) pour les rendez-vous | X(F) | 3 | conçu (non repris) | |
| INS-06 | Pièces à fournir, réception, validation | C(H) | 3, 7 | codé | |
| INS-07 | Documents à signer (contrat de scolarisation, droit à l'image) | C(H) | 3, 7 | codé | |
| INS-08 | Étapes d'instruction paramétrables, motifs, liste d'attente | C(H) | 3 | branche | |
| INS-09 | Vœux (filières, options) | C(H) | 3 | codé | |
| INS-10 | Réinscription « en 3 gestes », non-réinscription | C(H) E | 3 | branche | |
| INS-11 | Dépôt de dossier de rentrée par les familles (formulaire public, pièces) | X(F) | 3, 4 | conçu | |
| INS-12 | Dépôt automatique de pièces avec lecture / classement par OCR | X(F) | 7, 28 | carte | T-13 |
| INS-13 | Départ, radiation, exeat, changement d'établissement | C(H) | 3 | partiel | |
| INS-14 | Effectifs prévisionnels de la rentrée | T | 3, 5 | conçu | |
| INS-15 | Simulateur public de coût de scolarité et de liste de fournitures | X(F) | 24, 3 | TROU | T-18 |
| INS-16 | Import des élèves depuis SIECLE (XML), depuis tableur | P X(F) | 9 | partiel | |

### 3.4 Vie scolaire : appel, absences, retards, sorties (socles 12, 14)

| ID | Fonction | Sources | Socle | État | Trou |
|---|---|---|---|---|---|
| VS-01 | Appel par séance (web, mobile), préremplissage | C(V mod.) E(S) P(S) X(F) | 12 | codé | |
| VS-02 | Verrouillage de l'appel après délai, corrections encadrées | T(G20) | 12 | carte | |
| VS-03 | Vérification des appels non faits, relances | C(H) | 12 | codé | |
| VS-04 | Absences à la séance, demi-journées dérivées | C(H) | 12 | codé | |
| VS-05 | Retards, billets d'entrée | C(H) T(G21) | 12 | codé | |
| VS-06 | Déclaration par la famille, justificatifs en ligne, statut suivi | E(S) T(G22) X(F) | 12 | codé | |
| VS-07 | Relance automatique des absences non justifiées | T(G27) | 12, 8 | carte | |
| VS-08 | Motifs, décisions, régularisation | C(H) | 12 | codé | |
| VS-09 | Dispenses (EPS, médicales), prise en compte à l'appel | C(H) | 12, 34 | codé | |
| VS-10 | Compteurs, seuils, signalement d'assiduité à l'autorité académique | T(G23) | 12 | carte | |
| VS-11 | Absence déclarée par l'accueil (téléphone parent) | X(F) | 12, 14 | codé | |
| VS-12 | Élève en sortie scolaire impossible à déclarer absent | X(F) | 12, 42 | carte | |
| VS-13 | Tableau de présence du jour, vue vie scolaire | X(F) | 12 | codé | |
| VS-14 | Calendrier vie scolaire (créneaux, seed par défaut) | X(F) | 5, 12 | codé | |
| VS-15 | Statistiques d'assiduité | C(H) | 12, 47 | carte | |
| VS-16 | Autorisations de sortie (permanente, exception, badge) | C(H) | 14 | codé | |
| VS-17 | Registre des visiteurs, badge visiteur | P(S) | 14 | carte | |
| VS-18 | Remise de l'élève à une personne autorisée | T | 14, 4 | carte | |
| VS-19 | Mise en sûreté / évacuation, liste des présents en temps réel | T | 14 | carte | |
| VS-20 | Carnet / observations / suivi de l'élève | C(H) X(F) | 13 | codé | |
| VS-21 | Messages au foyer par la vie scolaire, avec réglages | X(F) | 16 | codé | |

### 3.5 Discipline et suivi éducatif (socle 13)

| ID | Fonction | Sources | Socle | État | Trou |
|---|---|---|---|---|---|
| DIS-01 | Incidents, punitions, sanctions | C(H) E(S) P(S) X(F) | 13 | codé | |
| DIS-02 | Retenues planifiées (salle, surveillant) | C(H) | 13 | codé | |
| DIS-03 | Conseils de discipline, commissions éducatives | C(H) | 13 | carte | |
| DIS-04 | Alertes CPE, détection précoce | C(H) | 13, 31 | codé | |
| DIS-05 | Effacement planifié des sanctions | T | 13, 27 | carte | |
| DIS-06 | Permis à points / système de points | C(H) | 13 | TROU | T-24 |
| DIS-07 | Éditions vie scolaire (courriers, relevés) | C(H) | 13, 7 | codé | |
| DIS-08 | Protocole de signalement de harcèlement | T | 13, 36 | carte | |

### 3.6 Emploi du temps, remplacements (socles 10, 15)

| ID | Fonction | Sources | Socle | État | Trou |
|---|---|---|---|---|---|
| EDT-01 | Consultation par professeur, classe, élève, salle | C(V mod.) E(S) P(S) X(F) | 10 | codé | |
| EDT-02 | Créneaux, semaines A/B, séances datées | C(V mod.) P | 10 | codé | |
| EDT-03 | Exceptions : annulation, déplacement, changement de salle | E(S) P(S) | 10 | codé | |
| EDT-04 | Imports depuis le logiciel d'emploi du temps | P(S) | 10, 9 | codé | |
| EDT-05 | iCal / abonnement agenda | X(F) | 10 | codé | |
| EDT-06 | Annulation en masse (grève, intempéries) avec effets automatiques | T | 10 | carte | |
| EDT-07 | **Construction de l'emploi du temps** (contraintes dures et souples, génération) | C(V mod.) P(S) | 10 ou N2 | TROU | T-03 |
| EDT-08 | Contraintes d'établissement saisies (disponibilités, salles) | P(S) | 10 | partiel | T-03 |
| EDT-09 | Table de remplacement interactive | P(S) | 15 | partiel | |
| EDT-10 | Absences des personnels : déclaration, validation, séances touchées | C(H) P X(F) | 15 | codé | |
| EDT-11 | Rattrapage des cours après absence d'un professeur (créneaux proposés, relance) | X(F) | 15, 10 | TROU | T-14 |
| EDT-12 | Information des familles d'une absence d'enseignant, sans motif | E(S) | 15, 8 | carte | |
| EDT-13 | Absence du personnel à partir d'un PDF (lecture automatique, tri, rattachement) | X(F) | 15, 7 | carte | T-13 |
| EDT-14 | Calendrier des absences des professeurs, impression du résumé du jour | X(F) | 15 | partiel | |
| EDT-15 | Enseignements transversaux : sessions, intervenants, validation, contraintes | X(F) | 10, 6 | TROU | T-15 |
| EDT-16 | Planning enseignant personnel (« mon planning ») | X(F) | 10 | codé | |
| EDT-17 | Récapitulatif des présences des enseignants aux conseils | P(S) | 23, 6 | carte | |

### 3.7 Notes, compétences, bulletins, conseils (socles 21 à 23)

| ID | Fonction | Sources | Socle | État | Trou |
|---|---|---|---|---|---|
| NOT-01 | Évaluations, barèmes, coefficients, saisie en grille | C(H) E(S) P(S) X(F) | 21 | codé | |
| NOT-02 | Absent / dispensé / non noté distincts | C(H) | 21 | codé | |
| NOT-03 | Publication explicite ou différée aux familles | E(S) T(G28) | 21 | carte | |
| NOT-04 | Moyennes paramétrables (arrondis, sous-matières), rangs | C(H) | 21 | codé | |
| NOT-05 | Rattrapages, correction tracée après clôture | C(H) | 21 | carte | |
| NOT-06 | Statistiques par évaluation / classe | C(H) | 21 | carte | |
| NOT-07 | Saisie des notes dans l'application mobile | E(S) | 21, 11 | branche | |
| NOT-08 | Compétences et socle commun (référentiels, positionnement) | C(V mod.) P(S) | 22 | codé | |
| NOT-09 | LSU au format officiel | C(H) P(S) | 22 | partiel (CSV seulement) | |
| NOT-10 | Contrôle préalable d'export (rapport d'erreurs lisible avant dépôt LSU / Parcoursup) | P(S) T(G9) | 22, 9 | TROU | T-28 |
| NOT-11 | Appréciations par matière et générale | C(H) E(S) | 23 | codé | |
| NOT-12 | Préparation automatique du conseil de classe | X(F) | 23 | carte | |
| NOT-13 | Tenue du conseil en séance (tablette), mentions, avis | C(H) P(S) | 23 | carte | |
| NOT-14 | Décisions de passage, redoublement, orientation | C(H) | 23 | carte | |
| NOT-15 | Bulletin figé, versionné, publié aux familles | C(H) E(S) X(F) | 23 | codé | |
| NOT-16 | LSL, Notanet, Parcoursup (exports) | C(H) P(S) | 23, 37, 38 | partiel (CSV) | |
| NOT-17 | Fiches de dialogue : campagnes famille / élève / conseil avant le conseil de classe, accord des deux parents, conflit | X(F) | 23, 19 | TROU | T-11 |
| NOT-18 | Recours sur décision | T | 23 | carte | |
| NOT-19 | Pilotage de l'élève (vue documentaire par classe, par élève) | X(F) | 47, 2 | carte | |

### 3.8 Cahier de textes et travail personnel (socle 20)

| ID | Fonction | Sources | Socle | État | Trou |
|---|---|---|---|---|---|
| CAH-01 | Contenu des séances, travail à faire | E(S) P(S) N(S) X(F) | 20 | branche | |
| CAH-02 | Cohérence agenda / devoirs, devoirs saisis tardivement | T(G18) | 20 | carte | |
| CAH-03 | Entrées visibles par la famille (pas seulement par l'élève) | E(S) | 20, 29 | carte | |
| CAH-04 | Dépôt de devoirs en ligne, retour annoté | E(S) | 20 | branche | |
| CAH-05 | QCM, exercices auto-corrigés, bibliothèque | E(S) P(S) | 20 | carte | |
| CAH-06 | Charge de travail (éviter trois contrôles le même jour) | T | 20 | branche | |
| CAH-07 | Visa de la direction | N(S) | 20 | carte | |

### 3.9 Communication, messagerie, diffusion (socles 8, 16 à 19)

| ID | Fonction | Sources | Socle | État | Trou |
|---|---|---|---|---|---|
| COM-01 | Messagerie famille ↔ établissement / professeur | C(H) E(S) P(S) N X(F) | 16 | codé | |
| COM-02 | Matrice « qui peut écrire à qui » par rôle | X(F) | 16 | codé | |
| COM-03 | Messagerie interne temps réel : accusés, saisie en cours, réactions, présence, recherche | X(F) | 16 | codé | |
| COM-04 | Salons / canaux d'équipe | X(F) N(S) | 16, 17 | carte | |
| COM-05 | Appel audio / visio depuis une conversation | X(F) E(S) | 33 | carte | |
| COM-06 | Pièces jointes privées, désactivables par établissement | X(F) | 16, 7 | codé | |
| COM-07 | Carnet de correspondance numérique avec signature | C(H) | 16 | carte | |
| COM-08 | Annonces ciblées, actualités, suivi de lecture | N(S) | 17 | codé | |
| COM-09 | SMS ciblés aux seules familles concernées | T(G25) | 17, 8 | codé | |
| COM-10 | Notifications push, e-mail, SMS ; heures calmes, regroupement | E(S) | 8 | partiel | |
| COM-11 | « Pourquoi ai-je reçu ce message ? » | T(G26) | 8 | carte | |
| COM-12 | Sondages et questionnaires ciblés | E(S) P(S) | 18 | carte | |
| COM-13 | Autorisations récurrentes, consentements versionnés | T | 18 | carte | |
| COM-14 | Réunions parents-professeurs : plages automatiques, réservation par la famille | E(S) T(G29,G30) | 19 | carte | |
| COM-15 | Rendez-vous à l'initiative de la famille avec un enseignant | E(S) | 19 | carte | |
| COM-16 | Agenda de l'établissement, iCal | N(S) | 19 | partiel | |
| COM-17 | Blog / wiki / forum de classe, casier numérique | N(S) | 32 | carte | T-22 |
| COM-18 | Droit à la déconnexion (messages différés hors horaires) | P(S) T(G42) | 8, 16 | carte | |
| COM-19 | Événements avec invitation et réponse (RSVP), éligibles importés, doublons | X(F) | 19, 18 | TROU | T-16 |
| COM-20 | Offres de partenaires (écoles, entreprises) avec fenêtre d'inscription | X(F) | 17, 18 | TROU | T-16 |
| COM-21 | Site public de l'établissement (actualités, logo, pages publiques) | X(F) N(S) | 17 | TROU | T-17 |
| COM-22 | Tableau de bord « signaux » et feuille de semaine importée | X(F) | 29 | partiel | |

### 3.10 Documents, signatures, GED (socle 7)

| ID | Fonction | Sources | Socle | État | Trou |
|---|---|---|---|---|---|
| DOC-01 | GED de l'élève et des personnes (catégories, versions) | C(H) | 7 | codé | |
| DOC-02 | Stockage objet privé, URL pré-signées | X(F) | 7 | codé | |
| DOC-03 | Gabarits, publipostage, génération PDF (certificat de scolarité, courriers) | C(H) X(F) | 7 | partiel | |
| DOC-04 | Documents à signer, preuve de signature | C(H) | 7 | codé | |
| DOC-05 | Coffre-fort famille (factures, bulletins, attestations) | C(H) E(S) | 7 | carte | |
| DOC-06 | Certificat de scolarité en libre-service | T | 7 | carte | |
| DOC-07 | Attestations / diplômes délivrés par l'établissement : parcours, chaîne de signatures (professeur, direction), PDF, vérification publique par jeton | X(F) | 7, 22 | TROU | T-10 |
| DOC-08 | Classement automatique de PDF (découpage, extraction, rapprochement élève) | X(F) | 7, 28 | TROU | T-13 |
| DOC-09 | Espace documentaire partagé (dossiers, partage, quota) | X(F) N(S) | 32 | carte | |
| DOC-10 | Bureautique en ligne (traitement de texte, tableur, présentation) | X(F) | 32 | carte | |
| DOC-11 | Conversion de fichiers | T | 7 | carte | |

### 3.11 Facturation, paiements, aides (socles 24 à 26)

| ID | Fonction | Sources | Socle | État | Trou |
|---|---|---|---|---|---|
| FIN-01 | Articles, tarifs, formules, barèmes au quotient | C(H) | 24 | codé | |
| FIN-02 | Factures, avoirs, échéanciers, génération annuelle | C(H) X(F) | 24 | codé | |
| FIN-03 | Répartition multi-payeurs (garde alternée) | T | 24 | branche | |
| FIN-04 | Facturation des consommations par événement, au prorata | T | 24 | carte | |
| FIN-05 | Réductions (fratrie, personnel) | C(H) | 24 | partiel | |
| FIN-06 | TVA et catégories | C(H) | 24 | codé | |
| FIN-07 | Encaissements (chèque, espèces, virement), quittances | C(H) X(F) | 25 | codé | |
| FIN-08 | Prélèvements SEPA : mandats, fichiers, rejets | C(H) X(F) | 25 | codé | |
| FIN-09 | Paiement en ligne par carte | C(S) E(S) X(F) | 25 | partiel | |
| FIN-10 | Relances par paliers, plans d'apurement | C(H) T(G31) | 25 | codé | |
| FIN-11 | Médiation avant recouvrement | T(G32) | 25 | carte | T-27 |
| FIN-12 | Exclusion de cantine pour impayé sans lien avec le suivi social | T(G33) | 40, 25, 36 | carte | T-27 |
| FIN-13 | Rapprochement bancaire | C(H) | 25 | partiel | |
| FIN-14 | Solde et échéances visibles par la famille, PDF | E(S) X(F) | 25, 29 | codé | |
| FIN-15 | Bourses nationales, fonds social, aides locales | C(H) | 26 | partiel | |
| FIN-16 | Quotient familial | C(H) | 26 | codé | |
| FIN-17 | Porte-monnaie / compte prépayé (cantine) | C(H) | 40, 25 | TROU | T-23 |
| FIN-18 | Facturation de la restauration : forfait ou réel, tarif repas | X(F) | 24, 40 | codé | |

### 3.12 Restauration, internat, santé, accompagnement (socles 34 à 36, 40, 41)

| ID | Fonction | Sources | Socle | État | Trou |
|---|---|---|---|---|---|
| SAN-01 | Passages à la cantine, badge | C(V mod.) X(F) | 40 | codé | |
| SAN-02 | Menus et allergènes publiés aux familles | C(H) X(F) | 40 | codé | |
| SAN-03 | Réservation et annulation de repas par la famille | P(S) | 40 | codé | |
| SAN-04 | Prévision des repas, effectifs cuisine | X(F) | 40 | carte | |
| SAN-05 | Fiche santé chiffrée, vaccinations | C(V mod.) X(F) | 34 | codé | |
| SAN-06 | Passages à l'infirmerie, registre | C(V mod.) P(S) X(F) | 34 | codé | |
| SAN-07 | PAI, trousses, protocoles d'urgence | X(F) | 34 | codé | |
| SAN-08 | Médicaments et prises à l'infirmerie | X(F) | 34 | codé | |
| SAN-09 | Registre des accidents (élèves) | X(F) | 34 | codé | |
| SAN-10 | Inaptitudes EPS | X(F) | 34, 12 | codé | |
| SAN-11 | Extraits de fiche santé transmis pour les sorties | X(F) | 34, 42 | carte | |
| SAN-12 | Contraintes minimales publiées (PAI alimentaire) | T | 34, 40 | carte | |
| SAN-13 | Chambres, lits datés, affectations d'internat | C(H) X(F) | 41 | codé | |
| SAN-14 | Appel du soir, repas du soir, suivi | X(F) | 41 | codé | |
| SAN-15 | Sorties de week-end avec autorisation parentale par lien | X(F) | 41, 18 | codé | |
| SAN-16 | Activités, études, groupes d'étude d'internat, journal, incidents | X(F) | 41 | partiel | |
| SAN-17 | Planning des surveillants d'internat | X(F) | 41, 6 | carte | |
| SAN-18 | Résumé hebdomadaire envoyé aux parents d'internes | X(F) | 41, 8 | carte | |
| SAN-19 | Installation à l'internat : inscription publique, créneaux | X(F) | 41, 3 | carte | |
| SAN-20 | Statistiques d'internat et export | X(F) | 41, 47 | carte | |
| SAN-21 | Suivi psychologue, assistant social, dossier cloisonné | C(H) | 36 | codé | |
| SAN-22 | Signalement confidentiel par l'élève, écoute et orientation vers un référent | X(F) | 36, 13 | TROU | T-12 |
| SAN-23 | Informations préoccupantes | T | 36 | carte | |
| SAN-24 | Besoins particuliers (PAP, PPS, PPRE, AESH) | T | 35 | partiel | |

### 3.13 Orientation, examens, stages (socles 37 à 39, 51)

| ID | Fonction | Sources | Socle | État | Trou |
|---|---|---|---|---|---|
| ORI-01 | Orientation : intentions, vœux, avis du conseil, formulaire famille | P(S) | 37 | partiel | |
| ORI-02 | Parcoursup / Affelnet | C(H) P(S) | 37 | partiel | |
| EXA-01 | Examens blancs, salles, placements, surveillances, convocations | P(S) | 38 | carte | |
| EXA-02 | Résultats officiels (brevet, baccalauréat) | C(H) | 38 | carte | |
| STA-01 | Entreprises, tuteurs, offres | C(V mod.) X(F) | 39 | codé | |
| STA-02 | Conventions de stage : modèles, signatures à plusieurs parties, relances en masse | X(F) | 39, 7 | codé | |
| STA-03 | Candidature publique, pré-convention, dépôt par l'élève | X(F) | 39 | codé | |
| STA-04 | Périodes, référents, suivi, présence | X(F) | 39 | codé | |
| STA-05 | Repas et absences liés au stage | X(F) | 39, 40, 12 | carte | |
| STA-06 | Mode hors ligne des conventions, purge | X(F) | 39, 27 | carte | |
| STA-07 | Apprentissage (contrats, CFA, taxe d'apprentissage) | C(H) | 51 | carte | |

### 3.14 Voyages, sorties, vie de l'établissement (socles 42 à 46)

| ID | Fonction | Sources | Socle | État | Trou |
|---|---|---|---|---|---|
| VOY-01 | Projet de voyage simple / complexe, budget, validation par étapes | X(F) | 42 | carte | |
| VOY-02 | Circulaire aux familles, liste d'élèves, confirmation | X(F) | 42, 18 | carte | |
| VOY-03 | Devis de transport, relances, signature du devis | X(F) | 42 | carte | T-20 |
| VOY-04 | Commande cuisine / panier-repas pour un départ | X(F) | 42, 40 | carte | T-20 |
| VOY-05 | Récapitulatif pour l'association de parents, feuille de compta, export vers GED comptable | X(F) | 42, 48 | carte | T-20 |
| VOY-06 | Blog des parents pendant le voyage, suivi public par jeton | X(F) | 42, 17 | carte | T-20 |
| VOY-07 | Messages entrants d'e-mail rattachés à un voyage | X(F) | 42 | carte | T-20 |
| VOY-08 | Fiche d'urgence hors ligne, bilan et solde | T | 42 | carte | |
| VOY-09 | Clubs, association sportive, études, garderie | T | 43 | carte | |
| VOY-10 | Transports scolaires | C(H) | 43 | carte | |
| VOY-11 | Covoiturage entre familles ou personnels | X(F) | 43 | TROU | T-19 |
| VOY-12 | Réservation de salles et de matériel sans conflit avec l'emploi du temps | P(S) X(F) | 44 | partiel | |
| VOY-13 | Prêts (ordinateurs, manuels), clés, inventaire, maintenance | P(S) | 44 | carte | |
| VOY-14 | Catalogue CDI, prêts, retours | N(S) | 45 | carte | |
| VOY-15 | Élections (délégués, conseil d'administration, parents), vote secret | P(S) | 46 | carte | |
| VOY-16 | Secret Santa, jeux collectifs d'établissement | X(F) | — | écarté | |

### 3.15 Personnels, RH, administration (socles 6, 15, 49)

| ID | Fonction | Sources | Socle | État | Trou |
|---|---|---|---|---|---|
| PER-01 | Fiche personnel (fonctions, disciplines, quotité, statut) | C(H) X(F) | 6 | partiel | |
| PER-02 | Arrivées et départs, fin d'accès | T | 6 | conçu | |
| PER-03 | Annuaire interne, organigramme, « demander une tâche » | X(F) | 6 | conçu | |
| PER-04 | Services d'enseignement, professeur principal, équipes, référents | C(H) P | 6 | conçu | |
| PER-05 | Casquettes multiples (professeur-parent) | X(F) | 6 | conçu | |
| PER-06 | Habilitations utiles au quotidien (secourisme, encadrement de sortie) | X(F) | 6 | conçu | |
| PER-07 | Accueil d'un nouvel agent : lien d'onboarding public, pièces, signatures, validation, activation | X(F) | 6 | TROU | T-07 |
| PER-08 | Dépôt des documents RH (individuel et en lot, rattachement automatique) | X(F) | 6, 7 | carte | |
| PER-09 | Congés : types, demande, décision, annulation | X(F) | 15, 49 | carte | |
| PER-10 | Planning de service du personnel non enseignant (fixe / rotation, exceptions) | X(F) C(S) | 6, 15 | TROU | T-09 |
| PER-11 | Registre RH : alertes d'habilitation, formation, médecine, conformité, entretien | X(F) | 49 | TROU | T-06 |
| PER-12 | Santé et sécurité au travail : fiches, consultations, émargement, suivi | X(F) | 49 | TROU | T-06 |
| PER-13 | Entretiens professionnels, formations | X(F) | 49 | carte | |
| PER-14 | Pouls d'humeur quotidien anonyme du personnel | X(F) | 6, 47 | TROU | T-08 |
| PER-15 | Heures supplémentaires effectives : demande, acceptation, PDF | X(F) | 15, 49 | TROU | T-02 |
| PER-16 | Paie : périodes, éléments variables (heures, absences, primes, retenues) | C(V mod.) X(F) | 49 | carte | T-04 |
| PER-17 | Paie : bulletins, déclarations sociales, conventions collectives | C(V mod.) | 49 | TROU | T-04 |
| PER-18 | Import Excel du personnel, des enseignants | X(F) | 6, 9 | conçu | |
| PER-19 | Absences RH : justificatif, validation, clôture, réouverture | X(F) | 15 | codé | |
| PER-20 | Reconversion en arrêt maladie, traiteurs d'absence paramétrables | X(F) | 15 | carte | |

### 3.16 Comptabilité, budget, achats (socle 48)

| ID | Fonction | Sources | Socle | État | Trou |
|---|---|---|---|---|---|
| CPT-01 | Plan comptable, journaux, écritures, grand livre, balance | C(V mod.) C(S) | 48 | carte | T-05 |
| CPT-02 | Écritures générées depuis la facturation | C(H) | 48, 24 | carte | T-05 |
| CPT-03 | Clôture d'exercice, immobilisations | C(H) | 48 | carte | T-05 |
| CPT-04 | Budget prévisionnel, analytique (par sortie, activité) | C(H) | 48 | carte | |
| CPT-05 | Fournisseurs, commandes, factures fournisseurs | C(H) | 48 | carte | |
| CPT-06 | Caisse / banque simplifiées, dépenses, mouvements manuels | X(F) | 48 | carte | |
| CPT-07 | Export OGEC | C(H) | 48 | partiel | |

### 3.17 Pilotage, conformité, mise en service, IA (socles 27 à 31, 47)

| ID | Fonction | Sources | Socle | État | Trou |
|---|---|---|---|---|---|
| PIL-01 | Tableaux de bord de direction, indicateurs | X(F) | 47 | partiel | |
| PIL-02 | Remontées statistiques de rentrée, enquêtes | C(H) | 47, 9 | carte | T-25 |
| PIL-03 | Échanges avec le rectorat / la DDEC | C(V mod.) | 9 | carte | T-25 |
| PIL-04 | Registre des traitements, durées de conservation, purge | X(F) | 27 | partiel | |
| PIL-05 | Assistant guidé de conformité RGPD (questionnaire, score, documents applicables) | X(F) | 27 | carte | |
| PIL-06 | Registre et gestion des incidents / violations de données | X(F) | 27 | carte | |
| PIL-07 | Droit d'accès, rectification, journal des consultations visible | T(G4,G5) | 27 | carte | |
| PIL-08 | Création d'un établissement, assistant de paramétrage | X(F) | 28 | carte | |
| PIL-09 | Reprise des données depuis Charlemagne / École Directe / Pronote / SIECLE | T | 28 | carte | |
| PIL-10 | Support (ticket avec zone, pièces, référence, e-mail de confirmation) | X(F) | 28 | carte | T-01 |
| PIL-11 | Abonnement, souscription, facturation de l'établissement, suspension / réactivation | X(F) | 28 | carte | |
| PIL-12 | Assistant IA dans le périmètre de l'utilisateur, actions confirmées | X(F) | 30 | partiel | |
| PIL-13 | Base de connaissances de l'assistant (texte, PDF, portée public / privé) | X(F) | 30 | carte | |
| PIL-14 | Automatismes et règles sans code | T | 31 | partiel | |
| PIL-15 | Authentification forte : passkey, SSO, sessions | X(F) | 1 | codé | |
| PIL-16 | Accès par établissement ; règles de visibilité par rôle | X(F) | 1 | codé | |
| PIL-17 | Application mobile unique pour tous les rôles | E(S) C(S) | 11 | branche | |
| PIL-18 | Accessibilité (RGAA), hors ligne | T | 29, 11 | carte | |
| PIL-19 | Page d'accueil par rôle, à-faire unifié, recherche globale | X(F) | 29 | partiel | |
| PIL-20 | Fonctions d'identité nationale : ÉduConnect, GAR, schéma directeur | P(S) N(S) | 9 | partiel | |

### 3.18 Demandes, services internes, divers (aucun socle)

| ID | Fonction | Sources | Socle | État | Trou |
|---|---|---|---|---|---|
| DEM-01 | Guichet de demandes internes avec routage par service, prise en charge, libération, délégation, transmission à la direction | X(F) | — | TROU | T-01 |
| DEM-02 | Demande d'un membre du personnel : texte, pièces, confirmation par e-mail, page de suivi | X(F) | — | TROU | T-01 |
| DEM-03 | Demande des parents via un portail dédié (activable) | X(F) | 16, 29 | TROU | T-01 |
| DEM-04 | Demandes de reprographie (copies couleur / noir et blanc) avec circuit de validation et prêt | X(F) | — | TROU | T-02 |
| DEM-05 | Demande d'heures supplémentaires effectives (voir PER-15) | X(F) | 15, 49 | TROU | T-02 |
| DEM-06 | Fournitures : éditeur de listes par classe, moteur de calcul | X(F) | — | TROU | T-18 |
| DEM-07 | Affiches / communication : génération et suggestion de texte | X(F) | 17, 30 | TROU | T-17 |
| DEM-08 | Météo dans le tableau de bord | X(F) | — | écarté | |
| DEM-09 | Établissements d'un même ensemble ou réseau, vue consolidée consentie | C(H) | hors carte | TROU | T-21 |

---

## 4. Application Next.js : où sont les preuves

Pour chaque module, quelques chemins (lecture seule) ; l'inventaire détaillé est dans les rapports de sous-agents (non conservés dans le dépôt), les lignes de la matrice portant « X(F) » renvoient à ces chemins.

| Module | Chemins |
|---|---|
| Absences du personnel | `app/(admin)/absences`, `app/api/absences/` (ingest, manual, processors), `app/lib/absences-*.ts`, `app/components/absences/` |
| Accueil, portes ouvertes | `app/api/accueil/`, `app/components/accueil/AccueilAbsencesClient.tsx`, `AccueilPortesOuvertesClient.tsx` |
| RH, personnel | `app/api/rh/` (espace, onboarding, planning, registre, sst, mood-pulse), `app/api/personnel/`, `app/lib/rh/`, `app/lib/personnel-types.ts` |
| Paie, compta | `app/api/paie/route.ts`, `app/api/compta/route.ts`, `app/components/paie/`, `app/components/compta/` |
| Certificats | `app/api/certificates/`, `app/lib/certificates-*.ts`, `app/certificates/verify/[token]` |
| Demandes | `app/api/requests/`, `app/(admin)/requests`, `app/(demandes)/`, `app/api/assistance/route.ts` |
| Fiches de dialogue | `app/api/fiches-dialogue/`, `app/(admin)/fiches-dialogue` |
| Bien-être, IA | `app/api/bien-etre/`, `app/lib/bien-etre-*.ts`, `app/api/chatbot/`, `app/api/agentIAOCR/` |
| Voyages, stages, internat, santé | `app/api/travels/`, `app/api/stages/`, `app/api/internat/`, `app/api/sante/` |
| Cantine | `app/api/passages/route.ts`, `app/lib/passages-facturation-shared.ts` |
| Répartition des classes | `app/lib/class-allocation-*.ts`, `app/api/toolbox/class-allocation/` |
| Photocopies, HSE, covoiturage | `app/api/photocopies-couleur/`, `app/api/demandes-hse/`, `app/api/covoiturage/` |
| Partenariats, invitations, planning transversal | `app/api/partenariats/`, `app/api/invitation/`, `app/api/domain-planning/` |
| Inscriptions | `app/api/eleves/`, `app/api/rdv-inscription/`, `app/api/rentree/`, `app/api/document-templates/` |
| Facturation | `app/api/facturation/`, `app/api/billing/` (EasyTransac) |
| Famille, vie scolaire, messagerie | `app/api/famille/`, `app/api/vie-scolaire/`, `app/api/messaging/`, `app/api/channels/` |
| Plateforme | `app/api/platform/`, `app/plateforme/`, `app/souscrire/` |
| RGPD | `app/api/rgpd/`, `app/components/rgpd/` |

Les conceptions des socles 2 à 6 contiennent déjà, chacune au §2.4, une revue de ce que Next.js fait (« à ne pas perdre ») ; cette matrice **n'y revient pas** pour les points déjà tranchés (ex. constitution des classes, STR-10 à 12).

---

## 5. Manques de terrain (ce que les logiciels ne font pas ou font mal)

Source : recensement documentaire, **fiabilité faible à moyenne** (voir §0). Colonne « Statut » : **V** extrait lu ou règle publiée ; **H** supposé. Les identifiants G renvoient aux lignes ci-dessus.

| G | Manque | Acteurs | Statut | Où dans ScolIA | Trou |
|---|---|---|---|---|---|
| G1–G3 | Parents séparés : coordonnées de l'autre parent exposées, accès par responsable lourds à créer | secrétariat, familles | V (règle) / H | socle 4, ELE-15 | |
| G4 | Effacement des données d'enfants difficile à obtenir | familles, DPO | V (titre) | socle 27 | |
| G5 | Surveillance perçue comme excessive | élèves, familles | V (titre) | socle 27 (transparence des consultations) | |
| G6 | Ressaisie entre base élèves et vie scolaire | secrétariat | V | socle 9 | |
| G7 | Réconciliation manuelle des absences entre logiciels | enseignants, CPE | V | socle 9, 12 | |
| G9 | Export LSU / Parcoursup bloqué par des données manquantes, signalées par une simple infobulle | secrétariat | V | socles 22, 9 | T-28 |
| G10 | Exports limités aux listes affichées, pas d'export complet d'un dossier | secrétariat | V (listes) / H | socles 2, 27 | T-26 |
| G12–G14 | Bascule d'année : perte de notes si l'export précède mal la bascule ; pas de retour arrière partagé | secrétariat, direction | V / H | socle 5 | T-31 |
| G13 | Familles voient l'année précédente jusqu'à la bascule, sans explication | familles | V | socle 5, 29 | |
| G16 | Documents et pièces jointes invisibles sur mobile | familles | V | socle 11 | |
| G17 | Récupération d'identifiants indirecte | familles | V | socle 1 | |
| G18 | Devoirs ajoutés au dernier moment, agenda incohérent | élèves, familles | V | socle 20 | |
| G19–G21 | Appel à chaque heure, verrouillage après un quart d'heure, circuit manuel des retards | enseignants, CPE, AED | V | socle 12 | |
| G22 | Justificatifs papier, pas de dépôt ni de suivi en ligne | familles, CPE | V / H | socle 12 | |
| G23 | Seuils de sanction et de signalement gérés à la main | CPE, direction | V / H | socles 12, 13 | |
| G24 | Messagerie famille-enseignant demandée mais parfois refusée | familles | V | socle 16 | |
| G25 | SMS ciblés aux seules familles d'élèves absents | CPE | V (outil concurrent) | socle 17 | |
| G26 | Notifications trop nombreuses | familles | H | socle 8 | |
| G27 | Relance automatique de justification | CPE | V (extrait) | socle 12 | |
| G28 | Publication des notes réclamée | familles | V | socle 21 | |
| G29–G30 | Réunions parents-professeurs complexes ; sondage de créneaux à choix unique | enseignants, familles séparées | V | socle 19 | |
| G31–G33 | Relances graduées absentes, huissier découvert tard, exclusion de cantine après un mois d'impayé | familles, mairies | V (règlement) / témoignage isolé | socles 25, 40 | T-27 |
| G34 | Plateforme de réservation cantine indisponible | familles | V | socle 40 | |
| G35 | Saisie de notes peu ergonomique | enseignants | V (avis de blog) | socle 21 | |
| G37–G39 | ENT régionaux : interface difficile, outils collaboratifs peu utilisés | tous | V (2014, 2018, 2019) | socles 29, 32 | |
| G42–G43 | Usage hors temps de service ; charge de saisie cumulée | enseignants | V / H | socles 8, 31 | T-29 |
| G44 | Responsabilité de l'établissement envers ses sous-traitants | DPO, direction | V (extrait) | socle 27 | |
| G15 | Secrétariat insuffisant (premier degré) | directeurs d'école | V | socles 31, 50 | |

Constats d'ordre stratégique (non fonctionnels) relevés par la Cour des comptes (duopole d'éditeurs, usage faible des outils collaboratifs) : **contexte seulement**, aucune fonction à en tirer sans autre analyse.

**Pas retenu** : avis d'agrégateurs incohérents ; une sanction CNIL citée par un blog juridique, non vérifiée.

---

## 6. Registre des trous et où les ajouter

Un **nouveau socle** est noté N1…N4 (numéros définitifs à donner par Florian, en tenant compte de la place déjà réservée au « socle 52 » solveur dans `SOCLES.md` §7).

| Trou | Objet | Preuve | Où l'ajouter [R] | Remarque |
|---|---|---|---|---|
| T-01 | **Guichet de demandes** internes et familles : routage par service, prise en charge, délégation, suivi, support | X(F) | **Nouveau socle N1 « Demandes et flux de travail »**, ou à défaut socle 29 (à-faire) + 31 (files) | Chaque établissement a des dizaines de demandes (clés, matériel, attestation, reprographie) ; sans N1 elles repartent en e-mail |
| T-02 | Demandes de services : reprographie, heures supplémentaires effectives | X(F) | Types de demandes de N1 ; HSE alimentent 15 puis 49 | |
| T-03 | **Construction de l'emploi du temps** (génération avec contraintes) | C(V mod.) P(S) | Socle 10 (lot « contraintes + génération ») ou **N2** | `SOCLES.md` §7 le met hors carte ; D-024 impose l'équivalence : **trancher Q-F plus tôt** |
| T-04 | **Paie** (bulletins, déclarations sociales, éléments variables) | C(V mod.) X(F, partielle) | Socle 49 | La checklist dit « DROP » ; D-024 et le module Paie vérifié par son nom invitent à le **garder** ; étudier le coût réglementaire (Q-B) |
| T-05 | **Comptabilité d'organisme** (OGEC) complète | C(V mod.) C(S) | Socle 48 | Idem (Q-C) ; le public reste interfacé (agent comptable) |
| T-06 | **Santé et sécurité au travail** : registre, fiches, médecine du travail, habilitations obligatoires, alertes | X(F) | Socle 49 (sous-domaine) ; habilitations de journée déjà au 6 | Obligation de l'employeur [H] ; absent des conceptions 06 (aucune occurrence) |
| T-07 | **Accueil d'un nouvel agent** : lien public, pièces, signatures, activation | X(F) | Socle 6 (arrivées) + 7 | La conception 06 traite l'arrivée mais pas ce parcours |
| T-08 | **Pouls d'humeur anonyme** du personnel | X(F) | Socle 47 ou 6 | À soumettre à Florian : risque de perception de surveillance |
| T-09 | **Planning de service** du personnel non enseignant (rotation, exceptions) | X(F) C(S) | Socle 6 (+ 15) | |
| T-10 | **Attestations et diplômes d'établissement** : chaîne de signatures, PDF, vérification publique | X(F) | Socle 7 (signatures) | |
| T-11 | **Fiches de dialogue** (cycle pré-conseil famille / élève / conseil, accord des deux parents) | X(F) | Socle 23, avec 19 | Déjà en production chez le porteur : à ne pas perdre |
| T-12 | **Signalement bien-être confidentiel** par l'élève et écoute | X(F) | Socle 36 (+ 13 harcèlement) | Un assistant d'écoute relèverait du socle 30, sous cadre strict |
| T-13 | **Classement automatique de pièces** (PDF découpé, extrait, rapproché d'un élève ou d'un agent) | X(F) | Socle 7 + 28 (reprise) | Le Next.js le fait avec un service externe : à refaire sans fournisseur non autorisé |
| T-14 | **Rattrapage des cours** des professeurs absents | X(F) | Socle 15 (+ 10) | |
| T-15 | **Interventions transversales** (sessions, intervenants, contraintes de niveau) | X(F) | Socle 10 (+ 6) | |
| T-16 | **Événements avec RSVP** (cérémonies, remise de diplômes) et **offres de partenaires** | X(F) | Socle 19 (+ 18) ; billetterie : hors carte | |
| T-17 | **Site public** de l'établissement, communication (affiches) | X(F) | Socle 17, ou **N3 « Présence publique »** | |
| T-18 | **Simulateurs publics** (tarifs, fournitures), listes de fournitures | X(F) | Socle 24 + 3 | |
| T-19 | **Covoiturage** | X(F) | Socle 43 | Faible priorité |
| T-20 | Voyages : circulaire, devis de transport, commande cuisine, récapitulatif association de parents, GED comptable, blog parents, e-mails entrants | X(F) | Socle 42 : à porter dans sa conception | Déjà utilisés en production |
| T-21 | **Ensemble scolaire / réseau** : plusieurs établissements, fratries communes | C(H) | Hors carte (§7) ; Q13 | La clientèle privée est souvent en ensemble (H) : à décider |
| T-22 | Blog, wiki, forum, casier numérique | N(S) | Socle 32 | |
| T-23 | **Porte-monnaie / compte prépayé** de cantine | C(H) indice | Socles 40 + 25 | Décision D-024 à prendre (la checklist dit « DROP ») |
| T-24 | **Permis à points** | C(H) indice | Socle 13 | La checklist a décidé « garder » |
| T-25 | **Échanges avec le rectorat / DDEC** et enquêtes de rentrée | C(V mod.) | Socle 9 (+ 47) | Contenu à préciser sur la documentation de l'éditeur |
| T-26 | **Export complet d'un dossier** (portabilité, droit d'accès, départ) | T(G10) | Socles 2 + 27 | |
| T-27 | **Garde-fous sociaux** : médiation avant recouvrement, exclusion de cantine liée au suivi social | T(G31–G33) | Socles 25, 40, 36 | |
| T-28 | **Pré-contrôle des exports officiels** avec rapport d'erreurs lisible | P(S) T(G9) | Socles 22 et 9 | |
| T-29 | **Charge de saisie** et droit à la déconnexion des enseignants (indicateurs) | T(G42) | Socles 8, 20, 47 | |
| T-30 | Sondage de créneaux / vote de date | T(G30) | Socle 18 | |
| T-31 | **Annulation d'une bascule d'année** complète | T(G14) | Socle 5 | La conception prévoit la réouverture limitée, pas le retour arrière total |

**Nouveaux socles proposés** : **N1** Demandes et flux de travail (T-01, T-02) · **N2** Construction de l'emploi du temps (T-03), seulement si Q-F est tranchée « oui » · **N3** Présence publique de l'établissement (T-17). Tout le reste se range dans un socle existant.

---

## 7. Constats annexes

Faits relevés **en passant** dans le code Next.js (aucune action prise ; hors périmètre de la matrice) :
- Le Next.js appelle un service météo externe (`app/api/weather/route.ts`, `api.open-meteo.com`) : à ne **pas** reprendre (règle « pas de fallback externe non autorisé ») ; matrice DEM-08.
- Le Next.js utilise TOTP pour le deuxième facteur et une bibliothèque tierce d'authentification ; l'ENT Rust a décidé passkey (D-009). Écart attendu, sans conséquence pour le Rust.
- Plusieurs routes à jeton (`app/api/billing/dunning/process`, `app/api/rdv-inscription/reconfirm-cron`, webhook de paiement) comparent un secret par simple égalité, et l'une accepte le secret dans le corps de la requête. **À confirmer** par un audit du Next.js que le porteur décidera (cette matrice n'y touche pas).
- Le Next.js envoie des saisies à un modèle d'IA externe (constitution des classes, bien-être, OCR). Pour la reprise, `05_STRUCTURE.md` §2.4 décide déjà de ne pas le faire pour les souhaits de famille ; T-12 et T-13 devront trancher.

---

## 8. Chiffres

Comptage des lignes de la matrice (§3), fait par script le 2026-11-06 :

| Mesure | Nombre |
|---|---|
| Fonctions recensées (lignes de §3) | **283** |
| … couvertes par au moins un socle de la carte | 251 |
| … dont **codé** (existant sur `ENT`, jamais « validé pilote ») | 86 |
| … dont **partiel** | 31 |
| … dont **branche** non fusionnée | 9 |
| … dont **conçu** (document `docs/socles/`) | 29 |
| … dont **carte** seulement (socle nommé, rien d'écrit sur la fonction) | 96 |
| … écartées volontairement (météo externe, jeux d'établissement) | 2 |
| **Trous entiers** (aucun socle) | **30 lignes** |
| Fonctions couvertes mais avec une partie à préciser (référence `T-xx`) | 20 |
| Trous distincts au registre (§6) | **31** (T-01 à T-31) |

Lecture honnête : « couvertes » veut dire qu'un socle **nommé** doit les porter, pas qu'elles sont faites ; seules 86 lignes sont déjà codées. Le nombre de lignes dépend de mon découpage ; il prouve que la carte a été parcourue, pas qu'elle est exhaustive (§0).

---

## 9. Sources

**9.1 Charlemagne** (niveau V pour les noms de modules, S pour les fonctions)
- [C1] Fiches de paquets de déploiement, éditeur Aplim, modules Vie Scolaire, Passage, Infirmerie, Paie, Emploi du temps, Compétences, Comptabilité, Administratif, Entreprise, Outils : `https://wapt.tranquil.it/store/en/details/tis-charlemagne-viescolaire_12.7.0.0-8_windows_10.0_PROD.wapt` et fiches sœurs `…tis-charlemagne-passage_7.0.0.0-6…`, `…infirmerie_3.1.1.0-7…`, `…paie_8.9.0.0-6…`, `…emploidutemps_6.4.1.0-6…`, `…competences_7.8.0.0-10…`, `…comptabilite_6.9.0.0-10…`, `…administratif_12.10.0.0-6…`, `…entreprise_2.0.3.0-7…`, `…outils_13.0.0.0-6…` (même hôte).
- [C2] Lettre d'information de l'éditeur, mars 2020 : `https://www.aplim.fr/ressources/communication/infomag/infomagaplim_202003.pdf` (extrait).
- [C3] Cabinet d'expertise, organismes de gestion : `https://www.crowe.com/fr/groupe-fideliance/industries/ogec` (extrait).
- [C4] Délibération départementale : `https://departement13.fr/uploads/delibs/R0WOE.pdf` (extrait).
- [C5] Presse locale : `https://www.lejournaldesentreprises.com/article/lediteur-de-logiciels-savoyard-aplim-lance-un-outil-pour-les-ecoles-primaires-et-se-diversifie-2123721` (extrait, non ouvert).
- [C6] Émission radio : `https://www.sudradio.fr/emission/cest-ca-la-france-395` (extrait).

**9.2 École Directe, Pronote, ENT** (extraits S ; aucune page éditeur ouverte)
- École Directe : tutoriel élèves `https://lewebpedagogique.com/loctendron4/files/2020/09/Tutoriel-Ecole-directe-El%C3%A8ves.pdf` ; réservation de rendez-vous `https://saint-michel-solesmes.com/wp-content/uploads/2024/10/2024.10.11-Procedure-de-prise-de-rendez-vous-reunions-parents-professeurs.pdf` ; tutoriel professeur d'académie `https://graaaf.web.ac-grenoble.fr/sites/default/files/Media/document/tutoriel_ecoledirecte_-_professeur.pdf` ; fiche de l'application `https://apps.apple.com/FR/app/id1296851886` ; règlement d'agenda `https://www.ecole-alsacienne.org/wp-content/uploads/sites/18/2020/12/Reglement-Annexe-Utilisation-de-lagenda-dEcoleDirecte-et-de-Google-Classroom.pdf`.
- Pronote : plaquette 2024 `https://www.index-education.com/contenu/telechargement/pn/v2024.0/pdf/PRONOTE2024_Plaquette-Fonctionnalites.pdf` ; cahier des nouveautés EDT 2025 `https://maj.index-education.com/contenu/telechargement/edt/v2025.0/pdf/CAHIER-NOUVEAUTES-EDT-PRONOTE-FR-2025.pdf` ; documentation Orientation `https://doc.index-education.com/en/pronote/pronote/O/Orientation.htm` ; guide de passage d'année `https://www.index-education.com/contenu/telechargement/pn/v2023.0/pdf/Transition-Annee-Scolaire-PRONOTE-FR-2024.pdf` ; guide EduConnect d'un lycée `https://lepine.lycee.ac-normandie.fr/IMG/pdf/educonnect_ent_lepine.pdf`.
- ENT : page ministère `https://www.education.gouv.fr/les-projets-numeriques-territoriaux-les-espaces-numeriques-de-travail-ent-450478` ; cahier des charges premier degré `https://cache.media.eduscol.education.fr/file/sdet/15/3/Annuaire_ENT1D_Specifications_v1.0_acom_228153.pdf` ; Mon Bureau Numérique `https://pedagogie.ac-strasbourg.fr/fileadmin/pedagogie/dane/Continuite_peda/Le_classeur.pdf` et `https://meuse.fr/proche/colleges/aller-au-college/mon-bureau-numerique-mbn/pdf` ; Hauts-de-France `https://informations-pratiques.org/articles/mon-compte-ent-hdf.html` ; Beneylu `https://ecole-st-jean-de-niost.etab.ac-lyon.fr/sitesjn2020/tutoriel_ent.pdf`.

**9.3 Manques de terrain** (principales)
- Protocoles de vie scolaire : `https://etab.ac-reunion.fr/clg-hippolyte-foucque/wp-content/uploads/sites/50/2024/10/PROTOCOLE-VS-2024-2025-1.pdf`, `https://miclot.college.ac-normandie.fr/IMG/pdf/1_protocole_vie_scolaire.pdf`.
- Autorité parentale : `https://elbeuf.circonscription.ac-normandie.fr/IMG/pdf/autorite_parentale_en_milieur_scolaire.pdf`.
- Export LSU : `https://docs.index-education.com/docs_en/en-pronote-support-page-502-1837-export-to-lsu-step-2-verify-that-data-can-be-exported.php`.
- Transition d'année en ENT : `https://dane.site.ac-lille.fr/wp-content/uploads/sites/8/2025/06/ENT-HDF-Transition-dannée-projets-2025-3-1.pdf`.
- Cantine et impayés : `https://www.verneuil-en-halatte.fr/wp-content/uploads/2023/02/Reglement-cantine-Verneuil-2.pdf`, `https://www.amf.asso.fr/documents-tarifs-la-cantine-pour-les-enfants-avec-panier-repas-fourni-par-la-famille/40810`.
- Cour des comptes (presse) : `https://maire-info.com/numerique-educatif-la-cour-des-comptes-epingle-un-concept-sans-strategie-article-23242`.
- Conseil d'établissement : `https://www.lyceesaintmarc.org/college-saint-louis/conseil-des-partenaires/2020_11_24_cr_cdp_saint_louis_4_3_valide.pdf/view`.
- Autres sources du rapport de terrain : liste complète conservée dans le rapport de l'agent (non versé).

**9.4 À refaire quand le réseau le permet** : autoriser `aplim.fr`, `ecoledirecte.com`, `index-education.com` (dont `doc.` et `maj.`), `education.gouv.fr`, `apps.apple.com`, `wapt.tranquil.it`, les sites syndicaux (SNES, SE-Unsa), puis ouvrir les fiches produit et les aides en ligne de Charlemagne et d'École Directe, passer les lignes « S » en « V », et chercher les forums (Café pédagogique, listes de CPE et de secrétaires) et les avis des magasins d'applications.
