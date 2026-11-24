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

## Passe 2 (contre-revue du pilote, D-069)

Passe de correctifs sur la même branche (PR n° 122, brouillon), partie de la tête distante **79b436cb**, puis fusion d'`origin/ENT` (F3, V3, V5, messagerie M12). Migrations **2828 et 2829** (non versées) : une seule modification, la liste des motifs de participant de 2828 (`accord_annuel_tombe`, F-a). Aucune autre migration versée ne change. Niveau : **implémenté, testé en local** ; ni validé, ni prêt pilote. Frontend non touché (règle 27). Aucun test ne dépend de l'heure réelle : jours fixes d'avril 2026 ; la tâche planifiée reçoit désormais son horloge (`internat_sorties_tache_a`), seul le planificateur passe l'heure réelle.

### Décisions du pilote appliquées

| Point | Fait | Où | Tests (`tests_internat_i4_passe2.rs`) |
|---|---|---|---|
| **É1** un élève parti reste suivi jusqu'à son retour pointé | La veille et la marque « en activité » de l'appel suivent tout inscrit pointé `present` sans retour, **quel que soit le statut du temps ou le retrait de l'inscription**. L'annulation d'un temps reste possible. Le retour se pointe aussi sur un temps annulé (sans fenêtre horaire) : seul le retour, pas un nouveau départ ni une absence. La désinscription d'un élève de l'activité pointé `present` sans retour est **refusée** (« parti en activité… »). | `groupes.rs` (veille, marque à l'ouverture), `vie.rs` (`pointer_temps_a`, `desinscrire_temps_a`) | `e1_annuler_un_temps…`, `e1_la_desinscription…` |
| **M7** l'accompagnateur compte ses élèves, protégés compris | Temps du soir : le protégé n'est plus retiré, il est marqué `protege` (nouveau champ de `PointageInscritDto`, sans nom). Sorties : déjà masqués (nom absent, état et départ visibles) ; événements sans nom (testé). | `vie.rs` (`temps_dto_a`), `ent-types/internat_vie.rs` | `m7_*` ; `m8_*` d'origine adapté |
| **F-g** un refus s'enregistre toujours | Un refus hors ligne est accepté pour tout parent ayant l'autorité parentale, même interdit ; il est tracé (auteur, date, canal). Un **accord** d'un parent interdit reste refusé. | `groupes.rs` (`accord_hors_ligne_groupe_a`) | `fg_*` |
| Tolérance « non rentré » | 15 min, paramètre d'établissement, inchangé. | | `e1_*` d'origine |

### Constat par constat

| Constat | État | Ce qui a été fait | Tests |
|---|---|---|---|
| **M1** un cas existant n'est jamais rouvert | fait | Un cas existant pour (élève, sortie ou temps, nuit), quelle que soit son issue, bloque la veille. Un cas **sans origine** (ouvert par l'appel) encore ouvert est **rattaché** à la sortie ou au temps et à l'heure prévue (le retour le ferme) ; fermé par l'appel **depuis le départ** de l'élève, il compte comme cas existant (un cas fermé avant le départ ne compte pas : l'élève peut partir plus tard dans la nuit). | `m1_*` (cas fermé par le CPE puis minutes suivantes ; cas fermé par l'appel ; cas ouvert par l'appel avant la veille, rattaché puis fermé au retour ; un événement par cas, aucun bruit) |
| **M2** point de reprise par ligne | fait | `veille_non_rentres_bilan_a` et `veille_etudes_bilan_a` : un `SAVEPOINT` par élève (comme `veille_sorties_bilan_a`) ; une ligne en erreur est comptée et n'empêche pas les autres. Les **trois** veilles alimentent **ensemble** `suivre_echecs_veille` (un seul appel par établissement et par minute : une veille réussie ne remet pas à zéro l'échec d'une autre). | `m2_*` (déclencheur de test qui fait échouer une ligne ; reprise à la minute suivante) |
| **M3** étude et signaux sans destinataire | fait | Un inscrit pointé « absent » est signalé comme un non-pointé. Nouveau consommateur `i4.internat_alertes` (`internat_reactions.rs`) : `internat.cas_ouvert`, `internat.etude_non_pointee`, `internat.alerte` → avis dans l'ENT (et e-mail / push selon les préférences) aux comptes qui détiennent `internat.gerer` (direction, CPE, administration + rôles attribués, relus au traitement) ; texte fixe, **aucun nom d'élève** ; I3 ne consommait pas `INTERNAT_CAS_OUVERT` : branché ici. | `m3_*` (le CPE et la direction reçoivent l'avis ; un professeur sans droit non) |
| **M4** accord annuel « soirée » | fait | L'accord ne vaut plus pour une sortie qui **commence dans la nuit** (avant 6 h) en plus des exclusions existantes. **Écart assumé** : le texte de la consigne dit « recoupe [ancre, matin 6 h] » ; l'ancre est à **22 h**, donc 21 h → 23 h recouperait la nuit alors que la consigne demande qu'elle reste autorisée. Règle retenue : même jour local, début **après 6 h**, aucune nuit couverte. Une sortie 22 h → 23 h 30 reste donc autorisée par l'accord annuel. À trancher par le pilote si l'ancre doit s'appliquer. | `m4_*` (0 h 30 → 2 h 00 et 5 h 30 → 7 h : réponse de la famille ; 21 h → 23 h et 18 h → 22 h : autorisé) |
| **M5** registre des données | fait | `ent_internat_sortie_groupe_participant` : `hors_export(["motif_code", "reponse_user_id"])`. `ent_internat_cas` : `hors_export(["nature", "issue_note"])` (la nature `interdiction_auteur` révèle la cause). Le test lit la déclaration réelle et vérifie l'existence de chaque colonne. | `m5_*` |
| **M6** régressions | fait | Branchement de la tâche (la suppression d'un des deux appels fait échouer `m6_la_tâche…`) ; sortie à cheval sur minuit (cas ouvert après minuit, dans la nuit du 17) ; cas déjà ouvert par l'appel ; É1 ; M1. | `m6_*`, `e1_*`, `m1_*` |
| **F-a** cause de la chute de l'accord annuel | fait | Hors bornes (durée, jour, nuit, type désactivé) : `accord_annuel_tombe`. Verdict plus « autorisé » : `interdiction` **seulement** si un décideur de l'accord est visé par une interdiction ; sinon `accord_annuel_tombe`. Motif ajouté à la liste de 2828. | `fa_*` |
| **F-b** un signal par élève | fait | Malaise : un élève déclaré victime et présent reçoit un seul signal. | `fb_*` |
| **F-c** erreur claire | fait | Inviter un élève à une sortie acceptée dont des élèves sont déjà partis : conflit lisible (« des élèves sont déjà partis… »). | `fc_*` |
| **F-d** fusion avec « parti » chez l'absorbé | **non fait** | Pas de correction sûre sans casser la règle « le conservé l'emporte » : si la ligne « parti » reste sur la fiche absorbée, la veille la suit sur cet identifiant. Le cas serait ouvert sur la fiche absorbée (jamais perdu). À traiter avec une décision de fusion (reprendre la ligne « parti » sur le conservé) ; hypothèse non testée. | |
| **F-e** paramètres écrits à chaque minute | fait | `reglage_vie_ou_creer_tx` : lecture d'abord, écriture seulement si la ligne manque. | couvert par `m2_*`, `e1_*` |
| **F-f** accord annuel sans contrôle de majorité | **limite notée** | Sans conséquence de protection (l'accord annuel est celui du socle 18, qui relève de ses propres règles de majorité) ; contrôle à étudier au socle 18. | |

### Tests, compteurs, vérifications

- **17 tests nouveaux** (`tests_internat_i4_passe2.rs`), **2 adaptés** (`m8_*` : le protégé reste compté et marqué ; `m9_*` : l'absent à l'étude est signalé). En local, base **neuve** `ent_ci_test_p2` (PostgreSQL 16, `ENT_ENV=test`, `--locked`, un seul `CARGO_TARGET_DIR`, Rust 1.99.0) : les 17 tests de la passe 2 verts ; tous les tests `tests_internat*` (i2, i3, i4, correctifs) verts, hors deux tests de rejeu / d'attendu qui échouaient en exécution **parallèle** et sont verts en série (voir ci-dessous) ; RLS, registre, migrations et réactions verts ; `tests_voyages_v3` vert (les règles de 654 tiennent).
- **Constat fait en fusionnant `origin/ENT`** : 654 (socle 42) et 2829 redéfinissent toutes deux `ent_internat_attendu_calcule`. Avec 654 après 2829, la branche des sorties de groupe d'I4 disparaissait de la base (échec de `e2_sortie_de_plusieurs_jours…` et du test de rejeu). Correctif : 654 passe avant 2828-2829 dans `migrate.rs`, et 2829 reprend les deux ajouts de 654. Garde : `m6_l_attendu_garde_la_branche…`.
- Compteurs (règle 28), recalculés **depuis la base commune** (`c941d680`) : RLS = base + 22 (ENT : F3 +8, V3 +6, I5 +8) + 16 (I4) ; la valeur de ENT (+1 V5 déjà dans la base) n'est pas comptée deux fois ; `SOURCES` / `NOMS_SOURCES` 70 → **73** (`finances_grilles.rs` et les inscriptions I5 d'ENT, `internat_vie.rs` d'I4) ; `MIGRATION_TAG` inchangé (3808) ; `Action::TOUTES` inchangé (279 + ce qu'ENT apporte : la compilation l'atteste) ; événements : aucun nouveau ; consommateurs : +1 (`i4.internat_alertes`).
- Clippy 1.99.0, les trois commandes de la CI (dont `ent-lab`), `--locked`, `-D warnings` : **0 avertissement**. `cargo metadata --locked` dans `ent-rust/web` : OK. `rustfmt` ciblé sur les fichiers de la passe qui étaient déjà propres (pas de `cargo fmt --all`).

### Non fait, limites (passe 2)

- **F-d** (fusion, ligne « parti » chez l'absorbé) et **F-f** : voir le tableau.
- **M4** : écart sur l'ancre (22 h), à confirmer par le pilote.
- Le retrait d'une inscription à un temps ne peut pas, en l'état, laisser un élève pointé en activité sans retour (la désinscription est refusée) ; la veille et le retour ne dépendent pourtant pas de ce retrait (garde-fou pour une donnée ancienne ou un autre chemin) : ce garde-fou n'est pas testé en base (le journal d'ajout seul interdit de forger cet état par SQL).
- La désinscription n'est refusée que pour une **activité** (l'étude n'a pas de retour) ; l'élève pointé présent à une étude se désinscrit normalement.
- Les avis `i4.internat_alertes` partent à tout compte `internat.gerer` de l'établissement (pas encore filtrés par périmètre d'internat) : à affiner avec le socle 8 si l'établissement a plusieurs internats.
- Un cas fermé par l'appel **avant** le départ de l'élève ne bloque pas la veille (l'élève peut partir plus tard dans la nuit et ne pas rentrer).

### Intégration (passe 2)

- Tête de code poussée : `17e7d872` (fusion d'`origin/ENT` incluse, compteurs recalculés selon la règle 28).
- CI « ENT Rust » sur `17e7d872` : **verte** (run 45107606805). Les commits suivants ne touchent que cette documentation.
- PR n° 122 laissée en brouillon.

## Suites (passe 3)

Branche `ent-socle-41-code-i4-suites` (base `origin/ENT`, PR en brouillon vers `ENT`) ; la passe 2 est versée sur ENT. Migrations **2820 à 2829 inchangées** (règle 25) ; nouvelle **2830** (plage 2820–2839 du socle 41). Niveau : **implémenté, testé en local** ; ni validé, ni prêt pilote. Frontend non touché (règle 27). Aucun test ne dépend de l'heure réelle (jours fixes d'avril 2026 ; les seuls instants posés « par la base » règlent l'âge d'un événement par rapport à `now()` de la base, sans lire l'horloge du test).

### Correctifs moyens

| Constat | État | Ce qui a été fait | Tests (`tests_internat_i4_passe3.rs` sauf mention) |
|---|---|---|---|
| **1** contrainte `motif_code` de 2828 posée en ligne | fait | **2830** remplace les CHECK en ligne par des contraintes **nommées** (`ent_internat_sg_part_motif_code_chk` : toutes les valeurs du code, dont `accord_annuel_tombe` ; `ent_internat_sg_fait_acte_chk` : ajout de `refus_apres_depart`). Retrait de toute contrainte de contrôle de la colonne (ancien nom automatique ou rejeu), puis ajout ; rejouable. | `s3_le_rejeu_de_2830_*` (base portant l'ancien CHECK, rejeu deux fois, puis l'accord annuel « tombe » sans 23514) |
| **2** protégé du temps du soir | fait | `pointer_temps_a` ne passe plus par `exiger_eleve_visible` : pour un élève **inscrit au temps** il contrôle l'inscription ; envers un tiers non inscrit, l'opacité reste « n'est pas inscrit ». | `s3_un_encadrant_sans_droit_*` (absence d'une étude, départ, veille et retour d'une activité, cas fermé ; tiers protégé non inscrit : « n'est pas inscrit ») |
| **3** `m6_la_tâche_planifiée…` | fait | Nouvelle variante **limitée à un établissement**, `internat_sorties_tache_etab_a` (publique : relance ciblée utile en exploitation) ; la tâche générale la boucle. Le test l'appelle pour son établissement seulement. | `m6_la_tâche_planifiée_*` (adapté) |
| **4** É1, marque « en activité » sur un temps annulé | fait | Test d'ouverture d'appel après l'annulation d'une activité avec un élève pointé présent sans retour : marque « activité » + retour attendu. | `s3_e1_*` |
| **5** M1, pas de réouverture chaque nuit | fait | Un cas lié à la sortie ou au temps ne se rouvre pas, **toutes nuits confondues**, tant qu'il est ouvert ou clos depuis le départ de l'élève ; seul un fait nouveau (un départ postérieur à la clôture) en rouvre un. Même nuit : toute issue bloque, comme avant. | `s3_m1_*` (deux nuits, plusieurs minutes) ; `m1_*` inchangé et vert |

### Faibles

| Point | État | Détail |
|---|---|---|
| Code mort `retire_le IS NULL OR $4` ; ligne en cas de réinscription | fait | La lecture du pointage ne vise que la ligne **active** (`retire_le IS NULL`, le déclencheur 2829 interdit toute modification d'une inscription retirée, la désinscription d'un élève en activité est refusée). La veille choisit la ligne active d'abord (`ORDER BY retire_le DESC NULLS FIRST`). |
| Course veille / retour | fait | `ouvrir_cas_non_rentre_tx` relit l'état de l'élève **sous verrou de ligne** (`parti` sans retour, ou pointé présent sans retour) avant d'ouvrir ; un élève rentré entre la liste et l'ouverture n'ouvre aucun cas. |
| Avis M3 regroupés | fait | Un avis **par établissement, par nature et par minute** (`cas_ouvert`, `etude_non_pointee`, `retour_demande`), avec le nombre d'élèves, **sans nom** (l'avis dans l'ENT est idempotent sur un identifiant dérivé de la clé de regroupement). L'alerte critique n'est **jamais** regroupée. Limite : le nombre est celui connu au traitement du premier événement de la minute. Test : `s3_avis_*`. |
| Marquage « protégé » identique | fait | Sorties : `protege` = protégé ou masqué, comme les temps du soir. Test : `s3_le_marquage_*`. |
| **F-g (D-069)** | fait | Un refus téléphonique reçu **après le départ**, ou pour un participant ni `en_attente` ni `autorise`, est enregistré comme **fait tracé** (`refus_apres_depart` : auteur, date, canal ; ligne d'accord hors ligne `accord = false`) **sans changer l'état** ; pour un élève « parti », il émet `internat.retour_demande` (sans nom ni identité du parent) et un avis aux responsables. Il n'annule pas le départ. Un accord reçu après le départ reste refusé. Test : `s3_fg_*`. |
| Tests lâches | fait | `fg_*` vérifie `refuse` ; `m3_*` vérifie l'absence de nom et d'identifiant dans l'avis ; `m5_*` : plus de `unwrap_or(true)`. |
| Consommateur `i4.internat_alertes` | fait | Au premier passage il ne reçoit que les événements encore **en attente** dans l'outbox (la distribution ne rejoue pas l'historique déjà distribué). Garde supplémentaire : un événement de **plus de 24 h** ne produit aucun avis. Test : `s3_avis_*` + test unitaire. |

### Non fait / limites

- `search_path` des gardes de 2828 et 2829 et de `ent_internat_attendu_calcule` : chantier transverse (inchangé). 2830 ne crée aucune fonction.
- M5 : restent exportables `ent_internat_cas_etape.note`, `accord_hors_ligne` (refus d'un parent) et `reponse_qualite` → lot G3 (inchangé). Le refus tardif ajoute une ligne `accord = false` à la table hors ligne : même régime.
- F-d (fusion, ligne « parti » chez l'absorbé) et F-f (accord annuel sans contrôle de majorité) : inchangés (passe 2).
- Le consommateur d'avis n'est pas filtré par périmètre d'internat (inchangé).

### Tests, compteurs, vérifications (passe 3)

- **9 tests nouveaux** : 7 d'intégration (`tests_internat_i4_passe3.rs`) et 2 unitaires (`internat_reactions.rs`) ; **4 adaptés** (`m6_la_tâche_planifiée…`, `fg_*`, `m3_*`, `m5_*`). En local, base **neuve** `ent_i4s3_test` (PostgreSQL 16, `ENT_EXIGER_BASE=1`, `ENT_ENV=test`, `--locked`, un seul `CARGO_TARGET_DIR`, exécution en série) : tous les tests dont le nom contient `internat`, `migrat`, `tests_rls` ou `registre` (317) verts après correction du seul test de rejeu 2830 (la base partagée portait déjà des lignes `accord_annuel_tombe` : l'ancien CHECK est posé `NOT VALID` pour reproduire l'état d'une base migrée), puis les 9 nouveaux rejoués verts.
- Compteurs (règle 28) : migrations +1 (**2830**) ; `MIGRATION_TAG` inchangé (3808) ; `Action::TOUTES`, RLS, registre des tables, `TABLES_ELEVE`, `SOURCES` : **inchangés** (aucune table, aucune action, aucune route) ; événements +1 (`internat.retour_demande`) ; consommateurs +0 (`i4.internat_alertes` écoute un type de plus) ; routes +0.
- `cargo metadata --locked` dans `ent-rust/web` : OK (aucun `Cargo.lock` modifié). `rustfmt` ciblé sur les fichiers déjà propres (pas de `cargo fmt --all`).
- **Écart d'outillage, à lire** : le conteneur ne fournit que **rustc/clippy 1.97.0**, pas 1.99.0. Clippy 1.97 `-D warnings` sur `ent-db` (bibliothèque, avec et sans `labo`) : **0 avertissement** ; sur les cibles de test il ne signale que deux `unwrap_err` après `is_err` dans `tests_voyages_v2a.rs:865,877` (fichier non touché, lint propre à 1.97). Le workspace entier et `ent-lab` n'ont pas pu être linté en local (mémoire du conteneur, arrêts par le noyau) : c'est la CI (1.99) qui fait foi pour les trois commandes.
### Tête et CI (passe 3)

- Tête de code poussée : `ce0438d9` ; PR n° 131, laissée en brouillon vers `ENT`. `origin/ENT` n'a pas bougé depuis la base : aucune fusion, aucun compteur à recalculer.
- CI « ENT Rust » sur `ce0438d9` : **verte** (run 45183332228), y compris clippy 1.99 sur le workspace et `ent-lab`. Les commits suivants ne touchent que cette documentation.

### Correctifs du pilote après contre-revue des suites (2026-11-23)
- **M-1 (régression du lot)** : la boucle de 2830 retirait aussi `ent_internat_sg_part_bloque_chk` (« refusé, non autorisé ou retiré ⇒ motif »), qui cite `motif_code`. Elle l'épargne désormais, et 2830 la repose à l'identique. Le test de rejeu vérifie sa présence.
- **M-2** : un retour pointé puis annulé ne relançait jamais l'alerte (« même nuit, quelle que soit l'issue »).
  - `annuler_retour_groupe_a` remet `parti_le` à l'heure de l'annulation ; le départ réel reste au journal.
  - La veille ne compte plus que les cas ouverts, ou clos **depuis le départ en cours**.
  - Test : `s3_m2_un_retour_annule_rouvre_l_alerte`.
- **M-3** : la demande de retour (refus d'un parent après le départ) n'est jamais regroupée, comme l'alerte critique.
- **M-4** : l'alerte critique est toujours relayée, même au-delà de 24 h. Les autres avis trop anciens sont écartés avec un `tracing::warn!`, plus jamais en silence.
- **Ordre** : 2830 placée après 690 (règle 38).
- **Local** : `tests_internat*` (249 verts), `garde_search_path`, `migrate`. Clippy 1.99, trois commandes, `ent-lab` compris : sans avertissement.
- **Reste en file (faibles)** :
  - tests F-g plus serrés (`refus_de` seul, contenu de l'avis, cas limites) ;
  - encadrant d'un autre temps et autre établissement testés ;
  - portée `InternatAppel` sans droit de confidentialité : confirmer, ou restreindre à l'encadrant ;
  - `internat_sorties_tache_etab_a` en `pub(crate)` ;
  - avis de retour aussi aux accompagnateurs, avec la sortie désignée.

## Suites 2

Branche `ent-socle-41-code-i4-suites-2` (base `origin/ENT`, PR en brouillon vers `ENT`). **Aucune migration** (2820 à 2830 inchangées, ordre de `migrate.rs` intact, règle 38). Niveau : **implémenté, testé en local** ; ni validé, ni prêt pilote. Frontend non touché. Aucun test ne dépend de l'heure réelle (jours fixes d'avril 2026 ; l'âge d'un événement est posé par la base sur une minute calculée une seule fois).

### Fait

| Point | Ce qui a été fait | Tests (`tests_internat_i4_suites2.rs`) |
|---|---|---|
| **1** F-g plus serrés | `refus_de(…) \|\| r.is_err()` remplacé par le refus exact (`tests_internat_i4_passe3.rs`). Cas ajoutés : parent interdit **après** le départ (refus tracé, état `parti`, un seul fait `refus_apres_depart`) ; élève majeur (« répond lui-même », rien d'écrit) ; personne non titulaire (« ne peut pas être enregistré », rien d'écrit, aucun événement) ; élève `rentre` et élève `non_autorise` (trace sans changement d'état ni demande de retour) ; second refus sur un élève déjà `refuse` (trace de plus, état inchangé). Contenu de l'avis : ni nom, ni prénom, ni identifiant d'élève ou de parent. | `s4_fg_*` (6) |
| **2** pointage du soir | Encadrant d'un **autre** temps : `Forbidden` exact ; **autre établissement** : `NotFound` exact ; l'encadrant du temps pointe normalement. | `s4_pointage_un_encadrant_*` |
| **3** portée `InternatAppel` sans droit de confidentialité (**D-069**) | `pointer_temps_a` : seuls l'**encadrant du temps** et les détenteurs du droit de confidentialité pointent un élève protégé. Un détenteur de portée (`internat.appel`) sans ce droit reçoit « n'est pas inscrit », comme un tiers ; il pointe toujours un élève non protégé (la portée reste en vigueur). La direction (droit de confidentialité) pointe un protégé inscrit. | `s4_pointage_un_protege_*` |
| **4** `internat_sorties_tache_etab_a` | `pub(crate)`. | (compilation ; test existant `m6_*`) |
| **5** avis de retour | `internat.retour_demande` part aussi aux **accompagnateurs de la sortie** (membres de l'établissement) et **désigne la sortie** : titre et heure de fin (fuseau de l'établissement, `18h00`), jamais l'élève. Un compte du personnel qui n'est ni responsable ni accompagnateur ne reçoit rien. | `s4_fg_un_parent_interdit_*` |
| **6** code mort `retire_le IS NULL OR $4` | **Déjà absent** du dépôt (retiré en passe 3) : aucune occurrence. La lecture du pointage ne vise que la ligne active ; la veille ne retient que la ligne portant `pointage = 'present'` (le `ORDER BY retire_le DESC NULLS FIRST` n'est plus qu'un départage déterministe). Rien à changer. | (couvert par `s3_*`) |
| **7** avis regroupés | Le texte (le nombre) d'un avis regroupé est **mis à jour** (upsert) quand un second événement de la minute arrive après la première distribution : `ent_notification` (`DO UPDATE … WHERE lu_le IS NULL`) et l'envoi externe encore `en_attente`. Un avis déjà **lu** garde son texte. Les autres appelants de `notifier_personne` sont inchangés (`DO NOTHING`). | `s4_avis_regroupe_*` |

### Non fait / limites
- Point 6 : aucune modification de code (déjà fait en passe 3).
- Le texte d'un avis regroupé déjà **parti** (envoi externe non `en_attente`) n'est pas rattrapé.
- Un compte accompagnateur dont l'appartenance à l'établissement a pris fin ne reçoit pas l'avis (jointure sur `ent_membership`).
- Le consommateur d'avis n'est toujours pas filtré par périmètre d'internat (inchangé).

### Tests, compteurs, vérifications (suites 2)
- **9 tests nouveaux** (`tests_internat_i4_suites2.rs` : `s4_fg_*` ×6, `s4_pointage_*` ×2, `s4_avis_regroupe_*` ×1) ; 1 test existant resserré (`s3_fg_*`). Compteurs (règle 28) : migrations +0, `MIGRATION_TAG`, `Action::TOUTES`, RLS, registre, `TABLES_ELEVE`, `SOURCES`, routes, événements, consommateurs : **inchangés**.
- **Local** : base neuve `ent_i4s4_test` (PostgreSQL 16, `ENT_EXIGER_BASE=1`, `ENT_ENV=test`, `--locked`, série) : 9 tests `s4_*` verts, puis 388 tests (`internat`, `notif`, `migrat`, `garde_search_path`, `tests_rls`, `registre`) verts. `rustfmt` ciblé sur les fichiers propres à l'origine (pas de `cargo fmt --all`). Écart d'outillage : le conteneur n'a que rustc/clippy 1.97 ; clippy 1.99 (trois commandes, `ent-lab` compris) est celui de la CI.

### Tête et CI (suites 2)
- Tête de code : `8a5a086e` ; PR n° 135, en brouillon vers `ENT`. Aucune fusion d'`ENT` nécessaire.
- CI « ENT Rust » sur `8a5a086e` : **verte**. Les commits suivants ne touchent que cette documentation.

### Correctifs du pilote après contre-revue des suites 2
- **M1** (protégé discernable d'un non-inscrit) : dans `pointer_temps_a`, l'inscription et la protection se contrôlent désormais **avant** l'état et la fenêtre du temps, par le même refus « n'est pas inscrit » ; un temps annulé ou hors fenêtre ne distingue plus un protégé d'un élève non inscrit. Test `s4_pointage_un_protege_est_indiscernable_d_un_non_inscrit_hors_fenetre_ou_temps_annule`.
- **M2** (titre de sortie libre en clair hors de l'ENT) : `internat_retour_demande` rejoint `canal_externe_neutre` : e-mail, push et SMS neutres, détail dans l'ENT seulement. Test `s4_la_demande_de_retour_est_neutre_hors_de_l_ent`.
- Vérifié en local : clippy 1.99 (3 crates, `labo`) sans avertissement ; `tests_internat_i4*` : 105 verts.
- **En file** : F1 (cas d'une minute dont l'avis est déjà lu : remettre à non lu ou documenter), F2 (envoi externe déjà parti non mis à jour : à documenter), F4 (test d'avis regroupé : preuve que le 3e événement est consommé ; `notifier_personne` inchangé ; envoi `envoye` non réécrit), F6 (libellé `refus_apres_depart`), F7 (connexion supplémentaire de `eleve_cache_au_tiers` dans une transaction ouverte).

## Suites 3

Branche `ent-socle-41-code-i4-suites-3` (base `origin/ENT`, PR en brouillon vers `ENT`). **Aucune migration** (2828 à 2830 inchangées, règle 25 ; pas de 2831). Niveau : **implémenté, testé en local** ; ni validé, ni prêt pilote. Frontend non touché. Aucun test ne dépend de l'heure réelle (minute posée une fois par la base ; jours fixes).

### Fait

| Point | Ce qui a été fait | Test |
|---|---|---|
| **F1** avis lu | **Décision (D-069, prudente)** : quand le texte d'un avis regroupé change (le nombre d'élèves augmente), l'avis dans l'ENT est mis à jour **et remis à non lu** (`lu_le = NULL`). Lire un avis une fois ne doit pas cacher qu'un autre élève s'est ajouté dans la même minute. Un avis dont le texte ne change pas reste lu. `notifications.rs`, clause `DO UPDATE` de `notifier_personne_selon`. | `s5_avis_regroupe_lu_repasse_non_lu_*` ; `s4_avis_regroupe_*` resserré |
| **F2** envoi externe déjà parti | **Documenté, pas d'avis rectificatif** : l'envoi externe n'est mis à jour que tant qu'il est `en_attente` ; un envoi parti (`envoye` et tout autre statut) n'est jamais réécrit (on ne falsifie pas un message remis). L'avis de l'ENT, lui, porte le nombre à jour et repasse non lu. Un avis rectificatif aurait doublé les messages externes pour une rafale : écarté, à rouvrir si le pilote le demande. | même test |
| **F4** | Le 3e événement est consommé (texte « 3 élèves », un seul avis, plus aucun événement sans traitement) ; `notifier_personne` inchangé (`DO NOTHING`, test direct) ; un envoi `envoye` n'est jamais réécrit. | `s5_avis_regroupe_*`, `s5_notifier_personne_ne_reecrit_*` |
| **F6** libellé | `libelle_fait_groupe_famille("refus_apres_depart")` (`ent-types`) : « Votre refus a bien été enregistré. Votre enfant était déjà parti avec le groupe : l'établissement a été prévenu et demande son retour. En cas d'urgence, appelez directement l'établissement. » Sans code interne, sans nom d'élève, sans promesse d'heure. Les autres faits n'ont pas de libellé famille (`None`). **Écart** : aucun écran ni DTO famille n'expose encore le journal des faits ; la fonction est prête, son branchement viendra avec l'écran famille (non fait ici). | `tests_libelles_famille` (2) |
| **F7** connexion supplémentaire | `eleve_cache_au_tiers_conn` (+ chaîne `profil_champs_conn`, `droits_effectifs_conn`, `eleves_autorises_conn`, `droits_delegues_conn`, `charger_droits_conn`, `eleves_suivis_par_prof_conn`, `eleves_en_lecture_seule_du_prof_conn`, `proteges_caches_conn`, `aujourdhui_etablissement_conn`) : mêmes requêtes sur la connexion fournie. Les versions par le pool ouvrent **une** transaction pour toute la chaîne (au lieu d'une par requête) et appellent la version `_conn` : même résultat. Les deux appelants en transaction (`internat_sorties/groupes.rs` filtre des invités, `vie.rs` pointage) utilisent la connexion de leur transaction. Les autres appelants en transaction de `eleve_cache_au_tiers` (`service.rs`) sont inchangés (hors périmètre). | `s5_eleve_cache_au_tiers_*` (pool d'une seule connexion, délai) |

### Non fait / limites
- Libellé F6 non branché à une surface famille (voir ci-dessus).
- Pas d'avis rectificatif externe (F2).
- `service.rs` (2 appels de `eleve_cache_au_tiers`) non convertis : hors du lot internat.

### Tests, compteurs, vérifications
- **3 tests backend** (`tests_internat_i4_suites3.rs` : `s5_*`) + **2 tests purs** (`ent-types`, libellé F6) ; `s4_avis_regroupe_*` resserré (l'avis lu repasse non lu). Local : base neuve `ent_i4s5_test` (PostgreSQL 16, `ENT_EXIGER_BASE=1`, `ENT_ENV=test`, `--locked`, série) : 54 tests (`s4_*`, `s5_*`) verts. Le reste de la suite est laissé à la CI (le refactor F7 touche le moteur de droits : `droits_effectifs`, `eleves_autorises`, `profil_champs`).
- Compteurs (règle 28) : migrations +0, `MIGRATION_TAG`, `Action::TOUTES`, RLS, registre, `TABLES_ELEVE`, `SOURCES`, routes, événements : **inchangés**.
- Clippy (conteneur : 1.97, pas 1.99) sur `ent-db` + `ent-types` avec `labo` et tests : aucun avertissement dans les lignes touchées (2 avertissements antérieurs, `tests_voyages_v2a.rs`). Les trois commandes de la CI (clippy 1.99, `ent-lab` compris) restent à la CI. Pas de `cargo fmt --all`.

### Tête et CI (suites 3)
- PR n° 142, en brouillon vers `ENT`. Tête de code : `3286b534` (un premier passage, sur `096d3bba`, était rouge sur le seul contrôle de format ; corrigé par `rustfmt` ciblé sur `ent-types` et le nouveau fichier de tests).
- CI « ENT Rust » sur `3286b534` : **verte** (run 45353586522). Les commits suivants ne touchent que cette documentation.
