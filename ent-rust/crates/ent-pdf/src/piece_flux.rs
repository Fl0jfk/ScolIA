//! PDF 1.5 et suivants : flux d'objets (`/Type /ObjStm`) et table xref en flux (`/Type /XRef`).
//!
//! Word, les imprimantes virtuelles et la plupart des scanners en produisent. On ne peut pas
//! neutraliser « sur place » un nom actif caché dans un flux compressé (la longueur changerait,
//! donc la table xref). On **reconstruit** donc un fichier équivalent :
//!
//! 1. on parcourt le fichier **sans suivre** ni la table xref ni l'arbre des pages (aucune boucle
//!    d'objets possible : un seul passage linéaire, nombre d'objets borné) ;
//! 2. chaque objet « direct » est recopié tel quel ; chaque flux d'objets est décompressé
//!    (`inflate`, plafond cumulé) et ses objets sont écrits à plat ; en cas de doublon de numéro,
//!    le dernier dans le fichier l'emporte (mises à jour incrémentales) ;
//! 3. les flux xref sont **abandonnés** et une table xref classique est réécrite : aucune référence
//!    vers un objet compressé que l'on n'aurait pas lu ne survit ;
//! 4. les autres flux (images, pages, polices) sont recopiés sans être touchés.
//!
//! Le résultat n'a plus d'objet compressé : le désarmement habituel s'applique ensuite.
//! Les PDF chiffrés sont refusés avant d'arriver ici.

use crate::inflate::{self, ErreurInflate};
use crate::piece::{delimiteur, noms_hors_flux, regions_flux, trouver, ErreurPiece};
use std::collections::BTreeMap;

const OBJETS_MAX: usize = 200_000;
const NUMERO_MAX: u32 = 500_000;
const ENTETE: &str = "PDF compressé non inspectable : enregistrez-le en PDF simple, ou prenez une photo";

fn refus(m: &'static str) -> ErreurPiece {
    ErreurPiece::PdfRefuse(m)
}

/// Un objet repéré : numéro, génération, étendue du corps (entre `obj` et `endobj`).
struct Repere {
    num: u32,
    gen: u16,
    debut: usize,
    fin: usize,
}

fn entier(b: &[u8]) -> Option<u32> {
    if b.is_empty() || b.len() > 10 || !b.iter().all(u8::is_ascii_digit) {
        return None;
    }
    std::str::from_utf8(b).ok()?.parse().ok()
}

/// Repère les objets `N G obj … endobj` hors flux binaires, chaînes et commentaires.
fn reperer(b: &[u8]) -> Vec<Repere> {
    let flux = regions_flux(b);
    let mut r = 0;
    let mut i = 0;
    let mut precedents: [(usize, usize); 2] = [(0, 0); 2];
    let mut en_cours: Option<(u32, u16, usize)> = None;
    let mut sortie = Vec::new();
    while i < b.len() {
        while r < flux.len() && flux[r].1 <= i {
            r += 1;
        }
        if r < flux.len() && i >= flux[r].0 {
            i = flux[r].1;
            continue;
        }
        match b[i] {
            b'%' => {
                while i < b.len() && b[i] != b'\n' && b[i] != b'\r' {
                    i += 1;
                }
            }
            b'(' => {
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
            }
            b'/' => {
                i += 1;
                while i < b.len() && !delimiteur(b[i]) {
                    i += 1;
                }
            }
            c if delimiteur(c) => i += 1,
            _ => {
                let d = i;
                while i < b.len() && !delimiteur(b[i]) {
                    i += 1;
                }
                let mot = &b[d..i];
                if mot == b"obj" {
                    let n = entier(&b[precedents[0].0..precedents[0].1]);
                    let g = entier(&b[precedents[1].0..precedents[1].1]);
                    if let (Some(n), Some(g)) = (n, g) {
                        if g <= 65535 {
                            en_cours = Some((n, g as u16, i));
                        }
                    }
                } else if mot == b"endobj" {
                    if let Some((num, gen, debut)) = en_cours.take() {
                        sortie.push(Repere {
                            num,
                            gen,
                            debut,
                            fin: d,
                        });
                    }
                }
                precedents = [precedents[1], (d, i)];
            }
        }
    }
    sortie
}

/// Valeur entière directe qui suit le nom `/cle` (jamais une référence `N G R`).
fn entier_apres(corps: &[u8], noms: &[(usize, usize, String)], cle: &str) -> Option<usize> {
    let (_, fin, _) = noms.iter().find(|(_, _, n)| n == cle)?;
    let mut i = *fin;
    while i < corps.len() && corps[i].is_ascii_whitespace() {
        i += 1;
    }
    let d = i;
    while i < corps.len() && corps[i].is_ascii_digit() {
        i += 1;
    }
    let v = entier(&corps[d..i])? as usize;
    // Référence indirecte ? `12 0 R`.
    let mut j = i;
    while j < corps.len() && corps[j].is_ascii_whitespace() {
        j += 1;
    }
    let k = j;
    while j < corps.len() && corps[j].is_ascii_digit() {
        j += 1;
    }
    if j > k {
        let mut m = j;
        while m < corps.len() && corps[m].is_ascii_whitespace() {
            m += 1;
        }
        if corps.get(m) == Some(&b'R') {
            return None;
        }
    }
    Some(v)
}

fn type_est(noms: &[(usize, usize, String)], valeur: &str) -> bool {
    noms.windows(2)
        .any(|w| w[0].2 == "Type" && w[1].2 == valeur)
}

/// Filtres déclarés : liste des noms après `/Filter` (nom seul ou tableau).
fn filtres(corps: &[u8], noms: &[(usize, usize, String)]) -> Vec<String> {
    let Some(p) = noms.iter().position(|(_, _, n)| n == "Filter") else {
        return Vec::new();
    };
    let mut i = noms[p].1;
    while i < corps.len() && corps[i].is_ascii_whitespace() {
        i += 1;
    }
    if corps.get(i) == Some(&b'[') {
        let fin = corps[i..].iter().position(|&c| c == b']').map_or(corps.len(), |x| x + i);
        noms[p + 1..]
            .iter()
            .take_while(|(d, _, _)| *d < fin)
            .map(|(_, _, n)| n.clone())
            .collect()
    } else {
        noms.get(p + 1).map(|(_, _, n)| n.clone()).into_iter().collect()
    }
}

/// Octets bruts du flux d'un objet : valeur de `/Length` si elle est cohérente, sinon jusqu'à `endstream`.
fn octets_du_flux<'a>(
    corps: &'a [u8],
    noms: &[(usize, usize, String)],
) -> Option<&'a [u8]> {
    let (d, fin) = *regions_flux(corps).first()?;
    let borne = fin.max(d);
    if let Some(len) = entier_apres(corps, noms, "Length") {
        if let Some(fin_l) = d.checked_add(len) {
            if fin_l <= corps.len() {
                let reste = &corps[fin_l..];
                let sans = reste
                    .iter()
                    .position(|c| !c.is_ascii_whitespace())
                    .unwrap_or(reste.len());
                if reste[sans..].starts_with(b"endstream") {
                    return Some(&corps[d..fin_l]);
                }
            }
        }
    }
    let mut f = borne;
    if f > d && corps[f - 1] == b'\n' {
        f -= 1;
    }
    if f > d && corps[f - 1] == b'\r' {
        f -= 1;
    }
    Some(&corps[d..f])
}

/// Décompresse un flux d'objets et rend ses objets `(numéro, octets)`.
fn objets_du_flux(
    corps: &[u8],
    noms: &[(usize, usize, String)],
    reste_plafond: &mut usize,
) -> Result<Vec<(u32, Vec<u8>)>, ErreurPiece> {
    let n = entier_apres(corps, noms, "N").ok_or(refus(ENTETE))?;
    let premier = entier_apres(corps, noms, "First").ok_or(refus(ENTETE))?;
    if n > OBJETS_MAX {
        return Err(refus("PDF compressé : trop d'objets"));
    }
    let brut = octets_du_flux(corps, noms).ok_or(refus(ENTETE))?;
    let f = filtres(corps, noms);
    let donnees = match f.as_slice() {
        [] => {
            if brut.len() > *reste_plafond {
                return Err(refus("PDF compressé : taille décompressée excessive"));
            }
            brut.to_vec()
        }
        [x] if x == "FlateDecode" || x == "Fl" => {
            let v = inflate::zlib(brut, *reste_plafond).map_err(|e| match e {
                ErreurInflate::Plafond => refus("PDF compressé : taille décompressée excessive"),
                ErreurInflate::Tronque => refus("PDF compressé tronqué ou illisible"),
                _ => refus("PDF compressé corrompu"),
            })?;
            let predicteur = entier_apres(corps, noms, "Predictor").unwrap_or(1);
            match predicteur {
                1 => v,
                10..=15 => {
                    let col = entier_apres(corps, noms, "Columns").unwrap_or(1);
                    let cou = entier_apres(corps, noms, "Colors").unwrap_or(1);
                    let bpc = entier_apres(corps, noms, "BitsPerComponent").unwrap_or(8);
                    inflate::predicteur_png(&v, col, cou, bpc)
                        .map_err(|_| refus("PDF compressé corrompu"))?
                }
                _ => return Err(refus(ENTETE)),
            }
        }
        _ => return Err(refus(ENTETE)),
    };
    *reste_plafond = reste_plafond.saturating_sub(donnees.len());
    if premier > donnees.len() {
        return Err(refus("PDF compressé corrompu"));
    }
    // En-tête : n paires « numéro décalage ».
    let entete = String::from_utf8_lossy(&donnees[..premier]);
    let mut jetons = entete.split_ascii_whitespace();
    let mut paires: Vec<(u32, usize)> = Vec::with_capacity(n.min(4096));
    for _ in 0..n {
        let num: u32 = jetons
            .next()
            .and_then(|x| x.parse().ok())
            .ok_or(refus("PDF compressé corrompu"))?;
        let off: usize = jetons
            .next()
            .and_then(|x| x.parse().ok())
            .ok_or(refus("PDF compressé corrompu"))?;
        paires.push((num, off));
    }
    let corps_objets = &donnees[premier..];
    let mut sortie = Vec::with_capacity(paires.len());
    for (k, (num, off)) in paires.iter().enumerate() {
        let fin = paires.get(k + 1).map_or(corps_objets.len(), |p| p.1);
        if *off > fin || fin > corps_objets.len() {
            return Err(refus("PDF compressé corrompu"));
        }
        let o = &corps_objets[*off..fin];
        // Un objet compressé ne peut être ni un flux, ni un flux d'objets (boucle) ni une table xref.
        let n_internes = noms_hors_flux(o);
        if trouver(o, b"stream", 0).is_some()
            || type_est(&n_internes, "ObjStm")
            || type_est(&n_internes, "XRef")
        {
            return Err(refus("PDF compressé : flux d'objets imbriqués"));
        }
        sortie.push((*num, o.to_vec()));
    }
    Ok(sortie)
}

/// Dernière référence `/<cle> N G R` rencontrée hors flux.
fn derniere_reference(b: &[u8], noms: &[(usize, usize, String)], cle: &str) -> Option<(u32, u16)> {
    for (_, fin, _) in noms.iter().rev().filter(|(_, _, n)| n == cle) {
        let t = &b[*fin..b.len().min(*fin + 40)];
        let s = String::from_utf8_lossy(t);
        let mut it = s.split_ascii_whitespace();
        let n = it.next().and_then(|x| x.parse::<u32>().ok());
        let g = it.next().and_then(|x| x.parse::<u16>().ok());
        let r = it.next().is_some_and(|x| x.starts_with('R'));
        if let (Some(n), Some(g), true) = (n, g, r) {
            return Some((n, g));
        }
    }
    None
}

/// Reconstruit `entree` en PDF sans flux d'objets ni xref en flux. `plafond` : octets décompressés cumulés.
pub(crate) fn reconstruire(entree: &[u8], plafond: usize) -> Result<Vec<u8>, ErreurPiece> {
    let reperes = reperer(entree);
    if reperes.is_empty() {
        return Err(refus("PDF incomplet ou illisible"));
    }
    if reperes.len() > OBJETS_MAX {
        return Err(refus("PDF compressé : trop d'objets"));
    }
    let mut reste = plafond;
    let mut objets: BTreeMap<u32, (u16, Vec<u8>)> = BTreeMap::new();
    for r in &reperes {
        if r.num > NUMERO_MAX {
            return Err(refus("PDF compressé : numéro d'objet excessif"));
        }
        let corps = &entree[r.debut..r.fin];
        let noms = noms_hors_flux(corps);
        if type_est(&noms, "XRef") {
            continue;
        }
        if type_est(&noms, "ObjStm") {
            for (num, o) in objets_du_flux(corps, &noms, &mut reste)? {
                if num > NUMERO_MAX {
                    return Err(refus("PDF compressé : numéro d'objet excessif"));
                }
                let mut c = Vec::with_capacity(o.len() + 2);
                c.push(b'\n');
                c.extend_from_slice(&o);
                c.push(b'\n');
                objets.insert(num, (0, c));
            }
        } else {
            let mut c = corps.to_vec();
            // Un fichier linéarisé décrit des positions qui n'existent plus après reconstruction.
            for (d, f, n) in &noms {
                if n == "Linearized" {
                    c[*d + 1..*f].fill(b'x');
                }
            }
            objets.insert(r.num, (r.gen, c));
        }
        if objets.len() > OBJETS_MAX {
            return Err(refus("PDF compressé : trop d'objets"));
        }
    }
    let noms_fichier = noms_hors_flux(entree);
    let racine = derniere_reference(entree, &noms_fichier, "Root")
        .ok_or(refus("PDF compressé sans catalogue lisible"))?;
    let info = derniere_reference(entree, &noms_fichier, "Info");
    if !objets.contains_key(&racine.0) {
        return Err(refus("PDF compressé sans catalogue lisible"));
    }

    let mut sortie = b"%PDF-1.7\n%\xe2\xe3\xcf\xd3\n".to_vec();
    let mut positions: BTreeMap<u32, (usize, u16)> = BTreeMap::new();
    for (num, (gen, corps)) in &objets {
        positions.insert(*num, (sortie.len(), *gen));
        sortie.extend_from_slice(format!("{num} {gen} obj").as_bytes());
        sortie.extend_from_slice(corps);
        sortie.extend_from_slice(b"endobj\n");
    }
    let taille = positions.keys().next_back().map_or(1, |m| m + 1);
    let debut_xref = sortie.len();
    sortie.extend_from_slice(format!("xref\n0 {taille}\n").as_bytes());
    for n in 0..taille {
        match positions.get(&n) {
            Some((pos, gen)) => {
                sortie.extend_from_slice(format!("{pos:010} {gen:05} n \n").as_bytes())
            }
            None => sortie.extend_from_slice(b"0000000000 65535 f \n"),
        }
    }
    let mut trailer = format!("trailer\n<< /Size {taille} /Root {} {} R", racine.0, racine.1);
    if let Some((n, g)) = info.filter(|(n, _)| objets.contains_key(n)) {
        trailer.push_str(&format!(" /Info {n} {g} R"));
    }
    trailer.push_str(" >>\n");
    sortie.extend_from_slice(trailer.as_bytes());
    sortie.extend_from_slice(format!("startxref\n{debut_xref}\n%%EOF\n").as_bytes());
    // L'objet 0 est libre : un objet numéroté 0 recopié écraserait l'entrée libre.
    if objets.contains_key(&0) {
        return Err(refus("PDF compressé corrompu"));
    }
    Ok(sortie)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::inflate::fixtures::{bombe, zlib_stocke};
    use crate::piece::{desarmer_avec_plafond, desarmer_pdf, PLAFOND_DECOMPRESSE};

    /// Flux d'objets d'un PDF « Word » (7 objets, bloc **dynamique**), produit localement par `zlib.compress`.
    /// Catalogue 1 (avec `/OpenAction` et `/Names /JavaScript`), pages 2, page 3, action 5, lien 8, noms 9, police 10.
    const OBJSTM_WORD: &str = "78da6550cb4ec33010bcf3157323b9d471d2a004aa4a2d1282f2540ba7b40793ae8a45ea58895b95bfc78e03a542b256deddd999d9e58810234b9080c72952c469820c4914234792e7e01186f110a311d8eb9726b06b6144556fc05ec4865a3b1c610ef6ac494d4a236b6539baca93d8dab69b9b89bd58948dd4c672bade786cdfd991d233b17bb96e51240eb2b23af54e19f0ff48171bb2bd5efa91d6524ceb038ac816d23c45368c3b02652cacc585c74d94aa6d5664bdc09cda7ad794bdc91b8bf61fee76feb5f9a3bf38d983cd160884d60351516396c1b996b4a1f36578159e1aee44edf4eedd74f983549f4eba34de2e77cf9a99a017799bdff9107c18a32f19a3bdac0674105b5d110bff38f2072e0211fa93af4e95bb858ec2aec8c1a6a225dfb9a56a4f4696c28d7d03b35d8e2c";
    const OBJSTM_WORD_PLAIN: usize = 519;

    fn depuis_hex(h: &str) -> Vec<u8> {
        (0..h.len() / 2)
            .map(|i| u8::from_str_radix(&h[2 * i..2 * i + 2], 16).unwrap())
            .collect()
    }

    fn objet(n: u32, corps: &[u8]) -> Vec<u8> {
        let mut v = format!("{n} 0 obj\n").into_bytes();
        v.extend_from_slice(corps);
        v.extend_from_slice(b"\nendobj\n");
        v
    }

    /// `<< extra /Length n >> stream … endstream`.
    fn flux(extra: &str, donnees: &[u8]) -> Vec<u8> {
        let mut v = format!("<< {extra} /Length {} >>\nstream\n", donnees.len()).into_bytes();
        v.extend_from_slice(donnees);
        v.extend_from_slice(b"\nendstream");
        v
    }

    fn assembler(objets: &[Vec<u8>], fin: &str) -> Vec<u8> {
        let mut v = b"%PDF-1.5\n%\xe2\xe3\xcf\xd3\n".to_vec();
        for o in objets {
            v.extend_from_slice(o);
        }
        v.extend_from_slice(fin.as_bytes());
        v.extend_from_slice(b"\nstartxref\n0\n%%EOF\n");
        v
    }

    fn objstm(extra: &str, z: &[u8]) -> Vec<u8> {
        objet(
            4,
            &flux(
                &format!("/Type /ObjStm /N 7 /First 40 /Filter /FlateDecode {extra}"),
                z,
            ),
        )
    }

    fn xref_flux() -> Vec<u8> {
        // Contenu quelconque : la reconstruction n'utilise jamais la table xref d'origine.
        let entrees = [0u8, 0, 0, 0, 255, 2, 0, 4, 0, 2, 0, 4, 1, 3, 1, 0, 0];
        objet(
            7,
            &flux(
                "/Type /XRef /W [1 2 1] /Size 11 /Root 1 0 R /Filter /FlateDecode",
                &zlib_stocke(&entrees),
            ),
        )
    }

    fn pdf_word() -> Vec<u8> {
        let z = depuis_hex(OBJSTM_WORD);
        assert_eq!(inflate::zlib(&z, 10_000).unwrap().len(), OBJSTM_WORD_PLAIN);
        assert_eq!((z[2] >> 1) & 3, 2, "bloc dynamique");
        assembler(
            &[
                objstm("", &z),
                objet(6, &flux("/Filter /FlateDecode", &zlib_stocke(b"BT /F1 12 Tf (Bonjour) Tj ET"))),
                xref_flux(),
            ],
            "",
        )
    }

    /// Chaque entrée de la table xref réécrite pointe sur « N G obj ».
    fn xref_exacte(pdf: &[u8]) {
        let s = pdf;
        let sx = trouver(s, b"startxref\n", 0).unwrap() + 10;
        let fin = s[sx..].iter().position(|&c| c == b'\n').unwrap() + sx;
        let debut: usize = std::str::from_utf8(&s[sx..fin]).unwrap().parse().unwrap();
        assert!(s[debut..].starts_with(b"xref\n0 "));
        let texte = String::from_utf8_lossy(&s[debut..]).to_string();
        let mut lignes = texte.lines();
        lignes.next();
        let taille: usize = lignes.next().unwrap().split(' ').nth(1).unwrap().parse().unwrap();
        for n in 0..taille {
            let l = lignes.next().unwrap();
            assert_eq!(l.len() + 1, 20, "entrée de 20 octets");
            if l.ends_with('n') || l.ends_with("n ") {
                let pos: usize = l[..10].parse().unwrap();
                assert!(
                    s[pos..].starts_with(format!("{n} 0 obj").as_bytes()),
                    "objet {n} à {pos}"
                );
            }
        }
    }

    #[test]
    fn pdf_word_flux_d_objets_est_desarme() {
        let src = pdf_word();
        let d = desarmer_pdf(&src).unwrap();
        assert_eq!(d.nb_pages, 1);
        for nom in ["JavaScript", "JS", "OpenAction", "URI"] {
            assert!(d.neutralises.iter().any(|n| n == nom), "{nom} : {:?}", d.neutralises);
        }
        let txt = String::from_utf8_lossy(&d.octets).to_string();
        assert!(!txt.contains("ObjStm") && !txt.contains("/XRef"), "plus d'objet compressé");
        for interdit in ["/JavaScript", "/JS ", "/OpenAction", "/URI"] {
            assert!(!txt.contains(interdit), "{interdit} subsiste");
        }
        // Les objets du flux sont devenus des objets à plat, lisibles.
        assert!(txt.contains("/Type /Catalog") && txt.contains("/BaseFont /Helvetica"));
        assert!(txt.contains("/Root 1 0 R"));
        xref_exacte(&d.octets);
        // Le flux de page est recopié tel quel.
        assert!(trouver(&d.octets, &zlib_stocke(b"BT /F1 12 Tf (Bonjour) Tj ET"), 0).is_some());
        // Idempotent : le résultat est un PDF simple, déjà sûr.
        let encore = desarmer_pdf(&d.octets).unwrap();
        assert!(encore.neutralises.is_empty());
        assert_eq!(encore.octets, d.octets);
        assert_eq!(encore.nb_pages, 1);
    }

    #[test]
    fn flux_d_objets_avec_predicteur_png() {
        // Mêmes objets, non compressés mais filtrés par lignes de 5 octets (filtre 0).
        let clair = inflate::zlib(&depuis_hex(OBJSTM_WORD), 10_000).unwrap();
        let mut filtre = Vec::new();
        for l in clair.chunks(5) {
            filtre.push(0);
            filtre.extend_from_slice(l);
            filtre.resize(filtre.len() + (5 - l.len()), 0);
        }
        let src = assembler(
            &[
                objet(
                    4,
                    &flux(
                        "/Type /ObjStm /N 7 /First 40 /Filter /FlateDecode /DecodeParms << /Predictor 12 /Columns 5 >>",
                        &zlib_stocke(&filtre),
                    ),
                ),
                xref_flux(),
            ],
            "",
        );
        // Les zéros de bourrage finaux sont lus comme du blanc après le dernier objet.
        let d = desarmer_pdf(&src).unwrap();
        assert!(d.neutralises.iter().any(|n| n == "JavaScript"));
    }

    #[test]
    fn scanner_flate_sans_flux_d_objets_reste_sur_place() {
        let image = zlib_stocke(&(0..2000u32).map(|i| (i * 7 % 251) as u8).collect::<Vec<_>>());
        let src = assembler(
            &[
                objet(1, b"<< /Type /Catalog /Pages 2 0 R >>"),
                objet(2, b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>"),
                objet(3, b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] /Resources << /XObject << /Im0 4 0 R >> >> >>"),
                objet(4, &flux("/Type /XObject /Subtype /Image /Width 50 /Height 40 /ColorSpace /DeviceGray /BitsPerComponent 8 /Filter /FlateDecode", &image)),
            ],
            "trailer\n<< /Root 1 0 R /Size 5 >>",
        );
        let d = desarmer_pdf(&src).unwrap();
        assert_eq!(d.octets.len(), src.len(), "désarmement sur place");
        assert_eq!(d.nb_pages, 1);
    }

    #[test]
    fn scanner_xref_en_flux_seul_est_reconstruit() {
        let image = zlib_stocke(&(0..3000u32).map(|i| (i * 13 % 255) as u8).collect::<Vec<_>>());
        let src = assembler(
            &[
                objet(1, b"<< /Type /Catalog /Pages 2 0 R >>"),
                objet(2, b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>"),
                objet(3, b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] /Resources << /XObject << /Im0 4 0 R >> >> >>"),
                objet(4, &flux("/Type /XObject /Subtype /Image /Width 50 /Height 60 /ColorSpace /DeviceGray /BitsPerComponent 8 /Filter /FlateDecode", &image)),
                xref_flux(),
            ],
            "",
        );
        let d = desarmer_pdf(&src).unwrap();
        assert_eq!(d.nb_pages, 1);
        assert!(trouver(&d.octets, &image, 0).is_some(), "image recopiée sans changement");
        assert!(!String::from_utf8_lossy(&d.octets).contains("/XRef"));
        xref_exacte(&d.octets);
    }

    #[test]
    fn pdf_lineaire_perd_sa_description_de_positions() {
        let z = depuis_hex(OBJSTM_WORD);
        let src = assembler(
            &[
                objet(11, b"<< /Linearized 1 /L 9999 /H [100 20] /O 3 /E 500 /N 1 /T 800 >>"),
                objstm("", &z),
                xref_flux(),
            ],
            "",
        );
        let d = desarmer_pdf(&src).unwrap();
        assert!(!String::from_utf8_lossy(&d.octets).contains("Linearized"));
    }

    #[test]
    fn chiffre_toujours_refuse() {
        let z = depuis_hex(OBJSTM_WORD);
        let xref = objet(
            7,
            &flux(
                "/Type /XRef /W [1 2 1] /Size 11 /Root 1 0 R /Encrypt 12 0 R /Filter /FlateDecode",
                &zlib_stocke(&[0, 0, 0, 0]),
            ),
        );
        let src = assembler(&[objstm("", &z), xref], "");
        assert!(matches!(
            desarmer_pdf(&src),
            Err(ErreurPiece::PdfRefuse(m)) if m.contains("sans mot de passe") && m.contains("photo")
        ));
    }

    #[test]
    fn flux_tronque_ou_corrompu_refuse() {
        let z = depuis_hex(OBJSTM_WORD);
        for coupe in [3, z.len() / 2, z.len() - 4] {
            let src = assembler(&[objstm("", &z[..coupe]), xref_flux()], "");
            assert!(
                matches!(desarmer_pdf(&src), Err(ErreurPiece::PdfRefuse(m)) if m.contains("compress")),
                "coupé à {coupe}"
            );
        }
        let mut abime = z.clone();
        abime[20] ^= 0x55;
        let src = assembler(&[objstm("", &abime), xref_flux()], "");
        assert!(matches!(desarmer_pdf(&src), Err(ErreurPiece::PdfRefuse(_))));
        // Filtre non géré (LZW, ASCII85) : refus net, jamais de désarmement partiel.
        let src = assembler(
            &[
                objet(4, &flux("/Type /ObjStm /N 7 /First 40 /Filter /LZWDecode", &z)),
                xref_flux(),
            ],
            "",
        );
        assert!(matches!(desarmer_pdf(&src), Err(ErreurPiece::PdfRefuse(_))));
        // En-tête de flux d'objets incohérent.
        let src = assembler(
            &[
                objet(4, &flux("/Type /ObjStm /N 99999999 /First 40 /Filter /FlateDecode", &z)),
                xref_flux(),
            ],
            "",
        );
        assert!(matches!(desarmer_pdf(&src), Err(ErreurPiece::PdfRefuse(_))));
        let src = assembler(
            &[
                objet(4, &flux("/Type /ObjStm /N 7 /First 99999 /Filter /FlateDecode", &z)),
                xref_flux(),
            ],
            "",
        );
        assert!(matches!(desarmer_pdf(&src), Err(ErreurPiece::PdfRefuse(_))));
    }

    #[test]
    fn bombe_de_decompression_refusee() {
        // ≈ 1 Mio décompressé pour ≈ 6 Kio : refusé sous un plafond de 64 Kio, sans rien construire.
        let src = assembler(
            &[
                objet(
                    4,
                    &flux("/Type /ObjStm /N 1 /First 4 /Filter /FlateDecode", &bombe(4100)),
                ),
                xref_flux(),
            ],
            "",
        );
        assert!(matches!(
            desarmer_avec_plafond(&src, 65_536),
            Err(ErreurPiece::PdfRefuse(m)) if m.contains("décompressée")
        ));
        // Plafond réel : plus de 32 Mio décompressés pour ≈ 210 Kio de fichier.
        let src = assembler(
            &[
                objet(
                    4,
                    &flux("/Type /ObjStm /N 1 /First 4 /Filter /FlateDecode", &bombe(131_000)),
                ),
                xref_flux(),
            ],
            "",
        );
        assert!(src.len() < 300_000 && 131_000 * 258 > PLAFOND_DECOMPRESSE);
        assert!(matches!(
            desarmer_pdf(&src),
            Err(ErreurPiece::PdfRefuse(m)) if m.contains("décompressée")
        ));
        // Le plafond est cumulé : deux flux de 40 Kio en refusent le second sous un plafond de 60 Kio.
        let petit = zlib_stocke(&[b' '; 40_000]);
        let src = assembler(
            &[
                objet(4, &flux("/Type /ObjStm /N 0 /First 0 /Filter /FlateDecode", &petit)),
                objet(5, &flux("/Type /ObjStm /N 0 /First 0 /Filter /FlateDecode", &petit)),
                xref_flux(),
            ],
            "",
        );
        assert!(matches!(
            desarmer_avec_plafond(&src, 60_000),
            Err(ErreurPiece::PdfRefuse(m)) if m.contains("décompressée")
        ));
    }

    #[test]
    fn boucles_d_objets() {
        let z = depuis_hex(OBJSTM_WORD);
        // Flux d'objets caché dans un flux d'objets : refusé.
        let interne = b"1 0 << /Type /ObjStm /N 1 /First 4 >>";
        let src = assembler(
            &[
                objet(
                    4,
                    &flux("/Type /ObjStm /N 1 /First 4 /Filter /FlateDecode", &zlib_stocke(interne)),
                ),
                xref_flux(),
            ],
            "",
        );
        assert!(matches!(
            desarmer_pdf(&src),
            Err(ErreurPiece::PdfRefuse(m)) if m.contains("imbriqués")
        ));
        // /Extends circulaire, page qui est son propre parent et son propre contenu : aucune
        // traversée, donc ni boucle ni débordement de pile ; le résultat reste désarmé.
        let src = assembler(
            &[
                objet(4, &flux("/Type /ObjStm /N 7 /First 40 /Extends 12 0 R /Filter /FlateDecode", &z)),
                objet(12, &flux("/Type /ObjStm /N 0 /First 0 /Extends 4 0 R /Filter /FlateDecode", &zlib_stocke(b""))),
                objet(13, b"<< /Type /Page /Parent 13 0 R /Kids [13 0 R] /Contents 13 0 R >>"),
                xref_flux(),
            ],
            "",
        );
        let d = desarmer_pdf(&src).unwrap();
        assert_eq!(d.nb_pages, 2);
        assert!(d.neutralises.iter().any(|n| n == "OpenAction"));
        // Nombre d'objets borné : 200 001 objets vides.
        let mut gros = b"%PDF-1.5\n".to_vec();
        for n in 1..=200_001u32 {
            gros.extend_from_slice(format!("{n} 0 obj\nnull\nendobj\n").as_bytes());
        }
        gros.extend_from_slice(&xref_flux());
        gros.extend_from_slice(b"\nstartxref\n0\n%%EOF\n");
        assert!(matches!(desarmer_pdf(&gros), Err(ErreurPiece::PdfRefuse(m)) if m.contains("trop")));
        // Numéro d'objet démesuré (table xref réécrite gigantesque) : refusé.
        let src = assembler(
            &[
                objet(4, &flux("/Type /ObjStm /N 7 /First 40 /Filter /FlateDecode", &z)),
                objet(4_000_000_000, b"<< >>"),
                xref_flux(),
            ],
            "",
        );
        assert!(matches!(desarmer_pdf(&src), Err(ErreurPiece::PdfRefuse(_))));
    }

    #[test]
    fn sans_catalogue_refuse() {
        let z = depuis_hex(OBJSTM_WORD);
        let xref = objet(7, &flux("/Type /XRef /W [1 2 1] /Size 11 /Filter /FlateDecode", &zlib_stocke(&[0])));
        let src = assembler(&[objstm("", &z), xref], "");
        assert!(matches!(
            desarmer_pdf(&src),
            Err(ErreurPiece::PdfRefuse(m)) if m.contains("catalogue")
        ));
    }
}
