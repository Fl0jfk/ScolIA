# Socle 24 « Finances » — lots F1 et F2 : livraison

Branche `ent-socle-24-code-f1f2` (partie de `origin/ENT`, fusionnée avec E7-E8 : 47a5be1e). Conception : `docs/socles/24_FINANCES.md` (R-F*, MIG-3, MIG-4, FD-15, §19.1) et `PERSONAS_24_25.md`. **Statut : implémenté et testé en local ; non validé par Florian, non prêt pilote, non prêt production.**

## 1. Livré (F1)

- **Service unique** : `ent-db/src/facturation_service.rs` fixe la liste des fichiers autorisés à écrire en finances (`FICHIERS_DU_SERVICE`) et porte `Acte`, `journaliser_tx`, `payload_neutre`. Les routes historiques `/facturation/*` (`service.rs`, `fac_repo.rs`) et `/finances/*` passent par les mêmes écrivains ; **MIG-3** : les routes historiques sont des adaptateurs (mêmes réponses, mêmes statuts).
- **Preuve de comportement identique** : test d'or `ent-api/tests/or_facturation_historique.rs` (+ `tests/or/facturation_historique.json`), généré sur `origin/ENT` propre, stable sur 3 exécutions, identique sur la branche.
- **Événements** `facturation.*` (nom canonique) avec **alias de lecture** `fac.*` / `fin.*` (26 correspondances, `ent_events::ALIAS_FACTURATION`) : un consommateur déclaré sur l'ancien nom reçoit l'événement canonique ; plus aucun ancien nom n'est émis. Aucune charge d'événement ne porte montant, IBAN, RUM ni nom (`payload_neutre`, R-F25).
- **Journal financier en ajout seul** (migration 521, `ent_fin_journal`) : chaque écriture est journalisée dans la même transaction (acteur, casquette, chemin, action, objet, compte, élève, montant, motif, détail). Visible par la direction (tout) et par chacun pour ses propres gestes (M-676).
- **Garde R-F1** : `ent-db/tests/garde_finances_ecritures.rs` (aucune écriture SQL sur les tables financières hors des fichiers du service ; outils génériques listés nominativement) ; `garde_inscriptions_ecritures.rs` : les deux exceptions financières supprimées (le mandat signé à l'inscription passe par `facturation_mandats::creer_mandat_signe_tx`).
- **Droits fins (§19.1)** : 16 actions `facturation.*` (guichet, emettre, avoirs, echeanciers, reductions, tarifs, voter, simuler, parametrer, engagements, cloturer, exporter, pilotage, journal, reprise, publier) ; `Action::TOUTES` = **269**. Modèles par défaut `socle_24_finances` (une ligne d'appel, règle 21). **Changement de comportement assumé (R-F23)** : l'Administratif n'a plus `facturation.lire` (plus de liste de tous les comptes) ; il a le **guichet** (un compte à la fois, preuve d'identité tracée). Les tests F0/séparation ont été alignés.
- **Vue guichet** (R-F37, CF-95) : recherche de payeur (3 caractères mini, 10 résultats), solde / échéances / documents d'un seul compte, consultation journalisée avec la preuve d'identité ; l'accueil n'a pas les documents.

## 2. Livré (F2)

- **Comptes débiteurs** `ent_fin_compte` (migration 522) : personne, organisme ou foyer historique « à vérifier » ; un compte par foyer ; chaque pièce porte `compte_id` (posé par la base, figé ensuite) ; entité de facturation par défaut (R-F33, sans consolidation).
- **MIG-4 (reprise)** : une seule implémentation SQL, rejouable ; rattache les pièces historiques sans modifier aucun montant, numéro ou date ; signale (jamais n'éclate) les foyers à plusieurs payeurs ou sans payeur (CF-09) ; rapport conservé.
- **Port `payeurs`** (adaptateur du socle 4 : `ent_eleve_lien.est_payeur/part_paiement`, MIG-5) + **désignations datées par catégorie** : une catégorie précise l'emporte sur `toutes` (CF-02) ; découpage d'une période par date de désignation (`tranches` / `decouper`, plus fort reste) ; avant la première désignation, les liens restent en vigueur ; un lien payeur à part nulle ne paie rien.
- **Catégorie sans payeur** : non facturée, tâche « payeur manquant » idempotente, jamais « au responsable 1 » (R-F4).
- **Changement de payeur (CF-03)** : conséquences **proposées** (avoir au sortant, ligne à l'entrant ; arrondi en faveur de la famille : crédit au supérieur, débit à l'inférieur), **aucune pièce émise touchée**, décision de la comptabilité → brouillons d'avoir et de facture (émission = geste humain).
- **Acceptation du payeur désigné et pièce de répartition (R-F30, FAC-217/218)** : sans acceptation ni pièce (jugement, convention, accord des deux) la répartition reste *en attente* et l'ancienne reste en vigueur (CF-74) ; un refus ouvre une tâche (CF-75) ; seule la personne désignée peut répondre (sinon « introuvable ») ; contrainte en base.
- **Élève majeur payeur (FD-15, FAC-253/254, CF-84)** : transfert avec acceptation de l'élève ; l'élève voit son compte via sa fiche personne ; les parents n'en voient rien ; un mineur est refusé ; le passé reste aux parents.
- **Organismes payeurs (FAC-256)** : SIRET contrôlé (Luhn) et unique, Chorus Pro réservé aux organismes publics, **prises en charge** avec plafond en centimes (FAC-255, CF-82 : au-delà du plafond, la part non couverte passe au payeur de rang 2, sinon tâche `part_non_couverte`), **alertes à 30 jours** et passage « échue » (FAC-257, CF-83 ; le jour est un paramètre, aucune horloge dans le calcul).
- **Isolation** : toutes les requêtes portent `etablissement_id` (garde statique `isolation_sql_statique`, aucune nouvelle exception) ; toute route d'écriture répond 404/refus sur une cible absente ou d'un autre établissement (matrice de séparation `domaine_finances_comptes`).

## 3. Compteurs (règle 28, après fusion d'`origin/ENT`)

| Compteur | Valeur |
|---|---|
| `Action::TOUTES` | 269 (+16) |
| Tables avec RLS (`tests_rls.rs`) | base ENT + **8** (1 en 521, 7 en 522) |
| Migrations | 521, 522 ; `MIGRATION_TAG` inchangé |
| `SOURCES` / `NOMS_SOURCES` (séparation) | 64 (63 d'ENT + `finances_comptes.rs`) |
| `TABLES_ELEVE` | +5 entrées financières ; `ent_fac_ligne` et `ent_fin_reduction_eleve` passent en « Reste » |
| Registre des données | +8 tables (`registre_donnees.rs`) |
| Conservation (RGPD) | nouvelle catégorie `fin.tache` (24 mois) ; les autres tables F2 en `fin.piece` (10 ans) ; catalogue `2026-11-17-f2` |
| Routes | +21 (`ent-api/src/finances_comptes.rs`) |

**Justification des durées** : pièces, comptes, désignations, prises en charge, journal et conséquences = justificatifs comptables → 10 ans (C. com. L123-22 ; LPF L102 B pour la preuve fiscale). Les tâches de travail (payeur manquant, demande de répartition, alertes) n'ont pas valeur de pièce : 24 mois.

## 4. Écarts et décisions à valider

1. **Personne non unique par compte** : l'index d'unicité sur `(établissement, personne)` a été retiré (un payeur peut l'être dans deux foyers : familles recomposées) ; l'unicité reste par foyer.
2. **Administratif sans `facturation.lire`** (R-F23) : écart de comportement volontaire par rapport à F0 (tests alignés) ; à confirmer par Florian.
3. **Brouillons non recalculés** : un changement de payeur ne recalcule pas les brouillons existants avant F4 ; la ligne entrante est calculée au tarif en vigueur.
4. **Non livré** : gel au décès (FAC-209 / CF-10) ; intégration « à faire » (`afaire`) des tâches ; l'accueil au guichet n'est couvert que par le test de modèle de droits.
5. Aucune dépendance ajoutée ; dates en `chrono` minimal, aucun nouvel utilitaire de date.

## 4 bis. Ce que l'Administratif (secrétariat) peut et ne peut plus faire

| Geste | Avant F1 | Après F1 |
|---|---|---|
| Consulter le solde, les échéances et les documents **d'une famille présente** | oui (liste complète) | **oui, via le guichet** (un compte à la fois, identité tracée) |
| Lister toutes les factures / tous les comptes | oui | **non** (R-F23) |
| Encaisser un règlement | non (F0, FD-27) | non (inchangé : droit `facturation.gerer`/comptabilité) |
| Remettre un reçu / une quittance | non (routes sous `facturation.lire`) | **non — non couvert par le guichet** |
| Désigner les payeurs, créer un organisme, prises en charge (`engagements`) | non | oui |
| Émettre, avoir, paramétrer, exporter, journal, reprise | non | non |

**Lacune explicite** : le guichet est en consultation seule. « Encaisser au comptoir et remettre un reçu » reste à la comptabilité ; si le secrétariat doit le faire, il faut une action dédiée (par exemple `facturation.encaisser_guichet` limitée aux espèces et chèques, avec reçu) — à décider par Florian, à prévoir avant la bascule de février.

## 4 ter. Préalables explicites (écart 3)

**Suites à faire (notées à la demande du pilote)** : (1) liste blanche des clés d'événements **par type d'événement** (aujourd'hui globale, par forme de clé) ; (2) quota du guichet : compter le **jour de l'établissement** (fuseau) au lieu du jour de Paris.

Le **gel au décès (FAC-209 / CF-10) doit passer avant toute relance automatique en production**. Recalcul des brouillons avant F4 ; intégration « à faire » des tâches.

## 4 quater. Correctifs de la revue de sécurité indépendante (PR 97)

Aucun critique, pas de fuite entre établissements. Un test par correctif (`tests_finances_f2.rs`, `separation.rs`, `garde_finances_ecritures.rs`).

| Réf. | Correctif | Test |
|---|---|---|
| E1 | La demande de répartition d'un parent ne fuit plus vers l'autre : la tâche porte le compte du **demandeur** (clé d'idempotence par demandeur), la réponse à la famille est un **accusé neutre** (`AccuseDemandeDto` : id et statut), jamais le commentaire | `e1_demande_de_repartition_ne_fuit_pas_entre_parents` |
| E2 | Une part portée par un compte `organisme` exige une prise en charge **active** couvrant la date et la catégorie ; sinon l'organisme n'est pas facturé, tâche `part_non_couverte`, rien sur la famille (échue / annulée / hors période = blocage) | `e2_m1_organisme_sans_prise_en_charge_active_n_est_pas_facture` |
| E3 | Guichet : quota quotidien par agent (30 par défaut, paramétrable 1–1000 par `POST /finances/guichet/quota`, droit `parametrer`), recherches et ouvertures confondues ; au-delà : refus, journal `guichet.quota_depasse`, tâche d'alerte `guichet_quota` pour la direction ; recherche par **nom et prénom complets**, correspondance exacte (sans `ILIKE`), 5 résultats au plus ; `GET …/payeurs` reste fermé au rôle guichet seul (aucun `compte_id` hors désignation ou ouverture journalisée) | `e3_quota_du_guichet_et_alerte_a_la_direction`, `f1_vue_guichet_du_secretariat` |
| E4 | Garde R-F1 : retire seulement les blocs `#[cfg(test)]` (accolades équilibrées), plus tout ce qui suit le premier ; test de la garde elle-même | `la_garde_analyse_le_code_apres_un_bloc_de_tests` |
| M1 | `POST …/prises-en-charge/alertes` ignore tout jour fourni (jour de l'établissement) ; réactivation journalisée d'une prise en charge échue (`…/{id}/reactiver`, motif obligatoire, fin future) | `e2_m1_…` |
| M2 | Plafond calculé **prise en charge par prise en charge** (catégories et période propres), jamais additionné ; verrou du foyer avant la lecture du restant ; `CHECK plafond_centimes ≤ 1 000 000 000` ; arithmétique contrôlée | `m2_plafond_par_prise_en_charge_jamais_additionne` |
| M3 | Reste **signé** pour les lignes de réduction ; verrou du foyer avant `lignes_existantes` | `cf_03_…` (régression) |
| M4 | « jugement » / « convention » exigent un document existant dans l'établissement ; « accord des deux » n'ouvre jamais la répartition seul (les deux acceptations restent exigées) ; un compte désigné est lié à l'élève, ou organisme avec prise en charge ; pas de règle à deux personnes (noté pour Florian) | `m4_pieces_et_comptes_designes_controles` |
| M5 | L'élève n'accède au compte et n'accepte que s'il est majeur **aujourd'hui** et la date d'effet atteinte | `m5_eleve_mineur_n_accede_ni_n_accepte`, `fd_15_…` |
| M6 | Plus de dérogation par `current_setting('ent.purge_etablissement')` : fonction `ent_fin_purger_etablissement` (SECURITY DEFINER, exécution retirée à PUBLIC) ; `BEFORE TRUNCATE` sur les tables F2 | `m6_journal_et_historique_sans_derogation_de_session` |
| M7 | **Liste blanche** de clés dans les charges d'événements financiers (`cle_autorisee`) en plus de la liste noire : plus de `deja_plus_ce`, `reste`, IBAN masqué | `payload_neutre_retire_…`, `f1_journal_evenements_…` |
| M8 | Dossier : dès qu'une répartition à plusieurs payeurs change, une tâche `repartition_incomplete` au lieu d'une répartition rendue effective | `m8_dossier_changement_entre_plusieurs_payeurs_ouvre_une_tache` |
| M9 | Reprise : le reste de la division va au premier payeur ; élèves aux parts invalides comptés « à vérifier » (`eleves_a_verifier`) ; adaptateur des liens renormalisé à 100 % | `m9_reprise_reste_au_premier_et_eleves_a_verifier` |
| M10 | Séparation : de **vrais** compte, groupe et tâche de B, corps valides, refus exact 401/403/404 ; test inter-établissements réel (second jeu de labo) sans effet en base | `domaine_finances_comptes`, `m10_acces_inter_etablissements_refuse_sans_effet` |
| F1 | Documents du guichet : liste blanche (Administratif ou `facturation.lire`) | — |
| F2 | Référence d'une prise en charge masquée pour le secrétariat | `e2_m1_…` |
| F3 | `mes-comptes` : DTO réduit (`MonCompteDto`) | `fd_15_…` |
| F4 | Plus de relecture sans filtre après `ON CONFLICT DO NOTHING` (accusé filtré sur le demandeur) | `e1_…` |
| F5 | SIRET obligatoire pour Chorus Pro | `organisme_payeur_siret_controle_et_unique` |
| F7 | Bornes (8 payeurs, 8 catégories dédoublonnées) | `f7_repartition_bornee_a_huit_payeurs` |
| F8 | `.export()` sur journal et tâche (registre) ; purge et déclencheur cohérents | `registre` |
| F6 | **À noter pour plus tard** (non traité dans ce lot) | — |

Migration 522 modifiée dans le même lot (jamais fusionnée) : nature de tâche `guichet_quota`, colonne `ent_fin_parametres.guichet_quota_jour`, plafond ≤ 10 M€, `eleves_a_verifier`, reste de la reprise, déclencheurs sans dérogation, `ent_fin_purger_etablissement`.

## 5. Vérifications (locales)

Formatage (2 commandes du job « Formatage »), clippy 1.99.0 sans avertissement (workspace, `--features ent-api/labo,ent-lab/labo`, `-p ent-types`), tests : `tests_finances`, `tests_finances_f2` (23 scénarios : CF-01, 02, 03, 16, 17, 74, 75, 82, 83, 84, FD-15, MIG-4/CF-09, journal, alias, droits fins, guichet, isolation), RLS, conservation, extrait de la personne, registre, rôles, fusion, `isolation_sql_statique`, gardes R-F1, `separation`, `recette_securite_3`, golden historique, `recette_finances_f0`.
