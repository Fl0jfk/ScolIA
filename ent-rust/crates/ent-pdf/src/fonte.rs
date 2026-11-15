//! Moteur de texte : police TrueType incorporée en sous-ensemble (CID Type 2, `Identity-H`)
//! ou, en repli, polices standard WinAnsi.
//!
//! Le texte circule en `Vec<char>` normalisé. Avec la police incorporée, chaque caractère
//! utilisé reçoit un CID dans l'ordre de première utilisation ; le sous-ensemble est construit
//! avec CID = GID (`CIDToGIDMap /Identity`), les largeurs vont dans `/W` et une `ToUnicode`
//! garde le texte copiable et recherchable.

use crate::afm::{largeur as largeur_std, Police};
use crate::ecrit::{nb, Ecrivain};
use crate::normalise::{est_marque, lettre_de_base, nfc};
use crate::sous_ensemble::sous_ensemble;
use crate::ttf::Ttf;
use crate::winansi;
use std::collections::{HashMap, HashSet};

static REGULIER: &[u8] = include_bytes!("../polices/DejaVuSans.ttf");
static GRAS: &[u8] = include_bytes!("../polices/DejaVuSans-Bold.ttf");

/// Mode de rendu du texte.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Police TrueType incorporée (DejaVu Sans) : tout nom latin, grec ou cyrillique s'écrit juste.
    Incorpore,
    /// Polices standard Helvetica/Times en WinAnsi (repli) ; ce qui sort de la table est translittéré.
    Standard,
}

struct Fonte {
    ttf: Ttf,
    gras: bool,
    cids: HashMap<char, u16>,
    ordre: Vec<char>,
}

impl Fonte {
    fn nouvelle(donnees: &'static [u8], gras: bool) -> Result<Fonte, &'static str> {
        let ttf = Ttf::lire(donnees)?;
        if ttf.glyphe_de(' ').is_none() || ttf.glyphe_de('é').is_none() {
            return Err("cmap Unicode absente ou incomplète");
        }
        Ok(Fonte {
            ttf,
            gras,
            cids: HashMap::new(),
            ordre: Vec::new(),
        })
    }

    fn gid(&self, c: char) -> u16 {
        self.ttf.glyphe_de(c).unwrap_or(0)
    }

    fn avance(&self, c: char) -> u16 {
        self.ttf.metrique(self.gid(c)).0
    }

    fn cid(&mut self, c: char) -> u16 {
        if let Some(&k) = self.cids.get(&c) {
            return k;
        }
        self.ordre.push(c);
        let k = self.ordre.len() as u16; // 0 = .notdef
        self.cids.insert(c, k);
        k
    }

    fn mille(&self, v: i32) -> i32 {
        let u = self.ttf.unites_em as i32;
        (v * 1000 + if v >= 0 { u / 2 } else { -u / 2 }) / u
    }

    /// Écrit les 5 objets de la police ; retourne le numéro de l'objet Type0.
    fn ecrire(&self, e: &mut Ecrivain) -> usize {
        let mut gids = vec![0u16];
        gids.extend(self.ordre.iter().map(|&c| self.gid(c)));
        // Un glyphe partagé par deux caractères reste en double dans la liste : CID = position.
        let sortie = sous_ensemble(&self.ttf, &gids);
        let base = format!("{}+{}", etiquette(&gids), self.ttf.nom_postscript);
        let id = e.nombre_objets() + 1;
        let (id_cid, id_desc, id_fichier, id_unicode) = (id + 1, id + 2, id + 3, id + 4);
        e.objet(
            format!(
                "<< /Type /Font /Subtype /Type0 /BaseFont /{base} /Encoding /Identity-H /DescendantFonts [{id_cid} 0 R] /ToUnicode {id_unicode} 0 R >>"
            )
            .as_bytes(),
        );
        let mut w = String::from("[");
        for g in &gids {
            w.push_str(&format!("{} ", self.mille(self.ttf.metrique(*g).0 as i32)));
        }
        w.push(']');
        e.objet(
            format!(
                "<< /Type /Font /Subtype /CIDFontType2 /BaseFont /{base} /CIDSystemInfo << /Registry (Adobe) /Ordering (Identity) /Supplement 0 >> /FontDescriptor {id_desc} 0 R /DW 1000 /W [0 {w}] /CIDToGIDMap /Identity >>"
            )
            .as_bytes(),
        );
        let t = &self.ttf;
        e.objet(
            format!(
                "<< /Type /FontDescriptor /FontName /{base} /Flags 32 /FontBBox [{} {} {} {}] /ItalicAngle {} /Ascent {} /Descent {} /CapHeight {} /StemV {} /FontFile2 {id_fichier} 0 R >>",
                self.mille(t.x_min as i32),
                self.mille(t.y_min as i32),
                self.mille(t.x_max as i32),
                self.mille(t.y_max as i32),
                t.angle_italique as i32,
                self.mille(t.ascendante as i32),
                self.mille(t.descendante as i32),
                self.mille(t.hauteur_capitale as i32),
                if self.gras || t.poids >= 600 { 140 } else { 80 },
            )
            .as_bytes(),
        );
        e.flux(&format!("/Length1 {}", sortie.octets.len()), &sortie.octets);
        e.flux("", self.to_unicode().as_bytes());
        id
    }

    fn to_unicode(&self) -> String {
        let mut s = String::from(
            "/CIDInit /ProcSet findresource begin\n12 dict begin\nbegincmap\n/CIDSystemInfo << /Registry (Adobe) /Ordering (UCS) /Supplement 0 >> def\n/CMapName /Adobe-Identity-UCS def\n/CMapType 2 def\n1 begincodespacerange\n<0000> <FFFF>\nendcodespacerange\n",
        );
        for (bloc, chars) in self.ordre.chunks(100).enumerate() {
            s.push_str(&format!("{} beginbfchar\n", chars.len()));
            for (i, c) in chars.iter().enumerate() {
                let cid = bloc * 100 + i + 1;
                let mut buf = [0u16; 2];
                let hex: String = c
                    .encode_utf16(&mut buf)
                    .iter()
                    .map(|u| format!("{u:04X}"))
                    .collect();
                s.push_str(&format!("<{cid:04X}> <{hex}>\n"));
            }
            s.push_str("endbfchar\n");
        }
        s.push_str("endcmap\nCMapName currentdict /CMap defineresource pop\nend\nend\n");
        s
    }
}

/// Étiquette de sous-ensemble de 6 majuscules (déterministe), exigée par la spécification.
fn etiquette(gids: &[u16]) -> String {
    let mut h: u32 = 0x811c_9dc5;
    for g in gids {
        for b in g.to_be_bytes() {
            h = (h ^ b as u32).wrapping_mul(0x0100_0193);
        }
    }
    (0..6)
        .map(|_| {
            let c = (b'A' + (h % 26) as u8) as char;
            h = h / 26 + 7919;
            c
        })
        .collect()
}

pub(crate) struct Moteur {
    pub mode: Mode,
    fontes: [Option<Fonte>; 2],
    avertissements: Vec<String>,
    deja_signales: HashSet<char>,
}

impl Moteur {
    pub fn new(mode: Mode) -> Moteur {
        let mut avertissements = Vec::new();
        let mut f = |d: &'static [u8], g: bool| match Fonte::nouvelle(d, g) {
            Ok(f) => Some(f),
            Err(m) => {
                avertissements.push(format!("police incorporée illisible ({m}) : repli WinAnsi"));
                None
            }
        };
        let (r, g) = (f(REGULIER, false), f(GRAS, true));
        let mode = if r.is_none() || g.is_none() {
            Mode::Standard
        } else {
            mode
        };
        Moteur {
            mode,
            fontes: [r, g],
            avertissements,
            deja_signales: HashSet::new(),
        }
    }

    pub fn avertissements(&self) -> &[String] {
        &self.avertissements
    }

    fn avertir(&mut self, c: char, message: &str) {
        if self.deja_signales.insert(c) {
            self.avertissements
                .push(format!("U+{:04X} {message}", c as u32));
        }
    }

    fn face(police: Police) -> usize {
        usize::from(police == Police::HelveticaGras)
    }

    fn fonte(&self, police: Police) -> &Fonte {
        self.fontes[Self::face(police)]
            .as_ref()
            .expect("police incorporée chargée")
    }

    /// Texte prêt à écrire : normalisé et sans caractère que la police ne saurait rendre.
    pub fn preparer(&mut self, s: &str) -> Vec<char> {
        match self.mode {
            Mode::Standard => {
                let mut inconnus = Vec::new();
                let octets = winansi::encoder_avec_inconnus(s, &mut inconnus);
                for c in inconnus {
                    self.avertir(c, "hors table WinAnsi : remplacé par « ? »");
                }
                octets.into_iter().map(winansi::depuis_octet).collect()
            }
            Mode::Incorpore => {
                let mut sortie = Vec::with_capacity(s.len());
                for c in nfc(s) {
                    self.resoudre(c, &mut sortie);
                }
                sortie
            }
        }
    }

    fn resoudre(&mut self, c: char, sortie: &mut Vec<char>) {
        match c {
            '\n' => return sortie.push('\n'),
            '\r' | '\u{200b}'..='\u{200d}' | '\u{feff}' | '\u{ad}' | '\u{2060}' => return,
            '\t' => return sortie.push(' '),
            c if c.is_control() => return sortie.push(' '),
            _ => {}
        }
        let f = self.fonte(Police::Helvetica);
        if f.ttf.glyphe_de(c).is_some() {
            return sortie.push(c);
        }
        if let Some(r) = winansi::repli(c) {
            let rc: Vec<char> = r.chars().collect();
            if rc.iter().all(|&x| f.ttf.glyphe_de(x).is_some()) {
                return sortie.extend(rc);
            }
        }
        let base = lettre_de_base(c);
        if base != c && f.ttf.glyphe_de(base).is_some() {
            self.avertir(c, "absent de la police : accent retiré");
            return sortie.push(base);
        }
        if est_marque(c) {
            return self.avertir(c, "marque combinante absente de la police : ignorée");
        }
        self.avertir(c, "absent de la police : glyphe de remplacement U+FFFD");
        sortie.push('\u{fffd}');
    }

    pub fn largeur(&self, police: Police, t: &[char], taille: f32) -> f32 {
        match self.mode {
            Mode::Standard => largeur_std(police, &vers_octets(t), taille),
            Mode::Incorpore => {
                let f = self.fonte(police);
                let somme: u32 = t.iter().map(|&c| f.avance(c) as u32).sum();
                somme as f32 * taille / f.ttf.unites_em as f32
            }
        }
    }

    /// Opérateurs d'un texte à la position (x, base), police et taille données.
    pub fn emettre(
        &mut self,
        page: &mut Vec<u8>,
        x: f32,
        base: f32,
        police: Police,
        taille: f32,
        t: &[char],
    ) {
        if t.is_empty() {
            return;
        }
        match self.mode {
            Mode::Standard => {
                page.extend_from_slice(
                    format!(
                        "BT /F{} {} Tf {} {} Td ",
                        police.indice() + 1,
                        nb(taille),
                        nb(x),
                        nb(base)
                    )
                    .as_bytes(),
                );
                crate::ecrit::litterale(&vers_octets(t), page);
            }
            Mode::Incorpore => {
                let i = Self::face(police);
                page.extend_from_slice(
                    format!(
                        "BT /F{} {} Tf {} {} Td <",
                        i + 1,
                        nb(taille),
                        nb(x),
                        nb(base)
                    )
                    .as_bytes(),
                );
                let f = self.fontes[i].as_mut().expect("police incorporée chargée");
                for &c in t {
                    page.extend_from_slice(format!("{:04X}", f.cid(c)).as_bytes());
                }
                page.push(b'>');
            }
        }
        page.extend_from_slice(b" Tj ET\n");
    }

    /// Nombre d'objets que `ecrire_polices` va écrire.
    pub fn nb_objets_polices(&self) -> usize {
        match self.mode {
            Mode::Standard => 3,
            Mode::Incorpore => {
                5 * self
                    .fontes
                    .iter()
                    .flatten()
                    .filter(|f| !f.ordre.is_empty())
                    .count()
            }
        }
    }

    /// Écrit les polices ; retourne (nom de ressource, numéro d'objet).
    pub fn ecrire_polices(&self, e: &mut Ecrivain) -> Vec<(String, usize)> {
        match self.mode {
            Mode::Standard => [Police::Helvetica, Police::HelveticaGras, Police::Times]
                .iter()
                .map(|p| {
                    let n = e.objet(
                        format!(
                            "<< /Type /Font /Subtype /Type1 /BaseFont /{} /Encoding /WinAnsiEncoding >>",
                            p.nom_base()
                        )
                        .as_bytes(),
                    );
                    (format!("F{}", p.indice() + 1), n)
                })
                .collect(),
            Mode::Incorpore => self
                .fontes
                .iter()
                .enumerate()
                .filter_map(|(i, f)| {
                    let f = f.as_ref()?;
                    (!f.ordre.is_empty()).then(|| (format!("F{}", i + 1), f.ecrire(e)))
                })
                .collect(),
        }
    }
}

fn vers_octets(t: &[char]) -> Vec<u8> {
    t.iter()
        .map(|&c| winansi::octet(c).unwrap_or(if c == '\n' { b'\n' } else { b'?' }))
        .collect()
}
