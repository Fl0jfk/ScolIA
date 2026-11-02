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
