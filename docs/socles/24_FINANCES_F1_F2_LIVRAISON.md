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

## 5. Vérifications (locales)

Formatage (2 commandes du job « Formatage »), clippy 1.99.0 sans avertissement (workspace, `--features ent-api/labo,ent-lab/labo`, `-p ent-types`), tests : `tests_finances`, `tests_finances_f2` (23 scénarios : CF-01, 02, 03, 16, 17, 74, 75, 82, 83, 84, FD-15, MIG-4/CF-09, journal, alias, droits fins, guichet, isolation), RLS, conservation, extrait de la personne, registre, rôles, fusion, `isolation_sql_statique`, gardes R-F1, `separation`, `recette_securite_3`, golden historique, `recette_finances_f0`.
