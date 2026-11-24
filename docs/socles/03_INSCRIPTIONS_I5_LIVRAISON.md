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

### Tête et CI (passe 2)
* `3e8c5689` (correctifs) → fusions d'`origin/ENT` à trois reprises (la base a avancé pendant la passe : messagerie, évaluations V5, voyages V3) ; à chaque
  fusion, **compte RLS** (`tests_rls.rs`) et **`SOURCES`/`NOMS_SOURCES`** (`separation`) recomptés depuis la base commune : le compte RLS a conflité (final
  **820**, dont +8 de I5), `SOURCES` a fusionné faux deux fois (70 → 71 → **72**), attrapé par la compilation ; `Action::TOUTES` (275) et `MIGRATION_TAG` relus.
* Première CI de la passe (`6c13ad5d`) : tout vert sauf **deux tranches du frontend** (1 et 2 sur 6), cause unique : les parcours web contre le backend
  (`ent-web-server`, `inscriptions` et `demo_p1`) validaient l'inscription sans acceptation de la place par la famille (règle 3). Ils n'avaient pas été
  rejoués en local (**les tests du frontend contre le backend, avec `--features labo`, sont à rejouer pour tout lot qui change une règle d'inscription**).
  Correction : méthode `repondre_proposition` au client typé `ent-web-api` (route de la famille) et vraie acceptation dans les deux parcours ; rejoués
  en local (4 + 2 verts, profil sans débogage comme la CI, 2 fils pour la mémoire).
* Tête de code finale : **`cf878246`** ; PR n° 120 laissée en **brouillon**.
* **CI sur `cf878246` : verte** (39 jobs en succès, 2 sautés : la RLS stricte, réservée à `ENT`, et un job conditionnel ; aucun échec, aucune relance).
* Statut : **implémenté, testé, CI verte** ; non validé par la relecture du pilote ; non prêt pilote, non prêt production.


## Passe 3 (contre-revue de la passe 2, tête cf878246) — branche `ent-socle-3-code-i5-p3`

Départ : `origin/ENT` (I5 versé). **313 est versée et ne change pas** (règle 25) : tout passe par la **314**
(`314_inscription_decision_i5_passe3`, après 313 dans `migrate.rs`). Statut : **implémenté et testé en local** (PostgreSQL 16 jetable,
base `ent_test_p3`, neuve) ; **non validé** tant que la CI n'a pas rendu son verdict ; non prêt pilote, non prêt production.

### Règles tenues
1. **Un seul ordre des verrous, posé en un point unique** : `inscriptions_attente::verrouiller_dans_l_ordre_tx`
   (**dossiers → campagne → offres → propositions**). Plusieurs dossiers : **un seul lot trié par UUID** ; plusieurs offres : par UUID. Un appelant calcule
   d'abord **en lecture** ce qu'il touchera (`verrouiller_pour_offres_tx` : le dossier, les premiers dossiers de la liste, les dossiers qui portent une place
   désignée, la campagne, les offres), puis verrouille ; il ne tient **jamais** un dossier avant d'avoir pris tout le lot. `designer_suivants_tx` n'**attend**
   plus aucun dossier : l'offre et la fenêtre sont tenues par l'appelant, un dossier hors fenêtre se prend en `NOWAIT` (55P03 → conflit réessayable, jamais
   un interblocage, jamais un autre dossier désigné à sa place).
2. **Ce que voit la famille ne dépend que des décisions publiées**, export RGPD compris (registre complété, vérifié par des lignes réellement exportables).
3. **Une décision publiée correspond à l'état du dossier à la publication**, sinon elle est **caduque** et tracée.

### Correctifs (numéros de la contre-revue)
| N° | Correctif | Fait / test |
|---|---|---|
| 1 | ordre des verrous : `decider_tx` (lot dossiers + campagne + offres `avant` et `cible`, **avant** `dossier_tx`), `verrouiller_proposition_tx` (écarter, envoyer, constat, guichet), réponse de la famille, `apres_transition_tx` (dossier déjà tenu : `NOWAIT`), `designer_suivants_for_actor`, `reutiliser_core` (dossier cible / source triés) ; `offre_cible_tx` extrait de `retenir_voeu_tx` (lecture seule) ; la campagne se prend avant l'offre dans le chemin « envoyer une place » (même ordre que `decider_tx`) | fait ; `p3_verrous_refus_famille_et_decision_sur_le_suivant` (6 tours : refus de la famille sur X / décision sur Y), `p3_verrous_deplacements_croises` (6 tours A→B / B→A), `p3_verrous_reutilisations_croisees` : **chaque opération doit réussir** (le 40P01 est mappé en conflit, donc un conflit est un échec du test) |
| 2 | registre : `export_si` ajoutés (propositions `envoyee`/`acceptee`/`declinee`/`expiree` ; conditions d'une décision d'admission publiée sans décision plus récente en attente ; suivi `visible_famille` et décision nulle ou publiée) ; `hors_export` étendu (dossier : `voeu_retenu_id`, `clos_le`, `motif_cloture`, `decide_le` **en entier**, aucune règle de ligne ne séparant le cas ; vœu : `statut` en entier ; décision : `remplace_decision_id`, `depassement_capacite`, `alternative`, `caduque_*`) | fait ; `p3_export_ne_rend_que_le_publie` (la condition SQL de chaque `export_si` **exécutée** après une décision non publiée : 0 ligne de décision, de proposition, de condition ; aucune ligne interne ni rattachée du suivi ; puis publiée : les lignes apparaissent) |
| 3 | publication caduque : `publier_decision_tx` renvoie `Publication::{Publiee, Caduque, Aucune}` ; état du dossier ≠ `etat_cible(sens)` (blocage pris en compte) → `caduque_le` / `caduque_motif` (écrits une fois, trigger), proposition ouverte annulée, suivi **interne**, **ni publiée ni notifiée, aucun `ADMISSION_DECIDEE`** ; la tâche de fond valide la transaction de la caducité ; `publier_ou_programmer_tx` la compte (`PublicationDto.caduques`) ; une décision caduque n'est plus une « décision en attente » côté famille (`visibilite_famille_tx`, `ent_inscription_en_liste_visible`, rattachement du suivi) | fait ; `p3_publication_caduque_dossier_annule_avant_l_echeance` (programmée à J+7, annulée à J+3 : rien publié, caducité tracée, 0 événement, 0 courriel, famille ne lit rien, publication manuelle refusée) |
| 4 | 314 : `setval('ent_inscription_attente_ordre_seq', GREATEST(max(ordre_attente), dernière valeur de la séquence, 1))` — **la séquence ne recule jamais** (rejeu) | fait ; `p3_sequence_ordre_attente_reprise_puis_nouvel_entrant` (dossier repris d'ordre élevé, puis entrant : derrière côté famille **et** côté désignation) |
| 5 | écarter : seulement `origine = liste_attente AND decision_id IS NULL`, sinon refus exact « cette place est rattachée à une décision : passer par une nouvelle décision pour la modifier » | fait ; `p3_ecarter_seulement_une_designation_de_la_liste` (le refus exact, la décision reste publiable, sa proposition part à la publication) |
| 6 | 314 rechaîne tout dossier à plusieurs racines ou à double successeur (ordre `decide_le`, puis `id`), **avant** la pose des index uniques (`IF NOT EXISTS`, les deux index recréés) ; l'ajout seul est suspendu le temps de la réparation | fait ; `p3_314_repare_une_chaine_partielle` (d1 racine, d2 remplace d1, d3 racine → d1, d2→d1, d3→d2, ordre vérifié, index revenus) |

**Gardes à `search_path` fixé (314).** Les six fonctions de garde de 313 sont **recréées** avec `SET search_path = pg_catalog, public, pg_temp` :
`ent_inscription_purge_voulue`, `ent_inscription_ajout_seul` (qui apprend la caducité), `ent_inscription_decision_chaine`,
`ent_inscription_membre_lie`, `ent_inscription_ecarte_garde`, `ent_inscription_en_liste_visible`. Vérifié dans `p3_314_repare_une_chaine_partielle` :
aucune fonction sans réglage, et une table temporaire `ent_inscription_decision` homonyme ne trompe pas la garde de chaîne (refus `23514` exact).

### Acceptation au guichet (D-069, F15)
Conservée **sans « deux personnes »** (motif obligatoire, journal en ajout seul). Désormais : **refusée après `repondre_avant`** tant que l'expiration n'est pas
constatée ; la proposition passe à `acceptee` (l'action « répondre » disparaît côté famille) ; la **famille est notifiée** ; refus **distincts** :
« le dossier n'est plus à l'état admis », « proposition déjà répondue », « place désignée, pas encore envoyée », « proposition expirée », « proposition annulée ou
écartée », « le délai de réponse est dépassé ». Test : `p3_guichet_refus_distincts_notification_et_delai` (403 sans droit, `designee`, `expiree`, `declinee`,
`annulee`, acteur écarté → introuvable, délai dépassé, succès : statut, plus d'action « répondre », courriel, motif de proposition masqué, seconde saisie refusée).

### Faibles
| Point | État |
|---|---|
| motif de proposition masqué hors `inscriptions.decider` | **fait** (`proposition_dto(voit_motif)`) ; `p3_motif_masque_et_ecartes_avant_la_limite` |
| filtre des écartés **avant** le `LIMIT` | **fait** (les dossiers écartés sont retirés par la requête, avant `LIMIT 500`) ; même test |
| `liberations` incrémenté du nombre de places ajoutées | **fait pour la capacité** (+3 de capacité = 3 numéros) ; `p3_liberations_autant_que_de_places`. **Non fait pour la réserve de réinscription** : c'est un booléen, la lever ouvre « au moins une » place (inconnue) : +1 conservé |
| désignation relancée si la ligne verrouillée a changé | **fait** (relance bornée à 5 si la lecture simple voit encore un candidat, plus de `break` silencieux) |
| `dossier_dto` lu dans une seule transaction | **fait** (une transaction `REPEATABLE READ` en lecture seule, `TenantTx::begin_lecture_coherente` : un seul instant pour l'état, les pièces, le suivi et le rang) |
| rang lu d'une seule façon | **fait** (`rang_lu_par_la_famille_tx`, utilisé par `dossier_famille` et `decision_famille` ; `p2_ma…` et `rang_vu` comparent les deux) |
| `information_schema` filtré par `table_schema` | **fait dans le test** (`p2_md…`). **Non fait dans 313** (versée, règle 25) |
| `r_m4_…` : assertion exacte | **fait** (au moins une revue passe, chaque refus est un conflit de dossier, la chaîne compte exactement `1 + revues réussies`) |

### Non fait / limites (à relire)
* **Réparation de chaîne avant 313 impossible.** 313 (versée) pose les index uniques **sans** réparer l'historique partiel : une base où 313 n'a jamais abouti
  à cause d'un tel historique s'arrête sur 313 avant d'atteindre 314. La 314 répare tout dossier **déjà** passé par 313 (état intermédiaire) et rend les index
  jamais bloquants pour ses propres rejeux ; le cas « 313 jamais appliquée sur historique partiel » demande une décision du pilote (réparer à la main avant la
  première application, ou autoriser un fichier numéroté avant 313).
* Les dossiers hors de la **fenêtre** de verrouillage (premiers de la liste, plus une marge de 5) se prennent en `NOWAIT` : sous forte concurrence, une désignation
  peut rendre un **conflit réessayable** (409) au lieu d'attendre. C'est le prix de l'absence d'interblocage ; jamais une désignation hors rang.
* `apres_transition_tx` (appelée depuis des transitions où l'appelant tient déjà son dossier) prend aussi les autres dossiers de la liste en `NOWAIT` : même
  conséquence. Les routes qui verrouillent le lot **d'abord** (décision, réponse de la famille, écarter, envoyer, désigner) attendent normalement.
* Les tests de concurrence rejouent chaque scénario 6 fois ; **ils n'ont pas été vus rouges sur le code d'avant** (non rejoué faute de budget) : ils prouvent
  l'absence d'échec avec le nouvel ordre, pas la reproduction de l'ancien interblocage.
* **Bogue préexistant trouvé et corrigé** (`inscriptions_modele.rs`) : déplacer un dossier vers **une autre offre** que celle de son vœu échouait en erreur 500
  (`COALESCE(max(rang), 0)::smallint + 1` rend un `INT4` lu en `i16`) ; mis au jour par le test des déplacements croisés.
* `pg_err.rs` : aucune modification conservée (une trace de débogage a été retirée).

### Vérifications locales (passe 3)
* Base locale **neuve** `ent_test_p3` (PostgreSQL 16, nom contenant « test »), un seul `CARGO_TARGET_DIR`, `--locked`, toolchain 1.99.0.
* `ent-db` (feature `labo`) : les **11** tests de `tests_inscriptions_i5_passe3` verts ; **141 + 1** tests d'inscriptions (`inscriptions`) verts, puis **150** (`r_m4`, `registre`,
  `migrat`, `d036`, `export`, `rgpd`, `p2_`, `i5_`) verts, dont `tests_d036` (le registre des migrations compte 314) et `r_m4` (assertion exacte). Pas de `#[ignore]`.
* **Tests du frontend contre le backend rejoués** (`ent-web-server`, `--features ent-web-server/labo,ent-api/labo`, `ENT_ENV=lab`, base neuve `ent_test_web`) :
  `inscriptions` **4 verts**, `demo_p1` **2 verts**.
* **clippy 1.99.0, zéro avertissement** sur les trois commandes de la CI (`--workspace --all-targets`, avec `ent-api/labo,ent-lab/labo` — `ent-lab` compris —,
  `-p ent-types`), `--locked` ; relancé après la fusion d'`origin/ENT`. fmt **ciblé** : le fichier neuf `tests_inscriptions_i5_passe3.rs` est formaté ; aucun autre
  fichier n'est reformaté (jamais `cargo fmt --all`). `cargo metadata --locked` dans `ent-rust/web` : vert, `Cargo.lock` inchangé.
* Ni `#[ignore]`, ni heure réelle, ni jour UTC dans les tests (instants posés en 2027).

### Compteurs (règle 28, passe 3)
Aucune table, aucune action, aucune source de séparation nouvelle : **compte RLS (`tests_rls.rs`) inchangé**, `Action::TOUTES` inchangé, `SOURCES`/`NOMS_SOURCES`
inchangés, registre des tables inchangé (colonnes hors export et conditions de ligne ajoutées) ; `MIGRATION_TAG` inchangé (3808) ; `MIGRATIONS` : **+1** (314, après 313).
Fusion d'`origin/ENT` (`f298bc7a`, internat I4, messagerie M12/E1, cahier C8/C9) **sans conflit** ; compteurs de la base relus, rien à recalculer de notre côté.
`ent-types` : `PublicationDto.caduques` ajouté (`#[serde(default)]`, aucun autre consommateur : `web/` ne construit pas ce type).

### Tête et CI (passe 3)
* PR n° 127, **brouillon**, vers `ENT`. Première CI (`06be5c4e`) : rouge, **cause unique** — la garde statique `garde_rls_stricte` refusait `TenantTx::begin_lecture_coherente` (accès « direct » au pool) ; déclarée dans la garde, test rejoué en local.
* Tête de code finale : **`d495fe98`**. **CI verte** sur cette tête (41 jobs, aucun échec). Statut : **implémenté, testé, CI verte** ; non validé par la relecture du pilote ; non prêt pilote, non prêt production.


## Passe 4 (nouveau modèle de verrous) — branche `ent-socle-3-code-i5-p4`

Départ : `origin/ENT` (passe 3 versée). **313 et 314 ne changent pas** (règle 25) ; tout passe par la **315** (`315_inscription_decision_i5_passe4`, après 314 dans `migrate.rs`).
Trois passes n'ayant pas fermé l'ordre des verrous, la passe 4 **change de modèle** au lieu de recoller des verrous. Statut : **implémenté et testé en local** (PostgreSQL 16 jetable, base neuve `ent_test_p4`) ; **non validé** tant que la CI n'a pas rendu son verdict ; non prêt pilote, non prêt production.

### Le modèle
1. **Le rang n'est plus stocké, il se calcule en lecture.** `compacter_rangs_tx` disparaît. Fonction SQL unique `ent_inscription_rang_attente(etab, dossier, famille)` : `row_number()` sur `ordre_attente` (jamais renuméroté), parmi les dossiers en liste (personnel) ou parmi les seules entrées et sorties **publiées** (`ent_inscription_en_liste_visible`, famille). Tous les appelants passent par `rang_calcule_tx`. `rang_attente` est conservée **en lecture seule** pour la reprise (déclencheur ; `COLS_DOSSIER` ne la lit plus). La désignation (`SUIVANT_A_DESIGNER`), la liste d'attente et les propositions trient sur `ordre_attente`.
2. **Un verrou unique et grossier par offre, pris en premier.** `verrouiller_dans_l_ordre_tx` (**seul** point du dépôt) : campagne (`FOR NO KEY UPDATE`, seulement `modifier_offre_for_actor`) → offres (`pg_advisory_xact_lock(ent_inscription_offre_cle(offre))`, triées par UUID) → dossiers (triés par UUID, `FOR NO KEY UPDATE`) → propositions. `verrouiller_dossiers_tx` lit les offres d'un dossier, verrouille, relit : un dossier déplacé entre-temps donne un **conflit réessayable** (la transaction s'arrête, elle n'attend jamais hors ordre). `dossier_tx` passe par là : **tout** verrou de dossier est précédé du verrou de ses offres. Plus de `NOWAIT`, plus de fenêtre de dossiers, plus de verrou de campagne en décision. Garde statique (`p4_garde_statique_un_seul_point_de_verrou`) : aucun `FOR UPDATE` / `FOR NO KEY UPDATE` sur une offre ou un dossier, aucun `pg_advisory`, `NOWAIT`, `SKIP LOCKED` sur la liste, `compacter_rangs` ou écriture de `rang_attente` hors de la fonction.
3. **Contrôle d'écart avant les verrous de lot (F3)** : `decider_tx`, `verrouiller_proposition_tx`, `publier_ou_programmer_tx` (qui résout d'abord offres et dossiers de la séance, écarte les dossiers écartés, puis verrouille **une seule fois**).

### Correctifs de la contre-revue
| N° | Fait |
|---|---|
| M1, M1b, M1c | ordre unique ; publication de séance en un lot ; `modifier_offre` : campagne puis offre |
| M3 | vœu créé par une décision : `origine = 'decision'`, exporté seulement quand une décision publiée du dossier retient l'offre ; test qui **exécute la vraie condition de chaque table** à règle de ligne (et échoue si une table nouvelle n'a pas de cas) |
| M4 | export des conditions : seul le bout de chaîne (sans successeur) bloque |
| M5 | test `search_path` : table temporaire **remplie**, deux sens (prédécesseur seulement temporaire → refusé ; prédécesseur réel → accepté) |
| M6 (D-069) | `etats_compatibles(sens)` : `admis`/`admis_sous_reserve` → {admis, confirme} ; `liste_attente` → {liste_attente} ; `refuse` → {refuse} ; `ajourne` → {en_instruction, attente_famille} (l'ajournement reporte l'examen, que le dossier attende la famille ne le contredit pas) ; décision caduque → **avis interne** à la direction et à l'auteur (`ent_notification`, titre sans nom d'élève, référence seulement, idempotent) |
| F1 | `dossier_dto` : une transaction, **une connexion** (campagne, responsables, invitations, fermeture comprises), état du dossier relu dans le snapshot ; test avec un pool d'une connexion |
| F5 | `40P01` n'est plus un conflit métier : `DomainError::Reessayer` (classe 40, 503) ; test avec un vrai interblocage ; aucun test ne l'accepte |
| F6 | `place_no`, `origine`, `designee_le` hors export (colonnes vérifiées réelles) |

### Tests (tests_inscriptions_i5_passe4.rs, 14)
Concurrence **déterministe** : une connexion externe tient le verrou de l'offre ; l'opération attend (`pg_locks` non accordé + `wait_event_type = 'Lock'`) ; **aucun dossier n'est tenu** pendant l'attente (`FOR UPDATE NOWAIT` externe réussit) ; on relâche ; elle aboutit. Scénarios : désistement du rang 3 pendant une décision sur le rang 20 (25 dossiers) ; déplacements croisés A→B / B→A (les deux attendent d'abord la plus petite offre) et campagne indépendante qui n'attend pas ; publication de séance concurrente d'une décision ; capacité concurrente d'une décision ; deux réutilisations croisées ; contrôle d'écart (offre **et** dossier tenus par l'extérieur : « introuvable » immédiat).
Point d'interprétation : « entre deux campagnes » — une offre voulue doit appartenir à la campagne du dossier, un croisement A→B / B→A a donc lieu dans une campagne ; le verrou de campagne qui le sérialisait a disparu, et une seconde campagne est testée comme n'attendant pas.

### Preuve rouge sur le code d'avant
Les mêmes tests lancés sur une copie d'`origin/ENT` (base neuve `ent_test_p4_ref`) : **11 rouges sur 14** — les 5 scénarios de concurrence échouent parce que l'opération **n'attend pas** le verrou de l'offre (0 attente) ; rang/clé, export (colonne `origine`), caducité, garde statique, `dossier_dto` (pool d'une connexion : erreur d'infrastructure) et interblocage échouent aussi ; le contrôle d'écart attend le dossier tenu (délai dépassé) ; seul `p4_garde_search_path_table_temporaire_remplie` est vert sur les deux (314 était déjà juste : c'est une garde de non-régression, **non démontrée rouge**).

### Non fait / limites
* Les dossiers modifiés en masse par un `UPDATE` (prolongation, fermeture) ne passent pas par `verrouiller_dossiers_tx` : ils tiennent des verrous de ligne sans verrou d'offre (non couvert par la garde, risque résiduel faible : aucun ne prend ensuite un verrou d'offre).
* Un dossier déplacé entre la lecture de ses offres et leur verrouillage rend un conflit réessayable (409) : rare, jamais d'attente hors ordre.
* 313 versée : `information_schema` non filtré par schéma dans ses tests historiques (inchangé).

### Vérifications locales
* Base neuve `ent_test_p4` (nom contenant « test »), un seul `CARGO_TARGET_DIR`, `--locked`, toolchain 1.99.0. **Leçon** : après avoir bâti une copie de référence dans le même répertoire de compilation, `touch` des sources avant de rejouer (règle 22) — un binaire de l'ancien code a faussé un rejeu.
* `ent-db` (labo) : 278 tests (`inscriptions`, `r_m4`, `registre`, `migrat`, `d036`, `export`, `rgpd`, `p2_`, `i5_`, `p3_`) verts ; les 14 de la passe 4 verts.

### Passe 4 — compteurs, tête et CI

- **Compteurs** : aucune nouvelle table, action ni source. `MIGRATIONS` +1 (315) ; compte RLS, `Action::TOUTES`, `SOURCES` et `MIGRATION_TAG` (3808) inchangés par la passe (après fusion d'`ENT` : SOURCES 75, TOUTES 282).
- **CI de la tête `770a7016` : rouge**, une seule cause — la garde `garde_rls_stricte::aucun_acces_direct_au_pool_hors_exception` signalait deux exceptions devenues inutiles (`inscriptions : .fetch_all(self.pool())` et `fermeture_en_cours(self.pool(), etab)`), car `dossier_dto` n'emploie plus le pool (une seule connexion, F1). Correctif : exceptions retirées (`081f09d9`). Les 20 tranches labo hors tranche 11 et tous les autres jobs étaient verts ; la tranche 11 échouait sur la même garde.
- Vérification locale après correctif : `cargo test --locked -p ent-db -p ent-api` (sans labo) vert, 754 tests `ent-db`.
- **CI de la tête `9352f9e3` (correctif + fusion d'`ENT`, garde `search_path` T1 incluse) : verte** (workflow « ENT Rust », succès). Tests locaux de la tête fusionnée : 103 tests ciblés (`garde_search_path`, `p4_`, `t1_`, `migrat`, `registre`) verts sur base neuve.

## Suites (passe 5) — contre-revue de la passe 4 — branche `ent-socle-3-code-i5-suites`

Départ : `origin/ENT` (passe 4 versée, ae6e4fc1). **313 à 315 ne changent pas** (règles 25 et 38) ; tout passe par la **316** (`316_inscription_i5_suites_sentinelle`), ajoutée **à la fin** du tableau de `migrate.rs` (après 315). Chaque fonction créée porte `SET search_path = pg_catalog, public, pg_temp`, relations qualifiées `public.` ; **aucune preuve de verrou** ne lit `PG_CONTEXT` ni `set_config` (précision de la passe 6, F2 : l'**exemption de suppression** de la sentinelle lit `ent_inscription_purge_voulue()`, donc le réglage de session `ent.purge_etablissement`, que tout appelant peut poser ; elle ne couvre que la suppression de lignes, avec la même limite que `ent_inscription_ajout_seul` ; la sentinelle est un filet, pas un contrôle de sécurité). Statut : **implémenté et testé en local** (PostgreSQL 16 jetable, base neuve dont le nom contient « test ») ; **non validé** tant que la CI n'a pas rendu son verdict ; non prêt pilote, non prêt production.

### Fait

| N° | Fait |
|---|---|
| **M-2** | **Sentinelle en base.** Déclencheurs `AFTER` sur `ent_inscription_proposition_place` (insertion, mise à jour, suppression), `ent_inscription_decision` (idem) et `ent_inscription_offre` (mise à jour qui change `capacite` ou `liberations`). Chacun exige que la **transaction tienne le verrou consultatif de l'offre concernée** : lecture de `pg_locks` filtrée sur `locktype = 'advisory'`, `granted`, `objsubid = 1`, la base courante, la clé `ent_inscription_offre_cle(offre)` et **`pid = pg_backend_pid()`**. Sinon : erreur **nommée** `ent_inscription_verrou_offre_manquant` (SQLSTATE `55000`). L'offre d'une ligne est son `offre_id`, à défaut celle de la liste de son dossier (`ent_inscription_offre_du_dossier`). Offre inconnue : rien à protéger. Suppression en cascade d'un dossier purgé (`ent_inscription_purge_voulue()`) : exemptée. Les déclencheurs sont **après** la ligne : une écriture déjà refusée par une contrainte ou par une garde de la table (unicité, écart, ajout seul) garde son refus d'origine. |
| **M-2 (parcours)** | Tous les `inscriptions*.rs` parcourus (liste des écritures des trois tables relevée par `grep`, puis tests). **Une écriture hors verrou trouvée par la sentinelle** : `synchroniser_offres_niveaux_tx` (capacité et places rendues d'une offre, via `modifier_campagne`) — voir F-a. Toutes les autres écritures des trois tables passent déjà par `verrouiller_dossiers_tx` / `verrouiller_dans_l_ordre_tx`. |
| **M-3** | `repondre_accord` : l'offre se prend **avant** la demande. La demande est lue **sans verrou** (pour connaître son dossier), puis `verrouiller_dossiers_tx` (offres → dossier), puis la demande est verrouillée **et relue**. Ordre : offres → dossiers → propositions, la demande en dernier. |
| **F-a** | `synchroniser_offres_niveaux_tx` prend le verrou de l'offre avant d'écrire `capacite` / `liberations` (la campagne est déjà tenue : campagne → offre, comme `modifier_offre`). **Trouvé par la sentinelle**, pas par la lecture : `i2_campagne_regles_annee_et_offres_des_niveaux` est devenu rouge sur ce chemin. |
| **F-b** | Écritures de masse sous l'ordre unique : `prolonger_dossiers_tx` (échéances de dossiers) et `declarer_fermeture_for_actor` (échéances qui tombent dans la fermeture) lisent les dossiers visés **sans verrou**, les verrouillent **avec leurs offres** (`verrouiller_dossiers_tx`), puis modifient ; les conditions sont rejouées dans l'`UPDATE` (un dossier qui a bougé n'est pas touché). La fermeture met à jour les **campagnes avant** les dossiers (campagne → offres → dossiers, comme `modifier_offre`). |
| **F-c** | **Une seule formule du rang** : fonctions SQL `ent_inscription_liste_attente(etab, offre, famille)` (la liste ordonnée avec sa position) et `ent_inscription_offre_du_dossier`. `ent_inscription_rang_attente` (rang d'un dossier), la **liste d'attente affichée** (`liste_attente_tx`) et la **désignation du suivant** (`SUIVANT_A_DESIGNER`) lisent la même fonction ; `COALESCE(ordre_attente, rang_attente) … row_number()` n'existe plus que dans la migration 316. Au passage, la liste affichée utilisait l'offre du **vœu** quand le rang utilisait `offre_attente_id` (deux définitions de « l'offre de la liste ») : il n'y en a plus qu'une. |
| **F-e** | L'avis « décision programmée non publiée » ne va qu'aux accès **ouverts aujourd'hui** (`ent_acces_ouvert`, définition unique : accès `ouvert`, date de fin non dépassée), pour la direction **et** pour l'auteur de la décision. Un directeur dont l'accès est fermé (fin de fonction), suspendu ou dont la date de fin est dépassée n'est plus alerté. |

### Tests (`tests_inscriptions_i5_suites.rs`, 9)

| Test | Prouve |
|---|---|
| `s5_sentinelle_refuse_l_ecriture_hors_verrou` | chaque écriture directe (offre : places rendues, capacité ; décision ; proposition : insertion avec ou sans offre propre, mise à jour, suppression) est **refusée** sans verrou (SQLSTATE `55000` **et** nom de l'erreur), **acceptée** avec le verrou de **cette** offre ; une colonne hors places ou une capacité inchangée ne sont pas gênées |
| `s5_sentinelle_ne_se_falsifie_pas` | le verrou d'une **autre offre**, d'une **autre session**, un `set_config`, une **table temporaire `pg_locks`** ne satisfont pas la sentinelle ; un verrou réellement tenu (session) la satisfait |
| `s5_cycle_complet_sous_sentinelle` | dépôt, décisions, liste, désistement, désignation, retrait, hausse de capacité : aucune écriture du service n'est refusée |
| `s5_repondre_accord_dans_l_ordre_contre_une_decision` | **M-3, concurrence déterministe** : une connexion externe tient l'offre ; la réponse d'accord **et** une décision de séance attendent (`pg_locks`, `wait_event_type = 'Lock'`) ; la demande (`FOR UPDATE NOWAIT`) et les dossiers sont **libres** pendant l'attente ; on relâche : les deux aboutissent, sans interblocage ni erreur technique |
| `s5_prolongation_sous_le_verrou_d_offre`, `s5_fermeture_sous_le_verrou_d_offre` | **F-b** : l'écriture de masse **attend l'offre**, ne tient **aucun dossier** pendant l'attente, aboutit ; échéances recalculées (dates fixes 2099) |
| `s5_rang_une_seule_formule` | **F-c** : après un départ au milieu, rang SQL = position affichée = position dans la liste SQL ; hors liste : pas de rang ; le suivant désigné est la position 1 |
| `s5_garde_statique_formule_de_rang` | **F-c** : aucune requête des fichiers d'inscription n'écrit la formule d'ordre (`ordre_attente` / `row_number()` dans un `ORDER BY` ou `OVER`) |
| `s5_directeurs_partis_non_alertes` | **F-e** : directeur en poste, à fin lointaine et auteur alertés **une fois** ; accès fermé, suspendu, date de fin dépassée (2020) **non** alertés |

Aucun test ne dépend de l'heure réelle ni du jour UTC : instants posés (2027), dates fixes (2099 pour les échéances, 2020 pour une fin dépassée).

**Fixtures des tests existants** : trois écritures directes (SQL de fixture) des tests de passes précédentes prenaient leurs lignes sans le verrou de l'offre ; elles passent maintenant par `tx_sous_verrou_offres` (même clé que le service) ; deux tests qui rejouent 313/314 sur un état ancien neutralisent la sentinelle le temps de poser l'historique (transaction annulée).

### Preuve rouge (code d'avant, migration 316 conservée)
Les deux fichiers corrigés (`inscriptions_accords.rs`, `inscriptions_modele.rs`) remis à leur état d'`origin/ENT`, les mêmes tests : **rouges** — `s5_repondre_accord_…` (la demande est tenue pendant l'attente de l'offre : ordre inversé), `s5_prolongation_…` et `s5_fermeture_…` (l'écriture de masse n'attend pas l'offre), `i2_campagne_regles_annee_et_offres_des_niveaux` (la sentinelle refuse l'écriture de capacité hors verrou). Verts sur l'ancien code, comme attendu : les deux tests de sentinelle (ils éprouvent la base, pas le service) et `s5_cycle_complet_sous_sentinelle` (garde de non-régression).

### Mesure : surcoût de la sentinelle par écriture
Banc court, PostgreSQL 16 local, 20 000 appels de la fonction que le déclencheur exécute (boucle PL/pgSQL, hors écriture), trois séries :
* **sans autre verrou consultatif dans l'instance : 45 à 52 µs par écriture** ;
* avec **500 verrous consultatifs** tenus par une autre session : **225 à 300 µs par écriture** — la lecture de `pg_locks` parcourt tous les verrous de l'instance et prend les verrous internes de la table des verrous.
Le chiffre est faible pour le volume de ce domaine (quelques écritures de places par décision), mais **il croît avec le nombre de verrous de l'instance** : à surveiller en pilote. Une mesure bout en bout (`UPDATE` d'offre) est trop bruitée par les versions mortes d'une même ligne pour être donnée.

### Liste nominative : tests de concurrence verts sur l'ancien code, et pourquoi
Les sept scénarios de concurrence de la passe 4 sont **verts** sur le code d'avant cette passe, alors que celui-ci contenait M-3, F-a et F-b :
1. `p4_desistement_rang_3_pendant_decision_rang_20`
2. `p4_deplacements_croises_et_campagnes_independantes`
3. `p4_publication_de_seance_concurrente_d_une_decision`
4. `p4_capacite_concurrente_d_une_decision`
5. `p4_reutilisations_croisees`
6. `p4_controle_d_ecart_avant_les_verrous`
7. `p4_interblocage_n_est_pas_un_conflit`

Explication : chacun entre par un **point d'entrée qui prenait déjà le verrou d'offre** (décision, publication, déplacement, capacité par `modifier_offre`, désistement) et observe l'attente **de ce point d'entrée** ; aucun ne fait entrer `repondre_accord` (qui prenait la demande avant l'offre), `modifier_campagne` (capacité sans verrou d'offre) ni une écriture de masse. La garde statique ne voyait ni l'ordre des verrous **d'autres tables** (la demande), ni une écriture **sans aucun verrou**. Un test de concurrence ne prouve l'ordre que des chemins qu'il exerce : d'où la sentinelle, qui vaut pour **tout** chemin qui écrit les places, propositions et décisions, exercé ou non.

### Non fait / limites
* « **Compte désactivé** » : le modèle n'a pas de drapeau de compte désactivé au niveau utilisateur ; l'accès à l'établissement porte l'état (`ouvert` / `suspendu` / `ferme`) et la date de fin, lus par `ent_acces_ouvert`. C'est cette définition qui fait foi pour F-e. Une « fin de fonction » sans fermeture de l'accès (une fonction d'agent qui se termine, l'accès restant ouvert) **n'est pas** lue : les fonctions d'agent (`ent_agent_fonction.au`) ne gouvernent pas l'accès.
* La sentinelle protège **les places, propositions et décisions** (et la capacité de l'offre) ; elle ne couvre pas les écritures de **dossier** (état, `ordre_attente`, `offre_attente_id`) ni de **vœu** : l'ordre de ces chemins reste garanti par `verrouiller_dossiers_tx` et par les tests de concurrence.
* La sentinelle lit `pg_locks` : coût linéaire en nombre de verrous de l'instance (mesure ci-dessus). Une écriture **hors transaction applicative** (SQL de maintenance) doit prendre le verrou de l'offre elle aussi : c'est voulu.
* `migrations_versees.txt` (règle 38) n'existe pas encore sur `ENT` : 316 est en fin de tableau, le pilote l'ajoutera au versement.
* Deux tests (`echec_d_envoi_trace_et_code_hmac`, `p3_export_ne_rend_que_le_publie`) ont échoué **une fois** à quatre fils en parallèle (file d'envoi et tâche d'échéance communes à tous les établissements, déjà documentées) : verts seuls, à un fil (réglage de la CI).

### Vérifications locales (passe 5)
* Base neuve dont le nom contient « test » (`ent_test_i5b`), un seul `CARGO_TARGET_DIR`, `--locked`, toolchain 1.99.0, `ENT_ENV=lab`. **Leçon** : modifier le fichier d'une migration déjà appliquée sur la base locale donne `FichierModifie` : recréer la base.
* `ent-db` (labo), filtres `inscription`, `r_m4`, `registre`, `migrat`, `d036`, `export`, `rgpd`, `p2_`, `i5_`, `p3_`, `p4_`, `i3_`, `i4_`, `garde_search_path` : **381 tests**, 379 verts à quatre fils + les 2 relancés seuls verts ; dont les 9 de la passe 5.

### Suites (passe 5) — compteurs, tête et CI
- **Compteurs** : `MIGRATIONS` +1 (316) ; aucune nouvelle table, action ni source ; compte RLS, `Action::TOUTES`, `SOURCES` et `MIGRATION_TAG` (3808) inchangés.
- **Qualité** : clippy 1.99 sans avertissement sur les 3 commandes de la CI (`ent-lab` compris) ; `cargo metadata --locked` dans `ent-rust/web` ; gardes statiques vertes sans et avec `labo`.
- **CI de la tête `7a040a70`** (PR 137) : workflow « ENT Rust » (run 45279958095) **vert**. Le commit suivant ne change que cette section de documentation.

### Correctifs du pilote après contre-revue des suites (passe 5)
- **M1** : la sentinelle contrôle aussi, sur un UPDATE, l'ancienne offre **déduite du dossier** (`OLD.offre_id` nul) ou d'un ancien dossier ; test `s5_sentinelle_changement_d_offre_exige_l_ancienne_et_la_nouvelle` (nouvelle seule refusée, ancienne seule refusée, les deux acceptées). 316 n'était pas versée : modifiée en place.
- **M3** : `publier_decision_tx` et la publication de séance verrouillent aussi l'offre **portée par la décision** (`offres_en_plus`), sans supposer qu'elle coïncide avec le vœu retenu.
- **F4** : la neutralisation de la sentinelle dans deux fixtures (`passe3`, `revue`) échoue franchement si le déclencheur n'existe plus.
- Vérifié en local : clippy 1.99 (3 crates, `labo`) sans avertissement ; `tests_inscriptions*` : 137 verts.
- **En file (suites I5, passe 6)** : **M2** (rejeu de 304 et restauration de données : la sentinelle bloque un INSERT de reprise sans verrou → garde de rejeu ou procédure de restauration, test de rejeu sur une base avec un dossier admis sans décision), M4 (coût de `pg_locks` sous charge : mesure en pilote, `FOR EACH STATEMENT` si besoin), F1 (`ENABLE ALWAYS TRIGGER` ; la sentinelle est un filet, pas un contrôle de sécurité), F2 (phrase sur `set_config` : `purge_voulue` le lit), F3 (`ent_inscription_liste_attente` sur 10 000 dossiers), F5 (en-tête « aucune heure réelle » à adoucir), F6 (alerte sans destinataire : repli ou trace), F7 (archivage d'offre sans verrou, antérieur).

## Suites, passe 6 — liste « En file (suites I5, passe 6) » — branche `ent-socle-3-code-i5-suites-6`

Départ : `origin/ENT`. **313 à 316 ne changent pas** (règles 25 et 38) ; tout passe par la **317** (`317_inscription_i5_suites_passe6`), ajoutée **à la fin** du tableau de `migrate.rs` (après 494). Ses fonctions portent `SET search_path = pg_catalog, public, pg_temp` et qualifient leurs relations `public.`. `migrations_versees.txt` n'est pas touché (le pilote l'ajoute au versement). Statut : **implémenté et testé en local** (PostgreSQL 16 jetable, base neuve dont le nom contient « test ») ; **non validé** tant que la CI n'a pas rendu son verdict ; non prêt pilote, non prêt production.

### Fait

| N° | Fait |
|---|---|
| **M2** | Le rejeu de 304 ne bute plus sur la sentinelle. Cause : au rejeu, 304 refait sa reprise (`INSERT INTO ent_inscription_decision … SELECT`, une décision par dossier admis qui n'en a pas) alors que la sentinelle existe déjà et exige un verrou d'offre que cet INSERT ne tient pas. **Garde de rejeu** (`GARDES_DE_REJEU`, comme 644, 2824 et 2412) : avant 304, les déclencheurs `ent_inscription_sentinelle_*` de la table des décisions sont retirés ; 316 puis 317, rejouées plus loin dans le même rejeu, les recréent. Les migrations ne sont pas modifiées (règle 25) ; la production n'exécute jamais ce tableau. **Restauration de données** : procédure ci-dessous. |
| **M4** | **Mesure reproductible** (test `s6_mesure_surcout_sentinelle`, `--nocapture`) puis **passage en `FOR EACH STATEMENT`** (tables de transition) : le surcoût par ligne était notable. Voir le tableau. Une vérification **par offre distincte** de l'instruction ; mêmes règles qu'en 316 (offre de la ligne ou de la liste de son dossier, ancienne **et** nouvelle offre sur une mise à jour, exemption de suppression d'un dossier purgé, même erreur nommée, SQLSTATE `55000`). Les déclencheurs par ligne de 316 sont retirés ; les sept nouveaux s'appellent `ent_inscription_sentinelle_{proposition,decision}_{ins,upd,del}_trg` et `ent_inscription_sentinelle_offre_upd_trg`. |
| **F1** | `ENABLE ALWAYS TRIGGER` sur les sept déclencheurs : `SET session_replication_role = replica` (testé) ne la coupe plus ; seuls `DISABLE TRIGGER` (propriétaire de la table) ou `DISABLE TRIGGER ALL` (`pg_restore --disable-triggers`, superutilisateur) la suspendent, explicitement. **La sentinelle est un filet contre l'oubli du verrou dans le code, pas un contrôle de sécurité** : le propriétaire de la table et le superutilisateur la suspendent, et l'exemption de suppression (F2) dépend d'un réglage de session. Le contrôle de sécurité reste le cloisonnement par établissement (RLS) et les droits des rôles. |
| **F2** | La phrase de 316 et de la section « passe 5 » (« aucune garde ne lit `set_config` ») est corrigée : vraie pour la **preuve du verrou**, fausse pour l'**exemption de suppression**, qui lit `ent_inscription_purge_voulue()` (réglage de session `ent.purge_etablissement`, que tout appelant peut poser). Cette exemption ne couvre que la **suppression** de lignes de décisions et de propositions (jamais leur création ni leur modification), avec la même limite que `ent_inscription_ajout_seul`, qui lit le même réglage. Le fichier 316 lui-même n'est pas modifié (versé) : sa phrase est rectifiée dans l'en-tête de 317. |
| **F3** | `ent_inscription_liste_attente` n'appelle plus `ent_inscription_offre_du_dossier` pour chaque dossier de l'établissement : l'offre se lit par **jointure** sur le vœu retenu, et la fonction par dossier de la famille (`ent_inscription_en_liste_visible`, requête récursive) ne s'exécute que sur les candidats **de l'offre** (CTE matérialisée). Même résultat, même formule d'ordre (seul endroit où elle est écrite). **Vérifié par un compte d'appels, pas une durée** (voir les tests). |
| **F5** | L'en-tête « aucun test ne dépend de l'heure réelle » (cinq fichiers de tests d'I5) était inexact : le service et la base lisent encore `now()` (horodatages de ligne) et le jour courant (`ent_acces_ouvert`). Il dit maintenant ce qui est vrai : les instants passés au **service** sont posés (2027) ; aucune assertion ne compare l'horloge de la base à un instant de test ; les dates qui en dépendent sont fixées très loin du présent (2020, 2099). |
| **F6** | `alerter_direction_caduque_tx` : si **aucun** destinataire n'a un accès ouvert (direction partie, auteur parti), **repli** sur les accès ouverts de l'administrateur et du secrétariat (`admin`, `administratif`, qui instruisent les dossiers) ; s'il n'y en a pas non plus, l'alerte est **tracée** dans le suivi interne du dossier (« Alerte de décision caduque sans destinataire… », jamais visible de la famille). Plus d'alerte perdue en silence. |
| **F7** | `archiver_offre_for_actor` prend, comme `modifier_offre`, la **campagne puis le verrou de l'offre** (`verrouiller_dans_l_ordre_tx`) **avant** de compter les vœux : le décompte est lu sous le verrou. |

### Mesure M4 (surcoût de la vérification, par instruction)

Table temporaire qui appelle la **même** fonction `ent_inscription_exiger_verrou_offre`, deux déclencheurs (par ligne = 316, par instruction = 317), verrou de l'offre tenu ; médiane de 5 passes d'un `UPDATE` de N lignes, PostgreSQL 16 local, un seul passage (ordres de grandeur, pas une garantie) :

| Lignes par instruction | Verrous d'une autre session | Par ligne (316) | Par instruction (317) |
|---|---|---|---|
| 1 | 0 | 0,83 ms | 0,66 ms |
| 100 | 0 | 6,6 ms | 0,75 ms |
| 1 000 | 0 | 62 ms | 1,6 ms |
| 1 | 500 | 1,0 ms | 0,77 ms |
| 100 | 500 | 31 ms | 0,99 ms |
| 1 000 | 500 | **237 ms** | 2,2 ms |

Lecture : par ligne, le coût croît avec les lignes **et** avec les verrous de l'instance (lecture de `pg_locks` à chaque ligne) ; par instruction, il est quasi constant (une lecture par offre distincte). Pour une instruction d'une ligne, le gain est faible (de l'ordre de 0,2 ms) ; il est décisif pour les écritures de masse (publication de séance, reprise). Les temps sont **imprimés**, jamais assertés.

### Procédure de restauration de données (M2, F1)

* **Restauration complète** (`pg_dump` / `pg_restore` de la base entière) : les déclencheurs sont recréés **après** les données (section post-data) ; rien à faire, `ENABLE ALWAYS` est conservé.
* **Reprise ou restauration de données seules** (INSERT de reprise, import, `pg_restore --data-only`) dans les trois tables protégées (`ent_inscription_offre` pour la capacité et les places rendues, `ent_inscription_proposition_place`, `ent_inscription_decision`) : la sentinelle refuse l'écriture sans verrou d'offre. Deux voies :
  1. **Préférée quand les offres concernées sont peu nombreuses** : la transaction de reprise prend le verrou de chaque offre (`SELECT pg_advisory_xact_lock(ent_inscription_offre_cle(<offre>))`) ; la sentinelle laisse passer, aucune suspension.
  2. **Reprise de masse** : en une transaction, par le propriétaire des tables, `ALTER TABLE … DISABLE TRIGGER` sur les sept déclencheurs `ent_inscription_sentinelle_*`, reprise, puis `ALTER TABLE … ENABLE ALWAYS TRIGGER` sur les sept, et contrôle : `SELECT tgname, tgenabled FROM pg_trigger WHERE tgname LIKE 'ent\_inscription\_sentinelle\_%'` doit rendre **7 lignes, toutes à `A`**. `session_replication_role = replica` **ne suffit plus** (voulu).
* Jamais sur une base de production sans validation du pilote (CLAUDE.md : aucun accès à la production dans ce dépôt).

### Tests (`tests_inscriptions_i5_suites6.rs`, 8)

| Test | Prouve |
|---|---|
| `s6_rejeu_de_304_dossier_admis_sans_decision` | **M2** : `rejouer_tout` sur une base qui contient un dossier **admis sans décision** passe ; la reprise a posé la décision ; sept déclencheurs de sentinelle de retour, tous `ALWAYS`, plus aucun déclencheur par ligne ; une écriture sans verrou est de nouveau refusée |
| `s6_sentinelle_toujours_active_et_restauration` | **F1** : sept déclencheurs à `A` ; `session_replication_role = replica` ne contourne pas ; la suspension explicite laisse passer une reprise sans verrou, le rétablissement `ALWAYS` refuse de nouveau |
| `s6_sentinelle_par_instruction_exige_chaque_offre` | **M4** : une instruction qui écrit les places de deux offres exige les deux verrous (aucun, l'un, l'autre : refus ; les deux : accepté) |
| `s6_mesure_surcout_sentinelle` | **M4** : la mesure ci-dessus ; les deux variantes refusent sans verrou et acceptent avec |
| `s6_liste_attente_borne_d_appels_sur_10000_dossiers` | **F3** : 10 000 dossiers en liste d'attente (50 sur l'offre lue) ; **0 appel** de `ent_inscription_offre_du_dossier` ; `ent_inscription_en_liste_visible` appelée **50 fois** (une par dossier de l'offre) côté famille ; positions 1 à 50 ; **preuve rouge** : la formule de 316 faisait ≥ 10 000 appels. Compte d'appels (`pg_stat_xact_user_functions`), aucune assertion de durée |
| `s6_alerte_sans_destinataire_repli_puis_trace` | **F6** : direction et auteur partis (date de fin 2020) : le secrétariat est alerté ; plus personne : trace interne « sans destinataire », invisible de la famille |
| `s6_archivage_d_offre_attend_le_verrou` | **F7** : l'archivage attend le verrou de l'offre (`pg_locks`), l'offre n'est pas archivée pendant l'attente, puis il aboutit |
| `s6_archivage_lit_les_voeux_sous_le_verrou` | **F7** : un vœu posé pendant l'attente est vu (décompte sous verrou) : archivage refusé, offre intacte |

Adaptations des tests existants : les deux fixtures qui neutralisaient la sentinelle (`passe3`, `revue`) suspendent maintenant les trois déclencheurs de la table des décisions ; les trois tests `s5_sentinelle_*` tiennent la garde de rejeu **partagée** (le rejeu de 304 retire puis recrée la sentinelle : sans elle, un rejeu parallèle pouvait la faire disparaître au milieu d'un test).

### Décisions par défaut prudentes (D-069, règles métier non tranchées)

* **Alerte de décision caduque sans destinataire (F6)** : repli sur `admin` et `administratif` à accès ouvert (rôles qui instruisent les dossiers), puis trace interne. À confirmer : faut-il plutôt un autre rôle (par exemple une fonction « responsable des admissions ») ?
* **Archivage d'une offre qui porte des vœux (F7)** : refusé (comportement d'avant, conservé), le décompte étant maintenant lu sous le verrou.
* **Reprise de masse (M2)** : la suspension explicite de la sentinelle est réservée au propriétaire des tables, dans une transaction, avec contrôle final obligatoire (7 déclencheurs à `A`).

### Non fait / limites

* **Dépôt de préinscription et vœux** : l'insertion d'un vœu par le **dépôt** (`deposer_preinscription`) ne prend pas le verrou de l'offre (elle ne touche ni places, ni liste, ni décisions) ; un archivage d'offre concomitant d'un dépôt qui cible cette offre peut donc encore laisser un vœu sur une offre archivée (le dépôt relit `archivee_le IS NULL` sans verrou). Fermer cette fenêtre demande soit un verrou d'offre au dépôt (sérialise les dépôts d'une même offre), soit un déclencheur sur les vœux ; décision à prendre (non tranchée, rien d'ajouté). Le chemin « décision » qui crée un vœu tient déjà le verrou (couvert par F7).
* **M4** : les chiffres sont d'une machine de développement, un seul passage ; la mesure en pilote (nombre réel de verrous de l'instance) reste à faire, mais le déclencheur par instruction rend le coût quasi indépendant du nombre de lignes.
* Sept tests sans rapport (`i4_reponse_en_ligne_sans_compte`, `fa_`/`fb_`/`e1_` d'internat I4, `migrations_le_rejeu_avec_des_donnees_d_i4…`, `p3_m2_les_bornes_reelles…`, `migration_2220_remplace…`) ont échoué **une fois** sur 447 à quatre fils en parallèle sur la machine de développement (délai d'instruction dépassé en attendant la garde de rejeu, et un test sensible à une transaction ouverte voisine) ; **tous verts seuls** (7/7). La CI isole chaque tranche.

### Suites (passe 6) — vérifications, compteurs, tête et CI
- **Compteurs (règle 28)** : `MIGRATIONS` +1 (317) ; aucune nouvelle table, action ni source ; compte RLS, `Action::TOUTES`, `SOURCES` et `MIGRATION_TAG` (3808) inchangés ; `migrations_versees.txt` inchangé (préfixe du tableau : 317 est après 494, dernière versée de la liste).
- **Qualité** : clippy 1.99.0 sans avertissement sur les 3 commandes de la CI (`ent-lab` compris, `-D warnings`) ; `cargo metadata --locked` dans `ent-rust/web` ; formatage ciblé seulement sur le fichier neuf (`rustfmt` ; le backend existant est une dette de format connue et non bloquante, pas de `cargo fmt --all`) ; gardes statiques (`garde_inscriptions_ecritures`, `isolation_sql_statique`, `jour_utc_interdit`) et 592 tests de la bibliothèque hors `tests_*` (dont `garde_search_path`, registre, ordre des migrations) verts.
- **Tests** : base neuve `ent_i5s6_test`, un seul `CARGO_TARGET_DIR`, `--locked`, toolchain 1.99.0, `ENT_ENV=lab`. Filtres `inscription r_m4 registre migrat d036 export rgpd p2_ i5_ p3_ p4_ i3_ i4_ s5_ s6_ garde_search_path` : 447 tests, 440 verts à quatre fils, les 7 autres (sans rapport, voir « Non fait ») verts seuls ; les 8 `s6_*` et les `s5_*` verts.
- **Compteurs (règle 28)** : `MIGRATIONS` +1 (317, en fin de tableau) ; tables, actions, sources, RLS et MIGRATION_TAG (3808) inchangés ; `migrations_versees.txt` non modifié.
- **Tête de code et CI** : tête `6ae1d2a2ff4c13ea6c61b13daeca0e39debb8a72`, CI « ENT Rust » (run 45369838911) **verte** (`success`). Le commit qui consigne ce résultat ne change que ce document.

### Correctifs du pilote après contre-revue (passe 6, 2026-11-24)
Rapport complet : `docs/pilotage/revues/03_I5_SUITES_6_CONTRE_REVUE.md` (branche `ent-mc0hoe`). Aucun Élevé. Équivalence de la
sentinelle par instruction confirmée (ancienne ET nouvelle offre, purge exemptée, 55000) ; F7 sans interblocage ; course
dépôt/archivage fermée par le verrou de campagne (`FOR SHARE` contre `FOR NO KEY UPDATE`).
- **Moyen A, restauration** : `pg_restore --disable-triggers` se termine par `ENABLE TRIGGER ALL`, qui remet les déclencheurs
  à l'état « origine » et **fait perdre `ENABLE ALWAYS`**. Toute restauration de ce type se termine donc par
  `ALTER TABLE … ENABLE ALWAYS TRIGGER …` sur les sept déclencheurs de la sentinelle, puis le contrôle
  `SELECT tgname, tgenabled FROM pg_trigger WHERE tgname LIKE 'ent\_inscription\_sentinelle\_%'` (toutes à `A`).
- **Moyen B, réplication logique** : côté abonné (PRA, changement d'hébergeur), le worker d'application tourne en rôle
  `replica` sans verrou consultatif : une sentinelle `ALWAYS` y bloquerait la réplication (55000). Sur un abonné, les sept
  déclencheurs sont passés en `DISABLE` (la cohérence est garantie par la base source). À reprendre dans la procédure PRA.
- En file : Moyen C (la garde de rejeu de 304, tests seulement, peut laisser la sentinelle absente si un rejeu échoue :
  la recréer en cas d'échec), Moyen D (appel par ligne résiduel de `ent_inscription_offre_du_dossier` → jointure ; test
  multi-lignes et deux offres sur la fonction réelle), F-1 (rejeu de 313/314 sur un historique non chaîné), F-2 (repli de
  l'alerte par **droit** `InscriptionsGerer` et non par rôle brut), F-3 (échec ouvert hérité de 316).
- `317_inscription_i5_suites_passe6` inscrite à `migrations_versees.txt` au versement.

## Suites, passe 7 — points « en file » de la contre-revue de la passe 6 — branche `ent-socle-3-code-i5-suites-7`

Départ : `origin/ENT`. **313 à 317 ne changent pas** (règles 25 et 38) ; tout passe par la **318** (`318_inscription_i5_suites_passe7`), ajoutée **à la fin** du tableau de `migrate.rs` (après 317). Sa fonction porte `SET search_path = pg_catalog, public, pg_temp` et qualifie ses relations `public.`. `migrations_versees.txt` n'est pas touché (le pilote l'ajoute au versement). Statut : **implémenté et testé en local** (PostgreSQL 16 jetable, base neuve dont le nom contient « test ») ; **non validé** tant que la CI n'a pas rendu son verdict ; non prêt pilote, non prêt production.

### Fait

| N° | Fait |
|---|---|
| **Moyen D** | `ent_inscription_sentinelle_instruction` (318, `CREATE OR REPLACE` : les déclencheurs de 317, et leur `ENABLE ALWAYS`, sont conservés) lit l'offre d'une ligne sans `offre_id` par **jointure ensembliste** (`COALESCE(ligne.offre_id, dossier.offre_attente_id, voeu_retenu.offre_id)`, même règle que `ent_inscription_offre_du_dossier`) au lieu d'appeler cette fonction **ligne par ligne**. Une vérification (`pg_locks`) par offre distincte, ancienne **et** nouvelle offre sur une mise à jour, exemption de purge et erreur nommée `55000` inchangées. **La mesure M4 exerce maintenant les fonctions réelles** (316 `ent_inscription_sentinelle_ligne`, 318 `ent_inscription_sentinelle_instruction`) sur une table temporaire, plus une fonction de remplacement. |
| **Moyen C** (tests) | `rejouer_tout` délègue à `rejouer_liste` ; si un rejeu échoue (après qu'une garde a retiré la sentinelle), `restaurer_sentinelle` rejoue 316, 317 et 318 (qui la recréent), journalise un éventuel échec de restauration, et rend l'erreur d'origine : la sentinelle n'est jamais laissée absente. |
| **Moyen A, suite** | `migrate::sentinelles_non_toujours_actives` (appelée par `apply_socle_migrations` après les migrations, comme `contraintes_non_validees`) journalise en **`error!`**, sans bloquer, chaque déclencheur `ent_inscription_sentinelle_*` dont `tgenabled <> 'A'` (suspendu `D`, ou remis à « origine » `O` par `pg_restore --disable-triggers`). |
| **Faible 1** | Rejeu de 313 et 314 sur un historique non chaîné (reprise de `publiee_le`, chaînage `remplace_decision_id`, écritures sans verrou d'offre) : **même garde de rejeu que 304** (`RETIRER_SENTINELLE_DECISIONS`) ; 316, 317 et 318 recréent la sentinelle plus loin dans le même rejeu. Test : `s6_rejeu_de_304…` et le test d'échec de rejeu. |
| **Faible 2** | Repli de l'alerte de décision caduque (F6) : les personnes à accès ouvert qui **détiennent le droit `InscriptionsGerer`** sur l'établissement entier (modèle par défaut du rôle, plus attributions de rôle en cours au jour de l'établissement), et non plus `role IN ('admin','administratif')`. Un CPE sans le droit n'est pas alerté ; une attribution échue ne compte pas. **Limite** : les délégations datées ne sont pas consultées (pas de destinataire désigné par délégation). |
| **Faible 3** | **Documenté comme filet, non fermé.** Si le dossier d'une ligne est invisible de l'écrivain (isolation par établissement, dossier absent), l'offre est NULL et aucun verrou n'est exigé (échec ouvert hérité de 316). Le fermer casserait la suppression en cascade d'un dossier déjà disparu au moment du déclencheur ; cohérent avec le rôle de la sentinelle (un filet contre l'oubli du verrou, pas un contrôle de sécurité). Écrit dans l'en-tête de 318. |
| **Faible 4** | Les tests de passe 4 et de la suite I5 n'ont **pas été modifiés** par cette passe ; ils sont rejoués en entier (voir les vérifications) et verts. Aucune anomalie relevée. Seuls changements de tests existants : trois helpers de `tests_inscriptions_i5_suites6.rs` passent en `pub(super)`, et la mesure M4 utilise les fonctions réelles. |

### Mesure M4 refaite avec les fonctions réelles (µs par instruction, médiane de 5, un passage, machine de développement)

| Lignes | Verrous d'une autre session | Par ligne (316) | Par instruction (318) |
|---|---|---|---|
| 1 | 0 | 351 | 507 |
| 100 | 0 | 3 909 | 530 |
| 1 000 | 0 | 35 960 | 1 403 |
| 1 | 500 | 484 | 768 |
| 100 | 500 | 25 241 | 1 368 |
| 1 000 | 500 | **146 361** | 2 903 |

Lecture : pour **une seule ligne**, la variante par instruction est un peu **plus lente** (≈ 0,15 à 0,3 ms : tables de transition) ; le gain est décisif dès quelques dizaines de lignes (écritures de masse). Les temps sont **imprimés**, jamais assertés ; ordres de grandeur, pas une garantie.

### Tests (`tests_inscriptions_i5_suites7.rs`, 4)

| Test | Prouve |
|---|---|
| `s7_sentinelle_multi_lignes_sans_offre_id_deux_offres` | **Moyen D** : 800 lignes sans `offre_id` sur **deux offres**, fonction **réelle** : aucun verrou / l'un / l'autre → refus `55000` nommé, les deux → accepté (insertion, mise à jour, changement de dossier = ancienne **et** nouvelle offre, suppression) ; **0 appel** de `ent_inscription_offre_du_dossier` et **4 vérifications** (une par offre distincte, avant/après) ; **preuve rouge** : le déclencheur par ligne de 316 appelle la fonction ≥ 800 fois |
| `s7_rejeu_en_echec_restaure_la_sentinelle` | **Moyen C** : rejeu de 1 à 304 suivi d'un fichier qui échoue : erreur d'origine rendue, sept déclencheurs de retour, tous `ALWAYS`, écriture sans verrou refusée |
| `s7_sante_sentinelle_signale_un_declencheur_non_always` | **Moyen A** : un déclencheur « origine » (`O`) et un suspendu (`D`) sont repérés par la requête de santé (dans une transaction annulée) ; en état normal `sentinelles_non_toujours_actives` ne rend rien. Le **message** `error!` n'est pas capturé (seul le retour l'est) |
| `s7_alerte_repli_par_le_droit_et_non_par_le_role` | **Faible 2** : CPE sans droit non alerté, CPE avec attribution `inscriptions.gerer` alerté, attribution échue (2020) non alertée |

### Décisions par défaut prudentes (D-069)
* **Repli de l'alerte** : par le droit `InscriptionsGerer` (pas de rôle « responsable des admissions » inventé) ; sans destinataire, trace interne (inchangé).
* **Échec ouvert** (dossier invisible) : conservé et documenté (filet).
* **Vérification de santé** : journalise, ne bloque pas le démarrage.

### Non fait / limites
* Les points « doc du pilote » (moyens A et B de la contre-revue : restauration, réplication logique) sont déjà consignés plus haut ; la **procédure PRA** reste au pilote.
* Les délégations datées ne désignent pas de destinataire de l'alerte (limite du repli par le droit).
* Mesure M4 : un seul passage, machine de développement.

### Suites (passe 7) — vérifications, compteurs, tête et CI
- **Compteurs (règle 28)** : `MIGRATIONS` +1 (318, en fin de tableau, après 317) ; aucune nouvelle table, action ni source ; compte RLS, `Action::TOUTES`, `SOURCES` et `MIGRATION_TAG` (3808) inchangés ; `migrations_versees.txt` non modifié (318 est après 494, dernière versée de la liste).
- **Qualité** : clippy 1.99.0 sans avertissement sur les 3 commandes de la CI (`ent-lab` compris, `-D warnings`) ; `cargo metadata --locked` dans `ent-rust/web` ; formatage ciblé du seul fichier neuf (`rustfmt`), pas de `cargo fmt --all`.
- **Tests** : base neuve `ent_i5s7_test` (PostgreSQL 16 local), un seul `CARGO_TARGET_DIR`, `--locked`, toolchain 1.99.0, `ENT_ENV=lab`. Filtres `inscription r_m4 registre migrat d036 export rgpd p2_ i5_ p3_ p4_ i3_ i4_ s5_ s6_ s7_ garde_search_path isolation_sql jour_utc` : 459 tests, 451 verts à quatre fils ; les 8 autres (sans rapport : `d036`, `evaluations`, `i4_*`, `internat_i4_*`) viennent de la base partagée entre tests parallèles : 7 verts rejoués ensemble, le dernier (`d036::les_treize_colonnes…`) vert seul sur base neuve. Les 4 `s7_*`, les `s6_*` et `s5_*` verts. La CI isole chaque tranche.
- **CI, 1re tête (`d90c2a44`, run 45471443083)** : **rouge**, tranche 3/20 : `tests_referentiel_s2::rattachement_idempotent_et_comparaison_avant_apres` (« cannot update view ent_devoir »). **Cause** : mon test `s7_rejeu_en_echec_restaure_la_sentinelle` rejouait le **préfixe** des migrations jusqu'à 304 puis échouait volontairement ; les objets rejoués restaient dans leur version d'alors pour les tests suivants de la base de la tranche. Reproduit en local (même ordre), corrigé : rejeu partiel réduit à 304 + le fichier qui échoue, puis `rejouer_tout` de remise en état en fin de test. Aucun code de production touché.
- **Tête de code et CI** : tête `96f662c2df7a4fbe86e0843c7da3d53e84bc93f8`, CI « ENT Rust » (run 45487404837) **verte** (`success`). Le commit qui consigne ce résultat ne change que ce document.

### Correctifs du pilote après contre-revue de la passe 7
Rapport : `docs/pilotage/revues/03_I5_SUITES_7_CONTRE_REVUE.md` (branche `ent-mc0hoe`). Aucun élevé.
- `restaurer_sentinelle` : nouvelles tentatives sur 40P01/40001 comme le rejeu ; échec définitif écrit sur la sortie d'erreur.
- Vérification de santé : liste attendue `SENTINELLES_ATTENDUES` (7 déclencheurs) ; un déclencheur **absent** est signalé (`absent`) ; la fonction accepte tout exécuteur et le test l'appelle sur la transaction (O, D et absent).
- `s7_rejeu_en_echec` : SQL neutre sous le nom de 304 (la garde se déclenche par le nom) ; plus de rejeu de 304 ni de rejeu complet de remise en état.
- Repli F6 : les attributions ne comptent que pour un rôle de base du personnel (garde-fou 1 de `droits_effectifs`).
- Local : 212 tests `inscription` + `migrat` verts, clippy 1.99 vert. 318 inscrite aux migrations versées.
