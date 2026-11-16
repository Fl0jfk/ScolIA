//! Décodage strict des **JSON stockés directement dans S3** (T0 §2.4) : dépôts de fiches de rentrée en
//! attente (`rentree/submissions/pending/<jeton>/meta.json`) et demandes en attente de vérification
//! d'e-mail (`requests/pending/<jeton>/meta.json`). Les autres objets sont classés sans être lus.

use super::cellule::{refus, Refus};
use super::forme::{valider, Champ, Forme};
use crate::s3;
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NatureS3 {
    /// `meta.json` d'un dépôt de fiche de rentrée en attente.
    RentreeEnAttente,
    /// `meta.json` d'une demande en attente de vérification.
    DemandeEnAttente,
    /// Chemin de secrets : refusé, jamais lu (R-16).
    Secret,
    /// Base de connaissances de l'assistant : perdable.
    Connaissance,
    /// Autre JSON : inventorié, non repris par défaut (T0 §2.4).
    JsonInventorie,
    /// Fichier (photo, pièce, PDF…) : copié par le lot fichiers (R7), pas lu ici.
    Fichier,
}

pub fn nature(cle: &str) -> NatureS3 {
    if s3::est_secret(cle) {
        return NatureS3::Secret;
    }
    let seg: Vec<&str> = cle.split('/').collect();
    if seg.len() == 5 && seg[..3] == ["rentree", "submissions", "pending"] && seg[4] == "meta.json" {
        return NatureS3::RentreeEnAttente;
    }
    if seg.len() == 4 && seg[..2] == ["requests", "pending"] && seg[3] == "meta.json" {
        return NatureS3::DemandeEnAttente;
    }
    if seg[0] == "knowledge" {
        return NatureS3::Connaissance;
    }
    if cle.to_ascii_lowercase().ends_with(".json") {
        return NatureS3::JsonInventorie;
    }
    NatureS3::Fichier
}

fn r(nom: &'static str, forme: Forme) -> Champ {
    Champ { nom, forme, requis: true }
}
fn o(nom: &'static str, forme: Forme) -> Champ {
    Champ { nom, forme, requis: false }
}
fn tab(f: Forme) -> Forme {
    Forme::Tableau(Box::new(f))
}

pub fn forme(n: NatureS3) -> Option<Forme> {
    Some(match n {
        NatureS3::RentreeEnAttente => Forme::Objet(vec![
            r("version", Forme::Nombre),
            r("senderEmail", Forme::Texte),
            o("studentName", Forme::Texte),
            r("fileName", Forme::Texte),
            r("contentType", Forme::Texte),
            r("size", Forme::Nombre),
            r("fileKey", Forme::Texte),
            r("itemId", Forme::Texte),
            r("itemTitle", Forme::Texte),
            r("establishmentId", Forme::Texte),
            r("establishmentLabel", Forme::Texte),
            r("recipientEmails", tab(Forme::Texte)),
            r("createdAt", Forme::Texte),
            r("expiresAt", Forme::Texte),
        ]),
        NatureS3::DemandeEnAttente => Forme::Objet(vec![
            r("version", Forme::Nombre),
            r("firstName", Forme::Texte),
            r("lastName", Forme::Texte),
            r("email", Forme::Texte),
            r("phone", Forme::Texte),
            r("subject", Forme::Texte),
            r("description", Forme::Texte),
            r("createdAt", Forme::Texte),
            r("expiresAt", Forme::Texte),
            r("attachmentKeys", tab(Forme::Texte)),
            r(
                "attachmentMeta",
                tab(Forme::Objet(vec![
                    r("id", Forme::Texte),
                    r("fileName", Forme::Texte),
                    r("contentType", Forme::Texte),
                    r("size", Forme::Nombre),
                    r("uploadedAt", Forme::Texte),
                ])),
            ),
            o(
                "parentContext",
                Forme::Objet(vec![
                    r("source", Forme::Enum(&["parent_portal"])),
                    r("matched", Forme::Booleen),
                    r(
                        "children",
                        tab(Forme::Objet(vec![
                            r("ine", Forme::Texte),
                            r("nom", Forme::Texte),
                            r("prenom", Forme::Texte),
                            o("classe", Forme::Texte),
                        ])),
                    ),
                ]),
            ),
        ]),
        _ => return None,
    })
}

/// Décode un `meta.json`. Retourne le JSON validé ou les refus.
pub fn decoder(n: NatureS3, contenu: &str) -> Result<Value, Vec<Refus>> {
    let f = forme(n).ok_or_else(|| vec![refus("s3_nature", "objet S3 sans décodeur")])?;
    if contenu.len() > 1 << 20 {
        return Err(vec![refus("s3_taille", "objet JSON au-delà de 1 Mio")]);
    }
    let v: Value = serde_json::from_str(contenu).map_err(|_| vec![refus("json_invalide", "JSON mal formé")])?;
    let mut e = valider(&f, &v);
    if v.get("version").and_then(Value::as_i64) != Some(1) && e.iter().all(|x| x.code != "json_type") {
        e.push(refus("s3_version", "version de document inconnue (1 attendu)"));
    }
    if n == NatureS3::DemandeEnAttente && e.is_empty() {
        let (k, m) = (v["attachmentKeys"].as_array().map(Vec::len), v["attachmentMeta"].as_array().map(Vec::len));
        if k != m {
            e.push(refus("s3_pieces_incoherentes", "attachmentKeys et attachmentMeta de longueurs différentes"));
        }
    }
    if e.is_empty() {
        Ok(v)
    } else {
        Err(e)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn natures() {
        assert_eq!(nature("rentree/submissions/pending/abc/meta.json"), NatureS3::RentreeEnAttente);
        assert_eq!(nature("requests/pending/abc/meta.json"), NatureS3::DemandeEnAttente);
        assert_eq!(nature("requests/pending/abc/files/x.pdf"), NatureS3::Fichier);
        assert_eq!(nature("tenants/index.json"), NatureS3::Secret);
        assert_eq!(nature("knowledge/a.json"), NatureS3::Connaissance);
        assert_eq!(nature("travels/1/devis.json"), NatureS3::JsonInventorie);
    }

    #[test]
    fn decodage_strict() {
        let bon = r#"{"version":1,"firstName":"A","lastName":"B","email":"a@example.test","phone":"0","subject":"s","description":"d","createdAt":"2026-11-01T00:00:00Z","expiresAt":"2026-11-04T00:00:00Z","attachmentKeys":[],"attachmentMeta":[]}"#;
        assert!(decoder(NatureS3::DemandeEnAttente, bon).is_ok());
        let inconnu = bon.replace("\"phone\"", "\"x\":1,\"phone\"");
        let e = decoder(NatureS3::DemandeEnAttente, &inconnu).unwrap_err();
        assert_eq!(e[0].code, "json_cle_inconnue");
        let v2 = bon.replace("\"version\":1", "\"version\":2");
        assert_eq!(decoder(NatureS3::DemandeEnAttente, &v2).unwrap_err()[0].code, "s3_version");
        assert_eq!(decoder(NatureS3::DemandeEnAttente, "{").unwrap_err()[0].code, "json_invalide");
    }
}
