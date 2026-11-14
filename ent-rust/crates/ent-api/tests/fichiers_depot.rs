//! D2 — contrôle des fichiers déposés, contre le vrai routeur `ent-api` et une base
//! PostgreSQL locale de test (documents, photos, pièces PAI, pièces de messagerie).
//!
//! Lancement : `ENT_ENV=test ENT_DATABASE_URL=… cargo test -p ent-api --features labo
//! --test fichiers_depot -- --test-threads=1`. Sans base : SKIP.

#[path = "separation/harness.rs"]
mod harness;

use axum::http::StatusCode;
use base64::Engine;
use harness::*;
use serde_json::{json, Value};
use std::sync::Once;
use uuid::Uuid;

use Acteur::*;

const PDF: &[u8] = b"%PDF-1.4\n1 0 obj\n<< /Type /Catalog >>\nendobj\ntrailer\n<< >>\n%%EOF\n";
const PNG: &[u8] = &[
    0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 0, 0, 0, 0x0D, b'I', b'H', b'D', b'R',
];
const JPEG: &[u8] = &[0xFF, 0xD8, 0xFF, 0xE0, 0, 16, b'J', b'F', b'I', b'F', 0, 1];
const HTML: &[u8] =
    b"<!DOCTYPE html><html><body><script>alert(document.cookie)</script></body></html>";
const SVG: &[u8] =
    br#"<svg xmlns="http://www.w3.org/2000/svg" onload="alert(1)"><script>alert(1)</script></svg>"#;
const EXE: &[u8] = b"MZ\x90\x00\x03\x00\x00\x00\x04\x00";

fn monde_depot() -> Option<&'static Monde> {
    static INIT: Once = Once::new();
    INIT.call_once(|| {
        std::env::set_var("ENT_OBJECT_MEMORY", "1");
        std::env::set_var(
            "ENT_OBJECT_SIGNING_KEY",
            "cle-de-signature-de-test-0123456789-abcdefghij",
        );
        // Plafond de pièce de messagerie abaissé pour tester le dépassement sans gros corps.
        std::env::set_var("ENT_UPLOAD_MAX_MESSAGE", "2k");
    });
    monde()
}

fn b64(octets: &[u8]) -> String {
    base64::engine::general_purpose::STANDARD.encode(octets)
}

fn rt_run<F: std::future::Future>(f: F) -> F::Output {
    rt().block_on(f)
}

async fn depot_document(m: &Monde, titre: &str, mime: &str, contenu: &[u8]) -> Rep {
    m.appel(
        "POST",
        &format!("/eleves/{}/documents", m.a.objets.eleve),
        Some(m.jeton(Direction)),
        Some(&json!({
            "titre": titre, "tiroir": "famille", "confidentialite": "standard",
            "mime_type": mime, "contenu_base64": b64(contenu),
        })),
    )
    .await
}

async fn depot_photo(m: &Monde, mime: &str, contenu: &[u8]) -> Rep {
    m.appel(
        "POST",
        &format!("/eleves/{}/photo", m.a.objets.eleve),
        Some(m.jeton(Direction)),
        Some(&json!({"mime_type": mime, "contenu_base64": b64(contenu)})),
    )
    .await
}

/// Message du fil de l'élève, auteur : la direction.
async fn message(m: &Monde) -> Uuid {
    let fil = m
        .appel(
            "POST",
            "/messages/fils",
            Some(m.jeton(Direction)),
            Some(&json!({"eleve_id": m.a.objets.eleve, "sujet": "D2", "message": "pièces"})),
        )
        .await;
    assert_eq!(fil.statut, StatusCode::CREATED, "{}", fil.texte);
    Uuid::parse_str(fil.json["messages"][0]["id"].as_str().unwrap()).unwrap()
}

async fn depot_piece(m: &Monde, msg: Uuid, nom: &str, mime: &str, contenu: &[u8]) -> Rep {
    m.appel(
        "POST",
        &format!("/messages/{msg}/pieces"),
        Some(m.jeton(Direction)),
        Some(&json!({"nom": nom, "mime_type": mime, "contenu_base64": b64(contenu)})),
    )
    .await
}

/// Type MIME réellement servi pour un document (via un lien court).
async fn type_servi(m: &Monde, doc: &Value) -> String {
    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt;
    let id = doc["id"].as_str().expect("id");
    let l = m
        .appel(
            "POST",
            &format!("/documents/{id}/lien"),
            Some(m.jeton(Direction)),
            None,
        )
        .await;
    assert_eq!(l.statut, StatusCode::OK, "{}", l.texte);
    let res = m
        .routeur
        .clone()
        .oneshot(
            Request::builder()
                .uri(l.json["url"].as_str().unwrap())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    res.headers()
        .get("content-type")
        .map(|v| v.to_str().unwrap().to_string())
        .unwrap_or_default()
}

fn zip_ooxml(avec_macro: bool) -> Vec<u8> {
    let mut entrees: Vec<(&str, &[u8])> = vec![
        ("[Content_Types].xml", b"<Types/>"),
        ("word/document.xml", b"<w:document/>"),
    ];
    if avec_macro {
        entrees.push(("word/vbaProject.bin", b"macro"));
    }
    let mut out = Vec::new();
    let mut cd = Vec::new();
    for (nom, data) in &entrees {
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
    out.extend_from_slice(b"PK\x05\x06\0\0\0\0");
    out.extend_from_slice(&(entrees.len() as u16).to_le_bytes());
    out.extend_from_slice(&(entrees.len() as u16).to_le_bytes());
    out.extend_from_slice(&(cd.len() as u32).to_le_bytes());
    out.extend_from_slice(&cd_off.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out
}

#[test]
fn document_vrai_pdf_accepte_type_reel_retenu() {
    let Some(m) = monde_depot() else { return };
    rt_run(async {
        let r = depot_document(m, "Certificat", "application/pdf", PDF).await;
        assert_eq!(r.statut, StatusCode::CREATED, "{}", r.texte);
        assert_eq!(type_servi(m, &r.json).await, "application/pdf");
        // Type déclaré générique : accepté, type réel retenu (PNG).
        let r = depot_document(m, "Scan", "application/octet-stream", PNG).await;
        assert_eq!(r.statut, StatusCode::CREATED, "{}", r.texte);
        assert_eq!(type_servi(m, &r.json).await, "image/png");
        // Document Office sans macro.
        let r = depot_document(
            m,
            "Lettre",
            "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
            &zip_ooxml(false),
        )
        .await;
        assert_eq!(r.statut, StatusCode::CREATED, "{}", r.texte);
    });
}

#[test]
fn document_fichiers_actifs_et_types_trompeurs_refuses() {
    let Some(m) = monde_depot() else { return };
    rt_run(async {
        // HTML renommé .pdf, déclaré PDF.
        let r = depot_document(m, "facture.pdf", "application/pdf", HTML).await;
        assert_eq!(r.statut, StatusCode::UNPROCESSABLE_ENTITY, "{}", r.texte);
        // SVG avec script déclaré image/png.
        let r = depot_document(m, "logo.png", "image/png", SVG).await;
        assert_eq!(r.statut, StatusCode::UNPROCESSABLE_ENTITY, "{}", r.texte);
        // Exécutable déclaré PDF.
        let r = depot_document(m, "x.pdf", "application/pdf", EXE).await;
        assert_eq!(r.statut, StatusCode::UNPROCESSABLE_ENTITY, "{}", r.texte);
        // Extension active dans le nom, même avec un vrai PDF.
        let r = depot_document(m, "facture.pdf.exe", "application/pdf", PDF).await;
        assert_eq!(r.statut, StatusCode::UNPROCESSABLE_ENTITY, "{}", r.texte);
        // Type déclaré contradictoire (octets PNG, déclaré PDF).
        let r = depot_document(m, "x", "application/pdf", PNG).await;
        assert_eq!(r.statut, StatusCode::UNPROCESSABLE_ENTITY, "{}", r.texte);
        // Document Office avec macros.
        let r = depot_document(m, "macro", "application/octet-stream", &zip_ooxml(true)).await;
        assert_eq!(r.statut, StatusCode::UNPROCESSABLE_ENTITY, "{}", r.texte);
        // Aucun lien de fichier n'a été produit pour un refus.
        assert!(!r.texte.contains("/fichiers/"));
    });
}

#[test]
fn document_trop_gros_refuse() {
    let Some(m) = monde_depot() else { return };
    rt_run(async {
        // 11 Mio (> défaut 10 Mio) : refusé (par le plafond de corps ou par le service).
        let mut gros = PDF.to_vec();
        gros.resize(11 * 1024 * 1024, b' ');
        let r = depot_document(m, "gros", "application/pdf", &gros).await;
        assert_eq!(r.statut, StatusCode::PAYLOAD_TOO_LARGE, "{}", r.statut);
    });
}

#[test]
fn photo_images_seulement_et_taille() {
    let Some(m) = monde_depot() else { return };
    rt_run(async {
        let r = depot_photo(m, "image/jpeg", JPEG).await;
        assert!(r.est_succes(), "{} {}", r.statut, r.texte);
        let r = depot_photo(m, "image/png", PNG).await;
        assert!(r.est_succes(), "{} {}", r.statut, r.texte);
        // PDF présenté comme photo : refusé, quel que soit le type déclaré.
        for mime in ["image/png", "image/jpeg", "application/pdf"] {
            let r = depot_photo(m, mime, PDF).await;
            assert!(
                matches!(
                    r.statut,
                    StatusCode::UNPROCESSABLE_ENTITY | StatusCode::BAD_REQUEST
                ),
                "{mime} : {} {}",
                r.statut,
                r.texte
            );
        }
        // SVG avec script, annoncé image/png : refusé.
        let r = depot_photo(m, "image/png", SVG).await;
        assert_eq!(r.statut, StatusCode::UNPROCESSABLE_ENTITY, "{}", r.texte);
        // Trop gros (> 5 Mio).
        let mut gros = JPEG.to_vec();
        gros.resize(6 * 1024 * 1024, 0);
        let r = depot_photo(m, "image/jpeg", &gros).await;
        assert_eq!(r.statut, StatusCode::PAYLOAD_TOO_LARGE, "{}", r.statut);
    });
}

#[test]
fn piece_pai_pdf_et_images_seulement() {
    let Some(m) = monde_depot() else { return };
    rt_run(async {
        let inf = m.jeton(Infirmiere).to_string();
        let eleve = m.a.objets.eleve;
        let r = m
            .appel(
                "PUT",
                &format!("/eleves/{eleve}/pai"),
                Some(&inf),
                Some(&json!({
                    "motif": "allergie", "mesures_accueil": "trousse", "protocole_urgence": "15",
                    "date_debut": "2025-09-01", "actif": true
                })),
            )
            .await;
        assert!(r.est_succes(), "PAI : {} {}", r.statut, r.texte);
        let depot = |nom: &'static str, mime: &'static str, c: Vec<u8>| {
            let inf = inf.clone();
            async move {
                m.appel(
                    "POST",
                    &format!("/eleves/{eleve}/pai/pieces"),
                    Some(&inf),
                    Some(&json!({"nom": nom, "mime_type": mime, "contenu_base64": b64(&c)})),
                )
                .await
            }
        };
        assert_eq!(
            depot("ordo.pdf", "application/pdf", PDF.to_vec())
                .await
                .statut,
            StatusCode::CREATED
        );
        assert_eq!(
            depot("ordo.jpg", "image/jpeg", JPEG.to_vec()).await.statut,
            StatusCode::CREATED
        );
        for (nom, mime, c) in [
            ("ordo.pdf", "application/pdf", HTML.to_vec()),
            ("ordo.txt", "text/plain", b"texte".to_vec()),
            ("ordo.svg", "image/svg+xml", SVG.to_vec()),
            ("ordo.pdf", "application/pdf", EXE.to_vec()),
        ] {
            let r = depot(nom, mime, c).await;
            assert!(
                matches!(
                    r.statut,
                    StatusCode::UNPROCESSABLE_ENTITY | StatusCode::BAD_REQUEST
                ),
                "{nom} : {} {}",
                r.statut,
                r.texte
            );
        }
    });
}

#[test]
fn piece_message_signatures_taille_et_nom() {
    let Some(m) = monde_depot() else { return };
    rt_run(async {
        let msg = message(m).await;
        // Vrai PDF avec un nom hostile : nom assaini dans la réponse.
        let r = depot_piece(m, msg, "../../dossier\r\n.pdf", "application/pdf", PDF).await;
        assert_eq!(r.statut, StatusCode::CREATED, "{}", r.texte);
        let nom = r.json["nom"].as_str().unwrap();
        assert!(
            !nom.contains('/') && !nom.contains('\\') && !nom.contains('\r') && !nom.contains('\n'),
            "{nom:?}"
        );
        assert_eq!(r.json["mime_type"], Value::String("application/pdf".into()));
        // octet-stream déclaré : le type réel est retenu.
        let r = depot_piece(m, msg, "scan", "application/octet-stream", PNG).await;
        assert_eq!(r.statut, StatusCode::CREATED, "{}", r.texte);
        assert_eq!(r.json["mime_type"], Value::String("image/png".into()));
        // HTML renommé .pdf / SVG à script : refusés.
        for (nom, mime, c) in [
            ("a.pdf", "application/pdf", HTML),
            ("b.png", "image/png", SVG),
            ("c.txt", "text/plain", HTML),
        ] {
            let r = depot_piece(m, msg, nom, mime, c).await;
            assert_eq!(
                r.statut,
                StatusCode::UNPROCESSABLE_ENTITY,
                "{nom} : {}",
                r.texte
            );
        }
        // Taille : plafond abaissé à 2 Kio pour ce test.
        let mut gros = PDF.to_vec();
        gros.resize(3 * 1024, b' ');
        let r = depot_piece(m, msg, "gros.pdf", "application/pdf", &gros).await;
        assert_eq!(r.statut, StatusCode::PAYLOAD_TOO_LARGE, "{}", r.texte);
        // Vide : refusé.
        let r = depot_piece(m, msg, "vide.pdf", "application/pdf", b"").await;
        assert!(r.statut.is_client_error(), "{}", r.statut);
    });
}

#[test]
fn depot_exige_toujours_les_droits() {
    let Some(m) = monde_depot() else { return };
    rt_run(async {
        // Sans session : 401, avant tout contrôle de contenu.
        let r = m
            .appel(
                "POST",
                &format!("/eleves/{}/documents", m.a.objets.eleve),
                None,
                Some(&json!({
                    "titre": "x", "tiroir": "famille", "confidentialite": "standard",
                    "mime_type": "application/pdf", "contenu_base64": b64(PDF),
                })),
            )
            .await;
        assert_eq!(r.statut, StatusCode::UNAUTHORIZED);
        // Parent d'un autre foyer : refusé même avec un fichier valide.
        let r = m
            .appel(
                "POST",
                &format!("/eleves/{}/documents", m.a.objets.eleve),
                Some(m.jeton(ParentAutreFoyer)),
                Some(&json!({
                    "titre": "x", "tiroir": "famille", "confidentialite": "standard",
                    "mime_type": "application/pdf", "contenu_base64": b64(PDF),
                })),
            )
            .await;
        assert!(r.est_refus(), "{}", r.statut);
    });
}
