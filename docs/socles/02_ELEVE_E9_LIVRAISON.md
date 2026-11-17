# Socle 2 « l'élève » — lot E9 : suites d'E7-E8 (partie 1) et écrans E9a / E9b (partie 2) : livraison

Branche `ent-socle-2-code-e9` (base `ENT` 47a5be1e, qui n'a pas bougé : fusion sans objet, règle 28 relue). Conception : `docs/socles/02_ELEVE.md` §7.11, §11, §14, R-E9, ligne E9 du plan de lots ; décisions du pilote du 2026-11-17 (`02_ELEVE_E7_E8_LIVRAISON.md` §6). Niveau : **implémenté, testé en local** (CI GitHub à venir). Ni validé, ni prêt pilote.

## 1. Partie 1 — suites d'E7-E8 (backend)

| Point | Livré | Test |
|---|---|---|
| (a) catalogue E5 | `photo`, `email_eleve`, `mobile_eleve` au catalogue des champs (cibles `photo` / `coordonnee`, **non proposables par la file des propositions** : chacune a son circuit). Réglage par défaut : la famille **propose** (comportement conservé) ; fermée par un réglage `saisie_famille = non`, le refus est `403` pour la famille, jamais pour le secrétariat. `ReglageChamp::defaut_pour` | `e9a_photo_email_et_mobile…` ; `photo_et_coordonnees_propres_sont_au_catalogue…` (domaine) |
| (b) pastille « protocole » | domaine `classe` (nouvelle tuile « Vie de classe »), action `contraintes.lire` (administration, vie scolaire, infirmerie **et l'enseignant de l'élève sur sa portée**, dont le PP ; pas l'accueil ni un enseignant sans lien : la fiche n'existe pas pour lui), **sans aucun contenu de santé** (ni nature, ni échéance, ni date de début). Suit le module santé (M6, drapeau `module_sante`) ; masquée quand `pai_actif` est visible (un fait, une pastille). Le PAI reste réservé à `sante.lire` | `e9b_protocole_signale_l_infirmerie…` |
| (c) pastilles R-E9 | `nouvel_arrivant` (entrée depuis moins de 30 jours, ou première année dans l'établissement), `arrive_en_cours_annee` (entrée après les 14 premiers jours de l'année courante), `majorite_proche` (**booléen** : ni `valeur_date`, ni `du` ne portent la date ; J-30 inclus ; pas pour un émancipé ni une date contestée ou imprécise), `modalite_particuliere` (sans le code de la modalité). Une pastille n'est visible que si sa tuile l'est (testé pour **tout le catalogue** et tous les rôles) | `e9c_*` (4) |
| M7 | plafond **journalier** de SMS de vérification : 6 par élève et 300 par établissement sur 24 h glissantes (en plus des 3 par heure) ; l'e-mail n'est pas concerné | `m7_plafond_journalier…` |
| F2 | photo **réécrite sans métadonnées** (EXIF/GPS, commentaires, XMP, ICC ; orientation conservée par un APP1 minimal ; JPEG, PNG, WebP), pixels intacts ; un contenu illisible est refusé. Objet **retiré** (stockage + registre marqué supprimé) si la transaction échoue après l'écriture, sauf si la ligne existe (réponse de commit perdue) | `f2_*` (2) + `photo_nettoyage::tests` (5) |
| F3 | `ent_eleve.statut` : lignes NULL reprises, **déclencheur** qui le dérive à l'insertion et refuse de l'effacer. **Écart** : pas de `NOT NULL` (voir §4) | `f3_*` |
| F4 | `eleves_proteges_pour_acteur` écrit une consultation `liste_proteges` (jamais sur un refus) | `f4_*` |
| F5 | `ent_eleve_confidentialite_regles_chk` et `ent_eleve_statut_chk` **validées** après assainissement minimal (code de levée inconnu → `autre`) ; si des lignes restent hors règle, la contrainte reste NOT VALID avec un avertissement | `f5_*` |

Isolation par établissement : `e9_isolation_par_etablissement…` (+ la liste, §2). Les tests existants d'E7/E8 touchés : attentes des pastilles mises à jour (nouvelles pastilles, JPEG de test **complet** puisqu'un simple en-tête n'est plus une image).

### Migration
Plage **3200–3219** (réservation dans `MIGRATIONS.md`) : **3200** `eleve_e9_pastilles_statut_contraintes`, placée en fin de tableau `MIGRATIONS`. `MIGRATION_TAG` **inchangé** (`3808_…`, plus haute). Aucune migration versée modifiée.

## 2. Partie 2 — écrans E9a et E9b (Leptos 0.8)

*(état de validation : §5 et §6)*

- **E9a** `/web/eleves` : recherche tolérante (pause de frappe 250 ms, une lettre ne cherche rien), filtres classe / statut / pastille (chips avec effectifs), pagination de 50, portée dite à l'écran (`perimetre` pour un professeur), plafond de recherche signalé, états vide / chargement / erreur, raccourci « / », pas de photo dans la liste (initiales). Route serveur ajoutée : `GET /eleves/liste` (même annuaire, même portée et mêmes protégés écartés que la recherche, filtres avant pagination ; personnel seulement).
- **E9b** `/web/eleves/:id` : en-tête carte (photo avec repli, désignation, statut, classe, pastilles), onglets **Synthèse** (une tuile par domaine ; masquée avec sa **raison**, jamais son contenu), **Identité** (groupes ouverts, masquages expliqués, identifiants avec statut), **Historique** (frise comparée à la version précédente, correction distinguée du changement d'état civil, consultations pour l'audit). Élève absent / autre établissement / protégé : **même écran « Fiche introuvable »**. Le dossier 360° historique passe à `/web/eleves/:id/dossier` ; les familles et élèves gardent le dossier sur `/web/eleves/:id`.
- Menu : entrée « Élèves » selon le moteur (`eleves.annuaire`), jamais pour les familles.
- Types : `ent-types::eleve_synthese` (`EleveLigneDto`, `EleveListeDto`, `EleveFicheDto`) ; un test d'`ent-api` garde l'identité du JSON de `EleveDto` et d'`EleveFicheDto`.

## 3. Compteurs (règle 28, base `ENT` 47a5be1e inchangée)
- **`Action::TOUTES`** : 253, **inchangé** (aucune action). **RLS** : 715, inchangé (aucune table). **Registre des données**, **`TABLES_ELEVE`**, **`CHAMPS_CHIFFRES`** : inchangés. **`SOURCES` / `NOMS_SOURCES`** : 63, inchangés (la route de liste est dans `eleve_synthese.rs`). **`MIGRATION_TAG`** : inchangé.
- **Catalogue des pastilles** : 8 → **13** (`CATALOGUE_INDICATEURS` = `ent_catalogue_indicateur`, test d'égalité vert). **Champs du catalogue E5** : 10 → **13**. `VERSION_CATALOGUE` (conservation) : sans objet.
- Événements : `scolarite.modalite_changee` écouté en plus par `e8.indicateurs`.

## 4. Écarts, hypothèses, questions pour le pilote
1. **F3 sans `NOT NULL`** : accepté par le pilote (voir 4). Le déclencheur garantit l'absence de NULL en fonctionnement normal.
2. **Seuils [H]** : « arrivé en cours d'année » = après 14 jours ; M7 = 6 SMS par élève et 300 par établissement par 24 h ; « nouvel arrivant » = 30 jours ou première année. À valider avec le secrétariat.
3. **Qui voit quoi (décision du pilote du 2026-11-17, appliquée)** : une tuile **« Vie de classe »** (`classe`, ouverte par `eleves.lire` ou `contraintes.lire` sur l'élève) porte `protocole`, `nouvel_arrivant`, `arrive_en_cours_annee`, `modalite_particuliere` (booléens, sans détail) : administration, vie scolaire, infirmerie et **enseignants de l'élève / PP** les voient ; l'**accueil** non. `majorite_proche` (même tuile, actions `eleves.etat_civil.lire` ou `eleves.badge.gerer`) reste à l'administration et à la vie scolaire (et à l'infirmerie), **jamais à l'enseignant**. Règle de visibilité unique (la pastille suit sa tuile), testée pour tout le catalogue et tous les rôles ; test dédié `e9d_*` : l'enseignant de l'élève voit les quatre, pas `majorite_proche` ni le PAI ; sans lien, la fiche est absente ; un élève protégé reste absent d'un tiers (l'enseignant de la classe garde la fiche — portée pédagogique d'E4 — sans la pastille ni la tuile de protection). *À noter pour le DPO.*
4. **Statut NULL (décision du pilote)** : `ent_eleve.statut` est un cache dérivé des scolarisations ; **NULL = aucune scolarisation encore** (ligne reprise), jamais « actif ». Contrainte CHECK sur les valeurs (NULL admis) validée ; les lecteurs (« photos manquantes », pastilles) le traitent comme « à venir » ; pas de `NOT NULL`.
5. **Réencodage** : « sans EXIF » est une **réécriture sans métadonnées**, pas un réencodage (aucune dépendance de décodage d'images, D-062) ; un JPEG CMJN ou sans perte est refusé.
6. **Lien mort préexistant** : l'action famille « Voir les propositions » de `ent-db/src/portail.rs` pointe `/web/eleves` (liste du personnel, `403` pour une famille) : à rediriger vers le portail famille (E9f).
7. **Dossier 360°** : son droit « famille » est toujours calculé côté client (note de la conception, §18) : non repris dans la fiche hub (aucun droit calculé), mais non corrigé dans le dossier lui-même.
8. **PILOTAGE.md** : ce dépôt ne contient que les règles 1 à 9 ; j'ai appliqué les règles 17, 25 et 27 à 35 d'après ce que les rapports précédents en disent (aucune migration modifiée, compteurs recalculés, pas d'heure réelle, PR en brouillon).
9. **Non fait** (hors lot) : ELV-44/46 (lot dédié à venir), E9c à E9f, actions d'écriture de la fiche (modifier l'identité), tuile scolarité détaillée, désambiguïsation R-E8, vues enregistrées, export.

## 5. Tests
Local, PostgreSQL 16, Rust **1.99.0** (`--locked`), `--test-threads=1`, `CARGO_INCREMENTAL=0` :
- **Backend** : `ent-db` (`tests_eleve_*`, `tests_d036`, `tests_rls*`) **verts** (dont `tests_eleve_e9` : 16, `tests_eleve_propositions` et `jour_etablissement` adaptés) ; `domain-eleve` **301** ; `ent-api` lib `eleve_synthese` 2, `separation` **55** ; clippy 1.99 `-D warnings` (workspace, tests, labo) : **0 avertissement**.
- **Frontend** (workspace `web`) : `ent-web-app` **574** tests natifs (dont 28 du modèle d'écran des élèves) ; `ent-web-server` contre le vrai backend : `eleves_e9` **8** (rendu serveur des pages, liste/portée/filtres/pagination, famille et autre établissement refusés, synthèse et masquages, identité/historique réservés, élève protégé absent pour un tiers, contrat d'E8 entièrement traité), `dossier` 2, `demo_p1` 4, `entetes_securite` ; clippy `-D warnings` (natif et `wasm32`) : 0 ; formatage des paquets web et d'`ent-types` par les commandes de la CI (jamais `--all`).
- Aucun test ignoré, aucune dépendance nouvelle, aucune migration versée modifiée.

## 6. Parcours navigateur (clavier, 375 px, axe-core)
`docs/socles/02_ELEVE_E9_captures/parcours.mjs` (Chromium + Playwright, axe-core 4.10.2 WCAG 2.0/2.1 A et AA + bonnes pratiques, jeu de données créé par l'API de labo : 62 élèves, un protégé) : **28 vérifications OK, 0 échec, 0 violation axe** sur la liste, la liste vide, la fiche (synthèse, identité, historique), « fiche introuvable » et les deux pages à **375 px** (aucun débordement horizontal). Clavier : recherche → classe → statut → premier élève atteints par Tab ; « / » met le focus sur la recherche et reste un caractère pendant une saisie ; ouverture de la fiche par Entrée ; onglets par les boutons radio natifs. Captures et `rapport.json` dans `02_ELEVE_E9_captures/captures/`. **Non fait : test avec un lecteur d'écran** (FRONTEND_LEPTOS §6 : un audit automatique ne suffit pas pour déclarer l'écran accessible) ; mesures de performance à processeur ralenti non faites.
