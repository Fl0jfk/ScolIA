# Socle 11 — complément du palier MB2 : parité web / appli sur la matrice complète, clés non lues (livraison)

Branche `claude/project-thread-uy5fn6` (base `origin/ENT` = `06333a59`). Conception : `11_APPLICATIONS_MOBILES.md` §7.2 et §7.3 (branche `ent-socle-11-conception`) ; restes de `11_MB2_LIVRAISON.md` (« Non fait » : décodage tolérant, champ renommé).

**Aucune migration, aucun code de production modifié** (serveur, cœurs, écrans) : ce palier n'ajoute que des tests, une liste déclarative et cette documentation. Aucun `Cargo.toml`, `Cargo.lock` ni fichier de build modifié. Les fichiers des PR ouvertes (165 à 172, dont `tests_contrat_mobile.rs`, `routes.txt`, `ContratMb2Test*`, `Modeles.*`, `ApiClient.*`, `PROJECT_STATUS.md`) ne sont pas touchés ; d'où des fichiers de test nouveaux plutôt qu'une extension des bancs existants.

## Ce que MB2 prouvait, ce qui manquait
| §7.2 / §7.3 | MB2 (versé) | Ce palier |
|---|---|---|
| Deux clients (la conception dit « cookie web » et « Bearer mobile ») | une seule session, seul l'en-tête `X-ScolIA-Appli` changeait | **deux sessions distinctes par personne** : celle du web (connexion sans `client`) et celle de l'appli (`"client":"mobile"` + en-tête). Toutes deux en Bearer : l'API ne lit pas de cookie, le cookie du web est tenu par `ent-web-server`, qui appelle l'API avec le jeton. Le banc vérifie que la session de l'appli est de nature web et de portée complète (MEN-11, jusqu'à MB4) |
| Personas | 9 | **12** : administrateur, direction, administratif, comptable, CPE, professeur, infirmière, psychologue, parent avec tous les droits (Lily), parent de l'autre foyer (Narcissa), parent sans droit aux notes ni à la vie scolaire (Molly), élève (Harry) |
| Cibles | lié, quelques refus explicites, un professeur d'un autre établissement | élève ou fil : **lié, non lié, autre établissement, inexistant** ; appel ou cours : celui du labo, autre établissement, inexistant ; classe : la sienne et une autre. **755 cellules × 2 sessions**, couverture exigée cellule par cellule (une cellule qui ne se joue plus rend le test rouge), au moins deux réponses non vides par persona, jour = celui de l'appel ouvert par le harnais (un jour avec créneaux, vérifié) |
| Oracle d'existence | — | l'objet d'un autre établissement doit recevoir **exactement** la réponse d'un inexistant (statut, code ; en succès, le corps), au web et dans l'appli |
| Écritures | refus `/eva/*` | **88 refus** : ouvrir, marquer, clore un appel (familles ; appel de l'autre établissement), justifier (autre famille, parent sans droit de justifier, élève sur un camarade, autre établissement), répondre à un fil (autre famille, autre établissement), révoquer la session ou désinscrire le push d'autrui ; même statut au web et dans l'appli, code `droit.refuse` ou `objet.introuvable` exigé (jamais un refus d'authentification), **aucune trace** (appels, absences et fils de A et de B relus avant et après ; session visée vivante), puis **témoins** : le même geste par qui en a le droit réussit (chemin et corps valides) |
| Droits du lien | parent sur l'enfant d'un autre | parent sans droit aux notes (Molly sur Ginny : refus), avec contrôles positifs (Narcissa sur Draco, Lily, Harry) |
| Oracle de fuite | — | aucune réponse ne contient un identifiant **connu** de l'autre établissement : ceux du harnais (élèves, foyers, personnes, documents, fils, factures, absences, matières, appel, compte) et, ajoutés ici, son cours, ses périodes et ses créneaux. Les devoirs, notes, annonces, notifications et sessions de B n'y sont pas |
| Complétude (§7.2 point 7) | liste blanche jouée par le banc MB2 | chaque route de `routes.txt` a une ligne dans ce banc (lecture, écriture refusée) ou une exclusion **motivée**, et chaque chemin joué correspond à son gabarit ; variantes du client jouées (`/vs/absences` sans jour, `/messages/fils?eleve_id=`) ; une route ajoutée sans ligne, ou une ligne périmée, rend le test rouge (sans base) |
| Champ renommé côté serveur (limite M6 de MB2) | invisible si le client a une valeur par défaut | **clés non lues** : chaque clé non nulle d'une fixture doit être lue par le modèle, ou déclarée dans `contrat/mobile/cles_ignorees.txt` avec sa raison. Limite : une clé nulle dans toutes les fixtures ou un élément de liste toujours vide n'est pas éprouvé (ex. `decision_note`, `motif_justif` du résumé, `signe_at`, `dossier` et `modele_id` des fils, `appareil` des sessions, `publication_prevue_le`, `retiree_le`, `pieces[]`, `avertissements[]`, `cibles[]`, la plupart des champs de `classes_effectif.eleves[]`) : des fixtures plus riches les éprouveraient |

## Fichiers
| Fichier | Rôle |
|---|---|
| `ent-rust/crates/ent-api/tests/parite_mobile.rs` | banc de parité (chemin prévu par la conception §7.2), sur le harnais `tests/separation/harness.rs` (deux établissements de labo, routeur réel) ; `#![cfg(feature = "labo")]` |
| `ent-rust/apps/android/core/src/test/kotlin/fr/scolia/mobile/core/ContratClesTest.kt` | parcourt chaque fixture avec le descripteur `kotlinx.serialization` du modèle qui la décode et liste les clés inconnues du modèle |
| `ent-rust/apps/ios/Tests/ScolIACoreTests/ContratClesTests.swift` | décode chaque fixture par la vraie méthode du client, réencode la valeur et liste les clés de la fixture absentes du réencodage |
| `ent-rust/contrat/mobile/cles_ignorees.txt` | clés ignorées volontairement : `tous` (les deux cœurs) ou `ios` (écart de parité Android / iOS) ; une ligne périmée rend les tests rouges |

## Constats
- **PM-01 (faible)** : dans le **même établissement**, un parent qui vise la justification d'une absence ou la réponse à un fil d'une autre famille reçoit `403 droit.refuse`, un identifiant inexistant `404 objet.introuvable` : l'existence de l'objet se lit. Les identifiants sont des UUID v4 non devinables, et l'autre établissement, lui, est indistinct (prouvé). Cellules gardées en refus, comparaison à l'inexistant levée et marquée `PM-01` dans le banc. Correctif proposé : répondre 404 quand l'objet n'est pas dans la portée de la personne (comme pour l'autre établissement), dans un lot du socle 12 / 16 ; resserrer alors les cellules.
- **PM-02 (moyen, parité Android / iOS)** : **46 clés** lues par le cœur Kotlin ne le sont pas par le cœur Swift (fils : `archive`, `priorite`, `created_at`… ; messages : `lu_par_moi`, `nb_accuses` ; devoirs : `coef` ; enfants : `lien`, `peut_autoriser_sorties` ; carnet à signer ; dates de période ; semaine A/B de l'EDT…). Elles sont listées `ios` dans `cles_ignorees.txt`. L'écran iOS qui en aurait besoin ne peut pas les afficher (ex. : un fil archivé ou prioritaire n'est pas distingué). `Modeles.swift` est modifié par la PR 171 : l'alignement se fait après son versement, ligne retirée par ligne alignée (le test rouge si une ligne `ios` devient lue).
- **PM-03 (faible)** : un élève lit le cahier de textes d'une **autre classe** (`/cahier-texte?classe=5A` pour Harry, en 4B), au web comme dans l'appli. Les parents, eux, n'obtiennent rien. Exception nommée dans le banc ; à trancher par le socle 20 (le cahier d'une classe est-il public dans l'établissement ?).
- **12 clés ignorées par les deux cœurs**, chacune motivée (aperçus de diffusion des annonces, casquette de session avant MB5, masques et site de l'effectif, signature du carnet non portée).
- Validation avant autorisation : un corps invalide reçoit 422 avant le contrôle des droits, y compris sur l'objet d'un autre établissement. Rien n'est révélé (même réponse pour tout identifiant) ; noté pour la conception de l'enveloppe, sans action.

## Règles non tranchées → défaut prudent (D-069)
1. **Session « mobile » par mot de passe** : tant que la clé d'appareil n'existe pas (MB4), le serveur l'ouvre comme une session web (MB0, MEN-11). Le banc compare donc deux sessions distinctes de même nature ; la comparaison avec une session signée viendra avec MB4.
2. **Liste des sessions** : mêmes identifiants des deux côtés, une seule session courante de chaque côté, et pas la même (le marqueur `courante` diffère légitimement) ; tout le reste par corps exact, `trace` exceptée.
3. **Écritures** : les refus d'abord, la relecture « aucune trace », puis les témoins autorisés (marquer et clore l'appel du labo, justifier l'absence de Harry, répondre à son fil, désinscrire le push de Lily, révoquer une session ouverte pour l'occasion) ; la session web du harnais n'est jamais révoquée (elle sert aux autres tests du même processus).

## Non fait (et pourquoi)
- Manifeste `/mobile/contexte` et « aucun écran hors manifeste » (§7.2 point 4, §7.3) : MB5.
- Baux hors ligne ⊆ lecture en ligne, opérations différées relues à la réception (§7.2 points 5 et 6) : MB7.
- Remplaçant en mission et hors mission, AED, élève majeur, parent séparé ou interdit, professeur d'une autre classe : absents du semis du harnais ; à ajouter avec les personas du socle 4 / 6. La personne d'un autre établissement est jouée par le banc MB2 (professeur de B) ; ici, l'autre établissement est joué comme cible.
- Purge sur 401 et horloge truquée côté cœurs (§7.3) : couverts en partie par `HorsLigne` (MB1), pas repris ici.
- Alignement des modèles Swift (PM-02) : après la PR 171.
- `PROJECT_STATUS.md` non mis à jour (modifié par les PR 170 et 172) : à reporter au versement.

## Tests (local, 2026-11-28)
- Rust (1.97, PostgreSQL 16 local, `ENT_ENV=test` comme la CI, feature `labo`) : `parite_mobile` **4/4**, chaque test aussi seul (processus séparés comme sous nextest). `cargo clippy -p ent-api --tests --features labo -D warnings` : 0 avertissement ; `rustfmt` 1.99 : fichier conforme.
- Mutation vérifiée à la main (non versée) : donner à l'appli de Lily la session de Narcissa fait échouer le banc des lectures ; une erreur d'ordre dans la construction des chemins (cible « inexistant » remplacée par l'objet du labo) a été détectée par l'oracle d'existence pendant les correctifs.
- Cœur Kotlin (`-Pscolia.coreSeulement=true :core:test`) : tous verts, dont `ContratClesTest` 2/2 (décodeur du vrai client).
- Cœur Swift (`swift:6.1-noble`, `swift test`) : **70/70**, dont `ContratClesTests` 2/2.
- Le test « clé renommée » de chaque cœur prouve la détection : `portee` renommée `portee_session` dans `auth_me` est signalée alors que le décodage tolérant l'accepte.

## Contre-revue (agent indépendant, 2026-11-28)
- Élevés : cellules sautées en silence quand un objet du labo manque (**corrigé** : objets exigés à l'ouverture, couverture par cellule) ; `moyenne_sur_20` signalée à tort côté Swift (`convertToSnakeCase` ne rend pas les chiffres) (**corrigé** : comparaison par nom ou nom Swift, réencodage sans stratégie ; ligne retirée, PM-02 = 46).
- Moyens corrigés : refus accepté pour n'importe quelle raison (codes exigés, témoins) ; « aucune trace » partielle (absences, fils, appel de B relus) ; objets de B facultatifs ; trous du cloisonnement des familles (cahier par élève, fils, classes jouées en lié et non lié, classe propre à chaque parent) ; fil « lié » de Molly ; dépendance au jour (jour de l'appel) ; oracle de fuite élargi et doc reformulée ; clés jamais éprouvées documentées.
- Faibles corrigés : nature de la session de l'appli figée par le test ; comparaison des sessions renforcée ; visées en double retirées, grille de B jouée avec la période de B ; contenu exigé par persona ; gabarits vérifiés ; décodeur du vrai client côté Kotlin ; parseurs des clés ignorées alignés (double déclaration entre plateformes refusée) ; doc (`ENT_ENV`, cookie, personas).
- Restent : table fixture → modèle recopiée côté Kotlin (vérifiée, contrôlée en partie par le décodage) ; biais possible du réencodage Swift pour un `init(from:)` écrit à la main (documenté).
