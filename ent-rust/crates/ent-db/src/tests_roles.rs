//! D-017 — rôles paramétrables contre la base locale de test (feature `labo`).
//! Distribution du labo : Rogue (`profs[0]`) enseigne en 4B (Harry), pas en 5A (Draco).

use super::*;
use domain_eleve::{today_utc, DomainError};
use ent_auth::{Action, Portee, Role};
use uuid::Uuid;

async fn labo() -> Option<(PgEnt, LaboCast)> {
    let url = std::env::var("ENT_DATABASE_URL").ok()?;
    assert_local_postgres_url(&url).expect("ENT_DATABASE_URL doit être locale");
    let mut ent = PgEnt::connect(&url).await.expect("connect");
    ent.migrate().await.expect("migrate");
    ent.install_brain_p0();
    let cast = ent.seed_labo_cast().await.expect("seed cast");
    Some((ent, cast))
}

fn droit(action: Action, portee: Portee) -> Droit {
    Droit { action, portee, cibles: vec![] }
}

fn saisie(nom: &str, droits: Vec<Droit>) -> RoleSaisie {
    RoleSaisie {
        nom: format!("{nom} {}", &Uuid::new_v4().simple().to_string()[..6]),
        description: None,
        modele: None,
        droits,
    }
}

#[tokio::test]
async fn roles_professeur_de_theatre_voit_son_atelier_sans_les_notes() {
    let Some((ent, cast)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = cast.etablissement_id;
    let dir = (cast.direction.user_id, cast.direction.role.as_str());
    let rogue = cast.profs[0].user_id;
    let (harry, draco) = (cast.eleve_lea_id, cast.eleve_tom_id);
    let prof = Role::Prof.as_str();

    // L'établissement crée l'atelier Théâtre, y inscrit Draco, nomme Rogue responsable.
    let code = format!("THEA-{}", &Uuid::new_v4().simple().to_string()[..6]);
    let g = ent
        .referentiel_creer_groupe(Role::Administratif.as_str(), etab, None, &code, Some("Atelier théâtre"), None)
        .await
        .unwrap();
    ent.referentiel_affecter_eleve_groupe(Role::Administratif.as_str(), etab, g.id, draco, Some(today_utc()))
        .await
        .unwrap();
    ent.definir_responsables_groupe_for_actor(etab, dir.0, dir.1, g.id, &[rogue])
        .await
        .unwrap();

    // Avant le rôle : Rogue ne voit pas Draco (hors de ses classes).
    assert!(!ent.actor_can_read_absences(etab, rogue, prof, draco).await.unwrap());

    let role = ent
        .creer_role_for_actor(
            etab,
            dir.0,
            dir.1,
            saisie(
                "Professeur de théâtre",
                vec![
                    droit(Action::AbsencesLire, Portee::MesGroupes),
                    droit(Action::DossierLire, Portee::MesGroupes),
                    droit(Action::MessagerieEleve, Portee::MesGroupes),
                ],
            ),
        )
        .await
        .unwrap();
    ent.attribuer_role_for_actor(etab, dir.0, dir.1, role.id, rogue, None)
        .await
        .unwrap();

    // Cumul : ses classes (profil) + son atelier (rôle), sans les notes de l'atelier.
    assert!(ent.actor_can_read_absences(etab, rogue, prof, draco).await.unwrap());
    assert!(ent.actor_can_access_msg_fil(etab, rogue, prof, draco).await.unwrap());
    assert!(!ent.actor_can_read_notes(etab, rogue, prof, draco).await.unwrap());
    assert!(ent.actor_can_read_notes(etab, rogue, prof, harry).await.unwrap());
    let vs = ent
        .eleves_autorises(etab, rogue, prof, Action::AbsencesLire)
        .await
        .unwrap()
        .expect("ensemble");
    assert!(vs.contains(&draco) && vs.contains(&harry));

    // Retrait : effet immédiat.
    assert!(ent
        .retirer_role_for_actor(etab, dir.0, dir.1, role.id, rogue)
        .await
        .unwrap());
    assert!(!ent.actor_can_read_absences(etab, rogue, prof, draco).await.unwrap());
    ent.supprimer_role_for_actor(etab, dir.0, dir.1, role.id).await.unwrap();
}

#[tokio::test]
async fn roles_garde_fous_escalade_familles_et_cibles() {
    let Some((ent, cast)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = cast.etablissement_id;
    let dir = (cast.direction.user_id, cast.direction.role.as_str());
    let adm = (cast.administratif.user_id, cast.administratif.role.as_str());
    let cpe = (cast.cpe.user_id, cast.cpe.role.as_str());

    // Sans « roles.gerer » : refusé.
    let r = ent
        .creer_role_for_actor(etab, adm.0, adm.1, saisie("Accueil", vec![droit(Action::AbsencesLire, Portee::Etablissement)]))
        .await;
    assert!(matches!(r, Err(EntDbError::Forbidden)), "{r:?}");

    // Portée interdite pour une action d'administration.
    let r = ent
        .creer_role_for_actor(etab, dir.0, dir.1, saisie("x", vec![droit(Action::RolesGerer, Portee::MesClasses)]))
        .await;
    assert!(matches!(r, Err(EntDbError::Domain(DomainError::ValeurInvalide))), "{r:?}");

    // Délégation : la direction donne à la CPE la gestion des rôles + l'absentéisme.
    let gestion = ent
        .creer_role_for_actor(
            etab,
            dir.0,
            dir.1,
            saisie("Gestion VS", vec![droit(Action::RolesGerer, Portee::Etablissement)]),
        )
        .await
        .unwrap();
    ent.attribuer_role_for_actor(etab, dir.0, dir.1, gestion.id, cpe.0, None).await.unwrap();
    // La CPE crée un rôle qu'elle couvre (absences) : accepté…
    let ok = ent
        .creer_role_for_actor(etab, cpe.0, cpe.1, saisie("Accueil", vec![droit(Action::AbsencesLire, Portee::Etablissement)]))
        .await
        .unwrap();
    // … mais pas un rôle donnant la politique de sécurité ou la saisie de notes (escalade).
    for a in [Action::SecuritePolitique, Action::NotesSaisir] {
        let r = ent
            .creer_role_for_actor(etab, cpe.0, cpe.1, saisie("Escalade", vec![droit(a, Portee::Etablissement)]))
            .await;
        assert!(matches!(r, Err(EntDbError::Forbidden)), "{a:?} : {r:?}");
    }
    // Ni s'attribuer un rôle qu'elle ne couvre pas.
    let sup = ent
        .creer_role_for_actor(etab, dir.0, dir.1, saisie("Notes", vec![droit(Action::NotesSaisir, Portee::Etablissement)]))
        .await
        .unwrap();
    assert!(matches!(
        ent.attribuer_role_for_actor(etab, cpe.0, cpe.1, sup.id, cpe.0, None).await,
        Err(EntDbError::Forbidden)
    ));

    // Familles et élèves : jamais de rôle libre.
    assert!(matches!(
        ent.attribuer_role_for_actor(etab, dir.0, dir.1, ok.id, cast.parent_lea.user_id, None).await,
        Err(EntDbError::Domain(DomainError::ValeurInvalide))
    ));
    // Cible désignée d'un autre établissement (ou inconnue) : refusée.
    let r = ent
        .creer_role_for_actor(
            etab,
            dir.0,
            dir.1,
            saisie("Désigné", vec![Droit { action: Action::NotesLire, portee: Portee::Designes, cibles: vec![Uuid::new_v4()] }]),
        )
        .await;
    assert!(matches!(r, Err(EntDbError::Domain(DomainError::ValeurInvalide))), "{r:?}");
    // Un autre établissement ne voit pas ces rôles.
    let autre = Uuid::new_v4();
    assert!(ent.list_roles_for_actor(autre, dir.0, dir.1).await.unwrap().is_empty());

    for id in [ok.id, sup.id, gestion.id] {
        ent.supprimer_role_for_actor(etab, dir.0, dir.1, id).await.unwrap();
    }
}

#[tokio::test]
async fn roles_portee_designee_sur_une_classe() {
    let Some((ent, cast)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = cast.etablissement_id;
    let dir = (cast.direction.user_id, cast.direction.role.as_str());
    let compta = (cast.compta.user_id, cast.compta.role.as_str());
    let (harry, draco) = (cast.eleve_lea_id, cast.eleve_tom_id);
    let classes = ent
        .referentiel_classes(Role::Direction.as_str(), etab, None, today_utc())
        .await
        .unwrap();
    let c5a = classes.iter().find(|c| c.code == "5A").expect("5A").id;
    assert!(!ent.actor_can_read_notes(etab, compta.0, compta.1, draco).await.unwrap());
    let role = ent
        .creer_role_for_actor(
            etab,
            dir.0,
            dir.1,
            saisie("Notes 5A", vec![Droit { action: Action::NotesLire, portee: Portee::Designes, cibles: vec![c5a] }]),
        )
        .await
        .unwrap();
    ent.attribuer_role_for_actor(etab, dir.0, dir.1, role.id, compta.0, None).await.unwrap();
    assert!(ent.actor_can_read_notes(etab, compta.0, compta.1, draco).await.unwrap(), "5A désignée");
    assert!(!ent.actor_can_read_notes(etab, compta.0, compta.1, harry).await.unwrap(), "4B non désignée");
    ent.supprimer_role_for_actor(etab, dir.0, dir.1, role.id).await.unwrap();
    assert!(!ent.actor_can_read_notes(etab, compta.0, compta.1, draco).await.unwrap());
}

// ─────────────────────── R2 : domaines sensibles ───────────────────────

async fn passage(ent: &PgEnt, etab: Uuid, par: &LaboPersona, eleve: Uuid) {
    ent.creer_passage_infirmerie_for_actor(
        par.role.as_str(),
        domain_eleve::CreerPassageInfirmerie {
            etablissement_id: etab,
            eleve_id: eleve,
            jour: today_utc(),
            motif: domain_eleve::PassageInfirmerieMotif::Soins,
            note_soins: "r2".into(),
            visible_famille: false,
            actor_user_id: par.user_id,
        },
    )
    .await
    .expect("passage");
}

#[tokio::test]
async fn r2_infirmiere_vacataire_cantonnee_a_une_classe() {
    let Some((ent, cast)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = cast.etablissement_id;
    let dir = (cast.direction.user_id, cast.direction.role.as_str());
    let inf = &cast.infirmieres[0];
    // « Vacataire » : un personnel sans droit santé dans son profil de base.
    let vac = (cast.compta.user_id, cast.compta.role.as_str());
    let (harry, draco) = (cast.eleve_lea_id, cast.eleve_tom_id);
    passage(&ent, etab, inf, harry).await;
    passage(&ent, etab, inf, draco).await;

    // Sans le rôle : refusé partout, y compris les listes.
    assert!(matches!(
        ent.get_fiche_sante_for_actor(etab, vac.0, vac.1, draco).await,
        Err(EntDbError::Forbidden)
    ));
    assert!(matches!(
        ent.list_passages_infirmerie_for_actor(etab, vac.0, vac.1, None, None).await,
        Err(EntDbError::Forbidden)
    ));

    let classes = ent
        .referentiel_classes(Role::Direction.as_str(), etab, None, today_utc())
        .await
        .unwrap();
    let c5a = classes.iter().find(|c| c.code == "5A").expect("5A").id;
    let role = ent
        .creer_role_for_actor(
            etab,
            dir.0,
            dir.1,
            saisie(
                "Infirmière vacataire",
                vec![Droit { action: Action::SanteLire, portee: Portee::Designes, cibles: vec![c5a] }],
            ),
        )
        .await
        .unwrap();
    ent.attribuer_role_for_actor(etab, dir.0, dir.1, role.id, vac.0, None).await.unwrap();

    // Lecture : seulement la classe désignée (5A = Draco), pas 4B (Harry).
    assert!(ent.get_fiche_sante_for_actor(etab, vac.0, vac.1, draco).await.is_ok());
    assert!(matches!(
        ent.get_fiche_sante_for_actor(etab, vac.0, vac.1, harry).await,
        Err(EntDbError::Forbidden)
    ));
    assert!(ent.get_pai_for_actor(etab, vac.0, vac.1, draco).await.is_ok());
    assert!(matches!(
        ent.get_pai_for_actor(etab, vac.0, vac.1, harry).await,
        Err(EntDbError::Forbidden)
    ));
    assert!(matches!(
        ent.list_passages_infirmerie_for_actor(etab, vac.0, vac.1, Some(harry), None).await,
        Err(EntDbError::Forbidden)
    ));
    // Liste du jour : filtrée sur la portée.
    let jour = ent
        .list_passages_infirmerie_for_actor(etab, vac.0, vac.1, None, None)
        .await
        .unwrap();
    assert!(!jour.is_empty());
    assert!(jour.iter().all(|p| p.eleve_id == draco), "fuite hors portée");
    // Lire n'est pas saisir : la saisie reste refusée.
    let r = ent
        .creer_passage_infirmerie_for_actor(
            vac.1,
            domain_eleve::CreerPassageInfirmerie {
                etablissement_id: etab,
                eleve_id: draco,
                jour: today_utc(),
                motif: domain_eleve::PassageInfirmerieMotif::Autre,
                note_soins: String::new(),
                visible_famille: false,
                actor_user_id: vac.0,
            },
        )
        .await;
    assert!(matches!(r, Err(EntDbError::Forbidden)), "{r:?}");
    // La consultation autorisée est tracée (acteur réel).
    let (n,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM ent_journal_modif
          WHERE etablissement_id = $1 AND actor_user_id = $2
            AND domaine = 'consultation' AND action = 'fiche_sante'",
    )
    .bind(etab)
    .bind(vac.0)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    assert_eq!(n, 1);

    // Retrait : effet immédiat.
    ent.retirer_role_for_actor(etab, dir.0, dir.1, role.id, vac.0).await.unwrap();
    assert!(matches!(
        ent.get_fiche_sante_for_actor(etab, vac.0, vac.1, draco).await,
        Err(EntDbError::Forbidden)
    ));
    ent.supprimer_role_for_actor(etab, dir.0, dir.1, role.id).await.unwrap();
}

#[tokio::test]
async fn r2_psy_sanctions_documents_audit_sans_role_refuses_puis_ouverts() {
    let Some((ent, cast)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = cast.etablissement_id;
    let dir = (cast.direction.user_id, cast.direction.role.as_str());
    let cpe = (cast.cpe.user_id, cast.cpe.role.as_str());
    let compta = (cast.compta.user_id, cast.compta.role.as_str());
    let draco = cast.eleve_tom_id;

    // Profils historiques inchangés (aucun rôle paramétré) : la CPE n'a ni psy,
    // ni documents ; la compta n'a ni sanctions, ni audit.
    assert!(matches!(
        ent.get_dossier_psy_for_actor(etab, cpe.0, cpe.1, draco).await,
        Err(EntDbError::Forbidden)
    ));
    assert!(matches!(
        ent.list_grants_pending_pour_acteur(etab, cpe.0, cpe.1).await,
        Err(EntDbError::Forbidden)
    ));
    assert!(matches!(
        ent.list_commissions_pour_acteur(etab, compta.0, compta.1, today_utc()).await,
        Err(EntDbError::Forbidden)
    ));
    assert!(matches!(
        ent.list_journal_domaine_for_actor(etab, compta.0, compta.1, "consultation", 10).await,
        Err(EntDbError::Forbidden)
    ));
    // … et la CPE garde ses sanctions, la direction son audit.
    assert!(ent
        .list_commissions_pour_acteur(etab, cpe.0, cpe.1, today_utc())
        .await
        .is_ok());
    assert!(ent
        .list_journal_domaine_for_actor(etab, dir.0, dir.1, "consultation", 10)
        .await
        .is_ok());

    // Rôle paramétré : psy.lire (établissement), sanctions, audit, documents.
    let role = ent
        .creer_role_for_actor(
            etab,
            dir.0,
            dir.1,
            saisie(
                "Référent sensible",
                vec![
                    droit(Action::PsyLire, Portee::Etablissement),
                    droit(Action::SanctionsPrononcer, Portee::Etablissement),
                    droit(Action::AuditLire, Portee::Etablissement),
                    droit(Action::DocumentsGererDroits, Portee::Etablissement),
                ],
            ),
        )
        .await
        .unwrap();
    ent.attribuer_role_for_actor(etab, dir.0, dir.1, role.id, compta.0, None).await.unwrap();
    assert!(ent.get_dossier_psy_for_actor(etab, compta.0, compta.1, draco).await.is_ok());
    assert!(ent
        .list_commissions_pour_acteur(etab, compta.0, compta.1, today_utc())
        .await
        .is_ok());
    assert!(ent
        .list_journal_domaine_for_actor(etab, compta.0, compta.1, "consultation", 10)
        .await
        .is_ok());
    assert!(ent.list_grants_pending_pour_acteur(etab, compta.0, compta.1).await.is_ok());
    // psy.lire n'ouvre pas la saisie psy.
    let r = ent
        .upsert_dossier_psy_for_actor(
            compta.1,
            domain_eleve::UpsertDossierPsy {
                etablissement_id: etab,
                eleve_id: draco,
                suivi_actif: true,
                axes_suivi: String::new(),
                contacts_externes: String::new(),
                note_confidentielle: String::new(),
                signale_famille: false,
                actor_user_id: compta.0,
            },
        )
        .await;
    assert!(matches!(r, Err(EntDbError::Forbidden)), "{r:?}");
    ent.retirer_role_for_actor(etab, dir.0, dir.1, role.id, compta.0).await.unwrap();
    assert!(matches!(
        ent.get_dossier_psy_for_actor(etab, compta.0, compta.1, draco).await,
        Err(EntDbError::Forbidden)
    ));

    // Anti-escalade : la CPE (roles.gerer délégué) ne peut pas donner santé/psy/audit.
    let gestion = ent
        .creer_role_for_actor(etab, dir.0, dir.1, saisie("Gestion R2", vec![droit(Action::RolesGerer, Portee::Etablissement)]))
        .await
        .unwrap();
    ent.attribuer_role_for_actor(etab, dir.0, dir.1, gestion.id, cpe.0, None).await.unwrap();
    for a in [Action::SanteLire, Action::SanteSaisir, Action::PsyLire, Action::PsySaisir, Action::AuditLire, Action::DocumentsLire] {
        let r = ent
            .creer_role_for_actor(etab, cpe.0, cpe.1, saisie("Escalade R2", vec![droit(a, Portee::Etablissement)]))
            .await;
        assert!(matches!(r, Err(EntDbError::Forbidden)), "{a:?} : {r:?}");
    }
    // Elle peut en revanche déléguer ce qu'elle a (sanctions).
    let ok = ent
        .creer_role_for_actor(etab, cpe.0, cpe.1, saisie("Sanctions déléguées", vec![droit(Action::SanctionsPrononcer, Portee::Etablissement)]))
        .await
        .unwrap();
    for id in [ok.id, gestion.id, role.id] {
        ent.supprimer_role_for_actor(etab, dir.0, dir.1, id).await.unwrap();
    }
}
