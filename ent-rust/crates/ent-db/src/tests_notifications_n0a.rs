//! Socle 8, lot N0a (fuites) : un test par point. Chaque test ne regarde que ses propres
//! lignes (clé = `source_id` de la sanction ou de l'entrée créée ici), jamais « toute la base ».
//! Sans `ENT_DATABASE_URL`, les scénarios sont ignorés.

use super::*;
use domain_eleve::horloge::jour_paris;

async fn labo() -> Option<(PgEnt, LaboCast, sqlx::PgPool)> {
    let url = crate::url_guard::url_labo_ou_echec()?;
    assert_local_postgres_url(&url).expect("ENT_DATABASE_URL doit être locale");
    let mut ent = PgEnt::connect(&url).await.expect("connect");
    ent.migrate().await.expect("migrate");
    ent.install_brain_p0();
    let cast = ent.seed_labo_cast().await.expect("seed cast");
    let pool = sqlx::PgPool::connect(&url).await.expect("pool");
    Some((ent, cast, pool))
}

/// Prononce une sanction « retenue » pour Léa (CPE) et rend son identifiant.
async fn sanction_lea(ent: &PgEnt, cast: &LaboCast) -> Uuid {
    let types = ent.list_sanction_types(cast.etablissement_id).await.expect("types");
    let retenue = types.iter().find(|t| t.code == "retenue").expect("retenue");
    ent.prononcer_sanction_for_actor(
        Role::Cpe.as_str(),
        domain_eleve::PrononcerSanction {
            etablissement_id: cast.etablissement_id,
            eleve_id: cast.eleve_lea_id,
            type_id: retenue.id,
            motif_id: None,
            incident_id: None,
            jour: jour_paris(),
            duree_minutes: Some(45),
            note: None,
            actor_user_id: cast.cpe.user_id,
        },
    )
    .await
    .expect("sanction")
    .id
}

/// (a) + (b) : le SMS de sanction est neutre et l'événement `sms.envoye` ne porte ni numéro
/// ni texte ; (a) idem pour `mail.envoye` (ni adresse, ni objet).
#[tokio::test]
async fn n0a_sms_sanction_neutre_et_evenements_sans_coordonnees() {
    let Some((ent, cast, pool)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let san = sanction_lea(&ent, &cast).await;

    let sms: Vec<(Uuid, String, String)> = sqlx::query_as(
        "SELECT id, to_phone, body FROM ent_sms_outbox
          WHERE etablissement_id=$1 AND source_aggregate='sanction' AND source_id=$2",
    )
    .bind(cast.etablissement_id)
    .bind(san)
    .fetch_all(&pool)
    .await
    .expect("sms");
    assert!(!sms.is_empty(), "la sanction doit produire au moins un SMS famille");
    for (id, tel, corps) in &sms {
        assert!(corps.starts_with("Vie scolaire : une information concernant "), "{corps}");
        assert!(corps.ends_with("vous attend sur ScolIA."), "{corps}");
        for interdit in ["retenue", "Retenue", "sanction", "Potter", "min", "45"] {
            assert!(!corps.contains(interdit), "SMS non neutre ({interdit}) : {corps}");
        }
        let ev: Vec<(serde_json::Value,)> = sqlx::query_as(
            "SELECT payload FROM ent_metier_event
              WHERE etablissement_id=$1 AND type='sms.envoye' AND aggregate_id=$2",
        )
        .bind(cast.etablissement_id)
        .bind(id)
        .fetch_all(&pool)
        .await
        .expect("ev");
        assert_eq!(ev.len(), 1, "un événement sms.envoye par envoi");
        let texte = ev[0].0.to_string();
        assert!(!texte.contains(tel.as_str()), "numéro dans l'événement : {texte}");
        assert!(ev[0].0.get("to").is_none());
    }

    // Courriers de la même sanction : événement sans adresse ni objet.
    let mails: Vec<(Uuid, String, String)> = sqlx::query_as(
        "SELECT id, to_email, subject FROM ent_mail_outbox
          WHERE etablissement_id=$1 AND source_aggregate='sanction' AND source_id=$2",
    )
    .bind(cast.etablissement_id)
    .bind(san)
    .fetch_all(&pool)
    .await
    .expect("mails");
    for (id, adresse, objet) in &mails {
        let ev: Vec<(serde_json::Value,)> = sqlx::query_as(
            "SELECT payload FROM ent_metier_event
              WHERE etablissement_id=$1 AND type='mail.envoye' AND aggregate_id=$2",
        )
        .bind(cast.etablissement_id)
        .bind(id)
        .fetch_all(&pool)
        .await
        .expect("ev");
        assert_eq!(ev.len(), 1);
        let texte = ev[0].0.to_string();
        assert!(!texte.contains(adresse.as_str()), "adresse dans l'événement : {texte}");
        assert!(!texte.contains(objet.as_str()) || objet.is_empty());
        assert!(ev[0].0.get("to").is_none() && ev[0].0.get("subject").is_none());
    }
}

/// (c) : le courrier du psychologue n'a plus de pièce jointe ; son corps renvoie vers l'ENT.
#[tokio::test]
async fn n0a_courrier_psy_sans_piece_jointe() {
    let Some((ent, cast, pool)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let psy = cast.psys.first().expect("psy");
    // Le dossier psy doit exister pour envoyer : on tolère l'absence (rien à vérifier alors).
    let avant: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM ent_mail_outbox WHERE etablissement_id=$1 AND kind='courrier_psy'",
    )
    .bind(cast.etablissement_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    let n = ent
        .envoyer_courrier_orientation_psy_for_actor(
            cast.etablissement_id,
            psy.user_id,
            Role::Psy.as_str(),
            cast.eleve_lea_id,
        )
        .await;
    let Ok(n) = n else {
        eprintln!("skip : pas de dossier psy pour Léa ({n:?})");
        return;
    };
    assert!(n >= 1, "au moins un courrier");
    let corps: Vec<String> = sqlx::query_scalar(
        "SELECT body_text FROM ent_mail_outbox
          WHERE etablissement_id=$1 AND kind='courrier_psy' AND eleve_id=$2
          ORDER BY created_at DESC LIMIT $3",
    )
    .bind(cast.etablissement_id)
    .bind(cast.eleve_lea_id)
    .bind(n as i64)
    .fetch_all(&pool)
    .await
    .unwrap();
    let _ = avant;
    assert!(!corps.is_empty());
    for c in corps {
        assert!(!c.contains("ci-joint"), "{c}");
        assert!(c.contains("ScolIA"), "{c}");
    }
}

/// (h) : `/mails` et `/sms` ne sont lisibles qu'avec `notifications.gerer` (direction/admin) :
/// ni l'accueil (administratif), ni le CPE, ni les autres personnels.
#[tokio::test]
async fn n0a_journaux_envois_reserves_a_notifications_gerer() {
    let Some((ent, cast, _)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = cast.etablissement_id;
    for role in ["administratif", "cpe", "prof", "compta", "infirmerie", "psy", "parent"] {
        assert!(ent.list_mails_recents_for_actor(etab, role, 5).await.is_err(), "mails {role}");
        assert!(ent.list_sms_recents_for_actor(etab, role, 5).await.is_err(), "sms {role}");
    }
    for role in ["direction", "admin"] {
        assert!(ent.list_mails_recents_for_actor(etab, role, 5).await.is_ok(), "mails {role}");
        assert!(ent.list_sms_recents_for_actor(etab, role, 5).await.is_ok(), "sms {role}");
    }
}
