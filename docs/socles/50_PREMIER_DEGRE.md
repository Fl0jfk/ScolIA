# Socle 50 — Premier degré (écoles maternelles et élémentaires, groupes scolaires)

**Date :** 2026-11-11 — **Statut :** conception de fond, **non décidée** (à relire par Florian). Rédigée **avant** tout code Rust propre au premier degré. Aucun code n'a été écrit ni modifié. Aucun accès à une base, à un secret ni à un environnement de production. Le code Next.js a été **lu seulement**, de façon ciblée, dans le dépôt cloné (rien exécuté, rien construit, rien modifié).
**Branche :** `ent-socle-50-51-conception`, partie de `origin/ent-mc0hoe`.
**Passe personas :** `docs/socles/PERSONAS_50.md` (même branche). Les ajouts qu'elle a provoqués sont faits **ici**, marqués **« ajout personas »**, surtout au §26. **En cas de contradiction, le §26 l'emporte.**
**Socle jumeau :** `51_POST_BAC_APPRENTISSAGE.md` (même branche). Les deux socles sont des **extensions de périmètre** (étage L de `SOCLES.md`) : ils ne créent presque pas de nouveaux domaines, ils disent **ce qui change** dans les socles déjà conçus quand le public est un enfant de 3 à 11 ans (ici) ou un étudiant, un apprenti (51).
**À lire avec :** `CLAUDE.md` ; `docs/DECISIONS.md` D-013, D-017, D-019, D-021, D-022, D-023, D-026, D-029, D-031, D-034, D-035, D-036, D-038 ; `docs/SOCLES.md` §50 ; `docs/PRA_SAUVEGARDES.md` §18.4 (E1–E14) ; conceptions `02_ELEVE.md` (Q-E13), `04_ADMINISTRATIF.md`, `05_STRUCTURE.md` (§5.4), `09_INTEROP.md` (FX-04, X14 bis, D-8), `12_ASSIDUITE.md`, `13_DISCIPLINE.md`, `14_ACCUEIL_SURETE.md`, `18_FORMULAIRES.md`, `20_CAHIER_DE_TEXTES.md`, `22_COMPETENCES.md`, `23_BULLETINS.md`, `24_FINANCES.md`, `34_SANTE.md`, `40_RESTAURATION.md`, `42_SORTIES_VOYAGES.md` (§8.2, NX-66, NX-78, Q-V35).

**Conventions.**
- **[F]** fait vérifié dans le code (Rust : chemins depuis `ent-rust/` ; Next.js : chemins depuis la racine du dépôt) ;
- **[V]** règle lue sur une source officielle (extrait de la page Légifrance, éduscol ou education.gouv.fr ; URL au §5, consultée le 2026-11-11) ;
- **[V-partiel]** règle confirmée par la source mais version consolidée en vigueur non relue en entier ;
- **[NV]** non vérifié ; **[H]** hypothèse de terrain ; **[R]** recommandation, non décidée.

**Numérotation propre au socle** (vérifiée libre sur toutes les branches distantes le 2026-11-11 par `git grep`) :

| Élément | Numéros |
|---|---|
| Défauts de l'existant (Rust et Next.js) | **ED-01**… |
| Actions cibles | **ECO-100 à ECO-279** |
| Règles | **R-ECO1**… |
| Cas limites | **CEC-01**… |
| Questions | **Q-ECO1**… |
| Réglages signalés (D-023) | **S-5000 à S-5019** |
| Lots | **EC0 à EC13** |
| Manques de la passe personas | **M-5000 à M-5099** |

**Plage de migrations imposée par le pilote : 3000–3019** (inscrite dans `MIGRATIONS.md`). Elle ne suit pas la règle « 3000 + 100 × socle » (Q-B2, qui donnerait 8000–8099) ; aucune collision : aucun fichier `30xx_` n'existe sur les branches distantes au 2026-11-11.

Sommaire : §0 En bref · §1 Thèse · §2 Existant · §3 Acteurs · §4 L'année d'une école · §5 Réglementaire · §6 Groupe scolaire · §7 Ce qui change dans les autres socles · §8 Appel et assiduité · §9 Classe et pédagogie · §10 Suivi des apprentissages, LSU · §11 Passage et maintien · §12 Instances de l'école · §13 Commune, périscolaire, ATSEM · §14 ONDE et échanges · §15 Liaison école-collège · §16 Cas limites · §17 IA · §18 Actions · §19 Règles · §20 Droits et minimisation · §21 Événements et notifications · §22 Écrans et mobile · §23 Modèle de données · §24 Lots et migrations · §25 Décisions et questions · §26 Ajout personas · §27 Faits, hypothèses, contradictions.

---

## 0. En bref

1. **Le premier degré n'est pas un « petit collège ».** Un enseignant polyvalent tient toute la journée la même classe, souvent à plusieurs niveaux (CE1-CE2) ; l'appel se fait par **demi-journée** ; l'évaluation est **sans notes chiffrées** dans l'usage courant [H] (bilans périodiques et de fin de cycle du LSU, maternelle sans notes, carnet de suivi) ; les enfants ont 3 à 11 ans, n'ont **pas de compte** utile avant le CM [R], ne sortent **jamais seuls** sans autorisation écrite ; la **commune** est un acteur à part entière (inscription en mairie, cantine, périscolaire, ATSEM) ; le **directeur d'école** n'est pas un chef d'établissement (pas de personnalité morale de l'école publique, pas de pouvoir disciplinaire, décharge partielle, souvent seul au bureau).
2. **Deux publics très différents** :
   - (a) **l'école d'un groupe scolaire privé sous contrat** (école + collège + lycée sous un même organisme de gestion) : c'est le **premier client probable** (Next.js connaît déjà le « groupe scolaire » et le secteur « école ») ; elle veut la même famille, le même compte, la même facture et la même messagerie que le collège ;
   - (b) **l'école publique isolée**, plus tard (D-029 : ouverture au public avant 2028) : ONDE est la base officielle, la commune gère cantine et périscolaire, l'ENT 1er degré relève du schéma directeur (SDET) [NV] ; ScolIA n'y remplace pas ONDE.
3. **La pièce maîtresse est le groupe scolaire** (§6). Aujourd'hui, Rust n'a **qu'un UAI, un secteur et un type de contrat par établissement** (`332_structure_profil_parametres.sql:21-27`) [F] alors qu'une école sous **contrat simple** et un collège sous **contrat d'association** peuvent coexister sous un même tenant ; Next.js devine « école / collège / lycée » **à partir du libellé de la classe** par des expressions régulières, avec un repli « LYCÉE » par défaut et un filtre qui **laisse tout passer** quand la liste est vide (`app/lib/nomenclature-classes.ts:55-80`, `app/lib/stage-sector-scope.ts:30-35`) [F]. ScolIA introduit l'**unité scolaire** : une composante portant un UAI, un degré, un secteur, un contrat, une direction et une entité émettrice ; chaque classe appartient à une unité ; la direction de l'école ne voit que l'école, celle du collège que le collège, la coordination voit tout ; la **famille et l'élève restent uniques** (D-013 respectée : c'est un seul établissement ScolIA).
4. **Le premier degré change presque tous les socles déjà conçus** (§7, tableau de 27 lignes) : appel par demi-journée et registre d'appel (12), aménagement d'assiduité en petite section (12), absence de sanction de l'échelle (13), remise de l'enfant à chaque sortie (14), autorisations parentales et avis de l'enfant (18), cahier journal au lieu du cahier de textes par séance (20), pas de notes par défaut (21), LSU et carnet de suivi de maternelle (22, 23), conseil des maîtres au lieu du conseil de classe (23), ATSEM et AESH (6), commune comme tiers (4, 24, 40, 43), sorties scolaires avec taux d'encadrement (42), PAI à l'école (34).
5. **Réglementaire vérifié** (§5) : ONDE est un traitement créé par l'**arrêté du 25 mai 2023** (écoles publiques **et privées sous contrat**), complété par l'obligation d'usage d'ONDE et de SIECLE posée en **2021** [V-partiel] ; registre d'appel par classe (R131-5), dialogue avec la famille et dossier d'absences (R131-6), **quatre demi-journées non justifiées dans le mois ⇒ équipe éducative réunie, IEN et DASEN saisis** (R131-7, référentiel métier des directeurs, BO 2026-21) [V-partiel] ; aménagement du temps de présence en **petite section, l'après-midi seulement**, demande écrite, transmission à l'IEN sous **2 jours ouvrés**, **silence de 15 jours = accord** (R131-1-1) [V] ; LSU (arrêté du 31 décembre 2015, D311-6 à D311-9) avec bilans périodiques et de fin de cycle **visés par les parents** [V] ; carnet de suivi et **synthèse des acquis de fin de GS** sur modèle national [V] ; redoublement **exceptionnel**, un seul dans le primaire sauf avis de l'IEN, dialogue au plus tard fin du 2e trimestre, **recours en 15 jours** devant la commission départementale d'appel (D321-6, décret 2024-228) [V] ; conseil d'école **au moins une fois par trimestre**, ordre du jour **8 jours** avant, parents élus **en nombre égal aux classes** (D411-1) [V] ; 24 h sur 8 ou 9 demi-journées, **APC 36 h** annuelles par enseignant, dérogation 4 jours (D521-10 à D521-13) [V-partiel].
6. **Ce que ScolIA ne fait pas** : il **n'écrit pas dans ONDE** (aucune interface d'écriture publiée [V-partiel] ; FX-04 de `09_INTEROP.md`) ; il **importe** les extractions CSV d'ONDE (fiches éduscol 16 et 17) [V-partiel] et **compare** ; il ne remplace pas l'application nationale LSU tant que la convention éditeur n'est pas obtenue (D-032, F-06) ; il ne gère pas la cantine d'une école publique à la place de la commune (il échange avec elle).
7. **Existant Rust** : seulement `domain-eleve/src/onde_export.rs` (68 lignes, « P2 stub ») et sa route `GET /exports/onde` [F]. **Lot EC0** (correctifs urgents, §2.2) : l'export porte le nom ONDE sans en suivre aucun format, **inclut les élèves sortis**, **ignore la date**, **coupe à 500 lignes sans le dire**, **n'est pas protégé contre l'injection de formules** (contrairement à LSU et LSL) et lit le texte `classe` hérité.
8. **Le plan** :

   | Élément | Volume |
   |---|---|
   | Actions | 143 (numérotées ECO-100 à ECO-279 par blocs, trous réservés ; dont 6 ajoutées par la passe personas, ECO-274 à ECO-279) |
   | Règles | 45 (R-ECO1 à R-ECO45, dont 5 de la passe personas) |
   | Cas limites | 64 (CEC-01 à CEC-64, dont 8 de la passe personas) |
   | Réglages signalés | 16 (S-5000 à S-5015, dont 2 de la passe personas) |
   | Lots | 14 (EC0 à EC13) |
   | Migrations | 3000–3019 |

   **Parcours minimal [R]** : EC0 (correctifs), EC1 (unités scolaires du groupe), EC2 (structure et calendrier d'école), EC3 (appel par demi-journée et registre), EC4 (remise de l'enfant), EC7 (LSU interne et carnet de suivi). Le premier degré n'entre **pas** dans la V1 de février 2027 sauf si l'établissement de Florian a une école dans son groupe (Q-ECO1, Q-E de `SOCLES.md` : « après, sauf client pilote qui l'exige »).
9. **Décisions appliquées par défaut (D-031)** : 24 (§25.1). **Soumis à Florian** (§25.2) : l'école du groupe pilote existe-t-elle et entre-t-elle dans l'année de doublon ; une ou deux personnes morales (OGEC unique ou non) ; conventions avec la ou les communes (cantine, périscolaire, transmission des listes) ; démarche éditeur ONDE / LSU (Pléiade) ; choix d'un ENT 1er degré du SDET pour les écoles publiques (ScolIA n'est pas labellisé) ; accès des ATSEM, agents de la commune.

---

## 1. Thèse : une école, c'est une classe toute la journée, un adulte responsable à chaque instant, et une famille qu'on voit chaque jour

Au collège, l'élève circule et l'adulte change toutes les heures ; l'information circule par le logiciel. À l'école, **l'enfant ne quitte presque jamais sa classe ni son enseignant**, et la famille le dépose et le reprend **en personne** à la grille. Le logiciel n'a donc pas la même mission :
- **pour l'enseignant**, il doit **disparaître** : appel en deux gestes le matin et l'après-midi, cahier journal tenu à la semaine, observations de maternelle prises en photo sur la tablette, bilans de fin de période préparés à partir de ce qu'il a observé ; aucune saisie « de collège » (séance par séance, notes sur 20) ne lui est imposée ;
- **pour le directeur**, il doit **porter l'administration qu'il fait seul** : admissions et radiations (alignées sur ONDE), registre d'appel, absences du mois et seuil des quatre demi-journées, aménagements de petite section, conseil d'école, conseils des maîtres et de cycle, APC, PPMS, relations avec la mairie, liste de qui vient chercher chaque enfant ;
- **pour la famille**, il doit **rassurer et simplifier** : qui vient chercher l'enfant ce soir, l'absence signalée en un geste, le cahier de liaison qui ne se perd pas, les autorisations signées par les bons titulaires, le carnet de réussites de maternelle, le bilan expliqué ;
- **pour le groupe scolaire**, il doit **relier sans mélanger** : une famille, des enfants de 4, 12 et 17 ans, une facture, une messagerie, mais trois directions qui ne voient que leur unité et un passage CM2 → 6e qui transmet ce qu'il faut et rien de plus.

---

## 2. Évaluation franche de l'existant

### 2.1 Ce qui existe [F]

| Élément | Où | Constat |
|---|---|---|
| Export « ONDE » CSV | `crates/domain-eleve/src/onde_export.rs` ; `crates/ent-db/src/service.rs:15073-15108` ; route `GET /exports/onde` (`crates/ent-api/src/routes.rs:594`, `7551-7578`) | Stub « P2 » : colonnes `ine;nom;prenom;date_naissance;sexe;classe;mef` inventées ; réservé aux administratifs (`exiger_export_officiel`, D-022) et tracé (`tracer_acces`) : **bons points à garder** |
| Niveaux avec cycle | `migrations/110_referentiel_structure.sql:25-39` (`cycle` 1 à 4) | Le cycle 1 (maternelle) et les cycles 2-3 sont représentables |
| Sites avec UAI | `110:8-15` | Un UAI par site, mais un site peut accueillir école et collège d'UAI distincts |
| Profil d'établissement | `migrations/332_structure_profil_parametres.sql:13-50` | **Un seul** UAI, `secteur`, `contrat_type` pour tout le tenant ; `degres TEXT[]` (`premier`, `second`, `post_bac`) ; `jours_ouvres JSONB` (déjà listé « à normaliser » par D-036) |
| Calendrier, demi-journées | `333_structure_calendrier.sql` ; `05_STRUCTURE.md` §5.4 | Grille à deux créneaux possible ; semaine de 4 jours = paramétrage |
| Classes à plusieurs niveaux | `05_STRUCTURE.md` §9.6 (0..n niveaux) | Conçu, à vérifier dans le code au lot EC2 [NV] |
| Next.js : niveaux PS à CM2 | `app/lib/document-templates/inscription-levels.ts:15-22` | Inscriptions de l'école déjà prévues (modèles de dossier) |
| Next.js : « groupe scolaire » | `app/components/settings/SettingsSitePanel.tsx:68-72`, `app/components/platform/PlatformTenantEditor.tsx:26,321`, `app/components/onboarding/chapters/ChapterWelcome.tsx:53,100`, `app/souscrire/page.tsx:84,215` | Type d'organisation « groupe scolaire » (plusieurs sites) au niveau du tenant |
| Next.js : secteurs école / collège / lycée | `app/lib/stage-sector-scope.ts`, `app/lib/pilotage-eleves-access.ts:44-70`, `app/lib/nomenclature-classes.ts:55-80` | Direction « par secteur » : un directeur de l'école ne voit que l'école |
| Next.js : école hors SIECLE | `app/components/settings/SchoolRosterPanel.tsx:324-340` | « École : catalogue libre (hors Siècle pour l'instant) » |
| Next.js : récapitulatif APEL par niveau | `app/components/travels/TravelsComptaApelSummaryModal.tsx:113` | École, collège, lycée, groupe scolaire |

### 2.2 Défauts (lot EC0 pour ceux de Rust ; Next.js reste intouchable : signalés à Florian)

| # | Gravité | Défaut | Preuve | Correctif |
|---|---|---|---|---|
| **ED-01** | élevée | L'export « sans filtre de classe » prend `search(etab, "", 500, true)` : **tous les élèves de la table**, sortis et radiés compris (aucun filtre d'état ni de date, alors que le commentaire dit « élèves actifs au jour ») | `service.rs:15086-15091`, `eleve_repo.rs:117-160` | EC0 : élèves **scolarisés à la date** (port du socle 2), date obligatoire |
| **ED-02** | élevée | **Coupure silencieuse à 500 lignes** : une école de 220 élèves passe, un groupe scolaire de 1 400 non, sans avertissement | `service.rs:15090` | EC0 : pagination serveur ou flux ; compte annoncé dans l'en-tête de réponse |
| **ED-03** | élevée | **Injection de formule** non neutralisée (`=`, `+`, `-`, `@`, tabulation en tête), alors que `lsun_export.rs:41-50` et `lsl_export.rs:43-52` la neutralisent ; même défaut dans `notanet_export.rs:30` et `parcoursup_export.rs:40` | `onde_export.rs:35-41` | EC0 : un seul échappeur partagé (celui de LSU), test commun |
| **ED-04** | moyenne | Le format porte le nom **ONDE** sans correspondre à rien d'officiel ; une directrice pourrait croire pouvoir l'importer | `onde_export.rs:1, 19` | EC0 : renommer « liste des élèves (format ScolIA) », retirer « ONDE » des libellés et du nom de fichier tant qu'il n'existe pas de format publié (§14) |
| **ED-05** | moyenne | Le paramètre `jour` est accepté mais **ignoré** sans filtre de classe | `service.rs:15082-15091` | EC0 (avec ED-01) |
| **ED-06** | moyenne | Lit les colonnes texte héritées `classe`, `mef` au lieu de l'affectation datée et du MEF du socle 5 | `service.rs:15103-15104` | EC2, quand l'affectation datée est la source |
| **ED-07** | moyenne | Un seul UAI, secteur et contrat par tenant : impossible de décrire une école sous contrat simple et un collège sous contrat d'association, ou deux UAI sur un même site | `332:21-27` | EC1 : unités scolaires (§6) |
| **ED-08** | faible | `degres TEXT[]` : liste sans clé ni date (D-036, tableau servant de mémoire) | `332:27-28` | EC1 : déduit des unités, colonne retirée au lot de retrait |
| **ED-09** | info (Next.js) | Secteur **deviné par expression régulière** sur le libellé de classe, repli « LYCÉE » par défaut : une classe « Ulis école » ou « Petits-Moyens » est classée lycée | `app/lib/nomenclature-classes.ts:55-80` | Rust : rattachement explicite de la classe à son unité (R-ECO2) |
| **ED-10** | info (Next.js) | Filtre par secteur **ouvert par défaut** : liste de secteurs vide ⇒ tout est visible ; classe non reconnue ⇒ visible de toutes les directions | `app/lib/stage-sector-scope.ts:30-35, 52-60` | Rust : portée **fermée par défaut** (R-ECO3) |
| **ED-11** | info (Next.js) | Dossier de voyage « groupe scolaire » **signable par personne** (déjà VD-40 du 42) | `42_SORTIES_VOYAGES.md` NX-78, VD-40 | Q-V35 : sorties liées, chaque unité signe pour ses participants (repris R-ECO4) |

### 2.3 À ne pas perdre de Next.js (D-035)
- le choix « établissement unique / groupe scolaire » à l'ouverture (repris en STR-01 du socle 5 : profil « ensemble scolaire ») ;
- la **direction par secteur** (même idée que l'unité, rendue explicite) ;
- les niveaux **PS à CM2** dans les dossiers d'inscription ;
- le récapitulatif **par niveau d'enseignement** pour l'association de parents (APEL) ;
- les classes d'école ajoutées **à la main** (pas de SIECLE pour l'école) : ScolIA ajoute l'import ONDE (§14) sans retirer la saisie.

### 2.4 Verdict
Rien n'est à reprendre du code Rust du premier degré au-delà des deux bons réflexes de l'export (droit d'export officiel, trace d'accès). Le socle se construit **sur** les socles 2 à 29 : il ajoute des tables là où le premier degré a des objets propres (aménagement de petite section, cahier journal, carnet de suivi, synthèse de GS, conseils des maîtres, conseil d'école, APC, commune, ATSEM, import ONDE) et, pour le reste, il **paramètre** les socles existants par l'unité et le cycle.

---

## 3. Acteurs, besoins réels, irritants

| Acteur | Ce qu'il fait vraiment | Ce qui lui fait perdre du temps / ce qu'il redoute | Ce qui le ravirait |
|---|---|---|---|
| **Directrice d'école** (souvent chargée de classe, décharge partielle [V-partiel]) | Admissions, radiations, certificats, registre, absences du mois, conseil d'école, élections, PPMS, APC, relations mairie, RASED, équipes éducatives | Ressaisir ONDE et le logiciel ; tableaux de présence papier ; courir après les autorisations ; être joignable partout ; erreurs de remise d'enfant | Un tableau « ce matin » : absents non signalés, qui vient chercher qui, remplaçant attendu ; les absences du mois déjà comptées ; les convocations du conseil d'école générées |
| **Enseignant polyvalent** | Appel matin et après-midi, cahier journal, préparation, observation, évaluation, cahier de liaison, sorties | Saisie séance par séance inutile ; double outil (cahier journal papier + ENT) ; bilans du LSU à saisir à la main | Appel en 10 s ; cahier journal à la semaine, reconduit ; bilans pré-remplis de ses observations, qu'il relit |
| **Enseignant de maternelle** | Observations, photos de réussites, carnet de suivi, accueil échelonné, sieste, ATSEM | Photos sur son téléphone personnel ; carnet papier collé ; droit à l'image inconnu | Prendre la photo sur la tablette de l'école, l'enfant et la compétence étiquetés, le carnet qui se construit, photo interdite si refus de droit à l'image |
| **Enseignant remplaçant (brigade, TR)** | Arrive à 8 h 20 dans une école inconnue | Ne sait pas qui vient chercher, qui a un PAI, qui ne doit pas être remis | Une **fiche de classe du jour** (photos, PAI minimal, remises interdites, sorties), visible seulement le jour du remplacement |
| **Enseignant de RASED, maître E/G, psychologue EN** | Aides spécialisées, bilans | Dossiers dispersés, confidentialité | Un accès limité aux élèves suivis, sans le reste (socle 36) |
| **ATSEM** (agent de la commune) | Accueil, hygiène, sieste, aide en classe, parfois cantine et garderie | Ne pas savoir qui a une allergie ; ne pas savoir qui peut venir chercher | Voir pour **sa** classe et **son** temps : PAI minimal, remises, photo ; rien d'autre |
| **AESH** | Accompagne un ou plusieurs enfants | Accès à tout ou à rien | Ne voir que les enfants accompagnés et leurs aménagements (socle 35) |
| **Accueil / secrétariat du groupe scolaire** | Téléphone, absences signalées, retards, parents à la grille | Absences notées sur un post-it | Saisir l'absence signalée et la transmettre à l'enseignant (D-023, exemple vécu) |
| **Chef d'établissement coordinateur (privé)** | Pilote école + collège + lycée, OGEC | Trois outils, trois listes de familles | Une base, des directions par unité, des indicateurs par unité et consolidés |
| **Comptable de l'OGEC** | Contribution des familles, cantine, garderie, fratries | Fratrie entre école et collège calculée à la main | Une facture par foyer payeur pour tout le groupe (socle 24, entité émettrice) |
| **Mairie (service des affaires scolaires)** | Inscription administrative (certificat d'inscription), cantine, périscolaire, ATSEM, transports | Listes envoyées par mail, en retard, fausses ; données de santé envoyées sans précaution | Une liste à jour des inscrits par école et par classe, **minimale**, avec les régimes de restauration, par un échange tracé |
| **IEN de circonscription, conseiller pédagogique** | Inspection, aménagements de PS, maintiens, signalements d'absentéisme | Dossiers incomplets | Des demandes complètes, datées, transmises par la directrice (aucun accès direct dans ScolIA [R]) |
| **Famille** (dont séparée, en résidence alternée) | Dépose, reprend, signe, paie, lit | Ne pas savoir qui vient chercher (l'autre parent ?) ; cahier de liaison perdu ; papier à signer le matin même | Déclarer « mamie vient ce soir » en un geste, signer en ligne, voir le carnet de réussites |
| **Enfant de 3 à 11 ans** | Vit sa journée | — | Ne jamais être remis à la mauvaise personne ; voir ses réussites (maternelle) ; à partir du CM, un espace simple (Q-ECO6) |
| **Représentant de parents élu** | Siège au conseil d'école | Pas d'ordre du jour, pas de procès-verbal | Convocation, ordre du jour, PV publié, liste de diffusion des parents (socle 46) |
| **DDEN** (délégué départemental de l'éducation nationale) | Membre du conseil d'école [V] | Convocation oubliée | Convoqué automatiquement, sans accès aux données des élèves |

---

## 4. L'année d'une école [H, à confirmer avec Florian]

- **Janvier–juin (N+1)** : inscriptions en mairie (certificat d'inscription), admissions par la directrice, rendez-vous des nouvelles familles de PS, prévisions d'effectifs (fiche ONDE 08 [V-partiel]), répartition des classes (conseil des maîtres), demandes d'aménagement de PS pour la rentrée.
- **Fin juin** : conseil des maîtres (propositions de passage, maintiens exceptionnels avec dialogue préalable), synthèses de fin de GS transmises à l'élémentaire, bilans de fin de cycle (CE2, CM2), liaison CM2-6e (conseil école-collège), radiations (certificats), troisième conseil d'école.
- **Rentrée** : constat d'effectifs (ONDE), accueil échelonné des PS, autorisations de remise et de sortie, droit à l'image, PAI renouvelés, assurance [V-partiel : non exigible pour les activités obligatoires], réunions de rentrée par classe, élections des représentants de parents (octobre), premier conseil d'école dans le mois qui suit [V].
- **Toute l'année** : appel par demi-journée, registre, absences du mois, APC, cahier journal, sorties (régulières, occasionnelles, avec nuitée), exercices PPMS, équipes éducatives, bilans périodiques (2 ou 3 selon le choix de l'école [NV]).

---

## 5. Ce qui est réglementaire (vérifié le 2026-11-11)

| # | Règle | Texte | Statut | Conséquence pour le logiciel |
|---|---|---|---|---|
| RE-01 | **ONDE** : traitement de gestion administrative et pédagogique des élèves du 1er degré, de pilotage et de **contrôle de l'obligation d'instruction** ; écoles maternelles et élémentaires **publiques et privées sous contrat** ; accès : directeur, DASEN et services, DGESCO, DNE, DSI ; **maire de la commune de résidence** dans la limite de ses missions ; données : identification, identité, coordonnées de l'élève, des responsables, des personnes autorisées à venir chercher et à contacter en urgence, CSP | Arrêté du 25 mai 2023 (JORFTEXT000047704245) ; historique : arrêté du 20 octobre 2008, annulation partielle CE 19 juillet 2010 n° 317182, renommage en 2017 | [V-partiel] (intitulé « création » sur Légifrance, « modification » dans un manuel éduscol : Q-ECO10) | ONDE est **la** base officielle ; ScolIA en est un **miroir de travail** ; le **droit d'opposition** pour motif légitime existe (annulation CE 2010) : il vise ONDE, pas ScolIA, mais la notice de ScolIA doit le mentionner [R] |
| RE-02 | Usage **obligatoire** d'ONDE (et SIECLE au 2nd degré) par les écoles publiques et privées pour enregistrer ces données | Arrêté du 14 décembre 2021 [V-partiel, cité par la recherche, non relu] | [V-partiel] | Pas de « ScolIA au lieu d'ONDE » ; double saisie à réduire par import (§14) |
| RE-03 | **Extractions ONDE** : liste des élèves de l'école en CSV, filtrée par niveau, dispositif, classe, regroupement, élèves **admis définitivement** dans l'année ; extractions personnalisées (champs élève, responsables, scolarité) ; recommandation de garder nom, prénom, **INE**, école, **UAI** pour les rapprochements | Fiches éduscol ONDE n° 16 (extractions personnalisées, 27-02-2026) et n° 17 (listes par commune de résidence) | [V-partiel] (format décrit en texte, pas de schéma publié) | Import **tolérant** (en-têtes reconnus, pas positions), rapport de rapprochement, jamais d'écrasement silencieux (R-ECO20) |
| RE-04 | **Registre d'appel** tenu par classe dans chaque école ou établissement, public ou privé, qui recense les absences | C. éduc. R131-5 | [V-partiel] | Le registre est **produit** par ScolIA (édition mensuelle par classe, §8) |
| RE-05 | Absences répétées : le directeur **engage un dialogue** avec les responsables ; absences consignées dans un **dossier** de l'année | R131-6 | [V-partiel] | Fil de dialogue tracé (socle 12, dossier annuel) |
| RE-06 | **Au moins quatre demi-journées** sans motif légitime ni excuse valable **dans le mois** ⇒ le directeur **réunit l'équipe éducative** ; l'IEN et le DASEN sont saisis | R131-7 ; référentiel métier des directeurs d'école (BO n° 21 de 2026, MENE2612947C) ; règlement type départemental | [V-partiel] (décret 2022-182 a modifié la sous-section : version consolidée à relire, Q-ECO11) | Compteur **par mois civil** des demi-journées non justifiées ; alerte au seuil ; **jamais d'envoi automatique** à l'IEN (personne humaine, R-ECO12) |
| RE-07 | Instruction obligatoire de **3 à 16 ans** ; **aménagement du temps de présence en petite section, l'après-midi seulement**, à la demande écrite et signée des responsables ; le directeur transmet **avec son avis** à l'IEN sous **2 jours ouvrés** ; avis favorable ⇒ mise en œuvre **provisoire** ; **silence de 15 jours = accord** ; modifiable en cours d'année par la même voie | L131-1, R131-1-1 (décret 2019-826) | [V] | Objet « aménagement » avec échéances calculées (§8.3) ; les demi-journées aménagées ne sont **pas** des absences |
| RE-08 | **LSU** du CP à la 3e : bilans **périodiques** et bilans **de fin de cycle** (cycles 2, 3, 4 ; maîtrise de chaque composante du socle commun, appréciation, conseils), attestations ; à l'école élémentaire, bilans **signés par les enseignants, le directeur et les parents** ; numérisés dans l'application nationale ; suivent l'élève au changement d'école | Décret 2015-1929 ; arrêté du 31 décembre 2015 (articles 2 à 9) ; C. éduc. D311-6 à D311-9 | [V] | Bilans internes conformes au contenu ; **visa des parents** en ligne (socle 18) ; export vers LSU seulement avec convention éditeur (§14) |
| RE-09 | **Maternelle** : **carnet de suivi des apprentissages** tout au long du cycle 1, renseigné par l'enseignant à une fréquence adaptée, suit l'élève au changement d'école ; **synthèse des acquis** en fin de GS selon un **modèle national**, sans épreuve spécifique, transmise à l'école élémentaire et communiquée aux parents | Décret 2015-1929 art. 8 ; arrêté du 31 décembre 2015 (MENE1527416A) | [V] (programmes révisés en 2026 : le modèle peut évoluer, Q-ECO12) | Carnet = observations datées (+ photos, avec droit à l'image) ; synthèse sur trame nationale |
| RE-10 | **Redoublement** exceptionnel, après un accompagnement insuffisant ; décidé par le **conseil des maîtres** présidé par le directeur ; **dialogue préalable** avec les représentants légaux **au plus tard fin du 2e trimestre** ; **un seul** redoublement ou raccourcissement dans tout le primaire, un second à titre exceptionnel **après avis de l'IEN** ; élève handicapé : avis de l'IEN ; **recours en 15 jours** devant la commission départementale d'appel présidée par le DASEN, parents entendus à leur demande, décision **définitive** | D321-6 (décret 2024-228), D321-8 | [V] | Procédure de maintien complète (§11) ; compteur de maintiens sur toute la scolarité primaire (socle 2) |
| RE-11 | **Conseil d'école** : présidé par le directeur ; représentants des parents élus **en nombre égal au nombre de classes** ; IEN de droit ; maire ou représentant, conseiller municipal, un maître du RASED, DDEN… ; **au moins une réunion par trimestre**, obligatoirement dans le mois qui suit la proclamation des résultats ; ordre du jour adressé **8 jours** avant ; regroupement possible de conseils | D411-1 à D411-3 (version du 16 août 2023) | [V] | Instance avec composition calculée (§12), convocations générées, PV ; élections au socle 46 |
| RE-12 | Semaine de **24 h** d'enseignement sur **9 demi-journées** ; dérogation : **8 demi-journées sur 4 jours**, au moins 5 matinées, 6 h par jour au plus, sur proposition conjointe de la commune et des conseils d'école, décision du DASEN pour **3 ans** au plus ; **APC** en petits groupes, 36 h annuelles par enseignant | D521-10 à D521-13 ; décret 2017-1108 | [V-partiel] | Rythme d'école paramétré (§9.1) ; contrôle des 6 h/jour ; APC = activité appelée avec accord de la famille [NV : accord écrit exigé ?] |
| RE-13 | Sorties scolaires du 1er degré : délais de demande (3 jours, 15 jours, 4–6 semaines), **autorisation de l'IEN** ou du DASEN pour les nuitées, **taux d'encadrement** | Repris de `42_SORTIES_VOYAGES.md` §5 et §8.2 (circulaire du 16 juillet 2024 et guides éduscol de décembre 2025) | [V] par le socle 42 | Le moteur de contexte « premier degré » du 42 (V8) s'applique ; calcul bloquant des taux |
| RE-14 | **Pas de sanctions de l'échelle** au 1er degré ; réprimandes portées à la connaissance des représentants ; jamais d'atteinte à l'intégrité ; équipe éducative en cas de trouble grave | Repris de `13_DISCIPLINE.md` l. 276 (circulaire 2014-088, règlement type) | [V] par le socle 13 | Le socle 13 n'ouvre au 1er degré que l'observation, l'information de la famille et l'équipe éducative (§7) |
| RE-15 | **Redoublement d'un élève handicapé** et aménagements : PPS, ESS, AESH | D321-6 ; socle 35 | [V-partiel] | Avis IEN requis dans la procédure (§11) |
| RE-16 | **Conseil école-collège** : liaison du cycle 3 (CM1, CM2, 6e) | D401-1 et s. | [NV] | Instance du §15 ; aucun partage de données élève au-delà de ce que prévoit la transmission du LSU |
| RE-17 | **Directeur d'école** : délégation de compétences de l'autorité académique, autorité fonctionnelle (loi 2021-1716 dite « Rilhac ») | Loi du 21 décembre 2021 | [NV] (non retrouvée par la recherche) | Rôles « direction d'école » avec droits propres ; aucune logique ne dépend de la loi elle-même |
| RE-18 | **Privé sous contrat** : classes sous contrat d'association ou contrat simple ; mêmes applications nationales | L442-5, L442-12 ; RE-01, RE-02 | [NV] pour les articles ; [V-partiel] pour ONDE | Contrat **par unité** (§6), pas par tenant |
| RE-19 | **CNIL** : traitement de données d'enfants, droit à l'image, minimisation ; l'ENT relève du schéma directeur (SDET) [NV] | RGPD art. 8 (consentement des mineurs pour les services en ligne : 15 ans en France, loi 78-17 art. 45) [NV] ; SDET | [NV] | Aucun compte enfant obligatoire ; consentements donnés par les titulaires de l'autorité parentale (socle 18) |

**Sources (consultées le 2026-11-11)** :
- Arrêté du 25 mai 2023 (ONDE) : https://www.legifrance.gouv.fr/jorf/id/JORFTEXT000047704245 et article 6 : https://www.legifrance.gouv.fr/jorf/article_jo/JORFARTI000047704263
- Arrêté du 20 octobre 2008 (historique) : https://www.legifrance.gouv.fr/loda/id/JORFTEXT000019712192/2020-10-06/ ; BO 2017 n° 15 : https://www.education.gouv.fr/bo/17/Hebdo15/MENE1701369A.htm
- Fiches et manuels ONDE (éduscol) : fiche 16 https://eduscol.education.gouv.fr/sites/default/files/document/20260227-fiche-16-extractions-personnaliseespdf-126278.pdf ; fiche 17 https://eduscol.education.gouv.fr/sites/default/files/document/onde-fiche-17-extraction-des-listes-deleves-par-commune-de-residence-dec-127079.pdf ; fiche 08 https://eduscol.education.gouv.fr/sites/default/files/document/onde-fiche-08-previsions-des-effectifs-dsden-127052.pdf ; manuel mairie https://eduscol.education.gouv.fr/sites/default/files/document/manuel-de-reference-mairie-121235.pdf ; guide ÉduConnect 1D https://educonnect.education.gouv.fr/educt-aide/guides/EduConnect_Guide1D_Administration_Comptes.pdf
- Contrôle de l'assiduité R131-5 à R131-10 : https://www.legifrance.gouv.fr/codes/section_lc/LEGITEXT000006071191/LEGISCTA000006182466/ ; R131-5 : https://www.legifrance.gouv.fr/codes/article_lc/LEGIARTI000025164771/ ; R131-6 : https://www.legifrance.gouv.fr/codes/article_lc/LEGIARTI000006525784 ; référentiel métier des directeurs (2026) : https://www.education.gouv.fr/bo/2026/Hebdo21/MENE2612947C ; règlement type départemental : https://www.education.gouv.fr/bo/14/Hebdo28/MENE1416234C.htm
- Petite section : R131-1-1 https://www.legifrance.gouv.fr/codes/article_lc/LEGIARTI000038882474 ; décret 2019-826 https://www.legifrance.gouv.fr/jorf/id/JORFTEXT000038874655
- LSU : arrêté du 31 décembre 2015 https://www.legifrance.gouv.fr/jorf/id/JORFTEXT000031742317 ; décret 2015-1929 https://www.legifrance.gouv.fr/jorf/id/JORFTEXT000031742252 ; D311-6 à D311-9 https://www.legifrance.gouv.fr/codes/id/LEGISCTA000038895269 ; fiche technique LSU https://eduscol.education.gouv.fr/sites/default/files/document/fichetechlsudirpe630051pdf-74706.pdf
- Maternelle : https://eduscol.education.gouv.fr/4671/suivi-et-evaluation-des-apprentissages-des-eleves-l-ecole-maternelle ; modèle national de synthèse https://www.education.gouv.fr/bo/16/Hebdo3/MENE1527416A.htm
- Redoublement : D321-6 https://www.legifrance.gouv.fr/codes/article_lc/LEGIARTI000049289644 ; décret 2024-228 https://www.legifrance.gouv.fr/jorf/id/JORFTEXT000049286365
- Conseil d'école : D411-1 https://www.legifrance.gouv.fr/codes/article_lc/LEGIARTI000047971063
- Temps scolaire : D521-1 à D521-13 https://www.legifrance.gouv.fr/codes/id/LEGISCTA000020743271 ; décret 2017-1108 https://www.legifrance.gouv.fr/jorf/id/JORFTEXT000035022480 ; éduscol https://eduscol.education.gouv.fr/5073/organisation-du-temps-scolaire-dans-le-premier-degre

**Reste à vérifier avant les lots concernés** (Q-ECO10 à Q-ECO14) : version consolidée de R131-6/R131-7 après le décret 2022-182 ; texte de l'arrêté de 2021 sur l'usage obligatoire d'ONDE ; nombre de bilans périodiques au 1er degré ; accord écrit des familles pour les APC ; loi 2021-1716 ; conseil école-collège D401-1 ; SDET 1er degré ; spécifications ONDE V2 « interface école » (D-8 de `09_INTEROP.md`, inscription Pléiade).

---

## 6. Le groupe scolaire : une base, des unités, des directions

### 6.1 Deux sens du mot [R, vocabulaire fixé]
- **Groupe scolaire (public)** : une école maternelle et une école élémentaire (ou une école primaire) sur un même site, souvent avec un seul directeur ou deux [H].
- **Groupe scolaire (privé), ensemble scolaire** : école + collège + lycée (+ BTS, UFA) sous une même tutelle et souvent un même organisme de gestion. C'est le sens de Next.js (« Groupe scolaire Notre-Dame », plusieurs sites).
Dans ScolIA, **les deux** sont un **seul établissement** (un tenant, un sous-domaine, D-012) **découpé en unités scolaires**. C'est la recommandation déjà posée par Q-E13 (`02_ELEVE.md`) et Q-X37 / M-425 (`09_INTEROP.md`, `PERSONAS_09.md`) ; ce socle la rend opérationnelle. Plusieurs tenants seulement si les **entités juridiques et la famille ne doivent rien partager** (Q-ECO2).

### 6.2 L'unité scolaire [R]
Une **unité** est une composante administrative de l'établissement :
- **UAI** propre (ou aucun pour une unité hors contrat ou une section sans UAI distinct) ;
- **degré** (`premier`, `second`, `post_bac`) et **nature** (école maternelle, élémentaire, primaire, collège, lycée général et technologique, lycée professionnel, section de BTS, CPGE, UFA) ;
- **secteur** (public, privé sous contrat, hors contrat) et **contrat** (association, simple) **à la date** : une classe de l'école peut passer du contrat simple au contrat d'association une année donnée ;
- **direction** : un ou plusieurs agents (socle 6) avec la **fonction** « direction de l'unité », datée ;
- **entité émettrice** des documents (socle 7, R-D13) et **entité de facturation** (socle 24) ;
- **site(s)** où elle exerce (n–n : une école sur deux sites, un site pour école et collège) ;
- **calendrier et rythme** : la semaine de 4 jours de l'école et la semaine de 4,5 jours du collège coexistent (§9.1).

Chaque **classe** appartient à **une** unité (R-ECO2) ; un **groupe** transversal (chorale école-collège, APC) déclare les unités qu'il touche ; un **élève** appartient à l'unité de sa classe à la date (déduit, jamais saisi).

### 6.3 Ce que l'unité commande
| Domaine | Effet |
|---|---|
| **Portée des personnels** (socle 1, 6) | Une nouvelle portée **« mon unité »** : la direction de l'école voit les élèves de l'école ; la coordination voit l'établissement. **Fermée par défaut** : un agent sans unité ne voit aucune donnée élève hors de ses cours (R-ECO3, corrige ED-10) |
| **Exports officiels** (9, 22, 23) | Par UAI : ONDE et LSU 1D pour l'école, SIECLE/LSU 2D pour le collège, LSL pour le lycée ; un export ne mélange jamais deux UAI |
| **Documents** (7) | En-tête, signataire et UAI de l'unité de scolarisation à la date du document |
| **Calendrier, rythme** (5, 10) | Rythme, horaires et jours ouvrés **par unité** ; fermeture ciblée sur une unité |
| **Assiduité** (12) | Profil d'assiduité de l'unité (demi-journée et seuil de 4 au 1er degré ; seuils et décompte du 2nd degré) |
| **Instances** (23, 46) | Conseil d'école (école), conseil de classe, CA ou conseil d'établissement (collège-lycée) |
| **Finances** (24) | Grilles et entité de facturation par unité ; **fratrie au niveau du foyer payeur sur tout l'établissement** (FD-22, CF-18 du 24 levés, puisqu'il s'agit d'un seul tenant) |
| **Messagerie, annonces** (16, 17) | Audience « parents de l'école », « parents du groupe » |
| **Sûreté** (14) | PPMS par unité ou commun (M-1415 du 14 : un PPMS commun à plusieurs unités d'un même site) |
| **Sorties** (42) | Sortie **liée** entre unités : chaque direction autorise pour **ses** participants (Q-V35, corrige ED-11) |
| **Pilotage** (47) | Indicateurs par unité et consolidés |

### 6.4 Continuité de la famille et de l'élève
- La famille a **un** compte, voit **tous ses enfants** dans un même écran, reçoit les messages de chaque unité avec l'en-tête de l'unité.
- Le passage **CM2 → 6e dans le même groupe** n'est **pas** une nouvelle inscription : c'est une **réinscription** (socle 3) qui change d'unité ; le dossier administratif (4) et le compte restent ; la transmission pédagogique suit les règles de la liaison (§15) : **le collège ne lit pas** le carnet de maternelle, ni les observations de l'école, il reçoit le LSU (bilans de fin de cycle 3 et bilans périodiques prévus) [R].
- Au départ du groupe : certificat de radiation de l'unité (socle 3), ONDE mis à jour par la directrice.

---

## 7. Ce qui change dans les socles déjà conçus

Chaque ligne est une **attente** posée au socle propriétaire ; ce socle ne l'écrit pas à sa place (règle 1 de `SOCLES.md`). Les numéros entre parenthèses sont ceux du socle propriétaire quand ils existent.

| Socle | Ce qui change au premier degré | Qui le porte | Lot ici |
|---|---|---|---|
| **2 Élève** | Âge de 2 ans (TPS) à 11 ans ; **INE attribué par ONDE** (provenance `import_onde`, déjà prévue `02_ELEVE.md` l. 294) ; compteur des **maintiens et raccourcissements** sur la scolarité primaire (RE-10) ; prénom d'usage ; photo de trombinoscope soumise au droit à l'image (18) | 2 (attente) | EC1, EC10 |
| **3 Inscriptions** | **Inscription en mairie** (certificat d'inscription de la commune) **puis admission** par la directrice ; pièces : livret de famille ou acte, justificatif de domicile, vaccinations obligatoires [NV, Q-ECO13] ; sectorisation communale (public) ; certificat de radiation ; montée CM2 → 6e interne au groupe = réinscription changeant d'unité ; TPS selon places [H] | 3 | EC2, EC11 |
| **4 Dossier administratif** | **Personnes autorisées à venir chercher** l'enfant au cœur (ONDE les connaît, RE-01) ; résidence alternée et **foyer du jour** (déjà au 4) qui décide **qui peut reprendre ce soir** ; **commune de résidence** (déjà `personnes::commune_residence`) ; CSP des responsables (ONDE, RE-01) : **minimisation** : collectée seulement si l'école publique l'exige pour ONDE, jamais dans le privé [R] | 4 | EC4 |
| **5 Structure** | **Unités scolaires** (§6) ; classes **multi-niveaux** ; rythme d'école (8 ou 9 demi-journées) ; cycle déduit du niveau ; cours unique « classe entière, toutes disciplines » (§5.4 du 5) ; **remplacement de `degres TEXT[]` et `jours_ouvres JSONB`** (D-036) par des tables par unité | 5 (accord à obtenir : tables créées dans la plage 3000 du 50, comme le 14 l'a fait pour le 12) | EC1, EC2 |
| **6 Personnels** | **Directeur d'école** (décharge, chargé de classe) ; **enseignant polyvalent** ; **titulaire remplaçant** ; **RASED** ; **ATSEM** (agent de la **commune**, mis à disposition) ; **intervenants** (EPS, langues, musique, agréés) ; **AESH** ; personnel **municipal** du périscolaire | 6 | EC9 |
| **7 Documents** | Certificats de scolarité et de radiation de l'**unité** ; **trames 1er degré** (D-023 : 3 ou 4 par type) ; aucun document « division » révélatrice (D-027 B : « ULIS école ») | 7 | EC1 |
| **8 Notifications** | Destinataires = **titulaires**, jamais l'enfant avant le CM [R] ; **foyer du jour** pour la remise ; SMS de l'absence non signalée à 9 h (Q-N5 en attente : D-028) | 8 | EC3 |
| **9 Interopérabilité** | **ONDE** (X14 bis, FX-04) : import d'extractions CSV et rapprochement ; LSU 1D ; ÉduConnect 1D (ONDE alimente ÉduConnect pour les parents, guide 1D [V-partiel]) ; **pas** de GAR propre au 1er degré tant que l'école n'est pas raccordée par sa collectivité [NV] | 9 | EC12 |
| **10 Emploi du temps** | Emploi du temps **hebdomadaire de classe** sans génération (l'enseignant est seul) ; plages par domaine optionnelles ; **décloisonnements** et échanges de service entre classes ; interventions (EPS municipale, langue) ; APC hors temps de classe ; contrôle **6 h/jour** et 24 h/semaine (RE-12) | 10 | EC2 |
| **11 Mobile** | Appel par demi-journée en deux gestes ; remise de l'enfant (14) à la grille ; carnet de réussites (photo) ; **pas d'appli enfant** avant le CM [R] | 11 | EC3, EC7 |
| **12 Assiduité** | **Appel par demi-journée** (pas de séance) ; **registre d'appel par classe** édité au mois (RE-04) ; seuil **4 demi-journées par mois civil** (RE-06) ; **aménagement de PS** (RE-07) ; **retard de l'enfant** et **retard de la famille au soir** (enfant non repris) ; absence aux APC ; dialogue et équipe éducative | 12 | EC3 |
| **13 Discipline** | **Aucune sanction** ni punition de l'échelle ; observations, information de la famille, **équipe éducative** ; harcèlement (protocole pHARe 1er degré [NV]) ; le module « sanctions » est **désactivé** pour une unité du 1er degré (R-ECO15) | 13 | EC5 |
| **14 Accueil, sûreté** | **Remise de l'enfant à chaque sortie** (11 h 30, 16 h 30, garderie, APC) à une **personne autorisée**, avec pièce si inconnue ; **enfant non repris** à l'heure (procédure, appel des personnes autorisées, mairie, police en dernier recours [H]) ; sortie seul de l'élémentaire **seulement sur autorisation écrite** [NV, règlement type] ; PPMS de l'unité ou commun | 14 | EC4 |
| **16 Messagerie** | **Cahier de liaison numérique** (mot de l'enseignant, signature « vu » par un titulaire) ; pas de messagerie enfant avant le CM [R] ; ATSEM sans messagerie avec les familles par défaut [R] | 16 | EC6 |
| **17 Annonces** | Audience « parents de la classe » simple et fréquente ; affiche de la porte de l'école (photos interdites sans droit à l'image) | 17 | EC6 |
| **18 Formulaires** | **Autorisations de remise** (personnes autorisées, ponctuelles), **sortie seul**, droit à l'image, **APC** (accord famille), **aménagement de PS** (demande écrite et signée, RE-07), **visa du LSU** (RE-08) ; avis de l'enfant non demandé (CK-05 du 18 : moins de 11 ans) | 18 | EC3, EC4, EC7 |
| **19 Agenda** | Réunions de rentrée par classe ; rendez-vous parents-enseignant ; équipe éducative ; **conseil d'école** (convocations, 8 jours) | 19 | EC8 |
| **20 Cahier de textes** | Au 1er degré, le **cahier journal** de l'enseignant (outil professionnel, non publié) remplace le cahier de textes de séance ; le **travail à la maison** (devoirs écrits proscrits à l'élémentaire selon une circulaire ancienne [NV]) est publié sous forme de « à préparer » léger | 20 | EC6 |
| **21 Évaluations** | **Pas de notes par défaut** dans une unité du 1er degré (S-5003) ; évaluations par **niveau de maîtrise** à 4 positions (échelle du socle commun, R-C du 22) ; évaluations nationales (CP, CE1, CM1, CM2) **hors ScolIA** (plateforme nationale) [NV] | 21, 22 | EC7 |
| **22 Compétences** | Bilans de **fin de cycle 2 (CE2) et 3 (CM2)** (déjà prévus `22_COMPETENCES.md` l. 171, 233) ; **APER** (« savoir rouler » [NV]) et **ASSN** (« savoir nager ») en attestations (l. 358 : AER) ; **carnet de suivi de maternelle** et **synthèse de fin de GS** (RE-09) | 22 (attestations), 50 (carnet, synthèse) | EC7 |
| **23 Bulletins, conseils** | **Bilans périodiques LSU** au lieu des bulletins (trame 1er degré, D-023) ; **conseil des maîtres** et **conseil de cycle** au lieu du conseil de classe ; **procédure de maintien** (RE-10) ; **pas** d'avis de passage du conseil de classe au 1er degré | 23 (trames), 50 (conseils des maîtres) | EC7, EC8 |
| **24 Facturation** | Contribution des familles par unité ; **cantine et garderie** (privé) ; **crédit d'impôt pour frais de garde des moins de 6 ans** : attestation qui **sépare garde et nourriture** (déjà `24_FINANCES.md` l. 193) ; fratrie sur tout le groupe | 24 | EC10 |
| **29 Portails** | « Aujourd'hui » de la directrice (absents non signalés, remises du soir, remplaçant, aménagements à échéance) ; « Aujourd'hui » de la famille (qui vient chercher ce soir) | 29 | EC3, EC4 |
| **34 Santé** | **PAI à l'école** (trousse dans la classe, protocole d'urgence, ATSEM et remplaçant informés **minimalement**) ; vaccinations obligatoires à l'admission [NV] ; pas d'infirmière d'école en général [H] : passages **non** tracés comme au 2nd degré, registre des soins de l'école [NV] | 34 | EC4, EC9 |
| **35 Inclusion** | ULIS école, PPS, AESH, RASED, PPRE (déjà mentionné) | 35 | — |
| **40 Restauration** | **Privé** : cantine gérée par l'OGEC (socle 40 complet) ; **public** : cantine **de la commune** : ScolIA **transmet** la liste des présents attendus et les absences du jour à la commune, il ne facture pas (§13) | 40, 50 | EC10 |
| **42 Sorties** | Contexte « premier degré » (V8 du 42) : délais, autorisation IEN, taux d'encadrement, bénévoles agréés ; sorties **liées** du groupe scolaire (Q-V35) | 42 | — |
| **43 Périscolaire** | Garderie du matin et du soir, étude, accueil du mercredi : **commune ou OGEC** ; inscriptions, appel, remise (14), facturation (24) | 43 | EC10 |
| **46 Instances** | **Élection des représentants de parents au conseil d'école** (nombre égal aux classes, RE-11), vote par correspondance ou électronique [NV : vote électronique autorisé ?] | 46 | EC8 |

---

## 8. Appel, registre et assiduité au premier degré

### 8.1 L'appel [R]
- **Deux appels par jour ouvré** (matin, après-midi) **par classe**, pas par séance ; dans une semaine de 4 jours : 8 appels par semaine (RE-12). L'appel du matin est **fermé** à une heure réglée (S-5000 : 9 h 00 par défaut [H]) ; au-delà, les absents sans signalement passent en « absence non signalée », la famille est prévenue (socle 8, Q-N5).
- **Un geste** : la liste de la classe est pré-remplie « présent » ; on touche les absents. Les absences **déjà signalées** (famille, accueil) et les **aménagements de PS** l'après-midi apparaissent déjà.
- **Enfant arrivé en retard** : rectification datée (heure d'arrivée, qui l'a amené) ; l'absence de la demi-journée tombe si l'enfant arrive avant le seuil de demi-journée (S-5001 [H]).
- **Décloisonnement** : l'appel reste fait par l'enseignant **de la classe** (la classe de référence), même si l'enfant passe une heure dans une autre classe (R-ECO8).
- **APC** : appel de l'activité (socle 12, « activités appelées » 2255) ; l'absence aux APC n'est **pas** une absence scolaire (hors temps d'enseignement) mais est signalée à la famille [R].

### 8.2 Le registre d'appel (RE-04) [R]
- **Édition mensuelle par classe** : une grille enfants × demi-journées, avec motif codé, signalement et justification ; arrêtée et **figée** à la fin du mois (socle 12 : mois arrêtés, 2246) ; signature de l'enseignant puis de la directrice (socle 7, signature simple) ; conservation selon la durée du DPO (Q-ECO14).
- Le registre est **produit**, jamais tenu à côté. Une correction après arrêt crée un **avenant daté** (pas de réécriture).

### 8.3 L'aménagement du temps de présence en petite section (RE-07) [R]
1. La famille dépose une **demande écrite et signée** (formulaire du 18, signé par un titulaire ; si les deux titulaires sont connus et en désaccord : socle 18, désaccord connu) précisant les après-midi concernés.
2. La directrice donne son **avis** après dialogue avec l'équipe ; ScolIA calcule l'échéance « **transmettre avant J+2 jours ouvrés** » et l'affiche dans l'à-faire (29).
3. La directrice **transmet** à l'IEN (hors ScolIA : courrier ou messagerie académique ; ScolIA produit le PDF et enregistre la date de transmission).
4. Avis favorable ⇒ l'aménagement est **appliqué à titre provisoire** dès la date choisie ; l'appel de ces après-midi affiche « aménagement » (ni présent ni absent).
5. ScolIA calcule « **accord tacite le J+15** » après transmission ; la directrice enregistre la décision expresse si elle arrive (accord, refus, accord partiel) ; un refus met fin à l'aménagement à la date notifiée.
6. Modification en cours d'année : même circuit (nouvelle version, l'ancienne est close).
Seule la **petite section** est éligible, **l'après-midi seulement** (contrôle serveur, R-ECO10).

### 8.4 Absences du mois et équipe éducative (RE-05, RE-06) [R]
- Compteur **par mois civil** des demi-journées **sans motif légitime ni excuse valable** (le motif « légitime » est apprécié par la directrice : liste de motifs du socle 12 avec la case « valable » décidée par une personne, jamais par l'IA).
- **Dialogue** avec la famille (R131-6) dès les absences répétées, tracé dans le dossier annuel (socle 12, 2251).
- **Seuil de 4** atteint : tâche **bloquante** pour la directrice « réunir l'équipe éducative » (socle 19 : rendez-vous ; composition : directrice, enseignant, famille, RASED, médecin ou psychologue si besoin) et « saisir l'IEN et le DASEN » (document pré-rempli, envoi fait par la directrice). **Aucun envoi automatique** (R-ECO12).
- Les demi-journées **aménagées** (8.3) et les absences pour **maladie justifiée** ne comptent pas.

### 8.5 Fin de journée : l'enfant non repris
Socle 14, attente précisée ici : à l'heure de sortie, les enfants **non repris** (ni remis, ni inscrits en garderie) sont listés ; la directrice appelle les personnes autorisées dans l'ordre du dossier (4) ; au-delà d'un délai (S-5002), procédure du règlement intérieur [H] ; chaque appel est tracé. Fait déclenchant d'un éventuel suivi (36) si la situation se répète, sur décision humaine.

---

## 9. La classe et la pédagogie

### 9.1 Structure et rythme [R]
- **Unité** école ; **classes** multi-niveaux (PS-MS, CE1-CE2, CM1-CM2) ; **cycle** déduit du niveau de **chaque enfant** (une classe CM2-6e n'existe pas, mais une classe GS-CP existe [H] : l'enfant de GS reste en cycle 1).
- **Rythme d'école** : jours ouvrés et demi-journées **par unité**, en table (remplace `jours_ouvres JSONB` pour les écoles et, à terme, pour tous : D-036) ; horaires d'entrée et de sortie par demi-journée ; mercredi matin travaillé ou non ; contrôle « 6 h par jour au plus, 24 h par semaine » (RE-12) ; décision de dérogation du DASEN enregistrée avec sa date de fin (3 ans au plus).
- **Cours unique** « classe entière » pour l'enseignant titulaire (D-022 : la même règle de visibilité) ; cours supplémentaires pour un décloisonnement, une intervention, une langue.
- **Temps partiel et compléments de service** : une classe a deux enseignants (titulaire à 75 % et complément le jeudi) : deux interventions datées sur le même cours (socle 6), chacun voit la classe [R].

### 9.2 Le cahier journal [R]
- Outil **professionnel** de l'enseignant, recommandé par l'inspection [H] : ce qui est prévu chaque jour, par domaine, et ce qui a été fait.
- **Privé** à l'enseignant et à ses co-intervenants de la classe ; **lisible** par la directrice et l'IEN **sur présentation** (l'enseignant ouvre l'accès, journalisé) [R] ; **jamais** visible des familles.
- Semaine **type** reconduite d'une semaine à l'autre ; reprise de la programmation de période (progressions du socle 20, 2408).
- Pour le remplaçant : l'enseignant peut publier « **la journée de demain** » pour le remplaçant désigné (lien avec le socle 15).

### 9.3 Le cahier de liaison [R]
- Le mot de l'enseignant ou de la directrice à la famille ; le « **vu** » d'un titulaire (signature simple, socle 18) ; les réponses de la famille ; un mot **papier** reste possible (le numérique ne remplace pas le cartable pour les familles sans compte, D-020) : impression du mot avec talon.
- Distinct de la messagerie (16) : c'est un **fil par enfant**, lisible par les deux foyers (sauf restriction judiciaire, socle 4).

### 9.4 La journée du remplaçant
**Fiche de classe du jour** générée pour le remplaçant affecté (socle 15) : photos de classe (si droit à l'image), consignes PAI **minimales** (34), **interdictions de remise** (4), autorisations de sortie seul, emploi du temps, cahier journal publié. Accès **limité à la date** du remplacement, journalisé, révoqué à minuit (R-ECO17).

---

## 10. Suivi des apprentissages, maternelle et LSU

### 10.1 Maternelle : carnet de suivi (RE-09) [R]
- **Observation** = un fait daté : enfant(s), attendu du programme (référentiel de cycle 1 au socle 22), commentaire court, **photo ou courte vidéo** facultative (objet du socle 7, stockage hors base, E5).
- **Photo d'un enfant** : interdite si **le droit à l'image n'est pas accordé** pour l'usage « carnet de suivi » (consentement par usage, socle 18) ; photo **de groupe** : l'enfant sans accord est **flouté ou retiré** avant publication à sa famille et **jamais** montré à une autre famille (R-ECO22).
- Prise de vue sur **un appareil de l'école** (mobile du 11) ; jamais dans la pellicule du téléphone : l'image va directement au stockage chiffré [R].
- **Publication à la famille** : par lot, quand l'enseignant le décide (S-5004 : publication immédiate ou par période) ; la famille voit **son** enfant et peut répondre (« ce qu'il fait à la maison » : facultatif [R]).
- **Le carnet suit l'enfant** dans le groupe ; au départ, export PDF remis à la famille (pas de transmission automatique à une autre école) [R].

### 10.2 Synthèse des acquis de fin de GS (RE-09)
Trame **nationale** (D-023 : pas de mise en forme libre) ; préparée par l'enseignant de GS, validée par l'équipe de cycle ; communiquée aux parents ; transmise à l'école élémentaire (dans le groupe : visible de l'enseignant de CP à la rentrée ; hors groupe : remise à la famille, ou export LSU si convention).

### 10.3 Élémentaire : bilans périodiques et de fin de cycle (RE-08)
- **Bilans périodiques** (nombre de périodes choisi par l'école [NV]) : pour chaque domaine, éléments du programme travaillés, positionnement (4 niveaux), appréciation ; projets, parcours ; communication à la famille et **visa en ligne** d'un titulaire (le second titulaire voit le visa ; tous les deux peuvent viser) ; signature de l'enseignant et de la directrice.
- **Bilans de fin de cycle 2 et 3** (CE2, CM2) : 8 composantes du socle commun, arrêtés en conseil de cycle ou des maîtres (socle 22, CMP-170).
- **Attestations** : APER, ASSN (22).
- **Aide à la rédaction** (§17) : à partir des observations et évaluations de la période, l'IA **propose** une appréciation que l'enseignant relit et modifie (D-032 Q-EV8) ; jamais de positionnement proposé (règlement IA, `21_EVALUATIONS.md` R-22, R-33).
- **Export LSU** : seulement avec convention éditeur (F-06) ; d'ici là, édition PDF et saisie manuelle dans l'application nationale par la directrice (double saisie assumée et signalée) [R].

---

## 11. Passage, maintien et raccourcissement (RE-10)

- **Proposition de passage** par le conseil des maîtres en fin d'année ; par défaut, **passage** (le maintien est exceptionnel).
- **Maintien envisagé** : l'enseignant le signale dans un **dossier de maintien** ; ScolIA vérifie : (a) un **accompagnement** a eu lieu (PPRE, APC, aides du RASED : lien vers les faits des socles 35 et 12) ; (b) **aucun maintien antérieur** dans le primaire, sinon « avis de l'IEN requis » ; (c) élève **handicapé** : avis de l'IEN requis ; (d) **dialogue avec la famille** tenu **au plus tard à la fin du 2e trimestre** (ou du 1er semestre) — ScolIA crée la tâche datée dès janvier et **bloque** la décision si le dialogue n'est pas tracé (R-ECO25).
- **Décision** du conseil des maîtres (procès-verbal, socle 23) ; **notification** à chaque titulaire (socle 8, preuve de notification du 13) avec la **voie de recours** et le **délai de 15 jours** calculé depuis la notification (D-027 D).
- **Recours** : enregistré par la directrice (date de dépôt, auteur) ; dossier transmis à la commission départementale d'appel (décisions motivées + éléments) ; décision de la commission enregistrée, **définitive** ; la scolarité N+1 suit (socle 3, 5).
- **Raccourcissement de cycle** : même procédure, même compteur.

---

## 12. Instances de l'école

| Instance | Composition (calculée) | Ce que ScolIA fait | Lot |
|---|---|---|---|
| **Conseil des maîtres** | enseignants de l'école, directrice présidente | réunions, ordre du jour, relevé de décisions (passage, répartition, projet d'école, APC) | EC8 |
| **Conseil de cycle** | enseignants du cycle (cycle 3 : avec les professeurs de 6e au conseil école-collège) | bilans de fin de cycle, progressions | EC8 |
| **Conseil d'école** (RE-11) | directrice, maire ou représentant, conseiller municipal, enseignants, un maître du RASED, **représentants des parents (= nombre de classes)**, DDEN ; IEN de droit | composition au jour près (nombre de classes au 1er jour), convocation **8 jours** avant, ordre du jour, PV publié aux familles, contrôle « une réunion par trimestre » et « dans le mois suivant les élections » | EC8 |
| **Équipe éducative** | directrice, enseignant, famille, RASED, psychologue, médecin, autres | rendez-vous (19), compte rendu **confidentiel** (lecture limitée aux participants professionnels et à la famille) | EC3, EC8 |
| **Équipe de suivi de la scolarisation** | socle 35 | — | — |
| **Conseil école-collège** | §15 | — | EC11 |

Les membres extérieurs (maire, conseiller municipal, DDEN, IEN) sont des **personnes sans compte** (D-020) convoquées par lien nominatif (D-027 G) ; ils **ne voient aucune donnée d'élève** (R-ECO28).

---

## 13. La commune, le périscolaire et les ATSEM

### 13.1 La commune, tiers à part entière [R]
- Une **commune partenaire** (code COG) est rattachée à l'unité école publique (et, dans le privé, aux communes qui versent un forfait ou organisent un transport [H]).
- **Échanges** (socle 9, moteur d'échanges) : **listes d'effectifs par classe et par niveau** (pour la cantine, la garderie, les ATSEM, le forfait communal du privé [NV : L442-5-1]), **liste des élèves par commune de résidence** (même finalité que la fiche ONDE 17), **absents du jour** pour la cantine municipale (seulement le nombre par classe, ou la liste nominative si une convention le prévoit : S-5007, défaut « nombre ») ; **jamais** de donnée de santé (le PAI alimentaire passe par la commune elle-même, qui en est responsable pour sa cantine [H]).
- Chaque échange est **déclaré** (finalité, champs, fréquence, destinataire, convention signée : pièce du 7), **tracé**, et **minimal** (R-ECO30). Pas d'accès direct d'un agent de mairie aux données de l'école, sauf le cas des ATSEM (13.2).

### 13.2 ATSEM et personnels municipaux [R]
- L'ATSEM est un agent **de la commune** (statut territorial [NV, décret 92-850]) mis à disposition de l'école ; dans le privé, souvent un salarié de l'OGEC (socle 6 : engagement, employeur).
- **Accès** : seulement **sa** ou **ses** classes, seulement **sur ses horaires** de service (ou toute la journée scolaire si plus simple : S-5008), seulement : photo, prénom et nom, **remises et interdictions**, **consignes PAI minimales**, **régime de cantine du jour**, présence du jour ; **ni** bilans, **ni** cahier de liaison, **ni** coordonnées, **ni** messagerie avec les familles (R-ECO31).
- Personnel du **périscolaire municipal** (garderie, cantine) : même principe, par activité (socle 43).

### 13.3 Périscolaire et cantine
- **Privé** : l'OGEC gère tout (socles 40, 43, 24).
- **Public** : la commune gère ; ScolIA fournit les effectifs et les absences (13.1) et **reçoit** éventuellement la liste des inscrits à la garderie pour la remise de 16 h 30 (« Léo va à la garderie ce soir ») : liste nominative minimale (nom, classe, jours) importée ou saisie [R].

---

## 14. ONDE et les échanges officiels

- **ScolIA n'écrit pas dans ONDE** (RE-01 à RE-03) : aucune interface d'écriture publique ; la spécification « ONDE V2 interface école » n'est accessible qu'aux éditeurs inscrits (D-8 de `09_INTEROP.md`, [NV]).
- **Import d'une extraction ONDE** (CSV de la fiche 16) : la directrice téléverse le fichier ; ScolIA **reconnaît les en-têtes** (pas les positions), rapproche par **INE**, puis nom + prénom + date de naissance ; produit un **rapport** : élèves présents dans ONDE et absents de ScolIA, l'inverse, écarts d'identité, de classe, de niveau, de responsables ; **aucune** mise à jour automatique : chaque écart est accepté ou rejeté par la directrice, en proposition (socle 2, lot E5 : propositions de source `import`) (R-ECO20).
- **Ce qu'ONDE apporte** : INE (provenance `import_onde`), classe et niveau officiels, statut d'admission définitive, responsables légaux. **Ce qu'on n'importe pas** : la CSP si elle n'est pas nécessaire (minimisation, §20), les données que ScolIA ne doit pas détenir.
- **Contrôle de cohérence de rentrée** : avant le constat d'effectifs, ScolIA liste les écarts avec la dernière extraction (tableau de bord de rentrée du socle 5).
- **Export** : une liste au **format ScolIA** (correctif ED-04) pour aider la saisie manuelle dans ONDE ; **jamais** présentée comme un fichier ONDE.
- **LSU 1D** : comme pour le 2nd degré (socle 22, X7 du 9), seulement après convention éditeur (D-032, F-06).
- **ÉduConnect 1D** : les comptes des parents d'école publique sont alimentés par ONDE [V-partiel] ; ScolIA s'y raccorde comme fournisseur de service (X8 du 9, D-029) ; dans le privé, compte ScolIA.

---

## 15. Liaison école-collège

- **Dans le groupe** : à la montée en 6e, le collège reçoit : identité, responsables (déjà là), **LSU** (bilans périodiques de CM2 et bilan de fin de cycle 3 — en fait le cycle 3 se termine en 6e : le bilan de fin de cycle 3 est arrêté en fin de 6e, et le CM2 transmet ses bilans périodiques [V-partiel, à confirmer : Q-ECO15]), **PAP, PPS, PAI** (par les socles 35 et 34, avec leurs règles), **aménagements** utiles ; **pas** le carnet de maternelle, **pas** les observations de l'école, **pas** le cahier de liaison, **pas** le cahier journal (R-ECO33).
- **Hors groupe** : le collège reçoit le LSU par l'application nationale ; ScolIA ne transmet rien de plus.
- **Conseil école-collège** (RE-16) : instance avec les professeurs de 6e du ou des collèges de secteur (public) ; personnes sans compte ou agents d'un autre établissement ScolIA **sans visibilité croisée** (D-013) : on partage un ordre du jour et des actions, **pas** de listes d'élèves [R].
- **Professeurs du collège intervenant à l'école** (cycle 3, échanges de service) : intervention datée sur un cours de l'école (socle 6), portée D-022 limitée à ce cours.

---

## 16. Cas limites (catalogue)

| # | Cas | Réponse attendue |
|---|---|---|
| CEC-01 | Enfant de **TPS** (2 ans) inscrit en janvier | niveau TPS, cycle 1 ; pas d'obligation d'instruction avant 3 ans : les absences ne comptent pas au seuil (R-ECO11) |
| CEC-02 | Enfant qui a **3 ans en cours d'année** | l'obligation commence à la date de ses 3 ans (L131-1, [NV] sur la date exacte : année civile ?) ; compteur ouvert à cette date (Q-ECO16) |
| CEC-03 | Aménagement de PS demandé **pour le matin** | refusé par le serveur (RE-07) avec explication |
| CEC-04 | Aménagement demandé pour un enfant de **MS** | refusé (petite section seulement) ; un aménagement pour un enfant **malade** relève du PAI ou d'une absence justifiée |
| CEC-05 | IEN qui **ne répond pas** | accord tacite au J+15, affiché ; la directrice peut corriger si une réponse arrive tard |
| CEC-06 | Deux titulaires **en désaccord** sur l'aménagement | demande non recevable tant que le désaccord n'est pas levé (socle 18, désaccord connu) ; la directrice en est informée |
| CEC-07 | Classe **CE1-CE2** : bilans de deux cycles différents | non : CE1 et CE2 sont en cycle 2 ; une classe **CM2-6e** n'existe pas ; une classe **GS-CP** relie cycle 1 et 2 : carnet pour les GS, LSU pour les CP |
| CEC-08 | Enseignante à **temps partiel** + complément le jeudi | deux intervenants sur le cours ; l'appel du jeudi est fait par le complément ; le cahier journal est partagé |
| CEC-09 | **Remplaçant** pour une journée | fiche de classe du jour, accès révoqué à minuit (R-ECO17) |
| CEC-10 | **Grève** sans service minimum d'accueil communal | fermeture ciblée de la classe (socle 5) ; pas d'appel attendu ; information des familles |
| CEC-11 | **Service minimum d'accueil** organisé par la commune | liste des enfants accueillis transmise par la commune ; aucun appel « scolaire » ce jour [NV] |
| CEC-12 | Enfant **non repris** à 16 h 30, parents injoignables | §8.5 ; chaque tentative tracée ; jamais de remise à une personne non autorisée |
| CEC-13 | Grand-mère **non inscrite** se présente | refus de remise sans autorisation écrite d'un titulaire ; la famille peut l'ajouter **ponctuellement** en ligne (14), avec pièce d'identité contrôlée à la grille |
| CEC-14 | Parent **privé d'autorité parentale** ou interdiction judiciaire | remise refusée (4, 14) ; alerte discrète à l'adulte de la grille |
| CEC-15 | Résidence **alternée** : qui reprend le vendredi soir ? | foyer du jour (4) ; les deux foyers voient le planning |
| CEC-16 | Élève de **CM2** qui rentre seul | autorisation écrite des titulaires (18) ; jamais pour un enfant de maternelle (R-ECO19) |
| CEC-17 | Droit à l'image **refusé** et photo de classe | l'enfant n'apparaît pas sur les photos publiées ; la photo officielle de classe (photographe) relève d'un autre consentement (S-5005) |
| CEC-18 | Un titulaire accorde le droit à l'image, l'autre le **refuse** | désaccord connu ⇒ refus (socle 18) |
| CEC-19 | **ATSEM** qui veut voir l'adresse d'un enfant | refusé (R-ECO31) ; la directrice est l'interlocutrice |
| CEC-20 | ATSEM de la **commune** qui change d'école en cours d'année | fin d'engagement datée ; accès coupé le jour même |
| CEC-21 | **Quatrième** demi-journée non justifiée le 30 du mois | seuil atteint : tâche créée ; le compteur repart à zéro le 1er du mois suivant, la tâche reste (R-ECO12) |
| CEC-22 | Absences à **cheval sur deux mois** (3 + 3) | aucun seuil franchi par mois civil ; mais « absences répétées » ⇒ dialogue proposé (RE-05) |
| CEC-23 | **Maintien** proposé en mai sans dialogue tracé | décision bloquée ; la directrice voit pourquoi (R-ECO25) |
| CEC-24 | Deuxième maintien pour un enfant déjà maintenu en GS | avis de l'IEN requis (pièce) avant la décision |
| CEC-25 | Recours de la famille **le 16e jour** | enregistré comme tardif ; la directrice décide de le transmettre ou non (ScolIA ne rejette pas) |
| CEC-26 | Enfant **handicapé**, PPS : maintien | avis de l'IEN requis ; ESS (35) consultée |
| CEC-27 | **Changement d'école en cours d'année** dans le groupe (deux écoles du même groupe) | changement d'unité daté ; carnet et bilans suivent ; compteurs d'assiduité du mois conservés |
| CEC-28 | Départ vers une **école hors groupe** | certificat de radiation de l'unité ; carnet de suivi remis à la famille en PDF ; LSU par l'application nationale |
| CEC-29 | Enfant **instruit en famille** qui revient en cours d'année | admission ; aucun historique LSU ; positionnement par l'enseignant |
| CEC-30 | Enfant **allophone** arrivant (UPE2A 1D) | dispositif du socle 35 ; certificat sans mention du dispositif (D-027 B) |
| CEC-31 | Classe **ULIS école** | unité école, dispositif (35) ; inclusion dans les classes ordinaires : cours avec inclusions datées (socle 5, R-S5) |
| CEC-32 | École d'un groupe **sous contrat simple**, collège sous **contrat d'association** | contrat porté par l'unité (§6.2) |
| CEC-33 | Une classe de l'école passe **sous contrat d'association** au 1er septembre | contrat de l'unité daté ; documents et exports suivent la date |
| CEC-34 | **Deux UAI** sur un même site (maternelle et élémentaire distinctes) | deux unités, un site (§6.2) |
| CEC-35 | Une seule directrice pour deux écoles (maternelle et élémentaire, public) | deux unités, même agent en direction des deux |
| CEC-36 | Fermeture pour **neige** de l'école seulement | fermeture ciblée sur l'unité (socle 5) ; le collège ouvert |
| CEC-37 | **Semaine de 4 jours** à l'école, 4,5 au collège, même famille | rythmes par unité ; garderie du mercredi pour les petits (43) |
| CEC-38 | Sortie commune école + collège (chorale) | sortie liée, chaque direction autorise ses participants (Q-V35) |
| CEC-39 | Fratrie école + collège + lycée : réduction | foyer payeur sur tout l'établissement (24) |
| CEC-40 | Parent élu au conseil d'école **et** salarié de l'OGEC | deux casquettes (socle 6) ; aucune donnée d'élève par la casquette d'élu |
| CEC-41 | Nombre de classes qui **change** après l'élection | composition du conseil fixée par le scrutin ; pas de réélection [NV] |
| CEC-42 | **DDEN** convoqué sans compte | lien nominatif (D-027 G) ; aucune donnée élève |
| CEC-43 | Extraction ONDE avec un élève **inconnu** de ScolIA | proposition de création, jamais automatique (R-ECO20) |
| CEC-44 | Extraction ONDE avec un **INE différent** pour un même enfant | écart signalé ; la directrice tranche ; INE ONDE = confirmé (02 l. 308) |
| CEC-45 | Extraction ONDE **d'une autre école** téléversée par erreur | refus : UAI du fichier ≠ UAI de l'unité (R-ECO21) |
| CEC-46 | Fichier ONDE contenant une **formule** dans un nom | stocké et réexporté neutralisé (ED-03) |
| CEC-47 | Photo de maternelle prise sur le **téléphone personnel** | ScolIA ne la reçoit pas : l'appli de l'école capture sans passer par la pellicule ; dépôt depuis la galerie désactivé par défaut (S-5006) |
| CEC-48 | Famille qui veut **effacer** une photo du carnet | retrait de la publication à la famille (droit d'opposition) ; l'observation textuelle reste [R] |
| CEC-49 | Enfant **placé** (ASE) : qui reçoit le carnet ? | les titulaires selon la décision (4) ; le service gardien selon ses droits ; jamais une personne sans droit |
| CEC-50 | Enseignant qui demande l'accès aux **notes du frère** au collège | refusé : portée de l'unité et des cours (R-ECO3) |
| CEC-51 | Direction du **collège** qui cherche un enfant de l'école | annuaire (désignation) visible selon Q-S14 du 5 ; rien au-delà (R-ECO3) |
| CEC-52 | APC : la famille **refuse** | l'enfant n'est pas inscrit ; aucune absence comptée |
| CEC-53 | Cantine municipale : la commune demande la **liste des allergies** | refusé par ScolIA (R-ECO30) ; la commune la recueille elle-même |
| CEC-54 | Commune qui demande **toute la base** pour son portail famille | refusé : échange déclaré, champs minimaux, convention |
| CEC-55 | IEN qui demande un **accès direct** à ScolIA | non par défaut ; documents transmis par la directrice ; accès lecteur nominatif possible sur décision (Q-ECO8) |
| CEC-56 | École privée **hors contrat** dans le groupe | pas d'ONDE ni de LSU obligatoires [NV] ; contrôles désactivés par l'unité |

---

## 17. Intelligence artificielle (contrat IA, `IA_COEUR.md` §5)

**Ce que l'IA fait** [R] :
- **prépare** les appréciations des bilans périodiques à partir des observations et positionnements de l'enseignant (proposées une à une, relues, D-032) ;
- **rédige** le mot du cahier de liaison, la convocation du conseil d'école, l'ordre du jour à partir des sujets notés, le PV à partir des notes de séance ;
- **résume** pour la directrice : « ce matin », absences du mois, échéances (aménagements, dialogues de maintien) ;
- **aide à l'import ONDE** : explique chaque écart en langage clair, propose la correspondance des colonnes ;
- **traduit** un mot aux familles allophones (relu).

**Ce que l'IA ne fait pas** : aucun **positionnement** ni décision de passage ou de maintien (règlement IA, annexe III point 3) ; **aucune** analyse de photo d'enfant (pas de reconnaissance faciale, pas d'étiquetage automatique d'un enfant sur une photo : R-ECO23) ; aucune donnée de santé ; aucun envoi ; aucune décision sur le caractère « légitime » d'un motif d'absence ; aucun « profilage » de l'enfant.

**Capacités déclarées** : `ecole.resume_matin` (lecture, N0), `ecole.preparer_appreciation` (proposition, N1, relue), `ecole.rediger_convocation_conseil` (proposition, N1), `ecole.expliquer_ecarts_onde` (lecture, N0), `ecole.rediger_mot_liaison` (proposition, N2 avant envoi).

---

## 18. Actions métier (ECO-100 à ECO-279)

Colonnes : acteur (DIR direction de l'unité, COO coordination, ENS enseignant de la classe, REM remplaçant, ATS ATSEM, ACC accueil, SEC secrétariat, FAM titulaire, EXT personne sans compte, SYS système) et droit (catalogue §20.1) ; règles ; événement ; lot. Niveaux de confirmation N1–N3 (`IA_COEUR.md` §6.2).

### 18.1 Groupe scolaire et unités
| N° | Action | Acteur · droit | Règles | Événement | Lot |
|---|---|---|---|---|---|
| ECO-100 | Créer une unité scolaire (nature, degré, UAI, secteur, contrat, sites) | COO · `structure.unites` | UAI au format ; unicité de l'UAI dans l'établissement | `structure.unite_creee` | EC1 |
| ECO-101 | Modifier le contrat ou le secteur d'une unité à une date | COO | daté, journal ; documents futurs seulement | `structure.unite_modifiee` | EC1 |
| ECO-102 | Fermer une unité (fin datée) | COO | refus s'il reste des classes ouvertes | `structure.unite_fermee` | EC1 |
| ECO-103 | Désigner la direction d'une unité (agent, dates) | COO · `structure.unites` | agent du socle 6 ; plusieurs possibles | `personnels.fonction_attribuee` | EC1 |
| ECO-104 | Rattacher une classe à une unité | DIR, COO | une seule unité par classe ; reprise : proposition depuis le niveau | `structure.classe_modifiee` | EC1 |
| ECO-105 | Rattacher les classes existantes en masse (reprise) | COO | proposition par cycle du niveau ; **aucune** inférence par libellé (ED-09) | — | EC1 |
| ECO-106 | Déclarer un groupe transversal à plusieurs unités | DIR, COO | unités déclarées | `structure.groupe_cree` | EC1 |
| ECO-107 | Choisir l'entité émettrice et l'entité de facturation de l'unité | COO, comptable | socles 7, 24 | — | EC1 |
| ECO-108 | Voir l'établissement par unité (effectifs, classes, directions) | DIR (son unité), COO (tout) | portée `mon_unite` | — | EC1 |
| ECO-109 | Contrôler la cohérence des unités (classe sans unité, unité sans direction, UAI manquant en public) | SYS → COO | tableau de bord de rentrée (5) | — | EC1 |

### 18.2 Structure, rythme et classes de l'école
| N° | Action | Acteur · droit | Règles | Événement | Lot |
|---|---|---|---|---|---|
| ECO-110 | Définir le rythme de l'unité (jours, demi-journées, horaires d'entrée et de sortie) | DIR · `structure.calendrier` | 6 h/jour, 24 h/semaine, 5 matinées si 4 jours (RE-12) | `structure.rythme_modifie` | EC2 |
| ECO-111 | Enregistrer la décision de dérogation du DASEN (dates, 3 ans max) | DIR | pièce (7) | — | EC2 |
| ECO-112 | Créer une classe multi-niveaux | DIR | 1..n niveaux ; cycle par enfant | `structure.classe_creee` | EC2 |
| ECO-113 | Affecter l'enseignant titulaire et les compléments de service | DIR | interventions datées (6) | `personnels.intervention` | EC2 |
| ECO-114 | Créer le cours « classe entière » automatiquement à la création de la classe | SYS | un par classe | `structure.cours_cree` | EC2 |
| ECO-115 | Organiser un décloisonnement (groupe de besoin, langue, échange de service) | ENS, DIR | appel reste à la classe de référence (R-ECO8) | — | EC2 |
| ECO-116 | Déclarer une intervention extérieure régulière (EPS, musique, langue) | DIR | intervenant agréé [NV] ; présence de l'enseignant | — | EC2 |
| ECO-117 | Répartir les élèves dans les classes pour N+1 (conseil des maîtres) | DIR | outil de constitution du 5 ; multi-niveaux | `structure.repartition_proposee` | EC2 |
| ECO-118 | Publier les listes de classe de la rentrée aux familles (date choisie) | DIR | date programmée (S-5009) | `annonces.publiee` | EC2 |
| ECO-119 | Gérer l'accueil échelonné des PS (groupes, dates, horaires) | DIR, ENS | créneaux de rentrée (19) | — | EC2 |
| ECO-120 | Planifier les APC (groupes, horaires, période, enseignant) | ENS, DIR | 36 h/an par enseignant, compteur (RE-12) | `ecole.apc_planifiee` | EC2 |
| ECO-121 | Proposer les APC à une famille et recueillir son accord | ENS → FAM | formulaire (18) ; refus = pas d'inscription | `formulaires.reponse` | EC2 |
| ECO-122 | Suivre les heures d'APC faites par enseignant | ENS, DIR | compteur | — | EC2 |

### 18.3 Appel, registre, assiduité
| N° | Action | Acteur · droit | Règles | Événement | Lot |
|---|---|---|---|---|---|
| ECO-130 | Faire l'appel de la demi-journée | ENS, REM · `assiduite.appel` | une fois par demi-journée et classe | `assiduite.appel_fait` | EC3 |
| ECO-131 | Voir les absences déjà signalées et les aménagements dans l'appel | ENS | — | — | EC3 |
| ECO-132 | Rectifier : enfant arrivé en retard (heure, accompagnant) | ENS, ACC | S-5001 | `assiduite.retard` | EC3 |
| ECO-133 | Fermer l'appel du matin à l'heure réglée et lister les absents non signalés | SYS | S-5000 | `assiduite.non_signale` | EC3 |
| ECO-134 | Prévenir la famille d'une absence non signalée | SYS → FAM | socle 8 (Q-N5) ; foyer du jour | `notifications.avis` | EC3 |
| ECO-135 | Saisir l'absence signalée par téléphone (accueil) | ACC · `assiduite.signaler` | transmise à l'enseignant | `assiduite.signalee` | EC3 |
| ECO-136 | Signaler une absence (famille, en ligne) | FAM | motif ; pièce facultative | `assiduite.signalee` | EC3 |
| ECO-137 | Qualifier le motif (légitime ou non) | DIR · `assiduite.justifier` | décision humaine | `assiduite.justifiee` | EC3 |
| ECO-138 | Arrêter le registre du mois par classe | ENS puis DIR | figé ; avenant ensuite | `assiduite.mois_arrete` | EC3 |
| ECO-139 | Éditer le registre d'appel mensuel (PDF signé) | DIR | trame (7) | `documents.genere` | EC3 |
| ECO-140 | Corriger un mois arrêté par avenant | DIR | motif obligatoire | `assiduite.avenant` | EC3 |
| ECO-141 | Voir le compteur mensuel des demi-journées non justifiées | DIR | par mois civil | — | EC3 |
| ECO-142 | Alerte au seuil de 4 demi-journées | SYS → DIR | tâche bloquante (29) | `assiduite.seuil_atteint` | EC3 |
| ECO-143 | Tracer le dialogue avec la famille (R131-6) | DIR, ENS | dossier annuel (12) | `assiduite.dialogue` | EC3 |
| ECO-144 | Organiser l'équipe éducative (rendez-vous, participants) | DIR | socle 19 | `agenda.rdv_cree` | EC3 |
| ECO-145 | Rédiger le compte rendu d'équipe éducative | DIR | confidentiel | — | EC3 |
| ECO-146 | Préparer la saisine de l'IEN et du DASEN (document pré-rempli) | DIR | aucun envoi automatique (R-ECO12) | `documents.genere` | EC3 |
| ECO-147 | Enregistrer la date de transmission à l'IEN / DASEN | DIR | — | `assiduite.signalement_transmis` | EC3 |
| ECO-148 | Recevoir une demande d'aménagement de PS (famille) | FAM | PS et après-midi seulement (R-ECO10) | `ecole.amenagement_demande` | EC3 |
| ECO-149 | Donner l'avis de la direction sur l'aménagement | DIR | échéance 2 jours ouvrés | `ecole.amenagement_avis` | EC3 |
| ECO-150 | Enregistrer la transmission à l'IEN | DIR | J+15 calculé | `ecole.amenagement_transmis` | EC3 |
| ECO-151 | Appliquer l'aménagement à titre provisoire | SYS | si avis favorable | `ecole.amenagement_applique` | EC3 |
| ECO-152 | Constater l'accord tacite au J+15 | SYS | — | `ecole.amenagement_accorde` | EC3 |
| ECO-153 | Enregistrer la décision expresse de l'IEN | DIR | accord, refus, partiel | `ecole.amenagement_decide` | EC3 |
| ECO-154 | Modifier un aménagement en cours d'année | FAM → DIR | même circuit, nouvelle version | `ecole.amenagement_demande` | EC3 |
| ECO-155 | Faire l'appel des APC | ENS | activité appelée (12) | `assiduite.appel_fait` | EC3 |

### 18.4 Arrivées, sorties, remise de l'enfant
| N° | Action | Acteur · droit | Règles | Événement | Lot |
|---|---|---|---|---|---|
| ECO-160 | Déclarer les personnes autorisées à reprendre l'enfant (permanentes) | FAM | socle 4 ; pièce d'identité à la première venue | `personnes.autorisation_remise` | EC4 |
| ECO-161 | Déclarer une personne autorisée **ponctuelle** (« mamie ce soir ») | FAM | un titulaire suffit, sauf désaccord connu | `accueil.remise_ponctuelle` | EC4 |
| ECO-162 | Voir « qui vient chercher qui ce soir » par classe | ENS, ATS, ACC | foyer du jour ; garderie ; APC | — | EC4 |
| ECO-163 | Enregistrer la remise de l'enfant (personne, heure) | ENS, ATS, ACC · `accueil.remettre` | personne autorisée ; interdiction bloquante | `accueil.remis` | EC4 |
| ECO-164 | Refuser une remise et alerter la direction | ENS, ATS | interdiction, personne inconnue | `accueil.remise_refusee` | EC4 |
| ECO-165 | Lister les enfants non repris à l'heure | SYS → DIR | §8.5 | `accueil.non_repris` | EC4 |
| ECO-166 | Tracer les appels aux personnes autorisées | DIR | ordre du dossier | — | EC4 |
| ECO-167 | Confier l'enfant à la garderie (inscrite ou exceptionnelle) | DIR, ENS | socle 43 ; facturation éventuelle (24) | `periscolaire.confie` | EC4 |
| ECO-168 | Autoriser un élève d'élémentaire à sortir seul | FAM | formulaire signé ; jamais en maternelle (R-ECO19) | `formulaires.reponse` | EC4 |
| ECO-169 | Voir et imprimer la liste des remises du jour (panne réseau) | ENS, ACC | copie locale chiffrée du jour (11) | — | EC4 |
| ECO-170 | Enregistrer un retard de la famille au soir | ACC, DIR | répétition ⇒ dialogue | `accueil.retard_famille` | EC4 |

### 18.5 Vie de la classe, cahier journal, liaison
| N° | Action | Acteur · droit | Règles | Événement | Lot |
|---|---|---|---|---|---|
| ECO-180 | Tenir le cahier journal (jour, domaines, prévu / fait) | ENS · `ecole.cahier_journal` | privé à la classe | — | EC6 |
| ECO-181 | Reconduire une semaine type | ENS | — | — | EC6 |
| ECO-182 | Publier « la journée de demain » pour le remplaçant | ENS | remplaçant désigné (15) | — | EC6 |
| ECO-183 | Ouvrir le cahier journal à la directrice ou à l'IEN (temporaire) | ENS | journalisé ; durée limitée | `ecole.cahier_partage` | EC6 |
| ECO-184 | Écrire un mot dans le cahier de liaison | ENS, DIR | fil par enfant ; deux foyers | `liaison.mot` | EC6 |
| ECO-185 | Écrire un mot à toute la classe | ENS | un fil par enfant (pas de « répondre à tous ») | `liaison.mot` | EC6 |
| ECO-186 | Viser (« vu ») un mot | FAM | un titulaire suffit ; l'enseignant voit qui a visé | `liaison.vu` | EC6 |
| ECO-187 | Répondre à un mot | FAM | — | `liaison.reponse` | EC6 |
| ECO-188 | Relancer les mots non vus | SYS → ENS | à J+2 (S-5010) | — | EC6 |
| ECO-189 | Imprimer un mot avec talon (famille sans compte) | ENS | D-020 | — | EC6 |
| ECO-190 | Publier un « à préparer » léger (poésie, lecture) | ENS | socle 20 | `cdt.travail_publie` | EC6 |
| ECO-191 | Publier une annonce à la classe ou à l'école | ENS, DIR | 17 | `annonces.publiee` | EC6 |
| ECO-192 | Fiche de classe du jour pour le remplaçant | SYS → REM | accès révoqué à minuit (R-ECO17) | `ecole.fiche_jour` | EC6 |

### 18.6 Apprentissages : maternelle, LSU, attestations
| N° | Action | Acteur · droit | Règles | Événement | Lot |
|---|---|---|---|---|---|
| ECO-200 | Noter une observation (enfant(s), attendu, commentaire) | ENS · `ecole.observer` | référentiel de cycle 1 (22) | `ecole.observation` | EC7 |
| ECO-201 | Joindre une photo ou une vidéo courte | ENS | droit à l'image par usage (R-ECO22) ; appareil de l'école | `documents.depose` | EC7 |
| ECO-202 | Retirer un enfant d'une photo de groupe (flou ou recadrage) | ENS | avant publication | — | EC7 |
| ECO-203 | Publier le carnet de la période aux familles | ENS | S-5004 | `ecole.carnet_publie` | EC7 |
| ECO-204 | Voir le carnet de son enfant, réagir | FAM | son enfant seulement | — | EC7 |
| ECO-205 | Exporter le carnet en PDF (départ, fin de cycle) | DIR, FAM | — | `documents.genere` | EC7 |
| ECO-206 | Préparer la synthèse de fin de GS (trame nationale) | ENS | RE-09 | — | EC7 |
| ECO-207 | Valider la synthèse en équipe de cycle | DIR | — | `ecole.synthese_gs_validee` | EC7 |
| ECO-208 | Communiquer la synthèse aux familles | DIR | — | `ecole.synthese_gs_communiquee` | EC7 |
| ECO-209 | Transmettre la synthèse à l'enseignant de CP (groupe) | SYS | à la rentrée, CP du groupe | — | EC7 |
| ECO-210 | Positionner un élève par domaine (4 niveaux) | ENS · `competences.positionner` | socle 22 | `competences.positionne` | EC7 |
| ECO-211 | Rédiger l'appréciation du bilan périodique | ENS | proposition IA relue (§17) | — | EC7 |
| ECO-212 | Arrêter et signer les bilans périodiques de la classe | ENS, DIR | — | `ecole.bilans_arretes` | EC7 |
| ECO-213 | Publier les bilans aux familles | DIR | date programmée | `ecole.bilans_publies` | EC7 |
| ECO-214 | Viser un bilan (titulaire) | FAM | RE-08 ; les deux peuvent viser | `ecole.bilan_vise` | EC7 |
| ECO-215 | Relancer les visas manquants | SYS → ENS | — | — | EC7 |
| ECO-216 | Arrêter les bilans de fin de cycle (CE2, CM2) en conseil | DIR | socle 22 (CMP-170) | `competences.bilans_arretes` | EC7 |
| ECO-217 | Délivrer l'attestation APER ou ASSN | ENS, DIR | 22 | `competences.attestation` | EC7 |
| ECO-218 | Éditer les bilans (trame 1er degré, 3 ou 4 au choix) | DIR | D-023 | `documents.genere` | EC7 |
| ECO-219 | Préparer la saisie LSU nationale (liste de contrôle) | DIR | sans convention : double saisie signalée | — | EC7 |

### 18.7 Passage, conseils, instances
| N° | Action | Acteur · droit | Règles | Événement | Lot |
|---|---|---|---|---|---|
| ECO-220 | Planifier les conseils des maîtres et de cycle | DIR | 19 | `agenda.reunion` | EC8 |
| ECO-221 | Tenir le relevé de décisions | DIR | — | — | EC8 |
| ECO-222 | Ouvrir un dossier de maintien | ENS | RE-10 | `ecole.maintien_ouvert` | EC8 |
| ECO-223 | Tracer le dialogue de maintien avec la famille (avant fin T2) | DIR, ENS | bloquant (R-ECO25) | `ecole.maintien_dialogue` | EC8 |
| ECO-224 | Joindre l'avis de l'IEN (2e maintien, handicap) | DIR | pièce | — | EC8 |
| ECO-225 | Décider en conseil des maîtres (passage, maintien, raccourcissement) | DIR | contrôles ; un seul dans le primaire | `ecole.passage_decide` | EC8 |
| ECO-226 | Notifier la décision à chaque titulaire (voie et délai de recours) | DIR | preuve de notification (13) | `ecole.passage_notifie` | EC8 |
| ECO-227 | Enregistrer un recours | DIR | 15 jours ; tardif signalé | `ecole.recours_depose` | EC8 |
| ECO-228 | Transmettre le dossier à la commission départementale | DIR | PDF des pièces | — | EC8 |
| ECO-229 | Enregistrer la décision de la commission | DIR | définitive ; met à jour N+1 (3, 5) | `ecole.recours_decide` | EC8 |
| ECO-230 | Calculer la composition du conseil d'école | SYS | nombre de classes ; membres de droit | — | EC8 |
| ECO-231 | Convoquer le conseil d'école (8 jours avant) | DIR | personnes sans compte par lien | `agenda.convocation` | EC8 |
| ECO-232 | Publier l'ordre du jour et le PV aux familles | DIR | 17 | `annonces.publiee` | EC8 |
| ECO-233 | Alerte « pas de conseil ce trimestre » / « pas de conseil un mois après l'élection » | SYS → DIR | RE-11 | — | EC8 |
| ECO-234 | Organiser l'élection des représentants de parents | DIR | socle 46 | `instances.scrutin` | EC8 |
| ECO-235 | Regrouper des conseils d'école (deux écoles) | DIR | D411-3 | — | EC8 |

### 18.8 Personnels de l'école
| N° | Action | Acteur · droit | Règles | Événement | Lot |
|---|---|---|---|---|---|
| ECO-240 | Déclarer un ATSEM (commune ou OGEC), ses classes et ses horaires | DIR | engagement (6) | `personnels.engagement` | EC9 |
| ECO-241 | Vue ATSEM de la classe (remises, PAI minimal, cantine du jour) | ATS | R-ECO31 | — | EC9 |
| ECO-242 | Déclarer un intervenant extérieur agréé | DIR | dates, classe | — | EC9 |
| ECO-243 | Affecter un remplaçant à une classe pour une période | DIR | socle 15 | `personnels.remplacement` | EC9 |
| ECO-244 | Déclarer le RASED et ses élèves suivis | DIR | socle 36 ; liste restreinte | — | EC9 |
| ECO-245 | Déclarer la décharge de direction | DIR, COO | jour de décharge ; remplaçant | — | EC9 |

### 18.9 Commune, périscolaire, finances
| N° | Action | Acteur · droit | Règles | Événement | Lot |
|---|---|---|---|---|---|
| ECO-250 | Déclarer une commune partenaire et sa convention | DIR, COO | pièce (7) | — | EC10 |
| ECO-251 | Déclarer un échange avec la commune (finalité, champs, fréquence) | DIR | minimal (R-ECO30) | `interop.echange_declare` | EC10 |
| ECO-252 | Envoyer les effectifs par classe à la commune | DIR | tracé | `interop.envoi` | EC10 |
| ECO-253 | Envoyer les absents du jour à la cantine municipale (nombre ou liste) | SYS → commune | S-5007 | `interop.envoi` | EC10 |
| ECO-254 | Importer la liste des inscrits à la garderie municipale | DIR | minimal | `interop.reception` | EC10 |
| ECO-255 | Éditer la liste des élèves par commune de résidence | DIR | forfait communal (privé) [NV] | `documents.genere` | EC10 |
| ECO-256 | Éditer l'attestation de frais de garde (moins de 6 ans) séparant garde et repas | SYS | 24 | `documents.genere` | EC10 |
| ECO-257 | Appliquer la fratrie sur tout le groupe | SYS | 24 | — | EC10 |

### 18.10 ONDE, inscriptions, liaison
| N° | Action | Acteur · droit | Règles | Événement | Lot |
|---|---|---|---|---|---|
| ECO-260 | Téléverser une extraction ONDE | DIR · `interop.onde` | UAI contrôlé (R-ECO21) | `interop.reception` | EC12 |
| ECO-261 | Voir le rapport de rapprochement | DIR | — | — | EC12 |
| ECO-262 | Accepter ou rejeter chaque écart | DIR | propositions (2, E5) | `eleve.proposition_decidee` | EC12 |
| ECO-263 | Contrôle de cohérence avant le constat d'effectifs | SYS → DIR | tableau de bord (5) | — | EC12 |
| ECO-264 | Exporter la liste « format ScolIA » pour la saisie dans ONDE | DIR | ED-04 | `export.liste` | EC0 |
| ECO-265 | Admettre un enfant après inscription en mairie | DIR | certificat de la commune (pièce) | `inscriptions.admis` | EC11 |
| ECO-266 | Délivrer un certificat de radiation de l'unité | DIR | 3, 7 | `documents.genere` | EC11 |
| ECO-267 | Préparer la montée CM2 → 6e dans le groupe | DIR (école et collège) | réinscription qui change d'unité | `inscriptions.reinscription` | EC11 |
| ECO-268 | Transmettre au collège du groupe ce que prévoit la liaison | SYS | R-ECO33 | `ecole.liaison_transmise` | EC11 |
| ECO-269 | Tenir le conseil école-collège | DIR | personnes sans compte ; aucune liste d'élèves | — | EC11 |

**Correctifs et reprise** : ECO-270 (EC0) export neutralisé (ED-01 à ED-05) ; ECO-271 (EC0) échappeur CSV partagé pour tous les exports (ED-03) ; ECO-272 (EC13) reprise des classes d'école Next.js et rattachement aux unités ; ECO-273 (EC13) reprise des personnes autorisées ; ECO-274–ECO-279 : ajout personas (§26).

---

## 19. Règles transverses (R-ECO1 à R-ECO40) [R]

| # | Règle |
|---|---|
| R-ECO1 | Un **groupe scolaire est un seul établissement ScolIA** découpé en unités ; plusieurs établissements seulement par décision (Q-ECO2). |
| R-ECO2 | Une classe appartient à **une** unité, choisie explicitement ; **aucune** inférence par libellé. |
| R-ECO3 | La portée « mon unité » est **fermée par défaut** : sans unité ni cours, aucune donnée d'élève (hors annuaire, Q-S14). |
| R-ECO4 | Une sortie qui réunit plusieurs unités est autorisée **par chaque direction pour ses participants**. |
| R-ECO5 | Secteur, contrat, UAI, entité émettrice sont **portés par l'unité, datés** ; un document prend les valeurs à sa date. |
| R-ECO6 | Un export officiel ne mélange **jamais** deux UAI. |
| R-ECO7 | L'appel au 1er degré se fait **par demi-journée et par classe**. |
| R-ECO8 | L'appel est fait par la **classe de référence**, même en décloisonnement. |
| R-ECO9 | Les demi-journées sont celles du **rythme de l'unité** à la date. |
| R-ECO10 | Aménagement de présence : **petite section, après-midi** seulement ; demande signée ; échéances 2 jours ouvrés et 15 jours calculées. |
| R-ECO11 | Avant 3 ans, les absences **ne comptent pas** au seuil d'absentéisme. |
| R-ECO12 | Le seuil de 4 demi-journées par **mois civil** crée une **tâche** ; **aucun** envoi automatique à l'IEN, au DASEN ou à la commune. |
| R-ECO13 | Le caractère légitime d'un motif est **décidé par une personne**. |
| R-ECO14 | Le registre d'appel mensuel est **figé** ; une correction est un avenant. |
| R-ECO15 | Le module de sanctions du socle 13 est **désactivé** pour une unité du 1er degré ; restent observation, information de la famille, équipe éducative. |
| R-ECO16 | L'enfant n'est **remis qu'à une personne autorisée** ; une interdiction bloque. |
| R-ECO17 | L'accès d'un remplaçant est **limité à la période** de remplacement ; la fiche du jour expire à minuit. |
| R-ECO18 | Les destinataires sont les **titulaires** ; l'enfant n'a pas de compte avant le CM (Q-ECO6). |
| R-ECO19 | **Jamais** de sortie seul pour un enfant de maternelle ; en élémentaire, autorisation écrite. |
| R-ECO20 | Un import ONDE **propose**, il n'écrase jamais ; chaque écart est décidé. |
| R-ECO21 | L'UAI d'un fichier importé doit être celui de l'unité. |
| R-ECO22 | Une photo d'enfant n'est prise, conservée et publiée **que** si le droit à l'image est accordé **pour cet usage** ; sinon, l'enfant est retiré de la photo avant publication. |
| R-ECO23 | **Aucune** reconnaissance faciale ni étiquetage automatique d'un enfant sur une image. |
| R-ECO24 | Les photos sont prises sur un **appareil de l'école** et vont directement au stockage chiffré (E5). |
| R-ECO25 | Pas de décision de maintien sans **dialogue tracé** avant la fin du 2e trimestre ; second maintien ou handicap : **avis de l'IEN**. |
| R-ECO26 | Le délai de recours de 15 jours court **depuis la notification**. |
| R-ECO27 | Le conseil d'école se convoque **8 jours** avant ; sa composition est calculée au nombre de classes. |
| R-ECO28 | Un membre extérieur d'une instance ne voit **aucune** donnée d'élève. |
| R-ECO29 | Le cahier journal est **privé** à la classe ; il s'ouvre par geste de l'enseignant, journalisé. |
| R-ECO30 | Un échange avec la commune est **déclaré, minimal, tracé** ; **jamais** de donnée de santé. |
| R-ECO31 | L'ATSEM voit sa classe, ses horaires, et seulement : identité, photo, remises, PAI minimal, régime du jour, présence. |
| R-ECO32 | Une donnée de catégorie socio-professionnelle n'est collectée que si l'unité publique doit la fournir à ONDE. |
| R-ECO33 | La liaison école-collège transmet le LSU et les plans (34, 35) ; **jamais** le carnet de maternelle, les observations, le cahier de liaison ni le cahier journal. |
| R-ECO34 | Les bilans du LSU sont **visés** par un titulaire ; tous les titulaires peuvent viser. |
| R-ECO35 | Aucune **note chiffrée** par défaut dans une unité du 1er degré (S-5003). |
| R-ECO36 | L'IA ne propose **ni** positionnement **ni** décision de passage. |
| R-ECO37 | Les APC exigent l'**accord** de la famille ; une absence aux APC n'est pas une absence scolaire. |
| R-ECO38 | Le rythme respecte 6 h/jour et 24 h/semaine ; une dérogation 4 jours est datée (3 ans au plus). |
| R-ECO39 | Le format « ONDE » n'est **jamais** revendiqué sans spécification officielle. |
| R-ECO40 | Toute action sensible du socle est journalisée (E7) ; tout écart de reprise est listé. |

---

## 20. Droits et minimisation

### 20.1 Catalogue d'actions proposé (D-017) [R]
`structure.unites` (créer et modifier les unités : coordination) · portée **`mon_unite`** (nouvelle portée du moteur, déclarée par le socle 5 à la demande du 50) · `ecole.cahier_journal` · `ecole.observer` · `ecole.carnet_publier` · `ecole.bilans` · `ecole.amenagements` · `ecole.maintien` · `ecole.conseil_ecole` · `ecole.apc` · `accueil.remettre` · `interop.onde` · `interop.commune` · modèles de rôles par défaut : **Direction d'école** (portée `mon_unite`), **Enseignant du 1er degré** (portée cours), **Remplaçant** (cours à la date), **ATSEM** (classe et horaires, lecture limitée), **Intervenant** (cours), **Coordination du groupe** (établissement).

### 20.2 Ce que voit chacun
| Donnée | ENS | REM | ATS | DIR (unité) | Direction du collège | FAM | EXT |
|---|---|---|---|---|---|---|---|
| Identité, photo | sa classe | sa classe, à la date | sa classe | unité | annuaire seulement | ses enfants | non |
| Remises, interdictions | sa classe | idem | idem | unité | non | les siennes | non |
| PAI (minimal) | sa classe | idem | idem | unité | non (sauf liaison) | oui | non |
| Coordonnées | sa classe | non | non | unité | non | les siennes | non |
| Cahier journal | sa classe | si publié | non | sur ouverture | non | non | non |
| Carnet, bilans | sa classe | non | non | unité | non (LSU à la montée) | ses enfants | non |
| Absences | sa classe | du jour | du jour | unité | non | ses enfants | non |
| Équipe éducative | participant | non | non | oui | non | la sienne | non |

### 20.3 Conservation [R, durées soumises au DPO, Q-ECO14]
Registre d'appel : durée de l'archive réglementaire [NV] ; carnet de suivi : fin du cycle 1 + remise à la famille, puis destruction des photos à 1 an [R] ; cahier journal : fin d'année + 1 an [R] ; dossiers de maintien : scolarité primaire + 1 an [R] ; échanges avec la commune : 1 an [R] ; imports ONDE : fichier source détruit à 60 jours (zone d'import transitoire, comme S-1521 du 15).

### 20.4 Pièges à tester
Direction du collège qui lit un bilan de CE1 ; ATSEM qui lit une adresse ; remplaçant qui revient le lendemain ; photo d'un enfant sans droit à l'image vue par une autre famille ; export ONDE d'un groupe de 1 400 élèves ; fichier ONDE d'une autre école ; parent séparé privé d'autorité qui vise un bilan ; DDEN qui ouvre le lien du conseil d'école et cherche une liste d'élèves ; agent de mairie qui demande les allergies ; IA à qui l'on demande « quels enfants risquent de redoubler ».

---

## 21. Événements et notifications

**Émis** (outbox, dans la transaction) : `structure.unite_creee|modifiee|fermee`, `ecole.apc_planifiee`, `ecole.amenagement_*`, `assiduite.seuil_atteint`, `assiduite.mois_arrete`, `accueil.remis|remise_refusee|non_repris`, `ecole.observation`, `ecole.carnet_publie`, `ecole.bilans_*`, `ecole.synthese_gs_*`, `ecole.maintien_*`, `ecole.passage_*`, `ecole.recours_*`, `ecole.liaison_transmise`, `ecole.fiche_jour`.
**Consommés** : `eleve.*` (2), `inscriptions.*` (3), `personnes.*` (4 : autorisation de remise, interdiction), `structure.*` (5), `personnels.*` (6 : fin d'engagement d'une ATSEM ⇒ accès coupé), `sante.consigne_publiee` (34), `sorties.*` (42 : enfant en sortie à l'appel).
**Notifications** (natures du socle 8) : absence non signalée (famille), remise ponctuelle enregistrée (l'autre foyer, si S-5011), mot du cahier de liaison, carnet publié, bilan à viser, décision de passage (notification avec preuve), convocation du conseil d'école, aménagement accordé ou refusé.

---

## 22. Écrans et mobile (`DESIGN_SYSTEM.md`, `FRONTEND_CIBLE.md`)

- **Direction d'école — « Ce matin »** (tuiles Bento) : absents non signalés, remplaçant attendu, remises particulières du soir, aménagements à échéance, seuils atteints, mots non vus, conseil d'école à convoquer.
- **Classe (enseignant)** : appel de la demi-journée en grille de photos ; « ce soir » (qui vient chercher) ; cahier journal de la semaine ; observations ; bilans de la période.
- **Maternelle (tablette)** : prendre une observation en photo, choisir l'attendu, choisir les enfants (seuls ceux dont le droit à l'image l'autorise apparaissent pour la photo).
- **ATSEM (mobile)** : la classe du jour, remises, consignes minimales.
- **Famille** : « aujourd'hui » de chaque enfant (présent, qui vient le chercher, mot du jour), déclarer une remise ponctuelle, signaler une absence, carnet, bilans à viser.
- **Coordination du groupe** : vue par unité, effectifs, indicateurs.
- **Mobile (socle 11)** : appel, remise, carnet ; mode hors ligne du jour chiffré ; pas d'appli enfant.

---

## 23. Modèle de données cible (brique `ecole`) [R]

Toutes les tables : `etablissement_id UUID NOT NULL` (E1), clé composite `(etablissement_id, id)` et FK composites (E2), UUID applicatifs (E3), `cree_le`, `cree_par`, `modifie_le`, `version` (E6), RLS (E13), aucune colonne JSON ni tableau (E10, D-036), fichiers par `ent_objet` (E5), classement au registre (E12). Les journaux sont **en ajout seul** (déclencheur).

```sql
-- EC1 (3000–3001) : unités du groupe scolaire (tables de structure, avec l'accord du socle 5)
ent_unite (id, etablissement_id, code, libelle, nature CHECK (ecole_maternelle|ecole_elementaire|ecole_primaire|
  college|lycee_gt|lycee_pro|section_bts|cpge|ufa|autre), degre CHECK (premier|second|post_bac),
  uai TEXT NULL CHECK (uai ~ '^[0-9]{7}[A-Z]$'), du DATE, au DATE NULL, UNIQUE (etablissement_id, uai))
ent_unite_statut (id, etablissement_id, unite_id, du, au NULL, secteur CHECK (public|prive_sous_contrat|prive_hors_contrat),
  contrat CHECK (association|simple) NULL, entite_emettrice_id NULL, entite_facturation_id NULL,
  EXCLUDE gist (unite_id WITH =, daterange(du, au) WITH &&))
ent_unite_site (etablissement_id, unite_id, site_id, du, au NULL)
ent_unite_direction (id, etablissement_id, unite_id, agent_id, role CHECK (direction|adjoint|coordination), du, au NULL)
ent_classe_unite (etablissement_id, classe_id, unite_id, du, au NULL)   -- une seule unité par classe à une date (exclusion)
ent_groupe_unite (etablissement_id, groupe_id, unite_id)
ent_unite_journal (id, etablissement_id, unite_id, a, acteur_personne_id, action, objet, valeur_avant TEXT, valeur_apres TEXT, motif) -- ajout seul

-- EC2 (3002) : rythme par unité (remplace jours_ouvres JSONB et degres TEXT[] pour les unités)
ent_unite_rythme (id, etablissement_id, unite_id, du, au NULL, derogation_quatre_jours BOOL, decision_objet_id NULL, decision_fin DATE NULL)
ent_unite_rythme_demi_journee (id, etablissement_id, rythme_id, jour_semaine SMALLINT CHECK (1..6),
  moment CHECK (matin|apres_midi), entree TIME, sortie TIME, UNIQUE (rythme_id, jour_semaine, moment))
ent_ecole_apc (id, etablissement_id, unite_id, enseignant_agent_id, groupe_id, du, au, minutes_prevues INT)
ent_ecole_apc_seance (id, etablissement_id, apc_id, jour, debut TIME, fin TIME, faite BOOL)

-- EC3 (3003–3004) : registre et aménagements
ent_ecole_registre_mois (id, etablissement_id, classe_id, mois DATE, arrete_le NULL, arrete_par NULL,
  signe_enseignant_le NULL, signe_direction_le NULL, objet_pdf_id NULL, UNIQUE (classe_id, mois))
ent_ecole_registre_avenant (id, etablissement_id, registre_id, a, acteur, motif, description TEXT) -- ajout seul
ent_ecole_amenagement (id, etablissement_id, eleve_id, version INT, demande_formulaire_reponse_id,
  etat CHECK (demande|avis_favorable|avis_defavorable|transmis|accorde_tacite|accorde|refuse|partiel|clos),
  avis_le NULL, avis_par NULL, transmis_le NULL, echeance_transmission DATE, accord_tacite_le DATE NULL,
  decision_le NULL, decision_objet_id NULL, du DATE, au DATE NULL)
ent_ecole_amenagement_jour (etablissement_id, amenagement_id, jour_semaine SMALLINT, PRIMARY KEY (amenagement_id, jour_semaine))
-- (le moment est toujours l'après-midi : contrainte de la règle R-ECO10, pas de colonne)
ent_ecole_seuil_mois (id, etablissement_id, eleve_id, mois DATE, demi_journees INT, atteint_le, tache_id NULL,
  equipe_educative_rdv_id NULL, transmis_ien_le NULL, transmis_dasen_le NULL)

-- EC4 (3005) : remise de l'enfant (complète le socle 14)
ent_ecole_remise_ponctuelle (id, etablissement_id, eleve_id, jour DATE, personne_id NULL, nom TEXT NULL,
  declare_par_personne_id, piece_controlee BOOL, cree_le)
ent_ecole_non_repris (id, etablissement_id, eleve_id, jour, heure_constat, issue CHECK (remis|garderie|autre), remis_a_personne_id NULL)
ent_ecole_non_repris_appel (id, etablissement_id, non_repris_id, a, personne_id, resultat CHECK (joint|injoignable|refus)) -- ajout seul

-- EC6 (3006) : cahier journal, cahier de liaison
ent_ecole_cahier_journal (id, etablissement_id, cours_id, jour DATE, UNIQUE (cours_id, jour))
ent_ecole_cahier_journal_ligne (id, etablissement_id, journal_id, ordre, domaine_id NULL, debut TIME NULL, prevu TEXT, fait TEXT NULL)
ent_ecole_cahier_partage (id, etablissement_id, cours_id, avec_personne_id, du, au, motif) -- journal des ouvertures
ent_ecole_liaison_mot (id, etablissement_id, eleve_id, auteur_personne_id, a, texte, lot_id NULL)
ent_ecole_liaison_vu (etablissement_id, mot_id, personne_id, a, PRIMARY KEY (mot_id, personne_id))
ent_ecole_liaison_reponse (id, etablissement_id, mot_id, personne_id, a, texte)

-- EC7 (3007–3009) : carnet, synthèse GS, bilans LSU (les positionnements restent au socle 22)
ent_ecole_observation (id, etablissement_id, cours_id, auteur_agent_id, observe_le DATE, attendu_element_id, commentaire TEXT)
ent_ecole_observation_eleve (etablissement_id, observation_id, eleve_id, PRIMARY KEY (observation_id, eleve_id))
ent_ecole_observation_media (id, etablissement_id, observation_id, objet_id, retouche_de_objet_id NULL)
ent_ecole_observation_media_retrait (etablissement_id, media_id, eleve_id, mode CHECK (flou|recadrage|retire))
ent_ecole_carnet_publication (id, etablissement_id, cours_id, periode_id NULL, publie_le, publie_par)
ent_ecole_carnet_publication_ligne (etablissement_id, publication_id, observation_id)
ent_ecole_synthese_gs (id, etablissement_id, eleve_id, annee_id, etat CHECK (brouillon|validee|communiquee), valide_le NULL, objet_pdf_id NULL)
ent_ecole_synthese_gs_rubrique (id, etablissement_id, synthese_id, rubrique_code, ce_que_sait_faire TEXT, points_attention TEXT NULL)
ent_ecole_bilan_periodique (id, etablissement_id, eleve_id, periode_id, etat CHECK (brouillon|arrete|publie), appreciation TEXT,
  signe_enseignant_le NULL, signe_direction_le NULL, publie_le NULL)
ent_ecole_bilan_visa (etablissement_id, bilan_id, personne_id, a, PRIMARY KEY (bilan_id, personne_id))

-- EC8 (3010–3011) : passage, conseils, conseil d'école
ent_ecole_maintien (id, etablissement_id, eleve_id, annee_id, nature CHECK (maintien|raccourcissement),
  etat CHECK (ouvert|dialogue_tenu|decide|notifie|recours|definitif|abandonne), dialogue_le DATE NULL,
  avis_ien_objet_id NULL, decision CHECK (passage|maintien|raccourcissement) NULL, decision_le NULL, notifie_le NULL)
ent_ecole_maintien_accompagnement (etablissement_id, maintien_id, source CHECK (ppre|apc|rased|pap|pps|autre), source_ref UUID NULL, description TEXT)
ent_ecole_recours (id, etablissement_id, maintien_id, depose_le, depose_par_personne_id, tardif BOOL, transmis_le NULL,
  decision CHECK (passage|maintien|raccourcissement) NULL, decision_le NULL, decision_objet_id NULL)
ent_ecole_instance (id, etablissement_id, unite_id, nature CHECK (conseil_maitres|conseil_cycle|conseil_ecole|equipe_educative|conseil_ecole_college), annee_id)
ent_ecole_instance_membre (id, etablissement_id, instance_id, personne_id, qualite CHECK (president|enseignant|parent_elu|maire|conseiller_municipal|rased|dden|ien|invite), du, au NULL)
ent_ecole_seance_instance (id, etablissement_id, instance_id, reunion_id, convoque_le, ordre_du_jour_objet_id NULL, pv_objet_id NULL)

-- EC9 (3012) : personnels propres à l'école (complète le socle 6)
ent_ecole_atsem_affectation (id, etablissement_id, agent_id, classe_id, du, au NULL)
ent_ecole_atsem_horaire (id, etablissement_id, affectation_id, jour_semaine, debut TIME, fin TIME)

-- EC10 (3013) : commune et échanges
ent_ecole_commune (id, etablissement_id, unite_id, code_commune TEXT CHECK (code_commune ~ '^[0-9AB]{5}$'), libelle, convention_objet_id NULL, du, au NULL)
ent_ecole_commune_echange (id, etablissement_id, commune_id, nature CHECK (effectifs|absents_jour_nombre|absents_jour_liste|residents|garderie_import),
  frequence, finalite TEXT, du, au NULL)
ent_ecole_commune_echange_champ (etablissement_id, echange_id, champ_code)   -- liste fermée vérifiée par le service
ent_ecole_commune_envoi (id, etablissement_id, echange_id, a, lignes INT, acteur, objet_id NULL) -- ajout seul

-- EC11 (3014) : liaison
ent_ecole_liaison_transmission (id, etablissement_id, eleve_id, de_unite_id, vers_unite_id, a, nature CHECK (lsu|synthese_gs|plan_34|plan_35))

-- EC12 (3015) : import ONDE (zone transitoire en colonnes TEXT, détruite à 60 jours)
ent_onde_import (id, etablissement_id, unite_id, uai_fichier, objet_id, recu_le, lignes INT, etat CHECK (recu|rapproche|traite|purge))
ent_onde_import_ligne (id, etablissement_id, import_id, numero INT, ine TEXT NULL, nom TEXT, prenom TEXT, naissance TEXT,
  niveau TEXT NULL, classe TEXT NULL, eleve_id NULL, statut CHECK (identique|ecart|inconnu_scolia|inconnu_onde|rejete))
ent_onde_import_ecart (id, etablissement_id, ligne_id, champ_code, valeur_onde TEXT, valeur_scolia TEXT, decision CHECK (attente|acceptee|rejetee), proposition_id NULL)
```

**Registre des tables (E12)** : toutes `etablissement`, export `simple` sauf `ent_ecole_observation_media*` (`particuliere` : photos d'enfants), `ent_onde_import*` (volatile après 60 jours), `ent_ecole_commune_envoi` (neutralisé à la restauration : pas de renvoi, E9). **Exceptions D-036** : **aucune** (journaux en colonnes texte, listes en tables, aucun tableau).

---

## 24. Lots de code (équipe Sonnet) et migrations

| Lot | Contenu | Dépend de | Critères de fin | Migration |
|---|---|---|---|---|
| **EC0** correctifs urgents | ED-01 à ED-05 : élèves scolarisés à la date, date obligatoire, pagination ou flux, échappeur partagé (aussi Notanet, Parcoursup), libellé « format ScolIA » | — | tests : élève sorti absent de l'export, 1 400 élèves complets, formule neutralisée, nom de fichier | **aucune** |
| **EC1** unités du groupe | §6, ECO-100 à ECO-109 ; portée `mon_unite` fermée par défaut ; reprise : unité unique créée depuis le profil, classes rattachées par proposition | 5 S0–S1 ; 1 (moteur de portée) | tests d'isolation : direction de l'école / du collège ; R-ECO2, R-ECO3 | 3000–3001 |
| **EC2** structure et rythme d'école | ECO-110 à ECO-122 ; rythme par unité ; APC | EC1 ; 5 S3 ; 10 T1 | 6 h/24 h contrôlés ; classe multi-niveaux | 3002 |
| **EC3** appel et assiduité 1er degré | §8, ECO-130 à ECO-155 | EC2 ; 12 (appel v2, mois arrêtés) | seuil par mois civil ; aménagement PS bout à bout ; registre figé | 3003–3004 |
| **EC4** remise de l'enfant | ECO-160 à ECO-170 | 4 A3 ; 14 | interdiction bloquante ; non repris ; hors ligne du jour | 3005 |
| **EC5** discipline 1er degré | R-ECO15 (désactivation, observations, équipe éducative) | 13 D1 | aucune sanction possible dans une unité du 1er degré | aucune |
| **EC6** cahier journal et liaison | ECO-180 à ECO-192 | EC2 ; 16 ; 20 | cahier privé ; « vu » ; fiche du jour expirée | 3006 |
| **EC7** carnet, synthèse, bilans | ECO-200 à ECO-219 | 22 ; 18 (droit à l'image par usage) ; 7 (objets) | R-ECO22 à R-ECO24 testés ; visa | 3007–3009 |
| **EC8** passage et instances | ECO-220 à ECO-235 | 19 ; 23 ; 46 | maintien bloqué sans dialogue ; 15 jours ; composition du conseil | 3010–3011 |
| **EC9** personnels d'école | ECO-240 à ECO-245 | 6 ; 15 | ATSEM : lecture limitée testée | 3012 |
| **EC10** commune, périscolaire, finances | ECO-250 à ECO-257 | 9 X7 ; 24 ; 40 ; 43 | aucun champ de santé exportable | 3013 |
| **EC11** inscriptions et liaison | ECO-265 à ECO-269 | 3 ; 34 ; 35 | R-ECO33 testée | 3014 |
| **EC12** ONDE | ECO-260 à ECO-263 | 9 X7 ; 2 E5 | UAI contrôlé ; aucune écriture | 3015 |
| **EC13** reprise, conservation, retraits | ECO-272, ECO-273 ; purge ; RLS stricte ; retrait de `degres` et de `jours_ouvres` (avec le socle 5) | tous | registre E12 complet | 3016 (reprise), 3017 (personas), 3018 (RLS stricte), 3019 (retraits, sur décision, E8) |

---

## 25. Décisions appliquées par défaut (D-031) et questions

### 25.1 Appliquées par défaut (D-031)
| # | Question | Recommandation appliquée |
|---|---|---|
| Q-ECO2 | Groupe scolaire = un seul établissement ScolIA ? | **Oui**, découpé en unités (R-ECO1) |
| Q-ECO3 | Où vivent les unités ? | Tables de **structure** (socle 5), créées dans la plage 3000 du 50 avec son accord |
| Q-ECO4 | Portée par unité fermée par défaut ? | **Oui** (R-ECO3) |
| Q-ECO5 | Notes chiffrées au 1er degré ? | **Non par défaut**, activables par unité (S-5003) |
| Q-ECO6 | Compte enfant ? | **Pas avant le CM** ; au CM, compte élève simple optionnel (S-5012) |
| Q-ECO7 | Écrire dans ONDE ? | **Non** ; import et rapprochement seulement (R-ECO20) |
| Q-ECO8 | Accès direct de l'IEN ? | **Non par défaut** ; documents transmis ; accès lecteur nominatif sur décision |
| Q-ECO9 | Cantine municipale : liste nominative des absents ? | **Nombre par classe par défaut** (S-5007) |
| Q-ECO17 | Photos du carnet : galerie du téléphone autorisée ? | **Non par défaut** (S-5006) |
| Q-ECO18 | Le carnet suit-il l'enfant hors du groupe ? | **Remis à la famille** en PDF ; pas de transmission automatique |
| Q-ECO19 | Sanctions au 1er degré ? | **Désactivées** (R-ECO15) |
| Q-ECO20 | Appel par la classe de référence en décloisonnement ? | **Oui** (R-ECO8) |
| Q-ECO21 | Seuil d'absentéisme : envoi automatique ? | **Jamais** (R-ECO12) |
| Q-ECO22 | ATSEM : accès toute la journée ou sur ses horaires ? | **Toute la journée scolaire** de ses classes, lecture limitée (S-5008 permet de réduire) |
| Q-ECO23 | CSP des responsables ? | **Seulement** si l'unité publique doit la fournir (R-ECO32) |
| Q-ECO24 | Cahier journal lisible par la direction ? | **Sur ouverture** par l'enseignant (R-ECO29) |
| Q-ECO25 | Remise ponctuelle : un titulaire suffit ? | **Oui**, sauf désaccord connu ; l'autre foyer est informé si S-5011 |
| Q-ECO26 | Membres extérieurs des instances ? | **Sans compte**, lien nominatif, aucune donnée élève (R-ECO28) |
| Q-ECO27 | Liaison CM2-6e dans le groupe : quoi ? | LSU et plans seulement (R-ECO33) |
| Q-ECO28 | Export « ONDE » actuel ? | **Renommé** et corrigé en EC0 (ED-04) |
| Q-ECO29 | Échappeur CSV commun à tous les exports ? | **Oui**, EC0 (ED-03) |
| Q-ECO30 | Retrait de `degres TEXT[]` et `jours_ouvres JSONB` ? | **Oui**, au lot de retrait, avec le socle 5 |
| Q-ECO31 | Enfant avant 3 ans au seuil ? | **Non compté** (R-ECO11) |
| Q-ECO32 | Visa du LSU par un seul ou par tous les titulaires ? | **Un suffit**, tous peuvent (R-ECO34) |

### 25.2 Soumis à Florian (engagent l'établissement ou sont des faits que seul Florian connaît)
| # | Point | Pourquoi |
|---|---|---|
| **Q-ECO1** | Le groupe pilote a-t-il une **école** ? Entre-t-elle dans l'année de doublon 2027-2028 ? | Décide si le socle 50 est en V1 (février 2027) ou après (Q-E de `SOCLES.md`) |
| **Q-ECO33** | Une ou plusieurs **personnes morales** (OGEC unique pour école, collège, lycée ?) et **contrats** de chaque unité | Données réelles ; paramétrage des unités |
| **Q-ECO34** | **Communes** concernées et **conventions** d'échange (cantine, périscolaire, forfait communal) | Engagement extérieur |
| **Q-ECO35** | **Démarche éditeur** ONDE V2 et LSU 1D (Pléiade) | Démarche officielle (D-029, F-06) |
| **Q-ECO36** | Écoles **publiques** : ScolIA candidat au SDET 1er degré ou complément d'un ENT labellisé ? | Stratégie commerciale et conformité |
| **Q-ECO37** | **ATSEM et agents municipaux** : la commune accepte-t-elle qu'ils aient un compte ScolIA ? | Accord de l'employeur |
| **Q-ECO14** | Durées de conservation (registre, carnet, photos) | DPO |

### 25.3 À vérifier (réglementaire, avant le lot)
Q-ECO10 (intitulé et texte de l'arrêté ONDE de 2023) ; Q-ECO11 (R131-6/7 après 2022-182) ; Q-ECO12 (modèle de synthèse GS après les programmes 2026) ; Q-ECO13 (pièces d'admission, vaccinations) ; Q-ECO15 (bilans transmis en liaison CM2-6e) ; Q-ECO16 (date d'effet de l'obligation à 3 ans).

### 25.4 Réglages signalés (D-023)
| # | Réglage | Défaut | Avis |
|---|---|---|---|
| S-5000 | Heure de fermeture de l'appel du matin | 9 h 00 | utile |
| S-5001 | Seuil d'arrivée qui annule l'absence de la demi-journée | 10 h 00 | utile |
| S-5002 | Délai avant procédure « non repris » | 15 min | utile |
| S-5003 | Notes chiffrées dans l'unité | non | à garder (écoles qui notent [H]) |
| S-5004 | Publication du carnet : immédiate ou par période | par période | utile |
| S-5005 | Photo de classe du photographe : consentement séparé | oui | utile |
| S-5006 | Dépôt de photos depuis la galerie | non | sécurité |
| S-5007 | Absents du jour à la commune : nombre / liste | nombre | conformité |
| S-5008 | ATSEM : accès sur ses horaires seulement | non | candidat à l'allègement |
| S-5009 | Date de publication des listes de classe | fin août | utile |
| S-5010 | Relance des mots non vus | J+2 | utile |
| S-5011 | Informer l'autre foyer d'une remise ponctuelle | non | sensible (familles séparées) |
| S-5012 | Compte élève au CM | non | à décider à l'usage |
| S-5013 | Nombre de périodes des bilans | 3 | à vérifier (Q-ECO, [NV]) |
| S-5014 | (ajout personas) Réponses du cahier de liaison visibles de l'autre foyer | non | sensible (familles séparées) |
| S-5015 | (ajout personas) Téléchargement des photos de groupe par les familles | non | droit à l'image |

---

## 26. Ajout personas (`PERSONAS_50.md`)

> Cette section est écrite **après** la passe personas ; elle l'emporte sur ce qui précède en cas de contradiction.

### 26.1 Bloquants tranchés (appliqués par défaut, D-031)
- **A. Le registre et l'appel doivent marcher sans réseau et sans l'enseignant habituel** (P03, P05) : l'appel du matin du remplaçant et la liste des remises sont disponibles **hors ligne pour la journée**, chiffrés sur l'appareil de l'école, effacés le soir (déjà ECO-169 ; étendu à l'appel : ECO-274).
- **B. La remise de l'enfant doit fonctionner pour la garderie et la cantine municipales**, dont le personnel n'est pas dans ScolIA (P07, P10) : **liste du soir imprimable** par classe et par activité (ECO-275), et, si la commune l'accepte (Q-ECO37), **compte agent municipal** limité à la remise.
- **C. Le droit à l'image doit être par usage et par enfant, et visible au moment de la photo** (P02, P13) : l'écran de prise de vue ne propose que les enfants autorisés **pour l'usage « carnet »** ; un retrait d'accord en cours d'année **retire** les photos déjà publiées de la vue des familles sous 24 h (ECO-276, R-ECO41).
- **D. La séparation des unités doit valoir aussi pour la coordination quand elle n'est pas nécessaire** (P06, P14) : la coordination voit les **indicateurs** de toutes les unités, mais les **dossiers** d'une unité seulement par la fonction de direction de cette unité ou par une mission datée (R-ECO42) ; l'accès « tout l'établissement » est une mission explicite, journalisée.

### 26.2 Actions ajoutées (ECO-274 à ECO-279)
| N° | Action | Lot |
|---|---|---|
| ECO-274 | Appel du jour hors ligne (remplaçant compris), synchronisation au retour du réseau, conflits signalés | EC3 |
| ECO-275 | Liste du soir imprimable par classe et par activité (garderie, cantine, APC) avec remises et interdictions | EC4 |
| ECO-276 | Retrait d'un consentement à l'image : dépublication des médias concernés sous 24 h et compte rendu à la famille | EC7 |
| ECO-277 | Mot du cahier de liaison **traduit** pour une famille allophone (IA, relu) avec l'original joint | EC6 |
| ECO-278 | Déclarer un **projet d'accueil** de rentrée d'un enfant à besoins (avant la rentrée : rencontre famille, AESH, aménagements) | EC2 |
| ECO-279 | Tableau de rentrée de l'école (classes sans enseignant, enfants sans classe, PAI non renouvelés, autorisations de remise manquantes, droits à l'image non recueillis) | EC2 |

### 26.3 Règles ajoutées
| # | Règle |
|---|---|
| R-ECO41 | Le retrait d'un consentement à l'image dépublie les médias sous 24 h ; les médias restent visibles seulement de l'enseignant jusqu'à leur destruction (fin d'année). |
| R-ECO42 | La coordination voit les indicateurs de toutes les unités ; les dossiers d'une unité seulement par sa direction ou une mission datée. |
| R-ECO43 | Un parent **salarié** ou **élu** n'obtient aucune donnée d'élève par cette casquette (casquettes du socle 6). |
| R-ECO44 | Une liste imprimée de remises porte la date, la classe et la mention « à détruire ce soir ». |
| R-ECO45 | L'appel hors ligne ne contient que la classe du jour ; il s'efface à minuit ou à la synchronisation. |

### 26.4 Cas limites ajoutés (CEC-57 à CEC-64)
CEC-57 : retrait du droit à l'image en mars (R-ECO41) · CEC-58 : remplaçant sans réseau à 8 h 30 (R-ECO45) · CEC-59 : agent de garderie municipale sans compte (liste imprimée, R-ECO44) · CEC-60 : coordinatrice qui ouvre le dossier d'un enfant de l'école sans mission (refus, R-ECO42) · CEC-61 : parent élu qui demande la liste des familles d'une classe pour une campagne (refus ; diffusion par l'école, socle 46) · CEC-62 : famille qui signale une absence après la fermeture de l'appel (absence non signalée requalifiée, pas de second avis) · CEC-63 : enfant de PS avec aménagement qui reste exceptionnellement l'après-midi (présent, pas d'absence, aménagement inchangé) · CEC-64 : enseignant qui photographie la classe pour la fête de l'école (usage « communication » distinct du « carnet »).

### 26.5 bis Précisions d'actions et de règles existantes (`PERSONAS_50.md` §6.2)
- **ECO-133** liste aussi les **classes dont l'appel n'est pas fait** à l'heure de fermeture (M-5000).
- **ECO-119** crée une **date d'arrivée par enfant** ; l'appel ne l'attend qu'à partir de cette date (M-5006, attente au socle 12).
- **ECO-161** : une remise ponctuelle saisie par l'accueil au téléphone est **« à confirmer »** ; elle ne débloque la remise qu'après confirmation en ligne d'un titulaire **ou** rappel tracé au numéro du dossier (M-5014). La pièce d'identité contrôlée n'est **jamais** photographiée ni stockée : seules la case et l'heure (M-5037, ECO-163 aussi).
- **ECO-162** : le foyer du jour indique **qui reprend**, sans retirer l'autorisation de l'autre titulaire (M-5022) ; photo facultative de la personne autorisée, avec son consentement (M-5009).
- **ECO-205** : l'export PDF à la famille couvre aussi les **bilans** (M-5011).
- **ECO-268** : la transmission au collège du groupe peut porter une **note de liaison** rédigée par l'enseignant de CM2, **lisible par la famille** (M-5012).
- **R-ECO17** : la **fiche du jour** expire à minuit ; l'**accès à la classe** dure la période de remplacement.
- **R-ECO30** : l'adresse n'est transmise à la commune que si la convention le prévoit ; sinon, la commune de résidence suffit (M-5027).
- **R-ECO31** : la portée de l'ATSEM et des animateurs est **par classe et par activité** (garderie du jour : inscrits du jour, toutes classes) (M-5019).
- **Réglages ajoutés** : **S-5014** réponses du cahier de liaison visibles de l'autre foyer (défaut **non**) ; **S-5015** téléchargement des photos de groupe par les familles (défaut **non**).
- **Préalable de mise en service d'EC7** : **analyse d'impact (AIPD)** et avis du DPO avant d'activer les photos du carnet (M-5038, soumis à Florian).
- **EC7 est indépendant d'EC12 et de l'export LSU** : une école publique sans convention LSU utilise quand même l'appel, la remise, la liaison et le carnet (M-5031).

### 26.5 Lots révisés
EC4 passe **avant** EC3 si l'école entre dans le doublon (la remise est le geste à plus fort risque) ; EC7 dépend de l'usage « carnet » du droit à l'image au socle 18 (attente posée). Plage 3017 attribuée aux ajouts (appel hors ligne : journal de synchronisation ; dépublications ; missions de coordination).

---

## 27. Faits, hypothèses, contradictions

**Faits [F]** : relevés le 2026-11-11 sur `origin/ent-mc0hoe` (fichiers cités) ; rien compilé ni exécuté.
**Hypothèses [H]** : pratiques de terrain (heures, accueil échelonné, cahier journal, ATSEM, garderie, cantine municipale) ; à corriger par Florian et par la recette.
**Contradictions signalées**
- `SOCLES.md` §7 classe l'« ensemble scolaire » **hors carte** (Q13 : « contredit l'absence de visibilité entre établissements ») ; `02_ELEVE.md` Q-E13, `09_INTEROP.md` Q-X37 et ce socle le traitent comme **un seul établissement à plusieurs unités**, ce qui ne contredit pas D-013. Les deux lectures portent sur des choses différentes : Q13 vise un **réseau d'établissements distincts** ; l'unité vise un **établissement composite**. Proposition : clore Q13 dans ce sens.
- `332_structure_profil_parametres.sql` porte UAI, secteur et contrat au **tenant** ; `05_STRUCTURE.md` §5.2 dit que « l'UAI est porté par le site » ; ce socle les porte par l'**unité**. Le site garde son adresse ; l'UAI du site devient une valeur par défaut de l'unité.
- `24_FINANCES.md` CF-18 et Q-F24 traitent la fratrie « entre établissements d'un ensemble scolaire » ; avec les unités, la fratrie d'un groupe est **interne** : la règle de Q-F24 reste vraie pour les réseaux de plusieurs tenants.
- Le §8.2 du 42 cite des délais « 3 j / 15 j / 4-6 semaines » pour le premier degré ; non revérifiés ici.
