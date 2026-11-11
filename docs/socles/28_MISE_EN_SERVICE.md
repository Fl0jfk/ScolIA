# Socle 28 — Mise en service, reprise de données et support

**Date :** 2026-11-11 — **Statut :** conception de fond, **non décidée** (à relire par Florian) ; recommandations appliquées par défaut (D-031) sauf ce qui engage l'établissement ou l'éditeur à l'extérieur (§19.2). Aucun code écrit ni modifié ; aucun accès à une base, à un secret ou à un service de production ; aucune donnée réelle. Next.js lu seulement (lecture ciblée de `app/api/demo`, `app/api/onboarding`, et des documents de transition déjà écrits).
**Branche :** `ent-socle-27-28-conception`, partie de `origin/ent-mc0hoe`.
**Passe personas :** `docs/socles/PERSONAS_28.md` ; ajouts faits **ici**, marqués **« ajout personas »**, regroupés au **§21** (qui l'emporte en cas de contradiction ; rien n'est supprimé).
**À lire avec :** `CLAUDE.md` ; `docs/DECISIONS.md` D-009 (passkey pour tous), D-012 (un sous-domaine par établissement), D-013, D-017, D-018 (récupération), D-019 (public et privé), D-023 (peu de réglages), D-031, **D-034** (démo de décembre, V1 fin janvier–février 2027, **année de doublon 2027-2028**), **D-035** (l'add-on Next.js passe dans ScolIA), D-036, **D-037 à D-046** ; **`docs/TRANSITION_NEXT_RUST.md`** (calendrier T0–T4, principe de reprise rejouable) et **`docs/TRANSITION_T0_INVENTAIRE.md`** (inventaire, rapprochement, `ent-reprise`, anonymisation, lots R0–R11) — **le 28 les absorbe comme son volet « reprise depuis Next.js » et les généralise** ; `docs/MISE_EN_SERVICE_DONNEES.md` (A1–A15, B1–B14) ; `docs/A_FOURNIR_PAR_FLORIAN.md` (F-01 à F-08) ; `docs/PRA_SAUVEGARDES.md` (§10–§14) ; `docs/PORTABILITE_EXPORTS.md` ; `docs/ent-rust-prod-runbook.md`, `docs/migrations-prod-runbook.md`, `docs/ent-rust-observabilite-pra.md` ; `docs/MATRICE_CHARLEMAGNE.md` ; `docs/socles/09_INTEROP.md` (imports SIECLE/BEE, STS, formats), `27_RGPD_ARCHIVES.md` (portes de conformité), `29_PORTAILS.md` (accueil, aide), `42_SORTIES_VOYAGES.md` §32 (modèle de bascule par propriété unique). Modèles de forme : `42_SORTIES_VOYAGES.md`, `10_EMPLOI_DU_TEMPS.md`.

**Conventions.** **[F]** fait vérifié dans le code ; **[V]**/**[V-ext]** source officielle lue (§4) ; **[NV]** non vérifié ; **[H]** hypothèse ; **[R]** recommandation.

**Numérotation propre au socle** (vérifiée libre sur les 134 branches distantes le 2026-11-11) :

| Élément | Numéros |
|---|---|
| Défauts de l'existant | MSE-D01… |
| Actions cibles | **MSE-100 à MSE-279** |
| Règles | **R-MSE1…** |
| Cas limites | **CL28-01…** |
| Questions | **Q-MSE1…** |
| Réglages signalés | **S-560 à S-579** |
| Manques de la passe personas | M-2800 à M-2899 |
| **Migrations** | **2560–2579** (plage imposée par le pilote, §16) |

---

## 0. En bref

1. **Le premier frein au changement de logiciel est la peur de perdre ses données et de rater la rentrée** [H]. Le 28 répond par une promesse vérifiable : un établissement **paramétré, repris et ouvert sans perte**, avec un **rapport d'écarts** relu avant chaque ouverture, un **retour arrière** toujours possible, et un **support** qui aide sans jamais voir plus que ce que l'établissement autorise.
2. **Ce qui existe [F]** : un labo et des jeux de données fictives (`ent-lab`, `ent-db/src/seed.rs`, `scenarios.rs`, route `/labo/bootstrap` compilée seulement avec la fonctionnalité `labo`, vérifiée absente en production par la CI) ; un inventaire d'intégrité ligne à ligne (`/pra/inventaire`) ; des scripts de sauvegarde et d'exercice PRA ; des métriques (`ent-obs`) ; `/health` et `/ready` ; une déclaration d'établissement réservée à l'administrateur **de son propre établissement** (`etablissement_repo.rs:133-148`) — **aucun parcours de création d'un établissement par la plateforme** ; un import d'élèves au format maison (`scolia_eleves_v1`, `imports.rs`) avec lignes en **JSONB** et un état `annule` **jamais posé** ; un import d'emploi du temps à rapport JSONB. **Rien** pour : la reprise depuis Next.js ou Charlemagne (`ent-reprise` n'existe pas), le support, la page d'état, l'aide, l'abonnement.
3. **Sept piliers** :
   - **Création et paramétrage** d'un établissement par un parcours guidé, avec des **portes** qui ne s'ouvrent que si les prérequis sont remplis : *données fictives* → *données réelles* (accord écrit, DPO, AIPD, durées, socle 27) → *ouverture aux personnels* → *ouverture aux familles* (R-MSE1).
   - **Reprise de données** par un outil unique et rejouable, `ent-reprise`, avec des **adaptateurs de source** (Next.js d'abord, puis Charlemagne, SIECLE/BEE via le socle 9, École Directe, Pronote, tableurs), une **zone de reprise typée** (aucun JSON, D-036), une **table de correspondance** des identifiants, un **rapprochement** avec file humaine, un **mode simulation par défaut**, des **commandes des socles** pour écrire, un **rapport de contrôle** et l'**annulation par exécution**.
   - **Année de doublon (2027-2028)** : synchronisation périodique depuis Charlemagne, qui reste officiel, et **comparateur d'écarts** (absences, notes, effectifs) : c'est ce qui rendra le doublon utile au lieu d'une double saisie aveugle.
   - **Bascule module par module** depuis Next.js, sur le modèle éprouvé du socle 42 : **propriété unique** de chaque objet à tout instant, gel, reprise différentielle, retour arrière.
   - **Comptes** : activation par lots, première connexion par lien puis **passkey** (D-009), suivi de l'enrôlement, relances.
   - **Support tracé et consenti** : demandes de support par établissement ; **accès du support** seulement sur autorisation explicite d'un responsable de l'établissement, borné (durée, domaine, lecture par défaut), jamais à la santé, **visible** des familles dans le journal des accès (socle 27) ; **pas de « se connecter en tant que »**.
   - **Exploitation visible** : page d'état publique hors Scaleway, annonces de maintenance, notes de version, supervision et PRA testé (renvoi au PRA), abonnement et contrat.
4. **Réglementaire qui commande le logiciel** (§4) : la CNIL interdit d'utiliser des données réelles en test (fiche 11 du guide du développeur) ; le SDET 2024 exige la journalisation des personnels techniques, **prise en main à distance comprise**, une clause de réversibilité et une clause art. 28 ; le RGPD art. 28 et 32 encadrent le sous-traitant (confidentialité, sécurité, aide, audits) ; A7 interdit les données de santé sans hébergeur HDS.
5. **Le plan** :

   | Élément | Volume | Numéros |
   |---|---|---|
   | Actions | 180 | MSE-100 à MSE-279 |
   | Cas limites | 56 | CL28-01 à CL28-56 |
   | Règles | 36 | R-MSE1 à R-MSE36 |
   | Réglages signalés | 14 | S-560 à S-573 |
   | Lots | 14 | MS0 à MS13 |

   - **Lot MS0 (correctifs urgents)** : purge périodique des sessions et jetons ; l'état `annule` des imports enfin posé (annulation réelle) ou retiré ; route de création d'un établissement **par la plateforme** distincte de la déclaration locale ; `/health` public réduit (version détaillée réservée à la supervision) ; garde « jamais de production » des outils de labo étendue aux futurs outils de reprise.
   - **Démo de décembre 2026 (T1)** : MS0, MS1 (création d'établissement, environnement de démonstration), MS2 (portes et liste « prêt à ouvrir »), MS4 a (squelette `ent-reprise` : inspection d'un export fictif au format Next.js, = lot R0 de T0).
   - **Avant les premières données réelles** (printemps 2027) : MS3 (activation des modules), MS4 b–d (reprise Next.js : anonymisation, zone, rapprochement, rapports, première répétition), MS6 (comptes), MS8 (support tracé), MS9 (page d'état et annonces).
   - **Avant la rentrée 2027** : MS5 (adaptateurs Charlemagne, SIECLE via le socle 9), MS7 (synchronisation et comparateur du doublon), MS10 (aide contextuelle et formation).
6. **Décisions appliquées par défaut (D-031)** : 26 (§19.1). **Soumis à Florian** (§19.2) : accord écrit de l'établissement et avis du DPO avant toute donnée réelle ; exports réels (Next.js, Charlemagne) lancés par Florian ; fournisseur de la page d'état et de l'hébergement hors Scaleway ; conditions de l'abonnement et du contrat (prix, durée, réversibilité, préavis) ; modèle de contrat de sous-traitance ; arrêt de chaque module Next.js.

---

## 1. Thèse : on ne migre pas, on reprend, on compare, on bascule

Les éditeurs du marché vendent souvent la migration comme une opération ponctuelle, faite par eux, en boîte noire, un week-end [H]. Les établissements en retiennent deux souvenirs : des données perdues ou dédoublées, et une dépendance totale à l'éditeur pour comprendre ce qui s'est passé.

ScolIA fait l'inverse, en trois idées tirées de `TRANSITION_NEXT_RUST.md` et généralisées :
1. **Reprendre, ne jamais transformer la source** : la base d'origine (Next.js, Charlemagne) reste intacte ; ScolIA lit une **copie** ; le retour arrière est trivial.
2. **Rejouer jusqu'à ce que ce soit juste** : la reprise est **idempotente** (deux exécutions = même résultat), **annulable** par exécution, **chronométrée** ; on répète sur des exports anonymisés, puis une fois sur un export réel en pré-production fermée, avant toute ouverture.
3. **Prouver** : chaque exécution produit un rapport lisible (comptages, totaux au centime, rapprochements, échantillons côte à côte, refus motivés) que l'établissement relit avant d'ouvrir. **Rien n'est perdu en silence.**

Et une quatrième, propre au projet : **l'année de doublon** n'est pas une double saisie, c'est une **expérience mesurée**. ScolIA tourne à côté de Charlemagne, se synchronise depuis lui et montre où ils divergent ; chaque divergence est un bogue, un manque ou une amélioration à livrer (D-034).

---

## 2. Évaluation franche de l'existant

### 2.1 ENT Rust [F] (inventaire du 2026-11-11)
| Brique | Où | État | Utilité pour le 28 |
|---|---|---|---|
| Labo | `ent-lab/src/main.rs` (labo exploratoire sur Postgres local, `seed_labo_cast` l. 104, 186) ; fonctionnalité `labo` désactivée par défaut | outil de développement ; **un mot de passe de développement figure dans un commentaire** (l. 4) | base de l'environnement de démonstration |
| Données fictives | `ent-db/src/seed.rs` (l. 1-5, `seed_labo_cast` l. 129), `scenarios.rs` | jeu de labo, scénarios | à enrichir en jeu de démonstration « établissement complet » (MS1) |
| Amorçage labo | `/labo/bootstrap` (`routes.rs:864-866, 888-890`) seulement avec `labo` ; CI vérifie l'absence de route labo dans le binaire de production (`.github/workflows/ent-rust.yml:109-121`) | **bonne garde** | même garde pour `ent-reprise` et la démo |
| Inventaire d'intégrité | `ent-db/src/pra.rs` (nombre + empreinte md5 par table, tables volatiles exclues) ; routes `routes.rs:140-141`, garde `require_sauvegarde` (`ent-api/src/auth.rs:175`), Direction ou administrateur de plateforme (`service.rs:6190`) | réel | contrôle de fin de reprise (§6.4.8) |
| Observabilité | `ent-obs/src/lib.rs` (registre de métriques au format Prometheus) | réel | supervision, page d'état (MS9) |
| Santé du service | `/health` (`routes.rs:137`, l. 2015-2023 : version, `git_sha`, `migration_tag`) ; `/ready` (l. 2025-2030, ping de la base) | réel ; `/health` expose le `git_sha` **publiquement** | page d'état ; réduire l'exposition (MSE-D05) |
| Scripts | `scripts/sauvegarde-postgres.sh` (dump, SHA-256, rétention, `ALLOW_PROD_BACKUP` réservé à un humain), `scripts/pra-exercice.sh` (restauration dans une base jetable, comparaison, `--autotest`) | réels, sans PITR ni destination distante (PRA §2.3) | runbooks (MS9) |
| Établissement | `ent-db/src/etablissement_repo.rs:1-7, 133-148` (déclaration par l'administrateur, limitée à son établissement) ; routes `routes.rs:143-146` (`/auth/etablissement`, `/etablissement/fiche`, `/etablissement/domaines`) ; migration 096 (`ent_etablissement`, `ent_etablissement_domaine`), 265 (fuseau `Europe/Paris`, `ent_aujourdhui_etab`) ; fuseau `structure.rs:35`, `structure_etab.rs:524` | **pas de parcours plateforme** : qui crée l'établissement et son premier administrateur n'est pas identifié (MSE-D03) | MS1 |
| Rôles modèles | `ent-auth/src/actions.rs:936` (`modele_par_defaut`), CRUD `roles.rs:24-31`, provisionnement d'un personnel `routes.rs:201` → `service.rs:3429` | réels | rôles modèles chargés à la création (MS1) |
| Imports | migration 144 : `ent_import_lot` (états `previsualise`/`applique`/`annule`, l. 10-25), `ent_import_ligne.donnees JSONB` (l. 32), `motifs TEXT[]` (l. 34), format unique `scolia_eleves_v1` ; moteur `ent-db/src/imports.rs` (5 000 lignes et 2 Mio au plus, prévisualisation l. 365-428, application l. 628-725 avec `FOR UPDATE`, événement l. 711) ; droit `imports.appliquer` | bon schéma « prévisualiser puis appliquer » ; **`annule` jamais posé** ; JSONB (D-036) | modèle de l'étape « simulation / application » ; à ranger sous le socle 9 (lignes typées, son lot 3990) |
| Import d'emploi du temps | migration 060 `ent_edt_import` (`dry_run`, `report_json JSONB`) ; route `routes.rs:648` → `service.rs:7706` | réel ; rapport en JSONB | socle 10 |
| SIECLE | source `import_siecle` acceptée pour les MEF (`referentiel_structure.rs:453-462`) ; AAF et BEE « à brancher » (144, l. 2-4) | embryonnaire | socle 9 |
| Purges | sessions et jetons purgés **au démarrage seulement** (`ent-api/src/lib.rs:251-256`, `service.rs:3702, 3774`) | partiel | MS0 |
| Support | aucune occurrence d'usurpation, de « se connecter en tant que », de console de support ni d'aide contextuelle | absent | MS8, MS10 |
| `ent-reprise` | n'existe pas | absent | MS4 |

### 2.2 Next.js [F, lecture ciblée]
- **Démo** : `GET /api/demo/enter` (`app/api/demo/enter`) avec un locataire de démonstration (`/demo?dev_tenant=default`), refusé en production ; `docs/demo-one-click.md` décrit la démo de l'application Next.js. **Repris** comme besoin (une démo en un clic), pas comme code.
- **Onboarding** : `app/api/onboarding/status` et `microsoft-licenses` (état des licences Microsoft de l'établissement) : sans objet pour ScolIA tel quel ; l'idée d'une **liste d'état de mise en service** est reprise (MSE-120).
- **Registre des établissements de la plateforme** : `tenants/index.json` sur S3, avec un dossier de secrets par établissement (`app/lib/tenant-registry.ts:14, 47-50`, T0 §2.4) : **à ne pas reproduire** (D-036 ; secrets hors stockage objet, `MISE_EN_SERVICE_DONNEES.md` A4).
- **Support** : suppression de dossiers de test par le support, avec simulation (`support-cleanup-tests`, NX-77 du socle 42) : l'idée « simulation avant toute opération de support » est reprise (R-MSE20).
- **Facturation des établissements clients** par EasyTransac (`PERSONAS_42.md` §0 B, T0) : le module « abonnement » du 28 en hérite le besoin.

### 2.3 Documents existants à absorber
`TRANSITION_NEXT_RUST.md` (principe, calendrier T0–T4), `TRANSITION_T0_INVENTAIRE.md` (inventaire, règles de rapprochement §6, risques et contrôles §7, `ent-reprise` §8, anonymisation §9, lots R0–R11 §10, questions Q-T0-01 à 15) : **le 28 en fait sa partie « adaptateur Next.js »**, sans rien en retirer ; les numéros R0–R11 restent valables et sont rattachés aux lots MS4–MS5 (§15).

### 2.4 Défauts relevés
| N° | Défaut | Où [F] | Gravité | Correctif |
|---|---|---|---|---|
| MSE-D01 | Sessions et jetons purgés seulement au démarrage : un serveur qui tourne trois mois garde trois mois de jetons expirés | `ent-api/src/lib.rs:251-256` | moyenne | MS0 : tâche périodique (toutes les heures) |
| MSE-D02 | État `annule` des lots d'import prévu mais jamais posé : un import appliqué par erreur ne se défait pas | `144_import_lot.sql:10-25`, `imports.rs` | élevée (reprise) | MS0 : annulation réelle (objets créés par le lot retirés dans l'ordre inverse, si non modifiés depuis), sinon retrait de l'état |
| MSE-D03 | Aucun parcours de **création** d'un établissement par la plateforme ; l'administrateur « déclare » le sien | `etablissement_repo.rs:133-148` | élevée | MS1 : création par l'opérateur de la plateforme, premier administrateur invité |
| MSE-D04 | Lignes d'import en JSONB, motifs en tableau | `144_import_lot.sql:32-34` | moyenne (D-036) | zone typée (socle 9, migrations 3990/439 ; MS4 pour la reprise) |
| MSE-D05 | `/health` public expose le `git_sha` et la migration : aide un attaquant à cibler une version | `routes.rs:2015-2023` | faible | MS0 : `/health` public = « ok » ; détail sur `/health/detail` réservé à la supervision |
| MSE-D06 | Mot de passe de développement dans un commentaire | `ent-lab/src/main.rs:4` | faible | MS0 (même correctif que RG-D10 du 27) |
| MSE-D07 | Next.js : registre des établissements et **secrets par établissement** dans le stockage objet | `app/lib/tenant-registry.ts:14, 47-50` | élevée (côté Next.js, signalé à Florian) | ScolIA : établissements en table, secrets dans le gestionnaire de secrets, jamais exportés |
| MSE-D08 | Aucune garde « source = copie locale jetable » pour un futur outil de reprise (la garde existe pour les scripts PRA, `pra-lib.sh`) | — | élevée (risque de connexion à la production) | R-MSE10 : `ent-reprise` refuse toute adresse non locale et ouvre la source en lecture seule (T0 §8.2) |

---

## 3. Acteurs, besoins réels, irritants

| Acteur | Ce qu'il doit faire | Ce qui lui fait perdre du temps / ce qu'il redoute | Ce qui le ravirait |
|---|---|---|---|
| **Chef d'établissement** | décider de passer à ScolIA, signer, ouvrir aux familles au bon moment | rater la rentrée ; perdre des données ; former 80 personnes | un calendrier de mise en service clair, des portes explicites, un rapport d'écarts à signer, un retour arrière garanti |
| **Secrétariat** | reprendre élèves, familles, classes ; répondre aux familles qui n'arrivent pas à se connecter | ressaisir ; doublons ; parents qui appellent tous le même jour | une file de rapprochement claire ; activation des comptes par lots ; un tableau « qui n'a pas encore activé » |
| **Direction administrative / comptable** | reprendre les payeurs, soldes, mandats | un solde faux au centime ; un prélèvement en double | totaux comparés au centime, refus listés ; finances en **mode suivi** pendant le doublon (D-034) |
| **CPE, vie scolaire** | appel dès le premier jour | une panne d'avis d'absence le jour de la rentrée (A14) | l'appel testé avant l'ouverture, un plan B papier |
| **Professeurs** | se connecter, faire l'appel, saisir | une formation de deux heures ; un mot de passe de plus | passkey en un geste ; une visite guidée de cinq minutes ; l'aide à côté du bouton |
| **Personnels de l'add-on** (voyages, salles, RH…) | continuer leur travail sans rupture à la bascule | le dossier en cours « perdu » | propriété unique et reprise différentielle (§6.6) |
| **Familles** | activer un compte, trouver ce qu'il faut faire | un courriel d'activation pris pour un hameçonnage ; deux enfants, deux comptes | un message d'activation reconnaissable (sous-domaine de l'établissement, D-012), un compte pour tous ses enfants |
| **Élèves** | se connecter | — | idem |
| **Référent numérique de l'établissement** | relais du support, formation interne | être seul face aux questions | une console de demandes de support, des fiches d'aide, des notes de version lisibles |
| **DPO** | autoriser les données réelles | qu'on commence avant son avis | une porte « données réelles » qu'il voit fermée tant que son avis n'est pas saisi (27) |
| **Florian (opérateur de la plateforme)** | créer l'établissement, lancer les exports, relire les rapports, basculer, supporter | être seul, de nuit, avec des données de production ; se tromper de base | des outils qui refusent la production, des rapports lisibles, un runbook, un support borné |
| **Support ScolIA (plus tard, d'autres personnes)** | aider sans tout voir | accusation d'accès abusif | autorisations explicites, traces visibles, aucun accès permanent |
| **Autre éditeur** (Charlemagne, École Directe, Pronote) | exports | — | formats documentés, rapport des champs non repris |
| **Attaquant** | se faire passer pour le support, détourner une activation de compte, exfiltrer par un export de reprise | — | §9 cas limites |

---

## 4. Ce qui est réglementaire (vérifié le 2026-11-11, voir aussi `27_RGPD_ARCHIVES.md` §4.10)

| Point | Source | Règle | Effet |
|---|---|---|---|
| Données réelles en test | CNIL, guide RGPD du développeur, fiche 11 [V] | les données réelles « ne doivent pas être utilisées » en test ; jeu fictif ; à défaut, mêmes mesures de sécurité qu'en production | démo et recette sur données **fictives** ; répétitions de reprise sur exports **anonymisés** ; l'unique répétition sur export réel se fait en **pré-production fermée** avec les mesures de la production (R-MSE3) |
| Sous-traitant | RGPD art. 28.3 b (confidentialité), e (aide aux droits), g (suppression ou restitution), h (audits) [V] | — | personnels du support tenus à la confidentialité ; aide au 27 ; fin de contrat (§6.14) |
| Sécurité | RGPD art. 32 [V-ext] | mesures appropriées, dont la capacité de rétablir la disponibilité et des tests réguliers | PRA testé (renvoi) ; page d'état |
| Journalisation des personnels techniques | SDET 2024, annexe opérationnelle [V] | tracer les usagers **et** les personnels techniques (identifiant, date, heure, opérations), y compris la **prise de main à distance** ; journaux 12 mois au plus | accès support tracé et visible (§6.10, R-MSE16) |
| Réversibilité | SDET 2024 [V] ; RGPD 28.3 g [V] | clause de réversibilité ; portabilité entre ENT pour faciliter la reprise | export de portabilité (27, PORTABILITE) ; un établissement **arrivant** est repris par le même outil (§6.4) |
| Suppression des comptes | SDET 2024 [V] | suppression 3 mois après la fin du compte | cohérent avec S-545/S-546 du 27 |
| Données de santé | `MISE_EN_SERVICE_DONNEES.md` A7, B1, B9 ; C. santé publ. L1111-8, L1115-1 [V, VERIF R23] | pas de données de santé sans hébergeur HDS | module santé désactivé ; **aucune reprise de santé** tant que le paramètre HDS n'est pas vrai (T0 R-08, R-MSE5) |
| Accord de l'établissement, AIPD | A8, A15 ; RGPD art. 35 [V] | — | porte « données réelles » (§6.2) |
| Avis d'absence dès le premier jour | C. éduc. R131-5 [V, VERIF R1] ; A14 | « toute absence est immédiatement signalée » | test bout en bout de l'avis d'absence **avant** l'ouverture (porte « familles ») et plan de secours (R-MSE7) |
| Facturation électronique (abonnement) | CGI, réforme de la facturation électronique [NV pour les dates d'émission] ; B11 | réception obligatoire depuis le 1-9-2026 pour toutes les entreprises [V-ext, fiche DGFiP, VERIF R38] | l'abonnement ScolIA émet des factures conformes, par une plateforme agréée [NV] ; expert-comptable (Q-MSE7) |
| Accessibilité | loi 2005-102 art. 47, décret 2019-768 [NT] | — | aide et visites guidées accessibles (RGAA, socle 29) |

---

## 5. Concepts et vocabulaire [R]

| Terme | Sens |
|---|---|
| **Établissement** (locataire) | unité de cloisonnement : sous-domaine (D-012), UAI, profil (public, privé sous contrat, hors contrat), fuseau, modules actifs. |
| **Environnement** | `labo` (développement), `demo` (données fictives, réinitialisable), `preprod_fermee` (répétition sur export réel, accès nominatif, aucun envoi), `production`. Un établissement réel n'existe qu'en production ; un établissement de démonstration jamais. |
| **Porte** | étape de la mise en service qui n'ouvre qu'avec ses prérequis : `fictif` → `donnees_reelles` → `ouvert_personnels` → `ouvert_familles` ; et **par module**. |
| **Prérequis** | condition vérifiable (souvent par le système, parfois attestée par une personne avec pièce) : accord écrit signé, DPO désigné, durées validées (27), AIPD validée, sauvegarde restaurée et vérifiée (PRA), test de l'avis d'absence réussi… |
| **Module** | ensemble d'écrans et d'actions d'un socle, activable par établissement ; certains ont des prérequis (santé : HDS). |
| **Source** | système d'origine d'une reprise : `next`, `charlemagne`, `siecle`, `ecole_directe`, `pronote`, `tableur`, `scolia` (réimport d'un export de portabilité). |
| **Adaptateur** | code d'`ent-reprise` qui sait lire une source et la ranger en zone de reprise. |
| **Zone de reprise** | schéma `reprise` de la base cible, tables typées par format de source ; rien n'y est visible des utilisateurs ; vidée après la bascule. |
| **Exécution** | un passage complet de l'outil (simulation ou application), numérotée, chronométrée, avec son rapport. |
| **Correspondance** | ligne `(source, table source, id source) → (table cible, id cible, règle, statut)` ; base de l'idempotence et de l'annulation. |
| **Décision** | arbitrage humain d'un rapprochement incertain, rejoué aux exécutions suivantes. |
| **Rapport de contrôle** | document lisible et fichier machine d'une exécution (T0 §7.2). |
| **Répétition** | exécution complète sur une cible jetable, pour mesurer et corriger. |
| **Bascule** | passage d'un module d'un outil à l'autre, à une date de gel, avec propriété unique et retour arrière. |
| **Synchronisation de doublon** | import périodique depuis l'outil officiel pendant l'année de doublon, sans écraser ce qui a été saisi dans ScolIA (§6.5). |
| **Écart** | différence constatée entre ScolIA et l'outil officiel sur une même réalité (une absence, une note, un effectif). |
| **Autorisation de support** | permission datée, bornée et révocable donnée par un responsable de l'établissement à une personne du support. |

---

## 6. Les modules du socle

### 6.1 Créer un établissement (plateforme)
1. **Qui** : un opérateur de la plateforme (`plateforme.etablissements.creer`, Florian au départ), jamais un utilisateur d'établissement ; deux étapes : *préparer* puis *créer*, chacune journalisée.
2. **Prérequis de création** : contrat ou bon de commande enregistré (§6.13) — ou mention « établissement pilote, sans contrat commercial, convention de l'année de transition » ; **contrat de sous-traitance (art. 28) signé ou en cours** (sans lui, la porte « données réelles » reste fermée, 27 RGP-184).
3. **Saisie** : nom, UAI (contrôle de format ; recherche dans l'annuaire public des établissements par le socle 9 quand il existe), profil (public, privé sous contrat, hors contrat), niveaux, sites, fuseau (défaut `Europe/Paris`, migration 265), sous-domaine (D-012, unicité, liste de mots réservés), langue, premier administrateur (nom, adresse professionnelle).
4. **Effets, en une transaction** : établissement, domaine, rôles modèles (`modele_par_defaut`) dont le rôle **DPO** du 27, catalogue de conservation du profil (27 RGP-130), registre généré à vide (27), modules par défaut (santé **désactivé**, A7), paramètres par défaut (D-023), invitation du premier administrateur (lien court à usage unique, puis **passkey obligatoire** pour ce compte).
5. **Ce que la plateforme ne fait pas** : elle ne voit pas les données de l'établissement après création (cloisonnement, D-013) ; l'opérateur n'a aucun rôle dans l'établissement, sauf autorisation de support (§6.10).
6. **Groupe d'établissements** (OGEC de plusieurs établissements, réseau) : chaque établissement est créé séparément ; un lien « même organisme » permet seulement les tableaux consolidés prévus ailleurs (DPO du 27, réunion du 29), jamais une visibilité croisée des données (D-013).

### 6.2 Paramétrer, et les portes de la mise en service
1. **Assistant de paramétrage** (premier administrateur, puis direction et secrétariat) : une liste ordonnée d'étapes, chacune renvoyant à l'écran du socle propriétaire (rien n'est dupliqué ici) : identité et sites (5), année scolaire et calendrier (5), structure (niveaux, classes, groupes, salles, 5), personnels et rôles (6, 1), import ou reprise des élèves et familles (§6.4), emploi du temps (10), paramètres d'appel et d'avis d'absence (12, 8), modèles de documents (7), conformité (27 : DPO, durées, mentions, AIPD), modules (§6.3), comptes (§6.7).
2. **Tableau « prêt à ouvrir »** : par porte, les prérequis **vérifiés par le système** (vert), **attestés** par une personne avec pièce (bleu : accord écrit signé, avis du DPO) et **manquants** (orange) ; une porte ne s'ouvre que tout vert ou bleu (R-MSE1).
3. **Portes** et prérequis par défaut [R] :

| Porte | Prérequis (extrait) |
|---|---|
| `fictif` (création) | établissement créé ; aucune donnée réelle |
| `donnees_reelles` | accord écrit de l'établissement (A8) ; DPO désigné et avis saisi ; AIPD validée ou décision motivée du DPO (A15) ; durées validées pour les catégories des modules ouverts (27, A11) ; procédures des demandes et des violations publiées (A13, 27 G0) ; base fermée au réseau public, chiffrement au repos, bucket privé, clés au gestionnaire de secrets (A1–A4, **attestés par l'opérateur**) ; restauration testée (A6) ; contrat de sous-traitance signé (B3) ; santé désactivée sauf HDS (A7) |
| `ouvert_personnels` | structure et année prêtes ; comptes des personnels activés à N % (S-561) ; rôles revus ; formation de base proposée ; **avis d'absence testé de bout en bout** sur un compte de test (R-MSE7) |
| `ouvert_familles` | mentions d'information publiées (27) ; responsables repris et rapprochés sans cas « à décider » ouverts au-dessus d'un seuil (S-562) ; message d'activation validé par la direction ; page d'aide famille publiée ; support prêt (référent désigné) |

4. **Fermer une porte** : possible (incident grave) avec motif ; l'ouverture aux familles se suspend sans effacer (les familles voient une page d'information).
5. **Aucun prérequis n'est contournable par l'opérateur** ; une **dérogation** motivée et datée existe seulement pour les prérequis non légaux (ex. pourcentage de comptes activés), décidée par le chef d'établissement (R-MSE2).

### 6.3 Modules : activation et prérequis
- Chaque socle déclare ses **prérequis d'activation** (santé : HDS + AIPD révisée + catégories `sante.*` validées, 27 R-RG26 ; paiement en ligne : prestataire agréé, B7 ; prélèvements : ICS de l'établissement, B8 ; échanges nationaux : accréditation, F-06).
- Activation par module et par établissement, journalisée ; une **désactivation** n'efface rien (lecture seule, ou masquage selon le module) ; elle met le registre du 27 « à revoir ».
- **Régime du module** pendant l'année de doublon : `officiel_scolia` (usage réel : modules de l'add-on, D-035), `doublon` (Charlemagne fait foi), `suivi` (ScolIA calcule sans agir : finances, D-034). Le régime est **affiché** sur chaque écran du module (bandeau « Doublon — Charlemagne fait foi ») pour que personne ne confonde (R-MSE8).

### 6.4 Reprise de données : `ent-reprise`
Le cadre est celui de `TRANSITION_T0_INVENTAIRE.md` §8 ; le 28 l'étend à toutes les sources.

1. **Forme** : binaire Rust en ligne de commande (`ent-rust/crates/ent-reprise`), exécuté par l'opérateur, **jamais** exposé en route web ; compilé seulement avec la fonctionnalité `reprise` (même garde CI que `labo`) [R].
2. **Entrées** : `--source <type>` + chemin d'une **copie locale jetable** (base restaurée depuis un `pg_dump`, fichiers d'export, archive de fichiers) ; refus de toute adresse de base non locale ; ouverture en lecture seule (`default_transaction_read_only`) (R-MSE10) ; `--cible` (jetable en répétition, pré-production fermée, ou production pour l'application finale) ; `--etablissement`, `--annee`, `--modules`, `--decisions` (fichier de décisions rejoué).
3. **Adaptateurs** (un par source et par version de format) :

| Source | Contenu attendu | État de la connaissance | Lot |
|---|---|---|---|
| `next` | `pg_dump` + bucket (liste blanche de préfixes, secrets exclus) | inventaire T0 complet [F] | MS4 |
| `charlemagne` | exports de base élèves, familles et payeurs, notes et bulletins, emploi du temps (F-01 à F-04) | **formats inconnus** tant qu'un export anonymisé n'est pas fourni ; catalogue de 659 tables dans `ent-metier-checklist.md` [F] | MS5 |
| `siecle` / `bee` / `sts` | fichiers XML officiels (F-05, F-07) | par le **socle 9** (import officiel), le 28 l'orchestre | MS5 |
| `ecole_directe`, `pronote` | exports CSV ou XML proposés par l'éditeur [H] | inconnu | MS11 |
| `tableur` | modèles ScolIA téléchargeables (élèves, responsables, personnels, salles) avec dictionnaire | à écrire | MS5 |
| `scolia` | export de portabilité d'un autre ScolIA (`PORTABILITE_EXPORTS.md` §7.4) | format défini | MS11 |

4. **Zone de reprise typée** (D-036, T0 §2) : une table par format de source dans le schéma `reprise` (`reprise_next_travel`, `reprise_charlemagne_eleve`…), colonnes texte brutes **typées par nom** (une colonne par champ attendu), plus `source_ligne`, `execution_id` ; les documents aplatis et JSON de Next.js sont **décodés** en lignes ; une clé inconnue = ligne refusée et listée (T0 R-17) ; l'**instantané brut** de la source est seulement un fichier d'archive chiffré de l'exécution, détruit après la bascule (exception « instantané d'audit » de D-036, déclarée à `MIGRATIONS.md`).
5. **Étapes** : inspection (catalogue réel de la source comparé au format attendu ; arrêt sur écart inconnu) → chargement en zone → référentiels (établissement, sites, années, classes, T0 §6.1–6.3) → personnes (élèves, responsables, personnels, comptes, T0 §6.4–6.6) avec **file humaine** → objets des modules demandés, **par les commandes des socles propriétaires** (règle 1, `source = 'reprise'`) → fichiers (socle 7, empreintes) → contrôles → rapport.
6. **Simulation par défaut** : tout dans une transaction annulée à la fin, rapport complet ; `--appliquer` exige une **confirmation explicite** (saisie du numéro d'exécution simulée relue) et, en production, **deux personnes** (opérateur + responsable de l'établissement, R-MSE11).
7. **Rapprochement** : règles de T0 §6 (clé forte, puis composée, puis humain ; aucun rapprochement approximatif automatique) ; écran de décision côte à côte pour le secrétariat (MSE-150) ; décisions rejouées ; fusion annulable du socle 2 pour un doublon découvert dans le cœur.
8. **Contrôles** (T0 §7.2) : invariant `lues = reprises + archivées + ignorées + refusées + à décider` ; totaux au centime ; échantillons (graine notée) ; fichiers (référencés, trouvés, manquants, orphelins) ; écarts de schéma ; comparaison avec l'exécution précédente ; **inventaire d'intégrité** (`/pra/inventaire`) de l'établissement avant et après.
9. **Rapport** : HTML/PDF lisible sans donnée personnelle dans le résumé, détails nominatifs dans une annexe chiffrée accessible aux seules personnes habilitées ; fichier machine (format d'échange, D-036) ; relu et **signé** par la direction avant l'ouverture du module (R-MSE12).
10. **Annulation** d'une exécution appliquée : objets de l'`execution_id` retirés dans l'ordre inverse **s'ils n'ont pas été modifiés depuis** ; sinon listés pour décision (R-MSE13).
11. **Idempotence** : rejouer = aucun changement ; source plus récente = mises à jour et nouveautés ; une donnée modifiée dans ScolIA depuis la reprise n'est **jamais écrasée** (« modifiée depuis la reprise », signalée).
12. **Ce que l'outil ne fait jamais** (T0 §8.6) : se connecter à la production d'une source ; lire un secret ; écrire dans la source ; reprendre une donnée de santé sans HDS ; créer une fiche du cœur sans décision ; reprendre une empreinte de mot de passe ou un secret de double authentification (T0 §6.7) ; envoyer un message.
13. **Anonymisation** (`ent-reprise anonymiser`, T0 §9) : exécutée **chez Florian** sur la copie réelle ; pseudonymes déterministes par clé HMAC locale détruite après ; liste blanche de colonnes (toute colonne non classée bloque) ; contrôle final « zéro occurrence » des valeurs réelles fréquentes ; santé exclue ; livré pour Next.js d'abord, puis pour chaque format Charlemagne connu.
14. **Établissement qui arrive d'un autre logiciel** (commercialisation) : même outil, même rapport ; le rapport sert de **preuve** à l'établissement que rien n'a été perdu.

### 6.5 Année de doublon : synchronisation et comparateur
1. **Synchronisation** depuis Charlemagne (exports périodiques déposés par le secrétariat, ou SIECLE par le socle 9) : par **domaine** et selon le **régime** (§6.3) ; un domaine en `doublon` reçoit les mises à jour de l'outil officiel (élèves, classes, responsables) **sans écraser** ce qui a été saisi dans ScolIA (conflit → écart) ; fréquence S-563 (hebdomadaire par défaut, quotidienne en septembre).
2. **Comparateur d'écarts** : pour chaque domaine en doublon, comparaison chiffrée ScolIA / outil officiel sur la même période : effectifs par classe, absences par élève et par demi-journée, retards, notes et moyennes par période, sanctions, responsables et coordonnées ; écarts classés (« ScolIA plus complet », « Charlemagne plus complet », « valeurs différentes ») ; chaque écart peut devenir un **retour** (bogue, manque, amélioration) envoyé à l'équipe (§6.9), sans donnée personnelle dans le retour (identifiants opaques).
3. **Tableau du doublon** (direction, Florian) : taux de concordance par domaine et par semaine, retours ouverts et livrés ; c'est la mesure qui permettra de décider la bascule de la rentrée 2028 (D-034).
4. **Ce que le doublon n'est pas** : ScolIA ne renvoie rien à Charlemagne ; il n'envoie aux familles que ce qu'un module en usage réel ou une décision de la direction prévoit (pas de double avis d'absence : un seul outil avise, R-MSE9).

### 6.6 Bascule d'un module de Next.js vers ScolIA
Modèle du socle 42 §32 (propriété unique, R-SV37), généralisé :
1. **Propriété unique** : à tout instant, un objet (séjour, réservation, demande…) n'est modifiable que dans **un** outil ; ScolIA tient une ombre en lecture des objets encore portés par Next.js, mise à jour à chaque reprise.
2. **Date de gel** (vendredi soir de vacances, `TRANSITION_NEXT_RUST.md` §3) : Florian passe le module Next.js en lecture seule (action de Florian, Next.js intouchable par le pilote) ; export final ; reprise différentielle ; rapport relu ; ouverture du module dans ScolIA.
3. **Objets « en vol »** (en cours pendant la bascule) : règle par module, écrite dans sa conception (ex. séjours partant pendant les vacances, 42).
4. **Tableau de bascule** (MSE-170) : modules, régime, date de gel, état (préparé, répété, gelé, repris, ouvert, Next.js arrêté), lien vers les rapports, retour arrière.
5. **Retour arrière** : tant que Next.js n'est pas arrêté, on peut rouvrir Next.js en écriture et refermer le module ScolIA ; les objets créés dans ScolIA entre-temps sont listés pour ressaisie (R-MSE14).
6. **Liens déjà distribués** (certificats, suivi de voyage, T0 R-09) : redirections à prévoir par Florian au moment de la bascule.
7. **Arrêt de Next.js** module par module, puis en T4 : décision et geste de Florian.

### 6.7 Comptes : activation, passkeys, enrôlement
1. **Création** des comptes à partir des personnes reprises (agents, responsables, élèves) **sans secret** (T0 §6.7) ; un compte existant (même personne dans un autre établissement, D-013) est **rattaché**, pas recréé.
2. **Activation par lots** : choix d'une population (personnels d'abord, puis familles par niveau), aperçu du message, **envoi après confirmation** (N3), lien court à usage unique (D-018), sur le sous-domaine de l'établissement, avec un texte qui aide à reconnaître un vrai message (nom de l'établissement, pas de pièce jointe, jamais de demande de mot de passe) (R-MSE15).
3. **Enrôlement de la passkey** à la première connexion (D-009) ; parcours de secours pour qui n'a pas d'appareil compatible (code de récupération, accompagnement au guichet, D-018).
4. **Suivi** : tableau « invités, activés, avec passkey, en échec » par population et par classe ; relance automatique proposée à J+3 et J+10 (S-564), **jamais** plus de deux relances sans action humaine ; liste des adresses en erreur à corriger par le secrétariat.
5. **Guichet d'activation** : le secrétariat peut renvoyer un lien, constater une identité au guichet et activer pour une famille en présentiel (trace).
6. **Familles séparées** : chaque titulaire a son compte et son lien, jamais un lien partagé ; l'adresse protégée n'apparaît dans aucun message (socle 4).

### 6.8 Démonstration et environnements
1. **Établissement de démonstration** « Lycée Démo » (et un collège, un établissement public) : jeu de données **entièrement fictif**, riche (une année complète : élèves, familles séparées, absences, notes, voyages, factures), généré par `seed`/`scenarios` étendus (MS1), réinitialisable en un geste, marqué « démonstration » sur chaque écran.
2. **Isolation** : la démonstration vit dans un environnement `demo` séparé de la production [R] (pas seulement un établissement de plus dans la base de production) : aucun envoi réel (courriels, SMS, notifications bloqués en dur, R-MSE4), comptes de démonstration par rôle, accès par lien de démonstration temporaire.
3. **Démo de décembre 2026** (D-034) : le parcours du quotidien (élève et familles, inscriptions, structure, emploi du temps, appel et absences, notes et bulletins, messagerie, personnels) sur cet établissement de démonstration.
4. **Pré-production fermée** : répétition sur export réel (T2) avec les mesures de la production (CNIL fiche 11), accès nominatif, aucun envoi, détruite après (R-MSE3).
5. **Jamais** de copie de la production vers la démo ou le labo (A10).

### 6.9 Formation, aide contextuelle, nouveautés, retours
1. **Aide contextuelle** : un bouton « ? » sur chaque écran qui ouvre la fiche d'aide de cet écran (contenu versionné **en tables**, rédigé par l'équipe, illustré, court) ; recherche dans l'aide ; aucune donnée personnelle dans l'aide.
2. **Visites guidées** par rôle, de cinq minutes, rejouables, accessibles (RGAA).
3. **Parcours de formation** par rôle (professeur, vie scolaire, secrétariat, direction, famille) : suite de visites et de fiches ; suivi « fait / à faire » **visible de la personne seule** (pas de contrôle de la direction sur la formation de chacun, R-MSE18) ; le référent numérique voit un taux global.
4. **Nouveautés** : notes de version lisibles par rôle, publiées à chaque livraison, tuile « ce qui a changé » (socle 29, POR-103).
5. **Retours** : bouton « signaler un problème ou une idée » depuis chaque écran ; capture du contexte **technique** (écran, version, heure), jamais des données affichées sauf choix explicite ; suivi du retour visible du déclarant.
6. **Assistant d'aide** (IA) : répond sur la documentation, sans accès aux données (§7).

### 6.10 Support : demandes et accès tracé et consenti
1. **Demande de support** : ouverte par le référent numérique ou un responsable (ou par toute personne, selon S-565), avec catégorie, gravité, écran, version ; fil d'échanges ; états (ouverte, prise en charge, en attente de l'établissement, résolue, close) ; délais cibles par gravité (S-566).
2. **Accès du support aux données** : **jamais par défaut**. Une **autorisation de support** est demandée par le support (motif, demande liée) et **accordée** par un responsable de l'établissement (`support.autoriser`, passkey récente) : durée (≤ 4 h par défaut, S-567), **domaines** autorisés (liste), mode (`lecture` par défaut ; `ecriture` seulement sur un geste précis décrit, avec deuxième accord), **sujets** éventuels (un élève, une classe) ; jamais la santé ni le suivi psy et social ; révocable à tout instant.
3. **Pas d'usurpation d'identité** : le support ne se connecte jamais « en tant que » un utilisateur ; il ouvre une **vue support** de l'écran concerné, avec ses propres droits bornés par l'autorisation, et des données **masquées par défaut** (identités remplacées, sauf ce que l'autorisation démasque) (R-MSE16).
4. **Traces** : chaque action du support est tracée (SDET 2024) dans les traces d'accès du 27, avec la référence de l'autorisation ; **visible** des familles comme « Support ScolIA, intervention autorisée par l'établissement le … » (27 §11.2) ; rapport de fin d'autorisation envoyé à l'établissement (ce qui a été vu, fait).
5. **Urgence** (« bris de glace ») : pour une panne bloquante hors heures ouvrées sans responsable joignable, l'opérateur peut ouvrir un accès **technique** (journaux, état des files, pas de données métier) ; tout accès aux données reste soumis à autorisation ; l'usage d'urgence est notifié à la direction au plus tôt et figure au rapport mensuel (R-MSE17).
6. **Personnes du support** : comptes de plateforme nominatifs, passkey, engagement de confidentialité (art. 28.3 b), liste publiée dans le dossier de la plateforme (27).
7. **Opérations de support sur les données** (corriger une donnée, défaire un import) : toujours par une **commande du socle** avec simulation d'abord (R-MSE20) ; jamais de requête SQL directe en production hors procédure PRA à deux personnes.

### 6.11 Supervision, page d'état, incidents
1. **Supervision** : métriques (`ent-obs`), santé (`/ready`), files d'envoi, sauvegardes et restaurations vérifiées (PRA §12–13), alertes à l'opérateur.
2. **Page d'état publique** : hébergée **hors Scaleway** (sinon elle tombe avec le service), états par composant (ENT web, applis mobiles, avis d'absence, courriels, SMS, sauvegardes), incidents en cours et passés, maintenances prévues ; abonnement des référents aux alertes.
3. **Incident** : ouverture, communication aux établissements touchés (bandeau dans l'ENT si accessible, page d'état, courriel aux référents), suivi, clôture, compte rendu ; si l'incident est une **violation**, déclaration au 27 (port `violations::declarer`, RGP-241).
4. **Avis d'absence** : surveillance dédiée (A14) : une panne d'envoi est un incident **le jour même**, avec plan de secours (appel téléphonique par la vie scolaire, liste exportée) (R-MSE7).
5. **PRA** : runbooks (`PRA_SAUVEGARDES.md` §10–§11) accessibles depuis la console de l'opérateur ; exercices tracés ; le résultat du dernier test de restauration est un prérequis de la porte `donnees_reelles` (A6).

### 6.12 Versions, maintenance, communication
1. **Livraisons** fréquentes pendant le doublon (D-034) : fenêtre de maintenance annoncée (S-568 : hors heures de cours, 48 h avant), notes de version, état « maintenance » sur la page d'état.
2. **Migrations** : runbook existant (`migrations-prod-runbook.md`), additives et rejouables (E8) ; une migration destructrice exige décision et sauvegarde vérifiée.
3. **Fonctionnalités en essai** par établissement (un module en avant-première) : activées seulement avec l'accord de la direction ; mentionnées dans les notes de version.

### 6.13 Abonnement et contrat
1. **Contrat** : offre (modules, nombre d'élèves), durée, prix, période d'essai, préavis, **clause de réversibilité**, **contrat de sous-traitance** (art. 28) annexé, niveaux de service, support ; état (`pilote`, `essai`, `actif`, `preavis`, `termine`).
2. **Facturation de l'abonnement** (ScolIA → établissement) : échéancier, factures électroniques conformes [NV : plateforme agréée et calendrier d'émission, Q-MSE7], relances ; **aucun accès** de cette facturation aux données des élèves (seulement les effectifs agrégés si le prix en dépend).
3. **Établissement pilote** : convention de l'année de transition, sans facturation [H, Q-MSE6].

### 6.14 Fin de contrat, départ, fermeture
1. Préavis → l'établissement lance l'**export de portabilité** (double approbation, PORTABILITE §10) ; le 28 en suit l'avancement.
2. À la date convenue : porte fermée aux familles puis aux personnels ; suppression des données (27 RGP-291, deux personnes), **attestation** (RGP-292), archives remises (27 R-RG24).
3. Fermeture ou fusion d'établissements : transfert des archives au repreneur ou au service d'archives (27 CL27-61).

---

## 7. Intelligence artificielle (contrat IA)

### 7.1 Ce que l'IA fait [R]
- **Reprise** : propose des correspondances de libellés (classes « 2nde B » / « Seconde B », motifs d'absence de Charlemagne vers les motifs ScolIA), des **synonymes** relus une fois par l'établissement ; résume un rapport de reprise (« 3 points à regarder ») ; explique un refus.
- **Paramétrage** : propose un paramétrage à partir du profil (rôles, calendrier, catégories) ; repère des incohérences (« aucune personne n'a le droit de faire l'appel en 3ᵉ A »).
- **Aide** : répond aux questions sur l'usage de ScolIA à partir de l'aide et des notes de version (« comment justifier une absence ? »), avec lien vers la fiche.
- **Support** : propose une catégorie et une réponse type pour une demande de support.

### 7.2 Ce que l'IA ne fait pas
Elle ne décide **aucun** rapprochement de personnes (jamais de fusion automatique, T0 §6 principe 2), n'applique aucune reprise, n'ouvre aucune porte, n'accorde aucune autorisation de support, n'envoie aucun message d'activation. L'assistant d'aide **n'a pas accès aux données** de l'établissement ; il ne voit que la documentation (R-MSE19).

### 7.3 Données
Correspondances de libellés : seulement des **libellés de nomenclature** (classes, motifs, matières), jamais de noms de personnes. Pas de donnée personnelle vers un modèle externe sans garanties validées (`CLAUDE.md`, 27 R-RG33).

### 7.4 Capacités déclarées
`reprise.proposer_synonymes` (N1, opérateur), `reprise.resumer_rapport` (N0), `parametrage.incoherences` (N0, direction), `aide.repondre` (N0, tous, sans données), `support.categoriser` (N1, support).

---

## 8. Droits et minimisation (catalogue D-017) [R]

| Action | Libellé | Portée | Modèles par défaut |
|---|---|---|---|
| `plateforme.etablissements.creer` ★ | créer un établissement | plateforme | opérateurs nommés |
| `plateforme.etablissements.gerer` | contrat, modules, portes (côté plateforme) | plateforme | opérateurs |
| `plateforme.reprise.executer` ★ | lancer `ent-reprise` (outil) | plateforme | opérateurs nommés |
| `plateforme.supervision` | tableaux techniques, journaux, page d'état | plateforme | opérateurs |
| `plateforme.support` | traiter les demandes de support | plateforme | personnes du support |
| `mise_en_service.lire` | voir l'assistant, les portes, le tableau de bascule | établissement | Direction, Administrateur, Secrétariat |
| `mise_en_service.gerer` | compléter les étapes, attester un prérequis | établissement | Direction, Administrateur |
| `mise_en_service.ouvrir_porte` ★ | ouvrir ou fermer une porte | établissement | **Chef d'établissement** |
| `modules.activer` ★ | activer ou désactiver un module | établissement | Direction |
| `reprise.decider` | décider les rapprochements incertains | établissement | Secrétariat, Direction |
| `reprise.rapport_signer` ★ | signer un rapport de reprise | établissement | Direction |
| `reprise.appliquer_confirmer` ★ | second accord d'une application en production | établissement | Direction |
| `comptes.activer_lot` | envoyer des invitations par lots | établissement | Secrétariat, Direction |
| `doublon.comparer` | voir le comparateur et le tableau du doublon | établissement | Direction, Secrétariat, CPE (assiduité), professeurs (leurs notes) |
| `support.demander` | ouvrir une demande de support | établissement | Référent numérique, Direction, Secrétariat (S-565) |
| `support.autoriser` ★ | accorder, borner, révoquer une autorisation de support | établissement | Direction, Administrateur |
| `aide.gerer` (plateforme) | rédiger l'aide et les notes de version | plateforme | équipe |
★ : passkey récente, octroi journalisé.

**Minimisation** : la plateforme voit des **compteurs** par établissement (élèves, comptes activés, portes), jamais des données ; le rapport de reprise sépare résumé anonyme et annexe nominative chiffrée ; le support voit des données masquées par défaut ; les retours et demandes de support n'emportent pas de données affichées sans choix explicite.

---

## 9. Cas limites (CL28-01 à CL28-56)

### 9.1 Création, paramétrage, portes
| N° | Cas | Réponse |
|---|---|---|
| CL28-01 | Deux établissements demandent le même sous-domaine | unicité ; suggestion ; mots réservés (`www`, `admin`, `support`, `status`, `api`) |
| CL28-02 | UAI déjà utilisé par un établissement existant | refus ; vérification par l'opérateur (fusion, réouverture) |
| CL28-03 | Établissement avec deux UAI (lycée + lycée professionnel sous un même chef) | deux établissements ou un établissement à deux UAI selon le socle 5 [Q-MSE3] |
| CL28-04 | Premier administrateur sans appareil compatible passkey | parcours de secours D-018 ; porte « données réelles » fermée tant qu'au moins deux responsables n'ont pas de passkey (exports, 27) |
| CL28-05 | Le DPO ne répond pas ; la rentrée approche | porte `donnees_reelles` fermée ; alerte au chef d'établissement ; aucune dérogation (prérequis légal) |
| CL28-06 | L'accord écrit est signé par un adjoint, pas par le chef | prérequis attesté refusé si la personne n'a pas la délégation (socle 6) |
| CL28-07 | Une porte ouverte doit être refermée (fuite, panne) | fermeture motivée ; page d'information ; rien n'est effacé |
| CL28-08 | Le calendrier n'est pas fini mais les familles doivent activer leurs comptes | `ouvert_familles` exige le calendrier ; dérogation possible (non légal) par le chef |
| CL28-09 | Établissement hors contrat (nationalité des personnels, B13) | catégories RH spécifiques ; décision DPO (27) |
| CL28-10 | Établissement public : CA, agent comptable | rôles modèles du public ; finances en mode suivi pendant le doublon |
| CL28-11 | Module santé demandé avant l'HDS | refus d'activation, raison affichée (R-MSE5) |

### 9.2 Reprise
| N° | Cas | Réponse |
|---|---|---|
| CL28-12 | L'opérateur pointe l'outil sur l'adresse de la base de production Next.js | refus au démarrage (adresse non locale, R-MSE10) |
| CL28-13 | Export Next.js sans les tables créées par scripts (`messaging_*`, `portes_ouvertes_*`) | inspection : arrêt et liste (T0 Q-T0-01) |
| CL28-14 | Collection JSON inconnue dans l'export | arrêt (T0 R-01) |
| CL28-15 | Préfixe de secrets présent dans la copie du bucket | refus de l'objet, alerte (T0 R-16) |
| CL28-16 | Deux élèves Next.js pour un élève du cœur | les deux rattachés, rapport (T0 §6.4) |
| CL28-17 | Jumeaux | règle 3 sépare ; vérification humaine |
| CL28-18 | INE identique, date de naissance différente | « à décider » |
| CL28-19 | Montant « 1.350 » | refus si ambigu, liste (T0 R-06) |
| CL28-20 | Date en texte sans fuseau | Europe/Paris explicite ; invalide refusée (T0 R-07) |
| CL28-21 | Donnée de santé dans l'export | exclue tant que HDS faux (R-MSE5) |
| CL28-22 | Format Charlemagne inconnu (première fois) | adaptateur écrit sur l'export **anonymisé** ; aucune lecture de l'export réel avant que l'adaptateur passe ses tests |
| CL28-23 | Charlemagne change de format en cours d'année | inspection : arrêt ; nouvel adaptateur versionné |
| CL28-24 | Reprise appliquée puis découverte d'une erreur de rapprochement | annulation de l'exécution (R-MSE13) ou correction par fusion annulable du socle 2 |
| CL28-25 | Données modifiées dans ScolIA puis nouvelle reprise | jamais écrasées ; signalées « modifiées depuis la reprise » |
| CL28-26 | Responsable sans adresse électronique | compte non créé ; activation au guichet ; liste au secrétariat |
| CL28-27 | Adresse électronique partagée par deux parents | « à décider » (T0 §6.6) ; jamais un compte pour deux personnes |
| CL28-28 | Élève sorti présent dans Next.js, absent du cœur | archive seulement (T0 §6.4) |
| CL28-29 | Reprise trop longue pour un vendredi soir | chronométrage des répétitions ; découpage par module ; bascule reportée plutôt que précipitée |
| CL28-30 | Fichier manquant au stockage (référencé en base) | listé « manquant », jamais silencieux |
| CL28-31 | Rapport de reprise non relu, direction absente | module non ouvert (R-MSE12) |
| CL28-32 | Anonymisation : un nom réel subsiste dans un texte libre | contrôle « zéro occurrence » bloque la livraison (T0 §9.6) |
| CL28-33 | Ancien mot de passe Next.js | non repris ; lien de première connexion (T0 §6.7) |

### 9.3 Doublon et bascule
| N° | Cas | Réponse |
|---|---|---|
| CL28-34 | Une absence saisie dans ScolIA et pas dans Charlemagne | écart « ScolIA plus complet » ; la famille n'est avisée que par l'outil officiel (R-MSE9) |
| CL28-35 | Un élève change de classe dans Charlemagne | synchronisation : changement appliqué, impacts signalés (EDT, groupes) |
| CL28-36 | Conflit : coordonnée modifiée dans les deux outils | écart ; décision du secrétariat ; ScolIA n'écrase jamais sans décision |
| CL28-37 | Professeur qui fait l'appel dans les deux outils et en a assez | régime affiché ; la direction peut décider l'appel ScolIA seul + export vers Charlemagne [Q-MSE4] |
| CL28-38 | Séjour en vol à la bascule | règle du socle 42 (fin dans Next.js, reprise après retour) |
| CL28-39 | Retour arrière après trois jours d'usage de ScolIA | objets créés listés pour ressaisie (R-MSE14) |
| CL28-40 | Liens Next.js déjà envoyés aux familles | redirections (Florian) ; message d'information |

### 9.4 Comptes, support, exploitation
| N° | Cas | Réponse |
|---|---|---|
| CL28-41 | Hameçonnage imitant le message d'activation | texte d'activation qui annonce ce qu'il ne demandera jamais ; domaine de l'établissement ; page d'aide « reconnaître un vrai message » |
| CL28-42 | Un parent active le compte avec le lien reçu par l'autre parent | lien nominatif lié à une adresse ; activation liée à la personne ; vérification du code envoyé à l'adresse |
| CL28-43 | 600 familles activent le jour de la rentrée | lots étalés (S-564), file d'envoi, page d'état |
| CL28-44 | Faux support au téléphone qui demande un code | le support ne demande jamais de code ni de mot de passe (dit dans l'aide et les messages) |
| CL28-45 | Le support demande un accès étendu « pour aller plus vite » | refus ; autorisation bornée seulement (R-MSE16) |
| CL28-46 | Autorisation de support expirée pendant l'intervention | accès coupé ; nouvelle demande |
| CL28-47 | Panne la nuit, aucun responsable joignable | bris de glace technique sans données (R-MSE17) |
| CL28-48 | Le support doit corriger un import erroné | commande d'annulation avec simulation (R-MSE20) |
| CL28-49 | Page d'état hébergée chez Scaleway | refus de conception : hors Scaleway (§6.11) |
| CL28-50 | Panne d'envoi des avis d'absence à 8 h 15 | incident immédiat, plan de secours (R-MSE7) |
| CL28-51 | Maintenance pendant un conseil de classe | fenêtres hors heures de cours, calendrier de l'établissement consulté (S-568) |
| CL28-52 | Établissement de démonstration montré à un client avec un vrai nom d'élève | impossible : démo générée, jamais copiée (A10) |
| CL28-53 | Personne qui veut la formation sans que la direction le sache | suivi personnel (R-MSE18) |
| CL28-54 | Fin de contrat : l'établissement n'a pas fait son export | rappels ; export proposé ; aucune suppression avant la date convenue + délai (27) |
| CL28-55 | Établissement pilote qui devient client payant | changement d'état du contrat, sans reprise |
| CL28-56 | Démo réinitialisée pendant qu'un prospect l'utilise | réinitialisation planifiée, annoncée ; démo personnelle par prospect possible |

---

## 10. Actions métier (MSE-100 à MSE-279)

Acteurs : OPE opérateur de la plateforme, SUP support, DIR direction, CHE chef d'établissement, ADM administrateur, ADF secrétariat, CPT comptabilité, REF référent numérique, DPO, PER personnel, PAR responsable, ELV élève, PRO prospect, SYS système.

### 10.1 Établissement (MSE-100 à MSE-119)
| N° | Action | Acteur · droit | Règles | Événement | Lot |
|---|---|---|---|---|---|
| MSE-100 | Préparer la création d'un établissement | OPE · `plateforme.etablissements.creer` | §6.1 | — | MS1 |
| MSE-101 | Vérifier le sous-domaine (unicité, mots réservés) | SYS | CL28-01 | — | MS1 |
| MSE-102 | Vérifier l'UAI (format, unicité, annuaire si disponible) | SYS | socle 9 | — | MS1 |
| MSE-103 | Créer l'établissement (transaction unique) ★ | OPE | rôles modèles, catalogue 27, modules par défaut, santé désactivée | `etablissement.cree` | MS1 |
| MSE-104 | Inviter le premier administrateur | SYS | lien court ; passkey obligatoire | `compte.invite` | MS1 |
| MSE-105 | Renvoyer ou remplacer l'invitation du premier administrateur | OPE | ancien lien révoqué | — | MS1 |
| MSE-106 | Lier des établissements d'un même organisme | OPE | aucune visibilité croisée | `etablissement.organisme_lie` | MS12 |
| MSE-107 | Modifier l'identité plateforme (nom, sous-domaine, profil) | OPE | sous-domaine : redirection de l'ancien | `etablissement.modifie` | MS1 |
| MSE-108 | Changer le profil (privé sous contrat → public, etc.) | OPE, DIR | catalogue de conservation proposé à nouveau (27) | `etablissement.profil_change` | MS1 |
| MSE-109 | Voir la liste des établissements (compteurs seulement) | OPE | minimisation | — | MS1 |
| MSE-110 | Suspendre un établissement (impayé grave, demande, incident) ★ | OPE + DIR | lecture seule ; jamais de suppression | `etablissement.suspendu` | MS12 |
| MSE-111 | Réactiver | OPE | — | `etablissement.reactive` | MS12 |
| MSE-112 | Déclarer la fin de contrat | OPE | §6.14 | `etablissement.contrat_termine` | MS12 |
| MSE-113 | Supprimer les données à la date convenue (avec le 27) ★ | OPE + DIR | 27 RGP-291 | `etablissement.donnees_supprimees` | MS12 |
| MSE-114 | Créer un établissement de démonstration | OPE | environnement `demo` | `etablissement.demo_cree` | MS1 |
| MSE-115 | Distinguer la déclaration locale (identité, fiche) de la création plateforme | SYS | MSE-D03 | — | MS0 |
| MSE-116 | Purger périodiquement sessions et jetons | SYS | MSE-D01 | — | MS0 |
| MSE-117 | Réduire `/health` public ; détail réservé à la supervision | SYS | MSE-D05 | — | MS0 |
| MSE-118 | Annuler réellement un lot d'import (ou retirer l'état) | ADF · `imports.appliquer` | MSE-D02 | `imports.lot_annule` | MS0 |
| MSE-119 | Étendre la garde « jamais de production » à `ent-reprise` | SYS (CI) | MSE-D08 | — | MS0 |

### 10.2 Paramétrage et portes (MSE-120 à MSE-139)
| N° | Action | Acteur · droit | Règles | Événement | Lot |
|---|---|---|---|---|---|
| MSE-120 | Voir l'assistant de paramétrage et l'état d'avancement | ADM, DIR · `mise_en_service.lire` | étapes renvoyant aux socles | — | MS2 |
| MSE-121 | Marquer une étape faite (quand le système ne peut pas le vérifier) | ADM, DIR | attestation tracée | — | MS2 |
| MSE-122 | Voir le tableau « prêt à ouvrir » par porte | DIR | vérifié / attesté / manquant | — | MS2 |
| MSE-123 | Attester un prérequis avec pièce (accord écrit, avis du DPO) | DIR, DPO · `mise_en_service.gerer` | pièce au socle 7 | `mise_en_service.prerequis_atteste` | MS2 |
| MSE-124 | Attester les mesures d'infrastructure (A1–A4, A6) | OPE | par établissement ou plateforme | `mise_en_service.prerequis_atteste` | MS2 |
| MSE-125 | Ouvrir la porte `donnees_reelles` ★ | CHE · `mise_en_service.ouvrir_porte` | R-MSE1 ; aucune dérogation | `mise_en_service.porte_ouverte` | MS2 |
| MSE-126 | Ouvrir la porte `ouvert_personnels` ★ | CHE | test avis d'absence (R-MSE7) | `mise_en_service.porte_ouverte` | MS2 |
| MSE-127 | Ouvrir la porte `ouvert_familles` ★ | CHE | mentions publiées (27) | `mise_en_service.porte_ouverte` | MS2 |
| MSE-128 | Fermer une porte avec motif ★ | CHE | page d'information | `mise_en_service.porte_fermee` | MS2 |
| MSE-129 | Déroger à un prérequis non légal ★ | CHE | R-MSE2 | `mise_en_service.derogation` | MS2 |
| MSE-130 | Proposer un paramétrage initial par profil | SYS, IA | N1 | — | MS2 |
| MSE-131 | Repérer les incohérences de paramétrage | SYS, IA | N0 | — | MS10 |
| MSE-132 | Tester l'avis d'absence de bout en bout (compte de test) | ADF, CPE | trace du test | `mise_en_service.test_avis` | MS2 |
| MSE-133 | Plan de secours de l'avis d'absence (liste à appeler, export) | CPE | — | — | MS2 |
| MSE-134 | Calendrier de mise en service (jalons, dates des portes) | DIR | — | — | MS2 |
| MSE-135 | Valider le message d'activation des familles | DIR | modèle relu | — | MS6 |
| MSE-136 | Désigner le référent numérique | DIR | — | — | MS2 |
| MSE-137 | Désigner le référent support et ses suppléants | DIR | — | — | MS8 |
| MSE-138 | Voir l'historique des portes et dérogations | DIR, OPE | — | — | MS2 |
| MSE-139 | Exporter le dossier de mise en service (prérequis, pièces, dates) | DIR | preuve | — | MS2 |

### 10.3 Modules (MSE-140 à MSE-149)
| N° | Action | Acteur · droit | Règles | Événement | Lot |
|---|---|---|---|---|---|
| MSE-140 | Voir les modules, leur régime et leurs prérequis | DIR | — | — | MS3 |
| MSE-141 | Activer un module ★ | DIR · `modules.activer` | prérequis du socle | `module.active` | MS3 |
| MSE-142 | Désactiver un module ★ | DIR | lecture seule ; registre « à revoir » | `module.desactive` | MS3 |
| MSE-143 | Fixer le régime d'un module (officiel, doublon, suivi) | DIR | bandeau (R-MSE8) | `module.regime_change` | MS3 |
| MSE-144 | Bloquer la santé tant que HDS faux | SYS | R-MSE5 | — | MS3 |
| MSE-145 | Activer une fonctionnalité en essai | DIR | notes de version | — | MS3 |
| MSE-146 | Alerte : module actif sans prérequis encore valide (accord expiré) | SYS | — | — | MS3 |
| MSE-147 | Afficher le régime sur chaque écran du module | SYS | R-MSE8 | — | MS3 |
| MSE-148 | Journal des activations | DIR, OPE | — | — | MS3 |
| MSE-149 | Prérequis d'activation déclarés par chaque socle (kit) | SYS (CI) | test : tout module a ses prérequis | — | MS3 |

### 10.4 Reprise (MSE-150 à MSE-169)
| N° | Action | Acteur · droit | Règles | Événement | Lot |
|---|---|---|---|---|---|
| MSE-150 | Décider un rapprochement incertain, côte à côte | ADF · `reprise.decider` | décisions rejouées | `reprise.decision` | MS4 |
| MSE-151 | Inspecter une source (catalogue, comptages, format) | OPE · `plateforme.reprise.executer` | arrêt sur écart inconnu | `reprise.inspection` | MS4 |
| MSE-152 | Charger en zone de reprise | OPE | décodage strict (D-036) | — | MS4 |
| MSE-153 | Reprendre les référentiels (sites, années, classes) | OPE | T0 §6.1–6.3 | — | MS4 |
| MSE-154 | Rapprocher les personnes (élèves, responsables, personnels, comptes) | OPE | T0 §6.4–6.6 | — | MS4 |
| MSE-155 | Valider une table de synonymes (classes, motifs) | ADF, DIR | relue une fois | `reprise.synonymes_valides` | MS4 |
| MSE-156 | Reprendre un module (commandes du socle) | OPE | `source = 'reprise'` | — | MS4 |
| MSE-157 | Reprendre les fichiers (socle 7, empreintes) | OPE | — | — | MS4 |
| MSE-158 | Simuler (défaut) | OPE | transaction annulée | `reprise.simulee` | MS4 |
| MSE-159 | Appliquer ★ | OPE + DIR · `reprise.appliquer_confirmer` | R-MSE11 | `reprise.appliquee` | MS4 |
| MSE-160 | Rejouer (idempotence) ; reprise différentielle | OPE | — | — | MS4 |
| MSE-161 | Répéter sur cible jetable, chronométrer | OPE | — | `reprise.repetee` | MS4 |
| MSE-162 | Comparer à l'exécution précédente | OPE, DIR | — | — | MS4 |
| MSE-163 | Adaptateur `next` | équipe | T0 | — | MS4 |
| MSE-164 | Adaptateur `charlemagne` (par format) | équipe | export anonymisé d'abord (CL28-22) | — | MS5 |
| MSE-165 | Orchestrer l'import officiel SIECLE/BEE/STS (socle 9) | OPE, ADF | socle 9 | — | MS5 |
| MSE-166 | Modèles de tableurs ScolIA et leur dictionnaire | équipe | — | — | MS5 |
| MSE-167 | Import par tableur (secrétariat) avec prévisualisation | ADF · `imports.appliquer` | même cadre | — | MS5 |
| MSE-168 | Adaptateurs `ecole_directe`, `pronote` | équipe | formats à obtenir | — | MS11 |
| MSE-169 | Réimporter un export de portabilité ScolIA | OPE | PORTABILITE §7.4 | — | MS11 |

### 10.5 Bascule (MSE-170 à MSE-179)
| N° | Action | Acteur · droit | Règles | Événement | Lot |
|---|---|---|---|---|---|
| MSE-170 | Tableau de bascule par module | DIR, OPE | §6.6 | — | MS4 |
| MSE-171 | Planifier la date de gel | DIR, OPE | vacances (T1–T4) | `bascule.planifiee` | MS4 |
| MSE-172 | Constater le gel du module Next.js (action de Florian) | OPE | attestation | `bascule.gel` | MS4 |
| MSE-173 | Tenir l'ombre en lecture des objets encore dans Next.js | SYS | propriété unique | — | MS4 |
| MSE-174 | Appliquer la règle des objets « en vol » du module | OPE | conception du socle | — | MS4 |
| MSE-175 | Ouvrir le module dans ScolIA | DIR | rapport signé (R-MSE12) | `bascule.ouverte` | MS4 |
| MSE-176 | Retour arrière | DIR + OPE | R-MSE14 | `bascule.retour_arriere` | MS4 |
| MSE-177 | Lister les redirections de liens à faire | OPE | T0 R-09 | — | MS4 |
| MSE-178 | Constater l'arrêt du module Next.js (action de Florian) | OPE | — | `bascule.next_arrete` | MS4 |
| MSE-179 | Vider la zone de reprise et détruire les copies de la source | OPE | date notée au rapport (T0 R-12) | `reprise.zone_videe` | MS4 |

### 10.6 Rapports et annulation (MSE-180 à MSE-189)
| N° | Action | Acteur · droit | Règles | Événement | Lot |
|---|---|---|---|---|---|
| MSE-180 | Produire le rapport (lisible + machine) | SYS | T0 §7.2 | — | MS4 |
| MSE-181 | Voir le résumé (sans données personnelles) | DIR, OPE | — | — | MS4 |
| MSE-182 | Ouvrir l'annexe nominative (chiffrée) | DIR, ADF habilités | tracé | — | MS4 |
| MSE-183 | Relire les échantillons côte à côte | DIR, ADF | graine notée | — | MS4 |
| MSE-184 | Signer le rapport ★ | DIR · `reprise.rapport_signer` | R-MSE12 | `reprise.rapport_signe` | MS4 |
| MSE-185 | Annuler une exécution appliquée ★ | OPE + DIR | R-MSE13 | `reprise.annulee` | MS4 |
| MSE-186 | Lister les objets « modifiés depuis la reprise » | OPE | — | — | MS4 |
| MSE-187 | Inventaire d'intégrité avant / après | SYS | `/pra/inventaire` | — | MS4 |
| MSE-188 | Résumer le rapport par IA | SYS | N0 | — | MS10 |
| MSE-189 | Archiver le rapport (preuve) | SYS | socle 7 ; catégorie de conservation (27) | — | MS4 |

### 10.7 Anonymisation (MSE-190 à MSE-199)
| N° | Action | Acteur · droit | Règles | Événement | Lot |
|---|---|---|---|---|---|
| MSE-190 | Livrer l'outil d'anonymisation à Florian | équipe | T0 §9 | — | MS4 |
| MSE-191 | Générer la clé HMAC locale, la détruire après | SYS (machine de Florian) | jamais transmise | — | MS4 |
| MSE-192 | Remplacer identités, coordonnées, identifiants, IBAN | SYS | formats valides fictifs | — | MS4 |
| MSE-193 | Vider les textes libres à risque, remplacer les images | SYS | même longueur | — | MS4 |
| MSE-194 | Remplacer les fichiers par des fichiers factices de même taille et type | SYS | — | — | MS4 |
| MSE-195 | Contrôle « zéro occurrence » | SYS | bloque la livraison | — | MS4 |
| MSE-196 | Bloquer sur une colonne non classée (liste blanche) | SYS | — | — | MS4 |
| MSE-197 | Exclure la santé | SYS | — | — | MS4 |
| MSE-198 | Rapport d'anonymisation (colonnes traitées) | SYS | — | — | MS4 |
| MSE-199 | Anonymiser un export Charlemagne (par format) | SYS | après MSE-164 | — | MS5 |

### 10.8 Doublon (MSE-200 à MSE-209)
| N° | Action | Acteur · droit | Règles | Événement | Lot |
|---|---|---|---|---|---|
| MSE-200 | Déposer un export périodique de l'outil officiel | ADF | — | `doublon.depot` | MS7 |
| MSE-201 | Synchroniser un domaine en doublon | SYS | jamais d'écrasement sans décision | `doublon.synchronise` | MS7 |
| MSE-202 | Traiter un conflit de synchronisation | ADF | — | — | MS7 |
| MSE-203 | Comparer ScolIA et l'outil officiel (par domaine, période) | DIR, ADF, CPE · `doublon.comparer` | §6.5 | — | MS7 |
| MSE-204 | Transformer un écart en retour (bogue, manque, idée) | DIR, ADF, CPE, PER | identifiants opaques | `retour.cree` | MS7 |
| MSE-205 | Tableau du doublon (concordance par semaine) | DIR, OPE | — | — | MS7 |
| MSE-206 | Professeur : comparer ses notes avec Charlemagne | PER (ses classes) | — | — | MS7 |
| MSE-207 | Garantir qu'un seul outil avise les familles par domaine | SYS | R-MSE9 | — | MS7 |
| MSE-208 | Bilan de fin d'année du doublon (aide à la décision de bascule 2028) | DIR, OPE | — | — | MS7 |
| MSE-209 | Exporter vers l'outil officiel ce qu'il accepte (si la direction le décide) | ADF | Q-MSE4 | — | MS11 |

### 10.9 Comptes (MSE-210 à MSE-224)
| N° | Action | Acteur · droit | Règles | Événement | Lot |
|---|---|---|---|---|---|
| MSE-210 | Créer les comptes des personnes reprises, sans secret | SYS | rattacher un compte existant (D-013) | `compte.cree` | MS6 |
| MSE-211 | Choisir une population et prévisualiser l'invitation | ADF · `comptes.activer_lot` | — | — | MS6 |
| MSE-212 | Envoyer un lot d'invitations ★ | ADF | N3 ; étalement | `compte.invitations_envoyees` | MS6 |
| MSE-213 | Activer le compte et enrôler la passkey | PER, PAR, ELV | D-009 | `compte.active` | MS6 |
| MSE-214 | Parcours de secours sans passkey | PAR | D-018 | — | MS6 |
| MSE-215 | Suivre l'activation par population et par classe | ADF, DIR | — | — | MS6 |
| MSE-216 | Relancer (J+3, J+10) | SYS | deux relances au plus | — | MS6 |
| MSE-217 | Corriger une adresse en erreur | ADF | — | — | MS6 |
| MSE-218 | Activer au guichet après constat d'identité | ADF | trace | `compte.active` | MS6 |
| MSE-219 | Renvoyer un lien | ADF | ancien révoqué | — | MS6 |
| MSE-220 | Lier les enfants d'une famille dans plusieurs établissements à un même compte | SYS | D-013 | — | MS6 |
| MSE-221 | Page d'aide « reconnaître un vrai message » | équipe | CL28-41 | — | MS6 |
| MSE-222 | Rôles Next.js → rôles paramétrables (table relue) | OPE, DIR | T0 §6.6 | — | MS4 |
| MSE-223 | Désactiver les comptes non activés en fin de campagne | SYS | S-569 | — | MS6 |
| MSE-224 | Statistiques d'enrôlement des passkeys (plateforme) | OPE | compteurs | — | MS6 |

### 10.10 Démonstration et environnements (MSE-225 à MSE-234)
| N° | Action | Acteur · droit | Règles | Événement | Lot |
|---|---|---|---|---|---|
| MSE-225 | Générer le jeu de démonstration complet | SYS | fictif | — | MS1 |
| MSE-226 | Réinitialiser la démonstration | OPE | annoncée | `demo.reinitialisee` | MS1 |
| MSE-227 | Ouvrir une démo personnelle pour un prospect (lien temporaire) | OPE | — | — | MS1 |
| MSE-228 | Entrer dans la démo par rôle | PRO | comptes de démo | — | MS1 |
| MSE-229 | Bloquer tout envoi réel en démo | SYS | R-MSE4 | — | MS1 |
| MSE-230 | Marquer « démonstration » chaque écran | SYS | — | — | MS1 |
| MSE-231 | Créer une pré-production fermée pour une répétition | OPE | R-MSE3 | — | MS4 |
| MSE-232 | Détruire la pré-production après la répétition | OPE | trace | — | MS4 |
| MSE-233 | Refuser toute copie de la production vers démo ou labo | SYS | A10 | — | MS0 |
| MSE-234 | Scénarios de recette (personas) sur la démo | équipe | — | — | MS1 |

### 10.11 Aide, formation, retours (MSE-235 à MSE-249)
| N° | Action | Acteur · droit | Règles | Événement | Lot |
|---|---|---|---|---|---|
| MSE-235 | Ouvrir l'aide de l'écran | tous | — | — | MS10 |
| MSE-236 | Chercher dans l'aide | tous | — | — | MS10 |
| MSE-237 | Rédiger et versionner une fiche d'aide | équipe · `aide.gerer` | — | — | MS10 |
| MSE-238 | Visite guidée par rôle | tous | accessible | — | MS10 |
| MSE-239 | Parcours de formation et suivi personnel | tous | R-MSE18 | — | MS10 |
| MSE-240 | Taux global de formation (sans nominatif) | REF | — | — | MS10 |
| MSE-241 | Publier les notes de version par rôle | équipe | tuile 29 | `version.publiee` | MS9 |
| MSE-242 | Signaler un problème ou une idée depuis l'écran | tous | contexte technique seul | `retour.cree` | MS10 |
| MSE-243 | Suivre ses retours | tous | — | — | MS10 |
| MSE-244 | Trier les retours (équipe) | équipe | — | — | MS10 |
| MSE-245 | Assistant d'aide (sans données) | tous | R-MSE19 | — | MS10 |
| MSE-246 | Fiches imprimables pour la formation en salle des professeurs | REF | — | — | MS10 |
| MSE-247 | Aide pour les familles (activer, justifier, payer) | PAR | — | — | MS6 |
| MSE-248 | Aide pour les élèves | ELV | langage adapté | — | MS10 |
| MSE-249 | Lien d'aide dans chaque message d'activation et chaque avis | SYS | — | — | MS6 |

### 10.12 Support (MSE-250 à MSE-264)
| N° | Action | Acteur · droit | Règles | Événement | Lot |
|---|---|---|---|---|---|
| MSE-250 | Ouvrir une demande de support | REF, DIR · `support.demander` | — | `support.demande_ouverte` | MS8 |
| MSE-251 | Échanger sur la demande | REF, SUP | — | — | MS8 |
| MSE-252 | Prendre en charge, classer, résoudre | SUP · `plateforme.support` | délais S-566 | `support.demande_resolue` | MS8 |
| MSE-253 | Demander une autorisation d'accès (motif, domaines, durée, mode) | SUP | — | `support.autorisation_demandee` | MS8 |
| MSE-254 | Accorder ou refuser une autorisation ★ | DIR, ADM · `support.autoriser` | bornes S-567 ; jamais santé, psy | `support.autorisation_accordee` | MS8 |
| MSE-255 | Second accord pour l'écriture | DIR | geste décrit | — | MS8 |
| MSE-256 | Révoquer une autorisation | DIR, ADM | immédiat | `support.autorisation_revoquee` | MS8 |
| MSE-257 | Ouvrir la vue support (masquée par défaut) | SUP | R-MSE16 | — | MS8 |
| MSE-258 | Démasquer dans les bornes de l'autorisation | SUP | tracé | — | MS8 |
| MSE-259 | Rapport de fin d'autorisation à l'établissement | SYS | — | — | MS8 |
| MSE-260 | Bris de glace technique (sans données métier) | OPE | R-MSE17 ; notifié | `support.urgence` | MS8 |
| MSE-261 | Corriger par une commande du socle avec simulation | SUP (autorisé) | R-MSE20 | — | MS8 |
| MSE-262 | Rapport mensuel du support (demandes, accès, urgences) | DIR, OPE | — | — | MS8 |
| MSE-263 | Proposer une catégorie et une réponse type (IA) | SUP | N1 | — | MS10 |
| MSE-264 | Publier la liste des personnes du support | OPE | dossier de la plateforme (27) | — | MS8 |

### 10.13 Exploitation (MSE-265 à MSE-272)
| N° | Action | Acteur · droit | Règles | Événement | Lot |
|---|---|---|---|---|---|
| MSE-265 | Page d'état publique hors Scaleway | OPE | §6.11 | — | MS9 |
| MSE-266 | Ouvrir, suivre, clore un incident ; communiquer | OPE | violation → 27 | `incident.ouvert` | MS9 |
| MSE-267 | Annoncer une maintenance | OPE | S-568 | `maintenance.annoncee` | MS9 |
| MSE-268 | S'abonner aux alertes de la page d'état | REF, DIR | — | — | MS9 |
| MSE-269 | Surveiller les avis d'absence (incident le jour même) | SYS | R-MSE7 | `incident.ouvert` | MS9 |
| MSE-270 | Accéder aux runbooks PRA depuis la console | OPE | — | — | MS9 |
| MSE-271 | Tracer un exercice PRA et son résultat | OPE | prérequis A6 | `pra.exercice` | MS9 |
| MSE-272 | Tableau de supervision (métriques, files, sauvegardes) | OPE | — | — | MS9 |

### 10.14 Contrat et abonnement (MSE-273 à MSE-279)
| N° | Action | Acteur · droit | Règles | Événement | Lot |
|---|---|---|---|---|---|
| MSE-273 | Enregistrer le contrat (offre, durée, prix, préavis, réversibilité) | OPE · `plateforme.etablissements.gerer` | — | `contrat.enregistre` | MS12 |
| MSE-274 | Joindre le contrat de sous-traitance (27 RGP-184) | OPE, DIR | prérequis `donnees_reelles` | — | MS12 |
| MSE-275 | Émettre les factures de l'abonnement | SYS | facturation électronique [NV] | `contrat.facture` | MS13 |
| MSE-276 | Relancer une facture d'abonnement | SYS | — | — | MS13 |
| MSE-277 | Passer en préavis | OPE, DIR | export proposé | `contrat.preavis` | MS12 |
| MSE-278 | Voir son contrat et ses factures (établissement) | DIR, CPT | — | — | MS12 |
| MSE-279 | Convention de l'établissement pilote | OPE | Q-MSE6 | — | MS12 |

---

## 11. Règles transverses (R-MSE1 à R-MSE36) [R]

| N° | Règle |
|---|---|
| R-MSE1 | Une porte ne s'ouvre que lorsque tous ses prérequis sont vérifiés par le système ou attestés avec pièce. |
| R-MSE2 | Seuls les prérequis non légaux admettent une dérogation, motivée et datée, décidée par le chef d'établissement ; jamais par l'opérateur. |
| R-MSE3 | Une répétition sur export réel se fait en pré-production fermée, avec les mesures de la production, sans envoi, détruite après. |
| R-MSE4 | Démonstration et labo : aucun envoi réel (courriel, SMS, notification) possible, bloqué en dur. |
| R-MSE5 | Aucune donnée de santé n'est reprise ni activée tant que l'hébergement HDS n'est pas attesté. |
| R-MSE6 | Aucune donnée réelle n'entre dans le labo ou la démonstration ; aucune copie de la production vers eux. |
| R-MSE7 | L'avis d'absence est testé de bout en bout avant l'ouverture aux personnels ; sa panne est un incident le jour même, avec plan de secours. |
| R-MSE8 | Le régime d'un module (officiel, doublon, suivi) est affiché sur chacun de ses écrans. |
| R-MSE9 | Pour un domaine donné, un seul outil avise les familles. |
| R-MSE10 | `ent-reprise` n'accepte qu'une source locale jetable, ouverte en lecture seule ; il refuse toute adresse non locale. |
| R-MSE11 | Une application de reprise en production exige l'opérateur et un responsable de l'établissement. |
| R-MSE12 | Un module repris ne s'ouvre qu'après signature du rapport de reprise par la direction. |
| R-MSE13 | Une exécution s'annule en retirant ses objets non modifiés depuis ; les autres sont listés pour décision. |
| R-MSE14 | Un retour arrière liste les objets créés dans ScolIA depuis la bascule ; rien n'est perdu en silence. |
| R-MSE15 | Le message d'activation ne demande jamais de mot de passe ni de code, vient du sous-domaine de l'établissement et renvoie à l'aide. |
| R-MSE16 | Le support n'usurpe jamais un utilisateur : vue support bornée par une autorisation, données masquées par défaut, tout tracé et visible de l'établissement et des familles. |
| R-MSE17 | Le bris de glace donne un accès technique sans données métier ; il est notifié à la direction et rapporté. |
| R-MSE18 | Le suivi de formation est personnel ; la direction ne voit que des taux globaux. |
| R-MSE19 | L'assistant d'aide n'accède à aucune donnée d'établissement. |
| R-MSE20 | Toute opération du support sur des données passe par une commande du socle, simulée d'abord. |
| R-MSE21 | La reprise écrit par les commandes des socles propriétaires, jamais par insertion directe dans leurs tables. |
| R-MSE22 | Aucun rapprochement de personnes n'est appliqué sans règle forte ou décision humaine. |
| R-MSE23 | Toute clé, colonne ou collection inconnue d'une source est refusée et listée, jamais gardée en vrac. |
| R-MSE24 | Les empreintes de mots de passe et secrets de double authentification ne sont jamais repris. |
| R-MSE25 | Un rapport de reprise sépare un résumé sans donnée personnelle et une annexe nominative chiffrée. |
| R-MSE26 | Les copies de la source sont détruites après la bascule, à une date écrite au rapport. |
| R-MSE27 | La synchronisation du doublon n'écrase jamais une saisie ScolIA sans décision. |
| R-MSE28 | La plateforme ne voit que des compteurs par établissement. |
| R-MSE29 | La création d'un établissement charge rôles modèles, catalogue de conservation et modules par défaut, santé désactivée, en une transaction. |
| R-MSE30 | Le premier administrateur et tout détenteur d'une action sensible ont une passkey. |
| R-MSE31 | La page d'état est hébergée hors de l'hébergeur de production. |
| R-MSE32 | Les fenêtres de maintenance évitent les heures de cours et sont annoncées 48 h avant. |
| R-MSE33 | Les secrets des établissements vivent dans le gestionnaire de secrets, jamais en base ni en stockage objet, et ne sont jamais exportés. |
| R-MSE34 | Les outils `reprise` et `labo` sont absents du binaire de production (CI). |
| R-MSE35 | Une désactivation de module n'efface aucune donnée. |
| R-MSE36 | Toute transition d'état du 28 (porte, module, exécution, autorisation, contrat) écrit le journal dans la même transaction. |

---

## 12. Événements et notifications [R]

**Émis** : `etablissement.cree`, `etablissement.modifie`, `etablissement.profil_change`, `etablissement.suspendu`, `etablissement.reactive`, `etablissement.contrat_termine` (consommé par le 27), `etablissement.donnees_supprimees`, `etablissement.demo_cree`, `mise_en_service.prerequis_atteste`, `mise_en_service.porte_ouverte`, `mise_en_service.porte_fermee`, `mise_en_service.derogation`, `mise_en_service.test_avis`, `module.active`, `module.desactive` (consommés par le 27 : registre « à revoir »), `module.regime_change`, `reprise.inspection`, `reprise.simulee`, `reprise.appliquee`, `reprise.repetee`, `reprise.annulee`, `reprise.rapport_signe`, `reprise.decision`, `reprise.synonymes_valides`, `reprise.zone_videe`, `bascule.*`, `doublon.depot`, `doublon.synchronise`, `retour.cree`, `compte.invite`, `compte.invitations_envoyees`, `compte.active`, `support.*`, `incident.ouvert`, `maintenance.annoncee`, `version.publiee`, `pra.exercice`, `contrat.*`. Identifiants seulement (E14).

**Natures déclarées au socle 8** : `mise_en_service.porte_a_ouvrir` (chef d'établissement), `mise_en_service.prerequis_manquant`, `reprise.rapport_a_signer`, `reprise.decisions_en_attente` (secrétariat), `compte.invitation` (personne invitée), `compte.relance_activation`, `support.autorisation_a_accorder` (direction : **urgent**), `support.intervention_terminee`, `support.urgence_utilisee` (direction), `incident.service` (référents abonnés), `maintenance.prevue`, `version.nouveautes` (tuile 29, pas de courriel par défaut), `contrat.preavis`.

---

## 13. Modèle de données cible (brique `mise_en_service`, D-036, E1–E14) [R]

Mêmes règles que le 27 (§17) : UUID applicatifs, `etablissement_id NOT NULL` sauf tables de plateforme **P**, FK composites, horodatages, RLS, **aucun JSON ni tableau**, fichiers au socle 7, journal en transaction. Préfixes `mes_` (mise en service), `rep_` (reprise, schéma `public` pour les tables de pilotage ; schéma `reprise` pour la zone), `sup_` (support), `plt_` (plateforme).

| Table | Colonnes principales |
|---|---|
| `plt_etablissement_contrat` **P** | `etablissement_id`, `etat` (`pilote`/`essai`/`actif`/`preavis`/`termine`/`suspendu`), `offre_code`, `debut_le`, `fin_le`, `preavis_le`, `prix_centimes`, `periodicite`, `sous_traitance_document_id`, `reversibilite_clause_version` |
| `plt_etablissement_organisme` **P** | `organisme_id`, `etablissement_id` (lien « même organisme ») |
| `plt_operateur` **P** | `user_id`, `role` (`operateur`/`support`/`equipe`), `confidentialite_signee_le`, `actif` |
| `mes_porte` | `etablissement_id`, `porte` (enum), `etat` (`fermee`/`ouverte`), `ouverte_par_personne_id`, `ouverte_le`, `fermee_motif`, `fermee_le` |
| `mes_prerequis` **P** | `code`, `porte`, `libelle`, `mode` (`verifie`/`atteste`), `legal` (bool), `source` |
| `mes_prerequis_etat` | `etablissement_id`, `prerequis_code`, `etat` (`manquant`/`verifie`/`atteste`/`deroge`), `piece_document_id`, `par_personne_id`, `le`, `derogation_motif` |
| `mes_etape` | `etablissement_id`, `etape_code`, `faite_le`, `par` |
| `mes_module` | `etablissement_id`, `module_code`, `actif` (bool), `regime` (`officiel_scolia`/`doublon`/`suivi`), `active_le`, `active_par`, `essai` (bool) |
| `mes_module_prerequis` **P** | `module_code`, `prerequis_code` |
| `mes_test_avis` | `etablissement_id`, `le`, `par`, `resultat` (`ok`/`echec`), `detail` |
| `rep_execution` | `etablissement_id`, `numero`, `source` (enum), `format_version`, `mode` (`simulation`/`application`/`repetition`), `cible` (`jetable`/`preprod`/`production`), `debut_le`, `fin_le`, `duree_s`, `etat` (`en_cours`/`terminee`/`echec`/`annulee`), `operateur_user_id`, `confirmee_par_personne_id`, `rapport_document_id`, `annexe_cle_objet`, `source_detruite_le` |
| `rep_execution_compte` | `execution_id`, `etablissement_id`, `objet` (table ou collection source), `lues`, `reprises`, `archivees`, `ignorees`, `refusees`, `a_decider` (contrainte : somme = lues) |
| `rep_execution_total` | `execution_id`, `etablissement_id`, `objet`, `cle` (ex. séjour), `total_source_centimes`, `total_cible_centimes` |
| `rep_refus` | `execution_id`, `etablissement_id`, `source_objet`, `source_id`, `regle_code`, `message` (sans donnée personnelle) |
| `rep_correspondance` | `etablissement_id`, `source`, `source_objet`, `source_id`, `cible_objet`, `cible_id`, `regle_code`, `statut` (`rapprochee`/`a_decider`/`ignoree`/`creee`), `execution_id`, `modifie_depuis_reprise` (bool) ; unique (etab, source, source_objet, source_id) (T0 §8.4) |
| `rep_decision` | `etablissement_id`, `correspondance_id`, `decision` (`rattacher`/`creer`/`ignorer`), `cible_id`, `par_personne_id`, `le`, `motif` |
| `rep_synonyme` | `etablissement_id`, `source`, `domaine` (`classe`/`motif`/`matiere`/…), `valeur_source`, `valeur_cible_id`, `propose_par` (`sys`/`ia`/personne), `valide_par`, `valide_le` |
| `rep_echantillon` | `execution_id`, `etablissement_id`, `graine`, `objet`, `source_id`, `cible_id`, `relu_par`, `relu_le`, `conforme` (bool), `remarque` |
| `rep_bascule` | `etablissement_id`, `module_code`, `etat` (`prepare`/`repete`/`gele`/`repris`/`ouvert`/`next_arrete`/`retour_arriere`), `gel_le`, `ouvert_le`, `next_arrete_le`, `execution_id` |
| `rep_bascule_objet_vol` | `bascule_id`, `etablissement_id`, `source_id`, `regle`, `repris_le` |
| `rep_doublon_depot` | `etablissement_id`, `source`, `domaine`, `depose_le`, `fichier_document_id`, `execution_id` |
| `rep_doublon_ecart` | `etablissement_id`, `domaine`, `periode_debut`, `periode_fin`, `sujet_type`, `sujet_id`, `nature` (`scolia_plus`/`officiel_plus`/`valeurs_differentes`), `indicateur_code`, `valeur_scolia`, `valeur_officiel` (textes typés courts), `statut` (`ouvert`/`explique`/`retour_cree`), `retour_id` |
| `rep_doublon_mesure` | `etablissement_id`, `domaine`, `semaine`, `elements`, `concordants` |
| zone `reprise.*` | une table par format de source (`reprise.next_travel`, `reprise.charlemagne_eleve_v1`…), colonnes nommées, `execution_id`, `source_ligne` ; **hors RLS applicative** mais accessible au seul rôle de l'outil ; vidée par `MSE-179` |
| `mes_invitation_lot` | `etablissement_id`, `population_code`, `cree_par`, `envoye_le`, `nombre`, `confirme_par` |
| `mes_invitation` | `lot_id`, `etablissement_id`, `personne_id`, `user_id`, `envoyee_le`, `relance_1_le`, `relance_2_le`, `activee_le`, `passkey_le`, `erreur_code` |
| `sup_demande` | `etablissement_id`, `numero`, `ouverte_par`, `categorie`, `gravite`, `ecran_code`, `version`, `etat`, `prise_par_user_id`, `resolue_le`, `close_le` |
| `sup_message` | `demande_id`, `etablissement_id`, `auteur_user_id`, `le`, `texte` |
| `sup_autorisation` | `etablissement_id`, `demande_id`, `support_user_id`, `motif`, `mode` (`lecture`/`ecriture`), `debut_le`, `fin_le`, `accordee_par_personne_id`, `accordee_le`, `ecriture_accordee_par_personne_id`, `revoquee_le`, `revoquee_par` ; contrainte : santé jamais dans les domaines |
| `sup_autorisation_domaine` | `autorisation_id`, `etablissement_id`, `domaine` |
| `sup_autorisation_sujet` | `autorisation_id`, `etablissement_id`, `sujet_type`, `sujet_id` |
| `sup_urgence` | `etablissement_id` (nullable si plateforme entière), `operateur_user_id`, `motif`, `debut_le`, `fin_le`, `notifiee_le` |
| `plt_incident` **P** | `id`, `titre`, `composant`, `gravite`, `debut_le`, `fin_le`, `etat`, `public` (bool) |
| `plt_incident_etab` | `incident_id`, `etablissement_id`, `informe_le` |
| `plt_maintenance` **P** | `debut_le`, `fin_le`, `annonce_le`, `description` |
| `plt_version` **P** | `version`, `git_sha`, `publiee_le` |
| `plt_version_note` **P** | `version`, `role_code`, `rang`, `texte` |
| `plt_aide_fiche` **P** | `code`, `ecran_code`, `version`, `titre`, `corps`, `publiee_le` |
| `plt_aide_visite` **P** | `code`, `role_code`, `version` ; étapes : `plt_aide_visite_etape` (`visite_code`, `rang`, `ecran_code`, `cible`, `texte`) |
| `mes_formation_suivi` | `etablissement_id`, `user_id`, `visite_code`, `faite_le` (visible de la personne ; agrégé seulement pour le référent) |
| `mes_retour` | `etablissement_id`, `auteur_user_id`, `ecran_code`, `version`, `nature` (`probleme`/`idee`/`ecart_doublon`), `texte`, `etat`, `ecart_id` |
| `plt_facture_abonnement` **P** | `etablissement_id`, `numero`, `emise_le`, `montant_centimes`, `etat`, `document_id` |

**Registre des tables (E12)** : `mes_invitation` neutralisée à la restauration (pas de renvoi d'invitation, comme `ent_agent_invitation`) ; `sup_autorisation` neutralisée (fin forcée à la restauration) ; zone `reprise.*` classée `Technique`, **volatile**, jamais exportée ; `rep_*` catégorie `mes.reprise` (27 : durée de preuve, [R] 3 ans) ; `sup_*` catégorie `sup.demande` ([R] 2 ans) ; `mes_formation_suivi` `tech.usage` ([R] 1 an).

**Exceptions D-036** : **une**, déjà prévue par T0 §2 : l'**instantané brut** de la source conservé comme **fichier chiffré** de l'exécution (pas une colonne), jamais relu comme vérité, détruit après la bascule ; déclarée dans `MIGRATIONS.md`. Aucune colonne JSON, aucun tableau.

---

## 14. Écrans

1. **Console de la plateforme** (opérateurs) : établissements (compteurs, contrat, portes), création guidée, supervision, incidents, maintenances, versions, aide, support.
2. **Mise en service** (établissement) : assistant (étapes), tableau « prêt à ouvrir » (portes en colonnes, prérequis en lignes, vert / bleu / orange), modules et régimes, calendrier.
3. **Reprise** : tableau de bascule (modules × états), exécutions (liste, durées, comparaison), rapport (résumé, comptages, totaux, échantillons côte à côte, refus), **file de décisions** (deux fiches côte à côte, champs différents surlignés, trois boutons), synonymes.
4. **Doublon** : tableau de concordance par domaine et semaine ; liste des écarts filtrable ; fiche d'écart → « créer un retour ».
5. **Comptes** : populations, lots, suivi d'activation par classe (barres), adresses en erreur, guichet.
6. **Support** (établissement) : demandes, autorisations en cours (avec compte à rebours et bouton « révoquer »), historique, rapports.
7. **Support** (plateforme) : file, vue support masquée, demande d'autorisation.
8. **Aide** : panneau latéral « ? » ; centre d'aide ; visites ; nouveautés ; « signaler ».
9. **Page d'état** publique.
10. **Démonstration** : page d'entrée par rôle, bandeau « démonstration ».
**Mobile** (socle 11) : activation et enrôlement de la passkey, aide, page d'état, notifications d'incident ; pour la direction, **accorder ou révoquer une autorisation de support** (un geste, passkey).

---

## 15. Lots de code (équipe Sonnet)

| Lot | Contenu | Dépend de | Migrations | Quand |
|---|---|---|---|---|
| **MS0** | **Correctifs urgents** : purge périodique des sessions et jetons (MSE-D01) ; annulation réelle des lots d'import ou retrait de l'état `annule` (MSE-D02) ; séparation création plateforme / déclaration locale (MSE-D03, squelette) ; `/health` public réduit (MSE-D05) ; commentaire de mot de passe retiré (MSE-D06) ; garde CI « absent de la production » préparée pour `reprise` (MSE-D08, R-MSE34) ; refus de copie de production vers labo et démo (MSE-233) | — | **2560** | novembre–décembre 2026 |
| MS1 | Création d'un établissement par la plateforme, premier administrateur, rôles modèles, catalogue 27, modules par défaut ; **environnement et jeu de démonstration** (lycée, collège, public), réinitialisation, envois bloqués | MS0 | 2561 | **démo de décembre 2026** |
| MS2 | Assistant de paramétrage, portes, prérequis vérifiés et attestés, dérogations, test de l'avis d'absence, calendrier de mise en service | MS1, 27 G1–G2 | 2562 | décembre 2026–janvier 2027 |
| MS3 | Modules : activation, prérequis déclarés par les socles, régimes et bandeaux | MS2 | 2563 | janvier 2027 |
| MS4 | `ent-reprise` : (a) inspection et comptages sur un export fictif au format Next.js = **R0** ; (b) anonymisation = **R1** ; (c) zone typée, décodage, correspondance, simulation = **R2** ; (d) référentiels, personnes, file de décisions = **R3** ; première répétition (T1) = **R4** ; rapports, signature, annulation, tableau de bascule ; puis modules R5–R9 (salles, voyages, fichiers, absences et RH, demandes) | MS1 ; socles 2, 4, 5, 6, 7 | 2564, 2565, 2566 | décembre 2026 → avril 2027 (T1–T3) |
| MS5 | Adaptateurs Charlemagne (sur exports anonymisés F-02 à F-04), orchestration SIECLE (socle 9), modèles de tableurs | MS4, F-01 à F-07 | 2567 | printemps 2027 |
| MS6 | Comptes : création sans secret, invitations par lots, enrôlement, suivi, relances, guichet, aide famille | MS1, socle 1 | 2568 | février–mars 2027 (= **R10**) |
| MS7 | Doublon : dépôts, synchronisation, conflits, comparateur, retours, tableau de concordance | MS5 | 2569, 2570 | été 2027 (avant la rentrée) |
| MS8 | Support : demandes, autorisations bornées, vue support masquée, traces vers le 27, bris de glace, rapports | 27 G4 | 2571, 2572 | printemps 2027 (avant les données réelles) |
| MS9 | Exploitation : page d'état hors Scaleway, incidents, maintenances, notes de version, surveillance des avis d'absence, runbooks et exercices PRA tracés | MS0, PRA | 2573 | printemps 2027 |
| MS10 | Aide contextuelle, visites, parcours de formation, retours, assistant d'aide sans données | MS9 | 2574 | été 2027 |
| MS11 | Adaptateurs École Directe, Pronote, réimport ScolIA ; export vers l'outil officiel si décidé ; modules restants (R11) | MS5 | — | 2027-2028 |
| MS12 | Contrats, organismes, suspension, fin de contrat avec le 27 | MS1, 27 G10 | 2575 | avant la commercialisation |
| MS13 | Facturation de l'abonnement (factures électroniques) | MS12, expert-comptable | 2576 | avant la commercialisation |

**Parcours minimal** : démo de décembre = MS0, MS1, MS2, MS4 a ; avant les premières données réelles = MS3, MS4 b–d, MS6, MS8, MS9 ; avant la rentrée 2027 = MS5, MS7, MS10.

---

## 16. Migrations (plage imposée **2560–2579**, registre `MIGRATIONS.md`)

| N° | Contenu |
|---|---|
| **2560** | MS0 : colonne d'état d'annulation et date sur `ent_import_lot` si l'annulation réelle est retenue (sinon retrait de la valeur `annule` du `CHECK`) ; aucune autre modification de schéma (purge, `/health`, gardes sont du code) |
| 2561 | MS1 : `plt_etablissement_contrat`, `plt_etablissement_organisme`, `plt_operateur` |
| 2562 | MS2 : `mes_porte`, `mes_prerequis` (P), `mes_prerequis_etat`, `mes_etape`, `mes_test_avis` |
| 2563 | MS3 : `mes_module`, `mes_module_prerequis` (P) |
| 2564 | MS4 : `rep_execution`, `rep_execution_compte`, `rep_execution_total`, `rep_refus` |
| 2565 | MS4 : `rep_correspondance`, `rep_decision`, `rep_synonyme`, `rep_echantillon` |
| 2566 | MS4 : schéma `reprise` et rôle de l'outil ; `rep_bascule`, `rep_bascule_objet_vol` |
| 2567 | MS5 : tables de zone des formats Charlemagne et tableurs (ajoutées par format, additives) |
| 2568 | MS6 : `mes_invitation_lot`, `mes_invitation` |
| 2569 | MS7 : `rep_doublon_depot` |
| 2570 | MS7 : `rep_doublon_ecart`, `rep_doublon_mesure` |
| 2571 | MS8 : `sup_demande`, `sup_message` |
| 2572 | MS8 : `sup_autorisation`, `sup_autorisation_domaine`, `sup_autorisation_sujet`, `sup_urgence` |
| 2573 | MS9 : `plt_incident`, `plt_incident_etab`, `plt_maintenance`, `plt_version`, `plt_version_note` |
| 2574 | MS10 : `plt_aide_fiche`, `plt_aide_visite`, `plt_aide_visite_etape`, `mes_formation_suivi`, `mes_retour` |
| 2575 | MS12 : compléments de contrat et de suspension |
| 2576 | MS13 : `plt_facture_abonnement` |
| 2577 | ajout personas (§21) : `rep_preapprobation`, `rep_bascule_dependance`, `rep_doublon_perimetre`, `rep_correspondance_colonne`, `sup_vue_sujet_avis`, `mes_seance_activation` |
| **2578** | RLS stricte des tables `mes_`, `rep_`, `sup_` |
| **2579** | réservé : suppression des tables de zone `reprise.*` d'un format abandonné, **seulement sur décision** (E8) |

Aucun fichier de migration écrit par cette conception. La zone `reprise.*` et `rep_*` seront écrites en **2564–2567** et non dans la plage 3990 du socle 9 : les imports officiels restent au socle 9, l'orchestration de reprise au 28.

---

## 17. Tests et recette
- **Gardes** : `ent-reprise` refuse une adresse non locale (test avec une URL distante simulée) ; source ouverte en lecture seule (une écriture échoue) ; binaire de production sans `reprise` ni `labo` (CI) ; démo : tout envoi échoue.
- **Reprise** (jeu fictif au format Next.js construit à partir du schéma, sans donnée réelle, T0 R0) : invariant des comptages ; totaux au centime ; idempotence (deux exécutions = aucun changement) ; annulation (objets retirés, objets modifiés listés) ; clé JSON inconnue → refus ; préfixe de secrets → refus ; jumeaux, INE contradictoire, adresse partagée → « à décider » ; santé exclue ; empreintes de mots de passe non reprises.
- **Anonymisation** : zéro occurrence des valeurs du jeu d'origine ; colonne non classée → blocage.
- **Portes** : une porte ne s'ouvre pas avec un prérequis manquant ; dérogation refusée sur un prérequis légal ; l'opérateur ne peut ouvrir aucune porte d'établissement.
- **Support** : sans autorisation, aucune donnée ; autorisation expirée → coupure ; santé jamais accessible ; trace visible dans le journal famille (27) ; pas de route d'usurpation (test statique sur les routes).
- **Doublon** : synchronisation sans écrasement d'une saisie ScolIA ; un seul outil avise.
- **Isolation** : chaque table dans la matrice ; un opérateur ne lit que des compteurs.
- **Chronométrage** : la répétition doit tenir dans le créneau de bascule (vendredi soir → samedi) ; mesure à chaque répétition.
- **Recette personas** : `PERSONAS_28.md`.

---

## 18. Réglages signalés (D-023) — S-560 à S-573

| N° | Réglage | Défaut |
|---|---|---|
| S-560 | Sous-domaines réservés | `www`, `admin`, `api`, `support`, `status`, `demo`, `aide` |
| S-561 | Part des personnels activés pour ouvrir aux personnels | 80 % (dérogeable) |
| S-562 | Seuil de cas « à décider » ouverts pour ouvrir aux familles | 0 sur les responsables légaux |
| S-563 | Fréquence de synchronisation du doublon | hebdomadaire ; quotidienne en septembre |
| S-564 | Relances d'activation | J+3 et J+10 ; lots de 200 familles par heure |
| S-565 | Qui peut ouvrir une demande de support | référent, direction, secrétariat |
| S-566 | Délais cibles du support | bloquant 4 h ouvrées ; gênant 2 jours ; question 5 jours |
| S-567 | Durée maximale d'une autorisation de support | 4 h (prolongeable une fois) |
| S-568 | Fenêtres de maintenance | hors 7 h 30–18 h 30 les jours de cours ; annonce 48 h avant |
| S-569 | Désactivation des invitations non utilisées | 60 jours |
| S-570 | Réinitialisation de la démonstration | chaque nuit |
| S-571 | Durée de vie d'une pré-production fermée | 7 jours |
| S-572 | Destruction des copies de la source après bascule | 30 jours après l'ouverture du module |
| S-573 | Taille de l'échantillon d'un rapport de reprise | 20 objets par module |
| S-574 | Marge de comptages pour une application pré-approuvée (ajout personas) | 0 sur les personnes, 1 % sur les autres objets |
| S-575 | Rythme du résumé des nouveautés (ajout personas) | hebdomadaire |

---

## 19. Décisions appliquées par défaut (D-031) et questions à Florian

### 19.1 Appliquées par défaut (D-031, à valider par Florian)
1. Le 28 absorbe T0 comme volet « adaptateur Next.js » ; lots R0–R11 rattachés à MS4–MS6 et MS11.
2. Création d'un établissement par la plateforme, jamais par un utilisateur (MSE-D03).
3. Quatre portes, prérequis vérifiés ou attestés, dérogation seulement non légale (R-MSE1, R-MSE2).
4. Données réelles seulement après la porte `donnees_reelles` (A8, A11, A13, A15, B3, A1–A6).
5. `ent-reprise` en ligne de commande, hors production, source locale en lecture seule (R-MSE10, R-MSE34).
6. Simulation par défaut ; application en production à deux personnes (R-MSE11).
7. Reprise par les commandes des socles (R-MSE21) ; zone typée, refus des clés inconnues (R-MSE23).
8. Rapport signé avant ouverture (R-MSE12) ; résumé sans données, annexe chiffrée (R-MSE25).
9. Annulation par exécution (R-MSE13).
10. Pas de reprise des mots de passe, lien puis passkey (R-MSE24, T0 Q-T0-09).
11. Anonymisation chez Florian, clé locale détruite, contrôle « zéro occurrence » (§6.4.13).
12. Pré-production fermée pour l'unique répétition réelle (R-MSE3, CNIL fiche 11).
13. Propriété unique et retour arrière pour chaque bascule (§6.6, modèle du 42).
14. Doublon : synchronisation sans écrasement, comparateur d'écarts, un seul outil avise (R-MSE9, R-MSE27).
15. Régimes de module affichés (R-MSE8).
16. Activation des comptes par lots, deux relances au plus, message reconnaissable (R-MSE15).
17. Démonstration dans un environnement séparé, envois bloqués, réinitialisée chaque nuit (R-MSE4, S-570).
18. Support sans usurpation, autorisation bornée et visible (R-MSE16) ; bris de glace technique seulement (R-MSE17).
19. Page d'état hors Scaleway (R-MSE31).
20. Avis d'absence testé avant ouverture, plan de secours (R-MSE7).
21. Formation suivie personnellement (R-MSE18).
22. Assistant d'aide sans données (R-MSE19).
23. Maintenance hors heures de cours (R-MSE32).
24. Secrets jamais en base ni en stockage objet (R-MSE33).
25. Modules : désactivation sans effacement (R-MSE35).
26. Lot MS0 avant la démo de décembre.

### 19.2 Soumis à Florian
| N° | Question | Recommandation |
|---|---|---|
| Q-MSE1 | Accord écrit de l'établissement, avis du DPO, AIPD : quand les obtenir pour les premières données réelles ? | avant T2 (février 2027) pour la répétition réelle (D-035 point 4) |
| Q-MSE2 | Exports réels : Florian lance `pg_dump` + copie du bucket (liste blanche) et les exports Charlemagne, anonymisés d'abord | oui (`A_FOURNIR_PAR_FLORIAN.md` F-02 à F-04, T0 Q-T0-12, Q-T0-15) |
| Q-MSE3 | Un établissement à deux UAI : un ou deux établissements dans ScolIA ? | dépend du socle 5 ; défaut : un par UAI, organisme commun |
| Q-MSE4 | Pendant le doublon, l'appel peut-il se faire dans ScolIA seul avec export vers Charlemagne (fin de la double saisie) ? | à étudier en T2 selon ce que Charlemagne accepte en import |
| Q-MSE5 | Hébergeur de la page d'état (hors Scaleway) | un service simple chez OVHcloud ou un hébergeur statique européen [R] ; coût faible ; décision de Florian |
| Q-MSE6 | Convention de l'établissement pilote : sans facturation, rôle de Florian, propriété du logiciel (A8) | à écrire avec un juriste |
| Q-MSE7 | Facturation électronique de l'abonnement : plateforme agréée, calendrier d'émission | expert-comptable avant MS13 |
| Q-MSE8 | Modèle de contrat (offre, prix, préavis, réversibilité, niveaux de service, sous-traitance) | juriste avant la commercialisation |
| Q-MSE9 | Qui est support au départ (Florian seul ?) et quelles heures promettre ? | Florian, jours ouvrés ; à revoir à la commercialisation |
| Q-MSE10 | Démo de décembre : environnement `demo` séparé dès décembre, ou établissement de démonstration dans la production technique de T1 ? | séparé si le coût le permet ; sinon établissement de démo isolé, envois bloqués, **sans aucune donnée réelle dans la même base** tant que T2 n'est pas atteinte |

---

## 20. Faits, hypothèses, contradictions

**Faits [F]** : §2.1 relevé le 2026-11-11 sur `ent-mc0hoe` (chemins et lignes) ; Next.js : routes `demo/enter`, `onboarding/status`, `onboarding/microsoft-licenses`, registre `tenant-registry.ts` (déjà relevé par T0). Rien n'a été compilé ni exécuté.
**Hypothèses [H]** : formats d'export de Charlemagne, École Directe, Pronote (inconnus) ; durée d'une reprise ; capacités d'import de Charlemagne (Q-MSE4) ; frein principal au changement de logiciel.
**Contradictions et écarts signalés**
- `SOCLES.md` §3 (socle 28) cite « catalogue des 659 tables de Charlemagne, base d'une reprise » : c'est un catalogue de **noms**, pas un format d'export ; aucun adaptateur ne peut être écrit sans un export anonymisé (CL28-22).
- `TRANSITION_NEXT_RUST.md` §4 place la « mise en production technique » à Noël avec des données de démonstration **dans la production** ; le 28 recommande un environnement de démonstration **séparé** (Q-MSE10) ; à défaut, aucune donnée réelle dans la même base avant T2.
- `09_INTEROP.md` réserve 3990 pour retirer `ent_import_lot`/`ent_import_ligne` : l'annulation réelle de MS0 (2560) vaut **jusqu'à** ce retrait ; à coordonner (une seule évolution de 144).
- `MISE_EN_SERVICE_DONNEES.md` A6 : « une restauration réellement testée » est ici un **prérequis vérifié** de la porte `donnees_reelles`, pas seulement une case.

---

## 21. Ajout personas (`PERSONAS_28.md`, 2026-11-11) — l'emporte en cas de contradiction

### 21.1 Cinq bloquants tranchés (D-031)
- **A — Garde de la source.** `ent-reprise` ne reçoit **jamais** d'adresse de base : seulement des **fichiers** d'export (empreinte vérifiée), qu'il restaure lui-même dans une base éphémère sans réseau sortant, détruite à la fin. R-MSE10 est remplacée par R-MSE37 ; §6.4 point 2 corrigé.
- **B — Second accord.** La direction **pré-approuve une simulation numérotée** ; l'application n'est permise que sur le même export (même empreinte) et des comptages identiques à la marge S-574 près (R-MSE38). R-MSE11 précisée.
- **C — Zone chiffrée.** Colonnes sensibles déclarées par chaque adaptateur, chiffrées au chargement ou non chargées (R-MSE39). §13 (« colonnes texte brutes ») corrigé.
- **D — Vue support.** « Vue telle que la voit X » : droits de X, identité du support, lecture seule, masquée, autorisation qui nomme X, avis à X (R-MSE40).
- **E — Doublon allégé.** Périmètre de doublon par domaine choisi par la direction ; hors périmètre, saisie dans l'outil officiel seulement et synchronisation (R-MSE41, Q-MSE13).

### 21.2 Actions ajoutées (MSE-280 à MSE-294)
| N° | Action | Acteur · droit | Règles | Lot |
|---|---|---|---|---|
| MSE-280 | Définir le périmètre du doublon par domaine (classes, groupes) | DIR · `mise_en_service.gerer` | R-MSE41 | MS7 |
| MSE-281 | Déclarer les dépendances entre bascules et refuser une ouverture prématurée | OPE | — | MS4 |
| MSE-282 | Générer le runbook de bascule par module, à cocher | SYS, OPE | — | MS4 |
| MSE-283 | Décider par lot un motif homogène de rapprochement (jamais pour des élèves) | ADF · `reprise.decider` | aperçu, annulation | MS4 |
| MSE-284 | Créer l'établissement de formation (structure réelle, personnes fictives, personnels consentants) | OPE, DIR | R-MSE43 | MS10 |
| MSE-285 | Pré-remplir le calendrier de mise en service (T0–T4, vacances) | SYS | — | MS2 |
| MSE-286 | Lettre d'activation imprimée nominative (code + QR) et code de confirmation sur coordonnée vérifiée | ADF | R-MSE44 | MS6 |
| MSE-287 | Enrôler une clé de sécurité physique au guichet | ADF | Q-MSE12 | MS6 |
| MSE-288 | Afficher une référence de requête dans les erreurs et la relier aux journaux techniques | SYS | sans données | MS8 |
| MSE-289 | Adaptateur configurable : correspondance de colonnes par établissement, aperçu, version | OPE, ADF | formats officiels fixes | MS5 |
| MSE-290 | Choisir le périmètre historique (années en données, années en documents) | DIR, OPE | — | MS5 |
| MSE-291 | Séance d'activation en classe (code de séance, codes personnels imprimés) | PP | — | MS6 |
| MSE-292 | Signer le total général des soldes ouverts (comptabilité) | CPT | — | MS5 |
| MSE-293 | Mode dégradé de l'avis d'absence (liste imprimable, « prévenu par téléphone ») | CPE | R-MSE9 | MS9 |
| MSE-294 | Rapport de reprise fictif présenté dans la démonstration | SYS | — | MS1 |

### 21.3 Règles ajoutées (R-MSE37 à R-MSE45)
| N° | Règle |
|---|---|
| R-MSE37 | L'outil de reprise n'accepte que des fichiers d'export et les restaure lui-même dans une base éphémère sans réseau ; aucune option ne permet de lui donner une adresse de base. |
| R-MSE38 | Une application en production exige une simulation pré-approuvée par la direction, sur le même export (empreinte) et avec des comptages concordants. |
| R-MSE39 | Les colonnes sensibles d'une source sont chiffrées au chargement en zone de reprise, ou non chargées. |
| R-MSE40 | Le support peut afficher une page « telle que la voit X » avec les droits de X, sous sa propre identité, en lecture, sur autorisation nommant X, et X en est informé. |
| R-MSE41 | Le doublon porte sur un périmètre choisi ; hors périmètre, personne ne saisit deux fois. |
| R-MSE42 | Un objet modifié dans l'outil d'origine après l'heure de gel arrête la reprise différentielle pour décision. |
| R-MSE43 | L'établissement de formation n'emploie que des personnes fictives, sauf les personnels qui l'acceptent. |
| R-MSE44 | Un code d'activation imprimé ne suffit pas seul : il est complété par un code envoyé à une coordonnée vérifiée du dossier. |
| R-MSE45 | L'annexe nominative d'un rapport ne s'ouvre qu'avec un responsable de l'établissement et se détruit avec les copies de la source ; seul le résumé est gardé comme preuve. |

### 21.4 Cas limites ajoutés (CL28-57 à CL28-60)
| N° | Cas | Réponse |
|---|---|---|
| CL28-57 | Travail dans Next.js après le gel | arrêt, liste, décision (R-MSE42) |
| CL28-58 | Demande d'autorisation d'un compte de support inconnu | refus automatique, alerte à l'éditeur |
| CL28-59 | Colonne renommée entre deux exports Charlemagne | arrêt, nouvelle correspondance proposée |
| CL28-60 | Mandat SEPA repris pendant le doublon | jamais réactivé ; RUM et date gardées |

### 21.5 Modèle de données ajouté (migration 2577, sans JSON)
`rep_preapprobation` (`etablissement_id`, `execution_simulee_id`, `export_sha256`, `approuvee_par_personne_id`, `approuvee_le`, `expire_le`, `utilisee_par_execution_id`) ; `rep_bascule_dependance` (`etablissement_id`, `bascule_id`, `depend_de_bascule_id`) ; `rep_doublon_perimetre` (`etablissement_id`, `domaine`, `objet_type` (`classe`/`groupe`), `objet_id`) ; `rep_correspondance_colonne` (`etablissement_id`, `source`, `format_version`, `colonne_source`, `champ_attendu`, `version`, `valide_par`, `valide_le`) ; `sup_vue_sujet_avis` (`etablissement_id`, `autorisation_id`, `sujet_user_id`, `affichee_le`, `avis_envoye_le`) ; `mes_seance_activation` (`etablissement_id`, `classe_id`, `ouverte_par`, `code_seance_empreinte`, `debut_le`, `fin_le`, `activations`). `rep_execution` gagne `export_sha256`.

### 21.6 Lots
MS4 a intègre R-MSE37 dès le squelette ; MS4 c intègre R-MSE39 ; MS4 ajoute MSE-281 à MSE-283 ; MS5 ajoute MSE-289, MSE-290, MSE-292 ; MS6 ajoute MSE-286, MSE-287, MSE-291 ; MS7 ajoute MSE-280 ; MS8 ajoute MSE-288 et la vue telle que la voit X ; MS9 ajoute MSE-293 ; MS10 ajoute MSE-284.

### 21.7 Réglages
S-574 : marge de comptages pour la pré-approbation (0 sur les personnes, 1 % sur les autres objets). S-575 : résumé des nouveautés (hebdomadaire).

### 21.8 Bilan chiffré après la passe
195 actions (MSE-100 à MSE-294), 60 cas limites, 45 règles, 16 réglages (S-560 à S-575), 14 lots.
