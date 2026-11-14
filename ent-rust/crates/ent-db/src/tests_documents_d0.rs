//! Socle 7 (documents), lot D0 « correctifs immédiats » : téléchargement d'un `.docx` tel quel,
//! espace de clés `bulletins`, garde de signature concurrente. Aucune migration.
//!
//! Chaque test ne vérifie que ses propres lignes (identifiants et codes aléatoires) ; la partie
//! pure tourne sans base, le reste est ignoré sans `ENT_DATABASE_URL`.

use super::*;
use crate::doc_signer_repo::PgDocSignerRepo;
use crate::service::document_storage_key_is_owned;
use crate::tests_evaluations::{devoir, labo, monde, r, statut};
use domain_eleve::{
    DeposerDocument, DocConfidentialite, DocSignerStatut, DocTiroir, DomainError, EleveDocument,
    PublierDocASigner, SignerDocASigner,
};
use ent_types::evaluations::*;
use ent_auth::Role;
use uuid::Uuid;

/// ZIP minimal (méthode « stockée ») : assez pour être reconnu comme `.docx` par le dépôt.
fn docx_minimal() -> Vec<u8> {
    let entrees: [(&str, &[u8]); 2] = [("[Content_Types].xml", b"<x/>"), ("word/document.xml", b"<d/>")];
    let (mut out, mut cd) = (Vec::new(), Vec::new());
    for (nom, data) in entrees {
        let off = out.len() as u32;
        out.extend_from_slice(b"PK\x03\x04");
        out.extend_from_slice(&[20, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        out.extend_from_slice(&0u32.to_le_bytes());
        out.extend_from_slice(&(data.len() as u32).to_le_bytes());
        out.extend_from_slice(&(data.len() as u32).to_le_bytes());
        out.extend_from_slice(&(nom.len() as u16).to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(nom.as_bytes());
        out.extend_from_slice(data);
        cd.extend_from_slice(b"PK\x01\x02");
        cd.extend_from_slice(&[20, 0, 20, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        cd.extend_from_slice(&0u32.to_le_bytes());
        cd.extend_from_slice(&(data.len() as u32).to_le_bytes());
        cd.extend_from_slice(&(data.len() as u32).to_le_bytes());
        cd.extend_from_slice(&(nom.len() as u16).to_le_bytes());
        cd.extend_from_slice(&[0; 12]);
        cd.extend_from_slice(&off.to_le_bytes());
        cd.extend_from_slice(nom.as_bytes());
    }
    let cd_off = out.len() as u32;
    out.extend_from_slice(&cd);
    out.extend_from_slice(b"PK\x05\x06");
    out.extend_from_slice(&[0, 0, 0, 0]);
    out.extend_from_slice(&2u16.to_le_bytes());
    out.extend_from_slice(&2u16.to_le_bytes());
    out.extend_from_slice(&(cd.len() as u32).to_le_bytes());
    out.extend_from_slice(&cd_off.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out
}

fn est_stub(b: &[u8]) -> bool {
    b.starts_with("Document ENT (lab)".as_bytes())
}

fn doc_vide(etab: Uuid, eleve: Uuid) -> EleveDocument {
    EleveDocument {
        id: Uuid::new_v4(),
        etablissement_id: etab,
        eleve_id: eleve,
        tiroir: DocTiroir::Famille,
        titre: "x".into(),
        code: None,
        mime_type: None,
        storage_key: None,
        confidentialite: DocConfidentialite::Standard,
        source: "upload".into(),
        created_by_user_id: None,
    }
}

/// D0-1 et D0-2 (partie pure) : toutes les extensions de `TypeFichier` et l'espace `bulletins`.
#[test]
fn d0_cles_de_stockage_reconnues() {
    use ent_object::upload::TypeFichier as T;
    let d = doc_vide(Uuid::new_v4(), Uuid::new_v4());
    let (e, autre) = (d.etablissement_id, Uuid::new_v4());
    for t in [T::Pdf, T::Png, T::Jpeg, T::Gif, T::Webp, T::Texte, T::Docx, T::Xlsx, T::Pptx, T::Odt, T::Ods, T::Odp] {
        assert!(
            document_storage_key_is_owned(&d, &format!("{e}/documents/{}.{}", d.id, t.extension())),
            "{}",
            t.extension()
        );
    }
    assert!(!document_storage_key_is_owned(&d, &format!("{e}/documents/{}.exe", d.id)));
    assert!(!document_storage_key_is_owned(&d, &format!("{e}/documents/{}.html", d.id)));
    // Bulletins : `{etab}/bulletins/{uuid}.pdf` de CET établissement seulement.
    let b = Uuid::new_v4();
    assert!(document_storage_key_is_owned(&d, &format!("{e}/bulletins/{b}.pdf")));
    assert!(!document_storage_key_is_owned(&d, &format!("{autre}/bulletins/{b}.pdf")));
    assert!(!document_storage_key_is_owned(&d, &format!("{e}/bulletins/{b}.docx")));
    assert!(!document_storage_key_is_owned(&d, &format!("{e}/bulletins/pas-un-uuid.pdf")));
    assert!(!document_storage_key_is_owned(&d, &format!("{e}/bulletins/../{b}.pdf")));
}

/// D0-1 : un `.docx` déposé se télécharge tel quel (et non le stub texte « Document ENT (lab) »).
#[tokio::test]
async fn d0_docx_se_telecharge_tel_quel() {
    let Some((ent, cast)) = labo().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let (etab, lea) = (cast.etablissement_id, cast.eleve_lea_id);
    let p = Role::Parent.as_str();
    let octets = docx_minimal();
    let doc = ent
        .deposer_document_for_actor(
            p,
            DeposerDocument {
                etablissement_id: etab,
                eleve_id: lea,
                tiroir: DocTiroir::Famille,
                titre: format!("Docx d0 {}", &Uuid::new_v4().simple().to_string()[..16]),
                code: None,
                mime_type: Some(
                    "application/vnd.openxmlformats-officedocument.wordprocessingml.document".into(),
                ),
                storage_key: None,
                confidentialite: DocConfidentialite::Standard,
                actor_user_id: cast.parent_lea.user_id,
            },
            Some(&octets),
        )
        .await
        .expect("dépôt docx");
    assert!(doc.storage_key.as_deref().is_some_and(|k| k.ends_with(".docx")), "{:?}", doc.storage_key);
    let (_, lus) = ent
        .telecharger_document_bytes_for_actor(etab, cast.parent_lea.user_id, p, doc.id)
        .await
        .expect("téléchargement");
    assert!(!est_stub(&lus), "stub renvoyé à la place du .docx");
    assert_eq!(lus, octets);
}

/// D0-2 : un bulletin publié se télécharge depuis le dossier (PDF réel, pas le stub).
#[tokio::test]
async fn d0_bulletin_publie_se_telecharge_depuis_le_dossier() {
    let Some((ent, cast)) = labo().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let m = monde(&ent, &cast).await;
    let (etab, prof, dir, a) = (m.etab, cast.prof.user_id, cast.direction.user_id, m.eleves[0]);
    let d = ent
        .creer_devoir_grille_for_actor(etab, prof, "prof", &devoir(m.s_maths, m.periode, "Contrôle d0", 1.0, 20.0))
        .await
        .unwrap();
    ent.saisir_grille_for_actor(etab, prof, "prof", d.id, &SaisieGrilleBody { resultats: vec![r(a, "note", Some(13.0))] })
        .await
        .unwrap();
    ent.publier_devoir_for_actor(etab, prof, "prof", d.id).await.unwrap();
    statut(&ent, &cast, &m, "fermee").await;
    ent.generer_bulletins_for_actor(
        etab,
        dir,
        "direction",
        &GenererBulletinsBody { classe_id: m.classe, periode_id: m.periode, motif: None },
    )
    .await
    .unwrap();
    ent.publier_bulletins_for_actor(etab, dir, "direction", &PublierBulletinsBody { classe_id: m.classe, periode_id: m.periode })
        .await
        .unwrap();
    let (doc, cle): (Uuid, String) = sqlx::query_as(
        "SELECT id, storage_key FROM ent_eleve_document
          WHERE etablissement_id = $1 AND eleve_id = $2 AND source = 'bulletin' AND confidentialite = 'standard'",
    )
    .bind(etab)
    .bind(a)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    assert!(cle.starts_with(&format!("{etab}/bulletins/")), "{cle}");
    let (_, lus) = ent
        .telecharger_document_bytes_for_actor(etab, dir, "direction", doc)
        .await
        .expect("téléchargement du bulletin");
    assert!(!est_stub(&lus), "stub renvoyé à la place du bulletin");
    assert!(lus.starts_with(b"%PDF"), "le bulletin doit être le PDF généré");
}

async fn doc_a_signer(ent: &PgEnt, cast: &LaboCast) -> domain_eleve::DocASigner {
    ent.publier_doc_a_signer_for_actor(
        "administratif",
        PublierDocASigner {
            etablissement_id: cast.etablissement_id,
            eleve_id: cast.eleve_lea_id,
            code: format!("d0-{}", Uuid::new_v4().simple()),
            titre: "Règlement d0".into(),
            corps: "Texte".into(),
            annee_scolaire: "2026-2027".into(),
            echeance: None,
            actor_user_id: cast.administratif.user_id,
        },
    )
    .await
    .expect("publier")
}

/// D0-3 : deux signatures concurrentes du même document, une seule réussit.
#[tokio::test]
async fn d0_deux_signatures_concurrentes_une_seule_reussit() {
    let Some((ent, cast)) = labo().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let doc = doc_a_signer(&ent, &cast).await;
    let cmd = || SignerDocASigner {
        etablissement_id: cast.etablissement_id,
        doc_id: doc.id,
        actor_user_id: cast.parent_lea.user_id,
    };
    let p = Role::Parent.as_str();
    let (r1, r2) = tokio::join!(
        ent.signer_doc_a_signer_for_actor(p, cmd()),
        ent.signer_doc_a_signer_for_actor(p, cmd()),
    );
    let ok = [&r1, &r2].iter().filter(|r| r.is_ok()).count();
    assert_eq!(ok, 1, "exactement une signature : {r1:?} / {r2:?}");
    let (nb,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM ent_doc_a_signer WHERE etablissement_id = $1 AND id = $2 AND statut = 'signe'",
    )
    .bind(cast.etablissement_id)
    .bind(doc.id)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    assert_eq!(nb, 1);
}

/// D0-3 (déterministe) : la garde est dans la requête, pas seulement dans le plan en mémoire.
/// Une seconde écriture issue d'un instantané périmé (`a_signer`) est refusée et n'écrase rien.
#[tokio::test]
async fn d0_garde_a_signer_dans_la_requete() {
    let Some((ent, cast)) = labo().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let doc = doc_a_signer(&ent, &cast).await;
    let repo = PgDocSignerRepo::new(ent.pool().clone());
    let (premier, second) = (cast.parent_lea.user_id, cast.parent_tom.user_id);
    let signe = |par: Uuid| domain_eleve::DocASigner {
        statut: DocSignerStatut::Signe,
        signe_at: Some(chrono::Utc::now()),
        signe_par_user_id: Some(par),
        ..doc.clone()
    };
    repo.update_signature(&signe(premier)).await.expect("première signature");
    let r = repo.update_signature(&signe(second)).await;
    assert!(matches!(r, Err(DomainError::DocASignerDejaSigne)), "{r:?}");
    let lu = repo.get(cast.etablissement_id, doc.id).await.unwrap().unwrap();
    assert_eq!(lu.signe_par_user_id, Some(premier), "la première signature doit rester");
}
