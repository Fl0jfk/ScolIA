//! Mise en page : pages A4, en-tête et pied répétés, « Page n sur N », paragraphes,
//! tableaux à en-tête répété, images JPEG, QR code vectoriel.

use crate::afm::{largeur, Police};
use crate::ecrit::{chaine_texte, litterale, Ecrivain};
use crate::jpeg::Jpeg;
use crate::qr::Qr;
use crate::winansi;
use crate::PdfErreur;

pub const LARGEUR: f32 = 595.0;
pub const HAUTEUR: f32 = 842.0;
pub const MARGE_G: f32 = 56.0;
pub const MARGE_D: f32 = 539.0;
const BAS: f32 = 64.0;
const HAUT_SANS_EN_TETE: f32 = 786.0;
const HAUT_AVEC_EN_TETE: f32 = 748.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Align {
    Gauche,
    Centre,
    Droite,
}

/// Date de création (UTC) écrite dans les métadonnées ; absente = rendu déterministe sans date.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DateHeure {
    pub an: u16,
    pub mois: u8,
    pub jour: u8,
    pub heure: u8,
    pub minute: u8,
    pub seconde: u8,
}

#[derive(Debug, Clone, Default)]
pub struct Meta {
    pub titre: String,
    pub auteur: String,
    pub creation: Option<DateHeure>,
}

#[derive(Debug, Clone)]
pub struct Colonne {
    pub titre: String,
    /// Largeur relative (les poids sont normalisés sur la largeur utile).
    pub poids: f32,
    pub align: Align,
}

impl Colonne {
    pub fn new(titre: &str, poids: f32, align: Align) -> Colonne {
        Colonne { titre: titre.into(), poids, align }
    }
}

#[derive(Debug, Clone)]
pub struct Ligne {
    pub cellules: Vec<String>,
    pub gras: bool,
}

impl Ligne {
    pub fn new<S: Into<String>>(cellules: Vec<S>) -> Ligne {
        Ligne { cellules: cellules.into_iter().map(Into::into).collect(), gras: false }
    }
    pub fn grasse<S: Into<String>>(cellules: Vec<S>) -> Ligne {
        Ligne { cellules: cellules.into_iter().map(Into::into).collect(), gras: true }
    }
}

#[derive(Debug, Clone)]
pub struct Tableau {
    pub colonnes: Vec<Colonne>,
    pub lignes: Vec<Ligne>,
    pub taille: f32,
}

impl Tableau {
    pub fn new(colonnes: Vec<Colonne>) -> Tableau {
        Tableau { colonnes, lignes: Vec::new(), taille: 9.5 }
    }
}

struct Page {
    contenu: Vec<u8>,
    images: Vec<usize>,
}

pub struct Document {
    meta: Meta,
    en_tete: Option<Vec<u8>>,
    logo: Option<usize>,
    pied: Vec<u8>,
    pages: Vec<Page>,
    images: Vec<Jpeg>,
    y: f32,
}

/// Coupe un texte WinAnsi aux mots pour tenir dans `max` points ; un mot trop long est tranché.
pub fn couper(police: Police, taille: f32, texte: &[u8], max: f32) -> Vec<Vec<u8>> {
    let mut lignes: Vec<Vec<u8>> = Vec::new();
    for para in texte.split(|&b| b == b'\n') {
        let mut courant: Vec<u8> = Vec::new();
        for mot in para.split(|&b| b == b' ').filter(|m| !m.is_empty()) {
            let mut mot: &[u8] = mot;
            while largeur(police, mot, taille) > max && mot.len() > 1 {
                // Plus grand préfixe qui tient.
                let mut k = 1;
                while k < mot.len() && largeur(police, &mot[..k + 1], taille) <= max {
                    k += 1;
                }
                if !courant.is_empty() {
                    lignes.push(std::mem::take(&mut courant));
                }
                lignes.push(mot[..k].to_vec());
                mot = &mot[k..];
            }
            if mot.is_empty() {
                continue;
            }
            let mut essai = courant.clone();
            if !essai.is_empty() {
                essai.push(b' ');
            }
            essai.extend_from_slice(mot);
            if !courant.is_empty() && largeur(police, &essai, taille) > max {
                lignes.push(std::mem::take(&mut courant));
                courant = mot.to_vec();
            } else {
                courant = essai;
            }
        }
        lignes.push(courant);
    }
    while lignes.len() > 1 && lignes.last().is_some_and(|l| l.is_empty()) {
        lignes.pop();
    }
    lignes
}

fn nb(x: f32) -> String {
    let s = format!("{x:.2}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s.is_empty() || s == "-" { "0".into() } else { s.into() }
}

impl Document {
    pub fn new(meta: Meta) -> Document {
        Document {
            meta,
            en_tete: None,
            logo: None,
            pied: Vec::new(),
            pages: vec![Page { contenu: Vec::new(), images: Vec::new() }],
            images: Vec::new(),
            y: HAUT_SANS_EN_TETE,
        }
    }

    /// Texte d'en-tête répété sur chaque page (à définir avant le contenu).
    pub fn avec_en_tete(mut self, texte: &str) -> Document {
        self.en_tete = Some(winansi::encoder(texte));
        self.y = HAUT_AVEC_EN_TETE;
        self
    }

    /// Logo JPEG répété en en-tête (à définir avant le contenu).
    pub fn avec_logo(mut self, jpeg: &[u8]) -> Result<Document, PdfErreur> {
        self.images.push(Jpeg::lire(jpeg)?);
        self.logo = Some(self.images.len() - 1);
        self.y = HAUT_AVEC_EN_TETE;
        Ok(self)
    }

    /// Texte de pied de page (à gauche) ; « Page n sur N » est toujours à droite.
    pub fn avec_pied(mut self, texte: &str) -> Document {
        self.pied = winansi::encoder(texte);
        self
    }

    fn haut(&self) -> f32 {
        if self.en_tete.is_some() || self.logo.is_some() {
            HAUT_AVEC_EN_TETE
        } else {
            HAUT_SANS_EN_TETE
        }
    }

    fn page(&mut self) -> &mut Page {
        self.pages.last_mut().unwrap()
    }

    fn nouvelle_page(&mut self) {
        self.pages.push(Page { contenu: Vec::new(), images: Vec::new() });
        self.y = self.haut();
    }

    /// Passe à la page suivante si `h` points ne tiennent plus.
    fn assurer(&mut self, h: f32) {
        if self.y - h < BAS && self.y < self.haut() {
            self.nouvelle_page();
        }
    }

    pub fn saut_de_page(&mut self) {
        if self.y < self.haut() {
            self.nouvelle_page();
        }
    }

    pub fn espace(&mut self, h: f32) {
        self.y -= h;
    }

    fn emettre(page: &mut Vec<u8>, x: f32, base: f32, police: Police, taille: f32, texte: &[u8]) {
        if texte.is_empty() {
            return;
        }
        page.extend_from_slice(
            format!("BT /F{} {} Tf {} {} Td ", police.indice() + 1, nb(taille), nb(x), nb(base)).as_bytes(),
        );
        litterale(texte, page);
        page.extend_from_slice(b" Tj ET\n");
    }

    fn x_aligne(police: Police, taille: f32, texte: &[u8], align: Align, x0: f32, x1: f32) -> f32 {
        let l = largeur(police, texte, taille);
        match align {
            Align::Gauche => x0,
            Align::Centre => x0 + ((x1 - x0) - l) / 2.0,
            Align::Droite => x1 - l,
        }
    }

    /// Paragraphe avec retour à la ligne automatique et saut de page.
    pub fn texte(&mut self, texte: &str, police: Police, taille: f32, align: Align) {
        let lignes = couper(police, taille, &winansi::encoder(texte), MARGE_D - MARGE_G);
        let inter = taille * 1.3;
        for l in lignes {
            self.assurer(inter);
            let x = Self::x_aligne(police, taille, &l, align, MARGE_G, MARGE_D);
            let base = self.y - taille;
            Self::emettre(&mut self.page().contenu, x, base, police, taille, &l);
            self.y -= inter;
        }
    }

    pub fn titre(&mut self, texte: &str) {
        self.assurer(40.0);
        self.texte(texte, Police::HelveticaGras, 17.0, Align::Gauche);
        self.y -= 4.0;
    }

    pub fn sous_titre(&mut self, texte: &str) {
        self.y -= 6.0;
        self.assurer(40.0);
        self.texte(texte, Police::HelveticaGras, 12.0, Align::Gauche);
        self.y -= 2.0;
    }

    pub fn paragraphe(&mut self, texte: &str) {
        self.texte(texte, Police::Helvetica, 10.5, Align::Gauche);
    }

    /// Libellé à gauche (coupé aux mots), valeur alignée à droite sur la première ligne.
    pub fn ligne_droite(&mut self, gauche: &str, droite: &str, gras: bool) {
        let police = if gras { Police::HelveticaGras } else { Police::Helvetica };
        let t = 10.5;
        let d = winansi::encoder(droite);
        let ld = largeur(police, &d, t);
        let max = (MARGE_D - MARGE_G) - ld - 12.0;
        let lignes = couper(police, t, &winansi::encoder(gauche), max.max(60.0));
        let inter = t * 1.3;
        for (i, l) in lignes.iter().enumerate() {
            self.assurer(inter);
            let base = self.y - t;
            let dx = if i == 0 { 0.0 } else { 12.0 };
            Self::emettre(&mut self.page().contenu, MARGE_G + dx, base, police, t, l);
            if i == 0 {
                Self::emettre(&mut self.page().contenu, MARGE_D - ld, base, police, t, &d);
            }
            self.y -= inter;
        }
    }

    /// Filet horizontal sur toute la largeur utile.
    pub fn filet(&mut self) {
        self.assurer(6.0);
        let y = self.y - 3.0;
        let c = format!("0.5 w 0.6 G {} {} m {} {} l S 0 G\n", nb(MARGE_G), nb(y), nb(MARGE_D), nb(y));
        self.page().contenu.extend_from_slice(c.as_bytes());
        self.y -= 8.0;
    }

    fn poser_image(&mut self, idx: usize, x: f32, y: f32, w: f32, h: f32) {
        let derniere = self.pages.len() - 1;
        self.poser_image_page(derniere, idx, x, y, w, h);
    }

    fn poser_image_page(&mut self, page: usize, idx: usize, x: f32, y: f32, w: f32, h: f32) {
        let p = &mut self.pages[page];
        if !p.images.contains(&idx) {
            p.images.push(idx);
        }
        let c = format!("q {} 0 0 {} {} {} cm /Im{} Do Q\n", nb(w), nb(h), nb(x), nb(y), idx + 1);
        p.contenu.extend_from_slice(c.as_bytes());
    }

    /// Image JPEG en ligne, de largeur `largeur_pt` (hauteur proportionnelle).
    pub fn image(&mut self, jpeg: &[u8], largeur_pt: f32, align: Align) -> Result<(), PdfErreur> {
        let j = Jpeg::lire(jpeg)?;
        let (w, h) = (largeur_pt, largeur_pt * j.hauteur as f32 / j.largeur as f32);
        self.images.push(j);
        let idx = self.images.len() - 1;
        self.assurer(h + 6.0);
        let x = match align {
            Align::Gauche => MARGE_G,
            Align::Centre => MARGE_G + ((MARGE_D - MARGE_G) - w) / 2.0,
            Align::Droite => MARGE_D - w,
        };
        let y = self.y - h;
        self.poser_image(idx, x, y, w, h);
        self.y = y - 6.0;
        Ok(())
    }

    /// QR code vectoriel carré de `cote` points (zone de silence de 4 modules comprise).
    pub fn qr(&mut self, donnees: &str, cote: f32, align: Align) -> Result<(), PdfErreur> {
        let q = Qr::encoder(donnees.as_bytes())?;
        self.assurer(cote + 6.0);
        let x = match align {
            Align::Gauche => MARGE_G,
            Align::Centre => MARGE_G + ((MARGE_D - MARGE_G) - cote) / 2.0,
            Align::Droite => MARGE_D - cote,
        };
        let y = self.y - cote;
        Self::dessiner_qr(&mut self.page().contenu, &q, x, y, cote);
        self.y = y - 6.0;
        Ok(())
    }

    /// QR à une position absolue (coin bas gauche), sans toucher au curseur.
    pub fn qr_absolu(&mut self, donnees: &str, x: f32, y: f32, cote: f32) -> Result<(), PdfErreur> {
        let q = Qr::encoder(donnees.as_bytes())?;
        Self::dessiner_qr(&mut self.page().contenu, &q, x, y, cote);
        Ok(())
    }

    fn dessiner_qr(page: &mut Vec<u8>, q: &Qr, x: f32, y: f32, cote: f32) {
        let n = q.taille();
        let m = cote / (n as f32 + 8.0);
        let mut c = String::new();
        c.push_str(&format!("q 1 g {} {} {} {} re f 0 g\n", nb(x), nb(y), nb(cote), nb(cote)));
        for r in 0..n {
            for k in 0..n {
                if q.module(k, r) {
                    let px = x + (k as f32 + 4.0) * m;
                    let py = y + cote - (r as f32 + 5.0) * m;
                    c.push_str(&format!("{} {} {} {} re\n", nb(px), nb(py), nb(m), nb(m)));
                }
            }
        }
        c.push_str("f Q\n");
        page.extend_from_slice(c.as_bytes());
    }

    #[allow(clippy::too_many_arguments)]
    fn rangee(
        &mut self,
        cols: &[Colonne],
        larg: &[f32],
        taille: f32,
        cellules: &[Vec<Vec<u8>>],
        debut: usize,
        k: usize,
        police: Police,
        fond: bool,
    ) {
        const PAD: f32 = 4.0;
        let inter = taille * 1.25;
        let h = k as f32 * inter + 2.0 * PAD;
        let top = self.y;
        let mut x = MARGE_G;
        let mut c = Vec::new();
        for (i, col) in cols.iter().enumerate() {
            if fond {
                let r = format!("0.9 g {} {} {} {} re f 0 g\n", nb(x), nb(top - h), nb(larg[i]), nb(h));
                c.extend_from_slice(r.as_bytes());
            }
            let r = format!("0.5 w 0.4 G {} {} {} {} re S 0 G\n", nb(x), nb(top - h), nb(larg[i]), nb(h));
            c.extend_from_slice(r.as_bytes());
            for (j, l) in cellules[i].iter().skip(debut).take(k).enumerate() {
                let lx = Self::x_aligne(police, taille, l, col.align, x + PAD, x + larg[i] - PAD);
                let base = top - PAD - taille - j as f32 * inter - (inter - taille) / 2.0 + 1.0;
                Self::emettre(&mut c, lx, base, police, taille, l);
            }
            x += larg[i];
        }
        self.page().contenu.extend_from_slice(&c);
        self.y = top - h;
    }

    /// Tableau : en-tête répété à chaque page, cellules sur plusieurs lignes, bordures.
    /// Une rangée plus haute qu'une page est coupée ligne à ligne.
    pub fn tableau(&mut self, t: &Tableau) {
        const PAD: f32 = 4.0;
        let n = t.colonnes.len();
        if n == 0 {
            return;
        }
        let util = MARGE_D - MARGE_G;
        let somme: f32 = t.colonnes.iter().map(|c| c.poids).sum::<f32>().max(0.001);
        let larg: Vec<f32> = t.colonnes.iter().map(|c| util * c.poids / somme).collect();
        let inter = t.taille * 1.25;
        let decouper = |cellules: &[String], police: Police| -> Vec<Vec<Vec<u8>>> {
            (0..n)
                .map(|i| {
                    let txt = cellules.get(i).map(String::as_str).unwrap_or("");
                    couper(police, t.taille, &winansi::encoder(txt), larg[i] - 2.0 * PAD)
                })
                .collect()
        };
        let titres: Vec<String> = t.colonnes.iter().map(|c| c.titre.clone()).collect();
        let entete = decouper(&titres, Police::HelveticaGras);
        let k_entete = entete.iter().map(Vec::len).max().unwrap_or(1).max(1);
        let h_entete = k_entete as f32 * inter + 2.0 * PAD;
        let corps_max = self.haut() - BAS;
        let mut page_entete: Option<usize> = None;

        self.assurer(h_entete + inter + 2.0 * PAD);
        if t.lignes.is_empty() {
            self.rangee(&t.colonnes, &larg, t.taille, &entete, 0, k_entete, Police::HelveticaGras, true);
            self.y -= 8.0;
            return;
        }
        for ligne in &t.lignes {
            let police = if ligne.gras { Police::HelveticaGras } else { Police::Helvetica };
            let cellules = decouper(&ligne.cellules, police);
            let total = cellules.iter().map(Vec::len).max().unwrap_or(1).max(1);
            let mut fait = 0;
            while fait < total {
                let courante = self.pages.len() - 1;
                let besoin_entete = page_entete != Some(courante);
                let dispo = self.y - BAS - if besoin_entete { h_entete } else { 0.0 };
                let reste = total - fait;
                let h_reste = reste as f32 * inter + 2.0 * PAD;
                let en_haut = self.y >= self.haut() - 0.01;
                let k;
                if h_reste <= dispo {
                    k = reste;
                } else if fait == 0 && !en_haut && h_reste <= corps_max - h_entete {
                    self.nouvelle_page();
                    continue;
                } else {
                    let possible = ((dispo - 2.0 * PAD) / inter).floor();
                    if possible >= 1.0 {
                        k = (possible as usize).min(reste);
                    } else if !en_haut {
                        self.nouvelle_page();
                        continue;
                    } else {
                        k = 1;
                    }
                }
                if besoin_entete {
                    self.rangee(&t.colonnes, &larg, t.taille, &entete, 0, k_entete, Police::HelveticaGras, true);
                    page_entete = Some(courante);
                }
                self.rangee(&t.colonnes, &larg, t.taille, &cellules, fait, k, police, false);
                fait += k;
                if fait < total {
                    self.nouvelle_page();
                }
            }
        }
        self.y -= 8.0;
    }

    fn ecrire_info(&self, e: &mut Ecrivain) -> usize {
        let mut d = b"<< /Title ".to_vec();
        d.extend_from_slice(&chaine_texte(&self.meta.titre));
        d.extend_from_slice(b" /Author ");
        d.extend_from_slice(&chaine_texte(&self.meta.auteur));
        d.extend_from_slice(b" /Producer (ent-pdf)");
        if let Some(t) = self.meta.creation {
            d.extend_from_slice(
                format!(
                    " /CreationDate (D:{:04}{:02}{:02}{:02}{:02}{:02}Z)",
                    t.an, t.mois, t.jour, t.heure, t.minute, t.seconde
                )
                .as_bytes(),
            );
        }
        d.extend_from_slice(b" >>");
        e.objet(&d)
    }

    /// Assemble le fichier PDF.
    pub fn octets(mut self) -> Vec<u8> {
        let n = self.pages.len();
        // Habillage de chaque page : en-tête, filets, pied, numérotation.
        for i in 0..n {
            let mut c: Vec<u8> = Vec::new();
            if self.en_tete.is_some() || self.logo.is_some() {
                let mut x = MARGE_G;
                let haut = HAUTEUR - 30.0;
                if let Some(l) = self.logo {
                    let (lj, hj) = (self.images[l].largeur as f32, self.images[l].hauteur as f32);
                    let mut w = 34.0 * lj / hj;
                    if w > 120.0 {
                        w = 120.0;
                    }
                    let h = w * hj / lj;
                    self.poser_image_page(i, l, x, haut - h, w, h);
                    x += w + 12.0;
                }
                if let Some(t) = &self.en_tete {
                    Self::emettre(&mut c, x, haut - 22.0, Police::HelveticaGras, 11.0, t);
                }
                let f = format!(
                    "0.5 w 0.6 G {} {} m {} {} l S 0 G\n",
                    nb(MARGE_G),
                    nb(HAUTEUR - 72.0),
                    nb(MARGE_D),
                    nb(HAUTEUR - 72.0)
                );
                c.extend_from_slice(f.as_bytes());
            }
            let f = format!("0.5 w 0.6 G {} 50 m {} 50 l S 0 G\n", nb(MARGE_G), nb(MARGE_D));
            c.extend_from_slice(f.as_bytes());
            Self::emettre(&mut c, MARGE_G, 36.0, Police::Helvetica, 8.0, &self.pied);
            let num = winansi::encoder(&format!("Page {} sur {n}", i + 1));
            let x = MARGE_D - largeur(Police::Helvetica, &num, 8.0);
            Self::emettre(&mut c, x, 36.0, Police::Helvetica, 8.0, &num);
            self.pages[i].contenu.extend_from_slice(&c);
        }

        let mut e = Ecrivain::nouveau();
        // 1 catalogue, 2 pages, 3 info, 4-6 polices, puis images, puis (page, flux) par page.
        let n_images = self.images.len();
        let premiere_page = 7 + n_images;
        e.objet(b"<< /Type /Catalog /Pages 2 0 R >>");
        let kids: String = (0..n).map(|i| format!("{} 0 R ", premiere_page + 2 * i)).collect();
        e.objet(format!("<< /Type /Pages /Kids [ {kids}] /Count {n} >>").as_bytes());
        self.ecrire_info(&mut e);
        for p in [Police::Helvetica, Police::HelveticaGras, Police::Times] {
            e.objet(
                format!("<< /Type /Font /Subtype /Type1 /BaseFont /{} /Encoding /WinAnsiEncoding >>", p.nom_base())
                    .as_bytes(),
            );
        }
        for j in &self.images {
            let cs = if j.composantes == 1 { "/DeviceGray" } else { "/DeviceRGB" };
            e.flux(
                &format!(
                    "/Type /XObject /Subtype /Image /Width {} /Height {} /ColorSpace {cs} /BitsPerComponent 8 /Filter /DCTDecode",
                    j.largeur, j.hauteur
                ),
                &j.donnees,
            );
        }
        for (i, p) in self.pages.iter().enumerate() {
            let xobj: String = if p.images.is_empty() {
                String::new()
            } else {
                let l: String = p.images.iter().map(|k| format!("/Im{} {} 0 R ", k + 1, 7 + k)).collect();
                format!(" /XObject << {l}>>")
            };
            e.objet(
                format!(
                    "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {} {}] /Resources << /Font << /F1 4 0 R /F2 5 0 R /F3 6 0 R >>{xobj} >> /Contents {} 0 R >>",
                    LARGEUR as u32,
                    HAUTEUR as u32,
                    premiere_page + 2 * i + 1
                )
                .as_bytes(),
            );
            e.flux("", &p.contenu);
        }
        e.terminer(1, 3)
    }
}
