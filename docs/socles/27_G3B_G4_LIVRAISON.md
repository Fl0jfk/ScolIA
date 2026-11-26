# Socle 27, palier G3b + G4 — **extrait d'accès, remise, traces de consultation** (livraison)

Branche `ent-socle-27-palier-g3b-g4`, partie d'`origin/ENT` (règle 39 : un palier, cinq lots, une seule CI).
Migrations **2545** (lot 1) et **2546** (lot 4), en **fin** du tableau de `migrate.rs` (règle 38), **jamais** ajoutées à
`migrations_versees.txt`. `MIGRATION_TAG` inchangé (3808). `2544_rgpd_g3_demandes` a été versée dans un commit à part
(elle manquait à `migrations_versees.txt`).
Conception : `docs/socles/27_RGPD_ARCHIVES.md` (§9, §12, §14.3), `docs/socles/PERSONAS_27.md`, consigne
`docs/pilotage/consignes/27_G3B_G4_PALIER.md`.

---

## Lot 1 — migration 2545, moteur d'assemblage, correctifs G3a

### Fait

- **Migration 2545** : `rgp_reglage` (+ journal en ajout seul), `rgp_demande_sujet` (fratrie, RGP-302), `rgp_extrait`
  (versions, empreinte, péremption, soumission au DPO) et `rgp_extrait_section` (statut, motif, empreinte, décision
  de remise incomplète). RLS posée sur les 5 tables ; gardes qui gèlent les colonnes de structure ; l'ensemble des
  événements du journal G3a est élargi. **Le contenu d'un extrait n'est jamais stocké** : seuls la structure, les
  empreintes par section et les décisions humaines le sont ; la lecture reconstruit le contenu et compare l'empreinte
  (écart → extrait **périmé**, 409, rien n'est servi).
- **Moteur** `ent-db/src/droits_extrait.rs` : assemblage piloté par le registre des données (élève : table portant
  `eleve_id`, rattachements indirects, règles dédiées de la messagerie ; personne : fournisseurs écrits pour comptes,
  administratif, personnels, messagerie, documents). Colonnes jamais rendues (secrets, clés de stockage), champs scellés
  jamais déchiffrés (IBAN rendu `FR76 **** **** 1234`), santé exclue et **listée** tant que le module est inactif,
  tables du foyer-payeur exclues avec leur raison, protections (adresse protégée, personne réservée, identité
  scellée) **jamais lues ni nommées** (R-RG38/46). Un fournisseur non écrit rend une section « non couverte » : l'extrait
  est **incomplet** (R-RG18) et sa remise exige une décision motivée de `droits.arbitrer`, section par section.
- **Services** : `droits_sujet_ajouter`, `droits_extrait_assembler`, `droits_extraits`, `droits_extrait_lire` (lecture
  tracée **avant** d'être servie, R-RG8), `droits_extrait_arbitrer_incomplet`, `droits_extrait_soumettre_dpo`,
  `droits_extrait_avis_dpo`, `droits_reglage_lire`, `droits_reglage_modifier`.
- **DPO** : lit le contenu seulement s'il détient `droits.traiter`, ou `droits.arbitrer` **et** que l'extrait lui a été
  soumis (§12.2). Il n'assemble pas.
- **Correctifs G3a** : Pf1 (plafond des dépôts publics réglable `rgp_reglage`, verrou transactionnel par établissement),
  Pf5 (comportement conservé et testé : numéro attribué à la vérification réussie, même si la qualité reste à arbitrer),
  Pf6 (destruction de pièce : reprise sous verrou `FOR UPDATE SKIP LOCKED` → une seule ligne `piece_detruite` ; fichier
  partagé → avis unique à la direction ; refus possible sans décision sur les titulaires tant que l'identité n'est
  pas établie), Pf8 (horloge injectée : `droits_alerter_echeances_a`, `accuser_reception(.., maintenant)`), effet de
  bord de M-a (une pièce déjà rattachée à la **même** demande est déplacée sur la nouvelle vérification), rustdoc de
  `inserer`, motif libre de prolongation **hors du journal** (il reste dans sa colonne).

### Cas limites couverts

Demandeur sans identité établie (rien ne s'assemble), sujet d'un autre établissement (404), sujet déjà principal ou
déjà ajouté (409), qualité `absente` (refusée), qualité autre que confirmée (assemblage réservé à `droits.arbitrer`),
table qui échoue (savepoint par table : le reste de l'extrait survit), extrait périmé (refus de lecture et de
soumission), nouvelle soumission (efface l'avis précédent), réglage hors bornes ou inconnu.

### Tests (lot 1)

`tests_rgpd_g3b.rs` : 15 tests labo (assemblage et lecture, santé exclue, fournisseur en échec et arbitrage,
péremption, cloisonnement, fratrie, DPO avant/après soumission, fournisseurs écrits ou non couverts, Pf1, Pf5, Pf6 ×2,
Pf8, prolongation, M-a) + tests unitaires purs de `droits_extrait.rs` (masquage IBAN, empreintes déterministes, aucune
colonne interdite, protections jamais lues).

### Valeurs prudentes retenues (D-069)

- Santé écartée dès que la table est marquée `sante` et que le module est inactif (y compris les déclarations
  d'absence de santé).
- Champs scellés : jamais déchiffrés ; IBAN masqué.
- Extrait de personne : toujours « incomplet » tant que tous les fournisseurs ne sont pas écrits ; remise par arbitrage.
- Filtre de période appliqué aux seules tables portant une colonne de création datée.
- Qualité d'un sujet : `absente` refusée, hors `confirmee`/`sans_objet` → direction.
- `extrait_disponible_jours` = 7, `plafond_depots_publics_jour` = 30.

### Compteurs (règle 28)

RLS : **+5** tables (compte attendu dans `tests_rls.rs`). Aucune route ni action nouvelle dans ce lot
(`Action::TOUTES` inchangé à 286). Registre des tables : +5 entrées.

### Vérification locale du lot 1 (étape b)

- `rustfmt --check` des fichiers nouveaux : propre (la dette de formatage du backend existant reste non bloquante en CI,
  `cargo fmt --all --check` n'a pas été « corrigé » : `cargo fmt --all` interdit).
- Trois commandes clippy de la CI (`--workspace --all-targets` avec et sans `ent-api/labo,ent-lab/labo`, puis `ent-types`) :
  **0 avertissement** (rustc et clippy 1.99).
- `cargo nextest` ent-db avec labo, **4 tranches** (`--partition count:i/4`), **base neuve par tranche** :
  769 / 765 / 764 / 767 tests passés. Les échecs rencontrés (3 + 3 + 2 + 1) sont des tests qui **comptent des lignes
  globales pendant que d'autres tests écrivent** (`tests_reprise_r2/r3`, `tests_messagerie_p3`) ou des semis concurrents
  (`Reessayer`) : rejoués seuls avec `-j1`, les **40 tests concernés passent**. Un échec réel a été corrigé
  (ordre alphabétique du registre des tables).
- `cargo nextest -p ent-api -p ent-db` sans labo : 918 tests, 915 passés ; les 3 échecs sont les mêmes tests de reprise qui
  comptent des lignes globales ; rejoués seuls avec `-j1` : 18 / 18 passent.

### Contre-revue locale (étape c) — **non exécutée par `review-sonnet`**

L'agent `review-sonnet` n'existe pas dans l'environnement de cette session (le lancement répond « agent type not found » ;
seuls `explore`/`general-purpose`… sont listés, et la consigne interdit d'en utiliser d'autres). **Auto-revue** à la place,
sur le modèle de menaces du lot : identifiants SQL dynamiques (tables et colonnes issues du registre **et** filtrées par
`identifiant_sur`, jamais d'entrée libre), filtre `etablissement_id` explicite en plus de la RLS, contenu jamais stocké,
colonnes secrètes/scellées jamais rendues, DPO limité à l'extrait soumis, lecture tracée avant service, verrous de
concurrence (destruction de pièces, plafond de dépôts), aucune heure réelle dans les tests. À refaire par un relecteur
indépendant : **à signaler au coordinateur**.

---

## Lot 2 — marquage déterministe et relecture humaine

### Fait

- **Marquage** (`ent-db/src/droits_extrait_masquage.rs`) : règles **écrites**, jamais une IA (R-RG15). Un marquage désigne un
  élément (table, ligne ou rang, colonne), jamais une copie de la donnée. Raisons : `autre_parent`, `adresse_protegee`
  (et tout indice de localisation : adresse, ville, code postal, transport… tant qu'un autre parent est protégé),
  `parent_donnees_propres` (élève demandeur : seule l'identité des responsables reste, R-RG48), `autre_eleve`,
  `redacteur` (note interne), `signalement`, `piece_tiers` (une pièce du coffre qui concerne un tiers n'est jamais remise
  ni caviardée, R-RG45), `manuel`. Table `rgp_extrait_masquage` ajoutée à la migration **2545** (6e table RLS du lot).
- **Relecture** : `droits_masquages`, `droits_masquage_decider` (`masque` ou `leve` avec motif ; lever une protection,
  l'autre parent, les données propres des parents, un signalement ou une pièce de tiers exige `droits.arbitrer`),
  `droits_masquage_ajouter` (décidé « masqué » d'emblée). **Aucune remise tant qu'un marquage reste à décider** :
  `remissible_en_l_etat` tient compte de `masquages_a_decider`. Une décision déjà prise est **reportée** sur la nouvelle
  version quand l'élément n'a pas changé.
- **R-RG46** : un porteur de `droits.traiter` sans `droits.arbitrer` ne voit jamais le code `adresse_protegee` (la vue dit
  `a_arbitrer`), ni la valeur protégée (`[réservé à la direction]`), et reçoit « introuvable » s'il tente de lever.
  Les protections ne sont lues que pour savoir **si** une protection existe (identifiant de personne), jamais leur contenu.
- **Pf3** : la qualité `restreinte` ne vise plus que le demandeur protégé lui-même (ou l'élève : on ne sait pas qui est visé,
  prudence) ; la protection d'un autre parent ne bloque plus le titulaire. **Pf4** : la vue de la demande est épurée pour
  qui n'a pas `droits.arbitrer` (qualité, note, code d'arbitrage, détails du journal → « à arbitrer »).
- **API** : 9 routes (`/rgpd/demandes/{id}/extraits…`, `…/masquages…`, `/rgpd/reglages`), DTO `ent-types::rgpd_extrait`.
- **Écrans** (Leptos, `modules/rgpd_droits/extrait.rs` + `modele_extrait.rs` natif) : section **Extrait** de la fiche
  (assembler, état des sections, périmé, incomplet), **Relecture** (éléments marqués surlignés, compteur, masquer / lever
  avec motif), **DPO** (soumettre, avis). `ui.rs` non étendu.

### Tests

`tests_rgpd_g3b2.rs` (5, labo) : jeu fictif « parents séparés + adresse protégée » (l'extrait remis ne contient ni le
téléphone ni l'adresse de l'autre parent, les données du demandeur restent ; le secrétariat ne voit ni la valeur ni le
code, la levée lui est refusée ; remise bloquée tant qu'un marquage est ouvert), levée motivée et ajout manuel, report de
décision, élève demandeur (R-RG48), Pf3 + Pf4. Unitaires purs (5) dans `droits_extrait_masquage.rs`, 2 de modèle d'écran,
1 test HTTP (`tests_rgpd_g3b_http.rs`). **Non couverts** : « messages de la mère avec le CPE absents de l'extrait du père »
(M-2711) — la règle est dans le SQL des fils de messagerie du lot 1 mais n'a pas de jeu d'essai dédié ; le CSV (lot 3).

### Valeurs prudentes (D-069)

Tant qu'un autre parent est protégé, toute colonne dont le nom évoque une localisation est masquée dans tout l'extrait
de l'élève ; un marquage non décidé est traité comme « à masquer » par `appliquer_masques`.

### Compteurs

RLS **+1** (6 pour 2545). Routes : +9 (toutes authentifiées : aucune ligne à ajouter aux inventaires de routes publiques).
`Action::TOUTES` inchangé (286).

---

## Correctifs de la contre-revue des lots 1 et 2 (`docs/pilotage/revues/27_G3B_G4_LOTS_1_2_CONTRE_REVUE.md`)

Corrigés, chacun avec un test (`tests_rgpd_g3b3.rs`) :

- **E1** : l'« autre titulaire » se calcule depuis `ent_eleve_lien.personne_id` (le compte est facultatif) ; les personnes
  des dossiers d'inscription et des participants « famille » des fils de l'élève y sont ajoutées ; toute ligne de
  `ent_inscription_responsable` qui n'est pas celle du demandeur est marquée, fiche rapprochée ou non. Test : autre
  parent sans compte, protégé ou non (l'assemblage ne tombe plus, nom, téléphone et adresse ne sortent pas).
- **E2 (M-2711)** : chaque message (et ses versions, ses pièces) n'est rendu que s'il date de la **période de participation**
  du lecteur (`du` inclus, `au` exclu, avant `suspendu_le`). Test : père sorti d'un fil, père entré tard.
- **E3** : une demande « pour soi » refuse tout sujet ajouté (409) ; `dem_est_sujet` ne vaut vrai que pour le sujet
  principal de la demande.
- **M1** : les masquages ajoutés à la main sont recopiés, avec leur décision, sur la nouvelle version.
- **M3** : `rgp_extrait.marquage_empreinte` (empreinte des marquages **calculés**) ; la lecture et la remise la
  recalculent : une protection posée après l'assemblage périme l'extrait.

Restent en file (moyens et faibles non demandés) : M2 (report de décision par empreinte de l'élément), M4 (alias
`a_arbitrer` trop parlant pour le secrétariat), M5 (ajout manuel : `rang` et validation de l'élément), M6 (marquage des
sujets « personne »), F1 à F8 (garde de `rgp_extrait_masquage`, gel de `periode_*`, tables de messagerie écartées sans
ligne « non couverte », GET à effet de bord, avis du DPO conservé après une décision, 403/404 du DPO).

---

## Lot 3 — remise, clôture, renvois, Pf2, Pf7, M-2711

### Fait

- **Migration 2545 étendue** (non versée) : `rgp_extrait_fichier` (fichiers scellés, empreintes, échéance de mise à
  disposition, `purge_le` écrit une fois), `rgp_remise` (une par extrait ; en ligne ou main propre avec émargement),
  `rgp_defi` (défis de passkey récente), `rgp_telechargement` (ajout seul), `rgp_cloture`, `rgp_renvoi` ; RLS sur les 6 ;
  gardes (ajout seul, fichier : seule la purge s'écrit).
- **Module** `ent-db/src/droits_remise.rs` : PDF lisible par `ent-pdf` (sections, destinataires, durées, sources,
  exclusions et pièces non remises, masquages rendus « [masqué : …] ») ; **CSV** par section écrit à la main (RFC 4180,
  BOM UTF-8, CRLF, format long `table,ligne,colonne,valeur`, apostrophe devant `= + - @` contre l'injection de formule).
- **Chiffrement sans dépendance nouvelle** (D-062, Q-EXP1 non tranchée, valeur prudente D-069) : pas d'archive ZIP AES ni
  `age`. Chaque fichier est encodé en base64 (écrit à la main, `SecretBox` ne scelle que du texte) puis **scellé** par
  `SecretBox` (usage dérivé `extrait-rgpd:v1`, AAD = établissement, extrait, version, nom du fichier), sous la clé
  d'objets `{etab}/rgpd-extraits/{id}.bin` ; empreintes SHA-256 du clair et du scellé en base.
- **Remise** (`droits.remettre`) : défi de passkey lié à l'empreinte de l'extrait et à l'intention (« remettre »), 5
  minutes, usage unique, brûlé avant vérification ; **en ligne** seulement sur le compte du demandeur **membre de cet
  établissement** (sinon 409 : main propre) ; main propre : nom et pièce vue obligatoires, PDF rendu une fois, rien de
  stocké. Jamais une adresse ou un numéro de la demande. Notification `rgpd.extrait_pret` sans nom d'élève.
- **Téléchargement** : défi de passkey **du demandeur** lié à l'extrait ; un autre compte reçoit 404 ; chaque
  téléchargement est compté (`rgp_telechargement`) et journalisé ; réponse JSON en base64url, `no-store`, `nosniff`.
- **Mise à disposition** 7 jours (`extrait_disponible_jours`), tâche horaire `droits_extraits_purger_tache` (jour injecté
  dans `droits_extraits_purger_a`) : suppression de l'objet, **confirmation** qu'il n'existe plus, puis `purge_le` ;
  une suppression non confirmée est rejouée au tour suivant.
- **Clôture** : `droits_cloturer` (la route de clôture de G3a l'appelle) enregistre les éléments remis **par empreinte**
  (extrait, sections) et les refus partiels motivés ; **renvois tracés** (rectification vers le socle propriétaire,
  « à corriger dans la source (SIECLE) », retrait de consentement relié au socle 18, CADA d'un tiers).
- **Pf2** : le seau HTTP d'un `contact` porte sur le contact **normalisé** (`anti_abus::identifiant_limite`).
- **Pf7** : `droits_pieces_identite_a_choisir` (la pièce se choisit dans le coffre) ; une vérification réussie d'un dépôt
  **sans compte « pour soi »** exige un sujet rapproché qui est le demandeur.
- **API** : `…/extraits/{id}/remise/defi`, `…/remise`, `…/renvois` (GET, POST), `…/cloture`, `…/pieces-identite`,
  `/rgpd/mes-donnees` (+ `…/extraits/{id}/defi`, `…/fichiers/{id}/telecharger`). **Écrans** (Leptos) : « Réponse et
  remise » (fiche), « Mes extraits » (page Mes données) ; logique pure dans `modele_remise.rs`.

### Valeurs prudentes retenues (D-069)

- Pièces du coffre et documents émis : **listés, non remis, non joints** (une pièce qui concerne un tiers n'est jamais
  remise ni caviardée) ; le nettoyage des métadonnées des pièces et la régénération des documents émis ne sont pas
  faits : à reprendre quand le socle 7 saura les générer.
- Remise en main propre : la passkey du gestionnaire est exigée aussi (même geste que la remise en ligne).
- Fichiers servis en JSON base64url (le transport binaire de la page n'accepte que le GET).
- Pf7 : « la pièce se dépose depuis la fiche » n'est pas fait (le dépôt reste celui du coffre) ; la liste des pièces à
  choisir remplace la saisie d'identifiant.

### Tests

`tests_rgpd_g3b3.rs` (labo) : remise en ligne complète (passkey absente, périmée, rejeu, secrétariat refusé, scellé
illisible sans la bonne AAD, notification sans nom, « Mes données », 404 pour un autre compte, CSV sans aucune donnée du
tiers, compteurs, purge à 7 jours avec un jour **injecté**, plus de téléchargement ensuite), main propre et demandeur sans
compte, remise refusée tant qu'un marquage est ouvert, clôture par empreintes et renvois, **M-2711** (messages de la mère
avec le CPE absents du PDF et des CSV du père), Pf7, et les correctifs E1/E2/E3/M1/M3. `anti_abus::tests_pf2`,
`droits_remise::tests` (base64, CSV, AAD), `modele_remise::tests`, HTTP (401, 403, 404, 409, 422).

### Compteurs

RLS **+6** (12 tables pour 2545). Routes : +8, toutes authentifiées (aucune ligne aux inventaires de routes publiques).
`Action::TOUTES` inchangé (286).

---

## Commits par lot (pour les contre-revues)

Liste tenue à jour à chaque lot ; chaque lot compte au moins un commit de code et un commit de livraison.

- Préalable : `4e49258ae` (2544 versée dans `migrations_versees.txt`).
- Lot 1 : `258ae1ef8` (migration 2545, moteur d'extrait, correctifs G3a), `67392f4dd` (livraison), `41c094bad` (résultat de la suite sans labo).
- Lot 2 : `e2c517e43` (marquage, relecture, Pf3/Pf4, routes, écran, livraison).
- Contre-revue des lots 1 et 2 : `44dbfe28a` (rapport) ; correctifs `4c8058436`.
- Lot 3 : `f2a9acffe` (module de remise), puis les commits suivants de la branche (routes, écrans, livraison).
