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
/// Vrais fichiers 2×2 produits par un encodeur standard (Pillow) : le nettoyeur de photos refuse un simple
/// en-tête tronqué.
const PNG: &[u8] = &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x02, 0x08, 0x02, 0x00, 0x00, 0x00, 0xFD, 0xD4, 0x9A, 0x73, 0x00, 0x00, 0x00, 0x16, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9C, 0x63, 0x3C, 0x21, 0x27, 0xC7, 0xC0, 0xC0, 0xC0, 0xC4, 0xC0, 0xC0, 0xC0, 0xC0, 0xC0, 0x00, 0x00, 0x0D, 0x04, 0x01, 0x08, 0x7D, 0x71, 0xCA, 0x66, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82];
const JPEG: &[u8] = &[0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, 0x4A, 0x46, 0x49, 0x46, 0x00, 0x01, 0x01, 0x00, 0x00, 0x01, 0x00, 0x01, 0x00, 0x00, 0xFF, 0xDB, 0x00, 0x43, 0x00, 0x06, 0x04, 0x05, 0x06, 0x05, 0x04, 0x06, 0x06, 0x05, 0x06, 0x07, 0x07, 0x06, 0x08, 0x0A, 0x10, 0x0A, 0x0A, 0x09, 0x09, 0x0A, 0x14, 0x0E, 0x0F, 0x0C, 0x10, 0x17, 0x14, 0x18, 0x18, 0x17, 0x14, 0x16, 0x16, 0x1A, 0x1D, 0x25, 0x1F, 0x1A, 0x1B, 0x23, 0x1C, 0x16, 0x16, 0x20, 0x2C, 0x20, 0x23, 0x26, 0x27, 0x29, 0x2A, 0x29, 0x19, 0x1F, 0x2D, 0x30, 0x2D, 0x28, 0x30, 0x25, 0x28, 0x29, 0x28, 0xFF, 0xDB, 0x00, 0x43, 0x01, 0x07, 0x07, 0x07, 0x0A, 0x08, 0x0A, 0x13, 0x0A, 0x0A, 0x13, 0x28, 0x1A, 0x16, 0x1A, 0x28, 0x28, 0x28, 0x28, 0x28, 0x28, 0x28, 0x28, 0x28, 0x28, 0x28, 0x28, 0x28, 0x28, 0x28, 0x28, 0x28, 0x28, 0x28, 0x28, 0x28, 0x28, 0x28, 0x28, 0x28, 0x28, 0x28, 0x28, 0x28, 0x28, 0x28, 0x28, 0x28, 0x28, 0x28, 0x28, 0x28, 0x28, 0x28, 0x28, 0x28, 0x28, 0x28, 0x28, 0x28, 0x28, 0x28, 0x28, 0x28, 0x28, 0xFF, 0xC0, 0x00, 0x11, 0x08, 0x00, 0x02, 0x00, 0x02, 0x03, 0x01, 0x22, 0x00, 0x02, 0x11, 0x01, 0x03, 0x11, 0x01, 0xFF, 0xC4, 0x00, 0x1F, 0x00, 0x00, 0x01, 0x05, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0xFF, 0xC4, 0x00, 0xB5, 0x10, 0x00, 0x02, 0x01, 0x03, 0x03, 0x02, 0x04, 0x03, 0x05, 0x05, 0x04, 0x04, 0x00, 0x00, 0x01, 0x7D, 0x01, 0x02, 0x03, 0x00, 0x04, 0x11, 0x05, 0x12, 0x21, 0x31, 0x41, 0x06, 0x13, 0x51, 0x61, 0x07, 0x22, 0x71, 0x14, 0x32, 0x81, 0x91, 0xA1, 0x08, 0x23, 0x42, 0xB1, 0xC1, 0x15, 0x52, 0xD1, 0xF0, 0x24, 0x33, 0x62, 0x72, 0x82, 0x09, 0x0A, 0x16, 0x17, 0x18, 0x19, 0x1A, 0x25, 0x26, 0x27, 0x28, 0x29, 0x2A, 0x34, 0x35, 0x36, 0x37, 0x38, 0x39, 0x3A, 0x43, 0x44, 0x45, 0x46, 0x47, 0x48, 0x49, 0x4A, 0x53, 0x54, 0x55, 0x56, 0x57, 0x58, 0x59, 0x5A, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x69, 0x6A, 0x73, 0x74, 0x75, 0x76, 0x77, 0x78, 0x79, 0x7A, 0x83, 0x84, 0x85, 0x86, 0x87, 0x88, 0x89, 0x8A, 0x92, 0x93, 0x94, 0x95, 0x96, 0x97, 0x98, 0x99, 0x9A, 0xA2, 0xA3, 0xA4, 0xA5, 0xA6, 0xA7, 0xA8, 0xA9, 0xAA, 0xB2, 0xB3, 0xB4, 0xB5, 0xB6, 0xB7, 0xB8, 0xB9, 0xBA, 0xC2, 0xC3, 0xC4, 0xC5, 0xC6, 0xC7, 0xC8, 0xC9, 0xCA, 0xD2, 0xD3, 0xD4, 0xD5, 0xD6, 0xD7, 0xD8, 0xD9, 0xDA, 0xE1, 0xE2, 0xE3, 0xE4, 0xE5, 0xE6, 0xE7, 0xE8, 0xE9, 0xEA, 0xF1, 0xF2, 0xF3, 0xF4, 0xF5, 0xF6, 0xF7, 0xF8, 0xF9, 0xFA, 0xFF, 0xC4, 0x00, 0x1F, 0x01, 0x00, 0x03, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0xFF, 0xC4, 0x00, 0xB5, 0x11, 0x00, 0x02, 0x01, 0x02, 0x04, 0x04, 0x03, 0x04, 0x07, 0x05, 0x04, 0x04, 0x00, 0x01, 0x02, 0x77, 0x00, 0x01, 0x02, 0x03, 0x11, 0x04, 0x05, 0x21, 0x31, 0x06, 0x12, 0x41, 0x51, 0x07, 0x61, 0x71, 0x13, 0x22, 0x32, 0x81, 0x08, 0x14, 0x42, 0x91, 0xA1, 0xB1, 0xC1, 0x09, 0x23, 0x33, 0x52, 0xF0, 0x15, 0x62, 0x72, 0xD1, 0x0A, 0x16, 0x24, 0x34, 0xE1, 0x25, 0xF1, 0x17, 0x18, 0x19, 0x1A, 0x26, 0x27, 0x28, 0x29, 0x2A, 0x35, 0x36, 0x37, 0x38, 0x39, 0x3A, 0x43, 0x44, 0x45, 0x46, 0x47, 0x48, 0x49, 0x4A, 0x53, 0x54, 0x55, 0x56, 0x57, 0x58, 0x59, 0x5A, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x69, 0x6A, 0x73, 0x74, 0x75, 0x76, 0x77, 0x78, 0x79, 0x7A, 0x82, 0x83, 0x84, 0x85, 0x86, 0x87, 0x88, 0x89, 0x8A, 0x92, 0x93, 0x94, 0x95, 0x96, 0x97, 0x98, 0x99, 0x9A, 0xA2, 0xA3, 0xA4, 0xA5, 0xA6, 0xA7, 0xA8, 0xA9, 0xAA, 0xB2, 0xB3, 0xB4, 0xB5, 0xB6, 0xB7, 0xB8, 0xB9, 0xBA, 0xC2, 0xC3, 0xC4, 0xC5, 0xC6, 0xC7, 0xC8, 0xC9, 0xCA, 0xD2, 0xD3, 0xD4, 0xD5, 0xD6, 0xD7, 0xD8, 0xD9, 0xDA, 0xE2, 0xE3, 0xE4, 0xE5, 0xE6, 0xE7, 0xE8, 0xE9, 0xEA, 0xF2, 0xF3, 0xF4, 0xF5, 0xF6, 0xF7, 0xF8, 0xF9, 0xFA, 0xFF, 0xDA, 0x00, 0x0C, 0x03, 0x01, 0x00, 0x02, 0x11, 0x03, 0x11, 0x00, 0x3F, 0x00, 0xE0, 0x28, 0xA2, 0x8A, 0xF9, 0xE3, 0xF6, 0x13, 0xFF, 0xD9];
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
    let url = l.json["url"].as_str().unwrap();
    // Le lien est lié à l'utilisateur : sans session, refus (jamais le fichier).
    let sans = m
        .routeur
        .clone()
        .oneshot(Request::builder().uri(url).body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(sans.status(), StatusCode::UNAUTHORIZED, "lien servi sans session");
    let res = m
        .routeur
        .clone()
        .oneshot(
            Request::builder()
                .uri(url)
                .header("authorization", format!("Bearer {}", m.jeton(Direction)))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
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
