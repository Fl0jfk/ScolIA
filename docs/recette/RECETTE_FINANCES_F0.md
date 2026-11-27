# Finances — lot F0 (correctifs urgents de l'existant) : compte rendu de réalisation

**Date :** 2026-11-10 — **Branche :** `ent-socle-finances-f0` (base `origin/ENT`) — **Migration :** `520_finances_f0_immuabilite`.
**Référence :** conception `docs/socles/24_FINANCES.md` §2 et §25 (branche `ent-mc0hoe`), `25_PAIEMENTS.md`, `PERSONAS_24_25.md`.
**État (vocabulaire de `CLAUDE.md`) :** **implémenté et testé** (tests listés ci-dessous, locaux puis CI). **Non validé** par Florian, **non prêt production** : aucun accès à la production, aucune donnée réelle, aucune migration appliquée ailleurs que sur la base de test locale.

## Ce qui était déjà sur `ENT` (vérifié, non refait)
- **RS3-01** (`foyer_ids_parent` filtre `est_payeur`) : `rs3_01_…` vert.
- **RS3-02** (`executer_sepa_atomique`, migration 376) : `rs3_02_…` vert (adapté : la facture de labo a des échéances, le test crée désormais sa propre facture sans échéance, cf. FD-04).
- **RS3-04** (migration 377) : `rs3_04_…` vert.
- **RS3-03** : soldée par ce lot (voir FD-03).

## Correctifs de ce lot, un test chacun (fichier `ent-api/tests/recette_finances_f0.rs` sauf mention)
| Défaut | Correctif | Test |
|---|---|---|
| **FD-04** double chemin, double prélèvement | `/facturation/…/sepa/prelever` refusé sur une facture à échéances ; export historique limité aux prélèvements sans échéance, pré-notifiés, sur mandat actif ; la préparation par échéance écarte une facture qui a un prélèvement historique ouvert ; rappels historiques limités aux pièces historiques (`origine` NULL) ; **l'encaissement historique passe par un chemin unique** (encaissement + imputation/lettrage + quittance, avoirs comptés) | `f0_fd04_prelevement_historique_refuse_sur_facture_a_echeances`, `f0_fd04_export_historique_limite_a_ses_prelevements` |
| **FD-05** numéro fourni par le client | numéro provisoire `BRO-…` au brouillon ; numéro définitif `F<année>-<6 chiffres>` attribué **à l'émission**, série continue partagée avec `/finances` (même compteur, transaction annulée = pas de trou) ; champ client ignoré | `f0_fd05_numero_attribue_par_le_serveur` ; domaine `f0_numero_du_client_ignore` |
| **FD-06** solde avec prélèvement ouvert | le solde se décide sur l'encaissé réel + avoirs ; le dépassement sur l'engagé (encaissé + prélèvements ouverts) | `f0_fd06_prelevement_ouvert_ne_solde_pas_la_facture` ; domaine `f0_prelevement_ouvert_ne_solde_pas` |
| **FD-07** encaissements concurrents | verrou du foyer (même clé que `/finances`) puis `FOR UPDATE` de la facture ; encaissement, lettrage, solde et quittance dans **une** transaction ; rang de quittance sous verrou | `f0_fd07_encaissements_concurrents_sans_depassement` |
| **FD-08 / FD-09** cascade, immutabilité | `ON DELETE CASCADE` → `RESTRICT` (migration générique sur les 6 tables) ; déclencheurs : facture émise/soldée immuable hors `statut`/`soldee_at`, lignes figées, échéances/encaissements/quittances figés, aucune suppression des pièces et preuves (mandats, prélèvements, rappels, pré-notifications, rejets, règlements, lettrages, relances, exports) | `f0_fd08_fd09_piece_emise_immuable_et_non_supprimable_en_base` |
| **FD-10** mandat révoqué | export exclut les mandats non actifs ; la révocation annule les prélèvements « prévus » (compte des déjà exportés dans l'événement) ; la base refuse un prélèvement sur mandat non actif ; migration : annulation des « prévus » existants | `f0_fd10_mandat_revoque_exclu` |
| **FD-03 / RS3-03** export SEPA | `POST` (GET → 405) ; créancier lu **côté serveur** ; champ `cdtr_*` refusé (`deny_unknown_fields`) ; l'UI actuelle appelle le POST ; pré-notification exigée (service **et** déclencheur en base : `prevu → exporte` refusé sans `prenotification_id`) ; la planification historique remet la pré-notification (délai de l'établissement, payeur joignable) ; fichier construit **avant** de marquer exporté | `f0_fd03_export_sepa_en_post_sans_iban_dans_l_url`, `f0_r_p6_pas_d_export_sans_prenotification` |
| **FD-11** compte créancier | IBAN chiffré au repos (colonne `id` ajoutée, mêmes primitives que les mandats) ; changement du créancier réservé à Admin/Direction + événement `fin.crediteur_modifie` (IBAN masqué) + notification à la direction | `f0_fd11_compte_crediteur_n3_chiffre_et_alerte` ; `ent-db` `sepa_rejets_reglements_relances_export` (refus Compta) |
| **FD-13 / R-P16** frais | `frais_rejet_centimes` et `frais_centimes` des paliers refusés (> 0) et forcés à 0 par contrainte ; `facturer_frais` désactivée (point d'entrée unique conservé) | `f0_fd13_aucun_frais_de_relance_ni_de_rejet` ; `ent-db` `sepa_rejets_…` (plus de facture de frais) |
| **FD-14** payeur sans canal | aucune préparation de prélèvement pour un foyer sans compte payeur joignable (écarté, motif explicite) ; idem à la planification historique | `ent-db` `sepa_rejets_…` ; `f0_fd10_…` (foyer avec payeur) ; garde de `verifier_prenotifiable` |
| **FD-16** quantités / montants | consommation : montant 0 à 1 M€ (plus de négatif) ; facture historique : quantité 1 à 100 000, montant borné, total ≥ 0 ; contrainte `NOT VALID` en base | `f0_fd16_consommation_negative_refusee` ; domaine `f0_quantites_et_montants_bornes` |
| **FD-12** bourse (règle provisoire) | déduite seulement des lignes restauration/internat du foyer, à hauteur de ces lignes (excédent non imputé) | `ent-db` `prorata_fratrie_quotient_bourse` (externe : aucune ligne ; demi-pensionnaires : 1 200 € entier, 4 000 € plafonné à 3 000 €) |
| **FD-23** pagination mandats | filtre des mandats d'un parent **en SQL** (plus de 80 derniers filtrés en mémoire) | `f0_fd23_parent_voit_son_mandat_malgre_les_autres` (100 mandats plus récents) |
| **FD-24** fuseau | `aujourd_hui()` en heure de Paris (règle UE de l'heure d'été) ; période de l'export OGEC lue en heure de Paris | `ent-db` `f0_date_comptable_en_heure_de_paris` |
| **FD-26** | contrôle par **droit** (`facturation.lire`) et non `role == "parent"` ; reste à payer de la liste famille compte les avoirs | `f0_matrice_routes_financieres_refusees_aux_non_autorises` |
| **FD-27** droits du secrétariat | Administratif : `facturation.lire` seul ; `facturation.gerer` et `bourses.gerer` retirés par défaut (Admin, Direction, Compta les gardent ; attribuables par rôle, D-017) | `f0_fd27_secretariat_lecture_seule_par_defaut` ; `ent-auth` matrice historique mise à jour ; `ent-db` `r2_non_regression_…` |
| **R-F36 / R-P30** journal minimal | `tracer_acces` sur les lectures nominatives du personnel (liste et détail de facture, mandats) | `f0_r_f36_consultation_nominative_journalisee` |
| **RS3-08 / R-P5** RUM, IBAN | RUM générée par le serveur (`SCOLIA…`, ≤ 35 car.) ; clé IBAN (modulo 97) contrôlée | `f0_rum_serveur_et_iban_modulo_97` ; domaine `f0_iban_cle_modulo_97`, `f0_rum_serveur` |
| **pain.008** adresses | vérifié : aucune adresse (ni `AdrLine` ni `PstlAdr`) n'est émise | domaine : assertion ajoutée au test d'export |
| **CF-97** | vérifié par lecture : les avis financiers partent aux **comptes** des payeurs (`ent_user.email`), jamais à une coordonnée de foyer | — (lecture de code) |
| Matrice de séparation | parent, élève, professeur refusés sur export/émission/encaissement ; direction de B ne lit pas une pièce de A | `f0_matrice_routes_financieres_refusees_aux_non_autorises` |

## Hors périmètre / non fait / à décider (honnêtement)
- **Confirmation renforcée (second validateur, délai 24 h) du changement de créancier** (25 R-P2) : renvoyée à P3 par la conception ; F0 pose le niveau direction, l'événement et l'alerte.
- **IBAN existants en clair** dans `ent_fin_parametres` : lus tels quels (compatibilité) et **chiffrés à la prochaine écriture** ; pas de script de reprise (aucune donnée modifiée par la migration, règle MIG-P1). Utiliser l'outil de rechiffrement existant après décision.
- **Prélèvements déjà exportés** d'un mandat révoqué : non annulés (déjà dans un fichier bancaire) ; comptés dans l'événement pour traitement humain.
- **Avoirs dans la liste famille** : toujours listés comme pièces (affichage dédié = lot F7) ; seul le reste à payer en tient compte.
- **FD-12** : règle provisoire ; la règle complète (payeur bénéficiaire, reversement de l'excédent) relève du socle 26.
- **FD-16** : sur le chemin historique, une ligne négative reste permise (déduction de bourse existante) tant que le total n'est pas négatif ; l'interdiction stricte suppose des articles typés (chemin `/finances`).
- **Admin** conserve `bourses.gerer` (la conception ne parle que de l'Administratif) : à confirmer.
- **Détecteur de trous de numérotation nocturne** (R-F3) : non fait (la série est continue par construction, le contrôle relève de F5).
- **`ui.rs`** : seul l'appel d'export SEPA a été adapté (POST, plus de champs créancier) ; aucune extension de l'UI JavaScript.
- Les tests d'`ent-lab` (scénarios) ont été adaptés : RUM serveur, date de prélèvement au-delà du délai, créancier posé par la direction.

## Points pour Florian
1. **Q-F21** (frais de rejet / recouvrement) : désactivés tant que non tranchés par un juriste.
2. **Secrétariat** : confirmer que l'Administratif perd `facturation.gerer` (créer, émettre, encaisser) dès F0, avant la vue guichet de F1 ; un établissement qui s'appuie aujourd'hui sur le secrétariat pour encaisser doit lui attribuer un rôle (D-017).
3. **Migration 520** : à appliquer en pré-production avec une **sauvegarde** ; aucune donnée monétaire modifiée, mais elle annule les prélèvements « prévus » de mandats révoqués (liste à rapprocher avec la comptabilité avant application).
4. Reprise des **IBAN créanciers** déjà saisis (chiffrement).
