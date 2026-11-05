# Journal des échecs et apprentissages

Pour chaque échec significatif : date/contexte, objectif, approche, résultat observé, cause probable (distincte des faits), coût/risque, décision de reprise/arrêt et apprentissage durable.

Ne pas stocker secrets ou données personnelles réelles. Ne pas journaliser chaque erreur triviale. Ne pas répéter une approche déjà échouée sans expliquer ce qui a changé.

---

## F-001 — Interface web de l'ENT en JavaScript dans une chaîne Rust

- **Date / contexte :** constaté à l'audit du 2026-11-02 ; UI construite du 15 au 18 octobre 2026 (commits « lot A » à « lot CO » sur `ENT`).
- **Objectif :** livrer vite une interface web (staff, famille, élève) au-dessus de l'API Rust.
- **Approche :** une SPA JavaScript sans framework, écrite comme une seule chaîne de texte (`const APP: &str`) dans `ent-rust/crates/ent-api/src/ui.rs`, Tailwind v4 pour le style.
- **Résultat observé (faits) :** 54 000 lignes dans un seul fichier ; aucun test d'interface ; la pile Tokio a dû être augmentée (`RUST_MIN_STACK` 16 Mo, commit `f06fcc1d`) pour éviter un dépassement de pile ; la décision documentée (Leptos, `docs/ent-vision-rust.md`) n'a jamais été appliquée.
- **Cause probable (hypothèse) :** recherche de vitesse lors de lots générés en série, sans revue d'architecture du frontend.
- **Coût / risque :** UI difficile à maintenir et à tester ; contraire à l'objectif « tout en Rust ».
- **Décision :** remplacer par un frontend Rust (D-002 dans `DECISIONS.md`) ; framework à choisir.
- **Apprentissage :** une décision d'architecture écrite doit être vérifiée dans le code à chaque lot ; un fichier qui dépasse quelques milliers de lignes est un signal d'alerte à traiter tout de suite.

## F-002 — Reformatage involontaire du backend par `cargo fmt --all`

- **Date / contexte :** 2026-11-03, mission « Reproductibilité et industrialisation ».
- **Objectif :** formater uniquement le code du frontend (`ent-rust/web`).
- **Approche :** `cargo fmt --all` lancé dans le workspace `ent-rust/web`.
- **Résultat observé (faits) :** 76 fichiers du backend (`ent-api`, `ent-db`, `domain-eleve`…) reformatés, alors que le formatage du backend attend une décision. Détecté avant commit (relecture de `git status`), fichiers restaurés à l'identique (`git checkout`), aucune modification commitée.
- **Cause :** `cargo fmt --all` formate aussi les dépendances locales par chemin ; le workspace `web` dépend d'`ent-api` et d'`ent-types`.
- **Correction :** commande limitée aux crates du frontend (`cargo fmt -p ent-web-ui -p ent-web-api -p ent-web-app -p ent-web-server`) dans la CI, `CI_REPRODUCTIBILITE.md` et `FRONTEND_LEPTOS.md`.
- **Apprentissage :** toujours relire `git status` avant un commit ; dans un workspace qui dépend d'autres crates locales, ne jamais utiliser `--all` avec un outil qui modifie les fichiers.

## F-003 — Premier passage de la CI sur GitHub : échec des tests frontend contre le backend

- **Date / contexte :** 2026-11-03, premier push du workflow `ent-rust.yml` (run 41599498046).
- **Résultat observé (faits) :** formatage, audits et backend verts ; job frontend en échec à l'étape « Tests contre le vrai backend ». Journaux GitHub illisibles sans compte connecté ; échec **reproduit localement** sur une base PostgreSQL vierge, sans variables `LEPTOS_*` : `pages_rendues_par_le_serveur` ne trouvait pas `/pkg/ent-web.js` dans la page.
- **Cause :** Leptos lit `LEPTOS_OUTPUT_NAME` **à la compilation** pour nommer le script d'hydratation. `scripts/env.sh` la pose en local, le workflow non : la simulation locale du 2026-11-03 n'était donc pas identique à la CI.
- **Correction :** `LEPTOS_OUTPUT_NAME` et `LEPTOS_SITE_PKG_DIR` ajoutées à l'environnement du job frontend ; 6/6 tests verts en local dans les conditions de la CI (base vierge, sans autres variables). Correction **non poussée** (attente d'autorisation).
- **Apprentissage :** une simulation locale de CI doit partir d'un environnement vide (base vierge, variables de `env.sh` retirées), sinon elle masque les différences.

## F-004 — Tranche 1 du frontend : compilation interrompue, mauvaise chaîne Rust, pièges Leptos

- **Date / contexte :** 2026-11-03, tranche 1 (parcours enseignant, `FRONTEND_CIBLE.md`).
- **Résultat observé (faits) :**
  - la 1re compilation de la tranche avait été interrompue lors de la session précédente ; reprise en nouvelle session ;
  - compilation tentée avec la chaîne Rust système : échec (pas de cible `wasm32`, Perl MSYS sans module pour `openssl-src`) ;
  - lecture d'une ressource Leptos hors `<Suspense/>` (avertissement d'hydratation) ; `location.search` sans « ? » initial a cassé l'adresse de retour ; `Uuid::new_v4` indisponible (feature `v4` non activée) ; test `pages_rendues` cassé par un changement de balisage ;
  - la restriction des agents `explore-haiku` / `review-sonnet` à `tools: Read, Grep, Glob` n'a pas été prise en compte dans la session où les fichiers ont été modifiés (définitions lues au démarrage).
- **Cause :** chaîne non dédiée utilisée (fait) ; définitions d'agents lues au démarrage (hypothèse à confirmer).
- **Correction :** règle « toujours `. ent-rust/web/scripts/env.sh` » (chaîne `_outils-scolia` : Rust 1.99 + wasm32, Strawberry Perl, cargo-leptos) ; conventions ajoutées à `FRONTEND_LEPTOS.md` (§ 11) ; restriction des agents **à vérifier dans une nouvelle session**.
- **Apprentissage :** ne jamais compiler avec la chaîne système ; relire les assertions des tests de rendu quand le balisage change.

## F-005 — Défauts de sécurité trouvés par revue pendant la tranche 1

- **Date / contexte :** 2026-11-03, revue de la tranche 1.
- **Résultat observé (faits) :** (1) porte du labo (`POST /labo/bootstrap`) et inscription libre **fail-open** : `ENT_ENV` absent = ouvert ; (2) accès horizontal sur l'appel : seul `ouvrir_appel` contrôlait titulaire / remplaçant / supervision, lire, pointer et clore étaient permis à tout professeur de l'établissement connaissant l'`appel_id` ; (3) garde « base locale uniquement » des tests par sous-chaîne (`contains("@127.0.0.1:")`), contournable.
- **Cause probable (hypothèse) :** contrôle écrit sur le point d'entrée seulement ; garde par motif textuel au lieu d'une analyse d'URL.
- **Correction (implémentée, testée en local, non poussée) :** `gate_open` fail-closed (`ENVS_NON_PROD`) ; `autoriser_acces_appel` appliqué partout (403) avec tests ent-db et `appel_d_un_collegue_inaccessible` ; `ent_db::assert_local_postgres_url` (garde d’hôte loopback seulement, `url_guard.rs:17-29`) ; le nom de base test/proto/lab n’est vérifié que dans les tests `contre_backend.rs` et `session_cookie.rs` (ent-lab et les scénarios ent-db n’ont que la garde d’hôte) ; hôte exact dans `seed_labo_web`.
- **Apprentissage :** une règle d'autorisation doit être portée par une fonction unique appelée par chaque opération ; défaut sûr = fermé ; faire relire toute porte de dev par un second regard.

## F-006 — Incidents pendant l'intégration de la vague 1 du socle de confiance

- **Date / contexte :** 2026-11-04, intégration des correctifs de la vague 1 (lots L1 à L8, tranche 2) par des copies de travail d'agents, puis fusion des modifications.
- **Objectif :** intégrer les correctifs de sécurité sur `ENT` sans régression.
- **Approche :** copies de travail Git par agent ; intégration de blocs de code par correspondance de texte ; fusion à trois versions.
- **Résultat observé (faits) :**
  1. Copies de travail Git des agents créées sur `main` au lieu de `ENT` ; corrigé par `git checkout --detach f1e46f9b`.
  2. Un bloc de correctif intégré par correspondance approximative s'est appliqué dans la mauvaise fonction (commissions au lieu des points de vie scolaire) ; détecté par la matrice L8 (`separation.rs`).
  3. Fusion à trois versions faussée par les fins de ligne (CRLF du dépôt principal contre LF des copies) ; refaite après normalisation.
  4. Saturation de la machine avec six compilations Rust simultanées.
- **Cause probable (hypothèse) :** (1) point de départ des copies non fixé sur `ENT` ; (2) correspondance de contexte trop souple, sans vérification de la fonction cible ; (3) absence de normalisation des fins de ligne avant fusion ; (4) parallélisme non borné.
- **Coût / risque :** deux intégrations fausses à reprendre ; risque de régression silencieuse ; temps machine perdu.
- **Décision :** pas de décision structurante. Créer les copies de travail depuis `ENT`, normaliser les fins de ligne avant fusion, limiter à environ quatre compilations simultanées.
- **Correction :** copies ramenées sur `ENT` (détachées au commit `f1e46f9b`) ; fusion refaite après normalisation ; bloc mal placé détecté puis contrôlé par la matrice L8, dont la suite passe le 2026-11-04.
- **Apprentissage :** une vraie fusion à trois voies vaut mieux qu'une application de patch par correspondance approximative ; normaliser les fins de ligne avant toute fusion ; garder une matrice de tests de séparation comme filet d'intégration ; borner le nombre de compilations simultanées.

## F-007 — Sous-agents « remote » exécutés dans le dossier principal

- **Date / contexte :** 2026-11-05, socle 1, parallélisation demandée par Florian (7-8 agents).
- **Approche :** trois sous-agents lancés avec l'isolation `remote` du tool Agent, consignes « crée ta branche, pousse-la ».
- **Résultat observé (faits) :** les trois agents ont tourné **dans le dossier principal** (`/home/user/ScolIA`), ont changé la branche courante (`git checkout -b`) et créé un fichier de migration pendant qu'une suite de tests y tournait. Détecté par le hook « changements non commités » ; agents arrêtés, branches parasites supprimées (aucun commit, aucun push), dossier remis sur `ent-mc0hoe`, suite de tests relancée.
- **Cause probable (hypothèse) :** l'isolation `remote` n'est pas disponible dans cet environnement et retombe sur une exécution locale sans copie isolée.
- **Correction :** travaux parallèles qui compilent confiés à de **vraies sessions cloud** (`create_session`, une machine et un clone par session, branche de sortie dédiée) ; sous-agents locaux seulement en `worktree` ou en lecture seule.
- **Apprentissage :** vérifier l'isolation effective d'un agent (`git worktree list`, branche courante) dans les minutes qui suivent son lancement.

