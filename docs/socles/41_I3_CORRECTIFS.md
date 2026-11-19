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
