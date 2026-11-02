# Prototype Leptos — module d'appel

Date : 2026-11-02 — Branche : `ENT` — Décisions de référence : D-008 (Leptos), D-009 (auth `ent-auth` conservée).
Statut : **prototype testé contre le vrai backend Rust, sur données de labo fictives**. Rien n'est commité.

## 1. Résumé

| Question | Réponse |
|---|---|
| L'écran d'appel fonctionne-t-il avec Leptos contre le vrai `ent-api` ? | **Oui** : connexion, choix de séance, saisie, validation, clôture ; absences créées côté vie scolaire (vérifié en base). |
| Le backend a-t-il été modifié ? | **Non.** `ent-api`, `ent-auth`, le workspace `ent-rust/Cargo.toml` et `ui.rs` sont identiques à `HEAD` (vérifié par `git diff` et empreinte de `ui.rs`). |
| Permissions respectées ? | **Oui**, appliquées par `ent-api` : famille refusée, autre professeur refusé, CPE autorisée. |
| Performances | Page interactive en ~120 ms en local ; clic → affichage ≈ 17 ms (une image) pour 38 et 400 élèves ; 400 élèves avec processeur ralenti ×4 : affichage 573 ms, clic médian 37 ms. |
| Poids envoyé au navigateur | ~1,05 Mo brut, **~325 Ko compressés (brotli)** au premier chargement, puis 0,5 à 41 Ko par appel API. |
| Accessibilité | axe-core : **0 violation** WCAG 2.2 A/AA sur les pages d'appel et de connexion. **Non validé** avec un vrai lecteur d'écran ni audit RGAA. |
| Verdict | **Leptos validé techniquement**, avec des réserves non bloquantes listées en §10. |

## 2. Périmètre isolé

Tout le prototype est dans un **workspace Cargo séparé** : `ent-rust/web/` (non suivi par Git pour l'instant). Il dépend d'`ent-api` par chemin, sans toucher au manifeste ni au code du workspace principal.

```
ent-rust/web/
├── Cargo.toml              versions épinglées + configuration cargo-leptos
├── scripts/env.sh          environnement (à sourcer) — rien de global
├── scripts/pg.sh           PostgreSQL portable : start | stop | status | psql | reset
├── style/ent-web.css       feuille unique, jetons repris du thème existant
└── crates/
    ├── ent-web-ui/         composants réutilisables (bouton, alerte, dialogue <dialog>, squelette, état vide, annonce, pastille-filtre)
    ├── ent-web-api/        client typé des routes JSON existantes (gloo-net dans le navigateur, reqwest pour les tests)
    ├── ent-web-app/        application Leptos : coquille, routes /web/…, connexion, module appel (modèle pur + vues)
    └── ent-web-server/     serveur : ent_api::app(state) existant + pages Leptos, même origine ; données de labo ; tests
```

Environ 3 300 lignes (Rust, CSS, scripts). Aucun JavaScript applicatif : le seul JavaScript est le code de chargement généré par `wasm-bindgen` (26 Ko) et le petit script d'hydratation inséré par Leptos.

### Versions épinglées (stables sur crates.io au 2026-11-02)

| Élément | Version |
|---|---|
| Rust (chaîne dédiée) | 1.99.0 + cible `wasm32-unknown-unknown` |
| `leptos` / `leptos_router` / `leptos_meta` / `leptos_axum` | 0.8.23 / 0.8.17 / 0.8.7 / 0.8.10 |
| `cargo-leptos` | 0.3.11 |
| `wasm-bindgen` (crate et CLI) | 0.2.130 |
| `wasm-opt` (binaryen) | version_123 (version attendue par cargo-leptos 0.3.11) |
| axum (identique à `ent-api`) | 0.8 |

## 3. Architecture et choix techniques

### Une seule origine
`ent-web-server` démarre `ent_api::boot_from_env()` puis fusionne :
- `ent_api::app(state)` **tel quel** : toutes les routes JSON, l'auth, les droits, l'ancienne interface `/app` ;
- les pages Leptos sous `/web/…`, les fonctions serveur sous `/api/…`, les fichiers du navigateur sous `/pkg/…`.

Même origine ⇒ pas de CORS, et le jeton de l'interface actuelle (`localStorage.ent_token`) est partagé : on peut passer de `/app` à `/web/appel` sans se reconnecter.

### Rendu : SSR + hydratation (îlots non retenus pour cet écran)
| Mode | Pour l'appel | Décision |
|---|---|---|
| CSR pur | page blanche jusqu'au chargement du WebAssembly | écarté |
| **SSR + hydratation** | structure, formulaires et états de chargement affichés immédiatement ; tout devient interactif après hydratation | **retenu** |
| Îlots (`islands`) | l'écran d'appel est interactif presque partout : il serait une seule grosse île | réservé aux pages majoritairement statiques (portails famille / élève, consultation) |

**Limite structurante :** la session `ent-auth` est un jeton `Authorization: Bearer` stocké dans le navigateur. Le serveur ne la voit pas lors du rendu HTML, donc les **données métier sont chargées par le navigateur** après hydratation (`LocalResource`). Le rendu serveur se limite à la coquille et aux squelettes. Une **fonction serveur** Leptos (`config_connexion`) est utilisée pour la configuration publique de la page de connexion : elle prouve que le mécanisme fonctionne (rendu serveur + appel HTTP par le navigateur).

### Réactivité fine
Chaque élève a ses propres signaux (statut, commentaire, sélection). Changer un statut ne redessine que sa ligne et les compteurs, d'où des temps de clic identiques pour 38 et 400 élèves (§7).

### Aucune règle métier dupliquée
Le modèle `appel/modele.rs` ne gère que l'état d'écran (saisie en cours, filtres, sélection, liste des changements). Qui peut faire l'appel, quels élèves sont inscrits, ce que déclenche la clôture : tout reste décidé par `ent-api`. Les types JSON sont recopiés côté client (`ent-web-api`) car `ent-api/src/dto.rs` est privé et dépend de `ent-db`/sqlx — voir §9.

### Capacités de l'API
L'API actuelle ne connaît que `present | absent | retard` et aucun commentaire par ligne. L'écran affiche « Dispensé » et le commentaire **désactivés, avec explication**, et refuse de les envoyer (`CAPACITES_API_ACTUELLE`). Activer ces fonctions demande une évolution backend (§9).

## 4. Fonctionnalités réalisées

| Demandé | Réalisé |
|---|---|
| Sélection de la classe et de la séance | Date, classe (si plusieurs), séance (`GET /edt/mes-creneaux`) |
| Liste des élèves | `POST /appels` (idempotent), tri alphabétique, présents par défaut |
| Recherche | insensible aux accents et à la casse, plusieurs mots |
| Filtres par statut | pastilles-compteurs (`aria-pressed`) |
| Présent / absent / retard / dispensé | boutons radio natifs ; « dispensé » désactivé (API) ; raccourcis P / A / R |
| Commentaire | champ présent, désactivé (API) |
| Sélection multiple | cases + « tout sélectionner (affichés) » + actions groupées — **corrigé** : seules les lignes affichées sont concernées (§8) |
| Validation, confirmation | `<dialog>` natif : focus sur « Annuler », Échap, retour du focus |
| Résultat | message horodaté, feuille relue depuis le serveur, lecture seule une fois close |
| Erreurs et chargements | squelettes, états vides, erreurs avec « Réessayer », session expirée → reconnexion, échec partiel détaillé (l'appel n'est pas clos) |

## 5. Authentification et sessions (étude, rien modifié)

**Aujourd'hui (prototype)** : connexion par `POST /auth/login` existant (mot de passe, comptes de labo sans passkey) ; jeton en `localStorage` ; envoi en `Authorization: Bearer`. Les comptes avec passkey sont renvoyés vers `/app` (second facteur hors périmètre).

**Cible proposée (D-008, à décider séparément)** :
1. `POST /auth/login` (ou une variante web) pose un cookie `HttpOnly; Secure; SameSite=Strict; Path=/` contenant le jeton opaque ; le jeton n'est plus lisible par JavaScript (protection contre le vol par XSS).
2. L'extracteur `EntSession` accepte le cookie **ou** le Bearer (applis mobiles inchangées).
3. Le rendu serveur Leptos lit le cookie (`leptos_axum::extract`) : les pages arrivent **avec les données**, et les fonctions serveur peuvent appeler directement les services `ent-db` avec les mêmes contrôles de droits.
4. Protection CSRF : `SameSite=Strict` + en-tête personnalisé obligatoire sur les écritures.
5. Passkeys : le flux WebAuthn existant (`navigator.credentials` via `web-sys`) s'intègre à la page de connexion Leptos.

Cela modifie `ent-auth`/`ent-api` : **décision distincte requise** (non faite ici).

## 6. Environnement local (outils portables)

Tout est dans `C:\Users\f.hacqueville\Desktop\Code\_outils-scolia` (à côté du dépôt). Aucun installeur, aucun service Windows, aucune variable globale.

| Dossier | Contenu |
|---|---|
| `pgsql/`, `pgdata/`, `pg.log` | PostgreSQL 18.4 (binaires EDB) et sa base ; écoute **127.0.0.1:54329 uniquement** (`listen_addresses`, `pg_hba` limité à 127.0.0.1/32, SCRAM) |
| `pg-*.pwd` | mots de passe locaux générés (rôle `ent_local`, superutilisateur) |
| `perl/` | Strawberry Perl 5.42.3.1 portable (compilation d'OpenSSL pour `webauthn-rs`) |
| `rustup/`, `cargo/` | chaîne Rust 1.99.0 dédiée + cible wasm32, cache des dépendances |
| `bin/` | `cargo-leptos`, `wasm-bindgen` ; `binaryen/` : `wasm-opt` |
| `target-web/` | sorties de compilation (voir §8.4) |
| `chrome-mesures/` | profil Chrome temporaire des mesures (83 Mo) |
| `labo-etab.txt`, `labo-seed.txt`, `mesures-chrome.json`, `ent-web-server.log` | données et journaux du labo |

Base : `ent_proto_test`, réservée aux tests locaux, données de labo fictives uniquement (distribution `seed_labo_cast` + classes `PROTO38` et `PROTO400` créées via l'API). Aucune donnée réelle, aucun secret de production, aucune connexion à une base distante (`ent-db` refuse d'ailleurs tout hôte non local).

### Commandes (Git Bash, depuis la racine du dépôt)

```bash
. ent-rust/web/scripts/env.sh              # environnement du terminal courant
bash ent-rust/web/scripts/pg.sh start      # démarrer PostgreSQL
bash ent-rust/web/scripts/pg.sh stop       # arrêter PostgreSQL
bash ent-rust/web/scripts/pg.sh reset      # recréer la base de test vide
cd ent-rust/web
cargo leptos build                          # développement (WebAssembly + serveur)
cargo leptos build --release --precompress  # production (+ fichiers .br/.gz)
"$CARGO_TARGET_DIR/release/ent-web-server.exe"     # serveur sur http://127.0.0.1:8090
"$CARGO_TARGET_DIR/release/seed-labo-web.exe"      # données de labo (affiche l'URL de connexion)
cargo test -p ent-web-api -p ent-web-app --features ent-web-app/ssr                        # tests unitaires
cargo test -p ent-web-server --test contre_backend -- --test-threads=1 --nocapture          # tests contre le vrai backend
```

### Retrait complet
1. Arrêter le serveur du prototype et `pg.sh stop`.
2. Supprimer le dossier `_outils-scolia` : PostgreSQL et ses données, Perl, chaîne Rust, cache, compilations, outils et profil Chrome disparaissent.
3. Restent seulement : les sources `ent-rust/web/` et `ent-rust/web/target/site` (1,9 Mo, ignoré par Git).

Vérifié : rien dans `AppData/Local` (le cache que `cargo-leptos` y avait créé a été déplacé dans `_outils-scolia/bin`), aucun outil ajouté dans `~/.cargo/bin`, cible wasm retirée de la chaîne Rust globale. Seule trace hors dossier : les sources des dépendances de `cargo-leptos` téléchargées dans `~/.cargo/registry` lors d'une première installation, faite avant la consigne d'isolement (sans effet sur le système).

## 7. Résultats

### 7.1 Tests automatisés — tous verts

| Suite | Tests | Résultat |
|---|---|---|
| `ent-web-api` (unitaires) | décodage du contrat JSON, connexion aplatie / passkey, statuts HTTP → erreurs | 3 / 3 |
| `ent-web-app` modèle (unitaires) | normalisation, tri, filtres, changements, blocages dispensé/commentaire, compteurs/raccourcis, 400 élèves, **non-régression action groupée** | 8 / 8 |
| `contre_backend` (vrai `ent-api` + Postgres local) | 4 / 4, voir ci-dessous | 4 / 4 |

Tests contre le vrai backend :
- **pages rendues par le serveur** : `/web/appel` (langue, titre, squelette, script d'hydratation, CSS, lien d'évitement, champ étiqueté), `/web/connexion` (champs, `autocomplete`, fonction serveur exécutée au rendu), redirection `/web`, 404, en-têtes `X-Frame-Options` / `nosniff`, `/health` et `/app` toujours servis ;
- **appel complet** : 401 sans jeton et mauvais mot de passe, `/auth/me`, séances, ouverture idempotente, refus d'un mauvais jour, envoi des seuls changements, refus de « dispense » par l'API, relecture, clôture, refus de modification après clôture ;
- **permissions** : famille → 403 (séances et ouverture) ; autre professeur → 403 ; CPE → autorisé ;
- **classes nombreuses** : 38 et 400 élèves (temps en §7.3).

### 7.2 Tests dans le navigateur (vrai backend)
Réalisés dans le navigateur de l'application puis dans Chrome sans fenêtre :
- connexion : mauvais mot de passe → message clair ; bon mot de passe → redirection vers l'appel, identité et rôle affichés ;
- souris, clavier (Tab jusqu'au groupe de statuts puis « R » → retard), annonce lecteur d'écran (« Adèle ROUX-017 : Retard »), recherche sans accent (« anais » → Anaïs ×2), « tout sélectionner » + « Marquer absent », compteurs et marqueurs « modifié » ;
- dialogue : focus initial sur « Annuler », Échap ferme et rend le focus à « Valider l'appel » ; confirmation → « Appel enregistré à 15:33 », feuille close en lecture seule ;
- **vérification en base** : appel `clos`, 3 lignes absentes, **3 absences créées dans `ent_absence`** ;
- mobile 375 px : aucun défilement horizontal, lignes en cartes, boutons de statut en grille (hauteur min. 42 px).

### 7.3 Mesures

**Compilation** (poste de l'utilisateur, Windows, chaîne MSVC) :

| Opération | Durée |
|---|---|
| Debug, à froid (serveur + `ent-api` + OpenSSL + téléchargements) | ≈ 10 min (435 s + 159 s) |
| Debug, premier `cargo leptos build` (dépendances wasm à froid) | 587 s |
| Debug incrémental après modification d'`ent-web-app` | 21 à 33 s (WebAssembly 6–7 s, serveur 20–30 s) |
| Tests incrémentaux (unitaires + contre backend) | 37 à 48 s, dont 11–13 s d'exécution |
| Release, à froid (WebAssembly optimisé en parallèle du serveur, OpenSSL release) | 560 s |
| Release incrémental (`--precompress`) | 53 s |

Le serveur domine les temps : il embarque `ent-api` (dont `ui.rs`, 54 000 lignes) et lie un gros binaire.

**Ressources envoyées au navigateur** (build release) :

| Fichier | Rôle | Brut | gzip | brotli |
|---|---|---|---|---|
| HTML `/web/appel` | rendu serveur | 2,4 Ko | — | — |
| `ent-web.js` | chargeur généré (pas de JS applicatif) | 26,3 Ko | 8,3 Ko | 6,2 Ko |
| `ent-web.wasm` | application | 1 015 Ko | 468 Ko | **313 Ko** |
| `ent-web.css` | styles | 9,5 Ko | 3,0 Ko | 2,3 Ko |
| **Total initial** | | **≈ 1 053 Ko** | ≈ 480 Ko | **≈ 324 Ko** |
| `GET /auth/me` | à la demande | 0,5 Ko | | |
| `GET /edt/mes-creneaux` | à la demande | 1,1 Ko | | |
| `POST /appels` (38 / 400 élèves) | à la demande | 4,1 Ko / 41 Ko | | |
| `POST /appels/{id}/lignes` | à la demande, par élève modifié | < 0,1 Ko | | |

En développement, le WebAssembly non optimisé pèse 24,7 Mo (normal, non destiné aux utilisateurs). Pas de découpage du WebAssembly à la demande dans ce prototype : tout l'écran est dans un seul fichier.

**Chargement et fluidité** — Chrome sans fenêtre, serveur release local, cache désactivé, `mesures-chrome.json` :

| | Processeur normal | Processeur ralenti ×4 (poste modeste) |
|---|---|---|
| Premier octet HTML | 4 ms | 4 ms |
| Page interactive (séances chargées) | 121 ms | 117 ms |
| Classe de 38 : affichage de la liste | 46 ms | 70 ms |
| 38 : clic statut → image (médiane / max sur 20) | 16,7 / 18,9 ms | 16,5 / 25,8 ms |
| 38 : recherche | 15,8 ms | 13,6 ms |
| Classe de 400 : affichage | 115 ms | 573 ms |
| 400 : clic statut (médiane / max) | 16,6 / 23,6 ms | 37,3 / 106,9 ms |
| 400 : recherche (→ 20 lignes) / retour à la liste complète | 13,5 / 76 ms | 56 / 367 ms |

≈ 16,7 ms = une image à 60 Hz : le clic est perçu comme instantané pour une vraie classe (35–40 élèves), même ralenti ×4. À 400 lignes ralenti ×4, recréer toute la liste reste perceptible (0,4–0,6 s) — cas hors usage normal de l'appel, mais à traiter pour les grands tableaux (§10).

**API (Rust natif contre le vrai backend, debug)** : ouverture 48–75 ms (38) / 136–146 ms (400) ; 2 à 3 ms par ligne envoyée ; clôture 11–15 ms (38) / 46–60 ms (400). Enregistrer 400 changements ≈ 1 s (une requête par ligne).

### 7.4 Accessibilité
- axe-core 4.10.2 (WCAG 2 A/AA, 2.1 AA, 2.2 AA) : **0 violation** ; page d'appel 27 contrôles réussis, 1 « à vérifier manuellement » (contraste sur fonds superposés) ; page de connexion 18 réussis.
- Construit sur des éléments natifs (`<table>`, `<th scope>`, `<fieldset>/<legend>`, radios, cases, `<dialog>`), `lang="fr"`, lien d'évitement, focus visible, zone `aria-live`.
- **Non validé** : test avec un vrai lecteur d'écran (NVDA, VoiceOver), parcours clavier complet sur 400 lignes, audit RGAA, Firefox/Safari, vrai poste scolaire. Un audit automatique ne suffit pas à déclarer l'écran accessible.

## 8. Anomalies rencontrées et corrections

1. **Hydratation cassée par un élément placé à la racine de l'App** juste après les balises `leptos_meta` (`<Stylesheet>`, `<Title>`) : erreur « expected a marker node » et application figée. Correction : lien d'évitement déplacé dans la page. **Règle :** à la racine de `App`, seulement les balises meta et `<Router>`.
2. **Options de compilation différentes** entre `cargo-leptos` (ajoute `--cfg erase_components` en debug) et `cargo test` : le serveur recompilé par les tests produisait un HTML incompatible avec le WebAssembly. Correction : `disable-erase-components = true`, puis `#![recursion_limit = "512"]` (types de vues très imbriqués). Effet de bord : `ent-api` a été compilé en double dans `target-web` (§8.4).
3. **Collision de nom** : `#[server] fn config_connexion` génère un type `ConfigConnexion` ; la structure a été renommée `ParametresConnexion`.
4. **Action groupée** modifiant aussi un élève sélectionné puis masqué par la recherche (constaté dans le navigateur). **Corrigé** : seules les lignes sélectionnées **et** affichées sont modifiées ; la barre indique « N autre(s) sélection(s) masquée(s) : non concernée(s) ». Test de non-régression `action_groupee_ignore_les_eleves_masques_par_la_recherche`. Correction couverte par les tests unitaires et compilée ; pas re-testée à la souris dans le navigateur.
5. **Fichiers servis non compressés** : ajout de `precompressed_br().precompressed_gzip()` + `--precompress`. Vérifié avec `curl` (brotli 313 Ko). Dans les navigateurs de ce poste, la réponse arrive décompressée avec un en-tête `X-Content-Encoding-Over-Network: br` qu'aucune partie du serveur n'ajoute : un intermédiaire du poste (très probablement un logiciel de sécurité) décompresse le trafic des navigateurs, même vers 127.0.0.1. Les mesures navigateur du §7.3 portent donc sur 1 Mo non compressé.
6. **Navigateur de l'application masqué** : animations et minuteries suspendues, mesures impossibles → mesures refaites dans Chrome sans fenêtre (profil séparé dans `_outils-scolia`).

### 8.4 Les 25 Go de `target-web` et le nettoyage proposé

| Répertoire | Taille | Contenu |
|---|---|---|
| `debug/deps` | 11 Go | bibliothèques et **informations de débogage** : `libent_web_server` 2,1 Go, `.pdb` des tests 1,1 Go, du serveur 1,0 Go, du script de labo 0,6 Go ; `libent_api` **en double** (2 × 509 Mo, avant/après le changement d'option du §8.2) |
| `debug/incremental` | 7,6 Go | cache de recompilation (serveur 4,1 Go, `ent-api` 1,8 Go, `ent-db` 0,4 Go…) |
| `debug/build` | 0,3 Go | scripts de compilation (dont OpenSSL) |
| `debug/*.pdb` racine | 1,7 Go | débogage du serveur et du script de labo |
| `front/` | 3,2 Go | WebAssembly debug 1,7 Go, release 0,35 Go, outils hôtes 1,2 Go |
| `release/` | 1,5 Go | serveur et WebAssembly optimisés |

Cause principale : les informations de débogage d'un binaire qui embarque tout `ent-api` (et `ui.rs`), multipliées par les variantes (serveur, tests, script, deux options de compilation).

Nettoyage proposé, **non effectué** (à valider) :
1. **Sans risque, gain ≈ 7,6 Go** : supprimer `target-web/debug/incremental` (prochaine compilation debug plus lente une fois).
2. **Gain ≈ 21 Go** : `cargo clean --profile dev` (supprime `debug/`) ; le prototype reste utilisable avec `release/` ; recompilation debug ≈ 10 min.
3. **Durable** : dans `ent-rust/web/Cargo.toml`, `[profile.dev] debug = "line-tables-only"` et `[profile.dev.package."*"] debug = false` — divise fortement la taille des `.pdb` (après recompilation).
4. Supprimer `chrome-mesures/` (83 Mo) après lecture des résultats.
À ne pas supprimer pour garder le prototype opérationnel : `release/`, `bin/`, `binaryen/`, `rustup/`, `cargo/`, `perl/`, `pgsql/`, `pgdata/`, `pg-*.pwd`.

## 9. Écarts avec le backend (évolutions à décider, non faites)

| Écart | Proposition |
|---|---|
| Pas d'état « dispensé » ni de commentaire par ligne | migration `ent_appel_ligne` (état `dispense`, colonne `commentaire`) + contrat API |
| Une requête par élève modifié (400 lignes ≈ 1 s) | route groupée `POST /appels/{id}/lignes:lot` |
| Appel clos = plus de correction | règle métier à définir (correction tracée par CPE ?) |
| Mauvais jour de semaine → **404 « appel déjà clos ou introuvable »** (`map_db` classe les erreurs par mots du message) | erreurs typées dans `ent-api` (400/409 explicites) |
| `eleve_label` = « Prénom NOM » : tri par prénom côté client | exposer `nom` / `prenom` séparés dans `AppelLigneDto` |
| Ouvrir une séance crée l'appel en base (effet de bord à la simple consultation) | `GET` de prévisualisation sans création |
| DTO privés et liés à sqlx : copie des types côté client | crate `ent-types` partagée (sans dépendance serveur) |
| Jeton Bearer dans `localStorage`, invisible au rendu serveur | cookie `HttpOnly` (§5) |
| `ent-rust/Cargo.lock` ignoré par Git | versionner le lockfile (reproductibilité, sécurité) |
| `POST /labo/bootstrap` crée un nouvel établissement à chaque appel (la base de labo grossit) | option de réutilisation |

## 10. Limites et réserves

Non bloquantes pour valider Leptos :
- **CSP** : Leptos insère un petit script d'hydratation en ligne ; une politique CSP stricte demandera des *nonces* (supportés par Leptos, non configurés ici).
- **Grands tableaux** (centaines de lignes sur poste lent) : prévoir pagination ou virtualisation dans le design system.
- **Taille du WebAssembly** : 313 Ko compressés pour un seul écran ; à surveiller quand les modules s'ajouteront (découpage à la demande `lazy` en 0.9, ou un WebAssembly par grande section).
- **Écosystème de composants pauvre** : tout le design system est à construire (confirmé : composants faits maison ici).
- **Pièges Leptos rencontrés** (§8.1–8.3) : à documenter dans `CLAUDE.md` pour les prochains développements.
- **Temps de compilation** dominés par le serveur qui embarque `ent-api` : 20–30 s par modification ; acceptable, mais à surveiller (découper le serveur, réduire les infos de débogage).
- **Accessibilité** non validée en conditions réelles (§7.4).
- Leptos 0.8 n'est pas en version 1.0 ; 0.9 est en rc : migration à prévoir (annoncée comme légère).

Aucune réserve bloquante n'a été trouvée.

## 11. Recommandation

**Leptos est validé techniquement** pour devenir le socle frontend de l'ENT : intégration propre à axum sur la même origine, réutilisation du backend sans modification, permissions intactes, écran métier complet fonctionnel contre le vrai backend, fluidité suffisante pour l'usage quotidien même sur poste ralenti, aucune violation d'accessibilité automatique, aucun JavaScript applicatif.

Avant la construction progressive du frontend (dans cet ordre) :
1. décider le passage de la session web en cookie `HttpOnly` (§5) — il conditionne le rendu serveur avec données ;
2. créer la crate `ent-types` partagée (§9) ;
3. versionner `Cargo.lock`, réduire les infos de débogage (§8.4), intégrer le build Leptos et les tests dans la CI ;
4. faire un test avec un vrai lecteur d'écran et sur un poste scolaire réel ;
5. poser les fondations du design system (tableau paginé/virtualisé, formulaires, CSP avec nonces).

Pas de migration générale de l'interface tant que ces points ne sont pas tranchés.
