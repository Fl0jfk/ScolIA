//! Socle 8, lot N0b (justesse) : un test par point. Chaque test ne regarde que ses propres
//! lignes ; les écritures sur le jeu partagé du labo sont rendues ou faites dans une transaction
//! annulée. Instants fixes : aucun test ne dépend de l'heure réelle. Sans `ENT_DATABASE_URL`,
//! les scénarios sont ignorés.

use super::*;
use crate::foyer_repo::DroitLien;
use crate::notifications::{planifier_envois, EnvoiAPlanifier};
use crate::outbox::ConfigDistribution;
use chrono::{DateTime, Duration, NaiveDate, TimeZone, Utc};
use domain_eleve::horloge::jour_paris;
use domain_eleve::CreateEleve;
use ent_auth::Role;
use uuid::Uuid;

async fn labo() -> Option<(PgEnt, LaboCast)> {
    let url = crate::url_guard::url_labo_ou_echec()?;
    assert_local_postgres_url(&url).expect("ENT_DATABASE_URL doit être locale");
    let mut ent = PgEnt::connect(&url).await.expect("connect");
    ent.migrate().await.expect("migrate");
    ent.install_brain_p0();
    ent.install_mail_dry_run();
    let cast = ent.seed_labo_cast().await.expect("seed cast");
    Some((ent, cast))
}

async fn distribuer(ent: &PgEnt, etab: Uuid) {
    ent.distributeur(ConfigDistribution {
        taille_lot: 100,
        max_tentatives: 3,
        delai_base: std::time::Duration::ZERO,
        delai_max: std::time::Duration::ZERO,
        intervalle: std::time::Duration::from_millis(20),
        etablissement: Some(etab),
    })
    .distribuer_tout()
    .await
    .unwrap();
}

/// (e) Le décès d'un élève annule ses remises `en_attente` (motif `deces`) ; celles d'un autre
/// élève ne bougent pas.
#[tokio::test]
async fn n0b_deces_annule_les_remises_en_attente() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let (etab, parent) = (cast.etablissement_id, cast.parent_lea.user_id);
    let nom = format!("Deces{}", Uuid::new_v4().simple());
    let e = ent
        .create_eleve(CreateEleve {
            etablissement_id: etab,
            nom,
            prenom: "Zélie".into(),
            ine: None,
            sexe: Some("F".into()),
            date_naissance: NaiveDate::from_ymd_opt(2010, 3, 14),
            numero_badge: None,
            redoublant: false,
            classe: None,
            regime: None,
            mef: None,
            site: None,
            date_entree: Some(jour_paris() - Duration::days(50)),
            date_sortie: None,
            motif_sortie: None,
            actor_user_id: None,
        })
        .await
        .expect("élève");
    let mut ids = Vec::new();
    for (eleve, tag) in [(e.id, "deces"), (cast.eleve_lea_id, "temoin")] {
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_notification_envoi
                (id, etablissement_id, destinataire_user_id, canal, kind, titre, corps, eleve_id,
                 source_type, source_id, cle_dedup, regroupable, planifie_pour)
             VALUES ($1,$2,$3,'email','retard','t','c',$4,'systeme',$1,$5,true,
                     TIMESTAMPTZ '2100-01-01 00:00:00+00')",
        )
        .bind(id)
        .bind(etab)
        .bind(parent)
        .bind(eleve)
        .bind(format!("n0b-{tag}-{id}"))
        .execute(ent.pool_verif())
        .await
        .expect("envoi");
        ids.push(id);
    }
    ent.enregistrer_deces(Role::Direction.as_str(), etab, e.id, jour_paris(), cast.direction.user_id)
        .await
        .expect("décès");
    distribuer(&ent, etab).await;
    let lire = |id: Uuid| {
        let pool = ent.pool_verif().clone();
        async move {
            sqlx::query_as::<_, (String, Option<String>)>(
                "SELECT statut, motif FROM ent_notification_envoi WHERE id = $1",
            )
            .bind(id)
            .fetch_one(&pool)
            .await
            .unwrap()
        }
    };
    assert_eq!(lire(ids[0]).await, ("annule".to_string(), Some("deces".to_string())));
    assert_eq!(lire(ids[1]).await.0, "en_attente", "l'envoi d'un autre élève est intact");
    // Les envois du témoin sont retirés pour ne pas être expédiés par un autre test.
    sqlx::query("DELETE FROM ent_notification_envoi WHERE id = $1").bind(ids[1]).execute(ent.pool_verif()).await.unwrap();
}

/// (f) Ancien chemin : un responsable sans `peut_voir_notes` n'est pas destinataire du bulletin
/// (ni du livret, ni de la décision de conseil) ; sans `peut_voir_vs`, ni de la sanction ni du
/// carnet. Les autres responsables restent destinataires. Vérifié sur adresses et téléphones.
#[tokio::test]
async fn n0b_ancien_chemin_relit_le_droit_du_lien() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let (etab, eleve) = (cast.etablissement_id, cast.eleve_lea_id);
    let user = cast.parent_lea.user_id;
    let adresse = cast.parent_lea.email.trim().to_lowercase();
    let foyers = crate::foyer_repo::PgFoyerRepo::new(ent.pool().clone());
    let courriels = |d| foyers.emails_parents_eleve_avec_droit(etab, eleve, d);
    // Référence : tous les droits → destinataire.
    sqlx::query("UPDATE ent_eleve_lien SET peut_voir_notes = true, peut_voir_vs = true WHERE etablissement_id = $1 AND eleve_id = $2 AND user_id = $3")
        .bind(etab).bind(eleve).bind(user).execute(ent.pool_verif()).await.unwrap();
    assert!(courriels(None).await.unwrap().contains(&adresse), "parent de référence absent");
    assert!(courriels(Some(DroitLien::Notes)).await.unwrap().contains(&adresse));
    // Notes retirées.
    sqlx::query("UPDATE ent_eleve_lien SET peut_voir_notes = false WHERE etablissement_id = $1 AND eleve_id = $2 AND user_id = $3")
        .bind(etab).bind(eleve).bind(user).execute(ent.pool_verif()).await.unwrap();
    assert!(!courriels(Some(DroitLien::Notes)).await.unwrap().contains(&adresse), "bulletin sans droit");
    assert!(courriels(Some(DroitLien::VieScolaire)).await.unwrap().contains(&adresse));
    assert!(courriels(None).await.unwrap().contains(&adresse), "sans droit exigé : inchangé");
    // Vie scolaire retirée.
    sqlx::query("UPDATE ent_eleve_lien SET peut_voir_notes = true, peut_voir_vs = false WHERE etablissement_id = $1 AND eleve_id = $2 AND user_id = $3")
        .bind(etab).bind(eleve).bind(user).execute(ent.pool_verif()).await.unwrap();
    assert!(!courriels(Some(DroitLien::VieScolaire)).await.unwrap().contains(&adresse), "carnet sans droit");
    assert!(courriels(Some(DroitLien::Notes)).await.unwrap().contains(&adresse));
    // Le choix du droit selon la nature de l'envoi.
    for k in ["bulletin", "livret_competences", "conseil_decision"] {
        assert_eq!(DroitLien::pour_kind(k), Some(DroitLien::Notes), "{k}");
    }
    for k in ["sanction", "carnet", "seuil"] {
        assert_eq!(DroitLien::pour_kind(k), Some(DroitLien::VieScolaire), "{k}");
    }
    assert_eq!(DroitLien::pour_kind("infirmerie_passage"), None);
    // Remise de l'état de référence (jeu partagé).
    sqlx::query("UPDATE ent_eleve_lien SET peut_voir_notes = true, peut_voir_vs = true WHERE etablissement_id = $1 AND eleve_id = $2 AND user_id = $3")
        .bind(etab).bind(eleve).bind(user).execute(ent.pool_verif()).await.unwrap();
}

/// (d) Avis d'infirmerie : le parent sans `peut_voir_vs` reçoit l'envoi (e-mail) — la remise ne
/// exige plus que le lien actif (`droit_requis = lien`).
#[tokio::test]
async fn n0b_avis_infirmerie_envoye_sans_peut_voir_vs() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let (etab, eleve) = (cast.etablissement_id, cast.eleve_lea_id);
    let parent = cast.parent_lea.user_id;
    sqlx::query("UPDATE ent_eleve_lien SET peut_voir_vs = false WHERE etablissement_id = $1 AND eleve_id = $2 AND user_id = $3")
        .bind(etab).bind(eleve).bind(parent).execute(ent.pool_verif()).await.unwrap();
    let jour = format!("2030-0{}-15", 1 + (Uuid::new_v4().as_u128() % 9)); // jour fictif, lignes propres
    let evt = Uuid::new_v4();
    let mut tx = ent.pool().begin().await.unwrap();
    use crate::outbox::Publier;
    let e = tx
        .publier(ent_events::MetierEventDraft {
            etablissement_id: etab,
            type_name: "infirmerie.famille_informee".to_string(),
            aggregate: ent_events::AggregateKind::Eleve,
            aggregate_id: eleve,
            eleve_id: Some(eleve),
            payload: serde_json::json!({ "jour": jour, "n0b": evt.to_string() }),
            actor_user_id: None,
        })
        .await
        .unwrap();
    tx.commit().await.unwrap();
    distribuer(&ent, etab).await;
    let lignes: Vec<(String, Option<String>)> = sqlx::query_as(
        "SELECT canal, droit_requis FROM ent_notification_envoi
          WHERE etablissement_id = $1 AND destinataire_user_id = $2 AND source_id = $3
            AND kind = 'infirmerie_passage'",
    )
    .bind(etab)
    .bind(parent)
    .bind(e.id)
    .fetch_all(ent.pool_verif())
    .await
    .unwrap();
    assert!(lignes.iter().any(|l| l.0 == "email"), "e-mail planifié pour le parent sans peut_voir_vs : {lignes:?}");
    assert!(lignes.iter().all(|l| l.1.as_deref() == Some("lien")), "{lignes:?}");
    sqlx::query("UPDATE ent_eleve_lien SET peut_voir_vs = true WHERE etablissement_id = $1 AND eleve_id = $2 AND user_id = $3")
        .bind(etab).bind(eleve).bind(parent).execute(ent.pool_verif()).await.unwrap();
}

/// (g) Fuseau de l'établissement : pour un établissement à UTC+4 (Indian/Reunion), un message
/// regroupable créé à 11 h UTC (15 h locale) part au résumé de 7 h 30 **locale** du lendemain
/// (03 h 30 UTC), et non à 7 h 30 de Paris. Tout se passe dans une transaction annulée.
#[tokio::test]
async fn n0b_resume_a_l_heure_de_l_etablissement() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let (etab, user) = (cast.etablissement_id, cast.parent_lea.user_id);
    let maintenant: DateTime<Utc> = Utc.with_ymd_and_hms(2026, 11, 10, 11, 0, 0).unwrap();
    let mut tx = crate::tenant_tx::TenantTx::begin(ent.pool(), etab).await.unwrap();
    sqlx::query(
        "INSERT INTO ent_etablissement_parametre (etablissement_id, fuseau) VALUES ($1, 'Indian/Reunion')
         ON CONFLICT (etablissement_id) DO UPDATE SET fuseau = 'Indian/Reunion'",
    )
    .bind(etab)
    .execute(&mut *tx)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO ent_preference_notification (etablissement_id, user_id, canal_email, canal_sms, canal_push, resume)
         VALUES ($1, $2, true, false, false, 'quotidien')
         ON CONFLICT (etablissement_id, user_id) DO UPDATE SET canal_email = true, resume = 'quotidien',
             calme_debut = NULL, calme_fin = NULL",
    )
    .bind(etab)
    .bind(user)
    .execute(&mut *tx)
    .await
    .unwrap();
    let cle = format!("n0b-fuseau-{}", Uuid::new_v4());
    let envoi = EnvoiAPlanifier {
        etablissement_id: etab,
        user_id: user,
        kind: "retard",
        titre: "t",
        corps: "c",
        eleve_id: Some(cast.eleve_lea_id),
        notification_id: None,
        source_type: "systeme",
        source_id: Uuid::new_v4(),
        cle_dedup: cle.clone(),
        droit_requis: None,
        regroupable: true,
        canaux_voulus: &["email"],
    };
    let n = planifier_envois(&mut tx, &envoi, maintenant).await.unwrap();
    assert_eq!(n, 1, "un envoi e-mail planifié");
    let (pour,): (DateTime<Utc>,) = sqlx::query_as(
        "SELECT planifie_pour FROM ent_notification_envoi WHERE cle_dedup = $1",
    )
    .bind(&cle)
    .fetch_one(&mut *tx)
    .await
    .unwrap();
    assert_eq!(pour, Utc.with_ymd_and_hms(2026, 11, 11, 3, 30, 0).unwrap(), "7 h 30 locale = 03 h 30 UTC");
    // Témoin : sans paramètre d'établissement (Paris, UTC+1 en novembre) → 06 h 30 UTC.
    sqlx::query("DELETE FROM ent_etablissement_parametre WHERE etablissement_id = $1")
        .bind(etab)
        .execute(&mut *tx)
        .await
        .unwrap();
    let cle2 = format!("n0b-paris-{}", Uuid::new_v4());
    let envoi2 = EnvoiAPlanifier { cle_dedup: cle2.clone(), source_id: Uuid::new_v4(), ..envoi };
    planifier_envois(&mut tx, &envoi2, maintenant).await.unwrap();
    let (paris,): (DateTime<Utc>,) = sqlx::query_as(
        "SELECT planifie_pour FROM ent_notification_envoi WHERE cle_dedup = $1",
    )
    .bind(&cle2)
    .fetch_one(&mut *tx)
    .await
    .unwrap();
    assert_eq!(paris, Utc.with_ymd_and_hms(2026, 11, 11, 6, 30, 0).unwrap());
    // La transaction est abandonnée (rollback) : rien n'est écrit dans le jeu partagé.
    drop(tx);
}
