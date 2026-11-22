# Socle 41 « Internat » — lot I4 « Vie quotidienne » : livraison

Branche `ent-socle-41-code-i4` (base `origin/ENT`, PR en brouillon vers `ENT`). Conception : `docs/socles/41_INTERNAT.md` §10.3, §11, §19.6 à §19.8, §20 (R-IT7 à R-IT9, R-IT21, R-IT24, R-IT25, R-IT29), §22, §24 ; INT-170 à INT-194, INT-247, INT-257. Niveau : **implémenté, testé en local** ; CI : voir « Tête et CI ». Ni validé, ni prêt pilote. Frontend non touché (règle 27).

**Environnement des tests** : PostgreSQL 16 local, base **neuve** `ent_i4_test`, `ENT_EXIGER_BASE=1`, `ENT_ENV=test`, Rust 1.99.0, `--locked`, un seul `CARGO_TARGET_DIR`. Aucun test ne dépend de l'heure réelle : chaque commande `*_a` reçoit son instant ; jours fixes d'avril 2026 (heure d'été). Le banc HTTP lit la « nuit courante » du serveur, comme celui d'I3.

## 1. Ce qui est fait

| Domaine | Livré |
|---|---|
| Migrations | **2828** (sorties de groupe, réglages de vie quotidienne) et **2829** (liaison, événements, temps du soir, préoccupation, vigilance). Libres sur toutes les branches distantes. 16 tables, RLS, `etablissement_id`, `BEFORE TRUNCATE`, gardes d'ajout seul. |
| Sorties de groupe (INT-170 à 179) | création, modification du brouillon, accompagnateurs (comptes du personnel), invités (écartés avec motif, jamais en silence), soumission (préavis 24 h, encadrement 12 élèves par accompagnateur : réglages), décision de la direction (jamais l'auteur ; portée de formation vérifiée), annulation (familles prévenues), duplication sans reprise des accords, réponse des titulaires et de l'élève majeur, accord annuel du socle 18, **accord hors ligne daté et nommé**, appel de départ et de retour, clôture (bloquée tant qu'un parti n'est pas rentré), vue famille limitée à son enfant. |
| Liaison codée (INT-180 à 184, 247) | catalogue par établissement (10 codes, jamais de santé), écriture (texte ≤ 300, contrôle de santé, nuit courante ou 7 précédentes, interne seulement), chronologie, vue de nuit, vue « pour le jour » de la vie scolaire **sans texte**, gestes du CPE (masquer avec motif, à revoir, signaler au professeur principal sans texte), lectures journalisées. |
| Événements de nuit (INT-185 à 189) | déclaration (participants élèves et adultes, position, mesures), compléments en ajout seul, transmission au 13 (une fois), demande d'intervention technique sans nom d'élève, **malaise sans texte libre** avec signal codé vers le 34, **alerte critique** avec la seule position, préoccupation sans aucun contenu. |
| Temps du soir (INT-190 à 194) | études et activités, places sous verrou, inscription par le personnel ou par l'élève avant le début, pas de chevauchement (autre temps, sortie de groupe autorisée), annulation, absence à l'étude = fait de liaison rattaché à la séance, planning de l'élève sans autre élève. |
| Vigilance (INT-257) | 3 codes, référent du personnel, revue hebdomadaire, levée datée, sans texte. |
| Droits | 4 actions : `internat.liaison.ecrire`, `.lire`, `.jour`, `internat.evenement.declarer` (279 au total) ; modèle par défaut : administration, direction, CPE. Professeur, infirmerie, psychologue, comptabilité, familles, élèves : aucun. |
| API | 39 chemins `/internat/*` (une cinquantaine de méthodes) (`ent-api/src/internat_vie.rs`), session obligatoire. |

## 2. Règles générales de protection

« Un acte tombe avec son auteur » : au départ, l'accord d'un titulaire visé par une interdiction (chaîne de fusions comprise), d'une personne hors ligne devenue interdite ou d'un accord annuel dont les décideurs sont interdits est écarté (cause `interdiction_auteur`, jamais relayée). « Pas de départ constaté = élève attendu » : une sortie de groupe ne retire jamais un élève de l'attendu. « Une interdiction bloque toujours ». « Refus par défaut » : seul un élève `autorise` part ; une réponse manquante ne l'autorise jamais. Une seule fonction de vérification d'accord (`accord_tient_tx`). Aucune donnée de santé, aucun nom ni texte libre dans l'outbox (testé).

## 3. Tests (tous verts en local)

- **38 tests backend** (`ent-db/src/tests_internat_i4.rs`) : cycle complet, R-IT21, plus restrictif entre titulaires, refus par défaut, décision pas par l'auteur, accord tombé (titulaire interdit, chaîne de fusions, hors ligne, annuel), désaccord connu, majeur, R-IT24, R-IT25, encadrement, préavis, gardes SQL (suppression, modification, TRUNCATE), duplication, annulation, attendu inchangé, vue famille, lecture bornée, portée de formation, liaison (bornes, R-IT8, jour sans texte, portée, masquage), événements (validations, portée, malaise, alerte, intervention), préoccupation (colonnes), temps (places, chevauchement, élève, absence d'étude, planning), vigilance.
- **1 parcours HTTP** + toutes les routes vérifiées « sans session » (401) (`ent-api/src/tests_internat.rs`) ; suite `separation` (57) verte.
- Gardes : RLS (+16), registre de données (+16), `TABLES_ELEVE` (+8), actions (279), sources de routes (70), `jour_utc_interdit`, `isolation_sql_statique` : verts. I2 et I3 : verts.
- `ent-rust/web` : `cargo metadata --locked` OK. Zéro avertissement clippy 1.99 (workspace, labo). fmt ciblé sur les fichiers nouveaux.

## 4. Compteurs (règle 28)

`Action::TOUTES` 275 → 279 ; RLS +16 ; registre +16 tables ; `TABLES_ELEVE` +8 ; `SOURCES` / `NOMS_SOURCES` 69 → 70 (`internat_vie.rs`) ; `MIGRATION_TAG` inchangé (3808) ; événements +20. Fusion `origin/ENT` faite avant la CI finale : aucun compteur à recalculer (ENT n'a apporté que des tests et le semis relatif).

## 5. Non fait, écarts, limites

- **Marque « en activité » à l'appel du soir** pour les participants d'une sortie de groupe ou d'une activité, et **appel d'étude par le composant du 12** : non branchés (l'attendu est bien inchangé). Suite recommandée « I4b ».
- **Salle du socle 5** : le temps du soir porte un lieu court, pas une clé de salle.
- **Bénévoles vérifiés** comme accompagnateurs : non (comptes du personnel seulement, R-IT25).
- **Socles 13, 34, 36, 44** non livrés : le 41 émet `internat.evenement_transmis`, `.malaise_signale`, `.preoccupation_signalee`, `.intervention_demandee` sous son nom ; les socles destinataires les consommeront.
- **Visibilité famille de la liaison** (R-IT9) : aucune lecture ouverte ici ; la publication dans le bilan relève d'I7.
- **Catalogue de liaison par établissement** (copié à la première lecture) au lieu de `etablissement_id NULL` (§24) : écart assumé, documenté dans `MIGRATIONS.md`.
- **Déclenchement de l'alerte critique** (U3, niveau 8) : l'événement est émis ; l'acheminement vers l'astreinte relève du socle 8.
- Le parcours HTTP ne couvre pas les succès d'écriture d'une liaison ou d'une inscription (banc sans séjour d'hébergement) : ils sont éprouvés contre la base.
- Réglages par défaut prudents (D-069), sans écran de réglage dédié : préavis 24 h, 12 élèves par accompagnateur.
- Le test complet `cargo test -p ent-db` sur UNE base partagée échoue pour des tests sans rapport (sémis global) ; la CI partitionne ses bases.

## Tête et CI

Voir le message final au pilote (tête et état de la CI y sont consignés).
