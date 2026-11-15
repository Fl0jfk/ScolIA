# Socle N1 — lot N1-2 : prise en charge, délégation, échanges et pièces (livraison)

Conception suivie : `docs/socles/N1_DEMANDES.md` §11, §12, §37, §38 (D-061) — sur la branche `ent-mc0hoe`.
Code : `ent-db/src/demandes_prise.rs`, `demandes_echanges.rs`, `demandes.rs` (garde d'accès), `demandes_routage.rs`
(`router_demande_existante`). Migrations **3044** (messages, pièces, abonnés, unicités « vue » et « retard ») et **3045**
(consultations, partages, liens, personnes visées, garde de requalification). Tests : `tests_demandes_n1_2.rs`.

## Implémenté et testé (labo local)

| Sujet | Où | Test |
|---|---|---|
| Garde d'accès unique `charger_pour_acces` (demandeur, déposant, membre non visé, abonné, consulté ; 404 neutre sinon) | `demandes.rs` | tous |
| **NJ-81** : une demande d'une autre file ne se détourne pas (prendre, réaffecter, déléguer, transférer, requalifier, consulter, écrire, abonner, désigner, rouvrir… : 404 pour un responsable d'une autre file ; « interdit » pour un membre sans la main) | `demandes_prise.rs` | `t_nj81_detournement_autre_file`, `t_nj81_transfert_et_prise_dans_la_file` |
| **NJ-83** : pièces cloisonnées (référence au socle 7 seulement, rôle, visibilité ; jamais de nom de fichier ; jeton émis après le droit de N1 ; document d'autrui non rattachable) | `demandes_echanges.rs` | `t_nj83_pieces_cloisonnees`, `t_tables_n1_2_sans_json_ni_nom_de_fichier` |
| **NJ-80** étendu aux échanges, notes, corrections, masquages, pièces, abonnés | `demandes_echanges.rs` | `t_nj80_echanges_sans_droit_refuse`, `t_separation_etablissements_n1_2` |
| Trois preuves du demandeur (envoyée, vue, prise), quatre états simples, retard toujours montré, responsable prévenu | `preuves_demandeur_for_actor`, `signaler_retards` | `t_trois_preuves_visibles`, `t_retard_montre_et_responsable_prevenu` |
| Réaffectation en un geste d'un agent absent (socle 15), jamais silencieuse | `reaffecter_absent_for_actor` | `t_reaffectation_un_geste_absent` |
| Délégation ponctuelle ; reprise « pour » un délégant (délégation datée du socle 6) | `deleguer_for_actor`, `reprendre_pour_delegant_for_actor` | `t_delegation_ponctuelle_et_reprise_pour` |
| Consultation d'un collègue (partage choisi, jamais C2+) | `consulter_for_actor` et suite | `t_consultation_collegue` |
| Désignation d'une personne visée (DEM-360, R-DM47) | `designer_personne_visee_for_actor` | `t_designation_personne_visee` |
| Requalification (CDM-01, CDM-11) avec scellement des textes déjà écrits | `requalifier_for_actor` | `t_requalification_cdm01_cdm11` |
| « Résolu ? Oui / Non » (Non rouvre ; hors délai : nouvelle demande conseillée ; aucune note d'agent) | `repondre_resolu_for_actor` | `t_resolu_oui_non` |
| Complément demandé / reçu (suspension et reprise du décompte) | `demander_complement_for_actor` | `t_complement_suspend_et_reprise` |
| Abonnés : co-demandeur (même élève, lien actif, jamais pour un élève), copie interne, appui | `ajouter_abonne_for_actor`, `appuyer_for_actor` | `t_abonnes_co_demandeur_copie_appui` |
| Port du socle 16 (conversion message ↔ demande) sans modifier le 16 | `PortMessagerie`, `rattacher_conversation_for_actor` | `t_port_messagerie_conversation` |
| « Vues mais non prises » (DEM-399) | `vues_non_prises_for_actor` | `t_vues_non_prises` |

## Limites connues (à ne pas présenter comme faites)

- **Aucune route HTTP, aucun écran** (N1-4) ; aucune inscription à l'inventaire de séparation des routes.
- **Socle 7 absent** : les pièces passent par le port `PortDocuments` (adaptateur « source absente » : tout refusé). Le jeton court réel,
  l'antivirus et l'émission de pièces produites (DEM-186/187) viennent avec le socle 7.
- **Socle 16** : seul le port est défini ; le dépôt « canal conversion_message » (DEM-318) et la clôture « transmise_conversation » (DEM-192) restent à faire.
- **Socle 8** : les avis sont des événements à enveloppe neutre ; les destinataires, résumés et plages calmes ne sont pas calculés ici.
- **Non fait dans N1-2** : escalade par paliers et relances au demandeur (J+5, J+15), fusion de doublons (DEM-159), signalement de prise sans geste (DEM-194),
  report d'échéance (DEM-231), tableau des retards agrégé (DEM-181/230), restriction de dépôt (DEM-362, migration 3049), « service seul » par agent (DEM-398 : réglé par file seulement),
  tâches du socle 29, purge des échanges par l'ordonnanceur (les messages et pièces sont conservés tels quels), signalement d'absence à la file au début d'une absence (§11.4 : seule la lecture est faite).
- Délai de réouverture : constante de 30 jours (SN1-05) tant que le réglage ADM n'existe pas.
- La consultation ne vise qu'un **agent** (la colonne « file consultée » existe, sans code).
- Une demande requalifiée en C2+ garde son objet (titre) en clair ; seuls les textes sont scellés.
- Le test d'unicité du fait « vue » est vérifié ; l'âge d'une « vue » (≥ 1 jour) n'est pas testé (les faits sont immuables, aucun moyen honnête de les antidater).
