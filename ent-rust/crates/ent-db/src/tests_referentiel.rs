//! Référentiel scolaire contre la base locale de test (feature `labo`).
//! Sans `ENT_DATABASE_URL`, les scénarios sont ignorés.

use super::*;
use chrono::Duration;
use domain_eleve::{today_utc, ChangeClasse, DomainError};
use ent_auth::Role;
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

fn est_conflit(e: &EntDbError) -> bool {
    matches!(e, EntDbError::Domain(DomainError::Conflit))
}

fn est_introuvable(e: &EntDbError) -> bool {
    matches!(e, EntDbError::Domain(DomainError::NotFound))
}

fn est_invalide(e: &EntDbError) -> bool {
    matches!(e, EntDbError::Domain(DomainError::ValeurInvalide))
}

fn eleve_de(cast: &LaboCast, id: Uuid) -> &LaboEleve {
    cast.eleves
        .iter()
        .find(|e| e.id == id)
        .expect("élève du cast")
}

#[tokio::test]
async fn amorcage_idempotent_et_raccordement_a_l_existant() {
    let Some((ent, cast)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = cast.etablissement_id;
    let jour = today_utc();

    // Le seed de labo a déjà amorcé : un second amorçage ne crée plus rien.
    let r = ent.amorcer_referentiel(etab).await.unwrap();
    assert_eq!(
        (r.classes_creees, r.affectations_creees, r.services_crees),
        (0, 0, 0),
        "amorçage idempotent"
    );
    let annee = ent
        .referentiel_annee_courante(etab)
        .await
        .unwrap()
        .expect("année courante");
    assert_eq!(annee.id, r.annee_id);
    assert!(annee.libelle.contains('-') && annee.date_debut < annee.date_fin);

    // Classes issues des textes existants (élèves + EDT).
    let codes = ent.referentiel_codes_classes(etab, annee.id).await.unwrap();
    for c in ["4B", "5A"] {
        assert!(codes.contains(c), "classe {c} attendue, trouvé {codes:?}");
    }
    let classes = ent
        .referentiel_classes(Role::Direction.as_str(), etab, None, jour)
        .await
        .unwrap();
    let c4b = classes.iter().find(|c| c.code == "4B").expect("4B");
    assert_eq!(c4b.niveau.as_deref(), Some("4e"));

    // L'effectif du référentiel suit l'effectif historique (texte) pour 4B.
    let eleves_4b = ent.eleves_de_la_classe(etab, c4b.id, jour).await.unwrap();
    let texte_4b = ent.eleves_en_classe_au(etab, "4B", jour).await.unwrap();
    assert_eq!(c4b.effectif as usize, eleves_4b.len());
    assert!(!eleves_4b.is_empty());
    let ids_ref: std::collections::BTreeSet<_> = eleves_4b.iter().map(|e| e.id).collect();
    let ids_txt: std::collections::BTreeSet<_> = texte_4b.iter().map(|e| e.id).collect();
    assert_eq!(ids_ref, ids_txt, "mêmes élèves que le champ texte");

    // Services depuis l'EDT : Rogue enseigne en 4B, pas en 5A.
    let rogue = cast.profs[0].user_id;
    let mcg = cast.profs[1].user_id;
    let mes = ent.classes_du_prof(etab, rogue, jour).await.unwrap();
    assert!(mes.iter().any(|c| c.code == "4B") && !mes.iter().any(|c| c.code == "5A"));

    // enseigne_a = services ∩ affectations.
    let harry = cast.eleve_lea_id; // 4B
    let draco = cast.eleve_tom_id; // 5A
    assert_eq!(eleve_de(&cast, harry).classe, "4B");
    assert!(ent.enseigne_a(etab, rogue, harry, jour).await.unwrap());
    assert!(!ent.enseigne_a(etab, rogue, draco, jour).await.unwrap());
    assert!(ent.enseigne_a(etab, mcg, draco, jour).await.unwrap());
    // Hors année : aucun service ne s'applique.
    assert!(!ent
        .enseigne_a(etab, rogue, harry, jour + Duration::days(900))
        .await
        .unwrap());
    // Un parent n'enseigne à personne.
    assert!(!ent
        .enseigne_a(etab, cast.parent_lea.user_id, harry, jour)
        .await
        .unwrap());
}

#[tokio::test]
async fn crud_unicite_et_droits() {
    let Some((ent, cast)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = cast.etablissement_id;
    let (adm, prof, cpe, parent) = (
        Role::Administratif.as_str(),
        Role::Prof.as_str(),
        Role::Cpe.as_str(),
        Role::Parent.as_str(),
    );

    // Classe : l'administratif crée ; prof / cpe / parent refusés ; doublon refusé.
    let c = ent
        .referentiel_creer_classe(adm, etab, None, " 6Q ", Some("Sixième A"), None)
        .await
        .unwrap();
    assert_eq!((c.code.as_str(), c.libelle.as_str()), ("6Q", "Sixième A"));
    assert_eq!(c.niveau.as_deref(), Some("6e"));
    for r in [prof, cpe, parent] {
        assert!(
            matches!(
                ent.referentiel_creer_classe(r, etab, None, "6B", None, None)
                    .await,
                Err(EntDbError::Forbidden)
            ),
            "{r}"
        );
    }
    let doublon = ent
        .referentiel_creer_classe(adm, etab, None, "6Q", None, None)
        .await
        .unwrap_err();
    assert!(est_conflit(&doublon), "{doublon:?}");
    assert!(est_invalide(
        &ent.referentiel_creer_classe(adm, etab, None, "   ", None, None)
            .await
            .unwrap_err()
    ));

    // Lecture : tout le personnel, pas la famille.
    assert!(ent
        .referentiel_classes(prof, etab, None, today_utc())
        .await
        .is_ok());
    assert!(matches!(
        ent.referentiel_classes(parent, etab, None, today_utc())
            .await,
        Err(EntDbError::Forbidden)
    ));

    // Années : une seule courante ; dates cohérentes.
    let suivante = ent
        .referentiel_creer_annee(
            adm,
            etab,
            "2099-2100",
            chrono::NaiveDate::from_ymd_opt(2099, 9, 1).unwrap(),
            chrono::NaiveDate::from_ymd_opt(2100, 8, 31).unwrap(),
            false,
        )
        .await
        .unwrap();
    assert!(!suivante.courante);
    assert!(est_invalide(
        &ent.referentiel_creer_annee(
            adm,
            etab,
            "X",
            chrono::NaiveDate::from_ymd_opt(2030, 9, 1).unwrap(),
            chrono::NaiveDate::from_ymd_opt(2030, 9, 1).unwrap(),
            false,
        )
        .await
        .unwrap_err()
    ));
    let annees = ent.referentiel_annees(prof, etab).await.unwrap();
    assert_eq!(annees.iter().filter(|a| a.courante).count(), 1);

    // Groupe : demi-groupe d'une classe.
    let g = ent
        .referentiel_creer_groupe(adm, etab, None, "6Q-G1", None, Some(c.id))
        .await
        .unwrap();
    assert_eq!(g.annee_id, c.annee_id);
    assert!(est_conflit(
        &ent.referentiel_creer_groupe(adm, etab, None, "6Q-G1", None, Some(c.id))
            .await
            .unwrap_err()
    ));

    // Affectation : droits, présence dans la classe, groupe sans doublon.
    let harry = cast.eleve_lea_id;
    let a1 = ent
        .referentiel_affecter_eleve_classe(adm, etab, c.id, harry, None)
        .await
        .unwrap();
    assert_eq!(a1.classe_id, Some(c.id));
    assert!(matches!(
        ent.referentiel_affecter_eleve_classe(prof, etab, c.id, harry, None)
            .await,
        Err(EntDbError::Forbidden)
    ));
    let jour = today_utc();
    assert!(ent
        .eleves_de_la_classe(etab, c.id, jour)
        .await
        .unwrap()
        .iter()
        .any(|e| e.id == harry));
    ent.referentiel_affecter_eleve_groupe(adm, etab, g.id, harry, None)
        .await
        .unwrap();
    assert!(est_conflit(
        &ent.referentiel_affecter_eleve_groupe(adm, etab, g.id, harry, None)
            .await
            .unwrap_err()
    ));

    // Services : un seul professeur principal par classe (le nouveau remplace l'ancien).
    let rogue = cast.profs[0].user_id;
    let mcg = cast.profs[1].user_id;
    let nouveau = |prof_user_id, classe_id, groupe_id, pp| CreerService {
        prof_user_id,
        matiere_id: None,
        classe_id,
        groupe_id,
        professeur_principal: pp,
    };
    let s1 = ent
        .referentiel_creer_service(adm, etab, nouveau(rogue, Some(c.id), None, true))
        .await
        .unwrap();
    assert!(s1.professeur_principal);
    assert!(est_conflit(
        &ent.referentiel_creer_service(adm, etab, nouveau(rogue, Some(c.id), None, false))
            .await
            .unwrap_err()
    ));
    let s2 = ent
        .referentiel_creer_service(adm, etab, nouveau(mcg, Some(c.id), None, true))
        .await
        .unwrap();
    let services = ent
        .referentiel_services_de_classe(etab, c.id)
        .await
        .unwrap();
    assert_eq!(
        services
            .iter()
            .filter(|s| s.professeur_principal)
            .map(|s| s.id)
            .collect::<Vec<_>>(),
        vec![s2.id],
        "le nouveau professeur principal remplace l'ancien"
    );
    // Classe XOR groupe.
    for (classe, groupe) in [(None, None), (Some(c.id), Some(g.id))] {
        let e = ent
            .referentiel_creer_service(adm, etab, nouveau(rogue, classe, groupe, false))
            .await
            .unwrap_err();
        assert!(est_invalide(&e), "classe XOR groupe : {e:?}");
    }
    // Service par groupe : visible depuis la classe parente.
    ent.referentiel_creer_service(adm, etab, nouveau(cast.profs[2].user_id, None, Some(g.id), false))
        .await
        .unwrap();
    assert!(
        ent.referentiel_services_de_classe(etab, c.id)
            .await
            .unwrap()
            .len()
            >= 3
    );
    // Un compte parent n'est pas un enseignant.
    let e = ent
        .referentiel_creer_service(
            adm,
            etab,
            nouveau(cast.parent_lea.user_id, Some(c.id), None, false),
        )
        .await
        .unwrap_err();
    assert!(est_introuvable(&e), "{e:?}");
}

#[tokio::test]
async fn cloisonnement_entre_etablissements() {
    let Some((ent, a)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let b = ent.seed_labo_cast().await.expect("second établissement");
    let (ea, eb) = (a.etablissement_id, b.etablissement_id);
    let jour = today_utc();
    let adm = Role::Administratif.as_str();

    let classes_b = ent.referentiel_classes(adm, eb, None, jour).await.unwrap();
    let c4b_b = classes_b.iter().find(|c| c.code == "4B").expect("4B de B");
    let classes_a = ent.referentiel_classes(adm, ea, None, jour).await.unwrap();
    let c4b_a = classes_a.iter().find(|c| c.code == "4B").expect("4B de A");
    assert_ne!(c4b_a.id, c4b_b.id);
    assert!(!classes_a.iter().any(|c| c.id == c4b_b.id));

    // Lire la classe de B avec l'établissement de A : introuvable.
    assert!(ent
        .referentiel_classe(ea, c4b_b.id)
        .await
        .unwrap()
        .is_none());
    assert!(ent
        .referentiel_classe_resume(adm, ea, c4b_b.id, jour)
        .await
        .unwrap()
        .is_none());
    assert!(ent
        .eleves_de_la_classe(ea, c4b_b.id, jour)
        .await
        .unwrap()
        .is_empty());
    assert!(ent
        .referentiel_services_de_classe(ea, c4b_b.id)
        .await
        .unwrap()
        .is_empty());

    // Affecter un élève de B dans une classe de A, ou l'inverse : introuvable.
    let e = ent
        .referentiel_affecter_eleve_classe(adm, ea, c4b_a.id, b.eleve_lea_id, None)
        .await
        .unwrap_err();
    assert!(est_introuvable(&e), "{e:?}");
    let e = ent
        .referentiel_affecter_eleve_classe(adm, ea, c4b_b.id, a.eleve_lea_id, None)
        .await
        .unwrap_err();
    assert!(est_introuvable(&e), "{e:?}");

    // Service : professeur de B sur une classe de A, ou classe de B pour un prof de A.
    for (classe, prof) in [
        (c4b_a.id, b.profs[0].user_id),
        (c4b_b.id, a.profs[0].user_id),
    ] {
        let e = ent
            .referentiel_creer_service(
                adm,
                ea,
                CreerService {
                    prof_user_id: prof,
                    matiere_id: None,
                    classe_id: Some(classe),
                    groupe_id: None,
                    professeur_principal: false,
                },
            )
            .await
            .unwrap_err();
        assert!(est_introuvable(&e), "{e:?}");
    }

    // Groupe rattaché à la classe d'un autre établissement.
    let e = ent
        .referentiel_creer_groupe(adm, ea, None, "X", None, Some(c4b_b.id))
        .await
        .unwrap_err();
    assert!(est_introuvable(&e), "{e:?}");

    // enseigne_a et classes_du_prof ne traversent jamais la frontière.
    assert!(!ent
        .enseigne_a(ea, b.profs[0].user_id, a.eleve_lea_id, jour)
        .await
        .unwrap());
    assert!(!ent
        .enseigne_a(eb, a.profs[0].user_id, b.eleve_lea_id, jour)
        .await
        .unwrap());
    assert!(!ent
        .enseigne_a(ea, a.profs[0].user_id, b.eleve_lea_id, jour)
        .await
        .unwrap());
    assert!(ent
        .classes_du_prof(ea, b.profs[0].user_id, jour)
        .await
        .unwrap()
        .is_empty());
}

#[tokio::test]
async fn changement_de_classe_met_a_jour_l_affectation() {
    let Some((ent, cast)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = cast.etablissement_id;
    let jour = today_utc();
    let harry = cast.eleve_lea_id; // 4B au départ
    let rogue = cast.profs[0].user_id;
    let mcg = cast.profs[1].user_id;
    assert!(ent.enseigne_a(etab, rogue, harry, jour).await.unwrap());

    ent.change_classe(ChangeClasse {
        etablissement_id: etab,
        eleve_id: harry,
        nouvelle_classe: "5A".into(),
        date_effet: jour,
        actor_user_id: Some(cast.direction.user_id.to_string()),
    })
    .await
    .unwrap();

    let classes = ent
        .referentiel_classes(Role::Direction.as_str(), etab, None, jour)
        .await
        .unwrap();
    let c4b = classes.iter().find(|c| c.code == "4B").unwrap();
    let c5a = classes.iter().find(|c| c.code == "5A").unwrap();
    assert!(ent
        .eleves_de_la_classe(etab, c5a.id, jour)
        .await
        .unwrap()
        .iter()
        .any(|e| e.id == harry));
    assert!(!ent
        .eleves_de_la_classe(etab, c4b.id, jour)
        .await
        .unwrap()
        .iter()
        .any(|e| e.id == harry));
    // Historique conservé : la veille, Harry était encore en 4B.
    assert!(ent
        .eleves_de_la_classe(etab, c4b.id, jour - Duration::days(1))
        .await
        .unwrap()
        .iter()
        .any(|e| e.id == harry));
    // enseigne_a suit : Rogue (4B) n'enseigne plus à Harry, McGonagall (5A) oui.
    assert!(!ent.enseigne_a(etab, rogue, harry, jour).await.unwrap());
    assert!(ent.enseigne_a(etab, mcg, harry, jour).await.unwrap());
    assert!(ent
        .enseigne_a(etab, rogue, harry, jour - Duration::days(1))
        .await
        .unwrap());

    // Classe inconnue : créée à la volée dans l'année ; le texte et le référentiel concordent.
    ent.change_classe(ChangeClasse {
        etablissement_id: etab,
        eleve_id: harry,
        nouvelle_classe: "3Z".into(),
        date_effet: jour,
        actor_user_id: None,
    })
    .await
    .unwrap();
    let c3z = ent
        .referentiel_classes(Role::Direction.as_str(), etab, None, jour)
        .await
        .unwrap()
        .into_iter()
        .find(|c| c.code == "3Z")
        .expect("3Z créée");
    assert_eq!(c3z.effectif, 1);
    assert_eq!(c3z.niveau.as_deref(), Some("3e"));
    // Un amorçage ultérieur ne défait rien et ne signale aucune divergence.
    let r = ent.amorcer_referentiel(etab).await.unwrap();
    assert_eq!(
        (r.classes_creees, r.affectations_creees, r.divergences),
        (0, 0, 0)
    );
}
