# Socle 11 — complément du palier MB2 : parité web / appli sur la matrice complète, clés non lues (livraison)

Branche `claude/project-thread-uy5fn6` (base `origin/ENT` = `06333a59`). Conception : `11_APPLICATIONS_MOBILES.md` §7.2 et §7.3 (branche `ent-socle-11-conception`) ; restes de `11_MB2_LIVRAISON.md` (« Non fait » : décodage tolérant, champ renommé).

**Aucune migration, aucun code de production modifié** (serveur, cœurs, écrans) : ce palier n'ajoute que des tests, une liste déclarative et cette documentation. Aucun `Cargo.toml`, `Cargo.lock` ni fichier de build modifié. Les fichiers des PR ouvertes (165 à 172, dont `tests_contrat_mobile.rs`, `routes.txt`, `ContratMb2Test*`, `Modeles.*`, `ApiClient.*`, `PROJECT_STATUS.md`) ne sont pas touchés ; d'où des fichiers de test nouveaux plutôt qu'une extension des bancs existants.

## Ce que MB2 prouvait, ce qui manquait
| §7.2 / §7.3 | MB2 (versé) | Ce palier |
|---|---|---|
| Deux clients (session web, session de l'appli) | une seule session, seul l'en-tête `X-ScolIA-Appli` changeait | **deux sessions réelles par personne** : celle du web (connexion sans `client`) et celle de l'appli (`"client":"mobile"` + en-tête) |
| Personas | 9 | **12** : administrateur, direction, administratif, comptable, CPE, professeur, infirmière, psychologue, parent avec tous les droits (Lily), parent de l'autre foyer (Narcissa), parent sans droit aux notes ni à la vie scolaire (Molly), élève (Harry) |
| Cibles | lié, quelques refus explicites, un professeur d'un autre établissement | **lié, non lié, autre établissement, inexistant** pour chaque lecture à identifiant |
| Oracle d'existence | — | l'objet d'un autre établissement doit recevoir **exactement** la réponse d'un inexistant (statut, code ; en succès, le corps), au web et dans l'appli |
| Écritures | refus `/eva/*` | **88 refus** : ouvrir, marquer, clore un appel (familles ; appel de l'autre établissement), justifier (autre famille, parent sans droit de justifier, élève sur un camarade, autre établissement), répondre à un fil (autre famille, autre établissement), révoquer la session ou désinscrire le push d'autrui ; même statut au web et dans l'appli, enveloppe codée, **aucune trace** (appel relu identique, session et inscription de Lily intactes) |
| Droits du lien | parent sur l'enfant d'un autre | parent sans droit aux notes (Molly sur Ginny : refus), avec contrôles positifs (Narcissa sur Draco, Lily, Harry) |
| Oracle de fuite | — | aucune réponse ne contient un identifiant de l'autre établissement (oracle du harnais de séparation) |
| Complétude (§7.2 point 7) | liste blanche jouée par le banc MB2 | chaque route de `routes.txt` a une ligne dans ce banc (lecture, écriture refusée) ou une exclusion **motivée** ; une route ajoutée sans ligne, ou une ligne périmée, rend le test rouge (sans base) |
| Champ renommé côté serveur (limite M6 de MB2) | invisible si le client a une valeur par défaut | **clés non lues** : chaque clé non nulle d'une fixture doit être lue par le modèle, ou déclarée dans `contrat/mobile/cles_ignorees.txt` avec sa raison |

## Fichiers
| Fichier | Rôle |
|---|---|
| `ent-rust/crates/ent-api/tests/parite_mobile.rs` | banc de parité (chemin prévu par la conception §7.2), sur le harnais `tests/separation/harness.rs` (deux établissements de labo, routeur réel) ; `#![cfg(feature = "labo")]` |
| `ent-rust/apps/android/core/src/test/kotlin/fr/scolia/mobile/core/ContratClesTest.kt` | parcourt chaque fixture avec le descripteur `kotlinx.serialization` du modèle qui la décode et liste les clés inconnues du modèle |
| `ent-rust/apps/ios/Tests/ScolIACoreTests/ContratClesTests.swift` | décode chaque fixture par la vraie méthode du client, réencode la valeur et liste les clés de la fixture absentes du réencodage |
| `ent-rust/contrat/mobile/cles_ignorees.txt` | clés ignorées volontairement : `tous` (les deux cœurs) ou `ios` (écart de parité Android / iOS) ; une ligne périmée rend les tests rouges |

## Constats
- **PM-01 (faible)** : dans le **même établissement**, un parent qui vise la justification d'une absence ou la réponse à un fil d'une autre famille reçoit `403 droit.refuse`, un identifiant inexistant `404 objet.introuvable` : l'existence de l'objet se lit. Les identifiants sont des UUID v4 non devinables, et l'autre établissement, lui, est indistinct (prouvé). Cellules gardées en refus, comparaison à l'inexistant levée et marquée `PM-01` dans le banc. Correctif proposé : répondre 404 quand l'objet n'est pas dans la portée de la personne (comme pour l'autre établissement), dans un lot du socle 12 / 16 ; resserrer alors les cellules.
- **PM-02 (moyen, parité Android / iOS)** : **47 clés** lues par le cœur Kotlin ne le sont pas par le cœur Swift (fils : `archive`, `priorite`, `created_at`… ; messages : `lu_par_moi`, `nb_accuses` ; devoirs : `coef` ; moyennes : `moyenne_sur_20` ; enfants : `lien`, `peut_autoriser_sorties` ; carnet à signer ; dates de période ; semaine A/B de l'EDT…). Elles sont listées `ios` dans `cles_ignorees.txt`. L'écran iOS qui en aurait besoin ne peut pas les afficher (ex. : un fil archivé ou prioritaire n'est pas distingué). `Modeles.swift` est modifié par la PR 171 : l'alignement se fait après son versement, ligne retirée par ligne alignée (le test rouge si une ligne `ios` devient lue).
- **12 clés ignorées par les deux cœurs**, chacune motivée (aperçus de diffusion des annonces, casquette de session avant MB5, masques et site de l'effectif, signature du carnet non portée).
- Validation avant autorisation : un corps invalide reçoit 422 avant le contrôle des droits, y compris sur l'objet d'un autre établissement. Rien n'est révélé (même réponse pour tout identifiant) ; noté pour la conception de l'enveloppe, sans action.

## Règles non tranchées → défaut prudent (D-069)
1. **Session « mobile » par mot de passe** : tant que la clé d'appareil n'existe pas (MB4), le serveur l'ouvre comme une session web (MB0, MEN-11). Le banc compare donc deux sessions distinctes de même nature ; la comparaison avec une session signée viendra avec MB4.
2. **Liste des sessions** comparée par statut et nombre d'éléments (le marqueur `courante` diffère légitimement) ; tout le reste par corps exact, `trace` exceptée.
3. **Écritures** : seuls des refus sont joués (aucune écriture permise n'est rejouée sur le labo partagé du binaire) ; les succès sont prouvés par le banc MB2 et ses fixtures.

## Non fait (et pourquoi)
- Manifeste `/mobile/contexte` et « aucun écran hors manifeste » (§7.2 point 4, §7.3) : MB5.
- Baux hors ligne ⊆ lecture en ligne, opérations différées relues à la réception (§7.2 points 5 et 6) : MB7.
- Remplaçant en mission et hors mission, AED, élève majeur, parent séparé ou interdit : absents du semis de labo ; à ajouter avec les personas du socle 4 / 6.
- Purge sur 401 et horloge truquée côté cœurs (§7.3) : couverts en partie par `HorsLigne` (MB1), pas repris ici.
- Alignement des modèles Swift (PM-02) : après la PR 171.
- `PROJECT_STATUS.md` non mis à jour (modifié par les PR 170 et 172) : à reporter au versement.

## Tests (local, 2026-11-28)
- Rust (1.97, PostgreSQL 16 local, `ENT_ENV=test` comme la CI, feature `labo`) : `parite_mobile` **4/4** (complétude, lectures : plus de 600 cellules × 2 sessions, droits du lien, 88 écritures refusées). Mutation vérifiée : donner à l'appli de Lily la session de Narcissa fait échouer le banc (18 écarts). `cargo clippy -p ent-api --tests --features labo -D warnings` : 0 avertissement ; `rustfmt` 1.99 : fichier conforme.
- Cœur Kotlin (`-Pscolia.coreSeulement=true :core:test`) : tous verts, dont `ContratClesTest` 2/2.
- Cœur Swift (`swift:6.1-noble`, `swift test`) : **70/70**, dont `ContratClesTests` 2/2.
- Le test « clé renommée » de chaque cœur prouve la détection : `portee` renommée `portee_session` dans `auth_me` est signalée alors que le décodage tolérant l'accepte.
