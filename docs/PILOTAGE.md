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
5. **Une seule file d'intégration**, dans l'ordre ; pas deux intégrations en parallèle sur des bases différentes.
6. **Rien sur `ENT` sans les quatre jobs CI verts** (formatage, audits, backend, frontend) et un contrôle du pilote (journal des commits, `MIGRATION_TAG`, aucun test ignoré ajouté, décisions de Florian respectées).
7. **Au plus 6 à 7 sessions actives** ; au-delà, la limite d'usage coupe tout.
8. **Archiver une session** dès que son travail est poussé et qu'elle n'attend plus rien.
9. **Tout ce qui compte s'écrit dans le dépôt** (décisions, état, leçons), jamais seulement dans une conversation.

## 3. Cycle d'un socle (D-021)
Conception (Opus) → personas sur la conception (Opus) → lots de code (Sonnet) → intégration (file) → recette par personas sur le logiciel (Opus) → relecture de Florian. Un socle n'est « fini » qu'après la recette.

## 4. File d'intégration et état (à tenir à jour)

État au 2026-11-07, 11 h UTC :
- **Sur `ENT`** : socle 1, lot 6 (correctifs de la revue de sécurité, inscriptions corrigées I0, cahier de textes, documentation des socles).
- **Lot 7** (`ent-integration-lot7`) : évaluations, finances, communication, santé désactivée par défaut, emploi du temps ; vert en local, en validation CI GitHub.
- **Lot 8** (`ent-integration-lot8`) : socle 2 E0–E2, socle 4 A0 + A0 bis, failles faibles socle 1, D-022, mobile ; démarre sur le lot 7. Attention (contradicteur) : `ent-mobile-app` et `ent-socle1-revue-faibles` sont très en retard sur leur base ; `ent-d022-bis-enseignements` contient déjà `ent-d022-portee-groupes` et le lot 7 : ne pas intégrer D-022 deux fois.
- **Ordre de la file après le lot 8** (une branche à la fois, chacune repart de la nouvelle tête d'ENT) : E3-E4 → A1 → I1 → D-022 bis → correctifs de la recette. La recette personas 1 porte sur le socle 1 et le lot 7, **pas** sur les socles 2 à 4 : une recette P1 sera relancée sur une branche contenant E4, A1 et I1.
- **Branches concernées** : `ent-socle-inscriptions-i1` (socle 3 I1), puis à venir `ent-socle-eleve-e3-e4`, `ent-socle-administratif-a1`, `ent-d022-bis-enseignements`, `ent-recette-personas-1`.

## 5. Transmission à un futur pilote
Un pilote qui reprend : lire ce document, `CLAUDE.md`, `docs/DECISIONS.md` (les décisions de Florian font foi), `docs/ROADMAP.md` (jalons de preuve), `docs/SOCLES.md` (carte), `docs/socles/` (conceptions), `docs/PROJECT_STATUS.md` (état technique), `docs/DECISIONS_AND_FAILURES.md` (erreurs à ne pas refaire) ; lister les sessions et les branches ; reprendre la file du §4. Les conceptions et les décisions sont indépendantes du modèle : un modèle plus récent peut les approfondir sans les refaire.

## 6. Objections du contradicteur
Chaque revue du contradicteur est résumée ici avec le traitement de chaque objection (fait / écarté et pourquoi).
- **2026-11-07 (revue 1)** : recette 1 sur le mauvais code → recadrée, recette P1 à prévoir (fait) ; lignées divergentes → ordre de file écrit, chaque lot repart de la tête d'ENT (fait) ; `perimetre.rs` modifié deux fois → D-022 bis fait foi, le correctif de la recette sera reporté à la main avec revue (à faire) ; registre des migrations faux → corrigé (fait) ; D-022 bis code « cours » avant le socle 5 → marqué provisoire (à faire) ; branches périmées dans le lot 8 → signalé à l'intégrateur (à faire) ; cadence → débit à mesurer et annoncer chaque semaine (fait, §7) ; CI par PR → adopté pour les prochaines intégrations (à faire) ; trop de branches en parallèle sur les mêmes fichiers → **cause structurelle traitée par D-026 (architecture en briques)**.

## 7. Débit
Mesure : lots arrivés sur `ENT` par jour, et lots de code terminés sur branche par jour. Au 2026-11-07 : 1 intégration sur ENT en 1,5 jour (lot 6) ; environ 6 lots de code terminés par jour sur branche.
