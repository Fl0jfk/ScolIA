# Socle 12 — lots AS14d et AS14e (écrans web) : dossier d'assiduité d'un élève, signalement

**Date :** 2026-11-16 — **Branche :** `ent-socle-12-code-as14de` (depuis `origin/ENT`) — **Niveau :** implémenté + testé en local (PostgreSQL 16, Chromium) ; CI : voir le journal du pilote ; **non validé par Florian, pas prêt pilote.**
**Conception :** `12_ASSIDUITE.md` §13.2 à §13.6 (décompte, dossier annuel R131-6, seuils, signalement), §24.1 (ligne AS14 : AS14d, AS14e), R-AS16, R-AS26, CA-40 (sur `origin/ent-mc0hoe`).
**Aucune migration. Aucune route ni aucun code backend ajouté** (seuls les DTO d'`ent-types` ont reçu des champs `#[serde(default)]`). Aucune nouvelle action (`TOUTES` inchangé), aucune nouvelle source de routes, aucune route pour la matrice de séparation. Règle 28 relue : aucun compteur touché.

## 1. Ce qui est fait

### AS14d — dossier d'assiduité d'un élève (`/web/assiduite/eleves/:id`, module `assiduite_vs::dossier`)
Accès : onglet « Dossier d'un élève » de `/web/assiduite` (recherche par nom), lien « Ouvrir le dossier » du détail d'un épisode, lien « Dossier » des signalements. Année scolaire (1er sept. – 31 août).
- **Résumé** : demi-journées comptées et non justifiées de l'année, dernier mois et sa position par rapport au seuil (« Seuil atteint », « Proche du seuil », « Sous le seuil » : libellé complet, pas seulement une couleur), épisodes à qualifier, rectifications qui touchent un signalement, données non affichables dites (seuil illisible, droits).
- **Absences** : épisodes de l'année (retirés en dernier), détail réutilisé d'AS14b (séances, textes, justificatifs, propositions des AED, historique, avis aux familles), **qualification individuelle** (nature L131-8, libellé famille cohérent, note interne), et **requalification** (nouveau) : motif codé obligatoire (5 motifs du serveur), version lue, conflit traduit, aucune requalification vers la même valeur.
- **Décompte** : demi-journées regroupées par mois, **chaque ligne expliquée** (phrase du serveur tirée des champs codés, jamais le motif), état « compte, non justifiée » en toutes lettres, **rectifications postérieures à l'arrêt du mois** (avant → après, cause, « touche un signalement », « change la position par rapport au seuil »).
- **Contacts et courriers** (`assiduite.suivre`) et avis aux familles ; section refusée = message d'information.
- **Chronologie** unique (absences, contacts, courriers, rectifications, signalements), du plus récent au plus ancien.

### AS14e — signalements (`/web/signalements`, module `assiduite_vs::signalements`, remplace l'ancien écran de `discipline_vs`)
- **Élèves à suivre** du mois (≥ 1 demi-journée non justifiée), ceux au seuil d'abord : « n sur seuil », position, état du signalement, lien vers le dossier ; dit si le mois est arrêté (chiffres figés) ou non (provisoires).
- **Préparation** : « Préparer les signalements du mois » (CPE, `assiduite.signaler`), **avec confirmation** ; idempotent ; rien n'est envoyé.
- **Relecture** : dossier pré-rempli (demi-journées retenues, figées à la proposition) dans un `<details>` ; lien vers le dossier complet.
- **Suivi** : Valider / Refuser (direction, **`discipline.gerer`** : l'ancien écran proposait les boutons selon `assiduite.signaler`, ce que le serveur refusait), « Noter comme transmis » (libellé corrigé : ScolIA n'envoie rien à l'académie, R-AS26), confirmation à chaque décision. Sans le droit : « La décision revient à la direction ».
- **Rectifications après arrêt du mois** qui touchent un signalement (CA-40) : phrase « passe de 5 à 3 », badge si le seuil change de côté, « Marquer comme vue ».
- **Ce qui manque est dit à l'écran** (§2, `MANQUE_AS9`).

### Transverse
- `ent-types` : `DemiJourneeDto.raison` / `explication`, `DecompteMoisDto.arrete` (`serde(default)`), `ContactDto`, `CourrierDto`, `RectificationDto`, `RequalifierEpisodeBody`. `ent-web-api` : `requalifier_episode`, `contacts_eleve`, `courriers_eleve`, `rectifications_mois`, `rectification_vue`.
- Modèle pur testé en natif : `dossier_modele.rs` (année scolaire, regroupement par mois, seuil, requalification, chronologie, libellés, dossier de signalement), `signalements.rs`.
- `FRONTEND_LEPTOS.md` §17 ; CSS (`asd-mois`, `asd-chrono`…). Les signalements quittent `discipline_vs` (un module ne dépend pas d'un autre).

## 2. Ce qui n'est pas fait, et ce que ScolIA ne fait pas encore (à ne pas masquer)
**Signalement (AS9, backend non fait : rien n'est inventé)** : (1) dossier annuel d'événements (entretiens, commission, référent, engagements) ; (2) validation avec ré-authentification (passkey) : un clic de confirmation aujourd'hui ; (3) export PDF/CSV par profil académique ; (4) trace de la transmission (date, outil, personne) : « Noter comme transmis » n'enregistre que l'état ; (5) profils académiques et niveaux (persistance, âge du niveau pénal). Aussi : délégation de la validation (AS-229).
Autres limites :
1. **Année scolaire** calculée par l'écran (1/9 – 31/8), hypothèse ; l'année de l'établissement est au socle 5.
2. **Rectifications** : la route renvoie celles du mois pour tout l'établissement (droit `assiduite.signaler`, CPE et direction) ; l'écran du dossier filtre l'élève. À remplacer par un filtre serveur si le volume l'exige (je n'ai pas modifié le backend).
3. Les rectifications ne sont lues que pour les mois terminés où l'élève a des absences, une requête par mois (jusqu'à 12) ; les signalements, pour les mois où le seuil est atteint.
4. Le secrétariat a `assiduite.suivre` par défaut : il voit les contacts (sans motif médical). Aucun rôle par défaut n'a `absences.lire_motif` sans `assiduite.suivre`, la branche « contacts non accessibles » n'est donc jouée qu'en test de modèle.
5. Le détail d'un épisode ouvert depuis le dossier s'intitule « Épisode » (le nom est dans l'en-tête).
6. Un épisode sans séance (ancien circuit, « 14 novembre » dans les données de labo) s'affiche « 0 séance(s) » : repris tel que le serveur le rend.
7. **Lecteur d'écran** non testé avec un vrai lecteur (axe-core ne suffit pas, `FRONTEND_LEPTOS.md` §6). Le champ « Mois » est un `<input type=month>` natif (affichage dans la langue du navigateur).
8. Pas de mobile (socle 11) : les dossiers complets restent sur le web.
9. Message du pilote (tâche `as6_eleve_exclu_de_cours_non_arrive…`, changement de jour à Paris) : **non traitée dans cette session**, hors de ma mission (règle 1) ; je l'ai signalé au pilote. Relevé utile : mon propre script de navigateur a eu le même défaut (jour de semaine UTC au lieu de Paris entre 22 h et minuit UTC), corrigé.

## 3. Vérifications (locales, 2026-11-16)
- **rustc / clippy 1.99.0, `-D warnings`, zéro avertissement** : frontend ssr (`--all-targets`), serveur avec et sans labo, wasm `hydrate`, `ent-types`. Formatage : `cargo fmt -p …` (jamais `--all`), puis la commande exacte de la CI en `--check` : conforme.
- **Tests natifs** : `ent-web-app` (dont 27 nouveaux dans `dossier_modele` et `signalements`), `ent-web-api`, `ent-web-ui`.
- **Contre le vrai backend** (`tests/assiduite_as14de.rs`, inclus d'office par la CI) : 3 tests. *Pages* : rendues, CSP, aucun style en ligne. *AS14d* : pointage → épisode → décompte expliqué (raison et explication non vides, jamais le motif) ; la famille dépose un justificatif **médical** : le CPE le lit, **le secrétariat ne le reçoit pas et son détail d'épisode ne contient pas le texte** ; qualification, requalification (version périmée = conflit, motif hors liste refusé, historique complet) ; professeur et famille refusés (requalification, contacts, courriers, rectifications) ; **même réponse pour « interdit » et « inexistant »** ; mois non identifié par son 1er jour refusé ; **autre établissement** : fiche, demi-journées, contacts, courriers, épisode, justificatifs refusés, listes vides. *AS14e* : quatre absences non justifiées dans un mois terminé → seuil atteint, proposition idempotente (un signalement par élève et par mois), dossier pré-rempli de ≥ 4 lignes, décision (professeur, famille refusés ; « transmis » sans validation refusé ; rejeu = conflit ; action inconnue refusée), arrêt du mois par la direction seule, **mois arrêté jamais réécrit**, requalification postérieure → **rectification qui touche le signalement**, « vue », « transmis », cloisonnement (autre établissement : ni décompte, ni signalement, ni rectification, décision refusée). Aucun `#[ignore]`.
- **Navigateur** (Chromium + Playwright, vrai serveur, données de labo, `parcours.mjs`) : le CPE ouvre le dossier d'Hermione par la recherche, qualifie puis requalifie (message d'erreur sans motif, puis succès), le **décompte se met à jour (4 → 3 non justifiées, vérifié aussi après rechargement)**, contacts, chronologie ; **clavier** : flèches entre les cinq sections ; signalements : élèves au seuil, relecture, confirmation de la proposition ; la direction valide (« Validé, à transmettre », « n'envoie rien à l'académie ») ; le CPE ne décide pas ; le secrétariat ne voit aucun texte médical ; le professeur ne voit rien du dossier. **axe-core (wcag2a/aa, 21, best-practice) : 0 violation sur 16 vues**, dont **375 px** (dossier : résumé, absences, décompte ; signalements) **sans aucun débordement horizontal**. Erreurs de console : seulement des 403/404 attendus (professeur sans droit, secrétariat sans `assiduite.signaler` sur les rectifications, dits à l'écran).
- Captures : `docs/socles/ASSIDUITE_AS14_DE_captures/` (01–07 dossier, 10–14 signalements, 20–22 droits, 30–33 en 375 px) ; `rapport.json` ; `parcours.mjs` (prérequis : `npm install axe-core@4.10.2` à côté).
- Environnement : PostgreSQL 16 local, `ENT_ENV=lab` ; build `cargo leptos build` (dev, sans wasm-opt).
- CI : une seule exécution (`workflow_dispatch` `ent-rust.yml`) après fusion d'`origin/ENT` (règle 28) ; numéro de run communiqué au pilote.
