//! D-022 bis — conseil de classe borné au groupe du professeur et liste unique
//! « mes enseignements ». Base locale de test (feature `labo`) ; sans
//! `ENT_DATABASE_URL`, les scénarios sont ignorés.

use super::*;
use crate::referentiel_repo::CreerService;
use chrono::{Duration, NaiveTime};
use domain_eleve::{today_utc, CreateEleve, CreerNotePeriode};
use ent_auth::Role;
use ent_types::evaluations::*;
use std::collections::HashSet;
use uuid::Uuid;

const ADMIN: &str = "administratif";

async fn labo() -> Option<(PgEnt, LaboCast)> {
    let url = std::env::var("ENT_DATABASE_URL").ok()?;
    assert_local_postgres_url(&url).expect("ENT_DATABASE_URL doit être locale");
    let mut ent = PgEnt::connect(&url).await.expect("connect");
    ent.migrate().await.expect("migrate");
    ent.install_brain_p0();
    let cast = ent.seed_labo_cast().await.expect("seed cast");
    Some((ent, cast))
}

struct Monde {
    classe: Uuid,
    periode: Uuid,
    all: [Uuid; 2],
    esp: [Uuid; 2],
    prof_all: Uuid,
    prof_esp: Uuid,
    prof_principal: Uuid,
    s_all: Uuid,
}

/// Classe mixte (4 élèves : 2 allemand, 2 espagnol), un professeur par groupe, un
/// professeur principal de la classe entière.
async fn monde(ent: &PgEnt, cast: &LaboCast) -> Monde {
    let etab = cast.etablissement_id;
    let annee = ent.referentiel_annee_courante_assuree(etab).await.unwrap();
    let sfx = Uuid::new_v4().simple().to_string()[..6].to_string();
    let classe = ent
        .referentiel_creer_classe(ADMIN, etab, Some(annee.id), &format!("EB{sfx}"), Some("4e bis"), Some("4EME"))
        .await
        .unwrap();
    let matiere = |code: &str, lib: &str| {
        let (code, lib) = (format!("{code}{sfx}"), lib.to_string());
        let ent = ent;
        async move {
            ent.create_matiere(
                CreateMatiere { etablissement_id: etab, code, libelle: lib, source_nomenclature: None },
                None,
            )
            .await
            .unwrap()
            .id
        }
    };
    let (m_all, m_esp, m_fr) = (
        matiere("A", "Allemand").await,
        matiere("E", "Espagnol").await,
        matiere("F", "Français").await,
    );
    let mut eleves = Vec::new();
    for i in 0..4 {
        let e = ent
            .create_eleve(CreateEleve {
                etablissement_id: etab,
                nom: format!("Bis{i}"),
                prenom: "D022".into(),
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
            .unwrap();
        ent.referentiel_affecter_eleve_classe(ADMIN, etab, classe.id, e.id, None).await.unwrap();
        eleves.push(e.id);
    }
    let g_all = ent
        .referentiel_creer_groupe(ADMIN, etab, None, &format!("ALL{sfx}"), None, Some(classe.id))
        .await
        .unwrap();
    let g_esp = ent
        .referentiel_creer_groupe(ADMIN, etab, None, &format!("ESP{sfx}"), None, Some(classe.id))
        .await
        .unwrap();
    for e in &eleves[..2] {
        ent.referentiel_affecter_eleve_groupe(ADMIN, etab, g_all.id, *e, None).await.unwrap();
    }
    for e in &eleves[2..] {
        ent.referentiel_affecter_eleve_groupe(ADMIN, etab, g_esp.id, *e, None).await.unwrap();
    }
    let (prof_all, prof_esp, prof_principal) =
        (cast.profs[0].user_id, cast.profs[1].user_id, cast.profs[2].user_id);
    let svc = |prof_user_id, matiere_id, classe_id, groupe_id, pp| CreerService {
        prof_user_id,
        matiere_id: Some(matiere_id),
        classe_id,
        groupe_id,
        professeur_principal: pp,
    };
    let s_all = ent
        .referentiel_creer_service(ADMIN, etab, svc(prof_all, m_all, None, Some(g_all.id), false))
        .await
        .unwrap()
        .id;
    ent.referentiel_creer_service(ADMIN, etab, svc(prof_esp, m_esp, None, Some(g_esp.id), false))
        .await
        .unwrap();
    ent.referentiel_creer_service(ADMIN, etab, svc(prof_principal, m_fr, Some(classe.id), None, true))
        .await
        .unwrap();
    let t = today_utc();
    let periode = ent
        .creer_note_periode(CreerNotePeriode {
            etablissement_id: etab,
            code: format!("T{sfx}"),
            libelle: "Trimestre bis".into(),
            date_debut: (t - Duration::days(10)).max(annee.date_debut),
            date_fin: (t + Duration::days(30)).min(annee.date_fin),
            actor_user_id: None,
        })
        .await
        .unwrap()
        .id;
    Monde {
        classe: classe.id,
        periode,
        all: [eleves[0], eleves[1]],
        esp: [eleves[2], eleves[3]],
        prof_all,
        prof_esp,
        prof_principal,
        s_all,
    }
}

fn ens(v: &[Uuid]) -> HashSet<Uuid> {
    v.iter().copied().collect()
}

/// Conseil de classe : le professeur d'un groupe ne voit que son groupe (préparation,
/// décisions, vœux) ; le professeur principal et la direction gardent la classe entière.
#[tokio::test]
async fn conseil_borne_au_groupe_du_professeur() {
    let Some((ent, cast)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = cast.etablissement_id;
    let m = monde(&ent, &cast).await;
    let prof = Role::Prof.as_str();
    let dir = cast.direction.user_id;
    let conseil = ent
        .creer_conseil_eva_for_actor(
            etab,
            dir,
            "direction",
            &CreerConseilBody {
                classe_id: m.classe,
                periode_id: m.periode,
                jour: today_utc(),
                heure: NaiveTime::from_hms_opt(17, 0, 0).unwrap(),
                lieu: "Salle 3".into(),
            },
        )
        .await
        .unwrap();
    let ids = |p: &PreparationConseilDto| -> HashSet<Uuid> { p.eleves.iter().map(|e| e.eleve_id).collect() };

    // Professeur d'allemand : son groupe seulement, pas de moyenne de classe, pas de décision.
    let p = ent.preparation_conseil_for_actor(etab, m.prof_all, prof, conseil.id).await.unwrap();
    assert_eq!(ids(&p), ens(&m.all), "groupe allemand seulement");
    assert!(p.moyenne_classe.is_none(), "agrégat de classe non communiqué");
    assert!(!p.peut_decider);
    // Professeur d'espagnol : l'autre groupe, jamais celui d'allemand.
    let p = ent.preparation_conseil_for_actor(etab, m.prof_esp, prof, conseil.id).await.unwrap();
    assert_eq!(ids(&p), ens(&m.esp));
    // Professeur principal et direction : classe entière.
    let tous: HashSet<Uuid> = m.all.iter().chain(m.esp.iter()).copied().collect();
    let p = ent.preparation_conseil_for_actor(etab, m.prof_principal, prof, conseil.id).await.unwrap();
    assert_eq!(ids(&p), tous, "professeur principal : classe entière");
    assert!(p.peut_decider);
    let p = ent.preparation_conseil_for_actor(etab, dir, "direction", conseil.id).await.unwrap();
    assert_eq!(ids(&p), tous, "direction : classe entière");

    // Décisions : un professeur de groupe non principal ne décide pas ; le principal oui,
    // et le professeur d'allemand ne voit ensuite que les décisions de son groupe.
    let decision = |eleve_id| DecisionConseilBody {
        eleve_id,
        appreciation: "Bon trimestre.".into(),
        mention: Some("encouragements".into()),
        decision_code: Some("passage".into()),
        avis_orientation: None,
    };
    assert!(matches!(
        ent.decider_conseil_eva_for_actor(etab, m.prof_all, prof, conseil.id, &decision(m.all[0])).await,
        Err(EntDbError::Forbidden)
    ));
    for e in m.all.iter().chain(m.esp.iter()) {
        ent.decider_conseil_eva_for_actor(etab, m.prof_principal, prof, conseil.id, &decision(*e))
            .await
            .unwrap();
    }
    let p = ent.preparation_conseil_for_actor(etab, m.prof_all, prof, conseil.id).await.unwrap();
    assert_eq!(ids(&p), ens(&m.all));
    assert!(p.eleves.iter().all(|e| e.decision.is_some()));
    let p = ent.preparation_conseil_for_actor(etab, m.prof_esp, prof, conseil.id).await.unwrap();
    assert!(p.eleves.iter().all(|e| ens(&m.esp).contains(&e.eleve_id)));
}

/// Liste unique : le professeur d'un groupe obtient son enseignement de groupe avec un
/// effectif borné ; le professeur principal obtient la classe entière ; personne ne voit
/// les enseignements d'un autre.
#[tokio::test]
async fn mes_enseignements_classe_et_groupe_dans_une_seule_liste() {
    let Some((ent, cast)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = cast.etablissement_id;
    let m = monde(&ent, &cast).await;
    let prof = Role::Prof.as_str();
    // Le jeu de labo peut déjà donner d'autres services à ces professeurs : on retrouve les nôtres.
    let l = ent.mes_enseignements_for_actor(etab, m.prof_all, prof, today_utc()).await.unwrap();
    let e = l.iter().find(|e| e.service_id == m.s_all).expect("enseignement de groupe");
    assert_eq!(e.nature, "groupe");
    assert_eq!(e.matiere_libelle, "Allemand");
    assert_eq!(e.effectif, 2, "effectif du groupe, pas de la classe");
    assert_eq!(e.classe_id, Some(m.classe), "classe parente du groupe");
    assert!(e.libelle.contains("Allemand") && e.libelle.contains("groupe"));
    assert!(l.iter().all(|x| x.service_id != Uuid::nil()));
    let l = ent.mes_enseignements_for_actor(etab, m.prof_principal, prof, today_utc()).await.unwrap();
    let e = l
        .iter()
        .find(|e| e.nature == "classe" && e.classe_id == Some(m.classe))
        .expect("enseignement de classe entière");
    assert_eq!(e.effectif, 4);
    assert!(e.professeur_principal);
    // Jamais l'enseignement de groupe d'un autre professeur.
    assert!(l.iter().all(|x| x.service_id != m.s_all));
    // Les familles n'ont pas d'enseignements.
    assert!(matches!(
        ent.mes_enseignements_for_actor(etab, cast.parent_lea.user_id, "parent", today_utc()).await,
        Err(EntDbError::Forbidden)
    ));
    // Cohérence avec `eleves_autorises` : l'effectif ne dépasse jamais le périmètre.
    let ok = ent
        .eleves_autorises(etab, m.prof_all, prof, ent_auth::Action::SuiviEleve)
        .await
        .unwrap()
        .expect("borné");
    assert!(ens(&m.all).is_subset(&ok), "son groupe est suivi");
    assert!(ok.is_disjoint(&ens(&m.esp)), "jamais l'autre groupe");
}
