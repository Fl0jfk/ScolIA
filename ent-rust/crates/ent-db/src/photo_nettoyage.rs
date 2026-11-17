//! Socle 2, lot E9 (F2) — photos d'élèves **sans métadonnées**.
//!
//! Une photo prise au téléphone porte son EXIF : position GPS, appareil, date. Elle est servie aux
//! personnels et imprimée dans les trombinoscopes ; rien de cela n'a à quitter l'établissement.
//!
//! Le contenu est **réécrit sans métadonnées**, jamais réencodé : les pixels restent ceux de
//! l'original (pas de perte, pas de dépendance à un décodeur d'images, D-062).
//! * **JPEG** : `ent_pdf::piece::jpeg_nettoye` ne garde que les segments d'affichage. L'orientation
//!   EXIF est conservée sous la forme d'un segment APP1 **minimal** (la seule balise « orientation »,
//!   sans GPS ni aucune autre balise) : la photo s'affiche à l'endroit, sans décodage.
//! * **PNG** : seuls les blocs d'image et de rendu (`IHDR`, `PLTE`, `tRNS`, `IDAT`, `IEND`, `gAMA`,
//!   `cHRM`, `sRGB`, `sBIT`, `bKGD`, `pHYs`) sont gardés ; `eXIf`, `tEXt`, `zTXt`, `iTXt`, `tIME`,
//!   `iCCP` et tout bloc inconnu sont jetés.
//! * **WebP** : liste blanche (`VP8X`, `VP8 `, `VP8L`, `ALPH`, `ANIM`, `ANMF`) ; `EXIF`, `XMP `, `ICCP` et tout bloc inconnu sont retirés, les drapeaux du bloc `VP8X` et la
//!   taille de l'enveloppe RIFF sont corrigés.

use ent_pdf::piece::jpeg_nettoye;

const PNG_SIGNATURE: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];
const PNG_GARDES: [&[u8; 4]; 11] = [
    b"IHDR", b"PLTE", b"tRNS", b"IDAT", b"IEND", b"gAMA", b"cHRM", b"sRGB", b"sBIT", b"bKGD",
    b"pHYs",
];

/// Réécrit la photo sans métadonnées. `mime` : type déjà contrôlé par la politique de fichiers.
pub fn sans_metadonnees(mime: &str, octets: &[u8]) -> Result<Vec<u8>, &'static str> {
    match mime {
        "image/jpeg" => jpeg(octets),
        "image/png" => png(octets),
        "image/webp" => webp(octets),
        _ => Err("type d'image non pris en charge"),
    }
}

fn jpeg(octets: &[u8]) -> Result<Vec<u8>, &'static str> {
    let propre = jpeg_nettoye(octets).map_err(|_| "photo illisible")?;
    if propre.orientation == 1 {
        return Ok(propre.octets);
    }
    // APP1 : « Exif\0\0 » + TIFF (gros-boutiste) d'un seul champ, l'orientation (balise 0x0112, SHORT).
    let mut app1: Vec<u8> = vec![0xff, 0xe1, 0x00, 0x22];
    app1.extend_from_slice(b"Exif\0\0");
    app1.extend_from_slice(&[b'M', b'M', 0x00, 0x2a, 0x00, 0x00, 0x00, 0x08]); // en-tête TIFF, IFD0 à 8
    app1.extend_from_slice(&[0x00, 0x01]); // une entrée
    app1.extend_from_slice(&[
        0x01,
        0x12,
        0x00,
        0x03,
        0x00,
        0x00,
        0x00,
        0x01,
        0x00,
        propre.orientation,
        0x00,
        0x00,
    ]);
    app1.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]); // pas d'IFD suivant
    let mut sortie = Vec::with_capacity(propre.octets.len() + app1.len());
    sortie.extend_from_slice(&propre.octets[..2]); // SOI
    sortie.extend_from_slice(&app1);
    sortie.extend_from_slice(&propre.octets[2..]);
    Ok(sortie)
}

fn png(octets: &[u8]) -> Result<Vec<u8>, &'static str> {
    if octets.len() < 8 || octets[..8] != PNG_SIGNATURE {
        return Err("photo illisible");
    }
    let mut sortie = PNG_SIGNATURE.to_vec();
    let (mut i, mut vu_ihdr, mut vu_iend) = (8usize, false, false);
    while i < octets.len() {
        let lg = u32::from_be_bytes(
            octets
                .get(i..i + 4)
                .ok_or("photo illisible")?
                .try_into()
                .map_err(|_| "photo illisible")?,
        ) as usize;
        let fin = i
            .checked_add(12)
            .and_then(|x| x.checked_add(lg))
            .ok_or("photo illisible")?;
        let bloc = octets.get(i..fin).ok_or("photo illisible")?;
        let genre: &[u8; 4] = bloc[4..8].try_into().map_err(|_| "photo illisible")?;
        if i == 8 && genre != b"IHDR" {
            return Err("photo illisible");
        }
        vu_ihdr |= genre == b"IHDR";
        if PNG_GARDES.contains(&genre) {
            sortie.extend_from_slice(bloc);
        }
        if genre == b"IEND" {
            vu_iend = true;
            break; // tout ce qui suit IEND est jeté
        }
        i = fin;
    }
    if vu_ihdr && vu_iend {
        Ok(sortie)
    } else {
        Err("photo illisible")
    }
}

fn webp(octets: &[u8]) -> Result<Vec<u8>, &'static str> {
    if octets.len() < 12 || &octets[..4] != b"RIFF" || &octets[8..12] != b"WEBP" {
        return Err("photo illisible");
    }
    let declare = u32::from_le_bytes([octets[4], octets[5], octets[6], octets[7]]) as usize;
    let fin = declare.checked_add(8).ok_or("photo illisible")?;
    let corps = octets.get(12..fin).ok_or("photo illisible")?;
    let mut blocs: Vec<u8> = Vec::with_capacity(corps.len());
    let mut i = 0usize;
    let mut image = false;
    while i < corps.len() {
        let genre: [u8; 4] = corps
            .get(i..i + 4)
            .ok_or("photo illisible")?
            .try_into()
            .map_err(|_| "photo illisible")?;
        let lg = u32::from_le_bytes(
            corps
                .get(i + 4..i + 8)
                .ok_or("photo illisible")?
                .try_into()
                .map_err(|_| "photo illisible")?,
        ) as usize;
        let suivant = i.checked_add(8 + lg + (lg & 1)).ok_or("photo illisible")?;
        let bloc = corps
            .get(i..suivant.min(corps.len()))
            .ok_or("photo illisible")?;
        if suivant > corps.len() + (lg & 1) {
            return Err("photo illisible");
        }
        // Liste BLANCHE : seuls les blocs qui servent à l'affichage sont repris ; tout autre (EXIF, XMP, ICCP,
        // et tout bloc inconnu qui pourrait porter des métadonnées) disparaît.
        match &genre {
            b"VP8X" => {
                let mut v = bloc.to_vec();
                if v.len() < 9 {
                    return Err("photo illisible");
                }
                v[8] &= !(0x20 | 0x08 | 0x04); // ICC, EXIF, XMP
                blocs.extend_from_slice(&v);
            }
            b"VP8 " | b"VP8L" | b"ANMF" => {
                image = true;
                blocs.extend_from_slice(bloc);
            }
            b"ALPH" | b"ANIM" => blocs.extend_from_slice(bloc),
            _ => {}
        }
        i = suivant;
    }
    if !image {
        return Err("photo illisible");
    }
    let mut sortie = Vec::with_capacity(blocs.len() + 12);
    sortie.extend_from_slice(b"RIFF");
    sortie.extend_from_slice(
        &u32::try_from(blocs.len() + 4)
            .map_err(|_| "photo illisible")?
            .to_le_bytes(),
    );
    sortie.extend_from_slice(b"WEBP");
    sortie.extend_from_slice(&blocs);
    Ok(sortie)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn segment(marqueur: u8, corps: &[u8]) -> Vec<u8> {
        let mut v = vec![0xff, marqueur];
        v.extend_from_slice(&u16::try_from(corps.len() + 2).unwrap().to_be_bytes());
        v.extend_from_slice(corps);
        v
    }

    /// Orientation lue dans le premier APP1 EXIF (gros-boutiste), `None` sans segment.
    fn orientation(jpeg: &[u8]) -> Option<u8> {
        let p = jpeg.windows(6).position(|w| w == b"Exif\0\0")?;
        let tiff = &jpeg[p + 6..];
        (tiff[10..12] == [0x01, 0x12]).then(|| tiff[19])
    }

    /// JPEG minimal (niveaux de gris 1×1) portant un EXIF avec GPS, un commentaire et une miniature.
    fn jpeg_avec_exif(orientation: u8) -> Vec<u8> {
        let mut exif = b"Exif\0\0MM\0*\0\0\0\x08\0\x02".to_vec();
        exif.extend_from_slice(&[0x01, 0x12, 0x00, 0x03, 0, 0, 0, 1, 0, orientation, 0, 0]);
        exif.extend_from_slice(&[0x88, 0x25, 0x00, 0x04, 0, 0, 0, 1, 0, 0, 0, 0]); // pointeur GPS
        exif.extend_from_slice(b"\0\0\0\0GPSLatitude=48.8566N GPSLongitude=2.3522E");
        let mut j = vec![0xff, 0xd8];
        j.extend(segment(0xe1, &exif));
        j.extend(segment(0xfe, b"commentaire prive"));
        j.extend(segment(0xdb, &[0u8; 65]));
        j.extend(segment(0xc0, &[8, 0, 1, 0, 1, 1, 1, 0x11, 0]));
        j.extend(segment(
            0xc4,
            &[0x00, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        ));
        j.extend(segment(0xda, &[1, 1, 0, 0, 63, 0]));
        j.extend_from_slice(&[0x12, 0x34]);
        j.extend_from_slice(&[0xff, 0xd9]);
        j
    }

    #[test]
    fn jpeg_perd_le_gps_et_les_commentaires_mais_garde_l_orientation() {
        let propre = sans_metadonnees("image/jpeg", &jpeg_avec_exif(6)).unwrap();
        let texte = String::from_utf8_lossy(&propre);
        assert!(
            !texte.contains("GPS") && !texte.contains("48.8566") && !texte.contains("commentaire")
        );
        assert_eq!(&propre[..2], &[0xff, 0xd8]);
        assert_eq!(&propre[propre.len() - 2..], &[0xff, 0xd9]);
        assert_eq!(orientation(&propre), Some(6), "la photo reste à l'endroit");
        // Une seule balise : l'orientation. Le segment ne contient plus de pointeur GPS (0x8825).
        assert!(!propre.windows(2).any(|w| w == [0x88, 0x25]));
        // Le cadre et le balayage d'origine sont intacts.
        assert!(
            propre.windows(2).any(|w| w == [0xff, 0xc0])
                && propre.windows(2).any(|w| w == [0xff, 0xda])
        );
    }

    #[test]
    fn jpeg_sans_rotation_n_a_plus_aucun_exif() {
        let propre = sans_metadonnees("image/jpeg", &jpeg_avec_exif(1)).unwrap();
        assert!(!propre.windows(4).any(|w| w == b"Exif"));
        assert!(!propre.windows(3).any(|w| w == b"GPS"));
    }

    #[test]
    fn jpeg_illisible_est_refuse() {
        assert!(sans_metadonnees("image/jpeg", b"pas une image").is_err());
        assert!(sans_metadonnees("image/gif", b"GIF89a").is_err());
    }

    fn bloc_png(genre: &[u8; 4], corps: &[u8]) -> Vec<u8> {
        let mut v = u32::try_from(corps.len()).unwrap().to_be_bytes().to_vec();
        v.extend_from_slice(genre);
        v.extend_from_slice(corps);
        v.extend_from_slice(&[0, 0, 0, 0]); // CRC : repris tel quel, jamais recalculé
        v
    }

    #[test]
    fn png_perd_exif_texte_heure_et_profil_mais_garde_l_image() {
        let mut p = PNG_SIGNATURE.to_vec();
        p.extend(bloc_png(b"IHDR", &[0, 0, 0, 1, 0, 0, 0, 1, 8, 0, 0, 0, 0]));
        p.extend(bloc_png(b"eXIf", b"MM GPS 48.85"));
        p.extend(bloc_png(b"tEXt", b"Author\0Jean Dupont"));
        p.extend(bloc_png(b"tIME", &[7, 234, 11, 17, 10, 0, 0]));
        p.extend(bloc_png(b"iCCP", b"profil"));
        p.extend(bloc_png(b"IDAT", &[1, 2, 3, 4]));
        p.extend(bloc_png(b"IEND", &[]));
        p.extend_from_slice(b"octets ajoutes apres la fin");
        let propre = sans_metadonnees("image/png", &p).unwrap();
        let texte = String::from_utf8_lossy(&propre);
        for interdit in ["eXIf", "GPS", "tEXt", "Jean", "tIME", "iCCP", "ajoutes"] {
            assert!(!texte.contains(interdit), "{interdit} doit disparaître");
        }
        for garde in [b"IHDR", b"IDAT", b"IEND"] {
            assert!(propre.windows(4).any(|w| w == garde));
        }
        assert!(
            sans_metadonnees("image/png", &PNG_SIGNATURE).is_err(),
            "sans IHDR ni IEND"
        );
    }

    fn bloc_webp(genre: &[u8; 4], corps: &[u8]) -> Vec<u8> {
        let mut v = genre.to_vec();
        v.extend_from_slice(&u32::try_from(corps.len()).unwrap().to_le_bytes());
        v.extend_from_slice(corps);
        if corps.len() % 2 == 1 {
            v.push(0);
        }
        v
    }

    #[test]
    fn webp_perd_exif_xmp_et_leurs_drapeaux() {
        let mut blocs = bloc_webp(b"VP8X", &[0x3c, 0, 0, 0, 0, 0, 0, 0, 0, 0]); // ICC + EXIF + XMP + alpha
        blocs.extend(bloc_webp(b"ICCP", b"profil"));
        blocs.extend(bloc_webp(b"VP8 ", &[9, 9, 9]));
        blocs.extend(bloc_webp(b"EXIF", b"GPS 48.85"));
        blocs.extend(bloc_webp(b"XMP ", b"<x:xmpmeta/>"));
        let mut w = b"RIFF".to_vec();
        w.extend_from_slice(&u32::try_from(blocs.len() + 4).unwrap().to_le_bytes());
        w.extend_from_slice(b"WEBP");
        w.extend(blocs);
        let propre = sans_metadonnees("image/webp", &w).unwrap();
        let texte = String::from_utf8_lossy(&propre);
        for interdit in ["EXIF", "GPS", "XMP ", "xmpmeta", "ICCP"] {
            assert!(!texte.contains(interdit), "{interdit} doit disparaître");
        }
        let vp8x = propre.windows(4).position(|x| x == b"VP8X").unwrap();
        assert_eq!(
            propre[vp8x + 8] & (0x20 | 0x08 | 0x04),
            0,
            "drapeaux ICC, EXIF, XMP effacés"
        );
        assert_eq!(
            propre[vp8x + 8] & 0x10,
            0x10,
            "le drapeau alpha, lui, est conservé"
        );
        let declare = u32::from_le_bytes([propre[4], propre[5], propre[6], propre[7]]) as usize;
        assert_eq!(
            declare + 8,
            propre.len(),
            "la taille de l'enveloppe RIFF est recalculée"
        );
        assert!(
            sans_metadonnees("image/webp", b"RIFF\0\0\0\0WEBP").is_err(),
            "sans image"
        );
    }
}
