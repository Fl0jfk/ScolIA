//! Scénarios « vie réelle » multi-personas — labo local pour agents / sous-bots.
//!
//! Chaque scénario enchaîne des gestes **par rôle** et vérifie ce que les autres
//! voient (ou ne voient pas). Étendre ici quand notes / EDT / nomenclatures
//! s'enrichissent — le cast `seed_labo_cast` reste l'entrée unique.

use super::*;
use chrono::{Datelike, Duration, NaiveDate, NaiveTime};
use domain_eleve::{
    annee_scolaire_de, debut_annee_scolaire, today_utc, AffecterBourse, AffecterEleveInternat,
    AjouterDossierConseil, ChangeClasse, ChangeRegime, ChangeStatus, CreerAnnonce,
    CreerChambreInternat, CreerConseilClasse, CreerFacture, DeclareAbsence, DeciderBourse,
    DeciderDossierConseil,
    EleveStatus, EncaisserFacture, JoursMasque, LigneFactureCmd, MoyenPaiement, PoserGrilleRepas,
    PublierDocASigner, SortirEleve,
};
use domain_eleve::DeclarerRetard;
use domain_eleve::{
    CreerCompetenceItem, CreerNotePeriode, EvaluerCompetence, ExporterLsun, NiveauMaitrise,
};
use ent_auth::Role;
use uuid::Uuid;

/// Premier mercredi ≥ 1er septembre — couvert par une grille lun–ven et hors
/// absences seedées (le seed pose une absence pour Harry à `jour - 3`).
fn mercredi_scolaire(jour: NaiveDate) -> NaiveDate {
    let debut = debut_annee_scolaire(jour);
    let offset = (2 - debut.weekday().num_days_from_monday() as i64).rem_euclid(7);
    debut + Duration::days(offset)
}

async fn labo() -> Option<(PgEnt, LaboCast)> {
    let url = std::env::var("ENT_DATABASE_URL").ok()?;
    if assert_local_postgres_url(&url).is_err() {
        panic!("ENT_DATABASE_URL must be local");
    }
    let mut ent = PgEnt::connect(&url).await.expect("connect");
    ent.migrate().await.expect("migrate");
    ent.install_brain_p0();
    let cast = ent.seed_labo_cast().await.expect("seed cast");
    Some((ent, cast))
}

/// Adminstratif crée Maths (nomenclature) → prof note Léa →
/// parent Léa voit · parent Tom ne voit pas Léa · compta ne saisit pas ·
/// direction change classe · prof déclare absence · parent Tom ne justifie pas.
#[tokio::test]
async fn vie_etablissement_matiere_note_absence_roles() {
    let Some((ent, cast)) = labo().await else {
        eprintln!("skip scénario: ENT_DATABASE_URL");
        return;
    };

    // 1) Prof demande Anglais — c'est l'administratif qui la crée (nomenclature).
    // Code non fourni par le seed labo (qui pose déjà MATH/POTIONS/FR/HG).
    assert!(PgEnt::role_can_manage_matieres(Role::Administratif.as_str()));
    assert!(!PgEnt::role_can_manage_matieres(Role::Prof.as_str()));
    assert!(!PgEnt::role_can_manage_matieres(Role::Compta.as_str()));

    let anglais = ent
        .create_matiere(
            CreateMatiere {
                etablissement_id: cast.etablissement_id,
                code: "ANG".into(),
                libelle: "Anglais".into(),
                source_nomenclature: Some("SIECLE:ANG".into()),
            },
            Some(cast.administratif.user_id.to_string()),
        )
        .await
        .expect("admini crée anglais");

    let catalogue = ent
        .list_matieres(cast.etablissement_id)
        .await
        .expect("list");
    assert!(catalogue.iter().any(|m| m.id == anglais.id));

    // 2) Prof saisit 14/20 pour Léa — compta ne peut pas.
    assert!(PgEnt::role_can_saisir_notes(Role::Prof.as_str()));
    assert!(!PgEnt::role_can_saisir_notes(Role::Compta.as_str()));
    assert!(!PgEnt::role_can_saisir_notes(Role::Parent.as_str()));

    let note = ent
        .saisir_note(SaisirNote {
            etablissement_id: cast.etablissement_id,
            eleve_id: cast.eleve_lea_id,
            matiere_id: anglais.id,
            valeur: 14.0,
            sur: 20.0,
            date_note: today_utc(),
            auteur_user_id: cast.prof.user_id,
            libelle: Some("DS1".into()),
            devoir_id: None,
            periode_id: None,
        })
        .await
        .expect("prof note");

    // 3) Visibilité croisée notes
    assert!(ent
        .actor_can_read_notes(
            cast.etablissement_id,
            cast.parent_lea.user_id,
            Role::Parent.as_str(),
            cast.eleve_lea_id,
        )
        .await
        .unwrap());
    // Parent Tom n'est pas lié à Léa → pas de notes Léa
    assert!(!ent
        .actor_can_read_notes(
            cast.etablissement_id,
            cast.parent_tom.user_id,
            Role::Parent.as_str(),
            cast.eleve_lea_id,
        )
        .await
        .unwrap());
    // Parent Tom lié à Tom avec peut_voir_notes=true → OK pour son enfant
    assert!(ent
        .actor_can_read_notes(
            cast.etablissement_id,
            cast.parent_tom.user_id,
            Role::Parent.as_str(),
            cast.eleve_tom_id,
        )
        .await
        .unwrap());
    // Compta : pas lecture notes staff
    assert!(!ent
        .actor_can_read_notes(
            cast.etablissement_id,
            cast.compta.user_id,
            Role::Compta.as_str(),
            cast.eleve_lea_id,
        )
        .await
        .unwrap());

    let notes_lea = ent
        .notes_for_eleve(cast.etablissement_id, cast.eleve_lea_id)
        .await
        .expect("notes lea");
    assert!(notes_lea.iter().any(|n| n.id == note.id && n.valeur == 14.0));

    // Parent Tom ne « voit » pas Léa via get_eleve_for_actor
    assert!(ent
        .get_eleve_for_actor(
            cast.etablissement_id,
            cast.parent_tom.user_id,
            Role::Parent.as_str(),
            cast.eleve_lea_id,
        )
        .await
        .unwrap()
        .is_none());
    assert!(ent
        .get_eleve_for_actor(
            cast.etablissement_id,
            cast.parent_lea.user_id,
            Role::Parent.as_str(),
            cast.eleve_lea_id,
        )
        .await
        .unwrap()
        .is_some());

    // 4) Direction change la classe de Léa (date d'effet)
    assert!(PgEnt::role_can_manage_eleves(Role::Direction.as_str()));
    assert!(!PgEnt::role_can_manage_eleves(Role::Prof.as_str()));
    let jour = today_utc();
    ent.change_classe(ChangeClasse {
        etablissement_id: cast.etablissement_id,
        eleve_id: cast.eleve_lea_id,
        nouvelle_classe: "4A".into(),
        date_effet: jour,
        actor_user_id: Some(cast.direction.user_id.to_string()),
    })
    .await
    .expect("direction classe");
    let effectif = ent
        .eleves_en_classe_au(cast.etablissement_id, "4A", jour)
        .await
        .expect("effectif");
    assert!(effectif.iter().any(|e| e.id == cast.eleve_lea_id));

    // 5) Prof déclare absence Léa ; parent Léa peut justifier ; parent Tom non (flag off + autre enfant)
    ent.declare_absence(DeclareAbsence {
        etablissement_id: cast.etablissement_id,
        eleve_id: cast.eleve_lea_id,
        debut: jour,
        fin: jour,
        motif: "maladie".into(),
        actor_user_id: Some(cast.prof.user_id.to_string()),
    })
    .await
    .expect("prof absence");
    assert!(ent
        .actor_can_declare_absence(
            cast.etablissement_id,
            cast.parent_lea.user_id,
            Role::Parent.as_str(),
            cast.eleve_lea_id,
        )
        .await
        .unwrap());
    assert!(!ent
        .actor_can_declare_absence(
            cast.etablissement_id,
            cast.parent_tom.user_id,
            Role::Parent.as_str(),
            cast.eleve_tom_id,
        )
        .await
        .unwrap());

    // 6) Events visibles pour Brain / agents
    let events = ent
        .events_for_eleve(cast.etablissement_id, cast.eleve_lea_id)
        .await
        .expect("events");
    assert!(events.iter().any(|e| e.type_name == "note.saisie"));
    assert!(events.iter().any(|e| e.type_name == "eleve.absence_declared"));
    assert!(events
        .iter()
        .any(|e| e.type_name == "scolarite.classe_changed"));
}

/// Appel → a_traiter → parent soumet motif → CPE tranche → justifiee.
#[tokio::test]
async fn justif_parent_apres_appel() {
    let Some((ent, cast)) = labo().await else {
        eprintln!("skip scénario: ENT_DATABASE_URL");
        return;
    };
    let jour = today_utc();
    let (appel, _) = ent
        .ouvrir_appel(
            cast.etablissement_id,
            cast.creneau_rogue_4b,
            jour,
            cast.prof.user_id,
            Role::Prof.as_str(),
        )
        .await
        .expect("ouvrir");
    ent.marquer_appel_ligne(
        cast.etablissement_id,
        appel.id,
        cast.eleve_lea_id,
        LigneEtat::Absent,
    )
    .await
    .expect("mark");
    ent.clore_appel(
        cast.etablissement_id,
        appel.id,
        Some(cast.prof.user_id.to_string()),
    )
    .await
    .expect("clore");

    let file = ent
        .list_absences_a_traiter(cast.etablissement_id, jour)
        .await
        .expect("file");
    let abs = file
        .iter()
        .find(|a| a.eleve_id == cast.eleve_lea_id)
        .expect("lea in file");
    assert_eq!(abs.statut.as_str(), "a_traiter");

    let soumise = ent
        .justifier_absence_for_actor(
            cast.etablissement_id,
            cast.parent_lea.user_id,
            Role::Parent.as_str(),
            abs.id,
            "maladie".into(),
        )
        .await
        .expect("soumission");
    assert_eq!(soumise.statut.as_str(), "a_traiter");
    assert_eq!(soumise.motif_justif.as_deref(), Some("maladie"));

    assert!(ent
        .justifier_absence_for_actor(
            cast.etablissement_id,
            cast.parent_tom.user_id,
            Role::Parent.as_str(),
            abs.id,
            "non".into(),
        )
        .await
        .is_err());

    // Encore dans la file tant que VS n'a pas tranché
    let mid = ent
        .list_absences_a_traiter(cast.etablissement_id, jour)
        .await
        .expect("mid");
    assert!(mid.iter().any(|a| a.id == abs.id));

    let ok = ent
        .accepter_justificatif_for_actor(
            cast.etablissement_id,
            cast.cpe.user_id,
            Role::Cpe.as_str(),
            abs.id,
            Some("ok".into()),
        )
        .await
        .expect("cpe accepte");
    assert_eq!(ok.statut.as_str(), "justifiee");

    let after = ent
        .list_absences_a_traiter(cast.etablissement_id, jour)
        .await
        .expect("after");
    assert!(!after.iter().any(|a| a.id == abs.id));
}

/// Preuve Brain : clôture d'appel → absents écrits dans `ent_absence` (une vérité).
#[tokio::test]
async fn appel_cloture_remonte_dans_ent_absence() {
    let Some((ent, cast)) = labo().await else {
        eprintln!("skip scénario: ENT_DATABASE_URL");
        return;
    };

    assert!(PgEnt::role_can_faire_appel(Role::Prof.as_str()));
    assert!(!PgEnt::role_can_faire_appel(Role::Compta.as_str()));
    assert!(!PgEnt::role_can_faire_appel(Role::Parent.as_str()));

    let jour = today_utc();
    let (appel, lignes) = ent
        .ouvrir_appel(
            cast.etablissement_id,
            cast.creneau_rogue_4b,
            jour,
            cast.prof.user_id,
            Role::Prof.as_str(),
        )
        .await
        .expect("ouvrir");
    assert!(lignes.iter().any(|l| l.eleve_id == cast.eleve_lea_id));

    ent.marquer_appel_ligne(
        cast.etablissement_id,
        appel.id,
        cast.eleve_lea_id,
        LigneEtat::Absent,
    )
    .await
    .expect("marquer");

    ent.clore_appel(
        cast.etablissement_id,
        appel.id,
        Some(cast.prof.user_id.to_string()),
    )
    .await
    .expect("clore");

    // Remontée : même table que VS / cantine / famille
    assert!(ent
        .eleve_est_absent(cast.etablissement_id, cast.eleve_lea_id, jour)
        .await
        .expect("absent"));
    assert!(!ent
        .eleve_est_absent(cast.etablissement_id, cast.eleve_tom_id, jour)
        .await
        .expect("tom présent"));

    let brain = ent.brain().expect("brain");
    assert!(brain.journal.types().contains(&"appel.closed".to_string()));
}

/// Incident prof → sanction CPE ; parent VS voit ; prof ne sanctionne pas.
#[tokio::test]
async fn sanctions_incident_cpe_authz() {
    let Some((ent, cast)) = labo().await else {
        eprintln!("skip scénario: ENT_DATABASE_URL");
        return;
    };
    let jour = today_utc();
    let types = ent
        .list_sanction_types(cast.etablissement_id)
        .await
        .expect("types");
    let retenue = types.iter().find(|t| t.code == "retenue").expect("retenue");
    let inc_type = ent
        .creer_incident_type(domain_eleve::CreerReferentielVs {
            etablissement_id: cast.etablissement_id,
            code: "perturbation".into(),
            libelle: "Perturbation".into(),
            actor_user_id: None,
        })
        .await
        .expect("inc type");
    let incident = ent
        .declarer_incident_for_actor(
            Role::Prof.as_str(),
            domain_eleve::DeclarerIncident {
                etablissement_id: cast.etablissement_id,
                eleve_id: cast.eleve_lea_id,
                type_id: inc_type.id,
                jour,
                description: None,
                actor_user_id: cast.prof.user_id,
            },
        )
        .await
        .expect("incident");
    assert!(ent
        .prononcer_sanction_for_actor(
            Role::Prof.as_str(),
            domain_eleve::PrononcerSanction {
                etablissement_id: cast.etablissement_id,
                eleve_id: cast.eleve_lea_id,
                type_id: retenue.id,
                motif_id: None,
                incident_id: Some(incident.id),
                jour,
                duree_minutes: Some(30),
                note: None,
                actor_user_id: cast.prof.user_id,
            },
        )
        .await
        .is_err());
    let san = ent
        .prononcer_sanction_for_actor(
            Role::Cpe.as_str(),
            domain_eleve::PrononcerSanction {
                etablissement_id: cast.etablissement_id,
                eleve_id: cast.eleve_lea_id,
                type_id: retenue.id,
                motif_id: None,
                incident_id: Some(incident.id),
                jour,
                duree_minutes: Some(30),
                note: None,
                actor_user_id: cast.cpe.user_id,
            },
        )
        .await
        .expect("sanction");
    let d = ent
        .get_dossier_for_actor(
            cast.etablissement_id,
            cast.parent_lea.user_id,
            Role::Parent.as_str(),
            cast.eleve_lea_id,
            jour,
        )
        .await
        .unwrap()
        .unwrap();
    assert!(d.sanctions.iter().any(|s| s.id == san.id));
}

/// Continuité N-1/N/N+1 + montée rapide sans passage juridique.
#[tokio::test]
async fn continuum_montee_rapide_sans_passage_juridique() {
    let Some((ent, cast)) = labo().await else {
        eprintln!("skip scénario: ENT_DATABASE_URL");
        return;
    };
    let jour = today_utc();
    let (nm1, n, np1) = domain_eleve::annees_continuum(jour);

    let d = ent
        .get_dossier_for_actor(
            cast.etablissement_id,
            cast.administratif.user_id,
            Role::Administratif.as_str(),
            cast.eleve_lea_id,
            jour,
        )
        .await
        .unwrap()
        .unwrap();
    assert!(d.continuum.iter().any(|s| s.annee_scolaire == nm1));
    assert!(d.continuum.iter().any(|s| s.annee_scolaire == n));

    // Sans campagne ouverte pour l'année cible → refus.
    // Le seed ouvre désormais N+1 ; on vise donc N+1 en partant de N-1 (campagne de N absente).
    assert!(ent
        .montee_rapide_for_actor(
            Role::Administratif.as_str(),
            domain_eleve::MonteeRapide {
                etablissement_id: cast.etablissement_id,
                eleve_id: cast.eleve_lea_id,
                annee_courante: nm1.clone(),
                actor_user_id: None,
            },
            jour,
        )
        .await
        .is_err());

    ent.ouvrir_campagne_reinscription_for_actor(
        Role::Administratif.as_str(),
        domain_eleve::OuvrirCampagneReinscription {
            etablissement_id: cast.etablissement_id,
            annee_cible: np1.clone(),
            actor_user_id: None,
        },
    )
    .await
    .expect("campagne");

    let n1 = ent
        .montee_rapide_for_actor(
            Role::Administratif.as_str(),
            domain_eleve::MonteeRapide {
                etablissement_id: cast.etablissement_id,
                eleve_id: cast.eleve_lea_id,
                annee_courante: n,
                actor_user_id: None,
            },
            jour,
        )
        .await
        .expect("montee");
    assert_eq!(n1.annee_scolaire, np1);
    assert_eq!(n1.niveau.as_deref(), Some("3e"));
    assert!(!n1.passage_juridique);
    assert!(n1.redoublement_possible);
    assert!(n1.classe.is_none());

    assert!(ent
        .montee_rapide_for_actor(
            Role::Prof.as_str(),
            domain_eleve::MonteeRapide {
                etablissement_id: cast.etablissement_id,
                eleve_id: cast.eleve_lea_id,
                annee_courante: annee_scolaire_de(jour),
                actor_user_id: None,
            },
            jour,
        )
        .await
        .is_err());
}

/// Lot montée : toutes les 6e → 5e, idempotent, authz staff.
#[tokio::test]
async fn continuum_montee_lot_par_niveau() {
    let Some((ent, cast)) = labo().await else {
        eprintln!("skip scénario: ENT_DATABASE_URL");
        return;
    };
    let jour = today_utc();
    let (_, n, np1) = domain_eleve::annees_continuum(jour);

    ent.ouvrir_campagne_reinscription_for_actor(
        Role::Administratif.as_str(),
        domain_eleve::OuvrirCampagneReinscription {
            etablissement_id: cast.etablissement_id,
            annee_cible: np1.clone(),
            actor_user_id: None,
        },
    )
    .await
    .expect("campagne");

    let lot = ent
        .montee_lot_for_actor(
            Role::Administratif.as_str(),
            domain_eleve::MonteeLot {
                etablissement_id: cast.etablissement_id,
                annee_courante: n.clone(),
                niveau_source: "6e".into(),
                actor_user_id: None,
            },
            jour,
        )
        .await
        .expect("lot");
    assert_eq!(lot.niveau_source, "6e");
    assert_eq!(lot.niveau_cible, "5e");
    assert_eq!(lot.annee_cible, np1);
    assert!(lot.montes.len() >= 2, "attendu ≥2 sixièmes seed");

    let lot2 = ent
        .montee_lot_for_actor(
            Role::Administratif.as_str(),
            domain_eleve::MonteeLot {
                etablissement_id: cast.etablissement_id,
                annee_courante: n,
                niveau_source: "6e".into(),
                actor_user_id: None,
            },
            jour,
        )
        .await
        .expect("lot2");
    assert!(lot2.montes.is_empty());
    assert!(lot2
        .ignores
        .iter()
        .all(|i| i.motif == "deja_projetee"));

    assert!(ent
        .montee_lot_for_actor(
            Role::Prof.as_str(),
            domain_eleve::MonteeLot {
                etablissement_id: cast.etablissement_id,
                annee_courante: annee_scolaire_de(jour),
                niveau_source: "5e".into(),
                actor_user_id: None,
            },
            jour,
        )
        .await
        .is_err());
}

/// Grille repas + bourse procuration : cantine jour, déduction facture, authz prof/compta.
#[tokio::test]
async fn repas_bourses_grille_et_deduction() {
    let Some((ent, cast)) = labo().await else {
        eprintln!("skip scénario: ENT_DATABASE_URL");
        return;
    };
    let jour = NaiveDate::from_ymd_opt(2026, 10, 14).unwrap(); // mercredi
    // Le seed pose une absence pour Harry à `jour réel - 3` ; on teste l'inscription
    // midi sur un mercredi de début d'année (grille couverte, hors absences).
    let mercredi = mercredi_scolaire(jour);
    let acts = ent
        .list_repas_activites(cast.etablissement_id)
        .await
        .expect("acts");
    assert!(acts.len() >= 2);

    assert!(ent
        .eleve_inscrit_midi_present(cast.etablissement_id, cast.eleve_lea_id, mercredi)
        .await
        .unwrap());
    assert!(!ent
        .eleve_inscrit_midi_present(cast.etablissement_id, cast.eleve_tom_id, mercredi)
        .await
        .unwrap());

    let deduct = ent
        .bourses_deductibles_eleve(cast.etablissement_id, cast.eleve_lea_id, jour)
        .await
        .unwrap();
    assert_eq!(deduct.len(), 1);
    assert!(deduct[0].avec_procuration);

    let prof = ent
        .get_dossier_for_actor(
            cast.etablissement_id,
            cast.prof.user_id,
            Role::Prof.as_str(),
            cast.eleve_lea_id,
            jour,
        )
        .await
        .unwrap()
        .unwrap();
    assert!(prof.sections.repas);
    assert!(!prof.sections.bourses);
    assert!(prof.bourses.is_empty());

    let compta = ent
        .get_dossier_for_actor(
            cast.etablissement_id,
            cast.compta.user_id,
            Role::Compta.as_str(),
            cast.eleve_lea_id,
            jour,
        )
        .await
        .unwrap()
        .unwrap();
    assert!(compta.sections.bourses);
    assert!(!compta.bourses.is_empty());

    assert!(ent
        .poser_grille_repas(domain_eleve::PoserGrilleRepas {
            etablissement_id: cast.etablissement_id,
            eleve_id: cast.eleve_tom_id,
            activite_id: acts[0].id,
            jours: domain_eleve::JoursMasque::default(),
            date_debut: NaiveDate::from_ymd_opt(2026, 9, 1).unwrap(),
            actor_user_id: None,
        })
        .await
        .is_err());
}

/// PAP restreint : prof bloqué → grant → visible ; parent voit famille standard.
#[tokio::test]
async fn documents_pap_grant_circuit() {
    let Some((ent, cast)) = labo().await else {
        eprintln!("skip scénario: ENT_DATABASE_URL");
        return;
    };
    let jour = today_utc();
    let admin_d = ent
        .get_dossier_for_actor(
            cast.etablissement_id,
            cast.administratif.user_id,
            Role::Administratif.as_str(),
            cast.eleve_lea_id,
            jour,
        )
        .await
        .unwrap()
        .unwrap();
    let pap = admin_d
        .documents
        .iter()
        .find(|d| d.titre.contains("PAP"))
        .expect("pap")
        .clone();
    assert!(!admin_d.dispositifs.is_empty());

    let prof_avant = ent
        .get_dossier_for_actor(
            cast.etablissement_id,
            cast.prof.user_id,
            Role::Prof.as_str(),
            cast.eleve_lea_id,
            jour,
        )
        .await
        .unwrap()
        .unwrap();
    assert!(!prof_avant.documents.iter().any(|d| d.id == pap.id));

    let grant = ent
        .demander_grant_document(domain_eleve::DemanderGrant {
            etablissement_id: cast.etablissement_id,
            document_id: pap.id,
            grantee_user_id: cast.prof.user_id,
            duration_days: 7,
            note: None,
        })
        .await
        .expect("demande");
    ent.decider_grant_for_actor(
        Role::Administratif.as_str(),
        domain_eleve::DeciderGrant {
            etablissement_id: cast.etablissement_id,
            grant_id: grant.id,
            accepter: true,
            actor_user_id: cast.administratif.user_id,
            note: None,
        },
    )
    .await
    .expect("grant");

    let prof_apres = ent
        .get_dossier_for_actor(
            cast.etablissement_id,
            cast.prof.user_id,
            Role::Prof.as_str(),
            cast.eleve_lea_id,
            jour,
        )
        .await
        .unwrap()
        .unwrap();
    assert!(prof_apres.documents.iter().any(|d| d.id == pap.id));
}

/// Module dossier : identité + options + authz parent + parts 100 %.
#[tokio::test]
async fn dossier_eleve_vue_et_authz() {
    let Some((ent, cast)) = labo().await else {
        eprintln!("skip scénario: ENT_DATABASE_URL");
        return;
    };
    let jour = today_utc();
    let d = ent
        .get_dossier_for_actor(
            cast.etablissement_id,
            cast.administratif.user_id,
            Role::Administratif.as_str(),
            cast.eleve_lea_id,
            jour,
        )
        .await
        .expect("dossier")
        .expect("some");
    assert!(d.eleve.date_naissance.is_some());
    assert!(d.eleve.mef.is_some());
    assert!(d.options.len() >= 2);
    assert!(d.liens.iter().any(|l| l.est_payeur && l.part_paiement == Some(100)));

    assert!(ent
        .get_dossier_for_actor(
            cast.etablissement_id,
            cast.parent_tom.user_id,
            Role::Parent.as_str(),
            cast.eleve_lea_id,
            jour,
        )
        .await
        .unwrap()
        .is_none());

    let tom = ent
        .get_dossier_for_actor(
            cast.etablissement_id,
            cast.parent_tom.user_id,
            Role::Parent.as_str(),
            cast.eleve_tom_id,
            jour,
        )
        .await
        .unwrap()
        .expect("tom");
    assert!(tom.sections.notes);
    assert!(!tom.sections.vs);

    assert!(ent
        .link_parent(CreateLien {
            etablissement_id: cast.etablissement_id,
            eleve_id: cast.eleve_lea_id,
            user_id: cast.parent_tom.user_id,
            foyer_id: None,
            lien: "autre".into(),
            peut_voir_notes: false,
            peut_voir_vs: false,
            peut_justifier_absence: false,
            peut_autoriser_sorties: false,
            est_payeur: true,
            part_paiement: Some(30),
        })
        .await
        .is_err());
}

/// Volumes → seuil → signal VS (file ouverte).
#[tokio::test]
async fn seuil_absences_ouvre_signal_vs() {
    let Some((ent, cast)) = labo().await else {
        eprintln!("skip scénario: ENT_DATABASE_URL");
        return;
    };
    let debut = NaiveDate::from_ymd_opt(2026, 9, 1).unwrap();
    let cible = cast.eleves[5].id;
    for d in 0..10 {
        let j = debut + chrono::Duration::days(d);
        ent.declare_absence(DeclareAbsence {
            etablissement_id: cast.etablissement_id,
            eleve_id: cible,
            debut: j,
            fin: j,
            motif: "maladie".into(),
            actor_user_id: Some(cast.cpe.user_id.to_string()),
        })
        .await
        .expect("declare");
    }
    let signal = ent
        .signal_absence_ouvert(cast.etablissement_id, cible, debut)
        .await
        .expect("signal")
        .expect("ouvert");
    assert!(signal.compteur >= 10);
    assert_eq!(signal.seuil, 10);
    let file = ent
        .list_signaux_absences_ouverts(cast.etablissement_id)
        .await
        .expect("file");
    assert!(file.iter().any(|s| s.eleve_id == cible));
}

/// Motif établissement utilisable ; motif inconnu refusé.
#[tokio::test]
async fn motif_etab_et_refus_inconnu() {
    let Some((ent, cast)) = labo().await else {
        eprintln!("skip scénario: ENT_DATABASE_URL");
        return;
    };
    assert!(ent
        .create_motif_absence_etab(cast.etablissement_id, "sort ie", "bad", None)
        .await
        .is_err());

    let motif = ent
        .create_motif_absence_etab(
            cast.etablissement_id,
            "competition",
            "Compétition sportive",
            Some(cast.administratif.user_id.to_string()),
        )
        .await
        .expect("ok");
    assert_eq!(motif.code, "competition");

    let jour = today_utc();
    assert!(ent
        .declare_absence(DeclareAbsence {
            etablissement_id: cast.etablissement_id,
            eleve_id: cast.eleve_lea_id,
            debut: jour,
            fin: jour,
            motif: "pas_du_catalogue".into(),
            actor_user_id: None,
        })
        .await
        .is_err());

    ent.declare_absence(DeclareAbsence {
        etablissement_id: cast.etablissement_id,
        eleve_id: cast.eleve_lea_id,
        debut: jour + chrono::Duration::days(2),
        fin: jour + chrono::Duration::days(2),
        motif: "competition".into(),
        actor_user_id: Some(cast.cpe.user_id.to_string()),
    })
    .await
    .expect("motif etab");
}

/// Isolation tenant : données du cast B invisibles sous l'établissement A.
#[tokio::test]
async fn deux_etablissements_personas_isoles() {
    let Some((ent, cast_a)) = labo().await else {
        eprintln!("skip scénario: ENT_DATABASE_URL");
        return;
    };
    let cast_b = ent.seed_labo_cast().await.expect("cast b");
    assert_ne!(cast_a.etablissement_id, cast_b.etablissement_id);

    assert!(ent
        .get_eleve(cast_a.etablissement_id, cast_b.eleve_lea_id)
        .await
        .unwrap()
        .is_none());
    assert!(ent
        .notes_for_eleve(cast_a.etablissement_id, cast_b.eleve_lea_id)
        .await
        .unwrap()
        .is_empty());
    // Le seed fournit désormais des matières : l'isolation se prouve par l'absence
    // des entités de B sous A (jamais par un catalogue vide).
    let matieres_a = ent.list_matieres(cast_a.etablissement_id).await.unwrap();
    let matieres_b = ent.list_matieres(cast_b.etablissement_id).await.unwrap();
    assert!(!matieres_b.is_empty(), "cast B doit avoir ses matières seedées");
    assert!(
        matieres_a
            .iter()
            .all(|m| !matieres_b.iter().any(|n| n.id == m.id)),
        "aucune matière de B ne doit apparaître sous A"
    );
}

/// Changement de classe daté : **une** écriture, visible sur l'effectif daté,
/// l'EDT lu par la famille, le dossier staff — et publiée sur le bus (Brain).
#[tokio::test]
async fn changement_classe_date_visible_partout() {
    let Some((ent, cast)) = labo().await else {
        eprintln!("skip scénario: ENT_DATABASE_URL");
        return;
    };
    let jour = today_utc();
    let cedric = cast
        .eleves
        .iter()
        .find(|e| e.prenom == "Cedric")
        .expect("Cedric dans le cast")
        .clone();

    // Avant : Cedric en 3C (classe non planifiée → EDT famille vide).
    assert!(ent
        .eleves_en_classe_au(cast.etablissement_id, "3C", jour)
        .await
        .unwrap()
        .iter()
        .any(|e| e.id == cedric.id));
    assert!(ent
        .list_edt_eleve_jour_for_actor(
            cast.etablissement_id,
            Role::Parent.as_str(),
            cedric.parent.user_id,
            cedric.id,
            jour,
        )
        .await
        .unwrap()
        .is_empty());

    ent.change_classe(ChangeClasse {
        etablissement_id: cast.etablissement_id,
        eleve_id: cedric.id,
        nouvelle_classe: "5A".into(),
        date_effet: jour,
        actor_user_id: Some(cast.direction.user_id.to_string()),
    })
    .await
    .expect("change classe");

    // Effectif daté bascule.
    assert!(!ent
        .eleves_en_classe_au(cast.etablissement_id, "3C", jour)
        .await
        .unwrap()
        .iter()
        .any(|e| e.id == cedric.id));
    assert!(ent
        .eleves_en_classe_au(cast.etablissement_id, "5A", jour)
        .await
        .unwrap()
        .iter()
        .any(|e| e.id == cedric.id));

    // L'EDT famille suit la nouvelle classe (5A planifiée).
    assert!(!ent
        .list_edt_eleve_jour_for_actor(
            cast.etablissement_id,
            Role::Parent.as_str(),
            cedric.parent.user_id,
            cedric.id,
            jour,
        )
        .await
        .unwrap()
        .is_empty());

    // Dossier staff à jour + event socle (Brain).
    let dossier = ent
        .get_dossier_for_actor(
            cast.etablissement_id,
            cast.administratif.user_id,
            Role::Administratif.as_str(),
            cedric.id,
            jour,
        )
        .await
        .unwrap()
        .expect("dossier");
    assert_eq!(dossier.eleve.classe.as_deref(), Some("5A"));

    let events = ent
        .events_for_eleve(cast.etablissement_id, cedric.id)
        .await
        .unwrap();
    assert!(events
        .iter()
        .any(|e| e.type_name == "scolarite.classe_changed"));

    // Journal : bascule de classe tracée.
    let journal = ent
        .list_journal_eleve_for_actor(
            cast.etablissement_id,
            cast.administratif.user_id,
            Role::Administratif.as_str(),
            cedric.id,
            50,
        )
        .await
        .unwrap();
    assert!(journal.iter().any(|e| e.action == "classe_change"));

    // Isolation : un parent non lié ne lit pas l'EDT de Cedric.
    assert!(ent
        .list_edt_eleve_jour_for_actor(
            cast.etablissement_id,
            Role::Parent.as_str(),
            cast.parent_tom.user_id,
            cedric.id,
            jour,
        )
        .await
        .is_err());
}

/// Sortie d'un élève : effectif daté, statut/event, compte face détaché
/// (sessions révoquées), grilles repas clôturées — historique conservé.
#[tokio::test]
async fn sortie_eleve_revoque_compte_et_grille() {
    let Some((ent, cast)) = labo().await else {
        eprintln!("skip scénario: ENT_DATABASE_URL");
        return;
    };
    let jour = today_utc();
    let la_veille = jour - Duration::days(1);
    let eleve = cast
        .eleves
        .iter()
        .find(|e| e.prenom == "Neville")
        .expect("Neville dans le cast")
        .clone();
    let compte = eleve.compte.clone().expect("Neville a un compte face");

    assert!(ent
        .eleves_en_classe_au(cast.etablissement_id, &eleve.classe, la_veille)
        .await
        .unwrap()
        .iter()
        .any(|e| e.id == eleve.id));

    // Un doc à signer en attente avant le départ.
    let doc = ent
        .publier_doc_a_signer_for_actor(
            Role::Cpe.as_str(),
            PublierDocASigner {
                etablissement_id: cast.etablissement_id,
                eleve_id: eleve.id,
                code: "REGLEMENT".into(),
                titre: "Règlement intérieur".into(),
                corps: "Article 1 — …".into(),
                annee_scolaire: annee_scolaire_de(jour),
                echeance: None,
                actor_user_id: cast.cpe.user_id,
            },
        )
        .await
        .expect("publication doc à signer");

    let sorti = ent
        .sortir(SortirEleve {
            etablissement_id: cast.etablissement_id,
            eleve_id: eleve.id,
            date_sortie: jour,
            motif_sortie: Some("demenagement".into()),
            actor_user_id: Some(cast.administratif.user_id.to_string()),
        })
        .await
        .expect("sortie");
    assert_eq!(sorti.status, EleveStatus::Sorti);
    assert_eq!(sorti.date_sortie, Some(jour));

    // Effectif daté : présent la veille, absent le jour de sortie.
    assert!(!ent
        .eleves_en_classe_au(cast.etablissement_id, &eleve.classe, jour)
        .await
        .unwrap()
        .iter()
        .any(|e| e.id == eleve.id));
    assert!(ent
        .eleves_en_classe_au(cast.etablissement_id, &eleve.classe, la_veille)
        .await
        .unwrap()
        .iter()
        .any(|e| e.id == eleve.id));

    // Event socle + compte face détaché (plus d'accès élève).
    let events = ent
        .events_for_eleve(cast.etablissement_id, eleve.id)
        .await
        .unwrap();
    assert!(events.iter().any(|e| e.type_name == "eleve.sortie"));
    assert!(events.iter().any(|e| e.type_name == "eleve.compte_revoque"));
    assert_ne!(
        ent.eleve_id_for_compte(cast.etablissement_id, compte.user_id)
            .await
            .unwrap(),
        Some(eleve.id)
    );

    // Grilles repas clôturées avant la sortie.
    let grilles = ent
        .list_grilles_repas_eleve_for_actor(
            cast.etablissement_id,
            cast.administratif.user_id,
            Role::Administratif.as_str(),
            eleve.id,
        )
        .await
        .unwrap();
    assert!(grilles
        .iter()
        .all(|g| g.date_fin.map(|f| f < jour).unwrap_or(false)));

    // Doc à signer annulé (absent des listes « en attente » et « tous statuts »).
    let docs_pending = ent
        .list_docs_a_signer_eleve_for_actor(
            cast.etablissement_id,
            cast.administratif.user_id,
            Role::Administratif.as_str(),
            eleve.id,
            true,
        )
        .await
        .unwrap();
    assert!(docs_pending.iter().all(|d| d.id != doc.id));
    let docs_tous = ent
        .list_docs_a_signer_eleve_for_actor(
            cast.etablissement_id,
            cast.administratif.user_id,
            Role::Administratif.as_str(),
            eleve.id,
            false,
        )
        .await
        .unwrap();
    assert!(docs_tous.iter().all(|d| d.id != doc.id));

    // Historique des périodes conservé.
    assert!(!ent
        .list_periodes(cast.etablissement_id, eleve.id)
        .await
        .unwrap()
        .is_empty());

    // Journal : sortie tracée.
    let journal = ent
        .list_journal_eleve_for_actor(
            cast.etablissement_id,
            cast.administratif.user_id,
            Role::Administratif.as_str(),
            eleve.id,
            50,
        )
        .await
        .unwrap();
    assert!(journal.iter().any(|e| e.action == "eleve_sorti"));
}

/// Une bascule de statut vers `Sorti` — autre porte d'entrée que `sortir` daté —
/// doit porter les **mêmes répercussions transverses** : grille cantine fermée,
/// affectation internat close, compte élève détaché (sessions révoquées),
/// documents à signer en attente annulés. Vérifie aussi l'idempotence : une
/// seconde bascule ne republie rien.
#[tokio::test]
async fn bascule_statut_sorti_declenche_la_cascade_transverse() {
    let Some((ent, cast)) = labo().await else {
        eprintln!("skip scénario: ENT_DATABASE_URL");
        return;
    };
    let etab = cast.etablissement_id;
    let jour = today_utc();
    let la_veille = jour - Duration::days(1);

    let eleve = cast
        .eleves
        .iter()
        .find(|e| e.compte.is_some())
        .expect("un élève du cast avec compte face")
        .clone();
    let compte = eleve.compte.clone().expect("compte face");

    // 1) Grille cantine ouverte (sans date de fin) → doit être fermée.
    let act = ent
        .list_repas_activites(etab)
        .await
        .expect("activités repas")
        .into_iter()
        .next()
        .expect("au moins une activité repas");
    ent.poser_grille_repas(PoserGrilleRepas {
        etablissement_id: etab,
        eleve_id: eleve.id,
        activite_id: act.id,
        jours: JoursMasque::lun_ven(),
        date_debut: la_veille,
        actor_user_id: Some(cast.administratif.user_id.to_string()),
    })
    .await
    .expect("grille repas ouverte");

    // 2) Affectation internat ouverte (régime INT requis) → doit être close.
    ent.change_regime(ChangeRegime {
        etablissement_id: etab,
        eleve_id: eleve.id,
        nouveau_regime: "INT".into(),
        date_effet: jour,
        actor_user_id: Some(cast.cpe.user_id.to_string()),
    })
    .await
    .expect("passage régime INT");
    let chambre = ent
        .creer_chambre_internat_for_actor(
            Role::Cpe.as_str(),
            CreerChambreInternat {
                etablissement_id: etab,
                batiment: "Pavillon Test".into(),
                etage: 1,
                code: format!("T{}", &eleve.id.simple().to_string()[..4]),
                capacite: 2,
                genre: "mixte".into(),
                actor_user_id: cast.cpe.user_id,
            },
        )
        .await
        .expect("chambre internat");
    ent.affecter_eleve_internat_for_actor(
        Role::Cpe.as_str(),
        AffecterEleveInternat {
            etablissement_id: etab,
            eleve_id: eleve.id,
            chambre_id: chambre.id,
            lit: Some("1".into()),
            debut: la_veille,
            actor_user_id: cast.cpe.user_id,
        },
    )
    .await
    .expect("affectation internat");

    // 3) Document en attente de signature → doit être annulé.
    let doc = ent
        .publier_doc_a_signer_for_actor(
            Role::Cpe.as_str(),
            PublierDocASigner {
                etablissement_id: etab,
                eleve_id: eleve.id,
                code: "REGLEMENT".into(),
                titre: "Règlement intérieur".into(),
                corps: "Article 1".into(),
                annee_scolaire: annee_scolaire_de(jour),
                echeance: None,
                actor_user_id: cast.cpe.user_id,
            },
        )
        .await
        .expect("doc à signer");

    // Bascule directe de statut → Sorti (sans passer par `sortir`).
    let sorti = ent
        .change_status(ChangeStatus {
            etablissement_id: etab,
            eleve_id: eleve.id,
            nouveau_status: EleveStatus::Sorti,
            actor_user_id: Some(cast.administratif.user_id.to_string()),
        })
        .await
        .expect("bascule sorti");
    assert_eq!(sorti.status, EleveStatus::Sorti);

    // Répercussions transverses publiées.
    let events = ent.events_for_eleve(etab, eleve.id).await.unwrap();
    assert!(
        events.iter().any(|e| e.type_name == "eleve.grille_repas_close"),
        "grille repas fermée attendue"
    );
    assert!(
        events.iter().any(|e| e.type_name == "internat.affectation"),
        "affectation internat close attendue"
    );
    assert!(
        events.iter().any(|e| e.type_name == "eleve.compte_revoque"),
        "compte élève révoqué attendu"
    );

    // Cantine : plus aucune grille ouverte au jour de sortie.
    let grilles = ent
        .list_grilles_repas_eleve_for_actor(
            etab,
            cast.administratif.user_id,
            Role::Administratif.as_str(),
            eleve.id,
        )
        .await
        .unwrap();
    assert!(grilles
        .iter()
        .all(|g| g.date_fin.map(|f| f < jour).unwrap_or(false)));

    // Internat : l'élève n'est plus affecté au jour de sortie.
    let affectations = ent
        .list_affectations_internat_jour_for_actor(etab, Role::Cpe.as_str(), jour)
        .await
        .unwrap();
    assert!(affectations.iter().all(|a| a.eleve_id != eleve.id));

    // Scolarité : périodes closes la veille (historique conservé).
    let periodes = ent.list_periodes(etab, eleve.id).await.unwrap();
    assert!(!periodes.is_empty(), "périodes de scolarité attendues");
    assert!(
        periodes.iter().all(|p| p.date_fin.is_some()),
        "toutes les périodes doivent être closes après la sortie"
    );

    // Accès : compte face détaché de l'élève.
    assert_ne!(
        ent.eleve_id_for_compte(etab, compte.user_id)
            .await
            .unwrap(),
        Some(eleve.id)
    );

    // Docs en attente : annulé.
    let pending = ent
        .list_docs_a_signer_eleve_for_actor(
            etab,
            cast.administratif.user_id,
            Role::Administratif.as_str(),
            eleve.id,
            true,
        )
        .await
        .unwrap();
    assert!(pending.iter().all(|d| d.id != doc.id));

    // Idempotence : rejouer la bascule ne republie aucune répercussion de sortie
    // (le journal administratif, lui, reste alimenté à chaque geste).
    let cascade = [
        "eleve.grille_repas_close",
        "internat.affectation",
        "eleve.compte_revoque",
        "doc.a_signer",
    ];
    let avant = ent.events_for_eleve(etab, eleve.id).await.unwrap();
    let avant_cascade = avant
        .iter()
        .filter(|e| cascade.contains(&e.type_name.as_str()))
        .count();
    ent.change_status(ChangeStatus {
        etablissement_id: etab,
        eleve_id: eleve.id,
        nouveau_status: EleveStatus::Sorti,
        actor_user_id: Some(cast.administratif.user_id.to_string()),
    })
    .await
    .expect("re-bascule sorti (no-op)");
    let apres = ent.events_for_eleve(etab, eleve.id).await.unwrap();
    let apres_cascade = apres
        .iter()
        .filter(|e| cascade.contains(&e.type_name.as_str()))
        .count();
    assert_eq!(
        avant_cascade, apres_cascade,
        "aucune répercussion de sortie republiée sur un élève déjà sorti"
    );
}

/// Communication établissement ↔ familles : publier un **bulletin** ou un
/// **document à signer** n'est pas qu'un événement interne — les responsables
/// liés reçoivent une notification (outbox mail). Preuve de la répercussion
/// transverse notes / documents → familles.
#[tokio::test]
async fn publications_famille_notifient_les_responsables() {
    let Some((mut ent, cast)) = labo().await else {
        eprintln!("skip scénario: ENT_DATABASE_URL");
        return;
    };
    ent.install_mail_dry_run();
    let etab = cast.etablissement_id;
    let eleve_id = cast.eleve_lea_id;
    let jour = today_utc();

    // 1) Bulletin publié (Trimestre) → mail famille.
    let periodes = ent.list_note_periodes(etab).await.expect("note periodes");
    let periode = periodes
        .first()
        .expect("le seed pose au moins une période de notes")
        .clone();
    let bulletin = ent
        .publier_bulletin_for_actor(
            Role::Prof.as_str(),
            etab,
            eleve_id,
            periode.id,
            cast.prof.user_id,
        )
        .await
        .expect("publication bulletin");

    // 2) Document à signer publié → mail famille (action requise).
    let doc = ent
        .publier_doc_a_signer_for_actor(
            Role::Cpe.as_str(),
            PublierDocASigner {
                etablissement_id: etab,
                eleve_id,
                code: "REGLEMENT_LAB".to_string(),
                titre: "Règlement intérieur".to_string(),
                corps: "À signer avant la rentrée.".to_string(),
                annee_scolaire: annee_scolaire_de(jour),
                echeance: None,
                actor_user_id: cast.cpe.user_id,
            },
        )
        .await
        .expect("publication document à signer");

    let mails = ent
        .list_mails_recents_for_actor(etab, Role::Administratif.as_str(), 200)
        .await
        .expect("lecture outbox mail");
    assert!(
        mails
            .iter()
            .any(|m| m.kind == "bulletin" && m.eleve_id == Some(eleve_id)),
        "le bulletin publié doit être notifié aux responsables"
    );
    assert!(
        mails
            .iter()
            .any(|m| m.kind == "bulletin" && m.source_id == Some(bulletin.id)),
        "le mail bulletin doit être rattaché au document publié"
    );
    assert!(
        mails
            .iter()
            .any(|m| m.kind == "doc_a_signer" && m.source_id == Some(doc.id)),
        "le document à signer publié doit être notifié aux responsables"
    );
}

/// Communication établissement → familles : une **annonce** publiée par la
/// direction n'est pas qu'une donnée interne — chaque famille ciblée reçoit une
/// notification (outbox mail), avec un anti-doublon **par destinataire** : une
/// même annonce touche plusieurs familles sans qu'elles se bloquent mutuellement.
/// Le ciblage « classe » ne diffuse qu'aux familles concernées, et une annonce
/// « personnels » ne part jamais vers les familles.
#[tokio::test]
async fn annonce_etablissement_notifie_les_familles_ciblees() {
    let Some((mut ent, cast)) = labo().await else {
        eprintln!("skip scénario: ENT_DATABASE_URL");
        return;
    };
    ent.install_mail_dry_run();
    let etab = cast.etablissement_id;

    // Un parent ne publie pas une annonce d'établissement.
    let refuse = ent
        .publier_annonce_for_actor(
            Role::Parent.as_str(),
            CreerAnnonce {
                etablissement_id: etab,
                titre: "Interdit".into(),
                corps: "Un parent ne diffuse pas.".into(),
                audience: "parents".into(),
                cible_classe: None,
                actor_user_id: cast.parent_lea.user_id,
            },
        )
        .await;
    assert!(refuse.is_err(), "un parent ne peut pas publier une annonce");

    // Diffusion à toutes les familles.
    let annonce = ent
        .publier_annonce_for_actor(
            Role::Direction.as_str(),
            CreerAnnonce {
                etablissement_id: etab,
                titre: "Réunion parents-professeurs".into(),
                corps: "Le 12 mars à 17h.".into(),
                audience: "parents".into(),
                cible_classe: None,
                actor_user_id: cast.direction.user_id,
            },
        )
        .await
        .expect("publication annonce");

    let mails = ent
        .list_mails_recents_for_actor(etab, Role::Administratif.as_str(), 200)
        .await
        .expect("lecture outbox mail");
    let touches: std::collections::HashSet<Uuid> = mails
        .iter()
        .filter(|m| m.kind == "annonce" && m.source_id == Some(annonce.id))
        .filter_map(|m| m.eleve_id)
        .collect();
    assert!(
        touches.len() >= 2,
        "la diffusion doit atteindre plusieurs familles (got {})",
        touches.len()
    );
    assert!(touches.contains(&cast.eleve_lea_id));
    assert!(touches.contains(&cast.eleve_tom_id));

    // Diffusion ciblée classe → uniquement les élèves de la classe.
    let cible = ent
        .publier_annonce_for_actor(
            Role::Direction.as_str(),
            CreerAnnonce {
                etablissement_id: etab,
                titre: "Sortie pédagogique 4B".into(),
                corps: "Rendez-vous 8h devant le portail.".into(),
                audience: "classe".into(),
                cible_classe: Some("4B".into()),
                actor_user_id: cast.direction.user_id,
            },
        )
        .await
        .expect("publication annonce ciblée");
    let mails = ent
        .list_mails_recents_for_actor(etab, Role::Administratif.as_str(), 200)
        .await
        .expect("lecture outbox mail");
    let cible_ids: Vec<Option<Uuid>> = mails
        .iter()
        .filter(|m| m.kind == "annonce" && m.source_id == Some(cible.id))
        .map(|m| m.eleve_id)
        .collect();
    assert!(
        !cible_ids.is_empty(),
        "une annonce de classe doit notifier au moins une famille"
    );
    // Draco est en 5A : il ne doit pas recevoir l'annonce de la 4B.
    assert!(
        !cible_ids.contains(&Some(cast.eleve_tom_id)),
        "le ciblage classe ne diffuse pas hors de la classe"
    );

    // Audience « personnels » → aucune diffusion famille.
    let interne = ent
        .publier_annonce_for_actor(
            Role::Direction.as_str(),
            CreerAnnonce {
                etablissement_id: etab,
                titre: "Note de service interne".into(),
                corps: "Réservé aux personnels.".into(),
                audience: "personnels".into(),
                cible_classe: None,
                actor_user_id: cast.direction.user_id,
            },
        )
        .await
        .expect("publication annonce personnels");
    let mails = ent
        .list_mails_recents_for_actor(etab, Role::Administratif.as_str(), 200)
        .await
        .expect("lecture outbox mail");
    assert!(
        !mails
            .iter()
            .any(|m| m.kind == "annonce" && m.source_id == Some(interne.id)),
        "une annonce « personnels » ne part pas vers les familles"
    );

    // Ce que la famille voit : l'annonce générale + celle de sa classe, jamais
    // la note interne.
    let vues = ent
        .list_annonces_famille_for_actor(
            etab,
            cast.parent_lea.user_id,
            Role::Parent.as_str(),
            cast.eleve_lea_id,
        )
        .await
        .expect("annonces famille");
    assert!(vues.iter().any(|a| a.id == annonce.id), "annonce générale visible");
    assert!(vues.iter().any(|a| a.id == cible.id), "annonce de classe visible");
    assert!(
        !vues.iter().any(|a| a.id == interne.id),
        "la note de service interne n'est pas visible par les familles"
    );
}

/// Retards : pointé à l'appel, un retard devient un fait VS **canonique et
/// persistant** (`ent_retard`) — distinct de l'absence, visible par la famille
/// liée, jamais par une autre famille. Preuve de la répercussion appel → VS.
#[tokio::test]
async fn retard_pointe_a_l_appel_devient_fait_vs_persistant() {
    let Some((ent, cast)) = labo().await else {
        eprintln!("skip scénario: ENT_DATABASE_URL");
        return;
    };
    let etab = cast.etablissement_id;
    let jour = today_utc();
    let lea = cast.eleve_lea_id;

    // Absences de Léa au jour J **avant** tout geste (le seed peut en poser).
    let absences_avant: Vec<Uuid> = ent
        .list_absences_a_traiter(etab, jour)
        .await
        .expect("file VS")
        .into_iter()
        .filter(|a| a.eleve_id == lea)
        .map(|a| a.id)
        .collect();

    // Léa (4B) pointée « retard » à l'appel.
    let (appel, _) = ent
        .ouvrir_appel(
            etab,
            cast.creneau_rogue_4b,
            jour,
            cast.prof.user_id,
            Role::Prof.as_str(),
        )
        .await
        .expect("ouvrir");
    ent.marquer_appel_ligne(etab, appel.id, lea, LigneEtat::Retard)
        .await
        .expect("pointer retard");
    ent.clore_appel(etab, appel.id, Some(cast.prof.user_id.to_string()))
        .await
        .expect("clore");

    // Le retard est enregistré (staff VS) avec sa provenance (appel d'origine).
    let retards = ent
        .list_retards_eleve_for_actor(etab, cast.cpe.user_id, Role::Cpe.as_str(), lea, 50)
        .await
        .expect("lecture VS");
    assert_eq!(retards.len(), 1, "le retard pointé est persisté");
    assert_eq!(retards[0].jour, jour);
    assert_eq!(retards[0].source_appel_id, Some(appel.id));

    // Un retard n'est **pas** une absence : aucune absence de Léa en plus.
    let absences_apres: Vec<Uuid> = ent
        .list_absences_a_traiter(etab, jour)
        .await
        .expect("file VS")
        .into_iter()
        .filter(|a| a.eleve_id == lea)
        .map(|a| a.id)
        .collect();
    assert_eq!(
        absences_avant, absences_apres,
        "un retard ne doit pas créer d'absence"
    );

    // La famille liée voit le retard de son enfant ; une autre famille non.
    let vus_parent = ent
        .list_retards_eleve_for_actor(etab, cast.parent_lea.user_id, Role::Parent.as_str(), lea, 50)
        .await
        .expect("lecture famille");
    assert_eq!(vus_parent.len(), 1);
    assert!(
        ent.list_retards_eleve_for_actor(
            etab,
            cast.parent_tom.user_id,
            Role::Parent.as_str(),
            lea,
            50,
        )
        .await
        .is_err(),
        "un parent non lié ne voit pas les retards d'un autre élève"
    );

    // Saisie manuelle par la vie scolaire (durée), puis compteur annuel.
    let manuel = ent
        .declarer_retard_for_actor(
            Role::Cpe.as_str(),
            DeclarerRetard {
                etablissement_id: etab,
                eleve_id: lea,
                jour,
                minutes: Some(20),
                motif: Some("Bus".into()),
                source_appel_id: None,
                actor_user_id: Some(cast.cpe.user_id),
            },
        )
        .await
        .expect("saisie manuelle");
    assert_eq!(manuel.minutes, Some(20));

    // Un parent ne peut pas déclarer un retard.
    assert!(
        ent.declarer_retard_for_actor(
            Role::Parent.as_str(),
            DeclarerRetard {
                etablissement_id: etab,
                eleve_id: lea,
                jour,
                minutes: None,
                motif: None,
                source_appel_id: None,
                actor_user_id: Some(cast.parent_lea.user_id),
            },
        )
        .await
        .is_err(),
        "un parent ne déclare pas un retard"
    );

    // Re-jouer la clôture du même appel n'ajoute pas de doublon.
    assert!(
        ent.clore_appel(etab, appel.id, Some(cast.prof.user_id.to_string()))
            .await
            .is_err(),
        "un appel clos ne se re-clôture pas"
    );

    let total = ent
        .count_retards_eleve_depuis(etab, lea, debut_annee_scolaire(jour))
        .await
        .expect("compteur");
    assert_eq!(total, 2, "1 retard d'appel + 1 saisie manuelle");
}

/// Compétences / LSU : référentiel socle par établissement, évaluations réelles
/// par période, lecture famille, livret PDF et export LSU — **sans rien deviner**.
#[tokio::test]
async fn competences_lues_par_la_famille_et_export_lsu_reel() {
    let Some((mut ent, cast)) = labo().await else {
        eprintln!("skip scénario: ENT_DATABASE_URL");
        return;
    };
    ent.install_mail_dry_run();
    let etab = cast.etablissement_id;
    let lea = cast.eleve_lea_id;
    let jour = today_utc();
    // Période **ouverte** dédiée au test (le seed peut clôturer T1 pour ses audits).
    let debut = debut_annee_scolaire(jour);
    let periode = ent
        .creer_note_periode(CreerNotePeriode {
            etablissement_id: etab,
            code: "T1C".into(),
            libelle: "Trimestre compétences".into(),
            date_debut: debut,
            date_fin: debut + Duration::days(110),
            actor_user_id: Some(cast.direction.user_id.to_string()),
        })
        .await
        .expect("période ouverte");

    // 1) Le seed pose un référentiel socle + des évaluations pour Léa.
    let items = ent
        .list_competences_items_for_actor(Role::Prof.as_str(), etab, false)
        .await
        .expect("référentiel");
    assert!(
        items.len() >= 10,
        "le socle compte 5 domaines × 2 items, got {}",
        items.len()
    );
    let seed_evals = ent
        .list_evaluations_eleve_for_actor(etab, cast.prof.user_id, Role::Prof.as_str(), lea, None)
        .await
        .expect("évaluations seed");
    assert_eq!(seed_evals.len(), items.len(), "tous les items évalués");

    // Évalue l'item D1.1 dans la période dédiée (pour l'export LSU + le livret).
    let d1_1 = items
        .iter()
        .find(|i| i.code == "D1.1")
        .expect("item D1.1 du socle")
        .clone();
    ent.evaluer_competence_for_actor(
        Role::Prof.as_str(),
        EvaluerCompetence {
            etablissement_id: etab,
            eleve_id: lea,
            item_id: d1_1.id,
            periode_id: periode.id,
            niveau: "4".into(),
            commentaire: None,
            actor_user_id: cast.prof.user_id,
        },
    )
    .await
    .expect("évaluer D1.1");

    // 2) Un prof peut créer un item et évaluer ; un parent ne le peut pas.
    let nouvel = ent
        .creer_competence_item_for_actor(
            Role::Direction.as_str(),
            CreerCompetenceItem {
                etablissement_id: etab,
                domaine: "D1".into(),
                code: "D1.3".into(),
                libelle: "S'exprimer avec un vocabulaire précis".into(),
                ordre: Some(99),
            },
        )
        .await
        .expect("créer item");
    let evalue = ent
        .evaluer_competence_for_actor(
            Role::Prof.as_str(),
            EvaluerCompetence {
                etablissement_id: etab,
                eleve_id: lea,
                item_id: nouvel.id,
                periode_id: periode.id,
                niveau: "satisfaisante".into(),
                commentaire: Some("  progrès nets  ".into()),
                actor_user_id: cast.prof.user_id,
            },
        )
        .await
        .expect("évaluer");
    assert_eq!(evalue.niveau, NiveauMaitrise::Satisfaisante);
    assert_eq!(evalue.commentaire.as_deref(), Some("progrès nets"));
    assert!(
        ent.evaluer_competence_for_actor(
            Role::Parent.as_str(),
            EvaluerCompetence {
                etablissement_id: etab,
                eleve_id: lea,
                item_id: nouvel.id,
                periode_id: periode.id,
                niveau: "4".into(),
                commentaire: None,
                actor_user_id: cast.parent_lea.user_id,
            },
        )
        .await
        .is_err(),
        "un parent n'évalue pas les compétences"
    );

    // 3) Ré-évaluer met à jour (pas de doublon) : toujours 1 évaluation sur l'item.
    ent.evaluer_competence_for_actor(
        Role::Prof.as_str(),
        EvaluerCompetence {
            etablissement_id: etab,
            eleve_id: lea,
            item_id: nouvel.id,
            periode_id: periode.id,
            niveau: "4".into(),
            commentaire: None,
            actor_user_id: cast.prof.user_id,
        },
    )
    .await
    .expect("ré-évaluer");
    let apres = ent
        .list_evaluations_eleve_for_actor(etab, cast.prof.user_id, Role::Prof.as_str(), lea, Some(periode.id))
        .await
        .expect("relecture");
    assert_eq!(
        apres.iter().filter(|e| e.item_id == nouvel.id).count(),
        1,
        "une seule évaluation par (élève, item, période)"
    );

    // 4) La famille liée lit ; une autre famille non.
    let vus = ent
        .list_evaluations_eleve_for_actor(
            etab,
            cast.parent_lea.user_id,
            Role::Parent.as_str(),
            lea,
            Some(periode.id),
        )
        .await
        .expect("lecture famille");
    assert!(!vus.is_empty());
    assert!(
        ent.list_evaluations_eleve_for_actor(
            etab,
            cast.parent_tom.user_id,
            Role::Parent.as_str(),
            lea,
            Some(periode.id),
        )
        .await
        .is_err(),
        "un parent non lié ne voit pas les compétences d'un autre élève"
    );

    // 5) L'export LSU lit les évaluations réelles (codes 1..4), pas une devinette.
    let csv = ent
        .exporter_lsun_csv_for_actor(
            Role::Prof.as_str(),
            ExporterLsun {
                etablissement_id: etab,
                periode_id: periode.id,
                classe: Some("4B".into()),
                jour_reference: jour,
            },
        )
        .await
        .expect("export LSU");
    assert!(
        csv.contains("Potter;Harry;4B;D1;Comprendre et s'exprimer à l'oral;4;"),
        "LSU attendu sur évaluation réelle, got:\n{csv}"
    );

    // 6) Livret PDF : déposé famille + notifié ; refusé si aucune évaluation.
    let doc = ent
        .publier_livret_competences_for_actor(
            Role::Prof.as_str(),
            etab,
            lea,
            periode.id,
            cast.prof.user_id,
        )
        .await
        .expect("publier livret");
    assert_eq!(doc.tiroir.as_str(), "famille");
    assert_eq!(doc.mime_type.as_deref(), Some("application/pdf"));
    let mails = ent
        .list_mails_recents_for_actor(etab, Role::Administratif.as_str(), 200)
        .await
        .expect("outbox");
    assert!(
        mails
            .iter()
            .any(|m| m.kind == "livret_competences" && m.eleve_id == Some(lea)),
        "la publication du livret notifie la famille"
    );
    assert!(
        ent.publier_livret_competences_for_actor(
            Role::Prof.as_str(),
            etab,
            cast.eleve_tom_id,
            periode.id,
            cast.prof.user_id,
        )
        .await
        .is_err(),
        "pas de livret sans évaluation"
    );
}

/// Passkeys (WebAuthn) : enrôlement, 2ᵉ facteur **obligatoire** ensuite,
/// connexion sans mot de passe, protection contre le rejeu, récupération
/// administrateur et isolation par établissement.
///
/// Le test pilote un **authentificateur logiciel** (`SoftPasskey`) : c'est un
/// vrai enregistrement/assertion WebAuthn (CBOR, COSE, signature ES256), donc
/// un parcours de bout en bout, pas un simulacre.
#[tokio::test]
async fn passkeys_enrolement_2fa_passwordless_et_isolation() {
    use ent_auth::{CreationChallengeResponse, RequestChallengeResponse};
    use webauthn_authenticator_rs::prelude::WebauthnAuthenticator;
    use webauthn_authenticator_rs::softpasskey::SoftPasskey;

    let Some((ent, cast)) = labo().await else {
        eprintln!("skip scénario: ENT_DATABASE_URL");
        return;
    };
    let etab = cast.etablissement_id;
    // Compte **neuf** dédié : n'interfère jamais avec les personas seedées
    // (qui doivent rester sans passkey pour les autres scénarios).
    let email = format!("passkey-{}@ent.local", Uuid::new_v4());
    let password = "PasskeyLocal1!";
    let user = ent
        .register_user(&email, password, Some("Claire Passkey"), etab, Role::Prof.as_str())
        .await
        .expect("compte de test");
    let user_id = user.id;

    // 1) Aucune passkey : la connexion mot de passe ouvre une session directe.
    let outcome = ent
        .login_start_with_ip(&email, password, etab, None)
        .await
        .expect("login sans passkey");
    assert!(matches!(outcome, LoginOutcome::Session(_)));
    assert_eq!(ent.count_passkeys_for_user(user_id).await.unwrap(), 0);

    // 2) Enrôlement d'une passkey.
    let (reg_token, options) = ent
        .passkey_register_start_for_actor(etab, user_id)
        .await
        .expect("démarrage enrôlement");
    let mut token_dev = WebauthnAuthenticator::new(SoftPasskey::new(true));
    let origin = url::Url::parse("http://localhost:3000").expect("origine");
    let ccr: CreationChallengeResponse =
        serde_json::from_value(options).expect("options d'enrôlement");
    let registration = token_dev
        .do_registration(origin.clone(), ccr)
        .expect("authentificateur logiciel : enregistrement");
    let stored = ent
        .passkey_register_finish_for_actor(
            etab,
            user_id,
            &reg_token,
            &registration,
            Some("Pixel de test".into()),
        )
        .await
        .expect("fin d'enrôlement");
    assert_eq!(stored.name.as_deref(), Some("Pixel de test"));
    assert_eq!(ent.count_passkeys_for_user(user_id).await.unwrap(), 1);

    // Le challenge d'enrôlement est à usage unique : rejouer échoue.
    assert!(
        ent.passkey_register_finish_for_actor(etab, user_id, &reg_token, &registration, None)
            .await
            .is_err(),
        "un challenge d'enrôlement consommé ne peut pas être rejoué"
    );

    // 3) Désormais le mot de passe **seul** ne suffit plus : passkey exigée.
    let outcome = ent
        .login_start_with_ip(&email, password, etab, None)
        .await
        .expect("login mot de passe");
    let (auth_token, options) = match outcome {
        LoginOutcome::PasskeyRequired { token, options, .. } => (token, options),
        LoginOutcome::Session(_) => panic!("2ᵉ facteur non exigé alors qu'une passkey existe"),
    };
    // `login_with_ip` (chemin interne) refuse aussi : impossible de contourner.
    assert!(matches!(
        ent.login_with_ip(&email, password, etab, None).await,
        Err(EntDbError::PasskeyRequis)
    ));

    let rcr: RequestChallengeResponse =
        serde_json::from_value(options).expect("options d'authentification");
    let assertion = token_dev
        .do_authentication(origin.clone(), rcr)
        .expect("authentificateur logiciel : assertion");
    let session = ent
        .passkey_login_finish(&auth_token, &assertion)
        .await
        .expect("session par passkey");
    assert_eq!(session.user_id, user_id);
    assert_eq!(session.role, Role::Prof.as_str());

    // Rejeu du même challenge d'authentification → refusé (usage unique).
    assert!(
        ent.passkey_login_finish(&auth_token, &assertion).await.is_err(),
        "un challenge d'authentification consommé ne peut pas être rejoué"
    );

    // 4) Connexion **sans mot de passe** par passkey.
    let (t, opts, _) = ent
        .passkey_login_start(etab, &email)
        .await
        .expect("démarrage passwordless")
        .expect("passkeys présentes");
    let rcr: RequestChallengeResponse = serde_json::from_value(opts).expect("options");
    let assertion = token_dev.do_authentication(origin.clone(), rcr).expect("assertion");
    let s = ent
        .passkey_login_finish(&t, &assertion)
        .await
        .expect("session passwordless");
    assert_eq!(s.user_id, user_id);

    // 5) Isolation : un autre établissement ne voit ni n'utilise ces passkeys.
    let autre_etab = Uuid::new_v4();
    assert!(
        ent.list_passkeys_for_actor(autre_etab, user_id).await.is_err(),
        "hors établissement : accès refusé"
    );
    assert!(
        ent.passkey_login_start(autre_etab, &email)
            .await
            .expect("recherche")
            .is_none(),
        "un non-membre ne découvre aucune passkey dans un autre établissement"
    );
    let (autre_token, _) = ent
        .passkey_register_start_for_actor(etab, user_id)
        .await
        .expect("challenge");
    assert!(
        ent.passkey_register_finish_for_actor(
            autre_etab,
            user_id,
            &autre_token,
            &registration,
            None
        )
        .await
        .is_err(),
        "un challenge ne peut pas être finalisé depuis un autre établissement"
    );

    // 6) Récupération : un prof ne peut pas réinitialiser ; la Direction oui.
    assert!(
        ent.reset_passkeys_admin_for_actor(etab, Role::Prof.as_str(), user_id)
            .await
            .is_err(),
        "seuls Direction / Admin réinitialisent les passkeys"
    );
    let removed = ent
        .reset_passkeys_admin_for_actor(etab, Role::Direction.as_str(), user_id)
        .await
        .expect("réinitialisation direction");
    assert!(removed >= 1);
    assert_eq!(ent.count_passkeys_for_user(user_id).await.unwrap(), 0);

    // 7) Après réinitialisation, le mot de passe seul suffit de nouveau.
    let outcome = ent
        .login_start_with_ip(&email, password, etab, None)
        .await
        .expect("login après réinitialisation");
    assert!(matches!(outcome, LoginOutcome::Session(_)));
}

/// Extrait la valeur d'un paramètre de query depuis une URL (tests SSO).
fn query_param(url: &str, key: &str) -> String {
    let query = url.split_once('?').map(|(_, q)| q).unwrap_or_default();
    url::form_urlencoded::parse(query.as_bytes())
        .find(|(k, _)| k == key)
        .map(|(_, v)| v.into_owned())
        .unwrap_or_else(|| panic!("paramètre {key} absent de {url}"))
}

/// SSO OIDC : configuration validée par découverte, parcours PKCE + nonce,
/// rattachement stable par `sub`, refus des identités inconnues, `state` à
/// usage unique et isolation par établissement.
#[tokio::test]
async fn sso_oidc_rattachement_et_refus_des_inconnus() {
    use ent_oidc::testing::{FakeOidcTransport, TestIdp};
    use std::sync::Arc;

    let Some((ent, cast)) = labo().await else {
        eprintln!("skip scénario: ENT_DATABASE_URL");
        return;
    };
    let etab = cast.etablissement_id;

    // IdP en mémoire : aucun réseau, clé RSA éphémère.
    let mut idp = TestIdp::new();
    idp.email = format!("sso-{}@idp.test", Uuid::new_v4());
    idp.subject = format!("sub-{}", Uuid::new_v4());
    let transport = Arc::new(FakeOidcTransport::new(idp.clone()));
    ent.set_oidc_transport(transport.clone());

    // 1) La configuration est validée par **découverte réelle**.
    let provider = ent
        .upsert_oidc_provider_for_actor(
            etab,
            Role::Direction.as_str(),
            OidcProviderParams {
                slug: "idp-test".into(),
                libelle: "Annuaire de l'établissement".into(),
                issuer: idp.issuer.clone(),
                client_id: idp.client_id.clone(),
                client_secret: idp.client_secret.clone(),
                scopes: None,
                redirect_uri: None,
            },
        )
        .await
        .expect("configuration SSO");
    assert_eq!(provider.token_endpoint, idp.token_endpoint());
    // `redirect_uri` par défaut : dérivé de l'URL publique de l'ENT.
    assert!(
        provider.redirect_uri.ends_with("/auth/oidc/callback"),
        "redirect_uri par défaut attendu, obtenu {}",
        provider.redirect_uri
    );

    // Surcharge explicite (l'IdP impose parfois une URL enregistrée) : la
    // configuration est bien **mise à jour**, pas dupliquée.
    let provider = ent
        .upsert_oidc_provider_for_actor(
            etab,
            Role::Direction.as_str(),
            OidcProviderParams {
                slug: "idp-test".into(),
                libelle: "Annuaire de l'établissement".into(),
                issuer: idp.issuer.clone(),
                client_id: idp.client_id.clone(),
                client_secret: idp.client_secret.clone(),
                scopes: Some("openid email".into()),
                redirect_uri: Some(idp.redirect_uri.clone()),
            },
        )
        .await
        .expect("mise à jour SSO");
    assert_eq!(provider.redirect_uri, idp.redirect_uri);
    assert_eq!(provider.scopes, "openid email");
    assert_eq!(
        ent.list_oidc_providers_public(etab).await.unwrap().len(),
        1,
        "un upsert ne duplique pas le fournisseur"
    );

    // Un issuer injoignable est refusé (pas de fournisseur à moitié configuré).
    assert!(
        ent.upsert_oidc_provider_for_actor(
            etab,
            Role::Direction.as_str(),
            OidcProviderParams {
                slug: "casse".into(),
                libelle: "Casse".into(),
                issuer: "https://idp-inexistant.test/oidc".into(),
                client_id: "x".into(),
                client_secret: "y".into(),
                scopes: None,
                redirect_uri: None,
            },
        )
        .await
        .is_err(),
        "découverte impossible → refus"
    );

    // Un prof ne configure pas le SSO.
    assert!(
        ent.upsert_oidc_provider_for_actor(
            etab,
            Role::Prof.as_str(),
            OidcProviderParams {
                slug: "idp-test".into(),
                libelle: "Annuaire".into(),
                issuer: idp.issuer.clone(),
                client_id: idp.client_id.clone(),
                client_secret: idp.client_secret.clone(),
                scopes: None,
                redirect_uri: None,
            },
        )
        .await
        .is_err(),
        "seuls Direction / Admin configurent le SSO"
    );

    // 2) La page de connexion ne voit qu'une vue publique (slug + libellé).
    let publics = ent
        .list_oidc_providers_public(etab)
        .await
        .expect("fournisseurs publics");
    assert_eq!(publics.len(), 1);
    assert_eq!(publics[0].slug, "idp-test");

    // 3) Le compte local existe (créé par un administrateur) : même e-mail.
    let local = ent
        .register_user(&idp.email, "SsoLocal1!", Some("Claire SSO"), etab, Role::Prof.as_str())
        .await
        .expect("compte local");

    // 4) Parcours : autorisation → (IdP) → retour avec code + state.
    let url = ent.oidc_start(etab, "idp-test").await.expect("start");
    let state = query_param(&url, "state");
    let nonce = query_param(&url, "nonce");
    let challenge = query_param(&url, "code_challenge");
    assert!(url.contains("code_challenge_method=S256"));
    transport.arm_nonce(&nonce);
    transport.arm_code_challenge(&challenge);

    let session = match ent
        .oidc_complete("code-1", &state)
        .await
        .expect("retour SSO")
    {
        LoginOutcome::Session(s) => s,
        LoginOutcome::PasskeyRequired { .. } => panic!("aucune passkey sur ce compte de test"),
    };
    assert_eq!(session.user_id, local.id);
    assert_eq!(session.role, Role::Prof.as_str());
    assert_eq!(session.etablissement_id, etab);

    // 5) Le `state` est à usage unique : le rejouer échoue.
    assert!(
        ent.oidc_complete("code-1", &state).await.is_err(),
        "un state consommé ne peut pas être rejoué"
    );

    // 6) Le rattachement est stable par `sub` : l'e-mail peut changer chez
    //    l'IdP sans casser la connexion.
    let mut idp2 = idp.clone();
    idp2.email = format!("nouveau-{}@idp.test", Uuid::new_v4());
    let transport2 = Arc::new(FakeOidcTransport::new(idp2));
    ent.set_oidc_transport(transport2.clone());
    let url = ent.oidc_start(etab, "idp-test").await.expect("start 2");
    transport2.arm_nonce(&query_param(&url, "nonce"));
    transport2.arm_code_challenge(&query_param(&url, "code_challenge"));
    let session = match ent
        .oidc_complete("code-2", &query_param(&url, "state"))
        .await
        .expect("2ᵉ connexion SSO (email changé)")
    {
        LoginOutcome::Session(s) => s,
        LoginOutcome::PasskeyRequired { .. } => panic!("aucune passkey sur ce compte de test"),
    };
    assert_eq!(session.user_id, local.id);

    // 7) Identité inconnue → refus : aucun compte n'est créé à la volée.
    let mut idp3 = TestIdp::new();
    idp3.email = format!("inconnu-{}@idp.test", Uuid::new_v4());
    idp3.subject = format!("sub-inconnu-{}", Uuid::new_v4());
    let transport3 = Arc::new(FakeOidcTransport::new(idp3));
    ent.set_oidc_transport(transport3.clone());
    let url = ent.oidc_start(etab, "idp-test").await.expect("start 3");
    transport3.arm_nonce(&query_param(&url, "nonce"));
    transport3.arm_code_challenge(&query_param(&url, "code_challenge"));
    assert!(
        matches!(
            ent.oidc_complete("code-3", &query_param(&url, "state")).await,
            Err(EntDbError::OidcCompteInconnu)
        ),
        "une identité SSO inconnue ne crée pas de compte"
    );

    // 8) Isolation : le fournisseur n'existe que dans son établissement.
    let autre = Uuid::new_v4();
    assert!(
        ent.oidc_start(autre, "idp-test").await.is_err(),
        "un autre établissement n'a pas ce fournisseur"
    );
    assert!(ent.list_oidc_providers_public(autre).await.unwrap().is_empty());
    assert!(
        ent.list_oidc_providers_for_actor(etab, Role::Prof.as_str())
            .await
            .is_ok(),
        "un membre du personnel liste les fournisseurs de son établissement"
    );

    // 9) Un code invalide / nonce non conforme est refusé par la vérification.
    transport3.arm_nonce("nonce-inexistant");
    let url = ent.oidc_start(etab, "idp-test").await.expect("start 4");
    transport3.arm_code_challenge(&query_param(&url, "code_challenge"));
    assert!(
        matches!(
            ent.oidc_complete("code-4", &query_param(&url, "state")).await,
            Err(EntDbError::Oidc(_))
        ),
        "un nonce qui ne correspond pas au state est refusé"
    );

    // 10) Le SSO ne **contourne** pas un 2ᵉ facteur déjà enrôlé : une fois une
    //     passkey posée sur le compte, un parcours SSO exige l'assertion.
    {
        use ent_auth::CreationChallengeResponse;
        use webauthn_authenticator_rs::prelude::{Url, WebauthnAuthenticator};
        use webauthn_authenticator_rs::softpasskey::SoftPasskey;

        let mut device = WebauthnAuthenticator::new(SoftPasskey::new(true));
        let origin = Url::parse("http://localhost:3000").expect("origin");
        let (reg_token, options) = ent
            .passkey_register_start_for_actor(etab, local.id)
            .await
            .expect("start enrôlement");
        let ccr: CreationChallengeResponse = serde_json::from_value(options).expect("options");
        let registration = device
            .do_registration(origin.clone(), ccr)
            .expect("enregistrement WebAuthn logiciel");
        ent.passkey_register_finish_for_actor(
            etab,
            local.id,
            &reg_token,
            &registration,
            Some("Clé SSO".into()),
        )
        .await
        .expect("finish enrôlement");
    }
    let transport4 = Arc::new(FakeOidcTransport::new(idp.clone()));
    ent.set_oidc_transport(transport4.clone());
    let url = ent.oidc_start(etab, "idp-test").await.expect("start 5");
    transport4.arm_nonce(&query_param(&url, "nonce"));
    transport4.arm_code_challenge(&query_param(&url, "code_challenge"));
    match ent
        .oidc_complete("code-5", &query_param(&url, "state"))
        .await
        .expect("retour SSO avec passkey enrôlée")
    {
        LoginOutcome::PasskeyRequired { .. } => {}
        LoginOutcome::Session(_) => {
            panic!("le SSO contourne la passkey : le 2ᵉ facteur serait optionnel")
        }
    }
}

/// Facturation & bourses : émettre une **facture** et décider une **bourse**
/// notifient les responsables du foyer concerné. Preuve de la répercussion
/// transverse facturation / cantine-bourses → familles.
#[tokio::test]
async fn facture_et_bourse_notifient_les_responsables() {
    let Some((mut ent, cast)) = labo().await else {
        eprintln!("skip scénario: ENT_DATABASE_URL");
        return;
    };
    ent.install_mail_dry_run();
    let etab = cast.etablissement_id;
    let eleve_id = cast.eleve_lea_id;
    let jour = today_utc();

    // 1) Facture émise → mail famille (montant communiqué).
    let (facture, _lignes) = ent
        .creer_facture_for_actor(
            Role::Compta.as_str(),
            CreerFacture {
                etablissement_id: etab,
                foyer_id: cast.foyer_lea_id,
                numero: format!("F-{}", Uuid::new_v4().simple()),
                libelle: "Frais de scolarité".to_string(),
                nature: None,
                annee_scolaire: Some(annee_scolaire_de(jour)),
                lignes: vec![LigneFactureCmd {
                    eleve_id: Some(eleve_id),
                    libelle: "Scolarité".to_string(),
                    quantite: 1,
                    montant_unitaire_centimes: 12_000,
                }],
                actor_user_id: cast.compta.user_id,
            },
        )
        .await
        .expect("création facture");
    ent.emettre_facture_for_actor(etab, facture.id, cast.compta.user_id, Role::Compta.as_str())
        .await
        .expect("émission facture");

    // 1bis) Encaissement → quittance notifiée à la famille (PDF joint).
    let (_enc, _f, quittance) = ent
        .encaisser_facture_for_actor(
            Role::Compta.as_str(),
            EncaisserFacture {
                etablissement_id: etab,
                facture_id: facture.id,
                montant_centimes: facture.total_centimes,
                moyen: MoyenPaiement::Virement,
                reference: Some("LAB".to_string()),
                actor_user_id: cast.compta.user_id,
            },
        )
        .await
        .expect("encaissement facture");

    // 2) Bourse décidée (acceptée) → mail famille.
    let bt = ent
        .list_bourse_types(etab)
        .await
        .expect("types de bourse")
        .into_iter()
        .next()
        .expect("le seed pose au moins un type de bourse");
    let bourse = ent
        .affecter_bourse(AffecterBourse {
            etablissement_id: etab,
            eleve_id,
            bourse_type_id: bt.id,
            echelon: "3".to_string(),
            avec_procuration: false,
            date_debut: jour,
            actor_user_id: Some(cast.administratif.user_id.to_string()),
        })
        .await
        .expect("affectation bourse");
    ent.decider_bourse(DeciderBourse {
        etablissement_id: etab,
        bourse_id: bourse.id,
        accepter: true,
        actor_user_id: Some(cast.administratif.user_id.to_string()),
    })
    .await
    .expect("décision bourse");

    let mails = ent
        .list_mails_recents_for_actor(etab, Role::Administratif.as_str(), 200)
        .await
        .expect("lecture outbox mail");
    assert!(
        mails
            .iter()
            .any(|m| m.kind == "facture" && m.source_id == Some(facture.id)),
        "la facture émise doit être notifiée aux responsables"
    );
    assert!(
        mails
            .iter()
            .any(|m| m.kind == "quittance" && m.source_id == Some(quittance.id)),
        "l'encaissement doit notifier la quittance à la famille"
    );
    assert!(
        mails
            .iter()
            .any(|m| m.kind == "bourse_acceptee" && m.source_id == Some(bourse.id)),
        "la décision de bourse doit être notifiée aux responsables"
    );
}

/// Conseil de classe : une décision **publiée à la famille** (`visible_famille`)
/// est communiquée aux responsables. Une décision non publiée ne l'est pas.
#[tokio::test]
async fn decision_conseil_classe_publiee_notifie_la_famille() {
    let Some((mut ent, cast)) = labo().await else {
        eprintln!("skip scénario: ENT_DATABASE_URL");
        return;
    };
    ent.install_mail_dry_run();
    let etab = cast.etablissement_id;
    let eleve_id = cast.eleve_lea_id;
    let jour = today_utc();

    let conseil = ent
        .creer_conseil_classe_for_actor(
            Role::Direction.as_str(),
            CreerConseilClasse {
                etablissement_id: etab,
                classe: "4B".to_string(),
                periode_libelle: "Trimestre 1".to_string(),
                jour,
                heure: NaiveTime::from_hms_opt(17, 0, 0).expect("heure"),
                lieu: "Salle des conseils".to_string(),
                actor_user_id: cast.direction.user_id,
            },
        )
        .await
        .expect("création conseil de classe");
    ent.tenir_conseil_classe_for_actor(
        etab,
        Role::Direction.as_str(),
        conseil.id,
        cast.direction.user_id,
    )
    .await
    .expect("tenue du conseil");
    let dossier = ent
        .ajouter_dossier_conseil_for_actor(
            Role::Direction.as_str(),
            AjouterDossierConseil {
                etablissement_id: etab,
                conseil_id: conseil.id,
                eleve_id,
                actor_user_id: cast.direction.user_id,
            },
        )
        .await
        .expect("ajout dossier élève");

    // Décision **non publiée** : aucune notification famille attendue.
    let dossier_prive = ent
        .ajouter_dossier_conseil_for_actor(
            Role::Direction.as_str(),
            AjouterDossierConseil {
                etablissement_id: etab,
                conseil_id: conseil.id,
                eleve_id: cast.eleve_tom_id,
                actor_user_id: cast.direction.user_id,
            },
        )
        .await
        .expect("ajout dossier élève (privé)");
    ent.decider_dossier_conseil_for_actor(
        Role::Direction.as_str(),
        DeciderDossierConseil {
            etablissement_id: etab,
            dossier_id: dossier_prive.id,
            appreciation: "Interne".to_string(),
            mention: "aucune".to_string(),
            decision_texte: "Poursuite".to_string(),
            visible_famille: false,
            actor_user_id: cast.direction.user_id,
        },
    )
    .await
    .expect("décision privée");

    // Décision **publiée** : notification famille attendue.
    ent.decider_dossier_conseil_for_actor(
        Role::Direction.as_str(),
        DeciderDossierConseil {
            etablissement_id: etab,
            dossier_id: dossier.id,
            appreciation: "Trimestre sérieux".to_string(),
            mention: "encouragements".to_string(),
            decision_texte: "Poursuite de la scolarité".to_string(),
            visible_famille: true,
            actor_user_id: cast.direction.user_id,
        },
    )
    .await
    .expect("décision publiée");

    let mails = ent
        .list_mails_recents_for_actor(etab, Role::Administratif.as_str(), 200)
        .await
        .expect("lecture outbox mail");
    assert!(
        mails
            .iter()
            .any(|m| m.kind == "conseil_decision" && m.source_id == Some(dossier.id)),
        "une décision de conseil visible famille doit être notifiée"
    );
    assert!(
        !mails
            .iter()
            .any(|m| m.kind == "conseil_decision" && m.source_id == Some(dossier_prive.id)),
        "une décision non publiée ne doit pas être notifiée à la famille"
    );
}

/// Copilote : réponses déterministes sur les données du labo, **cloisonnées par
/// rôle** (le prof ne lit pas les impayés). Vérifie effectif, absences, refus.
#[tokio::test]
async fn copilote_repond_et_respecte_les_roles() {
    let Some((ent, cast)) = labo().await else {
        eprintln!("skip scénario: ENT_DATABASE_URL");
        return;
    };
    let etab = cast.etablissement_id;

    // 1) Effectif total (administratif) — le seed pose 10 élèves.
    let rep = ent
        .copilote_repondre(
            etab,
            cast.administratif.user_id,
            Role::Administratif.as_str(),
            "Combien d'élèves au total ?",
        )
        .await
        .expect("copilote effectif");
    assert_eq!(rep.intention, "effectif");
    assert!(!rep.refusee);
    let total = rep.donnees["effectif"].as_i64().expect("effectif entier");
    assert!(total >= 10, "effectif total: {total}");

    // 2) Effectif 4B — Harry, Hermione, Ginny.
    let rep4b = ent
        .copilote_repondre(
            etab,
            cast.administratif.user_id,
            Role::Administratif.as_str(),
            "combien d'eleves en 4B ?",
        )
        .await
        .expect("copilote 4B");
    assert_eq!(rep4b.donnees["classe"], "4B");
    assert_eq!(rep4b.donnees["effectif"].as_i64(), Some(3));

    // 3) Absences non justifiées 4B sur 30 jours — Harry a une absence seedée.
    let rep_abs = ent
        .copilote_repondre(
            etab,
            cast.cpe.user_id,
            Role::Cpe.as_str(),
            "absences non justifiées de la 4B depuis 30 jours",
        )
        .await
        .expect("copilote absences");
    assert_eq!(rep_abs.intention, "absences_non_justifiees");
    assert!(!rep_abs.refusee);
    assert!(rep_abs.donnees["total"].as_i64().unwrap_or(0) >= 1);

    // 4) Impayés : compta OK, prof refusé (capacité différente).
    let rep_compta = ent
        .copilote_repondre(
            etab,
            cast.compta.user_id,
            Role::Compta.as_str(),
            "Quelles factures sont impayées ?",
        )
        .await
        .expect("copilote impayés compta");
    assert_eq!(rep_compta.intention, "impayes");
    assert!(!rep_compta.refusee);

    let rep_prof = ent
        .copilote_repondre(
            etab,
            cast.prof.user_id,
            Role::Prof.as_str(),
            "Quelles factures sont impayées ?",
        )
        .await
        .expect("copilote impayés prof");
    assert!(rep_prof.refusee, "le prof ne doit pas lire les impayés");

    // 5) Question inconnue → aide + capacités proposées.
    let rep_inconnu = ent
        .copilote_repondre(
            etab,
            cast.administratif.user_id,
            Role::Administratif.as_str(),
            "raconte-moi une histoire",
        )
        .await
        .expect("copilote inconnu");
    assert_eq!(rep_inconnu.intention, "inconnu");
    assert!(rep_inconnu.donnees["capacites"].as_array().is_some());

    // 6) Cloisonnement inter-tenant : une autre établissement ne voit rien.
    let autre_etab = uuid::Uuid::new_v4();
    let rep_vide = ent
        .copilote_repondre(
            autre_etab,
            cast.administratif.user_id,
            Role::Administratif.as_str(),
            "Combien d'élèves au total ?",
        )
        .await
        .expect("copilote autre tenant");
    assert_eq!(rep_vide.donnees["effectif"].as_i64(), Some(0));
}

/// Réinitialisation de mot de passe par jeton : usage unique, révocation des
/// sessions, ancien mot de passe invalide, e-mail inconnu sans fuite.
#[tokio::test]
async fn reset_mot_de_passe_par_jeton_usage_unique() {
    let Some((ent, cast)) = labo().await else {
        eprintln!("skip scénario: ENT_DATABASE_URL");
        return;
    };
    let email = cast.administratif.email.clone();

    // Compte connu → demande « acceptée » (booléen interne), inconnu → false.
    assert!(ent
        .demander_reinitialisation_mot_de_passe(&email, "http://localhost:8088")
        .await
        .expect("demande"));
    assert!(!ent
        .demander_reinitialisation_mot_de_passe("inconnu@nulle.part", "http://localhost:8088")
        .await
        .expect("demande inconnu"));

    // Jeton (helper test) → réinitialisation.
    let token = ent
        .reset_token_for_test(&email)
        .await
        .expect("token")
        .expect("compte existant");

    // Politique : mot de passe trop court → refusé, jeton non consommé.
    assert!(ent
        .reinitialiser_mot_de_passe_par_token(&token, "court")
        .await
        .is_err());

    ent.reinitialiser_mot_de_passe_par_token(&token, "NouveauPass1!2026")
        .await
        .expect("reset");

    // Ancien mot de passe invalide, nouveau valide.
    assert!(ent
        .login(&email, LABO_PASSWORD, cast.etablissement_id)
        .await
        .is_err());
    let session = ent
        .login(&email, "NouveauPass1!2026", cast.etablissement_id)
        .await
        .expect("nouveau mdp");
    assert_eq!(session.role, "administratif");

    // Jeton à usage unique : le rejouer échoue.
    assert!(ent
        .reinitialiser_mot_de_passe_par_token(&token, "EncoreUnPass1!")
        .await
        .is_err());
}

/// Garde par IP : un bourrage réparti sur des comptes distincts finit verrouillé
/// par IP (le seuil par compte n'est jamais atteint), et une autre IP n'est pas
/// pénalisée. Le verrou par IP ne « consomme » pas le compte visé.
#[tokio::test]
async fn garde_login_par_ip() {
    let Some((ent, cast)) = labo().await else {
        eprintln!("skip scénario: ENT_DATABASE_URL");
        return;
    };
    let etab = cast.etablissement_id;
    // IP **et** e-mails uniques par exécution : gardes persistantes en base.
    let uid = uuid::Uuid::new_v4();
    let b = uid.into_bytes();
    let ip_brulee = format!("{}.{}.{}.{}", b[0], b[1], b[2], b[3]);
    let ip_neuve = format!("{}.{}.{}.{}", b[4], b[5], b[6], b[7]);

    // Seuil par IP abaissé pour garder le test rapide (argon2 coûteux) et pour
    // ne pas dépendre du défaut (50). Aucun autre test n'emprunte la voie IP.
    std::env::set_var("ENT_LOGIN_MAX_FAILED_IP", "6");

    // Bourrage : e-mails inconnus distincts → seul le compteur IP monte.
    let mut verrou = false;
    for i in 0..40 {
        let email = format!("brute-{uid}-{i}@ent.test");
        match ent
            .login_with_ip(&email, "MauvaisPass1!", etab, Some(ip_brulee.as_str()))
            .await
        {
            Err(e) if e.to_string().contains("trop de tentatives") => {
                verrou = true;
                break;
            }
            Err(_) => {}
            Ok(_) => panic!("connexion inattendue pour un e-mail inconnu"),
        }
    }
    assert!(verrou, "l'IP doit finir verrouillée après le seuil");

    // Même avec de bons identifiants, l'IP brûlée est refusée…
    let bloque = ent
        .login_with_ip(
            &cast.administratif.email,
            LABO_PASSWORD,
            etab,
            Some(ip_brulee.as_str()),
        )
        .await;
    assert!(bloque.is_err(), "l'IP verrouillée doit être refusée");
    assert!(bloque.unwrap_err().to_string().contains("trop de tentatives"));

    // …mais le compte n'a pas été verrouillé pour autant : depuis une autre IP,
    // la connexion passe.
    let ok = ent
        .login_with_ip(
            &cast.administratif.email,
            LABO_PASSWORD,
            etab,
            Some(ip_neuve.as_str()),
        )
        .await;
    assert!(ok.is_ok(), "une autre IP ne doit pas être pénalisée");
}

/// PRA / sauvegardes : l'inventaire d'intégrité est calculé **par
/// établissement** (jamais de compteur transverse), il détecte une dérive
/// ligne à ligne (preuve de fidélité d'une restauration) et une écriture d'un
/// tenant ne bouge pas l'inventaire d'un autre.
#[tokio::test]
async fn pra_inventaire_integrite_et_isolation() {
    let Some((ent, _cast)) = labo().await else {
        eprintln!("skip scénario: ENT_DATABASE_URL");
        return;
    };

    fn fiche(etab: Uuid, nom: &str, prenom: &str) -> domain_eleve::CreateEleve {
        domain_eleve::CreateEleve {
            etablissement_id: etab,
            nom: nom.into(),
            prenom: prenom.into(),
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
        }
    }

    // Établissements neufs (jamais partagés avec un autre test parallèle).
    let etab = Uuid::new_v4();
    let etab2 = Uuid::new_v4();
    ent.create_eleve(fiche(etab, "Pra", "Reference"))
        .await
        .expect("élève de référence");

    let inv1 = ent
        .inventaire_etablissement(etab)
        .await
        .expect("inventaire établissement");
    assert_eq!(inv1.version, INVENTAIRE_VERSION);
    assert_eq!(inv1.portee, Some(etab));
    assert_eq!(inv1.migration_tag, MIGRATION_TAG);
    assert_eq!(inv1.empreinte_schema.len(), 32, "md5 hexadécimal");
    assert!(
        inv1.nombre_tables() > 20,
        "inventaire trop maigre : {} tables",
        inv1.nombre_tables()
    );
    assert_eq!(inv1.table("ent_eleve").expect("ent_eleve").lignes, 1);
    // Les tables volatiles (sessions, défis, verrous) sont hors inventaire.
    for volatile in TABLES_VOLATILES {
        assert!(
            inv1.table(volatile).is_none(),
            "{volatile} ne doit pas être inventoriée"
        );
    }

    // Aucune écriture → vérification conforme (fidélité d'une restauration).
    let conforme = ent.verifier_inventaire(&inv1).await.expect("vérification");
    assert!(conforme.conforme, "{}", conforme.resume());
    assert_eq!(conforme.tables_conformes, inv1.nombre_tables());
    assert!(conforme.ecarts.is_empty());

    // Une inscription après l'inventaire → dérive détectée sur la bonne table.
    ent.create_eleve(fiche(etab, "Pra", "Derive"))
        .await
        .expect("second élève");
    let derive = ent.verifier_inventaire(&inv1).await.expect("vérification");
    assert!(!derive.conforme, "une dérive doit être détectée");
    let ecart = derive
        .ecarts
        .iter()
        .find(|e| e.nom == "ent_eleve")
        .expect("écart ent_eleve");
    assert_eq!(ecart.lignes_attendues, 1);
    assert_eq!(ecart.lignes_obtenues, 2);
    assert_ne!(ecart.digest_attendu, ecart.digest_obtenu);
    assert!(derive.resume().contains("NON conforme"));

    // Nouvel inventaire → de nouveau conforme.
    let inv2 = ent
        .inventaire_etablissement(etab)
        .await
        .expect("inventaire 2");
    assert!(ent
        .verifier_inventaire(&inv2)
        .await
        .expect("vérification 2")
        .conforme);

    // Cloisonnement : les écritures d'un autre établissement ne changent rien.
    ent.create_eleve(fiche(etab2, "Pra", "Autre"))
        .await
        .expect("élève autre établissement");
    let inv3 = ent
        .inventaire_etablissement(etab)
        .await
        .expect("inventaire 3");
    assert_eq!(inv3.table("ent_eleve"), inv2.table("ent_eleve"));
    assert_ne!(
        ent.inventaire_etablissement(etab2)
            .await
            .expect("inventaire etab2")
            .table("ent_eleve")
            .expect("ent_eleve etab2")
            .digest,
        inv2.table("ent_eleve").expect("ent_eleve").digest
    );

    // Garde par rôle et portée : prof refusé, inventaire d'un autre tenant refusé.
    assert!(ent
        .inventaire_etablissement_pour_acteur(etab, Role::Prof.as_str())
        .await
        .is_err());
    assert!(ent
        .inventaire_etablissement_pour_acteur(etab, Role::Direction.as_str())
        .await
        .is_ok());
    assert!(ent
        .verifier_inventaire_etablissement_pour_acteur(etab, Role::Direction.as_str(), &inv2)
        .await
        .expect("même établissement")
        .conforme);
    match ent
        .verifier_inventaire_etablissement_pour_acteur(etab2, Role::Direction.as_str(), &inv2)
        .await
    {
        Err(EntDbError::InventairePorteeEtrangere) => {}
        other => panic!("un inventaire étranger doit être refusé : {other:?}"),
    }

    // Inventaire global (surface opérateur) : plus large, et sans portée.
    let global = ent.inventaire_global().await.expect("inventaire global");
    assert!(global.portee.is_none());
    assert_eq!(global.migration_tag, MIGRATION_TAG);
    assert!(global.nombre_tables() >= inv2.nombre_tables());
    assert!(
        global.table("ent_user").is_some(),
        "l'inventaire global couvre aussi les comptes (tables sans établissement)"
    );
    assert!(
        global.lignes_total() >= inv2.lignes_total(),
        "le global couvre au moins l'établissement"
    );
}
