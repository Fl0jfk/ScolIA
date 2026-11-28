# Socle 21 — lot V3f « barème détaillé, conversion des notes, duplication » : livraison

Date : 2026-11-28. Branche `claude/project-thread-j72q4c`, base `origin/ENT` 06333a59. Conception : `docs/socles/21_EVALUATIONS.md` §8.9, §17.2 (EVA-118, EVA-125), §17.15 (EVA-262 à EVA-264, M-667), cas limites CE-39 et CE-83, tableau des lots §23.3 (« V3 complément »).

**Niveau : implémenté et testé en local (PostgreSQL 16, feature `labo`). La CI est à lire sur la PR. Ce lot n'est pas validé par Florian et n'est pas prêt pour le pilote.**

## Pourquoi ce lot maintenant

Les PR ouvertes du socle 21 couvrent V14, V6, V7, V8, V10 et la partie déterministe de V11 (701 à 706). V9 (mobile) attend V7, V15 et V16 attendent V8 et le socle 23, la partie « modèle de langage » de V11 attend l'étude juridique (Q-EV8). Le complément de V3 (§23.3) ne dépend que de V2 et de V3, déjà sur ENT : il part d'ENT et ne touche aucune table des PR ouvertes. Le rapport de V3 (§5 point 3) le listait comme « non fait » : barème détaillé, conversion des notes à un changement de barème, duplication multiple.

## Ce qui est livré (faits)

### Migration 707 `eva_v3f_bareme_detaille`

Additive et rejouable, en fin de `MIGRATIONS` (règle 38) ; aucune migration versée n'est modifiée (462 et 464 intactes) ; `MIGRATION_TAG` inchangé (3808) ; ses deux fonctions portent `SET search_path = pg_catalog, public, pg_temp` et ne lisent que des relations `public.`. **Aucune table nouvelle** : les tables du barème détaillé (`eva_bareme_item`, `eva_resultat_item`) existent depuis V1 mais n'étaient écrites par aucun code.

- `eva_evaluation.detail_visible_eleve` (faux par défaut) : le professeur montre ou non le détail par exercice (EVA-264).
- `eva_bareme_item` : `annule_le`, `annule_par` (contrainte : un exercice annulé porte sa date). Déclencheur `eva_bareme_item_garde` : un exercice naît non annulé ; annulé, il le reste et sa trace ne change pas ; un exercice qui porte des points ne change plus (ni libellé, ni maximum, ni rang) et ne se supprime pas ; troncature interdite.
- `eva_resultat_item` : `resultat_version` (version de la note que ce détail compose), `saisi_par`, `modifie_le`. Déclencheur `eva_resultat_item_garde` : l'exercice et la note sont de la même évaluation, les points ne dépassent jamais le maximum, aucun point ne s'écrit sur un exercice annulé, aucune suppression physique, troncature interdite.

### Service (`ent-db/src/evaluations_v3f.rs`) et routes (`ent-api/src/evaluations_v3f.rs`, 9 points d'entrée)

| Route | Action | Qui |
|---|---|---|
| `GET /eva/v3f/evaluations/{id}/bareme` | barème détaillé et **réussite par exercice** (EVA-263) : effectif, moyenne, répartition en quatre tranches du maximum, sur les seuls détails à jour ; jamais de nom ni de rang | comme la grille : professeur du cours, direction et secrétariat en lecture |
| `PUT /eva/v3f/evaluations/{id}/bareme` | définir les exercices (EVA-262) : la somme des maximums doit égaler le barème ; redéfinissable tant qu'aucun point n'est saisi ; liste vide = pas de barème détaillé | propriétaire de l'évaluation |
| `PUT /eva/v3f/evaluations/{id}/bareme/visibilite` | montrer ou cacher le détail à l'élève et à sa famille (EVA-264) | propriétaire |
| `GET /eva/v3f/evaluations/{id}/details` | grille de détail : points par exercice, note, version, détail à jour ou non | comme la grille |
| `POST /eva/v3f/evaluations/{id}/details` | saisir le détail de plusieurs élèves : la **note écrite est la somme**, par la voie de la grille (`eva_v3_une_case` : droit, période de l'élève, version attendue, historique, événement neutre) ; chaque élève est indépendant | comme la saisie de la grille |
| `POST /eva/v3f/evaluations/{id}/conversion/apercu` | aperçu d'une conversion : nouveau barème (CE-39) **ou** exercice à annuler (CE-83) ; notes avant / après, empreinte ; n'écrit rien | propriétaire ; après l'arrêt, la direction |
| `POST /eva/v3f/evaluations/{id}/conversion` | appliquer : même demande, même empreinte, motif ; **tout ou rien** | idem |
| `POST /eva/v3f/dupliquer` | dupliquer plusieurs évaluations (de l'an passé ou d'un autre de ses cours) dans un de ses cours, **sans résultats** (EVA-125, M-667) ; tout ou rien | saisie sur le cours cible ; la source lisible par l'acteur à **sa** date |
| `GET /eva/v3f/eleves/{eleve}/evaluations/{evaluation}/detail` | le détail d'une note **publiée**, **montré** et **à jour** | l'élève, ou un lien « voir les notes » ; sinon 403 ; rien à montrer : 404 |

Règles tenues :
- **Un détail n'est vrai que pour la version de la note qu'il compose.** Une note changée à la main dans la grille rend le détail « périmé » : il sort de la réussite, il n'est plus montré à la famille, la grille de détail le signale. Ressaisir le détail le remet à jour.
- **Conversion** : barème relevé (10 → 20) = barème d'abord, notes ensuite ; barème réduit = notes d'abord (la base refuse une note au-dessus du barème). Chaque note est réécrite par `ecrire_resultat_cause_tx` avec la version lue (l'historique garde l'ancienne valeur et le motif) ; le barème change par une correction motivée, journalisée par la base (M4, `eva_evaluation_historique`). Une note qui ne change pas n'est pas réécrite (élève qui avait 0 à l'exercice annulé).
- **Exercice annulé (CE-83)** : note d'un élève au détail à jour = somme des exercices restants ; note sans détail (ou détail périmé) = règle de trois arrondie au pas de saisie de l'établissement ; le détail suit la note convertie (même version) ; l'exercice annulé garde sa trace et ne compte plus.
- **Empreinte** : version de l'évaluation, barèmes avant et après, exercice, et pour chaque note (élève, version, valeur avant, valeur après). Toute écriture entre l'aperçu et la confirmation (une note corrigée, un barème touché) la rend fausse : rien n'est écrit (409). Un aperçu ne se rejoue pas.
- **Droits** : mêmes règles que le réglage d'une évaluation (V3, M3, CE-44) : en période ouverte, le propriétaire (titulaire, co-enseignant ; un suppléant pour ses seules évaluations) ; si une partie du public est arrêtée, seule la direction (`notes.corriger`), et chaque note passe par la règle de l'écriture (correction motivée après l'arrêt). Un élève protégé que l'acteur ne voit pas empêche la conversion (« revient à la direction »).
- **Famille et élève** : aucune route du personnel (403, testé en HTTP sur les 8 points d'entrée du personnel) ; la lecture du détail exige la note publiée (vue `eva_resultat_publie`), le choix du professeur et un détail à jour, sous `actor_can_read_notes` et la garde de l'élève protégé.
- **Événements neutres** (identifiant de l'évaluation seulement) : `evaluations.bareme_detaille_defini`, `evaluations.notes_converties`, et `evaluations.creee` / `evaluations.modifiee` existants.

### Types (`ent-types/src/evaluations_v3f.rs`, compile pour `wasm32`)
DTO et trois fonctions pures testées : `arrondir_au_pas` (demi vers le haut), `convertir_proportion` (règle de trois exacte en entiers 128 bits, bornée au nouveau barème), `tranche` (quart du maximum).

## Décisions prises par défaut (D-069, à faire valider)

1. **Somme des maximums = barème**, exigée à la définition ; sinon refus (« changez d'abord le barème »). Pas d'ajustement automatique du barème.
2. **Pas de changement de barème direct sur un barème détaillé** : on annule un exercice ou on redéfinit les exercices (tant qu'aucun point n'est saisi).
3. **Arrondi de la conversion** : au pas de saisie de l'établissement, demi vers le haut. Le professeur voit chaque note avant / après et ne valide qu'en bloc ; il corrige ensuite une note à la main s'il le souhaite.
4. **Un détail ne garde pas d'historique propre** : l'historique est celui de la note (`eva_resultat_historique`). Les points actuels se remplacent, sans suppression.
5. **Duplication** : publication explicite, non annoncée, public entier (une restriction de public ne suit pas), sous-matière seulement si le cours cible enseigne la même matière, exercices non annulés, choix « détail montré » repris. Une évaluation annulée ou de positionnement ne se duplique pas.
6. **Réussite par exercice** : pas de seuil d'effectif (vue du seul personnel, comme les statistiques de la grille).

## Ce qui n'est pas fait

- **Écrans Leptos** (grille de détail, aperçu de conversion, duplication) : API seule.
- **QCM corrigés automatiquement** (EVA-265, socle 20) : dépend du socle 20.
- **Déplacement vers un autre cours** (CE-51) : la base l'interdit depuis V1 (annuler et recréer) ; non repris.
- **Avis « note modifiée » à la famille** après une conversion d'une évaluation publiée (CE-39) : l'historique est là, l'affichage relève du relevé famille (V7, PR ouverte).
- **Synthèse agrégée par exercice pour l'IA** (§13.8) : sous l'étude juridique.

## Tests (locaux)

- `ent-types` : 3 tests (arrondi, règle de trois, tranches).
- `ent-db` (`tests_evaluations_v3f.rs`, feature `labo`) : 6 scénarios — note = somme et réussite sans classement, gardes en base, détail périmé puis remis à jour ; droits (propriétaire, direction en lecture, autre professeur, famille, autre établissement) ; famille (non publiée, non montrée, lien sans notes, détail périmé) ; CE-39 (aperçu, motif, empreinte périmée, historique de la note et du barème, aperçu non rejouable, autre professeur) ; CE-83 (somme des restants, règle de trois, note inchangée non réécrite, détail qui suit, exercice annulé figé en base, refus du changement de barème direct) ; duplication (exercices copiés sans résultats, tout ou rien, autre professeur, évaluation annulée).
- `ent-api` : `separation` (inventaire des routes : +1 source, 77 → 78 ; nouveau test 403 sur les 8 routes du personnel et la lecture famille).

## Compteurs (règle 28)

`tests_rls.rs` inchangé (aucune table) ; registre des données inchangé en nombre (`eva_resultat_item` : `saisi_par` hors export) ; `Action::TOUTES` inchangé ; sources de routes 77 → 78 sur la base d'ENT ; `TABLES_ELEVE` inchangé ; `MIGRATION_TAG` inchangé (3808).

## Base utilisée

PostgreSQL 16 local du conteneur, base neuve dont le nom contient « test ». Aucune base de production, aucun secret, aucune donnée réelle.
