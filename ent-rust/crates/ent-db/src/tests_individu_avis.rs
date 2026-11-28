//! Transverse « individu », suite des écrans : la personne est prévenue quand on relie ou délie ses fiches, quand un
//! autre compte se déclare être elle, et quand sa déclaration est confirmée. Contre la base locale de test (feature
//! `labo`). Sans `ENT_DATABASE_URL` : ignorés en local et **en échec en CI**.

use super::*;
use crate::individu::{
    AVIS_DECLARATION_CONFIRMEE, AVIS_DECLARATION_RECUE, AVIS_LIEN_LEVE, AVIS_LIEN_POSE, AVIS_TEXTE_MAX,
};
use crate::tests_individu::{compte, connecter, fiche_du_compte, fiche_sans_compte};
use uuid::Uuid;

/// Avis reçus par un compte, par nature (`kind`), dans l'ENT.
async fn avis(ent: &PgEnt, etab: Uuid, user: Uuid, kind: &str) -> Vec<(String, String)> {
    let mut tx = TenantTx::begin(ent.pool(), etab).await.unwrap();
    sqlx::query_as(
        "SELECT titre, corps FROM ent_notification
          WHERE etablissement_id = $1 AND destinataire_user_id = $2 AND kind = $3 ORDER BY created_at",
    )
    .bind(etab)
    .bind(user)
    .bind(kind)
    .fetch_all(&mut *tx)
    .await
    .unwrap()
}

/// Envois externes planifiés pour un compte et une nature : (canal, sujet, corps).
async fn envois(ent: &PgEnt, etab: Uuid, user: Uuid, kind: &str) -> Vec<(String, String, String)> {
    let mut tx = TenantTx::begin(ent.pool(), etab).await.unwrap();
    sqlx::query_as(
        "SELECT e.canal, e.titre, e.corps FROM ent_notification_envoi e
          WHERE e.etablissement_id = $1 AND e.destinataire_user_id = $2 AND e.cle_dedup LIKE $3 || ':%'",
    )
    .bind(etab)
    .bind(user)
    .bind(kind)
    .fetch_all(&mut *tx)
    .await
    .unwrap()
}

/// Relier prévient tous les comptes de l'individu (pas l'auteur), lever aussi ; un texte ne nomme jamais personne.
#[tokio::test]
async fn individu_avis_pose_et_levee() {
    let Some(ent) = connecter().await else {
        return eprintln!("skip : ENT_DATABASE_URL");
    };
    let etab = Uuid::new_v4();
    let (dir, _) = compte(&ent, etab, "direction").await;
    let (sec, _) = compte(&ent, etab, "administratif").await;
    let (prof, _) = compte(&ent, etab, "prof").await;
    let (parent, _) = compte(&ent, etab, "parent").await;
    let p_prof = fiche_du_compte(&ent, etab, prof).await;
    let p_parent = fiche_du_compte(&ent, etab, parent).await;
    let p_sans = fiche_sans_compte(&ent, etab, "Durand", None).await;

    let l1 = ent
        .individu_relier_for_actor(etab, sec, "administratif", p_prof, p_parent, "deux_comptes", false)
        .await
        .unwrap();
    for u in [prof, parent] {
        let a = avis(&ent, etab, u, AVIS_LIEN_POSE.0).await;
        assert_eq!(a.len(), 1, "chaque compte de l'individu est prévenu : {u}");
        assert_eq!(a[0].0, AVIS_LIEN_POSE.1);
        // Le texte ne nomme ni les fiches ni les personnes (il peut sortir par e-mail).
        assert!(!a[0].1.contains("Parent") && !a[0].1.contains("Nom "), "{}", a[0].1);
        let e = envois(&ent, etab, u, AVIS_LIEN_POSE.0).await;
        assert!(e.iter().any(|(c, _, _)| c == "email"), "e-mail planifié par défaut : {e:?}");
        assert!(e.iter().all(|(c, _, _)| c != "sms"), "jamais de SMS : {e:?}");
    }
    assert!(avis(&ent, etab, sec, AVIS_LIEN_POSE.0).await.is_empty(), "l'auteur n'est pas prévenu");
    assert!(avis(&ent, etab, dir, AVIS_LIEN_POSE.0).await.is_empty());

    // Chaîne : relier la fiche sans compte prévient les deux comptes déjà reliés (l'individu entier), une fois chacun
    // pour ce lien.
    let l2 = ent
        .individu_relier_for_actor(etab, dir, "direction", p_sans, p_parent, "fiche_sans_compte", false)
        .await
        .unwrap();
    assert_eq!(avis(&ent, etab, prof, AVIS_LIEN_POSE.0).await.len(), 2);
    assert_eq!(avis(&ent, etab, parent, AVIS_LIEN_POSE.0).await.len(), 2);

    // Ce que la personne voit : deux liens, sans les fiches.
    let moi = ent.individu_moi_for_actor(etab, prof).await.unwrap();
    assert_eq!(moi.liens.len(), 2);
    assert_eq!(moi.liens[0].origine, "declaration_direction");
    assert_eq!(moi.liens[0].motif_code, "fiche_sans_compte", "le plus récent d'abord");
    let json = serde_json::to_string(&moi).unwrap();
    for f in [p_prof, p_parent, p_sans] {
        assert!(!json.contains(&f.to_string()), "aucune fiche dans « moi » : {json}");
    }
    assert!(!json.contains("Durand"));
    // Un autre compte de l'établissement ne voit aucun lien.
    assert!(ent.individu_moi_for_actor(etab, sec).await.unwrap().liens.is_empty());

    // Lever l1 : prévient les deux comptes (calculés avant la levée), pas la direction qui lève.
    ent.individu_lever_for_actor(etab, dir, "direction", l1, "erreur").await.unwrap();
    for u in [prof, parent] {
        let a = avis(&ent, etab, u, AVIS_LIEN_LEVE.0).await;
        assert_eq!(a.len(), 1, "{u}");
        assert_eq!(a[0].0, AVIS_LIEN_LEVE.1);
    }
    assert!(avis(&ent, etab, dir, AVIS_LIEN_LEVE.0).await.is_empty());
    // Après la levée, le professeur n'a plus de lien ; le parent garde celui de sa fiche sans compte.
    assert!(ent.individu_moi_for_actor(etab, prof).await.unwrap().liens.is_empty());
    assert_eq!(ent.individu_moi_for_actor(etab, parent).await.unwrap().liens.len(), 1);
    let _ = l2;
}

/// La cible d'une déclaration est prévenue ; le déclarant ne l'est qu'à la confirmation, jamais au refus (une adresse
/// inconnue et un refus restent indiscernables pour lui).
#[tokio::test]
async fn individu_avis_declaration() {
    let Some(ent) = connecter().await else {
        return eprintln!("skip : ENT_DATABASE_URL");
    };
    let etab = Uuid::new_v4();
    let (prof, _) = compte(&ent, etab, "prof").await;
    let (parent, email_parent) = compte(&ent, etab, "parent").await;
    let (cpe, _) = compte(&ent, etab, "cpe").await;
    let (prof2, email_prof2) = compte(&ent, etab, "prof").await;
    fiche_du_compte(&ent, etab, prof).await;
    fiche_du_compte(&ent, etab, parent).await;

    // Adresse inconnue : rien n'est créé, personne n'est prévenu.
    ent.individu_declarer_for_actor(etab, prof, "personne@nulle-part.example").await.unwrap();
    // Deux déclarations vers la même cible : un seul avis (une seule déclaration en attente).
    ent.individu_declarer_for_actor(etab, prof, &email_parent).await.unwrap();
    ent.individu_declarer_for_actor(etab, prof, &email_parent).await.unwrap();
    let a = avis(&ent, etab, parent, AVIS_DECLARATION_RECUE.0).await;
    assert_eq!(a.len(), 1);
    assert!(!a[0].1.contains("Nom prof"), "le nom du déclarant reste dans l'ENT, pas dans l'avis : {}", a[0].1);
    assert!(avis(&ent, etab, prof, AVIS_DECLARATION_RECUE.0).await.is_empty());

    let d = ent.individu_moi_for_actor(etab, parent).await.unwrap().declarations_recues[0].id;
    ent.individu_repondre_for_actor(etab, parent, d, true).await.unwrap();
    assert_eq!(avis(&ent, etab, prof, AVIS_DECLARATION_CONFIRMEE.0).await.len(), 1, "le déclarant est prévenu");
    assert!(avis(&ent, etab, parent, AVIS_DECLARATION_CONFIRMEE.0).await.is_empty(), "la cible a répondu elle-même");

    // Refus : le déclarant n'apprend rien.
    ent.individu_declarer_for_actor(etab, cpe, &email_prof2).await.unwrap();
    let d2 = ent.individu_moi_for_actor(etab, prof2).await.unwrap().declarations_recues[0].id;
    ent.individu_repondre_for_actor(etab, prof2, d2, false).await.unwrap();
    let mut tx = TenantTx::begin(ent.pool(), etab).await.unwrap();
    let n: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM ent_notification WHERE etablissement_id = $1 AND destinataire_user_id = $2",
    )
    .bind(etab)
    .bind(cpe)
    .fetch_one(&mut *tx)
    .await
    .unwrap();
    assert_eq!(n, 0, "aucun avis au déclarant refusé");
    drop(tx);

    // Après un refus, le même déclarant ne relance pas la cible (30 jours) : même réponse neutre, aucun avis de plus.
    let avant = avis(&ent, etab, prof2, AVIS_DECLARATION_RECUE.0).await.len();
    for _ in 0..3 {
        ent.individu_declarer_for_actor(etab, cpe, &email_prof2).await.unwrap();
    }
    assert_eq!(avis(&ent, etab, prof2, AVIS_DECLARATION_RECUE.0).await.len(), avant, "pas de relance après refus");
    assert!(ent.individu_moi_for_actor(etab, prof2).await.unwrap().declarations_recues.is_empty());
}

/// Une confirmation prévient tous les autres comptes de l'individu ainsi formé, pas seulement le déclarant.
#[tokio::test]
async fn individu_avis_confirmation_previent_tout_l_individu() {
    let Some(ent) = connecter().await else {
        return eprintln!("skip : ENT_DATABASE_URL");
    };
    let etab = Uuid::new_v4();
    let (sec, _) = compte(&ent, etab, "administratif").await;
    let (prof, _) = compte(&ent, etab, "prof").await;
    let (parent, email_parent) = compte(&ent, etab, "parent").await;
    let (agent2, _) = compte(&ent, etab, "cpe").await;
    let p_prof = fiche_du_compte(&ent, etab, prof).await;
    fiche_du_compte(&ent, etab, parent).await;
    let p_agent2 = fiche_du_compte(&ent, etab, agent2).await;
    // Deux comptes du professeur déjà reliés par le secrétariat.
    ent.individu_relier_for_actor(etab, sec, "administratif", p_prof, p_agent2, "doublon_agent", false)
        .await
        .unwrap();
    ent.individu_declarer_for_actor(etab, prof, &email_parent).await.unwrap();
    let d = ent.individu_moi_for_actor(etab, parent).await.unwrap().declarations_recues[0].id;
    ent.individu_repondre_for_actor(etab, parent, d, true).await.unwrap();
    assert_eq!(avis(&ent, etab, prof, AVIS_DECLARATION_CONFIRMEE.0).await.len(), 1);
    assert_eq!(avis(&ent, etab, agent2, AVIS_DECLARATION_CONFIRMEE.0).await.len(), 1, "l'autre compte déjà relié");
    assert!(avis(&ent, etab, parent, AVIS_DECLARATION_CONFIRMEE.0).await.is_empty(), "pas la cible qui confirme");
}

/// Les textes tiennent dans un push (240 caractères) et ne citent jamais un nom.
#[test]
fn individu_avis_textes_courts() {
    for (kind, titre, corps) in [AVIS_LIEN_POSE, AVIS_LIEN_LEVE, AVIS_DECLARATION_RECUE, AVIS_DECLARATION_CONFIRMEE] {
        assert!(corps.chars().count() <= AVIS_TEXTE_MAX, "{kind} : {} caractères", corps.chars().count());
        assert!(titre.chars().count() <= 60, "{kind}");
        assert!(!corps.contains("  "), "{kind} : espace double (continuation de ligne)");
    }
}
