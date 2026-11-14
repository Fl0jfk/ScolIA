//! Lecture de l'en-tête d'un JPEG (sans décodage) pour l'incorporer tel quel en `DCTDecode`.

use crate::PdfErreur;

#[derive(Debug, Clone)]
pub struct Jpeg {
    pub donnees: Vec<u8>,
    pub largeur: u32,
    pub hauteur: u32,
    /// 1 = niveaux de gris, 3 = couleur.
    pub composantes: u8,
}

impl Jpeg {
    pub fn lire(octets: &[u8]) -> Result<Jpeg, PdfErreur> {
        if octets.len() < 4 || octets[0] != 0xff || octets[1] != 0xd8 {
            return Err(PdfErreur::Jpeg("signature JPEG absente"));
        }
        let mut i = 2;
        while i + 4 <= octets.len() {
            if octets[i] != 0xff {
                return Err(PdfErreur::Jpeg("marqueur attendu"));
            }
            let m = octets[i + 1];
            if m == 0xff {
                i += 1; // octets de remplissage
                continue;
            }
            if m == 0x01 || (0xd0..=0xd8).contains(&m) {
                i += 2;
                continue;
            }
            if m == 0xd9 || m == 0xda {
                return Err(PdfErreur::Jpeg("aucun cadre SOF avant les données"));
            }
            let lg = usize::from(octets[i + 2]) << 8 | usize::from(octets[i + 3]);
            if lg < 2 || i + 2 + lg > octets.len() {
                return Err(PdfErreur::Jpeg("segment tronqué"));
            }
            if matches!(m, 0xc0..=0xc2) {
                let s = &octets[i + 4..i + 2 + lg];
                if s.len() < 6 {
                    return Err(PdfErreur::Jpeg("SOF trop court"));
                }
                if s[0] != 8 {
                    return Err(PdfErreur::Jpeg("précision 8 bits requise"));
                }
                let hauteur = u32::from(s[1]) << 8 | u32::from(s[2]);
                let largeur = u32::from(s[3]) << 8 | u32::from(s[4]);
                let composantes = s[5];
                if largeur == 0 || hauteur == 0 {
                    return Err(PdfErreur::Jpeg("dimensions nulles"));
                }
                if composantes != 1 && composantes != 3 {
                    return Err(PdfErreur::Jpeg("seuls le gris et le RVB sont acceptés (pas de CMJN)"));
                }
                return Ok(Jpeg { donnees: octets.to_vec(), largeur, hauteur, composantes });
            }
            if matches!(m, 0xc3 | 0xc5..=0xc7 | 0xc9..=0xcb | 0xcd..=0xcf) {
                return Err(PdfErreur::Jpeg("JPEG sans perte ou arithmétique non pris en charge"));
            }
            i += 2 + lg;
        }
        Err(PdfErreur::Jpeg("cadre SOF introuvable"))
    }
}

/// JPEG minimal valide (en-têtes seuls, sans image décodable) : pour les tests.
#[cfg(test)]
pub(crate) fn jpeg_de_test(l: u16, h: u16, comp: u8) -> Vec<u8> {
    let mut v = vec![0xff, 0xd8];
    // APP0 JFIF
    v.extend_from_slice(&[0xff, 0xe0, 0, 16, b'J', b'F', b'I', b'F', 0, 1, 1, 0, 0, 1, 0, 1, 0, 0]);
    // SOF0
    let n = 8 + 3 * u16::from(comp);
    v.extend_from_slice(&[0xff, 0xc0, (n >> 8) as u8, n as u8, 8]);
    v.extend_from_slice(&[(h >> 8) as u8, h as u8, (l >> 8) as u8, l as u8, comp]);
    for c in 0..comp {
        v.extend_from_slice(&[c + 1, 0x11, 0]);
    }
    v.extend_from_slice(&[0xff, 0xda, 0, 2, 0xff, 0xd9]);
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lit_les_dimensions() {
        let j = Jpeg::lire(&jpeg_de_test(120, 80, 3)).unwrap();
        assert_eq!((j.largeur, j.hauteur, j.composantes), (120, 80, 3));
        let g = Jpeg::lire(&jpeg_de_test(10, 20, 1)).unwrap();
        assert_eq!((g.largeur, g.hauteur, g.composantes), (10, 20, 1));
    }

    #[test]
    fn refuse_le_reste() {
        assert!(Jpeg::lire(b"GIF89a....").is_err());
        assert!(Jpeg::lire(&jpeg_de_test(10, 10, 3)[..12]).is_err());
        let mut cmjn = jpeg_de_test(10, 10, 3);
        let pos = cmjn.windows(2).position(|w| w == [0xff, 0xc0]).unwrap();
        cmjn[pos + 9] = 4;
        assert!(Jpeg::lire(&cmjn).is_err());
    }
}
