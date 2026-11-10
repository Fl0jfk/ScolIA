# D-036 — inventaire des colonnes JSON/JSONB (2026-11-10)

**Règle (D-036)** : plus aucun JSON qui sert de mémoire. Permis : JSON comme format d'échange de l'API, instantané figé d'audit ou d'historique (jamais relu comme source de vérité), état technique opaque d'une bibliothèque (passkey `webauthn-rs`), charge utile transitoire d'une file d'envoi.

**Base de l'inventaire** : `origin/ENT` (migrations 001 à 480) ; 26 colonnes JSON/JSONB déclarées dans les migrations (`grep -in json ent-rust/migrations/*.sql`, hors fonctions `to_jsonb` des gardes). Branche `ent-d036-normalisation-json`, migrations 560–563.

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
| 123 | `ent_personne_fusion.instantane` | écrit à la fusion, **relu à l'annulation** (`dossier_personnes.rs`) | à normaliser | relu pour restaurer : sort du cadre « figé » (réserve) |
| 134 | `ent_signalement_assiduite.dossier` | écrit à la proposition, lu et renvoyé par l'API | **normalisée** (563) | détail dans `ent_signalement_assiduite_detail` ; JSON d'API reconstruit à la lecture |
| 144 | `ent_import_ligne.donnees` | écrit à l'import, relu à la validation/exécution (`imports.rs`) | à normaliser | lignes d'import par type : tables typées par type d'import (lot suivant) |
| 150 | `ent_cdt_historique.instantane` | écrit à chaque version, affiché dans l'historique | exception | instantané d'historique, jamais source de vérité |
| 172 | `ent_delegation.droits` | lu par le moteur de droits à chaque requête | **normalisée** (560) | `ent_delegation_droit` + `ent_delegation_droit_cible` ; le plus à risque (droits) |
| 182 | `ent_bulletin.contenu` | écrit à la publication, affiché | exception | copie figée d'un bulletin publié (nommée par D-036) |
| 240 | `ent_eleve_orphelin.ligne` | écrit au diagnostic, consulté | exception | ligne mise en quarantaine : instantané figé |
| 250 | `ent_inscription_dossier.resolutions` | écrit à la validation, jamais relu | **normalisée** (561) | `ent_inscription_resolution` (élève et responsables) |
| 275 | `ent_personne_preference.joignable` | lu/écrit, validé (`dossier_contact.rs`) | à normaliser | plages de joignabilité : table de plages (lot suivant) |
| 275 | `ent_personne_preference_version.joignable` | historique des plages | à normaliser | idem, historique |
| 292 | `ent_eleve_fusion.conflits_resolus` | relue à l'annulation (`eleve_fusion.rs`) | à normaliser | table des conflits résolus (lot suivant, socle élève) |
| 292 | `ent_eleve_fusion.resume` | écrit à la fusion, affiché | exception | résumé figé d'une fusion (registre en ajout seul) |
| 292 | `ent_eleve_fusion.doublons_touches` | relue à l'annulation (`jsonb_array_elements`) | à normaliser | table de liaison (lot suivant) |
| 292 | `ent_eleve_fusion_ligne.cle` | relue (`jsonb_to_recordset`) pour repointer à l'annulation | à normaliser | colonnes typées par table (lot suivant) |
| 302 | `ent_inscription_campagne.regle_relance` | lue pour planifier les relances | laissée | refonte prévue au socle finances 24–25 (lot F0, branche `ent-socle-finances-f0`) |
| 302 | `ent_inscription_campagne_version.instantane` | écrit à chaque version, affiché | exception | instantané d'historique de campagne |
| 304 | `ent_inscription_dossier.scolarite_actuelle` | **aucun code** ne la lit ni ne l'écrit | **normalisée** (562) | cinq colonnes `scol_*` ; ancienne colonne inutilisée |
| 332 | `ent_etablissement_profil.jours_ouvres` | lue par le calendrier (`structure_calendrier.rs`), API | à normaliser | semaine type : table jour × demi-journée ; touche DTO et calendrier (lot suivant) |
| 332 | `ent_parametre.valeur` | lue/écrite via le registre de paramètres | à normaliser | paramètres typés par clé (lot suivant) |
| 332 | `ent_structure_journal.avant` | écrit (ajout seul) | exception | journal de structure en ajout seul |
| 332 | `ent_structure_journal.apres` | écrit (ajout seul) | exception | idem |

Bilan : 12 exceptions, 4 normalisées, 1 laissée (finances), 9 à normaliser. (`COLONNES_JSON_D036` regroupe « à normaliser » et « laissée » sous la catégorie `dette`.)

## Lots réalisés (migrations 560–563)

- **560** `delegation_droits_normalises` : `ent_delegation_droit` (action, portée, ordre, unique par action/portée) et `ent_delegation_droit_cible` (cibles désignées), clés étrangères en cascade, RLS permissive. Code : `delegation.rs` (`charger_droits`, `ecrire_droits_tx`), `edt_remplacement.rs`. Le moteur de droits lit les tables.
- **561** `inscription_resolution` : une ligne pour l'élève (nouveau/existant, confirmation des responsables), une par responsable (personne nouvelle/existante, foyer nouveau/existant), contrainte de forme, unicité par dossier, RLS permissive.
- **562** `inscription_scolarite_actuelle` : colonnes `scol_etablissement_libelle`, `scol_uai` (format UAI), `scol_type`, `scol_niveau`, `scol_annee`.
- **563** `signalement_assiduite_detail` : détail jour/période/minutes ; le « dossier » renvoyé par l'API est reconstruit (`SELECT_SIGNALEMENT`), format d'échange inchangé.

Propriétés communes : reprise **idempotente** (les migrations sont rejouées à chaque démarrage) — seules les lignes sans équivalent typé sont reprises, valeurs invalides ignorées (jamais d'échec de démarrage) ; l'ancienne colonne n'est plus écrite par le code (`[]`, `{}` ou NULL), et pour 560 et 563 un déclencheur la rend **en lecture seule** (SQLSTATE 42501).

## À faire dans un lot ultérieur

1. **Retrait des anciennes colonnes** (`ent_delegation.droits`, `ent_inscription_dossier.resolutions`, `scolarite_actuelle`, `ent_signalement_assiduite.dossier`) après une release de coexistence : `DROP COLUMN` + retrait des déclencheurs + retrait des entrées « retrait » de `COLONNES_JSON_D036`.
2. Normaliser les neuf colonnes « à normaliser » ci-dessus, par risque : `jours_ouvres` et `valeur` (paramètres d'établissement), `donnees` (import), `joignable`, puis la famille « annulation de fusion » (socle élève).
3. `regle_relance` : suivre le lot F0 / socle finances ; ne pas la normaliser ici.
4. Côté Next.js (D-036 §3) : hors périmètre de ce lot (Next.js intouchable) ; inventaire T0 à part.

## Niveau de preuve

Implémenté et **compilé** ; test de garde (liste des exceptions) exécuté en local ; scénario DB `tests_d036` (reprise rejouée deux fois, lecture seule) et compte RLS (+4) validés par la CI (pas de serveur PostgreSQL dans l'environnement de développement local). Non validé par Florian, non pilote.
