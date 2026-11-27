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
  `rgp_defi` (défis de passkey récente), `rgp_telechargement` (ajout seul), `rgp_cloture`, `rgp_redirection` ; RLS sur les 6 ;
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

RLS **+9** après correction (12 puis 15 tables pour 2545, voir « Correctif de normalisation » au lot 4). Routes : +8, toutes authentifiées (aucune ligne aux inventaires de routes publiques).
`Action::TOUTES` inchangé (286).

## Lot 4 — traces d'accès séparées, motif hors périmètre, page « Consultations du dossier »

### Fait

- **Migration 2546** (fin du tableau, non versée) : `acc_trace` **partitionnée par mois** sur `le` (clé primaire `(id, le)`), partitions créées
  par la fonction `acc_trace_assurer_partitions(date)` (mois courant + 2 suivants, `SECURITY DEFINER`, `search_path` fixé), appelée à la
  migration et chaque heure par le service ; **RLS stricte posée sur le parent ET sur chaque partition** (les partitions n'héritent ni des
  politiques ni des privilèges), déclencheur d'ajout seul au parent, `TRUNCATE` interdit sur chaque partition. Tables `acc_motif` (liste
  S-542, personnalisable), `acc_controle` (durée max 31 jours, seule la clôture est modifiable), `acc_alerte` (une alerte par règle, agent
  et fenêtre), registre de purge de plateforme `acc_purge` (ajout seul, sans RLS ; pas de `cons_execution`). Réglages S-541 (12 mois,
  minimum 12), S-543 (rafale), S-554 (seuils par rôle) ajoutés à `rgp_reglage`. **Reprise** : copie (pas déplacement) des
  consultations de `ent_journal_modif` (12 derniers mois), même identifiant, fonction rétablie à la date, `a_preciser` à défaut.
- **R-RG8** : `tracer_consultation_a` écrit `acc_trace` dans la **même transaction** que la lecture ; un échec d'écriture refuse la lecture.
  **R-RG31** : fonction et service figés à la date de la consultation (le service est la famille de fonction, aucun nom de personne).
- **Hors périmètre** (RGP-191/310, R-RG49) : la fiche d'un élève hors du périmètre de l'agent répond **409 `motif_requis`** avec la liste
  S-542 ; le même geste avec `?motif=` ouvre la fiche (désignation seule) et trace `hors_perimetre = true` avec le motif. Remplaçant,
  conseil de classe et sortie = périmètres temporaires, jamais « hors périmètre », sans motif.
- **Page « Consultations du dossier »** (RGP-192) : `GET /acces/consultations` (+ `GET /acces/motifs`) ; fonction et service jamais le nom,
  regroupement par jour (jour d'établissement), année courante et précédente, jamais santé ni psy ni dossier de droits, diffère
  l'affichage des consultations reportées, « support sans mention » pour le support éditeur, section protégée en code générique, rien
  vers le parent visé par la protection (R-RG46 : aucune trace « coordonnées protégées » pour le parent séparé). Écran web
  `mes-donnees/consultations`, sélecteur de motif sur la fiche élève.
- **Correctif de normalisation (G3b-3)** : `rgp_cloture.elements/refus` (JSONB, interdit par D-036 et la consigne) remplacés par les
  tables typées `rgp_cloture_element`, `rgp_cloture_section`, `rgp_cloture_refus` (ajout seul, RLS stricte, registre). L'API de lecture
  reconstruit la même forme JSON (contrat inchangé). `rgp_renvoi` renommée `rgp_redirection` (le nom déclenchait la garde E9 « file d'envoi »).
- **Gardes** : catalogue du registre sans partitions, test `chaque_partition_est_couverte_par_son_parent` (parent classé, RLS forcée,
  politique stricte, `TRUNCATE`), exception ciblée de `garde_rls_stricte` pour l'appel de plateforme `acces_traces`.

### Cas limites couverts

Mois suivant créé à la volée (rien n'échoue la veille du 1er), partition interrogée directement sous rôle strict (isolation), reprise
idempotente, trace refusée = lecture refusée, fonction figée après changement de poste, titulaire à la date, parent protégé, élève
majeur (lui seul), agent : sa propre consultation, étab. étranger.

### Tests (lot 4)

`ent-db::tests_acces_g4` (une vingtaine : trace même identifiant que le journal, échec = refus, fonction figée, partitions et
append-only, reprise égale et idempotente, hors périmètre motif requis, remplaçant sans motif, conseil de classe, page famille sans nom,
pas de santé, parent protégé, support éditeur, isolation, motifs personnalisés), `tests_rls::rls_partition_de_traces_interrogee_directement`,
registre `chaque_partition_est_couverte_par_son_parent`, HTTP `tests_acces_g4_http` (401, motifs, 200/409/400, 404), web : 4 tests du
modèle de page. Aucune heure réelle (règle 33).

### Valeurs prudentes retenues (D-069)

- Service = famille de fonction ; périmètre de permanence non modélisé (pas de motif demandé aux permanences).
- Lectures du dossier d'un agent non encore tracées (la page sait afficher `sujet_personne_id`).
- Traces `dossier_droits` non visibles des familles.
- Purge par partition : on garde la plus longue durée (S-541 ≥ 12 mois) ; reprise limitée aux 12 derniers mois.
- S-549 (aucun nom d'agent) : constante de code, non réglable. Désignation seule en accès hors périmètre.
- Filtre de report R-RG28 : lot 5.

### Compteurs

RLS : **+4 tables** (`acc_trace` parent, `acc_motif`, `acc_controle`, `acc_alerte`) **+3** (normalisation de la clôture) ; le catalogue exclut les
partitions (`NOT relispartition`). Registre : +5 (`acc_alerte`, `acc_controle`, `acc_motif`, `acc_purge`, `acc_trace`) +3 (clôture). Routes :
+2 authentifiées. `Action::TOUTES` inchangé (286). Tests à rejouer isolément (`--test-threads=1`) : trois `tests_search_path_t1` expirent
sous forte charge parallèle (verrou de rejeu), verts seuls.

## Lot 5 — contrôle interne, alertes, purge, consultations dans l'extrait

### Fait

- **Actions** (`Action::TOUTES` 286 → **288**) : `acces.consulter_traces` (Direction ; DPO via un rôle D-017 qui la porte) et `acces.controle_ouvrir` ★
  (Direction, passkey récent obligatoire). Aucun défaut pour un DPO : l'établissement crée son rôle.
- **Migrations 2545/2546** (non versées, fin du tableau) : `rgp_defi.extrait_id` nullable et usage `controle` ; `rgp_extrait.consultations_jusqu_au` ;
  tables `acc_differe` (report R-RG28) et `acc_nom_communique` (nom d'agent communiqué, par demande) ; fonctions `acc_trace_partitions_anciennes`,
  `acc_trace_purger_partition`, `acc_trace_noter_purge_differee` ; RLS stricte sur 6 tables `acc_*`.
- **Réglages** avec défauts et historique (`acces_reglages_*`) : seuils d'alerte, rafale, durée de rétention des traces (plancher 12 mois en base).
- **Contrôle interne** (RGP-194/195, R-RG27) : demande → défi passkey → ouverture (`acces.controle_ouvrir`) ; lecture des traces d'un agent sous
  libellés génériques, 12 mois au plus, sans totaux ni classement ; clôture motivée ; tout est tracé (la lecture l'est aussi).
- **Traces d'un sujet** : lisibles seulement dans une demande de droits ouverte ou un contrôle ouvert (lecture tracée).
- **Report R-RG28** : une consultation liée à un signalement est différée (≤ 90 j) sous `droits.arbitrer`, exige un contrôle ouvert de motif
  `signalement` sur l'agent consultant ; filtre appliqué à toutes les lectures (famille, extrait, contrôle).
- **Alertes déterministes** (RGP-196) : rafale, seuil/jour par rôle, adresse protégée hors périmètre, élève sorti ou dossier archivé ; une alerte par
  fenêtre ; notification `acces.alerte` à la direction et au DPO, **jamais à l'agent** ; suite donnée tracée. Balayage horaire (fenêtre 26 h) dans `main.rs`.
- **Purge par partition** (RGP-198, S-541) : plus longue rétention des établissements ; une partition qui porte des traces d'une demande ou d'un
  contrôle ouvert est différée (journal hebdomadaire par partition + alerte `acces.purge_differee` au DPO).
- **Extrait** (RGP-199) : section « consultations » (jusqu'à `consultations_jusqu_au`, horloge injectée à l'assemblage) ; nom d'agent seulement
  sur arbitrage motivé de la direction (`acc_nom_communique`).
- **Écrans** (`/web/rgpd/acces`, lien depuis le guichet) : alertes, contrôles (flux passkey), réglages avec historique ; section « traces » dans la fiche
  de demande.

### Cas limites couverts

Fenêtre d'alerte unique (pas de doublon sur relance) ; seuil strict `>` ; sujet d'une demande non encore assemblée (UNION avec `rgp_demande`) ; trace
protégée ou différée exclue de l'extrait ; purge bloquée par demande/contrôle ouvert ; report non renouvelable en levée anticipée ; ouverture sans défi
valable refusée ; nom d'agent jamais par défaut ; partitions absentes.

### Tests (lot 5)

`ent-db` `tests_acces_g4b.rs` (9 tests, verts avec les 14 du lot 4 : 23/23) ; `ent-api` `g4_http_controles_reglages_alertes` (401/403/404, ouverture sans
défi refusée) ; web : 686 tests, clippy natif + wasm, `cargo metadata --locked` ; garde (rls, registre, migrations, d036, search_path…) : 212/213, le
seul échec est un `statement timeout` sous charge (`t1_gel_du_cahier…`), vert seul.

### Valeurs prudentes retenues (D-069)

- Section consultations de l'extrait arrêtée à `consultations_jusqu_au` : relecture et remise restent stables.
- Traces protégées, différées, de santé ou de droits exclues de l'extrait.
- Lecture de contrôle : tout l'agent, libellés génériques, 12 mois max, sans totaux ni classement.
- Report : ≤ 90 jours, renouvelable par nouvelle décision, aucune levée anticipée.
- Seuil/jour par rôle : direction/admin → seuil direction ; prof → enseignant ; CPE → vie scolaire ; autres → administratif.
- Adresse protégée = hors périmètre ET protégée ; conservation = élève `sorti` ou dossier `archive`.
- Purge : plus longue rétention entre établissements ; journal de report une fois par semaine et par partition.
- DPO : rôle D-017 portant `acces.consulter_traces` ; aucun défaut.

### Compteurs

RLS : **+2 tables** (`acc_differe`, `acc_nom_communique`), soit 6 tables `acc_*` au total ; registre : +2 ; routes : +12 entrées authentifiées ;
`Action::TOUTES` **288** (à recalculer après la fusion de `origin/ENT`).

## Validation finale du palier (règle 39)

- `origin/ENT` : déjà intégré (fusion `d8202f755`) ; `git merge-base HEAD origin/ENT` = tête d'`origin/ENT` au moment de la PR.
  Compteurs recalculés : `Action::TOUTES` **288** (286 + 2), RLS +9 tables au total sur le palier (`acc_*` ×6, normalisation de la clôture ×3),
  tableau `migrate.rs` : 2545 et 2546 en fin, `MIGRATION_TAG` 3808, `migrations_versees.txt` inchangé pour 2545/2546.
- Clippy ×3 (workspace, workspace avec `ent-api/labo,ent-lab/labo`, `ent-types`) : 0 avertissement. Web : clippy natif + wasm, 686 tests,
  `cargo metadata --locked`, `cargo fmt` des trois crates web propre.
- `ent-db` avec labo, 4 tranches `nextest` sur base neuve : 3 132 tests ; les seuls échecs (10 tests : reprise r2/r3, `tests_eleve_e9`,
  `tests_eleve_identifiants`, `tests_personnels_p4/p5`, `tests_portail_p1`, `tests_structure_s7`) comptent des lignes globales ou
  expirent sous charge parallèle ; **rejoués seuls (`-j1`) : 10/10 verts**.
- `ent-api` avec labo : 356 tests. Deux vrais écarts trouvés et corrigés (commit `0a9ae4e23`) : `acces_traces.rs` et `rgpd_extrait.rs`
  manquaient à l'inventaire des routes (`routes_src`) ; l'accès hors périmètre répond désormais 409 « motif requis » (lot 4), compté comme
  **refus** dans les quatre harnais (`separation`, `recette_personas_1`, `_2`, `_2_dossier`) car aucune donnée n'est servie. Binaires
  impactés rejoués : 94/94 verts.
- Sans labo (workspace, nextest) : 1 848 tests, seuls échecs les trois tests de reprise qui comptent des lignes globales (verts seuls).
- Contre-revue `review-sonnet` : **non exécutée** (consigne).

### Restent en file (revue indépendante à faire après la PR)

M2, M4, M5, M6, F1 à F8 (voir plus haut). Faibles du lot 5 : balayage d'alertes horaire sans reprise si le serveur est arrêté plus de 26 h ;
libellés génériques de la lecture de contrôle à relire avec la DPO ; `acc_purge` reste en portée technique.

### Pour le coordinateur

- **2556** est allégée de `rgp_demande_sujet` (créée en 2545 parce qu'elle sert l'assemblage) ; 2556 garde `cons_visa_exclusion` et le reste.
- **S-555** reste à reporter (non traité dans ce palier).
- **Tables partitionnées** : premier usage (`acc_trace`). Gardes adaptées : seul le parent est classé au registre ; le catalogue RLS exclut
  les partitions (`NOT relispartition`) mais un test vérifie RLS, FORCE, politique et déclencheur sur chaque partition ; exception
  `garde_rls_stricte` pour `acces_traces.rs` et `acces_controle.rs` (accès par `self.pool().acquire()` hors transaction).
- Environnement : le conteneur de dev redémarre à l'inactivité (Postgres local à reconstruire) ; sans incidence sur la CI.

---

## Commits par lot (pour les contre-revues)

Liste tenue à jour à chaque lot ; chaque lot compte au moins un commit de code et un commit de livraison.

- Préalable : `4e49258ae` (2544 versée dans `migrations_versees.txt`).
- Lot 1 : `258ae1ef8` (migration 2545, moteur d'extrait, correctifs G3a), `67392f4dd` (livraison), `41c094bad` (résultat de la suite sans labo).
- Lot 2 : `e2c517e43` (marquage, relecture, Pf3/Pf4, routes, écran, livraison).
- Contre-revue des lots 1 et 2 : `44dbfe28a` (rapport) ; correctifs `4c8058436`.
- Lot 3 : `f2a9acffe` (module de remise), `5cbc85ba1` (routes, écrans, livraison).
- Fusion de `origin/ENT` : `d8202f755`.
- Lot 4 : `49a7cfdb0`.
- Lot 5 : `6cc37b942`.
- Correctif d'inventaire et de harnais (validation finale) : `0a9ae4e23`.
