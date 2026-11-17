//! Socle 32 (espaces de travail), lots ET0 et ET1 — types, bornes et fonctions pures
//! (`docs/socles/32_BUREAUTIQUE.md` §7, §8, §10, §24 ; règles R-ET1 à R-ET6, R-ET9 à R-ET13, R-ET20, R-ET21, R-ET25).
//!
//! Ce module ne touche pas la base : il porte le vocabulaire (rôles d'espace, natures, états), les **bornes** (R-ET25),
//! les quotas par défaut (S-3201, S-3202), la normalisation des noms, la proposition « (2) » et le plan d'élagage des
//! versions (S-3210). Les commandes (`espaces_service.rs`), la réservation de quota (`espaces_quota.rs`) et les tâches
//! planifiées (`espaces_taches.rs`) s'appuient dessus. Aucune de ces fonctions ne lit l'horloge : l'instant est un
//! paramètre (règle 33 du pilotage : un test ne dépend jamais de l'heure réelle).

use chrono::{DateTime, Datelike, Duration, NaiveDate, Utc};
use domain_eleve::horloge::date_paris;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use uuid::Uuid;

// ───────────────────────── Bornes (R-ET25) et quotas (S-3201, S-3202) ─────────────────────────

const GIO: i64 = 1024 * 1024 * 1024;
const MIO: i64 = 1024 * 1024;

/// Quota d'un espace personnel : 2 Gio (reprise de M19, NX32-18).
pub const QUOTA_PERSONNEL_OCTETS: i64 = 2 * GIO;
/// Quota d'un espace libre entre personnels. **Hypothèse** (la conception ne le chiffre pas, §28.1 point 7 ne donne que
/// le personnel, l'élève, le cours, l'équipe et le service) : 2 Gio, comme le quota par propriétaire de M19 ; le volume
/// d'un espace collectif est compté à l'espace, pas à la personne qui dépose (R-ET20). Réglable par établissement
/// (`ent_espace_quota_profil`, lot ET9).
pub const QUOTA_LIBRE_OCTETS: i64 = 2 * GIO;
/// Plafond d'un fichier (plafond d'`ent-object`, 64 Mio).
pub const PLAFOND_FICHIER_OCTETS: i64 = 64 * MIO;
/// Longueur maximale du nom d'un espace, d'un dossier (R-ET25).
pub const NOM_MAX: usize = 120;
/// Longueur maximale du nom d'un fichier : la limite du noyau (`upload.rs`, 200 octets) ; R-ET25 borne les noms
/// d'espaces et de dossiers, les noms de fichiers repris de M19 pouvant être plus longs que 120 caractères.
pub const NOM_FICHIER_MAX: usize = 200;
/// Profondeur maximale de l'arborescence d'un espace (R-ET25) : la racine de l'espace n'est pas comptée.
pub const PROFONDEUR_MAX: usize = 8;
/// Éléments vivants par dossier (R-ET25).
pub const ELEMENTS_PAR_DOSSIER_MAX: i64 = 2000;
/// Membres ajoutés actifs par espace (R-ET25).
pub const MEMBRES_AJOUTES_MAX: i64 = 200;
/// Espaces libres qu'une personne peut créer et garder : borne d'usage (sans elle, chaque espace libre ayant son propre
/// quota, une personne pourrait épuiser le stockage). **Hypothèse** : 20.
pub const ESPACES_LIBRES_PAR_CREATEUR_MAX: i64 = 20;
/// Destinataires d'un même geste de partage.
pub const PARTAGES_PAR_GESTE_MAX: usize = 50;
/// Durée d'une réservation de quota (CET-18).
pub const RESERVATION_MINUTES: i64 = 15;
/// Corbeille : 30 jours (S-3205).
pub const CORBEILLE_JOURS: i64 = 30;
/// Versions toutes conservées pendant 30 jours, puis une par jour jusqu'à 90 jours, puis une par semaine (S-3210).
pub const ELAGAGE_TOUT_JOURS: i64 = 30;
pub const ELAGAGE_JOUR_JOURS: i64 = 90;
/// Analyses tentées au plus par version avant alerte d'exploitation (l'objet reste invisible : fermé par défaut).
pub const ANALYSE_TENTATIVES_ALERTE: i32 = 5;

/// Quota par défaut d'un type d'espace (profil du tableau `ent_espace_quota_profil`).
pub fn quota_defaut(type_espace: &str) -> i64 {
    match type_espace {
        "personnel" => QUOTA_PERSONNEL_OCTETS,
        "equipe" | "service" => 20 * GIO,
        "cours" | "groupe" | "classe" | "delegues" | "instance" | "projet" => 10 * GIO,
        _ => QUOTA_LIBRE_OCTETS,
    }
}

/// Profil de quota (`ent_espace_quota_profil.profil`) d'un type d'espace.
pub fn profil_quota(type_espace: &str) -> &'static str {
    match type_espace {
        "personnel" => "personnel",
        "equipe" => "espace_equipe",
        "service" => "espace_service",
        "cours" | "groupe" | "classe" | "delegues" | "instance" | "projet" => "espace_cours",
        _ => "espace_libre",
    }
}

/// Un espace « avec élèves » : téléchargements journalisés (ETR-118), archives ZIP refusées (R-ET22). Aucun des deux
/// types ouverts en ET1 n'en est un (R-ET12 : un élève n'entre pas dans un espace de personnels).
pub fn espace_avec_eleves(type_espace: &str) -> bool {
    matches!(type_espace, "cours" | "groupe" | "classe" | "delegues" | "instance" | "projet")
}

// ───────────────────────── Rôles et états ─────────────────────────

/// Rôle d'un membre dans un espace (R-ET5 : fermés ; une règle de dossier restreint, jamais n'élargit).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RoleEspace {
    Lecteur,
    Contributeur,
    Animateur,
}

impl RoleEspace {
    pub fn code(self) -> &'static str {
        match self {
            Self::Lecteur => "lecteur",
            Self::Contributeur => "contributeur",
            Self::Animateur => "animateur",
        }
    }

    pub fn depuis_code(c: &str) -> Option<Self> {
        match c {
            "lecteur" => Some(Self::Lecteur),
            "contributeur" => Some(Self::Contributeur),
            "animateur" => Some(Self::Animateur),
            _ => None,
        }
    }

    /// Déposer, créer des dossiers, nouvelle version.
    pub fn peut_contribuer(self) -> bool {
        self >= Self::Contributeur
    }

    pub fn est_animateur(self) -> bool {
        self == Self::Animateur
    }
}

/// Droit d'un partage nominatif.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DroitPartage {
    Lecture,
    Ecriture,
}

impl DroitPartage {
    pub fn code(self) -> &'static str {
        match self {
            Self::Lecture => "lecture",
            Self::Ecriture => "ecriture",
        }
    }

    pub fn depuis_code(c: &str) -> Option<Self> {
        match c {
            "lecture" => Some(Self::Lecture),
            "ecriture" => Some(Self::Ecriture),
            _ => None,
        }
    }
}

/// Catalogue fermé des couleurs et des icônes (§ ETR-107 : pas de téléversement, pas de texte libre).
pub const COULEURS: &[&str] = &["bleu", "vert", "rouge", "orange", "violet", "gris", "rose", "turquoise"];
pub const ICONES: &[&str] = &["dossier", "livre", "crayon", "equipe", "calendrier", "etoile", "ampoule", "projet"];

// ───────────────────────── Vues (DTO de lecture) ─────────────────────────

/// Un espace dans la liste « Mes espaces » (ETR-110, ETR-111).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EspaceResume {
    pub id: Uuid,
    pub type_espace: String,
    pub libelle: String,
    pub description: Option<String>,
    pub couleur_code: String,
    pub icone_code: String,
    /// Mon rôle dans l'espace (`animateur` pour le propriétaire d'un espace personnel).
    pub role: RoleEspace,
    pub nb_membres: i64,
    pub quota_octets: i64,
    pub octets_utilises: i64,
    pub octets_reserves: i64,
    pub derniere_activite_le: DateTime<Utc>,
    pub invitation_non_vue: bool,
}

/// Création d'un espace libre entre personnels (ETR-106, ETR-150).
#[derive(Debug, Clone, Deserialize)]
pub struct NouvelEspace {
    pub libelle: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub couleur_code: Option<String>,
    #[serde(default)]
    pub icone_code: Option<String>,
    /// Collègues invités en un geste, choisis dans l'annuaire.
    #[serde(default)]
    pub membres: Vec<NouveauMembre>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ModifEspace {
    #[serde(default)]
    pub libelle: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub couleur_code: Option<String>,
    #[serde(default)]
    pub icone_code: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NouveauMembre {
    pub personne_id: Uuid,
    pub role: RoleEspace,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct MembreVue {
    pub personne_id: Uuid,
    pub nom_affiche: String,
    pub role: RoleEspace,
    pub depuis: NaiveDate,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct EspaceDetail {
    pub espace: EspaceResume,
    pub membres: Vec<MembreVue>,
}

/// Un collègue de l'annuaire (ETR-151) : l'identifiant servi est celui de la personne, jamais un compte.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Collegue {
    pub personne_id: Uuid,
    pub nom_affiche: String,
    pub fonction: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct DossierVue {
    pub id: Uuid,
    pub nom: String,
    pub parent_id: Option<Uuid>,
    pub nb_elements: i64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ElementVue {
    pub id: Uuid,
    pub nom: String,
    pub dossier_id: Option<Uuid>,
    pub nature: String,
    pub etat: String,
    pub auteur_nom: String,
    pub auteur_moi: bool,
    pub taille: Option<i64>,
    pub type_reel: Option<String>,
    pub version_numero: Option<i32>,
    pub modifie_le: DateTime<Utc>,
}

/// Contenu d'un dossier (ou de la racine d'un espace), avec le chemin pour le fil d'Ariane.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Contenu {
    pub espace_id: Uuid,
    pub dossier_id: Option<Uuid>,
    pub chemin: Vec<DossierVue>,
    pub dossiers: Vec<DossierVue>,
    pub elements: Vec<ElementVue>,
    /// Mon rôle dans l'espace, ou `None` quand je ne vois qu'un partage nominatif.
    pub role: Option<RoleEspace>,
    pub peut_ecrire: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct VersionVue {
    pub id: Uuid,
    pub numero: i32,
    pub auteur_nom: String,
    pub cree_le: DateTime<Utc>,
    pub commentaire: Option<String>,
    pub taille: i64,
    pub type_reel: String,
    pub origine: String,
    pub etat: String,
    pub courante: bool,
    pub restaure_de_numero: Option<i32>,
}

// ───────────────────────── Dépôt (envoi direct, taille signée) ─────────────────────────

/// Que faire quand le nom existe déjà dans le dossier (§10 point 2 : jamais d'écrasement).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChoixSiExiste {
    /// Nouvelle version de l'élément existant.
    NouvelleVersion,
    /// Nouvel élément, nommé « X (2) ».
    NouveauFichier,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DemandeDepot {
    #[serde(default)]
    pub dossier_id: Option<Uuid>,
    pub nom: String,
    /// Taille annoncée en octets : **signée** dans l'URL d'envoi et revérifiée après l'envoi (R-ET21).
    pub taille: i64,
    #[serde(default)]
    pub mime: Option<String>,
    #[serde(default)]
    pub commentaire: Option<String>,
    #[serde(default)]
    pub si_existe: Option<ChoixSiExiste>,
}

/// Envoi autorisé : l'URL ne vaut que pour cette taille, 15 minutes, et un quota est réservé.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct DepotAutorise {
    pub reservation_id: Uuid,
    pub url: String,
    pub methode: String,
    pub en_tetes: Vec<(String, String)>,
    pub expire_le: DateTime<Utc>,
    /// Nom retenu (« X (2) » si le choix était « nouveau fichier »).
    pub nom: String,
}

/// Réponse à une demande de dépôt : autorisé, ou nom déjà pris (le client doit choisir, jamais d'écrasement).
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(tag = "resultat", rename_all = "snake_case")]
pub enum ReponseDepot {
    Autorise(DepotAutorise),
    NomDejaPris { element_id: Uuid, nom_propose: String },
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ResultatDepot {
    pub element_id: Uuid,
    pub version_id: Uuid,
    pub numero: i32,
    /// `analyse` : le fichier n'est visible que de son auteur jusqu'au verdict de l'antivirus.
    pub etat: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct LienTelechargement {
    pub url: String,
    pub expire_le: DateTime<Utc>,
    pub nom: String,
}

/// Aperçu avant mise à la corbeille d'un dossier (ETR-123) : rien n'est fait tant que l'animateur n'a pas confirmé.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ApercuCorbeilleDossier {
    pub nb_dossiers: i64,
    pub nb_elements: i64,
    /// Noms des éléments déposés par d'autres personnes que l'acteur.
    pub elements_d_autrui: Vec<String>,
    pub fait: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct LigneCorbeille {
    pub genre: String,
    pub id: Uuid,
    pub nom: String,
    pub supprime_le: DateTime<Utc>,
    pub auteur_nom: String,
    pub purge_le: DateTime<Utc>,
    /// Une version figée par un autre socle empêche l'effacement (R-ET6).
    pub figee: bool,
}

// ───────────────────────── Partages ─────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct NouveauPartage {
    pub destinataire_personne_id: Uuid,
    pub droit: DroitPartage,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DemandePartage {
    #[serde(default)]
    pub element_id: Option<Uuid>,
    #[serde(default)]
    pub dossier_id: Option<Uuid>,
    pub destinataires: Vec<NouveauPartage>,
    /// Date de fin (incluse), facultative ; jamais antérieure au jour du partage.
    #[serde(default)]
    pub fin: Option<NaiveDate>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct PartageVue {
    pub id: Uuid,
    pub element_id: Option<Uuid>,
    pub dossier_id: Option<Uuid>,
    pub destinataire_personne_id: Uuid,
    pub destinataire_nom: String,
    pub droit: DroitPartage,
    pub debut: NaiveDate,
    pub fin: Option<NaiveDate>,
}

/// Une ligne de « Partagés avec moi » (ETR-131).
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct PartageRecu {
    pub partage_id: Uuid,
    pub espace_id: Uuid,
    pub element_id: Option<Uuid>,
    pub dossier_id: Option<Uuid>,
    pub nom: String,
    pub de_nom: String,
    pub droit: DroitPartage,
    pub debut: NaiveDate,
    pub fin: Option<NaiveDate>,
    pub vu: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct InvitationsNonVues {
    pub nombre: i64,
}

// ───────────────────────── Quota et journal ─────────────────────────

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct QuotaEspace {
    pub espace_id: Uuid,
    pub libelle: String,
    pub type_espace: String,
    pub quota_octets: i64,
    pub octets_utilises: i64,
    pub octets_reserves: i64,
    /// Répartition par famille de format (versions et corbeille comprises).
    pub par_type: Vec<(String, i64)>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct LigneJournal {
    pub le: DateTime<Utc>,
    pub acteur_nom: Option<String>,
    pub action_code: String,
    pub detail_code: Option<String>,
    pub element_id: Option<Uuid>,
}

// ───────────────────────── Fonctions pures ─────────────────────────

/// Caractères invisibles ou de contrôle de direction qui permettraient de maquiller un nom (CET-22, CET-70).
fn est_invisible(c: char) -> bool {
    c.is_control()
        || matches!(c, '\u{200B}'..='\u{200F}' | '\u{202A}'..='\u{202E}' | '\u{2060}'..='\u{2064}' | '\u{2066}'..='\u{2069}' | '\u{FEFF}')
}

/// Nom propre d'un espace, d'un dossier ou d'un fichier : espaces de bord retirés, espaces multiples réduits, aucun
/// séparateur de chemin (`/`, `\`), aucun caractère de contrôle ni invisible, ni `.` ni `..`. `max` en caractères.
pub fn nom_propre(brut: &str, max: usize) -> Result<String, &'static str> {
    let propre = brut.split_whitespace().collect::<Vec<_>>().join(" ");
    if propre.is_empty() {
        return Err("Le nom ne peut pas être vide.");
    }
    if propre.chars().count() > max {
        return Err("Le nom est trop long.");
    }
    if propre.chars().any(|c| est_invisible(c) || c == '/' || c == '\\') {
        return Err("Le nom contient un caractère interdit.");
    }
    if propre == "." || propre == ".." {
        return Err("Ce nom est réservé.");
    }
    Ok(propre)
}

/// Forme comparable d'un nom (unicité parmi les frères) : minuscules, espaces réduits.
pub fn nom_normalise(propre: &str) -> String {
    propre.split_whitespace().collect::<Vec<_>>().join(" ").to_lowercase()
}

/// Proposition « X (2) » : premier numéro libre à partir de 2, l'extension (4 caractères alphanumériques au plus)
/// restant à la fin (`rapport.pdf` → `rapport (2).pdf`). `pris` : noms normalisés déjà présents.
pub fn nom_propose(propre: &str, pris: &BTreeSet<String>) -> String {
    let (base, ext) = match propre.rsplit_once('.') {
        Some((b, e)) if !b.is_empty() && !e.is_empty() && e.chars().count() <= 5 && e.chars().all(|c| c.is_ascii_alphanumeric()) => {
            (b.to_string(), format!(".{e}"))
        }
        _ => (propre.to_string(), String::new()),
    };
    // On borne la recherche : un dossier contient au plus ELEMENTS_PAR_DOSSIER_MAX éléments.
    for n in 2..=(ELEMENTS_PAR_DOSSIER_MAX as usize + 2) {
        let candidat = format!("{base} ({n}){ext}");
        if !pris.contains(&nom_normalise(&candidat)) {
            return candidat;
        }
    }
    format!("{base} ({}){ext}", Uuid::new_v4().simple())
}

/// Famille de format pour la répartition du quota (ETR-260).
pub fn famille_de_type(mime: &str) -> &'static str {
    match mime {
        "application/pdf" | "text/plain" => "document",
        m if m.starts_with("application/vnd.") => "document",
        m if m.starts_with("image/") => "image",
        m if m.starts_with("audio/") || m.starts_with("video/") => "audio_video",
        "application/zip" => "archive",
        _ => "autre",
    }
}

/// Une version candidate à l'élagage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VersionElagable {
    pub id: Uuid,
    pub numero: i32,
    pub cree_le: DateTime<Utc>,
    /// Version courante de l'élément : toujours conservée.
    pub courante: bool,
    /// Figée par un autre socle (R-ET6) ou nommée : jamais élaguée.
    pub protegee: bool,
}

/// Plan d'élagage (S-3210) : toutes les versions pendant 30 jours ; ensuite la **dernière de chaque jour** (jour de
/// Paris) jusqu'à 90 jours ; au-delà, la **dernière de chaque semaine**. La courante et les versions protégées sont
/// toujours gardées. Renvoie les identifiants à élaguer, du plus ancien au plus récent.
pub fn versions_a_elaguer(versions: &[VersionElagable], maintenant: DateTime<Utc>) -> Vec<Uuid> {
    let mut triees: Vec<&VersionElagable> = versions.iter().collect();
    // Du plus récent au plus ancien : la première rencontrée d'un jour (ou d'une semaine) est celle qu'on garde.
    triees.sort_by(|a, b| b.cree_le.cmp(&a.cree_le).then(b.numero.cmp(&a.numero)));
    let mut jours_vus: BTreeSet<NaiveDate> = BTreeSet::new();
    let mut semaines_vues: BTreeSet<(i32, u32)> = BTreeSet::new();
    let mut a_elaguer: Vec<(i32, Uuid)> = Vec::new();
    for v in triees {
        let age = maintenant - v.cree_le;
        let jour = date_paris(v.cree_le);
        let garde_par_regle = if age < Duration::days(ELAGAGE_TOUT_JOURS) {
            true
        } else if age < Duration::days(ELAGAGE_JOUR_JOURS) {
            jours_vus.insert(jour)
        } else {
            let iso = jour.iso_week();
            semaines_vues.insert((iso.year(), iso.week()))
        };
        // Une version jeune d'un jour déjà « vu » plus récemment reste gardée par sa fenêtre ; en revanche elle ne
        // « consomme » pas le créneau d'une version plus ancienne (chaque fenêtre a ses propres ensembles).
        if v.courante || v.protegee || garde_par_regle {
            continue;
        }
        a_elaguer.push((v.numero, v.id));
    }
    a_elaguer.sort();
    a_elaguer.into_iter().map(|(_, id)| id).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn t(j: u32, h: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2027, 1, j, h, 0, 0).unwrap()
    }

    fn v(n: i32, cree: DateTime<Utc>) -> VersionElagable {
        VersionElagable { id: Uuid::from_u128(n as u128), numero: n, cree_le: cree, courante: false, protegee: false }
    }

    #[test]
    fn noms_propres_et_interdits() {
        assert_eq!(nom_propre("  Cours   de  maths ", NOM_MAX).unwrap(), "Cours de maths");
        assert!(nom_propre("", NOM_MAX).is_err());
        assert!(nom_propre("   ", NOM_MAX).is_err());
        assert!(nom_propre("..", NOM_MAX).is_err());
        assert!(nom_propre("a/b", NOM_MAX).is_err());
        assert!(nom_propre("a\\b", NOM_MAX).is_err());
        assert!(nom_propre("a\u{202E}fdp.exe", NOM_MAX).is_err(), "marque de direction");
        assert!(nom_propre("a\u{200B}b", NOM_MAX).is_err(), "espace de largeur nulle");
        assert!(nom_propre("a\0b", NOM_MAX).is_err());
        assert!(nom_propre(&"é".repeat(121), NOM_MAX).is_err());
        assert!(nom_propre(&"é".repeat(120), NOM_MAX).is_ok());
        // Un nom n'est jamais interprété (CET-70) : il est échappé à l'affichage. Le balisage sans `/` est gardé tel
        // quel ; la balise fermante est refusée par le séparateur de chemin.
        assert_eq!(nom_propre("<b>gras</b>", NOM_MAX).unwrap_err(), "Le nom contient un caractère interdit.");
        assert_eq!(nom_propre("<b>gras<b>", NOM_MAX).unwrap(), "<b>gras<b>");
    }

    #[test]
    fn proposition_deux_garde_l_extension_et_saute_les_numeros_pris() {
        let mut pris = BTreeSet::new();
        assert_eq!(nom_propose("rapport.pdf", &pris), "rapport (2).pdf");
        pris.insert(nom_normalise("rapport (2).pdf"));
        pris.insert(nom_normalise("Rapport (3).PDF"));
        assert_eq!(nom_propose("rapport.pdf", &pris), "rapport (4).pdf");
        assert_eq!(nom_propose("notes", &BTreeSet::new()), "notes (2)");
        assert_eq!(nom_propose("archive.tar.gz", &BTreeSet::new()), "archive.tar (2).gz");
        assert_eq!(nom_propose(".cache", &BTreeSet::new()), ".cache (2)");
    }

    #[test]
    fn normalisation_insensible_a_la_casse_et_aux_espaces() {
        assert_eq!(nom_normalise("Rapport  Final.PDF"), nom_normalise("rapport final.pdf"));
    }

    #[test]
    fn quotas_par_defaut_et_profils() {
        assert_eq!(quota_defaut("personnel"), 2 * GIO);
        assert_eq!(quota_defaut("libre_personnels"), 2 * GIO);
        assert_eq!(quota_defaut("equipe"), 20 * GIO);
        assert_eq!(quota_defaut("cours"), 10 * GIO);
        assert_eq!(profil_quota("libre_personnels"), "espace_libre");
        assert_eq!(profil_quota("service"), "espace_service");
        assert!(!espace_avec_eleves("personnel"));
        assert!(!espace_avec_eleves("libre_personnels"));
        assert!(espace_avec_eleves("cours"));
    }

    #[test]
    fn roles_ordonnes() {
        assert!(RoleEspace::Animateur > RoleEspace::Contributeur);
        assert!(RoleEspace::Contributeur > RoleEspace::Lecteur);
        assert!(!RoleEspace::Lecteur.peut_contribuer());
        assert!(RoleEspace::Contributeur.peut_contribuer());
        assert_eq!(RoleEspace::depuis_code("animateur"), Some(RoleEspace::Animateur));
        assert_eq!(RoleEspace::depuis_code("admin"), None);
    }

    #[test]
    fn elagage_garde_tout_sous_trente_jours() {
        let maintenant = t(31, 12);
        let vs: Vec<_> = (1..=8).map(|n| v(n, t(20, n as u32))).collect();
        assert!(versions_a_elaguer(&vs, maintenant).is_empty());
    }

    #[test]
    fn elagage_une_par_jour_puis_une_par_semaine_courante_et_protegees_gardees() {
        // « Maintenant » : 2027-06-30 midi. Fenêtre « jour » : du 31 mars (91 j) au 31 mai (30 j) ; « semaine » : avant.
        let maintenant = Utc.with_ymd_and_hms(2027, 6, 30, 12, 0, 0).unwrap();
        let at = |m: u32, j: u32, h: u32| Utc.with_ymd_and_hms(2027, m, j, h, 0, 0).unwrap();
        // Quatre versions le même jour dans la fenêtre « jour » (10 mai) : on garde la dernière (n°4).
        let mut vs = vec![v(1, at(5, 10, 8)), v(2, at(5, 10, 9)), v(3, at(5, 10, 10)), v(4, at(5, 10, 15))];
        // Un autre jour de la fenêtre : une seule version, gardée.
        vs.push(v(5, at(5, 12, 9)));
        // Trois versions de la même semaine ISO, plus de 90 jours (lundi 1er, mercredi 3, vendredi 5 mars) : on garde
        // la dernière (n°8) ; la 6 est nommée (protégée) et la 7 est élaguée.
        let mut v6 = v(6, at(3, 1, 9));
        v6.protegee = true;
        vs.extend([v6, v(7, at(3, 3, 9)), v(8, at(3, 5, 9))]);
        // Version courante ancienne de la même semaine : toujours gardée.
        let mut v9 = v(9, at(3, 2, 9));
        v9.courante = true;
        vs.push(v9);
        let plan = versions_a_elaguer(&vs, maintenant);
        let numeros: Vec<i32> = plan.iter().map(|id| id.as_u128() as i32).collect();
        assert_eq!(numeros, vec![1, 2, 3, 7]);
    }

    #[test]
    fn elagage_est_deterministe_et_independant_de_l_ordre_d_entree() {
        let maintenant = Utc.with_ymd_and_hms(2027, 6, 30, 12, 0, 0).unwrap();
        let at = |m: u32, j: u32, h: u32| Utc.with_ymd_and_hms(2027, m, j, h, 0, 0).unwrap();
        let vs = vec![v(1, at(5, 10, 8)), v(2, at(5, 10, 9)), v(3, at(5, 11, 9)), v(4, at(2, 1, 9)), v(5, at(2, 2, 9))];
        let mut inverse = vs.clone();
        inverse.reverse();
        assert_eq!(versions_a_elaguer(&vs, maintenant), versions_a_elaguer(&inverse, maintenant));
    }

    #[test]
    fn familles_de_format() {
        assert_eq!(famille_de_type("application/pdf"), "document");
        assert_eq!(famille_de_type("application/vnd.oasis.opendocument.text"), "document");
        assert_eq!(famille_de_type("image/png"), "image");
        assert_eq!(famille_de_type("video/mp4"), "audio_video");
        assert_eq!(famille_de_type("application/zip"), "archive");
    }
}
