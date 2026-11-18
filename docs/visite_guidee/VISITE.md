# Visite guidée de l'ENT ScolIA : état au 18 novembre 2026

Ce document décrit **ce que l'ENT Leptos fait aujourd'hui**, tel qu'un utilisateur le voit : tous les écrans accessibles à chacun des 15 profils de labo, avec les vraies actions métier jouées dans le navigateur (Chromium) sur des données fictives. Rien n'est corrigé : un écran qui plante, un refus ou un message « pas encore disponible » est consigné tel quel. **Aucun pourcentage** : c'est Florian qui juge.

**Conditions.** Base PostgreSQL locale de test (`ENT_EXIGER_BASE=1`), serveur `ent-web-server` compilé avec la feature `labo` (`cargo leptos build`, profil de développement, donc plus lent qu'en production : ne pas lire les temps de chargement comme une mesure), établissement de labo fictif créé par `/labo/bootstrap`. Données de départ du labo + une seule écriture hors API : le lien entre un compte et l'élève majeur (l'invitation d'élève renvoie 503 « messagerie non configurée » en labo). Les séances datées du cahier de textes ont été générées par l'API d'administration (`POST /referentiel/annees/{id}/seances`).

**Profils visités** : direction, secrétariat (administratif), CPE / vie scolaire, accueil, professeur, professeur principal, infirmerie, comptabilité, psychologue, personnel (AED / agent), parent (un enfant), parent (deux enfants), élève mineur, élève majeur, administrateur. Il n'existe pas, dans le labo, de profil « prof principal » distinct : on a désigné un professeur comme principal de la 4B (`PUT /referentiel/classes/{id}/professeur-principal`) et on a regardé ce qui change.

**Fiabilité des signaux ⚠.** Le script repère automatiquement les écrans presque vides, les messages « pas encore disponible », les alertes, les refus du serveur (403) et les écrans restés en « Chargement… ». Ce sont des indices : chaque signal est relu à la main dans « Absent ou visiblement mince ». Les pages ne sont photographiées qu'une fois le chargement terminé (25 s au plus).

## Chiffres de la visite

- **368 écrans** photographiés (1 280 px), dont 85 chemins distincts ; une capture à 375 px pour les écrans principaux.
- **43 actions** jouées (saisies, validations, envois, créations), chacune avec capture et résultat réel.
- Captures par domaine : accueil 78 · eleve 58 · inscriptions 15 · assiduite 87 · absences-personnels 114 · edt 46 · cahier 34 · evaluations 25 · messagerie 10 · agenda 34 · voyages 34 · demandes 81 · formulaires 23 · finances 15 · sante-accompagnement 37 · administration 32.
- Dossier : `docs/visite_guidee/2026-11/<domaine>/` ; nom : `<rôle>-<n°>-<écran ou action>[-375].png` (le numéro suit l'ordre de la visite du rôle).

## Retours de Florian pendant la visite (18-11)

**Design** : correct, classique, minimaliste (constat de Florian). **Barre de gauche** : trop d'entrées, à plat. À structurer en domaines et sous-domaines, avec la **même coquille pour tout le personnel** : seules les entrées visibles changent selon les droits (pas de « version vie scolaire » ni « version administratif »). **Signaux** (compteurs de choses à traiter) à prévoir sur tous les domaines, comme dans la version Next.js mais mieux faits ; le mécanisme doit exister avant d'ajouter d'autres modules.

**Gros domaines proposés par Florian** : administratif, notes, vie scolaire, comptabilité, paye, peut-être emploi du temps ; puis « services » (réservation de salles, voyages…) et « établissement » (communication, paramètres généraux). **Un module a un seul propriétaire mais plusieurs surfaces** : le module Absences existe en propre, et les absences se voient aussi dans la fiche élève (raccourci), dans Aujourd'hui, etc. Chaque module doit déclarer dès sa conception : domaine, écrans, surfaces, signaux, actions à portée de l'IA, notifications.

**Position de cette session** (à trancher par le pilote et Florian, non appliquée ici) : faire la structure maintenant plutôt qu'à la fin, en deux temps : (1) un squelette décidé en conception (carte des domaines, règle « une entrée = un domaine + un droit », une source serveur unique de signaux), (2) migration des écrans au fil de l'eau, une ligne par module. Détail dans la réponse faite à Florian pendant la session ; questions ouvertes : « paye » = préparation (états d'heures) ou paye complète ? « Pédagogie » (notes + cahier de textes) ou « Notes » seul ?

**Constats chiffrés de la visite qui appuient ce retour** : la barre compte 48 entrées pour la direction, 44 pour l'administrateur, 37 pour le secrétariat, 33 pour la CPE, 26 pour un professeur, 16 à 19 pour les familles et élèves. Le seul signal est le compteur de la messagerie. Autres constats de cohérence : le **professeur principal a exactement le même menu que le professeur** ; l'**élève majeur a le même menu que l'élève mineur** ; le parent à deux enfants a le même menu que le parent à un enfant ; la **psychologue n'a aucun écran propre** (son menu est celui de l'infirmerie sans « Infirmerie »).

### Entrées de la barre de gauche, par rôle (constat)

| Rôle | Entrées | Exemples |
|---|---|---|
| Direction | **48** | Aujourd'hui, Mes files, Recherche, Élèves, Supervision des files… |
| Secrétariat | **37** | Aujourd'hui, Mes files, Recherche, Élèves, Faire une demande… |
| CPE / vie scolaire | **33** | Aujourd'hui, Mes files, Recherche, Élèves, Faire une demande… |
| Accueil | **20** | Aujourd'hui, Recherche, Élèves, Faire une demande, Mes demandes… |
| Professeur | **26** | Aujourd'hui, Recherche, Élèves, Faire une demande, Mes demandes… |
| Infirmerie | **19** | Aujourd'hui, Recherche, Élèves, Faire une demande, Mes demandes… |
| Comptabilité | **21** | Aujourd'hui, Recherche, Élèves, Faire une demande, Mes demandes… |
| Psychologue | **18** | Aujourd'hui, Recherche, Élèves, Faire une demande, Mes demandes… |
| Personnel (AED/agent) | **17** | Aujourd'hui, Recherche, Élèves, Faire une demande, Mes demandes… |
| Parent | **19** | Aujourd'hui, À faire, Faire une demande, Mes demandes, Mon agenda… |
| Parent (2 enfants) | **19** | Aujourd'hui, À faire, Faire une demande, Mes demandes, Mon agenda… |
| Élève mineur | **16** | Aujourd'hui, À faire, Faire une demande, Mes demandes, Mon agenda… |
| Élève majeur | **16** | Aujourd'hui, À faire, Faire une demande, Mes demandes, Mon agenda… |
| Administrateur | **44** | Aujourd'hui, Mes files, Recherche, Élèves, Recherches bloquées… |
| Professeur principal | **26** | Aujourd'hui, Recherche, Élèves, Faire une demande, Mes demandes… |

Aucun regroupement : la liste est plate, dans l'ordre d'ajout des modules. Le seul signal chiffré visible dans la barre est le compteur de messagerie.

## Domaine par domaine

Légende des signaux : ⚠ = relevé automatiquement (écran presque vide, « pas encore disponible », alerte, refus du serveur, 5xx, écran resté en chargement) ; chaque ⚠ est ensuite confirmé ou écarté à la main dans « Absent ou mince ».

### 00. Accueil, à faire, recherche

**Écrans visités** (7)

| Écran | Titre | Rôles qui y accèdent | Captures | Signaux |
|---|---|---|---|---|
| `/` | Bonjour | Direction, Secrétariat, CPE / vie scolaire, Accueil, Professeur, Infirmerie, Comptabilité, Psychologue, Personnel (AED/agent), Parent, Parent (2 enfants), Élève mineur, Élève majeur, Administrateur, Professeur principal | [direction](2026-11/00-accueil/direction-01-accueil.png) · [375 px](2026-11/00-accueil/direction-02-accueil-375.png) | ⚠ écran presque vide ; message « pas encore disponible » |
| `/mes-files` | Mes files | Direction, Secrétariat, CPE / vie scolaire, Administrateur | [direction](2026-11/00-accueil/direction-03-mes-files.png) · [375 px](2026-11/00-accueil/direction-04-mes-files-375.png) |  |
| `/recherche` | Recherche | Direction, Secrétariat, CPE / vie scolaire, Accueil, Professeur, Infirmerie, Comptabilité, Psychologue, Personnel (AED/agent), Administrateur, Professeur principal | [direction](2026-11/00-accueil/direction-05-recherche.png) · [375 px](2026-11/00-accueil/direction-06-recherche-375.png) |  |
| `/supervision-files` | Supervision des files | Direction, Administrateur | [direction](2026-11/00-accueil/direction-09-supervision-des-files.png) · [375 px](2026-11/00-accueil/direction-10-supervision-des-files-375.png) |  |
| `/blocages-recherche` | Recherches bloquées | Direction, Administrateur | [direction](2026-11/00-accueil/direction-118-recherches-bloquees.png) | ⚠ appel serveur refusé (403) pendant le chargement |
| `/mes-liens` | Mes liens à confirmer | Professeur, Parent | [prof](2026-11/00-accueil/prof-50-mes-liens.png) |  |
| `/a-faire` | À faire | Parent, Parent (2 enfants), Élève mineur, Élève majeur | [parent](2026-11/00-accueil/parent-03-a-faire.png) · [375 px](2026-11/00-accueil/parent-04-a-faire-375.png) |  |

**Actions jouées** (1)

| Rôle | Action | Résultat | Retour de l'écran | Capture |
|---|---|---|---|---|
| Direction | palette « Rechercher ou agir » (Ctrl K) | OK |  | [image](2026-11/00-accueil/direction-121-palette-rechercher-ou-agir-ctrl-k.png) |

**Ce que chaque rôle peut faire (constaté)**

- Tous : page « Aujourd'hui » (journée, messages non lus, annonces). Professeur : séances du jour avec appel à faire. Parent : journée de l'enfant, « À faire », sortie à venir, dernières notes (affichées à la demande).
- Direction, CPE, secrétariat, administrateur : « Mes files », « Supervision des files », « Recherche » (palette « Rechercher ou agir », Ctrl K, jouée : elle propose des actions comme « Faire l'appel », « Donner une punition », « Préparer une sortie »).
- Familles et élèves : « À faire » (liste unique de ce que la famille doit faire).

**Absent ou visiblement mince**

- **Aujourd'hui est presque vide** pour l'infirmerie, la psychologue, la comptabilité et le personnel (« Voici votre journée. À la une : Aucune annonce »), sans carte de travail à faire ni signal.
- Aucun compteur de tâches (absences à traiter, justificatifs en attente, demandes en file) ni sur Aujourd'hui ni dans la barre, hormis les messages non lus.
- Les familles n'ont pas de « Messages » dans leur page d'accueil au-delà du compteur ; l'accueil parent contient « Les notes s'affichent à votre demande ».

### 01. Élèves et classes

**Écrans visités** (6)

| Écran | Titre | Rôles qui y accèdent | Captures | Signaux |
|---|---|---|---|---|
| `/eleves` | Élèves | Direction, Secrétariat, CPE / vie scolaire, Accueil, Professeur, Infirmerie, Comptabilité, Psychologue, Personnel (AED/agent), Administrateur, Professeur principal | [direction](2026-11/01-eleve/direction-07-eleves.png) · [375 px](2026-11/01-eleve/direction-08-eleves-375.png) |  |
| `/classes` | Classes | Direction, Secrétariat, CPE / vie scolaire, Accueil, Professeur, Infirmerie, Comptabilité, Psychologue, Personnel (AED/agent), Administrateur | [direction](2026-11/01-eleve/direction-33-classes.png) · [375 px](2026-11/01-eleve/direction-34-classes-375.png) |  |
| `/classes/:id` | 4B | Direction, Secrétariat, CPE / vie scolaire, Professeur, Professeur principal | [direction](2026-11/01-eleve/direction-97-fiche-classe-4b.png) |  |
| `/eleves/:id` | Harry POTTER | Direction, Secrétariat, CPE / vie scolaire | [direction](2026-11/01-eleve/direction-98-fiche-eleve-harry.png) |  |
| `/eleves/:id/dossier` | Harry POTTER | Direction, Secrétariat, CPE / vie scolaire | [direction](2026-11/01-eleve/direction-99-dossier-eleve-harry.png) | ⚠ appel serveur refusé (403) pendant le chargement |
| `/mes-classes` | Mes classes | Professeur, Professeur principal | [prof](2026-11/01-eleve/prof-23-mes-classes.png) · [375 px](2026-11/01-eleve/prof-24-mes-classes-375.png) |  |

**Actions jouées** (2)

| Rôle | Action | Résultat | Retour de l'écran | Capture |
|---|---|---|---|---|
| Direction | élèves : rechercher puis ouvrir une fiche | OK | Section Synthèse | [image](2026-11/01-eleve/direction-128-eleves-rechercher-puis-ouvrir-une-fiche.png) |
| Direction | élèves : dossier administratif complet | OK |  | [image](2026-11/01-eleve/direction-129-eleves-dossier-administratif-complet.png) |

**Ce que chaque rôle peut faire (constaté)**

- Direction, secrétariat, CPE, accueil, infirmerie, comptabilité, psychologue, personnel, professeurs : liste « Élèves » (recherche tolérante aux fautes, filtres : sans INE, majeur, redoublant, nouvel arrivant, arrivé en cours d'année), fiche élève (section Synthèse, tuiles avec raison de masquage), dossier administratif, liste et fiche de classe, « Mes classes » (professeur). Le serveur limite la liste au périmètre du profil.
- Joué : recherche « Granger » puis ouverture de la fiche, puis du dossier (direction).

**Absent ou visiblement mince**

- Aucune action de **modification de la fiche** jouable depuis un écran existant : changement de classe, de régime, sortie, fusion de doublons, protection d'un élève, contacts. Ces fonctions existent côté serveur (119 routes `eleves/*` sans écran).
- Pas de création d'élève ni d'import (Siècle, Pronote, fichier) à l'écran.
- La fiche affiche la synthèse et des tuiles ; **le raccourci « absences » vers le module d'assiduité** (principe « un module, plusieurs surfaces ») n'a pas été vérifié comme tuile cliquable.

**Backend sans écran** : 169 routes de `ent-api` sur 219 ne sont appelées par aucun écran Leptos (ex. : `/classes/{}/autorisations`, `/dossier/eleves/{}/interdictions`, `/dossier/interdictions/{}/lever`, `/dossier/eleves/{}/conflit-interet`, `/dossier/conflits-interet`, `/dossier/conflits-interet/{}/lever`). 

### 02. Inscriptions et réinscriptions

**Écrans visités** (2)

| Écran | Titre | Rôles qui y accèdent | Captures | Signaux |
|---|---|---|---|---|
| `/inscriptions` | Inscriptions | Direction, Secrétariat, Administrateur | [direction](2026-11/02-inscriptions/direction-87-inscriptions.png) · [375 px](2026-11/02-inscriptions/direction-88-inscriptions-375.png) |  |
| `/reinscription` | Réinscription | Direction, Secrétariat, Parent, Parent (2 enfants), Élève majeur | [direction](2026-11/02-inscriptions/direction-117-reinscription.png) | ⚠ écran presque vide ; appel serveur refusé (403) pendant le chargement |

**Actions jouées** (1)

| Rôle | Action | Résultat | Retour de l'écran | Capture |
|---|---|---|---|---|
| Direction | inscriptions : créer une campagne d'admission | OK |  | [image](2026-11/02-inscriptions/direction-126-inscriptions-creer-une-campagne-d-admission.png) |

**Ce que chaque rôle peut faire (constaté)**

- Direction, secrétariat : écran « Inscriptions » (campagnes d'admission et de réinscription, instruction des dossiers). Joué : création d'une campagne d'admission « Admissions 2027-2028 » (6e, 5e).
- Parent, élève majeur : « Réinscription » (aucune campagne ouverte : « L'établissement ouvrira la campagne en temps voulu »). Pré-inscription publique : écran existant (`/web/preinscription`), non joué (accès sans compte).

**Absent ou visiblement mince**

- Après création de la campagne, l'écran ne confirme pas par un message : à vérifier sur la capture.
- Pas d'écran d'**instruction d'un dossier** joué (pas de dossier candidat dans le labo) ; la réinscription côté famille est vide faute de campagne publiée.
- 26 routes d'inscription et 10 de réinscription n'ont pas d'écran (pièces, décisions, imports de dossiers).

**Backend sans écran** : 49 routes de `ent-api` sur 74 ne sont appelées par aucun écran Leptos (ex. : `/scolarite/eleves/{}/regime`, `/scolarite/eleves/{}/sortie`, `/inscriptions/campagnes/{}`, `/inscriptions/campagnes/{}/offres`, `/inscriptions/campagnes/{}/statut`, `/inscriptions/campagnes/{}/prolonger`). 

### 03. Vie scolaire : appel, absences, assiduité, discipline

**Écrans visités** (9)

| Écran | Titre | Rôles qui y accèdent | Captures | Signaux |
|---|---|---|---|---|
| `/appel` | Faire l'appel | Direction, Secrétariat, CPE / vie scolaire, Professeur, Administrateur, Professeur principal | [direction](2026-11/03-assiduite/direction-27-appel.png) · [375 px](2026-11/03-assiduite/direction-28-appel-375.png) |  |
| `/vie-scolaire` | Vie scolaire | Direction, Secrétariat, CPE / vie scolaire, Administrateur | [direction](2026-11/03-assiduite/direction-37-vie-scolaire.png) · [375 px](2026-11/03-assiduite/direction-38-vie-scolaire-375.png) |  |
| `/journee` | Journée | Direction, Secrétariat, CPE / vie scolaire, Accueil, Administrateur | [direction](2026-11/03-assiduite/direction-39-journee.png) · [375 px](2026-11/03-assiduite/direction-40-journee-375.png) |  |
| `/assiduite` | Assiduité | Direction, Secrétariat, CPE / vie scolaire, Administrateur | [direction](2026-11/03-assiduite/direction-41-assiduite.png) · [375 px](2026-11/03-assiduite/direction-42-assiduite-375.png) |  |
| `/assiduite/accueil` | Accueil : situation d'un élève | Direction, Secrétariat, CPE / vie scolaire, Accueil, Administrateur | [direction](2026-11/03-assiduite/direction-43-accueil-eleve.png) · [375 px](2026-11/03-assiduite/direction-44-accueil-eleve-375.png) |  |
| `/discipline` | Punitions et sanctions | Direction, Secrétariat, CPE / vie scolaire, Professeur, Administrateur, Professeur principal | [direction](2026-11/03-assiduite/direction-45-discipline.png) · [375 px](2026-11/03-assiduite/direction-46-discipline-375.png) |  |
| `/signalements` | Signalements d'assiduité | Direction, CPE / vie scolaire, Administrateur | [direction](2026-11/03-assiduite/direction-47-signalements.png) · [375 px](2026-11/03-assiduite/direction-48-signalements-375.png) | ⚠ alerte : Signalement : fonctions à venir (lot AS9) ScolIA prépare, la direction décide, un |
| `/assiduite/eleves/:id` | Dossier d'assiduité : Harry Potter | Direction, CPE / vie scolaire | [direction](2026-11/03-assiduite/direction-100-dossier-assiduite-harry.png) |  |
| `/absences` | Absences | Parent, Parent (2 enfants), Élève mineur, Élève majeur | [parent](2026-11/03-assiduite/parent-19-absences.png) · [375 px](2026-11/03-assiduite/parent-20-absences-375.png) |  |

**Actions jouées** (11)

| Rôle | Action | Résultat | Retour de l'écran | Capture |
|---|---|---|---|---|
| Professeur | appel : choisir la séance (liste du jour) | OK |  | [image](2026-11/03-assiduite/prof-51-appel-choisir-la-seance-liste-du-jour.png) |
| Professeur | appel : Harry absent, Ginny en retard | OK | Ginny Weasley : Retard | [image](2026-11/03-assiduite/prof-52-appel-harry-absent-ginny-en-retard.png) |
| Professeur | appel : valider « Appel fait » | OK | Appel fait à Mercredi 18 novembre, 19:16 Les absences et retards sont transmis à la vie scolaire ; les familles sont prévenues selon les règles de l'é | [image](2026-11/03-assiduite/prof-53-appel-valider-appel-fait.png) |
| Professeur | appel : corriger un statut après validation | OK | Appel fait à Mercredi 18 novembre, 19:16 Les absences et retards sont transmis à la vie scolaire ; les familles sont prévenues selon les règles de l'é | [image](2026-11/03-assiduite/prof-54-appel-corriger-un-statut-apres-validation.png) |
| Parent | absences : donner le motif d'une absence relevée | OK | Motif transmis : la vie scolaire va l'examiner. / 3 absences attendent un motif | [image](2026-11/03-assiduite/parent-44-absences-donner-le-motif-d-une-absence-relevee.png) |
| Parent | absences : déclarer une absence à l'avance | OK | Absence déclarée : la vie scolaire est prévenue. / Motif transmis : la vie scolaire va l'examiner. | [image](2026-11/03-assiduite/parent-45-absences-declarer-une-absence-a-l-avance.png) |
| CPE / vie scolaire | vie scolaire : examiner l'absence avec justificatif | OK |  | [image](2026-11/03-assiduite/cpe-61-vie-scolaire-examiner-l-absence-avec-justificati.png) |
| CPE / vie scolaire | vie scolaire : examiner l'absence de Harry | OK |  | [image](2026-11/03-assiduite/cpe-62-vie-scolaire-examiner-l-absence-de-harry.png) |
| CPE / vie scolaire | vie scolaire : accepter le justificatif | OK |  | [image](2026-11/03-assiduite/cpe-63-vie-scolaire-accepter-le-justificatif.png) |
| CPE / vie scolaire | discipline : rechercher l'élève | OK |  | [image](2026-11/03-assiduite/cpe-64-discipline-rechercher-l-eleve.png) |
| CPE / vie scolaire | discipline : ouvrir le dossier de l'élève | **refusé par le serveur** | Accès refusé Votre profil ne permet pas d'accéder à ces informations. | [image](2026-11/03-assiduite/cpe-65-discipline-ouvrir-le-dossier-de-l-eleve.png) |

**Ce que chaque rôle peut faire (constaté)**

- Professeur : **appel** complet jouable (séance d'un autre jour, présent / absent / retard avec minutes, « Appel fait » avec confirmation, **correction après validation**, historique par élève).
- Parent : **déclarer une absence à l'avance**, **donner le motif** d'une absence relevée (motif + précision, envoi à la vie scolaire).
- CPE / vie scolaire, direction, secrétariat : « Vie scolaire » (absences du jour, filtres, **examiner**, **accepter ou refuser le justificatif**, annuler l'absence d'un élève présent), « Journée », « Assiduité » et dossier d'assiduité d'un élève, « Accueil : élève », « Signalements », « Discipline » (punitions et sanctions).
- Élèves : « Absences » (même écran famille, élève mineur et majeur identiques).

**Absent ou visiblement mince**

- **Discipline** : la recherche d'élève s'ouvre mais l'ouverture du dossier par la CPE a été **refusée par le serveur** (« Accès refusé : votre profil ne permet pas d'accéder à ces informations ») : aucune punition ni sanction n'a pu être donnée dans cette visite (à vérifier : droit du profil CPE de labo ou défaut).
- **Signalements** : « fonctions à venir (lot AS9) » : rien n'est transmis, l'écran dit lui-même ce qui manque.
- Vie scolaire : « le serveur ne fournit pas encore de liste des retards du jour ».
- Accueil de l'établissement : l'écran « Accueil : élève » existe ; l'**appel du matin / retards à l'accueil / remise d'un élève** n'ont pas été jouables faute d'élève en situation.
- 71 routes sur 137 sans écran (qualification, requalification, décompte, vérification, familles…).

**Backend sans écran** : 71 routes de `ent-api` sur 137 ne sont appelées par aucun écran Leptos (ex. : `/assiduite/profils-academiques`, `/assiduite/motifs`, `/assiduite/motifs/{}`, `/assiduite/motifs/{}/versions`, `/assiduite/regles`, `/assiduite/regle-en-vigueur`). 

### 04. Absences des personnels, remplacements, heures

**Écrans visités** (12)

| Écran | Titre | Rôles qui y accèdent | Captures | Signaux |
|---|---|---|---|---|
| `/edt/remplacements` | Absences et remplacements | Direction, Secrétariat, CPE / vie scolaire, Administrateur | [direction](2026-11/04-absences-personnels/direction-57-remplacements.png) · [375 px](2026-11/04-absences-personnels/direction-58-remplacements-375.png) |  |
| `/delegations` | Délégations de droits | Direction, Secrétariat, CPE / vie scolaire, Accueil, Professeur, Infirmerie, Comptabilité, Psychologue, Personnel (AED/agent), Administrateur, Professeur principal | [direction](2026-11/04-absences-personnels/direction-59-delegations.png) · [375 px](2026-11/04-absences-personnels/direction-60-delegations-375.png) |  |
| `/personnel/absences` | Mes absences | Direction, Secrétariat, CPE / vie scolaire, Accueil, Professeur, Infirmerie, Comptabilité, Psychologue, Personnel (AED/agent), Administrateur, Professeur principal | [direction](2026-11/04-absences-personnels/direction-61-mes-absences.png) · [375 px](2026-11/04-absences-personnels/direction-62-mes-absences-375.png) |  |
| `/personnel/remplacements` | Mes remplacements | Direction, Secrétariat, CPE / vie scolaire, Accueil, Professeur, Infirmerie, Comptabilité, Psychologue, Personnel (AED/agent), Administrateur, Professeur principal | [direction](2026-11/04-absences-personnels/direction-63-mes-remplacements.png) · [375 px](2026-11/04-absences-personnels/direction-64-mes-remplacements-375.png) | ⚠ écran presque vide |
| `/personnel/instruction-absences` | Absences à instruire | Direction, Secrétariat, Comptabilité | [direction](2026-11/04-absences-personnels/direction-65-absences-a-instruire.png) · [375 px](2026-11/04-absences-personnels/direction-66-absences-a-instruire-375.png) | ⚠ appel serveur refusé (403) pendant le chargement |
| `/personnel/signaler-absence` | Signaler une absence | Direction, Secrétariat, CPE / vie scolaire, Accueil | [direction](2026-11/04-absences-personnels/direction-67-signaler-une-absence.png) · [375 px](2026-11/04-absences-personnels/direction-68-signaler-une-absence-375.png) |  |
| `/personnel/ce-matin` | Remplacements du jour | Direction, Secrétariat, CPE / vie scolaire | [direction](2026-11/04-absences-personnels/direction-69-remplacements-du-jour.png) · [375 px](2026-11/04-absences-personnels/direction-70-remplacements-du-jour-375.png) | ⚠ écran presque vide |
| `/personnel/etats-heures` | Heures et états mensuels | Direction, Secrétariat, Comptabilité | [direction](2026-11/04-absences-personnels/direction-71-heures-et-etats.png) · [375 px](2026-11/04-absences-personnels/direction-72-heures-et-etats-375.png) | ⚠ appel serveur refusé (403) pendant le chargement |
| `/personnel/rcd` | Remplacement de courte durée | Direction, Secrétariat | [direction](2026-11/04-absences-personnels/direction-73-plan-de-remplacement.png) · [375 px](2026-11/04-absences-personnels/direction-74-plan-de-remplacement-375.png) |  |
| `/personnel/parametrage-absences` | Réglages des absences | Direction, Administrateur | [direction](2026-11/04-absences-personnels/direction-75-reglages-des-absences.png) · [375 px](2026-11/04-absences-personnels/direction-76-reglages-des-absences-375.png) |  |
| `/personnel/mes-heures` | Mes heures | Direction, Professeur | [direction](2026-11/04-absences-personnels/direction-119-mes-heures.png) |  |
| `/personnel/absences/declarer` | Déclarer une absence | Professeur | [prof](2026-11/04-absences-personnels/prof-48-declarer-une-absence.png) |  |

**Actions jouées** (3)

| Rôle | Action | Résultat | Retour de l'écran | Capture |
|---|---|---|---|---|
| Professeur | absences du personnel : déclarer une absence demain | OK | Absence enregistrée. | [image](2026-11/04-absences-personnels/prof-60-absences-du-personnel-declarer-une-absence-demai.png) |
| Professeur | absences du personnel : voir mes absences | OK |  | [image](2026-11/04-absences-personnels/prof-61-absences-du-personnel-voir-mes-absences.png) |
| Direction | absences du personnel : file d'instruction | OK | 0 absence(s) peuvent être validées en lot (convocations et formations). | [image](2026-11/04-absences-personnels/direction-120-absences-du-personnel-file-d-instruction.png) |

**Ce que chaque rôle peut faire (constaté)**

- Professeur : **déclarer une absence** (aujourd'hui, demain ou dates, journée ou heures, nature, pièce jointe), « Mes absences », « Mes remplacements », « Mes heures ». Joué : déclaration d'absence pour demain.
- Direction, secrétariat, administrateur, comptabilité : « Absences à instruire » (files à valider / à traiter / échéances / séances à couvrir), « Remplacements du jour », « Plan de remplacement », « Heures et états mensuels », « Remplacement de courte durée », « Réglages des absences » ; « Délégations de droits » pour tous.
- Accueil et AED : « Signaler une absence » (signalement par un tiers).

**Absent ou visiblement mince**

- La file d'instruction de la direction affichait **0 à valider** après la déclaration du professeur : à vérifier (validation automatique ou absence non transmise).
- Plusieurs écrans ne s'ouvrent pas pour la comptabilité (403 sur l'instruction et les états d'heures alors que le menu les propose).
- La **paye** n'est pas un écran : seuls les états d'heures et les exports existent ; grèves, rattrapages, pièces, impact : 145 routes sans écran sur 180.

**Backend sans écran** : 148 routes de `ent-api` sur 186 ne sont appelées par aucun écran Leptos (ex. : `/personnels/absences/signalements`, `/personnels/absences/natures`, `/personnels/absences/moi`, `/personnels/absences/organisation`, `/personnels/absences/a-valider`, `/personnels/absences/non-remplacees`). Les routes `personnels/*` comptent aussi les missions et dossiers RH, au-delà des absences.

### 05. Emploi du temps

**Écrans visités** (3)

| Écran | Titre | Rôles qui y accèdent | Captures | Signaux |
|---|---|---|---|---|
| `/emploi-du-temps` | Semaine du 16 au 20 novembre | Direction, Secrétariat, CPE / vie scolaire, Professeur, Élève mineur, Élève majeur, Administrateur, Professeur principal | [direction](2026-11/05-edt/direction-23-emploi-du-temps.png) · [375 px](2026-11/05-edt/direction-24-emploi-du-temps-375.png) |  |
| `/edt/semaine` | Emploi du temps de la semaine | Direction, Secrétariat, CPE / vie scolaire, Accueil, Professeur, Infirmerie, Comptabilité, Psychologue, Personnel (AED/agent), Administrateur, Professeur principal | [direction](2026-11/05-edt/direction-25-edt-de-la-semaine.png) · [375 px](2026-11/05-edt/direction-26-edt-de-la-semaine-375.png) |  |
| `/changements-edt` | Changements d'emploi du temps | Parent, Parent (2 enfants), Élève mineur, Élève majeur | [parent](2026-11/05-edt/parent-11-changements-d-edt.png) · [375 px](2026-11/05-edt/parent-12-changements-d-edt-375.png) |  |

**Ce que chaque rôle peut faire (constaté)**

- Familles, élèves : « Emploi du temps », « Changements d'EDT ». Professeurs, CPE, direction : « Emploi du temps » (semaine), « EDT de la semaine » (vue établissement), remplacements.
- Tous les personnels : « EDT de la semaine ».

**Absent ou visiblement mince**

- Aucun écran de **construction ou de modification** de l'emploi du temps (import, publication, changement de salle ou de créneau) : l'emploi du temps du labo arrive par l'API ; 36 routes `edt/*` sans écran.
- Chargement lent en développement (plus de 7 s pour la CPE) ; les captures attendent la fin du chargement.

**Backend sans écran** : 36 routes de `ent-api` sur 51 ne sont appelées par aucun écran Leptos (ex. : `/edt/modifications`, `/edt/modifications/{}`, `/edt/corrections-passe`, `/edt/lots/apercu`, `/edt/lots`, `/edt/lots/{}`). 

### 06. Cahier de textes et travail à faire

**Écrans visités** (6)

| Écran | Titre | Rôles qui y accèdent | Captures | Signaux |
|---|---|---|---|---|
| `/cahier` | Semaine du Lundi 16 novembre au Dimanche 22 novembre | Direction, Professeur, Administrateur, Professeur principal | [direction](2026-11/06-cahier/direction-29-cahier-de-textes.png) · [375 px](2026-11/06-cahier/direction-30-cahier-de-textes-375.png) | ⚠ message « pas encore disponible » |
| `/cahier/continuite` | Continuité | Direction, Professeur, Administrateur, Professeur principal | [direction](2026-11/06-cahier/direction-31-continuite.png) · [375 px](2026-11/06-cahier/direction-32-continuite-375.png) |  |
| `/cahier/charge` | Charge des classes | Direction, Professeur | [direction](2026-11/06-cahier/direction-114-cahier-charge.png) |  |
| `/cahier/cours` | Cahier du cours | Professeur | [prof](2026-11/06-cahier/prof-43-cahier-du-cours.png) |  |
| `/cahier/travail` | Donner un travail | Professeur | [prof](2026-11/06-cahier/prof-44-donner-du-travail.png) |  |
| `/cahier/agenda` | Semaine du Lundi 16 novembre au Dimanche 22 novembre | Parent, Parent (2 enfants), Élève mineur, Élève majeur | [parent](2026-11/06-cahier/parent-15-mon-cahier.png) · [375 px](2026-11/06-cahier/parent-16-mon-cahier-375.png) | ⚠ reste en « Chargement… » après 25 s ; écran presque vide |

**Actions jouées** (5)

| Rôle | Action | Résultat | Retour de l'écran | Capture |
|---|---|---|---|---|
| Professeur | cahier : donner du travail à la classe (publié) | OK | Travail créé. / Exercices de potions, page 42 · exercices · pour le 2026-11-24 à 19:00 · publie | [image](2026-11/06-cahier/prof-55-cahier-donner-du-travail-a-la-classe-publie.png) |
| Professeur | cahier : ouvrir le cahier du cours | OK |  | [image](2026-11/06-cahier/prof-56-cahier-ouvrir-le-cahier-du-cours.png) |
| Professeur | cahier : charge de travail des classes | OK |  | [image](2026-11/06-cahier/prof-57-cahier-charge-de-travail-des-classes.png) |
| Élève mineur | élève : mon cahier (séances et contenus) | OK | Aucun travail en attente, 1 évaluation annoncée. | [image](2026-11/06-cahier/eleve-mineur-22-eleve-mon-cahier-seances-et-contenus.png) |
| Élève mineur | élève : semaine suivante (travail donné par le professeur) | OK | 1 travail encore à faire, 8 évaluations annoncées. | [image](2026-11/06-cahier/eleve-mineur-23-eleve-semaine-suivante-travail-donne-par-le-prof.png) |

**Ce que chaque rôle peut faire (constaté)**

- Professeur : cahier du cours, **donner du travail** (jouée : travail « Exercices de potions » publié pour le 24-11, durée, mode de rendu, usage de l'IA), charge de travail des classes, continuité, adaptations, AESH.
- Élève, parent : « Travail à faire » (agenda), « Mon cahier » (jouée : le travail publié apparaît en semaine suivante, avec les évaluations annoncées).

**Absent ou visiblement mince**

- Le cahier du professeur indique que les **séances datées sont générées par l'administration** : sans elles (génération faite ici par l'API), aucune consigne ne peut être donnée.
- **« Travail à faire » (`/web/agenda`) reste en « Chargement… »** pour l'élève mineur et vide pour l'élève majeur ; « Mon cahier » du parent s'est aussi vu en chargement : à rejouer, mais l'écran n'a pas abouti pendant la visite.
- Rendu de copies / suivi des rendus : écrans présents (`cahier/rendu`, `suivi`), non joués faute de rendu.

**Backend sans écran** : 30 routes de `ent-api` sur 97 ne sont appelées par aucun écran Leptos (ex. : `/cahier/travaux/{}/historique`, `/cahier/travaux/{}/pieces`, `/cahier/contenus/{}/pieces`, `/cahier/pieces/{}`, `/cahier/travaux/{}/retrait`, `/cahier/visas`). 

### 07. Évaluations, notes, bulletins

**Écrans visités** (3)

| Écran | Titre | Rôles qui y accèdent | Captures | Signaux |
|---|---|---|---|---|
| `/evaluations` | Évaluations | Direction, Secrétariat, CPE / vie scolaire, Professeur, Parent, Parent (2 enfants), Élève mineur, Élève majeur, Administrateur, Professeur principal | [direction](2026-11/07-evaluations/direction-49-evaluations.png) · [375 px](2026-11/07-evaluations/direction-50-evaluations-375.png) |  |
| `/evaluations/classes/:id` | Moyennes de la classe · ScolIA | Direction, CPE / vie scolaire, Professeur principal | [direction](2026-11/07-evaluations/direction-101-evaluations-de-la-classe-4b.png) | ⚠ écran presque vide |
| `/evaluations/eleves/:id` | Notes et bulletins | Direction | [direction](2026-11/07-evaluations/direction-102-evaluations-de-harry.png) |  |

**Actions jouées** (1)

| Rôle | Action | Résultat | Retour de l'écran | Capture |
|---|---|---|---|---|
| Professeur | évaluations : choisir la période et ouvrir la grille | **refusé par le serveur** | Action refusée : valeur invalide : service sans matière du référentiel. Réessayer | [image](2026-11/07-evaluations/prof-62-evaluations-choisir-la-periode-et-ouvrir-la-gril.png) |

**Ce que chaque rôle peut faire (constaté)**

- Professeur : « Évaluations » (périodes, mes enseignements, conseils de classe, bulletins), vue de classe, évaluations d'un élève.
- Parent, élève : « Évaluations » (consultation).

**Absent ou visiblement mince**

- **La grille de saisie des notes n'a pas pu s'ouvrir** : « Action refusée : valeur invalide : service sans matière du référentiel » (le service du professeur de labo n'a pas de matière dans le référentiel). Aucune note n'a donc été saisie dans cette visite : c'est le cœur du domaine.
- La vue de classe (4B) s'ouvre presque vide (moyennes sans notes).
- Bulletins, conseils de classe : écrans présents, non joués (pas de notes).

**Backend sans écran** : 14 routes de `ent-api` sur 37 ne sont appelées par aucun écran Leptos (ex. : `/evaluations/periodes/{}/journal`, `/evaluations/regles`, `/evaluations/coefficients`, `/evaluations/devoirs/{}/corriger`, `/evaluations/bilans-cycle`, `/evaluations/bilans-cycle/{}/valider`). 

### 08. Messagerie

**Écrans visités** (1)

| Écran | Titre | Rôles qui y accèdent | Captures | Signaux |
|---|---|---|---|---|
| `/messages` | Messages | Direction, Parent, Professeur principal | [direction](2026-11/08-messagerie/direction-85-messages.png) · [375 px](2026-11/08-messagerie/direction-86-messages-375.png) |  |

**Actions jouées** (4)

| Rôle | Action | Résultat | Retour de l'écran | Capture |
|---|---|---|---|---|
| Professeur | messagerie : boîte de réception du professeur | OK | 27 conversations | [image](2026-11/08-messagerie/prof-58-messagerie-boite-de-reception-du-professeur.png) |
| Professeur | messagerie : nouveau message (écran de rédaction) | OK |  | [image](2026-11/08-messagerie/prof-59-messagerie-nouveau-message-ecran-de-redaction.png) |
| Parent | messagerie : écrire à l'établissement (relire puis envoyer) | OK | Message envoyé. | [image](2026-11/08-messagerie/parent-46-messagerie-ecrire-a-l-etablissement-relire-puis-.png) |
| Secrétariat | messagerie : ouvrir la conversation d'une famille | OK |  | [image](2026-11/08-messagerie/administratif-70-messagerie-ouvrir-la-conversation-d-une-famille.png) |

**Ce que chaque rôle peut faire (constaté)**

- Tous (sauf accueil, personnel, psychologue, infirmerie et comptabilité, dont le menu **n'a pas d'entrée Messages**) : boîte de réception, recherche, filtre non lus, **nouveau message** (objet, texte, pièce jointe, relire puis envoyer). Joué : parent écrit à l'établissement ; le secrétariat ouvre la conversation.

**Absent ou visiblement mince**

- La **réponse** d'un professeur ou du secrétariat n'a pas été jouée (ouverture seulement).
- Cinq profils de personnel n'ont pas la messagerie dans le menu.
- Pas de signal par domaine : compteur global seulement.

**Backend sans écran** : 9 routes de `ent-api` sur 19 ne sont appelées par aucun écran Leptos (ex. : `/annonces/{}/confirmer`, `/annonces/{}/retirer`, `/messages/fils/{}/desarchiver`, `/messages/non-traites`, `/messages/{}/annuler`, `/messagerie/reglage`). 

### 09. Agenda, salles, rendez-vous

**Écrans visités** (5)

| Écran | Titre | Rôles qui y accèdent | Captures | Signaux |
|---|---|---|---|---|
| `/mon-agenda` | Mon agenda | Direction, Parent, Professeur principal | [direction](2026-11/09-agenda/direction-21-mon-agenda.png) · [375 px](2026-11/09-agenda/direction-22-mon-agenda-375.png) | ⚠ message « pas encore disponible » ; alerte : Sources non affichées : échéance (pas encore branchée (travaux du cahier de texte |
| `/rencontres` | Rencontres parents-professeurs | Direction, Administrateur | [direction](2026-11/09-agenda/direction-81-rencontres.png) · [375 px](2026-11/09-agenda/direction-82-rencontres-375.png) |  |
| `/salles` | Réserver une salle | Direction, Secrétariat, CPE / vie scolaire, Professeur | [direction](2026-11/09-agenda/direction-113-salles.png) | ⚠ message « pas encore disponible » |
| `/rendez-vous` | Rendez-vous avec les enseignants | Professeur, Parent, Parent (2 enfants), Professeur principal | [prof](2026-11/09-agenda/prof-37-rendez-vous.png) · [375 px](2026-11/09-agenda/prof-38-rendez-vous-375.png) |  |
| `/agenda` | Semaine du Lundi 16 novembre au Dimanche 22 novembre | Parent, Parent (2 enfants), Élève mineur, Élève majeur | [parent](2026-11/09-agenda/parent-13-travail-a-faire.png) · [375 px](2026-11/09-agenda/parent-14-travail-a-faire-375.png) | ⚠ reste en « Chargement… » après 25 s ; écran presque vide |

**Actions jouées** (3)

| Rôle | Action | Résultat | Retour de l'écran | Capture |
|---|---|---|---|---|
| Professeur | agenda : créer un événement (brouillon) | OK | Semaine du 16 au 22 novembre / Sources non affichées : échéance (pas encore branchée (travaux du cahier de textes, échéances de l'établissement)) ; ag | [image](2026-11/09-agenda/prof-63-agenda-creer-un-evenement-brouillon.png) |
| Élève mineur | élève : travail à faire (agenda) | OK |  | [image](2026-11/09-agenda/eleve-mineur-21-eleve-travail-a-faire-agenda.png) |
| Direction | rencontres : créer une rencontre parents-professeurs | OK | Rencontre créée. | [image](2026-11/09-agenda/direction-125-rencontres-creer-une-rencontre-parents-professeu.png) |

**Ce que chaque rôle peut faire (constaté)**

- Personnel et familles : « Mon agenda » (semaine, sources à cocher, **nouvel événement** : joué, brouillon « Réunion équipe 4B »). Direction : « Rencontres parents-professeurs » (**création jouée**), réservation de salles (« Salles »). Parents : « Rendez-vous » avec les enseignants.

**Absent ou visiblement mince**

- Mon agenda annonce lui-même « **Sources non affichées : échéance, agenda partagé (pas encore branchées)** ».
- L'événement créé reste en brouillon : la **publication** n'a pas été jouée.
- Réservation de salle : formulaire présent, non joué (aucune réservation créée).
- Rendez-vous des parents : écran en attente des rencontres ouvertes.

**Backend sans écran** : 50 routes de `ent-api` sur 74 ne sont appelées par aucun écran Leptos (ex. : `/agenda/classes/{}`, `/agenda/types`, `/agenda/series`, `/agenda/evenements/{}/dupliquer`, `/agenda/evenements/{}/historique`, `/agenda/lieux/{}/regle`). 

### 10. Sorties et voyages

**Écrans visités** (3)

| Écran | Titre | Rôles qui y accèdent | Captures | Signaux |
|---|---|---|---|---|
| `/voyages` | Sorties et voyages | Direction, Secrétariat, CPE / vie scolaire, Accueil, Professeur, Infirmerie, Comptabilité, Psychologue, Personnel (AED/agent), Parent, Parent (2 enfants), Élève mineur, Élève majeur, Administrateur, Professeur principal | [direction](2026-11/10-voyages/direction-83-sorties.png) · [375 px](2026-11/10-voyages/direction-84-sorties-375.png) | ⚠ message « pas encore disponible » ; appel serveur refusé (403) pendant le chargement |
| `/voyages/nouveau` | Nouvelle sortie | Direction, Professeur | [direction](2026-11/10-voyages/direction-115-voyages-nouvelle-sortie.png) |  |
| `/voyages/serie` | Nouvelle série de sorties | Direction | [direction](2026-11/10-voyages/direction-116-voyages-nouvelle-serie.png) |  |

**Actions jouées** (1)

| Rôle | Action | Résultat | Retour de l'écran | Capture |
|---|---|---|---|---|
| Direction | voyages : créer un brouillon de sortie | OK | Sortie « Sortie au musée de la magie » créée en brouillon. | [image](2026-11/10-voyages/direction-122-voyages-creer-un-brouillon-de-sortie.png) |

**Ce que chaque rôle peut faire (constaté)**

- Tous les personnels : « Sorties » (liste filtrée par état, de brouillon à prête, recherche). Direction : **créer une sortie** (jouée : brouillon « Sortie au musée de la magie »), nouvelle série hebdomadaire. Familles et élèves : « Sorties ».
- Tous voient les compteurs « à valider / budgets à valider / à autoriser / ouvertes / prêtes ».

**Absent ou visiblement mince**

- Le **circuit de validation** (soumettre, valider, autoriser, ouvrir, budget, argent des sorties) n'a pas été joué : l'écran de fiche existe, les actions suivantes n'ont pas été enchaînées.
- Stages et conventions : aucun écran (14 routes).
- 119 routes de `voyages/*` sur 146 n'ont pas d'écran (achats, argent, remises de chèques, V0d, V1).

**Backend sans écran** : 119 routes de `ent-api` sur 146 ne sont appelées par aucun écran Leptos (ex. : `/entreprises`, `/conventions-stage`, `/conventions-stage/{}/statut`, `/stages/dispositifs`, `/stages/dispositifs/{}`, `/stages/parametres`). La part visible est la création et la liste ; le circuit de validation, l'argent et les achats de sorties n'ont pas d'écran.

### 11. Demandes et files de traitement

**Écrans visités** (5)

| Écran | Titre | Rôles qui y accèdent | Captures | Signaux |
|---|---|---|---|---|
| `/demandes` | Faire une demande | Direction, Parent, Professeur principal | [direction](2026-11/11-demandes/direction-11-faire-une-demande.png) · [375 px](2026-11/11-demandes/direction-12-faire-une-demande-375.png) | ⚠ alerte : Je veux signaler une situation grave : en cas de danger, appelez les secours (301 |
| `/demandes/mes` | Mes demandes | Direction, Parent, Professeur principal | [direction](2026-11/11-demandes/direction-13-mes-demandes.png) · [375 px](2026-11/11-demandes/direction-14-mes-demandes-375.png) |  |
| `/demandes/file` | Ma file | Direction, Secrétariat, CPE / vie scolaire, Accueil, Professeur, Infirmerie, Comptabilité, Psychologue, Personnel (AED/agent), Administrateur, Professeur principal | [direction](2026-11/11-demandes/direction-15-ma-file.png) · [375 px](2026-11/11-demandes/direction-16-ma-file-375.png) |  |
| `/demandes/hse` | Heures supplémentaires (HSE) | Direction, Secrétariat, CPE / vie scolaire, Accueil, Professeur, Infirmerie, Comptabilité, Psychologue, Personnel (AED/agent), Administrateur, Professeur principal | [direction](2026-11/11-demandes/direction-17-heures-supplementaires.png) · [375 px](2026-11/11-demandes/direction-18-heures-supplementaires-375.png) | ⚠ appel serveur refusé (403) pendant le chargement |
| `/demandes/copies` | Copies (reprographie) | Direction, Secrétariat, CPE / vie scolaire, Accueil, Professeur, Infirmerie, Comptabilité, Psychologue, Personnel (AED/agent), Administrateur, Professeur principal | [direction](2026-11/11-demandes/direction-19-copies.png) · [375 px](2026-11/11-demandes/direction-20-copies-375.png) | ⚠ alerte : Le dépôt de fichiers à imprimer arrive avec le coffre de documents : en attendant ; appel serveur refusé (403) pendant le chargement |

**Actions jouées** (3)

| Rôle | Action | Résultat | Retour de l'écran | Capture |
|---|---|---|---|---|
| Parent | demandes : choisir « Certificat de scolarité » | **page introuvable** | le clic sur le type de demande ouvre « Page introuvable » | [image](2026-11/11-demandes/parent-47-demandes-choisir-certificat-de-scolarite.png) |
| Parent | demandes : formulaire « Certificat de scolarité » | **page introuvable** | le clic sur le type de demande ouvre « Page introuvable » | [image](2026-11/11-demandes/parent-48-demandes-formulaire-certificat-de-scolarite.png) |
| Secrétariat | demandes : ma file de traitement | OK |  | [image](2026-11/11-demandes/administratif-69-demandes-ma-file-de-traitement.png) |

**Ce que chaque rôle peut faire (constaté)**

- Tous : « Faire une demande » (porte unique, recherche libre, types : certificat de scolarité, justifier une absence, changer de régime, exeat, aménagement d'examen, rendez-vous), « Mes demandes ». Personnel : « Ma file », « Heures supplémentaires (HSE) », « Copies ».
- Secrétariat : file de traitement ouverte.

**Absent ou visiblement mince**

- **Le clic sur un type de demande (ex. « Certificat de scolarité ») mène à « Page introuvable »** : la route de la demande n'existe pas dans l'application (seules `demandes`, `mes`, `file`, `hse`, `copies` sont déclarées). Aucune demande n'a pu être faite par la famille.
- « Copies » : « le dépôt de fichiers arrive avec le coffre de documents » : écran sans dépôt.
- HSE et files : plusieurs profils reçoivent des 403 au chargement.

**Backend sans écran** : 4 routes de `ent-api` sur 34 ne sont appelées par aucun écran Leptos (ex. : `/afaire/rapprochement`, `/demandes/{}`, `/docs-a-signer/{}/signer`, `/docs-a-signer/{}/annuler`). 

### 12. Formulaires, consentements, campagnes

**Écrans visités** (2)

| Écran | Titre | Rôles qui y accèdent | Captures | Signaux |
|---|---|---|---|---|
| `/formulaires` | Formulaires et consentements | Direction, Secrétariat, CPE / vie scolaire, Parent, Parent (2 enfants), Élève mineur, Élève majeur, Administrateur | [direction](2026-11/12-formulaires/direction-77-formulaires.png) · [375 px](2026-11/12-formulaires/direction-78-formulaires-375.png) |  |
| `/campagnes` | Campagnes d'information | Direction, Administrateur | [direction](2026-11/12-formulaires/direction-79-campagnes.png) · [375 px](2026-11/12-formulaires/direction-80-campagnes-375.png) |  |

**Actions jouées** (2)

| Rôle | Action | Résultat | Retour de l'écran | Capture |
|---|---|---|---|---|
| Direction | formulaires : créer un brouillon de consentement | OK | Brouillon créé : publiez-le pour prévenir les familles. | [image](2026-11/12-formulaires/direction-123-formulaires-creer-un-brouillon-de-consentement.png) |
| Direction | campagnes : préparer une information aux familles | OK | Aperçu des destinataires prêt : vérifiez-le avant de confirmer. | [image](2026-11/12-formulaires/direction-124-campagnes-preparer-une-information-aux-familles.png) |

**Ce que chaque rôle peut faire (constaté)**

- Direction : « Formulaires et consentements » (**création d'un brouillon jouée**), qui a répondu, « Campagnes d'information » (**aperçu des destinataires joué**, canaux e-mail / mobile / SMS avec plafond). Parents, élèves : « Formulaires » (consentements à signer, décisions datées modifiables).

**Absent ou visiblement mince**

- Brouillon créé mais **non publié** : aucun parent n'a donc eu de formulaire à signer ; l'envoi réel d'une campagne (confirmation humaine) n'a pas été joué.
- Documents à signer, accords, e-mails et SMS : 54 routes sans écran.

**Backend sans écran** : 54 routes de `ent-api` sur 66 ne sont appelées par aucun écran Leptos (ex. : `/accords/catalogue`, `/accords/objets`, `/accords/objets/{}/activer`, `/accords/objets/{}/desactiver`, `/accords/objets/{}/usages`, `/accords/objets/{}/versions`). 

### 13. Finances

**Écrans visités** (2)

| Écran | Titre | Rôles qui y accèdent | Captures | Signaux |
|---|---|---|---|---|
| `/comptabilite` | Comptabilité | Direction, Comptabilité, Administrateur | [direction](2026-11/13-finances/direction-89-comptabilite.png) · [375 px](2026-11/13-finances/direction-90-comptabilite-375.png) |  |
| `/mes-factures` | Factures et paiements | Comptabilité, Parent, Parent (2 enfants) | [compta](2026-11/13-finances/compta-33-factures-vue-famille.png) | ⚠ alerte : Le paiement en ligne n'est pas encore proposé par l'établissement. Prélèvement, v |

**Actions jouées** (4)

| Rôle | Action | Résultat | Retour de l'écran | Capture |
|---|---|---|---|---|
| Comptabilité | comptabilité : onglet Prélèvements | OK |  | [image](2026-11/13-finances/compta-34-comptabilite-onglet-prelevements.png) |
| Comptabilité | comptabilité : onglet Relances | OK |  | [image](2026-11/13-finances/compta-35-comptabilite-onglet-relances.png) |
| Comptabilité | comptabilité : onglet Tarifs | OK |  | [image](2026-11/13-finances/compta-36-comptabilite-onglet-tarifs.png) |
| Comptabilité | comptabilité : onglet Export comptable | OK |  | [image](2026-11/13-finances/compta-37-comptabilite-onglet-export-comptable.png) |

**Ce que chaque rôle peut faire (constaté)**

- Comptabilité : « Comptabilité » (factures par état, prélèvements, relances, tarifs, export comptable, génération annuelle) : **tous les onglets ouverts**. Parents : « Factures et paiements » par foyer (reste à payer).

**Absent ou visiblement mince**

- Aucune facture dans le labo : **génération, émission, prélèvement, relance, export** non joués ; écrans vides de données.
- Famille : « Le paiement en ligne n'est pas encore proposé ».
- Bourses, restauration, tarifs de consommation, TVA, comptes débiteurs, clôture : 66 routes sans écran.

**Backend sans écran** : 66 routes de `ent-api` sur 86 ne sont appelées par aucun écran Leptos (ex. : `/finances/articles/{}`, `/finances/tarifs/{}`, `/finances/reductions`, `/finances/reductions/{}/eleves`, `/finances/bourses/baremes`, `/finances/echeancier-type`). 

### 14. Santé, aménagements, accompagnement

**Écrans visités** (7)

| Écran | Titre | Rôles qui y accèdent | Captures | Signaux |
|---|---|---|---|---|
| `/infirmerie` | Infirmerie | Direction, Infirmerie, Administrateur | [direction](2026-11/14-sante-accompagnement/direction-51-infirmerie.png) · [375 px](2026-11/14-sante-accompagnement/direction-52-infirmerie-375.png) | ⚠ appel serveur refusé (403) pendant le chargement |
| `/accompagnement` | Accompagnement | Direction, Administrateur | [direction](2026-11/14-sante-accompagnement/direction-53-accompagnement.png) · [375 px](2026-11/14-sante-accompagnement/direction-54-accompagnement-375.png) | ⚠ appel serveur refusé (403) pendant le chargement |
| `/amenagements` | Aménagements et contraintes | Direction, Secrétariat, CPE / vie scolaire, Professeur, Infirmerie, Comptabilité, Psychologue, Administrateur, Professeur principal | [direction](2026-11/14-sante-accompagnement/direction-55-amenagements.png) · [375 px](2026-11/14-sante-accompagnement/direction-56-amenagements-375.png) | ⚠ appel serveur refusé (403) pendant le chargement |
| `/accompagnement/eleves/:id` | Accompagnement | Direction | [direction](2026-11/14-sante-accompagnement/direction-103-accompagnement-de-harry.png) | ⚠ appel serveur refusé (403) pendant le chargement |
| `/infirmerie/eleves/:id` | Dossier de santé | Direction, Infirmerie | [direction](2026-11/14-sante-accompagnement/direction-104-infirmerie-dossier-de-harry.png) | ⚠ appel serveur refusé (403) pendant le chargement |
| `/infirmerie/eleves/:id/urgence` | Fiche d'urgence · ScolIA | Infirmerie | [infirmerie](2026-11/14-sante-accompagnement/infirmerie-30-fiche-d-urgence-de-harry.png) | ⚠ écran presque vide ; appel serveur refusé (403) pendant le chargement |
| `/sante-enfant` | Santé de mon enfant | Parent, Parent (2 enfants) | [parent](2026-11/14-sante-accompagnement/parent-25-sante.png) · [375 px](2026-11/14-sante-accompagnement/parent-26-sante-375.png) | ⚠ appel serveur refusé (403) pendant le chargement |

**Actions jouées** (1)

| Rôle | Action | Résultat | Retour de l'écran | Capture |
|---|---|---|---|---|
| Infirmerie | infirmerie : chercher un élève pour un passage | **refusé par le serveur** | Accès refusé Votre profil ne permet pas d'accéder à ces informations. | [image](2026-11/14-sante-accompagnement/infirmerie-31-infirmerie-chercher-un-eleve-pour-un-passage.png) |

**Ce que chaque rôle peut faire (constaté)**

- Infirmerie : « Infirmerie » (passages du jour, PAI à réviser, dossier de santé, fiche d'urgence), « Aménagements ». Direction, CPE : « Accompagnement » (PAP, PPS, PPRE, ULIS, AESH), dossier d'accompagnement d'un élève. Parent : « Santé de mon enfant ».

**Absent ou visiblement mince**

- **Infirmerie** : l'infirmière de labo reçoit « **Accès refusé** » en cherchant un élève ; la liste des passages affiche « **Date invalide. Choisissez un jour valide** » pour le jour courant. Aucun passage n'a pu être enregistré.
- **Accompagnement** : la liste des dispositifs à réviser répond « Accès refusé » à la direction ; **Santé de mon enfant** répond « Accès refusé » au parent.
- Psychologue : aucun écran (dossiers, entretiens, rendez-vous, courrier d'orientation existent côté serveur).

**Backend sans écran** : 6 routes de `ent-api` sur 20 ne sont appelées par aucun écran Leptos (ex. : `/infirmerie/passages`, `/pai/pieces/{}`, `/pai/pieces/{}/lien`, `/psy/entretiens`, `/psy/rdvs`, `/psy/rdvs/{}/statut`). 

### 15. Administration : référentiel, rôles, sécurité

**Écrans visités** (12)

| Écran | Titre | Rôles qui y accèdent | Captures | Signaux |
|---|---|---|---|---|
| `/referentiel` | Référentiel scolaire | Direction, Secrétariat, Administrateur | [direction](2026-11/15-administration/direction-35-referentiel.png) · [375 px](2026-11/15-administration/direction-36-referentiel-375.png) |  |
| `/direction/roles` | Rôles et droits | Direction, Administrateur | [direction](2026-11/15-administration/direction-91-roles-et-droits.png) · [375 px](2026-11/15-administration/direction-92-roles-et-droits-375.png) |  |
| `/securite` | Sécurité du compte | Direction, Parent, Professeur principal | [direction](2026-11/15-administration/direction-93-compte.png) · [375 px](2026-11/15-administration/direction-94-compte-375.png) | ⚠ alerte : C'est votre dernière passkey : la connexion se fera par mot de passe seul. |
| `/notifications` | Mes notifications | Direction, Parent, Professeur principal | [direction](2026-11/15-administration/direction-95-notifications.png) · [375 px](2026-11/15-administration/direction-96-notifications-375.png) |  |
| `/referentiel/annees` | Années et bascule d'année | Direction | [direction](2026-11/15-administration/direction-105-referentiel-annees.png) |  |
| `/referentiel/structure` | Sites, niveaux et MEF | Direction | [direction](2026-11/15-administration/direction-106-referentiel-structure.png) |  |
| `/referentiel/classes` | Classes et groupes | Direction, Secrétariat | [direction](2026-11/15-administration/direction-107-referentiel-classes.png) |  |
| `/referentiel/calendrier` | Périodes et calendrier | Direction | [direction](2026-11/15-administration/direction-108-referentiel-calendrier.png) |  |
| `/referentiel/grille` | Grille horaire et séances | Direction | [direction](2026-11/15-administration/direction-109-referentiel-grille.png) |  |
| `/direction/securite` | Politique de sécurité | Direction | [direction](2026-11/15-administration/direction-110-politique-de-securite.png) |  |
| `/direction/protections` | Protections | Direction | [direction](2026-11/15-administration/direction-111-protections.png) |  |
| `/direction/roles/groupes` | Responsables des groupes et ateliers | Direction | [direction](2026-11/15-administration/direction-112-roles-responsables-de-groupes.png) |  |

**Actions jouées** (1)

| Rôle | Action | Résultat | Retour de l'écran | Capture |
|---|---|---|---|---|
| Direction | rôles : créer un rôle « Professeur de théâtre » | OK | Rôle créé. Vous pouvez maintenant l'attribuer. | [image](2026-11/15-administration/direction-127-roles-creer-un-role-professeur-de-theatre.png) |

**Ce que chaque rôle peut faire (constaté)**

- Direction, secrétariat, administrateur : « Référentiel » (années et bascule, sites, niveaux, MEF, classes, groupes, périodes, calendrier, grille horaire), « Rôles et droits » (**création d'un rôle « Professeur de théâtre » jouée**), politique de sécurité, protections, recherches bloquées.
- Tous : « Compte » (sécurité : passkey, enrôlement, récupération) et « Notifications » (préférences).

**Absent ou visiblement mince**

- Pas d'écran de gestion des **comptes utilisateurs** (créer un personnel, activer, réinitialiser) ni d'**attribution** d'un rôle à une personne dans le parcours joué.
- Pas d'écran d'import / export, de **journal d'audit**, de sauvegarde / restauration, de paramètres généraux de l'établissement (identité, horaires, communications).
- Message de sécurité permanent : « C'est votre dernière passkey » (mot de passe seul accepté en labo).

**Backend sans écran** : 164 routes de `ent-api` sur 231 ne sont appelées par aucun écran Leptos (ex. : `/imports/{}`, `/imports/{}/annuler`, `/imports/lignes/{}/traiter`, `/auth/casquettes`, `/auth/casquette`, `/referentiel/classes/{}/affectations`). 

### 16. Internat (sans écran)

**Ce que chaque rôle peut faire (constaté)**

- Aucun écran.

**Absent ou visiblement mince**

- Le module d'internat n'a aucun écran : tout est côté serveur.

**Backend sans écran** : 14 routes de `ent-api` sur 14 ne sont appelées par aucun écran Leptos (ex. : `/internat/zones`, `/internat/zones/{}`, `/internat/chambres`, `/internat/chambres/{}/lits`, `/internat/lits/{}/neutralisation`, `/internat/eleves/{}/sejours`). Domaine entièrement côté serveur.

### 17. Documents (sans écran)

**Ce que chaque rôle peut faire (constaté)**

- Aucun écran.

**Absent ou visiblement mince**

- Aucun coffre de documents à l'écran (« arrive avec le coffre de documents » sur l'écran Copies) : dépôt, lecture, demandes d'accès existent côté serveur.

**Backend sans écran** : 13 routes de `ent-api` sur 13 ne sont appelées par aucun écran Leptos (ex. : `/verifier/{}`, `/verifier`, `/documents/{}/fichier`, `/documents/{}/lien`, `/fichiers/{}`, `/documents/{}/grants`). Domaine entièrement côté serveur.

## Tableau de synthèse

| Domaine | Écrans existants | Actions jouables (jouées ici) | Manques visibles | Backend sans écran |
|---|---|---|---|---|
| Accueil, à faire, recherche | 7 | 1 sur 1 | Page d'accueil par rôle présente ; pas de signaux ; vide pour 4 profils |  |
| Élèves et classes | 6 | 2 sur 2 | Annuaire, fiche, dossier, classes ; aucune écriture sur la fiche | 169 / 219 |
| Inscriptions et réinscriptions | 2 | 1 sur 1 | Campagnes créables ; instruction non jouée ; côté famille vide | 49 / 74 |
| Vie scolaire : appel, absences, assiduité, discipline | 9 | 10 sur 11 | Appel, déclaration famille, décision CPE jouables ; discipline refusée ; signalements « à venir » | 71 / 137 |
| Absences des personnels, remplacements, heures | 12 | 3 sur 3 | Déclaration et files présentes ; instruction non jouée de bout en bout ; paye absente | 148 / 186 |
| Emploi du temps | 3 | 0 | Consultation seulement ; aucune édition à l'écran | 36 / 51 |
| Cahier de textes et travail à faire | 6 | 5 sur 5 | Travail donné et vu côté élève ; écran agenda élève en chargement | 30 / 97 |
| Évaluations, notes, bulletins | 3 | 0 sur 1 | Écrans présents ; saisie de notes non jouable sur le jeu de labo | 14 / 37 |
| Messagerie | 1 | 4 sur 4 | Envoi famille jouable ; réponse non jouée ; absente du menu de 5 profils | 9 / 19 |
| Agenda, salles, rendez-vous | 5 | 3 sur 3 | Événements et rencontres créables ; sources non branchées | 50 / 74 |
| Sorties et voyages | 3 | 1 sur 1 | Création de sortie jouable ; circuit de validation et argent sans écran | 119 / 146 |
| Demandes et files de traitement | 5 | 1 sur 3 | Porte d'entrée présente mais **la demande elle-même mène à une page introuvable** | 4 / 34 |
| Formulaires, consentements, campagnes | 2 | 2 sur 2 | Brouillons et aperçus jouables ; publication et signature non jouées | 54 / 66 |
| Finances | 2 | 4 sur 4 | Onglets présents, aucune opération jouée ; paiement en ligne absent | 66 / 86 |
| Santé, aménagements, accompagnement | 7 | 0 sur 1 | Écrans présents mais refus de droits sur le jeu de labo ; psychologue sans écran | 6 / 20 |
| Administration : référentiel, rôles, sécurité | 12 | 1 sur 1 | Référentiel et rôles présents ; comptes, imports, audit sans écran | 164 / 231 |
| Internat (sans écran) | 0 | 0 | Aucun écran | 14 / 14 |
| Documents (sans écran) | 0 | 0 | Aucun écran | 13 / 13 |

_Aucun pourcentage : c'est Florian qui juge._ « Backend sans écran » = routes de `ent-api` sans appel correspondant dans `ent-web-api` (comparaison automatique des chemins ; certaines routes servent l'ancienne interface, les applications mobiles ou des tâches internes, voir la méthode).

## Méthode et limites

- **Comptage « backend sans écran »** : les chemins `.route("…")` de `ent-api` sont comparés aux chemins écrits dans `ent-web-api` (client des écrans Leptos). Une route sans équivalent est comptée « sans écran » ; elle peut être utilisée par l'ancienne interface JavaScript, les applications mobiles ou des tâches internes. C'est donc un **majorant** ; les ordres de grandeur sont fiables, les chiffres exacts non. Domaines regroupés par premier segment de la route.

- **Actions** : « OK » signifie que le script a pu jouer l'action ; le texte renvoyé par l'écran est cité. Les ouvertures d'écran ou d'onglet comptent comme actions de consultation. Plusieurs actions décisives n'ont pas pu être jouées (voir les lignes « refusé par le serveur » et les « Absent ou mince »).
- **Rejouer** : `node docs/visite_guidee/parcours.mjs <base> <dossier>` (serveur de labo lancé, Playwright installé ; `PHASE=visite` ou `PHASE=actions`, `SCENARIO=a,b`, `INSPECT=rôle@/chemin` pour décrire un écran), puis `python3 compresser.py <dossier>` et `node rapport.mjs`. Chaque lancement crée un nouvel établissement de labo.
- Le rôle « professeur principal » a été rejoué séparément après un délai de connexion ; ses captures viennent d'un second établissement de labo (même jeu).

## À retenir pour la conception (sans pourcentage)

1. Les domaines les plus **épais côté écran** : vie scolaire (appel, absences, discipline), absences des personnels, cahier de textes, voyages (création), messagerie.
2. Les domaines **épais côté serveur mais minces à l'écran** : élèves (119 routes sans écran), voyages (119), absences des personnels (145), finances (66), formulaires (54), emploi du temps (36), agenda (50).
3. Les domaines **sans écran** : internat, documents, stages, psychologue.
4. **Défauts rencontrés** (non corrigés) : demande → page introuvable ; grille de notes impossible à ouvrir sur le jeu de labo ; refus de droits à l'infirmerie, à l'accompagnement, à la santé de l'enfant et à la discipline ; « Travail à faire » de l'élève en chargement ; « Date invalide » à l'infirmerie ; menu proposant des écrans refusés (403) pour plusieurs profils ; messagerie absente du menu de cinq profils.
5. Pour la structure de navigation et les signaux, voir « Retours de Florian » plus haut.
