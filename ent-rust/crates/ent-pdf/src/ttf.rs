//! Lecteur TrueType écrit à la main (D-062) : tables `head`, `hhea`, `maxp`, `hmtx`, `cmap`
//! (formats 4 et 12), `loca`, `glyf`, `post`, `name`, `OS/2`. Lecture seule, sans copie : la
//! structure emprunte les octets de la police.

fn u16_at(d: &[u8], o: usize) -> Option<u16> {
    Some(u16::from_be_bytes(d.get(o..o + 2)?.try_into().ok()?))
}
fn i16_at(d: &[u8], o: usize) -> Option<i16> {
    u16_at(d, o).map(|v| v as i16)
}
fn u32_at(d: &[u8], o: usize) -> Option<u32> {
    Some(u32::from_be_bytes(d.get(o..o + 4)?.try_into().ok()?))
}

pub(crate) struct Ttf {
    pub unites_em: u16,
    pub x_min: i16,
    pub y_min: i16,
    pub x_max: i16,
    pub y_max: i16,
    pub loca_long: bool,
    pub ascendante: i16,
    pub descendante: i16,
    pub nb_glyphes: u16,
    pub nb_metriques: u16,
    pub angle_italique: f32,
    pub poids: u16,
    pub hauteur_capitale: i16,
    pub nom_postscript: String,
    tables: Vec<([u8; 4], usize, usize)>,
    donnees: &'static [u8],
    cmap_off: usize,
    cmap_format: u16,
}

impl Ttf {
    pub fn lire(d: &'static [u8]) -> Result<Ttf, &'static str> {
        if d.len() < 12 || (u32_at(d, 0) != Some(0x0001_0000) && d.get(0..4) != Some(b"true")) {
            return Err("police TrueType attendue");
        }
        let n = u16_at(d, 4).ok_or("en-tête tronqué")? as usize;
        let mut tables = Vec::with_capacity(n);
        for i in 0..n {
            let e = 12 + 16 * i;
            let tag: [u8; 4] = d
                .get(e..e + 4)
                .ok_or("répertoire tronqué")?
                .try_into()
                .unwrap();
            let off = u32_at(d, e + 8).ok_or("répertoire tronqué")? as usize;
            let len = u32_at(d, e + 12).ok_or("répertoire tronqué")? as usize;
            if off.checked_add(len).is_none_or(|f| f > d.len()) {
                return Err("table hors du fichier");
            }
            tables.push((tag, off, len));
        }
        let mut t = Ttf {
            unites_em: 0,
            x_min: 0,
            y_min: 0,
            x_max: 0,
            y_max: 0,
            loca_long: false,
            ascendante: 0,
            descendante: 0,
            nb_glyphes: 0,
            nb_metriques: 0,
            angle_italique: 0.0,
            poids: 400,
            hauteur_capitale: 0,
            nom_postscript: String::new(),
            tables,
            donnees: d,
            cmap_off: 0,
            cmap_format: 0,
        };
        let head = t.table(b"head").ok_or("table head absente")?;
        t.unites_em = u16_at(head, 18).ok_or("head tronquée")?;
        t.x_min = i16_at(head, 36).ok_or("head tronquée")?;
        t.y_min = i16_at(head, 38).ok_or("head tronquée")?;
        t.x_max = i16_at(head, 40).ok_or("head tronquée")?;
        t.y_max = i16_at(head, 42).ok_or("head tronquée")?;
        t.loca_long = i16_at(head, 50).ok_or("head tronquée")? != 0;
        if t.unites_em == 0 {
            return Err("unitsPerEm nul");
        }
        let hhea = t.table(b"hhea").ok_or("table hhea absente")?;
        t.ascendante = i16_at(hhea, 4).ok_or("hhea tronquée")?;
        t.descendante = i16_at(hhea, 6).ok_or("hhea tronquée")?;
        t.nb_metriques = u16_at(hhea, 34).ok_or("hhea tronquée")?;
        let maxp = t.table(b"maxp").ok_or("table maxp absente")?;
        t.nb_glyphes = u16_at(maxp, 4).ok_or("maxp tronquée")?;
        if t.nb_metriques == 0 || t.nb_metriques > t.nb_glyphes {
            return Err("hhea incohérente");
        }
        let hmtx = t.table(b"hmtx").ok_or("table hmtx absente")?;
        if hmtx.len() < 4 * t.nb_metriques as usize {
            return Err("hmtx tronquée");
        }
        let loca = t.table(b"loca").ok_or("table loca absente")?;
        let attendu = (t.nb_glyphes as usize + 1) * if t.loca_long { 4 } else { 2 };
        if loca.len() < attendu {
            return Err("loca tronquée");
        }
        t.table(b"glyf").ok_or("table glyf absente")?;
        if let Some(post) = t.table(b"post") {
            if let Some(v) = u32_at(post, 4) {
                t.angle_italique = v as i32 as f32 / 65536.0;
            }
        }
        if let Some(os2) = t.table(b"OS/2") {
            t.poids = u16_at(os2, 4).unwrap_or(400);
            if u16_at(os2, 0).unwrap_or(0) >= 2 {
                t.hauteur_capitale = i16_at(os2, 88).unwrap_or(0);
            }
        }
        if t.hauteur_capitale <= 0 {
            t.hauteur_capitale = t.ascendante;
        }
        t.nom_postscript = t.nom(6).unwrap_or_else(|| "Police".into());
        t.choisir_cmap()?;
        Ok(t)
    }

    pub fn table(&self, tag: &[u8; 4]) -> Option<&'static [u8]> {
        let (_, o, l) = self.tables.iter().find(|(t, _, _)| t == tag)?;
        self.donnees.get(*o..*o + *l)
    }

    /// Nom de la table `name` pour l'identifiant donné (Windows Unicode ou Mac Roman).
    fn nom(&self, id: u16) -> Option<String> {
        let n = self.table(b"name")?;
        let nb = u16_at(n, 2)? as usize;
        let stockage = u16_at(n, 4)? as usize;
        for i in 0..nb {
            let e = 6 + 12 * i;
            if u16_at(n, e + 6)? != id {
                continue;
            }
            let (plat, long, off) = (
                u16_at(n, e)?,
                u16_at(n, e + 8)? as usize,
                u16_at(n, e + 10)? as usize,
            );
            let b = n.get(stockage + off..stockage + off + long)?;
            return match plat {
                3 | 0 => {
                    let v: Vec<u16> = b
                        .chunks_exact(2)
                        .map(|c| u16::from_be_bytes([c[0], c[1]]))
                        .collect();
                    Some(String::from_utf16_lossy(&v))
                }
                1 => Some(b.iter().map(|&c| c as char).collect()),
                _ => None,
            };
        }
        None
    }

    fn choisir_cmap(&mut self) -> Result<(), &'static str> {
        // Un sous-ensemble PDF n'a pas de cmap : lecture permise, `glyphe_de` répond alors `None`.
        let Some(c) = self.table(b"cmap") else {
            return Ok(());
        };
        let nb = u16_at(c, 2).ok_or("cmap tronquée")? as usize;
        // Préférence : (3,10) format 12, puis (0,4|6) format 12, puis (3,1) format 4, puis (0,*).
        let mut meilleur: Option<(u8, usize, u16)> = None;
        for i in 0..nb {
            let e = 4 + 8 * i;
            let (plat, enc) = (
                u16_at(c, e).ok_or("cmap tronquée")?,
                u16_at(c, e + 2).ok_or("cmap tronquée")?,
            );
            let off = u32_at(c, e + 4).ok_or("cmap tronquée")? as usize;
            let fmt = u16_at(c, off).unwrap_or(0);
            if fmt != 4 && fmt != 12 {
                continue;
            }
            let score = match (plat, enc, fmt) {
                (3, 10, 12) => 4,
                (0, _, 12) => 3,
                (3, 1, 4) => 2,
                (0, _, 4) => 1,
                _ => continue,
            };
            if meilleur.is_none_or(|(s, _, _)| score > s) {
                meilleur = Some((score, off, fmt));
            }
        }
        let (_, off, fmt) = meilleur.ok_or("aucune cmap Unicode (format 4 ou 12)")?;
        self.cmap_off = off;
        self.cmap_format = fmt;
        Ok(())
    }

    /// Glyphe d'un caractère Unicode ; `None` si la police ne le couvre pas (ou le mappe sur .notdef).
    pub fn glyphe_de(&self, c: char) -> Option<u16> {
        let cp = c as u32;
        if self.cmap_format == 0 {
            return None;
        }
        let t = self.table(b"cmap")?;
        let t = &t[self.cmap_off..];
        let g = if self.cmap_format == 12 {
            let n = u32_at(t, 12)? as usize;
            let (mut a, mut b) = (0usize, n);
            let mut trouve = None;
            while a < b {
                let m = (a + b) / 2;
                let e = 16 + 12 * m;
                let (d, f, g0) = (u32_at(t, e)?, u32_at(t, e + 4)?, u32_at(t, e + 8)?);
                if cp < d {
                    b = m;
                } else if cp > f {
                    a = m + 1;
                } else {
                    trouve = Some(g0.checked_add(cp - d)?);
                    break;
                }
            }
            trouve?
        } else {
            if cp > 0xffff {
                return None;
            }
            let cp = cp as u16;
            let seg2 = u16_at(t, 6)? as usize;
            let n = seg2 / 2;
            let fins = 14;
            let debuts = 16 + seg2;
            let deltas = debuts + seg2;
            let plages = deltas + seg2;
            let (mut a, mut b) = (0usize, n);
            while a < b {
                let m = (a + b) / 2;
                if u16_at(t, fins + 2 * m)? < cp {
                    a = m + 1;
                } else {
                    b = m;
                }
            }
            if a >= n {
                return None;
            }
            let debut = u16_at(t, debuts + 2 * a)?;
            if cp < debut {
                return None;
            }
            let delta = u16_at(t, deltas + 2 * a)?;
            let plage = u16_at(t, plages + 2 * a)?;
            if plage == 0 {
                (cp.wrapping_add(delta)) as u32
            } else {
                let o = plages + 2 * a + plage as usize + 2 * (cp - debut) as usize;
                let g = u16_at(t, o)?;
                if g == 0 {
                    return None;
                }
                g.wrapping_add(delta) as u32
            }
        };
        if g == 0 || g >= self.nb_glyphes as u32 {
            None
        } else {
            Some(g as u16)
        }
    }

    /// Avance horizontale (unités de la police) et débord gauche d'un glyphe.
    pub fn metrique(&self, g: u16) -> (u16, i16) {
        let h = self.table(b"hmtx").unwrap_or(&[]);
        let nm = self.nb_metriques as usize;
        let g = g as usize;
        if g < nm {
            (
                u16_at(h, 4 * g).unwrap_or(0),
                i16_at(h, 4 * g + 2).unwrap_or(0),
            )
        } else {
            let av = u16_at(h, 4 * (nm - 1)).unwrap_or(0);
            (av, i16_at(h, 4 * nm + 2 * (g - nm)).unwrap_or(0))
        }
    }

    /// Octets du glyphe (vides pour un glyphe sans contour, comme l'espace).
    pub fn glyphe(&self, g: u16) -> &'static [u8] {
        let (Some(loca), Some(glyf)) = (self.table(b"loca"), self.table(b"glyf")) else {
            return &[];
        };
        let g = g as usize;
        let (a, b) = if self.loca_long {
            (u32_at(loca, 4 * g), u32_at(loca, 4 * g + 4))
        } else {
            (
                u16_at(loca, 2 * g).map(|v| v as u32 * 2),
                u16_at(loca, 2 * g + 2).map(|v| v as u32 * 2),
            )
        };
        match (a, b) {
            (Some(a), Some(b)) if b > a => glyf.get(a as usize..b as usize).unwrap_or(&[]),
            _ => &[],
        }
    }

    /// Composants d'un glyphe composite : (position de l'identifiant dans les octets, identifiant).
    pub fn composants(glyphe: &[u8]) -> Vec<(usize, u16)> {
        let mut r = Vec::new();
        if glyphe.len() < 10 || i16_at(glyphe, 0).unwrap_or(0) >= 0 {
            return r;
        }
        let mut o = 10;
        while let (Some(flags), Some(g)) = (u16_at(glyphe, o), u16_at(glyphe, o + 2)) {
            r.push((o + 2, g));
            o += 4 + if flags & 1 != 0 { 4 } else { 2 };
            if flags & 0x08 != 0 {
                o += 2;
            } else if flags & 0x40 != 0 {
                o += 4;
            } else if flags & 0x80 != 0 {
                o += 8;
            }
            if flags & 0x20 == 0 {
                break;
            }
        }
        r
    }
}
