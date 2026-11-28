//! Socle 21, lot V14 « cohabitation de l'an 1 » : tests sur base (feature `labo`). Sans `ENT_DATABASE_URL`, ignorés.
//!
//! Monde de `tests_evaluations` : une classe de trois élèves (Hannah, INE 111111111AA ; Susan, INE 222222222BB ; Michael,
//! sans INE). Hannah a deux responsables : `parent_lea` (voit les notes) et `parent_tom` (privé de l'accès aux notes).
//! Aucune horloge réelle : le jour de référence est tiré des bornes de la période du monde.

use super::*;
use crate::outbox::ConfigDistribution;
use crate::tests_evaluations::{labo, monde, Monde};
use chrono::{Duration, NaiveDate};
use ent_types::evaluations_v14::*;
use uuid::Uuid;

const DIR: &str = "direction";
const ADM: &str = "administratif";

fn pdf(tag: &str) -> Vec<u8> {
    format!("%PDF-1.4\n% bulletin {tag}\n1 0 obj\n<< /Type /Catalog >>\nendobj\ntrailer\n<< /Root 1 0 R >>\n%%EOF\n")
        .into_bytes()
}

fn depot(nom: &str) -> DeposerBulletinTiersBody {
    DeposerBulletinTiersBody {
        nom_fichier: nom.into(),
        contenu_base64: String::new(),
        ine: None,
        nom: None,
        prenom: None,
        date_naissance: None,
    }
}

/// Bornes de la période du monde (`ent_periode`).
async fn bornes(ent: &PgEnt, m: &Monde) -> (NaiveDate, NaiveDate) {
    sqlx::query_as(
        "SELECT date_debut, date_fin FROM ent_periode WHERE etablissement_id = $1 AND id = $2",
    )
    .bind(m.etab)
    .bind(m.periode_eva)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap()
}

async fn jour(ent: &PgEnt, m: &Monde) -> NaiveDate {
    bornes(ent, m).await.0 + Duration::days(5)
}

async fn lot(ent: &PgEnt, cast: &LaboCast, m: &Monde) -> Uuid {
    let j = jour(ent, m).await;
    ent.eva_v14_ouvrir_lot_for_actor(
        m.etab,
        cast.administratif.user_id,
        ADM,
        j,
        &OuvrirLotBody {
            periode_id: m.periode_eva,
            logiciel: None,
            libelle: None,
        },
    )
    .await
    .unwrap()
    .id
}

async fn deposer(
    ent: &PgEnt,
    cast: &LaboCast,
    m: &Monde,
    lot: Uuid,
    b: DeposerBulletinTiersBody,
    tag: &str,
) -> BulletinTiersDto {
    ent.eva_v14_deposer_for_actor(m.etab, cast.administratif.user_id, ADM, lot, &b, &pdf(tag))
        .await
        .unwrap()
}

async fn distribuer_outbox(ent: &PgEnt, etab: Uuid) {
    ent.distributeur(ConfigDistribution {
        taille_lot: 200,
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

async fn notifs(ent: &PgEnt, etab: Uuid, user: Uuid) -> usize {
    ent.mes_notifications(etab, user, 200)
        .await
        .unwrap()
        .into_iter()
        .filter(|n| n.kind == "bulletin_publie")
        .count()
}

/// EVA-234, EVA-235, CE-89 : rapprochement par INE (lu dans le nom du fichier) et par identité ; un fichier non rapproché
/// n'est jamais distribué ; le parent privé des notes n'est pas destinataire et ne lit rien ; un rapprochement manuel
/// fait partir le dernier.
#[tokio::test]
async fn depot_rapprochement_puis_distribution_selon_les_droits_des_liens() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let m = monde(&ent, &cast).await;
    let (etab, dir) = (m.etab, cast.direction.user_id);
    let (lea, tom) = (cast.parent_lea.user_id, cast.parent_tom.user_id);
    let (hannah, susan, michael) = (m.eleves[0], m.eleves[1], m.eleves[2]);
    // Date de naissance posée directement (fixture : la garde d'identité du socle 2 n'est pas l'objet du test).
    let mut tx = ent.pool_verif().begin().await.unwrap();
    sqlx::query("SET LOCAL session_replication_role = replica")
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::query("UPDATE ent_eleve SET date_naissance = '2010-04-12' WHERE etablissement_id = $1 AND id = $2")
        .bind(etab)
        .bind(susan)
        .execute(&mut *tx)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    let j = jour(&ent, &m).await;
    let l = lot(&ent, &cast, &m).await;

    let b_hannah = deposer(
        &ent,
        &cast,
        &m,
        l,
        depot("2NDE_111111111aa_ABBOTT.pdf"),
        "h1",
    )
    .await;
    assert_eq!(b_hannah.statut, "rapproche");
    assert_eq!(b_hannah.eleve_id, Some(hannah));
    assert_eq!(b_hannah.rapprochement.as_deref(), Some("ine"));
    let mut d = depot("bones.pdf");
    d.nom = Some("BONES".into());
    d.prenom = Some("susan".into());
    d.date_naissance = NaiveDate::from_ymd_opt(2010, 4, 12);
    let b_susan = deposer(&ent, &cast, &m, l, d, "s1").await;
    assert_eq!(
        (b_susan.eleve_id, b_susan.rapprochement.as_deref()),
        (Some(susan), Some("identite"))
    );
    // Même identité mais un INE annoncé qui contredit celui de l'élève : jamais d'appariement.
    let mut d = depot("bones-bis.pdf");
    d.ine = Some("999999999ZZ".into());
    d.nom = Some("Bones".into());
    d.prenom = Some("Susan".into());
    d.date_naissance = NaiveDate::from_ymd_opt(2010, 4, 12);
    let b_contredit = deposer(&ent, &cast, &m, l, d, "s-contredit").await;
    assert_eq!(b_contredit.statut, "a_rapprocher");
    assert_eq!(b_contredit.eleve_id, None);
    let b_michael = deposer(&ent, &cast, &m, l, depot("corner.pdf"), "m1").await;
    assert_eq!(b_michael.statut, "a_rapprocher");

    // Contrôle : rien n'est écrit.
    let c = ent
        .eva_v14_controler_for_actor(etab, dir, DIR, j, l)
        .await
        .unwrap();
    assert!(c.controle);
    assert_eq!((c.distribues, c.a_rapprocher), (2, 2));
    assert_eq!(
        c.sans_destinataire.len(),
        1,
        "Susan n'a aucun destinataire en ligne : lot d'impression"
    );
    assert_eq!(c.sans_destinataire[0].eleve_id, susan);
    let (_, fichiers) = ent.eva_v14_lot_for_actor(etab, dir, DIR, l).await.unwrap();
    assert!(fichiers.iter().all(|f| f.statut != "distribue"));

    let r = ent
        .eva_v14_distribuer_for_actor(etab, dir, DIR, j, l)
        .await
        .unwrap();
    assert!(!r.controle);
    assert_eq!((r.distribues, r.a_rapprocher, r.remplacements), (2, 2, 0));
    assert_eq!(r.destinataires, 1, "seule Léa reçoit le bulletin de Hannah");

    // Léa lit ; Tom (privé des notes) ne voit rien et ne télécharge rien.
    let liste = ent
        .eva_v14_bulletins_eleve_for_actor(etab, lea, "parent", hannah)
        .await
        .unwrap();
    assert_eq!(liste.len(), 1);
    let (_, octets) = ent
        .eva_v14_pdf_for_actor(etab, lea, "parent", liste[0].id)
        .await
        .unwrap();
    assert_eq!(octets, pdf("h1"));
    let (_, octets) = ent
        .telecharger_document_bytes_for_actor(etab, lea, "parent", liste[0].document_id)
        .await
        .unwrap();
    assert_eq!(octets, pdf("h1"), "le même PDF est au coffre de l'élève");
    assert!(ent
        .eva_v14_bulletins_eleve_for_actor(etab, tom, "parent", hannah)
        .await
        .unwrap()
        .is_empty());
    assert!(matches!(
        ent.eva_v14_pdf_for_actor(etab, tom, "parent", liste[0].id)
            .await,
        Err(EntDbError::Domain(domain_eleve::DomainError::NotFound))
    ));
    assert!(ent
        .telecharger_document_bytes_for_actor(etab, tom, "parent", liste[0].document_id)
        .await
        .is_err());
    // Un bulletin non rapproché n'a jamais de document ni de destinataire, et la famille ne peut pas le lire.
    let (_, fichiers) = ent.eva_v14_lot_for_actor(etab, dir, DIR, l).await.unwrap();
    let m1 = fichiers.iter().find(|f| f.id == b_michael.id).unwrap();
    assert_eq!((m1.statut.as_str(), m1.destinataires), ("a_rapprocher", 0));
    assert!(ent
        .eva_v14_pdf_for_actor(etab, lea, "parent", b_michael.id)
        .await
        .is_err());
    let (dests,): (i64,) =
        sqlx::query_as("SELECT count(*) FROM eva_tiers_destinataire WHERE bulletin_id = $1")
            .bind(b_hannah.id)
            .fetch_one(ent.pool_verif())
            .await
            .unwrap();
    assert_eq!(dests, 1);

    // Rapprochement manuel (après aperçu) puis nouvelle distribution : seul Michael part.
    let (_, apercu) = ent
        .eva_v14_apercu_for_actor(etab, cast.administratif.user_id, ADM, b_michael.id)
        .await
        .unwrap();
    assert_eq!(apercu, pdf("m1"));
    let rb = ent
        .eva_v14_rapprocher_for_actor(
            etab,
            cast.administratif.user_id,
            ADM,
            b_michael.id,
            &RapprocherBulletinBody { eleve_id: michael },
        )
        .await
        .unwrap();
    assert_eq!(rb.rapprochement.as_deref(), Some("manuel"));
    let r = ent
        .eva_v14_distribuer_for_actor(etab, dir, DIR, j, l)
        .await
        .unwrap();
    assert_eq!((r.distribues, r.a_rapprocher), (1, 1));
    // Le fichier contredit est écarté avec un motif ; il ne sera jamais distribué.
    let e = ent
        .eva_v14_ecarter_for_actor(
            etab,
            cast.administratif.user_id,
            ADM,
            b_contredit.id,
            &MotifBulletinTiersBody {
                motif: "INE erroné, doublon".into(),
            },
        )
        .await
        .unwrap();
    assert_eq!(e.statut, "ecarte");
    let r = ent
        .eva_v14_distribuer_for_actor(etab, dir, DIR, j, l)
        .await
        .unwrap();
    assert_eq!((r.distribues, r.a_rapprocher), (0, 0));
    // Journal en ajout seul.
    let err = sqlx::query("DELETE FROM eva_tiers_journal WHERE etablissement_id = $1")
        .bind(etab)
        .execute(ent.pool_verif())
        .await
        .unwrap_err();
    assert!(err.to_string().contains("ajout seul"), "{err}");
}

/// Élève (majeur ou non) destinataire de son bulletin ; avis aux destinataires figés seulement.
#[tokio::test]
async fn eleve_destinataire_et_avis_aux_seuls_destinataires() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let m = monde(&ent, &cast).await;
    let (etab, dir) = (m.etab, cast.direction.user_id);
    let (lea, tom) = (cast.parent_lea.user_id, cast.parent_tom.user_id);
    let hannah = m.eleves[0];
    let compte = ent
        .register_login(
            &format!("v14-{}@ent.lab", Uuid::new_v4().simple()),
            ent_auth::Role::Eleve,
            "Hannah",
            etab,
        )
        .await
        .unwrap();
    ent.link_eleve_compte(etab, hannah, compte.user_id)
        .await
        .unwrap();
    let j = jour(&ent, &m).await;
    let l = lot(&ent, &cast, &m).await;
    let mut d = depot("hannah.pdf");
    d.ine = Some("111111111aa".into());
    deposer(&ent, &cast, &m, l, d, "h").await;
    let avant = (notifs(&ent, etab, lea).await, notifs(&ent, etab, tom).await);
    let r = ent
        .eva_v14_distribuer_for_actor(etab, dir, DIR, j, l)
        .await
        .unwrap();
    assert_eq!(r.destinataires, 2, "Léa et le compte de Hannah");
    distribuer_outbox(&ent, etab).await;
    assert_eq!(notifs(&ent, etab, lea).await, avant.0 + 1);
    assert_eq!(
        notifs(&ent, etab, tom).await,
        avant.1,
        "Tom, privé des notes, n'est pas prévenu"
    );
    assert_eq!(notifs(&ent, etab, compte.user_id).await, 1);
    let liste = ent
        .eva_v14_bulletins_eleve_for_actor(etab, compte.user_id, "eleve", hannah)
        .await
        .unwrap();
    assert_eq!(liste.len(), 1);
    ent.eva_v14_pdf_for_actor(etab, compte.user_id, "eleve", liste[0].id)
        .await
        .unwrap();
    // Un autre élève ne lit pas ce bulletin.
    assert!(ent
        .eva_v14_bulletins_eleve_for_actor(etab, compte.user_id, "eleve", m.eleves[1])
        .await
        .unwrap()
        .is_empty());
}

/// Une nouvelle version remplace la précédente ; un retrait la sort du coffre des familles.
#[tokio::test]
async fn nouvelle_version_remplace_puis_retrait_motive() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let m = monde(&ent, &cast).await;
    let (etab, dir, lea) = (m.etab, cast.direction.user_id, cast.parent_lea.user_id);
    let hannah = m.eleves[0];
    let j = jour(&ent, &m).await;
    let l1 = lot(&ent, &cast, &m).await;
    let v1 = deposer(&ent, &cast, &m, l1, depot("111111111AA.pdf"), "v1").await;
    ent.eva_v14_distribuer_for_actor(etab, dir, DIR, j, l1)
        .await
        .unwrap();
    // Le même fichier ne se redépose pas pour la période.
    let doublon = ent
        .eva_v14_deposer_for_actor(
            etab,
            cast.administratif.user_id,
            ADM,
            l1,
            &depot("111111111AA.pdf"),
            &pdf("v1"),
        )
        .await;
    assert!(
        matches!(doublon, Err(EntDbError::ConflitDossier(_))),
        "{doublon:?}"
    );
    let l2 = lot(&ent, &cast, &m).await;
    let v2 = deposer(&ent, &cast, &m, l2, depot("111111111AA.pdf"), "v2").await;
    assert_eq!(v2.statut, "rapproche");
    let r = ent
        .eva_v14_distribuer_for_actor(etab, dir, DIR, j, l2)
        .await
        .unwrap();
    assert_eq!((r.distribues, r.remplacements), (1, 1));
    let liste = ent
        .eva_v14_bulletins_eleve_for_actor(etab, lea, "parent", hannah)
        .await
        .unwrap();
    assert_eq!(liste.len(), 1);
    assert_eq!(liste[0].id, v2.id);
    let (statut_v1, doc_v1): (String, Uuid) =
        sqlx::query_as("SELECT statut, document_id FROM eva_tiers_bulletin WHERE id = $1")
            .bind(v1.id)
            .fetch_one(ent.pool_verif())
            .await
            .unwrap();
    assert_eq!(statut_v1, "remplace");
    assert!(ent
        .telecharger_document_bytes_for_actor(etab, lea, "parent", doc_v1)
        .await
        .is_err());
    assert!(ent
        .eva_v14_pdf_for_actor(etab, lea, "parent", v1.id)
        .await
        .is_err());
    // Un bulletin distribué ne s'écarte pas, il se retire (motif obligatoire, direction).
    assert!(ent
        .eva_v14_ecarter_for_actor(
            etab,
            cast.administratif.user_id,
            ADM,
            v2.id,
            &MotifBulletinTiersBody {
                motif: "erreur".into()
            }
        )
        .await
        .is_err());
    assert!(matches!(
        ent.eva_v14_retirer_for_actor(
            etab,
            cast.administratif.user_id,
            ADM,
            v2.id,
            &MotifBulletinTiersBody {
                motif: "mauvais élève".into()
            }
        )
        .await,
        Err(EntDbError::Forbidden)
    ));
    assert!(ent
        .eva_v14_retirer_for_actor(
            etab,
            dir,
            DIR,
            v2.id,
            &MotifBulletinTiersBody { motif: "x".into() }
        )
        .await
        .is_err());
    let r = ent
        .eva_v14_retirer_for_actor(
            etab,
            dir,
            DIR,
            v2.id,
            &MotifBulletinTiersBody {
                motif: "bulletin d'un autre élève".into(),
            },
        )
        .await
        .unwrap();
    assert_eq!(r.statut, "retire");
    assert!(ent
        .eva_v14_bulletins_eleve_for_actor(etab, lea, "parent", hannah)
        .await
        .unwrap()
        .is_empty());
    assert!(ent
        .telecharger_document_bytes_for_actor(etab, lea, "parent", liste[0].document_id)
        .await
        .is_err());
    // La base refuse de faire revenir un bulletin retiré.
    let err = sqlx::query("UPDATE eva_tiers_bulletin SET statut = 'distribue' WHERE id = $1")
        .bind(v2.id)
        .execute(ent.pool_verif())
        .await
        .unwrap_err();
    assert!(err.to_string().contains("ne change plus"), "{err}");
}

/// Droits et refus : personnel seulement ; le secrétariat dépose et rapproche, la direction distribue ; PDF seulement ;
/// deux bulletins en attente pour le même élève refusés.
#[tokio::test]
async fn droits_et_refus_du_depot() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let m = monde(&ent, &cast).await;
    let (etab, adm, lea, prof) = (
        m.etab,
        cast.administratif.user_id,
        cast.parent_lea.user_id,
        cast.prof.user_id,
    );
    let j = jour(&ent, &m).await;
    let corps = OuvrirLotBody {
        periode_id: m.periode_eva,
        logiciel: None,
        libelle: None,
    };
    for (u, r) in [(lea, "parent"), (prof, "prof"), (cast.cpe.user_id, "cpe")] {
        assert!(
            matches!(
                ent.eva_v14_ouvrir_lot_for_actor(etab, u, r, j, &corps)
                    .await,
                Err(EntDbError::Forbidden)
            ),
            "{r} ne dépose pas"
        );
    }
    // Avant le début de la période : refus.
    let (debut, _) = bornes(&ent, &m).await;
    assert!(ent
        .eva_v14_ouvrir_lot_for_actor(etab, adm, ADM, debut - Duration::days(1), &corps)
        .await
        .is_err());
    let l = lot(&ent, &cast, &m).await;
    assert!(matches!(
        ent.eva_v14_lot_for_actor(etab, lea, "parent", l).await,
        Err(EntDbError::Forbidden)
    ));
    assert!(matches!(
        ent.eva_v14_distribuer_for_actor(etab, adm, ADM, j, l).await,
        Err(EntDbError::Forbidden)
    ));
    assert!(matches!(
        ent.eva_v14_controler_for_actor(etab, prof, "prof", j, l)
            .await,
        Err(EntDbError::Forbidden)
    ));
    // Un fichier qui n'est pas un PDF est refusé.
    let png = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR\0\0\0\x01\0\0\0\x01\x08\x06\0\0\0\x1f\x15\xc4\x89\0\0\0\rIDATx\x9cc\0\x01\0\0\x05\0\x01\r\n-\xb4\0\0\0\0IEND\xaeB`\x82";
    assert!(ent
        .eva_v14_deposer_for_actor(etab, adm, ADM, l, &depot("photo.pdf"), png)
        .await
        .is_err());
    // Deux bulletins de Hannah en attente pour la période : le second reste à rapprocher, et ne peut pas être rapproché
    // à la main tant que le premier attend.
    let a = deposer(&ent, &cast, &m, l, depot("111111111AA-a.pdf"), "a").await;
    assert_eq!(a.statut, "rapproche");
    let b = deposer(&ent, &cast, &m, l, depot("111111111AA-b.pdf"), "b").await;
    assert_eq!(b.statut, "a_rapprocher");
    let r = ent
        .eva_v14_rapprocher_for_actor(
            etab,
            adm,
            ADM,
            b.id,
            &RapprocherBulletinBody {
                eleve_id: m.eleves[0],
            },
        )
        .await;
    assert!(matches!(r, Err(EntDbError::ConflitDossier(_))), "{r:?}");
    // Un élève d'un autre établissement ou inconnu : refus.
    let r = ent
        .eva_v14_rapprocher_for_actor(
            etab,
            adm,
            ADM,
            b.id,
            &RapprocherBulletinBody {
                eleve_id: Uuid::new_v4(),
            },
        )
        .await;
    assert!(r.is_err());
    // Une famille ne lit pas l'aperçu.
    assert!(matches!(
        ent.eva_v14_apercu_for_actor(etab, lea, "parent", a.id)
            .await,
        Err(EntDbError::Forbidden)
    ));
}

/// EVA-237 : bandeau réglé par la direction, lien en https seulement, lisible de tous.
#[tokio::test]
async fn bandeau_ou_sont_les_notes() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let m = monde(&ent, &cast).await;
    let (etab, dir) = (m.etab, cast.direction.user_id);
    let mut b = ReglerCohabitationBody {
        actif: true,
        logiciel_libelle: "Charlemagne".into(),
        url: Some("javascript:alert(1)".into()),
        message: "Cette année, les notes se consultent sur le portail de l'éditeur.".into(),
    };
    assert!(ent
        .eva_v14_regler_cohabitation_for_actor(etab, dir, DIR, &b)
        .await
        .is_err());
    b.url = Some("http://exemple.test".into());
    assert!(ent
        .eva_v14_regler_cohabitation_for_actor(etab, dir, DIR, &b)
        .await
        .is_err());
    b.url = Some("https://portail.exemple.test/notes".into());
    assert!(matches!(
        ent.eva_v14_regler_cohabitation_for_actor(etab, cast.parent_lea.user_id, "parent", &b)
            .await,
        Err(EntDbError::Forbidden)
    ));
    assert!(matches!(
        ent.eva_v14_regler_cohabitation_for_actor(etab, cast.administratif.user_id, ADM, &b)
            .await,
        Err(EntDbError::Forbidden)
    ));
    ent.eva_v14_regler_cohabitation_for_actor(etab, dir, DIR, &b)
        .await
        .unwrap();
    let lu = ent.eva_v14_cohabitation(etab).await.unwrap().unwrap();
    assert!(lu.actif);
    assert_eq!(
        lu.url.as_deref(),
        Some("https://portail.exemple.test/notes")
    );
    b.actif = false;
    b.url = None;
    ent.eva_v14_regler_cohabitation_for_actor(etab, dir, DIR, &b)
        .await
        .unwrap();
    let lu = ent.eva_v14_cohabitation(etab).await.unwrap().unwrap();
    assert!(!lu.actif && lu.url.is_none());
}

/// EVA-233 : export de l'assiduité, borné à la période et à aujourd'hui, journalisé ; famille et professeur refusés.
#[tokio::test]
async fn export_assiduite_borne_et_journalise() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let m = monde(&ent, &cast).await;
    let (etab, adm) = (m.etab, cast.administratif.user_id);
    let (debut, fin) = bornes(&ent, &m).await;
    // Fin de la période (les affectations du monde commencent le jour de sa création, dans la période).
    let j = fin;
    for (u, r) in [
        (cast.parent_lea.user_id, "parent"),
        (cast.prof.user_id, "prof"),
    ] {
        assert!(matches!(
            ent.eva_v14_exporter_assiduite_for_actor(etab, u, r, j, m.periode_eva, m.classe)
                .await,
            Err(EntDbError::Forbidden)
        ));
    }
    let (e, classe) = ent
        .eva_v14_exporter_assiduite_for_actor(etab, adm, ADM, j, m.periode_eva, m.classe)
        .await
        .unwrap();
    assert_eq!((e.du, e.au, e.partiel), (debut, fin, false));
    // Un jour avant la fin : bornes coupées à ce jour, export partiel.
    let (p, _) = ent
        .eva_v14_exporter_assiduite_for_actor(
            etab,
            adm,
            ADM,
            fin - Duration::days(1),
            m.periode_eva,
            m.classe,
        )
        .await
        .unwrap();
    assert_eq!((p.au, p.partiel), (fin - Duration::days(1), true));
    assert_eq!(e.lignes.len(), 3);
    let hannah = e.lignes.iter().find(|l| l.eleve_id == m.eleves[0]).unwrap();
    assert_eq!(hannah.ine.as_deref(), Some("111111111AA"));
    assert_eq!(
        hannah.justifiees + hannah.non_justifiees,
        hannah.demi_journees
    );
    let csv = crate::evaluations_v14::csv_assiduite_tiers(&e, &classe);
    assert_eq!(csv.lines().count(), 4, "en-tête et trois élèves");
    let (n,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM eva_tiers_export WHERE etablissement_id = $1 AND id = $2",
    )
    .bind(etab)
    .bind(e.export_id)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(n, 1, "export journalisé");
    // Avant le début de la période : rien à exporter.
    assert!(ent
        .eva_v14_exporter_assiduite_for_actor(
            etab,
            adm,
            ADM,
            debut - Duration::days(1),
            m.periode_eva,
            m.classe
        )
        .await
        .is_err());
}
