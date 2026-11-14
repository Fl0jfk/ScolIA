//! Relecture minimale d'un PDF produit par la brique : contrôle de structure (xref par
//! décalage d'octets) et texte décodé (WinAnsi). Sert aux tests des crates consommatrices ;
//! les fonctions `verifier_*` font échouer le test (panique) si le fichier est invalide.

use crate::depuis_octet;

/// Vérifie la structure : en-tête, xref exacte (chaque entrée pointe sur « n 0 obj »),
/// trailer, `startxref`, `%%EOF`. Retourne le nombre d'objets.
pub fn verifier_structure(pdf: &[u8]) -> usize {
    assert!(pdf.starts_with(b"%PDF-1.4\n"), "en-tête");
    assert!(pdf.ends_with(b"%%EOF\n"), "fin");
    let txt = |a: usize, b: usize| String::from_utf8_lossy(&pdf[a..b]).into_owned();
    let pos_sx = pdf.windows(9).rposition(|w| w == b"startxref").expect("startxref");
    let reste = txt(pos_sx + 10, pdf.len());
    let xref: usize = reste.lines().next().unwrap().trim().parse().expect("offset xref");
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
        assert_eq!(e.len(), 19, "entrée xref de 20 octets avec le saut de ligne");
        assert!(e.ends_with(" 00000 n "), "{e}");
        let off: usize = e[..10].parse().unwrap();
        let attendu = format!("{num} 0 obj\n");
        assert_eq!(&pdf[off..off + attendu.len()], attendu.as_bytes(), "objet {num} à l'offset {off}");
    }
    let trailer = it.next().unwrap();
    assert_eq!(trailer, "trailer");
    let t = it.next().unwrap();
    assert!(t.contains(&format!("/Size {n}")) && t.contains("/Root 1 0 R"), "{t}");
    n - 1
}

/// Texte de chaque page : chaînes `( … ) Tj` décodées (WinAnsi), séparées par des sauts de ligne.
pub fn texte_des_pages(pdf: &[u8]) -> Vec<String> {
    let mut pages = Vec::new();
    let mut i = 0;
    while let Some(p) = find(pdf, b"stream\n", i) {
        let fin = find(pdf, b"\nendstream", p).unwrap();
        let flux = &pdf[p + 7..fin];
        i = fin + 10;
        // Seuls les flux de contenu (texte) nous intéressent : ils contiennent « BT ».
        if !flux.windows(2).any(|w| w == b"BT") && !flux.windows(3).any(|w| w == b" l\n") {
            continue;
        }
        if flux.starts_with(&[0xff, 0xd8]) {
            continue;
        }
        let mut out = String::new();
        let mut k = 0;
        while k < flux.len() {
            if flux[k] == b'(' {
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
            k += 1;
        }
        pages.push(out);
    }
    pages
}

pub fn find(h: &[u8], a: &[u8], depuis: usize) -> Option<usize> {
    h[depuis..].windows(a.len()).position(|w| w == a).map(|p| p + depuis)
}

/// Tout le texte du document (pages concaténées).
pub fn texte(pdf: &[u8]) -> String {
    texte_des_pages(pdf).join("")
}

pub fn contient(pdf: &[u8], a: &str) -> bool {
    pdf.windows(a.len()).any(|w| w == a.as_bytes())
}
