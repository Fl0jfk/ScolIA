# Socle 20 — lots C8 (visa, inspection, archive) et C9 (progressions et reconduction) : livraison

**Date :** 2026-11-21 — **Branche :** `ent-socle-20-code-c8c9`, partie de `origin/ENT` d23a6f66 — **Statut :** implémenté et testé en local ; CI et relecture du pilote à venir (voir « Tête et CI »). Conception : `20_CAHIER_DE_TEXTES.md` (sur `ent-mc0hoe`) §14, §16, §19, §20.11, §21 (R-C17 à R-C19, R-C28), §23 (écrans 3, 7, 8), §26 (lots C8 et C9).

Légende : **implémenté et testé** (test automatisé local vert) · **implémenté, non testé automatiquement** · **non fait** (avec la raison).

## 1. Modèle de menace appliqué

| Règle | Où elle est tenue |
|---|---|
| 1. Ce qui est visé est figé | `ent_cdt_visa_element` (objet, version) en ajout seul ; visa sans UPDATE/DELETE/TRUNCATE (déclencheur) ; « modifié après visa » = version actuelle > version visée, avec texte vu et texte actuel, auteur, date, motif ; marque visible de la direction **et** de l'auteur. |
| 2. L'inspection voit ce qu'on lui ouvre | accès borné par CHECK (90 jours), cours listés, période du cahier ; lecture **journalisée avant d'être servie** (sans trace, rien n'est servi) ; entrées et consignes publiées seulement : aucune donnée structurée d'élève (ni remise, retour, variante), ni brouillon ; journal lu de la direction et du professeur concerné. |
| 3. Archives et exports figés, ajout seul | exports : empreinte **recalculée par la base** (CHECK), UPDATE/DELETE/TRUNCATE refusés ; archives immuables (seule la décision change, une fois) ; journal de clôture en ajout seul ; réouverture motivée (20 caractères, double validation) et tracée. |
| 4. Un professeur ne lit et ne copie que ce qui est à lui ou partagé | progression privée (l'existence d'une progression d'autrui n'est même pas révélée : « introuvable » pour la direction comme pour un collègue) ; partage explicite vers un cours **où l'auteur intervient** ou une discipline **qu'il enseigne** ; la copie de cours à cours ne reprend que **ses** entrées et exige d'avoir enseigné le cours source ; ni remise, ni coche, ni travail, ni contribution, ni nom d'élève. |
| 5. Cloisonnement établissement ; ordre personnel, droit, existence | `TenantTx` partout, `etablissement_id` de la session, clés composites, RLS posée ; les routes testent le droit du rôle avant l'existence ; un identifiant d'un autre établissement est « introuvable ». |

## 2. Ce qui est livré

### C8 — visa, inspection, archive (migration 2407 ; `cahier_c8.rs`, `cahier_c8_acces.rs` ; routes `/cahier/c8/*`)

| Id conception | Action | État |
|---|---|---|
| CDT-210 | Tableau de tenue (direction) : séances passées / renseignées, taux, travaux, dernière entrée, dernier visa, modifiés après visa, « à viser » (seuil 6 semaines), intervenants (noms d'affichage) | implémenté et testé |
| CDT-10, 211 | Viser un cours, une classe ou un professeur ; référence figée ; cahier vide : confirmation explicite | implémenté et testé |
| CDT-212, 213 | Observation (cours ou professeur, jamais une classe) ; réponse du professeur ; l'autre professeur et la vie scolaire ne lisent pas | implémenté et testé |
| CDT-214 | Détail d'un visa, modifications après visa, chronologie du cours (écran 3) | implémenté et testé |
| CDT-215 | Accorder, prolonger (jamais au-delà de 90 jours cumulés), révoquer ; bénéficiaire ni élève ni responsable ; professeur visé ou cours ; historique | implémenté et testé |
| CDT-216 | Lecture de l'inspecteur : accès expiré, révoqué, autre compte, cours non ouvert → 403 | implémenté et testé |
| CDT-217 | Export figé (litige : motif obligatoire ; inspection ; autre) ; empreinte stable (même cahier, même empreinte) | implémenté et testé |
| CDT-222 | Contrôle de clôture, clôture (cahier figé, une archive par cours portant du contenu, version +1 à chaque clôture), réouverture motivée | implémenté et testé |
| CDT-223 | Export du professeur (ses entrées et travaux, brouillons signalés, sans élève ; lu de lui seul, pas de la direction) | implémenté et testé |
| CDT-245 | Décision du chef d'établissement sur une archive à échéance (versée / à détruire), une fois | implémenté et testé (échéance simulée par insertion) |

Écrans Leptos (rendu serveur + hydratation) : **7** `/web/cahier/visa` (tenue et visa) ; **3** `/web/cahier/chronologie?cours=` (cahier du cours en lecture, visas, observations, réponse) ; **8** `/web/cahier/inspection` (direction : accorder, prolonger, révoquer ; professeur : journal) et `/web/cahier/inspection/lecture` (inspecteur) ; `/web/cahier/archives` (clôture, archives, exports). Entrées de menu selon le moteur de droits (`cahier.viser`, `cahier.acces_inspection`, `cahier.exporter`).

### C9 — progressions et reconduction (migration 2408 ; `cahier_c9.rs` ; routes `/cahier/c9/*`)

| Action | État |
|---|---|
| Créer, lire, modifier (étapes conservées par identifiant, une étape planifiée ne se supprime pas), archiver | implémenté et testé |
| Partager avec un cours (ses intervenants, suppléant compris) ou une discipline ; retirer le partage ; l'auteur seul voit les partages | implémenté et testé |
| Planifier sur les séances libres d'un cours : brouillons, jamais publiés, aperçu, idempotent, séances passées et annulées exclues, étape par étape, durée en séances, cahier figé → refus | implémenté et testé |
| Reconduire une progression (copie privée sans dates ni partage, liée à l'originale) | implémenté et testé |
| Copier les entrées de **ses** cours d'un cours à l'autre (brouillons, notions suivent, aperçu, entrées d'autrui et retirées non copiées, rien d'élève) | implémenté et testé |

Écran Leptos `/web/cahier/progressions` : liste (mes / partagées avec moi), création (une étape par ligne), planification avec aperçu, partage, reconduction, copie de cours à cours.

## 3. Cas limites dérivés

| Cas | Comportement implémenté |
|---|---|
| Visa sur un cahier vide | refusé sans confirmation ; avec confirmation, visa « vide » tracé (0 élément) |
| Professeur modifie après le visa | version suivante, « modifié après visa » pour la direction et pour lui ; texte vu conservé |
| Professeur parti | il n'est plus intervenant (délai de grâce de lecture de 7 jours du socle 6) : plus de visa ni d'observation lisibles, plus de réponse ; sa progression reste privée et sa copie de cours reste possible (il a enseigné) mais exige d'écrire dans le cours cible |
| Remplaçant d'une séance | n'est pas intervenant du cours : ni visa, ni observation, ni planification, ni copie |
| Co-enseignement / suppléant | tous les intervenants (titulaire, co-enseignant, suppléant, assistant, intervenant) lisent le visa et l'observation du cours et peuvent répondre ; seuls ceux qui ont la saisie planifient et copient |
| Cours supprimé | un cours n'est jamais supprimé (clés RESTRICT) : visas, archives, exports et accès restent lisibles |
| Changement d'année | clôture par année, une archive par cours, version 2 après réouverture ; reconduction de progression sans dates ; copie de cours à cours entre années |
| Accès d'inspection expiré / révoqué / autre compte / cours hors accès | 403 sans contenu ni journal de contenu |
| Inspecteur sans accès | liste vide ; lecture 403 |
| Année close au socle 5 | le cahier reste figé : réouverture refusée tant que l'année n'est pas rouverte |
| Clôture avec points du contrôle | refusée sans confirmation (CC-60) ; non-rendus signalés en nombre ; l'élève garde ses dépôts |
| Deux clôtures simultanées | verrou de l'année ; la seconde voit « déjà figé » |
| Export demandé pour litige | motif obligatoire ; aucune donnée structurée d'élève ; un nom d'élève écrit par un professeur dans un texte libre y figurerait (la saisie l'en avertit) |
| Décision d'archive avant l'échéance ou déjà prise | refusée |

## 4. Non fait, avec la raison

1. **Avis au professeur (socle 8) à l'octroi d'un accès d'inspection, à une observation, à une réponse** : les événements sont publiés dans l'outbox (`cahier.acces_inspection_accorde`, `cahier.observation`…) mais aucun `AvisSurEvenement` n'est déclaré (le catalogue d'avis est à fusionner avec le lot N11 du socle 8) ; le professeur voit les accès qui le concernent à l'écran 8.
2. **PDF/A du socle 7** : l'archive et l'export sont un **texte figé** dont l'empreinte SHA-256 est contrôlée par la base ; `ent_cdt_archive.objet_id` est devenu facultatif (l'`export_id` porte le document). Le dépôt au coffre « archives pédagogiques » reste à faire avec le socle 7.
3. **Pièces jointes** : ni les exports, ni la lecture d'inspection, ni la copie de cours à cours ne portent les documents des séances (« documents compris » de la conception) : copie d'objets du socle 7 non faite dans ce lot.
4. **Destruction effective** après la décision « à détruire » : la décision est tracée et définitive, mais les exports sont en ajout seul ; la purge relève du socle 27 (conservation).
5. **Compte invité de l'inspecteur** (socle 1) et compte académique fédéré (socle 9) : la direction désigne un compte existant de l'établissement par son identifiant (champ texte, pas de sélecteur) ; l'inspecteur n'a pas d'entrée de menu, il arrive par `/web/cahier/inspection/lecture`.
6. **Annuler une planification** : une entrée ne se supprime jamais (R-C3) ; une planification se complète (étapes manquantes, séances ajoutées) mais ne s'annule pas en lot ; une entrée planifiée de trop se retire entrée par entrée (C2).
7. **Visa de l'ancienne route** `POST /cahier/visas` (C0) : conservée telle quelle (visa de classe sans référence figée, « visa antérieur au cahier versionné ») ; la nouvelle route est `/cahier/c8/visas`. Retrait à prévoir avec C13.
8. **Visa en lot « tous les cours d'une classe »** : implémenté (cible classe, éléments figés de tous ses cours) mais sans observation ; pas d'écran dédié (l'écran 7 vise par cours).
9. **Coordinateur de discipline** : pas de rôle dédié ; le partage « discipline » ouvre la progression à tout collègue qui enseigne la matière (la matière du cours fait foi).
10. **Lecture du cahier de l'année close par élèves et familles pendant l'année suivante** (R-C28) : non touchée (C3).
11. Exercices (C10), mobile (C11), IA (C12), bascule des clients (C13), RLS stricte (C14) : hors lot.

## 5. Tests

**Backend (`ent-db`, base locale de test, un établissement neuf par test)** : `tests_cahier_c8.rs` (12) et `tests_cahier_c9.rs` (5), tous verts ; plus les tests `tests_cahier` existants, `registre_donnees`, `tests_rls`, `rgpd`, `conservation`, `migrate` : **173 verts** au total sur ce sous-ensemble ; gardes d'isolation SQL statique, `jour_utc_interdit` et gardes d'écriture : verts (la garde d'isolation a attrapé une requête sans filtre d'établissement dans le tableau de tenue, corrigée). Couverture de la matrice de séparation du §27 pour ce lot : direction, professeur en saisie, autre professeur, ancien/hors cours, CPE, comptabilité, élève, parent, inspecteur (pendant, expiré, révoqué, hors cours), autre établissement.
**Actions** (`ent-auth`) : 64 verts (positions de fin de catalogue mises à jour).
**HTTP de séparation** (`ent-api/tests/separation.rs`, 56 verts) : +25 spécifications d'établissement B (C8, C9) et +8 routes sans identifiant bornées à l'établissement de la session ; l'inventaire de routes (401 sans session) couvre les 34 routes nouvelles automatiquement.
**Frontend** : 14 tests natifs de logique d'écran (`modele_c8`, `modele_c9`) et tests de navigation (37 verts avec la suite filtrée), rendu serveur des 6 pages (CSP, langue, aucun style en ligne) et test de refus par profil contre le vrai backend (`ent-web-server/tests/cahier.rs`, 4 verts) ; parité des droits (`droits_parite`, 21 verts).
Aucun test ne dépend de l'heure réelle ni du jour UTC (jours de l'établissement, séances à jours explicites, accès expiré inséré avec des intervalles relatifs à `now()` de la base).

## 6. Compteurs (règle 28, recalculés depuis `d23a6f66`)

`Action::TOUTES` 275 → **277** · compte RLS de `tests_rls.rs` **+8** (`… + 5 + 3`) · registre des données **+8** (domaine `cahier_textes`, 2 exceptions `SansHorodatage`) · `TABLES_ELEVE` **inchangé** · `SOURCES` / `NOMS_SOURCES` 69 → **71** · routes **+34** (24 + 10) · `MIGRATION_TAG` **inchangé** (3808) · migrations **2407**, **2408** (2419 reste réservée à C14).
Dépendances : **aucune** nouvelle. Colonnes JSON / tableau / BYTEA : **aucune** (`grep -i jsonb` vide sur 2407 et 2408).

## 7. Qualité

Zéro avertissement rustc et clippy 1.99 sur le code du lot (`cargo clippy --locked --workspace --all-targets --features ent-api/labo,ent-lab/labo -- -D warnings` ; frontend `-p ent-web-ui -p ent-web-api -p ent-web-app --all-targets --features ent-web-app/ssr -- -D warnings`) ; formatage ciblé (`cargo fmt -p` des crates du frontend et d'`ent-types`, `rustfmt` des seuls fichiers neufs du backend, jamais `--all`) ; `cargo metadata --locked` dans `ent-rust/web` vérifié.

## 8. Tête et CI

Pull request en brouillon vers `ENT` : n° 110. Premier passage « ENT Rust » sur la tête `20822e0a` : **rouge, hors périmètre du lot**. 8 tests échouent, aucun du cahier : `tests_assiduite_as6::as6_relance_automatique_unique_a_debut_plus_delai`, `…::as6_aucune_relance_pour_une_seance_annulee_validee_ou_reprise`, `tests_evaluations_v0::les_lectures_historiques_utilisent_le_moteur_exact`, `…::une_absence_n_est_jamais_un_zero_et_le_rattrapage_reste_possible`, `tests_evaluations::e2e_v3b_une_note_saisie_puis_publiee_est_lue_partout_avec_la_meme_moyenne`, `…::periode_complete_de_la_grille_au_bulletin_et_a_l_export`, `…::coefficients_de_matiere_et_regles_de_calcul`, `ent-web-server::assiduite_as14de::as14e_seuil_proposition_decision_et_rectification`. Les jobs « Aucun test ne disparaît » et le verdict en découlent. Le run a eu lieu un **samedi à 08 h UTC** ; ENT d23a6f66 était vert le vendredi 20-11 à 15 h ; les mêmes tests échouent **en local sur cette branche** le samedi (même panique), donc dépendance au jour ou à l'heure (règle 33), pas au lot (ni migration, ni table, ni route de ces domaines modifiés). Non corrigés ici (autres socles, hors périmètre) ; à confirmer par un passage d'ENT un jour ouvré ou par un lot de correction des horloges. Les tests du lot (17 de base, 56 de séparation, frontend) passent dans le même run.

## 9. Correctifs de revue (2026-11-22)

Branche `ent-socle-20-code-c8c9` (PR n° 110), partie de sa tête distante `632c2555`, puis fusion d'`origin/ENT` 02659006 (commit `d581ac17`) ; compteurs recalculés (règle 28) dans `docs/socles/MIGRATIONS.md`. Les migrations **2407 et 2408 (non versées) sont modifiées** ; aucune autre migration ne change. Légende : **fait et testé** · **fait, non testé automatiquement** · **non fait / documenté**.

| Id | Correctif | État |
|---|---|---|
| **E1** | Déclencheurs `BEFORE INSERT OR UPDATE` (`ent_cdt_ecriture_gardee`) sur entrées, travaux, contributions, notions et leurs versions : 23514 si le cahier de l'année est figé ; seule la réouverture le désarme. Messages 409 clairs ajoutés dans `cdt_entree_retirer`, `cdt_contribuer`, `appliquer_report` (C6) ; planification et copie C9 passent par le contrôle dans leur transaction. Les reports automatiques d'une année figée échouent dans leur point de sauvegarde sans casser la lecture. | fait et testé (`c8_annee_figee_en_base_…` : retirer, contribuer, report C6, planification C9, saisie, + 6 écritures SQL directes → 23514, puis réouverture) |
| **M5** | Verrou partagé sur la ligne de l'année dans le déclencheur ; clôture et réouverture en `FOR NO KEY UPDATE` (passe 2) ; `cree_le` du journal de clôture en `clock_timestamp()` (l'ordre du journal suit les validations). | fait et testé (`c8_cloture_et_ecritures_se_serialisent_par_verrou_…`, refait en passe 2 : voir plus bas) |
| **M1** | `cahier.cloturer` (clôture, réouverture, décision d'archive) séparé de `cahier.exporter` ; **direction seule** (Admin ne l'a pas : technique, sans autorité de clôture) ; réouverture à **double validation** (`POST /cloture/reouvrir` = demande, motif ≥ 20 caractères ; `POST /cloture/reouvrir/valider` par une autre personne de la direction ; CHECK en base) ; l'octroyeur ne s'accorde pas un accès (code + CHECK) et le bénéficiaire ne prolonge pas le sien. Écran : demande, attente, validation. | fait et testé (Admin 403 ; demandeur 403 sur sa propre demande ; seconde demande 409 ; cahier figé tant que non validée ; accès à soi-même 403 + 23514) |
| **M2** | `GET /mon-export` rend l'aperçu **sans écriture** (`ExportApercuDto`) ; `POST /mon-export` conserve, **un seul export par (auteur, année, empreinte)** (index unique partiel) ; plafond de 4 000 000 caractères **avant** l'insertion (422, jamais 500) pour tous les exports. | fait et testé (GET ×3 = 0 ligne ; POST ×2 = 1 ligne ; cahier modifié = nouvel export ; autre auteur = son export ; plafond = 422) |
| **M3** | Message d'octroi corrigé (aucun avis n'existe) ; avis du professeur inscrit comme **prérequis de l'usage courant de « rendez-vous de carrière »** dans la liste des suites du socle 8 (`08_NOTIFICATIONS_LIVRAISON.md` §« suites » n° 11) ; l'écran d'octroi le dit. | fait (texte) |
| **M4** | « aucune donnée structurée d'élève » partout (code, types, écrans, ce rapport, en-tête des exports) ; avertissement à la saisie (contenu de séance, consigne) : un nom d'élève dans le texte libre est lu des inspecteurs et figure dans exports et archives. | fait (texte ; l'en-tête d'export change le contenu figé des nouveaux exports) |
| **M6** | `c8_intervenant`, `c8_agent_du_compte`, `c8_cours_ref`, `c9_intervient` (+ jour de l'établissement, `c9_lisible`, contrôle d'année ouverte de C9) prennent `&mut tx` : plus de transaction imbriquée sur le pool dans le lot. | fait (revue du code, suites C8/C9 vertes) |
| **M7** | « Ajouté depuis le visa » : liste `ajoutes_depuis_visa` du détail d'un visa, drapeau `ajoute_depuis_visa` de la chronologie, compte `ajoutes_apres_visa` du tableau de tenue (le dernier visa du cours qui couvre la date fait foi) ; badge et phrase à l'écran. | fait et testé (`c8_ajoute_depuis_le_visa_est_visible_du_viseur`) |
| F1 | TRUNCATE interdit sur `ent_cdt_acces_inspection`. | fait (migration ; suites vertes) |
| F2 | Gardes : `revoque_par_user_id`, `decide_par_user_id`, `decide_le` figés ; archive : empreinte, cours, année = ceux de l'export (déclencheur d'insertion). | fait (migration ; suites vertes) |
| F3 | Ordre personnel → droit → existence : réponse (403) identique pour inconnu et non autorisé (observation, visa, export, copie, planification) ; « source = cible » vérifié après le droit (le test de séparation l'a attrapé). | fait et testé |
| F4 | Comptages d'un visa de classe limités aux cours visibles du lecteur. | fait (non testé séparément : la fixture n'a qu'un cours par classe) |
| F5 | Liste des visas : plafond de 200 appliqué **après** le filtre de visibilité (examen borné à 5 000). | fait (revue du code) |
| F6 | Rôle et appartenance revérifiés à chaque lecture d'inspection. | fait et testé |
| F7 | Copie limitée au même niveau (niveau de la classe du cours). | fait (non testé séparément : pas de fixture à deux niveaux) |
| F8 | Progression archivée non planifiable (409). | fait et testé |
| F9 | Clés étrangères composites sur `matiere_id`, `niveau_id` (progression, partage). | fait et testé (matière inconnue refusée) |
| F10 | `.export()` sur accès d'inspection, historique, consultations, exports, partages. | fait |
| F11 | L'écran d'octroi liste les cours par `GET /inspections/cours` (droit `cahier.acces_inspection`), plus par le tableau de tenue (`cahier.viser`). | fait (non testé séparément) |
| F12, F13 | **Non traités : la consigne ne les décrit pas** (ni la PR, ni les documents lus ne contiennent leur énoncé). À préciser par le pilote. | non fait |

**Écarts déclarés.** (1) Le déclencheur fige sur le **journal de clôture du cahier** (`ent_cdt_cahier_fige`), pas sur `ent_annee_scolaire.cloturee_le` (socle 5) : la reprise et les corrections d'années anciennes restent possibles ; le contrôle de confort du code, lui, refuse aussi l'année close au socle 5. (2) Une publication programmée qui échoit dans un cahier figé reste programmée (la projection vers les anciennes tables l'ignore : sans ce filtre, chaque lecture aurait échoué sur la garde) ; elle se publie après une réouverture. Les reports C6 automatiques d'une année figée restent « proposés ». (3) Aucun avis (socle 8) n'est encore émis : voir M3.

**Tests (PostgreSQL 16 local, base neuve `ent_c8c9_test`, rôle `ent_ci`, `ENT_ENV=test`).** `ent-db` (labo, `--test-threads=4`) : `tests_cahier` complet (C0 à C9, dont **8 tests nouveaux ou refaits** pour C8, 1 pour C9) + `registre_donnees`, `tests_rls`, `rgpd`, `conservation`, `migrate` : **169 réussis, 0 échec** ; `isolation_sql_statique` 2 ; `ent-auth` 64 ; HTTP `separation` **57 réussis** (T1 : les 28 cas C8/C9 exigent maintenant un refus exact 401/403/404, plus « échec quelconque »). Frontend : `ent-web-app` (cahier) 57, clippy `-D warnings`, `cargo metadata --locked` dans `ent-rust/web` : verts ; le test d'intégration `cahier` d'`ent-web-server` **n'a pas pu être lié en local** (binaire de debug trop gros : erreurs `lld` « relocation out of range » dans `ring`), il est donc couvert par la seule CI ; `ent-db` `tests_cahier` (113) rejoué après le dernier changement de code. Aucun test ne dépend de l'heure ni du jour ; les tests de course n'asserent que l'ordre des verrous.
**Qualité.** clippy **1.99.0** `-D warnings` : backend (`--workspace --all-targets --features ent-api/labo,ent-lab/labo`) et frontend (`ent-web-ui`, `-api`, `-app`, `ssr`) : **0 avertissement** ; formatage ciblé (`rustfmt` des fichiers C8/C9 du backend, `cargo fmt -p` frontend et `ent-types`), jamais `--all`.
**Compteurs (règle 28, base `origin/ENT` 02659006).** `Action::TOUTES` 275 → **278** · routes C8 24 → **27** (+2 = `POST /mon-export`, `POST /cloture/reouvrir/valider`, +1 = `GET /inspections/cours`) · RLS, registre des données, `TABLES_ELEVE`, sources de routes, `MIGRATION_TAG` : inchangés.

**Tête et CI (correctifs).** Premier passage (`2944a797`, run 45002610511) : tout vert sauf `ent-web-server::cahier::visa_inspection_archive_et_progressions_refuses_aux_profils_sans_droit` (le test attendait « introuvable » pour un visa inconnu lu par un professeur ; F3 le rend 403, identique à un visa non lisible) ; test corrigé (professeur : 403, direction : 404), non rejoué en local (liaison impossible du binaire de debug). Tête finale et verdict : voir le message au pilote.

## 10. Correctifs de revue, passe 2 (2026-11-22, règles générales D-069)

Branche `ent-socle-20-code-c8c9` (PR n° 110, brouillon), partie de sa tête distante `25ee9968`, puis fusion d'`origin/ENT` `ebac1517` (commit `42c1d5f0`, aucun conflit) ; compteurs recalculés (règle 28) : **inchangés** (`Action::TOUTES` 278, RLS, registre : mêmes tables, `TABLES_ELEVE`, sources, `MIGRATION_TAG` 3808 ; aucune migration nouvelle : **2407 modifiée**, 2408 inchangée). Alerte RUSTSEC-2026-0592 : `origin/ENT` portait encore `url` 2.5.8 ; correctif exact de la PR 121 (`cargo +1.99.0 update -p url --precise 2.5.9` dans `ent-rust` et `ent-rust/web`, `url` 2.5.9 et `form_urlencoded` 1.2.3, rien d'autre dans les `Cargo.lock`), commit `38f7ef62`.

### Règles générales

| Règle | Réalisation | État |
|---|---|---|
| **1. Année close figée en base** | Une seule fonction de garde, `ent_cdt_annee_gelee(etab, annee)` = année close au socle 5 (`cloturee_le`) **ou** cahier clôturé au journal ; `ent_cdt_cahier_fige` ne dit plus que l'état du journal (commentaire corrigé). Le déclencheur `ent_cdt_ecriture_gardee` couvre **INSERT, UPDATE et DELETE** de 14 tables : entrées, versions d'entrée, notions, travaux, versions de travail, cibles, formats, variantes, variantes d'élève, contributions, versions de contribution, liens, pièces jointes, reports. L'année se résout par le parent (entrée, travail, variante, contribution, porteur) ; un UPDATE qui change de parent vérifie les deux années. Seule une réouverture tracée du cahier désarme le gel du cahier (elle ne rouvre pas une année close au socle 5). **Hors gel, par choix** : activité des élèves (coches, dépôts, retours, remises : on doit pouvoir corriger et noter après la clôture) et progressions personnelles (C9, sans année ; leur planification écrit des entrées, donc gardée). | fait et testé |
| **2. Journal de clôture cohérent par construction** | Déclencheur `BEFORE INSERT` sur `ent_cdt_cloture` (verrou de l'année, `cree_le` forcé à `clock_timestamp()` : plus de rétrodatage) : pas de clôture sur un cahier déjà clos ; pas de demande sur un cahier ouvert ; une `reouverture` exige une `reouverture_demandee` **d'une autre personne**, postérieure à la dernière clôture/réouverture, **non expirée** et non consommée (fonction `ent_cdt_demande_reouverture`), et reprend son motif. Délai : colonne `ent_cdt_reglage.reouverture_delai_jours` (défaut **14**, 1 à 60), ajoutée par 2407. Le code (`demande_en_attente`) lit la même fonction. **Non fait** : écran de réglage du délai (la colonne se modifie par le service de réglages à venir ; défaut prudent d'ici là). | fait et testé (colonne : valeur par défaut seulement) |
| **3. Un pouvoir = une action** | Admin n'a plus ni `cahier.cloturer`, ni `cahier.exporter`, ni `cahier.acces_inspection` : les trois sont à la **direction seule** (`modele_par_defaut`). Limites qui restent, **documentées** : (a) deux comptes d'une même personne (un compte direction et un compte tiers) ne sont pas détectables par la base ; (b) l'octroi croisé entre deux directeurs. Les deux sont **journalisés** (historique d'accès, journal de clôture) ; l'octroi croisé est **signalé** (`croise` sur chaque accès, mention à l'écran d'octroi de la direction). | fait et testé (Admin : clôture, demande de réouverture, export, octroi, liste d'archives refusés ; `croise` : non testé séparément) |
| **4. Ordre des contrôles ; pas de transaction imbriquée** | Personnel, droit, existence partout dans le lot. Recherche des transactions imbriquées (programme de balayage : toute fonction du lot qui prend `&mut tx` ou qui appelle une méthode ouvrant `TenantTx`/le pool après `TenantTx::begin`) : seuls `cdt_c9_planifier` et `cdt_c9_copier_cours` appelaient `cdt_exiger_saisie` dans leur transaction ; le contrôle passe **avant** `TenantTx::begin` (droit avant existence, ce que F3 demande aussi). Aucun autre cas dans `cahier_c8`, `cahier_c8_acces`, `cahier_c9` ; les appels de l'ancien code (C2 à C7) sont hors lot et inchangés. | fait (revue du code ; suites vertes) |

### Correctifs

| Id | Correctif | État |
|---|---|---|
| **M6 inachevé** | Voir règle 4. | fait |
| **E1-socle 5** | Règle 1. La publication programmée (`cdt_projeter_echus`, `cdt_projeter_travaux_echus`) et la synchronisation des reports C6 ignorent une année close au socle 5 (filtre `ent_cdt_annee_gelee`) : plus d'erreur de garde, la publication reste programmée. **Avant une clôture** : `c8_projeter_echus_avant_cloture` boucle sur `cdt_projeter_echus` jusqu'à épuisement (borné à 50 tours, arrêt si la projection ne progresse plus), puis la transaction de clôture refuse (409, « réessayez ») s'il reste une publication due **non publiée** (course d'un instant) : aucune publication due n'est perdue ni absente de l'archive. | fait et testé (`c8_cloture_projette_les_publications_echues…`, `c8_gel_…socle_5…`) |
| **E1-trous** | Garde INSERT/UPDATE/DELETE sur `ent_cdt_entree_notion`, `_travail_cible`, `_travail_format`, `_variante`, `_variante_eleve`, `_lien`, `_piece_jointe`, `_report` et les versions. | fait et testé : **un test SQL direct par table et par verbe** (14 tables × 3 verbes), avant (le jeu est valide), pendant (23514, comptage inchangé) et après réouverture ; deux contextes (cahier clôturé, année close au socle 5) |
| **M1** | Règles 2 et 3. | fait et testé (`reouverture` en SQL direct sans demande : refus ; demande expirée — rétrodatée sous `session_replication_role = replica` pour ce seul test — : refus base et code, puis nouvelle demande possible ; auto-validation, motif différent, double clôture, demande sans clôture : 23514 ; Admin sans les trois droits) |
| **Faible 5** | Projection des échus avant clôture. | fait et testé |
| **Faible 6** | `cahier_c8::db` : `40001` (sérialisation) et `40P01` (interblocage) sont un **conflit réessayable (409)**, jamais 500 ; le refus de garde « année figée » et « journal de clôture » en est un aussi (409 plutôt que 400 quand une écriture croise une clôture). Branché sur C2, C2 travaux, C6, C7, C8, C9. | fait et testé (SQLSTATE réels levés par `RAISE`) |
| **Faible 7** | `POST /mon-export` : `INSERT … ON CONFLICT DO NOTHING` sur l'index unique partiel, puis relecture de l'export retenu. Cours trop gros : **archive par morceaux** (la période est coupée en deux jusqu'à tenir sous le plafond de 4 000 000 caractères ; un morceau vide est omis ; titre « partie du … au … » ; une ligne d'archive par morceau, versions croissantes) ; **un seul jour** au-dessus du plafond est **tronqué avec la mention explicite « ARCHIVE TRONQUÉE … caractères omis »** (cas pratiquement impossible : 200 entrées de 20 000 caractères le même jour). Choix : sans perte et sans blocage de la clôture. | fait et testé (deux POST simultanés = un export ; 260 travaux de 20 000 caractères = plusieurs morceaux, 261 travaux au total, aucun perdu) ; troncature d'un jour : non testée |
| **Faible 8** | `c9_partager` : 404 uniforme (cours inexistant, sans droit, discipline non enseignée). | fait et testé (le test existant attendait 403) |
| **Faible 9** | Registre : `ent_cdt_cloture` en `perso`, `ent_cdt_acces_inspection_cours` en `.export()`. | fait (`registre_donnees` vert) |
| **Faible 10** | Rôle des inspecteurs lu avec les casquettes : une casquette `personnel` (ouverte, datée) fait un personnel même si le profil principal est « parent » ; une casquette `personnel` fermée ou à venir en prive même si le profil principal est un rôle de personnel ; sans casquette, le profil principal fait foi. À l'octroi **et** à chaque lecture. | fait et testé |
| **Faible 11** | `cdt_c8_archive_decider` relit l'archive dans sa transaction, sans second contrôle `cahier.exporter` : la décision enregistrée et la réponse suivent le seul droit `cahier.cloturer`. La liste des archives accepte `cahier.exporter` **ou** `cahier.cloturer`. | fait (le test de décision existant passe ; le cas « cloturer sans exporter » n'est pas isolé : aucun profil par défaut ne le porte) |
| **Faible 12** | « 20 caractères » partout (rapport §2, écran, code, conception de la passe 1). | fait |

### Tests de course (refaits) et tests ajoutés

* **Course, deux sens, sans `sleep` comme preuve** (`c8_cloture_et_ecritures_se_serialisent_par_verrou_dans_les_deux_sens_sans_delai`) : la synchronisation est une **attente conditionnelle** (sonde de `pg_stat_activity` jusqu'à observer une requête bloquée sur un verrou ; la cadence de la sonde, 5 ms, n'est pas la preuve). Sens 1 : écriture non validée → la clôture réelle est observée bloquée sur l'année → validation → l'archive contient l'écriture. Sens 2 : la clôture **réelle** (`cdt_c8_cloturer`) est retenue après avoir pris le verrou (une archive fantôme non validée occupe sa version) ; une écriture **réelle** démarre et se bloque à son tour ; on observe les deux blocages, puis on lève l'obstacle : la clôture aboutit, l'écriture échoue avec le **conflit exact** (409 « année close »), sans trace. Pour que ce banc existe, le verrou de l'année est `FOR NO KEY UPDATE` (les insertions qui ne font que référencer l'année prennent `FOR KEY SHARE` et ne sont plus bloquées inutilement).
* **F4** (`c8_f4_un_visa_de_classe_ne_compte_…`) : deux cours dans la classe, un visa de classe : la direction compte 2 éléments, le professeur d'un des deux cours 1. **F7** (`c9_f7_…`) : deux niveaux (`niveau_id`), copie 6e → 5e refusée (409), 6e → 6e permise. **F11** (`c8_f11_…`) : un rôle libre portant `cahier.acces_inspection` **sans** `cahier.viser` liste les cours et accorde un accès (jamais à soi-même).
* Tests ajoutés ou refaits : `tests_cahier_c8.rs` **28** (16 de la passe 1 + 12 de la passe 2), `tests_cahier_c9.rs` **7**.

### Mesures

PostgreSQL 16 local, bases neuves dont le nom contient « test » (`ent_test_passe2` à `ent_test_passe4`), `ENT_ENV=test`, `--locked`, un seul `CARGO_TARGET_DIR`. `ent-db` (labo) `tests_cahier` : **123 sur 124 verts** en `--test-threads=4` ; le 124e, `c4_groupe_de_4_classes_…_performance` (aperçu de 35 élèves en 250 à 284 ms pour un critère de 200 ms), est un critère de **durée** sensible à la charge : seul, il passe (même tête). `registre_donnees`, `tests_rls`, `rgpd`, `conservation`, `migrate`, isolation : **165 verts** ; `ent-auth` : **64 verts**. Aucun test ne dépend de l'heure réelle ni du jour UTC (publications échues rétrodatées de valeurs fixes ; seul le délai d'expiration d'une demande est rétrodaté de 15 jours).

