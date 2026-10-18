//! Persistance Postgres locale pour l'ENT Rust.
//!
//! **Sécurité :** `PgEnt::connect` refuse toute URL hors loopback.
//! **Anti-doublon :** pas de table miroir de `ent_eleve` (effectif = query datée).

mod absence_repo;
mod annonce_repo;
mod appel_repo;
mod auth_repo;
mod carnet_repo;
mod cahier_texte_repo;
mod continuum_repo;
mod copilote;
mod edt_repo;
mod document_repo;
mod eleve_compte_repo;
mod eleve_repo;
mod event_store;
mod foyer_repo;
mod infirmerie_repo;
mod psy_repo;
mod matiere_repo;
mod migrate;
mod motif_repo;
mod fac_repo;
mod mail_repo;
mod sms_repo;
mod push_repo;
mod bank_repo;
mod msg_repo;
mod note_repo;
mod option_repo;
mod periode_repo;
mod pg_err;
mod repas_repo;
mod sanction_repo;
mod sortie_repo;
mod internat_repo;
mod dispense_vs_repo;
mod journal_repo;
mod alerte_cpe_repo;
mod doc_signer_repo;
mod conseil_classe_repo;
mod info_comp_repo;
mod vacances_repo;
mod menu_cantine_repo;
mod quotient_repo;
mod retard_repo;
mod competence_repo;
mod passkey_repo;
mod oidc_repo;
mod pra;
mod tva_repo;
mod param_etab_repo;
mod salle_repo;
mod filiere_repo;
mod inscription_repo;
mod iban_histo_repo;
mod tarif_conso_repo;
mod absence_rh_repo;
mod edition_vs_repo;
mod stage_repo;
mod edt_contrainte_repo;
mod seed;
mod service;
mod signal_repo;
pub mod url_guard;

pub use appel_repo::{Appel, AppelLigne, AppelStatut, LigneEtat};
pub use auth_repo::{AuthUser, LoginIdentity, SessionInfo, StaffMember};
pub use edt_repo::{jour_semaine_iso, EdtCreneau};
pub use domain_eleve::{EdtException, EdtExceptionKind};
pub use domain_eleve::{Annonce, AudienceAnnonce, CreerAnnonce};
pub use domain_eleve::{DeclarerRetard, Retard};
pub use domain_eleve::{
    CompetenceItem, CreerCompetenceItem, EvaluerCompetence, EvaluationCompetence, NiveauMaitrise,
};
pub use annonce_repo::PgAnnonceRepo;
pub use retard_repo::PgRetardRepo;
pub use competence_repo::{EvaluationLue, PgCompetenceRepo};
pub use passkey_repo::{ChallengeRow, PasskeyRow, PgPasskeyRepo};
pub use oidc_repo::{
    OidcProviderPublic, OidcProviderRow, OidcProviderUpsert, OidcStateInsert, OidcStateRow,
    PgOidcRepo,
};
pub use eleve_compte_repo::EleveCompte;
pub use copilote::{
    analyser, format_eur, Intention, ReponseCopilote, CAPACITES, COPILOTE_JOURS_DEFAUT,
};
pub use foyer_repo::{CreateLien, EleveLien, LienDossierFoyer, UpdateLien};
pub use mail_repo::MailOutboxRow;
pub use iban_histo_repo::IbanHisto;
pub use sms_repo::SmsOutboxRow;
pub use push_repo::PushDevice;
pub use service::EnfantFamille;
pub use matiere_repo::{CreateMatiere, Matiere};
pub use migrate::{apply_socle_migrations, MigrateError, MIGRATION_TAG};
pub use pra::{
    EcartInventaire, Inventaire, InventaireTable, RapportIntegrite, INVENTAIRE_VERSION,
    TABLES_VOLATILES,
};
pub use motif_repo::AbsenceMotif;
pub use note_repo::{Note, SaisirNote};
pub use option_repo::PgOptionRepo;
pub use seed::{LaboCast, LaboEleve, LaboPersona, LABO_PASSWORD};
pub use service::{
    BankRapprochementSummary, EntDbError, LoginOutcome, OidcProviderParams, PgEnt,
    PersonnelProvisionne, ProvisionnerPersonnel, SondeMetrique,
};
pub use ent_oidc::{OidcError, OidcTransport};
pub use domain_eleve::SignalAbsence;
pub use url_guard::{assert_local_postgres_url, UrlGuardError};

#[cfg(test)]
mod scenarios;

#[cfg(test)]
mod integration_tests {
    use super::*;
    use chrono::NaiveDate;
    use domain_eleve::{
        ChangeClasse, CreateEleve, DeclareAbsence, SortirEleve, today_utc, EleveStatus,
    };
    use uuid::Uuid;

    fn local_url() -> Option<String> {
        std::env::var("ENT_DATABASE_URL")
            .ok()
            .filter(|u| !u.trim().is_empty())
    }

    async fn connect_local() -> Option<PgEnt> {
        let url = local_url()?;
        if assert_local_postgres_url(&url).is_err() {
            panic!("ENT_DATABASE_URL must be local — refus de lancer L1 sur un hôte distant");
        }
        let mut ent = PgEnt::connect(&url).await.expect("connect local");
        ent.migrate().await.expect("migrate");
        ent.install_brain_p0();
        Some(ent)
    }

    #[tokio::test]
    async fn connect_rejects_remote_even_if_env_points_away() {
        match PgEnt::connect("postgresql://u:p@51.158.57.80:20566/db").await {
            Err(EntDbError::Url(UrlGuardError::RemoteHost { host })) => {
                assert_eq!(host, "51.158.57.80");
            }
            Err(other) => panic!("unexpected error: {other}"),
            Ok(_) => panic!("remote must fail before TCP"),
        }
    }

    #[tokio::test]
    async fn create_and_change_classe_persists_when_local_db_available() {
        let Some(ent) = connect_local().await else {
            eprintln!("skip: set ENT_DATABASE_URL to a 127.0.0.1 Postgres to run L1");
            return;
        };

        let etab = Uuid::new_v4();
        let eleve = ent
            .create_eleve(CreateEleve {
                etablissement_id: etab,
                nom: "Test".into(),
                prenom: "L1".into(),
                ine: None,
                sexe: None,
                date_naissance: None,
                numero_badge: None,
                redoublant: false,
                classe: Some("6A".into()),
                regime: None,
                mef: None,
                site: None,
                date_entree: Some(NaiveDate::from_ymd_opt(2026, 9, 1).unwrap()),
                date_sortie: None,
                motif_sortie: None,
                actor_user_id: Some("coder".into()),
            })
            .await
            .expect("create");

        let jour = today_utc();
        ent.change_classe(ChangeClasse {
            etablissement_id: etab,
            eleve_id: eleve.id,
            nouvelle_classe: "5B".into(),
            date_effet: jour,
            actor_user_id: None,
        })
        .await
        .expect("change");

        let loaded = ent
            .get_eleve(etab, eleve.id)
            .await
            .expect("get")
            .expect("exists");
        assert_eq!(loaded.classe.as_deref(), Some("5B"));

        let in_5b = ent
            .eleves_en_classe_au(etab, "5B", jour)
            .await
            .expect("effectif");
        assert!(in_5b.iter().any(|e| e.id == eleve.id));
        let in_6a = ent
            .eleves_en_classe_au(etab, "6A", jour)
            .await
            .expect("old");
        assert!(!in_6a.iter().any(|e| e.id == eleve.id));

        let brain = ent.brain().expect("brain installed");
        assert!(brain
            .journal
            .types()
            .contains(&"scolarite.classe_changed".to_string()));

        let other = Uuid::new_v4();
        assert!(ent.get_eleve(other, eleve.id).await.expect("x").is_none());
        assert!(ent
            .eleves_en_classe_au(other, "5B", jour)
            .await
            .expect("iso")
            .is_empty());
    }

    #[tokio::test]
    async fn date_effet_jeudi_laisse_mardi_en_6a() {
        let Some(ent) = connect_local().await else {
            eprintln!("skip: set ENT_DATABASE_URL to a 127.0.0.1 Postgres to run L1");
            return;
        };

        let etab = Uuid::new_v4();
        let mardi = NaiveDate::from_ymd_opt(2026, 10, 6).unwrap();
        let jeudi = NaiveDate::from_ymd_opt(2026, 10, 8).unwrap();
        let eleve = ent
            .create_eleve(CreateEleve {
                etablissement_id: etab,
                nom: "Chrono".into(),
                prenom: "Daté".into(),
                ine: None,
                sexe: None,
                date_naissance: None,
                numero_badge: None,
                redoublant: false,
                classe: Some("6A".into()),
                regime: Some("DP".into()),
                mef: None,
                site: None,
                date_entree: Some(NaiveDate::from_ymd_opt(2026, 9, 1).unwrap()),
                date_sortie: None,
                motif_sortie: None,
                actor_user_id: None,
            })
            .await
            .expect("create");

        ent.change_classe(ChangeClasse {
            etablissement_id: etab,
            eleve_id: eleve.id,
            nouvelle_classe: "5B".into(),
            date_effet: jeudi,
            actor_user_id: None,
        })
        .await
        .expect("cutover");

        let mardi_6a = ent
            .eleves_en_classe_au(etab, "6A", mardi)
            .await
            .expect("mardi");
        assert!(mardi_6a.iter().any(|e| e.id == eleve.id));
        let jeudi_5b = ent
            .eleves_en_classe_au(etab, "5B", jeudi)
            .await
            .expect("jeudi");
        assert!(jeudi_5b.iter().any(|e| e.id == eleve.id));

        let periodes = ent.list_periodes(etab, eleve.id).await.expect("hist");
        assert!(periodes.iter().any(|p| p.valeur == "6A" && p.date_fin.is_some()));
        assert!(periodes
            .iter()
            .any(|p| p.valeur == "5B" && p.date_fin.is_none()));
    }

    #[tokio::test]
    async fn sortie_retire_de_leffectif_sans_effacer_historique() {
        let Some(ent) = connect_local().await else {
            eprintln!("skip: set ENT_DATABASE_URL to a 127.0.0.1 Postgres to run L1");
            return;
        };

        let etab = Uuid::new_v4();
        let entree = NaiveDate::from_ymd_opt(2026, 9, 1).unwrap();
        let sortie = today_utc();
        let eleve = ent
            .create_eleve(CreateEleve {
                etablissement_id: etab,
                nom: "Sortie".into(),
                prenom: "L1".into(),
                ine: None,
                sexe: None,
                date_naissance: None,
                numero_badge: None,
                redoublant: false,
                classe: Some("3C".into()),
                regime: None,
                mef: None,
                site: None,
                date_entree: Some(entree),
                date_sortie: None,
                motif_sortie: None,
                actor_user_id: None,
            })
            .await
            .expect("create");

        let avant = entree;
        assert!(ent
            .eleves_en_classe_au(etab, "3C", avant)
            .await
            .expect("avant")
            .iter()
            .any(|e| e.id == eleve.id));

        ent.sortir(SortirEleve {
            etablissement_id: etab,
            eleve_id: eleve.id,
            date_sortie: sortie,
            motif_sortie: Some("fin".into()),
            actor_user_id: None,
        })
        .await
        .expect("sortie");

        let in_3c = ent
            .eleves_en_classe_au(etab, "3C", sortie)
            .await
            .expect("effectif");
        assert!(!in_3c.iter().any(|e| e.id == eleve.id));
        let loaded = ent.get_eleve(etab, eleve.id).await.expect("get").expect("row");
        assert_eq!(loaded.status, EleveStatus::Sorti);
        assert_eq!(loaded.date_sortie, Some(sortie));
        // Historique encore lisible
        assert!(!ent
            .list_periodes(etab, eleve.id)
            .await
            .expect("hist")
            .is_empty());
    }

    #[tokio::test]
    async fn absence_motif_sejour_lue_partout_quand_local_db() {
        let Some(ent) = connect_local().await else {
            eprintln!("skip: set ENT_DATABASE_URL to a 127.0.0.1 Postgres to run L1");
            return;
        };

        let etab = Uuid::new_v4();
        let eleve = ent
            .create_eleve(CreateEleve {
                etablissement_id: etab,
                nom: "Sejour".into(),
                prenom: "JourJ".into(),
                ine: None,
                sexe: None,
                date_naissance: None,
                numero_badge: None,
                redoublant: false,
                classe: Some("4B".into()),
                regime: None,
                mef: None,
                site: None,
                date_entree: None,
                date_sortie: None,
                motif_sortie: None,
                actor_user_id: None,
            })
            .await
            .expect("create");

        let debut = today_utc();
        let fin = debut + chrono::Duration::days(1);
        ent.declare_absence(DeclareAbsence {
            etablissement_id: etab,
            eleve_id: eleve.id,
            debut,
            fin,
            motif: "sejour".into(),
            actor_user_id: None,
        })
        .await
        .expect("declare");

        assert!(ent
            .eleve_est_absent(etab, eleve.id, debut)
            .await
            .expect("read"));
        assert!(ent
            .eleve_est_absent(etab, eleve.id, fin)
            .await
            .expect("read fin"));
        let other = Uuid::new_v4();
        assert!(!ent
            .eleve_est_absent(other, eleve.id, debut)
            .await
            .expect("tenant"));
    }
}
