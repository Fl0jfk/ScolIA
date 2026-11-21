# Socle 31 — Automatisations et règles de l'établissement

**Date :** 2026-11-11 — **Statut :** conception de fond, **non décidée** (à relire par Florian) ; recommandations appliquées par défaut (D-031) sauf ce qui engage l'établissement à l'extérieur (§29.2). Aucun code écrit ni modifié ; aucun accès à une base, à un secret ou à un environnement de production ; Next.js **lu seulement** (routes « cron », routage des demandes, notifications), sans rien exécuter.
**Branche :** `ent-socle-30-31-conception`, partie de `origin/ent-mc0hoe`.
**Passe personas :** `docs/socles/PERSONAS_31.md` ; ses ajouts sont au §31, marqués « ajout personas » ; en cas de contradiction, le §31 l'emporte.
**À lire avec :** `CLAUDE.md` ; `docs/DECISIONS.md` D-017, D-023 (peu de réglages, modèles prêts), D-025 (« renommer `ent-brain` au socle 31 », Q-IA13), D-026 (briques), D-031, D-034, D-035, D-036 ; `docs/SOCLES.md` §31 (A6, A15, A20) ; `docs/PRA_SAUVEGARDES.md` §18.4 (E9 : files d'effets à neutraliser) ; `docs/securite/RLS_STRICTE.md` §4.1 (contextes système nommés) ; `docs/socles/IA_COEUR.md` §4.7, §4.8 ; `docs/socles/30_ASSISTANT_IA.md` (brouillons partagés, R-CH38) ; conceptions 8 (notifications, gel, plage calme, quatre yeux), 12 (alertes d'assiduité §13.4), 13 (procédure disciplinaire), 18 (relances de formulaires), 25 (relances de paiement), 27 (purges), 29 (à-faire : `ouvrir_tache`).

**Conventions.** [F] fait vérifié (Rust depuis `ent-rust/`, Next.js depuis la racine, lecture seule) ; [V] / [V~] source officielle (§5) ; [S] secondaire ; [NV] non vérifié ; [H] hypothèse ; [R] recommandation.

**Numérotation propre au socle** (libre sur `origin/ent-mc0hoe` le 2026-11-11) : existant Next.js **NXR-01…** ; défauts **DR-01…** ; actions **AUT-100…** ; règles **R-AU1…** ; cas limites **KR-01…** ; questions **Q-AU1…** ; réglages **S-3101…** ; manques personas **M-3101…** ; lots **AU0…** ; migrations **2620–2639** (plage imposée, §26).

**Sommaire :** §0 En bref · §1 Thèse · §2 Existant · §3 Acteurs · §4 L'année vue du terrain · §5 Réglementaire · §6 Frontières · §7 Vocabulaire · §8 Le registre des automatismes · §9 Les tâches de fond · §10 Les règles de l'établissement · §11 Déclencheurs, faits, effets · §12 Simulation et validation · §13 Exécution, sûreté, gel · §14 Contrôles de cohérence · §15 Vigilance précoce · §16 Recettes prêtes · §17 IA · §18 Droits et minimisation · §19 Cas limites · §20 Actions · §21 Règles transverses · §22 Événements · §23 Écrans · §24 Modèle de données · §25 Registre des tables · §26 Migrations · §27 Lots · §28 Tests · §29 Décisions · §30 Faits, hypothèses, contradictions · §31 Ajouts personas.

---

## 0. En bref

1. **Trois choses, une seule page.** Le socle 31 possède (a) le **registre des automatismes** : tout ce que ScolIA fait seul (relance d'appel, rappel de rendez-vous, effacement des sanctions échues, purge des conversations, alerte d'assiduité, majorité…) **déclaré par le socle qui le possède**, visible dans une page « Ce que ScolIA fait automatiquement ici », avec son état, sa dernière et sa prochaine exécution, son journal et, quand le socle le permet, son interrupteur ; (b) le **moteur des tâches de fond** qui remplace les boucles écrites à la main dans `ent-api/src/main.rs` ; (c) les **règles de l'établissement** « quand… si… alors… », composées sans code à partir de catalogues fermés, **simulées à blanc**, validées par une seconde personne, exécutées par un moteur déterministe et journalisées.
2. **Une règle ne décide rien et n'écrit à personne de l'extérieur.** Ses effets sont, au choix : créer une **tâche** (à-faire du 29), envoyer une **information interne** neutre à des personnels, **préparer** une carte (même table que l'assistant : `ent_brouillon_action`) qu'une personne confirme, ajouter un élève à une **liste de suivi** ou planifier un **rappel**. Aucune sanction, aucune note, aucune décision d'orientation, aucun message aux familles ou aux élèves **sans geste humain** en V1 (R-AU2) : c'est la lecture prudente de CLAUDE.md (« envois soumis à confirmation humaine ») et de l'art. 22 du RGPD. Un envoi aux familles préparé par une règle part en **un geste** depuis une carte de lot (« 14 rappels prêts — envoyer »).
3. **Pas d'usine à gaz (D-023).** Pas d'éditeur libre : une règle se compose d'**un** déclencheur choisi dans le catalogue que déclarent les socles, de **conditions** sur des faits typés déclarés (au plus 5, en « et », plus « au moins N parmi »), d'une **population** (niveau, classe, groupe, régime…), d'une **fenêtre** et d'**un à trois effets** du catalogue. Et surtout : **30 recettes prêtes** (« 3 retards dans la semaine → tâche au PP », « justificatif non reçu sous 48 h → carte de relance pour la vie scolaire »…) qu'on active et ajuste, plutôt qu'une page blanche.
4. **Simulation obligatoire.** Avant d'être activée, chaque règle est rejouée sur les **4 dernières semaines** (ou une période choisie) : « elle aurait créé 37 tâches pour 12 élèves, dont 9 pour M. Durand ». Une règle trop bavarde est refusée ou plafonnée.
5. **Déterministe, humain, explicable.** Aucun modèle de langage dans le moteur (R-AU4) ; chaque exécution sait dire « pourquoi » (fait, seuil, version de la règle). Un système de règles entièrement définies par des personnes n'est pas, en principe, un « système d'IA » au sens de l'AI Act [NV, considérant 12] : raison de plus pour ne jamais y glisser de modèle.
6. **Données de santé exclues** des règles de l'établissement (R-AU6) ; les automatismes de santé restent internes au socle 34.
7. **Sûreté** : idempotence par clé, profondeur de chaîne bornée (5, comme l'outbox), anti-rafale (par élève et par période), plafond quotidien par règle, **gel** par règle, par automatisme, par élève (socle 8) et **gel général** (« mode crise » : décès, événement grave), respect du calendrier (vacances, fermetures, jours ouvrés, plage calme), propriétaire et valideur toujours présents, règle suspendue si l'un part.
8. **Existant** : `ent-brain` est un **reste à retirer** : il garde **en mémoire, sans limite**, une trace de chaque événement depuis le démarrage du serveur de production (`install_brain_p0()` à l'amorçage, `crates/ent-api/src/lib.rs:250` ; `BrainJournal` = `Mutex<Vec<…>>`), et son unique action (`EnsureAbsence`) n'est jamais appliquée. Les tâches de fond sont des `tokio::spawn` en boucle dans `main.rs`, sans registre, sans journal visible, sans verrou entre instances. Next.js n'a **aucun moteur de règles** : des routes « cron » appelées par un ordonnanceur externe non versionné, protégées par un secret comparé sans temps constant.
9. **Lot 0 (AU0)** : retirer `ent-brain` du démarrage de production (et le crate), sans changement de comportement ; aucune migration.
10. **124 actions** (AUT-100 à AUT-223) au §20, **+ 20 ajoutées par les personas** au §31 ; lots AU0 à AU12.

---

## 1. Thèse : un automatisme est une délégation de vigilance, pas une délégation de décision

1. **On automatise l'attention, pas le jugement.** Le temps perdu dans un établissement est rarement dans la décision ; il est dans **le fait de penser à regarder** : le justificatif qui n'arrive pas, l'élève qui accumule les retards, le PAI qui expire, la convention de stage non signée à J-7. Une règle transforme « penser à regarder » en **une tâche au bon moment chez la bonne personne**.
2. **Tout automatisme a un propriétaire.** Un socle (automatisme intégré) ou une personne nommée de l'établissement (règle). Pas d'automatisme orphelin : si le propriétaire part, la règle s'arrête et la direction est prévenue.
3. **Rien n'est invisible.** Un établissement doit pouvoir répondre à « qu'est-ce que ScolIA fait tout seul chez nous ? » en une page. Les logiciels du marché cachent leurs automatismes dans des réglages épars : c'est l'« usine à gaz » de D-023.
4. **Avant d'agir, on rejoue.** Une règle se juge sur ce qu'elle **aurait fait** le mois dernier, pas sur sa formulation.
5. **Deux paires d'yeux.** Celle qui écrit la règle n'est pas celle qui l'active.
6. **La sortie de l'automatisme est une proposition.** Une tâche, une carte, une information interne. Une personne agit, avec ses droits.
7. **Le calendrier fait partie de la règle.** Une relance pendant les vacances, une alerte un dimanche, un rappel le jour d'un décès dans l'établissement sont des fautes, pas des bugs.

---

## 2. Évaluation franche de l'existant

### 2.1 ENT Rust [F]

| Élément | Où | État |
|---|---|---|
| `ent-brain` | `crates/ent-brain/src/` (lib 176 l., react 99, actions 24, runtime 47, journal 47) | réagit à `appel.closed` en produisant `BrainAction::EnsureAbsence` **jamais appliquée** (`runtime.rs` : `let _actions = react(event)`) ; enregistre chaque événement vu dans `BrainJournal` (`Mutex<Vec<SeenEvent>>`, **sans borne**) ; installé **en production** (`crates/ent-api/src/lib.rs:250`, `boot_from_env`) et dans les tests |
| Toile des événements | `crates/ent-db/src/outbox.rs` (837 l.) | solide : transaction unique fait + événement, distribution « au moins une fois », idempotence par `ent_event_traitement`, profondeur de chaîne `PROFONDEUR_MAX = 5`, causalité (`correlation_id`, `causation_id`), reprise avec délai, file des échecs, rejeu des consommateurs rejouables |
| Consommateurs métier | `reactions.rs` (notifications, relance d'appel, évaluations), `edt_reactions.rs`, `cahier_reactions.rs`, `finances_reactions.rs`, `inscriptions_reactions.rs` (affectation, invitations, accueil) | chacun dans son domaine, idempotent ; **non déclarés** à un registre visible de l'établissement |
| Tâches de fond | `crates/ent-api/src/main.rs:195-290` | boucles `tokio::spawn` + `sleep` : envois d'inscription (5 s), délégations échues (600 s), levées de fermetures (60 s), expéditeur de notifications et rappels de rendez-vous (300 s), tâche quotidienne des élèves et doublons (3 600 s) ; désactivables par variables d'environnement |
| Contextes système | `crates/ent-db/src/tenant_tx.rs`, `docs/securite/RLS_STRICTE.md` §4.1 | `JobPlanifie(nom)`, `TacheNocturneEleves`, `WorkerEnvoi`… : un passage par établissement sous RLS — **bon mécanisme à généraliser** |
| Fonctions périodiques sans ordonnanceur | `effacer_sanctions_echues` (`discipline_vs.rs:565-579`, appelée seulement à la demande d'un acteur), `JobsFinances` (réservé, non utilisé) | à brancher sur le moteur |
| Signaux et alertes | `012_absence_signal`, `035`, `043_absence_signal_alerte_famille`, `069_alerte_cpe` (file manuelle d'alertes) | propriété du socle 12 (§13.4 de sa conception) ; le 31 ne les reprend pas |

### 2.2 Défauts de l'existant Rust

| N° | Défaut | Preuve | Gravité | Correctif |
|---|---|---|---|---|
| **DR-01** | **Croissance mémoire sans limite en production** : chaque événement métier depuis le démarrage est ajouté à un vecteur en mémoire (identifiants d'établissement et d'élève compris), jamais vidé | `ent-brain/src/journal.rs`, `runtime.rs` ; `ent-api/src/lib.rs:250` | **important** (fuite mémoire lente, données personnelles en mémoire sans usage) | AU0 : retirer `install_brain_p0()` du démarrage de production ; les tests qui l'appellent passent à une sonde de test |
| DR-02 | Code mort trompeur : `EnsureAbsence` décrit une écriture directe dans `ent_absence` (hors propriétaire, contraire à `SOCLES.md` §1) qui n'est jamais exécutée ; le nom « brain » fait croire à une IA (IA_COEUR §2.2) | `ent-brain/src/actions.rs`, `react.rs` | confort | AU0 : supprimer le crate après retrait des appels (Q-IA13 tranchée : pas de renommage, suppression) |
| DR-03 | Tâches de fond **invisibles** : aucun registre, aucun journal consultable par l'établissement, échecs seulement dans les journaux applicatifs | `main.rs:195-290` | important (exploitation) | AU1 : registre des tâches de fond et journal des passages |
| DR-04 | **Aucun verrou entre instances** : deux serveurs lancent les mêmes boucles ; correct seulement parce que chaque tâche est idempotente par construction | idem | important à l'échelle | AU1 : réclamation `FOR UPDATE SKIP LOCKED` sur la ligne de la tâche, ou verrou consultatif de session |
| DR-05 | Fréquences codées en dur, « le jour » dépend du fuseau de chaque établissement (commentaire `main.rs` : tâche horaire pour cette raison) | idem | confort | AU1 : échéance calculée par établissement dans son fuseau (socle 5) |
| DR-06 | Effacement des sanctions échues jamais planifié (seulement à la demande) | `discipline_vs.rs:565-579` | important (conservation, socle 13) | AU1 : tâche planifiée déclarée par le 13 |
| DR-07 | Arrêt d'une tâche = variable d'environnement et redémarrage ; aucun gel par établissement | `main.rs` (`ENT_TACHES_ELEVES=0`…) | confort | AU2 : gel par automatisme et par établissement |

### 2.3 Next.js en production (lecture seule, D-035) [F]

**Pas de moteur générique « quand… alors… »** : aucune trace trouvée. Pas de `vercel.json`, pas d'ordonnanceur dans l'application (`Dockerfile:74` ne lance que `scripts/docker-entrypoint.sh`) ; les « cron » sont des routes POST appelées par un **ordonnanceur externe non versionné**, protégées par un secret partagé comparé par simple égalité (déjà relevé, `docs/MATRICE_CHARLEMAGNE.md:563`) ; aucune ne demande de confirmation humaine ; journal en console.

| N° | Automatisme Next.js | Où | Effet | Reprise ScolIA |
|---|---|---|---|---|
| NXR-01 | Reconfirmation des rendez-vous d'inscription à J-7 | `app/api/rdv-inscription/reconfirm-cron/route.ts:9-29`, `rdv-inscription-service.ts:1522` | e-mail « toujours d'accord ? » ; le silence ne supprime pas le rendez-vous | automatisme intégré du socle 19 (rappels de rendez-vous) |
| NXR-02 | Suivi des portes ouvertes | `app/api/portes-ouvertes/follow-up/route.ts:5-16`, `portes-ouvertes-mail.ts:603` | e-mail avec lien de préinscription (40 au plus par passage) | automatisme intégré du socle 3 (ou 19) ; **préparé** en carte de lot en V1 (R-AU2) |
| NXR-03 | Relances de paiement de l'abonnement, suspension automatique de l'établissement client | `app/api/billing/dunning/process/route.ts`, `tenant-billing.ts:337-365` | rappel, suspension, révocation de licences | **hors ENT** (facturation de ScolIA à ses clients) ; à ne pas mêler aux familles |
| NXR-04 | Résumé hebdomadaire de l'internat aux parents | `app/api/internat/weekly-parent-digest/route.ts`, `internat-weekly-digest.ts:57-67` | e-mail aux parents (activation, jour) | socle 41 + résumé du socle 8 (information due, réglée par la direction) |
| NXR-05 | Rappel d'appel de l'internat (lundi–jeudi, 21 h par défaut) | `app/api/internat/roll-call/reminder/route.ts`, `internat-roll-call-reminder.ts:31-50` | e-mail aux surveillants | automatisme intégré du socle 41 (interne, permis) |
| NXR-06 | Synchronisation hebdomadaire de la nomenclature Omogen | `app/api/nomenclature/omogen-sync/cron/route.ts:22-69` | import | tâche de fond du socle 9 |
| NXR-07 | Relève IMAP des devis de voyages | `app/api/travels/poll-email/route.ts:10-37` | ingestion (25 e-mails) | tâche de fond du socle 42 |
| NXR-08 | Rappels de voyages (car à J-3, communication aux parents à J0) | `app/api/travels/reminders/route.ts:27-212` | e-mail interne au créateur, une fois | automatisme intégré du 42 (tâches à l'organisateur) |
| NXR-09 | Routage des demandes : Mistral (confiance ≥ 0,55), repli par mots-clés (score ≥ 3), corbeille si ambigu (rapport 0,85) ; organigramme et mots-clés paramétrés par l'établissement | `app/lib/requests.ts:602`, `requests-routing-config.ts:38-43, 356-372`, `requests-org-routing.ts:36-110` | affectation | **règles de routage** du 31 (effet « affecter à une boîte ») pour le futur socle des demandes (N1, Q-T0-04) ; l'IA **propose** un routage, n'affecte jamais (R-CH16) |
| NXR-10 | File de validation des absences du personnel (valideurs OGEC / professeur) | `app/lib/absences-routing.ts:115-255` | circuit | circuit du socle 15 (`15_ABSENCES_PERSONNELS.md`) |
| NXR-11 | Notifications synchrones à chaque changement d'état (absences, documents, stages, voyages, internat, covoiturage, fiches de dialogue, PAP, bien-être) | `app/lib/*-notify.ts` | e-mails | natures du socle 8 déclarées par chaque socle |
| NXR-12 | Purges « paresseuses » : demandes à clôture + 30 jours, stages de l'année précédente au GET de `/api/stages`, blogs de voyage au GET des rappels, bien-être à la lecture ; `purgeExpiredAbsences` vide | `requests.ts:204-220`, `stage-auto-purge.ts:40-50`, `travels/reminders/route.ts:37-45`, `absences-storage.ts:141-143` | — | **tâches de fond déclarées** par chaque socle, planifiées, journalisées (socle 27) |

### 2.4 Défauts de Next.js à ne pas reproduire (signalés à Florian ; Next.js intouchable)

| N° | Défaut | Ce que fait ScolIA |
|---|---|---|
| DR-10 | Ordonnanceur externe non versionné : personne ne sait d'un coup d'œil ce qui tourne | registre dans le code, page « automatismes » |
| DR-11 | Secret partagé comparé sans temps constant, accepté aussi dans le corps de la requête (NXR-01) | aucune route « cron » exposée : le moteur tourne dans le serveur |
| DR-12 | Purges déclenchées par la consultation d'une page (effet de bord d'un GET) | purges planifiées, jamais sur lecture |
| DR-13 | Affectation automatique d'une demande sur une confiance de modèle | proposition seulement |
| DR-14 | Envois aux familles sans aucune validation (suivi des portes ouvertes) | carte de lot en V1 |
| DR-15 | Aucun journal des automatismes hors console | journal des exécutions en base |

### 2.5 À ne pas perdre (D-035) et verdict

À ne pas perdre : les rappels de rendez-vous (NXR-01), le rappel d'appel d'internat (NXR-05), le résumé internat (NXR-04), les rappels de voyage (NXR-08), le routage paramétrable des demandes (NXR-09). Chacun a son socle propriétaire ; le 31 leur donne **un registre, un journal, un interrupteur et un calendrier**.
**Verdict** : la toile des événements Rust est une excellente base ; `ent-brain` est à retirer ; les tâches de fond sont à ranger ; il n'existe nulle part (ni Rust, ni Next.js) de **règles de l'établissement** : c'est l'apport neuf du socle.

---

## 3. Acteurs, besoins réels, irritants

| Acteur | Ce qui lui fait perdre du temps | Ce qu'il redoute | Ce qui le ravirait |
|---|---|---|---|
| **Direction** | découvrir trop tard (le décrochage, la convention non signée, le dossier incomplet) ; ne pas savoir ce que le logiciel fait seul | l'automatisme qui écrit à une famille un jour de deuil ; l'alerte qui étiquette un élève | une page « ce que ScolIA fait chez nous », un bouton « tout suspendre », des règles qu'elle valide après avoir vu leur effet |
| **CPE, vie scolaire** | penser à relancer, recompter les retards, vérifier les justificatifs à 48 h | la règle trop bavarde (40 tâches le lundi), l'élève oublié | « 3 retards dans la semaine → tâche pour moi et le PP », « justificatif non arrivé à 48 h → carte de relance prête » |
| **Secrétariat** | suivre les dossiers incomplets, les fins de validité (pièce d'identité pour un voyage, attestation d'assurance) | relancer deux fois la même famille | « pièce manquante depuis 7 jours → relance préparée, groupée par foyer » |
| **Professeur principal** | ne pas voir l'élève discret qui décroche | être noyé de notifications | une **liste de suivi** courte, expliquée, hebdomadaire |
| **Professeurs** | — | qu'une règle les juge (« appel non fait ») publiquement | rappels discrets (« appel de 8 h non validé ») qu'ils peuvent couper pour eux-mêmes si le socle le permet |
| **Comptabilité** | relances par niveau, échéances | erreur de foyer, relance pendant un plan d'apurement | relances préparées par le 25, suspendues automatiquement par un plan ou une aide en cours |
| **Accueil** | — | — | rien de nouveau : les signalements arrivent déjà au bon endroit (12, 14) |
| **Infirmerie** | PAI qui expirent, vaccins | qu'une règle de l'établissement touche la santé | automatismes du 34 seulement (R-AU6) |
| **Familles, élèves** | relances en double, messages à 23 h | être « surveillé » par un algorithme | moins de messages, mieux groupés, jamais la nuit ; rien n'est décidé sur eux par une machine |
| **Florian, administrateur ScolIA** | savoir ce qui a tourné, échoué, été rejoué | deux instances qui envoient deux fois ; une restauration qui renvoie des relances | journal des passages, verrou, neutralisation à la restauration |
| **DPO, auditeur** | prouver l'absence de décision automatisée et de profilage opaque | une « détection de décrochage » qui devient un fichier d'élèves « à risque » | règles lisibles, journal, durées courtes, aucune étiquette persistée |

---

## 4. L'année vue du terrain [H, à confirmer avec Florian]

- **Fin août** : la direction ouvre « Automatismes » ; 30 recettes proposées, 6 cochées par défaut (celles qui ne produisent que des tâches internes et reprennent ce que Next.js faisait) ; elle en active 4 autres après simulation sur juin.
- **Rentrée** : gel automatique des règles d'assiduité pendant la **semaine de rentrée** (S-3105, réglable) — les emplois du temps bougent, les listes aussi.
- **Octobre** : la CPE crée avec l'assistant « 3 retards dans la semaine → tâche au PP » ; simulation sur septembre : 22 tâches, 15 élèves ; la directrice adjointe la valide.
- **Novembre** : un deuil dans l'établissement ; la direction active le **gel général** des envois préparés et des rappels non urgents pour 3 jours ; tout reprend seul ensuite, avec un récapitulatif de ce qui a été retenu.
- **Vacances** : les règles « jours ouvrés » se taisent ; les échéances qui tombent pendant les vacances glissent au premier jour de reprise.
- **Conseils de classe** : la liste de suivi du PP (vigilance précoce, §15) est jointe au dossier du conseil **pour le PP seulement**, puis remise à zéro au trimestre suivant.
- **Juin** : bilan annuel des automatismes (combien de tâches, combien traitées, combien jugées inutiles) ; les règles jamais utiles sont proposées à l'archivage.
- **Bascule d'année** : règles conservées, populations recalculées sur la nouvelle structure (niveaux, classes) ; une règle qui visait une classe qui n'existe plus est **suspendue** avec une tâche au propriétaire.

---
## 5. Ce qui est réglementaire (vérifié le 2026-11-11)

Recherche par sous-agent, sources officielles, le 2026-11-11 (mêmes limites qu'au §5 du socle 30 : Légifrance et CNIL lus à travers l'outil, [V~]).

### 5.1 Décision automatisée et profilage
- **RGPD art. 22** [V~] (https://www.cnil.fr/fr/reglement-europeen-protection-donnees/chapitre3) : pas de décision fondée **exclusivement** sur un traitement automatisé produisant des effets juridiques ou significatifs. **Conséquence** : aucun effet de règle ne produit, seul, un effet sur un élève, une famille ou un personnel (R-AU2) ; les effets sont des tâches et des propositions.
- **AIPD** [V~] : délibération CNIL n° 2018-327 (https://www.legifrance.gouv.fr/jorf/id/JORFTEXT000037559521) et critères du CEPD ; une « vigilance précoce » qui croise assiduité, résultats et discipline **sur des élèves** (personnes vulnérables) est un **profilage** au sens de l'art. 4 § 4 du RGPD [H] → AIPD avant activation (R-AU30) ; FAQ CNIL éducation 2025 : AIPD « oui, la plupart du temps » [V~].
- **AI Act** [V~ ; NV pour le considérant] : l'art. 6 § 3 classe toujours à haut risque un système d'IA qui fait du profilage ; mais un ensemble de **règles entièrement définies par des personnes** n'est en principe pas un système d'IA (considérant 12, **non vérifié** sur le texte). **Conséquence** : la vigilance précoce reste **déterministe**, faite de règles lisibles écrites par l'établissement ou fournies par ScolIA, **sans modèle** ni apprentissage, sans score (R-AU4, R-AU31) ; elle n'est jamais présentée comme une « prédiction ».

### 5.2 Ce que le code de l'éducation réserve à une décision humaine (public ; le privé l'applique par son règlement intérieur, socle 13)
- **Discipline** : le chef d'établissement prononce les sanctions (R.421-10, version du 4/07/2025) ; il est **tenu** d'engager une procédure dans certains cas (violence verbale envers un personnel, acte grave, atteinte grave aux principes de la République, harcèlement) ; procédure contradictoire, délai d'au moins deux jours ouvrables, information du représentant légal (R.421-10-1) [V~] (https://www.legifrance.gouv.fr/codes/section_lc/LEGITEXT000006071191/LEGISCTA000018377430/). **Conséquence** : aucune règle ne prononce ni ne prépare une sanction comme décision ; une règle peut créer une **tâche** « examiner la situation » à la direction (R-AU8).
- **Absentéisme** : motif à faire connaître « sans délai » par les responsables ; **au moins quatre demi-journées dans le mois** sans motif valable → le directeur saisit l'autorité académique (L.131-8, https://www.legifrance.gouv.fr/codes/article_lc/LEGIARTI000025165453) ; « toute absence est immédiatement signalée aux personnes responsables » (R.131-5) ; dossier d'absences de l'année, dialogue en cas d'absences répétées (R.131-6) ; commission éducative, avertissement, saisine (R.131-7) (https://www.legifrance.gouv.fr/codes/section_lc/LEGITEXT000006071191/LEGISCTA000006182466/) [V~]. **Conséquences** : la notification **immédiate** d'une absence aux responsables est une **obligation** que porte le socle 12/8 (information due, automatique) — elle n'est pas une « règle de l'établissement » et le 31 ne la gêne pas ; le seuil de 4 demi-journées est un **automatisme intégré du 12** (alerte), jamais une saisine automatique (R-AU9).

### 5.3 Droit à la déconnexion (public, arrêté du 7/09/2026)
Les services de vie scolaire **suspendent par défaut la diffusion des nouvelles informations de 20 h à 7 h en semaine et du vendredi 20 h au lundi 7 h**, la saisie restant possible (relevé par le socle 29, §5.1, [V~]). **Conséquence** : tout effet d'une règle qui atteint une famille ou un élève (y compris une tâche visible de lui) est **différé** à la fin de la plage (R-AU12) ; le privé applique la même règle par défaut (cohérence avec 16, 29).

### 5.4 Conservation
Le dossier d'absences est limité à l'année scolaire (R.131-6) [V~] ; les **listes de suivi** de la vigilance précoce ne survivent pas à la période (trimestre) sans décision d'une personne (R-AU32) ; le journal des exécutions sans contenu suit l'audit (27) ; ses détails nominatifs sont purgés à 1 an [H, DPO].

---

## 6. Frontières

| Sujet | Propriétaire | Le socle 31 |
|---|---|---|
| Automatismes métier (relance d'appel, rappel de rendez-vous, alertes d'assiduité, relances de paiement et de formulaires, effacement des sanctions, majorité, purges) | **le socle concerné** (12, 19, 25, 18, 13, 2, 27…) | les **inscrit au registre** (déclaration du socle), les **planifie**, les **journalise**, offre l'**interrupteur** quand le socle l'a permis ; ne réécrit pas leur logique |
| Informations dues aux familles (absence signalée, bulletin, convocation) | **8** et le socle source | aucune règle ne les remplace ni ne les bloque ; le gel général du 31 **ne touche pas** les informations dues urgentes (R-AU22) |
| Tâches (à-faire) | **29** (`ouvrir_tache`) | effet « créer une tâche » = appel du kit du 29 |
| Cartes, brouillons | cœur IA / **30** (`ent_brouillon_action`) | effet « préparer » = brouillon d'origine `regle`, destinataire personne ou mission |
| Notifications internes | **8** | effet « informer » = nature du 8 `regles.information` (neutre) |
| Règles, simulation, recettes, listes de suivi, contrôles de cohérence, tâches de fond, gel | **31** | possède |
| Seuils réglementaires d'assiduité, procédure | **12**, **13** | le 31 ne les duplique pas ; une règle de l'établissement peut **ajouter** une vigilance plus précoce (« 2 demi-journées »), jamais retirer une alerte réglementaire |
| Demandes et routage (N1) | socle des **demandes** à créer (Q-T0-04) | le 31 fournit l'effet « affecter à une boîte » et les règles de routage ; les demandes elles-mêmes n'existent pas encore |
| Santé | **34** | aucune règle de l'établissement sur un fait de santé (R-AU6) |
| Indicateurs | **47** | le 31 publie compteurs et taux |

### 6.1 Ports consommés
`evenements` (outbox : le moteur est un **consommateur** rejouable pour l'évaluation, non rejouable pour les effets externes) ; `faits_ouverts_aux_regles()` (chaque socle déclare ses faits typés) ; `populations(etablissement, type, id, date)` (5 : niveau, classe, groupe, cours, régime, internat) ; `calendrier(etablissement)` (5 : jours ouvrés, vacances, fermetures, fuseau) ; `plage_calme` (8) ; `ouvrir_tache` (29) ; `informer` (8) ; `preparer_brouillon` (30) ; `gel(sujet)` (8) ; `agents_et_missions` (6 : propriétaire, valideur, destinataires) ; `droits` (D-017 : le destinataire d'un effet doit pouvoir voir l'objet).

### 6.2 Ports exposés
`declarer_automatisme(...)` et `declarer_tache_de_fond(...)` (kit de branchement, D-026) ; `automatisme_actif(etab, code)` (le socle propriétaire demande avant d'agir) ; `gel_general_actif(etab, nature)` ; `listes_de_suivi_de(acteur)` (29, 23) ; `controles_ouverts(etab)` (29, 47) ; `simuler_regle(version, periode)` (30 pour l'explication) ; `regle_expliquer(execution_id)` (30).

---

## 7. Vocabulaire fixé [R]

| Terme | Sens |
|---|---|
| **Automatisme** | tout comportement que ScolIA exécute sans geste humain immédiat |
| **Automatisme intégré** | automatisme codé et possédé par un socle, déclaré au registre |
| **Tâche de fond** | exécution planifiée (périodique ou à date) d'un automatisme intégré ou du moteur |
| **Règle** | automatisme créé par l'établissement : déclencheur + conditions + population + fenêtre + effets |
| **Version de règle** | contenu figé d'une règle ; une règle active a une version active ; modifier = nouvelle version à revalider |
| **Déclencheur** | événement déclaré ouvert aux règles, ou échéance de calendrier |
| **Fait** | valeur typée et datée qu'un socle sait calculer pour un objet (élève, dossier…) et qu'il déclare ouverte aux règles (« nombre de retards sur 7 jours ») |
| **Effet** | ce qu'une règle produit : tâche, information interne, carte préparée, ajout à une liste de suivi, rappel, affectation à une boîte |
| **Exécution** | une évaluation d'une règle pour une cible, journalisée, avec son résultat |
| **Simulation** | exécution à blanc sur une période passée ; n'écrit rien d'autre que son rapport |
| **Recette** | modèle de règle fourni par ScolIA, prêt à activer |
| **Liste de suivi** | liste courte d'élèves (ou de dossiers) à regarder, propre à une personne ou une mission, alimentée par des règles, remise à zéro à chaque période |
| **Contrôle de cohérence** | vérification croisée déclarée par un socle (« élève inscrit sans classe ») qui ouvre une anomalie |
| **Gel** | suspension d'un automatisme, d'une règle, d'un élève ou de tout un établissement |

---

## 8. Le registre des automatismes

### 8.1 Déclaration (kit de branchement) [R]

Chaque socle déclare, **dans son code** (comme ses actions D-017), ses automatismes :

| Champ | Exemple |
|---|---|
| Code stable | `vs.relance_appel_non_fait` |
| Socle propriétaire | 12 |
| Libellé et description pour l'établissement | « Rappelle au professeur un appel non validé 15 minutes après le début du cours. » |
| Déclencheur | événement, échéance, ou tâche de fond |
| Effet, destinataires | notification interne au professeur |
| Touche l'extérieur ? | non / oui (familles, élèves, tiers) |
| **Obligatoire** (réglementaire ou sécurité) / **réglable** / **désactivable** | réglable (délai 5–60 min) |
| Paramètres exposés (typés, bornés) | `delai_minutes` (entier 5–60, défaut 15) |
| Gel possible | oui |
| Neutralisé à la restauration (E9) | oui |
| Droits pour le régler | `automatismes.regler` |

Un automatisme **non déclaré** ne peut pas tourner : le moteur des tâches de fond refuse un code inconnu, et un test CI liste les consommateurs de l'outbox et les tâches de fond non déclarés (comme le test du registre des tables, E12).

### 8.2 La page « Ce que ScolIA fait automatiquement ici » [R]

Pour chaque automatisme (intégrés et règles, filtrables par socle, par public touché, par état) : état (actif, suspendu, gelé, désactivé), dernière exécution, prochaine échéance, nombre d'effets sur 30 jours, taux de traitement des tâches produites, échecs, **paramètres** (dans leurs bornes), interrupteur si permis, journal. En tête : le **gel général** et son état. Exportable pour le CA ou le DPO (registre des automatismes, AUT-203).

### 8.3 Automatismes intégrés connus à ce jour (à déclarer par leurs socles)

| Code | Socle | Effet | Externe ? | Obligatoire ? |
|---|---|---|---|---|
| `eleve.tache_quotidienne` (statuts, majorité, identités effectives) | 2 | recalculs | non | oui |
| `eleve.doublons` | 2 | propositions de fusion | non | réglable |
| `eleve.propositions_expiration` | 2 | expiration | non | oui |
| `delegation.balayage` | 1 | expiration et événement | non | oui |
| `structure.fermetures_levees` | 5 | levée programmée | non | oui |
| `notifications.expedition` | 8 | envois en file | oui | oui |
| `rdv.rappels` | 19 | rappels de rendez-vous (NXR-01) | oui | réglable |
| `vs.relance_appel_non_fait` | 12 | notification au professeur | non | réglable |
| `vs.alertes_assiduite` (§13.4 du 12) | 12 | tâches CPE, direction | non | **seuil légal obligatoire** ; autres réglables |
| `discipline.effacement_sanctions` | 13 | effacement planifié (DR-06) | non | oui |
| `formulaires.relances` | 18 | relances des destinataires | oui | réglable |
| `fin.relances` | 25 | cartes de relance par palier | oui (préparé) | réglable |
| `internat.rappel_appel` | 41 | e-mail aux surveillants (NXR-05) | non | réglable |
| `internat.resume_familles` | 41 / 8 | résumé hebdomadaire (NXR-04) | oui | désactivable |
| `voyages.rappels` | 42 | tâches à l'organisateur (NXR-08) | non | réglable |
| `voyages.releve_devis` | 42 | ingestion (NXR-07) | non | désactivable |
| `nomenclatures.synchro` | 9 | import (NXR-06) | non | réglable |
| `inscriptions.envois` | 3 | envois d'inscription | oui | oui |
| `rgpd.purges` (par domaine) | 27 | purges | non | oui |
| `ia.effacement_conversations` | 30 | effacement à l'échéance | non | oui |
| `afaire.reconstruction` | 29 | index | non | oui |

---

## 9. Les tâches de fond

### 9.1 Ce que devient `main.rs` [R]

Un **ordonnanceur unique** (`ent-auto`, dans le serveur) remplace les boucles : il lit le registre des tâches de fond (code), calcule pour chaque établissement la prochaine échéance **dans son fuseau**, réclame la tâche (`SELECT … FOR UPDATE SKIP LOCKED` sur `ent_auto_planif`), l'exécute dans le contexte système nommé `JobPlanifie(code)` **par établissement** (RLS stricte), écrit un **passage** (début, fin, résultat, nombre d'objets, erreur codée) et calcule l'échéance suivante. Deux instances ne font jamais la même tâche en même temps (DR-04).

### 9.2 Formes d'échéance (typées, pas de chaîne « cron » libre) [R]
`toutes_les(n minutes)` (≥ 1) · `quotidienne(heure locale)` · `jours_ouvres(heure locale)` (calendrier du 5) · `hebdomadaire(jour, heure)` · `mensuelle(jour, heure)` · `annuelle(date relative : rentrée, fin d'année, bascule)` · `a_date(instant)` (une fois). Une échéance qui tombe en plage calme ou en fermeture est **reportée** si l'automatisme touche l'extérieur.

### 9.3 Fiabilité [R]
Chaque tâche est **idempotente** (déjà la règle) ; un passage échoué est retenté avec délai croissant (3 fois), puis l'automatisme passe « en échec » : tâche à l'administrateur de l'établissement et alerte d'exploitation ScolIA. Une tâche qui dure plus que sa période ne se chevauche pas avec elle-même. À la **restauration** (PRA), toutes les tâches à effet externe sont **suspendues** jusqu'à la levée explicite par l'exploitation (E9, R-AU25).

---

## 10. Les règles de l'établissement

### 10.1 Forme d'une règle [R]

```
QUAND   <déclencheur>             (un seul : événement du catalogue ou échéance)
POUR    <population>              (élèves d'un niveau / d'une classe / d'un groupe / d'un régime / tous ; ou dossiers d'un type)
SI      <condition 1> ET … ET <condition n ≤ 5>     (ou « au moins N parmi » ces conditions)
DANS    <fenêtre>                 (jours ouvrés seulement ? hors vacances ? période de l'année ?)
ALORS   <effet 1> [, <effet 2>, <effet 3>]
LIMITES une fois par cible et par <période> ; au plus <M> effets par jour ; gel respecté
```

Exemple (recette RC-03) : *Quand un retard est enregistré, pour les élèves de collège, si le nombre de retards de l'élève sur les 7 derniers jours ouvrés est ≥ 3, dans les jours ouvrés, alors créer une tâche « Échanger avec l'élève sur ses retards » pour son professeur principal et l'ajouter à la liste de suivi de la vie scolaire ; une fois par élève et par semaine.*

### 10.2 Cycle de vie [R]

`brouillon` → (simulation faite) `a_valider` → `active` ⇄ `suspendue` → `archivee`. Modifier une règle active crée une **nouvelle version** en `brouillon`, la version active continuant jusqu'à la validation de la nouvelle. Les états `suspendue_auto` (propriétaire ou valideur parti, population disparue, fait retiré par son socle, plafond dépassé trois jours de suite) exigent une action humaine pour reprendre.

### 10.3 Qui fait quoi [R]

- **Rédiger** : toute personne ayant `automatisations.creer` (direction, CPE, secrétariat par défaut) ; aussi avec l'assistant (`regle.rediger`, 30), qui produit un brouillon, jamais une règle active.
- **Valider** : une **autre** personne ayant `automatisations.valider` (direction par défaut) et ayant elle-même, au moment de la validation, les droits de **voir** ce que les effets montreront à leurs destinataires (R-AU14).
- **Propriétaire** : la personne qui a rédigé (transférable) ; reçoit les tâches d'échec et le bilan.
- **Destinataires** : désignés par **fonction** ou **mission** (« le PP de l'élève », « la vie scolaire du niveau », « la boîte du secrétariat ») plutôt que par nom ; une personne nommée est permise mais signalée (elle part un jour).

---

## 11. Déclencheurs, faits, effets (catalogues fermés)

### 11.1 Déclencheurs ouverts aux règles (déclarés par les socles) [R]

| Socle | Événements proposés |
|---|---|
| 2 | élève inscrit, changement de classe, sortie annoncée, élève majeur |
| 3 | dossier incomplet depuis N jours, dossier validé |
| 4 | responsable sans moyen de contact, changement de foyer |
| 7 | pièce expirée ou à expiration, document non signé depuis N jours |
| 10 | séance annulée, remplacement non pourvu |
| 12 | absence enregistrée, absence non justifiée à N heures, retard enregistré, appel non fait, mois arrêté |
| 13 | incident enregistré (catégorie seulement, sans contenu), punition non faite |
| 15 | absence de personnel validée |
| 16 | message d'une famille sans réponse depuis N jours ouvrés (boîte de service) |
| 18 | formulaire non rempli à J-N de l'échéance |
| 19 | rendez-vous demandé non traité |
| 20 | travail non rendu (si le professeur l'a enregistré) |
| 21 | note publiée sous un seuil fixé par l'établissement **pour l'élève lui-même** (variation forte) |
| 24, 25 | échéance impayée (agrégat foyer, sans montant dans les effets) |
| 39 | convention non signée à J-N |
| 42 | sortie à J-N sans autorisation complète |
| calendrier | chaque jour ouvré à une heure, chaque lundi, début / fin de période, J-N avant un conseil de classe |

### 11.2 Faits ouverts aux règles [R]

Chaque fait a : code, socle, type (entier, durée, date, booléen, code), unité, fenêtre glissante possible, **classe de sensibilité** (C0–C2 seulement : **aucun fait C3/C4**, R-AU6), description. Exemples : `vs.retards_n_jours`, `vs.demi_journees_non_justifiees_mois`, `vs.absences_episodes_n_semaines`, `cdt.travaux_non_rendus_n_jours`, `eva.variation_moyenne_periode` (pour l'élève par rapport à lui-même), `dis.incidents_n_jours` (nombre, catégorie), `dos.pieces_manquantes`, `dos.jours_depuis_dernier_contact`, `ins.jours_depuis_relance`, `eleve.niveau`, `eleve.regime`, `eleve.interne`, `eleve.majeur`, `eleve.nouveau_cette_annee`, `fil.jours_sans_reponse`. **Pas** de fait « boursier », « PAI », « handicap », « suivi social », « nationalité », « adresse ».

### 11.3 Effets ouverts aux règles [R]

| Effet | Ce qu'il fait | Limites |
|---|---|---|
| `tache.creer` | tâche du 29 au destinataire (fonction, mission ou personne), titre **neutre**, échéance, lien vers l'objet | le destinataire doit pouvoir voir l'objet, sinon effet sauté et journalisé |
| `information.interne` | notification du 8 aux personnels (nature `regles.information`, texte neutre fixé à la règle, sans nom d'élève dans le canal externe) | jamais aux familles ni aux élèves |
| `brouillon.preparer` | carte d'une commande **déclarée « préparable par règle »** par son socle (relance de pièce, relance de formulaire, rappel de rendez-vous, convocation à un entretien **non disciplinaire**…) destinée à une personne ou une file de mission | jamais une commande fermée à l'IA (30 §11.3) ; un envoi reste N3 à la confirmation |
| `suivi.ajouter` | ajoute la cible à une liste de suivi (personne ou mission) avec le motif de la règle | retiré à la fin de la période |
| `rappel.planifier` | rappel personnel du 29 au destinataire à une date | — |
| `boite.affecter` | affecte un objet routable (demande, fil de service) à une boîte (futur socle N1, 16) | seulement les objets déclarés routables |

**Effets absents par principe** : écrire dans un domaine (changer une absence, un régime, une note, une classe), sanctionner, envoyer directement à une famille ou un élève, publier, exporter, changer un droit, supprimer.

---

## 12. Simulation et validation

### 12.1 Simulation à blanc [R]

- Rejoue la version sur une **période passée** (défaut : 4 semaines ; au plus l'année en cours), à partir des événements de l'outbox et des faits **à la date** (les socles exposent leurs faits « à une date » ; un fait non historisable est signalé « approximatif »).
- **N'écrit rien** hors de son rapport (`ent_auto_simulation` + lignes agrégées) ; test : aucune tâche, aucun brouillon, aucun événement.
- **Rapport** : nombre d'exécutions, d'effets par type et par destinataire, nombre de cibles distinctes, répartition par jour (pic), effets qui auraient été sautés (destinataire sans droit, gel, plafond), comparaison avec la version active.
- **Nominatif seulement dans la portée du lecteur** : le rédacteur voit le détail des élèves **qu'il peut voir** ; au-delà, des nombres (R-AU15).
- **Seuils d'alerte** : plus de 20 effets par jour en moyenne, plus de 30 % de la population touchée, un destinataire qui reçoit plus de 10 tâches par semaine → avertissement à la validation ; au-delà du double → validation refusée tant que la règle n'est pas resserrée ou plafonnée (S-3102).

### 12.2 Validation [R]

La validatrice voit : la règle en français (générée par le serveur à partir de la structure, pas par un modèle), la simulation, les différences avec la version active, les destinataires, les avertissements ; elle se ré-authentifie (N3) ; la validation est journalisée. Elle peut **valider à l'essai** (« mode observation » : la règle s'exécute et journalise ses effets sans les produire, pendant 1 à 4 semaines, S-3103) — très utile pour une règle sensible.

---

## 13. Exécution, sûreté, gel

### 13.1 Moteur [R]

- **Consommateur de l'outbox** `ConsommateurRegles` (idempotent par `ent_event_traitement`) pour les déclencheurs par événement ; **tâche de fond** `regles.echeances` pour les déclencheurs de calendrier.
- Pour chaque version active dont le déclencheur correspond : population vérifiée **à la date de l'événement**, conditions évaluées par les **ports de faits** des socles (contexte système de l'établissement, lecture seule), fenêtre, limites ; puis les effets, **dans la transaction** de l'exécution (kit transactionnel du 29, création du brouillon) ou en file pour l'information du 8.
- **Journal d'exécution** (ajout seul) : version, déclencheur (événement ou échéance), cible, résultat (`conditions_non_remplies`, `hors_population`, `hors_fenetre`, `deja_fait_periode`, `plafond`, `gel`, `effets_produits`, `effet_saute`, `observation`, `erreur`), faits lus (code, valeur), effets (type, identifiant produit), `correlation_id`.

### 13.2 Garde-fous [R]

| Garde-fou | Règle |
|---|---|
| Idempotence | clé `(version_id, cible, période)` unique |
| Profondeur | un effet de règle ne déclenche pas une autre règle au-delà de la profondeur 2 (le moteur ignore les événements causés par une règle au-delà ; l'outbox borne déjà à 5) |
| Anti-rafale | « une fois par cible et par période » obligatoire ; plafond d'effets par jour et par règle (défaut 50) |
| Plafond par destinataire | au plus 20 tâches par jour issues des règles pour une même personne ; au-delà, une tâche de synthèse « 12 autres élèves concernés » |
| Calendrier | jours ouvrés, vacances, fermetures (5), semaine de rentrée (S-3105), fuseau |
| Plage calme | effets visibles des familles ou élèves différés (R-AU12) |
| Gel | par règle, par automatisme, par élève (socle 8 `notifications.gel`), **général** |
| Droits | destinataire sans droit sur l'objet → effet sauté, journalisé, compteur à la règle |
| Disparition | population, fait, effet ou destinataire disparus → `suspendue_auto` + tâche au propriétaire |
| Erreur | 3 échecs consécutifs → `suspendue_auto` |

### 13.3 Gel général (« mode crise ») [R]

Activé par la direction (N3), avec un motif codé (`deuil`, `evenement_grave`, `greve`, `fermeture`, `autre`) et une fin prévue (au plus 14 jours, prolongeable) : **suspend** les règles de l'établissement, les envois préparés par les automatismes réglables, les relances (18, 25, 3), les rappels non urgents ; **n'arrête pas** les informations dues urgentes (absence inquiétante, sécurité, alerte du 14), ni les automatismes obligatoires internes (expiration des délégations, effacements réglementaires). À la levée : récapitulatif « ce qui a été retenu » et choix **par famille d'automatismes** : rattraper, abandonner (défaut pour les relances), ou laisser reprendre à la prochaine échéance.

---

## 14. Contrôles de cohérence (A15)

Déclarés par les socles, exécutés chaque nuit (tâche `controles.nuit`) et à la demande, ils ouvrent des **anomalies** (une par objet et par contrôle, refermées seules quand la condition disparaît) adressées à une mission. Exemples : élève inscrit sans classe à la date (2/5) ; élève sans responsable joignable (4) ; responsable légal sans foyer (4) ; classe sans professeur principal (6) ; service d'enseignement sans intervenant à J+7 (6/15) ; élève dans un cours qui n'existe plus (5) ; élève interne sans chambre (41) ; élève en sortie et présent à l'appel (12/42) ; régime de cantine sans tarif (24/40) ; PAI expiré (34 : **anomalie adressée à l'infirmerie seulement**, sans contenu ; contrôle intégré au 34, pas une règle) ; compte famille rattaché à un élève sorti (1/4) ; pièce d'identité d'un voyageur expirée avant le départ (42).
Chaque anomalie : objet, contrôle, depuis quand, mission, état (`ouverte`, `prise`, `ignoree` avec motif codé et échéance de réexamen, `close`). Tableau par mission et par socle ; indicateur au 47.

---

## 15. Vigilance précoce (A6) — désactivée par défaut

### 15.1 Principe [R]

Une **liste de suivi** du professeur principal et de la vie scolaire, alimentée par des **règles lisibles** (recettes RC-20 à RC-24), qui signale un **faisceau d'indices** : par exemple « au moins 2 parmi : ≥ 3 demi-journées non justifiées sur 4 semaines ; ≥ 4 retards sur 4 semaines ; ≥ 3 travaux non rendus sur 4 semaines ; baisse de la moyenne de l'élève de plus de 3 points par rapport à la période précédente ; ≥ 2 incidents sur 4 semaines ».

### 15.2 Ce que la vigilance n'est pas [R]

Pas un score, pas un pourcentage, pas une étiquette (« à risque », « décrocheur ») ; pas visible de l'élève ni de la famille (ce n'est pas une information sur eux mais une **invitation à regarder** adressée à un adulte) ; pas transmise à d'autres socles (ni au conseil de classe comme « fait » : le PP peut en parler, le dossier du conseil ne l'importe pas) ; pas conservée au-delà de la période ; pas calculée par un modèle ; pas utilisée pour orienter, sanctionner ou refuser.

### 15.3 Conditions d'activation [R]

AIPD faite (le profilage d'élèves est un critère, §5.1) ; information au conseil d'administration / instance et dans la notice aux familles (« des règles automatiques peuvent signaler à l'équipe un élève à rencontrer ; aucune décision n'est prise automatiquement ») ; règles visibles par toute l'équipe éducative ; bilan annuel (combien de signalements, combien suivis d'un entretien, combien jugés inutiles). **Droit d'opposition** des responsables ou de l'élève majeur : **non** retenu par défaut (la mission d'intérêt public de suivi de l'assiduité le fonde, et l'opposition priverait l'élève d'attention) — **à faire valider par le DPO** (Q-AU3).

---

## 16. Recettes prêtes (D-023) — RC-01 à RC-30

Défaut : **désactivées**, sauf les six marquées ★ (internes, issues de ce que Next.js ou les socles faisaient déjà). Chaque recette se simule avant activation.

| N° | Recette | Effet | Défaut |
|---|---|---|---|
| RC-01 ★ | Justificatif d'absence non reçu 48 h ouvrées après l'absence | carte « relancer la famille » à la vie scolaire (gabarit du 12) | active |
| RC-02 | Absence non justifiée le lundi matin après un week-end, 3 fois en 6 semaines | tâche CPE « échanger avec la famille » | — |
| RC-03 ★ | 3 retards sur 7 jours ouvrés | tâche au PP + liste de suivi vie scolaire | active |
| RC-04 | 2 demi-journées non justifiées dans le mois (avant le seuil légal de 4) | tâche CPE « prévenir avant le seuil » | — |
| RC-05 | Appel non fait 2 fois dans la semaine pour un même cours | information interne à la vie scolaire (pas au professeur publiquement) | — |
| RC-06 ★ | Message d'une famille sans réponse dans une boîte de service depuis 2 jours ouvrés | tâche au responsable de la boîte | active |
| RC-07 | 3 travaux non rendus sur 4 semaines dans un même cours | tâche au professeur du cours | — |
| RC-08 | Élève nouveau dans l'établissement : J+15 | tâche au PP « point d'arrivée » | — |
| RC-09 | Élève qui change de classe en cours d'année | tâche aux nouveaux professeurs « élève arrivé » + au PP | — |
| RC-10 ★ | Pièce du dossier manquante depuis 7 jours | carte de relance groupée par foyer au secrétariat | active |
| RC-11 | Pièce à expiration dans 30 jours (assurance, identité pour un voyage) | carte de relance | — |
| RC-12 | Document à signer non signé depuis 5 jours | carte de relance (7) | — |
| RC-13 ★ | Formulaire non rempli à J-3 de l'échéance | carte de relance au propriétaire du formulaire (18) | active |
| RC-14 | Rendez-vous demandé non traité depuis 3 jours | tâche à la boîte concernée | — |
| RC-15 | Convention de stage non signée à J-10 du début | tâche au référent (39) | — |
| RC-16 ★ | Sortie à J-7 sans autorisations complètes | tâche à l'organisateur (42, NXR-08) | active |
| RC-17 | Séance remplacée non pourvue à J-1 | tâche à la direction adjointe (15) | — |
| RC-18 | Responsable sans moyen de contact valide | tâche au secrétariat (4) | — |
| RC-19 | Échéance impayée de plus de 30 jours **sans** plan ni aide en cours | carte de relance au service comptable (25) | — |
| RC-20 | Vigilance : faisceau de 2 indices sur 5 (§15) | liste de suivi PP + vie scolaire | — (AIPD) |
| RC-21 | Vigilance : baisse de la moyenne de l'élève > 3 points entre deux périodes | liste de suivi PP | — (AIPD) |
| RC-22 | Vigilance : aucun accès de la famille à l'ENT depuis 60 jours **et** absences | tâche CPE « contacter par un autre moyen » | — (AIPD) |
| RC-23 | Vigilance : 2 incidents en 4 semaines | liste de suivi CPE | — (AIPD) |
| RC-24 | Vigilance : élève interne absent à l'internat 2 soirs non prévus | tâche CPE de l'internat | — |
| RC-25 | J-10 avant un conseil de classe : appréciations manquantes | tâche aux professeurs concernés (23) | — |
| RC-26 | J-5 avant la fin d'une période : notes non publiées | tâche aux professeurs (21) | — |
| RC-27 | Chaque lundi : synthèse des tâches en retard de la semaine | information interne à la direction | — |
| RC-28 | Élève devenu majeur | tâche au secrétariat « revoir les droits et contacts » (2) | — |
| RC-29 | Fin de période : listes de suivi remises à zéro, bilan au PP | interne | obligatoire (pas une recette) |
| RC-30 | Demande entrante (futur socle N1) : routage par mots-clés vers une boîte | `boite.affecter` (NXR-09) | — |

---

## 17. IA

### 17.1 Ce que l'IA fait [R]
**Rédiger** une règle à partir d'une phrase (« quand un élève a 3 retards dans la semaine, préviens son PP ») → brouillon structuré, que le serveur traduit en français pour la relecture ; **expliquer** une règle, une simulation, une exécution (« pourquoi Tom est-il dans ma liste ? » → la règle, les faits, les valeurs) ; **proposer** des recettes adaptées à ce que l'établissement décrit.

### 17.2 Ce que l'IA ne fait jamais [R]
Activer, valider, modifier une règle active ; être un **fait** ou une **condition** (aucun « si le message semble urgent ») ; être un **effet** (aucun « rédige et envoie ») ; lire le journal nominatif au-delà de la portée de la personne ; produire un score.

### 17.3 Capacités déclarées (registre `ent-ia`)
Lectures : `regles.catalogue.lire` (C0), `regle.lire` (C0), `regle.simulation.lire` (C1, portée du lecteur), `regle.execution.expliquer` (C1), `automatismes.lire` (C0). Commandes : `AUT-110 regle.brouillon.creer` (N1, origine assistant), `AUT-112 regle.simuler` (N0). **Fermées à l'IA** : valider, activer, suspendre, gel général, paramètres d'automatismes, contrôles « ignorer ».

---

## 18. Droits et minimisation

### 18.1 Actions D-017 [R]

| Action | Portée | Modèle par défaut |
|---|---|---|
| `automatismes.lire` | établissement | tout le personnel (page de transparence) |
| `automatismes.regler` | établissement | administrateur, direction |
| `automatisations.creer` | établissement | direction, CPE, secrétariat |
| `automatisations.valider` | établissement | direction (≠ rédacteur) |
| `automatisations.suspendre` | établissement | direction, propriétaire de la règle |
| `automatisations.journal.lire` | établissement (nominatif dans la portée) | direction, propriétaire |
| `automatisations.gel_general` | établissement | direction |
| `controles.lire`, `controles.traiter` | mission | membres de la mission destinataire |
| `suivi.lire` | moi / mission | PP, vie scolaire |
| `taches_fond.superviser` | plateforme | exploitation ScolIA |

### 18.2 Ce que voit chacun
- Tout le personnel : la page des automatismes et des règles **sans données nominatives**.
- Propriétaire, valideur : simulations et journaux **dans leur portée**.
- Destinataire d'un effet : la tâche, la carte, la liste de suivi, avec « pourquoi » (règle, faits).
- Famille, élève : **rien** des règles internes ; la notice dit qu'elles existent ; l'extrait d'accès (27) inclut les **effets** qui les concernent s'ils ont produit une donnée de leur dossier (une tâche ouverte sur l'élève, une inscription en liste de suivi) [R, DPO].

### 18.3 Minimisation
Faits C0–C2 seulement ; journal : codes et valeurs numériques, pas de texte libre ; titres de tâches neutres (« Échanger avec l'élève », jamais « Tom décroche ») ; listes de suivi effacées en fin de période ; journal nominatif des exécutions purgé à 1 an [H, DPO], agrégats conservés.

---

## 19. Cas limites (catalogue)

| N° | Situation | Comportement |
|---|---|---|
| KR-01 | Rédacteur = valideur | refusé (R-AU13) |
| KR-02 | Valideur sans droit sur une partie de la population (CPE du collège validant une règle sur le lycée) | refusé ; message « vous ne voyez pas toute la population » |
| KR-03 | Propriétaire muté en cours d'année | `suspendue_auto` + tâche à la direction pour transférer |
| KR-04 | Règle sur une classe supprimée à la bascule | `suspendue_auto` |
| KR-05 | Fait retiré par son socle (nouvelle version) | règle suspendue, explication |
| KR-06 | Un événement corrigé (retard annulé) après exécution | l'effet n'est pas défait automatiquement ; la tâche porte « l'événement d'origine a été annulé » (le 29 la ferme si sa nature le permet) |
| KR-07 | Deux règles produisent la même tâche au même destinataire | dédoublonnage par `(nature, objet, destinataire)` du 29 |
| KR-08 | 400 élèves remplissent la condition le jour de la rentrée (reprise de données) | gel de rentrée (S-3105) ; plafond ; synthèse |
| KR-09 | Restauration de la base | effets externes suspendus ; exécutions déjà journalisées non rejouées (E9) |
| KR-10 | Deux instances du serveur | une seule exécute (verrou) |
| KR-11 | Changement d'heure | échéances en heure locale de l'établissement |
| KR-12 | Établissement en outre-mer (fuseau différent) | fuseau du 5 |
| KR-13 | Élève gelé par le socle 8 (situation protégée) | aucun effet le concernant ne part vers l'extérieur ; les tâches internes restent |
| KR-14 | Élève majeur | les cartes de relance vont à lui, pas à ses parents (4) |
| KR-15 | Parents séparés | carte de relance par foyer, selon les droits (4) |
| KR-16 | Règle « mode observation » oubliée | fin automatique au terme ; tâche au valideur |
| KR-17 | Le PP de l'élève n'existe pas (classe sans PP) | effet sauté + anomalie « classe sans PP » (contrôle) |
| KR-18 | Professeur qui ne veut pas des tâches issues d'une règle | il ne peut pas couper une règle de l'établissement ; il peut demander sa révision (tâche au propriétaire) |
| KR-19 | Une règle cible un fait de santé (tentative via un code) | refus à la création (catalogue) et en base (`CHECK` sur la classe) |
| KR-20 | Gel général pendant une échéance de formulaire | relances retenues ; à la levée, « abandonner » par défaut |
| KR-21 | Règle créée par l'assistant avec un fait inexistant | brouillon refusé par la validation de schéma |
| KR-22 | Règle qui provoquerait une boucle (effet → événement → même règle) | profondeur 2 ; et l'effet « tâche » n'est pas un déclencheur ouvert |
| KR-23 | Simulation sur une période où le fait n'était pas historisé | « approximatif » affiché ; validation possible avec avertissement |
| KR-24 | Élève hors IA (30) | sans effet sur le 31 (aucun modèle) |
| KR-25 | Établissement qui veut « prévenir automatiquement les parents à chaque retard » | non par une règle (R-AU2) ; c'est une **information due** du 12/8 si l'établissement l'a choisie, ou une carte de lot quotidienne |

---

## 20. Actions métier (AUT-100 à AUT-223)

Colonnes : acteur et droit (§18.1 ; « CRE » = `automatisations.creer`, « VAL » = `automatisations.valider`, « DIR » = direction, « ADM » = administrateur, « SYS » = système) ; niveau de confirmation ; événement ; lot.

### 20.1 Consulter les automatismes (AUT-100 à AUT-109)

| N° | Action | Acteur | Règles | Niveau | Événement | Lot |
|---|---|---|---|---|---|---|
| AUT-100 | Ouvrir « Ce que ScolIA fait automatiquement ici » | personnel (`automatismes.lire`) | sans nominatif | N0 | — | AU2 |
| AUT-101 | Filtrer par socle, public touché (interne / familles / élèves), état | idem | — | N0 | — | AU2 |
| AUT-102 | Voir la fiche d'un automatisme (déclencheur, effet, paramètres, dernière et prochaine exécution, compteurs 30 jours) | idem | — | N0 | — | AU2 |
| AUT-103 | Voir le journal agrégé d'un automatisme | idem | nominatif dans la portée seulement | N0 | — | AU2 |
| AUT-104 | Voir l'état du gel général | idem | — | N0 | — | AU5 |
| AUT-105 | Voir les échecs des tâches de fond de l'établissement | ADM | — | N0 | — | AU1 |
| AUT-106 | Exporter le registre des automatismes (CA, DPO) | DIR, ADM | PDF et CSV | N0 | — | AU2 |
| AUT-107 | Voir « pourquoi ai-je reçu cette tâche ? » (automatisme ou règle, faits) | destinataire | — | N0 | — | AU3 |
| AUT-108 | Voir les automatismes qui me concernent comme destinataire | personnel | — | N0 | — | AU2 |
| AUT-109 | Comparer l'activité des automatismes d'un mois sur l'autre | DIR | agrégats | N0 | — | AU10 |

### 20.2 Régler les automatismes intégrés (AUT-110 à AUT-119 ; AUT-110 et AUT-112 sont aussi ouverts à l'assistant, §17.3)

| N° | Action | Acteur | Règles | Niveau | Événement | Lot |
|---|---|---|---|---|---|---|
| AUT-110 | Créer un brouillon de règle (écran ou assistant) | CRE | §10 ; origine notée | N1 | `regle.brouillon_cree` | AU3 |
| AUT-111 | Modifier un paramètre d'automatisme dans ses bornes | `automatismes.regler` | journalisé ; effet à la prochaine échéance | N2 | `automatisme.regle` | AU2 |
| AUT-112 | Simuler une règle sur une période | CRE, VAL | §12.1 | N0 | `regle.simulee` | AU4 |
| AUT-113 | Désactiver un automatisme désactivable | `automatismes.regler` | interdit pour les obligatoires | N2 | `automatisme.desactive` | AU2 |
| AUT-114 | Réactiver un automatisme | idem | — | N2 | `automatisme.active` | AU2 |
| AUT-115 | Geler un automatisme pour une durée | `automatismes.regler` | ≤ 30 jours | N2 | `automatisme.gele` | AU5 |
| AUT-116 | Lever un gel d'automatisme | idem | rattraper / abandonner | N2 | `automatisme.degele` | AU5 |
| AUT-117 | Rétablir les paramètres par défaut | idem | — | N2 | — | AU2 |
| AUT-118 | Voir l'historique des réglages d'un automatisme | `automatismes.lire` | — | N0 | — | AU2 |
| AUT-119 | Choisir le destinataire (mission) d'un automatisme intégré quand le socle le permet | `automatismes.regler` | missions du 6 | N2 | — | AU2 |

### 20.3 Composer une règle (AUT-120 à AUT-144)

| N° | Action | Acteur | Règles | Niveau | Événement | Lot |
|---|---|---|---|---|---|---|
| AUT-120 | Partir d'une recette (RC-xx) | CRE | copie modifiable | N1 | `regle.brouillon_cree` | AU6 |
| AUT-121 | Partir d'une règle existante (dupliquer) | CRE | — | N1 | idem | AU3 |
| AUT-122 | Nommer et décrire la règle | CRE | nom ≤ 80, description ≤ 500 | N1 | — | AU3 |
| AUT-123 | Choisir le déclencheur (événement ou échéance) | CRE | catalogue §11.1 | N1 | — | AU3 |
| AUT-124 | Choisir la population (tous, niveau, classe, groupe, régime, internes, nouveaux) | CRE | dans sa portée | N1 | — | AU3 |
| AUT-125 | Exclure une partie de la population (une classe, les élèves en stage) | CRE | — | N1 | — | AU3 |
| AUT-126 | Ajouter une condition sur un fait (opérateur, seuil, fenêtre glissante) | CRE | ≤ 5 ; C0–C2 | N1 | — | AU3 |
| AUT-127 | Choisir « toutes les conditions » ou « au moins N parmi » | CRE | — | N1 | — | AU3 |
| AUT-128 | Définir la fenêtre (jours ouvrés, hors vacances, période de l'année, heures) | CRE | — | N1 | — | AU3 |
| AUT-129 | Ajouter un effet « créer une tâche » (destinataire par fonction ou mission, titre neutre, échéance) | CRE | §11.3 | N1 | — | AU3 |
| AUT-130 | Ajouter un effet « information interne » (texte neutre fixé) | CRE | personnels seulement | N1 | — | AU3 |
| AUT-131 | Ajouter un effet « préparer une carte » (commande préparable par règle, destinataire) | CRE | jamais une commande fermée | N1 | — | AU7 |
| AUT-132 | Ajouter un effet « ajouter à une liste de suivi » | CRE | — | N1 | — | AU8 |
| AUT-133 | Ajouter un effet « planifier un rappel » | CRE | — | N1 | — | AU3 |
| AUT-134 | Ajouter un effet « affecter à une boîte » (objets routables) | CRE | futur socle N1, 16 | N1 | — | AU11 |
| AUT-135 | Fixer les limites (une fois par cible et par période, plafond par jour) | CRE | obligatoires | N1 | — | AU3 |
| AUT-136 | Lire la règle « en français » générée par le serveur | CRE, VAL | pas par un modèle | N0 | — | AU3 |
| AUT-137 | Vérifier la règle (cohérence, faits existants, destinataires possibles) | CRE | erreurs bloquantes et avertissements | N0 | — | AU3 |
| AUT-138 | Soumettre à validation | CRE | simulation récente exigée (≤ 7 jours) | N1 | `regle.soumise` | AU4 |
| AUT-139 | Retirer une soumission | CRE | — | N1 | — | AU4 |
| AUT-140 | Rédiger une règle avec l'assistant (« quand… alors… » en phrase) | CRE | tâche `regle.rediger` du 30 ; brouillon seulement | N1 | `regle.brouillon_cree` | AU9 |
| AUT-141 | Transférer la propriété d'une règle | propriétaire, DIR | le nouveau propriétaire accepte | N2 | `regle.proprietaire_change` | AU3 |
| AUT-142 | Supprimer un brouillon jamais activé | propriétaire | — | N1 | — | AU3 |
| AUT-143 | Créer une nouvelle version d'une règle active | CRE | la version active continue | N1 | `regle.version_creee` | AU4 |
| AUT-144 | Comparer deux versions | CRE, VAL | — | N0 | — | AU4 |

### 20.4 Valider, activer, suspendre (AUT-145 à AUT-164)

| N° | Action | Acteur | Règles | Niveau | Événement | Lot |
|---|---|---|---|---|---|---|
| AUT-145 | Examiner une règle soumise (règle en français, simulation, avertissements, destinataires) | VAL | — | N0 | — | AU4 |
| AUT-146 | Valider et activer | VAL ≠ rédacteur | ré-authentification ; droits du valideur sur la population | N3 | `regle.activee` | AU4 |
| AUT-147 | Valider en mode observation (1–4 semaines) | VAL | effets journalisés, non produits | N3 | `regle.observation` | AU4 |
| AUT-148 | Passer de l'observation à l'activation | VAL | rapport de l'observation affiché | N3 | `regle.activee` | AU4 |
| AUT-149 | Refuser une règle, avec motif | VAL | retour en brouillon | N2 | `regle.refusee` | AU4 |
| AUT-150 | Demander une modification | VAL | — | N2 | — | AU4 |
| AUT-151 | Suspendre une règle | `automatisations.suspendre` | effet immédiat | N2 | `regle.suspendue` | AU4 |
| AUT-152 | Reprendre une règle suspendue | VAL | revalidation si suspendue plus de 30 jours | N3 | `regle.reprise` | AU4 |
| AUT-153 | Reprendre une règle `suspendue_auto` après correction | VAL | la cause doit être levée | N3 | `regle.reprise` | AU4 |
| AUT-154 | Archiver une règle | propriétaire, DIR | journal conservé | N2 | `regle.archivee` | AU4 |
| AUT-155 | Désarchiver (nouvelle version en brouillon) | CRE | — | N1 | — | AU4 |
| AUT-156 | Activer une recette ★ par défaut à la mise en service | DIR | §16 ; simulation sur les données reprises | N3 | `regle.activee` | AU6 |
| AUT-157 | Voir les règles en attente de validation | VAL | à-faire du 29 | N0 | — | AU4 |
| AUT-158 | Voir les règles suspendues automatiquement et leur cause | DIR, propriétaire | — | N0 | — | AU4 |
| AUT-159 | Recevoir le bilan mensuel d'une règle (effets, tâches traitées, ignorées) | propriétaire | — | N0 | — | AU10 |
| AUT-160 | Proposer l'archivage des règles inutiles (bilan annuel) | SYS | règle dont < 10 % des tâches sont traitées | — | — | AU10 |
| AUT-161 | Plafonner une règle à la validation (effets par jour) | VAL | — | N2 | — | AU4 |
| AUT-162 | Restreindre la population à la validation | VAL | crée une nouvelle version | N2 | — | AU4 |
| AUT-163 | Prolonger le mode observation | VAL | ≤ 4 semaines au total | N2 | — | AU4 |
| AUT-164 | Lire le journal nominatif d'une règle dans sa portée | `automatisations.journal.lire` | — | N0 | — | AU3 |

### 20.5 Recevoir et traiter les effets (AUT-165 à AUT-179)

| N° | Action | Acteur | Règles | Niveau | Événement | Lot |
|---|---|---|---|---|---|---|
| AUT-165 | Recevoir une tâche issue d'une règle (à-faire du 29) | destinataire | titre neutre, lien, « pourquoi » | — | `afaire.ouverte` (29) | AU3 |
| AUT-166 | Traiter, reporter, déléguer une tâche issue d'une règle | destinataire | règles du 29 | — | (29) | AU3 |
| AUT-167 | Marquer une tâche « non pertinente » (retour à la règle) | destinataire | compteur au bilan | N1 | `regle.retour` | AU10 |
| AUT-168 | Demander la révision d'une règle | destinataire | tâche au propriétaire | N1 | — | AU10 |
| AUT-169 | Confirmer une carte préparée par une règle (relance de lot) | destinataire (ou membre de la mission) | N3 pour un envoi, aperçu complet | N3 | (30, socle) | AU7 |
| AUT-170 | Abandonner une carte préparée par une règle | idem | — | N1 | (30) | AU7 |
| AUT-171 | Consulter sa liste de suivi | PP, vie scolaire | élèves de la portée ; motifs | N0 | — | AU8 |
| AUT-172 | Retirer un élève de sa liste de suivi, avec motif codé (« vu en entretien ») | PP, vie scolaire | ne revient pas avant la période suivante sauf nouveau fait | N1 | — | AU8 |
| AUT-173 | Noter un entretien fait depuis la liste (renvoi au 13 / 19) | PP, CPE | — | N1 | — | AU8 |
| AUT-174 | Voir les règles qui ont placé un élève dans sa liste | PP, vie scolaire | faits et valeurs | N0 | — | AU8 |
| AUT-175 | Recevoir le rappel planifié par une règle | destinataire | — | — | — | AU3 |
| AUT-176 | Recevoir l'information interne d'une règle | destinataire | neutre | — | (8) | AU3 |
| AUT-177 | Prendre en charge un objet affecté à une boîte par une règle | membre de la boîte | 16 / N1 | — | — | AU11 |
| AUT-178 | Réaffecter un objet mal routé (retour à la règle) | membre de la boîte | compteur | N1 | — | AU11 |
| AUT-179 | Voir combien de tâches lui viennent des règles cette semaine | personnel | — | N0 | — | AU10 |

### 20.6 Gel et crise (AUT-180 à AUT-187)

| N° | Action | Acteur | Règles | Niveau | Événement | Lot |
|---|---|---|---|---|---|---|
| AUT-180 | Activer le gel général (motif codé, fin prévue ≤ 14 jours) | `automatisations.gel_general` | §13.3 | N3 | `automatisations.gel_general` | AU5 |
| AUT-181 | Prolonger le gel général | idem | — | N3 | — | AU5 |
| AUT-182 | Lever le gel général ; choisir par famille : rattraper, abandonner, reprendre | idem | récapitulatif | N3 | `automatisations.degel_general` | AU5 |
| AUT-183 | Geler les automatismes pour un élève (situation particulière) | renvoi au gel du socle 8 | — | N2 | (8) | AU5 |
| AUT-184 | Geler une population (une classe en voyage, en stage) pour une période | DIR, CPE | règles et relances internes seulement | N2 | — | AU5 |
| AUT-185 | Régler la semaine de rentrée sans règles d'assiduité (S-3105) | DIR | — | N2 | — | AU5 |
| AUT-186 | Voir ce qui a été retenu pendant un gel | DIR | — | N0 | — | AU5 |
| AUT-187 | Être prévenu de la fin imminente d'un gel (J-1) | DIR | — | — | — | AU5 |

### 20.7 Contrôles de cohérence (AUT-188 à AUT-199)

| N° | Action | Acteur | Règles | Niveau | Événement | Lot |
|---|---|---|---|---|---|---|
| AUT-188 | Voir les anomalies ouvertes de sa mission | `controles.lire` | — | N0 | — | AU8 |
| AUT-189 | Prendre une anomalie | `controles.traiter` | — | N1 | — | AU8 |
| AUT-190 | Ouvrir l'objet pour corriger (écran du socle) | idem | la correction se fait dans le socle | N0 | — | AU8 |
| AUT-191 | Ignorer une anomalie avec motif et date de réexamen | idem | ≤ 1 an | N2 | `controle.ignore` | AU8 |
| AUT-192 | Lancer un contrôle à la demande | ADM, mission | — | N1 | — | AU8 |
| AUT-193 | Désactiver un contrôle non obligatoire | `automatismes.regler` | — | N2 | — | AU8 |
| AUT-194 | Changer la mission destinataire d'un contrôle | idem | — | N2 | — | AU8 |
| AUT-195 | Voir le tableau des anomalies par socle et par ancienneté | DIR, ADM | — | N0 | — | AU8 |
| AUT-196 | Clore automatiquement une anomalie quand la condition disparaît | SYS | — | — | `controle.clos` | AU8 |
| AUT-197 | Rouvrir une anomalie ignorée à sa date de réexamen | SYS | — | — | — | AU8 |
| AUT-198 | Contrôle avant bascule d'année (liste bloquante) | ADM | socle 5 | N0 | — | AU8 |
| AUT-199 | Contrôle avant export officiel (renvoi au socle 9) | ADM | pré-contrôle des exports (T-28) | N0 | — | AU8 |

### 20.8 Système et plateforme (AUT-200 à AUT-223)

| N° | Action | Acteur | Règles | Niveau | Événement | Lot |
|---|---|---|---|---|---|---|
| AUT-200 | Déclarer un automatisme ou une tâche de fond (kit) | développeur ScolIA | test CI : rien de non déclaré | — | — | AU1 |
| AUT-201 | Ordonnancer les tâches de fond par établissement dans son fuseau | SYS | §9 | — | — | AU1 |
| AUT-202 | Réclamer une tâche (une seule instance) | SYS | `SKIP LOCKED` | — | — | AU1 |
| AUT-203 | Journaliser chaque passage (début, fin, résultat, compteurs, erreur codée) | SYS | — | — | — | AU1 |
| AUT-204 | Retenter un passage échoué (3 fois), puis marquer « en échec » | SYS | tâche à l'administrateur | — | `tache_fond.echec` | AU1 |
| AUT-205 | Suspendre les effets externes après une restauration (E9) | SYS | levée par l'exploitation | — | `automatisations.restauration` | AU1 |
| AUT-206 | Lever la suspension post-restauration | exploitation | établissement par établissement | — | — | AU1 |
| AUT-207 | Superviser les tâches de fond de tous les établissements (sans données métier) | `taches_fond.superviser` | — | — | — | AU1 |
| AUT-208 | Évaluer une règle sur un événement (consommateur de l'outbox) | SYS | §13.1 | — | — | AU3 |
| AUT-209 | Évaluer les règles à échéance de calendrier | SYS | — | — | — | AU3 |
| AUT-210 | Lire un fait par le port de son socle (contexte système, lecture seule) | SYS | — | — | — | AU3 |
| AUT-211 | Produire les effets dans la transaction de l'exécution | SYS | — | — | effets | AU3 |
| AUT-212 | Sauter un effet (droit absent, gel, plafond) et le journaliser | SYS | — | — | — | AU3 |
| AUT-213 | Différer un effet visible des familles à la fin de la plage calme | SYS | R-AU12 | — | — | AU3 |
| AUT-214 | Suspendre automatiquement une règle (disparition, erreurs, plafond répété) | SYS | tâche au propriétaire | — | `regle.suspendue_auto` | AU4 |
| AUT-215 | Recalculer les populations à la bascule d'année | SYS | règles orphelines suspendues | — | — | AU4 |
| AUT-216 | Remettre à zéro les listes de suivi en fin de période | SYS | bilan au PP | — | — | AU8 |
| AUT-217 | Purger le journal nominatif des exécutions (1 an) | SYS (tâche du 27) | agrégats gardés | — | — | AU10 |
| AUT-218 | Calculer les bilans mensuels et annuels | SYS | — | — | — | AU10 |
| AUT-219 | Publier les indicateurs (47) | SYS | — | — | — | AU10 |
| AUT-220 | Exécuter la simulation (lecture seule, rapport) | SYS | aucune écriture hors rapport | — | — | AU4 |
| AUT-221 | Retirer `ent-brain` du démarrage et du dépôt | développeur | AU0 | — | — | AU0 |
| AUT-222 | Brancher les automatismes existants sur le registre (main.rs → ordonnanceur) | développeur | sans changement de comportement | — | — | AU1 |
| AUT-223 | Fermer la mode observation à son terme | SYS | tâche au valideur | — | — | AU4 |

**Total : 124 actions** (AUT-100 à AUT-223).

---

## 21. Règles transverses (R-AU1 à R-AU35) [R]

| N° | Règle |
|---|---|
| R-AU1 | Aucun automatisme non déclaré ne tourne (test CI). |
| **R-AU2** | Une règle de l'établissement ne produit **jamais** seule un effet sur une personne extérieure (famille, élève, tiers) : un envoi préparé reste une carte confirmée par une personne (N3) ; ni sanction, ni note, ni décision. |
| R-AU3 | Une règle n'écrit dans aucun domaine ; ses effets sont tâches, informations internes, cartes, listes de suivi, rappels, affectations à une boîte. |
| **R-AU4** | Aucun modèle de langage, aucun apprentissage, aucun score dans le moteur ; tout est déterministe et explicable. |
| R-AU5 | Chaque exécution sait dire pourquoi (version, faits, valeurs). |
| **R-AU6** | Aucun fait de classe C3 ou C4 (santé, handicap détaillé, social, protection de l'enfance, confidentialité renforcée) dans une règle de l'établissement ; contrôlé par le catalogue et en base. |
| R-AU7 | Les automatismes obligatoires (réglementaires, sécurité) ne sont ni désactivables ni gelables par une règle. |
| R-AU8 | Aucune règle ne prépare une sanction ; elle peut seulement créer une tâche « examiner la situation ». |
| R-AU9 | Les seuils réglementaires d'assiduité sont des automatismes du 12 ; une règle peut ajouter une vigilance, jamais en retirer une. |
| R-AU10 | Les informations dues (absence signalée aux responsables, R.131-5) ne passent pas par les règles et ne sont pas bloquées par elles. |
| R-AU11 | Le calendrier de l'établissement (jours ouvrés, vacances, fermetures, fuseau) s'applique à toute échéance. |
| R-AU12 | Tout effet visible d'une famille ou d'un élève est différé à la fin de la plage calme (20 h–7 h, week-end ; arrêté du 7/09/2026 dans le public). |
| **R-AU13** | Le valideur d'une règle n'est pas son rédacteur. |
| R-AU14 | Le valideur doit avoir, à la validation, les droits de voir ce que les effets montreront ; la règle ne donne aucun droit à quiconque. |
| R-AU15 | Une simulation n'est nominative que dans la portée de son lecteur. |
| R-AU16 | Simulation récente (≤ 7 jours) obligatoire avant la soumission ; règle trop bavarde refusée. |
| R-AU17 | Une modification d'une règle active est une nouvelle version, à revalider. |
| R-AU18 | « Une fois par cible et par période » et un plafond quotidien sont obligatoires. |
| R-AU19 | Profondeur de chaîne : un effet de règle ne déclenche pas d'autre règle au-delà de 2. |
| R-AU20 | Destinataires par fonction ou mission de préférence ; destinataire sans droit = effet sauté et journalisé. |
| R-AU21 | Propriétaire ou valideur parti, population ou fait disparu, erreurs répétées → suspension automatique et tâche. |
| R-AU22 | Le gel général n'arrête ni les informations dues urgentes, ni les automatismes obligatoires internes. |
| R-AU23 | Une tâche de fond s'exécute sur une seule instance à la fois. |
| R-AU24 | Une tâche de fond s'exécute par établissement, dans le contexte système nommé, sous RLS. |
| R-AU25 | Après une restauration, les effets externes sont suspendus jusqu'à une levée explicite (E9). |
| R-AU26 | Aucune purge ni effet sur une lecture (pas d'effet de bord d'un GET). |
| R-AU27 | Aucune route « cron » exposée sur le réseau. |
| R-AU28 | Titres de tâches et textes d'information neutres ; aucun motif ni jugement sur un élève dans un canal externe. |
| R-AU29 | La correction d'un événement source ne défait pas un effet ; elle l'annote. |
| **R-AU30** | La vigilance précoce (profilage d'élèves) est désactivée par défaut et n'est activable qu'après AIPD et information. |
| R-AU31 | Une liste de suivi n'est pas une étiquette : ni score, ni catégorie, ni visibilité par l'élève ou la famille, ni transmission à un autre socle. |
| R-AU32 | Les listes de suivi sont remises à zéro à chaque période ; le journal nominatif des exécutions est purgé à 1 an. |
| R-AU33 | L'IA peut rédiger et expliquer une règle ; elle ne la valide, ne l'active, ni n'y figure jamais. |
| R-AU34 | Les recettes sont désactivées par défaut, sauf les six internes ★, et toujours simulées avant activation. |
| R-AU35 | Une règle ne s'applique qu'aux objets de son établissement (D-013) ; aucune règle « réseau ». |

---

## 22. Événements

### 22.1 Émis
`regle.brouillon_cree`, `regle.soumise`, `regle.simulee`, `regle.activee`, `regle.observation`, `regle.refusee`, `regle.suspendue`, `regle.suspendue_auto`, `regle.reprise`, `regle.archivee`, `regle.version_creee`, `regle.proprietaire_change`, `regle.retour` · `automatisme.regle`, `automatisme.active`, `automatisme.desactive`, `automatisme.gele`, `automatisme.degele` · `automatisations.gel_general`, `automatisations.degel_general`, `automatisations.restauration` · `tache_fond.echec` · `controle.ouvert`, `controle.ignore`, `controle.clos`. **Les effets** (tâches, brouillons, informations) émettent les événements de leurs socles, avec `causation_id` = l'événement déclencheur et une marque `origine = regle:<version>`. Un événement causé par une règle **n'est pas** un déclencheur ouvert au-delà de la profondeur 2.

### 22.2 Consommés
Tous les déclencheurs ouverts (§11.1) ; `annee.basculee` (5), `agent.depart` / `mission.retiree` (6), `notifications.gel` (8), `fait.retire` (déclaration de socle, au déploiement), `rgpd.effacement` (27).

### 22.3 Notifications (natures du socle 8)
`regles.information` (effet interne, neutre) · `regles.a_valider` (valideur) · `regles.suspendue_auto` (propriétaire, direction) · `automatisations.gel_fin_proche` (direction) · `tache_fond.echec` (administrateur). Aucune nature vers les familles ou les élèves.

---

## 23. Écrans (`DESIGN_SYSTEM.md`, `FRONTEND_CIBLE.md`)

1. **Automatismes** (tuiles Bento) : bandeau du gel général ; tuiles « intégrés » par socle, « règles de l'établissement », « recettes », « contrôles », « tâches de fond » (administrateur) ; chaque tuile : état, compteurs, échecs.
2. **Fiche d'un automatisme** : description, paramètres (formulaire borné), historique, journal agrégé.
3. **Composer une règle** : cinq blocs verticaux (Quand / Pour / Si / Dans / Alors) avec listes fermées ; la **règle en français** se met à jour à droite ; bouton « simuler » ; aucune saisie libre hors nom, description, titres neutres.
4. **Simulation** : courbe des effets par jour, tableau par destinataire, cibles (nominatif dans la portée), avertissements ; comparer à la version active.
5. **Validation** : vue de lecture, différences, ré-authentification.
6. **Liste de suivi** (PP, vie scolaire) : élèves, motifs (règle, faits), dernier entretien, « retirer », sans couleur rouge ni libellé de risque.
7. **Anomalies** : tableau dense par mission, ancienneté, actions.
8. **Mobile** : liste de suivi et tâches seulement (socle 11) ; aucune composition de règle sur mobile.
Accessibilité : RGAA (29 §5.2), clavier, 375 px, aucune information portée par la couleur seule.

---

## 24. Modèle de données cible (brique `automatisations`, crate `ent-auto`) [R]

**Règles appliquées à toutes les tables** (E1–E14, D-036) : UUID générés par l'application ; `etablissement_id NOT NULL` partout, tables filles comprises (sauf les tables de **plateforme** listées au §25, sans donnée d'établissement) ; `UNIQUE (etablissement_id, id)` et FK composites ; `cree_le`, `modifie_le` ; RLS standard puis stricte ; **aucun JSON, aucun tableau, aucun BYTEA** ; codes `TEXT` + `CHECK` ; tables de journal en ajout seul. Les catalogues (automatismes intégrés, tâches de fond, déclencheurs, faits, effets, recettes, contrôles) sont **dans le code** (compilés, revus) : la base ne stocke que les **choix** de l'établissement et les **traces**.

### 24.1 Tâches de fond (AU1)

| Table | Colonnes principales | Remarques |
|---|---|---|
| `ent_auto_planif` | `etablissement_id`, `id`, `tache_code`, `prochaine_echeance`, `derniere_execution`, `etat` (`active`, `gelee`, `en_echec`, `suspendue_restauration`), `echecs_consecutifs`, `reclamee_par`, `reclamee_jusqu_au` | une ligne par tâche × établissement ; réclamation `FOR UPDATE SKIP LOCKED` ; **volatile** pour l'export, neutralisée à la restauration (E9) |
| `ent_auto_passage` | `etablissement_id`, `id`, `tache_code`, `debut`, `fin`, `resultat` (`succes`, `echec`, `saute_gel`, `saute_restauration`), `objets_traites`, `erreur_code`, `instance` | ajout seul ; purge à 90 jours [H] |
| `ent_plateforme_tache_passage` | `id`, `tache_code`, `debut`, `fin`, `resultat`, `instance` | **plateforme**, sans donnée d'établissement (passages globaux : sélection des établissements) |

### 24.2 Réglages des automatismes intégrés (AU2)

| Table | Colonnes principales | Remarques |
|---|---|---|
| `ent_auto_reglage` | `etablissement_id`, `id`, `automatisme_code`, `actif`, `mission_destinataire_id` (NULL = défaut), `modifie_par`, `modifie_le` | `automatisme_code` connu du code ; obligatoire → `actif` forcé vrai par le service |
| `ent_auto_reglage_param` | `etablissement_id`, `id`, `reglage_id`, `param_code`, `type` (`entier`, `duree_minutes`, `heure`, `booleen`, `code`), `valeur_entier`, `valeur_heure`, `valeur_booleen`, `valeur_code` | une seule colonne de valeur (`CHECK`) ; bornes vérifiées par le service |
| `ent_auto_gel` | `etablissement_id`, `id`, `portee` (`general`, `automatisme`, `regle`, `population`), `automatisme_code`, `regle_id`, `population_type`, `population_id`, `motif` (code), `debut`, `fin_prevue`, `leve_le`, `leve_par`, `choix_levee` (`rattraper`, `abandonner`, `reprendre`) | `CHECK` de cohérence portée / colonnes ; un seul gel général ouvert (index unique partiel) ; le gel d'un élève reste celui du socle 8 |

### 24.3 Règles (AU3–AU4)

| Table | Colonnes principales | Remarques |
|---|---|---|
| `ent_regle` | `etablissement_id`, `id`, `nom`, `description`, `proprietaire_user_id`, `recette_code` (NULL si création libre), `etat` (`brouillon`, `a_valider`, `observation`, `active`, `suspendue`, `suspendue_auto`, `archivee`), `version_active_id`, `motif_suspension` (code), `origine` (`ecran`, `assistant`, `recette`) | |
| `ent_regle_version` | `etablissement_id`, `id`, `regle_id`, `numero`, `declencheur_code`, `echeance_type`, `echeance_heure`, `echeance_jour`, `mode_conditions` (`toutes`, `au_moins`), `au_moins_n`, `jours_ouvres_seulement`, `hors_vacances`, `periode_debut` (mois-jour), `periode_fin`, `heure_debut`, `heure_fin`, `periodicite_cible` (`jour`, `semaine`, `mois`, `periode`, `annee`), `plafond_jour`, `redigee_par`, `soumise_le`, `validee_par`, `validee_le`, `observation_jusqu_au`, `simulation_id` | **immuable** après soumission (déclencheur) ; `CHECK (validee_par <> redigee_par)` |
| `ent_regle_population` | `etablissement_id`, `id`, `version_id`, `inclure` (bool), `type` (`tous`, `niveau`, `classe`, `groupe`, `cours`, `regime`, `internes`, `nouveaux`, `majeurs`), `objet_id` | FK composites vers le socle 5 quand `objet_id` est renseigné |
| `ent_regle_condition` | `etablissement_id`, `id`, `version_id`, `rang` (1–5), `fait_code`, `operateur` (`=`, `≠`, `<`, `≤`, `>`, `≥`, `vrai`, `faux`, `dans`), `fenetre_jours`, `valeur_entier`, `valeur_duree_minutes`, `valeur_booleen`, `valeur_code` | une seule valeur (`CHECK`) ; `fait_code` connu du code, **classe C0–C2** (contrôle du service et `CHECK` sur une colonne `classe_fait` recopiée du catalogue) |
| `ent_regle_condition_code` | `etablissement_id`, `id`, `condition_id`, `code` | valeurs de l'opérateur `dans` (au lieu d'un tableau) |
| `ent_regle_effet` | `etablissement_id`, `id`, `version_id`, `rang` (1–3), `effet_code`, `destinataire_type` (`fonction`, `mission`, `personne`, `boite`), `destinataire_fonction` (code : `pp`, `cpe_niveau`, `vie_scolaire`, `secretariat`, `direction`, `professeur_du_cours`, `referent_stage`, `organisateur`), `destinataire_id`, `titre_neutre`, `echeance_jours`, `commande_code` (pour `brouillon.preparer`), `liste_suivi_id` | `titre_neutre` ≤ 80 caractères |
| `ent_regle_effet_param` | `etablissement_id`, `id`, `effet_id`, `param_code`, `type`, `valeur_*` | paramètres typés d'une commande préparée |

### 24.4 Simulation, exécutions (AU3–AU4)

| Table | Colonnes principales | Remarques |
|---|---|---|
| `ent_auto_simulation` | `etablissement_id`, `id`, `version_id`, `lancee_par`, `periode_debut`, `periode_fin`, `executions`, `effets`, `cibles_distinctes`, `pic_jour`, `effets_sautes`, `approximatif` (bool), `avertissement` (code), `cree_le` | rapport figé |
| `ent_auto_simulation_ligne` | `etablissement_id`, `id`, `simulation_id`, `jour`, `effet_code`, `destinataire_ref`, `nombre` | agrégats ; le nominatif est **recalculé à l'affichage** dans la portée du lecteur, jamais stocké |
| `ent_regle_execution` | `etablissement_id`, `id`, `version_id`, `declencheur_event_id` ou `echeance`, `cible_type`, `cible_id`, `periode_cle`, `resultat` (codes §13.1), `observation` (bool), `correlation_id`, `cree_le` | ajout seul ; **clé unique** `(etablissement_id, version_id, cible_type, cible_id, periode_cle)` = idempotence ; détail nominatif purgé à 1 an |
| `ent_regle_execution_fait` | `etablissement_id`, `id`, `execution_id`, `fait_code`, `valeur_entier`, `valeur_duree_minutes`, `valeur_booleen`, `valeur_code` | les valeurs lues (explicabilité) |
| `ent_regle_execution_effet` | `etablissement_id`, `id`, `execution_id`, `effet_id`, `resultat` (`produit`, `saute_droit`, `saute_gel`, `saute_plafond`, `differe_plage_calme`, `observation`), `objet_type`, `objet_id` | lien vers la tâche, le brouillon, l'information |
| `ent_regle_retour` | `etablissement_id`, `id`, `execution_id`, `par_user_id`, `retour` (`utile`, `non_pertinent`, `demande_revision`) | |

### 24.5 Listes de suivi et contrôles (AU8)

| Table | Colonnes principales | Remarques |
|---|---|---|
| `ent_suivi_liste` | `etablissement_id`, `id`, `nom`, `proprietaire_type` (`personne`, `mission`, `fonction_classe`), `proprietaire_id`, `periode_id` (socle 5) | |
| `ent_suivi_entree` | `etablissement_id`, `id`, `liste_id`, `eleve_id` (FK composite), `ajoutee_le`, `execution_id`, `retiree_le`, `retiree_par`, `motif_retrait` (code) | **effacée** à la fin de la période (R-AU32) ; RLS : membres de la liste ayant l'élève dans leur portée |
| `ent_controle_reglage` | `etablissement_id`, `id`, `controle_code`, `actif`, `mission_id` | |
| `ent_controle_anomalie` | `etablissement_id`, `id`, `controle_code`, `objet_type`, `objet_id`, `ouverte_le`, `etat` (`ouverte`, `prise`, `ignoree`, `close`), `prise_par`, `ignoree_motif` (code), `reexamen_le`, `close_le` | unique partiel `(controle_code, objet_id)` tant que non close ; **reconstructible** |

### 24.6 Ce qui disparaît ou change ailleurs
`ent-brain` (crate et appels) disparaît (AU0) ; les boucles de `main.rs` deviennent des déclarations de tâches de fond (AU1) ; `effacer_sanctions_echues` (13) et les relances de finances (25, `JobsFinances`) deviennent des tâches déclarées ; le contexte système `JobPlanifie(nom)` est réutilisé, une variante par tâche déclarée (`RLS_STRICTE.md` §4.1, ligne à ajouter par tâche).

---

## 25. Registre des tables (E12) et restauration

| Table | Portée | Export | À la restauration |
|---|---|---|---|
| `ent_auto_planif` | `etablissement` | aucune (volatile) | **neutralisée** : `etat = suspendue_restauration` pour les tâches à effet externe (E9) |
| `ent_auto_passage` | `etablissement` | aucune | gardée |
| `ent_plateforme_tache_passage` | `plateforme` | aucune | gardée |
| `ent_auto_reglage*`, `ent_auto_gel`, `ent_regle*` (règles, versions, populations, conditions, effets) | `etablissement` | simple | gardées ; règles actives **repassent en observation** 24 h (R-AU25) |
| `ent_auto_simulation*` | `etablissement` | aucune | gardées |
| `ent_regle_execution*`, `ent_regle_retour` | `etablissement` | simple (agrégats), détail particulier | gardées ; **aucune exécution rejouée** |
| `ent_suivi_*` | `etablissement` | particulière | gardées |
| `ent_controle_*` | `etablissement` | aucune | anomalies **reconstruites** au prochain passage |

---

## 26. Migrations (plage imposée 2620–2639)

| N° | Nom | Contenu | Lot |
|---|---|---|---|
| 2620 | `2620_auto_planif` | `ent_auto_planif`, `ent_auto_passage` (ajout seul), `ent_plateforme_tache_passage`, RLS | AU1 |
| 2621 | `2621_auto_reglages` | `ent_auto_reglage`, `ent_auto_reglage_param` | AU2 |
| 2622 | `2622_auto_gel` | `ent_auto_gel`, index unique partiel du gel général | AU5 |
| 2623 | `2623_regles` | `ent_regle`, `ent_regle_version` (immuabilité, `validee_par <> redigee_par`), `ent_regle_population`, `ent_regle_condition`, `ent_regle_condition_code`, `ent_regle_effet`, `ent_regle_effet_param` | AU3 |
| 2624 | `2624_regles_executions` | `ent_regle_execution` (clé d'idempotence, ajout seul), `_fait`, `_effet`, `ent_regle_retour` | AU3 |
| 2625 | `2625_auto_simulation` | `ent_auto_simulation`, `_ligne` | AU4 |
| 2626 | `2626_suivi` | `ent_suivi_liste`, `ent_suivi_entree` (RLS de portée) | AU8 |
| 2627 | `2627_controles` | `ent_controle_reglage`, `ent_controle_anomalie` | AU8 |
| 2628 | `2628_auto_actions_d017` | actions `automatismes.*`, `automatisations.*`, `controles.*`, `suivi.*` si le catalogue est en base ; sinon numéro libéré | AU2 |
| 2629 | `2629_auto_personas` | (ajout personas) exclusions d'élèves, compteur de sollicitation par foyer, regroupement, validation par délégation | AU3–AU7 |
| 2630–2636 | réserve | routage (AU11) | |
| 2637 | `2637_auto_rls_stricte` | RLS stricte | AU12 |
| 2638 | réserve | — | |
| 2639 | `2639_auto_retraits` | retraits (aucune table aujourd'hui ; `ent-brain` n'a pas de table) | AU12 |

AU0 n'a **aucune migration**. Contrôle de revue : `grep -i "jsonb\|json\|bytea\|\[\]"` sur 2620–2639 ne doit rien trouver. Ordre croissant dans `migrate.rs` après la plage du 30 (2600–2619).

---

## 27. Lots (équipe Sonnet)

| Lot | Contenu | Dépend de | Critère de fin | Effort |
|---|---|---|---|---|
| **AU0 — retrait d'`ent-brain`** | retirer `install_brain_p0()` de `boot_from_env` (`ent-api/src/lib.rs:250`) et des états de test (`lib.rs:344, 3585`, `tests_abus.rs:27`, `ent-lab/src/main.rs:103, 184`, tests `ent-db`) ; remplacer dans les tests par une sonde d'événements de test si un test lit `brain().journal` ; supprimer le crate et la dépendance de `ent-db` | rien | même suite de tests verte ; plus aucune référence à `ent_brain` ; aucune migration ; mémoire stable sur un test de charge de 100 000 événements | S |
| **AU1 — ordonnanceur et registre des tâches de fond** | crate `ent-auto` ; déclarations des tâches existantes de `main.rs` ; `ent_auto_planif`, passages (2620) ; réclamation `SKIP LOCKED` ; fuseau par établissement ; contexte `JobPlanifie` ; neutralisation post-restauration ; brancher `effacer_sanctions_echues` (DR-06) ; test CI « rien de non déclaré » | AU0 | deux instances : une exécution par échéance (test) ; restauration : aucune tâche externe relancée (test) ; comportement des tâches inchangé | M |
| **AU2 — registre des automatismes et page de transparence** | déclarations des automatismes intégrés connus (§8.3), réglages (2621), page Leptos, export | AU1 | page complète ; obligatoires non désactivables (test) | M |
| **AU3 — moteur de règles (tâche, information, rappel)** | catalogues de déclencheurs, faits, effets (premiers socles : 12, 16, 3, 7, 18, 42), tables 2623–2624, consommateur de l'outbox, échéances, idempotence, plafonds, plage calme, journal, « pourquoi » | AU2 ; 29 (`ouvrir_tache`) | règle d'exemple de bout en bout ; idempotence (test de rejeu) ; destinataire sans droit → sauté ; fait C3 refusé | L |
| **AU4 — composition, simulation, validation** | écrans de composition, règle en français, simulation (2625) sans écriture, validation à quatre yeux, observation, versions, suspension automatique, bascule d'année | AU3 | simulation n'écrit rien (test) ; rédacteur = valideur refusé ; valideur sans portée refusé | L |
| **AU5 — gel** | gel par automatisme, règle, population, gel général, semaine de rentrée (2622) | AU3 | informations dues urgentes jamais gelées (test) ; levée avec choix | S |
| **AU6 — recettes** | RC-01 à RC-30 (sauf vigilance), activation des six ★ à la mise en service après simulation sur données reprises | AU4 | chaque recette a sa simulation de démonstration sur le labo | M |
| **AU7 — cartes préparées par règle** | effet `brouillon.preparer` sur `ent_brouillon_action` (30, CH3) ; cartes de lot par mission ; commandes « préparables par règle » déclarées par 3, 7, 12, 18, 19, 25 | AU3 ; 30 CH3 | aucun envoi sans N3 (test) | M |
| **AU8 — listes de suivi et contrôles de cohérence** | 2626–2627 ; contrôles déclarés par 2, 4, 5, 6, 12, 41, 42 ; remise à zéro par période | AU3 | liste effacée en fin de période (test) ; anomalie close seule | M |
| **AU9 — IA** | capacités §17.3 ; `regle.rediger` du 30 | AU4 ; 30 CH6 | brouillon seulement ; aucune activation par l'IA (test) | S |
| **AU10 — bilans et pilotage** | retours, bilans mensuels et annuels, purge du journal nominatif, indicateurs (47) | AU3 | — | S |
| **AU11 — routage** | effet `boite.affecter`, règles de routage par mots-clés (RC-30, NXR-09) | socle des demandes (N1) | — | S |
| **AU12 — vigilance précoce, RLS stricte, recette** | RC-20 à RC-24 (désactivées, conditions §15.3), RLS stricte (2637), recette par personas | AU8 ; **AIPD** | jeux de la passe personas ; aucun score ni libellé (revue) | M |

**Ordre** : AU0 (immédiat) → AU1 → AU2 → AU3 → AU4 → (AU5, AU6, AU7, AU8 en parallèle) → AU9, AU10 → AU11 quand le socle des demandes existe → AU12.
**Pour la démo de décembre** (D-034) : AU0 seulement ; AU1–AU2 sont des conditions de l'exploitation réelle (doublon 2027-2028) parce que les tâches de fond y tournent déjà.

---

## 28. Tests et recette

Idempotence (même événement deux fois → une exécution) ; deux instances ; restauration ; simulation sans écriture ; rédacteur = valideur ; valideur hors portée ; fait C3 refusé (service et base) ; destinataire sans droit → sauté ; plafond et synthèse ; plage calme ; vacances et jours ouvrés ; fuseau et changement d'heure ; gel général vs informations dues ; bascule d'année ; départ du propriétaire ; liste de suivi : visibilité par portée, remise à zéro ; journal nominatif purgé ; aucune règle « réseau » (isolation entre établissements, matrice de séparation) ; aucun effet externe direct (recherche statique des effets permis). Recette par personas (D-021, étape 4) avec `PERSONAS_31.md`.

---

## 29. Décisions

### 29.1 Appliquées par défaut (D-031)

| # | Décision |
|---|---|
| D31-1 | `ent-brain` est **supprimé**, pas renommé (Q-IA13 tranchée) ; AU0 immédiat. |
| D31-2 | Un ordonnanceur unique remplace les boucles de `main.rs` (AU1). |
| D31-3 | Registre des automatismes déclarés par leurs socles et page de transparence (§8). |
| D31-4 | Les règles de l'établissement ne produisent que des effets internes ou des cartes ; aucun envoi aux familles ou aux élèves sans geste humain en V1 (R-AU2). |
| D31-5 | Catalogues fermés (déclencheurs, faits, effets) déclarés par les socles ; ≤ 5 conditions ; ≤ 3 effets ; pas d'éditeur libre (D-023). |
| D31-6 | Simulation obligatoire, validation par une autre personne, mode observation possible. |
| D31-7 | Aucun modèle de langage dans le moteur ; l'IA rédige et explique seulement. |
| D31-8 | Aucun fait de santé ni C3/C4 dans les règles de l'établissement. |
| D31-9 | Gel général (« mode crise ») qui épargne les informations dues urgentes. |
| D31-10 | 30 recettes, six internes ★ actives par défaut après simulation. |
| D31-11 | Vigilance précoce désactivée par défaut, activable après AIPD ; listes de suivi remises à zéro par période, sans score ni étiquette. |
| D31-12 | Journal nominatif des exécutions purgé à 1 an [H, DPO]. |
| D31-13 | Semaine de rentrée sans règles d'assiduité par défaut. |

### 29.2 Soumis à Florian

| # | Question | Recommandation |
|---|---|---|
| **Q-AU1** | Autoriser plus tard des **envois automatiques de rappels neutres** aux familles par une règle (sans geste par envoi), avec gabarit fixe et activation N3 ? | **non en V1** ; à réexaminer après l'année de doublon, gabarits neutres seulement, avec l'avis du DPO |
| **Q-AU2** | Vigilance précoce : l'activer dans l'établissement pilote pendant le doublon ? | oui **après AIPD** et information du conseil / des familles ; RC-20 et RC-23 seulement au début |
| **Q-AU3** | Droit d'opposition à la vigilance précoce | non par défaut (mission d'intérêt public) ; **avis du DPO** requis |
| Q-AU4 | Durée du journal nominatif des exécutions (1 an) | DPO |
| Q-AU5 | Socle des **demandes** (N1, Q-T0-04) : le créer pour accueillir le routage (NXR-09) | oui (déjà recommandé par T0) |
| Q-AU6 | Risque de production Next.js : routes « cron » à secret comparé sans temps constant, accepté dans le corps (DR-11) | à corriger côté Next.js par Florian (Next.js intouchable pour la session) |

### 29.3 Réglages signalés (D-023) — S-3101 à S-3107

| N° | Réglage | Avis |
|---|---|---|
| S-3101 | Paramètres des automatismes intégrés (bornes déclarées par les socles) | garder, bornés |
| S-3102 | Seuils d'avertissement des simulations | candidat à règle fixe |
| S-3103 | Durée du mode observation (1–4 semaines) | garder |
| S-3104 | Plafond quotidien par règle (défaut 50) et par destinataire (20) | garder |
| S-3105 | Semaine de rentrée sans règles d'assiduité | garder, défaut oui |
| S-3106 | Durée maximale d'un gel général (14 jours) | candidat à règle fixe |
| S-3107 | Recettes actives par défaut | garder la liste courte |

---

## 30. Faits, hypothèses, contradictions

**Faits [F]** : `ent-brain` (fichiers, `Mutex<Vec>`, action jamais appliquée, installé au démarrage de production) ; boucles de `main.rs:195-290` ; outbox (`PROFONDEUR_MAX = 5`, idempotence, causalité) ; contextes système nommés ; Next.js (§2.3) par lecture seule.
**Sourcé** : §5.
**Hypothèses [H]** : seuils des recettes, plafonds, durées (journal 1 an, passages 90 jours), pratiques du §4, efforts.
**Contradictions signalées**
- `SOCLES.md` §31 place la « détection précoce explicable (A6) » au 31 ; `12_ASSIDUITE.md` §13.4 possède déjà des alertes (dont « tendance ») : **partage retenu** — le 12 garde ses alertes d'assiduité (y compris réglementaires), le 31 porte le **faisceau multi-domaines** et les listes de suivi ; aucune alerte n'est dupliquée.
- `IA_COEUR.md` §4.8 : « même file de brouillons (une règle et l'IA proposent de la même façon) » : confirmé, avec la table `ent_brouillon_action` (30 §6.4).
- CLAUDE.md « envois de messages soumis à confirmation humaine » est écrit pour l'IA ; ce document l'applique aussi aux **règles** de l'établissement (lecture prudente, D31-4) ; les **informations dues** des socles (absence signalée, R.131-5) restent automatiques — ce n'est pas une contradiction mais une frontière à confirmer par Florian (Q-AU1).
- D-025 / Q-IA13 proposaient de **renommer** `ent-brain` ; ce document recommande de le **supprimer** (rien n'y est utile).

---

## 31. Ajouts de la passe personas (ajout personas, 2026-11-11)

Source : `docs/socles/PERSONAS_31.md` (16 personas, 38 manques dont 6 bloquants). **En cas de contradiction avec les sections précédentes, ce §31 l'emporte.**

### 31.1 Règles ajoutées (R-AU36 à R-AU40)

| N° | Règle | Manque |
|---|---|---|
| **R-AU36** | Une règle portant sur l'**activité d'un personnel** ne peut que lui rappeler ce qu'il a à faire, ou prévenir le service qui pallie un besoin opérationnel immédiat ; jamais d'agrégat, de classement ou de liste de suivi de personnels, jamais d'effet vers la hiérarchie au-delà de ce besoin ; les personnels sont informés de ces règles (Q-AU7). | M-3110 |
| **R-AU37** | **Plafond de sollicitation par foyer**, toutes sources confondues (défaut 3 relances préparées par semaine, hors informations dues), tenu avec le socle 8 ; au-delà, la carte le dit et propose un message regroupé. | M-3114, M-3123 |
| **R-AU38** | Un fait n'est utilisable dans une règle que si le rédacteur, le valideur **et** chaque destinataire ont le droit de lire son domaine. | M-3117 |
| **R-AU39** | Règles de vigilance (`suivi.ajouter`) : le détail nominatif de leurs exécutions est purgé à la fin de la période, comme la liste. | M-3124 |
| **R-AU40** | La population d'une règle est entièrement dans la portée du rédacteur ; tout agrégat affiché hors portée nominative est masqué sous 5 ; les textes d'effets n'ont aucune variable, et pour un fait C2 le titre est choisi parmi les titres neutres déclarés par le socle. | M-3118, M-3128, M-3129 |

**Précisions** : R-AU13 porte sur l'**identité** de la personne (D-013), pas sur le compte (M-3130, KR-27) ; R-AU21 reçoit un **délai de grâce de 15 jours** avant suspension quand seul le propriétaire manque, la direction étant invitée à transférer en un geste (M-3122) ; AU1 reprend **exactement** les fréquences actuelles de `main.rs` avant de les rendre réglables (M-3134) ; §18.2 : la notice aux familles mentionne les rappels préparés automatiquement et relus (M-3115).

### 31.2 Actions ajoutées (AUT-224 à AUT-243)

| N° | Action | Acteur | Règles | Niveau | Lot |
|---|---|---|---|---|---|
| AUT-224 | Regrouper les effets d'une règle en une tâche de synthèse quotidienne ou hebdomadaire par destinataire | CRE | effet `tache.groupee` | N1 | AU3 |
| AUT-225 | Voir, sur une tâche issue d'une règle, les autres tâches ouvertes ou closes depuis 7 jours sur le même élève issues des règles, pour l'équipe éducative | destinataire | dans la portée ; état et preneur | N0 | AU3 |
| AUT-226 | Compter toutes les relances, manuelles ou préparées, dans les faits de relance | SYS | déclaration des socles 3, 7, 16, 18 | — | AU3 |
| AUT-227 | Appliquer le plafond de sollicitation par foyer et proposer un message regroupé | SYS, avec le 8 | R-AU37 | — | AU7 |
| AUT-228 | Veilleur « tâche silencieuse » : aucun passage depuis 2 × la période → alerte d'exploitation | SYS | — | — | AU1 |
| AUT-229 | Annoncer un automatisme nouvellement déclaré (badge « nouveau », information aux administrateurs) | SYS | au déploiement | — | AU2 |
| AUT-230 | Publier le catalogue des déclencheurs, faits et effets (registre, CA, AIPD) | DIR, ADM | export | N0 | AU3 |
| AUT-231 | Exclure un élève d'une règle, motif codé non affiché aux destinataires, avec échéance de revue | CPE, DIR | visible sur la fiche de règle seulement | N2 | AU4 |
| AUT-232 | Joindre un message interne neutre aux personnels lors d'un gel général | DIR | personnels seulement | N2 | AU5 |
| AUT-233 | Destinataire de fonction « service en cours » (internat de nuit, permanence), repli vers la mission | CRE | planning du 41 / 6 | N1 | AU3 |
| AUT-234 | Refus motivé d'un effet externe avec renvoi au réglage de l'information due du socle concerné | SYS | R-AU2 | — | AU3 |
| AUT-235 | Informer les personnels des règles qui portent sur leur activité (page et notice) | SYS, DIR | R-AU36 | — | AU2 |
| AUT-236 | Masquer sous 5 les agrégats hors portée nominative | SYS | R-AU40 | — | AU4 |
| AUT-237 | Purger en fin de période le détail des exécutions des règles de vigilance | SYS | R-AU39 | — | AU8 |
| AUT-238 | Valider par délégation (D-017), mention « validé par délégation de … » | délégataire | — | N3 | AU4 |
| AUT-239 | Reporter une tâche issue d'une règle (29) sans qu'elle soit recréée à l'échéance suivante | destinataire | idempotence sur la période | N1 | AU3 |
| AUT-240 | Vérifier à la composition et à la validation le droit de lecture de chaque fait utilisé (rédacteur, valideur, destinataires) | SYS | R-AU38 | — | AU4 |
| AUT-241 | Tester une règle sur un élève précis de sa portée (« que ferait-elle pour Tom ? ») | CRE, VAL | lecture seule | N0 | AU4 |
| AUT-242 | Suggérer une règle comme recette à ScolIA, sans aucune donnée | CRE | — | N1 | AU10 |
| AUT-243 | Produire le bilan annuel des automatismes pour le CA ou l'instance (sans nominatif), avec la revue de chaque règle de vigilance | DIR | — | N0 | AU10 |

**Total après la passe : 144 actions** (AUT-100 à AUT-243).

### 31.3 Cas limites ajoutés (KR-26 à KR-35)

| N° | Situation | Comportement |
|---|---|---|
| KR-26 | Propriétaire parti, règle utile | 15 jours de grâce, transfert proposé en un geste |
| KR-27 | Même personne avec deux comptes (rédige avec l'un, valide avec l'autre) | refus : séparation par identité |
| KR-28 | Déploiement pendant un passage | réclamation expirée, reprise idempotente sur une autre instance |
| KR-29 | Année de doublon, finances dans Charlemagne | RC-19 indisponible tant que la cohabitation (24 F14) est active |
| KR-30 | 1er septembre : 200 élèves sans classe | contrôles en mode « rentrée » : anomalies sans tâche pendant 5 jours ouvrés |
| KR-31 | Famille déjà à 3 relances cette semaine, nouvelle relance de pièce | carte : « déjà 3 relances », message regroupé proposé |
| KR-32 | CPE qui veut utiliser « impayé » dans une règle | refus : pas de droit de lecture des finances (R-AU38) |
| KR-33 | Règle « appel non fait » qui viserait la direction | refus (R-AU36) ; seule la vie scolaire, pour vérifier les présents |
| KR-34 | Simulation : 3 élèves d'une classe hors portée | « moins de 5 » |
| KR-35 | Élève exclu d'une règle dont l'échéance de revue passe | tâche à la CPE « revoir l'exclusion » |

### 31.4 Lots et migrations révisés

- **AU1** reçoit AUT-228 et le critère « fréquences actuelles reprises à l'identique ».
- **AU2** reçoit AUT-229, AUT-235. **AU3** reçoit AUT-224 à AUT-226, AUT-230, AUT-233, AUT-234, AUT-239. **AU4** reçoit AUT-231, AUT-236, AUT-238, AUT-240, AUT-241 et l'accessibilité du compositeur. **AU5** reçoit AUT-232. **AU7** reçoit AUT-227 (avec le 8). **AU8** reçoit AUT-237 et le mode « rentrée » des contrôles. **AU10** reçoit AUT-242, AUT-243.
- Migrations : **2629** `2629_auto_personas` — `ent_regle_exclusion` (`etablissement_id`, `id`, `version_id` ou `regle_id`, `eleve_id`, `motif` code, `revue_le`, `posee_par`), `ent_foyer_sollicitation` (compteur hebdomadaire par foyer, **reconstructible** depuis les journaux des socles et du 31), colonne `ent_regle_effet.regroupement` (`aucun`, `quotidien`, `hebdomadaire`), colonne `ent_regle_version.validee_par_delegation_de` ; 2630–2636 restent en réserve.
