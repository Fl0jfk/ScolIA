//! Socle 20, lot C6 : continuité (CC-11 à CC-30, CC-69, R-C4), contre la base locale de test (feature `labo`).
//!
//! Sous-module de `tests_cahier` (fixture `Fx`). Aucun résultat ne dépend de l'heure réelle : les séances sont datées par le
//! jour de l'établissement, l'annulation et le passage d'une séance dans le passé se posent par SQL, les publications échues
//! se forcent. Chaque test ne vérifie que ses propres lignes (établissement propre à la fixture).

use super::*;
use chrono::{DateTime, Duration, Utc};
use ent_types::cahier_c2::*;
use ent_types::cahier_c3::AgendaDto;
use ent_types::cahier_c6::*;

async fn jour(fx: &Fx) -> NaiveDate {
    fx.ent.jour_etab(fx.cast.etablissement_id).await.unwrap()
}

async fn ouvre(fx: &Fx, a_partir_de: NaiveDate) -> NaiveDate {
    sqlx::query_scalar(
        "SELECT d::date FROM generate_series($2::date, $2::date + 60, interval '1 day') AS d
          WHERE NOT ent_cdt_jour_ferme($1, d::date) ORDER BY 1 LIMIT 1",
    )
    .bind(fx.cast.etablissement_id)
    .bind(a_partir_de)
    .fetch_one(fx.ent.pool())
    .await
    .unwrap()
}

async fn cours_a(fx: &Fx) -> Uuid {
    sqlx::query_scalar("SELECT cours_id FROM ent_service_enseignement WHERE id = $1").bind(fx.service_a).fetch_one(fx.ent.pool()).await.unwrap()
}

async fn sql(fx: &Fx, requete: &str, id: Uuid) {
    sqlx::query(requete).bind(id).execute(fx.ent.pool()).await.unwrap();
}

async fn annuler(fx: &Fx, seance: Uuid) {
    sql(fx, "UPDATE ent_edt_seance SET statut = 'annulee' WHERE id = $1", seance).await;
}

fn corps_entree(contenu: &str) -> EntreeBody {
    EntreeBody { titre: None, contenu: contenu.into(), notions: vec![], publication: "brouillon".into(), publie_a: None, publie_jour: None, publie_heure: None, sans_avis: false, motif: None }
}

fn corps_travail(cours: Uuid, seance_echeance: Option<Uuid>, jour_echeance: Option<NaiveDate>) -> TravailC2Body {
    TravailC2Body {
        cours_ids: vec![cours],
        seance_donnee_id: None,
        echeance_seance_id: seance_echeance,
        echeance_prochaine_seance: false,
        echeance_jour: jour_echeance,
        echeance_heure: jour_echeance.and(NaiveTime::from_hms_opt(18, 0, 0)),
        nature: "exercices".into(),
        titre: format!("Report {}", &Uuid::new_v4().to_string()[..12]),
        consigne: "À faire.".into(),
        duree_estimee_min: Some(20),
        rendu: "aucun".into(),
        retard_accepte: true,
        formats: vec![],
        eleve_ids: vec![],
        ia_autorisee: "non".into(),
        publication: "maintenant".into(),
        publie_a: None,
        publie_jour: None,
        publie_heure: None,
    }
}

async fn rendre_visible(fx: &Fx) {
    sqlx::query(
        "UPDATE ent_cdt_travail SET visible_a = now() - interval '1 minute'
          WHERE etablissement_id = $1 AND ancien_id IS NULL AND etat IN ('programme', 'publie') AND visible_a > now()",
    )
    .bind(fx.cast.etablissement_id)
    .execute(fx.ent.pool())
    .await
    .unwrap();
}

async fn travail_sur_seance(fx: &Fx, seance: Uuid) -> Uuid {
    let cours = cours_a(fx).await;
    let id = fx.ent.cdt_travaux_creer(&fx.prof(), corps_travail(cours, Some(seance), None)).await.unwrap().travaux[0].id;
    rendre_visible(fx).await;
    id
}

/// Deux séances du cours (classe A) : l'une à annuler, l'autre qui la suit.
async fn deux_seances(fx: &Fx, decalage: i64) -> (Uuid, Uuid) {
    let j1 = ouvre(fx, jour(fx).await + Duration::days(decalage)).await;
    let j2 = ouvre(fx, j1 + Duration::days(1)).await;
    let p = fx.cast.profs[0].user_id;
    (fx.seance("CDT-A", p, Some(fx.service_a), j1).await, fx.seance("CDT-A", p, Some(fx.service_a), j2).await)
}

async fn echeance_seance(fx: &Fx, travail: Uuid) -> (Option<Uuid>, DateTime<Utc>) {
    sqlx::query_as("SELECT echeance_seance_id, echeance_le FROM ent_cdt_travail WHERE id = $1").bind(travail).fetch_one(fx.ent.pool()).await.unwrap()
}

fn accepter() -> ReportDecisionBody {
    ReportDecisionBody { decision: "accepter".into(), vers_seance_id: None, confirmer_avance: false, motif: None }
}

// ───────────────────────── §8.4 : propositions de report ─────────────────────────

#[tokio::test]
async fn seance_annulee_propose_un_report_rien_ne_disparait_un_seul_avis() {
    let Some(fx) = fixture().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let cours = cours_a(&fx).await;
    let (s1, s2) = deux_seances(&fx, 3).await;
    let entree = fx.ent.cdt_entree_enregistrer(&fx.prof(), cours, s1, corps_entree("Fractions : le programme prévu")).await.unwrap();
    let t = travail_sur_seance(&fx, s1).await;
    // Tant que la séance a lieu : aucune proposition.
    assert_eq!(fx.ent.cdt_reports_lister(&fx.prof()).await.unwrap().nb_ouvertes, 0);
    annuler(&fx, s1).await;
    let l = fx.ent.cdt_reports_lister(&fx.prof()).await.unwrap();
    assert_eq!(l.nb_ouvertes, 2, "une pour l'entrée, une pour le travail attendu");
    assert!(l.propositions.iter().all(|p| p.vers.as_ref().is_some_and(|v| v.seance_id == s2)), "la prochaine séance du cours");
    let proposition_travail = l.propositions.iter().find(|p| p.objet == "travail").unwrap().clone();
    let proposition_entree = l.propositions.iter().find(|p| p.objet == "entree").unwrap().clone();
    assert!(!proposition_travail.avance_l_echeance);
    // Rien n'a bougé tant que le professeur n'a pas décidé.
    assert_eq!(echeance_seance(&fx, t).await.0, Some(s1));
    // Accepter : l'échéance suit la séance d'arrivée, une version est gardée, un seul avis.
    let avant = fx.nb_evenements("cahier.report_applique").await;
    let r = fx.ent.cdt_report_traiter(&fx.prof(), proposition_travail.id, accepter()).await.unwrap();
    assert_eq!(r.etat, "appliquee");
    assert_eq!(echeance_seance(&fx, t).await.0, Some(s2));
    assert_eq!(fx.nb_evenements("cahier.report_applique").await, avant + 1);
    let versions: i64 = sqlx::query_scalar("SELECT count(*) FROM ent_cdt_travail_version WHERE travail_id = $1 AND motif LIKE 'report%'").bind(t).fetch_one(fx.ent.pool()).await.unwrap();
    assert_eq!(versions, 1);
    fx.ent.cdt_report_traiter(&fx.prof(), proposition_entree.id, accepter()).await.unwrap();
    let seance_entree: Uuid = sqlx::query_scalar("SELECT seance_id FROM ent_cdt_entree WHERE id = $1").bind(entree.id).fetch_one(fx.ent.pool()).await.unwrap();
    assert_eq!(seance_entree, s2, "le contenu prévu suit la séance d'arrivée");
    // Une proposition traitée ne se traite plus, et ne revient pas (idempotence de la détection).
    assert!(est_refus(&fx.ent.cdt_report_traiter(&fx.prof(), proposition_travail.id, accepter()).await.unwrap_err(), RefusMetier::Conflit));
    assert_eq!(fx.ent.cdt_reports_lister(&fx.prof()).await.unwrap().nb_ouvertes, 0);
    // L'élève, la CPE, un autre professeur : jamais de proposition ; la direction les voit et peut décider.
    assert!(est_interdit(&fx.ent.cdt_reports_lister(&fx.eleve(&fx.x)).await.unwrap_err()));
    assert!(est_interdit(&fx.ent.cdt_reports_lister(&fx.parent(&fx.x)).await.unwrap_err()));
    assert!(fx.ent.cdt_reports_lister(&fx.prof2()).await.unwrap().propositions.is_empty(), "sans saisie sur le cours : rien");
}

#[tokio::test]
async fn refus_echeance_avancee_confirmation_et_application_seule_a_l_heure_de_la_seance() {
    let Some(fx) = fixture().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let (s1, s2) = deux_seances(&fx, 3).await;
    let t = travail_sur_seance(&fx, s1).await;
    annuler(&fx, s1).await;
    let l = fx.ent.cdt_reports_lister(&fx.prof()).await.unwrap();
    let p = l.propositions.iter().find(|p| p.objet == "travail").unwrap().clone();
    // Refus : l'échéance reste, motif facultatif ; la proposition ne revient pas.
    let r = fx.ent.cdt_report_traiter(&fx.prof(), p.id, ReportDecisionBody { decision: "refuser".into(), vers_seance_id: None, confirmer_avance: false, motif: Some("le travail est déjà fait".into()) }).await.unwrap();
    assert_eq!(r.etat, "refusee");
    assert_eq!(echeance_seance(&fx, t).await.0, Some(s1));
    fx.ent.cdt_reports_synchroniser(fx.cast.etablissement_id).await.unwrap();
    assert_eq!(fx.ent.cdt_reports_lister(&fx.prof()).await.unwrap().nb_ouvertes, 0);

    // Une échéance qui serait avancée n'est jamais appliquée seule et demande une confirmation explicite (CC-13).
    let (s3, s4) = deux_seances(&fx, 10).await;
    let t2 = travail_sur_seance(&fx, s3).await;
    sqlx::query("UPDATE ent_cdt_travail SET echeance_le = echeance_le + interval '40 days' WHERE id = $1").bind(t2).execute(fx.ent.pool()).await.unwrap();
    annuler(&fx, s3).await;
    let p2 = fx.ent.cdt_reports_lister(&fx.prof()).await.unwrap().propositions.into_iter().find(|p| p.objet_id == t2).unwrap();
    assert!(p2.avance_l_echeance);
    assert!(est_refus(&fx.ent.cdt_report_traiter(&fx.prof(), p2.id, accepter()).await.unwrap_err(), RefusMetier::Conflit));
    // L'heure de la séance passe : la proposition d'une échéance avancée reste ouverte (jamais appliquée seule).
    sqlx::query("UPDATE ent_edt_seance SET jour = ent_aujourdhui_etab(etablissement_id) - 1 WHERE id = $1").bind(s3).execute(fx.ent.pool()).await.unwrap();
    fx.ent.cdt_projeter_echus(fx.cast.etablissement_id).await.unwrap();
    assert_eq!(echeance_seance(&fx, t2).await.0, Some(s3));
    // Confirmée explicitement, elle s'applique.
    let r = fx.ent.cdt_report_traiter(&fx.prof(), p2.id, ReportDecisionBody { decision: "accepter".into(), vers_seance_id: Some(s4), confirmer_avance: true, motif: None }).await.unwrap();
    assert_eq!(r.etat, "appliquee");
    assert_eq!(echeance_seance(&fx, t2).await.0, Some(s4));

    // S20-05 : proposition de travail non traitée à l'heure de la séance : appliquée seule (pas d'échéance avancée).
    let (s5, s6) = deux_seances(&fx, 20).await;
    let t3 = travail_sur_seance(&fx, s5).await;
    annuler(&fx, s5).await;
    fx.ent.cdt_reports_synchroniser(fx.cast.etablissement_id).await.unwrap();
    sqlx::query("UPDATE ent_edt_seance SET jour = ent_aujourdhui_etab(etablissement_id) - 1 WHERE id = $1").bind(s5).execute(fx.ent.pool()).await.unwrap();
    sqlx::query("UPDATE ent_cdt_travail SET echeance_le = ent_cdt_instant_local(etablissement_id, ent_aujourdhui_etab(etablissement_id) - 1, '08:00') WHERE id = $1").bind(t3).execute(fx.ent.pool()).await.unwrap();
    fx.ent.cdt_projeter_echus(fx.cast.etablissement_id).await.unwrap();
    let (vers, _) = echeance_seance(&fx, t3).await;
    assert_eq!(vers, Some(s6));
    let etat: String = sqlx::query_scalar("SELECT etat FROM ent_cdt_report WHERE objet_id = $1").bind(t3).fetch_one(fx.ent.pool()).await.unwrap();
    assert_eq!(etat, "appliquee_auto");
}

#[tokio::test]
async fn annulation_en_masse_un_ecran_traitement_en_lot_et_entree_non_ecrasee() {
    let Some(fx) = fixture().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let cours = cours_a(&fx).await;
    // CC-19 : plusieurs séances annulées d'un coup (grève) : un seul écran, un traitement en lot.
    let mut seances = Vec::new();
    let mut j = ouvre(&fx, jour(&fx).await + Duration::days(5)).await;
    for _ in 0..4 {
        seances.push(fx.seance("CDT-A", fx.cast.profs[0].user_id, Some(fx.service_a), j).await);
        j = ouvre(&fx, j + Duration::days(1)).await;
    }
    let apres = fx.seance("CDT-A", fx.cast.profs[0].user_id, Some(fx.service_a), j).await;
    let mut travaux = Vec::new();
    for s in &seances[..3] {
        travaux.push(travail_sur_seance(&fx, *s).await);
    }
    // L'entrée prévue pour la troisième séance est écrite **avant** son annulation (on n'écrit plus dans une séance annulée).
    let e_origine = fx.ent.cdt_entree_enregistrer(&fx.prof(), cours, seances[2], corps_entree("Contenu prévu mais la séance est annulée")).await.unwrap();
    for s in &seances[..3] {
        annuler(&fx, *s).await;
    }
    let l = fx.ent.cdt_reports_lister(&fx.prof()).await.unwrap();
    assert_eq!(l.nb_ouvertes, 4, "trois travaux attendus et une entrée prévue");
    // Chaque séance annulée a pour cible la séance suivante **non annulée** du cours.
    assert!(l.propositions.iter().all(|p| p.vers.as_ref().is_some_and(|v| v.seance_id == seances[3])), "{:?}", l.propositions);
    let lot = fx.ent.cdt_reports_lot(&fx.prof(), ReportsLotBody { ids: l.propositions.iter().filter(|p| p.objet == "travail").map(|p| p.id).collect(), decision: "accepter".into() }).await.unwrap();
    assert_eq!((lot.appliques, lot.refuses, lot.ignores.len()), (3, 0, 0));
    for t in &travaux {
        assert_eq!(echeance_seance(&fx, *t).await.0, Some(seances[3]));
    }
    // Entrée : la séance d'arrivée a déjà une entrée → rien n'est écrasé, la proposition reste ouverte avec sa raison.
    let e1 = fx.ent.cdt_entree_enregistrer(&fx.prof(), cours, seances[3], corps_entree("Déjà écrit pour la séance d'arrivée")).await.unwrap();
    let _ = fx.ent.cdt_entree_enregistrer(&fx.prof(), cours, apres, corps_entree("Contenu de la dernière")).await.unwrap();
    let l2 = fx.ent.cdt_reports_lister(&fx.prof()).await.unwrap();
    let p = l2.propositions.iter().find(|p| p.objet_id == e_origine.id).unwrap();
    assert_eq!(p.vers.as_ref().unwrap().seance_id, seances[3]);
    let lot2 = fx.ent.cdt_reports_lot(&fx.prof(), ReportsLotBody { ids: vec![p.id], decision: "accepter".into() }).await.unwrap();
    assert_eq!((lot2.appliques, lot2.ignores.len()), (0, 1), "{lot2:?}");
    let contenu: String = sqlx::query_scalar("SELECT contenu FROM ent_cdt_entree WHERE id = $1").bind(e1.id).fetch_one(fx.ent.pool()).await.unwrap();
    assert!(contenu.contains("Déjà écrit"));
    // Un autre professeur ne décide rien ; la direction décide.
    let id = p.id;
    assert!(est_interdit(&fx.ent.cdt_report_traiter(&fx.prof2(), id, accepter()).await.unwrap_err()));
    let r = fx.ent.cdt_report_traiter(&fx.direction(), id, ReportDecisionBody { decision: "refuser".into(), vers_seance_id: None, confirmer_avance: false, motif: None }).await.unwrap();
    assert_eq!(r.etat, "refusee");
}

// ───────────────────────── continuité d'un élève, poste vacant ─────────────────────────

#[tokio::test]
async fn continuite_d_un_eleve_sans_motif_roles_chevauchement_et_travail_pour_lui_seul() {
    let Some(fx) = fixture().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let cours = cours_a(&fx).await;
    let auj = jour(&fx).await;
    let corps = |nature: &str| ContinuiteBody { eleve_id: fx.x.id, du: auj, au: None, cours_ids: vec![cours], nature: nature.into() };
    // Ni l'élève, ni sa famille, ni le professeur qui n'est pas PP n'ouvrent une continuité.
    for ac in [fx.eleve(&fx.x), fx.parent(&fx.x), fx.prof(), fx.compta()] {
        assert!(est_interdit(&fx.ent.cdt_continuite_ouvrir(&ac, corps("absence_longue")).await.unwrap_err()), "{}", ac.role);
    }
    // La CPE ouvre : aucun motif n'existe nulle part.
    let c = fx.ent.cdt_continuite_ouvrir(&fx.cpe(), corps("absence_longue")).await.unwrap();
    assert_eq!(c.cours_ids, vec![cours]);
    // Le professeur voit « élève en continuité à distance », jamais pourquoi (ni la nature).
    let b = fx.ent.cdt_cours_bandeau(&fx.prof(), cours).await.unwrap();
    assert_eq!(b.eleves_en_continuite.len(), 1);
    assert_eq!(b.eleves_en_continuite[0].eleve_id, fx.x.id);
    let json = serde_json::to_string(&b).unwrap();
    assert!(!json.contains("absence") && !json.contains("exclusion") && !json.contains("motif") && !json.contains("nature"), "{json}");
    assert!(est_interdit(&fx.ent.cdt_cours_bandeau(&fx.prof2(), cours).await.unwrap_err()), "un autre professeur n'a pas le bandeau");
    // Chevauchement, cours hors élève, bornes.
    assert!(est_refus(&fx.ent.cdt_continuite_ouvrir(&fx.cpe(), corps("absence_longue")).await.unwrap_err(), RefusMetier::Conflit));
    let cours_b: Uuid = sqlx::query_scalar("SELECT id FROM ent_cours WHERE etablissement_id = $1 AND classe_id = $2 LIMIT 1").bind(fx.cast.etablissement_id).bind(fx.classe_b).fetch_optional(fx.ent.pool()).await.unwrap().unwrap_or_else(Uuid::new_v4);
    let hors = ContinuiteBody { eleve_id: fx.x.id, du: auj, au: None, cours_ids: vec![cours_b], nature: "absence_longue".into() };
    assert!(fx.ent.cdt_continuite_ouvrir(&fx.cpe(), hors).await.is_err());
    assert!(est_refus(&fx.ent.cdt_continuite_ouvrir(&fx.cpe(), ContinuiteBody { eleve_id: fx.w.id, du: auj, au: Some(auj - Duration::days(1)), cours_ids: vec![cours], nature: "absence_longue".into() }).await.unwrap_err(), RefusMetier::Invalide));
    // CDT-206 : le travail pour lui seul est un travail ordinaire à public restreint ; un camarade ne le voit pas.
    let mut tc = corps_travail(cours, None, Some(ouvre(&fx, auj + Duration::days(4)).await));
    tc.eleve_ids = vec![fx.x.id];
    fx.ent.cdt_travaux_creer(&fx.prof(), tc).await.unwrap();
    rendre_visible(&fx).await;
    let ax: AgendaDto = fx.ent.cdt_agenda(&fx.eleve(&fx.x), None, auj, auj + Duration::days(30)).await.unwrap();
    let aw: AgendaDto = fx.ent.cdt_agenda(&fx.eleve(&fx.w), None, auj, auj + Duration::days(30)).await.unwrap();
    assert!(ax.lignes.iter().any(|l| l.titre.starts_with("Report")));
    assert!(!aw.lignes.iter().any(|l| l.titre.starts_with("Report")));
    // PP : ouvre une absence longue, jamais une exclusion (vie scolaire et direction seulement).
    sqlx::query("UPDATE ent_service_enseignement SET professeur_principal = true WHERE id = $1").bind(fx.service_a).execute(fx.ent.pool()).await.unwrap();
    fx.ent.cdt_continuite_clore(&fx.cpe(), c.id, ContinuiteCloreBody { au: auj }).await.unwrap();
    assert!(est_refus(&fx.ent.cdt_continuite_clore(&fx.cpe(), c.id, ContinuiteCloreBody { au: auj }).await.unwrap_err(), RefusMetier::Conflit));
    let cw = fx.ent.cdt_continuite_ouvrir(&fx.prof(), ContinuiteBody { eleve_id: fx.w.id, du: auj, au: None, cours_ids: vec![cours], nature: "absence_longue".into() }).await.unwrap();
    assert!(est_interdit(&fx.ent.cdt_continuite_ouvrir(&fx.prof(), ContinuiteBody { eleve_id: fx.z.id, du: auj, au: None, cours_ids: vec![cours], nature: "exclusion".into() }).await.unwrap_err()));
    let ce = fx.ent.cdt_continuite_ouvrir(&fx.direction(), ContinuiteBody { eleve_id: fx.z.id, du: auj, au: None, cours_ids: vec![cours], nature: "exclusion".into() }).await.unwrap();
    // La nature (exclusion) n'est servie qu'à la vie scolaire ; le PP ne la voit pas.
    let vue_pp = fx.ent.cdt_continuites_lister(&fx.prof()).await.unwrap();
    assert!(vue_pp.iter().all(|c| c.nature.is_none()));
    assert!(vue_pp.iter().any(|c| c.id == cw.id), "le PP voit les continuités de sa classe, sans leur nature");
    let vue_cpe = fx.ent.cdt_continuites_lister(&fx.cpe()).await.unwrap();
    assert!(vue_cpe.iter().any(|c| c.id == ce.id && c.nature.as_deref() == Some("exclusion")));
}

#[tokio::test]
async fn poste_vacant_la_direction_publie_pas_de_cahier_non_tenu_jamais_sur_un_cours_pourvu() {
    let Some(fx) = fixture().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let cours = cours_a(&fx).await;
    let auj = jour(&fx).await;
    let echeance = Some(ouvre(&fx, auj + Duration::days(4)).await);
    // Cours pourvu : la direction ne publie pas à la place du professeur.
    let err = fx.ent.cdt_vacant_publier(&fx.direction(), corps_travail(cours, None, echeance)).await.unwrap_err();
    assert!(est_refus(&err, RefusMetier::Conflit), "{err:?}");
    assert!(est_interdit(&fx.ent.cdt_vacants_lister(&fx.prof()).await.unwrap_err()));
    // Le poste devient vacant (CC-29) : plus aucune intervention en saisie.
    sql_sans_garde(&fx, &format!("UPDATE ent_service_intervenant SET du = ent_aujourdhui_etab(etablissement_id) - 30, au = ent_aujourdhui_etab(etablissement_id) - 1 WHERE service_id = '{}'", fx.service_a), &[]).await;
    let v = fx.ent.cdt_vacants_lister(&fx.direction()).await.unwrap();
    assert!(v.cours.iter().any(|c| c.cours_id == cours));
    let b = fx.ent.cdt_cours_bandeau(&fx.direction(), cours).await.unwrap();
    assert!(b.poste_vacant, "bandeau « professeur en cours de nomination »");
    let ids = fx.ent.cdt_vacant_publier(&fx.direction(), corps_travail(cours, None, echeance)).await.unwrap();
    assert_eq!(ids.len(), 1);
    let contexte: String = sqlx::query_scalar("SELECT contexte FROM ent_cdt_travail WHERE id = $1").bind(ids[0]).fetch_one(fx.ent.pool()).await.unwrap();
    assert_eq!(contexte, "poste_vacant");
    rendre_visible(&fx).await;
    let ag = fx.ent.cdt_agenda(&fx.eleve(&fx.x), None, auj, auj + Duration::days(30)).await.unwrap();
    assert!(ag.lignes.iter().any(|l| l.id == ids[0]), "l'élève voit le travail de la direction");
    // L'ancien titulaire n'écrit plus (plus de saisie) ; la CPE, l'élève et la famille ne publient pas pour un poste vacant.
    for ac in [fx.cpe(), fx.eleve(&fx.x), fx.parent(&fx.x), fx.prof2()] {
        assert!(fx.ent.cdt_vacant_publier(&ac, corps_travail(cours, None, echeance)).await.is_err(), "{}", ac.role);
    }
}

// ───────────────────────── absence du professeur, fermeture ─────────────────────────

#[tokio::test]
async fn travail_en_l_absence_du_professeur_et_a_distance_publie_a_l_heure_de_la_seance() {
    let Some(fx) = fixture().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let (s1, s2) = deux_seances(&fx, 4).await;
    let corps = |ctx: &str| TravailSeanceBody { titre: "Travail en autonomie".into(), consigne: "Lisez le chapitre 3 et résumez-le.".into(), contexte: ctx.into(), duree_estimee_min: Some(30) };
    let r = fx.ent.cdt_travail_seance(&fx.prof(), s1, corps("absence_professeur")).await.unwrap();
    assert_eq!(r.contexte, "absence_professeur");
    let (contexte, speciale, vers): (String, Option<Uuid>, Option<Uuid>) =
        sqlx::query_as("SELECT contexte, seance_speciale_id, echeance_seance_id FROM ent_cdt_travail WHERE id = $1").bind(r.travail_id).fetch_one(fx.ent.pool()).await.unwrap();
    assert_eq!((contexte.as_str(), speciale, vers), ("absence_professeur", Some(s1), Some(s2)));
    assert!(fx.nb_evenements("cahier.travail_absence_depose").await >= 1);
    // « À distance » ne se dépose que pour une séance annulée ; « en son absence » que pour une séance qui a lieu.
    assert!(est_refus(&fx.ent.cdt_travail_seance(&fx.prof(), s1, corps("distance")).await.unwrap_err(), RefusMetier::Conflit));
    annuler(&fx, s1).await;
    assert!(est_refus(&fx.ent.cdt_travail_seance(&fx.prof(), s1, corps("absence_professeur")).await.unwrap_err(), RefusMetier::Conflit));
    let d = fx.ent.cdt_travail_seance(&fx.direction(), s1, corps("distance")).await.unwrap();
    assert_eq!(d.contexte, "distance");
    // Contexte inconnu, élève, famille, autre professeur : refusés.
    assert!(est_refus(&fx.ent.cdt_travail_seance(&fx.prof(), s2, corps("vacances")).await.unwrap_err(), RefusMetier::Invalide));
    for ac in [fx.eleve(&fx.x), fx.parent(&fx.x), fx.prof2(), fx.cpe()] {
        assert!(est_interdit(&fx.ent.cdt_travail_seance(&ac, s2, corps("absence_professeur")).await.unwrap_err()), "{}", ac.role);
    }
    // Sans séance suivante du cours, l'échéance ne peut pas être posée : message clair.
    let seule = fx.seance("CDT-A", fx.cast.profs[0].user_id, Some(fx.service_a), ouvre(&fx, jour(&fx).await + Duration::days(200)).await).await;
    assert!(est_refus(&fx.ent.cdt_travail_seance(&fx.prof(), seule, corps("absence_professeur")).await.unwrap_err(), RefusMetier::Conflit));
}

// ───────────────────────── remplaçant, suppléant ─────────────────────────

#[tokio::test]
async fn remplacant_d_une_seance_cahier_et_pastilles_jamais_les_copies_plus_rien_le_lendemain() {
    let Some(fx) = fixture().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let cours = cours_a(&fx).await;
    let auj = jour(&fx).await;
    let (prof2, e) = (fx.cast.profs[1].user_id, fx.cast.etablissement_id);
    let s = fx.seance("CDT-A", fx.cast.profs[0].user_id, Some(fx.service_a), auj).await;
    let s_autre = fx.seance("CDT-A", fx.cast.profs[0].user_id, Some(fx.service_a), ouvre(&fx, auj + Duration::days(7)).await).await;
    sqlx::query("UPDATE ent_edt_seance SET remplacant_user_id = $2 WHERE id = $1").bind(s).bind(prof2).execute(fx.ent.pool()).await.unwrap();
    // Un fait utile pour X : jamais un diagnostic ni un intitulé de plan.
    sqlx::query("INSERT INTO ent_cdt_amenagement_simule (id, etablissement_id, eleve_id, code, valeur_pct, du) VALUES ($1, $2, $3, 'temps_majore', 33, $4)")
        .bind(Uuid::new_v4())
        .bind(e)
        .bind(fx.x.id)
        .bind(auj - Duration::days(30))
        .execute(fx.ent.pool())
        .await
        .unwrap();
    // Une entrée publiée du cours (lisible du remplaçant).
    let passee = fx.seance("CDT-A", fx.cast.profs[0].user_id, Some(fx.service_a), auj - Duration::days(7)).await;
    let mut pb = corps_entree("Chapitre 2 : les fractions");
    pb.publication = "maintenant".into();
    fx.ent.cdt_entree_enregistrer(&fx.prof(), cours, passee, pb).await.unwrap();
    sqlx::query("UPDATE ent_cdt_entree SET visible_a = now() - interval '1 minute' WHERE etablissement_id = $1").bind(e).execute(fx.ent.pool()).await.unwrap();
    let v = fx.ent.cdt_remplacement_vue(&fx.prof2(), s).await.unwrap();
    assert!(v.entrees.iter().any(|x| x.contenu.contains("fractions")));
    let px = v.eleves.iter().find(|x| x.eleve_id == fx.x.id).unwrap();
    assert_eq!(px.pastilles, vec!["Temps majoré (+33 %)".to_string()]);
    assert!(v.eleves.iter().find(|x| x.eleve_id == fx.w.id).unwrap().pastilles.is_empty());
    let json = serde_json::to_string(&v).unwrap();
    assert!(!json.to_lowercase().contains("pap") && !json.contains("diagnostic") && !json.contains("PPS"), "{json}");
    // Le remplaçant écrit l'entrée de **cette** séance, rien d'autre (CC-23).
    fx.ent.cdt_entree_enregistrer(&fx.prof2(), cours, s, corps_entree("Séance de remplacement")).await.unwrap();
    assert!(est_interdit(&fx.ent.cdt_entree_enregistrer(&fx.prof2(), cours, s_autre, corps_entree("Autre séance")).await.unwrap_err()));
    // Il ne voit ni copies ni retours : le suivi des remises lui est fermé.
    let t = {
        let mut b = corps_travail(cours, None, Some(ouvre(&fx, auj + Duration::days(3)).await));
        b.rendu = "en_ligne".into();
        fx.ent.cdt_travaux_creer(&fx.prof(), b).await.unwrap().travaux[0].id
    };
    rendre_visible(&fx).await;
    assert!(est_interdit(&fx.ent.cdt_suivi(&fx.prof2(), t).await.unwrap_err()));
    // Les autres : jamais cette vue ; le lendemain : plus aucun droit.
    for ac in [fx.prof(), fx.cpe(), fx.eleve(&fx.x)] {
        assert!(est_interdit(&fx.ent.cdt_remplacement_vue(&ac, s).await.unwrap_err()), "{}", ac.role);
    }
    sqlx::query("UPDATE ent_edt_seance SET jour = ent_aujourdhui_etab(etablissement_id) - 1 WHERE id = $1").bind(s).execute(fx.ent.pool()).await.unwrap();
    assert!(est_interdit(&fx.ent.cdt_remplacement_vue(&fx.prof2(), s).await.unwrap_err()));
    // CDT-258 : le titulaire est prévenu d'un travail donné par le remplaçant pour sa séance.
    let s3 = fx.seance("CDT-A", fx.cast.profs[0].user_id, Some(fx.service_a), ouvre(&fx, auj + Duration::days(2)).await).await;
    sqlx::query("UPDATE ent_edt_seance SET remplacant_user_id = $2 WHERE id = $1").bind(s3).bind(prof2).execute(fx.ent.pool()).await.unwrap();
    let mut tr = corps_travail(cours, None, Some(ouvre(&fx, auj + Duration::days(9)).await));
    tr.seance_donnee_id = Some(s3);
    fx.ent.cdt_travaux_creer(&fx.prof2(), tr).await.unwrap();
    rendre_visible(&fx).await;
    let l = fx.ent.cdt_travaux_de_remplacants(&fx.prof()).await.unwrap();
    assert_eq!(l.len(), 1);
    assert!(!l[0].donne_par.contains('@'), "un nom d'affichage, jamais un identifiant de compte");
    assert!(fx.ent.cdt_travaux_de_remplacants(&fx.eleve(&fx.x)).await.is_err());
}

#[tokio::test]
async fn suppleance_accueil_lecture_avant_la_prise_de_poste_et_decision_sur_les_copies_en_attente() {
    let Some(fx) = fixture().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let e = fx.cast.etablissement_id;
    let cours = cours_a(&fx).await;
    let auj = jour(&fx).await;
    let prof2 = fx.cast.profs[1].user_id;
    // Une copie déposée, en attente de correction.
    let mut b = corps_travail(cours, None, Some(ouvre(&fx, auj + Duration::days(3)).await));
    b.rendu = "en_ligne".into();
    let t = fx.ent.cdt_travaux_creer(&fx.prof(), b).await.unwrap().travaux[0].id;
    rendre_visible(&fx).await;
    let echeance: DateTime<Utc> = sqlx::query_scalar("SELECT echeance_le FROM ent_cdt_travail WHERE id = $1").bind(t).fetch_one(fx.ent.pool()).await.unwrap();
    fx.ent
        .cdt_deposer(&fx.eleve(&fx.x), t, ent_db_depot("Ma copie"), echeance - Duration::hours(2))
        .await
        .unwrap();
    // Nommée dans quelques jours : lecture seule, pas de copies (CDT-255) ; ceux qui ne sont pas suppléants n'ont pas l'accueil.
    let agent2 = agent_du_compte(&fx, prof2).await;
    let service = fx
        .ent
        .nommer_suppleant_for_actor(e, fx.cast.direction.user_id, "direction", fx.service_a, &NommerSuppleantBody { agent_id: agent2, du: auj + Duration::days(5), au: None, origine: None, origine_ref: None })
        .await
        .unwrap();
    let supp = service.intervenants.iter().find(|i| i.nature == "suppleant").unwrap().id;
    let avant = fx.ent.cdt_suppleance_accueil(&fx.prof2(), cours).await.unwrap();
    assert!(!avant.saisie_ouverte);
    assert_eq!(avant.copies_en_attente, 0, "pas de copies avant la prise de poste");
    assert!(avant.travaux_en_cours.iter().any(|x| x.id == t));
    assert!(est_interdit(&fx.ent.cdt_copies_decider(&fx.prof2(), cours, CopiesDecisionBody { decision: "je_corrige".into() }).await.unwrap_err()), "pas encore en poste");
    assert!(est_interdit(&fx.ent.cdt_suppleance_accueil(&fx.eleve(&fx.x), cours).await.unwrap_err()));
    // Prise de poste (CC-21) : la saisie s'ouvre, les copies en attente apparaissent et attendent une décision.
    sql_sans_garde(&fx, &format!("UPDATE ent_service_intervenant SET du = '{}' WHERE id = '{supp}'", auj - Duration::days(1)), &[]).await;
    let acc = fx.ent.cdt_suppleance_accueil(&fx.prof2(), cours).await.unwrap();
    assert!(acc.saisie_ouverte);
    assert_eq!(acc.copies_en_attente, 1);
    let band = fx.ent.cdt_cours_bandeau(&fx.prof2(), cours).await.unwrap();
    assert!(band.suppleance_en_cours);
    assert_eq!(band.copies_a_decider, 1, "CC-69 : aucune copie ne reste sans décision");
    // La direction peut décider du sort sans corriger ; seule la suppléante en poste peut « corriger elle-même ».
    assert!(est_interdit(&fx.ent.cdt_copies_decider(&fx.direction(), cours, CopiesDecisionBody { decision: "je_corrige".into() }).await.unwrap_err()));
    assert!(est_refus(&fx.ent.cdt_copies_decider(&fx.prof2(), cours, CopiesDecisionBody { decision: "jamais".into() }).await.unwrap_err(), RefusMetier::Invalide));
    let d = fx.ent.cdt_copies_decider(&fx.prof2(), cours, CopiesDecisionBody { decision: "rendues_sans_correction".into() }).await.unwrap();
    assert_eq!(d.nb_copies, 1);
    let rendu: i64 = sqlx::query_scalar("SELECT count(*) FROM ent_cdt_retour WHERE etablissement_id = $1 AND etat = 'rendu'").bind(e).fetch_one(fx.ent.pool()).await.unwrap();
    assert_eq!(rendu, 1);
    assert_eq!(fx.ent.cdt_cours_bandeau(&fx.prof2(), cours).await.unwrap().copies_a_decider, 0);
    assert!(fx.nb_evenements("cahier.copies_reattribuees").await >= 1);
    // Le titulaire perd la saisie pendant la suppléance (C-D2) : l'accueil n'est pas pour lui.
    assert!(est_interdit(&fx.ent.cdt_suppleance_accueil(&fx.prof(), cours).await.unwrap_err()));
    assert!(est_interdit(&fx.ent.cdt_cours_bandeau(&fx.prof(), cours).await.unwrap_err()));
}

/// Dépôt d'un élève (texte seul) : le test ne dépend que de l'instant passé en paramètre.
fn ent_db_depot(texte: &str) -> crate::cahier_c5::DepotEntree {
    crate::cahier_c5::DepotEntree { texte: Some(texte.to_string()), commentaire: None, fichiers: vec![] }
}
