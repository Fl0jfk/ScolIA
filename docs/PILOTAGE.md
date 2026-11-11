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
7. **Au plus 6 à 7 sessions actives** ; au-delà, la limite d'usage coupe tout.
8. **Archiver une session** dès que son travail est poussé et qu'elle n'attend plus rien.
9. **Tout ce qui compte s'écrit dans le dépôt** (décisions, état, leçons), jamais seulement dans une conversation.

### Migrations figées (D-036, table `ent_migration_figee`)

Les migrations sont rejouées à chaque démarrage et une migration déjà versée sur `ENT` ne se modifie plus. Quand on **retire** une colonne ou une table qu'un ancien fichier lit ou recrée (reprise de données, `ADD COLUMN IF NOT EXISTS`, déclencheur), ce fichier ne peut plus être rejoué : on le **fige** au lieu de le modifier.

- **Qui décide** : le pilote, au cas par cas, après avoir lu les fichiers concernés. Une session de développement propose ; elle ne fige jamais seule.
- **Comment** : une migration de retrait (`DROP … IF EXISTS`, déclencheurs et fonctions compris), puis une migration qui inscrit les fichiers dans `ent_migration_figee` (modèle : 668–670). Le lanceur (`migrate.rs`) saute chaque fichier listé **et journalise chaque fichier sauté** ; il **refuse de démarrer** si la table nomme un fichier inconnu (`MigrateError::FigeInconnu`).
- **Quand** : seulement si le fichier a déjà été joué partout (base neuve : dans l'ordre, avant la migration de figeage ; base existante : déjà jouée), et seulement s'il **lit ou recrée** un objet retiré. Jamais pour « alléger » le démarrage.
- **Fichier qui crée une table** : à éviter. Quand le fichier mêle création d'une table typée et lecture de l'ancienne colonne (cas de 560–563 et 662–667, impossibles à scinder sans les modifier), le figer n'est admis que si **toutes les tables qu'il crée existent** ; le test `les_tables_creees_par_les_fichiers_figes_existent` (`tests_d036`) le vérifie. Une table à recréer plus tard passe par une **nouvelle** migration.
- **Preuves exigées** (`tests_d036`) : base neuve, démarrages successifs sur la même base, base migrée jusqu'à la dernière migration avant retrait puis passage au retrait (la reprise doit avoir lieu une dernière fois), refus d'un nom inconnu.
- Le test de garde des colonnes JSON (`COLONNES_JSON_D036`) retranche les `ALTER TABLE … DROP COLUMN` du schéma net ; `ent_migration_figee` est classée « Technique » au registre des tables.

## 3. Cycle d'un socle (D-021)
Conception (Opus) → personas sur la conception (Opus) → lots de code (Sonnet) → intégration (file) → recette par personas sur le logiciel (Opus) → relecture de Florian. Un socle n'est « fini » qu'après la recette.

## 4. File d'intégration et état (à tenir à jour)

État au 2026-11-08 (mise à jour de l'intégrateur du lot 10 ; reste du paragraphe : état du lot 8 bis) :
- **Sur `ENT`** : socle 1, lot 6 (correctifs de la revue de sécurité, inscriptions corrigées I0, cahier de textes, documentation des socles).
- **Lot 7** (`ent-integration-lot7`) : évaluations, finances, communication, santé désactivée par défaut, emploi du temps ; **sur `ENT` (d1326cd), validé par la CI GitHub**.
- **Lot 8 bis** (`ent-integration-lot8b`, base d1326cd) : fusions `--no-ff` de E3-E4 (socle 2, E0 à E4), A1 (socle 4, A0 à A1), I1 (socle 3) et D-022 bis (D-022 comprise, intégrée une seule fois) ; `MIGRATION_TAG` = `370_edt_creneau_groupe`. Vert en local (compilation, clippy, formatage, suites ent-db et ent-api exposées aux fusions) ; **prêt pour la CI GitHub**, à lancer par le pilote. Point à contrôler par le pilote : l'annuaire des élèves du professeur est borné à `mes_classes` (règle de groupe D-022) par-dessus le chemin E1 ; voir `PROJECT_STATUS.md`, section « Intégration du lot 8 bis ».
- **Lot 9** (`ent-integration-lot9`, base `ent-integration-lot8b`, 2026-11-08) : recette 1 (`ent-recette-1-correctifs`, qui contient `ent-recette-personas-1`), recette 2 (`ent-recette-2-correctifs`), socle 2 E5-E6, socle 3 I2, socle 5 S0-S1 ; `MIGRATION_TAG` = `375_msg_fil_prive`. Toutes les fusions sont faites ; tests ciblés verts (RP1 14/14, RP2 vert, aucun RP rouge) ; **prêt pour la CI GitHub**, à lancer par le pilote. Détail, conflits et adaptations de tests : `PROJECT_STATUS.md`, « Intégration du lot 9 ». Non intégrées : `ent-socle-administratif-a2` (en cours), `ent-socle1-revue-faibles`, `ent-mobile-app`. À contrôler par le pilote : l'annulation de validation d'I2 clôture désormais à durée nulle la scolarisation ouverte par le dossier (RP2-10) ; le profil `compta` perd `suivi.eleve` et `journal.lire` (RP1-07) ; coexistence des rôles `Accueil` et `Personnel`.
- **Lot 10** (`ent-integration-lot10`, base `ent-integration-lot9`, 2026-11-08) : socle 4 A2, socle 6 P0-P1, socle 5 S2, écrans de la démo P1 ; `MIGRATION_TAG` = `375_msg_fil_prive`. Toutes les fusions sont faites et poussées ; tests ciblés verts en local ; **CI GitHub : run 42525091457 (1ec58d2) rouge sur un seul test `domain-eleve` ; corrigé par la fusion de `ent-integration-lot9` ; nouveau run 42568472675 (78527a6f) annulé après un 504 transitoire sur le téléchargement de binaryen (job frontend) ; étape durcie (nouvelles tentatives, f22a82e0) ; run 42568816601 (f22a82e0) annulé : 504 persistant de github.com (plus de 30 s) ; fenêtre de nouvelles tentatives élargie (ce82cae4) ; nouveau run 42569240935 en cours**. Points à relire : `docs/PROJECT_STATUS.md`, « Intégration du lot 10 ».
- **Lot 11** (`ent-integration-lot11`, base `ent-integration-lot10` + ses 10 derniers commits, 2026-11-09) : socle 5 S5, S3, S7, S4 et socle 6 P2 ; `MIGRATION_TAG` = `375_msg_fil_prive`. Fusions faites et poussées ; tests locaux verts (voir `PROJECT_STATUS.md`, « Intégration du lot 11 ») ; **CI GitHub : run 42676968652 (553668a5), lancé le 2026-11-09 ; résultat à reporter ici par le pilote**. Points à relire : fusion d'élèves et `ent_cours_membre`, liste unique du journal de structure.
- **Lot 8** (`ent-integration-lot8`, remplacé par le lot 8 bis pour E0–E4, A0–A1, I1 et D-022 bis) : socle 2 E0–E2, socle 4 A0 + A0 bis, failles faibles socle 1, D-022, mobile ; démarre sur le lot 7. Attention (contradicteur) : `ent-mobile-app` et `ent-socle1-revue-faibles` sont très en retard sur leur base ; `ent-d022-bis-enseignements` contient déjà `ent-d022-portee-groupes` et le lot 7 : ne pas intégrer D-022 deux fois.
- **Ordre de la file après le lot 8** (une branche à la fois, chacune repart de la nouvelle tête d'ENT) : E3-E4 → A1 → I1 → D-022 bis → correctifs de la recette. La recette personas 1 porte sur le socle 1 et le lot 7, **pas** sur les socles 2 à 4 : une recette P1 sera relancée sur une branche contenant E4, A1 et I1.
- **Branches concernées** : `ent-socle-inscriptions-i1` (socle 3 I1), puis à venir `ent-socle-eleve-e3-e4`, `ent-socle-administratif-a1`, `ent-d022-bis-enseignements`, `ent-recette-personas-1`.

## 5. Transmission à un futur pilote
Un pilote qui reprend : lire ce document, `CLAUDE.md`, `docs/DECISIONS.md` (les décisions de Florian font foi), `docs/ROADMAP.md` (jalons de preuve), `docs/SOCLES.md` (carte), `docs/socles/` (conceptions), `docs/PROJECT_STATUS.md` (état technique), `docs/DECISIONS_AND_FAILURES.md` (erreurs à ne pas refaire) ; lister les sessions et les branches ; reprendre la file du §4. Les conceptions et les décisions sont indépendantes du modèle : un modèle plus récent peut les approfondir sans les refaire.

## 6. Objections du contradicteur
Chaque revue du contradicteur est résumée ici avec le traitement de chaque objection (fait / écarté et pourquoi).
- **2026-11-07 (revue 1)** : recette 1 sur le mauvais code → recadrée, recette P1 à prévoir (fait) ; lignées divergentes → ordre de file écrit, chaque lot repart de la tête d'ENT (fait) ; `perimetre.rs` modifié deux fois → D-022 bis fait foi, le correctif de la recette sera reporté à la main avec revue (à faire) ; registre des migrations faux → corrigé (fait) ; D-022 bis code « cours » avant le socle 5 → marqué provisoire (à faire) ; branches périmées dans le lot 8 → signalé à l'intégrateur (à faire) ; cadence → débit à mesurer et annoncer chaque semaine (fait, §7) ; CI par PR → adopté pour les prochaines intégrations (à faire) ; trop de branches en parallèle sur les mêmes fichiers → **cause structurelle traitée par D-026 (architecture en briques)**.

## 7. Débit
Mesure : lots arrivés sur `ENT` par jour, et lots de code terminés sur branche par jour. Au 2026-11-07 : 1 intégration sur ENT en 1,5 jour (lot 6) ; environ 6 lots de code terminés par jour sur branche.
