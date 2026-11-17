# Socle 2 « l'élève » — lot E9 : suites d'E7-E8 (partie 1) et écrans E9a / E9b (partie 2) : livraison

Branche `ent-socle-2-code-e9` (base `ENT` 47a5be1e, qui n'a pas bougé : fusion sans objet, règle 28 relue). Conception : `docs/socles/02_ELEVE.md` §7.11, §11, §14, R-E9, ligne E9 du plan de lots ; décisions du pilote du 2026-11-17 (`02_ELEVE_E7_E8_LIVRAISON.md` §6). Niveau : **implémenté, testé en local** (CI GitHub à venir). Ni validé, ni prêt pilote.

## 1. Partie 1 — suites d'E7-E8 (backend)

| Point | Livré | Test |
|---|---|---|
| (a) catalogue E5 | `photo`, `email_eleve`, `mobile_eleve` au catalogue des champs (cibles `photo` / `coordonnee`, **non proposables par la file des propositions** : chacune a son circuit). Réglage par défaut : la famille **propose** (comportement conservé) ; fermée par un réglage `saisie_famille = non`, le refus est `403` pour la famille, jamais pour le secrétariat. `ReglageChamp::defaut_pour` | `e9a_photo_email_et_mobile…` ; `photo_et_coordonnees_propres_sont_au_catalogue…` (domaine) |
| (b) pastille « protocole » | domaine `scolarite`, action `contraintes.lire` (secrétariat, direction, vie scolaire, infirmerie ; ni professeur ni accueil), **sans aucun contenu de santé** (ni nature, ni échéance, ni date de début). Suit le module santé (M6, drapeau `module_sante`) ; masquée quand `pai_actif` est visible (un fait, une pastille). Le PAI reste réservé à `sante.lire` | `e9b_protocole_signale_l_infirmerie…` |
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

*(voir §5 pour l'état de validation)*

- **E9a** `/web/eleves` : recherche tolérante (pause de frappe 250 ms, une lettre ne cherche rien), filtres classe / statut / pastille (chips avec effectifs), pagination de 50, portée dite à l'écran (`perimetre` pour un professeur), plafond de recherche signalé, états vide / chargement / erreur, raccourci « / », pas de photo dans la liste (initiales). Route serveur ajoutée : `GET /eleves/liste` (même annuaire, même portée et mêmes protégés écartés que la recherche, filtres avant pagination ; personnel seulement).
- **E9b** `/web/eleves/:id` : en-tête carte (photo avec repli, désignation, statut, classe, pastilles), onglets **Synthèse** (une tuile par domaine ; masquée avec sa **raison**, jamais son contenu), **Identité** (groupes ouverts, masquages expliqués, identifiants avec statut), **Historique** (frise comparée à la version précédente, correction distinguée du changement d'état civil, consultations pour l'audit). Élève absent / autre établissement / protégé : **même écran « Fiche introuvable »**. Le dossier 360° historique passe à `/web/eleves/:id/dossier` ; les familles et élèves gardent le dossier sur `/web/eleves/:id`.
- Menu : entrée « Élèves » selon le moteur (`eleves.annuaire`), jamais pour les familles.
- Types : `ent-types::eleve_synthese` (`EleveLigneDto`, `EleveListeDto`, `EleveFicheDto`) ; un test d'`ent-api` garde l'identité du JSON de `EleveDto` et d'`EleveFicheDto`.

## 3. Compteurs (règle 28, base `ENT` 47a5be1e inchangée)
- **`Action::TOUTES`** : 253, **inchangé** (aucune action). **RLS** : 715, inchangé (aucune table). **Registre des données**, **`TABLES_ELEVE`**, **`CHAMPS_CHIFFRES`** : inchangés. **`SOURCES` / `NOMS_SOURCES`** : 63, inchangés (la route de liste est dans `eleve_synthese.rs`). **`MIGRATION_TAG`** : inchangé.
- **Catalogue des pastilles** : 8 → **13** (`CATALOGUE_INDICATEURS` = `ent_catalogue_indicateur`, test d'égalité vert). **Champs du catalogue E5** : 10 → **13**. `VERSION_CATALOGUE` (conservation) : sans objet.
- Événements : `scolarite.modalite_changee` écouté en plus par `e8.indicateurs`.

## 4. Écarts, hypothèses, questions pour le pilote
1. **F3 sans `NOT NULL`** : la reprise (`ent_eleve_scolarisation_reprendre`) et ses tests insèrent volontairement des lignes sans statut hors déclencheurs ; un `NOT NULL` les casserait. Le déclencheur garantit l'absence de NULL en fonctionnement normal. *Question : accepter, ou faire évoluer la reprise ?*
2. **Seuils [H]** : « arrivé en cours d'année » = après 14 jours ; M7 = 6 SMS par élève et 300 par établissement par 24 h ; « nouvel arrivant » = 30 jours ou première année. À valider avec le secrétariat.
3. **Qui voit quoi** : `nouvel_arrivant` et `arrive_en_cours_annee` suivent la tuile « scolarité » (secrétariat, direction, vie scolaire, accueil) : le **professeur principal ne les voit pas** (pas de tuile ouverte) ; `majorite_proche` suit la tuile « identité » (état civil : secrétariat, direction, infirmerie), pas le PP. Les ouvrir au PP demande une tuile « vie de classe » ou un droit dédié (décision de conception).
4. **Pastille « protocole »** : réservée à `contraintes.lire` (donc pas aux professeurs ni à l'accueil, à la différence de `sante.urgence_lire`). *Question : élargir aux professeurs de la classe ?*
5. **Réencodage** : « sans EXIF » est une **réécriture sans métadonnées**, pas un réencodage (aucune dépendance de décodage d'images, D-062) ; un JPEG CMJN ou sans perte est refusé.
6. **Lien mort préexistant** : l'action famille « Voir les propositions » de `ent-db/src/portail.rs` pointe `/web/eleves` (liste du personnel, `403` pour une famille) : à rediriger vers le portail famille (E9f).
7. **Dossier 360°** : son droit « famille » est toujours calculé côté client (note de la conception, §18) : non repris dans la fiche hub (aucun droit calculé), mais non corrigé dans le dossier lui-même.
8. **PILOTAGE.md** : ce dépôt ne contient que les règles 1 à 9 ; j'ai appliqué les règles 17, 25 et 27 à 35 d'après ce que les rapports précédents en disent (aucune migration modifiée, compteurs recalculés, pas d'heure réelle, PR en brouillon).
9. **Non fait** (hors lot) : ELV-44/46 (lot dédié à venir), E9c à E9f, actions d'écriture de la fiche (modifier l'identité), tuile scolarité détaillée, désambiguïsation R-E8, vues enregistrées, export.

## 5. Tests
Local, PostgreSQL 16, Rust **1.99.0**, `--test-threads=1` :
- `ent-db` (`tests_eleve_*`, `tests_d036`, `tests_rls*`) : **164 verts** (dont `tests_eleve_e9` : 15) ; `domain-eleve` : 301 verts ; `ent-api` : lib `eleve_synthese` 2 verts, `separation` **55 verts** (+2 cas pour la liste).
- Clippy 1.99 `-D warnings` et formatage : voir la fin du rapport (mise à jour à chaque PR).
- Aucun test ignoré, aucune dépendance nouvelle, aucune migration versée modifiée ; `Cargo.lock` inchangé (`--locked`) ; jamais `cargo fmt --all` (le backend n'est pas formaté : seuls les fichiers nouveaux l'ont été).
