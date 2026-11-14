//! D-022 — portée du professeur au GROUPE dans une classe mixte, et exports officiels
//! réservés aux administratifs. Base locale de test (feature `labo`) ; sans
//! `ENT_DATABASE_URL`, les scénarios sont ignorés.
//!
//! Classe mixte : « LV2 allemand » (2 élèves) et « LV2 espagnol » (2 élèves). Les noms
//! de groupes sont ici de simples libellés d'établissement (aucune liste dans le code).

use super::*;
use crate::cahier::Cible;
use chrono::NaiveTime;
use domain_eleve::{today_utc, CreateEleve};
use ent_auth::{Action, Role};
use std::collections::HashSet;
use uuid::Uuid;

async fn labo() -> Option<(PgEnt, LaboCast)> {
    let url = crate::url_guard::url_labo_ou_echec()?;
    assert_local_postgres_url(&url).expect("ENT_DATABASE_URL doit être locale");
    let mut ent = PgEnt::connect(&url).await.expect("connect");
    ent.migrate().await.expect("migrate");
    ent.install_brain_p0();
    let cast = ent.seed_labo_cast().await.expect("seed cast");
    Some((ent, cast))
}

struct Mixte {
    classe_texte: String,
    classe_id: Uuid,
    allemand: Uuid,
    espagnol: Uuid,
    /// Élèves du groupe allemand puis du groupe espagnol.
    all: [Uuid; 2],
    esp: [Uuid; 2],
    prof_all: Uuid,
    prof_esp: Uuid,
    prof_principal: Uuid,
}

async fn mixte(ent: &PgEnt, cast: &LaboCast) -> Mixte {
    let etab = cast.etablissement_id;
    let adm = Role::Administratif.as_str();
    let suffixe = Uuid::new_v4().simple().to_string()[..6].to_string();
    let classe_texte = format!("MIX{suffixe}");
    let mut eleves = Vec::new();
    for i in 0..4 {
        let e = ent
            .create_eleve(CreateEleve {
                etablissement_id: etab,
                nom: format!("Mixte{i}"),
                prenom: "D022".into(),
                ine: None,
                sexe: None,
                date_naissance: None,
                numero_badge: None,
                redoublant: false,
                classe: Some(classe_texte.clone()),
                regime: None,
                mef: None,
                site: None,
                date_entree: Some(today_utc() - chrono::Duration::days(30)),
                date_sortie: None,
                motif_sortie: None,
                actor_user_id: Some("d022".into()),
            })
            .await
            .expect("élève");
        eleves.push(e.id);
    }
    // `create_eleve` écrit aussi le référentiel (classe + affectation) : on relit la classe.
    let annee = ent.referentiel_annee_courante_assuree(etab).await.expect("année");
    let classe = ent
        .referentiel_classe_par_code(etab, annee.id, &classe_texte)
        .await
        .expect("classe")
        .expect("classe créée avec les élèves");
    for e in &eleves {
        let (n,): (i64,) = sqlx::query_as(
            "SELECT count(*) FROM ent_affectation WHERE etablissement_id = $1 AND eleve_id = $2 AND classe_id = $3",
        )
        .bind(etab)
        .bind(e)
        .bind(classe.id)
        .fetch_one(ent.pool())
        .await
        .unwrap();
        if n == 0 {
            ent.referentiel_affecter_eleve_classe(adm, etab, classe.id, *e, None)
                .await
                .expect("affectation classe");
        }
    }
    let allemand = ent
        .referentiel_creer_groupe(adm, etab, None, &format!("ALL{suffixe}"), None, Some(classe.id))
        .await
        .expect("groupe allemand");
    let espagnol = ent
        .referentiel_creer_groupe(adm, etab, None, &format!("ESP{suffixe}"), None, Some(classe.id))
        .await
        .expect("groupe espagnol");
    for e in &eleves[..2] {
        ent.referentiel_affecter_eleve_groupe(adm, etab, allemand.id, *e, None).await.unwrap();
    }
    for e in &eleves[2..] {
        ent.referentiel_affecter_eleve_groupe(adm, etab, espagnol.id, *e, None).await.unwrap();
    }
    let (prof_all, prof_esp, prof_principal) =
        (cast.profs[0].user_id, cast.profs[1].user_id, cast.profs[2].user_id);
    let service = |prof_user_id, classe_id, groupe_id, pp| CreerService {
        prof_user_id,
        matiere_id: None,
        classe_id,
        groupe_id,
        professeur_principal: pp,
    };
    ent.referentiel_creer_service(adm, etab, service(prof_all, None, Some(allemand.id), false)).await.unwrap();
    ent.referentiel_creer_service(adm, etab, service(prof_esp, None, Some(espagnol.id), false)).await.unwrap();
    ent.referentiel_creer_service(adm, etab, service(prof_principal, Some(classe.id), None, true)).await.unwrap();
    Mixte {
        classe_texte,
        classe_id: classe.id,
        allemand: allemand.id,
        espagnol: espagnol.id,
        all: [eleves[0], eleves[1]],
        esp: [eleves[2], eleves[3]],
        prof_all,
        prof_esp,
        prof_principal,
    }
}

fn ensemble(v: &[Uuid]) -> HashSet<Uuid> {
    v.iter().copied().collect()
}

/// Périmètre « suivi » du professeur : groupe seulement ; principal = classe ; direction = tout.
#[tokio::test]
async fn d022_prof_de_groupe_ne_suit_que_son_groupe() {
    let Some((ent, cast)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = cast.etablissement_id;
    let m = mixte(&ent, &cast).await;
    let prof = Role::Prof.as_str();
    for e in m.all {
        assert!(ent.prof_suit_eleve(etab, m.prof_all, e).await.unwrap(), "allemand suit son groupe");
        assert!(!ent.prof_suit_eleve(etab, m.prof_esp, e).await.unwrap(), "espagnol ne suit pas l'allemand");
        assert!(ent.prof_suit_eleve(etab, m.prof_principal, e).await.unwrap(), "principal : classe");
    }
    for e in m.esp {
        assert!(!ent.prof_suit_eleve(etab, m.prof_all, e).await.unwrap(), "allemand ne suit pas l'espagnol");
        assert!(ent.prof_suit_eleve(etab, m.prof_esp, e).await.unwrap());
        assert!(ent.prof_suit_eleve(etab, m.prof_principal, e).await.unwrap());
    }
    // Listes (filtres) : mêmes ensembles pour chaque action « mes classes ».
    for action in [
        Action::SuiviEleve,
        Action::NotesLire,
        Action::NotesSaisir,
        Action::AbsencesLire,
        Action::DossierLire,
        Action::CahierLire,
    ] {
        let vus = ent.eleves_autorises(etab, m.prof_all, prof, action).await.unwrap().unwrap();
        assert!(ensemble(&m.all).is_subset(&vus) && vus.is_disjoint(&ensemble(&m.esp)), "{action:?}");
        let vus = ent.eleves_autorises(etab, m.prof_esp, prof, action).await.unwrap().unwrap();
        assert!(ensemble(&m.esp).is_subset(&vus) && vus.is_disjoint(&ensemble(&m.all)), "{action:?}");
    }
    // La direction garde l'établissement entier.
    let dir = ent
        .eleves_autorises(etab, cast.direction.user_id, Role::Direction.as_str(), Action::SuiviEleve)
        .await
        .unwrap();
    assert!(dir.is_none());
    // Classes du périmètre : le professeur de groupe garde la classe (il y enseigne un groupe).
    let classes = ent.classes_dans_perimetre(etab, m.prof_all, prof).await.unwrap().unwrap();
    assert!(classes.contains(&m.classe_texte));
}

/// Un créneau EDT en classe-texte (ancien repli) ne rouvre pas la classe entière ;
/// un remplaçant suit les élèves du titulaire (groupe), pas la classe.
#[tokio::test]
async fn d022_repli_edt_et_remplacement_ne_elargissent_pas() {
    let Some((ent, cast)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = cast.etablissement_id;
    let m = mixte(&ent, &cast).await;
    let t = |h| NaiveTime::from_hms_opt(h, 0, 0).unwrap();
    // Le professeur d'espagnol a un créneau « classe MIX » sans groupe (donnée héritée) :
    // il ne gagne pas pour autant les élèves allemands.
    ent.seed_creneau_du_jour(etab, &m.classe_texte, "LV2", m.prof_esp, today_utc(), t(8), t(9), None)
        .await
        .unwrap();
    for e in m.all {
        assert!(!ent.prof_suit_eleve(etab, m.prof_esp, e).await.unwrap(), "créneau classe-texte");
    }
    // Remplacement : un autre professeur (profs[3]) remplace le titulaire du groupe allemand.
    let remplacant = cast.profs[3].user_id;
    let c = ent
        .seed_creneau_du_jour(etab, &m.classe_texte, "All", m.prof_all, today_utc(), t(10), t(11), None)
        .await
        .unwrap();
    ent.rattacher_creneau_groupe_for_actor(Role::Administratif.as_str(), etab, c.id, Some(m.allemand))
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO ent_edt_exception (id, etablissement_id, creneau_id, jour, kind, remplacant_user_id, motif, created_by_user_id)
         VALUES ($1,$2,$3,$4,'remplacement',$5,'D-022',$6)",
    )
    .bind(Uuid::new_v4())
    .bind(etab)
    .bind(c.id)
    .bind(today_utc())
    .bind(remplacant)
    .bind(cast.administratif.user_id)
    .execute(ent.pool())
    .await
    .expect("exception de remplacement");
    for e in m.all {
        assert!(ent.prof_suit_eleve(etab, remplacant, e).await.unwrap(), "remplaçant : groupe du titulaire");
    }
    for e in m.esp {
        assert!(!ent.prof_suit_eleve(etab, remplacant, e).await.unwrap(), "remplaçant : pas l'autre groupe");
    }
}

/// Appel, notes, appréciations, cahier de textes, fiche élève : le professeur d'allemand
/// ne touche que le groupe allemand de la classe mixte.
#[tokio::test]
async fn d022_separation_prof_de_groupe_classe_mixte() {
    let Some((ent, cast)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = cast.etablissement_id;
    let m = mixte(&ent, &cast).await;
    let prof = Role::Prof.as_str();
    let t = |h| NaiveTime::from_hms_opt(h, 0, 0).unwrap();

    // ── Appel : l'effectif du créneau d'allemand (rattaché ou non au groupe) = groupe allemand.
    let c_groupe = ent
        .seed_creneau_du_jour(etab, &m.classe_texte, "Allemand", m.prof_all, today_utc(), t(8), t(9), None)
        .await
        .unwrap();
    ent.rattacher_creneau_groupe_for_actor(Role::Administratif.as_str(), etab, c_groupe.id, Some(m.allemand))
        .await
        .unwrap();
    let (_, lignes) = ent.ouvrir_appel(etab, c_groupe.id, today_utc(), m.prof_all, prof).await.unwrap();
    let ids: HashSet<Uuid> = lignes.iter().map(|l| l.eleve_id).collect();
    assert_eq!(ids, ensemble(&m.all), "appel de groupe");
    let c_classe = ent
        .seed_creneau_du_jour(etab, &m.classe_texte, "Allemand bis", m.prof_all, today_utc(), t(14), t(15), None)
        .await
        .unwrap();
    let (_, lignes) = ent.ouvrir_appel(etab, c_classe.id, today_utc(), m.prof_all, prof).await.unwrap();
    let ids: HashSet<Uuid> = lignes.iter().map(|l| l.eleve_id).collect();
    assert_eq!(ids, ensemble(&m.all), "créneau sans groupe : filtré par le périmètre du professeur");
    // Le professeur d'espagnol ne peut ni ouvrir ni lire l'appel d'allemand (déjà : titulaire).
    assert!(ent.ouvrir_appel(etab, c_groupe.id, today_utc(), m.prof_esp, prof).await.is_err());

    // ── Notes (saisie) et appréciations.
    let matiere = ent.list_matieres(etab).await.unwrap()[0].id;
    let periode = ent.list_periodes(etab, m.all[0]).await.unwrap()[0].id;
    let note = |eleve_id| SaisirNote {
        etablissement_id: etab,
        eleve_id,
        matiere_id: matiere,
        valeur: 12.0,
        sur: 20.0,
        date_note: today_utc(),
        auteur_user_id: m.prof_all,
        libelle: Some("D-022".into()),
        devoir_id: None,
        periode_id: None,
    };
    let r = ent.saisir_note_for_actor(prof, note(m.all[0])).await;
    assert!(!matches!(r, Err(EntDbError::Forbidden)), "note groupe allemand : {r:?}");
    let r = ent.saisir_note_for_actor(prof, note(m.esp[0])).await;
    assert!(matches!(r, Err(EntDbError::Forbidden)), "note groupe espagnol : {r:?}");
    let appr = |eleve_id| domain_eleve::SaisirAppreciation {
        etablissement_id: etab,
        periode_id: periode,
        eleve_id,
        matiere_id: None,
        texte: "D-022".into(),
        actor_user_id: m.prof_all,
    };
    let r = ent.saisir_appreciation_for_actor(prof, appr(m.all[1])).await;
    assert!(!matches!(r, Err(EntDbError::Forbidden)), "appréciation groupe allemand : {r:?}");
    let r = ent.saisir_appreciation_for_actor(prof, appr(m.esp[1])).await;
    assert!(matches!(r, Err(EntDbError::Forbidden)), "appréciation groupe espagnol : {r:?}");
    // Lecture des notes.
    assert!(ent.actor_can_read_notes(etab, m.prof_all, prof, m.all[0]).await.unwrap());
    assert!(!ent.actor_can_read_notes(etab, m.prof_all, prof, m.esp[0]).await.unwrap());

    // ── Cahier de textes : le groupe oui ; la classe entière non (pas de service de classe).
    let a = crate::ref_commun::Acteur { etablissement_id: etab, user_id: m.prof_all, role: prof };
    let cible_groupe = Cible { classe_id: None, groupe_id: Some(m.allemand) };
    let cible_autre = Cible { classe_id: None, groupe_id: Some(m.espagnol) };
    let cible_classe = Cible { classe_id: Some(m.classe_id), groupe_id: None };
    assert!(ent.cahier_peut(&a, Action::CahierEcrire, cible_groupe).await.unwrap());
    assert!(!ent.cahier_peut(&a, Action::CahierEcrire, cible_autre).await.unwrap());
    assert!(!ent.cahier_peut(&a, Action::CahierEcrire, cible_classe).await.unwrap());
    assert!(ent.autoriser_eleve(etab, m.prof_all, prof, Action::CahierLire, m.all[0]).await.unwrap());
    assert!(!ent.autoriser_eleve(etab, m.prof_all, prof, Action::CahierLire, m.esp[0]).await.unwrap());
    // Le professeur principal garde la classe.
    let p = crate::ref_commun::Acteur { etablissement_id: etab, user_id: m.prof_principal, role: prof };
    assert!(ent.cahier_peut(&p, Action::CahierEcrire, cible_classe).await.unwrap());

    // ── Fiche élève.
    let r = ent.get_eleve_for_actor(etab, m.prof_all, prof, m.all[0]).await;
    assert!(matches!(r, Ok(Some(_))), "fiche groupe allemand : {r:?}");
    let r = ent.get_eleve_for_actor(etab, m.prof_all, prof, m.esp[0]).await;
    assert!(!matches!(r, Ok(Some(_))), "fiche groupe espagnol : {r:?}");
    // La direction voit toute la classe.
    let r = ent
        .get_eleve_for_actor(etab, cast.direction.user_id, Role::Direction.as_str(), m.esp[0])
        .await;
    assert!(matches!(r, Ok(Some(_))));
}

/// D-022 (1) : un professeur n'exporte jamais ; direction et secrétariat gardent l'accès.
#[tokio::test]
async fn d022_exports_officiels_refuses_au_professeur() {
    let Some((ent, cast)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = cast.etablissement_id;
    let prof = (Role::Prof.as_str(), cast.profs[0].user_id);
    let periode = ent.list_note_periodes(etab).await.unwrap()[0].id;
    let jour = today_utc();
    let classe = "4B".to_string();
    let lsl = || domain_eleve::ExporterLsl {
        etablissement_id: etab,
        periode_id: periode,
        classe: Some(classe.clone()),
        jour_reference: jour,
    };
    let lsun = || domain_eleve::ExporterLsun {
        etablissement_id: etab,
        periode_id: periode,
        classe: Some(classe.clone()),
        jour_reference: jour,
    };
    let onde = || domain_eleve::ExporterOnde {
        etablissement_id: etab,
        classe: Some(classe.clone()),
        jour_reference: jour,
    };
    let notanet = || domain_eleve::ExporterNotanet {
        etablissement_id: etab,
        periode_id: periode,
        classe: classe.clone(),
    };
    let parcoursup = || domain_eleve::ExporterParcoursup {
        etablissement_id: etab,
        annee_scolaire: "2026-2027".into(),
        classe: classe.clone(),
    };
    let interdit = |r: &Result<String, EntDbError>| matches!(r, Err(EntDbError::Forbidden));
    // Professeur : refusé partout, même sur la classe qu'il enseigne.
    assert!(interdit(&ent.exporter_lsl_csv_for_actor(prof.0, prof.1, lsl()).await));
    assert!(interdit(&ent.exporter_lsun_csv_for_actor(prof.0, prof.1, lsun()).await));
    assert!(interdit(&ent.exporter_onde_csv_for_actor(prof.0, prof.1, onde()).await));
    assert!(interdit(&ent.exporter_notanet_csv_for_actor(prof.0, prof.1, notanet(), jour).await));
    assert!(interdit(&ent.exporter_parcoursup_csv_for_actor(prof.0, prof.1, parcoursup(), jour).await));
    // CPE : pas non plus (ne figurait pas dans les exports).
    let cpe = (Role::Cpe.as_str(), cast.cpe.user_id);
    assert!(interdit(&ent.exporter_lsl_csv_for_actor(cpe.0, cpe.1, lsl()).await));
    // Direction et secrétariat : accès conservé.
    for (role, uid) in [
        (Role::Direction.as_str(), cast.direction.user_id),
        (Role::Administratif.as_str(), cast.administratif.user_id),
    ] {
        assert!(!interdit(&ent.exporter_lsl_csv_for_actor(role, uid, lsl()).await), "{role} LSL");
        assert!(!interdit(&ent.exporter_lsun_csv_for_actor(role, uid, lsun()).await), "{role} LSUN");
        assert!(!interdit(&ent.exporter_onde_csv_for_actor(role, uid, onde()).await), "{role} ONDE");
        assert!(!interdit(&ent.exporter_notanet_csv_for_actor(role, uid, notanet(), jour).await), "{role} Notanet");
        assert!(
            !interdit(&ent.exporter_parcoursup_csv_for_actor(role, uid, parcoursup(), jour).await),
            "{role} Parcoursup"
        );
    }
}
