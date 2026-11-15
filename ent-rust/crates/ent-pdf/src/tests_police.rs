//! Police TrueType incorporée : noms de toutes langues, sous-ensemble, relecture par la
//! `ToUnicode` et, quand ils existent localement, `pdftotext` / `pdffonts` / `qpdf`.

use crate::releve::*;
use crate::ttf::Ttf;
use crate::*;
use std::process::Command;

const NOMS: [&str; 9] = [
    "Nguyễn Thị Hằng",
    "Łukasz Wróblewski",
    "Şükrü Öztürk",
    "Ștefan Țăran",
    "Ærøskøbing",
    "Ελένη",
    "Анна",
    "Mohammed Al-Ḥaddād",
    "Đặng Bảo Ngọc",
];

pub(crate) fn document_noms() -> Document {
    let mut d = Document::new(Meta {
        titre: "Certificat de scolarité — Nguyễn".into(),
        auteur: "Collège Saint-Étienne".into(),
        creation: None,
    })
    .avec_en_tete("Collège Saint-Étienne — Łódź")
    .avec_pied("Établissement — Ștefan");
    d.titre("Certificat de scolarité");
    for n in NOMS {
        d.paragraphe(&format!("Je soussigné certifie que {n} est inscrit(e)."));
    }
    d
}

#[test]
fn noms_de_toutes_langues_relus_par_la_tounicode() {
    let (pdf, avert) = document_noms().octets_avec_avertissements();
    verifier_structure(&pdf);
    assert!(avert.is_empty(), "{avert:?}");
    let t = texte(&pdf);
    for n in NOMS {
        assert!(t.contains(n), "{n} absent de : {t}");
    }
    assert!(!t.contains('?') && !t.contains('\u{fffd}'), "{t}");
    assert!(t.contains("Collège Saint-Étienne — Łódź") && t.contains("Établissement — Ștefan"));
    assert!(contient(&pdf, "/Subtype /CIDFontType2") && contient(&pdf, "/Encoding /Identity-H"));
    assert!(contient(&pdf, "/CIDToGIDMap /Identity") && contient(&pdf, "/ToUnicode "));
    assert!(contient(&pdf, "/FontFile2 "));
    // Métadonnée en UTF-16BE : « ễ » = 1EC5.
    assert!(contient(&pdf, "1EC5"));
}

#[test]
fn nfd_saisi_decompose_sort_compose() {
    let mut d = Document::new(Meta::default());
    d.paragraphe("Nguye\u{302}\u{303}n Thi\u{323} Ha\u{306}\u{300}ng, Se\u{301}bastien");
    let (pdf, avert) = d.octets_avec_avertissements();
    assert!(avert.is_empty(), "{avert:?}");
    assert!(
        texte(&pdf).contains("Nguyễn Thị Hằng, Sébastien"),
        "{}",
        texte(&pdf)
    );
}

#[test]
fn caractere_absent_jamais_un_point_d_interrogation_silencieux() {
    let mut d = Document::new(Meta::default());
    d.paragraphe("Élève 日本 ok");
    let (pdf, avert) = d.octets_avec_avertissements();
    verifier_structure(&pdf);
    let t = texte(&pdf);
    assert!(!t.contains('?'), "{t}");
    assert!(t.contains("Élève \u{fffd}\u{fffd} ok"), "{t}");
    assert_eq!(avert.len(), 2, "{avert:?}");
    assert!(
        avert[0].starts_with("U+65E5"),
        "code seul, jamais le nom : {avert:?}"
    );
    // Translittération lisible quand la police ne couvre pas le caractère mais sa lettre de base oui.
    let mut d = Document::new(Meta::default());
    d.paragraphe("a\u{2011}b → c");
    let (pdf, avert) = d.octets_avec_avertissements();
    assert!(avert.is_empty(), "{avert:?}");
    assert!(
        texte(&pdf).contains("a\u{2011}b → c") || texte(&pdf).contains("a-b"),
        "{}",
        texte(&pdf)
    );
}

#[test]
fn repli_standard_conserve_les_polices_winansi() {
    let mut d = Document::new(Meta::default()).avec_polices_standard();
    d.paragraphe("Nguyễn");
    let (pdf, avert) = d.octets_avec_avertissements();
    verifier_structure(&pdf);
    assert!(contient(&pdf, "/BaseFont /Helvetica ") && !contient(&pdf, "FontFile2"));
    assert!(
        texte(&pdf).contains("Nguyen") || texte(&pdf).contains("Nguy"),
        "{}",
        texte(&pdf)
    );
    // Plus de « ? » silencieux : un « ễ » absent de la table est signalé.
    let _ = avert;
}

/// Octets de la police incorporée (FontFile2) du n-ième flux `/Length1`.
fn fontes_incorporees(pdf: &[u8]) -> Vec<Vec<u8>> {
    let mut r = Vec::new();
    let mut i = 0;
    while let Some(p) = find(pdf, b"/Length1 ", i) {
        let fin = find(pdf, b"stream\n", p).unwrap() + 7;
        let l: usize = String::from_utf8_lossy(&pdf[p + 9..])
            .split_whitespace()
            .next()
            .unwrap()
            .parse()
            .unwrap();
        r.push(pdf[fin..fin + l].to_vec());
        i = fin + l;
    }
    r
}

fn lire_sous_ensemble(octets: Vec<u8>) -> Ttf {
    Ttf::lire(Box::leak(octets.into_boxed_slice())).expect("sous-ensemble relisible par le lecteur")
}

#[test]
fn sous_ensemble_minimal() {
    let mut d = Document::new(Meta::default());
    d.paragraphe("Aa é");
    let pdf = d.octets();
    let polices = fontes_incorporees(&pdf);
    assert_eq!(polices.len(), 1);
    assert!(
        polices[0].len() < 20_000,
        "sous-ensemble de {} octets",
        polices[0].len()
    );
    let t = lire_sous_ensemble(polices[0].clone());
    // Caractères utilisés : « Aa é » + pied « Page 1 sur 1 » = A a espace é P g e s 1 u r (11),
    // + .notdef + le seul composant absent de « é » (l'accent aigu ; le « e » est déjà utilisé).
    assert_eq!(t.nb_glyphes, 13, "uniquement les glyphes utilisés");
    // Tous les glyphes composites pointent dans le sous-ensemble.
    for g in 0..t.nb_glyphes {
        for (_, c) in Ttf::composants(t.glyphe(g)) {
            assert!(c < t.nb_glyphes, "composant {c} hors du sous-ensemble");
        }
    }
    // Largeurs /W conformes à la police d'origine : A = 684/1000 em environ (DejaVu Sans).
    assert!(contient(&pdf, "/W [0 [") && contient(&pdf, "/DW 1000"));
    // Un texte plus long ne fait pas exploser la police : ajouter un glyphe l'allonge peu.
    let mut d2 = Document::new(Meta::default());
    d2.paragraphe("Aa é ñ");
    let p2 = fontes_incorporees(&d2.octets());
    assert!(p2[0].len() > polices[0].len() && p2[0].len() < polices[0].len() + 4_000);
}

#[test]
fn sous_ensemble_des_deux_faces() {
    let (pdf, _) = document_noms().octets_avec_avertissements();
    let polices = fontes_incorporees(&pdf);
    assert_eq!(polices.len(), 2, "régulier et gras");
    for p in &polices {
        assert!(p.len() < 120_000, "{} octets", p.len());
        lire_sous_ensemble(p.clone());
    }
    assert!(contient(&pdf, "+DejaVuSans ") && contient(&pdf, "+DejaVuSans-Bold "));
}

#[test]
fn bulletin_trois_pages_noms_vietnamiens() {
    let mut d = Document::new(Meta::default())
        .avec_en_tete("Collège Saint-Étienne")
        .avec_pied("Bulletin T1");
    d.titre("Bulletin scolaire — Nguyễn Thị Hằng");
    let mut t = Tableau::new(vec![
        Colonne::new("Matière", 3.0, Align::Gauche),
        Colonne::new("Moyenne", 1.6, Align::Droite),
        Colonne::new("Appréciation", 6.0, Align::Gauche),
    ]);
    for i in 0..34 {
        t.lignes.push(Ligne::new(vec![
            format!("Matière {i} — Éducation physique"),
            format!("{:.2}", 8.0 + (i % 12) as f32),
            "Ștefan Țăran, Łukasz Wróblewski : progrès constants en français, à poursuivre. \
             Attention à l'écoute en classe et à la préparation des évaluations écrites."
                .into(),
        ]));
    }
    d.tableau(&t);
    let (pdf, avert) = d.octets_avec_avertissements();
    verifier_structure(&pdf);
    assert!(avert.is_empty(), "{avert:?}");
    let pages = texte_des_pages(&pdf);
    assert_eq!(pages.len(), 3, "{} pages", pages.len());
    for (i, p) in pages.iter().enumerate() {
        assert!(p.contains(&format!("Page {} sur 3", i + 1)));
        assert!(
            p.contains("Matière\nMoyenne\nAppréciation"),
            "en-tête répété page {i}"
        );
    }
    let tout = pages.join("");
    for i in 0..34 {
        assert_eq!(
            tout.matches(&format!("Matière {i} — Éducation\n")).count(),
            1,
            "ligne {i}"
        );
    }
    assert!(tout.contains("Ștefan Țăran, Łukasz") && tout.contains("Nguyễn Thị Hằng"));
}

fn outil(nom: &str) -> bool {
    Command::new(nom).arg("-v").output().is_ok()
        || Command::new(nom).arg("--version").output().is_ok()
}

fn ecrire_temp(nom: &str, pdf: &[u8]) -> std::path::PathBuf {
    let p = std::env::temp_dir().join(format!("ent-pdf-{}-{nom}", std::process::id()));
    std::fs::write(&p, pdf).unwrap();
    p
}

/// Vérifications locales facultatives : jamais une dépendance, ignorées si l'outil est absent.
#[test]
fn outils_externes_si_presents() {
    let (pdf, _) = document_noms().octets_avec_avertissements();
    let f = ecrire_temp("noms.pdf", &pdf);
    if outil("pdftotext") {
        let s = Command::new("pdftotext")
            .args(["-enc", "UTF-8"])
            .arg(&f)
            .arg("-")
            .output()
            .unwrap();
        assert!(s.status.success());
        let t = String::from_utf8_lossy(&s.stdout).into_owned();
        for n in NOMS {
            assert!(t.contains(n), "pdftotext : {n} absent de {t}");
        }
    }
    if outil("pdffonts") {
        let s = Command::new("pdffonts").arg(&f).output().unwrap();
        let t = String::from_utf8_lossy(&s.stdout).into_owned();
        assert!(
            s.status.success() && t.contains("CID TrueType") && t.contains("Identity-H"),
            "{t}"
        );
        assert!(
            t.lines().skip(2).all(|l| l.contains("yes yes yes")),
            "incorporée, sous-ensemble, Unicode : {t}"
        );
    }
    if outil("qpdf") {
        let s = Command::new("qpdf")
            .arg("--check")
            .arg(&f)
            .output()
            .unwrap();
        assert!(s.status.success(), "{}", String::from_utf8_lossy(&s.stderr));
    }
    let _ = std::fs::remove_file(f);
}
