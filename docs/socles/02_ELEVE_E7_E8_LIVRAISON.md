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
6. **Pastille `interne`** : visible de la vie scolaire (CPE) et de l'accueil (décision du pilote, information d'organisation) : actions `eleves.lire`, `internat.gerer` ou `edt.maintenant_lire` (aucune action nouvelle ; ce dernier droit est détenu par direction, secrétariat, CPE et accueil). La tuile « scolarité » suit la même règle (E2) ; l'accueil atteint la synthèse par ce même droit, avec la seule tuile « scolarité » ouverte. PAI reste réservé à `sante.lire` (pas de pastille neutre « protocole »). Test : `interne_visible_de_la_vie_scolaire_et_de_l_accueil_sans_rien_ouvrir_d_autre`. La migration 299 (propre à ce lot, non encore versée sur ENT) a été modifiée en conséquence ; une base locale déjà migrée doit être remise à jour.
7. **Lecture des coordonnées par le personnel** : `eleves.identite.modifier` (Admin, Direction, Administratif), pas G2 (l'infirmerie ne les lit pas).
8. **Audit** : `audit.lire` est détenu par Administratif en plus de Admin et Direction.
9. Les pastilles ne dépendent que des tables existantes ; `nouvel_arrivant`, `arrive_en_cours_annee`, `majorite_proche`, `modalite_particuliere` (R-E9) restent à ajouter au catalogue.
10. Suite complète ent-db non rejouée en local (≈ 3 h en série) : lots ciblés ci-dessous, la CI fait foi.

## 5. Tests
Local, PostgreSQL 16, `--test-threads=1`, Rust **1.99.0** :
- nouveaux : `tests_eleve_protection` 14, `tests_eleve_indicateurs` 6, `tests_eleve_revue_securite` 14 (correctifs de la revue, §7), `domain-eleve` 15 (298 au total, verts) ; `ent-api` : 2 tests unitaires (`eleve_garde`) ;
- `ent-db` lot A (`tests_eleve_*`, RLS, registre, conservation, extrait, D-036, champs chiffrés) : 173 verts ; lot B (administratif, documents, communication, formulaires, santé, rôles, inscriptions, dossier) : 174 verts ; série finale après fusion (E7, E8, registre, RLS, extrait, conservation, D-036) : 73 verts ;
- `ent-api` : `separation` 55 (54 + le test paramétré E6 ; deux passages avant la revue, dont après fusion), `recette_securite_3` 10, `fichiers_depot` 7, `fichiers_liens` 10, `recette_personas_1` 14, `recette_personas_2` 6, `audit_correctifs_1` 5 : verts ;
- `cargo +1.99.0 clippy --workspace --tests --features ent-db/labo` : **0 avertissement**, rustc compris ;
- `cargo +1.99.0 metadata --locked` : racine et `web/` OK ; aucune dépendance nouvelle ;
- aucun test ignoré, aucun test à l'horloge réelle (instants passés aux commandes `*_a`, mesures datées de janvier 2026, pastilles datées recalculées à un jour explicite) ; jamais `cargo fmt --all`.
- Les tests modifiés d'autres lots : aucun (la route photo héritée garde son comportement pour le personnel ; la famille propose désormais).

## 6. Réponses du pilote (2026-11-17)
- (a) **Oui** : `photo`, `email_eleve` et `mobile_eleve` entrent au catalogue E5 — lot suivant (avec E9).
- (b) **Oui** à une pastille neutre « protocole » pour la vie scolaire : elle signale qu'un protocole existe et qu'il faut s'adresser à l'infirmerie, sans aucun contenu de santé — lot suivant.
- (c) **Oui** à un lot dédié pour ELV-44 et ELV-46 ; plage de migrations réservée au lancement du lot.
- Écart 6 : la pastille `interne` est visible de la vie scolaire (CPE) et de l'accueil ; PAI reste réservé à `sante.lire`. **Fait** (voir écart 6, §4).

## 7. Correctifs de la revue de sécurité indépendante (2 critiques, 6 élevés, 6 moyens, 1 faible)
Un test par point, dans `ent-db/src/tests_eleve_revue_securite.rs` (14 tests) et `ent-api/tests/separation.rs` (E6, paramétré). Niveau : implémenté, testé en local ; la CI fait foi.

| Point | Correctif | Test |
|---|---|---|
| **C1** journal de l'élève | `list_journal_eleve_for_actor` : garde « élève visible » en tête ; lignes `eleve_*` (dont `eleve_sensible`, `eleve_photo`, `eleve_coordonnee`) réservées à G6 ; `list_journal_domaine_for_actor` écarte les `eleve_id` protégés | `c1_le_journal_d_un_eleve_protege_est_ferme_au_tiers` |
| **C2** élèves sans compte | filtre SQL avant le `LIMIT` dans `list_eleves_sans_compte_for_actor` (nouveau paramètre acteur/rôle), garde dans l'invitation (`POST /eleves/{id}/compte`, aucun e-mail) et dans le provisionnement en masse | `c2_sans_compte_et_invitation_ignorent_les_proteges` |
| **E1** consultations | `consultations_pour_acteur` : garde (404) | `e1_les_consultations_…` |
| **E2** pastille `majeur` | seul le fait est publié (`valeur_date` vide, `du` = jour courant) ; une pastille n'est visible que si la tuile de son domaine l'est (vérifié pour tous les rôles, dont `interne` et `redoublant`) | `e2_les_pastilles_ne_devinent_pas_la_naissance_et_suivent_leur_tuile` |
| **E3** photos manquantes | `e.statut` (et non `e.status`), `coalesce` pour un `statut` NULL ; test positif | `e3_un_eleve_sans_photo_…` |
| **E4** (décision du pilote) | portée pédagogique conservée comme « non tiers » ; `masquer_photo` et `adresse_protegee` appliquées à ces non-tiers (photo non servie et absente du trombinoscope ; les personnes liées à l'élève, donc leurs adresses F2, sont fermées à qui n'a pas G6, même avec un lien direct) ; l'alerte `alerte_consultation` est désormais décidée **dans** `tracer_consultation` : toute lecture d'un élève protégé par un membre du personnel sans G6 alerte (même définition que `proteges_caches`), jamais G6 | `e4_alerte_pour_toute_lecture_…`, `m3_adresse_protegee_ferme_les_personnes_liees_aux_non_g6` |
| **E5** conflit d'intérêts | `exiger_pas_de_conflit` sur toutes les écritures de photo (proposer, valider, rejeter, retirer, refus, levée) et de coordonnées (ajout, retrait, usages, vérification) ; un agent G6 en conflit ne lit pas le motif descellé | `e5_le_conflit_d_interet_bloque_…` |
| **E6** autres routes `/eleves/{id}/…` | couche HTTP `eleve_garde::exiger_eleve_visible` (404, jamais 403) sur **toutes** les routes `/eleves/<uuid>[/…]` (112 déclarations, toutes méthodes) ; garde aussi dans `fiche_urgence_for_actor` et `pai_complet_for_actor` | `separation::e6_toute_route_eleve_…` (5 rôles tiers × toutes les routes, contrôle positif avant, G6, puis levée) ; `e6_fiche_d_urgence_et_pai_complet_…` ; test unitaire du chemin |
| **M1** refus de photo | la famille ne pose ni ne lève un refus sur un majeur ou émancipé (404) ; elle ne lève que le refus qu'elle a posé (même source) ; le personnel lève tout | `m1_la_famille_n_a_plus_de_prise_…` |
| **M2** usages de coordonnée | changement d'usages journalisé (`eleve_coordonnee` / `usages_modifies`, sans valeur) ; l'usage `compte` est posé ou retiré par l'élève seul (le personnel : refusé) | `m2_les_usages_sont_journalises_…` |
| **M3** `hors_trombinoscope_exporte` | sans effet tant qu'aucun export n'existe : documenté **« effet en E11 »** (DTO, ce rapport) ; seul `eleves_hors_export` l'expose | `eleves_hors_export` (existant) |
| **M4** domaine de journal unique | toutes les lectures sensibles (identité, identifiants, scolarité, propositions, pastilles, listes par pastille) écrivent en domaine `consultation`, le code consulté dans l'action (`liste_pastille:pai_actif`…) | `m4_toutes_les_lectures_sensibles_…` (exhaustivité) |
| **M5** filtre avant la limite | protégés écartés en SQL **avant** le `LIMIT` : listes par pastille, file des photos à valider, photos manquantes, propositions | `m5_les_proteges_sont_ecartes_avant_la_limite` |
| **M6** module santé | les pastilles et la tuile « santé » sont masquées, et la liste par pastille de santé refusée, tant que le module n'est pas activé | `m6_sans_module_sante_…` |
| **F1** photo | `GET /eleves/{id}/photo` sert le type MIME réel (signature : jpeg, png, webp ; sinon flux binaire) avec `X-Content-Type-Options: nosniff` | `eleve_garde::mime_de_photo_par_signature` |

Effets de bord assumés : `list_eleves_sans_compte_for_actor` prend l'acteur et le rôle ; deux tests existants ajustés (usages « compte » posés par le personnel, alerte pour tout non-G6 et plus seulement l'ouverture de fiche).

### Suites notées (non faites dans ce lot, à la demande du pilote)
- **M7** : plafond journalier des SMS de vérification de coordonnée (le plafond actuel est de 3 codes par heure et par coordonnée).
- **F2** : photo orpheline dans le stockage si la transaction échoue après l'écriture de l'objet ; réencodage sans EXIF (métadonnées de géolocalisation).
- **F3** : `ent_eleve.statut` NULL (traité par `coalesce` dans « photos manquantes » ; la contrainte reste à poser).
- **F4** : tracer la lecture de la liste des élèves protégés (`eleves_proteges_pour_acteur`).
- **F5** : valider les contraintes `NOT VALID` posées par 298 (migration dédiée après assainissement des données existantes).
- Couche HTTP : seules les routes `/eleves/<uuid>…` sont gardées centralement ; les routes de santé ou de vie scolaire par identifiant d'élève situées sous d'autres préfixes (`/sante/…`, `/accompagnement/…`) gardent leur contrôle propre (à inventorier en E9).
