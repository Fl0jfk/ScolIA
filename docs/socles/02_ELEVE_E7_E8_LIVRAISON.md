# Socle 2 « l'élève » — lots E7 (confidentialité, photos, coordonnées) et E8 (pastilles, synthèse) : livraison

Branche `ent-socle-2-code-e7e8` (base `origin/ENT`). Conception : `docs/socles/02_ELEVE.md` §7.6, §7.7, §7.11, §10.4, §10.5, ELV-15/16/17/42, R-E9. Niveau : **implémenté, testé en local** (CI GitHub à venir). Ni validé, ni prêt pilote.

## 1. Livré

### E7
- **ELV-15 confidentialité renforcée** (`ent-db/src/eleve_confidentialite.rs`) : poser, modifier (lever + poser dans la même transaction), lever ; motif **scellé** (AES-256-GCM, jamais en clair en base, ni dans l'événement `eleve.confidentialite_changee`, ni dans le journal) ; effets minimaux non désactivables (`masquer_photo`, `hors_annuaire`) toujours posés ; mesures facultatives `hors_trombinoscope_exporte`, `adresse_protegee`, `alerte_consultation` ; période bornée ; une seule mesure en vigueur ; conflit d'intérêts (R-E10) refusé ; fiche absorbée refusée ; mesure immuable et non supprimable (déclencheur).
- **Élève protégé invisible d'un tiers** (ni G6 ni lien direct) sur : fiche (`get_eleve_for_actor` / vue), listes (`vues_eleves_pour_acteur`), trombinoscope, recherche (E6), doublons (E6), identité (historique G5), identifiants (G3), scolarité (G1), propositions (liste, détail, décision), photos, coordonnées, synthèse, listes par pastille et compteurs. Réponse = élève absent (`404`), jamais `403`.
- **Journal de consultation** (`ent_journal_modif`, ajout seul, domaine `consultation`) : ouverture d'une fiche protégée, lecture de l'état civil, des identifiants, des coordonnées, du motif, des pastilles sensibles, des listes par pastille ; `alerte_consultation` publie `eleve.consultation_protegee` pour un tiers (pas pour G6). Lecture : `GET /eleves/{id}/consultations` (`audit.lire`). Les traces E2/E3/E4 existantes (domaines `identite`, `identifiants`, `scolarite`) sont conservées.
- **Scellement des versions d'identité sensibles non courantes** (`eleve_identite_scellee.rs`, table `ent_eleve_identite_scellee`) : valeurs vidées (`[scellé]`), une ligne chiffrée par champ ; scellement dans la transaction de chaque écriture de version et dans la tâche quotidienne (reprise de l'existant) ; descellement transparent pour le code d'identité et pour les attestations ; G6 relit, G5 sans G6 ne voit pas la version ; la base refuse toute autre modification.
- **ELV-16 photos** (`eleve_photo.rs`) : proposer / valider / rejeter / retirer, une photo validée au plus, objet immuable, source déduite de l'acteur, famille et élève proposent seulement, projection `ent_eleve.photo_key`/`photo_id` par déclencheur ; file à valider ; « photos manquantes » ; **ELV-42** refus de la photo administrative (photo retirée, propositions rejetées, silhouette, révocable). La route héritée `POST /eleves/{id}/photo` passe désormais par ce circuit (plus de contournement).
- **ELV-17 coordonnées propres** (`eleve_coordonnee.rs`) : e-mail et mobile de l'élève ; la famille propose (secrétariat décide) tant que l'élève est mineur et non émancipé (404 ensuite) ; l'élève gère les siennes ; vérification par code (empreinte HMAC, 5 essais, 15 min, 3 codes/heure, envoi échoué = code annulé) ; usages `compte`/`urgence` réservés aux coordonnées vérifiées ; `coordonnees_eleve_pour_usage` = seule porte pour un envoi ; aucune valeur dans événements ni journal.
- **RLS** sur les 5 nouvelles tables ; `ent_eleve_confidentialite` et colonnes scellées ajoutées à `CHAMPS_CHIFFRES`.
- **Vérifié, déjà corrigé** : `outbox.rs` pose bien `app.etablissement_id` (note E7 de la conception caduque).

### E8
- **Catalogue** : `domain_eleve::CATALOGUE_INDICATEURS` (8 pastilles : `interne`, `pai_actif`, `sans_ine`, `ine_conteste`, `confidentialite_renforcee`, `majeur`, `en_mobilite`, `redoublant`) = table `ent_catalogue_indicateur` (test d'égalité, actions vérifiées contre `Action`).
- **Consommateur D0** `e8.indicateurs` (outbox, rejouable) : recalcule les pastilles de l'élève concerné (et des deux fiches d'une fusion) depuis les tables sources par UNE fonction SQL ; tâche quotidienne pour celles qui dépendent de la date ; `reconstruire_indicateurs` pour la reprise.
- **Contrat des tuiles** (`CONTRATS_TUILES` : identité, identifiants, scolarité, santé, confidentialité) : une tuile sans droit est masquée avec sa raison et ne laisse rien filtrer.
- **Trois pastilles de référence alimentées par événements** (interne ← `eleve.regime_changed`, PAI ← `infirmerie.pai`, sans INE ← `eleve.created` / `eleve.identifiant_change`), filtrées par les droits de l'acteur sur l'élève.
- **API pour E9** : `GET /eleves/{id}/synthese`, `/eleves/{id}/pastilles`, `/eleves/pastilles` (compteurs), `/eleves/pastilles/catalogue`, `/eleves/pastilles/{code}`.

### Routes ajoutées (22)
`ent-api/src/eleve_protection.rs` (17 déclarations) et `eleve_synthese.rs` (5) ; DTO `ent-types::eleve_protection` et `eleve_synthese`. Aucune écriture ne répond 2xx sur une cible absente ou d'un autre établissement (matrice de séparation, 12 cas « sur B »).

## 2. Migrations
Plage 290–299 : **298** `eleve_protection_photo_coordonnees`, **299** `eleve_indicateurs`. **La plage 290–299 est désormais épuisée** ; aucune autre n'a été prise. `MIGRATION_TAG` inchangé (plus haute = numéros > 299). Aucune migration existante modifiée ; pas de JSON métier.

## 3. Compteurs
Recalculés après fusion d'`origin/ENT` (96c287c5, socle 18 FO2-FO3), règle 28 :
- **RLS** (`tests_rls.rs`) : **715** tables = base ENT (+18 du socle 18) **+ 5** (298) **+ 1** (299). `ent_catalogue_indicateur` est global, sans RLS.
- **Registre des données** : **755** entrées, **+7** (6 `dossier_eleve` + le catalogue global) ; chaque table nouvelle classée, `ent_eleve_coordonnee_verification` neutralisée à la restauration.
- **RGPD / conservation / extrait de la personne (R-RG14)** : tables en domaine `dossier_eleve`, donc couvertes par le fournisseur `eleve` et la catégorie `scol.dossier` ; tests verts.
- **`TABLES_ELEVE`** : 168 entrées, **+6**, toutes `Reste` avec raison écrite ; domaines de fusion nommés.
- **Matrice de séparation** : +31 cas ; inventaire `SOURCES` / `NOMS_SOURCES` : **63** (61 d'ENT + `eleve_protection.rs`, `eleve_synthese.rs`).
- **`Action::TOUTES`** : **253**, inchangé par ce lot (aucune action nouvelle : `eleves.confidentiel.gerer`, `eleves.photo.gerer`, `eleves.identite.modifier`, `audit.lire` existaient).
- **Migrations** : 298, 299 ; `MIGRATION_TAG` inchangé. **`CHAMPS_CHIFFRES`** : +2 colonnes.
- **Événements** : +3 (`eleve.confidentialite_changee`, `eleve.consultation_protegee`, `eleve.coordonnees_modifiees`).

## 4. Écarts, limites, questions pour le pilote / Florian
1. **Rotation de clé `enc:v2:`** (annoncée en E2/E7) : non faite — elle touche le coffre commun (`ent-auth`), hors lot.
2. **ELV-44 bris de glace** et **ELV-46 lien personnel** : listés « E1/E7 » dans la conception mais hors de la liste demandée ; non faits.
3. **Paramétrage photo / e-mail / mobile par champ** (`ent_param_eleve_champ`) : le catalogue E5 est limité à l'identité ; la famille et l'élève peuvent toujours proposer (validation humaine obligatoire pour la photo, code pour la coordonnée). À trancher : ajouter ces champs au catalogue E5.
4. **Conservation des anciennes photos (N mois)** : aucune purge exécutée (durées à valider par le DPO).
5. **Effets hors socle 2** de `adresse_protegee` et de l'envoi de messages (socle 4) et exports nominatifs (E11) : fournis seulement `eleves_hors_export` et le filtrage ; l'exportateur doit les appeler.
6. **Pastille `interne`** : action `eleves.lire` ou `internat.gerer` ; la CPE et l'accueil ne la voient donc pas par défaut (cohérent avec l'écart n° 1 d'E1). Pastille PAI : `sante.lire` seulement (pas de pastille neutre « protocole »).
7. **Lecture des coordonnées par le personnel** : `eleves.identite.modifier` (Admin, Direction, Administratif), pas G2 (l'infirmerie ne les lit pas).
8. **Audit** : `audit.lire` est détenu par Administratif en plus de Admin et Direction.
9. Les pastilles ne dépendent que des tables existantes ; `nouvel_arrivant`, `arrive_en_cours_annee`, `majorite_proche`, `modalite_particuliere` (R-E9) restent à ajouter au catalogue.
10. Suite complète ent-db non rejouée en local (≈ 3 h en série) : lots ciblés ci-dessous, la CI fait foi.

## 5. Tests
Local, PostgreSQL 16, `--test-threads=1`, Rust **1.99.0** :
- nouveaux : `tests_eleve_protection` 14, `tests_eleve_indicateurs` 6, `domain-eleve` 15 (298 au total, verts) ;
- `ent-db` lot A (`tests_eleve_*`, RLS, registre, conservation, extrait, D-036, champs chiffrés) : 173 verts ; lot B (administratif, documents, communication, formulaires, santé, rôles, inscriptions, dossier) : 174 verts ; série finale après fusion (E7, E8, registre, RLS, extrait, conservation, D-036) : 73 verts ;
- `ent-api` : `separation` 54 (deux passages, dont après fusion), `recette_securite_3` 10, `fichiers_depot` 7, `fichiers_liens` 10, `recette_personas_1` 14, `recette_personas_2` 6, `audit_correctifs_1` 5 : verts ;
- `cargo +1.99.0 clippy --workspace --tests --features ent-db/labo` : **0 avertissement**, rustc compris ;
- `cargo +1.99.0 metadata --locked` : racine et `web/` OK ; aucune dépendance nouvelle ;
- aucun test ignoré, aucun test à l'horloge réelle (instants passés aux commandes `*_a`, mesures datées de janvier 2026, pastilles datées recalculées à un jour explicite) ; jamais `cargo fmt --all`.
- Les tests modifiés d'autres lots : aucun (la route photo héritée garde son comportement pour le personnel ; la famille propose désormais).
