# Socle 41, lot I3 — correctifs de la revue de sécurité du pilote

Branche `ent-socle-41-code-i3` (PR n° 106, brouillon), partie de la tête distante ce150278. Migration **2826 modifiée** (non versée) ; 2824 et 2825 intactes. Niveau : **implémenté, testé en local** ; ni validé, ni prêt pilote. Frontend non touché (règle 27).

Tests : PostgreSQL 16 local, bases **neuves** (`ent_i3d`, puis `ent_i3e` pour la suite large), `ENT_EXIGER_BASE=1`, Rust 1.99.0, `--locked`. Aucun test ne dépend de l'heure réelle (jours fixes d'avril 2026, instants passés à chaque commande ; seules des levées d'interdiction en base utilisent `now()` comme les tests existants).

## Constat par constat

| Constat | État | Ce qui a été fait | Tests |
|---|---|---|---|
| **C-1.1** parent interdit = pas acteur ni lecteur | fait | `titulaire_tx` écarte tout lien dont la personne est visée (chaîne de fusions **dans les deux sens**, objets remise/visite/contact_eleve, non levée, non terminée, `du` ignoré). `famille_tx` (acteur) et `famille_lecture_tx` (lecteur) : annonce, accord, opposition, annulation, correspondants, retard, révocation refusés ; lectures (absences, sorties, correspondants, autorisations, historique) refusées. | `c1_parent_interdit_n_est_ni_acteur_ni_lecteur_ni_personne_designee` (3 objets, levée, interdiction terminée), `c1_chaine_de_fusion_porte_l_interdiction_dans_les_deux_sens` |
| **C-1.2** mineur : personne qui récupère | fait | Annonce d'un mineur : personne exigée, sauf départ seul couvert par l'accord annuel `week_end_seul` (socle 18) ; contrôle de remise appliqué aussi quand l'annonceur est la personne. | `c1_mineur_exige_la_personne_qui_recupere_meme_pour_l_annonceur` |
| **C-1.3** départ d'une absence | fait | Nouveau `controler_depart_absence_a` (route `POST /internat/absences/{id}/depart`, droit `internat.appel`) : personne désignée seule, interdiction (tracée), accord annuel relu pour le départ seul ; absences de mineurs sur la **liste de remise** du jour (`absence_id`). La revue à la minute **bloque** l'absence (personne devenue interdite/non autorisée, accord annuel retiré) ; accepter ou lever un blocage ne rend pas valable une remise interdite. Colonnes `depart_effectif`, `depart_remis_a_personne_id`. | `c1_depart_d_une_absence_…`, `c1_interdiction_posee_apres_l_annonce_bloque_l_absence` |
| **E-1** majeur | fait | Parents d'un majeur : plus acteurs ; lecture seulement avec `majeur_info_parents = toujours` (« si_a_charge » ne vise que les payeurs, notion non lue ici : refusé, choix prudent). | `e1_majeur_parents_ni_acteurs_…` |
| **E-2** retard | fait | Retard qui fait couvrir la nuit à une sortie qui ne la couvrait pas : **demande au CPE sans effet** (`retard_demande_fin`, décisions `accepter_retard`/`refuser_retard`, visible dans la file) ; plafond = durée maximale du type + **6 h** (marge) ; seuls un titulaire avec `peut_autoriser_sorties`, l'élève majeur ou le CPE annoncent. | `e2_retard_…` |
| **E-3** attendu | fait | `ent_internat_attendu_calcule` : seules les sorties `partie`/`rentree` retirent l'élève. Les tests I3 qui supposaient l'inverse sont corrigés. | `e3_…`, tests I3 adaptés |
| **M-1** remise à la personne désignée / changement = opposition | fait | Autre personne que la désignée (ou aucune) : refus tracé, appel du CPE. Un titulaire qui change la personne d'un autre titulaire (désignation ou accord) : sortie bloquée, ancienne personne conservée. | `moyen_1_…`, `e3_…` |
| **M-2** passer « seul » | fait | Accord remis à zéro (fait `accord_remis_a_zero`) ; l'accord annuel le remplace sinon retour à la demande (transitions autorisée/suspendue → demandée ajoutées à la garde). | `moyen_2_et_4_…` |
| **M-3** accord annuel relu | fait | Au départ (seul ou accompagné) et dans `reviser_sorties_tx` (devenu méthode de `PgEnt`). | `moyen_3_…` |
| **M-4** mineur seul | fait | `exigences(type, majeur, seul)` : accord de la famille ou accord annuel toujours exigé, même décideur `cpe`. | `moyen_2_et_4_…` |
| **M-5** après le début de la nuit | fait (absences) | `blocage_effet_nuit` : l'absence bloquée continue de couvrir la nuit en cours, les suivantes redeviennent attendues ; événement `nuit_en_cours`, tête de file du CPE. Pour les sorties, E-3 rend le sujet sans objet (une sortie non partie ne retire déjà plus l'élève ; une sortie partie n'est plus opposable). | `moyen_5_…` |
| **M-6** textes montrés | fait | Famille/élève : texte d'un auteur `cpe` ou `direction` seulement (DTO sortie et historique). | `moyen_6_…` |
| **M-7** contact au lien suspendu | fait | Branche contact de `remise_possible_tx` exclut un lien suspendu (chaîne comprise). | `moyen_7_…` |
| **M-8** capacité | fait | `eleve_majeur_tx` : `ent_eleve_majorite_le` + émancipation (traité en majeur) ; tutelle (le tuteur décide). | `e1_…` |
| Faible : santé « FİEVRE » | fait | Refiltrage des marques après minuscule avec le même ensemble (`a_ignorer`). | `domain-eleve` `repli_ligatures…` |
| Faible : antidate | fait | `heure_du_geste` : pas plus de 30 min en arrière ; retour antidaté ramené au matin déjà écoulé. | `faible_geste_antidate_…` |
| Faible : doublons d'absence | fait | Contrainte d'exclusion gist en base + mappage 23P01. | `faible_doublons_…` (concurrence + SQL) |
| Faible : liste de remise protégée | fait | Élève caché : aucun titulaire ni contact listé. | `faible_liste_de_remise_…` |
| Faible : F-1 | fait | `lister_listes_a` ne rend plus `confie_a_user_id` si l'accès est terminé à la date de lecture. | `f1_…` (étendu) |
| Faible : M-1 | fait | Complément ouvert ignoré avec mention (`complements_ignores`) ; cas `non_couvert` pour le CPE (une fois par élève et nuit). | `faible_m1_…` |
| Faible : test faible | fait | Remplacé par `Ok([])` exact (cloisonnement RLS). | `droits_cloisonnement_…` |

Choix à valider : l'absence bloquée par la revue **ne reprend pas d'elle-même** (la direction tranche) ; sorties inchangées (reprise automatique).

## Tests locaux
`tests_internat_i3` : **43 verts** (25 d'origine adaptés + 18 nouveaux). Suite large sur base neuve (`internat rls registre migrat fusion conservation rgpd accords d036 roles`) : **304 verts**. `domain-eleve` internat : 14 verts. `ent-api` `tests_internat` : 5 verts. Clippy 1.99 `-D warnings` (3 commandes CI) et `cargo fmt -p ent-types --check` : voir ci-dessous. Non rejoué en local : suite complète d'`ent-db`, frontend.

## Compteurs (règle 28), recalculés
Tables **0** nouvelle (RLS +5, registre données +5, `TABLES_ELEVE` +3 : inchangés depuis la livraison) ; migration 2826 (modifiée) ; `MIGRATION_TAG` 3808 inchangé ; `Action::TOUTES` 275 ; événements +15 inchangés (le départ d'une absence réutilise `internat.sortie_partie` avec `objet: absence`) ; **routes +1 (36 pour I3)**, ajoutée au test « aucune route sans session » ; `NOMS_SOURCES`/`SOURCES` 68. Natures de cas : `non_couvert` ajoutée.

## Tête et CI
Voir le message au pilote.

---

# Passe 2 — contre-revue du pilote (E-A, M-A, M-B, F-A à F-F)

Même branche (`ent-socle-41-code-i3`, PR n° 106, brouillon), partie de 1e400696. Migration **2826 modifiée** (non versée) ; 2824 et 2825 intactes. Niveau : **implémenté, testé en local** ; ni validé, ni prêt pilote. Frontend non touché (règle 27). Tests : PostgreSQL 16 local, bases neuves, `ENT_ENV=test`, Rust 1.99.0, `--locked` ; aucun test ne dépend de l'heure réelle (les dates de pose d'interdiction et de création de lien sont fixées par UPDATE aux jours d'avril 2026).

## Constat par constat

| Constat | État | Ce qui a été fait | Tests |
|---|---|---|---|
| **E-A** ce qu'un parent a fait avant son interdiction ne reste pas effectif | fait | Helper `auteur_interdit_depuis_tx` : le compte (fiches `ent_personne.user_id` et liens avec l'élève) est-il **désormais** visé, sur toute la chaîne de fusion, par une interdiction non levée et non terminée de remise, visite ou contact ? **Sortie** : si l'auteur de l'accord (à défaut, de la demande) est visé, l'accord et la décision du CPE sont remis à zéro, la sortie retourne à « demandée » (acte `accord_remis_a_zero`, motif `interdiction_auteur`, événement de décision) ; au départ le départ est refusé et tracé. **Absence** : l'annonceur visé → absence « bloquée » (motif `interdiction_auteur`), départ refusé ; la direction tranche, et sa décision (`tranchee_levee`) postérieure à l'interdiction n'est pas rebloquée. **Correspondant** : désigné par un compte visé → suspendu (motif `interdiction_auteur`, événement), les sorties chez lui suivent par le contrôle existant ; seul le CPE le reprend (la revue respecte cette reprise). Au départ **et** à la revue à la minute. | `ea_sortie_seule_…`, `ea_absence_annoncee_…`, `ea_correspondant_designe_…` (chacun : départ et revue) |
| **M-A** interdiction sans personne identifiée | fait | (1) Règle D25 reprise de la messagerie (481) dans `titulaire_tx` : un lien créé **après** une interdiction non nominative non levée, non terminée, sans décision `confirme` (`ent_msg_lien_decision`) n'est ni acteur ni lecteur. (2) Tant qu'une interdiction non nominative d'objet `remise` est en vigueur, un mineur ne part pas seul : sortie ou absence **bloquée** (motif `interdiction_remise`) au départ et à la revue ; la direction qui tranche (« lever ») vaut décision (postérieure à la pose) | `ma_lien_cree_apres_…`, `ma_mineur_ne_part_pas_seul_…` |
| **M-B** absences et attendu | fait | **Pas de départ constaté = attendu.** L'attendu (migration 2826) ne retire un mineur pour absence qu'à partir de `depart_effectif` (antérieur à l'ancre de la nuit) ; le majeur et la régularisation (nuit passée) retirent dès l'annonce. Fonction SQL `ent_internat_eleve_majeur` (source unique, relue par Rust `eleve_majeur_tx`). `blocage_effet_nuit` : posé (opposition) seulement si l'élève est réellement parti. La revue ne regarde que les absences sans départ : leur blocage ne change pas l'attendu. Veille : une absence de mineur sans départ à l'heure limite de la nuit en cours (même heure et seuil que le week-end) ouvre un cas **non récupéré** (une fois par nuit) | `mb_pas_de_depart_constate_…` (refus à 22 h 10 après interdiction à 22 h 05, absence sans départ, majeur) ; tests I3 existants adaptés |
| **F-A** retard | fait | `nuits_couvertes_tx` compte les nuits avant/après ; plus de nuits → demande au CPE | `fa_retard_…` |
| **F-B** accord `week_end_seul` | fait | Ne couvre le départ seul d'une absence que si la première nuit est un **vendredi** (annonce, départ, revue). Limite : une veille de jour férié n'est pas reconnue | `fb_…`, `c1_*` adaptés |
| **F-C** effacer la personne de l'autre titulaire | fait | `changement_vaut_opposition_tx` ne dispense plus le cas « aucune personne » | `fc_fd_ff_…` |
| **F-D** historique famille | fait | `interdiction_remise` → `personne_non_autorisee`, `interdiction_auteur` → `autre` côté famille et élève ; le personnel lit le vrai motif | `fc_fd_ff_…` |
| **F-E** majeur, `si_a_charge` | fait | Aucun événement `majeur_informe` quand la politique n'est pas `toujours` (lecture refusée aux parents). Le futur résolveur de destinataires du **socle 8** devra écarter les parents visés par une interdiction (remise, visite, contact) avant tout envoi : documenté dans le code | `r_it22_…` adapté |
| **F-F** branche « contact d'urgence » | fait | Un contact dont une fiche de la chaîne a un lien suspendu, en garde `aucune` ou **sans autorité parentale** ne prend pas l'élève. Limite à valider : un proche (grand-parent) qui aurait un lien sans autorité parentale ne passe plus par cette porte | `fc_fd_ff_…` |

Choix à valider : `interdiction_auteur` ne reprend jamais d'elle-même ; la décision revient au CPE (sortie « demandée » : un autre titulaire répond, le CPE décide) ou à la direction (absence, blocage M-A). Les actes appuyés sur l'autorisation annuelle (socle 18, sans compte auteur) ne sont pas revus par E-A. **[Remplacé par la passe 3 : les décideurs de l'autorisation annuelle sont des auteurs comme les autres, et E-A n'est plus qu'un cas de la règle générale « un acte tombe avec son auteur ».]**

## Tests locaux
`tests_internat_i3` : **52 verts** (43 + 9 nouveaux) sur base neuve. Suite large (`internat rls registre migrat fusion conservation rgpd accords d036 roles`, base neuve) : **312 verts sur 313** ; `tests_eleve_fusion::defusion_refusee_si_une_ligne_deplacee_a_change` a échoué une fois en parallèle (table `ent_absence_seance`, hors internat) et **passe seul** : flottement de concurrence à surveiller, non lié à ce lot. `ent-api` `tests_internat` : 5 verts. Clippy 1.99 `-D warnings` (3 commandes de la CI) : propre ; `cargo fmt -p ent-types --check` : propre (fmt non lancé sur `ent-db`, règle du dépôt). Non rejoué : suite complète d'`ent-db`, frontend (sauté, règle 27).

## Compteurs (règle 28), recalculés
Tables **0** ; migration 2826 (modifiée : motif `interdiction_auteur`, fonction `ent_internat_eleve_majeur`, attendu) ; `MIGRATION_TAG` inchangé ; `Action::TOUTES` inchangé ; routes inchangées (36 pour I3) ; événements inchangés (réutilisés : décision de sortie, absence bloquée, correspondant) ; RLS et registre des données inchangés.

## Tête et CI
Tête de code vérifiée : **695e87ef** (branche `ent-socle-41-code-i3`, PR n° 106 en brouillon), partie de 1e400696. CI `ENT Rust`, run n° 440 : **verte** (formatage, audits, backend, frontend). Le premier passage avait rendu le job « compilation des binaires de test (archives nextest) » rouge par **perte de runner** (étape de compilation restée « en cours » 48 minutes, sans journal, alors que clippy et les autres jobs compilaient le même code) : un seul relancement des jobs en échec, vert. Le commit qui ajoute ce paragraphe ne touche que cette documentation (arbre de code identique à 695e87ef).

---

# Passe 3 — « un acte tombe avec son auteur » : une règle générale, puis les points restants

Même branche (`ent-socle-41-code-i3`, PR n° 106, brouillon), partie de la tête distante **d0d07a0e** (elle contient le commit du pilote sur `remise_possible_tx`, conservé : un parent ou tuteur privé d'autorité parentale est exclu de la porte « contact d'urgence », un proche de lien « autre » ne l'est pas). Migration **2826 modifiée** (non versée) ; 2824 et 2825 intactes. Niveau : **implémenté, testé en local** ; ni validé, ni prêt pilote. Frontend non touché (règle 27) ; `ent-types` et `ent-api` non touchés. Tests : PostgreSQL 16 local, base **neuve** (`ent_i3f`), `ENT_EXIGER_BASE=1`, `ENT_ENV=test`, Rust 1.99.0, `--locked` ; aucun test ne dépend de l'heure réelle (jours fixes d'avril 2026, instants passés à chaque commande, dates de pose des interdictions fixées).

## La règle générale (décision du pilote)

Deux passes avaient bouché les trous un par un (un champ d'auteur par champ d'auteur). La passe 3 remplace ces contrôles par **une seule fonction**, `actes_qui_tombent_tx` (`ent-rust/crates/ent-db/src/internat_sorties/actes.rs`), appelée **partout** : contrôle de départ d'une sortie, contrôle de départ d'une absence, revue à la minute (sorties, absences, correspondants, départs déjà constatés).

> Pour la sortie ou l'absence d'un **mineur**, au contrôle de départ et à chaque revue : si **une seule** des personnes qui ont posé un acte encore en vigueur sur elle est désormais visée (sur toute sa chaîne `fusionnee_dans`) par une interdiction non levée et non terminée d'objet `remise`, `visite` ou `contact_eleve`, l'acte tombe.

| Acte | Où l'auteur est lu |
|---|---|
| demande d'une sortie, annonce d'une absence | **journal des faits** (`demande`, `annonce`, qualité famille, en vigueur = non suivi d'un `accord_remis_a_zero`) |
| accord de la famille | colonne `accord_titulaire_user_id` (l'accord en vigueur) |
| désignation ou changement de la personne qui récupère | **journal** : dernier fait `demande` / `accord_titulaire` / `reprise` portant une personne (les faits `reprise` : un parent qui désigne la personne d'une sortie créée par le système en est l'auteur, aucune colonne de la ligne ne le dit) |
| désignation du correspondant | colonne `designe_par_user_id` et fait `designation` |
| autorisation annuelle (socle 18) dont l'objet dépend | **tous** les décideurs qui l'ont accordée (`ent_acc_decision`, dernière décision de chacun) |

**Effets, par un seul chemin par objet** : sortie non partie → accord et décision du CPE retirés, personne désignée effacée, autorisation annuelle écartée, retour à « demandée » (suspendue ou bloquée comprises : un blocage de l'autre titulaire reste à trancher, avec un état antérieur « demandée »), fait `accord_remis_a_zero` (motif `interdiction_auteur`, personne effacée tracée), événement ; absence non partie → personne effacée, **bloquée** (motif `interdiction_auteur`), l'élève reste attendu ; correspondant → suspendu (seul le CPE le reprend) ; **déjà parti** → rien n'est défait, un cas pour le CPE (point 1). Une décision du CPE ou de la direction **postérieure à la pose de l'interdiction** couvre l'acte (la revue ne défait pas ce qu'ils ont décidé). La famille ne lit jamais la cause (code générique, F-D) ni le fait `signalement`.

**Tests** (`tests_internat_i3`, préfixe `p3_` et `ea_`) : sortie de week-end créée par le système puis personne désignée par le parent A, A interdit → la personne tombe, à la revue et au départ, avec un **témoin** (parent non visé) qui part normalement : `p3_regle_sortie_week_end_du_systeme_…` ; sortie accompagnée, accord de B et personne de A : `p3_regle_sortie_accompagnee_accord_de_b_personne_de_a` ; autorisation annuelle signée par A seul / par A et B : `p3_regle_autorisation_annuelle_…` ; correspondant désigné par A : `ea_correspondant_…` (même fonction) ; E-A sur une sortie suspendue ou bloquée : `p3_regle_sortie_suspendue_ou_bloquee_…`. **Le contrôle échoue bien sans la règle** : avec la fonction neutralisée (mutation locale, retirée ensuite), les 8 tests de la règle (`p3_regle_*` ×4, `p3_deja_parti_*`, `ea_*` ×3) échouent tous (8 rouges sur 8) ; restaurée, ils passent.

## Les points restants

| Point | État | Ce qui a été fait | Tests |
|---|---|---|---|
| **1** déjà parti | fait | `signaler_departs_tx` : une absence ou une sortie « partie » dont un auteur **ou la personne remise** devient visé ouvre un cas **`interdiction_auteur`** pour le CPE (alerte `internat.cas_ouvert`, fait `signalement` illisible de la famille). `ent_internat_attendu_calcule` : **tant que ce cas est ouvert, les nuits SUIVANTES (nuit du cas exclue) ne retirent plus l'élève** de l'attendu ; le CPE tranche en fermant le cas ; un cas ouvert ou clos depuis la pose de l'interdiction n'est pas rouvert | `p3_deja_parti_…` (sortie de trois jours, absence partie, cas clos non rouvert) |
| **2** verdict annuel | fait | `verdicts_annuels` : un verdict « autorisé » dont **tous** les décideurs sont visés devient `decideur_interdit` (la vue `ent_eleve_lien_communicant` n'exclut que `communication`) ; la lecture d'autorisations (`autorisations_eleve_a`) le montre ; l'inscription au mercredi est refusée. Le verdict reste « autorisé » si un autre décideur subsiste, mais la règle générale fait tomber l'acte (« une seule » personne visée) | `p3_regle_autorisation_annuelle_…` |
| **3** faux « non récupéré » | fait | L'heure limite ne vaut que pour une absence qui **commence le vendredi** (première nuit) ; pour une sortie : l'heure de départ **prévue saisie**, ou l'heure de récupération du vendredi pour la sortie créée par le système (début nominal à midi) ; **le cas se ferme au départ constaté** (issue `rentre_famille`) ; une absence dont la nuit est couverte par une sortie « partie » est ignorée ; un cas ouvert **ou clos** pour la nuit n'est pas rouvert | `p3_non_recupere_…`, `mb_…` adapté (absence du vendredi) |
| **4** interdiction sans personne identifiée | fait | **La direction lève** (le CPE ne tranche pas : code inchangé, documentation alignée ci-dessous). Une sortie ou une absence **accompagnée** reste impossible tant qu'elle est en vigueur : à la revue et au départ elle est **bloquée** (jamais suspendue d'elle-même). **Levée ponctuelle** : la direction tranche « lever » sur ce cas avec un **motif obligatoire** (`motif_de_levee_tx`) ; la décision est tracée (`tranchee_levee`, motif `interdiction_remise`), vaut **pour ce cas seulement** (`remise_pour_tx` lit la décision postérieure à la pose) ; une interdiction nominative ne se lève jamais ainsi ; une nouvelle demande accompagnée reste refusée | `p3_interdiction_sans_personne_…` (dont M-A sur une absence : revue), `c1_*` et `ma_*` adaptés (motif de levée) |
| **5** lien échu | fait | Fonction SQL `ent_internat_remise` : la porte « contact d'urgence » écarte un parent ou tuteur dont le lien est **terminé** (`date_fin` passée) ou **pas encore commencé**, qu'il ait été privé d'autorité ou non. **Décision prudente, à valider** : un lien simplement échu d'un parent qui reste contact validé est écarté (la direction rétablit le lien) ; un proche de lien « autre » reste autorisé | `p3_lien_echu_…` (6 cas) |
| **6** `week_end_seul` | fait | `week_end_complet` : première nuit un vendredi **et** dernière nuit au plus celle du dimanche (annonce, départ, revue) ; la direction ne lève pas une absence sans personne ni autorisation annuelle | `p3_week_end_seul_…` |
| **7** course de la revue | fait | Sorties et absences lues `FOR UPDATE` ; chaque `UPDATE` gardé par l'état attendu (`AND statut = …`, `rows_affected`) : une sortie partie entre-temps (la contrainte `effectif_chk` aurait fait échouer toute la transaction de la minute) est ignorée | `p3_course_…` (deux transactions réelles) |
| **8** outbox | fait | Contrat écrit : `cause = interdiction_*` **ne se relaie jamais aux familles** (`docs/socles/41_INTERNAT.md` §22 ; note pour le socle 8, `docs/socles/08_NOTIFICATIONS.md` §6.2) | — (recette du socle 8) |
| **9** charge | fait | Revue sans requête par ligne : évaluation unique de la règle pour tous les objets, remises, verdicts annuels (une vérification par usage et jour), correspondants, interdictions sans personne, en lots (`unnest`) ; règles de remise et d'interdiction **déplacées en fonctions SQL** (une définition, appelée par ligne au départ et en lot à la revue) ; seules les écritures, rares, sont faites ligne à ligne. **Mesure** (300 internes : 150 sorties de week-end autorisées dont la personne est désignée par un parent, 100 absences annoncées) : **34 requêtes** pour toute la revue (journal du serveur, indépendant du nombre d'élèves), **69 ms** puis 65 ms (base locale) | `p3_charge_revue_a_la_minute_pour_300_internes` (borne large de 10 s, mesure imprimée) |
| **10** tests manquants | fait | désignation de tiers (`p3_regle_sortie_week_end_…`), annuel signé par le parent visé, revue M-A d'une absence, E-A sur sortie suspendue ou bloquée, lien échu | voir ci-dessus |

**Documentation du point 4 (alignement)** : une interdiction **sans personne identifiée** (non nominative) n'est levée que par la **direction** (ou l'administration), jamais par le CPE ; tant qu'elle est en vigueur, un mineur ne part pas seul et personne ne lui est remise (ni sortie ni absence accompagnée), sauf décision explicite et tracée de la direction sur le cas, avec motif obligatoire.

## Choix à valider

1. **Absence : la direction tranche, pas le CPE.** La consigne dit que « la décision revient au CPE » ; pour une sortie c'est exact (retour à « demandée »). Pour une **absence** le blocage existant (direction) est conservé : le CPE est alerté par l'événement, la direction tranche, et lever est impossible sans personne désignée ni autorisation annuelle (la personne a été effacée) : la famille annonce de nouveau.
2. Une sortie **bloquée ou suspendue** dont l'auteur est visé retourne à « demandée » : la levée de la direction ne rend pas l'accord.
3. Les décideurs de l'autorisation annuelle sont **tous** retenus comme auteurs (une décision expirée mais non retirée compte) : prudent.
4. Lien échu d'un parent contact validé : écarté (point 5).
5. Une nuit « en cours » s'arrête à midi (`nuit_a_tx`) : le cas du point 1 ouvert à 9 h porte sur la nuit précédente.

## Tests locaux

`tests_internat_i3` : **63 verts** sur base neuve (52 + 11 nouveaux ; 4 anciens adaptés : `mb_` (absence du vendredi), `ea_absence_` (levée motivée, personne effacée), `c1_` et `ma_` (motif de levée)). Suite large `ent-db` (`internat rls registre migrat fusion conservation rgpd accords d036 roles isolation jour_utc`, base neuve) : voir « Tête et CI ». Clippy 1.99 `-D warnings` (3 commandes de la CI) : voir « Tête et CI ». `cargo fmt -p ent-types --check` : inchangé (`ent-types` non touché ; fmt non lancé sur `ent-db`, règle du dépôt).

## Compteurs (règle 28), recalculés

Tables **0** (aucune) ; **fonctions SQL +5** (`ent_internat_personne_chaine`, `_personne_visee`, `_user_vise`, `_interdiction_sans_cible`, `_remise`) ; migration 2826 modifiée (nature de cas `interdiction_auteur`, acte `signalement`, attendu, fonctions) ; `MIGRATION_TAG` inchangé (aucune migration nouvelle) ; `Action::TOUTES` inchangé ; routes inchangées (36 pour I3) ; événements inchangés (réutilisés : `internat.sortie_decidee`, `internat.absence_bloquee`, `internat.correspondant`, `internat.cas_ouvert`) ; RLS et registre des données inchangés (aucune table) ; constantes publiques d'`ent-types` inchangées.

## Tête et CI (passe 3)

À compléter par le message au pilote (tête finale et état de la CI de la PR en brouillon).
