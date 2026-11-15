//! Pièces justificatives déposées par les familles (socle 7, lot D5, `07_DOCUMENTS.md` §11.4) : **photos assemblées en
//! un seul PDF** (métadonnées retirées, orientation appliquée) et **PDF désarmé**. Aucune dépendance (D-062) ; pas de
//! décodage de pixels (PDFium « non retenu pour la V1 »).
//!
//! # Photos → PDF
//! * **JPEG** : le flux est repris **sans réencodage** mais **nettoyé** : on ne garde que les segments qui servent à
//!   l'affichage (tables de quantification et de Huffman, cadre, intervalle de redémarrage, marqueur Adobe, balayages) ;
//!   *tout* le reste disparaît — EXIF (dont la **position GPS**), XMP, profil ICC, miniature JFIF, commentaires, données
//!   après la fin d'image. L'orientation EXIF est lue avant d'être jetée puis **appliquée** par la matrice de placement
//!   de la page (les 8 orientations) : la page s'affiche à l'endroit.
//! * **PNG** : les blocs `IDAT` sont repris tels quels (`FlateDecode` + prédicteur, sans décompresser), tous les blocs
//!   annexes (`tEXt`, `eXIf`, `iCCP`, `tIME`…) sont jetés. Sont refusés : l'entrelacement (Adam7) et la transparence
//!   (types 4 et 6), que PDF ne sait pas porter sans décodage.
//! * WebP, GIF, HEIC : refusés (l'appli convertit en JPEG avant l'envoi, §11.4 point 4).
//!
//! # PDF désarmé
//! Sans rastériseur, on ne peut pas « re-rendre page à page » ; on **neutralise sur place** les noms PDF qui déclenchent
//! une action ou cachent une charge — `/JavaScript`, `/JS`, `/Launch`, `/OpenAction`, `/AA`, `/URI`, `/GoToR`, `/GoToE`,
//! `/SubmitForm`, `/ImportData`, `/EmbeddedFile`, `/EF`, `/RichMedia`, `/XFA`, `/Movie`, `/Sound`, `/Rendition`… —
//! remplacés par un nom inerte **de même longueur** (la table xref reste exacte). Les noms écrits avec des échappements
//! `#xx` (`/J#61vaScript`) sont décodés avant comparaison. Les flux binaires ne sont **pas** touchés (on ne corrompt jamais
//! un contenu compressé). Sont **refusés** parce qu'on ne peut pas les inspecter : `/Encrypt`, `/ObjStm` (objets cachés
//! dans un flux compressé), PDF sans page. C'est une limite assumée (la famille est invitée à photographier ou imprimer
//! en PDF simple), consignée au document de livraison.

use crate::ecrit::Ecrivain;

/// Plus grande image acceptée (pixels) : au-delà, une bombe de décompression pour les lecteurs.
const PIXELS_MAX: u64 = 100_000_000;
/// Plus de pages qu'un dossier de pièces n'en demande.
pub const PAGES_MAX: usize = 50;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErreurPiece {
    /// Aucune image fournie, ou trop de pages.
    Pages,
    /// L'image n'est ni un JPEG ni un PNG lisible.
    ImageIllisible(&'static str),
    /// Format valide mais que la V1 ne sait pas porter (PNG transparent ou entrelacé, JPEG CMJN…).
    ImageNonPrise(&'static str),
    /// PDF qu'on ne sait pas désarmer (chiffré, objets compressés) ou illisible.
    PdfRefuse(&'static str),
}

impl std::fmt::Display for ErreurPiece {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pages => f.write_str("nombre de pages invalide"),
            Self::ImageIllisible(m) | Self::ImageNonPrise(m) | Self::PdfRefuse(m) => f.write_str(m),
        }
    }
}
impl std::error::Error for ErreurPiece {}

// ───────────────────────── JPEG : nettoyage et orientation ─────────────────────────

/// Image nettoyée et son orientation EXIF d'origine (1 à 8).
pub struct JpegNettoye {
    pub octets: Vec<u8>,
    pub largeur: u32,
    pub hauteur: u32,
    pub composantes: u8,
    pub orientation: u8,
}

fn u16_be(b: &[u8], o: usize) -> Option<u16> {
    Some(u16::from_be_bytes([*b.get(o)?, *b.get(o + 1)?]))
}

/// Lit l'orientation dans un segment APP1 EXIF (`Exif\0\0` + TIFF). 1 par défaut ; valeur hors 1..=8 ignorée.
fn orientation_exif(app1: &[u8]) -> u8 {
    let Some(tiff) = app1.strip_prefix(b"Exif\0\0") else { return 1 };
    let (le, ok) = match tiff.get(0..2) {
        Some(b"II") => (true, true),
        Some(b"MM") => (false, true),
        _ => (false, false),
    };
    if !ok {
        return 1;
    }
    let u16_at = |o: usize| -> Option<u16> {
        let b = [*tiff.get(o)?, *tiff.get(o + 1)?];
        Some(if le { u16::from_le_bytes(b) } else { u16::from_be_bytes(b) })
    };
    let u32_at = |o: usize| -> Option<u32> {
        let b = [*tiff.get(o)?, *tiff.get(o + 1)?, *tiff.get(o + 2)?, *tiff.get(o + 3)?];
        Some(if le { u32::from_le_bytes(b) } else { u32::from_be_bytes(b) })
    };
    if u16_at(2) != Some(42) {
        return 1;
    }
    let Some(ifd) = u32_at(4).map(|x| x as usize) else { return 1 };
    let Some(n) = u16_at(ifd) else { return 1 };
    for i in 0..usize::from(n.min(64)) {
        let e = ifd + 2 + i * 12;
        if u16_at(e) == Some(0x0112) && u16_at(e + 2) == Some(3) {
            return match u16_at(e + 8) {
                Some(v @ 1..=8) => v as u8,
                _ => 1,
            };
        }
    }
    1
}

/// Garde l'affichage, jette tout le reste. Voir le module.
pub fn jpeg_nettoye(entree: &[u8]) -> Result<JpegNettoye, ErreurPiece> {
    let il = ErreurPiece::ImageIllisible;
    if entree.len() < 4 || entree[0] != 0xff || entree[1] != 0xd8 {
        return Err(il("ce n'est pas un JPEG"));
    }
    let mut sortie = vec![0xff, 0xd8];
    let mut orientation = 1u8;
    let mut i = 2usize;
    let mut cadre = None;
    let mut balayage = false;
    loop {
        // Marqueur : 0xFF, éventuellement précédé d'octets de remplissage 0xFF.
        if *entree.get(i).ok_or(il("fichier tronqué"))? != 0xff {
            return Err(il("marqueur attendu"));
        }
        while entree.get(i + 1) == Some(&0xff) {
            i += 1;
        }
        let m = *entree.get(i + 1).ok_or(il("fichier tronqué"))?;
        match m {
            0xd9 => {
                // Fin d'image : on s'arrête ; tout ce qui suit (miniatures, données ajoutées) est jeté.
                if !balayage {
                    return Err(il("aucune donnée d'image"));
                }
                sortie.extend_from_slice(&[0xff, 0xd9]);
                break;
            }
            0x01 | 0xd0..=0xd8 => {
                // Marqueurs sans longueur hors flux : ne devraient pas apparaître ici.
                i += 2;
                continue;
            }
            _ => {}
        }
        let lg = usize::from(u16_be(entree, i + 2).ok_or(il("segment tronqué"))?);
        if lg < 2 || i + 2 + lg > entree.len() {
            return Err(il("segment tronqué"));
        }
        let segment = &entree[i..i + 2 + lg];
        let corps = &entree[i + 4..i + 2 + lg];
        match m {
            // Cadres : seuls le baseline et le progressif Huffman 8 bits sont pris en charge.
            0xc0..=0xc2 => {
                if corps.len() < 6 || corps[0] != 8 {
                    return Err(il("précision 8 bits requise"));
                }
                let h = u32::from(u16::from_be_bytes([corps[1], corps[2]]));
                let l = u32::from(u16::from_be_bytes([corps[3], corps[4]]));
                let c = corps[5];
                if l == 0 || h == 0 {
                    return Err(il("dimensions nulles"));
                }
                if u64::from(l) * u64::from(h) > PIXELS_MAX {
                    return Err(ErreurPiece::ImageNonPrise("image démesurée"));
                }
                if c != 1 && c != 3 {
                    return Err(ErreurPiece::ImageNonPrise("seuls le gris et le RVB sont acceptés (pas de CMJN)"));
                }
                cadre = Some((l, h, c));
                sortie.extend_from_slice(segment);
            }
            0xc3 | 0xc5..=0xc7 | 0xc9..=0xcb | 0xcd..=0xcf => {
                return Err(ErreurPiece::ImageNonPrise("JPEG sans perte ou arithmétique non pris en charge"));
            }
            // Tables, intervalle de redémarrage, marqueur Adobe (transformation de couleur) : nécessaires.
            0xc4 | 0xdb | 0xdd | 0xee => sortie.extend_from_slice(segment),
            // EXIF : on lit l'orientation, on jette le segment (position GPS comprise).
            0xe1 => {
                if corps.starts_with(b"Exif\0\0") {
                    orientation = orientation_exif(corps);
                }
            }
            0xda => {
                // Balayage : en-tête puis données entropiques jusqu'au prochain vrai marqueur.
                sortie.extend_from_slice(segment);
                let mut j = i + 2 + lg;
                let debut = j;
                loop {
                    let b = *entree.get(j).ok_or(il("fichier tronqué"))?;
                    if b == 0xff {
                        let s = *entree.get(j + 1).ok_or(il("fichier tronqué"))?;
                        if s == 0x00 || (0xd0..=0xd7).contains(&s) {
                            j += 2;
                            continue;
                        }
                        if s == 0xff {
                            j += 1;
                            continue;
                        }
                        break;
                    }
                    j += 1;
                }
                if j == debut {
                    return Err(il("balayage vide"));
                }
                sortie.extend_from_slice(&entree[debut..j]);
                balayage = true;
                i = j;
                continue;
            }
            // APPn, commentaires, extensions : jetés.
            _ => {}
        }
        i += 2 + lg;
    }
    let (largeur, hauteur, composantes) = cadre.ok_or(il("cadre d'image absent"))?;
    Ok(JpegNettoye { octets: sortie, largeur, hauteur, composantes, orientation })
}

/// Matrice de placement `[a b c d e f]` de l'image dans la page, pour l'orientation EXIF et les dimensions affichées
/// `(pw, ph)` en points. `wi`/`hi` : dimensions de l'image **telle que stockée**, mises à l'échelle.
fn matrice(orientation: u8, wi: f32, hi: f32) -> [f32; 6] {
    match orientation {
        2 => [-wi, 0.0, 0.0, hi, wi, 0.0],
        3 => [-wi, 0.0, 0.0, -hi, wi, hi],
        4 => [wi, 0.0, 0.0, -hi, 0.0, hi],
        5 => [0.0, -wi, -hi, 0.0, hi, wi],
        6 => [0.0, -wi, hi, 0.0, 0.0, wi],
        7 => [0.0, wi, hi, 0.0, 0.0, 0.0],
        8 => [0.0, wi, -hi, 0.0, hi, 0.0],
        _ => [wi, 0.0, 0.0, hi, 0.0, 0.0],
    }
}

// ───────────────────────── PNG ─────────────────────────

fn crc32(octets: &[u8]) -> u32 {
    let mut c = 0xFFFF_FFFFu32;
    for &b in octets {
        c ^= u32::from(b);
        for _ in 0..8 {
            c = if c & 1 == 1 { 0xEDB8_8320 ^ (c >> 1) } else { c >> 1 };
        }
    }
    !c
}

struct Png {
    largeur: u32,
    hauteur: u32,
    profondeur: u8,
    /// 0 gris, 2 RVB, 3 palette.
    couleur: u8,
    palette: Vec<u8>,
    idat: Vec<u8>,
}

fn lire_png(b: &[u8]) -> Result<Png, ErreurPiece> {
    let il = ErreurPiece::ImageIllisible;
    if b.len() < 8 || b[..8] != [0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a] {
        return Err(il("ce n'est pas un PNG"));
    }
    let mut i = 8usize;
    let mut ihdr: Option<(u32, u32, u8, u8, u8)> = None;
    let mut palette = Vec::new();
    let mut idat = Vec::new();
    let mut fin = false;
    while i + 12 <= b.len() {
        let n = u32::from_be_bytes([b[i], b[i + 1], b[i + 2], b[i + 3]]) as usize;
        let t = &b[i + 4..i + 8];
        let Some(donnees) = b.get(i + 8..i + 8 + n) else { return Err(il("bloc PNG tronqué")) };
        let Some(crc) = b.get(i + 8 + n..i + 12 + n) else { return Err(il("bloc PNG tronqué")) };
        if u32::from_be_bytes([crc[0], crc[1], crc[2], crc[3]]) != crc32(&b[i + 4..i + 8 + n]) {
            return Err(il("PNG corrompu (somme de contrôle)"));
        }
        match t {
            b"IHDR" => {
                if n != 13 {
                    return Err(il("IHDR invalide"));
                }
                let l = u32::from_be_bytes([donnees[0], donnees[1], donnees[2], donnees[3]]);
                let h = u32::from_be_bytes([donnees[4], donnees[5], donnees[6], donnees[7]]);
                ihdr = Some((l, h, donnees[8], donnees[9], donnees[12]));
                if donnees[10] != 0 || donnees[11] != 0 {
                    return Err(il("méthode de compression PNG inconnue"));
                }
            }
            b"PLTE" => palette = donnees.to_vec(),
            b"IDAT" => idat.extend_from_slice(donnees),
            b"IEND" => {
                fin = true;
                break;
            }
            _ => {} // blocs annexes (tEXt, eXIf, iCCP, tIME…) : jetés
        }
        i += 12 + n;
    }
    let (largeur, hauteur, profondeur, couleur, entrelace) = ihdr.ok_or(il("IHDR absent"))?;
    if !fin || idat.is_empty() {
        return Err(il("PNG incomplet"));
    }
    if largeur == 0 || hauteur == 0 {
        return Err(il("dimensions nulles"));
    }
    if u64::from(largeur) * u64::from(hauteur) > PIXELS_MAX {
        return Err(ErreurPiece::ImageNonPrise("image démesurée"));
    }
    if entrelace != 0 {
        return Err(ErreurPiece::ImageNonPrise("PNG entrelacé non pris en charge : enregistrez-le sans entrelacement"));
    }
    if couleur == 4 || couleur == 6 {
        return Err(ErreurPiece::ImageNonPrise("PNG avec transparence non pris en charge : prenez une photo ou exportez en JPEG"));
    }
    let valide = match couleur {
        0 => matches!(profondeur, 1 | 2 | 4 | 8),
        2 => profondeur == 8,
        3 => matches!(profondeur, 1 | 2 | 4 | 8) && !palette.is_empty() && palette.len() % 3 == 0 && palette.len() <= 768,
        _ => false,
    };
    if !valide {
        return Err(ErreurPiece::ImageNonPrise("combinaison de profondeur et de couleur PNG non prise en charge"));
    }
    Ok(Png { largeur, hauteur, profondeur, couleur, palette, idat })
}

// ───────────────────────── Assemblage ─────────────────────────

/// Un PDF fait de pages d'images, déterministe (aucune date), sans autre métadonnée que le producteur.
#[derive(Debug, Clone)]
pub struct PieceNormalisee {
    pub pdf: Vec<u8>,
    pub nb_pages: usize,
    /// Nombre de pages dont l'orientation EXIF n'était pas « normale » et a été appliquée.
    pub pages_redressees: usize,
}

fn f(x: f32) -> String {
    let s = format!("{x:.3}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s.is_empty() || s == "-" { "0".into() } else { s.into() }
}

/// Assemble des images (JPEG ou PNG, dans l'ordre des pages) en **un seul PDF**.
pub fn images_en_pdf(images: &[&[u8]]) -> Result<PieceNormalisee, ErreurPiece> {
    if images.is_empty() || images.len() > PAGES_MAX {
        return Err(ErreurPiece::Pages);
    }
    let mut e = Ecrivain::nouveau();
    // Objets : 1 = catalogue, 2 = pages (écrits à la fin faute de connaître les numéros) ; on réserve en écrivant d'abord
    // les images et pages, puis la racine. L'écrivain numérote dans l'ordre d'écriture.
    let mut pages_ids = Vec::new();
    let mut redressees = 0;
    // La racine « Pages » aura le numéro suivant les 3 objets par page : on le calcule d'avance.
    let n = images.len();
    let id_pages = 3 * n + 1;
    for img in images {
        let est_png = img.starts_with(&[0x89, b'P', b'N', b'G']);
        let (dict_image, donnees, wpx, hpx, orientation): (String, Vec<u8>, u32, u32, u8) = if est_png {
            let p = lire_png(img)?;
            let cs = match p.couleur {
                0 => "/DeviceGray".to_string(),
                2 => "/DeviceRGB".to_string(),
                _ => {
                    let mut h = String::new();
                    for o in &p.palette {
                        h.push_str(&format!("{o:02X}"));
                    }
                    format!("[/Indexed /DeviceRGB {} <{h}>]", p.palette.len() / 3 - 1)
                }
            };
            let colors = if p.couleur == 2 { 3 } else { 1 };
            (
                format!(
                    "/Type /XObject /Subtype /Image /Width {} /Height {} /ColorSpace {cs} /BitsPerComponent {} /Filter /FlateDecode /DecodeParms << /Predictor 15 /Colors {colors} /BitsPerComponent {} /Columns {} >>",
                    p.largeur, p.hauteur, p.profondeur, p.profondeur, p.largeur
                ),
                p.idat,
                p.largeur,
                p.hauteur,
                1,
            )
        } else {
            let j = jpeg_nettoye(img)?;
            let cs = if j.composantes == 1 { "/DeviceGray" } else { "/DeviceRGB" };
            (
                format!(
                    "/Type /XObject /Subtype /Image /Width {} /Height {} /ColorSpace {cs} /BitsPerComponent 8 /Filter /DCTDecode",
                    j.largeur, j.hauteur
                ),
                j.octets,
                j.largeur,
                j.hauteur,
                j.orientation,
            )
        };
        if orientation != 1 {
            redressees += 1;
        }
        // Dimensions affichées (après orientation), mises à l'échelle pour tenir dans un A4 (595 × 842) sans agrandir.
        let (dw, dh) = if orientation >= 5 { (hpx, wpx) } else { (wpx, hpx) };
        let (boite_l, boite_h) = if dw > dh { (842.0f32, 595.0f32) } else { (595.0, 842.0) };
        let echelle = (boite_l / dw as f32).min(boite_h / dh as f32).min(1.0);
        let (wi, hi) = (wpx as f32 * echelle, hpx as f32 * echelle);
        let (pw, ph) = (dw as f32 * echelle, dh as f32 * echelle);
        let m = matrice(orientation, wi, hi);
        let id_image = e.flux(&dict_image, &donnees);
        let contenu = format!(
            "q {} {} {} {} {} {} cm /Im1 Do Q",
            f(m[0]), f(m[1]), f(m[2]), f(m[3]), f(m[4]), f(m[5])
        );
        let id_contenu = e.flux("", contenu.as_bytes());
        let id_page = e.objet(
            format!(
                "<< /Type /Page /Parent {id_pages} 0 R /MediaBox [0 0 {} {}] /Resources << /XObject << /Im1 {id_image} 0 R >> >> /Contents {id_contenu} 0 R >>",
                f(pw), f(ph)
            )
            .as_bytes(),
        );
        pages_ids.push(id_page);
    }
    let kids: String = pages_ids.iter().map(|p| format!("{p} 0 R ")).collect();
    let pages = e.objet(format!("<< /Type /Pages /Kids [{kids}] /Count {n} >>").as_bytes());
    debug_assert_eq!(pages, id_pages);
    let racine = e.objet(format!("<< /Type /Catalog /Pages {pages} 0 R >>").as_bytes());
    let info = e.objet(b"<< /Producer (ScolIA) >>");
    Ok(PieceNormalisee { pdf: e.terminer(racine, info), nb_pages: n, pages_redressees: redressees })
}

// ───────────────────────── PDF : désarmement ─────────────────────────

/// Noms PDF qui déclenchent une action, lancent un programme, joignent un fichier ou portent un contenu actif.
const NOMS_ACTIFS: &[&str] = &[
    "JavaScript", "JS", "Launch", "OpenAction", "AA", "URI", "GoToR", "GoToE", "SubmitForm", "ImportData",
    "ResetForm", "EmbeddedFile", "EmbeddedFiles", "EF", "RF", "RichMedia", "XFA", "Movie", "Sound", "Rendition",
    "Screen", "SetOCGState", "Hide", "GoTo3DView", "FileAttachment", "Named",
];

fn delimiteur(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\r' | b'\n' | 0x0c | 0 | b'(' | b')' | b'<' | b'>' | b'[' | b']' | b'{' | b'}' | b'/' | b'%')
}

/// Décode les `#xx` d'un nom (les octets non UTF-8 deviennent des remplacements : ils ne peuvent pas égaler un nom actif).
fn nom_decode(brut: &[u8]) -> String {
    let mut o = Vec::with_capacity(brut.len());
    let mut i = 0;
    while i < brut.len() {
        if brut[i] == b'#' && i + 2 < brut.len() + 0 && i + 2 <= brut.len() - 1 + 0 {
            let h = std::str::from_utf8(&brut[i + 1..i + 3]).ok().and_then(|x| u8::from_str_radix(x, 16).ok());
            if let Some(v) = h {
                o.push(v);
                i += 3;
                continue;
            }
        }
        o.push(brut[i]);
        i += 1;
    }
    String::from_utf8_lossy(&o).into_owned()
}

/// Régions « flux binaire » : de `stream` + fin de ligne jusqu'à `endstream`. On n'y touche jamais.
fn regions_flux(b: &[u8]) -> Vec<(usize, usize)> {
    let mut r = Vec::new();
    let mut i = 0;
    while let Some(p) = trouver(b, b"stream", i) {
        // `endstream` contient `stream` : on ne démarre pas une région sur lui.
        if p >= 3 && &b[p - 3..p] == b"end" {
            i = p + 6;
            continue;
        }
        let mut d = p + 6;
        if b.get(d) == Some(&b'\r') {
            d += 1;
        }
        if b.get(d) == Some(&b'\n') {
            d += 1;
        } else if d == p + 6 {
            // `stream` non suivi d'une fin de ligne : un mot de texte, pas un flux.
            i = p + 6;
            continue;
        }
        let fin = trouver(b, b"endstream", d).unwrap_or(b.len());
        r.push((d, fin));
        i = fin;
    }
    r
}

fn trouver(b: &[u8], aiguille: &[u8], depuis: usize) -> Option<usize> {
    if depuis >= b.len() {
        return None;
    }
    b[depuis..].windows(aiguille.len()).position(|w| w == aiguille).map(|p| p + depuis)
}

/// Les jetons `/Nom` hors flux, avec leur position : `(début, fin, nom décodé)`.
fn noms_hors_flux(b: &[u8]) -> Vec<(usize, usize, String)> {
    let flux = regions_flux(b);
    let mut sortie = Vec::new();
    let mut i = 0;
    let mut r = 0;
    while i < b.len() {
        while r < flux.len() && flux[r].1 <= i {
            r += 1;
        }
        if r < flux.len() && i >= flux[r].0 {
            i = flux[r].1;
            continue;
        }
        if b[i] == b'(' {
            // Chaîne littérale : on saute (parenthèses imbriquées et échappements).
            let mut prof = 0i32;
            while i < b.len() {
                match b[i] {
                    b'\\' => i += 1,
                    b'(' => prof += 1,
                    b')' => {
                        prof -= 1;
                        if prof == 0 {
                            break;
                        }
                    }
                    _ => {}
                }
                i += 1;
            }
            i += 1;
            continue;
        }
        if b[i] == b'/' {
            let mut j = i + 1;
            while j < b.len() && !delimiteur(b[j]) {
                j += 1;
            }
            sortie.push((i, j, nom_decode(&b[i + 1..j])));
            i = j;
            continue;
        }
        i += 1;
    }
    sortie
}

/// PDF désarmé, noms neutralisés (sans doublon, ordre d'apparition) et nombre de pages.
#[derive(Debug, Clone)]
pub struct PdfDesarme {
    pub octets: Vec<u8>,
    pub neutralises: Vec<String>,
    pub nb_pages: usize,
}

/// Désarme un PDF entrant. Voir le module. Idempotent : désarmer un PDF désarmé ne neutralise plus rien.
pub fn desarmer_pdf(entree: &[u8]) -> Result<PdfDesarme, ErreurPiece> {
    let refus = ErreurPiece::PdfRefuse;
    let tete = &entree[..entree.len().min(1024)];
    if trouver(tete, b"%PDF-", 0).is_none() {
        return Err(refus("ce n'est pas un PDF"));
    }
    let queue = &entree[entree.len().saturating_sub(2048)..];
    if trouver(queue, b"%%EOF", 0).is_none() || trouver(entree, b"endobj", 0).is_none() {
        return Err(refus("PDF incomplet ou illisible"));
    }
    let noms = noms_hors_flux(entree);
    if noms.iter().any(|(_, _, n)| n == "Encrypt") {
        return Err(refus("PDF chiffré : enregistrez-le sans mot de passe, ou prenez une photo"));
    }
    if noms.iter().any(|(_, _, n)| n == "ObjStm") {
        return Err(refus("PDF à objets compressés non inspectable : prenez une photo ou imprimez-le en PDF simple"));
    }
    let mut sortie = entree.to_vec();
    let mut neutralises: Vec<String> = Vec::new();
    for (d, fin, nom) in &noms {
        if NOMS_ACTIFS.contains(&nom.as_str()) {
            sortie[*d + 1..*fin].fill(b'x');
            if !neutralises.contains(nom) {
                neutralises.push(nom.clone());
            }
        }
    }
    // Pages : `/Type /Page` (jamais `/Pages`).
    let mut nb_pages = 0;
    for w in noms.windows(2) {
        if w[0].2 == "Type" && w[1].2 == "Page" {
            nb_pages += 1;
        }
    }
    if nb_pages == 0 {
        return Err(refus("PDF sans page"));
    }
    // Contrôle de sortie : rien d'actif ne subsiste.
    debug_assert!(noms_hors_flux(&sortie).iter().all(|(_, _, n)| !NOMS_ACTIFS.contains(&n.as_str())));
    Ok(PdfDesarme { octets: sortie, neutralises, nb_pages })
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── JPEG synthétiques ──

    fn segment(m: u8, corps: &[u8]) -> Vec<u8> {
        let mut v = vec![0xff, m];
        v.extend_from_slice(&((corps.len() + 2) as u16).to_be_bytes());
        v.extend_from_slice(corps);
        v
    }

    fn exif(orientation: u16, gps: &str) -> Vec<u8> {
        // TIFF petit-boutiste : IFD0 avec l'orientation, puis une « zone GPS » factice qui doit disparaître.
        let mut t = b"II*\0".to_vec();
        t.extend_from_slice(&8u32.to_le_bytes());
        t.extend_from_slice(&1u16.to_le_bytes());
        t.extend_from_slice(&0x0112u16.to_le_bytes());
        t.extend_from_slice(&3u16.to_le_bytes());
        t.extend_from_slice(&1u32.to_le_bytes());
        t.extend_from_slice(&orientation.to_le_bytes());
        t.extend_from_slice(&[0, 0]);
        t.extend_from_slice(&0u32.to_le_bytes());
        t.extend_from_slice(gps.as_bytes());
        let mut corps = b"Exif\0\0".to_vec();
        corps.extend_from_slice(&t);
        segment(0xe1, &corps)
    }

    fn jpeg(l: u16, h: u16, comp: u8, avec: &[Vec<u8>], fin: &[u8]) -> Vec<u8> {
        let mut v = vec![0xff, 0xd8];
        v.extend_from_slice(&segment(0xe0, b"JFIF\0\x01\x01\0\0\x01\0\x01\0\0"));
        for a in avec {
            v.extend_from_slice(a);
        }
        v.extend_from_slice(&segment(0xdb, &[0u8; 65]));
        let mut sof = vec![8, (h >> 8) as u8, h as u8, (l >> 8) as u8, l as u8, comp];
        for c in 0..comp {
            sof.extend_from_slice(&[c + 1, 0x11, 0]);
        }
        v.extend_from_slice(&segment(0xc0, &sof));
        v.extend_from_slice(&segment(0xc4, &[0u8; 20]));
        let mut sos = vec![comp];
        for c in 0..comp {
            sos.extend_from_slice(&[c + 1, 0]);
        }
        sos.extend_from_slice(&[0, 63, 0]);
        v.extend_from_slice(&segment(0xda, &sos));
        // Données entropiques avec un 0xFF 0x00 (octet bourré) et un redémarrage.
        v.extend_from_slice(&[0x12, 0x34, 0xff, 0x00, 0x56, 0xff, 0xd0, 0x78]);
        v.extend_from_slice(&[0xff, 0xd9]);
        v.extend_from_slice(fin);
        v
    }

    #[test]
    fn exif_et_gps_disparaissent_orientation_lue() {
        let src = jpeg(
            400,
            300,
            3,
            &[exif(6, "GPS:48.8566N,2.3522E-SECRET"), segment(0xfe, b"commentaire-secret"), segment(0xe2, b"ICC_PROFILE-secret")],
            b"donnees ajoutees apres la fin SECRET2",
        );
        assert!(src.windows(6).any(|w| w == b"SECRET"));
        let j = jpeg_nettoye(&src).unwrap();
        assert_eq!((j.largeur, j.hauteur, j.composantes, j.orientation), (400, 300, 3, 6));
        for interdit in [&b"GPS"[..], b"SECRET", b"Exif", b"JFIF", b"ICC_PROFILE", b"commentaire", b"donnees ajoutees"] {
            assert!(!j.octets.windows(interdit.len()).any(|w| w == interdit), "{:?}", String::from_utf8_lossy(interdit));
        }
        // Le flux nettoyé reste un JPEG complet (SOI … EOI) avec ses données entropiques intactes.
        assert!(j.octets.starts_with(&[0xff, 0xd8]) && j.octets.ends_with(&[0xff, 0xd9]));
        assert!(j.octets.windows(8).any(|w| w == [0x12, 0x34, 0xff, 0x00, 0x56, 0xff, 0xd0, 0x78]));
        // Le résultat se relit avec le lecteur de JPEG du moteur.
        assert!(crate::Jpeg::lire(&j.octets).is_ok());
    }

    #[test]
    fn jpeg_invalides_refuses() {
        assert!(matches!(jpeg_nettoye(b"GIF89a...."), Err(ErreurPiece::ImageIllisible(_))));
        let ok = jpeg(10, 10, 3, &[], b"");
        assert!(jpeg_nettoye(&ok[..ok.len() - 12]).is_err(), "tronqué");
        assert!(matches!(jpeg_nettoye(&jpeg(10, 10, 4, &[], b"")), Err(ErreurPiece::ImageNonPrise(_))), "CMJN");
        assert!(matches!(jpeg_nettoye(&jpeg(0, 10, 3, &[], b"")), Err(ErreurPiece::ImageIllisible(_))));
        // Bombe : 20 000 × 20 000 pixels.
        assert!(matches!(jpeg_nettoye(&jpeg(20000, 20000, 3, &[], b"")), Err(ErreurPiece::ImageNonPrise(_))));
    }

    /// Chaque orientation place les quatre coins de l'image stockée où l'EXIF le dit.
    #[test]
    fn les_huit_orientations() {
        // Coins de l'image stockée (unité PDF : (0,0) bas-gauche, y vers le haut) : HG, HD, BG, BD.
        let (wi, hi) = (4.0f32, 2.0f32);
        let coins = [(0.0, 1.0), (1.0, 1.0), (0.0, 0.0), (1.0, 0.0)];
        // Position attendue de chaque coin dans la page, pour (largeur, hauteur) affichées.
        let attendu = |o: u8| -> [(f32, f32); 4] {
            let (pw, ph) = if o >= 5 { (hi, wi) } else { (wi, hi) };
            match o {
                1 => [(0.0, ph), (pw, ph), (0.0, 0.0), (pw, 0.0)],
                2 => [(pw, ph), (0.0, ph), (pw, 0.0), (0.0, 0.0)],
                3 => [(pw, 0.0), (0.0, 0.0), (pw, ph), (0.0, ph)],
                4 => [(0.0, 0.0), (pw, 0.0), (0.0, ph), (pw, ph)],
                5 => [(0.0, ph), (0.0, 0.0), (pw, ph), (pw, 0.0)],
                6 => [(pw, ph), (pw, 0.0), (0.0, ph), (0.0, 0.0)],
                7 => [(pw, 0.0), (pw, ph), (0.0, 0.0), (0.0, ph)],
                8 => [(0.0, 0.0), (0.0, ph), (pw, 0.0), (pw, ph)],
                _ => unreachable!(),
            }
        };
        for o in 1..=8u8 {
            let m = matrice(o, wi, hi);
            for (k, (x, y)) in coins.iter().enumerate() {
                let (px, py) = (m[0] * x + m[2] * y + m[4], m[1] * x + m[3] * y + m[5]);
                let (ax, ay) = attendu(o)[k];
                assert!((px - ax).abs() < 1e-4 && (py - ay).abs() < 1e-4, "orientation {o}, coin {k} : ({px},{py}) ≠ ({ax},{ay})");
            }
        }
    }

    #[test]
    fn photos_en_un_seul_pdf() {
        let a = jpeg(400, 300, 3, &[exif(6, "GPS:SECRET-A")], b"");
        let b = jpeg(200, 300, 1, &[exif(1, "GPS:SECRET-B")], b"");
        let p = images_en_pdf(&[&a, &b]).unwrap();
        assert_eq!((p.nb_pages, p.pages_redressees), (2, 1));
        assert!(p.pdf.starts_with(b"%PDF-1.4"));
        let d = desarmer_pdf(&p.pdf).unwrap();
        assert_eq!(d.nb_pages, 2, "relu par le désarmeur");
        assert!(d.neutralises.is_empty(), "un PDF fait par nous n'a rien d'actif");
        assert!(!p.pdf.windows(6).any(|w| w == b"SECRET") && !p.pdf.windows(4).any(|w| w == b"Exif"));
        assert!(p.pdf.windows(11).any(|w| w == b"/DCTDecode "));
        // Déterministe : mêmes entrées, mêmes octets.
        assert_eq!(p.pdf, images_en_pdf(&[&a, &b]).unwrap().pdf);
        // Page de l'image tournée : 300 × 400 points (largeur et hauteur échangées), sans agrandissement.
        assert!(String::from_utf8_lossy(&p.pdf).contains("/MediaBox [0 0 300 400]"));
        assert!(matches!(images_en_pdf(&[]), Err(ErreurPiece::Pages)));
        let trop: Vec<&[u8]> = (0..=PAGES_MAX).map(|_| &a[..]).collect();
        assert!(matches!(images_en_pdf(&trop), Err(ErreurPiece::Pages)));
    }

    // ── PNG synthétiques ──

    fn bloc(t: &[u8; 4], d: &[u8]) -> Vec<u8> {
        let mut v = (d.len() as u32).to_be_bytes().to_vec();
        let mut c = t.to_vec();
        c.extend_from_slice(d);
        v.extend_from_slice(&c);
        v.extend_from_slice(&crc32(&c).to_be_bytes());
        v
    }

    fn png(l: u32, h: u32, prof: u8, couleur: u8, entrelace: u8, extra: &[Vec<u8>]) -> Vec<u8> {
        let mut v = vec![0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];
        let mut ihdr = l.to_be_bytes().to_vec();
        ihdr.extend_from_slice(&h.to_be_bytes());
        ihdr.extend_from_slice(&[prof, couleur, 0, 0, entrelace]);
        v.extend_from_slice(&bloc(b"IHDR", &ihdr));
        for e in extra {
            v.extend_from_slice(e);
        }
        v.extend_from_slice(&bloc(b"IDAT", &[0x78, 0x01, 0x01, 0x00, 0x00, 0xff, 0xff, 0, 0, 0, 1]));
        v.extend_from_slice(&bloc(b"IEND", &[]));
        v
    }

    #[test]
    fn png_repris_sans_blocs_annexes() {
        let src = png(30, 20, 8, 2, 0, &[bloc(b"tEXt", b"Comment\0SECRET-PNG"), bloc(b"eXIf", b"GPS-SECRET")]);
        let p = images_en_pdf(&[&src]).unwrap();
        assert!(!p.pdf.windows(6).any(|w| w == b"SECRET"));
        let t = String::from_utf8_lossy(&p.pdf);
        assert!(t.contains("/FlateDecode") && t.contains("/Predictor 15 /Colors 3") && t.contains("/Columns 30"));
        // Palette.
        let pal = png(4, 4, 8, 3, 0, &[bloc(b"PLTE", &[255, 0, 0, 0, 255, 0])]);
        assert!(String::from_utf8_lossy(&images_en_pdf(&[&pal]).unwrap().pdf).contains("[/Indexed /DeviceRGB 1 <FF000000FF00>]"));
    }

    #[test]
    fn png_non_pris_en_charge() {
        assert!(matches!(images_en_pdf(&[&png(4, 4, 8, 6, 0, &[])]), Err(ErreurPiece::ImageNonPrise(_))), "RVBA");
        assert!(matches!(images_en_pdf(&[&png(4, 4, 8, 4, 0, &[])]), Err(ErreurPiece::ImageNonPrise(_))), "gris+alpha");
        assert!(matches!(images_en_pdf(&[&png(4, 4, 8, 2, 1, &[])]), Err(ErreurPiece::ImageNonPrise(_))), "entrelacé");
        assert!(matches!(images_en_pdf(&[&png(4, 4, 16, 2, 0, &[])]), Err(ErreurPiece::ImageNonPrise(_))), "16 bits");
        assert!(matches!(images_en_pdf(&[&png(4, 4, 8, 3, 0, &[])]), Err(ErreurPiece::ImageNonPrise(_))), "palette absente");
        let mut corrompu = png(4, 4, 8, 2, 0, &[]);
        corrompu[20] ^= 0xff;
        assert!(matches!(images_en_pdf(&[&corrompu]), Err(ErreurPiece::ImageIllisible(_))), "somme de contrôle");
        assert!(matches!(images_en_pdf(&[b"RIFF....WEBPVP8 "]), Err(ErreurPiece::ImageIllisible(_))), "WebP");
        assert!(matches!(images_en_pdf(&[b"GIF89a"]), Err(ErreurPiece::ImageIllisible(_))));
    }

    // ── PDF ──

    fn pdf_piege() -> Vec<u8> {
        let mut v = b"%PDF-1.4\n".to_vec();
        v.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R /OpenAction 5 0 R /AA << /O 5 0 R >> /Names << /JavaScript 6 0 R >> >>\nendobj\n");
        v.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");
        v.extend_from_slice(b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Annots [7 0 R] >>\nendobj\n");
        v.extend_from_slice(b"5 0 obj\n<< /S /JavaScript /JS (app.alert\\('piege'\\);) >>\nendobj\n");
        v.extend_from_slice(b"6 0 obj\n<< /S /#4Aava#53cript /J#53 (this.exportDataObject\\(\\)) >>\nendobj\n");
        v.extend_from_slice(b"7 0 obj\n<< /Type /Annot /Subtype /Link /A << /S /URI /URI (http://evil.example/) >> /A2 << /S /Launch /F (cmd.exe) >> >>\nendobj\n");
        v.extend_from_slice(b"8 0 obj\n<< /Type /EmbeddedFile /Length 11 >>\nstream\n/JavaScript!\nendstream\nendobj\n");
        v.extend_from_slice(b"trailer\n<< /Root 1 0 R >>\nstartxref\n0\n%%EOF\n");
        v
    }

    #[test]
    fn pdf_avec_javascript_est_desarme() {
        let src = pdf_piege();
        let d = desarmer_pdf(&src).unwrap();
        assert_eq!(d.nb_pages, 1);
        for nom in ["JavaScript", "JS", "OpenAction", "AA", "URI", "Launch", "EmbeddedFile"] {
            assert!(d.neutralises.iter().any(|n| n == nom), "{nom} neutralisé : {:?}", d.neutralises);
        }
        // Rien d'actif ne subsiste, même écrit avec des échappements `#xx`.
        let apres = String::from_utf8_lossy(&d.octets).to_string();
        // (Hors du flux binaire, que l'on ne touche jamais.)
        let hors_flux = apres.replace("stream\n/JavaScript!\nendstream", "");
        for interdit in ["/JavaScript", "/JS ", "/OpenAction", "/AA ", "/URI", "/Launch", "/EmbeddedFile", "/#4Aava", "/J#53"] {
            assert!(!hors_flux.contains(interdit), "{interdit} subsiste");
        }
        // Mêmes longueurs : la table xref reste exacte.
        assert_eq!(d.octets.len(), src.len());
        // Le contenu d'un flux binaire n'est jamais touché (même s'il contient un nom actif).
        assert!(apres.contains("stream\n/JavaScript!\nendstream"));
        // Structure conservée.
        assert!(d.octets.starts_with(b"%PDF-1.4") && apres.contains("/Type /Page ") && apres.trim_end().ends_with("%%EOF"));
        // Idempotent.
        let encore = desarmer_pdf(&d.octets).unwrap();
        assert!(encore.neutralises.is_empty());
        assert_eq!(encore.octets, d.octets);
    }

    #[test]
    fn pdf_non_inspectables_refuses() {
        let mut chiffre = pdf_piege();
        chiffre.splice(9..9, b"% x\n".iter().copied());
        let chiffre = String::from_utf8_lossy(&chiffre).replace("/Root 1 0 R", "/Root 1 0 R /Encrypt 9 0 R").into_bytes();
        assert!(matches!(desarmer_pdf(&chiffre), Err(ErreurPiece::PdfRefuse(m)) if m.contains("chiffré")));
        let objstm = String::from_utf8_lossy(&pdf_piege()).replace("/Type /EmbeddedFile", "/Type /ObjStm").into_bytes();
        assert!(matches!(desarmer_pdf(&objstm), Err(ErreurPiece::PdfRefuse(m)) if m.contains("compressés")));
        assert!(matches!(desarmer_pdf(b"%PDF-1.4\n1 0 obj\n<< >>\nendobj\n%%EOF"), Err(ErreurPiece::PdfRefuse(_))), "sans page");
        assert!(matches!(desarmer_pdf(b"MZ\x90\x00 ceci n'est pas un pdf"), Err(ErreurPiece::PdfRefuse(_))));
        assert!(matches!(desarmer_pdf(b"%PDF-1.4\n1 0 obj\n<< /Type /Page >>\n"), Err(ErreurPiece::PdfRefuse(_))), "sans fin");
        // Un nom actif dans une chaîne ou un commentaire de texte n'est pas un nom : pas de refus ni de neutralisation.
        let texte = String::from_utf8_lossy(&pdf_piege()).replace("(http://evil.example/)", "(voir /JavaScript ici)").into_bytes();
        let d = desarmer_pdf(&texte).unwrap();
        assert!(String::from_utf8_lossy(&d.octets).contains("(voir /JavaScript ici)"));
    }

    #[test]
    fn noms_decodes() {
        assert_eq!(nom_decode(b"J#61vaScript"), "JavaScript");
        assert_eq!(nom_decode(b"Arial#20Bold"), "Arial Bold");
        assert_eq!(nom_decode(b"abc#"), "abc#");
        assert_eq!(nom_decode(b"abc#4"), "abc#4");
        assert_eq!(nom_decode(b"#zz"), "#zz");
    }
}
