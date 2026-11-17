//! Ajout d'une **page de signatures** à un PDF produit par ce moteur, par mise à jour incrémentale (socle 7, lot D6).
//!
//! Le fichier d'origine n'est jamais modifié : ses octets restent un préfixe du résultat (une mise à jour incrémentale
//! ne fait qu'ajouter). La page n'est ajoutée qu'aux PDF dont le catalogue, l'arbre de pages et le trailer ont la forme
//! écrite par [`crate::Document`] (table xref classique, `/Kids [ … ] /Count n`) ; tout autre PDF est refusé (l'appelant
//! se rabat alors sur le seul dossier de preuve).

use crate::ecrit::{litterale, nb};
use crate::{couper, encoder_winansi, Police, HAUTEUR, LARGEUR, MARGE_D, MARGE_G};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ErreurSignatures(pub &'static str);

impl std::fmt::Display for ErreurSignatures {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "page de signatures impossible : {}", self.0)
    }
}
impl std::error::Error for ErreurSignatures {}

fn trouver(h: &[u8], a: &[u8], depuis: usize) -> Option<usize> {
    if a.is_empty() || h.len() < a.len() || depuis > h.len() - a.len() {
        return None;
    }
    h[depuis..].windows(a.len()).position(|w| w == a).map(|p| p + depuis)
}

fn dernier(h: &[u8], a: &[u8]) -> Option<usize> {
    if a.is_empty() || h.len() < a.len() {
        return None;
    }
    h.windows(a.len()).rposition(|w| w == a)
}

fn entier_apres(h: &[u8], cle: &[u8], depuis: usize) -> Option<usize> {
    let p = trouver(h, cle, depuis)? + cle.len();
    let s: String = h[p..]
        .iter()
        .skip_while(|b| b.is_ascii_whitespace())
        .take_while(|b| b.is_ascii_digit())
        .map(|&b| b as char)
        .collect();
    s.parse().ok()
}

/// Ajoute une page « Signatures » (titre puis lignes de texte) à `pdf`.
pub fn ajouter_page_signatures(
    pdf: &[u8],
    titre: &str,
    lignes: &[String],
) -> Result<Vec<u8>, ErreurSignatures> {
    let err = ErreurSignatures;
    if !pdf.starts_with(b"%PDF-") {
        return Err(err("ce n'est pas un PDF"));
    }
    let sx = dernier(pdf, b"startxref").ok_or(err("startxref absent"))?;
    let prev = entier_apres(pdf, b"startxref", sx).ok_or(err("startxref illisible"))?;
    let tr = dernier(&pdf[..sx], b"trailer").ok_or(err("trailer absent"))?;
    let trailer = &pdf[tr..sx];
    if trouver(trailer, b"/Prev", 0).is_some() || trouver(trailer, b"/Encrypt", 0).is_some() {
        return Err(err("PDF déjà mis à jour ou chiffré"));
    }
    let taille = entier_apres(trailer, b"/Size", 0).ok_or(err("/Size absent"))?;
    let racine = entier_apres(trailer, b"/Root", 0).ok_or(err("/Root absent"))?;
    let info = entier_apres(trailer, b"/Info", 0);
    let corps_objet = |n: usize| -> Option<&[u8]> {
        let marque = format!("\n{n} 0 obj\n");
        let d = trouver(pdf, marque.as_bytes(), 0)? + marque.len();
        let f = trouver(pdf, b"\nendobj", d)?;
        Some(&pdf[d..f])
    };
    let cat = corps_objet(racine).ok_or(err("catalogue introuvable"))?;
    let pages_id = entier_apres(cat, b"/Pages", 0).ok_or(err("/Pages absent"))?;
    let pages = corps_objet(pages_id).ok_or(err("arbre de pages introuvable"))?;
    let k0 = trouver(pages, b"/Kids [", 0).ok_or(err("/Kids absent"))? + b"/Kids [".len();
    let k1 = trouver(pages, b"]", k0).ok_or(err("/Kids illisible"))?;
    let kids = std::str::from_utf8(&pages[k0..k1]).map_err(|_| err("/Kids illisible"))?;
    let compte = entier_apres(pages, b"/Count", 0).ok_or(err("/Count absent"))?;
    if pages.windows(6).any(|w| w == b"/Parent") || compte == 0 {
        return Err(err("arbre de pages imbriqué non pris en charge"));
    }

    let id_police = taille;
    let id_flux = taille + 1;
    let id_page = taille + 2;

    // Contenu de la page.
    let mut c: Vec<u8> = Vec::new();
    let mut y = HAUTEUR - 80.0;
    let ecrire = |c: &mut Vec<u8>, gras: bool, t: f32, txt: &str, y: &mut f32| {
        let police = if gras { Police::HelveticaGras } else { Police::Helvetica };
        let nom = if gras { "FB" } else { "F1" };
        for l in couper(police, t, &encoder_winansi(txt), MARGE_D - MARGE_G) {
            if *y < 60.0 {
                return;
            }
            c.extend_from_slice(format!("BT /{nom} {} Tf {} {} Td ", nb(t), nb(MARGE_G), nb(*y)).as_bytes());
            litterale(&l, c);
            c.extend_from_slice(b" Tj ET\n");
            *y -= t * 1.35;
        }
    };
    ecrire(&mut c, true, 16.0, titre, &mut y);
    y -= 10.0;
    for l in lignes {
        if l.trim().is_empty() {
            y -= 8.0;
        } else {
            ecrire(&mut c, false, 9.5, l, &mut y);
        }
    }

    let mut sortie = pdf.to_vec();
    if !sortie.ends_with(b"\n") {
        sortie.push(b'\n');
    }
    let mut offsets: Vec<(usize, usize)> = Vec::new();
    let mut objet = |sortie: &mut Vec<u8>, n: usize, corps: &[u8]| {
        offsets.push((n, sortie.len()));
        sortie.extend_from_slice(format!("{n} 0 obj\n").as_bytes());
        sortie.extend_from_slice(corps);
        sortie.extend_from_slice(b"\nendobj\n");
    };
    objet(
        &mut sortie,
        pages_id,
        format!("<< /Type /Pages /Kids [ {}{id_page} 0 R ] /Count {} >>", kids.trim_start().to_string() + if kids.trim().is_empty() { "" } else { " " }, compte + 1)
            .as_bytes(),
    );
    objet(
        &mut sortie,
        id_police,
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>",
    );
    let mut flux = format!("<< /Length {} >>\nstream\n", c.len()).into_bytes();
    flux.extend_from_slice(&c);
    flux.extend_from_slice(b"\nendstream");
    objet(&mut sortie, id_flux, &flux);
    objet(
        &mut sortie,
        id_page,
        format!(
            "<< /Type /Page /Parent {pages_id} 0 R /MediaBox [0 0 {} {}] /Resources << /Font << /F1 {id_police} 0 R /FB {} 0 R >> >> /Contents {id_flux} 0 R >>",
            nb(LARGEUR),
            nb(HAUTEUR),
            id_police + 3
        )
        .as_bytes(),
    );
    // Police grasse (objet supplémentaire, après la page).
    objet(
        &mut sortie,
        id_police + 3,
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica-Bold /Encoding /WinAnsiEncoding >>",
    );
    let xref = sortie.len();
    offsets.sort();
    sortie.extend_from_slice(b"xref\n");
    for (n, o) in &offsets {
        sortie.extend_from_slice(format!("{n} 1\n{o:010} 00000 n \n").as_bytes());
    }
    let info_txt = info.map(|i| format!(" /Info {i} 0 R")).unwrap_or_default();
    sortie.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root {racine} 0 R{info_txt} /Prev {prev} >>\nstartxref\n{xref}\n%%EOF\n",
            taille + 4
        )
        .as_bytes(),
    );
    Ok(sortie)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Document, Meta};

    fn pdf_base() -> Vec<u8> {
        let mut d = Document::new(Meta::default());
        d.titre("Certificat");
        d.paragraphe("Texte du document.");
        d.octets()
    }

    #[test]
    fn page_ajoutee_sans_toucher_l_original() {
        let base = pdf_base();
        let signe = ajouter_page_signatures(
            &base,
            "Signatures électroniques",
            &["Signé par Léa Dupont le 2026-11-17".to_string()],
        )
        .unwrap();
        assert!(signe.starts_with(&base), "l'original reste un préfixe");
        assert!(signe.ends_with(b"%%EOF\n"));
        let sx = dernier(&signe, b"startxref").unwrap();
        let off = entier_apres(&signe, b"startxref", sx).unwrap();
        assert_eq!(&signe[off..off + 5], b"xref\n");
        assert!(trouver(&signe, b"Dupont", 0).is_some());
        // Deux mises à jour successives sont refusées (arbre déjà complété) ou restent un préfixe.
        assert!(ajouter_page_signatures(&signe, "x", &[]).is_err());
    }

    #[test]
    fn pdf_etranger_refuse() {
        assert!(ajouter_page_signatures(b"hello", "t", &[]).is_err());
    }
}
