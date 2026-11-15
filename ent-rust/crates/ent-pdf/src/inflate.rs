//! Décompression `zlib` (RFC 1950) et `deflate` (RFC 1951), maison (D-062 : aucune dépendance).
//!
//! Écrit pour lire des données **non fiables** (flux d'objets d'un PDF reçu) :
//! * la taille décompressée est **plafonnée**, et la sortie s'arrête dès que le plafond serait
//!   dépassé (une bombe de décompression ne remplit jamais la mémoire) ;
//! * aucune récursion, aucun `unsafe`, aucun indice non vérifié ;
//! * un flux tronqué, un code de Huffman invalide, une distance hors fenêtre ou une somme
//!   Adler-32 fausse sont des erreurs, jamais des données partielles.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErreurInflate {
    /// Le flux s'arrête avant sa fin.
    Tronque,
    /// Structure invalide (en-tête, code de Huffman, distance, longueurs de bloc…).
    Corrompu(&'static str),
    /// La sortie dépasserait le plafond demandé.
    Plafond,
    /// Somme de contrôle Adler-32 fausse.
    Adler,
}

impl std::fmt::Display for ErreurInflate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Tronque => f.write_str("flux compressé tronqué"),
            Self::Corrompu(m) => write!(f, "flux compressé corrompu ({m})"),
            Self::Plafond => f.write_str("flux compressé trop volumineux une fois décompressé"),
            Self::Adler => f.write_str("flux compressé : somme de contrôle fausse"),
        }
    }
}
impl std::error::Error for ErreurInflate {}

struct Bits<'a> {
    d: &'a [u8],
    pos: usize,
    tampon: u32,
    nb: u32,
}

impl<'a> Bits<'a> {
    fn new(d: &'a [u8]) -> Self {
        Self {
            d,
            pos: 0,
            tampon: 0,
            nb: 0,
        }
    }
    /// Lit `n` bits (0 ≤ n ≤ 16), poids faible d'abord.
    fn bits(&mut self, n: u32) -> Result<u32, ErreurInflate> {
        while self.nb < n {
            let o = *self.d.get(self.pos).ok_or(ErreurInflate::Tronque)?;
            self.pos += 1;
            self.tampon |= u32::from(o) << self.nb;
            self.nb += 8;
        }
        let v = self.tampon & ((1u32 << n) - 1);
        self.tampon >>= n;
        self.nb -= n;
        Ok(v)
    }
    /// Abandonne les bits restants de l'octet courant.
    fn aligner(&mut self) {
        self.tampon = 0;
        self.nb = 0;
    }
}

const MAX_BITS: usize = 15;

/// Code de Huffman canonique : nombre de codes par longueur, symboles triés.
struct Huffman {
    compte: [u16; MAX_BITS + 1],
    symbole: Vec<u16>,
}

impl Huffman {
    fn new(longueurs: &[u8]) -> Result<Self, ErreurInflate> {
        let mut compte = [0u16; MAX_BITS + 1];
        for &l in longueurs {
            compte[usize::from(l)] += 1;
        }
        compte[0] = 0;
        // Sur-souscription : plus de codes que la longueur n'en permet.
        let mut restant: i32 = 1;
        for &c in &compte[1..] {
            restant <<= 1;
            restant -= i32::from(c);
            if restant < 0 {
                return Err(ErreurInflate::Corrompu("code de Huffman sur-souscrit"));
            }
        }
        let mut offs = [0u16; MAX_BITS + 2];
        for l in 1..=MAX_BITS {
            offs[l + 1] = offs[l] + compte[l];
        }
        let mut symbole = vec![0u16; longueurs.len()];
        for (s, &l) in longueurs.iter().enumerate() {
            if l != 0 {
                symbole[usize::from(offs[usize::from(l)])] = s as u16;
                offs[usize::from(l)] += 1;
            }
        }
        Ok(Self { compte, symbole })
    }

    fn decoder(&self, b: &mut Bits) -> Result<u16, ErreurInflate> {
        let (mut code, mut premier, mut index) = (0i32, 0i32, 0i32);
        for l in 1..=MAX_BITS {
            code |= b.bits(1)? as i32;
            let c = i32::from(self.compte[l]);
            if code - c < premier {
                return self
                    .symbole
                    .get((index + (code - premier)) as usize)
                    .copied()
                    .ok_or(ErreurInflate::Corrompu("symbole hors table"));
            }
            index += c;
            premier += c;
            premier <<= 1;
            code <<= 1;
        }
        Err(ErreurInflate::Corrompu("code de Huffman invalide"))
    }
}

const LONG_BASE: [u16; 29] = [
    3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115, 131,
    163, 195, 227, 258,
];
const LONG_EXTRA: [u8; 29] = [
    0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0,
];
const DIST_BASE: [u16; 30] = [
    1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025, 1537,
    2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577,
];
const DIST_EXTRA: [u8; 30] = [
    0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13,
    13,
];
const ORDRE_LONGUEURS: [usize; 19] = [
    16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15,
];

fn bloc_codes(
    b: &mut Bits,
    sortie: &mut Vec<u8>,
    plafond: usize,
    lit: &Huffman,
    dist: &Huffman,
) -> Result<(), ErreurInflate> {
    loop {
        let s = usize::from(lit.decoder(b)?);
        match s {
            0..=255 => {
                if sortie.len() >= plafond {
                    return Err(ErreurInflate::Plafond);
                }
                sortie.push(s as u8);
            }
            256 => return Ok(()),
            257..=285 => {
                let i = s - 257;
                let lon = usize::from(LONG_BASE[i]) + b.bits(u32::from(LONG_EXTRA[i]))? as usize;
                let ds = usize::from(dist.decoder(b)?);
                if ds >= 30 {
                    return Err(ErreurInflate::Corrompu("code de distance invalide"));
                }
                let d = usize::from(DIST_BASE[ds]) + b.bits(u32::from(DIST_EXTRA[ds]))? as usize;
                if d > sortie.len() {
                    return Err(ErreurInflate::Corrompu("distance hors fenêtre"));
                }
                if sortie.len() + lon > plafond {
                    return Err(ErreurInflate::Plafond);
                }
                let debut = sortie.len() - d;
                for k in 0..lon {
                    let o = sortie[debut + k];
                    sortie.push(o);
                }
            }
            _ => return Err(ErreurInflate::Corrompu("symbole de longueur invalide")),
        }
    }
}

fn tables_fixes() -> Result<(Huffman, Huffman), ErreurInflate> {
    let mut l = [0u8; 288];
    for (i, x) in l.iter_mut().enumerate() {
        *x = match i {
            0..=143 => 8,
            144..=255 => 9,
            256..=279 => 7,
            _ => 8,
        };
    }
    Ok((Huffman::new(&l)?, Huffman::new(&[5u8; 30])?))
}

fn tables_dynamiques(b: &mut Bits) -> Result<(Huffman, Huffman), ErreurInflate> {
    let nlen = b.bits(5)? as usize + 257;
    let ndist = b.bits(5)? as usize + 1;
    let ncode = b.bits(4)? as usize + 4;
    if nlen > 286 || ndist > 30 {
        return Err(ErreurInflate::Corrompu("trop de codes"));
    }
    let mut lc = [0u8; 19];
    for &o in ORDRE_LONGUEURS.iter().take(ncode) {
        lc[o] = b.bits(3)? as u8;
    }
    let hc = Huffman::new(&lc)?;
    let mut l = vec![0u8; nlen + ndist];
    let mut i = 0;
    while i < nlen + ndist {
        let s = hc.decoder(b)?;
        if s < 16 {
            l[i] = s as u8;
            i += 1;
        } else {
            let (v, rep) = match s {
                16 => {
                    if i == 0 {
                        return Err(ErreurInflate::Corrompu("répétition sans précédent"));
                    }
                    (l[i - 1], 3 + b.bits(2)? as usize)
                }
                17 => (0, 3 + b.bits(3)? as usize),
                _ => (0, 11 + b.bits(7)? as usize),
            };
            if i + rep > nlen + ndist {
                return Err(ErreurInflate::Corrompu("répétition trop longue"));
            }
            l[i..i + rep].fill(v);
            i += rep;
        }
    }
    if l[256] == 0 {
        return Err(ErreurInflate::Corrompu("pas de code de fin de bloc"));
    }
    Ok((Huffman::new(&l[..nlen])?, Huffman::new(&l[nlen..])?))
}

/// Décompresse un flux `deflate` brut. Renvoie les données et le nombre d'octets d'entrée lus.
pub fn deflate_brut(entree: &[u8], plafond: usize) -> Result<(Vec<u8>, usize), ErreurInflate> {
    let mut b = Bits::new(entree);
    let mut sortie = Vec::new();
    loop {
        let dernier = b.bits(1)?;
        match b.bits(2)? {
            0 => {
                b.aligner();
                let p = b.pos;
                let h = entree.get(p..p + 4).ok_or(ErreurInflate::Tronque)?;
                let len = usize::from(u16::from_le_bytes([h[0], h[1]]));
                let nlen = u16::from_le_bytes([h[2], h[3]]);
                if nlen != !(len as u16) {
                    return Err(ErreurInflate::Corrompu("bloc stocké : longueur incohérente"));
                }
                let corps = entree
                    .get(p + 4..p + 4 + len)
                    .ok_or(ErreurInflate::Tronque)?;
                if sortie.len() + len > plafond {
                    return Err(ErreurInflate::Plafond);
                }
                sortie.extend_from_slice(corps);
                b.pos = p + 4 + len;
            }
            1 => {
                let (l, d) = tables_fixes()?;
                bloc_codes(&mut b, &mut sortie, plafond, &l, &d)?;
            }
            2 => {
                let (l, d) = tables_dynamiques(&mut b)?;
                bloc_codes(&mut b, &mut sortie, plafond, &l, &d)?;
            }
            _ => return Err(ErreurInflate::Corrompu("type de bloc réservé")),
        }
        if dernier == 1 {
            break;
        }
    }
    // Les bits entiers non consommés du tampon n'existent pas : on ne lit qu'octet par octet
    // et `bits` ne charge jamais plus que nécessaire.
    Ok((sortie, b.pos))
}

fn adler32(d: &[u8]) -> u32 {
    let (mut a, mut b) = (1u32, 0u32);
    for bloc in d.chunks(5552) {
        for &o in bloc {
            a += u32::from(o);
            b += a;
        }
        a %= 65521;
        b %= 65521;
    }
    (b << 16) | a
}

/// Décompresse un flux `zlib` complet (en-tête, deflate, Adler-32), sortie plafonnée à `plafond` octets.
pub fn zlib(entree: &[u8], plafond: usize) -> Result<Vec<u8>, ErreurInflate> {
    if entree.len() < 2 {
        return Err(ErreurInflate::Tronque);
    }
    let (cmf, flg) = (entree[0], entree[1]);
    if cmf & 0x0f != 8 || cmf >> 4 > 7 {
        return Err(ErreurInflate::Corrompu("en-tête zlib"));
    }
    if (u16::from(cmf) * 256 + u16::from(flg)) % 31 != 0 {
        return Err(ErreurInflate::Corrompu("en-tête zlib"));
    }
    if flg & 0x20 != 0 {
        return Err(ErreurInflate::Corrompu("dictionnaire prédéfini"));
    }
    let (sortie, lus) = deflate_brut(&entree[2..], plafond)?;
    let fin = 2 + lus;
    let a = entree.get(fin..fin + 4).ok_or(ErreurInflate::Tronque)?;
    if u32::from_be_bytes([a[0], a[1], a[2], a[3]]) != adler32(&sortie) {
        return Err(ErreurInflate::Adler);
    }
    Ok(sortie)
}

/// Annule un prédicteur PNG (`/Predictor` 10 à 15) : `colonnes` × `couleurs` × `bpc` bits par ligne.
pub fn predicteur_png(
    d: &[u8],
    colonnes: usize,
    couleurs: usize,
    bpc: usize,
) -> Result<Vec<u8>, ErreurInflate> {
    let ligne = colonnes
        .checked_mul(couleurs)
        .and_then(|x| x.checked_mul(bpc))
        .map(|x| x.div_ceil(8))
        .filter(|&x| x > 0 && x <= 1 << 24)
        .ok_or(ErreurInflate::Corrompu("prédicteur : paramètres"))?;
    let pas = (couleurs * bpc).div_ceil(8).max(1);
    let mut sortie: Vec<u8> = Vec::with_capacity(d.len());
    let mut prec = vec![0u8; ligne];
    for rang in d.chunks(ligne + 1) {
        let (&f, brut) = rang.split_first().ok_or(ErreurInflate::Tronque)?;
        if brut.len() != ligne {
            return Err(ErreurInflate::Tronque);
        }
        let mut cur = vec![0u8; ligne];
        for i in 0..ligne {
            let a = if i >= pas { cur[i - pas] } else { 0 };
            let b = prec[i];
            let c = if i >= pas { prec[i - pas] } else { 0 };
            let p = match f {
                0 => 0,
                1 => a,
                2 => b,
                3 => ((u16::from(a) + u16::from(b)) / 2) as u8,
                4 => {
                    let (ia, ib, ic) = (i32::from(a), i32::from(b), i32::from(c));
                    let pp = ia + ib - ic;
                    let (pa, pb, pc) = ((pp - ia).abs(), (pp - ib).abs(), (pp - ic).abs());
                    if pa <= pb && pa <= pc {
                        a
                    } else if pb <= pc {
                        b
                    } else {
                        c
                    }
                }
                _ => return Err(ErreurInflate::Corrompu("prédicteur : filtre inconnu")),
            };
            cur[i] = brut[i].wrapping_add(p);
        }
        sortie.extend_from_slice(&cur);
        prec = cur;
    }
    Ok(sortie)
}

#[cfg(test)]
pub(crate) mod fixtures {
    //! Encodeurs de **test** : fabriquent des flux `zlib` valides sans compresseur.
    pub fn adler(d: &[u8]) -> u32 {
        super::adler32(d)
    }

    /// `zlib` en blocs stockés.
    pub fn zlib_stocke(d: &[u8]) -> Vec<u8> {
        let mut o = vec![0x78, 0x01];
        let mut blocs = d.chunks(65535).peekable();
        if blocs.peek().is_none() {
            o.extend_from_slice(&[1, 0, 0, 0xff, 0xff]);
        }
        while let Some(b) = blocs.next() {
            o.push(u8::from(blocs.peek().is_none()));
            o.extend_from_slice(&(b.len() as u16).to_le_bytes());
            o.extend_from_slice(&(!(b.len() as u16)).to_le_bytes());
            o.extend_from_slice(b);
        }
        o.extend_from_slice(&adler(d).to_be_bytes());
        o
    }

    pub struct Ecrit {
        pub o: Vec<u8>,
        pub t: u32,
        pub n: u32,
    }
    impl Ecrit {
        pub fn bits(&mut self, v: u32, n: u32) {
            self.t |= v << self.n;
            self.n += n;
            while self.n >= 8 {
                self.o.push(self.t as u8);
                self.t >>= 8;
                self.n -= 8;
            }
        }
        /// Code de Huffman : bit de poids fort d'abord.
        pub fn code(&mut self, v: u32, n: u32) {
            for i in (0..n).rev() {
                self.bits((v >> i) & 1, 1);
            }
        }
    }

    /// Bombe : `zlib` en Huffman fixe, un octet nul puis `paires` répétitions de 258 octets
    /// (≈ 13 bits pour 258 octets). La taille décompressée est `1 + 258 × paires`.
    pub fn bombe(paires: usize) -> Vec<u8> {
        let mut e = Ecrit {
            o: vec![0x78, 0x01],
            t: 0,
            n: 0,
        };
        e.bits(1, 1);
        e.bits(1, 2);
        e.code(0x30, 8); // littéral 0
        for _ in 0..paires {
            e.code(0xc5, 8); // symbole 285 : longueur 258
            e.code(0, 5); // distance 1
        }
        e.code(0, 7); // fin de bloc
        if e.n > 0 {
            let r = 8 - e.n;
            e.bits(0, r);
        }
        let mut sortie = e.o;
        let mut plein = vec![0u8];
        plein.resize(1 + 258 * paires, 0);
        sortie.extend_from_slice(&adler(&plein).to_be_bytes());
        sortie
    }
}

#[cfg(test)]
mod tests {
    use super::fixtures::*;
    use super::*;

    /// Texte compressé par zlib (niveau 9) : bloc **dynamique**. Produit localement par `zlib.compress`.
    const TEXTE: &str = "Le bulletin de Camille Durand pour le premier trimestre : mathématiques, \
        français, histoire-géographie, sciences, éducation physique. Le bulletin de Camille \
        Durand pour le premier trimestre : mathématiques, français, histoire-géographie.";
    const TEXTE_Z: &str = "78dabd8ecb0dc3300c4357e1006906e8b53d6609d5566301f1a7b27de84485e7f0625196e88520013e821be3d58f839b2478c683a258c2b32b258f92bbc262518ec28aa612b93665dc11a985394ce5d3b92e781b317f246683d49645f9b6cf9177a512841754279cdc559dc37767604e28e15baf8115dbbf9eac27ae755e97";

    fn depuis_hex(h: &str) -> Vec<u8> {
        let h: String = h.split_whitespace().collect();
        (0..h.len() / 2)
            .map(|i| u8::from_str_radix(&h[2 * i..2 * i + 2], 16).unwrap())
            .collect()
    }

    #[test]
    fn stocke_aller_retour() {
        for d in [&b""[..], b"a", b"bonjour le monde"] {
            assert_eq!(zlib(&zlib_stocke(d), 1000).unwrap(), d);
        }
        let grand = vec![7u8; 70_000];
        assert_eq!(zlib(&zlib_stocke(&grand), 100_000).unwrap(), grand);
    }

    #[test]
    fn adler_connu() {
        assert_eq!(adler32(b"Wikipedia"), 0x11E6_0398);
    }

    #[test]
    fn huffman_fixe_et_plafond() {
        let z = bombe(100);
        let v = zlib(&z, usize::MAX).unwrap();
        assert_eq!(v.len(), 1 + 258 * 100);
        assert!(v.iter().all(|&o| o == 0));
        // Plafond exact accepté, plafond d'un octet de moins refusé.
        assert!(zlib(&z, 1 + 258 * 100).is_ok());
        assert_eq!(zlib(&z, 1 + 258 * 100 - 1), Err(ErreurInflate::Plafond));
        // Bombe : 1 Mio décompressés refusés sous un plafond de 64 Kio, sortie jamais construite en entier.
        assert_eq!(zlib(&bombe(4100), 65_536), Err(ErreurInflate::Plafond));
    }

    #[test]
    fn huffman_dynamique_de_zlib() {
        // Le flux de référence n'est pas stocké ni fixe : on le vérifie, puis on le relit.
        let z = depuis_hex(TEXTE_Z);
        let type_bloc = (z[2] >> 1) & 3;
        assert_eq!(type_bloc, 2, "bloc dynamique attendu");
        assert_eq!(zlib(&z, 10_000).unwrap(), TEXTE.as_bytes());
    }

    #[test]
    fn flux_tronque_ou_corrompu() {
        let z = depuis_hex(TEXTE_Z);
        for coupe in [0, 1, 2, 5, z.len() / 2, z.len() - 1] {
            assert!(zlib(&z[..coupe], 10_000).is_err(), "coupé à {coupe}");
        }
        // Adler faux.
        let mut a = z.clone();
        let n = a.len();
        a[n - 1] ^= 1;
        assert_eq!(zlib(&a, 10_000), Err(ErreurInflate::Adler));
        // En-tête invalide, dictionnaire, type de bloc réservé.
        assert!(zlib(&[0x79, 0x01, 0, 0], 10).is_err());
        assert!(zlib(&[0x78, 0xbb, 0, 0], 10).is_err());
        assert!(zlib(&[0x78, 0x01, 0x07, 0, 0, 0, 0], 10).is_err());
        // Distance hors fenêtre : bloc fixe, littéral 0, puis longueur 3 distance 2.
        let mut e = Ecrit {
            o: vec![0x78, 0x01],
            t: 0,
            n: 0,
        };
        e.bits(1, 1);
        e.bits(1, 2);
        e.code(0x30, 8);
        e.code(1, 7); // symbole 257 (longueur 3)
        e.code(1, 5); // distance 2 > 1 octet écrit
        e.code(0, 7);
        e.bits(0, 8 - e.n);
        assert!(matches!(
            zlib(&e.o, 100),
            Err(ErreurInflate::Corrompu(m)) if m.contains("fenêtre")
        ));
        // Octets quelconques : jamais de panique.
        let mut x = 12345u32;
        for _ in 0..300 {
            let mut v = vec![0x78, 0x9c];
            for _ in 0..40 {
                x = x.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                v.push((x >> 24) as u8);
            }
            let _ = zlib(&v, 4096);
        }
    }

    #[test]
    fn predicteur_png_aller_retour() {
        // Deux lignes de 3 colonnes, filtres Up puis Sub.
        let brut = [2u8, 1, 2, 3, 1, 10, 1, 1];
        let v = predicteur_png(&brut, 3, 1, 8).unwrap();
        assert_eq!(v, vec![1, 2, 3, 10, 11, 12]);
        assert!(predicteur_png(&[9, 1, 2, 3], 3, 1, 8).is_err());
        assert!(predicteur_png(&[0, 1, 2], 3, 1, 8).is_err());
        assert!(predicteur_png(&[0], 0, 1, 8).is_err());
    }
}

