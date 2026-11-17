# Socle 15 — rapport des lots A11d, A11e et A11f (écrans Leptos)

**Date :** 2026-11-16 — **Branche :** `ent-socle-15-code-a11def` (partie d'`origin/ENT`, fusionnée avec `origin/ENT` 95eb891e/c75d51af : A4bis et AS14b-bis inclus ; rien poussé sur `ENT`).
**Niveau atteint :** *implémenté et testé* (local ; CI : voir le message au pilote). **Non validé** par Florian, **pas prêt pilote**, rien déployé, aucune donnée de production touchée. Lecteur d'écran : **non fait** (axe-core et clavier seulement).

## 1. Ce qui est livré

| Lot | Écran (route web) | Menu | Contenu |
|---|---|---|---|
| **A11d** | **Mes heures** `/web/personnel/mes-heures` | lien depuis « Mes remplacements » | heures constatées par mois et nature avec leur état, total par nature, état annuel ; **séances remplacées sans appel** à attester ; contestation motivée (motif codé, note) |
| | **Heures et états mensuels** `/web/personnel/etats-heures` | « Heures et états » (moteur : `heures.*` ou `paie.absences`) | employeur (selon le rôle) et mois ; *Heures du mois* : filtre par nom, saisie d'une heure **sans séance**, validation du mois, correction (règle une contestation), régularisation d'une heure figée, contestation ; *État mensuel* : figer (mois terminé seulement), signer, exporter (CSV) ; *Paie de l'OGEC* : préparer, exporter ; *DSN* : arrêts sous cinq jours, « signalé le » |
| **A11e** | **Plan de remplacement** `/web/personnel/rcd` | « Plan de remplacement » (`rcd.gerer` ou `rcd.remontee`) ; volontariat par lien depuis « Mes remplacements » | plan de l'année versionné (brouillon modifiable, nouvelle version, avis du CP, présentation, transmission, campagne, comptes rendus, 2 par an signalés) ; mon engagement et **mon** compteur (position par rapport à la médiane, sans nommer personne) ; vivier et compteurs nominatifs (direction) ; remontée hebdomadaire **sans nom** : préparer, contrôler, laisser partir, rejeter, exporter (tableau équivalent) |
| **A11f** | **Réglages des absences** `/web/personnel/parametrage-absences` | « Réglages des absences » (`absences.parametrer`) | **seuil de la mention « répond rarement »** par établissement (1 à 20), EDT de référence (allumer/éteindre), circuit (règles de valideurs, instructeurs, informés) |

Actions de la conception couvertes : ABP-200, 201, 202, 203, 204, 205, 206 (export tableau), 207, 208, 210, 211, 213, 214 (vue par agent des heures), 219, 220 à 224, 226 à 228, 229 à 231, 260 à 262 (seuil), 272. **Partielles / non faites :** ABP-209 (heures et jours d'absence d'un salarié pour la paie : pas d'écran), 212 (alerte HSE convenance), 215, 216, 217, 218, 225 (campagne : lancée, sans vœux d'EDT), 227 (mettre fin : oui ; modifier la date de fin : non), 259, 263 (natures propres), 264 à 271 ; format académique de l'état (Q-AP5) et API SI-RCD (Q-AP3) non obtenus : export CSV.

**Règle « une heure constatée seulement sur séance assurée » (R-AP28).** L'écran ne propose de saisir que `hse`, `mission_pacte`, `vacation`, `service_non_fait` ; le remplacement de courte durée et le rattrapage dû n'existent que par une séance assurée (appel fait, ou attestation d'une séance que le **serveur** liste). Testé côté modèle et côté serveur (400 sur saisie de `rcd` / `rattrapage_du`).

## 2. Backend ajouté (strict nécessaire)

| Sujet | Détail |
|---|---|
| **Migration 2315** (accordée par le pilote) | `ALTER TABLE ent_absp_param` : `seuil_sans_reponse_min` SMALLINT 1–20 défaut 3, `seuil_modifie_le`. **Aucune nouvelle table** (la table `ent_absp_param` existait, RLS posée en 2301) : pas de compte RLS, pas d'entrée de registre, `MIGRATION_TAG` inchangé. Écart avec la forme proposée (nouvelle table `ent_absp_parametre`) : même PK, mêmes colonnes d'audit, plus simple. |
| Routes | `GET` et `PUT /personnels/absences/parametres` dans `absences_personnels_impact.rs` (source déjà dans `SOURCES`). Lecture : `parametrer` ou organisateur ; écriture : `parametrer` seul (direction, administration). Journal (objet `param`). |
| Action | **Aucune** : `personnels.absences.parametrer` existait (utilisée par circuit et natures) ; `TOUTES` inchangé. |
| Seuil lu | `propositions_impact` lit le seuil de l'établissement (`repond_rarement(expirees, traitees, seuil)`), défaut 3. |
| Corrigé après revue | `ValiderHeuresBody.employeur` (la validation de l'État ne validait pas aussi l'OGEC, jamais affiché) ; la saisie d'heure refuse un agent hors du périmètre d'employeur du saisisseur (404). |

Compteurs de la règle 28 relus après fusion : `TOUTES`, `SOURCES`, compte RLS, `MIGRATION_TAG` **inchangés** par ce lot.

## 3. Permissions et minimisation (jouées contre le vrai backend)

`ent-web-server/tests/absences_personnels_hrcd.rs` (4 tests) et `ent-db` (`tests_absences_personnels_parametres.rs`, `a11f_le_seuil_de_l_etablissement…`) :
- pages rendues avec CSP, sans style en ligne ;
- heures : un professeur ne saisit, ne lit la liste, ne fige, ne signe ni n'exporte rien ; ne lit que **ses** heures ; l'export des heures ne dit rien d'une absence ; une heure figée se **régularise** (jamais réécrite : la durée figée reste, l'écart est signé) ; l'autre employeur ne voit pas l'heure ;
- RCD : un brouillon n'est lisible que de la direction ; un professeur ne crée rien, ne lit ni les versions ni le vivier ; engagement unique par agent et par année ; **remontée sans nom** (test de structure sur le JSON, CSV sans nom ni adresse) ; une remontée non contrôlée ne s'exporte pas ; rien d'un autre établissement ;
- réglages : seuil lisible de l'organisateur, jamais d'un professeur ni d'un parent ; écriture refusée à professeur, CPE, secrétariat, accueil ; bornes ; autre établissement non touché ; **seuil changé → la mention apparaît ou disparaît** (3 → 4 : disparaît ; 2 : revient) ; refus sans le droit ; journal d'un fait par changement.

## 4. Vérifications locales (chaîne 1.99.0)

- Clippy `-D warnings` comme la CI : `ent-web-ui`/`api`/`app` (SSR, `--all-targets`), `ent-web-server` et `ent-web-session` (avec `labo`), `ent-web-app` wasm32 (`hydrate`) : **0 avertissement**. Backend : `ent-db`, `ent-api`, `ent-types` (`--tests`, `labo`) : voir §9. Formatage : commande exacte de la CI (web + `ent-types`) sans écart ; jamais `cargo fmt --all`. Aucun `#[ignore]`.
- Tests unitaires web : 33 + 524 + 14 verts (dont ~110 nouveaux : durées, mois, cycle d'une heure, plan, procédure, engagement, semaine, remontée, seuil, règles de circuit, menu).
- `ent-db` : 13 tests verts (A4bis, A7, A11f). `ent-web-server --features labo` : 4 verts (nouveau fichier) + `absences_personnels` (4 sur 5 ; le test `ce_matin_…` dépendait de l'heure (fuseau d'établissement à l'ouest de Paris → comptes « avant arrivée » entre 22 h et minuit UTC) : corrigé, fuseau fixé à l'est jusqu'à UTC+14 ; rejoué vert à 23 h 25–23 h 44 UTC, la fenêtre qui échouait).

## 5. Parcours joué dans Chromium

Script `docs/socles/15_ABSENCES_A11_DEF_captures/parcours.mjs` (Playwright, axe-core 4.10.2 WCAG A/AA + bonnes pratiques, serveur `labo`, établissement neuf). Parcours : le **professeur** voit sa HSE et la conteste → la **direction** règle la contestation, saisit une heure (natures de remplacement absentes de la liste), valide, **fige**, **signe**, exporte, **régularise** → plan RCD (créer, procédure, campagne) → le professeur se porte **volontaire** (il ne voit ni vivier ni remontée) → la direction lit le vivier, prépare, contrôle, exporte, laisse partir la remontée → réglages : seuil hors bornes refusé à l'écran, seuil à 5 enregistré, règle de circuit ajoutée → le CPE lit le seuil sans pouvoir le changer, un professeur est refusé.

| Contrôle | Résultat |
|---|---|
| axe-core, 22 états (dont 9 dialogues) | **0 violation** |
| Clavier | tout atteint (contester, régler, saisir, seuil, enregistrer) |
| **375 px** (mes heures, heures, état, plan, remontée, réglages) | débordement **0 px** |
| Console | uniquement les 403 attendus (refus du professeur, circuit pour l'organisateur) |

Captures : `docs/socles/15_ABSENCES_A11_DEF_captures/captures/` (01 à 34, `*-375.png`, `rapport-navigateur.json`).

## 6. Revue indépendante (`review-sonnet`, lecture seule) et traitement

**Corrigés :** seuil non lu (B1 : branché) ; validation débordant sur l'autre employeur (C1) ; saisie hors périmètre (C2) ; date du réglage partagée avec l'EDT (C4 : colonne dédiée) ; onglet Paie/DSN obsolète au changement d'employeur et droit `paie.absences` (C5) ; noms accessibles contenant le texte visible (C6) ; régularisation selon `valider` ou `exporter` (C11).
**Corrigé ensuite sur demande du pilote : C9, masquage des petits effectifs** (`SEUIL_PETIT_EFFECTIF` = 3, S-1512, **à confirmer avec le DPO**) : sous 3 agents distincts dans un regroupement (discipline, niveau, catégorie), la catégorie devient `non_detaille` (« non détaillé (effectif trop faible) »), lignes fusionnées, effectif affiché « < 3 » (jamais le chiffre), CSV compris ; migration 2315 étendue (CHECK de catégorie, colonne `effectif`). Test : une seule professeure de latin en 6ᵉ → aucune catégorie visible.
**Non corrigés (assumés) :** C3 (les écrans de gestion lancent leurs requêtes avant la garde de droit ; un 403 par section sur accès direct, rien ne fuit) ; C7 (dialogue fermé et note vidée avant le résultat : l'écriture commune `Ecriture` n'a pas de rappel de succès, fichier non touché) ; C8 (heure constatée après la figeure d'un mois : traitement backend à décider) ; C10 (boutons sur ses propres heures, refusés ensuite par le serveur) ; plafond de volume 500 h (écran) contre 999 h (serveur) ; « me porter volontaire » proposé à un compte sans fiche agent (refus serveur affiché).

## 7. Limites assumées

Fuseau d'affichage (§15 : heure de Paris) ; annuaire borné à 200 agents (sinon « Agent non listé ») ; en labo les noms sont des e-mails ; le flux SI-RCD n'est pas branché (tableau exportable) ; seul le seuil est réglable parmi S-1500 à S-1517 (les autres sont fixés dans le code, non montrés) ; mobile non fait ; lecteur d'écran non fait.

## 8. Fichiers communs touchés

`ent-web-app/src/lib.rs` (4 `<Route>`), `modules/absences_personnels/mod.rs`, `noyau/navigation.rs` (4 constantes, 3 entrées selon le moteur, 1 test ; **aucune** liste de profil modifiée), `ent-web-api/src/lib.rs` (`mod` + `pub use`), `remplacements.rs` (2 liens), `style/absences-personnels.css`, `ent-db/src/lib.rs` + `migrate.rs`, `docs/FRONTEND_LEPTOS.md` §17, `PROJECT_STATUS.md`, `MIGRATIONS.md`.

## 9. Fusion, CI

Voir le message au pilote (SHA, numéro de run). Clippy backend : `cargo clippy --locked -p ent-db -p ent-api -p ent-types --tests --features ent-db/labo,ent-api/labo`.
