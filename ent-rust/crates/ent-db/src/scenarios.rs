//! Scénarios « vie réelle » multi-personas — labo local pour agents / sous-bots.
//!
//! Chaque scénario enchaîne des gestes **par rôle** et vérifie ce que les autres
//! voient (ou ne voient pas). Étendre ici quand notes / EDT / nomenclatures
//! s'enrichissent — le cast `seed_labo_cast` reste l'entrée unique.

use super::*;
use chrono::NaiveDate;
use domain_eleve::{annee_scolaire_de, today_utc, ChangeClasse, DeclareAbsence};
use ent_auth::Role;

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

    // 1) Prof demande Maths — c'est l'administratif qui la crée (nomenclature).
    assert!(PgEnt::role_can_manage_matieres(Role::Administratif.as_str()));
    assert!(!PgEnt::role_can_manage_matieres(Role::Prof.as_str()));
    assert!(!PgEnt::role_can_manage_matieres(Role::Compta.as_str()));

    let maths = ent
        .create_matiere(
            CreateMatiere {
                etablissement_id: cast.etablissement_id,
                code: "MATH".into(),
                libelle: "Mathématiques".into(),
                source_nomenclature: Some("SIECLE:MATH".into()),
            },
            Some(cast.administratif.user_id.to_string()),
        )
        .await
        .expect("admini crée maths");

    let catalogue = ent
        .list_matieres(cast.etablissement_id)
        .await
        .expect("list");
    assert!(catalogue.iter().any(|m| m.id == maths.id));

    // 2) Prof saisit 14/20 pour Léa — compta ne peut pas.
    assert!(PgEnt::role_can_saisir_notes(Role::Prof.as_str()));
    assert!(!PgEnt::role_can_saisir_notes(Role::Compta.as_str()));
    assert!(!PgEnt::role_can_saisir_notes(Role::Parent.as_str()));

    let note = ent
        .saisir_note(SaisirNote {
            etablissement_id: cast.etablissement_id,
            eleve_id: cast.eleve_lea_id,
            matiere_id: maths.id,
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

    assert!(ent
        .montee_rapide_for_actor(
            Role::Administratif.as_str(),
            domain_eleve::MonteeRapide {
                etablissement_id: cast.etablissement_id,
                eleve_id: cast.eleve_lea_id,
                annee_courante: n.clone(),
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
    let acts = ent
        .list_repas_activites(cast.etablissement_id)
        .await
        .expect("acts");
    assert!(acts.len() >= 2);

    assert!(ent
        .eleve_inscrit_midi_present(cast.etablissement_id, cast.eleve_lea_id, jour)
        .await
        .unwrap());
    assert!(!ent
        .eleve_inscrit_midi_present(cast.etablissement_id, cast.eleve_tom_id, jour)
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
    assert!(ent
        .list_matieres(cast_a.etablissement_id)
        .await
        .unwrap()
        .is_empty());
}
