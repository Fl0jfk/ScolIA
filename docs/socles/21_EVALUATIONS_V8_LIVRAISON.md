# Socle 21 — lot V8 « fin de période » : livraison

Date : 2026-11-28. Branche `claude/project-thread-oewt41`, base `origin/ENT` 06333a59. Conception : `docs/socles/21_EVALUATIONS.md` §9 (EVA-178 à EVA-193, règles R-V9, R-V10, R-V11, R-03, CE-63).

**Niveau : implémenté et testé en local (PostgreSQL 16, feature `labo`). La CI est à lire sur la PR. Ce lot n'est pas validé par Florian et n'est pas prêt pour le pilote.**

## Ce qui est livré (faits)

### Migration 704 `eva_v8_fin_de_periode`

La migration est additive et rejouable. Elle a été insérée en fin de `MIGRATIONS` (règle 38). Aucune migration versée n'a été modifiée, et `MIGRATION_TAG` est inchangé (3808).

- **`eva_instantane_appreciation`** : appréciations validées figées dans un instantané, avec leur version. La table est en ajout seul (`ent_trg_eva_ajout_seul`), unique sur (instantané, appréciation). Elle complète `eva_instantane` et `eva_instantane_ligne` (463), qui ne figeaient que les moyennes.
- **`eva_correction_demande`** (table de 463, étendue) : demande d'un professeur sur un résultat existant, avec :
  - la version vue (compare-et-écrit) ;
  - le statut et la valeur proposés ;
  - l'effet calculé à la décision (moyenne matière et moyenne générale, avant et après) ;
  - la version écrite ;
  - le motif de refus et le retrait.

  Une seule demande peut être en attente par résultat (index partiel). Un déclencheur de garde impose les règles suivantes :
  - aucune suppression et aucune réécriture de la demande ;
  - une seule sortie de l'état « en attente » ;
  - un refus exige un motif, une acceptation exige la version écrite ;
  - la troncature est interdite.
- **`eva_reouverture`** : réouverture d'une classe pour une période, avec l'état d'avant, l'état visé (ouverte ou notes arrêtées), un motif de 10 à 500 caractères et une échéance. Au plus une réouverture est en cours par classe et période. La fermeture est posée une seule fois, soit automatiquement à l'échéance, soit par un geste.
- **`eva_relance`** : relances manuelles et rappels J-7 et J-2. On y garde l'instantané des manques au moment de l'envoi (non publiées, cases vides, cours sans évaluation) et le message. La table est en ajout seul, avec un unique par jour pour les relances manuelles et par échéance pour les rappels (pas de doublon quand le passage quotidien rejoue).
- **Fonction `eva_eleves_classe_periode`** : élèves dont la dernière affectation de classe chevauchant la période est cette classe.
- La RLS est posée sur les 3 nouvelles tables (`tests_rls.rs` : +3).

### Service (`ent-db/src/evaluations_v8.rs`), passage quotidien, avis et routes

- **Pilotage de la saisie** (EVA-178, EVA-179) : par professeur et par cours, l'état de saisie montre les évaluations non publiées, les cases vides, les cours sans évaluation et les appréciations manquantes. La relance manuelle est groupée et son message est construit à partir des manques réels. Le professeur est averti par une notification personnelle.
- **Arrêt des notes** :
  - aperçu préalable (EVA-180) ;
  - arrêt manuel par classes (EVA-181) ;
  - changement de la date d'arrêt (EVA-182) ;
  - arrêt automatique à la date.

  **À l'arrêt, tout ce qui n'était pas publié l'est** (R-V9 : motif `publiee_auto_arret`, avis de publication), puis un **instantané versionné** est pris.
- **Instantanés** (R-V11, EVA-185, EVA-186) : moyennes de chaque élève et appréciations validées, avec une empreinte SHA-256. Les fonctions sont :
  - la liste des versions, avec l'indicateur « à jour » (empreinte courante comparée à la dernière) ;
  - la lecture d'une version ;
  - la comparaison de deux versions (écarts par élève et appréciations changées) ;
  - une nouvelle prise si quelque chose a changé.

  Toute écriture postérieure à l'arrêt (correction, rattrapage) crée automatiquement une nouvelle version.
- **Rattrapages** (R-V10, EVA-183) : la direction peut autoriser une saisie après l'arrêt pour des élèves et une évaluation, jusqu'à la veille du conseil au plus tard.
- **Clôture des appréciations** (EVA-187) : par geste, ou automatiquement la veille du conseil quand l'arrêt automatique est actif et qu'aucune réouverture n'est en cours.
- **Réouverture** (EVA-184) : elle demande un motif, une durée (30 jours au plus) et des classes (60 au plus par geste). Elle revient à « ouverte » ou à « notes arrêtées ». À l'échéance, le passage quotidien referme la classe, ré-arrête les notes et reprend un instantané.
- **Corrections** (EVA-188 à EVA-191) :
  - le professeur demande une correction en indiquant la version qu'il a vue ;
  - la direction voit l'**effet chiffré** avant de décider : moyenne matière et moyenne générale, avant et après, et seuils franchis (8, 10, 12, 14, 16) ;
  - elle accepte (l'écriture passe par le moteur V3, en compare-et-écrit) ou refuse avec un motif ;
  - le professeur peut retirer sa demande, et il est averti de la décision ;
  - la direction peut aussi corriger directement ;
  - un **journal des corrections** réunit les demandes et les corrections directes.

  Les corrections ne sont possibles qu'entre l'arrêt et le conseil : avant l'arrêt, le professeur saisit lui-même ; après la clôture de la période, il faut une réouverture.
- **Rappels automatiques J-7 et J-2** (EVA-192) avant la date d'arrêt, envoyés aux seuls professeurs qui ont des manques.
- **Contrôle de 3ᵉ** (R-03, CE-63, EVA-193) : sur la dernière période d'une classe de 3ᵉ, il liste les élèves sans moyenne annuelle dans un enseignement obligatoire, hors dispense. **La clôture de la période est refusée tant qu'il reste un manque.**
- **Passage quotidien** : `eva_v8_passage_au` est appelé par `passages_quotidiens`, avec un compteur `fin_de_periode` dans le bilan.
- **19 routes** (22 couples méthode et chemin) dans `ent-api/src/evaluations_v8.rs`, qui délèguent toutes à `ent-db`. Elles sont ajoutées à `routes_src.rs` (78 sources).
- **Registre des données** : 3 entrées. La première est `eva_instantane_appreciation`. Les deux autres sont `eva_relance` et `eva_reouverture`, toutes deux classées « personnel ». Pour `eva_correction_demande`, les demandes de professeur ne sont pas exportées (`export_si statut_propose IS NULL`), et les nouvelles colonnes sont hors export.

## Décisions prises par défaut (D-069, à relire par Florian)

1. **Droits** : sans nouvelle `Action`, `Action::TOUTES` est inchangé.
   - L'arrêt, la réouverture, la clôture, les instantanés et la décision sur une correction relèvent de `PeriodesVerrouiller`.
   - Les rattrapages et la correction directe relèvent de `NotesCorriger`.
   - Le pilotage de la saisie est ouvert au personnel de direction et au professeur principal de la classe.
   - Une demande de correction est faite par le professeur du cours.
2. À l'arrêt, **toutes** les évaluations non publiées sont publiées, y compris celles qui ont été retirées ou qui sont en cours de correction. Ce choix suit la lettre de R-V9 et évite un trou dans le bulletin.
3. Une demande de correction ne porte que sur un **résultat existant** : on ne crée pas de case à partir d'une demande.
4. La réouverture dure **30 jours au plus**, ce qui empêche une classe de rester ouverte indéfiniment.
5. Les seuils signalés dans l'effet d'une correction sont 8, 10, 12, 14 et 16 (/20). Ils ne sont pas paramétrables dans ce lot.
6. La clôture automatique des appréciations suit le réglage `arret_auto` de la période : pas de réglage distinct.
7. La 3ᵉ est détectée par le code ou le libellé du niveau (motif `3e`/`3ème`/`troisième`). La moyenne annuelle d'un enseignement est présente dès qu'une période en a une. Le contrôle exclut les options facultatives, les dispositifs et l'accompagnement.
8. Le passage à « conseil tenu » reste fait par le socle 23 (conseils) ou par la fonction V1. V8 ne clôt la période qu'à partir de cet état.
9. Le bulletin v2 (export figé à partir de l'instantané) relève du socle 23 et n'est pas dans ce lot.

## Tests

- 4 scénarios en base (`ent-db/src/tests_evaluations_v8.rs`) :
  - arrêt, publication, instantané, correction directe et réouverture ;
  - demandes de correction, effet et décision ;
  - pilotage, relances, rappels, rattrapage et clôture automatique ;
  - contrôle de 3ᵉ bloquant la clôture.
- 5 tests unitaires : message de relance, seuils franchis, écarts entre versions, stabilité de l'empreinte et fenêtre de correction.
- Séparation : les 22 points sont refusés (403) aux parents (dont un autre foyer et un parent sans droits) et aux élèves.

## Ce qui reste (non fait)

- Les écrans Leptos du tableau de bord de fin de période. Ce lot ne livre que l'API.
- Des seuils paramétrables par établissement.
- Le rapprochement avec le conseil de classe (socle 23) : le passage à « conseil tenu » et le bulletin v2 lu depuis l'instantané.

## Lien avec les PR ouvertes

Ce lot est indépendant de 701 (V14), 702 (V6) et 703 (V7) : il ne touche aucune de leurs tables. Il peut être versé dans n'importe quel ordre.
