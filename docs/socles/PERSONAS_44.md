# Passe « personas » sur le socle 44 — Ressources matérielles, prêts et maintenance

**Date :** 2026-11-11 — **Statut :** proposition, **non décidée** (à relire par Florian) ; les recommandations s'appliquent par défaut (D-031), sauf ce qui engage l'établissement à l'extérieur (§8.2). Aucun code écrit ni modifié ; aucun accès à une base, à un secret ni à un service de production. Le code Next.js a été **lu seulement** (lecture ciblée, rien exécuté).
**Branche :** `ent-socle-43-44-conception`, partie de `origin/ent-mc0hoe` (`afeb8c4`).
**Éprouve :** `docs/socles/44_RESSOURCES_MATERIELLES.md` (§0 à §29), avec `CLAUDE.md`, `docs/DECISIONS.md` D-017, D-031, D-034, D-035, D-036, `docs/PRA_SAUVEGARDES.md` §18.4, `docs/socles/19_AGENDA_RDV.md` (§14) ; forme reprise de `PERSONAS_41.md`.
**Ajouts :** faits **directement** dans `44_RESSOURCES_MATERIELLES.md`, au **§30** (actions RSM-250 à RSM-269, règles R-RM35 à R-RM40, cas limites CRM-47 à CRM-58, questions, lots révisés, défauts relayés au 19). Rien n'est supprimé.
**Numérotation :** manques **M-4401 à M-4440** (plage propre au socle 44).

---

## 0. À lire en premier : quatre bloquants et leur recommandation (appliquée par défaut, D-031)

La conception tient : un bien a un emplacement, un détenteur et un état datés ; un prêt se solde ; la sécurité n'attend pas ; la réservation de salles reste au 19. Vécue par dix-sept personas, elle révèle quatre bloquants.

| # | Point | Pourquoi c'est bloquant | Recommandation (appliquée par défaut) | Lots |
|---|---|---|---|---|
| **A** | **L'étiquetage initial est irréaliste pour la rentrée 2027.** Le §11 suppose chaque manuel étiqueté ; l'établissement a **plusieurs milliers de manuels sans étiquette** et ne les collera pas en août (P01, P07). | Sans étiquettes, la campagne « mode caisse » ne marche pas ; le module ne sert pas l'année 1, et la reprise de Charlemagne n'apporte rien (Charlemagne ne suit pas les exemplaires [H]). | **Suivi en quantité par élève** possible pour les manuels (`lot` : modèle × quantité × état global, année de mise en service **estimée** par lot) ; **étiquetage progressif** au retour de juin 2028 (chaque exemplaire rendu reçoit son étiquette) ; le barème s'applique à l'année estimée du lot. **R-RM35**, RSM-250, RSM-251. | RM2 |
| **B** | **L'alerte de sécurité hors des heures** (soir, week-end, vacances) part vers « la direction et l'agent d'astreinte » : il n'y a pas d'astreinte déclarée par défaut, et personne n'est connecté (P02, P09). | Fuite d'eau un samedi, porte coupe-feu bloquée un soir d'internat : l'alerte reste lue lundi. | **Chaîne d'astreinte technique** déclarée par semaine (6, comme l'astreinte de l'internat, R-IT43 du 41) ; **escalade** SMS (8) après 15 min sans accusé ; affichage du **numéro d'urgence** (et des numéros des concessionnaires : eau, gaz, électricité) à l'écran de signalement ; si aucune astreinte n'est déclarée, alerte à la direction **et** affichage « appelez le … » ; **R-RM36**, RSM-252, RSM-253. | RM4 |
| **C** | **La reprographie cloisonnée par site casse l'organisation réelle** : beaucoup d'établissements ont **un seul reprographe pour tous les sites** (P08). Le correctif de DRM-01 (portée par site) le rendrait aveugle. | Le module Next.js, en usage, serait remplacé par un module qui ne correspond pas au service. | Le **réceptionnaire est désigné explicitement pour un ou plusieurs sites** (liste, `ent_repro_receptionnaire` par site) ; ce qui est interdit, c'est la désignation **implicite** par rôle ; validation de la couleur toujours par la direction **du site** de la demande. **R-RM27 précisée**, RSM-254. | RM5 |
| **D** | **Facturer des pertes pendant l'année de doublon** : les finances restent dans Charlemagne (D-034) ; le §11.3 et RM6 envoient les pertes au 24, qui ne facture pas réellement pendant le doublon (P01, P17). | Double facturation ou aucune facturation. | **Mode suivi** (comme 40, 41, 42) : ScolIA constate, calcule au barème et **exporte** la liste des pertes à facturer vers Charlemagne (ou le logiciel comptable) ; aucune facture émise par ScolIA tant que le 24 n'est pas en production ; **R-RM37**, RSM-255. | RM6 |

**Défauts Next.js relayés au socle 19** (réservation de salles, en production, à signaler à Florian) : **DRM-14** (annulation de n'importe quelle réservation ou série par tout compte) et **DRM-15** (modification de la réservation d'autrui) sont **critiques** et absents de la liste ND-01 à ND-12 du 19 : ajoutés au §30 du 44 pour report au 19.

---

## 1. Méthode
Celle de `PERSONAS_42.md` §1 : dix-sept personas (privé sous contrat — établissement pilote à deux sites, collège et lycée, internat ; lycée public EPLE avec agents de la région), dont un **attaquant** (six visages) et un **auditeur RGPD** ; manques **M-44xx**, contradictions, cas limites, versés au §30 du 44. Gravité : **B**, **H**, **M**, **F**. Vérifications ponctuelles du code Next.js : contrôle de propriété à l'annulation des réservations, désignation du réceptionnaire de reprographie [F].

---

## 2. Les personas en un tableau
| # | Persona | Situation | Socles touchés |
|---|---|---|---|
| P01 | **Mme Lefèvre, adjointe gestionnaire** (lycée public) / **DAF** (privé) | juin : retour des manuels, inventaire, pertes | 44, 24, 48 |
| P02 | **M. Martin, agent de maintenance** (agent de la région dans le public) | 14 demandes ouvertes, 3 bâtiments | 44, 6, 5 |
| P03 | **M. Chen, référent numérique** | 400 ordinateurs de la région pour les 2de, 60 pannes par an | 44, 7, 18 |
| P04 | **Mme Garnier, professeure de SVT** | réserve le labo, une malle de dissection et le vidéoprojecteur (en panne) | 44, 19, 10 |
| P05 | **M. Roche, professeur d'EPS** | minibus pour une rencontre UNSS | 44, 43, 42, 6 |
| P06 | **Famille Martin-Leroy**, séparée ; mère signataire, père payeur | tablette de Léa cassée | 44, 4, 24 |
| P07 | **Yanis, 3e**, déménage en février | 7 manuels, dont 2 perdus | 44, 3, 24 |
| P08 | **Mme Duval, accueil du site principal** | clés, badges, reprographie des deux sites | 44, 6 |
| P09 | **M. Garcia, proviseur** | commission de sécurité demain ; extincteurs échus | 44, 14 |
| P10 | **Nora, surveillante d'internat** | état des lieux de sortie de chambre, dégradation contestée | 44, 41 |
| P11 | **M. Vidal, agent comptable** (public) | sortie d'inventaire ; un collègue veut des chèques de caution | 44, 48 |
| P12 | **Ascensoriste** (prestataire sous contrat) | panne, intervention, rapport | 44 |
| P13 | **Lucas, 19 ans, BTS** | emprunte un ordinateur portable pour son stage | 44, 39 |
| P14 | **Mme Bonnet, responsable du laboratoire de chimie** | inventaire du matériel et des produits | 44, 49 |
| P15 | **Les attaquants** | six visages (§4) | 44, 1 |
| P16 | **M. Blanc, délégué à la protection des données** | revue du socle | 44, 27 |
| P17 | **Florian, juin–septembre 2027** | bascule de la reprographie, manuels de rentrée | 44, D-034, D-035 |

---

## 3. Audit de parité avec Next.js (fait de code)

### 3.1 Comportements vérifiés
- Photocopies : noir et blanc accepté d'emblée (`route.ts:446`), couleur en attente de la direction du site (`access.ts:51-58`), réceptionnaire par rôle « accueil » par défaut (`module-access-defaults.ts:273`) [F].
- Réservations de salles : `delete/route.ts:33-61` et `update/route.ts:14-24` ne vérifient que la session [F] (DRM-14, DRM-15).

### 3.2 Fonctions sans équivalent au 44 (ajoutées au §30)
- Next.js permet de **choisir la direction destinataire** par l'établissement (site) saisi sur la demande ; ScolIA déduit le site du demandeur : il faut permettre de **choisir le site d'impression** (un enseignant des deux sites imprime là où il fait cours ce jour-là). RSM-254 précisé.
- Le compteur « à imprimer » du tableau de bord (signal) est repris (RSM-231).

### 3.3 Défauts de sécurité nouveaux
Aucun au-delà de DRM-01 à DRM-19.

---

## 4. Parcours et manques, persona par persona

### P01 — Mme Lefèvre, gestionnaire, juin
- **M-4401 (B → A)** : 4 800 manuels sans étiquette. → bloquant A.
- **M-4402 (H)** : le barème dégressif suppose l'année de mise en service de chaque exemplaire ; elle ne la connaît pas. → année **estimée** par lot à la reprise (dernière commande du titre), modifiable ; à défaut, la 4e année et plus (20 %) pour un exemplaire d'année inconnue (R-RM13 précisée : en cas de doute, le taux le plus favorable à la famille).
- **M-4403 (H)** : la délibération du CA sur le barème est de 2019 : le §11.3 refuse de facturer sans décision enregistrée. → la saisie d'une **décision ancienne** (date, référence, pièce) suffit ; le barème national s'applique sinon (déjà prévu).
- **M-4404 (B → D)** : les factures de pertes de juin 2028 partent-elles par ScolIA ou Charlemagne ? → bloquant D.
- **M-4405 (M)** : elle veut un état « manuels non rendus par classe » à remettre aux professeurs principaux : sans montant ni aide ; ajouter cet état (RSM-256).

### P02 — M. Martin, agent de maintenance
- **M-4406 (H)** : 30 % de ses demandes concernent la même fuite signalée par 4 personnes en 4 formulations : le doublon par salle et catégorie (§12.1) ne suffit pas si la catégorie diffère. → proposition de doublon par **salle** quelle que soit la catégorie, ouverte depuis moins de 7 jours ; décision par la personne (RSM-161 étendu).
- **M-4407 (B → B)** : la fuite du samedi. → bloquant B.
- **M-4408 (H)** : agent de la région, il a un compte ScolIA ? Oui s'il est déclaré au 6 comme personnel de l'établissement (agent territorial affecté) ; à préciser au 6 (catégorie d'agent « collectivité ») : les agents territoriaux ont un compte avec le seul rôle technique.
- **M-4409 (M)** : il veut la liste du jour **hors ligne** (caves, chaufferie sans réseau) et « fait » synchronisé ensuite. → mobile hors ligne pour la liste du jour et la clôture (RM4).

### P03 — M. Chen, référent numérique
- **M-4410 (H)** : les 400 ordinateurs appartiennent à la région qui impose **sa** convention et **sa** charte : la convention de prêt est un document **fourni par la collectivité** (trame importée, 7) ; la signature par les titulaires reste dans ScolIA ; l'état des prêts est exportable au format de la région (RSM-158, déjà).
- **M-4411 (H)** : une panne d'ordinateur prêté = demande d'intervention **liée au prêt** ; l'élève reçoit un ordinateur de remplacement (prêt de remplacement, l'original en réparation) ; la chaîne prêt → panne → remplacement → retour doit être un geste (RSM-257).
- **M-4412 (M)** : fin de garantie : la région gère le service après-vente ; le 44 garde la date et le numéro de dossier du fournisseur (champ sur l'intervention).

### P04 — Mme Garnier, professeure de SVT
- **M-4413 (H)** : elle réserve salle + malle au 19 ; le vidéoprojecteur de la salle est en panne depuis 3 jours : l'avertissement à la réservation existe (RSM-127) ; elle veut aussi qu'on lui **propose une salle équivalente** sans panne. → recherche du 19 filtrée sur les équipements **en service** (port `equipements`, déjà) : à écrire au 19 (recherche de salle de repli).
- **M-4414 (M)** : elle signale la panne en scannant le QR code de la porte : le QR code de la salle doit être **imprimable en lot** pour toutes les salles (RSM-258).

### P05 — M. Roche, minibus
- **M-4415 (H)** : le permis de conduire « enregistré au 6 » : le 6 ne le prévoit peut-être pas ; et la **catégorie** (B, D1) et la **date de validité** comptent. → attribut d'habilitation au 6 (habilitation `conduite_minibus`, date, sans copie du permis) ; à demander au 6.
- **M-4416 (M)** : le carnet de bord du minibus doit aussi tracer les **passagers** pour une sortie : non, c'est la sortie du 42 qui porte les participants ; le carnet ne porte que le trajet.

### P06 — Famille Martin-Leroy
- **M-4417 (H)** : la mère a signé, le père paie ; la tablette est cassée par chute. Assurance scolaire de la famille ? → l'avis de facturation mentionne que la famille peut solliciter **son assurance** (responsabilité civile) et donne le **constat** (photo, date) comme pièce (RSM-259).
- **M-4418 (M)** : le père conteste ne pas avoir signé : la convention est signée par un titulaire (acte usuel) ; le payeur est celui du 4 ; l'avis au père au moment de la signature (« Léa a reçu une tablette ») doit exister (R-RM10, déjà) ; vérifier qu'il part bien aux deux titulaires.

### P07 — Yanis, départ en février
- **M-4419 (H)** : son départ ne doit **jamais** être bloqué par les manuels (il change d'établissement : certificat de radiation dû). R-RM17 dit « dossier non clos sans prêts soldés ou à recouvrer » : bien comprendre que **« à recouvrer » se pose en un geste** et ne bloque **aucun document** (R-RM15). Précision écrite (R-RM17 précisée).
- **M-4420 (M)** : ses 5 manuels rendus le dernier jour au secrétariat : retour hors campagne en mode caisse au secrétariat (RSM-150 hors campagne).

### P08 — Mme Duval, accueil
- **M-4421 (B → C)** : un seul reprographe pour les deux sites. → bloquant C.
- **M-4422 (H)** : clés : 140 clés en circulation ; reprise de l'existant = un cahier. → **inventaire des clés initial** en mode « lot » (une clé par numéro, détenteur saisi), sans organigramme complet au départ (l'organigramme est facultatif, R-RM38).
- **M-4423 (M)** : badge d'accès d'un intervenant pour une semaine : prêt court avec date de retour, rappel à l'accueil le jour de fin.

### P09 — M. Garcia, proviseur, commission demain
- **M-4424 (H)** : il veut le registre **imprimé à jour** et la liste des observations de la dernière commission avec leur état : RSM-187 et RSM-189 ; ajouter un **dossier de commission** (export unique : registre, vérifications à jour, observations levées avec justificatifs, exercices) (RSM-260).
- **M-4425 (H)** : les extincteurs ont 2 mois de retard : il veut savoir **pourquoi** (bon de commande, prestataire relancé) : la vérification échue porte une **action en cours** (tâche) visible.
- **M-4426 (M)** : public : les vérifications des bâtiments sont souvent **commandées par la région** ; l'établissement ne fait que recevoir les rapports : le responsable de la vérification peut être « collectivité » (attribut du modèle) ; l'établissement voit l'échéance et relance.

### P10 — Nora, surveillante d'internat
- **M-4427 (H)** : l'état des lieux de sortie un vendredi à 17 h, parents pressés, signature impossible : constat **contradictoire** avec un second personnel (R-RM33) ; et **avis** à la famille avec les photos, délai de contestation (S-4408).
- **M-4428 (M)** : la dégradation est due à un **autre** interne (casse en jouant) : la facturation peut viser l'**auteur** si identifié par une procédure du 13 ; sinon personne (jamais la chambre entière par défaut, R-RM39).

### P11 — M. Vidal, agent comptable
- **M-4429 (H)** : un collègue a fait signer 30 chèques de caution pour des calculatrices : ScolIA ne gère pas de caution (S-4405 non) — mais il faut un **message clair** au moment où quelqu'un veut l'activer : « à décider avec l'agent comptable, référence de la décision exigée ».
- **M-4430 (M)** : sortie d'inventaire : il veut la **liste valorisée** (valeur nette) jointe à la délibération : export (RSM-114 étendu).

### P12 — L'ascensoriste
- **M-4431 (M)** : il intervient sans compte : la demande lui est **transmise** (N3) ; son rapport est saisi par le gestionnaire ; aucun accès au prestataire dans V1 (pas de lien public qui modifie, R-RM34).

### P13 — Lucas, 19 ans, BTS
- **M-4432 (M)** : il emprunte un portable pour son stage en entreprise (39) : la date de retour suit la fin de la convention de stage ; il signe seul (majeur) ; il est payeur seulement s'il l'est au 4.

### P14 — Mme Bonnet, laboratoire de chimie
- **M-4433 (H)** : les **produits chimiques** (inventaire, fiches de données de sécurité, produits CMR, dates de péremption, armoire ventilée) ne sont pas des biens prêtables : le 44 ne les couvre pas. → hors V1 du 44 ; à rattacher au **socle 49** (santé et sécurité au travail, T-06) avec le registre des agents chimiques ; le 44 garde le **matériel** du laboratoire. Question Q-RM37.

### P15 — Les attaquants
1. **Personnel malveillant** qui annule les réservations des autres (comme DRM-14 dans Next.js) : au 19, commande serveur qui vérifie l'auteur, le bénéficiaire ou le gestionnaire (relayé au 19, §30).
2. **Élève** qui scanne le QR code d'une porte pour envoyer de fausses demandes : le QR code ouvre ScolIA, qui exige une **session de personnel** (S-4409) ; aucune demande anonyme ; limitation de débit.
3. **Agent qui part avec le passe général** : tâche bloquante au départ (R-RM20), changement de cylindres proposé (§14.1).
4. **Réceptionnaire** qui lit les PDF d'un site qui n'est pas le sien : portée explicite (bloquant C) ; chaque ouverture journalisée.
5. **Direction qui veut suivre un enseignant** par ses emprunts, ses clés, ses réservations (heures d'arrivée) : aucune vue « par personne » hors gestionnaire ; pas d'indicateur nominatif ; le journal des badges n'est pas importé (R-RM22) ; ajout : **R-RM40** (aucune donnée du 44 ne sert à l'évaluation d'un personnel).
6. **Photo d'intervention** qui montre des élèves ou un tableau avec des noms : avertissement à la prise de vue (« pas de personnes ») ; la photo transmise à l'extérieur peut être recadrée ; jamais transmise sans aperçu (N3, déjà).

### P16 — M. Blanc, délégué à la protection des données
- Finalités : gestion du patrimoine, prêts, maintenance, sécurité des locaux, reprographie ; base légale : mission de service public (public) ou intérêt légitime et contrat (privé) ; minimisation (aucune géolocalisation, aucun code de casier, PDF effacés à J+30) ; durées (§21.3).
- **M-4434 (H)** : les **PDF de reprographie** peuvent contenir des données d'élèves (listes, copies corrigées) : la case « contient des données d'élèves » doit **réduire** la durée (effacement à l'état « prête » + 7 jours au lieu de J+30) et interdire la lecture par l'IA (RSM-238 précisé, S-4420).
- **M-4435 (M)** : les **photos de constat** de dégradation (chambre, tablette) peuvent montrer l'élève ou ses affaires : consigne de prise de vue, durée = celle du prêt (1 an après le solde).
- **M-4436 (M)** : registre des traitements : quatre traitements (patrimoine et prêts, maintenance, sécurité des locaux, reprographie) ; pas d'analyse d'impact a priori.

### P17 — Florian, juin–septembre 2027
- **M-4437 (H)** : la reprographie Next.js est **en usage réel** : elle bascule quand RM5 a fait ses preuves ; propriété unique **par site et par date** (§26 du 44) ; les demandes en cours finies dans Next.js. → **bascule recommandée aux vacances de la Toussaint 2027** (faible volume), pas à la rentrée.
- **M-4438 (H)** : les manuels de la rentrée 2027 : sans étiquettes (bloquant A), la campagne de distribution de septembre 2027 se fait **en quantité par élève**, et la campagne de retour de juin 2028 étiquette.
- **M-4439 (M)** : le 19 doit être en production pour les salles **avant** que le 44 déclare du matériel réservable (RM3 dépend d'AG3).
- **M-4440 (M)** : rien du 44 ne doit attendre le socle N1 pour la rentrée 2027 : le guichet des interventions est porté par le 44 (Q-RM1) et migre à N1 sans perte (les demandes gardent leur identifiant).

---

## 5. Vérification réglementaire (2026-11-11)
Faite pour la conception (§5 du 44). La passe confirme deux lectures : (1) la caution est proscrite pour les **manuels** seulement (réponse ministérielle de 1993) : pour le reste du matériel, aucune règle trouvée, d'où Q-RM2 ; (2) le barème dégressif est un **plafond favorable aux familles** (en cas d'année inconnue, le taux le plus bas, M-4402). Aucune source nouvelle n'a été consultée.

---

## 6. Manques qui relèvent d'autres socles
| Socle | Manque | Recommandation |
|---|---|---|
| **19** | **DRM-14, DRM-15** (annulation et modification de la réservation d'autrui sans contrôle serveur), DRM-16 à DRM-19 | ajouter aux défauts ND du 19 et aux tests d'AG3 |
| 19 | Recherche d'une salle de repli filtrée sur les équipements en service (M-4413) | port `ressources::equipements` |
| 6 | Agents territoriaux affectés avec compte technique (M-4408) ; habilitation `conduite_minibus` (M-4415) ; astreinte technique (bloquant B) | à ajouter au 6 |
| 8 | Escalade SMS de l'alerte technique (bloquant B) | dépend de Q-N5 du 8 |
| 13 | Dégradation imputée à un élève auteur (M-4428) | procédure du 13, facturation au 44 |
| 49 | Produits chimiques, registre des agents chimiques (M-4433) | santé et sécurité au travail |
| 24 | Mode suivi des pertes pendant le doublon (bloquant D) | export vers Charlemagne |
| N1 | Guichet des demandes (M-4440) | identifiant de demande conservé à la migration |

---

## 7. Synthèse

### 7.1 Manques par gravité
- **Bloquants (4)** : A (M-4401, M-4438), B (M-4407), C (M-4421), D (M-4404).
- **Hauts (21)** : M-4402, M-4403, M-4406, M-4408, M-4410, M-4411, M-4413, M-4415, M-4417, M-4419, M-4422, M-4424, M-4425, M-4427, M-4429, M-4433, M-4434, M-4437, et DRM-14, DRM-15 relayés au 19.
- **Moyens et faibles (15)** : le reste.

### 7.2 Ajouts dans la conception (`44_RESSOURCES_MATERIELLES.md` §30)
20 actions (RSM-250 à RSM-269), 6 règles (R-RM35 à R-RM40) et précisions de R-RM13, R-RM17, R-RM27 ; 12 cas limites (CRM-47 à CRM-58) ; questions Q-RM37 à Q-RM39 ; lots révisés.

### 7.3 Les dix ajouts les plus importants
1. Manuels suivis en quantité l'année 1, étiquetage progressif au retour (R-RM35).
2. Chaîne d'astreinte technique et escalade de l'alerte de sécurité (R-RM36).
3. Réceptionnaire de reprographie désigné pour un ou plusieurs sites (R-RM27 précisée).
4. Mode suivi des pertes pendant le doublon, export vers Charlemagne (R-RM37).
5. DRM-14 et DRM-15 relayés au 19 (sécurité de la réservation de salles en production).
6. Un départ d'élève n'est jamais bloqué par un prêt (R-RM17 précisée).
7. Année de mise en service inconnue : taux le plus favorable à la famille (R-RM13 précisée).
8. Dossier de commission de sécurité en un export.
9. Prêt de remplacement lié à une panne d'appareil prêté.
10. Aucune donnée du 44 pour évaluer un personnel (R-RM40).

### 7.4 Ce que la passe a montré
La conception était juste sur le **modèle** mais trop **idéale sur le départ** : l'établissement réel n'a pas d'étiquettes, pas d'astreinte, un seul reprographe et des finances dans Charlemagne. Les quatre bloquants rendent le socle **utilisable dès la rentrée 2027** sans renoncer au modèle cible.

---

## 8. Décisions

### 8.1 Appliquées par défaut (D-031)
| # | Décision |
|---|---|
| A | Manuels en quantité par élève l'année 1 ; étiquetage au retour de juin 2028 (R-RM35) |
| B | Astreinte technique par semaine ; escalade après 15 min ; numéros d'urgence affichés (R-RM36) |
| C | Réceptionnaire désigné explicitement pour un ou plusieurs sites (R-RM27 précisée) |
| D | Pertes en mode suivi pendant le doublon, export (R-RM37) |
| E | Doublon proposé par salle, quelle que soit la catégorie (7 jours) |
| F | Départ d'un élève jamais bloqué ; « à recouvrer » en un geste |
| G | Organigramme des clés facultatif ; inventaire des clés en lot au départ (R-RM38) |
| H | Dégradation d'une chambre : jamais facturée à toute la chambre par défaut (R-RM39) |
| I | Aucune donnée du 44 pour évaluer un personnel (R-RM40) |
| J | Bascule de la reprographie aux vacances de la Toussaint 2027 |

### 8.2 Soumis à Florian
| Q | Question | Recommandation |
|---|---|---|
| Q-RM37 | Produits chimiques et registre des agents chimiques : quel socle, quand ? | socle 49 (santé et sécurité au travail), hors V1 |
| Q-RM38 | Date de bascule de la reprographie Next.js | vacances de la Toussaint 2027, après une semaine de parallèle sur un site |
| Q-RM39 | Astreinte technique dans l'établissement pilote : qui, quels numéros (concessionnaires, chauffagiste) ? | à fournir par l'établissement |

---

## 9. Faits, hypothèses, contradictions
**Faits [F]** : `app/api/reservation-rooms/reservations/delete/route.ts:33-61` et `update/route.ts:14-24` (session seule) ; `app/lib/module-access-defaults.ts:273` (réceptionnaire « accueil » par défaut) ; `app/lib/photocopies-couleur-access.ts:51-58` (direction du site).
**Hypothèses [H]** : manuels de l'établissement non étiquetés ; Charlemagne ne suit pas les exemplaires de manuels ; un seul reprographe pour les sites ; aucune astreinte technique formalisée.
**Contradictions** : (1) R-RM27 (portée par site) contre l'organisation à un seul reprographe : précisée (bloquant C) ; (2) R-RM17 pouvait se lire comme un blocage du départ : précisée (M-4419) ; (3) la liste ND du 19 ne contient pas DRM-14 et DRM-15 : à ajouter au 19.
