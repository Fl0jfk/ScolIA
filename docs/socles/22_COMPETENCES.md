# Socle 22 — Compétences et livrets

**Date :** 2026-11-11 — **Statut :** conception de fond, **non décidée** (à relire par Florian) ; les recommandations sont appliquées par défaut (D-031) et listées au §26. Aucun code écrit ni modifié ; aucun accès à une base, à un secret ni à un service de production ; l'application Next.js n'a pas été ouverte pour ce socle (aucun module de compétences n'y est en usage, `TRANSITION_T0_INVENTAIRE.md` C06).
**Branche :** `ent-socle-22-23-conception`, partie de `origin/ent-mc0hoe` (`7467f2d`).
**Passe personas :** `docs/socles/PERSONAS_22.md` ; ses ajouts sont faits **ici**, marqués **« ajout personas »** (§28) ; en cas de contradiction, le §28 l'emporte ; rien n'est supprimé.
**Cohérence stricte avec le socle 21** (`21_EVALUATIONS.md`, en particulier §4, §6.1, §9.4, §10, §14, §18, §21, §29) **et D-032** : notes et bulletins du lycée pilote restent dans Charlemagne l'an 1 ; les formats officiels (LSU) attendent la démarche éditeur (F-06, vers mai 2027) ; le reste avance.
**À lire avec :** `CLAUDE.md` ; `DECISIONS.md` D-017, D-021 à D-036 (D-022 portée au groupe et exports, D-023 modèles prêts, D-031, D-032, D-034, D-035, D-036) et D-037 à D-046 (exigences E1–E14, `PRA_SAUVEGARDES.md` §18.4) ; `SOCLES.md` §22 ; `MATRICE_CHARLEMAGNE.md` §3.7 (NOT-08 à NOT-10, T-28) ; `09_INTEROP.md` (moteur d'exports, FX-33, FX-36, R-X24) ; `23_BULLETINS.md` (conseil, bulletin) ; `IA_COEUR.md` §5.
**Conventions.** [F] fait vérifié dans le code (chemins depuis `ent-rust/`) ; [V] règle lue sur une source officielle (§5) ; [V*] copie fidèle ; [NV] non vérifié, **interdit de coder dessus** ; [H] hypothèse ; [R] recommandation.
**Numérotation (préfixes vérifiés libres sur `docs/` le 2026-11-11) :** actions **CMP-100 à CMP-231** ; règles **R-C1 à R-C20** ; réglementaire **R-C01 à R-C16** ; cas limites **CC-01 à CC-41** ; défauts de l'existant **CD1 à CD10** ; questions **Q-CP1 à Q-CP12** ; réglages **S22-01 à S22-08** ; points pour Florian **F22-1 à F22-5** ; lots **C0 à C13** ; migrations **2440–2459** (plage imposée).

---

## 0. En bref

1. **L'existant a deux modèles qui s'ignorent** : des items D1–D5 positionnés par période (088), **écrasés sans historique**, sur une graine **fabriquée** ; et un bilan de 8 composantes par cycle (181), **visible de la famille dès sa validation**, sans publication ni visa. Le livret se publie **sans contrôle**, un envoi à chaque clic. Aucun écran Leptos, aucun appel mobile, aucune attestation. Un **lot C0** de correctifs (dix défauts CD1–CD10, cinq urgents) passe avant tout (§2).
2. **Le réglementaire tranche [V, §5]** : 8 composantes (D. 122-1, D. 311-7) ; échelle de fin de cycle à 4 échelons **fixée par D. 122-3**, sans compensation entre composantes ; **trois visas** du bilan (PP, chef d'établissement, **parents**) ; attestations PSC, ASSR 1, ASSR 2, AER, ASSN (art. 7) et **Pix** en 3ᵉ et terminale (arrêté de 2019) ; en 5ᵉ–4ᵉ « la note ou un autre positionnement », en 3ᵉ la note ; **le socle ne compte plus au DNB 2026** mais le bilan de fin de cycle reste obligatoire au livret ; le LSL a **sa propre échelle** à 4 degrés ; les « groupes de besoins » sont remplacés depuis juillet 2026 par un accompagnement renforcé sans obligation de groupes.
3. **Un seul modèle** : référentiel **national fourni et figé** (composantes, échelles, attestations) ; référentiels **d'établissement** versionnés, chaque item rattaché à une composante ; référentiels **de diplôme** (lycée professionnel) ; **positionnements** datés, signés, historisés, saisis **dans la grille de notes du 21** (un seul geste) ou en observation rapide sur mobile (§6, §21).
4. **Proposer, jamais imposer** : la synthèse de période est **proposée** par une règle simple (médiane des derniers niveaux, expliquée) et **validée** par le professeur ; le bilan de fin de cycle est **préparé** (dossier d'appui de trois ans), **arrêté en conseil** (socle 23), **visé** par le chef d'établissement, **pris en connaissance** par chaque responsable ; aucun niveau pré-rempli, aucune « maîtrise globale » (§7, §8).
5. **Rien sans publication** (R-V7 du 21) ; positionnements au fil de l'eau par `voir_notes`, synthèses, bilans et attestations par `recevoir_bulletins` (comme les appréciations, R-V34) ; niveaux **toujours en mots**, avec un **libellé famille** à côté du libellé officiel (§9).
6. **L'IA ne positionne jamais un élève** (règlement IA, annexe III 3 b) : elle écrit des libellés lisibles, structure un référentiel de diplôme, rattache les questions d'un sujet aux éléments, explique un bilan publié ; tout est validé (§14).
7. **LSU** : déclaration produite par le 22 sur le moteur du socle 9 ; **pré-contrôle huit semaines avant** l'échéance (trou T-28 du marché) et **aide à la saisie** tout de suite ; format officiel **après la démarche éditeur** (C8b, D-032) (§10).
8. **93 actions** (CMP-100 à CMP-231), **41 cas limites**, **20 règles**, **14 lots** (C0 à C13), **14 migrations** planifiées dans 2440–2459 (6 en réserve) ; **parcours minimal vendable au collège** : C0 à C6 + C8a. Le pilote étant un lycée, son besoin est surtout C7 (échelle LSL, CECRL) et C5 (Pix) (§17, §23).
9. **Pour Florian** : démarche éditeur (F-06) ; exemples LSU anonymisés ; durées de conservation (DPO) ; référentiels modèles ; un collège pilote (§26.2).
10. **[ajout personas]** 17 personas, 78 manques, 3 bloquants tranchés (D-031) : publication aux familles suspendue au droit `recevoir_bulletins` du socle 4 (repli et tâche au secrétariat), observations jamais fondées sur un comportement isolé ni sur la discipline, bilan des élèves à deux équipes (ULIS) co-rédigé par le coordonnateur ; 27 actions ajoutées (CMP-240 à 266) ; voir §28.

---
---

## 1. Thèse : une compétence est un repère, un positionnement un jugement daté, un bilan un acte de l'équipe

Une famille de collège pose trois questions : **qu'est-ce que mon enfant sait faire, qu'est-ce qui lui manque encore, que peut-on faire ?** Une note y répond mal ; une liste de 80 items colorés y répond plus mal encore. Le socle 22 doit permettre aux équipes de dire, **avec peu de saisie**, où en est chaque élève par rapport à des attendus explicites, et produire les actes que le droit exige : le **bilan périodique** (avec positionnement à la place ou à côté de la note en 6ᵉ–4ᵉ), le **bilan de fin de cycle** (8 composantes du socle commun, quatre niveaux de maîtrise), les **attestations** (savoir nager, sécurité routière, Pix…), puis leur **remontée au LSU**.

Douze principes [R] :

1. **Un seul modèle de compétences.** Les deux modèles concurrents de l'existant (items D1–D5 par période ; 8 composantes par cycle, §2) fusionnent : un **référentiel versionné**, des **éléments** (domaine, composante, compétence, item) et des **échelles** explicites.
2. **Le national est fourni, figé et daté ; l'établissement ajoute, ne réécrit pas.** Les 8 composantes et l'échelle de fin de cycle sont des **données de référence** livrées par ScolIA (non modifiables) ; les items disciplinaires sont des **données de l'établissement** (D-022 : liberté des données), rattachés à une composante.
3. **Un positionnement est un fait daté, signé, historisé** : qui, quand, sur quel élément, à partir de quoi (une évaluation du socle 21, une observation en classe, un import). Il ne s'écrase jamais (corrige l'UPSERT existant).
4. **Positionner ne double pas la saisie.** Une évaluation du 21 peut porter des éléments : la saisie de la grille positionne dans le même geste. Un professeur qui n'évalue que par notes n'a **rien** à saisir de plus pour le bilan périodique.
5. **Une synthèse n'est jamais un calcul imposé.** ScolIA **propose** une synthèse périodique (règle déterministe, expliquée) ; le professeur la **valide** ou la change. Le bilan de fin de cycle est **arrêté par l'équipe** (conseil), **visé** par le chef d'établissement, jamais produit par une moyenne de couleurs.
6. **Rien n'est visible d'une famille sans publication** (R-V7 du 21) : positionnements avec l'évaluation, synthèses avec le bulletin, bilan de fin de cycle avec le bulletin de fin d'année.
7. **Le bilan de fin de cycle est un acte** : version, empreinte, visa, prise de connaissance des responsables ; une correction est une nouvelle version motivée.
8. **Une attestation est un fait administratif** (obtenue le …, par …, pièce), pas une compétence : elle a sa propre table, son document (socle 7) et sa place au LSU.
9. **L'IA ne positionne jamais un élève** (règlement IA, annexe III point 3 b, R-V40 du 21). Elle aide à **rédiger un référentiel lisible**, à **rattacher les questions d'un sujet** à des éléments (proposition validée), à **expliquer** un bilan à une famille, jamais à juger un élève.
10. **Peu de réglages** (D-023) : trois échelles de positionnement prêtes, deux méthodes de synthèse, un mode « avec notes / sans notes / les deux » par niveau. Pas d'éditeur de grilles de A à Z.
11. **Le LSU est une sortie, pas le modèle.** On modélise le besoin pédagogique et légal ; le format officiel est une **projection** par le moteur du socle 9, qui attend la démarche éditeur (D-032, F-06).
12. **Ce que chacun voit est minimal** (D-022) : le professeur, ses cours ; le PP, sa classe ; la famille, ce que son lien ouvre ; l'AESH, rien des positionnements ; le tiers, jamais la rubrique « accompagnements » (R-V37).

**Ce qui fera la différence [H — comparaison de mémoire avec Pronote, École Directe, Charlemagne]** : la **compétence saisie dans la grille de notes** (un seul geste) ; la **synthèse proposée et expliquée** (« 3 positionnements en octobre-novembre : fragile, satisfaisante, satisfaisante → proposition : satisfaisante, le plus récent prévaut ») ; le **bilan de fin de cycle préparé** pour le conseil de juin, avec l'historique des trois ans ; le **pré-contrôle LSU** huit semaines avant l'échéance (trou T-28 de la matrice) ; un **référentiel en mots simples** pour la famille, à côté du libellé officiel ; les **attestations** tenues à jour sans tableur (ASSN de 6ᵉ, ASSR de 5ᵉ et 3ᵉ, Pix).

---

## 2. Évaluation franche de l'existant

Lu sur `ent-mc0hoe` (`7467f2d`) par un sous-agent d'audit en lecture seule ; constats décisifs relus ici quand cité « relu ». Chemins depuis `ent-rust/`. Le front Leptos est sous `web/crates/ent-web-app/` (et non `crates/ent-web-app/` comme l'écrit le §2.1 du 21).

### 2.1 Ce qu'il y a [F]

| Brique | Où | Ce qu'elle fait | Verdict |
|---|---|---|---|
| **Modèle A** « items par période » | migration `088` (`ent_competence_item`, `ent_competence_evaluation`) ; `crates/domain-eleve/src/competence.rs:21,74,130,207` ; `crates/ent-db/src/competence_repo.rs:31,99` | référentiel de l'établissement, items rattachés à un domaine D1–D5 ; niveau 1–4 (insuffisante, fragile, satisfaisante, très bonne) par **(élève, item, période)** ; livret PDF minimal | **fusionner** dans le modèle unique (§21), reprise des données (§22) |
| Graine du référentiel A | `crates/ent-db/src/service.rs:6646` | 10 items « D1.1 … D5.2 » **fabriqués**, niveaux fictifs par modulo (`i % 4`) | **retirer** hors du labo (C0) |
| Création d'items (A) | `service.rs:6699` ; lecture `:6713` | par rôle (`role_can_manage_matieres` ou `manage_eleves`), **sans action du catalogue**, sans journal ; un libellé modifié change aussi l'historique (`competence_repo.rs:36-40`) | **corriger** (C0 : action D-017 ; C1 : versions) |
| Positionnement (A) | `service.rs:6729` ; route `crates/ent-api/src/routes.rs:5812` | action `competences.evaluer` (`crates/ent-auth/src/actions.rs:19,461`), portée `autoriser_eleve`, période ouverte requise, événement en outbox ; **UPSERT sans historique** (`competence_repo.rs:99-125`) : l'auteur et la date d'origine sont perdus | **garder** le contrôle de droits ; **corriger** l'écrasement (C0 historique, C2 modèle) |
| Lecture famille (A) | `service.rs:6795` | famille et élève ne voient que les périodes publiées (D3 b du 21, corrigé par V0) | **garder** |
| Publication du livret (A) | `service.rs:6822` ; route `routes.rs:5856` | PDF déposé au tiroir famille + courriel aux parents ; **aucun contrôle de période, aucune version, un dépôt et un envoi à chaque appel** (même défaut que D5 du 21 pour le bulletin, ici non corrigé) | **fermer** (C0, 410) |
| **Modèle B** « 8 composantes par cycle » | migration `181` (`ent_bilan_cycle`, `ent_bilan_cycle_composante`) ; `crates/domain-eleve/src/evaluation.rs:391-408` ; `crates/ent-db/src/evaluations_conseil.rs:964,1067,1122` | bilan par (élève, année, cycle 2/3/4), 8 codes de composantes en dur (`CPD_FRA` … `REP_MND`, marqués [NV] dans le code), niveau 1–4, brouillon → validé par le PP ou `bulletins.publier` | **garder comme base** du bilan de fin de cycle ; **corriger** : publication, réouverture, historique, cycle déduit du niveau |
| Bilan visible sans publication (B) | `evaluations_conseil.rs:1122-1131` | la famille voit le bilan **dès sa validation**, sans publication, sans visa | **corriger** (C0) |
| Bilan validé sans retour (B) | `evaluations_conseil.rs:1004,1067` | une fois validé, verrouillé sans réouverture ni historique | **corriger** (C4 : versions) |
| Export LSU XML | `crates/ent-db/src/evaluations_bulletin.rs:765` ; `crates/domain-eleve/src/evaluation.rs:420-600` | lit le modèle B ; espace de noms inventé (corrigé en mention « non officiel » par V0, D14) ; trace `ent_eva_export` avec `conforme_verifie = false` | **garder** la trace ; **refaire** au moteur du socle 9 (C8) |
| Export LSUN CSV historique | `service.rs:15775` ; `crates/domain-eleve/src/lsun_export.rs` (échappement des formules `:41-46`) ; route `routes.rs:7351` | lit le modèle A ; droit `exports.officiels` + périmètre de classe ; trace par le journal d'accès, pas par `ent_eva_export` | **garder** comme « export de travail » jusqu'à C8, puis retirer |
| UI JavaScript | `crates/ent-api/src/ui.rs:53252-53426` | écran compétences du modèle A | **figé** (F-001) ; remplacé par Leptos (C2) |
| Écrans Leptos | `web/crates/ent-web-app/src/modules/evaluations/` | **aucun** écran de compétences ni de bilan de cycle | **à faire** |
| Applis mobiles | `ent-rust/apps/` | **aucun** appel aux compétences | **à faire** (C10) |
| Attestations (ASSN, ASSR, Pix…) | — | **absentes** de toutes les migrations | **à faire** (C5) |
| EPI, AP, parcours | — | absents | **à faire** (C6) |
| Tests | `crates/ent-db/src/tests_evaluations_v0.rs:121` (compétences invisibles avant publication) ; `competence.rs:243` (unitaires) | rien sur le bilan de cycle, l'historique, la fusion | **compléter** |
| RLS | migration `460_eva_v0_correctifs.sql` (présente sur `ent-mc0hoe`, `migrate.rs:220`) | RLS **permissive** sur `ent_competence_item`, `ent_competence_evaluation`, `ent_bilan_cycle`, `ent_bilan_cycle_composante` ; **aucune** clé étrangère composée ; `eleve_id` et `periode_id` sans clé étrangère sur `ent_competence_evaluation` | **stricte** au lot C13 ; FK composées dans les nouvelles tables |

**Next.js (D-035)** : aucun module de compétences en usage. Les tables `note_*` du module C06 (« phases Charlemagne ») sont classées **ébauche, vide probable, perdable** (`TRANSITION_T0_INVENTAIRE.md` l. 88 et 135) : rien à reprendre, rien à perdre. Charlemagne fait foi pendant le doublon (D-034).

### 2.2 Défauts trouvés (à corriger avant toute extension)

| # | Défaut | Conséquence | Où [F] | Correction |
|---|---|---|---|---|
| CD1 | **Positionnement écrasé** (UPSERT), auteur et date d'origine perdus | impossible de répondre « ce niveau a-t-il changé après que je l'ai lu ? » ; aucune preuve en cas de contestation | `competence_repo.rs:99-125` | C0 : journal en ajout seul des valeurs écrasées (déclencheur) ; C2 : modèle historisé |
| CD2 | **Bilan de fin de cycle visible de la famille dès validation**, sans publication ni visa | la famille lit un bilan que le conseil n'a pas arrêté ; contraire à R-V7 | `evaluations_conseil.rs:1122-1131` | C0 : visible seulement avec un bulletin publié de la dernière période de l'année |
| CD3 | **Publication du livret sans contrôle** : ni période, ni version, ni droit du lien, un dépôt et un courriel à chaque appel | plusieurs livrets différents au coffre ; envoi à des parents sans droit (le courriel part à « tous les liens » par l'ancien chemin, défaut E2 du socle 8) | `service.rs:6822` ; `routes.rs:5856` | C0 : route **fermée (410)**, comme D5 du 21 |
| CD4 | **Référentiel modifiable sans action du catalogue** ni journal ; libellé modifié rétroactivement | un rôle large réécrit l'historique des positionnements | `service.rs:6699-6710` ; `competence_repo.rs:36-40` | C0 : action `competences.referentiels_gerer` (direction) ; C1 : versions |
| CD5 | **Graine fabriquée** de 10 items et niveaux fictifs | risque qu'un établissement réel reçoive un référentiel inventé | `service.rs:6646` | C0 : graine réservée au mode labo (refus hors labo) |
| CD6 | **Deux modèles non reliés** (A par période, B par cycle), aucune proposition de B depuis A | double saisie ; deux vérités | §2.1 | C1–C4 : modèle unique, reprise (§22) |
| CD7 | **Cycle non rapproché du niveau** de l'élève (B accepte cycle 2 pour un collégien) | bilan de cycle 2 au collège, bilan de cycle 4 en 5ᵉ | `evaluations_conseil.rs:964` [sous-agent] | C4 : cycle **déduit** du niveau (socle 5), contrôle |
| CD8 | **Codes des 8 composantes non vérifiés** | export LSU rejeté ou mal interprété | `evaluation.rs:393-402` | C8 : codes de la spécification éditeur (D-2) ; d'ici là, codes internes ScolIA et libellés officiels [V §5] |
| CD9 | **Aucune clé étrangère** sur `eleve_id` et `periode_id` de `ent_competence_evaluation` ; pas de FK composée | isolation par filtres seulement ; lignes orphelines possibles | migration `088` | nouvelles tables à FK composées (E2) ; RLS stricte (C13) |
| CD10 | **Écran compétences de l'UI JS** branché sur le modèle A ; aucun écran Leptos | les professeurs n'ont pas d'outil propre | `ui.rs:53252-53426` | C2 : écrans Leptos ; UI JS figée |

D1–D5, D8–D12 et D14 du socle 21 sont corrigés sur `ent-mc0hoe` par le lot V0 (migration 460 et routes fermées en 410) [F, sous-agent] ; D3 b (compétences visibles avant publication) l'est ; **CD1 à CD10 ne le sont pas**.

### 2.3 Ce que Charlemagne et Pronote font, et qu'il ne faut pas perdre [H, `MATRICE_CHARLEMAGNE.md` §3.7]
Compétences et socle commun, référentiels, positionnement (NOT-08, « C(V mod.) » : module Charlemagne vérifié) ; LSU au format officiel (NOT-09) ; **contrôle préalable d'export** (NOT-10, **trou** du marché, T-28, G9 : « export LSU bloqué par des données manquantes, signalées par une simple infobulle »). Pronote : évaluation par compétences dans le devoir, bilan de fin de cycle proposé, items du socle et « éléments signifiants » [H, pages publiques lues pour le 21 le 2026-11-09, sans détail]. **À ne pas reproduire** : l'avalanche d'items colorés par séance, qui épuise les professeurs et ne dit rien aux familles [H, terrain].

---

## 3. Acteurs, besoins réels, irritants

| Acteur | Ce qu'il fait ici | Ce qui lui fait perdre du temps aujourd'hui [H] | Ce qu'il redoute | Ce qui le ravirait | Fréquence |
|---|---|---|---|---|---|
| **Professeur de collège** (toutes matières) | positionne ses élèves sur quelques éléments par évaluation ; valide la synthèse de période ; renseigne les « éléments du programme travaillés » | double saisie notes + couleurs ; items trop nombreux ; synthèse recalculée à la main ; grille de compétences séparée de la grille de notes | être jugé sur la « couleur » d'un item qu'il n'a pas travaillé ; positionner faute de temps | positionner **dans la grille de notes** ; synthèse proposée qu'il accepte en un geste ; « éléments travaillés » repris du cahier de textes (socle 20) | chaque évaluation ; fin de période |
| **Professeur principal** | prépare les bilans périodiques de sa classe ; propose le bilan de fin de cycle (6ᵉ, 3ᵉ) | relancer chaque collègue ; reconstituer trois ans de positionnements pour le bilan de 3ᵉ | un bilan de fin de cycle injuste, non étayé | tableau « ce qui manque » ; bilan de fin de cycle **pré-rempli** avec les éléments d'appui de chaque composante | fin de période ; mai–juin |
| **Coordonnateur de discipline / conseil pédagogique** | construit la progression de cycle et les items de la discipline | référentiels recopiés d'un tableur ; incohérences entre collègues | « chacun son référentiel » | référentiel de discipline partagé, versionné, rattaché aux composantes ; reprise d'un modèle prêt | rentrée ; ajustements |
| **Direction (chef d'établissement, adjoint)** | arrête les choix (avec ou sans notes, échelle) ; vise les bilans de fin de cycle ; pilote l'état des bilans | découvrir en juin que 40 bilans de 3ᵉ sont incomplets ; exports bloqués la veille | un LSU rejeté ; un bilan contesté | tableau de préparation LSU **huit semaines avant** ; visa en lot avec contrôles | rentrée ; juin |
| **Secrétariat de scolarité** | attestations, exports LSU, imports des bilans de l'école ou d'un autre collège | rejets LSU incompréhensibles ; attestations sur papier | une attestation perdue (ASSR2 pour le permis) | attestations tenues avec pièce ; duplicata en un geste ; pré-contrôle | au fil de l'année ; juin |
| **CPE** | contribue au domaine 3 (formation de la personne et du citoyen) et aux parcours (citoyen, santé) ; engagements | rien pour valoriser l'engagement des élèves | voir l'engagement réduit à une note de vie scolaire | engagements (délégué, éco-délégué…) repris au bilan et au livret, sans ressaisie | par période |
| **Professeur d'EPS** | savoir nager (ASSN, 6ᵉ) ; composante « langages des arts et du corps » ; dispenses | ASSN tenue sur papier | positionner un élève dispensé | ASSN saisie au fil des séances de natation ; dispense appliquée (socle 34, sans motif) | 6ᵉ ; par cycle |
| **Professeur documentaliste, référent numérique** | Pix (compétences numériques), éducation aux médias | résultats Pix recopiés | — | résultats Pix rattachés à l'élève (saisie ou import), visibles au livret | 3ᵉ ; terminale |
| **Référent sécurité routière** (souvent technologie) | ASSR1 (5ᵉ), ASSR2 (3ᵉ) | listes papier ; duplicatas demandés des années après | une attestation introuvable pour le permis | attestation émise par le socle 7, retrouvable 10 ans après | annuel |
| **Coordonnateur ULIS / SEGPA / UPE2A** | positionne sur des éléments adaptés ; appréciation de dispositif (21 §12.3) | référentiels adaptés hors logiciel | qu'un document à un tiers révèle le dispositif | éléments adaptés rattachés aux composantes ; « accompagnements » au bulletin famille seulement | par période |
| **Professeur de lycée professionnel** | évalue les compétences du référentiel de diplôme ; PFMP (socle 39) ; CCF (socle 38) | grilles du référentiel recopiées ; positionnements PFMP sur papier | perdre la trace des compétences évaluées en entreprise | référentiel de diplôme chargé une fois ; grille de positionnement partagée avec le tuteur (socle 39) | par séquence |
| **Professeur de langue (lycée)** | positionne sur le CECRL (A1 à C1) | — | — | positionnement CECRL repris au LSL | fin d'année |
| **AESH** | accompagne ; **ne positionne pas** | — | lire des données qu'il ne doit pas voir | rien ici (calendrier et aménagements au 21) | — |
| **Famille** (dont parents séparés) | lit les positionnements publiés, le bilan périodique, le bilan de fin de cycle ; prend connaissance du bilan | couleurs incomprises ; jargon (« langages pour penser et communiquer ») | apprendre un « insuffisant » en fin de 3ᵉ sans l'avoir vu venir | **libellé en mots simples** à côté du libellé officiel ; évolution sur le cycle ; exemple de ce que l'élève sait faire | par période ; fin de cycle |
| **Élève mineur** | voit ses positionnements ; peut s'**auto-positionner** si le professeur l'ouvre | ne sait pas ce qu'on attend de lui | être étiqueté | « ce qu'il me reste à travailler », avec les ressources du cahier de textes | au fil de l'eau |
| **Élève majeur** (lycée) | voit tout ce qui le concerne ; décide de ce que voient ses parents (socle 4) | — | — | — | — |
| **Établissement d'accueil** | reçoit le livret d'un élève qui arrive | bilans papier, ressaisie | — | import des bilans antérieurs (fichier, pièce), visibles du PP | à l'arrivée |
| **École du secteur (liaison CM2–6ᵉ)** | le bilan de fin de cycle 3 se fait en fin de 6ᵉ, mais CM1–CM2 sont à l'école | aucune trace des positionnements de l'école au collège | — | bilans de CM1/CM2 lus (import LSU ou pièce) au dossier du PP de 6ᵉ | rentrée de 6ᵉ |
| **DPO, auditeur** | droit d'accès, conservation | — | des positionnements gardés sans limite | export d'accès (art. 15) ; durée « fin de cycle + 1 an » (avis CNIL LSUN [V]) | sur demande |

**Ce qui fait perdre le plus de temps, en résumé [H]** : (1) la **double saisie** notes + compétences ; (2) les **référentiels trop fins**, qui transforment chaque devoir en coloriage ; (3) la **reconstitution du bilan de fin de cycle** en juin de 3ᵉ ; (4) les **rejets LSU** découverts au dépôt ; (5) les **attestations** sur papier, perdues au moment où l'élève en a besoin.

---

## 4. Frontières

### 4.1 Tableau

| Sujet | 21 | **22** | 23 | Autres |
|---|---|---|---|---|
| Évaluations, résultats, publication, période par élève (R-V29) | **possède** | lit | lit | — |
| **Lien évaluation ↔ éléments** (une évaluation porte des éléments) | **possède** la table de lien (§6.1 du 21 : « le 21 stocke le lien ») | possède les éléments ; lit le lien | — | — |
| Référentiels, éléments, échelles | — | **possède** | lit | 39 (PFMP), 38 (CCF) lisent ; 50 (premier degré) réutilise |
| Positionnements (par évaluation ou observation), historique | — | **possède** | lit (instantané) | 39 écrit **par commande** du 22 (positionnement en entreprise, validé par le professeur référent) |
| Synthèse de période par enseignement, « éléments du programme travaillés » | — | **possède** | imprime au bilan périodique | 20 fournit les contenus de séance (proposition) |
| EPI, AP, parcours (Avenir, citoyen, éducation artistique et culturelle, santé) au bilan périodique | — | **possède** les projets et leur rattachement | imprime | 37 possède le parcours Avenir dans son fond (orientation) ; 22 n'en garde que la ligne du bilan |
| Bilan de fin de cycle (8 composantes, synthèse, visa) | — | **possède** | le conseil de la dernière période **l'arrête** par une commande du 22 ; l'imprime | 5 (niveau → cycle) |
| Attestations (ASSN, ASSR, Pix, PSC…) | — | **possède** les faits | imprime si le modèle l'imprime | 7 rend le document ; 38 pour les certifications d'examen |
| LSU (bilans périodiques, fin de cycle, moyennes de 3ᵉ, attestations) | fournit moyennes et statuts | **produit** la déclaration | fournit appréciations et vie scolaire | **9** moteur, pré-contrôle, trace, dépôt ; **bloqué** par D-2 |
| Compétences du LSL (lycée), CECRL | — | **fournit** les positionnements | **produit** le LSL | 9 |
| Dispenses, aménagements | lit | lit (sans motif) | — | 34, 35 possèdent |
| Accompagnements (PAP, PPS, PPRE, ULIS, UPE2A, SEGPA) au bilan | — | lit le **libellé de dispositif** (35) | imprime au bulletin famille | **35** possède ; jamais vers un tiers (R-V37) |

### 4.2 Ports exposés par le socle 22 [R]
- `competences::elements(tx, cours | matiere | composante, date) -> [Element]` (version en vigueur à la date) — 21 (grille), 20 (lien d'un travail), 39, 38, IA.
- `competences::positionnements(tx, eleve, periode | annee | cycle) -> [Positionnement]` (avec origine) — 23 (instantané), 37, IA.
- `competences::syntheses(tx, classe, periode) -> [SynthesePeriode]` — 23 (bilan périodique).
- `competences::bilan_cycle(tx, eleve, cycle) -> Option<BilanCycle>` (version publiée ou en cours selon l'acteur) — 23, 9.
- `competences::arreter_bilans(tx, conseil, decisions[]) -> Resultat` (commande appelée par le 23 pendant le conseil de la dernière période).
- `competences::attestations(tx, eleve) -> [Attestation]` — 23, 7, 9, 3 (dossier de transfert).
- `competences::etat_preparation(tx, classe | niveau, echeance) -> EtatPreparation` — 9 (pré-contrôle LSU), 47 (pilotage), 29 (à-faire).
- `competences::declaration_lsu(tx, perimetre, campagne) -> DeclarationLsu` — 9 (moteur d'exports).

### 4.3 Ports consommés [R]
- **21** : `evaluations::periode_eleve` (R-V29), `evaluations::moyennes_annuelles`, statuts (`en_attente`, `dispense`), publication d'une évaluation, lien évaluation ↔ éléments.
- **5** : niveau et MEF de l'élève à la date → cycle (§7) ; périodisations ; cours et public à la date ; matières.
- **6** : intervenants d'un cours à la date ; PP de la classe ; missions (coordonnateur de discipline, référent sécurité routière, référent numérique).
- **4** : droits du lien (`voir_notes`, `recevoir_bulletins`) ; majorité.
- **34 / 35** : dispense active (sans motif) ; dispositif de l'élève (libellé fonctionnel) ; aménagement d'évaluation.
- **7** : rendu des documents (attestation, bilan de fin de cycle), coffre, prise de connaissance ; **8** : avis ; **9** : moteur d'exports ; **20** : contenus de séance.

### 4.4 Points à accorder avec les socles voisins [R]
1. **Socle 21** : la table de lien évaluation ↔ éléments est **à créer par le 21** (dans sa réserve 468–477) sous le nom `eva_evaluation_element (etablissement_id, evaluation_id, element_id, ordre, bareme_item_id NULL)` ; `bareme_item_id` relie un exercice du barème détaillé (21 §8.9) à un élément. Le 22 la lit par port. C'est l'application stricte du §6.1 du 21.
2. **Socle 21** : une évaluation de mode `positionnement` (sans barème) n'a **aucun** résultat chiffré ; son résultat par élève est un ensemble de positionnements du 22. Le statut du résultat (`absent`, `dispense`, `non_rendu`) reste au 21 et s'applique à tous les éléments de l'évaluation.
3. **Socle 23** : le bilan de fin de cycle est **arrêté pendant le conseil** de la dernière période de 6ᵉ et de 3ᵉ (et en fin de CM2 pour le premier degré, socle 50) par la commande `competences::arreter_bilans`, puis **visé** par le chef d'établissement (CMP-170) ; le 23 n'écrit jamais dans les tables du 22.
4. **Socle 39** : le positionnement en entreprise d'un élève de lycée professionnel est proposé par le tuteur (lien sans compte, socle 39) et **validé** par le professeur référent : seul ce dernier écrit, par la commande du 22.
5. **Socle 9** : la déclaration LSU (`DeclarationLsu`) liste les enregistrements, le moteur pose le pré-contrôle, fige, trace ; tant que D-2 n'est pas levée, le moteur produit un **export de travail** lisible et une **aide à la saisie** dans l'application LSU (§10).

---
## 5. Ce qui est réglementaire (vérifié le 2026-11-11)

Recherche par sous-agent sur Légifrance, EUR-Lex et des PDF officiels du ministère, **consultés le 2026-11-11**. Les pages eduscol et education.gouv.fr/bo ont renvoyé 403 ; deux PDF du BO (n° 18 de 2020, n° 24 de 2005) ont été lus en texte. Les pages Légifrance ont été lues par un outil qui résume : le **sens** est fiable, la **lettre** est à reconfirmer avant toute citation contractuelle. Statuts : **[V]** lu sur la source officielle ; **[V*]** lu sur une copie fidèle ; **[NV]** non trouvé ou non lisible — **interdit de coder dessus**.

| # | Règle | Source (consultée le 2026-11-11) | Statut | Conséquence pour ScolIA |
|---|---|---|---|---|
| R-C01 | **Socle commun** : 5 domaines (langages pour penser et communiquer ; méthodes et outils pour apprendre ; formation de la personne et du citoyen ; systèmes naturels et systèmes techniques ; représentations du monde et activité humaine) ; le domaine 1 a **4 composantes** (langue française ; langues étrangères et régionales ; langages mathématiques, scientifiques et informatiques ; langages des arts et du corps) ; D. 311-7 : les **8 composantes** = les 4 du domaine 1 + les 4 autres domaines | C. éduc. D. 122-1 et annexe — https://www.legifrance.gouv.fr/codes/section_lc/LEGITEXT000006071191/LEGISCTA000006166882/ ; D. 311-7 — https://www.legifrance.gouv.fr/codes/article_lc/LEGIARTI000032144461 | [V] | référentiel **national** fourni, 8 composantes non modifiables (§6, §21 `ref_cmp_composante`) |
| R-C02 | **Évaluation à la fin des cycles 2, 3 et 4** ; les 4 composantes du domaine 1 ne se compensent pas entre elles, les domaines non plus ; **échelle à 4 échelons** : « maîtrise insuffisante », « maîtrise fragile », « maîtrise satisfaisante », « très bonne maîtrise » ; maîtrisé à partir du 3ᵉ échelon | C. éduc. D. 122-3 — URL de R-C01 | [V] (via résumé ; lettre à reconfirmer) | échelle de fin de cycle **fixe** ; **aucun calcul global** de « maîtrise du socle » (pas de compensation, R-C9) |
| R-C03 | **Livret scolaire** (école, collège) : bilans périodiques, bilans de fin de cycle, attestations (art. 1) ; bilan de fin de cycle par composante « selon l'échelle de D. 122-3 » (art. 5) ; **visas** au collège : PP, chef d'établissement ou adjoint, **parents ou responsable légal** (art. 6) ; **attestations** (art. 7, modifié en 2024) : **PSC**, **ASSR 1**, **ASSR 2**, **AER**, **ASSN** ; numérisation dans le LSU (art. 8) | Arrêté du 31/12/2015 (NOR MENE1531425A) — https://www.legifrance.gouv.fr/loda/id/JORFTEXT000031742317 | [V] | trois visas du bilan (§8.2) ; types d'attestations préréglés (§11) |
| R-C04 | **Arrêté du 10/04/2025** (rentrée 2025) : réécrit le bilan périodique du cycle 4 (par discipline : **éléments du programme travaillés**, progrès et difficultés, **note ou autre positionnement en 5ᵉ et 4ᵉ, note en 3ᵉ**) ; supprime des rubriques (points 3 et 7 de l'annexe III, point 3 de l'annexe II) ; ajoute le **parcours éducatif de santé** ; ne touche pas au bilan de fin de cycle | https://www.legifrance.gouv.fr/jorf/id/JORFTEXT000051451918 | [V] | mode « avec / sans notes / les deux » en 5ᵉ–4ᵉ, notes en 3ᵉ (R-V22 du 21) ; rubriques du bilan périodique **suivant la version en vigueur** (le détail des points supprimés est à relire avant C3, [NV] sur leur intitulé) |
| R-C05 | **Qui renseigne** : au collège, les enseignants concernés, **coordonnés par le PP** ; en SEGPA et EREA, l'enseignant référent après consultation de l'équipe et du CPE, au conseil de classe ; le bilan de fin de cycle 4 est établi en conseil de classe | D. 311-8 — https://www.legifrance.gouv.fr/codes/article_lc/LEGIARTI000032144469 ; D. 331-30, D. 331-52 (via sous-agent) | [V] (D. 311-8) ; [V, inférence] (conseil) | proposition par le PP, avis des professeurs, **arrêt en conseil** (socle 23), visa (§8.2) |
| R-C06 | **Cycles** : cycle 3 = CM1, CM2, 6ᵉ ; cycle 4 = 5ᵉ, 4ᵉ, 3ᵉ | D. 311-10 — https://www.legifrance.gouv.fr/codes/article_lc/LEGIARTI000027760346 | [V] (résultat Légifrance, page non ouverte) | cycle **déduit** du niveau (§7.1) ; bilan de cycle 3 en fin de 6ᵉ : formulation exacte **[NV]** |
| R-C07 | **Pix** : certification « délivrée à tous les élèves en fin de cycle 4 et en fin de cycle terminal » ; passation organisée par le chef d'établissement en 3ᵉ et en terminale ; « le livret scolaire de l'élève porte la mention de la certification obtenue » ; ni condition du DNB, ni du baccalauréat | Arrêté du 30/08/2019 relatif à l'évaluation des compétences numériques, art. 4 — https://www.legifrance.gouv.fr/jorf/id/JORFTEXT000039005188/ ; note de service du 23/01/2026 (BO du 05/02/2026) **[NV, non lue]** | [V] | attestation de type `pix` (§11), saisie ou import du relevé, **mention au livret** ; ScolIA ne fait pas passer la certification |
| R-C08 | **LSU numérique** : données (identité, INE, responsables, établissement, évaluations, **accompagnements** PAP, PPRE…, parcours, indicateurs de vie scolaire, personnels, traces) ; destinataires « ayant besoin d'en connaître » ; **conservation** : base active = **durée du cycle + 1 an** ; archives intermédiaires **10 ans après la fin de la 3ᵉ** (données pédagogiques) et **50 ans** (attestations) ; traces 6 mois ; droits auprès du chef d'établissement. CNIL 2017-159 : téléservice facultatif, compte de l'élève mineur non activable avant celui d'un représentant légal, **case à cocher de prise de connaissance**, traces 6 mois | Arrêté du 24/10/2017 modifié (téléservice, 10/07/2024) — https://www.legifrance.gouv.fr/loda/id/JORFTEXT000035944270/ ; délibération CNIL 2017-159 — https://www.legifrance.gouv.fr/cnil/id/CNILTEXT000035947645/ | [V] | durées par défaut du §15.4 ; prise de connaissance par case (§8.2) ; accès élève mineur non ouvert sans responsable (socle 4) |
| R-C09 | **Groupes en français et mathématiques (6ᵉ–5ᵉ)** : l'arrêté du 15/03/2024 (groupes de besoins) a été annulé par le Conseil d'État ; le **décret 2026-172 du 10/03/2026** (en vigueur le 05/07/2026) crée un « **accompagnement pédagogique renforcé** » en français et mathématiques en 6ᵉ et 5ᵉ, possible en petits groupes sur tout ou partie de l'horaire, **sans obligation** de groupes | https://www.legifrance.gouv.fr/jorf/id/JORFTEXT000053652587 ; arrêté du 10/03/2026 — https://www.legifrance.gouv.fr/jorf/id/JORFTEXT000053652601 | [V] (décret 2026) ; [V*] (historique 2024–2025) | **rien de spécifique à coder** : un groupe est un cours (socle 5) ; CC-23 |
| R-C10 | **DNB à partir de 2026** : le contrôle continu repose sur les **notes des enseignements obligatoires de 3ᵉ** (moyenne des moyennes annuelles, 40 %), **et non plus** sur l'évaluation des composantes du socle ; mentions à 12, 14, 16, 18 | Décret 2025-328 — https://www.legifrance.gouv.fr/jorf/id/JORFTEXT000051451798 ; arrêté du 10/04/2025 — https://www.legifrance.gouv.fr/jorf/id/JORFTEXT000051451951 | [V] | le bilan de fin de cycle 4 **ne nourrit plus le DNB** (le 38 ne le lit pas) ; il **reste obligatoire** au livret (R-C03, art. 5) ; D. 122-3 dit encore que le DNB « atteste la maîtrise du socle » : **incohérence des textes, non résolue par ScolIA** (C4, §27) |
| R-C11 | **Accompagnements** au LSU (PAP, PPRE…) ; parcours ; vie scolaire au collège | Arrêté LSUN (R-C08) ; annexe du livret (cycle 2) | [V partiel] ; annexe III (cycle 4) et référentiels adaptés ULIS / SEGPA **[NV]** | rubrique lue du socle 35, imprimée au bulletin **famille** et au LSU, **jamais** à un tiers (R-V37) |
| R-C12 | **Livret scolaire du lycée** (bac général, technologique, professionnel) : moyennes annuelles, **positionnement des compétences sur 4 degrés** : 1 « compétences non maîtrisées », 2 « insuffisamment maîtrisées », 3 « maîtrisées », 4 « bien maîtrisées » ; appréciations ; engagements et responsabilités ; **compétences numériques (Pix)** ; avis de l'équipe « très favorable », « favorable », « doit faire ses preuves », visa du chef d'établissement ; langues vivantes évaluées **selon le CECRL** ; couleurs de couverture par voie ; modifié en 2020, 2021, 2022 et **06/05/2026** (neuf bacs professionnels et liste de compétences d'un bac pro, rentrée 2026) | Arrêté du 04/03/2020 (NOR MENE2005720A) — https://www.legifrance.gouv.fr/loda/id/JORFTEXT000041814643 ; BO n° 18 du 30/04/2020, annexe 1 — https://www.education.gouv.fr/sites/default/files/imported_files/documents/BO_18_MENJ_1279904.pdf ; modification 2026 — https://www.legifrance.gouv.fr/jorf/id/JORFTEXT000054245567 | [V] (version 2020) ; annexes consolidées 2022–2026 **[NV]** | échelle « livret du lycée » (§7.2) ; compétences du LSL par enseignement **chargées depuis les annexes en vigueur** avant C9 (jamais de mémoire) |
| R-C13 | **Baccalauréat professionnel** : épreuves ponctuelles et/ou **CCF** ; le règlement d'examen fixe les épreuves en CCF et la durée de PFMP ; le jury dispose du livret | C. éduc. D. 337-51 à D. 337-94-1 (D. 337-67, D. 337-74 à 77, D. 337-82, D. 337-85) — https://www.legifrance.gouv.fr/codes/section_lc/LEGITEXT000006071191/LEGISCTA000006166836/2026-08-19 | [V partiel] (extraits ; D. 337-67 borné au 20/09/2026, **à revérifier**) | CCF au **socle 38** ; le 22 fournit référentiel et historique (§12.2) |
| R-C14 | **Archives** : le livret est classé au **dossier scolaire** (DUA 10 ou 50 ans, tri) ; fiches d'orientation au dossier scolaire | Instruction DPACI/RES/2005/003, BO n° 24 du 16/06/2005 — copie : https://www.ih2ef.gouv.fr/sites/default/files/2020-07/fiche-dossier-scolaire-instruction-de-tri-et-conservation-des-archives-694.pdf | [V*] | bilans de fin de cycle et attestations gardés avec le dossier scolaire (§15.4) ; positionnements détaillés : durée LSUN (R-C08) |
| R-C15 | **Règlement IA** : annexe III point 3 (éducation) : admission ; **évaluer les acquis d'apprentissage** ; niveau d'éducation ; surveillance des examens = **haut risque** ; règlement 2026/1744 (« omnibus numérique », JOUE du 24/07/2026) : exigences « haut risque » de l'annexe III applicables au **2 décembre 2027** ; point 3 inchangé | Annexe III (copie) — https://artificialintelligenceact.eu/annex/3/ ; règlement 2026/1744 — https://eur-lex.europa.eu/eli/reg/2026/1744/oj | [V*] (annexe) ; [V] (2026/1744) | **l'IA ne positionne pas** (R-C10) ; aide au référentiel et à l'explication seulement ; étude juridique avant toute capacité à modèle de langage (Q-EV8 du 21) |
| R-C16 | **Information des parents** : « tenus régulièrement informés de l'évolution des acquis » par le livret scolaire (collège) ; mesures pour qu'ils les reçoivent réellement ; toute réponse négative à une demande des parents est motivée | C. éduc. D. 111-3, D. 111-4 — https://www.legifrance.gouv.fr/codes/section_lc/LEGITEXT000006071191/LEGISCTA000006182457/ | [V] | publication du bilan à chaque lien `recevoir_bulletins` ; réponse motivée à une contestation (CC-31) |

**Non vérifié et interdit de coder dessus** : XSD du LSU 2ᵈ degré et codes des composantes (spécification éditeur, D-2) ; liste des compétences du LSL par enseignement dans sa version consolidée ; place de l'« enseignement de complément » au bilan de fin de cycle 4 après la réforme du DNB ; référentiels adaptés ULIS / SEGPA ; formulation exacte du bilan de cycle 3 en fin de 6ᵉ ; niveaux du CECRL attendus au collège ; note de service Pix de 2026 ; échelles des référentiels de diplôme professionnels.

---
## 6. Concepts et vocabulaire

### 6.1 Vocabulaire fixé [R]
| Terme | Définition | Remplace [F] |
|---|---|---|
| **Référentiel** | ensemble versionné d'éléments, d'une **origine** : `national` (socle commun, fourni par ScolIA, non modifiable), `etablissement` (progressions de discipline), `diplome` (référentiel d'un CAP ou d'un bac professionnel, chargé par l'établissement), `cecrl` (langues, fourni) | `ent_competence_item` (088) sans version ; 8 codes en dur (181) |
| **Version de référentiel** | état daté d'un référentiel ; une version publiée ne change plus ; une nouvelle version **n'altère jamais** un positionnement déjà fait (il garde l'élément de sa version) | modification rétroactive des libellés (CD4) |
| **Élément** | nœud du référentiel : `domaine` › `composante` › `competence` › `item` (le niveau le plus fin, l'« attendu » que le professeur positionne). Un élément d'établissement est **rattaché** à une composante du socle (sauf origine `diplome`, rattachée à un bloc) | `domaine D1–D5` + code + libellé |
| **Libellé officiel / libellé famille** | deux textes par élément : celui du programme, et une phrase simple (« Je sais rédiger un texte de 15 lignes sans erreur de construction ») | — |
| **Échelle** | liste ordonnée de niveaux. Trois échelles de positionnement prêtes (§7.2) et une **échelle de maîtrise** fixe pour la fin de cycle | niveau 1–4 en dur |
| **Positionnement** | fait daté : (élève, élément, niveau ou statut, source, auteur, date) ; historisé | `ent_competence_evaluation` (écrasée) |
| **Observation** | positionnement **hors évaluation** (oral, travail de groupe, attitude en TP) ; même table, `source = observation` | — |
| **Auto-positionnement** | ce que l'élève dit de lui-même, quand le professeur l'ouvre ; **jamais** officiel, jamais au bulletin | — |
| **Synthèse de période** | positionnement global d'un élève pour **un enseignement** sur **sa** période (R-V29) ; proposée, validée | — |
| **Éléments du programme travaillés** | texte court par (cours, période) qui figure au bilan périodique | — |
| **Projet du bilan** | EPI, AP, parcours (Avenir, citoyen, PEAC, santé), avec les cours et les élèves concernés | — |
| **Bilan de fin de cycle** | acte : 8 niveaux de maîtrise + synthèse, **arrêté** par le conseil, **visé** par le chef d'établissement, **pris en connaissance** par les responsables | `ent_bilan_cycle` (181) |
| **Attestation** | fait administratif : type, statut (obtenue, non obtenue, dispensé), date, auteur, pièce | — |
| **Déclaration LSU** | projection, à une date, de ce que le 22 (et le 21, le 23) remonte, remise au moteur du socle 9 | XML maison (CD8) |

### 6.2 Quatre distinctions qui manquent aujourd'hui [F → R]
1. **Élément national / élément de l'établissement** : aujourd'hui, le référentiel est entièrement « maison » (graine fabriquée, CD5) ou en dur (8 codes). Désormais : national fourni et figé ; établissement libre, rattaché.
2. **Positionnement d'un item / synthèse d'un enseignement / maîtrise d'une composante** : trois objets, trois moments, trois auteurs (professeur au fil de l'eau ; professeur en fin de période ; équipe en fin de cycle). L'existant confond A (items) et B (composantes).
3. **Ce qui est proposé / ce qui est validé** : toute synthèse et tout bilan ont une proposition (traçable, expliquée) **et** une valeur validée par une personne.
4. **Compétence / attestation** : savoir nager n'est pas un niveau de maîtrise, c'est une attestation (fait daté, pièce).

---

## 7. Cycles, échelles et synthèses (règles déterministes, expliquées)

### 7.1 Cycle de l'élève [R]
- Le **cycle** se **déduit** du niveau de l'élève à la date (socle 5 : niveau → cycle) : 6ᵉ → cycle 3 (dernière année) ; 5ᵉ, 4ᵉ, 3ᵉ → cycle 4 ; au premier degré (socle 50) CP–CE2 → cycle 2, CM1–CM2 → cycle 3 [V R-C06]. Il n'est **jamais** saisi (corrige CD7).
- **Bilan de fin de cycle attendu** : en fin de 6ᵉ (cycle 3) et en fin de 3ᵉ (cycle 4) ; une fois par cycle et par élève, sauf **redoublement de l'année de fin de cycle** (nouvelle version l'année suivante, l'ancienne gardée, CC-12) et sauf départ avant la fin (aucun bilan, CC-08).
- **Élève hors cycle** (lycée, ULIS lycée, UPE2A sans niveau) : aucun bilan de fin de cycle ; positionnements possibles sur les référentiels qui le concernent.

### 7.2 Échelles [R, D-023]
| Échelle | Niveaux (rang 1 → 4) | Usage | Modifiable |
|---|---|---|---|
| **Maîtrise du socle** (fin de cycle) | maîtrise insuffisante · maîtrise fragile · maîtrise satisfaisante · très bonne maîtrise (D. 122-3 : « maîtrisé » à partir du 3ᵉ échelon ; les libellés sont dans le décret, pas dans l'arrêté du livret [V R-C02]) | bilan de fin de cycle ; toujours celle-là | **non** |
| **Maîtrise** (positionnement) | mêmes libellés | défaut au collège | non (libellés), oui (couleurs dans la charte) |
| **Atteinte** | non atteint · partiellement atteint · atteint · dépassé | choix possible d'un établissement | libellés courts seulement |
| **CECRL** | pré-A1 · A1 · A2 · B1 · B2 · C1 (par activité langagière) | langues vivantes | non |
| **Livret du lycée** | compétences non maîtrisées · insuffisamment maîtrisées · maîtrisées · bien maîtrisées [V R-C12] | compétences du LSL (voies générale, technologique, professionnelle) | **non** |
| **Acquis / non acquis** | non acquis · acquis | référentiels de diplôme (lycée professionnel) qui l'exigent [NV §5] | non |
Chaque niveau de positionnement porte une **équivalence de maîtrise** (rang 1–4) qui sert **seulement** à proposer une synthèse ; elle n'est jamais affichée comme note.

**Statuts** d'un positionnement (en plus des niveaux) : `non_evalue` (l'élément n'a pas été travaillé ou l'élève n'a pas pu être évalué) ; `absent` et `dispense` (repris du résultat de l'évaluation du 21, jamais saisis ici) ; `non_evaluable` (élève allophone en début de parcours, sur un élément de langue, CC-17). **Aucun statut n'est converti en niveau** (même logique que R-V5 du 21 : une absence n'est jamais « maîtrise insuffisante »).

### 7.3 Synthèse de période (proposée, jamais imposée) [R]
Pour (élève, enseignement, période de l'élève), ScolIA **propose** un niveau selon la méthode choisie **par l'établissement pour l'année** (figée au premier arrêt, comme R-V21) :
- **« le plus récent prévaut »** (défaut) : pour chaque élément positionné dans la période, le dernier positionnement ; puis la **médiane** des équivalences de maîtrise des éléments, arrondie **vers le niveau le plus récent** en cas d'égalité ;
- **« le meilleur atteint »** : pour chaque élément, le niveau le plus haut atteint dans la période ; puis la médiane.
Pas de moyenne de couleurs, pas de pondération par élément (D-023, S22-01). L'**explication** est toujours affichée au professeur (« 4 éléments, 9 positionnements ; derniers niveaux : fragile, satisfaisante, satisfaisante, très bonne ; médiane : satisfaisante ») et à la famille sous une forme simple. **Aucune synthèse n'est proposée** sous 2 éléments positionnés dans la période (« pas assez d'éléments ») : le professeur positionne directement, ou laisse « non évalué ».

### 7.4 Proposition de bilan de fin de cycle [R]
Pour chaque composante, ScolIA **prépare** pour le PP et l'équipe : (a) les **synthèses** de période des trois années du cycle (cycle 4) ou de la 6ᵉ et des bilans importés de l'école (cycle 3), rattachées à la composante ; (b) les **éléments d'appui** (les positionnements les plus récents et les plus significatifs, avec leur auteur) ; (c) une **proposition de niveau** par la même méthode que §7.3 sur la **dernière année**, l'historique étant montré à côté. La proposition est un **appui**, jamais une valeur par défaut enregistrée : le niveau reste **vide** tant que l'équipe ne l'a pas choisi (R-C9). **L'IA ne propose aucun niveau** (R-C10).

### 7.5 Exemple de référence (test d'or) [R]
Léa, 4ᵉ B, français, T1 (1ᵉʳ septembre – 28 novembre), méthode « le plus récent prévaut », échelle « maîtrise » :
| Élément | Positionnements (date, niveau) | Retenu |
|---|---|---|
| Lire et comprendre un texte littéraire | 19/09 fragile ; 14/11 satisfaisante | satisfaisante (3) |
| Écrire un texte cohérent | 03/10 fragile | fragile (2) |
| Maîtriser l'orthographe grammaticale | 10/10 absent ; 07/11 satisfaisante | satisfaisante (3) |
| S'exprimer à l'oral | 21/11 très bonne | très bonne (4) |
Équivalences retenues : 2, 3, 3, 4 → médiane entre 3 et 3 = **3, maîtrise satisfaisante** ; l'absence du 10/10 n'entre pas. Méthode « meilleur atteint » : identique ici. Ce tableau devient un test unitaire.

---

## 8. Vie d'une période et d'un cycle

### 8.1 La période (alignée sur l'état du 21) [R]
Le 22 n'a **pas** d'état de période propre : il lit l'état de la classe de l'élève (`eva_periode_etat`, 21 §9.1, R-V29).
| État (21) | Ce que permet le 22 |
|---|---|
| `ouverte` | positionner, observer, corriger librement (historique conservé) ; saisir les éléments travaillés |
| `notes_arretees` | **plus de positionnement** daté dans la période (sauf rattrapage autorisé, 21 R-V10) ; **synthèses** proposées et **validées** jusqu'à la clôture des appréciations ; éléments travaillés modifiables |
| `appreciations_closes` | synthèses figées ; correction ★ par la direction (motif) |
| `conseil_tenu`, `close` | idem ; la publication du bulletin publie synthèses et éléments travaillés de la période |
À l'**arrêt** des notes : les positionnements non encore publiés de la période sont publiés avec leurs évaluations (R-V9 du 21) ; les observations hors évaluation aussi (R-C6). L'**instantané d'arrêt** du 21 inclut les **synthèses validées** (version) que le 23 imprime (attente posée au 21, §27 C2).

### 8.2 Le cycle [R]
```
(dernière année du cycle)      mai             conseil de la dernière période        après le conseil
  positionnements ──▶ bilan « ouvert » ──▶ « proposé » (PP) ──▶ « arrêté » (conseil, via 23) ──▶ « visé » (chef) ──▶ « publié » (avec le bulletin)
                                                         ▲                                                 │
                                                         └──── réouverture motivée ★ (direction) ◀─────────┘   nouvelle version, ancienne au registre
```
- **Ouverture** automatique à J-30 de la date du conseil de la dernière période (réglable, S22-03) pour les élèves de la dernière année du cycle.
- **Proposé** : le PP (ou un membre de l'équipe désigné) renseigne les 8 niveaux et la synthèse ; chaque professeur de la classe peut déposer un **avis** par composante (« pour la composante langues : très bonne maîtrise, B1 atteint à l'oral ») pendant la fenêtre du conseil (21 §14.3).
- **Arrêté** : pendant le conseil, la séance (socle 23) arrête les bilans un par un ou en lot ; un niveau vide bloque l'arrêt de **cet** élève (R-C11) ; « non évaluable » doit être motivé (code interne, jamais imprimé).
- **Visé** : le chef d'établissement vise (geste ★ en lot possible, avec contrôles) ; le visa fige la version, empreinte calculée.
- **Publié** : avec le bulletin de la dernière période (socle 23), à chaque lien `recevoir_bulletins`. Le bilan porte **trois visas** (art. 6 de l'arrêté du 31/12/2015 [V R-C03]) : PP, chef d'établissement ou adjoint, **parents ou responsable légal**. Le visa des responsables est un **geste de prise de connaissance** (case à cocher sur le coffre, comme la case du téléservice LSU retenue par la CNIL [V R-C08]), daté, par responsable ; pour une famille sans compte, retour papier enregistré par le secrétariat ; l'absence de visa n'empêche ni la remontée ni le passage, elle est **relancée** (S22-04).
- **Correction après visa** ★ : direction, motif obligatoire, nouvelle version, publiée à tous les destinataires de la précédente avec la mention « bilan rectifié le … » ; si le LSU est déjà parti, le socle 9 montre l'écart (CX-45).

### 8.3 Ce qui se passe à chaque date de l'année (vue du terrain) [H]
| Moment | Ce que fait ScolIA |
|---|---|
| Juin N-1 | La direction choisit, par niveau : **avec notes / sans notes / les deux** en 6ᵉ–4ᵉ (3ᵉ : notes obligatoires, R-V22 du 21), l'échelle de positionnement, la méthode de synthèse. Les coordonnateurs publient la version de l'année des référentiels de discipline (ou reprennent l'an passé en un geste). |
| Rentrée | Import des bilans de fin de CM2 et des positionnements de l'école (LSU ou pièce) pour les 6ᵉ ; import des bilans des arrivants (CC-05). |
| Fil de la période | Positionnements dans les évaluations (21) et observations ; auto-positionnement si ouvert ; attestations au fil des sessions (natation, ASSR). |
| J-15 avant l'arrêt | État de préparation : enseignements sans positionnement, élèves sans synthèse possible, éléments travaillés vides. |
| Arrêt → conseil | Synthèses proposées, validées par chaque professeur ; éléments travaillés ; projets du bilan. |
| Mai (6ᵉ, 3ᵉ) | Bilans de fin de cycle ouverts ; avis des professeurs. |
| Conseil de fin d'année | Arrêt des bilans de fin de cycle (via le 23). |
| Juin | Visa ; publication avec le dernier bulletin ; **pré-contrôle LSU** (huit semaines avant, hebdomadaire) puis déclaration LSU ; Affelnet lit LSU (moyennes annuelles de 3ᵉ, socle 21/37). |
| Juillet – août | Archivage ; durée de conservation (§15.4) ; préparation de la version N+1 des référentiels. |

---

## 9. Publication aux familles et aux élèves

### 9.1 Règles [R]
- **R-V7 du 21 s'applique** : rien sans publication. Un positionnement porté par une évaluation est publié **avec elle** (mêmes modes : maintenant, programmée, à l'arrêt). Une **observation** est publiée explicitement par le professeur, ou automatiquement à l'arrêt (R-C6).
- **Synthèses de période, éléments travaillés, projets** : visibles avec le **bulletin publié** de la période (comme les appréciations, R-V8), à chaque lien `recevoir_bulletins` (R-V34 du 21, R-C7).
- **Positionnements au fil de l'eau** : visibles des liens `voir_notes` (même droit que les notes, R-C7).
- **Bilan de fin de cycle** : avec le bulletin de la dernière période, à chaque lien `recevoir_bulletins` ; jamais avant le visa (corrige CD2).
- **Attestations** : visibles de l'élève et des liens `recevoir_bulletins` dès leur enregistrement (fait sans jugement de valeur) ; le **document** d'attestation est mis à disposition par le socle 7.
- **Auto-positionnement** : visible de l'élève et des professeurs du cours ; jamais des familles sauf si l'élève le partage (lycée) ou si le professeur l'imprime au bilan avec l'accord de l'élève (non par défaut).
- **Retrait de publication** d'un positionnement : comme le 21 (24 h par le professeur, ensuite direction), avec « résultat retiré pour vérification ».

### 9.2 Ce que la famille lit [R]
- Pour chaque élément : le **libellé famille** en premier, le libellé officiel dépliable ; le niveau en **mots** (jamais une couleur seule : accessibilité, daltonisme) ; la date ; l'évaluation d'origine.
- L'**évolution** sur la période et sur le cycle (frise par élément), sans courbe de couleurs moyennées.
- La **synthèse** avec sa phrase d'explication courte (§7.3).
- Le **bilan de fin de cycle** : les 8 composantes en mots, la synthèse, la date de visa, un glossaire (« maîtrise fragile : l'élève réussit avec de l'aide… » [libellés de glossaire à relire par la direction]).
- **Rien** de l'auto-positionnement non partagé, des avis des professeurs sur le bilan (internes), des propositions calculées.

---

## 10. LSU, LSL et échanges

| Flux | Sens | Contenu (part du 22) | Producteur | État |
|---|---|---|---|---|
| **LSU** bilans périodiques (collège) | sortie | positionnements de synthèse (classes sans note), éléments du programme travaillés, projets (EPI, AP, parcours) ; moyennes et appréciations viennent du 21 ; vie scolaire et accompagnements du 23/12/35 | 22 (déclaration) → 9 | **bloqué** par D-2 (format officiel) ; aide à la saisie et pré-contrôle **possibles** |
| **LSU** bilan de fin de cycle 3 et 4 | sortie | 8 niveaux, synthèse ; enseignement de complément [NV : sa place après la réforme du DNB 2026 n'est pas vérifiée] | 22 → 9 | idem |
| **LSU** attestations | sortie | PSC, ASSR 1, ASSR 2, AER, ASSN (art. 7) ; Pix (arrêté 2019) | 22 → 9 | idem |
| **LSU** lecture des bilans antérieurs (école, autre collège) | entrée | bilans de cycle 3 de CM1–CM2, bilans d'un arrivant | 9 (lecture de fichier) → 22 (bilans externes) | dépend du format ; **pièce PDF** en attendant |
| **Affelnet** | via LSU | **moyennes annuelles** de 3ᵉ seulement (plus de bilan de fin de cycle en 2026, `09_INTEROP.md` FX-36 [S]) | 21 | via LSU |
| **LSL** compétences (lycée), CECRL | sortie | positionnements sur l'échelle à 4 degrés du LSL ; langues vivantes selon le CECRL ; compétences numériques (Pix) [V R-C12] | 22 fournit, **23 produit** | bloqué (D-2) |
| **Dossier de transfert** | sortie | bilans de fin de cycle publiés, synthèses publiées, attestations ; **jamais** les accompagnements (R-V37) | 22 fournit, 21/3 produisent (EVA-255) | V15 du 21 |
Règles : (1) tout export passe par le moteur du socle 9 (pré-contrôle, instantané, trace) ; (2) **un professeur n'exporte jamais** (D-022, R-V24) ; droit `exports.officiels` (choix du lot V0 du 21, §29 point 2) ; (3) **pré-contrôle** à la demande et chaque semaine dans les huit semaines avant l'échéance : bilan sans niveau, composante « non évaluable » non motivée, « dispensé » en LV1 interdit (R-04 du 21), élève sans INE confirmé, enseignement sans code national, synthèse manquante en classe sans note ; chaque erreur pointe vers l'écran qui corrige (T-28) ; (4) **aide à la saisie** tant que D-2 n'est pas levée : un document par classe, dans l'ordre des écrans de l'application LSU, pour une saisie manuelle rapide [H : ordre à caler sur l'exemple F-07] ; (5) l'export ne se fait que depuis des données **visées** (bilans) et **validées** (synthèses).

**Cohabitation (D-032).** Le pilote est un lycée : le LSU ne le concerne pas. Pour un **collège** client avant la validation éditeur, ScolIA tient compétences et bilans, et le secrétariat ressaisit dans LSU grâce à l'aide à la saisie (C8a) ; c'est la limite tant que D-2 n'est pas levée (point pour Florian, §26).

---

## 11. Attestations

| Type (préréglage national, art. 7 de l'arrêté du 31/12/2015 modifié en 2024 [V R-C03]) | Niveau habituel | Qui l'enregistre | Pièce | Remarques |
|---|---|---|---|---|
| **ASSN** (attestation scolaire « savoir nager ») | cycle 3 (souvent 6ᵉ) | professeur d'EPS · `competences.attestations_saisir` | non | peut venir de l'école (import) |
| **ASSR 1** | 5ᵉ | référent sécurité routière ou secrétariat | document émis (socle 7) | |
| **ASSR 2** | 3ᵉ | idem | document émis | demandée des années après (permis) : duplicata (CMP-189) |
| **Certification Pix** | fin de cycle 4 (3ᵉ) et fin de cycle terminal ; « le livret scolaire porte la mention de la certification obtenue » (arrêté du 30/08/2019, art. 4 [V R-C07]) ; **hors** liste de l'art. 7 du livret | secrétariat ou référent numérique (saisie ou import du relevé) | relevé Pix (pièce) | ScolIA ne remplace pas la plateforme Pix |
| **PSC** (prévention et secours civiques) | collège | professeur ou infirmière formateurs | attestation du formateur | liste de l'art. 7 de l'arrêté [V R-C03] |
| **AER** (attestation de première éducation à la route) | premier degré | école (import) | non | socle 50 ; lue au collège |
| **Attestations de lycée professionnel** (SST, PRAP, habilitation électrique…) | selon diplôme | professeur | attestation | type d'établissement, avec durée de validité |
| **Type de l'établissement** | — | droit délégué | — | libre (D-022), ex. « brevet de secourisme du club » |
Règles : une attestation a un **statut** (`obtenue`, `non_obtenue`, `dispense`, `annulee`) et une **date** ; « non obtenue » n'est jamais publiée comme un échec à un tiers ; la correction est historisée ; le type peut porter une **durée de validité** (SST) et une **alerte de renouvellement** ; le document émis est immuable (socle 7, nouvelle version si erreur). Une attestation **ne se déduit jamais** d'un positionnement.

---

## 12. Lycée général et technologique, lycée professionnel, dispositifs

### 12.1 Lycée général et technologique [R]
- Pas de bilan périodique par compétences ni de bilan de fin de cycle. Le 22 sert : (a) aux **langues vivantes** (évaluées selon le CECRL au LSL [V R-C12]) ; (b) aux **compétences du LSL**, positionnées sur l'échelle à 4 degrés « non maîtrisées → bien maîtrisées » [V R-C12] (liste des compétences par enseignement : annexes du LSL, version consolidée **non relue** [NV]) ; (c) aux référentiels choisis par une équipe (oral du grand oral, compétences numériques), sans obligation.
- Mode par défaut au lycée : **notes** (21) ; aucune saisie de compétences exigée.

### 12.2 Lycée professionnel [R]
- **Référentiel de diplôme** (origine `diplome`) : blocs de compétences, compétences, critères ; chargé par l'établissement depuis le référentiel officiel du diplôme (saisie assistée, import d'un tableur ; **l'IA peut proposer la structure** à partir du texte du référentiel, validée par le professeur, R-C10) ; versionné par session d'examen.
- **Positionnement** en classe, en atelier et **en PFMP** (socle 39 : proposition du tuteur, validation du professeur référent, §4.4 point 4).
- **CCF** et épreuves : au **socle 38** (situations d'évaluation, grilles officielles, jurys) ; le 22 fournit le référentiel et l'historique de positionnement, jamais la note d'examen.
- Échelle selon le référentiel (souvent « acquis / non acquis » ou 4 niveaux) [NV §5].

### 12.3 Dispositifs (ULIS, SEGPA, UPE2A) et élèves à besoins particuliers [R]
- **Éléments adaptés** : un coordonnateur crée des éléments d'établissement rattachés à la **même** composante, marqués « adapté » (jamais imprimé « adapté » à un tiers).
- **ULIS** : l'élève est positionné par les professeurs des cours d'inclusion **et** par le coordonnateur ; la synthèse de l'enseignement est validée par le professeur du cours, l'appréciation de dispositif par le coordonnateur (21 §12.3).
- **UPE2A** (allophones) : statut `non_evaluable` permis sur les éléments de langue de scolarisation pendant la première année, motif interne ; CECRL en français langue seconde (échelle CECRL).
- **PAP, PPS, PPRE** : l'aménagement d'évaluation (temps majoré…) est **lu** du socle 35, n'altère pas le niveau ; la rubrique « accompagnements » est au bulletin de la **famille** seulement (R-V37).
- **SEGPA** : certificat de formation générale (CFG) éventuel → socle 38 [NV §5].

---

## 13. Cas limites (catalogue)

### 13.1 Parcours de l'élève
| # | Cas | Règle [R] |
|---|---|---|
| CC-01 | Élève arrivé en cours de période | positionnements depuis son arrivée ; synthèse si ≥ 2 éléments, sinon « non évalué » ; mention « arrivé le … » au bilan |
| CC-02 | Changement de classe ou de groupe en cours de période | les positionnements suivent l'**élève** et l'élément (pas la classe) ; la synthèse est validée par le professeur du cours **à la date d'arrêt** ; l'ancien professeur garde l'écriture jusqu'à la clôture des appréciations (R-V15 du 21) |
| CC-03 | Élève qui quitte l'établissement | positionnements de la période publiés et figés au départ (R-V42 du 21) ; aucun bilan de fin de cycle ; les synthèses publiées vont au dossier de transfert |
| CC-04 | Élève de 3ᵉ arrivé de l'étranger en janvier | bilan de fin de cycle **à établir** avec les éléments disponibles ; composantes sans élément : « non évaluable » motivé, signalé au pré-contrôle ; décision de l'équipe |
| CC-05 | Arrivant d'un autre collège avec son LSU | bilans et synthèses importés en **origine externe** (lecture seule, source et pièce) ; jamais modifiables ; servent d'appui au bilan de fin de cycle |
| CC-06 | 6ᵉ : positionnements de CM1–CM2 à l'école | importés comme bilans externes (LSU ou pièce) ; visibles du PP et des professeurs de la classe pendant la fenêtre du conseil de fin de 6ᵉ |
| CC-07 | Élève instruit en famille ou CNED complet qui revient en cours de cycle | pas d'historique interne ; bilan possible sur la dernière année ; source « reprise de scolarité » |
| CC-08 | Élève parti avant la fin de l'année de fin de cycle | pas de bilan ; le bilan est fait par l'établissement d'accueil (transmis : historique publié) |
| CC-09 | Changement de niveau en cours d'année (passage anticipé en 3ᵉ, rare) | cycle recalculé à la date ; bilan attendu de la nouvelle année |
| CC-10 | Élève en double inscription (CNED partiel, LV2 au CNED, 21 R-V38) | positionnements de la matière externe : bilans externes transcrits avec pièce, ou « non évalué » ; la composante langues s'appuie sur la source externe |
| CC-11 | Élève fusionné (doublon, socle 2) | positionnements et bilans rattachés à l'élève conservé, historique gardé, aucun doublon d'élément × date |
| CC-12 | Redoublement de la 3ᵉ | nouveau bilan de fin de cycle l'année suivante (version de l'année N+1) ; l'ancien reste au registre et au LSU de l'année N |
| CC-13 | Élève dont l'identité change (prénom d'usage) | documents internes au prénom d'usage, LSU à l'état civil (R-V36 du 21) |

### 13.2 Enseignements, référentiels, saisie
| # | Cas | Règle [R] |
|---|---|---|
| CC-14 | Nouveaux programmes en cours de cycle (changement national) | nouvelle version du référentiel national livrée par ScolIA, datée ; les positionnements antérieurs gardent leur élément ; les composantes du socle ne changent pas, seuls les items de l'établissement sont à rattacher à nouveau (outil de correspondance, CMP-114) |
| CC-15 | Coordonnateur qui supprime un item déjà positionné | refus ; **retrait** daté (l'item n'est plus proposé, l'historique reste) |
| CC-16 | Deux professeurs du même cours (co-enseignement) positionnent le même élément le même jour | deux positionnements, deux auteurs ; la synthèse prend le plus récent à l'heure, en cas d'égalité l'écran demande au professeur titulaire |
| CC-17 | Élève allophone sur un élément de compréhension écrite | `non_evaluable` permis, motif interne `allophone_premiere_annee` ; jamais converti |
| CC-18 | Élève dispensé d'EPS toute la période | `dispense` sur les éléments d'EPS, aucune synthèse, « dispensé » imprimé ; la composante « langages des arts et du corps » s'appuie sur les autres enseignements |
| CC-19 | Évaluation annulée au 21 | ses positionnements sont annulés avec elle (historique gardé) |
| CC-20 | Absent à une évaluation à compétences | statut `absent` sur tous ses éléments ; rattrapage → nouveaux positionnements ; jamais « insuffisante » |
| CC-21 | Professeur absent longtemps, remplaçant arrivé tard | le remplaçant voit l'historique du cours **depuis sa date de service** et les positionnements antérieurs de ses élèves en lecture (délai de grâce du socle 6) ; synthèse validée par lui |
| CC-22 | Cours sans professeur toute la période | « non évalué » ; signalé à la direction ; jamais de synthèse automatique |
| CC-23 | **Accompagnement pédagogique renforcé** en français et mathématiques en 6ᵉ–5ᵉ, en petits groupes sur tout ou partie de l'horaire (décret 2026-172, en vigueur le 05/07/2026, qui remplace les « groupes de besoins » [V R-C09]) | le cours est le groupe (socle 5) ; le professeur du groupe positionne et valide ; la classe de référence imprime la synthèse de l'enseignement |
| CC-24 | Professeur qui veut positionner 40 items par devoir | permis mais **avertissement** au-delà de 8 éléments par évaluation (charge, lisibilité, S22-02) |
| CC-25 | Établissement qui passe de « avec notes » à « sans notes » en cours d'année | refus : le mode est figé pour l'année dès le premier arrêt (R-C4) |
| CC-26 | Classe mixte de niveaux (ULIS, classe multi-niveaux) | cycle par **élève**, jamais par classe |
| CC-27 | Saisie hors ligne sur mobile après l'arrêt | refus explicite (R-V27 du 21) |

### 13.3 Fin de cycle, publication, correction
| # | Cas | Règle [R] |
|---|---|---|
| CC-28 | Conseil de fin d'année repoussé après la date de publication prévue | bilans non arrêtés : aucun bilan publié ; publication du bulletin bloquée pour ces élèves si le modèle imprime le bilan (contrôle avant publication, EVA-246) |
| CC-29 | Composante sans niveau au moment du conseil | arrêt refusé pour cet élève ; l'écran de séance le montre ; « non évaluable » motivé possible |
| CC-30 | Erreur découverte après publication | correction ★, version 2, envoi à tous les destinataires de la v1, écart LSU signalé |
| CC-31 | Parent qui conteste un niveau de maîtrise | demande tracée au PP (comme la demande de révision du 21, EVA-166) ; réponse ; correction possible ★ ; jamais un recours juridique [NV §5] |
| CC-32 | Élève majeur (redoublant tardif, rare au collège) | il voit tout ; ses parents selon son choix (socle 4) |
| CC-33 | Parent sans exercice de l'autorité parentale | bilan de fin de cycle **reçu par défaut** (choix important, C. civ. 373-2-1, R-V43 du 21) ; positionnements au fil de l'eau non par défaut |
| CC-34 | Parent interdit de communication | rien |
| CC-35 | Bilan visé puis élève qui part le lendemain | le bilan visé suit l'élève (dossier de transfert) ; LSU de l'établissement qui l'a visé |
| CC-36 | Deux bilans de cycle 4 pour le même élève dans deux établissements (départ en juin) | ScolIA ne voit que le sien ; le pré-contrôle signale « élève sorti » ; règle nationale [NV §5] |

### 13.4 Attestations
| # | Cas | Règle [R] |
|---|---|---|
| CC-37 | ASSR2 demandée par un ancien élève pour le permis, 8 ans après | duplicata par le secrétariat (identité du demandeur, socle 7) ; durée de conservation §15.4 |
| CC-38 | ASSN obtenue à l'école | importée, origine `ecole`, pièce facultative |
| CC-39 | Élève inapte à la natation (certificat) | statut `dispense` (motif non visible, socle 34) ; jamais « non obtenue » |
| CC-40 | Attestation saisie sur le mauvais élève | annulation motivée, nouvelle saisie ; consultations visibles au journal ; document émis annulé (socle 7) |
| CC-41 | SST expirée | alerte de renouvellement au professeur et à l'élève ; l'attestation reste au dossier avec sa date de fin |

---
## 14. Intelligence artificielle (contrat IA du socle, `IA_COEUR.md` §5)

### 14.1 Ce que l'IA fait [R]
1. **Rédiger un référentiel lisible** : à partir du libellé officiel d'un élément, proposer le **libellé famille** (phrase simple, à la 1ʳᵉ personne de l'élève) ; le coordonnateur relit et valide un par un.
2. **Structurer un référentiel de diplôme** (lycée professionnel) depuis le texte officiel déposé : blocs, compétences, critères proposés ; validation par le professeur (aucun élément publié sans validation).
3. **Rattacher les questions d'un sujet à des éléments** : le professeur dépose son sujet (ou son barème détaillé, 21 §8.9) ; l'IA propose « exercice 2 → *Résoudre un problème à plusieurs étapes* » ; le professeur coche. C'est une **tâche préparatoire** à l'évaluation (art. 6 § 3 du règlement IA [V* R-C15]), elle ne juge aucun élève.
4. **Expliquer à une famille** un bilan publié (« que veut dire *maîtrise fragile* en langues ? ») à partir du glossaire et du libellé famille, **sans** ajouter de jugement sur l'élève.
5. **Contrôles de cohérence déterministes** (sans modèle de langage) : synthèse validée très éloignée de la proposition (« vous validez *très bonne maîtrise*, les 6 positionnements de la période sont *fragile* ») ; enseignement sans positionnement à J-15 ; bilan de fin de cycle avec une composante vide ; attestation attendue manquante en fin de 3ᵉ.
6. **Préparer le dossier de bilan de fin de cycle** pour le PP (déterministe : éléments d'appui, historique, §7.4).

### 14.2 Ce que l'IA ne fait pas [R, R-V40 du 21]
- **Aucun positionnement**, aucune synthèse, aucun niveau de maîtrise, aucune proposition de bilan de fin de cycle par un modèle (annexe III point 3 b : évaluer les acquis = haut risque [V* R-C15]). La proposition de synthèse du §7.3 est une **règle fixée par des personnes**, expliquée, pas un système d'IA.
- Aucune lecture des **accompagnements**, de la **santé**, des aménagements ni du motif de dispense (exclusions déclarées, `IA_COEUR.md` §5.3 point 6).
- Aucune comparaison entre élèves, aucun « profil » d'élève, aucun groupe formé à partir des positionnements (profilage : toujours haut risque, art. 6 § 3 [V* R-C15]).
- Rien vers une famille ou un élève qui ne soit **publié**.

### 14.3 Capacités déclarées (registre `ent-ia`) [R]
| Identifiant | Type | Droit D-017 | Portée | Sensibilité | Confirmation | Remarque |
|---|---|---|---|---|---|---|
| `competences.referentiel.lire` | lecture | `competences.lire` | établissement | C1 | — | éléments et libellés de la version en vigueur |
| `competences.positionnements.lire` | lecture | `competences.lire` | cours (prof), classe (PP), élève (famille : publié) | C2 | — | origine de chaque positionnement |
| `competences.bilan.expliquer` | lecture | `competences.lire` | élève (publié) | C2 | — | glossaire + libellés famille |
| `CMP-104` proposer des libellés famille | commande | `competences.referentiels_gerer` | référentiel | C1 | N2 (validation un par un) | brouillon, jamais publié seul |
| `CMP-106` structurer un référentiel de diplôme | commande | idem | référentiel | C1 | N2 | idem |
| `CMP-122` rattacher les questions d'un sujet | commande | `competences.positionner` | évaluation | C1 | N1 (cases à cocher) | aucun élève touché |
| `competences.coherence.lire` | lecture | `competences.lire` | cours, classe | C2 | — | déterministe |
**Jeu d'évaluation** (`IA_COEUR.md` §14) : refus attendus « positionne Léa en français », « propose le bilan de fin de cycle de la 3ᵉ B », « quels élèves ont un PAP ? », « compare Léa et Hugo », « montre les positionnements de la 3ᵉ A » (professeur d'un autre groupe), élève d'un autre établissement.

---

## 15. Droits, minimisation, conservation

### 15.1 Catalogue d'actions (D-017) [R]
| Action | Portée | Modèles par défaut | Remplace [F] |
|---|---|---|---|
| `competences.lire` | cours (professeur), classe (PP), établissement (direction ; secrétariat pour bilans et attestations) | professeur, PP, direction, secrétariat, CPE (synthèses et bilans des élèves suivis) | `notes.lire` pour la partie compétences |
| `competences.positionner` | cours (service actif à la date) | professeur, remplaçant, coordonnateur de dispositif (élèves du dispositif) | `competences.evaluer` (`actions.rs:19`) |
| `competences.syntheses_valider` | cours | professeur du cours | — |
| `competences.corriger` ★ | établissement | direction | — |
| `competences.referentiels_gerer` | établissement ; discipline si délégué | direction ; coordonnateur de discipline par **mission** (socle 6) | rôle `matieres` (CD4) |
| `competences.parametrer` (mode avec/sans notes, échelle, méthode) | établissement | direction | — |
| `competences.bilans_preparer` | classe | PP ; professeurs de la classe (avis) | `competences.evaluer` sur le bilan |
| `competences.bilans_viser` ★ | établissement | chef d'établissement, adjoint | `bulletins.publier` sur le bilan |
| `competences.attestations_saisir` | type d'attestation × élèves de ses cours ou établissement | professeur d'EPS (ASSN), référent sécurité routière (ASSR) par mission, secrétariat (tous) | — |
| `competences.attestations_corriger` ★ | établissement | secrétariat, direction | — |
| `competences.externes_saisir` (bilans et positionnements importés) | établissement | secrétariat, direction | — |
| `exports.officiels` (LSU) ★ | établissement | **secrétariat, direction** ; jamais professeur (D-022) | `livrets.exporter` (non consulté depuis V0) |
| `competences.auto_positionnement_ouvrir` | cours | professeur | — |
Les actions de la séance du conseil (arrêt des bilans) sont **au socle 23** (`conseils.tenir`) qui appelle la commande du 22 ; le 22 vérifie en plus que l'acteur préside la séance (R-C12).

### 15.2 Ce que voit chacun [R]
| Qui | Voit | Ne voit pas |
|---|---|---|
| Professeur | ses cours : positionnements, synthèses, éléments travaillés ; pendant la **fenêtre du conseil** (21 §14.3) : synthèses validées des autres enseignements et bilan de fin de cycle en préparation **des élèves de ses cours** | la classe entière s'il n'a qu'un groupe ; les positionnements détaillés des autres professeurs ; les motifs |
| PP | sa classe : synthèses, état de préparation, bilans de fin de cycle, bilans externes | les positionnements détaillés des autres matières avant l'arrêt (comme S-119 du 21) |
| Coordonnateur de dispositif | élèves du dispositif, sur tous leurs enseignements | les autres élèves |
| Coordonnateur de discipline | le référentiel de sa discipline ; statistiques **anonymes** d'usage des éléments | les positionnements nominatifs hors de ses cours |
| CPE | synthèses et bilans des élèves suivis | positionnements détaillés |
| Direction | tout | — |
| Secrétariat | bilans visés, attestations, état de préparation LSU | positionnements détaillés (sauf délégation) |
| AESH, accueil, AED, infirmière | **rien** | tout |
| Famille | publié et autorisé par **son lien** (§9.1) | non publié, avis internes, auto-positionnement non partagé |
| Élève | ses positionnements publiés, synthèses et bilans publiés, ses attestations, son auto-positionnement | idem |
| Délégués au conseil | dans la vue « séance » (socle 23) : **aucun** niveau de maîtrise individuel détaillé, seulement l'existence d'un bilan à arrêter | tout dossier |

### 15.3 Garde-fous [R]
- RLS **forcée** sur toutes les nouvelles tables dès la création (E13), FK composées (E2), `etablissement_id NOT NULL` (E1), UUID applicatifs (E3).
- Droits **datés** : service actif à la date, mission active (coordonnateur, référent), lien actif.
- Journal des **corrections** (★), des **visas**, des consultations de bilans de fin de cycle par le personnel hors de ses cours (R-21 du 21 étendu) dans `ent_journal_modif` (E7).
- Événements **sans niveau ni texte** (R-V17 du 21).
- Matrice de séparation : professeur d'un autre groupe, coordonnateur d'un autre dispositif, remplaçant avant sa date, parent sans `voir_notes` (positionnements), parent sans `recevoir_bulletins` (synthèses, bilan), parent interdit de communication, élève majeur ayant retiré l'accès, AESH, établissement B, appli mobile, capacité IA.

### 15.4 Conservation (à valider avec le DPO, socle 27) [R]
| Donnée | Durée proposée | Base |
|---|---|---|
| Positionnements détaillés, observations, auto-positionnements | **fin du cycle + 1 an** en base active, puis effacement | LSUN, base active [V R-C08] ; Q-EV14 du 21 |
| Synthèses de période | idem | idem |
| Bilans de fin de cycle (version visée) | avec le **dossier scolaire** (10 ou 50 ans, réglage du 7) ; archives intermédiaires LSUN 10 ans après la 3ᵉ | [V* R-C14], [V R-C08] |
| Attestations | **50 ans** (ASSR demandées pour le permis) | archives intermédiaires LSUN : 50 ans pour les attestations [V R-C08] |
| Référentiels (versions) | tant qu'un positionnement conservé les cite | intégrité |
| Bilans externes importés | comme les bilans internes | — |

---

## 16. Notifications (socle 8)

| Nature | Urgence | Destinataires | Canal | Regroupement | Notes |
|---|---|---|---|---|---|
| `competences.positionnement_publie` | U0 | élève ; liens `voir_notes` | in-app, résumé | **fusionné** avec `evaluations.note_publiee` quand l'évaluation porte notes et éléments | jamais le niveau dans le message externe |
| `competences.observation_publiee` | U0 | idem | résumé | par jour et par foyer | désactivable par l'établissement |
| `competences.bilan_cycle_disponible` | U1 | chaque lien `recevoir_bulletins`, élève | avec `evaluations.bulletin_disponible` (un seul avis) | — | demande de prise de connaissance (visa des responsables) |
| `competences.bilan_visa_relance` | U0 | responsables qui n'ont pas visé | résumé, J+15 puis J+30 | — | **information due** : « ne pas solliciter » ne la supprime pas (D-028) |
| `competences.attestation_enregistree` | U0 | élève ; liens `recevoir_bulletins` | résumé | — | document au coffre |
| `competences.attestation_a_renouveler` | U0 | élève, professeur du type | in-app | — | SST, habilitations |
| `competences.etat_preparation` | U0 | professeurs (ce qui leur manque), PP | in-app, résumé | par professeur | J-7, J-2 avant l'arrêt, **regroupé** avec `evaluations.rappel_saisie` |
| `competences.lsu_precontrole` | U0 | secrétariat, direction | in-app | hebdomadaire | dans les 8 semaines avant l'échéance |
| `competences.bilan_a_viser` | U0 | chef d'établissement | in-app | lot | après le conseil |

---

## 17. Actions métier (CMP-100 et suivantes)

Acteurs : **PRF** professeur, **PP**, **CDD** coordonnateur de discipline, **CDP** coordonnateur de dispositif, **DIR** direction, **ADF** secrétariat, **CPE**, **PAR** responsable, **ELV** élève, **SYS** système. ★ = confirmation renforcée et journal.

### 17.1 Référentiels et paramétrage
| Id | Action | Qui · droit | Règles | Événement |
|---|---|---|---|---|
| CMP-100 | Consulter le référentiel national (8 composantes, échelle, version) | tous les personnels · `competences.lire` | lecture seule [V R-C01, R-C02] | — |
| CMP-101 | Créer un référentiel de discipline (brouillon) | CDD, DIR · `competences.referentiels_gerer` | rattachement de chaque item à une composante (R-C2) | `competences.referentiel_cree` |
| CMP-102 | Ajouter, ordonner, renommer des éléments en brouillon | CDD | libellé officiel + libellé famille ; matière ; niveaux concernés | — |
| CMP-103 | Publier une version de référentiel (datée) | CDD, DIR ★ | une version publiée ne change plus (R-C3) | `competences.referentiel_publie` |
| CMP-104 | Demander à l'IA des libellés famille | CDD | §14.1 ; validation un par un | `ia.brouillon_cree` |
| CMP-105 | Importer un référentiel depuis un tableur | CDD | aperçu, rapport de rejets, échappement des cellules | `competences.referentiel_importe` |
| CMP-106 | Charger un référentiel de diplôme (lycée professionnel), avec aide IA à la structure | CDD, DIR | session d'examen ; §12.2 | idem |
| CMP-107 | Reprendre le référentiel de l'an passé en un geste (nouvelle version) | CDD, DIR | les éléments retirés restent retirés | `competences.referentiel_publie` |
| CMP-108 | Partir d'un référentiel modèle fourni par ScolIA (par discipline, cycle 3 et 4) [H : à rédiger sur les programmes en vigueur, jamais de mémoire] | CDD | modèle = version de départ modifiable | idem |
| CMP-109 | Retirer un élément (plus proposé, historique gardé) | CDD ★ | jamais de suppression (CC-15) | `competences.element_retire` |
| CMP-110 | Marquer un élément « adapté » (dispositif) | CDP | jamais imprimé « adapté » à un tiers | — |
| CMP-111 | Choisir, par niveau et pour l'année : avec notes / sans notes / les deux (6ᵉ–4ᵉ) | DIR · `competences.parametrer` | 3ᵉ : notes obligatoires [V R-C04] ; figé au premier arrêt (R-C4) | `competences.parametres_figes` |
| CMP-112 | Choisir l'échelle de positionnement et la méthode de synthèse | DIR | 3 échelles, 2 méthodes (§7) | idem |
| CMP-113 | Voir l'usage des éléments (combien de fois positionnés, par qui en anonyme) | CDD, DIR | aide à l'allègement ; jamais nominatif hors de ses cours | — |
| CMP-114 | Établir la correspondance entre deux versions (nouveaux programmes) | CDD | outil côte à côte, propositions déterministes par libellé ; validation | `competences.correspondance_etablie` |
| CMP-115 | Gérer les types d'attestations de l'établissement (durée de validité, qui saisit) | DIR, ADF | types nationaux non modifiables | — |
| CMP-116 | Déclarer les projets de l'année (EPI, AP, parcours) : thématique, cours, élèves | DIR, PP, PRF | parcours éducatif de santé ajouté en 2025 [V R-C04] | `competences.projet_declare` |

### 17.2 Positionner
| Id | Action | Qui · droit | Règles | Événement |
|---|---|---|---|---|
| CMP-120 | Associer des éléments à une évaluation (création ou modification) | PRF · `competences.positionner` | table de lien du 21 (§4.4) ; avertissement au-delà de 8 éléments (S22-02) | `evaluations.modifiee` |
| CMP-121 | Associer un élément à un exercice du barème détaillé | PRF | 21 §8.9 ; le niveau d'un exercice peut être proposé depuis sa note (seuils du professeur) | — |
| CMP-122 | Demander à l'IA de rattacher les questions d'un sujet | PRF | §14.1 point 3 ; cases à cocher | `ia.brouillon_cree` |
| CMP-123 | Positionner dans la grille de l'évaluation (colonnes « éléments » à côté de la note) | PRF | même grille que le 21 ; clavier (1–4, `n` non évalué) ; statuts du résultat appliqués (R-C5) | `competences.positionnements_saisis` |
| CMP-124 | Proposer les niveaux depuis les notes d'exercice (seuils fixés par le professeur) | PRF | proposition, validation de la colonne en un geste | idem |
| CMP-125 | Positionner une observation hors évaluation (élève ou groupe, élément, date) | PRF | `source = observation` ; publication explicite ou à l'arrêt (R-C6) | `competences.observation_saisie` |
| CMP-126 | Positionner depuis la séance (appel ouvert, socle 12) ou le cahier de textes (socle 20) | PRF | raccourci ; même règle | idem |
| CMP-127 | Positionner sur mobile (liste d'élèves, élément, niveau) | PRF | socle 11 ; hors ligne avec version (R-V27 du 21) | idem |
| CMP-128 | Corriger un positionnement avant l'arrêt | PRF | historique (R-C1) | `competences.positionnement_modifie` |
| CMP-129 | Annuler un positionnement (motif) | PRF | jamais supprimé | idem |
| CMP-130 | Demander une correction après l'arrêt | PRF → DIR | comme EVA-183 du 21 | `competences.correction_demandee` |
| CMP-131 | Corriger après l'arrêt ★ | DIR · `competences.corriger` | motif ; instantané du 21 reversionné si une synthèse change | `competences.corrigee` |
| CMP-132 | Ouvrir l'auto-positionnement d'une évaluation ou d'un élément | PRF | élève seulement ; jamais officiel | — |
| CMP-133 | S'auto-positionner | ELV | visible du professeur du cours | `competences.auto_positionnement` |
| CMP-134 | Voir l'auto-positionnement d'un élève à côté du sien | PRF | dialogue, pas de calcul | — |
| CMP-135 | Valider la proposition d'un tuteur de PFMP (lycée professionnel) | PRF référent | proposition du socle 39, auteur « tuteur, validé par » | `competences.positionnements_saisis` |
| CMP-136 | Positionner les élèves d'un dispositif sur des éléments adaptés | CDP | §12.3 | idem |
| CMP-137 | Positionner sur le CECRL par activité langagière | PRF de langue | échelle CECRL | idem |

### 17.3 Synthèses et bilan périodique
| Id | Action | Qui · droit | Règles | Événement |
|---|---|---|---|---|
| CMP-140 | Voir la synthèse proposée de chaque élève avec son explication | PRF | §7.3 ; « pas assez d'éléments » sous 2 | — |
| CMP-141 | Valider les synthèses proposées en un geste (colonne) | PRF · `competences.syntheses_valider` | après l'arrêt, avant la clôture des appréciations | `competences.syntheses_validees` |
| CMP-142 | Changer une synthèse (niveau différent de la proposition) | PRF | contrôle de cohérence non bloquant (§14.1 point 5) | idem |
| CMP-143 | Saisir les éléments du programme travaillés (cours, période) | PRF | proposition depuis les titres de séances du cahier de textes (socle 20), à relire ; longueur limitée par le modèle de bulletin | `competences.elements_travailles_saisis` |
| CMP-144 | Reprendre les éléments travaillés d'un autre de ses groupes du même niveau | PRF | copie, puis ajustement | — |
| CMP-145 | Rattacher les élèves à un projet (EPI, AP, parcours) et saisir son appréciation de projet | PRF, PP | apparaît au bilan périodique ; appréciation au 21 si l'établissement le souhaite | `competences.projet_maj` |
| CMP-146 | Voir l'état de préparation du bilan périodique de la classe | PP, DIR | synthèses manquantes, éléments travaillés vides, projets sans élèves | — |
| CMP-147 | Relancer un professeur (aperçu confirmé) | PP, DIR | regroupé avec la relance du 21 | `evaluations.relance` |
| CMP-148 | Écrire « non évalué (professeur absent) » pour un enseignement | DIR | CC-22 | `competences.syntheses_validees` |
| CMP-149 | Voir l'aperçu du bilan périodique d'un élève (comme la famille le verra) | PRF (ses élèves), PP, DIR | avant publication, filigrane « aperçu » | — |

### 17.4 Bilan de fin de cycle
| Id | Action | Qui · droit | Règles | Événement |
|---|---|---|---|---|
| CMP-160 | Ouvrir les bilans de fin de cycle de la classe (auto à J-30 du conseil) | SYS, PP, DIR | élèves de la dernière année du cycle (§7.1) | `competences.bilans_ouverts` |
| CMP-161 | Voir le dossier d'appui par composante (synthèses, éléments, historique, bilans externes) | PP, professeurs (fenêtre) | §7.4 | — |
| CMP-162 | Déposer un avis par composante | PRF (ses élèves) · `competences.bilans_preparer` | interne, jamais publié | `competences.avis_bilan_depose` |
| CMP-163 | Renseigner les 8 niveaux et la synthèse (proposition) | PP | niveau vide tant que non choisi (R-C9) ; non évaluable motivé | `competences.bilan_propose` |
| CMP-164 | Voir la proposition déterministe à côté du choix | PP | jamais pré-cochée | — |
| CMP-165 | Arrêter les bilans en conseil (appelé par la séance, socle 23) | président de séance (via 23) | R-C11 : composante vide = arrêt refusé pour cet élève | `competences.bilan_arrete` |
| CMP-166 | Arrêter en lot les bilans sans remarque | président (via 23) | contrôle préalable ; aperçu de la liste | idem |
| CMP-167 | Rouvrir un bilan arrêté avant visa | DIR ★ | motif | `competences.bilan_rouvert` |
| CMP-170 | Viser les bilans (un par un ou en lot avec contrôles) | chef, adjoint · `competences.bilans_viser` ★ | empreinte ; fige la version | `competences.bilan_vise` |
| CMP-171 | Publier avec le bulletin de la dernière période | SYS (via 23) | destinataires à la publication (R-V47 du 21) | `competences.bilan_publie` |
| CMP-172 | Viser le bilan (prise de connaissance) | PAR | case cochée, datée, par responsable [V R-C03, R-C08] | `competences.bilan_pris_connaissance` |
| CMP-173 | Enregistrer un visa papier d'une famille sans compte | ADF | pièce facultative | idem |
| CMP-174 | Relancer les responsables qui n'ont pas visé | SYS | J+15, J+30 ; information due | `competences.bilan_visa_relance` |
| CMP-175 | Corriger un bilan visé ou publié ★ | DIR · `competences.corriger` | nouvelle version, mention « rectifié le … », écart LSU (CX-45) | `competences.bilan_corrige` |
| CMP-176 | Établir le bilan d'un élève arrivé en fin de cycle | PP, DIR | CC-04 | `competences.bilan_propose` |
| CMP-177 | Produire le bilan de fin de cycle au format document (PDF/A, socle 7) | SYS | modèle prêt (D-023) | `documents.emis` |

### 17.5 Attestations
| Id | Action | Qui · droit | Règles | Événement |
|---|---|---|---|---|
| CMP-180 | Enregistrer une attestation pour un élève ou une liste (session de natation, épreuve ASSR) | PRF d'EPS, référent, ADF · `competences.attestations_saisir` | statut, date ; « dispense » sans motif (CC-39) | `competences.attestation_enregistree` |
| CMP-181 | Importer des résultats (Pix, ASSR) depuis un fichier | ADF | rapprochement INE puis nom + naissance ; file « à rapprocher » | idem |
| CMP-182 | Émettre le document d'attestation (socle 7) | ADF | modèle prêt ; code de vérification | `documents.emis` |
| CMP-183 | Corriger ou annuler une attestation ★ | ADF, DIR · `competences.attestations_corriger` | motif ; document annulé et réémis | `competences.attestation_corrigee` |
| CMP-184 | Voir les attestations attendues manquantes (fin de 6ᵉ, 5ᵉ, 3ᵉ, terminale) | ADF, PP, DIR | liste par classe | — |
| CMP-185 | Voir les attestations à renouveler | PRF, ADF | durée de validité | — |
| CMP-186 | Importer une attestation de l'école (ASSN, AER) | ADF | origine `ecole` | idem |
| CMP-187 | Consulter ses attestations et télécharger le document | ELV, PAR (`recevoir_bulletins`) | — | — |
| CMP-188 | Recevoir une alerte de renouvellement | ELV, PRF | SST, habilitation | `competences.attestation_a_renouveler` |
| CMP-189 | Délivrer un duplicata d'attestation (ancien élève compris) | ADF | identité du demandeur (socle 7) ; 50 ans | `documents.duplicata` |

### 17.6 Familles et élèves
| Id | Action | Qui · droit | Règles | Événement |
|---|---|---|---|---|
| CMP-200 | Voir les positionnements publiés (libellé famille d'abord, niveau en mots, date, évaluation) | PAR `voir_notes`, ELV | §9.2 | — |
| CMP-201 | Voir l'évolution par élément sur le cycle | PAR, ELV | frise, sans moyenne de couleurs | — |
| CMP-202 | Voir le bilan périodique publié (synthèses, éléments travaillés, projets) | PAR `recevoir_bulletins`, ELV | avec le bulletin | — |
| CMP-203 | Voir le bilan de fin de cycle publié et le viser | PAR `recevoir_bulletins`, ELV (lecture) | CMP-172 | — |
| CMP-204 | Lire le glossaire des niveaux et des composantes | PAR, ELV | — | — |
| CMP-205 | Demander une explication sur un positionnement ou un bilan | PAR, ELV | demande tracée au professeur ou au PP (comme EVA-166) ; réponse motivée [V R-C16] | `competences.explication_demandee` |
| CMP-206 | Voir « ce qu'il me reste à travailler » (éléments fragiles récents + ressources du cahier de textes) | ELV | déterministe ; aucun classement | — |
| CMP-207 | Partager son auto-positionnement avec ses responsables (lycée) | ELV | choix de l'élève | — |
| CMP-208 | Demander à l'IA d'expliquer un bilan | PAR, ELV | §14.1 point 4 ; publié seulement | — |
| CMP-209 | Voir ses attestations et leurs documents | ELV, PAR | — | — |

### 17.7 Échanges, imports, pilotage
| Id | Action | Qui · droit | Règles | Événement |
|---|---|---|---|---|
| CMP-220 | Importer les bilans de l'école (CM1–CM2) pour les 6ᵉ | ADF · `competences.externes_saisir` | fichier LSU s'il est lisible, sinon pièce PDF ; origine externe | `competences.externes_importes` |
| CMP-221 | Importer les bilans d'un arrivant | ADF | idem ; CC-05 | idem |
| CMP-222 | Voir l'état de préparation LSU (8 semaines avant l'échéance) | ADF, DIR | pré-contrôle du socle 9 ; liens vers l'écran qui corrige | — |
| CMP-223 | Produire l'aide à la saisie LSU par classe (export de travail, non officiel) | ADF · `exports.officiels` | tant que D-2 n'est pas levée ; échappement des cellules | `export.realise` |
| CMP-224 | Préparer la déclaration LSU officielle (bilans périodiques, fin de cycle, attestations) | ADF, DIR ★ | **après** D-2 ; depuis des données visées et validées | `export.realise` |
| CMP-225 | Traiter le compte rendu LSU (rejets → tâches) | ADF | socle 9 FX-08 | `echanges.retour_traite` |
| CMP-226 | Fournir les compétences du LSL (positionnements, CECRL, Pix) au socle 23 | SYS | port | — |
| CMP-227 | Joindre bilans et attestations au dossier de transfert | ADF | publié et visé seulement ; jamais les accompagnements (R-V37) | — |
| CMP-228 | Voir les indicateurs (taux de bilans visés, synthèses validées, attestations obtenues) | DIR | socle 47 ; aucune comparaison nominative de professeurs | — |
| CMP-229 | Voir la charge de positionnement par professeur (anonyme pour les autres) | DIR | lutte contre la surcharge (T-29) ; jamais de classement | — |
| CMP-230 | Exporter toutes les données de compétences d'un élève (droit d'accès) | DIR, DPO | RGPD art. 15 ; journalisé | `competences.acces_exporte` |
| CMP-231 | Reprendre les modèles A et B de l'existant (lot de reprise) | SYS, DIR | §22 ; rapport d'écarts | `competences.reprise_appliquee` |

---

## 18. Règles transverses (R-C1 et suivantes) [R]

| Id | Règle |
|---|---|
| R-C1 | Un positionnement ne s'écrase ni ne se supprime : chaque changement est une version (auteur, moment, motif après l'arrêt). |
| R-C2 | Tout élément d'un référentiel d'établissement est rattaché à **une** composante du socle (collège) ou à un bloc (diplôme) ; le référentiel national n'est jamais modifiable par un établissement. |
| R-C3 | Une version de référentiel publiée ne change plus ; un positionnement cite l'élément de **sa** version. |
| R-C4 | Le mode (avec / sans notes / les deux), l'échelle et la méthode de synthèse sont figés pour l'année dès le premier arrêt de notes. |
| R-C5 | Les statuts `absent`, `dispense`, `non_rendu` d'un résultat du 21 s'appliquent à tous les éléments de l'évaluation ; **aucun statut n'est converti en niveau**. |
| R-C6 | Rien n'est visible d'une famille sans publication ; une observation non publiée l'est à l'arrêt des notes. |
| R-C7 | Positionnements au fil de l'eau : droit `voir_notes` du lien ; synthèses, éléments travaillés, projets, bilan de fin de cycle, attestations : droit `recevoir_bulletins` (comme R-V34 du 21). |
| R-C8 | Une synthèse de période n'existe qu'après validation par le professeur du cours ; une proposition n'est jamais publiée. |
| R-C9 | Un niveau de bilan de fin de cycle n'est **jamais pré-rempli** ; aucune compensation entre composantes ni de « maîtrise globale du socle » [V R-C02]. |
| R-C10 | L'IA ne produit **aucun** positionnement, synthèse ni niveau de bilan, et ne lit ni accompagnements ni santé [V* R-C15]. |
| R-C11 | Un bilan de fin de cycle n'est arrêté que si ses 8 composantes ont un niveau ou un « non évaluable » motivé. |
| R-C12 | Seul le président de la séance (socle 23) arrête les bilans ; seul le chef d'établissement ou son adjoint les vise. |
| R-C13 | Le cycle est **déduit** du niveau de l'élève à la date (socle 5), jamais saisi. |
| R-C14 | Un bilan ou une synthèse importés d'un autre établissement sont en lecture seule, avec leur source et leur pièce. |
| R-C15 | Une attestation est un fait daté (statut, date, auteur) ; « non obtenue » n'est jamais transmise à un tiers ; « dispense » ne porte aucun motif visible. |
| R-C16 | Un professeur n'exporte jamais ; toute déclaration LSU passe par le moteur du socle 9 et part de données visées et validées. |
| R-C17 | Un élément retiré n'est plus proposé mais reste lisible partout où il a été positionné. |
| R-C18 | Les documents internes portent le prénom d'usage, le LSU l'état civil (R-V36 du 21). |
| R-C19 | La rubrique « accompagnements » n'est jamais transmise à un tiers (R-V37 du 21). |
| R-C20 | Les avis des professeurs sur un bilan de fin de cycle sont internes : jamais publiés, jamais montrés aux délégués. |

---

## 19. Événements (outbox, dans la transaction du fait)

**Émis** : `competences.referentiel_cree`, `.referentiel_publie`, `.referentiel_importe`, `.element_retire`, `.correspondance_etablie`, `.parametres_figes`, `.projet_declare`, `.projet_maj`, `.positionnements_saisis`, `.positionnement_modifie`, `.observation_saisie`, `.auto_positionnement`, `.correction_demandee`, `.corrigee`, `.syntheses_validees`, `.elements_travailles_saisis`, `.bilans_ouverts`, `.avis_bilan_depose`, `.bilan_propose`, `.bilan_arrete`, `.bilan_rouvert`, `.bilan_vise`, `.bilan_publie`, `.bilan_pris_connaissance`, `.bilan_corrige`, `.attestation_enregistree`, `.attestation_corrigee`, `.attestation_a_renouveler`, `.externes_importes`, `.explication_demandee`, `.acces_exporte`, `.reprise_appliquee`. Charges utiles : identifiants seulement, **jamais un niveau ni un texte** (R-V17).
**Consommés** : `evaluations.publiee`, `.publication_retiree`, `.annulee`, `.notes_arretees`, `.appreciations_closes`, `.periode_rouverte`, `.rattrapage_autorise` (21) → publication, verrous ; `conseils.tenu`, `bulletins.publies` (23) → arrêt et publication des bilans ; `referentiel.cours_membre_*`, `affectation_corrigee`, `niveau_modifie` (5) → public, cycle ; `services.intervention_*`, `missions.*` (6) → droits ; `sante.inaptitude_*` (34), `accompagnement.*` (35) → dispenses, dispositifs ; `eleve.majeur`, `liens.*` (2, 4) → droits ; `eleve.fusionne` (2) → rattachement ; `eleve.sorti` (2, 3) → figer.

---

## 20. Écrans clés

| Écran | Pour qui | Contenu | Mobile |
|---|---|---|---|
| **Grille de l'évaluation** (celle du 21) avec colonnes d'éléments | PRF | note et niveaux côte à côte ; saisie au clavier ; statuts communs | oui |
| **Observation rapide** | PRF | liste d'élèves × un élément, niveau en un toucher | **oui** (usage principal) |
| **Synthèses de la période** | PRF | par élève : proposition, explication, valeur validée ; validation de la colonne | tablette |
| **Bilan périodique (aperçu)** | PRF, PP | tel que la famille le verra | non |
| **Préparation du bilan périodique** | PP, DIR | tableau classe × enseignement : manques, relances | non |
| **Bilan de fin de cycle** | PP, équipe | 8 composantes, dossier d'appui, avis, choix, synthèse | tablette (séance) |
| **Visa** | chef d'établissement | liste avec contrôles, visa en lot ★ | non |
| **Référentiels** | CDD, DIR | arbre, versions, libellés famille, usage | non |
| **Attestations** | ADF, PRF | par type et classe ; saisie en liste ; manquantes ; à renouveler | oui (saisie) |
| **État de préparation LSU** | ADF, DIR | erreurs bloquantes et avertissements, liens qui corrigent | non |
| **Famille / élève** | PAR, ELV | positionnements en mots, évolution, bilans, attestations, glossaire, visa | **oui** |
Règles de présentation (`DESIGN_SYSTEM.md`, `FRONTEND_LEPTOS.md`) : niveaux **toujours en mots** avec une forme (pastille + libellé), jamais la couleur seule ; tableaux denses au clavier pour la saisie ; tuiles pour la préparation ; 375 px pour la famille.

---
## 21. Modèle de données cible (brique `competences`)

Règles appliquées à **chaque** table (D-036, D-038, `PRA_SAUVEGARDES.md` §18.4) : `id UUID` généré par l'application (E3) ; `etablissement_id UUID NOT NULL` **y compris dans les tables filles** (E1) ; contrainte `UNIQUE (etablissement_id, id)` et **toute référence en FK composée** `(etablissement_id, x_id)` (E2) ; `cree_le TIMESTAMPTZ NOT NULL DEFAULT now()` et `modifie_le` sur les tables modifiables (E6) ; **aucune colonne JSON/JSONB** (E10) ; **aucun BYTEA** : les pièces sont des objets du registre `ent_objet` (500) et des documents du socle 7 (E5) ; **RLS forcée** par `ent_rls_poser` (580) dès la création (E13) ; chaque table classée au registre `registre_donnees.rs` (E12) ; corrections, visas et retraits journalisés dans `ent_journal_modif` (E7) ; aucun nom d'élève dans un nom d'objet ou un journal applicatif (E14). Les tables `ref_*` sont des **références globales** (portée `globale_reference`, E4 : aucune donnée d'établissement).

```
-- Références globales (fournies par ScolIA, versionnées, non modifiables)
ref_cmp_version               id, code ('socle_commun','cecrl','lsl'), libelle, texte_source, valable_du, valable_au NULL
ref_cmp_composante            id, version_id, code ('fra','lve','msi','arc','d2','d3','d4','d5'), domaine SMALLINT, libelle_officiel, ordre
ref_cmp_echelle               id, code ('maitrise_socle','maitrise','atteinte','cecrl','lsl','acquis'), libelle, modifiable BOOLEAN
ref_cmp_echelle_niveau        id, echelle_id, rang SMALLINT, code, libelle, equivalent_maitrise SMALLINT NULL CHECK (1..4)
ref_cmp_attestation_type      id, code ('assn','assr1','assr2','aer','psc','pix'), libelle, niveau_habituel, source_texte

-- Paramètres de l'établissement
cmp_parametre_annee           etablissement_id, annee_id, niveau_id, mode ('notes','positionnements','les_deux'),
                              echelle_code, methode_synthese ('plus_recent','meilleur'), seuil_elements SMALLINT DEFAULT 2,
                              ouverture_bilan_jours SMALLINT DEFAULT 30, fige_le NULL, version, cree_le, modifie_le
                              UNIQUE (etablissement_id, annee_id, niveau_id)

-- Référentiels de l'établissement
cmp_referentiel               id, etablissement_id, origine ('etablissement','diplome'), code, libelle, matiere_id NULL,
                              diplome_code NULL, cree_par, cree_le
cmp_referentiel_version       id, etablissement_id, referentiel_id, numero, etat ('brouillon','publiee','retiree'),
                              valable_du, valable_au NULL, publie_par NULL, publie_le NULL, cree_le
cmp_element                   id, etablissement_id, version_id, parent_id NULL, nature ('bloc','competence','item'),
                              code, libelle_officiel, libelle_famille NULL, composante_id NULL (ref_cmp_composante),
                              adapte BOOLEAN DEFAULT false, ordre, retire_le NULL, cree_le, modifie_le
                              CHECK (nature <> 'item' OR composante_id IS NOT NULL OR origine du référentiel = 'diplome')  -- par déclencheur
cmp_element_niveau            etablissement_id, element_id, niveau_id          -- niveaux concernés (table de liaison, pas de tableau)
cmp_correspondance            id, etablissement_id, element_source_id, element_cible_id, etabli_par, cree_le

-- Positionnements
cmp_positionnement            id, etablissement_id, eleve_id, element_id, cours_id NULL, date_positionnement DATE,
                              niveau_id NULL (ref_cmp_echelle_niveau), statut ('positionne','non_evalue','absent','dispense','non_evaluable'),
                              source ('evaluation','observation','import','reprise','pfmp'), evaluation_id NULL, resultat_id NULL,
                              publie_le NULL, annule_le NULL, motif_annulation_code NULL, version INT, auteur_user_id,
                              valide_par_user_id NULL (pfmp), cree_le, modifie_le
                              CHECK ((statut = 'positionne') = (niveau_id IS NOT NULL))
                              UNIQUE (etablissement_id, evaluation_id, eleve_id, element_id) WHERE evaluation_id IS NOT NULL
cmp_positionnement_historique id, etablissement_id, positionnement_id, version, niveau_id NULL, statut, auteur_user_id,
                              motif_code NULL, motif_texte NULL, cree_le               -- ajout seul (déclencheur)
cmp_auto_positionnement       id, etablissement_id, eleve_id, element_id, evaluation_id NULL, niveau_id, partage_famille BOOLEAN, cree_le
cmp_auto_positionnement_ouverture  etablissement_id, cours_id, evaluation_id NULL, element_id NULL, ouvert_par, du, au NULL

-- Synthèses, bilan périodique
cmp_synthese                  id, etablissement_id, eleve_id, cours_id, matiere_id, periode_id, niveau_propose_id NULL,
                              nb_elements SMALLINT, niveau_id NULL, statut_synthese ('validee','non_evalue','dispense'),
                              methode, valide_par, valide_le, version, cree_le, modifie_le
                              UNIQUE (etablissement_id, eleve_id, cours_id, periode_id)
cmp_synthese_historique       ajout seul (version, valeurs, auteur, motif)
cmp_elements_travailles       id, etablissement_id, cours_id, periode_id, texte VARCHAR(600), auteur_user_id, version, cree_le, modifie_le
cmp_projet                    id, etablissement_id, annee_id, nature ('epi','ap','parcours_avenir','parcours_citoyen','peac','parcours_sante','autre'),
                              thematique NULL, intitule, description VARCHAR(600), du, au NULL, cree_par, cree_le, modifie_le
cmp_projet_cours              etablissement_id, projet_id, cours_id
cmp_projet_eleve              etablissement_id, projet_id, eleve_id, du, au NULL

-- Bilan de fin de cycle
cmp_bilan_cycle               id, etablissement_id, eleve_id, cycle SMALLINT CHECK (cycle IN (2,3,4)), annee_id, classe_id,
                              origine ('scolia','externe','reprise'), etat ('ouvert','propose','arrete','vise','publie','remplace'),
                              version INT, synthese VARCHAR(1500) NULL, conseil_id NULL (socle 23), arrete_par NULL, arrete_le NULL,
                              vise_par NULL, vise_le NULL, publie_le NULL, empreinte CHAR(64) NULL, remplace_id NULL,
                              motif_correction_code NULL, motif_correction_texte NULL, document_id NULL (socle 7),
                              source_libelle NULL (externe), cree_le, modifie_le
                              UNIQUE (etablissement_id, eleve_id, cycle, annee_id, version)
                              -- immuable par déclencheur à partir de 'vise' (sur le modèle de ent_bulletin, 182:34-60)
cmp_bilan_composante          etablissement_id, bilan_id, composante_id, niveau_id NULL, non_evaluable BOOLEAN,
                              motif_non_evaluable_code NULL, niveau_propose_id NULL
                              PRIMARY KEY (etablissement_id, bilan_id, composante_id)
cmp_bilan_avis                id, etablissement_id, bilan_id, composante_id, auteur_user_id, niveau_suggere_id NULL,
                              texte VARCHAR(400) NULL, cree_le                       -- interne (R-C20)
cmp_bilan_prise_connaissance  id, etablissement_id, bilan_id, personne_id, lien_id, mode ('coffre','papier'), le, saisi_par NULL

-- Attestations
cmp_attestation_type          id, etablissement_id, ref_type_id NULL, code, libelle, duree_validite_mois NULL,
                              action_saisie ('competences.attestations_saisir'), archive_le NULL
cmp_attestation               id, etablissement_id, eleve_id, type_id, statut ('obtenue','non_obtenue','dispense','annulee'),
                              obtenue_le DATE NULL, valable_au DATE NULL, origine ('saisie','import','ecole','reprise'),
                              reference_externe NULL, document_id NULL (socle 7), piece_objet_id NULL (ent_objet),
                              version, saisi_par, cree_le, modifie_le
cmp_attestation_historique    ajout seul

-- Imports
cmp_import                    id, etablissement_id, nature ('bilans_ecole','bilans_arrivant','pix','assr','referentiel'),
                              depose_par, depose_le, etat ('depose','rapproche','applique','rejete'), objet_id (ent_objet)
cmp_import_ligne              id, etablissement_id, import_id, rang INT, eleve_id NULL, motif_rejet_code NULL, applique BOOLEAN
```
**Attente posée au 21** : `eva_evaluation_element (etablissement_id, evaluation_id, element_id, ordre, bareme_item_id NULL)` dans la réserve du 21 (§4.4). **Pas de JSON** : les listes (niveaux d'un élément, cours d'un projet, élèves d'un projet) sont des tables de liaison ; aucun tableau `TEXT[]`. **Exceptions D-036** : aucune. **Files d'effets de bord** : aucune nouvelle (outbox et file d'envoi du cœur, déjà classées, E9).

---

## 22. Plan de migration depuis l'existant (sans casser l'appel, le mobile ni le labo)

| Étape | Contenu | Compatibilité | Contrôle |
|---|---|---|---|
| M1 (C0) | journal en ajout seul des écrasements de `ent_competence_evaluation` (déclencheur `BEFORE UPDATE` qui copie l'ancienne ligne) ; route de publication du livret **fermée (410)** ; bilan de cycle visible de la famille seulement avec le bulletin publié de la dernière période ; création d'items soumise à `competences.referentiels_gerer` ; graine réservée au labo | lectures inchangées, filtrées | tests de fuite (bilan avant publication), 410, droit |
| M2 (C1) | tables `ref_cmp_*` remplies (8 composantes [V R-C01], échelles [V R-C02, R-C12], types d'attestations [V R-C03, R-C07]) ; `cmp_parametre_annee` par défaut (« notes » partout, échelle « maîtrise », « le plus récent ») | aucune | test du registre E12 |
| M3 (C12) | reprise du **modèle A** : chaque `ent_competence_item` → `cmp_element` d'un référentiel « Reprise 2026 » (version publiée, rattachement à une composante **proposé** depuis D1–D5 et à valider par le CDD, sinon élément « à rattacher ») ; chaque `ent_competence_evaluation` → `cmp_positionnement` (`source = reprise`, date = fin de la période) ; reprise du **modèle B** : `ent_bilan_cycle` validé → `cmp_bilan_cycle` (`origine = reprise`, état `vise` **seulement** si un bulletin publié de la dernière période existe, sinon `propose`) | **vues de compatibilité** en lecture (`ent_competence_evaluation`, `ent_bilan_cycle`) alimentées par les nouvelles tables pour l'UI JS et l'export CSV historique | comptes identiques ; écarts listés (items sans composante, bilans de cycle 2 au collège CD7) |
| M4 (C2–C8) | les services écrivent dans `cmp_*` | vues | — |
| M5 (C13) | RLS stricte ; retrait des vues et anciennes tables (088, 181) après la durée d'archive | — | aucune lecture des vues 30 jours |

---

## 23. Lots de code (équipe Sonnet)

### 23.1 Tableau des lots
| Lot | Contenu | Dépend de | Critères de fin |
|---|---|---|---|
| **C0** correctifs urgents | CD1 (journal des écrasements), CD2 (bilan visible seulement avec le bulletin publié), CD3 (publication du livret fermée, 410), CD4 (droit dédié), CD5 (graine labo seulement) | rien ; peut partir tout de suite | tests : famille ne voit pas un bilan validé non publié ; `POST …/livret` → 410 ; professeur sans droit ne crée pas d'item ; graine refusée hors labo ; ancienne valeur retrouvée au journal |
| **C1** référentiels et paramètres | tables `ref_cmp_*`, `cmp_parametre_annee`, `cmp_referentiel*`, `cmp_element*`, correspondance ; écrans référentiels ; import tableur ; reprise d'un an sur l'autre | C0 ; socle 5 (niveaux, matières) | référentiel national non modifiable (test) ; version publiée immuable ; élément retiré lisible |
| **C2** positionnements | `cmp_positionnement` + historique ; colonnes d'éléments dans la grille du 21 ; observation rapide ; statuts R-C5 ; publication liée au 21 ; correction après l'arrêt | C1 ; 21 V1 (lien `eva_evaluation_element`, période par élève) | exemple §7.5 ; matrice de séparation ; aucune conversion de statut en niveau (propriété) |
| **C3** synthèses et bilan périodique | proposition et explication (§7.3), validation en lot, éléments travaillés, projets (EPI, AP, parcours), état de préparation, aperçu | C2 ; 21 V8 (arrêt, instantané) | test d'or §7.5 ; propriétés (ordre des saisies sans effet ; « pas assez d'éléments ») ; synthèses dans l'instantané |
| **C4** bilan de fin de cycle | ouverture, dossier d'appui, avis, proposition, arrêt (port appelé par le 23), visa, publication, prise de connaissance, relances, correction en version 2 | C3 ; socle 23 B3 (séance) | R-C9, R-C11, R-C12 en tests ; immuabilité après visa (déclencheur) ; CC-04, CC-12, CC-28 à CC-30 |
| **C5** attestations | types, saisie en liste, import, document (socle 7), duplicata, renouvellement, manquantes | C1 ; socle 7 (émission) | CC-37 à CC-41 ; « non obtenue » jamais dans un document à un tiers |
| **C6** famille et élève | lecture en mots, évolution, bilans, glossaire, visa, demande d'explication, « ce qu'il me reste à travailler », auto-positionnement | C2, C3, C4 ; socle 8 | matrice famille complète (voir_notes / recevoir_bulletins / interdiction / majeur) |
| **C7** dispositifs et lycée | éléments adaptés, coordonnateur de dispositif, UPE2A (`non_evaluable`), CECRL, échelle LSL | C2 | §12 en tests |
| **C8a** LSU : pré-contrôle et aide à la saisie | état de préparation hebdomadaire, liens qui corrigent, aide à la saisie par classe (export de travail) | C3, C4, C5 ; socle 9 X0 | NOT-10 / T-28 : chaque erreur pointe un écran ; aucun professeur n'exporte |
| **C8b** LSU officiel | déclaration au format de la spécification éditeur, codes officiels, compte rendu | **D-2 levée** (F-06, vers mai 2027) | fichier accepté par l'application LSU sur un établissement de test |
| **C9** lycée professionnel | référentiels de diplôme (aide IA à la structure), positionnement PFMP validé (socle 39), lien au CCF (socle 38) | C2 ; socles 38, 39 | parcours PFMP complet ; aucun positionnement écrit par un tuteur sans validation |
| **C10** mobile | observation rapide, positionnement dans la grille, lecture famille et élève | C2, C6 ; socle 11 | parité des droits avec le web |
| **C11** IA | libellés famille, structure de diplôme, rattachement d'un sujet, explication d'un bilan ; contrôles de cohérence (déterministes d'abord) | C1–C6 ; cœur IA ; étude juridique (Q-EV8 du 21) pour les capacités à modèle de langage | refus testés (§14.3) ; contrôles déterministes livrables **avant** l'étude |
| **C12** reprise | modèles A et B, vues de compatibilité, rapport d'écarts | C1–C4 | comptes identiques ; écarts listés |
| **C13** RLS stricte et retraits | RLS stricte, retrait des vues et des tables 088/181 | C12, socle 21 V13 | inventaire RLS ; tests d'isolation |

**Ordre recommandé (appliqué par défaut, D-031)** : **C0** tout de suite (fuite CD2, envois CD3) → C1 → C2 → C3 → C4 → C5 → C6 → C12 ; C7, C9, C10, C11, C8a ensuite ; **C8b après D-2**. **Démo de décembre (D-034)** : le collège n'est pas dans la démo du pilote (lycée) ; C0 suffit pour qu'aucun écran existant ne fuie. **V1 (fin janvier – février 2027)** : C0 à C6 + C8a. **Épaississement (février – août 2027)** : C7, C9 à C12. Le **pilote (lycée)** a surtout besoin de C7 (échelle LSL, CECRL) et C9 s'il ouvre une voie professionnelle.

### 23.2 Migrations (plage imposée **2440–2459**, registre `MIGRATIONS.md`)
| N° | Lot | Contenu |
|---|---|---|
| **2440** | C0 | `ent_competence_evaluation_journal` (ajout seul) et déclencheur d'écrasement ; aucune nouvelle colonne métier |
| **2441** | C1 | `ref_cmp_version`, `ref_cmp_composante`, `ref_cmp_echelle`, `ref_cmp_echelle_niveau`, `ref_cmp_attestation_type` (+ données de référence) |
| **2442** | C1 | `cmp_parametre_annee` |
| **2443** | C1 | `cmp_referentiel`, `cmp_referentiel_version`, `cmp_element`, `cmp_element_niveau`, `cmp_correspondance` |
| **2444** | C2 | `cmp_positionnement`, `cmp_positionnement_historique` (déclencheur d'ajout seul) |
| **2445** | C6 | `cmp_auto_positionnement`, `cmp_auto_positionnement_ouverture` |
| **2446** | C3 | `cmp_synthese`, `cmp_synthese_historique`, `cmp_elements_travailles` |
| **2447** | C3 | `cmp_projet`, `cmp_projet_cours`, `cmp_projet_eleve` |
| **2448** | C4 | `cmp_bilan_cycle`, `cmp_bilan_composante`, `cmp_bilan_avis`, `cmp_bilan_prise_connaissance` (déclencheur d'immuabilité) |
| **2449** | C5 | `cmp_attestation_type`, `cmp_attestation`, `cmp_attestation_historique` |
| **2450** | C12 | `cmp_import`, `cmp_import_ligne` ; reprise des modèles A et B ; vues de compatibilité |
| 2451–2456 | réserve | C7, C9, C11 et ajouts de la passe personas |
| **2457** | C13 | RLS stricte (contrôle que toutes les tables `cmp_*` l'ont) |
| **2458–2459** | C13 | retrait des vues de compatibilité puis des tables 088 et 181 **après la durée d'archive** (migration destructrice : décision, point de sauvegarde vérifié, note dans `MIGRATIONS.md`, E8) |
Chaque migration est **additive et rejouable** (E8 : `IF NOT EXISTS`, données de référence en `ON CONFLICT DO NOTHING`) ; `MIGRATION_TAG` = la plus haute du répertoire ; ordre croissant dans `migrate.rs`.

---

## 24. Tests et recette

- **Tests d'or** : exemple §7.5 (deux méthodes) ; reprise M3 (comptes identiques, écarts listés) ; bilan de fin de cycle sans compensation (R-C9).
- **Propriétés** : l'ordre des saisies ne change pas une synthèse ; un statut n'est jamais un niveau ; une version publiée de référentiel ne change jamais ; un bilan visé ne change jamais (déclencheur).
- **Matrice de séparation** (§15.3) générée depuis la déclaration des routes (D-026), avec les capacités IA (équivalence écran / IA).
- **Recette par personas** (D-021) : professeur d'un autre groupe qui tente les positionnements ; coordonnateur qui tente de modifier le référentiel national ; parent sans `recevoir_bulletins` qui tente le bilan de fin de cycle par trois chemins (coffre, relevé, document) ; élève qui lit la proposition de synthèse ; professeur qui exporte un LSU ; IA à qui l'on demande un niveau ; établissement B.

---

## 25. Réglages signalés « usine à gaz » (D-023) — signalés, non supprimés

| Id | Réglage | Recommandation |
|---|---|---|
| S22-01 | Pondération des éléments dans la synthèse | **non** proposé (médiane seulement) |
| S22-02 | Nombre maximal d'éléments par évaluation | pas de limite, **avertissement** au-delà de 8 |
| S22-03 | Délai d'ouverture du bilan de fin de cycle | 30 jours, modifiable |
| S22-04 | Visa des responsables : relances | J+15, J+30, non désactivable (information due) |
| S22-05 | Couleurs des niveaux | dans la charte de l'établissement, toujours accompagnées du mot |
| S22-06 | Libellés courts de l'échelle « atteinte » | modifiables ; libellés longs fixes |
| S22-07 | Auto-positionnement | désactivé par défaut, ouvert par le professeur |
| S22-08 | Référentiels modèles fournis par ScolIA | proposés, jamais imposés |

---

## 26. Questions, décisions appliquées par défaut (D-031) et points pour Florian

### 26.1 Questions tranchées selon la recommandation (*appliqué par défaut, D-031*)
| Id | Question | Recommandation appliquée |
|---|---|---|
| Q-CP1 | Un seul modèle de compétences (fusion de 088 et 181) ? | **Oui** (§6, §21, §22) |
| Q-CP2 | Référentiel national figé, établissement libre et rattaché ? | **Oui** (R-C2) |
| Q-CP3 | Synthèse de période calculée ou proposée ? | **Proposée et validée** par le professeur, deux méthodes, défaut « le plus récent prévaut » (§7.3) |
| Q-CP4 | Bilan de fin de cycle pré-rempli depuis les synthèses ? | **Non** : proposition affichée à côté, niveau vide tant que l'équipe n'a pas choisi (R-C9) |
| Q-CP5 | Visa des responsables : signature électronique ou prise de connaissance ? | **Prise de connaissance** cochée et datée (comme le téléservice LSU [V R-C08]) ; papier enregistré par le secrétariat |
| Q-CP6 | IA et positionnements ? | **Jamais** (R-C10) ; aide au référentiel, au rattachement d'un sujet, à l'explication |
| Q-CP7 | Attestations : table propre ou compétences ? | **Table propre**, fait daté, document du socle 7 (§11) |
| Q-CP8 | Classes sans notes en 6ᵉ–4ᵉ ? | **Oui**, choix de la direction par niveau, figé pour l'année (R-C4) ; 3ᵉ : notes [V R-C04] |
| Q-CP9 | Durées de conservation | positionnements et synthèses : fin de cycle + 1 an ; bilans : dossier scolaire ; attestations : 50 ans (§15.4) — **à valider avec le DPO** |
| Q-CP10 | Droit d'export LSU | `exports.officiels` (choix du lot V0 du 21), secrétariat et direction seulement |
| Q-CP11 | Lien évaluation ↔ éléments : au 21 ou au 22 ? | **Au 21**, comme son §6.1 le pose (§4.4) |
| Q-CP12 | Parent sans exercice : bilan de fin de cycle ? | **Reçu par défaut** (R-V43 du 21) |

### 26.2 Ce qui engage Florian (non appliqué)
| Id | Point | Pourquoi Florian |
|---|---|---|
| F22-1 | **Démarche éditeur (F-06)** : sans elle, le LSU officiel (C8b) ne peut pas être produit ; un collège client ressaisit dans LSU avec l'aide à la saisie | démarche officielle (D-031), déjà reportée vers mai 2027 (D-032) |
| F22-2 | **Exemples anonymisés** d'un fichier LSU et d'un export de compétences Charlemagne (F-07) | données de l'établissement |
| F22-3 | **Durées de conservation** (§15.4) | DPO |
| F22-4 | **Référentiels modèles** par discipline (CMP-108) : les faire rédiger par des enseignants (contrat, coût) ou les laisser aux établissements | coût, relecture pédagogique |
| F22-5 | Un **collège** pilote ou client avant 2028 ? Sans collège, C3–C6 et C8 ne seront éprouvés qu'en labo | stratégie commerciale |

---

## 27. Faits, hypothèses, contradictions

**Faits vérifiés [F]** : modèles A et B (§2.1), écrasement (`competence_repo.rs:99-125`), bilan visible dès validation (`evaluations_conseil.rs:1122-1131`), publication du livret sans contrôle (`service.rs:6822`), graine (`service.rs:6646`), RLS permissive (460, `migrate.rs:220`), aucun écran Leptos ni appel mobile (audit du sous-agent, 2026-11-11).
**Vérifié [V]** le 2026-11-11 : R-C01 à R-C16 (§5), avec les réserves de lecture indiquées.
**Non vérifié [NV]** : voir la fin du §5.
**Hypothèses [H]** : irritants du terrain (§3) ; usages de Pronote et de Charlemagne (§2.3) ; ordre des écrans de l'application LSU (aide à la saisie).
**Contradictions relevées** :
- **C1** `21_EVALUATIONS.md` §6.1 parle d'une « échelle à 4 niveaux [V arrêté 2015] » pour les positionnements ; la vérification du 2026-11-11 montre que les libellés sont dans **D. 122-3** et ne visent que le **bilan de fin de cycle** (l'arrêté y renvoie) : confirme C13 du 21. Pour les positionnements au fil de l'eau, l'échelle est un **choix** de l'établissement (§7.2).
- **C2** `21_EVALUATIONS.md` §21 : `eva_instantane.contenu JSONB` sert d'« entrée unique du conseil et du bulletin » (21 §6.2), donc **relu comme source** ; D-036 n'admet un instantané JSON que s'il n'est **jamais relu comme source de vérité**. Recommandation (*appliquée par défaut, D-031*, détaillée au socle 23 §21) : l'instantané d'arrêt est **normalisé en lignes typées** (`eva_instantane_ligne`), qui portent aussi les **synthèses validées** du 22 ; la copie figée du bulletin publié reste l'exception déjà déclarée (`ent_bulletin.contenu`).
- **C3** `SOCLES.md` §22 annonce un existant « P » et « `lsun_export.rs` (CSV, pas le format officiel) » : exact ; il omet le modèle B (181), qui est la meilleure base.
- **C4** Réglementation : D. 122-3 dit encore que le DNB « atteste la maîtrise du socle », alors que le décret 2025-328 retire le socle du contrôle continu [V R-C10]. ScolIA garde le bilan de fin de cycle (obligatoire au livret, art. 5) et **ne le relie pas** au DNB.
- **C5** `21_EVALUATIONS.md` §2.1 situe les écrans sous `ent-web-app/src/modules/evaluations/` ; le chemin réel est `web/crates/ent-web-app/src/modules/evaluations/` [F, sous-agent].
- **C6** `TRANSITION_T0_INVENTAIRE.md` l. 135 range « compétences » avec les notes Next.js (C06, ébauche perdable) : cohérent avec ce document (rien à reprendre).

---

## 28. [ajout personas] Ce que change la passe personas (`PERSONAS_22.md`, 2026-11-11)

17 personas (dont un **attaquant** et le **DPO**), 78 manques (M22-001 à M22-078) : **3 bloquants**, 48 importants, 24 confort, 2 différenciants. **En cas de contradiction avec les §0 à §27, ce paragraphe l'emporte.**

### 28.1 Bloquants tranchés (*appliqué par défaut, D-031*)
- **A — `recevoir_bulletins` absent du code** : C6 ne publie à une famille synthèses, bilan et attestations qu'avec ce droit (socle 4, lot A14) ; d'ici là, repli `peut_voir_notes` (comme le V0 du 21) **et** tâche au secrétariat pour chaque lien sans ce droit qui devait recevoir (parent sans exercice, tiers) : courrier ou remise tracée (R-C21).
- **B — Observation et jugement de comportement** : aucune observation ne se fonde sur un fait disciplinaire (aucune lecture du socle 13) ; aucune observation sans **activité** nommée (projet, oral, travail de groupe, engagement) ; aucun texte libre ; purge fin de cycle + 1 an (R-C22, R-C23).
- **C — Élève à deux équipes** (ULIS, SEGPA, changement de classe en fin d'année) : bilan proposé et arrêté par la classe de rattachement **à la date du conseil** ; le coordonnateur de dispositif est **co-rédacteur de droit** et invité au point de l'élève ; un changement de classe transfère le bilan avec ses avis (R-C24).

### 28.2 Actions ajoutées
| Id | Action | Qui · droit | Règles | Manque |
|---|---|---|---|---|
| CMP-240 | Saisir un niveau pour toute une ligne (élève) ou toute une colonne (élément) | PRF · `competences.positionner` | puis ajustement case par case | M22-002 |
| CMP-241 | Lister les élèves « pas assez d'éléments » et ouvrir une observation ciblée avant la clôture | PRF | §7.3 ; R-C22 | M22-003 |
| CMP-242 | Enregistrer et réutiliser une **évaluation type** (éléments, seuils par exercice) | PRF | entre classes et années | M22-004, M22-007 |
| CMP-243 | Afficher « en progrès / stable / en recul » par élément (deux derniers positionnements, déterministe) | SYS → PAR, ELV | jamais une couleur seule | M22-005 |
| CMP-244 | Montrer à la création de l'évaluation ce que la famille verra | SYS → PRF | selon le mode du niveau | M22-006 |
| CMP-245 | Avertir d'un élément d'une autre discipline que celle du cours | SYS → PRF | non bloquant | M22-009 |
| CMP-246 | Vue « bilan de classe » (élèves × 8 composantes, proposition et choix, clavier) | PP | — | M22-010 |
| CMP-247 | Afficher la pièce du LSU antérieur dans le dossier d'appui | PP | origine externe | M22-011 |
| CMP-248 | Demander à des collègues un positionnement ciblé (élève, composante, échéance) | PP | tâche au professeur ; R-C20 | M22-012 |
| CMP-249 | Rappeler à J-7 du conseil les composantes vides | SYS → PP, PRF | regroupé au résumé | M22-014 |
| CMP-250 | Voir les changements d'un bilan entre proposition et arrêt | PP, DIR | historique | M22-015 |
| CMP-251 | Faire cohabiter deux versions d'un référentiel la même année (par niveau d'entrée ou par cohorte) | CDD | R-C27 | M22-016, M22-040 |
| CMP-252 | Rattacher un élément à une composante principale et à des composantes secondaires | CDD | R-C26 | M22-017 |
| CMP-253 | Copier un référentiel vers un autre établissement de l'organisme | CDD (soi) | geste de la personne, jamais lecture croisée (D-013) | M22-018 |
| CMP-254 | Viser en lot avec filtres (sans non évaluable, sans correction tardive, sans demande ouverte) | chef · `competences.bilans_viser` ★ | liste des exclus | M22-021 |
| CMP-255 | Paramétrer la date de remontée LSU (pré-contrôle calé) | ADF, DIR | — | M22-024 |
| CMP-256 | Inscrire une attestation au **journal à reporter** (Pix l'an 1 au pilote) | ADF | R-X24 du 9 | M22-025 |
| CMP-257 | Saisir une attestation au bord du bassin (mobile, hors ligne, liste) | PRF d'EPS | « non passé » distinct de « dispensé » ; « à repasser » | M22-030, M22-031 |
| CMP-258 | Rattacher un élément adapté à un élément ordinaire | CDP | §12.3 | M22-035 |
| CMP-259 | Positionner en CECRL (FLS) à l'arrivée et lever « non évaluable » à la date prévue | PRF UPE2A | rappel | M22-037, M22-038 |
| CMP-260 | Partager une grille au tuteur et recevoir ses propositions | PRF référent | socle 39 ; auteur « tuteur, validé par » | M22-041 |
| CMP-261 | Viser le bilan par **chacun** des titulaires (réputé visé dès le premier) | PAR, tiers selon décision | R-V46 : rien de l'autre parent | M22-052, M22-053 |
| CMP-262 | Donner à un personnel une portée par **mission** (documentaliste, référent) sur les élèves de ses séances | DIR | socle 6 | M22-062 |
| CMP-263 | Demander un duplicata d'attestation sans compte (ancien élève) | public → ADF | vérification d'identité ; socle 7 | M22-077 |
| CMP-264 | Purger à fin de cycle + 1 an (positionnements, observations, auto-positionnements, avis internes) avec rapport | SYS, DPO | D-043 pour les sauvegardes | M22-073 |
| CMP-265 | Produire la fiche de traitement du socle pour le registre (socle 27) | DPO | — | M22-071 |
| CMP-266 | Contrôle automatique « aucune donnée de compétence dans les journaux applicatifs » | SYS | test E14 | M22-076 |

### 28.3 Règles ajoutées
| Id | Règle |
|---|---|
| R-C21 | Tant que `recevoir_bulletins` n'existe pas, toute publication à une famille suit `peut_voir_notes` et crée une tâche au secrétariat pour chaque lien qui aurait dû recevoir. |
| R-C22 | Aucune observation ne lit ni ne cite un fait disciplinaire ; aucune observation sans activité nommée ; aucune observation en texte libre. |
| R-C23 | Les éléments du domaine 3 ne se positionnent que sur une activité pédagogique ou un engagement, jamais sur un comportement isolé. |
| R-C24 | Le bilan de fin de cycle est proposé et arrêté par la classe de rattachement à la date du conseil ; le coordonnateur de dispositif est co-rédacteur de droit. |
| R-C25 | L'auto-positionnement est un choix de niveau, sans texte libre, toujours marqué comme tel. |
| R-C26 | Un élément a une composante principale (seule lue par la proposition du bilan) et des composantes secondaires facultatives. |
| R-C27 | Plusieurs versions d'un référentiel peuvent coexister la même année, chacune liée à des niveaux ou à une cohorte ; un élève n'en voit qu'une par discipline. |
| R-C28 | Le visa du bilan par les responsables est individuel ; le bilan est réputé visé dès le premier visa ; aucun responsable ne voit le visa d'un autre. |
| R-C29 | Seul un lien titulaire (ou tiers selon décision) vise ; une requête de visa est idempotente par (bilan, personne). |
| R-C30 | Toute lecture par identifiant d'un élève répond de la même façon à « inexistant » et à « interdit ». |

### 28.4 Cas limites ajoutés
| # | Cas | Règle |
|---|---|---|
| CC-42 | Élève d'ULIS dont la classe de référence change en mai | bilan transféré avec ses avis (R-C24) |
| CC-43 | Coordonnateur absent au conseil | avis déposés lus par le président ; note au président (BUL-124 du 23) |
| CC-44 | Deux versions de référentiel dans la même classe (redoublant) | l'élève suit la version de sa cohorte (R-C27) |
| CC-45 | Élément rattaché à deux composantes | seule la principale nourrit la proposition (R-C26) |
| CC-46 | Observation saisie le jour d'un incident disciplinaire | aucune lecture du 13 ; le contrôle de cohérence ne fait aucun lien (R-C22) |
| CC-47 | Second titulaire qui vise après le premier | enregistré, aucun effet visible par le premier |
| CC-48 | Frère majeur sans lien qui tente de viser | refus (R-C29) |
| CC-49 | Identifiant d'un camarade dans l'URL | même réponse que pour un inexistant (R-C30) |
| CC-50 | Pix passé dans ScolIA, LSL produit par Charlemagne (an 1) | journal à reporter (CMP-256) |
| CC-51 | Tuteur qui propose un niveau pour un élève qui n'est pas son stagiaire | refus (socle 39) |
| CC-52 | Élève allophone dont le « non évaluable » expire | rappel ; jamais converti en niveau |
| CC-53 | Purge d'un positionnement cité par un bilan visé | le bilan visé garde ses niveaux ; l'appui détaillé disparaît (le bilan est un acte autonome) |
| CC-54 | Ancien élève qui demande un duplicata d'un établissement fermé | reprise par l'organisme (M22-078) |
| CC-55 | Demande d'accès d'un parent à un avis interne sur le bilan | communicable au titre du droit d'accès (M22-074), l'écran de saisie de l'avis le dit |

### 28.5 Questions ajoutées
| Id | Question | Recommandation |
|---|---|---|
| Q-CP13 | Visa du bilan : un titulaire ou chacun ? | **chacun peut**, un suffit (R-C28) — *appliqué par défaut* |
| Q-CP14 | Observations du domaine 3 | **sur activité seulement** (R-C23) — *appliqué par défaut* |
| Q-CP15 | Pix au pilote l'an 1 | **doublon** + journal à reporter (CMP-256) — *appliqué par défaut* |
| Q-CP16 | Rattachement multiple d'un élément | **principale + secondaires** (R-C26) — *appliqué par défaut* |
| Q-CP17 | Auto-positionnement en texte libre ? | **non** (R-C25) — *appliqué par défaut* |
| Q-CP18 | Base légale du traitement | mission d'intérêt public / obligation légale [H juridique] — **à valider par le DPO** (F22-6) |
| Q-CP19 | Avis internes communicables ? | **oui** au titre du droit d'accès, l'écran le dit — *appliqué par défaut* |

### 28.6 Compléments de modèle (réserve **2451–2456**, sans JSON, E1–E14)
```
2451  cmp_element_composante       etablissement_id, element_id, composante_id, principale BOOLEAN    -- R-C26 ; remplace cmp_element.composante_id
                                   (colonne gardée en lecture le temps de la reprise)
      cmp_referentiel_version_niveau  etablissement_id, version_id, niveau_id, cohorte_annee_id NULL  -- R-C27
2452  cmp_evaluation_type          id, etablissement_id, auteur_personne_id, libelle ; cmp_evaluation_type_element (element_id, seuils)  -- CMP-242
      cmp_observation_activite     etablissement_id, positionnement_id, nature ('projet','oral','groupe','engagement','tp','autre'),
                                   projet_id NULL                                                    -- R-C22, R-C23
2453  cmp_bilan_coredacteur        etablissement_id, bilan_id, personne_id, role ('pp','coordonnateur_dispositif')   -- R-C24
      cmp_bilan_demande_appui      id, etablissement_id, bilan_id, composante_id, professeur_personne_id, echeance, etat  -- CMP-248
2454  cmp_tuteur_proposition       id, etablissement_id, eleve_id, element_id, niveau_id, tuteur_libelle, recue_le, validee_par NULL  -- CMP-260
      cmp_purge_rapport            id, etablissement_id, cycle_fin_annee_id, nb_lignes, execute_le                  -- CMP-264
```
`cmp_bilan_prise_connaissance` porte déjà un enregistrement par personne (R-C28) ; contrainte unique `(etablissement_id, bilan_id, personne_id)` ajoutée en 2453 (R-C29). Aucune exception D-036.

### 28.7 Lots modifiés
- **C2** : + CMP-240 à 245, R-C22, R-C23, R-C25, R-C30 ; table `cmp_observation_activite`.
- **C4** : + CMP-246 à 250, 254, 261 ; R-C24, R-C28, R-C29 ; co-rédacteurs.
- **C6** : **dépend du lot A14 du socle 4** (bloquant A) ; repli R-C21 testé.
- **C1** : + CMP-251 à 253 ; R-C26, R-C27.
- **C5** : + CMP-256, 257, 263.
- **C7** : + CMP-258, 259, 262. **C9** : + CMP-260.
- **Nouveau critère transverse** : la purge (CMP-264) et la fiche de traitement (CMP-265) sont des critères de fin du socle (avec le socle 27).

### 28.7 bis Réglages signalés ajoutés (D-023)
| Id | Réglage | Recommandation |
|---|---|---|
| S22-09 | Seuils de niveau par exercice (évaluation type) | propres au professeur, aucun réglage d'établissement |
| S22-10 | Limite douce de taille d'un référentiel | avertissement au-delà de 40 items par discipline et cycle |
| S22-11 | « En progrès / stable / en recul » visible des familles | oui par défaut, désactivable |
| S22-12 | Avant/après de l'auto-positionnement | désactivé par défaut |

### 28.8 Contradictions levées
R-C2 (une composante) → **R-C26** ; R-C7 (droit inexistant) → **R-C21** ; CMP-163 (le PP seul) → **R-C24**.

### 28.9 Points pour Florian ajoutés
| Id | Point |
|---|---|
| F22-6 | Base légale et fiche de traitement à faire valider par le **DPO** (Q-CP18) |
| F22-7 | Le lot A14 du socle 4 (`recevoir_bulletins`) devient un **préalable** de la publication aux familles des compétences et des bulletins (bloquant A) : à placer avant la V1 |
| F22-8 | Format du relevé de la plateforme Pix (import) : exemple anonymisé à fournir si l'établissement en reçoit un |
