//! Moteur PDF maison de l'ENT (D-062) : aucune dépendance, aucune cryptographie.
//!
//! * police TrueType **incorporée en sous-ensemble** (DejaVu Sans, CID Type 2, `Identity-H`,
//!   `ToUnicode`) par défaut : tout nom latin, grec ou cyrillique s'écrit correctement ;
//! * repli : polices standard (Helvetica, Helvetica-Bold, Times) en `WinAnsiEncoding`
//!   (`Document::avec_polices_standard`) : le français s'affiche, le reste est translittéré ;
//! * mesure des largeurs d'après les métriques AFM, retour à la ligne, alignements ;
//! * pages multiples, en-tête et pied répétés, « Page n sur N », tableaux à en-tête répété ;
//! * logo JPEG (`DCTDecode`, sans décodage) ;
//! * QR code maison dessiné en vecteurs ;
//! * flux **non compressés** à l'écriture (aucun compresseur) ; à la lecture des pièces reçues,
//!   un `inflate` maison (`inflate`) ouvre les flux d'objets des PDF modernes.

mod afm;
mod compositions;
mod doc;
mod ecrit;
mod fonte;
pub mod inflate;
mod jpeg;
mod normalise;
pub mod piece;
mod signatures;
pub use signatures::{ajouter_page_signatures, ErreurSignatures};
mod piece_flux;
mod qr;
pub mod releve;
mod sous_ensemble;
mod ttf;
mod winansi;

pub use afm::{largeur, largeur_code, Police};
pub use doc::{
    couper, Align, Colonne, DateHeure, Document, Ligne, Meta, Tableau, HAUTEUR, LARGEUR, MARGE_D,
    MARGE_G,
};
pub use fonte::Mode;
pub use jpeg::Jpeg;
pub use qr::{Qr, QrErreur};
pub use winansi::{depuis_octet, encoder as encoder_winansi, octet as octet_winansi};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PdfErreur {
    Jpeg(&'static str),
    Qr(QrErreur),
}

impl From<QrErreur> for PdfErreur {
    fn from(e: QrErreur) -> Self {
        PdfErreur::Qr(e)
    }
}

impl std::fmt::Display for PdfErreur {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PdfErreur::Jpeg(m) => write!(f, "image JPEG refusée : {m}"),
            PdfErreur::Qr(e) => write!(f, "{e}"),
        }
    }
}
impl std::error::Error for PdfErreur {}

/// Document de texte simple : un titre (première ligne non vide) puis des lignes ;
/// une ligne faite de tirets devient un filet. Sert aux générateurs « une ligne = une information ».
pub fn rendre_lignes(meta: Meta, pied: &str, lignes: &[String]) -> Vec<u8> {
    let mut d = Document::new(meta).avec_pied(pied);
    let mut premier = true;
    for l in lignes {
        if l.trim().is_empty() {
            d.espace(8.0);
        } else if l.trim().chars().all(|c| c == '-') {
            d.filet();
        } else if premier {
            d.titre(l);
            premier = false;
        } else {
            d.paragraphe(l);
        }
    }
    d.octets()
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod tests_police;
