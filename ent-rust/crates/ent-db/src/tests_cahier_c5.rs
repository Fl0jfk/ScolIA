//! Socle 20, lot C5 : rendu et retour (CC-31 à CC-45, R-C9 à R-C13), contre la base locale de test (feature `labo`).
//!
//! Sous-module de `tests_cahier` (fixture `Fx`). Aucun résultat ne dépend de l'heure réelle : les instants de dépôt sont
//! passés explicitement (l'heure de **début** de réception est un paramètre), les échéances se posent par SQL, les jours
//! sont ceux de l'établissement. Chaque test ne vérifie que ses propres lignes (établissement propre à la fixture).

use super::*;
use crate::cahier_c5::DepotEntree;
use chrono::{DateTime, Duration, Utc};
use ent_types::cahier_c2::*;
use ent_types::cahier_c5::*;

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
    sqlx::query_scalar("SELECT cours_id FROM ent_service_enseignement WHERE id = $1")
        .bind(fx.service_a)
        .fetch_one(fx.ent.pool())
        .await
        .unwrap()
}

fn corps(cours: Vec<Uuid>, jour_echeance: NaiveDate, rendu: &str) -> TravailC2Body {
    TravailC2Body {
        cours_ids: cours,
        seance_donnee_id: None,
        echeance_seance_id: None,
        echeance_prochaine_seance: false,
        echeance_jour: Some(jour_echeance),
        echeance_heure: NaiveTime::from_hms_opt(18, 0, 0),
        nature: "exercices".into(),
        titre: format!("Rendu {}", &Uuid::new_v4().to_string()[..12]),
        consigne: "Rédigez et déposez.".into(),
        duree_estimee_min: Some(30),
        rendu: rendu.into(),
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

/// Un travail à rendre en ligne pour la classe A, visible, avec une échéance posée à un instant de référence fixe.
/// Renvoie (travail, échéance).
async fn travail_en_ligne(fx: &Fx, retard_accepte: bool) -> (Uuid, DateTime<Utc>) {
    let cours = cours_a(fx).await;
    let j = ouvre(fx, jour(fx).await + Duration::days(3)).await;
    let mut b = corps(vec![cours], j, "en_ligne");
    b.retard_accepte = retard_accepte;
    let id = fx.ent.cdt_travaux_creer(&fx.prof(), b).await.unwrap().travaux[0].id;
    rendre_visible(fx).await;
    let ech: DateTime<Utc> = sqlx::query_scalar("SELECT echeance_le FROM ent_cdt_travail WHERE id = $1").bind(id).fetch_one(fx.ent.pool()).await.unwrap();
    (id, ech)
}

fn depot(texte: &str, fichiers: Vec<FichierDepot>) -> DepotEntree {
    DepotEntree { texte: Some(texte.to_string()).filter(|t| !t.is_empty()), commentaire: None, fichiers }
}

async fn fixer_echeance(fx: &Fx, travail: Uuid, instant: DateTime<Utc>) {
    sqlx::query("UPDATE ent_cdt_travail SET echeance_le = $2, cloture_ramassage_le = NULL WHERE id = $1")
        .bind(travail)
        .bind(instant)
        .execute(fx.ent.pool())
        .await
        .unwrap();
}

async fn fixer_cloture(fx: &Fx, travail: Uuid, instant: DateTime<Utc>) {
    sqlx::query("UPDATE ent_cdt_travail SET cloture_ramassage_le = $2 WHERE id = $1").bind(travail).bind(instant).execute(fx.ent.pool()).await.unwrap();
}

async fn eleve_depose(fx: &Fx, e: &LaboEleve, t: Uuid, recu: DateTime<Utc>) -> Result<AccuseDto, EntDbError> {
    fx.ent.cdt_deposer(&fx.eleve(e), t, depot("Ma copie", vec![pdf("mon-devoir-secret.pdf")]), recu).await
}

async fn lire(fx: &Fx, e: &LaboEleve, t: Uuid) -> RemiseLueDto {
    fx.ent.cdt_remise_lire(&fx.eleve(e), t, None).await.unwrap()
}

// ───────────────────────── CC-31, R-C9, R-C10 : le dépôt ─────────────────────────

#[tokio::test]
async fn depot_horodate_au_debut_de_reception_accuse_immuable_nom_neutre() {
    let Some(fx) = fixture().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let (t, ech) = travail_en_ligne(&fx, true).await;
    // CC-31 : reçu à 17 h 59 min 58 s pour une échéance à 18 h : début de réception = à l'heure, même si la réception se
    // termine après l'échéance.
    let a1 = eleve_depose(&fx, &fx.x, t, ech - Duration::seconds(2)).await.unwrap();
    assert_eq!(a1.numero, 1);
    assert!(!a1.en_retard);
    assert!(a1.courant);
    assert_eq!(a1.nb_fichiers, 1);
    assert_eq!(a1.empreinte.len(), 64);
    // Aucun nom de fichier n'est stocké ni servi : la pièce s'appelle copie-1.pdf.
    assert_eq!(a1.pieces[0].nom, "copie-1.pdf");
    let noms: i64 = sqlx::query_scalar("SELECT count(*) FROM ent_cdt_piece_jointe WHERE etablissement_id = $1 AND nom ILIKE '%secret%'")
        .bind(fx.cast.etablissement_id)
        .fetch_one(fx.ent.pool())
        .await
        .unwrap();
    assert_eq!(noms, 0);
    // Jour et heure de l'établissement, jamais UTC brut.
    let (j, h): (NaiveDate, NaiveTime) = sqlx::query_as(
        "SELECT (x.l)::date, (x.l)::time FROM (SELECT $2::timestamptz AT TIME ZONE COALESCE((SELECT fuseau FROM ent_etablissement_parametre WHERE etablissement_id = $1), 'Europe/Paris') AS l) x",
    )
    .bind(fx.cast.etablissement_id)
    .bind(a1.recu_le)
    .fetch_one(fx.ent.pool())
    .await
    .unwrap();
    assert_eq!((a1.recu_jour, a1.recu_heure), (j, h));
    assert!(fx.nb_evenements("cahier.travail_rendu").await >= 1);
    // Redépôt après l'échéance : nouveau dépôt, l'ancien reste, « en retard ».
    let a2 = eleve_depose(&fx, &fx.x, t, ech + Duration::seconds(4)).await.unwrap();
    assert_eq!(a2.numero, 2);
    assert!(a2.en_retard);
    let r = lire(&fx, &fx.x, t).await;
    assert_eq!(r.nb_depots, 2);
    assert_eq!(r.depots.len(), 2);
    assert_eq!(r.depots.iter().filter(|d| d.courant).count(), 1);
    assert_eq!(r.statut.as_deref(), Some("en_retard"));
    assert!(fx.nb_evenements("cahier.redepot").await >= 1);
    // R-C9 : un dépôt est immuable (déclencheur d'ajout seul).
    let modif = sqlx::query("UPDATE ent_cdt_depot SET texte = 'trafiqué' WHERE etablissement_id = $1").bind(fx.cast.etablissement_id).execute(fx.ent.pool()).await;
    assert!(modif.is_err());
    let suppr = sqlx::query("DELETE FROM ent_cdt_depot WHERE etablissement_id = $1").bind(fx.cast.etablissement_id).execute(fx.ent.pool()).await;
    assert!(suppr.is_err());
}

#[tokio::test]
async fn fichier_refuse_aucun_depot_signalement_je_n_arrive_pas() {
    let Some(fx) = fixture().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let (t, ech) = travail_en_ligne(&fx, true).await;
    // CC-32 / CC-44 : un faux PDF (HTML actif) est refusé, aucun dépôt n'existe.
    let faux = FichierDepot { nom: "x.pdf".into(), mime_type: "application/pdf".into(), contenu: b"<html><script>1</script></html>".to_vec() };
    let err = fx.ent.cdt_deposer(&fx.eleve(&fx.x), t, depot("", vec![faux]), ech - Duration::hours(1)).await.unwrap_err();
    assert!(matches!(err, EntDbError::Fichier(_)), "{err:?}");
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM ent_cdt_depot WHERE etablissement_id = $1").bind(fx.cast.etablissement_id).fetch_one(fx.ent.pool()).await.unwrap();
    assert_eq!(n, 0);
    // Rien à déposer : refus clair.
    let err = fx.ent.cdt_deposer(&fx.eleve(&fx.x), t, depot("", vec![]), ech - Duration::hours(1)).await.unwrap_err();
    assert!(est_refus(&err, RefusMetier::Invalide));
    // « Je n'arrive pas à déposer » : trace horodatée, le professeur est prévenu (événement + suivi).
    let avant = fx.nb_evenements("cahier.depot_impossible_signale").await;
    let le = fx.ent.cdt_depot_signaler(&fx.eleve(&fx.x), t, SignalementBody { message: Some("fichier refusé".into()) }).await.unwrap();
    assert_eq!(fx.nb_evenements("cahier.depot_impossible_signale").await, avant + 1);
    let suivi = fx.ent.cdt_suivi(&fx.prof(), t).await.unwrap();
    let l = suivi.lignes.iter().find(|l| l.eleve_id == fx.x.id).unwrap();
    assert_eq!(l.signalement_le, Some(le));
    // Le signalement d'un élève hors du public est introuvable (pas de sondage).
    let err = fx.ent.cdt_depot_signaler(&fx.eleve(&fx.y), t, SignalementBody { message: None }).await.unwrap_err();
    assert!(est_introuvable(&err), "{err:?}");
}

#[tokio::test]
async fn retard_non_accepte_cloture_du_ramassage_et_clotures_par_defaut() {
    let Some(fx) = fixture().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let (t, ech) = travail_en_ligne(&fx, false).await;
    let err = eleve_depose(&fx, &fx.x, t, ech + Duration::minutes(1)).await.unwrap_err();
    assert!(est_refus(&err, RefusMetier::Interdit), "{err:?}");
    // Défaut S20-09 : 14 jours après l'échéance ; retard accepté : dépôt en retard possible jusque-là, pas après.
    let (t2, ech2) = travail_en_ligne(&fx, true).await;
    let r = lire(&fx, &fx.x, t2).await;
    assert_eq!(r.cloture_ramassage_le, Some(ech2 + Duration::days(14)));
    assert!(eleve_depose(&fx, &fx.x, t2, ech2 + Duration::days(13)).await.unwrap().en_retard);
    let err = eleve_depose(&fx, &fx.w, t2, ech2 + Duration::days(15)).await.unwrap_err();
    assert!(est_refus(&err, RefusMetier::Conflit), "{err:?}");
    // CDT-201 : le professeur fixe la clôture ; avant l'échéance, refusé.
    let j = ouvre(&fx, jour(&fx).await + Duration::days(40)).await;
    let c = fx.ent.cdt_cloture_fixer(&fx.prof(), t2, ClotureBody { cloture_jour: j, cloture_heure: NaiveTime::from_hms_opt(12, 0, 0).unwrap() }).await.unwrap();
    assert!(c > ech2);
    let err = fx.ent.cdt_cloture_fixer(&fx.prof(), t2, ClotureBody { cloture_jour: jour(&fx).await - Duration::days(30), cloture_heure: NaiveTime::from_hms_opt(12, 0, 0).unwrap() }).await.unwrap_err();
    assert!(est_refus(&err, RefusMetier::Invalide));
    // Un autre professeur (sans saisie sur le cours) ne fixe rien.
    let err = fx.ent.cdt_cloture_fixer(&fx.prof2(), t2, ClotureBody { cloture_jour: j, cloture_heure: NaiveTime::from_hms_opt(12, 0, 0).unwrap() }).await.unwrap_err();
    assert!(est_interdit(&err));
}

#[tokio::test]
async fn brouillon_ne_compte_pas_seul_le_depot_compte() {
    let Some(fx) = fixture().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let (t, ech) = travail_en_ligne(&fx, true).await;
    let r = fx.ent.cdt_brouillon_enregistrer(&fx.eleve(&fx.x), t, DepotEntree { texte: Some("début".into()), commentaire: Some("à relire".into()), fichiers: vec![pdf("a.pdf")] }).await.unwrap();
    assert_eq!(r.etat, "brouillon");
    let b = r.brouillon.expect("brouillon");
    assert_eq!(b.pieces.len(), 1);
    // Le professeur ne voit aucun dépôt ; le statut reste « à faire ».
    let suivi = fx.ent.cdt_suivi(&fx.prof(), t).await.unwrap();
    assert_eq!(suivi.lignes.iter().find(|l| l.eleve_id == fx.x.id).unwrap().nb_depots, 0);
    // Les fichiers du brouillon rejoignent le dépôt.
    let a = fx.ent.cdt_deposer(&fx.eleve(&fx.x), t, DepotEntree { texte: None, commentaire: None, fichiers: vec![pdf("b.pdf")] }, ech - Duration::hours(2)).await.unwrap();
    assert_eq!(a.nb_fichiers, 2);
    assert_eq!(a.texte.as_deref(), Some("début"));
    assert_eq!(a.commentaire_eleve.as_deref(), Some("à relire"));
    assert_eq!(lire(&fx, &fx.x, t).await.brouillon, None);
    // Un brouillon n'existe plus une fois déposé.
    let err = fx.ent.cdt_brouillon_enregistrer(&fx.eleve(&fx.x), t, depot("x", vec![])).await.unwrap_err();
    assert!(est_refus(&err, RefusMetier::Conflit));
}

// ───────────────────────── CC-34, CC-40, CC-41 : retour, réouverture, prolongation ─────────────────────────

#[tokio::test]
async fn redepot_apres_retour_refuse_sauf_rouverte_retard_recalcule_apres_prolongation() {
    let Some(fx) = fixture().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let (t, ech) = travail_en_ligne(&fx, true).await;
    // CC-40 : déposé à l'heure, puis le professeur avance l'échéance : la remise reste « à l'heure ».
    eleve_depose(&fx, &fx.x, t, ech - Duration::hours(1)).await.unwrap();
    fixer_echeance(&fx, t, ech - Duration::hours(10)).await;
    assert_eq!(lire(&fx, &fx.x, t).await.statut.as_deref(), Some("rendu"));
    fixer_echeance(&fx, t, ech).await;
    // Retour rendu ; CC-34 : le redépôt est refusé.
    let remise: Uuid = sqlx::query_scalar("SELECT id FROM ent_cdt_remise WHERE travail_id = $1 AND eleve_id = $2").bind(t).bind(fx.x.id).fetch_one(fx.ent.pool()).await.unwrap();
    let rt = fx.ent.cdt_retour_enregistrer(&fx.prof(), remise, RetourC5Body { commentaire: "Bien.".into(), rendre: true, ..Default::default() }, vec![]).await.unwrap();
    assert_eq!(rt.etat, "rendu");
    let err = eleve_depose(&fx, &fx.x, t, ech - Duration::minutes(5)).await.unwrap_err();
    assert!(est_refus(&err, RefusMetier::Conflit), "{err:?}");
    // Rouvrir : « à refaire » avec une nouvelle échéance, la première remise reste.
    let j = ouvre(&fx, jour(&fx).await + Duration::days(10)).await;
    fx.ent.cdt_remise_rouvrir(&fx.prof(), remise, RouvrirBody { echeance_jour: j, echeance_heure: NaiveTime::from_hms_opt(8, 0, 0).unwrap() }).await.unwrap();
    let r = lire(&fx, &fx.x, t).await;
    assert_eq!(r.etat, "a_refaire");
    assert!(r.delai_accorde);
    assert_eq!(r.nb_depots, 1);
    let a2 = eleve_depose(&fx, &fx.x, t, ech + Duration::days(1)).await.unwrap();
    assert!(!a2.en_retard, "la nouvelle échéance individuelle a été respectée");
    assert_eq!(a2.numero, 2);
    // Le retour précédent repasse en brouillon, sa version reste lisible.
    let rt2 = fx.ent.cdt_retour_lire_prof(&fx.prof(), remise).await.unwrap().unwrap();
    assert_eq!(rt2.etat, "brouillon");
    let versions: i64 = sqlx::query_scalar("SELECT count(*) FROM ent_cdt_retour_version WHERE retour_id = $1").bind(rt2.retour_id).fetch_one(fx.ent.pool()).await.unwrap();
    assert_eq!(versions, 1);
    assert!(fx.nb_evenements("cahier.remise_rouverte").await >= 1);
    // CC-41 : une copie en retard puis une prolongation accordée → statut recalculé « à l'heure », historique gardé.
    let (t2, ech2) = travail_en_ligne(&fx, true).await;
    let a = eleve_depose(&fx, &fx.w, t2, ech2 + Duration::hours(3)).await.unwrap();
    assert!(a.en_retard);
    let j2 = ouvre(&fx, jour(&fx).await + Duration::days(12)).await;
    fx.ent
        .cdt_prolonger(&fx.prof(), t2, ProlongationBody { eleve_ids: vec![fx.w.id], echeance_jour: j2, echeance_heure: NaiveTime::from_hms_opt(8, 0, 0).unwrap(), motif: "amenagement".into() })
        .await
        .unwrap();
    assert_eq!(lire(&fx, &fx.w, t2).await.statut.as_deref(), Some("rendu"));
    let lignes: i64 = sqlx::query_scalar("SELECT count(*) FROM ent_cdt_echeance_eleve WHERE travail_id = $1").bind(t2).fetch_one(fx.ent.pool()).await.unwrap();
    assert_eq!(lignes, 1);
    // Le motif codé n'est jamais servi à l'élève : « délai accordé » seulement.
    let json = serde_json::to_string(&lire(&fx, &fx.w, t2).await).unwrap();
    assert!(!json.contains("amenagement") && !json.contains("motif"), "{json}");
}

// ───────────────────────── groupes, CC-35, CC-36 ─────────────────────────

#[tokio::test]
async fn remise_de_groupe_compose_depot_commun_membre_parti_garde_sa_lecture() {
    let Some(fx) = fixture().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let cours = cours_a(&fx).await;
    let j = ouvre(&fx, jour(&fx).await + Duration::days(3)).await;
    let mut b = corps(vec![cours], j, "en_ligne");
    b.eleve_ids = vec![];
    let t = fx.ent.cdt_travaux_creer(&fx.prof(), b).await.unwrap().travaux[0].id;
    rendre_visible(&fx).await;
    // Sans groupe, un travail « par groupe » refuse le dépôt individuel.
    sqlx::query("UPDATE ent_cdt_travail SET remise_par_groupe = true, taille_groupe = 2 WHERE id = $1").bind(t).execute(fx.ent.pool()).await.unwrap();
    let ech: DateTime<Utc> = sqlx::query_scalar("SELECT echeance_le FROM ent_cdt_travail WHERE id = $1").bind(t).fetch_one(fx.ent.pool()).await.unwrap();
    let err = eleve_depose(&fx, &fx.x, t, ech - Duration::hours(1)).await.unwrap_err();
    assert!(est_refus(&err, RefusMetier::Conflit), "{err:?}");
    // Composition : un élève hors du cours (Y, classe B) ne peut pas être ajouté (R-C25), taille bornée à 2.
    let err = fx.ent.cdt_groupes_composer(&fx.prof(), t, GroupesBody { groupes: vec![GroupeBody { libelle: "G1".into(), eleve_ids: vec![fx.x.id, fx.y.id] }] }).await.unwrap_err();
    assert!(est_refus(&err, RefusMetier::Invalide), "{err:?}");
    let err = fx.ent.cdt_groupes_composer(&fx.prof(), t, GroupesBody { groupes: vec![GroupeBody { libelle: "G1".into(), eleve_ids: vec![fx.x.id, fx.w.id, fx.z.id] }] }).await.unwrap_err();
    assert!(est_refus(&err, RefusMetier::Invalide), "{err:?}");
    let g = fx.ent.cdt_groupes_composer(&fx.prof(), t, GroupesBody { groupes: vec![GroupeBody { libelle: "G1".into(), eleve_ids: vec![fx.x.id, fx.w.id] }] }).await.unwrap();
    assert_eq!(g.len(), 1);
    assert!(g[0].complet);
    // CC-36 : deux membres déposent : deux dépôts ordonnés, le dernier est courant ; tous les membres voient la remise.
    let a1 = eleve_depose(&fx, &fx.x, t, ech - Duration::hours(3)).await.unwrap();
    let a2 = eleve_depose(&fx, &fx.w, t, ech - Duration::hours(2)).await.unwrap();
    assert_eq!((a1.numero, a2.numero), (1, 2));
    assert_eq!(lire(&fx, &fx.x, t).await.nb_depots, 2);
    let rw = lire(&fx, &fx.w, t).await;
    assert_eq!(rw.depots.iter().find(|d| d.courant).unwrap().numero, 2);
    assert_eq!(rw.groupe.as_ref().unwrap().membres.len(), 2);
    // Un seul élève ne se retrouve pas dans deux groupes ; une fois la remise ouverte, plus de recomposition.
    let err = fx.ent.cdt_groupes_composer(&fx.prof(), t, GroupesBody { groupes: vec![GroupeBody { libelle: "G2".into(), eleve_ids: vec![fx.x.id] }] }).await.unwrap_err();
    assert!(est_refus(&err, RefusMetier::Conflit), "{err:?}");
    // CC-35 : un membre quitte l'établissement : la remise reste au groupe, il garde sa lecture mais ne dépose plus.
    sqlx::query("UPDATE ent_eleve SET date_sortie = ent_aujourdhui_etab(etablissement_id) - 1 WHERE id = $1").bind(fx.w.id).execute(fx.ent.pool()).await.unwrap();
    let rw = lire(&fx, &fx.w, t).await;
    assert_eq!(rw.nb_depots, 2);
    assert!(!rw.peut_deposer);
    let err = eleve_depose(&fx, &fx.w, t, ech - Duration::hours(1)).await.unwrap_err();
    assert!(est_interdit(&err), "{err:?}");
    assert_eq!(lire(&fx, &fx.x, t).await.nb_depots, 2);
    // Un élève d'un autre groupe ne lit pas cette remise : sa propre situation seulement.
    let rz = lire(&fx, &fx.z, t).await;
    assert_eq!(rz.nb_depots, 0);
}

#[tokio::test]
async fn inscription_libre_dans_les_groupes_taille_fixee_et_date_limite() {
    let Some(fx) = fixture().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let (t, _ech) = travail_en_ligne(&fx, true).await;
    sqlx::query("UPDATE ent_cdt_travail SET remise_par_groupe = true, taille_groupe = 2, inscription_groupes_jusqu_au = now() + interval '1 day' WHERE id = $1").bind(t).execute(fx.ent.pool()).await.unwrap();
    let g = fx.ent.cdt_groupe_libre(&fx.eleve(&fx.x), t, GroupeLibreBody { groupe_id: None, libelle: Some("Les as".into()) }).await.unwrap();
    assert_eq!(g.membres.len(), 1);
    let g2 = fx.ent.cdt_groupe_libre(&fx.eleve(&fx.w), t, GroupeLibreBody { groupe_id: Some(g.id), libelle: None }).await.unwrap();
    assert!(g2.complet);
    // Le groupe est complet : un troisième n'entre pas ; on n'est pas dans deux groupes.
    let err = fx.ent.cdt_groupe_libre(&fx.eleve(&fx.z), t, GroupeLibreBody { groupe_id: Some(g.id), libelle: None }).await.unwrap_err();
    assert!(est_refus(&err, RefusMetier::Conflit), "{err:?}");
    let err = fx.ent.cdt_groupe_libre(&fx.eleve(&fx.x), t, GroupeLibreBody { groupe_id: None, libelle: Some("Autre".into()) }).await.unwrap_err();
    assert!(est_refus(&err, RefusMetier::Conflit));
    // Inscription close.
    sqlx::query("UPDATE ent_cdt_travail SET inscription_groupes_jusqu_au = now() - interval '1 hour' WHERE id = $1").bind(t).execute(fx.ent.pool()).await.unwrap();
    let err = fx.ent.cdt_groupe_libre(&fx.eleve(&fx.z), t, GroupeLibreBody { groupe_id: None, libelle: Some("Tard".into()) }).await.unwrap_err();
    assert!(est_refus(&err, RefusMetier::Conflit));
}

// ───────────────────────── R-C13 : qui lit une copie ─────────────────────────

#[tokio::test]
async fn lecture_des_copies_matrice_de_separation() {
    let Some(fx) = fixture().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let (t, ech) = travail_en_ligne(&fx, true).await;
    let a = eleve_depose(&fx, &fx.x, t, ech - Duration::hours(1)).await.unwrap();
    let piece = a.pieces[0].id;
    // L'auteur et le professeur en saisie lisent ; liens courts émis après contrôle des droits.
    assert!(fx.ent.cdt_piece_lien(&fx.eleve(&fx.x), piece, None).await.is_ok());
    assert!(fx.ent.cdt_piece_lien(&fx.prof(), piece, None).await.is_ok());
    // Un camarade (même classe), un autre élève, la CPE, un autre professeur, la comptabilité : jamais.
    for ac in [fx.eleve(&fx.w), fx.eleve(&fx.y), fx.cpe(), fx.prof2(), fx.compta()] {
        let err = fx.ent.cdt_piece_lien(&ac, piece, None).await.unwrap_err();
        assert!(est_interdit(&err), "{} : {err:?}", ac.role);
    }
    // Le responsable ne lit jamais les dépôts avant le retour (R-C37).
    let err = fx.ent.cdt_piece_lien(&fx.parent(&fx.x), piece, None).await.unwrap_err();
    assert!(est_interdit(&err), "{err:?}");
    let vu = fx.ent.cdt_remise_lire(&fx.parent(&fx.x), t, Some(fx.x.id)).await.unwrap();
    assert!(vu.depots.is_empty() && vu.brouillon.is_none(), "le responsable ne voit pas les dépôts");
    // Direction : sans motif, invalide ; avec motif, lue et journalisée.
    let err = fx.ent.cdt_piece_lien(&fx.direction(), piece, None).await.unwrap_err();
    assert!(est_refus(&err, RefusMetier::Invalide), "{err:?}");
    assert!(fx.ent.cdt_piece_lien(&fx.direction(), piece, Some("réclamation de la famille")).await.is_ok());
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM ent_cdt_consultation WHERE etablissement_id = $1 AND nature = 'copie_motif'").bind(fx.cast.etablissement_id).fetch_one(fx.ent.pool()).await.unwrap();
    assert_eq!(n, 1);
    // Copie annotée : visible du responsable une fois rendue (jamais le brouillon), de l'élève, pas d'un camarade.
    let remise: Uuid = sqlx::query_scalar("SELECT id FROM ent_cdt_remise WHERE travail_id = $1 AND eleve_id = $2").bind(t).bind(fx.x.id).fetch_one(fx.ent.pool()).await.unwrap();
    let brouillon = fx.ent.cdt_retour_enregistrer(&fx.prof(), remise, RetourC5Body { commentaire: "à finir".into(), rendre: false, ..Default::default() }, vec![pdf("annotee.pdf")]).await.unwrap();
    let annotee = brouillon.pieces[0].id;
    assert_eq!(brouillon.pieces[0].nom, "retour-1.pdf");
    for ac in [fx.eleve(&fx.x), fx.parent(&fx.x)] {
        assert!(est_interdit(&fx.ent.cdt_piece_lien(&ac, annotee, None).await.unwrap_err()), "brouillon non lisible");
    }
    fx.ent.cdt_retour_enregistrer(&fx.prof(), remise, RetourC5Body { commentaire: "Bravo.".into(), rendre: true, ..Default::default() }, vec![]).await.unwrap();
    assert!(fx.ent.cdt_piece_lien(&fx.eleve(&fx.x), annotee, None).await.is_ok());
    assert!(fx.ent.cdt_piece_lien(&fx.parent(&fx.x), annotee, None).await.is_ok());
    assert!(est_interdit(&fx.ent.cdt_piece_lien(&fx.eleve(&fx.w), annotee, None).await.unwrap_err()));
    // Un responsable sans lien avec cet enfant : jamais.
    assert!(est_interdit(&fx.ent.cdt_piece_lien(&fx.parent(&fx.y), annotee, None).await.unwrap_err()));
    // Établissement B : la pièce est introuvable.
    let autre = fixture().await.unwrap();
    let err = autre.ent.cdt_piece_lien(&autre.prof(), piece, None).await.unwrap_err();
    assert!(est_introuvable(&err), "{err:?}");
    // Signalement d'une copie : la CPE saisie la lit, journalisée.
    fx.ent.cdt_copie_signaler(&fx.prof(), remise, SignalementCopieBody { vers: "cpe".into() }).await.unwrap();
    assert!(fx.ent.cdt_piece_lien(&fx.cpe(), piece, None).await.is_ok());
    assert!(fx.nb_evenements("cahier.copie_signalee").await >= 1);
}

#[tokio::test]
async fn responsable_et_eleve_parti_ne_deposent_pas() {
    let Some(fx) = fixture().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let (t, ech) = travail_en_ligne(&fx, true).await;
    let err = fx.ent.cdt_deposer(&fx.parent(&fx.x), t, depot("ma copie", vec![]), ech - Duration::hours(1)).await.unwrap_err();
    assert!(est_interdit(&err), "{err:?}");
    // C-D5 : un élève parti ne dépose plus.
    sqlx::query("UPDATE ent_eleve SET date_sortie = ent_aujourdhui_etab(etablissement_id) - 1 WHERE id = $1").bind(fx.w.id).execute(fx.ent.pool()).await.unwrap();
    assert!(est_interdit(&eleve_depose(&fx, &fx.w, t, ech - Duration::hours(1)).await.unwrap_err()));
    // Un élève hors du public ne trouve pas le travail ; un travail non publié n'existe pas pour lui.
    let err = fx.ent.cdt_remise_lire(&fx.eleve(&fx.y), t, None).await.unwrap_err();
    assert!(est_introuvable(&err), "{err:?}");
    sqlx::query("UPDATE ent_cdt_travail SET visible_a = now() + interval '1 day' WHERE id = $1").bind(t).execute(fx.ent.pool()).await.unwrap();
    assert!(est_introuvable(&fx.ent.cdt_remise_lire(&fx.eleve(&fx.x), t, None).await.unwrap_err()));
}

// ───────────────────────── suivi, constats, dépôt au nom de, délais ─────────────────────────

#[tokio::test]
async fn suivi_constats_depot_au_nom_de_et_telechargement() {
    let Some(fx) = fixture().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let (t, ech) = travail_en_ligne(&fx, true).await;
    eleve_depose(&fx, &fx.x, t, ech - Duration::hours(1)).await.unwrap();
    // CC-42 : dépôt par le professeur au nom de l'élève, sans pénalité de canal, marqué « déposé par ».
    let an = fx.ent.cdt_deposer_au_nom(&fx.prof(), t, fx.w.id, DepotEntree { texte: None, commentaire: None, fichiers: vec![pdf("scan.pdf")] }, ech + Duration::days(2)).await.unwrap();
    assert!(an.au_nom_de);
    assert!(!an.en_retard);
    assert!(an.depose_par.is_some());
    // Un élève, un responsable, ou un professeur sans saisie ne déposent pas au nom d'un autre.
    for ac in [fx.eleve(&fx.x), fx.parent(&fx.x), fx.prof2()] {
        let err = fx.ent.cdt_deposer_au_nom(&ac, t, fx.z.id, depot("x", vec![]), ech).await.unwrap_err();
        assert!(est_interdit(&err), "{} : {err:?}", ac.role);
    }
    // Hors du public (Y, classe B) : invalide.
    let err = fx.ent.cdt_deposer_au_nom(&fx.prof(), t, fx.y.id, depot("x", vec![]), ech).await.unwrap_err();
    assert!(est_refus(&err, RefusMetier::Invalide), "{err:?}");
    // Suivi : comptes justes, jamais un élève hors du public.
    let s = fx.ent.cdt_suivi(&fx.prof(), t).await.unwrap();
    assert!(s.lignes.iter().all(|l| l.eleve_id != fx.y.id));
    assert_eq!(s.comptes.rendus, 2);
    assert_eq!(s.comptes.retours_a_faire, 2);
    // Direction, CPE, autre professeur : aucun suivi (copies).
    for ac in [fx.direction(), fx.cpe(), fx.prof2()] {
        assert!(est_interdit(&fx.ent.cdt_suivi(&ac, t).await.unwrap_err()), "{}", ac.role);
    }
    // Constats : « non rendu » seulement à partir de la clôture, et jamais pour un élève qui a déposé.
    let err = fx.ent.cdt_constater(&fx.prof(), t, ConstatsBody { constats: vec![ConstatLigneBody { eleve_id: fx.z.id, statut: "non_rendu".into() }] }).await.unwrap_err();
    assert!(est_refus(&err, RefusMetier::Conflit), "{err:?}");
    fixer_cloture(&fx, t, ech + Duration::minutes(1)).await;
    sqlx::query("UPDATE ent_cdt_travail SET cloture_ramassage_le = now() - interval '1 hour', echeance_le = now() - interval '2 hours' WHERE id = $1").bind(t).execute(fx.ent.pool()).await.unwrap();
    let err = fx.ent.cdt_constater(&fx.prof(), t, ConstatsBody { constats: vec![ConstatLigneBody { eleve_id: fx.x.id, statut: "non_rendu".into() }] }).await.unwrap_err();
    assert!(est_refus(&err, RefusMetier::Conflit), "{err:?}");
    let s = fx.ent.cdt_suivi(&fx.prof(), t).await.unwrap();
    assert_eq!(s.lignes.iter().find(|l| l.eleve_id == fx.z.id).unwrap().constat_propose.as_deref(), Some("non_rendu"));
    let n = fx.ent.cdt_constater(&fx.prof(), t, ConstatsBody { constats: vec![ConstatLigneBody { eleve_id: fx.z.id, statut: "non_rendu".into() }] }).await.unwrap();
    assert_eq!(n, 1);
    let s = fx.ent.cdt_suivi(&fx.prof(), t).await.unwrap();
    assert_eq!(s.comptes.non_rendus, 1);
    // R-C12 : un fait, pas une note ; l'élève lit « non rendu », sans note.
    assert_eq!(lire(&fx, &fx.z, t).await.statut.as_deref(), Some("non_rendu"));
    // Constat inconnu / élève hors du public.
    assert!(est_refus(&fx.ent.cdt_constater(&fx.prof(), t, ConstatsBody { constats: vec![ConstatLigneBody { eleve_id: fx.z.id, statut: "zero".into() }] }).await.unwrap_err(), RefusMetier::Invalide));
    assert!(est_refus(&fx.ent.cdt_constater(&fx.prof(), t, ConstatsBody { constats: vec![ConstatLigneBody { eleve_id: fx.y.id, statut: "excuse".into() }] }).await.unwrap_err(), RefusMetier::Invalide));
    // Copies à télécharger : « NOM Prénom – titre », seulement les dépôts courants.
    let copies = fx.ent.cdt_copies_a_telecharger(&fx.prof(), t).await.unwrap();
    assert_eq!(copies.len(), 2);
    assert!(copies.iter().all(|c| c.nom_archive.contains(" – ") && c.nom_archive.ends_with(".pdf")), "{copies:?}");
    assert!(fx.ent.cdt_copies_a_telecharger(&fx.eleve(&fx.x), t).await.is_err());
}

#[tokio::test]
async fn demande_de_delai_acceptee_refusee_et_delai_accorde_a_plusieurs() {
    let Some(fx) = fixture().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let (t, ech) = travail_en_ligne(&fx, true).await;
    let d = fx.ent.cdt_delai_demander(&fx.eleve(&fx.x), t, None, DemandeDelaiBody { message: Some("malade".into()) }).await.unwrap();
    assert_eq!(d.etat, "ouverte");
    // Une demande ouverte à la fois ; le responsable habilité peut aussi demander pour son enfant.
    assert!(est_refus(&fx.ent.cdt_delai_demander(&fx.eleve(&fx.x), t, None, DemandeDelaiBody { message: None }).await.unwrap_err(), RefusMetier::Conflit));
    let dp = fx.ent.cdt_delai_demander(&fx.parent(&fx.w), t, Some(fx.w.id), DemandeDelaiBody { message: None }).await.unwrap();
    assert_eq!(dp.etat, "ouverte");
    // Le professeur voit les demandes, accepte l'une (nouvelle échéance), refuse l'autre.
    let attente = fx.ent.cdt_delais_attente(&fx.prof(), t).await.unwrap();
    assert_eq!(attente.len(), 2);
    let j = ouvre(&fx, jour(&fx).await + Duration::days(9)).await;
    fx.ent.cdt_delai_decider(&fx.prof(), d.id, DelaiDecisionBody { accepter: true, reponse: Some("ok".into()), echeance_jour: Some(j), echeance_heure: NaiveTime::from_hms_opt(8, 0, 0) }).await.unwrap();
    fx.ent.cdt_delai_decider(&fx.prof(), dp.id, DelaiDecisionBody { accepter: false, reponse: Some("trop tard".into()), echeance_jour: None, echeance_heure: None }).await.unwrap();
    let rx = lire(&fx, &fx.x, t).await;
    assert!(rx.delai_accorde && rx.echeance_le > ech);
    assert_eq!(rx.demande_delai.as_ref().unwrap().etat, "acceptee");
    let rw = lire(&fx, &fx.w, t).await;
    assert!(!rw.delai_accorde);
    assert_eq!(rw.demande_delai.as_ref().unwrap().reponse.as_deref(), Some("trop tard"));
    // Une demande ne se décide qu'une fois (la décidée est introuvable) ; un autre professeur ne décide rien.
    assert!(est_introuvable(&fx.ent.cdt_delai_decider(&fx.prof(), d.id, DelaiDecisionBody { accepter: false, reponse: None, echeance_jour: None, echeance_heure: None }).await.unwrap_err()));
    // Délai accordé à plusieurs d'un coup (« tous les absents ») : motif codé, interne.
    let err = fx.ent.cdt_prolonger(&fx.prof(), t, ProlongationBody { eleve_ids: vec![fx.z.id], echeance_jour: j, echeance_heure: NaiveTime::from_hms_opt(8, 0, 0).unwrap(), motif: "maladie-grave".into() }).await.unwrap_err();
    assert!(est_refus(&err, RefusMetier::Invalide));
    let err = fx.ent.cdt_prolonger(&fx.prof(), t, ProlongationBody { eleve_ids: vec![fx.z.id], echeance_jour: jour(&fx).await - Duration::days(2), echeance_heure: NaiveTime::from_hms_opt(8, 0, 0).unwrap(), motif: "absence".into() }).await.unwrap_err();
    assert!(est_refus(&err, RefusMetier::Invalide), "un délai ne raccourcit jamais");
    assert_eq!(fx.ent.cdt_prolonger(&fx.prof(), t, ProlongationBody { eleve_ids: vec![fx.z.id, fx.w.id], echeance_jour: j, echeance_heure: NaiveTime::from_hms_opt(9, 0, 0).unwrap(), motif: "absence".into() }).await.unwrap(), 2);
    let dz = fx.ent.cdt_delai_demander(&fx.eleve(&fx.z), t, None, DemandeDelaiBody { message: None }).await.unwrap();
    assert!(est_interdit(&fx.ent.cdt_delai_decider(&fx.prof2(), dz.id, DelaiDecisionBody { accepter: false, reponse: None, echeance_jour: None, echeance_heure: None }).await.unwrap_err()));
}

// ───────────────────────── retours ─────────────────────────

#[tokio::test]
async fn retours_note_masquee_versions_lot_et_prenom_d_un_autre_eleve() {
    let Some(fx) = fixture().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let (t, ech) = travail_en_ligne(&fx, true).await;
    eleve_depose(&fx, &fx.x, t, ech - Duration::hours(2)).await.unwrap();
    eleve_depose(&fx, &fx.w, t, ech - Duration::hours(2)).await.unwrap();
    let remise = |e: Uuid| {
        let ent = fx.ent.pool().clone();
        async move { sqlx::query_scalar::<_, Uuid>("SELECT id FROM ent_cdt_remise WHERE travail_id = $1 AND eleve_id = $2").bind(t).bind(e).fetch_one(&ent).await.unwrap() }
    };
    let (rx, rw) = (remise(fx.x.id).await, remise(fx.w.id).await);
    // Une note du professeur pour X, non publiée : la note n'apparaît pas (CC-37) ; avec « montrer », elle apparaît.
    let matiere: Uuid = sqlx::query_scalar("SELECT id FROM ent_matiere WHERE etablissement_id = $1 LIMIT 1").bind(fx.cast.etablissement_id).fetch_one(fx.ent.pool()).await.unwrap();
    let jour_note: NaiveDate = sqlx::query_scalar("SELECT ent_aujourdhui_etab($1)").bind(fx.cast.etablissement_id).fetch_one(fx.ent.pool()).await.unwrap();
    // La note n'est PAS publiée (CC-37) : la vue `ent_note` la porte avec `publiee = false`.
    let note: Uuid = crate::tests_evaluations::note_eva_brute(&fx.ent, fx.cast.etablissement_id, fx.x.id, matiere, 15.0, jour_note, fx.prof().user_id, false).await;
    let r = fx.ent.cdt_retour_enregistrer(&fx.prof(), rx, RetourC5Body { commentaire: format!("Bien, comme {}.", fx.w.prenom), note_id: Some(note), montrer_note: false, rendre: true, criteres: vec![CritereDto { libelle: "Rigueur".into(), niveau: Some("atteint".into()) }], ..Default::default() }, vec![]).await.unwrap();
    // CC-38 : le prénom d'un autre élève du cours est signalé à l'auteur (contrôle doux).
    assert_eq!(r.avertissements.len(), 1, "{:?}", r.avertissements);
    let lu = lire(&fx, &fx.x, t).await.retour.unwrap();
    assert_eq!(lu.note_valeur, None, "la note n'est jamais montrée sans le choix du professeur");
    assert_eq!(lu.criteres.len(), 1);
    assert!(!lu.modifie);
    // Rendre la note : nouvelle version, l'ancienne reste, « modifié » visible, avis « retour mis à jour ».
    let avant = fx.nb_evenements("cahier.retour_modifie").await;
    let r2 = fx.ent.cdt_retour_enregistrer(&fx.prof(), rx, RetourC5Body { commentaire: "Bien.".into(), note_id: Some(note), montrer_note: true, ..Default::default() }, vec![]).await.unwrap();
    assert_eq!(r2.version, 2);
    assert_eq!(fx.nb_evenements("cahier.retour_modifie").await, avant + 1);
    let lu = lire(&fx, &fx.x, t).await.retour.unwrap();
    assert_eq!((lu.note_valeur, lu.note_sur), (Some(15.0), Some(20.0)));
    assert!(lu.modifie);
    let v: i64 = sqlx::query_scalar("SELECT count(*) FROM ent_cdt_retour_version WHERE retour_id = $1").bind(r2.retour_id).fetch_one(fx.ent.pool()).await.unwrap();
    assert_eq!(v, 1);
    // Responsable : la note seulement avec le droit « voir les notes ».
    let pv = fx.ent.cdt_remise_lire(&fx.parent(&fx.x), t, Some(fx.x.id)).await.unwrap().retour.unwrap();
    assert_eq!(pv.note_valeur, Some(15.0));
    sqlx::query("UPDATE ent_eleve_lien SET peut_voir_notes = false WHERE eleve_id = $1").bind(fx.x.id).execute(fx.ent.pool()).await.unwrap();
    let pv = fx.ent.cdt_remise_lire(&fx.parent(&fx.x), t, Some(fx.x.id)).await.unwrap().retour.unwrap();
    assert_eq!(pv.note_valeur, None);
    // C-D8 : la note de X ne se lie pas à la copie de W, ni la note d'un collègue.
    let err = fx.ent.cdt_retour_enregistrer(&fx.prof(), rw, RetourC5Body { commentaire: "ok".into(), note_id: Some(note), ..Default::default() }, vec![]).await.unwrap_err();
    assert!(est_refus(&err, RefusMetier::Invalide), "{err:?}");
    // Rendre en lot : le brouillon non vide de W est rendu en un geste ; un brouillon vide reste brouillon.
    fx.ent.cdt_retour_enregistrer(&fx.prof(), rw, RetourC5Body { commentaire: "Correct.".into(), rendre: false, ..Default::default() }, vec![]).await.unwrap();
    assert_eq!(lire(&fx, &fx.w, t).await.retour, None, "un brouillon n'est pas visible");
    let avant = fx.nb_evenements("cahier.copie_rendue").await;
    assert_eq!(fx.ent.cdt_retours_rendre_en_lot(&fx.prof(), t).await.unwrap(), 1);
    assert_eq!(fx.nb_evenements("cahier.copie_rendue").await, avant + 1);
    assert_eq!(lire(&fx, &fx.w, t).await.statut.as_deref(), Some("retour_disponible"));
    // Un autre professeur ne corrige pas ; une copie sans dépôt n'a pas de retour.
    assert!(est_interdit(&fx.ent.cdt_retour_enregistrer(&fx.prof2(), rw, RetourC5Body::default(), vec![]).await.unwrap_err()));
}

// ───────────────────────── panne (R-C11), relance (CDT-196) ─────────────────────────

#[tokio::test]
async fn panne_prolonge_les_echeances_proches_une_fois_par_panne() {
    let Some(fx) = fixture().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let (t, ech) = travail_en_ligne(&fx, true).await;
    let debut = ech - Duration::minutes(90);
    let fin = ech - Duration::minutes(10);
    // Seule la direction (droit `cahier.parametrer`) déclare une panne.
    for ac in [fx.prof(), fx.eleve(&fx.x), fx.cpe()] {
        assert!(est_interdit(&fx.ent.cdt_panne_declarer(&ac, PanneBody { debut_le: debut, fin_le: fin }).await.unwrap_err()), "{}", ac.role);
    }
    let p = fx.ent.cdt_panne_declarer(&fx.direction(), PanneBody { debut_le: debut, fin_le: fin }).await.unwrap();
    // Durée minimale d'une heure ; ici 80 minutes.
    assert_eq!(p.prolongation_minutes, 80);
    assert!(p.travaux_prolonges >= 1);
    let r = lire(&fx, &fx.x, t).await;
    assert_eq!(r.echeance_le, ech + Duration::minutes(80));
    assert!(r.delai_accorde);
    let motif: String = sqlx::query_scalar("SELECT motif_code FROM ent_cdt_echeance_eleve WHERE travail_id = $1 AND eleve_id = $2").bind(t).bind(fx.x.id).fetch_one(fx.ent.pool()).await.unwrap();
    assert_eq!(motif, "panne");
    assert!(fx.nb_evenements("cahier.prolongation_panne").await >= 1);
    // Une panne trop longue ou inversée est refusée.
    assert!(est_refus(&fx.ent.cdt_panne_declarer(&fx.direction(), PanneBody { debut_le: fin, fin_le: debut }).await.unwrap_err(), RefusMetier::Invalide));
}

#[tokio::test]
async fn relance_des_non_rendus_apercu_puis_confirmation_une_par_jour() {
    let Some(fx) = fixture().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let (t, _ech) = travail_en_ligne(&fx, true).await;
    sqlx::query("UPDATE ent_cdt_travail SET echeance_le = now() - interval '3 hours' WHERE id = $1").bind(t).execute(fx.ent.pool()).await.unwrap();
    let ech: DateTime<Utc> = sqlx::query_scalar("SELECT echeance_le FROM ent_cdt_travail WHERE id = $1").bind(t).fetch_one(fx.ent.pool()).await.unwrap();
    eleve_depose(&fx, &fx.x, t, ech - Duration::hours(1)).await.unwrap();
    // Sans confirmation : un aperçu, rien n'est envoyé (action sensible).
    let avant = fx.nb_evenements("cahier.relance_non_rendus").await;
    let a = fx.ent.cdt_relancer(&fx.prof(), t, RelanceBody { eleve_ids: vec![], aux_familles: true, confirmer: false }).await.unwrap();
    assert!(!a.envoyee);
    assert!(a.destinataires >= 1);
    assert!(!a.texte.contains("note"));
    assert_eq!(fx.nb_evenements("cahier.relance_non_rendus").await, avant);
    // Confirmée : une relance par élève et par jour ; X (qui a déposé) n'est pas relancé.
    let r = fx.ent.cdt_relancer(&fx.prof(), t, RelanceBody { eleve_ids: vec![], aux_familles: true, confirmer: true }).await.unwrap();
    assert!(r.envoyee);
    assert_eq!(r.destinataires, a.destinataires);
    assert_eq!(fx.nb_evenements("cahier.relance_non_rendus").await, avant + 1);
    let x_relance: i64 = sqlx::query_scalar("SELECT count(*) FROM ent_cdt_relance WHERE travail_id = $1 AND eleve_id = $2").bind(t).bind(fx.x.id).fetch_one(fx.ent.pool()).await.unwrap();
    assert_eq!(x_relance, 0);
    let encore = fx.ent.cdt_relancer(&fx.prof(), t, RelanceBody { eleve_ids: vec![], aux_familles: true, confirmer: true }).await.unwrap();
    assert_eq!(encore.destinataires, 0);
    assert_eq!(encore.deja_relances, r.destinataires);
    assert!(est_interdit(&fx.ent.cdt_relancer(&fx.eleve(&fx.x), t, RelanceBody { eleve_ids: vec![], aux_familles: false, confirmer: true }).await.unwrap_err()));
}
