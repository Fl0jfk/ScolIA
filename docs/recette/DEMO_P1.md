# Démo P1 — le parcours joué dans un navigateur

**Date :** 2026-11-08 — **Branche :** `ent-demo-p1-ecrans` (base `ent-recette-2-correctifs`) — **Statut :** écrans corrigés et joués, **à relire par Florian**.
**Ce que c'est :** la recette 2 avait tout joué par l'API ; ici le même parcours est joué **à l'écran** (Chromium via Playwright, `deviceScaleFactor` 2, bureau 1280 px et mobile 375 px), sur le serveur Leptos réel (`ent-web-server`, compilé avec `--bin-features labo`), base PostgreSQL locale `ent_ci_test_a`, `ENT_ENV=lab ENT_ALLOW_LABO=1`, données de labo fictives. Aucune base ni secret de production ; Next.js non touché ; aucune migration.
**Rejouer :** `docs/recette/demo_p1/scripts/` (`reset_labo.sh` puis `NODE_PATH=$(npm root -g) node demo_p1.js`). Les captures citées ci-dessous sont dans `docs/recette/demo_p1/`.

**Aides de labo** (base locale seulement ; pas de messagerie en labo) : (1) le code de préinscription est retrouvé par son empreinte (`code_labo.py`, clé de développement publique) ; (2) le jeton d'invitation de la mère est remplacé en base par un jeton connu ; (3) son mot de passe est recopié d'un compte de labo (le lien « choisir un mot de passe » n'est pas créé en labo, RP2-21).

---

## 1. Ce que la démo a révélé (et ce qui a été corrigé)

| # | Constat vu à l'écran | Correction | Preuve |
|---|---|---|---|
| D1 | **La fiche élève se figeait** (onglet bloqué, boucle infinie) pour toute personne pouvant gérer les responsables, dès l'affichage. Deux `Effect` se répondaient (`ouvert ← cible`, `cible ← None`) et `RwSignal::set` notifie même une valeur inchangée. Invisible des tests (rendu serveur seulement). | `noyau/dialogue.rs::lier_ouverture` (comparaison avant écriture), utilisé par les 4 dialogues concernés (fiche : guichet, suspension, rétablissement ; infirmerie). | Navigateur : la fiche s'ouvre (captures 16 à 20). Piège ajouté ci-dessous (§5). |
| D2 | **Préinscription impossible** : « Action refusée : établissement requis » (la page n'envoyait pas l'établissement). | `?etab=` ou configuration serveur, comme la page de connexion (D-012 le rendra inutile). | Capture 03, test `pages_de_la_demo_rendues_par_le_serveur`. |
| D3 | **Le formulaire de préinscription ne permettait pas de déclarer** « ne pas solliciter » ni « adresse à protéger » (le contrat existait côté API). | Deux cases avec explications ; lien du second parent ; modèle d'écran testé. | Capture 04, test unitaire `reserves_declarees_a_la_preinscription`. |
| D4 | **La validation ne pouvait pas aboutir** dès qu'une fiche existante correspondait (409), et demandait un **identifiant de classe (UUID)**. | Écran de rapprochement humain (fiche existante ou nouvelle, foyer, ancien élève, confirmation des responsables déjà liés) puis récapitulatif et case de vérification ; plus de UUID ; la logique est dans `construire_validation` (testée). | Captures 12–13, tests unitaires + `admission_jusqu_a_la_fiche_et_cloisonnement`. |
| D5 | **Aucune invitation après validation** : le serveur web ne démarrait pas la distribution de l'outbox que lance le `main` d'`ent-api`. | `routeur_complet_avec_taches()` (binaire seulement ; désactivable `ENT_OUTBOX_DISTRIBUTEUR=0`). | Capture 14 (invitation reçue puis acceptée). |
| D6 | La fiche n'affichait ni **identité**, ni **scolarité**, ni **foyer** ; le lien s'affichait en code brut (`pere`). | Sections « Identité » (avec masquages expliqués) et « Scolarité » (statut dérivé, scolarisations, inscriptions annuelles avec l'année), foyer et libellé du lien dans le tableau. | Captures 15–17. |
| D7 | Réserves déclarées invisibles à l'instruction ; campagne en brouillon affichée « À confirmer ». | Pastilles « À ne pas solliciter (à confirmer par la direction) » et « Adresse protégée » ; état « Brouillon » pour les campagnes. | Capture 06. |

---

## 2. Le scénario pas à pas

Légende : ✅ joué et conforme · ⚠️ joué avec réserve (voir §4).

### Étape 1 — Secrétariat : ouvrir une campagne d'admission ✅
`/web/inscriptions` → « Nouvelle campagne » (libellé, année cible 2027-2028, niveaux et capacités, pièces dont une facultative) → « Créer » → « Ouvrir ». Captures `01`, `02`. Le professeur n'y a pas accès (403, test).

### Étape 2 — Famille (mère inconnue) : préinscription en ligne ✅
`/web/preinscription` : e-mail → code → espace (rien n'est montré avant la vérification) → formulaire : élève (Rose Martin, 4ᵉ), déclarante Inès Martin (**adresse à protéger**), second parent Marc Martin (père, **foyer distinct, « ne pas solliciter »**) → envoi → dépôt de deux pièces (déclaration du nom de fichier : **pas de vrai téléversement**, ⚠️ §4). Captures `03`, `04`, `05`, `05b` (375 px).

### Étape 3 — Instruire, admettre, valider ✅
- Secrétariat : instruction (`06`, les deux réserves sont visibles, jamais l'adresse), pièces acceptées, passage en instruction (`07`).
- Direction : décision « admis » (`08`) ; le secrétariat n'a pas ce bouton. La direction **prépare l'année 2027-2028** (`/web/referentiel/annees`, `09`, `10`) : sans elle, la scolarisation de l'élève attend la préparation (limite connue de RP2-10).
- Direction : écran « Protections » : la mention « ne pas solliciter » de Marc attend sa confirmation, l'adresse d'Inès est protégée (`11`).
- **Secrétariat valide** (RP2-11) : rapprochement, case de vérification, validation (`12`, `13`). Marc ne reçoit **aucune invitation** ; la mère reçoit la sienne, l'accepte (`14`).
- L'élève apparaît à la scolarité : **« Inscrit, entre à la rentrée — scolarisation à partir du 01/09/2027 · Première inscription · Année 2027-2028 · Prévue »** (`16`, `17`).

### Étape 4 — Fiche 360° et cloisonnement ✅
- **Direction** (`16`) et **secrétariat** (`17`) : identité, scolarité, responsables avec foyer, droits et payeur (100 % Inès), contacts, sections du dossier, actions possibles, chronologie ; le secrétariat voit les groupes de champs fermés expliqués.
- **Mère** (`15`, `15b` à 375 px) : voit l'élève et ses propres coordonnées ; le père « séparé » n'apparaît que par sa désignation, **coordonnées, adresse et droits remplacés par « Cette information concerne un autre responsable »** ; historique d'identité et données sensibles réservés au personnel.
- **Professeur sans classe** : « Dossier inaccessible » (`18`).
- **Professeur d'un seul groupe** (« 4B LV2 allemand », Prof 3) : voit Harry (`19`), **pas Ginny** (`20`).
- Tests contre le vrai backend : `ent-web-server/tests/demo_p1.rs` (3 tests) : parcours complet (famille → validation par le secrétariat → scolarité `a_venir` + inscription `prevue` → père protégé sans invitation → autre famille et professeur refusés), pages rendues, professeur de groupe.

### Étape 5 — Famille : signaler une erreur sur l'identité ❌ (route absente)
**Il n'existe aucune route de proposition** (ELV-18 / ELV-40) : `ent_eleve_proposition` et `eleves.propositions.traiter` n'ont ni table utilisée ni route (lot E5 non fait ; RP2-17 toujours vrai). Conformément à la consigne, **aucun écran n'a été ajouté** (le frontend ne promet pas une fonction absente) ; l'écran se branchera sur `POST /eleves/{id}/propositions` dès que le lot E5 existera.

---

## 3. Vérifications

- `cargo clippy --locked` : `ent-web-ui`, `ent-web-api`, `ent-web-app` (`--features ent-web-app/ssr --all-targets -D warnings`), `ent-web-app` pour `wasm32` (`--features hydrate -D warnings`), `ent-web-server --features labo --all-targets -D warnings` : **sans erreur**.
- `cargo test --locked -p ent-web-ui -p ent-web-api -p ent-web-app --features ent-web-app/ssr` : **26 + 308 + 14 réussis**.
- `cargo test --locked -p ent-web-server --features labo -- --test-threads=1` : **non lancé en entier** (chaque binaire de test pèse 3,5 Go, le conteneur a ≈ 38 Go) ; lancés un à un et **verts** : `demo_p1` (3), `inscriptions` (2), `dossier` (2). Les 18 autres fichiers sont à rejouer par la CI GitHub.
- Formatage : `cargo fmt --manifest-path ent-rust/web/Cargo.toml -p ent-web-app -p ent-web-server -p ent-web-api --check` : OK (jamais `--all`).
- Navigateur : parcours complet rejoué **depuis une base vide** par `demo_p1.js` (aucune erreur de console hors 401/403 attendus).

## 4. Ce qui manque encore pour une démo complète

1. **Propositions de la famille / du terrain** (étape 5) : route E5 inexistante.
2. **Pièces : pas de vrai dépôt de fichier** (le nom est déclaré) ; pas de visionneuse pour l'instruction.
3. **La famille ne voit pas sa « Scolarité »** (route refusée aux responsables : « ne vous est pas ouverte ») ni l'INE (RP2-22) ; la mère doit pouvoir lire la rentrée de son enfant.
4. **La fiche du personnel n'affiche pas la mention « ne pas solliciter » du père** (visible seulement à l'instruction et à l'écran « Protections ») : prévoir une alerte sur la fiche et sur le tableau de bord de la direction.
5. **Deux foyers de même libellé** (« Foyer MARTIN » pour la mère et le père séparé) : désambiguïser (foyer de la mère / du père).
6. **Messagerie absente en labo** : codes, invitations et mot de passe simulés ; le message « un lien pour choisir votre mot de passe vous a été envoyé » est inexact en labo (RP2-21).
7. **Année cible à préparer à la main** avant la validation pour voir la scolarisation (sinon elle attend) ; l'affectation en classe se fait ensuite (socle 5).
8. **Tableau des responsables à 375 px** : défile horizontalement (lisible, pas encore en cartes) ; les captures « fullPage » montrent la barre de navigation mobile au milieu de la page (artefact de capture).
9. **Gabarit des messages du serveur** : « n'ouvre pas l'données sensibles », « la identifiants » (accords à corriger côté `ent-api`) ; « aucun note / aucun option » dans les sections du dossier.
10. **Pièces acceptées** : les boutons « Accepter / Refuser » restent affichés ; confirmation avant « Ouvrir / Fermer la campagne » et « Valider » à ajouter (INS : sensibles à confirmer) — la validation a désormais sa case de vérification.
11. **Rien d'accessible validé au lecteur d'écran** (clavier et 375 px regardés, pas axe-core ni lecteur d'écran) : les écrans ne sont donc pas déclarés « accessibles ».
12. Restent ouverts de la recette 2 (hors écrans) : RP2-13 (correctifs de la recette 1 à fusionner), RP2-18, 19, 20, 24 (la mère doit-elle reconfirmer le lien ? ici le lien était actif directement).

## 5. Pièges ajoutés (à reporter dans `docs/FRONTEND_LEPTOS.md` §8 à la relecture)

| Symptôme | Cause | Règle |
|---|---|---|
| Onglet figé sur une page qui rend bien côté serveur | deux `Effect` qui s'écrivent mutuellement ; `RwSignal::set` notifie même sans changement | comparer avant `set`, ou utiliser `noyau::dialogue::lier_ouverture` ; les tests de rendu serveur ne voient pas ce défaut : jouer la page dans un navigateur |
| Le serveur web ne réagit pas aux événements (aucune invitation) | tâches de fond d'`ent-api` lancées dans son `main`, pas dans `routeur_complet` | lancer `routeur_complet_avec_taches()` depuis le binaire |
| `No space left on device` à l'édition de liens | chaque binaire de test d'`ent-web-server` pèse ≈ 3,5 Go | lancer les tests un fichier à la fois et effacer `target/debug/deps/<test>-*` |
