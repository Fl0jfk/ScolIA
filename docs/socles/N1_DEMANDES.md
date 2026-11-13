# Socle N1 — Demandes et flux de travail

**Date :** 2026-11-13 — **Statut :** conception de fond, **non décidée** (à relire par Florian) ; les recommandations s'appliquent par défaut (D-031), sauf ce qui engage l'établissement à l'extérieur (§34.2). Aucun code écrit ni modifié. Aucun accès à une base, à un secret ou à un environnement de production. Le code Next.js a été **lu seulement** (fichiers de `app/api/requests/*`, `app/api/assistance`, `app/api/demandes-hse/*`, `app/api/photocopies-couleur/*`, `app/api/settings/requests-*`, `app/lib/requests*.ts`, `app/lib/request-*.ts`, `app/lib/demandes-hse-access.ts`, `app/lib/hse-acceptance-pdf.ts`, `app/lib/photocopies-couleur-*.ts`, `app/lib/assistance.ts`, `db/schema-ent-relational.ts`), rien exécuté, rien construit, rien modifié.
**Branche :** `ent-socle-N1-conception`, partie de `origin/ent-mc0hoe` (`699275d`).
**Pourquoi maintenant :** risque n° 1 du plan de bascule (`docs/PLAN_BASCULE_FEVRIER.md`, lignes M04 et M08, RK-1, Q-PB5) : les personnels utilisent chaque jour, dans le Next.js en production, le module de demandes (assistance, demandes internes et leur routage, HSE, photocopies). Sans N1, rien ne le remplace le 20 février 2027.
**Passe personas :** `docs/socles/PERSONAS_N1.md` ; ses ajouts sont faits **ici**, au **§36**, qui **l'emporte** en cas de contradiction. **Précisions de Florian relayées par le pilote** et seconde passe : **§37**, qui l'emporte sur tout le reste. Rien n'a été supprimé.
**À lire avec :** `CLAUDE.md` ; `docs/DECISIONS.md` (D-009, D-017, D-021, D-022, D-023, D-026, D-028, D-031, D-035, **D-036**, D-037 à D-041, **D-048**, **D-051**, **D-055**, **D-057**, **D-059**) ; `docs/PLAN_BASCULE_FEVRIER.md` ; `docs/SOCLES.md` (§10 : trous T-01, T-02) ; `docs/MATRICE_CHARLEMAGNE.md` (DEM-01 à DEM-04, PER-15, PIL-10) ; `docs/TRANSITION_T0_INVENTAIRE.md` (M04, M08, R9, Q-T0-04) ; `docs/ALERTES_NEXTJS_PROD.md` (NJ-01 à NJ-79) ; `docs/PILOTAGE.md` (règles 13, 14, 16) ; conceptions `06_PERSONNELS.md` (missions, équipes, délégations, pouvoirs de signature), `07_DOCUMENTS.md` (`ServiceDocuments`, dépôts, jetons courts), `08_NOTIFICATIONS.md` (§6 : déclaration des natures d'avis), `15_ABSENCES_PERSONNELS.md` (§14 : heures constatées, état mensuel), `16_MESSAGERIE.md` (§11.1, §19.3 : boîtes de service), `19_AGENDA_RDV.md` (modèle de forme), `29_PORTAILS.md` (§8 : à-faire, P4 : files des personnels ; A4, A5, A13), `44_RESSOURCES_MATERIELLES.md` (§12 : interventions, §15 : reprographie, DRM-01 à DRM-13), `49_PAIE.md` (RH4, RHP-252), `IA_COEUR.md` ; `docs/FRONTEND_LEPTOS.md` ; `docs/PRA_SAUVEGARDES.md` §18.4 (E1–E14).

**Conventions.**
- **[F]** fait vérifié dans le code (Rust : chemins depuis `ent-rust/` ; Next.js : chemins depuis la racine du dépôt) ; **[S]** repris d'une autre conception qui l'a vérifié (référence donnée) ; **[NV]** règle citée de mémoire, **non vérifiée** dans cette session (à vérifier sur Légifrance pendant la recette) ; **[H]** hypothèse (pratique du terrain) ; **[R]** recommandation, non décidée.
- **« N1 » seul désigne ce socle.** Les lots du socle 8 sont écrits « lot N1 du socle 8 » ; les niveaux de confirmation d'`IA_COEUR.md` sont écrits **conf. 1**, **conf. 2**, **conf. 3** (aperçu simple ; aperçu détaillé ; empreinte et assurance récente).

**Numérotation propre au socle** :

| Élément | Numéros | Remarque |
|---|---|---|
| Fonctions existantes de Next.js à reprendre | **NXN-01**… | propres à ce document |
| Défauts de Next.js | **NJ-80 à NJ-95** | **à la suite du registre** `ALERTES_NEXTJS_PROD.md` (dernier : NJ-79) ; NJ-77 et NJ-78 (photocopies) sont aussi fermés ici |
| Actions cibles | **DEM-100**… | DEM-01 à DEM-04 de la matrice Charlemagne sont repris et renvoient ici |
| Règles | **R-DM1**… | |
| Cas limites | **CDM-01**… | |
| Questions | **Q-DM1**… | |
| Réglages signalés (D-023) | **SN1-01**… | |
| Manques de la passe personas | **MN1-001**… | |

**Plage de migrations : 3040–3059** (inscrite dans `MIGRATIONS.md`, détail §31). **Contradiction signalée** : le plan de bascule proposait **3000–3019** (§3 « contraintes de CI », Q-PB5) ; cette plage est **déjà réservée au socle 50** (et 3020–3039 au socle 51) depuis le 2026-11-11. La plage 3040–3059 a été vérifiée libre le 2026-11-13 : aucun fichier `ent-rust/migrations/30[4-5]x_` sur les **177 branches distantes**, aucune mention dans les `MIGRATIONS.md` de ces branches.

**Méthode.**
1. Lecture des décisions, du plan de bascule, de la carte, de la matrice, de l'inventaire T0 et du registre des défauts.
2. Lecture ciblée des conceptions voisines (6, 7, 8, 15, 16, 19, 29, 44, 49) pour tenir les frontières déjà écrites.
3. Lecture du module Next.js (liste en tête), sans rien exécuter. Aucune donnée réelle lue ni recopiée : une adresse nominative trouvée dans le code est remplacée par « [adresse nominative] ».
4. État du Rust [F] : **aucun** module de demandes (aucune table, aucune route ; `grep` de `demande`, `ticket`, `hse`, `reprograph` dans `ent-api` et `ent-db`).

Sommaire :

| Partie | Sections |
|---|---|
| Synthèse | §0 En bref · §1 Thèse |
| Existant | §2 Next.js, défauts, à ne pas perdre |
| Métier | §3 Périmètre et frontières · §4 Acteurs · §5 L'année vue du terrain · §6 Réglementaire |
| Cœur | §7 Concepts · §8 Natures de demande · §9 Le guichet · §10 Routage · §11 Prise en charge, délégation, absences · §12 Échanges et pièces · §13 Délais, relances, escalade · §14 Confidentialité · §15 Historique et preuve · §16 Notifications et à-faire |
| Natures intégrées | §17 HSE · §18 Reprographie · §19 Familles et élèves |
| Règles et sécurité | §20 IA · §21 Cas limites · §22 Actions · §23 Règles · §24 Droits et minimisation |
| Technique | §25 Événements · §26 Natures d'avis et de tâches · §27 Écrans · §28 Modèle de données |
| Mise en œuvre | §29 Reprise Next.js (R9) · §30 Lots N1-0 à N1-5 · §31 Migrations · §32 Tests et recette |
| Décisions | §33 Réglages · §34 Décisions appliquées et questions · §35 Faits, hypothèses, contradictions |
| Ajout personas | §36 |

---

## 0. En bref

1. **Une demande est un engagement.** Quelqu'un a besoin de quelque chose de l'établissement ; une personne **identifiée** en répond, dans un **délai connu**, et il en reste une **preuve**. Le guichet N1 est le moteur commun de cet engagement : dépôt, accusé de réception, routage, prise en charge, échanges, pièces, délais, relances, escalade, décision, clôture, historique. Il ne remplace ni la messagerie (16), ni l'à-faire (29), ni les avis (8) : il les utilise.
2. **Une seule porte, des circuits propriétaires.** Le personnel, la famille ou l'élève trouvent **toutes** leurs démarches dans un catalogue unique (« Faire une demande »). Mais une démarche qui a déjà son propriétaire (une absence au 15, une réservation de salle au 19, une sortie au 42, une justification d'absence au 12, une rectification RGPD au 27) **n'est pas recopiée** : le catalogue ouvre son parcours. N1 ne porte que ce qui n'a pas d'autre maison, et les natures que les briques lui déclarent (§8).
3. **Les natures sont déclarées par les briques** (comme les natures d'avis du socle 8) : code stable, publics autorisés, champs typés, routage par défaut, confidentialité, délais, circuit, pièces, avis, tâches, effets à la clôture, conservation. L'établissement **active** une nature et règle peu de choses (D-023) ; il ne crée pas de formulaire libre (le socle 18 le fait).
4. **Le routage va à une personne, à un rôle ou à une file**, jamais à « tout le personnel ». Une **file de service** (secrétariat, comptabilité, maintenance, reprographie, vie scolaire…) a des **membres calculés** depuis les missions et fonctions du socle 6, un responsable, une file de repli. Le routage est **déterministe** (nature, site, cycle de l'élève) ; l'IA peut **proposer**, jamais affecter seule, et ne reçoit jamais d'identifiant d'élève (§20).
5. **Une demande n'a qu'un seul responsable à la fois.** Prise en charge, relâche, réattribution, délégation, transfert vers une autre file, consultation d'un tiers, escalade : tout est un **fait daté** (ajout seul). Personne ne « vole » une demande d'une autre file (NJ-81).
6. **Absences et départs ne laissent rien en plan.** Une demande prise par un agent absent (socle 15) ou parti (socle 6) revient à sa file avec un signal ; les délégations datées du socle 6 s'appliquent ; la file de repli et l'escalade garantissent qu'aucune demande ne dort (D-028 « jamais de silence »).
7. **Confidentialité par nature, minimisation partout.** Une demande RH, sociale ou disciplinaire n'est lue que par sa file ; le demandeur voit ce qui lui est destiné, **jamais les notes internes** (NJ-82) ; les e-mails et SMS ne portent qu'une **enveloppe neutre** et un lien (NJ-86) ; les pièces passent par le socle 7 (jetons courts liés à la session, NJ-83).
8. **HSE en « light » utile** (§17) : la demande d'heures supplémentaires effectives (autorisation préalable) est une nature intégrée ; la direction du site décide (volume accordé, motif) ; les **séances réalisées** sont déclarées puis **attestées** (service fait) ; les heures attestées deviennent des **heures constatées du socle 15** (état mensuel signé, export au rectorat) ; pour un salarié de l'organisme de gestion, elles iront aux variables de paie du socle 49 (lot RH4, D-048, D-055). Une enveloppe annuelle est suivie, sans plafond codé. Plus de PDF à signature-image ni d'adresse codée en dur (NJ-91).
9. **Reprographie** (§18) : nature intégrée conforme à la conception du socle 44 (§15) : deux circuits (noir et blanc direct sous un seuil, couleur ou gros volume validés par la direction **du site**), file d'impression **cloisonnée par site** (NJ-77), PDF déposés au socle 7 et liés au déposant (NJ-78), « prête » par un geste authentifié, rappel des plafonds CFC. Le socle 44 reprendra plus tard compteurs, copieurs et déclaration CFC sans reprise de données.
10. **Familles et élèves** (§19) : de vrais comptes (D-009, D-051) ; une famille dépose une demande cataloguée (attestation, duplicata, changement de régime, demande au secrétariat…) ; une **question libre** reste une conversation du socle 16. Une conversation peut **devenir** une demande (et le fil se ferme en le disant) : jamais les deux à la fois. Plus de page publique sans compte rattachant des enfants par l'adresse e-mail (NJ-88).
11. **Existant** : Next.js a un module riche mais fragile (routage par IA, corbeille, file partagée, délégation, page parents, HSE, photocopies) et **16 défauts nouveaux** (NJ-80 à NJ-95), dont 6 élevés : n'importe quel compte peut commenter et lire n'importe quelle demande, détourner une demande d'une autre file, lire toutes les pièces, usurper l'identité du demandeur ; le demandeur lit les notes internes ; le texte intégral et des identifiants d'élèves partent à l'IA. Le Rust n'a **rien** : tout est à construire, sans dette à corriger.
12. **Taille et plan** : **251 actions** (DEM-100 à DEM-357 et DEM-360 à DEM-399, dont 24 de la passe personas et 16 des précisions de Florian, §37), **58 règles** (R-DM1 à R-DM58), **86 cas limites** (CDM-01 à CDM-86), 19 réglages signalés, 17 questions ; **6 lots pour la bascule** (N1-0 modèle et garde-fous ; N1-1 natures et routage ; N1-2 prise en charge, délégation, pièces ; N1-3 HSE et reprographie ; N1-4 écrans Leptos ; N1-5 reprise R9), puis N1-6 à N1-9 après février (mobile, IA, indicateurs, RLS stricte et natures des autres briques). Migrations **3040–3059**. Calendrier du plan : N1-0 semaine du 7 décembre, N1-1 du 14, N1-2 du 11 janvier, N1-3 à N1-5 du 25 janvier (`PLAN_BASCULE_FEVRIER.md` §4).
14. **Précisions de Florian (§37, l'emporte)** : la **certitude d'être traité** devient la promesse centrale — trois preuves visibles du demandeur (envoyée avec service, rôle et délai ; **vue** par le service ; **prise** par une personne nommée par son prénom), retard toujours montré ; une **porte unique « Écrire à l'établissement »** pour familles et élèves, qui aiguille par règles déclarées (sans IA obligatoire) vers une demande suivie, un raccourci ou un message du 16, sans jamais bloquer ; conversion message ↔ demande ; **file N1 et boîte du 16 jumelées** (mêmes membres, un seul écran « Accueil du service ») plutôt que fusionnées ; « Résolu ? Oui / Non » retenu, notes d'agents écartées ; seconde passe personas (`PERSONAS_N1.md` §10).
13. **[personas] Ce que la passe ajoute (`PERSONAS_N1.md`, §36)** : trois bloquants tranchés par défaut — **conflit d'intérêts** dans une file (la personne visée par une demande RH ou une question à la direction en est exclue, file de repli hors service, R-DM47), **restriction de dépôt** contre le harcèlement par le guichet (R-DM48), **avis de reprise unique** à l'ouverture de la bascule (R-DM49) ; puis attestation HSE fondée sur l'appel ou une déclaration sur l'honneur (R-DM50), accueil qui ne voit aucune demande d'autrui (R-DM51), copies multi-classes et récurrentes, tournée de maintenance, enveloppe prévisionnelle, délégation de signature des attestations, décisions implicites à venir.

---

## 1. Thèse : une demande est un engagement de l'établissement envers une personne

Dans un établissement, tout le monde demande quelque chose à quelqu'un, toute la journée : un professeur signale un vidéoprojecteur en panne, une famille veut un duplicata de bulletin, le secrétariat a besoin d'une facture de la comptabilité, un agent demande un congé de formation, une professeure demande 6 HSE pour un atelier de préparation au brevet, un AED a besoin de 120 copies couleur pour demain. Aujourd'hui, ces besoins passent par un couloir, un post-it, un e-mail perdu, un appel à l'accueil, ou — dans le Next.js — par un module unique dont la première qualité est d'exister.

Ce qui fait perdre du temps n'est pas l'absence d'outil, c'est l'absence d'**engagement lisible** : « à qui je demande ? », « est-ce que quelqu'un l'a vu ? », « qui s'en occupe ? », « pour quand ? », « pourquoi on ne m'a pas répondu ? », « qu'est-ce qui a été décidé, et par qui ? ». Le guichet N1 répond à ces six questions, pour chaque demande, **à la fois pour le demandeur et pour celui qui traite**.

Trois idées guident la conception :

1. **Le demandeur ne doit pas connaître l'organigramme.** Il dit ce dont il a besoin (une nature, trouvée en tapant quelques mots) ; le logiciel sait qui s'en occupe, sur quel site, pour quel cycle, et qui remplace qui.
2. **Celui qui traite ne doit pas tout voir.** Il voit **sa** file, **ses** demandes, ce dont il a besoin pour agir, et rien de plus ; ce qui est confidentiel (RH, social, discipline) reste dans la file qui en a le droit. La transparence va **vers le demandeur** (état, délai, personne en charge, décision), pas vers tout le personnel.
3. **Rien ne tombe dans un trou.** Chaque demande a toujours une file, une échéance et un filet (file de repli, escalade). Une absence, un départ, une fermeture, une erreur de routage ne laissent jamais une demande sans responsable (D-028).

Ce que N1 **n'est pas** :

| N1 n'est pas | C'est | Pourquoi la distinction compte |
|---|---|---|
| une messagerie | le **socle 16** (conversation libre, boîte de service) | une conversation n'a ni nature, ni champs, ni décision, ni délai d'engagement ; une demande en a |
| un à-faire | le **socle 29** (index des tâches de chacun) | N1 **ouvre** des tâches (« demande à traiter », « complément à fournir ») ; le 29 les montre |
| une notification | le **socle 8** (avis, canaux, preuves d'envoi) | N1 **déclare** ses natures d'avis ; le 8 décide du canal et de la plage horaire |
| un formulaire | le **socle 18** (collecte de réponses, autorisations, consentements) | un formulaire collecte ; une demande attend un **traitement** par quelqu'un |
| un logiciel de tickets informatique | — | ScolIA ne vise pas les métiers de l'assistance informatique ; la nature « assistance numérique » couvre le besoin réel d'un établissement |

---

## 2. Évaluation franche de l'existant

### 2.1 ENT Rust [F]

Aucun module de demandes : aucune table (`ent-rust/migrations/` : 240 migrations, aucune `demande`, `ticket`, `request`, `hse`, `reprographie`), aucune route ni écran. Les briques utiles existent ou sont conçues : missions et délégations (socle 6, migration 355 `missions`), dépôt de fichiers et liens courts (`presign.rs`, à durcir par le socle 7), outbox d'événements (D-028), fonction `ent_rls_poser` (migration 580), champs chiffrés (`ent-db/src/champs_chiffres.rs`). **Conséquence** : N1 part d'une page blanche, sans dette Rust à corriger ; le lot N1-0 n'a pas de « correctifs de l'existant Rust », il porte les **garde-fous qui ferment par construction** les défauts du Next.js (§30).

### 2.2 Next.js : ce que fait le module (M04, M08) [F]

| Partie | Fichiers | Stockage | Usage (T0) |
|---|---|---|---|
| **Demandes** (M04) | `app/api/requests/{create,list,update,confirm,parent-portal,attachment-url,access,routes}/route.ts` ; `app/lib/requests.ts` (927 l.), `requests-routing-config.ts` (822 l.), `requests-org-*.ts`, `requests-board*.ts`, `request-pending-verify.ts`, `requests-eleve-context.ts` ; pages `app/(demandes)/*` | Postgres `request`, `request_attr` (attributs « chemin / valeur » en texte), `request_comment`, `request_history`, `request_attachment` ; configuration JSON `settings/requests-routing.json`, `settings/requests-org.json` ; pièces S3 `requests/<id>/files/` | **en usage** (personnels) |
| **Assistance** | `app/api/assistance/route.ts`, `app/lib/assistance.ts` | rien : un e-mail | en usage [H] |
| **HSE** (M08) | `app/api/demandes-hse/route.ts` (508 l.), `[id]/pdf`, `app/lib/demandes-hse-access.ts`, `hse-acceptance-pdf.ts` | **un index JSON** `demandes-hse/index.json` en stockage objet, PDF d'acceptation | en usage [H] |
| **Photocopies** (M08) | `app/api/photocopies-couleur/*` (802 l.), `app/lib/photocopies-couleur-*.ts` | **un index JSON** `photocopies-couleur/index.json`, PDF en S3 | en usage [H] ; inventorié aussi par le 44 (NRM-01 à NRM-10) |

**Fonctions à reprendre (NXN)** — toutes reprises, sous une forme corrigée :

| NXN | Fonction Next.js [F] | Où | Repris par |
|---|---|---|---|
| NXN-01 | Dépôt d'une demande (sujet, description ≥ 15 caractères, jusqu'à 12 pièces) par un personnel connecté | `create/route.ts` | DEM-100, DEM-104 |
| NXN-02 | Dépôt sans compte avec confirmation par lien e-mail (72 h), anti-robot (« pot de miel »), limite 8 dépôts par heure et par adresse IP | `create/route.ts:24-29`, `request-pending-verify.ts:7` | **non repris tel quel** (D-051) : dépôt par l'accueil pour le compte (DEM-106) ou compte famille (§19) |
| NXN-03 | Page « demande des parents » activable, avec rattachement des enfants | `parent-portal/route.ts` | §19, **sans** rattachement par e-mail (NJ-88) |
| NXN-04 | Routage automatique par IA (Mistral) parmi un catalogue de routes, seuil de confiance 0,52, sinon corbeille ; repli par mots-clés | `requests.ts:554-700`, `requests-routing-config.ts:440-520` | §10, §20 (proposition seulement) |
| NXN-05 | Routes « direction » jamais affectées directement : dépôt à l'administratif du cycle avec un indicateur « direction » | `requests.ts:371-420` | R-DM14 (escalade explicite), DEM-160 |
| NXN-06 | Corbeille (file de tri) ; prise depuis la corbeille qui affecte la demande à la branche de l'agent | `requests-board.ts`, `update/route.ts:135-150` | file de tri (§10.4), DEM-158 |
| NXN-07 | File partagée : le premier qui prend la main la retire de la file des autres | `requests.ts:516-540` | DEM-170 |
| NXN-08 | Relâcher, renvoyer à la corbeille (responsable), réassigner vers un autre service (responsable) | `update/route.ts:152-200, 345-375` | DEM-172, DEM-176, DEM-177 |
| NXN-09 | Délégation par le responsable à un collaborateur de la file | `update/route.ts:60-102` | DEM-174 |
| NXN-10 | Transmission à la direction | `update/route.ts:261-298` | DEM-178 |
| NXN-11 | Statuts NOUVELLE, EN_COURS, EN_ATTENTE, TERMINEE ; commentaire interne ou au demandeur, avec pièces | `requests.ts:203`, `update/route.ts:379-420` | §11.1, §12 |
| NXN-12 | Supervision « globale » et « par métier » (responsables d'unités) | `requests-oversight.ts` | DEM-181, `demandes.file.superviser` |
| NXN-13 | Organigramme des services (unités, responsables, membres, tâches) et routage paramétrables | `settings/requests-org`, `settings/requests-routing` | §10.2 (sur les missions du 6, sans listes d'adresses) |
| NXN-14 | File « RH » forcée depuis le module RH (préfixe « [Demande RH] ») | `create/route.ts:33`, `requests-routing-defaults.ts` | nature `rh.demande_agent` (§8.4) |
| NXN-15 | Avis par e-mail au dépôt, à la prise en charge, à la mise en attente et à la clôture ; pièces de clôture | `requests.ts:799-927` | §16 (enveloppe neutre, lien) |
| NXN-16 | Purge 30 jours après clôture | `requests.ts:205-265` | conservation par nature (§15.3) |
| NXN-17 | Assistance : zone concernée, description, pièces, référence de ticket, accusé par e-mail | `assistance.ts` | nature `numerique.assistance` (§8.4) avec suivi réel |
| NXN-18 | HSE : demande par un enseignant (objet, nombre d'heures au quart d'heure, classe, précisions), **pour le compte** d'un collègue par l'administratif, la comptabilité ou la direction ; décision par la direction de l'établissement concerné ; annulation par le demandeur tant qu'elle est en attente ; PDF d'acceptation ; e-mail à la gestion | `demandes-hse/route.ts`, `hse-acceptance-pdf.ts` | §17 |
| NXN-19 | Photocopies : deux circuits (noir et blanc direct, couleur validée par la direction du site), dépôt pour un enseignant, 1 à 5 PDF, file d'impression, « prête » et « prête, pas encore vue » | `photocopies-couleur/*` | §18 (et NRM-01 à NRM-10 du 44) |

### 2.3 Défauts trouvés dans Next.js (à ne pas reproduire ; Next.js reste intouchable, D-051)

Numérotés **à la suite du registre** `ALERTES_NEXTJS_PROD.md` (le dernier était NJ-79). Chacun est **fermé par un lot et un test** (règle 14 de `PILOTAGE.md`), colonne de droite. Gravité selon la grille du registre.

| NJ | Gravité | Défaut | Preuve [F] | Fermé par (lot · test) |
|---|---|---|---|---|
| **NJ-80** | **élevé** | **Tout compte connecté peut écrire dans n'importe quelle demande** : la branche « commentaire » de la mise à jour ne contrôle aucun droit (seul l'ajout de fichiers est réservé au personnel) ; avec `toRequester`, un e-mail part **de l'adresse de l'établissement** vers le demandeur avec un texte libre (hameçonnage) ; la réponse renvoie **la demande entière** (description, échanges, pièces) : lecture de toute demande dont on connaît l'identifiant (`Date.now()` + 6 caractères) | `app/api/requests/update/route.ts:379-443` | N1-0 · `t_nj80_ecrire_sans_droit_refuse` : un compte hors file et non demandeur reçoit 404 (pas 403 : on ne révèle pas l'existence), aucun avis émis, aucune donnée renvoyée |
| **NJ-81** | **élevé** | **Détournement d'une demande d'une autre file** : `claim_self` saute le contrôle de file quand la demande est **déjà prise** ; tout personnel « habilité au tableau » (maintenance, surveillant, comptabilité…) reprend ainsi une demande RH ou de vie scolaire et la lit ; `transmit_to_direction` est ouvert à tout personnel habilité, sur toute demande | `update/route.ts:208-260`, `261-298` | N1-2 · `t_nj81_prise_hors_file_refusee`, `t_nj81_transmission_hors_file_refusee` |
| **NJ-82** | **élevé** | **Le demandeur lit les notes internes** : la liste « mes demandes » renvoie la fiche complète (commentaires internes, hypothèse de routage de l'IA, adresses des membres de la file, motif) | `app/api/requests/list/route.ts:34-41` | N1-0 · `t_nj82_projection_demandeur` : aucune note interne, aucune adresse du personnel, aucune trace de routage dans la projection du demandeur |
| **NJ-83** | **élevé** | **Toutes les pièces lisibles de tout le personnel du tableau** : le lien de pièce vérifie seulement « personnel habilité » ou « demandeur » (par égalité d'e-mail), **pas la file** ; lien S3 valable **une heure** | `attachment-url/route.ts:33-41` | N1-2 · `t_nj83_piece_hors_file_refusee` ; jeton du socle 7 lié à la session, court |
| **NJ-84** | **élevé** | **Identité du demandeur fournie par le client** : même connecté, prénom, nom, e-mail et téléphone sont saisis librement ; les e-mails de suivi partent à l'adresse saisie (l'établissement écrit à n'importe qui) ; « mes demandes » et le droit de lire une pièce se rattachent par **égalité d'adresse e-mail** | `create/route.ts:96-290`, `list/route.ts:35-40`, `attachment-url/route.ts:32` | N1-0 · `t_nj84_demandeur_de_session` : le demandeur est la personne de la session (ou la personne désignée par un dépôt « pour le compte », tracé) ; aucun champ d'identité accepté du client |
| NJ-85 | moyen | **Corbeille ouverte à tout le personnel** : une demande non classée est visible de **tous** les membres de **toutes** les files (contenu compris) | `app/lib/requests-board.ts:54` | N1-1 · `t_nj85_file_de_tri_restreinte` : la file de tri a des membres désignés |
| **NJ-86** | **élevé** | **Contenu intégral hors de l'ENT** : description complète, nom, e-mail et téléphone du demandeur dans les e-mails aux files ; **pièces de clôture jointes** aux e-mails ; PDF de photocopies joints aux e-mails du service impressions ; HSE : motif, classe et PDF envoyés par e-mail | `requests.ts:799-845, 863-906` ; `photocopies-couleur/route.ts:165-215` ; `demandes-hse/route.ts:200-240, 455-480` | N1-0 · `t_nj86_enveloppe_neutre` (lint de minimisation du socle 8 sur toutes les natures N1) |
| **NJ-87** | **élevé** | **IA externe sans minimisation** : le texte intégral de la demande part à Mistral pour la classer ; pour le routage par catalogue, les **élèves reconnus** (INE, nom, prénom, cycle) et les étiquettes des personnels sont joints ; aucune trace, aucune information du demandeur | `requests.ts:602-640` ; `requests-routing-config.ts:440-505` ; `requests-eleve-context.ts:102-140` | N1-6 (IA) ; dès N1-1, **aucun appel d'IA** pour router (routage par nature, déterministe) · `t_nj87_aucun_identifiant_eleve_vers_ia` |
| NJ-88 | moyen | **Page publique « parents » qui rattache des enfants par l'e-mail** : sans compte, la demande confirmée reçoit la liste des enfants (INE, nom, prénom, classe) dont un parent a la même adresse e-mail, sans contrôle du lien, de l'autorité ni d'une interdiction ; des fichiers sont stockés **avant** toute confirmation ; la confirmation est un **GET** (un antivirus de messagerie peut confirmer à la place de la personne) | `parent-portal/route.ts:113-120` ; `confirm/route.ts:13` ; `create/route.ts:150-185` | non repris (D-051) ; N1-0 · `t_nj88_aucun_depot_public` (route absente) ; §19 |
| NJ-89 | moyen | **Preuve détruite** : l'historique, les échanges et les pièces sont **effacés 30 jours** après « terminée », par une purge **déclenchée par la lecture de la liste** ; rouvrir annule la purge ; suppression en cascade (`ON DELETE CASCADE`) | `requests.ts:205-265` ; `list/route.ts:28` ; `db/schema-ent-relational.ts:404-476` | N1-0 · `t_nj89_faits_ajout_seul`, `t_nj89_purge_par_ordonnanceur` (durée par nature, §15.3) |
| NJ-90 | moyen | **Écritures perdues et données non normalisées** : chaque action relit, modifie en mémoire et réécrit la demande sans contrôle de version ; HSE et photocopies réécrivent **un index JSON unique** en stockage objet (contraire à D-036) ; attributs en paires « chemin / valeur » texte | `update/route.ts` (toutes les branches) ; `demandes-hse/route.ts:63-70` ; `photocopies-couleur/route.ts:44-70` | N1-0 · `t_nj90_version_concurrente` (deux prises simultanées : une seule réussit, l'autre reçoit 409) |
| NJ-91 | moyen | **HSE** : une **adresse nominative** de gestionnaire académique est codée en dur comme destinataire par défaut des HSE acceptées (avec le PDF) ; l'attestation dit « signée par la direction » mais appose **une image de signature** automatiquement ; l'acceptation vaut heures à payer (**aucun service fait**) ; aucune modification après décision ; le dépôt « pour le compte » enregistre le collègue comme auteur **sans trace du déposant** | `demandes-hse/route.ts:28-31, 155-178, 455-480` ; `hse-acceptance-pdf.ts:133` | N1-3 · `t_nj91_aucun_destinataire_code`, `t_nj91_service_fait_obligatoire`, `t_nj91_deposant_trace` ; signature par le socle 7 avec `peut_signer` (6) |
| NJ-92 | moyen | **Photocopies : action par lien public** : « marquer comme prête » est un GET sans session, valable **30 jours** ; transférer l'e-mail suffit pour agir (complète DRM-04, DRM-05 du 44) | `mark-ready/route.ts:55-100` ; `photocopies-couleur-ready-token.ts:5` | N1-3 · `t_nj92_pret_par_geste_authentifie` |
| NJ-93 | faible | Messages d'exception internes renvoyés au client (création, paramétrage) | `create/route.ts:289` ; `settings/requests-routing/route.ts` (PUT) | N1-0 · `t_nj93_erreurs_neutres` |
| NJ-94 | moyen | **Organisation par listes d'adresses e-mail** : files, responsables, membres, délégations et prises en charge sont des adresses dans une configuration JSON (D-036) ; un départ ou un changement d'adresse laisse des demandes orphelines ; la délégation enregistre l'adresse comme nom, sans compte ni date de fin ; aucune prise en compte des absences | `requests-routing-config.ts:36`, `requests-org-config.ts:32`, `update/route.ts:80-90` | N1-1 · `t_nj94_membres_calcules_du_socle_6`, `t_nj94_depart_rend_les_demandes` |
| NJ-95 | faible | **Assistance sans suivi** : le « ticket » n'est qu'un e-mail (aucun statut, aucune trace, aucune réponse rattachée) ; nom de l'utilisateur inséré sans échappement dans le HTML de l'e-mail | `assistance.ts:60-127` (l. 83, 119) | N1-1 · nature `numerique.assistance` ; `t_nj95_rendu_echappe` |

**Défauts déjà au registre, fermés aussi par N1** : **NJ-77** (file de photocopies de tous les sites) et **NJ-78** (PDF d'un autre rattachable) → lot N1-3 (`t_nj77_file_par_site`, `t_nj78_document_du_deposant`) ; DRM-01 à DRM-13 du socle 44 sont appliqués à la nature `reprographie.tirage` (§18) ; **NJ-72** (limitation de débit) pour les routes de N1 → N1-0 (`t_nj72_debit_depot`).

**Mesures immédiates suggérées à Florian** (Next.js intouchable ; décision et geste de Florian seulement) : restreindre le module « Demandes » aux personnels tant que NJ-80 n'est pas corrigé ; demander de ne pas mettre de donnée d'élève ni de donnée RH sensible dans une demande (elle part en clair par e-mail et à l'IA) ; retirer l'adresse nominative par défaut des HSE (NJ-91) en définissant la variable prévue (`HSE_OPS_EMAIL`) — ces mesures relèvent du chantier D-053 si Florian le décide.

### 2.4 Ce qu'il ne faut pas perdre (D-035, point 5)

- **La file partagée** et « le premier qui prend la main » : c'est ce que les personnels apprécient (NXN-07).
- **La corbeille de tri** pour ce qui ne rentre nulle part, avec un responsable (NXN-06).
- **L'indicateur « direction »** : une demande qui relève de la direction n'atterrit pas directement chez le chef, elle passe par le secrétariat qui la qualifie (NXN-05).
- **Le dépôt pour le compte d'un collègue** (HSE, photocopies) : réel besoin du secrétariat et de l'accueil (NXN-18, NXN-19).
- **Les deux circuits de photocopies** et le signal « prête, pas encore vue ».
- **Les avis aux étapes importantes** (et pas à chaque geste).
- **Le quart d'heure** comme unité de volume d'HSE (NXN-18).

### 2.5 Verdict

Le module Next.js **prouve le besoin** et son fonctionnement est bon sur le fond (file partagée, tri, indicateur direction). Sa sécurité ne l'est pas (six défauts élevés), ni sa preuve (purge à 30 jours), ni ses données (JSON, adresses e-mail). **On reprend les fonctions, pas le code ni le modèle.** Le Rust n'a rien : N1 est construit d'emblée sur les briques du cœur (personnes 4, missions 6, documents 7, avis 8, à-faire 29).

---

## 3. Périmètre et frontières : qui possède quoi

### 3.1 Tableau

| Objet | Propriétaire | Ce que fait N1 |
|---|---|---|
| **Demande** (dépôt, nature, valeurs, routage, prise en charge, échanges, délais, décision, clôture, faits) | **N1** | possède |
| **Natures de demande** (déclaration) | **la brique qui en porte le métier** ; N1 pour ses natures intégrées (§8.4) | registre, contrôle, moteur |
| **Files de service** (définition, membres calculés, responsables, horaires) | **N1** | possède ; membres **lus** du socle 6 (missions, fonctions, équipes) |
| Agents, missions, équipes, délégations, pouvoirs de signature | **6** | lit (`missions`, `delegations_actives`, `peut_signer`, `agent_du_compte`) |
| Absences et indisponibilités des personnels | **15** | lit `absences::indisponibilites(agent, plage)` (sans motif) |
| **Heures constatées**, état mensuel, export au rectorat, éléments variables | **15** | appelle `heures::constater(…)` (port à ajouter au 15, §17.6) |
| Paie des salariés de l'organisme de gestion | **49** (RH4) | publie `demandes.hse_attestee` ; le 49 la consomme (lot RH4) |
| Fichiers, pièces, documents émis, signatures, dépôt au dossier | **7** | appelle `ServiceDocuments` (dépôt, émission d'une attestation, dépôt au dossier de l'élève ou de l'agent) |
| Avis, canaux, plage calme, preuves d'envoi | **8** | déclare ses natures d'avis (§26.1) ; notifie par événement |
| Index des tâches, pastille, accueil | **29** | ouvre et ferme ses tâches (§26.2) ; déclare ses **tuiles de file** (A4) |
| Conversations, boîtes de service | **16** | conversion « fil → demande » (§19.4) ; jamais de fil parallèle |
| Formulaires, autorisations, consentements | **18** | une nature peut **demander** une autorisation (port du 18) ; N1 ne collecte pas de consentement |
| Rendez-vous, réservation de salles | **19** | le catalogue **ouvre** le parcours du 19 (raccourci) |
| Absences des élèves, justificatifs | **12** | raccourci vers « justifier une absence » |
| Sorties et voyages | **42** | raccourci |
| Interventions techniques, copieurs, compteurs, CFC, clés, prêts | **44** | la demande d'intervention **naît au guichet** (44 §12.2) ; le 44 porte le suivi technique (`volet` de nature, §8.3) ; reprographie : §18 |
| Inscriptions, dérogations, radiations, exeat | **3** | raccourci, ou nature déclarée par le 3 |
| Coordonnées, foyers, interdictions, autorité parentale | **4** | lit (`liens_de`, `droits_lien`, `interdictions`) ; une demande de changement de coordonnées = **proposition** du 4 |
| Remboursements, échéanciers, aides | **24, 25, 26** | natures déclarées par ces socles |
| Demandes d'exercice des droits (RGPD), conservation | **27** | nature `rgpd.exercice_droit` déclarée par le 27 ; durées de conservation des natures (§15.3) |
| Signalements de harcèlement, informations préoccupantes, bien-être | **13, 36** | **jamais une demande N1** : le catalogue ouvre leur canal protégé (R-DM31) |
| Règles automatiques de l'établissement | **31** | les échéances de N1 tournent sur l'ordonnanceur du 31 (`ent_auto_planif`) |
| Assistant IA | **30**, cœur IA | N1 déclare ses capacités (§20) |
| Indicateurs dans le temps | **47** | N1 publie des agrégats (§22.12) |
| Mise en service, reprise | **28** | N1-5 utilise l'outil de reprise rejouable |

### 3.2 Règles de frontière [R]

1. **Une seule porte, des circuits propriétaires** (R-DM1). Le catalogue de N1 liste toutes les démarches ouvertes à la personne ; une démarche dont une brique possède déjà le circuit complet est un **raccourci** (lien profond) et ne crée **aucune** demande N1. Test : aucune table de N1 ne contient d'absence, de réservation, de sortie ou de justificatif.
2. **N1 ne lit aucune table d'une autre brique** et n'y écrit jamais : ports et lectures publiées seulement (D-026). Garde de code : un test lit le SQL des autres modules et échoue s'il contient `INSERT`, `UPDATE` ou `DELETE` sur `ent_dem_*`.
3. **Une nature déclarée par une autre brique garde son métier chez elle** : N1 porte le guichet (dépôt, routage, prise en charge, échanges, délais, preuve) ; la brique porte son **volet** (champs propres, effets à la clôture, suivi technique) dans ses tables, liées par `demande_id` sans clé étrangère inter-brique (même règle que `document_id` au socle 7).
4. **Demande ou conversation ?** (R-DM2) Une conversation (16) devient une demande quand il faut un **engagement** (une décision, une production, une intervention, un délai) ; la conversion ferme le fil en le disant à la famille (« Votre message est devenu la demande n° 2026-00123, suivez-la ici »). Une demande ne s'ouvre **jamais** de fil de messagerie parallèle : ses échanges sont **dans** la demande (§12).

---

## 4. Acteurs, besoins réels, irritants

Pour chaque acteur : ce qui lui fait perdre du temps, ce qu'il redoute, ce qui le ravirait [H, à confirmer avec Florian et les personnels de l'établissement pilote].

| Acteur | Perd du temps | Redoute | Serait ravi de |
|---|---|---|---|
| **Chef d'établissement** | être sollicité dans le couloir pour des choses qui ne sont pas de son ressort ; relancer « qui s'occupe de ça ? » ; signer des HSE sans savoir ce qui a été fait | une demande grave (plainte, conflit, sécurité) restée trois semaines sans réponse ; une HSE payée deux fois ; découvrir une demande RH qu'un agent technique a lue | ne voir que ce qui doit monter jusqu'à lui (escalade explicite), avec un tableau « en retard » par file, sans le contenu confidentiel des autres ; signer d'un geste un état d'HSE dont chaque ligne est attestée |
| **Adjoint(e) de direction** | arbitrer des HSE par e-mail ; suivre une enveloppe d'heures dans un tableur ; gérer les photocopies couleur | dépasser l'enveloppe en avril ; valider des heures jamais faites | voir l'enveloppe consommée et engagée en temps réel ; attester le service fait par lot à partir des séances réalisées |
| **Secrétariat de direction / scolarité** | ressaisir des demandes reçues au téléphone, au guichet, par e-mail ; être « la corbeille » de tout l'établissement | une demande de famille perdue (certificat pour un dossier de bourse, duplicata pour un concours) ; être tenu pour responsable d'un oubli d'un autre service | une file claire, des **réponses types**, le dépôt **pour le compte** d'une famille au comptoir en 30 secondes, une famille qui suit seule l'avancement au lieu de rappeler |
| **Comptabilité, intendance** | les demandes de facture, d'attestation de paiement, de remboursement mêlées aux e-mails ; les demandes d'achat sans devis | payer sans pièce ; une demande de remboursement sans décision tracée | une nature par besoin avec les pièces exigées dès le dépôt ; un historique opposable au contrôle |
| **Gestionnaire RH (organisme de gestion)** | les demandes d'attestation employeur, de changement de RIB, de mutuelle par e-mail | qu'un collègue lise la demande d'un salarié | une file RH **confidentielle** dont même la direction ne voit que le nombre |
| **Professeurs** | chercher à qui demander ; redemander ; ne pas savoir si les copies seront prêtes à 8 h ; remplir deux fois une HSE | arriver en classe sans ses copies ; une HSE refusée sans motif ; que sa demande traîne parce que quelqu'un est absent | déposer en 3 gestes depuis le téléphone, voir « prête — retrait salle des professeurs », voir ses HSE accordées, faites et attestées |
| **Professeur principal** | relayer des demandes des familles de sa classe | une demande de famille adressée à lui pour un autre service | « transmettre au bon service » d'un geste, la famille en étant informée |
| **CPE, vie scolaire, AED** | le tri des demandes des familles (attestations, carnets perdus, cartes d'élève) ; les demandes de matériel | qu'une demande d'élève relevant du harcèlement soit traitée comme une demande ordinaire | les natures « carte d'élève », « carnet », « casier », traitées en file, et le **renvoi protégé** vers le 13/36 quand une demande révèle autre chose |
| **Agents de maintenance** | les demandes floues (« ça ne marche pas en B12 »), les doublons, les appels dans le couloir | une demande de sécurité (fuite, électricité) noyée parmi les ampoules | une file triée par urgence, avec salle, photo et « déjà signalé », depuis le téléphone ; clore avec une photo |
| **Agents d'entretien, ménage** | être prévenus au dernier moment d'une salle à préparer ; les demandes orales | être accusés d'un oubli dont ils n'ont pas été informés | une file simple, les demandes du jour par bâtiment, un « fait » en un geste |
| **Reprographie (réceptionnaire)** | les demandes reçues à la dernière minute, les PDF illisibles, les quantités erronées | l'imprimante qui tombe la veille d'un examen blanc ; imprimer des sujets visibles par d'autres | une file par site, triée par « pour quand », quantités pré-remplies depuis les effectifs, « prête » en un geste |
| **Accueil** | prendre des messages sur papier ; ne pas savoir à qui transmettre | transmettre une demande à la mauvaise personne ; donner une information à un appelant non vérifié | **déposer pour le compte** d'une famille ou d'un visiteur identifié, avec le bon routage automatique, et dire « on vous répond sous 3 jours » |
| **Infirmière, PsyEN, assistante sociale** | — (peu de demandes passent par N1) | qu'une demande révèle une santé ou un suivi | que les natures sensibles **n'existent pas** dans N1 (elles vont à 34, 36) |
| **Familles** | appeler trois fois pour un certificat ; ne pas savoir si le message a été lu ; se déplacer pour un papier | être ignorées ; que l'autre parent voie leur demande (séparation) | un catalogue simple, un accusé daté, l'état en temps réel, la pièce produite **dans le coffre** (socle 7) |
| **Élèves mineurs** | demander une carte d'élève, un casier, un accès au compte par un adulte | être moqués (demande visible) | quelques natures à leur portée, simples, sur le téléphone |
| **Élèves majeurs** | que les démarches les concernant passent encore par leurs parents | que ses parents voient ses demandes | déposer lui-même toutes les natures ouvertes aux responsables, sans que ses parents les voient (selon le socle 4) |
| **Organisme de gestion, collectivité** | être saisi sans référence | — | recevoir une demande qualifiée (lieu, catégorie, photo) avec sa référence (44 §12.2) |
| **DPO** | ne pas savoir où sont les données des demandes, ni combien de temps | une demande contenant une donnée de santé partie à l'IA ou par e-mail | une durée de conservation par nature, un export de preuve, un journal d'accès pour le confidentiel |
| **Administrateur ScolIA (Florian)** | reparamétrer à chaque arrivée et départ | une bascule de février qui perd des demandes ouvertes | des files qui suivent seules les missions, une reprise rejouable et vérifiée |

**Ce que montrent ces portraits [R].** (1) La première attente est la **réponse à « qui s'en occupe et quand »** : l'état et l'échéance visibles du demandeur valent plus que toute fonction. (2) Le risque principal est la **mauvaise lecture** (RH lue par un agent, sensible traité comme ordinaire), pas le mauvais routage : un mauvais routage se corrige, une lecture ne se reprend pas. (3) Le personnel a besoin de **dépôts pour le compte** (accueil, secrétariat) : un guichet qui suppose que chacun dépose lui-même exclut les familles sans numérique. (4) Les demandes techniques vivent **sur le téléphone**, près de la panne.

---

## 5. L'année vue du terrain [H, à confirmer avec Florian]

| Période | Ce qui arrive au guichet |
|---|---|
| **Fin août – septembre** | pic : maintenance de rentrée (salles, vidéoprojecteurs, clés), comptes et mots de passe (assistance), cartes d'élève et casiers, certificats de scolarité (bourses, transports, CAF), photocopies de rentrée en masse, premières demandes d'HSE (accompagnement personnalisé, ateliers) ; demandes RH d'arrivée |
| **Octobre – novembre** | HSE des projets du premier trimestre ; photocopies d'évaluations ; demandes de remboursement (voyages annulés) ; attestations pour les familles |
| **Décembre** | examens blancs (reprographie massive, délais serrés) ; fermeture de Noël (délais suspendus) ; **répétition de bascule** (D-057) |
| **Janvier – février** | HSE du deuxième trimestre (soutien avant examens) ; **bascule des vacances d'hiver** (20 février – 8 mars 2027) ; demandes de dérogation et de stage |
| **Mars – mai** | préparation des examens (reprographie, HSE de préparation) ; demandes de duplicata pour les concours ; enveloppe d'HSE à surveiller |
| **Juin – juillet** | clôture de l'année : HSE attestées et état de juin, demandes de relevés de notes et d'attestations de fin d'année, départs des agents (redistribution des demandes), fermeture d'été (délais suspendus, accusés automatiques) |
| **Toute l'année** | pannes, ménage, fournitures, assistance numérique, demandes des familles au secrétariat et à la comptabilité |

---

## 6. Ce qui est réglementaire

Aucune vérification en ligne n'a été faite dans cette session : les règles ci-dessous sont **[NV]** (citées de mémoire, à vérifier sur Légifrance pendant la recette réglementaire) ou **[S]** (reprises d'une conception qui les a vérifiées).

### 6.1 Établissements publics (EPLE) : relations avec le public (CRPA) [NV]

Un EPLE est une administration au sens du code des relations entre le public et l'administration. Pour une **demande d'un usager** (famille, élève majeur) qui appelle une décision :
- **Accusé de réception** (art. L112-3, R112-5) : date de réception, service qui instruit, délai au terme duquel le silence vaut décision, voies de recours ; à défaut, les délais de recours ne sont pas opposables (L112-6).
- **Saisine par voie électronique** (L112-8 et suivants) : l'usager peut saisir l'administration par voie électronique ; un accusé électronique est dû.
- **Demande incomplète** (L114-5) : l'administration indique les pièces manquantes et le délai pour les fournir ; le délai de décision ne court qu'à la réception des pièces.
- **Administration incompétente** (L114-2) : elle **transmet** à l'autorité compétente et en avise l'intéressé.
- **Silence de l'administration** (L231-1, L231-4) : en principe, le silence gardé **deux mois** vaut acceptation, avec de nombreuses exceptions (demandes qui ne tendent pas à une décision individuelle, réclamations, matières financières…).
→ **Conséquences** [R] : chaque nature déclare si elle est une **demande d'usager appelant une décision** (`decision_usager`) ; pour un établissement public, son accusé de réception porte les mentions ci-dessus (gabarit du socle 7) ; l'état « en attente de complément » suspend le délai (R-DM20) ; une demande mal adressée est **transmise** avec avis au demandeur (R-DM12, conforme à L114-2) ; l'échéance de décision implicite est calculée et montrée au personnel (R-DM21). **Q-DM1** : liste des natures familles soumises à ces règles, à faire valider (DPO, juriste du rectorat).

### 6.2 Établissements privés sous contrat [H]

L'établissement privé n'est pas une administration au sens du CRPA pour ses relations avec les familles (relation contractuelle) [H]. Mêmes **bonnes pratiques** par défaut (accusé daté, délai annoncé, transmission au bon service), sans mentions de recours. Le réglage vient du profil d'établissement (public / privé, socle 5), pas d'un paramètre de N1.

### 6.3 Données personnelles [NV]

- **Minimisation** (RGPD art. 5.1.c) : les champs d'une nature sont **déclarés** et justifiés ; pas de texte libre obligatoire quand une liste suffit ; avertissement visible « n'écrivez pas de donnée de santé » sur les natures ouvertes au texte libre (R-DM30).
- **Conservation** (art. 5.1.e) : durée par nature, au registre du socle 27 (§15.3).
- **Droits des personnes** (art. 12 à 22) : la réponse est due dans un mois ; c'est le **socle 27** qui les instruit (nature `rgpd.exercice_droit` déclarée par lui).
- **Sous-traitance de l'IA** (art. 28) : Mistral seulement, sous les conditions de D-055 ; aucun identifiant d'élève (§20).

### 6.4 HSE et heures des personnels [S]

- Repris de `15_ABSENCES_PERSONNELS.md` §5 : **R-04** (régime spécifique des HSE de remplacement de courte durée supprimé au 01/09/2023 ; anciens plafonds caducs ; RCD par le Pacte ou en HSE de droit commun) ; **R-05** (HSE : 1/36ᵉ du taux annuel de l'HSA majoré de 25 % ; ScolIA **ne calcule pas** la rémunération des agents de l'État, il **constate** et **exporte**) ; **R-06** (pas d'HSE pour remplacer un congé de convenance personnelle : alerte sans blocage).
- **Agents de l'État** (enseignants du public, maîtres contractuels du privé sous contrat d'association) : payés par l'État sur **état de service fait** signé par le chef d'établissement (15 §14) [S].
- **Salariés de l'organisme de gestion** : heures supplémentaires et complémentaires au sens du code du travail (décompte, majorations, contingent) [NV] : calculées par la paie (49, D-048), pas par N1.
- **Enveloppe d'HSE** : dotation de l'établissement par l'autorité académique [H] (forme et unité à confirmer : heures ou euros, Q-DM6).

### 6.5 Reprographie [S]

Licence du CFC (`44_RESSOURCES_MATERIELLES.md` §5.5, notice de janvier 2025 lue par le 44) : extraits seulement (au plus 10 % d'un livre, 30 % d'une revue), au plus 180 pages par élève et par an, déclaration par échantillon. → case « contient des extraits d'œuvres protégées » et rappel des plafonds (§18).

### 6.6 Accessibilité, déconnexion [S]

RGAA pour tous les écrans (`29_PORTAILS.md` §5.2) ; plage calme de l'arrêté du 7 septembre 2026 (20 h – 7 h, week-end) appliquée par le socle 8 aux avis de N1 (`29_PORTAILS.md` §5.1) : aucun avis non urgent hors plage, aucune relance à un personnel hors de ses horaires.

### 6.7 Actions et règles issues du réglementaire

R-DM20, R-DM21, R-DM12 (§6.1) ; DEM-112 (accusé de réception à mentions) ; DEM-222 (échéance de décision implicite) ; R-DM30 (avertissement de minimisation) ; §17.7 (pas de calcul de rémunération).

---

## 7. Concepts et vocabulaire [R]

| Mot | Sens fixé | Table |
|---|---|---|
| **Demande** | Un besoin exprimé à l'établissement, d'une nature donnée, qui attend un traitement par une personne identifiée dans un délai | `ent_dem_demande` |
| **Nature** | Le type d'une demande, **déclaré dans le code** par une brique (champs, publics, routage, confidentialité, délais, circuit, avis, tâches, effets) ; activée par l'établissement | déclaration Rust + `ent_dem_nature_etab` |
| **Guichet** | Le catalogue des natures ouvertes à une personne, et le parcours de dépôt | — |
| **Raccourci** | Entrée du catalogue qui ouvre le parcours d'une autre brique (absence, salle, sortie) et ne crée pas de demande | déclaration |
| **Demandeur** | La personne au nom de qui la demande est faite (personne du socle 4) | colonne |
| **Déposant** | La personne qui a saisi la demande, si elle n'est pas le demandeur (accueil, secrétariat, collègue) | colonne |
| **Bénéficiaire / sujet** | Ce sur quoi porte la demande : un élève (dans les droits du demandeur), un agent (HSE pour un collègue), un lieu, un bien | colonnes typées |
| **File de service** | Une file de traitement : membres calculés (missions, fonctions, équipes du 6), responsable, horaires, délai visé, file de repli | `ent_dem_file` |
| **File de tri** | File désignée qui reçoit ce que le routage ne sait pas placer ; jamais « tout le personnel » | `ent_dem_file` (type `tri`) |
| **Destination** | Personne désignée, porteurs d'une action D-017 sur le sujet, ou file | colonnes de routage |
| **Prise en charge** | L'attribution de la demande à **un** agent de la file ; une seule à la fois | colonne + faits |
| **Délégation** | Attribution d'une demande par le responsable de file à un membre ; ou délégation datée du socle 6 qui fait suivre les demandes nominatives | faits ; `ent_delegation` (6) |
| **Transfert** | Changement de file, motivé, avec avis au demandeur (« transmise au secrétariat ») | fait |
| **Consultation** | Question posée à un tiers (autre file, agent) sans lui confier la demande ; il répond dans la demande | `ent_dem_consultation` |
| **Escalade** | Ajout d'un niveau de responsabilité (responsable de file, direction) à l'échéance ou sur geste ; n'ôte pas le responsable | fait |
| **Échéance** | Date due : prise en charge, traitement, complément, décision implicite | `ent_dem_echeance` |
| **Suspension** | Arrêt du décompte du délai (attente du demandeur, fermeture de l'établissement) | faits |
| **Échange** | Message dans la demande : **au demandeur** (visible de lui) ou **interne** (jamais visible de lui) | `ent_dem_message` |
| **Pièce** | Fichier déposé au socle 7 et rattaché à la demande (justificatif, annexe, réponse) | `ent_dem_piece` |
| **Décision** | Pour les natures qui en ont : accordée, accordée en partie, refusée (motif codé + texte au demandeur), sans objet | colonnes + faits |
| **Issue** | Comment la demande se termine : `traitee`, `accordee`, `accordee_partiellement`, `refusee`, `sans_objet`, `annulee_par_demandeur`, `doublon`, `transmise_hors_scolia`, `close_sans_reponse_du_demandeur` | colonne |
| **Fait** | Ligne d'historique, ajout seul, chaînée par empreinte | `ent_dem_fait` |
| **Volet** | Partie métier propre à une nature, dans les tables de la brique propriétaire (HSE, reprographie, intervention technique) | tables de la brique |
| **Confidentialité** | Classe C0 à C3 (IA_COEUR §7.2) de la nature, qui fixe qui lit quoi | colonne |

### 7.1 Quatre distinctions qui manquaient dans Next.js

1. **Demandeur ≠ déposant ≠ bénéficiaire** (NJ-84, NJ-91) : trois colonnes, trois personnes possibles, toutes issues du serveur.
2. **Échange au demandeur ≠ note interne** (NJ-82) : deux visibilités, deux projections.
3. **Accordé ≠ fait ≠ attesté** (HSE, NJ-91) : trois faits distincts.
4. **Clôture ≠ effacement** (NJ-89) : une demande close reste lisible (par qui le doit) jusqu'à la fin de sa conservation.

---
## 8. Natures de demande

### 8.1 Principe [R]

Une nature est **déclarée dans le crate de la brique qui en porte le métier**, comme une nature d'avis (`08_NOTIFICATIONS.md` §6.1) ou une nature de tâche (`29_PORTAILS.md` §8.2). L'assemblage vérifie la déclaration (§8.5). L'établissement **active** une nature, choisit les publics parmi ceux qu'elle autorise, peut **raccourcir** un délai ou désigner la file qui la reçoit ; il ne crée ni champ ni circuit (D-023 : peu de réglages, signalés). Le besoin d'un formulaire local sans traitement relève du socle 18 ; le besoin d'une nature nouvelle est un **lot de code** (une déclaration de quelques dizaines de lignes, sans migration si la nature n'a pas de volet).

### 8.2 Déclaration (forme indicative, le lot N1-1 fixe les noms)

```rust
pub const ATTESTATION_SCOLARITE_DIVERSE: NatureDemandeDecl = NatureDemandeDecl {
    code: "scolarite.attestation",               // stable ; préfixe = domaine de la brique propriétaire
    version: 1,                                   // nouvelle version = nouveaux champs ; les demandes gardent la leur
    proprietaire: Brique::Demandes,               // N1 pour ses natures intégrées ; sinon la brique (Ressources, Rh…)
    libelle: "Demander une attestation",
    mots_cles: &["attestation", "certificat", "justificatif", "assurance", "CAF"],
    publics: &[Public::Responsable(DroitLien::RecevoirDocuments), Public::EleveMajeur, Public::Personnel],
    sujet: Sujet::EleveDansMesDroits,             // Aucun | Moi | EleveDansMesDroits | Agent | Lieu | Bien
    champs: &[
        Champ::liste("type_attestation", &["scolarite_periode", "assiduite", "inscription_examen", "autre"]),
        Champ::date_facultative("periode_du"), Champ::date_facultative("periode_au"),
        Champ::texte_court_facultatif("precision", 280),
    ],
    texte_libre: TexteLibre::Facultatif { max: 2000, avertissement_sensible: true },
    pieces: Pieces::Aucune,                       // Aucune | Facultatives(n) | Exigees(&[TypePiece])
    routage_defaut: Routage::File(CodeFile::SECRETARIAT_SCOLARITE), // §10
    critere_routage: &[Critere::SiteDeLEleve, Critere::CycleDeLEleve],
    confidentialite: Sensibilite::C1,
    delais: Delais { prise_en_charge_jours_ouvres: 2, traitement_jours_ouvres: 5, borne_min_etab: 1 },
    circuit: Circuit::Simple,                     // Simple | AvecDecision | AvecValidation(n étapes) | AvecVerification
    decision_usager: true,                        // §6.1 (EPLE : accusé à mentions, décision implicite)
    issue_produit: Some(Production::Document("scolarite.attestation")), // émis par le socle 7 (gabarit déclaré)
    cardinalite: Cardinalite::ParDemandeur,       // ParDemandeur | ParFoyer | ParEleve (doublons, §9.5)
    annulable_par_demandeur: Annulation::TantQueNonPrise,
    ia: IaNature { suggestion_nature: true, resume: true, brouillon_reponse: true },
    avis: &[AVIS_DEPOT, AVIS_PRISE, AVIS_COMPLEMENT, AVIS_CLOTURE], // natures d'avis (8)
    taches: &[TACHE_A_TRAITER, TACHE_COMPLEMENT],                  // natures de tâche (29)
    effets_cloture: &[],                          // ports appelés dans la transaction de clôture
    conservation: CategorieConservation::DemandeFamilleCourante,   // §15.3, registre du 27
    raccourci: None,                              // Some(LienProfond) pour un raccourci (aucune demande créée)
};
```

### 8.3 Le volet d'une autre brique [R]

Une brique qui déclare une nature avec un traitement propre fournit, **dans son crate** : ses champs (typés, dans ses tables, clé `(etablissement_id, demande_id)`), une **lecture publiée** de son volet (pour l'écran de la demande et les avis), ses **commandes** (ex. 44 : planifier l'intervention, saisir le temps passé, condamner un lieu) et ses **réactions** aux événements de N1 (`demandes.prise`, `demandes.close`). N1 garde le cycle de la demande ; la brique ne change jamais le statut de la demande directement : elle appelle `ServiceDemandes::avancer(tx, acteur, demande_id, Geste)` (§28.3), qui vérifie les droits et écrit le fait.

### 8.4 Catalogue initial

Colonnes : **P** propriétaire (N1 = nature intégrée au guichet, sinon numéro du socle) ; **Pub** publics (PER personnel, RESP responsable, ELV élève mineur, MAJ élève majeur) ; **C** confidentialité ; **Lot**. Les natures marquées **R** sont des **raccourcis** (aucune demande N1).

**Personnels — fonctionnement interne (bascule de février : remplace M04, M08)**

| Code | Libellé | P | Pub | C | File par défaut | Particularités | Lot |
|---|---|---|---|---|---|---|---|
| `numerique.assistance` | Assistance numérique (compte, mot de passe, poste, imprimante, logiciel, ENT) | N1 | PER (ELV, RESP pour « accès au compte ») | C1 | référent numérique | zone concernée (liste), capture facultative ; remplace l'« assistance » par e-mail (NJ-95) | N1-1 |
| `maintenance.intervention` | Signaler une panne ou un problème dans un lieu | N1 → **44** | PER | C0 | maintenance du site | salle (du 5), catégorie, photo, urgence ; doublon « déjà signalé » ; catégorie sécurité → alerte (44 §12.1) ; volet technique repris par le 44 (RM4) | N1-1 |
| `entretien.demande` | Demande d'entretien ou de ménage (salle à préparer, nettoyage, poubelles) | N1 → **44** | PER | C0 | entretien du site | lieu, date souhaitée ; récurrence non (le 44 planifie) | N1-1 |
| `reprographie.tirage` | Demande de reprographie (copies) | N1 → **44** | PER | C1 | reprographie du site | volet §18 | N1-3 |
| `rh.hse` | Demande d'heures supplémentaires effectives | N1 | PER (enseignant ; pour le compte : SEC, CPT, DIR) | C1 | direction du site | volet §17 | N1-3 |
| `rh.demande_agent` | Demande à la gestion du personnel (attestation employeur, changement de coordonnées bancaires, mutuelle, formation, question de contrat) | N1 → **49** | PER | **C3** | RH (organisme de gestion ; public : secrétariat de direction) | file confidentielle ; jamais de motif dans les avis ; pièces chiffrées ; remplace la file « RH » forcée (NXN-14) | N1-1 |
| `comptabilite.demande_interne` | Demande à la comptabilité (facture, bon de commande, avance, remboursement de frais d'un personnel) | N1 → **48** | PER | C1 | comptabilité | montant (décimal, euros), pièces exigées selon le type | N1-1 |
| `achat.fournitures` | Demande de fournitures ou de petit matériel | N1 → **48** / **44** | PER | C0 | intendance | quantité, référence, budget de la discipline (lecture du 48 quand il existe) | N1-1 |
| `secretariat.demande_interne` | Demande au secrétariat (courrier, édition, convocation à préparer, appel à passer) | N1 | PER | C1 | secrétariat du site | | N1-1 |
| `direction.question` | Question ou demande à la direction | N1 | PER | C1 (C2 au choix du demandeur) | **secrétariat de direction** qui qualifie (NXN-05) | escalade explicite vers le chef (R-DM14) | N1-1 |
| `vie_scolaire.demande_interne` | Demande à la vie scolaire (surveillance, salle d'étude, appel à faire passer) | N1 | PER | C1 | vie scolaire | | N1-1 |
| `ressources.cle` | Demande de clé ou de badge | **44** | PER | C0 | accueil / intendance | prêt signé au 44 | après N1 (RM6) |
| `ressources.materiel` | Emprunt de matériel | **44** | PER | C0 | — | **R** : moteur de réservation (19/44) | — |
| `agenda.salle` | Réserver une salle | **19** | PER | C0 | — | **R** | — |
| `absences.declarer` | Déclarer une absence, demander une autorisation d'absence | **15** | PER | C3 | — | **R** (circuit complet au 15) | — |
| `sorties.projet` | Proposer une sortie ou un voyage | **42** | PER | C1 | — | **R** | — |

**Familles et élèves (ouvertes à la bascule si Florian le décide, Q-DM2 ; sinon après)**

| Code | Libellé | P | Pub | C | File par défaut | Particularités | Lot |
|---|---|---|---|---|---|---|---|
| `scolarite.certificat` | Certificat de scolarité | **7** | RESP, MAJ | C0 | — | **R** : émis **immédiatement** par le socle 7, sans demande (R-DM3 : ce qui peut être servi tout de suite ne devient pas une demande) | — |
| `scolarite.attestation` | Autre attestation (assiduité, période, examen) | N1 | RESP (`recevoir_documents`), MAJ | C1 | secrétariat élèves | document émis à la clôture (7) | N1-1 |
| `scolarite.duplicata` | Duplicata de bulletin, relevé de notes, diplôme d'établissement | N1 → 23 | RESP, MAJ, anciens élèves (Q-DM3) | C1 | secrétariat élèves | si le bulletin est au coffre (7) : raccourci « le télécharger » | N1-1 |
| `scolarite.carte_eleve` | Carte d'élève, carnet de correspondance perdu, casier | N1 | ELV, MAJ, RESP | C0 | vie scolaire | éventuel coût (24) | N1-1 |
| `secretariat.demande_famille` | Demande au secrétariat (autre) | N1 | RESP, MAJ | C1 | secrétariat élèves | texte libre ; transfert fréquent | N1-1 |
| `comptabilite.demande_famille` | Demande à la comptabilité (attestation de paiement, facture, question de facture) | N1 → 24 | payeurs (24), MAJ payeur | C1 | comptabilité | lecture du compte du payeur seulement | N1-1 |
| `finances.remboursement` | Demande de remboursement | **24/25** | payeurs | C1 | comptabilité | déclarée par le 24 (RIB par le 25, jamais en clair) | après N1 |
| `finances.echeancier` | Demande de délai de paiement, échéancier | **25** | payeurs | C2 | comptabilité | | après N1 |
| `aides.fonds_social` | Demande d'aide (fonds social) | **26** | RESP, MAJ | C3 | commission | déclarée par le 26 | après N1 |
| `administratif.coordonnees` | Changer mes coordonnées, ma situation | **4** | RESP, MAJ | C1 | — | **R** : proposition du socle 4 | — |
| `restauration.regime` | Changer de régime (demi-pension, externat) | **40** | RESP, MAJ | C1 | — | **R** ou nature du 40 | — |
| `inscriptions.depart` | Départ, radiation, exeat | **3** | RESP, MAJ | C1 | — | **R** | — |
| `examens.amenagement` | Aménagement d'examen | **38/35** | RESP, MAJ | C2 | — | **R** | — |
| `agenda.rdv` | Demander un rendez-vous | **19** | RESP, ELV, MAJ | C1 | — | **R** | — |
| `assiduite.justifier` | Justifier une absence | **12** | RESP, MAJ | C1 | — | **R** | — |
| `rgpd.exercice_droit` | Exercer un droit sur mes données | **27** | toute personne | C2 | DPO / direction | déclarée par le 27 (délai d'un mois, §6.3) | après N1 |
| `numerique.acces_compte` | Je n'arrive pas à me connecter | N1 | RESP, ELV, MAJ (sans compte : par l'accueil) | C1 | référent numérique | parcours de récupération du socle 1 d'abord (R-DM3) | N1-1 |

**Jamais une demande N1** (R-DM31) : signalement de harcèlement, de violence, de danger, d'information préoccupante (13, 36) ; question de santé (34) ; plainte contre un personnel (direction, hors catalogue : courrier ou rendez-vous) ; recours contre une sanction (13). Le catalogue affiche une entrée « **Je veux signaler une situation grave** » qui ouvre le canal protégé du 36 ou du 13 et les numéros d'urgence (3018, 119, 15, 17), comme la messagerie (R-M26 du 16).

### 8.5 Ce que vérifie l'assemblage [R]

Code unique et préfixé par un domaine de la brique propriétaire ; libellé et mots-clés en français ; publics compatibles avec la confidentialité (aucun public ELV sur une nature C2+) ; **aucune variable de confidentialité > C1 dans un gabarit d'avis externe** (lint du socle 8) ; routage par défaut résolu vers une file **et** une file de repli ; délais présents (sauf raccourci) ; natures d'avis et de tâches déclarées ; catégorie de conservation connue du 27 ; pour une nature avec `decision_usager`, gabarit d'accusé de réception à mentions ; pour un raccourci, lien profond enregistré et **aucun** champ. Un test refuse une nature qui ne respecte pas ces règles (`t_kit_nature_complete`).

---

## 9. Le guichet

### 9.1 Trouver la bonne nature

- **Catalogue** « Faire une demande » : par catégories (Numérique, Locaux et matériel, Reprographie, Heures et paie, Secrétariat, Comptabilité, Direction, Vie scolaire, Scolarité), avec recherche par mots (mots-clés des natures, tolérance aux fautes, sans IA par défaut).
- **Les plus utilisées par moi** et **les plus utilisées dans mon établissement** en tête (calcul local, aucun profilage partagé).
- **Proposition par l'IA** (facultative, N1-6) : la personne décrit son besoin en une phrase, l'IA propose **une à trois natures** ; la personne choisit ; rien n'est déposé par l'IA (§20).
- **Raccourcis** : une nature raccourci ouvre le parcours de la brique ; un bandeau dit « cette démarche se fait ici ».
- **« Je ne trouve pas »** : nature `secretariat.demande_interne` (personnels) ou `secretariat.demande_famille` (familles), qui part à la **file de tri** du site ; jamais un message dans le vide.

### 9.2 Déposer

1. Champs de la nature (typés, validés côté serveur), objet court (≤ 140 caractères, pré-rempli par le libellé), texte libre si la nature l'autorise (avertissement « pas de donnée de santé ni d'information sur un tiers qui n'est pas nécessaire »).
2. Sujet : élève **dans les droits** du demandeur (liste proposée par le serveur : ses enfants, ses élèves), lieu (liste des salles du site), agent (HSE pour un collègue).
3. Pièces (socle 7 : type réel contrôlé, antivirus, taille plafonnée, identifiant émis par le serveur et lié au déposant, NJ-78).
4. **Aperçu** : « Votre demande ira au secrétariat du lycée ; réponse visée sous 5 jours ouvrés. » (conf. 1).
5. Dépôt : numéro lisible (`2026-00123`, séquence par établissement et par année scolaire), **accusé de réception** daté (dans l'ENT, et par le socle 8 selon les préférences), tâche « à traiter » ouverte à la file (29).
- **Brouillon** : conservé 30 jours pour son auteur seulement (SN1-04), jamais lu par la file.
- **Hors ligne** (mobile) : le brouillon est local ; le dépôt n'est jamais rejoué en silence au retour du réseau (R-PO20 du 29) : il est proposé à nouveau.

### 9.3 Déposer pour le compte d'une personne

- **Qui** : accueil, secrétariat, comptabilité, direction (`demandes.deposer_pour_compte`) ; un collègue pour une HSE ou une reprographie selon la nature.
- **Pour qui** : une personne du socle 4 (famille connue, agent), choisie dans une recherche bornée (nom + élève, ou nom + fonction) ; **jamais** une adresse e-mail libre.
- **Comment c'est arrivé** : canal codé (`comptoir`, `telephone`, `courrier`, `courriel_recu`, `conversion_message`) ; pour un appel, l'accueil **ne confirme aucune information** à un appelant non vérifié (NTF-147 du 8 : « rappeler au numéro connu »).
- **Trace** : le déposant est enregistré et voit ce qu'il a déposé (DRM-08) ; le demandeur voit « déposée par l'accueil le 12/11 à 10 h 05 » ; il reçoit l'accusé comme s'il avait déposé lui-même.

### 9.4 Accusé de réception

Contenu : numéro, nature, date et heure de réception, **service** qui traite (jamais le nom d'un agent avant la prise en charge), délai visé, et pour un EPLE et une nature `decision_usager` : délai de décision implicite et voies de recours (§6.1, gabarit du socle 7). Il est **dans l'ENT** (la demande elle-même) et sur le canal choisi (8). Une demande déposée pendant une fermeture reçoit l'accusé et la mention « l'établissement est fermé jusqu'au 5 janvier ; le délai court à la réouverture » (R-DM22).

### 9.5 Doublons et appuis

- Avant le dépôt, le serveur montre les demandes **ouvertes** de même nature et même sujet **visibles du demandeur** : les siennes (« vous avez déjà demandé une attestation le 3/11, en cours ») et, pour les natures C0 sur un lieu (maintenance), les signalements du même lieu et de la même catégorie (« déjà signalé hier : +1 ? ») sans le nom du signaleur.
- **Appui** : un « +1 » abonne la personne à la demande (avis de clôture) sans créer de doublon.
- **Cardinalité** déclarée : `ParFoyer` (une demande par foyer suffit), `ParEleve` (une attestation par élève et par période) ; un doublon n'est **jamais bloqué** pour une famille (elle peut avoir une raison) : il est signalé à la file, qui peut fusionner (DEM-159).

### 9.6 Mes demandes (demandeur)

Liste : numéro, objet, état en mots simples (« reçue », « prise en charge par le secrétariat », « en attente de votre réponse », « traitée »), échéance visée, dernier échange **qui lui est adressé** ; filtres ouvertes / closes ; action « compléter », « répondre », « annuler » (si la nature le permet), « relancer » (une fois par délai dépassé, R-DM24). Jamais : note interne, nom d'un agent consulté, file de tri, hypothèse de routage, adresse d'un personnel (NJ-82).

---

## 10. Routage

### 10.1 Principe [R]

Le routage est **déterministe** et **explicable** : la nature donne une destination par défaut ; des **règles de l'établissement** l'affinent par critères ; l'IA n'affecte rien (NJ-87). Chaque demande garde la **raison** de son routage (« règle : maintenance, site Lycée, catégorie électricité ») lisible des membres de la file, jamais du demandeur.

### 10.2 Les files [R]

| Champ | Exemple | Règle |
|---|---|---|
| Code, libellé | `secretariat_lycee`, « Secrétariat du lycée » | unique par établissement |
| Type | `service` · `tri` · `direction` · `confidentielle` | une file de tri au moins par site (repli) |
| Site | Lycée | facultatif (file commune aux sites) |
| **Membres** | règles **calculées** : mission « secrétariat » (6) sur le site ; fonction « agent de maintenance » ; équipe constituée ; agent désigné (daté) | **jamais une liste d'adresses** (NJ-94) ; aperçu des membres à la date |
| Responsable(s) | mission « responsable de service » ou agent désigné | au moins un ; sinon la file de repli |
| Superviseurs | direction (lecture des compteurs et délais, jamais du contenu C2+) | §14 |
| Horaires | 8 h – 17 h, jours ouvrés | pour les délais et la réponse automatique |
| Délai visé | 3 jours ouvrés | borne la nature (on ne rallonge pas un délai déclaré) |
| File de repli | secrétariat de direction | obligatoire (A13 du 29) |
| Réponses types | « Votre attestation est disponible dans vos documents. » | modèles textuels de la file |
| Message d'accueil | « Les demandes de reprographie reçues avant 16 h sont prêtes le lendemain. » | affiché au dépôt |

Les membres sont **recalculés à chaque lecture** (comme les droits, D-017) : une mission qui commence ou finit change la file **le jour même**, sans geste.

### 10.3 Les règles de routage [R]

Une règle = (nature ou catégorie, critères, destination, ordre, période). Critères disponibles : **site** (du lieu, de l'élève, du demandeur), **cycle ou niveau** de l'élève (école, collège, lycée, post-bac), **catégorie** (champ liste de la nature), **public** (personnel, famille), **urgence** déclarée. Destinations : une **file**, une **personne désignée** (rare : le référent numérique unique d'une petite école), ou les **porteurs d'une action D-017 sur le sujet** (« les personnes qui ont `absences.trancher` sur la classe de l'élève ») qui forment alors une file implicite. La première règle qui s'applique gagne ; à défaut, la destination par défaut de la nature ; à défaut, la **file de tri** du site. Un écran **simule** le routage (« une attestation pour un élève de 2nde du site Lycée irait à… », DEM-334).

### 10.4 La file de tri [R]

- Membres désignés (secrétariat de direction, par exemple), **jamais tout le personnel** (NJ-85).
- Le tri est un geste : choisir la nature exacte (ce qui corrige le routage et le délai) et/ou la file ; le demandeur est prévenu seulement si le délai visé change (« transmise au service comptabilité, réponse sous 5 jours »).
- **Prendre depuis la tri** n'affecte pas la demande à « la branche de l'agent » par devinette (NXN-06) : le membre choisit la file cible ou la traite lui-même s'il en est membre.
- Une file de tri qui a des demandes de plus de 2 jours ouvrés alerte son responsable puis la direction (escalade, §13).

### 10.5 Ce que le routage ne fait jamais [R]

Lire le texte libre pour router ; envoyer quoi que ce soit à un service externe pour router ; affecter à un agent nommé sans que la règle le dise ; affecter à une file dont aucun membre n'est actif (→ file de repli, avec signal au responsable de la file vide) ; router une nature C3 vers une file non confidentielle (contrôlé à l'assemblage et au paramétrage).

---

## 11. Prise en charge, délégation, transfert, absences

### 11.1 États d'une demande [R]

| État | Signification | Visible du demandeur comme | Transitions |
|---|---|---|---|
| `brouillon` | non déposée | « brouillon » | → `deposee`, supprimée par son auteur |
| `deposee` | reçue, dans une file, personne ne l'a prise | « reçue » | → `prise`, `transferee`, `annulee` |
| `prise` | un agent la traite | « prise en charge par le secrétariat » (nom de l'agent si la nature l'autorise, SN1-02) | → `en_attente_demandeur`, `en_attente_tiers`, `en_validation`, `traitee`, `deposee` (relâchée) |
| `en_attente_demandeur` | complément demandé | « en attente de votre réponse » + ce qui est demandé | → `prise` (réponse), `close` (sans réponse après le délai, avec avis préalable) |
| `en_attente_tiers` | consultation, commande, intervention extérieure | « en cours » (+ motif public codé : « pièce commandée ») | → `prise` |
| `en_validation` | étape de décision (HSE, couleur, montant au-delà d'un seuil) | « en cours de décision » | → `prise` (accordée), `close` (refusée) |
| `traitee` | le traitant a terminé ; vérification facultative (sécurité, intervention) | « traitée » | → `close` (vérifiée ou délai de vérification écoulé), `prise` (vérification refusée) |
| `close` | terminée, avec une **issue** (§7) | « terminée : accordée / refusée / traitée » | → `rouverte` (dans le délai de réouverture, SN1-05) |
| `rouverte` | même problème, même demande | « rouverte » | → `prise` |
| `annulee` | annulée par le demandeur avant prise (ou selon la nature) | « annulée » | finale |

Le **transfert** n'est pas un état : c'est un fait qui change la file et remet la demande à `deposee`.

### 11.2 Prendre, relâcher, réattribuer [R]

- **Prendre** : un membre **actif** de la file (droit `demandes.traiter` sur la file, recalculé) ; contrainte d'unicité : une seule prise active par demande ; deux prises simultanées → la seconde reçoit « déjà prise par X à 10 h 02 » (NJ-90). Prendre ouvre la tâche nominative « en cours » (29) et ferme la tâche de file.
- **Relâcher** : celui qui l'a prise ; motif facultatif ; la demande revient à la file (avec son historique).
- **Réattribuer** : le responsable de la file, à un **membre** de la file ; journalisé ; avis au nouvel attributaire.
- **Expiration de prise** : une prise sans geste depuis N jours ouvrés (SN1-03, défaut 5) est **signalée** au responsable (pas relâchée seule : on ne retire pas une demande à quelqu'un qui y travaille hors ligne).
- **Jamais** : prendre une demande d'une file dont on n'est pas membre, même « déjà prise » (NJ-81) ; le responsable d'une autre file ne voit pas la demande.

### 11.3 Délégation [R]

- **Délégation ponctuelle** (NXN-09) : le responsable confie une demande à un membre de sa file (= réattribution motivée).
- **Délégation datée du socle 6** (`ent_delegation`, objet `mission` ou `role`) : pendant la délégation, le délégataire est **membre** des files que la mission déléguée ouvre ; les demandes **prises** par le délégant **restent** à son nom et sont **aussi visibles** du délégataire, qui peut les reprendre d'un geste (« pour Mme X ») ; à la fin de la délégation, rien ne bouge seul.
- **Pas d'escalade de droits** : déléguer une demande C3 à un membre qui n'a pas le droit de la lire est refusé (anti-escalade du 6).

### 11.4 Absences et remplacements (socle 15) [R]

- N1 lit `absences::indisponibilites(agent, plage)` (sans motif).
- **Au début d'une absence** d'un agent qui a des demandes prises : chaque demande dont l'échéance tombe pendant l'absence est **signalée à sa file** (« 3 demandes de Mme Y échoient pendant son absence ») et au responsable ; la file voit « absent jusqu'au 20/11 » sans motif ; **aucune réattribution sans geste** (29 §8.9), sauf si la file l'a choisi (réglage SN1-06 : « remettre en file à l'absence de plus de 2 jours »).
- **Absence du seul membre** d'une file : les nouvelles demandes vont à la file de repli **pendant l'absence** (règle temporaire calculée, pas un paramétrage), le demandeur voit le délai recalculé.
- **Suppléant nommé** (socle 6, `nommer_suppleant`) : il hérite de la mission, donc des files ; les demandes du titulaire lui sont proposées (« reprendre les 4 demandes de M. Z »).
- **Retour** : l'agent retrouve ses demandes prises (si elles n'ont pas été reprises) et un résumé de ce qui a bougé.

### 11.5 Départ d'un agent (socle 6) [R]

À J-15 de la fin d'engagement : l'agent et la direction voient ses demandes prises par file ; à la date de fin, les demandes encore prises **reviennent à leur file** (fait `rendue_au_depart`), avec un avis au responsable ; ses échanges restent signés de son nom ; ses demandes **déposées** restent visibles de lui jusqu'au départ, puis restent dans les files (le demandeur parti ne reçoit plus d'avis).

### 11.6 Transfert, consultation, escalade [R]

- **Transférer** à une autre file : un membre de la file actuelle ; motif codé (« hors de notre service », « relève de la comptabilité ») + note interne facultative ; la demande perd sa prise et repart `deposee` dans la file cible ; le demandeur voit « transmise au service comptabilité » (L114-2 pour un EPLE, §6.1) ; la file d'origine **ne voit plus** la demande sauf si elle reste consultée (R-DM12).
- **Consulter** un tiers (autre file, agent nommé dans ses droits) : question dans la demande, échéance ; le consulté voit la **question** et ce que le traitant choisit de partager (pas toute la demande si elle est C2+) ; sa réponse est un échange interne ; la demande reste chez le traitant.
- **Escalader** : au responsable de la file, puis à la direction ; geste ou échéance (§13) ; l'escalade **ajoute** un lecteur et une tâche, n'ôte pas le traitant.
- **Transmettre à la direction** (NXN-10) : réservé aux membres de la file et à son responsable ; pour les natures `direction.question`, c'est la voie normale après qualification par le secrétariat (R-DM14).

---

## 12. Échanges et pièces

### 12.1 Le fil de la demande [R]

- Deux visibilités : **au demandeur** (et au déposant, et aux co-demandeurs déclarés) ; **interne** (membres de la file, consultés, direction escaladée). L'interface les distingue par la couleur **et** un libellé (« Note interne — non visible de la famille »), jamais par la couleur seule (RGAA).
- Un message au demandeur est **relu** avant envoi (aperçu, conf. 1) ; il porte la signature de la file (« Secrétariat du lycée — Paul D. » ; le prénom et l'initiale par défaut, SN1-02).
- **Demander un complément** : liste de ce qui manque (pièce exigée, champ, précision) ; passe en `en_attente_demandeur`, ouvre la tâche « compléter » (29) chez le demandeur ; relance au demandeur à J+5 ouvrés (une seule) ; clôture « sans réponse » à J+15 ouvrés avec avis 3 jours avant (SN1-07).
- **Réponses types** de la file (texte, variables typées : prénom, numéro, date) ; aucune variable libre qui injecterait du HTML (rendu échappé, NJ-95).
- **Aucune modification** d'un message envoyé ; une correction est un nouveau message (« Correction : … ») ; un message interne peut être **masqué** par son auteur dans les 10 minutes (faute de frappe), le fait restant au journal.
- **Langue** : traduction proposée (IA, relue) pour un demandeur allophone (16 MSG-146) ; le texte original reste.

### 12.2 Pièces [R]

- Toujours par le **socle 7** : `ServiceDocuments::deposer` (type réel, antivirus, taille, identifiant serveur lié au déposant) ; N1 garde seulement `document_id` et le **rôle** de la pièce (`justificatif`, `annexe`, `reponse`, `photo_lieu`, `fichier_a_imprimer`).
- Lecture par **jeton court lié à la session** du socle 7 (§17.3 de `07_DOCUMENTS.md`), après contrôle de droit **par N1** : membre de la file, consulté avec partage de la pièce, demandeur pour les pièces qu'il a déposées ou qui lui sont adressées (NJ-83).
- **Pièce produite** (attestation, duplicata) : émise par le socle 7 (`emettre`, gabarit de la nature, signature si `peut_signer`), déposée **au coffre** du demandeur (et au dossier de l'élève si la nature le déclare) ; l'avis de clôture dit « disponible dans vos documents », **jamais en pièce jointe** (NJ-86).
- **Pièces sensibles** (RH, C3) : chiffrées au repos par le socle 7, lecture tracée (`tracer_acces_sensible`).

### 12.3 Abonnés [R]

Co-demandeur (l'autre responsable **du même foyer**, si le demandeur l'ajoute et que la nature le permet), appui (« +1 » sur une panne), copie interne (un collègue que le traitant veut tenir informé, dans ses droits). Un abonné reçoit les avis de clôture ; il ne voit que ce que sa qualité lui ouvre.

---

## 13. Délais, relances et escalade

### 13.1 Les échéances [R]

| Échéance | Calcul | Effet à l'échéance |
|---|---|---|
| **Prise en charge** | dépôt + délai de la nature (jours ouvrés du site, socle 5) | tâche de file → « due » ; à +1 jour : avis au responsable de la file |
| **Traitement** | dépôt + délai de traitement, **moins les suspensions** | à J-1 : rappel au traitant (dans son résumé du jour, 8) ; à l'échéance : escalade palier 1 (responsable de la file) ; à +N jours (SN1-08, défaut 5) : palier 2 (direction) |
| **Complément** | demande de complément + 5 jours ouvrés | relance unique au demandeur ; à +15 : clôture « sans réponse » annoncée 3 jours avant |
| **Décision implicite** (EPLE, `decision_usager`) | réception (ou réception des pièces) + 2 mois [NV] | affichée au personnel dès le dépôt ; alerte à J-15 et J-5 au responsable et à la direction ; jamais d'effet automatique |
| **Vérification** | traitée + 3 jours ouvrés | sans retour : `close` (vérifiée tacitement) ; sécurité : vérification **obligatoire** par la direction (44 §12.3) |
| **Pour quand** (reprographie, entretien) | date souhaitée | tri de la file ; « en retard » si dépassée |

- **Jours ouvrés** : calendrier du site (socle 5), fermetures et vacances exclues ; une demande déposée pendant une fermeture voit son délai partir à la réouverture (R-DM22).
- **Suspensions** : `en_attente_demandeur` et, si la nature le déclare, `en_attente_tiers` (pièce commandée) ; une suspension est un fait daté.

### 13.2 Relances [R]

- **Au personnel** : jamais une relance par demande et par jour ; elles entrent dans le **résumé du jour** du socle 8 (« 3 demandes à échéance aujourd'hui, 1 en retard »), dans ses horaires (plage calme, déconnexion, §6.6) ; une relance immédiate seulement pour une nature `urgence: haute` (sécurité, panne bloquante).
- **Au demandeur** : seulement pour un complément attendu (une fois) et pour l'annonce d'une clôture sans réponse.
- **Par le demandeur** (« relancer ») : une fois par délai dépassé ; la file reçoit un signal, le demandeur voit « relance envoyée le … » ; pas de bouton actif tant que l'échéance n'est pas passée (R-DM24).
- Le socle 8 **n'envoie aucune relance** pour une demande close (il interroge `etat_taches`, R-PO6 du 29).

### 13.3 Escalade [R]

Paliers déclarés par la file (défaut : responsable de file à l'échéance, direction à +5 jours ouvrés). L'escalade ajoute une tâche « demande en retard » au palier et un lecteur ; pour une nature C2+, la direction voit **l'existence et le retard**, le contenu seulement si elle a le droit de la nature (R-DM15). **Tableau des retards** par file pour la direction (compte, plus ancienne, médiane), sans nom d'agent hors de la direction (comme S-143 du 16).

### 13.4 Urgence [R]

`normale` (défaut) · `haute` (déclarée par la nature ou par le demandeur dans les bornes de la nature : panne bloquante, sujet d'examen à imprimer demain) · `securite` (catégorie sécurité de `maintenance.intervention` : alerte immédiate à la direction et à l'astreinte technique, par le socle 8, et proposition de condamner le lieu, 44 §12.1). Le demandeur ne choisit pas « sécurité » à tort sans conséquence : la file peut requalifier (fait), sans reproche affiché.

---

## 14. Confidentialité par nature

### 14.1 Les classes [R]

| Classe | Exemples de natures | Qui lit le contenu | Ce que voient les autres |
|---|---|---|---|
| **C0** | maintenance, entretien, fournitures | membres de la file ; demandeur ; appuis (sans le nom des autres appuis) | lieu et catégorie pour le « déjà signalé » |
| **C1** | attestation, assistance, reprographie, secrétariat, comptabilité, HSE | membres de la file ; demandeur ; consultés (partie partagée) ; direction (`demandes.lire_toutes`) | — |
| **C2** | question à la direction marquée confidentielle par le demandeur, échéancier | membres de la file **désignés** pour le confidentiel ; demandeur | superviseurs : nombre et retard |
| **C3** | demande RH d'un agent, aide sociale | file **confidentielle** seulement ; demandeur | **personne** d'autre, pas même le nombre en dehors de la direction (S-143) |

### 14.2 Règles [R]

1. **Titre neutre** pour tout ce qui sort de la demande (à-faire, avis, recherche globale, tableau de bord) : composé depuis la nature et le numéro (« Demande RH n° 2026-00045 »), **jamais** depuis l'objet saisi pour une nature C2+ (R-DM16).
2. **Recherche globale** (29) : les demandes C0–C1 de ses files ; aucune demande C2+ hors de la file confidentielle.
3. **Journal d'accès** : toute lecture d'une demande C2+ est tracée (`tracer_acces_sensible`) ; le demandeur peut savoir **quel service** a lu sa demande (pas quel agent, sauf demande motivée au DPO).
4. **Aucun contenu dans les journaux techniques** (E14) : identifiants seulement.
5. **Le demandeur élève mineur** : ses demandes sont lisibles de ses responsables **selon la nature** (`visibilite_responsables` : oui pour une carte d'élève payante, non pour une demande d'accès au compte) ; il le voit écrit **avant** de déposer.
6. **Parents séparés** : la demande d'un responsable n'est **jamais** visible de l'autre (aucun indice : ni nombre, ni date), sauf co-demandeur du même foyer ajouté par le demandeur ; une interdiction (socle 4) empêche d'ajouter l'autre parent comme co-demandeur et d'envoyer à l'autre parent la pièce produite (R-DM17).

---

## 15. Historique et preuve

### 15.1 Les faits [R]

Chaque changement est un **fait** ajouté (`ent_dem_fait`, ajout seul, déclencheur qui refuse `UPDATE` et `DELETE`) : type, auteur (personne et compte, ou tâche système déclarée, jamais l'IA comme auteur d'un geste), date, état avant et après, file avant et après, motif codé, référence (message, pièce, document produit, décision). Chaînage par **empreinte SHA-256** du fait précédent : une altération se voit (`verifier_chaine`). Les valeurs saisies au dépôt sont **figées** ; une correction par le demandeur (avant prise) crée une nouvelle version des valeurs et un fait.

### 15.2 La preuve [R]

- **Dossier de preuve** d'une demande (DEM-252) : PDF émis par le socle 7 — accusé, faits, échanges au demandeur, décision, pièces produites (références et empreintes), dates de notification effective (8) — pour une contestation, l'inspection ou un recours.
- **Notification effective** : la date à laquelle le demandeur a été informé de la décision est celle du socle 8 (ouverture ou remise, selon la nature), pas la date de clôture.
- **Horodatage** : heure murale de l'établissement (fuseau du socle 5), stockée en UTC.

### 15.3 Conservation [R, durées soumises au DPO, Q-DM4]

| Catégorie | Natures | Durée proposée après clôture | Ensuite |
|---|---|---|---|
| `DemandeTechniqueCourte` | maintenance, entretien, assistance, fournitures | 1 an | anonymisée (demandeur retiré), agrégats gardés pour le 47 |
| `DemandeInterneCourante` | secrétariat, vie scolaire, direction (C1) | 2 ans | effacée |
| `DemandeFamilleCourante` | attestation, carte, secrétariat famille | année en cours + 1 | effacée (la pièce produite suit la conservation du coffre, 7) |
| `DemandeComptable` | comptabilité, remboursement | 10 ans [NV, pièces comptables] | archive |
| `DemandeRh` | `rh.demande_agent`, `rh.hse` | durée de la relation de travail + 5 ans [NV] | archive / effacement |
| `Reprographie` | `reprographie.tirage` | fichiers à imprimer : **effacés 30 jours après « retirée »** ; la demande : 1 an ; compteurs : 5 ans | — |

La purge passe par l'**ordonnanceur** du socle 31 (jamais par une lecture, NJ-89), produit un rapport (compte par nature, aucun nom) et respecte le **gel** (contentieux, demande de l'inspection : socle 27).

---

## 16. Notifications et à-faire

### 16.1 Publier n'est pas notifier [R]

Un fait de N1 **n'envoie rien** par lui-même : il publie un événement ; les natures d'avis déclarées (§26.1) décident **qui** est prévenu, le socle 8 décide **comment et quand** (canal, regroupement, plage calme, préférences). Les personnels reçoivent un **résumé** (une fois par demi-journée par défaut) plutôt qu'un avis par demande ; seules les natures `haute` et `securite` partent tout de suite.

### 16.2 Ce que reçoit chacun [R]

| Qui | Avis | Canal par défaut |
|---|---|---|
| Demandeur | accusé de réception ; prise en charge (si la nature le déclare) ; complément demandé ; décision ; clôture ; relance avant clôture sans réponse | dans l'ENT + push ; e-mail **enveloppe neutre** (« Votre demande n° 2026-00123 a avancé — voir dans ScolIA ») |
| Membres de la file | nouvelle demande (résumé) ; demande en retard ; demande rendue à la file (absence, départ) | résumé ; push pour `haute` |
| Traitant | réponse du demandeur ; réponse d'un consulté ; échéance demain | résumé ; push si la personne l'a choisi |
| Responsable de file, direction | escalade ; file de tri en souffrance ; file sans membre actif | résumé ; immédiat pour `securite` |

### 16.3 À-faire (29) [R]

N1 déclare ses natures de tâches (§26.2) : « demande à traiter » (file, collective), « demande en cours » (nominative), « compléter ma demande » (demandeur), « répondre à une consultation », « demande en retard » (escalade), « décider une HSE », « attester des HSE », « imprimer » (reprographie). N1 déclare aussi ses **tuiles de file** (A4 du 29) : compte, plus ancienne échéance, lien vers la file. **Prendre** depuis l'à-faire appelle la commande de N1 (carte d'action) : la prise reste **un fait de N1**, le 29 la reflète (arbitrage : pour les natures de N1, `ent_afaire_prise` du 29 n'est pas utilisée, la source de vérité est `ent_dem_demande.attribuee_a`, R-DM9).

---
## 17. Heures supplémentaires effectives (HSE) — nature `rh.hse`

### 17.1 Ce que recouvre la demande [R]

Une **HSE** est une heure faite en plus du service, ponctuellement, pour un besoin identifié (soutien avant un examen, atelier, accompagnement d'une sortie hors temps de service, stage de réussite, préparation d'un concours, tutorat) [H, liste des motifs à confirmer, Q-DM5]. La demande N1 est une **autorisation préalable** : « je propose de faire 6 h de soutien en 3ᵉB entre le 12 janvier et le 6 février ; me les accordez-vous ? ». Elle n'est **pas** le paiement : le paiement repose sur le **service fait** (15 §14), après réalisation.

Ce qui **n'est pas** une demande `rh.hse` :
- le **remplacement de courte durée** d'un collègue absent : il naît d'une sollicitation du socle 15 (§11–12 du 15), et les heures sont constatées par le 15 ;
- les **HSA** (heures supplémentaires année, au service) : socle 6 (services) et 10 ;
- les heures d'une **mission du Pacte** : socle 15 (`mission_pacte`) ;
- les heures supplémentaires d'un **salarié** de l'organisme de gestion qui ne sont pas des heures d'enseignement : nature `rh.demande_agent` ou planning (socle 6, P15) puis paie (49).

### 17.2 Régimes [R]

| Bénéficiaire | Qui paie | Où vont les heures attestées |
|---|---|---|
| Enseignant du public (titulaire, contractuel de l'État) | État (rectorat) | heures constatées du **15** (`hse`), état mensuel signé, export rectorat |
| Maître contractuel ou agréé du privé sous contrat | État (rectorat ou DEEP [NV]) | idem |
| Salarié de l'organisme de gestion (professeur hors contrat, personnel OGEC) | organisme de gestion | **49** (RH4, éléments variables), quand le 49 existe ; sinon export CSV du 15 vers la comptabilité |
| Contractuel recruté par l'EPLE sur ses fonds propres [H] | EPLE | Q-DM7 |

Le régime est **lu** de l'engagement de l'agent (socle 6 : employeur, nature d'engagement), jamais saisi dans la demande.

### 17.3 Champs [R]

Bénéficiaire (soi ; ou un collègue enseignant si déposé pour le compte, DRM-08) ; **motif** (liste codée, Q-DM5) ; **objet** (texte court) ; **public** : classes, groupes ou élèves désignés (dans les droits du demandeur, socle 5), facultatif pour un motif qui n'a pas d'élèves (préparation d'un concours d'établissement) ; **volume demandé** en quarts d'heure (entier, ≥ 1, ≤ 400 h : garde-fou de saisie, pas un plafond réglementaire) ; **période** (du … au) ; **séances prévues** facultatives (dates et créneaux ; peuvent créer des séances « ajoutées » au socle 10 après accord, DEM-273) ; **lieu** facultatif ; précisions (texte, 2 000 caractères).

### 17.4 Circuit [R]

1. **Dépôt** par l'enseignant (ou pour son compte : secrétariat, comptabilité, direction ; le déposant est tracé et voit la demande, NJ-91).
2. **Décision** par la **direction du site** de l'agent (chef d'établissement, adjoint, ou délégataire du socle 6 avec l'action `demandes.hse.decider` sur le site) : accordée (volume accordé ≤ demandé), accordée en partie (volume réduit, motif au demandeur), refusée (motif codé + texte), complément demandé. **Jamais par soi-même** : une personne qui a le droit de décider et qui est le bénéficiaire ne peut pas décider sa propre demande (R-DM40) ; la décision va à un autre décideur ou au chef.
3. **Enveloppe** : à la décision, l'écran montre l'enveloppe du site pour l'année (dotation, déjà attesté, déjà engagé, restant) ; dépassement = **avertissement**, pas blocage (R-04 du 15 : aucun plafond codé) ; une décision au-delà de l'enveloppe exige une confirmation conf. 2 et un motif.
4. **Accord** : document émis par le socle 7 (gabarit `rh.hse.accord`), **signé** par le décideur si `peut_signer(agent, "personnels", date)` (socle 6), déposé au coffre de l'agent ; **aucun e-mail vers une adresse codée** (NJ-91) ; aucune image de signature.
5. **Réalisation** : l'enseignant déclare les **séances réalisées** (date, horaire, durée en quarts d'heure, public, lieu), au fil de l'eau ou en fin de période ; si les séances ont été créées au 10, il confirme d'un geste « faite » (l'appel du socle 12, s'il a été fait, vaut indice de réalisation et pré-coche).
6. **Attestation du service fait** : par l'adjoint, le CPE ou le responsable désigné (`demandes.hse.attester` sur le site), séance par séance ou par lot ; une séance attestée devient **une heure constatée du socle 15** (§17.6) ; une séance contestée revient à l'enseignant avec motif.
7. **Clôture** : à la fin de la période, ou quand le volume accordé est atteint ; heures accordées non faites → issue `accordee` avec « 2 h non réalisées » (rendues à l'enveloppe).
8. **Modification après accord** : l'enseignant demande un **avenant** (volume, période) : nouvelle décision, même demande, fait daté ; l'accord précédent reste au coffre avec la mention « remplacé par l'avenant du … ».
9. **Annulation** : par le demandeur tant que rien n'est attesté ; par le décideur (motif) ; les heures déjà attestées restent dues.

### 17.5 Enveloppe [R]

Table `ent_dem_hse_enveloppe` : par année scolaire et par site (ou établissement), volume en quarts d'heure (ou montant, Q-DM6), source (`dotation`, `abondement`, `retrait`), date, note ; mouvements ajout seul. Calculs : **engagé** (accordé non encore attesté), **attesté**, **restant** ; vue par motif, discipline, niveau (agrégats ; effectifs < 5 masqués pour une vue partagée). Aucune enveloppe saisie = aucune alerte (l'établissement n'est pas obligé de la tenir).

### 17.6 Lien avec le socle 15 (heures constatées) [R, port à ajouter au 15]

Port proposé au 15 : `heures::constater(tx, acteur, HeureConstatee { agent_id, nature: Hse, date, duree_quarts, origine: Origine::DemandeN1 { demande_id, seance_id }, public })` → `heure_id`, **idempotent** sur `(origine)`. Le 15 garde son cycle (`constatee` → `validee` → état mensuel `fige`, `signe`, `exporte`) : une heure attestée par N1 entre **validée** (l'attestation vaut validation, R-DM41) sauf si l'établissement veut une seconde validation (SN1-10). Une correction après export est une **régularisation** du 15 (15 §14.2), jamais une réécriture : N1 appelle `heures::regulariser`. Tant que le lot du 15 qui porte `ent_absp_heure` (migration 2308, lot A7) n'est pas versé, N1 garde les séances attestées dans ses tables et un **export CSV** mensuel de secours (DEM-282) ; le port les reprend ensuite (idempotence par origine), sans double compte (R-DM42).

### 17.7 Lien futur avec la paie (49, RH4) [R]

N1 publie `demandes.hse_attestee` (agent, date, durée, régime) ; le 49 (lot RH4, variables) consomme les heures des **salariés** de l'organisme de gestion, applique les majorations de la convention (calcul hors N1, D-048 « light »), et rend l'élément variable. N1 ne calcule **aucun** montant. Contradiction à lever : `49_PAIE.md` RHP-252 prévoit d'« importer les HSE de l'année (Next.js M08) » ; **recommandation** : la reprise des HSE est faite **une seule fois** par N1-5 (§29) et le 49 lit les heures du 15 ; RHP-252 devient sans objet (signalé, §35).

### 17.8 Ce que voit chacun [R]

- **Enseignant** : ses demandes, l'accord (coffre), ses séances réalisées et attestées, « mes heures » du 15 (EDT-119).
- **Décideur** : les demandes de son site, l'enveloppe, l'historique des HSE d'un agent **pour l'année** (pour arbitrer équitablement).
- **Attestant** : les séances déclarées à attester de son site.
- **Comptabilité, gestion** : l'état mensuel (15), pas les demandes une par une.
- **Collègues** : rien.

---

## 18. Reprographie — nature `reprographie.tirage` (conforme à `44_RESSOURCES_MATERIELLES.md` §15)

### 18.1 Pourquoi dans N1 avant le 44 [R, appliqué par défaut, D-031]

Le module Next.js est en usage et le socle 44 (étage J) ne sera pas codé avant février. La nature est donc **intégrée à N1** (lot N1-3) avec le **modèle du 44** (§15 de sa conception, NRM-01 à NRM-10, DRM-01 à DRM-13) ; quand le 44 arrivera (RM5), il **reprendra la déclaration** (même code de nature, préfixe `reprographie` attribué au 44) et ajoutera ses volets (copieurs, compteurs relevés, coût, déclaration CFC) **sans reprise de données** : les tables `ent_dem_repro*` restent la source des demandes (R-DM45). Florian peut préférer la perte (photocopies « perdables », T0) : Q-DM8.

### 18.2 Demande [R]

Qui : professeurs, administratifs, vie scolaire, accueil ; **pour le compte** d'un enseignant (personne qui a un service au 6) par l'administratif, l'accueil, la comptabilité ou la direction (DRM-08). Champs : **site** (du 5 ; défaut : site de la séance suivante de l'enseignant) ; 1 à 5 **PDF** (dépôt du 7, liés au déposant, NJ-78 ; nombre de pages lu du PDF) ; **exemplaires** (pré-remplis depuis l'effectif des classes ou groupes choisis, modifiables ; borne 5 000) ; recto-verso, agrafage, format (A4, A3), **couleur ou noir et blanc** ; **pour quand** (date et moment : « lundi 8 h ») ; lieu de retrait (liste du site) ; case **« contient des extraits d'œuvres protégées »** (titre, auteur, éditeur, pages copiées) ; motif facultatif.

### 18.3 Circuit [R]

- **Noir et blanc** sous le seuil du site (SN1-11 = S-4410 du 44, défaut 500 copies) : accepté d'emblée → file d'impression.
- **Couleur**, ou noir et blanc au-delà du seuil : **validation par la direction du site** (ou délégataire désigné, `demandes.repro.valider`) ; une seule décision ; note facultative. Le choix « noir et blanc » ne contourne pas la validation au-delà du seuil (DRM-07).
- **Œuvres protégées** au-delà des plafonds CFC (10 % d'un livre, 30 % d'une revue, d'après les pages déclarées) : avertissement à l'enseignant, sans blocage.
- **Délai réaliste** : si « pour quand » est plus court que le délai du site (SN1-12, défaut 1 jour ouvré), avertissement « la reprographie demande 24 h ; la demande sera traitée au mieux » ; jamais un refus automatique.
- **Annulation** par le demandeur tant que ce n'est pas `en_impression`.

### 18.4 File d'impression [R]

- **Par site** : le réceptionnaire (mission « reprographie » du 6 ou agent désigné pour un ou plusieurs sites, R-RM27 du 44) ne voit **que** les demandes acceptées de **son** site (NJ-77, DRM-01) ; jamais un rôle entier par défaut.
- Tri par « pour quand », puis dépôt ; regroupement « à imprimer ce matin ».
- Ouvrir le PDF : jeton court du 7, journalisé ; imprimer ; marquer **prête** d'un geste **authentifié** (NJ-92, DRM-05) avec lieu de retrait ; marquer **retirée** (facultatif) ; signaler un problème (PDF illisible, quantité douteuse) = demande de complément.
- Le demandeur voit « prête — retrait salle des professeurs » et le signal « prête, pas encore vue » disparaît quand il l'a vue.
- **Compteurs simples** (N1-3) : copies par site, demandeur, discipline, couleur/noir, par mois ; export CSV pour une refacturation interne éventuelle (48). Le 44 ajoutera relevés des copieurs, coûts et déclaration CFC.

### 18.5 Données [R]

Les PDF à imprimer peuvent contenir des données d'élèves (listes, copies annotées) : ils ne sont **jamais** joints à un e-mail (DRM-12) et sont **effacés 30 jours après « retirée »** (ou 60 jours après « prête » sans retrait), par l'ordonnanceur (§15.3).

---

## 19. Familles et élèves

### 19.1 Comptes, pas de formulaire public [R]

Parents et élèves ont de **vrais comptes** (D-009) ; toute demande d'une famille se fait **depuis le compte** (D-051). La page publique Next.js « demande des parents » n'est **pas reprise** (NJ-88). Une famille sans compte (pas encore activé, en difficulté avec le numérique) dépose **par l'accueil ou le secrétariat**, pour son compte (§9.3), ou au téléphone ; l'accueil peut aussi lui envoyer l'invitation d'activation (socle 1). Exception : la nature `numerique.acces_compte` est déposable par l'accueil pour une personne qui ne peut pas se connecter.

### 19.2 Qui peut demander quoi [R]

- **Responsable** : les natures qui ouvrent le public `Responsable(droit_du_lien)` **pour un enfant sur lequel il a ce droit** (socle 4 : `recevoir_documents`, `payeur`, autorité…). Un parent sans exercice de l'autorité parentale peut demander ce que son lien lui ouvre (attestation, duplicata s'il reçoit les bulletins), pas une décision d'autorité.
- **Élève mineur** : quelques natures (`scolarite.carte_eleve`, `numerique.acces_compte`, demande au CDI quand le 45 existera) ; jamais une nature qui engage l'autorité parentale ou de l'argent sans ses responsables.
- **Élève majeur** : toutes les natures ouvertes aux responsables le concernant ; ses responsables ne voient pas ses demandes, sauf ce que le socle 4 prévoit (payeur pour une facture) (R-DM34).
- **Ancien élève** : duplicata d'un document (Q-DM3) : compte conservé en lecture après le départ ? Sinon dépôt par le secrétariat.

### 19.3 Parents séparés, interdictions [R]

La demande appartient à son demandeur ; l'autre responsable n'en voit **rien** (ni existence, ni nombre, ni date) ; la pièce produite va au coffre du demandeur seulement, sauf nature `ParEleve` qui la dépose aussi au dossier de l'élève (où chaque responsable la voit selon son droit, socle 7). Une interdiction ou une adresse protégée (socle 4) empêche toute mention de l'autre dans les avis et les pièces produites (adresse de l'autre parent jamais imprimée sur une attestation demandée par un parent, R-DM17).

### 19.4 Messagerie (16) : pas de doublon [R]

| Situation | Où |
|---|---|
| Question libre (« à quelle heure finit la réunion ? ») | **16** (boîte de service) |
| Besoin catalogué (« une attestation de scolarité pour la CAF ») | **N1** |
| Conversation qui devient un engagement | la boîte de service **convertit** le fil en demande (DEM-318) : nature choisie, valeurs pré-remplies par le membre, demandeur = l'auteur du fil ; le fil reçoit un dernier message « Votre message est devenu la demande n° … » et passe **clos** ; la demande garde la référence du fil (`ent_dem_lien`, type `issue_de_conversation`) |
| Demande qui appelle une discussion longue | les échanges se font **dans la demande** (§12) ; si la discussion dépasse la demande, la file peut **ouvrir une conversation** (16) et clore la demande en le disant (issue `transmise_conversation`) |
| Le parent écrit au PP pour une attestation | le PP **transmet** à la bonne file en convertissant (la famille est prévenue) ; le PP ne garde pas la demande |

Côté famille, **« Mes démarches »** (29 A2) montre les demandes N1 avec leurs états ; **« Messages »** montre les conversations ; la pastille d'à-faire compte les compléments à fournir (R-PO4 du 29), pas les demandes en cours.

### 19.5 Accessibilité, langues [R]

Parcours en trois écrans au plus sur téléphone ; libellés courts ; lecture vocale native ; traduction des **libellés de natures** dans les langues du socle 8 (ScolIA, pas l'établissement) ; les réponses du personnel traduites sur demande (relues).

---

## 20. Intelligence artificielle (contrat IA du socle, `IA_COEUR.md`)

### 20.1 Ce que l'IA fait [R]

- **Proposer une nature** au demandeur à partir de la phrase qu'il tape, **avant** le dépôt : d'abord par mots-clés (local, déterministe) ; par modèle de langage seulement si l'établissement l'active (Mistral, D-055) et **après pseudonymisation** : noms d'élèves et de personnes reconnus (liste de l'établissement **dans les droits du demandeur**) remplacés par des marqueurs avant envoi, chiffres longs masqués (NJ-87).
- **Proposer une file** sur une demande de la file de tri (aide au tri, conf. 1 pour l'appliquer), à partir de la nature et de l'objet pseudonymisé ; jamais le texte libre d'une nature C2+.
- **Résumer** un fil long pour le traitant (natures C0–C1), **rédiger un brouillon de réponse** à relire (jamais envoyé seul), **traduire** une réponse relue.
- **Détecter un doublon** probable (même lieu, même catégorie) pour proposer l'appui.
- **Repérer un signal grave** (mots d'alerte, déterministe d'abord) dans une demande ordinaire et proposer au traitant le **renvoi protégé** vers le 13 ou le 36 (DEM-246).

### 20.2 Ce que l'IA ne fait pas

Déposer, router, prendre, transférer, clore, décider (HSE, couleur, remboursement), envoyer un message, lire une nature C2+ ou ses pièces, recevoir un INE ou un nom d'élève, apprendre des demandes (aucun entraînement), s'exécuter avec plus de droits que l'utilisateur.

### 20.3 Capacités déclarées (registre `ent-ia`)

| Capacité | Type | Droit requis | Sensibilité max | Confirmation |
|---|---|---|---|---|
| `demandes.suggerer_nature` | lecture | être connecté | C1 (texte pseudonymisé) | aucune (proposition) |
| `demandes.suggerer_file` | lecture | membre de la file de tri | C1 | conf. 1 pour appliquer |
| `demandes.resumer` | lecture | `demandes.traiter` sur la file | C1 | — |
| `demandes.brouillon_reponse` | préparation | idem | C1 | relu, conf. 1 à l'envoi (geste humain) |
| `demandes.lister_mes_demandes` | lecture | soi | C1 | — |
| `demandes.deposer` | commande | soi | C1 | **conf. 2** (aperçu complet), geste humain |
| Fermées à l'IA | — | — | — | prise, transfert, décision, clôture, paramétrage, toutes les natures C2+ |

---

## 21. Cas limites (catalogue)

### 21.1 Dépôt et routage

| N° | Situation | Traitement |
|---|---|---|
| CDM-01 | Le demandeur choisit une mauvaise nature (assistance pour une fuite d'eau) | la file transfère ou requalifie ; le délai suit la nouvelle nature ; avis au demandeur seulement si le délai change |
| CDM-02 | Aucune règle de routage ne s'applique | destination par défaut de la nature ; sinon file de tri du site ; jamais nulle part |
| CDM-03 | La file de destination n'a aucun membre actif (départ, absence du seul membre) | file de repli, signal au responsable et à la direction ; le demandeur voit le délai recalculé |
| CDM-04 | Un professeur enseigne sur deux sites ; la panne est sur le site B | critère « site du lieu » prioritaire sur « site du demandeur » |
| CDM-05 | Établissement à plusieurs cycles : une famille a un enfant au collège et un au lycée | le routage suit **l'élève sujet** ; deux enfants = deux demandes (ou nature `ParFoyer` si elle le déclare) |
| CDM-06 | Dépôt pendant les vacances | accusé immédiat, mention de la fermeture, délai qui part à la réouverture |
| CDM-07 | Dépôt d'une nature désactivée depuis un ancien lien | « cette démarche n'est plus proposée » + raccourci vers le secrétariat ; aucune demande créée |
| CDM-08 | La nature change de version pendant qu'une demande est ouverte | la demande garde sa version ; l'écran la rend avec sa version |
| CDM-09 | Le demandeur dépose dix fois la même demande en une minute | limitation de débit par compte et par nature (NJ-72) ; doublons signalés |
| CDM-10 | Une demande de maintenance est en fait un danger (odeur de gaz) | catégorie sécurité ou mot d'alerte : alerte immédiate à la direction et à l'astreinte, consigne affichée au demandeur (« quittez les lieux, appelez le 18 »), proposition de condamner le lieu (44) |
| CDM-11 | Un agent dépose une demande RH en se trompant de nature (« question à la direction ») | le secrétariat qui qualifie voit une demande C1 ; il la **requalifie** en `rh.demande_agent` (C3) : la demande quitte la file et l'historique de lecture garde qui l'a lue avant |

### 21.2 Traitement

| N° | Situation | Traitement |
|---|---|---|
| CDM-12 | Deux agents prennent la même demande à la même seconde | contrainte d'unicité : un seul gagne, l'autre voit qui l'a prise |
| CDM-13 | Le traitant tombe malade avec 6 demandes prises | §11.4 : signal à la file, demandes dues pendant l'absence proposées à la reprise ; aucune réattribution silencieuse |
| CDM-14 | Un agent quitte l'établissement en cours d'année | §11.5 : ses demandes prises reviennent à leur file à la date de fin |
| CDM-15 | Une demande traîne depuis 3 semaines | escalade palier 1 puis 2 ; tableau des retards ; jamais de silence |
| CDM-16 | Le responsable de la file est lui-même absent | l'escalade va au palier suivant (direction) |
| CDM-17 | Une demande est transférée d'une file à l'autre en boucle | au 3ᵉ transfert, la demande va au responsable de la file de tri avec le signal « transferts répétés » |
| CDM-18 | La demande a besoin de deux services (attestation qui demande une vérification de la comptabilité) | consultation (§11.6), pas transfert |
| CDM-19 | Le demandeur répond après la clôture « sans réponse » | dans le délai de réouverture (SN1-05) : réouverture ; sinon nouvelle demande pré-remplie avec le lien vers l'ancienne |
| CDM-20 | Le traitant veut répondre « oui » mais la décision appartient au chef | étape de validation de la nature, ou escalade explicite ; jamais une décision au nom d'un autre |
| CDM-21 | Un message au demandeur a été envoyé par erreur (mauvaise demande) | il ne peut pas être effacé ; message de correction ; le socle 8 émet la correction si la nature le prévoit ; fait au journal |
| CDM-22 | Le demandeur est décédé | `sans_objet` silencieux, aucune relance ; la file garde la demande (29 A14) |
| CDM-23 | L'élève sujet quitte l'établissement | la demande reste traitable (une attestation de scolarité passée reste due) ; la nature déclare si elle devient sans objet |

### 21.3 Confidentialité et sécurité

| N° | Situation | Traitement |
|---|---|---|
| CDM-24 | Un agent de maintenance tente d'ouvrir une demande RH par son numéro | 404 neutre ; tentative tracée (attaque) |
| CDM-25 | Un parent devine le numéro de la demande de l'autre parent | numéro non énumérable côté famille : la famille ne lit que ses demandes ; 404 |
| CDM-26 | Une demande de famille contient une information de santé (« mon fils a un PAI ») | avertissement au dépôt ; le traitant peut **masquer** le passage (fait, original chiffré gardé pour la durée de conservation) et proposer le canal infirmerie (34) |
| CDM-27 | Une demande ordinaire révèle du harcèlement | renvoi protégé (DEM-246) : le traitant propose au demandeur le canal du 36/13, la demande N1 est close avec issue `transmise_canal_protege` et un texte neutre ; aucun contenu n'est recopié sans le geste de l'agent habilité du 36 |
| CDM-28 | Un personnel dépose une demande au nom d'un collègue sans son accord (HSE) | le bénéficiaire est prévenu et peut **retirer** la demande ; la décision n'est possible qu'après sa confirmation (R-DM43) |
| CDM-29 | La direction veut lire toutes les demandes RH « pour vérifier » | refusé sans le droit de la nature ; la direction voit le nombre et les retards ; un accès exceptionnel passe par une supervision motivée, datée et tracée (comme la supervision du 16) |
| CDM-30 | Un PDF à imprimer contient des copies d'élèves | jamais en pièce jointe ; effacé 30 jours après retrait ; lien court journalisé |
| CDM-31 | Un compte compromis dépose des centaines de demandes | limitation de débit, alerte technique (28), gel du compte par l'administrateur |
| CDM-32 | Une pièce est infectée | refus au dépôt (antivirus du 7), message neutre au déposant |

### 21.4 Familles et élèves

| N° | Situation | Traitement |
|---|---|---|
| CDM-33 | Parent sans autorité parentale demande un duplicata de bulletin | possible s'il a le droit `recevoir_bulletins` (4) ; sinon nature absente de son catalogue |
| CDM-34 | Interdiction de contact : un parent demande une attestation qui porterait l'adresse de l'autre | gabarit sans l'adresse de l'autre ; contrôle à l'émission (7) |
| CDM-35 | Élève de 17 ans qui devient majeur pendant sa demande | la demande reste à son demandeur ; à la majorité, les demandes **de ses parents le concernant** restent aux parents mais leurs futures pièces suivent la règle du 4 |
| CDM-36 | Élève mineur dépose une demande de carte d'élève payante | ses responsables payeurs sont informés (nature `visibilite_responsables`) ; la facture va au payeur (24) |
| CDM-37 | Famille allophone | libellés traduits ; réponse traduite relue |
| CDM-38 | Famille qui a deux comptes (un par parent) et deux foyers | chaque parent dépose de son côté ; aucun recoupement entre eux |
| CDM-39 | Famille sans numérique au guichet | dépôt pour le compte par l'accueil ; accusé **imprimé** remis au comptoir (DEM-108) |
| CDM-40 | Un parent écrit au secrétariat par la messagerie pour une attestation | conversion du fil en demande (§19.4) |

### 21.5 HSE

| N° | Situation | Traitement |
|---|---|---|
| CDM-41 | Le chef d'établissement est aussi enseignant et demande des HSE | il ne décide pas sa propre demande : l'adjoint ou le délégataire décide ; à défaut, alerte (R-DM40) |
| CDM-42 | HSE accordées, séances faites, mais l'enseignant oublie de les déclarer | rappel à la fin de la période (dans son résumé) ; l'attestant voit « 6 h accordées, 0 déclarée » |
| CDM-43 | Une séance déclarée a eu lieu pendant une absence de l'enseignant (15) | incohérence signalée à l'attestant ; pas d'attestation automatique |
| CDM-44 | Une séance HSE chevauche un cours de l'enseignant (10) | avertissement à la déclaration (« vous aviez cours en 4A à cette heure ») |
| CDM-45 | Enveloppe dépassée en avril | avertissement, décision possible avec motif (conf. 2) ; tableau de l'enveloppe |
| CDM-46 | Changement d'année scolaire au milieu d'une période HSE | la demande reste sur son année d'origine ; les séances après le 31 août sont refusées (nouvelle demande) |
| CDM-47 | L'enseignant part en cours de période | séances attestées dues ; reste rendu à l'enveloppe à la date de fin |
| CDM-48 | Une heure attestée est contestée après l'export de l'état mensuel | régularisation du 15 sur l'état suivant (§17.6) |
| CDM-49 | HSE pour remplacer un collègue en congé de convenance personnelle | alerte R-06 (15), sans blocage |
| CDM-50 | Reprise : une HSE acceptée dans Next.js a déjà été envoyée par e-mail au rectorat | reprise **sans** création d'heures au 15 (risque de double paiement) ; marquée « transmise hors ScolIA » (§29) |

### 21.6 Reprographie

| N° | Situation | Traitement |
|---|---|---|
| CDM-51 | Le réceptionnaire couvre deux sites | il voit les deux files, séparées |
| CDM-52 | Demande pour demain 8 h déposée à 18 h | avertissement de délai ; tri en tête de file ; aucune promesse |
| CDM-53 | PDF de 300 pages, 30 exemplaires, en couleur | validation de la direction ; avertissement de volume |
| CDM-54 | Le copieur du site est en panne | le réceptionnaire peut **transférer** la demande à la file d'un autre site (le demandeur choisit le lieu de retrait) ou la passer `en_attente_tiers` (motif public « copieur en panne ») |
| CDM-55 | Copies jamais retirées | rappel au demandeur à J+3 ; effacement du PDF à J+60 |
| CDM-56 | Extraits d'un manuel au-delà de 10 % | avertissement CFC, sans blocage |

---
## 22. Actions métier (DEM-100 et suivantes)

Abréviations : PER personnel ; ENS enseignant ; SEC secrétariat ; ACC accueil ; CPT comptabilité ; DIR direction (chef, adjoint) ; RF responsable de file ; MF membre de file ; RESP responsable légal ; ELV élève mineur ; MAJ élève majeur ; ADM administrateur ; SYS système (tâche déclarée). DEM-01 à DEM-04 (matrice Charlemagne) sont couverts par DEM-100 à DEM-180, DEM-310 à DEM-318 et DEM-290 à DEM-306.

### 22.1 Déposer

| N° | Action | Qui · droit | Règles | Lot |
|---|---|---|---|---|
| DEM-100 | Ouvrir le catalogue « Faire une demande » (natures ouvertes **à moi**, raccourcis compris) | tous · `demandes.deposer` (PER) ; lien (RESP) ; capacité (ELV, MAJ) | R-DM1, R-DM3 | N1-1 |
| DEM-101 | Chercher une nature par mots (tolérance aux fautes, mots-clés déclarés) | tous | local, sans IA | N1-1 |
| DEM-102 | Voir « mes natures fréquentes » et « fréquentes ici » | tous | calcul local | N1-4 |
| DEM-103 | Suivre un **raccourci** vers le parcours d'une autre brique | tous | aucune demande créée | N1-1 |
| DEM-104 | Remplir la demande : champs typés, objet, texte libre si autorisé, sujet dans mes droits | tous | §9.2 ; valeurs validées par le serveur | N1-0 |
| DEM-105 | Joindre des pièces (socle 7) | tous | types et taille de la nature ; NJ-78 | N1-2 |
| DEM-106 | **Déposer pour le compte** d'une personne (famille au comptoir, au téléphone ; collègue) | ACC, SEC, CPT, DIR · `demandes.deposer_pour_compte` | §9.3 ; canal codé ; déposant tracé ; NJ-84 | N1-1 |
| DEM-107 | Voir l'aperçu du routage et du délai avant de déposer | tous | conf. 1 | N1-1 |
| DEM-108 | Imprimer l'accusé de réception (dépôt au comptoir) | ACC, SEC | gabarit du 7 | N1-4 |
| DEM-109 | Enregistrer un brouillon ; le reprendre ; le supprimer | auteur | 30 jours (SN1-04) ; invisible de la file | N1-0 |
| DEM-110 | Déposer | auteur | numéro, accusé, tâche à la file ; idempotence (clé de dépôt) | N1-0 |
| DEM-111 | Recevoir l'accusé de réception (ENT et canal du 8) | demandeur | §9.4 | N1-1 |
| DEM-112 | Accusé **à mentions** (délai de décision implicite, voies de recours) pour un EPLE et une nature `decision_usager` | SYS | §6.1 | N1-1 |
| DEM-113 | Être averti d'une demande ouverte similaire (doublon) avant de déposer | demandeur | §9.5 | N1-2 |
| DEM-114 | « +1 » : appuyer un signalement existant (lieu, catégorie) | PER | abonnement aux avis de clôture | N1-2 |
| DEM-115 | Ajouter un co-demandeur du même foyer | RESP | interdiction : refusé (R-DM17) | N1-2 |
| DEM-116 | Déposer depuis le téléphone (appli unique, D-003) avec photo | PER | brouillon hors ligne ; jamais rejoué en silence | N1-7 |
| DEM-117 | Déposer depuis un QR code de salle (lieu pré-rempli) | PER | QR du 44 / du 5 | N1-7 |
| DEM-118 | Déclarer l'urgence dans les bornes de la nature | demandeur | requalifiable par la file | N1-1 |
| DEM-119 | Lire l'avertissement de minimisation avant le texte libre | demandeur | R-DM30 | N1-0 |

### 22.2 Suivre sa demande (demandeur, déposant)

| N° | Action | Qui · droit | Règles | Lot |
|---|---|---|---|---|
| DEM-120 | Voir « Mes demandes » (ouvertes, closes), état en mots simples, échéance visée | demandeur, déposant | projection demandeur (NJ-82) | N1-0 |
| DEM-121 | Ouvrir une demande : valeurs, échanges qui me sont adressés, pièces, décision | demandeur | jamais note interne ni nom d'un consulté | N1-0 |
| DEM-122 | Corriger ma demande tant qu'elle n'est pas prise | demandeur | nouvelle version des valeurs, fait | N1-2 |
| DEM-123 | Répondre à une demande de complément (texte, pièce) | demandeur | reprend le décompte du délai | N1-2 |
| DEM-124 | Annuler ma demande (selon la nature) | demandeur | `annulee` ; avis à la file si prise | N1-2 |
| DEM-125 | Relancer une demande en retard | demandeur | une fois par délai dépassé (R-DM24) | N1-2 |
| DEM-126 | Voir qui s'en occupe (service ; prénom de l'agent si la nature l'autorise) | demandeur | SN1-02 | N1-2 |
| DEM-127 | Télécharger la pièce produite (au coffre du 7) | demandeur | jeton court | N1-2 |
| DEM-128 | Contester une décision (refus) | demandeur | message à la file ; pour un EPLE, rappel des voies de recours | N1-2 |
| DEM-129 | Vérifier une intervention (« c'est réparé » / « toujours en panne ») | demandeur, appuis | §13.1 | N1-2 |
| DEM-130 | Rouvrir une demande close dans le délai | demandeur | SN1-05 | N1-2 |
| DEM-131 | Voir l'historique de ma demande (faits qui me concernent) | demandeur | projection | N1-2 |
| DEM-132 | Télécharger le dossier de preuve de ma demande | demandeur | PDF du 7 ; DEM-252 | N1-8 |
| DEM-133 | Choisir mes canaux d'avis pour les demandes | demandeur | préférences du 8 | N1-1 |
| DEM-134 | Voir ma demande depuis l'à-faire (« compléter ») | demandeur | tâche du 29 | N1-2 |
| DEM-135 | Savoir quel **service** a lu ma demande confidentielle | demandeur | journal d'accès (R-DM16) | N1-8 |
| DEM-136 | Voir les demandes que j'ai déposées pour d'autres | déposant | jusqu'à la clôture + 30 jours | N1-2 |
| DEM-137 | Retirer une demande déposée à mon nom sans mon accord | bénéficiaire | R-DM43 | N1-3 |
| DEM-138 | Recevoir l'annonce d'une clôture sans réponse 3 jours avant | demandeur | §12.1 | N1-2 |
| DEM-139 | Rechercher dans mes demandes (numéro, objet, nature) | demandeur | | N1-4 |

### 22.3 Routage et tri

| N° | Action | Qui · droit | Règles | Lot |
|---|---|---|---|---|
| DEM-150 | Router une demande au dépôt (règles, nature, défaut, tri) | SYS | §10 ; raison enregistrée | N1-1 |
| DEM-151 | Voir la raison du routage | MF | jamais le demandeur | N1-1 |
| DEM-152 | Voir la file de tri du site | membres de la file de tri | NJ-85 | N1-1 |
| DEM-153 | **Requalifier** la nature (corrige délai, file, confidentialité) | MF (tri ou file) | fait ; avis si le délai change | N1-1 |
| DEM-154 | Envoyer de la file de tri vers une file | membres de la file de tri | motif facultatif | N1-1 |
| DEM-155 | Router vers les porteurs d'une action sur le sujet (file implicite) | SYS | §10.3 | N1-1 |
| DEM-156 | Recalculer les membres d'une file à la date | SYS | missions du 6 | N1-1 |
| DEM-157 | Router vers la file de repli quand la file est vide | SYS | signal au responsable | N1-1 |
| DEM-158 | Prendre depuis la file de tri pour la traiter soi-même (si membre de la file cible) | MF | pas de devinette de branche (NXN-06) | N1-2 |
| DEM-159 | Fusionner deux demandes (doublon) | MF | l'une devient `doublon` de l'autre ; les deux demandeurs sont abonnés ; aucun ne voit l'autre (familles) | N1-2 |
| DEM-160 | Indiquer « relève de la direction » (indicateur) sans transmettre | MF | NXN-05 | N1-1 |
| DEM-161 | Voir les transferts répétés (≥ 3) | RF de la file de tri | CDM-17 | N1-2 |
| DEM-162 | Router selon le site du lieu, de l'élève ou du demandeur | SYS | CDM-04 | N1-1 |
| DEM-163 | Router une nature C3 seulement vers une file confidentielle | SYS | contrôlé à l'assemblage et au paramétrage | N1-1 |
| DEM-164 | Proposer une file par l'IA (aide au tri) | IA → MF | §20 ; conf. 1 | N1-6 |

### 22.4 Prise en charge et traitement

| N° | Action | Qui · droit | Règles | Lot |
|---|---|---|---|---|
| DEM-170 | Voir sa file : à prendre, prises par moi, prises par d'autres, en attente, en retard | MF · `demandes.traiter` (portée : mes files) | tri par urgence, échéance, dépôt | N1-2 |
| DEM-171 | **Prendre** une demande | MF | unicité (NJ-90) ; NJ-81 | N1-2 |
| DEM-172 | Relâcher une demande | celui qui l'a prise | | N1-2 |
| DEM-173 | Réattribuer à un membre de la file | RF · `demandes.file.repartir` | journalisé | N1-2 |
| DEM-174 | Déléguer ponctuellement à un membre (avec note) | RF | anti-escalade (C3) | N1-2 |
| DEM-175 | Voir « mes demandes en cours » (toutes files) | MF | à-faire nominatif | N1-2 |
| DEM-176 | Transférer à une autre file (motif codé) | MF de la file actuelle | R-DM12 ; avis au demandeur | N1-2 |
| DEM-177 | Renvoyer à la file de tri | RF | NXN-08 | N1-2 |
| DEM-178 | Transmettre à la direction | MF, RF | R-DM14 | N1-2 |
| DEM-179 | Mettre en attente d'un tiers (motif public codé) | traitant | suspend si la nature le déclare | N1-2 |
| DEM-180 | Demander un complément au demandeur (liste de ce qui manque) | traitant | §12.1 ; L114-5 (EPLE) | N1-2 |
| DEM-181 | Superviser une file : compteurs, délais, retards, sans contenu C2+ | DIR, RF · `demandes.file.superviser` | NXN-12 | N1-2 |
| DEM-182 | Marquer traitée (avec réponse au demandeur) | traitant | vérification si la nature la prévoit | N1-2 |
| DEM-183 | Clore avec une issue | traitant | §7 ; avis au demandeur | N1-2 |
| DEM-184 | Décider (accordée, en partie, refusée) pour une nature `AvecDecision` | décideur de la nature | motif codé + texte | N1-2 |
| DEM-185 | Valider une étape (`AvecValidation`) | valideur désigné | | N1-2 |
| DEM-186 | Produire la pièce de réponse (attestation…) par le socle 7 | traitant | gabarit de la nature ; signature si `peut_signer` | N1-2 |
| DEM-187 | Déposer la pièce au dossier de l'élève (nature qui le déclare) | traitant | `deposer_au_dossier` (7) | N1-2 |
| DEM-188 | Utiliser une réponse type de la file | MF | variables typées | N1-2 |
| DEM-189 | Requalifier l'urgence | MF | sans reproche affiché | N1-1 |
| DEM-190 | Traiter plusieurs demandes d'un même geste (ex. 12 attestations du même modèle) | MF | action groupée déclarée par la nature ; aperçu | N1-4 |
| DEM-191 | Voir la fiche du demandeur nécessaire au traitement (nom, élève, classe) | MF | minimisée par nature | N1-2 |
| DEM-192 | Ouvrir une conversation (16) depuis une demande et la clore en le disant | MF | issue `transmise_conversation` | N1-2 |
| DEM-193 | Rouvrir une demande close par erreur | traitant, RF | fait ; avis | N1-2 |
| DEM-194 | Signaler une prise sans geste depuis N jours | SYS → RF | SN1-03 | N1-2 |

### 22.5 Échanges, pièces, consultations

| N° | Action | Qui · droit | Règles | Lot |
|---|---|---|---|---|
| DEM-200 | Écrire au demandeur (aperçu avant envoi) | traitant, MF | §12.1 | N1-2 |
| DEM-201 | Écrire une note interne | MF, consultés | jamais visible du demandeur | N1-2 |
| DEM-202 | Masquer sa note interne dans les 10 minutes | auteur | fait gardé | N1-2 |
| DEM-203 | Corriger un message envoyé (message de correction) | auteur | CDM-21 | N1-2 |
| DEM-204 | Joindre une pièce à un échange | auteur | socle 7 | N1-2 |
| DEM-205 | Lire une pièce (jeton court) | selon §12.2 | NJ-83 | N1-2 |
| DEM-206 | **Consulter** un tiers (question, échéance, partie partagée) | traitant | §11.6 | N1-2 |
| DEM-207 | Répondre à une consultation | consulté | échange interne | N1-2 |
| DEM-208 | Ajouter une copie interne (collègue tenu informé) | traitant | dans ses droits | N1-2 |
| DEM-209 | Traduire une réponse (IA, relue) | traitant | 16 MSG-146 | N1-6 |
| DEM-210 | Masquer un passage sensible écrit par le demandeur | traitant | CDM-26 ; original chiffré | N1-2 |
| DEM-211 | Voir les pièces d'une demande avec leur rôle | MF | | N1-2 |
| DEM-212 | Retirer une pièce déposée par erreur (avant traitement) | déposant | fait ; l'objet du 7 suit sa règle | N1-2 |
| DEM-213 | Recevoir la réponse d'un consulté dans son résumé | traitant | | N1-2 |
| DEM-214 | Gérer les réponses types de sa file | RF | | N1-2 |

### 22.6 Délais, relances, escalade

| N° | Action | Qui · droit | Règles | Lot |
|---|---|---|---|---|
| DEM-220 | Calculer les échéances (jours ouvrés du site, suspensions) | SYS | §13.1 | N1-1 |
| DEM-221 | Suspendre et reprendre le décompte (attente du demandeur, d'un tiers) | SYS | faits | N1-2 |
| DEM-222 | Afficher l'échéance de décision implicite (EPLE) | SYS → MF | R-DM21 | N1-2 |
| DEM-223 | Mettre une demande « due » à l'échéance de prise en charge | SYS | tâche du 29 | N1-2 |
| DEM-224 | Escalader au palier 1, puis 2 | SYS ; geste RF | §13.3 | N1-2 |
| DEM-225 | Escalader manuellement | traitant, RF | motif | N1-2 |
| DEM-226 | Relancer le demandeur pour un complément (une fois) | SYS | §12.1 | N1-2 |
| DEM-227 | Clore « sans réponse du demandeur » | SYS | avis 3 jours avant | N1-2 |
| DEM-228 | Résumé du jour des demandes (personnels) | SYS → 8 | plage calme, horaires | N1-2 |
| DEM-229 | Avis immédiat pour `haute` et `securite` | SYS → 8 | §13.4 | N1-2 |
| DEM-230 | Voir le tableau des retards par file | DIR, RF | sans nom hors direction | N1-2 |
| DEM-231 | Reporter une échéance (motif, une fois) | RF | avis au demandeur si visible | N1-2 |
| DEM-232 | Suspendre les délais d'une file pendant une fermeture exceptionnelle | DIR | calendrier du 5 ; avis aux demandeurs | N1-2 |

### 22.7 Absences, délégations, départs

| N° | Action | Qui · droit | Règles | Lot |
|---|---|---|---|---|
| DEM-235 | Signaler à la file les demandes d'un agent absent | SYS (15) | §11.4 | N1-2 |
| DEM-236 | Reprendre la demande d'un collègue absent | MF | « pour Mme X » | N1-2 |
| DEM-237 | Router vers la file de repli pendant l'absence du seul membre | SYS | §11.4 | N1-2 |
| DEM-238 | Appliquer une délégation datée du 6 (délégataire membre) | SYS | §11.3 | N1-2 |
| DEM-239 | Proposer au suppléant les demandes du titulaire | SYS (6) | | N1-2 |
| DEM-240 | Voir à J-15 de son départ ses demandes prises | agent partant, DIR | §11.5 | N1-2 |
| DEM-241 | Rendre ses demandes à la file au départ | SYS | fait `rendue_au_depart` | N1-2 |
| DEM-242 | Voir au retour ce qui a bougé | agent | résumé | N1-2 |

### 22.8 Confidentialité, preuve, conservation

| N° | Action | Qui · droit | Règles | Lot |
|---|---|---|---|---|
| DEM-245 | Lire une demande C2+ (tracé) | membres désignés | R-DM15 | N1-0 |
| DEM-246 | **Renvoi protégé** vers le canal du 13 ou du 36 | traitant | CDM-27 ; R-DM31 | N1-2 |
| DEM-247 | Supervision motivée exceptionnelle d'une file confidentielle | DIR · `demandes.superviser_confidentiel` | motif, durée, trace, avis au DPO | N1-8 |
| DEM-248 | Vérifier la chaîne des faits d'une demande | ADM, DPO | `verifier_chaine` | N1-0 |
| DEM-249 | Voir le journal d'accès d'une demande C2+ | DIR, DPO | | N1-8 |
| DEM-250 | Geler une demande (contentieux, inspection) | DIR, DPO | socle 27 | N1-8 |
| DEM-251 | Purger par l'ordonnanceur selon la conservation | SYS (31) | NJ-89 | N1-2 |
| DEM-252 | Émettre le dossier de preuve d'une demande | DIR, traitant, demandeur | PDF du 7 | N1-8 |
| DEM-253 | Exporter les demandes d'une personne (droit d'accès, 27) | DPO | minimisé | N1-8 |
| DEM-254 | Anonymiser les demandes techniques à l'échéance | SYS | agrégats gardés | N1-8 |
| DEM-255 | Rapport de purge (compte par nature) | ADM | aucun nom | N1-8 |
| DEM-256 | Effacer les PDF de reprographie à J+30 du retrait | SYS | §18.5 | N1-3 |

### 22.9 HSE

| N° | Action | Qui · droit | Règles | Lot |
|---|---|---|---|---|
| DEM-260 | Demander des HSE (motif, objet, public, volume en quarts d'heure, période, séances prévues) | ENS | §17.3 | N1-3 |
| DEM-261 | Demander pour le compte d'un collègue enseignant | SEC, CPT, DIR | déposant tracé ; bénéficiaire prévenu (R-DM43) | N1-3 |
| DEM-262 | Voir la file des HSE à décider de son site | DIR · `demandes.hse.decider` (site) | | N1-3 |
| DEM-263 | Voir l'enveloppe du site pendant la décision | décideur | §17.5 | N1-3 |
| DEM-264 | Accorder (volume ≤ demandé) | décideur | R-DM40 (pas sa propre demande) | N1-3 |
| DEM-265 | Accorder en partie (motif) | décideur | | N1-3 |
| DEM-266 | Refuser (motif codé + texte) | décideur | | N1-3 |
| DEM-267 | Demander un complément | décideur | | N1-3 |
| DEM-268 | Émettre l'accord signé au coffre de l'agent | SYS (7) | `peut_signer` ; NJ-91 | N1-3 |
| DEM-269 | Déclarer une séance réalisée | ENS bénéficiaire | §17.4 ; contrôles CDM-43, CDM-44 | N1-3 |
| DEM-270 | Confirmer d'un geste une séance prévue « faite » | ENS | | N1-3 |
| DEM-271 | Attester le service fait (séance ou lot) | `demandes.hse.attester` (site) | heure constatée au 15 | N1-3 |
| DEM-272 | Contester une séance déclarée (motif) | attestant | retour à l'enseignant | N1-3 |
| DEM-273 | Créer les séances prévues au socle 10 (séances ajoutées) après accord | ENS, DIR | port du 10 ; facultatif | N1-3 |
| DEM-274 | Demander un avenant (volume, période) | ENS | nouvelle décision | N1-3 |
| DEM-275 | Annuler une demande HSE (avant attestation) | ENS ; décideur (motif) | | N1-3 |
| DEM-276 | Clore la période (non réalisé rendu à l'enveloppe) | SYS ; décideur | | N1-3 |
| DEM-277 | Gérer l'enveloppe (dotation, abondement, retrait) | DIR, gestionnaire · `demandes.hse.enveloppe.gerer` | mouvements ajout seul | N1-3 |
| DEM-278 | Voir l'enveloppe : engagé, attesté, restant, par motif et discipline | DIR | agrégats | N1-3 |
| DEM-279 | Voir « mes HSE » (demandées, accordées, faites, attestées) | ENS | et « mes heures » du 15 | N1-3 |
| DEM-280 | Voir l'historique des HSE d'un agent sur l'année (pour arbitrer) | décideur | | N1-3 |
| DEM-281 | Pousser les heures attestées au 15 (port `heures::constater`, idempotent) | SYS | R-DM42 | N1-3 |
| DEM-282 | Export CSV mensuel de secours des heures attestées (si le 15 n'est pas prêt) | DIR, gestionnaire | tracé (`ent_export_trace`) | N1-3 |
| DEM-283 | Publier `demandes.hse_attestee` pour le 49 | SYS | R-DM44 | N1-3 |
| DEM-284 | Alerte « 6 h accordées, rien déclaré » en fin de période | SYS → ENS, attestant | CDM-42 | N1-3 |

### 22.10 Reprographie

| N° | Action | Qui · droit | Règles | Lot |
|---|---|---|---|---|
| DEM-290 | Demander des copies (site, PDF, exemplaires, options, pour quand, retrait, œuvres protégées) | PER | §18.2 | N1-3 |
| DEM-291 | Pré-remplir les exemplaires depuis l'effectif des classes ou groupes | SYS | socle 5 | N1-3 |
| DEM-292 | Déposer pour le compte d'un enseignant | SEC, ACC, CPT, DIR | DRM-08 | N1-3 |
| DEM-293 | Être averti des plafonds CFC | demandeur | §6.5 | N1-3 |
| DEM-294 | Être averti d'un délai trop court | demandeur | SN1-12 | N1-3 |
| DEM-295 | Valider une demande couleur ou au-delà du seuil | `demandes.repro.valider` (site) | DRM-07 | N1-3 |
| DEM-296 | Voir la file d'impression **de son site** | réceptionnaire · `demandes.repro.imprimer` (site) | NJ-77 | N1-3 |
| DEM-297 | Ouvrir le PDF à imprimer (jeton court, journalisé) | réceptionnaire | | N1-3 |
| DEM-298 | Marquer « en impression » | réceptionnaire | bloque l'annulation | N1-3 |
| DEM-299 | Marquer **prête** (lieu de retrait), par un geste authentifié | réceptionnaire | NJ-92 | N1-3 |
| DEM-300 | Marquer retirée | réceptionnaire, demandeur | | N1-3 |
| DEM-301 | Signaler un problème (PDF illisible, quantité douteuse) | réceptionnaire | complément | N1-3 |
| DEM-302 | Transférer à la file d'un autre site (copieur en panne) | réceptionnaire | CDM-54 | N1-3 |
| DEM-303 | Annuler avant impression | demandeur | | N1-3 |
| DEM-304 | Voir les compteurs (site, demandeur, discipline, couleur, mois) ; export CSV | DIR, gestionnaire | | N1-3 |
| DEM-305 | Régler le seuil noir et blanc, le délai et les lieux de retrait du site | ADM, DIR | SN1-11, SN1-12 | N1-3 |
| DEM-306 | Rappeler au demandeur des copies non retirées à J+3 | SYS | | N1-3 |

### 22.11 Familles et élèves

| N° | Action | Qui · droit | Règles | Lot |
|---|---|---|---|---|
| DEM-310 | Voir le catalogue famille (par enfant, selon mes droits du lien) | RESP | §19.2 | N1-4 |
| DEM-311 | Demander une attestation pour mon enfant | RESP (`recevoir_documents`), MAJ | | N1-1 |
| DEM-312 | Demander un duplicata | RESP (`recevoir_bulletins`), MAJ | raccourci si au coffre | N1-1 |
| DEM-313 | Demander une carte d'élève, un carnet, un casier | ELV, MAJ, RESP | `visibilite_responsables` | N1-1 |
| DEM-314 | Demander au secrétariat, à la comptabilité | RESP, MAJ, payeurs | | N1-1 |
| DEM-315 | Voir mes démarches (29 A2) avec leurs états | RESP, ELV, MAJ | | N1-4 |
| DEM-316 | Recevoir la pièce produite au coffre | demandeur | jamais en pièce jointe | N1-2 |
| DEM-317 | Être orienté vers le canal protégé pour une situation grave | tous | R-DM31 | N1-1 |
| DEM-318 | **Convertir** une conversation (16) en demande | membre de boîte · `messagerie.boite.traiter` + `demandes.deposer_pour_compte` | §19.4 | N1-2 |
| DEM-319 | Déposer pour une famille sans compte (accueil) et lui proposer l'activation | ACC, SEC | §19.1 | N1-1 |
| DEM-320 | Élève majeur : déposer seul, sans visibilité des parents | MAJ | R-DM34 | N1-1 |
| DEM-321 | Voir la règle de visibilité avant de déposer (« tes parents verront cette demande ») | ELV | §14.2.5 | N1-1 |
| DEM-322 | Lire les libellés des natures dans sa langue | RESP | socle 8 N4 | N1-7 |

### 22.12 Paramétrage, pilotage, reprise, système

| N° | Action | Qui · droit | Règles | Lot |
|---|---|---|---|---|
| DEM-330 | Activer ou désactiver une nature ; choisir ses publics | ADM, DIR · `demandes.parametrer` | dans les bornes déclarées | N1-1 |
| DEM-331 | Créer une file (catalogue de types), son site, ses horaires, son délai, sa file de repli | ADM | §10.2 | N1-1 |
| DEM-332 | Définir les membres d'une file par règles (mission, fonction, équipe, agent désigné daté) | ADM | aucune liste d'adresses | N1-1 |
| DEM-333 | Écrire les règles de routage | ADM | §10.3 | N1-1 |
| DEM-334 | **Simuler** le routage d'une demande type | ADM, DIR | | N1-1 |
| DEM-335 | Voir les membres d'une file à une date | ADM, RF | | N1-1 |
| DEM-336 | Définir les paliers d'escalade d'une file | ADM | | N1-2 |
| DEM-337 | Voir les files sans membre actif, sans responsable | ADM | alerte | N1-1 |
| DEM-338 | Préréglages par profil d'établissement (collège, lycée, groupe scolaire privé) | ScolIA | §33 | N1-1 |
| DEM-339 | Indicateurs agrégés : volumes, délais médians, retards, par file et nature (pour le 47) | DIR · `demandes.indicateurs.lire` | effectifs < 5 masqués | N1-8 |
| DEM-340 | Exporter la liste des demandes d'une file (sans contenu C2+) | RF, DIR | tracé | N1-8 |
| DEM-341 | Reprendre les demandes du Next.js (R9) | ADM · `demandes.reprise` | §29 | N1-5 |
| DEM-342 | Reprendre l'organisation du Next.js comme **proposition** de files et de règles, relue | ADM | §29.2 | N1-5 |
| DEM-343 | Rapport de reprise (comptes, écarts, échantillons) | ADM | | N1-5 |
| DEM-344 | Rejouer la reprise (idempotente) ; retour arrière | ADM | répétition de Noël | N1-5 |
| DEM-345 | Rapprocher les tâches ouvertes avec le 29 (`taches_ouvertes`) | SYS | 29 §8.10 | N1-2 |
| DEM-346 | Neutraliser les files d'envoi et échéances à la restauration (E9) | SYS | registre E12 | N1-0 |
| DEM-347 | Déclarer les natures d'avis et de tâches au kit | code | §26 | N1-1 |
| DEM-348 | Lire le journal des actions de paramétrage | DIR, ADM | | N1-1 |
| DEM-349 | Mettre en lecture seule le guichet (bascule, incident) avec message | ADM | dépôts refusés proprement | N1-5 |

### 22.13 IA et mobile

| N° | Action | Qui · droit | Règles | Lot |
|---|---|---|---|---|
| DEM-350 | Proposer une nature depuis une phrase (pseudonymisée) | IA → demandeur | §20.1 | N1-6 |
| DEM-351 | Résumer un fil long | IA → traitant | C0–C1 | N1-6 |
| DEM-352 | Préparer un brouillon de réponse | IA → traitant | relu | N1-6 |
| DEM-353 | Détecter un doublon probable | IA/SYS | | N1-6 |
| DEM-354 | Repérer un signal grave et proposer le renvoi protégé | SYS (mots d'alerte) puis IA | DEM-246 | N1-6 |
| DEM-355 | Traiter sa file depuis le téléphone (prendre, répondre, clore, photo) | MF | appli unique | N1-7 |
| DEM-356 | Marquer « prête » depuis le téléphone | réceptionnaire | | N1-7 |
| DEM-357 | Déclarer une séance HSE depuis le téléphone | ENS | | N1-7 |

---
## 23. Règles transverses (R-DM1 à R-DM46 ; R-DM47 et suivantes au §36) [R]

| N° | Règle |
|---|---|
| R-DM1 | **Une seule porte, des circuits propriétaires** : une démarche qui a un circuit chez une brique est un raccourci ; elle ne crée aucune demande N1. |
| R-DM2 | Une conversation devient une demande quand il faut un engagement ; la conversion ferme le fil en le disant ; jamais de fil parallèle à une demande. |
| R-DM3 | Ce qui peut être servi tout de suite (certificat de scolarité, document au coffre, récupération de compte) ne devient pas une demande. |
| R-DM4 | Le **demandeur est la personne de la session**, ou la personne choisie par un déposant habilité (pour le compte), jamais une identité saisie (NJ-84). |
| R-DM5 | Le déposant est toujours enregistré et voit ce qu'il a déposé. |
| R-DM6 | Une demande a **toujours** une file, une échéance et une file de repli. |
| R-DM7 | Aucune file n'est « tout le personnel » ; la file de tri a des membres désignés (NJ-85). |
| R-DM8 | Les membres d'une file sont **calculés** depuis les missions, fonctions, équipes et désignations datées du 6, à chaque lecture (NJ-94). |
| R-DM9 | **Une seule prise active** par demande (contrainte d'unicité) ; la prise est un fait de N1, le 29 la reflète. |
| R-DM10 | On ne prend, ne lit, ne transfère ni ne commente une demande d'une file dont on n'est pas membre, consulté, escaladé ou demandeur (NJ-80, NJ-81) ; la réponse est une 404 neutre. |
| R-DM11 | Toute écriture vérifie la **version** de la demande ; une écriture sur une version périmée est refusée (409) (NJ-90). |
| R-DM12 | Un transfert change la file, remet la demande en `deposee` et prévient le demandeur ; la file d'origine ne voit plus la demande (sauf consultation). |
| R-DM13 | Une réattribution ne se fait qu'à un membre de la file, par son responsable. |
| R-DM14 | Une demande « direction » passe d'abord par le secrétariat de direction qui la qualifie ; la transmission au chef est un geste explicite (NXN-05). |
| R-DM15 | L'escalade ajoute un lecteur et une tâche sans ôter le traitant ; pour une nature C2+, l'escalade montre l'existence et le retard, le contenu seulement au porteur du droit de la nature. |
| R-DM16 | **Titre neutre** hors de la demande pour toute nature C2+ ; toute lecture C2+ est tracée ; le demandeur peut savoir quel service l'a lue. |
| R-DM17 | Parents séparés : aucune demande d'un responsable n'est visible de l'autre ; une interdiction empêche toute mention de l'autre dans les avis et pièces produites. |
| R-DM18 | Le demandeur ne voit **jamais** une note interne, le nom d'un consulté, la raison du routage ni une adresse du personnel (NJ-82). |
| R-DM19 | Tout message au demandeur est relu (aperçu) ; aucun message envoyé n'est modifié ; une correction est un nouveau message. |
| R-DM20 | L'attente du demandeur **suspend** le délai ; l'attente d'un tiers aussi si la nature le déclare. |
| R-DM21 | Pour un EPLE et une nature `decision_usager`, l'accusé porte les mentions du CRPA et l'échéance de décision implicite est affichée au personnel [NV]. |
| R-DM22 | Les délais se comptent en jours ouvrés du site ; une demande déposée pendant une fermeture voit son délai partir à la réouverture. |
| R-DM23 | Les personnels reçoivent un **résumé**, pas un avis par demande, dans leurs horaires et hors plage calme ; seules `haute` et `securite` partent tout de suite. |
| R-DM24 | Le demandeur relance une fois par délai dépassé ; pas de relance avant l'échéance. |
| R-DM25 | Aucune relance ni avis pour une demande close, une personne décédée ou un demandeur parti. |
| R-DM26 | Les pièces passent par le socle 7 ; lecture par jeton court lié à la session, après contrôle de droit par N1 (NJ-83). |
| R-DM27 | Aucune pièce jointe dans un e-mail ; les avis externes portent une **enveloppe neutre** et un lien (NJ-86). |
| R-DM28 | Les faits sont en ajout seul et chaînés ; aucune suppression en cascade (NJ-89). |
| R-DM29 | La purge suit la conservation de la nature et passe par l'ordonnanceur, jamais par une lecture ; elle respecte le gel. |
| R-DM30 | Toute nature à texte libre affiche l'avertissement de minimisation ; une nature ne demande pas de donnée de santé. |
| R-DM31 | Harcèlement, violence, danger, information préoccupante, santé : **jamais une demande N1** ; le catalogue ouvre le canal protégé (13, 36, 34) et les numéros d'urgence. |
| R-DM32 | Le routage est déterministe ; l'IA ne route, ne prend, ne décide et n'envoie rien ; elle ne reçoit aucun identifiant d'élève (NJ-87). |
| R-DM33 | Un élève mineur voit, avant de déposer, si ses responsables verront sa demande. |
| R-DM34 | Les demandes d'un élève majeur ne sont pas visibles de ses parents, sauf ce que le socle 4 prévoit. |
| R-DM35 | Une nature C3 ne peut être routée que vers une file confidentielle ; une requalification vers C3 sort la demande de la file précédente et garde l'historique des lectures. |
| R-DM36 | Aucune adresse e-mail n'est jamais saisie comme destinataire d'une demande, d'un avis ou d'une transmission (NJ-84, NJ-91, NJ-94). |
| R-DM37 | Une absence ou un départ ne réattribue rien en silence ; la file est prévenue et la file de repli prend le relais si la file est vide. |
| R-DM38 | Une délégation datée du 6 fait du délégataire un membre des files de la mission déléguée, sans élargir ses droits au-delà de ceux du délégant. |
| R-DM39 | Une action par lien (e-mail) ouvre une page qui demande un geste authentifié ; aucun GET ne modifie (NJ-88, NJ-92). |
| R-DM40 | **Personne ne décide sa propre demande** (HSE, reprographie couleur, remboursement de frais) ; la décision va à un autre décideur. |
| R-DM41 | Une HSE n'est payable que **faite et attestée** ; l'accord seul ne crée aucune heure (NJ-91). |
| R-DM42 | Les heures attestées vont au socle 15 par un port idempotent sur leur origine ; aucune heure n'est comptée deux fois (reprise comprise). |
| R-DM43 | Une demande déposée au nom d'un collègue est confirmée ou retirée par lui avant décision. |
| R-DM44 | N1 ne calcule aucun montant de rémunération ; il constate et publie (15, 49). |
| R-DM45 | La nature `reprographie.tirage` garde son code et ses tables quand le 44 la reprend ; aucune reprise de données entre N1 et le 44. |
| R-DM46 | La file d'impression d'un réceptionnaire ne montre que les sites qui lui sont désignés (NJ-77) ; un PDF à imprimer n'est rattachable que par son déposant (NJ-78). |

---

## 24. Droits et minimisation

### 24.1 Catalogue d'actions proposé (D-017) [R]

| Action | Ce qu'elle ouvre | Portée | Modèle par défaut |
|---|---|---|---|
| `demandes.deposer` | déposer les natures ouvertes au public « personnel » | soi | tout personnel |
| `demandes.deposer_pour_compte` | déposer au nom d'une personne du 4 | établissement | ACC, SEC, CPT, DIR |
| `demandes.traiter` | voir, prendre, traiter les demandes d'une file | **mes files** (calculées) | — (l'appartenance à la file ouvre l'action ; aucun modèle ne la donne à tout un rôle) |
| `demandes.file.repartir` | réattribuer, déléguer, reporter une échéance | files dont on est responsable | responsables de file |
| `demandes.file.superviser` | compteurs, délais, retards, contenu C0–C1 | files désignées ou établissement | DIR |
| `demandes.lire_toutes` | lire les demandes C0–C1 de l'établissement | établissement | chef d'établissement |
| `demandes.superviser_confidentiel` | supervision motivée et tracée d'une file C2+ | file désignée | chef d'établissement (geste exceptionnel) |
| `demandes.hse.decider` | décider les HSE | site | DIR (chef, adjoint) |
| `demandes.hse.attester` | attester le service fait | site | adjoint, CPE (selon l'établissement) |
| `demandes.hse.enveloppe.gerer` | enveloppe d'HSE | site ou établissement | DIR, gestionnaire |
| `demandes.repro.valider` | valider couleur et gros volumes | site | DIR du site |
| `demandes.repro.imprimer` | file d'impression | sites désignés | réceptionnaire (mission « reprographie ») |
| `demandes.parametrer` | natures, files, règles, réponses types de toutes les files | établissement | ADM, DIR |
| `demandes.journal.lire` | journaux de paramétrage et d'accès | établissement | DIR, ADM, DPO |
| `demandes.indicateurs.lire` | indicateurs agrégés | établissement | DIR |
| `demandes.reprise` | reprise Next.js | établissement | ADM |

Familles et élèves : **aucune action de rôle** (D-017) : leurs droits viennent des publics de la nature et des droits du lien (socle 4) ; un responsable n'a jamais `demandes.traiter`.

### 24.2 Ce que voit chacun [R]

| Profil | Voit | Ne voit pas |
|---|---|---|
| Demandeur | ses demandes (projection demandeur) | notes internes, consultés, routage, autres demandeurs (appuis anonymes) |
| Déposant | ce qu'il a déposé, jusqu'à la clôture + 30 jours | la suite des échanges internes |
| Membre de file | demandes de ses files, fiche minimisée du demandeur | demandes des autres files, C2+ sans désignation |
| Consulté | la question et la partie partagée | le reste de la demande |
| Responsable de file | sa file, membres, délais | autres files |
| Direction | supervision (compteurs, retards), C0–C1 si `lire_toutes` | contenu C2+ sans droit |
| Réceptionnaire | demandes acceptées de ses sites, PDF | demandes d'autres sites, demandes non validées |
| Décideur HSE | demandes HSE de son site, enveloppe, historique de l'année d'un agent | HSE des autres sites |
| Responsable légal | ses demandes | celles de l'autre parent, celles de son enfant majeur |
| Élève mineur | ses demandes | — |
| IA | ce que l'utilisateur voit, C0–C1, pseudonymisé pour l'extérieur | C2+, identifiants d'élèves |

### 24.3 Pièges à tester (matrice de séparation)

1. Agent de maintenance → demande RH par son numéro (404). 2. Membre d'une file → prise d'une demande d'une autre file déjà prise (NJ-81). 3. Demandeur → projection sans note interne (NJ-82). 4. Personnel d'une file → pièce d'une autre file (NJ-83). 5. Compte connecté → commentaire sur une demande qui n'est pas la sienne (NJ-80). 6. Corps de requête avec `email` du demandeur (ignoré, NJ-84). 7. Parent A → demande du parent B (404, aucun compteur). 8. Établissement A → demande de B (RLS, `separation.rs`). 9. Réceptionnaire du site 1 → file du site 2 (NJ-77). 10. PDF d'un autre déposant rattaché à ma demande (NJ-78). 11. Décideur HSE → sa propre demande (R-DM40). 12. IA → texte d'une demande C3 (refus). 13. Lien e-mail « prête » ouvert par un tiers (page de connexion, aucun effet, NJ-92). 14. Élève majeur → ses parents ne voient pas sa demande. 15. Deux prises simultanées (une seule). 16. Purge déclenchée par une lecture (aucune, NJ-89).

### 24.4 Conservation et minimisation

§15.3. Aucun numéro de téléphone ni adresse e-mail du demandeur n'est stocké dans N1 : ils sont lus au socle 4 au moment de l'avis (et masqués dans les preuves du 8).

---

## 25. Événements (outbox, dans la transaction du fait) [R]

| Événement | Charge (identifiants seulement) | Consommateurs |
|---|---|---|
| `demandes.deposee` | demande, nature, file, urgence | 8 (accusé, avis file), 29 (tâche de file), 47 |
| `demandes.routee` / `demandes.transferee` | demande, file avant, file après, motif codé | 8, 29 |
| `demandes.prise` / `demandes.relachee` / `demandes.reattribuee` | demande, agent | 29, 8 |
| `demandes.complement_demande` / `demandes.complement_recu` | demande | 8, 29 |
| `demandes.consultation_ouverte` / `demandes.consultation_repondue` | demande, consulté | 8, 29 |
| `demandes.escaladee` | demande, palier | 8, 29 |
| `demandes.decidee` | demande, décision | 8, 29, volets |
| `demandes.traitee` / `demandes.close` / `demandes.rouverte` / `demandes.annulee` | demande, issue | 8, 29, volets (44), 47 |
| `demandes.hse_accordee` | demande, agent, volume, période | 10 (séances proposées), 47 |
| `demandes.hse_attestee` | demande, séance, agent, date, durée, régime | 15 (heures), 49 (RH4) |
| `demandes.repro_validee` / `demandes.repro_prete` / `demandes.repro_retiree` | demande, site | 8, 29 |
| `demandes.file_vide` | file | 8 (responsable, direction) |
| `demandes.renvoi_protege` | demande (sans contenu) | 13 ou 36 (ouvre leur tâche d'accueil, sans contenu recopié) |

Réactions de N1 à des événements des autres : `personnel.absence_debut/fin` (15), `personnel.depart` et `personnel.mission_changee` (6), `delegation.accordee/revoquee` (6), `eleve.decede`, `personne.decedee` (2, 4), `eleve.parti` (3), `lien.modifie` et `interdiction.posee` (4), `calendrier.fermeture` (5), `structure.site_ferme` (5).

---

## 26. Natures déclarées aux socles 8 et 29

### 26.1 Natures d'avis (socle 8) [R]

| Code | Urgence | Sensibilité du contenu | Destinataires | Regroupement | Preuve |
|---|---|---|---|---|---|
| `demandes.accuse_reception` | U1 | C1 | demandeur (et déposant) | aucun | remise ; ouverture pour `decision_usager` |
| `demandes.prise_en_charge` | U0 | C1 | demandeur (si la nature le déclare) | résumé | — |
| `demandes.complement_demande` | U1 | C1 | demandeur | aucun | ouverture |
| `demandes.decision` | U1 | C1 (C0 dans l'enveloppe) | demandeur, bénéficiaire | aucun | **ouverture** (notification effective) |
| `demandes.close` | U0 | C1 | demandeur, appuis, co-demandeurs | résumé | remise |
| `demandes.cloture_sans_reponse_annoncee` | U1 | C0 | demandeur | aucun | remise |
| `demandes.nouvelle_dans_ma_file` | U0 (U2 si `haute`) | C0 (titre neutre) | membres de la file | résumé demi-journée | — |
| `demandes.securite` | **U2** | C0 | direction, astreinte technique, file maintenance | aucun | accusé |
| `demandes.en_retard` | U1 | C0 | palier d'escalade | résumé | — |
| `demandes.file_vide` | U1 | C0 | responsable de file, direction | aucun | — |
| `demandes.repro_prete` | U0 | C0 | demandeur | aucun | ouverture (signal « pas encore vue ») |
| `demandes.hse_a_decider` / `hse_a_attester` | U0 | C1 | décideurs, attestants du site | résumé hebdomadaire (SN1-13) | — |

Toutes : lint de minimisation (aucune variable C2+ dans une enveloppe externe), plage calme, enveloppe e-mail « Votre demande n° … a avancé », aucun objet saisi dans l'objet d'un e-mail pour une nature C2+.

### 26.2 Natures de tâches (socle 29) [R]

| Code | Destinataires | Cardinalité | Échéance | Action de clôture | File de repli |
|---|---|---|---|---|---|
| `dem.a_traiter` | membres de la file | collective | prise en charge | carte « Prendre » (commande N1) | file de repli de la file |
| `dem.en_cours` | traitant | nominative | traitement | lien profond | file |
| `dem.completer` | demandeur | par demandeur | complément | carte « Répondre » | — (sans objet à la clôture) |
| `dem.consultation` | consulté | nominative | échéance de consultation | lien | traitant |
| `dem.en_retard` | palier | nominative | — | lien | direction |
| `dem.hse_decider` | décideurs du site | collective | 5 jours ouvrés | lien | chef d'établissement |
| `dem.hse_attester` | attestants du site | collective | fin de mois | lien (action groupée) | adjoint |
| `dem.repro_imprimer` | réceptionnaires du site | collective | « pour quand » | carte « Prête » | intendance |
| `dem.repro_valider` | valideurs du site | collective | 1 jour ouvré | carte « Valider » | DIR |

Titres neutres : « Demande n° {numero} — {libelle_nature} » (C0–C1) ; « Demande n° {numero} » (C2+).

---

## 27. Écrans (Leptos, `FRONTEND_LEPTOS.md`, `DESIGN_SYSTEM.md`)

Module `ent-web-app/src/modules/demandes/` : `modele.rs` (logique d'écran pure, testée en natif : filtres de file, calcul des états affichés, validation locale des champs, regroupements), vues séparées ; client typé dans `ent-web-api` ; **aucune règle métier dans le frontend** (droits, routage, délais au serveur). Rendu serveur + hydratation ; îlots pour les pages famille.

| Écran | Pour | Contenu | Lot |
|---|---|---|---|
| **Faire une demande** | tous | catalogue par catégories, recherche, raccourcis, « je ne trouve pas », situation grave | N1-4 |
| **Formulaire de nature** | tous | champs déclarés rendus par type (liste, date, nombre, texte, lieu, élève, pièce), aperçu du routage et du délai, avertissement | N1-4 |
| **Mes demandes** | demandeur | liste, états, échéances, actions (compléter, relancer, annuler) | N1-4 |
| **Fiche demande (demandeur)** | demandeur | valeurs, chronologie des échanges qui lui sont adressés, pièces, décision, preuve | N1-4 |
| **Ma file** | membres | colonnes ou liste (à prendre, à moi, aux autres, en attente, en retard), filtres (nature, site, urgence), tri, actions groupées ; clavier | N1-4 |
| **Fiche demande (traitant)** | membres | en-tête (nature, demandeur minimisé, échéances, urgence, raison du routage), fil avec bascule « au demandeur / interne » **explicite**, pièces, consultations, faits, volet de la nature, boutons d'état | N1-4 |
| **Déposer pour le compte** | accueil, secrétariat | recherche bornée de la personne, canal, nature, accusé imprimable | N1-4 |
| **Tableau des retards** | direction, responsables | par file : compte, plus ancienne, médiane | N1-4 |
| **HSE** | enseignant ; décideur ; attestant | « Mes HSE » ; file de décision avec enveloppe ; attestation par lot (séances à cocher) ; enveloppe | N1-4 |
| **Reprographie** | demandeur ; réceptionnaire ; valideur | formulaire (PDF, effectifs, options) ; file d'impression par site triée « pour quand » ; « prête » en un geste | N1-4 |
| **Paramétrage** | administrateur | natures (activer, publics), files (membres par règles, aperçu à la date), règles de routage + simulateur, réponses types | N1-4 |

Exigences : 375 px, clavier, `aria-live` pour les changements d'état, focus visible ; la note interne se distingue par un **libellé** et un **encadré**, pas par la seule couleur ; pagination au-delà de 200 lignes de file ; tests du §9 de `FRONTEND_LEPTOS.md` (modèle natif, contre le vrai serveur avec un rôle refusé, navigateur ralenti ×4).

**Démo de décembre** (D-034) : facultative ; si le temps le permet, « Faire une demande » + « Ma file » sur les natures `numerique.assistance` et `maintenance.intervention` (lots N1-0 à N1-2 seulement).

---
## 28. Modèle de données cible (brique `demandes`)

### 28.1 Principes

Crate `domain-demandes` (nom à aligner sur le kit D-026) ; tables préfixées `ent_dem_` ; chaque table : `etablissement_id NOT NULL`, `UNIQUE (etablissement_id, id)`, clés étrangères **composites** `(etablissement_id, …)`, identifiants UUID, horodatages `TIMESTAMPTZ`, RLS posée par `SELECT ent_rls_poser(ARRAY[…])` dans la migration qui crée la table, classement au registre des tables (`registre_donnees.rs`, E12) ; **aucune colonne JSON** (D-036) ; **aucun `BYTEA`** (pièces au 7) ; `ON DELETE RESTRICT` partout (NJ-89) ; références vers les autres briques (`personne_id`, `eleve_id`, `site_id`, `document_id`, `agent_id`, `fil_id`) **sans** clé étrangère inter-brique quand la brique n'est pas du cœur (règle du 7), avec clé étrangère composite vers les tables du cœur (personnes, élèves, structure).

### 28.2 Tables

```
-- Paramétrage (N1-1)
ent_dem_nature_etab          -- activation d'une nature déclarée dans le code
  id, etablissement_id, nature_code TEXT, actif BOOL, publics_ouverts (table fille ent_dem_nature_public : public_code),
  delai_prise_jours SMALLINT NULL, delai_traitement_jours SMALLINT NULL  -- ≤ délai déclaré (CHECK applicatif)
  file_defaut_id UUID NULL, texte_aide TEXT NULL (≤ 500), maj_le, maj_par
ent_dem_file
  id, etablissement_id, code, libelle, type CHECK (service|tri|direction|confidentielle), site_id NULL,
  confidentialite_max CHECK (C0..C3), delai_vise_jours, horaires_debut TIME, horaires_fin TIME,
  file_repli_id UUID NULL (FK composite), message_accueil TEXT NULL, actif, cree_le, clos_le NULL
ent_dem_file_membre_regle    -- membres CALCULÉS (aucune liste d'adresses)
  id, etablissement_id, file_id, source CHECK (mission_type|fonction|equipe|agent|action_portee),
  source_ref UUID NULL, source_code TEXT NULL, qualite CHECK (membre|responsable|superviseur|confidentiel),
  du DATE, au DATE NULL
ent_dem_file_palier          -- escalade
  id, etablissement_id, file_id, rang SMALLINT, apres_jours SMALLINT, vers CHECK (responsable_file|direction|file), vers_file_id NULL
ent_dem_routage_regle
  id, etablissement_id, nature_code NULL, categorie_code NULL, site_id NULL, cycle_code NULL, public_code NULL,
  urgence NULL, destination CHECK (file|agent|action_portee), destination_file_id NULL, destination_agent_id NULL,
  destination_action TEXT NULL, ordre SMALLINT, du, au NULL
ent_dem_reponse_type
  id, etablissement_id, file_id, libelle, texte (≤ 4000), actif

-- Cœur (N1-0)
ent_dem_demande
  id, etablissement_id, numero TEXT (UNIQUE par établissement), annee_id, nature_code, nature_version SMALLINT,
  statut CHECK (brouillon|deposee|prise|en_attente_demandeur|en_attente_tiers|en_validation|traitee|close|rouverte|annulee),
  issue NULL CHECK (…§7), confidentialite CHECK (C0..C3), urgence CHECK (normale|haute|securite),
  demandeur_personne_id, deposant_personne_id NULL, canal_depot CHECK (web|mobile|comptoir|telephone|courrier|courriel_recu|conversion_message|reprise),
  beneficiaire_agent_id NULL, eleve_id NULL, lieu_id NULL, site_id NULL,
  objet TEXT (≤ 140), texte TEXT NULL (chiffré si confidentialite ≥ C2 : champs_chiffres),
  file_id, attribuee_a_agent_id NULL, attribuee_le NULL,
  depose_le NULL, clos_le NULL, decision CHECK (accordee|accordee_partiellement|refusee) NULL,
  conservation_jusqu_au DATE NULL, gel BOOL DEFAULT false, version INT NOT NULL DEFAULT 1, cree_le, maj_le
  -- index unique partiel : une seule prise active (attribuee_a_agent_id NOT NULL) par demande = colonne unique ; R-DM9
ent_dem_valeur               -- valeurs des champs déclarés, TYPÉES (une ligne par champ et par rang)
  id, etablissement_id, demande_id, valeurs_version SMALLINT, champ_code TEXT, rang SMALLINT,
  valeur_texte TEXT NULL, valeur_entier BIGINT NULL, valeur_decimal NUMERIC(12,2) NULL, valeur_date DATE NULL,
  valeur_heure TIME NULL, valeur_bool BOOL NULL, valeur_code TEXT NULL, valeur_ref UUID NULL,
  CHECK (num_nonnulls(valeur_texte, valeur_entier, valeur_decimal, valeur_date, valeur_heure, valeur_bool, valeur_code, valeur_ref) = 1)
ent_dem_fait                 -- ajout seul (déclencheur refusant UPDATE/DELETE), chaîné
  id, etablissement_id, demande_id, rang INT, type TEXT, auteur_personne_id NULL, auteur_tache TEXT NULL, le,
  statut_avant, statut_apres, file_avant_id, file_apres_id, agent_avant_id, agent_apres_id,
  motif_code TEXT NULL, ref_id UUID NULL, empreinte TEXT (SHA-256 hex), empreinte_precedente TEXT
ent_dem_echeance
  id, etablissement_id, demande_id, type CHECK (prise_en_charge|traitement|complement|cloture_sans_reponse|decision_implicite|verification|pour_quand|escalade),
  palier SMALLINT NULL, due_le TIMESTAMPTZ, suspendue_depuis NULL, traitee_le NULL
ent_dem_numerotation         -- séquence lisible par établissement et année
  etablissement_id, annee_id, dernier INT

-- Échanges et liens (N1-2)
ent_dem_message
  id, etablissement_id, demande_id, auteur_personne_id, visibilite CHECK (demandeur|interne|consultation),
  consultation_id NULL, texte (≤ 8000 ; chiffré si C2+), corrige_message_id NULL, masque_le NULL, cree_le
ent_dem_piece
  id, etablissement_id, demande_id, message_id NULL, document_id UUID, role CHECK (justificatif|annexe|reponse|photo_lieu|fichier_a_imprimer|accord),
  depose_par_personne_id, retire_le NULL, cree_le
ent_dem_abonne
  id, etablissement_id, demande_id, personne_id, qualite CHECK (co_demandeur|appui|copie_interne), du, au NULL
ent_dem_consultation
  id, etablissement_id, demande_id, consulte_agent_id NULL, consulte_file_id NULL, question TEXT, echeance, repondue_le NULL
ent_dem_lien
  id, etablissement_id, demande_id, type CHECK (doublon_de|issue_de_conversation|reouverture_de|avenant_de|reprise_de),
  cible_demande_id NULL, cible_ref UUID NULL, cree_le

-- HSE (N1-3)
ent_dem_hse
  demande_id PK (composite), etablissement_id, beneficiaire_agent_id, regime CHECK (agent_etat|salarie_og|contractuel_etab),
  motif_code, volume_demande_quarts INT CHECK (> 0 AND ≤ 1600), volume_accorde_quarts INT NULL, periode_du, periode_au,
  decideur_agent_id NULL, decide_le NULL, document_accord_id NULL, enveloppe_id NULL
ent_dem_hse_public           -- classes, groupes, élèves désignés (référence + type)
ent_dem_hse_seance
  id, etablissement_id, demande_id, date, debut TIME NULL, duree_quarts SMALLINT, seance_edt_id NULL, lieu_id NULL,
  etat CHECK (prevue|declaree|attestee|contestee|annulee), declaree_le, attestee_par NULL, attestee_le NULL,
  motif_contestation NULL, heure_15_id UUID NULL   -- renseigné par le port du 15 (idempotence)
ent_dem_hse_enveloppe        -- par année et site
  id, etablissement_id, annee_id, site_id NULL, libelle
ent_dem_hse_enveloppe_mvt    -- ajout seul
  id, etablissement_id, enveloppe_id, type CHECK (dotation|abondement|retrait), volume_quarts INT, le, par, note

-- Reprographie (N1-3 ; reprise par le 44 sans reprise de données)
ent_dem_repro
  demande_id PK (composite), etablissement_id, site_id, exemplaires INT CHECK (1..5000), couleur BOOL, recto_verso BOOL,
  agrafage BOOL, format CHECK (A4|A3), pour_le TIMESTAMPTZ, retrait_lieu_id NULL, oeuvres_protegees BOOL,
  pages_total INT, etat CHECK (a_valider|a_imprimer|en_impression|prete|retiree|annulee),
  valide_par NULL, valide_le NULL, prete_par NULL, prete_le NULL, retiree_le NULL
ent_dem_repro_document       -- document_id (7), rang, pages
ent_dem_repro_cible          -- classe_id | groupe_id, effectif au dépôt
ent_dem_repro_oeuvre         -- type (livre|revue|partition|autre), titre, auteur, editeur, pages_copiees, pages_oeuvre NULL
ent_dem_repro_site_reglage   -- site_id, seuil_nb, delai_jours, actif
ent_dem_repro_lieu_retrait   -- site_id, lieu_id, libelle

-- Reprise (N1-5)
ent_dem_reprise_correspondance
  id, etablissement_id, source CHECK (request|demandes_hse|photocopies|routing|org), source_id TEXT, cible_id UUID NULL,
  decision CHECK (reprise|archive|ignoree|a_verifier), raison_code, lot_reprise_id, le
```

**Choix à signaler (D-036)** : `ent_dem_valeur` est une table de **valeurs typées** (une colonne par type, contrainte « exactement une valeur », code de champ **déclaré dans le code** et validé à l'écriture). Ce n'est pas un sac d'attributs texte (défaut de `request_attr`, NJ-90) : chaque valeur a un type, une contrainte et une déclaration. Les natures à forte logique (HSE, reprographie, volets des autres briques) ont **leurs tables dédiées**. Alternative écartée : une table par nature (des dizaines de tables pour des natures de deux champs). **Q-DM9** pour validation par Florian ; recommandation appliquée par défaut (D-031).

### 28.3 Service publié aux autres briques [R]

```rust
pub trait ServiceDemandes {
    /// Crée une demande d'une nature de la brique appelante, dans SA transaction (ex. 44 : demande d'intervention depuis un QR code).
    async fn deposer(&self, tx: &mut TenantTx<'_>, a: &Acteur, d: DepotDemande) -> Result<DemandeCreee, Refus>;
    /// Fait avancer une demande dont la brique porte le volet (ex. 44 : intervention terminée → traitee).
    async fn avancer(&self, tx: &mut TenantTx<'_>, a: &Acteur, id: DemandeId, g: Geste) -> Result<Etat, Refus>;
    /// État minimal d'une demande (pour un écran d'une autre brique), borné par l'acteur.
    async fn etat(&self, a: &Acteur, id: DemandeId) -> Result<EtatDemande, Refus>;
}
```
Lectures publiées : `demandes::mes_demandes(acteur)` (29, 11), `demandes::files_de(agent)` (29 tuiles, 6 départ guidé), `demandes::ouvertes_de(personne)` (27 droit d'accès, 6 départ), `demandes::taches_ouvertes(nature, depuis)` (29 rapprochement), `demandes::agregats(periode)` (47).

---

## 29. Reprise des données du Next.js (N1-5 = R9)

### 29.1 Ce qu'on reprend [R, sur l'inventaire T0]

| Source Next.js | Décision | Comment |
|---|---|---|
| `request` **ouvertes** (NOUVELLE, EN_COURS, EN_ATTENTE) | **reprise intégrale** | nature déduite de la route (`maintenance` → `maintenance.intervention`, `comptabilite` → `comptabilite.demande_interne`, `admin_*` → `secretariat.demande_interne`, `rh_personnel` → `rh.demande_agent`, `cpe_*` et `vie_scolaire_*` → `vie_scolaire.demande_interne`, `accueil` → `secretariat.demande_interne` du site, `corbeille` → file de tri) ; description, commentaires (interne / au demandeur), historique en faits `reprise`, pièces copiées au socle 7 (antivirus, type) ; prise en charge reprise si l'agent est retrouvé, sinon demande remise en file |
| `request` **closes** (TERMINEE) non purgées | **archive** jusqu'à `purge_at` | lecture seule, statut `close`, issue `traitee`, aucun avis, conservation = `purge_at` (pas de prolongation) |
| `request_attr` | repris en valeurs typées quand le champ correspond à une nature ; sinon **note de reprise** interne (texte) | jamais de nouveau champ créé |
| Demandeur | `requester_user_id` → compte Rust par la **table de passage des comptes** (28, MS) ; sinon personne retrouvée **par le secrétariat** dans une liste « demandeurs à rapprocher » ; à défaut, « demandeur externe non vérifié (reprise) » avec l'adresse **masquée** | aucun rattachement automatique par e-mail (NJ-84, NJ-88) ; aucun avis envoyé à un demandeur non rapproché |
| `parentContext` (enfants rattachés par e-mail) | **non repris** | NJ-88 ; le secrétariat rattache l'élève à la main s'il le faut |
| Configuration `requests-routing.json`, `requests-org.json` | **proposition de paramétrage** relue par Florian | unités → files ; tâches → natures ; responsables et membres (adresses) → **agents** retrouvés, transformés en **désignations datées** à convertir en missions ; rapport des adresses non retrouvées |
| HSE `demandes-hse/index.json` (année en cours) | **reprise** | EN_ATTENTE → `rh.hse` `en_validation` (décision à refaire dans ScolIA) ; ACCEPTEE → `close`, issue `accordee`, volume accordé, PDF d'acceptation déposé au coffre de l'agent comme **pièce historique** ; **aucune heure créée au 15** : marquée « transmise hors ScolIA » (CDM-50 : déjà envoyée au rectorat par e-mail ; double paiement sinon) ; REFUSEE, ANNULEE → archive |
| Photocopies `photocopies-couleur/index.json` | **demandes ouvertes seulement** (EN_ATTENTE, ACCEPTEE non prêtes) ; le reste perdu (T0 : perdable) sauf décision de Florian (Q-DM8) | PDF repris au 7 seulement pour les ouvertes |
| Assistance | rien (e-mails) | — |

### 29.2 Déroulé [R]

1. **Répétition de Noël** (D-057) sur l'export anonymisé n° 1 : comptages (par statut, par route), échantillons relus, temps de reprise mesuré.
2. Export anonymisé n° 2 (15 janvier) : seconde répétition, rapport d'écarts à zéro.
3. **Bascule** (20 février) : guichet Next.js en lecture seule (geste de Florian), reprise, contrôle (comptes égaux, 20 demandes ouvertes relues, toutes les pièces lisibles), ouverture de ScolIA ; les demandeurs de demandes ouvertes reçoivent **un** avis « votre demande n° … est désormais suivie dans ScolIA » (enveloppe neutre).
4. **Retour arrière** : la reprise est idempotente (correspondances) ; le Next.js reste intact en lecture jusqu'à la décision de Florian.
Critère go/no-go (plan de bascule) : 100 % des demandes ouvertes reprises et attribuées à une file ; 0 pièce illisible ; 0 heure HSE créée au 15 par la reprise.

---

## 30. Lots de code (équipe Sonnet)

### 30.1 Tableau des lots

| Lot | Contenu | Dépend de | Critère de fin (défauts fermés, tests) | Taille | Semaine (plan) |
|---|---|---|---|---|---|
| **N1-0 — modèle et garde-fous** | crate de la brique ; actions D-017 (§24.1) et modèles de rôles ; tables du cœur (demande, valeurs typées, faits chaînés, échéances, numérotation) ; projections **demandeur** et **traitant** séparées ; contrôle de version ; 404 neutre ; limitation de débit ; dépôt par la session uniquement ; enveloppe neutre ; erreurs neutres ; RLS ; registre E12 ; neutralisation à la restauration ; adaptateurs « source absente » pour 7, 8, 29 si leurs lots ne sont pas versés | socle 1 ; 6 (agents) ; outbox | **NJ-80, NJ-82, NJ-84, NJ-86, NJ-89, NJ-90, NJ-93, NJ-72, NJ-88** fermés (un test chacun, §2.3) ; `t_kit_nature_complete` ; matrice §24.3 pièges 3, 5, 6, 8, 15, 16 | M | 7–13 déc. |
| **N1-1 — natures et routage** | registre des natures (déclaration, assemblage §8.5), catalogue et raccourcis, natures intégrées de la bascule (`numerique.*`, `maintenance.intervention`, `entretien.demande`, `rh.demande_agent`, `comptabilite.*`, `achat.fournitures`, `secretariat.*`, `direction.question`, `vie_scolaire.demande_interne`, `scolarite.*` familles si Q-DM2) ; files, membres calculés, file de tri, file de repli, règles de routage, simulateur ; accusé de réception (à mentions EPLE) ; échéances calculées ; natures d'avis et de tâches déclarées | N1-0 ; 5 (sites, calendrier) ; 6 (missions) ; 8 lot N1 (ou adaptateur) | **NJ-85, NJ-94, NJ-95** ; R-DM1 à R-DM8 ; CDM-01 à CDM-11 | L | 14–18 déc. |
| **N1-2 — prise en charge, délégation, pièces** | prendre, relâcher, réattribuer, déléguer, transférer, consulter, escalader, transmettre à la direction ; complément ; échanges (au demandeur / internes), réponses types ; pièces par le 7 ; pièces produites ; doublons, appuis, fusion ; absences (15), délégations et départs (6) ; résumés et relances ; tableau des retards ; tâches 29 et rapprochement ; purge par l'ordonnanceur (31) | N1-1 ; 7 (dépôt, émission) ; 15 (`indisponibilites`) ; 29 P2–P4 (ou adaptateur) ; 31 ordonnanceur (2620) | **NJ-81, NJ-83** ; R-DM9 à R-DM29, R-DM37, R-DM38 ; CDM-12 à CDM-32 ; pièges 1, 2, 4, 7 | L | 11–17 janv. |
| **N1-3 — HSE et reprographie** | nature `rh.hse` (§17) : demande, pour le compte, décision, enveloppe, accord signé (7), séances, attestation, port `heures::constater` (15) ou export de secours, événement pour le 49 ; nature `reprographie.tirage` (§18) : circuits, validation par site, file d'impression par site, « prête » authentifiée, compteurs, CFC, effacement des PDF | N1-2 ; 6 (`peut_signer`, engagements) ; 7 ; 15 lot A7 (ou export de secours) ; 10 (séances ajoutées, facultatif) | **NJ-91, NJ-92, NJ-77, NJ-78** ; DRM-01 à DRM-13 appliqués ; R-DM40 à R-DM46 ; CDM-41 à CDM-56 ; pièges 9, 10, 11, 13 | L | 25–31 janv. |
| **N1-4 — écrans Leptos** | les écrans du §27 ; mobile web 375 px | N1-1 à N1-3 ; `ent-web-ui` | tests §9 de `FRONTEND_LEPTOS.md` ; un rôle refusé par écran ; lecteur d'écran sur « Ma file » et « Faire une demande » | L | 25–31 janv. (commencé dès N1-1) |
| **N1-5 — reprise R9** | §29 : correspondances, reprise des demandes, de l'organisation (proposition), des HSE, des photocopies ouvertes ; rapport ; rejouable ; lecture seule du guichet pendant la bascule | N1-3 ; outil de reprise (28, R0–R2) ; table de passage des comptes | répétitions de Noël et du 15 janvier sans écart ; critère go/no-go §29.2 ; CDM-50 | M | 25–31 janv. |
| N1-6 — IA | capacités §20 ; pseudonymisation ; jeu d'évaluation | N1-4 ; cœur IA | **NJ-87** (test d'absence d'identifiant d'élève dans tout appel) | S | après bascule |
| N1-7 — mobile natif | iOS, Android (appli unique) : déposer avec photo, QR, traiter sa file, « prête », séances HSE | N1-4 ; socle 11 | parcours mobiles | M | après bascule |
| N1-8 — preuve, conformité, indicateurs | dossier de preuve, journal d'accès, gel, export du droit d'accès, anonymisation, indicateurs pour le 47, supervision confidentielle | N1-2 ; 27 ; 47 | DEM-245 à DEM-255, DEM-339 | M | printemps 2027 |
| N1-9 — natures des autres briques | accueil des natures déclarées par 3, 4, 24, 25, 26, 27, 38, 40, 44 (volets) ; RLS stricte et retraits | selon les briques | kit vérifié pour chaque nature | S par nature | 2027 |

**Ordre** : N1-0 → N1-1 → N1-2 → N1-3 → N1-5, avec N1-4 en parallèle dès que N1-1 est versé (un seul développeur d'écrans, pas de conflit de fichiers : D-026). **Tout N1-0 à N1-5 est nécessaire à la bascule** (M04, M08). Repli si le calendrier glisse (RK-1 du plan) : N1-3 reprographie peut être reporté (photocopies « perdables » : les personnels reviennent temporairement à un dépôt par la nature `secretariat.demande_interne` avec PDF) ; **HSE ne peut pas l'être** (année en cours, alimente la paie).

### 30.2 Préalables chez les autres socles (à confirmer par le pilote)

| Socle | Ce que N1 attend | Si absent au moment du lot |
|---|---|---|
| 6 | agents, missions, fonctions, `delegations_actives`, `peut_signer`, engagement (régime HSE) | désignations datées d'agents dans `ent_dem_file_membre_regle` (source `agent`) ; signature : accord émis sans signature, mention « validé électroniquement par … » |
| 7 | dépôt (type réel, antivirus, identifiant serveur), jeton court lié à la session, émission de gabarits, coffre | **bloquant pour N1-2** : à défaut, pièces refusées (pas de repli vers S3 direct) |
| 8 | natures d'avis, résumés, plage calme, enveloppes | adaptateur sur la chaîne actuelle (comme le 16 M6) : enveloppe neutre seule |
| 15 | `indisponibilites`, port `heures::constater` (à ajouter au 15, §17.6) | absences : aucune proposition automatique (signal manuel) ; heures : export de secours DEM-282 |
| 29 | `ouvrir_tache`, `fermer_tache`, tuiles de file (P2–P4) | « Ma file » de N1 suffit ; tâches ajoutées au rapprochement quand le 29 arrive |
| 31 | ordonnanceur (`ent_auto_planif`, 2620) | tâche de fond propre à N1, idempotente, à retirer ensuite |

---

## 31. Migrations (plage 3040–3059, registre `MIGRATIONS.md`)

| N° | Nom | Lot | Contenu |
|---|---|---|---|
| **3040** | `demandes_n1_0_coeur` | N1-0 | `ent_dem_demande`, `ent_dem_valeur`, `ent_dem_fait` (déclencheur ajout seul), `ent_dem_echeance`, `ent_dem_numerotation` ; RLS (`ent_rls_poser`) ; index unique de prise ; registre |
| **3041** | `demandes_n1_0_droits` | N1-0 | actions D-017 ajoutées aux modèles de rôles par défaut (aucune à tout un rôle pour `demandes.traiter`) |
| **3042** | `demandes_n1_1_files` | N1-1 | `ent_dem_file`, `ent_dem_file_membre_regle`, `ent_dem_file_palier`, `ent_dem_reponse_type` |
| **3043** | `demandes_n1_1_routage` | N1-1 | `ent_dem_nature_etab`, `ent_dem_nature_public`, `ent_dem_routage_regle` ; préréglages par profil (§33) en données de départ **désactivées** |
| **3044** | `demandes_n1_2_echanges` | N1-2 | `ent_dem_message`, `ent_dem_piece`, `ent_dem_abonne` |
| **3045** | `demandes_n1_2_consultations_liens` | N1-2 | `ent_dem_consultation`, `ent_dem_lien` |
| **3046** | `demandes_n1_3_hse` | N1-3 | `ent_dem_hse`, `ent_dem_hse_public`, `ent_dem_hse_seance`, `ent_dem_hse_enveloppe`, `ent_dem_hse_enveloppe_mvt` |
| **3047** | `demandes_n1_3_reprographie` | N1-3 | `ent_dem_repro`, `ent_dem_repro_document`, `ent_dem_repro_cible`, `ent_dem_repro_oeuvre`, `ent_dem_repro_site_reglage`, `ent_dem_repro_lieu_retrait` |
| **3048** | `demandes_n1_5_reprise` | N1-5 | `ent_dem_reprise_correspondance` (classée « reprise », purgeable après décision) |
| **3049** | `demandes_exclusions_restrictions` | N1-2 | **attribuée par la passe personas** (§36.6) : exclusions pour conflit d'intérêts, restrictions de dépôt, compléments HSE et reprographie |
| 3050–3055 | réserve | N1-6 à N1-9 | — |
| **3056** | réserve | supervision confidentielle, journal d'accès (N1-8) | — |
| 3057 | réserve | — | — |
| **3058** | `demandes_rls_stricte` | N1-9 | bascule en RLS stricte (`app.rls_strict`) des tables `ent_dem_*` |
| **3059** | `demandes_retraits` | N1-9 | retrait de la table de reprise après décision (E8) |

**Exceptions D-036** : **aucune** (aucune colonne JSON, aucun tableau, aucun `BYTEA`). Contrôle de revue : `grep -i -E "json|bytea|\[\]" ent-rust/migrations/30[45]*.sql` vide. **`MIGRATION_TAG`** : la plus haute migration présente ; 3040+ ne deviendrait la plus haute que si les socles 50 et 51 (3000–3039) ne sont pas versés avant (aucun impact fonctionnel, règle du registre).

---

## 32. Tests et recette

1. **Un test par défaut Next.js** (§2.3, règle 14 de `PILOTAGE.md`) : 16 NJ nouveaux + NJ-72, NJ-77, NJ-78 ; listés dans le rapport de chaque lot.
2. **Matrice de séparation** (§24.3) générée depuis la déclaration des routes (D-026), dont le cloisonnement entre établissements (`separation.rs`).
3. **Kit** : `t_kit_nature_complete` sur chaque nature déclarée ; lint de minimisation des avis (8) ; titres neutres des tâches (29).
4. **Concurrence** : prises simultanées, transfert pendant une réponse, attestation HSE doublée (idempotence du port 15).
5. **Délais** : calendrier avec vacances, fermeture exceptionnelle, suspension, décision implicite (EPLE).
6. **Reprise** : jeux de données Next.js de labo (ouvertes, closes, HSE acceptées, photocopies ouvertes) ; rejouer deux fois = même résultat ; CDM-50.
7. **Écrans** : §9 de `FRONTEND_LEPTOS.md`.
8. **Recette par personas sur le logiciel** (D-021, étape 4) : les personas de `PERSONAS_N1.md` se connectent en labo, font leur vrai travail et **attaquent** (agent curieux, parent séparé, compte compromis, élève qui devine un numéro).

---

## 33. Réglages signalés (D-023 : peu de réglages, signalés)

| N° | Réglage | Défaut | Qui |
|---|---|---|---|
| SN1-01 | Natures activées et publics ouverts | préréglage du profil d'établissement | ADM |
| SN1-02 | Nom de l'agent visible du demandeur (prénom + initiale / service seul) | prénom + initiale pour les personnels, service seul pour les familles | ADM |
| SN1-03 | Signal d'une prise sans geste | 5 jours ouvrés | ADM |
| SN1-04 | Conservation des brouillons | 30 jours | ScolIA (non réglable) |
| SN1-05 | Délai de réouverture après clôture | 30 jours | ADM |
| SN1-06 | Remettre en file les demandes d'un agent absent de plus de N jours | non (signal seulement) | par file |
| SN1-07 | Relance et clôture sans réponse du demandeur | J+5 / J+15 ouvrés | ScolIA (bornes) |
| SN1-08 | Palier 2 d'escalade (direction) | +5 jours ouvrés après l'échéance | par file |
| SN1-09 | Délais des natures (raccourcissement seulement) | délai déclaré | ADM |
| SN1-10 | Seconde validation des heures HSE au 15 | non (l'attestation vaut validation) | DIR |
| SN1-11 | Seuil noir et blanc sans validation (reprographie) | 500 copies | par site |
| SN1-12 | Délai annoncé de la reprographie | 1 jour ouvré | par site |
| SN1-13 | Fréquence du résumé des HSE à décider / attester | hebdomadaire | DIR |

**Préréglages par profil** (DEM-338) : collège public (files : secrétariat, intendance, vie scolaire, maintenance — agents de la collectivité —, reprographie, direction) ; lycée public (idem + secrétariat des examens) ; groupe scolaire privé sous contrat (files par site et par cycle comme le Next.js actuel, file RH de l'organisme de gestion, comptabilité de l'OGEC).

---

## 34. Décisions appliquées par défaut (D-031) et questions à Florian

### 34.1 Appliquées par défaut (D-031)

1. **Créer le socle N1** (Q-PB5, Q-T0-04, Q-PO3, Q-H) : fait par ce document.
2. **Plage 3040–3059** au lieu de 3000–3019 (déjà prise par le socle 50).
3. Une seule porte, des circuits propriétaires (R-DM1) ; natures déclarées par les briques.
4. Routage déterministe ; IA en proposition seulement, après la bascule (N1-6).
5. Pas de page publique sans compte (D-051) ; dépôt pour le compte par l'accueil.
6. Reprographie intégrée à N1 avec le modèle du 44, reprise par le 44 sans reprise de données.
7. HSE : autorisation préalable, service fait, heures au 15 ; aucun montant calculé.
8. Valeurs typées `ent_dem_valeur` (Q-DM9).
9. Reprise des HSE acceptées **sans** création d'heures au 15 (CDM-50).
10. RHP-252 du 49 sans objet (la reprise des HSE est faite une fois, par N1-5).

### 34.2 Soumis à Florian (engagent l'établissement, ou faits que seul Florian connaît)

| N° | Question | Recommandation |
|---|---|---|
| Q-DM1 | Établissements publics : quelles natures familles sont des « demandes d'usager » au sens du CRPA (accusé à mentions, décision implicite) ? À faire valider par le DPO ou un juriste | toutes celles qui appellent une décision (attestation, duplicata, remboursement, dérogation) ; pas les demandes techniques internes |
| Q-DM2 | Ouvrir les natures **familles** dès la bascule de février, ou seulement les natures internes (ce que le Next.js fait aujourd'hui) ? | **internes seulement en février** (parité, risque réduit) ; familles à la rentrée 2027 ou au printemps si la page parents du Next.js est réellement utilisée |
| Q-DM3 | Anciens élèves (duplicata après le départ) : compte conservé en lecture, ou dépôt par le secrétariat ? | dépôt par le secrétariat pour la V1 |
| Q-DM4 | Durées de conservation par nature (§15.3) | à valider par le DPO |
| Q-DM5 | Liste des **motifs d'HSE** utilisés dans l'établissement (et ceux du public : devoirs faits, stages de réussite…) | liste de départ §17.1, à compléter avec l'adjoint |
| Q-DM6 | L'enveloppe d'HSE se suit-elle en **heures** ou en **euros** ? Qui la notifie (rectorat, DEEP) ? | heures (pas de calcul de taux dans ScolIA) |
| Q-DM7 | Contractuels recrutés par l'établissement sur fonds propres (public) : existent-ils chez vous ? | hors V1 |
| Q-DM8 | Photocopies : reprendre seulement les demandes ouvertes (recommandé) ou rien (T0 : perdable) ; compteurs de l'année à garder (refacturation) ? | ouvertes seulement ; compteurs si refacturés |
| Q-DM9 | Valider le modèle de valeurs typées pour les natures simples (§28.2) | oui |
| Q-DM10 | Qui **atteste** le service fait des HSE dans l'établissement pilote (adjoint, CPE, responsable de niveau) ? | adjoint |
| Q-DM11 | La page Next.js « demande des parents » est-elle activée et utilisée en production ? (à vérifier dans le paramétrage, sans lire la base) | — (conditionne Q-DM2) |
| Q-DM12 | Les demandes de maintenance d'un établissement **public** doivent-elles aller aux agents de la collectivité par ScolIA (partage avec la collectivité) ? | non en V1 : export qualifié (44 §12.2) |
| Q-DM13 | Accepter le **repli** de la reprographie (report de N1-3 reprographie) si le calendrier glisse ? | oui ; HSE jamais reportée |

---

## 35. Faits, hypothèses, contradictions

**Faits [F]** : lecture du module Next.js listé en tête (chemins et lignes cités au §2.2–2.3) ; absence de tout module de demandes dans le Rust (`ent-rust/`, 240 migrations, tête 2240) ; plage 3000–3039 réservée aux socles 50 et 51 dans `MIGRATIONS.md` ; plage 3040–3059 libre sur 177 branches distantes (2026-11-13) ; dernier défaut du registre : NJ-79.

**Hypothèses [H]** : usage réel des HSE et des photocopies (T0 : « partiel ») ; liste des motifs d'HSE ; acteurs et irritants du §4 ; calendrier du §5 ; régime des contractuels sur fonds propres ; règles du CRPA (§6.1 : **[NV]**, citées de mémoire, à vérifier) ; durées de conservation.

**Recommandations [R]** : tout ce qui est marqué [R] ; appliquées par défaut (D-031) sauf §34.2.

**Contradictions signalées**
1. **Plage de migrations** : `PLAN_BASCULE_FEVRIER.md` (§3, Q-PB5) propose 3000–3019, déjà réservée au socle 50 → **3040–3059** ; le plan est à corriger par le pilote.
2. **HSE et paie** : `49_PAIE.md` RHP-252 prévoit d'importer les HSE du Next.js ; `49_PAIE.md` l. 362 dit « HSE des enseignants : état mensuel au rectorat (socle 15) — hors 49 ». Recommandation : reprise par N1-5, heures au 15, le 49 ne lit que les heures des salariés de l'organisme de gestion ; RHP-252 sans objet.
3. **Demandes d'intervention** : `44_RESSOURCES_MATERIELLES.md` §12 porte le guichet des interventions « d'ici N1 » (Q-RM1) ; avec N1, la demande **naît au guichet** et le 44 garde le suivi technique (volet) — conforme au 44 §12.2.
4. **Prise en charge** : `29_PORTAILS.md` possède les prises en charge des tâches collectives (`ent_afaire_prise`, POR-152) ; pour les natures de N1, la prise est un fait de N1 et le 29 la reflète (R-DM9) — à inscrire dans le 29 comme précision de l'arbitrage A4.
5. **Socle 15 : port à ajouter** : `heures::constater` (§17.6) n'est pas dans la liste des ports du 15 (§6.2) ; à ajouter par le pilote ou au lot A7.
6. **Socle 16** : la conversion « fil → demande » (DEM-318) est une action nouvelle pour le 16 (§19.3 du 16) ; à ajouter à sa liste (MSG-176 proposé).

---

## 36. Ajout personas (`PERSONAS_N1.md`, 2026-11-13) — l'emporte en cas de contradiction

### 36.1 Les trois bloquants et leur traitement (appliqués par défaut, D-031)

| # | Bloquant | Traitement |
|---|---|---|
| A | La personne visée lit la demande qui la vise (file RH à un seul membre, question qui concerne le chef) | **R-DM47** : exclusion pour conflit d'intérêts ; file de repli **hors service** obligatoire pour chaque file confidentielle et pour la file direction |
| B | Le guichet devient un canal de harcèlement | **R-DM48** : restriction de dépôt motivée, datée, réexaminée, symétrique de la restriction du 16 |
| C | Avalanche d'avis à l'ouverture de la bascule | **R-DM49** : avis de reprise unique par personne, différé à l'ouverture ; reprises hors des résumés « nouvelles » |

### 36.2 Actions ajoutées (DEM-360 à DEM-383)

| N° | Action | Qui · droit | Règles | Lot |
|---|---|---|---|---|
| DEM-360 | Indiquer « cette demande concerne une personne de ce service » (choix parmi les membres, sans motif) | demandeur (natures C1+) | R-DM47 | N1-2 |
| DEM-361 | Router hors de la personne concernée ; file de repli hors service si la file devient vide ; orientation hors ScolIA si le chef est visé | SYS | R-DM47 ; CDM-57 à CDM-59 | N1-2 |
| DEM-362 | **Restreindre le dépôt** d'une personne (plafond hebdomadaire ou dépôt par l'accueil seulement), motivé, daté, avec date de réexamen | chef d'établissement · `demandes.restreindre` | R-DM48 ; SN1-14 | N1-2 |
| DEM-363 | Voir, réexaminer, lever les restrictions actives | chef d'établissement | trace | N1-2 |
| DEM-364 | Envoyer **un** avis de reprise par personne, regroupé, le jour d'ouverture à 8 h | SYS | R-DM49 ; SN1-16 | N1-5 |
| DEM-365 | Afficher l'indisponibilité du guichet pendant la bascule, avec consigne (« appelez l'accueil ») | ADM | CDM-62 | N1-5 |
| DEM-366 | **Dupliquer** une demande (reprographie, fournitures) | demandeur | R-DM52 | N1-3 |
| DEM-367 | Préparer une demande **récurrente** (brouillon chaque semaine + rappel, jamais de dépôt automatique) | demandeur | R-DM52 | N1-3 |
| DEM-368 | Reprographie **multi-classes** : exemplaires et lieu de retrait par cible | demandeur | | N1-3 |
| DEM-369 | Vue « **ma tournée** » : demandes par site, bâtiment, étage ; urgences en tête | MF maintenance, entretien | | N1-4 |
| DEM-370 | Joindre une **photo avant / après** à la clôture d'une intervention | traitant | pièce `photo_lieu` | N1-2 |
| DEM-371 | Clore « **hors de notre ressort** » (prestataire, collectivité, organisme de gestion) avec destinataire codé et texte au demandeur | traitant | issue `transmise_hors_scolia` ; volet du 44 plus tard | N1-2 |
| DEM-372 | **Transférer ou requalifier en masse** depuis la file de tri | membres de la file de tri | aperçu ; un fait par demande | N1-2 |
| DEM-373 | Mettre « en attente de la direction » sans transmettre (tâche de lecture au chef) | MF secrétariat de direction | | N1-2 |
| DEM-374 | Attestation HSE **pré-cochée par l'appel** d'une séance créée au 10 | SYS → attestant | R-DM50 ; CDM-66 | N1-3 |
| DEM-375 | Attester sur **déclaration sur l'honneur** de l'enseignant, avec contrôle par échantillon | attestant | R-DM50 ; SN1-15 ; CDM-65 | N1-3 |
| DEM-376 | Saisir une **enveloppe prévisionnelle** avant notification | DIR | mouvement `prevision` ; CDM-67 | N1-3 |
| DEM-377 | Voir la **répartition** des HSE accordées par agent sur l'année (équité) | décideur, chef | jamais visible des collègues | N1-3 |
| DEM-378 | Émettre une attestation **signée par délégation** (habilitation de signature du 6) | secrétariat habilité | `peut_signer` ; CDM-68 | N1-2 |
| DEM-379 | Accueil : proposer de **rappeler au numéro connu** avant de déposer pour un appelant | ACC | R-DM51 | N1-1 |
| DEM-380 | Accueil : recherche de la personne **sans** montrer ses demandes existantes | ACC | R-DM51 ; CDM-69 | N1-1 |
| DEM-381 | Élève mineur : déposer sans pièce ; la vie scolaire complète (photo prise au bureau) | ELV ; vie scolaire | | N1-1 |
| DEM-382 | Payer la carte d'élève (lien vers le 25) avant remise ; décision de la vie scolaire sans payeur actif | payeur ; vie scolaire | CDM-70 | N1-9 |
| DEM-383 | Voir les **décisions implicites à venir** sur 30 jours (EPLE) | chef d'établissement | R-DM21 | N1-2 |

### 36.3 Règles ajoutées (R-DM47 à R-DM52)

| N° | Règle |
|---|---|
| R-DM47 | **Conflit d'intérêts** : une personne désignée par le demandeur comme concernée est exclue de la demande (lecture, prise, supervision, escalade, consultation), y compris si elle est responsable de la file ou membre de la direction ; chaque file confidentielle et la file direction ont une **file de repli hors service** ; si le chef d'établissement est visé, la nature oriente vers l'autorité compétente hors ScolIA et ne crée pas de demande. L'exclusion est tracée ; la personne exclue n'est pas informée de la demande. |
| R-DM48 | **Restriction de dépôt** : décidée par le chef d'établissement, motivée, datée, réexaminée au plus tard après 3 mois ; plafond hebdomadaire ou dépôt par l'accueil seulement ; les demandes en cours et les informations dues continuent ; la personne en est informée sans motif détaillé. |
| R-DM49 | **Avis de reprise** : un seul par personne, quel que soit le nombre de demandes reprises, envoyé le jour d'ouverture ; les demandes reprises n'entrent pas dans les résumés « nouvelles » ; aucun avis à un demandeur non rapproché. |
| R-DM50 | **Attestation HSE fondée** : sur un indice (séance au 10 et appel du 12, ou séance confirmée par un tiers) ou sur une déclaration sur l'honneur de l'enseignant avec contrôle par échantillon ; une attestation en masse sans indice exige conf. 2. |
| R-DM51 | **Accueil** : la recherche d'une personne pour un dépôt pour son compte ne montre aucune demande existante ; un appelant non vérifié est rappelé au numéro connu avant toute information ; le doublon n'est signalé qu'à la file. |
| R-DM52 | **Duplication et récurrence** : une demande dupliquée reprend les valeurs non sensibles de la demande du même auteur, jamais ses pièces ni celles d'un autre ; une récurrence prépare un brouillon et un rappel, jamais un dépôt automatique. |

### 36.4 Cas limites ajoutés (CDM-57 à CDM-72)

| N° | Situation | Traitement |
|---|---|---|
| CDM-57 | Une salariée se plaint de la gestionnaire RH, seule membre de la file RH | R-DM47 : exclusion, file de repli hors service (président de l'organisme de gestion, Q-DM14) |
| CDM-58 | Une famille d'un EPLE conteste une décision du principal par `direction.question` | ce n'est pas une demande N1 : rappel des voies de recours ; si la famille maintient, la demande va au secrétariat **sans** être transmise au principal pour décision sur lui-même |
| CDM-59 | Petite école : la secrétaire cumule RH et direction | sa qualité de membre ouvre la lecture (tracée) ; R-DM47 s'applique si elle est visée |
| CDM-60 | Un parent dépose 15 demandes injurieuses en une journée | R-DM48 ; les messages restent au journal ; la direction peut porter plainte hors ScolIA |
| CDM-61 | Ouverture de la bascule : 40 demandeurs de demandes ouvertes | R-DM49 : 40 avis, un par personne, à 8 h le jour d'ouverture |
| CDM-62 | Fuite d'eau pendant la lecture seule du guichet | consigne affichée (accueil) ; demande saisie à l'ouverture par l'accueil, datée de l'appel |
| CDM-63 | Mêmes copies chaque semaine pour quatre classes | une demande multi-classes (DEM-368), dupliquée ou préparée en récurrence (DEM-366, DEM-367) |
| CDM-64 | Collège public : la panne relève des agents de la collectivité | DEM-371 : `transmise_hors_scolia` (collectivité), texte au demandeur ; envoi qualifié par le 44 plus tard |
| CDM-65 | HSE de préparation d'un concours, sans élèves | déclaration sur l'honneur, contrôle par échantillon (R-DM50) |
| CDM-66 | HSE de soutien en 3ᵉB : séances créées au 10, appel fait | attestation pré-cochée (DEM-374) |
| CDM-67 | Septembre : enveloppe pas encore notifiée | enveloppe prévisionnelle (DEM-376), remplacée par la dotation |
| CDM-68 | La délégation de signature de la secrétaire a expiré pendant qu'elle émettait des attestations | `peut_signer` refusé à l'émission : attestation non signée, renvoyée au chef ; aucune attestation déjà émise n'est modifiée |
| CDM-69 | Un père appelle l'accueil pour connaître « la demande de mon ex » | R-DM51 : l'accueil ne voit rien et ne dit rien ; il propose au père ses propres démarches |
| CDM-70 | Élève mineure, carte perdue, aucun payeur actif | la vie scolaire décide : remise sans paiement ou attente ; rien n'est facturé à un responsable sans lien payeur |
| CDM-71 | Élève majeur : relevé de notes nécessaire avant une date proche | champ « pour le » : ordonne la file, n'engage pas un délai plus court que celui de la nature ; urgence `haute` si la nature la permet |
| CDM-72 | Le seul agent de maintenance est absent une semaine | file de repli **métier** (intendance, qui appelle un prestataire), pas le secrétariat de direction (§36.5) |

### 36.5 Précisions (catalogue, préréglages, modèle, écrans)

1. **Réponses types** : une réponse type peut être restreinte à une nature (MN1-012) ; « vue par » interne visible des membres de la file seulement (MN1-014).
2. **Préréglage des files de repli** : maintenance et entretien → intendance ; reprographie → intendance ; RH → **hors service** (Q-DM14) ; direction → **hors service** pour les demandes qui visent le chef ; les autres → secrétariat de direction (MN1-033).
3. **Comptabilité** : `comptabilite.demande_interne` déclare des **pièces exigées par type** (remboursement de frais : justificatif), un circuit **avec validation** par la direction au-delà d'un seuil (SN1-17, défaut 150 €), le doublon par montant, date et demandeur, et **refuse** tout champ de coordonnées bancaires (le RIB est tenu par le 25 et le 49) (MN1-050 à MN1-054).
4. **RH** : la nature `rh.demande_agent` affiche, quand le motif « conflit, harcèlement » est choisi, les voies propres (référent de l'employeur, représentants du personnel, inspection du travail) [NV] et ne force pas le dépôt (MN1-061) ; le cumul direction + RH ne donne aucune lecture par la supervision (MN1-062).
5. **Familles** : champ « pour le » sur les natures familles (MN1-080) ; dépôt pour le compte d'une **personne de confiance** seulement si le socle 4 lui ouvre le droit `deposer_demandes` (MN1-072) ; l'accusé imprimé ne porte rien de l'autre parent (MN1-073).
6. **Décisions groupées** : l'action groupée (DEM-190) s'applique aux HSE de même motif et même volume (MN1-043).
7. **Effectifs pré-remplis** : jamais corrigés des élèves absents de longue durée (MN1-025, refusé : donnée de santé déduite).
8. **Bascule** : rapport « adresses non retrouvées » dès la répétition de Noël ; désignations provisoires datées (source `agent`) acceptées pour le jour J, à convertir en missions avant la rentrée 2027 (MN1-090) ; note aux personnels sur les trois changements voulus (MN1-091, Q-DM15) ; tableau de reprise par statut, par file, demandes sans file, pièces non copiées (MN1-094).
9. **Direction** : résumé hebdomadaire de direction (volumes, retards, décisions implicites) au lieu d'avis unitaires (MN1-098).
10. **Personnel sans accès** (mot de passe perdu) : il dépose par un collègue ou l'accueil (`numerique.acces_compte`, MN1-024).

### 36.6 Modèle (compléments, plage 3040–3059)

| N° | Nom | Lot | Contenu |
|---|---|---|---|
| **3049** | `demandes_exclusions_restrictions` | N1-2 | `ent_dem_exclusion` (demande_id, personne_id exclue, origine `declaration_demandeur` / `systeme`, le) ; `ent_dem_restriction` (personne_id, type `plafond_hebdo` / `accueil_seulement`, plafond, motif TEXT, du, au, reexamen_le, decide_par, levee_le) ; `ent_dem_repro_cible` reçoit `retrait_lieu_id` ; `ent_dem_hse_seance` reçoit `fondement CHECK (appel|tiers|declaration_honneur)` et `echantillon BOOL` ; `ent_dem_hse_enveloppe_mvt.type` accepte `prevision` |

Action ajoutée au catalogue D-017 : `demandes.restreindre` (établissement, chef d'établissement).

### 36.7 Lots révisés

- **N1-1** : + DEM-379, DEM-380, DEM-381 (accueil, élève mineur).
- **N1-2** : + exclusions et restrictions (3049), DEM-360 à DEM-363, DEM-370 à DEM-373, DEM-378, DEM-383 ; critère : pièges « conflit d'intérêts », « restriction », « accueil sans doublon d'autrui ».
- **N1-3** : + DEM-366 à DEM-368, DEM-374 à DEM-377 ; critère : R-DM50.
- **N1-4** : + DEM-369 (tournée).
- **N1-5** : + DEM-364, DEM-365 ; critère go/no-go : un seul avis par personne à l'ouverture.

### 36.8 Réglages ajoutés

SN1-14 (plafond d'une restriction, 3 par semaine), SN1-15 (échantillon de contrôle HSE, 10 %), SN1-16 (heure de l'avis de reprise, 8 h), SN1-17 (seuil de validation d'un remboursement de frais, 150 €).

---

## 37. Précisions de Florian relayées par le pilote (2026-11-13) — « certitude d'être traité » et « Écrire à l'établissement »

**Source** : deux messages du pilote (session de pilotage) qui résument les mots de Florian ; ils ne sont pas une décision écrite de Florian dans `DECISIONS.md`. Ils sont traités ici comme **fil directeur** et passés à l'épreuve par une seconde passe personas (`PERSONAS_N1.md` §10). Ce §37 **l'emporte** sur les §0 à §36 en cas de contradiction.

### 37.1 Le fil directeur

1. **Partir de ce qui marche.** Florian a conçu le module Next.js et le juge « pas trop mal ». La conception garde ce qui fait son succès : file partagée et « le premier qui prend », corbeille de tri, indicateur « direction », dépôt pour un collègue, deux circuits de photocopies (§2.4). Ce qu'elle change : la sécurité (NJ-80 à NJ-95) et la preuve. **Plus simple et mieux** : le demandeur ne voit que **quatre états** en mots simples (« envoyée », « vue par le secrétariat », « en cours », « terminée ») plus « on attend votre réponse » quand c'est le cas ; les dix états du §11.1 restent internes.
2. **La certitude d'être traité est la promesse centrale.** Elle tient en trois preuves visibles du demandeur, dans cet ordre :
   - **envoyée** : accusé immédiat avec le **service** qui traite, le **rôle** de la personne (« le secrétariat des élèves ») et un **délai indicatif** ;
   - **vue** : **nouveau fait `vue_par_service`** — la première ouverture par un membre de la file est montrée au demandeur (« vue par le secrétariat mardi à 10 h 12 ») ; c'est la réponse au « est-ce que quelqu'un l'a lue ? » (R-DM53) ;
   - **prise** : le **prénom et l'initiale** de la personne qui traite dès la prise en charge (« Mme L. s'en occupe »), pour les personnels **et** les familles (modifie SN1-02 : le défaut devient « prénom + initiale » pour tous ; le service seul reste possible par file, pour les files confidentielles et à la demande d'un agent exposé).
   **Ce que j'écarte** : nommer une personne **dès l'accusé**, avant la prise. Le routage va à une file ; nommer « Mme L. » à 22 h alors qu'elle est absente demain trahirait la promesse. L'accusé dit le rôle ; le nom vient avec la prise, qui notifie aussitôt (avis `demandes.prise_en_charge`, désormais **activé par défaut** pour toutes les natures).
3. **Rien ne dort, et ça se voit.** « Personne n'a répondu depuis X jours » remonte seul (escalade, §13.3, déjà là) **et** le demandeur le voit : passé le délai indicatif, son écran dit « en retard de 2 jours — la direction du service est prévenue » (R-DM54). Le retard n'est jamais caché au demandeur.

### 37.2 Une seule porte pour les familles et les élèves : « Écrire à l'établissement »

**Avis franc** : je **rejoins** l'idée de Florian et la recommandation du pilote, avec trois garde-fous. Pour une famille, la plupart des écrits vers l'établissement sont des **démarches** (attestation, régime, rendez-vous, absence prévue, carte perdue) ; les traiter comme des messages libres est précisément ce qui crée l'incertitude. Mais je **ne recommande pas** que les demandes « remplacent » la messagerie : l'échange avec un professeur, le mot au PP, la discussion avec le CPE restent des **conversations** (16), et les formaliser en demandes alourdirait sans rien garantir de plus. D'où une porte unique, et deux destinations.

**Le parcours** (web et mobile, bouton principal de l'accueil famille et élève) :
1. **J'écris librement** (« Mon fils a perdu sa carte de cantine » / « Je voudrais parler au professeur de maths de Léa »), pour **un enfant** choisi (ou pour moi, élève). Le texte reste **local** tant que rien n'est envoyé.
2. **ScolIA propose un aiguillage**, calculé **sur le serveur, avec les droits de la personne**, par **règles explicites** : mots-clés et expressions **déclarés par chaque nature** (et par les raccourcis des autres briques), puis catégories de boîtes du 16. Au plus **trois propositions**, en mots simples :
   - « **C'est une démarche** : *Carte d'élève ou de cantine perdue* — traitée par la vie scolaire, réponse sous 3 jours » → formulaire pré-rempli avec le texte ;
   - « **C'est une démarche gérée ailleurs** : *Prévenir d'une absence* » → raccourci vers le 12 ;
   - « **C'est une question ou un échange** : écrire à *la vie scolaire* / *au professeur principal de Léa* » → message (16), destinataire calculé par la matrice du 16 ;
   - et **toujours** : « **Envoyer simplement comme message** » (au service le plus probable, ou au secrétariat).
3. **La personne valide ou garde un message**, sans jamais être bloquée (R-DM55). Le choix est un geste ; ScolIA n'envoie rien seul.
4. **Signal grave** (mots d'alerte déterministes : harcèlement, frappé, menace, idées noires, suicide…) : en tête des propositions, **sans** bloquer l'envoi, « Parler à un adulte de confiance / signaler une situation » (canal protégé du 36 ou du 13) et les numéros d'urgence (3018, 119) ; si la personne envoie quand même un message, il part à la **boîte vie scolaire** avec l'urgence déterministe du 16 (CM-44), jamais à une file N1 ordinaire (R-DM31 tient).
5. **IA** : **jamais nécessaire**. Elle peut, si l'établissement l'active (N1-6), proposer une nature quand les règles ne trouvent rien, sur le texte **pseudonymisé**, avec les droits de l'utilisateur, par le service métier ; **jamais** sur le texte d'un **élève mineur** par défaut (SN1-18 : désactivé), jamais sur un texte où un mot d'alerte a été repéré.

**Ce que la porte unique change à la conception** :
- Le catalogue « Faire une demande » (§9.1) reste pour les personnels et comme **second chemin** des familles (« voir toutes les démarches ») ; pour les familles, **la porte par défaut est l'écriture libre**.
- L'**aiguilleur** est un service de N1 (`demandes::aiguiller(acteur, texte, eleve) → [Proposition]`) qui interroge les déclarations des natures **et** un port du 16 (`messagerie::destinataires_probables(acteur, eleve, categorie)`) ; il ne stocke rien (texte non conservé, aucun journal du texte, E14) ; il ne propose **que** ce que la personne a le droit de faire (une nature non activée, non ouverte à ce public ou fermée par le lien n'apparaît pas — R-DM56).
- Une **nature non activée** par l'établissement n'est jamais une impasse : l'aiguillage propose alors le message au service correspondant (« Le secrétariat vous répondra »).

### 37.3 Côté établissement : conversion dans les deux sens, une seule boîte de travail

1. **Message → demande** en un geste (DEM-318, déjà là) : le fil d'origine est rattaché, la famille est prévenue en mots simples (« Votre message est devenu une demande suivie : … »), le fil est clos. **Demande → message** (DEM-192) : la demande est close avec l'issue `transmise_conversation`, la conversation reprend le contexte.
2. **Pas deux boîtes concurrentes pour le personnel** (question du pilote) : **recommandation** [R] — la **file de demandes** (N1) et la **boîte de service** (16) d'un même service sont **jumelées** :
   - **mêmes membres** : la règle de membres est définie **une fois**, par les missions du socle 6 (la boîte du 16 rattache déjà ses membres par mission, MSG-161) ; une file N1 peut déclarer `boite_jumelee_id` et **hérite** alors des membres de la boîte (ou l'inverse), jamais deux listes à tenir ;
   - **un seul écran de travail** « **Accueil du service** » (lot N1-4, composé par lecture des deux ports, sans fusion des données) : une liste unique triée par urgence et échéance, chaque ligne marquée *Demande* ou *Message*, mêmes gestes de prise en charge (« Paul s'en occupe »), conversion en un clic ; les compteurs de la tuile de file du 29 additionnent les deux ;
   - **pourquoi pas une fusion des modèles** : une conversation n'a ni nature, ni champs, ni décision, ni délai d'engagement ; une demande en a ; fusionner les tables forcerait l'un des deux modèles à porter les contraintes de l'autre (et reviendrait au sac d'attributs du Next.js, NJ-90). Jumeler les **personnes** et l'**écran** donne au personnel une seule boîte sans perdre les garanties.
3. **Familles qui écrivent « n'importe où »** (au PP, à un professeur, à la mauvaise boîte) : le destinataire **transmet** au bon service (16 MSG-164 pour un message, DEM-176 pour une demande), la famille voit « transmis au secrétariat », jamais un silence.

### 37.4 Mobile (socle 11)

Bouton principal **« Écrire à l'établissement »** ; onglet **« Mes demandes »** à côté de **« Messages »** ; statut en quatre mots simples (§37.1) ; un avis à chaque **étape utile** (envoyée, vue, prise, réponse attendue, terminée) et rien d'autre, regroupé par le socle 8, hors plage calme ; la pastille d'application compte les **réponses attendues de moi** (29 R-PO4), pas les demandes en cours.

### 37.5 Pistes du pilote : retenues, adaptées, écartées

| Piste | Décision | Pourquoi |
|---|---|---|
| Accusé immédiat avec nom ou rôle et délai | **retenue, adaptée** : rôle + délai à l'accusé, nom à la prise, « vue par » entre les deux (R-DM53) | nommer avant la prise promettrait une personne peut-être absente |
| Statut en mots simples | **retenue** (quatre états visibles, §37.1) | — |
| « Personne n'a répondu depuis X jours » qui remonte seul | **retenue** (déjà §13.3) et **montrée au demandeur** (R-DM54) | la certitude passe aussi par l'aveu du retard |
| Réaffectation en un geste quand le destinataire est absent | **retenue** : le responsable de file réaffecte **toutes** les demandes d'un agent absent en un geste, vers un membre ou vers la file (DEM-384) ; toujours pas de réaffectation **silencieuse** | un geste humain garde la responsabilité claire (29 §8.9) |
| Demande sur photo depuis le téléphone (maintenance) | **retenue** : photo d'abord, lieu proposé (QR de la salle, sinon salle de la séance en cours du demandeur au 10, sinon dernière salle choisie), catégorie en un geste (DEM-385) ; aucune géolocalisation | rapide sur le terrain ; la géolocalisation n'apporte rien dans un bâtiment et serait une donnée de trop |
| Modèles de demandes fréquentes | **retenue** : « demandes toutes prêtes » de l'établissement (« Ampoule grillée », « Vidéoprojecteur ne s'allume pas », « Attestation pour la CAF ») = une nature + des valeurs pré-remplies, au plus 20 par établissement (DEM-386, SN1-19) ; et « refaire comme la dernière fois » (DEM-366) | gain de temps réel, aucun formulaire libre |
| Satisfaction en un clic à la clôture | **retenue en partie** : « **Résolu ?** Oui / Non, toujours un problème » (Non rouvre la demande, DEM-129) ; un indicateur agrégé **par file** pour la direction, effectifs < 5 masqués. **Écartée** : la note en étoiles et toute satisfaction **par agent nommé** | une note individuelle devient une évaluation du personnel hors de tout cadre (et un levier de pression des familles) ; ce qui compte pour la promesse est « est-ce réglé ? » |
| Tableau de bord des retards de la direction, sans rien de confidentiel | **retenue** (déjà DEM-181, DEM-230) : compteurs par file et par nature, plus ancienne demande, médiane, demandes « vues mais non prises » ; titres neutres ; aucun contenu C2+ | — |
| Demandes qui « remplacent presque » la messagerie | **adaptée** : porte unique, deux destinations (§37.2) | les échanges pédagogiques restent des conversations |

### 37.6 Cas limites ajoutés (CDM-73 à CDM-86)

| N° | Situation | Traitement |
|---|---|---|
| CDM-73 | Le parent écrit un texte qui mêle une démarche et une plainte contre un professeur | aiguillage : la démarche **et** « écrire à la direction » ; le parent choisit ; aucune nature n'est déduite d'une plainte (R-DM31) |
| CDM-74 | Message sensible d'un élève (« on me frappe dans les vestiaires ») | proposition en tête : canal protégé + 3018, 119 ; envoi possible à la vie scolaire avec urgence ; jamais une demande N1 ordinaire ; jamais d'IA sur ce texte |
| CDM-75 | Élève mineur écrit « je veux changer de régime de cantine » | la nature n'est pas ouverte aux mineurs : proposition « en parler à tes responsables » et message à la vie scolaire ; aucune démarche engageant l'argent |
| CDM-76 | Parent sous interdiction (socle 4) tape « envoyez-moi le bulletin de Léa » | seules les natures que son lien lui ouvre apparaissent ; sinon « Envoyer comme message au secrétariat », traité par le secrétariat selon le 4 ; l'aiguillage ne révèle rien de l'autre parent ni de la mesure |
| CDM-77 | Élève qui devient majeur alors qu'un parent a une demande ouverte le concernant | la demande reste au parent jusqu'à la clôture ; l'aiguillage du parent n'offre plus que les natures que le socle 4 lui laisse ; l'élève majeur voit les siennes |
| CDM-78 | La nature « Attestation » n'est pas activée dans l'établissement | aucune proposition de nature ; « Envoyer comme message au secrétariat » ; le secrétariat peut le convertir en demande d'une autre nature activée |
| CDM-79 | Parent qui écrit en arabe ou en portugais | les mots-clés français ne trouvent rien : l'aiguillage propose le **catalogue traduit** (libellés des natures dans sa langue, socle 8 N4) et « envoyer comme message » ; traduction par IA seulement si activée, sur texte pseudonymisé |
| CDM-80 | Parent pressé qui envoie trois fois le même texte en cinq minutes | doublon signalé avant l'envoi (« vous l'avez déjà envoyé à 8 h 02, il est vu par la vie scolaire ») ; envoi possible quand même, fusion par la file |
| CDM-81 | Le demandeur voit « vue par le secrétariat » mais rien ne bouge pendant 4 jours | passé le délai indicatif : « en retard de 1 jour — le responsable du service est prévenu » (R-DM54) |
| CDM-82 | « Vue par » : un membre ouvre la demande par erreur depuis la recherche | le fait `vue_par_service` est posé à la **première ouverture du détail** par un membre de la file de destination, pas par un consulté ni par une recherche qui n'affiche que le titre |
| CDM-83 | L'agent qui a pris la demande est absent depuis 3 jours | le responsable réaffecte en un geste (DEM-384) ; le demandeur voit le nouveau prénom |
| CDM-84 | Un agent exposé (conflit avec une famille) ne veut pas que son prénom apparaisse | réglage par file ou par agent : « service seul » (SN1-02 modifié), sans justification à donner |
| CDM-85 | Panne signalée en photo pendant un cours dans une salle sans QR | lieu proposé = salle de la séance en cours du demandeur (10), modifiable ; jamais la localisation du téléphone |
| CDM-86 | « Non, toujours un problème » cliqué un mois après la clôture | hors délai de réouverture : nouvelle demande pré-remplie avec le lien vers l'ancienne |

### 37.7 Règles ajoutées (R-DM53 à R-DM58)

| N° | Règle |
|---|---|
| R-DM53 | **Trois preuves visibles** du demandeur : envoyée (service, rôle, délai indicatif), **vue** (première ouverture du détail par un membre de la file, datée), **prise** (prénom et initiale, sauf « service seul »). |
| R-DM54 | Un **retard** est montré au demandeur, avec l'information que le responsable du service est prévenu ; il n'est jamais caché. |
| R-DM55 | L'aiguillage **propose**, la personne **choisit** ; « envoyer comme message » est toujours possible ; rien n'est envoyé ni déposé sans geste. |
| R-DM56 | L'aiguillage ne propose que ce que la personne a le droit de faire (nature activée, ouverte à son public, permise par son lien) et ne révèle rien d'un tiers ni d'une mesure. |
| R-DM57 | Le texte saisi avant l'envoi n'est **ni conservé ni journalisé** ; l'aiguillage est calculé sur le serveur par règles déclarées ; l'IA n'y est jamais nécessaire, jamais appliquée au texte d'un mineur par défaut ni à un texte porteur d'un mot d'alerte. |
| R-DM58 | Une file N1 et une boîte du 16 jumelées ont **une seule** définition de membres ; le personnel les traite dans un **seul** écran ; aucune satisfaction n'est rattachée à un agent nommé. |

### 37.8 Actions ajoutées (DEM-384 à DEM-399)

| N° | Action | Qui · droit | Règles | Lot |
|---|---|---|---|---|
| DEM-384 | **Réaffecter en un geste** toutes les demandes prises par un agent absent (vers un membre ou la file) | RF | §37.5 ; CDM-83 | N1-2 |
| DEM-385 | **Signaler par photo** : photo d'abord, lieu proposé (QR, séance en cours, dernière salle), catégorie | PER | CDM-85 | N1-4 (web mobile), N1-7 (natif) |
| DEM-386 | Gérer les **demandes toutes prêtes** de l'établissement (au plus 20) | ADM | SN1-19 | N1-1 |
| DEM-387 | **« Écrire à l'établissement »** : saisie libre pour un enfant ou pour soi | RESP, ELV, MAJ | §37.2 | N1-4 |
| DEM-388 | **Aiguiller** : proposer au plus trois destinations (nature, raccourci, message) par règles déclarées | SYS | R-DM55, R-DM56, R-DM57 | N1-1 |
| DEM-389 | Choisir une proposition ou « envoyer comme message » | RESP, ELV, MAJ | | N1-4 |
| DEM-390 | Proposer en tête le canal protégé et les numéros d'urgence sur un mot d'alerte | SYS | CDM-74 | N1-1 |
| DEM-391 | Voir le **catalogue traduit** quand l'aiguillage ne trouve rien | RESP | CDM-79 | N1-4 |
| DEM-392 | Poser le fait **« vue par le service »** à la première ouverture du détail | SYS | R-DM53 ; CDM-82 | N1-2 |
| DEM-393 | Montrer au demandeur les **quatre états simples** et le retard | demandeur | R-DM54 | N1-4 |
| DEM-394 | Répondre **« Résolu ? Oui / Non »** à la clôture (Non rouvre) | demandeur | CDM-86 | N1-2 |
| DEM-395 | Voir l'indicateur **« résolu du premier coup »** par file (agrégé) | DIR | effectifs < 5 masqués | N1-8 |
| DEM-396 | **Jumeler** une file N1 et une boîte du 16 (membres hérités) | ADM | R-DM58 | N1-1 |
| DEM-397 | Travailler dans l'**Accueil du service** (demandes et messages, une liste) | membres | R-DM58 | N1-4 |
| DEM-398 | Choisir « service seul » pour ne pas montrer son prénom | agent ; RF pour la file | CDM-84 | N1-2 |
| DEM-399 | Voir les demandes **« vues mais non prises »** depuis plus d'un jour | DIR, RF | tableau des retards | N1-2 |

### 37.9 Ce que cela change pour le socle 16 (à signaler, sans modifier sa conception)

1. **Port à exposer** par le 16 : `messagerie::destinataires_probables(acteur, eleve, categorie)` pour l'aiguilleur (destinataires calculés par sa matrice §8, aucune liste libre).
2. **Jumelage** boîte ↔ file : champ `file_jumelee_id` (ou équivalent) côté boîte, membres hérités d'une seule règle par mission ; lecture des fils de la boîte par l'écran « Accueil du service » de N1 par un port de lecture (`messagerie::boite_lister`), sans accès aux tables.
3. **Conversion** : MSG-176 « convertir en demande » (DEM-318) et réception d'une demande transformée en conversation (DEM-192).
4. **Composeur famille** : le bouton « Écrire » du 16 côté famille devient « Écrire à l'établissement » de N1 (ou l'appelle) ; une seule porte, pas deux boutons concurrents.
5. **Restriction** : aligner la restriction de dépôt (R-DM48) et MSG-226 (une décision, deux effets).
6. **Mots d'alerte** : la liste déterministe du 16 (CM-44) est **partagée** avec l'aiguilleur (une seule liste, tenue au 16 ou au 36).

### 37.10 Lots et réglages révisés

- **N1-1** : + aiguilleur (DEM-388, DEM-390), demandes toutes prêtes (DEM-386), jumelage (DEM-396).
- **N1-2** : + « vue par » (DEM-392), réaffectation en un geste (DEM-384), « Résolu ? » (DEM-394), « service seul » (DEM-398), « vues mais non prises » (DEM-399).
- **N1-4** : + « Écrire à l'établissement » (DEM-387, DEM-389, DEM-391), quatre états (DEM-393), signalement par photo (DEM-385), **Accueil du service** (DEM-397). Si Q-DM2 garde les familles hors de février, l'écran famille est livré mais **désactivé** ; l'Accueil du service sert dès février aux personnels.
- **Réglages** : SN1-02 modifié (défaut « prénom + initiale » pour tous, « service seul » par file ou par agent) ; **SN1-18** IA sur le texte d'un élève mineur (désactivée) ; **SN1-19** nombre de demandes toutes prêtes (20 au plus).
- **Question nouvelle** : **Q-DM17** — Florian valide-t-il que les échanges pédagogiques (professeur, PP, CPE) **restent** des conversations du 16, la porte unique n'envoyant en demande suivie que les démarches ? Recommandation : oui.

### 37.11 Corrections de la seconde passe personas (`PERSONAS_N1.md` §10)

1. **« Je préfère être rappelé »** : case sur les natures familles et sur « Écrire à l'établissement » ; le numéro est celui de la fiche (socle 4), jamais saisi ; le service voit « rappel souhaité » et le rappel est un fait (« appelé le … ») (P13).
2. **Lecture difficile** : les huit démarches les plus fréquentes de l'établissement sont proposées en **grandes vignettes avec pictogramme** sous la zone d'écriture ; la dictée vocale du téléphone (native, rien n'est envoyé à ScolIA avant la validation) est mise en avant (P14).
3. **Élève timide** : l'entrée « Parler à un adulte de confiance » et tout passage par le canal protégé n'apparaissent **jamais** dans ce que les responsables voient (y compris quand la nature d'une demande de l'élève est visible des parents, R-DM33) ; l'élève est prévenu de ce que ses parents verront **avant** chaque envoi (P15).
4. **Secrétaire débordée** : l'Accueil du service filtre par type (demandes, messages), par enfant ou par famille, et propose « tout ce qui attend une réponse de nous » en premier ; les messages de simple information (« merci ») se classent d'un geste sans réponse (P01 bis).
5. **Direction** : le tableau des retards ajoute les demandes « vues mais non prises » (DEM-399) et la part « résolu du premier coup » par file (DEM-395) ; jamais par agent (P10 bis).

## §38 — Décisions de Florian du 2026-11-13 (D-061) — **l'emportent sur tout le reste du document, §37 compris**
- **Q-DM2, Q-DM11** : la page parents du Next.js n'a jamais été ouverte. **Module complet dès le départ**, natures **familles et élèves comprises**, porte unique incluse. Pas de version réduite : ce qui ne sert pas encore est désactivable par établissement, mais existe et est testé dans les lots N1-0 à N1-5.
- **Q-DM6** : l'enveloppe d'HSE et toutes les saisies se comptent **en heures**. Les euros sont calculés par l'application académique, pas par ScolIA (export en heures par type d'heure ; nom de l'application à vérifier [NV]).
- **Q-DM10** : circuit HSE **demande du professeur (heures faites ou prévues) → validation par la direction**, rien d'autre. **Pas d'attestation séparée du service fait** : l'acceptation vaut confiance. L'étape « service fait attesté » devient un réglage facultatif, **désactivé par défaut**. Les heures sont transmises au socle 15 à la validation.
- **Q-DM14** : file de repli RH = **attaché de gestion ou direction**, jamais le président de l'OGEC.
- **Q-DM1, Q-DM4** : pas de DPO identifié ; questions ouvertes (voir D-061).
