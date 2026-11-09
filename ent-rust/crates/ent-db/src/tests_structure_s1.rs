//! Socle 5, lots S0 et S1 : garde-fous d'état et objet unique « cours » (membres
//! individuels, règle de visibilité unique). Base locale de test (feature `labo`) ; sans
//! `ENT_DATABASE_URL`, ignorés en local et **en échec en CI**.

use super::*;
use crate::ref_commun::Acteur;
use crate::tests_d022_bis::{labo, monde};
use chrono::Duration;
use domain_eleve::{today_utc, CreateEleve, DomainError};
use ent_auth::{Action, Portee, Role};
use std::collections::HashSet;
use uuid::Uuid;

const ADMIN: &str = "administratif";

fn adm(cast: &LaboCast) -> Acteur<'static> {
    Acteur {
        etablissement_id: cast.etablissement_id,
        user_id: cast.administratif.user_id,
        role: Role::Administratif.as_str(),
    }
}

async fn eleve_libre(ent: &PgEnt, etab: Uuid, nom: &str) -> Uuid {
    ent.create_eleve(CreateEleve {
        etablissement_id: etab,
        nom: nom.into(),
        prenom: "S1".into(),
        ine: None,
        sexe: None,
        date_naissance: None,
        numero_badge: None,
        redoublant: false,
        classe: None,
        regime: None,
        mef: None,
        site: None,
        date_entree: None,
        date_sortie: None,
        motif_sortie: None,
        actor_user_id: None,
    })
    .await
    .unwrap()
    .id
}

async fn suit(ent: &PgEnt, etab: Uuid, prof: Uuid, eleve: Uuid, jour: chrono::NaiveDate) -> bool {
    ent.prof_suit_eleve_le(etab, prof, eleve, jour).await.unwrap()
}

async fn suivis(ent: &PgEnt, etab: Uuid, prof: Uuid) -> HashSet<Uuid> {
    ent.eleves_autorises(etab, prof, Role::Prof.as_str(), Action::SuiviEleve)
        .await
        .unwrap()
        .expect("borné")
}

/// Élève d'ULIS (hors de la classe) inclus dans un cours de groupe : vu du seul professeur du
/// cours, sur la seule période d'inclusion, sur toutes les voies de la règle unique ; plus vu
/// une fois l'inclusion annulée.
#[tokio::test]
async fn membre_inclus_suivi_par_le_professeur_du_cours_et_de_lui_seul() {
    let Some((ent, cast)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = cast.etablissement_id;
    let m = monde(&ent, &cast).await;
    let a = adm(&cast);
    let x = eleve_libre(&ent, etab, "Ulis").await;
    let t = today_utc();
    assert!(!suit(&ent, etab, m.prof_all, x, t).await, "avant inclusion");

    let inc = ent
        .cours_membre_ajouter(&a, m.s_all, x, "inclusion", "ulis", t - Duration::days(1), Some(t + Duration::days(5)))
        .await
        .unwrap();
    assert!(suit(&ent, etab, m.prof_all, x, t).await, "prof du cours : suit l'inclus");
    assert!(!suit(&ent, etab, m.prof_esp, x, t).await, "autre groupe : ne le voit pas");
    assert!(!suit(&ent, etab, m.prof_principal, x, t).await, "professeur principal d'une autre classe : non");
    assert!(!suit(&ent, etab, m.prof_all, x, t - Duration::days(2)).await, "avant la période");
    assert!(!suit(&ent, etab, m.prof_all, x, t + Duration::days(6)).await, "après la période");
    assert!(suivis(&ent, etab, m.prof_all).await.contains(&x), "liste d'élèves autorisés");
    assert!(!suivis(&ent, etab, m.prof_esp).await.contains(&x));
    assert!(ent.autoriser_eleve(etab, m.prof_all, Role::Prof.as_str(), Action::SuiviEleve, x).await.unwrap());
    let l = ent.mes_enseignements_for_actor(etab, m.prof_all, Role::Prof.as_str(), t).await.unwrap();
    assert_eq!(l.iter().find(|e| e.service_id == m.s_all).unwrap().effectif, 3, "2 du groupe + 1 inclus");

    // Public du cours : visible du professeur du cours, refusé à l'autre professeur.
    let public = ent.cours_public_pour_acteur(&Acteur { etablissement_id: etab, user_id: m.prof_all, role: Role::Prof.as_str() }, m.s_all, t).await.unwrap();
    assert_eq!(public.iter().copied().collect::<HashSet<_>>(), m.all.iter().copied().chain([x]).collect());
    let err = ent.cours_public_pour_acteur(&Acteur { etablissement_id: etab, user_id: m.prof_esp, role: Role::Prof.as_str() }, m.s_all, t).await;
    assert!(matches!(err, Err(EntDbError::Forbidden)), "{err:?}");

    // Annulation : effet immédiat ; trace conservée (jamais supprimée).
    ent.cours_membre_annuler(&a, inc.id).await.unwrap();
    ent.cours_membre_annuler(&a, inc.id).await.unwrap(); // idempotent
    assert!(!suit(&ent, etab, m.prof_all, x, t).await);
    let traces = ent.cours_membres(&a, m.s_all).await.unwrap();
    assert!(traces.iter().any(|t| t.id == inc.id && t.annulee));
    // Immuable hors annulation : ni réactivation, ni changement de nature ou de dates.
    for sql in [
        "UPDATE ent_cours_membre SET annulee_le = NULL WHERE id = $1",
        "UPDATE ent_cours_membre SET nature = 'exclusion' WHERE id = $1",
        "UPDATE ent_cours_membre SET du = du + 1 WHERE id = $1",
    ] {
        assert!(sqlx::query(sql).bind(inc.id).execute(ent.pool()).await.is_err(), "{sql}");
    }
    let del = sqlx::query("DELETE FROM ent_cours_membre WHERE id = $1").bind(inc.id).execute(ent.pool()).await;
    assert!(del.is_err(), "suppression physique refusée");
}

/// Exclusion datée : l'élève sort du public du cours (groupe ou classe entière) pour les dates
/// concernées ; il reste visible avant ; l'autre cours qui le couvre continue de le montrer.
#[tokio::test]
async fn exclusion_retire_l_eleve_du_public_a_partir_de_sa_date() {
    let Some((ent, cast)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = cast.etablissement_id;
    let m = monde(&ent, &cast).await;
    let a = adm(&cast);
    let t = today_utc();
    let (parti, reste) = (m.all[0], m.all[1]);
    assert!(suit(&ent, etab, m.prof_all, parti, t).await);
    let depart = t + Duration::days(3);
    ent.cours_membre_ajouter(&a, m.s_all, parti, "exclusion", "echange", depart, None).await.unwrap();
    assert!(!suit(&ent, etab, m.prof_all, parti, depart).await, "exclu à partir de la date");
    assert!(suit(&ent, etab, m.prof_all, parti, depart - Duration::days(1)).await, "visible avant : bulletin honnête");
    assert!(suit(&ent, etab, m.prof_all, reste, depart).await, "les autres membres restent");
    assert!(suit(&ent, etab, m.prof_all, parti, t).await, "pas encore exclu aujourd'hui");
    let l = ent.mes_enseignements_for_actor(etab, m.prof_all, Role::Prof.as_str(), depart).await.unwrap();
    assert_eq!(l.iter().find(|e| e.service_id == m.s_all).unwrap().effectif, 1);

    // Cours de classe entière (professeur principal) : même règle.
    let s_classe: Uuid = sqlx::query_scalar(
        "SELECT id FROM ent_service_enseignement WHERE etablissement_id = $1 AND prof_user_id = $2 AND classe_id = $3",
    )
    .bind(etab).bind(m.prof_principal).bind(m.classe)
    .fetch_one(ent.pool()).await.unwrap();
    let cible = m.esp[0];
    assert!(suit(&ent, etab, m.prof_principal, cible, t).await);
    ent.cours_membre_ajouter(&a, s_classe, cible, "exclusion", "dispense_cours", t, Some(t + Duration::days(3))).await.unwrap();
    assert!(!suit(&ent, etab, m.prof_principal, cible, t).await);
    assert!(suit(&ent, etab, m.prof_principal, cible, t + Duration::days(4)).await, "après l'exclusion");
}

/// Un élève inclus à la fois comme membre et exclu : l'exclusion l'emporte ; garde-fous de
/// saisie (nature, motif, dates, chevauchement, cours ou élève inconnu, droits).
#[tokio::test]
async fn membres_garde_fous_de_saisie() {
    let Some((ent, cast)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = cast.etablissement_id;
    let m = monde(&ent, &cast).await;
    let a = adm(&cast);
    let t = today_utc();
    let x = eleve_libre(&ent, etab, "Garde").await;
    let invalide = |r: Result<crate::cours::CoursMembre, EntDbError>| matches!(r, Err(EntDbError::Domain(DomainError::ValeurInvalide)));
    assert!(invalide(ent.cours_membre_ajouter(&a, m.s_all, x, "autre_nature", "ulis", t, None).await));
    assert!(invalide(ent.cours_membre_ajouter(&a, m.s_all, x, "inclusion", "motif_libre", t, None).await));
    assert!(invalide(ent.cours_membre_ajouter(&a, m.s_all, x, "inclusion", "ulis", t, Some(t - Duration::days(1))).await));
    assert!(
        matches!(ent.cours_membre_ajouter(&a, Uuid::new_v4(), x, "inclusion", "ulis", t, None).await, Err(EntDbError::Domain(DomainError::NotFound))),
        "cours inconnu : introuvable (lot S4 : identifiant résolu avant toute écriture)"
    );
    assert!(ent.cours_membre_ajouter(&a, m.s_all, Uuid::new_v4(), "inclusion", "ulis", t, None).await.is_err(), "élève inconnu");
    // Hors de l'année du cours.
    assert!(invalide(ent.cours_membre_ajouter(&a, m.s_all, x, "inclusion", "ulis", t + Duration::days(900), None).await));
    // Chevauchement de deux inclusions actives : refusé ; après annulation, accepté.
    let i1 = ent.cours_membre_ajouter(&a, m.s_all, x, "inclusion", "ulis", t, Some(t + Duration::days(10))).await.unwrap();
    let r = ent.cours_membre_ajouter(&a, m.s_all, x, "inclusion", "decloisonnement", t + Duration::days(5), None).await;
    assert!(matches!(r, Err(EntDbError::Domain(DomainError::Conflit))), "{r:?}");
    ent.cours_membre_annuler(&a, i1.id).await.unwrap();
    ent.cours_membre_ajouter(&a, m.s_all, x, "inclusion", "decloisonnement", t + Duration::days(5), None).await.unwrap();
    // Exclusion qui recouvre une inclusion : l'exclusion l'emporte.
    ent.cours_membre_ajouter(&a, m.s_all, x, "exclusion", "echange", t + Duration::days(7), None).await.unwrap();
    assert!(suit(&ent, etab, m.prof_all, x, t + Duration::days(6)).await);
    assert!(!suit(&ent, etab, m.prof_all, x, t + Duration::days(8)).await);
    // Droits : un professeur ne gère pas la structure.
    let prof = Acteur { etablissement_id: etab, user_id: m.prof_all, role: Role::Prof.as_str() };
    assert!(matches!(
        ent.cours_membre_ajouter(&prof, m.s_all, x, "inclusion", "ulis", t, None).await,
        Err(EntDbError::Forbidden)
    ));
    assert!(matches!(ent.cours_membres(&prof, m.s_all).await, Err(EntDbError::Forbidden)));
    // Événements publiés, sans motif.
    let (n,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM ent_metier_event WHERE etablissement_id = $1 AND type = 'referentiel.cours_membre_ajoute'
            AND payload::text NOT LIKE '%ulis%' AND payload::text NOT LIKE '%echange%'",
    )
    .bind(etab).fetch_one(ent.pool()).await.unwrap();
    assert!(n >= 3, "événements publiés sans motif ({n})");
}

/// Un membre individuel d'un cours d'un autre établissement est invisible et refusé ; la règle
/// ne mélange jamais deux établissements.
#[tokio::test]
async fn cours_d_un_autre_etablissement_introuvable() {
    let Some((ent, cast)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = cast.etablissement_id;
    let m = monde(&ent, &cast).await;
    let autre = Uuid::new_v4();
    let t = today_utc();
    assert!(ent.public_cours(autre, t, Some(m.s_all), None, None).await.unwrap().is_empty());
    let acteur_autre = Acteur { etablissement_id: autre, user_id: cast.direction.user_id, role: Role::Direction.as_str() };
    let r = ent.cours_public_pour_acteur(&acteur_autre, m.s_all, t).await;
    assert!(matches!(r, Err(EntDbError::Domain(DomainError::NotFound))), "{r:?}");
    assert!(ent.public_cours(etab, t, Some(m.s_all), None, None).await.unwrap().len() >= 2);
}

/// S0 n° 18 : un élève sorti n'est plus visible du responsable de son groupe.
#[tokio::test]
async fn responsable_de_groupe_ne_voit_pas_l_eleve_sorti() {
    let Some((ent, cast)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = cast.etablissement_id;
    let dir = (cast.direction.user_id, cast.direction.role.as_str());
    let resp = cast.profs[1].user_id;
    let code = format!("ATL-{}", &Uuid::new_v4().simple().to_string()[..6]);
    let g = ent.referentiel_creer_groupe(ADMIN, etab, None, &code, Some("Atelier"), None).await.unwrap();
    let (reste, sorti) = (eleve_libre(&ent, etab, "Reste").await, eleve_libre(&ent, etab, "Sorti").await);
    for e in [reste, sorti] {
        ent.referentiel_affecter_eleve_groupe(ADMIN, etab, g.id, e, Some(today_utc())).await.unwrap();
    }
    ent.definir_responsables_groupe_for_actor(etab, dir.0, dir.1, g.id, &[resp]).await.unwrap();
    let role = ent
        .creer_role_for_actor(
            etab, dir.0, dir.1,
            RoleSaisie {
                nom: format!("Atelier {code}"),
                description: None,
                modele: None,
                droits: vec![Droit { action: Action::AbsencesLire, portee: Portee::MesGroupes, cibles: vec![] }],
            },
        )
        .await
        .unwrap();
    ent.attribuer_role_for_actor(etab, dir.0, dir.1, role.id, resp, None).await.unwrap();
    let prof = Role::Prof.as_str();
    assert!(ent.actor_can_read_absences(etab, resp, prof, sorti).await.unwrap(), "avant la sortie");
    sqlx::query("UPDATE ent_eleve SET date_sortie = $2 WHERE id = $1")
        .bind(sorti).bind(today_utc() - Duration::days(1)).execute(ent.pool()).await.unwrap();
    assert!(!ent.actor_can_read_absences(etab, resp, prof, sorti).await.unwrap(), "élève sorti : plus visible");
    assert!(ent.actor_can_read_absences(etab, resp, prof, reste).await.unwrap());
    let v = ent.eleves_autorises(etab, resp, prof, Action::AbsencesLire).await.unwrap().expect("borné");
    assert!(v.contains(&reste) && !v.contains(&sorti));
    ent.retirer_role_for_actor(etab, dir.0, dir.1, role.id, resp).await.unwrap();
    ent.supprimer_role_for_actor(etab, dir.0, dir.1, role.id).await.unwrap();
}

/// S0 n° 3, 4, 5, 17 : aucune année ni classe créée par une lecture, ouverture contrôlée comme
/// la clôture, année clôturée non rouvrable, affectation corrigée archivée.
#[tokio::test]
async fn garde_fous_d_etat_de_l_annee_et_des_affectations() {
    let Some((ent, cast)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = cast.etablissement_id;
    let t = today_utc();
    // n° 3 : lire « mes classes » à une date sans année ne crée rien.
    let loin = chrono::NaiveDate::from_ymd_opt(2090, 1, 15).unwrap();
    let avant: i64 = sqlx::query_scalar("SELECT count(*) FROM ent_annee_scolaire WHERE etablissement_id = $1")
        .bind(etab).fetch_one(ent.pool()).await.unwrap();
    let l = ent.classes_du_prof(etab, cast.profs[0].user_id, loin).await.unwrap();
    assert!(l.is_empty());
    let apres: i64 = sqlx::query_scalar("SELECT count(*) FROM ent_annee_scolaire WHERE etablissement_id = $1")
        .bind(etab).fetch_one(ent.pool()).await.unwrap();
    assert_eq!(avant, apres, "aucune année créée par une lecture");
    // n° 5 : l'ancienne route ne retire plus la courante sans la clôturer.
    let r = ent
        .referentiel_creer_annee(ADMIN, etab, "2091-2092",
            chrono::NaiveDate::from_ymd_opt(2091, 9, 1).unwrap(), chrono::NaiveDate::from_ymd_opt(2092, 8, 31).unwrap(), true)
        .await;
    assert!(matches!(r, Err(EntDbError::Domain(DomainError::Conflit))), "{r:?}");
    let courantes: i64 = sqlx::query_scalar("SELECT count(*) FROM ent_annee_scolaire WHERE etablissement_id = $1 AND courante")
        .bind(etab).fetch_one(ent.pool()).await.unwrap();
    assert_eq!(courantes, 1);
    // n° 5 : une année clôturée ne se rouvre pas (garde en base).
    let close: Option<Uuid> = sqlx::query_scalar("SELECT id FROM ent_annee_scolaire WHERE etablissement_id = $1 AND cloturee_le IS NOT NULL LIMIT 1")
        .bind(etab).fetch_optional(ent.pool()).await.unwrap();
    if let Some(id) = close {
        let r = sqlx::query("UPDATE ent_annee_scolaire SET cloturee_le = NULL WHERE id = $1").bind(id).execute(ent.pool()).await;
        assert!(r.is_err(), "réouverture refusée");
    }
    // n° 17 : une affectation corrigée est archivée, pas perdue.
    let annee = ent.referentiel_annee_courante_assuree(etab).await.unwrap();
    let classe = ent
        .referentiel_creer_classe(ADMIN, etab, Some(annee.id), &format!("AR{}", &Uuid::new_v4().simple().to_string()[..5]), None, None)
        .await.unwrap();
    let autre = ent
        .referentiel_creer_classe(ADMIN, etab, Some(annee.id), &format!("AS{}", &Uuid::new_v4().simple().to_string()[..5]), None, None)
        .await.unwrap();
    let e = eleve_libre(&ent, etab, "Archive").await;
    ent.referentiel_affecter_eleve_classe(ADMIN, etab, classe.id, e, Some(t)).await.unwrap();
    ent.referentiel_affecter_eleve_classe(ADMIN, etab, autre.id, e, Some(t)).await.unwrap(); // même jour : correction
    let (n,): (i64,) = sqlx::query_as("SELECT count(*) FROM ent_affectation_supprimee WHERE etablissement_id = $1 AND eleve_id = $2 AND classe_id = $3")
        .bind(etab).bind(e).bind(classe.id).fetch_one(ent.pool()).await.unwrap();
    assert_eq!(n, 1, "affectation corrigée archivée");
    // n° 16 : une affectation vers la classe d'un autre établissement est refusée.
    let r = sqlx::query("INSERT INTO ent_affectation (id, etablissement_id, annee_id, eleve_id, classe_id, date_debut) VALUES ($1,$2,$3,$4,$5,$6)")
        .bind(Uuid::new_v4()).bind(Uuid::new_v4()).bind(annee.id).bind(e).bind(classe.id).bind(t)
        .execute(ent.pool()).await;
    assert!(r.is_err(), "incohérence d'établissement refusée");
}
