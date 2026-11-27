# Socle 39 — Stages, PFMP et entreprises

**Date :** 2026-11-11 — **Statut :** conception de fond, **non décidée** (à relire par Florian). Rédigée **avant** le code Rust et **après** un module Next.js en usage réel (D-035). Aucun code écrit ni modifié ; aucun accès à une base, à un secret ou à un environnement de production. Le code Next.js a été **lu seulement** (rien exécuté, rien construit, aucun `.env` lu).
**Branche :** `ent-socle-39-conception`, partie de `origin/ent-mc0hoe`.
**Passe personas :** `docs/socles/PERSONAS_39.md` ; ajouts faits **ici**, au **§32** (marqués « ajout personas ») ; **en cas de contradiction, le §32 l'emporte**, rien n'a été supprimé.
**À lire avec :** `CLAUDE.md` ; `docs/DECISIONS.md` D-017, D-021, D-023, D-024, D-029 à D-036 (notamment **D-031**, **D-034**, **D-035**, **D-036**) ; `docs/PRA_SAUVEGARDES.md` §18.4 (E1–E14) ; `docs/TRANSITION_T0_INVENTAIRE.md` (M12) ; `docs/SOCLES.md` §39 ; `docs/MATRICE_CHARLEMAGNE.md` §3.13 (STA-01 à STA-07) ; `docs/SOCLE2_DOMAINES.md` D12 (INT-08 à INT-10, repris ici) ; conceptions `07_DOCUMENTS.md` (§10 signatures, liens externes), `06_PERSONNELS.md` (missions, habilitations), `10_EMPLOI_DU_TEMPS.md` (§11.9 PFMP), `12_ASSIDUITE.md` (situation `pfmp`, AS-118, AS-240), `40_RESTAURATION.md` (cause « stage »), `41_INTERNAT.md`, `42_SORTIES_VOYAGES.md` (modèle de forme), `IA_COEUR.md` (§6.2).

**Conventions.** **[F]** fait vérifié dans le code (Next.js : chemins depuis la racine ; Rust : depuis `ent-rust/`) ; **[V]** règle lue sur une source officielle (§5.10) ; **[H]** hypothèse ; **[R]** recommandation, non décidée.

| Élément | Numéros | Remarque |
|---|---|---|
| Actions existantes de Next.js | **NST-01**… | NS- est pris ailleurs |
| Défauts de Next.js | **DN-01**… | |
| Défauts du Rust existant | **RS-01**… | |
| Actions cibles | **STG-100**… | INT-08 à INT-10 (`SOCLE2_DOMAINES.md`) et STA-01 à STA-07 (matrice) renvoient ici |
| Règles | **R-SG1**… | |
| Cas limites | **CSG-01**… | |
| Questions | **Q-SG1**…, **F-SG1** (pour Florian, urgent) | |
| Réglages signalés (D-023) | **S-3900 à S-3949** | plage propre au socle |
| Manques (passe personas) | **M-3901**… | plage propre au socle |

**Plage de migrations imposée : 2780–2799** (vérifiée libre sur les 134 branches distantes le 2026-11-11, inscrite dans `docs/socles/MIGRATIONS.md`). Le lot 0 (ST0) part en **2780**.

**Méthode.** Lecture des décisions, de la carte, de l'inventaire T0 et des conceptions liées ; état des lieux du Rust existant (migration 082, `domain-eleve/src/stage.rs`, `ent-db/src/stage_repo.rs`, service, routes, laboratoire) ; **inventaire en lecture seule du module Next.js** (sous-agent, ≈ 40 fichiers) ; **recherche réglementaire** (sous-agent, 34 lectures : Légifrance, code.travail.gouv.fr, PDF du BO et d'éduscol, service-public ; pages HTML d'éduscol en 403).

Sommaire : §0 En bref · §1 Thèse · §2 Existant (Next.js, Rust, lot 0) · §3 Acteurs · §4 L'année · §5 Réglementaire · §6 Branchements · §7 Vocabulaire · §8 Cycle de vie · §9 Dispositifs et périodes · §10 Organismes et contacts · §11 Recherche et offres · §12 Convention · §13 Signatures, départ, suivi · §14 Évaluation · §15 Argent · §16 Documents · §17 Cas limites · §18 IA · §19 Actions · §20 Règles · §21 Droits · §22 Événements · §23 Notifications · §24 Écrans · §25 Modèle · §26 Reprise · §27 Lots et migrations · §28 Tests · §29 Réglages · §30 Décisions · §31 Faits · §32 Ajout personas.

---

## 0. En bref

1. **Un stage est un engagement à trois dont l'établissement reste garant.** L'élève en stage reste un élève : statut scolaire, autorité du chef d'établissement, assiduité, enseignant référent. ScolIA ne le « sort » pas : il le **localise ailleurs**, à des dates et des horaires connus.
2. **La règle d'or : pas de convention signée, pas de départ** (R-SG1). Tant que toutes les signatures exigées ne sont pas réunies, l'élève est attendu au lycée ; alertes à J-7, J-3, J-1 ; aucune dérogation. Next.js marque aujourd'hui « en stage » des élèves dont la convention n'est **pas signée** (DN-14).
3. **Le module Next.js marche et sert** : offres déposées par les parents, candidatures, préconvention par l'élève, horaires contrôlés selon l'âge, alerte hors période, signatures multiples (« un parent suffit »), papier du tuteur, avenants qui remettent les signatures à zéro, relances en masse, classement automatique, présence et repas pendant le stage. **On garde ces 37 actions** (NST-01 à NST-37).
4. **Mais il est ouvert** : **4 failles graves** (DN-01 à DN-04) — l'élève reçoit les **liens et codes de signature de ses parents et de son tuteur** ; tout parent lit toute convention ; tout utilisateur modifie toute convention, même signée ; un ancien lien reste valable. **À signaler sans attendre à Florian (F-SG1)**, Next.js restant intouchable pour le pilote. 20 défauts au total.
5. **Le Rust existant est un embryon** (082) dont la « convention » `active` n'a aucune signature : **lot 0 (ST0)**, migration 2780, avant tout branchement sur l'appel (10 défauts RS-01 à RS-10).
6. **Ce qui manque partout et que ScolIA apporte** : visites et tournées du référent, page mobile du **tuteur sans compte** (absence en deux clics, « tout va bien », évaluation, attestation), **accidents** avec le bon circuit (l'organisme déclare en PFMP, le chef d'établissement en observation, sous 48 h), **travaux réglementés** et aptitude, **décompte des semaines** pour l'examen, **allocation PFMP** calculée au jour près (forfait de 10 à 20 € par jour, plafonds, ApLyPro), **gratification** au-delà de 44 jours, **équité** (élèves sans lieu, refus discriminatoires), signalement des violences en un geste.
7. **Réglementaire vérifié** (§5) : L124-1 à L124-20 valent aussi pour les PFMP ; circulaire du **21-11-2025** pour le collège et la seconde ; convention-type PFMP de 2023 ; arrêtés du **29-11-2025** (allocation) et du **22-04-2026** (terminale : deux semaines dédiées) ; durée et repos des mineurs ; conservation **10 ans** des pièces d'allocation.
8. **Le plan** :

   | Élément | Volume | Numéros |
   |---|---|---|
   | Actions | 200 (+ 40 ajout personas, §32) | STG-100 à STG-319, STG-320 à STG-359 |
   | Cas limites | 57 (+ 18) | CSG-01 à CSG-75 |
   | Règles | 26 (+ 8) | R-SG1 à R-SG34 |
   | Réglages | 15 (+ 4) | S-3900 à S-3918 |
   | Lots | 13 | ST0 à ST12 |

   **Parcours pour la rentrée 2027** (usage réel, D-035) : ST0, ST1, ST2, ST3, ST5, ST6 en mode suivi.
9. **Décisions appliquées par défaut (D-031)** : 18 (§30.1) et celles de la passe personas (§32). **Soumis à Florian** (§30.2) : les failles de Next.js en production, l'assurance et la décision équivalente au CA dans le privé, les services externes (Sirene, géocodage, OneDrive), la procédure ApLyPro, les durées de conservation (DPO), le niveau de signature (juriste), l'export des données réelles, l'arrêt du module Next.js.

---

## 1. Thèse : un stage est un engagement à trois, daté, dont l'établissement reste garant

1. **L'élève en stage reste un élève.** Sous statut scolaire du premier au dernier jour, il est sous la responsabilité du chef d'établissement, couvert par le régime des accidents du travail des élèves, compté dans l'assiduité, suivi par un enseignant référent. Le logiciel ne « sort » jamais l'élève de l'établissement : il le **localise ailleurs**, pour une période et à des horaires connus.
2. **Pas de convention signée, pas de départ.** C'est la règle de sécurité la plus importante du socle. Tant que la convention n'est pas signée par toutes les parties exigées, l'élève est **attendu au lycée** (ou au collège) : l'appel ne le marque pas « en stage », la cantine le compte, la famille et le référent sont prévenus à J-7, J-3 et J-1. Aucune dérogation ne la contourne (l'assurance et la responsabilité en dépendent).
3. **Le stage référence, il ne recopie pas.** L'élève (2), ses titulaires et leurs droits (4), sa formation et sa classe (5), son référent (6, mission propriétaire 39), ses séances (10), ses absences (12), ses repas (40), ses nuits d'internat (41), ses compétences (22), ses pièces et signatures (7) : tout est lu à la date, jamais retapé. Ce qui doit être figé (la convention signée, l'attestation) est un **document** du socle 7 avec son empreinte.
4. **L'entreprise est un partenaire, pas un utilisateur.** Elle n'a pas de compte. Ses personnes (tuteur, signataire, RH) sont des **contacts** de l'établissement, avec une base légale, une information et une durée de conservation. Elles agissent par **lien nominatif à portée limitée** et code à usage unique (socle 7 §10.5), ne voient que ce qui les concerne (un élève, une convention, ses horaires) et rien d'autre de l'ENT.
5. **Le règlement est un garde-fou calculé.** Âge minimal, durée quotidienne et hebdomadaire des mineurs, repos, travail de nuit et du dimanche, travaux réglementés et leur dérogation, aptitude médicale, durée minimale des PFMP pour l'examen, seuil de gratification, quota de stagiaires : le serveur **bloque** ce qui est interdit, **avertit** ce qui est douteux, et dit pourquoi, texte à l'appui.
6. **Chaque stage a son fil.** Recherche, candidatures, placement, convention, avenants, départ, présences et absences, visites, incidents, évaluation, attestation, allocation : autant de **faits datés** en ajout seul, qui disent à chaque instant où en est l'élève et ce qui manque.
7. **On partage le strict nécessaire.** Le tuteur sait qui arrive, quand, à quels horaires, qui appeler à l'établissement ; il ne voit ni les notes, ni la santé, ni la situation familiale. Un aménagement nécessaire (handicap, PAI) n'est transmis que sous forme de **consigne** consentie par la famille ou l'élève majeur, jamais de diagnostic (socle 34). Les familles séparées voient chacune ce que leur lien autorise ; l'élève majeur décide seul.
8. **L'équité est un indicateur, pas un vœu.** Les élèves « sans lieu de stage » à J-30 sont une liste de travail, pas une fatalité : le socle la fait apparaître, propose des offres, trace les refus discriminatoires signalés, et mesure les écarts (classe, établissement d'origine, quartier) sans jamais classer un élève.
9. **L'IA lit, prépare, signale ; elle ne signe, n'envoie, n'évalue ni ne décide rien** (`IA_COEUR.md` §6.2). Elle propose une convention pré-remplie, une tournée de visites, un brouillon d'appréciation que le référent relit ; elle ne note pas un élève, ne classe pas une entreprise et ne lit aucune donnée de santé.

---

## 2. Évaluation franche de l'existant

### 2.1 Où et comment Next.js le fait [F]

Inventaire en **lecture seule** du module Next.js présent dans la copie de travail de la branche `ent-mc0hoe` (rien exécuté, rien construit, aucun `.env` lu ; l'écart éventuel avec la version en production n'a pas été vérifié, Q-SG10). Chemins relatifs à la racine du dépôt.

- **Cœur** : `app/lib/stage-workflow.ts` (3 051 lignes), `stage-db.ts` (1 530), `stage-pdf.ts` (1 270), `stage-notify.ts` (882), `stage-types.ts` (563), plus `stage-constraints.ts`, `stage-schedule.ts`, `stage-period-alignment*.ts`, `stage-identity-otp.ts`, `stage-candidature.ts`, `stage-convention-ingest.ts`, `stage-auto-purge.ts`, `stage-eleve-dossier-filing.ts`, `stage-onedrive-filing.ts`.
- **34 routes** sous `app/api/stages/**` ; **pages publiques** `app/stages/{candidater,deposer,eleve,preconvention,signer}` ; **intranet** `app/(admin)/stages/StagesPageClient.tsx` et 31 composants `app/components/stages/*`.
- **Stockage** : quatre paires de tables clé-valeur (`stage_offer`, `stage_convention`, `stage_application`, `stage_token` et leurs `_attr` en `path → value`, `db/schema-ent-relational.ts:722-849`) ; le code métier lit et écrit toujours des « fichiers JSON » (`getJson`/`putJson` sur `STAGE_S3`, `stage-types.ts:3-34`) redirigés par expressions régulières vers ces tables (`app/lib/ent-json-postgres.ts:143-260, 385-530`) ; anciennes collections `stages__*` migrées à la volée (`stage-db.ts:459-860`) et par `scripts/migrate-stages-to-typed.mjs`. Fichiers réels en stockage objet : PDF générés et déposés, images de signature, scans. Cache Valkey.
- **Maturité** : l'inventaire T0 le classait « partiel à fiable [H] » (M12). La lecture confirme : **actions métier riches et éprouvées par l'usage, sécurité et modèle de données à refaire.** C'est un module **à reprendre** au sens de D-035 (on garde les actions, on réécrit tout), pas une ébauche.

### 2.2 Ce que fait Next.js : actions existantes, toutes reprises [F]

| N° | Action Next.js | Où [F] | Reprise (§19) |
|---|---|---|---|
| NST-01 | Un **parent dépose une offre** de stage (type pfmp / observation / job d'été / autre, entreprise, adresse, SIRET, secteur, description, places, niveaux visés, période, contact) | `api/stages/offers/route.ts:90`, `stage-types.ts:40-65` | STG-140 (élargi : entreprise par lien, personnel) |
| NST-02 | La direction **valide ou refuse** une offre ; statuts `pending/approved/rejected/filled/archived` | `offers/[id]/route.ts:30-79` | STG-142 |
| NST-03 | **Candidature par lien** à une offre : places restantes, refus des doublons, convention brouillon pré-remplie depuis l'offre, offre `filled` quand complète | `api/stages/public/candidater`, `stage-candidature.ts` | STG-146 à STG-149 (identité de l'élève **vérifiée**, DN-07) |
| NST-04 | **Préconvention par l'élève** : identification (nom, prénom, naissance, classe), contrôle de la classe éligible, code à usage unique par e-mail aux adresses **masquées** du responsable, jeton de preuve | `public/preconvention/route.ts`, `stage-student-identity.ts`, `stage-identity-otp.ts:8-62` | STG-160 (dans l'ENT, compte élève ; le parcours sans compte n'est gardé que pour l'élève sans compte, R-SG17) |
| NST-05 | Tableau de bord élève : ses dossiers, les périodes disponibles ; création d'un brouillon lié à une période | `stage-student-dossier.ts`, `stage-workflow.ts:2281` | STG-161, STG-162 |
| NST-06 | Enregistrer ou **soumettre** la préconvention tant qu'elle est brouillon ou refusée | `public/student/route.ts` | STG-163 |
| NST-07 | **Demande de changement d'e-mail du tuteur**, validée par l'établissement | `stage-workflow.ts:925, 963` | STG-176 |
| NST-08 | Convention : élève, entreprise (nom, adresse, code postal, ville, SIRET, activité), **tuteur** (nom, e-mail, téléphone), **RH signataire** facultatif, horaires, période, référent | `stage-types.ts:243-407` | STG-167 à STG-175 (champs étendus, §12.1) |
| NST-09 | **Horaires** en semaine type ou jour par jour, matin et après-midi ou journée continue, jours de présence | `stage-types.ts:83-118`, `stage-schedule.ts`, `StageScheduleEditor.tsx` | STG-180, STG-181 |
| NST-10 | **Contrôles des horaires selon l'âge et le cycle** : plage horaire autorisée (fin plus tôt avant 16 ans), jours autorisés, 30 h par semaine avant 15 ans et 35 h au-delà, 7 h par jour, périodes bloquées | `stage-constraints.ts:67-100, 353` | STG-182 à STG-186 (règles réglementaires §5, versionnées) |
| NST-11 | **Alerte « hors période »** : horaires hors de la période officielle, correction proposée | `stage-period-alignment*.ts`, `StageOutOfPeriodAlert.tsx` | STG-187 |
| NST-12 | Validation des champs à la soumission | `stage-workflow.ts:353-402` | STG-164 (côté serveur) |
| NST-13 | **Validation administrative** : refus motivé, ou acceptation qui construit les signataires (parent, parent 2, tuteur, RH, professeur principal, direction par cycle), génère le PDF, émet un lien par signataire et envoie les e-mails | `stage-workflow.ts:160, 1363-1412` | STG-190, STG-191, STG-200 |
| NST-14 | **Signatures** parallèles ; règle : tous les non-parents signent, **un seul responsable légal suffit** | `stage-types.ts:553` | STG-201, R-SG8 (règle d'accord du 7, §10.4) |
| NST-15 | Trois méthodes : **code par e-mail** (6 chiffres, 30 min), **tracé au doigt**, **papier déposé** (tuteur et RH seulement), le scan devenant le document principal avec une annexe des signatures | `stage-workflow.ts:1558-1576, 1722`, `stage-pdf-sign.ts:216` | STG-202 à STG-205 (niveaux S1, S2, P du 7 ; le tracé n'est plus une signature, DN-12) |
| NST-16 | Signature des personnels par une **image de signature** enregistrée, apposée sur le PDF | `my-signature/route.ts`, `stage-pdf-sign.ts:374` | STG-203, STG-206 (S2 passkey ; l'image n'est plus qu'un rendu) |
| NST-17 | L'administration **refuse, revalide, révoque** une signature (PDF reconstruit), **ajoute ou retire** un signataire, marque une signature « faite à la main », **relance** un ou tous les signataires, **en masse** | `conventions/[id]/route.ts:335-640`, `stage-workflow.ts:2164`, `resend-signatures-bulk` | STG-207 à STG-212 |
| NST-18 | Convention signée : **classement automatique** dans le dossier de l'élève et sur OneDrive ; e-mail « convention signée » | `stage-eleve-dossier-filing.ts:267`, `stage-onedrive-filing.ts:161, 312` | STG-213 (dossier de l'élève via le 7 ; OneDrive : Q-SG9) |
| NST-19 | **Avenant d'horaires et de dates**, demandé depuis le lien ou proposé par l'établissement ; signatures suspendues pendant la demande ; validation qui **remet à zéro toutes les signatures** | `stage-workflow.ts:1023, 1098, 1227, 1269, 1529` | STG-220 à STG-224 (avenant = nouveau document lié, D28 du 7) |
| NST-20 | **Fil de discussion** sur le lien de signature (élève exclu), notifications, débit limité | `stage-types.ts:160`, `stage-notify.ts:188`, `public/sign/route.ts:37` | STG-230 (socle 16, fil de la convention) |
| NST-21 | **Dépôt anonyme d'un PDF papier** (5 par heure, 15 Mo), OCR, extraction des champs et contrôle des signatures par IA, rapprochement d'élève, rejet motivé ou statut `convention_deposited`, approbation qui ne lance que les signatures internes | `public/deposer`, `stage-convention-ingest.ts:361`, `stage-workflow.ts:208, 1435` | STG-240 à STG-243 (dépôt **non anonyme** ; IA locale ; contrôle humain, DN-08) |
| NST-22 | Création par l'administration d'une convention **déjà signée hors plateforme** | `conventions/offline`, `stage-workflow.ts:2392` | STG-244 |
| NST-23 | Création **sur plateforme** par l'administration | `conventions/platform`, `stage-workflow.ts:2607` | STG-170 |
| NST-24 | **Annulation** confirmée par un mot tapé ; liens retirés ; anciens liens affichent une page « annulée » (410) | `stage-workflow.ts:2874`, `StageCancelledPublicPage.tsx` | STG-226 |
| NST-25 | **Périodes officielles** par année et par classe, avec rappels | `stage-periods-config.ts`, `StagePeriodsEditor.tsx` | STG-120 à STG-124 |
| NST-26 | **Contraintes** par cycle (collège, lycée, école) | `stage-constraints.ts` | STG-101, STG-186 (règles versionnées, non modifiables sous le minimum légal) |
| NST-27 | **Référents** par classe ou par élève ; le professeur principal **délègue** ; affectation en lot ; avis aux référents | `stage-referents-config.ts`, `referents/assign-students`, `referents/notify-assignments` | STG-130 à STG-133 (mission du 6, propriétaire 39) |
| NST-28 | **Observateurs** (« watchers ») CPE ou restauration par classe ou élève, en lecture | `stage-watchers-config.ts` | STG-134 (remplacé par des projections par droit, R-SG21) |
| NST-29 | **Présence « en stage »** lue en direct (statuts signés ou en cours de signature, dates de la période) ; exclusion du bulletin | `presence/route.ts`, `occupancy/stages-read.ts:90` | STG-250 (seulement après signature complète, DN-14) |
| NST-30 | **Jours de repas** non pris pendant le stage, sur 14 jours, pour CPE, restauration, administration, filtre par secteur | `repas-absences/route.ts` | STG-252 (cause « stage » du 40) |
| NST-31 | **Import CSV** des contacts (e-mails élèves et parents) | `stage-contacts-import.ts:153` | **non repris** : les contacts viennent du socle 4 (DN-15) |
| NST-32 | **Tableau par classe** (qui a trouvé, où en est chaque convention), élèves sortis exclus | `stage-class-roster.ts`, `StageClassRosterPanel.tsx` | STG-150, STG-151 |
| NST-33 | Hub : compteurs, signatures en attente, tableau de bord | `GET /api/stages`, `StagesBoardPanel.tsx`, `StagePendingSignaturesPanel.tsx` | STG-152, STG-153 |
| NST-34 | **Archivage annuel automatique** des offres et conventions de l'année précédente, et purge manuelle avec simulation | `stage-auto-purge.ts`, `purge/route.ts` | STG-303 à STG-305 (conservation du 27, purge réelle des jetons et contacts) |
| NST-35 | Outils de l'assistant IA sur les stages | `brain-ai/tools/handlers/stages*.ts` | §18 (contrat IA) |
| NST-36 | Code d'identification **jamais envoyé à l'adresse de la direction** | `stage-identity-otp.ts:50` | R-SG16 |
| NST-37 | Création possible par tout personnel (professeur, surveillant, CPE) | `stage-access.ts:63` | STG-170 (droit `stages.preparer`, D-017) |

**Ce que Next.js ne fait pas [F]** : pas de **gratification** ni de champ d'assurance (texte d'assurance par défaut codé en dur, `stage-pdf.ts:38-40`) ; pas de **visites**, d'**évaluation**, d'**attestation**, d'**allocation PFMP**, de **décompte des semaines** pour l'examen, de **travaux réglementés**, d'**accident**, de **rupture** ; pas d'**absences en entreprise** (écriture désactivée le 23-09-2026, `stage-absences-sync.ts`) ; pas de SMS ; pas d'**entreprise** comme objet (chaque convention recopie l'entreprise) ; pas de **mobilité internationale** ; pas d'import de la plateforme du stage de seconde.

### 2.3 Défauts trouvés dans Next.js (à ne pas reproduire ; corrigés dans la réécriture)

Next.js est intouchable (CLAUDE.md) : ces défauts sont **signalés à Florian**, qui décide d'une éventuelle correction côté production ; les quatre premiers justifient une **alerte immédiate** (§30.2, F-SG1).

| N° | Gravité | Défaut [F] | Où | Correctif ScolIA |
|---|---|---|---|---|
| **DN-01** | **critique** | Le lien de l'élève renvoie la convention brute, **jetons de signature et codes à usage unique des parents et du tuteur compris** : l'élève peut signer à leur place | `public/student/route.ts:79, 99, 156, 190, 194` | projections par destinataire (R-SG21) ; jetons jamais relus (hachés) ; test de non-régression |
| **DN-02** | **critique** | Tout compte **parent** lit n'importe quelle convention par son identifiant, liens de signature compris | `conventions/[id]/route.ts:73` | lien parent–élève et droits du lien (4) vérifiés côté serveur ; RLS |
| **DN-03** | **critique** | Tout utilisateur authentifié **modifie** n'importe quelle convention, même signée (entreprise, e-mails, référent) | `conventions/[id]/route.ts:140-159, 217-222` | commandes serveur avec droit D-017 et état de départ ; convention signée **immuable** (avenant seulement) |
| **DN-04** | haute | Un **ancien lien de signature reste valable** après régénération (le jeton n'est pas comparé à celui du signataire) : un lien envoyé à une mauvaise adresse signe encore | `stage-workflow.ts:278, 551, 1518-1528, 1699` | un seul lien actif par signataire et par version ; révocation à l'émission du suivant |
| DN-05 | haute | Les signataires **internes** (direction, professeurs) signent par simple possession du lien, sans code | `stage-workflow.ts:1551` | personnels : S2 passkey dans l'ENT, jamais par lien |
| DN-06 | haute | Jetons **sans expiration**, stockés **en clair** comme clé primaire ; codes en clair, **sans compteur d'essais**, comparaison non constante ; débit limité en mémoire par instance | `stage-workflow.ts:82`, `stage-db.ts:33-45` | jetons hachés, échéance, 5 essais par code, comparaison constante, limitation partagée (socle 1) |
| DN-07 | haute | L'élève **remplace les e-mails de ses parents** à la création ou par le PATCH ; la vérification de l'e-mail parent est du code mort : il peut se faire envoyer le lien « parent » | `preconvention/route.ts:454-462`, `stage-workflow.ts:428` | signataires titulaires **calculés** depuis le 4 (coordonnées vérifiées), jamais saisis par l'élève |
| DN-08 | moyenne | Le **dépôt anonyme** renvoie l'élève rapproché par OCR ; les PDF partent chez **Mistral** (API externe) | `deposer/route.ts`, `stage-convention-ingest.ts` | dépôt par lien nominatif ou par un personnel ; lecture locale ; rapprochement proposé à un humain |
| DN-09 | moyenne | Le détail d'une offre (contacts, jeton de candidature) est ouvert à tout compte ; la présence ne contrôle que l'activation du module | `offers/[id]`, `presence/route.ts` | projections par droit |
| DN-10 | moyenne | Le rôle surveillant voit **toutes** les conventions | `stage-access.ts:39` | AED : « en stage chez X jusqu'au … » seulement |
| DN-11 | moyenne | Cloisonnement par établissement tiré du contexte global, **jeton non lié à l'établissement** ; identifiants globaux | `stage-db.ts:273` | lien porte `etablissement_id` ; RLS ; FK composites |
| DN-12 | moyenne | Le **tracé au doigt** est présenté comme une signature ; « preuve » `SC-xxxxxx` = hachage non cryptographique ; ni empreinte du PDF, ni horodatage | `stage-signature-proof.ts` | preuve du 7 (empreinte SHA-256 de la version, défi, horodatage) |
| DN-13 | moyenne | Toute signature déposée est **acceptée d'office** ; le contrôle humain n'existe que pour le papier | `stage-workflow.ts:1609-1610` | papier « déposé, à valider » ; S1/S2 valides par construction |
| DN-14 | moyenne | La présence « en stage » compte les conventions **non encore signées** (`signatures_pending`, `convention_ready`, `convention_deposited`) et les week-ends ; la cantine, elle, ne compte que les jours travaillés : deux vérités | `occupancy/stages-read.ts:90`, `repas-absences/route.ts` | **R-SG1** : rien avant signature complète ; jours réels depuis les horaires, une seule règle |
| DN-15 | faible | Import CSV de contacts qui **double** le dossier administratif | `stage-contacts-import.ts` | supprimé |
| DN-16 | faible | Écritures concurrentes « le dernier gagne » ; purge annuelle lancée en parallèle à chaque lecture ; e-mails envoyés sans reprise | `stage-db.ts:382-438`, `stage-auto-purge.ts` | verrou optimiste ; travaux planifiés ; outbox |
| DN-17 | faible | Les identifiants d'absences de stage sont perdus à chaque sauvegarde | `stage-workflow.ts:2938-3050` | sans objet (modèle normalisé) |
| DN-18 | faible | Assurance de l'établissement **codée en dur** en repli (nom d'un assureur) | `stage-pdf.ts:38-40` | paramètre de l'établissement, obligatoire avant la première convention |
| DN-19 | faible | Aucun test du workflow métier (signatures, codes, droits, contraintes légales, PDF, purge) | `*.test.ts` | §28 |
| DN-20 | faible | Données jamais minimisées : codes et preuves conservés, jetons et contacts jamais purgés | `stage-auto-purge.ts` | §21.3 |

### 2.4 Ce qu'il ne faut pas perdre (D-035, point 5)

1. Le **moteur de contraintes** horaires selon l'âge et le cycle, avec périodes bloquées (NST-10, NST-26) : il devient le contrôle réglementaire du §12.3.
2. L'**éditeur d'horaires** semaine type ou jour par jour (NST-09) et l'**alignement sur la période** (NST-11).
3. La règle **« un seul responsable légal suffit, l'autre peut signer »** (NST-14), déjà retenue par le 7 (§10.4).
4. L'**avenant qui remet les signatures à zéro** (NST-19) et la **suspension** des signatures pendant une demande.
5. Le **papier du tuteur qui devient le document principal** avec une annexe électronique (NST-15).
6. Le **changement d'e-mail du tuteur validé** par l'établissement (NST-07) et le code **jamais envoyé à la direction** (NST-36).
7. La **révocation** d'une signature avec reconstruction du PDF (NST-17), la **relance en masse**.
8. Le **classement automatique** dans le dossier de l'élève (NST-18).
9. Les **offres déposées par les parents** et la candidature (NST-01 à NST-03) : c'est la bourse aux stages de l'établissement.
10. La **page « convention annulée »** sur les anciens liens (NST-24).
11. La **lecture automatique** des conventions papier (NST-21), en local.

### 2.5 Le code Rust existant (lot 0 au §2.6) [F]

`ent-rust/migrations/082_stage.sql`, `crates/domain-eleve/src/stage.rs`, `crates/ent-db/src/stage_repo.rs`, service `crates/ent-db/src/service.rs:15686-15809`, routes `crates/ent-api/src/routes.rs:604-610, 7456-7540` (`GET/POST /entreprises`, `POST /conventions-stage`, `GET /eleves/{id}/conventions-stage`), scénario de laboratoire `crates/ent-lab/src/main.rs:8310`, tuile de l'ancienne interface `ui.rs:4170-4240` (figée, F-001). Registre E12 : `ent_entreprise` et `ent_convention_stage` classées `dossier_eleve`, personnelles, exportées (`registre_donnees.rs:284, 351`) ; RLS posée par la 580 ; fusion d'élèves prise en charge (`domain-eleve/src/fusion.rs:79, 221`).

C'est un **embryon** : une entreprise (raison sociale, SIRET, adresse) et une « convention » (élève, entreprise, dates, nom du tuteur, statut `active/cloturee/annulee`). Il n'y a ni signature, ni horaire, ni lien externe, ni effet sur l'appel.

| N° | Gravité | Défaut [F] | Où |
|---|---|---|---|
| **RS-01** | haute | Une « convention » `active` existe **sans aucune signature** : le mot ment, et un futur branchement sur l'appel marquerait « en stage » un élève sans convention | `082_stage.sql`, `stage.rs:105` |
| **RS-02** | haute | **Droits codés par rôle** (`role_can_manage_eleves` ou administratif, direction, admin, et CPE pour les conventions), sans action `stages.*` au catalogue D-017 | `service.rs:15691-15696, 15733-15737` |
| **RS-03** | haute | Le parent lit les conventions dès qu'un **lien** existe, sans contrôle des **droits du lien** (information, interdiction) ni de la **majorité** de l'élève (un élève majeur peut refuser que ses parents voient) | `service.rs:15783-15794` |
| RS-04 | moyenne | Événement publié **hors de la transaction** de l'écriture (l'insertion est validée, puis l'événement est écrit à part) : un fait sans événement est possible | `service.rs:15698-15712, 15751-15768`, `event_store.rs:28` |
| RS-05 | moyenne | Le **nom du tuteur** (personne extérieure) est recopié dans la charge utile de l'événement, conservée longtemps et diffusée | `service.rs:15763` |
| RS-06 | moyenne | `eleve_id` **sans clé étrangère** ; FK simple vers `ent_entreprise` (E2) ; `created_by_user_id` vers le compte global au lieu de la personne (D-020) | `082_stage.sql` |
| RS-07 | moyenne | Ni `modifie_le`, ni `version`, ni journal ; aucune commande pour changer le statut | `082_stage.sql` |
| RS-08 | faible | SIRET : 14 chiffres sans contrôle de clé (Luhn) ; **pas d'unicité** par établissement (doublons d'entreprises) | `stage.rs:44-49` |
| RS-09 | faible | Aucun contrôle de chevauchement de deux stages d'un élève, ni des dates contre l'année et la scolarisation | `stage.rs:93` |
| RS-10 | faible | Une seule erreur `StageInvalide` sans motif : l'interface ne peut pas dire ce qui ne va pas | `stage.rs` |

### 2.6 Lot 0 (ST0) : correctifs urgents du Rust existant [R]

À faire **avant** tout autre lot du socle, et avant de brancher quoi que ce soit sur l'appel. Migration **2780** (additive, rejouable, E8).

1. **Renommer la sémantique sans casser** : `ent_convention_stage.statut` gagne la valeur `declaree` (stage déclaré, sans convention signée dans ScolIA) ; les lignes existantes `active` passent `declaree` ; l'API répond `statut = declaree` et le texte « convention non signée dans ScolIA » (RS-01). Aucune lecture de ces lignes ne pourra produire une situation « en stage » (test).
2. **Actions D-017** `stages.lire`, `stages.preparer`, `stages.organismes.gerer` ajoutées au catalogue (§21.1) ; les trois routes les utilisent ; le CPE perd la création (RS-02).
3. **Lecture famille** par le port du 4 : droit `voir_scolarite` du lien à la date, interdiction, élève majeur ayant refusé le partage (RS-03) ; test : parent sans droit → 403.
4. **Événement dans la transaction** (outbox du socle 1) ; charge utile réduite aux identifiants et aux dates, sans nom de tuteur (RS-04, RS-05).
5. **FK composites** `(etablissement_id, eleve_id)` → `ent_eleve` en `RESTRICT`, `(etablissement_id, entreprise_id)` → `ent_entreprise` ; `cree_par_personne_id` ajoutée à côté de `created_by_user_id` ; `modifie_le`, `version` (RS-06, RS-07).
6. **SIRET** : contrôle de clé, index unique partiel `(etablissement_id, siret) WHERE siret IS NOT NULL` après détection des doublons (rapport, pas de fusion automatique) (RS-08).
7. **Chevauchement** refusé pour un même élève (contrainte d'exclusion `btree_gist` sur `daterange(debut, fin, '[]')` des stages non annulés) ; dates dans l'année scolaire du 5 (RS-09).
8. **Erreurs typées** (`StageErreur::{SiretInvalide, DatesHorsAnnee, Chevauchement, TuteurManquant, …}`) (RS-10).
9. Tests : RLS entre deux établissements (déjà dans la matrice), refus du professeur, parent sans droit, chevauchement, événement absent si l'écriture échoue.

Critère de fin : `cargo test` vert, `grep -i jsonb` vide sur 2780, scénario `explorer_stages` du laboratoire mis à jour, aucune régression des routes existantes (contrat de réponse inchangé hormis `statut`).

---

## 3. Acteurs, besoins réels, irritants

| Acteur | Ce qu'il fait réellement | Ce qui lui fait perdre du temps | Ce qu'il redoute | Ce qui le ravirait |
|---|---|---|---|---|
| **Chef d'établissement** (public et privé) | signe toutes les conventions ; répond de l'élève en entreprise ; déclare les accidents | signer des dizaines de conventions papier en juin et en septembre ; découvrir qu'un élève est parti sans convention | l'accident d'un mineur sur une machine sans dérogation ; l'élève parti sans convention signée | un parapheur trié par date de départ, contrôles réglementaires déjà faits, signature par lot en passkey, « 0 élève parti sans convention » vérifié par le serveur |
| **DDFPT** (directeur délégué aux formations, lycée professionnel) | organise le calendrier des PFMP, le réseau d'entreprises, les visites, le suivi des attestations pour l'examen | tableurs de suivi, relances à la main, attestations manquantes en mai | un élève inscrit à l'examen sans les semaines requises ; une entreprise problématique qui revient | un tableau « semaines faites / dues » par élève et par cycle, alertes de rattrapage, carnet d'entreprises avec historique, échelonnement simulé avec l'EDT |
| **Enseignant référent** (tout professeur désigné) | aide à trouver, relit la convention, signe, visite, évalue avec le tuteur | retrouver adresses et téléphones, organiser sa tournée, remplir trois fois la même grille | ne pas savoir qu'un élève est absent en entreprise depuis trois jours | ses élèves en stage sur une carte, une tournée proposée, la grille pré-remplie par le tuteur, une alerte d'absence le jour même, la visite saisie sur mobile hors ligne |
| **Professeur principal** (collège, seconde) | suit qui a trouvé un stage d'observation, oral et rapport de stage | relances orales, listes papier | l'élève qui n'a rien trouvé la veille | « qui n'a pas trouvé » à J-30, relance automatique, offres internes |
| **Professeurs de la classe** | perdent la classe ou une partie | ne pas savoir qui est en stage, quand | faire cours à trois élèves sans le savoir | séances retirées ou maintenues automatiquement (10), plages libérées affectées au suivi |
| **Secrétariat / gestionnaire de stages** | saisit conventions, scanne, archive, répond aux entreprises | ressaisie des mêmes entreprises, papier perdu, relances | convention perdue ; mauvaise version signée | modèles pré-remplis, signatures en ligne, papier déposé et contrôlé une fois, numérotation automatique, archivage |
| **CPE / vie scolaire** | appel, absences, sécurité ; élève revenu au lycée pendant son stage | élève marqué absent alors qu'il est en stage, ou l'inverse | l'élève censé être en entreprise qui n'y est pas (et n'est pas au lycée) | appel pré-rempli « en stage », signalement immédiat du tuteur, alerte « ni en entreprise ni au lycée » |
| **Accueil** | reçoit l'élève dont le stage s'interrompt, le tuteur qui appelle | ne pas savoir à qui passer l'appel | ne pas savoir si l'élève devait être là | fiche minimale : élève en stage chez X jusqu'au …, référent joignable |
| **Infirmière, médecin scolaire** | avis d'aptitude pour travaux réglementés (mineurs, lycée pro) ; aménagements | demandes tardives, listes à la main | avis donné sans connaître le poste | liste des avis à rendre avant la date, aptitude oui/non transmise sans diagnostic (34) |
| **Comptable / gestionnaire** (privé : OGEC) | allocation PFMP (pièces), frais de déplacement des enseignants, remise de demi-pension | compter les jours faits à la main | erreur de calcul de l'allocation, ordre de mission oublié | jours effectifs calculés, barème versionné, export, remises de restauration automatiques (24, 40) |
| **AESH / coordinateur ULIS** | accompagne un élève à besoins particuliers dans la recherche et parfois en stage | recherche d'entreprises accueillantes | une entreprise mal préparée | aménagements transmis sous forme de consignes consenties, entreprise « déjà accueillante », visite renforcée |
| **Familles** (dont séparées, recomposées, tutelle, ASE) | cherchent un stage (3e, seconde), signent, s'inquiètent | papiers à imprimer, signer, scanner ; ne pas savoir où en est la convention | l'enfant mineur exposé à un danger ; ne pas être informées (parent séparé) | signature en ligne par le bon titulaire, état de la convention visible, horaires et adresse, contact du référent, notification d'absence |
| **Élève mineur** | cherche, candidate, co-signe, va en entreprise, rédige un rapport | perdre sa convention ; ne pas savoir qui appeler | un tuteur absent ou abusif ; être noté sans comprendre | ses offres, ses candidatures, sa convention signée, ses horaires sur le téléphone, un bouton « j'ai un problème » qui joint le référent |
| **Élève majeur** | signe seul ; peut cacher l'information à ses parents (s'il le décide, socle 2) | — | que ses parents soient sollicités sans son accord | décider seul ; recevoir lui-même l'allocation |
| **Tuteur en entreprise** (sans compte) | accueille, encadre, signale les absences, évalue, signe l'attestation | trois formulaires papier différents, un compte à créer, des relances | un accident ; un élève absent qu'il ne sait pas signaler | un seul lien, un code, tout sur une page mobile : horaires, contacts, absence en deux clics, grille d'évaluation courte, attestation pré-remplie |
| **Signataire / RH de l'entreprise** | signe la convention, gère l'assurance, la gratification éventuelle | papier, imprimante, scanner | une convention non conforme | aperçu clair, signature par code, copie signée immédiate |
| **Plateformes nationales** (stage de seconde, offres) | génèrent des conventions (seconde) | double saisie | — | import du PDF signé, rattachement à l'élève |
| **Inspection, DPO, auditeur** | contrôlent conformité et données | données éparpillées | données de tiers conservées sans limite | registre des traitements, durées appliquées, journal des accès |
| **Attaquant** (élève, tiers, entreprise malveillante, parent interdit) | tente d'usurper un lien, de falsifier une signature, d'apprendre l'adresse d'un élève protégé | — | — | (ce que le socle doit empêcher, §21.4) |

---

## 4. L'année des stages vue du terrain [H, à confirmer avec Florian]

| Moment | Collège (3e, SEGPA, ULIS) | Lycée général et technologique (seconde) | Lycée professionnel (CAP, bac pro) |
|---|---|---|---|
| **Juin – juillet (année N-1)** | — | stage de seconde (deux semaines, fin d'année) | calendrier des PFMP de l'année suivante arrêté ; conventions des PFMP de septembre préparées |
| **Septembre** | information aux familles sur le stage de 3e | — | PFMP de début d'année (terminale) ; avis d'aptitude des mineurs ; liste des dérogations « travaux réglementés » |
| **Octobre – novembre** | recherche du stage, offres, conventions | — | PFMP de première ; visites |
| **Décembre – février** | **stage d'observation** (une semaine, date fixée par l'établissement) ; rapport et oral | information sur le stage de seconde, recherche | PFMP de seconde ; rattrapages |
| **Mars – mai** | — | conventions générées par la plateforme nationale, signatures | PFMP ; **contrôle des semaines** avant l'inscription définitive à l'examen ; attestations |
| **Juin** | — | **stage de seconde** | dernières attestations, évaluations certificatives, allocation PFMP finale, bilan des entreprises |
| **Toute l'année** | stages individuels (élèves décrocheurs, dispositifs relais, immersion en lycée pro) | mini-stages d'immersion | rupture, avenants, accidents, absences, rattrapages |

Pointes de charge : septembre (conventions de terminale) et mai–juin (seconde, attestations). Le socle est conçu pour le **traitement en lot** à ces moments (préparation, signature, relance) et pour le **cas individuel** le reste du temps.

---

## 5. Ce qui est réglementaire (vérifié le 2026-11-11)

Recherche faite par un sous-agent sur Légifrance, code.travail.gouv.fr, les **PDF officiels** du BO et d'éduscol, service-public.gouv.fr (les pages HTML d'éduscol et d'education.gouv.fr renvoient 403 ; leurs PDF ont été lus). **[V]** lu sur la source ; **[H]** non confirmé. Légifrance a été lu par un outil qui en donne un extrait fidèle, pas la copie intégrale : les citations exactes sont à relire avant tout texte opposable.

### 5.1 Statut des textes (changements récents)
- **Collège et LGT** : la **circulaire du 21-11-2025** (NOR MENE2517062C, BO n° 47 du 11-12-2025) couvre visites d'information, séquences d'observation et stages au collège et au lycée général et technologique ; elle **abroge** les circulaires du 28-03-2024 (seconde) et du 12-07-2024 (collège) [V].
- **PFMP** : convention-type DGESCO de **septembre 2023**, avec quatre annexes (pédagogique, financière, attestation type, évaluation de la qualité de l'accueil), qui actualise la circulaire n° 2016-053 [V].
- **Allocation PFMP** : décret n° 2023-765 et arrêté du 11-08-2023, arrêté modifié le **29-11-2025** (forfaits et plafonds) [V], puis le 22-04-2026 (formations en un an, BNMA) [V d'après la fiche éduscol, texte non lu].
- **Terminale bac pro** : **arrêté du 22-04-2026** (NOR MENE2609636A, JO du 12-05-2026), applicable à la rentrée 2026 : **deux semaines dédiées** organisées par l'établissement (immersion en entreprise en plus des PFMP certificatives, ou préparation à la poursuite d'études) [V]. L'organisation de 2025-2026 (quatre semaines, arrêté du 27-10-2025 et note du 15-12-2025) ne valait que pour cette année-là [V].

### 5.2 Cadre légal
| Règle | Texte | Effet dans ScolIA |
|---|---|---|
| Les articles **L124-1 à L124-20** du code de l'éducation s'appliquent **aussi aux PFMP** du secondaire | C. éduc. L124-1 s. ; visas de la convention-type [V] | moteur de règles commun |
| Convention tripartite obligatoire ; points réguliers référent – tuteur | L124-1 [V] | R-SG1, §13.5 |
| L'établissement aide à trouver le lieu et **désigne l'enseignant référent** ; le **CA fixe le nombre de stagiaires par référent** (public) | L124-2 [V] | STG-130 ; alerte de charge (S-3910) |
| Le stagiaire remet une **évaluation de la qualité de l'accueil**, sans effet sur son évaluation | L124-4 [V] | STG-265 |
| **Six mois au plus** par année d'enseignement dans un même organisme | L124-5 [V] | contrôle bloquant |
| **Gratification** obligatoire au-delà de deux mois dans un même organisme sur l'année, **PFMP comprises** ; minimum 15 % du plafond horaire de la sécurité sociale | L124-6 ; convention-type art. 7 [V] ; valeur 2026 du plafond **[H]** (URSSAF injoignable) | §15.2, R-SG13 |
| Nombre maximal de stagiaires par organisme fixé par l'**autorité académique** pour les PFMP | L124-8 [V] | alerte (valeur académique en paramètre, S-3911) |
| **Tuteur obligatoire** désigné par l'organisme | L124-9 [V] | exigence bloquante |
| Accès au restaurant d'entreprise ou aux titres-restaurant, frais de transport, congés liés à la parentalité | L124-13 [V] | annexe financière ; cas limite CSG-31 |
| Durées maximales de présence des salariés ; **décompte des présences obligatoire** ; tâches dangereuses interdites | L124-14 [V] | jours effectifs (§7.1) |
| Durée appréciée **en présence effective** ; 7 h = 1 jour, 22 jours = 1 mois | L124-18, D124-6 [V] | calcul des jours |
| Moins de 16 ans : convention établissement – organisme ; **statut scolaire conservé** | D331-1, D331-3, D331-4 [V] | §1 thèse |
| Séquences d'observation en 4e, 3e et au lycée **sans condition d'âge** | D331-6 (décret 2019-636) [V] | pas de blocage d'âge pour l'observation |
| Pendant visites et observation : **aucune machine interdite aux mineurs, aucune manipulation, aucun travail même léger** | D331-9 [V] | case « observation seulement » obligatoire ; aucun travail réglementé |
| Stages d'initiation et d'application **à partir de 14 ans** | D331-10 à D331-14, D332-14 [V via la circulaire] | âge minimal par dispositif |
| Séquence d'observation **obligatoire en seconde GT** | D333-3-1 (décret 2023-1111) [V] | dispositif `seconde_gt` |
| Interdiction d'employer des moins de 16 ans, sauf observation (2°) et stages ou PFMP (3°) | C. trav. L4153-1 [V] | — |
| Convention obligatoire ; **interdite** si les services de contrôle ont relevé des conditions dangereuses | L4153-2 [V] | organisme `interdit` ; motif « signalement de l'inspection » |
| Le mineur n'effectue que des travaux légers | D4153-4 [V] | — |
| **Travaux réglementés** (D4153-15 à D4153-37) par **dérogation** de 3 ans : évaluation des risques, formation à la sécurité, encadrement compétent, **avis médical d'aptitude annuel** ; déclaration à l'inspection du travail **par l'entreprise et par le chef d'établissement**, chacun pour ce qui le concerne ; à partir de **15 ans** | R4153-40, R4153-41 ; convention-type art. 13 [V] | exigences : date de la déclaration de l'organisme, déclaration de l'établissement (registre, §13), aptitude du 34 |
| **Durée** des mineurs : 8 h par jour, 35 h par semaine ; dérogation de 5 h par semaine au plus par l'inspecteur après avis du médecin ; jamais plus que les adultes de l'entreprise | L3162-1 [V] | blocage ; dérogation saisie avec sa référence |
| **Repos** : deux jours consécutifs par semaine, dimanche compris ; quotidien de **14 h avant 16 ans**, **12 h de 16 à 18 ans** ; **pause de 30 min** au-delà de 4 h 30 | convention-type art. 10 [V] | blocage |
| **Nuit interdite** : 22 h – 6 h de 16 à 18 ans, **20 h – 6 h avant 16 ans**, sans dérogation | convention-type art. 10 [V] | blocage |
| **Jours fériés** non travaillés par les mineurs, sauf dérogations sectorielles | L3164-6, R3164-2, L3164-8 [V via la FAQ] | blocage avec dérogation sectorielle déclarée |
| Travail de nuit d'un **majeur** seulement s'il est **nommément désigné** par le chef d'établissement | convention-type art. 9 [V] | autorisation nominative (STG-188) |

**Écart avec Next.js** [F] : `stage-constraints.ts` limite à **7 h par jour** et 30 h par semaine avant 15 ans. Ces valeurs viennent de modèles académiques ou de l'ancienne réglementation de l'observation [H] ; le texte lu donne 8 h par jour et 35 h pour les mineurs en PFMP. ScolIA garde **les deux niveaux** : la valeur légale (blocage) et la valeur de l'établissement plus stricte (alerte ou blocage selon le dispositif, S-3912), pour ne pas perdre la pratique actuelle.

### 5.3 Conventions et signataires
- **PFMP** (convention-type, art. 3) : chef d'établissement, représentant de l'organisme, **élève ou son représentant légal s'il est mineur**, **enseignant référent**, **tuteur** ; la convention est ensuite **adressée à la famille pour information** [V]. Dans le public, une **délibération du CA** approuve la convention-type et autorise le chef d'établissement à signer [V] ; le privé n'a pas d'équivalent légal (adaptation à faire, Q-SG1) [H].
- Contenu à modéliser [V] : identification de l'organisme (dont numéro d'immatriculation) et **date de la déclaration de dérogation** ; tuteur, référent, élève ; **dates, nombre de jours, horaires jour par jour** ; activités, compétences du référentiel et blocs ; travaux soumis à dérogation ; modalités de suivi (sur site ou à distance) ; évaluation formative ou certificative ; frais (repas, transport, hébergement) ; gratification ; **assureurs et numéros de contrat** ; habilitation électrique et carnet de formation (art. 14) ; **suspension ou résiliation** (art. 19) ; **validation alternative ou report** en cas d'interruption (art. 20).
- **Observation, collège et seconde** (circulaire 2025, partie V) : chef d'établissement et organisme, **avec les titulaires si l'élève est mineur** ; objectifs, calendrier, horaires, encadrement, activités, suivi, évaluation, sécurité, frais, assurances ; nom du référent et coordonnées de l'établissement [V]. En seconde, **une convention par lieu** si les deux semaines se font dans deux lieux [V]. Modèles éduscol (collège, lycée, visite, 3e prépa-métiers, 3e SEGPA, STHR) **non lus** (accès bloqué) [H] : à relire au lot ST1.
- **1élève1stage** : plateforme nationale des offres de séquences d'observation (seconde surtout, 3e, et 4e dans le privé), connexion des élèves par ÉduConnect [V] ; la convention y serait pré-remplie et signée électroniquement [H, documents académiques] ; un témoignage d'usager signale qu'elle ne contrôle pas que toutes les signatures sont présentes [H] : ScolIA **contrôle à l'import** (R-SG18).

### 5.4 Calendriers [V]
- **3e** : 5 jours, consécutifs ou non, entre la Toussaint et les vacances de printemps.
- **Seconde GT** : deux semaines consécutives en fin d'année (15–26 juin en 2026) ; dispenses : service national, mobilité, 2de STHR.
- **Bac pro** (rentrée 2026) : 4 à 6 semaines en seconde, 6 à 8 en première, 6 en terminale, **16 à 20** sur le cycle, plus 2 semaines dédiées en terminale ; seule la PFMP obligatoire compte pour l'examen (décret 2024-542 [V, fiche éduscol]). Les conséquences d'un manque de semaines pour l'examen ne sont **pas vérifiées** [H] : ScolIA alerte et laisse la décision au DDFPT et au chef d'établissement (R-SG12).
- Un élève sans lieu d'accueil est **accueilli dans son établissement** [V] : ScolIA le prévoit (il est attendu en cours ou en permanence, §13.2).

### 5.5 Argent [V, FAQ éduscol d'octobre 2026, arrêté du 29-11-2025]
- **Allocation PFMP** pour les élèves sous statut scolaire du **public et du privé sous contrat d'association** en CAP, bac pro, BMA, BNMA, DTMS, CS, FCIL (et AvenirPro+) ; exclus : apprentis, formation continue, BTS, bacs technologiques, stages d'observation.
- **Forfait journalier** × jours **entiers** effectués et attestés : CAP 1re année 10 € ; CAP 2e année et CAP en un an 15 € ; bac pro seconde 10 €, première 15 €, terminale et bac pro en un an 20 € ; BMA/DTMS 15 puis 20 € ; CS et FCIL 15 € (niveau 3) ou 20 € (niveau 4).
- **Plafonds annuels** : CAP 1re année 350 €, CAP 2e année 525 €, bac pro seconde 300 €, première 600 €, terminale **800 €** pour l'entrée en terminale à la rentrée 2026 (1 000 € à la rentrée 2025, 1 200 € à la rentrée 2024).
- Versement par l'**ASP** via **ApLyPro** : décision d'attribution annuelle validée par le chef d'établissement ; **état liquidatif** après chaque PFMP, au plus tard deux mois après la fin, strictement conforme à l'attestation (sinon remboursement exigible) ; pas d'acompte.
- **Bénéficiaire** : l'élève majeur ; l'élève mineur sur son compte avec **autorisation écrite annuelle** des titulaires ; sinon un titulaire ; RIB SEPA ; cas ASE, mineur non accompagné, tutelle, mandat.
- Pièces (identité, filiation, RIB, autorisation, convention, avenants, attestation) **conservées 10 ans** ; seules les coordonnées bancaires sont saisies dans ApLyPro.
- Absences **jamais payées** ; rattrapage l'année suivante au tarif d'origine ; semaine « poursuite d'études » non payée ; prescription quadriennale ; non imposable ; **cumulable** avec la gratification.

### 5.6 Assurance et accidents [V]
- **PFMP, stages d'initiation et d'application** : régime des accidents du travail (C. séc. soc. L412-8 2° a et b ; D412-6). **L'organisme d'accueil déclare** l'accident à la CPAM par recommandé **sous 48 h** (hors dimanches et jours fériés, R412-4) et en adresse **sans délai une copie** au chef d'établissement.
- **Visites et observation** : **pas** de régime accident du travail ; responsabilité administrative et substitution de l'État (C. éduc. L911-4) ; l'organisme prévient immédiatement l'établissement ; le chef d'établissement établit une **déclaration d'accident scolaire en deux exemplaires sous 48 h**.
- **Responsabilité civile** : l'organisme garantit la sienne (l'État est son propre assureur) ; en PFMP, le chef d'établissement assure la responsabilité civile de l'élève (art. 17) ; en observation, assurance scolaire « vivement recommandée » ; **dans le privé sous contrat, l'établissement souscrit ses propres polices**.

### 5.7 Statut, absences, sécurité de l'élève [V]
- L'élève reste **sous statut scolaire et sous l'autorité du chef d'établissement** ; il n'entre pas dans l'effectif de l'organisme (art. 5, D331-4).
- L'**obligation d'assiduité** (L511-1) s'applique pendant la PFMP ; le tuteur prévient **sans délai** l'établissement d'une absence, d'une maladie ou d'un incident ; absentéisme et discipline sont traités ensemble par le chef d'établissement et l'organisme (art. 19).
- L'attestation ne porte que les jours effectués (D124-9, modèle national) ; elle est signée le dernier jour, indique la durée **effective en jours**, les activités, les compétences et la gratification, et sert de base à l'allocation.
- **Discriminations et violences sexistes et sexuelles** : l'élève les signale au référent ; l'organisme l'informe de ses procédures de signalement dès son arrivée.
- **Handicap et santé** : avec un PPS, l'équipe et le référent s'assurent des compensations, aménagements et de l'accessibilité ; avec un PAI, compatibilité vérifiée, avis du médecin de l'Éducation nationale si besoin, trousse d'urgence ; **les titulaires informent le lieu d'accueil** : le dossier médical n'est pas transmis (circulaire 2025, V.B).
- **Mobilité à l'étranger** : autorisation du chef d'établissement (circulaire 2016-091, note du 04-08-2022, DAREIC) ; allocation due pour une PFMP à l'étranger avec convention tripartite.
- **Bureau des entreprises** dans chaque lycée professionnel public (circulaire du 24-05-2023 [V, fiche éduscol ; texte non lu]).

### 5.8 Données, archives, signature
- **Archives** (instruction n° 2005-003 du 22-02-2005, BO n° 24 du 16-06-2005, tableau 2.3 « Stages ») [V] : offres et fiches entreprises 1 an puis destruction ; conventions **avec incidence financière** 1 an après apurement des comptes ; **sans incidence financière** 2 ans ; **états récapitulatifs 5 ans puis conservation** ; rapports de stage 1 an puis tri ; attestations et évaluations suivent le dossier de l'élève ; copies d'attestations des formations à risque ou liées à la sécurité 50 ans.
- **Contradiction** : l'ASP exige **10 ans** pour les pièces de l'allocation, conventions et attestations de PFMP comprises [V] ; ScolIA retient **10 ans** pour tout stage ayant ouvert droit à l'allocation, et les durées de 2005 pour les autres (§21.3, au DPO).
- **CNIL** : aucune doctrine spécifique sur les tuteurs ou les stages trouvée [H] ; la minimisation se règle sur les champs des modèles officiels (tuteur : nom, fonction, courriel, téléphone ; élève : identité, date de naissance, adresse, classe).
- **Signature électronique** : seule la signature **qualifiée** bénéficie de la présomption de fiabilité (C. civ. 1367, décret 2017-1416 ; Cass. 3e civ., 5-03-2026, n° 24-21.034) [V, extraits] ; une signature simple ou avancée reste recevable mais doit être **prouvée** en cas de contestation ; aucun texte n'impose un niveau pour les conventions de stage [H] : ScolIA garde le **dossier de preuve** du 7 (identité, horodatage, empreinte, intention).

### 5.9 Ce qui reste à vérifier [H]
Valeur 2026 du plafond horaire de la sécurité sociale ; modèles éduscol de convention d'observation (collège, seconde) et éventuel modèle de PFMP actualisé pour 2026 ; sort des plateformes « Mon stage de 3e » et « Mon stage de seconde » ; doctrine CNIL ou référentiel d'archives postérieur à 2005 ; conséquences d'un manque de semaines pour l'examen ; établissements sous contrat simple ; autorisation de travail d'un élève étranger sous statut scolaire ; origine des valeurs 7 h et 30 h de Next.js ; texte de l'arrêté du 22-04-2026 sur l'allocation.

### 5.10 Sources (consultées le 2026-11-11)
- Circulaire du 21-11-2025, MENE2517062C, BO n° 47 du 11-12-2025 : https://www.education.gouv.fr/sites/default/files/document/bulletin-officiel-ndeg-47-du-11-decembre-2025-517124.pdf [V]
- Note de service du 15-12-2025, MENE2535318N, BO n° 48 : https://www.education.gouv.fr/sites/default/files/document/Bulletin%20officiel%20n%C2%B0%2048%20du%2018%20d%C3%A9cembre%202025-479657.pdf [V]
- FAQ allocation PFMP (éduscol, octobre 2026) : https://eduscol.education.gouv.fr/sites/default/files/document/faq-allocation-des-pfmp-pour-les-lyceens-professionnels-102372.pdf [V]
- Convention-type PFMP et annexes (DGESCO, 09-2023) : https://eduscol.education.gouv.fr/sites/default/files/document/fiche-6convention-type-pfmp-et-annexespdf-100929.pdf [V]
- Cadre juridique de la réforme du LP 2026 : https://eduscol.education.gouv.fr/sites/default/files/document/fiche-formation-pro-cadre-juridique-de-la-reforme-lp-105714.pdf [V]
- FAQ cursus bac pro : https://eduscol.education.gouv.fr/sites/default/files/document/faq-reorganisation-du-cursus-preparant-au-baccalaureat-professionnel-104592.pdf [V partiel]
- Arrêtés du 29-11-2025 (https://www.legifrance.gouv.fr/jorf/id/JORFTEXT000053010495), du 27-10-2025 (https://www.legifrance.gouv.fr/jorf/id/JORFTEXT000052572482), du 22-04-2026 (https://www.legifrance.gouv.fr/jorf/id/JORFTEXT000054063222) [V]
- Code de l'éducation L124-1 à L124-20 : https://www.legifrance.gouv.fr/codes/id/LEGISCTA000029233447 ; D331-1 à D331-15 : https://www.legifrance.gouv.fr/codes/id/LEGISCTA000006151437 ; D333-3-1 : https://www.legifrance.gouv.fr/codes/article_lc/LEGIARTI000048485993/ [V]
- Code civil 1367 : https://www.legifrance.gouv.fr/codes/article_lc/LEGIARTI000032042456 [V extraits]
- Code du travail L4153-1, L4153-2, L4153-3, D4153-4, R4153-40, R4153-41, L3162-1 : https://code.travail.gouv.fr/code-du-travail/l4153-1 (même forme d'adresse pour chaque article) [V]
- Service-public, actualité A18028 du 12-02-2026 : https://www.service-public.gouv.fr/particuliers/actualites/A18028 [V]
- Instruction 2005-003, BO n° 24 du 16-06-2005 : https://www.education.gouv.fr/bo/BoAnnexes/2005/24/tableaux_encart24.pdf [V]
- Non lus : circulaire du 24-05-2023 (bureaux des entreprises), arrêté du 29-11-2023 (stage de seconde, art. 8-1), URSSAF (plafond horaire), CNIL (aucune doctrine trouvée) [H].

---

## 6. Branchements sur le cœur

### 6.1 Qui possède quoi

| Donnée | Propriétaire | Le 39 fait |
|---|---|---|
| Organismes d'accueil, sites, contacts (tuteurs, signataires), offres, candidatures | **39** | possède |
| Dispositifs (types de stage) et leurs règles, périodes collectives, dossiers de stage, horaires, activités, conventions (données), avenants, visites, évaluations de stage, attestations (données), décompte des semaines, allocation et gratification calculées, incidents de stage | **39** | possède |
| L'élève, sa formation, sa capacité (mineur, majeur, protégé) | 2 | lit à la date |
| Titulaires de l'autorité parentale, droits des liens, interdictions, coordonnées vérifiées, coordonnées bancaires de l'élève majeur | 4 | lit ; calcule les signataires et les destinataires |
| Classes, formations, diplômes préparés, calendrier, jours fériés, vacances | 5 | lit ; les périodes de PFMP sont des **périodes du 5 de nature `entreprise`** créées par commande du 39 (§9.2) |
| Personnels, missions « enseignant référent de stage » (type propriété 39), habilitations de signature | 6 | **attribue** la mission par le port `missions::attribuer` ; lit les habilitations |
| Documents, versions, circuits de signature, liens nominatifs des externes, papier, coffre | 7 | **commande** : émettre la convention, l'avenant, l'attestation ; lire l'état du circuit |
| Avis et envois | 8 | déclare ses natures ; demande les envois |
| Séances | 10 | publie `pfmp.periode_planifiee` ; le 10 retire ou maintient les séances (§11.9 du 10) ; propose les plages libérées pour les visites |
| Situations et absences | 12 | **pose** une situation `pfmp` (ou `stage`) par commande ; les absences en entreprise sont des absences du 12 (AS-118), signalées par le tuteur via le lien du 39 (AS-240) |
| Incident disciplinaire, mesure | 13 | signale ; le 13 décide |
| Accueil (élève revenu au lycée, tuteur au téléphone) | 14 | lit la projection « en stage » |
| Fils de discussion | 16 | ouvre un fil « convention » ; les externes y écrivent par leur lien |
| Consentements (transmission d'un aménagement, image) | 18 | campagne ou formulaire ; lit le résultat |
| Rendez-vous de visite, réunion de préparation | 19 | crée par commande |
| Évaluations notées (CCF) | 21 | transmet la proposition ; le 21 enregistre la note |
| Référentiels de compétences, positionnements | 22 | lit les compétences ; écrit les positionnements par commande (propositions des tuteurs, 22 §2451) |
| Bulletins | 23 | fournit la mention « PFMP : semaines faites / dues, appréciation » |
| Remise de demi-pension, frais | 24, 25 | événement ; le 24 décide |
| Aide aux frais de transport ou d'hébergement en stage | 26 | lit l'éligibilité |
| Aptitude aux travaux réglementés, consignes d'aménagement | 34, 35 | lit **oui / non / à revoir** et des consignes, jamais un diagnostic |
| Orientation, parcours Avenir | 37 | publie les stages d'observation au parcours |
| Inscription à l'examen, attestations exigées | 38 | fournit le décompte certifié des semaines |
| Repas | 40 | publie les jours de stage (cause « stage ») |
| Nuits d'internat | 41 | publie les jours de stage (l'interne ne dort pas à l'internat sauf indication) |
| Visite collective d'entreprise (classe entière, une journée) | 42 | **n'est pas un stage** : c'est une sortie |
| Indicateurs | 47 | expose |
| Frais de déplacement des référents | 6 / 49 | demande l'ordre de mission |
| Stages de l'enseignement supérieur et apprentissage | 51 | **hors 39** ; le 51 réutilise les organismes et contacts |

### 6.2 Ports consommés

| Port | Socle | Usage |
|---|---|---|
| `eleve::a_la_date(eleve, jour)` | 2 | identité, formation, capacité, majorité, mesure de protection |
| `dossier::titulaires(eleve, jour)`, `dossier::droits_lien`, `dossier::interdictions` | 4 | signataires calculés, projections familles, refus d'une adresse saisie |
| `structure::formation(eleve, annee)`, `structure::calendrier`, `structure::creer_periode(nature=entreprise)` | 5 | dispositif proposé, périodes collectives, jours ouvrés |
| `missions::attribuer(type=referent_stage, agent, perimetre=eleves, du, au, origine=39)` | 6 | référent |
| `habilitations::peut_signer(personne, nature=convention_stage)` | 6 | signataire de l'établissement |
| `documents::emettre(nature, donnees, circuit)`, `documents::etat_circuit`, `documents::lien_externe` | 7 | convention, avenant, attestation, évaluation signée |
| `notifications::avis` | 8 | relances, alertes |
| `seances::du_public(eleve, du, au)` | 10 | séances concernées, plages libérées |
| `assiduite::poser_situation(nature=pfmp, eleve, du, au, horaires, origine=stage:{id})`, `assiduite::absences(eleve, origine)` | 12 | appel pré-rempli, décompte |
| `conversations::fil(objet=convention)` | 16 | fil de la convention |
| `formulaires::consentement(eleve, objet)` | 18 | transmission d'un aménagement |
| `agenda::rdv` | 19 | visites |
| `competences::referentiel(diplome)`, `competences::proposer_positionnement` | 22 | grilles |
| `sante::aptitude(eleve, nature=travaux_reglementes)`, `sante::consignes(eleve, contexte=stage)` | 34 | minimal |
| `besoins::amenagements(eleve, contexte=stage)` | 35 | consignes consenties |

### 6.3 Ports exposés par le 39

| Port | Consommateurs |
|---|---|
| `stages::en_stage(eleve, jour) -> Option<{organisme_libelle, du, au, horaires_du_jour, referent}>` | 12, 14, 40, 41, 29 |
| `stages::jours_effectifs(eleve, dispositif, cycle)` | 23, 38, 47 |
| `stages::semaines_dues_faites(eleve, cycle)` | 23, 38 |
| `stages::organismes(recherche)` | 51, 37 |
| `stages::periodes(classe, annee)` | 10, 19 |

### 6.4 Répercussions automatiques (le cœur bouge, le stage suit)

| Événement du cœur | Effet sur le 39 |
|---|---|
| L'élève quitte l'établissement (2) | stages futurs annulés (motif « départ »), conventions non signées caduques, signataires prévenus (N3, aperçu) ; stage en cours : **alerte** au référent et à la direction (l'entreprise doit être prévenue) |
| L'élève change de classe ou de formation (2, 5) | dispositif et période revus ; conflit signalé, jamais modifié seul |
| L'élève devient majeur pendant la préparation (2) | circuit de signature recalculé : il signe seul ; les titulaires ne sont plus sollicités sauf accord (7 §10.4) ; les liens titulaires en attente révoqués |
| Un titulaire perd son droit, décès, interdiction nouvelle (4) | signataire retiré, lien révoqué, circuit recalculé ; une signature déjà donnée reste valide si elle l'était à sa date |
| Coordonnées du titulaire changées (4) | lien renvoyé à la nouvelle adresse seulement après vérification ; ancien lien révoqué |
| Mission de référent retirée (6) | stage « sans référent » dans la liste de travail du DDFPT |
| Période du 5 modifiée (vacances, jour férié) | horaires et jours effectifs recalculés ; avenant proposé si la convention signée est touchée |
| Séance ajoutée pendant le stage (10) | l'élève n'y est pas attendu |
| Absence en entreprise saisie (12) | jours effectifs, compteur de gratification et allocation recalculés ; alerte au référent |
| Aptitude « à revoir » (34) | stage avec travaux réglementés bloqué au départ |
| Organisme radié ou fermé (vérification manuelle ou import, §10.1) | stages futurs signalés ; aucune annulation automatique |

---

## 7. Concepts et vocabulaire

### 7.1 Vocabulaire fixé [R]

| Terme | Sens dans ScolIA |
|---|---|
| **Organisme d'accueil** | entreprise, association, administration, collectivité, établissement scolaire ou de santé, organisme étranger, qui accueille un élève. Table `ent_entreprise` (nom historique conservé). |
| **Site** | lieu où l'élève se rend, distinct du siège (agence, chantier, magasin). |
| **Contact** | personne physique de l'organisme connue de l'établissement : **tuteur**, **signataire** (représentant légal ou délégué), **RH**, autre. Jamais un compte ScolIA. |
| **Tuteur** | le contact désigné par l'organisme pour encadrer l'élève. **Ne pas confondre** avec la mission « tuteur » du socle 6 ni avec le tuteur légal d'un mineur ou d'un majeur protégé (4, 18) : l'interface dit toujours « tuteur en entreprise ». |
| **Enseignant référent** | le personnel de l'établissement qui suit l'élève (mission du 6, type propriété du 39). Le socle 6 §1 dit « tuteur de stage » pour cette mission : **contradiction de vocabulaire** relevée, le 39 propose « enseignant référent de stage » (§31). |
| **Dispositif** | le type de stage, qui porte ses règles : `observation_college` (4e, 3e, SEGPA), `seconde_gt`, `pfmp` (CAP, bac pro), `immersion` (mini-stage en lycée professionnel ou en entreprise d'un élève hors parcours), `stage_individuel` (dispositif relais, décrochage, prépa-métiers), `mobilite_internationale`, `autre_encadre`. |
| **Période** | intervalle de dates pendant lequel un public (classe, groupe, liste) est en stage ; collective (fixée par l'établissement ou nationale) ou individuelle (rattrapage, décalage). |
| **Stage** (dossier de stage) | un élève, un organisme, des dates, des horaires, un référent, un tuteur, une convention : l'objet central du socle. Un élève peut avoir plusieurs stages dans une même période (deux employeurs, deux sites) s'ils ne se chevauchent pas. |
| **Convention** | le document tripartite (organisme, établissement, élève ou ses titulaires) **signé** ; données au 39, document et signatures au 7. Une convention a des **versions** ; un **avenant** est un nouveau document lié. |
| **Jour effectif** | jour où l'élève devait être présent selon les horaires et où aucune absence n'est constatée. Base du décompte des semaines, de l'allocation et de la gratification. |
| **Semaine de PFMP** | unité réglementaire du décompte pour l'examen (§5) ; calculée, jamais saisie. |
| **Recherche** | état d'un élève attendu en stage qui n'a pas encore d'organisme ; il est suivi, relancé, aidé. |

### 7.2 Distinctions qui manquent aujourd'hui [R]

1. **Stage déclaré** (une ligne qui dit qu'un élève ira chez X) ≠ **stage conventionné** (convention signée de toutes les parties). Seul le second localise l'élève (R-SG1).
2. **Absence au stage** (l'élève devait être en entreprise et n'y est pas : absence du 12, origine stage) ≠ **retour au lycée** (stage interrompu, l'élève est attendu en cours) ≠ **jour non travaillé** (fermeture de l'entreprise, jour férié : ni absence ni jour effectif, rattrapage éventuel).
3. **Allocation PFMP** (versée par l'État à l'élève de lycée professionnel, §5) ≠ **gratification** (due par l'organisme au-delà d'une durée, §5) ≠ **avantages** (repas, transport offerts par l'organisme).
4. **Organisme** ≠ **personne** : les données de l'organisme (raison sociale, SIRET) sont publiques ; celles du tuteur sont des données personnelles de tiers (§21.3).

---

## 8. Cycle de vie

### 8.1 États du stage [R]

```
recherche ──► trouve ──► en_preparation ──► a_valider ──► a_signer ──► signe ──► en_cours ──► termine ──► evalue ──► clos
    │            │             │                │             │           │           │
    └────────────┴─────────────┴────────────────┴─────────────┴───────────┴─► annule  ├─► interrompu ─► (repris | clos)
                                                │                                     │
                                                └─► a_corriger ─► a_valider           └─► rompu (rupture anticipée) ─► clos
```

| État | Sens | Effets |
|---|---|---|
| `recherche` | élève attendu dans une période, sans organisme | liste « sans lieu » ; relances ; offres proposées |
| `trouve` | organisme et dates connus, convention non commencée | rien sur l'appel |
| `en_preparation` | convention en cours de saisie (élève, famille, référent ou secrétariat) | contrôles réglementaires en direct |
| `a_valider` | soumise à l'établissement (référent, puis secrétariat ou DDFPT) | — |
| `a_corriger` | renvoyée avec motif | — |
| `a_signer` | convention émise par le 7, circuit ouvert | relances ; **J-7, J-3, J-1** alertes si incomplète |
| `signe` | toutes les signatures exigées réunies | **situation `pfmp`/`stage` posée au 12** ; jours de stage publiés au 40 et au 41 ; séances concernées signalées au 10 |
| `en_cours` | entre le premier et le dernier jour (calculé) | tuteur : lien actif pour les absences ; référent : visites |
| `interrompu` | arrêt temporaire (maladie, fermeture, incident) | situation suspendue : l'élève redevient attendu au lycée sauf arrêt maladie (absence) |
| `rompu` | fin anticipée décidée (§13.4) | situation close à la date ; rattrapage à prévoir |
| `termine` | dernier jour passé | attestation demandée au tuteur |
| `evalue` | évaluation et attestation reçues | décompte certifié |
| `clos` | dossier complet ou clos avec motif | lecture seule ; conservation §21.3 |
| `annule` | abandonné avant le début | situation retirée si posée ; signataires prévenus |

Le serveur seul change l'état (R-SG3) ; chaque transition est une commande journalisée avec son auteur, son motif et l'état de départ attendu (verrou optimiste).

### 8.2 Ce qui part et quand

| Moment | Envoi (N3, aperçu des destinataires calculés) |
|---|---|
| ouverture d'une période (J-60 par défaut, S-3901) | information aux familles et aux élèves ; offres disponibles |
| J-30 | relance des élèves « sans lieu » et de leurs titulaires ; liste au référent et au DDFPT |
| émission de la convention | liens aux signataires externes, avis aux titulaires et à l'élève, tâche aux signataires internes |
| J-7, J-3, J-1 si non signée | relance des signataires manquants ; **J-1** : alerte direction, référent, famille « l'élève ne pourra pas partir » |
| premier jour | lien du tuteur actif ; rappel de son usage (absences, contacts) |
| absence signalée | avis à la famille selon les règles du 12 ; avis au référent |
| dernier jour | demande d'évaluation et d'attestation au tuteur |
| J+15 sans attestation | relance du tuteur ; liste du DDFPT |

---

## 9. Dispositifs et périodes

### 9.1 Dispositifs fournis par ScolIA (D-023 : modèles prêts)

Chaque dispositif est une ligne de référence globale (`ref_stage_dispositif`, E12 `globale_reference`) avec ses règles **typées et versionnées** (`ref_stage_regle`, une ligne par règle, date d'effet, texte source) ; l'établissement **active** les dispositifs qui le concernent et peut **durcir** une règle, jamais l'assouplir sous le minimum légal (R-SG5).

| Dispositif | Public | Durée type | Particularités |
|---|---|---|---|
| `observation_college` | 4e, 3e (dont prépa-métiers), SEGPA, ULIS ; **sans condition d'âge** (C. éduc. D331-6) | 3e : 5 jours, consécutifs ou non, entre la Toussaint et les vacances de printemps | **observation seulement** : aucun travail même léger, aucune machine (D331-9) ; pas de régime accident du travail (accident scolaire, §5.6) ; rapport et oral (21), portfolio Avenir(s) |
| `seconde_gt` | seconde générale et technologique (D333-3-1) | 2 semaines consécutives fin juin (15–26 juin en 2026) | offres et conventions souvent sur **1élève1stage** (import, §16.3) ; **une convention par lieu** si deux lieux ; dispenses (service national, mobilité, 2de STHR) |
| `pfmp` | CAP, bac pro, BMA, CS, FCIL | bac pro à la rentrée 2026 : 4 à 6 semaines en 2de, 6 à 8 en 1re, 6 en terminale (16 à 20) + 2 semaines dédiées en terminale | régime accident du travail ; travaux réglementés possibles dès 15 ans avec dérogation et aptitude ; évaluation ; attestation (modèle national) ; **allocation** ; décompte pour l'examen |
| `immersion` | élève d'un autre établissement ou élève accueilli quelques jours dans un atelier ou une entreprise | 1 à 5 jours | convention entre établissements ; élève extérieur sans dossier complet (§17) |
| `stage_individuel` | dispositifs relais, décrochage, prépa-métiers, élève allophone, aménagement de scolarité | variable | décision de l'équipe ; référent obligatoire |
| `mobilite_internationale` | PFMP ou stage à l'étranger | variable | convention bilingue, engagement de formation, assurance rapatriement, Ariane (§13.6) |
| `autre_encadre` | tout stage prévu par un texte non couvert | variable | règles les plus strictes par défaut |

### 9.2 Périodes collectives

- Le DDFPT ou le chef d'établissement **planifie** les périodes par classe ou groupe (STG-120) ; le 39 crée la période du 5 de nature `entreprise`, publie `pfmp.periode_planifiee` ; le 10 en calcule l'effet (séances retirées pour les élèves en stage, maintenues pour ceux qui restent).
- **Échelonnement** : le 10 simule l'occupation des ateliers et des professeurs (EDT-233) ; le 39 propose les dates.
- Une période a un **état** (`prevue`, `publiee`, `close`) ; publiée, elle est visible des familles et des élèves.
- **Périodes individuelles** : rattrapage, décalage (élève arrivé en cours d'année, maladie), sur décision du DDFPT avec motif.
- **Décompte réglementaire** : pour la PFMP, le total exigé par le diplôme et le cycle (§5) est porté par la formation (5) ; le 39 compare les semaines planifiées puis effectives et alerte (R-SG12).

## 10. Organismes, sites, contacts

### 10.1 Organismes
- Saisie par un personnel (STG-110), ou **proposée** par une famille, un élève ou l'organisme lui-même par lien (STG-111) : une proposition n'entre au carnet qu'après validation (R-SG14).
- **SIRET** : contrôle de clé local ; recherche des doublons (SIRET, puis nom normalisé et code postal) **proposée**, jamais fusionnée sans humain (STG-113). La vérification dans le répertoire Sirene de l'INSEE est **utile** (organisme actif, adresse, code NAF, tranche d'effectif pour le quota de stagiaires) mais c'est une API externe : **non activée par défaut**, soumise à Florian (Q-SG3) ; sans elle, saisie et contrôle manuels.
- **État** : `actif`, `a_verifier`, `ferme`, `deconseille` (motif codé : sécurité, encadrement insuffisant, refus discriminatoire signalé, non-respect des horaires, autre ; texte libre court visible des seuls DDFPT et direction ; **jamais** de motif qui vise une personne nommément), `interdit` (décision de la direction).
- **Historique d'accueil** calculé : nombre d'élèves accueillis par année et par formation, dernière évaluation de l'accueil par le référent (grille courte, §13.5).

### 10.2 Contacts (personnes de l'organisme)
- Nom, prénom, fonction, e-mail, téléphone, rôle(s) ; **date d'information** (les contacts sont informés du traitement de leurs données à la première sollicitation, mention dans le premier envoi, §21.3).
- Un contact n'a **jamais** de compte ; il agit par **lien nominatif** à portée limitée (7 §10.5).
- Changement d'adresse : demandé par le contact ou un tiers, **validé** par l'établissement (NST-07) ; l'ancien lien est révoqué.
- Conservation : supprimé ou anonymisé **3 ans après son dernier stage** (S-3903, durée soumise au DPO), sauf s'il figure sur une convention conservée (il reste alors dans le document figé, pas dans le carnet).

## 11. Recherche, offres, candidatures

### 11.1 Recherche
- Chaque période crée pour chaque élève attendu un stage `recherche` (STG-150).
- **Liste « sans lieu »** par classe, avec l'ancienneté de la recherche, les candidatures en cours et les offres compatibles ; relance des élèves et des titulaires (N3).
- **Aide** : le référent ou le professeur principal propose une offre à un élève (STG-155) ; le DDFPT peut **réserver** une place.
- **Indicateurs d'équité** (agrégés, jamais nominatifs hors de l'équipe) : taux « sans lieu » à J-30 par classe, par dispositif ; temps moyen de recherche ; part des stages trouvés par le réseau de l'établissement (§20, R-SG20).
- **Refus discriminatoire** : l'élève ou sa famille peut signaler un refus discriminatoire (STG-158) ; le signalement va à la direction, qui peut **déconseiller** l'organisme et informer la famille des recours (Défenseur des droits, §5).

### 11.2 Offres
- Une offre : organisme, site, contact, intitulé, description courte, dispositifs et formations visés, places, dates possibles, conditions (âge minimal, permis, horaires particuliers), **état** (`proposee`, `publiee`, `pourvue`, `close`, `refusee`).
- Origines : personnel, famille (NST-01), **organisme par lien** (STG-141), reprise des stages des années précédentes (« accueillerez-vous encore ? », STG-144, N3).
- Publiée, l'offre est visible des élèves des formations visées et de leurs titulaires, **sans** les coordonnées du contact (elles sont données à l'élève qui candidate, ou l'établissement transmet la candidature, S-3904).

### 11.3 Candidatures
- L'élève **connecté** candidate (STG-146) ; un CV ou une lettre peut être joint (objet du 7) ; l'élève sans compte (immersion) passe par le lien de l'offre avec identification (NST-04).
- États : `envoyee`, `vue`, `entretien`, `acceptee`, `refusee`, `retiree`. L'acceptation par l'organisme (lien) ou la saisie par le référent fait passer le stage en `trouve` et pré-remplit la convention.
- Places comptées côté serveur, en transaction (deux candidatures acceptées pour la dernière place : la seconde est refusée, CSG-07).

## 12. La convention

### 12.1 Contenu (données du 39, rendu par le 7)
- **Parties** : établissement (nom, adresse, UAI, chef d'établissement, signataire habilité) ; organisme (raison sociale, SIRET, adresse, représentant signataire) ; élève (nom, prénom, date de naissance, formation, classe) ; titulaires si mineur (calculés du 4).
- **Stage** : dispositif, dates, lieu (site), **horaires jour par jour** (§12.3), activités prévues et compétences visées (lien au référentiel du 22 pour la PFMP), tuteur, enseignant référent, modalités de suivi (visites prévues), modalités d'évaluation.
- **Clauses** : assurance responsabilité civile de l'organisme et de l'établissement (paramètre obligatoire, DN-18), accident du travail (régime des élèves, §5) et circuit de déclaration, gratification éventuelle et avantages (repas, transport), **travaux réglementés** (oui/non ; si oui : déclaration de dérogation de l'organisme, date, et aptitude médicale de l'élève), équipements de protection, conditions de rupture, discipline (l'élève reste soumis au règlement de l'établissement et au règlement de l'organisme).
- **Annexes** : pédagogique (activités, compétences, suivi), financière (gratification, avantages, frais), modèles nationaux quand ils existent (§5) ; ScolIA fournit les **trames** par dispositif (D-023), l'établissement ajoute ses clauses dans un champ limité, relu.

### 12.2 Préparation
- Par l'élève (dans l'ENT), un titulaire, le référent ou le secrétariat ; **pré-remplie** depuis l'élève, l'offre, l'organisme, le dernier stage chez le même organisme.
- L'élève ne saisit **jamais** l'adresse d'un titulaire (DN-07) : les signataires sont calculés.
- Soumission → validation par le référent (contenu pédagogique), puis par le secrétariat ou le DDFPT (conformité) ; chacun peut renvoyer `a_corriger` avec motif.

### 12.3 Horaires et contrôles réglementaires (moteur repris de NST-10, R-SG5)
- Saisie en **semaine type** avec exceptions, ou **jour par jour** ; chaque jour : plages (début, fin), pause.
- Contrôles **bloquants** selon l'âge de l'élève **à chaque jour du stage** (un élève qui a 16 ans le mercredi change de règle le mercredi) et le dispositif : durée quotidienne et hebdomadaire maximales, repos quotidien et hebdomadaire, pause, travail de nuit, dimanche et jours fériés, âge minimal (§5 pour les valeurs et leurs sources ; dérogations sectorielles prévues par les textes déclarées avec leur fondement).
- Contrôles **d'alerte** : horaires hors de la période (NST-11), plus de N stages simultanés chez le même tuteur (quota, §5), trajet incompatible avec les horaires (non calculé par défaut), stage pendant une épreuve (38).
- Les valeurs sont des règles versionnées (`ref_stage_regle`) ; un changement de texte crée une nouvelle version datée ; une convention garde la version de sa signature.

### 12.4 Validation et contrôles avant émission (exigences, R-SG6)
Liste d'exigences calculée par stage, chacune `attendue | remplie | sans_objet | bloquante` : organisme identifié, site, tuteur avec un canal, référent désigné, horaires conformes, assurance renseignée, période valide, **dérogation travaux réglementés** et **aptitude** si travaux réglementés, consentement à la transmission d'un aménagement si aménagement, autorisation de sortie du territoire et Ariane si étranger (§13.6). Une exigence non légale peut être **dérogée** par la direction avec motif ; les exigences légales, jamais.

## 13. Signatures, départ, suivi

### 13.1 Circuit de signature (socle 7, nature `convention_stage`)
- Signataires calculés selon le dispositif [V, §5.3] : **PFMP** — chef d'établissement (ou délégataire habilité, 6), représentant de l'organisme, **tuteur**, **enseignant référent**, élève majeur ou titulaire(s) si mineur, la convention étant ensuite adressée à la famille pour information ; **observation** — chef d'établissement, organisme, titulaires si mineur. ScolIA ajoute l'**élève mineur co-signataire** (7 §10.4, il prend connaissance et s'engage, sans remplacer ses titulaires) [R] ; règle d'accord des titulaires `un` par défaut (R-SG8), `tous` pour la mobilité internationale.
- **Ordre** : parallèle, l'établissement **en dernier** (il signe une convention complète et conforme) [R, S-3905].
- Niveaux : personnels et comptes familles **S2** (passkey) par défaut, S1 accepté pour les familles tant que la passkey n'est pas généralisée (D-009) ; externes **S1** (lien + code par e-mail ou SMS OVH) ; **P** (papier déposé puis contrôlé) pour l'organisme qui l'exige ; **G** au guichet pour une famille sans compte.
- Signature **par lot** pour le chef d'établissement (parapheur trié par date de départ, contrôles affichés, une passkey par lot, chaque convention signée séparément).
- Relances, remplacement d'un signataire, révocation : actions du 7 (DCT-74 et suivantes) appelées par le 39.

### 13.2 Départ
- À la signature complète (`signe`) : situation `pfmp`/`stage` posée au 12 pour les **jours et horaires réels** ; jours publiés au 40 (non-repas cause « stage » les jours de présence) et au 41 ; séances signalées au 10.
- Si la convention n'est pas complète **la veille à 16 h** (S-3906) : alerte de dernière chance ; **le matin du premier jour**, l'élève est **attendu au lycée** ; l'appel le dit (« convention non signée ») ; la famille est prévenue (N3 préparé, envoi selon S-3907).

### 13.3 Pendant le stage
- **Tuteur** : un lien unique pour la durée du stage + 15 jours ; il **signale une absence** ou un retard (le 12 enregistre, AS-240), **signale un problème** (comportement, sécurité, accident → §13.4), répond aux messages, déclare la fin.
- **Élève** : voit ses horaires, l'adresse, le tuteur, le référent ; bouton « **j'ai un problème** » qui joint le référent (et la vie scolaire en dehors des heures) ; peut déclarer une absence comme pour un cours (12).
- **Famille** : voit l'état, les horaires, l'adresse du site (sauf adresse protégée de l'élève… sans objet ici ; l'adresse de l'organisme n'est pas protégée), les absences signalées.
- **Référent** : tableau de ses élèves en stage, absences du jour, visites à faire.

### 13.4 Incidents, accidents, rupture
- **Accident** : signalé par le tuteur (lien), l'élève, la famille ou le référent ; le 39 ouvre un **dossier d'accident** horodaté et prévient la direction immédiatement (R-SG10). Le circuit dépend du dispositif (§5.6) : en **PFMP**, c'est l'**organisme d'accueil** qui déclare à la CPAM sous 48 h (hors dimanches et jours fériés) et en adresse copie au chef d'établissement : ScolIA **demande et suit** cette copie (échéance, relance du tuteur, dépôt par le lien) ; en **observation**, pas de régime accident du travail : c'est le **chef d'établissement** qui établit la déclaration d'accident scolaire sous 48 h, échéance rappelée. La déclaration est un document du 7 (restreint) ; **aucun** détail médical dans le 39 (renvoi au 34).
- **Discriminations et violences sexistes et sexuelles** [V, circulaire du 21-11-2025] : l'organisme informe l'élève de ses procédures de signalement à son arrivée (case de la convention) ; l'élève signale au référent ; ScolIA rend ce signalement **possible en un geste** depuis son téléphone, vers le référent et la direction seulement.
- **Danger, harcèlement, violence** : signalement de l'élève ou d'un tiers → direction et référent ; le 39 peut **suspendre** le stage immédiatement (l'élève est attendu au lycée ou mis à l'abri selon décision), ouvrir une rupture ; les éléments protégés vont au 36 (protection de l'enfance) ; l'organisme peut être **interdit**.
- **Rupture** : à l'initiative de l'organisme, de la famille, de l'élève majeur ou de l'établissement, avec motif codé ; date d'effet ; situation close au 12 ; rattrapage proposé (PFMP).
- **Interruption** (maladie, fermeture temporaire) : jours non effectifs, reprise ou rattrapage.

### 13.5 Visites et suivi par le référent
- Suivi minimal [V, convention-type PFMP art. 18] : **prise de contact au début**, suivi régulier, évaluation, et **au moins une rencontre** référent – élève – tuteur ; ScolIA en fait des exigences du stage (S-3908 permet d'en ajouter, jamais d'en retirer) ; le nombre de stagiaires par référent est fixé par le **conseil d'administration** dans le public (L124-2) : paramètre affiché et contrôlé en alerte ; types `preparation`, `suivi`, `evaluation`, `telephone`, `visio`.
- **Tournée proposée** : regroupement des visites par zone et par jour, sur les plages libérées de l'EDT (EDT-234) ; calcul des distances seulement si un géocodage est disponible (adresse BAN : service public, à valider, Q-SG4) ; sinon regroupement par code postal.
- Rendez-vous avec le tuteur par le 19 (lien) ; **compte rendu** court sur mobile, **hors ligne** (11) : présence constatée, activités, points forts, points à travailler, sécurité constatée (oui/non), suite.
- **Ordre de mission** et frais de déplacement demandés au 6/49 depuis la visite.
- Évaluation de **l'accueil** par le référent (encadrement, sécurité, activités conformes) : alimente l'historique de l'organisme.

### 13.6 Mobilité internationale
- Convention bilingue, engagement de formation, organisme étranger (pas de SIRET : identifiant national étranger), assurance (rapatriement, responsabilité civile à l'étranger), **autorisation de sortie du territoire** des mineurs et **Ariane** (repris du 42), documents de mobilité (Europass) [H] ; financements éventuels (programme européen) hors du 39 (48).

## 14. Évaluation et attestation

- **Évaluation** : grille du dispositif (PFMP : compétences du référentiel du diplôme, lues au 22 ; observation : grille courte de l'établissement) ; le **tuteur propose** son positionnement par lien (proposition au 22, jamais une note), le référent conduit le **bilan** avec le tuteur (visite ou visio), valide ; la note certificative éventuelle est saisie au 21 par le professeur habilité.
- **Attestation** : modèle du dispositif (§5) pré-rempli : dates, jours effectifs, absences, activités ; signée par l'organisme (S1 ou P) ; document du 7 dans le coffre de l'élève ; **le décompte des semaines** pour l'examen est calculé depuis les jours effectifs et les attestations, et **certifié** par le DDFPT avant transmission au 38.
- **Évaluation de l'accueil par l'élève** (L124-4, annexe 4 de la convention-type) : questionnaire court rempli par l'élève à la fin, **sans effet sur son évaluation** ; il alimente l'historique de l'organisme, agrégé, jamais montré nominativement à l'organisme.
- **Rapport de stage** (collège, seconde) : dépôt par l'élève au cahier de textes (20), évaluation au 21 ; oral planifié au 19.

## 15. Argent

### 15.1 Allocation PFMP (État) [V, §5.5]
- **Forfait journalier × jours entiers effectués et attestés** ; barème et plafonds annuels versionnés (`ref_stage_bareme`, par niveau et par année d'entrée dans le cycle : la terminale entrée en 2026 n'a pas le plafond de celle entrée en 2024) ; une PFMP rattrapée l'année suivante est payée **au tarif de l'année d'origine** ; les absences ne sont **jamais** payées, quel qu'en soit le motif ; la semaine du parcours « poursuite d'études » n'est pas payée.
- **Le 39 calcule, justifie et prépare ; il ne paie pas.** Le paiement passe par l'ASP via **ApLyPro** : décision d'attribution annuelle validée par le chef d'établissement, puis **état liquidatif** après chaque PFMP, au plus tard **deux mois** après la fin de la période, strictement conforme à l'attestation. ScolIA produit l'état prêt à reporter (ou à téléverser si ApLyPro l'accepte un jour, Q-SG5) et suit l'échéance ; états `a_calculer → calculee → controlee → reportee_aplypro → versement_constate`.
- **Bénéficiaire** calculé : l'élève majeur ; l'élève mineur sur son propre compte avec **autorisation écrite annuelle** des titulaires (formulaire du 18) ; sinon un titulaire ; cas ASE, tutelle, mineur non accompagné signalés pour traitement humain. Coordonnées bancaires lues au 4 (D-014), **jamais visibles** du référent ni du DDFPT.
- **Pièces justificatives** (identité, filiation, RIB, autorisation, convention, avenants, attestation) rangées au 7 avec une conservation de **10 ans** (§21.3).
- Le privé sous contrat d'association est éligible ; le contrat simple n'est pas mentionné par les textes lus (Q-SG6).
- Mode **suivi** pendant l'année de doublon (D-034) : ScolIA calcule et compare avec ce qui est saisi dans ApLyPro, sans rien transmettre (R-SG25).

### 15.2 Gratification (organisme) [V, §5.5]
- Obligatoire au-delà de **deux mois** de présence effective (plus de 44 jours, 7 h = 1 jour) dans un **même organisme** sur l'année scolaire, PFMP comprises, au minimum 15 % du plafond horaire de la sécurité sociale (valeur à jour en paramètre global, S-3909) ; **compteur** par élève, organisme et année ; alerte à 35 jours ; refus d'émettre une convention qui franchit le seuil sans clause de gratification (R-SG13).
- Plafond de **six mois** par année d'enseignement dans un même organisme (L124-5) : contrôle bloquant.
- Cumulable avec l'allocation. ScolIA ne paie rien.

### 15.3 Restauration, transport, hébergement
- Non-repas cause « stage » (40) ; remise selon le règlement financier (24, S-157 du 24).
- Aides aux frais (transport, hébergement) : lues au 26 ; le 39 fournit les jours effectifs et les distances si disponibles.

## 16. Documents (socle 7)

### 16.1 Natures déclarées au 7
`convention_stage` (versions, circuit), `avenant_stage`, `attestation_stage`, `evaluation_stage` (copie figée signée), `compte_rendu_visite` (interne), `rapport_stage` (élève, via 20), `declaration_accident_stage` (interne, restreint), `engagement_mobilite`, `convention_immersion`, `annexe_pedagogique`, `annexe_financiere`. Série de numérotation `STG` (7 §9, série existante).

### 16.2 Rendu
PDF rendu par le 7 depuis les données du 39 et la trame du dispositif ; **aucune** saisie libre de jetons `$$nom$$` ; l'empreinte de la version signée est la référence.

### 16.3 Import et papier
- Convention de la **plateforme nationale** (seconde) ou d'un autre outil : import du PDF signé (DCT-72), rattachement à l'élève et au stage, champs extraits **proposés** (lecture locale, NST-21) et confirmés par un humain ; la convention importée compte comme signée si toutes les signatures sont constatées (P).
- **Papier** : dépôt par lien nominatif (organisme) ou par un personnel ; contrôle humain obligatoire (DN-13).

---

## 17. Cas limites (catalogue)

### 17.1 Élève et famille
| N° | Cas | Comportement attendu |
|---|---|---|
| CSG-01 | Élève qui devient **majeur** entre la préparation et la signature | circuit recalculé : il signe seul ; liens des titulaires révoqués ; titulaires informés seulement si l'élève l'accepte (2) |
| CSG-02 | Élève qui a **16 ans au milieu du stage** | contrôles horaires appliqués **jour par jour** ; les horaires peuvent légalement changer à la date anniversaire |
| CSG-03 | **Parents séparés**, l'un refuse de signer | règle `un` : la signature du premier suffit ; le refus du second après coup est signalé à l'établissement (7 §10.4) ; mobilité internationale (`tous`) : bloqué |
| CSG-04 | Titulaire sous **interdiction** de contact ou sans droit d'information | pas sollicité, ne voit rien ; l'adresse de l'organisme ne lui est jamais montrée (risque d'enlèvement) |
| CSG-05 | Élève **placé** (ASE), famille d'accueil | signataire = service gardien selon le mandat (18 §7.1) ; famille d'accueil informée selon le lien |
| CSG-06 | Majeur sous **tutelle** ou **curatelle** | signature selon la mesure (7 §10.4, 18) ; allocation au bénéficiaire désigné par la mesure, traitement humain |
| CSG-07 | Deux candidatures acceptées pour la **dernière place** | serveur : la seconde refusée, message clair aux deux |
| CSG-08 | L'élève **change de classe** (de bac pro à CAP) pendant la préparation | dispositif et période revus ; signalé au référent, rien modifié seul |
| CSG-09 | L'élève **quitte l'établissement** pendant son stage | alerte direction et référent ; l'organisme doit être prévenu par un humain (N3) ; convention résiliée à la date de sortie |
| CSG-10 | Élève **sans lieu** à J-1 | attendu au lycée ; liste à la vie scolaire ; famille prévenue ; aucune « absence » posée |
| CSG-11 | Élève **allophone**, famille qui ne lit pas le français | convention et avis dans la langue de la famille quand la trame existe (8) ; signature au guichet (G) |
| CSG-12 | Famille **sans compte ni e-mail** | guichet ou papier ; code par SMS si un mobile vérifié existe |
| CSG-13 | Élève en **situation de handicap**, PPS | aménagements transmis comme consignes consenties (35) ; visite renforcée ; accessibilité du site à vérifier (exigence) |
| CSG-14 | Élève avec **PAI** (allergie, épilepsie) | les titulaires informent le lieu d'accueil ; ScolIA propose le texte de la consigne (34), jamais le diagnostic ; trousse d'urgence en exigence |
| CSG-15 | Élève **enceinte** ou parent | congés prévus par L124-13 ; aucune donnée de santé dans le 39 ; interruption sans motif détaillé |
| CSG-16 | Élève **mineur non accompagné**, titre de séjour | autorisation de travail non vérifiée [H] : exigence « à vérifier » posée par le secrétariat, Q-SG7 |
| CSG-17 | Élève accueilli en **immersion** d'un autre établissement (sans dossier chez nous) | fiche élève extérieur minimale (2) ; convention d'immersion entre établissements ; présence attendue à notre appel du jour (12) |
| CSG-18 | Élève **interne** en stage loin | nuits d'internat publiées au 41 ; hébergement sur place éventuel (annexe financière) ; trajets du lundi |
| CSG-19 | Élève **décrocheur** qui ne va jamais en entreprise | absences au 12 dès le premier jour ; alerte au référent et au CPE ; décision de rupture humaine |
| CSG-20 | Élève qui **ment** sur son stage (convention signée, mais il n'y va pas) | absence signalée par le tuteur ; sans signalement, la visite ou l'appel du référent la révèle ; le tuteur est relancé J+2 s'il n'a rien déclaré (« tout va bien ? » en un clic) |

### 17.2 Organisme, tuteur, convention
| N° | Cas | Comportement attendu |
|---|---|---|
| CSG-21 | Le **tuteur change** pendant le stage | remplacement par l'établissement (DCT-74) ; ancien lien révoqué ; avenant si la convention nomme le tuteur |
| CSG-22 | L'organisme veut **changer les dates ou horaires** après signature | avenant (nouveau document lié) ; signatures du seul avenant ; la convention reste valable pour ce qu'elle dit |
| CSG-23 | L'organisme **ferme** ou fait faillite pendant le stage | interruption ; rattrapage ; jours non effectifs |
| CSG-24 | **Deux sites** pour un même stage, ou deux organismes la même semaine | deux stages (deux conventions, comme en seconde, §5.3) ; chevauchement horaire refusé |
| CSG-25 | Organisme **sans SIRET** (association non immatriculée, organisme étranger, administration) | identifiant alternatif typé (RNA, identifiant étranger, SIREN de l'administration) ; jamais de SIRET inventé |
| CSG-26 | Organisme **interdit** (signalement de l'inspection du travail, L4153-2) | aucune convention émise ; les stages futurs signalés |
| CSG-27 | Organisme qui **refuse la signature électronique** | papier (P) : il télécharge, signe, dépose ; contrôle humain |
| CSG-28 | Le signataire de l'organisme **n'est pas habilité** (stagiaire RH) | ScolIA ne le vérifie pas [H] ; la convention mentionne la qualité déclarée ; contestation traitée humainement |
| CSG-29 | Le **lien est transféré** à un collègue | le code va au canal du contact nommé ; le collègue ne peut pas signer ; il peut demander le changement (validé par l'établissement) |
| CSG-30 | Mauvaise adresse e-mail du tuteur saisie | lien révoqué au changement ; DN-04 impossible ; journal |
| CSG-31 | Organisme qui offre **repas et transport** | avantages en annexe financière ; non-repas au 40 |
| CSG-32 | **Gratification** franchie par un avenant qui prolonge | refus de l'avenant sans clause ; alerte |
| CSG-33 | Stage chez un **parent** ou dans l'entreprise familiale | autorisé sauf règle de l'établissement (S-3913) ; tuteur ≠ titulaire signataire (conflit signalé) |
| CSG-34 | Stage dans **l'établissement lui-même** (service de restauration, atelier) | organisme = l'établissement ; les personnels sont des contacts internes ; pas de lien externe |
| CSG-35 | Tuteur qui **ne répond jamais** | relances J+2, J+7 ; le référent appelle ; attestation au guichet ou papier |

### 17.3 Calendrier, présence, examen
| N° | Cas | Comportement attendu |
|---|---|---|
| CSG-36 | **Jour férié** pendant le stage | non travaillé pour un mineur sauf dérogation sectorielle déclarée ; ni absence ni jour effectif |
| CSG-37 | **Grève des transports**, intempéries | absence du 12 avec motif ; jours non payés (allocation) ; rattrapage possible |
| CSG-38 | Élève **malade** pendant la PFMP | absence justifiée au 12 ; interruption si longue ; validation alternative ou report (art. 20) |
| CSG-39 | Élève qui doit passer une **épreuve** pendant son stage | alerte à la préparation (38) ; journée d'épreuve sortie du stage |
| CSG-40 | Élève **revenu au lycée** un jour (stage annulé ce jour) | l'organisme ou le référent le déclare ; situation suspendue pour ce jour ; l'élève est attendu à l'appel |
| CSG-41 | Il **manque des semaines** de PFMP en mai de terminale | alerte depuis février ; rattrapage proposé ; décision humaine pour l'examen (§5.4) |
| CSG-42 | Élève **redoublant** ou arrivé en cours de cycle | semaines des années précédentes reprises (attestations) ; positionnement réglementaire éventuel (réduction de durée) saisi par le DDFPT avec la décision [H] |
| CSG-43 | Période de PFMP **déplacée** après signature des conventions | avenants en lot proposés ; aucune modification silencieuse |
| CSG-44 | **Changement d'heure**, stage de nuit d'un majeur | horaires en heure locale (Europe/Paris) ; nuit d'un majeur seulement avec autorisation nominative |
| CSG-45 | Stage **à l'étranger** avec décalage horaire | horaires en heure locale du lieu, contrôlés selon le droit français (mineur sous statut scolaire) [H] ; Ariane |

### 17.4 Données, sécurité, argent
| N° | Cas | Comportement attendu |
|---|---|---|
| CSG-46 | Un élève tente de **lire le lien de signature** de son parent (DN-01) | impossible : aucune projection ne contient un jeton ; jetons hachés |
| CSG-47 | Un **parent interdit** tente de savoir où est son enfant | aucune projection ; journal ; alerte si tentative répétée |
| CSG-48 | Une **entreprise malveillante** essaie des liens au hasard | jeton de 256 bits, débit limité partagé, codes à 5 essais ; aucune information sur l'existence d'un jeton |
| CSG-49 | Un ancien tuteur garde son lien après le stage | expiration à fin + 15 jours ; révocation à la clôture |
| CSG-50 | **Fusion** de deux dossiers élèves (2) | stages rattachés au survivant (fusion.rs couvre déjà `ent_convention_stage` : à étendre aux nouvelles tables) |
| CSG-51 | Allocation saisie dans ApLyPro **différente** du calcul de ScolIA | écart signalé au gestionnaire, jamais corrigé seul |
| CSG-52 | RIB de l'élève **changé** après la décision d'attribution | alerte au gestionnaire ; nouvelle autorisation si mineur |
| CSG-53 | Demande d'**effacement** d'un tuteur | supprimé du carnet ; les conventions signées restent (obligation légale, 10 ans si allocation) |
| CSG-54 | **Restauration** de sauvegarde (PRA) | liens externes et envois neutralisés (E9) ; jetons réémis à la demande |
| CSG-55 | Convention signée puis **erreur découverte** (date de naissance) | rectification par avenant ou nouvelle version ; jamais d'édition du document signé |
| CSG-56 | **Accident** un samedi soir dans un restaurant (élève majeur, nuit autorisée) | signalement par le lien ; direction prévenue ; échéance de 48 h calculée hors dimanches et jours fériés ; copie de la déclaration demandée à l'organisme |
| CSG-57 | Signalement de **violences sexuelles** par l'élève | référent et direction seulement ; suspension immédiate possible ; renvoi au 36 ; aucune trace détaillée dans le 39 (seulement « signalement transmis le … ») |

---

## 18. Intelligence artificielle (contrat IA du socle, `IA_COEUR.md` §5, §6.2)

### 18.1 Ce que l'IA fait [R]
- **Pré-remplir** une convention depuis une offre, un ancien stage ou un document déposé (lecture locale, NST-21) : propositions champ par champ, sources affichées (N1).
- **Expliquer** un blocage réglementaire en langage simple (« Léa a 15 ans : 20 h n'est pas possible, la nuit commence à 20 h pour elle ») (N0).
- **Proposer** une tournée de visites (regroupement par zone et par jour, plages libérées) que le référent ajuste (N1).
- **Rédiger** un brouillon de relance, d'avis, de message au tuteur (N3 à l'envoi : jamais envoyé par l'IA).
- **Résumer** l'état d'une classe (« 4 sans lieu, 3 conventions en attente du chef d'établissement, 1 sans aptitude ») (N0).
- **Proposer** une appréciation de stage depuis le compte rendu de visite et le positionnement du tuteur, relue une à une par le référent (N1, comme D-032 pour les bulletins).
- **Rapprocher** une convention papier ou importée d'un élève et d'un organisme (proposition, confirmée par un humain).
- **Signaler** les anomalies : horaires identiques pour toute une classe chez des organismes différents (copier-coller), tuteur qui encadre trop d'élèves, attestation qui contredit les absences.

### 18.2 Ce que l'IA ne fait pas [R]
- Ne **signe**, n'**émet**, n'**envoie**, ne **valide** rien ; ne pose ni ne lève aucune situation au 12.
- Ne **note** pas un élève, ne **classe** pas un organisme, ne **déconseille** personne.
- Ne lit **aucune** donnée de santé, d'aménagement, de protection de l'enfance, ni le motif d'une rupture pour violence.
- Ne contourne **aucune** règle réglementaire et ne propose jamais de dérogation à une règle légale.
- N'appelle **aucune API externe** avec les pièces des élèves (DN-08) ; lecture des PDF par un moteur local.

### 18.3 Capacités déclarées (registre `ent-ia`)
| Capacité | Droit | Portée | Sensibilité | Niveau |
|---|---|---|---|---|
| `stages.etat_classe` | `stages.lire` | classes de la personne | interne | N0 |
| `stages.expliquer_controle` | `stages.preparer` | un stage | interne | N0 |
| `stages.preremplir_convention` | `stages.preparer` | un stage | personnelle | N1 |
| `stages.proposer_tournee` | `stages.suivre` | ses élèves | interne | N1 |
| `stages.brouillon_relance` | `stages.preparer` | destinataires calculés | personnelle | N3 à l'envoi |
| `stages.proposer_appreciation` | `stages.evaluer` | un stage | personnelle | N1 |
| `stages.rapprocher_document` | `stages.preparer` | un document | personnelle | N2 |

### 18.4 Cadre
Tests de garde (équivalence écran / IA, isolation entre établissements, simulation sans écriture), jeu d'évaluation du socle (20 conventions types, 10 cas d'horaires illégaux, 5 documents papier dégradés), étude juridique du règlement européen sur l'IA avant toute fonction à modèle de langage (comme D-032).

---

## 19. Actions métier (STG-100 à STG-319)

Colonnes : acteur et droit (D-017, catalogue au §21.1) ; règles ; événement (outbox, §22) ; lot (§27). Acteurs : **DIR** chef d'établissement ou adjoint, **DDF** DDFPT ou responsable des stages, **REF** enseignant référent, **PP** professeur principal, **SEC** secrétariat ou gestionnaire des stages, **GES** gestionnaire ou comptable, **CPE**, **ACC** accueil, **INF** infirmière ou médecin, **ELV** élève (mineur), **ELM** élève majeur, **TIT** titulaire, **TUT** tuteur (lien), **ORG** signataire ou contact de l'organisme (lien), **SYS** système. **(NST-xx)** : action reprise de Next.js ; « nouveau » sinon.

### 19.1 Paramétrage
| N° | Action | Acteur · droit | Règles | Événement | Lot |
|---|---|---|---|---|---|
| STG-100 | Activer les dispositifs de l'établissement (nouveau) | DIR · `stages.parametrer` | modèles fournis ; règles légales non modifiables | `stages.parametres_modifies` | ST1 |
| STG-101 | Durcir une règle horaire de l'établissement (NST-26) | DIR, DDF · `stages.parametrer` | jamais sous le minimum légal (R-SG5) | idem | ST1 |
| STG-102 | Renseigner les assurances de l'établissement (assureur, n° de contrat) (nouveau, DN-18) | DIR, GES | obligatoire avant la première convention | idem | ST1 |
| STG-103 | Renseigner la délibération du CA approuvant la convention-type (public) ou la décision équivalente (privé) (nouveau) | DIR | date, référence ; affichée dans la convention | idem | ST1 |
| STG-104 | Renseigner la déclaration de dérogation « travaux réglementés » de l'établissement (date, formations, lieux) (nouveau) | DIR, DDF | R4153-41 ; échéance de 3 ans suivie | `stages.derogation_etablissement` | ST2 |
| STG-105 | Fixer le nombre maximal de stagiaires par référent (CA, L124-2) et par organisme (académie, L124-8) (nouveau) | DIR | alertes | idem | ST2 |
| STG-106 | Choisir les clauses propres de l'établissement (champ limité, relu) (nouveau) | DIR | D-023 | idem | ST1 |
| STG-107 | Désigner les délégataires de signature des conventions (via le 6) (nouveau) | DIR | habilitation du 6 | `personnel.habilitation` (6) | ST1 |
| STG-108 | Consulter le barème de l'allocation et les plafonds en vigueur (nouveau) | GES, DDF | référence globale versionnée | — | ST6 |
| STG-109 | Choisir les délais de relance (J-60, J-30, J-7…) dans des bornes (nouveau) | DDF | S-3901, S-3906 | idem | ST1 |

### 19.2 Organismes et contacts
| N° | Action | Acteur · droit | Règles | Événement | Lot |
|---|---|---|---|---|---|
| STG-110 | Créer un organisme (raison sociale, identifiant typé, adresse, NAF, effectif) (existant Rust, élargi) | SEC, DDF, REF · `stages.organismes.gerer` | clé SIRET ; doublons proposés | `stages.organisme_cree` | ST0/ST1 |
| STG-111 | Proposer un organisme (famille, élève, organisme par lien) (nouveau) | TIT, ELV, ORG | proposition à valider (R-SG14) | `stages.organisme_propose` | ST1 |
| STG-112 | Valider ou refuser une proposition d'organisme (nouveau) | SEC, DDF | — | `stages.organisme_cree` | ST1 |
| STG-113 | Rapprocher deux organismes en double (nouveau) | DDF | humain ; historique conservé | `stages.organismes_fusionnes` | ST1 |
| STG-114 | Ajouter un site (lieu d'accueil) (nouveau) | SEC, REF | adresse structurée | — | ST1 |
| STG-115 | Ajouter ou modifier un contact (tuteur, signataire, RH) (NST-08) | SEC, REF | canal requis pour signer ; date d'information | `stages.contact_modifie` | ST1 |
| STG-116 | Valider un changement d'adresse d'un contact demandé par lien (NST-07) | SEC, DDF | ancien lien révoqué | idem | ST1 |
| STG-117 | Changer l'état d'un organisme (à vérifier, fermé, déconseillé avec motif codé, interdit) (nouveau) | DDF (déconseillé), DIR (interdit) | motif codé, jamais une personne nommée ; journal | `stages.organisme_etat` | ST1 |
| STG-118 | Voir l'historique d'accueil d'un organisme (élèves par année, évaluations d'accueil agrégées) (nouveau) | DDF, REF, PP | agrégé ; pas de noms d'élèves hors périmètre | — | ST5 |
| STG-119 | Vérifier un organisme dans le répertoire Sirene (nouveau, option) | SEC | **désactivé par défaut** (Q-SG3) | — | ST11 |

### 19.3 Périodes
| N° | Action | Acteur · droit | Règles | Événement | Lot |
|---|---|---|---|---|---|
| STG-120 | Planifier une période collective pour une classe, un groupe ou une liste (NST-25) | DDF, DIR · `stages.periodes.gerer` | dans l'année du 5 ; dispositif | `pfmp.periode_planifiee` | ST1 |
| STG-121 | Publier une période aux familles et aux élèves (nouveau) | DDF | N3 (avis) | `stages.periode_publiee` | ST1 |
| STG-122 | Modifier ou déplacer une période ; avenants proposés en lot (nouveau, CSG-43) | DDF | aucune convention signée modifiée sans avenant | `pfmp.periode_modifiee` | ST1 |
| STG-123 | Créer une période individuelle (rattrapage, décalage) avec motif (nouveau) | DDF | — | `pfmp.periode_planifiee` | ST1 |
| STG-124 | Fermer une période (nouveau) | DDF | stages non clos listés | `stages.periode_close` | ST1 |
| STG-125 | Voir le plan des semaines dues et planifiées par classe et par cycle (nouveau) | DDF, DIR | depuis la formation du 5 | — | ST5 |
| STG-126 | Simuler l'échelonnement des périodes (ateliers, professeurs) (nouveau) | DDF | port du 10 (EDT-233) | — | ST7 |
| STG-127 | Dupliquer le plan de l'année précédente (nouveau) | DDF | dates recalées sur le calendrier | — | ST1 |
| STG-128 | Planifier les deux semaines dédiées de terminale (immersion ou poursuite d'études) (nouveau) | DDF | arrêté du 22-04-2026 ; la semaine « poursuite d'études » n'ouvre pas l'allocation | `pfmp.periode_planifiee` | ST5 |
| STG-129 | Afficher les séances touchées par une période (nouveau) | DDF, professeurs | lecture du 10 | — | ST7 |

### 19.4 Référents et projections
| N° | Action | Acteur · droit | Règles | Événement | Lot |
|---|---|---|---|---|---|
| STG-130 | Désigner l'enseignant référent d'un élève ou d'une classe (NST-27) | DDF, PP · `stages.referents.gerer` | mission du 6 (type propriété 39) | `mission.attribuee` (6) | ST1 |
| STG-131 | Déléguer la désignation (le PP la confie à un collègue) (NST-27) | PP | — | idem | ST1 |
| STG-132 | Affecter en lot les élèves aux référents, équilibrage proposé (NST-27) | DDF | quota du CA en alerte | idem | ST1 |
| STG-133 | Prévenir les référents de leurs affectations (NST-27) | DDF | N3 | `notifications.avis_cree` | ST1 |
| STG-134 | Donner à la vie scolaire et à la restauration la projection « en stage » (remplace les « watchers », NST-28) | SYS | par droit, sans affectation manuelle | — | ST1 |
| STG-135 | Remplacer un référent absent (nouveau) | DDF | lien avec le 15 (absence longue) | `mission.attribuee` | ST3 |
| STG-136 | Voir ses élèves suivis, tous dispositifs (nouveau) | REF | — | — | ST1 |
| STG-137 | Voir la charge des référents (nombre d'élèves, visites) (nouveau) | DDF | — | — | ST4 |

### 19.5 Offres et candidatures
| N° | Action | Acteur · droit | Règles | Événement | Lot |
|---|---|---|---|---|---|
| STG-140 | Déposer une offre (personnel, famille) (NST-01) | SEC, REF, TIT · `stages.offres.deposer` | proposée, à valider | `stages.offre_proposee` | ST1 |
| STG-141 | Un organisme dépose une offre par lien (nouveau) | ORG (lien de dépôt) | identification par code ; à valider | idem | ST1 |
| STG-142 | Valider, refuser ou retirer une offre (NST-02) | DDF, SEC · `stages.offres.valider` | motif du refus | `stages.offre_publiee` | ST1 |
| STG-143 | Consulter les offres de sa formation (nouveau ; NST-03 par lien) | ELV, ELM, TIT | coordonnées du contact masquées (S-3904) | — | ST1 |
| STG-144 | Relancer les organismes des années précédentes (« accueillerez-vous encore ? ») (nouveau) | DDF | N3 ; organismes actifs non déconseillés | `stages.offre_sollicitee` | ST11 |
| STG-145 | Réserver une place d'une offre pour un élève (nouveau) | DDF, REF | — | `stages.place_reservee` | ST1 |
| STG-146 | Candidater à une offre (NST-03) | ELV, ELM | places comptées en transaction (CSG-07) | `stages.candidature_envoyee` | ST1 |
| STG-147 | Suivre ses candidatures (envoyée, vue, entretien, acceptée, refusée) (nouveau) | ELV, TIT, REF | — | — | ST1 |
| STG-148 | Accepter ou refuser une candidature (organisme par lien, ou référent) (NST-03) | ORG, REF | l'acceptation crée le stage `trouve` pré-rempli | `stages.candidature_acceptee` | ST1 |
| STG-149 | Clore une offre pourvue (NST-02) | SYS | à zéro place | `stages.offre_pourvue` | ST1 |

### 19.6 Recherche et tableaux de bord
| N° | Action | Acteur · droit | Règles | Événement | Lot |
|---|---|---|---|---|---|
| STG-150 | Créer les stages `recherche` des élèves attendus d'une période (nouveau) | SYS | scolarisés à la date | `stages.recherche_ouverte` | ST1 |
| STG-151 | Tableau par classe : qui a trouvé, où en est chaque convention (NST-32) | PP, REF, DDF | projection par rôle | — | ST1 |
| STG-152 | Tableau du DDFPT et de la direction : compteurs, conventions à signer, alertes (NST-33) | DDF, DIR | — | — | ST1 |
| STG-153 | Parapheur des conventions à signer, trié par date de départ (NST-33) | DIR, délégataire | — | — | ST1 |
| STG-154 | Liste « sans lieu » à J-30 avec relance (nouveau) | PP, REF, DDF | N3 | `notifications.avis_cree` | ST1 |
| STG-155 | Proposer une offre à un élève (nouveau) | REF, PP | — | `stages.offre_proposee_eleve` | ST1 |
| STG-156 | Déclarer un lieu trouvé hors offre (élève ou famille) (nouveau) | ELV, TIT | organisme proposé si inconnu | `stages.lieu_declare` | ST1 |
| STG-157 | Noter une démarche de recherche (entreprise contactée, réponse) (nouveau) | ELV, REF | journal de recherche de l'élève, privé à l'élève, ses titulaires et l'équipe | — | ST11 |
| STG-158 | Signaler un refus discriminatoire (nouveau) | ELV, TIT, REF | direction ; recours indiqués | `stages.signalement_discrimination` | ST3 |
| STG-159 | Accueillir l'élève sans lieu dans l'établissement (emploi du temps de remplacement) (nouveau) | DDF, CPE | §5.4 ; attendu en cours ou en permanence | — | ST1 |

### 19.7 Préparation de la convention et horaires
| N° | Action | Acteur · droit | Règles | Événement | Lot |
|---|---|---|---|---|---|
| STG-160 | L'élève prépare sa convention dans l'ENT (NST-04) | ELV, ELM · soi | pré-remplie ; jamais d'adresse de titulaire saisie | `stages.convention_preparee` | ST1 |
| STG-161 | Voir ses stages et les périodes ouvertes (NST-05) | ELV, TIT | — | — | ST1 |
| STG-162 | Créer le brouillon lié à une période (NST-05) | ELV, REF, SEC | une période valide | idem | ST1 |
| STG-163 | Enregistrer, soumettre (NST-06) | ELV, REF, SEC | verrou optimiste | `stages.convention_soumise` | ST1 |
| STG-164 | Contrôler les champs et les exigences à la soumission (NST-12) | SYS | serveur ; exigences du §12.4 | — | ST1 |
| STG-165 | Préparer la convention pour l'élève sans compte (lien d'identification, NST-04) | ELV sans compte | code aux adresses **vérifiées** du 4, masquées ; jamais à la direction (NST-36) | idem | ST1 |
| STG-166 | Pré-remplir depuis un stage précédent chez le même organisme (nouveau) | REF, SEC | — | — | ST1 |
| STG-167 | Choisir le tuteur et le signataire parmi les contacts, ou les créer (NST-08) | REF, SEC, ELV | canal requis | — | ST1 |
| STG-168 | Décrire les activités et les compétences visées (référentiel du 22) (nouveau) | REF | PFMP : obligatoire | — | ST5 |
| STG-169 | Renseigner les clauses : assurance de l'organisme, gratification, avantages, frais (nouveau) | SEC, ORG (lien) | annexe financière | — | ST1 |
| STG-170 | Créer une convention sur la plateforme par l'administration (NST-23) | SEC, DDF · `stages.preparer` | — | `stages.convention_preparee` | ST1 |
| STG-171 | Déclarer des travaux réglementés et la date de la dérogation de l'organisme (nouveau) | ORG, SEC | dès 15 ans ; exige l'aptitude (34) | — | ST2 |
| STG-172 | Demander l'avis d'aptitude au médecin ou à l'infirmière (nouveau) | DDF | liste des avis à rendre ; réponse oui/non/à revoir | `stages.aptitude_demandee` | ST2 |
| STG-173 | Rendre l'avis d'aptitude (oui, non, à revoir) sans motif médical dans le 39 (nouveau) | INF · `sante.aptitude` (34) | annuel | `sante.aptitude_rendue` (34) | ST2 |
| STG-174 | Joindre la consigne d'aménagement consentie (handicap, PAI) (nouveau) | REF ; consentement TIT/ELM | port 34/35 ; consentement 18 | — | ST2 |
| STG-175 | Indiquer les modalités de suivi et d'évaluation (sur site, à distance) (nouveau) | REF | §13.5 | — | ST1 |
| STG-176 | Demander un changement d'e-mail du tuteur (NST-07) | ELV, ORG, TUT | validé par STG-116 | `stages.contact_changement_demande` | ST1 |
| STG-177 | Informer les contacts du traitement de leurs données (nouveau) | SYS | mention au premier envoi | — | ST1 |
| STG-178 | Mettre le stage en `trouve` sans convention (lieu connu) (nouveau) | REF, SEC | rien sur l'appel | `stages.lieu_declare` | ST1 |
| STG-179 | Voir les exigences remplies et manquantes d'un stage (nouveau) | REF, SEC, DDF, ELV, TIT | projection | — | ST1 |
| STG-180 | Saisir les horaires en semaine type (NST-09) | ELV, REF, SEC, ORG | plages et pause | — | ST1 |
| STG-181 | Saisir les horaires jour par jour, exceptions (NST-09) | idem | — | — | ST1 |
| STG-182 | Contrôler la durée quotidienne et hebdomadaire selon l'âge à chaque jour (NST-10) | SYS | §5.2 ; blocage | — | ST1 |
| STG-183 | Contrôler repos quotidien, repos hebdomadaire, pause (nouveau) | SYS | §5.2 | — | ST1 |
| STG-184 | Contrôler la nuit, le dimanche, les jours fériés (NST-10) | SYS | dérogation sectorielle déclarée | — | ST1 |
| STG-185 | Contrôler l'âge minimal du dispositif (NST-10) | SYS | observation sans âge ; initiation 14 ans ; travaux réglementés 15 ans | — | ST1 |
| STG-186 | Contrôler les règles plus strictes de l'établissement (NST-26) | SYS | S-3912 | — | ST1 |
| STG-187 | Alerter des horaires hors de la période et proposer la correction (NST-11) | SYS | — | — | ST1 |
| STG-188 | Autoriser nommément le travail de nuit d'un élève majeur (nouveau) | DIR | art. 9 de la convention-type | `stages.nuit_autorisee` | ST2 |
| STG-189 | Saisir une dérogation de durée accordée par l'inspecteur du travail (référence, avis médical) (nouveau) | SEC | L3162-1 ; pièce | — | ST2 |

### 19.8 Validation, émission, signatures
| N° | Action | Acteur · droit | Règles | Événement | Lot |
|---|---|---|---|---|---|
| STG-190 | Valider le contenu pédagogique (référent) (NST-13) | REF · `stages.valider` (ses élèves) | état `a_valider` | `stages.convention_validee_ref` | ST1 |
| STG-191 | Valider la conformité ou renvoyer à corriger avec motif (NST-13) | SEC, DDF · `stages.valider` | exigences bloquantes levées | `stages.convention_validee` / `_a_corriger` | ST1 |
| STG-192 | Valider en lot les conventions d'une classe (nouveau) | DDF | une par une côté serveur | idem | ST1 |
| STG-193 | Déroger à une exigence non légale avec motif (nouveau) | DIR | jamais une exigence légale | `stages.derogation` | ST2 |
| STG-200 | Émettre la convention : document du 7, circuit calculé, liens externes, avis (NST-13) | SEC, DDF · `stages.emettre` | N3 : aperçu des signataires et des canaux | `stages.convention_emise` | ST1 |
| STG-201 | Appliquer la règle « un titulaire suffit, l'autre peut signer » (NST-14) | SYS | 7 §10.4 | — | ST1 |
| STG-202 | Signer en ligne par code (externes, familles sans passkey) (NST-15) | ORG, TUT, TIT, ELM | S1 ; 5 essais ; comparaison constante | `documents.signature` (7) | ST1 |
| STG-203 | Signer par passkey (familles, élève, personnels) (nouveau ; remplace l'image NST-16) | TIT, ELV, REF, DIR | S2 | idem | ST1 |
| STG-204 | Déposer la convention signée sur papier (organisme) (NST-15) | ORG (lien), SEC | P ; contrôle humain | `documents.papier_depose` (7) | ST1 |
| STG-205 | Signer au guichet (famille sans compte) (nouveau) | SEC (agent) | G | idem | ST1 |
| STG-206 | Signer en lot (chef d'établissement), contrôles affichés (nouveau) | DIR · `stages.signer_etablissement` | une passkey, chaque convention signée séparément | idem | ST1 |
| STG-207 | Valider ou refuser un papier déposé (NST-17) | SEC | « signature présente, pages complètes, version identique » | `documents.papier_valide` | ST1 |
| STG-208 | Révoquer une signature, régénérer le document (NST-17) | SEC, DDF | motif ; circuit rouvert | `documents.signature_revoquee` | ST1 |
| STG-209 | Ajouter ou retirer un signataire (NST-17) | SEC | jamais retirer un signataire obligatoire | — | ST1 |
| STG-210 | Remplacer un signataire (tuteur qui change, parent qui n'est plus titulaire) (NST-17) | SEC | DCT-74 | — | ST1 |
| STG-211 | Relancer un signataire, tous, ou en masse (NST-17) | SEC, DDF | N3 ; dédoublonné | `notifications.avis_cree` | ST1 |
| STG-212 | Constater une signature impossible (décès, injoignable) (nouveau) | DIR | DCT-80 | — | ST3 |
| STG-213 | Classer la convention signée dans le dossier de l'élève et le coffre de la famille (NST-18) | SYS | 7 | `documents.archive` | ST1 |
| STG-214 | Adresser la convention signée à la famille pour information (nouveau, art. 3) | SYS | titulaires ayant droit d'information | `notifications.avis_cree` | ST1 |
| STG-215 | Passer le stage `signe` à la dernière signature exigée (nouveau) | SYS | R-SG1 | `stages.convention_signee` | ST1 |
| STG-216 | Alerter à J-7, J-3, J-1 si incomplète (nouveau) | SYS | S-3906 | `stages.depart_compromis` | ST1 |
| STG-217 | Voir l'état du circuit : qui a signé, qui manque, quand relancé (NST-33) | SEC, REF, ELV, TIT (sa part) | projection : jamais de jeton | — | ST1 |
| STG-218 | Voir et télécharger sa copie signée (signataire externe) (nouveau) | ORG, TUT | lien | — | ST1 |
| STG-219 | Vérifier l'authenticité d'une convention (référence et QR) (nouveau) | tout porteur | page de vérification du 7 | — | ST1 |

### 19.9 Avenants, annulation, échanges
| N° | Action | Acteur · droit | Règles | Événement | Lot |
|---|---|---|---|---|---|
| STG-220 | Demander un avenant (dates, horaires, tuteur, lieu) depuis le lien ou l'ENT (NST-19) | ORG, TUT, ELV, TIT, REF | circuit en attente suspendu (NST-19) | `stages.avenant_demande` | ST1 |
| STG-221 | Proposer un avenant (établissement) (NST-19) | SEC, DDF | contrôles réglementaires refaits | idem | ST1 |
| STG-222 | Accepter ou refuser une demande d'avenant (NST-19) | SEC, DDF | motif | `stages.avenant_decide` | ST1 |
| STG-223 | Émettre l'avenant, signatures de l'avenant seul (avant départ : nouvelle version de la convention, toutes les signatures remises à zéro, NST-19) | SEC | 7 D28 | `stages.avenant_emis` | ST1 |
| STG-224 | Appliquer l'avenant signé (dates, horaires) aux situations, repas, nuits (nouveau) | SYS | §6.4 | `stages.modifie` | ST1 |
| STG-225 | Avenants en lot après déplacement d'une période (nouveau) | DDF | CSG-43 | idem | ST1 |
| STG-226 | Annuler un stage avant le départ (NST-24) | SEC, DDF, DIR | confirmation ; liens révoqués ; page « annulée » | `stages.annule` | ST1 |
| STG-227 | Rétablir un stage annulé par erreur (nouveau) | DIR | dans les 7 jours ; journal | `stages.retabli` | ST1 |
| STG-230 | Échanger dans le fil de la convention (personnels, organisme, tuteur ; élève et titulaires sur un fil séparé) (NST-20) | parties | 16 ; débit limité | `messagerie.message` | ST1 |
| STG-231 | Joindre une pièce au fil (nouveau) | parties | 7, contrôle du type | — | ST1 |
| STG-232 | Message groupé aux tuteurs d'une période (nouveau) | DDF | N3 | `notifications.avis_cree` | ST3 |

### 19.10 Papier, import, lecture automatique
| N° | Action | Acteur · droit | Règles | Événement | Lot |
|---|---|---|---|---|---|
| STG-240 | Déposer une convention papier signée (lien nominatif ou personnel) (NST-21, sans anonymat) | ORG, SEC | 15 Mo ; type réel contrôlé | `documents.depose` | ST1 |
| STG-241 | Lire le document localement et proposer champs, élève, organisme (NST-21) | SYS (IA locale) | proposition | — | ST10 |
| STG-242 | Confirmer le rapprochement et les champs (NST-21) | SEC | humain | `stages.convention_importee` | ST1 |
| STG-243 | Approuver une convention déposée, lancer les seules signatures internes manquantes (NST-21) | SEC | — | `stages.convention_emise` | ST1 |
| STG-244 | Enregistrer une convention signée hors plateforme (NST-22) | SEC | P ; pièce obligatoire | `stages.convention_importee` | ST1 |
| STG-245 | Importer les conventions de 1élève1stage (PDF), contrôler toutes les signatures (nouveau) | SEC | R-SG18 | idem | ST8 |
| STG-246 | Importer en lot les conventions d'une classe (dossier de PDF) (nouveau) | SEC | rapprochement proposé | idem | ST8 |

### 19.11 Départ et pendant le stage
| N° | Action | Acteur · droit | Règles | Événement | Lot |
|---|---|---|---|---|---|
| STG-250 | Poser la situation `pfmp`/`stage` au 12 pour les jours et horaires réels (NST-29 corrigé) | SYS | **après signature complète seulement** (R-SG1) | `stage.periode_ouverte` (nom historique) | ST1 |
| STG-251 | Signaler au 10 les séances concernées (nouveau) | SYS | §6.1 | `pfmp.periode_planifiee` | ST1 |
| STG-252 | Publier les jours de stage au 40 (non-repas cause « stage ») (NST-30) | SYS | jours de présence réels | idem | ST1 |
| STG-253 | Publier les jours de stage au 41 (internat) (nouveau) | SYS | — | idem | ST1 |
| STG-254 | Activer le lien du tuteur pour la durée du stage + 15 jours (nouveau) | SYS | E9 | `stages.lien_tuteur_actif` | ST1 |
| STG-255 | Le tuteur signale une absence ou un retard (nouveau) | TUT (lien) | commande du 12 (AS-240) ; référent et famille prévenus | `assiduite.absence_pfmp` (12) | ST3 |
| STG-256 | Le tuteur confirme « tout va bien » à J+2 (nouveau) | TUT | CSG-20 | `stages.presence_confirmee` | ST3 |
| STG-257 | Le référent saisit une absence en entreprise (AS-118) | REF | commande du 12 | idem | ST1 |
| STG-258 | L'élève voit horaires, adresse, contacts sur son téléphone, hors ligne (nouveau) | ELV | 11 | — | ST4 |
| STG-259 | L'élève alerte « j'ai un problème » (nouveau) | ELV | référent puis vie scolaire hors heures | `stages.alerte_eleve` | ST3 |
| STG-260 | Signaler un incident (comportement, sécurité) (nouveau) | TUT, ELV, REF | — | `stages.incident_signale` | ST3 |
| STG-261 | Ouvrir un dossier d'accident, calculer l'échéance, demander la copie de déclaration à l'organisme (PFMP) ou préparer la déclaration d'accident scolaire (observation) (nouveau) | DIR, SEC · `stages.incidents.gerer` | §5.6 ; aucune donnée médicale | `stages.accident_signale` | ST3 |
| STG-262 | Suspendre un stage immédiatement (danger) (nouveau) | DIR, DDF | situation suspendue ; élève attendu ou mis à l'abri | `stages.suspendu` | ST3 |
| STG-263 | Interrompre et reprendre un stage (maladie, fermeture) (nouveau) | DDF | jours non effectifs | `stages.interrompu` / `_repris` | ST3 |
| STG-264 | Rompre une convention avec motif codé, date d'effet, rattrapage proposé (nouveau) | DIR | art. 19 ; N3 aux parties | `stages.rompu` | ST3 |
| STG-265 | Signaler une discrimination ou des violences sexistes et sexuelles (nouveau) | ELV | référent et direction seulement ; renvoi au 36 | `stages.signalement_protege` | ST3 |
| STG-266 | Déclarer un jour « revenu au lycée » (nouveau, CSG-40) | REF, ORG | situation suspendue ce jour | `stages.jour_au_lycee` | ST3 |
| STG-267 | Voir à l'accueil « en stage chez X jusqu'au …, référent » (nouveau) | ACC, CPE | projection minimale | — | ST1 |
| STG-268 | Alerte « ni en entreprise ni au lycée » (absence signalée par le tuteur + absent à l'appel) (nouveau) | SYS | CPE et référent | `stages.eleve_introuvable` | ST3 |
| STG-269 | Interdire un organisme après un incident grave (nouveau) | DIR | stages futurs signalés | `stages.organisme_etat` | ST3 |

### 19.12 Visites et suivi
| N° | Action | Acteur · droit | Règles | Événement | Lot |
|---|---|---|---|---|---|
| STG-270 | Planifier les visites et contacts exigés (début, rencontre) (nouveau) | REF · `stages.suivre` | §13.5 ; plages libérées (10) | `stages.visite_planifiee` | ST4 |
| STG-271 | Recevoir une tournée proposée (nouveau) | REF | regroupement par zone | — | ST4 |
| STG-272 | Prendre rendez-vous avec le tuteur par lien (nouveau) | REF | port du 19 | `agenda.rdv` (19) | ST4 |
| STG-273 | Saisir le compte rendu de visite, hors ligne (nouveau) | REF | mobile (11) ; texte court | `stages.visite_faite` | ST4 |
| STG-274 | Demander l'ordre de mission et les frais (nouveau) | REF | 6/49 | `personnel.ordre_mission_demande` | ST4 |
| STG-275 | Évaluer l'accueil de l'organisme (référent) (nouveau) | REF | grille courte | `stages.accueil_evalue` | ST4 |
| STG-276 | Voir les visites en retard (DDFPT) (nouveau) | DDF | — | — | ST4 |
| STG-277 | Remplacer une visite par un échange à distance, motivé (nouveau) | REF | autorisé selon la convention | — | ST4 |
| STG-278 | Ajouter une visite supplémentaire (difficulté) (nouveau) | REF, DDF | — | idem | ST4 |
| STG-279 | Consulter l'historique de suivi d'un élève sur le cycle (nouveau) | REF, DDF, PP | — | — | ST4 |

### 19.13 Évaluation, attestation, examen
| N° | Action | Acteur · droit | Règles | Événement | Lot |
|---|---|---|---|---|---|
| STG-280 | Envoyer au tuteur la grille d'évaluation (nouveau) | SYS, REF | dernier jour ou date choisie | `stages.evaluation_demandee` | ST5 |
| STG-281 | Le tuteur positionne l'élève par lien (proposition au 22) (nouveau) | TUT | jamais une note | `competences.proposition` (22) | ST5 |
| STG-282 | Conduire et valider le bilan avec le tuteur (nouveau) | REF · `stages.evaluer` | — | `stages.evalue` | ST5 |
| STG-283 | Transmettre la proposition de note certificative au 21 (nouveau) | REF | professeur habilité | `evaluations.proposition` (21) | ST5 |
| STG-284 | Pré-remplir l'attestation (jours effectifs, absences, activités, gratification) (nouveau) | SYS | D124-9, modèle national | — | ST5 |
| STG-285 | Faire signer l'attestation par l'organisme (S1 ou papier) (nouveau) | ORG | — | `stages.attestation_signee` | ST5 |
| STG-286 | Contester une attestation (jours) (nouveau) | ELV, TIT, REF | échange avec l'organisme ; nouvelle version | — | ST5 |
| STG-287 | Certifier le décompte des semaines d'un élève pour l'examen (nouveau) | DDF | transmis au 38 | `stages.decompte_certifie` | ST5 |
| STG-288 | Alerter des semaines manquantes, proposer un rattrapage (nouveau) | SYS | R-SG12 | `stages.semaines_manquantes` | ST5 |
| STG-289 | Remplir l'évaluation de l'accueil (élève, L124-4) et le rapport ou l'oral (collège, seconde) (nouveau) | ELV | sans effet sur l'évaluation ; rapport via 20, oral via 19 | `stages.accueil_evalue_eleve` | ST5 |

### 19.14 Argent
| N° | Action | Acteur · droit | Règles | Événement | Lot |
|---|---|---|---|---|---|
| STG-290 | Calculer l'allocation par élève et par période (nouveau) | SYS | barème versionné ; jours entiers effectués et attestés | `stages.allocation_calculee` | ST6 |
| STG-291 | Recueillir l'autorisation annuelle des titulaires pour un versement au mineur (nouveau) | TIT | formulaire du 18 | `formulaires.reponse` (18) | ST6 |
| STG-292 | Contrôler l'état liquidatif (écarts, pièces) (nouveau) | GES · `stages.allocation.gerer` | 2 mois ; conformité à l'attestation | `stages.allocation_controlee` | ST6 |
| STG-293 | Produire l'état prêt à reporter dans ApLyPro (nouveau) | GES | export journalisé, N3 | `stages.allocation_exportee` | ST6 |
| STG-294 | Constater le versement (nouveau) | GES | — | `stages.allocation_versee` | ST6 |
| STG-295 | Suivre le compteur de gratification et alerter (nouveau) | SYS | L124-6 | `stages.gratification_seuil` | ST2 |
| STG-296 | Contrôler le plafond de six mois par organisme et par année (nouveau) | SYS | L124-5 | — | ST2 |
| STG-297 | Transmettre au 24 les jours de stage pour la remise de demi-pension (nouveau) | SYS | règlement financier | `stages.jours_effectifs` | ST1 |
| STG-298 | Fournir au 26 les jours et lieux pour une aide aux frais (nouveau) | SYS | — | idem | ST11 |
| STG-299 | Comparer avec ApLyPro pendant le doublon (mode suivi) (nouveau) | GES | R-SG25 | — | ST6 |

### 19.15 Documents, conservation, pilotage, reprise
| N° | Action | Acteur · droit | Règles | Événement | Lot |
|---|---|---|---|---|---|
| STG-300 | Exporter la liste des stages d'une période (tableur) (nouveau) | DDF · `stages.exporter` | colonnes minimales ; journalisé | `stages.export` | ST1 |
| STG-301 | Produire l'état récapitulatif annuel des stages (archives 5 ans) (nouveau) | DDF | §5.8 | `documents.emis` | ST12 |
| STG-302 | Indicateurs : taux sans lieu, délais, semaines faites, visites, organismes, équité (nouveau) | DIR, DDF | agrégés (47) | — | ST11 |
| STG-303 | Archiver l'année (NST-34) | SYS | lecture seule ; pas de suppression | `stages.annee_archivee` | ST1 |
| STG-304 | Purger les jetons, codes, contacts inactifs, offres échues selon les durées (NST-34 complété) | SYS | §21.3 ; journal | `stages.purge` | ST12 |
| STG-305 | Simuler une purge (NST-34) | DIR | aperçu | — | ST12 |
| STG-306 | Répondre à une demande d'accès ou d'effacement d'un contact (nouveau) | DIR (DPO) | CSG-53 | — | ST12 |
| STG-307 | Charger l'export Next.js dans la zone de reprise (nouveau) | administrateur · `stages.reprise` | §26 | `stages.reprise_chargee` | ST1 |
| STG-308 | Rapprocher élèves, organismes et contacts de la reprise (nouveau) | SEC | humain pour l'incertain | — | ST1 |
| STG-309 | Valider la reprise et produire le rapport (nouveau) | DIR | rejouable, idempotente | `stages.reprise_validee` | ST1 |
| STG-310 | Basculer une période de Next.js vers ScolIA (propriété unique par période) (nouveau) | DIR (Florian) | R-SG26 | `stages.bascule` | ST1 |
| STG-311 | Mobilité : saisir l'organisme étranger, la convention bilingue, l'assurance, l'AST et Ariane (nouveau) | DDF, SEC | §13.6 | `stages.mobilite` | ST9 |
| STG-312 | Immersion : accueillir un élève d'un autre établissement (convention entre établissements) (nouveau) | DDF, SEC | CSG-17 | `stages.immersion` | ST8 |
| STG-313 | Collège : préparer la séquence d'observation d'une classe de 3e (paquet : période, offres, conventions, rapport, oral) (nouveau) | PP, DIR | §9.1 | — | ST8 |
| STG-314 | Mention sur le bulletin (semaines faites/dues, appréciation) (nouveau) | SYS | port du 23 | — | ST5 |
| STG-315 | Verser le stage d'observation au parcours Avenir de l'élève (nouveau) | SYS | port du 37 | — | ST8 |
| STG-316 | Réémettre les liens après une restauration (nouveau) | SEC | E9 | — | ST12 |
| STG-317 | Consulter le journal d'un stage (toutes les transitions) (nouveau) | DDF, DIR | ajout seul | — | ST1 |
| STG-318 | Consulter les accès aux données d'un stage (qui a lu quoi) (nouveau) | DIR (DPO) | journal des lectures sensibles | — | ST12 |
| STG-319 | Accessibilité de la page du tuteur (mobile, sans compte, contraste, lecture d'écran) (nouveau) | — | RGAA | — | ST1 |

**Total : 200 actions** (numéros non utilisés, réservés : 194–199, 228–229, 233–239, 247–249).

---

## 20. Règles transverses (R-SG1 à R-SG26) [R]

| N° | Règle |
|---|---|
| **R-SG1** | **Pas de convention signée, pas de stage.** Aucune situation `pfmp`/`stage` au 12, aucun non-repas au 40, aucune nuit retirée au 41 tant que toutes les signatures exigées ne sont pas réunies (corrige DN-14, RS-01). Aucune dérogation. |
| R-SG2 | Un stage référence l'élève (`eleve_id`), l'organisme, le contact, le référent (personne) ; rien n'est retapé ; le document signé fige ce qui doit l'être. |
| R-SG3 | Le serveur seul change l'état ; chaque commande vérifie le droit, l'état de départ, la version ; fait, journal et événement dans **la même transaction**. |
| R-SG4 | Les âges se calculent **à chaque jour** du stage, depuis la date de naissance du 2, en Europe/Paris. |
| R-SG5 | Les règles légales sont **bloquantes** et versionnées ; l'établissement peut les durcir, jamais les assouplir ; une convention garde la version de règles de sa signature. |
| R-SG6 | Exigences calculées par stage ; une exigence non légale peut être dérogée par la direction avec motif ; une exigence légale jamais. |
| R-SG7 | Les signataires sont **calculés** (4, 6, contacts validés) ; l'élève ne saisit jamais l'adresse d'un titulaire ; un externe ne change jamais lui-même son adresse. |
| R-SG8 | Règle d'accord des titulaires `un` par défaut (actes usuels, 7 §10.4) ; `tous` pour la mobilité internationale ; l'élève majeur signe seul. |
| R-SG9 | Un lien externe a une **portée** (un stage, une ou plusieurs actions), une **échéance**, un seul exemplaire actif par personne et par portée ; jeton haché, jamais renvoyé par une API. |
| R-SG10 | Un accident ou un danger prévient la direction **immédiatement** (envoi urgent autorisé sans délai d'annulation, mais tracé) ; les échéances de déclaration sont calculées hors dimanches et jours fériés. |
| R-SG11 | Aucune donnée de santé ni d'aménagement dans le 39 : seulement « aptitude oui/non/à revoir » et des consignes consenties lues aux 34 et 35. |
| R-SG12 | Le décompte des semaines de PFMP est **calculé** depuis les jours effectifs et les attestations ; une alerte n'est jamais une décision d'examen. |
| R-SG13 | Gratification : refus d'émettre une convention ou un avenant qui franchit le seuil de L124-6 sans clause ; plafond de six mois bloquant. |
| R-SG14 | Un organisme proposé par un tiers n'entre au carnet qu'après validation d'un personnel. |
| R-SG15 | Un organisme `interdit` ne reçoit aucune convention ; `deconseille` exige une confirmation motivée du DDFPT. |
| R-SG16 | Un code d'identification ou de signature ne part jamais vers une adresse de personnel (NST-36) ni vers une adresse non vérifiée. |
| R-SG17 | L'élève ayant un compte passe par l'ENT ; le parcours par lien n'existe que pour l'élève sans compte (immersion, nouvel arrivé). |
| R-SG18 | Une convention importée (plateforme nationale, papier) n'est `signee` que si **chaque** signature exigée est constatée par un humain. |
| R-SG19 | Une convention signée est **immuable** ; toute correction est un avenant ou une nouvelle version. |
| R-SG20 | Les indicateurs d'équité sont **agrégés** (classe, dispositif) ; aucun classement d'élève ni d'organisme par l'IA. |
| R-SG21 | Chaque lecteur reçoit une **projection** propre à son droit (§21.2) ; aucune réponse d'API ne contient un objet brut. |
| R-SG22 | Le tuteur ne voit que l'élève accueilli, ses horaires, l'établissement et le référent ; jamais les titulaires, sauf la mention prévue par la convention (§21.2). |
| R-SG23 | L'adresse de l'organisme d'un élève n'est jamais montrée à un titulaire sans droit d'information ni à un titulaire visé par une interdiction. |
| R-SG24 | L'allocation se calcule sur des **jours entiers** effectués et attestés ; une absence n'est jamais payée ; un rattrapage garde le tarif de l'année d'origine. |
| R-SG25 | Pendant l'année de doublon (D-034), l'allocation est en **mode suivi** : rien n'est transmis depuis ScolIA. |
| R-SG26 | **Propriété unique par période** à la bascule : une période est gérée soit dans Next.js, soit dans ScolIA, jamais les deux (comme R-SV37 du 42). |

---

## 21. Droits et minimisation

### 21.1 Catalogue d'actions proposé (D-017) [R]
| Action | Ce qu'elle permet | Portée | Modèles de rôle par défaut |
|---|---|---|---|
| `stages.lire` | voir les stages | établissement / mes classes / mes élèves suivis | Direction, DDFPT, Secrétariat (établissement) ; PP (ses classes) ; Professeur (ses élèves suivis) ; CPE et AED : projection « en stage » seulement |
| `stages.preparer` | préparer, soumettre, déclarer un lieu | établissement / mes élèves suivis | Secrétariat, DDFPT, Référent |
| `stages.valider` | valider, renvoyer à corriger | établissement / mes élèves suivis (contenu pédagogique) | DDFPT, Secrétariat ; Référent (pédagogie) |
| `stages.emettre` | émettre convention et avenants, relancer, gérer le circuit | établissement | Secrétariat, DDFPT |
| `stages.signer_etablissement` | signer au nom de l'établissement | établissement | Chef d'établissement et délégataires habilités (6) ★ |
| `stages.organismes.gerer` | carnet d'organismes et de contacts | établissement | Secrétariat, DDFPT ; Référent (créer, proposer) |
| `stages.offres.deposer` / `stages.offres.valider` | offres | établissement | tous les personnels / DDFPT, Secrétariat |
| `stages.periodes.gerer` | périodes, échelonnement | établissement | DDFPT, Direction |
| `stages.referents.gerer` | désigner les référents | établissement / mes classes | DDFPT ; PP (ses classes) |
| `stages.suivre` | visites, absences en entreprise, comptes rendus | mes élèves suivis | Référent |
| `stages.evaluer` | évaluation, bilan, décompte | mes élèves suivis / établissement (certifier) | Référent ; DDFPT (certifier) |
| `stages.incidents.gerer` | accidents, suspension, rupture | établissement | Direction ★ ; DDFPT |
| `stages.allocation.gerer` | allocation, coordonnées bancaires à l'export | établissement | Gestionnaire ★ |
| `stages.exporter` | exports nominatifs | établissement | DDFPT, Direction ★ |
| `stages.parametrer` | dispositifs, clauses, assurances, délais | établissement | Direction |
| `stages.reprise` | reprise de Next.js | établissement | administrateur ★ |

★ : action sensible (réauthentification récente, journal renforcé).

### 21.2 Ce que voit chacun (projections) [R]
| Lecteur | Voit | Ne voit jamais |
|---|---|---|
| Élève mineur | ses stages, horaires, organisme, tuteur (nom, téléphone professionnel), référent, état de sa convention et de sa part de signature, ses absences, son attestation | les jetons, l'état de signature détaillé de chaque parent, les notes du référent, l'évaluation de l'accueil par d'autres |
| Élève majeur | idem, et choisit si ses parents voient (2) | idem |
| Titulaire avec droit | état, horaires, adresse du site, tuteur, référent, absences, documents signés | les jetons des autres signataires ; les comptes rendus internes |
| Titulaire sans droit ou interdit | rien | tout (R-SG23) |
| Tuteur (lien) | prénom, nom, formation, dates, horaires, nom et téléphone du référent et de l'établissement, consignes consenties, sa grille d'évaluation, l'attestation à signer ; **mention « mineur » et contact d'urgence de l'établissement** | adresse et téléphone de la famille (sauf si le modèle national l'exige, à vérifier au lot ST1, Q-SG8), notes, santé, absences au lycée, autres élèves |
| Signataire de l'organisme (lien) | la convention, ses annexes, sa copie signée | le reste |
| Référent | ses élèves suivis : tout le dossier de stage, comptes rendus | coordonnées bancaires ; données de santé |
| PP | ses classes : états, organismes, absences ; pas les comptes rendus détaillés | idem |
| Professeur de la classe | « en stage du … au … » sur ses listes et son appel | organisme, adresse |
| CPE, AED, accueil | « en stage chez X jusqu'au …, référent, horaires du jour » | convention, évaluation |
| Restauration, internat | jours de stage (cause) | organisme |
| Gestionnaire | jours effectifs, allocation, coordonnées bancaires à l'export | évaluations, comptes rendus |
| Infirmière, médecin | liste des avis d'aptitude à rendre : élève, dispositif, travaux déclarés, poste | — |
| Direction | tout, journalisé pour les éléments protégés | le contenu des signalements protégés (au 36) |

### 21.3 Conservation [R, durées soumises au DPO]
| Donnée | Durée proposée | Fondement |
|---|---|---|
| Convention, avenants, attestation, état liquidatif et pièces d'un stage **ayant ouvert droit à l'allocation** | **10 ans** après la fin de l'année scolaire | FAQ allocation (ASP) [V] |
| Convention d'un stage **sans incidence financière** | 2 ans, puis destruction | instruction 2005-003 [V] |
| Convention **avec gratification** (incidence financière) | 1 an après apurement des comptes | instruction 2005-003 [V] |
| État récapitulatif annuel des stages | 5 ans, puis **conservation** (archives) | instruction 2005-003 [V] |
| Attestations et évaluations | suivent le dossier de l'élève | instruction 2005-003 [V] |
| Copie d'attestation d'une formation à risque ou liée à la sécurité (habilitation électrique) | 50 ans | instruction 2005-003 [V] |
| Offres et fiches organismes | 1 an après la dernière utilisation, puis destruction | instruction 2005-003 [V] |
| Contacts (tuteurs, signataires) du carnet | 3 ans après leur dernier stage (S-3903) ; restent dans les documents figés conservés | [R, au DPO] : la durée de 1 an de 2005 rendrait le carnet inutilisable d'une année sur l'autre ; à arbitrer |
| Comptes rendus de visite | fin du cycle de l'élève + 1 an | [R] |
| Dossier d'accident | 10 ans | [R, au DPO, prescription] |
| Jetons et codes | jetons : échéance + 30 jours ; codes : 30 minutes, effacés à l'usage | [R] |
| Journal de recherche de l'élève (démarches) | fin de la période + 1 an | [R] |
| Signalements de discrimination | 5 ans (preuve éventuelle) | [R, au DPO] |

### 21.4 Pièges à tester
1. Élève qui lit un jeton de parent (DN-01) : aucune projection ne contient de jeton.
2. Parent d'un autre élève qui lit une convention par identifiant (DN-02) : 404.
3. Utilisateur authentifié qui modifie une convention signée (DN-03) : refus.
4. Ancien lien après régénération (DN-04) : refus.
5. Personnel qui signe par lien (DN-05) : impossible.
6. Élève qui remplace l'adresse de son parent (DN-07) : champ inexistant.
7. Situation « en stage » posée avant la dernière signature (DN-14, RS-01) : impossible.
8. Titulaire interdit qui obtient l'adresse de l'organisme (R-SG23) : rien.
9. Tuteur qui obtient le téléphone d'un parent (R-SG22) : rien.
10. IA qui reçoit une aptitude ou une consigne de santé dans son contexte : test de garde.
11. Lien d'un établissement utilisé dans un autre (DN-11) : refus, RLS.
12. Code essayé 6 fois : bloqué.

---

## 22. Événements (outbox, dans la transaction du fait) [R]

**Émis** (charges utiles : identifiants, dates, états ; **jamais** de nom de contact, d'adresse, de motif de rupture ou de signalement, corrige RS-05) : `stages.organisme_cree`, `stages.organisme_etat`, `stages.organismes_fusionnes`, `stages.contact_modifie`, `pfmp.periode_planifiee`, `pfmp.periode_modifiee` (attendus par le 10), `stages.periode_publiee`, `stages.recherche_ouverte`, `stages.offre_*`, `stages.candidature_*`, `stages.convention_preparee`, `stages.convention_soumise`, `stages.convention_validee`, `stages.convention_emise`, `stages.convention_signee`, `stages.convention_importee`, `stages.depart_compromis`, **`stage.periode_ouverte`** (nom historique gelé, Q-B13 du 7 ; consommé par 12, 40, 41, 10), `stages.modifie`, `stages.annule`, `stages.suspendu`, `stages.interrompu`, `stages.repris`, `stages.rompu`, `stages.accident_signale`, `stages.incident_signale`, `stages.signalement_protege` (sans contenu), `stages.visite_planifiee`, `stages.visite_faite`, `stages.evalue`, `stages.attestation_signee`, `stages.decompte_certifie`, `stages.semaines_manquantes`, `stages.jours_effectifs`, `stages.gratification_seuil`, `stages.allocation_*`, `stages.purge`, `stages.bascule`. Noms historiques gelés de l'existant : `stage.entreprise`, `stage.convention` (`types::ENTREPRISE`, `types::CONVENTION_STAGE`), émis encore par l'adaptateur jusqu'au retrait.

**Consommés** : `eleve.sortie`, `eleve.changement_classe`, `eleve.majorite_atteinte` (2) ; `dossier.lien_modifie`, `dossier.interdiction_posee`, `dossier.coordonnee_verifiee` (4) ; `structure.calendrier_modifie` (5) ; `mission.retiree` (6) ; `documents.signature`, `documents.circuit_complet`, `documents.papier_valide` (7) ; `assiduite.absence_pfmp`, `assiduite.absence_modifiee` (12) ; `sante.aptitude_rendue` (34) ; `formulaires.reponse` (18).

## 23. Notifications (natures déclarées au socle 8) [R]
`stages.periode_ouverte` (élèves, titulaires) ; `stages.sans_lieu_relance` ; `stages.convention_a_signer` (signataire) ; `stages.convention_relance` ; `stages.depart_compromis` (direction, référent, famille, élève) ; `stages.convention_signee` (famille pour information, élève) ; `stages.lien_tuteur` (premier jour) ; `stages.absence_signalee` (référent ; la famille par le 12) ; `stages.probleme_eleve` (référent, astreinte vie scolaire) ; `stages.accident` (direction, **urgent**) ; `stages.visite_rdv` ; `stages.evaluation_demandee` (tuteur) ; `stages.attestation_relance` ; `stages.semaines_manquantes` (DDFPT, élève, famille) ; `stages.allocation_echeance` (gestionnaire, J-15 avant les deux mois). SMS (OVH, D-004) seulement pour les codes et l'urgence.

## 24. Écrans (`DESIGN_SYSTEM.md`, `FRONTEND_CIBLE.md`, Leptos)
1. **Tableau de bord des stages** (DDFPT, direction) : tuiles Bento — conventions à signer (par date de départ), sans lieu à J-30, absences du jour en entreprise, visites en retard, semaines manquantes, allocation à reporter ; frise des périodes de l'année par classe.
2. **Classe** (PP, référent) : tableau dense, une ligne par élève : état, organisme, dates, signatures (pastilles), exigences manquantes, dernière visite ; actions en lot.
3. **Dossier de stage** : en-tête (élève, organisme, dates, état, prochaine étape et qui doit agir), onglets Convention · Horaires · Signatures · Suivi · Évaluation · Documents · Journal ; contrôles réglementaires expliqués en clair.
4. **Éditeur d'horaires** : grille de la semaine, jours, exceptions ; blocages en rouge avec la règle et le texte ; aperçu « Léa a 16 ans à partir du mercredi ».
5. **Parapheur** du chef d'établissement : liste, aperçu, contrôles, signature par lot.
6. **Carnet d'organismes** : recherche, fiche, sites, contacts, historique d'accueil, état.
7. **Offres** : pour l'élève, cartes filtrées par formation ; pour le DDFPT, file de validation.
8. **Espace élève et famille** (web et mobile) : « mon stage » — état, à faire, horaires, adresse, contacts, bouton « j'ai un problème », documents.
9. **Page du tuteur** (lien, mobile d'abord, sans compte) : une page — l'élève, les horaires, « signaler une absence », « tout va bien », « signaler un problème », message au référent, évaluation, attestation. Accessible (RGAA).
10. **Référent mobile** (11) : ses élèves en stage, tournée du jour, compte rendu hors ligne, appel au tuteur.
11. **Allocation** (gestionnaire) : par période, jours, montants, écarts, état liquidatif prêt.

## 25. Modèle de données cible [R]

Règles : toutes les tables portent `etablissement_id UUID NOT NULL` (E1), FK composites `(etablissement_id, x_id)` (E2), UUID applicatifs (E3), `cree_le`, `cree_par_personne_id`, `modifie_le`, `version` sur les tables modifiables (E6), suppression = changement d'état daté (E6), RLS standard (E13), **aucune colonne JSON ni tableau ni `BYTEA`** (D-036, E5, E10). Fichiers : objets du 7 (`ent_objet`) référencés par identifiant.

```sql
-- Références globales (E12 : globale_reference ; pas d'etablissement_id, aucune donnée d'établissement)
ref_stage_dispositif (code PK, libelle, observation_seule bool, regime_at bool, allocation bool)
ref_stage_regle (id, dispositif_code NULL, code, age_min SMALLINT NULL, age_max_exclu SMALLINT NULL,
  valeur_minutes INT NULL, heure_debut TIME NULL, heure_fin TIME NULL, bloquante bool,
  texte_source TEXT, effet_du DATE, effet_au DATE NULL)          -- une ligne par règle, versionnée
ref_stage_bareme (id, niveau_code, annee_entree_cycle SMALLINT, forfait_jour_centimes INT,
  plafond_annuel_centimes INT, effet_du DATE, effet_au DATE NULL, texte_source TEXT)
ref_stage_parametre_global (code PK, valeur_centimes BIGINT NULL, valeur_texte TEXT NULL, effet_du DATE) -- plafond horaire SS

-- Paramètres de l'établissement (colonnes typées, pas de clé-valeur)
ent_stage_parametre (etablissement_id PK, assureur_nom, assurance_contrat, deliberation_ref, deliberation_le,
  derogation_trav_regl_le DATE NULL, max_stagiaires_referent SMALLINT NULL, max_stagiaires_organisme SMALLINT NULL,
  relance_ouverture_j SMALLINT, relance_sans_lieu_j SMALLINT, alerte_veille_heure TIME, modifie_le, version)
ent_stage_dispositif_etab (etablissement_id, dispositif_code, actif bool, PK (etablissement_id, dispositif_code))
ent_stage_regle_etab (id, etablissement_id, dispositif_code, regle_code, valeur_minutes INT NULL,
  heure_debut TIME NULL, heure_fin TIME NULL, bloquante bool)     -- durcissement seulement (CHECK au service)
ent_stage_clause_etab (id, etablissement_id, dispositif_code, ordre, texte TEXT CHECK (length <= 2000), version)

-- Organismes (ent_entreprise conservée et étendue par 2781)
ent_entreprise (+ type CHECK (entreprise|association|administration|collectivite|etablissement_scolaire|sante|etranger|autre),
  identifiant_type CHECK (siret|siren|rna|etranger|aucun), identifiant TEXT NULL, enseigne, code_naf, tranche_effectif,
  adresse_ligne1, adresse_ligne2, code_postal, commune, pays CHAR(2), etat CHECK (actif|a_verifier|ferme|deconseille|interdit),
  etat_motif_code NULL, etat_motif_texte TEXT NULL CHECK (length <= 300), etat_decide_par, etat_decide_le,
  origine CHECK (saisie|proposition|reprise), modifie_le, version)
ent_entreprise_site (id, etablissement_id, entreprise_id, libelle, adresse_ligne1, adresse_ligne2, code_postal, commune, pays, actif)
ent_entreprise_contact (id, etablissement_id, entreprise_id, nom, prenom, fonction, email, telephone,
  est_tuteur bool, est_signataire bool, est_rh bool, informe_le DATE NULL, dernier_stage_le DATE NULL, actif bool)
ent_entreprise_contact_changement (id, etablissement_id, contact_id, champ CHECK (email|telephone), nouvelle_valeur,
  demande_par_type, demande_le, decide_par, decide_le, decision CHECK (acceptee|refusee) NULL)
ent_entreprise_proposition (id, etablissement_id, raison_sociale, identifiant, adresse..., propose_par_type, propose_par_ref, etat, entreprise_id NULL)

-- Périodes et offres
ent_stage_periode (id, etablissement_id, annee_id, dispositif_code, libelle, du, au, type CHECK (collective|individuelle),
  periode_structure_id NULL,            -- période du 5, nature entreprise
  motif TEXT NULL, etat CHECK (prevue|publiee|close), version)
ent_stage_periode_public (id, etablissement_id, periode_id, type CHECK (classe|groupe|eleve), cible_id)
ent_stage_offre (id, etablissement_id, entreprise_id, site_id NULL, contact_id NULL, intitule, description CHECK (length <= 4000),
  places SMALLINT, du DATE NULL, au DATE NULL, age_min SMALLINT NULL, etat CHECK (proposee|publiee|pourvue|close|refusee),
  origine CHECK (personnel|famille|organisme|reprise), propose_par_ref, valide_par, valide_le, version)
ent_stage_offre_cible (id, etablissement_id, offre_id, dispositif_code, formation_id NULL)
ent_stage_candidature (id, etablissement_id, offre_id, eleve_id, etat CHECK (envoyee|vue|entretien|acceptee|refusee|retiree),
  piece_objet_id NULL, cree_le, modifie_le, UNIQUE (etablissement_id, offre_id, eleve_id))

-- Stage (dossier)
ent_stage (id, etablissement_id, eleve_id, periode_id NULL, dispositif_code, entreprise_id NULL, site_id NULL,
  tuteur_contact_id NULL, signataire_contact_id NULL, referent_personne_id NULL, du DATE NULL, au DATE NULL,
  etat CHECK (recherche|trouve|en_preparation|a_valider|a_corriger|a_signer|signe|en_cours|interrompu|rompu|termine|evalue|clos|annule),
  regles_version_le DATE NULL, travaux_reglementes bool, derogation_organisme_le DATE NULL,
  nuit_autorisee_par NULL, gratification_centimes_heure INT NULL, avantages_repas bool, avantages_transport bool,
  suivi_mode CHECK (sur_site|distance|mixte), source CHECK (scolia|reprise_nextjs|reprise_rust_082|import_plateforme|papier),
  source_ref TEXT NULL, version, cree_le, modifie_le,
  EXCLUDE USING gist (etablissement_id WITH =, eleve_id WITH =, daterange(du, au, '[]') WITH &&)
    WHERE (etat NOT IN ('recherche','annule') AND du IS NOT NULL))
ent_stage_horaire_type (id, etablissement_id, stage_id, jour_semaine SMALLINT CHECK (1..7), debut TIME, fin TIME, ordre SMALLINT)
ent_stage_horaire_exception (id, etablissement_id, stage_id, jour DATE, debut TIME NULL, fin TIME NULL, ordre SMALLINT, motif_code)
ent_stage_activite (id, etablissement_id, stage_id, ordre, libelle CHECK (length <= 300), competence_id NULL)
ent_stage_exigence (id, etablissement_id, stage_id, code, statut CHECK (attendue|remplie|sans_objet|bloquante|derogee),
  source_ref NULL, derogation_par NULL, derogation_motif NULL, modifie_le)
ent_stage_fait (id, etablissement_id, stage_id, a TIMESTAMPTZ, type, etat_avant, etat_apres, motif_code NULL,
  motif_texte NULL CHECK (length <= 500), acteur_type CHECK (personne|contact|systeme), acteur_ref)   -- ajout seul (journal)

-- Convention et documents (références au 7)
ent_stage_convention (id, etablissement_id, stage_id, numero TEXT,  -- série STG, compteur par établissement (E3)
  version SMALLINT, document_id, circuit_id, etat CHECK (brouillon|a_signer|signee|caduque|resiliee),
  signee_le TIMESTAMPTZ NULL, origine CHECK (scolia|papier|plateforme|hors_plateforme), UNIQUE (etablissement_id, numero, version))
ent_stage_avenant (id, etablissement_id, convention_id, numero SMALLINT, motif_code, document_id, circuit_id, etat, signe_le NULL)
ent_stage_avenant_modif (id, etablissement_id, avenant_id, champ_code, avant TEXT, apres TEXT)   -- instantané d'audit, jamais relu comme source
ent_stage_demande_avenant (id, etablissement_id, stage_id, demande_par_type, demande_par_ref, objet_code, texte CHECK (length <= 1000),
  etat CHECK (en_attente|acceptee|refusee), decide_par, decide_le)
ent_stage_lien (id, etablissement_id, stage_id, lien_id,  -- lien nominatif du socle 7 (jeton haché, échéance, portée)
  portee CHECK (signature|tuteur|depot|offre|identification), actif bool, revoque_le NULL)

-- Suivi
ent_stage_visite (id, etablissement_id, stage_id, referent_personne_id, type CHECK (contact_debut|suivi|rencontre|evaluation|telephone|visio),
  prevue_le DATE NULL, faite_le TIMESTAMPTZ NULL, rdv_id NULL, ordre_mission_ref NULL,
  presence_constatee bool NULL, securite_ok bool NULL, compte_rendu TEXT NULL CHECK (length <= 3000), version)
ent_stage_accueil_eval (id, etablissement_id, stage_id, auteur CHECK (referent|eleve), critere_code, niveau SMALLINT, cree_le)
ent_stage_incident (id, etablissement_id, stage_id, type CHECK (accident|danger|comportement|discrimination|vss|autre),
  survenu_le TIMESTAMPTZ, signale_par_type, signale_le, echeance_declaration TIMESTAMPTZ NULL,
  declaration_document_id NULL, renvoi_socle SMALLINT NULL, etat CHECK (ouvert|traite|clos))   -- aucun texte pour vss/discrimination

-- Évaluation, attestation, décompte
ent_stage_evaluation (id, etablissement_id, stage_id, grille_code, etat CHECK (demandee|proposee_tuteur|validee), valide_par, valide_le)
ent_stage_evaluation_ligne (id, etablissement_id, evaluation_id, competence_id, niveau_tuteur SMALLINT NULL, niveau_valide SMALLINT NULL)
ent_stage_attestation (id, etablissement_id, stage_id, document_id, jours_effectifs SMALLINT, signee_le NULL, version)
ent_stage_jour (etablissement_id, stage_id, jour DATE, attendu bool, effectif bool, cause_non_effectif CHECK (absence|ferie|fermeture|retour_lycee|interruption) NULL,
  PK (etablissement_id, stage_id, jour))   -- reconstructible depuis horaires + absences du 12 (E12 : reconstructible)
ent_stage_decompte (id, etablissement_id, eleve_id, formation_id, cycle_code, semaines_dues NUMERIC(4,1), semaines_faites NUMERIC(4,1),
  certifie_par NULL, certifie_le NULL)

-- Argent
ent_stage_allocation (id, etablissement_id, stage_id, bareme_id, jours SMALLINT, montant_centimes INT,
  beneficiaire_type CHECK (eleve|titulaire|tiers), beneficiaire_personne_id, autorisation_reponse_id NULL,
  etat CHECK (a_calculer|calculee|controlee|reportee_aplypro|versement_constate|ecart), echeance DATE, version)
ent_stage_gratification_compteur (etablissement_id, eleve_id, entreprise_id, annee_id, jours SMALLINT, PK (...))  -- reconstructible

-- Reprise
ent_stage_reprise_lot (id, etablissement_id, source CHECK (nextjs|rust_082), charge_le, etat, rapport_document_id NULL)
ent_stage_reprise_ligne (id, etablissement_id, lot_id, source_type, source_id, cible_type, cible_id NULL,
  statut CHECK (a_rapprocher|rapprochee|ignoree|en_erreur), raison_code NULL)
ent_stage_reprise_champ (id, etablissement_id, ligne_id, chemin TEXT, valeur TEXT)  -- zone de transit, purgée après validation (§26)
```

**Liens externes** : pas de table de jetons propre au 39. Les liens nominatifs (signature, tuteur, dépôt, identification) sont ceux du **socle 7** (§10.5 : jeton aléatoire stocké **haché**, échéance, portée), avec la table de rattachement `ent_stage_lien` ci-dessus : le 39 ne stocke **aucun** jeton ni aucune empreinte de jeton.

**Contrôles de revue** : `grep -i "json\|bytea\|\[\]"` sur 2780–2799 → vide, sauf `ent_stage_reprise_champ` (texte, zone de transit) et `ent_stage_avenant_modif` (instantané d'audit), **déclarés** à `MIGRATIONS.md` comme non-JSON mais à surveiller.

**Registre E12** : `ref_stage_*` globale_reference ; `ent_stage_jour`, `ent_stage_gratification_compteur` reconstructibles ; `ent_stage_lien` neutralisé à la restauration (E9) ; `ent_stage_reprise_*` volatiles (purgées) ; le reste établissement, personnel, export simple ; `ent_stage_incident` et `ent_stage_allocation` export particulier (accès restreint).

## 26. Plan de reprise (D-035, point 4)

1. **Aucun accès à la base de production.** Florian fournit un export (`stage_offer`, `stage_convention`, `stage_application` et leurs `_attr`, plus les PDF et scans du stockage objet, avec empreintes) ; **anonymisé** pour les essais ; réel seulement après l'accord écrit de l'établissement et l'avis du DPO (printemps 2027). Les **jetons et codes ne sont pas repris** (`stage_token` exclu, TRANSITION_T0 §R).
2. Chargement en **zone de reprise** (`ent_stage_reprise_*`), reconstruction du dossier aplati (codec `path → value`), puis conversion.
3. **Correspondances** :

   | Next.js | ScolIA |
   |---|---|
   | `draft`, `admin_review`, `admin_rejected` | `en_preparation`, `a_valider`, `a_corriger` |
   | `convention_ready`, `signatures_pending` | `a_signer` : **circuit relancé dans ScolIA** (signatures Next.js partielles non reprises comme preuves, R-SG18) — ou fini dans Next.js (propriété unique, R-SG26) |
   | `signed` | `signe` (ou `termine` / `clos` selon la date) ; PDF signé repris comme document **P** avec empreinte ; preuve Next.js conservée en pièce, marquée « hachage non cryptographique » |
   | `convention_deposited` | `a_valider` (papier à contrôler) |
   | `cancelled`, `archived` | `annule`, `clos` |
   | élève (INE, nom) | `eleve_id` : INE, puis file humaine ; **jamais** de rapprochement par nom automatique |
   | entreprise recopiée dans chaque convention | **organisme** dédoublonné (SIRET, puis nom + code postal proposé) ; contacts créés |
   | `teacherReferent` | mission du 6 |
   | horaires | `ent_stage_horaire_*` ; **contrôles rejoués** : une convention signée non conforme est signalée, pas modifiée |
   | offres | offres `publiee`/`close` |
   | discussion | fil en lecture seule (16) |
   | périodes, contraintes, référents (réglages en « jetons ») | périodes, règles d'établissement, missions |

4. **Rust 082** : chaque `ent_convention_stage` devient un `ent_stage` `source = reprise_rust_082`, état `trouve` (pas `signe`), et `ent_entreprise` est étendue en place.
5. **Rapport** : nombre de stages, d'organismes dédoublonnés, d'élèves à rapprocher, de conventions signées sans PDF, d'horaires non conformes ; rejouable, idempotent par `source_id`.
6. **Bascule** : par **période** (R-SG26) ; les nouvelles périodes naissent dans ScolIA à partir d'une date fixée par Florian ; les périodes en cours finissent dans Next.js ; l'arrêt de l'écran Next.js est une décision de Florian.

## 27. Lots de code et migrations

### 27.1 Lots
| Lot | Contenu | Dépend de | Critères de fin |
|---|---|---|---|
| **ST0** correctifs | §2.6 | — | RS-01 à RS-10 corrigés et testés |
| **ST1** parité branchée (a–d) | **(a)** paramètres, dispositifs, organismes, sites, contacts, périodes, référents (mission 6), projections ; **(b)** stage, horaires, contrôles horaires (§5.2), exigences, préparation par l'élève dans l'ENT, validations ; **(c)** convention et avenants par le 7 (S1, S2, P, G), circuit calculé, liens externes, parapheur, relances, papier, offres et candidatures, fil ; **(d)** R-SG1 : situation au 12, jours au 40 et 41, séances au 10, projection accueil, reprise Next.js et 082, comparaison | 2, 4, 5, 6 (missions, habilitations), 7 (D1 : documents, circuits, liens externes, S1/S2), 8, 12 (situations), 40 | parité NST-01 à NST-37 action par action ; DN-01 à DN-07 et DN-14 impossibles (tests) ; reprise d'un export anonymisé sans perte |
| **ST2** contrôles réglementaires complets | travaux réglementés, dérogations, aptitude (34), nuit d'un majeur, dérogation de durée, quotas, gratification et plafond de 6 mois, dérogations non légales | ST1 ; 34 (aptitude) | une table de cas par règle du §5.2 |
| **ST3** pendant le stage | lien du tuteur (absences par le 12, « tout va bien »), « j'ai un problème », incidents, accidents (circuit par dispositif), suspension, interruption, rupture, signalements protégés (36), « ni en entreprise ni au lycée » | ST1 ; 12 AS11 ; 36 | CSG-19, CSG-20, CSG-40, CSG-56, CSG-57 |
| **ST4** visites et mobile | exigences de suivi, tournée, rendez-vous (19), comptes rendus hors ligne (11), ordre de mission, évaluation de l'accueil | ST1 ; 10 (plages), 11, 19 | — |
| **ST5** évaluation et examen | grilles (22), positionnement du tuteur, bilan, note au 21, attestation, décompte certifié (38), semaines manquantes, deux semaines de terminale, bulletin (23) | ST3 ; 21, 22, 38 | CSG-41, CSG-42 |
| **ST6** allocation | barème, bénéficiaire, autorisation annuelle (18), état liquidatif, export ApLyPro, mode suivi | ST5 ; 4 (RIB), 18 | calcul exact sur 20 cas, dont rattrapage et absences |
| **ST7** EDT | échelonnement simulé, séances touchées, plages libérées | ST1 ; 10 T15 | EDT-233, EDT-234 |
| **ST8** collège, seconde, immersion | paquet « 3e », import 1élève1stage et lots de PDF, immersion d'élèves extérieurs, parcours Avenir (37) | ST1 | R-SG18 |
| **ST9** mobilité internationale | organisme étranger, convention bilingue, AST, Ariane | ST2 ; 4 | — |
| **ST10** IA | capacités du §18.3, lecture locale des PDF | ST1 ; `ent-ia` | tests de garde |
| **ST11** pilotage et réseau | indicateurs, équité, relance des organismes, journal de recherche, Sirene (si accepté), aides (26) | ST5 | — |
| **ST12** conservation et retrait | purges, état récapitulatif, demandes des contacts, réémission après restauration, RLS stricte, retrait des tables héritées | tous | §21.3 vérifié |

**Parcours pour la rentrée 2027** (usage réel, D-035 : le module Next.js est en service) : **ST0, ST1, ST2, ST3, ST5, ST6 (mode suivi)**. ST1 vise la V1 de février 2027 (D-034).

### 27.2 Migrations (plage imposée 2780–2799, `MIGRATIONS.md`)
| N° | Lot | Contenu |
|---|---|---|
| **2780** | ST0 | statut `declaree`, FK composites et `RESTRICT`, `cree_par_personne_id`, `modifie_le`, `version`, unicité SIRET partielle, exclusion de chevauchement sur 082 |
| 2781 | ST1a | références globales (`ref_stage_*`), paramètres et règles de l'établissement, extension d'`ent_entreprise`, sites, contacts, changements, propositions |
| 2782 | ST1a | périodes, publics |
| 2783 | ST1c | offres, cibles, candidatures |
| 2784 | ST1b | stage, horaires, activités, exigences, faits (journal) |
| 2785 | ST1c | conventions, avenants, demandes d'avenant, rattachement des liens du 7 |
| 2786 | ST1d | jours (reconstructible), reprise (lots, lignes, champs) et conversion de 082 |
| 2787 | ST2 | dérogations, aptitudes demandées, compteur de gratification |
| 2788 | ST3 | incidents |
| 2789 | ST4 | visites, évaluation de l'accueil |
| 2790 | ST5 | évaluations, lignes, attestations, décompte |
| 2791 | ST6 | allocation |
| 2792 | ST9 | mobilité (identifiants étrangers, AST) |
| 2793–2796 | réserve | passe personas, ST8, ST11 |
| **2797** | ST12 | RLS stricte |
| 2798 | réserve | — |
| **2799** | ST12 | retrait de `ent_convention_stage` (après reprise et durée d'archive, décision et sauvegarde vérifiée, E8) |

## 28. Tests et recette
- **Unitaires** : moteur de règles horaires (âge par jour × dispositif × règle), anniversaire en cours de stage, jours fériés, nuit, repos ; calcul des jours effectifs ; décompte des semaines ; barème (niveaux, année d'entrée, rattrapage, plafond) ; seuil de gratification ; transitions (état × commande × rôle) ; signataires calculés (mineur, majeur, séparés, tutelle, ASE).
- **Intégration** : RLS entre deux établissements ; R-SG1 de bout en bout (aucune situation avant la dernière signature) ; concurrence sur la dernière place ; outbox en transaction ; répercussions du §6.4 ; reprise idempotente.
- **Sécurité** : chaque DN critique et haut devient un test qui échouerait sur le comportement de Next.js ; pièges du §21.4.
- **Recette comparée** (D-035) : les 37 NST rejoués sur une période anonymisée, Next.js et ScolIA côte à côte.
- **Personas** : `PERSONAS_39.md`.

## 29. Réglages signalés (D-023) — S-3900 à S-3949
| N° | Réglage | Défaut | Bornes |
|---|---|---|---|
| S-3900 | Dispositifs actifs | selon le type d'établissement | — |
| S-3901 | Information à l'ouverture d'une période | J-60 | J-90 à J-15 |
| S-3902 | Relance des « sans lieu » | J-30 et J-15 | — |
| S-3903 | Conservation des contacts inactifs | 3 ans | 1 à 5 ans (DPO) |
| S-3904 | Coordonnées du contact d'une offre visibles des élèves | non (l'établissement transmet) | oui / non |
| S-3905 | L'établissement signe en dernier | oui | oui / non |
| S-3906 | Alertes de convention incomplète | J-7, J-3, J-1 à 16 h | — |
| S-3907 | Avis à la famille « départ impossible » | préparé, envoyé après confirmation | préparé / automatique |
| S-3908 | Suivi minimal par dispositif | légal + rien | ajouter, jamais retirer |
| S-3909 | Plafond horaire de la sécurité sociale (global, plateforme) | à saisir chaque année | — |
| S-3910 | Nombre maximal d'élèves par référent (CA) | non fixé | — |
| S-3911 | Nombre maximal de stagiaires par organisme (académie) | non fixé | — |
| S-3912 | Règles horaires plus strictes de l'établissement (dont 7 h/jour, 30 h avant 15 ans, reprises de Next.js) | reprises de Next.js | jamais sous le légal |
| S-3913 | Stage chez un titulaire ou dans l'entreprise familiale | autorisé avec alerte | autorisé / interdit |
| S-3914 | Règle d'accord des titulaires pour la PFMP | `un` | `un` / `tous` |

---

## 30. Décisions appliquées par défaut (D-031) et questions à Florian

### 30.1 Appliquées par défaut (D-031) — Florian peut revenir dessus
| N° | Question | Recommandation appliquée |
|---|---|---|
| Q-SG11 | Garder le Rust 082 ou repartir de zéro ? | **Garder et corriger** (ST0), étendre `ent_entreprise` en place, convertir `ent_convention_stage` en `ent_stage` « trouvé » puis retirer la table (2799) |
| Q-SG12 | Départ sans convention complète, avec dérogation de la direction ? | **Jamais** (R-SG1) |
| Q-SG13 | Préconvention de l'élève : par lien (Next.js) ou dans l'ENT ? | **Dans l'ENT** pour tout élève qui a un compte ; lien seulement sans compte (R-SG17) |
| Q-SG14 | Ordre des signatures | parallèle, **établissement en dernier** (S-3905) |
| Q-SG15 | Élève mineur co-signataire en plus de ses titulaires ? | **Oui** (7 §10.4), sans remplacer ses titulaires ; la convention-type ne l'exige pas [V] |
| Q-SG16 | Règle d'accord des titulaires | **un suffit** (acte usuel), **tous** pour la mobilité internationale |
| Q-SG17 | Tracé au doigt | **plus une signature** : S1 (code), S2 (passkey), G (guichet), P (papier) |
| Q-SG18 | Valeurs horaires plus strictes de Next.js (7 h/jour, 30 h avant 15 ans) | **gardées** comme règles de l'établissement (S-3912), la loi restant le plancher |
| Q-SG19 | Lecture automatique des PDF par Mistral (Next.js) | **lecture locale** ; aucune API externe avec des pièces d'élèves |
| Q-SG20 | Import CSV de contacts | **supprimé** (le 4 fait foi) |
| Q-SG21 | « Watchers » | remplacés par des **projections par droit** |
| Q-SG22 | Conservation des contacts | **3 ans** après le dernier stage (au DPO) |
| Q-SG23 | Conventions ayant ouvert l'allocation | **10 ans** (exigence ASP) plutôt que 1 à 2 ans (instruction 2005) |
| Q-SG24 | Allocation pendant le doublon | **mode suivi** (R-SG25) |
| Q-SG25 | Visite collective d'entreprise | **socle 42** (sortie), pas un stage |
| Q-SG26 | Vocabulaire du socle 6 « tuteur de stage » pour la mission interne | renommée **« enseignant référent de stage »** ; « tuteur » réservé à l'organisme (à reporter au 6) |
| Q-SG27 | Bascule | **par période**, propriété unique (R-SG26), nouvelles périodes dans ScolIA à une date fixée par Florian |
| Q-SG28 | Signalements de discrimination et de VSS | **référent et direction seulement**, renvoi au 36, aucun contenu dans le 39 |

### 30.2 Soumis à Florian (engagent l'établissement, des coûts, des données réelles, le DPO ou un juriste)
| N° | Sujet | Pourquoi Florian | Recommandation |
|---|---|---|---|
| **F-SG1** | **Failles critiques de Next.js en production** (DN-01 à DN-04 : l'élève peut lire les codes et liens de signature de ses parents et de son tuteur ; tout parent lit toute convention ; tout utilisateur modifie toute convention ; anciens liens valables) | Next.js est intouchable pour le pilote ; seul Florian décide d'une correction en production | **Corriger en priorité côté Next.js** (masquer `signatures` dans les réponses publiques, contrôler l'accès parent et la modification), ou restreindre l'usage ; ne pas attendre ScolIA |
| Q-SG1 | Équivalent de la délibération du CA dans le privé (convention-type) | décision de l'organisme de gestion | une décision du chef d'établissement, consignée (STG-103) |
| Q-SG2 | Assurance de l'établissement (assureur, numéro de contrat) | contrat | saisie avant la première convention |
| Q-SG3 | Vérification Sirene (API INSEE) | service externe | **non** au lancement ; à étudier (gratuite, publique) |
| Q-SG4 | Géocodage des adresses pour les tournées (Base adresse nationale) | service externe | **non** au lancement ; regroupement par code postal |
| Q-SG5 | Procédure ApLyPro réelle de l'établissement (qui saisit, quand) | données réelles | ScolIA prépare l'état, une personne habilitée reporte |
| Q-SG6 | Établissement sous contrat d'association ou simple (éligibilité de l'allocation) | fait de l'établissement | à confirmer |
| Q-SG7 | Élèves étrangers : autorisation de travail en PFMP | juriste | exigence « à vérifier » en attendant |
| Q-SG8 | Coordonnées de la famille visibles du tuteur (modèle national) | juriste / DPO | non par défaut ; contact d'urgence = établissement |
| Q-SG9 | Classement des conventions sur OneDrive (Next.js) | service externe Microsoft, contrat | **non repris** ; dossier de l'élève et coffre du 7 |
| Q-SG10 | Écart entre le code Next.js lu et la production | fait | à confirmer avant la recette comparée |
| Q-SG29 | Lycée professionnel dans le premier pilote (comme Q-EDT24) | périmètre | décide la priorité de ST5–ST7 |
| Q-SG30 | Durées de conservation (§21.3) | **DPO** | proposées |
| Q-SG31 | Niveau de signature suffisant (S1/S2) pour les conventions | **juriste** (comme Q-D8 du 7) | S2 personnels, S1 externes, dossier de preuve |
| Q-SG32 | Export réel des données Next.js (stages) | accord écrit de l'établissement, DPO | export anonymisé d'abord |
| Q-SG33 | Arrêt du module Next.js des stages | décision et geste de Florian | après une période complète réussie dans ScolIA |

## 31. Faits, hypothèses, contradictions
**Faits [F]** : module Next.js riche (34 routes, ≈ 7 300 lignes de cœur), en usage ; stockage clé-valeur et pseudo-JSON redirigé vers Postgres ; aucune gratification, visite, évaluation, attestation, allocation ; écriture des absences de stage désactivée depuis le 23-09-2026 ; quatre failles critiques ou hautes de contrôle d'accès et de jetons. Rust : embryon 082 (une table d'organismes, une « convention » sans signature), droits par rôle, événement hors transaction.

**Hypothèses [H]** : calendrier du §4 ; usage réel par l'établissement de Florian des offres déposées par les parents ; valeur 2026 du plafond horaire de la sécurité sociale ; modèles éduscol non lus ; signataire de l'organisme habilité ; conséquences d'un manque de semaines pour l'examen.

**Contradictions relevées** :
1. **Socle 6** : la mission interne « tuteur de stage » (`06_PERSONNELS.md` §1, §6, `proprietaire_socle 39`) désigne l'enseignant référent ; « tuteur » est, dans les textes, la personne de l'organisme (L124-9). Proposition : renommer la mission « enseignant référent de stage » (Q-SG26).
2. **Socle 12** (`RA-30`) cite L124-15, D124-3, D124-4 pour l'interruption maladie : non relus ici ; la convention-type (art. 20) parle de « validation alternative ou report ». Rien d'incompatible, à harmoniser au lot ST3.
3. **Socle 7** (§10.1) indiquait une recherche négative sur la signature électronique des conventions ; c'est confirmé (aucun texte n'impose de niveau), et l'arrêt de la Cour de cassation du 5-03-2026 rappelle que seule la qualifiée est présumée fiable.
4. **Socle 7** (§10.4) fait co-signer l'élève mineur ; la convention-type de PFMP fait signer « l'élève ou son représentant légal s'il est mineur » : la co-signature est un **ajout** de ScolIA, pas une obligation.
5. **Instruction d'archives 2005** (conventions 1 à 2 ans) contre **ASP** (10 ans) : 10 ans retenus pour les stages à allocation.
6. **Next.js** : 7 h par jour et 30 h par semaine avant 15 ans, contre 8 h et 35 h pour les mineurs dans la convention-type de PFMP : niveaux distingués (S-3912).
7. **Next.js** : la présence « en stage » commence avant la signature, la cantine ne compte que les jours travaillés : une seule règle (R-SG1, jours réels).

---

## 32. Ajout personas : année suivante, mise à l'abri, convention de l'organisme, attestation (`PERSONAS_39.md`)

**En cas de contradiction avec les §0 à §31, ce §32 l'emporte.** Rien n'a été supprimé plus haut.

### 32.1 Les quatre bloquants et leur traitement (appliqués par défaut, D-031)
- **A — Préparer en juin l'année suivante** (M-3901) : périodes et stages de l'année N+1 créés dès juin, rattachés à l'**inscription prévue** (socle 3) et à la **formation visée** ; reclassement automatique à la rentrée ; non-réinscription → annulation avec avis (R-SG27). STG-120 (« dans l'année du 5 ») est **élargi** à l'année suivante dès qu'elle est prévue au 5 ; l'archivage annuel (STG-303) n'archive jamais un stage de l'année N+1.
- **B — Mise à l'abri** (M-3958) : au signalement d'une violence, d'un harcèlement ou d'un danger imputé à l'organisme, **tous** ses liens pour ce stage sont suspendus, relances et avis coupés, fil gelé, absences invisibles pour lui ; un humain l'informe par un message neutre préparé (R-SG28).
- **C — Convention de l'organisme et convention-cadre** (M-3945, M-3922) : acceptée comme document P si **chaque** mention exigée par la trame du dispositif est constatée et **chaque** signature exigée présente ; ScolIA joint son annexe pédagogique et ses horaires contrôlés ; convention-cadre annuelle avec annexes individuelles (R-SG29).
- **D — Attestation non rendue** (M-3941) : attestation **constatée en présence** par le référent (niveau G du 7, sur son mobile) ; relances ; échéance ApLyPro suivie ; jamais d'attestation établie par l'établissement à la place de l'organisme (R-SG30).

### 32.2 Actions ajoutées (STG-320 à STG-359)
| N° | Action | Acteur · droit | Règles | Événement | Lot | Manque |
|---|---|---|---|---|---|---|
| STG-320 | Planifier les périodes de l'année N+1 dès qu'elle est prévue au 5 | DDF · `stages.periodes.gerer` | R-SG27 | `pfmp.periode_planifiee` | ST1 | M-3901 |
| STG-321 | Créer les stages N+1 des élèves ayant une inscription prévue, par formation visée | SYS | R-SG27 | `stages.recherche_ouverte` | ST1 | M-3901 |
| STG-322 | Reclasser les stages N+1 dans la classe réelle à la rentrée | SYS | aucun changement de convention | `stages.modifie` | ST1 | M-3901 |
| STG-323 | Annuler les stages d'un élève non réinscrit, avis préparé aux signataires | SYS, SEC | N3 | `stages.annule` | ST1 | M-3901 |
| STG-324 | Faire signer en juin une convention de septembre (référent N+1 provisoire, signataire de l'établissement de l'année en cours) | SEC, DIR | 7 ; R-SG27 | `stages.convention_emise` | ST1 | M-3901 |
| STG-325 | Plan pluriannuel des semaines par élève (cycle, redoublement, réorientation) | DDF | R-SG12 | — | ST5 | M-3902 |
| STG-326 | Enregistrer un positionnement réglementaire (réduction de durée) avec sa décision | DDF | pièce ; semaines dues recalculées | `stages.positionnement` | ST5 | M-3903 |
| STG-327 | Solliciter les organismes par formation et période ; réponse en un clic par lien | DDF | N3 ; organismes actifs | `stages.offre_sollicitee` | ST1 | M-3904 |
| STG-328 | Dupliquer une période d'un niveau à l'autre avec décalage | DDF | — | `pfmp.periode_planifiee` | ST1 | M-3905 |
| STG-329 | Proposer dans le parapheur la délégation temporaire en cours (6) | SYS | habilitation datée du 6 | — | ST1 | M-3907 |
| STG-330 | Mettre à l'abri sur signalement : suspendre tous les liens de l'organisme pour ce stage | SYS (à la réception), DIR | R-SG28 ; immédiat | `stages.mise_a_l_abri` (sans contenu) | ST3 | M-3958 |
| STG-331 | Couper relances et avis vers l'organisme, geler le fil, masquer les absences | SYS | R-SG28 | — | ST3 | M-3958 |
| STG-332 | Préparer le message neutre à l'organisme ; envoi par un humain | DIR | N3 | `notifications.avis_cree` | ST3 | M-3958 |
| STG-333 | Prévenir l'astreinte de l'établissement hors heures ; afficher le numéro national choisi | SYS | 6 (astreinte) ; Q-SG39 | `stages.alerte_astreinte` | ST3 | M-3959 |
| STG-334 | Accepter une convention de l'organisme : contrôle des mentions case par case | SEC · `stages.emettre` | R-SG29 | `stages.convention_importee` | ST1 | M-3945, M-3922 |
| STG-335 | Joindre l'annexe pédagogique et les horaires contrôlés de ScolIA à une convention d'organisme | SEC | R-SG29 | `documents.emis` | ST1 | M-3945 |
| STG-336 | Enregistrer une convention-cadre annuelle avec un organisme | DIR, SEC | P ou S1 ; mentions contrôlées | `stages.convention_cadre` | ST1 | M-3945 |
| STG-337 | Émettre une annexe individuelle par élève sous convention-cadre, tuteur désigné par site | SEC, ORG | circuit allégé : tuteur, élève ou titulaires, référent | `stages.convention_emise` | ST1 | M-3946, M-3947 |
| STG-338 | Désigner un signataire unique de l'organisme pour plusieurs sites | SEC | — | — | ST1 | M-3946 |
| STG-339 | Faire signer l'attestation par le tuteur en présence du référent (mobile, G) | REF, TUT | R-SG30 ; hors ligne, envoi au retour du réseau | `stages.attestation_signee` | ST5 | M-3941 |
| STG-340 | Relancer l'attestation (J+3, J+10, J+30) | SYS | S-3916 | `notifications.avis_cree` | ST5 | M-3941 |
| STG-341 | Tableau des attestations manquantes et des échéances ApLyPro | DDF, GES | S-3917 | — | ST6 | M-3952 |
| STG-342 | Rattrapage sans perte pour un élève dont l'interruption ne lui est pas imputable | DDF | R-SG33 | `pfmp.periode_planifiee` | ST3 | M-3960 |
| STG-343 | Indicateur de risque par convention dans le parapheur | SYS | mineur, travaux réglementés, organisme nouveau, pas de visite | — | ST2 | M-3908 |
| STG-344 | Produire l'attestation « aucun départ sans convention » d'une période | DIR | état calculé, daté, signé | `documents.emis` | ST1 | M-3909 |
| STG-345 | Ouvrir la fiche de crise d'un accident | DIR | élève, organisme, tuteur, référent, titulaires joignables, assurances, état de la déclaration | — | ST3 | M-3910 |
| STG-346 | Notifier immédiatement le référent d'une absence en entreprise | SYS | 8 ; push | `notifications.avis_cree` | ST1 | M-3911 |
| STG-347 | Kit hors ligne minimal du référent (adresses, téléphones, horaires du jour) | REF | 11 | — | ST1 | M-3912 |
| STG-348 | Déclarer les kilomètres d'une visite | REF | sans géocodage ; Q-SG40 | — | ST4 | M-3913 |
| STG-349 | Visite commune à plusieurs élèves d'un organisme ; remplacement ponctuel d'une visite | REF, DDF | — | `stages.visite_planifiee` | ST4 | M-3914, M-3915 |
| STG-350 | Regrouper deux stages d'une même séquence (seconde, deux lieux) | SYS | une obligation, deux conventions | — | ST8 | M-3955 |
| STG-351 | Publier volontairement une offre à d'autres établissements ScolIA (sans aucune donnée d'élève) | DDF | **attend Q-SG34** | `stages.offre_partagee` | ST11 | M-3917 |
| STG-352 | Exiger l'habilitation électrique et le carnet de formation ; conserver la copie 50 ans | SEC | §5.8 | — | ST2 | M-3927 |
| STG-353 | Suivre les échéances : aptitude annuelle, dérogation de l'organisme (3 ans) | SYS | alerte avant la fin du stage | `stages.echeance` | ST2 | M-3928, M-3929 |
| STG-354 | Donner à l'avance l'autorisation de nuit effective à la majorité | DIR | art. 9 | `stages.nuit_autorisee` | ST2 | M-3931 |
| STG-355 | Remettre un document expurgé au titulaire visé par une interdiction ; consigne « ne doit pas contacter » à l'organisme, sans motif | SYS, SEC | R-SG32 ; Q-SG35 | — | ST1 | M-3934, M-3935 |
| STG-356 | Prévoir la présence de l'AESH et un rendez-vous préalable avec l'organisme (PPS) | REF | 35, 6, 19 ; consentement | `agenda.rdv` | ST4 | M-3937, M-3938 |
| STG-357 | Un lien par tuteur et par année, portée par élève ; appareil vérifié 30 jours | SYS | R-SG31 ; S-3915 | — | ST1 | M-3940, M-3943 |
| STG-358 | Décision d'attribution annuelle, pièces de l'allocation par élève, dossier de preuve pour l'ASP | GES · `stages.allocation.gerer` | 10 ans | `stages.allocation_attribution` | ST6 | M-3948, M-3949, M-3951 |
| STG-359 | Avis « une signature a été faite en votre nom » sur un second canal | SYS | SMS si l'e-mail a servi | `notifications.avis_cree` | ST1 | M-3968 |

Précisions sans nouveau numéro : STG-159 devient une **semaine d'accueil interne type** (S-3918, M-3918) ; STG-116 inclut la **vérification par rappel** au numéro du carnet (M-3969) ; STG-300 et tout export de contacts suivent R-SG34 (M-3970) ; STG-172 transmet au 34 **les travaux et machines déclarés** (M-3953) ; STG-157 est lisible du PP (M-3920) ; l'allocation d'un élève devenu majeur suit le bénéficiaire **à la date de l'état liquidatif** (M-3932).

### 32.3 Règles ajoutées (R-SG27 à R-SG34)
| N° | Règle |
|---|---|
| R-SG27 | Un stage de l'année N+1 est rattaché à l'inscription prévue et à la formation visée ; il suit l'élève à la rentrée ou s'annule s'il ne se réinscrit pas. |
| R-SG28 | Un signalement de violence, de harcèlement ou de danger imputé à l'organisme **coupe immédiatement** tout ce que l'organisme reçoit ou voit pour ce stage ; seul un humain l'informe, sans motif. |
| R-SG29 | Une convention de l'organisme vaut convention si **chaque** mention exigée et **chaque** signature exigée sont constatées par un humain ; l'annexe pédagogique et les horaires de ScolIA y sont joints. |
| R-SG30 | L'établissement n'établit jamais l'attestation à la place de l'organisme ; il peut **constater en présence** la signature du tuteur (G). |
| R-SG31 | Un tuteur a **un lien par année scolaire**, dont la portée liste ses stagiaires ; le code est demandé pour signer et évaluer. Remplace la partie « un lien par stage » de R-SG9. |
| R-SG32 | Un destinataire visé par une interdiction ne reçoit qu'un **document expurgé** (sans adresse de l'organisme ni nom du tuteur), ou n'est pas invité, selon la décision du juriste (Q-SG35). |
| R-SG33 | Une interruption non imputable à l'élève (fermeture, violences, défaillance de l'organisme) ouvre un **rattrapage** sans conséquence sur son dossier ; le motif n'apparaît ni sur l'attestation ni sur le bulletin. |
| R-SG34 | L'export des contacts est réservé aux rôles habilités, journalisé, plafonné (200 lignes, au-delà double validation). |

### 32.4 Cas limites ajoutés (CSG-58 à CSG-75)
| N° | Cas | Comportement |
|---|---|---|
| CSG-58 | Convention de septembre signée en juin, l'élève redouble | stage reclassé sur la formation réelle ; si le dispositif change, convention caduque, avis |
| CSG-59 | Élève non réinscrit en juillet, convention de septembre signée | annulation, avis préparé à l'organisme (N3) |
| CSG-60 | Référent de N+1 inconnu en juin | référent provisoire (DDFPT) ; transfert à la rentrée |
| CSG-61 | Chef d'établissement qui change au 1er septembre | la signature de juin reste valable (habilitation à sa date) |
| CSG-62 | Signalement de violences un dimanche soir | astreinte prévenue ; liens coupés ; le lundi, l'élève est attendu au lycée |
| CSG-63 | Signalement **contre l'élève** par le tuteur (vol, violence) | pas de mise à l'abri de l'organisme ; incident au 13 ; procédure contradictoire |
| CSG-64 | Signalement non fondé établi après enquête | rétablissement des liens par la direction, motif interne |
| CSG-65 | Convention d'enseigne sans horaires jour par jour | mention manquante → pas `signe` ; annexe ScolIA des horaires à faire signer |
| CSG-66 | Convention-cadre expirée en cours d'année | alerte 30 jours avant ; annexes suivantes bloquées |
| CSG-67 | Annexe individuelle signée sans convention-cadre valide | refus |
| CSG-68 | Tuteur qui refuse de signer l'attestation (conflit) | constat par la direction ; demande écrite à l'organisme ; décision humaine sur le décompte, jamais d'attestation fabriquée |
| CSG-69 | Visite de bilan sans réseau | attestation signée hors ligne, transmise au retour du réseau, horodatage de l'appareil et du serveur |
| CSG-70 | Aptitude d'un élève expirée la veille du départ | stage bloqué si travaux réglementés |
| CSG-71 | Dérogation de l'organisme expirant pendant le stage | alerte ; travaux réglementés interdits après la date |
| CSG-72 | Élève qui devient majeur après l'état liquidatif de la période | bénéficiaire de cet état inchangé |
| CSG-73 | Titulaire visé par une interdiction qui demande la convention | document expurgé (R-SG32) |
| CSG-74 | Deux stages de seconde dans deux lieux, l'un annulé | séquence incomplète ; recherche rouverte pour une semaine |
| CSG-75 | Tuteur qui encadre deux élèves de deux établissements ScolIA | deux liens distincts (un par établissement) ; aucune donnée croisée |

### 32.5 Lots révisés (remplacent le parcours du §27.1)
| Lot | Changement |
|---|---|
| ST1 | **élargi** : année N+1 (STG-320 à STG-324, STG-328), convention de l'organisme et convention-cadre (STG-334 à STG-338), délégation dans le parapheur, notification d'absence, kit hors ligne du référent, lien par tuteur et par année, document expurgé, second canal, attestation « aucun départ sans convention » |
| **ST3a** (nouveau, avec ST1) | **mise à l'abri** (STG-330 à STG-333) et « j'ai un problème » : aucun usage réel sans eux |
| ST3 | le reste du suivi pendant le stage, fiche de crise, rattrapage sans perte |
| ST5 | plan pluriannuel, positionnement, attestation en présence, relances |
| ST6 | décision d'attribution, pièces, dossier de preuve, échéances |

**Parcours pour la rentrée 2027** (remplace le §27.1) : **ST0, ST1, ST3a, ST2, ST3, ST5, ST6** (en mode suivi). Pour que les PFMP de septembre 2027 naissent dans ScolIA, **ST1 et ST3a doivent être en production en juin 2027** (bloquant A, Q-SG38).

**Migrations** : la **2793** reçoit les compléments de la passe (`ent_stage_convention_cadre`, `ent_stage_annexe_individuelle`, `ent_stage_habilitation` (titre, niveau, date, objet de la copie), `ent_stage_echeance` (aptitude, dérogation), `ent_stage_allocation_piece` (type, statut, objet du 7), `ent_stage_mise_a_l_abri` (stage, déclenchée le, par, levée le, sans contenu)) ; 2794–2796 restent en réserve. Aucune colonne JSON, aucun tableau, aucun `BYTEA`.

### 32.6 Tests ajoutés (§21.4)
13. Signalement protégé : **aucun** avis, relance, absence ni message vers l'organisme après l'horodatage du signalement.
14. Convention d'organisme sans une mention exigée : jamais `signe`.
15. Attestation : aucune commande ne permet à un personnel de la créer « au nom de » l'organisme.
16. Stage N+1 d'un élève non réinscrit : annulé à la date de non-réinscription, aucune situation posée.
17. Export de 201 contacts : double validation exigée.

### 32.7 Réglages et questions
Réglages **S-3915 à S-3918** (`PERSONAS_39.md` §8 bis). Questions **Q-SG34 à Q-SG40** soumises à Florian (`PERSONAS_39.md` §8.2), notamment le partage d'offres entre établissements (Q-SG34), le document expurgé (Q-SG35, juriste), l'AIPD (Q-SG36, DPO) et la **date de bascule de l'été 2027** (Q-SG38).
