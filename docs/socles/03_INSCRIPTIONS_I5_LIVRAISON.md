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

CI sur la tête `39698c2d` : tout vert sauf 3 tranches de tests (2, 7, 8 sur 20), pour une seule cause. Trois tests de recette d'`ent-api`
(`rp2_t02_…`, `rp2_10_…`, `rp2_11_…`) validaient une inscription juste après l'admission, sans que la famille ait accepté la place
(règle 3 : 409 « la famille n'a pas encore accepté la place proposée »). Ces tests n'avaient pas été rejoués en local (seuls `separation` et
`recette_securite_3` l'avaient été). Correction : même raccourci de test que côté `ent-db` (`famille_a_accepte`, SQL de la base de test) dans les deux
fichiers de recette ; aucun code de production touché. Les deux fichiers de recette passent en entier (6 + 11 tests), clippy et fmt ciblé verts.
Verdict CI de la tête finale : voir le message au pilote.

## Passe 2 (contre-revue du pilote)

Partie de la tête distante `6fb4f561` de `ent-socle-3-code-suite` (PR n° 120, brouillon), fusionnée avec `origin/ENT` (messagerie M12 :
aucune migration, aucune action, aucune source ni table RLS de ce côté ; règle 28 : `Action::TOUTES`, `SOURCES`, compte RLS, `MIGRATION_TAG`
relus, inchangés par la fusion). **Migration 313 toujours non versée : modifiée** (aucune autre migration ne change). Statut : **implémenté et
testé en local** (PostgreSQL 16 jetable, base « test », `ENT_ENV=test`), **non validé** tant que la CI n'a pas rendu son verdict.

### Règles tenues
1. **Ce que voit la famille ne dépend que des décisions publiées, rang compris.** Un seul calcul, `rang_famille_tx`, utilisé par `dossier_famille`
   et `decision_famille` : un dossier compte dans la file selon sa dernière décision **publiée** (fonction SQL `ent_inscription_en_liste_visible`) ;
   il reste compté tant que sa sortie de liste n'est pas publiée, n'y entre qu'une fois son entrée publiée ; l'ordre est celui de l'**entrée**
   (`ordre_attente`, jamais renuméroté, contrairement au rang affiché au personnel qui se compacte à chaque départ).
2. **Personnel → droit → existence** partout (balayage de toutes les routes à identifiant de dossier) ; la garde d'écart d'`decider_tx` et
   d'`donner_avis` précède aussi tout constat sur le dossier (nature, blocage).
3. **Un refus se prouve par son refus exact** (variante, SQLSTATE) avec un compte qui a le droit.

### Correctifs
| N° | Correctif | Fait / test |
|---|---|---|
| M-A | rang famille : file calculée sur les décisions publiées (colonnes `ordre_attente`, `offre_attente_id`) | fait ; `p2_ma_rang_famille_sortie_de_liste_non_publiee` (sortie non publiée, entrée non publiée, publication ; `dossier_famille` = `decision_famille`) |
| M-B | `transition` : droit, puis action inconnue, puis écart ; autres routes vérifiées (script sur tous les appels de garde) ; `decider_piece` : droit puis écart avant la forme ; avis et décision : écart avant la nature | fait ; `p2_mb_droit_avant_ecart` (sans droit et lié → 403 exact ; avec droit et lié → introuvable) ; le test de `transition` de `r_e2_…` utilise une action valide |
| M-C | « passer son tour » vaut pour **la même place** : `ent_inscription_offre.liberations` (numéro de la place, incrémenté à chaque place rendue ou ajoutée), `proposition_place.place_no` ; un dossier qui a écarté la place n° n n'est plus désigné pour elle (désignation automatique **et** appel sans exclusion), redevient désignable pour n + 1 | fait ; `p2_mc_passer_son_tour_une_seule_place` (A/B/A, appel sans exclusion, place suivante) ; `r_m11_…` ouvre la place par la route de l'offre |
| M-D | registre : dossier sans motif, auteur, état interne, rang bruts ; suivi sans lignes d'une décision non publiée ; décision limitée aux décisions publiées (`export_si`), sans motif interne ni auteur | fait ; `p2_md_registre_export_publie_seulement` (colonnes existantes, condition SQL valide) |
| Raccourci de test | `parcours_famille_de_bout_en_bout` et les deux recettes `ent-api` (`rp2_t02`, `rp2_10`, `rp2_11`) font la **vraie** acceptation (`repondre_proposition_famille` / `POST …/proposition/repondre`) ; le raccourci SQL restant (`famille_a_accepte`) exige `rows_affected >= 1` | fait |
| F2 | refus exacts (SQLSTATE `23514` pour les gardes en base, `23505` pour la chaîne ; conflit classé pour la concurrence acceptation/expiration ; saisie invalide ; conflit pour la confirmation sans motif) | fait (`tests_inscriptions_i5_revue.rs`) |
| F3 | règle 1 sur `decider_en_lot` en séance | fait ; `p2_f3_decisions_de_seance_non_publiees` |
| F4 | masquage par la **chaîne** : toute ligne de suivi écrite pendant une décision non publiée lui est rattachée (`suivi_tx`) ; plus de filtre sur `min(decide_le)` ni `created_at` ; `visibilite_famille_tx` ne lit que la chaîne | fait ; `p2_f4_masquage_par_la_chaine` (suivi antidaté) |
| F5 | ordre des verrous unique **dossier → offre → propositions** : offre verrouillée avant les propositions (envoi, réponse, constat, transitions), premier suivant verrouillé avant l'offre, plus de `SKIP LOCKED` (sauter un dossier désignerait le suivant à sa place) ; un interblocage résiduel est classé en conflit | fait (pas de test de concurrence dédié : voir limites) |
| F6 | doublon de `refuser_si_ecarte` (décision avancée) supprimé | fait |
| F7 | `se_deporter` idempotent (déjà déporté : même réponse, ni ligne, ni trace, ni événement) | fait ; `p2_f7_se_deporter_idempotent` |
| F8 | `conflit_possible` **supprimé** (inatteignable : un lien écarte d'office, la garde répond « introuvable ») | fait |
| F9 | création et modification d'une commission refusent (introuvable) un dossier écarté pour l'acteur, y compris au retrait | fait ; `p2_f9_f10_commission_et_dossiers_ecartes` |
| F10 | `publier_commission` n'échoue plus en bloc : les dossiers écartés sont ignorés et comptés (`PublicationDto.ignorees`) | fait ; même test |
| F11 | `ecartes_parmi` : une requête groupée sur les seuls dossiers de la liste (plus de balayage de l'établissement ni de garde par ligne) | fait |
| F12 | dépôt et reprise de pièce de la famille : contrôle de l'état visible **sous le verrou du dossier**, dans la transaction ; retrait/désistement idem (verrou avant le contrôle) | fait (couvert par `r_m2_…`, pas de test de course dédié) |
| F13 | `DecisionDto.motif_interne` en `Option` : `None` = masqué, jamais une chaîne vide | fait ; `r_m8_…` |
| F14 | `remplir` journalisé en ajout seul (`ent_inscription_condition_levee.genre = 'remplie'`, auteur, motif, objet source) | fait ; `p2_f14_f15_constat_et_acceptation_au_guichet` |
| F15 | « accepté au guichet » : `POST /inscriptions/propositions/{id}/accepter-au-guichet` (droit `inscriptions.gerer`, motif obligatoire), mêmes effets que la réponse de la famille mais tracé à part (`genre = 'guichet'`, condition `source = 'personnel'`, suivi, événement) | fait ; même test, plus la séparation entre établissements dans `separation` |
| F16 | 313 rejouable sur une base de l'ancienne version : objets recréés idempotents ; **réparation** d'un historique de décisions sans chaîne (plusieurs racines) avant les index uniques ; colonnes ajoutées en `IF NOT EXISTS` | fait ; `p2_f16_313_rejouable_sur_ancienne_version` (transaction annulée) |

### Non fait / limites
* Pas de test de **concurrence** pour l'ordre des verrous (F5) ni pour la course du dépôt de pièce (F12) : relus et couverts par les tests
  fonctionnels, non éprouvés sous charge.
* F16 ne répare que le cas « aucune décision du dossier ne remplace une autre » ; une chaîne partiellement renseignée et incohérente fait échouer
  l'index unique (échec franc, à traiter à la main).
* L'acceptation au guichet n'a pas de contrôle « deux personnes » : un secrétariat seul peut l'enregistrer ; le motif et le journal en ajout seul
  sont la seule contrepartie (à arbitrer avec le lot des droits).
* Le suivi écrit pendant une décision non publiée n'est lu qu'avec elle, y compris un commentaire destiné à la famille : voulu (règle 1).
* Les `.is_err()` des tests I5 d'origine (`tests_inscriptions_i5.rs`) hors liste du pilote ne sont pas resserrés.

### Vérifications locales (passe 2)
PostgreSQL 16 jetable (`ent_test_p2`, nom contenant « test », `ENT_ENV=test`), toolchain 1.99.0, `--locked`, un seul `CARGO_TARGET_DIR`, un fil :
* `ent-db` (labo) : `tests_inscriptions*`, `registre_donnees`, `tests_rls`, `migrate` : **130 verts, 0 échec** (dont 9 nouveaux tests `p2_…` et les 15 de
  revue d'origine, un resserré) ; gardes statiques (`garde_inscriptions_ecritures`, `garde_finances_ecritures`, `isolation_sql_statique` — qui a attrapé
  une condition d'export sans filtre d'établissement, corrigée —, `jour_utc_interdit`) : 0 échec.
* `ent-api` (labo) : `separation` (58, dont la nouvelle route d'acceptation au guichet), `recette_securite_3` (10), `recette_personas_2` (6) et
  `recette_personas_2_ouvertes` (11), cette fois **rejoués en entier** avec la vraie acceptation de la famille ; `tests_inscriptions` de la lib (4) : 0 échec.
* Clippy 1.99.0, les trois commandes de la CI (`--workspace --all-targets`, avec `ent-api/labo,ent-lab/labo`, `-p ent-types`) : **0 avertissement** ;
  rustc : 0 ; `cargo metadata --locked` dans `ent-rust/web` : vert ; fmt **ciblé** (jamais `--all`) : fichiers neufs ou déjà conformes seulement
  (`tests_inscriptions_i5_revue.rs`, les deux recettes `ent-api`, `inscriptions_decisions.rs`).
* Le raccourci SQL `famille_a_accepte` exige désormais `rows_affected >= 1` (et dit d'où il est appelé) : il a fait apparaître **dix appels sans objet**
  (seconde acceptation d'un dossier déjà accepté, réinscription), supprimés. Les tests qui consomment la file d'envoi partagée (`expedier`) restent
  sensibles au parallélisme, d'où le fil unique.
* La suite complète d'`ent-db` n'est pas rejouée en local (CI).

### Compteurs (règle 28, passe 2)
Deux fusions d'`origin/ENT` (messagerie M12, puis évaluations V5), **recomptées depuis la base commune** (`bc69cd0d`) : le compte RLS de `tests_rls.rs` a
conflité (**+8** de I5 et **+1** de la base ENT : `… + 4 + 1 + 8`) ; `SOURCES`/`NOMS_SOURCES` de `separation` **ont fusionné sans conflit mais faux** (base 69,
+1 de chaque côté, déclaré 70 des deux côtés : 71 entrées) — corrigé à **71** (seule la compilation l'a attrapé, exactement le piège de la règle 28) ;
`Action::TOUTES` inchangé (aucune action) ; **compte RLS de ce lot inchangé par la passe 2** (aucune table nouvelle : une séquence, des colonnes, une
fonction SQL) ; registre des données : aucune table nouvelle, colonnes hors export et condition de ligne (`export_si`) ajoutées ;
`MIGRATION_TAG` inchangé (3808) ; migration 313 modifiée (non versée) ; `ent-types` : `DeportEtatDto.conflit_possible` supprimé,
`PublicationDto.ignorees` et `DecisionDto.motif_interne: Option` ajoutés (aucun autre consommateur : le frontend `web/` ne les lit pas).

