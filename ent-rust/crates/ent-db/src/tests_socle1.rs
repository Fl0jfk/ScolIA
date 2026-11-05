//! Socle 1 — périmètre des professeurs (B1, B2, B3) contre la base locale de test
//! (feature `labo`). Sans `ENT_DATABASE_URL`, les scénarios sont ignorés.
//!
//! Distribution du labo : Rogue (`profs[0]`) enseigne en 4B (Harry = `eleve_lea_id`),
//! pas en 5A (Draco = `eleve_tom_id`) ; McGonagall (`profs[1]`) enseigne en 5A.

use super::*;
use domain_eleve::{today_utc, OuvrirFil};
use ent_auth::Role;

async fn labo() -> Option<(PgEnt, LaboCast)> {
    let url = std::env::var("ENT_DATABASE_URL").ok()?;
    assert_local_postgres_url(&url).expect("ENT_DATABASE_URL doit être locale");
    let mut ent = PgEnt::connect(&url).await.expect("connect");
    ent.migrate().await.expect("migrate");
    ent.install_brain_p0();
    let cast = ent.seed_labo_cast().await.expect("seed cast");
    Some((ent, cast))
}

#[tokio::test]
async fn socle1_prof_ne_lit_que_ses_eleves() {
    let Some((ent, cast)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = cast.etablissement_id;
    let (rogue, mcg) = (cast.profs[0].user_id, cast.profs[1].user_id);
    let (harry, draco) = (cast.eleve_lea_id, cast.eleve_tom_id);
    let prof = Role::Prof.as_str();

    assert!(ent.prof_suit_eleve(etab, rogue, harry).await.unwrap());
    assert!(!ent.prof_suit_eleve(etab, rogue, draco).await.unwrap());
    assert!(ent.prof_suit_eleve(etab, mcg, draco).await.unwrap());
    let suivis = ent
        .eleves_suivis_par_prof(etab, rogue, today_utc())
        .await
        .unwrap();
    assert!(suivis.contains(&harry) && !suivis.contains(&draco));

    // Notes et vie scolaire : ses élèves seulement.
    assert!(ent.actor_can_read_notes(etab, rogue, prof, harry).await.unwrap());
    assert!(!ent.actor_can_read_notes(etab, rogue, prof, draco).await.unwrap());
    assert!(ent.actor_can_read_absences(etab, rogue, prof, harry).await.unwrap());
    assert!(!ent.actor_can_read_absences(etab, rogue, prof, draco).await.unwrap());
    // Journal de l'élève.
    assert!(ent
        .list_journal_eleve_for_actor(etab, rogue, prof, harry, 10)
        .await
        .is_ok());
    assert!(matches!(
        ent.list_journal_eleve_for_actor(etab, rogue, prof, draco, 10).await,
        Err(EntDbError::Forbidden)
    ));
    // Les autres personnels gardent le périmètre de l'établissement.
    for p in [&cast.cpe, &cast.direction, &cast.administratif] {
        let r = p.role.as_str();
        assert!(ent.actor_can_read_notes(etab, p.user_id, r, draco).await.unwrap(), "{r}");
        assert!(ent.actor_can_read_absences(etab, p.user_id, r, draco).await.unwrap(), "{r}");
    }
}

#[tokio::test]
async fn socle1_prof_ne_note_pas_un_eleve_hors_de_ses_classes() {
    let Some((ent, cast)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = cast.etablissement_id;
    let rogue = cast.profs[0].user_id;
    let r = ent
        .saisir_note_for_actor(
            Role::Prof.as_str(),
            SaisirNote {
                etablissement_id: etab,
                eleve_id: cast.eleve_tom_id,
                matiere_id: Uuid::new_v4(),
                valeur: 12.0,
                sur: 20.0,
                date_note: today_utc(),
                auteur_user_id: rogue,
                libelle: None,
                devoir_id: None,
                periode_id: None,
            },
        )
        .await;
    assert!(matches!(r, Err(EntDbError::Forbidden)), "{r:?}");
    // Un rôle sans droit de saisie est refusé d'emblée.
    let r = ent
        .saisir_note_for_actor(
            Role::Parent.as_str(),
            SaisirNote {
                etablissement_id: etab,
                eleve_id: cast.eleve_lea_id,
                matiere_id: Uuid::new_v4(),
                valeur: 20.0,
                sur: 20.0,
                date_note: today_utc(),
                auteur_user_id: cast.parent_lea.user_id,
                libelle: None,
                devoir_id: None,
                periode_id: None,
            },
        )
        .await;
    assert!(matches!(r, Err(EntDbError::Forbidden)));
}

#[tokio::test]
async fn socle1_justification_reservee_a_la_vie_scolaire_et_aux_responsables() {
    let Some((ent, cast)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = cast.etablissement_id;
    let harry = cast.eleve_lea_id;
    let rogue = cast.profs[0].user_id;
    assert!(!ent
        .actor_can_justifier_absence(etab, rogue, Role::Prof.as_str(), harry)
        .await
        .unwrap(), "un professeur, même de l'élève, ne justifie pas");
    assert!(ent
        .actor_can_justifier_absence(etab, cast.cpe.user_id, Role::Cpe.as_str(), harry)
        .await
        .unwrap());
}

#[tokio::test]
async fn socle1_messagerie_prof_limitee_a_ses_eleves_et_a_ses_fils() {
    let Some((ent, cast)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = cast.etablissement_id;
    let (rogue, mcg) = (cast.profs[0].user_id, cast.profs[1].user_id);
    let (harry, draco) = (cast.eleve_lea_id, cast.eleve_tom_id);
    let prof = Role::Prof.as_str();

    // La vie scolaire ouvre un fil à propos de Draco (5A).
    let (fil_draco, _) = ent
        .ouvrir_fil_for_actor(
            Role::Cpe.as_str(),
            OuvrirFil {
                etablissement_id: etab,
                eleve_id: draco,
                sujet: "Socle 1 — fil confidentiel".into(),
                premier_message: "Échange avec la famille.".into(),
                actor_user_id: cast.cpe.user_id,
            },
        )
        .await
        .unwrap();

    // Rogue ne le voit pas, ne le lit pas, ne peut pas en ouvrir un sur Draco.
    let boite = ent
        .list_fils_for_actor(etab, rogue, prof, None, true, None)
        .await
        .unwrap();
    assert!(!boite.iter().any(|f| f.id == fil_draco.id), "fil hors périmètre listé");
    assert!(boite.iter().all(|f| f.eleve_id != draco));
    assert!(matches!(
        ent.list_messages_for_actor(etab, rogue, prof, fil_draco.id).await,
        Err(EntDbError::Forbidden)
    ));
    let r = ent
        .ouvrir_fil_for_actor(
            prof,
            OuvrirFil {
                etablissement_id: etab,
                eleve_id: draco,
                sujet: "x".into(),
                premier_message: "x".into(),
                actor_user_id: rogue,
            },
        )
        .await;
    assert!(matches!(r, Err(EntDbError::Forbidden)), "{r:?}");

    // McGonagall (enseigne en 5A) le voit ; Rogue voit ses propres élèves.
    let boite_mcg = ent
        .list_fils_for_actor(etab, mcg, prof, None, true, None)
        .await
        .unwrap();
    assert!(boite_mcg.iter().any(|f| f.id == fil_draco.id));
    let (fil_harry, _) = ent
        .ouvrir_fil_for_actor(
            prof,
            OuvrirFil {
                etablissement_id: etab,
                eleve_id: harry,
                sujet: "Socle 1 — suivi".into(),
                premier_message: "Bonjour.".into(),
                actor_user_id: rogue,
            },
        )
        .await
        .unwrap();
    assert!(ent
        .list_messages_for_actor(etab, rogue, prof, fil_harry.id)
        .await
        .is_ok());
}
