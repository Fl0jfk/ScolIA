//! Socle 42, lot V0b : rendu PDF **côté serveur** de la fiche budget et de la circulaire
//! (`docs/socles/42_SORTIES_VOYAGES.md` §12.1, §16 ; NX-57, NX-59).
//!
//! PDF 1.4 sans dépendance : texte seul en Helvetica, A4, plusieurs pages, **accents conservés**
//! (encodage WinAnsi, `€` compris), montants alignés à droite. Le rendu est **déterministe**
//! (aucune date, aucun identifiant aléatoire) : deux rendus du même contenu sont identiques
//! octet pour octet, ce qui permet de comparer l'empreinte au registre des objets. Le contenu vient
//! des seuls faits du dossier lus en base ; rien n'est repris d'un texte venu du navigateur.

const LARGEUR: f32 = 595.0;
const HAUTEUR: f32 = 842.0;
const MARGE_G: f32 = 56.0;
const MARGE_D: f32 = 539.0;
const HAUT: f32 = 790.0;
const BAS: f32 = 64.0;

/// Un bloc de mise en page.
#[derive(Debug, Clone, PartialEq)]
pub enum Bloc {
    Titre(String),
    SousTitre(String),
    Paragraphe(String),
    /// Libellé à gauche, montant ou valeur alignés à droite.
    Ligne {
        gauche: String,
        droite: String,
        gras: bool,
    },
    Espace,
}

struct Op {
    x: f32,
    y: f32,
    taille: f32,
    gras: bool,
    texte: Vec<u8>,
}

/// Octets WinAnsi d'un texte ; ce que l'encodage ne connaît pas devient « ? ».
fn winansi(s: &str) -> Vec<u8> {
    s.chars()
        .map(|c| match c {
            '\n' | '\r' | '\t' => b' ',
            '\u{202f}' | '\u{2009}' => b' ',
            c if (' '..='~').contains(&c) => c as u8,
            '\u{a0}'..='\u{ff}' => c as u32 as u8,
            '€' => 0x80,
            '…' => 0x85,
            'Œ' => 0x8c,
            '‘' => 0x91,
            '’' => 0x92,
            '“' => 0x93,
            '”' => 0x94,
            '•' => 0x95,
            '–' => 0x96,
            '—' => 0x97,
            'œ' => 0x9c,
            _ => b'?',
        })
        .collect()
}

fn echapper(octets: &[u8]) -> Vec<u8> {
    let mut o = Vec::with_capacity(octets.len() + 8);
    for &b in octets {
        match b {
            b'(' | b')' | b'\\' => {
                o.push(b'\\');
                o.push(b);
            }
            32..=126 => o.push(b),
            _ => o.extend_from_slice(format!("\\{b:03o}").as_bytes()),
        }
    }
    o
}

/// Largeur approchée en points (Helvetica) : suffisante pour aligner des montants à droite.
fn largeur(octets: &[u8], taille: f32) -> f32 {
    let milli: u32 = octets
        .iter()
        .map(|&b| match b {
            b'0'..=b'9' | 0x80 => 556,
            b' ' | b',' | b'.' | 0xa0 => 278,
            b'-' => 333,
            b'%' => 889,
            b'm' | b'M' | b'W' => 833,
            b'i' | b'l' | b'j' | b'I' => 222,
            b'A'..=b'Z' => 667,
            _ => 520,
        })
        .sum();
    milli as f32 * taille / 1000.0
}

/// Coupe un texte aux mots, en lignes d'au plus `max` caractères.
fn couper(texte: &str, max: usize) -> Vec<String> {
    let mut lignes = Vec::new();
    for para in texte.split('\n') {
        let mut courant = String::new();
        // Seuls l'espace et la tabulation séparent les mots : l'espace insécable lie un montant à « € ».
        for mot in para.split([' ', '\t']).filter(|m| !m.is_empty()) {
            let mut mot = mot.to_string();
            // Un mot plus long qu'une ligne est tronqué en tranches (jamais de débordement).
            while mot.chars().count() > max {
                let tete: String = mot.chars().take(max).collect();
                mot = mot.chars().skip(max).collect();
                if !courant.is_empty() {
                    lignes.push(std::mem::take(&mut courant));
                }
                lignes.push(tete);
            }
            let n = courant.chars().count();
            if n > 0 && n + 1 + mot.chars().count() > max {
                lignes.push(std::mem::take(&mut courant));
            }
            if !courant.is_empty() {
                courant.push(' ');
            }
            courant.push_str(&mot);
        }
        lignes.push(courant);
    }
    // Les lignes vides des paragraphes vides sont conservées (espacement voulu) sauf en queue.
    while lignes.last().is_some_and(|l| l.is_empty()) && lignes.len() > 1 {
        lignes.pop();
    }
    lignes
}

/// Rend le document. `pied` figure en bas de chaque page, avec « Page i/n ».
pub fn rendre(pied: &str, blocs: &[Bloc]) -> Vec<u8> {
    fn saut(pages: &mut Vec<Vec<Op>>, y: &mut f32, besoin: f32) {
        if *y - besoin < BAS {
            pages.push(Vec::new());
            *y = HAUT;
        }
    }
    let mut pages: Vec<Vec<Op>> = vec![Vec::new()];
    let mut y = HAUT;
    for b in blocs {
        match b {
            Bloc::Espace => y -= 8.0,
            Bloc::Titre(t) => {
                for l in couper(t, 52) {
                    saut(&mut pages, &mut y, 26.0);
                    pages.last_mut().unwrap().push(Op {
                        x: MARGE_G,
                        y,
                        taille: 17.0,
                        gras: true,
                        texte: winansi(&l),
                    });
                    y -= 24.0;
                }
            }
            Bloc::SousTitre(t) => {
                y -= 6.0;
                for l in couper(t, 80) {
                    saut(&mut pages, &mut y, 40.0);
                    pages.last_mut().unwrap().push(Op {
                        x: MARGE_G,
                        y,
                        taille: 12.0,
                        gras: true,
                        texte: winansi(&l),
                    });
                    y -= 18.0;
                }
            }
            Bloc::Paragraphe(t) => {
                for l in couper(t, 92) {
                    saut(&mut pages, &mut y, 15.0);
                    if !l.is_empty() {
                        pages.last_mut().unwrap().push(Op {
                            x: MARGE_G,
                            y,
                            taille: 10.5,
                            gras: false,
                            texte: winansi(&l),
                        });
                    }
                    y -= 14.5;
                }
            }
            Bloc::Ligne { gauche, droite, gras } => {
                let lignes = couper(gauche, 66);
                for (i, l) in lignes.iter().enumerate() {
                    saut(&mut pages, &mut y, 15.0);
                    let page = pages.last_mut().unwrap();
                    page.push(Op {
                        x: MARGE_G + if i == 0 { 0.0 } else { 12.0 },
                        y,
                        taille: 10.5,
                        gras: *gras,
                        texte: winansi(l),
                    });
                    if i == 0 && !droite.is_empty() {
                        let d = winansi(droite);
                        let x = MARGE_D - largeur(&d, 10.5);
                        page.push(Op { x, y, taille: 10.5, gras: *gras, texte: d });
                    }
                    y -= 14.5;
                }
            }
        }
    }
    let n = pages.len();
    for (i, p) in pages.iter_mut().enumerate() {
        p.push(Op {
            x: MARGE_G,
            y: 34.0,
            taille: 8.0,
            gras: false,
            texte: winansi(pied),
        });
        let t = winansi(&format!("Page {}/{n}", i + 1));
        let x = MARGE_D - largeur(&t, 8.0);
        p.push(Op { x, y: 34.0, taille: 8.0, gras: false, texte: t });
    }

    // Objets : 1 catalogue, 2 pages, 3 Helvetica, 4 Helvetica-Gras, puis (page, contenu) par page.
    let mut sortie: Vec<u8> = b"%PDF-1.4\n%\xe2\xe3\xcf\xd3\n".to_vec();
    let mut offsets: Vec<usize> = Vec::new();
    fn objet(sortie: &mut Vec<u8>, offsets: &mut Vec<usize>, corps: Vec<u8>) {
        offsets.push(sortie.len());
        let num = offsets.len();
        sortie.extend_from_slice(format!("{num} 0 obj\n").as_bytes());
        sortie.extend_from_slice(&corps);
        sortie.extend_from_slice(b"\nendobj\n");
    }
    objet(&mut sortie, &mut offsets, b"<< /Type /Catalog /Pages 2 0 R >>".to_vec());
    let kids: String = (0..n).map(|i| format!("{} 0 R ", 5 + 2 * i)).collect();
    objet(
        &mut sortie,
        &mut offsets,
        format!("<< /Type /Pages /Kids [ {kids}] /Count {n} >>").into_bytes(),
    );
    for base in ["Helvetica", "Helvetica-Bold"] {
        objet(
            &mut sortie,
            &mut offsets,
            format!("<< /Type /Font /Subtype /Type1 /BaseFont /{base} /Encoding /WinAnsiEncoding >>")
                .into_bytes(),
        );
    }
    for (i, p) in pages.iter().enumerate() {
        let mut flux: Vec<u8> = Vec::new();
        for op in p {
            flux.extend_from_slice(
                format!(
                    "BT /{} {:.1} Tf {:.2} {:.2} Td (",
                    if op.gras { "F2" } else { "F1" },
                    op.taille,
                    op.x,
                    op.y
                )
                .as_bytes(),
            );
            flux.extend_from_slice(&echapper(&op.texte));
            flux.extend_from_slice(b") Tj ET\n");
        }
        objet(
            &mut sortie,
            &mut offsets,
            format!(
                "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {LARGEUR} {HAUTEUR}] \
                 /Resources << /Font << /F1 3 0 R /F2 4 0 R >> >> /Contents {} 0 R >>",
                6 + 2 * i
            )
            .into_bytes(),
        );
        let mut corps = format!("<< /Length {} >>\nstream\n", flux.len()).into_bytes();
        corps.extend_from_slice(&flux);
        corps.extend_from_slice(b"endstream");
        objet(&mut sortie, &mut offsets, corps);
    }
    let xref = sortie.len();
    sortie.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \n", offsets.len() + 1).as_bytes());
    for o in &offsets {
        sortie.extend_from_slice(format!("{o:010} 00000 n \n").as_bytes());
    }
    sortie.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
            offsets.len() + 1
        )
        .as_bytes(),
    );
    sortie
}

#[cfg(test)]
mod tests {
    use super::*;

    fn contient(hay: &[u8], aiguille: &str) -> bool {
        hay.windows(aiguille.len()).any(|w| w == aiguille.as_bytes())
    }

    #[test]
    fn document_valide_deterministe_et_accents_conserves() {
        let blocs = vec![
            Bloc::Titre("Fiche budget — Séjour à Rome".into()),
            Bloc::Paragraphe("Réunion (parents) \\ échéance : 15 € — « ok »".into()),
            Bloc::Ligne { gauche: "Transport".into(), droite: "1\u{a0}350,50\u{a0}€".into(), gras: false },
        ];
        let a = rendre("Établissement", &blocs);
        let b = rendre("Établissement", &blocs);
        assert_eq!(a, b, "rendu déterministe");
        assert!(a.starts_with(b"%PDF-1.4"));
        assert!(a.ends_with(b"%%EOF\n"));
        // « é » (0xE9 = 351 en octal), « € » (0x80 = 200), parenthèses et antislash échappés.
        assert!(contient(&a, "Fiche budget \\227 S\\351jour"), "accent et tiret cadratin");
        assert!(contient(&a, "\\(parents\\) \\\\"));
        assert!(contient(&a, "15 \\200"));
        // Le fichier reste ASCII hors de l'en-tête binaire : aucun octet brut ≥ 0x80 dans le flux.
        let corps = &a[15..];
        assert!(corps.iter().all(|&o| o < 0x80), "flux 7 bits");
    }

    #[test]
    fn table_xref_exacte() {
        let blocs: Vec<Bloc> = (0..120)
            .map(|i| Bloc::Paragraphe(format!("Ligne numéro {i}")))
            .collect();
        let pdf = rendre("pied", &blocs);
        let texte = String::from_utf8_lossy(&pdf).to_string();
        let debut = texte.rfind("startxref\n").unwrap() + "startxref\n".len();
        let fin = texte[debut..].find('\n').unwrap() + debut;
        let xref: usize = texte[debut..fin].parse().unwrap();
        assert!(pdf[xref..].starts_with(b"xref\n0 "));
        // Chaque entrée pointe sur « N 0 obj » (la fin du fichier est en ASCII : décalages sûrs).
        let apres = String::from_utf8_lossy(&pdf[xref..]).to_string();
        let lignes: Vec<&str> = apres.lines().skip(3).collect();
        let mut n = 1;
        for l in lignes.iter().take_while(|l| l.ends_with(" n ")) {
            let off: usize = l[..10].parse().unwrap();
            assert!(pdf[off..].starts_with(format!("{n} 0 obj").as_bytes()), "objet {n}");
            n += 1;
        }
        assert!(n > 6, "plusieurs pages : {n} objets");
    }

    #[test]
    fn plusieurs_pages_numerotees_et_texte_long_coupe() {
        let blocs: Vec<Bloc> = (0..201)
            .map(|i| Bloc::Ligne { gauche: format!("Poste {i}"), droite: "10,00\u{a0}€".into(), gras: i % 50 == 0 })
            .collect();
        let pdf = rendre("pied", &blocs);
        assert!(contient(&pdf, "/Count 5"), "201 lignes de 14,5 pt (50 par page) : cinq pages");
        assert!(contient(&pdf, "(Page 1/5)"));
        assert!(contient(&pdf, "(Page 5/5)"));
        // Un mot géant ne déborde pas : il est coupé en tranches.
        let long = "x".repeat(300);
        let pdf = rendre("pied", &[Bloc::Paragraphe(long.clone())]);
        assert!(!contient(&pdf, &long));
        assert_eq!(couper(&long, 92).len(), 4);
    }

    #[test]
    fn montants_alignes_a_droite() {
        let d = winansi("1\u{a0}350,50\u{a0}€");
        assert!(largeur(&d, 10.5) > largeur(&winansi("5,00"), 10.5));
        assert_eq!(winansi("é€’œ"), vec![0xe9, 0x80, 0x92, 0x9c]);
        assert_eq!(winansi("日"), b"?");
    }

    #[test]
    fn coupe_aux_mots() {
        assert_eq!(couper("un deux trois quatre", 9), ["un deux", "trois", "quatre"]);
        assert_eq!(couper("a\n\nb", 10), ["a", "", "b"]);
    }
}
