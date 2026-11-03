# Frontend ENT — direction artistique, architecture cible, tranche 1

Date : 2026-11-03 — Statut : **proposition ; tranche 1 implémentée et testée en local** (voir § 7 ; non validée pilote, non poussée). Les § 1 à 6 sont conservés tels que proposés.
Maquettes : canevas « Direction artistique ScolIA » (fondations, connexion, accueil Bento, appel ordinateur interactif, appel téléphone).
Décisions conservées : Leptos 0.8 (D-010), backend axum existant, `ent-types` (D-015/D-016), passkeys (D-009), sessions web (D-011), un sous-domaine par établissement (D-012).

## 1. Étude de `emilkowalski/skills` (consulté le 2026-11-03)

Dépôt MIT, actif, 17 « skills » texte (aucun code à reprendre). Lus : `apple-design`, `emil-design-eng`, `animate`, `animation-performance`, `break-ui`, `mobile-native`, `review-animations`, `pick-ui-library`, aide-mémoire UI et performance.

| Ce que le dépôt apporte | Ce qu'on en retient pour l'ENT |
|---|---|
| Principes Apple (WWDC) : réponse immédiate, hiérarchie, familiarité, « simplicité ≠ minimalisme », artisanat | Retour **au moment de l'appui** (`:active` scale 0,97) ; un seul élément principal par écran ; libellés précis (« Appel », pas « Outils ») |
| Cadre de décision des animations : fréquence d'usage, but, courbe, durée | **Aucune animation** sur ce qui est répété 100 fois par jour (statut d'appel, raccourcis clavier) ; 120–240 ms ; `ease-out` fort ; seulement `transform`/`opacity` ; mouvement réduit respecté |
| Détails invisibles : `tabular-nums`, interlettrage négatif des titres, 65 caractères par ligne, ombres semi-transparentes, menus qui partent du déclencheur | Intégrés aux jetons (§2) |
| `break-ui` : données extrêmes (noms longs, composés, accents, 0 / 1 / 1 000 éléments) | Grille de tests visuels obligatoire pour chaque écran (noms d'élèves composés, classes de 400, listes vides) |
| `mobile-native` : `100dvh`, champs à 16 px, survol limité aux souris, zones sûres | Règles CSS de base de la coquille |
| Bibliothèques React (Base UI, Sonner, Motion) | **Non applicables** (Leptos) : seuls les principes sont repris, réalisés en CSS et en Rust |

**Le dépôt ne traite ni du Bento, ni des palettes, ni des grilles.** La direction artistique ci-dessous est donc la nôtre, construite sur ses principes.

## 2. Direction artistique (voir l'artboard « Fondations »)

- **Personnalité** : calme, net, rapide. Le contenu d'abord, l'interface s'efface.
- **Couleurs** : fond `#F5F5F7`, surfaces blanches, encre `#1D1D1F` / `#5E5E63`, **un seul accent** (bleu ScolIA `#0B5FD7`). États métier : présent `#1A7F37`, absent `#C5221F`, retard `#8A5100`, justifié `#5E35B1`, chacun sur un fond teinté, **toujours avec un libellé**. Thème sombre défini dès maintenant (jetons), activé plus tard.
- **Typographie** : police système (SF Pro, Segoe UI Variable, Roboto) : rien à télécharger. Échelle 34 / 28 / 22 / 17 / 15 / 13 / 12, interlettrage négatif au-dessus de 20 px, chiffres tabulaires dans les tableaux.
- **Espacement** : base 4 (4, 8, 12, 16, 24, 32, 56). **Rayons** : 6 pastille, 10 contrôle, 14 carte, 22 tuile Bento. **Bordures** : filet 8 % de noir. **Ombres** : 3 niveaux semi-transparents.
- **Bento, avec mesure** : grille 12 colonnes, gouttière 16, tuiles de 4 / 6 / 8 / 12 colonnes, **une information et une action par tuile**. Pour les tableaux de bord : accueil par profil, synthèse élève, vie scolaire. **Jamais** pour la saisie (appel, notes) : là, tableaux denses.
- **Tableaux métier** : lignes de 44–48 px (32 en mode compact), en-tête collant, contrôle segmenté pour les statuts, actions groupées, barre d'action collante en bas.
- **Navigation** : ordinateur ≥ 1200 px : barre latérale 248 px ; 768–1199 px et écrans de saisie : rail d'icônes 72 px ; téléphone : barre d'onglets en bas (4 entrées + « Plus »). Même liste d'entrées, filtrée par les droits.
- **Responsive** : CSS seulement (requêtes média et conteneurs), pas de détection d'appareil ; cibles ≥ 44 px au tactile.

## 3. Architecture cible du frontend (`ent-rust/web`)

```
crates/
  ent-web-ui       Design system : jetons CSS, composants génériques, AUCUNE notion métier
                   + à ajouter : Coquille, BarreLaterale/Rail/BarreOnglets, EnTetePage, GrilleBento/Tuile,
                     Tableau dense, Segmente, Avatar (initiales robustes), Statistique, Icone (SVG en ligne)
  ent-web-api      Client HTTP + un service par domaine (auth, edt, appel, eleves…) sur ent-types ;
                   transport unique, erreurs typées ; aucune vue
  ent-web-app      Application : routage, coquilles, session, navigation, droits, notifications, erreurs
    src/noyau/     session.rs, droits.rs, navigation.rs, erreurs.rs, coquille.rs
    src/modules/   un dossier par domaine : accueil/, appel/ (puis edt/, eleves/…)
                   chaque module : mod.rs (routes + entrées de navigation), pages/, composants/, modele.rs (logique pure testée)
  ent-web-server   Rendu serveur, fichiers, composition avec ent-api (inchangé)
  ent-web-session  Prototype cookie isolé (non branché)
```

Règles qui évitent la duplication :
1. Un module dépend de `ent-web-ui`, `ent-web-api` et du `noyau`, **jamais d'un autre module**. Ce qui est partagé (étiquette élève, sélecteur de classe) monte dans `ent-web-app/src/composants_metier/`.
2. Tout composant sans connaissance métier va dans `ent-web-ui`. Le CSS d'un module vit avec lui : `ent-web.css` ne garde que la coquille.
3. **Le backend reste l'autorité des droits.** Le frontend ne s'en sert que pour masquer ce qui serait refusé (`Capacites` déduites du rôle), et chaque 403 est traité. Un test compare cette correspondance aux règles d'`ent-auth` pour tous les rôles, sans modifier `ent-auth`.
4. **Session** : un contexte unique (`Moi` chargé une seule fois par la coquille, et non par chaque page). Sur un 401 n'importe où : la session est effacée, retour à la connexion avec l'adresse de départ. Les autres onglets sont prévenus.
5. **Erreurs** : 401 → reconnexion ; 403 → état « accès refusé » ; 404 → introuvable ; réseau → bandeau avec « Réessayer » ; validation → message sous le champ. Les notifications passent par le fournisseur existant.
6. **Routage** : `/web/connexion` (coquille publique), puis `/web` (accueil selon le profil), `/web/appel` et `/web/appel/{creneau}/{jour}` dans la coquille authentifiée (route parente avec `Outlet`).
7. **Temps de compilation** : `.into_any()` aux frontières de route. Les modules seront découpés en crates seulement quand la compilation le justifiera (mesure à l'appui).

## 4. Problèmes de l'architecture actuelle

| # | Constat (vérifié) | Conséquence | Traitement proposé |
|---|---|---|---|
| 1 | Jeton en `localStorage`, données chargées dans le navigateur | exposition au XSS ; le rendu serveur n'a pas de données | tranche 1 : rester en Bearer derrière une abstraction `Session` ; la bascule vers le cookie (prototype prêt) devient un changement localisé, **après décision** |
| 2 | Pas de coquille : chaque page charge `/auth/me` et a son propre en-tête | requêtes en double, navigation incohérente | coquille + contexte de session |
| 3 | Pas de modèle de droits côté frontend, navigation codée en dur | menus à maintenir à la main | registre de navigation + `Capacites` |
| 4 | `ent-web.css` mélange le générique et le spécifique à l'appel (`.feuille`, `.lot`, `.liste`) | duplication dès le 2e module | composants `ent-web-ui` + CSS par module |
| 5 | Design system sans mise en page, tableau, contrôle segmenté, icônes ; jetons incomplets (pas de thème sombre, ni de mouvement, ni d'échelle typographique) | chaque écran réinvente | nouveaux jetons (§2) et composants (§3) |
| 6 | `ent-web-api` : un seul fichier, plus des appels génériques `get_json` | grossira avec chaque module | un service par domaine |
| 7 | `recursion_limit = 512` déjà nécessaire | compilation plus lente avec les modules | frontières `.into_any()`, mesure du temps de compilation |
| 8 | Backend : **aucune route en lecture des appels du jour**, ordre des lignes d'appel non garanti, refus « clos ou introuvable » ambigu (404) | tuile « Appels du jour » impossible sans backend ; ordre affiché instable | tri côté frontend ; petite route en lecture **à autoriser** |

## 5. Tranche 1 — expérience complète « enseignant fait l'appel »

| Étape | Contenu | Backend utilisé (existant) | Fini quand |
|---|---|---|---|
| 1. Jetons et composants | jetons §2 (clair + sombre), Coquille, navigation responsive, EnTetePage, GrilleBento/Tuile, Tableau, Segmente, Avatar, Statistique, Icone | — | tests de rendu ; page `/web/_composants` mise à jour ; axe-core 0 violation |
| 2. Noyau | contexte de session, gestion du 401, `Capacites` + test de parité avec `ent-auth`, registre de navigation, erreurs | `/auth/me`, `/auth/logout` | tests unitaires ; 401 et 403 vérifiés contre le vrai backend |
| 3. Connexion | écran de la maquette : mot de passe, second facteur passkey, passkey seule | `/auth/login`, `/auth/passkey/login/*` | essai Chrome avec passkey virtuelle |
| 4. Accueil enseignant | Bento : prochain cours + « Faire l'appel », séances du jour, messages, annonces | `/edt/mes-creneaux`, `/messages/fils`, `/annonces` | données réelles du labo ; tuile « Appels du jour » **seulement si** la route en lecture est autorisée |
| 5. Appel finalisé | écran de la maquette : compteurs, filtres, recherche, contrôle segmenté, raccourcis P/A/R, actions groupées (correctif conservé), clôture confirmée ; version téléphone | `/appels`, `/appels/{id}`, `/appels/{id}/lignes`, `/appels/{id}/clore` | règles métier **inchangées** ; tests du modèle conservés ; refus 403 et 404 affichés |
| 6. Validation | tests contre le vrai backend, Chrome (ordinateur et 375 px), clavier, axe-core, données extrêmes (`break-ui`), CI | — | tout vert, bilan présenté |

Hors tranche 1 : accueils famille / élève / direction (même coquille, autres tuiles), cookie de session, nouvelles routes backend non autorisées, retrait de l'ancienne interface `/app`.

## 6. Existant réutilisé

| Élément | Devient |
|---|---|
| `ent-web-ui` : Bouton, ChampTexte et autres champs, Alerte, Dialogue / Confirmation, Notifications, états (Chargement, Squelette, Vide, Erreur, AccèsRefusé) | conservés, restylés par les nouveaux jetons |
| `ent-web-app/src/appel/modele.rs` (statuts, compteurs, filtres, actions groupées, tests) | conservé tel quel, logique de l'écran final |
| `appel/feuille.rs`, `appel/mod.rs` | refondus dans `modules/appel/` avec les nouveaux composants |
| `connexion.rs`, `entete.rs`, `session.rs` | remplacés par l'écran de connexion et le noyau (session, coquille) |
| `ent-web-api` + `ent-types` | découpés en services, mêmes types |
| `ent-web-server` (rendu serveur, précompression, en-têtes) et ses tests contre le backend | conservés, étendus |
| `ent-web-session` | réservé à la bascule cookie, après décision |

## 7. État d'avancement de la tranche 1 (2026-11-03)

Niveau : **implémenté + testé en local**. Pas validé pilote, pas prêt production, rien poussé.

| Étape du § 5 | État |
|---|---|
| 1. Jetons et composants | fait : avatar, coquille, icône, mise en page, segmenté, bouton Sombre ; feuille `ent-web.css` sur les jetons de `ui.css`. Thème sombre **incomplet** (jetons de statut) |
| 2. Noyau | fait : `noyau/` (session, droits, navigation, coquille, erreurs, temps) ; parité des droits testée (`droits_parite`, 12 tests) |
| 3. Connexion | fait : mot de passe + passkey (`modules/connexion/webauthn.rs`). Parcours WebAuthn **non testé au navigateur** (pas d'authentificateur) ; la passkey virtuelle prévue n'a pas été utilisée |
| 4. Accueil enseignant | fait : prochain cours, séances du jour, tuiles messages non lus et annonces. Tuile « Appels du jour » **non faite** (route en lecture non autorisée, § 4 point 8) |
| 5. Appel finalisé | fait : statuts, clavier, sélection multiple, confirmation, clôture, verrouillage, mobile. Commentaire et « dispensé » non gérés par le backend |
| 6. Validation | tests verts (ui 14, api 5, app 28, server 24, session 9 ; clippy 0), essai au navigateur intégré (ordinateur, 375 px, 400 élèves). axe-core, lecteur d'écran, Firefox / Safari **non refaits** |

Écarts à la proposition :
- Décision de Florian (2026-11-03) : messages et annonces = tuiles seulement ; la messagerie complète est un module ultérieur ; le lien « Ouvrir la messagerie » mène à l'ancienne UI `/app`.
- Ajout : emploi du temps semaine (`modules/edt/`), non listé au § 5.
- La consultation d'une feuille d'appel passe par `POST /appels` (écriture), faute de route en lecture.
- Sécurité ajoutée : CSP à nonce sur `/web` et `/pkg`, durcissement de l'adresse de retour, synchro d'onglets (voir `SECURITY.md`).

Limites restantes : `PROJECT_STATUS.md`, section « Tranche 1 ».
