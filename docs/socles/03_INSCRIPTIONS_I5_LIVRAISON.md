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


## Correctifs de revue (D-069)

Passe de correctifs sur la branche `ent-socle-3-code-suite` (PR n° 120, brouillon), partie de la tête distante `ee89efed`, fusionnée avec
`origin/ENT` (règle 28 : compteurs recalculés depuis la base commune ; `Action::TOUTES` et `SOURCES` inchangés, **compte RLS +1**
dans `tests_rls.rs`, `MIGRATION_TAG` inchangé). **La migration 313 n'étant pas versée, elle est modifiée** (aucune autre migration ne change).
`Cargo.lock` : `url` 2.5.9 (RUSTSEC-2026-0592), correctif de la PR 121 reproduit à l'identique, commit séparé.

Statut : **implémenté et testé en local** (PostgreSQL 16 jetable, base « test ») ; **non validé** tant que la CI n'a pas rendu son verdict ;
non prêt pilote, non prêt production.

### Règles générales mises en œuvre

| Règle | Mise en œuvre |
|---|---|
| 1. Une décision non publiée n'existe pas pour la famille | Chaque ligne de suivi qu'une décision écrit (changement d'état, motif communiqué) est **rattachée** à cette décision (`ent_inscription_suivi_dossier.decision_id`, posée par `decider_tx`) ; la famille ne la lit **que si cette décision est publiée**, aujourd'hui comme après une décision ultérieure. **Une seule fonction d'état visible** (`visibilite_famille_tx`, chaîne de décisions) : elle masque l'état, le rang, l'échéance, la proposition, les conditions **et autorise** les actions de la famille. `ADMISSION_DECIDEE` n'est plus émis qu'à la publication. |
| 2. Un membre lié est écarté d'office | Fonction SQL `ent_inscription_membre_lie` (compte ou adresse d'un responsable, du déposant, d'une invitation ; compte ayant déjà la charge de l'élève ; fiche de personne identique ou fusionnée) ; **garde unique** `ecarte_tx` (déport déclaré **ou** lien) appelée par chaque route du personnel à identifiant de dossier (`refuser_si_ecarte`, `dossier_visible_tx`, `decider_tx`, publication) ; **garde en base** : déclencheurs sur l'avis et la décision. Le refus est « introuvable » (le dossier n'existe pas pour ce membre). Un déport déclaré **marque** les avis déjà donnés (`auteur_deporte`). En séance, seul un **membre de la commission** décide (membres validés : personnel de l'établissement, en poste). |
| 3. Inscription définitive après décision publiée et acceptée | `valider` exige la dernière décision de la chaîne **publiée** (sens d'admission) et refuse tant qu'une proposition est `designee` ou `envoyee` (acceptation de la famille attendue). |
| 4. Ajout seul, ordre certain | L'ordre des décisions suit la chaîne `remplace_decision_id` (une racine par dossier, un seul successeur par décision : index uniques ; lecture récursive), jamais `now()` ; UPDATE, DELETE **et TRUNCATE** gardés sur décisions, avis, déports, levées ; **un seul chemin de purge** : `SET LOCAL ent.purge_etablissement = 'on'` (convention du dépôt, jamais posé par l'application) à la place de `pg_trigger_depth() >= 2`. |
| 5. Ordre des contrôles | Personnel, droit, existence : le droit précède désormais la garde de déport/lien ; chaque test de refus exige le refus exact avec un compte qui a le droit. |

### Correctifs

| N° | Correctif | Fait |
|---|---|---|
| E1 | suivi rattaché à la décision, jamais lu avant (ni après) sa publication | fait, test `r_e1_refus_non_publie_jamais_visible` |
| E2, M1 | lien = écart d'office sur **toutes** les routes à identifiant (liste ci-dessous), garde en base, séance réservée aux membres | fait, `r_e2_membre_lie_ecarte_de_toutes_les_routes`, `r_e2_seance_membres_et_plafonds` |
| M2 | actions de la famille (`retirer`, `desister`, pièce, réponse à la place) : tant qu'une décision plus récente que la dernière publiée existe, refus **neutre** (message d'une modification concurrente, rien de l'état) ; rien n'est exécuté | fait, `r_m2_actions_famille_sur_etat_visible` |
| M3 | règle 3 | fait, `r_m3_…` (les tests I5 existants donnent désormais la réponse de la famille avant la validation) |
| M4 | lectures de « dernière décision » par la chaîne (`CHAINE`) ; conflit de décisions concurrentes classé en conflit | fait, `r_m4_chaine_et_concurrence` |
| M5 | admission directe sur une place désignée : confirmation explicite + motif (`confirmer_passer_devant`, `motif_passer_devant`), désignation reprise annulée, tracée sur les deux dossiers, le dossier garde son rang | fait, `r_m5_…` |
| M6 | proposition `envoyee` jamais réécrite : annulée (motif) et nouvelle proposition `designee` ; réponse de la famille réinitialisée, avec trace | fait, `r_m6_m7_…` |
| M7 | levée d'une condition journalisée (`ent_inscription_condition_levee`, ajout seul : code, motif, auteur) ; réinitialisation ou retrait de la condition tracés au suivi interne | fait, `r_m6_m7_…` |
| M8 | `motif_interne` (historique des décisions, `decision_motif` du dossier) réservé à `inscriptions.decider` | fait, `r_m8_…` |
| M9 | TRUNCATE gardé, chemin de purge explicite, tests d'attaque SQL directs (avis, déport, décision, levée) | fait, `r_m9_ajout_seul_attaques_sql` |
| M10 | `i5_deport_ferme_toutes_les_routes` exige le refus exact ; tests de concurrence (deux envois, acceptation contre expiration, deux décisions) | fait, `r_m10_…`, `r_m4_…` |
| M11 | « passer son tour » ne vaut que pour la place concernée (exclusion limitée à la désignation en cours, plus d'exclusion durable) ; un dossier écarté redevient désignable à son rang pour une place ultérieure | fait, `r_m11_…` (documenté) |
| F1 | ordre des verrous unique : dossier, puis proposition (envoi, passage de tour, constat, réponse, publication) ; 40P01 et 40001 classés en conflit (message neutre) | fait |
| F2 | rang lu par la famille calculé sur les seules décisions publiées | fait, `r_f2_…` |
| F3 | plafonds sur `modifier_dossiers_commission` (500 par modification et par séance), membres validés, `publier_le` borné à 90 jours | fait, `r_e2_seance_…` |
| F4 | `maintenant` en paramètre : constat et levée de condition, date de décision du dossier ; l'exclusion « décidé dans la transaction » (`decide_le = now()`) est remplacée par une liste explicite | fait |
| F5 | ordre des contrôles : droit avant garde de déport | fait |
| F6 | récapitulatif de lot : messages de la personne seulement, aucune erreur technique | fait, `r_f6_…` |
| F7 | `ADMISSION_DECIDEE` émis à la publication seulement | fait |
| F8 | export à la personne concernée : sans avis, sans motif interne (ni auteurs, rang, motifs de proposition ou de condition, texte du suivi) ; **règles explicites** : le déport (identité d'un agent), les levées de condition et le procès-verbal de commission (document collectif) ne sont pas exportés | fait (registre de données) |
| F9 | une famille qui répond **après** le délai : la réponse est reçue tant qu'une personne n'a pas constaté l'expiration (le constat n'est jamais automatique, `sans_suite`) ; si le constat passe d'abord, la réponse est refusée | **documenté**, comportement conservé |

Routes à identifiant de dossier couvertes par `r_e2_membre_lie_ecarte_de_toutes_les_routes` (deux variantes de lien : déposant, second
responsable) : dossier, statut, pièce, commentaire, relance, décision (ancienne et avancée), validation, transition, blocage, ne pas
solliciter, annulation de validation, correspondances, accords (lecture, demande, papier, désaccord, dispense, levée), pièces (lecture
attendues, contrôle, lecture, guichet, réutilisation), décisions (lecture, décider, courrier, publier), avis (donner, lire), déport (état, se
déporter), conditions (lire, réévaluer, remplir, lever) ; propositions et liste d'attente passent par `verrouiller_proposition_tx` /
`dossier_visible_tx` ; listes de dossiers, de propositions, de séance et d'attente filtrées.

### Limites et points à relire (non faits ou décidés)
* Pendant qu'une décision plus récente que la dernière publiée existe, **la famille ne peut rien faire** (retrait, désistement, pièce, réponse) :
  c'est le prix de ne rien révéler ; une décision programmée longtemps à l'avance (jusqu'à 90 jours) bloque donc ces actions.
* La validation exige toujours l'acceptation de la place par la famille : au guichet, la direction peut **lever** la condition `reponse_famille`
  (si la campagne l'exige), mais il n'existe pas encore de saisie « accepté au guichet » d'une proposition : à ouvrir avec le lot des droits.
* Le lien « deux comptes d'une même personne » ne vaut que lorsqu'il est **connu** (même fiche ou fiche fusionnée) ; deux comptes d'adresses
  différentes sans fiche commune ne sont pas détectables.
* La liste de dossiers du personnel évalue le lien dossier par dossier (fonction SQL) : à mesurer sur de gros établissements (index sur
  `lower(email)` à envisager).
* Les listes des I5 non couvertes restent : recours (INS-66), décisions liées de fratrie (INS-64), dossier de justification (INS-67), courrier en
  document du socle 7, paiement/contrat par événements (I7).

### Vérifications locales
PostgreSQL 16 jetable (`scolia_test_i5c`, `ENT_ENV=test`), `--locked`, un seul `CARGO_TARGET_DIR`, 3 fils :
* `ent-db` (labo) : `tests_inscriptions*` (I0 à I5 : **133 verts**, dont 30 pour I5 = 16 d'origine + 14 de revue), `registre`, `tests_rls`, `migrate`,
  gardes statiques (`garde_*`, `isolation_sql_statique`, `jour_utc_interdit`) : 0 échec. Les 21 tests I0 à I4 qui validaient juste après l'admission
  ont reçu un raccourci de test (`famille_a_accepte`) : la validation attend désormais la réponse de la famille (règle 3).
* `ent-api` (labo) : `separation` (58) et `recette_securite_3` (10) verts.
* Clippy 1.99.0, les trois commandes de la CI (`--workspace --all-targets`, avec `ent-api/labo,ent-lab/labo`, `-p ent-types`) : **0 avertissement** ;
  rustc : 0 ; `cargo metadata --locked` dans `ent-rust/web` : vert ; fmt ciblé sur le seul fichier neuf (`tests_inscriptions_i5_revue.rs`).
* La suite complète d'`ent-db` n'est pas rejouée en local (CI). Les tests qui consomment la file d'envoi partagée (`expedier`) restent sensibles au
  parallélisme (même cause qu'avant ce lot).

### Compteurs (règle 28)
`Action::TOUTES` inchangé (aucune action ajoutée) ; `SOURCES`/`NOMS_SOURCES` inchangés (70) ; **compte RLS : I5 = +8** (7 d'origine, plus
`ent_inscription_condition_levee`) ; registre des données : 1 table de plus (`condition_levee`), colonnes hors export déclarées ;
`MIGRATION_TAG` inchangé (3808) ; migration 313 modifiée (non versée).

### Tête et CI
Tête de code poussée : `40264c98` (après la fusion d'`origin/ENT` `94c22c66` et le correctif `url` `f6bcda93`) ; PR n° 120 laissée en **brouillon**.
CI : voir la ligne ci-dessous, complétée à la réception du verdict.

CI_À_COMPLÉTER
