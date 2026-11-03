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
