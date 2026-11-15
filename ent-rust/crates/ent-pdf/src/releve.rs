//! Relecture minimale d'un PDF produit par la brique : contrôle de structure (xref par
//! décalage d'octets) et texte décodé (WinAnsi). Sert aux tests des crates consommatrices ;
//! les fonctions `verifier_*` font échouer le test (panique) si le fichier est invalide.

use crate::depuis_octet;
use std::collections::HashMap;

/// Vérifie la structure : en-tête, xref exacte (chaque entrée pointe sur « n 0 obj »),
/// trailer, `startxref`, `%%EOF`. Retourne le nombre d'objets.
pub fn verifier_structure(pdf: &[u8]) -> usize {
    assert!(pdf.starts_with(b"%PDF-1.4\n"), "en-tête");
    assert!(pdf.ends_with(b"%%EOF\n"), "fin");
    let txt = |a: usize, b: usize| String::from_utf8_lossy(&pdf[a..b]).into_owned();
    let pos_sx = pdf
        .windows(9)
        .rposition(|w| w == b"startxref")
        .expect("startxref");
    let reste = txt(pos_sx + 10, pdf.len());
    let xref: usize = reste
        .lines()
        .next()
        .unwrap()
        .trim()
        .parse()
        .expect("offset xref");
    assert_eq!(&pdf[xref..xref + 5], b"xref\n", "startxref pointe sur xref");
    let corps = txt(xref + 5, pos_sx);
    let mut it = corps.lines();
    let l = it.next().unwrap();
    let (debut, n) = l.split_once(' ').unwrap();
    assert_eq!(debut, "0");
    let n: usize = n.parse().unwrap();
    assert_eq!(it.next().unwrap(), "0000000000 65535 f ");
    for num in 1..n {
        let e = it.next().unwrap();
        assert_eq!(
            e.len(),
            19,
            "entrée xref de 20 octets avec le saut de ligne"
        );
        assert!(e.ends_with(" 00000 n "), "{e}");
        let off: usize = e[..10].parse().unwrap();
        let attendu = format!("{num} 0 obj\n");
        assert_eq!(
            &pdf[off..off + attendu.len()],
            attendu.as_bytes(),
            "objet {num} à l'offset {off}"
        );
    }
    let trailer = it.next().unwrap();
    assert_eq!(trailer, "trailer");
    let t = it.next().unwrap();
    assert!(
        t.contains(&format!("/Size {n}")) && t.contains("/Root 1 0 R"),
        "{t}"
    );
    n - 1
}

/// Décalages des objets d'après la table xref (le fichier doit être valide : voir
/// [`verifier_structure`]).
fn offsets(pdf: &[u8]) -> Vec<usize> {
    let pos_sx = pdf
        .windows(9)
        .rposition(|w| w == b"startxref")
        .expect("startxref");
    let xref: usize = String::from_utf8_lossy(&pdf[pos_sx + 10..])
        .lines()
        .next()
        .unwrap()
        .trim()
        .parse()
        .unwrap();
    let corps = String::from_utf8_lossy(&pdf[xref + 5..pos_sx]).into_owned();
    let mut it = corps.lines();
    let n: usize = it
        .next()
        .unwrap()
        .split_once(' ')
        .unwrap()
        .1
        .parse()
        .unwrap();
    it.next();
    (1..n)
        .map(|_| it.next().unwrap()[..10].parse().unwrap())
        .collect()
}

/// Dictionnaire (ou corps) de l'objet `num`, et flux éventuel.
fn objet<'a>(pdf: &'a [u8], offs: &[usize], num: usize) -> (&'a [u8], Option<&'a [u8]>) {
    let debut = offs[num - 1];
    let fin = find(pdf, b"\nendobj", debut).unwrap();
    let corps = &pdf[debut..fin];
    match find(corps, b"stream\n", 0) {
        Some(p) => {
            let longueur = {
                let k = find(corps, b"/Length ", 0).unwrap() + 8;
                let f = corps[k..].iter().position(|b| !b.is_ascii_digit()).unwrap();
                std::str::from_utf8(&corps[k..k + f])
                    .unwrap()
                    .parse::<usize>()
                    .unwrap()
            };
            (&corps[..p], Some(&corps[p + 7..p + 7 + longueur]))
        }
        None => (corps, None),
    }
}

/// Numéro cible de la référence `/Cle n 0 R`.
fn reference(d: &[u8], cle: &str) -> Option<usize> {
    let p = find(d, cle.as_bytes(), 0)? + cle.len();
    let t = std::str::from_utf8(&d[p..]).ok()?;
    t.trim_start().split(' ').next()?.parse().ok()
}

/// Table CID → caractère d'une CMap `ToUnicode` (blocs `bfchar` uniquement, ceux que nous écrivons).
fn lire_to_unicode(flux: &[u8]) -> HashMap<u16, String> {
    let t = String::from_utf8_lossy(flux).into_owned();
    let mut m = HashMap::new();
    let mut dedans = false;
    for l in t.lines() {
        if l.ends_with("beginbfchar") {
            dedans = true;
        } else if l == "endbfchar" {
            dedans = false;
        } else if dedans {
            let v: Vec<&str> = l
                .split(['<', '>'])
                .filter(|x| !x.trim().is_empty())
                .collect();
            let cid = u16::from_str_radix(v[0], 16).unwrap();
            let u: Vec<u16> = (0..v[1].len() / 4)
                .map(|i| u16::from_str_radix(&v[1][4 * i..4 * i + 4], 16).unwrap())
                .collect();
            m.insert(cid, String::from_utf16(&u).unwrap());
        }
    }
    m
}

/// Table (nom de ressource → CMap ToUnicode) des polices incorporées d'une page.
fn polices_de_page(
    pdf: &[u8],
    offs: &[usize],
    page: &[u8],
) -> HashMap<String, HashMap<u16, String>> {
    let mut r = HashMap::new();
    let Some(p) = find(page, b"/Font << ", 0) else {
        return r;
    };
    let fin = find(page, b">>", p).unwrap();
    let jetons: Vec<&str> = std::str::from_utf8(&page[p + 9..fin])
        .unwrap()
        .split_whitespace()
        .collect();
    for tri in jetons.chunks(4) {
        if tri.len() < 4 {
            break;
        }
        let nom = tri[0].trim_start_matches('/').to_string();
        let (d, _) = objet(pdf, offs, tri[1].parse().unwrap());
        if let Some(u) = reference(d, "/ToUnicode ") {
            let (_, flux) = objet(pdf, offs, u);
            r.insert(nom, lire_to_unicode(flux.unwrap()));
        }
    }
    r
}

/// Texte de chaque page : chaînes `( … ) Tj` (polices standard, WinAnsi) et `<…> Tj` (police
/// incorporée, décodée par la `ToUnicode`), séparées par des sauts de ligne.
pub fn texte_des_pages(pdf: &[u8]) -> Vec<String> {
    let offs = offsets(pdf);
    let mut pages = Vec::new();
    for num in 1..=offs.len() {
        let (d, _) = objet(pdf, &offs, num);
        if find(d, b"/Type /Page ", 0).is_none() {
            continue;
        }
        let polices = polices_de_page(pdf, &offs, d);
        let contenu = reference(d, "/Contents ").expect("/Contents");
        let flux = objet(pdf, &offs, contenu).1.expect("flux de contenu");
        pages.push(texte_du_flux(flux, &polices));
    }
    pages
}

fn texte_du_flux(flux: &[u8], polices: &HashMap<String, HashMap<u16, String>>) -> String {
    let mut out = String::new();
    let mut courante: Option<&HashMap<u16, String>> = None;
    let mut k = 0;
    while k < flux.len() {
        match flux[k] {
            b'/' => {
                let f = flux[k + 1..].iter().position(|b| *b == b' ').unwrap_or(0);
                let nom = String::from_utf8_lossy(&flux[k + 1..k + 1 + f]).into_owned();
                if flux[k + 1 + f..].starts_with(b" ")
                    && flux[k + 2 + f..].iter().take(12).any(|&b| b == b'T')
                {
                    courante = polices.get(&nom);
                }
            }
            b'<' if courante.is_some() && flux.get(k + 1) != Some(&b'<') => {
                let f = flux[k..].iter().position(|b| *b == b'>').unwrap();
                let hex = std::str::from_utf8(&flux[k + 1..k + f]).unwrap();
                let cmap = courante.unwrap();
                for i in 0..hex.len() / 4 {
                    let cid = u16::from_str_radix(&hex[4 * i..4 * i + 4], 16).unwrap();
                    out.push_str(cmap.get(&cid).map(String::as_str).unwrap_or("\u{fffd}"));
                }
                out.push('\n');
                k += f;
            }
            b'(' => {
                k += 1;
                let mut nest = 0;
                while k < flux.len() {
                    match flux[k] {
                        b'\\' => {
                            let c = flux[k + 1];
                            if c.is_ascii_digit() {
                                let o = std::str::from_utf8(&flux[k + 1..k + 4]).unwrap();
                                out.push(depuis_octet(u8::from_str_radix(o, 8).unwrap()));
                                k += 4;
                            } else {
                                out.push(c as char);
                                k += 2;
                            }
                            continue;
                        }
                        b'(' => {
                            nest += 1;
                            out.push('(');
                        }
                        b')' if nest == 0 => break,
                        b')' => {
                            nest -= 1;
                            out.push(')');
                        }
                        b => out.push(depuis_octet(b)),
                    }
                    k += 1;
                }
                out.push('\n');
            }
            _ => {}
        }
        k += 1;
    }
    out
}

pub fn find(h: &[u8], a: &[u8], depuis: usize) -> Option<usize> {
    h[depuis..]
        .windows(a.len())
        .position(|w| w == a)
        .map(|p| p + depuis)
}

/// Tout le texte du document (pages concaténées).
pub fn texte(pdf: &[u8]) -> String {
    texte_des_pages(pdf).join("")
}

pub fn contient(pdf: &[u8], a: &str) -> bool {
    pdf.windows(a.len()).any(|w| w == a.as_bytes())
}
