//! Largeurs des polices standard (métriques AFM Adobe, en 1/1000 d'em), indexées par code
//! WinAnsi. Les lettres accentuées ont la largeur de leur lettre de base (c'est le cas dans
//! les AFM de Helvetica et de Times) ; les autres glyphes sont listés un à un.
//! La mesure ne sert qu'à la mise en page : les polices ne sont pas incorporées.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Police {
    Helvetica,
    HelveticaGras,
    Times,
}

impl Police {
    pub(crate) fn nom_base(self) -> &'static str {
        match self {
            Police::Helvetica => "Helvetica",
            Police::HelveticaGras => "Helvetica-Bold",
            Police::Times => "Times-Roman",
        }
    }
    pub(crate) fn indice(self) -> usize {
        match self {
            Police::Helvetica => 0,
            Police::HelveticaGras => 1,
            Police::Times => 2,
        }
    }
}

// Codes 0x20..=0x7e.
const HELV: [u16; 95] = [
    278, 278, 355, 556, 556, 889, 667, 191, 333, 333, 389, 584, 278, 333, 278, 278, // 20-2f
    556, 556, 556, 556, 556, 556, 556, 556, 556, 556, 278, 278, 584, 584, 584, 556, // 30-3f
    1015, 667, 667, 722, 722, 667, 611, 778, 722, 278, 500, 667, 556, 833, 722, 778, // 40-4f
    667, 778, 722, 667, 611, 722, 667, 944, 667, 667, 611, 278, 278, 278, 469, 556, // 50-5f
    333, 556, 556, 500, 556, 556, 278, 556, 556, 222, 222, 500, 222, 833, 556, 556, // 60-6f
    556, 556, 333, 500, 278, 556, 500, 722, 500, 500, 500, 334, 260, 334, 584, // 70-7e
];
const HELV_B: [u16; 95] = [
    278, 333, 474, 556, 556, 889, 722, 238, 333, 333, 389, 584, 278, 333, 278, 278, //
    556, 556, 556, 556, 556, 556, 556, 556, 556, 556, 333, 333, 584, 584, 584, 611, //
    975, 722, 722, 722, 722, 667, 611, 778, 722, 278, 556, 722, 611, 833, 722, 778, //
    667, 778, 722, 667, 611, 722, 667, 944, 667, 667, 611, 333, 278, 333, 584, 556, //
    333, 556, 611, 556, 611, 556, 333, 611, 611, 278, 278, 556, 278, 889, 611, 611, //
    611, 611, 389, 556, 333, 611, 556, 778, 556, 556, 500, 389, 280, 389, 584, //
];
const TIMES: [u16; 95] = [
    250, 333, 408, 500, 500, 833, 778, 180, 333, 333, 500, 564, 250, 333, 250, 278, //
    500, 500, 500, 500, 500, 500, 500, 500, 500, 500, 278, 278, 564, 564, 564, 444, //
    921, 722, 667, 667, 722, 611, 556, 722, 722, 333, 389, 722, 611, 889, 722, 722, //
    556, 722, 667, 556, 611, 722, 722, 944, 722, 722, 611, 333, 278, 333, 469, 500, //
    333, 444, 500, 444, 500, 444, 333, 500, 500, 278, 278, 500, 278, 778, 500, 500, //
    500, 500, 333, 389, 278, 500, 500, 722, 500, 500, 444, 480, 200, 480, 541, //
];

// Codes 0x80..=0x9f (0 = indéfini en WinAnsi).
const HELV_80: [u16; 32] = [
    556, 0, 222, 556, 333, 1000, 556, 556, 333, 1000, 667, 333, 1000, 0, 611, 0, //
    0, 222, 222, 333, 333, 350, 556, 1000, 333, 1000, 500, 333, 944, 0, 500, 667,
];
const HELV_B_80: [u16; 32] = [
    556, 0, 278, 556, 500, 1000, 556, 556, 333, 1000, 667, 333, 1000, 0, 611, 0, //
    0, 278, 278, 500, 500, 350, 556, 1000, 333, 1000, 556, 333, 944, 0, 500, 667,
];
const TIMES_80: [u16; 32] = [
    500, 0, 333, 500, 444, 1000, 500, 500, 333, 1000, 556, 333, 889, 0, 611, 0, //
    0, 333, 333, 444, 444, 350, 500, 1000, 333, 980, 389, 333, 722, 0, 444, 722,
];

// Codes 0xa0..=0xbf.
const HELV_A0: [u16; 32] = [
    278, 333, 556, 556, 556, 556, 260, 556, 333, 737, 370, 556, 584, 333, 737, 333, //
    400, 584, 333, 333, 333, 556, 537, 278, 333, 333, 365, 556, 834, 834, 834, 611,
];
const HELV_B_A0: [u16; 32] = [
    278, 333, 556, 556, 556, 556, 280, 556, 333, 737, 370, 556, 584, 333, 737, 333, //
    400, 584, 333, 333, 333, 611, 556, 278, 333, 333, 365, 556, 834, 834, 834, 611,
];
const TIMES_A0: [u16; 32] = [
    250, 333, 500, 500, 500, 500, 200, 500, 333, 760, 276, 500, 564, 333, 760, 333, //
    400, 564, 300, 300, 333, 500, 453, 250, 333, 300, 310, 500, 750, 750, 750, 444,
];

// Lettre de base des codes 0xc0..=0xff (`?` : glyphe propre, voir `speciale`).
const BASES: &[u8; 64] = b"AAAAAA?CEEEEIIII?NOOOOO??UUUUY??aaaaaa?ceeeeiiii?nooooo??uuuuy?y";

fn speciale(police: Police, code: u8) -> u16 {
    // (Æ, Ð, ×, Ø, Þ, ß, æ, ð, ÷, ø, þ)
    let t: [u16; 11] = match police {
        Police::Helvetica => [1000, 722, 584, 778, 667, 611, 889, 556, 584, 611, 556],
        Police::HelveticaGras => [1000, 722, 584, 778, 667, 611, 889, 611, 584, 611, 611],
        Police::Times => [889, 722, 564, 722, 556, 500, 667, 500, 564, 500, 500],
    };
    match code {
        0xc6 => t[0],
        0xd0 => t[1],
        0xd7 => t[2],
        0xd8 => t[3],
        0xde => t[4],
        0xdf => t[5],
        0xe6 => t[6],
        0xf0 => t[7],
        0xf7 => t[8],
        0xf8 => t[9],
        0xfe => t[10],
        _ => 0,
    }
}

/// Largeur (1/1000 em) d'un code WinAnsi.
pub fn largeur_code(police: Police, code: u8) -> u16 {
    let (ascii, h80, ha0) = match police {
        Police::Helvetica => (&HELV, &HELV_80, &HELV_A0),
        Police::HelveticaGras => (&HELV_B, &HELV_B_80, &HELV_B_A0),
        Police::Times => (&TIMES, &TIMES_80, &TIMES_A0),
    };
    match code {
        0x20..=0x7e => ascii[(code - 0x20) as usize],
        0x80..=0x9f => h80[(code - 0x80) as usize],
        0xa0..=0xbf => ha0[(code - 0xa0) as usize],
        0xc0..=0xff => {
            let b = BASES[(code - 0xc0) as usize];
            if b == b'?' {
                speciale(police, code)
            } else {
                ascii[(b - 0x20) as usize]
            }
        }
        _ => 0,
    }
}

/// Largeur en points d'un texte WinAnsi.
pub fn largeur(police: Police, octets: &[u8], taille: f32) -> f32 {
    let milli: u32 = octets.iter().map(|&b| u32::from(largeur_code(police, b))).sum();
    milli as f32 * taille / 1000.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valeurs_connues() {
        assert_eq!(largeur_code(Police::Helvetica, b'A'), 667);
        assert_eq!(largeur_code(Police::Helvetica, b' '), 278);
        assert_eq!(largeur_code(Police::HelveticaGras, b'a'), 556);
        assert_eq!(largeur_code(Police::Times, b'a'), 444);
        assert_eq!(largeur_code(Police::Times, b'W'), 944);
        assert_eq!(largeur_code(Police::Helvetica, 0xe9), 556, "é = e");
        assert_eq!(largeur_code(Police::Helvetica, 0xc9), 667, "É = E");
        assert_eq!(largeur_code(Police::Helvetica, 0x80), 556, "€");
        assert_eq!(largeur_code(Police::Helvetica, 0x9c), 944, "œ");
        assert_eq!(largeur_code(Police::Helvetica, 0xe7), 500, "ç = c");
        assert_eq!(largeur_code(Police::HelveticaGras, 0x8c), 1000, "Œ");
        assert_eq!(largeur_code(Police::Times, 0xe6), 667, "æ");
        assert_eq!(largeur_code(Police::Helvetica, 0x81), 0, "indéfini");
    }

    #[test]
    fn chaine_de_reference() {
        // « Hello » en Helvetica 10 pt : H722 e556 l222 l222 o556 = 2278 → 22,78 pt.
        let l = largeur(Police::Helvetica, b"Hello", 10.0);
        assert!((l - 22.78).abs() < 1e-3, "{l}");
    }

    #[test]
    fn toute_lettre_francaise_mesurable() {
        for c in "éèêëàâçôùûüïîœŒæÆ«»€–—’…".chars() {
            let o = crate::winansi::octet(c).unwrap();
            for p in [Police::Helvetica, Police::HelveticaGras, Police::Times] {
                assert!(largeur_code(p, o) > 0, "{c} {p:?}");
            }
        }
    }
}
