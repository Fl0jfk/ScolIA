# Socle 33 — Visioconférence et continuité pédagogique

**Date :** 2026-11-11 — **Statut :** conception de fond, **non décidée** (à relire par Florian). Rédigée **avant** tout code Rust du domaine. Aucun code écrit ni modifié ; aucun accès à une base, à un secret ou à un service de production. Le code Next.js des appels de la messagerie a été **lu seulement**, de façon ciblée.
**Branche :** `ent-socle-32-33-conception`, partie de `origin/ent-mc0hoe`.
**Passe personas :** `docs/socles/PERSONAS_33.md` ; ajouts versés au **§30**, marqués **« ajout personas »** ; **en cas de contradiction, le §30 l'emporte**.
**À lire avec :** `CLAUDE.md` (« ne pas adopter une dépendance/API cloud comme solution finale par défaut », « ne propose pas d'alternatives non demandées sans expliquer et demander accord ») ; `docs/DECISIONS.md` (D-002 tout en Rust, D-003 mobile, D-008/D-010 Leptos, D-017, D-023, D-026, D-031, D-034, D-035, D-036, D-038) ; `docs/PRA_SAUVEGARDES.md` §18.4 ; `docs/MATRICE_CHARLEMAGNE.md` (COM-05) ; conceptions `10_EMPLOI_DU_TEMPS.md` (statut de séance `distance`, CL-68), `12_ASSIDUITE.md`, `15_ABSENCES_PERSONNELS.md` (épidémie), `16_MESSAGERIE.md` (§ « Visio depuis une conversation », fils adulte–mineur, boîtes C3), `18_FORMULAIRES.md` (usage `enregistrement_visio`, CK-65), `19_AGENDA_RDV.md` (modalité visio, R-AG22), `20_CAHIER_DE_TEXTES.md` (R-23, CDT-129, fermeture), `23_BULLETINS.md` (CB-07, BUL-154), `32_BUREAUTIQUE.md` (supports et tableau partagés, dossiers de continuité), `IA_COEUR.md`.

**Conventions** : [F], [V], [V-extrait] (page officielle en 403, lue par extrait de moteur de recherche), [S], [NV], [H], [R] comme au socle 32.

**Numérotation propre au socle :** existant Next.js **NX33-01…** ; défauts **ND33-01…** ; actions **VIS-100 à VIS-299** ; règles **R-VI1…** ; cas limites **CVI-01…** ; questions **Q-VI1…** ; réglages **S-3301…** ; manques (passe personas) **M-3301…**.

**Plage de migrations imposée : 2660–2679** (vérifiée libre sur les 134 branches distantes le 2026-11-11).

---

## 0. En bref

1. **Ce qui existe.** Côté Rust : **rien** [F]. Côté Next.js : des **appels vidéo maison dans la messagerie** des personnels (WebRTC en **maillage**, sans serveur média, **6 participants au plus**, sonnerie, accepter / refuser, couper micro et caméra, fenêtre flottante) [F `app/components/messaging/MessagingCallProvider.tsx`, `app/api/messaging/conversations/[id]/call/route.ts`] ; ni partage d'écran, ni enregistrement, ni classe virtuelle, ni élèves.
2. **Défauts de Next.js signalés à Florian** (§2.3, Next.js intouchable) : le **STUN public de Google** est utilisé par défaut (`stun.l.google.com`, `route.ts:330-334`) : **l'adresse IP de chaque participant est communiquée à Google** — point RGPD à porter au registre ou à corriger dans Next.js par Florian ; **identifiants TURN statiques** envoyés en clair à tous les clients ; état des appels **en mémoire du processus** (appel « introuvable » dès qu'il y a deux instances) ; aucune trace.
3. **La thèse (§1).** Une visio n'est pas un lien : c'est **une séance de l'établissement tenue à distance**. Chaque salle naît d'un **contexte** qui existe déjà ailleurs (une séance de cours du 10, un rendez-vous du 19, une réunion ou un conseil du 19 / 23 / 46, une conversation du 16) et **en hérite** ses participants, ses droits, son horaire et son cadre. Pas de salle « libre » ouverte au monde, pas de lien public.
4. **La brique média : recommandation à valider (Q-VI1).** Après vérification des licences et de l'activité (§11) : **LiveKit** (serveur média SFU en Go, **Apache-2.0**, v1.13.11 du 25 octobre 2026, TURN intégré, jetons JWT, enregistrement par *Egress* Apache-2.0 vers un stockage S3) **auto-hébergé chez Scaleway**, piloté depuis axum par la crate Rust `livekit-api` (Apache-2.0). C'est aussi le moteur de **Visio** de l'État (DINUM, MIT) et le moteur média par défaut de **BigBlueButton 4.0** [S]. **Aucune API cloud** (pas de LiveKit Cloud). Alternatives écrites et comparées (§11.3) : Galène (MIT), BigBlueButton (LGPL-3.0), Jitsi (Apache-2.0), mediasoup (ISC), str0m (MIT/Apache, Rust). ScolIA isole la brique derrière un **port `visio`** : la changer ne change pas les socles.
5. **Le client web : contradiction à trancher (Q-VI2).** Il n'existe pas de client LiveKit pour le navigateur en Rust/WASM (le SDK Rust vise les clients natifs) [S] : la recommandation est le **SDK JavaScript officiel** (`livekit-client`, Apache-2.0) **dans un îlot isolé**, version figée, exception écrite à D-002 comme l'est l'éditeur ArtCraft ; à défaut, un client Rust/WASM minimal sur `web-sys` (plusieurs mois de travail). Mobile (D-003) : SDK Swift et Android de LiveKit (Apache-2.0) [NV].
6. **Le cadre (§5).** **Caméra jamais obligatoire** pour un élève (CNIL : « En principe, non ! ») [V] ; **micro et caméra coupés à l'entrée** pour les élèves ; **enregistrement désactivé par défaut** (comme la classe virtuelle d'apps.education) [V-extrait], possible seulement avec l'accord du socle 18 (`enregistrement_visio`), indicateur visible, conservation courte ; **pas de télésurveillance d'examen** (recommandation CNIL 2023-058 : alternative en présence) [V] ; **pas de visio entre élèves sans adulte** ; **pas de visio en tête-à-tête adulte–mineur hors d'un cadre institutionnel tracé** (rendez-vous du 19, séance, boîte de service).
7. **La continuité pédagogique (§13).** Le plan est un **objet métier** : **préparé et formalisé à chaque rentrée** par le chef d'établissement, public et privé sous contrat [V-extrait] ; deux scénarios au moins (hybride, à distance) ; à l'activation : séances passées en `distance` (10), classes virtuelles créées, travail en autonomie au cahier de textes (20), dossiers de continuité dans les espaces (32), information des familles (8, 17), **contact de chaque élève au moins une fois par semaine** suivi [V : guide éduscol du 2/10/2026], repère **1/3 en classe virtuelle, 2/3 en autonomie** [V], prêt de matériel (44), attention aux élèves fragiles **sans détail de santé**.
8. **Volume.** **133 actions** numérotées dans la plage VIS-100 à VIS-299 (103 de la conception, 30 ajoutées par la passe personas), **70 cas limites**, **42 règles**, **14 lots** (VI0 à VI13). **Pas de lot 0 de correctifs Rust** (aucun code) ; **VI0 = preuve technique** (serveur média auto-hébergé, jetons, TURN, essai de charge sur le réseau d'un établissement) et inventaire des appels Next.js.
9. **Soumis à Florian** (§28.2) : choix de la brique média (Q-VI1) ; exception JavaScript au « tout en Rust » pour le client (Q-VI2) ; coût du serveur média et de la bande passante (Q-VI3) ; traitement du STUN Google dans Next.js (Q-VI4) ; raccordement éventuel à la classe virtuelle nationale (Q-VI5) ; enregistrement des cours (DPO, Q-VI6).

---

## 1. Thèse : une visio est une séance de l'établissement tenue à distance

Huit principes [R] :

1. **Pas de salle sans contexte.** Une salle est créée **à partir** d'une séance (10), d'un rendez-vous (19), d'une réunion (19), d'un conseil (23), d'une instance (46), d'une conversation (16), d'une permanence de service (16) ou d'un plan de continuité (§13). Elle hérite des **participants attendus** et de l'**horaire** ; elle ne les recopie pas.
2. **Les droits viennent du contexte.** Entrer dans la salle d'un cours = être du public du cours ou son professeur à la date ; dans celle d'un rendez-vous = en être participant ; d'un conseil = en être membre. Le jeton du serveur média n'est émis qu'**après** ce contrôle, pour une salle, une personne, une durée courte (R-VI2).
3. **L'hôte tient la salle.** Rôles fermés : **hôte** (adulte responsable), **co-hôte**, **intervenant** (peut parler et partager), **participant** (élève : micro et caméra coupés à l'entrée, main levée), **observateur** (voit et entend, ne publie pas). Salle d'attente selon le contexte (obligatoire pour les invités extérieurs).
4. **La présence en visio n'est pas l'assiduité.** Le journal technique (entrée, sortie) **propose** l'appel ; c'est le professeur qui le valide au socle 12 (R-VI20).
5. **Rien n'est gardé sans raison.** Pas d'enregistrement par défaut ; la conversation écrite d'une séance s'efface à la fin (sauf réunion avec compte rendu) ; seules restent les **métadonnées** (qui, quand, combien de temps) pour la durée de sécurité.
6. **Les mineurs sont protégés.** Pas de salle entre élèves sans adulte ; les sous-groupes d'élèves restent sous l'œil de l'hôte (il peut y entrer à tout moment, ils le savent) ; pas de tête-à-tête adulte–mineur hors cadre ; pas d'obligation de caméra ; floutage d'arrière-plan proposé ; aucune capture par l'établissement.
7. **Souveraineté.** Serveur média, TURN, enregistrements sur nos serveurs en France ; aucun STUN ni service public d'un tiers ; aucune API cloud par défaut ; la brique est remplaçable (port `visio`).
8. **La continuité est un plan, pas une improvisation.** Préparé à la rentrée, activé en un geste, suivi élève par élève, clos par un bilan.

---

## 2. Évaluation franche de l'existant

### 2.1 ENT Rust [F]
Rien pour la visio ni la continuité. Points d'accroche existants ou conçus : statut de séance `distance` [R, `10_EMPLOI_DU_TEMPS.md`] ; rendez-vous (`ent_creneau_rdv`, `rencontres.rs`) [F] avec modalité visio conçue au 19 ; temps réel de la messagerie (socle 16) ; `ent-push` (avis mobiles) [F].

### 2.2 Ce que fait Next.js (appels de la messagerie) [F]

| N° | Action | Source | Reprise |
|---|---|---|---|
| NX33-01 | Lancer un appel vidéo depuis une conversation individuelle ou de groupe (un seul appel actif par conversation) | `call/route.ts:195-241` | VIS-110 |
| NX33-02 | Sonnerie entrante, accepter, refuser | `MessagingCallOverlay.tsx:46-78`, `route.ts:274-278` | VIS-111 |
| NX33-03 | Rejoindre un appel en cours | `route.ts:255-272` | VIS-112 |
| NX33-04 | Couper son micro, sa caméra | `MessagingCallProvider.tsx:377-391` | VIS-140 |
| NX33-05 | Quitter ; terminer pour tous (hôte, ou dernier parti) | `route.ts:302-315` | VIS-113, VIS-114 |
| NX33-06 | Fenêtre flottante déplaçable, mode côte à côte | `MessagingCallOverlay.tsx:82-128` | VIS-141 |
| NX33-07 | Vidéo 1280×720 avec annulation d'écho et réduction de bruit | `Provider.tsx:130-143` | VIS-142 |
| NX33-08 | Purge des appels vides ou de plus de 2 h | `route.ts:100-111` | R-VI10 |

Règles reprises : appel réservé aux personnels (élèves exclus, `messaging/access.ts:18-26`), participant de la conversation vérifié (`route.ts:122, :251, :282`).

### 2.3 Défauts trouvés dans Next.js (signalés à Florian, Next.js intouchable)

| N° | Défaut | Source | Gravité | Correctif côté ScolIA |
|---|---|---|---|---|
| ND33-01 | **STUN public de Google** par défaut, côté serveur et client : IP des participants transmise à Google | `route.ts:330-334`, `Provider.tsx:69-74` | **élevée (RGPD)** | TURN/STUN auto-hébergés (R-VI30) |
| ND33-02 | Identifiants TURN **statiques et partagés**, renvoyés en clair à tous les clients | `route.ts:335-344, :239, :270` | élevée | identifiants éphémères (TURN intégré de LiveKit, ou REST TURN à HMAC) |
| ND33-03 | État des appels **en mémoire du processus** : « appel introuvable » avec plusieurs instances, tout perdu au redémarrage | `route.ts:39-50` | élevée | état dans le serveur média + table d'occupation |
| ND33-04 | Maillage : chaque participant envoie N-1 flux ; limité à 6 ; échecs derrière des NAT stricts sans TURN | `call-types.ts:3` | moyenne | serveur média (SFU) |
| ND33-05 | SDP et candidats relayés **sans contrôle** de taille ni de contenu | `route.ts:285-298` | moyenne | signalisation par le serveur média |
| ND33-06 | Rejoindre sans invitation ; refus notifié à l'hôte seul ; établissement de l'appel jamais revérifié | `route.ts:216, :255` | moyenne | jetons par personne, contrôle du contexte |
| ND33-07 | Aucune trace ni historique des appels | — | moyenne | journal des sessions (métadonnées) |
| ND33-08 | Pas de partage d'écran, pas de salle d'attente, pas de rôles | — | — | §9 |

### 2.4 Ce qu'il ne faut pas perdre
L'appel **en un geste depuis une conversation** entre collègues, la sonnerie, la fenêtre flottante qui laisse travailler à côté.

### 2.5 Verdict
Appels Next.js : **partiel, à refaire** (architecture inadaptée au-delà de 6 et au multi-instance). Classe virtuelle, réunions, conseils, continuité : **à concevoir de zéro**.

---

## 3. Acteurs, besoins réels, irritants

| Acteur | Ce qu'il fait | Ce qui lui fait perdre du temps | Ce qu'il redoute | Ce qui le ravirait |
|---|---|---|---|---|
| **Professeur** | Cours à distance, élève à l'hôpital ou empêché, option rare suivie à distance, aide individuelle | Créer un lien par cours, le copier dans le cahier de textes, l'envoyer aux familles ; faire l'appel à la main | Intrusion d'un inconnu ; élève qui filme et diffuse ; « on ne m'entend pas » ; être enregistré à son insu | La salle **déjà là** dans la séance de l'emploi du temps ; appel **pré-rempli** par la présence ; partage du document de l'espace ; sous-groupes en deux clics |
| **Élève** | Suit, participe, travaille en groupe | Ne trouve pas le lien ; connexion faible ; ordinateur partagé avec la fratrie | Être vu chez lui ; être enregistré ; se faire moquer | Entrer d'un clic depuis « Aujourd'hui » ; caméra libre ; rejoindre au téléphone si la connexion est mauvaise |
| **Élève hospitalisé, à domicile, empêché** | Suit certains cours à distance | Organisation au cas par cas | Être oublié | Une **présence à distance** prévue dans l'emploi du temps (sans dire pourquoi) |
| **Famille** | Rendez-vous avec un professeur, réunion parents-professeurs, aide d'un jeune enfant à se connecter | Se déplacer après le travail ; lien qui expire | Ne pas savoir se connecter ; être vue | Rendez-vous en visio **sans compte supplémentaire**, depuis le téléphone, avec interprète si besoin |
| **Parents séparés** | Rendez-vous chacun ou ensemble | Devoir se croiser | Que l'autre parent l'entende sans le savoir | Choix : rendez-vous commun ou séparé (19), et liste des présents visible de tous |
| **CPE, vie scolaire** | Entretiens, suivi des élèves à distance pendant une fermeture | Joindre chaque élève | Élève « perdu » | Liste des élèves **non joints cette semaine** |
| **Direction** | Conseils, réunions d'équipe, CA à distance, plan de continuité | Écrire le plan chaque rentrée de zéro ; tout piloter au téléphone le jour de la fermeture | Fermeture sans plan prêt ; une séance d'un conseil qui fuit | Plan **reconduit** et mis à jour ; activation **en un geste** ; tableau de suivi |
| **Secrétariat** | Organiser des réunions avec des extérieurs (MDPH, partenaires) | Liens Teams ou Zoom des partenaires | Données d'élèves dans un outil étranger | Inviter un extérieur par **lien nominatif** à salle d'attente |
| **Infirmière, psychologue, assistante sociale** | Entretiens confidentiels | Outils généraux non confidentiels | Qu'un tiers entende ; qu'une trace dise de quoi on a parlé | Salle **chiffrée de bout en bout**, sans enregistrement, sans IA, titre neutre partout |
| **AESH** | Accompagne un élève qui suit à distance | N'est pas dans la salle | — | Entrer comme co-participant de l'élève accompagné |
| **Interprète, intervenant** | Traduit un rendez-vous ; intervient dans un cours | Comptes, liens | — | Lien nominatif, rôle « intervenant » |
| **Administrateur, exploitation** | Capacité, pannes | Qualité perçue | Saturation un matin de fermeture | Tableau de charge, essais, bascule de capacité |
| **DPO** | Registre, AIPD, droits | Enregistrements oubliés | Enregistrement sans base légale ; données transmises à un tiers | Aucun enregistrement par défaut ; durée courte ; journal des accès |

---

## 4. L'année vue du terrain [H]

| Moment | Ce qui se passe | Ce que fait ScolIA |
|---|---|---|
| Rentrée | Plan de continuité à actualiser [V-extrait] ; coordonnées des familles à collecter [V] | Plan N reconduit depuis N-1, à relire et valider par le chef d'établissement (VIS-200) ; contrôle « familles sans moyen de contact » (4) |
| Toute l'année | Rendez-vous parents-professeurs, réunions, élèves empêchés, options à distance | salles des contextes |
| Trimestres | Conseils de classe, parfois à distance ou hybrides | salle du conseil, vue partagée sans téléchargement (23) |
| Hiver | Intempéries, épidémies | activation du plan (scénario hybride ou distance) |
| Examens | Pas de télésurveillance | refus (R-VI37) |
| Année de doublon 2027-2028 (D-034) | Charlemagne officiel ; appels Next.js en usage | la visio ScolIA est **nouvelle** (aucune fonction de Charlemagne n'est remplacée) : usage réel possible dès qu'elle est prête, sans doublon |

---

## 5. Ce qui est réglementaire (vérifié le 2026-11-11)

### 5.1 Continuité pédagogique [V]
- **Guide « Préparer son plan de continuité pédagogique »** (MEN, fichier du 2/10/2026, lu en entier) : plan **préparé et formalisé dès la rentrée** par le chef d'établissement ; apprentissages obligatoires « sur l'intégralité du temps scolaire », en présence, à distance ou en autonomie ; le chef d'établissement **harmonise les outils numériques**, **collecte les coordonnées des familles dès la rentrée**, fixe des objectifs puis évalue la progression, veille à **l'égalité de traitement** entre élèves d'une même classe ; scénarios **hybride** (accueil par niveau au moins à mi-temps, ou demi-groupes) et **entièrement à distance** ; repère indicatif **1/3 du temps en classe virtuelle, 2/3 en autonomie** ; matériel prêté par l'établissement ou la collectivité, à défaut accueil dans l'établissement ; à distance, **chaque élève contacté au moins une fois par semaine** ; vigilance sur les élèves fragiles et le décrochage. Source : `https://eduscol.education.gouv.fr/sites/default/files/document/preparer-son-plan-de-continuite-pedagogique-94851.pdf`.
- Plan **actualisé à chaque rentrée par tout établissement public et privé sous contrat** [V-extrait : `https://eduscol.education.gouv.fr/5058/preparer-son-plan-de-continuite-pedagogique`].
- Fiche « chefs d'établissement » : fermeture « quelle qu'en soit la cause (intempéries, épidémie…) », information des familles « aussitôt que possible » [V-extrait : `…/fichechefsetablissementcontinuitepedagogiquepdf-130159.pdf`].
- Cahier de textes : charge de travail et temps d'écran régulés, alternance visio et autonomie (`20_CAHIER_DE_TEXTES.md` R-23, déjà vérifié).
- Aucune circulaire récente dédiée aux intempéries trouvée [NV].

### 5.2 CNIL [V]
- **Caméra des élèves** : « En principe, non ! » ; obligation générale disproportionnée (minimisation, vie privée du foyer) ; exception si l'établissement justifie de circonstances précises et de l'absence de moyen moins intrusif, l'élève pouvant refuser pour des raisons tenant à sa situation (`https://www.cnil.fr/fr/cnil-direct/question/cours-en-ligne-les-eleves-sont-ils-obliges-dactiver-leur-camera`).
- **Choix des outils** : outils protecteurs, pas de réutilisation des données, chiffrement de bout en bout (`https://www.cnil.fr/fr/les-conseils-de-la-cnil-pour-utiliser-les-outils-de-visioconference`, 9/4/2020).
- **Télésurveillance des examens** : délibération n° 2023-058 du 8/6/2023 : proportionnalité, **alternative moins intrusive** (présence), opposition dans le public, enregistrement continu à 360° disproportionné, biométrie sur consentement avec alternative, aucune décision automatique (`https://www.cnil.fr/sites/cnil/files/2023-09/recommandation_dispositifs_de_telesurveillance_des_examens.pdf`) — vise surtout le supérieur ; ScolIA **n'en fait pas** (R-VI37).
- **Enregistrement des cours** : aucune position CNIL dédiée trouvée [NV] ; fiches académiques : consentement écrit de l'élève et du responsable [S] ; apps.education : enregistrement **désactivé par défaut** [V-extrait : `https://eduscol.education.gouv.fr/4914/le-service-classe-virtuelle-sur-la-plateforme-appseducationfr`].

### 5.3 Image et voix [V]
C. civ. art. 9 ; art. 372-1 (loi 2024-120 : droit à l'image protégé **en commun** par les parents, enfant associé) (`https://www.legifrance.gouv.fr/codes/article_lc/LEGIARTI000006426505`). Une captation (enregistrement, capture d'écran publiée) exige l'accord du socle 18 ; la simple transmission en direct d'une séance de cours relève de la mission d'enseignement [H, à confirmer par le DPO, Q-VI6].

### 5.4 Correspondance et confidentialité [V, rappel du 16]
Les échanges audio et vidéo privés relèvent du secret des correspondances (C. pén. 226-15 ; `16_MESSAGERIE.md` R-10) : aucune écoute ni « entrée discrète » d'un tiers dans une salle (hors sous-groupes d'élèves, où l'hôte peut entrer, ce que les élèves savent) (R-VI15).

### 5.5 Offres de l'État [V]
- **Visio (DINUM)** : code MIT, « Powered by LiveKit » ; instance réservée aux **agents publics** (ProConnect) ; une organisation non éligible peut **déployer sa propre instance** (`https://github.com/suitenumerique/meet`, `https://lasuite.numerique.gouv.fr/`) ; pas d'accès élèves ni familles.
- **Apps Education** : **classe virtuelle BigBlueButton** pour les usages pédagogiques avec les élèves, service de référence depuis l'arrêt de « Mes classes virtuelles » du CNED fin juillet 2022 [V-extrait] ; intégrée aux ENT qui en font la **demande** (FAQ DNE du 19/11/2025) ; **le privé sous contrat n'est pas concerné** par l'opposabilité de la Doctrine technique et n'a pas souhaité y participer [V : `https://www.education.gouv.fr/sites/default/files/document/Fiche%20Questions-R%C3%A9ponses%20du%20fil%20de%20discussion%20Tchap%20de%20la%20journ%C3%A9e%20nationale-476241.pdf`] ; **Visio-Agents** réservé aux agents du ministère.
- **Doctrine** : réponse ministérielle QE n° 10342 (JO 10/02/2026) : suites collaboratives d'éditeurs non européens à « proscrire », privilégier les ENT [V].
- Conséquence : ScolIA **héberge sa propre visio** (seul moyen d'ouvrir aux élèves et familles du privé et du public avec les mêmes règles) ; un raccordement à la classe virtuelle nationale pour les établissements publics est une **option** (Q-VI5).

### 5.6 Accessibilité [NV]
RGAA pour l'interface ; sous-titres et transcription : pas d'obligation trouvée pour une visio pédagogique ; interprète en langue des signes : au cas par cas (socle 35).

### 5.7 Ce qui reste à vérifier [NV]
Base légale de l'enregistrement d'un cours (mission d'intérêt public ou consentement) ; AIPD de l'ENT incluant la visio ; conservation des métadonnées de connexion (obligations des hébergeurs) ; SDET 2025 (classe virtuelle) ; responsabilité en cas de captation par un élève.

---

## 6. Branchements sur le cœur

### 6.1 Qui possède quoi
| Donnée | Propriétaire | Le 33 |
|---|---|---|
| Séances, statut `distance` | 10 | lit, crée la salle de la séance |
| Présence, appel | 12 | propose une présence (jamais n'écrit l'appel) |
| Rendez-vous, réunions, participants | 19 | crée la salle du rendez-vous, lit les participants |
| Conseils de classe | 23 | salle du conseil, vue partagée |
| Instances | 46 | salle de séance d'instance |
| Conversations, boîtes | 16 | appel depuis un fil, permanence d'une boîte |
| Supports, tableau | 32 | élément figé ou document ouvert en lecture |
| Travail en autonomie | 20 | lien vers la salle d'une séance, travaux de continuité |
| Accords d'enregistrement | 18 | vérifie `enregistrement_visio` |
| Avis | 8 | natures |
| Annonce de fermeture | 5 (fermeture), 17 (annonce) | consomme ; propose l'annonce |
| Matériel prêté | 44 | lit la liste des prêts de continuité |
| Coordonnées, moyens de contact | 4 | lit l'existence d'un moyen (pas le numéro) |
| Absences des personnels | 15 | consomme (professeur absent → classe virtuelle annulée) |
| BEP, AESH | 35 | lit la pastille « suit à distance » et l'AESH attestée |

### 6.2 Ports consommés
`edt::seance(seance_id)`, `edt::seances_du_jour(public)` (10) ; `assiduite::proposer_presence(seance, eleve, source=visio)` (12) ; `agenda::rdv(rdv_id)`, `agenda::participants(rdv_id)` (19) ; `conseils::membres(conseil_id)` (23) ; `messagerie::participants(fil_id)` (16) ; `espaces::element_fige` (32) ; `autorisations::verifier(sujets, 'cours_enregistre_classe', 'enregistrement_visio', date)` (18) ; `notifications::publier` (8).

### 6.3 Ports exposés (`visio`)
- `visio::salle_pour(contexte) -> salle_id` (idempotent par contexte) : pour 10, 16, 19, 23, 46, 20.
- `visio::presence(session_id) -> Vec<PresenceEleve>` : pour 12 (proposition d'appel).
- `visio::lien_entree(salle_id, personne) -> url` : lien court vers la page d'entrée (jamais vers le serveur média).
- `continuite::plan_actif(etab, date) -> Option<plan>` et `continuite::eleve_suivi(eleve, semaine)` : pour 29 (portail), 12, 20, 32.

### 6.4 Répercussions automatiques
| Fait | Effet |
|---|---|
| Séance passée en `distance` (10) | salle créée, lien dans la séance, le cahier de textes et « Aujourd'hui » (29) |
| Séance annulée (10, 15) | salle fermée, participants avertis par l'avis de la séance (pas un second avis) |
| Élève change de groupe (5) | droit d'entrée recalculé à la date |
| Rendez-vous déplacé ou annulé (19) | salle suit l'horaire, ou ferme |
| Accord d'enregistrement retiré (18) | enregistrements futurs : l'élève est exclu de la captation (CVI-25) ; existants : réexamen par l'hôte (VIS-175) |
| Élève majeur (2) | accord d'enregistrement de l'élève seul |
| Plan de continuité activé (§13) | séances du périmètre en `distance` proposées en lot à l'adjoint (10), dossiers de continuité (32), suivi hebdomadaire ouvert |

---

## 7. Concepts et vocabulaire

| Terme | Définition [R] |
|---|---|
| **Salle** | Lieu virtuel rattaché à un **contexte** (`seance`, `rdv`, `reunion`, `conseil`, `instance`, `fil`, `permanence`, `continuite`), avec un cadre (§8.2) |
| **Session** | Occurrence datée d'une salle (de l'ouverture à la fermeture) |
| **Participant attendu** | Personne qui peut entrer, déduite du contexte à la date |
| **Rôle de salle** | `hote`, `cohote`, `intervenant`, `participant`, `observateur` |
| **Salle d'attente** | Antichambre où l'hôte admet nominativement |
| **Sous-groupe** | Salle fille d'une session, à durée limitée, où l'hôte entre à volonté |
| **Présence technique** | Journal d'entrée et de sortie d'une personne dans une session (métadonnées) |
| **Proposition de présence** | Présence technique résumée envoyée au 12 (présent, entré en retard, sorti tôt, jamais connecté) |
| **Enregistrement** | Captation d'une session, toujours annoncée, jamais par défaut |
| **Plan de continuité** | Document versionné par année : scénarios, périmètres, outils, règles de contact, prêts, responsabilités |
| **Activation** | Mise en œuvre datée d'un scénario sur un périmètre (établissement, niveaux, classes) |
| **Suivi hebdomadaire** | Pour chaque élève du périmètre actif : contact établi ou non dans la semaine, par qui, par quel moyen (sans contenu) |

---

## 8. Salles

### 8.1 Création
Toujours par un contexte (VIS-100 à VIS-106) ; la salle d'une séance est **créée seule** quand la séance passe en `distance` (ou à la demande du professeur pour une séance hybride, un élève empêché) ; celle d'un rendez-vous quand sa modalité est « visio » ; celle d'une conversation au premier appel.

### 8.2 Cadres par contexte [R]

| Contexte | Hôte | Entrée | Salle d'attente | Élèves | Chat écrit | Enregistrement | E2EE |
|---|---|---|---|---|---|---|---|
| `seance` (cours, classe virtuelle) | professeur du cours (suppléant à la date) | public du cours | non par défaut ; oui pour les entrées après 10 min (S-3301) | participants, micro/caméra coupés | oui, effacé en fin de session | possible avec accord (§12) | non (enregistrement et sous-titres côté serveur incompatibles) |
| `rdv` (19) | hôte du rendez-vous | participants du rendez-vous | **oui** (l'hôte admet) | si invité au rendez-vous | oui, effacé | **jamais** | **oui** si disponible (S-3302) |
| `reunion` (19 : équipe, parents) | organisateur | invités | oui pour les extérieurs | selon réunion | oui, conservé avec le compte rendu si demandé | sur décision de l'organisateur avec information, jamais d'élève mineur | option |
| `conseil` (23) | président du conseil | membres du conseil à la date | oui | délégués élèves selon le conseil | non | **jamais** | option |
| `instance` (46) | président | membres | oui | élus | non | selon le règlement de l'instance [NV] | option |
| `fil` (16, appel entre personnels) | celui qui appelle | participants du fil | non | **interdit** (comme Next.js) | — | jamais | oui |
| `permanence` (boîte de service 16, ex. infirmerie, CPE) | membre de la boîte | personne qui a un rendez-vous ou un fil ouvert avec la boîte | **oui** | oui (élève suivi) | non | **jamais** | **oui** ; boîtes C3 : jamais d'IA, titre neutre |
| `continuite` (point hebdomadaire, cellule de crise) | direction ou PP | désignés | selon | selon | non | jamais | option |

### 8.3 Entrée
Depuis le contexte (séance du jour, rendez-vous, fil, « Aujourd'hui » du 29) : page d'entrée ScolIA (test micro et caméra, choix « caméra éteinte », floutage, nom affiché = nom d'usage) ; jeton émis à l'entrée (durée de validité 2 min pour se connecter, droits de la salle, identité = identifiant opaque, nom affiché) (R-VI2) ; ouverture de la salle 10 min avant l'horaire (S-3303) ; un participant attendu sans compte (parent sans compte, extérieur) entre par **lien nominatif à usage limité** (§10).

### 8.4 Pendant
Rôles et pouvoirs §9 ; qualité adaptée (simulcast) ; repli **audio seul** proposé si la bande passante est faible ; **rejoindre par téléphone** (passerelle SIP) : **hors V1** (coût d'opérateur, Q-VI7).

### 8.5 Fermeture
À l'heure de fin + 15 min (S-3304) avec avertissement, ou par l'hôte ; le dernier adulte qui sort d'une salle avec élèves **ferme** la salle (R-VI12) ; chat effacé ; présence technique transmise au 12.

---

## 9. Rôles, modération, outils

1. **Hôte** : admet, refuse, met en sourdine, retire la caméra d'un participant (sans l'allumer jamais, R-VI16), exclut (et interdit le retour pour la session), donne la parole, nomme un co-hôte, lance les sous-groupes, partage, termine.
2. **Participant (élève)** : micro et caméra **coupés à l'entrée** ; peut les ouvrir sauf si l'hôte a verrouillé ; **main levée**, réactions (liste fermée), chat (si ouvert), partage d'écran **sur autorisation** de l'hôte.
3. **Partage d'écran** : intervenants ; avertissement « vous partagez tout l'écran » ; partage d'onglet recommandé.
4. **Tableau partagé** : un document de l'espace du cours (32) **ouvert en lecture** à l'écran de l'hôte, ou un tableau blanc simple (dessin vectoriel en îlot Leptos, état éphémère, **enregistrable** par l'hôte comme image dans l'espace) (VIS-146) ; pas d'éditeur collaboratif temps réel (32 Q-ET2).
5. **Sous-groupes** : l'hôte répartit (aléatoire, groupes de l'espace du 32, à la main), fixe une durée ; il entre et sort à volonté, diffuse un message à tous ; à la fin, retour automatique en salle principale ; **jamais un sous-groupe sans que l'hôte puisse y entrer** ; les élèves sont prévenus (R-VI15).
6. **Sondage rapide** : question fermée, réponses agrégées anonymes ou nominatives annoncées, éphémère.
7. **Chat** : texte court, liens vers des éléments de l'espace ; **pas de pièces jointes** (passer par l'espace) ; message privé **seulement vers l'hôte** pour un élève (jamais entre élèves, R-VI14) ; effacé en fin de session (sauf réunion avec compte rendu).
8. **Signalement** par un participant (« quelqu'un filme », « intrus ») : alerte à l'hôte, trace au journal.
9. **Pas d'indicateur collectif** de caméras allumées ni de classement des participants par caméra (pression des pairs, ajout personas).
10. **Arrière-plan** : floutage et fonds neutres fournis, traitement **dans l'appareil** (aucun envoi d'image de la pièce au serveur au-delà du flux) ; pas de fond personnalisé téléversé par les élèves en V1.

---

## 10. Invités sans compte et extérieurs

1. **Parent sans compte** invité à un rendez-vous (19) : lien nominatif à usage limité (code envoyé par le canal du socle 8), valable pour ce rendez-vous, passage par la salle d'attente, nom affiché vérifié par l'hôte.
2. **Extérieur** (interprète, intervenant, partenaire MDPH, entreprise de stage) : invité nominatif par l'organisateur (droit `visio.inviter_externe`), adresse vérifiée par code, salle d'attente **obligatoire**, rôle `intervenant` ou `observateur`, jamais dans une salle de cours avec des élèves mineurs sans adulte de l'établissement présent (R-VI18).
3. **Aucun lien public** permanent ; aucun « lien de salle » réutilisable par n'importe qui (R-VI1).

---

## 11. La brique média (vérifiée le 2026-11-11) [S]

### 11.1 Exigences
Auto-hébergeable en France ; licence permissive ou copyleft faible (pas d'AGPL, pas de GPL liée) ; SFU ; TURN sans dépendance externe ; jetons par personne et par salle ; enregistrement optionnel vers S3 ; partage d'écran ; simulcast ; E2EE possible ; SDK mobiles natifs ; pilotable depuis Rust ; activité soutenue.

### 11.2 Briques vérifiées
| Brique | Licence | Activité | Points forts | Limites |
|---|---|---|---|---|
| **LiveKit server** | Apache-2.0 | v1.13.11 (25/10/2026), sorties mensuelles | SFU distribué (Go, Pion) ; JWT ; **TURN intégré** à authentification (TURN/TLS 5349 ou 443, TURN/UDP 443, média UDP 50000–60000) ; E2EE ; charge mesurée (16 cœurs : 150 publient et 150 reçoivent, ou 1 émetteur et 3 000 spectateurs) ; moteur de Visio (DINUM) et de BBB 4.0 | lobby, sous-groupes, tableau à construire côté application |
| LiveKit Egress | Apache-2.0 | actif | enregistrement salle (Chrome sans tête + GStreamer) ou pistes, vers S3 | Redis partagé ; bac à sable Chrome désactivé par défaut (à isoler) |
| `livekit-api` (Rust) | Apache-2.0 | `livekit` 0.10.1 (04/11/2026) | jetons, gestion des salles depuis axum | SDK client Rust = natif, pas navigateur |
| Galène | MIT | v1.2.2 (09/11/2026) | simple (Go), TURN intégré, enregistrement disque, sous-groupes, charge faible (≈ 100 élèves par ¼ de cœur) ; utilisé par des universités françaises | **pas d'E2EE** ; un mainteneur principal ; pas de lobby annoncé |
| BigBlueButton 4.0 | **LGPL-3.0** | v4.0.2 (06/11/2026) | classe virtuelle complète (tableau, sous-groupes, notes, sondages, enregistrement et relecture) ; utilisé par apps.education | lourd (Ubuntu 24.04 dédié, nombreux composants) ; interface propre (pas la nôtre) ; intégration par API, salle hors de l'ENT |
| Jitsi Meet / Videobridge | Apache-2.0 (Meet) ; JVB non confirmé ce jour | sorties régulières | complet, lobby, sous-groupes, JWT | piège JWT/sous-groupes ; enregistrement par Jibri peu documenté ; pile Java + Prosody |
| mediasoup | ISC | 0.33.2 (crate Rust, 04/11/2026) | bibliothèque SFU, crate Rust | ni TURN, ni enregistrement, ni interface : tout à construire |
| str0m | MIT/Apache-2.0 | 0.25.0 (03/11/2026) | WebRTC Rust « sans I/O », SFU possible | ni TURN, ni codecs ; panique sur bug interne ; très gros travail |
| coturn | BSD-3 | 4.18.0 (08/09/2026) | TURN de référence | inutile avec LiveKit (TURN intégré) |
| Janus | **GPL-3.0** | v1.4.3 | — | copyleft fort : **écarté** |

### 11.3 Recommandation (Q-VI1, soumise à Florian)
**LiveKit auto-hébergé** (serveur + TURN intégré ; Egress plus tard pour l'enregistrement), sur une instance Scaleway dédiée, piloté par axum (`livekit-api`) ; aucun service LiveKit Cloud. Pourquoi : licence Apache-2.0, activité, TURN intégré (pas de coturn à exploiter), E2EE, SDK mobiles, et **convergence** avec Visio de l'État et BigBlueButton 4.0 (compétences, interopérabilité future). **Repli** documenté : Galène (plus simple, MIT, mais sans E2EE) si l'exploitation de LiveKit se révèle trop lourde à l'essai VI0. **Non retenu** : BigBlueButton (interface étrangère à l'ENT, exploitation lourde) — mais son **raccordement** comme service national reste une option pour les établissements publics (Q-VI5) ; Janus (GPL-3.0) ; un SFU maison sur str0m (trop tôt). **Rien n'est adopté sans votre accord.**

### 11.4 Client
- **Web** : SDK `livekit-client` (JavaScript, Apache-2.0) dans **un îlot isolé** de la page de salle, version figée et vérifiée (empreinte), chargée depuis notre origine, sans CDN ; l'interface (boutons, liste, chat, sous-groupes) reste en Leptos et dialogue avec l'îlot par une interface étroite. C'est une **exception à D-002** à faire valider (Q-VI2), du même ordre que l'éditeur ArtCraft (code tiers isolé). Alternative : client Rust/WASM sur `web-sys` (`RTCPeerConnection` du navigateur) parlant le protocole LiveKit (`livekit-protocol`, Apache-2.0) : plus cohérent, beaucoup plus long.
- **Mobile** (D-003) : SDK LiveKit Swift (iOS) et Android (Kotlin) [NV licences : Apache-2.0 attendu], écrans natifs ; usage quotidien : rejoindre un cours, un rendez-vous ; pas d'hôte de classe virtuelle sur téléphone en V1 (tablette : oui).

### 11.5 Exploitation [R]
Une instance dédiée (réseau : ports UDP ouverts, 443 TCP/TLS pour les réseaux d'établissement filtrés) ; supervision (`ent-obs`) : sessions, participants, débit, pertes ; **capacité à dimensionner** pour un matin de fermeture (toutes les classes en même temps) : essai de charge au lot VI0 ; **coût** (instance et bande passante sortante) à chiffrer avec Florian (Q-VI3) ; mises à jour mensuelles après recette ; clés API du serveur média inscrites à l'inventaire des clés (E11) ; aucune donnée d'établissement dans les noms de salles (identifiants opaques, E14).

---

## 12. Enregistrement

1. **Désactivé par défaut** pour tout l'établissement (S-3305) ; activable par la direction pour les **cours** seulement, jamais pour les rendez-vous, permanences, conseils (R-VI22).
2. **Conditions** à chaque enregistrement : (a) accord `enregistrement_visio` valable au socle 18 pour **chaque** élève mineur présent (titulaires) ou majeur (lui-même) ; (b) annonce avant le début, **indicateur permanent** visible de tous, rappel aux entrants ; (c) élève sans accord : **exclu de la captation** (son micro, sa caméra, son nom et ses messages ne sont pas enregistrés ; il le sait) ou l'hôte renonce (repris de `18_FORMULAIRES.md` CK-65) ; (d) par défaut **seulement l'écran partagé et la voix de l'hôte** (S-3306) (R-VI23).
3. **Stockage** : objet immuable dans l'espace du cours (32), visible du public du cours **en lecture en ligne seulement** (pas de téléchargement), conservation **jusqu'à la fin de la période** (trimestre) puis effacement (S-3307, DPO) (R-VI24).
4. **Interdits** : enregistrement par un élève (l'interface ne le propose pas ; une captation par un autre moyen relève du règlement intérieur et du socle 13) ; diffusion hors de l'espace du cours ; analyse par l'IA sans accord spécifique (§16).

---

## 13. Continuité pédagogique

### 13.1 Le plan (objet métier)
- **Un plan par année**, versionné, **reconduit** depuis N-1 et **validé** par le chef d'établissement à la rentrée (rappel à la rentrée tant qu'il n'est pas validé) (R-VI26).
- **Contenu** (trame ScolIA, D-023) : contacts et responsabilités (cellule : direction, adjoint, CPE, référent numérique), **outils harmonisés** (ScolIA : espaces, cahier de textes, classe virtuelle, messagerie ; rien d'autre par défaut), **scénarios** (`hybride_demi_groupes`, `hybride_niveaux`, `distance_totale`, `classe_fermee`, `eleve_empeche` — ce dernier hors plan : il relève de l'aménagement individuel), pour chaque scénario la **règle de temps** (repère 1/3 synchrone, 2/3 autonomie, ajustable par niveau), les **priorités** (niveaux à examen), la **règle de contact** (au moins une fois par semaine, par le PP ou un adulte désigné), les **prêts de matériel**, l'**accueil** des élèves sans équipement ou des enfants de personnels prioritaires [NV], la **communication** aux familles (trames d'annonce du 17), le **bilan**.
- Contrôles de préparation (déterministes) : familles **sans aucun moyen de contact** (4) ; élèves déclarés **sans équipement** (question de rentrée facultative, socle 18) ; personnels sans compte actif ; classes sans espace de cours ouvert (32) ; professeurs n'ayant jamais ouvert une classe virtuelle (proposer une séance d'essai).

### 13.2 Activation
- **Déclencheurs** : décision de la direction (fermeture préfectorale, intempéries, épidémie, travaux), éventuellement suggérée par une **fermeture** déclarée au socle 5 ou un **taux d'absence** des personnels (15) — jamais automatique (R-VI27).
- **Aperçu** (N3) : périmètre (établissement, niveaux, classes), dates, scénario ; effets listés : séances passées en `distance` ou en autonomie (lot proposé à l'adjoint, socle 10), salles créées, dossiers de continuité (32, ETR-284), annonce aux familles prête (17) et avis (8), cahier de textes prévenu (20 : une seule fenêtre « vos séances à distance »), portail « Aujourd'hui » (29) en mode continuité.
- **Scénario hybride** : demi-groupes générés (alphabétique, groupes existants, à la main) et alternance semaine A/B ou jours ; les élèves à la maison suivent en classe virtuelle **ou** en autonomie selon le plan.

### 13.3 Suivi
- **Tableau de continuité** (direction, CPE, PP) : par classe, élèves **contactés cette semaine** / non contactés ; présences en classe virtuelle (agrégées) ; travaux remis (20) ; ouvertures des espaces (32, ETR-285) ; aucune note, aucun score (R-VI29).
- **Contact hebdomadaire** : un adulte marque « contact établi » pour un élève (moyen : classe virtuelle, message, téléphone, visite ; sans contenu) ; élèves non joints au jeudi : liste au PP puis au CPE (S-3308).
- **Élèves fragiles** : liste « à contacter en priorité » tenue par le CPE et la direction, **sans motif** (aucune donnée de santé, de handicap ni sociale n'apparaît ; le motif reste dans le socle propriétaire) (R-VI28).
- **Charge de travail** : la charge estimée du cahier de textes (20) par classe et par jour est affichée à l'équipe ; alerte si elle dépasse le repère du plan.
- **Prêts** : liste des matériels prêtés pour la continuité (44), retour attendu à la désactivation.

### 13.4 Désactivation et bilan
Fin datée (N2), retour des séances à leur statut, annonce aux familles ; **bilan** (trame) : durée, participation agrégée, élèves non joints (nombre), difficultés, matériel non rendu ; versé au plan de l'année suivante.

---

## 14. Présence et appel

1. La présence technique (entrées, sorties, durée) est résumée par élève : `present`, `retard` (entrée > 10 min après le début, S-3309), `sorti_tot`, `jamais_connecte`, `deconnexions` (nombre) (R-VI20).
2. Elle est **proposée** au professeur dans l'appel du socle 12 (« présent en visio 52 min ») ; il valide, corrige, ou marque « absent » ; un élève connecté mais caméra éteinte est **présent** (R-VI21).
3. Un élève **jamais connecté** à une classe virtuelle obligatoire : absence proposée ; pendant une continuité, il entre dans la liste « non joints » s'il n'a aucun autre contact.
4. Les métadonnées de connexion ne servent **à rien d'autre** (ni évaluation, ni discipline automatique) ; elles sont effacées selon §20.3.

---

## 15. Sécurité

1. **Jetons** : émis par axum après contrôle du contexte ; durée de connexion 2 min ; droits minimaux (publier ou non, partager l'écran ou non, s'abonner) ; identité opaque ; renouvellement par le serveur pendant la session ; un retrait de droit (exclusion, fin de rendez-vous) coupe la personne par l'API du serveur média (R-VI2, R-VI3).
2. **Salles** : nom = UUID ; aucune donnée personnelle dans le nom ni les métadonnées de salle (E14).
3. **TURN/STUN** : uniquement ceux du serveur média, identifiants éphémères ; **aucun STUN tiers** (correctif de ND33-01) (R-VI30).
4. **E2EE** : activé pour `rdv`, `permanence`, `fil` (S-3302) quand le client le permet ; désactivé où l'enregistrement ou le traitement serveur est requis ; l'interface affiche l'état.
5. **Isolation du client JS** : îlot sans accès aux données de l'ENT hors jeton et nom affiché ; CSP stricte (`connect-src` limité au serveur média et à notre origine) ; version figée par empreinte (SRI).
6. **Intrusion** (« zoombombing ») : impossible sans jeton ; salle d'attente pour tout extérieur ; verrouillage de la salle par l'hôte ; exclusion avec interdiction de retour.
7. **Captation** : l'interface ne propose aucune capture côté élève ; filigrane léger du nom de l'utilisateur sur la vidéo affichée aux élèves dans les classes virtuelles (dissuasif, S-3310) [R, Q-VI8].
8. **Journal** (E7) : création de salle, entrées et sorties, admissions, exclusions, enregistrements (début, fin, accords vérifiés), invitations externes ; **jamais le contenu** (audio, vidéo, chat).
9. **Déni de service** : limites par personne (salles ouvertes, sessions simultanées), par établissement (capacité réservée) ; priorité aux contextes `seance` en continuité.

---

## 16. Intelligence artificielle

### 16.1 Ce que l'IA fait [R]
- **Préparer le plan de continuité** à partir de la trame et du plan N-1 (N1, brouillon) ; **proposer** la répartition en demi-groupes (N2) ; **rédiger** l'annonce aux familles (N3 : envoi par le 17).
- **Résumer** une réunion **sur transcription consentie** (hors V1 : transcription locale par un modèle hébergé chez nous, à étudier ; jamais pour `conseil`, `rdv`, `permanence`) (Q-VI9).
- **Proposer** au professeur l'appel depuis la présence (déterministe, pas d'IA nécessaire).
### 16.2 Ce que l'IA ne fait pas
Pas d'écoute, pas d'analyse des visages ni des émotions, pas de mesure d'« attention », pas de détection de triche, pas de transcription d'une salle avec élève sans accord, **jamais** sur `permanence` (C3) ni `rdv` ; aucune admission ni exclusion (R-VI35, R-VI36).
### 16.3 Capacités déclarées
Lectures : `continuite.plan.lire` (C1), `continuite.suivi.lire` (C1, agrégé). Commandes : `VIS-200` préparer le plan (N1), `VIS-212` proposer des demi-groupes (N2), `VIS-215` préparer l'annonce (N3 via 17). Fermées : salles, admission, enregistrement, présence.

---

## 17. Actions métier (VIS-100 à VIS-245 ; VIS-246 à VIS-299 au §30)

Acteurs : **HOT** hôte, **PRF**, **ELV**, **PAR**, **PER**, **DIR**, **ADJ** adjoint, **CPE**, **PP**, **ADM**, **EXT** extérieur, **EXP** exploitation ScolIA.

### 17.1 Salles et sessions
| N° | Action | Acteur · droit | Règles | Événement | Lot |
|---|---|---|---|---|---|
| VIS-100 | Créer la salle d'une séance passée en `distance` | système | contexte `seance` ; idempotent | `visio.salle_creee` | VI2 |
| VIS-101 | Ouvrir une salle pour une séance en présence (élève empêché, hybride) | PRF · `visio.animer` | participants = public du cours ; signalé à l'adjoint | idem | VI2 |
| VIS-102 | Créer la salle d'un rendez-vous en visio (19) | système | contexte `rdv` | idem | VI1 |
| VIS-103 | Créer la salle d'une réunion (19), d'un conseil (23), d'une instance (46) | organisateur | cadres §8.2 | idem | VI5 |
| VIS-104 | Créer une salle de permanence d'une boîte de service (16) | membre de la boîte | jamais d'enregistrement, E2EE | idem | VI5 |
| VIS-105 | Créer la salle d'une cellule de continuité | DIR | contexte `continuite` | idem | VI6 |
| VIS-106 | Voir les salles du jour (« Aujourd'hui ») | tous | contextes du jour lisibles | — | VI2 |
| VIS-107 | Ouvrir la salle avant l'horaire (anticipation) | HOT | jusqu'à 30 min (S-3303) | `visio.session_ouverte` | VI2 |
| VIS-108 | Fermer la salle, terminer pour tous | HOT | chat effacé, présence transmise | `visio.session_fermee` | VI1 |
| VIS-109 | Prolonger la session | HOT | 15 min par geste | — | VI2 |
| VIS-110 | Appeler un collègue ou un groupe depuis une conversation (NX33-01) | PER · `visio.appeler` | contexte `fil` ; personnels seulement | `visio.appel_lance` | VI1 |
| VIS-111 | Recevoir un appel, accepter, refuser (NX33-02) | PER | sonnerie web et mobile (push) | — | VI1 |
| VIS-112 | Rejoindre un appel en cours (NX33-03) | participant du fil | jeton individuel | — | VI1 |
| VIS-113 | Quitter (NX33-05) | tous | — | — | VI1 |
| VIS-114 | Terminer un appel pour tous (NX33-05) | lanceur | — | — | VI1 |
| VIS-115 | Entrer dans une salle (page d'entrée, test du matériel, caméra éteinte possible) | participant attendu | jeton 2 min | `visio.entree` | VI1 |
| VIS-116 | Rejoindre en audio seul | tous | — | — | VI1 |
| VIS-117 | Voir les participants et leur rôle | tous | élèves : noms d'usage ; extérieurs marqués | — | VI1 |
### 17.2 Admission, rôles, modération
| N° | Action | Acteur · droit | Règles | Événement | Lot |
|---|---|---|---|---|---|
| VIS-120 | Admettre ou refuser depuis la salle d'attente | HOT, COH | nominatif | `visio.admis` / `.refuse` | VI1 |
| VIS-121 | Admettre tous les participants attendus | HOT | jamais les extérieurs en lot | — | VI2 |
| VIS-122 | Nommer un co-hôte | HOT | adulte de l'établissement | — | VI2 |
| VIS-123 | Donner ou retirer la parole, le partage d'écran | HOT | — | — | VI2 |
| VIS-124 | Couper le micro d'un participant, de tous | HOT | — | — | VI2 |
| VIS-125 | Éteindre la caméra d'un participant (jamais l'allumer) | HOT | R-VI16 | — | VI2 |
| VIS-126 | Verrouiller micros et caméras des élèves | HOT | — | — | VI2 |
| VIS-127 | Exclure un participant, interdire son retour pour la session | HOT | motif codé ; journal | `visio.exclu` | VI2 |
| VIS-128 | Verrouiller la salle (plus aucune entrée) | HOT | — | — | VI2 |
| VIS-129 | Signaler un problème (intrus, captation, propos) | tous | alerte à l'hôte ; journal | `visio.signalement` | VI2 |
| VIS-130 | Transmettre un incident de séance à la vie scolaire (13) | HOT | proposition de fait | — | VI2 |
| VIS-131 | Diffuser un message à toute la salle (y compris sous-groupes) | HOT | — | — | VI4 |
### 17.3 Médias et outils
| N° | Action | Acteur · droit | Règles | Événement | Lot |
|---|---|---|---|---|---|
| VIS-140 | Couper et rétablir son micro et sa caméra (NX33-04) | tous | sauf verrou | — | VI1 |
| VIS-141 | Réduire la salle en fenêtre flottante (NX33-06) | tous | web | — | VI1 |
| VIS-142 | Choisir micro, caméra, haut-parleur ; qualité réduite | tous | — | — | VI1 |
| VIS-143 | Flouter l'arrière-plan, choisir un fond neutre | tous | traitement local | — | VI2 |
| VIS-144 | Partager son écran, un onglet, une fenêtre | intervenant, autorisé | avertissement plein écran | — | VI2 |
| VIS-145 | Montrer un élément de l'espace du cours à l'écran (32) | HOT | lecture seule, pas de téléchargement imposé | — | VI4 |
| VIS-146 | Utiliser un tableau blanc ; l'enregistrer comme image dans l'espace | HOT ; élèves si autorisés | état éphémère | — | VI4 |
| VIS-147 | Lever la main, réagir (liste fermée) | tous | file des mains | — | VI2 |
| VIS-148 | Écrire dans le chat ; message privé à l'hôte | tous (si ouvert) | jamais entre élèves | — | VI2 |
| VIS-149 | Ouvrir, fermer le chat | HOT | — | — | VI2 |
| VIS-150 | Lancer un sondage rapide | HOT | agrégé ; nominatif annoncé | — | VI4 |
| VIS-151 | Créer des sous-groupes (aléatoire, groupes de l'espace, à la main) avec une durée | HOT | R-VI15 | `visio.sous_groupes` | VI4 |
| VIS-152 | Entrer dans un sous-groupe, en sortir | HOT, COH | les élèves voient l'hôte entrer | — | VI4 |
| VIS-153 | Rappeler tout le monde en salle principale | HOT | — | — | VI4 |
| VIS-154 | Partager l'écran dans un sous-groupe | élève autorisé | — | — | VI4 |
| VIS-155 | Afficher les sous-titres (transcription locale) | tous | hors V1 (Q-VI9) | — | VI11 |
### 17.4 Présence
| N° | Action | Acteur · droit | Règles | Événement | Lot |
|---|---|---|---|---|---|
| VIS-160 | Résumer la présence technique d'une session par élève | système | R-VI20 | `visio.presence_resumee` | VI3 |
| VIS-161 | Proposer l'appel au professeur depuis la présence (12) | système | R-VI21 | — | VI3 |
| VIS-162 | Voir la présence d'une session (hôte) ; exporter la présence des invités extérieurs pour leur établissement (ajout personas) | HOT | — | — | VI3 |
| VIS-163 | Voir l'historique de ses sessions (élève : ses entrées et sorties) | ELV | transparence | — | VI3 |
| VIS-164 | Marquer un élève « suit à distance » pour une séance (empêché) | PRF, CPE | sans motif ; appel « présent à distance » | — | VI3 |
### 17.5 Enregistrement
| N° | Action | Acteur · droit | Règles | Événement | Lot |
|---|---|---|---|---|---|
| VIS-170 | Autoriser l'enregistrement des cours dans l'établissement | DIR · `visio.parametrer` | S-3305 ; information des familles via 17 | — | VI8 |
| VIS-171 | Vérifier les accords avant d'enregistrer (liste des élèves sans accord) | HOT | 18 | — | VI8 |
| VIS-172 | Démarrer, arrêter un enregistrement | HOT | R-VI23 ; indicateur ; annonce | `visio.enregistrement_debute` / `_fin` | VI8 |
| VIS-173 | Choisir ce qui est capté (écran et voix de l'hôte ; salle entière avec accords) | HOT | — | — | VI8 |
| VIS-174 | Publier l'enregistrement dans l'espace du cours | HOT | lecture en ligne seulement | `espaces.element_cree` | VI8 |
| VIS-175 | Retirer un enregistrement (retrait d'accord, erreur) | HOT, DIR | effacement de l'objet | `visio.enregistrement_retire` | VI8 |
| VIS-176 | Voir un enregistrement | public du cours | R-VI24 | journal | VI8 |
### 17.6 Rendez-vous, réunions, conseils
| N° | Action | Acteur · droit | Règles | Événement | Lot |
|---|---|---|---|---|---|
| VIS-180 | Choisir la modalité visio pour un rendez-vous (19) | hôte du RDV | parmi celles qu'il accepte | — | VI1 |
| VIS-181 | Basculer un rendez-vous en visio à la dernière minute | hôte | avis aux participants | — | VI1 |
| VIS-182 | Entrer dans un rendez-vous depuis l'avis ou le portail famille | PAR | salle d'attente | — | VI1 |
| VIS-183 | Recevoir un rappel 15 min avant | participants | 8 | — | VI1 |
| VIS-184 | Inviter un interprète au rendez-vous | hôte | §10 | `visio.externe_invite` | VI5 |
| VIS-185 | Tenir une réunion parents-professeurs en visio, chaque professeur dans sa salle, file d'attente par famille | organisateur (19) | salles par professeur, minuterie | — | VI5 |
| VIS-186 | Tenir un conseil de classe à distance ou hybride (23) | président | vue partagée sans téléchargement ; présence « à distance » au PV | `visio.conseil_tenu` | VI5 |
| VIS-187 | Faire sortir les délégués pendant un point individuel du conseil | président | salle d'attente temporaire | — | VI5 |
| VIS-188 | Tenir une séance d'instance à distance (46), avec feuille de présence | président | quorum calculé par le 46 | — | VI5 |
| VIS-189 | Garder le chat d'une réunion pour le compte rendu | organisateur | annoncé à l'ouverture | — | VI5 |
### 17.7 Invités
| N° | Action | Acteur · droit | Règles | Événement | Lot |
|---|---|---|---|---|---|
| VIS-195 | Inviter un parent sans compte par lien nominatif à usage limité | hôte | 19 ; code par le 8 | `visio.lien_emis` | VI1 |
| VIS-196 | Inviter un extérieur (intervenant, partenaire) | organisateur · `visio.inviter_externe` | R-VI18 ; adresse vérifiée | `visio.externe_invite` | VI5 |
| VIS-197 | Révoquer une invitation | organisateur | — | — | VI5 |
| VIS-198 | Rejoindre en invité (page minimale, sans compte) | EXT, PAR | salle d'attente | — | VI1 |
### 17.8 Continuité pédagogique
| N° | Action | Acteur · droit | Règles | Événement | Lot |
|---|---|---|---|---|---|
| VIS-200 | Préparer le plan de l'année (reconduction de N-1, trame) | DIR · `continuite.gerer` | brouillon ; IA facultative (N1) | — | VI6 |
| VIS-201 | Valider le plan | chef d'établissement | version figée ; rappel tant que non validé | `continuite.plan_valide` | VI6 |
| VIS-202 | Désigner la cellule et le référent numérique | DIR | — | — | VI6 |
| VIS-203 | Décrire les scénarios et leurs règles (temps, priorités, contact) | DIR | trame ; repère 1/3–2/3 | — | VI6 |
| VIS-204 | Voir les contrôles de préparation (familles sans contact, élèves sans équipement, classes sans espace) | DIR, CPE | nombres puis listes selon droits | — | VI6 |
| VIS-205 | Recenser l'équipement des familles (question de rentrée facultative) | DIR | campagne du 18, réponse facultative | — | VI6 |
| VIS-206 | Organiser une séance d'essai de classe virtuelle par classe | PRF, PP | — | — | VI6 |
| VIS-207 | Publier le plan aux personnels, et un résumé aux familles | DIR | 17 | — | VI6 |
| VIS-210 | Activer un scénario sur un périmètre et des dates (aperçu des effets) | DIR · `continuite.activer` | N3 ; R-VI27 | `continuite.activee` | VI7 |
| VIS-211 | Passer en lot les séances du périmètre en `distance` ou en autonomie | ADJ (10) | proposition, puis geste au 10 | — | VI7 |
| VIS-212 | Générer les demi-groupes et l'alternance du scénario hybride | ADJ | aperçu ; IA facultative (N2) | — | VI7 |
| VIS-213 | Créer les dossiers de continuité des espaces (32) | système | ETR-284 | — | VI7 |
| VIS-214 | Prévenir les professeurs : une fenêtre « vos séances à distance » | système | 20 | — | VI7 |
| VIS-215 | Préparer et envoyer l'annonce aux familles (17) | DIR | N3 | — | VI7 |
| VIS-216 | Voir le tableau de continuité par classe | DIR, CPE, PP | R-VI29 | — | VI7 |
| VIS-217 | Marquer « contact établi » avec un élève cette semaine (moyen, sans contenu) | PP, PRF, CPE, AED | — | `continuite.contact` | VI7 |
| VIS-218 | Voir les élèves non joints de la semaine | PP puis CPE | S-3308 | — | VI7 |
| VIS-219 | Tenir la liste « à contacter en priorité » (sans motif) | CPE, DIR | R-VI28 | — | VI7 |
| VIS-220 | Voir la charge de travail par classe et par jour | équipe, PP | 20 | — | VI7 |
| VIS-221 | Suivre les prêts de matériel de continuité (44) | DIR, gestion | — | — | VI7 |
| VIS-222 | Organiser l'accueil sur place des élèves sans équipement | DIR | liste, appel du 12 | — | VI7 |
| VIS-223 | Prolonger, modifier le périmètre d'une activation | DIR | N3 | `continuite.modifiee` | VI7 |
| VIS-224 | Désactiver ; retour des séances ; annonce | DIR | N2 | `continuite.desactivee` | VI7 |
| VIS-225 | Rédiger le bilan (trame, chiffres agrégés) | DIR | versé au plan N+1 | `continuite.bilan` | VI7 |
| VIS-226 | Voir « Aujourd'hui » en mode continuité (élève, famille) | ELV, PAR | 29 : salles du jour, travaux, contacts | — | VI7 |
### 17.9 Administration et exploitation
| N° | Action | Acteur · droit | Règles | Événement | Lot |
|---|---|---|---|---|---|
| VIS-240 | Régler les options de l'établissement (enregistrement, salle d'attente, ouvertures anticipées) | DIR · `visio.parametrer` | bornes ScolIA | — | VI2 |
| VIS-241 | Voir l'usage agrégé (sessions, participants, durée) | ADM | sans contenu | — | VI12 |
| VIS-242 | Voir le journal des sessions et des enregistrements | DIR, DPO | métadonnées | — | VI12 |
| VIS-243 | Superviser la capacité et la qualité (exploitation) | EXP | `ent-obs` | — | VI0 |
| VIS-244 | Réserver de la capacité pour une activation | EXP, DIR | — | — | VI12 |
| VIS-245 | Mettre à jour la brique média après recette | EXP | version figée ; retour arrière | — | VI12 |

---

## 18. Règles transverses (R-VI1 à R-VI37) [R]

| N° | Règle |
|---|---|
| R-VI1 | Aucune salle sans contexte ; aucun lien public ni réutilisable par un tiers. |
| R-VI2 | Le jeton du serveur média est émis après le contrôle du contexte, pour une personne, une salle et 2 min de connexion, avec les droits minimaux du rôle. |
| R-VI3 | Le retrait d'un droit (exclusion, fin de contexte, sortie du public) coupe la personne côté serveur média. |
| R-VI4 | Les participants attendus sont lus à la date dans le contexte, jamais recopiés. |
| R-VI5 | L'hôte d'une séance est le professeur du cours à la date ou son suppléant (15). |
| R-VI6 | Élèves : micro et caméra coupés à l'entrée. |
| R-VI7 | La caméra n'est jamais obligatoire pour un élève ; aucune règle ne l'impose. |
| R-VI8 | Salle d'attente obligatoire pour tout extérieur et tout parent sans compte. |
| R-VI9 | Un nom affiché est le nom d'usage du cœur ; un élève ne le modifie pas. |
| R-VI10 | Une session se ferme seule 15 min après l'heure de fin avertie, ou au départ du dernier adulte. |
| R-VI11 | Pas de salle entre élèves sans adulte de l'établissement. |
| R-VI12 | Le départ du dernier adulte ferme la salle d'une séance avec élèves. |
| R-VI13 | Pas de tête-à-tête adulte–élève mineur hors contexte institutionnel tracé (séance, rendez-vous, permanence de service). |
| R-VI14 | Le chat privé d'un élève ne va qu'à l'hôte ; jamais entre élèves. |
| R-VI15 | L'hôte peut entrer dans tout sous-groupe ; les élèves le savent ; aucun tiers n'entre discrètement dans une autre salle. |
| R-VI16 | L'hôte peut éteindre une caméra ou couper un micro, jamais les activer. |
| R-VI17 | Le chat est effacé en fin de session, sauf réunion avec compte rendu annoncée. |
| R-VI18 | Un extérieur n'est jamais dans une salle avec des élèves mineurs sans adulte de l'établissement présent. |
| R-VI19 | Aucune pièce jointe dans le chat ; les documents passent par l'espace (32). |
| R-VI20 | La présence technique est résumée par élève et proposée ; elle n'écrit jamais l'appel. |
| R-VI21 | Un élève connecté caméra éteinte est présent. |
| R-VI22 | L'enregistrement est désactivé par défaut et impossible pour les rendez-vous, permanences et conseils. |
| R-VI23 | Enregistrer exige l'accord du 18 pour chaque élève capté ; sans accord, l'élève est exclu de la captation ou l'hôte renonce ; par défaut seuls l'écran et la voix de l'hôte. |
| R-VI24 | Un enregistrement est lisible en ligne par le public du cours seulement, sans téléchargement, et s'efface à la fin de la période. |
| R-VI25 | Les métadonnées de session ne servent qu'à la présence, à la sécurité et aux agrégats. |
| R-VI26 | Le plan de continuité de l'année est reconduit et validé à la rentrée ; rappel tant qu'il ne l'est pas. |
| R-VI27 | Aucune activation automatique d'un scénario ; la direction décide (N3). |
| R-VI28 | La liste « à contacter en priorité » ne porte aucun motif. |
| R-VI29 | Le tableau de continuité ne contient ni note, ni score, ni classement. |
| R-VI30 | Aucun STUN ni TURN tiers ; identifiants TURN éphémères. |
| R-VI31 | Aucune donnée personnelle dans les noms et métadonnées de salle. |
| R-VI32 | Le client tiers (JavaScript) est isolé dans un îlot, version figée par empreinte, sans accès aux données de l'ENT. |
| R-VI33 | E2EE pour les rendez-vous, permanences et appels entre personnels quand le client le permet ; l'état est affiché. |
| R-VI34 | Les salles `permanence` des boîtes C3 ont un titre neutre partout, sans IA ni enregistrement. |
| R-VI35 | L'IA n'écoute pas, ne transcrit pas sans accord, ne mesure ni attention ni émotion. |
| R-VI36 | L'IA n'admet, n'exclut, n'enregistre et ne note rien. |
| R-VI37 | ScolIA ne fait pas de télésurveillance d'examen. |

---

## 19. Cas limites (CVI-01 à CVI-50 ; CVI-51 à CVI-70 au §30)

| N° | Situation | Réponse |
|---|---|---|
| CVI-01 | Professeur absent le jour d'une classe virtuelle | séance annulée au 10/15 ; salle fermée ; élèves avertis par l'avis de la séance |
| CVI-02 | Suppléant désigné le matin | hôte à la date |
| CVI-03 | Élève d'un autre groupe qui tente d'entrer | refusé (pas du public) |
| CVI-04 | Élève qui partage le lien d'entrée à un ami | le lien mène à la page d'entrée ScolIA, qui exige la connexion de l'ami : refusé |
| CVI-05 | Élève connecté deux fois (ordinateur et téléphone) | une seule présence ; seconde connexion remplace la première (S-3311) |
| CVI-06 | Fratrie sur un seul ordinateur, deux cours simultanés | annoncé dans le plan (prêt, alternance) ; le suivi le montre (« jamais connecté » des deux côtés, contact par le PP) |
| CVI-07 | Connexion très faible | audio seul ; replis du serveur média ; travail en autonomie |
| CVI-08 | Réseau de l'établissement qui bloque l'UDP | TURN/TLS sur 443 |
| CVI-09 | Élève qui allume sa caméra avec un contenu choquant | caméra coupée par l'hôte, exclusion, signalement, fait au 13 |
| CVI-10 | Élève qui filme l'écran avec un autre appareil | non empêchable ; filigrane dissuasif ; règlement intérieur ; 13 |
| CVI-11 | Intrus (ancien élève) avec un compte encore actif | refusé : il n'est pas du public à la date |
| CVI-12 | Parent qui assiste au cours derrière l'élève | non contrôlable ; rappel au règlement dans la charte ; pas de captation |
| CVI-13 | Élève mineur qui demande un entretien individuel en visio avec un professeur | rendez-vous du 19 (tracé) ; ou présence d'un second adulte proposée ; jamais une salle improvisée |
| CVI-14 | Professeur qui veut ouvrir une salle « libre » pour du soutien | salle d'une séance déclarée (10 ou 19), jamais libre |
| CVI-15 | Sous-groupe où un élève est harcelé | l'hôte peut entrer à tout moment ; signalement ; retour en salle principale |
| CVI-16 | Parents séparés, rendez-vous commun, l'un est en retard | salle d'attente ; l'hôte admet ; présents visibles de tous |
| CVI-17 | Parent sous interdiction (4) invité par erreur | invitation refusée par le contrôle du contexte (19) |
| CVI-18 | Adresse protégée : le parent ne doit pas voir l'autre | rendez-vous séparés par défaut (19) |
| CVI-19 | Interprète qui arrive avant le parent | salle d'attente jusqu'à l'admission par l'hôte |
| CVI-20 | Conseil de classe hybride, délégués à distance | présence au PV (23) ; sortie des délégués pour les cas individuels (VIS-187) |
| CVI-21 | Conseil où un participant enregistre avec son téléphone | non empêchable ; rappel à l'ouverture ; responsabilité (charte) |
| CVI-22 | Enregistrement demandé, 2 élèves sans accord | exclus de la captation, prévenus ; ou l'hôte renonce |
| CVI-23 | Élève sans accord qui parle pendant un enregistrement « salle entière » | sa piste n'est pas captée ; il est averti à l'entrée |
| CVI-24 | Enregistrement à effacer avant la fin de période (erreur, retrait) | VIS-175 |
| CVI-25 | Retrait d'accord après un enregistrement où l'élève figure | l'hôte réexamine (VIS-175) ; par défaut l'enregistrement est masqué en attendant |
| CVI-26 | Élève majeur qui refuse l'enregistrement, parents d'accord | l'élève décide (majeur) |
| CVI-27 | Classe fermée pour une semaine (cas d'épidémie) | scénario `classe_fermee` sur cette classe seule |
| CVI-28 | Neige un matin, décision à 6 h 30 | activation depuis le téléphone (VIS-210 sur mobile, aperçu simple) ; annonce prête |
| CVI-29 | Activation alors que l'EDT de la semaine est temporaire (10) | les séances de la version en vigueur sont traitées |
| CVI-30 | Professeur sans équipement à la maison | accueil dans l'établissement ou séance en autonomie ; signalé dans les contrôles |
| CVI-31 | Élève non joint trois semaines | escalade PP → CPE → direction ; information préoccupante éventuelle relève du 36, jamais déclenchée par le logiciel |
| CVI-32 | Élève hospitalisé qui suit quelques cours | « suit à distance » par séance, sans motif (VIS-164) ; l'organisation relève du 35 / 34 |
| CVI-33 | Option rare suivie dans un autre établissement en visio (10 CL-68) | séance à public individuel ; l'hôte est le professeur de l'autre établissement : **extérieur** invité (R-VI18 : adulte de l'établissement présent ou élève majeur ; à défaut, convention, Q-VI10) |
| CVI-34 | Saturation du serveur média un matin de fermeture | capacité réservée ; priorité aux séances ; consignes : 1/3 synchrone seulement, décalage des horaires proposé au plan |
| CVI-35 | Panne du serveur média | message clair ; bascule vers le travail en autonomie (20) ; pas de service de secours externe par défaut |
| CVI-36 | Navigateur ancien sans WebRTC | page d'entrée le dit ; appli mobile proposée |
| CVI-37 | Élève malentendant | sous-titres hors V1 ; interprète LSF invité (35) ; chat |
| CVI-38 | Élève avec lecteur d'écran | interface Leptos accessible ; boutons étiquetés ; raccourcis clavier |
| CVI-39 | Même personne parent et personnel | casquette choisie à l'entrée selon le contexte |
| CVI-40 | Personnel qui appelle un collègue d'un autre établissement | impossible (D-013) ; invitation externe d'une réunion |
| CVI-41 | Appel entre personnels au milieu de la nuit | permis (personnels), aucune sonnerie hors heures pour les élèves |
| CVI-42 | Rendez-vous avec la psychologue, l'élève est mineur | permanence C3, E2EE, titre neutre, aucune trace hors métadonnées |
| CVI-43 | Famille qui demande l'enregistrement de son rendez-vous | refusé (R-VI22) ; compte rendu écrit du 19 possible |
| CVI-44 | Prise de notes de l'hôte pendant un rendez-vous | dans le 19 (suite du rendez-vous), jamais dans la salle |
| CVI-45 | Jeton volé (copié depuis le navigateur) | valable 2 min pour se connecter, lié à l'identité ; la connexion double remplace l'autre et alerte |
| CVI-46 | Attaquant qui énumère des identifiants de salle | UUID ; aucune salle accessible sans jeton |
| CVI-47 | Établissement qui veut utiliser Teams ou Zoom pour un rendez-vous | hors ScolIA ; aucun lien tiers généré ; un lien saisi dans le 19 est une information, pas une salle |
| CVI-48 | Examen oral à distance (élève à l'étranger) | séance ou rendez-vous ordinaire, sans télésurveillance ; décision pédagogique (R-VI37) |
| CVI-49 | Fin d'année : élève parti en juin, salle d'une séance de juillet (stage, rattrapage) | public à la date |
| CVI-50 | Bascule : appel lancé dans Next.js et dans ScolIA en même temps | coexistence ; aucun pont ; l'arrêt des appels Next.js est un geste de Florian |

---

## 20. Droits et minimisation

### 20.1 Catalogue (D-017) [R]
| Action | Portée | Rôles par défaut |
|---|---|---|
| `visio.participer` | contextes dont on est participant | tous |
| `visio.animer` | ses cours, ses rendez-vous, ses réunions | professeurs, CPE, organisateurs |
| `visio.appeler` | ses conversations entre personnels | personnels |
| `visio.inviter_externe` | ses réunions et rendez-vous | direction, secrétariat, organisateurs désignés |
| `visio.enregistrer` | ses cours, si l'établissement l'a permis | professeurs |
| `visio.parametrer` | établissement | direction |
| `visio.journal.lire` | établissement | direction, DPO |
| `continuite.gerer` | établissement | direction |
| `continuite.activer` | établissement | chef d'établissement, adjoint |
| `continuite.suivre` | ses classes ; établissement pour CPE et direction | PP, CPE, direction |

### 20.2 Projections
Élève : ses salles du jour, ses sessions ; famille : ses rendez-vous et réunions ; professeur : ses salles, présences de ses sessions ; direction : agrégats, journal, plan, tableau de continuité ; DPO : journal, règles ; aucun accès au contenu des sessions pour personne (il n'est pas conservé).

### 20.3 Conservation [R, DPO, Q-VI6]
| Donnée | Durée |
|---|---|
| Contenu audio, vidéo, chat | **non conservé** (sauf enregistrement et chat de réunion annoncés) |
| Enregistrement de cours | fin de la période (trimestre ou semestre) |
| Chat de réunion conservé | avec le compte rendu (19) |
| Métadonnées de session (entrées, sorties) | 1 an (sécurité) [NV] ; résumé de présence : transmis au 12, qui conserve selon ses règles |
| Plan de continuité et bilans | 5 ans [NV] |
| Contacts hebdomadaires | fin de l'année scolaire suivante |

### 20.4 Pièges à tester
Entrée d'un élève hors public ; jeton rejoué ; extérieur dans un cours sans adulte ; sous-groupe sans accès de l'hôte ; enregistrement d'un élève sans accord ; rendez-vous enregistré ; liste prioritaire qui laisse voir un motif ; STUN tiers dans la configuration ; nom de salle contenant un nom ; client JS qui lit un cookie ; caméra allumée à distance par l'hôte (doit être impossible).

---

## 21. Événements [R]
Émis : `visio.salle_creee`, `.session_ouverte`, `.session_fermee`, `.entree`, `.admis`, `.refuse`, `.exclu`, `.signalement`, `.sous_groupes`, `.presence_resumee`, `.enregistrement_debute`, `.enregistrement_fin`, `.enregistrement_retire`, `.appel_lance`, `.lien_emis`, `.externe_invite`, `.conseil_tenu` ; `continuite.plan_valide`, `.activee`, `.modifiee`, `.desactivee`, `.contact`, `.bilan`. Charges utiles : identifiants seulement.
Consommés : `edt.seance_*` (10), `agenda.rdv_*` (19), `conseils.*` (23), `instances.*` (46), `absences_personnels.*` (15), `structure.fermeture_*` (5), `autorisations.retiree` (18), `messagerie.fil_*` (16).

## 22. Notifications (natures au socle 8)
`visio.appel_entrant` (U3, temps réel, personnels) ; `visio.rappel_rdv` (U2, 15 min avant) ; `visio.seance_a_distance` (U2, inclus dans l'avis de séance du 10, pas un second avis) ; `visio.invitation_externe` (e-mail avec code) ; `continuite.activee` (U3 aux personnels, annonce des familles par le 17) ; `continuite.non_joints` (U1, PP le jeudi) ; `continuite.plan_a_valider` (U1, direction).

## 23. Écrans
1. **Page d'entrée** : vignette vidéo, choix des appareils, caméra éteinte, flou, nom affiché, état E2EE, « Entrer ».
2. **Salle** : mosaïque ou orateur, bande de participants, file des mains, chat repliable, barre d'outils (micro, caméra, écran, mains, réactions, sous-groupes, tableau, sondage, enregistrement), indicateur d'enregistrement permanent en rouge, panneau de salle d'attente pour l'hôte.
3. **Sous-groupes** : grille des groupes avec minuterie ; glisser-déposer des élèves.
4. **Tableau de continuité** (Bento) : classes × semaine (contactés / à joindre), charge de travail, prêts, présence agrégée.
5. **Plan de continuité** : document structuré, scénarios, contrôles de préparation, historique des versions.
6. **Mobile** (D-003) : rejoindre depuis « Aujourd'hui », appel entrant (CallKit / ConnectionService pour les personnels [NV]), rendez-vous en visio pour les familles.

---

## 24. Modèle de données cible (brique `visio`) [R]

E1–E14 ; aucune colonne JSON, aucun tableau, aucun `BYTEA`.

```sql
ent_visio_salle (id, etablissement_id, contexte CHECK (seance|rdv|reunion|conseil|instance|fil|permanence|continuite),
                 seance_id NULL, rdv_id NULL, reunion_id NULL, conseil_id NULL, instance_id NULL, fil_id NULL, boite_id NULL, activation_id NULL,
                 cle_media UUID UNIQUE,            -- nom de salle opaque côté serveur média
                 salle_attente BOOL, e2ee BOOL, chat_ouvert BOOL, chat_conserve BOOL, enregistrement_permis BOOL,
                 etat CHECK (prete|ouverte|fermee), CHECK (exactement une référence de contexte non nulle))
ent_visio_session (id, etablissement_id, salle_id, ouverte_le, fermee_le NULL, hote_personne_id, motif_fermeture_code NULL)
ent_visio_participation (id, etablissement_id, session_id, personne_id NULL, invite_id NULL, role CHECK (hote|cohote|intervenant|participant|observateur),
                         entree_le, sortie_le NULL, deconnexions INT, admis_par NULL, exclu_par NULL, exclu_motif_code NULL,
                         CHECK (personne_id IS NOT NULL OR invite_id IS NOT NULL))
ent_visio_presence_resume (session_id, etablissement_id, eleve_id, statut CHECK (present|retard|sorti_tot|jamais_connecte), minutes INT, transmis_le NULL)
ent_visio_sous_groupe (id, etablissement_id, session_id, rang, cle_media UUID, debut, fin_prevue, fin NULL)
ent_visio_sous_groupe_membre (sous_groupe_id, etablissement_id, personne_id)
ent_visio_invite (id, etablissement_id, salle_id, nom_affiche, adresse_empreinte, empreinte_jeton, role, expire_le, usages_max, usages, revoque_le NULL, invite_par)
ent_visio_chat_reunion (id, etablissement_id, session_id, auteur_personne_id NULL, invite_id NULL, texte, le)   -- seulement si chat_conserve
ent_visio_enregistrement (id, etablissement_id, session_id, debut, fin NULL, captation CHECK (ecran_voix_hote|salle_accords),
                          objet_id NULL, element_espace_id NULL, retire_le NULL, retire_motif_code NULL, efface_le NULL)
ent_visio_enregistrement_exclusion (enregistrement_id, etablissement_id, eleve_id, raison CHECK (sans_accord|refus_majeur|retrait))
ent_visio_signalement (id, etablissement_id, session_id, auteur_personne_id, motif_code, le, traite_par NULL)
ent_visio_reglage (etablissement_id PRIMARY KEY, enregistrement_cours_permis BOOL, ouverture_anticipee_min SMALLINT, fermeture_tolerance_min SMALLINT,
                   retard_seuil_min SMALLINT, filigrane BOOL)
ent_visio_journal (id, etablissement_id, salle_id NULL, session_id NULL, acteur_personne_id NULL, action_code, le)   -- ajout seul

-- continuité
ent_continuite_plan (id, etablissement_id, annee_id, version INT, etat CHECK (brouillon|valide|remplace), valide_par NULL, valide_le NULL, reconduit_de NULL)
ent_continuite_cellule (plan_id, etablissement_id, personne_id, fonction CHECK (chef|adjoint|cpe|referent_numerique|gestion|autre))
ent_continuite_scenario (id, etablissement_id, plan_id, type CHECK (hybride_demi_groupes|hybride_niveaux|distance_totale|classe_fermee),
                         part_synchrone_pct SMALLINT, contact_hebdo_min SMALLINT, priorites_texte TEXT NULL, accueil_texte TEXT NULL)
ent_continuite_scenario_niveau (scenario_id, etablissement_id, niveau_id, part_synchrone_pct SMALLINT NULL)
ent_continuite_section (id, etablissement_id, plan_id, rang, code CHECK (outils|communication|materiel|evaluation|bilan|autre), texte)
ent_continuite_activation (id, etablissement_id, plan_id, scenario_id, debut DATE, fin DATE NULL, decide_par, decide_le, motif_code CHECK (intemperies|epidemie|travaux|securite|autre))
ent_continuite_activation_perimetre (activation_id, etablissement_id, type CHECK (etablissement|niveau|classe), niveau_id NULL, classe_id NULL)
ent_continuite_demi_groupe (id, etablissement_id, activation_id, classe_id, code CHECK (A|B))
ent_continuite_demi_groupe_eleve (demi_groupe_id, etablissement_id, eleve_id)
ent_continuite_contact (id, etablissement_id, activation_id, eleve_id, semaine DATE, par_personne_id, moyen CHECK (visio|message|telephone|visite|sur_place|autre), le)
ent_continuite_prioritaire (id, etablissement_id, activation_id NULL, eleve_id, inscrit_par, debut, fin NULL)   -- sans motif (R-VI28)
ent_continuite_bilan (activation_id, etablissement_id, redige_par, texte, nb_eleves_non_joints INT, materiel_non_rendu INT)
```

**Registre (E12)** : `ent_visio_participation`, `_presence_resume`, `_journal` : `etablissement`, export `simple` ; `ent_visio_salle`, `_session`, `_sous_groupe*` : `etablissement` ; état du serveur média : **volatile**, non sauvegardé ; `ent_visio_invite` : jetons à **neutraliser à la restauration** (E9) ; enregistrements : objets `ent_objet` de l'espace (32). Clé API du serveur média : inventaire des clés (E11).

---

## 25. Pilotage (47)
Sessions par semaine, part des séances à distance tenues en classe virtuelle, durée moyenne, rendez-vous en visio (%), taux de contact hebdomadaire pendant une continuité, incidents ; jamais par professeur nommé.

## 26. Reprise et bascule (appels Next.js)
Rien à reprendre (aucun historique). Coexistence : les appels Next.js continuent tant que Florian ne les arrête pas ; ScolIA propose ses appels aux personnels dès VI1 ; **ND33-01 (STUN Google)** est signalé pour traitement par Florian dès maintenant (correction dans Next.js ou inscription au registre), sans attendre la bascule (Q-VI4).

---

## 27. Lots de code et migrations

### 27.1 Lots
| Lot | Contenu | Dépend de | Critères de fin |
|---|---|---|---|
| **VI0** preuve technique | serveur média choisi (Q-VI1) auto-hébergé sur une instance de labo, TURN intégré, jetons émis par axum (`livekit-api`), îlot client (Q-VI2), essai de charge (30 salles × 30 participants), test depuis un réseau filtré (UDP bloqué), aucun STUN tiers ; port `visio` (trait) et adaptateur | décision de Florian | rapport de charge et de qualité ; coût mensuel estimé |
| **VI1** appels et rendez-vous | salles `fil` et `rdv`, page d'entrée, salle d'attente, invités sans compte, rappels, parité NX33-01 à NX33-08 | VI0 ; 16, 19, 8 | ND33-01 à ND33-07 impossibles (tests) |
| **VI2** classe virtuelle | salle de séance, rôles, modération, partage d'écran, chat, mains, réglages | VI1 ; 10 | CVI-01 à CVI-12 |
| **VI3** présence | résumé, proposition au 12, « suit à distance » | VI2 ; 12 | R-VI20, R-VI21 |
| **VI4** outils pédagogiques | sous-groupes, tableau, sondage, support de l'espace | VI2 ; 32 | CVI-15 |
| **VI5** réunions, conseils, instances, extérieurs | réunions parents-professeurs, conseils hybrides, instances, interprètes, chat conservé | VI1 ; 19, 23, 46 | CVI-16 à CVI-21 |
| **VI6** plan de continuité | plan, cellule, scénarios, contrôles de préparation, essais, publication | 4, 5, 18, 32 | R-VI26 |
| **VI7** activation et suivi | activation, séances en lot, demi-groupes, annonce, tableau, contacts, prioritaires, prêts, bilan | VI6 ; 10, 17, 20, 44 | CVI-27 à CVI-31 ; R-VI27 à R-VI29 |
| **VI8** enregistrement | Egress isolé, accords du 18, exclusions, publication dans l'espace, effacement | VI2 ; 18, 32 | CVI-22 à CVI-26 |
| **VI9** mobile | rejoindre, appel entrant, rendez-vous famille | VI1 ; 11 | — |
| **VI10** accessibilité | audit RGAA de l'interface, raccourcis, interprète | VI2 | — |
| **VI11** IA et sous-titres | capacités §16.3 ; étude d'une transcription locale (Q-VI9) | VI7 ; `ent-ia` | aucun contexte C3 transmis (test) |
| **VI12** exploitation et conservation | supervision, capacité réservée, mises à jour, journal, effacements | VI2 | §20.3 appliqué |
| **VI13** bascule des appels Next.js | communication, coexistence, arrêt par Florian | VI1 | — |

**Parcours minimal V1 (février 2027)** : VI0, VI1, VI2, VI3, VI6 ; puis VI7 avant l'hiver 2027-2028 (l'année de doublon peut connaître une fermeture).

### 27.2 Migrations (plage imposée 2660–2679)
| N° | Lot | Contenu |
|---|---|---|
| 2660 | VI1 | salles, sessions, participations, invités, journal, réglages |
| 2661 | VI2 | signalements |
| 2662 | VI3 | résumés de présence |
| 2663 | VI4 | sous-groupes |
| 2664 | VI5 | chat de réunion conservé |
| 2665 | VI6 | plans, cellule, scénarios, sections |
| 2666 | VI7 | activations, périmètres, demi-groupes, contacts, prioritaires, bilans |
| 2667 | VI8 | enregistrements, exclusions |
| 2668–2672 | réserve | ajouts personas (§30) : 2668 dispositifs « élève à distance » durables, 2669 essais et préparation, 2670 capacité réservée, 2671–2672 réserve |
| 2673–2677 | — | réserve |
| 2678 | VI12 | RLS stricte et validation des FK composites |
| 2679 | — | retraits (réservé) |

Exceptions D-036 : **aucune**.

---

## 28. Décisions appliquées par défaut (D-031) et questions à Florian

### 28.1 Appliquées par défaut (D-031)
1. Salles toujours issues d'un contexte, sans lien public (R-VI1).
2. Caméra jamais obligatoire ; micro et caméra coupés à l'entrée pour les élèves (R-VI6, R-VI7).
3. Enregistrement désactivé par défaut, interdit pour rendez-vous, permanences et conseils (R-VI22).
4. Pas de salle entre élèves sans adulte ; hôte présent dans les sous-groupes ; pas de tête-à-tête adulte–mineur hors cadre (R-VI11 à R-VI15).
5. Présence proposée à l'appel, jamais écrite (R-VI20).
6. Plan de continuité comme objet métier reconduit et validé chaque rentrée ; activation humaine (R-VI26, R-VI27).
7. Liste prioritaire sans motif ; tableau de continuité sans score (R-VI28, R-VI29).
8. Aucun STUN/TURN tiers (R-VI30).
9. Pas de télésurveillance d'examen (R-VI37).
10. Port `visio` indépendant de la brique (§11.3) ; conception compatible avec les briques comparées.
11. Pas de passerelle téléphonique en V1.

### 28.2 Soumis à Florian
| N° | Question | Recommandation |
|---|---|---|
| Q-VI1 | **Brique média** | LiveKit auto-hébergé (Apache-2.0), TURN intégré, sans LiveKit Cloud ; repli Galène ; décision après la preuve VI0 |
| Q-VI2 | **Client web** : SDK JavaScript officiel dans un îlot isolé (exception à D-002), ou client Rust/WASM maison | Îlot JavaScript figé et isolé pour la V1 ; client Rust étudié ensuite |
| Q-VI3 | **Coût** : instance média dédiée et bande passante sortante | Chiffrage après l'essai de charge VI0 |
| Q-VI4 | **STUN Google dans Next.js** (IP des personnels transmise à Google) | À traiter par vous dans Next.js (STUN auto-hébergé ou désactivation des appels) ou à inscrire au registre ; le pilote ne touche pas Next.js |
| Q-VI5 | Raccordement à la classe virtuelle nationale (apps.education, BBB) pour les établissements publics | Option plus tard, sur demande de l'établissement ; non pertinent pour le privé |
| Q-VI6 | Enregistrement des cours : base légale, durées | DPO ; ScolIA livre désactivé |
| Q-VI7 | Rejoindre par téléphone (opérateur SIP) | Non en V1 (coût, contrat) |
| Q-VI8 | Filigrane nominatif sur la vidéo vue par les élèves | Oui, léger, désactivable |
| Q-VI9 | Sous-titres et transcription locale | Étude après V1 ; jamais un service externe |
| Q-VI10 | Cours suivi dans un autre établissement (convention) | Conventions entre établissements, hors logiciel |

---

## 29. Faits, hypothèses, contradictions
- **Faits** : aucun code Rust ; appels Next.js inventoriés (8 actions, 8 défauts dont STUN Google) ; briques vérifiées le 2026-11-11 (licences et versions §11.2) ; guide de continuité du 2/10/2026 lu ; position CNIL sur la caméra ; Visio (DINUM) réservée aux agents ; privé sous contrat non concerné par la Doctrine technique.
- **Hypothèses** : charges annoncées par les éditeurs ; licences des SDK mobiles LiveKit ; licence de Jitsi Videobridge ; disponibilité d'E2EE dans tous les navigateurs ciblés.
- **Contradictions** : (1) D-002 (« tout en Rust, frontend compris ») contre l'absence de client navigateur Rust pour un SFU éprouvé (Q-VI2) ; (2) `19_AGENDA_RDV.md` §2 écrit « appels WebRTC dans la messagerie, aucun service externe » : exact pour la cible, mais l'existant Next.js utilise un STUN externe (ND33-01) ; (3) `10_EMPLOI_DU_TEMPS.md` prévoit un « lien visio » dans la séance `distance` : ici, ce n'est pas un lien mais une **salle** issue de la séance (aucune URL tierce).

---

## 30. Ajouts personas (`PERSONAS_33.md`)

### 30.1 Bloquants tranchés (appliqués par défaut, D-031)
- **A. L'élève durablement à distance** (M-3301) : un élève hospitalisé ou empêché plusieurs semaines n'est pas un cas « par séance » : dispositif **« suit à distance »** daté (début, fin, cours concernés) posé par le CPE ou la direction **sans motif**, qui ouvre automatiquement la salle des séances concernées et prévient les professeurs ; l'organisation pédagogique (et les données de santé) reste au 34 / 35 (R-VI38).
- **B. Le matin de la fermeture** (M-3306) : l'activation doit tenir sur un téléphone avant 7 h ; **activation en deux temps** : (1) « Fermeture aujourd'hui » (annonce aux familles et aux personnels, séances du jour en autonomie par défaut) en un geste ; (2) organisation fine (classes virtuelles, demi-groupes) dans la journée (R-VI39).
- **C. Attaquant qui se fait passer pour un parent invité** (M-3315) : le lien nominatif sans compte exige un **code** reçu sur le moyen de contact enregistré du parent (4), et l'hôte voit « invité sans compte, vérifié par code envoyé au … (masqué) » ; aucune invitation vers une adresse saisie à la main sans vérification (R-VI40).
- **D. Auditeur RGPD : métadonnées de connexion des élèves** (M-3320) : le résumé de présence est la seule donnée de connexion conservée au-delà de la session pour un élève ; les entrées et sorties détaillées sont effacées **30 jours** après la session (et non 1 an) (R-VI41) ; le journal de sécurité ne garde que les événements anormaux (exclusion, signalement, double connexion).

### 30.2 Actions ajoutées (VIS-246 à VIS-275)
| N° | Action | Acteur · droit | Règles | Lot |
|---|---|---|---|---|
| VIS-246 | Poser un dispositif « suit à distance » daté pour un élève (cours, dates), sans motif | CPE, DIR | R-VI38 | VI3 |
| VIS-247 | Lever ou prolonger le dispositif | CPE, DIR | — | VI3 |
| VIS-248 | Voir ses élèves « à distance » de la semaine (professeur) | PRF | sans motif | VI3 |
| VIS-249 | Déclarer « Fermeture aujourd'hui » en un geste depuis le téléphone | chef d'établissement | R-VI39 ; N3 simplifié | VI7 |
| VIS-250 | Compléter l'organisation de la fermeture dans la journée | DIR, ADJ | VIS-210 à VIS-215 | VI7 |
| VIS-251 | Voir l'état de préparation du plan sur le tableau de bord de rentrée (29) | DIR | — | VI6 |
| VIS-252 | Faire tester son matériel à l'avance (salle d'essai personnelle) | tous | salle `essai`, sans autre participant | VI2 |
| VIS-253 | Demander de l'aide technique depuis la salle (boîte « référent numérique », 16) | tous | — | VI2 |
| VIS-254 | Permuter vidéo et audio seul automatiquement selon la qualité | tous | — | VI2 |
| VIS-255 | Voir la qualité de sa connexion | tous | — | VI2 |
| VIS-256 | Afficher le support partagé en grand pour un élève malvoyant (agrandissement local) | ELV | — | VI10 |
| VIS-257 | Épingler la vidéo de l'interprète LSF | tous | — | VI10 |
| VIS-258 | Programmer une réunion parents-professeurs en visio avec créneaux par professeur (19) | organisateur | salles créées par professeur | VI5 |
| VIS-259 | Faire patienter une famille et lui indiquer le temps d'attente estimé | hôte | file du 19 | VI5 |
| VIS-260 | Passer de la salle d'un professeur à celle du suivant (parcours famille) | PAR | — | VI5 |
| VIS-261 | Transférer un participant d'une salle à une autre (ex. vers la salle du CPE) | HOT | même établissement, contexte valide pour la cible | VI5 |
| VIS-262 | Ouvrir une permanence planifiée (heures fixes) d'une boîte de service | membre de la boîte | file d'attente | VI5 |
| VIS-263 | Inviter un AESH comme co-participant d'un élève accompagné | PRF, CPE | attestation 35 | VI2 |
| VIS-264 | Vérifier le code d'un invité sans compte, voir sa méthode de vérification | HOT | R-VI40 | VI1 |
| VIS-265 | Signaler une double connexion suspecte | système | alerte à l'hôte et à la personne | VI2 |
| VIS-266 | Voir et exporter ses données de connexion (droit d'accès) | tous ; DPO pour une demande | 27 | VI12 |
| VIS-267 | Purger les entrées et sorties détaillées à 30 jours | système | R-VI41 | VI12 |
| VIS-268 | Voir la capacité disponible avant d'activer un scénario | DIR | — | VI7 |
| VIS-269 | Décaler les horaires des classes virtuelles pour lisser la charge | ADJ | proposition | VI7 |
| VIS-270 | Organiser la journée d'accueil des élèves sans équipement (liste, salle, encadrant) | DIR, CPE | 12 | VI7 |
| VIS-271 | Signaler qu'on n'a pas d'équipement pendant une continuité | PAR, ELV | ajout à la liste des prêts (44) | VI7 |
| VIS-272 | Recevoir le récapitulatif hebdomadaire de continuité de son enfant (salles, travaux) | PAR | 29 | VI7 |
| VIS-273 | Visio du professeur depuis la salle de classe vers un élève à distance (cours hybride) | PRF | caméra de salle : élèves présents informés, pas d'enregistrement | VI2 |
| VIS-274 | Clore une salle de séance si aucun élève n'est entré 15 min après le début | système | avertit le professeur | VI2 |
| VIS-275 | Voir la charte visio (règles affichées à la première entrée) | tous | accusé de lecture une fois par an | VI2 |

### 30.3 Règles ajoutées
| N° | Règle |
|---|---|
| R-VI38 | Le dispositif « suit à distance » est daté, porte sur des cours, n'a aucun motif dans le 33. |
| R-VI39 | La fermeture du jour s'active en un geste ; l'organisation fine suit. |
| R-VI40 | Un invité sans compte est vérifié par un code envoyé au moyen de contact connu du cœur ; jamais vers une adresse saisie sans vérification. |
| R-VI41 | Les entrées et sorties détaillées d'un élève s'effacent 30 jours après la session ; seul le résumé de présence passe au 12. |
| R-VI42 | En cours hybride, la caméra de salle ne filme que le tableau et le professeur par défaut ; les élèves présents en sont informés. |

### 30.4 Cas limites ajoutés (CVI-51 à CVI-70)
| N° | Situation | Réponse |
|---|---|---|
| CVI-51 | Élève à distance trois mois pour maladie, les professeurs ne savent pas pourquoi | dispositif sans motif ; le CPE informe selon le 34 / 35 |
| CVI-52 | Chef d'établissement absent le matin de la neige | délégation (6) : l'adjoint active |
| CVI-53 | Fermeture annoncée la veille au soir | « Fermeture demain » : même geste, date choisie |
| CVI-54 | Plan non validé le jour d'une fermeture | activation possible avec la trame par défaut ; rappel de validation |
| CVI-55 | Professeur qui ne sait pas animer une classe virtuelle | salle d'essai, aide, séance en autonomie |
| CVI-56 | Famille sans ordinateur, un téléphone partagé | appli mobile ; audio seul ; prêt |
| CVI-57 | Famille allophone au rendez-vous | interprète (VIS-184) ; annonce du 17 traduite |
| CVI-58 | Parent qui fait entrer un tiers non invité devant son écran | non contrôlable ; rappel de la charte |
| CVI-59 | Attaquant avec un code volé sur le téléphone d'un parent | code à usage unique, lié au rendez-vous ; l'hôte admet visuellement ; révocation |
| CVI-60 | Attaquant qui tente d'injecter du contenu dans le chat (lien piégé) | liens affichés en texte, cliquables seulement vers notre origine et les éléments de l'espace |
| CVI-61 | Attaquant qui sature une salle d'attente (comptes compromis multiples) | limite par salle ; verrouillage ; alerte |
| CVI-62 | Déni de service sur le TURN | limites du serveur média ; supervision |
| CVI-63 | Professeur qui laisse sa session ouverte après le cours | fermeture à +15 min (R-VI10) |
| CVI-64 | Session d'un conseil qui déborde | prolongation par le président |
| CVI-65 | Élève qui suit à distance un cours hybride et ne voit pas le tableau | support de l'espace partagé (VIS-145) |
| CVI-66 | Demande de la famille de voir la présence de son enfant en visio | l'appel du 12 (absences) ; pas les métadonnées techniques |
| CVI-67 | DPO qui demande la liste des sous-traitants de la visio | aucun hors hébergeur (Scaleway) ; registre (27) |
| CVI-68 | Mise à jour du serveur média pendant une fermeture | gel des mises à jour pendant une activation |
| CVI-69 | Deux activations qui se chevauchent (classe fermée puis établissement) | la plus large l'emporte ; périmètres fusionnés |
| CVI-70 | Bilan demandé par l'autorité académique (public) | export du bilan agrégé |

### 30.5 Lots révisés
- **VI3** inclut le dispositif « suit à distance » (VIS-246 à VIS-248), migration 2668.
- **VI7** inclut la fermeture en un geste (VIS-249, VIS-250) avant le reste ; ajout de la capacité (2670).
- **VI12** inclut la purge à 30 jours (R-VI41) et le droit d'accès (VIS-266).
