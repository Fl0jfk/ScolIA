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

## Commits par lot (pour les contre-revues)

Liste tenue à jour à chaque lot ; chaque lot compte au moins un commit de code et un commit de livraison.

- Préalable : `4e49258ae` (2544 versée dans `migrations_versees.txt`).
- Lot 1 : `258ae1ef8` (migration 2545, moteur d'extrait, correctifs G3a), `67392f4dd` (livraison), `41c094bad` (résultat de la suite sans labo).
- Lot 2 : `e2c517e43` (marquage, relecture, Pf3/Pf4, routes, écran, livraison).
