# Mission initiale — audit lecture seule

## Objectif
Comprendre l'état réel du dépôt `Fl0jfk/ScolIA`, distinguer ancien ScolIA et nouvelle ENT Rust, retrouver les choix antérieurs et préparer une documentation cohérente sans casser l'existant.

## Phase 0 — sécurité
- Confirme la racine du dépôt, branche courante et état Git, sans changer de branche.
- Identifie le nom exact de la branche Rust autorisée uniquement si les preuves sont suffisantes ; sinon « à confirmer ».
- Pas d'opération Git distante.
- Ne lis jamais `.env`, secrets, tokens, credentials, clés privées ou secrets CI. Signale seulement leur présence/chemin.
- Next.js de production : lecture ciblée seulement, aucune action sur ses fichiers ou son environnement.
- DB de production : aucun accès, même lecture.

## Phase 1 — inventaire non destructif
Cartographie arborescence, manifests, lockfiles, documentation, notes, TODO, ADR, workflows, scripts, code Rust, migrations statiques, tests, apps Android/iOS, dépendances et licences. Ne lance rien qui pourrait modifier l'environnement, installer une dépendance, contacter un service ou accéder à la production.

## Phase 2 — générations
Distingue ancien ScolIA, nouvelle ENT Rust, Next.js en production (frontière intouchable), services partagés, code abandonné/doublonné/incertain. Ne déclare pas du code mort sans indices.

## Phase 3 — choix à retrouver
Identifie avec preuves : framework frontend Rust, backend, authentification Rust, PostgreSQL/migrations, architecture tenant, Android/iOS et périmètre par rôle, stockage fichiers, email, SMS, IA, conversion documentaire, outils bureautiques et usages WASM. Ne remplace aucun composant par préférence personnelle.

## Phase 4 — documentation historique
Lire tous les Markdown et notes utiles. Classer : valide, à actualiser, contradictoire, redondant, idée à conserver, décision abandonnée, question ouverte. L'objectif est de synthétiser et fusionner, pas de repartir de zéro. Ne supprime aucun document historique ni code pendant cette mission.

## Rapport attendu
1. Résumé exécutif.
2. Cartographie du dépôt et séparation ancien/nouveau.
3. Stack trouvée avec chemins/symboles comme preuves.
4. État des applications mobiles.
5. Matrice fonctionnelle par domaine.
6. Sécurité, multi-tenant, données et tests.
7. Documentation à conserver/fusionner/actualiser.
8. Risques/dettes techniques.
9. Faits certains, hypothèses, recommandations, questions.
10. Plan documentaire et plan de travail priorisé.

## Interdictions pendant toute la mission
Aucun changement de branche, fichier, code, doc, formatage, suppression, installation, migration, seed, requête DB, accès aux secrets, build risqué, commit, push, merge, PR, release, déploiement ou action distante. Ne touche pas à Next.js. N'accède jamais à la DB de production.

**Arrêt obligatoire :** présente le rapport puis attends validation avant toute écriture documentaire ou développement.
