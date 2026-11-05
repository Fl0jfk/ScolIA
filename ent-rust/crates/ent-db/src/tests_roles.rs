//! D-017 — rôles paramétrables contre la base locale de test (feature `labo`).
//! Distribution du labo : Rogue (`profs[0]`) enseigne en 4B (Harry), pas en 5A (Draco).

use super::*;
use domain_eleve::{today_utc, CreerTauxTva, DomainError};
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

// ─────────────── R2 (D-017) : domaines de gestion basculés sur le moteur ───────────────

/// Les 13 contrôles `role_can_*` de gestion et l'action qui les porte.
fn controles_r2() -> Vec<(&'static str, Action, fn(&str) -> bool)> {
    vec![
        ("facturation", Action::FacturationGerer, PgEnt::role_can_manage_facturation),
        ("bourses", Action::BoursesGerer, PgEnt::role_can_manage_bourses),
        ("edt", Action::EdtGerer, PgEnt::role_can_manage_edt),
        ("annonces", Action::AnnoncesPublier, PgEnt::role_can_publier_annonces),
        ("repas", Action::RepasGerer, PgEnt::role_can_manage_repas),
        ("scan repas", Action::RepasScanner, PgEnt::role_can_scan_repas),
        ("internat", Action::InternatGerer, PgEnt::role_can_gerer_internat),
        ("sorties", Action::SortiesGerer, PgEnt::role_can_gerer_sorties),
        ("contrôle sortie", Action::SortiesControler, PgEnt::role_can_controle_sortie),
        ("élèves", Action::ElevesGerer, PgEnt::role_can_manage_eleves),
        ("matières", Action::MatieresGerer, PgEnt::role_can_manage_matieres),
        ("conseils", Action::ConseilsGerer, PgEnt::role_can_gerer_conseil_classe),
        ("docs à signer", Action::DocsSignerPublier, PgEnt::role_can_publier_docs_a_signer),
    ]
}

fn ancien(role: Role, action: Action) -> bool {
    match action {
        Action::FacturationGerer | Action::FacturationLire => role.can_manage_facturation(),
        Action::BoursesGerer => role.can_manage_bourses(),
        Action::EdtGerer => role.can_manage_edt(),
        Action::AnnoncesPublier => role.can_publier_annonces(),
        Action::RepasGerer => role.can_manage_repas(),
        Action::RepasScanner => role.can_scan_repas(),
        Action::InternatGerer => role.can_gerer_internat(),
        Action::SortiesGerer => role.can_gerer_sorties(),
        Action::SortiesControler => role.can_controle_sortie(),
        Action::ElevesGerer => role.can_manage_eleves(),
        Action::MatieresGerer => role.can_manage_matieres(),
        Action::ConseilsGerer => role.can_gerer_conseil_classe(),
        Action::DocsSignerPublier => role.can_publier_docs_a_signer(),
        _ => unreachable!(),
    }
}

#[tokio::test]
async fn r2_non_regression_profils_hors_et_dans_le_contexte() {
    let Some((ent, cast)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = cast.etablissement_id;
    let personas = [
        (&cast.admin, Role::Admin),
        (&cast.direction, Role::Direction),
        (&cast.administratif, Role::Administratif),
        (&cast.compta, Role::Compta),
        (&cast.prof, Role::Prof),
        (&cast.cpe, Role::Cpe),
        (&cast.parent_lea, Role::Parent),
    ];
    for (persona, role) in personas {
        let r = role.as_str();
        for (nom, action, f) in controles_r2() {
            // Sans contexte : modèle du profil.
            assert_eq!(f(r), ancien(role, action), "hors contexte {r} × {nom}");
            // Avec le contexte réel de la personne (aucun rôle attribué) : identique.
            let dans = ent
                .avec_acteur(etab, persona.user_id, r, async { f(r) })
                .await
                .unwrap();
            assert_eq!(dans, ancien(role, action), "dans le contexte {r} × {nom}");
        }
        assert_eq!(
            PgEnt::role_can_lire_facturation(r),
            role.can_manage_facturation(),
            "lecture facturation {r}"
        );
    }
}

#[tokio::test]
async fn r2_role_accueil_obtient_exactement_ses_droits() {
    let Some((ent, cast)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = cast.etablissement_id;
    let dir = (cast.direction.user_id, cast.direction.role.as_str());
    let (user, base) = (cast.compta.user_id, Role::Compta.as_str());

    let annonce = |titre: &str| CreerAnnonce {
        etablissement_id: etab,
        titre: titre.into(),
        corps: "Fermeture exceptionnelle.".into(),
        audience: "parents".into(),
        cible_classe: None,
        actor_user_id: user,
    };
    // Avant : la comptable ne publie pas et ne contrôle pas les sorties.
    assert!(matches!(
        ent.publier_annonce_for_actor(base, annonce("Avant")).await,
        Err(EntDbError::Forbidden)
    ));

    let accueil = ent
        .creer_role_for_actor(
            etab,
            dir.0,
            dir.1,
            saisie(
                "Accueil",
                vec![
                    droit(Action::SortiesControler, Portee::Etablissement),
                    droit(Action::AnnoncesPublier, Portee::Etablissement),
                ],
            ),
        )
        .await
        .unwrap();
    ent.attribuer_role_for_actor(etab, dir.0, dir.1, accueil.id, user, None)
        .await
        .unwrap();

    let jour = today_utc();
    ent.avec_acteur(etab, user, base, async {
        // Exactement ces deux droits…
        assert!(PgEnt::role_can_controle_sortie(base));
        assert!(PgEnt::role_can_publier_annonces(base));
        ent.publier_annonce_for_actor(base, annonce("Fermeture")).await.unwrap();
        ent.list_controles_sortie_jour_for_actor(etab, base, jour).await.unwrap();
        // … et pas un de plus (sorties.gerer, EDT, élèves, docs à signer, internat).
        assert!(!PgEnt::role_can_gerer_sorties(base));
        assert!(!PgEnt::role_can_manage_edt(base));
        assert!(!PgEnt::role_can_manage_eleves(base));
        assert!(!PgEnt::role_can_publier_docs_a_signer(base));
        assert!(!PgEnt::role_can_gerer_internat(base));
        // Ce que le profil Compta donne déjà reste acquis.
        assert!(PgEnt::role_can_manage_facturation(base));
        assert!(matches!(
            ent.list_sortie_regles_niveau_for_actor(etab, base).await,
            Err(EntDbError::Forbidden)
        ));
    })
    .await
    .unwrap();

    // Le contexte d'un autre profil n'est jamais appliqué à un profil différent.
    ent.avec_acteur(etab, user, base, async {
        assert!(!PgEnt::role_can_publier_annonces(Role::Prof.as_str()));
    })
    .await
    .unwrap();

    // Retrait : effet immédiat à la requête suivante.
    ent.retirer_role_for_actor(etab, dir.0, dir.1, accueil.id, user).await.unwrap();
    ent.avec_acteur(etab, user, base, async {
        assert!(!PgEnt::role_can_controle_sortie(base));
        assert!(!PgEnt::role_can_publier_annonces(base));
    })
    .await
    .unwrap();
    ent.supprimer_role_for_actor(etab, dir.0, dir.1, accueil.id).await.unwrap();
}

#[tokio::test]
async fn r2_comptable_adjoint_lit_sans_emettre() {
    let Some((ent, cast)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = cast.etablissement_id;
    let dir = (cast.direction.user_id, cast.direction.role.as_str());
    // La CPE n'a aucun droit de facturation par son profil.
    let (user, base) = (cast.cpe.user_id, Role::Cpe.as_str());
    let tva = |code: &str| CreerTauxTva {
        etablissement_id: etab,
        code: format!("{code}{}", &Uuid::new_v4().simple().to_string()[..4]),
        libelle: "Test".into(),
        taux_bps: 550,
        actor_user_id: user,
    };
    ent.avec_acteur(etab, user, base, async {
        assert!(matches!(
            ent.list_rappels_for_actor(etab, base).await,
            Err(EntDbError::Forbidden)
        ));
    })
    .await
    .unwrap();

    let lecteur = ent
        .creer_role_for_actor(
            etab,
            dir.0,
            dir.1,
            saisie("Comptable adjoint", vec![droit(Action::FacturationLire, Portee::Etablissement)]),
        )
        .await
        .unwrap();
    ent.attribuer_role_for_actor(etab, dir.0, dir.1, lecteur.id, user, None).await.unwrap();
    ent.avec_acteur(etab, user, base, async {
        // Lit…
        ent.list_rappels_for_actor(etab, base).await.unwrap();
        ent.list_factures_for_actor(etab, user, base).await.unwrap();
        // … n'émet ni ne paramètre rien.
        assert!(!PgEnt::role_can_manage_facturation(base));
        assert!(matches!(
            ent.creer_taux_tva_for_actor(base, tva("A")).await,
            Err(EntDbError::Forbidden)
        ));
    })
    .await
    .unwrap();
    ent.retirer_role_for_actor(etab, dir.0, dir.1, lecteur.id, user).await.unwrap();
    ent.supprimer_role_for_actor(etab, dir.0, dir.1, lecteur.id).await.unwrap();

    // `facturation.gerer` inclut la lecture.
    let gestion = ent
        .creer_role_for_actor(
            etab,
            dir.0,
            dir.1,
            saisie("Facturation", vec![droit(Action::FacturationGerer, Portee::Etablissement)]),
        )
        .await
        .unwrap();
    ent.attribuer_role_for_actor(etab, dir.0, dir.1, gestion.id, user, None).await.unwrap();
    ent.avec_acteur(etab, user, base, async {
        ent.list_rappels_for_actor(etab, base).await.unwrap();
        ent.creer_taux_tva_for_actor(base, tva("B")).await.unwrap();
    })
    .await
    .unwrap();
    ent.supprimer_role_for_actor(etab, dir.0, dir.1, gestion.id).await.unwrap();
}

#[tokio::test]
async fn r2_garde_fous_portee_et_escalade() {
    let Some((ent, cast)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = cast.etablissement_id;
    let dir = (cast.direction.user_id, cast.direction.role.as_str());
    let cpe = (cast.cpe.user_id, cast.cpe.role.as_str());

    // Les actions de gestion n'acceptent que l'établissement entier.
    for a in [Action::FacturationGerer, Action::EdtGerer, Action::ElevesGerer] {
        let r = ent
            .creer_role_for_actor(etab, dir.0, dir.1, saisie("x", vec![droit(a, Portee::MesClasses)]))
            .await;
        assert!(matches!(r, Err(EntDbError::Domain(DomainError::ValeurInvalide))), "{a:?} : {r:?}");
    }
    // La CPE reçoit la gestion des rôles, mais ne peut donner que ce qu'elle a.
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
    let ok = ent
        .creer_role_for_actor(
            etab,
            cpe.0,
            cpe.1,
            saisie("Sorties", vec![droit(Action::SortiesGerer, Portee::Etablissement)]),
        )
        .await
        .unwrap();
    for a in [Action::FacturationGerer, Action::FacturationLire, Action::BoursesGerer, Action::ElevesGerer] {
        let r = ent
            .creer_role_for_actor(etab, cpe.0, cpe.1, saisie("Escalade", vec![droit(a, Portee::Etablissement)]))
            .await;
        assert!(matches!(r, Err(EntDbError::Forbidden)), "{a:?} : {r:?}");
    }
    for id in [ok.id, gestion.id] {
        ent.supprimer_role_for_actor(etab, dir.0, dir.1, id).await.unwrap();
    }
}
