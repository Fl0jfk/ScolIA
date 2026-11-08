//! Socle 2 (l'élève), lot E2 — identité versionnée (logique pure, sans base).
//!
//! Tout se modifie, rien ne s'efface : une modification crée une **version** qui garde
//! sa date d'effet, sa nature (`correction` d'une erreur ≠ `changement_etat_civil`
//! réel ≠ `usage`), sa source, son auteur et sa pièce justificative. Une correction
//! **remplace** une version (elle reste lisible) ; un changement ajoute une version
//! datée. Aucune valeur d'identité ne sort de ce module vers un événement : seuls les
//! NOMS des champs modifiés ([`ChampIdentite::nom`]) y figurent.

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Nature d'une version (CHECK de `ent_eleve_identite.nature`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NatureVersion {
    Creation,
    Reprise,
    Import,
    Correction,
    ChangementEtatCivil,
    Usage,
    Fusion,
}

impl NatureVersion {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Creation => "creation",
            Self::Reprise => "reprise",
            Self::Import => "import",
            Self::Correction => "correction",
            Self::ChangementEtatCivil => "changement_etat_civil",
            Self::Usage => "usage",
            Self::Fusion => "fusion",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "creation" => Self::Creation,
            "reprise" => Self::Reprise,
            "import" => Self::Import,
            "correction" => Self::Correction,
            "changement_etat_civil" => Self::ChangementEtatCivil,
            "usage" => Self::Usage,
            "fusion" => Self::Fusion,
            _ => return None,
        })
    }
}

/// Champs de l'identité (noms stables, seuls exposés dans les événements et le journal).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ChampIdentite {
    NomNaissance,
    NomUsage,
    Prenoms,
    RangPrenomUsuel,
    PrenomUsage,
    Sexe,
    DateNaissance,
    NaissanceCommune,
    NaissanceCommuneCog,
    NaissancePaysCog,
    NaissanceLieuEtranger,
    DateNaissancePrecision,
    DateNaissanceStatut,
    NomEcritureOrigine,
    PrenomsEcritureOrigine,
}

impl ChampIdentite {
    pub fn nom(self) -> &'static str {
        match self {
            Self::NomNaissance => "nom_naissance",
            Self::NomUsage => "nom_usage",
            Self::Prenoms => "prenoms",
            Self::RangPrenomUsuel => "rang_prenom_usuel",
            Self::PrenomUsage => "prenom_usage",
            Self::Sexe => "sexe",
            Self::DateNaissance => "date_naissance",
            Self::NaissanceCommune => "naissance_commune",
            Self::NaissanceCommuneCog => "naissance_commune_cog",
            Self::NaissancePaysCog => "naissance_pays_cog",
            Self::NaissanceLieuEtranger => "naissance_lieu_etranger",
            Self::DateNaissancePrecision => "date_naissance_precision",
            Self::DateNaissanceStatut => "date_naissance_statut",
            Self::NomEcritureOrigine => "nom_ecriture_origine",
            Self::PrenomsEcritureOrigine => "prenoms_ecriture_origine",
        }
    }
}

/// Une version d'identité (ligne de `ent_eleve_identite`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IdentiteVersion {
    pub id: Uuid,
    pub etablissement_id: Uuid,
    pub eleve_id: Uuid,
    pub nom_naissance: String,
    pub nom_usage: Option<String>,
    pub prenoms: Vec<String>,
    /// Rang (à partir de 1) du prénom usuel parmi les prénoms.
    pub rang_prenom_usuel: i16,
    pub prenom_usage: Option<String>,
    pub sexe: Option<String>,
    pub date_naissance: Option<NaiveDate>,
    /// `jour`, `mois`, `annee` ou `inconnue` (jamais de « 1er janvier » inventé).
    pub date_naissance_precision: String,
    /// `declaree`, `justifiee`, `presumee` ou `contestee`.
    pub date_naissance_statut: String,
    pub naissance_commune_cog: Option<String>,
    pub naissance_commune: Option<String>,
    pub naissance_pays_cog: Option<String>,
    pub naissance_lieu_etranger: Option<String>,
    /// Graphie d'origine (écriture non latine), facultative.
    pub nom_ecriture_origine: Option<String>,
    pub prenoms_ecriture_origine: Option<String>,
    pub nature: NatureVersion,
    pub effet_du: NaiveDate,
    pub remplace_version_id: Option<Uuid>,
    pub remplacee_par: Option<Uuid>,
    /// Motif codé (liste du logiciel, voir [`MOTIFS_CORRECTION`], [`MOTIFS_CHANGEMENT`]).
    pub motif_code: Option<String>,
    /// Texte libre : obligatoire seulement pour le code `autre`.
    pub motif: Option<String>,
    pub piece_document_id: Option<Uuid>,
    pub source: String,
    pub source_ref: Option<Uuid>,
    pub sensible: bool,
    pub enregistre_le: DateTime<Utc>,
    pub enregistre_par: Option<Uuid>,
}

/// Modification demandée : chaque champ `None` est inchangé ; pour les champs
/// facultatifs, `Some(None)` le retire.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModifIdentite {
    pub nom_naissance: Option<String>,
    pub nom_usage: Option<Option<String>>,
    pub prenoms: Option<Vec<String>>,
    pub rang_prenom_usuel: Option<i16>,
    pub prenom_usage: Option<Option<String>>,
    pub sexe: Option<Option<String>>,
    pub date_naissance: Option<Option<NaiveDate>>,
    pub naissance_commune_cog: Option<Option<String>>,
    pub naissance_commune: Option<Option<String>>,
    pub naissance_pays_cog: Option<Option<String>>,
    pub naissance_lieu_etranger: Option<Option<String>>,
    pub date_naissance_precision: Option<String>,
    pub date_naissance_statut: Option<String>,
    pub nom_ecriture_origine: Option<Option<String>>,
    pub prenoms_ecriture_origine: Option<Option<String>>,
}

/// Erreur de planification. Les messages sont fixes et ne contiennent aucune donnée
/// personnelle (ils remontent tels quels à l'utilisateur et aux journaux).
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ErreurIdentite {
    #[error("{0}")]
    Invalide(&'static str),
    #[error("{0}")]
    Conflit(&'static str),
}

/// Commande de planification (la nature décide des règles).
#[derive(Debug, Clone)]
pub struct DemandeVersion {
    pub nature: NatureVersion,
    pub modif: ModifIdentite,
    /// Date d'effet demandée (ignorée pour une correction : celle de la version corrigée).
    pub effet_du: Option<NaiveDate>,
    pub motif_code: Option<String>,
    pub motif: Option<String>,
    pub piece_document_id: Option<Uuid>,
    pub source: String,
    pub source_ref: Option<Uuid>,
    pub acteur: Option<Uuid>,
}

/// Résultat : la nouvelle version, les noms des champs réellement modifiés.
#[derive(Debug, Clone)]
pub struct VersionPlanifiee {
    pub version: IdentiteVersion,
    pub champs: Vec<ChampIdentite>,
}

/// Motifs d'une **correction** (la valeur était fausse depuis le début).
pub const MOTIFS_CORRECTION: &[&str] = &[
    "faute_de_frappe",
    "casse_accents",
    "erreur_de_transcription",
    "d_apres_piece",
    "erreur_d_import",
    "autre",
];
/// Motifs d'un **changement réel** d'état civil.
pub const MOTIFS_CHANGEMENT: &[&str] = &[
    "decision_etat_civil",
    "adoption",
    "changement_prenom",
    "changement_mention_sexe",
    "reconnaissance",
    "protection",
    "autre",
];
/// Motifs d'un nom ou prénom d'usage (facultatifs).
pub const MOTIFS_USAGE: &[&str] = &["demande_famille", "demande_eleve", "autre"];
/// Motifs qui **forcent** la sensibilité (non modifiable par l'opérateur).
pub const MOTIFS_SENSIBLES: &[&str] = &[
    "adoption",
    "changement_prenom",
    "changement_mention_sexe",
    "protection",
];
const PRECISIONS: &[&str] = &["jour", "mois", "annee", "inconnue"];
const STATUTS_NAISSANCE: &[&str] = &["declaree", "justifiee", "presumee", "contestee"];

const SOURCES: &[&str] = &[
    "saisie",
    "admission",
    "import",
    "famille",
    "eleve",
    "ancien_etablissement",
    "reprise",
    "fusion",
];

/// Source valide pour une commande saisie par une personne.
pub fn source_valide(s: &str) -> bool {
    SOURCES.contains(&s)
}

/// Espaces normalisés (trim, espaces multiples réduits), apostrophes typographiques
/// ramenées à l'apostrophe droite.
fn nettoyer(s: &str) -> String {
    s.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .replace(['’', '‘', 'ʼ'], "'")
}

/// Contrôle d'un nom ou prénom : 1 à 120 caractères, ni chiffre ni caractère de contrôle.
fn nom_propre(s: &str, vide: &'static str) -> Result<String, ErreurIdentite> {
    let n = nettoyer(s);
    if n.is_empty() {
        return Err(ErreurIdentite::Invalide(vide));
    }
    if n.chars().count() > 120 {
        return Err(ErreurIdentite::Invalide(
            "nom ou prénom trop long (120 caractères au plus)",
        ));
    }
    if n.chars().any(|c| c.is_control() || c.is_numeric()) {
        return Err(ErreurIdentite::Invalide(
            "un nom ou un prénom ne contient ni chiffre ni caractère de contrôle",
        ));
    }
    Ok(n)
}

fn facultatif(v: Option<String>, vide: &'static str) -> Result<Option<String>, ErreurIdentite> {
    match v {
        None => Ok(None),
        Some(s) if nettoyer(&s).is_empty() => Ok(None),
        Some(s) => nom_propre(&s, vide).map(Some),
    }
}

fn texte_libre(v: Option<String>) -> Result<Option<String>, ErreurIdentite> {
    match v {
        None => Ok(None),
        Some(s) => {
            let n = nettoyer(&s);
            if n.is_empty() {
                Ok(None)
            } else if n.chars().count() > 120 || n.chars().any(|c| c.is_control()) {
                Err(ErreurIdentite::Invalide("lieu de naissance invalide"))
            } else {
                Ok(Some(n))
            }
        }
    }
}

fn code_court(v: Option<String>) -> Result<Option<String>, ErreurIdentite> {
    match v {
        None => Ok(None),
        Some(s) => {
            let n = s.trim().to_ascii_uppercase();
            if n.is_empty() {
                Ok(None)
            } else if n.len() > 10 || !n.chars().all(|c| c.is_ascii_alphanumeric()) {
                Err(ErreurIdentite::Invalide("code géographique invalide"))
            } else {
                Ok(Some(n))
            }
        }
    }
}

/// Applique `modif` sur `base` ; renvoie les champs modifiés (valeurs normalisées).
fn appliquer(
    base: &IdentiteVersion,
    modif: &ModifIdentite,
    aujourdhui: NaiveDate,
) -> Result<(IdentiteVersion, Vec<ChampIdentite>), ErreurIdentite> {
    let mut v = base.clone();
    let mut champs = Vec::new();
    let maj = |champ: ChampIdentite, change: bool, champs: &mut Vec<ChampIdentite>| {
        if change {
            champs.push(champ);
        }
    };

    if let Some(n) = &modif.nom_naissance {
        let n = nom_propre(n, "le nom de naissance est obligatoire")?;
        maj(
            ChampIdentite::NomNaissance,
            n != v.nom_naissance,
            &mut champs,
        );
        v.nom_naissance = n;
    }
    if let Some(ps) = &modif.prenoms {
        if ps.is_empty() || ps.len() > 10 {
            return Err(ErreurIdentite::Invalide("1 à 10 prénoms"));
        }
        let ps = ps
            .iter()
            .map(|p| nom_propre(p, "un prénom est vide"))
            .collect::<Result<Vec<_>, _>>()?;
        maj(ChampIdentite::Prenoms, ps != v.prenoms, &mut champs);
        v.prenoms = ps;
    }
    if let Some(r) = modif.rang_prenom_usuel {
        maj(
            ChampIdentite::RangPrenomUsuel,
            r != v.rang_prenom_usuel,
            &mut champs,
        );
        v.rang_prenom_usuel = r;
    }
    if v.rang_prenom_usuel < 1 || v.rang_prenom_usuel as usize > v.prenoms.len() {
        return Err(ErreurIdentite::Invalide(
            "le prénom usuel doit être l'un des prénoms",
        ));
    }
    if let Some(u) = &modif.nom_usage {
        let u = facultatif(u.clone(), "nom d'usage vide")?;
        maj(ChampIdentite::NomUsage, u != v.nom_usage, &mut champs);
        v.nom_usage = u;
    }
    if let Some(u) = &modif.prenom_usage {
        let u = facultatif(u.clone(), "prénom d'usage vide")?;
        maj(ChampIdentite::PrenomUsage, u != v.prenom_usage, &mut champs);
        v.prenom_usage = u;
    }
    if let Some(s) = &modif.sexe {
        let s = match s.as_deref().map(str::trim) {
            None | Some("") => None,
            Some("M") => Some("M".to_string()),
            Some("F") => Some("F".to_string()),
            Some(_) => return Err(ErreurIdentite::Invalide("sexe : M ou F")),
        };
        maj(ChampIdentite::Sexe, s != v.sexe, &mut champs);
        v.sexe = s;
    }
    if let Some(d) = modif.date_naissance {
        if let Some(d) = d {
            if d > aujourdhui {
                return Err(ErreurIdentite::Invalide(
                    "la date de naissance est dans le futur",
                ));
            }
            if d < NaiveDate::from_ymd_opt(1900, 1, 1).expect("date valide") {
                return Err(ErreurIdentite::Invalide(
                    "date de naissance invraisemblable",
                ));
            }
        }
        maj(
            ChampIdentite::DateNaissance,
            d != v.date_naissance,
            &mut champs,
        );
        v.date_naissance = d;
    }
    if let Some(x) = &modif.naissance_commune {
        let x = texte_libre(x.clone())?;
        maj(
            ChampIdentite::NaissanceCommune,
            x != v.naissance_commune,
            &mut champs,
        );
        v.naissance_commune = x;
    }
    if let Some(x) = &modif.naissance_commune_cog {
        let x = code_court(x.clone())?;
        maj(
            ChampIdentite::NaissanceCommuneCog,
            x != v.naissance_commune_cog,
            &mut champs,
        );
        v.naissance_commune_cog = x;
    }
    if let Some(x) = &modif.naissance_pays_cog {
        let x = code_court(x.clone())?;
        maj(
            ChampIdentite::NaissancePaysCog,
            x != v.naissance_pays_cog,
            &mut champs,
        );
        v.naissance_pays_cog = x;
    }
    if let Some(x) = &modif.naissance_lieu_etranger {
        let x = texte_libre(x.clone())?;
        maj(
            ChampIdentite::NaissanceLieuEtranger,
            x != v.naissance_lieu_etranger,
            &mut champs,
        );
        v.naissance_lieu_etranger = x;
    }
    if let Some(x) = &modif.date_naissance_precision {
        if !PRECISIONS.contains(&x.as_str()) {
            return Err(ErreurIdentite::Invalide(
                "précision de la date de naissance inconnue",
            ));
        }
        maj(
            ChampIdentite::DateNaissancePrecision,
            *x != v.date_naissance_precision,
            &mut champs,
        );
        v.date_naissance_precision = x.clone();
    }
    if let Some(x) = &modif.date_naissance_statut {
        if !STATUTS_NAISSANCE.contains(&x.as_str()) {
            return Err(ErreurIdentite::Invalide(
                "statut de la date de naissance inconnu",
            ));
        }
        maj(
            ChampIdentite::DateNaissanceStatut,
            *x != v.date_naissance_statut,
            &mut champs,
        );
        v.date_naissance_statut = x.clone();
    }
    // Une date inconnue n'a pas de valeur ; une date partielle reste ancrée au 1er du
    // mois / de l'année (la précision dit qu'elle est incomplète).
    if v.date_naissance_precision == "inconnue" && v.date_naissance.is_some() {
        if modif.date_naissance_precision.is_some() && modif.date_naissance.is_none() {
            v.date_naissance = None;
            maj(ChampIdentite::DateNaissance, true, &mut champs);
        } else {
            return Err(ErreurIdentite::Invalide(
                "une date de naissance inconnue n'a pas de valeur",
            ));
        }
    }
    if let Some(x) = &modif.nom_ecriture_origine {
        let x = texte_libre(x.clone())?;
        maj(
            ChampIdentite::NomEcritureOrigine,
            x != v.nom_ecriture_origine,
            &mut champs,
        );
        v.nom_ecriture_origine = x;
    }
    if let Some(x) = &modif.prenoms_ecriture_origine {
        let x = texte_libre(x.clone())?;
        maj(
            ChampIdentite::PrenomsEcritureOrigine,
            x != v.prenoms_ecriture_origine,
            &mut champs,
        );
        v.prenoms_ecriture_origine = x;
    }
    champs.sort();
    Ok((v, champs))
}

/// Une version est **sensible** quand son existence révèle une information intime :
/// prénom d'usage, mention de sexe, prénoms de l'état civil. La sensibilité est **collante** :
/// une version construite sur une version sensible en recopie les valeurs et l'est donc
/// aussi (sinon un lecteur sans G6 les déduirait de la version suivante). Les valeurs ne sont jamais copiées hors de la version
/// (événements : noms de champs seulement).
fn est_sensible(
    champs: &[ChampIdentite],
    nature: NatureVersion,
    heritee: bool,
    motif_code: Option<&str>,
) -> bool {
    if motif_code.is_some_and(|m| MOTIFS_SENSIBLES.contains(&m)) {
        return true;
    }
    let intime = champs.iter().any(|c| {
        matches!(
            c,
            ChampIdentite::PrenomUsage
                | ChampIdentite::Sexe
                | ChampIdentite::Prenoms
                | ChampIdentite::RangPrenomUsuel
        )
    });
    match nature {
        // Une simple correction de prénoms (faute de frappe) n'est pas intime ; changer le
        // sexe ou poser un prénom d'usage l'est toujours.
        NatureVersion::Correction => {
            heritee
                || champs
                    .iter()
                    .any(|c| matches!(c, ChampIdentite::PrenomUsage | ChampIdentite::Sexe))
        }
        _ => intime || heritee,
    }
}

fn motif_valide(m: &Option<String>) -> Result<Option<String>, ErreurIdentite> {
    match m {
        None => Ok(None),
        Some(s) => {
            let n = s.trim();
            if n.is_empty() {
                Ok(None)
            } else if n.chars().count() > 500 || n.chars().any(|c| c.is_control() && c != '\n') {
                Err(ErreurIdentite::Invalide(
                    "motif invalide (500 caractères au plus)",
                ))
            } else {
                Ok(Some(n.to_string()))
            }
        }
    }
}

/// Planifie la version qui résulte d'une commande.
///
/// * `base` : pour une **correction**, la version corrigée (non remplacée) ; pour un
///   **changement** ou un **usage**, la version en vigueur à la date d'effet.
/// * Un `usage` daté d'un jour où une version existe déjà la remplace.
/// * `existantes_effet` : dates d'effet des versions non remplacées de l'élève
///   (un changement ne peut pas viser une date déjà occupée : c'est une correction).
pub fn planifier_version(
    base: &IdentiteVersion,
    existantes_effet: &[NaiveDate],
    d: DemandeVersion,
    aujourdhui: NaiveDate,
) -> Result<VersionPlanifiee, ErreurIdentite> {
    if !source_valide(&d.source) {
        return Err(ErreurIdentite::Invalide("source inconnue"));
    }
    let motif = motif_valide(&d.motif)?;
    match d.nature {
        NatureVersion::Correction | NatureVersion::ChangementEtatCivil | NatureVersion::Usage => {}
        _ => return Err(ErreurIdentite::Invalide("nature réservée au système")),
    }
    let motif_code = d
        .motif_code
        .clone()
        .map(|m| m.trim().to_string())
        .filter(|m| !m.is_empty());
    let liste: &[&str] = match d.nature {
        NatureVersion::Correction => MOTIFS_CORRECTION,
        NatureVersion::ChangementEtatCivil => MOTIFS_CHANGEMENT,
        _ => MOTIFS_USAGE,
    };
    if matches!(
        d.nature,
        NatureVersion::Correction | NatureVersion::ChangementEtatCivil
    ) && motif_code.is_none()
    {
        return Err(ErreurIdentite::Invalide("le motif (code) est obligatoire"));
    }
    if motif_code.as_deref().is_some_and(|m| !liste.contains(&m)) {
        return Err(ErreurIdentite::Invalide(
            "motif inconnu pour cette nature de modification",
        ));
    }
    if motif_code.as_deref() == Some("autre") && motif.is_none() {
        return Err(ErreurIdentite::Invalide(
            "le motif « autre » exige une précision en texte",
        ));
    }
    if d.nature == NatureVersion::ChangementEtatCivil && d.piece_document_id.is_none() {
        return Err(ErreurIdentite::Invalide(
            "la pièce justificative est obligatoire pour un changement d'état civil",
        ));
    }
    if d.nature == NatureVersion::Usage {
        // Un usage ne touche ni l'état civil ni la naissance : seulement nom/prénom d'usage.
        let m = &d.modif;
        let hors_usage = m.nom_naissance.is_some()
            || m.prenoms.is_some()
            || m.rang_prenom_usuel.is_some()
            || m.sexe.is_some()
            || m.date_naissance.is_some()
            || m.naissance_commune.is_some()
            || m.naissance_commune_cog.is_some()
            || m.naissance_pays_cog.is_some()
            || m.naissance_lieu_etranger.is_some();
        if hors_usage {
            return Err(ErreurIdentite::Invalide(
                "un nom ou prénom d'usage ne modifie pas l'état civil",
            ));
        }
    }
    if base.remplacee_par.is_some() {
        return Err(ErreurIdentite::Conflit("cette version a déjà été corrigée"));
    }

    let (mut version, champs) = appliquer(base, &d.modif, aujourdhui)?;
    if champs.is_empty() {
        return Err(ErreurIdentite::Invalide("aucune modification"));
    }

    let effet_du = match d.nature {
        NatureVersion::Correction => base.effet_du,
        _ => {
            let e = d.effet_du.unwrap_or(aujourdhui);
            // Une version existe déjà à cette date : un nom ou prénom d'usage la
            // remplace (deux valeurs ne peuvent pas valoir le même jour, l'ancienne
            // reste lisible) ; un changement d'état civil est une correction.
            let usage_meme_jour = d.nature == NatureVersion::Usage && base.effet_du == e;
            if existantes_effet.contains(&e) && !usage_meme_jour {
                return Err(ErreurIdentite::Conflit(
                    "une version existe déjà à cette date d'effet : utiliser une correction",
                ));
            }
            e
        }
    };
    // Un changement ne s'applique pas avant la naissance supposée ni trop loin dans le futur.
    if effet_du > aujourdhui + chrono::Duration::days(366) {
        return Err(ErreurIdentite::Invalide("date d'effet trop lointaine"));
    }
    if effet_du < NaiveDate::from_ymd_opt(1900, 1, 1).expect("date valide") {
        return Err(ErreurIdentite::Invalide("date d'effet invraisemblable"));
    }

    version.id = Uuid::new_v4();
    version.nature = d.nature;
    version.effet_du = effet_du;
    let remplace = d.nature == NatureVersion::Correction
        || (d.nature == NatureVersion::Usage && base.effet_du == effet_du);
    version.remplace_version_id = remplace.then_some(base.id);
    version.remplacee_par = None;
    version.motif_code = motif_code.clone();
    version.motif = motif;
    version.piece_document_id = d.piece_document_id;
    version.source = d.source;
    version.source_ref = d.source_ref;
    version.sensible = est_sensible(&champs, d.nature, base.sensible, motif_code.as_deref());
    version.enregistre_le = Utc::now();
    version.enregistre_par = d.acteur;
    Ok(VersionPlanifiee { version, champs })
}

/// Vue d'historique filtrée par les droits du lecteur : une version sensible n'apparaît
/// qu'avec le droit G6 ; un lien vers une version masquée est effacé (aucune trace de
/// l'existence d'une version sensible).
pub fn filtrer_historique(
    mut versions: Vec<IdentiteVersion>,
    voit_sensible: bool,
) -> Vec<IdentiteVersion> {
    if voit_sensible {
        return versions;
    }
    let masquees: std::collections::HashSet<Uuid> = versions
        .iter()
        .filter(|v| v.sensible)
        .map(|v| v.id)
        .collect();
    // RP2-02 : l'ancienne identité d'un changement d'état civil sensible est scellée à son tour
    // (sinon le secrétariat lit « Chang » après l'adoption en « Diggory »).
    let mut scellees: std::collections::HashSet<Uuid> = std::collections::HashSet::new();
    // Un changement d'état civil ne remplace pas l'ancienne version : elle lui succède par la
    // date d'effet. Tout ce qui est antérieur est l'ancienne identité. Le secrétariat lit
    // l'identité courante sur la fiche ; le prénom d'usage et les corrections ne scellent rien.
    for s in versions
        .iter()
        .filter(|v| v.sensible && v.nature == NatureVersion::ChangementEtatCivil)
    {
        for ancienne in versions.iter().filter(|v| v.effet_du < s.effet_du) {
            scellees.insert(ancienne.id);
        }
    }
    let masquees: std::collections::HashSet<Uuid> = masquees.union(&scellees).copied().collect();
    versions.retain(|v| !masquees.contains(&v.id));
    for v in &mut versions {
        if v.remplace_version_id
            .is_some_and(|id| masquees.contains(&id))
        {
            v.remplace_version_id = None;
        }
        if v.remplacee_par.is_some_and(|id| masquees.contains(&id)) {
            // Remplacée par une version masquée : aucune trace de son existence.
            v.remplacee_par = None;
        }
    }
    versions
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(y: i32, m: u32, j: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, j).unwrap()
    }

    fn base() -> IdentiteVersion {
        IdentiteVersion {
            id: Uuid::new_v4(),
            etablissement_id: Uuid::new_v4(),
            eleve_id: Uuid::new_v4(),
            nom_naissance: "Dupond".into(),
            nom_usage: None,
            prenoms: vec!["Jean".into(), "Pierre".into()],
            rang_prenom_usuel: 1,
            prenom_usage: None,
            sexe: Some("M".into()),
            date_naissance: Some(d(2010, 3, 14)),
            date_naissance_precision: "jour".into(),
            date_naissance_statut: "declaree".into(),
            nom_ecriture_origine: None,
            prenoms_ecriture_origine: None,
            motif_code: None,
            naissance_commune_cog: None,
            naissance_commune: None,
            naissance_pays_cog: None,
            naissance_lieu_etranger: None,
            nature: NatureVersion::Creation,
            effet_du: d(2024, 9, 1),
            remplace_version_id: None,
            remplacee_par: None,
            motif: None,
            piece_document_id: None,
            source: "saisie".into(),
            source_ref: None,
            sensible: false,
            enregistre_le: Utc::now(),
            enregistre_par: None,
        }
    }

    fn demande(nature: NatureVersion, modif: ModifIdentite) -> DemandeVersion {
        DemandeVersion {
            nature,
            modif,
            effet_du: None,
            motif_code: match nature {
                NatureVersion::Correction => Some("faute_de_frappe".into()),
                NatureVersion::ChangementEtatCivil => Some("decision_etat_civil".into()),
                _ => None,
            },
            motif: Some("acte de naissance".into()),
            piece_document_id: Some(Uuid::new_v4()),
            source: "saisie".into(),
            source_ref: None,
            acteur: Some(Uuid::new_v4()),
        }
    }

    fn nom(n: &str) -> ModifIdentite {
        ModifIdentite {
            nom_naissance: Some(n.into()),
            ..Default::default()
        }
    }

    #[test]
    fn correction_remplace_la_version_et_garde_sa_date() {
        let b = base();
        let p = planifier_version(
            &b,
            &[b.effet_du],
            demande(NatureVersion::Correction, nom("Dupont")),
            d(2026, 11, 6),
        )
        .unwrap();
        assert_eq!(p.version.effet_du, b.effet_du);
        assert_eq!(p.version.remplace_version_id, Some(b.id));
        assert_eq!(p.champs, vec![ChampIdentite::NomNaissance]);
        assert_eq!(p.version.nature, NatureVersion::Correction);
    }

    #[test]
    fn changement_ajoute_une_version_datee_sans_remplacer() {
        let b = base();
        let mut dm = demande(NatureVersion::ChangementEtatCivil, nom("Martin"));
        dm.effet_du = Some(d(2026, 1, 15));
        let p = planifier_version(&b, &[b.effet_du], dm, d(2026, 11, 6)).unwrap();
        assert_eq!(p.version.effet_du, d(2026, 1, 15));
        assert_eq!(p.version.remplace_version_id, None);
    }

    #[test]
    fn changement_sur_une_date_deja_occupee_est_un_conflit() {
        let b = base();
        let mut dm = demande(NatureVersion::ChangementEtatCivil, nom("Martin"));
        dm.effet_du = Some(b.effet_du);
        assert!(matches!(
            planifier_version(&b, &[b.effet_du], dm, d(2026, 11, 6)),
            Err(ErreurIdentite::Conflit(_))
        ));
    }

    #[test]
    fn correction_et_changement_exigent_un_motif_le_changement_une_piece() {
        let b = base();
        let mut dm = demande(NatureVersion::Correction, nom("Dupont"));
        dm.motif_code = None;
        assert!(planifier_version(&b, &[], dm, d(2026, 11, 6)).is_err());
        // Le texte libre est facultatif quand le code suffit.
        let mut dm = demande(NatureVersion::Correction, nom("Dupont"));
        dm.motif = None;
        assert!(planifier_version(&b, &[b.effet_du], dm, d(2026, 11, 6)).is_ok());
        let mut dm = demande(NatureVersion::ChangementEtatCivil, nom("Martin"));
        dm.piece_document_id = None;
        assert!(planifier_version(&b, &[], dm, d(2026, 11, 6)).is_err());
        // La correction n'exige pas de pièce (paramétrage, lot E5).
        let mut dm = demande(NatureVersion::Correction, nom("Dupont"));
        dm.piece_document_id = None;
        assert!(planifier_version(&b, &[b.effet_du], dm, d(2026, 11, 6)).is_ok());
    }

    #[test]
    fn usage_ne_touche_pas_l_etat_civil() {
        let b = base();
        let mut dm = demande(NatureVersion::Usage, nom("Autre"));
        dm.motif = None;
        assert!(planifier_version(&b, &[], dm, d(2026, 11, 6)).is_err());
        let m = ModifIdentite {
            prenom_usage: Some(Some("Jo".into())),
            ..Default::default()
        };
        let mut dm = demande(NatureVersion::Usage, m);
        dm.motif = None;
        dm.piece_document_id = None;
        let p = planifier_version(&b, &[b.effet_du], dm, d(2026, 11, 6)).unwrap();
        assert!(p.version.sensible, "un prénom d'usage est sensible");
        assert_eq!(p.champs, vec![ChampIdentite::PrenomUsage]);
    }

    #[test]
    fn usage_le_meme_jour_remplace_la_version_du_jour_et_herite_de_sa_sensibilite() {
        let mut b = base();
        b.sensible = true;
        let m = ModifIdentite {
            nom_usage: Some(Some("Martin-Dupond".into())),
            ..Default::default()
        };
        let mut dm = demande(NatureVersion::Usage, m);
        dm.effet_du = Some(b.effet_du);
        let p = planifier_version(&b, &[b.effet_du], dm, d(2026, 11, 6)).unwrap();
        assert_eq!(p.version.remplace_version_id, Some(b.id));
        assert!(
            p.version.sensible,
            "hérite de la version remplacée (elle porte un prénom d'usage)"
        );
        // Un changement d'état civil le même jour reste un conflit.
        let mut dm = demande(NatureVersion::ChangementEtatCivil, nom("Martin"));
        dm.effet_du = Some(b.effet_du);
        assert!(planifier_version(&b, &[b.effet_du], dm, d(2026, 11, 6)).is_err());
    }

    #[test]
    fn la_sensibilite_est_collante_sur_les_versions_suivantes() {
        let mut b = base();
        b.sensible = true;
        b.prenom_usage = Some("Jo".into());
        let m = ModifIdentite {
            nom_usage: Some(Some("Martin".into())),
            ..Default::default()
        };
        let mut dm = demande(NatureVersion::Usage, m);
        dm.effet_du = Some(d(2026, 11, 7));
        let p = planifier_version(&b, &[b.effet_du], dm, d(2026, 11, 6)).unwrap();
        assert!(p.version.sensible, "recopie le prénom d'usage sensible");
        let mut dm = demande(NatureVersion::ChangementEtatCivil, nom("Autre"));
        dm.effet_du = Some(d(2026, 11, 7));
        assert!(
            planifier_version(&b, &[b.effet_du], dm, d(2026, 11, 6))
                .unwrap()
                .version
                .sensible
        );
    }

    #[test]
    fn motifs_codes_et_sensibilite_forcee() {
        let b = base();
        // Code hors liste, ou d'une autre nature : refusé.
        for mauvais in ["adoption", "inconnu"] {
            let mut dm = demande(NatureVersion::Correction, nom("Dupont"));
            dm.motif_code = Some(mauvais.into());
            assert!(
                planifier_version(&b, &[b.effet_du], dm, d(2026, 11, 6)).is_err(),
                "{mauvais}"
            );
        }
        // « autre » exige un texte.
        let mut dm = demande(NatureVersion::Correction, nom("Dupont"));
        dm.motif_code = Some("autre".into());
        dm.motif = None;
        assert!(planifier_version(&b, &[b.effet_du], dm, d(2026, 11, 6)).is_err());
        // Adoption, changement de prénom, de mention de sexe, protection : sensible, même
        // quand seul le nom change (l'opérateur ne choisit pas).
        for code in [
            "adoption",
            "changement_prenom",
            "changement_mention_sexe",
            "protection",
        ] {
            let mut dm = demande(NatureVersion::ChangementEtatCivil, nom("Martin"));
            dm.motif_code = Some(code.into());
            dm.effet_du = Some(d(2026, 11, 7));
            let p = planifier_version(&b, &[b.effet_du], dm, d(2026, 11, 6)).unwrap();
            assert!(p.version.sensible, "{code}");
            assert_eq!(p.version.motif_code.as_deref(), Some(code));
        }
        // Un changement ordinaire de nom ne l'est pas.
        let mut dm = demande(NatureVersion::ChangementEtatCivil, nom("Martin"));
        dm.effet_du = Some(d(2026, 11, 7));
        assert!(
            !planifier_version(&b, &[b.effet_du], dm, d(2026, 11, 6))
                .unwrap()
                .version
                .sensible
        );
    }

    #[test]
    fn date_de_naissance_incomplete_ou_presumee() {
        let b = base();
        // Mois seul, présumée : ancrée, mais la précision et le statut le disent.
        let m = ModifIdentite {
            date_naissance: Some(Some(d(2010, 3, 1))),
            date_naissance_precision: Some("mois".into()),
            date_naissance_statut: Some("presumee".into()),
            ..Default::default()
        };
        let p = planifier_version(
            &b,
            &[b.effet_du],
            demande(NatureVersion::Correction, m),
            d(2026, 11, 6),
        )
        .unwrap();
        assert_eq!(p.version.date_naissance_precision, "mois");
        assert_eq!(p.version.date_naissance_statut, "presumee");
        assert!(p.champs.contains(&ChampIdentite::DateNaissancePrecision));
        // Inconnue : la date est effacée.
        let m = ModifIdentite {
            date_naissance_precision: Some("inconnue".into()),
            ..Default::default()
        };
        let p = planifier_version(
            &b,
            &[b.effet_du],
            demande(NatureVersion::Correction, m),
            d(2026, 11, 6),
        )
        .unwrap();
        assert_eq!(p.version.date_naissance, None);
        // Inconnue avec une valeur : refusé ; précision ou statut hors liste : refusé.
        let m = ModifIdentite {
            date_naissance: Some(Some(d(2010, 1, 1))),
            date_naissance_precision: Some("inconnue".into()),
            ..Default::default()
        };
        assert!(planifier_version(
            &b,
            &[b.effet_du],
            demande(NatureVersion::Correction, m),
            d(2026, 11, 6)
        )
        .is_err());
        for (pr, st) in [(Some("heure"), None), (None, Some("certaine"))] {
            let m = ModifIdentite {
                date_naissance_precision: pr.map(Into::into),
                date_naissance_statut: st.map(Into::into),
                ..Default::default()
            };
            assert!(planifier_version(
                &b,
                &[b.effet_du],
                demande(NatureVersion::Correction, m),
                d(2026, 11, 6)
            )
            .is_err());
        }
    }

    #[test]
    fn ecriture_d_origine_facultative() {
        let b = base();
        let m = ModifIdentite {
            nom_ecriture_origine: Some(Some("دوبون".into())),
            ..Default::default()
        };
        let p = planifier_version(
            &b,
            &[b.effet_du],
            demande(NatureVersion::Correction, m),
            d(2026, 11, 6),
        )
        .unwrap();
        assert_eq!(p.version.nom_ecriture_origine.as_deref(), Some("دوبون"));
        assert_eq!(p.champs, vec![ChampIdentite::NomEcritureOrigine]);
    }

    #[test]
    fn nom_usage_seul_n_est_pas_sensible_changement_de_sexe_l_est() {
        let b = base();
        let m = ModifIdentite {
            nom_usage: Some(Some("Martin-Dupond".into())),
            ..Default::default()
        };
        let p =
            planifier_version(&b, &[], demande(NatureVersion::Usage, m), d(2026, 11, 6)).unwrap();
        assert!(!p.version.sensible);
        let m = ModifIdentite {
            sexe: Some(Some("F".into())),
            ..Default::default()
        };
        let p = planifier_version(
            &b,
            &[],
            demande(NatureVersion::ChangementEtatCivil, m),
            d(2026, 11, 6),
        )
        .unwrap();
        assert!(p.version.sensible);
    }

    #[test]
    fn correction_d_une_version_sensible_reste_sensible() {
        let mut b = base();
        b.sensible = true;
        let p = planifier_version(
            &b,
            &[b.effet_du],
            demande(NatureVersion::Correction, nom("Dupont")),
            d(2026, 11, 6),
        )
        .unwrap();
        assert!(p.version.sensible);
    }

    #[test]
    fn refuse_modification_vide_et_valeurs_invalides() {
        let b = base();
        // Même valeur : aucune modification.
        assert!(planifier_version(
            &b,
            &[b.effet_du],
            demande(NatureVersion::Correction, nom("Dupond")),
            d(2026, 11, 6)
        )
        .is_err());
        for mauvais in ["", "  ", "Dup0nd", "Du\u{0}pond"] {
            assert!(
                planifier_version(
                    &b,
                    &[b.effet_du],
                    demande(NatureVersion::Correction, nom(mauvais)),
                    d(2026, 11, 6)
                )
                .is_err(),
                "{mauvais:?}"
            );
        }
        let futur = ModifIdentite {
            date_naissance: Some(Some(d(2030, 1, 1))),
            ..Default::default()
        };
        assert!(planifier_version(
            &b,
            &[b.effet_du],
            demande(NatureVersion::Correction, futur),
            d(2026, 11, 6)
        )
        .is_err());
        let sexe = ModifIdentite {
            sexe: Some(Some("X".into())),
            ..Default::default()
        };
        assert!(planifier_version(
            &b,
            &[b.effet_du],
            demande(NatureVersion::Correction, sexe),
            d(2026, 11, 6)
        )
        .is_err());
        let rang = ModifIdentite {
            rang_prenom_usuel: Some(3),
            ..Default::default()
        };
        assert!(planifier_version(
            &b,
            &[b.effet_du],
            demande(NatureVersion::Correction, rang),
            d(2026, 11, 6)
        )
        .is_err());
    }

    #[test]
    fn natures_systeme_refusees_et_version_deja_remplacee() {
        let b = base();
        for n in [
            NatureVersion::Creation,
            NatureVersion::Reprise,
            NatureVersion::Import,
            NatureVersion::Fusion,
        ] {
            assert!(planifier_version(&b, &[], demande(n, nom("Autre")), d(2026, 11, 6)).is_err());
        }
        let mut r = base();
        r.remplacee_par = Some(Uuid::new_v4());
        assert!(matches!(
            planifier_version(
                &r,
                &[],
                demande(NatureVersion::Correction, nom("Dupont")),
                d(2026, 11, 6)
            ),
            Err(ErreurIdentite::Conflit(_))
        ));
    }

    #[test]
    fn historique_masque_les_versions_sensibles_sans_trace() {
        let a = base();
        let mut s = base();
        s.sensible = true;
        s.remplace_version_id = Some(a.id);
        let mut c = base();
        c.remplace_version_id = Some(s.id);
        let mut a2 = a.clone();
        a2.remplacee_par = Some(s.id);
        let vue = filtrer_historique(vec![a2, s.clone(), c.clone()], false);
        assert_eq!(vue.len(), 2);
        assert!(vue.iter().all(|v| !v.sensible));
        assert!(vue.iter().all(|v| v.remplace_version_id != Some(s.id)));
        assert!(vue.iter().all(|v| v.remplacee_par != Some(s.id)));
        assert_eq!(filtrer_historique(vec![s, c], true).len(), 2);
    }

    #[test]
    fn historique_scelle_l_ancienne_identite_d_un_changement_sensible() {
        // RP2-02 : adoption (changement d'état civil sensible) : l'ancienne version, non sensible,
        // n'est lisible qu'avec G6 ; une version non liée au changement reste visible.
        let mut ancienne = base();
        ancienne.nom_naissance = "Chang".into();
        let mut adoption = base();
        adoption.nom_naissance = "Diggory".into();
        adoption.nature = NatureVersion::ChangementEtatCivil;
        adoption.sensible = true;
        adoption.effet_du = d(2026, 10, 15);
        let vue = filtrer_historique(vec![ancienne.clone(), adoption.clone()], false);
        assert!(vue.is_empty(), "ni Chang ni Diggory pour le secrétariat : {vue:?}");
        let tout = filtrer_historique(vec![ancienne, adoption], true);
        assert_eq!(tout.len(), 2);
    }

    #[test]
    fn noms_de_champs_stables() {
        assert_eq!(ChampIdentite::PrenomUsage.nom(), "prenom_usage");
        assert_eq!(
            NatureVersion::parse("changement_etat_civil"),
            Some(NatureVersion::ChangementEtatCivil)
        );
        assert_eq!(NatureVersion::parse("x"), None);
    }
}
