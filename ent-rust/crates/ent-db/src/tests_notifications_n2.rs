//! Socle 8, lot N2 (destinataires = personnes) : personne sans compte, exclusions tracées (interdiction,
//! « ne pas solliciter »), dédoublonnage par destination, tâche « à prévenir autrement », gel au décès,
//! liens nominatifs. Chaque test a son établissement ; instants dérivés de ceux des événements.
//! Sans `ENT_DATABASE_URL`, les scénarios sont ignorés.

use super::*;
use crate::notifications::ConfigExpedition;
use crate::ntf::catalogue::{ABSENCE_CONSTATEE, PASSAGE_INFIRMERIE};
use crate::ntf::expedition::expedier_avis_dus;
use crate::ntf::liens::{creer_lien, ouvrir_lien, revoquer_liens};
use crate::tenant_tx::TenantTx;
use crate::tests_notifications_n1::{avis_de, basculer, compter, distribuer, evenement, labo, notifications_legacy, publier};
use chrono::Duration;
use ent_auth::Role;
use ent_events::AggregateKind;
use serde_json::json;
use uuid::Uuid;

/// Responsable sans compte, lié à l'élève (droits par défaut), avec l'adresse donnée.
async fn personne_sans_compte(ent: &PgEnt, etab: Uuid, eleve: Uuid, email: Option<&str>, telephone: Option<&str>) -> Uuid {
    let pid = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO ent_personne (id, etablissement_id, nom, prenom, email, telephone, source)
         VALUES ($1, $2, $3, 'Sans', $4, $5, 'saisie')",
    )
    .bind(pid)
    .bind(etab)
    .bind(format!("Compte{}", Uuid::new_v4().simple()))
    .bind(email)
    .bind(telephone)
    .execute(ent.pool_verif())
    .await
    .expect("personne");
    sqlx::query(
        "INSERT INTO ent_eleve_lien (id, etablissement_id, eleve_id, personne_id, lien) VALUES ($1, $2, $3, $4, 'autre')",
    )
    .bind(Uuid::new_v4())
    .bind(etab)
    .bind(eleve)
    .bind(pid)
    .execute(ent.pool_verif())
    .await
    .expect("lien sans compte");
    pid
}

fn infirmerie(etab: Uuid, eleve: Uuid) -> ent_events::MetierEventDraft {
    evenement(etab, "infirmerie.famille_informee", AggregateKind::Eleve, eleve, json!({"jour": "2026-11-05"}))
}

/// Une personne sans compte est prévenue par e-mail (adresse lue dans sa fiche) ; l'avis se termine.
#[tokio::test]
async fn n2_parent_sans_compte_prevenu_par_courriel() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let (etab, eleve) = (cast.etablissement_id, cast.eleve_lea_id);
    basculer(&ent, etab, PASSAGE_INFIRMERIE.code).await;
    let adresse = format!("sans-compte-{}@example.invalid", Uuid::new_v4().simple());
    let pid = personne_sans_compte(&ent, etab, eleve, Some(&adresse), None).await;
    let evt = publier(&ent, infirmerie(etab, eleve)).await;
    distribuer(&ent, etab).await;
    let a = &avis_de(&ent, etab, evt.id).await[0];
    let (dest, compte): (Uuid, Option<Uuid>) = sqlx::query_as(
        "SELECT id, compte_id FROM ent_ntf_destinataire WHERE etablissement_id = $1 AND avis_id = $2 AND personne_id = $3 AND retenu",
    )
    .bind(etab)
    .bind(a.id)
    .bind(pid)
    .fetch_one(ent.pool_verif())
    .await
    .expect("destinataire sans compte retenu");
    assert!(compte.is_none());
    // Aucune remise ENT ni push : seulement l'e-mail.
    let canaux: Vec<String> = sqlx::query_scalar("SELECT canal FROM ent_ntf_remise WHERE etablissement_id = $1 AND destinataire_id = $2 ORDER BY canal")
        .bind(etab)
        .bind(dest)
        .fetch_all(ent.pool_verif())
        .await
        .unwrap();
    assert_eq!(canaux, vec!["email".to_string()]);
    let exp = ent.expediteur(ConfigExpedition { etablissement: Some(etab), ..ConfigExpedition::default() });
    let bilan = exp.expedier(a.effet_le).await.unwrap();
    assert!(bilan.simules + bilan.envoyes >= 1, "{bilan:?}");
    let (st_remise, st_dest): (String, String) = sqlx::query_as(
        "SELECT r.statut, d.statut FROM ent_ntf_remise r JOIN ent_ntf_destinataire d ON d.id = r.destinataire_id WHERE r.destinataire_id = $1",
    )
    .bind(dest)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!((st_remise.as_str(), st_dest.as_str()), ("remise", "remis"));
    let envoyes: i64 = sqlx::query_scalar("SELECT count(*) FROM ent_mail_outbox WHERE etablissement_id = $1 AND to_email = $2")
        .bind(etab)
        .bind(&adresse)
        .fetch_one(ent.pool_verif())
        .await
        .unwrap();
    assert_eq!(envoyes, 1, "un seul courriel, à l'adresse de la fiche");
    // Un second passage ne renvoie rien.
    exp.expedier(a.effet_le + Duration::minutes(5)).await.unwrap();
    let encore: i64 = sqlx::query_scalar("SELECT count(*) FROM ent_mail_outbox WHERE etablissement_id = $1 AND to_email = $2")
        .bind(etab)
        .bind(&adresse)
        .fetch_one(ent.pool_verif())
        .await
        .unwrap();
    assert_eq!(encore, 1);
}

/// CN-33 : deux personnes à la même adresse : une seule remise, la seconde est tracée « doublon ».
#[tokio::test]
async fn n2_meme_destination_une_seule_remise() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let (etab, eleve) = (cast.etablissement_id, cast.eleve_lea_id);
    basculer(&ent, etab, PASSAGE_INFIRMERIE.code).await;
    let adresse = format!("partagee-{}@example.invalid", Uuid::new_v4().simple());
    let p1 = personne_sans_compte(&ent, etab, eleve, Some(&adresse), None).await;
    let p2 = personne_sans_compte(&ent, etab, eleve, Some(&adresse.to_uppercase()), None).await;
    let evt = publier(&ent, infirmerie(etab, eleve)).await;
    distribuer(&ent, etab).await;
    let a = &avis_de(&ent, etab, evt.id).await[0];
    let lire = |p: Uuid| {
        let pool = ent.pool_verif().clone();
        async move {
            sqlx::query_as::<_, (String, Option<String>)>(
                "SELECT r.statut, r.motif FROM ent_ntf_remise r JOIN ent_ntf_destinataire d ON d.id = r.destinataire_id
                  WHERE d.avis_id = $1 AND d.personne_id = $2 AND r.canal = 'email'",
            )
            .bind(a.id)
            .bind(p)
            .fetch_one(&pool)
            .await
            .unwrap()
        }
    };
    let (r1, r2) = (lire(p1).await, lire(p2).await);
    let planifiees = [&r1, &r2].iter().filter(|r| r.0 == "planifiee").count();
    let doublons = [&r1, &r2].iter().filter(|r| r.0 == "ignoree" && r.1.as_deref() == Some("doublon_destination")).count();
    assert_eq!((planifiees, doublons), (1, 1), "{r1:?} {r2:?}");
    let aucune_tache: i64 = compter(&ent, "SELECT count(*) FROM ent_ntf_a_prevenir WHERE etablissement_id = $1 AND destinataire_id IN (SELECT id FROM ent_ntf_destinataire WHERE avis_id = $2)", etab, a.id).await;
    assert_eq!(aucune_tache, 0, "la destination est servie : aucune tâche manuelle");
}

/// Personne sans compte ni coordonnée : exclue (aucune_coordonnée) ; avec un numéro mais sans e-mail, ni compte
/// pour le SMS : tâche « à prévenir autrement » (le SMS sans compte attend les gabarits du lot N4).
#[tokio::test]
async fn n2_sans_coordonnee_exclue_et_tache_a_prevenir() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let (etab, eleve) = (cast.etablissement_id, cast.eleve_lea_id);
    basculer(&ent, etab, PASSAGE_INFIRMERIE.code).await;
    let rien = personne_sans_compte(&ent, etab, eleve, None, None).await;
    let tel = personne_sans_compte(&ent, etab, eleve, None, Some("0692123456")).await;
    let evt = publier(&ent, infirmerie(etab, eleve)).await;
    distribuer(&ent, etab).await;
    let a = &avis_de(&ent, etab, evt.id).await[0];
    let exclu: Option<String> = sqlx::query_scalar(
        "SELECT exclusion_code FROM ent_ntf_destinataire WHERE avis_id = $1 AND personne_id = $2 AND NOT retenu",
    )
    .bind(a.id)
    .bind(rien)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(exclu.as_deref(), Some("aucune_coordonnee"));
    let taches: i64 = compter(
        &ent,
        "SELECT count(*) FROM ent_ntf_a_prevenir t JOIN ent_ntf_destinataire d ON d.id = t.destinataire_id
          WHERE t.etablissement_id = $1 AND d.avis_id = $2 AND d.personne_id IN (SELECT id FROM ent_personne WHERE telephone IN ('0692123456', '+33692123456') AND etablissement_id = $1)",
        etab,
        a.id,
    )
    .await;
    assert_eq!(taches, 1, "numéro seul, sans compte : à prévenir autrement ({tel})");
}

/// CN-10 et CN-12 : un responsable sous interdiction ou « ne pas solliciter » est exclu, avec sa raison ;
/// l'autre responsable reçoit toujours l'avis. L'exclu ne reçoit rien à la remise non plus.
#[tokio::test]
async fn n2_interdiction_et_ne_pas_solliciter_tracees() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let (etab, eleve) = (cast.etablissement_id, cast.eleve_lea_id);
    basculer(&ent, etab, PASSAGE_INFIRMERIE.code).await;
    let interdit = personne_sans_compte(&ent, etab, eleve, Some(&format!("i-{}@example.invalid", Uuid::new_v4().simple())), None).await;
    let protege = personne_sans_compte(&ent, etab, eleve, Some(&format!("p-{}@example.invalid", Uuid::new_v4().simple())), None).await;
    sqlx::query(
        "INSERT INTO ent_eleve_interdiction (id, etablissement_id, eleve_id, personne_id, objet, fondement, consigne, du, decide_par)
         VALUES ($1, $2, $3, $4, ARRAY['communication'], 'decision_justice', 'consigne de test', DATE '2020-01-01', $5)",
    )
    .bind(Uuid::new_v4())
    .bind(etab)
    .bind(eleve)
    .bind(interdit)
    .bind(cast.direction.user_id)
    .execute(ent.pool_verif())
    .await
    .expect("interdiction");
    sqlx::query("INSERT INTO ent_personne_reserve (id, etablissement_id, personne_id, ne_pas_solliciter) VALUES ($1, $2, $3, true)")
        .bind(Uuid::new_v4())
        .bind(etab)
        .bind(protege)
        .execute(ent.pool_verif())
        .await
        .expect("réserve");
    let evt = publier(&ent, infirmerie(etab, eleve)).await;
    distribuer(&ent, etab).await;
    let a = &avis_de(&ent, etab, evt.id).await[0];
    let code = |p: Uuid| {
        let pool = ent.pool_verif().clone();
        async move {
            sqlx::query_scalar::<_, Option<String>>(
                "SELECT exclusion_code FROM ent_ntf_destinataire WHERE avis_id = $1 AND personne_id = $2",
            )
            .bind(a.id)
            .bind(p)
            .fetch_one(&pool)
            .await
            .unwrap()
        }
    };
    assert_eq!(code(interdit).await.as_deref(), Some("interdiction"));
    assert_eq!(code(protege).await.as_deref(), Some("ne_pas_solliciter"));
    let remises_exclus: i64 = compter(
        &ent,
        "SELECT count(*) FROM ent_ntf_remise r JOIN ent_ntf_destinataire d ON d.id = r.destinataire_id
          WHERE r.etablissement_id = $1 AND d.avis_id = $2 AND NOT d.retenu",
        etab,
        a.id,
    )
    .await;
    assert_eq!(remises_exclus, 0, "aucune remise pour un exclu");
    // Les responsables avec compte reçoivent toujours l'avis (droit = lien).
    expedier_avis_dus(ent.pool(), etab, a.effet_le).await.unwrap();
    assert_eq!(notifications_legacy(&ent, etab, cast.parent_lea.user_id, "infirmerie_passage").await, 1);
}

/// L'interdiction posée entre l'avis et la remise est relue à la remise (R-N1) : rien ne part.
#[tokio::test]
async fn n2_interdiction_posee_apres_l_avis() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let (etab, eleve, parent) = (cast.etablissement_id, cast.eleve_lea_id, cast.parent_lea.user_id);
    basculer(&ent, etab, ABSENCE_CONSTATEE.code).await;
    let evt = publier(&ent, crate::tests_notifications_n1::absence_appel(etab, eleve)).await;
    distribuer(&ent, etab).await;
    let a = &avis_de(&ent, etab, evt.id).await[0];
    let personne: Uuid = sqlx::query_scalar("SELECT personne_id FROM ent_eleve_lien WHERE etablissement_id = $1 AND eleve_id = $2 AND user_id = $3")
        .bind(etab)
        .bind(eleve)
        .bind(parent)
        .fetch_one(ent.pool_verif())
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO ent_eleve_interdiction (id, etablissement_id, eleve_id, personne_id, objet, fondement, consigne, du, decide_par)
         VALUES ($1, $2, $3, $4, ARRAY['communication'], 'decision_justice', 'consigne de test', DATE '2020-01-01', $5)",
    )
    .bind(Uuid::new_v4())
    .bind(etab)
    .bind(eleve)
    .bind(personne)
    .bind(cast.direction.user_id)
    .execute(ent.pool_verif())
    .await
    .unwrap();
    expedier_avis_dus(ent.pool(), etab, a.confirme_apres + Duration::minutes(1)).await.unwrap();
    assert_eq!(notifications_legacy(&ent, etab, parent, "absence_constatee").await, 0);
    let motif: Option<String> = sqlx::query_scalar(
        "SELECT r.motif FROM ent_ntf_remise r JOIN ent_ntf_destinataire d ON d.id = r.destinataire_id
          WHERE d.avis_id = $1 AND d.compte_id = $2 AND r.canal = 'inapp'",
    )
    .bind(a.id)
    .bind(parent)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(motif.as_deref(), Some("interdiction"));
}

/// CN-13 : le décès de l'élève annule l'avis en attente, pose un gel tracé, et aucun nouvel avis ne naît.
#[tokio::test]
async fn n2_deces_annule_les_avis_et_gele_le_sujet() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let (etab, eleve, parent) = (cast.etablissement_id, cast.eleve_lea_id, cast.parent_lea.user_id);
    basculer(&ent, etab, ABSENCE_CONSTATEE.code).await;
    let evt = publier(&ent, crate::tests_notifications_n1::absence_appel(etab, eleve)).await;
    distribuer(&ent, etab).await;
    let a = &avis_de(&ent, etab, evt.id).await[0];
    assert_eq!(a.statut, "planifie");
    ent.enregistrer_deces(Role::Direction.as_str(), etab, eleve, domain_eleve::horloge::jour_paris(), cast.direction.user_id)
        .await
        .expect("décès");
    distribuer(&ent, etab).await;
    let a = &avis_de(&ent, etab, evt.id).await[0];
    assert_eq!((a.statut.as_str(), a.annule_motif.as_deref()), ("annule", Some("deces")));
    let restantes: i64 = compter(
        &ent,
        "SELECT count(*) FROM ent_ntf_remise r JOIN ent_ntf_destinataire d ON d.id = r.destinataire_id
          WHERE r.etablissement_id = $1 AND d.avis_id = $2 AND r.statut <> 'annulee'",
        etab,
        a.id,
    )
    .await;
    assert_eq!(restantes, 0);
    let gels: i64 = compter(
        &ent,
        "SELECT count(*) FROM ent_ntf_gel WHERE etablissement_id = $1 AND sujet_id = $2 AND motif_code = 'deces' AND leve_le IS NULL",
        etab,
        eleve,
    )
    .await;
    assert_eq!(gels, 1, "gel tracé");
    expedier_avis_dus(ent.pool(), etab, a.confirme_apres + Duration::hours(1)).await.unwrap();
    assert_eq!(notifications_legacy(&ent, etab, parent, "absence_constatee").await, 0);
    // Un nouvel avis pour le même élève est refusé (sujet gelé).
    let evt2 = publier(&ent, crate::tests_notifications_n1::absence_appel(etab, eleve)).await;
    distribuer(&ent, etab).await;
    assert!(avis_de(&ent, etab, evt2.id).await.is_empty(), "aucun avis pour un élève décédé");
}

/// Lien nominatif : ouvertures limitées, expiration, révocation ; le jeton n'est jamais stocké en clair.
#[tokio::test]
async fn n2_liens_nominatifs() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let (etab, eleve) = (cast.etablissement_id, cast.eleve_lea_id);
    basculer(&ent, etab, PASSAGE_INFIRMERIE.code).await;
    let pid = personne_sans_compte(&ent, etab, eleve, Some(&format!("l-{}@example.invalid", Uuid::new_v4().simple())), None).await;
    let evt = publier(&ent, infirmerie(etab, eleve)).await;
    distribuer(&ent, etab).await;
    let a = &avis_de(&ent, etab, evt.id).await[0];
    let dest: Uuid = sqlx::query_scalar("SELECT id FROM ent_ntf_destinataire WHERE avis_id = $1 AND personne_id = $2")
        .bind(a.id)
        .bind(pid)
        .fetch_one(ent.pool_verif())
        .await
        .unwrap();
    let t0 = a.effet_le;
    let mut tx = TenantTx::begin(ent.pool(), etab).await.unwrap();
    let lien = creer_lien(&mut tx, etab, dest, t0, Duration::hours(2), 2, false).await.unwrap();
    // Deux ouvertures permises, la troisième refusée.
    for _ in 0..2 {
        let o = ouvrir_lien(&mut tx, etab, &lien.jeton, t0 + Duration::minutes(5)).await.unwrap().expect("ouverture");
        assert_eq!((o.destinataire_id, o.avis_id), (dest, a.id));
    }
    assert!(ouvrir_lien(&mut tx, etab, &lien.jeton, t0 + Duration::minutes(6)).await.unwrap().is_none(), "épuisé");
    // Expiré.
    let l2 = creer_lien(&mut tx, etab, dest, t0, Duration::hours(1), 5, false).await.unwrap();
    assert!(ouvrir_lien(&mut tx, etab, &l2.jeton, t0 + Duration::hours(2)).await.unwrap().is_none(), "expiré");
    // Révoqué ; jeton mal formé ou inconnu : même réponse.
    let l3 = creer_lien(&mut tx, etab, dest, t0, Duration::hours(1), 5, false).await.unwrap();
    assert_eq!(revoquer_liens(&mut tx, etab, dest, t0).await.unwrap(), 3);
    assert!(ouvrir_lien(&mut tx, etab, &l3.jeton, t0 + Duration::minutes(1)).await.unwrap().is_none(), "révoqué");
    assert!(ouvrir_lien(&mut tx, etab, "ntf_inconnu", t0).await.unwrap().is_none());
    tx.commit().await.unwrap();
    // Jamais en clair en base.
    let en_clair: i64 = sqlx::query_scalar("SELECT count(*) FROM ent_ntf_lien WHERE jeton_hash = $1")
        .bind(&lien.jeton)
        .fetch_one(ent.pool_verif())
        .await
        .unwrap();
    assert_eq!(en_clair, 0);
}
