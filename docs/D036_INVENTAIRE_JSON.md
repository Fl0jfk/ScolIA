# D-036 — inventaire des colonnes JSON/JSONB (2026-11-10)

**Règle (D-036)** : plus aucun JSON qui sert de mémoire. Permis : JSON comme format d'échange de l'API, instantané figé d'audit ou d'historique (jamais relu comme source de vérité), état technique opaque d'une bibliothèque (passkey `webauthn-rs`), charge utile transitoire d'une file d'envoi.

**Base de l'inventaire** : `origin/ENT` (migrations 001 à 480, puis 560–563 ; lot 2 : 661–667) ; 26 colonnes JSON/JSONB déclarées dans les migrations (`grep -in json ent-rust/migrations/*.sql`, hors fonctions `to_jsonb` des gardes). Branche `ent-d036-normalisation-json`, migrations 560–563.

Garde automatique : le test `migrate::tests::aucune_colonne_json_hors_liste_d036` (`ent-db`) échoue si une colonne JSON/JSONB apparaît dans une migration sans être déclarée dans `COLONNES_JSON_D036` ; il échoue aussi si une entrée déclarée a disparu.

## Tableau

Statuts : **exception** = permise par D-036 ; **retirée** = reprise faite dans une table typée (migration indiquée) puis colonne supprimée (668 ou 669) ; **à normaliser** = dette déclarée, lot suivant ; **laissée** = refonte déjà prévue ailleurs.

| Migration | Table.colonne | Usage dans le code | Statut | Justification / traitement |
|---|---|---|---|---|
| 001 | `ent_metier_event.payload` | écrit à la publication, lu par le diffuseur/outbox et les abonnés | exception | charge utile transitoire d'événement ; format d'échange |
| 060 | `ent_edt_import.report_json` | écrit à l'import, affiché | exception | rapport d'import figé |
| 068 | `ent_journal_modif.payload` | écrit (ajout seul), consulté en audit | exception | journal d'audit en ajout seul |
| 089 | `ent_passkey_credential.passkey` | `webauthn-rs` | exception | état opaque de la bibliothèque |
| 089 | `ent_passkey_challenge.state` | `webauthn-rs`, transitoire | exception | état opaque de la bibliothèque |
| 123 | `ent_personne_fusion.instantane` | écrit à la fusion, **relu à l'annulation** (`dossier_personnes.rs`) | **retirée** (669) ; reprise en 667 | **relu à l'annulation** : ce n'est pas un instantané d'audit figé mais la mémoire de la défusion → colonnes typées (`compte_transfere`, `agent_deplace`, `agents_fusionnes`), `ent_personne_fusion_objet`, copies typées `…_lien_supprime` / `…_contact_supprime` |
| 134 | `ent_signalement_assiduite.dossier` | écrit à la proposition, lu et renvoyé par l'API | **retirée** (668) ; reprise en 563 | détail dans `ent_signalement_assiduite_detail` ; JSON d'API reconstruit à la lecture |
| 144 | `ent_import_ligne.donnees` | écrit à l'import, relu à la validation/exécution (`imports.rs`) | **retirée** (669) ; reprise en 664 | dix-sept colonnes `TEXT` (une par colonne du format `scolia_eleves_v1`) ; le JSON d'API/transit est reconstruit (`ent_import_ligne_donnees`) |
| 150 | `ent_cdt_historique.instantane` | écrit à chaque version, affiché dans l'historique | exception | instantané d'historique, jamais source de vérité |
| 172 | `ent_delegation.droits` | lu par le moteur de droits à chaque requête | **retirée** (668) ; reprise en 560 | `ent_delegation_droit` + `ent_delegation_droit_cible` ; le plus à risque (droits) |
| 182 | `ent_bulletin.contenu` | écrit à la publication, affiché | exception | copie figée d'un bulletin publié (nommée par D-036) |
| 240 | `ent_eleve_orphelin.ligne` | écrit au diagnostic, consulté | exception | ligne mise en quarantaine : instantané figé |
| 250 | `ent_inscription_dossier.resolutions` | écrit à la validation, jamais relu | **retirée** (668) ; reprise en 561 | `ent_inscription_resolution` (élève et responsables) |
| 275 | `ent_personne_preference.joignable` | lu/écrit, validé (`dossier_contact.rs`) | **retirée** (669) ; reprise en 665 | `ent_personne_plage_joignable` (+ `joignable_definie`) |
| 275 | `ent_personne_preference_version.joignable` | historique des plages | **retirée** (669) ; reprise en 665 | `ent_personne_preference_version_plage` |
| 292 | `ent_eleve_fusion.conflits_resolus` | relue à l'annulation (`eleve_fusion.rs`) | **retirée** (669) ; reprise en 666 | `ent_eleve_fusion_conflit` + `_conflit_cle` |
| 292 | `ent_eleve_fusion.resume` | écrit à la fusion, affiché | exception | résumé figé d'une fusion (registre en ajout seul) |
| 292 | `ent_eleve_fusion.doublons_touches` | relue à l'annulation (`jsonb_array_elements`) | **retirée** (669) ; reprise en 666 | `ent_eleve_fusion_doublon` |
| 292 | `ent_eleve_fusion_ligne.cle` | relue (`jsonb_to_recordset`) pour repointer à l'annulation | **retirée** (669) ; reprise en 666 | `ent_eleve_fusion_ligne_cle` : relue pour repointer les lignes → mémoire, pas audit figé |
| 302 | `ent_inscription_campagne.regle_relance` | lue pour planifier les relances | laissée | refonte prévue au socle finances 24–25 (lot F0, branche `ent-socle-finances-f0`) |
| 302 | `ent_inscription_campagne_version.instantane` | écrit à chaque version, affiché | exception | instantané d'historique de campagne |
| 304 | `ent_inscription_dossier.scolarite_actuelle` | **aucun code** ne la lit ni ne l'écrit | **retirée** (668) ; reprise en 562 | cinq colonnes `scol_*` ; ancienne colonne inutilisée |
| 332 | `ent_etablissement_profil.jours_ouvres` | lue par le calendrier (`structure_calendrier.rs`), API | **retirée** (669) ; reprise en 662 | `ent_etablissement_jour_ouvre` ; semaine type reconstruite au format de l'API (`ent_etablissement_jours_ouvres`) |
| 332 | `ent_parametre.valeur` | lue/écrite via le registre de paramètres | **retirée** (669) ; reprise en 663 | `type_param` + `valeur_entier` / `valeur_booleen` / `valeur_heure` (types du registre du code) |
| 332 | `ent_structure_journal.avant` | écrit (ajout seul) | exception | journal de structure en ajout seul |
| 332 | `ent_structure_journal.apres` | écrit (ajout seul) | exception | idem |

Bilan : 12 exceptions, 13 colonnes normalisées **et retirées** (lots 1 et 2, migrations 668–669), 1 laissée (finances, `regle_relance`), 0 à normaliser. `COLONNES_JSON_D036` ne contient plus que les exceptions et `regle_relance` (catégorie `dette`).

## Lots réalisés (migrations 560–563)

- **560** `delegation_droits_normalises` : `ent_delegation_droit` (action, portée, ordre, unique par action/portée) et `ent_delegation_droit_cible` (cibles désignées), clés étrangères en cascade, RLS permissive. Code : `delegation.rs` (`charger_droits`, `ecrire_droits_tx`), `edt_remplacement.rs`. Le moteur de droits lit les tables.
- **561** `inscription_resolution` : une ligne pour l'élève (nouveau/existant, confirmation des responsables), une par responsable (personne nouvelle/existante, foyer nouveau/existant), contrainte de forme, unicité par dossier, RLS permissive.
- **562** `inscription_scolarite_actuelle` : colonnes `scol_etablissement_libelle`, `scol_uai` (format UAI), `scol_type`, `scol_niveau`, `scol_annee`.
- **563** `signalement_assiduite_detail` : détail jour/période/minutes ; le « dossier » renvoyé par l'API est reconstruit (`SELECT_SIGNALEMENT`), format d'échange inchangé.

Propriétés communes : reprise **idempotente** (les migrations sont rejouées à chaque démarrage) — seules les lignes sans équivalent typé sont reprises, valeurs invalides ignorées (jamais d'échec de démarrage) ; l'ancienne colonne n'est plus écrite par le code (`[]`, `{}` ou NULL), et pour 560 et 563 un déclencheur la rend **en lecture seule** (SQLSTATE 42501).

## Lot 2 (migrations 661–667, branche `ent-d036-normalisation-json-2`)

- **661** corrige le lot 1 : **E2** références composites `(etablissement_id, id)` (droits et cibles de délégation, résolutions, détail des signalements ; fonctions `ent_d036_contrainte` / `ent_d036_retirer_fk_simple`, clé posée `NOT VALID` puis validée sans jamais bloquer le démarrage) ; **E3** plus de défaut `gen_random_uuid()` sur `ent_inscription_resolution` (l'application fournit l'identifiant) ; **E6** `cree_le` sur les trois tables filles (les exceptions « sans horodatage » du registre sont retirées).
- **662** `ent_etablissement_jour_ouvre` ; **663** colonnes typées de `ent_parametre` ; **664** colonnes typées de `ent_import_ligne` (+ références composites) ; **665** plages de joignabilité (état courant et historique) ; **666** annulation de fusion d'élèves ; **667** annulation de fusion de personnes.
- Mêmes propriétés que le lot 1 : reprise idempotente (rejouée à chaque démarrage, valeurs invalides ignorées), ancienne colonne facultative et en **lecture seule** (SQLSTATE 42501), format de l'API inchangé (JSON reconstruit en lecture), RLS par `ent_rls_poser`, tables classées au registre (`registre_donnees.rs`), compte RLS de `tests_rls.rs` mis à jour.
- **Fusions : ce n'est pas de l'audit figé.** `ent_eleve_fusion_ligne.cle` (relue par `jsonb_to_recordset` pour repointer les lignes) et `ent_personne_fusion.instantane` (relu pour restaurer liens, contacts, compte et agent) servent de **source de vérité à l'annulation** : D-036 ne les permet donc pas comme exceptions. Seul `ent_eleve_fusion.resume` reste une exception (résumé affiché, jamais relu pour agir). Les tables de fusion d'élèves sont en ajout seul. Pour les personnes, les lignes en doublon supprimées sont copiées **colonne pour colonne** (`…_lien_supprime`, `…_contact_supprime`) ; un test de garde (`tests_d036`) échoue si ces copies divergent de `ent_eleve_lien` / `ent_eleve_contact_urgence`.
- Identifiants des lignes reprises par migration : `gen_random_uuid()` ou, quand deux instructions doivent se retrouver (conflits, objets déplacés), un `md5` déterministe (les instructions d'une migration peuvent passer par des connexions différentes : pas de table temporaire).
- Compromis connus : `ent_import_ligne` limite chaque valeur à 4000 caractères (refus explicite à la lecture du CSV) ; « joignable : liste vide » reste distingué de « jamais renseigné » par `joignable_definie` ; la référence des objets déplacés d'une fusion de personnes est polymorphe (sans clé étrangère, ces lignes pouvant disparaître ensuite).
- Le retrait des 13 colonnes (`DROP COLUMN`, déclencheurs, entrées `retrait`) reste un lot ultérieur, après une release de coexistence.

## Lot 3 (migrations 668–670, branche `ent-d036-retrait-colonnes`) — retrait des 13 colonnes

Feu vert du pilote (aucune base ScolIA n'est déployée : pas de release de coexistence nécessaire).

- **668** (lot 1) : `ent_delegation.droits`, `ent_inscription_dossier.resolutions` et `scolarite_actuelle`, `ent_signalement_assiduite.dossier`. **669** (lot 2) : `jours_ouvres`, `ent_parametre.valeur`, `ent_import_ligne.donnees`, `joignable` (×2), `conflits_resolus`, `doublons_touches`, `ent_eleve_fusion_ligne.cle`, `ent_personne_fusion.instantane`. Les déclencheurs de lecture seule et leurs fonctions partent avec les colonnes (un déclencheur « UPDATE OF colonne » empêche le retrait). Tout est en `IF EXISTS` : rejouable.
- **Obstacle et parade.** Les migrations sont rejouées à chaque démarrage. Les fichiers de reprise (560–563, 662–667) lisent les anciennes colonnes, et 250, 275, 304 refont un `ADD COLUMN IF NOT EXISTS` : après un `DROP COLUMN`, le démarrage suivant aurait échoué (vérifié : « column d.droits does not exist ») ou ressuscité une colonne. Sans modifier aucune migration existante (règle 15 de PILOTAGE), la migration **670** crée `ent_migration_figee` et y inscrit ces 13 fichiers ; le lanceur (`migrate.rs`, tableau désormais nommé par la macro `m!`) **saute** les fichiers figés. Base neuve : tout est joué dans l'ordre, puis 670 fige ; base existante : les fichiers ont déjà été joués, ils ne le sont plus.
- **Test de garde** (`aucune_colonne_json_hors_liste_d036`) : le schéma net retranche les `ALTER TABLE … DROP COLUMN` ; les 13 entrées « retrait » ont été supprimées de `COLONNES_JSON_D036`. `tests_d036` vérifie que les 13 colonnes et leurs fonctions ont disparu, que rejouer deux fois les migrations n'en ressuscite aucune, que 13 fichiers sont figés, et conserve la garde de dérive des copies de fusion de personnes.
- **Code** : la seule écriture restante dans une ancienne colonne (`dossier = '{}'` du signalement d'assiduité, `discipline_vs.rs`) est retirée.
- **Garde-fous du figeage** (conditions du pilote) : le lanceur journalise chaque fichier sauté (`tracing::info!`) et refuse un nom inconnu dans `ent_migration_figee` (`MigrateError::FigeInconnu`). Tests (`tests_d036`, bases jetables) : base neuve, trois démarrages successifs, base migrée jusqu'à 667 avec lignes héritées puis passage à 670 (la valeur du paramètre et la semaine type sont reprises avant le retrait), refus d'un nom inconnu, et existence de toutes les tables créées par les fichiers figés.
- **Écart assumé à la règle « ne jamais figer un fichier qui crée une table »** : 560–563 et 662–667 créent leurs tables typées **et** lisent l'ancienne colonne, et ne peuvent pas être scindés sans être modifiés. Ils sont figés en bloc, avec le test d'existence des tables ci-dessus comme garde. Règle et procédure : `docs/PILOTAGE.md`, « Migrations figées ».
- **Pour une future suppression de colonne** : une migration de retrait (`DROP … IF EXISTS`), puis une migration qui inscrit dans `ent_migration_figee` les fichiers qui la lisent ou la recréent, sur décision du pilote.

## À faire dans un lot ultérieur

1. ~~Retrait des anciennes colonnes~~ : fait (lot 3, migrations 668–670).
2. `regle_relance` : suivre le lot F0 / socle finances ; ne pas la normaliser ici.
3. Côté Next.js (D-036 §3) : hors périmètre (Next.js intouchable) ; inventaire T0 à part.

## Niveau de preuve

- **Lot 1 (560–563)** : validé par la CI.
- **Lot 2 (661–667)** : CI verte (run 43010995060 sur `c376da46`, 38 jobs : clippy bloquant, séparation labo/production, 20 tranches avec labo, 12 tranches en RLS stricte, tests sans labo) ; reprise testée sur des lignes synthétiques (à rejouer sur une copie anonymisée avant toute bascule).
- **Lot 3 (668–670)** : implémenté et compilé ; voir la CI de la branche `ent-d036-retrait-colonnes` pour le SHA vert. Non validé par Florian, non pilote.
