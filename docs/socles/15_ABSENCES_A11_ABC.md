# Socle 15 — rapport des lots A11a, A11b et A11c (écrans Leptos)

**Date :** 2026-11-16 — **Branche :** `ent-socle-15-code-a11` (partie d'`origin/ENT`, rien poussé sur `ENT`).
**Conception :** `docs/socles/15_ABSENCES_PERSONNELS.md` (sur `origin/ent-mc0hoe`) : ligne A11 du §28.1, écrans du §25, droits du §23, cas limites du §20.
**Niveau atteint :** *implémenté et testé* (local + CI, voir le run en §9) ; **non validé** par Florian, **pas prêt pilote**, rien déployé, aucune donnée de production touchée. Lecteur d'écran : **non fait** (axe-core et clavier seulement).
**Aucune migration. Aucune route, aucune action, aucun type backend ajoutés ni modifiés** : les écrans appellent les routes `/personnels/absences/*` des lots A0 à A9, sans les réécrire. Les compteurs de la règle 28 (`TOUTES`, `SOURCES`, `NOMS_SOURCES`, compte RLS, `MIGRATION_TAG`) ne bougent donc pas par ce lot.

## 1. Ce qui est livré

| Lot | Écran (route web) | Pour | Ce qu'il fait |
|---|---|---|---|
| **A11a** | **Déclarer une absence** (`/web/personnel/absences/declarer`) | tout le personnel ; collègue absent si droit `declarer` sur l'établissement | quand (aujourd'hui, demain, dates précises, matin, après-midi, heures), nature dans la **liste courte de l'engagement** ou « je préciserai plus tard », texte libre **seulement** pour une nature non sensible (200 caractères), préférence de traitement, plusieurs périodes (absence discontinue), pièce facultative (arrêt : attestation « volets 2 et 3 ») ; résumé avant l'envoi ; après l'envoi : chevauchements signalés (jamais bloquants), **« mes séances touchées »**, lien vers la continuité pédagogique |
| | **Mes absences** (`/web/personnel/absences`) | tout le personnel | en cours et à venir, passées ; alerte « une absence a été signalée à votre nom » |
| | **Signaler une absence** (`/web/personnel/signaler-absence`) | accueil, vie scolaire, adjoint, direction (`signaler`) | sans nature, sans motif ; l'agent est invité à confirmer |
| **A11b** | **Fiche d'une absence** (`/web/personnel/absences/{id}`) | l'agent (la sienne) ; valideur, instructeur, direction | en-tête (agent, période, état, nature **si l'API la fournit**, traitement, **prochaine étape**), sections : décision et actions, pièces, fil, séances touchées (et « au retour » pour l'agent), historique |
| | **Files** (`/web/personnel/instruction-absences`) | valideur, instructeur | à valider, à traiter, échéances (arrêt sous 48 h, pièces), compteurs, filtre par nom, **validation en lot** des seules convocations et formations |
| **A11c** | **Remplacements du jour** (`/web/personnel/ce-matin`) | adjoint, direction, vie scolaire (`lire_organisation`) | aujourd'hui, demain, autre date (veille à +14 jours) ; absents ; séances par heure et par état ; **propositions avec leurs raisons** ; solliciter, désigner en urgence, appel à volontaires, annuler la séance (élèves pris en charge, sortie anticipée), sans objet, lever une réponse ; **plan en lot** avec aperçu et cases à décocher |
| | **Mes remplacements** (`/web/personnel/remplacements`) | tout le personnel | accepter, refuser (motif légitime codé, ou sans motif), accuser réception d'une désignation |

Fichiers : `ent-web-api/src/absences_personnels.rs` (client, méthodes `absp_*`), `ent-web-app/src/modules/absences_personnels/` (`modele.rs` pur et testé ; `declarer.rs`, `signaler.rs`, `mes_absences.rs`, `fiche.rs`, `fiche_pieces.rs`, `files.rs`, `ce_matin.rs`, `remplacements.rs`, `fichiers.rs`, `commun.rs`), `style/absences-personnels.css`, `ent-web-server/tests/absences_personnels.rs`. **L'UI JavaScript `ui.rs` n'est pas touchée** (F-001).

### Actions de la conception couvertes

| Lot | Couvertes | Partiellement |
|---|---|---|
| A11a | ABP-100, 101, 102, 103, 107, 109, 110, 111, 112, 113, 114, 115, 117, 120, 134, 140, 143, 144, 145, 146, 148, 150, 159, 172 | 108 (chevauchements **signalés** et annulation depuis la fiche ; pas le choix « fusionner / garder les deux » dans l'écran) ; 118 (lien vers la continuité pédagogique, aucune écriture) ; 116 (possible par l'annuaire, **non testé** avec un agent sans compte) |
| A11b | ABP-121, 122, 123, 124, 125, 126, 127, 128, 131, 135, 141, 142, 147, 170, 171, 174, 288, 289 | 120 (le circuit montre des rôles et des sources, pas de nom : le DTO n'en porte pas) ; 127 (la **liste des effets à défaire** n'est pas fournie par l'API : l'écran dit seulement qu'ils sont à défaire) ; 124 (pas d'aperçu serveur d'un lot : confirmation et liste blanche côté écran) |
| A11c | ABP-156, 158, 165, 166, 167, 168, 169, 175, 176, 177, 178, 185, annuler la séance et sans objet (§11.1), lever une réponse | 158 (liste, séance, salle, classe, heure ; **pas** le travail prévu, la liste d'appel ni les aménagements : le DTO n'en porte pas) ; 168 (une demande par séance, première proposition du serveur) |

## 2. Ce qui n'est pas fait (et pourquoi)

| Absent | Raison |
|---|---|
| A11d (heures et états), A11e (RCD), A11f (paramétrage) ; mobile | hors périmètre demandé |
| ABP-104 (saisir une absence déjà validée ★), 105 (import OCR), 106 (choix de l'engagement), 119 (réponse d'absence de messagerie), 129, 130 (injustifiée ★), 132, 133 (déléguer, reprendre une file), 136 à 139, 149 | pas de route correspondante, ou ré-authentification récente (★) non câblée dans le frontend |
| ABP-161 à 164 (purge à la demande, gel de conservation, confidentialité renforcée), 180 et 181 (suppléance) | les routes existent (A2, A5) mais l'écran appartient à un autre sous-lot (DPO, direction) ; **aucune interface n'est donc proposée** : ne pas conclure que la conservation est « prête » |
| ABP-155 (présent / absent / inconnu), 157 (professeur principal), 154 (AESH), 173 (calendrier), 179, 182, 183 (autre cours, déplacer, continuité encadrée) | pas d'écran dans ces lots ; 155 a sa méthode de client (`absp_presence`), testée, sans page |
| Famille et élève (écran 10 du §25) | c'est l'EDT du socle 10 |
| Lecteur d'écran | non fait ; `FRONTEND_LEPTOS.md` §6 l'exige avant de déclarer un écran accessible |

## 3. Permissions et minimisation (côté serveur, jouées contre le vrai backend)

Tests `ent-web-server/tests/absences_personnels.rs` (5 tests, base PostgreSQL locale, un établissement neuf par test) :

| Règle | Test |
|---|---|
| Pages rendues par le serveur : squelette, CSP, aucun style en ligne, aucun script sans nonce | `les_pages_sont_rendues_avec_csp_et_sans_style_en_ligne` |
| Parent : 403 partout ; collègue : **404 uniforme** sur la fiche, le fil, les pièces, le lien, l'historique, l'impact et l'étape suivante ; l'agent est refusé sur sa propre validation ; un professeur ne valide pas | `declarer_instruire_pieces_et_fil_avec_les_permissions_du_serveur` |
| Aucun nom de fichier renvoyé ; lien de pièce refusé au collègue ; consultations de pièces lisibles de l'agent seul | idem |
| Signalement de l'accueil : **aucune nature lue** ; l'agent confirme et précise | idem |
| **Autre établissement** : fiche 404, fil, pièces, historique, impact refusés ; file « à valider » sans l'absence ; décision refusée ; tableau sans l'absente | `rien_d_un_autre_etablissement_n_est_visible` |
| **Un professeur ne lit jamais un motif** : « ce matin » refusé (403), nature lue de l'agent seul, collègue en 404, propositions et sollicitations fermées | `un_professeur_ne_lit_ni_la_nature_ni_la_liste_des_absents` |
| Test de **structure** : le JSON du tableau « ce matin » ne contient ni `nature`, ni le code de la nature, ni `traitement`, ni `decision`, ni `texte_libre` ; une proposition n'a que des comptes et la discipline | `rien_d_un_autre_etablissement…`, `ce_matin_propositions…` |
| Parcours complet : séances touchées → tableau → propositions → solliciter (le message **ne dit pas la nature**) → seule la personne sollicitée accepte → « ce matin » reflète la réponse → lever → désigner → accuser réception → annuler la séance (sortie anticipée **refusée** sans information des familles) | `ce_matin_propositions_sollicitation_et_reponse_du_remplacant` |

Dans l'interface : aucune décision n'est proposée sur **sa propre absence** (R-AP10, décidé par la liste « mes absences ») ; les boutons d'écriture suivent le moteur de droits (`valider`, `instruire`, `edt.gerer`), un refus du serveur est affiché tel quel ; la 404 est dite « introuvable ou inaccessible », sans distinguer.

## 4. Vérifications locales (chaîne 1.99.0, la version de la CI)

- **Clippy `-D warnings`**, comme la CI : `ent-web-ui`, `ent-web-api`, `ent-web-app` (SSR, `--all-targets`), `ent-web-server` et `ent-web-session` (avec et sans `labo`), `ent-web-app` pour `wasm32` (`hydrate`) : **0 avertissement** (rustc et clippy). Formatage : `cargo fmt -p ent-web-ui -p ent-web-api -p ent-web-app -p ent-web-server -- --check` sans écart ; **jamais `cargo fmt --all`**. Aucun `#[ignore]`.
- **Tests unitaires** (`cargo test -p ent-web-ui -p ent-web-api -p ent-web-app --features ent-web-app/ssr`) : **29 + 458 + 14** tests verts (design system, client API, application ; dont 2 nouveaux tests de navigation et ~30 tests du modèle d'écran) ; `entetes_securite` et `droits_parite` verts (21 + 3). Le modèle d'écran (`modele.rs`) porte ses tests : composition de la déclaration (champ par champ), natures et texte libre, demi-journées, plusieurs périodes, signalement, actions de la fiche selon l'état, **jamais de décision sur sa propre absence**, lot sans nature sensible, arrêt sur 401, 403, 429, fenêtre du tableau, plan de remplacements, raisons d'une proposition sans donnée personnelle.
- **Tests contre le vrai backend** : `cargo test -p ent-web-server --features labo --test absences_personnels -- --test-threads=1` : 5 tests verts, et `contre_backend` (9) toujours vert (ajoutés à la CI, étape « Tests contre le vrai backend »).

## 5. Parcours joué dans un navigateur

Chromium + Playwright, axe-core 4.10.2 (WCAG 2 A et AA, 2.1 A et AA, bonnes pratiques), serveur `ent-web-server` compilé avec `labo`, base locale, **établissement neuf** et comptes créés par l'API de labo. Script : `docs/socles/15_ABSENCES_A11_captures/parcours.mjs` ; compte rendu brut : `rapport-navigateur.json`.

Parcours : le **professeur** déclare sa journée (formation, avec précision) → la fiche → la **direction** ouvre la file, la fiche, valide (clavier : Entrée, Échap, retour du focus), voit les séances touchées et l'historique → « ce matin » (chargement automatique du jour, puis autre date), propositions avec leurs raisons, sollicitation → le **remplaçant** voit sa demande et l'accepte → un **collègue** ouvre la fiche (introuvable) et « ce matin » (refusé).

| Contrôle | Résultat |
|---|---|
| axe-core, 10 états (mes absences, déclarer, déclaration confirmée, fiche agent, file, fiche valideur, boîte de validation, ce matin, propositions, mes remplacements…) | **0 violation** (voir §6, correctif du design system) |
| Clavier seul | Tab atteint la nature, « Déclarer l'absence », « Ouvrir », « Répondre » ; boîte de validation : Entrée l'ouvre, le focus y entre, Échap la ferme, **le focus revient sur « Valider »** |
| **375 px** (mes absences, déclarer, files, fiche, ce matin, mes remplacements) | **aucun débordement horizontal** (0 px) |
| Erreurs de la console | uniquement les 404 et 403 **attendus** du parcours « collègue » |

Captures (`docs/socles/15_ABSENCES_A11_captures/`) : `01` mes absences, `02` déclarer, `03` déclaration confirmée, `04` fiche de l'agent, `05` file à valider, `06` fiche du valideur, `07` boîte de validation, `08` fiche validée, `09` séances touchées, `10` historique, `11` ce matin, `12` propositions, `13` après sollicitation, `14` mes remplacements, `15` remplacement accepté, `16` fiche vue d'un collègue (introuvable), `17` « ce matin » vu d'un collègue (refusé), et les `*-375.png` à 375 px.

## 6. Défauts trouvés en chemin

| Défaut | Où | Traitement |
|---|---|---|
| `ZoneNotifications` : `role="status"` sur une liste `<ol>` (axe : **aria-allowed-role** sur *toutes* les pages, et **listitem** dès qu'une notification s'affiche) | design system `ent-web-ui/src/notification.rs`, préexistant | **corrigé** (zone annoncée = `div`, liste à l'intérieur ; 3 lignes de CSS). Seuls fichiers du design system touchés. |
| « Date invalide » affiché jusqu'à un clic sur « Actualiser » : un `Effect` posant le jour après le premier chargement d'une `LocalResource` ne relançait pas la requête | cet écran | corrigé (jour lu avant la ressource ; `FRONTEND_LEPTOS.md` §14) |
| **Remplacement accepté mais non appliqué tant que l'absence n'est pas validée** : `edt_remplacement::controler_absence_citee` exige `statut = validee` ; sinon la demande passe en `annulee/echec`, la séance revient « à traiter », et l'API répond 409 « valeur déjà existante (conflit d'unicité) » | **backend (A4)**, à signaler au pilote | écran : mention « le remplacement n'est appliqué que lorsque l'absence est validée » et message d'acceptation explicite. **Incohérent avec §8.1** (« on organise même sans confirmation ») : décision du pilote (valider d'abord, ou lever la règle pour `declaree`) |
| L'entrée de menu « Mes absences » s'allumait aussi sur la page des files (`lien_actif` compare par préfixe) | cet écran | chemins disjoints (`/web/personnel/instruction-absences`) |
| Fiche à 375 px : le contrôle segmenté à 5 sections débordait de 129 px | cet écran | passage à la ligne |

## 6 bis. Revue indépendante (`review-sonnet`, lecture seule)

Aucun constat bloquant ; minimisation, sécurité du frontend, arrêt des lots sur 401/403/429 confirmés. **Corrigés** : bilan d'un lot ou d'un plan perdu au rechargement de la liste ; lignes d'un plan écartées sans le dire (même remplaçant à la même heure : maintenant listées dans le bilan) ; échec de « est-ce la mienne ? » qui proposait les décisions (maintenant : aucune écriture) ; « Déclarer une autre absence » qui gardait la pièce et l'attestation « volets 2 et 3 » ; texte libre masqué mais envoyé ; message du fil vidé avant l'envoi ; course dans l'aperçu du plan (compteur de génération) ; cases cochées perdues quand le filtre masque puis réaffiche une ligne ; nom du fichier choisi affiché ; identifiant d'erreur de la partie de journée ; affirmation « aucune donnée personnelle » du classement reformulée.
**Non corrigés (assumés)** : saisie perdue quand `Ecriture::lancer` ignore un appel pendant une autre écriture (fichier commun `noyau/ecriture.rs`, non touché) ; désignation en urgence sans seconde confirmation (la case « urgence » est l'étape volontaire) ; erreur non bloquante sur une proposition présentée comme « aucun remplaçant libre » ; erreurs des périodes supplémentaires non rattachées à leur champ ; résumé sans les heures quand il y a plusieurs périodes ; `aujourdhui()` du navigateur (fuseau local) pour la fenêtre du tableau ; annuaire borné à 200 agents sans avertissement ; formulaire de signalement non vidé après « Signaler une autre absence » ; « N demandes sans suite » dans les raisons d'une proposition : **à arbitrer par Florian** (c'est un historique de comportement montré à l'encadrement, la conception §11.2 le prévoit pour l'équité, sans le nommer à un autre collègue).

## 7. Limites assumées

- **Heure de Paris** : `AbsenceDto` donne des instants UTC (fin exclusive) ; l'écran les affiche en heure de Paris (`a_paris`), comme les écrans plus anciens. Un établissement hors fuseau de Paris verrait un décalage : il faut que le DTO porte le jour et l'heure de l'établissement (comme `ElementAgenda`, `FRONTEND_LEPTOS.md` §13).
- **Traitement des heures** : la liste proposée à la validation est l'union des listes État et OGEC ; le serveur impose celle de l'engagement et sa réponse est affichée (le DTO ne dit pas l'employeur de l'agent).
- **Changer la nature** : la liste est celle de l'engagement **de la personne connectée** ; le serveur contrôle l'acceptabilité.
- **Circuit** : rôles et sources sans nom de personne (le DTO ne porte que des identifiants).
- **Consultations de pièces** : « une personne habilitée a consulté », sans nom (le journal ne donne que l'identifiant du lecteur).
- **Lot de validation** : séquentiel, une absence après l'autre ; le serveur revérifie chacune.
- **Tableau** : fenêtre de la veille à +14 jours (règle du serveur) ; au-delà, l'écran le dit et renvoie à la fiche.
- **Lecteur d'écran** non testé.
- Ne pas déduire du tableau « absents connus » qu'un agent est présent : une absence **signalée** n'est annoncée qu'avec la mention « pas encore confirmée ».

## 8. Fichiers communs touchés (pour l'union avec la session d'assiduité AS14)

| Fichier | Changement | Conflit probable |
|---|---|---|
| `ent-web-app/src/lib.rs` | 7 lignes `<Route>` (blocs `personnel/…`) après `absences` | union |
| `ent-web-app/src/modules/mod.rs` | `pub mod absences_personnels;` | union |
| `ent-web-app/src/noyau/navigation.rs` | 6 constantes, un bloc d'entrées après « Délégations », test de profil (2 entrées en plus par rôle du personnel), test des droits du moteur | union : les listes attendues de `entrees_selon_le_profil` doivent contenir **les deux** jeux d'entrées |
| `ent-web-api/src/lib.rs` | `mod absences_personnels;` et `pub use … ActionDecision;` | union |
| `style/ent-web.css` | une ligne `@import "absences-personnels.css";` | union |
| `ent-web-ui/src/notification.rs`, `ui.css` | correctif de §6 | aucun attendu |
| `.github/workflows/ent-rust.yml` | `--test absences_personnels` ajouté à l'étape « Tests contre le vrai backend » | union (une ligne) |
| `docs/FRONTEND_LEPTOS.md` §14, `docs/PROJECT_STATUS.md` | ajouts | union |

## 9. Fusion, CI et résultats chiffrés

*À compléter à la livraison* (fusion d'`origin/ENT` avec la règle 28, une seule CI, SHA et run) : voir le bas de ce fichier.

## 10. Reproduire

```bash
# Outillage (une fois) : chaîne 1.99.0 + wasm32, cargo-leptos 0.3.11, wasm-bindgen-cli 0.2.130, binaryen version_123.
rustup toolchain install 1.99.0 --profile minimal -c clippy -c rustfmt -t wasm32-unknown-unknown
# Le binaire de test avec `ent-api` dépasse 4 Go d'infos de débogage (liaison : « relocation R_X86_64_32 out of range ») :
export CARGO_PROFILE_DEV_DEBUG=0 CARGO_TARGET_DIR=$HOME/target-run
export ENT_DATABASE_URL=postgres://ent_ci@127.0.0.1:5432/ent_ci_test ENT_ENV=lab ENT_ALLOW_LABO=1 LEPTOS_OUTPUT_NAME=ent-web LEPTOS_SITE_PKG_DIR=pkg
cd ent-rust/web
cargo +1.99.0 test -p ent-web-ui -p ent-web-api -p ent-web-app --features ent-web-app/ssr
cargo +1.99.0 test -p ent-web-server --features labo --test absences_personnels -- --test-threads=1
cargo leptos build --bin-features labo && $CARGO_TARGET_DIR/debug/ent-web-server &   # http://127.0.0.1:3000
node ../../docs/socles/15_ABSENCES_A11_captures/parcours.mjs http://127.0.0.1:3000 ./captures   # Playwright + axe-core 4.10
```
