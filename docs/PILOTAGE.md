# Pilotage du chantier ScolIA (sessions IA)

Ce document est la **mémoire du pilotage** : comment le travail est organisé entre sessions, ce qui a été appris, et où en est la file. Il est écrit pour que n'importe quel pilote — la session actuelle, une session suivante, ou un modèle plus récent — puisse reprendre le chantier sans rien perdre. À relire en premier, avec `CLAUDE.md`, `docs/DECISIONS.md`, `docs/SOCLES.md` et `docs/ROADMAP.md`.

## 1. Rôles

| Rôle | Modèle | Ce qu'il fait | Ce qu'il ne fait pas |
|---|---|---|---|
| **Pilote** | Opus | Décide de l'ordre, rédige les consignes, tient ce document, parle à Florian, pousse sur `ENT` après contrôle. | Ne code pas les lots ; ne pousse jamais sans validation verte. |
| **Contradicteur** | Opus (sous-agent ou session courte) | Relit **avant lancement** chaque vague de consignes et chaque décision structurante : décisions de Florian bien incluses, modèle juste, plages de migration, dépendances entre branches, risques de conflit, oublis. Relit aussi ce document à chaque jalon. | Ne décide pas ; rend une liste d'objections que le pilote traite ou écarte par écrit. |
| **Concepteur de socle** | Opus | Conception de fond d'un socle (`docs/socles/NN_*.md`). | Pas de code. |
| **Personas (conception)** | Opus | Passe de personas sur une conception : manques, réglages en trop signalés. | Ne supprime rien. |
| **Équipe de code** | Sonnet (sous-agents Sonnet, recherches Haiku) | Un lot par session, sur sa branche, tests et revue de sécurité. | Ne pousse jamais sur `ENT`, `main`, `dev`. |
| **Intégrateur** | Sonnet, **un seul à la fois** | Prend la file d'intégration dans l'ordre, fusionne, fait valider par la **CI GitHub** (workflow `ent-rust.yml` lancé à la main sur la branche d'intégration), corrige ce que la fusion casse. | N'ajoute pas de fonctionnalité. |
| **Recette par personas (logiciel)** | Opus + sous-agents Sonnet | Après le code d'un socle : personas qui travaillent et **attaquent** le logiciel réel (données de labo), chaque trouvaille devient un test. | Pas de données réelles. |

## 2. Règles apprises (à appliquer sans exception)

1. **Toute consigne métier va dans le message de lancement d'une session**, jamais dans un message envoyé ensuite : une session refuse, à raison, d'agir sur la demande d'une autre session (F-012). Pour changer une consigne en cours de route : nouvelle session dédiée qui part de la branche.
2. **Choisir le modèle à chaque lancement** (F-010) : Sonnet par défaut, Opus pour conception, personas, contradicteur, recette.
3. **Plage de migrations réservée avant lancement** dans `docs/socles/MIGRATIONS.md`.
4. **La CI GitHub fait foi** pour valider une intégration : les conteneurs cloud redémarrent et coupent les suites longues (F-012). Les suites locales servent au développement, découpées par paquet pour être reprises.
   **Le job « Backend » est parallèle depuis `ent-ci-parallele` (2026-11-09)** : le job unique de 1 h 45 à 3 h 50 (suite complète d'un bloc, un seul fil, une seule base ; run séquentiel 42569240935 = **3 h 51**) est remplacé par `backend-lint` (clippy avec et sans labo, `ent-types -D warnings`, compilation de production, séparation L4), `backend-archive` (compile une fois les binaires de test, avec et sans labo, en archives `cargo nextest archive`), `tests-labo` (**20 tranches** `count:i/20`, un fil chacune, **un PostgreSQL éphémère par tranche**), `tests-sans-labo`, puis le job final `Backend (clippy, compilation, tests)` (nom conservé) qui exige tous les jobs verts **et** vérifie la couverture. Un nouveau test est réparti automatiquement (partition par rang, pas de liste de modules à tenir à jour). `cargo-nextest` est épinglé (0.9.150, installé par `taiki-e/install-action`, déjà utilisé pour `cargo-deny`). **Contrôle de couverture** : l'inventaire `cargo nextest list` de chaque archive est comparé aux journaux des tranches ; tout test listé non exécuté, exécuté deux fois ou exécuté sans être listé fait échouer le job final, et le tableau par binaire est écrit dans le résumé du run. Les tests de documentation (`cargo test --doc`) restent exécutés par `backend-archive` (nextest ne les couvre pas). Lire le journal du job final avant de chercher dans les 20 tranches : il nomme les tests en échec.
5. **Une seule file d'intégration**, dans l'ordre ; pas deux intégrations en parallèle sur des bases différentes.
6. **Rien sur `ENT` sans les quatre jobs CI verts** (formatage, audits, backend, frontend) et un contrôle du pilote (journal des commits, `MIGRATION_TAG`, aucun test ignoré ajouté, décisions de Florian respectées).
7. **Trois sessions actives en continu** (Florian, 2026-11-08) ; on peut en lancer une ou deux de plus quand les autres ne font qu'attendre un retour de CI ou de tests. On accélère en fin de semaine si le quota hebdomadaire reste large. Jamais plus de 6 à 7 : au-delà, la limite d'usage coupe tout.
11. **Toujours répondre à Florian en français** (Florian, 2026-11-09), y compris les points d'étape, les rapports techniques et les résumés de sessions ; termes techniques expliqués simplement.
10. **Suivi toutes les 30 minutes** (rappel programmé) : voir ce que font les sessions, relancer la CI, archiver, remplacer une session finie ; ne jamais laisser les sessions sans travail.
8. **Archiver une session** dès que son travail est poussé et qu'elle n'attend plus rien.
9. **Tout ce qui compte s'écrit dans le dépôt** (décisions, état, leçons), jamais seulement dans une conversation.

## 3. Cycle d'un socle (D-021)
Conception (Opus) → personas sur la conception (Opus) → lots de code (Sonnet) → intégration (file) → recette par personas sur le logiciel (Opus) → relecture de Florian. Un socle n'est « fini » qu'après la recette.

## 4. File d'intégration et état (à tenir à jour)

État au 2026-11-09, 13 h 10 UTC :
- **Sur `ENT`** (`e8a69c53`, CI parallèle verte : 27 jobs, 1 137 tests, 30 min) : socle 1 ; lots 6, 7 ; lot 8 bis (socles 2–4 de base) ; **lot 9** (correctifs recettes 1 et 2, E5–E6, I2, S0–S1) ; **lot 10** (A2, P0–P1, S2, écrans de la démo P1, nouvelle CI parallèle). Exception d'audit RUSTSEC-2026-0459 (SQLite non compilé, vérifié par le pilote) à revoir au 2027-02-01.
- **Lot 11** (`ent-integration-lot11`) : socle 5 S3, S4, S5, S7 ; socle 6 P2 ; en cours, à rebaser sur la nouvelle tête d'ENT.
- **Correctifs recette sécurité 3** (`ent-recette-securite-3`) : RS3-01 corrigée ; RS3-02 à 04 en cours ; à intégrer au lot 12.

### Rattrapage des domaines codés avant la méthode (Florian, 2026-11-09)
Le socle 1 et le lot 7 (évaluations, finances et SEPA, communication, santé, emploi du temps) ont été codés avant D-021 : pas de conception de fond, pas de passe personas, pas de recette. À rattraper, dans cet ordre, sans bloquer les socles en cours :
1. **Recette sécurité n° 3** (cloisonnement multi-établissements, attaques sur toutes les routes, tous modules) — lancée le 2026-11-09 (`ent-recette-securite-3`).
2. **Socle 10 Emploi du temps** : conception de fond à partir de l'existant — lancée le 2026-11-09 (`ent-socle-edt-conception`), puis personas, recette.
3. Puis, un à la fois : socle 21 Évaluations (avec 22–23), socle 16 Messagerie (avec 17), socles 24–25 Facturation et paiements, socle 34 Santé (désactivé tant qu'il n'y a pas d'HDS).
Chaque rattrapage suit le cycle complet : conception (évaluation de l'existant) → personas → lots de correction → recette.

## 5. Transmission à un futur pilote
Un pilote qui reprend : lire ce document, `CLAUDE.md`, `docs/DECISIONS.md` (les décisions de Florian font foi), `docs/ROADMAP.md` (jalons de preuve), `docs/SOCLES.md` (carte), `docs/socles/` (conceptions), `docs/PROJECT_STATUS.md` (état technique), `docs/DECISIONS_AND_FAILURES.md` (erreurs à ne pas refaire) ; lister les sessions et les branches ; reprendre la file du §4. Les conceptions et les décisions sont indépendantes du modèle : un modèle plus récent peut les approfondir sans les refaire.

## 6. Objections du contradicteur
Chaque revue du contradicteur est résumée ici avec le traitement de chaque objection (fait / écarté et pourquoi).
- **2026-11-07 (revue 1)** : recette 1 sur le mauvais code → recadrée, recette P1 à prévoir (fait) ; lignées divergentes → ordre de file écrit, chaque lot repart de la tête d'ENT (fait) ; `perimetre.rs` modifié deux fois → D-022 bis fait foi, le correctif de la recette sera reporté à la main avec revue (à faire) ; registre des migrations faux → corrigé (fait) ; D-022 bis code « cours » avant le socle 5 → marqué provisoire (à faire) ; branches périmées dans le lot 8 → signalé à l'intégrateur (à faire) ; cadence → débit à mesurer et annoncer chaque semaine (fait, §7) ; CI par PR → adopté pour les prochaines intégrations (à faire) ; trop de branches en parallèle sur les mêmes fichiers → **cause structurelle traitée par D-026 (architecture en briques)**.

## 7. Débit
Mesure : lots arrivés sur `ENT` par jour, et lots de code terminés sur branche par jour. Au 2026-11-07 : 1 intégration sur ENT en 1,5 jour (lot 6) ; environ 6 lots de code terminés par jour sur branche.
