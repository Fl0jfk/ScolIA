use crate::releve::*;
use crate::*;

fn meta() -> Meta {
    Meta {
        titre: "Bulletin — Élève Hélène".into(),
        auteur: "Établissement Saint-Étienne".into(),
        creation: Some(DateHeure {
            an: 2026,
            mois: 11,
            jour: 14,
            heure: 9,
            minute: 5,
            seconde: 3,
        }),
    }
}

#[test]
fn accents_francais_relus() {
    let mut d = Document::new(meta()).avec_pied("Bulletin — Hélène");
    d.titre("Élève Hélène, Français");
    d.paragraphe("é è ê ë à â ç ô ù û ü ï î œ Œ æ « » € – — ’ … « œuvre » à 12 €");
    let pdf = d.octets();
    verifier_structure(&pdf);
    let t = texte(&pdf);
    assert!(t.contains("Élève Hélène, Français"), "{t}");
    assert!(
        t.contains("é è ê ë à â ç ô ù û ü ï î œ Œ æ « » € – — ’ …"),
        "{t}"
    );
    assert!(!t.contains('?'), "aucun « ? » de remplacement");
    // Les octets sont bien WinAnsi (É = 0xC9 = \311) et la police le déclare.
    assert!(contient(&pdf, "/Encoding /WinAnsiEncoding"));
    assert!(contient(
        &pdf,
        "(\\311l\\350ve H\\351l\\350ne, Fran\\347ais)"
    ));
}

#[test]
fn caractere_hors_table_lisible() {
    let mut d = Document::new(Meta::default());
    d.paragraphe("Łukasz → Wrocław ≤ 5 日本");
    let t = texte(&d.octets());
    assert!(t.contains("Lukasz -> Wroclaw <= 5 ??"), "{t}");
}

#[test]
fn xref_exacte_et_metadonnees() {
    let mut d = Document::new(meta());
    d.titre("Test");
    let pdf = d.octets();
    let n = verifier_structure(&pdf);
    assert_eq!(
        n,
        3 + 3 + 2,
        "catalogue, pages, info, 3 polices, page, contenu"
    );
    // Titre accentué en UTF-16BE (BOM FEFF) ; « É » = 00C9.
    assert!(contient(&pdf, "/Title <FEFF00420075006C006C00650074"));
    assert!(
        contient(&pdf, "00C9006C00E8007600650020"),
        "Élève en UTF-16"
    );
    assert!(contient(&pdf, "/CreationDate (D:20261114090503Z)"));
    assert!(contient(&pdf, "/Producer (ent-pdf)"));
}

#[test]
fn sans_date_deterministe() {
    let f = || {
        let mut d = Document::new(Meta {
            titre: "T".into(),
            auteur: "A".into(),
            creation: None,
        });
        d.paragraphe("même contenu");
        d.octets()
    };
    let (a, b) = (f(), f());
    assert_eq!(a, b);
    assert!(!contient(&a, "/CreationDate"));
}

const NB_LIGNES: usize = 34;

fn bulletin_long() -> Vec<u8> {
    let mut d = Document::new(meta())
        .avec_en_tete("Collège Saint-Étienne")
        .avec_pied("Bulletin T1");
    d.titre("Bulletin scolaire");
    let mut t = Tableau::new(vec![
        Colonne::new("Matière", 3.0, Align::Gauche),
        Colonne::new("Moyenne", 1.6, Align::Droite),
        Colonne::new("Appréciation", 6.0, Align::Gauche),
    ]);
    for i in 0..NB_LIGNES {
        t.lignes.push(Ligne::new(vec![
            format!("Matière {i} — Éducation physique"),
            format!("{:.2}", 8.0 + (i % 12) as f32),
            "Élève sérieux, œuvre régulière ; progrès constants en français, à poursuivre. \
             Attention à l'écoute en classe et à la préparation des évaluations écrites."
                .into(),
        ]));
    }
    t.lignes
        .push(Ligne::grasse(vec!["Moyenne générale", "13,40", ""]));
    d.tableau(&t);
    d.paragraphe("Fin du bulletin.");
    d.octets()
}

#[test]
fn bulletin_trois_pages_entete_repete() {
    let pdf = bulletin_long();
    verifier_structure(&pdf);
    let pages = texte_des_pages(&pdf);
    assert_eq!(
        pages.len(),
        3,
        "3 pages attendues, {} obtenues",
        pages.len()
    );
    assert!(contient(&pdf, "/Count 3 "));
    for (i, p) in pages.iter().enumerate() {
        assert!(p.contains(&format!("Page {} sur 3", i + 1)), "page {i}");
        assert!(p.contains("Collège Saint-Étienne"), "en-tête page {i}");
        assert!(p.contains("Bulletin T1"), "pied page {i}");
        assert!(
            p.contains("Matière\nMoyenne\nAppréciation"),
            "ligne d'en-tête du tableau, page {i}: {p}"
        );
    }
    assert!(pages[2].contains("Moyenne générale"));
    assert!(pages[2].contains("Fin du bulletin."));
    // Aucune ligne du tableau perdue ni dupliquée.
    let tout = pages.join("");
    for i in 0..NB_LIGNES {
        assert_eq!(
            tout.matches(&format!("Matière {i} — Éducation\n")).count(),
            1,
            "ligne {i}"
        );
    }
}

#[test]
fn rangee_plus_haute_qu_une_page_est_coupee() {
    let mut d = Document::new(Meta::default());
    let mut t = Tableau::new(vec![Colonne::new("Note", 1.0, Align::Gauche)]);
    let long: Vec<String> = (0..160).map(|i| format!("ligne{i}")).collect();
    t.lignes.push(Ligne::new(vec![long.join("\n")]));
    d.tableau(&t);
    let pdf = d.octets();
    verifier_structure(&pdf);
    let pages = texte_des_pages(&pdf);
    assert!(pages.len() >= 3, "{}", pages.len());
    let tout = pages.join("");
    for i in 0..160 {
        assert_eq!(tout.matches(&format!("ligne{i}\n")).count(), 1, "ligne{i}");
    }
    for p in &pages {
        assert!(p.contains("Note\n"), "en-tête répété");
    }
}

#[test]
fn retour_a_la_ligne_par_largeur_mesuree() {
    let l = couper(
        Police::Helvetica,
        10.0,
        b"un deux trois quatre cinq six sept huit neuf dix",
        80.0,
    );
    assert!(l.len() > 2);
    for x in &l {
        assert!(largeur(Police::Helvetica, x, 10.0) <= 80.0 + 0.01);
    }
    // Mot plus long que la ligne : tranché, rien de perdu.
    let l = couper(
        Police::Helvetica,
        10.0,
        b"ABCDEFGHIJKLMNOPQRSTUVWXYZABCDEFGHIJKLMNOPQRSTUVWXYZ",
        100.0,
    );
    assert!(l.len() >= 3);
    assert_eq!(
        l.concat(),
        b"ABCDEFGHIJKLMNOPQRSTUVWXYZABCDEFGHIJKLMNOPQRSTUVWXYZ".to_vec()
    );
    // Paragraphes et lignes vides.
    assert_eq!(couper(Police::Times, 10.0, b"a\n\nb", 100.0).len(), 3);
}

#[test]
fn alignements() {
    let mut d = Document::new(Meta::default());
    d.texte("droite", Police::Helvetica, 10.0, Align::Droite);
    d.texte("centre", Police::Helvetica, 10.0, Align::Centre);
    d.texte("gauche", Police::Times, 10.0, Align::Gauche);
    let pdf = d.octets();
    let s = String::from_utf8_lossy(&pdf).into_owned();
    // « droite » en Helvetica 10 : d556 r333 o556 i222 t278 e556 = 2501 → 25,01 pt ; x = 539 − 25,01.
    assert!(s.contains("513.99 "), "x de l'alignement à droite : {s}");
    assert!(s.contains("/F3 10 Tf 56 "), "Times à gauche");
}

#[test]
fn ligne_droite_montants() {
    let mut d = Document::new(Meta::default());
    d.ligne_droite("Voyage à Rome — hébergement", "1 250,00 €", true);
    let pdf = d.octets();
    let t = texte(&pdf);
    assert!(t.contains("Voyage à Rome — hébergement") && t.contains("1 250,00 €"));
}

#[test]
fn logo_jpeg_dct() {
    let jpg = crate::jpeg::jpeg_de_test(200, 100, 3);
    let d = Document::new(meta())
        .avec_logo(&jpg)
        .unwrap()
        .avec_en_tete("Établissement");
    let mut d = d;
    d.titre("Avec logo");
    d.saut_de_page();
    d.paragraphe("page 2");
    let pdf = d.octets();
    verifier_structure(&pdf);
    assert!(contient(&pdf, "/Filter /DCTDecode"));
    assert!(contient(&pdf, "/ColorSpace /DeviceRGB /BitsPerComponent 8"));
    assert!(contient(&pdf, "/Width 200 /Height 100"));
    // Les octets JPEG sont repris tels quels, une seule fois, et utilisés sur les deux pages.
    assert_eq!(
        pdf.windows(jpg.len())
            .filter(|w| *w == jpg.as_slice())
            .count(),
        1
    );
    assert_eq!(pdf.windows(7).filter(|w| *w == b"/Im1 Do").count(), 2);
    assert!(Document::new(meta()).avec_logo(b"pas un jpeg").is_err());
}

#[test]
fn image_en_ligne_gris() {
    let jpg = crate::jpeg::jpeg_de_test(40, 40, 1);
    let mut d = Document::new(Meta::default());
    d.image(&jpg, 80.0, Align::Centre).unwrap();
    let pdf = d.octets();
    verifier_structure(&pdf);
    assert!(contient(&pdf, "/DeviceGray"));
}

/// Relit le QR dessiné dans le PDF (rectangles `x y m m re` entre `1 g` et `f Q`) et le décode.
fn qr_depuis_pdf(pdf: &[u8]) -> Vec<u8> {
    let s = String::from_utf8_lossy(pdf).into_owned();
    let debut = s.find("q 1 g ").expect("bloc QR");
    let fin = s[debut..].find("f Q\n").unwrap() + debut;
    let mut lignes = s[debut..fin].lines();
    let fond: Vec<f32> = lignes
        .next()
        .unwrap()
        .split_whitespace()
        .filter_map(|t| t.parse().ok())
        .collect();
    // q 1 g x y cote cote re f 0 g → [1, x, y, cote, cote, 0]
    let (x0, y0, cote) = (fond[1], fond[2], fond[3]);
    let rects: Vec<(f32, f32, f32)> = lignes
        .filter(|l| l.ends_with(" re"))
        .map(|l| {
            let v: Vec<f32> = l
                .split_whitespace()
                .take(4)
                .map(|t| t.parse().unwrap())
                .collect();
            (v[0], v[1], v[2])
        })
        .collect();
    let m = rects[0].2;
    let total = (cote / m).round() as usize; // n + 8
    let n = total - 8;
    let mut mat = vec![false; n * n];
    for (px, py, _) in rects {
        let col = ((px - x0) / m).round() as usize - 4;
        let lig = ((y0 + cote - py) / m).round() as usize - 5;
        mat[lig * n + col] = true;
    }
    crate::qr::decodeur::decoder_matrice(n, &mat).unwrap()
}

#[test]
fn qr_dessine_en_vecteurs_et_decodable() {
    for url in [
        "https://ent.example/verifier/3F2A9C41",
        "HTTPS://ENT.EXAMPLE/V/ABC123",
        "https://ent.example/v?id=é&c=0123456789abcdef0123456789abcdef0123456789abcdef",
    ] {
        let mut d = Document::new(Meta::default());
        d.titre("Vérification");
        d.qr(url, 110.0, Align::Droite).unwrap();
        let pdf = d.octets();
        verifier_structure(&pdf);
        assert!(!contient(&pdf, "/Subtype /Image"), "vectoriel, pas d'image");
        assert_eq!(qr_depuis_pdf(&pdf), url.as_bytes(), "{url}");
    }
    let mut d = Document::new(Meta::default());
    assert!(d.qr(&"x".repeat(300), 100.0, Align::Gauche).is_err());
}

#[test]
fn pas_de_compression_sans_compresseur() {
    let pdf = bulletin_long();
    assert!(!contient(&pdf, "FlateDecode"));
}

#[test]
fn rendre_lignes_titre_filets_et_accents() {
    let lignes: Vec<String> = ["Fiche — Élève", "", "Nom : Hélène", "-----", "Œuvre"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    let pdf = rendre_lignes(Meta::default(), "pied", &lignes);
    verifier_structure(&pdf);
    let t = texte(&pdf);
    assert!(t.contains("Fiche — Élève") && t.contains("Nom : Hélène") && t.contains("Œuvre"));
    assert!(!t.contains("-----"));
}

/// Sortie pour vérification locale (`pdfinfo`, `qpdf --check`, `pdftotext`) :
/// `ENT_PDF_SORTIE=/chemin cargo test -p ent-pdf sortie_locale`.
#[test]
fn sortie_locale() {
    if let Ok(dir) = std::env::var("ENT_PDF_SORTIE") {
        std::fs::write(format!("{dir}/bulletin.pdf"), bulletin_long()).unwrap();
        let mut d = Document::new(meta())
            .avec_logo(&crate::jpeg::jpeg_de_test(200, 100, 3))
            .unwrap()
            .avec_en_tete("Établissement");
        d.titre("Vérification publique");
        d.qr(
            "https://ent.example/verifier/3F2A9C41",
            120.0,
            Align::Centre,
        )
        .unwrap();
        std::fs::write(format!("{dir}/qr.pdf"), d.octets()).unwrap();
    }
}
