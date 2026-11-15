#![allow(clippy::manual_is_multiple_of)]
//! QR code maison (ISO/IEC 18004) : modes octet et alphanumérique, correction d'erreur M,
//! versions 1 à 10, huit masques choisis par pénalités. Aucune dépendance.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QrErreur {
    /// Les données dépassent la capacité de la version 10-M (213 octets).
    TropLong,
}

impl std::fmt::Display for QrErreur {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QrErreur::TropLong => f.write_str("données trop longues pour un QR code 10-M"),
        }
    }
}
impl std::error::Error for QrErreur {}

/// (octets de correction par bloc, [(nombre de blocs, octets de données par bloc); 2]),
/// niveau M, index = version − 1.
const BLOCS_M: [(usize, [(usize, usize); 2]); 10] = [
    (10, [(1, 16), (0, 0)]),
    (16, [(1, 28), (0, 0)]),
    (26, [(1, 44), (0, 0)]),
    (18, [(2, 32), (0, 0)]),
    (24, [(2, 43), (0, 0)]),
    (16, [(4, 27), (0, 0)]),
    (18, [(4, 31), (0, 0)]),
    (22, [(2, 38), (2, 39)]),
    (22, [(3, 36), (2, 37)]),
    (26, [(4, 43), (1, 44)]),
];

const ALIGNEMENTS: [&[usize]; 10] = [
    &[],
    &[6, 18],
    &[6, 22],
    &[6, 26],
    &[6, 30],
    &[6, 34],
    &[6, 22, 38],
    &[6, 24, 42],
    &[6, 26, 46],
    &[6, 28, 50],
];

fn octets_donnees(version: usize) -> usize {
    let (_, g) = BLOCS_M[version - 1];
    g.iter().map(|(n, k)| n * k).sum()
}

// ---------- Corps de Galois GF(256), polynôme 0x11D ----------

#[allow(clippy::needless_range_loop)]
fn tables_gf() -> ([u8; 256], [u8; 256]) {
    let mut exp = [0u8; 256];
    let mut log = [0u8; 256];
    let mut x: u16 = 1;
    for i in 0..255 {
        exp[i] = x as u8;
        log[x as usize] = i as u8;
        x <<= 1;
        if x & 0x100 != 0 {
            x ^= 0x11d;
        }
    }
    exp[255] = exp[0];
    (exp, log)
}

fn mul(a: u8, b: u8, exp: &[u8; 256], log: &[u8; 256]) -> u8 {
    if a == 0 || b == 0 {
        0
    } else {
        exp[(usize::from(log[a as usize]) + usize::from(log[b as usize])) % 255]
    }
}

/// Octets de correction Reed-Solomon de `donnees` (n octets de correction).
pub(crate) fn reed_solomon(donnees: &[u8], n: usize) -> Vec<u8> {
    let (exp, log) = tables_gf();
    // Polynôme générateur ∏ (x − α^i), i = 0..n.
    let mut g = vec![0u8; n];
    g[n - 1] = 1;
    let mut racine = 1u8;
    for _ in 0..n {
        for j in 0..n {
            g[j] = mul(g[j], racine, &exp, &log);
            if j + 1 < n {
                g[j] ^= g[j + 1];
            }
        }
        racine = mul(racine, 2, &exp, &log);
    }
    let mut reste = vec![0u8; n];
    for &d in donnees {
        let f = d ^ reste[0];
        reste.remove(0);
        reste.push(0);
        for (r, &c) in reste.iter_mut().zip(&g) {
            *r ^= mul(c, f, &exp, &log);
        }
    }
    reste
}

// ---------- Flux de bits ----------

const ALNUM: &[u8; 45] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ $%*+-./:";

fn est_alnum(d: &[u8]) -> bool {
    !d.is_empty() && d.iter().all(|b| ALNUM.contains(b))
}

fn ajouter(bits: &mut Vec<bool>, valeur: u32, n: usize) {
    for i in (0..n).rev() {
        bits.push((valeur >> i) & 1 == 1);
    }
}

fn flux_donnees(donnees: &[u8], version: usize, alnum: bool) -> Vec<bool> {
    let mut bits = Vec::new();
    if alnum {
        ajouter(&mut bits, 0b0010, 4);
        ajouter(
            &mut bits,
            donnees.len() as u32,
            if version <= 9 { 9 } else { 11 },
        );
        let val = |b: u8| ALNUM.iter().position(|&c| c == b).unwrap() as u32;
        for p in donnees.chunks(2) {
            if p.len() == 2 {
                ajouter(&mut bits, val(p[0]) * 45 + val(p[1]), 11);
            } else {
                ajouter(&mut bits, val(p[0]), 6);
            }
        }
    } else {
        ajouter(&mut bits, 0b0100, 4);
        ajouter(
            &mut bits,
            donnees.len() as u32,
            if version <= 9 { 8 } else { 16 },
        );
        for &b in donnees {
            ajouter(&mut bits, u32::from(b), 8);
        }
    }
    bits
}

fn bits_necessaires(len: usize, version: usize, alnum: bool) -> usize {
    if alnum {
        4 + if version <= 9 { 9 } else { 11 } + (len / 2) * 11 + (len % 2) * 6
    } else {
        4 + if version <= 9 { 8 } else { 16 } + len * 8
    }
}

/// Mots de code de données complétés (terminateur, bourrage 0xEC/0x11).
fn mots_de_code(donnees: &[u8], version: usize, alnum: bool) -> Vec<u8> {
    let cap = octets_donnees(version) * 8;
    let mut bits = flux_donnees(donnees, version, alnum);
    let t = (cap - bits.len()).min(4);
    bits.extend(std::iter::repeat_n(false, t));
    while bits.len() % 8 != 0 {
        bits.push(false);
    }
    let mut mots: Vec<u8> = bits
        .chunks(8)
        .map(|c| c.iter().fold(0u8, |a, &b| a << 1 | u8::from(b)))
        .collect();
    let mut pad = [0xecu8, 0x11].into_iter().cycle();
    while mots.len() < cap / 8 {
        mots.push(pad.next().unwrap());
    }
    mots
}

/// Données + correction, entrelacées selon la norme.
pub(crate) fn mots_entrelaces(donnees: &[u8], version: usize) -> Vec<u8> {
    let (nec, groupes) = BLOCS_M[version - 1];
    let mut blocs: Vec<&[u8]> = Vec::new();
    let mut pos = 0;
    for (n, k) in groupes {
        for _ in 0..n {
            blocs.push(&donnees[pos..pos + k]);
            pos += k;
        }
    }
    let ecs: Vec<Vec<u8>> = blocs.iter().map(|b| reed_solomon(b, nec)).collect();
    let max = blocs.iter().map(|b| b.len()).max().unwrap_or(0);
    let mut sortie = Vec::new();
    for i in 0..max {
        for b in &blocs {
            if i < b.len() {
                sortie.push(b[i]);
            }
        }
    }
    for i in 0..nec {
        for e in &ecs {
            sortie.push(e[i]);
        }
    }
    sortie
}

// ---------- Matrice ----------

#[derive(Debug, Clone)]
pub struct Qr {
    pub version: usize,
    pub masque: u8,
    taille: usize,
    modules: Vec<bool>,
}

struct Grille {
    n: usize,
    v: Vec<bool>,
    f: Vec<bool>,
}

impl Grille {
    fn nouvelle(n: usize) -> Grille {
        Grille {
            n,
            v: vec![false; n * n],
            f: vec![false; n * n],
        }
    }
    fn poser(&mut self, x: usize, y: usize, noir: bool) {
        self.v[y * self.n + x] = noir;
        self.f[y * self.n + x] = true;
    }
}

fn bch_format(masque: u8) -> u32 {
    // Niveau M = 00.
    let data = u32::from(masque);
    let mut rem = data;
    for _ in 0..10 {
        rem = (rem << 1) ^ ((rem >> 9) * 0x537);
    }
    (data << 10 | rem) ^ 0x5412
}

fn bch_version(version: u32) -> u32 {
    let mut rem = version;
    for _ in 0..12 {
        rem = (rem << 1) ^ ((rem >> 11) * 0x1f25);
    }
    version << 12 | rem
}

fn ecrire_format(g: &mut Grille, masque: u8) {
    let n = g.n;
    let bits = bch_format(masque);
    let b = |i: usize| (bits >> i) & 1 == 1;
    for i in 0..=5 {
        g.poser(8, i, b(i));
    }
    g.poser(8, 7, b(6));
    g.poser(8, 8, b(7));
    g.poser(7, 8, b(8));
    for i in 9..15 {
        g.poser(14 - i, 8, b(i));
    }
    for i in 0..8 {
        g.poser(n - 1 - i, 8, b(i));
    }
    for i in 8..15 {
        g.poser(8, n - 15 + i, b(i));
    }
    g.poser(8, n - 8, true); // module sombre
}

/// Motifs de fonction (finders, synchronisation, alignements, version) ; format réservé.
fn motifs_fonction(version: usize) -> Grille {
    let n = 17 + 4 * version;
    let mut g = Grille::nouvelle(n);
    for i in 0..n {
        g.poser(6, i, i % 2 == 0);
        g.poser(i, 6, i % 2 == 0);
    }
    // Finders + séparateurs.
    for &(cx, cy) in &[(3i32, 3i32), (n as i32 - 4, 3), (3, n as i32 - 4)] {
        for dy in -4i32..=4 {
            for dx in -4i32..=4 {
                let (x, y) = (cx + dx, cy + dy);
                if x < 0 || y < 0 || x >= n as i32 || y >= n as i32 {
                    continue;
                }
                let d = dx.abs().max(dy.abs());
                g.poser(x as usize, y as usize, d != 2 && d != 4);
            }
        }
    }
    // Alignements.
    let pos = ALIGNEMENTS[version - 1];
    for (i, &cy) in pos.iter().enumerate() {
        for (j, &cx) in pos.iter().enumerate() {
            let coin = (i == 0 && j == 0)
                || (i == 0 && j == pos.len() - 1)
                || (i == pos.len() - 1 && j == 0);
            if coin {
                continue;
            }
            for dy in -2i32..=2 {
                for dx in -2i32..=2 {
                    let d = dx.abs().max(dy.abs());
                    g.poser((cx as i32 + dx) as usize, (cy as i32 + dy) as usize, d != 1);
                }
            }
        }
    }
    ecrire_format(&mut g, 0); // réserve la zone
    if version >= 7 {
        let bits = bch_version(version as u32);
        for i in 0..18 {
            let bit = (bits >> i) & 1 == 1;
            let a = n - 11 + i % 3;
            let b = i / 3;
            g.poser(a, b, bit);
            g.poser(b, a, bit);
        }
    }
    g
}

#[allow(clippy::manual_is_multiple_of)]
fn condition_masque(m: u8, i: usize, j: usize) -> bool {
    match m {
        0 => (i + j) % 2 == 0,
        1 => i % 2 == 0,
        2 => j % 3 == 0,
        3 => (i + j) % 3 == 0,
        4 => (i / 2 + j / 3) % 2 == 0,
        5 => (i * j) % 2 + (i * j) % 3 == 0,
        6 => ((i * j) % 2 + (i * j) % 3) % 2 == 0,
        _ => ((i + j) % 2 + (i * j) % 3) % 2 == 0,
    }
}

fn penalite(g: &Grille) -> u32 {
    let n = g.n;
    let m = |x: usize, y: usize| g.v[y * n + x];
    let mut p = 0u32;
    // N1 : séries de 5 modules identiques ou plus, en lignes et en colonnes.
    for sens in 0..2 {
        for a in 0..n {
            let mut run = 1;
            for b in 1..n {
                let (c0, c1) = if sens == 0 {
                    (m(b - 1, a), m(b, a))
                } else {
                    (m(a, b - 1), m(a, b))
                };
                if c0 == c1 {
                    run += 1;
                    if run == 5 {
                        p += 3;
                    } else if run > 5 {
                        p += 1;
                    }
                } else {
                    run = 1;
                }
            }
        }
    }
    // N2 : blocs 2×2.
    for y in 0..n - 1 {
        for x in 0..n - 1 {
            let c = m(x, y);
            if c == m(x + 1, y) && c == m(x, y + 1) && c == m(x + 1, y + 1) {
                p += 3;
            }
        }
    }
    // N3 : motif 1:1:3:1:1 bordé de 4 modules clairs.
    const A: [bool; 11] = [
        false, false, false, false, true, false, true, true, true, false, true,
    ];
    const B: [bool; 11] = [
        true, false, true, true, true, false, true, false, false, false, false,
    ];
    for sens in 0..2 {
        for a in 0..n {
            for b in 0..=n - 11 {
                let get = |k: usize| if sens == 0 { m(b + k, a) } else { m(a, b + k) };
                if (0..11).all(|k| get(k) == A[k]) {
                    p += 40;
                }
                if (0..11).all(|k| get(k) == B[k]) {
                    p += 40;
                }
            }
        }
    }
    // N4 : proportion de modules sombres.
    let noirs = g.v.iter().filter(|&&b| b).count();
    let total = n * n;
    let k = (noirs * 20)
        .abs_diff(total * 10)
        .div_ceil(total)
        .saturating_sub(1);
    p + 10 * k as u32
}

impl Qr {
    /// Encode `donnees` (octets, normalement de l'ASCII ou de l'UTF-8) au niveau M.
    pub fn encoder(donnees: &[u8]) -> Result<Qr, QrErreur> {
        let alnum = est_alnum(donnees);
        let version = (1..=10)
            .find(|&v| bits_necessaires(donnees.len(), v, alnum) <= octets_donnees(v) * 8)
            .ok_or(QrErreur::TropLong)?;
        let mots = mots_entrelaces(&mots_de_code(donnees, version, alnum), version);
        let base = motifs_fonction(version);
        let n = base.n;

        let mut g = Grille {
            n,
            v: base.v.clone(),
            f: base.f.clone(),
        };
        let total_bits = mots.len() * 8;
        let mut i = 0;
        let mut droite = n as i32 - 1;
        while droite >= 1 {
            if droite == 6 {
                droite = 5;
            }
            for vert in 0..n {
                for j in 0..2 {
                    let x = (droite - j) as usize;
                    let haut = ((droite + 1) & 2) == 0;
                    let y = if haut { n - 1 - vert } else { vert };
                    if !g.f[y * n + x] && i < total_bits {
                        g.v[y * n + x] = (mots[i / 8] >> (7 - i % 8)) & 1 == 1;
                        i += 1;
                    }
                }
            }
            droite -= 2;
        }

        let mut meilleur: Option<(u32, u8, Grille)> = None;
        for masque in 0..8u8 {
            let mut c = Grille {
                n,
                v: g.v.clone(),
                f: g.f.clone(),
            };
            for y in 0..n {
                for x in 0..n {
                    if !base.f[y * n + x] && condition_masque(masque, y, x) {
                        c.v[y * n + x] ^= true;
                    }
                }
            }
            ecrire_format(&mut c, masque);
            let p = penalite(&c);
            if meilleur.as_ref().is_none_or(|(bp, _, _)| p < *bp) {
                meilleur = Some((p, masque, c));
            }
        }
        let (_, masque, c) = meilleur.unwrap();
        Ok(Qr {
            version,
            masque,
            taille: n,
            modules: c.v,
        })
    }

    pub fn taille(&self) -> usize {
        self.taille
    }

    /// `true` = module sombre ; `(x, y)` = (colonne, ligne), origine en haut à gauche.
    pub fn module(&self, x: usize, y: usize) -> bool {
        self.modules[y * self.taille + x]
    }
}

#[cfg(test)]
pub(crate) mod decodeur {
    //! Décodeur de test : relit la matrice comme le ferait un lecteur (format → masque →
    //! zigzag → désentrelacement → contrôle Reed-Solomon → flux de bits).
    use super::*;

    pub fn decoder(qr: &Qr) -> Result<Vec<u8>, String> {
        decoder_matrice(qr.taille(), &qr.modules)
    }

    /// Décode une matrice brute (`n × n`, ligne par ligne, `true` = sombre).
    pub fn decoder_matrice(n: usize, modules: &[bool]) -> Result<Vec<u8>, String> {
        let version = (n - 17) / 4;
        let m = |x: usize, y: usize| modules[y * n + x];
        // Format, première copie.
        let mut f = 0u32;
        let ordre: Vec<(usize, usize)> = (0..=5)
            .map(|i| (8, i))
            .chain([(8, 7), (8, 8), (7, 8)])
            .chain((9..15).map(|i| (14 - i, 8)))
            .collect();
        for (i, &(x, y)) in ordre.iter().enumerate() {
            f |= u32::from(m(x, y)) << i;
        }
        let masque = (0..8u8)
            .find(|&k| (bch_format(k) ^ f).count_ones() <= 3)
            .ok_or("format illisible")?;
        // Seconde copie identique.
        let mut f2 = 0u32;
        for i in 0..8 {
            f2 |= u32::from(m(n - 1 - i, 8)) << i;
        }
        for i in 8..15 {
            f2 |= u32::from(m(8, n - 15 + i)) << i;
        }
        if f2 != f {
            return Err("copies du format différentes".into());
        }
        let fonction = motifs_fonction(version);
        // Zigzag.
        let mut bits = Vec::new();
        let mut droite = n as i32 - 1;
        while droite >= 1 {
            if droite == 6 {
                droite = 5;
            }
            for vert in 0..n {
                for j in 0..2 {
                    let x = (droite - j) as usize;
                    let haut = ((droite + 1) & 2) == 0;
                    let y = if haut { n - 1 - vert } else { vert };
                    if !fonction.f[y * n + x] {
                        bits.push(m(x, y) ^ condition_masque(masque, y, x));
                    }
                }
            }
            droite -= 2;
        }
        let mots: Vec<u8> = bits
            .chunks_exact(8)
            .map(|c| c.iter().fold(0u8, |a, &b| a << 1 | u8::from(b)))
            .collect();
        // Désentrelacement.
        let (nec, groupes) = BLOCS_M[version - 1];
        let tailles: Vec<usize> = groupes
            .iter()
            .flat_map(|&(c, k)| std::iter::repeat_n(k, c))
            .collect();
        let nb = tailles.len();
        let mut blocs: Vec<Vec<u8>> = vec![Vec::new(); nb];
        let mut ecs: Vec<Vec<u8>> = vec![Vec::new(); nb];
        let mut p = 0;
        for i in 0..*tailles.iter().max().unwrap() {
            for (b, &k) in tailles.iter().enumerate() {
                if i < k {
                    blocs[b].push(mots[p]);
                    p += 1;
                }
            }
        }
        for _ in 0..nec {
            for e in ecs.iter_mut() {
                e.push(mots[p]);
                p += 1;
            }
        }
        let mut donnees = Vec::new();
        for (b, e) in blocs.iter().zip(&ecs) {
            if reed_solomon(b, nec) != *e {
                return Err("correction d'erreur incohérente".into());
            }
            donnees.extend_from_slice(b);
        }
        // Flux.
        let bits: Vec<bool> = donnees
            .iter()
            .flat_map(|&o| (0..8).rev().map(move |i| (o >> i) & 1 == 1))
            .collect();
        let mut pos = 0;
        let mut lire = |k: usize| -> u32 {
            let mut v = 0;
            for _ in 0..k {
                v = v << 1 | u32::from(bits[pos]);
                pos += 1;
            }
            v
        };
        let mode = lire(4);
        let mut sortie = Vec::new();
        match mode {
            0b0100 => {
                let len = lire(if version <= 9 { 8 } else { 16 }) as usize;
                for _ in 0..len {
                    sortie.push(lire(8) as u8);
                }
            }
            0b0010 => {
                let len = lire(if version <= 9 { 9 } else { 11 }) as usize;
                for _ in 0..len / 2 {
                    let v = lire(11) as usize;
                    sortie.push(ALNUM[v / 45]);
                    sortie.push(ALNUM[v % 45]);
                }
                if len % 2 == 1 {
                    sortie.push(ALNUM[lire(6) as usize]);
                }
            }
            m => return Err(format!("mode {m:#b} inattendu")),
        }
        Ok(sortie)
    }
}

#[cfg(test)]
mod tests {
    use super::decodeur::decoder;
    use super::*;

    fn bin(s: &str) -> Vec<u8> {
        s.split_whitespace()
            .map(|o| u8::from_str_radix(o, 2).unwrap())
            .collect()
    }

    /// Exemple « 01234567 », version 1-M, de l'annexe de l'ISO/IEC 18004.
    #[test]
    fn vecteur_iso_reed_solomon() {
        let data = bin(
            "00010000 00100000 00001100 01010110 01100001 10000000 11101100 00010001 \
                         11101100 00010001 11101100 00010001 11101100 00010001 11101100 00010001",
        );
        let ec = reed_solomon(&data, 10);
        assert_eq!(ec, bin("10100101 00100100 11010100 11000001 11101101 00110110 11000111 10000111 00101100 01010101"));
    }

    /// « HELLO WORLD », 1-M, alphanumérique : mots de code et correction de référence.
    #[test]
    fn vecteur_hello_world() {
        let mots = mots_de_code(b"HELLO WORLD", 1, true);
        assert_eq!(
            mots,
            vec![32, 91, 11, 120, 209, 114, 220, 77, 67, 64, 236, 17, 236, 17, 236, 17]
        );
        let ec = reed_solomon(&mots, 10);
        assert_eq!(ec, vec![196, 35, 39, 119, 235, 215, 231, 226, 93, 23]);
    }

    #[test]
    fn mots_de_code_bien_formes() {
        for v in 1..=10 {
            let (nec, g) = BLOCS_M[v - 1];
            let nb: usize = g.iter().map(|x| x.0).sum();
            let total = octets_donnees(v) + nb * nec;
            // Capacité brute des mots de code de la norme (hors bits résiduels).
            let attendu = [26, 44, 70, 100, 134, 172, 196, 242, 292, 346][v - 1];
            assert_eq!(total, attendu, "version {v}");
        }
    }

    #[test]
    fn mots_de_format_et_version() {
        // Table de l'ISO 18004 : niveau M, masques 0 à 7.
        let attendu = [
            0b101010000010010,
            0b101000100100101,
            0b101111001111100,
            0b101101101001011,
            0b100010111111001,
            0b100000011001110,
            0b100111110010111,
            0b100101010100000,
        ];
        for (m, &a) in attendu.iter().enumerate() {
            assert_eq!(bch_format(m as u8), a, "masque {m}");
        }
        assert_eq!(bch_version(7), 0b000111110010010100);
        assert_eq!(bch_version(8), 0b001000010110111100);
        assert_eq!(bch_version(9), 0b001001101010011001);
        assert_eq!(bch_version(10), 0b001010010011010011);
    }

    #[test]
    fn dimensions_et_motifs() {
        let q = Qr::encoder(b"HELLO WORLD").unwrap();
        assert_eq!(q.version, 1);
        assert_eq!(q.taille(), 21);
        // Finder en haut à gauche : bord sombre, centre sombre, anneau clair.
        assert!(q.module(0, 0) && q.module(6, 0) && q.module(3, 3));
        assert!(!q.module(1, 1) && !q.module(5, 5));
        // Synchronisation et module sombre.
        assert!(q.module(8, 6) && !q.module(9, 6) && q.module(10, 6));
        assert!(q.module(8, 13));
    }

    #[test]
    fn aller_retour_toutes_les_versions() {
        for v in 1..=10usize {
            // Texte sur-mesure : remplit presque la capacité de la version visée.
            let cap = octets_donnees(v) - if v <= 9 { 2 } else { 3 };
            let texte: Vec<u8> = (0..cap).map(|i| b'a' + (i * 7 % 26) as u8).collect();
            let qr = Qr::encoder(&texte).unwrap();
            assert_eq!(qr.version, v, "version attendue");
            assert_eq!(decoder(&qr).unwrap(), texte, "version {v}");
        }
    }

    #[test]
    fn alphanumerique_et_octet() {
        let a = Qr::encoder(b"HTTPS://ENT.EXAMPLE/V/ABC123-XYZ").unwrap();
        assert_eq!(decoder(&a).unwrap(), b"HTTPS://ENT.EXAMPLE/V/ABC123-XYZ");
        let o = "https://ent.example/v/é?c=Ab9".as_bytes();
        let b = Qr::encoder(o).unwrap();
        assert_eq!(decoder(&b).unwrap(), o);
        // Alphanumérique plus compact : même longueur, version ≤ à l'octet.
        let long_a = vec![b'A'; 150];
        let long_o = vec![b'a'; 150];
        assert!(Qr::encoder(&long_a).unwrap().version < Qr::encoder(&long_o).unwrap().version);
    }

    #[test]
    fn deterministe_et_trop_long() {
        let a = Qr::encoder(b"https://ent.example/v/x").unwrap();
        let b = Qr::encoder(b"https://ent.example/v/x").unwrap();
        assert_eq!(a.modules, b.modules);
        assert_eq!(
            Qr::encoder(&vec![b'a'; 214]).unwrap_err(),
            QrErreur::TropLong
        );
        assert!(Qr::encoder(&vec![b'a'; 213]).is_ok());
    }

    #[test]
    fn le_masque_choisi_est_de_penalite_minimale() {
        let q = Qr::encoder(b"https://ent.example/verifier/3f2a9c").unwrap();
        // Recalcule toutes les pénalités et vérifie que le masque retenu est le premier minimum.
        let version = q.version;
        let alnum = false;
        let mots = mots_entrelaces(
            &mots_de_code(b"https://ent.example/verifier/3f2a9c", version, alnum),
            version,
        );
        assert!(!mots.is_empty());
        let mut g = Grille {
            n: q.taille,
            v: q.modules.clone(),
            f: motifs_fonction(version).f,
        };
        let p0 = penalite(&g);
        // Le masque retenu a au plus la pénalité de n'importe quel autre masque.
        for autre in 0..8u8 {
            if autre == q.masque {
                continue;
            }
            for y in 0..g.n {
                for x in 0..g.n {
                    if !g.f[y * g.n + x] {
                        g.v[y * g.n + x] ^=
                            condition_masque(q.masque, y, x) ^ condition_masque(autre, y, x);
                    }
                }
            }
            ecrire_format(&mut g, autre);
            assert!(
                p0 <= penalite(&g),
                "masque {autre} meilleur que {}",
                q.masque
            );
            // Retour à l'état du masque retenu.
            for y in 0..g.n {
                for x in 0..g.n {
                    if !g.f[y * g.n + x] {
                        g.v[y * g.n + x] ^=
                            condition_masque(q.masque, y, x) ^ condition_masque(autre, y, x);
                    }
                }
            }
            ecrire_format(&mut g, q.masque);
        }
    }
}
