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
- **Correction (implémentée, testée en local, non poussée) :** `gate_open` fail-closed (`ENVS_NON_PROD`) ; `autoriser_acces_appel` appliqué partout (403) avec tests ent-db et `appel_d_un_collegue_inaccessible` ; `ent_db::assert_local_postgres_url` + nom de base test/proto/lab ; hôte exact dans `seed_labo_web`.
- **Apprentissage :** une règle d'autorisation doit être portée par une fonction unique appelée par chaque opération ; défaut sûr = fermé ; faire relire toute porte de dev par un second regard.
