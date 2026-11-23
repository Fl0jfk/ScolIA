# Socle 16 — lot « suite » : flux M12 (interrogation longue) et pièce sur son propre message différé

Branche `ent-socle-16-code-suite`, créée depuis `origin/ENT` (02659006c, déjà à jour à la fin du lot). **Aucune migration** (la plage 491–497 reste libre ; 3300–3309 non utilisée).
Niveau : **implémenté et testé en local, non validé par Florian, non prêt pilote.** Base PostgreSQL 16 locale neuve `ent_msg_test`, `ENT_EXIGER_BASE=1`.

## Choix du lot
M7 (carnet et cartes) et M8 (élèves) dépendent de plusieurs socles propriétaires et ne sont pas faits ; la consigne attendait M9, M10, M12. Ce lot prend **M12 (partie serveur)**, la plus autonome, et la suite connue (pièce sur message différé). **M7, M8, M9, M10, M11 restent à faire.**

## Fait
| Sujet | Où | Test |
|---|---|---|
| **Pièce sur son propre message différé** : l'**auteur** peut joindre une pièce avant la remise (elle naît en quarantaine, suit le message, n'est visible de personne d'autre : lecture des pièces déjà filtrée par `auteur = lecteur` ou remise passée) ; un autre participant reste refusé (409) ; retiré : toujours refusé ; limite de 5 toujours sous verrou | `msg_repo.rs::insert_piece` (relit l'auteur sous `FOR UPDATE`) | `corr_f1_…` mis à jour (autre auteur refusé, aucune ligne ; auteur accepté) |
| **M12 flux « quelque chose a changé »** : `GET /messages/flux?apres=<curseur>&attente=<s ≤ 25>` ; **aucun contenu** (identifiant du fil et instant) ; vue `ent_msg_fil_lisible` relue à **chaque** interrogation : aucun événement pour un non-participant, plus rien dès qu'un participant est retiré ; message différé muet avant sa remise ; curseur = heure de la base − 5 s (un message validé en retard n'est pas perdu, un doublon est inoffensif) ; même route pour le web et l'appli unique (parité des droits, D-003) | `msg_repo.rs::evenements_depuis` (classée « filtre intégré » dans la garde), `msg_m12.rs`, route `flux_messagerie` | `m12_flux_participants_seulement_sans_contenu_et_retrait`, `m12_flux_message_differe_invisible_avant_remise` |

## Non fait / écarts
1. **Pas de SSE** : l'interrogation longue ne demande aucune dépendance. Un flux SSE exige `tokio-stream` ou `futures-util` (déjà dans `Cargo.lock`, mais nouvel arc pour `ent-api`) : **à valider avant**. Le contrat (route, curseur, payload) reste valable.
2. Pas d'attente active côté base (pas de `LISTEN/NOTIFY`) : relecture toutes les 2 s par appel en attente ; à revoir si la charge l'exige.
3. Pas de test HTTP de la route (les tests couvrent le service) ; brouillons hors ligne et dictée (M12) non faits ; aucun écran Leptos.
4. M7, M8, M9, M10, M11, M13 à M17 : non faits.

## Vérifications
- ent-db (labo, base démarrée) : 199 tests de messagerie / gardes / isolation / corrections réussis, 0 échec ; `isolation_sql_statique` et `jour_utc_interdit` verts. Un premier lancement de `corr_f1` a échoué une fois (première migration de la base neuve, rejoué vert) ; à surveiller.
- clippy 1.99 `-D warnings`, `ent-db` et `ent-api` (labo, `--all-targets`) : 0 avertissement. Tous les `--locked`. Pas de `cargo fmt --all` (rustfmt non installé pour 1.99 ici, fichiers `ent-db` hors périmètre de formatage). Web non touché.
- Aucun test à l'heure réelle ni au jour UTC (instants lus dans la base).

## Compteurs (règle 28)
Tables avec RLS : **+0** ; `Action::TOUTES` inchangé ; routes **+1** (`/messages/flux`) ; `MIGRATION_TAG` inchangé.

## Correctif CI n° 479 (suite)
- `corr_f1_…` paniquait sur `Domain(MsgPiecePasAuteur)` : **pas une régression du code**. Le domaine (`plan_attacher_piece`) refuse tout non-auteur *avant* le dépôt ; le test attendait un conflit (repli du dépôt, seconde barrière). Le test accepte désormais `MsgPiecePasAuteur` ou le conflit pour « un autre que l'auteur » (aucune ligne créée), et garde : cinq pièces au plus sous verrou, refus sur message **retiré**, auteur accepté sur son message différé. Aucune règle de sécurité affaiblie.
- Le correctif a d'abord manqué l'import de `DomainError` (vu à la compilation locale, jamais poussé vert).
- Local (base neuve `ent_msg_test2`, `ENT_EXIGER_BASE=1`) : 124 tests (messagerie, `corr_`, M12, `isolation_sql_statique`, `jour_utc_interdit`) verts, 0 échec. Fusion d'`origin/ENT` faite (ebac151) ; compteurs inchangés (RLS +0, routes +1, migrations 0).
- Toujours non fait : SSE, LISTEN/NOTIFY, test HTTP du flux, brouillons hors ligne, dictée, M7–M11, M13–M17. Compilation de `ent-db` : tuée par manque de mémoire avec les réglages par défaut ici ; passée avec `CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_CODEGEN_UNITS=64`.

## Correctifs de revue (passe du pilote, 2026-11-22)

Niveau : **implémenté et testé en local, non validé par Florian, non prêt pilote.** Base PostgreSQL 16 locale neuve `ent_s16_test` (nom « test »), `ENT_EXIGER_BASE=1`, `RUST_MIN_STACK=16777216`. Aucune migration ; aucune donnée ni secret de production. Branche partie de la tête distante `ecbe20a2` ; `origin/ENT` n'avait pas bougé (aucune fusion à faire).

### Fait
| Point | Correctif | Test |
|---|---|---|
| **RUSTSEC-2026-0592** (`url`) | `ENT` ne contenait que `url` 2.5.8 : correctif de la PR 121 appliqué tel quel (`cargo +1.99.0 update -p url --precise 2.5.9` dans `ent-rust` et `ent-rust/web` ; `form_urlencoded` 1.2.3 suit, imposé par `url`), rien d'autre dans les `Cargo.lock` | `cargo +1.99.0 metadata --locked` dans `web` : réussi |
| **M1** (règle 1) | Le flux garde la vue de participation `ent_msg_fil_lisible` comme **ensemble de candidats**, puis passe **chaque fil** par `actor_can_access_fil` avec le **rôle de la session** (la route le transmet), à chaque appel : même jugement que la lecture (droit sur l'élève, fils réservés, boîtes, rôle). Les candidats écartés ne sont pas rejugés à chaque tour de l'interrogation longue | `revue_m1_flux_suit_le_jugement_de_lecture_avec_le_role` : un personnel participant sans droit reçoit exactement ce que la lecture lui accorde (rien s'il ne peut pas lire), un rôle de session différent ne reçoit rien, une interdiction posée ensuite vide le flux |
| **M2** (règle 2) | Ordre total `(instant, fil)` ; position rendue = **dernier événement rendu** quand la liste est tronquée (`tronque`, `curseur`, `curseur_fil`, rappel avec `apres_fil`) ; sinon `min(maintenant − 5 s, début de la plus ancienne transaction en cours)`. `apres` au-delà de **600 s** : `resynchro = true`, rien d'autre (le client recharge tout) | `revue_m2_liste_tronquee_le_curseur_ne_saute_aucun_evenement` : limite 2 sur 4 fils, quatre événements **au même instant exact** avec limite 1 (aucun saut, aucun doublon), fenêtre maximale, témoin d'un curseur récent |
| **M3** | `created_at` d'un message est posé **par la base** (`now()`) à l'insertion, plus par l'application ; le curseur est lu dans la même base ; un message validé tard n'est pas perdu : le curseur ne dépasse jamais le début de la plus ancienne transaction en cours (`pg_stat_activity`), la marge de 5 s ne sert plus que de plancher | `revue_m3_created_at_vient_de_la_base` (horodatage d'application de 10 ans en arrière ignoré) ; `revue_m3_message_valide_tard_n_est_pas_perdu` (transaction ouverte, 6 s d'attente > marge, validation, puis le message paraît) |
| **F1** | `GET /messages/flux` : **un appel à la fois et un par seconde au plus par compte** (`ent-api/src/flux_limite.rs`, 429 au-delà) ; `attente=0` ne permet pas le martèlement | `flux_limite::tests::un_seul_appel_a_la_fois_et_pas_de_martelement` ; HTTP `tests_flux_m12` (429 sur rappel immédiat, 429 sur second appel pendant une attente) |
| **F2** | Le flux n'annonce **pas à l'auteur** ses propres messages ; un message **retiré** est signalé (`genre = "retire"`, sans contenu) à tous sauf à celui qui l'a retiré ; un message retiré ne compte plus comme « nouveau » ; un différé retiré avant remise reste muet | `revue_f2_flux_n_annonce_pas_ses_propres_messages_et_signale_un_retrait` |
| **`corr_f1`** (règle 3) | Exige désormais l'erreur **exacte** : `NotFound` pour un non-auteur sur un différé, `MsgPiecePasAuteur` sur un message remis. Ajout d'un test **au niveau du dépôt** (`insert_piece` direct, `created_by` autre que l'auteur d'un message différé) qui exige `InsertionPiece::MessageDiffere`, plus le témoin de l'auteur | `corr_f1_…` (modifié), `revue_f1_depot_direct_par_un_autre_que_l_auteur_d_un_differe_exige_message_differe` |
| **Fuite d'existence** | Un non-auteur qui vise un message **différé d'autrui** reçoit `NotFound` (404), identique à un identifiant inconnu, **avant** tout jugement d'auteur (`message_differe_d_autrui`) | `revue_message_differe_d_autrui_est_un_404_pas_un_refus_d_auteur` (différé et inconnu : même variante ; message remis : `MsgPiecePasAuteur` exact) |
| **Tests manquants** | Métadonnées de pièce invisibles du destinataire avant la remise (liste, JSON, téléchargement identique à une pièce inconnue) ; pièce en quarantaine **non téléchargeable** (conflit) après la remise, **servie** une fois saine ; plafond de 5 sur un message différé (6ᵉ : `SaisieInvalide`) ; annulation d'un différé **avec pièces** : message, pièces et objets détruits ; test HTTP du flux (401, deux établissements, absence de contenu, 429) | `revue_pieces_du_differe_invisibles_avant_remise_quarantaine_apres`, `revue_differe_plafond_de_cinq_pieces_et_annulation_detruit_les_objets`, `tests_flux_m12::flux_http_authentifie_cloisonne_sans_contenu_et_borne` |

**Premier échec de `corr_f1` sur base neuve (diagnostic).** Cause : le test attendait un **conflit** pour « un autre que l'auteur » alors que le domaine (`plan_attacher_piece`) refuse avant le dépôt par `MsgPiecePasAuteur` ; ce n'était ni l'ordre des migrations ni une course. Ce n'était donc pas « à surveiller » : une attente fausse, remplacée par l'erreur exacte (voir ci-dessus). Sur la base neuve `ent_s16_test`, `corr_f1` réussit dès le premier lancement.

### Contrat du flux (modifié)
`GET /messages/flux?apres=<instant>&apres_fil=<uuid>&apres_genre=<nouveau|retire>&attente=<s ≤ 25>` → `{ curseur, curseur_fil, curseur_genre, tronque, resynchro, fils: [{ fil_id, instant, genre }] }` (position = `(instant, fil, genre)` complète). Sans `apres` : curseur de départ seulement. `tronque` : rappeler aussitôt avec `curseur`, `curseur_fil` et `curseur_genre` (jamais refusé, voir « Suites du flux M12 »). Réponse 429 : en-tête `Retry-After`. `resynchro` : recharger les listes, puis repartir de `curseur`. Aucun contenu.

### Non fait / limites connues
1. **Toujours pas de SSE ni de `LISTEN/NOTIFY`** (voir plus haut).
2. *(Remplacé par « Suites du flux M12 » : la borne ne recule plus au-delà de la fenêtre moins 60 s ; la visibilité de `xact_start` est vérifiée au démarrage.)*
3. Le limiteur est **en mémoire du processus** (comme l'anti-abus de connexion) : avec plusieurs instances, la limite est par instance. *(Depuis « Suites du flux M12 » : par session, plus par compte.)*
4. `created_at` rendu par l'insertion d'un message (objet `Message` du service) reste l'horodatage d'application ; seul l'horodatage **stocké** vient de la base. Aucun appelant ne s'en sert pour le flux.
5. Le labo est partagé entre les tests d'un même processus : les tests de flux filtrent sur leurs propres fils (CI : un test à la fois par job).
6. Toujours non faits : brouillons hors ligne, dictée, M7–M11, M13–M17, écran Leptos.

### Vérifications
- Tests ent-db (labo, base neuve, `--test-threads=1`) : filtres `tests_messagerie`, `garde_messagerie`, `isolation_sql_statique`, `jour_utc_interdit` ; tests ent-api : `flux`. Résultats : voir « Résultat final » ci-dessous.
- clippy 1.99.0, les trois commandes de la CI, `--locked`, `-D warnings` : voir « Résultat final ».
- fmt **ciblé** sur les deux fichiers neufs d'`ent-api` (`rustfmt` sur ces fichiers, pas de `cargo fmt --all`) ; fichiers `ent-db` hors périmètre de formatage.
- Aucun test à l'heure réelle ni au jour UTC : les instants viennent de la base ; les seules attentes réelles sont celles qui éprouvent l'horloge elle-même (marge de 5 s du curseur, intervalle d'une seconde du limiteur).

### Compteurs (règle 28, recalculés depuis la base commune `ebac1517`)
Tables avec RLS : **+0** ; `Action::TOUTES` inchangé ; routes **+0** (la route `/messages/flux` existait, deux paramètres de requête et trois champs de réponse en plus) ; `MIGRATION_TAG` inchangé ; migrations : **0** (plage 491–497 toujours libre). Gardes : `message_differe_d_autrui` classée « par identifiant » dans `garde_messagerie_lecture.rs`.

### Résultat final (local)
- ent-db (labo, base neuve, un test à la fois) : filtres `tests_messagerie`, `garde_messagerie`, `isolation_sql_statique`, `jour_utc_interdit` : **128 tests, 0 échec** (127 au premier passage ; le seul échec était une hypothèse fausse de mon test M3 — le fil avait déjà un premier message remis —, corrigée en comparant les **instants**, puis les 9 tests `revue_*` rejoués verts). ent-api : `flux` (3 tests dont le test HTTP) verts.
- clippy 1.99.0 `-D warnings`, les trois commandes de la CI (`--workspace --all-targets` ; avec `ent-api/labo,ent-lab/labo` ; `-p ent-types`) : **0 avertissement**, code retour 0 ; compilation des tests : 0 avertissement. `cargo metadata --locked` dans `ent-rust/web` : réussi.

## Suites du flux M12 (contre-revue du pilote, 2026-11-22)

Niveau : **implémenté et testé en local, non validé par Florian, non prêt pilote.** Branche `ent-socle-16-code-m12-suites`, partie d'`origin/ENT` (bc69cd0d). Base PostgreSQL 16 locale neuve `ent_s16m12_test` (nom « test », rôle superutilisateur local, mot de passe de test), `ENT_EXIGER_BASE=1`, `RUST_MIN_STACK=16777216`. **Aucune migration** (plage 491–497 toujours libre) ; aucune donnée ni secret de production.

Règles tenues : (1) le curseur ne saute jamais un événement et progresse toujours ; (2) le temps réel marche sur plusieurs appareils ; (3) un non-auteur ne distingue jamais un différé d'autrui d'un identifiant inconnu ; (4) une seule horloge, la base.

### Fait
| Point | Correctif | Test |
|---|---|---|
| **M-A** | `msg_repo::borne_sure` (fonction pure) : `min(maintenant − 5 s, début de la plus ancienne transaction visible)`, **jamais plus vieille que la fenêtre − 60 s (540 s)**. Une transaction de plusieurs heures (autovacuum, sauvegarde, « idle in transaction ») ne fige plus le curseur : il reste dans la fenêtre, donc plus de `resynchro` en boucle. Limite assumée : un message validé par une transaction de plus de 540 s peut être raté (la fenêtre de 600 s l'aurait de toute façon rejeté) | `borne_sure_progresse_toujours_…` (instants injectés : 2 s, 30 s, 601 s, 1 h, 24 h) ; `revue_ma_curseur_progresse_avec_une_transaction_ouverte_…` (transaction ouverte : pas de `resynchro`, le curseur avance) |
| **M-B** | Liste tronquée : le curseur rendu est la position du dernier événement **si elle est ≤ la borne sûre** ; sinon on ne rend que le stable (≤ borne) et le curseur est la borne (le reste est relu à l'appel suivant, la borne progresse). La troncature est donc bornée par la limite sûre | `revue_mb_troncature_borne_le_curseur_par_la_limite_sure` |
| **M-C** | `annuler_message_differe_for_actor`, `message_de_l_auteur` (corriger, retirer) : `message_differe_d_autrui` **avant** le jugement d'auteur → `NotFound` identique à un identifiant inconnu ; message remis d'autrui : `Forbidden` exact | `revue_mc_differe_d_autrui_est_un_404_sur_annuler_corriger_et_retirer` |
| **M-D** | Limiteur **par session** (`session_id`) : un appel à la fois et un début par seconde par session ; plafond de 8 appels simultanés par compte ; `Retry-After` sur tout 429 ; une réponse `tronque` libère le rappel immédiat de la session (c'est le serveur qui s'en souvient, pas un paramètre contournable) : la pagination n'est jamais refusée | `flux_limite::tests` (horloge injectée : deux sessions d'un compte, plafond, pagination tronquée ×50 sans refus, `Retry-After`) ; HTTP : deux sessions d'un même compte reçoivent toutes deux l'événement, seconde session servie pendant qu'un appel de la première attend |
| **M-E** | `PgEnt::verifier_flux_messagerie` appelé au démarrage (`boot_from_env`) : si le rôle de l'application n'a ni `pg_read_all_stats` ni le droit superutilisateur, **erreur journalisée** (« GRANT pg_read_all_stats TO <rôle> ») ; la borne retombe alors sur la marge de 5 s. La dépendance reste, mais elle est vérifiée et le curseur progresse sans elle (M-A) | `revue_m3_message_valide_tard_n_est_pas_perdu` affirme que la transaction ouverte est visible et que la vérification réussit |
| **F-1** | Le test HTTP de concurrence passe `apres=<instant>` (futur : l'appel attend vraiment) : refus `Concurrent`, pas `TropVite` | `tests_flux_m12` |
| **F-2** | Plus de `sleep` comme preuve : le test M3 (attente de 6 s) devient une vérification de visibilité de la transaction ; M2/MB reculent explicitement les instants ; le test HTTP attend une **barrière** (place du compte prise), plus 500 ms ni 1,2 s ; limiteur à horloge injectée | idem |
| **F-3** | Le participant sans droit est **construit** (professeur d'une autre classe que celle de l'élève), la précondition « ne peut pas lire » est affirmée sans condition | `revue_m1_…` |
| **F-4** | Position complète `(instant, fil, genre)` : `curseur_genre` rendu, `apres_genre` accepté (compatible : absent = pas de genre) | `revue_f4_curseur_a_trois_termes_aucun_saut_entre_genres` |
| **F-5** | Règle 4 : les délais de correction (15 min) et de retrait (10 min) se mesurent sur `now()` de la base (`maintenant_base`), comme `created_at`. `created_at` d'un message = `clock_timestamp()` (distinct par message, même dans une transaction) et la lecture trie `created_at, id` : ordre stable | `revue_f5_delais_sur_l_horloge_de_la_base_et_ordre_stable` |
| **F-6** | Le flux juge **tous** les candidats en un seul passage : `get_fils_lot` (1 requête) + `actor_can_access_fils_lot` (droits de l'acteur calculés une fois ; boîtes, élèves protégés, périmètre et participation groupés) au lieu de `get_fil` + `actor_can_access_fil` par fil | `revue_f6_jugement_groupe_equivaut_a_la_boucle_et_va_plus_vite` : équivalence avec le jugement individuel pour 7 acteurs (famille ×2, direction, deux professeurs, CPE, administratif) et temps mesuré pour 200 candidats |
| **F-7** | Documentation à jour (contrat du flux, limites 2 et 3 ci-dessus) | — |

### Découvert, non corrigé (hors périmètre)
Corriger son message **différé** puis l'**annuler** échoue (`Domain(ReferenceInconnue)`) : la version conservée (`ent_msg_version`, ajout seul) empêche la suppression du message. À traiter avec M4 (annulation : conserver ou purger les versions d'un message jamais remis, décision de conception).

### Non fait / limites
1. Toujours pas de SSE ni de `LISTEN/NOTIFY`. 2. Le limiteur reste en mémoire du processus (par instance). 3. Pas de test HTTP d'une pagination tronquée (il faudrait plus de 200 événements) : couverte au niveau du limiteur et du dépôt. 4. Pas de test HTTP du rappel immédiat refusé (dépendrait de l'horloge réelle) : couvert avec horloge injectée. 5. Un message validé par une transaction de plus de 540 s peut être raté (voir M-A). 6. Brouillons hors ligne, dictée, M7–M11, M13–M17, écran Leptos : non faits.

### Compteurs (règle 28, depuis `origin/ENT` bc69cd0d)
Tables avec RLS : **+0** ; `Action::TOUTES` inchangé ; routes **+0** ; `MIGRATION_TAG` inchangé ; migrations **0**. Garde de lecture : `get_fils_lot` classée « par identifiant ».

### Résultats
- **ent-db** (labo, base neuve, un test à la fois) : filtres `tests_messagerie`, `garde_messagerie`, `isolation_sql_statique`, `jour_utc_interdit`, `borne_sure` : **135 tests, 0 échec** (134 au premier passage complet ; le seul échec était une entrée périmée de la garde de lecture, corrigée, `garde_messagerie` rejoué : 6 verts). Avant cela, deux passes ont fait échouer mes propres tests neufs (instants récents : l'événement d'un fil est celui de son dernier message, donc tous les messages du fil doivent être reculés ; borne de comparaison calculée avant l'appel) : défauts de test, pas du code.
- **ent-api** : filtre `flux` : **7 tests verts** (limiteur ×5 dont horloge injectée, HTTP flux 1, dont deux sessions d'un compte).
- **clippy 1.99.0 `-D warnings`**, les trois commandes de la CI (`--workspace --all-targets` ; avec `ent-api/labo,ent-lab/labo` ; `-p ent-types`) : **0 avertissement**, code retour 0 chacune. Un premier passage avait relevé deux erreurs (méthode de test non utilisée hors labo ; `Err` de 128 octets dans le gestionnaire) : corrigées. `cargo metadata --locked` dans `ent-rust/web` : réussi. fmt **ciblé** (`rustfmt` sur `flux_limite.rs` et `tests_flux_m12.rs`, fichiers neufs d'`ent-api`) ; pas de `cargo fmt --all`.
- Temps de F-6 (200 candidats) : voir la sortie `--nocapture` du test ; l'assertion exige groupé < boucle et < 2 s.
- Tête et CI : voir ci-dessous (complétés après la CI).

