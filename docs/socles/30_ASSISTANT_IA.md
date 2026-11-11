# Socle 30 — Assistant IA et « chat first »

**Date :** 2026-11-11 — **Statut :** conception de fond, **non décidée** (à relire par Florian) ; les recommandations s'appliquent par défaut (D-031) sauf ce qui engage l'établissement à l'extérieur (§30.2). Aucun code écrit ni modifié. Aucun accès à une base, à un secret ou à un environnement de production. Le code Next.js a été **lu seulement** (lecture ciblée de `app/lib/brain-ai/`, `app/api/chatbot/`, `app/api/bien-etre/`), sans rien exécuter ni modifier.
**Branche :** `ent-socle-30-31-conception`, partie de `origin/ent-mc0hoe`.
**Passe personas :** `docs/socles/PERSONAS_30.md` ; ses ajouts sont faits **ici**, au §32, marqués « ajout personas ». En cas de contradiction, le §32 l'emporte.
**À lire avec :** `CLAUDE.md` (« Sécurité, données et IA ») ; `docs/socles/IA_COEUR.md` (**architecture de cœur : ce document ne la refait pas, il la complète et la corrige sur quatre points, §6.4**) ; `docs/AI_PLATFORM.md` ; `docs/DECISIONS.md` D-017, D-023, D-024, D-025, D-026, D-031, D-032 (Q-EV8 : appréciations), D-034, D-035, D-036 ; `docs/SOCLES.md` §1 bis et §30 ; `docs/PRA_SAUVEGARDES.md` §18.4 (E1–E14) ; `docs/TRANSITION_T0_INVENTAIRE.md` (P05 « Assistant IA, OCR ») ; conceptions `16_MESSAGERIE.md` (cartes d'action, §16.3), `29_PORTAILS.md` (palette, à-faire, §5.1 cadre de référence du numérique, §17), `08_NOTIFICATIONS.md` (traductions proposées NTF-48, NTF-130), `18_FORMULAIRES.md` (consentement `outils_ia_eleve`), `21_EVALUATIONS.md` et `23_BULLETINS.md` (appréciations), `34_SANTE.md` §13.3 (refus explicites), `31_AUTOMATISATIONS.md` (file de validation commune).

**Conventions.** **[F]** fait vérifié dans le dépôt (chemin cité ; Rust depuis `ent-rust/`, Next.js depuis la racine, lecture seule) ; **[V]** règle lue sur une source officielle (URL et date au §5) ; **[S]** source secondaire ; **[NV]** non vérifié ; **[H]** hypothèse ; **[R]** recommandation non décidée.

**Numérotation propre au socle** (vérifiée libre sur `origin/ent-mc0hoe` le 2026-11-11) :

| Élément | Numéros |
|---|---|
| Existant Next.js repris | **NXB-01…** |
| Défauts de l'existant (Rust et Next.js) | **DI-01…** |
| Actions cibles | **AST-100…** |
| Règles | **R-CH1…** |
| Cas limites | **KC-01…** |
| Questions | **Q-CH1…** |
| Réglages signalés (D-023) | **S-3001…** |
| Manques de la passe personas | **M-3001…** (dans `PERSONAS_30.md`) |
| Lots | **CH0…** |
| Migrations | **2600–2619** (plage imposée par le pilote ; §27) |

Les identifiants du cœur (`IA-P1`…`IA-P14`, `IA-L0`…`IA-L10`, `Q-IA1`…`Q-IA14`, classes `C0`–`C4`, niveaux `N0`–`N3`) sont repris tels quels.

**Sommaire :** §0 En bref · §1 Thèse · §2 Existant · §3 Acteurs et besoins · §4 Une journée vue du terrain · §5 Réglementaire · §6 Frontières, branchements, corrections du cœur · §7 Vocabulaire · §8 La conversation · §9 Comprendre une demande · §10 Répondre : sources, refus, explications · §11 Agir : cartes, brouillons, enchaînements · §12 Rédiger, traduire, résumer · §13 Les surfaces · §14 Usages par profil · §15 Familles et élèves · §16 Mobile et voix · §17 Réglages de l'établissement · §18 Qualité et évaluation · §19 Coûts et quotas · §20 Sécurité · §21 Droits et minimisation · §22 Cas limites · §23 Actions métier · §24 Règles transverses · §25 Événements et notifications · §26 Modèle de données · §27 Migrations · §28 Lots · §29 Tests et recette · §30 Décisions · §31 Faits, hypothèses, contradictions · §32 Ajouts personas.

---

## 0. En bref

1. **Le socle 30 n'invente pas un deuxième système.** Le cœur IA (`IA_COEUR.md`) définit déjà ce que l'IA a le droit de faire : rien en propre, tout par les capacités déclarées des socles, sous les droits effectifs de la personne, avec brouillon, aperçu rédigé par le socle et confirmation hors modèle. Le socle 30 y ajoute **la conversation** (un fil personnel, court, privé), **la compréhension des demandes**, **les enchaînements** (un plan visible, une commande par brouillon), **les surfaces** (panneau partout, barre d'accueil, fiche élève, fil de messagerie, conseil de classe, mobile), **l'ouverture encadrée aux familles et aux élèves**, et les **réglages, quotas et évaluations** qui rendent tout cela exploitable.
2. **Aucune ligne de code du cœur IA n'existe** [F] : ni crate `ent-ia`, ni table `ent_ia_*`, ni action `ia.*` ; et **aucune plage de migrations ne lui a été réservée**. Le socle 30 **héberge donc les tables du cœur** dans sa plage (2600–2605) et en fait son premier lot, avant toute conversation (R-CH1). Sans ce choix, chaque socle qui déclare des capacités (13, 14, 15, 16, 17, 19, 22, 24, 25, 34, 42 l'ont déjà fait sur le papier) attendrait un registre que personne ne construit.
3. **« Chat first » ne veut pas dire « chat only ».** Chaque réponse renvoie à l'écran qui fait foi ; chaque action passe par l'écran normal ou par une carte d'aperçu ; tout fonctionne sans IA (IA-P13). Le chat est le **chemin le plus court**, jamais le seul.
4. **Le déterministe d'abord.** Une demande est d'abord reconnue sans modèle (intentions du copilote, alias de la palette du 29, noms résolus dans la portée) ; le modèle n'intervient que pour comprendre une phrase que les règles ne savent pas lire, rédiger ou résumer. Effet : coût divisé, latence faible, et l'assistant reste utile **avec le fournisseur coupé** (« fournisseur nul »).
5. **La conversation est privée et courte.** Elle n'appartient qu'à son auteur ; ni la direction ni l'administrateur ne la lisent hors incident motivé et tracé (R-CH12) ; son contenu est scellé et effacé à **30 jours** (Q-IA9, à valider DPO) ; **aucune mémoire de faits** sur les élèves d'une conversation à l'autre (R-CH9) : l'assistant relit les services à chaque question. Seules des **préférences typées** (langue, ton, longueur) sont gardées.
6. **Toujours ses sources, toujours ce qu'il n'a pas lu.** Chaque réponse cite les objets consultés (lien vers l'écran) et dit ce qui a été **masqué** ou **non consulté** (« la santé n'est pas accessible à l'assistant ») : jamais « aucune allergie » quand la santé est fermée (T11 du cœur).
7. **Agir = une carte.** « Fais l'appel de la 4B comme lundi sauf Léa » produit une **carte d'aperçu** rédigée par le socle 12 (élèves, statuts, avant / après), décochable, confirmée d'un geste ; un enchaînement produit un **plan visible** puis une carte par étape, la suivante préparée après l'exécution de la précédente (R-CH17).
8. **Familles et élèves : fermé par défaut, ouvert par établissement, après conditions.** Ouverture possible seulement si l'AIPD est faite, la notice publiée, l'information adaptée à l'âge diffusée et un fournisseur contractualisé actif ; tâches limitées (comprendre, se repérer, préparer un message qu'on envoie soi-même, traduire) ; **jamais d'aide à faire le travail scolaire à la place de l'élève** dans le socle ; collégiens exclus en V1 ; parents séparés strictement séparés (R-CH26 à R-CH32). Dans le **public**, le cadre de référence du numérique (arrêté du 7 septembre 2026) conditionne l'IA générative ; le défaut est « fermé » tant que la lecture juridique n'est pas faite (§5).
9. **Jamais de données de santé** (consigne de Florian, CLAUDE.md) : la classe C3 reste **fermée sans exception** dans le socle 30, y compris pour l'infirmerie, tant que le DPO, un hébergeur qualifié et Florian n'en ont pas décidé autrement (Q-IA5, R-CH6). Les pastilles déclarées C1 par leur socle (« PAI : oui ») restent permises.
10. **Lots** : CH0 correctifs urgents du copilote existant (aucune migration) ; CH1–CH4 le **cœur** sans modèle (registre, journal, brouillons, fournisseur nul et simulé, copilote rebranché) ; CH5 panneau et conversation ; CH6 compréhension et enchaînements ; CH7 rédaction et traduction ; CH8 fournisseurs (après la revue contractuelle) ; CH9 mobile et dictée ; CH10 évaluations ; CH11 quotas et pilotage ; CH12 ouverture aux familles ; CH13 élèves de lycée ; CH14 recette. **163 actions** (AST-100 à AST-262) au §23, **+ 24 ajoutées par les personas** au §32.

---

## 1. Thèse : une conversation est un chemin court vers des écrans qui font foi

1. **L'assistant n'est pas une source.** Il n'a pas de données à lui. Ce qu'il dit vient d'une capacité d'un socle, servie sous les droits de la personne au moment de la question ; ce qu'il fait est une commande d'un socle, confirmée par la personne. S'il se trompe, l'écran qui fait foi le montre, à un clic.
2. **On demande comme on parle à un collègue compétent.** « Qui n'a pas rendu le devoir de jeudi en 3e B ? », « Prépare la réponse à Mme Martin, poliment, je refuse le rendez-vous de mardi mais je propose jeudi », « Qu'est-ce qui a changé pour Léa depuis le conseil ? ». La valeur est dans le **raccourci** (plus de filtres à empiler, plus de dix écrans à traverser), pas dans la conversation pour elle-même.
3. **Le bon geste au bon endroit.** L'assistant est **contextuel** : ouvert sur la fiche de Léa, « résume » veut dire « résume Léa » ; ouvert dans un fil, « réponds » veut dire « prépare une réponse à ce fil, à ses participants ». Le contexte vient de l'écran (identifiants connus du serveur), jamais d'un nom deviné.
4. **Il propose, la personne décide.** Toute écriture est une carte ; tout envoi est un aperçu complet avec ré-authentification ; aucune décision sur un élève (note, sanction, orientation, admission, bourse) n'est préparée comme décision (IA-P12).
5. **Il dit ce qu'il ne sait pas.** Refus clair (« vous n'avez pas accès aux notes de ce groupe »), masquage expliqué, incertitude avouée, aucune invention de chiffre : un chiffre vient toujours d'une capacité.
6. **Il ne garde rien qu'on ne lui ait demandé.** Pas de profil de l'utilisateur, pas de mémoire des élèves, pas d'entraînement, pas de partage entre personnes ; ce qui vaut d'être gardé devient une donnée métier **enregistrée par une personne** dans le socle propriétaire (synthèse de conseil, compte rendu).
7. **Il marche sans modèle.** Coupé, l'assistant garde ses réponses déterministes, sa recherche, ses cartes d'action (construites depuis des listes), et renvoie aux écrans.

---

## 2. Évaluation franche de l'existant

### 2.1 ENT Rust [F]

| Élément | Où | État |
|---|---|---|
| Copilote déterministe | `crates/ent-db/src/copilote.rs` (302 lignes), `PgEnt::copilote_repondre` (`crates/ent-db/src/service.rs`, ≈ l. 20190–20360), routes `POST /copilote/ask` et `GET /copilote/capacites` (`crates/ent-api/src/routes.rs:193-194`, handler `:5324-5358`) | lecture seule, intentions par mots-clés (effectif, absences non justifiées, impayés, sans compte, alertes CPE, capacités), bornée à l'établissement de la session, question ≤ 300 caractères ; **le post-filtre RF-04 b est fusionné** (`eleves_autorises(…, Action::AbsencesLire)`) |
| Cœur IA | — | **absent** : aucun crate `ent-ia`, aucune table `ent_ia_*`, aucune action `ia.*`, aucune plage de migrations réservée (`MIGRATIONS.md` lu le 2026-11-11) |
| Capacités déclarées par les socles | conceptions 13, 14, 15, 16, 17, 19, 22, 24, 25, 29, 34, 42 (§ « Capacités déclarées (registre `ent-ia`) ») | **sur le papier seulement** ; aucun socle codé n'a déclaré de capacité |
| Palette de commandes, recherche globale | `29_PORTAILS.md` §12 | conçue, non codée ; elle sera **cliente du même registre** |
| Cartes d'action dans la messagerie | `16_MESSAGERIE.md` §1 point 7 | conçues ; même cycle que les brouillons |
| Fournisseur, clés, quotas | — | rien ; aucune dépendance IA dans les `Cargo.toml` |

### 2.2 Défauts de l'existant Rust (à corriger en CH0, avant tout ajout)

| N° | Défaut | Preuve | Gravité | Correctif |
|---|---|---|---|---|
| **DI-01** | **Deux moteurs de droits** dans le copilote : effectif, absences et alertes par `Role::can_*` figés ; impayés et « sans compte » par le moteur D-017 (`role_can_*`) | `service.rs` ≈ l. 20211, 20240, 20301, 20327, 20343 | important | toutes les intentions par `a_action_etablissement` / `eleves_autorises` (D-017) |
| **DI-02** | **Effectif d'une classe ouvert à tout le personnel** (`can_read_effectif`), sans portée au groupe (D-022) | idem l. 20211 | important (énumération des classes) | effectif d'une classe seulement si la personne a la classe dans sa portée ; sinon effectif global agrégé, sans liste |
| **DI-03** | **Aucun journal** des questions ni des réponses ; un audit ne sait pas que des listes nominatives ont été servies par ce canal | `copilote_ask` n'écrit rien | important | journal minimal sans contenu (CH0) puis journal IA (CH2) |
| **DI-04** | **Aucune limite de débit** propre ; refus renvoyé en HTTP 200 avec `refusee: true` | handler `:5324-5358` | confort / sécurité | limite par personne (`ent_rate_limit` existant) ; refus HTTP 403 explicite, corps inchangé |
| **DI-05** | **Listes nominatives non plafonnées** (`donnees.eleves`) : « absences non justifiées » d'un CPE renvoie toute l'école sur 30 jours | `copilote_repondre` | important (volume) | plafond (50) + « et N autres, ouvrir la liste » vers l'écran du 12 |
| **DI-06** | Les **impayés** par nom de famille sont servis à quiconque a la facturation en lecture, **sans** la confidentialité de foyer du socle 24 (F15) | `role_can_lire_facturation` | important | passer par le port de lecture du socle 24 quand il existera ; d'ici là : compteur seul, sans nom |
| **DI-07** | Le copilote reconnaît « classe » par extraction libre du texte et la compare à un libellé : une classe homonyme d'une autre année ou un libellé partiel peut répondre sur la mauvaise classe | `copilote.rs` (détection) | confort | résolution par identifiant (socle 5, classe de l'année en cours), ambiguïté = liste de choix |

### 2.3 Next.js en production (lecture seule, D-035) [F]

Le module P05 « Assistant IA, OCR » est classé **« sans objet (non repris) »** par l'inventaire T0 (conversations perdables) ; ses **idées** sont reprises quand elles tiennent, son **code** non. Inventaire détaillé : §2.4 de `IA_COEUR.md` (repris, non répété), complété le 2026-11-11 par une lecture ciblée de `app/lib/brain-ai/`, `app/api/chatbot/`, `app/api/bien-etre/` (sous-agent en lecture seule, aucun fichier d'environnement ouvert). Chaîne d'exécution [F] : point d'entrée `app/api/chatbot/route.ts:39-128` (session requise, profil élève « bien-être » refusé l. 48, 30 requêtes / 60 s par utilisateur, historique **fourni par le navigateur** tronqué à 12 messages l. 64) ; orchestrateur `app/lib/brain-ai/run-chat.ts` (rejeu sans modèle des choix et confirmations l. 640-772, détection d'intention par expressions régulières l. 777-786, classement de domaine de la base de connaissances par Mistral l. 157-188, boucle d'outils ≤ 5 tours l. 40 et 902) ; `mistral-small-latest`, température 0,2 ; **résultats complets des outils envoyés au modèle** (noms, PAP, régime, présence, l. 1083-1125) ; 41 outils dans `tools/registry.ts:62-921` (dont 17 écritures : régime et grille de repas d'un élève, préinscription, absences d'accueil, voyage, réservation de salle, demande, absence RH et décision, photocopies, HSE, relance de signatures de stage, appel ouvert / enregistré / clos, chambre d'internat) ; droits à deux niveaux (liste d'outils filtrée `registry.ts:933-955`, contrôle à l'exécution `permissions.ts:59-99`) ; tests : `core-read.test.ts`, `wizard-intent.test.ts` seulement.

Ce que Next.js fait et que ScolIA reprend (idées, réécrites) :

| N° | Fonction Next.js | Où | Reprise ScolIA |
|---|---|---|---|
| NXB-01 | Assistant « Brain AI » avec outils filtrés par rôle | `app/lib/brain-ai/run-chat.ts`, `tools/registry.ts`, `tools/execute.ts` | registre de capacités (cœur), lot CH4–CH6 |
| NXB-02 | Contrôle serveur à l'exécution des outils | `tools/execute.ts` | portée à la source (IA-P3), plus strict |
| NXB-03 | Confirmation à l'écran des écritures | `run-chat.ts` | cartes N1–N3 (§11) |
| NXB-04 | Limite de 5 tours d'outils | `run-chat.ts` | budget par tâche (§9.5) |
| NXB-05 | Conversations enregistrées (`scolia_conversation`, `scolia_message`) | `scolia-conversations-db.ts` | conversation privée, 30 jours, scellée (§8) |
| NXB-06 | « Mémoire » de l'assistant : 80 derniers messages et état en cours dans le `localStorage` du navigateur, sans expiration (`scolia-memory.ts:4, 88-103`) ; conversations serveur `scolia_conversation` (état en `jsonb`) et `scolia_message`, 50 par liste, archivage doux, **aucune purge** (`drizzle/0055_scolia_conversations.sql:3-33`) | `scolia-memory.ts`, `scolia-conversations-db.ts` | conversations **serveur seulement**, scellées, 30 jours ; rien de lisible dans le navigateur ; aucune mémoire de faits ; préférences typées (R-CH9, R-CH10) |
| NXB-07 | « Wizard » : tant qu'il manque un champ, l'outil renvoie un état `needsChoices` (choix unique, multiple, date, texte), `needsFileUpload` ou `needsConfirmation` ; l'écran affiche listes et boutons ; un seul état en attente (`wizard.ts:5-48`, `conversation-state.ts:74-129`, `choice-options.ts`) | `wizard.ts`, `wizard-intent.ts` | **très bonne idée, reprise** : questions de précision **sans modèle** (listes de choix, dates en jours ouvrés, dépôt de pièce), état tenu **par le serveur** (§9.3) |
| NXB-08 | « Signaux personnels » à l'ouverture | `personal-signals.ts` | à-faire unifié du socle 29 (une seule source), expliqué par l'assistant |
| NXB-09 | Catalogue de destinations (« emmène-moi à… ») | `destination-catalog.ts` | navigation par la palette du 29 et `AST-131` |
| NXB-10 | Barre d'assistant en tête de l'accueil (« chat first ») | `app/(admin)/dashboard/page.tsx` (cité par `29_PORTAILS.md` NXP-01) | barre d'accueil (§13.2) |
| NXB-11 | Aides à la rédaction (affiches, modèles, circulaires) | `posters/suggest-copy`, `document-templates/reformulate`, `travels/generate-circular` | tâche `texte.reformuler` et rédactions de socle (§12) |
| NXB-12 | Routage des demandes vers un catalogue fermé, repli déterministe | `app/lib/requests.ts`, `requests-routing-config.ts` | **socle 31** (règles) + proposition de routage par l'assistant, jamais affectation automatique sur seuil |
| NXB-13 | OCR et extraction de pièces | `app/lib/ocr-*.ts` | socle 7 (`document.extraire`, propositions seulement) — hors socle 30 |
| NXB-14 | Ingestion de « connaissances » (`knowledge/*.json`) injectées dans les consignes | `app/api/chatbot/ingest/route.ts` | **remplacée** par une base documentaire de l'établissement **validée** (§10.6), jamais injectée comme consigne |

### 2.4 Défauts de Next.js (à ne pas reproduire ; Next.js reste intouchable, signalés à Florian)

| N° | Défaut | Preuve | Ce que fait ScolIA |
|---|---|---|---|
| DI-10 | Clé IA empruntée à **un autre établissement** si la sienne manque | `app/lib/tenant-config.ts:17-44` (`IA_COEUR.md` §2.4) | fournisseur par établissement, liste blanche, **aucun repli** (IA-P9) |
| DI-11 | Ingestion de connaissances **sans contrôle de rôle**, injectée dans les consignes (injection persistante) | `app/api/chatbot/ingest/route.ts:21-29` | base documentaire validée par `ia.documentation.valider`, toujours encadrée comme donnée (§10.6) |
| DI-12 | Conversation « bien-être » avec des **élèves mineurs** sur leur détresse, envoyée à un fournisseur externe ; secret de repli codé en dur ; numéro d'aide erroné | `app/api/bien-etre/*`, `app/lib/bien-etre-session.ts:10-16` | **aucune conversation IA sur la santé, la détresse ou le harcèlement** ; l'assistant d'un élève **oriente** vers les personnes et les numéros officiels (3020, 3018, 119, 116 111 [NV]) sans analyser (R-CH33) ; le signalement est un écran du socle 36 |
| DI-13 | NIR demandé au modèle (extraction RH) ; INE envoyés | `app/lib/personnel-match.ts:92` | identifiants nationaux **jamais** dans le contexte (C4 pour le NIR, C2 masqué pour l'INE, R-CH7) |
| DI-14 | Écriture automatique sur « confiance » déclarée par le modèle (classement à 0,9, routage à 0,52) | `ocr-analyze-eleve.ts:516-529`, `requests.ts` | la confiance **ordonne** une file, elle ne déclenche rien (R-CH16) |
| DI-15 | Arguments d'outils journalisés en clair ; erreur du fournisseur affichée telle quelle | `run-chat.ts` (IA_COEUR §2.4) | journal sans contenu ; message d'erreur neutre, code d'incident (R-CH13) |
| DI-16 | Limite de débit en mémoire du processus (repli) | `app/lib/memory-rate-limit.ts:13` | compteurs en base, par personne et par établissement (§19) |
| DI-17 | **Confirmation non liée côté serveur** : `confirm: true` et les arguments de l'action viennent de la requête ; un client exécute n'importe quelle écriture permise à son rôle, avec ses propres arguments, sans proposition préalable (pas de jeton, pas d'empreinte) | `app/api/chatbot/route.ts:60, 92, 118` → `run-chat.ts:711-746` | brouillon **stocké côté serveur**, empreinte des paramètres et de l'état simulé, confirmation par l'identifiant du brouillon seulement (IA_COEUR §6.1, R-CH18) |
| DI-18 | **Historique fourni par le navigateur** : on peut y glisser de faux messages « assistant » | `route.ts:64` | historique relu **en base**, jamais reçu du client (R-CH11) |
| DI-19 | Pièces jointes de l'état de conversation fournies par le client : une clé de stockage arbitraire de l'établissement peut être attachée ; dépôts sans limite de taille ni purge | `run-chat.ts:646-666, 975-1003`, `upload/route.ts:28-41` | pièce = objet `ent_objet` déposé par la personne dans cette conversation, vérifié par le serveur (R-CH24) |
| DI-20 | Audit : succès seulement, **toutes les écritures notées « search »**, refus en console | `tools/execute.ts:72-95` | journal IA de chaque appel, refus compris, genre exact (lecture / commande) |
| DI-21 | Outil `create_request` sans contrôle de module : tout connecté | `tools/registry.ts:573-596` | toute capacité déclare son droit, test de garde qui refuse le contraire (IA_COEUR §5.4) |
| DI-22 | Prénom de l'utilisateur et « signaux personnels » injectés dans les consignes système | `run-chat.ts:812-827` | les consignes ne contiennent aucune donnée personnelle ; le contexte de la personne est une donnée encadrée |
| DI-23 | « Bien-être » : détection de crise **par le modèle seul**, aucune règle déterministe ; numéros d'aide seulement dans les consignes ; consignes qui affirment « ne stocke rien » alors qu'un résumé est conservé 90 jours ; signalement non relancé si l'e-mail échoue | `app/api/bien-etre/chat/route.ts:30-118`, `bien-etre-prompt.ts:15-17`, `signaler/route.ts:54-86`, `bien-etre-config.ts:12, 25` | hors socle 30 (R-CH33) ; à signaler à Florian **en priorité** (risque pour des mineurs en production) |

### 2.5 Ce qu'il ne faut pas perdre (D-035 point 5)

L'utilisateur de Next.js a aujourd'hui : (1) une **barre d'assistant** en tête de son accueil ; (2) des **réponses avec actions** confirmables ; (3) des **aides à la rédaction** dans les circulaires et les affiches ; (4) un **historique** de ses conversations. Les quatre existent dans ScolIA dès CH5–CH7. Ce qui disparaît volontairement : la mémoire libre (NXB-06), l'ingestion sans contrôle (DI-11), la conversation bien-être (DI-12). Ces retraits sont **à annoncer** aux utilisateurs au moment de la bascule (Q-CH14).

### 2.6 Verdict

L'existant Rust est un **bon germe** (intentions nommées, réponses structurées, bornage par établissement) entouré de défauts de droits (DI-01 à DI-07) qu'il faut corriger avant de le brancher sur quoi que ce soit. Next.js prouve l'usage (les personnels utilisent l'assistant) et donne les bons réflexes (outils filtrés, confirmation des écritures), mais sa manière de traiter les données (santé de mineurs, NIR, repli de clé) est **exactement ce que ScolIA doit éviter**. Le cœur IA est conçu, pas construit : c'est le premier chantier du socle 30.

---

## 3. Acteurs, besoins réels, irritants

| Acteur | Ce qui lui fait perdre du temps aujourd'hui | Ce qu'il redoute | Ce qui le ravirait |
|---|---|---|---|
| **Secrétariat de scolarité** | croiser des listes (sans INE, sans e-mail, pièces manquantes), répondre dix fois à la même question de famille, ressaisir ce qu'un parent a écrit | l'erreur sur une identité, l'envoi au mauvais parent, une IA qui « invente » | « Qu'est-ce qui bloque les dossiers de 2nde ? » trié par urgence, avec le bouton pour relancer ; une réponse type préparée à partir du fil |
| **Direction** | préparer un entretien, une commission, un CA ; lire des dizaines d'échanges avant un appel difficile | qu'une synthèse oublie un fait, ou mette un jugement d'adulte en vérité ; une fuite | « Synthèse de Tom avant le rendez-vous de 17 h, avec sources, et ce que je n'ai pas le droit de voir » ; « qu'est-ce qui a changé cette semaine dans l'établissement ? » |
| **CPE, vie scolaire, AED** | trier les messages des familles le matin, justifier une à une, retrouver « qui a vu Tom à 10 h » | l'absence inquiétante manquée ; l'automatisme qui écrit seul à une famille | « Justifie les absences dont les parents ont donné un motif ce matin » → une carte de 12 lignes décochables ; « mes élèves à suivre cette semaine » expliqué |
| **Accueil** | le parent au téléphone qui épelle mal, l'Excel partagé avec la vie scolaire (D-023) | se tromper d'enfant ; remettre un enfant à la mauvaise personne | « le petit Martin de 6e, son père appelle » → bon élève, qui peut venir le chercher ; l'absence dite au téléphone devient une carte |
| **Professeur** | l'appel sur mobile entre deux salles, les messages aux familles d'un groupe, les appréciations | qu'on lui reproche un texte « écrit par une IA », qu'un élève d'un autre groupe apparaisse | « appel : tous présents sauf Léa, Tom en retard » dicté ; « qui n'a pas rendu le devoir ? » ; un brouillon d'appréciation **à partir de ses propres notes** relu (fermé jusqu'à l'avis juridique, D-032) |
| **Professeur principal** | préparer le conseil (lire chaque matière), les fiches de dialogue | oublier un élève discret qui décroche | « ce qui a changé depuis le dernier conseil pour la 3e A » ; synthèse par élève sourcée |
| **Comptabilité, intendance** | expliquer un solde au téléphone, rapprocher un virement mal libellé | l'erreur de foyer (parents séparés), un ton humiliant dans une relance | « explique le solde des enfants Martin » ; relances préparées, sobres, une carte par foyer |
| **Infirmerie, psychologue, assistante sociale** | — | **que l'IA lise la santé** | rien de nouveau dans le socle 30 : l'assistant **ne lit pas** leurs données ; il les aide sur leurs tâches non sensibles (agenda, messages neutres) |
| **AESH** | savoir où est l'élève accompagné, quels aménagements appliquer | lire ce qu'on ne doit pas lire | « où est Léo à 14 h ? » ; pastilles d'aménagement seulement |
| **Agents, documentaliste, personnels techniques** | trouver une salle libre, une procédure | — | « salle libre jeudi 10 h pour 30 » ; la documentation interne retrouvée (§10.6) |
| **Responsable légal** (si ouvert) | comprendre un document administratif, un bulletin, une facture ; savoir quoi faire ; écrire à la bonne personne | qu'on lise ses messages, qu'on profile son enfant | « que dois-je faire pour Léa cette semaine ? » ; traduction d'un avis en arabe ; aide à rédiger un justificatif qu'il envoie lui-même |
| **Parents séparés** | recevoir des informations incomplètes | que l'autre parent voie ses échanges | un assistant qui ne voit **que** ce que voit ce parent (D-014) |
| **Élève majeur, lycéen** | comprendre son emploi du temps modifié, ses absences à justifier, une procédure (stage, Parcoursup) | être surveillé ; que l'IA rapporte ses questions | « mon emploi du temps de demain ? », « comment justifier mon absence ? » ; ses questions ne sont lues par personne |
| **Collégien** | — | — | **pas d'assistant en V1** (§15.4) |
| **DPO, auditeur, Florian administrateur** | prouver ce que fait l'IA | l'opacité, le transfert hors UE, la conservation | journal lisible sans contenu, registre des traitements pré-rempli, interrupteur général, coûts visibles |

---

## 4. Une journée vue du terrain [H, à confirmer avec Florian]

- **7 h 40, CPE.** Elle ouvre ScolIA : la barre « Que voulez-vous faire ? » est en tête de l'accueil. Elle tape « messages des parents de ce matin pour des absences ». L'assistant (déterministe : lecture des fils de la boîte vie scolaire, socle 16) liste 9 messages ; il **propose** une carte « justifier 7 absences » (2 messages sont ambigus, laissés de côté et cités) ; elle décoche une ligne, confirme. 7 absences passent `justifiée` par la commande du socle 12, chaque ligne marquée `origine = ia:<brouillon>`.
- **8 h 05, professeur, mobile.** Entre deux salles, il dicte « appel 4B tous présents sauf Léa, Tom en retard ». La reconnaissance vocale se fait **sur l'appareil** ; la carte du 12 s'affiche, il confirme du pouce.
- **9 h 30, accueil.** Un père appelle « pour le petit Martin en 6e ». « Martin 6e » → trois élèves ; la carte de choix montre classe et prénom ; le bon, avec les personnes autorisées à venir le chercher (socle 14). L'absence annoncée devient une carte pour la vie scolaire.
- **11 h, secrétariat.** « Qu'est-ce qui bloque les dossiers de réinscription de 1re ? » → 23 dossiers, triés (pièce manquante, signature du second parent, paiement), chacun avec son bouton ; « prépare une relance pour les 14 qui n'ont que la pièce manquante » → **un brouillon d'envoi N3** (14 destinataires résolus par le socle 4, texte du gabarit du socle 3, aperçu complet), elle se ré-authentifie et envoie.
- **14 h, direction.** Avant un rendez-vous : « synthèse de Tom D. depuis septembre ». Réponse en six blocs (scolarité, assiduité, vie scolaire, résultats, échanges, à-faire), **chaque fait sourcé**, et en pied : « non consulté : santé, suivi social (non accessibles à l'assistant) ».
- **17 h 30, professeur principal.** « Qu'est-ce qui a changé pour la 3e A depuis le conseil du 1er trimestre ? » → par élève, seulement ce qu'il peut voir (D-022, fenêtre du conseil hors période : son groupe).
- **20 h 45, parent (si ouvert).** « Je ne comprends pas ce courrier » (convocation à une commission éducative) : explication en mots simples, **dans sa langue**, avec le lien vers le courrier et la personne à contacter. Il demande « aide-moi à répondre que je ne peux pas venir mardi » : brouillon qu'**il** envoie, remis le lendemain à 7 h (plage calme du 16).

---
## 5. Ce qui est réglementaire (vérifié le 2026-11-11)

Recherche par sous-agent sur sources officielles le 2026-11-11. **[V]** lu sur la source officielle ; **[V~]** lu à travers l'outil de lecture (extraits ou paraphrase) ; **[S]** source secondaire ; **[NT]** non trouvé. EUR-Lex (anti-robot), les pages HTML d'education.gouv.fr, d'eduscol et du Conseil de l'UE ont refusé l'accès : l'AI Act a été lu sur l'**AI Act Service Desk** de la Commission (version consolidée au 27/07/2026, **en anglais**). Aucune règle n'est étendue au-delà du texte lu.

### 5.1 Cadre de référence du numérique pour l'éducation (arrêté du 7 septembre 2026) — public seulement

Source : https://www.legifrance.gouv.fr/jorf/id/JORFTEXT000054883370 (NOR MENN2623554A, JORF n° 222 du 23/09/2026) [V~].
- **Champ** (art. 2) : SI, services et outils numériques mis en œuvre dans les **collèges et lycées publics** ; le privé sous contrat n'est pas visé [V~]. ScolIA y est soumis dès qu'il équipe un EPLE (D-029).
- **Annexe 3, chapitre 5 « Usages de l'IA », § 5.4** [V~] : **exigence n° 10** : les services « prennent en compte les principes fixés » par le **cadre d'usage de l'IA en éducation** ; **exigence n° 11** : les outils d'**IA générative** ne peuvent être intégrés **pour un usage en classe** que si l'accès est « **réellement anonymisé et sécurisé à travers le GAR** » ; ouverture aux élèves **à partir de la 4e sous supervision de l'enseignant**, puis **en autonomie au lycée dans un cadre défini par l'enseignant** (paraphrase, texte intégral non obtenu). § 5.3 : pas d'IA générative grand public quand des données personnelles sont en jeu ; transparence ; devoir fait par IA sans autorisation = fraude ; formation des enseignants.
- **Annexe 4, délais** [V~] : critères n° 35 (fonctionnalités d'IA conformes au cadre d'usage) et n° 36 (IA générative : supervision dès la 4e, autonomie au lycée) : **10 mois** (≈ 23/07/2027) pour les EDT, ENT, contenus et gestion administrative ; **« sans objet » pour la vie scolaire**.
- **Lecture pour ScolIA [R, à confirmer par un juriste, Q-CH1]** : l'exigence n° 11 vise l'**usage pédagogique en classe** par les élèves ; l'assistant du socle 30 n'est pas un outil de classe (il aide à se repérer dans la vie de l'établissement), et ses usages par les **personnels** ne sont pas traités par le passage lu. Prudence retenue : **dans le public, l'assistant des élèves reste fermé** tant que la lecture juridique n'est pas faite (R-CH26), et aucune tâche du 30 n'est présentée comme un outil pédagogique ; l'assistant des personnels et des familles reste possible sous les autres conditions.

### 5.2 Cadre d'usage de l'IA en éducation (ministère)

Affiche officielle, PDF du 21/08/2025 : https://eduscol.education.fr/document/67356/download [V] (page HTML https://www.education.gouv.fr/cadre-d-usage-de-l-ia-en-education-450647 : 403, [NT]). Elle dit : « aucune donnée confidentielle ou à caractère personnel ne doit y être utilisée » dans les services grand public ; « ne demandez en aucun cas aux élèves de se créer un compte personnel » sur ces services ; « **signalez toute utilisation de l'IA dans une prise de décision** » ; au collège, usage « limité, encadré, expliqué et accompagné par l'enseignant », autorisé **en classe à partir de la 4e** ; au lycée, usage autonome dans un cadre « explicitement défini par l'enseignant » ; éviter les détecteurs d'IA ; privilégier les solutions libres.
**Conséquences** : l'assistant de ScolIA n'est pas un service grand public (contrat, hébergement UE, pas de réutilisation) et c'est précisément ce qui permet aux personnels de **ne plus copier** des messages de parents dans une IA publique (objectif de `AI_PLATFORM.md`) ; la mention « préparé avec l'assistant » et le journal `origine = assistant:…` répondent à « signalez toute utilisation de l'IA dans une prise de décision » (R-CH20) ; collégiens fermés en V1 (plus strict que le cadre, D-031) ; **aucun détecteur d'IA** dans ScolIA (R-CH35).

### 5.3 Règlement (UE) 2024/1689 (AI Act), version consolidée après l'omnibus

Source : https://ai-act-service-desk.ec.europa.eu/en/ai-act/ (articles cités) [V~, anglais].
- **Calendrier** (art. 113 consolidé) : interdictions (art. 5) depuis le 2/02/2025 ; application générale le 2/08/2026 ; **haut risque de l'annexe III le 2/12/2027** [V~]. Omnibus : **règlement (UE) 2026/1744**, lien vu sur le Service Desk [V~] ; publication au JOUE le 24/07/2026 et entrée en vigueur le 27/07/2026 [S]. *Correction de `IA_COEUR.md` §9.3* : le report au 2/12/2027 est désormais sourcé (il était [NV]).
- **Art. 5 § 1 f)** [V~] : interdiction d'utiliser des systèmes d'IA « to infer emotions of a natural person in the areas of workplace and education institutions », sauf raisons médicales ou de sécurité. **Conséquence** : ScolIA n'infère **jamais** d'émotion, d'humeur, de détresse ou d'état d'un élève ou d'un personnel, ni à partir de la voix, ni de l'image, ni du texte (R-CH34) — c'est aussi pourquoi la classification de détresse de Next.js (DI-12, DI-23) n'est pas reprise.
- **Annexe III, point 3** [V~] : haut risque pour (a) l'accès, l'admission, l'affectation ; (b) « evaluate learning outcomes », y compris pour orienter l'apprentissage ; (c) l'évaluation du niveau approprié ; (d) la surveillance des comportements interdits pendant les épreuves. **Art. 6 § 3** [V~] : pas de haut risque pour une tâche procédurale étroite, l'amélioration d'une activité humaine déjà réalisée, la détection d'écarts sans remplacer l'évaluation humaine, ou une tâche préparatoire — **mais un système qui fait du profilage de personnes reste toujours à haut risque**. **Conséquences** : aucune des quatre finalités n'est confiée à l'assistant (IA-P12) ; l'aide à la rédaction d'appréciations reste **fermée** (D-032, Q-IA12) car elle touche (b) ; aucun **profilage** d'élève par l'assistant (pas de « profil », pas de score, pas de catégorie) ; l'assistant ne fait que des tâches préparatoires et d'amélioration d'un texte humain, documentées comme telles (évaluation art. 6 § 4 rédigée par ScolIA, Q-CH2).
- **Art. 4** (modifié par l'omnibus) [V~] : fournisseurs et déployeurs prennent des mesures pour soutenir la **maîtrise de l'IA** de leur personnel, sans obligation d'un niveau garanti. **Conséquence** : courte formation intégrée à la première activation par une personne (AST-226), attestée et datée (`ent_ia_formation`).
- **Art. 26** (déployeurs, pour les systèmes à haut risque) [V~] : contrôle humain compétent, surveillance, journaux conservés au moins six mois, information des travailleurs, AIPD. ScolIA ne vise pas le haut risque ; il adopte **par prudence** les bonnes pratiques compatibles : journal IA conservé comme l'audit (≥ 6 mois), information des personnels, AIPD type.
- **Art. 50** [V~] : le **fournisseur** informe la personne qu'elle interagit avec une IA, au plus tard à la première interaction, sauf évidence ; un texte généré publié pour informer le public sur des questions d'intérêt public est signalé **sauf relecture humaine sous responsabilité éditoriale**. **Conséquences** : mention permanente dans le panneau et l'appli (R-CH19) ; un texte relu et endossé par son auteur peut perdre la mention visible (Q-IA10), le journal la garde.
- **Rôles** (art. 3, définitions) [NT, page refusée] : ScolIA serait **fournisseur** du système d'IA intégré, l'établissement **déployeur** [H, à confirmer, Q-CH2].

### 5.4 RGPD et loi Informatique et libertés

- **Art. 22** [V~] (https://www.cnil.fr/fr/reglement-europeen-protection-donnees/chapitre3) : droit de ne pas faire l'objet d'une décision fondée exclusivement sur un traitement automatisé produisant des effets juridiques ou significatifs. **Conséquence** : aucune décision par l'assistant (IA-P12) ; chaque carte est confirmée par une personne qui voit l'aperçu complet.
- **Art. 8 et loi I&L art. 45** [V~] : 15 ans en France pour le consentement seul aux services de la société de l'information. **Mais** dans un ENT la base légale est la **mission d'intérêt public**, pas le consentement (CNIL, ci-dessous) : l'opposition de l'élève ou de ses responsables à l'assistant (18 `outils_ia_eleve`) est un **droit d'opposition** et une préférence, pas un consentement.
- **AIPD** [V~] : délibération CNIL n° 2018-327 (https://www.legifrance.gouv.fr/jorf/id/JORFTEXT000037559521) et critères du CEPD (deux critères sur neuf suffisent) : personnes vulnérables (élèves) + usage innovant → **AIPD requise** pour l'assistant.

### 5.5 CNIL

- **FAQ « Éducation : mettre en place un système d'IA »** du 20/06/2025 [V~] (https://www.cnil.fr/fr/education-mise-en-place-systeme-ia) : responsable de traitement en second degré = **chef d'établissement** (conjointement avec la collectivité si elle met l'outil en place) ; **AIPD « oui, la plupart du temps »**, et la CNIL recommande une **AIPD-cadre fournie par l'éditeur** ; base légale : **mission d'intérêt public** (le consentement n'est en principe pas valable) ; information « simple et pédagogique » des mineurs ; l'outil « ne doit pas réutiliser les données personnelles des élèves » ; transferts hors UE encadrés, solutions sur l'appareil ou sur site recommandées ; ne pas noter ni donner un travail en se fondant uniquement sur la sortie de l'outil ; détecteurs d'IA déconseillés. *Contradiction* : la FAQ annonce l'AI Act complet au 2/08/2027, dépassé par l'omnibus.
- **FAQ « Enseignant »** du 20/06/2025 [V~] (https://www.cnil.fr/fr/enseignant-usage-systeme-ia) : informer le chef d'établissement, ressources du GAR recommandées, ne pas déléguer l'évaluation à l'IA.
- **Conséquences** : ScolIA fournit l'**AIPD-cadre** pré-remplie (AST-227) ; la notice aux familles et la notice « élèves » sont fournies dans un langage adapté ; aucune réutilisation (contrat fournisseur, §17) ; dictée sur l'appareil (Q-IA11) conforme à la recommandation « sur l'appareil ».
- **ENT** : avis CNIL de 2006 [S] (chaque compte n'accède qu'aux données de son titulaire) ; aucune recommandation ENT 2025–2026 trouvée [NT] ; SDET 2025 vu sur une copie académique, contenu IA non vérifié [NT].

### 5.6 Fournisseurs (pages officielles, 2026-11-11)

| Fournisseur | Ce que dit la page | Statut |
|---|---|---|
| **Mistral AI** — conditions commerciales (en vigueur au 25/09/2026), https://legal.mistral.ai/terms/commercial-terms-of-service/ | § 4.2 : pas d'entraînement sur les données et sorties du client, **sauf** opt-in, **retours (feedback)** ou bon de commande ; § 4.3 : les modèles **Labs / Preview** peuvent servir à l'entraînement et la zéro rétention ne s'y applique pas | [V~] |
| Mistral — zéro rétention, https://docs.mistral.ai/admin/monitor-comply/zero-data-retention | **sur demande, à la discrétion de Mistral**, plans payants ; couvre les points d'accès sans état (chat, embeddings, modération, OCR, voix), **exclut** Agents, Batch, Conversations, Libraries, fichiers | [V~] |
| Mistral — localisation, https://help.mistral.ai/en/articles/347629 (12/08/2026) | hébergé **dans l'UE par défaut** ; point d'accès US optionnel ; transferts temporaires possibles vers des sous-traitants | [V~] |
| Mistral — rétention 30 jours sans ZDR, DPA (27/07/2026) | extraits de recherche seulement | [S] |
| **Scaleway Generative APIs**, https://www.scaleway.com/en/docs/generative-apis/reference-content/data-privacy/ | **zéro rétention par défaut**, « we do not collect, read, reuse, or analyze » les requêtes, pas d'entraînement, **Paris** ; exception : trafic anormal → requête complète conservée jusqu'à deux semaines ; statistiques anonymisées 6 mois | [V~] |
| **OVHcloud AI Endpoints**, https://docs.ovhcloud.com/en/guides/public-cloud/ai-machine-learning/ai-endpoints-capabilities (10/11/2026) | « data is not stored or shared during or after model use », Europe ; annexe contractuelle « AI Endpoints » non lue | [V~] / [S] |

**Conséquences [R]** : (1) jamais de modèle « Labs / Preview » ni de point d'accès avec état (Agents, Conversations, Batch, fichiers) chez Mistral : seuls les points d'accès **sans état** sont permis, la conversation est tenue par ScolIA (R-CH21) ; (2) **le bouton « pouce » de ScolIA ne transmet jamais rien au fournisseur** (les retours peuvent servir à l'entraînement chez Mistral, § 4.2) ; (3) **recommandation pour Florian (Q-IA2, contrat)** : premier candidat pour la classe standard = **Scaleway Generative APIs** (zéro rétention par défaut, Paris, même hébergeur que ScolIA) **avec des modèles Mistral ouverts** hébergés par Scaleway — ce qui satisfait « Mistral prioritaire » (CLAUDE.md) sans dépendre d'une zéro rétention accordée « à la discrétion » ; second candidat : API Mistral avec ZDR **écrite au contrat** ; OVHcloud en troisième, après lecture de son annexe. Aucun contrat n'est signé par la session.

### 5.7 Ce qui reste à vérifier [NV]

Texte intégral de l'exigence n° 11 et de la partie « IA » de l'annexe 3 (PDF du JO) ; visas de l'arrêté (décret n° 2025-1165 non vu) ; AI Act en français sur EUR-Lex ; définitions de l'art. 3 ; référence JOUE de l'omnibus ; DPA Mistral et annexe AI Endpoints d'OVHcloud ; numéros d'aide cités au §15.4 (3018, 3020, 119, 3114) sur leurs sites officiels ; contenu IA du SDET 2025.


---

## 6. Frontières, branchements, corrections du cœur

### 6.1 Qui possède quoi

| Sujet | Propriétaire | Le socle 30 |
|---|---|---|
| Registre des capacités, journal IA, brouillons, confirmation, minimisation, pseudonymisation, fournisseurs, classes de traitement | **cœur IA** (`IA_COEUR.md`) | **le construit** (lots CH1–CH4, CH8), faute d'autre porteur (R-CH1) ; ne change aucun de ses principes |
| Conversation, compréhension des demandes, enchaînements, surfaces, préférences, retours, ouverture familles et élèves, réglages et quotas visibles | **30** | possède |
| Capacités (lectures, commandes, aperçus, niveaux de confirmation) | **chaque socle** | consomme ; n'en ajoute **aucune** sur les données d'un autre socle |
| Palette de commandes, recherche globale, à-faire | **29** | client du même registre ; l'assistant appelle `a_faire_de(acteur)` |
| Cartes d'action dans les fils, destinataires, plage calme | **16** | une réponse préparée est une commande du 16, destinataires imposés par le fil |
| Envois (e-mail, SMS, push), traduction des gabarits | **8** | la traduction proposée d'un texte libre (NTF-48, NTF-130) est une tâche du 30, relue dans l'aperçu du 8 |
| Règles automatiques, tâches planifiées, file de validation des propositions de règles | **31** | partage la **même table de brouillons** (§26.2) ; une règle peut être **rédigée** avec l'assistant (31 AUT-140), jamais activée par lui |
| Extraction et classement de pièces (OCR) | **7** | `document.extraire` reste au 7 ; l'assistant peut l'appeler comme capacité |
| Synthèse de conseil, fiche de dialogue | **23** | la synthèse « officielle » est un document du 23, enregistré par une personne |
| Appréciations | **21 / 23** | tâche préparée mais **fermée** jusqu'à l'avis juridique (D-032, Q-IA12) |
| Registre des traitements, durées, droit d'accès | **27** | traitements IA déclarés ; l'extrait d'accès inclut le journal IA de la personne |
| Indicateurs d'usage, de coût, de refus | **47** | le 30 publie des compteurs agrégés |
| Mobile (coquille, biométrie, dictée sur l'appareil) | **11** | le 30 fournit les routes et les cartes |

### 6.2 Ports consommés

`capacites(acteur, tache)` (cœur) ; `a_faire_de(acteur)` (29) ; `contexte_ecran(type, id)` (chaque écran Leptos transmet un identifiant connu du serveur : élève ouvert, fil, classe, conseil) ; `resoudre_personnes(texte, portee)` (recherche lexicale du 2, E6, et annuaire des personnels du 6) ; `destinataires_fil(fil_id)` (16) ; `gabarits(nature, langue)` (8) ; `plage_calme(etablissement, instant)` (8, 16) ; `fuseau(etablissement)` (5) ; `objets.deposer` (`ent-object`, E5) ; `journal_acces_sensible` (1) ; `notices_publiees` (27) ; `consentement(outils_ia_eleve)` (18).

### 6.3 Ports exposés par le socle 30

`assistant.demander(acteur, texte, contexte)` (web, mobile) ; `assistant.preparer(acteur, commande, parametres)` pour le 31 et la palette (crée un brouillon) ; `assistant.brouillons_de(acteur)` (29 : « 3 cartes à confirmer ») ; `assistant.etat(etablissement)` (actif, fournisseur, tâches ouvertes : affiché dans « ce que voit ScolIA de moi », 29) ; `assistant.usage(etablissement, periode)` (47) ; `assistant.exclusion(eleve)` (« hors IA », lu par la construction du contexte, Q-IA6).

### 6.4 Corrections apportées au cœur IA (proposées, appliquées par défaut D-031)

1. **D-036 s'applique au cœur, qui le précède.** `IA_COEUR.md` §6.3 décrit `capacites_appelees` et `donnees_envoyees` comme des listes dans une ligne de journal : elles deviennent des **tables filles** (`ent_ia_journal_capacite`, `ent_ia_journal_donnee`, `ent_ia_journal_alerte`) ; aucune colonne JSON (§26).
2. **La fiche de contexte n'est jamais stockée.** Le cœur la décrit comme un document structuré envoyé au modèle : c'est une **charge transitoire** (exception déjà admise par D-036 pour une file d'envoi), construite en mémoire, jamais écrite en base ; seul son **inventaire** est journalisé (types d'entités, nombres, classes, pseudonymisation).
3. **Une seule table de brouillons pour l'IA, la palette et les règles.** Le cœur l'appelle `ent_ia_brouillon` ; elle devient `ent_brouillon_action` avec une colonne `origine` (`assistant`, `palette`, `regle`, `controle`) et un destinataire qui peut être une **personne** ou une **file de mission** (pour le 31). Le cycle est identique (empreinte, expiration, re-vérification).
4. **Le contenu des échanges** n'est pas un champ unique scellé : c'est une suite de **messages** (`ent_ia_message`, un texte scellé par ligne) avec leurs **sources** en lignes typées (`ent_ia_message_source`). La conversation (socle 30) et l'échange (cœur) sont la même chose : une seule structure.

### 6.5 Points à accorder avec les autres socles

- **29** : la barre d'accueil (NXP-01) appartient au 29 comme emplacement, au 30 comme comportement ; la palette garde les mots-clés, l'assistant le langage naturel (29 §12.2) ; une même demande tapée dans la palette et dans l'assistant mène à la même commande.
- **16** : un brouillon de réponse est un **brouillon de message du 16** (non envoyé, visible dans le fil de son auteur seulement), pas un objet du 30 ; l'envoi suit les règles du 16 (plage calme, foyer, gel).
- **18** : le consentement `outils_ia_eleve` du 18 sert d'**information et d'opposition** pour l'assistant des élèves (§15) ; il ne vaut pas base légale (la base est la mission d'intérêt public ou l'exécution du contrat de scolarisation [H, DPO]).
- **31** : une règle n'appelle jamais le modèle de langage (R-CH38 et 31 R-AU4) ; l'assistant peut **expliquer** une règle et en **rédiger** une, jamais l'activer.
- **27** : durées (§21.4), extrait d'accès, registre, AIPD type fournie par ScolIA.

---

## 7. Vocabulaire fixé [R]

| Terme | Sens |
|---|---|
| **Assistant** | l'interface conversationnelle de ScolIA, une par personne et par établissement ; il n'a ni compte ni droit |
| **Conversation** | suite de messages entre une personne et l'assistant, privée, courte, effaçable |
| **Demande** | ce que la personne écrit ou dit |
| **Compréhension** | traduction d'une demande en une **tâche** du catalogue (avec paramètres résolus), par règles puis, au besoin, par le modèle |
| **Tâche** | opération typée du cœur (`eleve.resumer`, `action.preparer`…) : capacités permises, budget, consignes versionnées, schéma de sortie |
| **Capacité** | lecture ou commande déclarée par un socle (cœur §4.3) |
| **Source** | objet consulté pour répondre, cité avec un lien vers son écran |
| **Masquage** | donnée existante que la personne (ou l'assistant) ne peut pas lire ; toujours dit |
| **Carte** | aperçu d'une commande, rédigé par le socle propriétaire, avec Confirmer / Modifier / Abandonner |
| **Brouillon d'action** | la carte côté serveur (`ent_brouillon_action`) : commande, paramètres résolus, empreinte, expiration |
| **Plan** | suite d'étapes annoncée avant d'agir ; chaque étape d'écriture est une carte distincte |
| **Préférences** | réglages personnels typés de l'assistant (langue, ton, longueur, format) ; pas une mémoire |
| **Base documentaire** | textes de l'établissement (règlement, procédures, FAQ) **validés** et versionnés, consultables comme sources |
| **Hors IA** | marque posée sur un élève (ou une personne) : aucune de ses données n'entre dans un contexte envoyé à un modèle |
| **Fournisseur nul** | réponse « assistant sans modèle » : déterministe seulement |

---

## 8. La conversation

### 8.1 Ce qu'elle est [R]

- **Une par sujet**, créée à la première demande ; titre **neutre** proposé par le serveur à partir de la tâche (« Synthèse d'élève — 11/11 », jamais un nom ni un motif), modifiable par l'auteur.
- **Rattachée à un contexte** facultatif (élève, fil, classe, conseil) : l'en-tête l'affiche (« sur la fiche de Léa D. ») ; changer d'élève ouvre une nouvelle conversation (R-CH8), pour ne pas mêler deux élèves dans un même fil de contexte.
- **Messages** : demande, réponse, carte, refus, plan, question de précision ; chaque réponse garde ses sources et l'identifiant du journal.
- **Liste** des conversations de la personne (50 dernières), recherche dans **ses** titres, épingle, suppression immédiate par l'auteur (le journal sans contenu reste).
- **Durée** : contenu scellé effacé à 30 jours après le dernier message (Q-IA9, à valider DPO ; réglable par l'établissement **vers le bas** seulement, de 0 à 30 jours, S-3001) ; « 0 jour » = rien n'est gardé après la fermeture.
- **Jamais partagée.** Pas de lien de partage, pas de « transférer à un collègue » : si une réponse doit être transmise, la personne la copie dans un message (16) ou l'enregistre comme document du socle propriétaire, sous sa responsabilité.

### 8.2 Ce qu'elle n'est pas

- Pas une **mémoire** : l'assistant ne réutilise pas les faits d'une conversation dans une autre ; il relit les services (une absence justifiée entre-temps est vue justifiée).
- Pas un **dossier** : une conversation n'est pas une pièce du dossier de l'élève ; elle n'apparaît pas dans sa chronologie ; elle n'est pas dans l'extrait d'accès de la famille (seul le **journal d'accès** l'est, comme pour une consultation à l'écran).
- Pas une **preuve** : une décision prise après une réponse de l'assistant est une décision de la personne, tracée par la commande qu'elle confirme.

### 8.3 Lecture par d'autres (incident seulement) [R]

- Personne ne lit les conversations d'autrui, **ni la direction, ni l'administrateur de l'établissement, ni ScolIA**.
- Exception : **incident** (fuite suspectée, contenu illicite signalé, demande d'une autorité). Procédure : demande motivée par l'administrateur (`ia.incident.ouvrir`) → validation par une **seconde personne** (direction ou DPO de l'établissement) → accès en lecture à la seule conversation visée, pour 7 jours, journalisé, **l'auteur est informé** sauf interdiction légale (R-CH12). La conservation est **gelée** le temps de l'incident.
- Le **support ScolIA** ne lit aucun contenu ; il voit le journal sans contenu (identifiants, tâches, coûts, erreurs) si l'établissement ouvre un ticket.

---

## 9. Comprendre une demande

### 9.1 L'ordre de la compréhension [R]

1. **Contexte d'écran** : l'écran fournit un identifiant (élève, fil, classe, conseil, document) ; il borne la demande.
2. **Commandes directes** : la demande commence par un verbe du catalogue ou un alias de la palette (« abs Léa », « appel 4B ») → tâche sans modèle.
3. **Intentions déterministes** : les questions fréquentes (effectif, absences du jour, appels non faits, impayés agrégés, élèves sans compte, salles libres, mon emploi du temps…) reconnues par motifs et servies par une capacité : **aucun appel au modèle** (l'actuel copilote, étendu et rebranché).
4. **Résolution des noms** : les noms de personnes, classes, groupes, salles sont cherchés **dans la portée** (cœur §7.3) ; un nom ambigu donne une liste de choix (sans modèle) ; un nom inconnu dans la portée donne « je ne trouve pas cette personne parmi celles que vous pouvez consulter », **sans dire si elle existe ailleurs**.
5. **Modèle** (si actif et si nécessaire) : classification de la demande dans le **catalogue fermé** des tâches ouvertes à la personne, extraction des paramètres par schéma ; sortie validée (tâche connue, paramètres typés, références = jetons présents).
6. **Échec** : « je n'ai pas compris ; voici ce que je sais faire ici » + 3 suggestions liées au contexte + le lien vers l'écran.

### 9.2 Catalogue de tâches du socle 30 (en plus de celles du cœur)

| Tâche | Capacités permises | Classe | Sortie | Modèle ? |
|---|---|---|---|---|
| `question.frequente` | une lecture déterministe | — | réponse structurée | non |
| `naviguer` | destinations du 29 | C0 | lien | non (ou classification) |
| `aide.utilisation` | base documentaire ScolIA (aide du logiciel) | C0 | texte + liens | oui (ou recherche) |
| `etablissement.documentation` | base documentaire de l'établissement validée | C0 | texte + sources | oui |
| `liste.filtrer` | ports de liste des socles (élèves, absences, travaux, factures…) | C1–C2 | filtre structuré exécuté par le socle | oui |
| `eleve.resumer`, `eleve.quoi_de_neuf` | cœur | C1–C2 | blocs sourcés | oui |
| `classe.quoi_de_neuf` | lectures agrégées par classe de la portée | C1 | par élève, sourcé | oui |
| `fil.resumer` | messages d'un fil où la personne participe | C2 (non fiable) | résumé + points en attente | oui |
| `message.preparer_reponse` | fil + gabarits | C2 | **brouillon du 16** | oui |
| `message.preparer` (nouveau) | destinataires choisis à l'écran ou par la commande du 16 | C1–C2 | brouillon du 16 | oui |
| `texte.reformuler`, `texte.corriger`, `texte.simplifier` | aucune donnée élève | C0 | texte | oui |
| `texte.traduire` | texte de la personne ou document qu'elle peut lire | C0–C2 | texte marqué « traduction automatique » | oui |
| `document.expliquer` | un document lisible par la personne (7) | C1–C2 | explication + passages cités | oui |
| `action.preparer` | commandes ouvertes | selon la commande | **carte** | oui (ou sans modèle si commande directe) |
| `plan.preparer` | 2 à 5 commandes ouvertes | selon | plan + première carte | oui |
| `regle.rediger` (31) | catalogue des déclencheurs, faits, effets du 31 | C0 | **brouillon de règle** (jamais active) | oui |
| `regle.expliquer` (31) | une règle et ses exécutions visibles | C0–C1 | texte | oui |
| `a_faire.expliquer` | `a_faire_de(acteur)` | C1 | liste priorisée et expliquée | non (ou formulation) |

### 9.3 Questions de précision sans modèle (repris de NXB-07) [R]

Quand une commande manque d'un paramètre, le serveur renvoie un **état de précision** (choix unique, choix multiple, date, créneau, pièce à déposer, texte court) ; l'écran affiche la liste ou le contrôle ; la réponse de la personne **est un identifiant**, pas une phrase réinterprétée. Un seul état de précision en attente par conversation ; il expire avec le brouillon (15 min). Les dates relatives (« jeudi », « demain », « la semaine prochaine ») sont calculées par le serveur dans le **fuseau de l'établissement** et en **jours ouvrés** du calendrier du 5 ; une date ambiguë (« jeudi » un mercredi soir) donne un choix.

### 9.4 Langues [R]

L'assistant comprend et répond dans la langue de la demande pour les tâches de lecture et d'explication ; les **brouillons destinés à l'établissement** (messages, justificatifs) écrits par une famille dans une autre langue sont proposés **dans sa langue et en français** ; les textes produits par l'établissement restent en français, la traduction étant marquée (NTF-48).

### 9.5 Budget d'une demande [R]

Par demande : au plus **5 appels de capacités** (repris de NXB-04), **2 appels au modèle** (compréhension, rédaction), un budget de jetons par tâche (cœur §7.4), une durée maximale (20 s pour une réponse, 60 s pour une synthèse longue, au-delà : file avec reprise et notification). Dépassement : réponse partielle annoncée (« j'ai lu les 3 derniers mois ; ouvrir la chronologie complète »).

---

## 10. Répondre : sources, refus, explications

### 10.1 Forme d'une réponse [R]

1. **Le fait d'abord**, en une phrase (« 3 élèves n'ont pas rendu le devoir du 7/11 »).
2. **Le détail** en liste ou tableau compact (au plus 20 lignes ; au-delà, « ouvrir la liste complète » vers l'écran du socle, filtrée de la même façon).
3. **Les sources** : un lien par objet ou par liste consultée (« Travaux de 3e B — maths, cahier de textes »).
4. **Ce qui n'a pas été consulté** : « non consulté : santé (non accessible à l'assistant) ; notes d'histoire (hors de vos groupes) ».
5. **Les actions possibles** : 1 à 3 boutons (cartes à préparer, écrans à ouvrir).

### 10.2 Chiffres et dates [R]

Un **chiffre** n'est jamais produit par le modèle : il vient d'une capacité (le modèle peut le citer, pas le calculer) ; la validation de sortie compare les nombres de la réponse à ceux des résultats de capacités et **rejette** une réponse qui contient un nombre absent des sources (sauf numérotation et dates de la question). Les **dates** sont affichées au format de l'établissement, avec le jour de la semaine.

### 10.3 Refus [R]

| Cas | Message | HTTP |
|---|---|---|
| droit absent | « Votre rôle ne permet pas de consulter les notes. » | 200 dans la conversation, 403 sur la capacité |
| hors portée | « Tom n'est pas dans vos groupes : je ne peux pas lire ses absences. » | idem |
| donnée fermée à l'IA | « Les informations de santé ne sont jamais lues par l'assistant. Ouvrez l'infirmerie si vous y avez accès. » | — |
| élève « hors IA » | « La famille a demandé que les données de cet élève ne soient pas traitées par l'assistant. » (visible au personnel qui a déjà le droit de voir l'élève) | — |
| tâche fermée | « La préparation d'appréciations n'est pas ouverte dans l'établissement. » | — |
| fournisseur indisponible | « L'assistant ne peut pas rédiger pour l'instant. Les réponses simples et les actions restent disponibles. » | — |
| quota | « Vous avez atteint la limite du jour ; elle se réinitialise demain à 6 h. » | 429 |

### 10.4 Explications [R]

« Pourquoi ? » sur une alerte, un montant, un masquage, une règle du 31 : l'assistant lit la **chaîne de causalité** (événements corrélés de l'outbox, `correlation_id`) et la règle déclarée par le socle, puis l'explique ; il ne reconstruit jamais une explication que le socle ne fournit pas.

### 10.5 Ton et neutralité [R]

Consignes versionnées : registre administratif sobre ; aucun jugement de valeur sur un élève ou une famille ; un jugement écrit par un adulte est **cité comme tel** (« M. X a écrit : … »), jamais repris comme un fait ; aucun diagnostic, aucune catégorie psychologique ; aucune prédiction sur l'avenir d'un élève ; aucune comparaison nominative entre élèves en dehors de ce que l'écran montre déjà (classement de classe : non).

### 10.6 Base documentaire de l'établissement (remplace NXB-14 / DI-11) [R]

- **Contenu** : règlement intérieur, procédures internes (« que faire si un élève se blesse en EPS »), FAQ des familles, calendrier commenté, notices ; **textes**, pas des données d'élèves.
- **Chemin** : un document du socle 7 marqué « documentation de l'assistant », **validé** par une personne qui a `ia.documentation.valider` (direction par défaut), versionné, avec un **public** (personnels, familles, élèves) ; le retrait d'une version la retire des réponses.
- **Usage** : recherche plein texte (PostgreSQL, portée = public du document) puis citation des passages ; **toujours encadrée comme donnée** (jamais concaténée aux consignes), donc incapable de donner un ordre à l'assistant.
- **Aide du logiciel** : la documentation de ScolIA (comment justifier une absence…) est une base globale versionnée par ScolIA, de classe C0.

---

## 11. Agir : cartes, brouillons, enchaînements

### 11.1 La carte [R]

Le cycle est celui du cœur (§6.1) ; le socle 30 précise l'affichage :
- **En-tête** : verbe et objet (« Justifier 7 absences — vie scolaire »).
- **Corps rédigé par le socle propriétaire** à partir de la simulation : lignes avant / après, destinataires, montants, effets prévus (« la famille de Léa recevra une notification à 7 h »).
- **Commentaire de l'assistant** (facultatif), **visuellement séparé** et marqué « commentaire de l'assistant » (T7 du cœur).
- **Cases à décocher** pour un lot ; compteur « 7 sur 9 retenues, 2 écartées (motif ambigu) » avec la raison de l'écart.
- **Boutons** : Confirmer (avec passkey en N3), Modifier (ouvre l'écran normal pré-rempli), Abandonner ; expiration affichée.
- **Après** : message de résultat avec lien vers l'objet créé et, si la commande en a, le bouton **Annuler** pendant son délai.

### 11.2 Plan et enchaînements [R]

- Une demande qui implique plusieurs écritures (« fais l'appel puis préviens les parents des absents ») donne d'abord un **plan** : étapes numérotées, chacune avec sa commande et son niveau ; aucune écriture.
- La personne lance l'étape 1 ; la carte 1 est préparée, confirmée, exécutée ; **la carte 2 est préparée ensuite**, à partir de l'état réel (les absents effectivement enregistrés), pas de la phrase d'origine (R-CH17).
- Au plus **5 étapes** ; une étape refusée ou abandonnée arrête le plan (la personne peut reprendre).
- Un plan n'est jamais exécuté par une règle du 31, ni repris après la fin de la session de la personne.

### 11.3 Ce que l'assistant ne prépare jamais [R]

Toute commande marquée **« fermée à l'IA »** par son socle, et en particulier : sanctions et punitions (préparer le **courrier** d'une décision déjà prise par une personne reste possible, N3), décisions d'orientation, d'admission, d'affectation, de bourse ; notes, moyennes, appréciations (Q-IA12) ; suppression, fusion, radiation (l'assistant peut **lire la simulation** et la commenter) ; droits, rôles, sécurité ; exports nominatifs ; tout ce qui touche la santé, le suivi social, la protection de l'enfance ; tout ce qui touche un autre établissement.

### 11.4 Preuve de l'origine [R]

Toute écriture issue d'une carte porte `origine = assistant:<brouillon_id>` dans le journal métier, à côté de l'acteur ; l'écran de l'objet montre « préparé avec l'assistant, confirmé par Mme X » aux personnes qui voient le journal de l'objet.

---

## 12. Rédiger, traduire, résumer

### 12.1 Rédaction [R]

- **Sur le texte de la personne** : corriger, reformuler, raccourcir, rendre plus courtois, plus ferme, plus simple ; pas de données élève envoyées (C0).
- **À partir d'un fil** (16) : réponse préparée, destinataires = participants du fil, gabarit du socle si la nature en a un ; **brouillon du 16**, que la personne relit et envoie.
- **À partir de faits** (courrier de convocation d'une décision prise, compte rendu de réunion à partir de notes, circulaire de sortie à partir du projet du 42) : la trame vient du socle (D-023 : 3 ou 4 trames), l'assistant remplit les parties libres encadrées.
- **Mention** « préparé avec l'assistant » sur le brouillon ; retirable par l'auteur qui endosse le texte (Q-IA10) ; **conservée** dans le journal de l'objet.

### 12.2 Traduction [R]

- Langues proposées : celles déclarées par les familles (socle 4) et une liste courte par défaut (arabe, anglais, espagnol, portugais, turc, roumain, ukrainien, chinois) [H : à ajuster à la population de l'établissement, S-3004].
- Toujours marquée « traduction automatique » ; texte français joint ; jamais pour un document à valeur juridique sans le français (convocation, décision) ; jamais pour une urgence non relue (NTF-48).

### 12.3 Résumé [R]

- **D'élève** (`eleve.resumer`) : blocs par domaine, période choisie, sources, « non consulté » ; jamais stocké pour autrui (cœur §4.7).
- **De fil** : qui a écrit quoi et quand, questions sans réponse, engagements pris (« vous avez promis de rappeler jeudi ») ; le texte des familles reste **donnée non fiable** encadrée.
- **De classe** avant conseil (PP) : par élève, seulement les matières et les élèves de sa portée (D-022, fenêtre du conseil D-032).
- **D'une semaine** (direction) : agrégats déterministes (absences, incidents, à-faire en retard) formulés ; aucun nom au-delà de ce que l'écran de pilotage montre.

---

## 13. Les surfaces

### 13.1 Panneau de l'assistant (partout, web) [R]

Bouton en haut à droite et raccourci clavier (Ctrl/⌘ + J, distinct de la palette Ctrl/⌘ + K du 29) ; panneau latéral de 400 px, plein écran sous 768 px ; le contexte de la page est affiché et retirable (« sur la fiche de Léa D. — retirer ») ; historique des conversations ; accessibilité : rôle `complementary`, annonce des nouvelles réponses (`aria-live="polite"`), navigation au clavier dans les cartes, contraste AA, aucune information portée par la couleur seule.

### 13.2 Barre d'accueil (« chat first », NXB-10) [R]

En tête de « Aujourd'hui » (29) : un champ « Que voulez-vous faire ? » et 3 suggestions tirées de l'à-faire et de l'heure (« Faire l'appel de 10 h », « 4 justificatifs à traiter », « Préparer le conseil de jeudi ») ; même moteur que le panneau. Désactivable par la personne (préférence) et absent si l'assistant est désactivé.

### 13.3 Fiche élève [R]

Bouton « Résumer » et « Qu'est-ce qui a changé depuis… » ; la réponse s'ouvre dans le panneau, contexte = l'élève ; jamais de résumé affiché automatiquement à l'ouverture de la fiche (coût, et parce que lire la fiche est un acte tracé : le résumé l'est aussi).

### 13.4 Fil de messagerie (16) [R]

Bouton « Préparer une réponse » et « Résumer le fil » ; alerte d'injection affichée sur le brouillon si le message reçu contient des instructions adressées à un assistant (cœur §8.2 point 4) ; jamais dans un fil gelé ou signalé.

### 13.5 Conseil de classe (23) [R]

Pendant la préparation : « synthèse de la classe », « élèves dont la situation a changé » ; pendant la séance : **rien de généré à l'écran projeté** (mode projection du 29) ; le secrétaire du conseil peut faire préparer le **procès-verbal** à partir des décisions saisies (trame du 23), relu.

### 13.6 Écrans de saisie [R]

Pas d'assistant intrusif dans les grilles de saisie (notes, appel) : seul un bouton « dicter » sur mobile (§16) et « remplir à partir d'un texte » quand le socle l'a déclaré (justificatifs reçus, liste d'élèves collée depuis un tableur).

---

## 14. Usages par profil

Les usages du cœur (`IA_COEUR.md` §13) sont **repris tels quels** ; ce tableau ne liste que ce que le socle 30 ajoute (conversation, enchaînements, surfaces) et la date à laquelle chaque usage devient possible (le socle qui fournit les capacités doit être codé et décrit).

| Profil | Usage ajouté | Tâche | Socles nécessaires |
|---|---|---|---|
| Secrétariat | « Qu'est-ce qui bloque les dossiers de 2nde ? » → liste triée + plan « relancer les 14 dont il ne manque qu'une pièce » | `liste.filtrer`, `plan.preparer` | 3, 7, 16 |
| Secrétariat | Préparer un certificat demandé par un parent dans un fil, joindre le PDF du 7 au brouillon de réponse | `plan.preparer` (certificat puis réponse) | 7, 16 |
| Secrétariat | « Combien d'internes en 1re garçons cette semaine ? » | `question.frequente` | 41 |
| Direction | « Qu'est-ce qui a changé cette semaine ? » (agrégats) | `etablissement.quoi_de_neuf` | 12, 13, 29, 47 |
| Direction | Préparer l'ordre du jour d'un conseil pédagogique à partir des demandes reçues dans sa boîte | `fil.resumer`, `message.preparer` | 16, 46 |
| Direction | Relire une règle du 31 proposée par la CPE : « explique-moi ce qu'elle aurait fait le mois dernier » | `regle.expliquer` | 31 |
| CPE | « Justifie les absences dont les parents ont donné un motif ce matin » (carte de lot) | `action.preparer` | 12, 16 |
| CPE | « Prépare la convocation de la famille de Tom pour la commission éducative » (décision prise) | `action.preparer` N3 | 13, 7 |
| CPE | « Crée une règle : 3 retards dans la semaine → tâche pour le PP » | `regle.rediger` | 31 |
| AED | « Où est Tom à 10 h ? » (emploi du temps, sortie, infirmerie **sans motif** : « à l'infirmerie depuis 9 h 40 » seulement si le 34 déclare la présence en C1) | `question.frequente` | 10, 14, 34 |
| Accueil | Phrase au téléphone → carte « signalement d'absence » transmise à la vie scolaire | `action.preparer` | 12, 14 |
| Professeur | « Qui n'a pas rendu le devoir de jeudi ? » puis « prépare un message à leurs familles » (plan, N3) | `liste.filtrer`, `plan.preparer` | 20, 16 |
| Professeur | « Réserve la salle informatique jeudi de 10 h à 11 h » | `action.preparer` N1 | 19 / 44 |
| Professeur | « Mets dans le cahier de textes : contrôle chapitre 4 mardi » | `action.preparer` N1 | 20 |
| PP | « Ce qui a changé depuis le dernier conseil pour la 3e A » | `classe.quoi_de_neuf` | 12, 13, 21, 23 |
| Comptabilité | « Prépare les relances de niveau 1 de novembre » → une carte par foyer, gabarit du 25 | `action.preparer` N3 | 24, 25 |
| Intendance, agents | « Qui mange à la cantine jeudi ? » (effectif par service, sans liste de régimes motivés) | `question.frequente` | 40 |
| Documentaliste, personnels techniques | « Quelle est la procédure en cas de fuite d'eau ? » | `etablissement.documentation` | base documentaire |
| Tous les personnels | « Mon emploi du temps de demain, mes remplacements », « ma demande de congé », « mes à-faire » | `question.frequente`, `a_faire.expliquer` | 10, 15, 29 |
| Responsable légal | §15 | | |
| Élève de lycée | §15 | | |

---

## 15. Familles et élèves

### 15.1 Conditions d'ouverture (toutes requises) [R]

1. **IA active** dans l'établissement et **fournisseur de classe standard contractualisé** actif (§17).
2. **AIPD** marquée « faite » avec sa date et la personne qui l'atteste (`ia.parametrer`), et revue annuelle.
3. **Notice d'information** publiée (27) dans une version adaptée aux familles et, pour les élèves, **adaptée à l'âge** (langage simple, « ce que l'assistant fait, ce qu'il ne fait pas, qui peut lire tes questions : personne ») [V : RGPD art. 12 ; recommandations CNIL sur les mineurs, §5].
4. **Information du conseil d'administration** (public) ou de l'instance équivalente (privé) [H] : une case à cocher datée, pas un blocage technique.
5. **Public** : lecture juridique du cadre de référence du numérique (arrêté du 7 septembre 2026, exigence relative à l'IA générative, §5) faite et notée ; à défaut, **fermé** (R-CH26).
6. Activation par profil (responsables, élèves de lycée majeurs, élèves de lycée mineurs) **séparée**.

### 15.2 Responsables légaux : ce qui est ouvert [R]

| Tâche | Exemple | Limites |
|---|---|---|
| `a_faire.expliquer` | « que dois-je faire pour Léa cette semaine ? » | à-faire du 29 de ce responsable |
| `document.expliquer` | « je ne comprends pas cette facture / ce bulletin / ce courrier » | documents que ce responsable peut ouvrir ; **jamais** d'interprétation d'une appréciation (« votre fille est en difficulté ») au-delà du texte |
| `texte.traduire` | « traduis cet avis en arabe » | marqué ; français joint |
| `message.preparer` | « aide-moi à écrire au professeur principal que Léa sera absente mardi » | **c'est le parent qui envoie**, depuis son fil du 16 ; destinataires permis par le 16 |
| `action.preparer` (commandes famille) | « justifie l'absence de Léa d'hier : rendez-vous chez le dentiste » | carte du 12 (N2) ; le parent confirme |
| `etablissement.documentation` | « à quelle heure ouvre la grille ? », « comment payer la cantine ? » | base documentaire au public « familles » |
| `naviguer` | « où sont les notes ? » | destinations permises |

**Fermé aux familles** : résumé d'élève par le modèle (le parent a les écrans ; un résumé ajouterait une interprétation), questions sur d'autres élèves (refus sans révéler l'existence), tout ce qui touche un autre foyer, la santé (y compris les pastilles), la discipline au-delà des documents notifiés, la comparaison avec la classe.

### 15.3 Parents séparés, situations protégées [R]

- L'assistant d'un parent ne voit **que** ce que voit ce parent (lien actif, droits cochés, foyer, D-014) ; il ne dit jamais « l'autre parent a déjà répondu » si ce n'est pas visible pour lui.
- Un responsable **privé de certains droits** (socle 4 : retrait d'autorité, interdiction de contact) : l'assistant applique les mêmes refus que l'écran, avec le même message neutre (« cette information n'est pas disponible dans votre espace »), **jamais la raison**.
- Adresse protégée, confidentialité renforcée (C4) : jamais dans un contexte ; refus neutre.
- Tiers (grand-parent, assistant familial) : aucun assistant en V1 (S-3007).

### 15.4 Élèves [R]

| Population | V1 | Pourquoi |
|---|---|---|
| Collège (mineurs, < 15 ans pour la plupart) | **fermé** | public jeune, cadre ministériel prudent [V/NV §5], aucune tâche dont le gain justifie le risque ; reconsidéré après un an d'usage au lycée |
| Lycée, élève mineur | **fermé par défaut**, ouvrable par l'établissement | tâches : son emploi du temps et ses changements, ses absences à justifier (lycéen qui justifie lui-même selon le règlement), ses travaux à rendre (lecture), procédures (stage, Parcoursup : documentation), reformuler un message qu'il envoie lui-même ; **opposition** possible par l'élève et par ses responsables (18, `outils_ia_eleve`) |
| Lycée, élève majeur | idem, sans opposition des parents | ses propres données ; mêmes tâches qu'un responsable sur lui-même |

**Jamais pour un élève** : rédiger un devoir, résoudre un exercice, produire une rédaction ou un exposé (choix pédagogique hors socle, IA_COEUR §9.4) — l'assistant répond « je ne fais pas le travail à ta place ; ton professeur a laissé ces consignes : … » ; conversation sur la santé, la détresse, le harcèlement, la sexualité, la religion, la famille : **orientation sans analyse** (R-CH33) — « parles-en à la CPE, à l'infirmière ou au psychologue de l'établissement ; tu peux aussi appeler le 3018 (harcèlement en ligne), le 3020 (harcèlement), le 119 (enfance en danger) » [numéros à vérifier, §5] — sans enregistrer la question au-delà de la conversation, sans prévenir qui que ce soit (ce n'est pas un canal de signalement : le signalement est un écran du 36, conçu pour cela).

### 15.5 Ce que la famille et l'élève voient de l'assistant

- Mention permanente « vous échangez avec un assistant automatique » (AI Act art. 50 [V §5]).
- Lien « ce que fait l'assistant » (notice), « supprimer mes conversations », « ne plus utiliser l'assistant » (préférence), « demander qu'aucune donnée de mon enfant ne soit traitée par l'assistant » (hors IA, Q-IA6, effet sur **tous** les profils, personnels compris).

---

## 16. Mobile et voix

- **Mêmes routes**, jeton Bearer (D-011) ; cartes rendues en natif (SwiftUI, Compose) avec la confirmation **dans l'interface native** ; N3 = biométrie de l'appareil liée à la session récente (cœur §4.4).
- **Dictée sur l'appareil** (Q-IA11) : reconnaissance vocale du système, **hors ligne si possible** ; aucun son envoyé à ScolIA ni à un fournisseur ; le texte reconnu est affiché avant envoi.
- **Appel dicté** (prof) : « tous présents sauf Léa, Tom en retard » → carte du 12 ; les prénoms sont résolus **dans la liste de l'appel ouvert** (portée minimale) ; un prénom inconnu ou ambigu → choix.
- **Hors ligne** : pas d'assistant (les cartes exigent le serveur) ; l'appel hors ligne du 12 reste disponible.
- **Notifications** : « 2 cartes à confirmer » seulement si la personne l'a demandé ; jamais le contenu.

---

## 17. Réglages de l'établissement (`ia.parametrer`)

Peu de réglages (D-023), chacun justifié ; signalés au §30.3.

| Réglage | Défaut | Bornes |
|---|---|---|
| Assistant actif | **non** (activation explicite après AIPD et contrat) | — |
| Fournisseur (classe standard) | aucun ; choix dans la liste blanche ScolIA | jamais hors liste |
| Tâches ouvertes par profil | personnels : tâches sans modèle + rédaction + résumé ; familles, élèves : aucune | une tâche fermée par ScolIA (appréciations) ne s'ouvre pas ici |
| Niveau de confirmation | celui du socle | **monter** seulement (cœur §6.2) |
| Quotas | §19 | plafond mensuel obligatoire |
| Durée de conservation des conversations | 30 jours | 0 à 30 |
| Ouverture familles, élèves de lycée | non | conditions du §15.1 |
| Langues de traduction | liste par défaut | ajout de langues |
| Mention « préparé avec l'assistant » retirable | oui | non = toujours affichée |
| Base documentaire : valideurs | direction | — |

Chaque modification est journalisée, effective immédiatement, et notifiée aux administrateurs ; la désactivation est **instantanée** (coupe-circuit, AST-221) et n'efface rien.

---

## 18. Qualité et évaluation

- **Jeux d'évaluation du socle 30** (en plus de ceux de chaque socle) : compréhension (300 demandes types par profil, attendu = tâche et paramètres), résolution de noms (homonymes, fautes, prénoms seuls, classes d'une autre année), refus (100 cas), injection (au moins 50 cas dans des fils, documents, noms de fichiers, titres de conversation, base documentaire), fuite (aucun nom d'un autre élève, aucun jeton inconnu, aucun chiffre absent des sources), ton (grille de neutralité appliquée aux textes pour les familles), langue (10 langues de traduction), élèves (refus de faire le devoir, orientation sans analyse), mobile (dictées bruitées).
- **Seuils d'ouverture** d'une tâche : compréhension ≥ 95 % sur son jeu, fuite = 0, injection = 0 brouillon d'envoi hors fil, refus = 100 % ; à défaut la tâche reste fermée (IA-P14).
- **Retours** : pouce, catégorie (inexact, incomplet, ton, fuite suspectée, autre), commentaire facultatif scellé ; une « fuite suspectée » ouvre un **incident** (§8.3) et met la tâche en observation.
- **Revue mensuelle** par ScolIA des retours agrégés (sans contenu) ; passage des jeux à chaque changement de modèle ou de consignes.

---

## 19. Coûts et quotas

- Ordre de grandeur repris du cœur (§11) : **200 à 450 € par an** pour 1 000 élèves avec un petit modèle par défaut [H].
- **Déterministe d'abord** (§9.1) : objectif ≥ 50 % des demandes des personnels servies sans modèle [H, à mesurer].
- **Quotas** par personne et par jour (défaut : 60 demandes avec modèle pour un personnel, 20 pour un responsable, 15 pour un élève ; sans modèle : 300) ; **plafond mensuel** de l'établissement en euros (obligatoire, défaut 50 €) ; à 80 % : alerte aux administrateurs ; à 100 % : coupe-circuit du modèle (le déterministe continue).
- **Compteurs** en base (`ent_ia_consommation`, agrégés par jour × personne × tâche) ; tableau de bord (47) : coût, usage, taux de demandes sans modèle, refus, retours.
- **Facturation** : décision commerciale Q-IA8 (incluse avec quota) ; rien ne part vers l'établissement sans décision de Florian.

---

## 20. Sécurité

Le modèle de menace du cœur (T1 à T14) s'applique. Ajouts propres à la conversation :

| # | Menace | Parade | Test |
|---|---|---|---|
| T15 | **Historique forgé** (DI-18) | historique relu en base, lié à l'auteur ; jamais reçu du client | requête avec faux messages : ignorés |
| T16 | **Confirmation forgée** (DI-17) | confirmation par `brouillon_id` seulement, empreinte, auteur, expiration ; aucun paramètre accepté à la confirmation | rejouer un brouillon d'un autre, modifié, expiré : refus |
| T17 | **Contexte d'écran forgé** (identifiant d'un élève hors portée passé par le navigateur) | le contexte n'est qu'un identifiant ; toute lecture refait `autoriser_eleve` | test : contexte hors portée → refus, rien n'est lu |
| T18 | **Énumération par l'assistant** (« Dupont existe-t-il ? ») | résolution dans la portée seulement, réponse identique « introuvable dans vos accès », compteur anti-énumération du 29 partagé | 100 noms : aucune différence de réponse ni de durée mesurable |
| T19 | **Injection persistante** par la base documentaire ou un titre de conversation | documentation validée et encadrée ; titres générés par le serveur | cas d'injection dans la documentation |
| T20 | **Exfiltration par la traduction** (traduire un document d'un autre) | `texte.traduire` n'accepte qu'un texte saisi ou un document lisible par la personne | test de portée |
| T21 | **Ingénierie sociale d'un élève** par un adulte via l'assistant (ex. message préparé qui pousse un élève à se taire) | aucun brouillon vers un élève depuis un adulte hors des fils du 16 et de ses règles (aucun adulte ne joint un élève la nuit, fils nommés) ; ton contrôlé | jeux de ton |
| T22 | **Lecture des conversations par la hiérarchie** | §8.3, double validation, information de l'auteur | test de droits : direction refusée |
| T23 | **Attaque par coût** (boucle de demandes, gros documents) | quotas, budget par demande, taille maximale de document (50 pages), plafond mensuel | tests de quota |
| T24 | **Fuite par les journaux applicatifs** | aucune demande ni réponse dans les journaux (`tracing`), seulement identifiants et tailles (E14) | grep des journaux en recette |

Revues : `security-review` sur CH1–CH4, CH8 (fournisseurs), CH12 (familles) ; `review-sonnet` sur chaque nouvelle capacité C2+.

---

## 21. Droits et minimisation

### 21.1 Actions D-017 (catalogue) [R]

| Action | Portée | Modèle par défaut |
|---|---|---|
| `ia.utiliser` (cœur) | moi | tout le personnel si l'IA est active |
| `ia.familles.utiliser` | moi | responsables, si ouvert (§15) |
| `ia.eleves.utiliser` | moi | élèves de lycée, si ouvert |
| `ia.parametrer` (cœur) | établissement | administrateur |
| `ia.journal.lire` (cœur) | établissement | administrateur, direction |
| `ia.documentation.valider` | établissement | direction |
| `ia.incident.ouvrir` | établissement | administrateur |
| `ia.incident.valider` | établissement | direction (et le DPO s'il a un compte) — **jamais la même personne que celle qui ouvre** |
| `ia.exclusion.poser` | élève | secrétariat, direction (à la demande d'une famille), et la famille elle-même dans son espace |
| `ia.donnees_sensibles` (cœur) | moi | **aucun profil, et non attribuable** tant que Q-IA5 n'est pas tranchée (R-CH6) |

### 21.2 Ce que voit chacun

- **La personne** : ses conversations, ses brouillons, ses préférences, son journal IA (« ce que l'assistant a lu pour moi »), ses consommations.
- **Administrateur, direction** (`ia.journal.lire`) : journal sans contenu (qui, quand, quelle tâche, quelles capacités, combien de données, quel coût, quelle décision sur les cartes) ; **jamais le contenu**.
- **Famille** (extrait d'accès, 27) : les consultations de la fiche de son enfant, par canal (écran ou assistant), comme pour toute consultation tracée.
- **ScolIA (exploitation)** : journal sans contenu sur ticket ; agrégats.

### 21.3 Minimisation propre au 30 [R]

- Les consignes système ne contiennent **aucune** donnée personnelle (DI-22) : ni nom de l'utilisateur, ni à-faire, ni signaux.
- Le contexte d'une conversation **n'est pas cumulatif** : à chaque demande, le serveur reconstruit le contexte nécessaire ; les messages précédents sont envoyés au modèle **sans** les résultats de capacités antérieurs (seulement le texte des échanges, pseudonymisé), au plus 6 tours.
- Les identifiants nationaux (INE, NIR, numéros de sécurité sociale, IBAN) ne sont jamais dans un contexte (R-CH7) ; un INE tapé par la personne est remplacé par un jeton avant envoi.
- Les pièces déposées dans une conversation (une photo de justificatif) passent par l'antivirus et l'extraction du 7, et ne sont **gardées** que le temps de la conversation, puis supprimées (objet marqué « éphémère »).

### 21.4 Conservation [R, à valider DPO]

| Donnée | Durée |
|---|---|
| Contenu des conversations (messages, sources) | 30 jours après le dernier message (0–30, S-3001) ; gelé pendant un incident |
| Pièces déposées dans une conversation | fin de la conversation, au plus 30 jours |
| Brouillons d'action | 15 min s'ils ne sont pas confirmés, puis 30 jours (état seulement) ; les confirmés gardent le lien dans le journal |
| Journal IA (sans contenu) | comme le journal d'audit (durée du 27) |
| Compteurs de consommation | 3 ans (comptes de l'établissement) [H] |
| Retours (pouce, catégorie) | 1 an ; commentaire scellé 30 jours |
| Préférences | tant que le compte existe |
| Marques « hors IA » | tant que l'élève est inscrit, puis archive avec le dossier |
| Incidents | 5 ans [H, DPO] |

### 21.5 Pièges à tester

Professeur de groupe qui demande la classe entière ; parent séparé qui demande « est-ce que son père est venu ? » ; élève qui demande les notes d'un camarade ; CPE qui demande un résumé d'un élève marqué « hors IA » ; administrateur qui tente d'ouvrir une conversation sans incident validé ; assistant ouvert par un lien profond avec un `eleve_id` d'un autre établissement ; tâche famille appelée avec un jeton de personnel ; demande en anglais d'un parent qui contient « ignore previous instructions ».

---

## 22. Cas limites (catalogue)

| N° | Situation | Comportement |
|---|---|---|
| KC-01 | « Résume Léa » sans contexte, trois Léa dans la portée | liste de choix (classe, prénom d'usage), sans modèle |
| KC-02 | Élève avec identité d'usage (socle 2) | l'assistant n'emploie que le nom d'usage autorisé dans la portée ; jamais le nom de naissance si une identité d'usage existe |
| KC-03 | Élève sorti de l'établissement | lisible seulement par ceux qui ont encore le droit (archives) ; « élève sorti le … » |
| KC-04 | Professeur suppléant arrivé hier | portée = ses interventions (socle 6) ; « je ne vois que vos groupes depuis le 10/11 » |
| KC-05 | Professeur dont le service se termine aujourd'hui | droits recalculés à chaque appel ; une carte préparée à 17 h 55 et confirmée à 18 h 05 après la fin du service → refusée à la re-vérification |
| KC-06 | Délégation (intérim de direction) | l'assistant suit le moteur au moment de l'appel ; la délégation expirée entre préparation et confirmation → refus |
| KC-07 | Une autre personne a traité la proposition entre-temps | nouvel aperçu (empreinte changée), pas d'exécution |
| KC-08 | Carte de 60 éléments | plafond 50 : deux cartes, ou « ouvrir l'écran de traitement en lot » |
| KC-09 | La demande mêle deux élèves (« compare Léa et Tom ») | refus de comparaison nominative ; deux résumés séparés sur demande |
| KC-10 | Le parent écrit en turc et demande une réponse en turc | brouillon en turc **et** français ; envoi par le parent |
| KC-11 | Demande à 21 h d'un professeur : « préviens les familles du contrôle demain » | brouillon du 16 ; remise à 7 h (plage calme), affichée sur la carte |
| KC-12 | Fournisseur tombé au milieu d'une synthèse | réponse « indisponible », rien de partiel présenté comme complet ; le déterministe reste |
| KC-13 | Plafond mensuel atteint le 25 | coupe-circuit du modèle ; bandeau ; administrateurs avertis à 80 % |
| KC-14 | Personne avec plusieurs casquettes (prof et parent d'élève dans l'établissement) | l'assistant suit la **casquette active** (29 §11.2) ; ses conversations de parent et de professeur sont séparées |
| KC-15 | Personne dans deux établissements | assistants séparés, conversations séparées, quotas séparés (IA-P8) ; aucun « que se passe-t-il dans mes deux établissements » |
| KC-16 | Élève qui devient majeur pendant une conversation | droits recalculés ; la tâche famille de ses parents perd l'accès selon le socle 2 |
| KC-17 | Élève marqué « hors IA » dans une liste filtrée | il apparaît dans la liste servie par le socle (lecture déterministe) mais aucune de ses données n'est envoyée au modèle ; une synthèse de classe dit « 1 élève non traité par l'assistant » |
| KC-18 | Question sur la santé par un personnel qui a le droit à l'écran (infirmière) | refus « jamais lue par l'assistant », lien vers l'écran |
| KC-19 | Lien externe dans un message de parent | jamais rendu cliquable dans une réponse ; l'URL apparaît en texte avec avertissement |
| KC-20 | Demande d'export (« fais-moi un tableau Excel des absences ») | renvoi vers l'export du socle propriétaire (qui a ses droits D-022 et sa double validation) ; jamais un fichier produit par l'assistant |
| KC-21 | Demande en dehors de l'école (« écris-moi une lettre de motivation personnelle ») | `texte.reformuler` sur le texte de la personne permis ; pas de génération libre sans données (tâche `texte.rediger_libre` fermée par défaut, S-3006) |
| KC-22 | Élève qui demande « fais mon exposé » | refus pédagogique (§15.4), consignes du professeur citées |
| KC-23 | Élève qui écrit « je veux mourir » | orientation immédiate, numéros d'urgence (3114 [NV]), « parle à un adulte de l'établissement maintenant » ; **pas** d'analyse, **pas** de signalement automatique ; la conversation n'est lue par personne (R-CH33) ; question ouverte à Florian et au DPO (Q-CH9) |
| KC-24 | Conversation gelée par un incident et effacement demandé par l'auteur | effacement différé jusqu'à la clôture ; l'auteur est informé |
| KC-25 | Bascule d'année (socle 5) pendant une conversation | contexte reconstruit sur l'année active ; les cartes en attente expirent |
| KC-26 | Restauration de la base (PRA) | les brouillons et la file de tâches sont **neutralisés** (E9) : aucune carte rejouée ; les conversations restaurées gardent leur date d'effacement |
| KC-27 | Changement de modèle par ScolIA | jeux d'évaluation passés avant ; version des consignes dans le journal ; annonce aux administrateurs |
| KC-28 | Demande vocale mal reconnue (« Léa » → « Léo ») | carte de l'appel montre les prénoms ; correction avant confirmation |
| KC-29 | Personne qui colle 40 pages dans la demande | taille maximale (20 000 caractères) ; proposer de déposer le document (7) |
| KC-30 | Établissement public, IA générative non ouverte | assistant **sans modèle** seulement (déterministe, navigation, cartes depuis listes) ; tâches de rédaction absentes |

---

## 23. Actions métier (AST-100 à AST-262)

Colonnes : acteur et droit (§21.1 ; « P » = toute personne ayant `ia.utiliser`, « F » = responsable avec `ia.familles.utiliser`, « E » = élève avec `ia.eleves.utiliser`, « ADM » = `ia.parametrer`) ; niveau (N0 lecture, N1–N3 écriture) ; événement émis (outbox) ; lot.

### 23.1 Converser (AST-100 à AST-119)

| N° | Action | Acteur | Règles | Niveau | Événement | Lot |
|---|---|---|---|---|---|---|
| AST-100 | Ouvrir le panneau de l'assistant (bouton, Ctrl/⌘ + J) | P, F, E | contexte de la page proposé | N0 | — | CH5 |
| AST-101 | Poser une demande écrite | P, F, E | §9.1 ; ≤ 20 000 caractères | N0 | `ia.demande_traitee` | CH5 |
| AST-102 | Poser une demande dictée (mobile, reconnaissance sur l'appareil) | P, F, E | texte affiché avant envoi | N0 | idem | CH9 |
| AST-103 | Retirer ou changer le contexte d'écran | P, F, E | changer d'élève = nouvelle conversation | N0 | — | CH5 |
| AST-104 | Répondre à une question de précision (choix, date, créneau, pièce) | P, F, E | réponse = identifiant | N0 | — | CH6 |
| AST-105 | Interrompre une réponse en cours | P, F, E | coût compté jusqu'à l'arrêt | N0 | — | CH5 |
| AST-106 | Régénérer une rédaction (autre formulation) | P, F | compte dans le quota ; même sources | N0 | — | CH7 |
| AST-107 | Copier une réponse (avec ou sans sources) | P, F, E | la copie porte « préparé avec l'assistant » si l'établissement l'impose | N0 | — | CH5 |
| AST-108 | Ouvrir une source citée | P, F, E | ouvre l'écran du socle, contrôle de droits refait | N0 | — | CH5 |
| AST-109 | Ouvrir « la liste complète » d'une réponse tronquée | P | même filtre sur l'écran du socle | N0 | — | CH6 |
| AST-110 | Voir « ce que l'assistant a lu pour cette réponse » (capacités, volumes, masquages) | P, F, E | issu du journal | N0 | — | CH5 |
| AST-111 | Lister ses conversations (50 dernières), chercher dans ses titres | P, F, E | — | N0 | — | CH5 |
| AST-112 | Renommer une conversation | P, F, E | titre ≤ 80 caractères | N1 | — | CH5 |
| AST-113 | Épingler une conversation | P, F, E | 5 au plus | N1 | — | CH5 |
| AST-114 | Supprimer une conversation (immédiat) | P, F, E | journal sans contenu conservé ; différé si gel d'incident | N1 | `ia.conversation_supprimee` | CH5 |
| AST-115 | Supprimer toutes ses conversations | P, F, E | idem | N1 | idem | CH5 |
| AST-116 | Reprendre une conversation | P, F, E | contexte **reconstruit** (droits actuels) | N0 | — | CH5 |
| AST-117 | Déposer une pièce dans la conversation (photo d'un justificatif, PDF) | P, F, E | antivirus et extraction du 7 ; éphémère ; ≤ 50 pages | N1 | — | CH7 |
| AST-118 | Utiliser une suggestion (barre d'accueil, panneau) | P, F, E | suggestions tirées de l'à-faire et de l'heure | N0 | — | CH5 |
| AST-119 | Enregistrer une demande fréquente (« raccourci de demande ») | P | 10 au plus, texte de la personne ; pas de données d'autrui figées | N1 | — | CH6 |

### 23.2 Chercher et comprendre (AST-120 à AST-139)

| N° | Action | Acteur | Règles | Niveau | Événement | Lot |
|---|---|---|---|---|---|---|
| AST-120 | Question fréquente servie sans modèle (effectif, absences du jour, appels non faits, salles libres, mon emploi du temps…) | P | intentions déterministes rebranchées (CH4) | N0 | — | CH4 |
| AST-121 | Trouver un élève en langage naturel (« le petit Martin de 6e ») | P | résolution dans la portée ; choix si ambigu | N0 | — | CH6 |
| AST-122 | Trouver un personnel, un service, une boîte (« qui s'occupe des bourses ? ») | P, F | annuaire du 6 et boîtes du 16 visibles de la personne | N0 | — | CH6 |
| AST-123 | Filtrer une liste en langage naturel (« internes de 2nde sans responsable joignable ») | P | `liste.filtrer` : filtre structuré exécuté par le socle | N0 | — | CH6 |
| AST-124 | Résumer un élève sur une période | P | `eleve.resumer` ; sources et « non consulté » | N0 | — | CH6 |
| AST-125 | « Qu'est-ce qui a changé depuis… » pour un élève | P | `eleve.quoi_de_neuf` | N0 | — | CH6 |
| AST-126 | « Qu'est-ce qui a changé depuis… » pour une classe, un groupe | P | portée au groupe (D-022) | N0 | — | CH6 |
| AST-127 | « Qu'est-ce qui a changé cette semaine dans l'établissement ? » | direction | agrégats seulement | N0 | — | CH6 |
| AST-128 | Résumer un fil de messagerie | P, F | participant du fil ; texte non fiable encadré | N0 | — | CH7 |
| AST-129 | Expliquer un document (facture, bulletin, courrier, convocation) | P, F, E | document lisible par la personne | N0 | — | CH7 |
| AST-130 | Expliquer son à-faire et le prioriser (« que dois-je faire ? ») | P, F, E | `a_faire_de` du 29 | N0 | — | CH6 |
| AST-131 | Aller à un écran (« ouvre les justificatifs à traiter ») | P, F, E | destinations du 29 | N0 | — | CH5 |
| AST-132 | Expliquer une alerte, un masquage, un montant, une règle (« pourquoi ? ») | P, F | chaîne de causalité du socle | N0 | — | CH6 |
| AST-133 | Chercher dans la base documentaire de l'établissement | P, F, E | public du document ; citations | N0 | — | CH7 |
| AST-134 | Demander de l'aide sur ScolIA (« comment justifier une absence ? ») | P, F, E | aide du logiciel | N0 | — | CH5 |
| AST-135 | Comparer deux périodes pour un élève (« ce trimestre et le précédent ») | P | une seule personne, sources | N0 | — | CH6 |
| AST-136 | Lister ses brouillons en attente | P, F, E | — | N0 | — | CH3 |
| AST-137 | Questions sur l'emploi du temps à venir (« ai-je cours vendredi après-midi ? ») | P, E | socle 10, fuseau | N0 | — | CH4 |
| AST-138 | Disponibilité d'une salle, d'un matériel | P | socles 19 / 44 | N0 | — | CH6 |
| AST-139 | Calendrier de l'établissement (« quand sont les conseils ? ») | P, F, E | socle 5 | N0 | — | CH4 |

### 23.3 Préparer et agir (AST-140 à AST-169)

| N° | Action | Acteur | Règles | Niveau | Événement | Lot |
|---|---|---|---|---|---|---|
| AST-140 | Préparer une commande ouverte à partir d'une phrase | P, F, E | `action.preparer` ; carte du socle | N1–N3 | `ia.brouillon_cree` | CH6 |
| AST-141 | Confirmer une carte | auteur du brouillon | re-vérification droits, empreinte, état ; passkey en N3 | — | `ia.brouillon_confirme` + événement du socle | CH3 |
| AST-142 | Modifier une carte (écran normal pré-rempli) | auteur | l'écran reste maître | — | `ia.brouillon_modifie` | CH3 |
| AST-143 | Abandonner une carte | auteur | — | — | `ia.brouillon_abandonne` | CH3 |
| AST-144 | Décocher des éléments d'une carte de lot | auteur | ≤ 50 éléments | — | — | CH3 |
| AST-145 | Annuler une action confirmée pendant son délai d'annulation | auteur | délai déclaré par le socle | — | événement du socle | CH3 |
| AST-146 | Préparer un plan (2 à 5 étapes) | P | aucune écriture ; une carte par étape | N0 | `ia.plan_propose` | CH6 |
| AST-147 | Lancer l'étape suivante d'un plan | P | carte préparée sur l'état réel | selon | — | CH6 |
| AST-148 | Abandonner un plan | P | les étapes faites restent faites | — | — | CH6 |
| AST-149 | Préparer l'appel d'une séance à partir d'une phrase (« comme lundi sauf Léa ») | professeur | socle 12 ; prénoms résolus dans l'appel | N2 | — | CH6 |
| AST-150 | Préparer la justification groupée d'absences à partir des messages reçus | CPE, AED | socles 12, 16 ; ambigus écartés et cités | N2 | — | CH6 |
| AST-151 | Préparer un signalement d'absence à partir d'un appel téléphonique (accueil) | accueil | socles 12, 14 | N2 | — | CH6 |
| AST-152 | Préparer une réservation de salle ou de matériel | P | socles 19, 44 | N1 | — | CH6 |
| AST-153 | Préparer une entrée de cahier de textes, un travail à faire | professeur | socle 20 | N1 | — | CH6 |
| AST-154 | Préparer une demande d'absence ou de congé pour soi | personnel | socle 15 | N2 | — | CH6 |
| AST-155 | Préparer un rendez-vous ou une proposition de créneaux | P, F | socle 19 | N2 | — | CH6 |
| AST-156 | Préparer une relance de pièces manquantes (lot) | secrétariat | socles 3, 7, 16 ; gabarit | N3 | — | CH6 |
| AST-157 | Préparer des relances de paiement adaptées (lot) | comptabilité | socle 25 ; ton sobre ; confidentialité de foyer | N3 | — | CH6 |
| AST-158 | Préparer un courrier d'une décision **déjà prise** (convocation, notification) | direction, CPE | trame du socle ; jamais la décision | N3 | — | CH7 |
| AST-159 | Préparer le procès-verbal d'un conseil à partir des décisions saisies | secrétaire du conseil | trame du 23 | N1 (document brouillon) | — | CH7 |
| AST-160 | Préparer un compte rendu de réunion à partir de notes | P | document du 7 en brouillon | N1 | — | CH7 |
| AST-161 | Préparer une demande de document à une famille (pièce, signature) | secrétariat | socles 7, 18 | N3 | — | CH6 |
| AST-162 | Préparer une justification d'absence par la famille | F | socle 12 | N2 | — | CH12 |
| AST-163 | Préparer une réponse à un formulaire ou un sondage (famille) | F | socle 18 ; la famille relit chaque réponse | N2 | — | CH12 |
| AST-164 | Préparer un rendez-vous avec un professeur (famille) | F | socle 19 | N2 | — | CH12 |
| AST-165 | Remplir une commande à partir d'un texte collé (liste d'élèves d'un tableur) | P | seulement si le socle l'a déclaré ; noms résolus dans la portée | N2 | — | CH6 |
| AST-166 | Remplir à partir d'une pièce déposée (justificatif photographié) | P, F | extraction du 7 → propositions de champs | N2 | — | CH7 |
| AST-167 | Préparer la création d'une règle du 31 (« quand… si… alors… ») | P ayant `automatisations.creer` | brouillon de règle **inactif** | N1 | `regle.brouillon_cree` (31) | CH6 |
| AST-168 | Préparer un rappel personnel (« rappelle-moi jeudi d'appeler Mme X ») | P | rappel personnel du 29 (§8.12) | N1 | — | CH6 |
| AST-169 | Préparer une tâche pour un collègue (« demande à la vie scolaire de vérifier… ») | P | tâche du 29 ou message du 16, destinataire permis | N2 | — | CH6 |

### 23.4 Rédiger et traduire (AST-170 à AST-189)

| N° | Action | Acteur | Règles | Niveau | Événement | Lot |
|---|---|---|---|---|---|---|
| AST-170 | Corriger l'orthographe et la grammaire de son texte | P, F, E | C0 | N0 | — | CH7 |
| AST-171 | Reformuler (plus court, plus courtois, plus ferme, plus simple) | P, F, E | C0 | N0 | — | CH7 |
| AST-172 | Préparer une réponse à un fil | P, F | destinataires = participants ; brouillon du 16 | N3 à l'envoi | — | CH7 |
| AST-173 | Préparer un nouveau message (destinataires choisis à l'écran) | P, F | règles du 16 | N3 à l'envoi | — | CH7 |
| AST-174 | Préparer un message aux familles d'un groupe | professeur | ≤ 30 fils (16 : au-delà, diffusion du 8) | N3 | — | CH7 |
| AST-175 | Préparer une diffusion ou une annonce (texte libre) | direction, vie scolaire | socles 8, 17 ; aperçu du 8 ; quatre yeux selon seuil | N3 | — | CH7 |
| AST-176 | Traduire un texte de la personne | P, F, E | « traduction automatique » | N0 | — | CH7 |
| AST-177 | Traduire un message reçu (famille non francophone) | P, F | texte non fiable encadré | N0 | — | CH7 |
| AST-178 | Proposer la traduction du texte libre d'une diffusion (NTF-48, NTF-130) | auteur de la diffusion | relue dans l'aperçu du 8 ; français joint | N0 | — | CH7 |
| AST-179 | Rédiger la circulaire d'une sortie à partir du projet (42) | organisateur | trame du 42 | N1 | — | CH7 |
| AST-180 | Rédiger le texte d'une affiche ou d'une annonce (17) | P | C0 | N0 | — | CH7 |
| AST-181 | Remplir les parties libres d'un courrier type (D-023) | P | parties encadrées seulement | N1 | — | CH7 |
| AST-182 | Proposer une appréciation à partir des notes **et des remarques du professeur** | professeur | **fermée** jusqu'à Q-IA12 (D-032) | N1 | — | CH7 (fermée) |
| AST-183 | Rédiger un ordre du jour à partir de demandes reçues | direction | — | N1 | — | CH7 |
| AST-184 | Retirer la mention « préparé avec l'assistant » d'un brouillon qu'on endosse | auteur | si l'établissement le permet (S-3005) | N1 | — | CH7 |
| AST-185 | Choisir la langue des réponses de l'assistant | P, F, E | préférence | N1 | — | CH7 |
| AST-186 | Rédiger un texte pour un parent dans sa langue, avec le français joint | P | — | N0 | — | CH7 |
| AST-187 | Simplifier un document administratif pour une famille (« en mots simples ») | F | explication, pas de réécriture du document | N0 | — | CH12 |
| AST-188 | Proposer un titre neutre pour un objet (rendez-vous, message) | P | aucun motif dans le titre | N0 | — | CH7 |
| AST-189 | Préparer un mot d'excuse ou un message d'élève à un professeur | E | l'élève envoie lui-même | N0 | — | CH13 |

### 23.5 Familles et élèves (AST-190 à AST-204)

| N° | Action | Acteur | Règles | Niveau | Événement | Lot |
|---|---|---|---|---|---|---|
| AST-190 | « Que dois-je faire pour mon enfant cette semaine ? » | F | à-faire du responsable | N0 | — | CH12 |
| AST-191 | « Pourquoi mon enfant n'a-t-il pas cours ? » (changement d'emploi du temps) | F, E | socle 10, raisons publiques seulement (jamais « professeur malade ») | N0 | — | CH12 |
| AST-192 | Comprendre une facture ou un échéancier | F | socle 24, son foyer | N0 | — | CH12 |
| AST-193 | Comprendre un bulletin (sans interprétation des appréciations) | F, E majeur | socle 23 | N0 | — | CH12 |
| AST-194 | Trouver à qui écrire (« à qui demander une attestation ? ») | F, E | boîtes du 16 | N0 | — | CH12 |
| AST-195 | Questions pratiques (horaires, cantine, transport) | F, E | base documentaire « familles » | N0 | — | CH12 |
| AST-196 | Choisir l'enfant concerné quand on en a plusieurs | F | sélecteur du 29 | N0 | — | CH12 |
| AST-197 | S'opposer à l'assistant pour son enfant (« hors IA ») | F | effet sur tous les profils | N1 | `ia.exclusion_posee` | CH12 |
| AST-198 | Lever l'opposition « hors IA » | F (qui l'a posée) ou majeur | — | N1 | `ia.exclusion_levee` | CH12 |
| AST-199 | Lire la notice de l'assistant | F, E | version adaptée | N0 | — | CH12 |
| AST-200 | Élève : « mon emploi du temps de demain, mes changements » | E | socle 10 | N0 | — | CH13 |
| AST-201 | Élève : « qu'ai-je à rendre cette semaine ? » | E | socle 20 ; jamais la réponse à l'exercice | N0 | — | CH13 |
| AST-202 | Élève : « comment justifier mon absence ? » (lycée) | E | selon le règlement | N0 | — | CH13 |
| AST-203 | Élève : demande d'aide personnelle (santé, détresse, harcèlement) → orientation sans analyse | E | R-CH33 | N0 | — (aucun) | CH13 |
| AST-204 | Élève : demande de faire un travail scolaire → refus pédagogique et consignes citées | E | §15.4 | N0 | — | CH13 |

### 23.6 Préférences, retours, transparence (AST-205 à AST-219)

| N° | Action | Acteur | Règles | Niveau | Événement | Lot |
|---|---|---|---|---|---|---|
| AST-205 | Régler ses préférences (langue, ton, longueur, format liste ou texte) | P, F, E | colonnes typées | N1 | — | CH5 |
| AST-206 | Masquer la barre d'accueil | P, F, E | — | N1 | — | CH5 |
| AST-207 | Ne plus utiliser l'assistant (préférence) | P, F, E | aucun effet sur les écrans | N1 | — | CH5 |
| AST-208 | Donner un retour (pouce, catégorie, commentaire) | P, F, E | **jamais transmis au fournisseur** | N1 | `ia.retour_donne` | CH10 |
| AST-209 | Signaler une fuite suspectée ou un contenu inapproprié | P, F, E | ouvre un incident ; tâche mise en observation | N1 | `ia.incident_signale` | CH10 |
| AST-210 | Voir son journal IA (« ce que l'assistant a lu pour moi », 90 jours) | P, F, E | sans contenu des autres | N0 | — | CH5 |
| AST-211 | Voir sa consommation et ses quotas | P | — | N0 | — | CH11 |
| AST-212 | Suivre la courte formation à l'assistant (art. 4) | P | à la première utilisation ; attestée | N1 | `ia.formation_suivie` | CH5 |
| AST-213 | Voir l'état de l'assistant (actif, fournisseur, tâches ouvertes) dans « ce que voit ScolIA de moi » | P, F, E | — | N0 | — | CH5 |
| AST-214 | Exporter ses conversations (droit d'accès et portabilité) | P, F, E | format d'échange (D-046) ; 30 jours de contenu au plus | N1 | — | CH11 |
| AST-215 | Être informé quand une de ses conversations est lue pour incident | auteur | sauf interdiction légale | — | — | CH10 |
| AST-216 | Être prévenu d'une carte expirée non confirmée | auteur | dans le panneau seulement | — | — | CH3 |
| AST-217 | Voir pourquoi une tâche est fermée (« pas ouverte dans l'établissement ») | P, F, E | — | N0 | — | CH5 |
| AST-218 | Voir « préparé avec l'assistant, confirmé par Mme X » sur un objet | qui voit le journal de l'objet | §11.4 | N0 | — | CH3 |
| AST-219 | Demander la suppression de son journal IA personnel | P, F, E | refus motivé (journal d'audit) ; contenu déjà effacé | N1 | — | CH11 |

### 23.7 Administration de l'établissement (AST-220 à AST-244)

| N° | Action | Acteur | Règles | Niveau | Événement | Lot |
|---|---|---|---|---|---|---|
| AST-220 | Activer l'assistant (conditions : contrat, AIPD, notice) | ADM | ré-authentification | N3 | `ia.active` | CH8 |
| AST-221 | Désactiver l'assistant immédiatement (coupe-circuit) | ADM, direction | instantané ; rien n'est effacé | N2 | `ia.desactive` | CH1 |
| AST-222 | Choisir le fournisseur de la classe standard dans la liste blanche | ADM | jamais hors liste ; aucun repli | N3 | `ia.fournisseur_change` | CH8 |
| AST-223 | Ouvrir ou fermer une tâche par profil | ADM | tâches fermées par ScolIA non ouvrables | N2 | `ia.tache_ouverte` / `_fermee` | CH8 |
| AST-224 | Monter le niveau de confirmation d'une commande | ADM | jamais descendre | N2 | — | CH3 |
| AST-225 | Fixer les quotas et le plafond mensuel | ADM | plafond obligatoire | N2 | — | CH11 |
| AST-226 | Rendre la formation obligatoire avant usage | ADM | défaut : oui | N1 | — | CH5 |
| AST-227 | Télécharger l'AIPD-cadre pré-remplie et la notice type | ADM, direction | fournies par ScolIA, versionnées | N0 | — | CH8 |
| AST-228 | Attester l'AIPD faite (date, personne, revue annuelle) | direction | condition d'ouverture | N2 | `ia.aipd_attestee` | CH8 |
| AST-229 | Publier la notice d'information (personnels, familles, élèves) | direction | socle 27 | N2 | — | CH8 |
| AST-230 | Ouvrir l'assistant aux familles | direction | §15.1 | N3 | `ia.ouverture_familles` | CH12 |
| AST-231 | Ouvrir l'assistant aux élèves de lycée (majeurs, mineurs séparément) | direction | §15.1 ; public : lecture juridique notée | N3 | `ia.ouverture_eleves` | CH13 |
| AST-232 | Fixer la durée de conservation des conversations (0–30 jours) | ADM | vers le bas seulement | N2 | — | CH5 |
| AST-233 | Fixer les langues de traduction proposées | ADM | — | N1 | — | CH7 |
| AST-234 | Imposer la mention « préparé avec l'assistant » | ADM | — | N1 | — | CH7 |
| AST-235 | Déposer un document dans la base documentaire | P ayant le droit du 7 | brouillon tant que non validé | N1 | — | CH7 |
| AST-236 | Valider, versionner, retirer un document de la base documentaire | `ia.documentation.valider` | public du document | N2 | `ia.documentation_validee` | CH7 |
| AST-237 | Lire le journal IA de l'établissement (sans contenu) | `ia.journal.lire` | filtres : personne, tâche, période, refus, alertes | N0 | — | CH2 |
| AST-238 | Voir le tableau d'usage, de coût, de refus, de retours | ADM, direction | agrégats (47) | N0 | — | CH11 |
| AST-239 | Ouvrir un incident (motif, conversation visée) | `ia.incident.ouvrir` | gèle la conservation | N2 | `ia.incident_ouvert` | CH10 |
| AST-240 | Valider l'accès à une conversation pour incident | `ia.incident.valider` (autre personne) | 7 jours ; journalisé ; auteur informé | N3 | `ia.incident_acces_valide` | CH10 |
| AST-241 | Clore un incident (conclusion codée, actions) | ADM | levée du gel | N2 | `ia.incident_clos` | CH10 |
| AST-242 | Poser ou lever une marque « hors IA » à la demande d'une famille | `ia.exclusion.poser` | pièce de la demande facultative | N1 | `ia.exclusion_posee` / `_levee` | CH4 |
| AST-243 | Exporter le registre du traitement IA (pour le registre de l'établissement) | ADM | socle 27 | N0 | — | CH8 |
| AST-244 | Mettre une tâche en observation (après retours) | ADM | tâche fermée aux profils sauf administrateurs | N2 | — | CH10 |

### 23.8 Plateforme ScolIA et système (AST-245 à AST-262)

| N° | Action | Acteur | Règles | Niveau | Événement | Lot |
|---|---|---|---|---|---|---|
| AST-245 | Déclarer une capacité au registre (code, revue) | développeur ScolIA | tests de garde (cœur §5.4) | — | — | CH1 |
| AST-246 | Ajouter un fournisseur à la liste blanche (après revue contractuelle) | exploitation ScolIA | critères du cœur §10.2 ; décision de Florian | — | — | CH8 |
| AST-247 | Retirer un fournisseur de la liste blanche | exploitation ScolIA | les établissements concernés passent en « sans modèle » et sont prévenus | — | `ia.fournisseur_retire` | CH8 |
| AST-248 | Publier une version de consignes (après passage des jeux) | ScolIA | version au journal | — | — | CH10 |
| AST-249 | Passer les jeux d'évaluation (simulé en CI, réel à la demande) | ScolIA | rapport archivé | — | — | CH10 |
| AST-250 | Rejeter une sortie du modèle invalide (schéma, jeton inconnu, nombre absent des sources, lien externe) | système | réponse de repli | — | alerte au journal | CH4 |
| AST-251 | Détecter des instructions dans une donnée non fiable et l'afficher | système | alerte, pas blocage seul | — | alerte | CH4 |
| AST-252 | Pseudonymiser et dé-pseudonymiser | système | cœur §7.3 | — | — | CH4 |
| AST-253 | Expirer les brouillons non confirmés (15 min) | système | — | — | `ia.brouillon_expire` | CH3 |
| AST-254 | Effacer le contenu des conversations échues | système (tâche planifiée du 31) | gel respecté ; idempotent | — | — | CH5 |
| AST-255 | Effacer les pièces éphémères | système | — | — | — | CH7 |
| AST-256 | Compter les consommations et appliquer quotas et plafond | système | coupe-circuit du modèle à 100 % | — | `ia.plafond_atteint` | CH11 |
| AST-257 | Alerter les administrateurs à 80 % du plafond | système | une fois par mois | — | `ia.plafond_80` | CH11 |
| AST-258 | Exécuter une tâche longue en file (synthèse lourde), droits rechargés à l'exécution | système | arrêt si session révoquée | — | — | CH6 |
| AST-259 | Neutraliser brouillons et file à la restauration (E9) | système | aucune carte rejouée | — | — | CH2 |
| AST-260 | Recalculer l'état d'ouverture quand une condition tombe (AIPD échue, contrat retiré) | système | fermeture automatique **vers le sûr** + tâche aux administrateurs | — | `ia.ouverture_suspendue` | CH8 |
| AST-261 | Servir le copilote historique (`/copilote/ask`) comme façade du registre | système | compatibilité jusqu'au retrait de `ui.rs` | — | — | CH4 |
| AST-262 | Fournisseur nul : répondre sans modèle partout où c'est possible | système | défaut de tous les établissements | — | — | CH4 |

**Total : 163 actions** (AST-100 à AST-262).

---

## 24. Règles transverses (R-CH1 à R-CH40) [R]

| N° | Règle |
|---|---|
| **R-CH1** | Le socle 30 **construit le cœur IA** (lots CH1–CH4) avant toute conversation ; aucune capacité d'un socle n'est ouverte avant que le registre et les tests de garde existent. |
| R-CH2 | Aucune donnée, aucun droit, aucune action ne sont propres à l'assistant ; il est client du registre (IA-P1, IA-P2). |
| R-CH3 | Le déterministe d'abord : une demande servable sans modèle ne passe jamais par le modèle. |
| R-CH4 | Le contexte d'écran est un identifiant ; toute lecture refait le contrôle de droits. |
| R-CH5 | Tout chiffre et toute date d'une réponse viennent d'une capacité ; une réponse qui contient un nombre absent des sources est rejetée. |
| **R-CH6** | **Jamais de données de santé** : la classe C3 est fermée dans le socle 30, `ia.donnees_sensibles` n'est attribuable à aucun profil tant que Q-IA5 n'est pas tranchée par Florian et le DPO ; les pastilles déclarées C1 par leur socle restent permises. |
| R-CH7 | Jamais d'identifiant national (INE, NIR), d'IBAN, d'adresse, de téléphone, d'e-mail dans un contexte envoyé au modèle ; un identifiant tapé est remplacé par un jeton. |
| R-CH8 | Une conversation à contexte d'élève ne porte que sur cet élève ; changer d'élève ouvre une conversation. |
| R-CH9 | Aucune mémoire de faits d'une conversation à l'autre ; seules des préférences typées sont gardées. |
| R-CH10 | Rien de la conversation n'est stocké dans le navigateur au-delà de l'affichage (pas de `localStorage` de contenu). |
| R-CH11 | L'historique est relu en base, jamais reçu du client. |
| **R-CH12** | Une conversation n'est lue que par son auteur ; exception : incident ouvert par une personne, validé par une autre, 7 jours, journalisé, auteur informé. |
| R-CH13 | Aucun message d'erreur du fournisseur n'est montré ; code d'incident neutre. |
| R-CH14 | Aucune demande, réponse ou donnée personnelle dans les journaux applicatifs (E14). |
| R-CH15 | Les consignes système ne contiennent aucune donnée personnelle. |
| R-CH16 | Une « confiance » produite par un modèle ordonne une file, elle ne déclenche jamais une écriture. |
| R-CH17 | Une commande par carte ; un plan prépare l'étape suivante après l'exécution de la précédente, à partir de l'état réel. |
| R-CH18 | La confirmation se fait par l'identifiant du brouillon, par son auteur, avant expiration, avec la même empreinte ; aucun paramètre n'est accepté à la confirmation. |
| R-CH19 | L'interface dit en permanence qu'on échange avec un assistant automatique (AI Act art. 50). |
| R-CH20 | Toute écriture issue d'une carte porte `origine = assistant:<brouillon>` ; l'objet le montre à qui voit son journal. |
| R-CH21 | Fournisseurs : points d'accès sans état seulement, aucun modèle expérimental, aucun repli, liste blanche par établissement et par classe. |
| R-CH22 | Les retours des utilisateurs ne sont jamais transmis au fournisseur. |
| R-CH23 | Un texte destiné à une famille passe toujours par un brouillon relu et un aperçu N3 ; jamais d'envoi depuis l'assistant sans geste. |
| R-CH24 | Une pièce utilisée par l'assistant est un objet déposé par la personne dans la conversation ou un document qu'elle peut lire, vérifié par le serveur. |
| R-CH25 | Les quotas et le plafond mensuel coupent le modèle, jamais l'assistant déterministe ni les écrans. |
| **R-CH26** | Familles et élèves : fermé par défaut ; ouvert seulement si les conditions du §15.1 sont toutes remplies ; dans le public, élèves fermés tant que la lecture juridique de l'arrêté du 7/09/2026 n'est pas faite. |
| R-CH27 | Collégiens : pas d'assistant en V1. |
| R-CH28 | L'assistant d'un responsable ne voit que ce que voit ce responsable (lien, droits, foyer) ; refus neutres, jamais la raison. |
| R-CH29 | Aucune comparaison nominative entre élèves. |
| R-CH30 | Un jugement écrit par un adulte est cité comme tel, jamais repris comme un fait. |
| R-CH31 | Une marque « hors IA » exclut toutes les données de l'élève de tout contexte envoyé à un modèle, pour tous les profils. |
| R-CH32 | Une condition d'ouverture qui tombe ferme automatiquement l'ouverture correspondante. |
| **R-CH33** | Santé, détresse, harcèlement, violences : l'assistant **oriente** vers les personnes et les numéros officiels, sans analyse, sans signalement automatique, sans conservation au-delà de la conversation ; ce n'est pas un canal de signalement. |
| R-CH34 | Aucune inférence d'émotion, d'humeur ou d'état d'une personne (AI Act art. 5 § 1 f). |
| R-CH35 | Aucun détecteur de texte produit par IA dans ScolIA. |
| R-CH36 | L'assistant ne fait pas le travail scolaire d'un élève. |
| R-CH37 | Un résumé n'est jamais stocké pour être servi à une autre personne ; une synthèse « officielle » est un document du socle propriétaire enregistré par une personne. |
| R-CH38 | Une règle du 31 n'appelle jamais un modèle de langage ; l'assistant peut rédiger et expliquer une règle, jamais l'activer. |
| R-CH39 | La plage calme du 16 et du 8 s'applique aux envois préparés ; la carte montre l'heure de remise. |
| R-CH40 | Le support ScolIA ne voit jamais le contenu des conversations. |

---

## 25. Événements et notifications

### 25.1 Émis (outbox, dans la transaction du fait)

`ia.demande_traitee` (sans contenu : tâche, coût, durée ; **non** distribué aux consommateurs métier, sert au 47) · `ia.brouillon_cree`, `_confirme`, `_modifie`, `_abandonne`, `_expire` · `ia.plan_propose` · `ia.conversation_supprimee` · `ia.exclusion_posee`, `_levee` · `ia.active`, `ia.desactive`, `ia.fournisseur_change`, `ia.fournisseur_retire`, `ia.tache_ouverte`, `_fermee` · `ia.aipd_attestee`, `ia.ouverture_familles`, `ia.ouverture_eleves`, `ia.ouverture_suspendue` · `ia.documentation_validee` · `ia.retour_donne`, `ia.incident_signale`, `ia.incident_ouvert`, `ia.incident_acces_valide`, `ia.incident_clos` · `ia.formation_suivie` · `ia.plafond_80`, `ia.plafond_atteint`. Charges utiles : identifiants seulement (D-036 : charge transitoire).

### 25.2 Consommés

`eleve.majeur` (2 : droits recalculés, les responsables perdent l'accès), `eleve.sortie` (2), `lien.retire` (4), `service.termine` (6), `delegation.expiree` (1), `annee.basculee` (5 : brouillons expirés), `objet.supprime` (7), `notifications.gel` (8 : brouillons d'envoi suspendus), `regle.activee` (31 : rien ; l'assistant ne suit pas les règles), `rgpd.effacement` (27 : effacement des conversations et du « hors IA » selon la demande).

### 25.3 Notifications (natures déclarées au socle 8)

`ia.plafond_80` et `ia.plafond_atteint` (administrateurs) · `ia.ouverture_suspendue` (administrateurs, direction) · `ia.incident_acces_valide` (auteur de la conversation) · `ia.fournisseur_retire` (administrateurs). Aucune notification aux familles ni aux élèves, hormis l'information d'incident (exceptionnelle).

---

## 26. Modèle de données cible (brique `assistant` et cœur `ent-ia`) [R]

**Règles appliquées à toutes les tables** (E1–E14, D-036) : `id UUID` généré par l'application (E3) ; `etablissement_id UUID NOT NULL` y compris dans les tables filles (E1) ; `UNIQUE (etablissement_id, id)` et FK **composites** `(etablissement_id, x_id)` (E2) ; `cree_le TIMESTAMPTZ NOT NULL DEFAULT now()`, `modifie_le` sur les tables modifiables (E6) ; RLS standard puis stricte (E13) ; **aucune colonne JSON/JSONB, aucun tableau, aucun BYTEA** (E5, E10) ; textes de contenu **scellés** par `champs_chiffres.rs` avec identifiant de clé (E11) ; codes en `TEXT` + `CHECK` (listes fermées connues du code) ; tables en ajout seul protégées par déclencheur ; registre de classement des tables (E12) mis à jour.

### 26.1 Réglages de l'établissement (cœur, CH1)

| Table | Colonnes principales | Remarques |
|---|---|---|
| `ent_ia_reglage` | `etablissement_id` (PK), `actif`, `conservation_jours` (0–30), `mention_retirable`, `formation_obligatoire`, `plafond_mensuel_centimes`, `quota_personnel_jour`, `quota_famille_jour`, `quota_eleve_jour`, `aipd_attestee_le`, `aipd_attestee_par`, `aipd_revue_avant`, `lecture_juridique_public_le`, `modifie_le`, `modifie_par` | une ligne par établissement ; historique par `ent_journal_modif` |
| `ent_ia_fournisseur_autorise` | `etablissement_id`, `id`, `classe` (`standard`, `sensible`, `local`), `fournisseur_code`, `modele_code`, `actif_du`, `actif_au`, `reference_contrat` | `fournisseur_code` ∈ liste blanche **du code** ; aucune clé ici (secrets d'exploitation) ; exclusion de chevauchement par classe |
| `ent_ia_tache_ouverte` | `etablissement_id`, `id`, `tache_code`, `profil` (`personnel`, `responsable`, `eleve_majeur`, `eleve_mineur`), `ouverte`, `niveau_minimal` (`N1`…`N3`, monter seulement) | une ligne par tâche × profil |
| `ent_ia_ouverture` | `etablissement_id`, `id`, `public` (`responsables`, `eleves_majeurs`, `eleves_mineurs`), `ouverte_le`, `ouverte_par`, `notice_version_id`, `info_instance_le`, `suspendue_le`, `motif_suspension` (code) | conditions §15.1 vérifiées par le code |
| `ent_ia_langue` | `etablissement_id`, `id`, `code_langue` (BCP 47), `ordre` | langues de traduction |

### 26.2 Brouillons d'action (cœur, CH3) — table partagée avec la palette (29) et les règles (31)

| Table | Colonnes principales | Remarques |
|---|---|---|
| `ent_brouillon_action` | `etablissement_id`, `id`, `origine` (`assistant`, `palette`, `regle`, `controle`), `commande_code`, `auteur_user_id` (NULL si origine règle ou contrôle), `destinataire_user_id` ou `destinataire_mission_id` (un seul des deux, `CHECK`), `regle_execution_id` (31, NULL sinon), `conversation_id` (NULL sinon), `niveau`, `empreinte` (SHA-256 des paramètres résolus et de l'état simulé), `apercu_texte_scelle` (rédigé par le socle), `commentaire_ia_scelle`, `etat` (`propose`, `confirme`, `execute`, `abandonne`, `expire`, `refuse_reverification`, `echec`), `expire_le`, `session_creation_id`, `cle_idempotence`, `cree_le`, `modifie_le` | **neutralisée à la restauration** (E9) ; RLS : auteur, ou membres de la mission destinataire |
| `ent_brouillon_parametre` | `etablissement_id`, `id`, `brouillon_id`, `nom`, `type` (`uuid`, `date`, `instant`, `entier`, `decimal`, `texte_court`, `code`, `booleen`), `valeur_uuid`, `valeur_date`, `valeur_instant`, `valeur_entier`, `valeur_decimal`, `valeur_texte_scelle`, `valeur_code`, `valeur_booleen` | **une seule colonne de valeur remplie**, contrôlée par `CHECK` selon `type` (même modèle que les réponses du socle 18, déjà admis) ; les identifiants sont **résolus** (jamais de jeton) |
| `ent_brouillon_element` | `etablissement_id`, `id`, `brouillon_id`, `rang`, `objet_type`, `objet_id`, `retenu` (bool), `motif_ecart` (code), `apercu_ligne_scelle` | éléments décochables d'un lot (≤ 50) |
| `ent_brouillon_evenement` | `etablissement_id`, `id`, `brouillon_id`, `etat_avant`, `etat_apres`, `par_user_id`, `instant`, `motif` (code) | ajout seul |
| `ent_ia_plan` | `etablissement_id`, `id`, `conversation_id`, `auteur_user_id`, `etat` (`propose`, `en_cours`, `termine`, `abandonne`), `cree_le` | |
| `ent_ia_plan_etape` | `etablissement_id`, `id`, `plan_id`, `rang` (1–5), `commande_code`, `libelle_scelle`, `brouillon_id` (NULL tant que non préparée), `etat` | |

### 26.3 Journal IA (cœur, CH2) — ajout seul, sans contenu

| Table | Colonnes principales |
|---|---|
| `ent_ia_journal` | `etablissement_id`, `id`, `acteur_user_id`, `session_id`, `canal` (`web`, `mobile`), `profil`, `tache_code`, `version_consignes`, `classe_traitement`, `fournisseur_code` (NULL si sans modèle), `modele_code`, `avec_modele` (bool), `jetons_entree`, `jetons_sortie`, `cout_millicentimes`, `duree_ms`, `brouillon_id`, `decision` (code), `conversation_id`, `cree_le` |
| `ent_ia_journal_capacite` | `etablissement_id`, `id`, `journal_id`, `rang`, `capacite_code`, `cible_type`, `cible_id`, `resultat` (`servi`, `refuse`, `masque`, `hors_ia`), `volume` |
| `ent_ia_journal_donnee` | `etablissement_id`, `id`, `journal_id`, `type_entite`, `nombre`, `classe_sensibilite` (`C0`–`C2` ; C3/C4 refusés par `CHECK`), `pseudonymise` (bool) |
| `ent_ia_journal_alerte` | `etablissement_id`, `id`, `journal_id`, `alerte` (`injection_suspectee`, `sortie_invalide`, `fuite_bloquee`, `nombre_hors_source`, `quota`, `lien_externe_supprime`) |

### 26.4 Conversations (socle 30, CH5)

| Table | Colonnes principales | Remarques |
|---|---|---|
| `ent_ia_conversation` | `etablissement_id`, `id`, `auteur_user_id`, `casquette` (code du 29), `titre_scelle`, `contexte_type` (`eleve`, `fil`, `classe`, `groupe`, `conseil`, `document`, `aucun`), `contexte_id`, `epinglee`, `dernier_message_le`, `effacer_apres`, `gel_incident_id`, `supprimee_le` | RLS **propriétaire** : seul l'auteur (et l'accès d'incident validé) |
| `ent_ia_message` | `etablissement_id`, `id`, `conversation_id`, `rang`, `role` (`personne`, `assistant`, `systeme_refus`, `precision`, `carte`, `plan`), `texte_scelle`, `tache_code`, `journal_id`, `brouillon_id`, `cree_le` | ajout seul ; effacé en bloc à l'échéance |
| `ent_ia_message_source` | `etablissement_id`, `id`, `message_id`, `rang`, `source_type` (code), `source_id`, `libelle_neutre_scelle`, `statut` (`consulte`, `masque`, `non_consulte`, `hors_ia`) | citations et « non consulté » |
| `ent_ia_precision_attente` | `etablissement_id`, `id`, `conversation_id`, `brouillon_id`, `type` (`choix_unique`, `choix_multiple`, `date`, `creneau`, `piece`, `texte_court`), `expire_le` | une seule par conversation |
| `ent_ia_precision_option` | `etablissement_id`, `id`, `precision_id`, `rang`, `valeur_uuid`, `valeur_date`, `libelle_scelle` | |
| `ent_ia_piece_ephemere` | `etablissement_id`, `id`, `conversation_id`, `objet_id` (`ent_objet`), `supprimer_le` | E5 ; objet supprimé avec la conversation |
| `ent_ia_raccourci_demande` | `etablissement_id`, `id`, `auteur_user_id`, `libelle`, `texte_scelle`, `rang` | ≤ 10 par personne |

### 26.5 Personnes, préférences, retours, incidents (CH5, CH10)

| Table | Colonnes principales | Remarques |
|---|---|---|
| `ent_ia_preference` | `etablissement_id`, `user_id` (PK composite), `langue`, `ton` (`neutre`, `chaleureux`, `concis`), `longueur` (`courte`, `normale`, `detaillee`), `format` (`liste`, `texte`), `barre_accueil` (bool), `utiliser` (bool) | typées, aucune mémoire |
| `ent_ia_formation` | `etablissement_id`, `id`, `user_id`, `version_formation`, `suivie_le` | art. 4 AI Act |
| `ent_ia_exclusion` | `etablissement_id`, `id`, `eleve_id` (FK composite), `posee_le`, `posee_par_user_id`, `demandee_par` (`responsable`, `eleve_majeur`, `etablissement`), `levee_le`, `levee_par_user_id` | lue par la construction du contexte (R-CH31) |
| `ent_ia_retour` | `etablissement_id`, `id`, `journal_id`, `auteur_user_id`, `valeur` (`positif`, `negatif`), `categorie` (`inexact`, `incomplet`, `ton`, `fuite_suspectee`, `autre`), `commentaire_scelle`, `commentaire_effacer_le` | |
| `ent_ia_incident` | `etablissement_id`, `id`, `ouvert_par`, `ouvert_le`, `motif` (code) + `motif_detail_scelle`, `conversation_id`, `valide_par` (≠ `ouvert_par`, `CHECK`), `valide_le`, `acces_jusqu_au`, `auteur_informe_le`, `clos_le`, `conclusion` (code) | |
| `ent_ia_incident_acces` | `etablissement_id`, `id`, `incident_id`, `lecteur_user_id`, `instant` | ajout seul |

### 26.6 Consommation et base documentaire (CH7, CH11)

| Table | Colonnes principales | Remarques |
|---|---|---|
| `ent_ia_consommation` | `etablissement_id`, `id`, `jour`, `user_id`, `tache_code`, `demandes`, `demandes_avec_modele`, `jetons_entree`, `jetons_sortie`, `cout_millicentimes` | une ligne par jour × personne × tâche ; mise à jour atomique ; **reconstructible** depuis le journal |
| `ent_ia_documentation` | `etablissement_id`, `id`, `document_id` (socle 7), `version_id`, `public` (`personnels`, `familles`, `eleves`), `validee_le`, `validee_par`, `retiree_le` | le texte est celui du document du 7 (aucune copie) ; index plein texte sur l'extraction du 7 |

### 26.7 Ce qui n'est **pas** en base

Le registre des capacités et des tâches, les consignes et leurs versions, la liste blanche des fournisseurs, les jeux d'évaluation : **dans le code** (compilés, revus). Les clés des fournisseurs : secrets d'exploitation. La fiche de contexte et la table de correspondance des jetons d'une requête : **en mémoire** ; pour une tâche en file, la correspondance est persistée **scellée** dans `ent_ia_tache_file` (CH6, volatile, neutralisée à la restauration) et effacée à la fin de la tâche.

### 26.8 Classement au registre des tables (E12) et restauration

| Table | Portée | Export | À la restauration |
|---|---|---|---|
| réglages, fournisseurs autorisés, tâches ouvertes, ouvertures, langues, documentation, préférences, formation, exclusions, incidents | `etablissement` | simple | gardées |
| journal et filles | `etablissement` | simple (sans contenu) | gardées |
| conversations, messages, sources, pièces éphémères, raccourcis | `etablissement` | **particulière** (contenu scellé, durée courte ; exclues de l'export complet de l'établissement, incluses dans l'export de la personne) | gardées ; échéances d'effacement **réappliquées** au démarrage |
| `ent_brouillon_*`, `ent_ia_plan*`, `ent_ia_precision_*`, `ent_ia_tache_file` | `etablissement` / `volatile` | aucune | **neutralisées** (E9) : tous les brouillons ouverts passent `expire` |
| `ent_ia_consommation` | `etablissement` | simple | reconstruite depuis le journal si écart |

---

## 27. Migrations (plage imposée 2600–2619)

| N° | Nom | Contenu | Lot |
|---|---|---|---|
| 2600 | `2600_ia_reglages` | `ent_ia_reglage`, `ent_ia_fournisseur_autorise`, `ent_ia_tache_ouverte`, `ent_ia_ouverture`, `ent_ia_langue`, RLS | CH1 |
| 2601 | `2601_ia_journal` | `ent_ia_journal` + 3 filles, déclencheurs d'ajout seul, RLS | CH2 |
| 2602 | `2602_brouillons_action` | `ent_brouillon_action`, `_parametre`, `_element`, `_evenement` (ajout seul), RLS (auteur ou mission) | CH3 |
| 2603 | `2603_ia_file_taches` | `ent_ia_tache_file` (volatile, E9), `ent_ia_plan`, `ent_ia_plan_etape` | CH6 |
| 2604 | `2604_ia_consommation` | `ent_ia_consommation`, index par jour | CH11 (table créée en CH2 pour les compteurs) |
| 2605 | `2605_ia_exclusion` | `ent_ia_exclusion`, FK composite vers l'élève | CH4 |
| 2606 | `2606_ia_conversations` | `ent_ia_conversation`, `ent_ia_message`, `ent_ia_message_source`, `ent_ia_precision_attente`, `ent_ia_precision_option`, `ent_ia_piece_ephemere`, `ent_ia_raccourci_demande`, RLS **propriétaire** | CH5 |
| 2607 | `2607_ia_preferences_formation` | `ent_ia_preference`, `ent_ia_formation` | CH5 |
| 2608 | `2608_ia_retours_incidents` | `ent_ia_retour`, `ent_ia_incident`, `ent_ia_incident_acces` (ajout seul), `CHECK (valide_par <> ouvert_par)` | CH10 |
| 2609 | `2609_ia_documentation` | `ent_ia_documentation`, index plein texte (`pg_trgm`, `unaccent`) | CH7 |
| 2610 | `2610_ia_actions_d017` | actions `ia.*` au catalogue si le catalogue est en base ; sinon dans le code et numéro libéré | CH1 |
| 2611 | `2611_ia_series` | (ajout personas) `ent_brouillon_action.serie_id`, `ent_ia_reauth_serie` (R-CH41) | CH7 |
| 2612 | `2612_ia_journal_purge` | (ajout personas) index de purge du détail nominatif (R-CH47) | CH2 |
| 2613–2617 | réserve | lots CH12–CH13 | |
| 2618 | `2618_ia_rls_stricte` | passage en RLS stricte | CH14 |
| 2619 | `2619_ia_retraits` | retrait de la façade `/copilote/ask` si décidé (aucune table à retirer aujourd'hui) | CH14 |

CH0 n'a **aucune migration**. Contrôle de revue : `grep -i "jsonb\|json\|bytea\|\[\]"` sur 2600–2619 ne doit rien trouver. `MIGRATION_TAG` : 2600 dépasse les plus hautes migrations écrites (642 sur `ent-mc0hoe`) et reste sous 2620 (socle 31) ; ordre croissant dans `migrate.rs`.

---

## 28. Lots (équipe Sonnet)

Chaque lot livre : code, migrations additives, tests Postgres (isolation, matrice de droits, garde), documentation (ce fichier, `PROJECT_STATUS.md`), et le **contrat IA** de ses capacités. Effort : S ≤ 3 j, M 1–2 sem., L > 2 sem. [H].

| Lot | Contenu | Correspond au cœur | Dépend de | Critère de fin | Effort |
|---|---|---|---|---|---|
| **CH0 — correctifs urgents du copilote** | DI-01 (moteur D-017 unique), DI-02 (effectif borné à la portée), DI-03 (journal minimal dans `ent_journal_modif` : qui, intention, volume, sans contenu), DI-04 (limite de débit, 403), DI-05 (plafond 50 + lien), DI-06 (impayés : compteur sans nom), DI-07 (classe résolue par identifiant) | — | rien | tests : professeur de groupe → effectif de sa classe seulement ; absences : 50 lignes au plus ; impayés : aucun nom ; refus 403 ; aucune migration | S |
| **CH1 — contrat et registre** | crate `ent-ia` (types, registre statique, tâches, politique), tests de garde (§5.4 du cœur), actions `ia.*`, `ent_ia_reglage` et tables de réglages (2600), coupe-circuit | IA-L0 | CH0 | registre compilé ; capacité sans droit refusée ; commande sans simulation refusée ; aucun réseau dans le crate | M |
| **CH2 — journal** | 2601, 2604 ; marque `origine` dans le journal métier ; neutralisation E9 déclarée | IA-L1 | CH1 | journal non modifiable (test) ; aucun contenu ; registre E12 à jour | S |
| **CH3 — brouillons et cartes** | 2602 ; routes `/ia/brouillons/{id}` (lire, confirmer, modifier, abandonner) ; carte générique Leptos ; re-vérification ; empreinte ; expiration ; passkey N3 | IA-L1, IA-L2, IA-L7 (partie) | CH2 | DI-17 impossible par construction (test : confirmer avec d'autres paramètres → refus) ; rejeu, brouillon d'autrui, expiré → refus ; simulation sans écriture | M |
| **CH4 — orchestrateur sans modèle et copilote rebranché** | fournisseur nul et simulé ; intentions déterministes → capacités du registre ; résolution des noms dans la portée ; minimisation, pseudonymisation, validation de sortie (nombres, jetons, liens) ; « hors IA » (2605) ; `/copilote/ask` en façade | IA-L2, IA-L3, IA-L4 | CH3 | RF-04 b sans post-filtre ; toutes les questions du copilote servies sans modèle ; jeton inconnu rejeté ; « hors IA » respecté | M |
| **CH5 — panneau et conversation** | 2606, 2607 ; panneau Leptos, barre d'accueil (avec le 29), historique, préférences, formation, journal personnel ; effacement à l'échéance (tâche planifiée du 31) | IA-L7 | CH4 | clavier, 375 px, axe-core ; historique jamais reçu du client (test DI-18) ; RLS propriétaire (direction refusée) | M |
| **CH6 — compréhension et enchaînements** | tâches `liste.filtrer`, `eleve.*`, `classe.quoi_de_neuf`, `action.preparer`, `plan.preparer`, `regle.rediger` ; questions de précision (NXB-07) ; file des tâches longues (2603) | IA-L6 | CH5 ; socles décrits | jeux de compréhension ≥ 95 % avec simulé ; plan : étape 2 préparée sur l'état réel (test) | L |
| **CH7 — rédaction, traduction, documentation** | `texte.*`, `message.preparer*`, `fil.resumer`, `document.expliquer`, base documentaire (2609), pièces éphémères ; appréciations **fermées** | — | CH6 ; socles 7, 8, 16 | brouillons du 16 seulement ; documentation non validée jamais servie ; injection par documentation sans effet | M |
| **CH8 — fournisseurs** | crate `ent-ia-fournisseurs` (compatible OpenAI pour Scaleway et OVHcloud, Mistral si utile), liste blanche, routage par classe, aucun repli ; activation, AIPD-cadre, notice | IA-L5 | CH4 ; **Q-IA2 tranchée par Florian (contrat)** | fournisseur non autorisé → refus ; panne → sans modèle ; `cargo deny` vert ; `security-review` | M |
| **CH9 — mobile et dictée** | routes Bearer, cartes natives, biométrie N3, dictée sur l'appareil, appel dicté | IA-L10 | CH6 ; socle 11 | parité web / mobile ; aucun son envoyé (test réseau) | M |
| **CH10 — qualité, retours, incidents** | harnais d'évaluation (300 + 100 + 50 cas), retours, incidents à double validation (2608), observation de tâche | IA-L8 | CH6, CH8 | 50 injections sans effet ; incident : même personne refusée ; rapport archivé | M |
| **CH11 — quotas, coûts, pilotage** | compteurs, plafonds, coupe-circuit, tableau (47), export personnel | IA-L9 | CH8 | dépassement → arrêt propre du modèle, déterministe intact | S |
| **CH12 — familles** | tâches familles (§15.2), « hors IA » par la famille, notices, parents séparés | IA-5 | CH7, CH10 ; socles 4, 16, 29 ; **AIPD et notice** | matrice parents séparés ; refus neutres ; `security-review` | M |
| **CH13 — élèves de lycée** | tâches élèves (§15.4), refus pédagogique, orientation sans analyse, opposition (18) | IA-5 | CH12 ; **lecture juridique** (public) | jeux élèves ; aucun travail scolaire produit ; aucune conservation des demandes d'aide au-delà de la conversation | S |
| **CH14 — recette et durcissement** | recette par personas sur le logiciel, RLS stricte (2618), retraits (2619) | — | tous | méthode D-021 (4) | M |

**Ordre** : CH0 → CH1 → CH2 → CH3 → CH4 (le cœur, **sans aucun modèle**, utile tout de suite : copilote sûr, palette, cartes du 16 et du 29) → CH5 → CH6 → CH7 ; CH8 dès que le contrat est signé ; CH9 avec le socle 11 ; CH10–CH11 avant toute ouverture réelle ; CH12–CH13 après un an d'usage par les personnels [R].

**Lot 0 (CH0)** : seul lot qui touche du code existant ; il peut partir immédiatement, sans migration, en parallèle des autres socles.

---

## 29. Tests et recette

- **Garde** (fournis par le cœur, appliqués à toutes les capacités) : capacité sans droit, commande sans simulation, équivalence écran / IA, isolation inter-établissements, simulation sans écriture, sensibilité C3/C4.
- **Propres au 30** : historique forgé ignoré (T15) ; confirmation forgée refusée (T16) ; contexte hors portée refusé (T17) ; énumération indiscernable (T18) ; conversation illisible par la direction (T22) ; incident par une seule personne refusé ; « hors IA » sur liste, résumé et synthèse de classe ; nombre absent des sources rejeté ; lien externe supprimé ; quota et plafond ; neutralisation des brouillons à la restauration ; effacement à l'échéance et gel d'incident ; parent séparé ; élève majeur ; collégien sans assistant ; public sans lecture juridique → élèves fermés ; fournisseur nul partout.
- **Jeux d'évaluation** : §18 ; exécution en CI avec le fournisseur simulé, avec un fournisseur réel à la demande, résultats archivés.
- **Recette par personas** (D-021, étape 4) : sur données fictives du labo, chaque persona de `PERSONAS_30.md` fait son vrai travail et **attaque** (élève qui demande les notes d'un autre, parent séparé, professeur hors de ses groupes, autre établissement, injection dans un message).

---

## 30. Décisions

### 30.1 Appliquées par défaut (D-031)

| # | Décision |
|---|---|
| D30-1 | Le socle 30 construit le cœur IA dans sa plage (2600–2610) et en fait ses lots CH1–CH4 (R-CH1). |
| D30-2 | Corrections du cœur pour D-036 (tables filles du journal, fiche de contexte jamais stockée, table de brouillons partagée `ent_brouillon_action`, messages en lignes) (§6.4). |
| D30-3 | Lot CH0 de correctifs du copilote existant, sans migration (DI-01 à DI-07). |
| D30-4 | Déterministe d'abord (R-CH3) ; fournisseur nul par défaut partout. |
| D30-5 | Conversations privées, scellées, 30 jours au plus, lisibles seulement par leur auteur ; incident à double validation et auteur informé (R-CH12, Q-IA9 à confirmer par le DPO). |
| D30-6 | Aucune mémoire de faits ; préférences typées seulement (R-CH9). |
| D30-7 | Jamais de données de santé, même pour l'infirmerie (R-CH6) ; Q-IA5 reste ouverte pour Florian et le DPO. |
| D30-8 | Familles : fermé par défaut, ouvrable après AIPD, notice, information de l'instance et contrat ; élèves de lycée seulement, collégiens fermés en V1 ; public : élèves fermés tant que la lecture juridique de l'arrêté du 7/09/2026 n'est pas faite (R-CH26, R-CH27). |
| D30-9 | Pas d'aide au travail scolaire ; orientation sans analyse pour les demandes d'aide personnelle (R-CH33, R-CH36). |
| D30-10 | Aucune inférence d'émotion ; aucun détecteur de texte IA (R-CH34, R-CH35). |
| D30-11 | Appréciations : tâche préparée, **fermée** jusqu'à l'avis juridique (D-032 confirmé, AI Act annexe III 3 b). |
| D30-12 | Points d'accès sans état seulement, aucun modèle expérimental, retours jamais transmis au fournisseur (R-CH21, R-CH22). |
| D30-13 | Mention « préparé avec l'assistant » sur le brouillon, retirable par l'auteur (Q-IA10) ; journal la garde toujours. |
| D30-14 | Dictée sur l'appareil seulement (Q-IA11). |
| D30-15 | Marque « hors IA » posable par la famille, par l'élève majeur et par l'établissement (Q-IA6). |
| D30-16 | Base documentaire validée remplace l'ingestion de Next.js. |
| D30-17 | Quotas par défaut et plafond mensuel obligatoire (50 € par défaut) [H]. |

### 30.2 Soumis à Florian (engagent l'extérieur, des coûts, des contrats ou des faits qu'il connaît)

| # | Question | Recommandation |
|---|---|---|
| **Q-CH1** | Lecture juridique de l'exigence n° 11 de l'arrêté du 7/09/2026 (IA générative « à travers le GAR ») pour un assistant de vie scolaire dans le public | faire lire par un juriste avant toute ouverture dans un EPLE ; d'ici là : personnels seulement dans le public |
| **Q-CH2** | Rôle de ScolIA au sens de l'AI Act (fournisseur) et évaluation art. 6 § 3-4 à rédiger | oui, par ScolIA, avant CH8 ; relue par un juriste |
| **Q-IA2** (cœur) | Revue contractuelle et choix du fournisseur | **Scaleway Generative APIs avec modèles Mistral** en premier (zéro rétention par défaut, Paris) ; API Mistral avec zéro rétention écrite au contrat en second ; aucun contrat signé par la session |
| **Q-IA5** (cœur) | Données de santé et IA | **jamais** tant que le DPO, un hébergeur qualifié et Florian n'ont pas décidé ; recommandation : ne pas l'ouvrir du tout en V1 |
| **Q-IA8** (cœur) | Modèle commercial (incluse avec quota ou option) | incluse avec quota |
| Q-IA9 (cœur, DPO) | Durée des conversations | 30 jours au plus, réglable vers le bas |
| Q-CH3 | Avis du DPO sur l'AIPD-cadre fournie par ScolIA | à faire relire avant la première activation réelle |
| Q-CH9 | Élève qui écrit une détresse grave à l'assistant (KC-23) : l'assistant doit-il pouvoir **proposer à l'élève** de prévenir un adulte de l'établissement (bouton qui ouvre un message du 16 vers la CPE ou l'infirmière, que l'élève envoie lui-même) ? | **oui, proposition à l'élève, jamais d'alerte automatique** ; à valider avec le DPO et un psychologue de l'Éducation nationale |
| Q-CH14 | Annoncer aux utilisateurs de Next.js ce qui disparaît (mémoire libre, ingestion, bien-être) | oui, au moment de la bascule |
| Q-CH15 | **Risques en production dans Next.js** (DI-10 à DI-23 : confirmation non liée, ingestion sans rôle, santé mentale de mineurs envoyée à Mistral, repli de clé entre établissements) | les traiter côté Next.js **par Florian** (Next.js est intouchable pour la session) ; priorité : bien-être (DI-12, DI-23) et confirmation (DI-17) |

### 30.3 Réglages signalés (D-023) — S-3001 à S-3008

| N° | Réglage | Pourquoi il existe | Avis |
|---|---|---|---|
| S-3001 | Durée de conservation des conversations (0–30 jours) | DPO, choix de l'établissement | garder, vers le bas seulement |
| S-3002 | Quotas par profil | coûts | garder, défauts raisonnables |
| S-3003 | Plafond mensuel | coûts | obligatoire |
| S-3004 | Langues de traduction | population de l'établissement | garder |
| S-3005 | Mention retirable ou non | politique de l'établissement | candidat à l'allègement (une règle fixe : retirable) |
| S-3006 | Tâche « rédaction libre » (sans données) | usage personnel du personnel | fermée par défaut ; candidat à suppression |
| S-3007 | Assistant pour les tiers (grand-parent) | — | absent en V1 |
| S-3008 | Formation obligatoire avant usage | art. 4 | défaut oui ; candidat à règle fixe |

---

## 31. Faits, hypothèses, contradictions

**Faits [F]** : copilote déterministe et ses défauts (§2.1–2.2), lus sur `origin/ent-mc0hoe` le 2026-11-11 ; absence de tout code du cœur IA et de toute plage de migrations pour lui ; production : `install_brain_p0()` appelé au démarrage (`crates/ent-api/src/lib.rs:250`, voir 31) ; Next.js : inventaire du §2.3–2.4 par lecture seule.

**Sourcé [V]/[V~]** : §5 (arrêté du 7/09/2026, cadre d'usage, AI Act consolidé, RGPD, CNIL, fournisseurs).

**Hypothèses [H]** : coûts et quotas, part des demandes servies sans modèle, durées non fixées par un texte, langues de traduction, pratiques de terrain du §4, efforts des lots.

**Contradictions signalées**
- `IA_COEUR.md` (2026-11-06) précède D-036 : ses listes dans le journal, sa « fiche de contexte » en JSON et son `ent_ia_brouillon` sont corrigés ici (§6.4). Le cœur reste la référence pour les principes.
- `IA_COEUR.md` §9.3 donnait le report de l'annexe III au 2/12/2027 comme [NV] : il est désormais sourcé (§5.3).
- `IA_COEUR.md` §13.7 envisageait des usages infirmerie et psychologue en classe « sensible » ; CLAUDE.md et la consigne de cette mission excluent toute donnée de santé : **fermés** ici (R-CH6), jusqu'à décision explicite de Florian.
- La FAQ de la CNIL (06/2025) date l'application complète de l'AI Act au 2/08/2027 : dépassée par l'omnibus.
- Le cadre d'usage du ministère autorise l'IA en classe dès la 4e ; ScolIA ferme les collégiens en V1 : plus strict, pas contradictoire.
- La consigne « Mistral prioritaire » (CLAUDE.md) et la zéro rétention de Mistral accordée « à la discrétion » : réconciliées par des modèles Mistral hébergés chez Scaleway (§5.6), à décider par Florian.

---

## 32. Ajouts de la passe personas (ajout personas, 2026-11-11)

Source : `docs/socles/PERSONAS_30.md` (16 personas, 45 manques dont 6 bloquants). **En cas de contradiction avec les sections précédentes, ce §32 l'emporte.**

### 32.1 Règles ajoutées (R-CH41 à R-CH48)

| N° | Règle | Manque |
|---|---|---|
| R-CH41 | Une ré-authentification vaut 5 minutes pour les cartes N3 **d'une même série** (réponses en série), jamais pour une autre série ni au-delà. | M-3001 |
| R-CH42 | Une conversation n'est jamais versée à un dossier ; seul un document enregistré par une personne l'est. | M-3005 |
| **R-CH43** | Confirmer une carte ne marque **jamais** un message comme lu ou traité dans le 16 ; chaque ligne d'une carte renvoie à son message source. | M-3006 |
| **R-CH44** | L'assistant ne dit jamais rien d'un autre responsable au-delà de ce que l'écran montre à la personne (« 1 signature sur 2 », sans nom). | M-3013 |
| R-CH45 | Aucune prédiction sur la scolarité d'un élève (passage, orientation, examen, réussite), pour aucun profil. | M-3016 |
| **R-CH46** | Une pastille C1 de santé ou d'aménagement peut être montrée **sur un élève** à qui a le droit de la voir ; elle n'est **jamais** un critère de liste, de filtre ou de ciblage par l'assistant. | M-3026 |
| **R-CH47** | Le détail nominatif du journal IA (cibles lues) est purgé à 1 an (DPO, Q-CH17) ; l'agrégat suit l'audit. | M-3030 |
| **R-CH48** | La sortie est rendue en texte avec un Markdown restreint, liens internes générés par le serveur seulement ; aucun HTML, aucune image distante. | M-3036 |

**Précisions** : R-CH4 vaut **à chaque demande**, y compris dans une conversation reprise (participation à un fil revérifiée, M-3035) ; §21.5 ajoute la portée **AESH** (élève accompagné seulement, M-3029) ; §16 ajoute le message hors connexion (M-3021) et le régime de remise affiché sur les cartes de nuit (M-3022).

**Base légale (M-3033)** [H, à valider DPO] : public : mission d'intérêt public ; privé sous contrat : exécution du contrat de scolarisation et intérêt légitime encadré pour les personnels ; **jamais le consentement** (CNIL, FAQ du 20/06/2025 [V~]). L'opposition « hors IA » (Q-IA6) est un droit offert au-delà de la loi.

### 32.2 Actions ajoutées (AST-263 à AST-286)

| N° | Action | Acteur | Règles | Niveau | Lot |
|---|---|---|---|---|---|
| AST-263 | Enregistrer une réponse comme brouillon de note d'entretien (19) ou de document (7), que l'auteur endosse | P | mention « préparé avec l'assistant » ; R-CH42 | N1 | CH7 |
| AST-264 | Effacer par anticipation les conversations ayant un élève pour contexte (pose de « hors IA » ou demande d'effacement du 27) | SYS | auteurs informés « effacée à la demande de la famille » | — | CH5 |
| AST-265 | Consulter la fiche « fournisseur et sous-traitants ultérieurs » (localisation, rétention, garanties, contrat) | ADM, DPO ; résumé dans la notice | versionnée, issue de la liste blanche | N0 | CH8 |
| AST-266 | Reprendre la base de connaissances de Next.js en documents **brouillons** de la base documentaire, relus un à un | ADM, `ia.documentation.valider` | jamais injectés tels quels ; Q-CH18 | N2 | CH14 |
| AST-267 | Tableau de plateforme : coûts, volumes et part sans modèle par établissement, sans contenu | exploitation ScolIA | — | N0 | CH11 |
| AST-268 | Joindre la pièce reçue dans un message comme justificatif de l'absence (12) depuis la carte | CPE, AED | objet existant, pas de copie | N2 | CH6 |
| AST-269 | Voir et modifier, ligne par ligne, le motif proposé et la phrase source d'une carte de justification | CPE, AED | motif flou → ligne décochée | N2 | CH6 |
| AST-270 | Préparer des réponses en série (jusqu'à 30 fils, une carte par fil, pièce par fil) et les confirmer en une fois | secrétariat, vie scolaire | R-CH41 ; fils gelés exclus (KC-31) | N3 | CH7 |
| AST-271 | Corriger une réponse inexacte : l'écran source est ouvert à l'endroit en cause, le retour est classé « inexact » | P | — | N1 | CH10 |
| AST-272 | Voir les différences entre la référence d'une carte (« comme lundi ») et la liste actuelle | professeur | la carte part de la liste actuelle | N0 | CH6 |
| AST-273 | Voir, dans un résumé de fil, le nombre de messages non lus en entier et leurs liens | P | le résumé ne remplace pas la lecture | N0 | CH7 |
| AST-274 | Voir le badge « assistant sans rédaction » et ce qui reste possible | P, F, E | modèle coupé ou non ouvert | N0 | CH5 |
| AST-275 | Lire la notice de l'assistant destinée aux personnels (fournisseur, durées, ce que voit la hiérarchie : rien) | P | — | N0 | CH5 |
| AST-276 | Être rappelé à J-30 de l'échéance de revue de l'AIPD | direction | à l'échéance : fermeture (AST-260) | — | CH8 |
| AST-277 | Proposer un motif d'absence parmi les codes du 12 (jamais un détail médical) | SYS | C1 | — | CH6 |
| AST-278 | À l'accueil, indiquer l'identité déclarée d'un appelant et recevoir l'avertissement du 14 (interdit, non autorisé), sans la raison | accueil | aucune carte préparée pour un appelant interdit (KC-37) | N0 | CH6 |
| AST-279 | Élève : « écrire à un adulte de l'établissement » → message du 16 vers la CPE, l'infirmerie ou le psychologue, **envoyé par l'élève** ; numéros d'urgence affichés | E | remise dans la boîte immédiate (Q-CH16) ; aucune alerte automatique | N3 (par l'élève) | CH13 |
| AST-280 | Refus de prédiction (« vais-je avoir mon bac ? ») avec les règles publiques et le renvoi au professeur principal | P, F, E | R-CH45 | N0 | CH6 |
| AST-281 | Purger le détail nominatif du journal IA à 1 an | SYS (tâche du 31) | R-CH47 | — | CH2 |
| AST-282 | Rendre la sortie sans HTML (Markdown restreint) | SYS | R-CH48 ; test d'injection de balises | — | CH5 |
| AST-283 | Revérifier à chaque demande la participation à un fil et la portée sur l'élève d'une conversation reprise | SYS | R-CH4 | — | CH5 |
| AST-284 | Infirmerie : proposer les tâches non sensibles, refuser tout ciblage dérivé d'une donnée de santé, rédiger un texte général sans données d'élève | infirmerie | R-CH6, R-CH46 ; KC-36 | N0 | CH7 |
| AST-285 | Appliquer un budget en jetons par personne et par jour (en plus du nombre de demandes) | SYS | M-3037 | — | CH11 |
| AST-286 | Afficher la fenêtre de droits utilisée (fenêtre du conseil D-032, intérim, suppléance) dans la réponse | P | — | N0 | CH6 |

**Total après la passe : 187 actions** (AST-100 à AST-286).

### 32.3 Cas limites ajoutés (KC-31 à KC-42)

| N° | Situation | Comportement |
|---|---|---|
| KC-31 | Un fil d'une série de réponses est gelé | exclu de la série, raison affichée |
| KC-32 | Un responsable sans droit de justifier a écrit le motif | ligne écartée « ce responsable ne peut pas justifier » |
| KC-33 | Élève changé de groupe depuis la référence « comme lundi » | différences affichées ; carte sur la liste actuelle |
| KC-34 | Élève qui écrit à un adulte un dimanche soir | message remis dans la boîte, l'écran dit quand il sera lu et affiche les numéros joignables tout de suite |
| KC-35 | Relance préparée pour un foyer en plan d'apurement ou avec une aide en cours | ligne écartée par la commande du 25, raison affichée |
| KC-36 | Infirmière qui demande un message aux parents des élèves non vaccinés | refus du ciblage par l'assistant ; écran de l'infirmerie ; texte général possible |
| KC-37 | Appelant déclaré = responsable sous interdiction de contact | avertissement du 14, aucune carte, aucune information, sans la raison |
| KC-38 | AESH qui demande la liste du cours | refus : portée = élève accompagné |
| KC-39 | Conversation reprise après le retrait de l'auteur du fil | le fil n'est plus lu ; la conversation reste lisible par son auteur, sans nouvelle lecture |
| KC-40 | Sortie du modèle contenant `<script>` ou une image distante | rendu en texte, élément supprimé, alerte au journal |
| KC-41 | Marque « hors IA » posée alors que trois personnels ont une conversation sur l'élève | conversations effacées par anticipation (AST-264) |
| KC-42 | Établissement public, démo sans contrat de fournisseur | tout le cœur sans modèle reste démontrable (CH0–CH4) |

### 32.4 Lots révisés

- **CH2** reçoit AST-281 (purge du détail nominatif).
- **CH3** reçoit les critères d'accessibilité des cartes de lot (M-3044) et la non-modification de l'état de lecture des messages (R-CH43, test).
- **CH5** reçoit R-CH48 (rendu sûr, test d'injection de balises), AST-264, AST-274, AST-275, AST-283.
- **CH6** reçoit AST-268, AST-269, AST-272, AST-277, AST-278, AST-280, AST-286.
- **CH7** reçoit AST-263, AST-270 (avec R-CH41), AST-273, AST-284.
- **CH13** reçoit AST-279 (après Q-CH16) ; **CH14** reçoit AST-266 (après Q-CH18) et le texte d'annonce de la bascule (M-3041).
- Migrations : une table nouvelle (`ent_ia_reauth_serie`, volatile, neutralisée à la restauration) et une colonne, dans la plage : **2611** `2611_ia_series` (`ent_brouillon_action.serie_id`, `ent_ia_reauth_serie` : session, série, valable jusqu'à) ; **2612** `2612_ia_journal_purge` (index par date pour la purge du détail, sans changement de schéma métier) ; 2613–2617 restent en réserve.

### 32.5 Questions ajoutées
Q-CH16 (messages d'élèves hors des heures, DPO et psychologue), Q-CH17 (durée du détail nominatif du journal, DPO), Q-CH18 (copie de la base de connaissances de Next.js, données réelles après accord écrit).
