# Registre de classement des tables (D-037, E12) et neutralisation à la restauration (E9)

Source : `ent-rust/crates/ent-db/src/registre_donnees.rs` (Rust, versionné avec le code ; aucune donnée métier, que du
catalogue). Choix : Rust plutôt qu'une table SQL de référence — le registre doit être lisible **sans base** (revue,
extracteur, CI), il évolue avec les migrations et un test le confronte au schéma réel. Pas de JSON-mémoire (D-036).

Statut : **implémenté et testé en local** ; D-037 est *proposée* (branche `ent-mc0hoe`), appliquée par défaut car sans
coût et réversible (D-031). Rétentions : **à définir** pour toutes les tables sauf les états éphémères.

## Ce que le registre classe (une ligne par table, 345 au 2026-11-10, dont les 9 du socle 42 ; **+5 le 2026-11-11** : messagerie M1 et M2, `ent_msg_fil_eleve`, `ent_msg_participant`, `ent_msg_reprise_rapport`, `ent_msg_reglage_version`, `ent_msg_version`, toutes par établissement avec RLS ; les deux tables qui portent `eleve_id` sont déclarées dans `TABLES_ELEVE`)
portée (`Etablissement` / `ReferentielGlobal` / `CompteMultiEtablissement` / `Technique`), présence de `etablissement_id`,
données personnelles, santé, fichiers (`Aucun` / `Reference` / `Objet` / `ObjetAvecResidu` / `Bytea`), export de portabilité, neutralisation
à la restauration, rétention, **exceptions déclarées** (avec raison).

## Test (`registre_donnees::tests`, tourne en CI avec et sans labo)
Échoue si : une table de la base n'est pas classée ; une entrée n'a plus de table ; `colonne_etablissement` ne dit pas
la vérité ; une table « par établissement » n'a pas `etablissement_id NOT NULL` ou pas de RLS **sans exception
déclarée** ; une table n'a aucun horodatage sans exception ; une colonne `BYTEA` ou `storage_key` n'est pas déclarée ;
une table dont le nom évoque une file (`outbox`, `envoi`, `invitation`, `verification`) n'est pas déclarée « à neutraliser » ;
**et aussi si une exception devient inutile** (table corrigée) : la dette ne peut ni être masquée ni rester périmée.

**Ajouter une table** : ajouter sa ligne dans `REGISTRE` (ordre alphabétique) — le test l'exige. Toute nouvelle file
d'effets de bord (courriel, SMS, push, jeton, rappel) doit porter une `Neutralisation`.

## Dette déclarée (mesurée sur la base migrée, 2026-11-10)
- **160 tables « par établissement » sans RLS** (`RAISON_RLS` : couverture progressive, `RLS_ISOLATION.md`, lot C5).
- **25 tables sans colonne d'horodatage** (E6) : lignes filles datées par leur parent, compteurs, liaisons, référentiels
  statiques ; **à corriger** : `ent_inscription_responsable`. (L'audit annonçait 30 avec un critère plus strict, `created_at`/`*_le` ;
  le registre retient « aucune colonne `timestamp` ».)
- **2 tables « par établissement » à `etablissement_id` nullable** : `ent_fonction` (fonctions communes), `ent_eleve_orphelin`
  (aussi sans RLS).
- **11 tables sans `etablissement_id`** (classées, non masquées) : `ent_user`, `ent_oidc_identity`, `ent_profil_personnel`
  (compte multi-établissement) ; `ent_absence_motif`, `ent_systeme_identifiant`, `ref_calendrier_officiel` (référentiel global) ;
  `ent_etablissement`, `ent_coord_reprise_marque` (technique) ; `ent_login_guard`, `ent_rate_limit`, `ent_password_reset` (état éphémère).
- Fichiers **hors base, colonne `BYTEA` résiduelle non écrite** (`ObjetAvecResidu`) : `ent_msg_piece`, `ent_pai_piece`
  (E5). Depuis la migration 601, plus aucun chemin applicatif n'écrit `contenu` ; il ne garde d'octets que pour
  l'historique non repris (`ent-api reprise-pieces`, voir `docs/PRA_STOCKAGE_OBJETS.md` §5). L'exception `Bytea` est
  levée ; la colonne disparaît à l'étape 4 (table alors classée `Objet`). Aucune table n'est plus déclarée `Bytea`.

## Neutralisation des files à la restauration (E9, F-014)
`neutraliser_files_restauration(conn, PerimetreNeutralisation::{Tout | Etablissement(id)})` : à appeler dans une
transaction **avant** de démarrer distributeur, passerelles et planificateurs. Idempotente, ne supprime pas les envois
(statut explicite, rejouables par un humain) sauf les états éphémères. Rapport d'une ligne par table classée.

| Table | Action |
|---|---|
| `ent_outbox` | `a_distribuer` → `echec` (`derniere_erreur = neutralise_restauration`) |
| `ent_notification_envoi` | `en_attente`/`envoi` → `ignore` (`motif = neutralise_restauration`) |
| `ent_inscription_envoi` | `a_envoyer` → `annule` |
| `ent_inscription_invitation` | `a_envoyer`/`non_envoyee`/`envoyee` → `annulee` (jeton invalidé : **ré-inviter**) |
| `ent_agent_invitation`, `ent_eleve_invitation`, `ent_coordonnee_verification` | jetons / codes non utilisés annulés |
| `ent_creneau_rdv` | `rappel_envoye_le` posé pour les RDV réservés non rappelés (sinon rappel renvoyé) |
| `ent_session`, `ent_passkey_challenge`, `ent_oidc_state`, `ent_login_guard`, `ent_rate_limit`, `ent_password_reset`, `ent_recuperation_passkey`, `ent_email_change`, `ent_preinscription_verification` | purge (état éphémère) |
| `ent_fac_sepa_prelevement` (`prevu`/`exporte`), `ent_fin_prenotification` (échéance future) | **non modifiés, comptés** dans le rapport (`a_verifier`) : revue humaine avant tout export bancaire (double prélèvement) |
| `ent_mail_outbox`, `ent_sms_outbox`, `ent_push_outbox`, `ent_fac_rappel`, `ent_fin_relance`, `ent_event_traitement`, `ent_metier_event`, `ent_outbox_agregat`, `ent_campagne` | classées « effet de bord », **sans état à suspendre** (journaux d'envois déjà faits ; la raison est dans le rapport) |

**À noter** : les pré-notifications SEPA n'ont pas de file d'envoi (`envoyee_le` est posée à la génération) ; aucun état
ne peut donc être « suspendu » — d'où le comptage pour revue. Si un envoi automatique de pré-notification est ajouté un
jour, il devra déclarer sa file ici. La procédure de restauration qui appellera cette fonction reste à écrire (PRA §10).

## À signaler au pilote
Les branches `ent-stockage-objets-s3` (table `500_objet_registre`), `ent-socle-messagerie-m0b` (481) et
`ent-d036-normalisation-json` (561-563) ajoutent des tables : après fusion avec ce registre, le test échouera tant
qu'elles ne sont pas classées (c'est voulu, E12). Une ligne par table à ajouter.
