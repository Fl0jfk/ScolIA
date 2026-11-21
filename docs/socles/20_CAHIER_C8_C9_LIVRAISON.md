# Socle 20 — lots C8 (visa, inspection, archive) et C9 (progressions et reconduction) : livraison

**Date :** 2026-11-21 — **Branche :** `ent-socle-20-code-c8c9`, partie de `origin/ENT` d23a6f66 — **Statut :** implémenté et testé en local ; CI et relecture du pilote à venir (voir « Tête et CI »). Conception : `20_CAHIER_DE_TEXTES.md` (sur `ent-mc0hoe`) §14, §16, §19, §20.11, §21 (R-C17 à R-C19, R-C28), §23 (écrans 3, 7, 8), §26 (lots C8 et C9).

Légende : **implémenté et testé** (test automatisé local vert) · **implémenté, non testé automatiquement** · **non fait** (avec la raison).

## 1. Modèle de menace appliqué

| Règle | Où elle est tenue |
|---|---|
| 1. Ce qui est visé est figé | `ent_cdt_visa_element` (objet, version) en ajout seul ; visa sans UPDATE/DELETE/TRUNCATE (déclencheur) ; « modifié après visa » = version actuelle > version visée, avec texte vu et texte actuel, auteur, date, motif ; marque visible de la direction **et** de l'auteur. |
| 2. L'inspection voit ce qu'on lui ouvre | accès borné par CHECK (90 jours), cours listés, période du cahier ; lecture **journalisée avant d'être servie** (sans trace, rien n'est servi) ; entrées et consignes publiées seulement : aucun élève, remise, retour, variante ni brouillon ; journal lu de la direction et du professeur concerné. |
| 3. Archives et exports figés, ajout seul | exports : empreinte **recalculée par la base** (CHECK), UPDATE/DELETE/TRUNCATE refusés ; archives immuables (seule la décision change, une fois) ; journal de clôture en ajout seul ; réouverture motivée (10 caractères) et tracée. |
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
| Export demandé pour litige | motif obligatoire ; aucun nom d'élève possible dans le texte |
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

**Backend (`ent-db`, base locale de test, un établissement neuf par test)** : `tests_cahier_c8.rs` (12) et `tests_cahier_c9.rs` (5), tous verts ; plus les 156 tests `tests_cahier` existants, `registre_donnees`, `tests_rls`, `rgpd`, `conservation`, `migrate` : **173 verts** au total sur le sous-ensemble lancé. Couverture de la matrice de séparation du §27 pour ce lot : direction, professeur en saisie, autre professeur, ancien/hors cours, CPE, comptabilité, élève, parent, inspecteur (pendant, expiré, révoqué, hors cours), autre établissement.
**Actions** (`ent-auth`) : 64 verts (positions de fin de catalogue mises à jour).
**HTTP de séparation** (`ent-api/tests/separation.rs`) : +25 spécifications d'établissement B (C8, C9) et +8 routes sans identifiant bornées à l'établissement de la session ; l'inventaire de routes (401 sans session) couvre les 34 routes nouvelles automatiquement.
**Frontend** : 14 tests natifs de logique d'écran (`modele_c8`, `modele_c9`), tests de navigation, rendu serveur des 6 pages (CSP, langue, aucun style en ligne) et test de refus par profil contre le vrai backend (`ent-web-server/tests/cahier.rs`).
Aucun test ne dépend de l'heure réelle ni du jour UTC (jours de l'établissement, séances à jours explicites, accès expiré inséré avec des intervalles relatifs à `now()` de la base).

## 6. Compteurs (règle 28, recalculés depuis `d23a6f66`)

`Action::TOUTES` 275 → **277** · compte RLS de `tests_rls.rs` **+8** (`… + 5 + 3`) · registre des données **+8** (domaine `cahier_textes`, 2 exceptions `SansHorodatage`) · `TABLES_ELEVE` **inchangé** · `SOURCES` / `NOMS_SOURCES` 69 → **71** · routes **+34** (24 + 10) · `MIGRATION_TAG` **inchangé** (3808) · migrations **2407**, **2408** (2419 reste réservée à C14).
Dépendances : **aucune** nouvelle. Colonnes JSON / tableau / BYTEA : **aucune** (`grep -i jsonb` vide sur 2407 et 2408).

## 7. Qualité

Zéro avertissement rustc et clippy 1.99 sur le code du lot (`cargo clippy --locked --workspace --all-targets --features ent-api/labo,ent-lab/labo -- -D warnings` ; frontend `-p ent-web-ui -p ent-web-api -p ent-web-app --all-targets --features ent-web-app/ssr -- -D warnings`) ; formatage ciblé (`cargo fmt -p` des crates du frontend et d'`ent-types`, `rustfmt` des seuls fichiers neufs du backend, jamais `--all`) ; `cargo metadata --locked` dans `ent-rust/web` vérifié.

## 8. Tête et CI

(à compléter après le push : tête, numéro de la pull request en brouillon, verdict de la CI « ENT Rust »)
