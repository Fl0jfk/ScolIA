# D-036 — inventaire des colonnes JSON/JSONB (2026-11-10)

**Règle (D-036)** : plus aucun JSON qui sert de mémoire. Permis : JSON comme format d'échange de l'API, instantané figé d'audit ou d'historique (jamais relu comme source de vérité), état technique opaque d'une bibliothèque (passkey `webauthn-rs`), charge utile transitoire d'une file d'envoi.

**Base de l'inventaire** : `origin/ENT` (migrations 001 à 480, puis 560–563 ; lot 2 : 661–667) ; 26 colonnes JSON/JSONB déclarées dans les migrations (`grep -in json ent-rust/migrations/*.sql`, hors fonctions `to_jsonb` des gardes). Branche `ent-d036-normalisation-json`, migrations 560–563.

Garde automatique : le test `migrate::tests::aucune_colonne_json_hors_liste_d036` (`ent-db`) échoue si une colonne JSON/JSONB apparaît dans une migration sans être déclarée dans `COLONNES_JSON_D036` ; il échoue aussi si une entrée déclarée a disparu.

## Tableau

Statuts : **exception** = permise par D-036 ; **normalisée** = reprise faite (migration indiquée), ancienne colonne conservée en lecture seule jusqu'au lot de retrait ; **à normaliser** = dette déclarée, lot suivant ; **laissée** = refonte déjà prévue ailleurs.

| Migration | Table.colonne | Usage dans le code | Statut | Justification / traitement |
|---|---|---|---|---|
| 001 | `ent_metier_event.payload` | écrit à la publication, lu par le diffuseur/outbox et les abonnés | exception | charge utile transitoire d'événement ; format d'échange |
| 060 | `ent_edt_import.report_json` | écrit à l'import, affiché | exception | rapport d'import figé |
| 068 | `ent_journal_modif.payload` | écrit (ajout seul), consulté en audit | exception | journal d'audit en ajout seul |
| 089 | `ent_passkey_credential.passkey` | `webauthn-rs` | exception | état opaque de la bibliothèque |
| 089 | `ent_passkey_challenge.state` | `webauthn-rs`, transitoire | exception | état opaque de la bibliothèque |
| 123 | `ent_personne_fusion.instantane` | écrit à la fusion, **relu à l'annulation** (`dossier_personnes.rs`) | **normalisée** (667) | **relu à l'annulation** : ce n'est pas un instantané d'audit figé mais la mémoire de la défusion → colonnes typées (`compte_transfere`, `agent_deplace`, `agents_fusionnes`), `ent_personne_fusion_objet`, copies typées `…_lien_supprime` / `…_contact_supprime` |
| 134 | `ent_signalement_assiduite.dossier` | écrit à la proposition, lu et renvoyé par l'API | **normalisée** (563) | détail dans `ent_signalement_assiduite_detail` ; JSON d'API reconstruit à la lecture |
| 144 | `ent_import_ligne.donnees` | écrit à l'import, relu à la validation/exécution (`imports.rs`) | **normalisée** (664) | dix-sept colonnes `TEXT` (une par colonne du format `scolia_eleves_v1`) ; le JSON d'API/transit est reconstruit (`ent_import_ligne_donnees`) |
| 150 | `ent_cdt_historique.instantane` | écrit à chaque version, affiché dans l'historique | exception | instantané d'historique, jamais source de vérité |
| 172 | `ent_delegation.droits` | lu par le moteur de droits à chaque requête | **normalisée** (560) | `ent_delegation_droit` + `ent_delegation_droit_cible` ; le plus à risque (droits) |
| 182 | `ent_bulletin.contenu` | écrit à la publication, affiché | exception | copie figée d'un bulletin publié (nommée par D-036) |
| 240 | `ent_eleve_orphelin.ligne` | écrit au diagnostic, consulté | exception | ligne mise en quarantaine : instantané figé |
| 250 | `ent_inscription_dossier.resolutions` | écrit à la validation, jamais relu | **normalisée** (561) | `ent_inscription_resolution` (élève et responsables) |
| 275 | `ent_personne_preference.joignable` | lu/écrit, validé (`dossier_contact.rs`) | **normalisée** (665) | `ent_personne_plage_joignable` (+ `joignable_definie`) |
| 275 | `ent_personne_preference_version.joignable` | historique des plages | **normalisée** (665) | `ent_personne_preference_version_plage` |
| 292 | `ent_eleve_fusion.conflits_resolus` | relue à l'annulation (`eleve_fusion.rs`) | **normalisée** (666) | `ent_eleve_fusion_conflit` + `_conflit_cle` |
| 292 | `ent_eleve_fusion.resume` | écrit à la fusion, affiché | exception | résumé figé d'une fusion (registre en ajout seul) |
| 292 | `ent_eleve_fusion.doublons_touches` | relue à l'annulation (`jsonb_array_elements`) | **normalisée** (666) | `ent_eleve_fusion_doublon` |
| 292 | `ent_eleve_fusion_ligne.cle` | relue (`jsonb_to_recordset`) pour repointer à l'annulation | **normalisée** (666) | `ent_eleve_fusion_ligne_cle` : relue pour repointer les lignes → mémoire, pas audit figé |
| 302 | `ent_inscription_campagne.regle_relance` | lue pour planifier les relances | laissée | refonte prévue au socle finances 24–25 (lot F0, branche `ent-socle-finances-f0`) |
| 302 | `ent_inscription_campagne_version.instantane` | écrit à chaque version, affiché | exception | instantané d'historique de campagne |
| 304 | `ent_inscription_dossier.scolarite_actuelle` | **aucun code** ne la lit ni ne l'écrit | **normalisée** (562) | cinq colonnes `scol_*` ; ancienne colonne inutilisée |
| 332 | `ent_etablissement_profil.jours_ouvres` | lue par le calendrier (`structure_calendrier.rs`), API | **normalisée** (662) | `ent_etablissement_jour_ouvre` ; semaine type reconstruite au format de l'API (`ent_etablissement_jours_ouvres`) |
| 332 | `ent_parametre.valeur` | lue/écrite via le registre de paramètres | **normalisée** (663) | `type_param` + `valeur_entier` / `valeur_booleen` / `valeur_heure` (types du registre du code) |
| 332 | `ent_structure_journal.avant` | écrit (ajout seul) | exception | journal de structure en ajout seul |
| 332 | `ent_structure_journal.apres` | écrit (ajout seul) | exception | idem |

Bilan : 12 exceptions, 13 normalisées (en lecture seule jusqu'au retrait), 1 laissée (finances, `regle_relance`), 0 à normaliser. (`COLONNES_JSON_D036` : catégories `exception`, `retrait`, `dette` — la seule dette restante est `regle_relance`.)

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

## À faire dans un lot ultérieur

1. **Retrait des anciennes colonnes** (les 4 du lot 1 et les 9 du lot 2) après une release de coexistence : `DROP COLUMN` + retrait des déclencheurs + retrait des entrées « retrait » de `COLONNES_JSON_D036`.
2. ~~Normaliser les neuf colonnes « à normaliser »~~ : fait (lot 2, migrations 661–667).
3. `regle_relance` : suivre le lot F0 / socle finances ; ne pas la normaliser ici.
4. Côté Next.js (D-036 §3) : hors périmètre de ce lot (Next.js intouchable) ; inventaire T0 à part.

## Niveau de preuve

Implémenté et **compilé** ; test de garde (liste des exceptions) exécuté en local ; scénario DB `tests_d036` (reprise rejouée deux fois, lecture seule) et compte RLS (+4) validés par la CI (pas de serveur PostgreSQL dans l'environnement de développement local). Non validé par Florian, non pilote.
