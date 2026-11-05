//! Contrôle des fichiers déposés (D2) : type **réel** par signature binaire, liste
//! blanche par usage, taille maximale par usage, nom assaini, refus des fichiers
//! actifs, point d'accroche antivirus.
//!
//! Principes :
//! - le type déclaré par le client (`Content-Type`, `mime_type`) et l'extension du nom
//!   ne sont **jamais** une preuve : le type retenu est celui que révèlent les octets ;
//!   un type déclaré précis qui contredit les octets est refusé ;
//! - l'allow-list est par usage (une photo ne peut être qu'une image) ;
//! - sont refusés : HTML/XML/SVG (même sans script : un SVG est un document actif),
//!   exécutables (PE, ELF, Mach-O, `#!`), scripts, PDF avec JavaScript/lancement,
//!   documents bureautiques avec macros ou objets ActiveX, archives ZIP quelconques ;
//! - aucune décompression : les archives ZIP (OOXML/ODF) ne sont lues que par leur
//!   répertoire central (aucune bombe de décompression possible) ;
//! - aucune dépendance nouvelle.
//!
//! Limite assumée : la signature prouve la *forme* du fichier, pas son innocuité ;
//! d'où le point d'accroche [`AnalyseurAntivirus`] (par défaut [`AucuneAnalyse`]).

use std::fmt;
use std::sync::Arc;

use crate::presign::sanitize_filename;

pub const KIB: usize = 1024;
pub const MIB: usize = 1024 * 1024;

/// Plancher / plafond des tailles configurables par variable d'environnement.
const MIN_MAX_OCTETS: usize = KIB;
const PLAFOND_MAX_OCTETS: usize = 64 * MIB;
/// Nombre maximal d'entrées lues dans un ZIP.
const ZIP_MAX_ENTREES: usize = 20_000;
/// Octets de texte examinés pour repérer du balisage actif.
const TEXTE_BALISAGE_SCAN: usize = 64 * KIB;
/// Longueur maximale d'un nom de fichier conservé (les schémas bornent à 200).
const NOM_MAX_OCTETS: usize = 200;

/// Extensions (n'importe quel segment du nom) jugées actives : le dépôt est refusé.
const EXTENSIONS_ACTIVES: &[&str] = &[
    "html", "htm", "xhtml", "shtml", "svg", "svgz", "xml", "xsl", "xslt", "js", "mjs", "cjs",
    "jsx", "ts", "exe", "dll", "com", "scr", "msi", "bat", "cmd", "ps1", "psm1", "vbs", "vbe",
    "wsf", "hta", "jar", "class", "sh", "bash", "zsh", "php", "phtml", "pl", "py", "rb", "lnk",
    "reg", "swf", "apk", "dmg", "app", "docm", "xlsm", "pptm", "dotm", "xltm", "potm", "iso",
];

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Format reconnu par signature.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TypeFichier {
    Pdf,
    Png,
    Jpeg,
    Gif,
    Webp,
    Texte,
    Docx,
    Xlsx,
    Pptx,
    Odt,
    Ods,
    Odp,
}

impl TypeFichier {
    pub fn mime(self) -> &'static str {
        match self {
            Self::Pdf => "application/pdf",
            Self::Png => "image/png",
            Self::Jpeg => "image/jpeg",
            Self::Gif => "image/gif",
            Self::Webp => "image/webp",
            Self::Texte => "text/plain",
            Self::Docx => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
            Self::Xlsx => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
            Self::Pptx => {
                "application/vnd.openxmlformats-officedocument.presentationml.presentation"
            }
            Self::Odt => "application/vnd.oasis.opendocument.text",
            Self::Ods => "application/vnd.oasis.opendocument.spreadsheet",
            Self::Odp => "application/vnd.oasis.opendocument.presentation",
        }
    }

    /// Extension canonique (sans point).
    pub fn extension(self) -> &'static str {
        match self {
            Self::Pdf => "pdf",
            Self::Png => "png",
            Self::Jpeg => "jpg",
            Self::Gif => "gif",
            Self::Webp => "webp",
            Self::Texte => "txt",
            Self::Docx => "docx",
            Self::Xlsx => "xlsx",
            Self::Pptx => "pptx",
            Self::Odt => "odt",
            Self::Ods => "ods",
            Self::Odp => "odp",
        }
    }

    /// Extensions acceptées pour ce type (minuscules, sans point).
    fn extensions(self) -> &'static [&'static str] {
        match self {
            Self::Jpeg => &["jpg", "jpeg", "jpe"],
            Self::Texte => &["txt", "text", "csv", "md", "log"],
            Self::Pdf => &["pdf"],
            Self::Png => &["png"],
            Self::Gif => &["gif"],
            Self::Webp => &["webp"],
            Self::Docx => &["docx"],
            Self::Xlsx => &["xlsx"],
            Self::Pptx => &["pptx"],
            Self::Odt => &["odt"],
            Self::Ods => &["ods"],
            Self::Odp => &["odp"],
        }
    }

    pub fn est_image(self) -> bool {
        matches!(self, Self::Png | Self::Jpeg | Self::Gif | Self::Webp)
    }

    /// Type correspondant à un MIME (stockage historique) ; alias usuels compris.
    pub fn depuis_mime(mime: &str) -> Option<Self> {
        let m = mime
            .split(';')
            .next()
            .unwrap_or("")
            .trim()
            .to_ascii_lowercase();
        Some(match m.as_str() {
            "application/pdf" => Self::Pdf,
            "image/png" => Self::Png,
            "image/jpeg" | "image/jpg" | "image/pjpeg" => Self::Jpeg,
            "image/gif" => Self::Gif,
            "image/webp" => Self::Webp,
            "text/plain" => Self::Texte,
            _ => {
                return [
                    Self::Docx,
                    Self::Xlsx,
                    Self::Pptx,
                    Self::Odt,
                    Self::Ods,
                    Self::Odp,
                ]
                .into_iter()
                .find(|t| t.mime() == m)
            }
        })
    }
}

/// Usage d'un dépôt : détermine la liste blanche et la taille maximale.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UsageFichier {
    /// Documents du dossier élève, justificatifs, pièces de réinscription.
    Document,
    /// Pièce jointe de messagerie.
    PieceMessage,
    /// Pièce du PAI (ordonnance…).
    PiecePai,
    /// Photo d'identité de l'élève (trombinoscope).
    Photo,
}

const TOUS_DOCUMENTS: &[TypeFichier] = &[
    TypeFichier::Pdf,
    TypeFichier::Png,
    TypeFichier::Jpeg,
    TypeFichier::Gif,
    TypeFichier::Webp,
    TypeFichier::Texte,
    TypeFichier::Docx,
    TypeFichier::Xlsx,
    TypeFichier::Pptx,
    TypeFichier::Odt,
    TypeFichier::Ods,
    TypeFichier::Odp,
];

impl UsageFichier {
    pub fn autorises(self) -> &'static [TypeFichier] {
        match self {
            Self::Document | Self::PieceMessage => TOUS_DOCUMENTS,
            Self::PiecePai => &[TypeFichier::Pdf, TypeFichier::Png, TypeFichier::Jpeg],
            Self::Photo => &[TypeFichier::Png, TypeFichier::Jpeg, TypeFichier::Webp],
        }
    }

    pub fn libelle(self) -> &'static str {
        match self {
            Self::Document => "document",
            Self::PieceMessage => "pièce jointe de message",
            Self::PiecePai => "pièce de PAI",
            Self::Photo => "photo",
        }
    }

    fn variable_env(self) -> &'static str {
        match self {
            Self::Document => "ENT_UPLOAD_MAX_DOCUMENT",
            Self::PieceMessage => "ENT_UPLOAD_MAX_MESSAGE",
            Self::PiecePai => "ENT_UPLOAD_MAX_PAI",
            Self::Photo => "ENT_UPLOAD_MAX_PHOTO",
        }
    }

    fn defaut_octets(self) -> usize {
        match self {
            Self::Document => 10 * MIB,
            // Contenu stocké en base (P0) : plafond historique conservé.
            Self::PieceMessage | Self::PiecePai => 256 * KIB,
            Self::Photo => 5 * MIB,
        }
    }
}

/// Motif de refus (messages affichables, sans donnée sensible).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErreurFichier {
    Vide,
    TropGros {
        max: usize,
    },
    /// Format non reconnu ou non géré (dont OLE hérité, archives quelconques, binaires).
    TypeInconnu,
    /// Format reconnu mais hors liste blanche de l'usage.
    TypeNonAutorise {
        type_detecte: &'static str,
    },
    /// Fichier actif (HTML, SVG, exécutable, script, macro…).
    FichierActif(&'static str),
    /// Le type déclaré contredit les octets.
    TypeDeclareIncoherent {
        declare: String,
        detecte: &'static str,
    },
    NomInvalide,
    /// Refus de l'analyseur antivirus.
    Infecte(String),
    /// Analyseur antivirus configuré mais indisponible (fail-closed).
    AnalyseIndisponible,
}

impl ErreurFichier {
    /// `true` pour un refus de taille (HTTP 413).
    pub fn est_taille(&self) -> bool {
        matches!(self, Self::TropGros { .. })
    }

    /// `true` pour une indisponibilité du service d'analyse (HTTP 503).
    pub fn est_indisponibilite(&self) -> bool {
        matches!(self, Self::AnalyseIndisponible)
    }
}

impl fmt::Display for ErreurFichier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Vide => f.write_str("fichier vide refusé"),
            Self::TropGros { max } => {
                write!(f, "fichier trop volumineux (maximum {} Ko)", max / KIB)
            }
            Self::TypeInconnu => f.write_str(
                "type de fichier non reconnu ou non pris en charge (formats acceptés : PDF, images PNG/JPEG/GIF/WebP, texte, documents Office ou OpenDocument sans macro)",
            ),
            Self::TypeNonAutorise { type_detecte } => {
                write!(f, "type de fichier non autorisé pour cet usage ({type_detecte})")
            }
            Self::FichierActif(motif) => write!(f, "fichier actif refusé ({motif})"),
            Self::TypeDeclareIncoherent { declare, detecte } => write!(
                f,
                "le type déclaré ({declare}) ne correspond pas au contenu ({detecte})"
            ),
            Self::NomInvalide => f.write_str("nom de fichier refusé"),
            Self::Infecte(m) => write!(f, "fichier refusé par l'analyse antivirus ({m})"),
            Self::AnalyseIndisponible => {
                f.write_str("analyse antivirus indisponible : dépôt refusé")
            }
        }
    }
}

impl std::error::Error for ErreurFichier {}

// ---------------------------------------------------------------------------
// Antivirus (point d'accroche)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VerdictAnalyse {
    Sain,
    /// Menace identifiée (nom de la signature).
    Infecte(String),
    /// Le service d'analyse n'a pas pu répondre : le dépôt est refusé (fail-closed).
    Indisponible,
}

/// Contexte transmis à l'analyseur (le contenu a déjà passé le contrôle de forme).
pub struct ContexteAnalyse<'a> {
    pub usage: UsageFichier,
    pub type_fichier: TypeFichier,
    pub nom: &'a str,
    pub octets: &'a [u8],
}

/// Point d'accroche d'un futur antivirus (ClamAV via `clamd`, service tiers…).
///
/// Branchement : implémenter ce trait et l'installer avec
/// [`PolitiqueFichiers::avec_analyseur`]. L'analyse est **synchrone et appelée avant
/// l'écriture** dans le magasin ; une implémentation réseau doit borner son délai et
/// renvoyer [`VerdictAnalyse::Indisponible`] en cas d'échec (le dépôt est alors refusé).
pub trait AnalyseurAntivirus: Send + Sync {
    fn nom(&self) -> &'static str;
    fn analyser(&self, ctx: &ContexteAnalyse<'_>) -> VerdictAnalyse;
}

/// Implémentation par défaut : **aucune analyse** (tout est déclaré sain).
///
/// Elle n'offre AUCUNE protection antivirus : seul le contrôle de forme de ce module
/// s'applique. À remplacer avant d'accepter des dépôts de tiers non fiables à grande
/// échelle (voir `docs/SECURITY.md`, « Fichiers déposés »).
#[derive(Debug, Default, Clone, Copy)]
pub struct AucuneAnalyse;

impl AnalyseurAntivirus for AucuneAnalyse {
    fn nom(&self) -> &'static str {
        "aucune"
    }
    fn analyser(&self, _ctx: &ContexteAnalyse<'_>) -> VerdictAnalyse {
        VerdictAnalyse::Sain
    }
}

// ---------------------------------------------------------------------------
// Politique
// ---------------------------------------------------------------------------

/// Tailles maximales par usage (octets).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TaillesMax {
    pub document: usize,
    pub piece_message: usize,
    pub piece_pai: usize,
    pub photo: usize,
}

impl Default for TaillesMax {
    fn default() -> Self {
        Self {
            document: UsageFichier::Document.defaut_octets(),
            piece_message: UsageFichier::PieceMessage.defaut_octets(),
            piece_pai: UsageFichier::PiecePai.defaut_octets(),
            photo: UsageFichier::Photo.defaut_octets(),
        }
    }
}

impl TaillesMax {
    /// Lit `ENT_UPLOAD_MAX_DOCUMENT|MESSAGE|PAI|PHOTO` (octets, ou suffixe `k`/`m`).
    /// Valeur absente ou illisible → défaut ; bornée à [1 Kio, 64 Mio].
    pub fn depuis_env() -> Self {
        Self::depuis(|k| std::env::var(k).ok())
    }

    pub fn depuis(get: impl Fn(&str) -> Option<String>) -> Self {
        let lire = |u: UsageFichier| {
            get(u.variable_env())
                .and_then(|v| parse_taille(&v))
                .map(|n| n.clamp(MIN_MAX_OCTETS, PLAFOND_MAX_OCTETS))
                .unwrap_or_else(|| u.defaut_octets())
        };
        Self {
            document: lire(UsageFichier::Document),
            piece_message: lire(UsageFichier::PieceMessage),
            piece_pai: lire(UsageFichier::PiecePai),
            photo: lire(UsageFichier::Photo),
        }
    }

    pub fn pour(&self, usage: UsageFichier) -> usize {
        match usage {
            UsageFichier::Document => self.document,
            UsageFichier::PieceMessage => self.piece_message,
            UsageFichier::PiecePai => self.piece_pai,
            UsageFichier::Photo => self.photo,
        }
    }
}

fn parse_taille(raw: &str) -> Option<usize> {
    let t = raw.trim().to_ascii_lowercase();
    let (num, mult) = if let Some(n) = t.strip_suffix("mib").or_else(|| t.strip_suffix('m')) {
        (n, MIB)
    } else if let Some(n) = t.strip_suffix("kib").or_else(|| t.strip_suffix('k')) {
        (n, KIB)
    } else {
        (t.as_str(), 1)
    };
    num.trim().parse::<usize>().ok()?.checked_mul(mult)
}

/// Résultat d'un contrôle réussi : à utiliser **à la place** des valeurs du client.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FichierValide {
    pub type_fichier: TypeFichier,
    /// MIME déduit des octets.
    pub mime: &'static str,
    /// Extension canonique (sans point).
    pub extension: &'static str,
    /// Nom assaini (extension alignée sur le type réel).
    pub nom: String,
}

/// Politique de dépôt : tailles + analyseur.
#[derive(Clone)]
pub struct PolitiqueFichiers {
    pub tailles: TaillesMax,
    analyseur: Arc<dyn AnalyseurAntivirus>,
}

impl fmt::Debug for PolitiqueFichiers {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PolitiqueFichiers")
            .field("tailles", &self.tailles)
            .field("analyseur", &self.analyseur.nom())
            .finish()
    }
}

impl Default for PolitiqueFichiers {
    fn default() -> Self {
        Self::nouvelle(TaillesMax::default())
    }
}

impl PolitiqueFichiers {
    pub fn nouvelle(tailles: TaillesMax) -> Self {
        Self {
            tailles,
            analyseur: Arc::new(AucuneAnalyse),
        }
    }

    pub fn depuis_env() -> Self {
        Self::nouvelle(TaillesMax::depuis_env())
    }

    pub fn avec_analyseur(mut self, a: Arc<dyn AnalyseurAntivirus>) -> Self {
        self.analyseur = a;
        self
    }

    pub fn nom_analyseur(&self) -> &'static str {
        self.analyseur.nom()
    }

    pub fn taille_max(&self, usage: UsageFichier) -> usize {
        self.tailles.pour(usage)
    }

    /// Taille maximale d'un corps JSON portant ce fichier en base64 (marge incluse).
    pub fn limite_corps_base64(&self, usage: UsageFichier) -> usize {
        self.taille_max(usage).div_ceil(3) * 4 + 16 * KIB
    }

    /// Valide un dépôt. `declare` = type annoncé par le client (jamais fiable).
    pub fn valider(
        &self,
        usage: UsageFichier,
        nom: &str,
        declare: Option<&str>,
        octets: &[u8],
    ) -> Result<FichierValide, ErreurFichier> {
        let max = self.taille_max(usage);
        if octets.is_empty() {
            return Err(ErreurFichier::Vide);
        }
        if octets.len() > max {
            return Err(ErreurFichier::TropGros { max });
        }
        controler_nom(nom)?;
        let t = detecter(octets)?;
        if !usage.autorises().contains(&t) {
            return Err(ErreurFichier::TypeNonAutorise {
                type_detecte: t.mime(),
            });
        }
        controler_declare(declare, t)?;
        let nom_final = nom_aligne(nom, t);
        match self.analyseur.analyser(&ContexteAnalyse {
            usage,
            type_fichier: t,
            nom: &nom_final,
            octets,
        }) {
            VerdictAnalyse::Sain => {}
            VerdictAnalyse::Infecte(m) => return Err(ErreurFichier::Infecte(m)),
            VerdictAnalyse::Indisponible => return Err(ErreurFichier::AnalyseIndisponible),
        }
        Ok(FichierValide {
            type_fichier: t,
            mime: t.mime(),
            extension: t.extension(),
            nom: nom_final,
        })
    }
}

// ---------------------------------------------------------------------------
// Nom de fichier
// ---------------------------------------------------------------------------

/// Refuse un nom contenant un segment d'extension actif (`rapport.html`,
/// `facture.pdf.exe`, `x.svg.pdf`…). Un nom sans point n'est pas concerné.
pub fn controler_nom(nom: &str) -> Result<(), ErreurFichier> {
    let propre = sanitize_filename(nom).to_ascii_lowercase();
    let mut segs = propre.split('.');
    let _stem = segs.next();
    for seg in segs {
        if EXTENSIONS_ACTIVES.contains(&seg.trim()) {
            return Err(ErreurFichier::NomInvalide);
        }
    }
    Ok(())
}

/// Nom assaini (`sanitize_filename` de `presign`), borné, dont l'extension correspond
/// au type réel (ajoutée si absente ou incohérente).
pub fn nom_aligne(nom: &str, t: TypeFichier) -> String {
    let mut base = sanitize_filename(nom);
    if base.len() > NOM_MAX_OCTETS - 8 {
        let mut cut = NOM_MAX_OCTETS - 8;
        while !base.is_char_boundary(cut) {
            cut -= 1;
        }
        base.truncate(cut);
    }
    let ext_ok = base
        .rsplit_once('.')
        .map(|(_, e)| t.extensions().contains(&e.to_ascii_lowercase().as_str()))
        .unwrap_or(false);
    if ext_ok {
        base
    } else if base == "fichier" {
        format!("fichier.{}", t.extension())
    } else {
        format!("{base}.{}", t.extension())
    }
}

fn controler_declare(declare: Option<&str>, t: TypeFichier) -> Result<(), ErreurFichier> {
    let Some(d) = declare else { return Ok(()) };
    let d = d
        .split(';')
        .next()
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase();
    if d.is_empty() || d == "application/octet-stream" || d == "binary/octet-stream" {
        return Ok(());
    }
    let coherent = match TypeFichier::depuis_mime(&d) {
        Some(dt) => dt == t,
        // Types texte usuels (csv, markdown…) : compatibles avec un contenu texte.
        None => t == TypeFichier::Texte && d.starts_with("text/") && !d.contains("html"),
    };
    if coherent {
        Ok(())
    } else {
        Err(ErreurFichier::TypeDeclareIncoherent {
            declare: d.chars().take(80).collect(),
            detecte: t.mime(),
        })
    }
}

// ---------------------------------------------------------------------------
// Détection par signature
// ---------------------------------------------------------------------------

/// Détecte le format réel d'un contenu, ou refuse (fichier actif / inconnu).
pub fn detecter(b: &[u8]) -> Result<TypeFichier, ErreurFichier> {
    if let Some(m) = signature_executable(b) {
        return Err(ErreurFichier::FichierActif(m));
    }
    // Balisage (HTML, SVG, XML) : même précédé d'espaces ou d'un BOM.
    let tete = sans_bom_ni_blancs(b);
    if tete.first() == Some(&b'<') {
        return Err(ErreurFichier::FichierActif("HTML, SVG ou XML"));
    }
    if b.starts_with(b"%PDF-") {
        if let Some(m) = pdf_actif(b) {
            return Err(ErreurFichier::FichierActif(m));
        }
        return Ok(TypeFichier::Pdf);
    }
    if b.starts_with(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]) {
        return Ok(TypeFichier::Png);
    }
    if b.starts_with(&[0xFF, 0xD8, 0xFF]) {
        return Ok(TypeFichier::Jpeg);
    }
    if b.starts_with(b"GIF87a") || b.starts_with(b"GIF89a") {
        return Ok(TypeFichier::Gif);
    }
    if b.len() >= 12 && &b[0..4] == b"RIFF" && &b[8..12] == b"WEBP" {
        return Ok(TypeFichier::Webp);
    }
    if b.starts_with(b"PK\x03\x04") {
        return detecter_zip(b);
    }
    if b.starts_with(&[0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1]) {
        // Ancien format OLE (.doc/.xls/.ppt) : macros fréquentes, non géré.
        return Err(ErreurFichier::TypeInconnu);
    }
    if est_texte_sur(b)? {
        return Ok(TypeFichier::Texte);
    }
    Err(ErreurFichier::TypeInconnu)
}

fn signature_executable(b: &[u8]) -> Option<&'static str> {
    if b.starts_with(b"MZ") {
        return Some("exécutable Windows");
    }
    if b.starts_with(b"\x7fELF") {
        return Some("exécutable ELF");
    }
    if b.starts_with(b"#!") {
        return Some("script");
    }
    if b.len() >= 4 {
        let m = &b[0..4];
        if m == [0xFE, 0xED, 0xFA, 0xCE]
            || m == [0xFE, 0xED, 0xFA, 0xCF]
            || m == [0xCE, 0xFA, 0xED, 0xFE]
            || m == [0xCF, 0xFA, 0xED, 0xFE]
            || m == [0xCA, 0xFE, 0xBA, 0xBE]
        {
            return Some("exécutable Mach-O ou classe Java");
        }
    }
    None
}

fn sans_bom_ni_blancs(b: &[u8]) -> &[u8] {
    let mut s = b.strip_prefix(&[0xEF, 0xBB, 0xBF][..]).unwrap_or(b);
    while let Some((c, rest)) = s.split_first() {
        if c.is_ascii_whitespace() {
            s = rest;
        } else {
            break;
        }
    }
    s
}

/// Jeton PDF présent en clair, suivi d'un caractère de délimitation (évite `/JSON`).
fn pdf_a_jeton(b: &[u8], jeton: &[u8]) -> bool {
    let mut i = 0;
    while i + jeton.len() <= b.len() {
        if &b[i..i + jeton.len()] == jeton {
            let suite = b.get(i + jeton.len()).copied();
            if !matches!(suite, Some(c) if c.is_ascii_alphanumeric()) {
                return true;
            }
        }
        i += 1;
    }
    false
}

/// PDF actif repérable en clair. Heuristique : un PDF dont ces jetons sont cachés dans
/// un flux compressé n'est pas détecté ici (voir limites dans `docs/SECURITY.md`) ; le
/// service des fichiers impose en plus CSP `sandbox` et `nosniff`.
fn pdf_actif(b: &[u8]) -> Option<&'static str> {
    if pdf_a_jeton(b, b"/JavaScript") || pdf_a_jeton(b, b"/JS") {
        return Some("PDF avec JavaScript");
    }
    if pdf_a_jeton(b, b"/Launch") {
        return Some("PDF avec lancement de programme");
    }
    None
}

/// Texte UTF-8 sans octet nul ni caractère de contrôle exotique et sans balisage actif.
fn est_texte_sur(b: &[u8]) -> Result<bool, ErreurFichier> {
    let corps = b.strip_prefix(&[0xEF, 0xBB, 0xBF][..]).unwrap_or(b);
    let Ok(s) = std::str::from_utf8(corps) else {
        return Ok(false);
    };
    if s.chars()
        .any(|c| c.is_control() && !matches!(c, '\n' | '\r' | '\t' | '\u{000C}'))
    {
        return Ok(false);
    }
    let debut: String = s
        .chars()
        .take(TEXTE_BALISAGE_SCAN)
        .collect::<String>()
        .to_ascii_lowercase();
    for m in [
        "<script",
        "<html",
        "<svg",
        "<iframe",
        "<!doctype",
        "javascript:",
    ] {
        if debut.contains(m) {
            return Err(ErreurFichier::FichierActif("texte contenant du code actif"));
        }
    }
    Ok(true)
}

// ---- ZIP (OOXML / ODF) : répertoire central uniquement, aucune décompression ----

struct EntreeZip {
    nom: String,
    methode: u16,
    taille_comprimee: u32,
    offset_local: u32,
}

fn u16le(b: &[u8], o: usize) -> Option<u16> {
    Some(u16::from_le_bytes(b.get(o..o + 2)?.try_into().ok()?))
}
fn u32le(b: &[u8], o: usize) -> Option<u32> {
    Some(u32::from_le_bytes(b.get(o..o + 4)?.try_into().ok()?))
}

fn lire_zip(b: &[u8]) -> Option<Vec<EntreeZip>> {
    // Fin de répertoire central : signature 0x06054b50 dans les 64 Kio + 22 derniers octets.
    let min = b.len().saturating_sub(65_557);
    let eocd = (min..b.len().saturating_sub(21))
        .rev()
        .find(|&i| b.get(i..i + 4) == Some(b"PK\x05\x06"))?;
    let total = u16le(b, eocd + 10)? as usize;
    let cd_taille = u32le(b, eocd + 12)? as usize;
    let cd_offset = u32le(b, eocd + 16)? as usize;
    if total > ZIP_MAX_ENTREES || cd_offset.checked_add(cd_taille)? > eocd {
        return None;
    }
    let mut out = Vec::with_capacity(total);
    let mut p = cd_offset;
    for _ in 0..total {
        if b.get(p..p + 4)? != b"PK\x01\x02" {
            return None;
        }
        let methode = u16le(b, p + 10)?;
        let comp = u32le(b, p + 20)?;
        let n = u16le(b, p + 28)? as usize;
        let x = u16le(b, p + 30)? as usize;
        let c = u16le(b, p + 32)? as usize;
        let off = u32le(b, p + 42)?;
        let nom = String::from_utf8_lossy(b.get(p + 46..p + 46 + n)?).into_owned();
        out.push(EntreeZip {
            nom,
            methode,
            taille_comprimee: comp,
            offset_local: off,
        });
        p = p.checked_add(46 + n + x + c)?;
    }
    Some(out)
}

fn detecter_zip(b: &[u8]) -> Result<TypeFichier, ErreurFichier> {
    let entrees = lire_zip(b).ok_or(ErreurFichier::TypeInconnu)?;
    for e in &entrees {
        let n = e.nom.to_ascii_lowercase();
        if n.contains("..") || n.starts_with('/') || n.starts_with('\\') || n.contains(':') {
            return Err(ErreurFichier::FichierActif("archive à chemins dangereux"));
        }
        let fin = n.rsplit('/').next().unwrap_or("");
        if fin == "vbaproject.bin" || n.starts_with("basic/") || n.starts_with("scripts/") {
            return Err(ErreurFichier::FichierActif("document avec macros"));
        }
        if n.contains("/activex/") || n.starts_with("activex/") {
            return Err(ErreurFichier::FichierActif(
                "document avec contrôles ActiveX",
            ));
        }
        if let Some((_, ext)) = fin.rsplit_once('.') {
            if matches!(
                ext,
                "exe"
                    | "dll"
                    | "bat"
                    | "cmd"
                    | "com"
                    | "scr"
                    | "js"
                    | "vbs"
                    | "ps1"
                    | "jar"
                    | "msi"
            ) {
                return Err(ErreurFichier::FichierActif(
                    "document contenant un programme",
                ));
            }
        }
    }
    let a = |nom: &str| entrees.iter().any(|e| e.nom == nom);

    // ODF : premier élément « mimetype », non compressé, dont le contenu nomme le type.
    if let Some(m) = entrees.iter().find(|e| e.nom == "mimetype") {
        if m.offset_local == 0 && m.methode == 0 {
            let nl = u16le(b, 26).unwrap_or(0) as usize;
            let xl = u16le(b, 28).unwrap_or(0) as usize;
            let debut = 30 + nl + xl;
            let fin = debut + m.taille_comprimee as usize;
            if let Some(contenu) = b.get(debut..fin) {
                return match contenu {
                    b"application/vnd.oasis.opendocument.text" => Ok(TypeFichier::Odt),
                    b"application/vnd.oasis.opendocument.spreadsheet" => Ok(TypeFichier::Ods),
                    b"application/vnd.oasis.opendocument.presentation" => Ok(TypeFichier::Odp),
                    _ => Err(ErreurFichier::TypeInconnu),
                };
            }
        }
        return Err(ErreurFichier::TypeInconnu);
    }
    // OOXML : [Content_Types].xml + partie principale.
    if a("[Content_Types].xml") {
        if a("word/document.xml") {
            return Ok(TypeFichier::Docx);
        }
        if a("xl/workbook.xml") {
            return Ok(TypeFichier::Xlsx);
        }
        if a("ppt/presentation.xml") {
            return Ok(TypeFichier::Pptx);
        }
    }
    Err(ErreurFichier::TypeInconnu)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub const PDF: &[u8] =
        b"%PDF-1.4\n1 0 obj\n<< /Type /Catalog >>\nendobj\ntrailer\n<< /Root 1 0 R >>\n%%EOF\n";
    pub const PNG: &[u8] = &[
        0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 0, 0, 0, 0x0D, b'I', b'H', b'D', b'R',
    ];
    pub const JPEG: &[u8] = &[0xFF, 0xD8, 0xFF, 0xE0, 0, 0x10, b'J', b'F', b'I', b'F'];

    /// ZIP minimal (méthode « stockée ») à partir de (nom, contenu).
    pub fn zip(entrees: &[(&str, &[u8])]) -> Vec<u8> {
        let mut out = Vec::new();
        let mut cd = Vec::new();
        for (nom, data) in entrees {
            let off = out.len() as u32;
            out.extend_from_slice(b"PK\x03\x04");
            out.extend_from_slice(&[20, 0, 0, 0, 0, 0, 0, 0, 0, 0]); // ver, flags, méthode, heure, date
            out.extend_from_slice(&0u32.to_le_bytes()); // crc (non vérifié ici)
            out.extend_from_slice(&(data.len() as u32).to_le_bytes());
            out.extend_from_slice(&(data.len() as u32).to_le_bytes());
            out.extend_from_slice(&(nom.len() as u16).to_le_bytes());
            out.extend_from_slice(&0u16.to_le_bytes());
            out.extend_from_slice(nom.as_bytes());
            out.extend_from_slice(data);
            cd.extend_from_slice(b"PK\x01\x02");
            cd.extend_from_slice(&[20, 0, 20, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
            cd.extend_from_slice(&0u32.to_le_bytes());
            cd.extend_from_slice(&(data.len() as u32).to_le_bytes());
            cd.extend_from_slice(&(data.len() as u32).to_le_bytes());
            cd.extend_from_slice(&(nom.len() as u16).to_le_bytes());
            cd.extend_from_slice(&[0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
            cd.extend_from_slice(&off.to_le_bytes());
            cd.extend_from_slice(nom.as_bytes());
        }
        let cd_off = out.len() as u32;
        out.extend_from_slice(&cd);
        out.extend_from_slice(b"PK\x05\x06");
        out.extend_from_slice(&[0, 0, 0, 0]);
        out.extend_from_slice(&(entrees.len() as u16).to_le_bytes());
        out.extend_from_slice(&(entrees.len() as u16).to_le_bytes());
        out.extend_from_slice(&(cd.len() as u32).to_le_bytes());
        out.extend_from_slice(&cd_off.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out
    }

    fn pol() -> PolitiqueFichiers {
        PolitiqueFichiers::default()
    }

    #[test]
    fn formats_courants_reconnus() {
        assert_eq!(detecter(PDF), Ok(TypeFichier::Pdf));
        assert_eq!(detecter(PNG), Ok(TypeFichier::Png));
        assert_eq!(detecter(JPEG), Ok(TypeFichier::Jpeg));
        assert_eq!(detecter(b"GIF89a\x01\x00"), Ok(TypeFichier::Gif));
        assert_eq!(detecter(b"RIFF\x10\0\0\0WEBPVP8 "), Ok(TypeFichier::Webp));
        assert_eq!(detecter("Bonjour é\n".as_bytes()), Ok(TypeFichier::Texte));
    }

    #[test]
    fn ooxml_et_odf_reconnus() {
        let docx = zip(&[
            ("[Content_Types].xml", b"<x/>"),
            ("word/document.xml", b"<d/>"),
        ]);
        assert_eq!(detecter(&docx), Ok(TypeFichier::Docx));
        let xlsx = zip(&[
            ("[Content_Types].xml", b"<x/>"),
            ("xl/workbook.xml", b"<d/>"),
        ]);
        assert_eq!(detecter(&xlsx), Ok(TypeFichier::Xlsx));
        let pptx = zip(&[
            ("[Content_Types].xml", b"<x/>"),
            ("ppt/presentation.xml", b"<d/>"),
        ]);
        assert_eq!(detecter(&pptx), Ok(TypeFichier::Pptx));
        let odt = zip(&[
            ("mimetype", b"application/vnd.oasis.opendocument.text"),
            ("content.xml", b"<c/>"),
        ]);
        assert_eq!(detecter(&odt), Ok(TypeFichier::Odt));
        let ods = zip(&[(
            "mimetype",
            b"application/vnd.oasis.opendocument.spreadsheet",
        )]);
        assert_eq!(detecter(&ods), Ok(TypeFichier::Ods));
    }

    #[test]
    fn macros_et_zip_quelconque_refuses() {
        let docm = zip(&[
            ("[Content_Types].xml", b"<x/>"),
            ("word/document.xml", b"<d/>"),
            ("word/vbaProject.bin", b"x"),
        ]);
        assert!(matches!(
            detecter(&docm),
            Err(ErreurFichier::FichierActif(_))
        ));
        let zip_simple = zip(&[("a.txt", b"bonjour")]);
        assert_eq!(detecter(&zip_simple), Err(ErreurFichier::TypeInconnu));
        let traversal = zip(&[("../evil.txt", b"x")]);
        assert!(matches!(
            detecter(&traversal),
            Err(ErreurFichier::FichierActif(_))
        ));
        // ZIP tronqué / corrompu : pas de panique.
        assert_eq!(
            detecter(b"PK\x03\x04garbage"),
            Err(ErreurFichier::TypeInconnu)
        );
    }

    #[test]
    fn fichiers_actifs_refuses() {
        for contenu in [
            &b"<!DOCTYPE html><html><script>alert(1)</script></html>"[..],
            b"  \n<html><body>x</body></html>",
            b"<svg xmlns=\"http://www.w3.org/2000/svg\"><script>alert(1)</script></svg>",
            b"<?xml version=\"1.0\"?><svg onload=\"alert(1)\"/>",
            b"\xEF\xBB\xBF<svg/>",
            b"MZ\x90\x00\x03",
            b"\x7fELF\x02\x01",
            b"#!/bin/sh\nrm -rf /\n",
            b"\xCA\xFE\xBA\xBE\x00\x00",
        ] {
            assert!(
                matches!(detecter(contenu), Err(ErreurFichier::FichierActif(_))),
                "{:?}",
                String::from_utf8_lossy(contenu)
            );
        }
        // Texte avec balisage actif non initial.
        assert!(matches!(
            detecter(b"bonjour <script>alert(1)</script>"),
            Err(ErreurFichier::FichierActif(_))
        ));
    }

    #[test]
    fn pdf_actif_refuse_mais_pas_json() {
        let js = b"%PDF-1.4\n1 0 obj << /S /JavaScript /JS (app.alert(1)) >> endobj";
        assert!(matches!(detecter(js), Err(ErreurFichier::FichierActif(_))));
        let launch = b"%PDF-1.4\n<< /S /Launch /F (cmd.exe) >>";
        assert!(matches!(
            detecter(launch),
            Err(ErreurFichier::FichierActif(_))
        ));
        assert_eq!(
            detecter(b"%PDF-1.4\n<< /Type /JSONish >>"),
            Ok(TypeFichier::Pdf)
        );
    }

    #[test]
    fn binaire_inconnu_et_ole_refuses() {
        assert_eq!(
            detecter(&[0, 1, 2, 3, 4, 5]),
            Err(ErreurFichier::TypeInconnu)
        );
        assert_eq!(
            detecter(&[0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1, 0]),
            Err(ErreurFichier::TypeInconnu)
        );
        assert_eq!(detecter(b"abc\0def"), Err(ErreurFichier::TypeInconnu));
    }

    #[test]
    fn vrai_pdf_accepte_et_type_reel_retenu() {
        let v = pol()
            .valider(
                UsageFichier::Document,
                "Certificat.pdf",
                Some("application/pdf"),
                PDF,
            )
            .unwrap();
        assert_eq!(v.mime, "application/pdf");
        assert_eq!(v.nom, "Certificat.pdf");
        // Type déclaré générique : accepté, type réel retenu.
        let v = pol()
            .valider(
                UsageFichier::Document,
                "scan",
                Some("application/octet-stream"),
                PNG,
            )
            .unwrap();
        assert_eq!(v.mime, "image/png");
        assert_eq!(v.nom, "scan.png");
    }

    #[test]
    fn html_renomme_pdf_refuse() {
        let html = b"<html><body onload=alert(1)></body></html>";
        let e = pol()
            .valider(
                UsageFichier::Document,
                "facture.pdf",
                Some("application/pdf"),
                html,
            )
            .unwrap_err();
        assert!(matches!(e, ErreurFichier::FichierActif(_)));
    }

    #[test]
    fn svg_avec_script_refuse_meme_declare_image() {
        let svg = br#"<svg xmlns="http://www.w3.org/2000/svg"><script>alert(document.cookie)</script></svg>"#;
        let e = pol()
            .valider(UsageFichier::Photo, "photo.png", Some("image/png"), svg)
            .unwrap_err();
        assert!(matches!(e, ErreurFichier::FichierActif(_)));
    }

    #[test]
    fn taille_depassee_refusee() {
        let p = PolitiqueFichiers::nouvelle(TaillesMax {
            piece_message: 64,
            ..TaillesMax::default()
        });
        let mut gros = PDF.to_vec();
        gros.resize(65, b' ');
        assert_eq!(
            p.valider(UsageFichier::PieceMessage, "a.pdf", None, &gros),
            Err(ErreurFichier::TropGros { max: 64 })
        );
        assert!(p
            .valider(
                UsageFichier::PieceMessage,
                "a.pdf",
                None,
                b"%PDF-1.4\n%%EOF"
            )
            .is_ok());
        assert_eq!(
            p.valider(UsageFichier::Document, "a.pdf", None, b""),
            Err(ErreurFichier::Vide)
        );
    }

    #[test]
    fn photo_non_image_refusee() {
        assert!(matches!(
            pol().valider(UsageFichier::Photo, "photo.jpg", Some("image/jpeg"), PDF),
            Err(ErreurFichier::TypeNonAutorise { .. })
        ));
        assert!(pol()
            .valider(UsageFichier::Photo, "photo.jpg", Some("image/jpeg"), JPEG)
            .is_ok());
        // GIF : image, mais hors liste blanche « photo ».
        assert!(matches!(
            pol().valider(UsageFichier::Photo, "p.gif", None, b"GIF89a\x01\x00"),
            Err(ErreurFichier::TypeNonAutorise { .. })
        ));
    }

    #[test]
    fn pai_limite_a_pdf_et_images() {
        assert!(pol()
            .valider(UsageFichier::PiecePai, "o.pdf", None, PDF)
            .is_ok());
        assert!(matches!(
            pol().valider(UsageFichier::PiecePai, "o.txt", None, b"texte"),
            Err(ErreurFichier::TypeNonAutorise { .. })
        ));
    }

    #[test]
    fn type_declare_contradictoire_refuse() {
        assert!(matches!(
            pol().valider(
                UsageFichier::Document,
                "a.pdf",
                Some("application/pdf"),
                PNG
            ),
            Err(ErreurFichier::TypeDeclareIncoherent { .. })
        ));
        // Texte annoncé en CSV : compatible.
        assert!(pol()
            .valider(
                UsageFichier::Document,
                "a.csv",
                Some("text/csv"),
                b"a;b\n1;2\n"
            )
            .is_ok());
        // HTML annoncé « text/html » sur un texte : refusé.
        assert!(pol()
            .valider(
                UsageFichier::Document,
                "a.txt",
                Some("text/html"),
                b"bonjour"
            )
            .is_err());
    }

    #[test]
    fn noms_assainis_et_extensions_actives_refusees() {
        assert_eq!(
            controler_nom("rapport.html"),
            Err(ErreurFichier::NomInvalide)
        );
        assert_eq!(
            controler_nom("facture.pdf.exe"),
            Err(ErreurFichier::NomInvalide)
        );
        assert_eq!(controler_nom("x.svg.pdf"), Err(ErreurFichier::NomInvalide));
        assert!(controler_nom("Dr. Martin - bilan.pdf").is_ok());
        // Traversée de chemin et caractères de contrôle neutralisés.
        let v = pol()
            .valider(
                UsageFichier::Document,
                "../../etc/passwd\r\n",
                None,
                b"texte",
            )
            .unwrap();
        assert!(!v.nom.contains('/') && !v.nom.contains('\r') && !v.nom.contains('\n'));
        assert!(v.nom.ends_with(".txt"));
        // Extension incohérente remplacée par l'extension réelle.
        assert_eq!(nom_aligne("scan.jpg", TypeFichier::Png), "scan.jpg.png");
        assert_eq!(nom_aligne("", TypeFichier::Pdf), "fichier.pdf");
        // Nom très long borné.
        let long = "a".repeat(400);
        assert!(nom_aligne(&long, TypeFichier::Pdf).len() <= 200);
    }

    #[test]
    fn antivirus_branchable_et_fail_closed() {
        struct Refus;
        impl AnalyseurAntivirus for Refus {
            fn nom(&self) -> &'static str {
                "refus"
            }
            fn analyser(&self, _: &ContexteAnalyse<'_>) -> VerdictAnalyse {
                VerdictAnalyse::Infecte("Test.EICAR".into())
            }
        }
        struct Panne;
        impl AnalyseurAntivirus for Panne {
            fn nom(&self) -> &'static str {
                "panne"
            }
            fn analyser(&self, _: &ContexteAnalyse<'_>) -> VerdictAnalyse {
                VerdictAnalyse::Indisponible
            }
        }
        assert_eq!(pol().nom_analyseur(), "aucune");
        let p = pol().avec_analyseur(Arc::new(Refus));
        assert_eq!(
            p.valider(UsageFichier::Document, "a.pdf", None, PDF),
            Err(ErreurFichier::Infecte("Test.EICAR".into()))
        );
        let p = pol().avec_analyseur(Arc::new(Panne));
        assert_eq!(
            p.valider(UsageFichier::Document, "a.pdf", None, PDF),
            Err(ErreurFichier::AnalyseIndisponible)
        );
    }

    #[test]
    fn tailles_depuis_env_bornees() {
        let env = |k: &str| match k {
            "ENT_UPLOAD_MAX_DOCUMENT" => Some("2m".to_string()),
            "ENT_UPLOAD_MAX_PHOTO" => Some("10".to_string()), // < plancher
            "ENT_UPLOAD_MAX_MESSAGE" => Some("999999999m".to_string()), // > plafond
            "ENT_UPLOAD_MAX_PAI" => Some("pas un nombre".to_string()),
            _ => None,
        };
        let t = TaillesMax::depuis(env);
        assert_eq!(t.document, 2 * MIB);
        assert_eq!(t.photo, KIB);
        assert_eq!(t.piece_message, PLAFOND_MAX_OCTETS);
        assert_eq!(t.piece_pai, 256 * KIB);
    }
}
