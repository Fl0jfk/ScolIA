# Socle 3 — Lot I5 (décisions, commission, liste d'attente, propositions, conditions) : livraison

Branche `ent-socle-3-code-suite` (partie d'`origin/ENT` 02659006, fusionnée avec `origin/ENT` avant la CI, règle 28).
Conception : `03_INSCRIPTIONS.md` §7.1, §8.7, §11.4, §11.5, §12.2, §15.3. Lots précédents : I0 à I4 (`INSCRIPTIONS_I3_I4.md`).
D-068 : le Next.js n'a ni liste d'attente, ni commission, ni proposition de place : **création pure**.

Statut : **implémenté et testé en local** (PostgreSQL 16 jetable, base « test ») ; **non validé** tant que la CI n'a pas
rendu son verdict ; non prêt pilote, non prêt production. **Aucun écran** (I14d : commission et attente).

## Ce qui est livré

| Domaine | Contenu |
|---|---|
| Décisions (INS-04, 33, 65) | cinq sens (`admis`, `admis_sous_reserve`, `liste_attente`, `refuse`, `ajourne`) ; **motif interne obligatoire** ; **ajout seul** (déclencheur : une revue = une nouvelle ligne qui remplace la précédente, rien ne s'écrit sur l'ancienne, suppression directe refusée, la purge du dossier passe) ; revue obligatoirement motivée et distincte ; offre alternative ; capacité sous verrou, **dépassement confirmé et tracé** ; l'ancienne route `/decision` passe par la même fonction |
| Publication (INS-62) | une décision est prise puis **publiée** : tout de suite, ou à l'instant choisi par la direction (confirmation en plage calme 20 h–8 h et dimanche, heure de l'établissement) ; tant qu'elle ne l'est pas, **la famille n'en voit rien** (état, suivi, rang, conditions, échéance) ; la tâche de fond (5 s) ne publie que ce qu'une personne a programmé ; message à la famille **sans le contenu** ; courrier : projet texte, jamais le motif interne |
| Avis et déport (INS-31, 63) | avis internes en ajout seul (la direction les lit tous, un autre agent les siens) ; déport : le dossier disparaît des listes, de ses avis, de ses actions, il ne décide ni ne publie plus |
| Commission (INS-32) | séance, dossiers examinés, **places restantes en direct**, décisions en lot (une transaction par dossier, récapitulatif, jamais de revue silencieuse), publication groupée, clôture avec procès-verbal (texte) sans décider les dossiers non examinés |
| Liste d'attente (INS-34) | rangs sans trou ; place libérée (désistement, refus, revue, expiration, autre offre) → **le système désigne** le suivant, rien ne part ; une personne **confirme l'envoi** (= la décision « admis », auteur humain), peut **passer son tour** (motif) ou **constater** un délai dépassé (jamais automatique, `sans_suite`) ; un dossier qui vient de quitter l'offre n'est pas re-désigné ; dossier bloqué pour désaccord jamais désigné |
| Famille (INS-35) | lit la décision **publiée** seulement (motif communiqué, réserve, sa position, sa proposition, ses conditions génériques) ; répond (accepter / décliner = désistement, place libérée) ; déposant vérifié uniquement |
| Conditions | par campagne, aucune par défaut (D-069 : comportement d'avant conservé) parmi réponse de la famille, contrat, frais, acompte, situation financière, passage ; **réserve** propre à la décision ; accord des responsables (I4) et pièces bloquantes (I3, quarantaine : une pièce non contrôlée ne compte pas) **calculés** ; tout rempli → `confirme` (conséquence mécanique) ; la validation exige le tout ; levée avec motif par la direction |

Routes : `ent-api/src/inscriptions_decisions.rs` (26 routes personnel, 2 famille). Événements : `inscription.decisions_publiees`,
`place_proposee`, `reponse_famille`, `commission_tenue`, `deport` (sans donnée personnelle).

## Migrations
**313** `inscription_decision_i5` (plage 313–329 du socle 3 ; la plage de repli 3330–3339 n'est pas utilisée ; 314–329 libres),
appliquée après 312 ; `MIGRATION_TAG` inchangé (3808). RLS posée sur 7 tables (compteur `tests_rls.rs` +7). Registre des données : 6 tables.
Compteurs (règle 28) : `Action::TOUTES` inchangé (aucune action ajoutée), `SOURCES`/`NOMS_SOURCES` 69 → 70.

## Décisions de conception (à relire)
1. **Pas de nouvelles actions** `inscriptions.avis` / `avis.lire` (§12.2) : elles toucheraient le catalogue et son oracle ; l'avis exige `inscriptions.gerer`, la lecture complète `inscriptions.decider`. À faire avec le lot des droits.
2. **L'envoi d'une place est la décision « admis »** (auteur humain, motif, publication immédiate) ; décliner = désistement (pas de retour en liste).
3. **Délai de réponse** : celui de la campagne, 8 jours par défaut (valeur prudente du logiciel) ; décalé par les fermetures.
4. Décisions de commission : **non publiées par défaut** (publication groupée décidée par une personne).
5. La route historique `/decision` **communique encore le motif** par défaut (comportement d'avant) ; les nouvelles routes ne communiquent que le motif communiqué explicite.
6. Le suivi lu par la famille ne contient plus le motif interne (seulement le motif communiqué).
7. Test existant adapté : `attaque_email_parent_existant_aucun_acces` exclut les messages `decision-*` du compte d'envois.

## Revue indépendante (review-sonnet) — traitée
Corrigés : masquage des décisions non publiées dans l'espace, la démarche, la vue de décision et le suivi (chaîne comprise) ;
déport appliqué à la validation, l'instruction, les accords, les pièces, la publication ; désignation relancée quand une désignation tombe
ou quand un confirmé change d'offre ; tâche de publication une transaction par décision, sans les décisions remplacées ; lot sans revue
silencieuse ; levée d'une réserve réservée à la direction ; FK proposition → décision en cascade ; verrou du candidat (`SKIP LOCKED`) ;
expiration refusée hors état `admis`.
**Restes connus** : `deposer_piece_famille` répond « dossier clos » sur l'état réel (peut trahir un refus non publié) ; le message part vers le
responsable de rang 1 (pas vers le seul déposant) ; ordre des verrous proposition/dossier (risque de deadlock, pas d'incohérence) ;
proposition « désignée » d'un dossier devenu bloqué à écarter à la main ; `source_ref` d'une condition non validé ; membres de commission non
validés ; `ADMISSION_DECIDEE` émis à la décision (aucun consommateur) ; recours (INS-66), décisions liées de fratrie (INS-64), dossier de
justification (INS-67), courrier en document du socle 7, paiement/contrat par événements (I7) : **non faits**.

## Garde-fous vérifiés (tests `tests_inscriptions_i5.rs`)
Motif obligatoire et ajout seul ; capacité et dépassement ; réserve et confirmation mécanique ; désignation sans envoi (aucun message avant confirmation) ;
passer son tour, expiration constatée ; revue qui libère la place ; publication programmée et plage calme ; conditions et levée ; pièce en quarantaine ;
commission en lot ; avis et déport (toutes routes) ; famille sans motif interne, avis, second responsable ni autre candidat ; dossier bloqué ; droits et isolation entre établissements ; masquage d'une chaîne non publiée.
`separation.rs` : 25 routes à identifiant visant l'établissement B, jamais un 2xx.

## Vérifications
Locales (PostgreSQL 16 jetable `scolia_test_i5`, `ENT_ENV=test`) : `tests_inscriptions*` (I0 à I5), `registre`, `tests_rls`, `migrate` : 0 échec
(63 tests ciblés après correctifs de revue, 114 sur la passe précédente) ; `ent-api` : `separation` (58, dont `domaine_inscriptions_i5`) et
`recette_securite_3` (10) verts. `cargo clippy --locked` backend (workspace, labo, tests) et `ent-types` : **0 avertissement** ;
rustc : 0 ; `cargo metadata --locked` dans `ent-rust/web` : vert ; fmt ciblé sur les fichiers neufs. La suite complète d'ent-db n'a pas été rejouée
(CI). Les tests I4 `expedier` dépendent d'une file d'envoi partagée : deux rouges en parallèle à 3 fils, verts isolés (même cause qu'avant ce lot).
