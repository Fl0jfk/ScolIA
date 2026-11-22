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

## Correctifs de revue

Passe de correctifs sur la branche `ent-socle-41-code-i4` (PR n° 122, brouillon), partie de la tête distante **eb639da6** (`origin/ENT` n'avait pas bougé). Migrations **2828 et 2829 modifiées** (non versées) ; aucune autre migration versée ne change. Niveau : **implémenté, testé en local** ; ni validé, ni prêt pilote. Frontend non touché (règle 27). Aucun test ne dépend de l'heure réelle (jours fixes d'avril 2026, nuit du changement d'heure du 29 mars 2026 ; les seules lectures de l'horloge sont les conditions SQL de fusion, que les tests rendent indépendantes de la date du jour).

### Échecs de CI (run 45018483894)

| Échec | Cause établie | Correction | Test |
|---|---|---|---|
| `reprise_idempotente_sans_perte…`, `reprise_des_identifiants_existants` (23514 sur `ent_internat_lecture_sensible_objet_check`) | Ce n'est pas 2828/2829 : c'est **2826 (versée)** qui recrée la contrainte (liste I3) à chaque rejeu ; I4 avait élargi la liste en 2829, donc les lignes `evenement`/`sortie_groupe` la violaient au rejeu. Même piège pour `ent_internat_cas_nature_check` (2826). | **Aucune contrainte de 2824/2826 n'est élargie.** Les lectures d'I4 se journalisent sous `registre` (événements) et `sorties` (sorties de groupe) ; les cas ouverts par une sortie non rentrée portent la nature `non_rentre` (déjà permise). | `migrations_le_rejeu_avec_des_donnees_d_i4_ne_viole_aucune_contrainte` ; les deux tests rouges, rejoués sur une base remplie par les tests d'I4 |
| `s_m4_m5_motif_au_decideur_seul…` (famille voit `en_cours_d_examen`) | **Conséquence du premier échec** : le rejeu avortait au milieu de 2826 (hors transaction) et laissait, sur la base partagée de la tranche, l'ancienne `ent_internat_remise` (2826, sans la correction d'autorité de 2827). Le libellé neutre de la famille (M5) n'a **pas** changé. | Aucune modification du code ni du test : le rejeu va jusqu'au bout. | le test passe seul, après les tests d'I4, et après les deux tests de reprise sur la même base |

### Constat par constat

| Constat | État | Ce qui a été fait | Tests |
|---|---|---|---|
| **E1** un élève parti ne disparaît jamais | fait | Tâche périodique (`veille_non_rentres_a`, branchée dans `internat_sorties_tache`) : parti d'une sortie acceptée ou « en activité » d'une activité, non rentré à la fin + **tolérance de l'établissement (15 min par défaut)** → cas de nuit I3 (`non_rentre`, auteur système) avec la sortie ou le temps d'origine, l'heure prévue et les comptes à joindre (vue du cas) ; le retour ferme le cas. L'appel du soir marque « en activité » (retour attendu) les élèves partis, à l'ouverture et au départ, sans fausse alerte « absent ». Étude : pointage présent/absent, inscrit non pointé au-delà du délai (15 min) **signalé** (événement `internat.etude_non_pointee`, une fois). Réglages lisibles et modifiables (`/internat/reglages-vie`). | `e1_*`, `m9_*` |
| **E2** un accord ne vaut que pour ce qu'il couvre | fait | L'accord annuel « sortie de soirée » ne vaut que dans les bornes du type (durée ≤ `duree_max_heures`, même jour, aucune nuit couverte) à l'invitation **et** au départ ; sinon la famille répond. Une sortie de plusieurs jours : accord dédié ; l'élève n'est retiré de l'attendu **qu'après départ constaté** (`ent_internat_attendu_calcule` redéfinie en 2829 : copie de 2826 + une branche). | `e2_*` |
| **E3** la cause est une donnée de protection | fait | `voit_la_cause` (`internat.lire`, `internat.gerer`, `internat.sorties.decider`) : sans l'un d'eux, le JSON ne porte ni `motif` ni `accord_tombe` ; le résultat du départ est « non_autorise ». | `e3_*` |
| **M1 / F10** tout fait se corrige par un fait | fait | `POST …/retour/annulation` (motif codé) remet l'élève « parti » ; un refus reçu par téléphone se saisit comme un accord hors ligne (`accord: false`, auteur, date, canal). | `m1_*` |
| **M2** une seule fonction par règle | fait | `titulaire_pour_tx(Compte \| Personne)` : réponse en ligne **et** accord hors ligne (règle M-A, chaîne de fusions, interdictions `remise`, `visite`, `contact_eleve`). | `m2_*` |
| **M3** malaise | fait | Au moins un élève non témoin (422) ; signaux comptés et rendus (`signaux_sante`) ; `INTERNAT_ALERTE` documenté au contrat (`08_NOTIFICATIONS.md`). | `m3_*` |
| **M4** retours des écritures sur un événement | fait | `declarer`, `completer`, `transmettre`, `demander_intervention` : événement complet seulement avec `internat.liaison.lire` et tous les participants dans la portée (lecture journalisée) ; sinon **accusé minimal** (`accuse_minimal`). | `m4_*` |
| **M5** fusion de fiches | fait | `TableEleve.condition` : vigilances actives, inscriptions en cours et participations de sorties non closes **suivent** la fiche conservée (doublon : le conservé l'emporte) ; l'historique reste ; gardes SQL ouvertes au seul repointage de fusion ; défusion à l'identique. | `m5_*` |
| **M6** | fait | Remplacement d'un accompagnateur, y compris pendant la sortie (faits `encadrant_remplace`/`encadrant_retire`) ; présence pointée au départ, encadrement compté sur les présents ; encadrant **obligatoire** pour un temps du soir (colonne NOT NULL). | `m6_*` |
| **M7** | fait | Élèves ajoutés après l'acceptation : la sortie repasse à la décision de la direction (fait `nouvelle_decision`), pas de départ avant. | `m7_*` |
| **M8** | fait | `exiger_eleve_visible` à l'inscription et à la désinscription ; élèves protégés absents des inscrits. | `m8_*` |
| **M9** | fait | Voir E1 (étude et activité). | `m9_*` |
| **M10** | fait | Registre : plus d'export pour `ent_internat_evenement` et `_complement` ; `ent_internat_liaison` exportée sans `texte` (`hors_export`). | `m10_*` |
| **F1** droit avant existence | fait | `repondre` (qualité de titulaire avant la sortie), `completer_evenement`, inscriptions. | `f1_*` |
| **F2** transitions en base | fait | Déclencheurs : `parti` seulement depuis `autorise` (ou `rentre` : correction), `refuse`/`non_autorise` terminaux, pas d'annulation d'une sortie avec des partis. | `f2_*` |
| **F3** clés composites, pièce | fait | `donne_par_personne_id`, `reponse_personne_id`, `liaison.temps_id` ; `piece_id` validé contre `ent_doc_piece` (socle 7, table postérieure : pas de clé étrangère). | `f3_*` |
| **F4** accompagnateur | fait | `est_encadrant_tx` revérifie l'adhésion et borne la lecture à 7 jours après la fin. | `f4_*` |
| **F5** journal par élève | fait | La lecture d'une sortie journalise la liste des élèves lus. | `f5_*` |
| **F6** nuit, pas jour civil | fait | Départ, invitations, inscriptions et listes raisonnés en nuit ; tests à cheval sur minuit et au changement d'heure. | `f6_*` |
| **F7** | fait | `nb_inscrits`/`places` jamais rendus à un élève. | `f7_*` |
| **F8** majorité | fait | L'accord d'un titulaire (en ligne ou hors ligne) tombe à la majorité (`majorite`, fait `blocage_majorite`). | `f8_*` |
| **F9** chevauchements | fait | Temps/sortie dans les deux sens, **sous verrou consultatif par élève** ; motif `chevauchement`. | `f9_*`, `concurrence_*` |
| **F11** | fait | `blocage_plus_interne` (jamais `blocage_interdiction`). | `f11_*` |
| **F12** | fait | `avertissement` rendu à chaque saisie de texte libre (liaison, événement), en plus de la liste de mots. | `m3_*`, `f12_*` |

### Tests, compteurs, vérifications

- **69 tests I4** en local sur base neuve : 38 d'origine (adaptés : l'accompagnateur sans droit lit « non autorisé », le CPE lit la cause) + **31 nouveaux** (`tests_internat_i4_correctifs.rs`, dont concurrence : deux surveillants sur un départ, réponse et départ simultanés, double inscription et chevauchement sous verrou).
- Compteurs (règle 28) : `Action::TOUTES` 279 (inchangé), RLS +0, registre des tables +0, `TABLES_ELEVE` +0 (trois stratégies de fusion à condition), `MIGRATION_TAG` 3808, événements +1 (`internat.etude_non_pointee`), routes +4 chemins (retour/annulation, encadrants/remplacement, temps/pointage, reglages-vie GET et PUT), ajoutées au test « aucune route sans session ». `origin/ENT` n'a pas bougé : aucune fusion.
- `url` 2.5.9 (RUSTSEC-2026-0592) : `cargo update -p url --precise 2.5.9` dans `ent-rust` et `ent-rust/web`, rien d'autre dans les `Cargo.lock` ; `cargo metadata --locked` OK dans les deux.
- Clippy 1.99 (les 3 commandes de la CI) : 0 avertissement ; `rustfmt` ciblé sur les fichiers d'I4 et `cargo fmt -p ent-types --check` verts.

### Non fait, limites

- Le signal `internat.etude_non_pointee` et la cible de `internat.alerte` restent à brancher côté socle 8 (contrat documenté).
- L'étude non pointée est **signalée**, elle n'ouvre pas de cas de nuit (l'élève est dans l'internat) ; l'appel d'étude par le composant du 12 reste non branché.
- La liste de mots de santé reste une aide : l'avertissement le dit à chaque saisie.
