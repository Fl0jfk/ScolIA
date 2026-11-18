# Socle 41 — Internat

**Date :** 2026-11-11 — **Statut :** conception de fond, **non décidée** (à relire par Florian). Rédigée **avant** le code des lots suivants, **après** un code Rust existant (migration 066, `domain-eleve/src/internat.rs`) et un module Next.js en production (add-on, D-035). Aucun code écrit ni modifié. Aucun accès à une base, à un secret ni à un environnement de production. Le code Next.js a été **lu seulement**, sur une copie de `origin/main` extraite par `git archive` dans le dossier temporaire de la session (rien exécuté, rien construit, rien modifié, aucun fichier `.env` lu).
**Branche :** `ent-socle-40-41-conception`, partie de `origin/ent-mc0hoe` (`7467f2d4`).
**Passe personas :** `docs/socles/PERSONAS_41.md` (même branche). Ses ajouts sont faits **ici**, marqués **« ajout personas »**, et regroupés au **§30**. **En cas de contradiction, le §30 l'emporte** ; rien n'a été supprimé.
**Socle jumeau :** `docs/socles/40_RESTAURATION.md` (conçu dans la même session). Les deux socles se répondent : la restauration sert le dîner et le petit-déjeuner des internes et apprend de l'internat qui sera là ce soir.
**À lire avec :** `CLAUDE.md` ; `docs/DECISIONS.md` D-017 (droits portés par les actions), D-021 (méthode), D-023 (peu de réglages), D-026 (briques), D-029 (public), D-031 (recommandations appliquées par défaut), **D-034** (feuille de route, année de doublon 2027-2028, finances dans Charlemagne), **D-035** (reprise de l'add-on, branchement sur le cœur), **D-036** (aucun JSON-mémoire) ; `docs/PRA_SAUVEGARDES.md` §18.4 (E1–E14) ; `docs/TRANSITION_T0_INVENTAIRE.md` (M11) ; `docs/SOCLES.md` §41 ; `docs/SOCLE2_DOMAINES.md` D12 (INT-01 à INT-07, repris ici) ; `docs/MATRICE_CHARLEMAGNE.md` §3.12 (SAN-13 à SAN-20) ; conceptions `02_ELEVE.md` (régime, `eleve.regime_changed`), `03_INSCRIPTIONS.md` (R-I15, unités d'hébergement), `04_ADMINISTRATIF.md` (titulaires, interdictions de remise, résidences, `internat_tiers`), `05_STRUCTURE.md` (bâtiments `ent_batiment` 337, fermetures à effets différenciés R-S1, services), `06_PERSONNELS.md` (postes de service `internat_nuit`, planning PRS-133, honorabilité PRS-143, mission `surveillance_internat`), `08_NOTIFICATIONS.md` (`internat.absence_soir`, `internat.bilan_hebdomadaire`), `09_INTEROP.md` (FX-63 registre des internes R. 442-1 ; FX-74 Op@le), `24_FINANCES.md` §11, `34_SANTE.md` (audience « internat », CS-20, fiche d'urgence), `42_SORTIES_VOYAGES.md` (« en sortie », NX-82) et `IA_COEUR.md` (§5, §6.2).

**Conventions.** **[F]** fait vérifié dans le code (Next.js : chemins relatifs à `app/` de `origin/main` ; Rust : chemins depuis `ent-rust/`). **[V]** règle lue sur une source officielle (URL et date au §5). **[V-copie]** texte officiel rapporté par une copie ou un extrait (l'original a renvoyé 403 depuis la session). **[NV]** non vérifié. **[H]** hypothèse de terrain. **[R]** recommandation, non décidée.

Numérotation propre à ce socle (plages choisies pour ne pas heurter les sessions parallèles : le plus haut S- pris sur les branches est S-466, le plus haut M- est M-2591) :

| Élément | Numéros | Remarque |
|---|---|---|
| Actions existantes de Next.js | **NI-01**… | inventaire §2.2 |
| Défauts de Next.js | **DI-01**… | §2.3 |
| Défauts du code Rust existant | **RI-01**… | §2.5 |
| Actions cibles | **INT-100**… | INT-01 à INT-09 sont pris par `SOCLE2_DOMAINES.md` D12 ; INT-01 à INT-07 sont repris ici et renvoient vers INT-1xx |
| Règles | **R-IT1**… | |
| Cas limites | **CIT-01**… | |
| Questions | **Q-IT1**… | |
| Réglages signalés (D-023) | **S-4100 à S-4149** | |
| Manques (passe personas) | **M-4101**… | |

**Plage de migrations imposée : 2820–2839** (inscrite dans `docs/socles/MIGRATIONS.md`). Le lot 0 (I0) part en **2820**.

Sommaire : §0 En bref · §1 Thèse · §2 Existant (Next.js et Rust) · §3 Acteurs · §4 L'année de l'internat · §5 Réglementaire · §6 Frontières et branchements · §7 Concepts · §8 Hébergement · §9 La nuit : appels et présence · §10 Absences, sorties, autorisations · §11 Vie quotidienne · §12 Sécurité, santé, protection · §13 Encadrement · §14 Familles et élèves · §15 Rentrée et installation · §16 Argent · §17 Cas limites · §18 IA · §19 Actions · §20 Règles · §21 Droits et minimisation · §22 Événements · §23 Écrans et mobile · §24 Modèle de données · §25 Lots · §26 Reprise et bascule · §27 Tests et recette · §28 Réglages · §29 Questions et décisions · §30 Ajouts personas · §31 Faits, hypothèses, contradictions.

---

## 0. En bref

1. **L'internat est le seul endroit où l'établissement répond d'un mineur 24 heures sur 24.** Le soir, la nuit, le matin, il n'y a ni professeur ni secrétariat : un ou deux surveillants, un CPE d'astreinte, une liste. La question unique est **« qui dort ici ce soir, et où est chacun maintenant ? »**. Tout le socle sert à y répondre juste, vite, et à prouver ensuite qu'on y a répondu.
2. **Le module Next.js marche et sert** [F] : bâtiments et chambres, internes, appel du soir par section garçons / filles avec états « activité », validation, **correction motivée après validation**, PDF aux directions, rappel automatique ; sorties de groupe avec **décision de la direction puis des parents par lien** ; sorties de week-end ; repas du soir attendus / pris ; incidents à témoins ; alertes d'urgence ; activités, études, planning des surveillants ; **campagne d'installation de rentrée** sur créneaux ; récapitulatif hebdomadaire aux parents ; import Excel et Siècle. **48 actions** (NI-01 à NI-48), **toutes reprises** (§2.2).
3. **Mais il vit sur deux générations qui ne se parlent pas** [F] : un hub JSON (documents entiers réécrits à chaque clic, élèves rapprochés **par nom**, un déduplicateur qui **écrit à chaque lecture**) et six tables typées plus récentes (lits datés, appel par bâtiment, sorties de week-end). Le même soir, **deux appels peuvent diverger**. Surtout : la fiche de l'interne contient **allergies, PAI, traitements, notes médicales** lisibles par les surveillants et le secrétariat (DI-02), le périmètre collège / lycée n'est qu'un filtre d'affichage (DI-01), un seul refus parental annule **toute** une sortie (DI-14). **25 défauts** (DI-01 à DI-25), dont 8 critiques.
4. **Le code Rust existant est un bon embryon, avec trois failles à fermer d'abord (lot I0)** [F] : un professeur lit les **suivis privés** d'un interne (RI-03) ; la présence du soir est **écrasée sans historique** (RI-02) ; la capacité d'une chambre se dépasse en concurrence (RI-10) ; FK simples (RI-01), régime en texte libre (RI-04). **10 défauts** (RI-01 à RI-10).
5. **La valeur ajoutée est le branchement sur le cœur** (§6) : l'interne est un élève du socle 2 dont le **régime daté** ouvre et ferme l'hébergement ; ses titulaires, ses interdictions de remise et ses correspondants viennent du 4 ; le bâtiment du 5 ; les surveillants de nuit et leur planning du 6 ; les autorisations du 18 ; les avis du 8 ; le repas du soir et le petit-déjeuner du 40 ; l'absence du soir et les études du 12 ; l'incident du 13 ; la remise du vendredi du 14 ; les consignes de nuit du 34 (sans diagnostic) ; le séjour scolaire du 42 ; les nuitées et remises d'ordre du 24 (en mode suivi pendant le doublon). **Un changement côté élève se répercute seul** : passage externe, départ, nouvelle interdiction, PAI signé, voyage.
6. **Le réglementaire** (§5) : pas de texte national unique sur l'internat ; un **référentiel national** (chambres et sanitaires **non mixtes**, espace personnel fermant à clé, projet d'internat dans le projet d'établissement, encadrement adapté localement) [V] ; règlement de l'internat dans le règlement intérieur [V-copie] ; **deux exercices d'évacuation de nuit** par an dans les bâtiments d'hébergement (type R, art. R 33) [V-copie] ; PPMS unique (circulaire 2023) [V-copie] ; **prime à l'internat** 327 à 672 € en 2026-2027 [V] ; vidéo interdite dans les lieux de vie (CNIL 2025) [V] ; registre des internes pour le **hors contrat** (R. 442-1) [V-copie]. Neuf points restent à lire sur Légifrance (§5.9).
7. **Ce que ScolIA apporte de plus [H]** :
   - **un seul appel**, par chambre, par étage ou par liste, sur téléphone, **hors ligne**, avec les absents annoncés, les élèves en sortie, en voyage, en stage, en étude ou à l'infirmerie déjà placés ;
   - une **alerte « non retrouvé »** à minutage fixé qui déclenche une procédure guidée (CPE d'astreinte, famille, direction, forces de l'ordre) et horodate chaque geste ;
   - les **absences d'internat annoncées par la famille** dans l'appli, avant l'heure limite, qui **annulent le dîner et le petit-déjeuner** (40) et préviennent le surveillant ;
   - les **sorties** (mercredi, soirée, week-end) autorisées par **les bons titulaires** (pas par une adresse recopiée), l'élève majeur décidant pour lui, avec remise du vendredi vérifiée (14) ;
   - la **liste des présents de la nuit en un geste** pour une évacuation, un confinement ou les pompiers ; les exercices de nuit tracés ;
   - un **cahier de liaison jour / nuit** fait de **faits codés**, jamais de diagnostic ni de jugement ;
   - un **espace famille** de l'internat (chambre, planning, sorties, absences annoncées, bilan de la semaine) ;
   - une **IA qui prépare** (répartition des chambres proposée sous contraintes, synthèse de la semaine, brouillon du bilan aux familles) **et ne décide ni n'envoie rien**, **jamais sur la santé**.
8. **Le plan** :

   | Élément | Volume | Numéros |
   |---|---|---|
   | Actions | 170 | INT-100 à INT-269 (dont 20 ajout personas, INT-250 à INT-269) |
   | Cas limites | 62 | CIT-01 à CIT-62 (dont 14 ajout personas) |
   | Règles | 44 | R-IT1 à R-IT44 (dont 3 ajout personas, R-IT42 à R-IT44) |
   | Réglages signalés | 18 | S-4100 à S-4117 |
   | Lots | 15 | I0 à I14 |

   - **Lot I0** (correctifs urgents de l'existant Rust) : lecture des suivis privés fermée, présence en ajout seul, capacité atomique, FK composites, régime codé, tests (§25.1).
   - **Parcours minimal pour la rentrée 2027** (usage réel, D-035 point 2) : I0, I1, I2 (hors ligne compris, ajout personas), I3, I4 (partie liaison), I5, I7 (partie avis), I8, I10, I11.
9. **Décisions appliquées par défaut (D-031)** : 24 questions tranchées (§29.1). **Soumis à Florian** (§29.2) : le moment de la bascule (recommandé : **été 2027**, avant l'ouverture des créneaux d'installation) ; les heures et seuils du protocole « non retrouvé » à faire valider par la direction et le CPE ; la reprise ou la destruction des données médicales de la fiche Next.js (avec le DPO) ; l'accueil d'internes d'un autre établissement ; la confirmation que le module est en usage réel (Q-T0-14).

---

## 1. Thèse : une nuit est un engagement daté, une présence est un fait horodaté

L'existant traite l'internat comme une **liste d'internes** à laquelle on accroche des marques. C'est ce qui produit les dérives : une fiche qui recopie les parents, une marque qui en écrase une autre, un élève « interne » parce qu'un texte de régime contient « INT ».

ScolIA le traite autrement :
1. **Être interne n'est pas un attribut, c'est une période.** L'élève est interne parce que sa **scolarité datée** (socle 2) porte le régime « interne » sur une période, et l'internat en tire un **séjour d'internat** (dates, rythme de nuits). Changer de régime ferme le séjour ; rien n'est recopié.
2. **Une nuit est un engagement.** Pour chaque soir, ScolIA calcule **qui est attendu** : internes dont le séjour couvre ce soir et ce jour de semaine, **moins** les absences annoncées et autorisées, les sorties, les séjours scolaires (42), les stages (39), les exclusions (13), les fermetures (5), **plus** les nuits exceptionnelles. Cet **attendu** est une vue calculée, jamais saisie.
3. **Une présence est un fait.** Chaque pointage (soir, extinction, ronde, matin) est **ajouté** au journal de la nuit, avec son auteur et son heure ; la valeur courante en est déduite. On corrige en ajoutant un fait motivé, jamais en effaçant.
4. **Un écart est une alerte, pas une ligne.** Attendu non pointé à l'heure, absent sans annonce, sortie non revenue : chaque écart ouvre un **cas** qui ne se ferme que par une issue (retrouvé, famille jointe, autorité saisie), avec un minutage.
5. **On partage des conséquences, jamais des causes.** Le surveillant sait « consigne de nuit : trousse d'urgence, armoire de l'infirmerie, clé au bureau » ; il ne sait pas la maladie (34). Il sait « ne pas laisser sortir avec M. X » ; il ne sait pas pourquoi (4).

---

## 2. Évaluation franche de l'existant

### 2.1 Où et comment Next.js le fait [F]
- **Deux générations** :
  - **JSON** : hub `/gestion-internat?tab=…` (14 onglets dont 6 dans la navigation), routes `/api/internat/*`, gardes `requireInternatAccess` / `requireInternatManage` (`lib/internat-rbac.ts:12-39`). Stockage : documents entiers par établissement dans `ent_collection_record` (anciens chemins S3 `internat/*.json` redirigés, `lib/ent-json-postgres.ts`), lus puis **réécrits en entier** à chaque modification ; un document par soir pour l'appel.
  - **Typée** : six tables `internat_batiment`, `internat_chambre`, `internat_affectation`, `internat_appel`, `internat_appel_ligne`, `internat_sortie` (`db/schema-socle-faits.ts:432-575`), sous-pages `affectations/`, `appel-soir/`, `sorties/`, `repas-soir/`, garde `requireModule("internat")` (qui inclut l'infirmerie). La page d'appel typé l'écrit : « Le roll-call JSON n'est pas modifié ».
- **Environ 50 fichiers propres** : 28 routes, 17 composants, 18 bibliothèques, 3 pages publiques (installation, autorisation par jeton), 6 pages d'administration, 2 scripts d'import (Excel, Siècle).
- Aucun `TODO` dans le code : le « beaucoup de TODO » de T0 est de l'inachevé structurel (deux générations), pas des commentaires.

### 2.2 Ce que fait Next.js : actions existantes, toutes reprises [F]

**Hébergement**

| NI | Action | Où | Reprise |
|---|---|---|---|
| NI-01 | Bâtiments et étages (libellé, ordre, « étage utilisé ») | `api/internat/buildings` | INT-100, INT-101 |
| NI-02 | Chambres 1 à 4 places (bâtiment, étage, aile garçons / filles / mixte) ; refus de réduire sous l'occupation | `api/internat/rooms` | INT-102, INT-104 |
| NI-03 | Plan des chambres, analyse de surcapacité | `InternatRoomsPlanView`, `lib/internat-room-insights.ts` | INT-107 |
| NI-04 | Créer un interne à la main, garde anti-doublon, réactivation | `students` POST | **remplacé** : l'interne naît du régime (INT-110) ; création manuelle = changer le régime au 2 |
| NI-05 | Importer des élèves du référentiel dans l'internat | `students` POST `import` | INT-110 (automatique) |
| NI-06 | Fiche : chambre, contacts parents, infos médicales, autorisations spéciales datées, « sous surveillance » | `students` PATCH | INT-115, INT-160 ; **médical non repris** (R-IT6) ; contacts lus au 4 |
| NI-07 | Sortir ou réactiver un interne avec motif et historique | `students` PATCH | INT-112 (régime, au 2) |
| NI-08 | Supprimer un interne | `students` DELETE | **non repris** (fin de séjour datée, R-IT3) |
| NI-09 | Fusionner les doublons (photo, chambre, contacts) | `students` POST `dedupe` | **sans objet** : l'élève est unique au 2 (fusion ELV) |
| NI-10 | Resynchroniser la classe depuis le dossier | GET | **sans objet** : lue au 2 / 5 |
| NI-11 | Synchroniser le régime (devient interne, sort de l'internat) | `lib/internat-import.ts:393+` | INT-110, INT-112 |
| NI-12 | Liste de référence, import Excel / Siècle, normalisation des régimes | `students/roster` | INT-243 (reprise) ; régimes codés (R-IT2) |
| NI-13 | Scripts d'import hors interface | `scripts/import-*.ts` | INT-243 (outil de reprise) |
| NI-14 | Tableau de bord (internes, occupation, surcapacité, appel du soir) | `lib/internat-stats.ts` | INT-230 |
| NI-15 | Export statistique | `stats/export` | INT-232 |
| NI-16 | Ouvrir un bâtiment, créer une chambre à la volée, **affecter un lit daté**, clore, rechercher | `api/internat/affectations` | INT-103, INT-116 à INT-119 |

**Appel**

| NI | Action | Où | Reprise |
|---|---|---|---|
| NI-17 | Appel du soir ou du matin par section garçons / filles ; marques présent / absent / excusé / activité ; note ; auteur, heure | `roll-call` PATCH | INT-130 à INT-134 (par chambre, étage ou liste ; marques étendues) |
| NI-18 | Terminer une section, valider l'appel ; PDF et courriel aux directions et CPE (collège / lycée séparés) | `roll-call` POST | INT-136, INT-137 |
| NI-19 | **Corriger un appel validé** : motif obligatoire, historique, courriel de correction | `roll-call` PATCH | INT-138 |
| NI-20 | Signal « absent en cours » croisé avec la vie scolaire | `lib/internat-course-absences.ts` | INT-135 (par `eleve_id`) |
| NI-21 | Historique des appels, PDF | `roll-call/history` | INT-139 |
| NI-22 | Rappel si l'appel est incomplet à l'heure paramétrée (21 h, lundi–jeudi) | `roll-call/reminder` | INT-140 (calendrier réel) |
| NI-23 | Périmètre de consultation selon le rôle (direction collège ou lycée, CPE) | `lib/internat-rbac.ts:60-110` | R-IT30 (périmètre serveur) |
| NI-24 | Appel typé sur les lits datés, global ou par bâtiment ; validation qui verrouille | `api/internat/appel-soir` | INT-130 |
| NI-25 | Retirer de l'appel les élèves en sortie de week-end ou en voyage | `lib/internat-sorties-db.ts:190` | INT-129 (attendu calculé) |
| NI-26 | Outil IA : état de l'internat, ouvrir l'appel, affecter une chambre (confirmation) | `brain-ai/tools/handlers/internat.ts` | §18 |

**Sorties**

| NI | Action | Où | Reprise |
|---|---|---|---|
| NI-27 | Sortie de **groupe** (titre, activité, destination, accompagnateurs, date, heures, internes) ; refus si courriel parent absent | `outings` POST | INT-170 |
| NI-28 | Décision de la direction de chaque établissement concerné, par lien ou dans l'ENT | `outings/decision`, `internat/autorisation/page.tsx` | INT-171 |
| NI-29 | Après accord de la direction, demande aux parents ; chaque parent autorise ou refuse par lien | `lib/internat-outing-decision.ts` | INT-172 (socle 18, titulaires) |
| NI-30 | Annuler une sortie ; renvoyer ; décider à la place de la direction ou d'un parent | `outings` PATCH | INT-173, INT-174 (preuve obligatoire) |
| NI-31 | Vue des autorisés du jour ou de la semaine | `outings` GET | INT-175 |
| NI-32 | Récapitulatif hebdomadaire aux parents des sorties de la semaine suivante | `weekly-parent-digest` | INT-201 |
| NI-33 | **Sortie de week-end ou chez un correspondant** (élève, début, fin, motif) ; clôture | `api/internat/sorties` | INT-150 à INT-155 |

**Vie quotidienne**

| NI | Action | Où | Reprise |
|---|---|---|---|
| NI-34 | Repas du soir des internes : attendus, pris, manquants, en sortie | `lib/internat-repas-soir-db.ts` | INT-145 (fourni au 40) |
| NI-35 | Déclarer un incident (plusieurs internes, lieu, heure, témoins élèves et adultes, mesures) + courriel | `incidents` POST | INT-185 (fait de nuit, transmis au 13) |
| NI-36 | Consulter, supprimer un incident | `incidents` | INT-186 ; **pas de suppression** |
| NI-37 | Journal de bord (date, catégorie, contenu) | `journal` | INT-180 (cahier de liaison codé) |
| NI-38 | Messagerie interne par audience | `messages` | **remplacée** par le socle 16 (fil d'équipe « internat ») |
| NI-39 | Activités du soir ; les participants passent « activité » à l'appel | `activities` | INT-190, INT-191 |
| NI-40 | Alertes d'urgence (info, urgent, critique) par courriel | `alerts` | INT-195 (socle 8, U3) |
| NI-41 | Groupes d'étude (jour, heures, salle, surveillant, internes) | `study-groups` | INT-192 |
| NI-42 | Planning des surveillants (date, plage, nom, aile) | `supervisor-shifts` | INT-210 (planning du 6, lu ici) |
| NI-43 | Planning agrégé (activités et sorties visibles des parents) | `planning` | INT-200 |

**Rentrée**

| NI | Action | Où | Reprise |
|---|---|---|---|
| NI-44 | Campagne d'installation : jours, horaires, durée de créneau, familles par créneau, créneaux fermés | `installation` PUT | INT-220 |
| NI-45 | Page publique des créneaux restants, sans donnée personnelle | `installation/public` | INT-221 |
| NI-46 | Réservation d'un créneau par la famille ; confirmation par courriel valable 2 h ; pot de miel, limiteur | `installation/register` | INT-222 (famille connectée ; lien nominatif à défaut) |
| NI-47 | Confirmation par lien, fichier `.ics` | `installation/confirm` | INT-223 (confirmation par POST) |
| NI-48 | Supprimer une réservation | `installation` DELETE | INT-224 (annulation datée) |

### 2.3 Défauts trouvés dans Next.js (à ne pas reproduire ; corrigés dans la réécriture)

Next.js reste intouchable (CLAUDE.md) : ces défauts sont **signalés à Florian** ; ils sont corrigés dans ScolIA, au lot qui reprend l'action.

| DI | Gravité | Défaut [F] | Où | Correctif ScolIA |
|---|---|---|---|---|
| DI-01 | **critique** | Le périmètre collège / lycée n'est qu'un filtre d'affichage : validation et modification de l'appel ne vérifient pas le périmètre | `api/internat/roll-call/route.ts:140, 274` | R-IT30, test d'isolation de périmètre (I2) |
| DI-02 | **critique** | **Allergies, PAI, traitements, notes médicales** dans la fiche JSON de l'interne, lisibles par surveillants et secrétariat, sans journal | `api/internat/students` GET | **non repris** ; consignes du 34 (R-IT6) ; données à remettre au DPO (§26) |
| DI-03 | **critique** | Deux régimes de droits incohérents (rôles JSON ; module typé qui inclut l'infirmerie, sans distinction lecture / gestion / appel) | `lib/internat-rbac.ts`, `lib/intranet-modules.ts:585` | catalogue D-017 (§21.1) |
| DI-04 | haute | Le rôle de terrain `internat` ne peut pas déclarer d'incident mais peut créer des alertes et supprimer des activités | `lib/internat-rbac.ts:32-39`, `alerts/route.ts:14` | §21.1 |
| DI-05 | **critique** | Jetons de décision de sortie **sans expiration**, cherchés parmi les 200 dernières sorties ; nom du répondant saisi librement ; limiteur en mémoire | `lib/internat-storage.ts:339-347` | socle 18 (titulaire authentifié, lien nominatif à usage unique) |
| DI-06 | **critique** | Le personnel peut « autoriser à la place d'un parent » **sans accord de la direction ni preuve** | `lib/internat-outing-decision.ts` | INT-174 : accord reçu hors ligne = **pièce** (7) ou déclaration datée de l'auteur |
| DI-07 | haute | Confirmation d'installation consommée par un **GET** (un antivirus de messagerie confirme à la place du parent) ; réservation sans lien avec l'élève | `api/internat/installation/confirm` | INT-223 (POST, élève du 2) |
| DI-08 | haute | Routes planifiées protégées par un secret comparé en temps non constant, transmis dans le corps, avec repli sur un autre secret | `weekly-parent-digest`, `roll-call/reminder` | tâches internes du serveur, sans route publique |
| DI-09 | haute | Scripts d'import avec `NODE_TLS_REJECT_UNAUTHORIZED=0` par défaut | `scripts/import-internat-excel.ts:113`, `import-siecle-eleves-internat.ts:56` | outil de reprise Rust (§26) |
| DI-10 | moyenne | « Établissement » collège / lycée en texte libre inféré par heuristique ; `eleve_id` sans FK | `lib/internat-etablissement-kind.ts` | niveau et formation lus au 2 / 5 ; FK composites (E2) |
| DI-11 | **critique** | **Double source de vérité** (chambre, appel, sorties) sans synchronisation : deux appels divergents le même soir | JSON / typé | une seule source (§9) |
| DI-12 | haute | Rapprochement des internes **par nom / INE / dossier** ; déduplicateur qui **écrit à chaque lecture** | `students/route.ts:98-110` | `eleve_id` partout |
| DI-13 | **critique** | Écritures non atomiques sur documents entiers : deux surveillants s'écrasent ; cache de 45 s | `lib/internat-storage.ts:56-97` | faits en ajout seul, transactions |
| DI-14 | haute | **Un seul refus parental annule toute la sortie de groupe** | `lib/internat-outing.ts:63-64` | R-IT21 : seul l'élève refusé est exclu |
| DI-15 | haute | Affectation : capacité contrôlée puis insertion sans transaction ; ni sexe, ni régime, ni date vérifiés | `lib/internat-chambres-db.ts:236-338` | R-IT10, R-IT11 |
| DI-16 | moyenne | Jours d'appel lundi–jeudi codés en dur ; jour calculé au fuseau du serveur | `lib/internat-stats.ts:90-93` | rythme du séjour, calendrier du 5, Europe/Paris |
| DI-17 | moyenne | Type d'incident forcé à « incident » ; pas de modification ; suppression dure | `incidents/route.ts:128` | INT-185 (13) |
| DI-18 | moyenne | Messagerie interne : audience jamais appliquée en lecture, accusés jamais écrits | `messages` | socle 16 |
| DI-19 | moyenne | Appel typé : marque acceptée pour n'importe quel élève de l'établissement ; aucune correction après validation | `lib/internat-appel-db.ts` | R-IT14, INT-138 |
| DI-20 | faible | Sorties de week-end qui se chevauchent ; clôtures sans contrôle de dates | `lib/internat-sorties-db.ts` | contrainte d'exclusion (§24) |
| DI-21 | haute | Pas d'historique ni d'audit (incidents, sorties de groupe, journal, chambres, plannings) ; suppressions dures | divers | E6, E7 |
| DI-22 | moyenne | Tables typées sans auteur, sans période d'appel ; surveillants, études, accompagnateurs **en texte libre** | `db/schema-socle-faits.ts` | personnels du 6 |
| DI-23 | moyenne | Sections d'appel `boys` / `girls` figées (sexe binaire), libellés collège / lycée figés | `InternatRollCall` | appel par chambre, étage ou liste (§9) |
| DI-24 | faible | Libellés « Hub JSON » visibles ; sous-pages typées absentes de la navigation | `InternatHubNav.tsx` | sans objet |
| DI-25 | moyenne | Aucune facturation liée aux nuitées ; aucune autorisation parentale persistée ; aucun lien transport | — | §16, §10 |

### 2.4 Ce qu'il ne faut pas perdre (D-035, point 5)
1. L'**appel par sections** avec état « activité » en attente de retour, et l'**interdiction de valider** tant qu'une activité n'est pas revenue.
2. La **correction après validation, motivée, historisée, notifiée** (NI-19) : c'est la bonne pratique, à garder telle quelle.
3. Le **PDF de l'appel** aux directions et au CPE, séparé par formation (collège, lycée).
4. Le **rappel unique** si l'appel n'est pas complet à l'heure dite.
5. Le circuit **direction puis familles** des sorties de groupe, la vue « autorisés du jour », le récapitulatif hebdomadaire.
6. La **campagne d'installation** (créneaux, capacité par créneau, créneaux fermés, double confirmation, `.ics`, protections anti-robot).
7. Les **lits datés** avec une seule affectation ouverte par élève ; l'**appel par bâtiment** ; le retrait automatique des élèves en sortie ou en voyage.
8. Le **refus de réduire une chambre sous son occupation**.
9. Le **repas du soir attendu / pris / manquant / en sortie**.
10. Les **incidents à témoins multiples** et les **alertes d'urgence**.
11. La **confirmation humaine** avant toute action de l'outil IA.
12. L'import **Excel et Siècle** (pour la reprise), qui ne prend jamais de coordonnées bancaires.

### 2.5 Code Rust existant [F]
| Élément | Où | État |
|---|---|---|
| Tables | `migrations/066_internat.sql` : `ent_internat_chambre`, `ent_internat_affectation`, `ent_internat_presence`, `ent_internat_suivi` | `etablissement_id NOT NULL`, UUID, RLS posée (587, mode permissif) ; `created_at` seul ; aucune `modifie_le` |
| Domaine | `crates/domain-eleve/src/internat.rs` (affectation, capacité, normalisation du régime ; 2 tests) | réutilisable |
| Service | `crates/ent-db/src/service.rs` ~12758-13050 ; dépôt `ent-db/src/internat_repo.rs` | |
| Routes | `crates/ent-api/src/routes.rs` : `GET/POST /vs/internat/chambres`, `/affectations`, `POST /affectations/{id}/clore`, `GET/POST /presences`, `POST /eleves/{id}/internat/suivis`, `GET /eleves/{id}/internat` | toutes `internat.gerer` sauf la dernière |
| Droits | `crates/ent-auth/src/actions.rs:112-114, 505-508` : `internat.gerer` (Admin, Direction, Administratif, CPE) | pas d'action pour le surveillant |
| Événements | `internat.chambre`, `internat.affectation`, `internat.presence`, `internat.suivi` (outbox) ; la sortie effective de l'élève ferme l'affectation (`service.rs` ~4955-4975) | |
| Interface | JavaScript `crates/ent-api/src/ui.rs` (`/app/internat`) — F-001, à ne pas étendre ; aucun écran Leptos | |

| RI | Gravité | Défaut [F] | Où | Lot |
|---|---|---|---|---|
| RI-01 | haute | FK simples (`eleve_id`, `chambre_id`, `affectation_id`) ; tables absentes de la reprise composite de 240 ; pas de `UNIQUE (etablissement_id, id)` | `066_internat.sql` | I0 |
| RI-02 | haute | Présence **écrasée** (`ON CONFLICT DO UPDATE` remplace statut, note, auteur, date) ; un seul pointage par jour ; jour passé accepté sans motif | `internat_repo.rs:262-272`, `internat.rs` ~250 | I0 |
| RI-03 | **critique** | Tout rôle qui lit tous les élèves (professeur, infirmerie, psychologue, comptabilité) lit les **suivis privés** (`visible_famille = false`) ; aucune trace de lecture | `service.rs` 13003-13050 | I0 |
| RI-04 | haute | Aucun contrôle du public de la chambre ; régime en texte libre normalisé dans le code (`normalize_regime_internat`) ; trois vocabulaires de régime dans la base | `internat.rs:117-165` | I0 (régime codé), I1 |
| RI-05 | haute | Le changement de régime ne ferme pas l'affectation ; pas de reprise d'une année à l'autre | `service.rs` 4219 | I0 (réaction), I8 |
| RI-06 | haute | Une absence au pointage ne crée ni absence, ni avis, ni annulation de repas, ni alerte | — | I2, I3 |
| RI-07 | moyenne | Chambre non modifiable ni désactivable ; bâtiment en texte | — | I1 |
| RI-08 | moyenne | Aucune consommation d'internat transmise au 24 (seulement tarif par régime et remise d'ordre) | `finances.rs:1368` | I9 |
| RI-09 | haute | Suivis en texte libre de 2 000 caractères (peut contenir de la santé), sans catégorie, sans modification ni audit ; « visible famille » sans règle | — | I0 (borne, avertissement), I4 |
| RI-10 | haute | Capacité : comptage puis insertion non atomiques ; aucun test API ni d'isolation | `service.rs` 12818-12835 | I0 |

### 2.6 Verdict
- **Next.js** : on garde **toutes** les actions (NI-01 à NI-48), on ne reprend **ni** la fiche médicale, **ni** la recopie des parents, **ni** la messagerie maison, **ni** la double génération. Les sorties de week-end typées et l'appel JSON (avec ses corrections) sont les deux meilleures pièces : on les fusionne.
- **Rust** : on garde les tables et le domaine, on les **corrige** (I0) puis on les **étend** (I1–I3) par migrations additives ; `ent_internat_presence` devient une table de faits en ajout seul.

---

## 3. Acteurs, besoins réels, irritants

Abréviations des actions : DIR (direction), CPE, RIN (responsable d'internat, souvent un CPE ou un « maître d'internat » référent), SUR (surveillant de nuit : AED, maître d'internat, veilleur), VS (vie scolaire de jour), ADF (secrétariat), CPT (comptabilité, intendance), INF (infirmière), CUI (restauration), ACC (accueil), PRF (professeur, dont professeur principal), PAR (titulaire de l'autorité parentale), COR (correspondant d'internat), ELV (élève interne mineur), MAJ (élève majeur), AGT (agent d'entretien, de maintenance), SYS (automatisme).

| Acteur | Ce qu'il fait | Ce qui lui fait perdre du temps | Ce qu'il redoute | Ce qui le ravirait | Fréquence |
|---|---|---|---|---|---|
| **Surveillant de nuit** (AED, maître d'internat) | Accueille à 17 h 30, conduit l'étude, le dîner, l'appel, l'extinction, les rondes, le réveil ; gère les incidents ; seul ou à deux | Listes papier fausses (« il est parti chez sa mère, personne ne m'a dit ») ; recopier l'appel ; appeler le CPE pour savoir qui a le droit de sortir ; écrire un compte rendu le matin | **Un élève introuvable à minuit** ; ne pas savoir quoi faire d'un malaise ; être seul face à une bagarre ; ne pas joindre le CPE d'astreinte | Appel sur téléphone, **déjà à jour** des absences, sorties, voyages ; un **bouton « non retrouvé »** qui dit quoi faire ; consignes de nuit visibles en un geste ; transmission du matin pré-remplie | chaque soir |
| **CPE / responsable d'internat** | Règlement de l'internat, affectations, autorisations, astreinte, lien avec les familles, suivi éducatif, transmissions jour / nuit | Courriels des familles « il ne rentre pas ce soir » à 18 h ; tableaux de chambres ; relancer les autorisations ; refaire le planning ; reconstituer un incident | Une fugue découverte au matin ; un appel à minuit sans les informations ; une plainte de famille sur la chambre ; une affectation « garçon dans l'aile filles » | **Tableau de la nuit** (attendus, présents, écarts) ; demandes de sortie **déjà triées** ; répartition proposée ; bilan de la semaine prêt à relire | quotidienne |
| **Direction** | Responsable de la sécurité, adopte le règlement, décide des exclusions d'internat, reçoit les appels graves, valide les sorties de groupe | Être réveillée pour une information incomplète ; ne pas avoir la liste des présents pour les pompiers | **Un drame la nuit** ; un exercice d'évacuation de nuit non fait ; un contrôle de sécurité | **Liste des présents de nuit** en un geste, exercices tracés, alerte graduée (elle n'est réveillée que pour ce qui le mérite) | hebdomadaire (et la nuit, rarement) |
| **Surveillant de jour / vie scolaire** | Absences de cours des internes ; sortie du mercredi ; transmission avec la nuit | Ne pas savoir qu'un interne a passé une mauvaise nuit ; découvrir le matin un « absent » qui dormait à l'infirmerie | Un élève qui manque les cours après une sortie du mercredi | **Transmission du matin** lue en un écran ; « absent de l'appel du soir » visible à 8 h | quotidienne |
| **Infirmière** | Traitements du soir, PAI, consignes de nuit, retour d'un malade | Recopier les consignes pour la nuit ; apprendre le lendemain une crise nocturne | Une trousse introuvable la nuit ; une consigne non vue | Consignes **publiées une fois** (34, audience « internat ») et visibles des surveillants ; **mouvement de nuit** saisi par le surveillant, transmis à elle | quotidienne |
| **Restauration** | Dîner et petit-déjeuner des internes | Effectif du soir faux (absents non annoncés) | Gaspillage ; un panier oublié | **Effectif du soir calculé** à l'heure limite (attendus moins absences annoncées) | quotidienne |
| **Secrétariat / intendance** | Inscriptions à l'internat, trousseau, chèques de caution, facturation, prime d'internat | Ressaisir les internes de Charlemagne ; ne pas savoir qui a quitté l'internat | Facturer un trimestre à un élève parti | Régime daté du 2 lu directement ; **nuitées et remises d'ordre calculées** (24) | mensuelle |
| **Accueil** | Reçoit les familles du vendredi, les livraisons, les appels | « Est-ce que mon fils est rentré ? » au téléphone ; une personne inconnue vient chercher un interne | Remettre un mineur à la mauvaise personne | **Remise** du 14 avec les personnes **autorisées aujourd'hui** et les **interdictions en rouge** | hebdomadaire |
| **Professeur** (dont professeur principal) | Peut animer une étude ; voit un élève fatigué | Rien de structuré sur la vie à l'internat | Ignorer qu'un élève vit mal l'internat | Indicateur « difficultés à l'internat » partagé **sans détail** par le CPE (s'il le juge utile) | rare |
| **Titulaires de l'autorité parentale** (dont parents séparés, à l'étranger) | Inscrivent, signent le règlement, autorisent les sorties, préviennent des absences, paient | Courriels sans réponse ; papiers à signer le lundi matin ; ne pas savoir le programme du mercredi | **Ne pas être prévenu** d'un incident ; que l'autre parent récupère l'enfant un week-end qui n'est pas le sien | **Annoncer une absence en deux gestes** avant 17 h ; **autoriser la sortie** sur le téléphone ; voir la chambre, le planning, le bilan de la semaine ; être prévenu **à temps** | hebdomadaire |
| **Correspondant d'internat** (tiers désigné par la famille) | Accueille l'interne le mercredi ou le week-end, peut le récupérer | Ne pas savoir quand l'élève arrive | Être tenu pour responsable sans en avoir été informé | Être **désigné par les titulaires** avec ses jours ; recevoir l'avis de départ | hebdomadaire |
| **Interne mineur** | Vit là : chambre, étude, repas, sorties, activités | Demander un papier pour tout ; ne pas savoir qui est de garde ; files au self | **Être humilié** (appel public, fouille, exposition de sa vie privée) ; harcèlement dans la chambrée | Voir **son planning**, ses sorties autorisées, son lit ; **signaler** un problème discrètement (socle 36) ; s'inscrire à une activité | quotidienne |
| **Interne majeur** | Idem, signe pour lui-même | Qu'on demande l'accord de ses parents | Être traité comme un mineur | **Décider de ses sorties** dans les règles du règlement, ses parents informés seulement s'il est à leur charge et si le règlement le prévoit | hebdomadaire |
| **AESH** (élève en situation de handicap interne) | Accompagne en journée ; rarement le soir | Pas de lien avec l'équipe de nuit | Ce que vit l'élève la nuit lui échappe | Consignes d'accompagnement de nuit transmises (35) ; chambre adaptée (accessibilité) | rare |
| **Agent d'entretien, maintenance** | Répare, nettoie, gère les clés | Signalements oraux | Entrer dans une chambre occupée | **Demande d'intervention** sur une chambre (socle 44), sans nom d'élève | hebdomadaire |
| **Chef de sécurité / référent sûreté** | Exercices, PPMS, registre | Compter les présents à l'improviste | Un exercice de nuit sans liste fiable | Liste des présents **par bâtiment**, horodatée, exportable ; exercice tracé avec durée | 4 fois par an |
| **Assistant IA** (au nom d'une personne) | Prépare une répartition, un bilan, un résumé | — | — | Lectures dans la portée de la personne ; **aucune** donnée de santé ; aucune décision | — |

---

## 4. L'année de l'internat vue du terrain [H, à confirmer avec Florian]

| Période | Ce qui se passe | Ce que ScolIA fait |
|---|---|---|
| **Mars – juin** | Demandes d'internat (inscriptions, réinscriptions), commission d'internat, capacité par unité | Le 3 compte les places par **unité d'hébergement** (R-I15) ; le 41 fournit la capacité **réelle** (lits actifs, filles / garçons, chambres adaptées) et les sortants |
| **Juin – juillet** | Préparation des chambres ; ouverture des créneaux d'installation ; règlement de l'internat envoyé à signer ; trousseau | **Répartition proposée** (contraintes, vœux, fratries, niveaux, accessibilité) ; campagne d'installation ouverte aux familles des internes inscrits ; signature du règlement (7, 18) ; autorisations annuelles de sortie (18) |
| **Fin août – septembre** | Installation (souvent le dimanche soir ou la veille de la rentrée) ; premiers appels ; premier **exercice d'évacuation de nuit** dans le mois | Créneaux réservés ; **affectations ouvertes à la date d'installation** ; rappel de l'exercice de nuit ; consignes de nuit publiées par l'infirmière |
| **Chaque semaine** | Arrivée du dimanche soir ou du lundi, sorties du mercredi, départ du vendredi | Attendus calculés, absences annoncées, **avis de départ** aux familles et correspondants, remise du vendredi (14) |
| **Chaque soir** | Étude, dîner, activités, appel, extinction, rondes | Appel guidé, alerte « non retrouvé », transmissions |
| **Chaque mois** | Bilan, facturation | Nuitées et remises d'ordre transmises au 24 (en mode suivi pendant le doublon) |
| **Vacances** | Internat fermé (ou ouvert pour quelques élèves éloignés) | Rythme suspendu par le calendrier (5) ; **nuits exceptionnelles** pour les élèves accueillis |
| **Juin** | Départ, état des lieux, restitution des clés et cautions | Fin des séjours datée ; **état des lieux de chambre** (44) ; cautions (24) ; données de l'année archivées selon les durées (§21.3) |

---

## 5. Ce qui est réglementaire (recherche du 2026-11-11)

**Limite d'accès** : Légifrance et le Bulletin officiel ont renvoyé 403 ou 503 depuis la session pour la plupart des pages ; plusieurs points sont donc **[V-copie]** et doivent être relus sur le texte consolidé avant d'être codés en dur (§5.9). Les autres conceptions (24, 34, 42) ont déjà vérifié une partie des textes communs ; elles sont citées.

### 5.1 Cadre
- **Référentiel national « L'internat de la réussite pour tous »** (education.gouv.fr, PDF) [V] : le règlement intérieur organise la vie collective en préservant l'espace personnel ; **chambres et sanitaires non mixtes** ; **espace personnel fermant à clé** ; pas de réunion d'élèves dans une chambre ; le **projet d'internat** fait partie du projet d'établissement et définit les missions des assistants d'éducation et l'articulation **équipe de jour / équipe de nuit** ; le **CPE** assure le lien jour / nuit et la relation avec les familles ; **« les normes de référence pour l'encadrement en internat doivent être adaptées aux situations locales »**.
- **Label « internat du XXIe siècle »** : arrêté du 16 novembre 2020, BO n° 46 du 3 décembre 2020, cahier des charges annexé [V-copie] ; plan lancé le 1er juillet 2019 [V-copie].
- **Aucun taux d'encadrement national chiffré trouvé** [NV pour tout chiffre] → **conséquence [R]** : l'effectif minimal de nuit est un **réglage de l'établissement** par bâtiment (S-4106), jamais une constante.
- **Règlement de l'internat** : partie ou annexe du règlement intérieur adopté par le conseil d'administration dans le public (C. éduc. R. 421-20) [V-copie] ; contenu du règlement intérieur (circulaire 2000-106 du 11 juillet 2000) [V-copie]. Dans le **privé sous contrat**, le règlement est arrêté par le chef d'établissement et l'organisme de gestion [H].
- **Surveillance des élèves** : circulaire 96-248 du 25 octobre 1996 [V sur copie académique] : les déplacements des internes sont encadrés sauf autorisation ; les **élèves majeurs** sont soumis au règlement comme les autres ; le majeur justifie lui-même ses absences, les parents restant informés des perturbations de scolarité s'il est à leur charge.

### 5.2 Mixité, intimité
- Chambres et sanitaires **non mixtes** [V, référentiel] → **R-IT11** : une chambre porte un **public** (filles, garçons) ; l'affectation d'un élève dont le sexe d'état civil (2) ne correspond pas est **refusée**, sauf **dérogation** de la direction, motivée par un code, **jamais affiché** (cas d'un élève transgenre accompagné, d'une chambre individuelle adaptée) (CIT-12).
- **Espace personnel fermant à clé** [V] → la remise des clés et casiers est tracée (44), pas leur contenu.
- **Vidéo** : la CNIL (fiche du 12 septembre 2025) [V] limite la vidéo aux accès et circulations ; elle exclut les **lieux de vie** ; elle ne parle pas des dortoirs, mais par analogie (chambres d'EHPAD) la caméra en chambre est à exclure [NV] → **ScolIA ne gère aucune vidéo** ; le socle ne prévoit aucun champ « caméra » (R-IT41).

### 5.3 Sorties et autorisations
- **Aucun texte national unique** sur les sorties de l'internat : elles relèvent du **règlement** [V-copie, circulaire 2000-106] ; les règlements imposent en pratique une **autorisation écrite des titulaires** pour les sorties de semaine [V-copie, règlements d'établissements].
- **Conséquence [R]** : types de sorties **paramétrables** par l'établissement (mercredi après-midi, soirée, nuit chez le correspondant, week-end, exceptionnelle) avec, pour chacun, **qui autorise** (titulaires, direction, CPE), **quel délai**, **qui peut récupérer** ; preuve conservée (18) ; **majeur** : il décide lui-même dans le cadre du règlement (C. civ. 414).
- Autorité parentale : l'accord d'un titulaire vaut pour les actes usuels, l'accord de l'autre étant présumé (C. civ. 372-2, vérifié par les socles 4 et 42) ; **le désaccord connu** de l'autre titulaire bloque (R-IT20).

### 5.4 Argent
- **Prime à l'internat** : C. éduc. **D. 531-42 et D. 531-43** (décret 2020-1011) [V-copie] ; attribuée à l'interne **boursier**, versée par trimestre, imputée sur la pension ; montants annuels **2026-2027** [V, annexes 6 et 7 du barème, education.gouv.fr, 20 mai 2026] : **collège** échelons 1 à 3 = 327 / 396 / 465 € ; **lycée** échelons 1 à 6 = 327 / 396 / 465 / 534 / 603 / 672 €. Le socle 24 a relevé les mêmes montants [V].
- **Tarifs** : dans le public, fixés par la **collectivité de rattachement** (C. éduc. R. 531-52 [V par le socle 24]) ; **remise d'ordre** selon le règlement du service d'hébergement de la collectivité, seuil courant « plus de deux semaines » [V-copie, réponses parlementaires anciennes] ; la remise de principe a été abrogée (décret 2016-328 [V par le socle 24]). Dans le **privé sous contrat**, l'internat est un service facultatif de l'organisme de gestion ; les redevances figurent au contrat (R. 442-48 [V par le socle 24]).
- **Conséquence [R]** : l'internat transmet au 24 des **quantités** (nuits dues, nuits non servies et leur cause codée), jamais un montant (règle du 24 §11) ; seuils et cas de remise = règlement financier de l'établissement (réglages du 24, S-157).

### 5.5 Sécurité
- **Établissements recevant du public, type R avec locaux à sommeil** : arrêté du 25 juin 1980 (dispositions générales) et arrêté du 4 juin 1982 (type R) modifié [V sommaire Légifrance ; article non lu]. **Article R 33** [V-copie, guides académiques] : exercices d'évacuation pendant l'année avec compte rendu au **registre de sécurité** ; dans les établissements avec internat, **deux exercices supplémentaires de nuit** dans les bâtiments d'hébergement ; premier exercice **dans le mois qui suit la rentrée**.
- **Registre de sécurité** : CCH R. 123-51 [V-copie] → tenu par l'établissement ; ScolIA **trace** les exercices (date, heure, bâtiment, durée, effectif présent, observations) et les exporte, sans prétendre tenir le registre officiel (Q-IT6).
- **Liste nominative des présents la nuit** : aucun texte lu ne la nomme [NV] ; elle découle du besoin d'évacuation et du PPMS → ScolIA la **produit en temps réel** (INT-196).
- **PPMS unique** (risques majeurs et attentat-intrusion) : circulaire du 8 juin 2023, BO n° 26 du 29 juin 2023 [V-copie] ; dispositions propres à l'internat [NV] → le socle 14 possède le PPMS ; le 41 lui fournit les **présents de nuit par zone**.

### 5.6 Santé la nuit
- Missions des infirmiers : circulaire 2015-119 du 10 novembre 2015 [V-copie] ; **aucune permanence infirmière de nuit générale** trouvée [NV].
- **Conséquence [R]** : la nuit, le surveillant voit les **consignes de nuit** (34, audience « internat ») : code fonctionnel, lieu de la trousse, conduite « voir la trousse » ; il saisit un **mouvement de premiers secours** sans texte médical (34 SAN-224), appelle le **15** en cas d'urgence, informe le CPE d'astreinte ; la famille est informée **sans détail médical** (34). Les traitements personnels du soir relèvent du protocole de l'infirmière (34) [NV sur la pratique de prise de traitement par un AED].
- Données de santé (RGPD art. 9) : jamais dans le 41 (R-IT6).

### 5.7 Protection des mineurs
- **Article 40 du code de procédure pénale** : tout agent public informe sans délai le procureur d'un crime ou d'un délit ; information préoccupante à la cellule départementale (CRIP) ; **119** joignable 24 h / 24 [V-copie, protocole éduscol de mars 2026, guide d'octobre 2026] → le 41 ne traite pas le signalement (socle 36) ; il **oriente** (bouton « préoccupation » qui ouvre le circuit du 36) et ne garde dans son cahier que le fait codé « transmis au CPE / à la direction ».
- **Honorabilité** : incapacité d'exercer après certaines condamnations (C. éduc. L. 911-5 ; L. 914-3 pour le privé) [V-copie] ; consultation du bulletin n° 2 et du FIJAIS au recrutement [V-copie] ; une proposition de loi sénatoriale de février 2026 sur une attestation d'honorabilité généralisée [V fiche Sénat ; **adoption non vérifiée**] → la vérification relève du 6 (PRS-143) ; **le 41 refuse d'affecter à un poste de nuit** un personnel non bénévole dont la vérification est expirée **si** l'établissement a activé cette garde (S-4111, désactivée par défaut, Q-IT8).
- **Téléphones** : interdiction au collège (C. éduc. L. 511-5, loi 2018-698) [V-copie] ; une circulaire du BO n° 27 du 2 juillet 2026 l'étendrait au lycée à la rentrée 2026 [V-copie, **à vérifier**] → le dépôt des téléphones la nuit est une règle du **règlement** ; ScolIA propose une **liste de dépôt** facultative (S-4112, désactivée par défaut), jamais un relevé de contenu.
- **Géolocalisation** des élèves : aucune doctrine trouvée [NV] → **jamais** dans ScolIA (R-IT41).

### 5.8 Hors contrat et registres
- **Registre des élèves internes** : C. éduc. **R. 442-1** (décret 2018-407) pour les établissements **hors contrat**, « en permanence à la disposition des autorités » (identité, adresse, responsables, dates d'entrée et de sortie de l'internat) [V-copie, vérifié par le socle 9, FX-63] → INT-235 : registre **daté et figé**, exportable, généré depuis les séjours (pas une saisie).

### 5.9 Ce qui reste à vérifier sur Légifrance ou au BO [NV]
1. Texte exact de l'**article R 33** (type R) : nombre et période des exercices de nuit.
2. Circulaire du **2 juillet 2026** (BO n° 27) sur le téléphone au lycée.
3. État de la **proposition de loi** sur l'honorabilité (adoptée ou non).
4. Circulaire **2000-106** (règlement intérieur) : place du règlement de l'internat.
5. Textes sur les **assistants d'éducation** en service de nuit (décret 2003-484 [NV]) et sur le logement par **nécessité absolue de service**.
6. Règlements de **remise d'ordre** d'hébergement des collectivités du premier client public.
7. Application de la **prime à l'internat** aux boursiers du privé sous contrat (probable par D. 531-42 [NV]).
8. Textes sur l'accueil d'**internes d'un autre établissement** (hébergement mutualisé, convention) [NV].
9. Doctrine CNIL sur la **conservation** des appels de nuit et des cahiers de liaison [NV] (durées proposées §21.3, **soumises au DPO**).

---

## 6. Frontières et branchements sur le cœur

### 6.1 Qui possède quoi

| Donnée | Propriétaire | Le 41 |
|---|---|---|
| Élève, état civil (sexe), scolarité datée, **régime** (externe, demi-pensionnaire, interne, interne externé) | **2** | lit `eleve::scolarite_a(eleve, date)` ; réagit à `eleve.regime_changed` |
| Demande d'internat, places, unités d'hébergement, vœux | **3** | fournit la capacité réelle et les lits libérés ; lit `inscription.validee` |
| Titulaires, interdictions de remise, résidences, **personnes autorisées** à récupérer | **4** | lit `personnes::autorises_remise(eleve, date)`, `personnes::destinataires(eleve, nature)` |
| Sites, **bâtiments** (`ent_batiment`, 337), calendrier, fermetures à effets différenciés, jours d'ouverture des services | **5** | rattache chambres et lits au bâtiment ; lit `structure::calendrier`, `referentiel.fermeture_exceptionnelle` |
| Personnels, fonctions, **postes de service** `internat_nuit`, planning, astreinte, honorabilité | **6** | déclare les besoins (effectif minimal par bâtiment), lit qui est de service |
| Règlement de l'internat signé, pièces | **7** | demande la signature ; lit l'état |
| Avis aux familles | **8** | émet des faits ; le 8 route |
| Absences et retards de cours ; **composant d'appel** ; études surveillées | **12** | réutilise le composant d'appel pour l'étude ; publie `internat.absence_nuit` que le 12 rattache au dossier d'assiduité |
| Incidents, punitions, sanctions, exclusion d'internat | **13** | transmet un fait de nuit ; lit l'exclusion d'internat datée (attendu) |
| Remise de l'élève, sorties individuelles de l'établissement, PPMS | **14** | fournit l'attendu de départ du vendredi et les présents de nuit ; lit la remise |
| Messagerie | **16** | fils d'équipe et familles ; plus de messagerie maison |
| Autorisations (annuelles et ponctuelles) | **18** | demande, lit l'état |
| Rendez-vous (créneaux d'installation) | **19** | utilise le moteur de créneaux (à défaut : le sien, INT-220) |
| Facturation | **24** (25, 26) | transmet nuits et remises (quantités) ; prime d'internat par le 26 |
| Santé : consignes, trousses, mouvements de premiers secours | **34** | lit les consignes de l'audience « internat », la trousse ; écrit un mouvement de nuit par le port du 34 |
| Besoins particuliers | **35** | lit l'aménagement d'hébergement (chambre accessible), jamais le motif |
| Préoccupation, protection de l'enfance | **36** | ouvre le circuit, ne garde rien |
| Stages | **39** | lit les périodes de stage (attendu) |
| **Dîner, petit-déjeuner** | **40** | fournit l'attendu et les absences annoncées ; lit les passages |
| Séjours scolaires | **42** | lit « en sortie » |
| Activités et transports | **43** | lit les inscriptions aux activités du soir organisées par le 43 ; transports de retour |
| Clés, casiers, état des lieux, interventions | **44** | lit, demande |
| **Hébergement : chambres, lits, affectations, séjours d'internat, nuits, appels de nuit, faits de présence, absences d'internat, sorties d'internat, liaison jour / nuit, événements de nuit, exercices de nuit, études et activités de l'internat, campagne d'installation, bilan aux familles** | **41** | possède |

### 6.2 Ports consommés
| Port | Socle | Usage |
|---|---|---|
| `eleve::scolarite_a(eleve, date)`, `eleve::regime_a(eleve, date)` | 2 | interne ou non à la date |
| `eleve::identite_affichee(eleve)` (nom d'usage, prénom, photo), `eleve::sexe_etat_civil(eleve)` | 2 | appel ; contrôle du public de la chambre (lecture **par le serveur** seulement) |
| `eleve::est_majeur(eleve, date)` | 2 | qui autorise |
| `personnes::titulaires(eleve, date)`, `personnes::autorises_remise(eleve, date)`, `personnes::interdits(eleve)` | 4 | autorisations, avis, remise, consigne « ne pas laisser sortir avec » |
| `structure::calendrier(etab, date)`, `structure::service_ouvert(etab, 'internat', date)` | 5 | nuits ouvertes |
| `personnels::de_service(etab, poste, date)`, `personnels::astreinte(etab, date)` | 6 | qui est de nuit, qui appeler |
| `autorisations::etat(eleve, type, date)` | 18 | sortie permise ? |
| `discipline::exclusion_internat(eleve, date)` | 13 | attendu |
| `stages::en_stage(eleve, date)` | 39 | attendu |
| `sorties::en_sortie(eleve, date, plage)` | 42 | attendu |
| `sante::consignes(eleve, 'internat', date)`, `sante::consignes_du_jour(etab, 'internat', date)`, `sante::trousse(eleve)`, `sante::mouvement_premiers_secours(...)` | 34 | nuit |
| `inclusion::amenagement_hebergement(eleve)` | 35 | chambre accessible |
| `restauration::passages(eleve, jour, service)` | 40 | repas du soir pris |
| `facturation::facturer_source(source)` | 24 | quantités |

### 6.3 Ports exposés par le 41
| Port | Consommateurs |
|---|---|
| `internat::attendu(etab, nuit)` → liste (élève, lit, statut attendu, motif codé d'absence) | 40 (effectif du soir et du matin), 12, 14, 34 (contacts en crise), 47 |
| `internat::present_maintenant(etab, batiment?)` → présents horodatés | 14 (PPMS, évacuation), 34 |
| `internat::heberge(eleve, date)` → lit, chambre, bâtiment (sans voisins) | 2 (fiche 360°), 14, 40 |
| `internat::nuits(eleve, mois)` → nuits dues, servies, non servies par cause | 24 |
| `internat::capacite(etab, annee)` → lits par public et accessibilité, libres | 3 |
| `internat::absences_annoncees(etab, date)` | 40, 12 |

### 6.4 Répercussions automatiques (le cœur bouge, l'internat suit)
| Événement | Effet dans le 41 | Confirmation |
|---|---|---|
| `eleve.regime_changed` vers interne | séjour proposé (dates, rythme par défaut) ; tâche « affecter un lit » au responsable | **N2** (le séjour est créé, le lit est proposé) |
| `eleve.regime_changed` quittant l'interne | fin du séjour et de l'affectation à la date d'effet ; sorties futures annulées ; nuits futures retirées ; 40 et 24 informés | auto, avec avis au responsable |
| `scolarite.depart_enregistre`, `eleve.decede` | idem, immédiat ; pour un décès : arrêt de tout avis, **aucun** rappel de clés ni de caution envoyé à la famille avant décision de la direction | auto |
| `personnes.interdiction_ajoutee` | consigne « ne pas laisser sortir avec » sur la fiche de nuit ; sorties futures avec cette personne **suspendues** et revues | auto |
| `personnes.titulaires_modifies` | autorisations en cours **revérifiées** (18) | auto |
| `sante.consigne_publiee` / `retiree` (audience internat) | fiche de nuit mise à jour | auto |
| `sorties.participant_parti` / `rentre` (42) | attendu recalculé | auto |
| `discipline.exclusion_internat` (13) | attendu ; nuits non servies cause « exclusion » (remise selon S-157 du 24) | auto |
| `referentiel.fermeture_exceptionnelle` avec service `internat` fermé | nuits retirées, familles prévenues par le 5 / 8 ; si internat maintenu (R-S1 du 5) : liste des internes présents | auto |
| `stage.periode_ouverte` (39) | attendu ; nuits non servies cause « stage » si l'élève ne dort pas à l'internat | auto |

---

## 7. Concepts et vocabulaire

### 7.1 Vocabulaire fixé [R]
| Terme | Définition | Table |
|---|---|---|
| **Bâtiment** | du socle 5 (`ent_batiment`) ; l'internat y ajoute ses **zones** (étage, aile) | `ent_batiment`, `ent_internat_zone` |
| **Zone** | étage ou aile d'un bâtiment, avec un **public** (filles, garçons, mixte pour les seules parties communes) et un ordre | `ent_internat_zone` |
| **Chambre** | unité fermée, dans une zone, avec son public, son accessibilité, son état (active, fermée temporairement, retirée) | `ent_internat_chambre` (existante, étendue) |
| **Lit** | place numérotée dans une chambre ; l'unité d'affectation | `ent_internat_lit` |
| **Séjour d'internat** | période où l'élève est interne (issue du régime du 2), avec son **rythme** (nuits de la semaine) | `ent_internat_sejour` |
| **Rythme** | ensemble des soirs de la semaine où l'interne est attendu (ex. dimanche à jeudi ; lundi à jeudi) ; versions datées | `ent_internat_rythme` |
| **Affectation** | lien daté élève → lit (existant, étendu) | `ent_internat_affectation` |
| **Nuit** | désignée par la **date du soir** (la nuit du 12 au 13 = nuit du 12), Europe/Paris | — (clé) |
| **Attendu** | vue calculée : qui doit dormir ici cette nuit, et pourquoi tel autre ne le doit pas | vue `ent_internat_attendu_v` |
| **Appel** | instance d'une prise de présence (soir, extinction, ronde, matin, étude) sur un **périmètre** (bâtiment, zone, liste), avec son état (ouvert, clos, corrigé) | `ent_internat_appel` |
| **Fait de présence** | pointage horodaté d'un élève dans un appel (présent, absent, excusé, en activité, à l'infirmerie, autre), en **ajout seul** | `ent_internat_presence_fait` (remplace la ligne écrasée de 066) |
| **Écart** | attendu non présent (ou présent non attendu) à la clôture ; ouvre un **cas** | `ent_internat_cas` |
| **Cas « non retrouvé »** | procédure minutée, ses étapes et leur heure | `ent_internat_cas_etape` |
| **Absence d'internat** | nuit(s) où l'élève ne dort pas à l'internat, **annoncée** (famille, majeur) ou **constatée**, avec statut (annoncée, acceptée, refusée, régularisée) | `ent_internat_absence` |
| **Sortie d'internat** | permission de quitter l'internat sur une plage (mercredi, soirée, nuit chez le correspondant, week-end, exceptionnelle), avec qui autorise, qui récupère, retour attendu | `ent_internat_sortie` |
| **Sortie de groupe** | sortie collective organisée par l'internat (cinéma, piscine), avec circuit direction → familles | `ent_internat_sortie_groupe` |
| **Correspondant** | personne désignée par les titulaires (4) pour accueillir ou récupérer l'interne, avec ses jours | `ent_internat_correspondant` (lien vers la personne du 4) |
| **Liaison** | fait codé transmis entre équipes (jour → nuit, nuit → jour) : « couché tôt, fatigué », « à rappeler à la famille », « clé perdue » ; jamais de diagnostic | `ent_internat_liaison` |
| **Événement de nuit** | incident, intrusion, alarme, malaise, dégradation, avec ses participants ; transmis au 13, 34 ou 44 selon la nature | `ent_internat_evenement` |
| **Exercice de nuit** | évacuation (ou confinement) de nuit, avec heure, bâtiment, durée, présents comptés, observations | `ent_internat_exercice` |
| **Étude, activité** | temps encadré du soir ; l'étude utilise le composant d'appel du 12 | `ent_internat_temps` |
| **Bilan de la semaine** | synthèse destinée aux titulaires (présences, sorties, faits positifs et à suivre), relue avant envoi | `ent_internat_bilan` |

### 7.2 Distinctions qui manquent aujourd'hui [R]
1. **Interne** (régime du 2) ≠ **hébergé cette nuit** (attendu) ≠ **présent maintenant** (fait).
2. **Absence d'internat annoncée** (ne dort pas ici, prévenu) ≠ **absence constatée** (attendu, pas là) ≠ **absence de cours** (12).
3. **Sortie d'internat** (individuelle, règlement de l'internat, 41) ≠ **sortie scolaire** (collective, 42) ≠ **sortie de l'établissement** en journée (14).
4. **Liaison** (fait éducatif codé, partagé dans l'équipe) ≠ **événement** (incident à qualifier, 13) ≠ **mouvement de santé** (34) ≠ **préoccupation** (36).
5. **Interne externé** (prend ses repas, ne dort pas) : régime du 2, sans séjour d'hébergement ; il est un « demi-pensionnaire du soir » pour le 40 [H].

---

## 8. Hébergement

### 8.1 Bâtiments, zones, chambres, lits (corrige RI-07, DI-15, DI-23)
- Le **bâtiment** vient du 5 ; l'internat y déclare ses **zones** (étages, ailes) avec leur **public** et leur ordre ; une zone peut être fermée (travaux) avec dates.
- La **chambre** porte : libellé, zone, public (`filles`, `garcons` ; `mixte` interdit pour une chambre, R-IT11), nombre de lits (1 à 8 [F Next.js : 1 à 8 en typé, 1 à 4 en JSON]), **accessibilité** (PMR, douche adaptée), équipements codés (lavabo, douche, bureau), **état** (active, fermée du … au …, retirée), notes d'entretien sans nom d'élève.
- Le **lit** est numéroté ; il est l'unité d'affectation (un lit = une place) ; un lit peut être **neutralisé** (cassé, réservé à un accueil d'urgence).
- Réduire une chambre sous son occupation est **refusé** (NI-02 repris) ; fermer une chambre occupée propose le **relogement** des occupants (INT-118).
- Plan visuel par zone (NI-03 repris) : lits libres / occupés, public, accessibilité ; **pas de nom d'élève** sur le plan affiché à un écran commun.

### 8.2 Séjours et rythmes
- Le séjour naît du **régime** daté du 2 (R-IT1) ; le responsable confirme le **rythme** : soirs attendus (par défaut : ceux du règlement, S-4100), **arrivée du dimanche soir** possible, nuits du mercredi (certains rentrent), nuits du vendredi et du samedi pour un internat ouvert le week-end.
- Le rythme a des **versions datées** (« à partir du 3 février, rentre le mercredi soir ») ; un changement ponctuel est une **absence** ou une **nuit exceptionnelle**, pas une nouvelle version.
- Un élève **interne externé** n'a pas de séjour d'hébergement (§7.2).

### 8.3 Affectations (corrige DI-15, RI-10, RI-04, RI-05)
- Affecter = lier un séjour à un **lit**, à une date de début ; une seule affectation ouverte par élève (repris) et **un seul élève par lit à une date** (contrainte d'exclusion, R-IT10) ; contrôle **atomique** (verrou sur le lit).
- Contrôles : régime interne à la date (R-IT1) ; **public** de la chambre (R-IT11) ; **accessibilité** si l'élève a un aménagement (35) ; fratrie et niveau signalés, pas bloquants ; un élève sous **interdiction de contact** avec un autre (13, mesure conservatoire) ne partage pas sa chambre (alerte bloquante sauf dérogation de la direction, R-IT12).
- **Changement de chambre** : fin de l'affectation à J, nouvelle à J (ou J+1), motif codé (demande de l'élève, conflit, travaux, santé — affiché « autre »), historisé.
- **Répartition de rentrée** : proposition sous contraintes (public, niveau, accessibilité, vœux de cohabitation mutuels, séparations demandées par le CPE, anciens et nouveaux), affichée avec ses raisons, **acceptée chambre par chambre** (INT-108, §18) ; les vœux des élèves sont recueillis par le formulaire du 18 (« avec qui aimerais-tu partager ta chambre ? »), **lus du seul responsable**.

### 8.4 Capacité pour les inscriptions (socle 3, R-I15)
- `internat::capacite` donne, par **unité d'hébergement** déclarée au 3 (filles, garçons, chambres adaptées) : lits actifs, lits occupés à la rentrée N+1 (réinscrits internes), lits libres ; le 3 ne compte jamais une place qui n'a pas de lit réel.

---

## 9. La nuit : attendus, appels et présence

### 9.1 L'attendu de la nuit (corrige DI-11, DI-16, RI-06)
Pour la nuit N (Europe/Paris), `internat::attendu` calcule, pour chaque élève dont le séjour couvre N et dont le rythme contient le jour de N :
- **retiré** (avec motif codé visible du surveillant) : absence annoncée et acceptée ; sortie (nuit chez le correspondant, week-end) ; séjour scolaire (42) ; stage avec hébergement ailleurs (39) ; exclusion d'internat (13) ; fermeture de l'internat (5) ; hospitalisation (« absent : santé », sans détail, posé par l'infirmière ou la direction) ;
- **ajouté** : nuit exceptionnelle (élève accueilli une nuit de plus, interne d'un jour pour un examen, élève éloigné le week-end).
L'attendu est **recalculé** à chaque fait ; la vue d'appel affiche l'heure de son dernier calcul.

### 9.2 Les appels (reprend NI-17 à NI-25 ; corrige DI-01, DI-13, DI-19, DI-23, RI-02)
- **Types** (réglables, S-4101) : **arrivée** (retour du week-end ou de la journée), **étude** (composant du 12), **soir** (l'appel principal), **extinction**, **ronde** (pointage rapide « vu »), **matin** (réveil, départ en cours).
- **Périmètre** : tout l'internat, un bâtiment, une zone, ou une **liste** (groupe de chambres confié à un surveillant) ; plusieurs surveillants font chacun leur périmètre **en même temps**, sans s'écraser (faits en ajout seul, R-IT13).
- **Ordre d'affichage** : par chambre (par défaut), par nom, par classe ; photo ; marques pré-remplies selon l'attendu (« en sortie jusqu'à dimanche 20 h », « absent annoncé par sa mère », « à l'infirmerie depuis 19 h 40 ») ; **consigne de nuit** en pastille (34) ; « ne pas laisser sortir avec » en pastille (4).
- **Marques** : présent ; absent ; excusé (absence couverte) ; en activité (sortie de groupe, activité du soir, en attente de retour) ; à l'infirmerie ; parti (retour famille dans la soirée, remise faite) ; non attendu présent (élève là alors qu'il devait être absent).
- **Clôture** : impossible tant qu'une marque « en activité » est en attente de retour (repris) ou qu'un attendu n'est pas pointé ; chaque absent **non couvert** ouvre un **cas** (§9.3).
- **Validation** : PDF et avis au CPE et à la direction concernée (formation collège / lycée lue du 5 ; **périmètre vérifié au serveur**, R-IT30) ; envoi en U1 (non urgent) si aucun écart, U3 si un cas est ouvert.
- **Correction après validation** (NI-19 repris) : nouveau fait motivé (code + texte court), visible dans l'historique, avis de correction aux mêmes destinataires.
- **Rappel** : si l'appel n'est pas validé à l'heure limite (S-4102, par défaut 21 h 30), avis au surveillant puis au CPE d'astreinte ; une seule fois par niveau (repris de NI-22, sans « lundi–jeudi » codé).
- **Hors ligne** (socle 11) : la liste de la nuit est chiffrée sur l'appareil à l'ouverture de l'appel ; les faits sont horodatés localement et synchronisés au retour du réseau ; l'effacement local a lieu à la validation + 12 h (R-IT15).

### 9.3 Écart, « non retrouvé » et fugue
- Un absent **non couvert** à la clôture ouvre un **cas** avec une **procédure** déclarée par l'établissement (modèle fourni, S-4103) ; modèle par défaut [R, à valider par la direction et le CPE, Q-IT2] :
  1. **0 min** : vérifications sur place (chambre, sanitaires, étude, infirmerie, autres zones) — case par lieu ;
  2. **+10 min** : appel du **CPE d'astreinte** (numéro lu au 6) ; message au fil d'équipe ;
  3. **+15 min** : appel des **titulaires** (numéros lus au 4, **personnes autorisées seulement**) ;
  4. **+30 min** : **direction** prévenue ; décision d'appeler le **17** ;
  5. issue : retrouvé (où, à quelle heure) ; rentré chez la famille (qui l'a confirmé) ; autorité saisie (heure, numéro d'intervention).
- Chaque étape est **horodatée** (« fait à 22 h 41 par Y. M. ») ; le cas ne se ferme que par une issue ; le lendemain, il est **transmis** à la vie scolaire (12 : absence de nuit) et, si nécessaire, au 13 (fait de discipline) ou au 36 (préoccupation).
- **Ce que le surveillant voit** : la procédure, les numéros à appeler, le minuteur ; **jamais** l'adresse de la famille protégée (4), jamais un numéro non autorisé.
- **Retour d'un élève absent** : marque « rentré à 23 h 10 », remise de qui (14 si la famille le ramène), cas clos.

### 9.4 Présence en temps réel
- `internat::present_maintenant` : dernier fait de chaque élève attendu, avec son heure ; utilisé pour l'**évacuation** (§12.1) et la **liste des contacts** d'un cas contagieux (34, sur demande de l'ARS).

---

## 10. Absences, sorties, autorisations

### 10.1 Absences d'internat annoncées (INT-03 de D12 ; corrige RI-06, DI-25)
- Un **titulaire** (ou l'**élève majeur**) annonce dans l'appli : « Léa ne dormira pas à l'internat mardi soir » (une ou plusieurs nuits), avec qui la récupère (personne autorisée du 4) et quand elle revient ; **heure limite** du jour (S-4104, par défaut 16 h) ; après l'heure limite, l'annonce reste possible mais part au CPE comme **demande** (« tardive »).
- Effets : attendu recalculé ; **dîner et petit-déjeuner annulés** au 40 (avec le délai de prévenance de la restauration ; en deçà, le repas reste dû selon le règlement du 24) ; avis au surveillant de service ; nuits « non servies » cause « absence de la famille » au 24 (remise selon le règlement).
- **Parents séparés** : chacun peut annoncer ; l'autre titulaire est **informé** (sans les coordonnées du premier, R-S17 du 34 transposé) ; un **désaccord connu** (4) bloque l'annonce et la renvoie au CPE (R-IT20).
- Refus possible par le CPE (examen le lendemain matin, sanction) avec motif lu par la famille.

### 10.2 Types de sorties [R, paramétrables, S-4105]
| Type | Exemple | Qui autorise | Qui récupère | Retour |
|---|---|---|---|---|
| **Mercredi après-midi** | sortie en ville de 13 h à 18 h | titulaires (autorisation **annuelle**, 18) + inscription du jour par l'élève | seul | heure fixée ; pointage d'arrivée |
| **Soirée** | entraînement sportif, conservatoire | titulaires (annuelle ou ponctuelle) + CPE | seul ou personne autorisée | heure fixée |
| **Nuit chez le correspondant** | | titulaires ; correspondant désigné | correspondant | lendemain, cours |
| **Week-end** | départ vendredi 16 h, retour dimanche 20 h ou lundi 8 h | règle de l'établissement (souvent implicite : rentrer chez soi) | personne autorisée ou seul (transports) | arrivée |
| **Exceptionnelle** | rendez-vous médical, obsèques | titulaires + CPE | personne autorisée | heure fixée |

- **Élève majeur** : il décide de ses sorties dans le cadre du règlement ; ses parents sont informés **si** le règlement le prévoit et s'il est à leur charge (circulaire 96-248 [V copie]) (R-IT22).
- **Autorisations annuelles** (18) signées à la rentrée : liste des sorties permises, horaires, personnes, moyens de transport ; révocables à tout moment par un titulaire.
- **Contrôle** au départ et au retour : pointage par le surveillant ou l'accueil (14) ; **interdiction de remise** en rouge ; retour en retard au-delà du seuil (S-4107) → **cas** (§9.3, procédure « retard de retour »).

### 10.3 Sorties de groupe (reprend NI-27 à NI-32 ; corrige DI-05, DI-06, DI-14)
- Créées par le responsable : activité, lieu, horaires, accompagnateurs (**personnels du 6**, ou bénévoles avec vérification), internes invités.
- Circuit : **direction** (accord ou refus, dans l'ENT) → **titulaires** de chaque élève (socle 18 : autorisation nominative, accord d'un titulaire, désaccord connu bloquant) ; un **refus n'exclut que l'élève concerné** (R-IT21) ; un majeur répond pour lui.
- **Accord reçu hors ligne** (téléphone, papier) : saisi par le personnel **avec preuve** (pièce déposée au 7, ou déclaration datée « accord oral de Mme X le 12/11 à 14 h, reçu par Y ») ; jamais un « autorisé par l'équipe » anonyme (corrige DI-06).
- Le jour J : appel de départ, marques « en activité », retour pointé ; récapitulatif hebdomadaire aux familles (repris de NI-32, via le 8).

### 10.4 Départ du vendredi et retour du dimanche
- **Attendu de départ** (vendredi, veille de vacances) : qui part, à quelle heure, avec qui (personne autorisée) ou seul (autorisation), par quel transport (43) ; l'accueil (14) voit la liste de remise ; un élève non récupéré à l'heure limite ouvre un **cas** (« non récupéré »).
- **Retour** : appel d'arrivée ; un absent non annoncé au retour du week-end → avis aux titulaires (U2) puis cas si non joint.

---

## 11. Vie quotidienne

### 11.1 Cahier de liaison jour / nuit (reprend NI-37 ; corrige RI-09)
- **Faits codés** d'un catalogue fermé (S-4108, modifiable dans le catalogue sans texte libre médical) : « fatigué, couché tôt », « conflit dans la chambre, réglé », « a demandé à parler au CPE », « travail non fait à l'étude », « comportement remarquable » (valorisation), « objet perdu », « à rappeler à la famille », « demande d'intervention technique », « exercice fait » ; avec un **texte court** (300 caractères) assorti de l'avertissement « pas de donnée de santé, pas de jugement sur la famille » et d'un **contrôle automatique** de mots de santé (liste de mots déclenchant une invite, pas un blocage, R-IT7).
- **Destinataires** : l'équipe d'internat, le CPE ; le **matin**, la vie scolaire voit les faits marqués « pour le jour » ; jamais les professeurs (sauf mention « pour le professeur principal » posée par le CPE, sans texte).
- **Visibilité famille** : un fait n'est visible des titulaires que s'il est **publié** dans le bilan de la semaine par le responsable (corrige RI-03 / RI-09 : « visible famille » n'est plus une case au moment de la saisie).

### 11.2 Événements de nuit (reprend NI-35, NI-36, NI-40 ; corrige DI-17)
- Nature codée : incident entre élèves, comportement, intrusion, alarme incendie, malaise (→ mouvement 34), dégradation (→ 44), objet dangereux, préoccupation (→ 36).
- Participants (élèves, rôle : auteur présumé, victime, témoin), adultes présents, lieu (zone, chambre), heure, mesures prises.
- **Transmission** : le fait est **proposé** au 13 (qui le qualifie en incident, punition, sanction) ; le 41 garde le fait et le lien ; pas de suppression, correction par fait ajouté.
- **Alerte d'urgence** (NI-40 repris) : bouton « alerte » avec trois niveaux ; « critique » prévient la direction et le CPE d'astreinte en U3 (8) avec la **position** (bâtiment, zone) et rien d'autre.

### 11.3 Études, activités, sorties de groupe (reprend NI-39, NI-41, NI-43)
- **Études du soir** : groupes (jour, heure, salle du 5, surveillant du 6, internes), **appel par le composant du 12** ; absence à l'étude → fait de liaison, pas une absence de cours.
- **Activités** du soir (atelier, sport, club) organisées par l'internat (ou par le 43, alors lues) : inscription, places, appel, participants marqués « en activité » à l'appel du soir.
- **Planning** visible des familles et des élèves (repris de NI-43) : études, activités, sorties de groupe, sans nom d'autres élèves.

---

## 12. Sécurité, santé, protection

### 12.1 Évacuation, confinement, exercices
- **Liste des présents de nuit** (INT-196) : en un geste, par bâtiment, avec l'heure du dernier pointage de chacun et les absents couverts (pour ne pas les chercher) ; **imprimable** et **hors ligne** ; utilisée par le responsable d'évacuation et les secours.
- **Exercice de nuit** (INT-197) : date, heure, bâtiment, type (évacuation, confinement), durée, **présents comptés au point de rassemblement** (appel spécial de type « rassemblement »), écarts, observations ; rappel : premier exercice dans le mois de la rentrée, deux exercices de nuit par an (S-4109, conforme au §5.5 sous réserve de §5.9.1) ; export pour le registre de sécurité.
- **PPMS** (14) : le 41 fournit les présents par zone de confinement.

### 12.2 Santé (34)
- **Fiche de nuit** de l'interne pour le surveillant : photo, chambre, **consignes de l'audience « internat »** (codes fonctionnels), lieu de la trousse, numéros d'urgence, titulaires **joignables et autorisés** ; **jamais** de diagnostic, de traitement nommé, de texte médical (R-IT6).
- **Mouvement de premiers secours** la nuit (34 SAN-224) : heure, mesure (codée : repos, appel du 15, appel du CPE, transfert), famille prévenue ; transmis à l'infirmière le matin ; l'élève passe « à l'infirmerie » ou « hospitalisé » dans l'attendu sans motif.
- **Retour de l'infirmerie** : l'infirmière marque « peut regagner l'internat » ; aucun motif.
- **Mode santé désactivé** (D-034 : pas d'hébergeur HDS ; N-1 non activé sans avis du DPO) : la fiche de nuit affiche « consignes de santé : non gérées dans ScolIA — se référer au classeur de l'infirmerie » ; **le 41 ne recueille aucune donnée de santé en remplacement** (R-IT6, CIT-30).

### 12.3 Protection des mineurs
- **Préoccupation** : bouton accessible à tout adulte de l'internat → circuit du 36 ; le 41 ne garde que « transmis le … à … » (aucun contenu).
- **Interdictions** (4) : « ne pas laisser sortir avec » sur la fiche de nuit et au contrôle de sortie ; **sans motif**.
- **Chambres** : aucune entrée du personnel en chambre occupée n'est tracée par ScolIA (pas de pointage « fouille ») ; une **fouille** relève du 13 (procédure disciplinaire) [H].
- **Honorabilité** du personnel de nuit : affaire du 6 ; garde facultative (S-4111).

---

## 13. Encadrement de nuit
- Le 41 **déclare les besoins** : pour chaque bâtiment ou zone et chaque soir du rythme, un **effectif minimal** (S-4106) et les fonctions acceptées (AED, maître d'internat, CPE logé, veilleur) ; il **lit** le planning du 6 (postes `internat_nuit`, PRS-133) et signale les **trous** (« mardi 14 : bâtiment B sans surveillant ») au responsable et au 6 (remplacement, socle 15).
- **CPE d'astreinte** : lu du 6 (astreinte datée) ; affiché sur l'écran du surveillant avec son numéro professionnel.
- **Passation** : à la prise de service, le surveillant voit la liaison du jour, les cas ouverts, les consignes nouvelles ; à la fin, il clôt sa **transmission** (faits du soir proposés automatiquement : écarts, événements, mouvements).
- **Planning des surveillants** de Next.js (NI-42, texte libre) : **non repris en texte** ; la reprise rapproche les noms des personnels du 6 (liste de cas non rapprochés pour le responsable).

---

## 14. Familles et élèves

### 14.1 Espace famille de l'internat
- Pour chaque **titulaire** (et pour l'élève) : bâtiment, chambre (sans le nom des voisins, S-4113), planning de la semaine, sorties et autorisations en cours, **absences annoncées** et leur statut, nuits du mois, **bilan de la semaine** publié, règlement signé, contacts de l'internat (fil du 16).
- **Annonces** : absence, sortie exceptionnelle, retard au retour (« le train a du retard ») ; **révocation** d'une autorisation.
- **Avis** (8) : absence non annoncée à l'appel (U2, SMS selon Q-N5), retard non justifié au retour, événement concernant l'élève (sans détail des autres élèves), bilan hebdomadaire (U0).

### 14.2 Correspondants
- Désignés par les titulaires dans le 4 (personne autorisée à récupérer, avec mention « correspondant d'internat », jours, adresse **non** affichée au surveillant) ; un correspondant reçoit l'avis de départ et peut annoncer un retard ; il ne voit **ni** le bilan, **ni** les faits de liaison.

### 14.3 L'interne
- Voit son lit, son planning, ses sorties, ses absences annoncées ; **s'inscrit** au mercredi et aux activités ; demande une sortie exceptionnelle (qui part aux titulaires s'il est mineur) ; **signale** une préoccupation (36) en confidentialité.
- **Majeur** : annonce lui-même ses absences et ses sorties dans les règles ; ses parents sont informés selon le règlement (R-IT22).

### 14.4 Bilan de la semaine (reprend NI-32 étendu ; SAN-18 de la matrice)
- Généré le jeudi soir (S-4114) : nuits, absences, retards au retour, faits **publiés** par le responsable (valorisations, points à suivre), sorties à venir ; **relu et publié** par le responsable (pas d'envoi automatique d'un texte non relu) ; l'IA peut proposer la rédaction (§18), jamais l'envoyer.

---

## 15. Rentrée et installation (reprend NI-44 à NI-48 ; corrige DI-07)
- **Campagne d'installation** : jours, horaires, durée de créneau, familles par créneau, créneaux fermés (repris) ; utilise le **moteur de créneaux du 19** dès qu'il existe, sinon le sien.
- **Réservation** par le **titulaire connecté**, pour **son** enfant interne inscrit (élève du 2, pas un nom tapé) ; à défaut de compte, **lien nominatif à usage unique** envoyé par le 8 ; confirmation par **POST** ; fichier `.ics` ; rappel la veille.
- **Le jour de l'installation** : la famille est accueillie ; l'affectation s'ouvre à la date de début ; état des lieux d'entrée de la chambre (44) ; remise des clés et du casier (44) ; signature du règlement si elle manque (7).
- **Avant** : liste de trousseau (document de l'établissement), autorisations annuelles (18), règlement (7), consignes de santé de nuit publiées (34), contacts d'urgence vérifiés (4).

---

## 16. Argent (socles 24, 25, 26)
- Le 41 transmet au 24 des **quantités** par élève et par mois : nuits **dues** (rythme × calendrier), nuits **non servies** par cause codée (fermeture, stage, séjour scolaire, exclusion, absence longue justifiée, absence de la famille), **nuits exceptionnelles** ; jamais un montant (règle du 24 §11).
- Forfait trimestriel ou mensuel, nuitées supplémentaires, week-ends : **tarifs et formules au 24** ; remise d'ordre au 24 (seuils du règlement financier, S-157 du 24) ; **prime à l'internat** : aide affectée du 26, imputée par le 24 sur la seule ligne internat du payeur qui a donné procuration (24 §10.5).
- **Pendant l'année de doublon** (D-034) : **mode suivi** — le 41 publie les quantités, le 24 calcule ce qu'il aurait facturé, **rien n'est émis ni prélevé** ; un état mensuel « nuits et remises calculées » est exporté pour comparaison avec Charlemagne (R-IT36).
- **Cautions** (clé, badge, chambre) : dépôts du 24 (article « caution »), restitution après l'état des lieux de sortie (44) ; le 41 ne connaît que « caution versée / restituée ».

---

## 17. Cas limites (catalogue)

| # | Situation | Traitement |
|---|---|---|
| CIT-01 | Élève passé externe au 1er février, mais encore en chambre le 2 | Fin de séjour à la date du régime ; tâche « libérer le lit » ; si l'élève dort encore là (délai accordé), **nuit exceptionnelle** datée par le CPE |
| CIT-02 | Changement de régime rétroactif saisi le 15 pour le 1er | Fin de séjour au 1er ; les faits de présence entre le 1er et le 15 restent (historique) ; nuits recalculées pour le 24 avec avis à la comptabilité |
| CIT-03 | Deux surveillants pointent le même élève au même moment (un « présent », un « absent ») | Deux faits ; la valeur courante est la plus récente ; **conflit signalé** aux deux et au CPE |
| CIT-04 | Appel commencé hors ligne, téléphone déchargé | Faits locaux perdus non synchronisés : l'appel reste ouvert ; un autre appareil reprend ; l'écart est visible (« 12 pointages non reçus ») |
| CIT-05 | Élève absent à l'appel, la mère dit l'avoir récupéré sans prévenir | Issue « rentré chez la famille, confirmé par Mme X à 22 h 05 » ; fait de liaison « annonce manquante » ; pas de remise faite : rappel du règlement à la famille par le CPE |
| CIT-06 | Le père, visé par une interdiction de remise, vient chercher sa fille vendredi | Non proposé à la remise (14) ; consigne en rouge « appeler la direction » ; la fille reste ; la mère est appelée |
| CIT-07 | Parents séparés : la mère annonce une absence, le père s'y oppose (désaccord connu) | Annonce bloquée, renvoyée au CPE ; l'élève reste attendu ; aucun parent ne voit les coordonnées de l'autre |
| CIT-08 | Garde alternée : week-ends un sur deux chez chaque parent | La sortie de week-end porte la **personne qui récupère** ; le calendrier de partage (4, ADM-79) propose le bon titulaire ; l'autre est informé |
| CIT-09 | Élève majeur qui sort le mercredi soir sans prévenir | Majeur : sortie permise si le règlement l'autorise ; sinon écart ; ses parents informés si le règlement le prévoit (R-IT22) |
| CIT-10 | Élève devient majeur en cours d'année | Recalcul : les autorisations deviennent les siennes ; ses parents gardent l'information si le règlement le prévoit ; avis au CPE |
| CIT-11 | Fratrie : frère et sœur internes | Chambres séparées par public ; une seule annonce peut couvrir les deux (case par enfant) |
| CIT-12 | Élève transgenre accompagné, chambre individuelle | Dérogation au public de la chambre décidée par la direction, code « dérogation accordée », **sans motif** affiché ; nom d'usage du 2 affiché à l'appel |
| CIT-13 | Élève en fauteuil, chambre adaptée au rez-de-chaussée occupée par un autre | Alerte : la chambre accessible est réservée en priorité ; relogement proposé ; l'exercice d'évacuation tient compte de l'assistance (consigne fonctionnelle du 35) |
| CIT-14 | Plus de demandes que de lits filles | Le 3 voit la capacité réelle ; liste d'attente au 3 ; aucun lit « virtuel » |
| CIT-15 | Chambre fermée pour dégât des eaux, deux occupants | Relogement proposé (lits libres compatibles) ; affectations changées à la date ; familles informées |
| CIT-16 | Élève à l'infirmerie à 21 h, l'infirmière est partie | Marque « à l'infirmerie » couverte par un mouvement en cours ; si l'infirmerie ferme, le surveillant reprend l'élève et pointe ; consigne d'urgence affichée |
| CIT-17 | Malaise à 2 h du matin | Mouvement de premiers secours (34) ; appel du 15 ; CPE d'astreinte ; famille prévenue sans détail ; transmission à l'infirmière |
| CIT-18 | Élève hospitalisé trois jours | Attendu « absent : santé » posé par l'infirmière ou la direction, sans motif ; nuits non servies cause « absence justifiée » ; dîner annulé |
| CIT-19 | Alarme incendie à 23 h | Appel « rassemblement » ; présents de nuit ; écarts ; fait « exercice / alerte réelle » |
| CIT-20 | Évacuation réelle : réseau coupé | Liste des présents hors ligne (dernière synchronisation) et **imprimée** chaque soir à la clôture (S-4110) |
| CIT-21 | Élève non retrouvé à 22 h 30 | Procédure §9.3 ; minuteur ; issue obligatoire ; transmission le matin |
| CIT-22 | Élève retrouvé dans une autre chambre | Issue « retrouvé (zone B) » ; fait de liaison ; éventuel fait au 13 |
| CIT-23 | Retour du dimanche : train supprimé (grève) | Annonce de retard par la famille ou l'élève ; attendu décalé ; pas de cas tant que le délai annoncé court |
| CIT-24 | Neige : transports interdits vendredi, internes restent le week-end | Nuits exceptionnelles en lot pour les internes concernés (R-S1 du 5 : internat maintenu) ; repas prévenus (40) ; encadrement demandé (6) |
| CIT-25 | Fermeture d'une semaine (crise) | Nuits retirées ; remise d'ordre en lot (24, CF-28) ; familles prévenues (5, 8) |
| CIT-26 | Voyage scolaire de 5 jours pour une partie des internes | Attendu « en sortie » lu au 42 ; dîners annulés ; nuits non servies cause « séjour scolaire » (le 24 décide) |
| CIT-27 | Stage de 4 semaines, l'élève dort chez lui | Attendu retiré ; nuits non servies cause « stage » (remise selon S-157 du 24) |
| CIT-28 | Exclusion d'internat de 3 jours, l'élève reste externe en journée | Attendu retiré (13) ; dîner annulé ; libellé famille « non hébergé du … au … », jamais « exclusion » dans un document financier |
| CIT-29 | Élève décédé | Arrêt immédiat des avis ; séjour clos ; chambre **non réaffectée** sans décision de la direction ; affaires remises à la famille selon la direction ; aucune relance (clé, caution) |
| CIT-30 | Santé désactivée (pas d'HDS) | Fiche de nuit sans consigne ; mention « classeur de l'infirmerie » ; **aucune** saisie de santé de remplacement |
| CIT-31 | Le surveillant écrit « crise d'asthme » dans la liaison | Invite de l'outil (mot de santé détecté) : « utiliser un mouvement de premiers secours » ; si maintenu, le texte est marqué « à revoir » pour le CPE qui peut le masquer (trace) |
| CIT-32 | Interne d'un autre établissement hébergé par convention | Q-IT4 : hors V1 ; recommandation : fiche « hébergé conventionné » minimale, lot I13 |
| CIT-33 | Correspondant qui ne répond plus | Les titulaires reçoivent un avis ; la sortie chez le correspondant est suspendue par le CPE |
| CIT-34 | Interne de 11 ans (6e) | Règlement par niveau (heures, sorties) : réglage par formation (S-4105) |
| CIT-35 | Élève d'un internat ouvert le week-end (pays étranger) | Rythme 7 nuits ; encadrement de week-end déclaré ; sortie du samedi avec le correspondant |
| CIT-36 | Erreur : l'appel validé marque présent un élève parti | Correction motivée ; avis de correction ; le cas est rouvert si l'élève est introuvable |
| CIT-37 | Un titulaire annonce une absence pour une nuit déjà passée | Refusé comme annonce ; possible comme **régularisation** demandée au CPE (motif), qui couvre l'écart |
| CIT-38 | Interne qui découche régulièrement le mercredi | Indicateur au CPE (répétition des écarts), jamais une sanction automatique |
| CIT-39 | Élève présent non attendu (devait partir en week-end) | Marque « non attendu présent » ; dîner et petit-déjeuner ajoutés au 40 si encore possible ; famille informée |
| CIT-40 | Deux appels du soir ouverts sur le même périmètre | Refusé : un seul appel ouvert par type, nuit et périmètre ; les surveillants se partagent des listes |
| CIT-41 | Surveillant de nuit absent, remplaçant non habilité au module | Délégation datée (D-017) ; le 6 / 15 désigne le remplaçant ; à défaut, le CPE d'astreinte reçoit l'alerte « bâtiment sans surveillant » |
| CIT-42 | Demande d'accès d'un parent aux faits de liaison le concernant (RGPD art. 15) | Accès aux données de l'élève : faits de liaison inclus, sans les noms d'autres élèves ni d'adultes en texte (pseudonymisés) |
| CIT-43 | Changement d'année : réinscrit interne | Séjour N+1 proposé ; affectation reconduite **si** le responsable la confirme (pas automatique : nouvelles chambres, nouvelles cohabitations) |
| CIT-44 | Nouvel interne arrivé le 10 novembre | Séjour et lit à la date ; consignes de nuit en attente de l'infirmière ; autorisations annuelles demandées (18) |
| CIT-45 | Élève transféré d'un bâtiment à l'autre en cours de soirée (conflit) | Nouvelle affectation datée du jour ; le pointage suit la nouvelle zone ; ancienne et nouvelle zones informées |
| CIT-46 | Sortie de groupe : un parent ne répond pas | L'élève reste « en attente » puis « non autorisé » à l'heure limite ; la sortie se fait sans lui ; l'autre titulaire peut répondre |
| CIT-47 | Bénévole accompagnateur de sortie de groupe | Personne du 6 (P21 bénévoles) avec vérification ; sans compte, il ne voit que la liste du jour |
| CIT-48 | Panne totale de ScolIA un soir | Liste des présents imprimée à 18 h (S-4110) ; appel papier ; saisie a posteriori comme faits datés « saisis le lendemain » |

---

## 18. Intelligence artificielle (contrat IA, `IA_COEUR.md` §5, §6.2)

### 18.1 Ce que l'IA fait [R]
- **Répartition de rentrée proposée** : sous contraintes déterministes (public, accessibilité, capacité) et souples (niveau, vœux mutuels, séparations demandées, anciens / nouveaux), avec l'**explication** de chaque choix ; acceptée **chambre par chambre** (N2). Le solveur est déterministe ; le modèle de langage ne fait qu'expliquer.
- **Résumé de la nuit** pour le CPE à 7 h : écarts, cas, événements, mouvements (sans contenu de santé), dans la portée de la personne (N0).
- **Brouillon du bilan de la semaine** à partir des faits **publiables** (N0 pour le brouillon, publication par la personne).
- **Recherche** : « qui est en sortie mercredi ? », « chambres libres filles en 2de ? » (N0).
- **Préparation** d'une sortie de groupe (liste des invités compatibles, accompagnateurs disponibles) (N1 brouillon).

### 18.2 Ce que l'IA ne fait pas [R]
- **Aucun accès** aux consignes de santé pour en déduire quoi que ce soit ; la fiche de nuit n'est pas lue par l'IA (R-IT6, Q-IA5 du cœur).
- Ne **décide** d'aucune sortie, d'aucune absence, d'aucun refus, d'aucune affectation, d'aucune clôture de cas ; ne **contacte** ni famille, ni autorité (N3 fermé : pas de préparation d'appel au 17).
- Ne note pas, ne qualifie pas un élève (« perturbateur ») ; ne produit aucun profil de comportement.
- Ne lit pas les préoccupations (36).

### 18.3 Capacités déclarées (registre `ent-ia`)
| Capacité | Sensibilité | Portée | Niveau |
|---|---|---|---|
| `internat.lire_attendu` | C1 | périmètre du lecteur (§21.1) | lecture |
| `internat.resumer_nuit` | C2 (faits de liaison, sans santé) | CPE, responsable, direction | lecture |
| `internat.proposer_repartition` | C1 | responsable | N2 |
| `internat.rediger_bilan` | C2 | responsable | N0 (brouillon) ; publication N3 par l'écran |
| `internat.preparer_sortie_groupe` | C1 | responsable | N1 |
| fiche de nuit, consignes, cas, préoccupations | — | — | **fermé à l'IA** |

### 18.4 Cadre
Étude juridique (règlement européen sur l'IA) avant mise en service des fonctions à modèle de langage (D-032) ; jeu d'évaluation du socle : 30 questions de répartition et de résumé, dont 6 pièges (demande de déduire une maladie, demande de classer les élèves « à problème »).

---
## 19. Actions métier (INT-100 à INT-249)

Colonnes : acteur · droit D-017 (§21.1) ; règles ; événement émis ; lot. ★ = réauthentification récente. Les actions INT-01 à INT-07 de `SOCLE2_DOMAINES.md` D12 correspondent à INT-116, INT-131, INT-150, INT-180, INT-162, INT-169, INT-159.

### 19.1 Hébergement
| INT | Action | Acteur · droit | Règles | Événement | Lot |
|---|---|---|---|---|---|
| INT-100 | Déclarer les zones d'un bâtiment du 5 (étage, aile, public, ordre) (NI-01) | RIN · `internat.gerer` | public filles / garçons / parties communes | `internat.zone_modifiee` | I1 |
| INT-101 | Fermer ou rouvrir une zone (travaux), avec dates | RIN · `internat.gerer` | occupants à reloger signalés | `internat.zone_fermee` | I1 |
| INT-102 | Créer une chambre (zone, public, lits, accessibilité, équipements) (NI-02) | RIN · `internat.gerer` | R-IT11 | `internat.chambre_creee` [F `internat.chambre`] | I1 |
| INT-103 | Créer une chambre « à la volée » depuis l'affectation (NI-16) | RIN · `internat.gerer` | mêmes contrôles | idem | I1 |
| INT-104 | Modifier une chambre ; réduire ses lits (refus sous l'occupation) | RIN · `internat.gerer` | NI-02 repris | `internat.chambre_modifiee` | I0 (modifier), I1 |
| INT-105 | Fermer temporairement ou retirer une chambre | RIN · `internat.gerer` | relogement proposé (INT-118) | `internat.chambre_fermee` | I1 |
| INT-106 | Neutraliser ou rétablir un lit (cassé, accueil d'urgence) | RIN · `internat.gerer` | | `internat.lit_modifie` | I1 |
| INT-107 | Voir le plan des lits par zone, occupation, surcapacités (NI-03) | RIN, CPE, DIR · `internat.lire` | plan d'écran commun sans nom (S-4113) | — | I1 |
| INT-108 | Obtenir une répartition proposée et l'accepter chambre par chambre | RIN · `internat.gerer` (IA N2) | §8.3, §18.1 | `internat.affectation_ouverte` (par chambre) | I8 |
| INT-109 | Publier la capacité réelle vers le 3 (unités d'hébergement) | SYS | R-I15 du 3 | `internat.capacite_mise_a_jour` | I8 |

### 19.2 Séjours et affectations
| INT | Action | Acteur · droit | Règles | Événement | Lot |
|---|---|---|---|---|---|
| INT-110 | Ouvrir le séjour d'un élève devenu interne (régime du 2) (NI-05, NI-11) | SYS → RIN confirme · `internat.gerer` | R-IT1 | `internat.sejour_ouvert` | I1 |
| INT-111 | Définir ou changer le rythme (soirs attendus), version datée | RIN · `internat.gerer` | R-IT4 | `internat.rythme_modifie` | I1 |
| INT-112 | Clore le séjour (changement de régime, départ, décès) (NI-07) | SYS ; RIN | R-IT3 | `internat.sejour_clos` | I0 (réaction), I1 |
| INT-113 | Ajouter une nuit exceptionnelle (élève accueilli une nuit de plus, week-end) | CPE, RIN · `internat.gerer` | motif codé | `internat.nuit_exceptionnelle` | I3 |
| INT-114 | Gérer l'interne externé (repas du soir sans hébergement) | ADF (au 2) ; SYS informe le 40 | §7.2 | — | I1 |
| INT-115 | Voir la section « internat » de la fiche 360° (lit, rythme, sorties en cours, absences du mois) | selon §21.2 | sans voisins de chambre | — | I1 |
| INT-116 | Affecter un lit daté (NI-16, INT-01 de D12) | RIN · `internat.gerer` | R-IT10, R-IT11, R-IT12 | `internat.affectation_ouverte` [F `internat.affectation`] | I0 (atomique), I1 |
| INT-117 | Changer de chambre (motif codé) | RIN · `internat.gerer` | historique | `internat.affectation_changee` | I1 |
| INT-118 | Reloger les occupants d'une chambre fermée (proposition, acceptation) | RIN · `internat.gerer` | lits compatibles | `internat.affectation_changee` | I1 |
| INT-119 | Clore une affectation (fin d'année, départ) | RIN · `internat.gerer` | | `internat.affectation_close` [F] | I0 |

### 19.3 Préparer la nuit
| INT | Action | Acteur · droit | Règles | Événement | Lot |
|---|---|---|---|---|---|
| INT-120 | Voir l'attendu de la nuit (avec les retirés et leurs motifs codés) | SUR, CPE, RIN · `internat.lire` | §9.1 | — | I2 |
| INT-121 | Voir le tableau de la nuit (attendus, présents, écarts, cas, effectif par bâtiment) | CPE, RIN, DIR · `internat.lire` | | — | I2 |
| INT-122 | Imprimer la liste des présents prévus à 18 h (secours en cas de panne) | SYS ; SUR | S-4110 | — | I5 |
| INT-123 | Préparer la liste hors ligne sur l'appareil | SUR · `internat.appel` | R-IT15 | — | I10 |
| INT-124 | Prendre son service (voir la liaison, les cas ouverts, les consignes nouvelles) | SUR · `internat.appel` | §13 | `internat.service_pris` | I6 |
| INT-125 | Se voir confier une liste (groupe de chambres) | RIN, CPE · `internat.gerer` | | — | I2 |
| INT-126 | Voir le CPE d'astreinte et son numéro professionnel | SUR · `internat.lire` | lu au 6 | — | I6 |
| INT-127 | Voir la fiche de nuit d'un interne (photo, chambre, consignes, trousse, contacts autorisés) | SUR, CPE · `internat.lire` | R-IT6, lecture journalisée | `internat.fiche_nuit_lue` (journal) | I5 |
| INT-128 | Voir les interdictions de remise et de contact en pastille, sans motif | SUR, ACC · `internat.lire` | 4 | — | I3 |
| INT-129 | Recalcul de l'attendu à chaque fait (absence, sortie, voyage, stage, exclusion, fermeture) (NI-25) | SYS | §9.1 | `internat.attendu_modifie` | I2 |

### 19.4 Appels et présence
| INT | Action | Acteur · droit | Règles | Événement | Lot |
|---|---|---|---|---|---|
| INT-130 | Ouvrir un appel (type, périmètre) (NI-17, NI-24) | SUR · `internat.appel` | un seul ouvert par type, nuit, périmètre (CIT-40) | `internat.appel_ouvert` | I2 |
| INT-131 | Pointer un élève (présent, absent, excusé, en activité, à l'infirmerie, parti, non attendu présent) (INT-02 de D12) | SUR · `internat.appel` | R-IT13, R-IT14 | `internat.presence_pointee` [F `internat.presence`] | I0 (ajout seul), I2 |
| INT-132 | Pointer une chambre entière en un geste (« tous présents ») | SUR · `internat.appel` | chaque élève reçoit son fait | idem | I2 |
| INT-133 | Marquer « en activité », avec heure de retour attendue | SUR · `internat.appel` | | idem | I2 |
| INT-134 | Pointer le retour d'activité | SUR · `internat.appel` | | idem | I2 |
| INT-135 | Voir le signal « absent en cours aujourd'hui » (NI-20) | SUR, CPE · `internat.lire` | lu au 12 par `eleve_id` | — | I2 |
| INT-136 | Clore un appel (contrôle des attendus, ouverture des cas) (NI-18) | SUR · `internat.appel` | R-IT16 | `internat.appel_clos` | I2 |
| INT-137 | Valider l'appel : PDF, avis au CPE et à la direction concernée (NI-18) | SUR · `internat.appel.valider` | R-IT30 | `internat.appel_valide` | I2 |
| INT-138 | Corriger un appel validé (motif, historique, avis de correction) (NI-19) | SUR, CPE · `internat.appel.valider` | R-IT17 | `internat.appel_corrige` | I2 |
| INT-139 | Consulter l'historique des appels et leurs PDF (NI-21) | CPE, RIN, DIR · `internat.lire` | durée §21.3 | — | I2 |
| INT-140 | Rappel : appel non validé à l'heure limite (NI-22) | SYS | S-4102 ; calendrier du 5 | `internat.appel_en_retard` | I2 |
| INT-141 | Synchroniser les pointages faits hors ligne | SUR (appareil) | horodatage local, conflits signalés (CIT-03) | `internat.presence_pointee` | I10 |
| INT-142 | Faire une ronde (« vu » par chambre) | SUR · `internat.appel` | type `ronde` | idem | I2 |
| INT-143 | Faire l'appel du matin (réveil, départ en cours, malades) | SUR · `internat.appel` | absences du matin transmises au 12 | idem | I2 |
| INT-144 | Faire l'appel d'arrivée (retour de week-end ou de journée) | SUR · `internat.appel` | absent non annoncé → avis U2 (INT-205) | idem | I2 |
| INT-145 | Fournir au 40 l'effectif du dîner et du petit-déjeuner (NI-34) | SYS | heure limite du 40 | `internat.effectif_repas` | I2 |
| INT-146 | Ouvrir un cas « non retrouvé », « non rentré », « non récupéré » | SYS ; SUR | §9.3, R-IT18 | `internat.cas_ouvert` | I2 |
| INT-147 | Suivre la procédure d'un cas (cocher les étapes horodatées, appeler) | SUR, CPE · `internat.appel` | numéros autorisés seulement | `internat.cas_etape` | I2 |
| INT-148 | Clore un cas par une issue | SUR, CPE · `internat.appel` ; DIR pour « autorité saisie » | issue obligatoire | `internat.cas_clos` | I2 |
| INT-149 | Transmettre la nuit au matin (absences de nuit au 12, faits au 13, mouvements au 34) | SYS ; SUR valide | §13 | `internat.absence_nuit`, `internat.transmission` | I2 |

### 19.5 Absences d'internat et sorties individuelles
| INT | Action | Acteur · droit | Règles | Événement | Lot |
|---|---|---|---|---|---|
| INT-150 | Annoncer une absence d'internat (une ou plusieurs nuits, qui récupère, retour) (INT-03 de D12) | PAR (titulaire), MAJ | R-IT19, R-IT20 | `internat.absence_annoncee` | I3 |
| INT-151 | Accepter ou refuser une absence annoncée (motif lu par la famille) | CPE · `internat.sorties.decider` | | `internat.absence_decidee` | I3 |
| INT-152 | Traiter une annonce tardive (après l'heure limite) | CPE · `internat.sorties.decider` | repas selon délai du 40 | idem | I3 |
| INT-153 | Régulariser un écart a posteriori (motif) | CPE · `internat.sorties.decider` | CIT-37 | `internat.absence_regularisee` | I3 |
| INT-154 | S'inscrire à la sortie du mercredi | ELV, MAJ | autorisation annuelle valide (18) | `internat.sortie_demandee` | I3 |
| INT-155 | Demander ou autoriser une sortie de soirée | PAR, MAJ ; CPE | | `internat.sortie_decidee` | I3 |
| INT-156 | Autoriser une nuit chez le correspondant | PAR ; CPE | correspondant désigné (4) | idem | I3 |
| INT-157 | Établir l'attendu de départ du vendredi ou de veille de vacances (qui, quand, avec qui, transport) | SYS ; CPE ajuste | | `internat.depart_prevu` | I3 |
| INT-158 | Annoncer un retard de retour (train) | PAR, MAJ, COR | attendu décalé (CIT-23) | `internat.retard_annonce` | I3 |
| INT-159 | Contrôler un départ (pointage, remise du 14) (INT-07 de D12) | SUR, ACC · `internat.appel` | interdictions bloquantes | `internat.sortie_partie` | I3 |
| INT-160 | Contrôler un retour | SUR, ACC · `internat.appel` | | `internat.sortie_rentree` | I3 |
| INT-161 | Ouvrir un cas « retard de retour » au-delà du seuil | SYS | S-4107 | `internat.cas_ouvert` | I3 |
| INT-162 | Demander les autorisations annuelles de sortie (18) (INT-05 de D12) | RIN · `internat.gerer` | modèle par formation | `internat.autorisations_demandees` | I3 |
| INT-163 | Révoquer une autorisation (titulaire) | PAR | effet immédiat, sorties futures suspendues | `internat.autorisation_revoquee` | I3 |
| INT-164 | Décider soi-même d'une sortie (majeur) | MAJ | R-IT22 | `internat.sortie_decidee` | I3 |
| INT-165 | Bloquer une demande en cas de désaccord connu des titulaires | SYS | R-IT20 | `internat.absence_bloquee` | I3 |
| INT-166 | Ouvrir un cas « non récupéré » (vendredi) | SYS | heure limite S-4107 | `internat.cas_ouvert` | I3 |
| INT-167 | Fournir à l'accueil la liste de remise du départ | SYS → 14 | personnes autorisées du jour | — | I3 |
| INT-168 | Voir les correspondants et leurs jours (sans adresse) | SUR, CPE · `internat.lire` | 4 | — | I3 |
| INT-169 | Demander une sortie exceptionnelle (élève) qui part aux titulaires (INT-06 de D12) | ELV | | `internat.sortie_demandee` | I3 |

### 19.6 Sorties de groupe
| INT | Action | Acteur · droit | Règles | Événement | Lot |
|---|---|---|---|---|---|
| INT-170 | Créer une sortie de groupe (activité, lieu, horaires, accompagnateurs du 6, invités) (NI-27) | RIN · `internat.gerer` | accompagnateurs = personnes du 6 | `internat.sortie_groupe_creee` | I4 |
| INT-171 | Décider (direction) (NI-28) | DIR · `internat.sorties.decider` | | `internat.sortie_groupe_decidee` | I4 |
| INT-172 | Recueillir l'autorisation des titulaires (18) (NI-29) | SYS ; PAR | R-IT21 | `autorisations.*` | I4 |
| INT-173 | Annuler une sortie ; relancer (NI-30) | RIN | avis aux familles | `internat.sortie_groupe_annulee` | I4 |
| INT-174 | Enregistrer un accord reçu hors ligne **avec preuve** (corrige DI-06) | RIN, CPE · `internat.sorties.decider` | pièce ou déclaration datée | `autorisations.accord_hors_ligne` | I4 |
| INT-175 | Voir les autorisés du jour, de la semaine (NI-31) | SUR, CPE · `internat.lire` | | — | I4 |
| INT-176 | Faire l'appel de départ et de retour d'une sortie de groupe | accompagnateur · `internat.appel` | marques « en activité » | `internat.presence_pointee` | I4 |
| INT-177 | Dupliquer une sortie de groupe récurrente (piscine du mardi) | RIN | | idem 170 | I4 |
| INT-178 | Inviter les seuls internes compatibles (autorisation annuelle, pas d'exclusion, pas d'absence) | SYS | | — | I4 |
| INT-179 | Clore la sortie (bilan court, faits de liaison) | RIN | | `internat.sortie_groupe_close` | I4 |

### 19.7 Liaison et événements
| INT | Action | Acteur · droit | Règles | Événement | Lot |
|---|---|---|---|---|---|
| INT-180 | Écrire un fait de liaison codé (NI-37 ; INT-04 de D12) | SUR, CPE, VS · `internat.liaison.ecrire` | R-IT7 | `internat.liaison_ecrite` [F `internat.suivi`] | I0 (borne), I4 |
| INT-181 | Lire la liaison de son périmètre | SUR, CPE, RIN, VS (faits « pour le jour ») · `internat.liaison.lire` | R-IT8 | — (lecture journalisée) | I0 (fermeture RI-03), I4 |
| INT-182 | Marquer un fait « à revoir » ou le masquer (avec trace) | CPE · `internat.gerer` | CIT-31 | `internat.liaison_masquee` | I4 |
| INT-183 | Signaler un fait au professeur principal (sans texte) | CPE | | `internat.signal_pp` | I4 |
| INT-184 | Voir la chronologie de liaison d'un interne | CPE, RIN · `internat.liaison.lire` | | — | I4 |
| INT-185 | Déclarer un événement de nuit (nature, participants et rôles, lieu, heure, mesures) (NI-35) | SUR, CPE · `internat.evenement.declarer` | transmis au 13, 34, 44 | `internat.evenement_declare` | I4 |
| INT-186 | Compléter un événement (fait ajouté ; pas de suppression) (NI-36) | auteur, CPE | | `internat.evenement_complete` | I4 |
| INT-187 | Transmettre un événement au 13 pour qualification | CPE | | `discipline.incident_propose` | I4 |
| INT-188 | Demander une intervention technique sur une chambre (44) | SUR, RIN | sans nom d'élève | `ressources.intervention_demandee` | I4 |
| INT-189 | Ouvrir une préoccupation (36) | tout adulte de l'internat | aucun contenu gardé | `suivi.preoccupation_ouverte` | I4 |

### 19.8 Temps du soir, alertes, sécurité
| INT | Action | Acteur · droit | Règles | Événement | Lot |
|---|---|---|---|---|---|
| INT-190 | Créer une activité du soir (date, titre, places, encadrant) (NI-39) | RIN, SUR · `internat.gerer` | | `internat.temps_cree` | I4 |
| INT-191 | Inscrire des internes à une activité ; ils passent « en activité » à l'appel | RIN ; ELV | | `internat.temps_inscription` | I4 |
| INT-192 | Organiser les études du soir (groupes, salle, surveillant) (NI-41) | RIN · `internat.gerer` | appel du 12 | `internat.temps_cree` | I4 |
| INT-193 | Faire l'appel d'étude (composant du 12) | SUR | | `appel.*` | I4 |
| INT-194 | Signaler une absence à l'étude (fait de liaison) | SUR | | `internat.liaison_ecrite` | I4 |
| INT-195 | Lancer une alerte (info, urgent, critique) avec la position (NI-40) | SUR · `internat.appel` | critique → U3 direction et astreinte | `internat.alerte` | I5 |
| INT-196 | Produire la liste des présents de nuit (par bâtiment, horodatée, hors ligne, imprimable) | SUR, CPE, DIR · `internat.securite` | §12.1 | `internat.presents_produits` (journal) | I5 |
| INT-197 | Enregistrer un exercice de nuit (type, durée, présents au rassemblement, observations) | RIN, DIR · `internat.securite` | §5.5 | `internat.exercice_enregistre` | I5 |
| INT-198 | Rappeler l'exercice à faire (mois de rentrée, deux de nuit par an) | SYS | S-4109 | `internat.exercice_a_faire` | I5 |
| INT-199 | Fournir au 14 les présents par zone de confinement (PPMS) | SYS → 14 | | — | I5 |

### 19.9 Familles et élèves
| INT | Action | Acteur · droit | Règles | Événement | Lot |
|---|---|---|---|---|---|
| INT-200 | Voir le planning de la semaine (études, activités, sorties de groupe) (NI-43) | PAR, ELV, MAJ | sans autres élèves | — | I7 |
| INT-201 | Recevoir le récapitulatif hebdomadaire des sorties à venir (NI-32) | PAR | 8 | `internat.recap_hebdomadaire` | I7 |
| INT-202 | Voir l'espace famille (chambre, nuits du mois, absences annoncées, autorisations, règlement) | PAR, ELV | §14.1 | — | I7 |
| INT-203 | Générer le brouillon du bilan de la semaine | SYS ; IA propose | §14.4 | — | I7 |
| INT-204 | Relire et publier le bilan aux titulaires | RIN, CPE · `internat.bilan.publier` | N3 par l'écran | `internat.bilan_hebdomadaire` | I7 |
| INT-205 | Être prévenu d'une absence non annoncée à l'appel ou au retour | PAR | U2 ; SMS selon Q-N5 | `internat.absence_soir` | I7 |
| INT-206 | Être prévenu d'un événement concernant son enfant (sans détail des autres) | PAR | U1 ou U2 selon nature | `internat.evenement_famille` | I7 |
| INT-207 | Voir son lit, ses sorties, ses absences annoncées (élève) | ELV | | — | I7 |
| INT-208 | S'inscrire à une activité (élève) | ELV | places | idem 191 | I7 |
| INT-209 | Écrire à l'équipe de l'internat (fil du 16) | PAR, ELV | | `messagerie.*` | I7 |

### 19.10 Encadrement et santé
| INT | Action | Acteur · droit | Règles | Événement | Lot |
|---|---|---|---|---|---|
| INT-210 | Déclarer les besoins d'encadrement par bâtiment et soir (NI-42) | RIN · `internat.parametrer` | S-4106 | `internat.besoin_modifie` | I6 |
| INT-211 | Voir les trous du planning de nuit et les signaler au 6 / 15 | RIN, CPE · `internat.lire` | | `internat.poste_non_pourvu` | I6 |
| INT-212 | Voir le planning de nuit (lu du 6) | SUR, RIN | | — | I6 |
| INT-213 | Clore sa transmission de fin de service | SUR · `internat.appel` | faits proposés | `internat.transmission` | I6 |
| INT-214 | Alerter « bâtiment sans surveillant » à l'heure d'ouverture | SYS | CIT-41 | `internat.sans_surveillant` | I6 |
| INT-215 | Voir les consignes de nuit du jour (audience « internat ») | SUR · `internat.lire` | 34, R-IT6 | — | I5 |
| INT-216 | Saisir un mouvement de premiers secours la nuit (port du 34) | SUR | sans texte médical | `sante.mouvement_*` | I5 |
| INT-217 | Recevoir « peut regagner l'internat » de l'infirmière | SYS ← 34 | | — | I5 |
| INT-218 | Poser « absent : santé » (hospitalisation) sans motif | INF, DIR | | `internat.absence_sante` | I5 |
| INT-219 | Refuser un poste de nuit à un personnel dont la vérification d'honorabilité est expirée (si activé) | SYS | S-4111 | — | I6 |

### 19.11 Rentrée, installation, fin d'année
| INT | Action | Acteur · droit | Règles | Événement | Lot |
|---|---|---|---|---|---|
| INT-220 | Configurer la campagne d'installation (NI-44) | RIN · `internat.gerer` | 19 si disponible | `internat.installation_configuree` | I8 |
| INT-221 | Afficher les créneaux restants (NI-45) | PAR | sans donnée personnelle | — | I8 |
| INT-222 | Réserver un créneau pour son enfant inscrit interne (NI-46) | PAR (connecté ou lien nominatif) | élève du 2 | `internat.installation_reservee` | I8 |
| INT-223 | Confirmer par POST ; recevoir `.ics` et rappel (NI-47) | PAR | corrige DI-07 | `internat.installation_confirmee` | I8 |
| INT-224 | Annuler ou déplacer une réservation (NI-48) | PAR, RIN | | `internat.installation_annulee` | I8 |
| INT-225 | Recueillir les vœux de cohabitation (18), lus du seul responsable | ELV ; RIN | | — | I8 |
| INT-226 | Reconduire ou non l'affectation d'un réinscrit (confirmation explicite) | RIN | CIT-43 | `internat.affectation_ouverte` | I8 |
| INT-227 | Faire l'état des lieux d'entrée et de sortie de chambre (44) | RIN, AGT | | `ressources.etat_des_lieux` | I8 |
| INT-228 | Remettre et reprendre clés et casiers (44) | RIN | caution au 24 | `ressources.pret_*` | I8 |
| INT-229 | Clore l'année : fin des séjours et affectations, archivage | SYS ; RIN confirme | §21.3 | `internat.annee_close` | I8 |

### 19.12 Pilotage, exports, conformité
| INT | Action | Acteur · droit | Règles | Événement | Lot |
|---|---|---|---|---|---|
| INT-230 | Tableau de bord (internes, occupation, écarts du mois, cas, exercices) (NI-14) | DIR, RIN · `internat.lire` | | — | I12 |
| INT-231 | Indicateurs au 47 (taux d'occupation, écarts, délais de clôture des cas, agrégés) | SYS | sans nom | — | I12 |
| INT-232 | Exporter les statistiques (NI-15) | DIR, RIN · `internat.exporter` | agrégées | `internat.export` (journal) | I12 |
| INT-233 | Publier au 24 les nuits dues, servies, non servies par cause | SYS | R-IT35 | `internat.nuits_mois` | I9 |
| INT-234 | Exporter l'état mensuel « nuits et remises calculées » pendant le doublon | CPT · `internat.exporter` | R-IT36 | `internat.export` | I9 |
| INT-235 | Produire le registre des internes (hors contrat, R. 442-1), daté et figé | DIR · `internat.exporter` ★ | §5.8 | `internat.registre_produit` | I12 |
| INT-236 | Répondre à une demande d'accès (RGPD) pour un interne | DIR (délégué) | CIT-42 | journal | I12 |
| INT-237 | Purger selon les durées (appels, liaison, cas, événements) | SYS | §21.3 | `internat.purge` | I12 |
| INT-238 | Consulter le journal des lectures sensibles (fiche de nuit, liaison, présents) | DIR, délégué à la protection des données | | — | I12 |
| INT-239 | Exporter le PDF d'un appel validé | CPE, DIR · `internat.lire` | | journal | I2 |

### 19.13 Système, paramétrage, reprise, IA
| INT | Action | Acteur · droit | Règles | Événement | Lot |
|---|---|---|---|---|---|
| INT-240 | Réagir à `eleve.regime_changed` (ouvrir ou clore le séjour) | SYS | §6.4 | — | I0 |
| INT-241 | Réagir au départ et à la radiation | SYS | | — | I0 |
| INT-242 | Réagir au décès (arrêt immédiat, chambre non réaffectée sans décision) | SYS | CIT-29 | — | I1 |
| INT-243 | Reprendre les données de Next.js (deux générations) (NI-12, NI-13) | opérateur · Florian | §26 | rapport de reprise | I11 |
| INT-244 | Comparer chaque soir l'appel ScolIA et l'appel Next.js pendant le parallèle | SYS | §26 | rapport d'écart | I11 |
| INT-245 | Paramétrer l'internat (types d'appel, heures limites, types de sorties, rythme par défaut) | DIR, RIN · `internat.parametrer` | §28 | `internat.parametres_modifies` | I1 |
| INT-246 | Modifier le modèle de procédure « non retrouvé » | DIR · `internat.parametrer` ★ | Q-IT2 | idem | I2 |
| INT-247 | Modifier le catalogue des faits de liaison | DIR, RIN · `internat.parametrer` | pas de catégorie de santé | idem | I4 |
| INT-248 | IA : proposer la répartition (§18) | RIN | N2 | — | I13 |
| INT-249 | IA : résumer la nuit, rédiger le brouillon du bilan | CPE, RIN | N0 | — | I13 |

---

## 20. Règles transverses (R-IT1 à R-IT41) [R]

| # | Règle | Lot |
|---|---|---|
| R-IT1 | **L'hébergement naît du régime.** Un séjour n'existe que si le régime du 2 est « interne » sur la période ; jamais de création d'interne à la main dans le 41. | I0 |
| R-IT2 | **Régime codé** : `externe`, `demi_pensionnaire`, `interne`, `interne_externe` (table de correspondance des trois vocabulaires existants, RI-04 ; codes BCN / Siècle à la reprise) ; jamais de texte libre interprété. | I0 |
| R-IT3 | **Pas de suppression** d'un séjour, d'une affectation, d'un fait de présence, d'un cas, d'un événement : fin datée ou fait correctif (E6). | I0 |
| R-IT4 | **Rythme versionné** : un changement de rythme est une nouvelle version datée ; un écart ponctuel est une absence ou une nuit exceptionnelle. | I1 |
| R-IT5 | **Nuit = date du soir, Europe/Paris**, y compris aux changements d'heure ; toutes les bornes de jour sont calculées dans le fuseau de l'établissement (corrige DI-16). | I0 |
| R-IT6 | **Aucune donnée de santé dans le 41** : ni allergie, ni traitement, ni diagnostic, ni texte médical ; la nuit lit les **consignes** du 34 ; santé désactivée = aucune consigne affichée, **aucune saisie de remplacement** (corrige DI-02). | I0 |
| R-IT7 | **Liaison codée** : catalogue fermé + texte court (300 caractères) ; mots de santé détectés → invite vers le mouvement du 34 ; jamais de jugement sur la famille. | I4 |
| R-IT8 | **Lecture de la liaison et des événements** réservée à l'équipe de l'internat, au CPE et à la direction ; la vie scolaire de jour ne voit que les faits marqués « pour le jour » ; **aucun professeur** (corrige RI-03). | I0 |
| R-IT9 | **Visibilité famille** : un fait n'est vu des titulaires que publié dans le bilan, par une personne. | I4 |
| R-IT10 | **Un lit, un élève, une date** (contrainte d'exclusion) ; **une affectation ouverte par élève** ; capacité contrôlée sous verrou (corrige RI-10, DI-15). | I0 |
| R-IT11 | **Public de la chambre** (filles, garçons) contrôlé contre le sexe d'état civil au serveur ; dérogation par la direction, code sans motif. Aucune chambre mixte. | I1 |
| R-IT12 | **Interdiction de contact** (mesure du 13) entre deux élèves : pas dans la même chambre, sauf dérogation de la direction. | I1 |
| R-IT13 | **Faits de présence en ajout seul** ; valeur courante = fait le plus récent ; conflits simultanés signalés (corrige RI-02, DI-13). | I0 |
| R-IT14 | **On ne pointe que l'attendu** (et « non attendu présent ») du périmètre de l'appel ; un élève hors périmètre ou hors séjour est refusé (corrige DI-19). | I2 |
| R-IT15 | **Hors ligne** : liste chiffrée, clé liée à la session, effacement à la validation + 12 h ou à la déconnexion ; pas de données de santé ni d'adresses dans la liste locale (seulement consignes codées). | I10 |
| R-IT16 | **Clôture** impossible avec un attendu non pointé ou une activité en attente ; chaque absent non couvert ouvre un cas. | I2 |
| R-IT17 | **Correction après validation** : fait motivé, historique visible, avis de correction aux destinataires de la validation. | I2 |
| R-IT18 | **Un cas ne se ferme que par une issue**, avec l'heure de chaque étape ; il est transmis au matin. | I2 |
| R-IT19 | **Annonce d'absence** par un titulaire (ou le majeur) seulement ; heure limite ; au-delà, demande au CPE. | I3 |
| R-IT20 | **Désaccord connu des titulaires** (4) : l'annonce ou la sortie est bloquée et renvoyée au CPE ; chacun ne voit que ce qui le concerne. | I3 |
| R-IT21 | **Sortie de groupe** : un refus n'exclut que l'élève concerné (corrige DI-14). | I4 |
| R-IT22 | **Élève majeur** : il décide de ses sorties et annonces dans le cadre du règlement ; ses parents sont informés seulement si le règlement le prévoit et qu'il est à leur charge (S-4115). | I3 |
| R-IT23 | **Remise** : un départ accompagné se fait à une personne **autorisée aujourd'hui** (14, 4) ; une interdiction bloque toujours. | I3 |
| R-IT24 | **Accord hors ligne** = pièce ou déclaration datée nommant l'auteur ; jamais un « autorisé par l'équipe » anonyme. | I4 |
| R-IT25 | **Accompagnateurs** = personnes du 6 (personnels ou bénévoles vérifiés) ; aucun nom en texte libre. | I4 |
| R-IT26 | **Le surveillant ne voit que les numéros autorisés** (titulaires joignables, personnes autorisées, astreinte), jamais une adresse protégée (4). | I2 |
| R-IT27 | **Présents de nuit** : produits en un geste, journalisés, disponibles hors ligne et imprimés à la clôture du soir. | I5 |
| R-IT28 | **Exercices de nuit** tracés et rappelés ; l'export ne remplace pas le registre de sécurité. | I5 |
| R-IT29 | **Préoccupation** : le 41 ouvre le circuit du 36 et ne garde aucun contenu. | I4 |
| R-IT30 | **Périmètre vérifié au serveur** : direction d'une formation (collège / lycée) ne valide et ne lit que ses élèves ; aucun filtre d'affichage tenant lieu de droit (corrige DI-01). | I2 |
| R-IT31 | **Un seul appel ouvert** par type, nuit et périmètre ; plusieurs surveillants se partagent des listes, pas un document. | I2 |
| R-IT32 | **Plan sans nom** sur un écran commun ; nom des voisins de chambre non montré aux familles (S-4113). | I1 |
| R-IT33 | **Aucune décision par l'IA** ; aucune lecture de santé ni de préoccupation par l'IA ; aucun profil de comportement. | I13 |
| R-IT34 | **Avis graduels** : U1 pour l'appel sans écart, U2 pour une absence non annoncée, U3 pour un cas non résolu à +30 min ou une alerte critique ; la direction n'est réveillée que pour U3. | I2 |
| R-IT35 | **Quantités, pas montants** vers le 24 ; causes de non-service codées, jamais le motif (« exclusion » ne figure sur aucun document financier). | I9 |
| R-IT36 | **Doublon** (D-034) : mode suivi — rien n'est facturé par ScolIA ; état mensuel exporté pour comparaison avec Charlemagne. | I9 |
| R-IT37 | **Un seul propriétaire par donnée pendant la bascule** : une nuit est tenue soit dans Next.js, soit dans ScolIA, jamais dans les deux (§26). | I11 |
| R-IT38 | **Chaque lecture sensible est journalisée** (fiche de nuit, liaison, présents, registre) (E7). | I0 |
| R-IT39 | **Pas d'action par lien sans compte** autre que celles du 18 et du 8 (lien nominatif, à usage unique, expirant, POST) (corrige DI-05, DI-07). | I3 |
| R-IT40 | **Calendrier réel** : nuits, rappels et exercices suivent le calendrier et les fermetures du 5 ; jamais de jours codés en dur. | I2 |
| R-IT41 | **Ni vidéo, ni géolocalisation, ni relevé de téléphone** dans le socle ; le dépôt des téléphones est une liste facultative sans contenu. | — |

---

## 21. Droits et minimisation

### 21.1 Catalogue d'actions proposé (D-017) [R]
| Droit | Qui par défaut (rôles modèles) | Portée |
|---|---|---|
| `internat.lire` | Direction, CPE, responsable d'internat, surveillant d'internat | périmètre : formation (direction), bâtiments confiés (surveillant), tout l'internat (CPE, responsable) |
| `internat.appel` | surveillant d'internat, CPE, responsable | périmètre de l'appel |
| `internat.appel.valider` | surveillant d'internat, CPE | idem |
| `internat.gerer` (existant) | responsable d'internat, CPE, direction | internat |
| `internat.sorties.decider` | CPE, responsable, direction | internat |
| `internat.liaison.ecrire` | surveillant, CPE, vie scolaire | internes |
| `internat.liaison.lire` | surveillant, CPE, responsable, direction ; vie scolaire : faits « pour le jour » | idem |
| `internat.evenement.declarer` | surveillant, CPE | |
| `internat.securite` | direction, responsable, CPE, surveillant (présents seulement) | |
| `internat.bilan.publier` | responsable, CPE | |
| `internat.parametrer` | direction, responsable | |
| `internat.exporter` ★ | direction, comptabilité (états de nuits) | |
| Famille, élève | implicites par le lien (4) et la scolarité (2) | soi ou son enfant |

Le rôle **Professeur**, **Infirmerie**, **Psychologue**, **Comptabilité** n'a **aucun** droit de lecture de la liaison, des événements, des cas ou de l'appel (corrige RI-03, DI-03) ; l'infirmière agit par le 34.

### 21.2 Ce que voit chacun (projections) [R]
| Information | Surveillant | CPE / responsable | Direction | Vie scolaire de jour | Professeur | Restauration | Comptabilité | Titulaires | Élève |
|---|---|---|---|---|---|---|---|---|---|
| Lit, chambre | ✔ (périmètre) | ✔ | ✔ | ✔ | — | — | — | ✔ | ✔ |
| Voisins de chambre | ✔ | ✔ | ✔ | — | — | — | — | — (S-4113) | ✔ (il les voit…) |
| Attendu, motif codé d'absence | ✔ | ✔ | ✔ | absences de nuit | — | effectif agrégé | — | son enfant | soi |
| Faits de présence | ✔ | ✔ | ✔ | absences de nuit | — | — | — | absences non annoncées | soi |
| Consignes de nuit (34) | ✔ | ✔ | — | — | — | — | — | ✔ | ✔ |
| Liaison | ✔ | ✔ | ✔ | « pour le jour » | — (signal PP sans texte) | — | — | publiée seulement | publiée seulement |
| Événements | ✔ (siens, périmètre) | ✔ | ✔ | — | — | — | — | ceux de son enfant, sans autres noms | — |
| Cas | ✔ (en cours) | ✔ | ✔ | transmis | — | — | — | issue le concernant | — |
| Nuits du mois, causes codées | — | ✔ | ✔ | — | — | — | ✔ | ✔ | — |
| Vœux de cohabitation | — | responsable | — | — | — | — | — | — | soi |

### 21.3 Conservation [R, durées soumises au DPO, Q-T0-08]
| Donnée | Durée active | Ensuite |
|---|---|---|
| Faits de présence, appels, PDF | année scolaire + 3 mois (T0 proposait 3 mois d'archive pour Next.js) | supprimés ; agrégats conservés |
| Cas « non retrouvé » et leurs étapes | année + 1 an | archivés si contentieux, sinon supprimés |
| Liaison | année scolaire | supprimée (sauf faits transmis au 13 / 36, qui suivent leur socle) |
| Événements | 1 an, puis selon le 13 | |
| Absences et sorties, autorisations | année + 1 an (preuve) | supprimées |
| Exercices de nuit | 5 ans [H : durée de vie du registre] | |
| Séjours, affectations | durée de la scolarité + 1 an | registre R. 442-1 : selon le texte (hors contrat) |
| Nuits transmises au 24 | suivent la pièce comptable (10 ans) | |
| Journal des lectures | 1 an [R] | |

---

## 22. Événements

**Émis** (outbox, en transaction ; charges minimales : identifiants, dates, codes ; jamais de texte libre de liaison) : `internat.zone_modifiee`, `internat.chambre_creee` / `_modifiee` / `_fermee`, `internat.lit_modifie`, `internat.sejour_ouvert` / `_clos`, `internat.rythme_modifie`, `internat.affectation_ouverte` / `_changee` / `_close`, `internat.nuit_exceptionnelle`, `internat.attendu_modifie`, `internat.appel_ouvert` / `_clos` / `_valide` / `_corrige` / `_en_retard`, `internat.presence_pointee`, `internat.effectif_repas`, `internat.cas_ouvert` / `_etape` / `_clos`, `internat.absence_nuit`, `internat.transmission`, `internat.absence_annoncee` / `_decidee` / `_regularisee` / `_bloquee` / `_sante`, `internat.sortie_demandee` / `_decidee` / `_partie` / `_rentree`, `internat.retard_annonce`, `internat.depart_prevu`, `internat.autorisations_demandees`, `internat.autorisation_revoquee`, `internat.sortie_groupe_*`, `internat.liaison_ecrite` / `_masquee`, `internat.signal_pp`, `internat.evenement_declare` / `_complete` / `_famille`, `internat.temps_cree` / `_inscription`, `internat.alerte`, `internat.presents_produits`, `internat.exercice_enregistre` / `_a_faire`, `internat.recap_hebdomadaire`, `internat.bilan_hebdomadaire`, `internat.absence_soir`, `internat.besoin_modifie`, `internat.poste_non_pourvu`, `internat.sans_surveillant`, `internat.installation_*`, `internat.capacite_mise_a_jour`, `internat.nuits_mois`, `internat.registre_produit`, `internat.annee_close`, `internat.export`, `internat.purge`, `internat.parametres_modifies`.
Les quatre événements existants (`internat.chambre`, `internat.affectation`, `internat.presence`, `internat.suivi`) restent émis pendant I0–I1 pour compatibilité, puis sont remplacés.

**Consommés** : `eleve.regime_changed`, `scolarite.opened` / `closed`, `scolarite.depart_enregistre`, `eleve.decede`, `eleve.majeur` (2) ; `inscription.validee`, `inscription.desistement` (3) ; `personnes.interdiction_ajoutee` / `_retiree`, `personnes.titulaires_modifies`, `personnes.autorises_modifies` (4) ; `referentiel.fermeture_exceptionnelle`, `structure.calendrier_modifie` (5) ; `planning.*`, `astreinte.*` (6) ; `autorisations.*` (18) ; `discipline.exclusion_internat`, `discipline.interdiction_contact` (13) ; `sante.consigne_publiee` / `_retiree`, `sante.mouvement_clos` (34) ; `stage.periode_ouverte` / `close` (39) ; `sorties.participant_parti` / `rentre` / `desiste` (42) ; `restauration.passage` (40, pour le dîner pris).

---

## 23. Écrans et mobile

**Web (Leptos, `FRONTEND_LEPTOS.md`, `DESIGN_SYSTEM.md`)**
1. **Ce soir** (tuiles Bento) : attendus / présents / écarts par bâtiment ; cas ouverts avec minuteur ; activités en cours ; trous d'encadrement ; dîner (effectif) ; dernière synchronisation.
2. **Appel** : liste dense par chambre avec photo, pastilles (consigne, interdiction, sortie, activité), gestes rapides (tap = présent ; glisser = absent ; maintien = autres marques) ; barre « 12 / 48 pointés » ; bouton « tous présents » par chambre.
3. **Cas** : procédure pas à pas, numéros autorisés à composer, minuteur, issue.
4. **Hébergement** : plan des zones, lits, accessibilité ; glisser-déposer pour affecter (contrôles en direct) ; répartition proposée avec ses raisons.
5. **Absences et sorties** : demandes à traiter, autorisés du jour, départs du vendredi, retours attendus.
6. **Liaison** : fil de la semaine par interne et par équipe, filtres par code.
7. **Sécurité** : présents de nuit, exercices, rappel.
8. **Paramètres** : types d'appel, heures, sorties, rythmes, procédure, catalogue de liaison, besoins d'encadrement.
9. **Famille** (espace famille) : chambre, semaine, nuits, annoncer une absence, autorisations, bilans, contacts.

**Mobile (appli unique, D-003 ; usage quotidien)** : surveillant — **appel hors ligne**, cas, fiche de nuit, alerte, liaison, présents de nuit ; CPE — tableau de la nuit, décisions, cas, astreinte ; famille — **annoncer une absence en deux gestes**, autoriser une sortie, retard de retour, bilan ; élève — planning, sorties, inscription au mercredi et aux activités. Les tâches longues (répartition, campagne d'installation, paramètres, exports) restent sur le web.

---

## 24. Modèle de données (sans JSON, D-036 ; E1–E14)

Règles communes à toutes les tables : `id UUID PRIMARY KEY` généré par l'application (E3) ; `etablissement_id UUID NOT NULL` avec le contrôle « non nul uuid » (E1) ; `UNIQUE (etablissement_id, id)` et **FK composites** `(etablissement_id, x_id)` (E2) ; `cree_le TIMESTAMPTZ NOT NULL DEFAULT now()` et `modifie_le` si modifiable (E6) ; tables de faits en **ajout seul** par déclencheur (fonction commune `ent_ajout_seul()` ou celle du socle) ; RLS par `ent_rls_poser` (E13) ; aucune colonne `jsonb`, `json`, `bytea`, aucun tableau d'identifiants (E5, E10) ; classement au registre des tables (E12) ; aucun nom d'élève dans un nom d'objet ni un journal applicatif (E14). Les PDF d'appel sont des **objets** du registre `ent_objet` (500), référencés par `objet_id`.

```
-- Hébergement (I1 ; 2822)
ent_internat_zone (id, etablissement_id, batiment_id → ent_batiment (337), code, libelle,
  public CHECK (filles|garcons|commun), ordre SMALLINT, ferme_du DATE NULL, ferme_au DATE NULL,
  cree_le, modifie_le, UNIQUE (etablissement_id, batiment_id, code))
ent_internat_chambre (existante 066, étendue) + zone_id, public CHECK (filles|garcons),
  accessible_pmr BOOLEAN, douche BOOLEAN, lavabo BOOLEAN, bureau BOOLEAN,
  etat CHECK (active|fermee|retiree), ferme_du, ferme_au, modifie_le, UNIQUE (etablissement_id, id)
  -- colonnes 066 conservées (batiment TEXT, genre) en lecture seule jusqu'au retrait (I14)
ent_internat_lit (id, etablissement_id, chambre_id, numero SMALLINT, neutralise_du DATE NULL,
  neutralise_au DATE NULL, motif_code NULL, cree_le, modifie_le, UNIQUE (etablissement_id, chambre_id, numero))

-- Séjours et affectations (I1 ; 2823)
ent_internat_sejour (id, etablissement_id, eleve_id → ent_eleve, du DATE, au DATE NULL,
  origine CHECK (regime|reprise|manuel_direction), ferme_motif_code NULL, cree_le, modifie_le,
  EXCLUDE (etablissement_id, eleve_id, daterange(du, au, '[]') WITH &&))
ent_internat_rythme (id, etablissement_id, sejour_id, du DATE, au DATE NULL,
  dimanche BOOLEAN, lundi BOOLEAN, mardi BOOLEAN, mercredi BOOLEAN, jeudi BOOLEAN,
  vendredi BOOLEAN, samedi BOOLEAN, cree_le)                       -- versions, ajout seul
ent_internat_affectation (existante 066, étendue) + sejour_id, lit_id, motif_code NULL,
  modifie_le ; FK composites ; EXCLUDE (etablissement_id, lit_id, daterange(debut, fin, '[]') WITH &&)
ent_internat_derogation (id, etablissement_id, eleve_id, nature CHECK (public_chambre|interdiction_contact),
  decide_par_user_id, decide_le, du, au NULL, cree_le)            -- jamais de motif en clair
ent_internat_nuit_exceptionnelle (id, etablissement_id, eleve_id, nuit DATE, motif_code, par_user_id, cree_le)

-- Nuit, appels, présence (I0 pour la présence ; I2 ; 2820, 2824)
ent_internat_appel (id, etablissement_id, nuit DATE, type CHECK (arrivee|etude|soir|extinction|ronde|matin|rassemblement),
  perimetre CHECK (internat|batiment|zone|liste), batiment_id NULL, zone_id NULL, liste_id NULL,
  ouvert_par, ouvert_le, clos_le NULL, valide_par NULL, valide_le NULL, objet_pdf_id NULL, cree_le, modifie_le,
  UNIQUE (etablissement_id, nuit, type, perimetre, batiment_id, zone_id, liste_id) -- un ouvert par périmètre
ent_internat_liste (id, etablissement_id, libelle, cree_le) ; ent_internat_liste_chambre (liste_id, chambre_id, etablissement_id)
ent_internat_presence_fait (id, etablissement_id, appel_id, eleve_id, marque CHECK (present|absent|excuse|activite|
  infirmerie|parti|non_attendu_present|vu), retour_attendu TIMESTAMPTZ NULL, a TIMESTAMPTZ (heure du geste,
  éventuellement locale hors ligne), recu_le TIMESTAMPTZ DEFAULT now(), par_user_id, appareil_id NULL,
  correction_de UUID NULL, motif_correction_code NULL, motif_correction_texte NULL CHECK (≤ 300), cree_le)  -- ajout seul
  -- remplace ent_internat_presence (066), conservée en lecture seule, reprise en faits (2820)
ent_internat_cas (id, etablissement_id, eleve_id, nuit, nature CHECK (non_retrouve|non_rentre|non_recupere|retard_retour),
  ouvert_le, procedure_version_id, issue CHECK (retrouve|rentre_famille|autorite_saisie|annule_erreur) NULL,
  issue_le NULL, issue_par NULL, cree_le, modifie_le)
ent_internat_cas_etape (id, etablissement_id, cas_id, etape_id, fait_le, par_user_id, note CHECK (≤ 300) NULL, cree_le) -- ajout seul
ent_internat_procedure_version (id, etablissement_id, nature, version, publiee_le, publiee_par)
ent_internat_procedure_etape (id, etablissement_id, procedure_version_id, rang, code, libelle, delai_minutes, appelle CHECK (astreinte|titulaires|direction|secours|aucun))

-- Absences et sorties (I3 ; 2825)
ent_internat_absence (id, etablissement_id, eleve_id, du_nuit DATE, au_nuit DATE, annonce_par_user_id,
  annonce_qualite CHECK (titulaire|eleve_majeur|personnel), recupere_par_personne_id NULL, retour_prevu TIMESTAMPTZ NULL,
  statut CHECK (annoncee|acceptee|refusee|bloquee|regularisee|annulee), tardive BOOLEAN, decide_par NULL, decide_le NULL,
  motif_refus_texte NULL CHECK (≤ 300), cree_le, modifie_le)
ent_internat_sortie_type (id, etablissement_id, code, libelle, autorisation CHECK (annuelle|ponctuelle|aucune),
  decideur CHECK (titulaires|cpe|direction|titulaires_et_cpe), seul_permis BOOLEAN, heure_retour TIME NULL, actif, cree_le, modifie_le)
ent_internat_sortie (id, etablissement_id, eleve_id, type_id, debut TIMESTAMPTZ, fin_prevue TIMESTAMPTZ,
  avec_personne_id NULL, seul BOOLEAN, autorisation_id NULL (18), statut CHECK (demandee|autorisee|refusee|partie|rentree|annulee),
  cree_le, modifie_le, EXCLUDE chevauchement par élève)
ent_internat_sortie_fait (id, etablissement_id, sortie_id, nature CHECK (depart|retour|retard_annonce|revocation), a, par_user_id NULL, cree_le) -- ajout seul
ent_internat_correspondant (id, etablissement_id, eleve_id, personne_id (4), du, au NULL, lundi … dimanche BOOLEAN, cree_le, modifie_le)

-- Sorties de groupe (I4 ; 2826)
ent_internat_sortie_groupe (id, etablissement_id, titre, lieu, debut, fin, statut CHECK (brouillon|soumise|acceptee|refusee|annulee|close),
  decide_par NULL, decide_le NULL, cree_le, modifie_le)
ent_internat_sortie_groupe_encadrant (etablissement_id, sortie_groupe_id, personnel_id (6))
ent_internat_sortie_groupe_participant (id, etablissement_id, sortie_groupe_id, eleve_id,
  etat CHECK (invite|en_attente|autorise|refuse|non_autorise|parti|rentre), autorisation_id NULL, cree_le, modifie_le)

-- Vie quotidienne (I4 ; 2827)
ent_internat_liaison_code (id, etablissement_id NULL pour le catalogue fourni, code, libelle, pour_le_jour BOOLEAN, actif)
  -- table de référence : etablissement_id NULL = catalogue ScolIA (classé « globale_reference », E12)
ent_internat_liaison (id, etablissement_id, eleve_id, code_id, texte NULL CHECK (≤ 300), nuit DATE, par_user_id,
  masque_le NULL, masque_par NULL, a_revoir BOOLEAN, publie_bilan_id NULL, cree_le)  -- ajout seul sauf masquage
ent_internat_evenement (id, etablissement_id, nature CHECK (incident|comportement|intrusion|alarme|malaise|degradation|objet_dangereux),
  zone_id NULL, chambre_id NULL, a TIMESTAMPTZ, declare_par, mesures_code NULL, transmis_13_le NULL, cree_le)
ent_internat_evenement_participant (etablissement_id, evenement_id, eleve_id NULL, personnel_id NULL, role CHECK (auteur_presume|victime|temoin|present))
ent_internat_evenement_complement (id, etablissement_id, evenement_id, texte CHECK (≤ 1000), par_user_id, cree_le) -- ajout seul
ent_internat_temps (id, etablissement_id, nature CHECK (etude|activite), libelle, debut, fin, salle_id NULL (5), encadrant_personnel_id NULL, places NULL, cree_le, modifie_le)
ent_internat_temps_participant (etablissement_id, temps_id, eleve_id, inscrit_le)

-- Sécurité, encadrement (I5, I6 ; 2828)
ent_internat_exercice (id, etablissement_id, batiment_id, type CHECK (evacuation|confinement|alerte_reelle), a TIMESTAMPTZ,
  de_nuit BOOLEAN, duree_secondes INTEGER, presents_comptes INTEGER, attendus INTEGER, appel_id NULL, observations CHECK (≤ 2000), saisi_par, cree_le)
ent_internat_besoin_encadrement (id, etablissement_id, batiment_id NULL, zone_id NULL, jour_semaine SMALLINT, effectif_min SMALLINT, du, au NULL, cree_le)
ent_internat_lecture_sensible (id, etablissement_id, user_id, objet CHECK (fiche_nuit|liaison|presents|registre|cas), eleve_id NULL, a) -- ajout seul (ou ent_journal_modif, E7)

-- Familles, rentrée (I7, I8 ; 2829–2830)
ent_internat_bilan (id, etablissement_id, eleve_id, semaine_du DATE, etat CHECK (brouillon|publie), publie_par NULL, publie_le NULL, cree_le, modifie_le)
ent_internat_bilan_ligne (id, etablissement_id, bilan_id, nature CHECK (nuits|absence|retard|fait_publie|sortie_a_venir), liaison_id NULL, texte CHECK (≤ 500), rang)
  -- copie publiée : instantané figé par ligne (texte), jamais relu comme source (exception D-036 non nécessaire : colonnes typées)
ent_internat_installation_campagne (id, etablissement_id, annee_id, titre, lieu, duree_creneau_min SMALLINT, familles_par_creneau SMALLINT, ouverte_du, ouverte_au, cree_le, modifie_le)
ent_internat_installation_jour (id, etablissement_id, campagne_id, jour DATE, ouverture TIME, fermeture TIME)
ent_internat_installation_creneau_ferme (etablissement_id, campagne_id, debut TIMESTAMPTZ)
ent_internat_installation_reservation (id, etablissement_id, campagne_id, eleve_id, debut TIMESTAMPTZ, par_user_id NULL,
  statut CHECK (confirmee|annulee), cree_le, modifie_le, UNIQUE (etablissement_id, campagne_id, eleve_id) WHERE statut = 'confirmee')
ent_internat_voeu_cohabitation (id, etablissement_id, annee_id, eleve_id, avec_eleve_id, cree_le)  -- lu du seul responsable

-- Argent (I9 ; 2831)
ent_internat_nuit_mois (id, etablissement_id, eleve_id, mois DATE, dues SMALLINT, servies SMALLINT, exceptionnelles SMALLINT, calcule_le, version) -- état transmis au 24
ent_internat_nuit_non_servie (etablissement_id, nuit_mois_id, cause CHECK (fermeture|stage|sejour_scolaire|exclusion|absence_justifiee|absence_famille|sante), nombre SMALLINT)

-- Reprise (I11 ; 2832)
ent_internat_reprise_correspondance (id, etablissement_id, source CHECK (next_json|next_type), cle_source TEXT, cible_table, cible_id UUID, lot_id, cree_le)

-- Paramètres (I1 ; 2821) : colonnes typées, pas de clé-valeur
ent_internat_parametres (etablissement_id PRIMARY KEY, heure_limite_annonce TIME, heure_limite_appel TIME,
  seuil_retard_minutes SMALLINT, jour_bilan SMALLINT, voisins_visibles_familles BOOLEAN, depot_telephones BOOLEAN,
  garde_honorabilite BOOLEAN, impression_presents BOOLEAN, majeur_info_parents CHECK (toujours|si_a_charge|jamais), modifie_le, modifie_par)
ent_internat_type_appel_actif (etablissement_id, type, actif, heure_prevue TIME NULL)
```

**Vues calculées** : `ent_internat_attendu_v` (nuit × élève × statut × motif codé), `ent_internat_presence_courante_v` (dernier fait par élève et appel), `ent_internat_occupation_v`. Aucune n'est matérialisée par défaut ; l'attendu du soir est figé dans l'appel à l'ouverture (copie des élèves attendus en lignes typées `ent_internat_appel_attendu`) pour savoir **ce qu'on croyait** à l'heure de l'appel.

**Registre (E12)** : toutes les tables `etablissement` / export `simple` ; `ent_internat_liaison`, `_evenement*`, `_cas*`, `_lecture_sensible` : export `double` (validation) ; `ent_internat_liaison_code` à `etablissement_id NULL` : `globale_reference` ; **E9** : aucun effet de bord propre (les avis passent par l'outbox du cœur, déjà classée « à neutraliser »).

---

## 25. Lots

### 25.1 Lot I0 — correctifs urgents de l'existant Rust (avant tout autre lot)
| # | Correctif | Défaut | Migration |
|---|---|---|---|
| 1 | Fermer la lecture des suivis privés à tout rôle hors `internat.gerer` et direction ; journaliser la lecture ; test API (professeur, infirmerie, comptabilité refusés) | RI-03 | — (code) |
| 2 | Présence en **ajout seul** : table de faits `ent_internat_presence_fait`, reprise des lignes 066 en faits, l'ancienne table en lecture seule ; jour futur refusé, jour passé avec motif | RI-02 | **2820** |
| 3 | FK composites (NOT VALID puis VALIDATE) sur `ent_internat_*` ; `UNIQUE (etablissement_id, id)` ; `modifie_le` | RI-01 | **2820** |
| 4 | Capacité **atomique** (verrou de ligne sur la chambre) et contrainte « une affectation ouverte par élève » conservée ; test de concurrence | RI-10 | **2820** |
| 5 | Régime codé : table de correspondance des trois vocabulaires ; fonction serveur unique ; plus de `normalize_regime_internat` dans les décisions | RI-04 | **2821** |
| 6 | Réaction à `eleve.regime_changed` : fin d'affectation à la date d'effet, avis au responsable | RI-05 | — |
| 7 | Suivis : borne 300 caractères, avertissement « pas de santé », plus de case « visible famille » à la saisie (la publication passe par le bilan, I7) | RI-09 | — |
| 8 | Modifier, fermer une chambre (sans réduction sous l'occupation) | RI-07 | — |
| 9 | Tests : isolation entre établissements (matrice), droits par rôle, concurrence, fuseau (nuit du changement d'heure) | RI-10 | — |

### 25.2 Lots suivants
| Lot | Contenu | Dépend de | Critère de fin | Migrations |
|---|---|---|---|---|
| **I1** Hébergement et séjours | zones, chambres étendues, lits, séjours, rythmes, affectations datées sous contraintes (public, accessibilité, interdiction), dérogations, plan, paramètres | I0 ; 2, 5 | R-IT1–R-IT5, R-IT10–R-IT12 testés | 2821–2823 |
| **I2** La nuit | attendu calculé, appels (types, périmètres, listes), faits, clôture, validation, PDF, correction, rappel, cas et procédure, transmission au matin, effectif au 40 | I1 ; 12 (absence de nuit), 4 (numéros), 6 (astreinte) | parité NI-17 à NI-25 ; DI-01, DI-11, DI-13, DI-19 impossibles | 2824 |
| **I3** Absences et sorties | annonces des familles, décisions, sorties typées, correspondants, départ du vendredi, retours, remise (14), autorisations annuelles (18), majeurs, désaccords | I2 ; 4, 14, 18 | parité NI-33 ; R-IT19–R-IT23 testés | 2825 |
| **I4** Vie quotidienne | liaison codée, événements (→ 13, 34, 44), sorties de groupe, études (12), activités, préoccupation (36) | I2 ; 13, 18 | parité NI-27 à NI-41 ; DI-06, DI-14, DI-17 impossibles | 2826–2827 |
| **I5** Sécurité et santé | présents de nuit, impression de secours, exercices et rappels, PPMS, fiche de nuit (consignes du 34), mouvements | I2 ; 14, 34 | présents produits < 2 s pour 300 internes ; R-IT6 testé (aucune donnée de santé) | 2828 |
| **I6** Encadrement | besoins, lecture du planning du 6, trous, astreinte, prise et fin de service, garde d'honorabilité facultative | I2 ; 6, 15 | | 2828 |
| **I7** Familles et élèves | espace famille, annonces mobiles, planning, avis (8), bilan relu et publié | I3, I4 ; 8, 16 | | 2829 |
| **I8** Rentrée | répartition proposée (déterministe), vœux, reconduction, campagne d'installation, états des lieux et clés (44), capacité au 3, clôture d'année | I1 ; 3, 18, 19, 44 | parité NI-44 à NI-48 ; DI-07 impossible | 2830 |
| **I9** Argent | nuits du mois et causes au 24 ; mode suivi et état de comparaison pendant le doublon ; cautions | I3 ; 24 | R-IT35, R-IT36 testés | 2831 |
| **I10** Mobile surveillant | appel hors ligne chiffré, synchronisation et conflits, alerte, cas, fiche de nuit | I2, I5 ; 11 | appel complet en mode avion, 0 perte à la resynchronisation | — |
| **I11** Reprise et parallèle | outil de reprise des deux générations Next.js, correspondances, rapport, comparaison des appels pendant le parallèle | I1–I4 | reprise d'un export anonymisé sans perte des données « à conserver » | 2832 |
| **I12** Pilotage et conformité | tableau de bord, indicateurs (47), exports, registre R. 442-1, accès RGPD, purges, journal des lectures | I2–I8 ; 27, 47 | | 2833 |
| **I13** IA | capacités §18.3, jeu d'évaluation, tests de garde | I8, I2 ; `ent-ia` | 6 pièges réussis | — |
| **I14** Retrait de l'ancien | retrait des colonnes 066 héritées (`batiment` texte, `genre`), de `ent_internat_presence` et `ent_internat_suivi` après reprise, sur décision et sauvegarde vérifiée (E8) | toutes | | 2839 |

**Parcours minimal de la rentrée 2027 (usage réel, D-035)** : I0, I1, I2, I3, I4 (liaison et événements), I5, I7 (annonces et avis), I8, I10, I11. Ordre de réalisation recommandé : I0 → I1 → I2 → I3 → I5 → I10 → I8 → I4 → I7 → I11 → I6 → I9 → I12 → I13 → I14.

### 25.3 Plage de migrations
| N° | Lot | Objet |
|---|---|---|
| 2820 | I0 | présence en faits (ajout seul), reprise de 066 ; FK composites ; uniques ; `modifie_le` |
| 2821 | I0 / I1 | régime codé (correspondance) ; paramètres de l'internat ; types d'appel actifs |
| 2822 | I1 | zones, chambres étendues, lits |
| 2823 | I1 | séjours, rythmes, affectations étendues, dérogations, nuits exceptionnelles |
| 2824 | I2 | appels, listes, attendu figé, cas, procédures |
| 2825 | I3 | absences, types de sortie, sorties, faits de sortie, correspondants |
| 2826 | I4 | sorties de groupe |
| 2827 | I4 | liaison, événements, temps du soir |
| 2828 | I5 / I6 | exercices, besoins d'encadrement, lectures sensibles |
| 2829 | I7 | bilans |
| 2830 | I8 | campagne d'installation, réservations, vœux |
| 2831 | I9 | nuits du mois |
| 2832 | I11 | correspondances de reprise |
| 2833 | I12 | index et vues de pilotage |
| 2834–2838 | réserve | |
| 2839 | I14 | retrait des tables et colonnes héritées (sur décision) |

---

## 26. Reprise de Next.js et bascule

1. **Classement T0 confirmé** : bâtiments, chambres, affectations, autorisations de sortie, incidents, journal de l'année **à conserver** ; appels passés en **archive** (3 mois, ou année selon le DPO) ; messages, créneaux d'installation passés **perdables**.
2. **Fusion des deux générations** : les tables typées **priment** (affectations, appels typés, sorties de week-end) ; le JSON **complète** (bâtiments et étages, ailes, appels JSON avec leurs corrections, sorties de groupe, incidents, journal, activités) ; chaque interne JSON est **rapproché** d'un élève du cœur (INE, puis nom + prénom + date de naissance) ; cas incertains listés à Florian ou au CPE, jamais devinés (règle de `TRANSITION_NEXT_RUST.md` §2 bis).
3. **Données médicales de la fiche JSON** (DI-02) : **non reprises** dans le 41 ; liste remise à Florian et au DPO pour décision (destruction, ou transmission à l'infirmière par le 34 **quand** l'hébergement HDS existe) (Q-IT3).
4. **Contacts parents recopiés** : non repris ; les titulaires du 4 font foi ; les écarts (e-mail différent) sont listés pour le secrétariat.
5. **Journal, incidents** : repris comme faits de liaison au code « reprise » (texte figé) et événements ; les textes contenant des mots de santé sont signalés au CPE avant import.
6. **Planning des surveillants, accompagnateurs** (texte libre) : rapprochés des personnels du 6 ; non rapprochés listés.
7. **Bascule** [R, Q-IT1] : **été 2027 (T4)**, une fois les séjours de l'année 2027-2028 créés dans ScolIA, **avant l'ouverture des créneaux d'installation** (fin juin) ; l'année 2026-2027 se termine dans Next.js ; la reprise porte l'historique utile. Alternative (vacances de printemps, T3) déconseillée : elle déplace des appels vivants et des sorties en cours en milieu d'année.
8. **Parallèle de répétition** (juin 2027, deux semaines) : ScolIA calcule l'attendu et les surveillants volontaires font l'appel **dans les deux** ; comparaison automatique chaque soir (INT-244) ; **un seul propriétaire** officiel (Next.js) jusqu'à la bascule (R-IT37).
9. Next.js reste en lecture seule quelques semaines ; son arrêt est une décision et un geste de Florian (D-035).

---

## 27. Tests et recette
- **Isolation** : chaque route dans la matrice entre établissements ; périmètre collège / lycée (R-IT30) ; RLS stricte.
- **Droits** : professeur, infirmerie, psychologue, comptabilité **refusés** sur liaison, événements, cas, appels ; famille limitée à son enfant ; majeur.
- **Concurrence** : deux surveillants, même appel ; deux affectations sur le dernier lit ; annonce d'absence pendant l'appel.
- **Temps** : nuits des changements d'heure ; appel validé après minuit ; calendrier (vacances, fermeture).
- **Santé** : aucune colonne, aucun événement, aucun export du 41 ne contient de donnée de santé (test de schéma : liste de mots interdits dans les noms de colonnes ; test de charge utile des événements).
- **Hors ligne** : appel complet sans réseau ; effacement local.
- **Parité** : chaque NI vérifiée action par action contre Next.js (recette D-035) ; chaque DI rejoué comme test négatif.
- **IA** : tests de garde du cœur (équivalence écran / IA, simulation sans écriture), jeu d'évaluation §18.4.

---

## 28. Réglages signalés (D-023)
| S | Réglage | Défaut [R] |
|---|---|---|
| S-4100 | Rythme par défaut | dimanche soir à jeudi soir, selon le règlement |
| S-4101 | Types d'appel actifs et heures | arrivée, soir, matin ; extinction et ronde facultatives |
| S-4102 | Heure limite de validation de l'appel du soir | 21 h 30 |
| S-4103 | Procédure « non retrouvé » (étapes, délais) | modèle §9.3 |
| S-4104 | Heure limite d'annonce d'absence par la famille | 16 h le jour même |
| S-4105 | Types de sorties, par formation (collège, lycée) | mercredi, soirée, correspondant, week-end, exceptionnelle |
| S-4106 | Effectif minimal de nuit par bâtiment | 1 |
| S-4107 | Seuil de retard au retour, heure limite de récupération du vendredi | 30 min ; 18 h |
| S-4108 | Catalogue des faits de liaison | catalogue fourni |
| S-4109 | Exercices de nuit par an | 2 (sous réserve §5.9.1) |
| S-4110 | Impression automatique des présents à la clôture | oui |
| S-4111 | Garde d'honorabilité sur les postes de nuit | non |
| S-4112 | Liste de dépôt des téléphones | non |
| S-4113 | Voisins de chambre visibles des familles | non |
| S-4114 | Jour du bilan de la semaine | jeudi |
| S-4115 | Information des parents d'un majeur | « si à charge » |
| S-4116 | Délai de reconduction proposée des affectations des réinscrits | désactivé (confirmation explicite) |
| S-4117 | Liste du soir pour les sorties de groupe : réponse attendue jusqu'à | la veille 18 h |

---

## 29. Questions et décisions

### 29.1 Appliquées par défaut (D-031)
| Q | Question | Recommandation appliquée |
|---|---|---|
| Q-IT5 | L'internat possède-t-il les bâtiments ? | Non : bâtiment du 5, zones et lits au 41 |
| Q-IT7 | Le planning des surveillants est-il au 41 ? | Non : postes et planning au 6, besoins et lecture au 41 |
| Q-IT9 | Messagerie interne de l'internat ? | Remplacée par les fils d'équipe du 16 |
| Q-IT10 | Faits de présence écrasables ? | Non : ajout seul (R-IT13) |
| Q-IT11 | Sections garçons / filles ? | Appel par chambre, zone ou liste ; le public est porté par la zone |
| Q-IT12 | Chambre mixte possible ? | Non (référentiel national) ; dérogation individuelle codée |
| Q-IT13 | Visibilité des suivis par une case à la saisie ? | Non : publication dans le bilan relu |
| Q-IT14 | Un refus parental annule-t-il la sortie de groupe ? | Non : seul l'élève (R-IT21) |
| Q-IT15 | Accord parental hors ligne ? | Possible avec preuve (R-IT24) |
| Q-IT16 | L'appel d'étude au 41 ? | Composant du 12 |
| Q-IT17 | Les incidents au 41 ? | Fait de nuit au 41, qualification au 13 |
| Q-IT18 | Majeur : qui autorise ? | Lui, dans le cadre du règlement (R-IT22) |
| Q-IT19 | Heures et jours codés ? | Calendrier du 5, réglages §28 |
| Q-IT20 | Liste des présents hors ligne et imprimée ? | Oui (R-IT27) |
| Q-IT21 | Dépôt des téléphones ? | Liste facultative, désactivée |
| Q-IT22 | Vidéo, géolocalisation ? | Jamais (R-IT41) |
| Q-IT23 | Reconduction automatique des chambres ? | Non : confirmation explicite |
| Q-IT24 | Nom des voisins aux familles ? | Non par défaut (S-4113) |
| Q-IT25 | Facturation pendant le doublon ? | Mode suivi (R-IT36) |
| Q-IT26 | Créneaux d'installation : moteur propre ou 19 ? | 19 dès qu'il existe, sinon propre |
| Q-IT27 | IA : répartition automatique ? | Proposée, acceptée chambre par chambre |
| Q-IT28 | Interne externé : séjour d'hébergement ? | Non (§7.2) |
| Q-IT29 | Durées de conservation | §21.3, à confirmer avec le DPO |
| Q-IT30 | Ancien événement `internat.suivi` | émis jusqu'à I1, puis remplacé |

### 29.2 Soumis à Florian (engagent l'établissement, des données réelles, ou des faits que seul Florian connaît)
| Q | Question | Recommandation |
|---|---|---|
| **Q-IT1** | Moment de la bascule du module Next.js | **Été 2027**, avant l'ouverture des créneaux d'installation ; parallèle de répétition en juin |
| **Q-IT2** | Protocole « non retrouvé » : délais et étapes (appel des titulaires, de la direction, du 17) | modèle §9.3, **à valider par la direction et le CPE** de l'établissement |
| **Q-IT3** | Données médicales de la fiche Next.js : destruction ou transmission à l'infirmière (HDS) | décision avec le DPO ; en attendant, non reprises |
| **Q-IT4** | Accueil d'internes d'un autre établissement (convention) | hors V1 ; si besoin réel, fiche minimale au lot I13 |
| **Q-IT6** | Le registre de sécurité est-il tenu ailleurs (papier, logiciel du chef de sécurité) ? | ScolIA trace et exporte, ne remplace pas |
| **Q-IT8** | Activer la garde d'honorabilité sur les postes de nuit | non par défaut ; à décider avec la direction |
| **Q-T0-14** | Confirmer que le module internat de Next.js est **en usage réel** (et quelles parties : JSON ou typé) | nécessaire pour la parité et la reprise |

---
## 30. Ajouts personas (`PERSONAS_41.md`, 2026-11-11)

**En cas de contradiction avec les §0 à §29, ce paragraphe l'emporte.** Manques M-4101 à M-4160 ; quatre bloquants (A à D) appliqués par défaut (D-031).

### 30.1 Bloquants tranchés
| # | Bloquant | Décision appliquée | Effet sur la conception |
|---|---|---|---|
| A | Nuit sans réseau | Hors ligne de l'appel, de la fiche de nuit et des cas **dès I2** ; impression des présents prévus **obligatoire** | R-IT42 ; S-4110 n'est plus désactivable ; I10 ne garde que le confort |
| B | Astreinte sans seconde ligne | Chaîne d'astreinte à deux niveaux, bascule après 10 min sans accusé, refus d'ouvrir la nuit sans astreinte | R-IT43 ; INT-250 à INT-252 ; dépend du 6 |
| C | Annonce d'absence usurpable | « Attendu absent » jusqu'à la remise ou au départ seul autorisé ; passkey sur appareil inconnu ; avis immédiat à tous les titulaires | R-IT44 ; INT-253 à INT-255 ; §10.1 précisé |
| D | Trou de la bascule d'été | Propriété **par année scolaire** : 2027-2028 naît dans ScolIA dès juin | R-IT37 précisée ; INT-266 |

### 30.2 Actions ajoutées (INT-250 à INT-269)
| INT | Action | Acteur · droit | Règles | Événement | Lot |
|---|---|---|---|---|---|
| INT-250 | Voir un cas à distance (étapes faites, heures, dernière marque, dernière sortie) sur mobile | astreinte · `internat.lire` | M-4108 | — | I2 |
| INT-251 | Accuser réception d'une alerte d'astreinte (dans l'appli ou par appel tracé) | astreinte | R-IT43 | `internat.astreinte_accusee` | I2 |
| INT-252 | Déclarer la chaîne d'astreinte de la semaine ; refus d'ouvrir la nuit sans astreinte (alerte au responsable dès 17 h) | RIN, DIR · `internat.parametrer` ; SYS | R-IT43 | `internat.astreinte_manquante` | I6 (I2 pour le refus) |
| INT-253 | Confirmer par passkey une annonce faite depuis un appareil inconnu ou partagé avec le compte de l'élève | PAR | R-IT44 | — | I3 |
| INT-254 | Tenir l'élève « attendu absent » jusqu'à la remise (14) ou au départ seul autorisé ; sinon retour dans l'attendu à l'appel | SYS ; SUR, ACC | R-IT44 | `internat.attendu_modifie` | I3 |
| INT-255 | Aviser immédiatement **tous** les titulaires d'une annonce d'absence (chacun sans les coordonnées de l'autre) | SYS | R-IT44 | `internat.absence_annoncee` | I3 |
| INT-256 | Déclarer les destinataires des PDF d'appel par formation (collège, lycée) | DIR · `internat.parametrer` | parité Next.js | `internat.parametres_modifies` | I2 |
| INT-257 | Poser une **vigilance** codée (« suivi rapproché », « premier mois », « à accompagner au coucher ») et un **référent d'internat** pour un élève, datés, révisés chaque semaine, sans texte | CPE, RIN · `internat.gerer` | parité NI-06 « sous surveillance » | `internat.vigilance` | I4 |
| INT-258 | Accorder une **permission particulière** datée (« salle informatique jusqu'à 22 h ») | CPE · `internat.sorties.decider` | parité NI-06 | `internat.permission` | I3 |
| INT-259 | Pointer « vu, momentanément absent » (sanitaires, douche) : cas ouvert seulement après 15 min | SUR · `internat.appel` | M-4103 | `internat.presence_pointee` | I2 |
| INT-260 | Recevoir le rappel de ronde | SYS → SUR | S-4101 | — | I2 |
| INT-261 | Lancer l'alerte « surveillant en difficulté » (critique, sans élève) | SUR | R-IT34 | `internat.alerte` | I5 |
| INT-262 | Tenir l'annuaire d'urgence de l'établissement (15, 17, 18, 112, brigade ou commissariat de secteur) | DIR · `internat.parametrer` | | idem | I2 |
| INT-263 | Générer le compte rendu d'un cas (chronologie des étapes), relu par le CPE, transmis à la direction | SYS ; CPE | R-IT18 | `internat.cas_compte_rendu` | I2 |
| INT-264 | Imprimer les étiquettes de porte (prénom seul) | RIN | R-IT32 | — | I8 |
| INT-265 | Voir le tableau de rentrée par interne (règlement signé, autorisations, caution, état des lieux, consignes de nuit publiées) | RIN · `internat.lire` | M-4115 | — | I8 |
| INT-266 | Ouvrir l'année suivante dans ScolIA dès juin (séjours, autorisations, campagne, répartition) même si les nuits en cours restent dans Next.js | RIN ; Florian | R-IT37 précisée | — | I8, I11 |
| INT-267 | Remettre par lot (fratrie, covoiturage désigné par les titulaires) et pointer « parti seul autorisé » | ACC, SUR · `internat.appel` | R-IT23 | `internat.sortie_partie` | I3 |
| INT-268 | Revoir chaque trimestre les lectures inhabituelles (hors service, lecture massive) | délégué à la protection des données, DIR | R-IT38 | — | I12 |
| INT-269 | Révoquer sessions et données locales des appareils à la fin de l'engagement (6) | SYS | M-4147 | `internat.appareil_revoque` | I10 |

### 30.3 Règles ajoutées
| # | Règle | Lot |
|---|---|---|
| R-IT42 | **La nuit fonctionne sans réseau** : appel, fiche de nuit, procédure de cas et liste des présents disponibles hors ligne dès l'ouverture du service ; liste des présents prévus **imprimée** chaque soir, portant « à détruire le … » et aucun texte de santé (codes seuls). | I2, I5 |
| R-IT43 | **Pas de nuit sans astreinte** : une chaîne d'au moins deux niveaux est déclarée ; une alerte sans accusé bascule au niveau suivant après 10 min ; l'ouverture de la nuit sans astreinte est signalée en U3 au responsable et à la direction. | I2, I6 |
| R-IT44 | **Une annonce ne fait pas disparaître un mineur** : l'élève reste « attendu absent » jusqu'à sa remise ou son départ seul autorisé ; sans l'un ou l'autre, il redevient attendu à l'appel ; une annonce depuis un appareil inconnu ou partagé avec le compte de l'élève exige une passkey ; tous les titulaires sont avisés tout de suite. | I3 |

**Précisions** : R-IT6 — sans N-1, le seul fait de santé permis est le fait de liaison codé « premiers secours donnés, infirmière à informer », sans texte (M-4137) ; R-IT11 — la dérogation est portée **par l'élève pour l'année** et l'écran ne dit jamais « sexe », seulement « affectation à valider par la direction » (M-4134) ; R-IT33 — le résumé de nuit ne réécrit aucun texte de liaison (M-4156) ; R-IT37 — propriété des données **par année scolaire** pendant la bascule (bloquant D).

### 30.4 Cas limites ajoutés (CIT-49 à CIT-62)
| # | Situation | Traitement |
|---|---|---|
| CIT-49 | Deux élèves séparés l'an passé pour un conflit, réinscrits | Séparation reconduite comme contrainte de la répartition, révisable par le CPE |
| CIT-50 | Un appelant se dit « le père » au téléphone de l'internat | Aucune information par téléphone à un appelant non identifié ; consigne affichée pour les élèves ayant une interdiction |
| CIT-51 | Exercice d'évacuation avec une élève en fauteuil au 1er étage | Marque « assistance à l'évacuation » (code du 35) sur la liste des présents ; temps d'évacuation noté |
| CIT-52 | Élève transgenre réaffecté en cours d'année | Dérogation de l'élève déjà accordée : aucune nouvelle demande ; écran sans mention du motif |
| CIT-53 | Santé désactivée, malaise à 23 h | Fiche de nuit : lieu du classeur de l'infirmerie et des trousses ; fait « premiers secours donnés » ; 15 ; astreinte |
| CIT-54 | Élève qui a annoncé son absence depuis le compte de sa mère sur son propre téléphone | Passkey exigée (appareil connu de l'élève) ; à défaut, annonce en attente ; la mère et le père avisés |
| CIT-55 | Annonce d'absence acceptée, mais personne ne vient chercher l'élève | Attendu absent non remis à 18 h : l'élève redevient attendu ; appel des titulaires |
| CIT-56 | CPE d'astreinte injoignable 10 min | Bascule au niveau 2 (direction ou adjoint) ; trace des tentatives |
| CIT-57 | Le surveillant fait un malaise | Alerte « surveillant en difficulté » ; astreinte ; un autre adulte prend le service (délégation datée) |
| CIT-58 | Élève majeur qui travaille le samedi, revient le lundi en cours | Retour attendu en cours (12), pas à l'internat ; aucun cas d'internat |
| CIT-59 | Week-end ouvert, internat à 6 élèves | Rythme du week-end, besoins d'encadrement, repas (40), correspondants joignables à l'étape « titulaires » |
| CIT-60 | 90 départs le vendredi à 16 h | Remise par lot et pointage rapide ; interdictions toujours bloquantes |
| CIT-61 | AED parti en fin de contrat, l'appli reste sur son téléphone | Sessions révoquées ; données locales effacées au prochain démarrage |
| CIT-62 | Liste imprimée des présents retrouvée dans une poubelle | Fait d'incident de sécurité (27) ; rappel de la procédure de destruction ; aucun contenu de santé n'y figurait (R-IT42) |

### 30.5 Questions nouvelles
| Q | Question | Statut |
|---|---|---|
| Q-IT31 | Téléphones de service des surveillants de nuit | **Florian** (coût) |
| Q-IT32 | AIPD de l'internat | **Florian et DPO** |
| Q-IT33 | Chaîne d'astreinte réelle de l'établissement | **Florian** (direction) |
| Q-IT34 | Export quotidien de l'appel Next.js pendant le parallèle de juin | **Florian** (geste sur la production) |
| Q-IT35 | Durée de la liaison non publiée : 3 mois | appliqué par défaut (D-031), sous réserve du DPO |
| Q-IT36 | Fréquence du bilan pour les plus jeunes (6e, 5e) : chaque semaine le premier trimestre, puis selon S-4114 | appliqué par défaut (D-031) |

### 30.6 Lots révisés
- **I2** prend le hors ligne de l'appel, de la fiche de nuit et des cas (R-IT42), l'astreinte vue à distance et l'accusé (INT-250, INT-251), le refus d'ouvrir la nuit sans astreinte (INT-252), « vu, momentanément absent », le rappel de ronde, l'annuaire d'urgence, le compte rendu de cas.
- **I3** prend R-IT44 (INT-253 à INT-255), les permissions particulières, la remise par lot.
- **I10** garde la synchronisation fine, la gestion des conflits et la révocation des appareils.
- **I8** et **I11** prennent l'ouverture de l'année suivante dans ScolIA dès juin (INT-266).
- Migrations : la plage 2820–2839 suffit ; 2824 reçoit en plus `ent_internat_astreinte_accuse` et la marque `vu_momentanement_absent` ; 2827 reçoit `ent_internat_vigilance (id, etablissement_id, eleve_id, code, referent_personnel_id NULL, du, au NULL, revue_le, cree_le)` et `ent_internat_permission (id, etablissement_id, eleve_id, code, du, au, decide_par, cree_le)` ; 2825 reçoit la contrainte `EXCLUDE (etablissement_id, eleve_id, tstzrange(debut, fin_prevue) WITH &&) WHERE (statut IN ('autorisee','partie'))` (M-4158). Aucune colonne JSON.

---

## 31. Faits, hypothèses, contradictions

**Faits [F] clés**
- Next.js : deux générations sans pont (hub JSON en `ent_collection_record`, six tables typées) ; données médicales dans la fiche JSON de l'interne ; périmètre collège / lycée appliqué à l'affichage seulement ; jetons de sortie sans expiration ; confirmation d'installation par GET ; un refus parental annule la sortie de groupe ; marque « activité » à l'appel ; correction motivée après validation ; sorties de week-end typées à motif libre ; aucun contrôle de sexe à l'affectation.
- Rust : 066 (quatre tables), présence écrasée, suivis privés lisibles par tout rôle qui lit les élèves, capacité non atomique, FK simples ; `ent_batiment` existe au 5 (337) ; postes de service `internat_nuit` conçus au 6 (PRS-133).
- Prime à l'internat 2026-2027 : 327 à 672 € [V].

**Hypothèses [H]**
- Le module internat de Next.js est en usage réel dans l'établissement pilote, surtout la génération JSON (à confirmer, Q-T0-14).
- Réseau faible la nuit ; surveillant seul par bâtiment ; appareils personnels.
- Rythme dominant « dimanche ou lundi soir à jeudi soir ».

**Contradictions relevées (aucune n'est corrigée dans les autres documents par cette session)**
1. `TRANSITION_T0_INVENTAIRE.md` (M11) annonce « beaucoup de TODO » : **aucun** `TODO`, `FIXME` ni `HACK` n'a été trouvé dans le module ; l'inachevé est structurel (deux générations).
2. T0 compte « 80 fichiers / 15 700 lignes » : environ **50 fichiers propres** au module ; le reste vient de références croisées.
3. `SOCLES.md` §41 met le **planning des surveillants** dans l'internat ; la conception le place au **6** (postes et planning, PRS-133) et ne garde au 41 que les besoins et la lecture (Q-IT7).
4. `SOCLE2_DOMAINES.md` D12 range dans l'internat les **sorties de l'établissement** (INT-05 à INT-07 : autorisations de sortie permanentes, sorties exceptionnelles, contrôle par badge), qui relèvent du **14** ; la conception ne reprend que les sorties **d'internat** et renvoie le reste au 14.
5. `08_NOTIFICATIONS.md` prévoit un **SMS** pour `internat.absence_soir` ; le 41 le conditionne à Q-N5 (encore ouverte).
6. `05_STRUCTURE.md` §R-S1 fait cocher « internes présents » faute d'internat dans le Rust : à remplacer par `internat::attendu` une fois I2 livré.
7. `04_ADMINISTRATIF.md` note que l'internat n'est pas une résidence (régime) ; cohérent ; le **correspondant d'internat** demande une nature de personne autorisée au 4 (§6 de `PERSONAS_41.md`).
