//! Socle 12, lot AS3 (épisodes, qualification, justificatifs, demandes d'autorisation). Réf. :
//! `docs/socles/12_ASSIDUITE.md` §8.3, §8.4, §10, §17.3, §22.4, §24.1. Critères de fin : reprise (nombre de
//! demi-journées identique avant/après), un épisode multi-séances justifié couvre ses séances (DA-12),
//! aucun motif ni pièce visible du professeur (test de DTO).
//! Chaque test travaille dans un établissement neuf (jeu de labo) et ne vérifie que ses propres lignes.
//! Sans `ENT_DATABASE_URL` : ignorés par absence de base (le test échoue en CI, qui fournit Postgres).

use super::*;
use crate::droits_repo::{Droit, RoleSaisie};
use crate::cahier::FichierDepot;
use chrono::{NaiveDate, NaiveTime};
use domain_eleve::DomainError;
use ent_auth::{Action, Portee, Role};
use ent_object::presign::Presigner;
use uuid::Uuid;

const PDF: &[u8] = b"%PDF-1.4\n1 0 obj\n<< /Type /Catalog >>\nendobj\ntrailer\n<< /Root 1 0 R >>\n%%EOF\n";

async fn labo() -> Option<(PgEnt, LaboCast)> {
    let url = crate::url_guard::url_labo_ou_echec()?;
    let (ent, cast) = crate::tests_rls_stricte::labo_selon_mode(&url, |e| {
        e.install_brain_p0();
        e.installer_presigner(Presigner::local(b"cle-de-test-assiduite-as3-0123456789", 60).unwrap());
    })
    .await;
    Some((ent, cast))
}

fn t(h: u32, m: u32) -> NaiveTime {
    NaiveTime::from_hms_opt(h, m, 0).unwrap()
}

fn introuvable<T: std::fmt::Debug>(r: Result<T, EntDbError>) -> bool {
    matches!(r, Err(EntDbError::Domain(DomainError::NotFound)))
}

fn interdit<T: std::fmt::Debug>(r: Result<T, EntDbError>) -> bool {
    matches!(r, Err(EntDbError::Forbidden))
}

fn invalide<T: std::fmt::Debug>(r: Result<T, EntDbError>) -> bool {
    matches!(r, Err(EntDbError::SaisieInvalide(_)))
}

fn conflit<T: std::fmt::Debug>(r: Result<T, EntDbError>) -> bool {
    matches!(r, Err(EntDbError::ConflitDossier(_)))
}

async fn aujourdhui(ent: &PgEnt, cast: &LaboCast) -> NaiveDate {
    ent.aujourdhui_etablissement(cast.etablissement_id).await.unwrap()
}

async fn n(ent: &PgEnt, sql: &str, a: Uuid, b: Uuid) -> i64 {
    let (c,): (i64,) = sqlx::query_as(sql).bind(a).bind(b).fetch_one(ent.pool_verif()).await.unwrap();
    c
}

fn saisie(etat: &str) -> SaisieLigne {
    SaisieLigne { etat: etat.into(), minutes: None, lieu_code: None }
}

fn correction(etat: &str) -> CorrectionLigne {
    CorrectionLigne {
        etat: etat.into(),
        minutes: None,
        lieu_code: None,
        motif_code: None,
        motif_texte: Some("erreur de saisie".into()),
        version_base: None,
    }
}

/// Appel de 4B (professeur du labo) sur `[debut, fin)` : les `absents` sont marqués absents, les autres
/// restent « non pointés » (présents à la validation). Renvoie `(créneau, appel)`.
async fn appel(
    ent: &PgEnt,
    cast: &LaboCast,
    jour: NaiveDate,
    debut: (u32, u32),
    fin: (u32, u32),
    absents: &[Uuid],
) -> (Uuid, Uuid) {
    let etab = cast.etablissement_id;
    let c = ent
        .seed_creneau_du_jour(etab, "4B", "AS3", cast.prof.user_id, jour, t(debut.0, debut.1), t(fin.0, fin.1), None)
        .await
        .expect("créneau")
        .id;
    let (a, _) = ent.ouvrir_appel(etab, c, jour, cast.prof.user_id, "prof").await.expect("ouvrir");
    for e in absents {
        ent.pointer_ligne_v2(etab, a.id, *e, cast.prof.user_id, "prof", saisie("absent"), "web").await.expect("pointer");
    }
    ent.valider_appel_v2(
        etab,
        a.id,
        cast.prof.user_id,
        "prof",
        ValidationAppel { confirme_non_pointes: false, canal: None },
    )
    .await
    .expect("valider");
    (c, a.id)
}

fn hermione(cast: &LaboCast) -> Uuid {
    cast.eleves.iter().find(|e| e.prenom == "Hermione").expect("Hermione").id
}

fn ginny(cast: &LaboCast) -> Uuid {
    cast.eleves.iter().find(|e| e.prenom == "Ginny").expect("Ginny").id
}

async fn episodes(ent: &PgEnt, cast: &LaboCast, eleve: Uuid) -> Vec<EpisodeAsd> {
    ent.episodes_for_actor(
        cast.etablissement_id,
        cast.cpe.user_id,
        "cpe",
        FiltreEpisodes { eleve_id: Some(eleve), etat: Some("tous".into()), ..Default::default() },
    )
    .await
    .expect("episodes")
}

async fn legacy_statut(ent: &PgEnt, cast: &LaboCast, eleve: Uuid) -> Vec<(Uuid, String)> {
    let jour = aujourdhui(ent, cast).await;
    sqlx::query_as("SELECT id, statut FROM ent_absence WHERE etablissement_id = $1 AND eleve_id = $2 AND debut = $3 ORDER BY created_at, id")
        .bind(cast.etablissement_id)
        .bind(eleve)
        .bind(jour)
        .fetch_all(ent.pool_verif())
        .await
        .unwrap()
}

fn qualifier(q: &str, v: i32) -> QualifierEpisode {
    QualifierEpisode {
        qualification: q.into(),
        version: v,
        nature_code: None,
        motif_id: None,
        libelle_famille_code: None,
        note_interne: None,
        proposition_id: None,
    }
}

async fn poser_nature(ent: &PgEnt, cast: &LaboCast, ep: Uuid, nature: &str) {
    sqlx::query("UPDATE ent_asd_absence SET nature_code = $3, version = version + 1 WHERE etablissement_id = $1 AND id = $2")
        .bind(cast.etablissement_id)
        .bind(ep)
        .bind(nature)
        .execute(ent.pool_verif())
        .await
        .unwrap();
}

/// Rôle libre « AED » : fait l'appel, lit les motifs, propose sans qualifier, dépose, lit les pièces non médicales.
async fn aed(ent: &PgEnt, cast: &LaboCast) -> (Uuid, &'static str) {
    let (etab, dir) = (cast.etablissement_id, cast.direction.user_id);
    let compte = cast.comptas[0].user_id;
    let droit = |action| Droit { action, portee: Portee::Etablissement, cibles: vec![] };
    let role = ent
        .creer_role_for_actor(etab, dir, "direction", RoleSaisie {
            nom: format!("AED {}", &Uuid::new_v4().simple().to_string()[..6]),
            description: None,
            modele: None,
            droits: vec![
                droit(Action::AppelFaire),
                droit(Action::AbsencesLireMotif),
                droit(Action::AbsencesProposerQualification),
                droit(Action::AbsencesDeclarer),
                droit(Action::AbsencesJustificatifLire),
            ],
        })
        .await
        .unwrap();
    ent.attribuer_role_for_actor(etab, dir, "direction", role.id, compte, None).await.unwrap();
    (compte, Role::Personnel.as_str())
}

async fn declarer_texte(ent: &PgEnt, cast: &LaboCast, jour: NaiveDate) {
    // Amorçage des motifs d'un établissement neuf (le motif « maladie » reçoit sa nature).
    ent.motifs_famille(cast.etablissement_id).await.unwrap();
    ent.declarer_absence_accueil(cast.cpe.user_id, "cpe", DeclarerAbsence {
        etablissement_id: cast.etablissement_id,
        eleve_id: cast.eleve_lea_id,
        debut_jour: jour,
        debut_heure: None,
        fin_jour: jour,
        fin_heure: None,
        motif: "maladie".into(),
        commentaire: Some("Gastro-entérite aiguë du petit".into()),
        declarant_nom: Some("Lily Potter".into()),
        declarant_qualite: Some("mère".into()),
        piece_document_id: None,
    })
    .await
    .expect("déclaration");
}

// ─── Épisodes : contiguïté (§8.3) ───────────────────────────────────────────

#[tokio::test]
async fn as3_deux_seances_contigues_forment_un_seul_episode() {
    let Some((ent, cast)) = labo().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let jour = aujourdhui(&ent, &cast).await;
    let lea = cast.eleve_lea_id;
    appel(&ent, &cast, jour, (8, 0), (9, 0), &[lea]).await;
    appel(&ent, &cast, jour, (9, 0), (10, 0), &[lea]).await;
    let eps = episodes(&ent, &cast, lea).await;
    assert_eq!(eps.len(), 1, "8 h–9 h et 9 h–10 h se touchent : un seul épisode");
    assert_eq!(eps[0].seances.len(), 2);
    assert!(eps[0].seances.iter().all(|s| !s.retiree && s.kind == "seance"));
    assert_eq!(eps[0].etat, "ouvert", "constats du jour : l'épisode est encore ouvert");
    assert_eq!(eps[0].source, "appel");
    // Une absence ancienne (DA-05) et l'historique d'ouverture.
    assert_eq!(legacy_statut(&ent, &cast, lea).await.len(), 1);
    let d = ent.episode_for_actor(cast.etablissement_id, cast.cpe.user_id, "cpe", eps[0].id).await.unwrap();
    assert_eq!(d.historique.first().map(|h| h.origine.as_str()), Some("appel"));
}

#[tokio::test]
async fn as3_present_entre_deux_absences_donne_deux_episodes() {
    let Some((ent, cast)) = labo().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let jour = aujourdhui(&ent, &cast).await;
    let lea = cast.eleve_lea_id;
    appel(&ent, &cast, jour, (8, 0), (9, 0), &[lea]).await;
    appel(&ent, &cast, jour, (9, 0), (10, 0), &[]).await; // présent
    appel(&ent, &cast, jour, (10, 0), (11, 0), &[lea]).await;
    let eps = episodes(&ent, &cast, lea).await;
    assert_eq!(eps.len(), 2, "absent, présent, absent : deux épisodes (le second sera `absence_apres_presence`)");
    assert!(eps.iter().all(|e| e.seances.len() == 1));
    // Une seule absence ancienne par jour (DA-05) : la distinction vit dans l'épisode.
    assert_eq!(legacy_statut(&ent, &cast, lea).await.len(), 1);
    // Une séance n'appartient qu'à un épisode (R-AS3).
    let doublons = n(
        &ent,
        "SELECT count(*) FROM (SELECT seance_ref FROM ent_asd_absence_seance
          WHERE etablissement_id = $1 AND eleve_id = $2 AND retiree_le IS NULL GROUP BY seance_ref HAVING count(*) > 1) x",
        cast.etablissement_id,
        lea,
    )
    .await;
    assert_eq!(doublons, 0);
}

#[tokio::test]
async fn as3_declaration_prevenue_rapprochee_et_texte_reserve_a_la_vie_scolaire() {
    let Some((ent, cast)) = labo().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let jour = aujourdhui(&ent, &cast).await;
    let lea = cast.eleve_lea_id;
    declarer_texte(&ent, &cast, jour).await;
    appel(&ent, &cast, jour, (8, 0), (9, 0), &[lea]).await;
    let eps = episodes(&ent, &cast, lea).await;
    assert_eq!(eps.len(), 1);
    assert!(eps[0].prevenue, "une déclaration couvre la séance : épisode prévenu");
    assert_eq!(n(&ent, "SELECT count(*) FROM ent_asd_absence_declaration WHERE etablissement_id = $1 AND absence_id = $2", cast.etablissement_id, eps[0].id).await, 1);
    assert!(eps[0].nature_code.is_some(), "la nature vient du motif déclaré");
    // Le CPE (qui qualifie) lit le texte libre ; la nature seule suffit à l'AED (R-AS19).
    assert!(eps[0].textes.iter().any(|x| x.contains("Gastro")));
    let (aed_id, base) = aed(&ent, &cast).await;
    let vu = ent.episodes_for_actor(cast.etablissement_id, aed_id, base, FiltreEpisodes { eleve_id: Some(lea), ..Default::default() }).await.unwrap();
    assert_eq!(vu.len(), 1);
    assert!(vu[0].nature_code.is_some());
    assert!(vu[0].textes.is_empty() && vu[0].note_interne.is_none(), "l'AED ne lit pas le texte libre");
    assert!(!serde_json::to_string(&vu).unwrap().contains("Gastro"));
}

#[tokio::test]
async fn as3_correction_retire_l_episode_sans_rien_supprimer() {
    let Some((ent, cast)) = labo().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let jour = aujourdhui(&ent, &cast).await;
    let (etab, lea) = (cast.etablissement_id, cast.eleve_lea_id);
    let (_, a) = appel(&ent, &cast, jour, (8, 0), (9, 0), &[lea]).await;
    ent.corriger_ligne_for_actor(etab, a, lea, cast.prof.user_id, "prof", correction("present"), "web").await.unwrap();
    let eps = episodes(&ent, &cast, lea).await;
    assert_eq!(eps.len(), 1);
    assert_eq!(eps[0].etat, "retire", "plus aucune séance : épisode retiré, pas supprimé (R-AS8)");
    assert_eq!(eps[0].seances.len(), 1, "la ligne de séance reste");
    assert!(eps[0].seances[0].retiree);
    assert_eq!(n(&ent, "SELECT count(*) FROM ent_asd_absence_seance WHERE etablissement_id = $1 AND eleve_id = $2 AND retiree_cause = 'correction'", etab, lea).await, 1);
    let d = ent.episode_for_actor(etab, cast.cpe.user_id, "cpe", eps[0].id).await.unwrap();
    assert!(d.historique.iter().any(|h| h.origine == "correction" && h.etat == "retire"));
    // Oubli finalement : de nouveau absent → nouvel épisode, l'ancien reste retiré.
    ent.corriger_ligne_for_actor(etab, a, lea, cast.prof.user_id, "prof", correction("absent"), "web").await.unwrap();
    let eps = episodes(&ent, &cast, lea).await;
    assert_eq!(eps.len(), 2);
    assert_eq!(eps.iter().filter(|e| e.etat == "retire").count(), 1);
    assert_eq!(eps.iter().filter(|e| e.etat == "ouvert").count(), 1);
    // L'historique d'un épisode est en ajout seul.
    let r = sqlx::query("DELETE FROM ent_asd_absence_etat WHERE etablissement_id = $1").bind(etab).execute(ent.pool_verif()).await;
    assert!(r.is_err());
}

#[tokio::test]
async fn as3_seance_annulee_apres_l_appel_retire_l_episode() {
    let Some((ent, cast)) = labo().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let jour = aujourdhui(&ent, &cast).await;
    let (etab, lea) = (cast.etablissement_id, cast.eleve_lea_id);
    let (c, _) = appel(&ent, &cast, jour, (8, 0), (9, 0), &[lea]).await;
    assert!(ent.neutraliser_appel_seance(etab, c, jour, "annulation_seance", None).await.unwrap());
    let eps = episodes(&ent, &cast, lea).await;
    assert_eq!(eps.len(), 1);
    assert_eq!(eps[0].etat, "retire");
    assert_eq!(n(&ent, "SELECT count(*) FROM ent_asd_absence_seance WHERE etablissement_id = $1 AND eleve_id = $2 AND retiree_cause = 'neutralisation'", etab, lea).await, 1);
}

// ─── Qualification (R-AS5) et projection vers l'ancien circuit ──────────────

#[tokio::test]
async fn as3_reputee_legitime_exige_l_une_des_cinq_natures_de_l131_8() {
    let Some((ent, cast)) = labo().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let jour = aujourdhui(&ent, &cast).await;
    let (etab, lea, cpe) = (cast.etablissement_id, cast.eleve_lea_id, cast.cpe.user_id);
    appel(&ent, &cast, jour, (8, 0), (9, 0), &[lea]).await;
    let ep = episodes(&ent, &cast, lea).await.remove(0);
    // Sans nature, ou avec une nature de classement : refus.
    assert!(invalide(ent.qualifier_episode_for_actor(etab, cpe, "cpe", ep.id, qualifier("reputee_legitime", ep.version)).await));
    let mut q = qualifier("reputee_legitime", ep.version);
    q.nature_code = Some("transport".into());
    assert!(invalide(ent.qualifier_episode_for_actor(etab, cpe, "cpe", ep.id, q).await));
    let mut q = qualifier("reputee_legitime", ep.version);
    q.nature_code = Some("nature_inventee".into());
    assert!(invalide(ent.qualifier_episode_for_actor(etab, cpe, "cpe", ep.id, q).await));
    // Garde de dernier recours côté base : même un accès direct ne contourne pas la règle.
    poser_nature(&ent, &cast, ep.id, "transport").await;
    let r = sqlx::query("UPDATE ent_asd_absence SET etat = 'qualifie', qualification = 'reputee_legitime' WHERE etablissement_id = $1 AND id = $2")
        .bind(etab).bind(ep.id).execute(ent.pool_verif()).await;
    assert!(r.is_err(), "la base refuse « réputée légitime » hors des cinq natures");
    // Une nature légale passe, avec libellé famille par défaut.
    let ep = episodes(&ent, &cast, lea).await.remove(0);
    let mut q = qualifier("reputee_legitime", ep.version);
    q.nature_code = Some("maladie_enfant".into());
    let r = ent.qualifier_episode_for_actor(etab, cpe, "cpe", ep.id, q).await.unwrap();
    assert_eq!((r.etat.as_str(), r.qualification.as_str()), ("qualifie", "reputee_legitime"));
    assert_eq!(r.libelle_famille_code.as_deref(), Some("absence_justifiee"));
    // Libellé famille hors liste, ou contraire à la qualification : refus.
    appel(&ent, &cast, jour, (14, 0), (15, 0), &[lea]).await;
    let ep2 = episodes(&ent, &cast, lea).await.into_iter().find(|e| e.etat == "ouvert").unwrap();
    let mut q = qualifier("non_valable", ep2.version);
    q.libelle_famille_code = Some("absence_justifiee".into());
    assert!(invalide(ent.qualifier_episode_for_actor(etab, cpe, "cpe", ep2.id, q).await));
    let mut q = qualifier("non_valable", ep2.version);
    q.libelle_famille_code = Some("Votre enfant ment".into());
    assert!(invalide(ent.qualifier_episode_for_actor(etab, cpe, "cpe", ep2.id, q).await));
    assert!(invalide(ent.qualifier_episode_for_actor(etab, cpe, "cpe", ep2.id, qualifier("peut_etre", ep2.version)).await));
    let mut q = qualifier("non_valable", ep2.version);
    q.note_interne = Some("x".repeat(501));
    assert!(invalide(ent.qualifier_episode_for_actor(etab, cpe, "cpe", ep2.id, q).await));
}

#[tokio::test]
async fn as3_qualification_projetee_sur_l_ancien_statut_et_requalification_tracee() {
    let Some((ent, cast)) = labo().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let jour = aujourdhui(&ent, &cast).await;
    let (etab, lea, cpe) = (cast.etablissement_id, cast.eleve_lea_id, cast.cpe.user_id);
    appel(&ent, &cast, jour, (8, 0), (9, 0), &[lea]).await;
    let ep = episodes(&ent, &cast, lea).await.remove(0);
    assert_eq!(legacy_statut(&ent, &cast, lea).await[0].1, "a_traiter");
    let mut q = qualifier("excuse_valable", ep.version);
    q.note_interne = Some("certificat vu par la CPE".into());
    let r = ent.qualifier_episode_for_actor(etab, cpe, "cpe", ep.id, q).await.unwrap();
    assert_eq!(r.note_interne.as_deref(), Some("certificat vu par la CPE"));
    assert_eq!(legacy_statut(&ent, &cast, lea).await[0].1, "justifiee", "la famille et le décompte lisent encore l'ancien statut");
    assert_eq!(n(&ent, "SELECT count(*) FROM ent_outbox WHERE etablissement_id = $1 AND aggregate_id = $2 AND type = 'assiduite.absence_qualifiee'", etab, ep.id).await, 1);
    // Déjà qualifié : conflit ; requalification : motif codé obligatoire.
    assert!(conflit(ent.qualifier_episode_for_actor(etab, cpe, "cpe", ep.id, qualifier("non_valable", r.version)).await));
    let rq = |q: &str, v: i32, m: &str| RequalifierEpisode {
        qualification: q.into(),
        version: v,
        motif_code: m.into(),
        nature_code: None,
        libelle_famille_code: None,
        note_interne: None,
    };
    assert!(invalide(ent.requalifier_episode_for_actor(etab, cpe, "cpe", ep.id, rq("non_valable", r.version, "parce_que")).await));
    assert!(conflit(ent.requalifier_episode_for_actor(etab, cpe, "cpe", ep.id, rq("excuse_valable", r.version, "autre")).await));
    assert!(conflit(ent.requalifier_episode_for_actor(etab, cpe, "cpe", ep.id, rq("non_valable", r.version + 7, "autre")).await), "version périmée");
    let r2 = ent.requalifier_episode_for_actor(etab, cpe, "cpe", ep.id, rq("non_valable", r.version, "nouvelle_piece")).await.unwrap();
    assert_eq!(r2.qualification, "non_valable");
    assert_eq!(legacy_statut(&ent, &cast, lea).await[0].1, "refusee");
    let d = ent.episode_for_actor(etab, cpe, "cpe", ep.id).await.unwrap();
    let origines: Vec<_> = d.historique.iter().map(|h| (h.origine.as_str(), h.motif_code.as_deref())).collect();
    assert!(origines.contains(&("requalification", Some("nouvelle_piece"))), "{origines:?}");
    assert_eq!(n(&ent, "SELECT count(*) FROM ent_outbox WHERE etablissement_id = $1 AND aggregate_id = $2 AND type = 'assiduite.absence_requalifiee'", etab, ep.id).await, 1);
    // Le passage « valable » → « non valable » est annoncé comme changement de sens (avis à la famille, AS5).
    let (sens,): (bool,) = sqlx::query_as(
        "SELECT (payload->>'sens_change')::bool FROM ent_outbox WHERE etablissement_id = $1 AND aggregate_id = $2 AND type = 'assiduite.absence_requalifiee'",
    )
    .bind(etab).bind(ep.id).fetch_one(ent.pool_verif()).await.unwrap();
    assert!(sens);
}

#[tokio::test]
async fn as3_decision_de_l_ancien_circuit_s_applique_aux_episodes() {
    let Some((ent, cast)) = labo().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let jour = aujourdhui(&ent, &cast).await;
    let (etab, lea, cpe) = (cast.etablissement_id, cast.eleve_lea_id, cast.cpe.user_id);
    appel(&ent, &cast, jour, (8, 0), (9, 0), &[lea]).await;
    let legacy = legacy_statut(&ent, &cast, lea).await[0].0;
    ent.accepter_justificatif_for_actor(etab, cpe, "cpe", legacy, None).await.unwrap();
    let eps = episodes(&ent, &cast, lea).await;
    assert_eq!((eps[0].etat.as_str(), eps[0].qualification.as_str()), ("qualifie", "excuse_valable"));
    let d = ent.episode_for_actor(etab, cpe, "cpe", eps[0].id).await.unwrap();
    assert!(d.historique.iter().any(|h| h.origine == "projection"));
    // Annulation « vers présent » par l'ancien circuit : l'épisode est retiré.
    appel(&ent, &cast, jour, (14, 0), (15, 0), &[lea]).await;
    let legacy2 = legacy_statut(&ent, &cast, lea).await.into_iter().find(|(_, s)| s == "a_traiter").unwrap().0;
    ent.annuler_absence_vers_present_for_actor(etab, cpe, "cpe", legacy2, None).await.unwrap();
    assert_eq!(episodes(&ent, &cast, lea).await.iter().filter(|e| e.etat == "retire").count(), 1);
}

#[tokio::test]
async fn as3_episode_multi_seances_justifie_couvre_ses_seances() {
    let Some((ent, cast)) = labo().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let jour = aujourdhui(&ent, &cast).await;
    let (etab, lea, cpe) = (cast.etablissement_id, cast.eleve_lea_id, cast.cpe.user_id);
    appel(&ent, &cast, jour, (8, 0), (9, 0), &[lea]).await;
    appel(&ent, &cast, jour, (9, 0), (10, 0), &[lea]).await;
    appel(&ent, &cast, jour, (10, 0), (11, 0), &[lea]).await;
    let ep = episodes(&ent, &cast, lea).await.remove(0);
    assert_eq!(ep.seances.len(), 3);
    ent.qualifier_episode_for_actor(etab, cpe, "cpe", ep.id, qualifier("excuse_valable", ep.version)).await.unwrap();
    // DA-12 : une seule qualification couvre les trois séances ; plus rien « à traiter » côté ancien circuit.
    let (nb, qualifiees): (i64, i64) = sqlx::query_as(
        "SELECT count(*), count(*) FILTER (WHERE e.qualification = 'excuse_valable')
           FROM ent_asd_absence_seance s JOIN ent_asd_absence e ON e.etablissement_id = s.etablissement_id AND e.id = s.absence_id
          WHERE s.etablissement_id = $1 AND s.eleve_id = $2 AND s.retiree_le IS NULL",
    )
    .bind(etab).bind(lea).fetch_one(ent.pool_verif()).await.unwrap();
    assert_eq!((nb, qualifiees), (3, 3));
    assert!(legacy_statut(&ent, &cast, lea).await.iter().all(|(_, s)| s == "justifiee"));
    assert_eq!(episodes(&ent, &cast, lea).await.len(), 1);
}

#[tokio::test]
async fn as3_deux_episodes_d_un_meme_jour_se_projettent_ensemble() {
    let Some((ent, cast)) = labo().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let jour = aujourdhui(&ent, &cast).await;
    let (etab, lea, cpe) = (cast.etablissement_id, cast.eleve_lea_id, cast.cpe.user_id);
    appel(&ent, &cast, jour, (8, 0), (9, 0), &[lea]).await;
    appel(&ent, &cast, jour, (9, 0), (10, 0), &[]).await;
    appel(&ent, &cast, jour, (10, 0), (11, 0), &[lea]).await;
    let eps = episodes(&ent, &cast, lea).await;
    assert_eq!(eps.len(), 2);
    ent.qualifier_episode_for_actor(etab, cpe, "cpe", eps[0].id, qualifier("excuse_valable", eps[0].version)).await.unwrap();
    assert_eq!(legacy_statut(&ent, &cast, lea).await[0].1, "a_traiter", "un épisode attend encore : l'ancien statut ne bouge pas");
    ent.qualifier_episode_for_actor(etab, cpe, "cpe", eps[1].id, qualifier("non_valable", eps[1].version)).await.unwrap();
    assert_eq!(legacy_statut(&ent, &cast, lea).await[0].1, "refusee");
}

#[tokio::test]
async fn as3_consolidation_au_jour_metier_de_l_etablissement() {
    let Some((ent, cast)) = labo().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let jour = aujourdhui(&ent, &cast).await;
    let (etab, lea) = (cast.etablissement_id, cast.eleve_lea_id);
    appel(&ent, &cast, jour, (8, 0), (9, 0), &[lea]).await;
    let ep = episodes(&ent, &cast, lea).await.remove(0);
    assert_eq!(ep.etat, "ouvert");
    // L'épisode d'hier n'est plus « ouvert » : il passe à qualifier à la prochaine lecture de la file.
    sqlx::query("UPDATE ent_asd_absence SET jour_debut = jour_debut - 1, jour_fin = jour_fin - 1 WHERE etablissement_id = $1 AND id = $2")
        .bind(etab).bind(ep.id).execute(ent.pool_verif()).await.unwrap();
    let eps = episodes(&ent, &cast, lea).await;
    assert_eq!(eps[0].etat, "a_qualifier");
    let d = ent.episode_for_actor(etab, cast.cpe.user_id, "cpe", ep.id).await.unwrap();
    assert!(d.historique.iter().any(|h| h.origine == "consolidation"));
}

// ─── Qualification en masse (AS-122) ────────────────────────────────────────

#[tokio::test]
async fn as3_qualification_en_masse_apercu_puis_confirmation() {
    let Some((ent, cast)) = labo().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let jour = aujourdhui(&ent, &cast).await;
    let (etab, cpe) = (cast.etablissement_id, cast.cpe.user_id);
    let (lea, herm, gin) = (cast.eleve_lea_id, hermione(&cast), ginny(&cast));
    appel(&ent, &cast, jour, (8, 0), (9, 0), &[lea, herm, gin]).await;
    let mut ids = Vec::new();
    for e in [lea, herm, gin] {
        ids.push(episodes(&ent, &cast, e).await.remove(0).id);
    }
    poser_nature(&ent, &cast, ids[0], "maladie_enfant").await;
    let sel = |q: &str, ids: &[Uuid], emp: Option<String>| SelectionMasse {
        absence_ids: ids.to_vec(),
        qualification: q.into(),
        libelle_famille_code: None,
        note_interne: None,
        empreinte: emp,
    };
    // Aperçu « réputée légitime » : seul l'épisode de nature légale est retenu.
    let ap = ent.apercu_masse_for_actor(etab, cpe, "cpe", &sel("reputee_legitime", &ids, None)).await.unwrap();
    assert_eq!(ap.retenus, vec![ids[0]]);
    assert_eq!(ap.ecartes.len(), 2);
    assert!(ap.ecartes.iter().all(|(_, r)| r == "nature_non_legitime"));
    // Sans empreinte, pas d'application ; l'aperçu n'a rien écrit.
    assert!(invalide(ent.qualifier_masse_for_actor(etab, cpe, "cpe", sel("reputee_legitime", &ids, None)).await));
    assert_eq!(n(&ent, "SELECT count(*) FROM ent_asd_absence WHERE etablissement_id = $1 AND etat = 'qualifie'", etab, Uuid::nil()).await, 0);
    let b = ent.qualifier_masse_for_actor(etab, cpe, "cpe", sel("reputee_legitime", &ids, Some(ap.empreinte.clone()))).await.unwrap();
    assert_eq!(b.qualifies, vec![ids[0]]);
    // L'état a changé depuis l'aperçu : la même empreinte est refusée.
    assert!(conflit(ent.qualifier_masse_for_actor(etab, cpe, "cpe", sel("reputee_legitime", &ids, Some(ap.empreinte))).await));
    // Deuxième lot : aperçu, modification concurrente, refus ; nouvel aperçu, succès atomique.
    let reste = &ids[1..];
    let ap = ent.apercu_masse_for_actor(etab, cpe, "cpe", &sel("excuse_valable", reste, None)).await.unwrap();
    assert_eq!(ap.retenus.len(), 2);
    poser_nature(&ent, &cast, reste[0], "transport").await;
    assert!(conflit(ent.qualifier_masse_for_actor(etab, cpe, "cpe", sel("excuse_valable", reste, Some(ap.empreinte))).await));
    assert_eq!(n(&ent, "SELECT count(*) FROM ent_asd_absence WHERE etablissement_id = $1 AND etat = 'qualifie'", etab, Uuid::nil()).await, 1, "tout ou rien");
    let ap = ent.apercu_masse_for_actor(etab, cpe, "cpe", &sel("excuse_valable", reste, None)).await.unwrap();
    let b = ent.qualifier_masse_for_actor(etab, cpe, "cpe", sel("excuse_valable", reste, Some(ap.empreinte))).await.unwrap();
    assert_eq!(b.qualifies.len(), 2);
    // Épisode déjà qualifié, ou hors établissement : écarté (jamais qualifié deux fois).
    let ap = ent.apercu_masse_for_actor(etab, cpe, "cpe", &sel("non_valable", &[ids[0], Uuid::new_v4()], None)).await.unwrap();
    assert!(ap.retenus.is_empty());
    assert_eq!(ap.ecartes.len(), 2);
    // Trop large : refusé.
    let large: Vec<Uuid> = (0..201).map(|_| Uuid::new_v4()).collect();
    assert!(invalide(ent.apercu_masse_for_actor(etab, cpe, "cpe", &sel("non_valable", &large, None)).await));
}

// ─── Proposition de l'AED (AS-124) et R-AS19 ────────────────────────────────

#[tokio::test]
async fn as3_l_aed_propose_la_vie_scolaire_decide() {
    let Some((ent, cast)) = labo().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let jour = aujourdhui(&ent, &cast).await;
    let (etab, lea, cpe) = (cast.etablissement_id, cast.eleve_lea_id, cast.cpe.user_id);
    appel(&ent, &cast, jour, (8, 0), (9, 0), &[lea]).await;
    let ep = episodes(&ent, &cast, lea).await.remove(0);
    let (aed_id, base) = aed(&ent, &cast).await;
    // L'AED ne qualifie ni en masse ni à l'unité, mais propose.
    assert!(interdit(ent.qualifier_episode_for_actor(etab, aed_id, base, ep.id, qualifier("excuse_valable", ep.version)).await));
    let sel = SelectionMasse { absence_ids: vec![ep.id], qualification: "excuse_valable".into(), libelle_famille_code: None, note_interne: None, empreinte: None };
    assert!(interdit(ent.apercu_masse_for_actor(etab, aed_id, base, &sel).await));
    let p = ent
        .proposer_qualification_for_actor(etab, aed_id, base, ep.id, ProposerQualification { qualification: "excuse_valable".into(), note: Some("mot des parents reçu".into()) })
        .await
        .unwrap();
    assert_eq!(p.etat, "en_attente");
    let apres = ent.episode_for_actor(etab, cpe, "cpe", ep.id).await.unwrap();
    assert_eq!(apres.episode.etat, "ouvert", "une proposition ne change ni l'état ni l'ancien statut");
    assert_eq!(apres.episode.propositions_en_attente, 1);
    assert_eq!(legacy_statut(&ent, &cast, lea).await[0].1, "a_traiter");
    // La CPE accepte : la proposition est acceptée, la qualification est la sienne.
    let mut q = qualifier("excuse_valable", apres.episode.version);
    q.proposition_id = Some(p.id);
    ent.qualifier_episode_for_actor(etab, cpe, "cpe", ep.id, q).await.unwrap();
    let apres = ent.episode_for_actor(etab, cpe, "cpe", ep.id).await.unwrap();
    assert_eq!(apres.propositions[0].etat, "acceptee");
    assert_eq!(apres.episode.propositions_en_attente, 0);
    // Une proposition sur un épisode déjà qualifié est refusée ; le professeur ne propose pas.
    assert!(conflit(ent.proposer_qualification_for_actor(etab, aed_id, base, ep.id, ProposerQualification { qualification: "non_valable".into(), note: None }).await));
}

// ─── NJ-30 / NJ-35 : le professeur, la famille, la note interne ─────────────

#[tokio::test]
async fn as3_nj30_le_professeur_ne_qualifie_pas_et_ne_voit_que_la_regularisation() {
    let Some((ent, cast)) = labo().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let jour = aujourdhui(&ent, &cast).await;
    let (etab, lea, cpe) = (cast.etablissement_id, cast.eleve_lea_id, cast.cpe.user_id);
    let (prof, autre_prof) = (cast.prof.user_id, cast.profs.iter().map(|p| p.user_id).find(|u| *u != cast.prof.user_id).unwrap());
    declarer_texte(&ent, &cast, jour).await;
    appel(&ent, &cast, jour, (8, 0), (9, 0), &[lea]).await;
    let ep = episodes(&ent, &cast, lea).await.remove(0);
    // Ni la file, ni le détail, ni la qualification, ni la proposition, ni les pièces : « introuvable ».
    assert!(introuvable(ent.episodes_for_actor(etab, prof, "prof", FiltreEpisodes::default()).await));
    assert!(introuvable(ent.episode_for_actor(etab, prof, "prof", ep.id).await));
    assert!(introuvable(ent.qualifier_episode_for_actor(etab, prof, "prof", ep.id, qualifier("excuse_valable", ep.version)).await));
    assert!(introuvable(ent.requalifier_episode_for_actor(etab, prof, "prof", ep.id, RequalifierEpisode { qualification: "non_valable".into(), version: ep.version, motif_code: "autre".into(), nature_code: None, libelle_famille_code: None, note_interne: None }).await));
    assert!(introuvable(ent.proposer_qualification_for_actor(etab, prof, "prof", ep.id, ProposerQualification { qualification: "excuse_valable".into(), note: None }).await));
    assert!(introuvable(ent.justificatifs_for_actor(etab, prof, "prof", ep.id).await));
    assert!(introuvable(ent.demandes_for_actor(etab, prof, "prof", None).await));
    assert_eq!(legacy_statut(&ent, &cast, lea).await[0].1, "a_traiter", "le professeur ne change aucun statut");
    // Sa vue : l'état de régularisation, rien d'autre.
    let vue = ent.episodes_de_mes_seances_for_actor(etab, prof, "prof", jour).await.unwrap();
    assert_eq!(vue.len(), 1);
    assert_eq!(vue[0].regularisation, "a_regulariser");
    ent.qualifier_episode_for_actor(etab, cpe, "cpe", ep.id, qualifier("non_valable", ep.version)).await.unwrap();
    let vue = ent.episodes_de_mes_seances_for_actor(etab, prof, "prof", jour).await.unwrap();
    assert_eq!(vue[0].regularisation, "regularisee", "il voit « régularisée », pas l'issue");
    // Test de DTO : aucun motif, nature, texte, note, libellé, qualification ni pièce dans ce qu'il reçoit.
    let json = serde_json::to_string(&vue).unwrap().to_lowercase();
    for interdit in ["motif", "nature", "texte", "note", "libelle", "qualification", "justificatif", "valable", "gastro", "maladie", "prevenue"] {
        assert!(!json.contains(interdit), "le DTO du professeur ne doit pas contenir « {interdit} » : {json}");
    }
    // Un professeur qui n'a pas fait l'appel ne voit rien.
    assert!(ent.episodes_de_mes_seances_for_actor(etab, autre_prof, "prof", jour).await.unwrap().is_empty());
}

#[tokio::test]
async fn as3_nj35_note_interne_et_texte_jamais_vus_de_la_famille() {
    let Some((ent, cast)) = labo().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let jour = aujourdhui(&ent, &cast).await;
    let (etab, lea, cpe) = (cast.etablissement_id, cast.eleve_lea_id, cast.cpe.user_id);
    declarer_texte(&ent, &cast, jour).await;
    appel(&ent, &cast, jour, (8, 0), (9, 0), &[lea]).await;
    let ep = episodes(&ent, &cast, lea).await.remove(0);
    let mut q = qualifier("non_valable", ep.version);
    q.note_interne = Some("suspicion de fausse signature, voir direction".into());
    q.libelle_famille_code = Some("motif_non_retenu_piece".into());
    ent.qualifier_episode_for_actor(etab, cpe, "cpe", ep.id, q).await.unwrap();
    // Témoin positif : la vie scolaire (qualifier) lit la note et le texte.
    let vs = serde_json::to_string(&episodes(&ent, &cast, lea).await).unwrap();
    assert!(vs.contains("suspicion de fausse signature") && vs.contains("Gastro"));
    // La famille voit un libellé à liste fermée.
    let parent = cast.parent_lea.user_id;
    let fam = ent.episodes_famille_for_actor(etab, parent, "parent", lea).await.unwrap();
    assert_eq!(fam.len(), 1);
    assert_eq!(fam[0].libelle_code.as_deref(), Some("motif_non_retenu_piece"));
    assert_eq!(fam[0].libelle, "Motif non retenu : pièce justificative manquante");
    assert!(!fam[0].a_justifier);
    let json = serde_json::to_string(&fam).unwrap();
    for interdit in ["suspicion", "signature", "Gastro", "note", "nature", "qualification", "non_valable"] {
        assert!(!json.contains(interdit), "la famille ne doit pas voir « {interdit} » : {json}");
    }
    // L'autre famille ne voit rien (même réponse que pour un élève inexistant) ; le professeur non plus.
    assert!(introuvable(ent.episodes_famille_for_actor(etab, cast.parent_tom.user_id, "parent", lea).await));
    assert!(introuvable(ent.episodes_famille_for_actor(etab, cast.parent_lea.user_id, "parent", Uuid::new_v4()).await));
    assert!(introuvable(ent.episodes_famille_for_actor(etab, cast.prof.user_id, "prof", lea).await));
}

// ─── Justificatifs (E5, C3, NJ-38, NJ-39) ───────────────────────────────────

async fn episode_ouvert_de_lea(ent: &PgEnt, cast: &LaboCast) -> EpisodeAsd {
    let jour = aujourdhui(ent, cast).await;
    appel(ent, cast, jour, (8, 0), (9, 0), &[cast.eleve_lea_id]).await;
    episodes(ent, cast, cast.eleve_lea_id).await.remove(0)
}

fn pdf() -> FichierDepot {
    FichierDepot { nom: "certificat-medical-dr-martin.pdf".into(), mime_type: "application/pdf".into(), contenu: PDF.to_vec() }
}

#[tokio::test]
async fn as3_justificatif_medical_lu_de_la_vie_scolaire_seule_avec_journal() {
    let Some((ent, cast)) = labo().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let (etab, parent, cpe, dir) = (cast.etablissement_id, cast.parent_lea.user_id, cast.cpe.user_id, cast.direction.user_id);
    let ep = episode_ouvert_de_lea(&ent, &cast).await;
    let dep = DepotJustificatif { medical: Some(true), ..Default::default() };
    let j = ent.deposer_justificatif_for_actor(etab, parent, "parent", ep.id, dep, Some(pdf())).await.unwrap();
    assert_eq!((j.presente.as_str(), j.medical, j.depose_qualite.as_str()), ("depose", true, "famille"));
    assert!(j.conserve_jusqu_au.is_some(), "conservation d'un an (RA-34)");
    // Clé d'objet générée par le serveur (NJ-39), sans nom de fichier (E14), objet enregistré (E5).
    let (cle, sha): (String, String) = sqlx::query_as("SELECT cle_objet, sha256 FROM ent_asd_justificatif WHERE etablissement_id = $1 AND id = $2")
        .bind(etab).bind(j.id).fetch_one(ent.pool_verif()).await.unwrap();
    assert!(cle.starts_with(&format!("{etab}/assiduite/")), "{cle}");
    assert!(!cle.contains("certificat") && !cle.contains("martin"), "aucun nom de fichier dans la clé");
    assert_eq!(sha.len(), 64);
    let (reg,): (i64,) = sqlx::query_as("SELECT count(*) FROM ent_objet WHERE etablissement_id = $1 AND cle_objet = $2")
        .bind(etab).bind(&cle).fetch_one(ent.pool_verif()).await.unwrap();
    assert_eq!(reg, 1);
    let nom: Option<(String,)> = sqlx::query_as("SELECT column_name::text FROM information_schema.columns WHERE table_name = 'ent_asd_justificatif' AND column_name IN ('nom', 'nom_fichier', 'filename')")
        .fetch_optional(ent.pool_verif()).await.unwrap();
    assert!(nom.is_none(), "aucun nom de fichier n'est conservé");
    // Lecture : CPE et direction (pièces médicales) ; secrétariat, AED, professeur et autre famille : non.
    assert_eq!(ent.justificatifs_for_actor(etab, cpe, "cpe", ep.id).await.unwrap().len(), 1);
    assert_eq!(ent.justificatifs_for_actor(etab, dir, "direction", ep.id).await.unwrap().len(), 1);
    let secretariat = cast.administratif.user_id;
    assert!(ent.justificatifs_for_actor(etab, secretariat, "administratif", ep.id).await.unwrap().is_empty(), "pièce médicale omise pour le secrétariat");
    let (aed_id, base) = aed(&ent, &cast).await;
    assert!(ent.justificatifs_for_actor(etab, aed_id, base, ep.id).await.unwrap().is_empty(), "l'AED ne lit pas les pièces médicales");
    assert!(introuvable(ent.justificatifs_for_actor(etab, cast.prof.user_id, "prof", ep.id).await));
    assert!(introuvable(ent.justificatifs_for_actor(etab, cast.parent_tom.user_id, "parent", ep.id).await));
    assert!(introuvable(ent.lien_justificatif_for_actor(etab, secretariat, "administratif", ep.id, j.id).await), "même réponse que pour une pièce inexistante");
    assert!(introuvable(ent.lien_justificatif_for_actor(etab, aed_id, base, ep.id, j.id).await));
    // Le lien : après contrôle, journalisé avant l'émission (E7) ; 5 minutes au plus.
    let l = ent.lien_justificatif_for_actor(etab, cpe, "cpe", ep.id, j.id).await.unwrap();
    assert!(!l.url.is_empty());
    let lectures = ent.lectures_justificatifs_for_actor(etab, dir, "direction", ep.id).await.unwrap();
    assert_eq!(lectures.len(), 1);
    assert_eq!((lectures[0].1, lectures[0].2), (cpe, true));
    assert!(introuvable(ent.lectures_justificatifs_for_actor(etab, cpe.min(secretariat), "administratif", ep.id).await));
    // Le journal est en ajout seul ; le justificatif ne se supprime pas.
    assert!(sqlx::query("DELETE FROM ent_asd_justificatif_lecture WHERE etablissement_id = $1").bind(etab).execute(ent.pool_verif()).await.is_err());
    assert!(sqlx::query("DELETE FROM ent_asd_justificatif WHERE etablissement_id = $1").bind(etab).execute(ent.pool_verif()).await.is_err());
    // Une pièce non médicale : lisible du secrétariat et de l'AED, lecture journalisée aussi.
    let dep = DepotJustificatif { medical: Some(false), ..Default::default() };
    let j2 = ent.deposer_justificatif_for_actor(etab, parent, "parent", ep.id, dep, Some(pdf())).await.unwrap();
    assert_eq!(ent.justificatifs_for_actor(etab, secretariat, "administratif", ep.id).await.unwrap().len(), 1);
    assert_eq!(ent.justificatifs_for_actor(etab, aed_id, base, ep.id).await.unwrap().len(), 1);
    ent.lien_justificatif_for_actor(etab, secretariat, "administratif", ep.id, j2.id).await.unwrap();
    assert_eq!(n(&ent, "SELECT count(*) FROM ent_asd_justificatif_lecture WHERE etablissement_id = $1 AND medical = false AND lecteur_user_id = $2", etab, secretariat).await, 1);
}

#[tokio::test]
async fn as3_nj38_le_type_est_controle_par_signature_pas_par_le_client() {
    let Some((ent, cast)) = labo().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let (etab, parent) = (cast.etablissement_id, cast.parent_lea.user_id);
    let ep = episode_ouvert_de_lea(&ent, &cast).await;
    // Page HTML déguisée en PDF, exécutable déguisé, fichier vide : refusés, rien n'est écrit.
    for (nom, mime, contenu) in [
        ("certificat.pdf", "application/pdf", b"<html><script>alert(1)</script></html>".to_vec()),
        ("certificat.pdf", "application/pdf", b"MZ\x90\x00\x03\x00\x00\x00".to_vec()),
        ("mot.svg", "image/svg+xml", b"<svg xmlns=\"http://www.w3.org/2000/svg\"><script>1</script></svg>".to_vec()),
        ("vide.pdf", "application/pdf", Vec::new()),
    ] {
        let f = FichierDepot { nom: nom.into(), mime_type: mime.into(), contenu };
        let r = ent.deposer_justificatif_for_actor(etab, parent, "parent", ep.id, DepotJustificatif::default(), Some(f)).await;
        assert!(matches!(r, Err(EntDbError::Fichier(_))), "{nom} : {r:?}");
    }
    assert_eq!(n(&ent, "SELECT count(*) FROM ent_asd_justificatif WHERE etablissement_id = $1 AND absence_id = $2", etab, ep.id).await, 0);
    // Le type annoncé par le client ne fait pas foi : un vrai PDF « annoncé » image est reclassé par signature.
    let f = FichierDepot { nom: "mot.png".into(), mime_type: "image/png".into(), contenu: PDF.to_vec() };
    let r = ent.deposer_justificatif_for_actor(etab, parent, "parent", ep.id, DepotJustificatif::default(), Some(f)).await;
    if let Ok(j) = r {
        assert_eq!(j.mime.as_deref(), Some("application/pdf"), "le type vient de la signature");
    }
}

#[tokio::test]
async fn as3_justificatif_texte_vu_non_conserve_et_retrait() {
    let Some((ent, cast)) = labo().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let (etab, parent, cpe) = (cast.etablissement_id, cast.parent_lea.user_id, cast.cpe.user_id);
    let ep = episode_ouvert_de_lea(&ent, &cast).await;
    // Rien déposé : refusé. Texte seul : accepté, texte lu du personnel habilité seulement.
    assert!(invalide(ent.deposer_justificatif_for_actor(etab, parent, "parent", ep.id, DepotJustificatif::default(), None).await));
    let dep = DepotJustificatif { texte: Some("Rendez-vous chez l'orthodontiste".into()), medical: Some(false), vu_non_conserve: false };
    let j = ent.deposer_justificatif_for_actor(etab, parent, "parent", ep.id, dep, None).await.unwrap();
    assert_eq!(j.presente, "texte");
    let vu_cpe = ent.justificatifs_for_actor(etab, cpe, "cpe", ep.id).await.unwrap();
    assert_eq!(vu_cpe[0].texte.as_deref(), Some("Rendez-vous chez l'orthodontiste"));
    let vu_parent = ent.justificatifs_for_actor(etab, parent, "parent", ep.id).await.unwrap();
    assert_eq!(vu_parent.len(), 1);
    assert!(vu_parent[0].texte.is_none());
    // « Vu, non conservé » : le personnel seulement, aucune pièce, aucune clé.
    let dep = DepotJustificatif { vu_non_conserve: true, ..Default::default() };
    assert!(interdit(ent.deposer_justificatif_for_actor(etab, parent, "parent", ep.id, dep.clone(), None).await));
    let j2 = ent.deposer_justificatif_for_actor(etab, cpe, "cpe", ep.id, dep.clone(), None).await.unwrap();
    assert_eq!((j2.presente.as_str(), j2.depose_qualite.as_str()), ("vu_non_conserve", "vie_scolaire"));
    assert!(invalide(ent.deposer_justificatif_for_actor(etab, cpe, "cpe", ep.id, dep, Some(pdf())).await), "pas de pièce avec « vu, non conservé »");
    let (cle,): (Option<String>,) = sqlx::query_as("SELECT cle_objet FROM ent_asd_justificatif WHERE etablissement_id = $1 AND id = $2")
        .bind(etab).bind(j2.id).fetch_one(ent.pool_verif()).await.unwrap();
    assert!(cle.is_none());
    // Retrait : le dépositaire (ou qui qualifie) ; un autre responsable ne retire pas.
    assert!(introuvable(ent.retirer_justificatif_for_actor(etab, cast.parent_tom.user_id, "parent", ep.id, j.id).await));
    let r = ent.retirer_justificatif_for_actor(etab, parent, "parent", ep.id, j.id).await.unwrap();
    assert_eq!(r.presente, "retire");
    assert_eq!(n(&ent, "SELECT count(*) FROM ent_asd_justificatif WHERE etablissement_id = $1 AND presente = 'retire'", etab, Uuid::nil()).await, 1);
    // L'autre famille ne dépose pas non plus (introuvable, comme pour un épisode inexistant).
    let dep = DepotJustificatif { texte: Some("x".into()), ..Default::default() };
    assert!(introuvable(ent.deposer_justificatif_for_actor(etab, cast.parent_tom.user_id, "parent", ep.id, dep.clone(), None).await));
    assert!(introuvable(ent.deposer_justificatif_for_actor(etab, parent, "parent", Uuid::new_v4(), dep, None).await));
}

// ─── Demandes d'autorisation d'absence (AS-125) ─────────────────────────────

#[tokio::test]
async fn as3_demande_accordee_prerempli_l_appel_et_excuse_d_office() {
    let Some((ent, cast)) = labo().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let jour = aujourdhui(&ent, &cast).await;
    let (etab, parent, dir, lea) = (cast.etablissement_id, cast.parent_lea.user_id, cast.direction.user_id, cast.eleve_lea_id);
    let motif = ent.motifs_famille(etab).await.unwrap().remove(0);
    let demande = |j: NaiveDate, f: NaiveDate| DemandeAutorisation { eleve_id: lea, jour_debut: j, jour_fin: f, heure_debut: None, heure_fin: None, motif_id: motif.id };
    // Passé, trop long, motif inconnu, autre famille : refusés.
    assert!(invalide(ent.demander_autorisation_for_actor(etab, parent, "parent", demande(jour - chrono::Duration::days(1), jour)).await));
    assert!(invalide(ent.demander_autorisation_for_actor(etab, parent, "parent", demande(jour, jour + chrono::Duration::days(121))).await));
    let mut mauvais = demande(jour, jour);
    mauvais.motif_id = Uuid::new_v4();
    assert!(invalide(ent.demander_autorisation_for_actor(etab, parent, "parent", mauvais).await));
    assert!(introuvable(ent.demander_autorisation_for_actor(etab, cast.parent_tom.user_id, "parent", demande(jour, jour)).await));
    assert!(introuvable(ent.demander_autorisation_for_actor(etab, cast.prof.user_id, "prof", demande(jour, jour)).await));
    let d = ent.demander_autorisation_for_actor(etab, parent, "parent", demande(jour, jour)).await.unwrap();
    assert_eq!(d.etat, "en_attente");
    assert!(conflit(ent.demander_autorisation_for_actor(etab, parent, "parent", demande(jour, jour)).await), "doublon sur la période");
    // File : la direction et la vie scolaire la voient ; la famille voit la sienne ; l'autre famille et le professeur non.
    assert_eq!(ent.demandes_for_actor(etab, dir, "direction", None).await.unwrap().len(), 1);
    assert_eq!(ent.demandes_for_actor(etab, parent, "parent", Some(lea)).await.unwrap().len(), 1);
    assert!(introuvable(ent.demandes_for_actor(etab, cast.parent_tom.user_id, "parent", Some(lea)).await));
    assert!(introuvable(ent.demandes_for_actor(etab, parent, "parent", None).await));
    // Décision : la direction seule ; un refus porte un code ; le professeur et la CPE ne décident pas.
    let dec = |x: &str, c: Option<&str>| DecisionDemande { decision: x.into(), decision_code: c.map(str::to_string) };
    assert!(introuvable(ent.decider_demande_for_actor(etab, cast.prof.user_id, "prof", d.id, dec("accordee", None)).await));
    assert!(introuvable(ent.decider_demande_for_actor(etab, cast.cpe.user_id, "cpe", d.id, dec("accordee", None)).await));
    assert!(invalide(ent.decider_demande_for_actor(etab, dir, "direction", d.id, dec("refusee", None)).await));
    assert!(invalide(ent.decider_demande_for_actor(etab, dir, "direction", d.id, dec("refusee", Some("parce_que"))).await));
    let t1 = ent.decider_demande_for_actor(etab, dir, "direction", d.id, dec("transmise_dasen", None)).await.unwrap();
    assert_eq!(t1.etat, "transmise_dasen");
    assert!(conflit(ent.decider_demande_for_actor(etab, dir, "direction", d.id, dec("transmise_dasen", None)).await));
    let a = ent.decider_demande_for_actor(etab, dir, "direction", d.id, dec("accordee", None)).await.unwrap();
    assert_eq!((a.etat.as_str(), a.decision_code.as_deref()), ("accordee", Some("accordee")));
    assert!(conflit(ent.decider_demande_for_actor(etab, dir, "direction", d.id, dec("refusee", Some("refusee_autre"))).await), "décision définitive");
    assert_eq!(n(&ent, "SELECT count(*) FROM ent_asd_situation WHERE etablissement_id = $1 AND eleve_id = $2 AND nature = 'autorisation'", etab, lea).await, 1);
    // L'appel du jour est pré-rempli ; à la validation, l'absence est excusée d'office.
    let c = ent.seed_creneau_du_jour(etab, "4B", "AS3", cast.prof.user_id, jour, t(8, 0), t(9, 0), None).await.unwrap().id;
    let (ap, lignes) = ent.ouvrir_appel(etab, c, jour, cast.prof.user_id, "prof").await.unwrap();
    assert!(lignes.iter().any(|l| l.eleve_id == lea));
    let v = ent.appel_v2_lu(etab, ap.id, cast.prof.user_id, "prof").await.unwrap();
    let l = v.lignes.iter().find(|l| l.eleve_id == lea).unwrap();
    assert!(l.pre_remplie && l.etat == "absent", "autorisation = absent prévenu, jamais sans qu'un adulte confirme");
    assert!(!serde_json::to_string(l).unwrap().to_lowercase().contains("fete") , "aucun motif dans l'appel");
    ent.valider_appel_v2(etab, ap.id, cast.prof.user_id, "prof", ValidationAppel { confirme_non_pointes: false, canal: None }).await.unwrap();
    let eps = episodes(&ent, &cast, lea).await;
    assert_eq!(eps.len(), 1);
    assert_eq!((eps[0].etat.as_str(), eps[0].qualification.as_str()), ("qualifie", "excuse_valable"));
    assert!(eps[0].prevenue);
    assert_eq!(legacy_statut(&ent, &cast, lea).await[0].1, "justifiee");
    // Retrait par le seul demandeur, tant qu'elle attend.
    let motif2 = DemandeAutorisation { eleve_id: lea, jour_debut: jour + chrono::Duration::days(3), jour_fin: jour + chrono::Duration::days(3), heure_debut: None, heure_fin: None, motif_id: motif.id };
    let d2 = ent.demander_autorisation_for_actor(etab, parent, "parent", motif2).await.unwrap();
    assert!(introuvable(ent.retirer_demande_for_actor(etab, cast.parent_tom.user_id, d2.id).await));
    assert_eq!(ent.retirer_demande_for_actor(etab, parent, d2.id).await.unwrap().etat, "retiree");
    assert!(introuvable(ent.retirer_demande_for_actor(etab, parent, d.id).await), "une demande décidée ne se retire plus");
}

// ─── Séparation, établissements, droits par défaut ──────────────────────────

#[tokio::test]
async fn as3_matrice_de_separation_autre_etablissement_et_famille() {
    let Some((ent, cast)) = labo().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let (etab, lea, cpe) = (cast.etablissement_id, cast.eleve_lea_id, cast.cpe.user_id);
    let ep = episode_ouvert_de_lea(&ent, &cast).await;
    let autre = crate::tests_rls_stricte::seed_supplementaire(&ent).await;
    // Le CPE d'un autre établissement ne lit ni ne qualifie un épisode d'ailleurs : même réponse que « inexistant ».
    assert!(introuvable(ent.episode_for_actor(autre.etablissement_id, autre.cpe.user_id, "cpe", ep.id).await));
    assert!(introuvable(ent.qualifier_episode_for_actor(autre.etablissement_id, autre.cpe.user_id, "cpe", ep.id, qualifier("excuse_valable", ep.version)).await));
    assert!(introuvable(ent.justificatifs_for_actor(autre.etablissement_id, autre.cpe.user_id, "cpe", ep.id).await));
    assert!(ent.episodes_for_actor(autre.etablissement_id, autre.cpe.user_id, "cpe", FiltreEpisodes { eleve_id: Some(lea), ..Default::default() }).await.unwrap().is_empty());
    assert!(introuvable(ent.episode_for_actor(etab, cpe, "cpe", Uuid::new_v4()).await));
    // La famille, l'élève et le personnel sans rôle ne qualifient ni ne lisent la file (introuvable).
    for (u, r) in [(cast.parent_lea.user_id, "parent"), (cast.compta.user_id, "compta")] {
        assert!(introuvable(ent.episodes_for_actor(etab, u, r, FiltreEpisodes::default()).await), "{r}");
        assert!(introuvable(ent.qualifier_episode_for_actor(etab, u, r, ep.id, qualifier("excuse_valable", ep.version)).await), "{r}");
        assert!(introuvable(ent.apercu_masse_for_actor(etab, u, r, &SelectionMasse { absence_ids: vec![ep.id], qualification: "excuse_valable".into(), libelle_famille_code: None, note_interne: None, empreinte: None }).await), "{r}");
    }
    // Le CPE ne décide pas d'une autorisation (direction seule) ; la secrétaire ne lit pas les pièces médicales.
    assert!(introuvable(ent.decider_demande_for_actor(etab, cpe, "cpe", Uuid::new_v4(), DecisionDemande { decision: "accordee".into(), decision_code: None }).await));
}

#[test]
fn as3_droits_par_defaut_de_l_assiduite() {
    use ent_auth::modele_par_defaut;
    let a = |r: Role, x: Action| modele_par_defaut(r).iter().any(|(act, _)| *act == x);
    for r in [Role::Direction, Role::Cpe] {
        assert!(a(r, Action::AbsencesJustificatifMedicalLire), "{r:?} lit les pièces médicales");
        assert!(a(r, Action::AbsencesJustificatifLire));
    }
    for r in [Role::Administratif, Role::Admin] {
        assert!(a(r, Action::AbsencesJustificatifLire));
        assert!(!a(r, Action::AbsencesJustificatifMedicalLire), "{r:?} ne lit pas les pièces médicales (C3)");
    }
    for r in [Role::Prof, Role::Parent, Role::Eleve, Role::Compta, Role::Infirmerie] {
        assert!(!a(r, Action::AbsencesJustificatifLire) && !a(r, Action::AbsencesJustificatifMedicalLire), "{r:?}");
        assert!(!a(r, Action::AbsencesAutorisationDecider) && !a(r, Action::AbsencesQualifier), "{r:?}");
    }
    assert!(a(Role::Direction, Action::AbsencesAutorisationDecider));
    assert!(!a(Role::Cpe, Action::AbsencesAutorisationDecider), "la décision d'autorisation est celle de la direction");
}

// ─── Reprise (critère de fin : demi-journées identiques avant/après) ────────

#[tokio::test]
async fn as3_reprise_des_absences_anciennes_est_fidele_et_idempotente() {
    let Some((ent, cast)) = labo().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let (etab, pool) = (cast.etablissement_id, ent.pool_verif());
    let jour = aujourdhui(&ent, &cast).await - chrono::Duration::days(10);
    let eleves = [cast.eleve_lea_id, hermione(&cast), ginny(&cast), cast.eleve_tom_id];
    let statuts = ["a_traiter", "justifiee", "refusee", "annulee_present"];
    let mut anciennes = Vec::new();
    for (i, (e, st)) in eleves.iter().zip(statuts).enumerate() {
        let id = Uuid::new_v4();
        let fin = if i == 1 { jour + chrono::Duration::days(2) } else { jour };
        sqlx::query("INSERT INTO ent_absence (id, etablissement_id, eleve_id, debut, fin, motif, statut) VALUES ($1,$2,$3,$4,$5,'maladie',$6)")
            .bind(id).bind(etab).bind(e).bind(jour).bind(fin).bind(st).execute(pool).await.unwrap();
        if i == 1 {
            // Absence historique à la journée : trois jours × deux demi-journées.
            for d in 0..3 {
                for p in ["matin", "apres_midi"] {
                    sqlx::query("INSERT INTO ent_absence_seance (id, etablissement_id, eleve_id, absence_id, jour, kind, periode, seance_source, source) VALUES ($1,$2,$3,$4,$5,'demi_journee',$6,'reprise','reprise')")
                        .bind(Uuid::new_v4()).bind(etab).bind(e).bind(id).bind(jour + chrono::Duration::days(d)).bind(p).execute(pool).await.unwrap();
                }
            }
        } else {
            sqlx::query("INSERT INTO ent_absence_seance (id, etablissement_id, eleve_id, absence_id, jour, kind, seance_ref, seance_source, heure_debut, heure_fin, source, prevenue) VALUES ($1,$2,$3,$4,$5,'seance',$6,'edt_creneau',$7,$8,'appel',$9)")
                .bind(Uuid::new_v4()).bind(etab).bind(e).bind(id).bind(jour).bind(Uuid::new_v4()).bind(t(8, 0)).bind(t(9, 0)).bind(i == 0).execute(pool).await.unwrap();
        }
        anciennes.push(id);
    }
    // La reprise est rejouée deux fois : aucun doublon (E8).
    // On rejoue seulement la section de reprise (les INSERT idempotents) : rejouer le DDL (déclencheurs,
    // verrous exclusifs) en parallèle d'autres tests de la même base provoquerait des interblocages.
    let script = include_str!("../../../migrations/2244_assiduite_as3_episodes.sql");
    let debut = script.find("-- ── Reprise des absences anciennes").expect("section de reprise");
    let fin = script.find("-- ── Projection de l'ancien circuit").expect("section de projection");
    for _ in 0..2 {
        sqlx::raw_sql(&script[debut..fin]).execute(pool).await.expect("reprise rejouable (E8)");
    }
    // Mêmes séances et mêmes demi-journées avant et après.
    let avant: Vec<(Uuid, NaiveDate, String, Option<String>, Option<NaiveTime>, Option<NaiveTime>)> = sqlx::query_as(
        "SELECT id, jour, kind, periode, heure_debut, heure_fin FROM ent_absence_seance WHERE etablissement_id = $1 AND absence_id = ANY($2) ORDER BY id",
    ).bind(etab).bind(&anciennes).fetch_all(pool).await.unwrap();
    let apres: Vec<(Uuid, NaiveDate, String, Option<String>, Option<NaiveTime>, Option<NaiveTime>)> = sqlx::query_as(
        "SELECT id, jour, kind, periode, heure_debut, heure_fin FROM ent_asd_absence_seance WHERE etablissement_id = $1 AND absence_id IN (SELECT id FROM ent_asd_absence WHERE absence_legacy_id = ANY($2)) ORDER BY id",
    ).bind(etab).bind(&anciennes).fetch_all(pool).await.unwrap();
    assert_eq!(avant.len(), 3 + 6);
    assert_eq!(avant, apres, "séances et demi-journées reprises à l'identique");
    let demi = |l: &[(Uuid, NaiveDate, String, Option<String>, Option<NaiveTime>, Option<NaiveTime>)]| {
        let mut v: Vec<_> = l.iter().filter(|x| x.2 == "demi_journee").map(|x| (x.1, x.3.clone())).collect();
        v.sort();
        v.dedup();
        v.len()
    };
    assert_eq!(demi(&avant), demi(&apres));
    assert_eq!(demi(&apres), 6);
    // Un épisode par absence ancienne, statut → état.
    let eps: Vec<EpisodeAsd> = episodes(&ent, &cast, cast.eleve_lea_id).await.into_iter().filter(|e| e.jour_debut == jour).collect();
    assert_eq!(eps.len(), 1);
    assert_eq!((eps[0].etat.as_str(), eps[0].source.as_str(), eps[0].prevenue), ("a_qualifier", "reprise", true));
    let lignes: Vec<(Uuid, String, String)> = sqlx::query_as(
        "SELECT absence_legacy_id, etat, qualification FROM ent_asd_absence WHERE etablissement_id = $1 AND absence_legacy_id = ANY($2) ORDER BY absence_legacy_id",
    ).bind(etab).bind(&anciennes).fetch_all(pool).await.unwrap();
    assert_eq!(lignes.len(), 4, "une seule fois malgré deux rejeux");
    let de = |id: Uuid| lignes.iter().find(|l| l.0 == id).map(|l| (l.1.as_str(), l.2.as_str())).unwrap();
    assert_eq!(de(anciennes[0]), ("a_qualifier", "en_attente"));
    assert_eq!(de(anciennes[1]), ("qualifie", "excuse_valable"));
    assert_eq!(de(anciennes[2]), ("qualifie", "non_valable"));
    assert_eq!(de(anciennes[3]), ("retire", "en_attente"));
    // La reprise a laissé son historique ; l'épisode repris est qualifiable comme les autres.
    let ep = &eps[0];
    let r = ent.qualifier_episode_for_actor(etab, cast.cpe.user_id, "cpe", ep.id, qualifier("excuse_valable", ep.version)).await.unwrap();
    assert_eq!(r.qualification, "excuse_valable");
    let (st,): (String,) = sqlx::query_as("SELECT statut FROM ent_absence WHERE id = $1").bind(anciennes[0]).fetch_one(pool).await.unwrap();
    assert_eq!(st, "justifiee");
}
