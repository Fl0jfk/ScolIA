//! Socle 27, **lot G3b-3** : fichiers remis (PDF, CSV), scellement, remise par passkey récente, téléchargement,
//! purge à 7 jours (jour **injecté**, règle 33), clôture par empreintes, renvois, Pf7 et la règle M-2711
//! (les messages de la mère avec le CPE ne sont jamais dans l'extrait du père). Base exigée sous `CI`.

use super::tests_rgpd_g3b2::{
    demande, labo, parents_separes, personne_de, ADF, ADRESSE_AUTRE, DIR, TEL_AUTRE,
};
use super::*;
use crate::droits_extrait::ContenuExtrait;
use crate::droits_remise::{
    aad_fichier, base64_decoder, csv_section, lignes_pdf, Emargement, ModeRemise, USAGE_SCELLE,
};
use crate::service::RefusMetier;
use chrono::{DateTime, Duration, Utc};
use uuid::Uuid;
use webauthn_authenticator_rs::prelude::WebauthnAuthenticator;
use webauthn_authenticator_rs::softpasskey::SoftPasskey;

fn origine() -> url::Url {
    url::Url::parse("http://localhost:3000").unwrap()
}

/// Horloge **injectée** : jamais l'heure réelle dans un test.
fn t0() -> DateTime<Utc> {
    DateTime::parse_from_rfc3339("2027-01-15T10:00:00Z")
        .unwrap()
        .with_timezone(&Utc)
}

async fn enroler(ent: &PgEnt, etab: Uuid, user: Uuid) -> WebauthnAuthenticator<SoftPasskey> {
    let mut dev = WebauthnAuthenticator::new(SoftPasskey::new(true));
    let (token, options) = ent
        .passkey_register_start_for_actor(etab, user)
        .await
        .expect("start enrôlement");
    let ccr: ent_auth::CreationChallengeResponse = serde_json::from_value(options).unwrap();
    let reg = dev.do_registration(origine(), ccr).unwrap();
    ent.passkey_register_finish_for_actor(etab, user, &token, &reg, Some("Clé".into()))
        .await
        .expect("enrôlement");
    dev
}

fn assertion(
    dev: &mut WebauthnAuthenticator<SoftPasskey>,
    options: &serde_json::Value,
) -> ent_auth::PublicKeyCredential {
    let rcr: ent_auth::RequestChallengeResponse = serde_json::from_value(options.clone()).unwrap();
    dev.do_authentication(origine(), rcr).unwrap()
}

fn est_conflit(e: &EntDbError) -> bool {
    matches!(
        e,
        EntDbError::Refus(RefusMetier::Conflit, _) | EntDbError::ConflitDossier(_)
    )
}

fn est_introuvable(e: &EntDbError) -> bool {
    matches!(e, EntDbError::Domain(domain_eleve::DomainError::NotFound))
}

/// Extrait assemblé et relu pour la demande du parent de Léa : tous les marquages décidés « masqués ».
async fn extrait_pret(ent: &PgEnt, cast: &LaboCast, d: Uuid) -> Uuid {
    let (etab, dir) = (cast.etablissement_id, cast.direction.user_id);
    let e = ent
        .droits_extrait_assembler(etab, dir, DIR, d, None)
        .await
        .unwrap();
    let id = e[0].id;
    for m in ent.droits_masquages(etab, dir, DIR, d, id).await.unwrap() {
        if m.decision == "a_decider" {
            ent.droits_masquage_decider(etab, dir, DIR, d, m.id, "masque", None)
                .await
                .unwrap();
        }
    }
    id
}

/// Le contenu **remis** (masques appliqués), tel que les fichiers le portent.
async fn contenu_remis(ent: &PgEnt, cast: &LaboCast, d: Uuid, extrait: Uuid) -> ContenuExtrait {
    let etab = cast.etablissement_id;
    let mut tx = crate::tenant_tx::TenantTx::begin(ent.pool(), etab)
        .await
        .unwrap();
    let (_, c, _) = ent
        .contenu_a_remettre(&mut tx, etab, d, extrait)
        .await
        .unwrap();
    c
}

/// Les octets lisibles de tous les fichiers d'un extrait : CSV tels quels, PDF réduit à ses lignes sources.
fn tout_le_texte(c: &ContenuExtrait, reference: &str) -> String {
    let mut t = lignes_pdf(c, reference, 1).join("\n");
    for s in c.sections.iter().filter(|s| s.lignes > 0) {
        t.push_str(&String::from_utf8_lossy(&csv_section(s)));
    }
    t
}

/// Remise en ligne complète : fichiers scellés, AAD, téléchargement par passkey du demandeur, 404 pour un autre
/// compte, journal, notification sans nom d'élève, purge à 7 jours avec un jour injecté.
#[tokio::test]
async fn g3c_remise_en_ligne_scellee_telechargement_et_purge() {
    let Some((ent, cast)) = labo().await else {
        return;
    };
    let (etab, dir, parent, tom) = (
        cast.etablissement_id,
        cast.direction.user_id,
        cast.parent_lea.user_id,
        cast.parent_tom.user_id,
    );
    parents_separes(&ent, &cast, true).await;
    let d = demande(&ent, &cast, "titulaire_autorite", Some(parent)).await;
    let extrait = extrait_pret(&ent, &cast, d.id).await;

    // Sans passkey enregistrée, le gestionnaire ne peut pas démarrer la remise.
    let r = ent
        .droits_remise_defi_a(etab, dir, DIR, d.id, extrait, t0())
        .await
        .unwrap_err();
    assert!(est_conflit(&r), "{r:?}");
    // Le secrétariat n'a pas `droits.remettre`.
    let r = ent
        .droits_remise_defi_a(etab, cast.administratif.user_id, ADF, d.id, extrait, t0())
        .await
        .unwrap_err();
    assert!(matches!(r, EntDbError::Forbidden), "{r:?}");

    let mut cle_dir = enroler(&ent, etab, dir).await;
    let mut cle_parent = enroler(&ent, etab, parent).await;

    // Passkey **périmée** : un défi de plus de cinq minutes ne remet rien.
    let defi = ent
        .droits_remise_defi_a(etab, dir, DIR, d.id, extrait, t0())
        .await
        .unwrap();
    assert!(defi.intention.contains("remets"));
    let cred = assertion(&mut cle_dir, &defi.options);
    let r = ent
        .droits_remettre_a(
            etab,
            dir,
            DIR,
            d.id,
            extrait,
            ModeRemise::EnLigne,
            defi.defi_id,
            &cred,
            None,
            t0() + Duration::minutes(6),
        )
        .await
        .unwrap_err();
    assert!(est_conflit(&r), "{r:?}");

    // Remise valide.
    let defi = ent
        .droits_remise_defi_a(etab, dir, DIR, d.id, extrait, t0())
        .await
        .unwrap();
    let cred = assertion(&mut cle_dir, &defi.options);
    // Un autre compte ne peut pas terminer le défi du gestionnaire.
    let r = ent
        .droits_remettre_a(
            etab,
            cast.administratif.user_id,
            ADF,
            d.id,
            extrait,
            ModeRemise::EnLigne,
            defi.defi_id,
            &cred,
            None,
            t0(),
        )
        .await
        .unwrap_err();
    assert!(matches!(r, EntDbError::Forbidden), "{r:?}");
    let faite = ent
        .droits_remettre_a(
            etab,
            dir,
            DIR,
            d.id,
            extrait,
            ModeRemise::EnLigne,
            defi.defi_id,
            &cred,
            None,
            t0(),
        )
        .await
        .expect("remise");
    let fichiers = &faite.remise.fichiers;
    assert!(fichiers
        .iter()
        .any(|f| f.nom == "extrait.pdf" && f.type_ == "pdf"));
    assert!(
        fichiers.iter().any(|f| f.type_ == "csv"),
        "un CSV par section"
    );
    // Rejeu du même défi : refusé (brûlé). Seconde remise du même extrait : refusée.
    let r = ent
        .droits_remettre_a(
            etab,
            dir,
            DIR,
            d.id,
            extrait,
            ModeRemise::EnLigne,
            defi.defi_id,
            &cred,
            None,
            t0(),
        )
        .await
        .unwrap_err();
    assert!(est_conflit(&r), "{r:?}");
    let jr = |n: i64| t0() + Duration::days(n);
    assert_eq!(
        fichiers[0].disponible_jusqu_le,
        jr(7),
        "mise à disposition de 7 jours"
    );

    // Le fichier est **scellé** au repos : ni PDF lisible, ni donnée du jeu d'essai dans l'objet.
    let (cle, vrai_extrait_version): (String, i32) = sqlx::query_as(
        "SELECT f.cle_objet, e.version FROM rgp_extrait_fichier f JOIN rgp_extrait e
           ON e.etablissement_id = f.etablissement_id AND e.id = f.extrait_id
          WHERE f.etablissement_id = $1 AND f.id = $2",
    )
    .bind(etab)
    .bind(fichiers[0].id)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    let scelle = ent.objet_get(&cle).await.unwrap();
    let s = String::from_utf8_lossy(&scelle).to_string();
    assert!(!s.contains("%PDF") && !s.contains("0600000001") && !s.contains("Granger"));
    // Ouverture : seulement avec la bonne AAD (un fichier recopié sur un autre extrait ne s'ouvre pas).
    let boite = ent_auth::SecretBox::from_env()
        .unwrap_or_else(|_| ent_auth::SecretBox::dev())
        .derive(USAGE_SCELLE);
    let nom = &fichiers[0].nom;
    let bonne = aad_fichier(etab, extrait, vrai_extrait_version, nom);
    let clair = boite.open(&s, &bonne);
    if let Ok(c) = &clair {
        assert!(base64_decoder(c).is_some());
    }
    for faux in [
        aad_fichier(etab, Uuid::new_v4(), vrai_extrait_version, nom),
        aad_fichier(etab, extrait, vrai_extrait_version + 1, nom),
        aad_fichier(etab, extrait, vrai_extrait_version, "autre.csv"),
    ] {
        assert!(
            boite.open(&s, &faux).is_err(),
            "une AAD différente n'ouvre pas le scellé"
        );
    }

    // Notification : sans nom d'élève.
    let avis: Vec<(String, String)> = sqlx::query_as(
        "SELECT titre, corps FROM ent_notification
          WHERE kind = 'rgpd.extrait_pret' AND destinataire_user_id = $1",
    )
    .bind(parent)
    .fetch_all(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(avis.len(), 1);
    assert!(
        !avis[0].1.contains("Léa") && !avis[0].1.contains("Granger"),
        "{:?}",
        avis[0]
    );

    // « Mes données » : le demandeur voit ses fichiers ; l'autre parent ne voit rien.
    let mes = ent.droits_mes_extraits(etab, parent).await.unwrap();
    assert_eq!(mes.len(), 1);
    assert_eq!(mes[0].remise.fichiers.len(), fichiers.len());
    assert!(ent.droits_mes_extraits(etab, tom).await.unwrap().is_empty());

    // Téléchargement : un autre compte reçoit 404 ; le demandeur passe par sa passkey.
    let r = ent
        .droits_telechargement_defi(etab, tom, extrait, t0())
        .await
        .unwrap_err();
    assert!(est_introuvable(&r), "{r:?}");
    let defi = ent
        .droits_telechargement_defi(etab, parent, extrait, jr(1))
        .await
        .unwrap();
    let cred = assertion(&mut cle_parent, &defi.options);
    let r = ent
        .droits_telecharger(etab, tom, fichiers[0].id, defi.defi_id, &cred, jr(1))
        .await
        .unwrap_err();
    assert!(est_introuvable(&r), "{r:?}");
    let servi = ent
        .droits_telecharger(etab, parent, fichiers[0].id, defi.defi_id, &cred, jr(1))
        .await
        .expect("téléchargement");
    assert_eq!(servi.nom, fichiers[0].nom);
    assert!(
        servi.octets.starts_with(b"%PDF"),
        "le PDF est servi déchiffré, au demandeur seulement"
    );
    // Tous les fichiers servis : aucun nom ni coordonnée de l'autre parent.
    for f in fichiers {
        let defi = ent
            .droits_telechargement_defi(etab, parent, extrait, jr(1))
            .await
            .unwrap();
        let cred = assertion(&mut cle_parent, &defi.options);
        let s = ent
            .droits_telecharger(etab, parent, f.id, defi.defi_id, &cred, jr(1))
            .await
            .unwrap();
        if f.type_ == "csv" {
            let t = String::from_utf8(s.octets.clone()).unwrap();
            assert!(t.starts_with('\u{feff}'), "BOM UTF-8 pour les tableurs");
            for tiers in [TEL_AUTRE, ADRESSE_AUTRE, "Dupont"] {
                assert!(!t.contains(tiers), "{tiers} dans {}", f.nom);
            }
        }
    }
    // Chaque téléchargement est compté et journalisé.
    let nb: i64 =
        sqlx::query_scalar("SELECT count(*) FROM rgp_telechargement WHERE etablissement_id = $1")
            .bind(etab)
            .fetch_one(ent.pool_verif())
            .await
            .unwrap();
    assert_eq!(nb as usize, fichiers.len() + 1);
    let evts: Vec<String> = sqlx::query_scalar(
        "SELECT evenement FROM rgp_demande_journal WHERE etablissement_id = $1 AND demande_id = $2",
    )
    .bind(etab)
    .bind(d.id)
    .fetch_all(ent.pool_verif())
    .await
    .unwrap();
    assert!(
        evts.iter().any(|e| e == "extrait_remis") && evts.iter().any(|e| e == "extrait_telecharge")
    );

    // Purge : rien avant l'échéance ; à 7 jours, objets supprimés puis « purgé » ; plus de téléchargement.
    assert_eq!(
        ent.droits_extraits_purger_etab_a(etab, jr(6))
            .await
            .unwrap(),
        0
    );
    assert!(ent.objet_existe(&cle).await.unwrap());
    let n = ent
        .droits_extraits_purger_etab_a(etab, jr(8))
        .await
        .unwrap();
    assert_eq!(n, fichiers.len());
    assert!(!ent.objet_existe(&cle).await.unwrap(), "objet supprimé");
    let purges: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM rgp_extrait_fichier WHERE etablissement_id = $1 AND purge_le IS NOT NULL",
    )
    .bind(etab)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(purges as usize, fichiers.len());
    assert_eq!(
        ent.droits_extraits_purger_etab_a(etab, jr(9))
            .await
            .unwrap(),
        0,
        "idempotent"
    );
    let defi = ent
        .droits_telechargement_defi(etab, parent, extrait, jr(9))
        .await
        .unwrap();
    let cred = assertion(&mut cle_parent, &defi.options);
    let r = ent
        .droits_telecharger(etab, parent, fichiers[0].id, defi.defi_id, &cred, jr(9))
        .await
        .unwrap_err();
    assert!(est_conflit(&r), "{r:?}");
}

/// Remise en main propre : émargement obligatoire, PDF rendu une fois pour l'impression, rien de stocké ; un
/// demandeur sans compte ne reçoit jamais de remise en ligne, et aucune adresse de la demande n'est utilisée.
#[tokio::test]
async fn g3c_remise_en_main_propre_et_demandeur_sans_compte() {
    let Some((ent, cast)) = labo().await else {
        return;
    };
    let (etab, dir) = (cast.etablissement_id, cast.direction.user_id);
    parents_separes(&ent, &cast, true).await;
    // Demandeur sans compte : la demande porte un courriel, jamais utilisé pour la remise.
    let d = demande(&ent, &cast, "soi_meme", None).await;
    let extrait = extrait_pret(&ent, &cast, d.id).await;
    let mut cle = enroler(&ent, etab, dir).await;

    let defi = ent
        .droits_remise_defi_a(etab, dir, DIR, d.id, extrait, t0())
        .await
        .unwrap();
    let cred = assertion(&mut cle, &defi.options);
    let r = ent
        .droits_remettre_a(
            etab,
            dir,
            DIR,
            d.id,
            extrait,
            ModeRemise::EnLigne,
            defi.defi_id,
            &cred,
            None,
            t0(),
        )
        .await
        .unwrap_err();
    assert!(
        est_conflit(&r),
        "sans compte : remise en main propre, jamais en ligne : {r:?}"
    );

    // Sans émargement : refus de saisie ; avec émargement : PDF à imprimer.
    let defi = ent
        .droits_remise_defi_a(etab, dir, DIR, d.id, extrait, t0())
        .await
        .unwrap();
    let cred = assertion(&mut cle, &defi.options);
    let r = ent
        .droits_remettre_a(
            etab,
            dir,
            DIR,
            d.id,
            extrait,
            ModeRemise::MainPropre,
            defi.defi_id,
            &cred,
            None,
            t0(),
        )
        .await
        .unwrap_err();
    assert!(matches!(r, EntDbError::SaisieInvalide(_)), "{r:?}");
    let faite = ent
        .droits_remettre_a(
            etab,
            dir,
            DIR,
            d.id,
            extrait,
            ModeRemise::MainPropre,
            defi.defi_id,
            &cred,
            Some(Emargement {
                nom: "Mme Granger".into(),
                piece_vue: "carte d'identité".into(),
            }),
            t0(),
        )
        .await
        .expect("remise en main propre");
    assert!(faite
        .pdf_a_imprimer
        .as_deref()
        .is_some_and(|p| p.starts_with(b"%PDF")));
    assert!(
        faite.remise.fichiers.is_empty(),
        "rien n'est stocké pour une remise en main propre"
    );
    let (mode, nom, piece): (String, Option<String>, Option<String>) = sqlx::query_as(
        "SELECT mode, emargement_nom, emargement_piece_vue FROM rgp_remise WHERE etablissement_id = $1 AND extrait_id = $2",
    )
    .bind(etab)
    .bind(extrait)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(
        (mode.as_str(), nom.as_deref(), piece.as_deref()),
        ("main_propre", Some("Mme Granger"), Some("carte d'identité"))
    );
    // Aucune notification à qui que ce soit : la remise ne part pas d'une adresse de la demande.
    let n: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM ent_notification WHERE kind = 'rgpd.extrait_pret' AND etablissement_id = $1",
    )
    .bind(etab)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(n, 0);
}

/// Un extrait dont un marquage reste à décider, ou périmé, ne se remet pas.
#[tokio::test]
async fn g3c_remise_refusee_tant_qu_un_marquage_est_ouvert() {
    let Some((ent, cast)) = labo().await else {
        return;
    };
    let (etab, dir) = (cast.etablissement_id, cast.direction.user_id);
    parents_separes(&ent, &cast, true).await;
    let d = demande(
        &ent,
        &cast,
        "titulaire_autorite",
        Some(cast.parent_lea.user_id),
    )
    .await;
    let e = ent
        .droits_extrait_assembler(etab, dir, DIR, d.id, None)
        .await
        .unwrap();
    let _cle = enroler(&ent, etab, dir).await;
    let r = ent
        .droits_remise_defi_a(etab, dir, DIR, d.id, e[0].id, t0())
        .await
        .unwrap_err();
    assert!(est_conflit(&r), "marquage ouvert : {r:?}");
}

/// Clôture : éléments remis par **empreinte** (jamais les données) ; renvois tracés (rectification, SIECLE, retrait
/// de consentement, CADA d'un tiers).
#[tokio::test]
async fn g3c_cloture_par_empreintes_et_renvois() {
    let Some((ent, cast)) = labo().await else {
        return;
    };
    let (etab, dir, parent) = (
        cast.etablissement_id,
        cast.direction.user_id,
        cast.parent_lea.user_id,
    );
    parents_separes(&ent, &cast, true).await;
    let d = demande(&ent, &cast, "titulaire_autorite", Some(parent)).await;
    let extrait = extrait_pret(&ent, &cast, d.id).await;
    let mut cle = enroler(&ent, etab, dir).await;
    let defi = ent
        .droits_remise_defi_a(etab, dir, DIR, d.id, extrait, t0())
        .await
        .unwrap();
    let cred = assertion(&mut cle, &defi.options);
    let faite = ent
        .droits_remettre_a(
            etab,
            dir,
            DIR,
            d.id,
            extrait,
            ModeRemise::EnLigne,
            defi.defi_id,
            &cred,
            None,
            t0(),
        )
        .await
        .unwrap();

    // Renvois.
    let r = ent
        .droits_renvoi_tracer(
            etab,
            dir,
            DIR,
            d.id,
            "rectification",
            "SIECLE",
            "adresse de naissance à corriger",
        )
        .await
        .unwrap();
    assert!(r.texte.contains("à corriger dans la source"), "{}", r.texte);
    let r = ent
        .droits_renvoi_tracer(
            etab,
            dir,
            DIR,
            d.id,
            "retrait_consentement",
            "",
            "retrait du droit à l'image",
        )
        .await
        .unwrap();
    assert_eq!(r.cible, "socle-18");
    ent.droits_renvoi_tracer(
        etab,
        dir,
        DIR,
        d.id,
        "cada_tiers",
        "CADA",
        "document occulté des noms de tiers",
    )
    .await
    .unwrap();
    assert!(ent
        .droits_renvoi_tracer(etab, dir, DIR, d.id, "inconnu", "x", "texte")
        .await
        .is_err());
    assert_eq!(
        ent.droits_renvois(etab, dir, DIR, d.id)
            .await
            .unwrap()
            .len(),
        3
    );
    // Le secrétariat ne lit pas les renvois d'une demande qu'il ne traite pas : `droits.traiter` suffit, `recevoir` non.
    // Clôture : droits décidés d'abord.
    ent.droits_decider(etab, dir, DIR, d.id, "acces", "accorde", None, None)
        .await
        .unwrap();
    let v = ent
        .droits_cloturer(
            etab,
            dir,
            DIR,
            d.id,
            "Extrait remis en ligne le 15 janvier.",
        )
        .await
        .unwrap();
    assert_eq!(v.etat, "close");
    let (elements, refus) = ent
        .droits_cloture_lire(etab, dir, DIR, d.id)
        .await
        .unwrap()
        .expect("clôture consignée");
    let el = elements.as_array().unwrap();
    assert_eq!(el.len(), 1);
    assert_eq!(
        el[0]["empreinte"].as_str().unwrap(),
        faite.remise.empreinte_extrait
    );
    assert!(el[0]["sections"]
        .as_array()
        .unwrap()
        .iter()
        .all(|s| s["empreinte"].is_string() || s["statut"] != "ok"));
    let brut = format!("{elements}{refus}");
    for donnee in [TEL_AUTRE, ADRESSE_AUTRE, "Dupont", "0600000001"] {
        assert!(
            !brut.contains(donnee),
            "la clôture ne garde que des empreintes : {donnee}"
        );
    }
}

/// M-2711 : les messages de la mère avec le CPE ne figurent ni dans l'extrait du père, ni dans son PDF, ni dans ses CSV ;
/// ceux du père avec le CPE y figurent.
#[tokio::test]
async fn g3c_m2711_messages_de_la_mere_avec_le_cpe_absents_de_l_extrait_du_pere() {
    let Some((ent, cast)) = labo().await else {
        return;
    };
    let (etab, dir) = (cast.etablissement_id, cast.direction.user_id);
    let mere = parents_separes(&ent, &cast, false).await; // personne du parent de Tom : l'autre parent
    let pere = personne_de(&ent, etab, cast.parent_lea.user_id).await;
    let cpe = personne_de(&ent, etab, cast.cpe.user_id).await;
    let p = ent.pool_verif();
    let fil = |auteur: Uuid, famille: Uuid, sujet: &'static str, corps: &'static str| {
        let (fid, mid) = (Uuid::new_v4(), Uuid::new_v4());
        let eleve = cast.eleve_lea_id;
        async move {
            sqlx::query(
                "INSERT INTO ent_msg_fil (id, etablissement_id, eleve_id, sujet, created_by_user_id)
                 VALUES ($1,$2,$3,$4,$5)",
            )
            .bind(fid).bind(etab).bind(eleve).bind(sujet).bind(auteur)
            .execute(p).await.expect("fil");
            sqlx::query("INSERT INTO ent_msg_fil_eleve (etablissement_id, fil_id, eleve_id) VALUES ($1,$2,$3)")
                .bind(etab).bind(fid).bind(eleve).execute(p).await.expect("fil_eleve");
            for (pers, cote, qualite, raison) in [
                (famille, "famille", "destinataire", "titulaire_foyer"),
                (cpe, "etablissement", "auteur", "vie_scolaire"),
            ] {
                sqlx::query(
                    "INSERT INTO ent_msg_participant (id, etablissement_id, fil_id, personne_id, cote, qualite, raison)
                     VALUES ($1,$2,$3,$4,$5,$6,$7)",
                )
                .bind(Uuid::new_v4()).bind(etab).bind(fid).bind(pers).bind(cote).bind(qualite).bind(raison)
                .execute(p).await.expect("participant");
            }
            sqlx::query("INSERT INTO ent_msg (id, etablissement_id, fil_id, auteur_user_id, corps) VALUES ($1,$2,$3,$4,$5)")
                .bind(mid).bind(etab).bind(fid).bind(auteur).bind(corps)
                .execute(p).await.expect("message");
        }
    };
    fil(
        cast.cpe.user_id,
        mere,
        "Rendez-vous",
        "SECRET-MERE-CPE message confidentiel de la mère",
    )
    .await;
    fil(
        cast.cpe.user_id,
        pere,
        "Retard",
        "VISIBLE-PERE-CPE message adressé au père",
    )
    .await;

    let d = demande(
        &ent,
        &cast,
        "titulaire_autorite",
        Some(cast.parent_lea.user_id),
    )
    .await;
    let e = ent
        .droits_extrait_assembler(etab, dir, DIR, d.id, None)
        .await
        .unwrap();
    let id = e[0].id;
    for m in ent
        .droits_masquages(etab, dir, DIR, d.id, id)
        .await
        .unwrap()
    {
        if m.decision == "a_decider" {
            ent.droits_masquage_decider(etab, dir, DIR, d.id, m.id, "masque", None)
                .await
                .unwrap();
        }
    }
    let c = contenu_remis(&ent, &cast, d.id, id).await;
    let texte = tout_le_texte(&c, "2027-001");
    assert!(
        texte.contains("VISIBLE-PERE-CPE"),
        "les échanges du père figurent dans ses fichiers"
    );
    assert!(
        !texte.contains("SECRET-MERE-CPE"),
        "les messages de la mère avec le CPE ne sont ni dans le PDF ni dans les CSV du père"
    );
    // L'extrait brut de la direction, lui non plus, ne les contient pas : ce n'est pas un masquage, la règle de
    // sélection des fils ne les choisit jamais.
    let brut = ent
        .droits_extrait_lire(etab, dir, DIR, d.id, id)
        .await
        .unwrap();
    let t = format!("{:?}", brut.sections);
    assert!(!t.contains("SECRET-MERE-CPE") && t.contains("VISIBLE-PERE-CPE"));
}

/// Pf7 : un dépôt sans compte « pour soi » n'est vérifié que sur une personne rapprochée ; la pièce se choisit dans
/// une liste (jamais un identifiant saisi).
#[tokio::test]
async fn g3c_pf7_sans_compte_pour_soi_exige_un_sujet_rapproche() {
    let Some((ent, cast)) = labo().await else {
        return;
    };
    let (etab, dir) = (cast.etablissement_id, cast.direction.user_id);
    let jour = ent.aujourdhui_etablissement(etab).await.unwrap();
    let v = ent
        .droits_saisir_au_guichet(
            etab,
            dir,
            DIR,
            crate::droits_demandes::SaisieDemande {
                canal: "courrier",
                recue_le: jour,
                demandeur_compte_id: None,
                demandeur_personne_id: None,
                demandeur_nom_saisi: Some("Ancien élève fictif"),
                demandeur_contact: Some("courrier postal"),
                qualite: "ancien_eleve",
                sujet_type: "inconnu",
                sujet_id: None,
                sujet_nom_saisi: Some("Ancien élève fictif"),
                droits: &["acces"],
                perimetre_texte: None,
                perimetre_du: None,
                perimetre_au: None,
            },
        )
        .await
        .expect("saisie");
    // Sujet non rapproché : la vérification réussie est refusée.
    let r = ent
        .droits_verifier_identite(etab, dir, DIR, v.id, "code_courriel", true, None, None)
        .await
        .unwrap_err();
    assert!(matches!(r, EntDbError::SaisieInvalide(_)), "{r:?}");
    // Une vérification **échouée** reste possible (rien n'est déclaré vérifié).
    ent.droits_verifier_identite(
        etab,
        dir,
        DIR,
        v.id,
        "code_courriel",
        false,
        Some("pièce illisible"),
        None,
    )
    .await
    .expect("échec consigné");
    // Rapproché d'un dossier : la vérification peut réussir.
    ent.droits_rapprocher(
        etab,
        dir,
        DIR,
        v.id,
        Some(("eleve", cast.eleve_tom_id)),
        None,
    )
    .await
    .unwrap();
    let ok = ent
        .droits_verifier_identite(etab, dir, DIR, v.id, "code_courriel", true, None, None)
        .await
        .unwrap();
    assert!(ok.identites.iter().any(|i| i.resultat == "verifiee"));
    // La pièce se choisit dans le coffre : la liste ne porte aucun nom ni contenu.
    let liste = ent
        .droits_pieces_identite_a_choisir(etab, dir, DIR, v.id)
        .await
        .unwrap();
    assert!(liste.iter().all(|p| !p.etat.is_empty()));
    let r = ent
        .droits_pieces_identite_a_choisir(etab, cast.prof.user_id, "professeur", v.id)
        .await
        .unwrap_err();
    assert!(matches!(r, EntDbError::Forbidden), "{r:?}");
}

// ───────────────────── Correctifs de la contre-revue des lots 1 et 2 ─────────────────────

const TEL_MERE: &str = "0677889900";
const ADRESSE_MERE: &str = "7 chemin Introuvable 33000 Bordeaux";

/// Une mère **sans compte** : fiche personne, lien de responsabilité sans compte, responsable saisi au dossier
/// d'inscription **sans fiche rapprochée**. Rend sa personne.
async fn mere_sans_compte(ent: &PgEnt, cast: &LaboCast, protegee: bool) -> Uuid {
    let etab = cast.etablissement_id;
    let p = ent.pool_verif();
    let mere = Uuid::new_v4();
    sqlx::query("INSERT INTO ent_personne (id, etablissement_id, nom, prenom) VALUES ($1, $2, 'Lemaire', 'Odile')")
        .bind(mere)
        .bind(etab)
        .execute(p)
        .await
        .expect("personne sans compte");
    sqlx::query(
        "INSERT INTO ent_eleve_lien (id, etablissement_id, eleve_id, user_id, personne_id, autorite_parentale)
         VALUES ($1, $2, $3, NULL, $4, true)",
    )
    .bind(Uuid::new_v4())
    .bind(etab)
    .bind(cast.eleve_lea_id)
    .bind(mere)
    .execute(p)
    .await
    .expect("lien sans compte");
    if protegee {
        sqlx::query(
            "INSERT INTO ent_adresse_protegee (id, etablissement_id, personne_id, adresse, origine)
             VALUES ($1, $2, $3, 'enc:v1:fictif', 'saisie')",
        )
        .bind(Uuid::new_v4())
        .bind(etab)
        .bind(mere)
        .execute(p)
        .await
        .expect("protection");
    }
    let campagne = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO ent_inscription_campagne (id, etablissement_id, nature, libelle, annee_cible)
         VALUES ($1, $2, 'reinscription', 'Fictive', '2026-2027')",
    )
    .bind(campagne)
    .bind(etab)
    .execute(p)
    .await
    .unwrap();
    let dossier = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO ent_inscription_dossier (id, etablissement_id, campagne_id, nature, eleve_id, eleve_nom, eleve_prenom)
         VALUES ($1, $2, $3, 'reinscription', $4, 'Granger', 'Léa')",
    )
    .bind(dossier)
    .bind(etab)
    .bind(campagne)
    .bind(cast.eleve_lea_id)
    .execute(p)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO ent_inscription_responsable (id, etablissement_id, dossier_id, rang, nom, telephone, adresse, lien, personne_id)
         VALUES ($1, $2, $3, 2, 'Lemaire', $4, $5, 'autre', NULL)",
    )
    .bind(Uuid::new_v4())
    .bind(etab)
    .bind(dossier)
    .bind(TEL_MERE)
    .bind(ADRESSE_MERE)
    .execute(p)
    .await
    .expect("responsable sans fiche");
    mere
}

/// E1 : une autre responsable **sans compte** (protégée ou non) ne fait pas échouer l'assemblage, et ses données
/// (nom, téléphone, adresse protégée) ne sortent pas dans l'extrait du père, même sans fiche rapprochée.
#[tokio::test]
async fn g3c_e1_autre_parent_sans_compte_marque_meme_protege() {
    for protegee in [true, false] {
        let Some((ent, cast)) = labo().await else {
            return;
        };
        let (etab, dir) = (cast.etablissement_id, cast.direction.user_id);
        mere_sans_compte(&ent, &cast, protegee).await;
        let d = demande(
            &ent,
            &cast,
            "titulaire_autorite",
            Some(cast.parent_lea.user_id),
        )
        .await;
        assert_eq!(
            d.etat, "en_cours",
            "Pf3 : la protection de l'autre parent ne bloque pas le titulaire"
        );
        let e = ent
            .droits_extrait_assembler(etab, dir, DIR, d.id, None)
            .await
            .expect("l'assemblage ne tombe pas sur un responsable sans compte");
        assert!(e[0].masquages_a_decider > 0);
        let brut = ent
            .droits_extrait_lire(etab, dir, DIR, d.id, e[0].id)
            .await
            .unwrap();
        assert!(
            format!("{:?}", brut.sections).contains(TEL_MERE),
            "le fixture atteint l'extrait brut"
        );
        if protegee {
            assert!(brut
                .marquages
                .iter()
                .any(|m| m.raison == "adresse_protegee"));
        }
        for m in &brut.marquages {
            ent.droits_masquage_decider(etab, dir, DIR, d.id, m.id, "masque", None)
                .await
                .unwrap();
        }
        let c = contenu_remis(&ent, &cast, d.id, e[0].id).await;
        let t = tout_le_texte(&c, "2027-001");
        for donnee in [TEL_MERE, ADRESSE_MERE, "Lemaire"] {
            assert!(
                !t.contains(donnee),
                "{donnee} ne sort pas (protégée : {protegee})"
            );
        }
    }
}

/// E2 (M-2711) : un père sorti d'un fil ne reçoit pas les messages postérieurs à son départ ; celui qui entre tard
/// ne reçoit pas l'historique antérieur ; les messages de sa période, eux, figurent.
#[tokio::test]
async fn g3c_e2_messages_bornes_par_la_periode_de_participation() {
    let Some((ent, cast)) = labo().await else {
        return;
    };
    let (etab, dir) = (cast.etablissement_id, cast.direction.user_id);
    let mere = parents_separes(&ent, &cast, false).await;
    let pere = personne_de(&ent, etab, cast.parent_lea.user_id).await;
    let cpe = personne_de(&ent, etab, cast.cpe.user_id).await;
    let p = ent.pool_verif();
    // Fil « sortie » : le père participe du -10 j au -5 j (séparation) ; la mère reste.
    // Fil « entrée tardive » : le père n'entre qu'au -1 j.
    for (sujet, du_pere, au_pere, messages) in [
        (
            "Sortie",
            "now() - interval '10 days'",
            Some("now() - interval '5 days'"),
            [
                ("-7 days", "AVANT-DEPART visible"),
                ("-2 days", "APRES-DEPART invisible"),
            ],
        ),
        (
            "Entree",
            "now() - interval '1 day'",
            None,
            [
                ("-3 days", "AVANT-ENTREE invisible"),
                ("-1 hour", "APRES-ENTREE visible"),
            ],
        ),
    ] {
        let fil = Uuid::new_v4();
        sqlx::query("INSERT INTO ent_msg_fil (id, etablissement_id, eleve_id, sujet, created_by_user_id) VALUES ($1,$2,$3,$4,$5)")
            .bind(fil).bind(etab).bind(cast.eleve_lea_id).bind(sujet).bind(cast.cpe.user_id)
            .execute(p).await.expect("fil");
        sqlx::query(
            "INSERT INTO ent_msg_fil_eleve (etablissement_id, fil_id, eleve_id) VALUES ($1,$2,$3)",
        )
        .bind(etab)
        .bind(fil)
        .bind(cast.eleve_lea_id)
        .execute(p)
        .await
        .expect("fil_eleve");
        for (pers, cote, qualite, raison, du, au) in [
            (
                cpe,
                "etablissement",
                "auteur",
                "vie_scolaire",
                "now() - interval '30 days'",
                None,
            ),
            (
                mere,
                "famille",
                "destinataire",
                "titulaire_foyer",
                "now() - interval '30 days'",
                None,
            ),
            (
                pere,
                "famille",
                "destinataire",
                "titulaire_foyer",
                du_pere,
                au_pere,
            ),
        ] {
            let motif = au.map(|_| ", motif_sortie").unwrap_or_default();
            let motif_v = au.map(|_| ", 'separation'").unwrap_or_default();
            let (au_c, au_v) = au.map(|a| (", au", format!(", {a}"))).unwrap_or_default();
            let sql = format!(
                "INSERT INTO ent_msg_participant (id, etablissement_id, fil_id, personne_id, cote, qualite, raison, du{au_c}{motif})
                 VALUES ($1,$2,$3,$4,$5,$6,$7,{du}{au_v}{motif_v})"
            );
            sqlx::query(&sql)
                .bind(Uuid::new_v4())
                .bind(etab)
                .bind(fil)
                .bind(pers)
                .bind(cote)
                .bind(qualite)
                .bind(raison)
                .execute(p)
                .await
                .expect("participant");
        }
        for (quand, corps) in messages {
            sqlx::query(&format!(
                "INSERT INTO ent_msg (id, etablissement_id, fil_id, auteur_user_id, corps, created_at)
                 VALUES ($1,$2,$3,$4,$5, now() + interval '{quand}')"
            ))
            .bind(Uuid::new_v4()).bind(etab).bind(fil).bind(cast.cpe.user_id).bind(corps)
            .execute(p).await.expect("message");
        }
    }
    let d = demande(
        &ent,
        &cast,
        "titulaire_autorite",
        Some(cast.parent_lea.user_id),
    )
    .await;
    let e = ent
        .droits_extrait_assembler(etab, dir, DIR, d.id, None)
        .await
        .unwrap();
    let brut = ent
        .droits_extrait_lire(etab, dir, DIR, d.id, e[0].id)
        .await
        .unwrap();
    let t = format!("{:?}", brut.sections);
    assert!(
        t.contains("AVANT-DEPART") && t.contains("APRES-ENTREE"),
        "la période du père figure"
    );
    assert!(
        !t.contains("APRES-DEPART"),
        "message postérieur au départ du père"
    );
    assert!(
        !t.contains("AVANT-ENTREE"),
        "historique antérieur à l'entrée du père"
    );
}

/// E3 : une demande « pour soi » n'accepte pas de sujet supplémentaire (aucune qualité n'y serait constatée) ;
/// un titulaire peut ajouter la fratrie.
#[tokio::test]
async fn g3c_e3_fratrie_refusee_pour_une_demande_pour_soi() {
    let Some((ent, cast)) = labo().await else {
        return;
    };
    let (etab, dir) = (cast.etablissement_id, cast.direction.user_id);
    let pour_soi = demande(&ent, &cast, "eleve_mineur", None).await;
    let r = ent
        .droits_sujet_ajouter(etab, dir, DIR, pour_soi.id, "eleve", cast.eleve_tom_id)
        .await
        .unwrap_err();
    assert!(est_conflit(&r), "{r:?}");
    let titulaire = demande(
        &ent,
        &cast,
        "titulaire_autorite",
        Some(cast.parent_lea.user_id),
    )
    .await;
    let ok = ent
        .droits_sujet_ajouter(etab, dir, DIR, titulaire.id, "eleve", cast.eleve_tom_id)
        .await;
    // Le titulaire n'a pas de lien avec Tom : la qualité constatée sera à arbitrer, mais l'ajout est tracé.
    assert!(ok.is_ok(), "{ok:?}");
}

/// M1 : un masquage ajouté à la main survit au réassemblage et bloque encore l'élément.
#[tokio::test]
async fn g3c_m1_masquage_manuel_reporte_au_reassemblage() {
    let Some((ent, cast)) = labo().await else {
        return;
    };
    let (etab, dir) = (cast.etablissement_id, cast.direction.user_id);
    parents_separes(&ent, &cast, false).await;
    let d = demande(
        &ent,
        &cast,
        "titulaire_autorite",
        Some(cast.parent_lea.user_id),
    )
    .await;
    let e = ent
        .droits_extrait_assembler(etab, dir, DIR, d.id, None)
        .await
        .unwrap();
    let c = ent
        .droits_extrait_lire(etab, dir, DIR, d.id, e[0].id)
        .await
        .unwrap();
    let sec = c
        .sections
        .iter()
        .find(|s| !s.tables.is_empty())
        .expect("une section");
    let (f, t) = (sec.fournisseur.clone(), sec.tables[0].nom.clone());
    ent.droits_masquage_ajouter(
        etab,
        dir,
        DIR,
        d.id,
        e[0].id,
        &f,
        &t,
        None,
        None,
        None,
        "précision de la famille",
    )
    .await
    .unwrap();
    let e2 = ent
        .droits_extrait_assembler(etab, dir, DIR, d.id, None)
        .await
        .unwrap();
    let marques = ent
        .droits_masquages(etab, dir, DIR, d.id, e2[0].id)
        .await
        .unwrap();
    assert!(
        marques
            .iter()
            .any(|m| m.raison == "manuel" && m.table == t && m.decision == "masque"),
        "le masquage manuel est recopié sur la nouvelle version"
    );
}

/// M3 : une protection posée **après** l'assemblage périme l'extrait : ni lecture, ni remise, tant qu'on n'a pas
/// réassemblé (et relu les marquages recalculés).
#[tokio::test]
async fn g3c_m3_protection_posee_apres_l_assemblage_perime_l_extrait() {
    let Some((ent, cast)) = labo().await else {
        return;
    };
    let (etab, dir) = (cast.etablissement_id, cast.direction.user_id);
    let autre = parents_separes(&ent, &cast, false).await;
    let d = demande(
        &ent,
        &cast,
        "titulaire_autorite",
        Some(cast.parent_lea.user_id),
    )
    .await;
    let extrait = extrait_pret(&ent, &cast, d.id).await;
    let _cle = enroler(&ent, etab, dir).await;
    // Avant : l'extrait est remissible.
    ent.droits_remise_defi_a(etab, dir, DIR, d.id, extrait, t0())
        .await
        .expect("remissible");
    sqlx::query(
        "INSERT INTO ent_adresse_protegee (id, etablissement_id, personne_id, adresse, origine)
         VALUES ($1, $2, $3, 'enc:v1:fictif', 'saisie')",
    )
    .bind(Uuid::new_v4())
    .bind(etab)
    .bind(autre)
    .execute(ent.pool_verif())
    .await
    .expect("protection posée après coup");
    let r = ent
        .droits_remise_defi_a(etab, dir, DIR, d.id, extrait, t0())
        .await
        .unwrap_err();
    assert!(est_conflit(&r), "remise refusée : {r:?}");
    let r = ent
        .droits_extrait_lire(etab, dir, DIR, d.id, extrait)
        .await
        .unwrap_err();
    assert!(est_conflit(&r), "lecture refusée : {r:?}");
    let vues = ent.droits_extraits(etab, dir, DIR, d.id).await.unwrap();
    assert!(vues.iter().find(|v| v.id == extrait).unwrap().perime || vues.len() > 1);
    // Réassembler recalcule les marquages, dont la protection.
    let e2 = ent
        .droits_extrait_assembler(etab, dir, DIR, d.id, None)
        .await
        .unwrap();
    let c = ent
        .droits_extrait_lire(etab, dir, DIR, d.id, e2[0].id)
        .await
        .unwrap();
    assert!(c.marquages.iter().any(|m| m.raison == "adresse_protegee"));
}
