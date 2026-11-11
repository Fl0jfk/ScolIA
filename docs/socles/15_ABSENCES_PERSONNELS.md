# Socle 15 — Absences des personnels et remplacements

**Date :** 2026-11-11 — **Statut :** conception de fond, **non décidée** (à relire par Florian) ; les recommandations s'appliquent par défaut (D-031), sauf ce qui est listé au §30.2. Aucun code écrit ni modifié. Aucun accès à une base, à un secret ou à un environnement de production. Le code Next.js a été **lu seulement** (inventaire ciblé du module M05, rien exécuté, rien construit, rien modifié).
**Branche :** `ent-socle-15-conception`, partie de `origin/ent-mc0hoe` (`7467f2d4`).
**Passe personas :** `docs/socles/PERSONAS_15.md` (même branche). Ses ajouts sont faits **ici**, marqués **« ajout personas »**, pour l'essentiel au **§32** ; **en cas de contradiction, le §32 l'emporte** ; rien n'a été supprimé.
**À lire avec :** `CLAUDE.md` ; `docs/DECISIONS.md` D-017 (droits portés par les actions), D-020 (personne ≠ compte), D-021 (méthode), D-022 (portée au groupe), D-023 (peu de réglages), D-024, D-025, D-026 (briques), D-029 (public), D-031, D-033, **D-034** (doublon 2027-2028), **D-035** (add-on Next.js), **D-036** (aucun JSON-mémoire), D-037 à D-046 (PRA, exigences E1–E14) ; `docs/SOCLES.md` §15 ; `docs/TRANSITION_T0_INVENTAIRE.md` (M05, §4.4, R8, Q-T0-14) ; `docs/socles/06_PERSONNELS.md` (interventions, suppléance, port `nommer_suppleant`, planning de service P15, responsable fonctionnel PRS-139) ; `docs/socles/10_EMPLOI_DU_TEMPS.md` (séance datée, modification `intervenant`, R-05, R-09, EDT-147, EDT-167, EDT-170, EDT-198, EDT-228) ; `08_NOTIFICATIONS.md`, `09_INTEROP.md`, `12` (appel, non conçu à part), `14` (sorties anticipées), `16_MESSAGERIE.md` (réponse d'absence, MSG-180), `20` (cahier de textes), `34_SANTE.md`, `42_SORTIES_VOYAGES.md` (encadrants), `IA_COEUR.md` §5–§6 ; `docs/PRA_SAUVEGARDES.md` §18.4. Modèles de forme : `10_EMPLOI_DU_TEMPS.md`, `21_EVALUATIONS.md`, `42_SORTIES_VOYAGES.md`.

**Conventions.**
- **[F]** fait vérifié dans le code. Rust : chemins depuis `ent-rust/`. Next.js : chemins depuis la racine du dépôt (`app/`, `db/`, `drizzle/`), lecture seule.
- **[V]** règle lue sur une source officielle (URL et date au §5) ; **[V*]** texte officiel lu sur une copie intégrale non officielle ; **[S]** repris d'un document du dépôt qui l'a vérifié (cité) ; **[NV]** non vérifié ; **[H]** hypothèse de terrain ; **[R]** recommandation, non décidée.

**Numérotation propre à ce socle** (vérifiée libre sur la branche de départ le 2026-11-11) :

| Élément | Numéros | Remarque |
|---|---|---|
| Fonctions existantes de Next.js | **NX15-01**… | préfixe distinct des NX du socle 42 |
| Défauts de Next.js | **AD-01**… | signalés à Florian, Next.js reste intouchable |
| Défauts de l'ENT Rust | **AR-01**… | corrigés au lot A0 ou au lot indiqué |
| Actions cibles | **ABP-100**… | RH-01 à RH-05 de `SOCLE2_DOMAINES.md` (D19) sont repris et renvoient ici |
| Règles | **R-AP1**… | |
| Cas limites | **CAP-01**… | |
| Questions | **Q-AP1**… | |
| Réglages signalés | **S-1500 à S-1529** | plage propre au socle (S-466 est le plus haut numéro pris) |
| Manques (passe personas) | **M-1501 à M-1599** | plage propre au socle (M-2301 à M-2412 sont pris par le socle 16) |

**Plage de migrations imposée : 2300–2319** (consigne du pilote, vérifiée libre sur **les 124 branches distantes** le 2026-11-11 : aucune migration 2xxx n'existe ; la plus haute écrite est **667** `667_d036_personne_fusion_normalisee`). Registre : `MIGRATIONS.md`.

**Méthode.**
1. Lecture des décisions, de la carte, de l'inventaire T0 et des conceptions citées.
2. Trois enquêtes indépendantes, par sous-agents, en lecture seule :
   - (A) inventaire fonctionnel du module Next.js M05 : environ 40 lectures sur environ 11 000 lignes (`app/api/absences/*`, `app/lib/absence*`, `app/(admin)/absences`, composants, migrations drizzle 0006, 0028–0033, 0051, 0052) ;
   - (B) état des lieux du code Rust : migrations 080, 042, 110, 171, 172, 354, 362, 377, 581, 587 ; `domain-eleve/src/absence_rh.rs` ; `ent-db` (`service.rs`, `absence_rh_repo.rs`, `edt_remplacement.rs`, `edt_masse.rs`, `edt_reactions.rs`, `registre_donnees.rs`) ; `ent-api` (routes, DTO, tests de séparation) ; écrans Leptos `remplacements` ; `ui.rs` ;
   - (C) recherche réglementaire, environ 60 lectures : fonction-publique.gouv.fr, service-public.gouv.fr, cnil.fr, senat.fr, ih2ef.gouv.fr, data.gouv.fr, msa.fr. **Légifrance et education.gouv.fr (donc le BO) ont renvoyé 403** à la session de recherche du 2026-11-11 ; les points déjà vérifiés sur Légifrance par la session du socle 10 le 2026-11-09 sont repris en [S].

Sommaire : §0 En bref · §1 Thèse · §2 Existant (Next.js, Rust) · §3 Acteurs · §4 L'année vue du terrain · §5 Réglementaire · §6 Frontières et ports · §7 Concepts · §8 Cycle de vie d'une absence · §9 Natures d'absence et circuits · §10 Impact : les séances touchées · §11 Répondre : remplacer, suppléer, annuler · §12 Remplacement de courte durée (RCD) · §13 Familles et élèves · §14 Heures et service fait · §15 Rattrapages · §16 Absences collectives, grève · §17 Personnels non enseignants · §18 Confidentialité, pièces, conservation · §19 IA · §20 Cas limites · §21 Actions métier · §22 Règles · §23 Droits · §24 Événements et notifications · §25 Écrans et mobile · §26 Modèle de données · §27 Reprise de Next.js · §28 Lots et migrations · §29 Tests · §30 Décisions et questions · §31 Faits, hypothèses, contradictions · §32 Ajout personas.

---

## 0. En bref

1. **Ce qui existe aujourd'hui est en deux morceaux qui ne se parlent pas.**
   - **Next.js (M05)** gère la vie **administrative** de l'absence : déclaration par l'agent, par un collègue, par l'accueil, import de convocations d'examen par OCR, validation par la direction, « traitement administratif » (rectorat, ONISE, comptabilité de l'OGEC), traitement des heures (rattrapage, retenue, déclaration rectorat), 16 congés exceptionnels de la convention EPNL, justificatifs, fil de messages, purge des pièces sensibles, élément de paie. Il **ne connaît ni l'emploi du temps, ni les remplaçants, ni les familles** [F].
   - **L'ENT Rust** gère la vie **organisationnelle** : absence à trois états, séances touchées, candidats remplaçants sans conflit, remplacement avec délégation d'un jour, annulation en masse, notifications aux familles **sans motif**. Il n'a **ni circuit de validation, ni pièces, ni heures** [F].
2. **Maturité du module Next.js : partielle** (l'inventaire T0 hésitait entre « partiel » et « fiable », Q-T0-14). Ses **règles métier** sont riches et en partie testées ; mais sa persistance **perd des champs** (validateur, nature « retard », auteur de la saisie), sa logique de validation est **dupliquée et divergente**, son `GET` **fusionne et supprime** des absences, et il **expose le motif médical** à l'administratif, aux surveillants et dans les mails et la paie. **Méthode retenue (D-035)** : on traite Next.js comme une **spécification fonctionnelle** ; toutes ses actions sont reprises (NX15-01 à NX15-58, §2.1), aucune ligne de code ; ses règles sont réécrites et testées.
3. **L'ENT Rust a trois fuites et pertes à corriger avant tout (lot A0)** [F] :
   - **tout le personnel lit le motif d'absence de tous ses collègues**, avec leur nom (`GET /rh/absences` n'exige que `is_staff()`, `service.rs:15371`) ; les professeurs lisent aussi le **motif libre** des exceptions d'emploi du temps (`service.rs:7937`) ;
   - **un directeur ou un administratif peut valider sa propre absence** ;
   - **la validation efface l'auteur de la déclaration** (`absence_rh.rs:66`) ; une absence ne peut être ni refusée ni annulée ; la levée d'un remplacement **supprime physiquement** l'exception.
   Vingt-trois autres défauts sont listés (AR-01 à AR-26, §2.3).
4. **Thèse (§1).** Une absence de personnel est **trois choses à droits différents** : un **fait RH confidentiel** (nature, pièces, décision), une **conséquence publique sans motif** (des séances et des postes à couvrir), une **réponse d'organisation** (remplacer, suppléer, déplacer, annuler, accueillir les élèves). S'y ajoute un **fait de service constaté** : les heures réellement faites par le remplaçant et celles que l'absent doit, qui vont à la paie ou au rectorat.
5. **Le réglementaire change la conception** (§5) :
   - le **décret 2023-732** (plan annuel de remplacement de courte durée, volontaires sur créneaux fixes d'au moins une heure, refus pour motif légitime seulement) [S, V] et l'**arrêté du 14/08/2023** (remontée **hebdomadaire par API**, depuis le logiciel de vie scolaire, des heures prévues, absentes, remplacées, avec un **motif catégorisé**) [S] ; remontée étendue au **privé sous contrat depuis la rentrée 2024** (décret 2023-738, C. éduc. R442-39) [V] ;
   - la **circulaire 96-248** : toute modification prévisible des horaires d'entrée ou de sortie due à l'absence d'un enseignant est **portée à la connaissance des familles** ; sans information, l'élève est gardé aux horaires habituels ; sortie anticipée seulement si le règlement intérieur et les parents l'autorisent [V*] ;
   - le **référentiel CNIL de gestion du personnel** (2019-160) : aucun diagnostic ; motif limité, accès réservé aux gestionnaires [V] ; arrêt transmis sous 48 h, volet 1 gardé par l'agent [V] ; ASA réformées au **1er janvier 2027** (décret 2026-604) [V].
6. **Le plan.**

   | Élément | Volume | Numéros |
   |---|---|---|
   | Fonctions de Next.js reprises | 58 | NX15-01 à NX15-58 |
   | Défauts de Next.js | 16 | AD-01 à AD-16 |
   | Défauts de l'ENT Rust | 26 | AR-01 à AR-26 |
   | Actions cibles | 172 | ABP-100 à ABP-271 |
   | Cas limites | 72 | CAP-01 à CAP-72 |
   | Règles | 44 | R-AP1 à R-AP44 |
   | Réglages signalés | 18 | S-1500 à S-1517 |
   | Lots | 16 | A0 à A15 |

   - **Lot A0** (correctifs urgents de l'existant, migration 2300) : peut partir tout de suite.
   - **Parcours minimal pour la rentrée 2027** (usage réel, D-035 point 2 : le module remplace Next.js) : A0, A1, A2, A3, A4, A7, A8, A10, A11 (sous-lots a à d).
7. **Décisions appliquées par défaut (D-031)** : 28 questions tranchées selon la recommandation (§30.1). **Soumis à Florian** (§30.2) : les défauts de Next.js **en production** (motif médical lisible par les surveillants, auto-validation, URL libre de justificatif) ; qui organise les remplacements aujourd'hui dans son lycée et ce qui devient réel pendant le doublon ; le circuit des maîtres contractuels vers le rectorat et l'outil académique ; les durées de conservation (DPO) ; l'accès à la spécification de l'API SI-RCD ; la confirmation de la convention EPNL (maintien de salaire, congés exceptionnels) par le juriste ou la comptable de l'OGEC.

8. **Ajout personas (§32, `PERSONAS_15.md`)** : quatre bloquants tranchés par défaut — (A) **aucun effet vers les familles ni sortie anticipée sans « EDT de référence »** posé par la direction (pendant le doublon, l'EDT officiel peut rester celui de Charlemagne) ; (B) **un signalement non confirmé** (appel à l'accueil) n'a aucun effet extérieur, contre le canular d'un élève ; (C) **circuit externe** : pour un fonctionnaire, un maître ou une AESH, l'établissement prend en compte et transmet, l'employeur décide ; (D) la **présence** d'un collègue n'est visible que de qui en a besoin. Plus : échanges et avances de séances, sortie calculée par élève (car scolaire, internat), une réception par arrêt, compteurs annuels, clôture des variables de paie, silence pendant l'absence, anti-inférence de la nature. Totaux après la passe : **200 actions** (ABP-100 à ABP-299), **59 règles**, **90 cas limites**, migrations **2300–2313, 2318, 2319**.

---

## 1. Thèse : une absence est un fait confidentiel aux conséquences publiques

1. **Trois objets, pas un.**
   - (a) **L'absence** : un fait RH daté, rattaché à un **agent** (socle 6), avec une **nature** prise dans un référentiel (maladie, enfant malade, formation, convocation, ASA, grève, convenance…), des pièces, une décision. **Confidentielle** : la nature n'est lue que par ceux qui instruisent ou gèrent la paie ; le motif détaillé n'existe pas (aucun diagnostic, jamais).
   - (b) **L'impact** : ce que l'absence enlève à l'établissement, calculé **automatiquement** : les **séances** de l'emploi du temps (socle 10) où l'agent était en saisie, les **postes de service** du planning des non-enseignants (socle 6, P15), les **engagements** ailleurs (encadrement de sortie au socle 42, surveillance d'examen au 38, conseil de classe au 23, rendez-vous au 19). Chaque élément d'impact a un **état de traitement**. **Public sans motif** pour ceux qui organisent la journée.
   - (c) **La réponse** : pour chaque élément d'impact, une décision d'organisation : remplacer la séance (un collègue, le RCD), confier le service à un suppléant (socle 6), changer le cours, déplacer, annuler et accueillir les élèves (permanence, étude, CDI, sortie anticipée selon régime), proposer une séquence de continuité numérique. Elle produit les **modifications de séance** du socle 10, qui informent familles et élèves.
   - (d) S'y ajoute **le service fait** : chaque séance réellement assurée par un remplaçant, chaque heure de rattrapage due puis faite, chaque retenue à signaler. **Constaté**, pas déclaré : il se lit sur les séances assurées (appel fait, socle 12), se conteste, se valide en état mensuel, s'exporte vers la paie (socle 49) ou le rectorat.
2. **Le motif ne circule jamais.** Les familles et les élèves apprennent qu'un cours n'a pas lieu, qu'il est remplacé, par qui, ou qu'il est déplacé ; jamais pourquoi, jamais le mot « malade ». Les collègues voient « absent » (s'ils doivent le savoir), jamais la nature. L'accueil sait si quelqu'un est là, pas pourquoi. Un événement, un mail, une notification, une ligne de paie, un export ne portent **jamais** le libellé de la nature quand elle est sensible (R-AP2).
3. **Le circuit dépend de l'employeur, pas du lieu.** Un **maître contractuel** du privé ou un **fonctionnaire** du public est un agent de l'État : son absence se déclare à l'établissement, qui la transmet au rectorat ; l'établissement ne paie pas. Un **salarié de l'OGEC** (surveillant, secrétaire, agent, comptable) est un salarié de droit privé : son absence alimente la paie de l'OGEC, la DSN, la convention EPNL. Le même lycée a les deux ; la même personne peut avoir les deux engagements (maître + surveillances payées par l'OGEC, cas P47 du socle 6). Le circuit est porté par l'**engagement** (socle 6), pas par le rôle.
4. **De l'absence à la famille prévenue sans saisie intermédiaire** (ligne « Faire mieux » de la carte). L'enseignante déclare à 7 h 05 depuis son téléphone ; à 7 h 06 l'adjoint voit les quatre séances du jour, deux sont déjà couvertes (une volontaire RCD libre, un collègue de la discipline libre), une est déplaçable, la dernière tombe en fin de journée pour une classe de 4ᵉ dont les externes ont l'autorisation de sortir ; il valide les quatre propositions d'un geste ; les familles concernées et les élèves sont prévenus **avant** l'heure qui change, sans motif ; la vie scolaire voit l'effectif attendu en permanence ; le remplaçant accuse réception ; le soir, ses heures sont dans son relevé.
5. **Tout est daté, y compris le futur, et rien ne s'efface.** Une absence prévue (formation, convocation, congé) se saisit à l'avance ; une absence se prolonge, s'écourte (reprise anticipée), se requalifie, s'annule — chaque fois par un **fait** ajouté, jamais par une réécriture. Une séance remplacée puis rétablie garde l'historique de la réponse.
6. **On ne note pas les gens.** Pas de « palmarès » des absents, pas de score d'absentéisme individuel, pas de « facteur de Bradford ». La direction voit, dans le cadre de sa gestion, l'historique d'un agent ; les indicateurs du pilotage sont **agrégés** (heures non assurées par discipline, taux de remplacement). Le choix d'un remplaçant n'utilise **aucun** historique d'absence ni trait personnel (règlement européen sur l'IA, annexe III 4 b, [S] R-26 du socle 10).

**Ce qui fera la différence au quotidien [H]** : la déclaration en trente secondes depuis le téléphone, sans motif à écrire ; le tableau **« ce matin »** de l'adjoint, avec les propositions déjà prêtes ; l'acceptation du remplacement par le remplaçant d'un geste, avec la liste d'appel et le travail du jour ; les familles prévenues juste à temps, une seule fois ; la permanence qui connaît ses effectifs ; le **relevé d'heures** que personne n'a ressaisi ; la remontée RCD qui part seule le lundi ; le motif qui n'apparaît jamais là où il ne doit pas.

---

## 2. Évaluation franche de l'existant

### 2.1 Ce que fait Next.js (M05) : fonctions reprises [F]

Source : inventaire en lecture seule (enquête A). Fichiers principaux : `app/api/absences/route.ts` (1 315 lignes : GET, POST, PATCH à 13 actions, DELETE), sous-routes `manual`, `ingest`, `document-url`, `delete-document`, `processors`, `access-check`, `directory-users` ; `app/lib/absences-types.ts`, `absence-hours-treatment.ts` (857 lignes), `absence-db.ts`, `absences-storage.ts`, `absences-routing.ts`, `absences-ogec-validators*.ts`, `absences-validation-recipients.ts`, `absences-workflow-mail.ts`, `absences-paie-sync*.ts`, `absence-ingest-process.ts`, `absences-privacy.ts`, `absences-dedup.ts`, `absences-leave-spans.ts`, `absences-suivi.ts` ; interface `app/(admin)/absences/AbsencesPageClient.tsx` (1 998 lignes, redirigée vers `/rh?tab=dashboard&section=absences`) ; schéma `db/schema-ent-relational.ts:44-151`, `db/schema-socle-faits.ts:577-631`. Environ 70 tests unitaires (`node:test`) sur les fonctions pures.

| N° | Fonction | Où [F] | Reprise (action cible) |
|---|---|---|---|
| NX15-01 | L'agent déclare sa propre absence (`source=self`) | `route.ts` POST l.206-468 | ABP-100 |
| NX15-02 | Un administratif, la comptabilité, la direction déclarent pour un collègue, cherché dans l'annuaire | `canDeclareAbsenceOnBehalf` l.226 ; `directory-users` | ABP-101 |
| NX15-03 | L'accueil saisit une absence signalée (téléphone) | `accueil-absences-db.ts` ; `source=accueil` | ABP-102 |
| NX15-04 | Saisie manuelle d'une absence déjà validée par un administratif | `manual/route.ts` ; `buildAdminAbsenceRecord` l.551 | ABP-104 |
| NX15-05 | Import d'un **PDF de convocation d'examen** : OCR, extraction des créneaux, une absence par jour | `absence-ingest-process.ts` ; Mistral OCR et `mistral-small-latest` | ABP-105, ABP-248 |
| NX15-06 | Suivi du travail d'import (en attente, en cours, terminé, échec ; reprise après 90 s) | `ingest/status`, `tryClaimIngestJob` | ABP-105 |
| NX15-07 | Fusion des créneaux d'une convocation à un par jour, heure de Paris | `mergeOneSlotPerCalendarDay` | R-AP31 |
| NX15-08 | Catégorie « professeur » ou « OGEC » ; double casquette : l'agent choisit | `getRoleFlags`, `resolveSelfDeclarationScope` l.157 | ABP-106 (par engagement, R-AP4) |
| NX15-09 | Journée unique ou plusieurs jours ; heures de début et de fin ; 8 h–18 h par défaut | `computeStartEndAt` l.448 | ABP-100, R-AP6 |
| NX15-10 | Motif libre ou normalisé ; détails | `reason`, `details` | **remplacé** par la nature (R-AP1) |
| NX15-11 | **Préférence de traitement** exprimée par le déclarant (rattrapage, retenue, déclaration…) | `staff_preferred_treatment` | ABP-107 |
| NX15-12 | Créneaux de rattrapage proposés par le déclarant, confirmés par la direction | `AbsenceMakeupSlotsEditor`, `direction_confirmed_makeup_slots` | ABP-190 à ABP-193 |
| NX15-13 | Détection d'un doublon (même personne, chevauchement) avant création | `absences-dedup.ts` | ABP-108 (proposition, jamais fusion silencieuse, AD-06) |
| NX15-14 | Routage du valideur : **fiche RH nominative** d'abord, puis liste paramétrée, puis directeur, puis rôle de direction du cycle | `absences-routing.ts:115-252` | ABP-120, R-AP9 |
| NX15-15 | Valideurs distincts par cycle (école, collège, lycée) pour les professeurs | `absencesValidatorsProf{Ecole|College|Lycee}` | ABP-260 |
| NX15-16 | « Processeurs » de traitement administratif : secrétariat (rectorat, ONISE) pour les professeurs, comptabilité pour l'OGEC, responsable de surveillance pour un surveillant | `absences-validation-recipients.ts:122` | ABP-125, ABP-261 |
| NX15-17 | **Valider** avec un traitement des heures obligatoire | PATCH `VALIDER` ; `validateHoursTreatmentForAbsence` | ABP-121 |
| NX15-18 | **Refuser** ; refus **interdit** pour un motif non discrétionnaire (arrêt de travail, enfant malade, congé exceptionnel) | PATCH `REFUSER` | ABP-122, R-AP12 |
| NX15-19 | Traitement **forcé** pour un motif non discrétionnaire | `nonDiscretionaryTreatmentFromReason` l.250 | R-AP12 |
| NX15-20 | **Clôture automatique** après validation si sans incidence ou rattrapage interne | `autoClosesAfterDirectionValidation` l.510 | R-AP13 |
| NX15-21 | Traitement administratif (par le processeur, après validation) puis clôture | PATCH `TRAITER_ADMIN` | ABP-125 |
| NX15-22 | Clôturer, interdit après validation sans traitement administratif | PATCH `CLOTURER` | ABP-126 |
| NX15-23 | Rouvrir | PATCH `REOUVRIR` | ABP-127 (avec effets défaits, AD-10) |
| NX15-24 | Déposer un justificatif | PATCH `DEPOSER_JUSTIFICATIF` | ABP-140 |
| NX15-25 | Relancer le justificatif | `RELANCER_JUSTIFICATIF` | ABP-141 |
| NX15-26 | Relancer, puis renseigner les créneaux de rattrapage | `RELANCER_CRENEAUX_RATTRAPAGE`, `RENSEIGNER_CRENEAUX_RATTRAPAGE` | ABP-191, ABP-192 |
| NX15-27 | Relance automatique des créneaux si le traitement est un rattrapage sans créneau | `needsMakeupSlotsFromStaff` l.743 | R-AP27 |
| NX15-28 | Corriger la catégorie (professeur / OGEC) | `CORRIGER_SCOPE` | ABP-129 |
| NX15-29 | **Reclasser en arrêt maladie** (passe validée) | `RECLASSER_ARRET_MALADIE` | ABP-128 |
| NX15-30 | Modifier les dates et heures (« calendrier ») | `MODIFIER_CALENDRIER` | ABP-110, ABP-111 |
| NX15-31 | **Fil de messages** en ajout seul entre déclarant, direction, traitement administratif (4 000 car.) | `absence_message` (0051) ; `POST_MESSAGE` | ABP-145 |
| NX15-32 | Historique des actions (création, décision, traitement, relances, RGPD, corrections…) | `absence_history` | journal en ajout seul (R-AP20) |
| NX15-33 | Visibilité au calendrier (`calendar_visible`), trous dans le planning de l'agent | `absences-leave-spans.ts:85` ; `mon-planning` | ABP-150 (impact) |
| NX15-34 | Motif masqué au calendrier (`getPublicAbsenceReason`) | | R-AP2 |
| NX15-35 | **Retard** (≤ 4 h, même jour) traité sans incidence | `isRhRetardAbsence` l.382, `RH_RETARD_MAX_DURATION_MINUTES = 240` | ABP-109, R-AP14 |
| NX15-36 | Options de traitement selon catégorie et cycle : OGEC (rattrapage, retenue sur salaire) ; professeur (rattrapage interne, déclaration ONISE ou rectorat) ; communs (sans incidence, maladie, enfant malade, congé exceptionnel) | `getHoursTreatmentOptions` l.408 | §9.3 |
| NX15-37 | Préremplissage du traitement depuis la préférence | `suggestHoursTreatmentFromPreference` l.618 | ABP-121 |
| NX15-38 | Passage obligatoire au traitement administratif selon le traitement | `requiresProcessorAfterValidation` l.808 | R-AP13 |
| NX15-39 | **16 congés exceptionnels** de la convention EPNL et du code du travail, avec jours ouvrables suggérés | `absence-hours-treatment.ts:88-185`, `addOuvrableDays` l.205 | référentiel des natures (§9.2), Q-AP9 |
| NX15-40 | Élément de paie créé à la validation (nature absence, quantité estimée) | `absences-paie-sync.ts:94` | ABP-207 (sans libellé sensible, AD-03) |
| NX15-41 | Quantité d'heures estimée (écart horaire, plafond 7 h par jour) | `estimateAbsenceQuantiteHours` l.71 | **remplacé** par l'impact réel (§14.4) |
| NX15-42 | Rattachement de l'absence au personnel de la paie (identifiant, compte, e-mail) | | port socle 6 `agent_du_compte` |
| NX15-43 | Dépôt de pièces (URL pré-signée), PDF 15 Mo | `/api/travels/upload` réutilisé | ABP-140 (ent-object, E5) |
| NX15-44 | Lecture d'une pièce par URL signée de 10 minutes | `document-url/route.ts` | ABP-142 |
| NX15-45 | Suppression d'une pièce | `delete-document` | ABP-143 (état daté, pas de suppression avant purge) |
| NX15-46 | Qui voit les pièces : OGEC → comptabilité et direction ; professeur → administratif et direction ; toujours le déclarant | `absences-types.ts:265-299` | §23.2 |
| NX15-47 | **Purge** après traitement : pièces sensibles supprimées, détails vidés, motif remplacé par « Absence » | `applyPostValidationPrivacy`, `absences-storage.ts:203` | ABP-160, R-AP21 |
| NX15-48 | Détection de contenus sensibles par expressions | `absences-privacy.ts:3-30` | **remplacé** : la sensibilité est portée par la nature (R-AP1) |
| NX15-49 | Mail aux valideurs à la création | `notifyAbsenceCreated` l.103 | ABP-232 |
| NX15-50 | Mail aux processeurs à la validation (avec pièces jointes) | `notifyAbsenceValidated` l.159 | ABP-233 (**sans** pièce jointe, AD-04) |
| NX15-51 | Mail au déclarant : validée, justificatif demandé, traitée | | ABP-234 |
| NX15-52 | Mails des créneaux de rattrapage, du justificatif déposé, du fil | | ABP-235 |
| NX15-53 | File d'attente « à valider » de la direction, « à traiter » des processeurs | `absences-admin-access.ts` | ABP-170, ABP-171 |
| NX15-54 | Suivi par étapes pour l'interface (prochaine étape, qui doit agir) | `absences-suivi.ts` | ABP-172 |
| NX15-55 | Calendrier des absences | `absences-calendar.ts` | ABP-173 |
| NX15-56 | Compteur « professeur absent » au tableau de bord | `lib/dashboard-signals.ts:1037` | ABP-174 |
| NX15-57 | Assistant IA : outils de lecture et de décision sur les absences | `brain-ai/tools/handlers/rh-absences.ts` | §19 (décision **retirée** à l'IA) |
| NX15-58 | Configuration des valideurs et processeurs | `processors/route.ts` | ABP-260, ABP-261 |

**Ce que Next.js ne fait pas [F]** : aucun lien avec l'emploi du temps (pas de séances touchées), aucun remplaçant, aucune information des élèves ni des familles, aucune heure de remplacement, aucun plan RCD ni remontée nationale, aucune absence d'agent sans compte, aucune notification dans l'application, aucun mail en cas de refus, aucun décompte annuel, pas de DSN.

### 2.2 Défauts de Next.js (à ne pas reproduire ; signalés à Florian, Next.js reste intouchable)

| N° | Gravité | Défaut [F] | Où | Correctif dans ScolIA |
|---|---|---|---|---|
| AD-01 | **critique** | **URL de justificatif libre** fournie par le client, puis récupérée par le serveur (`fetch`) et jointe au mail ; `document-url` renvoie l'URL brute si elle ne correspond à aucune clé (risque SSRF ou exfiltration, exploitabilité à vérifier) | `travels-s3.ts:382` ; `document-url/route.ts:52` | pièces par `ent-object` seulement (clé produite par le serveur, E5) ; jamais d'URL fournie |
| AD-02 | **critique** | **Auto-validation** : le valideur peut valider sa propre absence (seules la file et l'outil IA l'excluent) | `route.ts:551-561` | R-AP10, contrôle en base |
| AD-03 | **critique** | **Motif médical exposé** : renvoyé en clair par le `GET` à l'administratif et à **tout surveillant ou CPE** pour une absence de professeur ; en clair dans tous les mails ; en clair dans `paie_element.libelle`, jamais purgé | `route.ts:388` ; `absences-workflow-mail.ts` ; `absences-paie-sync.ts:60` | R-AP1, R-AP2 : nature catégorisée, projection sans nature hors instructeurs |
| AD-04 | important | Pièces jointes PDF envoyées par mail aux processeurs | `loadAbsenceValidationAttachments` | lien vers ScolIA seulement (R-AP22) |
| AD-05 | important | Les surveillants et CPE peuvent faire des saisies manuelles et des imports d'absences **déjà validées** (`canAdminIngest = canViewCalendar`) | `route.ts:209-223` | droit `personnels.absences.saisir_validee` réservé (§23) |
| AD-06 | important | Le `GET` **fusionne et supprime** des absences en attente sur un critère nom + chevauchement | `consolidatePendingAbsencesInIndex` | aucune écriture en lecture ; doublon = proposition (ABP-108) |
| AD-07 | important | **Champs perdus à l'enregistrement** : validateur OGEC, nature de la déclaration (« retard »), code de congé exceptionnel, auteur de la saisie (`submittedBy`) ; conséquence : celui qui a saisi pour un collègue perd ses droits, le « retard » saisi par l'accueil disparaît | `absence-db.ts:24-90` | modèle en colonnes typées, test aller-retour (§29) |
| AD-08 | important | Logique de validation **en double** (route et IA) et déjà **divergente** : la synchronisation paie manque côté IA | `absences-manager-decision.ts` | une seule commande serveur, appelée par l'écran et l'IA (IA_COEUR §3) |
| AD-09 | important | Configuration des processeurs (qui reçoivent les pièces médicales) ouverte à tout rôle contenant « direction », quel que soit le cycle | `viewerCanConfigureAbsenceProcessors` | droit `personnels.absences.parametrer` (§23) |
| AD-10 | important | `REOUVRIR` ne défait ni le traitement, ni la clôture administrative, ni l'élément de paie ; le motif déjà anonymisé est perdu ; changer les dates après validation ne met pas la paie à jour | `route.ts` PATCH | R-AP17 : toute correction recalcule ses effets |
| AD-11 | important | Élément de paie **ignoré** sans période ouverte, jamais rejoué ; pas d'unicité en base → doublons possibles | `absences-paie-sync.ts` ; `paie_element.absence_id` | R-AP29 : file rejouable, unicité |
| AD-12 | moyen | Fuseau du serveur pour « aujourd'hui » et pour l'estimation des heures ; week-ends comptés | `route.ts:80` ; `estimateAbsenceQuantiteHours` | fuseau de l'établissement (R-P2 du socle 6) |
| AD-13 | moyen | Identité incohérente : le `GET` sous « vue effective », le reste sous l'utilisateur réel | | une identité par requête (socle 1) |
| AD-14 | moyen | `managerNote` écrasée à chaque modification, même par le déclarant qui dépose une pièce | `route.ts:564-567` | note de décision attachée à la décision (en ajout seul) |
| AD-15 | moyen | Historique **réécrit** à chaque enregistrement (suppression puis insertion) | `absence-db.ts:347-357` | journal en ajout seul |
| AD-16 | faible | Scripts ad hoc avec des noms de personnes en dur ; pont Charlemagne et purge des absences expirées vides | `scripts/fix-ogec-absence-validator-colas.mjs` ; `absences-sync/port.ts` ; `purgeExpiredAbsences` | — |

### 2.3 Défauts de l'ENT Rust (lot A0 sauf indication)

Source : enquête B. Tables : `ent_absence_rh` (080 ; 362 ajoute `agent_id` ; 377 l'unicité composite ; RLS 581), `ent_edt_exception` (042, 171), `ent_edt_seance` (110, 362), `ent_delegation` (172, 560), `ent_service_intervenant` (354). Code : `domain-eleve/src/absence_rh.rs`, `ent-db/src/service.rs:15236-15371`, `edt_remplacement.rs`, `edt_masse.rs`, `edt_reactions.rs`.

| N° | Gravité | Défaut [F] | Où | Lot |
|---|---|---|---|---|
| AR-01 | **fuite** | Le **motif** de toute absence de personnel est lisible par **tout le personnel** (professeurs, accueil, comptabilité, infirmerie, psychologue…), avec le nom | `service.rs:15371` (`is_staff`) ; `dto.rs:3350-3358` ; `routes.rs:6323` ; affiché par `ui.rs:39888, 40633, 41113, 48594` | A0 |
| AR-02 | **fuite** | Le **motif libre** des exceptions d'EDT (souvent la raison de l'absence) est lisible par les professeurs | `service.rs:7937` (`can_faire_appel`) ; `dto.rs:913` ; motif obligatoire `domain-eleve/src/edt.rs:573-575` | A0 (déjà relevé D3 au socle 10, T0) |
| AR-03 | **fuite** | Motif en clair, table classée dans le domaine « assiduité » des élèves, **sans** marque santé ; aucune règle de conservation | `registre_donnees.rs:238` | A0 |
| AR-04 | fuite | Les familles voient l'**e-mail** du remplaçant quand il n'a pas de nom affiché | `edt_masse.rs:313` ; `edt_admin.rs:950` | A0 |
| AR-05 | [H] | RLS sans effet sur ces chemins (mode `off` par défaut, requêtes hors `TenantTx`) ; l'isolation tient par les filtres applicatifs, présents partout | `tenant_tx.rs:7,30` ; `service.rs:15255,15322` ; `edt_remplacement.rs:808…` | A14 (avec le chantier RLS général) |
| AR-06 | **contrôle** | **Auto-validation** possible | `service.rs:15310` ; `absence_rh.rs:57-69` | A0 |
| AR-07 | contrôle | Aucune action D-017 propre : valider découle de `eleves.gerer` (gestion des **élèves**) | `service.rs:15310` ; `roles.rs:75-79` | A0 |
| AR-08 | contrôle | Route historique `POST /edt/exceptions` (remplacement) : **aucun contrôle de disponibilité ni de rôle** ; un compte déjà en cours, absent, voire parent ou élève membre, peut être désigné et reçoit « Vous assurez le cours » ; pas de délégation, pas de verrou | `service.rs:7564-7650` ; `edt_reactions.rs:213-235` | A0 |
| AR-09 | contrôle | Concurrence : disponibilité vérifiée **hors transaction**, verrou par séance et non par remplaçant ; deux remplacements simultanés peuvent affecter la même personne à la même heure | `edt_remplacement.rs:791-809` | A0 |
| AR-10 | contrôle | Un cours **déplacé** vers ce jour ne compte comme occupation que pour son titulaire : son remplaçant paraît libre | `edt_remplacement.rs:254-266` | A4 |
| AR-11 | contrôle | `edt_annuler_seance` accepte une absence **non validée** ou d'un autre enseignant | `edt_remplacement.rs:940-952` | A0 |
| AR-12 | contrôle | Aucune détection de chevauchement entre deux absences d'une même personne | `service.rs:15236-15253` | A1 |
| AR-13 | **perte** | La validation **écrase l'auteur** de la déclaration ; ni valideur ni date de validation | `absence_rh.rs:66` ; `service.rs:15327` | A0 |
| AR-14 | perte | L'UPSERT des exceptions réécrit auteur et date ; la route historique remet `absence_rh_id` et `lot_id` à NULL (lien absence → séance perdu) | `edt_remplacement.rs:440-444` ; `service.rs:7603-7607` | A0 |
| AR-15 | perte | **Suppression physique** à la levée d'une exception (exception, déplacement, séance déplacée) | `edt_remplacement.rs:1159, 1170, 1180` | A0 (état « levée » ; la refonte de la modification est au socle 10, T2) |
| AR-16 | perte | Ni refus ni annulation : une absence saisie par erreur ne peut être retirée ; elle bloque la personne comme remplaçante (une absence seulement **signalée** compte) | `absence_rh.rs` ; `edt_remplacement.rs:299-307` | A0 |
| AR-17 | traçabilité | Aucun journal d'audit : seuls les événements font trace | | A0 |
| AR-18 | incohérence | La route historique ne révoque pas la délégation d'un remplacement qu'elle écrase et ne synchronise pas la séance : le remplaçant précédent garde son droit pour la journée | `service.rs:7564-7650` | A0 |
| AR-19 | incohérence | `agent_id` maintenu mais jamais lu ; la fusion d'agents ne met à jour que lui ; un intervenant sans compte ne peut être ni absent ni remplaçant | `362` ; `personnels_fiche.rs:1217-1231` | A1 |
| AR-20 | dette | Clés étrangères simples encore actives (`user_id`, `created_by_user_id`, `creneau_id`, `remplacant_user_id`) ; composites de 362 et 377 `NOT VALID` | 080, 042, 171, 362, 377 | A1 (validation après contrôle des données) |
| AR-21 | incohérence | L'écran Leptos affiche « Valider » à qui a `edt.gerer` ; le serveur exige `eleves.gerer` : le CPE reçoit un 403 | `web/.../remplacements/mod.rs:153, 398` | A0 |
| AR-22 | incohérence | Le tableau de bord historique envoie `?statut=a_valider`, ignoré : le compteur ne porte que sur le jour | `ui.rs:27806` ; `routes.rs:6317-6334` | A0 (paramètre traité) |
| AR-23 | incohérence | Deux notions de remplacement sans lien : la **suppléance** de service (354) et l'**exception** de séance (171) ; une absence validée ne bascule pas la saisie au suppléant | | A5 |
| AR-24 | dette | Code mort `role_can_declare_absence_staff` ; absences RH rangées avec l'assiduité des élèves | `service.rs:6164` | A0 |
| AR-25 | manque | Pas d'acceptation par le remplaçant, pas d'heures, pas d'écran de déclaration ; absences tronquées à 62 jours | `edt_remplacement.rs:593` | A3, A4, A7, A11 |
| AR-26 | mineur | Tri des candidats écrit avec des tuples croisés (résultat correct) | `edt_remplacement.rs:704-710` | A4 |

**À garder** [F] : l'outbox dans la transaction et l'événement **sans motif** ; le calcul des séances touchées ; le filtre `peut_remplacer` (pas le titulaire, pas absent, pas de chevauchement, pas d'indisponibilité) ; le verrou consultatif ; la délégation datée **d'un jour** et sa révocation ; l'annulation en masse avec aperçu et **motif public** ; le consommateur `d4.notifications_edt` qui relit l'état courant et n'envoie jamais de motif libre ; les tests de séparation entre établissements (`separation.rs:675, 1648, 1865`, RS3-04).

### 2.4 Ce qu'il ne faut pas perdre (D-035, point 5)

- **Les règles métier de Next.js** : la matrice des traitements par catégorie et cycle ; le retard de 4 h au plus sans incidence ; l'interdiction de refuser un motif non discrétionnaire ; la clôture automatique ; les 16 congés exceptionnels avec leurs jours ouvrables ; la relance des créneaux de rattrapage ; le routage valideur → processeur avec priorité à la fiche RH ; la purge après clôture ; la fusion des créneaux d'une convocation par jour ; le fil de messages.
- **Les gestes du quotidien** : l'agent qui déclare seul ; l'accueil qui saisit l'appel de 7 h 30 ; l'administratif qui importe une convocation ; la comptable qui voit « à traiter ».
- **Les données en cours** au moment de la bascule (absences de l'année en cours et de la précédente : droits, paie, contentieux possibles), à reprendre si Florian confirme qu'elles sont réelles (Q-T0-14, §27).
- **Côté Rust** : tout ce qui est listé « à garder » au §2.3 et les tests S2-5.

### 2.5 Verdict

On **réécrit** le domaine dans une brique `absences_personnels` (D-026), on **corrige d'abord** l'existant Rust (A0), on **reprend** les règles de Next.js sous forme de référentiel et de commandes testées, et on **branche** ce que ni l'un ni l'autre ne fait : séances, remplaçants, familles, heures, RCD, rectorat, paie.

---

## 3. Acteurs, besoins réels, irritants

| Acteur | Ce qu'il fait | Ce qui lui fait perdre du temps aujourd'hui [H] | Ce qu'il redoute | Ce qui le ravirait |
|---|---|---|---|---|
| **Professeur absent** | Prévient (souvent à 6 h 45, malade), envoie l'arrêt, laisse du travail, revient | Prévenir trois personnes (accueil, vie scolaire, direction) ; écrire son motif dans un mail lu par tous ; renvoyer l'arrêt deux fois ; ne pas savoir si sa classe a été prise | Que ses collègues ou les familles sachent pourquoi ; être soupçonné ; trouver le chaos au retour | Déclarer en trente secondes sans motif écrit ; déposer le travail du jour en une phrase ; voir au retour ce qui a été fait (séances assurées, travail donné, incidents) |
| **Professeur remplaçant (collègue, volontaire RCD)** | Accepte ou non, fait cours, fait l'appel, est payé | Apprendre le remplacement en salle des professeurs ; ne pas avoir la liste ni le travail ; heures oubliées en fin de mois | Être désigné sur son heure libre sans délai de prévenance ; ne pas être payé ; être sollicité toujours le même | Une demande sur son téléphone avec l'heure, la salle, la classe, le travail prévu ; accepter d'un geste ; ses heures comptées seules ; une répartition juste |
| **Suppléant (absence longue : maître délégué, contractuel, TZR)** | Arrive pour des semaines ; prend la classe | Arriver sans accès, sans progression, sans connaître les élèves à besoins particuliers | Être perdu, ne pas savoir où en était le titulaire | Tout hérité du service (socle 6, P5) : cahier, notes, progression ; accès le jour de la prise de poste |
| **Adjoint de direction / chargé des remplacements** | Organise chaque matin, tient le plan RCD, rend compte | Tableaux Excel, coups de téléphone, sollicitations répétées, comptes en fin de mois | Une classe sans adulte ; une famille non prévenue ; un remplaçant déjà en cours | Le tableau « ce matin » avec des propositions prêtes et explicables ; un geste pour tout valider ; l'état RCD qui se remplit seul |
| **Chef d'établissement** | Valide les absences, décide, signe les états, présente le plan au CA | Valider des absences au fil de l'eau sans contexte ; ressaisir pour le rectorat | Une erreur sur un arrêt (sanction de l'agent, contentieux) ; un parent qui se plaint d'heures perdues | Une file claire, des pièces au bon endroit, des indicateurs agrégés pour le CA et le rectorat |
| **Secrétariat de direction / gestion des personnels** | Transmet au rectorat (maîtres, fonctionnaires), tient les états d'heures | Ressaisir dans l'outil académique, chercher les pièces dans les mails | Oublier un délai (arrêt sous 48 h, état mensuel) | Un « à transmettre » daté, un export au format attendu, une trace de transmission |
| **Comptable de l'OGEC** | Paie des salariés de droit privé, DSN, maintien de salaire, congés | Recevoir l'information en retard ou incomplète ; libellés à recoder | Une DSN d'arrêt hors délai (5 jours) ; une retenue injustifiée | Une file « paie » propre : nature, jours, heures, pièces, échéance DSN |
| **CPE, vie scolaire, AED** | Accueille les élèves sans cours, tient la permanence, prévient | Découvrir la classe dans le couloir ; listes de permanence au crayon | Un élève mineur sorti sans autorisation ; une classe sans surveillance | Effectifs attendus par heure, liste d'appel de permanence, sorties anticipées calculées selon régime et autorisation |
| **Accueil** | Reçoit l'appel du professeur, oriente | Transmettre par post-it ; ne pas savoir qui est déjà au courant | Mal noter ; dire le motif à un parent qui appelle | Saisir en trois champs, sans motif, transmis à qui doit savoir ; répondre « M. X est absent aujourd'hui » quand c'est permis |
| **Professeur principal** | Répond aux familles de sa classe | Apprendre les absences de ses collègues par les familles | Être interrogé sans savoir | Le résumé du jour des séances non assurées de sa classe (déjà au socle 10, §14.4 point 5) |
| **Familles (dont séparées)** | S'organisent (garde, transport, repas) | Apprendre la veille au soir ou pas du tout ; un seul parent prévenu | Que l'enfant sorte sans qu'elles le sachent ; des heures perdues non rattrapées | Une notification juste à temps, une fois, aux deux parents ayant droit ; l'état du remplacement pour une absence longue |
| **Élèves mineurs, majeurs, internes** | Vont en permanence, au CDI, sortent selon le régime | Attendre devant une salle vide | Être sanctionnés pour être sortis | Savoir avant le cours ce qui se passe et où aller |
| **AESH** | Accompagne un élève ; peut elle-même être absente | Son absence n'est connue de personne qui s'occupe de l'élève | L'élève seul en cours | Son absence prévient la vie scolaire et le professeur concerné, sans motif ; la famille de l'élève accompagné prévenue selon le PAP/PPS |
| **Agents, personnels administratifs (OGEC)** | Absents eux aussi : leur poste de service doit être couvert | Aucun outil : le planning tient sur papier | Ne pas être remplacé à la loge, à la cantine | Le poste à couvrir apparaît dans le planning de service (socle 6, P15) |
| **Rectorat, DSDEN, référent RCD** | Reçoit les absences, les états d'heures, la remontée RCD | Données hétérogènes | Remontée absente ou fausse | Remontée automatique au format, aucun nom dans les agrégats |
| **DPO, auditeur** | Vérifie minimisation, accès, conservation | Motifs médicaux disséminés | Fuite de données de santé de salariés et d'agents | Natures catégorisées, accès journalisés, purges datées, registre |

---

## 4. L'année des absences vue du terrain [H, à confirmer avec Florian]

| Moment | Ce qui se passe | Socles |
|---|---|---|
| **Juin–juillet** | Plan RCD de l'année suivante préparé avec les équipes, volontaires recensés, volume horaire de chacun ; créneaux de disponibilité saisis avec les vœux d'EDT | 15, 10 (EDT-147), 6 |
| **Rentrée** | Plan présenté au CA (public) ou à l'instance de l'établissement (privé) puis transmis au recteur ; vivier activé ; postes non pourvus → services vacants (socle 6) ; premières absences (rentrée des stagiaires, formations) | 15, 6, 10 |
| **Septembre–juin, chaque jour** | 6 h 45–8 h : déclarations ; tableau « ce matin » ; remplacements, annulations, permanences, familles prévenues ; dans la journée : absences inopinées (enfant malade, accident) ; soir : tableau « demain » | 15, 10, 12, 14, 8 |
| **Chaque semaine** | Lundi : remontée RCD de la semaine passée (agrégats) ; relances des justificatifs et des créneaux de rattrapage | 15, 9 |
| **Chaque mois** | État des heures effectuées (remplacements, HSE, missions) signé par le chef d'établissement, transmis ; pour l'OGEC : éléments variables de paie, signalements DSN | 15, 49 |
| **Vacances, examens** | Convocations d'examen (jurys, surveillances) importées ; absences pour formation regroupées ; sessions de juin : beaucoup de professeurs absents de leurs cours (lycée) | 15, 38 |
| **Grèves** | Le matin : liste des absents à cocher, annulations, familles prévenues, retenue à signaler (État : 1/30ᵉ) | 15, 10 (EDT-170) |
| **Fin d'année** | Bilan RCD au CA (au moins deux comptes rendus par an [S R-05]) ; clôture des rattrapages dus ; purges des pièces | 15, 27 |

---

## 5. Ce qui est réglementaire (vérifié le 2026-11-11)

Recherche du 2026-11-11 (enquête C) : **Légifrance et education.gouv.fr ont renvoyé 403** à la session ; les textes ont été lus sur fonction-publique.gouv.fr (sommaires du JORF), senat.fr (rapport d'information sur le remplacement des enseignants du 11/06/2025, réponses ministérielles), service-public.gouv.fr, cnil.fr, ih2ef.gouv.fr, data.gouv.fr, msa.fr. Les deux textes du RCD avaient été **lus sur Légifrance** par la session du socle 10 le 2026-11-09 [S, `10_EMPLOI_DU_TEMPS.md` §5, R-05 et R-09].

| N° | Règle | Référence (URL, consultée le 2026-11-11 sauf mention) | Statut | Conséquence pour le socle 15 |
|---|---|---|---|---|
| R-01 | **Remplacement de courte durée** (absences ≤ 2 semaines) : **plan annuel** du chef d'établissement, élaboré avec les équipes, présenté au CA après avis du conseil pédagogique et transmis au recteur ; compte rendu au moins deux fois par an ; remplacement d'abord en heures de cours, séquences numériques encadrées par des personnels d'éducation possibles ; volontaires sur des **créneaux fixes d'au moins une heure par semaine**, délai de prévenance fixé par le plan, **refus seulement pour motif légitime** ; **abroge le décret 2005-1035** | décret n° 2023-732 du 08/08/2023 — https://legifrance.gouv.fr/loda/id/JORFTEXT000047948581/ ; rapport Sénat 11/06/2025 p. 77-78 — https://www.senat.fr ; ih2ef.gouv.fr (Pacte) | [S] (lu le 2026-11-09) ; [V] (Sénat) | plan RCD versionné (§12) ; vivier, créneaux (lus au socle 10, EDT-147) ; sollicitation, refus **avec motif légitime** codé ; séance « continuité encadrée » |
| R-02 | **Suivi du remplacement de courte durée** : traitement ministériel (SI-RCD) alimenté par les **logiciels de vie scolaire**, données transmises **hebdomadairement par API** : UAI, type d'établissement, disciplines, niveau de classe, heures prévues à l'EDT, heures d'absence déclarées, heures remplacées, mode de continuité pédagogique, **motif d'absence par catégories** (raison de santé, événement familial, convenance personnelle…, « en attente de motif ») ; conservation 5 ans ; pas de droit d'opposition | arrêté du 14/08/2023 — https://www.legifrance.gouv.fr/jorf/id/JORFTEXT000047989129 ; existence confirmée par QE Sénat n° 05531 (JO Sénat 20/11/2025 p. 5770) et n° 07457 (JO Sénat 02/04/2026 p. 1619) | [S] (lu et relu le 2026-11-09) ; [V] (existence) | **chaque nature porte sa catégorie nationale** (§9.2) ; agrégats hebdomadaires **sans nom** (port `edt::agregats_remplacement` du socle 10 + catégorie du 15) ; flux au socle 9 ; **spécification de l'API non publique** : à demander (Q-AP3) |
| R-03 | **Continuité pédagogique** dans les EPLE **et le privé sous contrat** : le chef d'établissement définit les modalités d'organisation de la continuité pédagogique en cas d'absence d'un enseignant ; **remontée des données RCD du privé depuis la rentrée 2024** | décret n° 2023-738 du 09/08/2023 ; C. éduc. R442-39 ; Sénat p. 38-40 | [V] (sommaire JORF et Sénat) | le RCD et la remontée valent **aussi pour l'établissement pilote** (privé) ; plan et modalités tenus de la même façon |
| R-04 | Régime spécifique des **HSE de RCD supprimé** au 01/09/2023 ; RCD payé par la part fonctionnelle du Pacte (note de service MENH2320037N, BO n° 30 du 27/07/2023 ; privé : décret 2023-764) ou en HSE de droit commun ; anciens plafonds (5 h/semaine, 60 h/an) **caducs** | décret n° 2023-734 du 08/08/2023 (JORFTEXT000047948663) ; IH2EF | [V] (abrogation) ; [H] (plafonds, Pacte 2026-2027) | heures typées **HSE / mission Pacte / RCD** (§14) ; aucun plafond codé ; Pacte 2026-2027 non confirmé (Q-AP4) |
| R-05 | **HSE** : 1/36ᵉ du taux annuel de l'HSA majoré de 25 % ; **deux HSA** imposables (sauf raison de santé) | décret n° 50-1253 du 06/10/1950 art. 5 [H] ; décret 2014-940 art. 4 modifié par 2019-309 [S R-04 du socle 10] | [H] / [S] | ScolIA **ne calcule pas** la rémunération des agents de l'État : il **constate** les heures et les exporte (§14) |
| R-06 | HSE pour remplacer un congé de **convenance personnelle** : non (même en auto-remplacement) | guide RCD d'une académie (La Réunion), document 403 | [H] | alerte (pas de blocage) à la saisie du service fait (R-AP33) |
| R-07 | **Heures supplémentaires des maîtres délégués** (privé) | décret n° 2023-739 du 09/08/2023 et arrêté du même jour (JORF n° 184) | [V] (sommaire) | nature d'heure distincte à l'export rectorat |
| R-08 | **Congé de maladie ordinaire** (fonctionnaires) : arrêt transmis **sous 48 h** (prolongations comprises) ; 8 jours pour justifier un retard ; un 2ᵉ retard en 24 mois réduit de moitié la rémunération ; **90 % pendant 3 mois** (loi de finances 2025, art. 189, loi 2025-127 du 14/02/2025, CGFP L822-1), puis 50 % ; **jour de carence** ; l'agent envoie les volets 2 et 3 et **garde le volet 1** (éléments médicaux) | https://www.service-public.gouv.fr/particuliers/vosdroits/F490 (vérifiée par le site le 01/09/2026) ; décret 86-442 art. 24-27 ; décret 2014-1133 [H] | [V] | horodatage de la **réception** de l'arrêt, échéance 48 h affichée (R-AP16) ; **interdiction d'un champ de diagnostic** ; avertissement au dépôt : « ne déposez pas le volet 1 » ; aucune rémunération calculée (rectorat) |
| R-09 | **Contractuels de l'État** : indemnités journalières CPAM et complément de l'État après 4 mois d'ancienneté | décret 86-83 du 17/01/1986 art. 2 et 12 (Sénat) | [V] | nature « maladie » commune ; pas d'effet de paie local |
| R-10 | **Autorisations spéciales d'absence** : réforme **au 1er janvier 2027** — ASA **de droit** (décès d'un proche, mariage ou PACS…) et ASA **soumises aux nécessités du service** ; circulaire annoncée | décret n° 2026-604 du 06/07/2026 (CGFP L622-1, L622-2) — https://www.fonction-publique.gouv.fr (publié le 07/07/2026) | [V] (existence, distinction) ; [H] (durées) | référentiel des natures **versionné par date d'effet** (bascule au 01/01/2027) ; ASA de droit = refus impossible (R-AP12) ; durées à saisir à la publication de la circulaire (Q-AP8) |
| R-11 | **Grève** : préavis de 5 jours francs ; **aucune déclaration individuelle préalable dans le second degré** (la déclaration à 48 h ne vaut que pour les écoles, C. éduc. L133-4) ; fonction publique de l'État : **retenue de 1/30ᵉ par jour**, même pour moins d'une journée | https://www.service-public.gouv.fr/particuliers/vosdroits/F499 (vérifiée par le site le 03/03/2026) ; circulaire du 30/07/2003 | [V] | grève saisie **le jour même ou après** (§16) ; aucune pré-déclaration demandée ; retenue **signalée** à l'export rectorat ; OGEC : retenue proportionnelle [H] (Q-AP10) |
| R-12 | **Formation** : la formation continue liée au projet personnel se suit en priorité **hors des obligations de service d'enseignement** ; baisse constatée des absences pour formation ou réunions internes | C. éduc. L912-1-2 (loi 2019-791) [H] ; QE Sénat 07457 [V] | [H] / [V] | absence « formation » prévue : alerte si elle tombe sur des heures de cours (R-AP35), sans blocage |
| R-13 | **Surveillance des élèves** : surveillance continue pendant le temps scolaire ; **toute modification prévisible des horaires d'entrée ou de sortie due à l'absence d'un enseignant est portée à la connaissance des parents** ; sans information préalable, la surveillance est assurée **dans les horaires habituels** ; le règlement intérieur peut permettre la **sortie en cas d'absence inopinée en fin de demi-journée ou de journée**, avec autorisation des parents et pour les classes qu'il précise ; au lycée, régime adapté ; majeurs soumis au règlement intérieur | circulaire n° 96-248 du 25/10/1996 (NOR MENL9603026C, BO n° 39 du 31/10/1996), modifiée par 2004-054 — lue sur copie intégrale (site syndical) | [V*] | familles prévenues **avant** l'heure qui change, avec trace (§13) ; **sans information, aucune sortie anticipée** (R-AP24) ; sortie seulement en fin de demi-journée ou de journée, selon régime et autorisation (socle 14) |
| R-14 | **Données de santé des personnels** : art. 9 RGPD, base 9-2-b ; données admises pour un arrêt : dates, reprise, **motif limité à accident du travail ou maladie professionnelle** ; **aucun diagnostic** ; accès réservé aux personnes chargées de la gestion du personnel, la hiérarchie seulement pour l'exercice de ses fonctions ; information préalable des agents | référentiel CNIL « gestion du personnel », délibération n° 2019-160 du 21/11/2019 (PDF cnil.fr) ; fiche CNIL du 17/08/2023 | [V] | nature catégorisée, **sensibilité par nature** (§9.2) ; deux niveaux de lecture (§18) ; notice d'information des personnels (Q-AP12) |
| R-15 | **Durées de conservation RH** : registre du personnel 5 ans après le départ ; décompte du temps de travail 1 an (3 ans pour l'action en paiement) ; éléments de cotisations et DSN 6 ans ; bulletins de paie 5 ans ; **exclut le dossier individuel des agents publics** (arrêté du 21/12/2012) | référentiel CNIL des durées de conservation en matière de gestion des ressources humaines (publié le 02/04/2026, mis à jour le 20/05/2026) | [V] | durées par catégorie (§18.4), à valider par le DPO ; pour un agent de l'État, **ScolIA n'est pas le dossier** : il transmet puis purge sa copie (R-AP21) |
| R-16 | **DSN** : signalement d'un arrêt de travail **dans les 5 jours** suivant sa connaissance, même court ; prolongations par la DSN mensuelle | https://www.msa.fr (« 9 règles d'or de la DSN ») | [V partiel] | échéance « DSN J+5 » dans la file de la comptable de l'OGEC (R-AP30) ; la DSN elle-même est au socle 49 |
| R-17 | **Convention collective EPNL** (IDCC 3218, texte du 11/04/2022, applicable au 01/09/2022) : congés exceptionnels, maintien de salaire (chapitre 7) ; code du travail L3142-4 (congés pour événements familiaux) | sources secondaires (Légifrance 403) ; règles reprises du code Next.js `absence-hours-treatment.ts:88-185` [F] | [H] | les 16 congés exceptionnels de Next.js deviennent des natures **versionnées**, à faire confirmer par la comptable ou le juriste de l'OGEC (Q-AP9) |
| R-18 | **Privé sous contrat** : pas de brigade de remplacement ; mobilisation des pairs, voire du chef d'établissement ; pour une absence longue, **demande de suppléance au rectorat**, qui recrute des **maîtres délégués** pour « la durée du besoin » (R914-57), viviers académiques ou diocésains, agrément | Sénat p. 38-40 ; C. éduc. R914-57 [S R-07 du socle 10] | [V] | commande « demander une suppléance » (ABP-180) avec trace de la demande et de la réponse ; le chef d'établissement peut être remplaçant |
| R-19 | **Emploi du temps** (traitement ministériel) : les familles voient la civilité, le nom et la matière des enseignants, **aucune absence** dans ce traitement | arrêté du 21/03/2018 [S R-11 du socle 10] | [S] | le nom du remplaçant est montré ; jamais l'absence d'un professeur sous forme de motif, ni de liste des absents |
| R-20 | **Règlement européen sur l'IA**, annexe III 4 b (attribution de tâches sur la base du comportement ou de caractéristiques personnelles) ; art. 6 §3 (pas de haut risque si tâche préparatoire sans incidence significative, **sauf profilage**) | règlement (UE) 2024/1689 [S R-26 du socle 10] | [S] | proposition de remplaçants **sans** trait personnel ni historique d'absence ; humain décide ; évaluation art. 6 §4 à documenter (lot A13) |
| R-21 | **RGPD art. 22** : pas de décision exclusivement automatisée à effet significatif | [S R-27 du socle 10] | [S] | désignation d'un remplaçant toujours humaine ; contestation possible (ABP-187) |
| R-22 | **Assistants d'éducation** : service annualisé sur 39 à 45 semaines | décret 2003-484 [S R-25 du socle 10] | [S] | absence d'un AED = poste du planning à couvrir (socle 6, P15) |
| R-23 | **Données publiques du remplacement** : jeu DGRH sur les absences de **15 jours et plus** dans le public ; indicateurs nationaux « heures non assurées », taux de remplacement des absences courtes ≈ 10,6 % contre ≈ 95 % pour les longues | data.gouv.fr ; Sénat | [V] | indicateurs du pilotage (§14.7) alignés sur ces définitions |

**Restés ouverts** (à lire depuis un navigateur, Q-AP5) : texte de l'arrêté du 14/08/2023 relu pour la liste exacte des catégories de motif ; spécification de l'API SI-RCD ; Pacte 2026-2027 ; durées des ASA du décret 2026-604 ; maintien de salaire de la convention EPNL ; circuit et outil académique des absences des maîtres du second degré (« ASIE / GI-GC / SUPPLE » [H]) ; circulaire 2002-168 sur les autorisations d'absence [H].

---
## 6. Frontières et ports

### 6.1 Qui possède quoi

| Objet | Possédé par | Le socle 15 |
|---|---|---|
| Agent, engagements (employeur, nature, quotité), responsable fonctionnel (PRS-139), interventions sur les services, suppléance (`ent_service_intervenant`), planning de service des non-enseignants (P15) | **6** | lit ; appelle `services::nommer_suppleant` ; lit les postes du planning à couvrir |
| Séance datée, modification de séance (`intervenant`, `annulation`, `deplacement`, `ajout`), créneaux de disponibilité RCD (EDT-147), plages libres, agrégats hebdomadaires | **10** | lit ; appelle `edt::modifier_seance` dans sa transaction |
| Appel d'une séance, appel de permanence | **12** | lit « appel fait » comme preuve de service fait |
| Sorties anticipées, régime de sortie, présents | **14** | lit les élèves autorisés à sortir ; **ne décide pas** de la sortie |
| Absence, nature, décision, pièces, fil, impact et son traitement, sollicitations de remplaçant, plan RCD, vivier, heures constatées, états mensuels, rattrapages dus, exports d'absences et d'heures | **15** | possède |
| Notifications, plage calme, préférences, « ne pas solliciter » | **8** | déclare ses natures, écrit dans la file d'envoi dans sa transaction |
| Réponse d'absence de la messagerie (MSG-180), redirection | **16** | publie « absent du … au … » (sans nature) ; le 16 l'utilise |
| Cahier de textes, travail donné | **20** | l'absent ou le remplaçant y déposent le travail ; le 15 propose le lien |
| Conseils de classe, rendez-vous, réunions, surveillances d'examen, encadrement de sortie | **23, 19, 38, 42** | lit leurs engagements de l'agent pour l'impact ; propose la réponse à leur propriétaire |
| Éléments variables de paie, DSN, compteurs de congés payés, contrats | **49** | lui fournit les faits (absences validées, heures constatées) par port ; ne calcule aucun salaire |
| Remontée SI-RCD, exports rectorat, formats nationaux | **9** | fournit les agrégats et les lignes ; le 9 transmet et trace |
| Données de santé des **élèves** | **34** | aucune lecture |
| Pièces, stockage, signatures, purge d'objets | **7** (et `ent-object`) | dépose et lit par le port documents ; aucune pièce en base |

### 6.2 Ports exposés par le socle 15 [R]

| Port | Appelé par | Sens |
|---|---|---|
| `absences::present(agent, instant) -> Presence` (présent, absent, inconnu ; **jamais la nature**) | 14 (accueil, présents en cas d'alerte), 16, 19, 29 | lecture minimisée |
| `absences::absences_validees(agent?, plage)` (dates, demi-journées, nature **codée**) | 49 (paie), 6 (fiche agent pour la direction) | lecture restreinte (`personnels.absences.lire_nature`) |
| `absences::impact(cause)` (séances et postes touchés, état) | 10 (tableau « demain »), 29 | lecture sans motif |
| `absences::etat_remplacement(service | seance)` (« en recherche », « remplacé par … à partir du … ») | 10 (EDT-228), 8 | lecture sans motif |
| `absences::heures_constatees(agent, mois)` et `etat_mensuel(mois)` | 49, 9 | lecture restreinte |
| `absences::agregats_rcd(semaine)` (par discipline, niveau, catégorie nationale ; sans nom) | 9 (SI-RCD) | lecture agrégée |
| `absences::declarer_cause_externe(source, agent, plage)` (sortie, jury, réunion, formation déclarée ailleurs) | 42, 38, 19, 23 | commande : crée une **indisponibilité de service** sans nature RH (§7.2) |
| `absences::indisponibilites(agent, plage)` | 10 (génération, réparation), 19 (créneau commun) | lecture sans motif |

### 6.3 Ports consommés [R]

Socle 6 : `agent_du_compte`, `engagements(agent, date)` (employeur, nature, quotité), `responsable_fonctionnel(agent, nature_demande)`, `services::de_l_agent`, `services::intervenants(service, date)`, `services::en_saisie`, `services::nommer_suppleant(service, agent, du, au, origine = absence)`, planning de service (`postes(agent, plage)`). Socle 10 : `edt::seances_touchees(intervenant, plage)`, `edt::seances(cible, plage)`, `edt::plages_libres(ressources, plage, duree)`, `edt::occupe`, `edt::modifier_seance(cmd)`, `edt::heures_assurees`, `edt::agregats_remplacement(semaine)`, `edt::effectifs_attendus(plage, lieu?)`, créneaux RCD (EDT-147). Socle 5 : `jours_ouvres`, `grille(date, site, niveau)` (demi-journées), `annee_a_la_date`, fuseau. Socle 12 : `appel_fait(seance)`. Socle 14 : `sorties_autorisees(eleves, seance)`. Socle 8 : `notifier(nature, destinataires, contenu)`. Socle 7 : `deposer_piece`, `lire_piece` (URL courte), `purger`. Socle 1 : `autoriser(action, portee)`, ré-authentification (actions ★).

### 6.4 Points à accorder avec les socles voisins [R]

- **Socle 6** : le « responsable fonctionnel » (PRS-139) est le **premier valideur** d'une absence d'agent non enseignant ; le port `nommer_suppleant` doit accepter `origine = absence` et l'identifiant de l'absence ; la suspension d'engagement (R-P4 du 6) n'est **pas** une absence (disponibilité, détachement : pas de séance à remplacer, le service est vacant ou suppléé).
- **Socle 10** : une séance touchée par une absence passe `non_assuree` **tant qu'aucune réponse** n'est posée (état calculé, §10.3) ; le motif public d'une modification venue du 15 est toujours « absence du professeur » ou « changement d'enseignant », jamais la nature ; le 10 garde l'annulation de masse et la grève du jour (EDT-170), le 15 y ajoute les absences.
- **Socle 9** : le flux SI-RCD est construit au 9 (lot à créer, Q-EDT11) à partir de `absences::agregats_rcd`.
- **Socle 49** : tant que le 49 n'existe pas, le 15 fournit un **export** des éléments variables (CSV ou format du logiciel de paie actuel de l'OGEC, Q-AP11).

---

## 7. Concepts et vocabulaire

### 7.1 Vocabulaire fixé [R]

| Terme | Sens | Exemple | Table |
|---|---|---|---|
| **Absence** | Période où un agent n'assure pas son service, pour une **nature** donnée | « Mme M., du 12 au 14 novembre, maladie » (nature lue par la direction seulement) | `ent_absp_absence` |
| **Nature** | Entrée du référentiel : famille, catégorie nationale (R-02), employeur concerné, sensibilité, refus possible ou non, pièce attendue, effet sur les heures | « Enfant malade » : famille événement familial, catégorie nationale « événement familial », non discrétionnaire, sensible | `ent_absp_nature` |
| **Indisponibilité de service** | Absence **sans fait RH** : l'agent est ailleurs pour l'établissement (sortie, jury, réunion, formation déclarée par un autre socle) | « M. B. accompagne la sortie au musée » | `ent_absp_absence` avec `cause = 'service'` |
| **Plage** | Morceau daté d'une absence (une absence peut être discontinue) | « lundis matin du 3 au 24 mars » | `ent_absp_plage` |
| **Décision** | Fait daté sur une absence : validation, refus, requalification, prolongation, reprise anticipée, annulation, réouverture | « validée le 12/11 par M. D., traitement : sans incidence » | `ent_absp_decision` |
| **Circuit** | Suite des intervenants d'une absence selon l'employeur et la nature : déclarant, valideur, instructeur (traitement administratif), destinataire paie | | `ent_absp_circuit_regle` |
| **Impact** | Élément de service touché par une absence : séance, poste de service, engagement d'un autre socle | « 4B maths, jeudi 10 h » | `ent_absp_impact` |
| **Réponse** | Décision d'organisation sur un impact | remplacée, autre cours, déplacée, annulée + permanence, continuité numérique, suppléance, sans objet | colonne de `ent_absp_impact` + `ent_absp_reponse` |
| **Sollicitation** | Demande faite à un remplaçant pour un impact, avec acceptation ou refus motivé | « demandé à M. L. à 7 h 12, accepté à 7 h 14 » | `ent_absp_sollicitation` |
| **Volontaire RCD** | Enseignant engagé sur un volume de remplacement de courte durée, avec ses créneaux | « Mme Z., 18 h dans l'année, jeudi M4 » | `ent_absp_rcd_engagement` (créneaux au 10) |
| **Plan RCD** | Document annuel versionné du chef d'établissement (R-01) | « Plan 2027-2028, v2, présenté au CA le 3/10 » | `ent_absp_rcd_plan` |
| **Heure constatée** | Heure de service réellement faite qui ouvre un droit (remplacement, HSE, mission, rattrapage) ou une retenue à signaler | « 1 h RCD, 4B, 14/11, appel fait » | `ent_absp_heure` |
| **État mensuel** | Relevé figé des heures constatées d'un mois, signé, exporté | | `ent_absp_etat` |
| **Rattrapage dû** | Heures qu'un agent doit rendre après une absence « à rattraper » | « 3 h à rattraper avant le 30 juin » | `ent_absp_rattrapage` |

### 7.2 Distinctions qui manquent aujourd'hui

1. **Absence ≠ impact ≠ réponse** : aujourd'hui l'exception d'EDT porte à la fois le motif, le remplaçant et le lien à l'absence (`ent_edt_exception.motif`, `absence_rh_id`) [F].
2. **Fait RH ≠ indisponibilité de service** : un accompagnement de sortie n'a ni nature, ni validation RH, ni pièce ; il a le même impact.
3. **Remplacement d'une séance ≠ suppléance d'un service** : la première est une modification de séance (socle 10), la seconde une intervention datée (socle 6) qui hérite de tout ; une même absence peut produire les deux (deux premières semaines au RCD, puis suppléant).
4. **Agent ≠ compte** (D-020) : l'absent et le remplaçant sont des **agents**, même sans compte (AR-19).
5. **Déclarant ≠ auteur de la saisie ≠ absent** : l'accueil saisit pour l'agent ; l'agent confirme ; la direction valide (AD-07, AR-13).

---

## 8. Cycle de vie d'une absence

### 8.1 États [R]

| État | Sens | Entrées | Effets |
|---|---|---|---|
| `signalee` | Connue de l'établissement, pas encore confirmée par l'agent (saisie de l'accueil, d'un collègue, de la direction) | ABP-102, ABP-103 | **impact calculé et visible** (on organise même sans confirmation) ; l'agent est invité à confirmer |
| `declaree` | Confirmée par l'agent ou saisie par lui | ABP-100, ABP-112 | impact ; file du valideur |
| `en_instruction` | Le valideur a demandé une pièce ou une précision | ABP-123 | relance datée |
| `validee` | Décision favorable, traitement des heures choisi | ABP-121 | file de traitement administratif ou de paie ; heures dues ou retenue signalées |
| `refusee` | Décision défavorable (seulement si la nature le permet) | ABP-122 | l'impact reste (l'agent n'était pas là) ; la qualification passe à « absence non autorisée » pour les heures (R-AP15) |
| `traitee` | Traitement administratif fait (transmis au rectorat, à la paie) | ABP-125 | purge programmée des pièces |
| `close` | Plus rien à faire (rattrapages faits, état exporté) | ABP-126 ou automatique | conservation selon §18.4 |
| `annulee` | Saisie par erreur ou absence qui n'a pas eu lieu | ABP-113 | impact retiré, réponses proposées à la levée, heures retirées ; **rien n'est supprimé** |

**Indisponibilité de service** : états réduits `prevue`, `annulee`, `passee` ; pas de validation RH, l'organisateur du socle source en répond.

### 8.2 Faits qui modifient une absence (ajout seul) [R]

`prolongee` (nouvelle fin ; nouvel arrêt), `ecourtee` (reprise anticipée), `requalifiee` (nature changée : « convenance » → « enfant malade », « maladie » → « accident de service »), `plage_ajoutee` / `plage_retiree`, `reouverte` (retour à `declaree`, avec liste des effets à défaire, R-AP17), `piece_deposee`, `piece_purgee`, `message`, `transmise` (rectorat, paie), `relancee`.
Chaque fait porte : auteur (agent et compte), instant, ancienne et nouvelle valeur en **colonnes** (jamais un JSON relu, D-036), raison codée, note facultative visible des instructeurs.

### 8.3 Ce que chaque transition déclenche [R]

| Transition | Impact | Familles | Heures | Paie / rectorat |
|---|---|---|---|---|
| création (`signalee` ou `declaree`) | calculé, propositions de réponse | rien tant qu'aucune réponse n'est posée ; **mais** à H-2 d'une séance encore `a_traiter`, alerte à l'adjoint et à la vie scolaire (R-AP23) | — | — |
| `validee` | inchangé | — | retenue à signaler ou rattrapage dû selon le traitement | file de traitement (rectorat, comptabilité) ; DSN J+5 si OGEC et arrêt |
| `refusee` | inchangé | — | qualification « non autorisée » | signalé (retenue possible, décision humaine) |
| `prolongee` | impact étendu ; réponses récurrentes proposées | une seule notification par étape (R-E35 du socle 10) | — | nouvelle transmission |
| `ecourtee` | impacts au-delà de la reprise retirés ; réponses à lever **proposées** (pas levées seules : le remplaçant a peut-être préparé) | familles prévenues du retour si une annulation est levée | heures du remplaçant au-delà retirées si non faites | correction transmise |
| `annulee` | idem `ecourtee` sur toute la période | idem | idem | annulation transmise si déjà transmise |
| `requalifiee` | — | — | traitement recalculé, proposé | correction transmise |

---

## 9. Natures d'absence et circuits

### 9.1 Principe [R]

Le **motif libre disparaît** (R-AP1). L'agent choisit une **nature** dans une courte liste adaptée à son engagement ; le texte libre est réservé aux natures non sensibles (« réunion au rectorat ») et limité à 200 caractères relus. Le référentiel est **fourni** (versionné, daté), l'établissement peut **désactiver** une nature ou en **ajouter** une non sensible, jamais changer la sensibilité ou la catégorie nationale d'une nature fournie (D-023 : la protection n'est pas un réglage).

### 9.2 Référentiel fourni (version initiale) [R ; contenu à confirmer, Q-AP8, Q-AP9]

| Code | Libellé affiché à l'agent | Famille | Catégorie nationale (R-02) | Employeur | Sensible | Refus possible | Pièce attendue | Traitement par défaut |
|---|---|---|---|---|---|---|---|---|
| `maladie` | Arrêt de travail (maladie) | santé | raison de santé | tous | **oui** | non | arrêt (volets 2 et 3 ; OGEC : selon CCN) sous 48 h | État : transmis rectorat ; OGEC : maintien de salaire (CCN) |
| `accident_service` | Accident du travail / de service | santé | raison de santé | tous | **oui** | non | déclaration | idem |
| `maternite_paternite` | Congé de maternité, de paternité, d'adoption | parentalité | événement familial [NV] | tous | oui | non | justificatif | suppléance proposée |
| `enfant_malade` | Garde d'enfant malade | famille | événement familial | tous | **oui** | non (NX15-18) | certificat | État : ASA ; OGEC : CCN |
| `evenement_familial` | Événement familial (décès, mariage, PACS, naissance…) — **sous-codes** des 16 congés exceptionnels de Next.js | famille | événement familial | tous (sous-codes OGEC : CCN EPNL et L3142-4 ; État : décret 2026-604 dès le 01/01/2027) | oui | non pour les ASA **de droit** (R-10) | justificatif | jours ouvrables suggérés (NX15-39) |
| `asa_service` | Autorisation d'absence soumise aux nécessités du service | autorisation | convenance personnelle [NV] | État | non | **oui** | selon le cas | décision du chef d'établissement |
| `formation` | Formation | service | formation [NV] | tous | non | oui (si sur temps de cours, R-AP35) | convocation | sans incidence |
| `convocation` | Convocation (examen, jury, rectorat, justice) | service | institutionnel [NV] | tous | non | non | convocation (import OCR possible, ABP-105) | sans incidence |
| `representation` | Mandat syndical, représentation du personnel, instance | représentation | [NV] | tous | **oui** (appartenance syndicale, art. 9) | non dans la limite des droits [NV] | — | sans incidence |
| `greve` | Grève | grève | [NV] | tous | **oui** (opinion) | non | — | État : retenue 1/30ᵉ signalée ; OGEC : retenue proportionnelle [H] |
| `convenance` | Convenance personnelle (sans solde) | autorisation | convenance personnelle | tous | non | **oui** | — | retenue ou rattrapage |
| `retard` | Retard (moins de 4 h, même jour) | — | — | tous | non | oui | — | sans incidence (NX15-35) |
| `conge_paye` | Congés payés, RTT, récupération | congés | — (hors temps d'enseignement pour un enseignant [H]) | OGEC | non | **oui** (demande préalable) | — | décompte au socle 49 |
| `absence_injustifiee` | (posée par la direction seulement) Absence sans justification | — | « en attente de motif » | tous | non | — | — | service non fait signalé |
| `autre_service` | Indisponibilité de service (posée par un autre socle) | service | — | tous | non | — | — | sans incidence |
| `inconnue` | « Je préciserai plus tard » | — | **en attente de motif** (R-02) | tous | — | — | relance | à qualifier |

Pour l'agent, la liste est courte et dans ses mots ; pour le valideur, la nature porte ses conséquences. **Une nature sensible n'apparaît jamais en clair hors du cercle d'instruction** : le reste du système voit « absent » ou, pour les familles, rien (R-AP2).

### 9.3 Traitement des heures (repris de NX15-36) [R]

| Engagement | Options | Par défaut |
|---|---|---|
| Agent de l'État (maître, fonctionnaire, contractuel) | `sans_incidence`, `rattrapage_interne`, `transmission_rectorat` (ou ONISE, premier degré, socle 50), `service_non_fait` (retenue signalée) | selon la nature (§9.2) |
| Salarié de l'OGEC | `sans_incidence`, `rattrapage`, `retenue_salaire`, `maintien_salaire` (selon CCN), `conge_paye` | selon la nature |
| Retard (≤ 4 h) | `sans_incidence`, `rattrapage` | `sans_incidence` |

Natures non discrétionnaires : le traitement est **imposé** par la nature (NX15-19) ; le valideur ne choisit pas, il constate.

### 9.4 Circuit [R]

Pour chaque absence, le serveur calcule à la création (et fige dans `ent_absp_circuit`, copie datée, corrigeant AD-07) :
1. **Valideur** : le responsable fonctionnel de l'agent pour la nature (socle 6, PRS-139) ; à défaut, la règle de l'établissement par employeur et cycle (NX15-14, NX15-15) ; à défaut, le chef d'établissement. **Jamais l'absent lui-même** (R-AP10) ; si l'absent est le valideur désigné, on remonte d'un cran ; si c'est le chef d'établissement, c'est son délégataire désigné (socle 6, P6), ou l'autorité de tutelle ou l'OGEC selon le paramétrage (Q-AP6).
2. **Instructeur** (traitement administratif) : secrétariat pour les agents de l'État, comptabilité de l'OGEC pour les salariés, plus le responsable du planning de service si l'agent tient un poste (surveillant, agent) (NX15-16).
3. **Destinataires d'information sans nature** : adjoint ou chargé des remplacements, vie scolaire (impact seulement).

Délégation : un valideur absent ou en congé délègue (socle 6, P6) ; la file passe au délégataire **sans** changer le circuit figé, avec trace.

---

## 10. Impact : les séances et postes touchés

### 10.1 Calcul [R]

À chaque création ou changement d'une absence ou d'une indisponibilité, le serveur calcule (dans la transaction) :
- les **séances** du socle 10 où l'agent est **en saisie** à la date (titulaire, co-enseignant, suppléant, remplaçant d'une séance) — `edt::seances_touchees(agent, plage)` ;
- les **postes de service** du planning du socle 6 (surveillance, loge, cantine, internat) ;
- les **engagements** déclarés par les socles 19, 23, 38, 42 (conseil de classe, réunion parents, surveillance d'examen, encadrement) ;
- pour une **AESH**, les séances des élèves accompagnés (`edt::seances_accompagnees`, socle 10 §8.3 bis) ;
- pour un **co-enseignement** : la séance n'est touchée que si **aucun** autre intervenant en saisie n'est présent (R-AP25).
Chaque élément devient une ligne d'impact avec un **état de traitement** ; une plage partielle (absence de 10 h à 12 h) ne touche que les séances qui la recoupent.

### 10.2 États d'un impact [R]

`a_traiter` (rien n'est posé) · `propose` (une réponse proposée, non validée) · `sollicite` (un remplaçant a été demandé, en attente) · `remplace` · `autre_cours` · `deplace` · `continuite` (séquence encadrée, R-01) · `annule` (élèves pris en charge) · `supplee` (couvert par un suppléant de service) · `sans_objet` (la séance n'avait pas lieu : vacances, sortie de toute la classe, examen) · `non_assure` (rien n'a été fait et l'heure est passée).

### 10.3 Lien avec l'état de la séance du socle 10 [R]

Le socle 10 calcule `non_assuree` pour une séance dont aucun intervenant en saisie n'est disponible ; le 15 le lui fournit (`absences::indisponibilites`). Toute réponse est une **modification** du 10 (type `intervenant`, `annulation`, `deplacement`, `ajout`), créée par le port dans la transaction du 15. **Le 15 n'écrit jamais dans les tables du 10** (règle 1 de la carte).

### 10.4 Tableau « ce matin » et « demain » [R]

Pour l'adjoint, la direction, la vie scolaire (sans nature) : pour chaque heure de la journée, les absents connus, les impacts par état, les propositions prêtes, les sollicitations en attente (avec l'heure de la demande), les élèves attendus en permanence (`edt::effectifs_attendus`), les salles libres, les familles déjà prévenues et celles qui le seront (avec l'heure d'envoi prévue). Mis à jour en continu. C'est l'écran 1 du §25.

---

## 11. Répondre : remplacer, suppléer, annuler

### 11.1 Les réponses possibles [R]

| Réponse | Quand | Effet (socle) | Heures |
|---|---|---|---|
| **Remplacer la séance** par un collègue | absence courte, collègue libre | modification `intervenant` (10) ; délégation de la séance seulement (1, D12 du 10) | heure constatée pour le remplaçant si appel fait (§14) |
| **Autre cours** (le collègue de français fait français) | pas de remplaçant de la discipline | séance d'origine annulée, séance `ajoutee` du cours du remplaçant (10, CL-12) | idem, au cours du remplaçant |
| **Déplacer** (le titulaire refera la séance plus tard, ou avance un cours d'un autre jour) | absence prévue | modification `deplacement` (10) | sans heure supplémentaire si dans le service |
| **Continuité encadrée** (séquence numérique, devoir surveillé, CDI encadré par un AED) | aucun enseignant (R-01) | séance `annulee` côté enseignement + « continuité » avec encadrant (AED, documentaliste) | pas d'heure d'enseignement ; l'AED est dans son planning |
| **Annuler** et prendre en charge les élèves | rien d'autre possible | modification `annulation` (10) ; permanence (12) ; sortie anticipée selon régime **si les familles ont été prévenues** (14, R-13) | — |
| **Suppléance** (absence longue) | durée > seuil (défaut [R] 10 jours ouvrés, S-1503) ou nature longue | `services::nommer_suppleant` (6) : le suppléant hérite du service ; les séances changent d'intervenant à la date | heures du suppléant = son service (pas d'heure supplémentaire) |
| **Demander une suppléance au rectorat** (privé ; public : à l'IA-IPR / DPE [NV]) | absence longue d'un maître | demande tracée, réponse attendue (ABP-180, R-18) | — |
| **Sans objet** | séance qui n'avait pas lieu | — | — |

### 11.2 Proposer les remplaçants (déterministe, explicable) [R]

Candidats : agents ayant un engagement actif d'enseignement (ou habilités pour la continuité encadrée), **libres** sur la plage (`edt::plages_libres`, indisponibilités, absences, autres sollicitations acceptées), **pas absents**, présents dans l'établissement ce jour-là (pas de complément de service ailleurs ce jour, socle 6 P3).
Ordre, chaque critère affiché :
1. **Volontaire RCD dont un créneau couvre la plage** (R-01) ;
2. **même discipline** (discipline de l'engagement, socle 6 — fin du rapprochement par libellé, D15 du 10), puis connaît la classe (enseigne déjà à ce public) ;
3. présent dans l'établissement juste avant ou après (pas de venue spéciale) ;
4. **équité** : le moins sollicité sur la période (compte de remplacements faits **et** refusés pour motif légitime), le plus loin de son volume RCD consommé ;
5. respect du **délai de prévenance** du plan (R-01) : un candidat hors délai est montré « hors délai, sur accord seulement ».
**Jamais** : âge, ancienneté, historique d'absences, santé, situation familiale, temps partiel thérapeutique (R-AP37, R-20). Les contraintes d'un collègue ne sont **jamais nommées** à un autre (D-033 B).

### 11.3 Solliciter, accepter, refuser [R]

- La sollicitation part au remplaçant (notification du socle 8) avec : classe ou groupe, heure, salle, cours, **travail prévu** (lien vers le cahier de textes du 20), effectif, élèves à besoins particuliers **dont il doit connaître l'aménagement** (socle 35, minimal), sans aucune nature de l'absence.
- **Plage calme** (socle 16 : plage nationale sans diffusion de 20 h à 7 h) : une sollicitation pour une séance du jour même ou du lendemain matin est une **exception de service** autorisée de 6 h 30 à 20 h ; hors de cette fenêtre elle attend 6 h 30 (Q-AP14, appliqué par défaut).
- Il **accepte** d'un geste (mobile) ; il peut **refuser**. Pour un volontaire RCD sur son créneau, le refus exige un **motif légitime** codé (« absent », « autre service à cette heure », « délai de prévenance non respecté », « autre ») — R-01 ; pour un collègue hors RCD, le refus est libre et **sans motif** (il n'a aucune obligation).
- **Désignation sans sollicitation** : l'adjoint peut désigner directement (urgence) ; le remplaçant reçoit l'information et accuse réception ; sans accusé une heure avant, alerte à la vie scolaire (EDT-238 du 10).
- Expiration : une sollicitation sans réponse expire au délai fixé (défaut [R] 20 minutes pour le jour même, 12 h sinon, S-1505) ; la suivante est proposée.
- Plusieurs sollicitations simultanées pour une même séance : interdites par défaut (la première qui accepte gagne serait injuste et confuse) ; possible en « appel à volontaires » explicite (ABP-185) où le premier qui accepte est retenu et les autres sont prévenus aussitôt.

### 11.4 Suppléance et absence longue [R]

- Au-delà du seuil (S-1503) ou pour une nature longue (maternité), le 15 **propose** la suppléance : recherche d'un suppléant (agent existant, contractuel à créer au socle 6, demande au rectorat), dates, recouvrement, simulation de ce que verra le suppléant (socle 6, écran Suppléance).
- En attendant le suppléant : réponses séance par séance (RCD les deux premières semaines [H]), puis bascule ; **état du remplacement** publié aux familles (EDT-228 : « remplacement en recherche », « remplacé par M. Y à partir du 18 novembre »), une seule notification par étape.
- Suppléant contraint (libre trois jours sur cinq) : le 15 demande au socle 10 une **réparation** (§11.8 du 10, version temporaire) ; le 15 n'en fait pas lui-même.
- Retour anticipé du titulaire : le 15 propose d'écourter la suppléance (port du 6) ; jamais seule.

### 11.5 Garanties [R]

Une séance a **au plus une** réponse active ; un remplaçant n'a **jamais** deux séances acceptées qui se chevauchent (contrainte d'exclusion en base sur `(etablissement_id, agent_id, tstzrange)` des sollicitations acceptées, corrigeant AR-09) ; toute réponse est réversible (levée = fait daté, AR-15) ; la levée d'un remplacement **révoque** sa délégation (AR-18) ; une séance passée ne change plus de réponse sauf **correction** (droit spécial, motif, journal ; effets aval signalés : appel fait, heures).

---

## 12. Remplacement de courte durée (RCD)

### 12.1 Plan annuel [R, R-01, R-03]

Objet versionné par année : objectifs, ordre de recours (volontaires, puis collègues, puis continuité encadrée), **délai de prévenance**, modalités de continuité numérique, référent, date d'avis du conseil pédagogique, date de présentation au CA (public) ou à l'instance de l'établissement (privé, Q-AP7), date de transmission au recteur, **comptes rendus** (au moins deux par an) produits depuis les indicateurs. Document généré depuis une trame (D-023), déposé au socle 7, signé.

### 12.2 Vivier [R]

Engagement de chaque volontaire : volume annuel (heures), période, statut (Pacte, HSE, autre, R-04) ; créneaux tenus **au socle 10** (EDT-147, préservés par la génération) ; compteurs : sollicité, accepté, refusé avec motif, fait, restant ; alerte au volume atteint (pas de blocage : l'accord vaut).

### 12.3 Remontée hebdomadaire [R, R-02]

Chaque lundi (paramétrable), le 15 prépare la semaine passée : par discipline, niveau, catégorie nationale du motif, mode de continuité : heures prévues à l'EDT, heures d'absence déclarées (≤ 2 semaines), heures remplacées. **Aucun nom, aucune nature fine**. Un administratif **contrôle puis laisse partir** (EDT-198) ; le flux est au socle 9. Tant que la spécification n'est pas obtenue (Q-AP3), l'écran produit un **tableau exportable** équivalent.

---

## 13. Familles et élèves

### 13.1 Règles [R]

1. **Jamais de motif, jamais le mot « absent » appliqué à un adulte nommé dans un message aux familles.** Formulations : « Le cours de mathématiques de 10 h n'aura pas lieu ; Léa est attendue en permanence (salle 12) » ; « Le cours de mathématiques de 10 h sera assuré par M. Y » ; « Le cours est déplacé au … » (R-19 : le nom de l'intervenant est montré).
2. **Prévenir avant l'heure qui change** (R-13) : une famille est prévenue dès qu'une réponse **change l'heure d'arrivée ou de départ** de l'enfant, ou une journée entière (R-E36 du socle 10) ; un remplacement en milieu de journée entre dans le résumé du jour ; l'élève est prévenu de tout.
3. **Les deux parents** ayant droit à l'information scolaire sont prévenus (socle 4, droits du lien ; parents séparés) ; l'élève majeur l'est lui-même, ses parents selon son choix (socle 2).
4. **Pas de sortie anticipée sans information préalable** (R-13, R-AP24) : la sortie anticipée n'est proposée que si la séance annulée est **la dernière de la demi-journée ou de la journée** pour cet élève, que son régime et l'autorisation parentale le permettent (socle 14) **et** que la famille a été prévenue au moins N heures avant (S-1508, défaut [R] la veille à 18 h ou 2 h avant). Sinon : permanence.
5. **Absence longue** : l'état du remplacement est montré dans l'EDT de l'élève, une notification par étape (EDT-228).
6. **Heures perdues** : la famille peut voir, pour l'enfant, le **compte des séances non assurées** de l'année par matière (transparence, différenciant [R], S-1510 : désactivable) ; jamais par professeur nommé.

### 13.2 AESH [R]

Absence d'une AESH : la vie scolaire, le référent et les professeurs des séances concernées sont prévenus (« l'accompagnement de Léo n'est pas assuré ce matin »), la famille de l'élève accompagné aussi si le PAP/PPS le prévoit (socle 35), sans motif ; une réponse peut être « autre AESH » ou « aménagement » ; aucun remplacement automatique.

---

## 14. Heures et service fait

### 14.1 Principe [R]

ScolIA **constate**, ne calcule pas de rémunération des agents de l'État (R-05) et ne fait pas la paie de l'OGEC (socle 49). Une **heure constatée** naît d'un fait : une séance remplacée **assurée** (l'appel du socle 12 est fait par le remplaçant, ou à défaut le remplaçant atteste), une séance `ajoutee` de rattrapage assurée, une mission comptée. Elle a une **nature** : `rcd`, `hse`, `mission_pacte`, `rattrapage_du` (ne paie rien : solde une dette), `service_non_fait` (retenue à signaler), `vacation` (vacataire payé à l'heure, socle 6 P57).

### 14.2 Cycle [R]

`constatee` (automatique) → `contestee` (par l'agent ou l'adjoint, motif) → `validee` (adjoint) → dans un **état mensuel** `fige` puis `signe` (chef d'établissement, ★) → `exporte` (rectorat, paie) ; une correction après export = ligne de **régularisation** sur l'état suivant, jamais une réécriture (D-027 E, même principe que les factures).

### 14.3 Ce que voit l'agent [R]

« Mes heures » (EDT-119 du 10) : séances assurées en plus, par mois, par nature, avec leur état ; ce qui manque (séance remplacée sans appel) ; contestation d'un geste.

### 14.4 Heures d'absence (pour la paie de l'OGEC et les retenues) [R]

Calculées depuis l'**impact réel** (séances et postes touchés) et non estimées (fin de l'estimation à 7 h par jour de Next.js, NX15-41) ; pour un salarié sans planning, depuis son horaire contractuel (socle 6, engagement, quotité) ; en **demi-journées et jours ouvrés** pour les natures décomptées en jours (CCN) — `jours_ouvres` du socle 5, fuseau de l'établissement.

### 14.5 Exports [R]

| Export | Pour | Contenu | Format |
|---|---|---|---|
| État mensuel des heures (agents de l'État) | rectorat (DEP/DPE ; privé : DEEP [NV]) | par agent : HSE, RCD, missions, retenues signalées | trame ScolIA + CSV ; format académique à obtenir (Q-AP5) |
| Éléments variables de paie (OGEC) | comptabilité, socle 49 | par salarié et par mois : absences par nature **codée**, heures, jours, maintien, retenues, heures faites | CSV ou format du logiciel de paie (Q-AP11) |
| Absences à transmettre (agents de l'État) | rectorat | par absence : agent, dates, nature **codée**, date de réception de l'arrêt | trame + CSV |
| Remontée RCD | SI-RCD (socle 9) | agrégats sans nom | API (Q-AP3) |
Tous réservés aux administratifs (D-022), tracés (`ent_export_trace` du socle 9), à double validation si nominatifs complets (D-041, proposé).

### 14.6 DSN (OGEC) [R]

Tant que le socle 49 n'existe pas : la comptable voit la file « arrêts à signaler » avec l'échéance **J+5** (R-16), coche « signalé le » ; ScolIA ne produit pas de DSN.

### 14.7 Indicateurs (pilotage, socle 47) [R]

Agrégés seulement : heures prévues, absentes, remplacées, non assurées, par discipline, niveau, semaine, catégorie nationale ; taux de remplacement des absences courtes et longues (R-23) ; délai moyen de réponse ; part des familles prévenues avant l'heure. **Pas d'indicateur nominatif d'absentéisme** (R-AP38). Effectifs < 5 masqués (secret statistique, S-1512).

---

## 15. Rattrapages

Repris de Next.js (NX15-12, NX15-26, NX15-27) et rendu concret :
- Une absence validée avec traitement `rattrapage` crée un **rattrapage dû** : nombre d'heures (depuis l'impact), échéance (défaut [R] fin de l'année scolaire, S-1514), forme (cours rendu aux mêmes élèves, heures de service pour un salarié).
- L'agent **propose des créneaux** (choisis parmi ses plages libres et celles du public concerné, `edt::plages_libres`), la direction confirme ; le créneau confirmé devient une séance `ajoutee` (10) pour un enseignant, ou une plage de planning (6) pour un salarié ; famille prévenue (séance ajoutée).
- Fait → solde ; relance automatique si aucun créneau proposé 10 jours après la validation (NX15-27, S-1515) ; à l'échéance non soldée : alerte à la direction (décision : prolonger, requalifier en retenue, abandonner avec motif).
- Le rattrapage d'**heures d'élèves perdues** (cours non assurés, sans dette d'un agent : poste vacant, absence non remplacée) est une proposition au socle 10 (séances `ajoutee`), pas une dette (T-14 de la matrice).

---

## 16. Absences collectives : grève, intempéries, épidémie

- **Grève** (R-11) : aucune pré-déclaration ; le matin, l'adjoint coche les absents d'une liste (EDT-170 du 10) ; le 15 crée une absence `greve` par agent, **signalee** ; un agent peut se déclarer lui-même gréviste après coup. Nature sensible (opinion) : visible de la direction et du secrétariat seulement ; jamais une liste des grévistes pour les collègues ni les familles. Familles : annulations sans motif nominatif, motif public « mouvement social » possible pour un lot (motif public du 10). Retenue 1/30ᵉ signalée à l'export (État) ; OGEC : selon Q-AP10.
- **Intempéries, fermeture** : ce n'est pas une absence de personnel ; c'est une fermeture (socle 5) ; les personnels absents ce jour-là ne sont pas comptés absents.
- **Épidémie, nombreux arrêts** : tableau de crise (taux d'impact par heure, classes sans aucun cours) ; bascule possible en continuité pédagogique à distance (socle 33) ; aucune information de santé collective sur les personnels n'est diffusée.

---

## 17. Personnels non enseignants

- Surveillants, AED, agents, administratifs, comptables, infirmière, documentaliste : leur absence a un **impact sur des postes** du planning de service (socle 6, P15), pas sur des séances (sauf documentaliste qui enseigne, infirmière qui intervient, PRS-124).
- Réponse : **poste couvert par un autre agent** (échange, heure en plus, socle 6), poste **fermé** (loge fermée de 12 h à 14 h, CDI fermé : affichage public), ou **redistribution** (surveillance de cour à deux au lieu de trois : alerte si sous le minimum fixé par le planning).
- Circuit OGEC : valideur = responsable fonctionnel (socle 6, PRS-139 : CPE pour les AED, gestionnaire pour les agents) ; instructeur = comptabilité ; salariés de droit privé : repos quotidien de 11 h et pause de 20 min respectés pour l'agent qui couvre (C. trav., R-30 du socle 10) — **alerte**, pas de blocage.
- Infirmière absente : le socle 34 bascule son protocole (« infirmerie fermée ») ; le 15 ne lit aucune donnée de santé d'élève.

---

## 18. Confidentialité, pièces, conservation

### 18.1 Deux niveaux de lecture [R, R-14]

| Niveau | Contenu | Qui |
|---|---|---|
| **Organisation** | agent absent, dates, plages, impact, réponses, remplaçants, état du remplacement | adjoint, direction, vie scolaire, CPE (et l'accueil en lecture « présent / absent »), PP pour sa classe (séances seulement), remplaçant pour sa séance, familles et élèves (séances seulement) |
| **RH** | nature, décisions et leurs notes, pièces, fil de messages, traitement des heures, transmissions, rattrapages dus | l'agent lui-même ; le valideur du circuit ; l'instructeur du circuit ; le chef d'établissement ; la comptabilité pour les salariés de l'OGEC ; **personne d'autre**, administrateur technique compris |

La projection d'organisation est une **vue serveur sans colonne de nature** (et non un filtre d'affichage) ; les DTO correspondants n'ont pas de champ « nature » ni « motif » (test de garde, §29).

### 18.2 Pièces [R]

Déposées par `ent-object` (E5 : objet immuable, empreinte, chiffrement au repos), jamais par URL fournie (AD-01) ; type et taille contrôlés ; antivirus (socle 7) ; **avertissement** au dépôt d'un arrêt : « ne déposez que les volets 2 et 3, jamais le volet 1 » (R-08) ; lecture par URL pré-signée de 5 minutes après contrôle du droit, **journalisée** ; jamais en pièce jointe de mail (AD-04) ; aucune lecture par l'IA (R-AP40). Les convocations (non sensibles) peuvent être lues par l'OCR à la demande d'un administratif (ABP-105) ; un arrêt de travail, jamais.

### 18.3 Ce qu'on ne collecte pas [R]

Diagnostic, nom du médecin, pathologie, durée prévisible de traitement, volet 1, nom d'un enfant malade, lien de parenté détaillé au-delà du sous-code (« décès d'un parent » suffit), appartenance syndicale au-delà de « représentation », motif d'une grève.

### 18.4 Conservation [R, à valider par le DPO, Q-AP12]

| Donnée | Durée proposée | Fondement |
|---|---|---|
| Pièce d'une absence d'un **agent de l'État** | supprimée **30 jours après la transmission** au rectorat (ScolIA n'est pas le dossier individuel) | R-15 (arrêté du 21/12/2012 hors référentiel) |
| Pièce d'une absence d'un **salarié de l'OGEC** | durée de la pièce justificative de paie [NV], à défaut 3 ans après la fin de l'année, puis purge | R-15 (décompte, action en paiement) |
| Nature d'une absence sensible (après clôture) | conservée **codée** 5 ans (registre, contentieux), sans pièce | R-15 |
| Absence, impact, réponses (niveau organisation) | 5 ans (remontée RCD : 5 ans, R-02) | R-02, R-15 |
| Heures constatées, états mensuels | 6 ans (éléments de cotisation) | R-15 |
| Sollicitations et refus | 1 an après la fin de l'année scolaire | minimisation |
| Fil de messages | purgé avec les pièces | minimisation |
| Journal | 5 ans | E7 |
Purges **datées et journalisées** (catégorie de conservation du registre E12), jamais par expression régulière sur le texte (NX15-48 remplacé).

### 18.5 Les personnels comme personnes concernées [R]

Notice d'information (R-14) ; accès de l'agent à **toutes** ses absences et à leur historique ; rectification par requalification ; « qui a lu ma pièce ? » (journal des lectures, visible de l'agent) ; pas de décision automatisée (R-21).

---

## 19. Intelligence artificielle (contrat IA du socle, `IA_COEUR.md` §5, §6.2)

### 19.1 Ce que l'IA fait [R]

- **Prépare la journée** : à partir de l'impact et des propositions déterministes (§11.2), rédige la synthèse « ce matin » et un **plan de réponse** que l'adjoint valide élément par élément (N2) ; jamais une désignation sans geste.
- **Rédige** les messages aux familles et au remplaçant (N3 pour l'envoi, sur le texte final), **sans aucune nature**, en vérifiant que le texte ne contient pas de mot de la liste d'exclusion (maladie, enfant, décès, grève…) — contrôle **déterministe** en plus.
- **Lit une convocation** (PDF non sensible) déposée par un administratif et propose les plages (NX15-05), relues avant création (N2).
- **Prépare** le compte rendu du plan RCD et l'état mensuel (N1/N2), à partir des agrégats.
- **Explique** : « pourquoi ce remplaçant ? », avec les critères du §11.2 et rien d'autre.

### 19.2 Ce que l'IA ne fait pas [R]

Lire une nature d'absence, une pièce, un fil de messages, un traitement d'heures (**sensibilité santé ou RH : exclu du contexte**, IA_COEUR §7.2) ; valider, refuser, requalifier une absence (AD-08 : la décision n'est pas une capacité IA) ; désigner un remplaçant sans geste humain ; classer les agents ; prédire des absences ; envoyer.

### 19.3 Capacités déclarées (registre `ent-ia`) [R]

| Capacité | Type | Droit | Sensibilité | Confirmation |
|---|---|---|---|---|
| `absences.impact_du_jour` | lecture | `personnels.remplacements.organiser` | organisation (sans nature) | N0 |
| `absences.proposer_reponses` | simulation | idem | organisation | N0 |
| `absences.appliquer_plan` | commande | idem | organisation | **N2** |
| `absences.solliciter` | commande (envoi) | idem | organisation | **N3** |
| `absences.rediger_message_familles` | brouillon | idem | organisation | N0 (envoi N3 par le socle 8) |
| `absences.lire_convocation` | lecture d'une pièce **non sensible** désignée par l'utilisateur | `personnels.absences.saisir` | interne | N2 pour créer |
| `absences.mes_absences` / `mes_heures` | lecture | `moi` | RH de soi (jamais envoyé au modèle au-delà des dates) | N0 |
| `absences.preparer_etat_mensuel` | simulation | `personnels.heures.valider` | interne | N1 |

### 19.4 Cadre [R]

Règlement IA : la proposition de remplaçants est une tâche préparatoire sans incidence significative ni profilage (R-20) si elle respecte R-AP37 ; évaluation art. 6 §4 documentée au lot A13 ; fournisseur et garanties : `IA_COEUR.md` (Mistral prioritaire, sous réserve contractuelle).

---
## 20. Cas limites (catalogue)

### 20.1 Déclaration et circuit

| N° | Situation | Traitement attendu |
|---|---|---|
| CAP-01 | Professeure malade à 6 h 45, déclare depuis son téléphone, ne sait pas encore pour combien de temps | nature `maladie`, fin = aujourd'hui par défaut, « je prolongerai » ; impact du jour aussitôt ; prolongation d'un geste (ABP-110) |
| CAP-02 | Elle téléphone à l'accueil au lieu de déclarer | l'accueil saisit `signalee`, **sans nature** (ABP-102) ; l'agent reçoit « confirmez votre absence » ; l'organisation commence sans attendre |
| CAP-03 | Elle ne confirme jamais (hospitalisée) | après 48 h, la direction peut confirmer pour elle (ABP-112 par délégation), nature `inconnue` → « en attente de motif » (R-02) |
| CAP-04 | Agent à double engagement : maître (État) + surveillances payées par l'OGEC, absent toute la journée | **deux** absences liées, une par engagement, deux circuits (rectorat et comptabilité) ; une seule déclaration côté agent (R-AP4) |
| CAP-05 | Absence d'une demi-journée seulement, l'après-midi | plage 13 h 30–17 h ; seules les séances de l'après-midi sont touchées |
| CAP-06 | Absence discontinue : rendez-vous médicaux chaque lundi matin pendant un mois | une absence, quatre plages (ABP-115) ; temps partiel thérapeutique **jamais** distingué dans l'organisation (R-24 du 10) |
| CAP-07 | Deux déclarations qui se chevauchent (l'agent et l'accueil) | proposition de fusion à l'agent ou au valideur (ABP-108) ; **jamais** de fusion silencieuse (AD-06) |
| CAP-08 | Le valideur désigné est l'absent (le directeur adjoint absent valide les absences des surveillants) | remonte d'un cran : chef d'établissement (R-AP10) |
| CAP-09 | Le chef d'établissement est absent | valideur = son délégataire désigné (socle 6, P6) ; à défaut, la règle Q-AP6 (président de l'OGEC pour un salarié, autorité académique informée pour un agent de l'État) |
| CAP-10 | Le valideur est lui-même absent trois semaines | sa file passe à son délégataire (ABP-132) ; circuit figé inchangé, trace |
| CAP-11 | Absence prévue à six mois (congé de maternité) | saisie dès l'annonce ; impact calculé sur l'EDT en vigueur, **recalculé** à chaque nouvelle version d'EDT (ABP-151) ; suppléance proposée (§11.4) |
| CAP-12 | Absence saisie sur le mauvais agent (homonyme) | annulation (ABP-113), réponses proposées à la levée, notifications de rétablissement ; nouvelle absence sur le bon agent ; journal |
| CAP-13 | Nature changée : « convenance » saisie, c'était un enfant malade | requalification (ABP-128) ; traitement recalculé ; refus devenu impossible ; rien d'autre ne change côté organisation |
| CAP-14 | Absence refusée (convenance pendant une semaine de conseils) mais l'agent est quand même absent | l'absence reste `refusee` ; l'impact existe ; heures qualifiées « non autorisée » ; retenue signalée **sur décision humaine** (R-AP15) |
| CAP-15 | Arrêt reçu au quatrième jour (délai de 48 h dépassé) | date de réception notée (ABP-131) ; alerte au secrétariat ; ScolIA ne sanctionne rien (R-08 : décision du rectorat) |
| CAP-16 | Retard de 40 minutes le matin (train) | nature `retard` ; la première séance touchée seulement ; sans incidence par défaut (NX15-35) |
| CAP-17 | Retard qui dure 5 heures | au-delà de 4 h, la nature est requalifiée en absence (R-AP14) |
| CAP-18 | Convocation d'examen importée en PDF : un jury sur trois jours, heures variables | OCR proposé, une plage par jour (NX15-07), relue et confirmée par l'administratif (N2) |
| CAP-19 | Convocation d'un professeur à une formation sur ses heures de cours | alerte « tombe sur 6 h de cours » (R-AP35) ; validation possible ; proposition de déplacement des séances |
| CAP-20 | Absence d'un intervenant sans compte (vacataire, contractuel pas encore activé) | absence sur l'**agent** (D-020) saisie par le secrétariat (ABP-116) ; il est prévenu par courriel si une coordonnée existe |
| CAP-21 | Agent qui part de l'établissement pendant son arrêt (fin de contrat) | l'absence s'arrête à la fin d'engagement ; impact au-delà nul ; dossier transmis puis purgé selon §18.4 |
| CAP-22 | Décès d'un agent | aucune notification automatique à l'agent, aucune relance (socle 6, P7) ; la direction clôt ; services en suppléance ou vacants |
| CAP-23 | Suspension conservatoire (socle 6, P9) | pas une absence RH : l'engagement est suspendu ; les séances sont « non assurées » ; **aucune** nature, même côté RH du 15 ; élèves et familles voient le remplacement seulement |
| CAP-24 | Agent en congé long qui veut garder un œil sur sa classe | accès selon le socle 6 (P10) ; notifications du 15 en pause (sollicitations, relances non urgentes) |

### 20.2 Impact et réponse

| N° | Situation | Traitement attendu |
|---|---|---|
| CAP-25 | Absence d'un co-enseignant (deux professeurs sur la séance) | séance non touchée si l'autre est présent (R-AP25) ; information du co-enseignant |
| CAP-26 | Absence le jour d'une sortie de toute la classe | séance `sans_objet` (déjà neutralisée par le 42) |
| CAP-27 | Absence pendant les vacances | aucun impact ; l'absence existe pour la paie (OGEC) |
| CAP-28 | Professeur absent le matin d'un devoir surveillé de 2 h | proposition : collègue libre pour surveiller (remplacement « surveillance »), ou déplacement du devoir (socle 21 prévenu) |
| CAP-29 | Le seul remplaçant possible a cours ailleurs à 10 h 55, la séance finit à 10 h 55 | chevauchement nul : proposé ; trajet entre sites signalé si deux sites (socle 5) |
| CAP-30 | Deux adjoints désignent en même temps deux remplaçants sur la même séance | une seule réponse active (contrainte en base) ; le second reçoit « déjà traité par … » |
| CAP-31 | Le même remplaçant désigné sur deux séances simultanées par deux personnes | contrainte d'exclusion sur ses plages acceptées (R-AP19, AR-09) |
| CAP-32 | Un remplaçant accepte puis tombe malade | sa propre absence touche **sa séance de remplacement** (impact en cascade) ; la séance repasse `a_traiter` |
| CAP-33 | Volontaire RCD qui refuse sur son créneau sans motif | refus impossible sans motif légitime (R-01) ; « autre » demande un texte court relu, visible de l'adjoint seulement |
| CAP-34 | Volontaire sollicité 10 minutes avant le cours alors que le plan prévoit 24 h de prévenance | proposé « hors délai, sur accord seulement » ; refus libre dans ce cas (R-AP36) |
| CAP-35 | Le titulaire revient plus tôt que prévu, le remplaçant a préparé | réponses à lever **proposées** à l'adjoint, pas levées seules ; le remplaçant est prévenu ; si la séance a lieu avec le titulaire, aucune heure pour le remplaçant |
| CAP-36 | Absence longue, deux premières semaines en RCD, puis suppléant | impacts des deux premières semaines en réponses de séance ; à la date de suppléance, les suivants passent `supplee` |
| CAP-37 | Suppléant nommé mais pas encore arrivé (procès-verbal d'installation, socle 6 P50) | séances `a_traiter` jusqu'à l'installation ; l'état du remplacement affiche « à partir du … » |
| CAP-38 | Absence d'un professeur dont le service est déjà vacant | rien à remplacer de plus ; l'impact existe déjà comme vacance (socle 6) |
| CAP-39 | Séance remplacée par un collègue d'une autre discipline qui fait **son** cours | séance d'origine annulée, séance du cours du remplaçant ajoutée (CL-12 du 10) ; famille prévenue du changement de matière |
| CAP-40 | Séance annulée en fin de journée pour une classe de 6ᵉ : certains externes sont autorisés à sortir | sortie anticipée **seulement** pour les autorisés **et** si la famille a été prévenue avant (R-AP24) ; les autres en permanence |
| CAP-41 | Séance annulée en milieu de matinée | jamais de sortie ; permanence ; effectif attendu à la vie scolaire |
| CAP-42 | Absence d'une AESH | impact sur les séances de l'élève accompagné ; prévenus : professeur, vie scolaire, référent ; famille selon PAP/PPS ; aucune réponse automatique |
| CAP-43 | Absence d'un AED : le poste de surveillance de cour 10 h n'a qu'un adulte | impact « poste » ; alerte sous le minimum ; réponse : échange de poste (socle 6) |
| CAP-44 | Accompagnateur d'une sortie (socle 42) qui a cours ce jour-là | indisponibilité de service ; séances proposées au remplacement, l'organisateur de la sortie est l'auteur |
| CAP-45 | Professeur convoqué en conseil de discipline comme témoin | indisponibilité de service (`autre_service`) |
| CAP-46 | Absence à cheval sur une nouvelle version d'EDT | impact recalculé à la publication de la version ; les réponses des séances qui ont gardé leur identité sont conservées, les autres signalées |
| CAP-47 | Séance passée laissée `a_traiter` | devient `non_assure` à son heure de fin ; comptée dans les heures non assurées ; la vie scolaire voit si l'appel de permanence a été fait |
| CAP-48 | Correction d'une séance passée : en réalité un collègue l'avait prise | correction (ABP-188), droit spécial, motif ; heure constatée créée ; effets signalés |
| CAP-49 | Remplacement par le chef d'établissement lui-même (privé, R-18) | possible s'il a un engagement d'enseignement ou une habilitation « encadrement » ; pas d'heure payée sauf paramétrage |
| CAP-50 | Remplaçant sans compte (vacataire d'étude) | désignation sur l'agent ; liste d'appel imprimée ou lien nominatif à usage limité (socle 7, C) ; appel ressaisi par la vie scolaire |

### 20.3 Familles, heures, données

| N° | Situation | Traitement attendu |
|---|---|---|
| CAP-51 | Parents séparés, l'un des deux a demandé à ne pas être sollicité (« ne pas solliciter », D-028) | une modification d'horaire est une **information due** : elle part selon la décision de la direction pour cette personne (D-028) |
| CAP-52 | Élève majeur | prévenu lui-même ; ses parents selon son choix (socle 2) |
| CAP-53 | Interne | aucune sortie anticipée ; l'internat (41) voit la permanence |
| CAP-54 | Famille qui appelle l'accueil : « pourquoi Mme M. est-elle absente ? » | l'accueil voit « absente », jamais la nature (R-AP2) ; réponse type proposée : « nous ne communiquons pas les raisons d'absence » |
| CAP-55 | Cinq jours d'absence non remplacée en mathématiques en 3ᵉ, la famille se plaint | elle voit le compte des séances non assurées (S-1510) et l'état du remplacement ; la direction voit le même chiffre |
| CAP-56 | Remplaçant fait la séance mais oublie l'appel | heure `constatee` en attente d'attestation (ABP-202) ; relance à la fin de la journée |
| CAP-57 | Heure de remplacement contestée par l'agent (« j'ai fait 2 h, pas 1 h ») | contestation avec motif ; l'adjoint valide ou corrige ; trace |
| CAP-58 | Erreur découverte après l'export de l'état mensuel | régularisation sur l'état suivant (ABP-208) |
| CAP-59 | Salarié de l'OGEC en arrêt de 3 jours | file DSN J+5 (R-16) ; éléments variables du mois ; maintien de salaire selon CCN (Q-AP9) |
| CAP-60 | Agent qui demande « qui a vu mon arrêt ? » | journal des lectures de ses pièces (ABP-144) |
| CAP-61 | Un administrateur technique veut lire les pièces | refusé : il n'est pas dans le cercle RH (§18.1) ; accès infrastructure hors application assumé (D-041) |
| CAP-62 | Export nominatif des absences demandé par le président de l'OGEC | réservé, double validation (D-041 proposé), tracé ; nature codée seulement pour les salariés de l'OGEC, rien sur les agents de l'État |
| CAP-63 | Purge d'une pièce encore utile (contentieux en cours) | gel de conservation posé par la direction (motif, date de fin), signalé au DPO |
| CAP-64 | Grève : un professeur fait cours puis rejoint la grève l'après-midi | plage de l'après-midi seulement ; retenue 1/30ᵉ signalée pour la journée (règle de l'État, R-11) — décision et calcul au rectorat |
| CAP-65 | Grève : une collègue demande « qui est gréviste ? » | aucune liste pour les collègues (R-AP2) |
| CAP-66 | Changement d'heure (passage à l'heure d'hiver) pendant une absence | instants en UTC, dates en fuseau de l'établissement ; aucune heure perdue ou doublée |
| CAP-67 | Établissement d'outre-mer (fuseau différent) | fuseau de l'établissement (R-P2 du 6) |
| CAP-68 | Agent présent dans deux établissements ScolIA (complément de service) | une absence par établissement (D-013 : un agent par établissement) ; proposition de reporter la déclaration dans l'autre (geste de l'agent, aucune donnée partagée entre établissements) |
| CAP-69 | Absence posée par l'IA d'après une convocation mal lue (mauvaise date) | rien n'est créé sans relecture (N2) ; si validé à tort : annulation et journal |
| CAP-70 | Bascule depuis Next.js : absence en cours, validée dans Next, pas encore traitée par la comptabilité | reprise dans l'état `validee`, circuit reconstruit, pièces copiées (§27) ; Next.js n'est plus la source (propriété unique, R-AP43) |
| CAP-71 | Absence de Next.js au motif anonymisé (« Absence ») | reprise en nature `inconnue` classée « reprise anonymisée », jamais relancée |
| CAP-72 | Agent de Next.js introuvable dans le socle 6 | file « à décider » de la reprise (§27) ; aucune absence orpheline créée |

---

## 21. Actions métier (ABP-100 à ABP-271)

Acteurs : **AGT** l'agent lui-même · **COL** un collègue qui saisit pour lui · **ACC** accueil · **ADJ** adjoint ou chargé des remplacements · **DIR** chef d'établissement · **VAL** valideur du circuit · **INS** instructeur (secrétariat, comptabilité) · **CPT** comptabilité de l'OGEC · **VS** vie scolaire, CPE · **PP** professeur principal · **RPL** remplaçant · **FAM** familles · **ELV** élèves · **DPO** · **ADM** administrateur de l'établissement · **SYS** système. ★ = ré-authentification récente (passkey). Colonne « Origine » : NX15 (Next.js), RH (SOCLE2_DOMAINES D19), EDT (socle 10), nouveau.

### 21.1 Déclarer (ABP-100 à ABP-119)

| N° | Action | Acteurs | Règles et effets | Origine |
|---|---|---|---|---|
| ABP-100 | Déclarer sa propre absence (nature, début, fin ou « je prolongerai », plages) | AGT (web, mobile) | nature dans la liste de son engagement ; impact calculé ; file du valideur | NX15-01, RH-02 |
| ABP-101 | Déclarer pour un collègue | COL (secrétariat, direction, comptabilité) | collègue choisi dans l'annuaire du 6 ; auteur ≠ absent tracé ; l'agent est prévenu et confirme | NX15-02 |
| ABP-102 | Saisir une absence **signalée** par téléphone | ACC | **sans nature** ; transmis à l'ADJ et à la VS (organisation) ; l'agent confirme | NX15-03 |
| ABP-103 | Constater l'absence d'un agent non prévenu (il n'est pas là à 8 h 05) | ADJ, VS, DIR | `signalee`, nature `inconnue` ; l'agent est contacté | nouveau |
| ABP-104 | Saisir une absence **déjà validée** (régularisation, convocation reçue) ★ | INS, DIR | droit réservé (AD-05) ; circuit marqué « saisie validée » | NX15-04 |
| ABP-105 | Importer une **convocation** PDF (OCR, plages proposées) | INS | pièce non sensible seulement ; relecture N2 ; une plage par jour (R-AP31) | NX15-05, NX15-06 |
| ABP-106 | Choisir l'engagement concerné (double engagement) | AGT | une absence par engagement touché (R-AP4) | NX15-08 |
| ABP-107 | Exprimer une préférence de traitement (rattrapage, sans solde…) | AGT | proposée au valideur ; jamais imposée | NX15-11 |
| ABP-108 | Être averti d'un chevauchement et choisir : fusionner, garder les deux, annuler l'une | AGT, VAL | proposition seulement (AD-06) | NX15-13 |
| ABP-109 | Déclarer un retard | AGT, ACC | ≤ 4 h, même jour ; au-delà requalifié (R-AP14) | NX15-35 |
| ABP-110 | Prolonger (nouvel arrêt, nouvelle fin) | AGT, INS | fait `prolongee` ; impact étendu ; nouvelle transmission | NX15-30 |
| ABP-111 | Écourter (reprise anticipée) | AGT, ADJ | fait `ecourtee` ; réponses à lever proposées (CAP-35) | NX15-30 |
| ABP-112 | Confirmer une absence signalée | AGT ; DIR par délégation après 48 h | `declaree` | nouveau |
| ABP-113 | Annuler une absence (erreur, n'a pas eu lieu) | AGT (avant le début), VAL, DIR | rien supprimé ; impact retiré ; levées proposées | AR-16 |
| ABP-114 | Déclarer à l'avance une absence prévue (formation, convocation, congé) | AGT, INS | alerte si sur heures de cours (R-AP35) | nouveau |
| ABP-115 | Déclarer une absence **discontinue** (plusieurs plages) | AGT, INS | une absence, N plages | nouveau |
| ABP-116 | Déclarer l'absence d'un agent **sans compte** | INS, DIR | sur l'agent (D-020, AR-19) | nouveau |
| ABP-117 | Déclarer sans préciser la nature (« je préciserai ») | AGT | nature `inconnue`, relance à J+2 | nouveau |
| ABP-118 | Déposer le **travail prévu** pour les séances touchées | AGT | lien vers le cahier de textes (20) ; visible du remplaçant et des élèves | nouveau |
| ABP-119 | Poser sa **réponse d'absence** de messagerie depuis la déclaration | AGT | port du 16 (MSG-180), sans nature, redirection proposée | nouveau |

### 21.2 Instruire et décider (ABP-120 à ABP-139)

| N° | Action | Acteurs | Règles et effets | Origine |
|---|---|---|---|---|
| ABP-120 | Voir le **circuit** d'une absence (valideur, instructeur, étape suivante) | AGT, VAL, INS, DIR | circuit figé à la création (§9.4) | NX15-14 |
| ABP-121 | **Valider** avec le traitement des heures | VAL | jamais l'absent (R-AP10) ; traitement imposé si non discrétionnaire (R-AP12) ; clôture automatique si sans incidence (R-AP13) | NX15-17, RH-03 |
| ABP-122 | **Refuser** (motif codé, note) | VAL | seulement si la nature le permet (R-AP12) ; l'agent est prévenu (manque de Next.js) | NX15-18 |
| ABP-123 | Demander une pièce ou une précision | VAL, INS | `en_instruction`, relance datée | NX15-25 |
| ABP-124 | Valider **en lot** des absences non sensibles (convocations, formations) | VAL | aperçu ; aucune nature sensible dans un lot | nouveau |
| ABP-125 | **Traiter** administrativement (transmis au rectorat, à la paie) | INS | `traitee` ; date et voie de transmission ; purge programmée | NX15-21 |
| ABP-126 | Clôturer | INS, VAL | interdit si un traitement est dû (R-AP13) | NX15-22 |
| ABP-127 | **Rouvrir** | VAL, DIR | liste des effets à défaire présentée, appliqués après confirmation (R-AP17) | NX15-23 |
| ABP-128 | **Requalifier** la nature | VAL, INS | traitement recalculé ; corrections transmises | NX15-29 |
| ABP-129 | Corriger l'engagement concerné | INS | circuit recalculé, ancien circuit gardé au journal | NX15-28 |
| ABP-130 | Qualifier une absence **injustifiée** (service non fait) ★ | DIR | signalé à l'export ; l'agent est prévenu et peut répondre dans le fil | nouveau |
| ABP-131 | Noter la **date de réception** de l'arrêt | INS | échéance 48 h affichée (R-AP16) | nouveau |
| ABP-132 | **Déléguer** sa file de validation pour une période | VAL | délégation du socle 6 (P6) ; trace | nouveau |
| ABP-133 | Reprendre une absence en souffrance chez un valideur | DIR | après N jours (S-1501) ; trace | nouveau |
| ABP-134 | Voir l'**historique** complet d'une absence | AGT, VAL, INS, DIR | journal en ajout seul (R-AP20) | NX15-32 |
| ABP-135 | Annoter une décision | VAL, INS | note attachée à la décision, jamais écrasée (AD-14) | NX15 |
| ABP-136 | Demander le **déplacement** d'une formation ou d'une réunion qui tombe sur des heures de cours | VAL | message à l'agent ; décision humaine | nouveau |
| ABP-137 | Accorder ou refuser une **demande de congé payé** (salarié OGEC) avant la date | VAL | impact simulé (poste du planning) avant décision | nouveau |
| ABP-138 | Signaler un **deuxième retard** de transmission d'arrêt (24 mois) | INS | information au rectorat ; aucune sanction calculée (R-08) | nouveau |
| ABP-139 | Constater la **reprise** (retour effectif) | AGT, ADJ | fin réelle ; impacts suivants retirés | nouveau |

### 21.3 Pièces, fil, impact, confidentialité (ABP-140 à ABP-164)

| N° | Action | Acteurs | Règles et effets | Origine |
|---|---|---|---|---|
| ABP-140 | **Déposer une pièce** | AGT, INS | `ent-object` (E5) ; avertissement volet 1 (R-08) ; antivirus | NX15-24, NX15-43 |
| ABP-141 | Relancer la pièce | INS, SYS | relance datée ; plafond de relances (S-1502) | NX15-25 |
| ABP-142 | **Lire une pièce** | cercle RH | URL de 5 min ; lecture journalisée | NX15-44 |
| ABP-143 | Retirer une pièce déposée par erreur | AGT (avant traitement), INS | état `retiree` ; purge de l'objet au cycle de purge | NX15-45 |
| ABP-144 | Voir **qui a lu** mes pièces | AGT | journal des lectures | nouveau |
| ABP-145 | Écrire dans le **fil** de l'absence | AGT, VAL, INS | ajout seul, 4 000 car. | NX15-31 |
| ABP-146 | Lire le fil | cercle RH | | NX15-31 |
| ABP-147 | Voir ses **échéances** (48 h, DSN J+5, pièces, créneaux) | INS, CPT | file datée | nouveau |
| ABP-148 | Voir **mes absences** et leur historique | AGT | portée `moi` | nouveau |
| ABP-149 | Exporter mes propres données d'absence | AGT | portabilité de soi (socle 27) | nouveau |
| ABP-150 | Voir l'**impact** d'une absence (séances, postes, engagements) | ADJ, DIR, VS ; AGT pour soi | sans nature | NX15-33, RH-04 |
| ABP-151 | **Recalculer** l'impact (nouvelle version d'EDT, changement de service) | SYS | réponses conservées sur les séances stables (CAP-46) | nouveau |
| ABP-152 | Voir l'impact sur les **postes** du planning | ADJ, VS, gestionnaire | socle 6 P15 | nouveau |
| ABP-153 | Voir l'impact sur les **engagements** d'autres socles | ADJ, DIR | 19, 23, 38, 42 | nouveau |
| ABP-154 | Voir l'impact **AESH** | VS, référent | séances des élèves accompagnés, sans motif | nouveau |
| ABP-155 | Savoir si un agent est **présent** (oui / non / inconnu) | ACC, personnels | port `present`, jamais la nature ni le lieu | EDT-110 |
| ABP-156 | Voir les **absents du jour** (organisation) | ADJ, VS, DIR | sans nature | nouveau |
| ABP-157 | Voir les séances non assurées ou remplacées **de sa classe** | PP | résumé du jour | EDT §14.4 |
| ABP-158 | Voir **ma séance de remplacement** (liste, salle, travail prévu, aménagements nécessaires) | RPL | délégation de la séance seulement | nouveau |
| ABP-159 | Au retour, voir **ce qui a été fait** pendant mon absence (séances assurées, travail donné, incidents de la classe) | AGT | lecture des socles 10, 12, 13, 20 sur ses services | nouveau |
| ABP-160 | **Purge** programmée des pièces et du fil | SYS | catégorie de conservation (§18.4) ; journal | NX15-47 |
| ABP-161 | Purger une pièce à la demande | DPO ★ | motif ; journal | nouveau |
| ABP-162 | Voir les **purges à venir** | DPO, DIR | | nouveau |
| ABP-163 | Poser un **gel de conservation** (contentieux) | DIR ★ | motif, date de fin ; signalé au DPO | nouveau |
| ABP-164 | Marquer une absence « **confidentialité renforcée** » | DIR, AGT (demande) | niveau organisation réduit à l'ADJ et à la DIR ; la VS voit « séance à couvrir », pas le nom | nouveau |

### 21.4 Organiser la réponse (ABP-165 à ABP-189)

| N° | Action | Acteurs | Règles et effets | Origine |
|---|---|---|---|---|
| ABP-165 | Tableau **« ce matin »** | ADJ, DIR, VS | §10.4 | EDT §14.3 |
| ABP-166 | Tableau **« demain »** | ADJ, DIR, VS | généré à 18 h, continu | EDT §14.3 |
| ABP-167 | Voir les **propositions** de réponse pour chaque impact, avec leurs raisons | ADJ | §11.2 | nouveau |
| ABP-168 | **Appliquer un plan** de réponses en lot (cases à décocher) | ADJ | aperçu élément par élément ; une transaction par impact, échecs listés | nouveau |
| ABP-169 | **Désigner** un remplaçant (urgence, sans sollicitation) | ADJ | contrôle de disponibilité en transaction (R-AP19) ; accusé attendu | EDT-167, RH-04 |
| ABP-170 | File **à valider** | VAL | par échéance | NX15-53 |
| ABP-171 | File **à traiter** | INS, CPT | par échéance | NX15-53 |
| ABP-172 | Suivi « **prochaine étape** » d'une absence (qui doit agir) | AGT, VAL, INS | | NX15-54 |
| ABP-173 | **Calendrier** des absences (organisation) | ADJ, DIR | sans nature | NX15-55 |
| ABP-174 | Compteurs du tableau de bord (à valider, à traiter, séances à couvrir aujourd'hui) | DIR, ADJ | | NX15-56, AR-22 |
| ABP-175 | **Solliciter** un remplaçant | ADJ | §11.3 ; expiration (S-1505) | nouveau |
| ABP-176 | **Accepter** une sollicitation | RPL (mobile) | réservation de la plage en base | nouveau |
| ABP-177 | **Refuser** une sollicitation (motif légitime si RCD sur créneau) | RPL | R-01 | nouveau |
| ABP-178 | **Accuser réception** d'une désignation | RPL | sans accusé à H-1 : alerte VS (EDT-238) | EDT-238 |
| ABP-179 | Remplacer par un **autre cours** | ADJ | CL-12 du 10 | EDT-166 |
| ABP-180 | **Demander une suppléance** au rectorat (absence longue) | DIR, INS | trace de la demande, de la réponse, du suppléant proposé (R-18) | nouveau |
| ABP-181 | **Nommer un suppléant** sur le service | DIR | port `nommer_suppleant` (6, PRS-46), origine = absence | PRS-46 |
| ABP-182 | **Déplacer** une séance | ADJ | modification `deplacement` (10) | EDT |
| ABP-183 | Organiser une **continuité encadrée** (séquence numérique, devoir surveillé, CDI) | ADJ, VS | R-01 ; encadrant au planning | nouveau |
| ABP-184 | **Annuler** une séance et prévoir la prise en charge (permanence, étude, sortie selon §13) | ADJ | modification `annulation` (10) ; effectif à la VS | RH-05 |
| ABP-185 | Lancer un **appel à volontaires** pour une séance | ADJ | plusieurs destinataires, premier accord retenu, autres prévenus | nouveau |
| ABP-186 | **Lever** une réponse (rétablir) | ADJ | fait daté ; délégation révoquée (AR-18) ; familles prévenues si l'heure change | AR-15 |
| ABP-187 | **Contester** une désignation (agent) | RPL | motif ; l'ADJ tranche ; trace (R-21) | nouveau |
| ABP-188 | **Corriger** une séance passée | DIR, ADJ (droit spécial) ★ | effets aval signalés (appel, heures) | EDT §9.6 |
| ABP-189 | Marquer un impact **sans objet** | ADJ, SYS | | nouveau |

### 21.5 Rattrapages (ABP-190 à ABP-199)

| N° | Action | Acteurs | Règles et effets | Origine |
|---|---|---|---|---|
| ABP-190 | Créer le **rattrapage dû** (à la validation « rattrapage ») | SYS | heures depuis l'impact ; échéance (S-1514) | NX15-12 |
| ABP-191 | Relancer les créneaux de rattrapage | SYS, VAL | R-AP27 | NX15-26, NX15-27 |
| ABP-192 | **Proposer** des créneaux de rattrapage | AGT | parmi les plages libres communes | NX15-26 |
| ABP-193 | **Confirmer** des créneaux | VAL | séance `ajoutee` (10) ou plage de planning (6) ; familles prévenues | NX15-12 |
| ABP-194 | Marquer un rattrapage **fait** | SYS (séance assurée), VAL | solde | nouveau |
| ABP-195 | Prolonger l'échéance | VAL | motif | nouveau |
| ABP-196 | Requalifier en retenue | VAL ★ | l'agent est prévenu | nouveau |
| ABP-197 | Abandonner avec motif | DIR | trace | nouveau |
| ABP-198 | Proposer le **rattrapage d'heures d'élèves** perdues (sans dette d'un agent) | ADJ, DIR | proposition au 10 | T-14 |
| ABP-199 | Voir **mes rattrapages dus** | AGT | | nouveau |

### 21.6 Heures et service fait (ABP-200 à ABP-219)

| N° | Action | Acteurs | Règles et effets | Origine |
|---|---|---|---|---|
| ABP-200 | Voir **mes heures** (remplacements, HSE, missions, rattrapages) | AGT | EDT-119 | EDT-119 |
| ABP-201 | **Contester** une heure | AGT | motif | nouveau |
| ABP-202 | **Attester** une séance assurée sans appel | RPL ; VS | l'heure passe `constatee` | nouveau |
| ABP-203 | **Valider** les heures du mois | ADJ | | nouveau |
| ABP-204 | **Figer** l'état mensuel | ADJ, INS | instantané | nouveau |
| ABP-205 | **Signer** l'état mensuel ★ | DIR | signature (socle 7) | nouveau |
| ABP-206 | Exporter l'état des heures vers le **rectorat** | INS | tracé ; format Q-AP5 | EDT-194 |
| ABP-207 | Exporter les **éléments variables** de paie (OGEC) | CPT | natures codées, jamais un libellé sensible (AD-03) ; unicité (R-AP29) | NX15-40 |
| ABP-208 | **Régulariser** après export | INS, CPT | ligne de régularisation (§14.2) | nouveau |
| ABP-209 | Voir les **heures et jours d'absence** d'un salarié pour la paie | CPT | depuis l'impact réel (§14.4) | NX15-41 |
| ABP-210 | File **DSN J+5** | CPT | R-16 | nouveau |
| ABP-211 | Marquer « signalé en DSN le » | CPT | | nouveau |
| ABP-212 | Alerte HSE sur une absence de **convenance personnelle** | SYS → ADJ | R-AP33 | nouveau |
| ABP-213 | Voir le **compteur RCD** d'un volontaire | ADJ ; AGT pour soi | | nouveau |
| ABP-214 | Relevé du **service fait** d'un vacataire | ADJ, CPT | socle 6 PRS-145 | PRS-145 |
| ABP-215 | Alerte **repos de 11 h / pause** pour un salarié qui couvre un poste | SYS → gestionnaire | R-30 du 10 | nouveau |
| ABP-216 | Exporter les **absences à transmettre** au rectorat | INS | nature codée, date de réception | nouveau |
| ABP-217 | Marquer une absence **transmise** | INS | `traitee` | NX15-21 |
| ABP-218 | Compter les heures d'un **remplaçant sans compte** | ADJ | sur l'agent | nouveau |
| ABP-219 | **État annuel** des heures d'un agent | AGT, DIR, INS | | nouveau |

### 21.7 Remplacement de courte durée (ABP-220 à ABP-231)

| N° | Action | Acteurs | Règles et effets | Origine |
|---|---|---|---|---|
| ABP-220 | Créer le **plan RCD** de l'année (trame) | DIR, ADJ | R-01 | nouveau |
| ABP-221 | Versionner le plan | DIR | historique | nouveau |
| ABP-222 | Enregistrer l'**avis du conseil pédagogique** et la **présentation au CA** (ou à l'instance du privé) | DIR | dates, pièce | nouveau |
| ABP-223 | Marquer la **transmission au recteur** | DIR | date | nouveau |
| ABP-224 | Produire un **compte rendu** (au moins deux par an) | DIR | depuis les indicateurs | nouveau |
| ABP-225 | Lancer la **campagne** de volontariat (avec les vœux d'EDT) | ADJ | socle 10 §12 | nouveau |
| ABP-226 | **S'engager** comme volontaire (volume, statut) | AGT | créneaux au 10 (EDT-147) | EDT-147 |
| ABP-227 | Modifier ou mettre fin à son engagement | AGT, ADJ | à date | nouveau |
| ABP-228 | Voir l'**état du vivier** (volumes, consommés, créneaux couverts par jour) | ADJ, DIR | | nouveau |
| ABP-229 | **Préparer** la remontée hebdomadaire | SYS | agrégats sans nom (§12.3) | EDT-198 |
| ABP-230 | **Contrôler** puis laisser partir la remontée | INS, DIR | flux au 9 | EDT-198 |
| ABP-231 | Exporter le **tableau équivalent** (en l'absence d'API) | INS | | nouveau |

### 21.8 Familles, élèves, notifications (ABP-232 à ABP-247)

| N° | Action | Acteurs | Règles et effets | Origine |
|---|---|---|---|---|
| ABP-232 | Notifier le **valideur** d'une nouvelle absence | SYS | sans nature dans le titre (R-AP2) | NX15-49 |
| ABP-233 | Notifier l'**instructeur** d'une absence validée | SYS | lien, jamais pièce jointe (R-AP22) | NX15-50 |
| ABP-234 | Notifier l'**agent** : validée, **refusée**, pièce demandée, traitée | SYS | | NX15-51 |
| ABP-235 | Notifier : créneaux demandés ou proposés, message du fil | SYS | | NX15-52 |
| ABP-236 | Prévenir les **familles** d'un changement d'horaire d'entrée ou de sortie | SYS (depuis la modification du 10) | avant l'heure (R-13) ; deux parents ayant droit ; sans motif | RH-05 |
| ABP-237 | **Résumé du jour** aux familles (remplacements en milieu de journée) | SYS | | EDT §14.4 |
| ABP-238 | Prévenir les **élèves** de tout changement | SYS | | EDT |
| ABP-239 | Publier l'**état du remplacement** d'une absence longue | SYS | une notification par étape (EDT-228) | EDT-228 |
| ABP-240 | Prévenir la **vie scolaire** des effectifs attendus en permanence | SYS | par heure | EDT-219 |
| ABP-241 | Alerter l'ADJ et la VS d'un impact **non traité à H-2** | SYS | R-AP23 | nouveau |
| ABP-242 | Informer le **PP** | SYS | résumé du jour | EDT §14.4 |
| ABP-243 | Informer le **référent** et le professeur de l'absence d'une AESH | SYS | §13.2 | nouveau |
| ABP-244 | Voir les **séances non assurées** de l'année de son enfant, par matière | FAM, ELV | S-1510 | nouveau |
| ABP-245 | Voir **qui assure** le cours | FAM, ELV | nom de l'intervenant (R-19), jamais l'e-mail (AR-04) | EDT |
| ABP-246 | Prévenir du **retour** anticipé du titulaire | SYS | si l'heure change | nouveau |
| ABP-247 | Alerter si le remplaçant **n'a pas accusé** réception à H-1 | SYS | EDT-238 | EDT-238 |

### 21.9 IA (ABP-248 à ABP-252)

| N° | Action | Acteurs | Règles et effets | Origine |
|---|---|---|---|---|
| ABP-248 | Lire une **convocation** et proposer les plages | IA pour INS | N2 ; pièce non sensible | NX15-05 |
| ABP-249 | Rédiger le **plan du matin** | IA pour ADJ | N0 ; application par ABP-168 (N2) | nouveau |
| ABP-250 | Rédiger le **message** aux familles ou au remplaçant | IA pour ADJ | N0 ; envoi N3 ; liste d'exclusion de mots (R-AP41) | nouveau |
| ABP-251 | **Expliquer** une proposition de remplaçant | IA | critères du §11.2 seulement | nouveau |
| ABP-252 | Préparer le **compte rendu** RCD et l'état mensuel | IA pour DIR, INS | N1 | nouveau |

### 21.10 Absences collectives (ABP-253 à ABP-258)

| N° | Action | Acteurs | Règles et effets | Origine |
|---|---|---|---|---|
| ABP-253 | Cocher les **grévistes** du matin | ADJ, DIR | absences `greve` signalées ; nature sensible | EDT-170 |
| ABP-254 | Se déclarer **gréviste** après coup | AGT | | nouveau |
| ABP-255 | Annuler en lot les séances des grévistes | ADJ | motif public « mouvement social » possible | EDT-170 |
| ABP-256 | **Tableau de crise** (beaucoup d'absents) | DIR, ADJ | classes sans cours, bascule en continuité à distance (33) | nouveau |
| ABP-257 | Signaler la **retenue** (1/30ᵉ, État) à l'export | INS | décision et calcul hors ScolIA (R-11) | nouveau |
| ABP-258 | **Fermeture** de l'établissement : neutraliser les absences du jour | SYS (socle 5) | absences non comptées ce jour | nouveau |

### 21.11 Paramétrage, reprise, système (ABP-259 à ABP-271)

| N° | Action | Acteurs | Règles et effets | Origine |
|---|---|---|---|---|
| ABP-259 | Activer ou désactiver une **nature** fournie | ADM | jamais en changer la sensibilité | nouveau |
| ABP-260 | Configurer les **règles de valideurs** (employeur, cycle, nature) | ADM, DIR ★ | priorité au responsable fonctionnel du 6 | NX15-15, NX15-58 |
| ABP-261 | Configurer les **instructeurs** | ADM, DIR ★ | droit dédié (AD-09) | NX15-16, NX15-58 |
| ABP-262 | Régler les paramètres (§28 réglages S-1500 à S-1517) | ADM | | nouveau |
| ABP-263 | Ajouter une nature **non sensible** propre | ADM | catégorie nationale obligatoire | nouveau |
| ABP-264 | **Importer** les absences de Next.js | ADM (Florian) | §27 | nouveau |
| ABP-265 | Lire le **rapport de reprise** | ADM | | nouveau |
| ABP-266 | Rapprocher les agents de Next.js et du socle 6 | ADM | file « à décider » | nouveau |
| ABP-267 | **Basculer** la propriété du module (Next.js → ScolIA) | ADM | R-AP43 | nouveau |
| ABP-268 | Lire le **journal** du domaine | DIR, DPO | | nouveau |
| ABP-269 | **Tâche quotidienne** : expirer les sollicitations, passer `non_assure`, relancer, purger, appliquer les absences futures | SYS | idempotente | nouveau |
| ABP-270 | Exposer les **indicateurs** au pilotage | SYS | agrégés (§14.7) | nouveau |
| ABP-271 | Exporter le domaine pour la **portabilité** de l'établissement | ADM ★★ | D-041, D-046 (proposés) | nouveau |

---

## 22. Règles transverses (R-AP1 à R-AP44) [R]

| N° | Règle |
|---|---|
| R-AP1 | **Pas de motif libre** : une absence porte une **nature** du référentiel ; texte libre seulement pour les natures non sensibles (200 car.). Aucun champ de diagnostic, nulle part. |
| R-AP2 | Une nature **sensible** n'apparaît **jamais** hors du cercle RH (§18.1) : ni dans un événement, un titre ou un corps de notification, un mail, une ligne de paie, un export non RH, un journal applicatif, une réponse d'API du niveau organisation. Test de garde par recherche des libellés dans les sorties. |
| R-AP3 | La projection « organisation » est une vue serveur **sans** colonne de nature ; jamais un filtre d'affichage. |
| R-AP4 | Le circuit est porté par l'**engagement** (socle 6), pas par le rôle ; un agent à deux engagements touchés a deux absences liées, une seule déclaration. |
| R-AP5 | L'absent, le déclarant et l'auteur de la saisie sont trois colonnes distinctes, jamais écrasées (AR-13, AD-07). |
| R-AP6 | Dates en **fuseau de l'établissement** ; instants en UTC ; journée entière = plages de la grille du jour (socle 5), pas 8 h–18 h codées. |
| R-AP7 | Une absence se modifie par **faits** ajoutés (§8.2), jamais par réécriture. |
| R-AP8 | Rien ne s'efface : annuler, lever, retirer = états datés ; la suppression physique n'existe que dans la purge de conservation. |
| R-AP9 | Valideur = responsable fonctionnel de l'agent pour la nature ; à défaut règle de l'établissement ; à défaut chef d'établissement ; **circuit figé** à la création (copie datée). |
| R-AP10 | **Personne ne valide sa propre absence**, ni par délégation ; contrôle en base (`valideur_agent_id <> agent_id`). |
| R-AP11 | Les droits du domaine sont des actions dédiées (`personnels.absences.*`, `personnels.remplacements.*`, `personnels.heures.*`), jamais déduits de `eleves.gerer` (AR-07). |
| R-AP12 | Natures **non discrétionnaires** (maladie, accident, enfant malade, maternité, événement familial de droit, ASA de droit, convocation, représentation dans la limite des droits) : **refus impossible**, traitement imposé. |
| R-AP13 | Clôture automatique si traitement `sans_incidence` ou `rattrapage_interne` réglé ; sinon traitement administratif obligatoire avant clôture. |
| R-AP14 | Retard ≤ 4 h, même jour, sans incidence par défaut ; au-delà, requalification proposée en absence. |
| R-AP15 | Un refus ne fait pas disparaître l'impact : l'agent n'était pas là ; les heures sont qualifiées « non autorisée » ; toute retenue est une **décision humaine**. |
| R-AP16 | La **date de réception** de l'arrêt est notée ; l'échéance de 48 h est affichée à l'instructeur ; ScolIA ne sanctionne pas. |
| R-AP17 | Réouvrir, requalifier, écourter, annuler **recalculent** leurs effets (impact, heures, rattrapage, éléments de paie, transmissions), présentés avant confirmation. |
| R-AP18 | L'impact est **calculé**, jamais saisi ; recalculé à chaque nouvelle version d'EDT, de service ou de planning ; les réponses des séances à identité stable sont gardées. |
| R-AP19 | Un remplaçant n'a **jamais** deux plages acceptées qui se chevauchent (exclusion en base) ; une séance a **au plus une** réponse active. |
| R-AP20 | Journal du domaine en **ajout seul** (déclencheur) ; colonnes typées avant/après, pas de JSON relu (D-036). |
| R-AP21 | Pièces : agent de l'État → purge 30 jours après transmission ; salarié de l'OGEC → selon §18.4 ; nature sensible conservée **codée** ; purges datées et journalisées ; gel possible (contentieux). |
| R-AP22 | Jamais de pièce en pièce jointe d'un mail ni d'une notification ; un lien vers ScolIA seulement. |
| R-AP23 | Un impact `a_traiter` à **H-2** (S-1506) d'une séance déclenche une alerte à l'ADJ et à la VS ; à l'heure de fin, il devient `non_assure`. |
| R-AP24 | **Aucune sortie anticipée** sans information préalable de la famille (R-13) : la sortie n'est proposée que pour la dernière séance de la demi-journée ou de la journée, selon régime et autorisation (14), si la famille a été prévenue dans le délai (S-1508). |
| R-AP25 | Co-enseignement : la séance n'est touchée que si **aucun** autre intervenant en saisie n'est présent. |
| R-AP26 | Une réponse de séance passe par le **port du socle 10** dans la transaction du 15 ; le 15 n'écrit jamais dans les tables du 10 ni du 6. |
| R-AP27 | Rattrapage dû sans créneau proposé à J+10 (S-1515) : relance automatique ; à l'échéance : décision de la direction. |
| R-AP28 | Une heure constatée naît d'un **fait** (séance assurée, appel fait ou attestation) ; jamais d'une déclaration seule. |
| R-AP29 | Éléments de paie : un par (absence ou heure, mois, nature) — unicité en base ; file d'envoi rejouable ; une période fermée reçoit une régularisation, jamais une réécriture. |
| R-AP30 | Salarié de l'OGEC en arrêt : échéance **DSN J+5** dans la file de la comptabilité. |
| R-AP31 | Convocation importée : une plage par jour (fuseau de l'établissement), relue avant création. |
| R-AP32 | Une indisponibilité de service (sortie, jury, réunion) n'a **ni nature RH ni validation RH** ; son auteur est le socle source. |
| R-AP33 | HSE ou RCD pour remplacer une absence de **convenance** : alerte, pas de blocage (R-06 [H]). |
| R-AP34 | Volontaire RCD : refus sur son créneau seulement avec **motif légitime** codé ; collègue hors RCD : refus libre et **sans motif**. |
| R-AP35 | Formation ou réunion sur heures de cours : alerte au valideur ; décision humaine. |
| R-AP36 | Délai de prévenance du plan RCD : un candidat hors délai est proposé « sur accord seulement » ; son refus est alors libre. |
| R-AP37 | Proposition de remplaçant : **jamais** âge, ancienneté, sexe, santé, situation familiale, temps partiel (thérapeutique ou non), historique d'absences ; seulement disponibilité, volontariat, discipline, connaissance du public, présence sur site, équité de sollicitation. |
| R-AP38 | Aucun indicateur **nominatif** d'absentéisme ; indicateurs agrégés, effectifs < 5 masqués. |
| R-AP39 | Les sollicitations respectent la plage calme, sauf la fenêtre de service 6 h 30–20 h pour une séance du jour ou du lendemain matin. |
| R-AP40 | L'IA ne lit **jamais** une nature, une pièce, un fil ou un traitement d'heures ; elle ne valide, ne refuse, ne désigne, n'envoie rien sans geste humain. |
| R-AP41 | Tout texte sortant vers les familles ou les collègues passe un **contrôle déterministe** de mots exclus (libellés de natures sensibles, liste fournie) avant envoi ; un mot trouvé bloque l'envoi et demande une reformulation. |
| R-AP42 | Une absence d'un établissement n'est **jamais** lisible d'un autre, même pour le même agent (D-013). |
| R-AP43 | Propriété unique : à la bascule, une absence est soit dans Next.js, soit dans ScolIA, jamais dans les deux (même principe que R-SV37 du socle 42). |
| R-AP44 | Le remplaçant d'une séance voit **la séance seulement** (liste, aménagements nécessaires, travail), le jour même ; aucune autre donnée de la classe (fin de la fenêtre de ±14 jours, D12 du 10). |

---
## 23. Droits et minimisation

### 23.1 Catalogue d'actions proposé (D-017) [R]

| Action D-017 | Portées possibles | Sens | ★ |
|---|---|---|---|
| `personnels.absences.declarer` | `moi` ; `etablissement` (pour autrui) | ABP-100, 101, 106 à 119 | — |
| `personnels.absences.signaler` | `etablissement` | ABP-102, 103 (sans nature) | — |
| `personnels.absences.presence` | `etablissement` | ABP-155 (oui / non / inconnu) | — |
| `personnels.absences.lire_organisation` | `etablissement` ; `mes_classes` (PP) | ABP-150 à 157, 165, 166, 173 | — |
| `personnels.absences.lire_nature` | `moi` ; `circuit` (valideur ou instructeur désigné) ; `etablissement` (chef d'établissement) | nature, décisions, fil, traitement | — |
| `personnels.absences.valider` | `circuit` | ABP-121 à 139 | — |
| `personnels.absences.instruire` | `circuit` | ABP-123, 125, 126, 131, 138, 216, 217 | — |
| `personnels.absences.saisir_validee` | `etablissement` | ABP-104 | ★ |
| `personnels.absences.pieces` | `moi` ; `circuit` | ABP-140 à 144 | — |
| `personnels.absences.parametrer` | `etablissement` | ABP-259 à 263 | ★ |
| `personnels.absences.conservation` | `etablissement` | ABP-161 à 163 | ★ |
| `personnels.remplacements.organiser` | `etablissement` ; `sites_designes` | ABP-165 à 189 (sauf 176 à 178, 187) | — |
| `personnels.remplacements.repondre` | `moi` | ABP-176, 177, 178, 187, 158 | — |
| `personnels.remplacements.corriger_passe` | `etablissement` | ABP-188 | ★ |
| `personnels.heures.lire` | `moi` ; `etablissement` | ABP-200, 213, 219 | — |
| `personnels.heures.valider` | `etablissement` | ABP-201 à 204, 208 | — |
| `personnels.heures.signer` | `etablissement` | ABP-205 | ★ |
| `personnels.heures.exporter` | `etablissement` (administratifs seulement, D-022) | ABP-206, 207, 216 | ★ |
| `personnels.paie.absences` | `etablissement` (employeur OGEC) | ABP-207, 209 à 211 | — |
| `personnels.rcd.gerer` | `etablissement` | ABP-220 à 228 | — |
| `personnels.rcd.remontee` | `etablissement` | ABP-229 à 231 | — |
| `personnels.absences.reprise` | `etablissement` | ABP-264 à 267 | ★ |

### 23.2 Modèles de rôles par défaut [R]

| Modèle | Droits |
|---|---|
| Tout personnel | `declarer` (moi), `presence`, `remplacements.repondre` (moi), `heures.lire` (moi), `lire_nature` (moi), `pieces` (moi) |
| Accueil | + `signaler` |
| Vie scolaire, CPE | + `lire_organisation`, `remplacements.organiser` pour la continuité encadrée et la permanence (S-1509 : ouvrir l'organisation complète au CPE) ; **jamais** `lire_nature` hors de son circuit (valideur des AED) |
| Professeur principal | + `lire_organisation` (`mes_classes`) |
| Adjoint, chargé des remplacements | + `lire_organisation`, `remplacements.organiser`, `heures.lire`, `heures.valider`, `rcd.gerer` ; `valider` selon le circuit |
| Secrétariat de direction (agents de l'État) | + `declarer` (établissement), `instruire` (circuit État), `saisir_validee`, `heures.exporter`, `rcd.remontee` |
| Comptabilité de l'OGEC | + `declarer` (établissement), `instruire` (circuit OGEC), `paie.absences`, `heures.exporter` |
| Chef d'établissement | tout, sauf `conservation` (DPO) ; `lire_nature` sur l'établissement |
| Administrateur technique | `parametrer`, `reprise` ; **jamais** `lire_nature` ni `pieces` (CAP-61) |
| DPO | `conservation`, journal ; `lire_nature` **non** (il contrôle les accès, pas les contenus) [R, Q-AP13] |

### 23.3 Ce que voit chacun [R]

| Qui | Voit | Ne voit jamais |
|---|---|---|
| Collègue professeur | qu'un collègue est absent **s'il en a besoin** (co-enseignant, remplaçant, PP de la classe) ; la séance qu'il remplace | la nature ; la liste des absents de l'établissement ; les grévistes |
| Accueil | présent / absent / inconnu ; les absences qu'il a saisies, jusqu'à confirmation | la nature ; l'historique |
| Vie scolaire | impacts, réponses, effectifs, remplaçants | la nature (sauf circuit des AED) |
| Famille, élève | séances changées, intervenant, état du remplacement, compte des séances non assurées | nom de l'absent dans un message, nature, e-mail d'un personnel |
| Comptabilité de l'OGEC | absences des **salariés de l'OGEC** (nature codée, jours, heures, pièces du circuit) | absences des agents de l'État (sauf l'engagement OGEC d'un agent à double engagement) |
| Secrétariat | absences des agents de l'État de son circuit | celles des salariés de l'OGEC hors circuit |

### 23.4 Pièges de minimisation à tester [R]

1. Une absence d'un professeur à double engagement ne montre à la comptabilité que la partie OGEC.
2. Le titre d'une notification push ne contient ni nature ni nom de l'absent (socle 16, D4).
3. Le DTO de `GET /…/organisation` n'a **pas** de champ nature (test de structure, pas seulement de valeur).
4. Le journal applicatif (`ent-obs`) ne contient aucune nature (E14).
5. L'export SI-RCD ne contient aucun identifiant de personne.
6. Une recherche plein texte du personnel ne trouve pas une absence par sa nature.
7. Le remplaçant d'une séance ne voit pas la séance suivante de la même classe.
8. L'IA reçoit « absent » et jamais la nature (fiche de contexte, test d'équivalence).

---

## 24. Événements et notifications

### 24.1 Émis (outbox, dans la transaction du fait) [R]

| Événement | Charge utile (jamais de nature ni de texte libre) | Consommateurs |
|---|---|---|
| `absence_personnel.signalee` / `.declaree` | `absence_id`, `agent_id`, plages, `cause` (rh ou service) | 10 (indisponibilités), 16 (réponse d'absence), 14 (présence), 29 |
| `absence_personnel.validee` / `.refusee` / `.traitee` / `.close` | `absence_id`, `agent_id`, état | 49, 29 |
| `absence_personnel.modifiee` (prolongée, écourtée, plage) | `absence_id`, anciennes et nouvelles plages | 10, 16, 14 |
| `absence_personnel.requalifiee` | `absence_id` seulement | 49 |
| `absence_personnel.annulee` | `absence_id` | 10, 16, 14, 49 |
| `absence_impact.a_traiter` / `.traite` / `.non_assure` | `impact_id`, `seance_id` ou `poste_id`, état | 10 (tableau demain), 12 (appel non attendu), 29 |
| `remplacement.sollicite` / `.accepte` / `.refuse` / `.expire` / `.leve` | `sollicitation_id`, `seance_id`, `agent_id` du remplaçant | 8, 10, 1 (délégation) |
| `suppleance.demandee` / `.obtenue` | `absence_id`, `service_id` | 6, 8 |
| `heure.constatee` / `.validee` | `heure_id`, `agent_id`, mois, nature d'heure | 49 |
| `etat_heures.signe` / `.exporte` | `etat_id`, mois | 49, 9 |
| `rcd.remontee_prete` / `.transmise` | semaine | 9 |
Transition : `rh.absence` et `rh.absence_validee` (existants, `ent-events/src/lib.rs:220-221`) sont émis **en plus** pendant A1, puis retirés au lot A14 (aucun consommateur aujourd'hui [F]).

### 24.2 Consommés [R]

`edt.version_publiee`, `edt.seance_modifiee` (10) → recalcul de l'impact ; `service.suppleance_ouverte` / `_close`, `intervention.ouverte` / `_close`, `agent.parti`, `personnel.engagement_suspendu`, `planning.publie` (6) → impact, circuit ; `sortie.encadrant_affecte` (42), `jury.convocation` (38), `reunion.convocation` (19) → indisponibilités de service ; `appel.fait` (12) → heure constatée ; `etablissement.ferme` (5) → ABP-258 ; `document.purge` (7) → état de la pièce.

### 24.3 Natures de notification déclarées au socle 8 [R]

| Nature | Destinataires | Urgence | Contenu |
|---|---|---|---|
| `absences.a_valider` | valideur | normale | « Une absence de M. X est à valider » — sans nature |
| `absences.decision` | agent | normale | « Votre absence du … est validée / refusée » |
| `absences.piece_demandee`, `absences.echeance` | agent, instructeur | normale | échéance |
| `remplacement.sollicitation` | remplaçant | **haute** (fenêtre de service) | classe, heure, salle |
| `remplacement.designation` | remplaçant | haute | idem, accusé demandé |
| `remplacement.sans_accuse`, `absences.impact_non_traite` | ADJ, VS | haute | séance |
| `heures.etat_a_signer` | chef d'établissement | normale | mois |
| `rcd.remontee_a_controler` | secrétariat | normale | semaine |
Familles et élèves : **aucune nature propre** au 15 ; elles reçoivent les notifications de **modification de séance** du socle 10 (§14.2 du 10), qui portent seulement le motif public.

---

## 25. Écrans (maquette décrite, `DESIGN_SYSTEM.md`, `FRONTEND_CIBLE.md`) et mobile

| N° | Écran | Pour | Contenu |
|---|---|---|---|
| 1 | **Ce matin / demain** | ADJ, DIR, VS | ligne du temps de la journée ; par heure : impacts colorés par état, propositions prêtes (avec « pourquoi »), sollicitations en cours (compte à rebours), effectifs en permanence, familles prévenues ; bouton « appliquer les propositions cochées » ; tuile « à valider » |
| 2 | **Déclarer** | AGT (mobile d'abord) | trois champs : quand (aujourd'hui / demain / dates / demi-journée), nature (liste courte de son engagement, « je préciserai »), pièce facultative ; puis « mes séances touchées » et « déposer le travail » |
| 3 | **Fiche d'une absence** | cercle RH ; AGT | en-tête (agent, dates, état, étape suivante, qui doit agir) ; onglets : décision (traitement, note), pièces, fil, impact (organisation), heures et rattrapage, historique |
| 4 | **Files** | VAL, INS, CPT | à valider, à traiter, à transmettre, DSN J+5, pièces attendues ; échéances ; validation en lot des natures non sensibles |
| 5 | **Ma séance de remplacement** | RPL (mobile) | heure, salle, classe ou groupe, liste d'appel du jour, travail prévu, aménagements nécessaires ; accepter / refuser ; « appel » ouvre l'appel du 12 |
| 6 | **Mes heures** | AGT | mois par mois, par nature ; contester |
| 7 | **États mensuels** | ADJ, INS, DIR | préparation, validation, signature, export, régularisations |
| 8 | **Plan RCD et vivier** | DIR, ADJ | plan versionné, dates, comptes rendus ; volontaires, volumes, créneaux couverts par jour de la semaine ; remontée de la semaine |
| 9 | **Paramétrage** | ADM | natures, règles de circuit, délais (S-1500 à S-1517) |
| 10 | **Côté famille et élève** | FAM, ELV | dans l'EDT de l'élève (socle 10) : changement et intervenant ; état du remplacement ; compte des séances non assurées |

**Mobile (socle 11, une appli par plateforme)** : déclarer, prolonger, déposer une pièce (photo des volets 2 et 3, avertissement volet 1) ; accepter ou refuser une sollicitation ; ma séance de remplacement ; mes heures ; pour l'ADJ : « ce matin » en lecture et validation des propositions. Les tâches longues (états mensuels, plan RCD, paramétrage) restent sur le web (D-003).

---

## 26. Modèle de données cible (brique `absences_personnels`) [R]

Toutes les tables : `id UUID` généré par l'application (E3), `etablissement_id UUID NOT NULL` (E1, tables filles comprises), unicité `(etablissement_id, id)`, clés étrangères **composites** vers des lignes d'établissement (E2), `cree_le` et, si modifiable, `modifie_le` (E6), RLS standard (E13), classement au registre (E12 : domaine `personnels_absences`, portée `etablissement`, catégorie d'export `particuliere` pour les tables à nature, sensibilité **santé** pour `ent_absp_absence.nature_code` et les pièces), **aucune colonne JSON** (E10, D-036), **aucun BYTEA** (E5). Préfixe `ent_absp_`.

```
ent_absp_nature                      -- référentiel versionné ; lignes fournies (etablissement_id = plateforme, voir note) et lignes propres
  id, etablissement_id NULL (fournie) | NOT NULL (propre), code, sous_code NULL, libelle_agent, famille,
  categorie_nationale (CHECK liste R-02), employeur (etat | ogec | tous), sensible BOOL, refus_possible BOOL,
  piece_attendue (aucune | arret | justificatif | convocation), delai_piece_heures NULL, traitement_impose NULL,
  jours_ouvrables_suggeres NULL, texte_libre_permis BOOL, valable_du DATE, valable_au DATE NULL, actif BOOL
ent_absp_nature_etab                 -- activation par établissement d'une nature fournie
  etablissement_id, nature_id, actif, modifie_le
ent_absp_param                       -- une ligne par établissement (S-1500 à S-1517) : colonnes typées
ent_absp_circuit_regle               -- règles de valideur / instructeur : employeur, cycle NULL, famille de nature NULL,
                                     -- role (valideur | instructeur | informe), agent_id NULL | fonction_code NULL, rang
ent_absp_absence
  id, etablissement_id, agent_id (FK composite ent_agent), engagement_id (FK composite), cause (rh | service),
  source_socle NULL, source_ref UUID NULL,            -- indisponibilité de service (R-AP32)
  nature_id NULL (NULL si signalée sans nature), texte_libre VARCHAR(200) NULL (CHECK nature non sensible),
  etat (signalee | declaree | en_instruction | validee | refusee | traitee | close | annulee | prevue | passee),
  debut TIMESTAMPTZ, fin TIMESTAMPTZ NULL (« je prolongerai »), fin_prevue_ouverte BOOL,
  declare_par_agent_id, saisie_par_user_id, saisie_canal (agent | collegue | accueil | import | reprise | systeme),
  traitement (sans_incidence | rattrapage_interne | transmission_rectorat | service_non_fait | rattrapage | retenue_salaire | maintien_salaire | conge_paye) NULL,
  preference_traitement NULL, arret_recu_le TIMESTAMPTZ NULL, confidentialite_renforcee BOOL,
  absence_liee_id NULL (double engagement), cree_le, modifie_le
  -- CHECK (fin IS NULL OR fin > debut)
ent_absp_plage                       -- plages discontinues : absence_id, debut, fin ; exclusion de chevauchement par absence
ent_absp_circuit                     -- copie figée à la création : absence_id, role, agent_id, rang, source (regle | responsable_fonctionnel | defaut | delegation)
ent_absp_decision                    -- ajout seul : absence_id, type (validation | refus | requalification | prolongation | ecourtement | reouverture | annulation | traitement | cloture | transmission),
                                     -- auteur_agent_id, auteur_user_id, le, nature_avant_id, nature_apres_id, traitement_avant, traitement_apres,
                                     -- fin_avant, fin_apres, raison_code, note VARCHAR(1000) NULL, voie_transmission NULL
                                     -- CHECK (type <> 'validation' OR auteur_agent_id <> agent de l'absence)  -- par déclencheur (R-AP10)
ent_absp_piece                       -- absence_id, objet_id (registre ent-object, 500), type_piece, etat (deposee | retiree | purgee), deposee_par, deposee_le, purge_prevue_le, purgee_le
ent_absp_piece_lecture               -- ajout seul : piece_id, lecteur_user_id, le
ent_absp_message                     -- ajout seul : absence_id, auteur_user_id, role_affiche, texte VARCHAR(4000), le
ent_absp_gel                         -- gel de conservation : absence_id, motif, du, au, pose_par
ent_absp_impact                      -- absence_id, type (seance | poste | engagement), seance_id NULL, poste_ref NULL, engagement_socle NULL, engagement_ref NULL,
                                     -- debut, fin, etat (§10.2), reponse_active_id NULL, calcule_le
                                     -- UNIQUE (etablissement_id, absence_id, seance_id) ; une réponse active par séance
ent_absp_reponse                     -- impact_id, type (remplacement | autre_cours | deplacement | continuite | annulation | suppleance | sans_objet),
                                     -- remplacant_agent_id NULL, encadrant_agent_id NULL, modification_edt_ref UUID NULL, delegation_id NULL,
                                     -- etat (active | levee), posee_par, posee_le, levee_par NULL, levee_le NULL
ent_absp_sollicitation               -- impact_id, agent_id, mode (sollicitation | designation | appel_volontaires), envoyee_le, expire_le,
                                     -- etat (envoyee | acceptee | refusee | expiree | annulee), repondue_le, refus_code NULL (motif légitime), plage TSTZRANGE
                                     -- EXCLUDE USING gist (etablissement_id WITH =, agent_id WITH =, plage WITH &&) WHERE (etat = 'acceptee')   -- R-AP19
ent_absp_demande_suppleance          -- absence_id, service_id, destinataire (rectorat | dsden | diocese | interne), envoyee_le, reponse, reponse_le, agent_propose_id NULL
ent_absp_rcd_plan                    -- annee_id, version, etat (brouillon | presente | transmis | archive), avis_cp_le, presente_le, instance, transmis_le, document_id
ent_absp_rcd_compte_rendu            -- plan_id, le, document_id
ent_absp_rcd_engagement              -- agent_id, annee_id, volume_heures NUMERIC(5,1), statut (pacte | hse | autre), du, au, etat
ent_absp_rcd_remontee                -- semaine (date du lundi), etat (preparee | controlee | transmise | rejetee), controlee_par, transmise_le
ent_absp_rcd_remontee_ligne          -- remontee_id, discipline_code, niveau_code, categorie_nationale, mode_continuite,
                                     -- heures_prevues, heures_absence, heures_remplacees   (aucun identifiant de personne)
ent_absp_heure                       -- agent_id, seance_id NULL, impact_id NULL, rattrapage_id NULL, nature (rcd | hse | mission_pacte | rattrapage_du | service_non_fait | vacation),
                                     -- duree_minutes, le, etat (constatee | contestee | validee | figee | exportee), preuve (appel | attestation | saisie), etat_id NULL
ent_absp_heure_fait                  -- ajout seul : heure_id, type (contestation | validation | correction), auteur, le, raison, duree_avant, duree_apres
ent_absp_etat                        -- mois, employeur (etat | ogec), etat (prepare | fige | signe | exporte), signe_par, signe_le, document_id, export_trace_id
ent_absp_etat_ligne                  -- etat_id, agent_id, nature, duree_minutes, regularisation_de_id NULL
ent_absp_rattrapage                  -- absence_id, agent_id, heures_dues NUMERIC(5,1), echeance, etat (du | planifie | solde | prolonge | retenue | abandonne)
ent_absp_rattrapage_creneau          -- rattrapage_id, debut, fin, propose_par, confirme_par NULL, seance_ref NULL, etat
ent_absp_element_paie                -- absence_id NULL, heure_id NULL, agent_id, mois, nature_code, quantite NUMERIC(7,2), unite (heure | jour | demi_jour),
                                     -- etat (prepare | envoye | regularise), UNIQUE (etablissement_id, coalesce(absence_id, heure_id), mois, nature_code)   -- R-AP29
ent_absp_dsn_suivi                   -- absence_id, echeance_le, signale_le NULL, signale_par NULL
ent_absp_journal                     -- ajout seul : objet, objet_id, action, auteur_user_id, le, colonne, valeur_avant TEXT, valeur_apres TEXT (jamais une nature sensible en clair : code seulement)
ent_absp_reprise_lot / _ligne / _correspondance   -- §27 ; zone transitoire en colonnes TEXT, purgée après la bascule
```

**Note sur les natures fournies** : `ent_absp_nature` contient des lignes de référentiel communes (`etablissement_id` NULL) ; c'est une **table de référence globale** au sens d'E4 (aucune donnée d'établissement), classée `globale_reference` au registre ; les lignes propres d'un établissement ont `etablissement_id NOT NULL`. Même schéma que `ent_mission_type` (355) [F].

**Projection de transition** : `ent_absence_rh` reste lue par l'existant (`edt_remplacement.rs`, `ui.rs`) jusqu'au lot A14 ; un déclencheur la tient à jour depuis `ent_absp_absence` (`motif` = « Absence », jamais la nature), ce qui ferme AR-01 même pour les routes héritées oubliées.

**Exceptions D-036** : aucune. Les instantanés du journal sont des colonnes texte typées ; la zone de reprise est en colonnes texte (exception « zone d'import transitoire » déjà admise au socle 42, `MIGRATIONS.md`).

---

## 27. Plan de reprise des données de Next.js (D-035, point 4)

1. **Préalable** : Florian confirme que les absences de Next.js sont **réelles et à garder** (Q-T0-14) ; essais sur export **anonymisé** ; données réelles après l'accord écrit de l'établissement et l'avis du DPO (D-035 point 4).
2. **Périmètre** [R] : absences de l'année en cours et de la précédente (`absence`, `absence_history`, `absence_message`) ; pièces encore présentes (non purgées) **seulement** pour les absences non traitées ; `paie_periode`, `paie_element` : **perdables** (ébauche, T0) ; `absence_ingest_job` : abandonnée.
3. **Rapprochement des agents** : `personnel_id`, puis compte relié, puis e-mail professionnel en indice seulement (T0 §6.5) ; non rapprochés → file « à décider », aucune absence orpheline (CAP-72).
4. **Correspondance des états** :

   | Next.js `workflow_status` / `manager_decision` | ScolIA |
   |---|---|
   | `OUVERTE` ou `JUSTIFICATIF_DEPOSE` / `EN_ATTENTE` | `declaree` (ou `signalee` si `source=accueil`) |
   | `A_TRAITER` / `VALIDEE` | `validee` (file de traitement) |
   | `CLOTUREE` / `VALIDEE` | `close` |
   | `CLOTUREE` / `REFUSEE` | `refusee` puis `close` |
   | `source=admin_manual` ou `admin_pdf` | `validee` saisie validée, canal `import` |

5. **Natures** : `hours_treatment` et les natures non discrétionnaires reconnues (arrêt de travail, enfant malade, congé exceptionnel et son code s'il existe) donnent la nature ; tout le reste → `inconnue` « à qualifier par la direction », **jamais** relancé chez l'agent ; un motif anonymisé (`privacy_reason_redacted`) → `inconnue` « reprise anonymisée » (CAP-71). Le texte libre du motif **n'est jamais repris** dans ScolIA hors de la zone de reprise (supprimée à la fin de la bascule).
6. **Historique et fil** : repris en ajout seul (`ent_absp_decision`, `ent_absp_message`), avec leur date et leur auteur (texte du rôle).
7. **Impact** : recalculé dans ScolIA pour les absences futures seulement ; les absences passées n'ont pas d'impact (Next.js n'en avait pas).
8. **Bascule** (T0 R8 : mars–avril 2027) : propriété unique (R-AP43) — à la date choisie, Next.js passe en lecture pour M05 (geste de Florian, Next.js restant intouchable par le pilote) ; reprise différentielle des absences modifiées depuis le dernier passage ; rapport : comptes par état, par employeur, rejets, agents non rapprochés, heures d'absence par agent et par mois (T0 §6, point 2).

---

## 28. Lots de code (équipe Sonnet) et migrations

### 28.1 Lots

Chaque lot livre : migration(s), dépôt, service en transaction unique, routes et types, tests Postgres (isolation, matrice de droits, cas limites), écrans s'il en a, documentation. Revue `review-sonnet` sur chaque lot ; **revue sécurité** (`security-review`) sur A0, A1, A2, A4, A7, A10. Effort : S ≤ 3 j, M 1–2 sem., L > 2 sem. [H].

| Lot | Contenu | Dépend de | Critère de fin | Effort |
|---|---|---|---|---|
| **A0 Correctifs urgents** | 2300 ; AR-01 à AR-04, AR-06 à AR-09, AR-11, AR-13 à AR-18, AR-21, AR-22, AR-24 (détail §28.4) | — | un professeur ne lit plus aucun motif (test) ; auto-validation refusée (test) ; valideur et déclarant conservés (test) ; refus et annulation possibles ; route historique alignée ou fermée ; levée sans suppression ; famille ne voit jamais un e-mail | M |
| **A1 Cœur de l'absence** | 2301, 2302 ; natures, circuit, déclarer, signaler, confirmer, valider, refuser, requalifier, prolonger, écourter, annuler, rouvrir, journal ; projection `ent_absence_rh` ; ABP-100 à 139 ; droits §23 | A0 ; socle 6 P1, P3 (agents, engagements) | une absence de professeur et une de surveillant OGEC suivent leur circuit (test) ; double engagement = deux absences liées ; aucune nature hors cercle RH (test de garde R-AP2) | L |
| **A2 Pièces, fil, conservation** | 2303 ; ABP-140 à 149, 160 à 164 | A1 ; `ent-object` (500) | pièce lisible du seul cercle, lecture journalisée ; purge à la date (test d'horloge) ; jamais de pièce jointe | M |
| **A3 Impact et tableaux** | 2304 ; ABP-150 à 157, 159, 165 à 167, 170 à 174, 189 ; recalcul ; tâche quotidienne (part) | A1 ; socle 10 (séance existante 110 ; port de la séance stable dès T1/T2) | impact juste sur les cas CAP-05, 06, 25, 26, 27, 46 (tests) ; tableau « ce matin » sur le labo | M |
| **A4 Réponse et sollicitation** | 2305 ; ABP-158, 168, 169, 175 à 179, 182 à 189 ; familles par le socle 10 (ABP-236 à 238, 240 à 247) ; correction AR-10, AR-26 | A3 ; socle 10 (port `modifier_seance`) ; socle 8 | jamais deux plages acceptées qui se chevauchent (test de concurrence) ; sortie anticipée refusée sans information (test R-AP24) ; aucune nature dans les messages (R-AP41) | L |
| **A5 Absence longue et suppléance** | 2306 ; ABP-180, 181, 239 ; AR-23 | A4 ; socle 6 (`nommer_suppleant`) | deux semaines de RCD puis suppléant (CAP-36) ; état du remplacement publié une fois par étape | M |
| **A6 RCD** | 2307 ; ABP-220 à 231 ; flux au socle 9 | A4 ; socle 10 (EDT-147) ; Q-AP3 pour l'API | plan versionné ; volontaires et compteurs ; remontée sans nom (test) | M |
| **A7 Heures et états** | 2308, 2310 (part) ; ABP-200 à 219 | A4 ; socle 12 (`appel.fait`) | heure constatée seulement sur séance assurée (test R-AP28) ; régularisation après export ; unicité des éléments de paie | L |
| **A8 Rattrapages** | 2309 ; ABP-190 à 199 | A7 ; socle 10 (séances ajoutées) | rattrapage dû soldé par une séance ajoutée assurée (test) | M |
| **A9 Absences collectives** | ABP-253 à 258 | A4 | grève du matin sur 12 agents en une liste ; aucune liste de grévistes hors cercle | S |
| **A10 Reprise de Next.js** | 2311 ; ABP-264 à 267 ; §27 | A1, A2 ; export anonymisé fourni par Florian | rapport complet sur l'export anonymisé ; rejouable ; aucun motif libre hors zone de reprise | M |
| **A11 Écrans** | a déclarer et mes absences ; b files et fiche ; c « ce matin », propositions, réponses ; d heures et états ; e RCD ; f paramétrage | lots back | rendu et parcours contre le vrai backend ; clavier, 375 px, axe-core | L (total) |
| **A12 Mobile** | §25 | A1, A4, A7 ; socle 11 | contrat JSON testé ; parité des droits | M |
| **A13 IA** | ABP-248 à 252 ; contrat `ent-ia` ; évaluation art. 6 §4 | A4 ; cœur IA | tests de garde (équivalence écran / IA, simulation sans écriture, aucune nature dans le contexte) | M |
| **A14 RLS stricte, conservation, retrait** | 2318, 2319 ; validation des clés composites (AR-20) ; retrait de `ent_absence_rh` et des routes `/rh/absences` ; événements `rh.*` retirés | tous ; socle 10 T2 | grep des lectures de `ent_absence_rh` à zéro ; RLS stricte testée | M |
| **A15 Personnels non enseignants** | ABP-137, 152, 215 ; postes du planning | A3 ; socle 6 P15 | absence d'un AED : poste à couvrir et alerte sous le minimum | M |

**Ordre recommandé** : **A0** (indépendant, immédiat) → A1 → A2, A3 → A4 → (A7, A5, A9) → A8, A6 → A10 → A11 au fil de l'eau → A12, A13 → A15 → A14.
**Parcours minimal de la rentrée 2027** (usage réel, remplace Next.js M05) : A0, A1, A2, A3, A4, A7, A8, A10, A11 a à d.
**Pour la démo de décembre 2026** (D-034) : A0 seulement, plus l'écran « remplacements » existant corrigé.

### 28.2 Migrations (plage imposée **2300–2319**)

| N° | Nom | Lot | Contenu |
|---|---|---|---|
| 2300 | `2300_absp_a0_correctifs` | A0 | `ent_absence_rh` : `declare_par_user_id` (reprise de `created_by_user_id` **avant** tout écrasement futur), `valide_par_user_id`, `valide_le`, `refuse_*`, `annule_*`, statut `annulee` ; déclencheur « valideur ≠ absent » ; `ent_edt_exception` : `levee_le`, `levee_par` (fin du `DELETE`) ; registre : domaine `personnels_absences`, marque santé |
| 2301 | `2301_absp_referentiel` | A1 | `ent_absp_nature` (+ lignes fournies), `ent_absp_nature_etab`, `ent_absp_param`, `ent_absp_circuit_regle` |
| 2302 | `2302_absp_absence` | A1 | `ent_absp_absence`, `ent_absp_plage`, `ent_absp_circuit`, `ent_absp_decision`, `ent_absp_journal` ; reprise de `ent_absence_rh` (nature `inconnue`, motif **non** repris en clair : copié en zone de qualification réservée au chef d'établissement, purgée après qualification, Q-AP15) ; projection inverse vers `ent_absence_rh` |
| 2303 | `2303_absp_pieces_fil` | A2 | `ent_absp_piece`, `ent_absp_piece_lecture`, `ent_absp_message`, `ent_absp_gel` |
| 2304 | `2304_absp_impact` | A3 | `ent_absp_impact`, `ent_absp_reponse` ; reprise des liens `ent_edt_exception.absence_rh_id` |
| 2305 | `2305_absp_sollicitation` | A4 | `ent_absp_sollicitation` (exclusion gist, `btree_gist`) |
| 2306 | `2306_absp_suppleance` | A5 | `ent_absp_demande_suppleance` |
| 2307 | `2307_absp_rcd` | A6 | `ent_absp_rcd_plan`, `_compte_rendu`, `_engagement`, `_remontee`, `_remontee_ligne` |
| 2308 | `2308_absp_heures` | A7 | `ent_absp_heure`, `ent_absp_heure_fait`, `ent_absp_etat`, `ent_absp_etat_ligne` |
| 2309 | `2309_absp_rattrapage` | A8 | `ent_absp_rattrapage`, `ent_absp_rattrapage_creneau` |
| 2310 | `2310_absp_paie_dsn` | A7 | `ent_absp_element_paie`, `ent_absp_dsn_suivi` |
| 2311 | `2311_absp_reprise_next` | A10 | zone de reprise (colonnes texte), correspondances |
| 2312–2317 | réserve | personas, A15, ajustements | |
| 2318 | `2318_absp_rls_stricte` | A14 | RLS stricte, validation des FK composites |
| 2319 | `2319_absp_retrait` | A14 | retrait de `ent_absence_rh` et des colonnes historiques (après décision, point de sauvegarde, E8) |

Règles : chaque migration additive et rejouable (E8) ; insérée dans `migrate.rs` en ordre croissant ; `MIGRATION_TAG` = la plus haute du répertoire ; nouvelles tables comptées dans `tests_rls.rs` ; file d'envoi déclarée « à neutraliser à la restauration » (E9) — le 15 n'a pas de file propre, il écrit dans l'outbox et la file du socle 8.

### 28.3 Réglages signalés (D-023 : peu de réglages, signalés) — S-1500 à S-1517

| N° | Réglage | Défaut [R] | Utile ? |
|---|---|---|---|
| S-1500 | Natures fournies actives | toutes celles de l'employeur | oui |
| S-1501 | Délai avant reprise d'une absence par la direction (valideur silencieux) | 3 jours ouvrés | oui |
| S-1502 | Nombre de relances de pièce | 2 | à alléger |
| S-1503 | Seuil de proposition d'une suppléance | 10 jours ouvrés | oui |
| S-1504 | Fenêtre de service des sollicitations | 6 h 30 – 20 h | oui |
| S-1505 | Expiration d'une sollicitation (jour même / à l'avance) | 20 min / 12 h | oui |
| S-1506 | Alerte d'impact non traité | H-2 | oui |
| S-1507 | Délai de prévenance du plan RCD | lu dans le plan | oui (plan) |
| S-1508 | Délai d'information avant une sortie anticipée | la veille 18 h, ou 2 h avant | oui |
| S-1509 | CPE : organisation complète des remplacements | non | signalé |
| S-1510 | Familles : compte des séances non assurées de l'enfant | oui | signalé (différenciant) |
| S-1511 | Remplaçant : voir les aménagements des élèves de la séance | oui (minimal) | oui |
| S-1512 | Seuil de masquage des indicateurs | 5 | à fixer avec le DPO |
| S-1513 | Validation en lot des natures non sensibles | oui | oui |
| S-1514 | Échéance des rattrapages dus | fin de l'année scolaire | oui |
| S-1515 | Relance des créneaux de rattrapage | J+10 | à alléger |
| S-1516 | Jour de préparation de la remontée RCD | lundi 7 h | à alléger |
| S-1517 | Chef d'établissement remplaçant payé | non | signalé |

### 28.4 Lot A0 en détail (peut partir tout de suite)

1. **Motif (AR-01, AR-02, AR-03)** : `GET /rh/absences` exige une action dédiée `personnels.absences.lire_nature` (direction, secrétariat) ; pour les autres, une vue sans motif (« Absence ») ; `GET /edt/exceptions` ne renvoie plus le motif libre aux rôles qui font seulement l'appel (déjà prévu au lot T0 du socle 10, à faire **une seule fois**, dans le lot qui part en premier) ; `ui.rs` : les tableaux affichent « Absence » (D-010 : `ui.rs` figé, correctif de sécurité seulement) ; registre : domaine `personnels_absences`, marque santé.
2. **Auto-validation (AR-06)** : refus en service et déclencheur en base.
3. **Auteur (AR-13)** : la validation n'écrase plus `created_by_user_id` ; colonnes valideur et date.
4. **Refus, annulation (AR-16)** : commandes et routes ; une absence annulée ne bloque plus la personne comme remplaçante ; `signalee` ne bloque plus non plus (seulement `validee` et `declaree`) [R].
5. **Route historique (AR-08, AR-14, AR-18)** : `POST /edt/exceptions` en `kind=remplacement` passe par `edt_organiser_remplacement` (disponibilité, délégation, verrou) ou renvoie 410 ; l'UPSERT ne remet plus `absence_rh_id` à NULL.
6. **Concurrence (AR-09)** : verrou consultatif **par remplaçant** en plus de la séance, contrôle de disponibilité **dans** la transaction.
7. **Annulation (AR-11)** : `edt_annuler_seance` vérifie l'état et l'agent de l'absence citée.
8. **Levée (AR-15)** : état « levée » daté au lieu du `DELETE` (exception, déplacement, séance déplacée marquée retirée).
9. **E-mail (AR-04)** : jamais l'e-mail d'un personnel vers une famille ; à défaut de nom affiché, « un enseignant de l'établissement ».
10. **Écran (AR-21, AR-22)** : bouton de validation selon le droit réel ; paramètre `statut` traité.
11. **Journal (AR-17)** : déclaration, validation, refus, annulation, remplacement, levée journalisés (`ent_journal_modif`).
12. **Dette (AR-24)** : code mort retiré.
Tests : un professeur, l'accueil, la comptabilité ne lisent aucun motif (routes `rh/absences`, `edt/exceptions`, `ui.rs` par l'API) ; auto-validation refusée ; route historique refuse un parent comme remplaçant ; deux remplacements simultanés sur le même agent : un seul passe ; levée conservée.

---

## 29. Tests et recette

1. **Garde de minimisation (R-AP2)** : pour chaque nature sensible du référentiel, une absence de labo est créée ; on parcourt **toutes** les sorties (réponses d'API des modèles de rôles non RH, événements, notifications, exports non RH, journaux applicatifs) et on y cherche le libellé et le code : zéro occurrence.
2. **Structure des DTO** : le type de la projection « organisation » n'a pas de champ `nature`, `motif`, `texte_libre` (test de compilation ou de schéma).
3. **Matrice de droits** (§23) sur les dix modèles ; isolation entre établissements (`separation.rs` étendu) ; agent à deux établissements (CAP-68).
4. **Circuit** : professeur, surveillant OGEC, double engagement, valideur absent, chef d'établissement absent (CAP-04, 08, 09, 10).
5. **Auto-validation** impossible, y compris par délégation et par l'IA.
6. **Concurrence** : deux sollicitations acceptées en parallèle sur la même plage d'un agent ; deux réponses en parallèle sur une séance.
7. **Aller-retour** de chaque colonne (rien de perdu, contre AD-07).
8. **Horloge** : absence qui franchit minuit, le changement d'heure, la fin de mois (états), l'échéance de purge, H-2.
9. **R-AP24** : séance annulée de fin de journée, élève autorisé, famille prévenue trop tard → permanence.
10. **Heures** : séance remplacée sans appel → pas d'heure validée ; correction après export → régularisation.
11. **Reprise** : export anonymisé de Next.js (fourni par Florian) rejoué deux fois → même résultat ; rapport exact.
12. **IA** : équivalence écran / IA ; simulation sans écriture ; fiche de contexte sans nature ; message aux familles contenant « malade » bloqué (R-AP41).
13. **Recette par personas** (D-021 étape 4) sur le logiciel codé : les seize personas de `PERSONAS_15.md`, y compris les attaques.

---

## 30. Décisions appliquées par défaut (D-031) et questions à Florian

### 30.1 Appliquées par défaut (D-031 ; Florian peut revenir dessus)

| N° | Question | Recommandation appliquée |
|---|---|---|
| Q-AP1 | Reprendre le code Next.js M05 ? | **non** : spécification fonctionnelle seulement (maturité partielle, §0 point 2) ; toutes ses actions reprises |
| Q-AP2 | Motif libre ? | **supprimé** au profit d'une nature catégorisée (R-AP1) |
| Q-AP6 | Valideur quand le chef d'établissement est absent | son délégataire désigné ; à défaut, pour un salarié de l'OGEC, le président de l'OGEC ou son délégué ; pour un agent de l'État, l'adjoint, avec information de l'autorité académique |
| Q-AP7 | Instance de présentation du plan RCD dans le privé | l'instance que l'établissement désigne (conseil d'établissement, conseil pédagogique), date et pièce tracées comme au public |
| Q-AP13 | Le DPO lit-il les natures ? | **non** : il contrôle les accès et les purges |
| Q-AP14 | Sollicitations et plage calme | fenêtre de service 6 h 30 – 20 h pour une séance du jour ou du lendemain matin (R-AP39) |
| Q-AP15 | Motifs libres déjà saisis dans `ent_absence_rh` | non repris en clair ; qualifiés par le chef d'établissement puis effacés |
| Q-AP16 | Seuil de suppléance | 10 jours ouvrés (S-1503) |
| Q-AP17 | Refus d'un collègue hors RCD | libre et sans motif (R-AP34) |
| Q-AP18 | Plusieurs sollicitations simultanées | non, sauf « appel à volontaires » explicite |
| Q-AP19 | Expiration des sollicitations | 20 min le jour même, 12 h à l'avance |
| Q-AP20 | Familles : compte des séances non assurées | oui, par matière, jamais par professeur (S-1510) |
| Q-AP21 | Sortie anticipée | seulement en fin de demi-journée ou de journée, avec information préalable (R-AP24) |
| Q-AP22 | Heure constatée | sur séance assurée (appel fait ou attestation) |
| Q-AP23 | Paie de l'OGEC tant que le socle 49 n'existe pas | export des éléments variables ; file DSN J+5 manuelle |
| Q-AP24 | Pièces des agents de l'État | purgées 30 jours après transmission (à confirmer par le DPO, Q-AP12) |
| Q-AP25 | Indicateurs | agrégés seulement, aucun indicateur nominatif d'absentéisme |
| Q-AP26 | IA | propose et rédige, ne décide ni n'envoie ; jamais sur les natures ni les pièces |
| Q-AP27 | Grève | saisie le jour même ou après, nature sensible, retenue signalée, jamais calculée |
| Q-AP28 | Indisponibilités de service (sortie, jury) | même moteur d'impact, sans fait RH (R-AP32) |
| Q-AP29 | Planning de service des non-enseignants | reste au socle 6 (Q-P30 du 6) ; le 15 lit les postes |
| Q-AP30 | Remplaçant : aménagements des élèves | oui, minimal et le jour de la séance (S-1511) |
| Q-AP31 | Validation en lot | seulement les natures non sensibles |
| Q-AP32 | Confidentialité renforcée d'une absence | possible à la demande de l'agent ou de la direction (ABP-164) |
| Q-AP33 | Absence signalée bloque-t-elle la personne comme remplaçante ? | non tant qu'elle n'est pas confirmée — **sauf** si elle couvre aujourd'hui (bon sens : quelqu'un d'absent aujourd'hui ne remplace pas) |
| Q-AP34 | Lot A0 avant la démo de décembre | oui |
| Q-AP35 | `ent_absence_rh` | gardée en projection jusqu'à A14 |
| Q-AP36 | Numérotation propre (ABP-, R-AP, CAP-, S-15xx, M-15xx) | oui, pour éviter les collisions entre sessions parallèles |

### 30.2 Soumis à Florian (engagent l'établissement, des données réelles, des contrats ou une interdiction de CLAUDE.md)

| N° | Point | Pourquoi Florian |
|---|---|---|
| Q-AP-F1 | **Défauts de Next.js en production** (AD-01 URL libre récupérée par le serveur, AD-02 auto-validation, AD-03 motif médical lisible par les surveillants et l'administratif, dans les mails et la paie, AD-05, AD-06 `GET` destructeur) | Next.js est intouchable pour le pilote ; c'est aujourd'hui une fuite possible de données de santé de salariés et d'agents : à corriger ou restreindre par Florian, ou à accepter en connaissance de cause jusqu'à la bascule |
| Q-AP-F2 | **Maturité et données de M05** : les absences de Next.js sont-elles en usage réel et à reprendre ? (Q-T0-14) | fait connu de Florian seul |
| Q-AP-F3 | **Qui organise les remplacements aujourd'hui** dans le lycée pilote (Charlemagne, Excel, papier, EDT d'Index) ? Pendant le doublon 2027-2028, les remplacements et l'information des familles se font-ils **réellement** dans ScolIA ? (risque de double notification aux familles si Charlemagne ou École Directe les préviennent aussi) | engage l'établissement ; recommandation : **usage réel** pour la déclaration, la validation, les heures et la paie (remplacement de Next.js, D-035) ; **organisation des remplacements en réel dès que l'EDT de ScolIA est chargé** ; information des familles **en réel** seulement si l'outil actuel ne prévient pas déjà |
| Q-AP3 | **Spécification de l'API SI-RCD** et inscription éditeur | démarche extérieure (DNE, DGRH ; inscription éditeur D-029, F-06) |
| Q-AP4 | Pacte 2026-2027 (missions RCD, part fonctionnelle) | texte non trouvé ; à suivre |
| Q-AP5 | **Circuit des maîtres contractuels** vers le rectorat (division de l'enseignement privé), outil académique et format des états d'heures | fait local, rectorat |
| Q-AP8 | Durées et liste des ASA du décret 2026-604 (au 01/01/2027) | circulaire attendue |
| Q-AP9 | **Convention EPNL** : 16 congés exceptionnels de Next.js, maintien de salaire | à faire confirmer par la comptable ou le juriste de l'OGEC |
| Q-AP10 | Grève d'un salarié de l'OGEC : retenue | droit social, juriste |
| Q-AP11 | Logiciel de paie actuel de l'OGEC et format d'import | fait local |
| Q-AP12 | **Durées de conservation** (§18.4) et notice d'information des personnels | DPO |

---

## 31. Faits, hypothèses, contradictions

**Faits [F]** : M05 de Next.js est partiel (persistance qui perd des champs, logique en double, `GET` destructeur, motif médical exposé, aucun lien avec l'EDT ni les familles) ; l'ENT Rust expose le motif à tout le personnel, permet l'auto-validation, écrase le déclarant, supprime physiquement les levées ; la plus haute migration est 667 ; la plage 2300–2319 est libre sur les 124 branches.
**Hypothèses [H]** : HSE = 1/36ᵉ + 25 % ; pas de HSE pour remplacer une convenance ; outils académiques ASIE / GI-GC / SUPPLE ; règles de la CCN EPNL ; retenue d'un gréviste salarié de l'OGEC ; pratiques de terrain du §3 et du §4.
**Contradictions signalées** :
- C-15-1 : la carte (`SOCLES.md` §15) met « décompte des heures supplémentaires effectives » au 15 ; `PERSONAS_05-06.md` (PER-15) le met au 15 ou au N1 « demandes » ; le socle 6 porte la nature de rémunération des missions. **Proposé** : le 15 constate les heures liées aux absences et aux remplacements **et** les HSE ponctuelles ; les HSA restent au service (socle 6) ; les demandes d'HSE hors remplacement (projet, sortie) au N1.
- C-15-2 : le socle 10 (D3, lot T0) et ce socle (A0) corrigent tous deux le motif des exceptions d'EDT : **un seul** correctif, dans le lot qui part en premier.
- C-15-3 : `SOCLE2_DOMAINES.md` D19 nomme `ent_remplacement`, `ent_personnel_fiche`, `ent_service_rh` : remplacés par le modèle §26 (et le socle 6 pour la fiche).
- C-15-4 : `TRANSITION_T0_INVENTAIRE.md` dit « Partiel à fiable » pour M05 ; l'inventaire de cette conception conclut **partiel** (à confirmer par Florian, Q-AP-F2).
- C-15-5 : l'inventaire T0 compte 48 colonnes à `absence`, le schéma Drizzle en déclare 46 [F] (colonnes ajoutées par script hors migration, `scripts/_add-absence-cols.mjs`, à vérifier à la reprise).

---
## 32. Ajout personas : effets extérieurs, circuit externe, inférences, actions et lots révisés

Source : `docs/socles/PERSONAS_15.md` (16 personas, 62 manques M-1501 à M-1562). Les recommandations sont **appliquées par défaut** (D-031). **Ce paragraphe l'emporte** sur les précédents en cas de contradiction ; rien n'a été supprimé plus haut.

### 32.1 Les quatre bloquants tranchés

1. **EDT de référence (A, R-AP45).** Un interrupteur par établissement, posé par la direction (ABP-272) après un contrôle « EDT à jour » du socle 10, conditionne **tout effet vers les familles et les élèves** (ABP-236 à 239, 244 à 246) et **toute proposition de sortie anticipée**. Sans lui, le socle 15 calcule l'impact et organise en interne. Pendant le doublon 2027-2028, il reste **éteint** tant que l'EDT officiel est celui de Charlemagne (Q-AP-F3).
2. **Signalement non confirmé (B, R-AP46).** Une absence `signalee` (accueil, collègue, constat) n'a **aucun effet extérieur** tant qu'elle n'est pas confirmée par l'agent (son compte) ou par un **rappel tracé** (ABP-273 : l'ADJ ou la direction rappelle l'agent par le bouton « appeler » de sa fiche, sans voir le numéro, et coche « confirmé par rappel à 7 h 24 »). L'organisation interne (préparer un remplaçant) peut commencer.
3. **Circuit externe (C, R-AP47).** Chaque couple (employeur, nature) déclare où se prend la décision : **interne** (salarié de l'OGEC, convenance d'un agent de l'État, ASA soumise aux nécessités du service) ou **externe** (congé de maladie d'un fonctionnaire ou d'un maître : rectorat ; AESH : DSDEN ou employeur mutualisateur, PIAL informé). En externe, les états `validee` et `refusee` sont remplacés par `prise_en_compte` (l'établissement a constaté et transmis) puis, si l'information revient, `decision_employeur` (accordée, refusée, requalifiée : date, pièce) (ABP-288, ABP-289) ; le refus n'est jamais proposé à l'établissement ; l'organisation et les heures ne dépendent pas de la décision externe.
4. **Présence (D, R-AP48).** `absences::present` : l'accueil et la vie scolaire pour tout le personnel ; un professeur pour les agents avec qui il partage, ce jour-là, une séance, une équipe (socle 6) ou un rendez-vous ; limitation de débit ; consultations hors accueil journalisées ; jamais d'historique. ABP-155 est restreint en conséquence.

### 32.2 Corrections ponctuelles

- **S-1510** (compte des séances non assurées pour les familles) : **désactivé par défaut** ; activé, il compte par période et sur l'ensemble des matières, jamais par matière seule (R-AP49) ; ABP-244 et Q-AP20 sont modifiés en ce sens.
- **CAP-06** (rendez-vous médicaux réguliers) : un **temps partiel thérapeutique** n'est **pas** une absence ; c'est une modification datée de quotité au socle 6, dont le socle 10 tire les séances à replacer ou à suppléer (R-AP52). L'absence discontinue reste pour des rendez-vous ponctuels.
- **ABP-128** (requalifier) : ouvert à l'agent lui-même **avant** décision.
- **ABP-124** (validation en lot) : décision possible **par plage** pour une convocation de plusieurs jours.
- **ABP-158** (ma séance de remplacement) : avec le **trombinoscope** de la séance si l'établissement montre les photos aux professeurs (socle 2).
- **ABP-213** (compteur RCD) : position du volontaire par rapport à la médiane, sans nommer les autres.
- **ABP-234** (décision à l'agent) : dit en clair ce que devient la paie (« maintenue », « à rattraper », « sans solde »).
- **§14.4** : jours **calendaires** en plus des jours ouvrés et des heures (arrêts, carence).
- **§27 et migration 2302** : les zones de reprise (motifs libres de Next.js et de `ent_absence_rh`) sont **chiffrées**, réservées au chef d'établissement, et **détruites** à la fin de la qualification, au plus 60 jours (S-1521, ABP-298).
- **R-AP41** s'applique aussi **à la saisie** du texte libre (refus expliqué).
- **A15** (postes des non-enseignants) entre dans le **parcours minimal de la rentrée 2027** (les surveillants de l'OGEC utilisent Next.js aujourd'hui).
- **Bascule de M05** : au **premier jour d'un mois**, après la clôture des variables du mois précédent (R-AP55) ; message aux personnels dans Next.js par Florian.

### 32.3 Actions ajoutées (ABP-272 à ABP-299)

| N° | Action | Acteurs | Règles et effets | Manque |
|---|---|---|---|---|
| ABP-272 | Poser ou retirer l'interrupteur **« EDT de référence »** | DIR ★ | après contrôle « EDT à jour » (socle 10) ; date, auteur | M-1507 |
| ABP-273 | **Confirmer par rappel** une absence signalée | ADJ, DIR | bouton « appeler » sans afficher le numéro ; heure du rappel | M-1545, M-1546 |
| ABP-274 | « **Le remplaçant choisit** » : partager sa progression au lieu de déposer du travail | AGT | lien au cahier et à la progression (20) | M-1502 |
| ABP-275 | Voir son absence dans **son** emploi du temps | AGT | socle 10 | M-1503 |
| ABP-276 | **Échanger** deux séances entre deux professeurs (aujourd'hui / plus tard) | ADJ ; deux accords | deux modifications liées du 10 ; heures neutres | M-1508 |
| ABP-277 | **Avancer** un cours de la même classe dans la journée | ADJ | familles prévenues si l'heure de sortie change | M-1509 |
| ABP-278 | Solliciter **par SMS** (secours, lien à usage unique) | SYS, ADJ | S-1519 ; coût Q-N22 | M-1510 |
| ABP-279 | Établir un **bordereau** de transmission au rectorat | INS | plusieurs absences, date, accusé | M-1516 |
| ABP-280 | Alerte de **capacité** d'une salle de permanence | SYS → VS | proposition d'une deuxième salle ou du CDI | M-1531 |
| ABP-281 | Rendre ponctuellement un **créneau RCD** (« pas ce jeudi ») | AGT | indisponibilité lue au 10 | M-1533 |
| ABP-282 | Prévenir le **suppléant** d'un retour anticipé du titulaire | SYS | port du 6 | M-1539 |
| ABP-283 | Traiter une **remontée RCD rejetée** | INS | motif, correction, renvoi | M-1551 |
| ABP-284 | Informer le **coordonnateur du PIAL** | INS, VS | message ou export tracé, sans nature | M-1554 |
| ABP-285 | Produire la **notice** aux personnels et la fiche de registre | DIR, DPO | trame | M-1560 |
| ABP-286 | **Revue des validations croisées** | DIR | dans la revue des accès (socle 6, P10) | M-1561 |
| ABP-287 | **Contester** une absence déclarée à mon nom | AGT | l'absence passe « contestée » ; décision de la direction | M-1562 |
| ABP-288 | **Prendre en compte** une absence à décision externe | INS | `prise_en_compte` (R-AP47) | M-1549 |
| ABP-289 | Enregistrer la **décision de l'employeur** (accordée, refusée, requalifiée) | INS | date, pièce ; aucune réorganisation du passé | M-1517 |
| ABP-290 | Enregistrer un **arrêt** (initial ou de prolongation) avec sa date de réception | INS, AGT | une ligne par arrêt, échéance 48 h chacun | M-1515 |
| ABP-291 | Voir les **compteurs** annuels par nature (pris, restants) | AGT, VAL, CPT | R-AP53 | M-1505, M-1520 |
| ABP-292 | Marquer une absence connue après la clôture comme **« mois suivant »** | SYS, CPT | R-AP55 | M-1519 |
| ABP-293 | Lever la **pause** de ses notifications pendant son absence | AGT | R-AP51 | M-1501 |
| ABP-294 | Voir l'**état final** de la matinée (accusés, familles prévenues) et son journal | ADJ, DIR | écran 1 | M-1511 |
| ABP-295 | Produire la **trame du compte rendu au CA** | DIR | taux, heures non assurées, modes de continuité, volontaires | M-1550 |
| ABP-296 | Remplacer un **surveillant** d'examen blanc ou de devoir commun | ADJ | candidats : tout personnel habilité à surveiller (R-AP54) | M-1512 |
| ABP-297 | Voir les **jours calendaires** d'un arrêt | CPT | §14.4 | M-1521 |
| ABP-298 | **Détruire** une zone de reprise | SYS, ADM | S-1521 ; journal | M-1558 |
| ABP-299 | Refuser proprement une **acceptation hors ligne** périmée | SYS | sollicitation expirée ou prise | §4 transverses |

### 32.4 Règles ajoutées (R-AP45 à R-AP59)

| N° | Règle |
|---|---|
| R-AP45 | Aucun effet vers les familles et les élèves, aucune sortie anticipée, sans **EDT de référence** posé. |
| R-AP46 | Une absence **non confirmée** (ni par l'agent, ni par un rappel tracé) n'a aucun effet extérieur. |
| R-AP47 | La décision d'une absence est **interne** ou **externe** selon (employeur, nature) ; en externe, l'établissement prend en compte, transmet, organise, et n'accorde ni ne refuse. |
| R-AP48 | La **présence** d'un agent n'est visible que de l'accueil, de la vie scolaire et des agents qui partagent avec lui une séance, une équipe ou un rendez-vous ce jour-là ; débit limité ; consultations journalisées. |
| R-AP49 | Aucun compte de séances non assurées **par matière** pour les familles ; désactivé par défaut. |
| R-AP50 | Le collègue qui a saisi une absence pour un autre voit l'organisation et l'étape, jamais la nature choisie ensuite. |
| R-AP51 | Pendant son absence, l'agent ne reçoit que ce qui le concerne directement (décision, pièce demandée) ; le reste attend son retour, résumé ; il peut lever la pause. |
| R-AP52 | Un **temps partiel thérapeutique** est une quotité temporaire au socle 6, jamais une absence. |
| R-AP53 | Une nature à **plafond annuel** (enfant malade, congés exceptionnels) a un compteur par agent et par période de référence ; dépasser le plafond ne bloque pas, cela demande une décision. |
| R-AP54 | Pour une **surveillance** (examen blanc, devoir commun), les candidats sont les personnels habilités à surveiller, pas seulement les enseignants. |
| R-AP55 | Un fait de paie connu après la **clôture des variables** du mois part le mois suivant, signalé ; jamais réécrit dans un mois parti. |
| R-AP56 | La sortie anticipée se calcule **par élève** : sa dernière séance de la demi-journée ou de la journée, son régime, son transport scolaire, son internat (socle 14). |
| R-AP57 | Le message aux familles dépend de l'heure : avant le départ habituel de l'élève, il annonce le changement d'horaire ; après, il dit où l'élève est accueilli (aucun changement d'horaire n'est imposé). |
| R-AP58 | La projection d'organisation ne montre **ni pièce attendue, ni circuit, ni traitement, ni fin ouverte** (« je prolongerai ») : rien dont on puisse déduire la nature. |
| R-AP59 | L'IA qui lit une convocation ne change **jamais** l'agent choisi par l'administratif ; si le document en nomme un autre, elle l'avertit ; sa sortie suit un schéma fermé (plages, lieu, objet). |

### 32.5 Cas limites ajoutés (CAP-73 à CAP-90)

| N° | Situation | Traitement attendu |
|---|---|---|
| CAP-73 | Un élève imite un professeur au téléphone | `signalee`, aucun effet extérieur ; rappel de l'ADJ : l'agent est présent → annulation, journal (R-AP46) |
| CAP-74 | EDT de ScolIA en retard d'une modification de Charlemagne pendant le doublon | interrupteur éteint : aucune notification ; l'ADJ organise en interne (R-AP45) |
| CAP-75 | Séance de fin de journée annulée ; deux élèves prennent le car de 17 h 30, cinq ont une option à 16 h | sortie proposée seulement aux autres élèves autorisés et prévenus (R-AP56) |
| CAP-76 | Le suppléant est absent trois jours | impact sur le service suppléé ; circuit de **son** engagement (maître délégué, externe) |
| CAP-77 | Le rectorat requalifie un congé de maladie ordinaire en congé de longue maladie, rétroactivement | décision externe enregistrée (ABP-289) ; l'organisation passée n'est pas refaite |
| CAP-78 | Absence d'une AESH employée par la DSDEN | `prise_en_compte`, information du PIAL (ABP-284), élève accompagné : impact (CAP-42) |
| CAP-79 | Cinquième prolongation d'un arrêt, reçue un vendredi soir | ligne d'arrêt avec sa réception, échéance de 48 h propre (ABP-290) |
| CAP-80 | Congés payés d'une surveillante pendant les vacances, alors qu'elle tient l'internat ouvert | impact sur le poste de vacances seulement |
| CAP-81 | Salariée de l'OGEC au-delà du plafond annuel d'enfant malade | compteur dépassé : décision demandée, pas de blocage (R-AP53) |
| CAP-82 | Absence connue le 22 pour un arrêt du 18 ; variables parties le 20 | « mois suivant », signalé (R-AP55) |
| CAP-83 | Deux adjoints valident mutuellement leurs absences chaque mois | revue des validations croisées (ABP-286) |
| CAP-84 | Un professeur interroge chaque matin la présence de tous ses collègues | refusé hors de son périmètre (R-AP48), consultations journalisées |
| CAP-85 | Échange de séances : le second professeur tombe malade le jour de la contrepartie | la contrepartie devient un impact de sa propre absence ; l'échange reste tracé |
| CAP-86 | Avancer un cours de 15 h à 10 h pour une classe dont des élèves ont une autorisation de sortie | nouvelle heure de sortie : familles prévenues avant (R-AP24, R-AP57) |
| CAP-87 | PDF de convocation qui contient du texte demandant de créer une absence pour un autre agent | l'agent ne change pas ; avertissement (R-AP59) |
| CAP-88 | Acceptation d'une sollicitation depuis un téléphone resté hors ligne 30 min | refusée à la synchronisation, message clair (ABP-299) |
| CAP-89 | Texte libre « RDV gynéco » sur une nature non sensible | refusé à la saisie, explication (R-AP41) |
| CAP-90 | Absence d'un professeur dans un établissement d'une seule section d'allemand : la remontée RCD le désigne de fait | obligation légale ; mentionné au registre (M-1552) |

### 32.6 Modèle : compléments (D-036 respecté)

```
ent_absp_arret              -- une ligne par arrêt (initial, prolongation) : absence_id, type (initial | prolongation | rechute),
                            -- du, au, recu_le, recu_par, piece_id NULL, echeance_le (48 h)        -- M-1515 (arret_recu_le de l'absence devient dérivé)
ent_absp_decision_externe   -- absence_id, employeur (rectorat | dsden | employeur_mutualisateur | autre), decision (accordee | refusee | requalifiee),
                            -- nature_apres_id NULL, le, piece_id NULL, saisie_par                   -- R-AP47
ent_absp_compteur_regle     -- nature_id, employeur, plafond_jours NUMERIC(5,1), periode (annee_scolaire | annee_civile | glissante_12_mois), valable_du
ent_absp_compteur           -- agent_id, nature_id, periode_debut, jours_pris NUMERIC(5,1), calcule_le         -- R-AP53 (dérivé, recalculable)
ent_absp_echange            -- reponse_aller_id, reponse_retour_id, agent_a_id, agent_b_id, accord_a_le, accord_b_le, etat
ent_absp_bordereau          -- destinataire, envoye_le, accuse_le NULL ; ent_absp_bordereau_ligne (bordereau_id, absence_id, arret_id NULL)
ent_absp_presence_lecture   -- ajout seul : lecteur_user_id, agent_id, le   (hors accueil et vie scolaire)  -- R-AP48
ent_absp_param              -- + edt_reference BOOL, edt_reference_pose_le, edt_reference_pose_par ; + jour_cloture_variables SMALLINT ;
                            -- + sms_secours_minutes ; + pause_notifications BOOL ; + reprise_conservation_jours
ent_absp_absence            -- + etat 'prise_en_compte' | 'decision_employeur' | 'contestee' ; + confirmee_par (agent | rappel) ; confirmee_le ; rappel_par_user_id
ent_absp_nature             -- + decision (interne | externe) par employeur ; + piece_a_partir_de_jours SMALLINT NULL
ent_absp_reponse            -- + type 'echange' | 'avance'
ent_absp_impact             -- + type 'surveillance'
```

### 32.7 Migrations et lots révisés

| N° | Nom | Lot | Contenu |
|---|---|---|---|
| **2312** | `2312_absp_personas_circuit_arrets` | A1 | `ent_absp_arret`, `ent_absp_decision_externe`, colonnes d'états et de confirmation, `ent_absp_nature.decision`, paramètres (interrupteur, clôture, pause, SMS, reprise) |
| **2313** | `2313_absp_personas_compteurs_echanges` | A4, A7 | `ent_absp_compteur_regle`, `ent_absp_compteur`, `ent_absp_echange`, `ent_absp_bordereau(_ligne)`, `ent_absp_presence_lecture` |
2314–2317 restent en réserve.

| Lot | Changement |
|---|---|
| A0 | + restreindre la présence (R-AP48) dans l'existant : la route des absences du jour n'est plus ouverte à tout le personnel (déjà couvert par AR-01) |
| A1 | + R-AP46, R-AP47, R-AP50, R-AP52, R-AP58 ; ABP-273, 287 à 290 ; migration 2312 |
| A3 | + interrupteur (R-AP45, ABP-272) |
| A4 | + ABP-276, 277, 278, 280, 281, 296, 299 ; R-AP54, R-AP56, R-AP57 ; migration 2313 (échanges) |
| A7 | + compteurs, clôture des variables, jours calendaires (ABP-291, 292, 297) |
| A10 | + destruction des zones de reprise (ABP-298) ; bascule au premier jour d'un mois |
| A13 | + R-AP59 |
| A15 | **entre dans le parcours minimal de la rentrée 2027** |
**Parcours minimal de la rentrée 2027 (remplace §28.1)** : A0, A1, A2, A3, A4, A7, A8, A10, A11 a à d, **A15**.

### 32.8 Questions (suite)

Appliquées par défaut : Q-AP38 à Q-AP44 (`PERSONAS_15.md` §8.1). Soumise : **Q-AP37 — analyse d'impact (AIPD)** avant la mise en service (DPO).

Réglages ajoutés (D-023, signalés) :

| N° | Réglage | Défaut [R] |
|---|---|---|
| S-1518 | Jour de clôture des variables de paie | le 20 |
| S-1519 | Délai de secours SMS quand une sollicitation n'est pas lue | 5 min |
| S-1520 | Pause des notifications pendant une absence | oui |
| S-1521 | Conservation maximale des zones de reprise | 60 jours |

S-1510 passe à **non** par défaut.

### 32.9 Totaux après la passe

| Élément | Avant | Après |
|---|---|---|
| Actions | 172 (ABP-100 à 271) | **200** (ABP-100 à 299) |
| Règles | 44 | **59** |
| Cas limites | 72 | **90** |
| Réglages | 18 | **22** (S-1500 à S-1521) |
| Migrations | 2300–2311, 2318, 2319 | + **2312, 2313** |
