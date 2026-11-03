# Design system Leptos — fondations (proposition)

Date : 2026-11-02 (mise à jour 2026-11-03) — Statut : **étapes 1 et 2 commencées** (fondations réalisées, voir §7) ; tableaux, recherche avancée et navigation non commencés.
Complète `docs/DESIGN_SYSTEM.md` (principes) et `docs/FRONTEND_LEPTOS.md` (conventions).

## 1. Point de départ : ce qui existe dans `ent-web-ui`

| Composant | État | À reprendre |
|---|---|---|
| `Bouton` (primaire, secondaire, discret, danger) | fait, natif, `type="button"` par défaut | icône, état « en cours », taille compacte |
| `Alerte` (info, succès, avertissement, erreur) | fait ; `role="alert"` / `status` | bouton de fermeture optionnel |
| `Squelette` | fait, annoncé une fois | variantes ligne / carte / tableau |
| `EtatVide` | fait | illustration facultative, action principale |
| `Annonce` (zone `aria-live`) | fait | file de messages (plusieurs annonces rapprochées) |
| `Dialogue` (`<dialog>` natif) | fait : focus, Échap, retour du focus | `Confirmation` prête à l'emploi, taille, défilement interne |
| `FiltrePastille` (`aria-pressed`) | fait | regroupement dans `Filtres` |
| Jetons CSS | `web/style/ent-web.css` (couleurs, rayons, ombre, focus) repris du thème existant | à extraire en couche « jetons » documentée |

Style : **CSS simple à base de variables** (aucun outil de build), classes préfixées `ui-`. Choix à confirmer (§6).

## 2. Architecture

```
ent-web-ui/src/
├── lib.rs              réexports
├── jetons/             (CSS) couleurs, typographie, espacements, rayons, ombres, focus, points de rupture
├── base/               Bouton, Lien, Icone (SVG en ligne), Badge, Spinner
├── mise_en_page/       Page, EnTetePage, Carte, Pile, Grille, BarreActions (pied collant)
├── retours/            Alerte, Annonce, Notification (toasts), Squelette, EtatVide, EtatErreur, AccesRefuse
├── superposition/      Dialogue, Confirmation, Tiroir (panneau latéral), Infobulle
├── formulaire/         Formulaire, Champ, ChampTexte, ZoneTexte, Selection, Case, GroupeRadio,
│                       GroupeSegmente, ChampDate, ChampFichier, ResumeErreurs, validation
├── recherche/          ChampRecherche (délai de frappe), Filtres, PastillesActives
├── donnees/            Tableau<T> (colonnes typées, tri, sélection, actions groupées),
│                       Pagination, Virtualisation, ListeCartes (petits écrans)
└── navigation/         BarreApp, MenuLateral, Onglets, FilAriane, MenuDeroulant (bouton menu)
```

Règles de construction (rappel de `FRONTEND_LEPTOS.md`) :
- éléments natifs d'abord ; motifs ARIA de l'APG (W3C) quand le natif ne suffit pas (onglets, menu, combobox) ;
- aucun appel réseau ni règle métier dans `ent-web-ui` ; les composants reçoivent des signaux et des callbacks ;
- chaque composant gère explicitement ses états : désactivé, chargement, erreur, vide ;
- responsive par défaut (testé à 375 px) ; animations courtes, coupées si `prefers-reduced-motion`.

### Tableau (le composant le plus structurant)
- `Tableau<T>` générique : définition des colonnes (titre, rendu de cellule, clé de tri), `<table>` natif, `<th scope>`, `aria-sort` sur la colonne triée.
- **Tri et pagination côté serveur par défaut** (les listes de l'ENT dépassent vite la centaine de lignes) ; tri local seulement pour les petites listes déjà chargées.
- Sélection (cases + « tout sélectionner (affichés) »), actions groupées **limitées aux lignes affichées** (règle issue du prototype).
- **Virtualisation** seulement au-delà de quelques centaines de lignes affichées en même temps (mesure du prototype : 400 lignes, processeur ralenti ×4 → 0,4 à 0,6 s pour recréer la liste) ; sinon pagination.
- Petits écrans : bascule en cartes (`ListeCartes`), comme le prototype.

### Formulaires et validation
- `Champ` = étiquette + contrôle + aide + message d'erreur reliés (`aria-describedby`, `aria-invalid`).
- Validation **côté navigateur pour le confort** (champs requis, formats), **le serveur reste l'autorité** : les erreurs de l'API sont rattachées aux champs concernés.
- `ResumeErreurs` en haut du formulaire après envoi (liens vers les champs, focus déplacé dessus).
- Types de formulaires partagés via `ent-types` (proposition `ENT_TYPES_PROPOSITION.md`) pour ne pas redéfinir les champs.

### Notifications
- `Notification` (toast) pour les confirmations brèves, **doublée** d'une annonce `aria-live` ; jamais pour une erreur bloquante (→ `Alerte` dans la page).
- Disparition automatique désactivable, jamais pendant que le focus est dessus.

## 3. Ordre de réalisation proposé

| Étape | Contenu | Raison |
|---|---|---|
| 1 | Jetons + base + mise en page | tout le reste en dépend |
| 2 | Retours (états, alertes, notifications) + superpositions (`Confirmation`, `Tiroir`) | utilisés par tous les écrans |
| 3 | Formulaires + validation | nécessaires à presque tous les modules et à l'édition dans les tableaux |
| 4 | Recherche + filtres | entrée des listes |
| 5 | Tableau (tri, pagination serveur, sélection, actions) puis virtualisation si une mesure l'exige | composant le plus complexe, s'appuie sur 1 à 4 |
| 6 | Navigation (barre, menu latéral, onglets, fil d'Ariane, menu déroulant) | utile dès qu'il y a plusieurs modules |

Chaque étape est livrée avec ses tests (§4) et migrée dans le prototype d'appel quand c'est pertinent (le prototype sert de banc d'essai). Aucun nouveau module métier tant que les étapes 1 à 3 ne sont pas faites.

## 4. Vérification de chaque composant

1. **Page de démonstration** réservée au développement (`/web/_composants`) : chaque composant dans tous ses états — sert à la relecture visuelle et aux tests d'accessibilité.
2. **Tests de rendu serveur** (natifs) : attributs ARIA, étiquettes, rôles.
3. **Tests navigateur** (Chrome sans fenêtre, comme les mesures du prototype) : clavier (Tab, Échap, flèches), focus, axe-core sans violation, 375 px sans défilement horizontal.
4. Lecteur d'écran : voir `docs/PLAN_VALIDATION_FRONTEND.md`.

## 5. Accessibilité et responsive (exigences communes)

- Contraste ≥ 4,5:1 (texte), ≥ 3:1 (éléments d'interface) ; focus toujours visible.
- Cibles tactiles ≥ 44 px sur mobile (le prototype est à 42 px : à corriger dans les jetons).
- Tout est utilisable au clavier ; ordre de tabulation logique ; pas de piège à focus hors dialogues.
- Pas d'information portée par la couleur seule (statuts : couleur + texte).
- Textes en français, langue déclarée ; formats de date et nombres français.

## 6. Décisions à prendre

1. **Style** : rester en CSS simple à base de variables (choix du prototype : pas d'outil, rendu prévisible) ou revenir à Tailwind v4 (utilisé par l'ancienne interface) ?
2. **Icônes** : jeu d'icônes libre à choisir (SVG en ligne, licence à vérifier) ou icônes dessinées en interne ?
3. **Thème sombre** : dès maintenant (les jetons le permettent) ou plus tard ?
4. Référence `emilkowalski/skills` citée dans `DESIGN_SYSTEM.md` : à étudier sélectivement (licence et pertinence pour Leptos) avant usage.

## 7. Fondations réalisées (2026-11-03)

Architecture CSS : `ent-rust/web/crates/ent-web-ui/style/ui.css` (1. jetons, 2. base, 3. composants `ui-*`), importée par `ent-rust/web/style/ent-web.css` (styles de l'application) ; cargo-leptos fusionne les `@import` en **un seul fichier** (`style-bundle = true`). Choix « CSS simple » appliqué (décision §6.1 implicite de la mission ; Tailwind non retenu à ce stade).

| Famille | Composants (`ent-web-ui`) | Accessibilité |
|---|---|---|
| Jetons | typographie, couleurs (contraste ≥ 4,5:1), espacements, rayons, ombres, focus, cible tactile 44 px, durées, z-index | `prefers-reduced-motion` respecté |
| Boutons | `Bouton` (4 variantes, 2 tailles, désactivé, **en cours** `aria-busy`), `LienBouton` | natif, `type="button"` par défaut |
| Champs | `ChampTexte`, `ZoneTexte`, `Selection`, `Case`, `GroupeChoix`, `ResumeErreurs` | étiquette visible, aide et erreur reliées (`aria-describedby`), `aria-invalid`, « obligatoire » annoncé |
| Alertes | `Alerte` (4 niveaux, fermable), `Annonce` (`aria-live`) | `role="alert"` pour les erreurs, `status` sinon |
| Dialogues | `Dialogue` (`<dialog>` natif), `Confirmation` (« Annuler » en premier, variante danger) | focus, Échap, retour du focus par le navigateur |
| Notifications | `fournir_notifications`, `use_notifications`, `ZoneNotifications` (succès, info, fermeture, disparition après 6 s) | zone `role="status"` `aria-live="polite"` |
| États | `Chargement`, `Squelette`, `EtatVide`, `EtatErreur` (avec « Réessayer »), `AccesRefuse` | textes toujours présents |

Vérification :
- 9 tests de rendu (attributs ARIA, états) dans `ent-web-ui` ;
- page de démonstration `/web/_composants` (tous les composants et leurs états) ;
- la page de connexion du prototype utilise désormais `ChampTexte` et `Bouton` (`en_cours`) ;
- audit axe-core et parcours clavier de la page de démonstration : voir le bilan de la mission.

Non commencés (volontairement) : tableaux, pagination, virtualisation, recherche avancée, navigation, menus.
