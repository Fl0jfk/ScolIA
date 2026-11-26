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
