# Design system

Direction : moderne, sobre, soignée et productive. Inspirations de qualité Apple/Google et Bento, sans appliquer Bento à chaque écran.

- hiérarchie claire, cohérence inter-modules ;
- tables denses mais lisibles, recherche, filtres combinables, tris, pagination/virtualisation ;
- actions groupées et raccourcis utiles ;
- états loading/empty/error/success/access denied ;
- responsive, clavier, focus visible, contraste et accessibilité ;
- thèmes si adaptés ;
- animations courtes et utiles, jamais au détriment de la performance.

Inspecter d'abord le système UI du frontend Rust existant. Documenter tokens, composants et patterns communs avant refonte.

Référence skills : https://github.com/emilkowalski/skills. Étudier sélectivement les skills de design engineering, animation, performance, review, prototypes et mobile. Vérifier licences/provenance, n'installer que ce qui sert, adapter au frontend Rust. Ne pas appliquer mécaniquement React/Expo/Swift, ne pas activer toute la collection, pas de refonte globale automatique.

Fondations du frontend Leptos (architecture des composants, ordre de réalisation) : voir `DESIGN_SYSTEM_LEPTOS.md`.
