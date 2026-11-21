//! Rendu PDF minimal (Helvetica, A4) — facture & quittance.
//! Pas de dépendance externe : PDF 1.4 texte seul, prêt à télécharger.

use crate::facturation::{reste_a_payer, Facture, FactureLigne, Quittance};

/// Formate des centimes en « 70,00 EUR ».
pub fn euros_centimes(centimes: i64) -> String {
    let neg = centimes < 0;
    let abs = centimes.unsigned_abs();
    let euros = abs / 100;
    let cts = abs % 100;
    let s = format!("{euros},{cts:02} EUR");
    if neg {
        format!("-{s}")
    } else {
        s
    }
}

fn pdf_escape(s: &str) -> String {
    let safe = ascii_latin(s);
    let mut out = String::with_capacity(safe.len());
    for c in safe.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '(' => out.push_str("\\("),
            ')' => out.push_str("\\)"),
            '\r' | '\n' => out.push(' '),
            other => out.push(other),
        }
    }
    out
}

fn ascii_latin(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            'à' | 'á' | 'â' | 'ä' | 'À' | 'Á' | 'Â' | 'Ä' => 'a',
            'ç' | 'Ç' => 'c',
            'è' | 'é' | 'ê' | 'ë' | 'È' | 'É' | 'Ê' | 'Ë' => 'e',
            'ì' | 'í' | 'î' | 'ï' | 'Ì' | 'Í' | 'Î' | 'Ï' => 'i',
            'ñ' | 'Ñ' => 'n',
            'ò' | 'ó' | 'ô' | 'ö' | 'Ò' | 'Ó' | 'Ô' | 'Ö' => 'o',
            'ù' | 'ú' | 'û' | 'ü' | 'Ù' | 'Ú' | 'Û' | 'Ü' => 'u',
            'ÿ' | 'Ÿ' => 'y',
            'œ' | 'Œ' => 'o',
            '€' => 'E',
            c if c.is_ascii() => c,
            _ => '?',
        })
        .collect()
}

fn build_pdf(lines: &[String]) -> Vec<u8> {
    let mut content = String::from("BT\n/F1 11 Tf\n50 800 Td\n14 TL\n");
    for (i, line) in lines.iter().enumerate() {
        if i > 0 {
            content.push_str("T*\n");
        }
        content.push_str(&format!("({}) Tj\n", pdf_escape(line)));
    }
    content.push_str("ET\n");
    let stream = content.into_bytes();

    let mut objects: Vec<Vec<u8>> = Vec::new();
    // 1 Catalog
    objects.push(b"<< /Type /Catalog /Pages 2 0 R >>".to_vec());
    // 2 Pages
    objects.push(b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec());
    // 3 Page
    objects.push(
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] \
/Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >>"
            .to_vec(),
    );
    // 4 Contents
    let mut contents_obj = format!("<< /Length {} >>\nstream\n", stream.len()).into_bytes();
    contents_obj.extend_from_slice(&stream);
    contents_obj.extend_from_slice(b"\nendstream");
    objects.push(contents_obj);
    // 5 Font
    objects.push(b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_vec());

    let mut out: Vec<u8> = b"%PDF-1.4\n%\xE2\xE3\xCF\xD3\n".to_vec();
    let mut offsets = Vec::with_capacity(objects.len() + 1);
    offsets.push(0); // unused index 0
    for (i, obj) in objects.iter().enumerate() {
        offsets.push(out.len());
        out.extend_from_slice(format!("{} 0 obj\n", i + 1).as_bytes());
        out.extend_from_slice(obj);
        out.extend_from_slice(b"\nendobj\n");
    }
    let xref_at = out.len();
    out.extend_from_slice(format!("xref\n0 {}\n", objects.len() + 1).as_bytes());
    out.extend_from_slice(b"0000000000 65535 f \n");
    for off in offsets.iter().skip(1) {
        out.extend_from_slice(format!("{off:010} 00000 n \n").as_bytes());
    }
    out.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n",
            objects.len() + 1
        )
        .as_bytes(),
    );
    out
}

/// PDF facture (émise ou soldée) avec lignes + soldes.
pub fn render_facture_pdf(
    facture: &Facture,
    lignes: &[FactureLigne],
    encaisse_centimes: i64,
) -> Vec<u8> {
    let mut lines = vec![
        "ScolIA — Facture".into(),
        String::new(),
        format!("Numero : {}", facture.numero),
        format!("Libelle : {}", facture.libelle),
        format!("Statut  : {}", facture.statut.as_str()),
    ];
    if let Some(n) = &facture.nature {
        lines.push(format!("Nature  : {n}"));
    }
    if let Some(a) = &facture.annee_scolaire {
        lines.push(format!("Annee   : {a}"));
    }
    lines.push(String::new());
    lines.push("Lignes".into());
    lines.push("------".into());
    for l in lignes {
        lines.push(format!(
            "{}  x{}  {}  = {}",
            l.libelle,
            l.quantite,
            euros_centimes(l.montant_unitaire_centimes),
            euros_centimes(l.total_centimes())
        ));
    }
    lines.push(String::new());
    lines.push(format!("Total    : {}", euros_centimes(facture.total_centimes)));
    lines.push(format!("Encaisse : {}", euros_centimes(encaisse_centimes)));
    lines.push(format!(
        "Reste    : {}",
        euros_centimes(reste_a_payer(facture.total_centimes, encaisse_centimes))
    ));
    lines.push(String::new());
    lines.push("Document generique ENT — sans valeur fiscale OGEC.".into());
    build_pdf(&lines)
}

/// PDF quittance (reçu de paiement).
pub fn render_quittance_pdf(quittance: &Quittance, facture_numero: &str) -> Vec<u8> {
    let lines = vec![
        "ScolIA — Quittance".into(),
        String::new(),
        format!("Numero quittance : {}", quittance.numero),
        format!("Facture          : {facture_numero}"),
        format!("Montant          : {}", euros_centimes(quittance.montant_centimes)),
        format!("Moyen            : {}", quittance.moyen.as_str()),
        format!(
            "Emise le         : {}",
            quittance.emise_at.format("%Y-%m-%d %H:%M UTC")
        ),
        String::new(),
        "Recu de paiement — document generique ENT.".into(),
    ];
    build_pdf(&lines)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::facturation::{FactureStatut, MoyenPaiement};
    use chrono::Utc;
    use uuid::Uuid;

    #[test]
    fn facture_pdf_commence_par_header() {
        let f = Facture {
            id: Uuid::new_v4(),
            etablissement_id: Uuid::new_v4(),
            foyer_id: Uuid::new_v4(),
            numero: "F-PDF-1".into(),
            libelle: "Cantine octobre".into(),
            statut: FactureStatut::Emise,
            total_centimes: 7000,
            nature: None,
            annee_scolaire: None,
            created_by_user_id: None,
            created_at: Utc::now(),
            emise_at: Some(Utc::now()),
            soldee_at: None,
        };
        let lignes = vec![FactureLigne {
            id: Uuid::new_v4(),
            etablissement_id: f.etablissement_id,
            facture_id: f.id,
            eleve_id: None,
            libelle: "Cantine".into(),
            quantite: 20,
            montant_unitaire_centimes: 450,
        }];
        let pdf = render_facture_pdf(&f, &lignes, 0);
        assert!(pdf.starts_with(b"%PDF-1.4"));
        let s = String::from_utf8_lossy(&pdf);
        assert!(s.contains("F-PDF-1"));
        assert!(s.contains("%%EOF"));
    }

    #[test]
    fn quittance_pdf_ok() {
        let q = Quittance {
            id: Uuid::new_v4(),
            etablissement_id: Uuid::new_v4(),
            foyer_id: Uuid::new_v4(),
            facture_id: Uuid::new_v4(),
            encaissement_id: Uuid::new_v4(),
            numero: "Q-F-1-01".into(),
            montant_centimes: 3500,
            moyen: MoyenPaiement::Cheque,
            emise_at: Utc::now(),
            created_by_user_id: None,
        };
        let pdf = render_quittance_pdf(&q, "F-1");
        assert!(pdf.starts_with(b"%PDF-1.4"));
        assert!(String::from_utf8_lossy(&pdf).contains("Q-F-1-01"));
    }
}
