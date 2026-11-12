//! Socle 29, lot P2 : l'**index des tâches** et le **kit de branchement** (D-026) de l'à-faire
//! unifié (`docs/socles/29_PORTAILS.md` §8, §22.1).
//!
//! * L'index (`ent_afaire*`) est une **projection jetable** : ni motif, ni montant, ni donnée de
//!   santé (R-PO5) ; il se reconstruit par le rapprochement et se neutralise à la restauration.
//! * Chaque nature de tâche est **déclarée dans le code** ([`NATURES`]) par la brique propriétaire ;
//!   [`verifier_declarations`] (POR-243, R-PO23) refuse une déclaration incomplète : sans gabarit
//!   neutre, sans destinataire, sans file de repli, sans action de clôture, bloquante sans
//!   escalade, ou dont le gabarit contient une variable sensible.
//! * Les ports d'écriture ([`ouvrir_tache`], [`fermer_tache`]) prennent une `PgConnection` : ils
//!   s'appellent **dans la transaction du fait** (consommateur de l'outbox, ou transaction du
//!   propriétaire). Ils sont idempotents sur `(nature, objet, destinataire)` et ordonnés par
//!   `version_source` (un fait plus ancien n'écrase jamais un plus récent, CPO-51).
//! * Le 29 n'exécute aucune action métier (R-PO1) : il ouvre, ferme et montre.
//! * L'heure vient de l'appelant (`maintenant`) ; les heures murales se calculent dans le fuseau de
//!   l'établissement (R-PO21), jamais en UTC.

use crate::service::EntDbError;
use chrono::{DateTime, Datelike, Duration, NaiveDateTime, Timelike, Utc, Weekday};
use ent_types::afaire::{EtatAfaire, PrioriteAfaire};
use sqlx::{PgConnection, Row};
use std::collections::HashSet;
use uuid::Uuid;

fn db(e: sqlx::Error) -> EntDbError {
    EntDbError::Sqlx(e)
}

// ---------------------------------------------------------------------------------------------
// Déclaration d'une nature (kit, §8.2)
// ---------------------------------------------------------------------------------------------

/// Variable typée d'un gabarit de titre : aucune variable libre (R-PO5).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Var {
    Prenom,
    Date,
    Heure,
    Classe,
    Matiere,
    Nombre,
    /// Sensible (C2) : refusée dans un gabarit, sauf nature réservée à des profils habilités.
    Montant,
    /// Sensible (C2) : un motif ne figure jamais dans l'index.
    Motif,
}

impl Var {
    pub fn code(self) -> &'static str {
        match self {
            Self::Prenom => "prenom",
            Self::Date => "date",
            Self::Heure => "heure",
            Self::Classe => "classe",
            Self::Matiere => "matiere",
            Self::Nombre => "nombre",
            Self::Montant => "montant",
            Self::Motif => "motif",
        }
    }
    pub fn depuis_code(c: &str) -> Option<Self> {
        Some(match c {
            "prenom" => Self::Prenom,
            "date" => Self::Date,
            "heure" => Self::Heure,
            "classe" => Self::Classe,
            "matiere" => Self::Matiere,
            "nombre" => Self::Nombre,
            "montant" => Self::Montant,
            "motif" => Self::Motif,
            _ => return None,
        })
    }
    /// Sensibilité C0 à C3 (IA_COEUR §7.2).
    pub fn sensibilite(self) -> u8 {
        match self {
            Self::Prenom => 1,
            Self::Montant | Self::Motif => 2,
            _ => 0,
        }
    }
}

/// Qui peut être cardinalité d'une tâche (§8.3, §8.8).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cardinalite {
    /// Une tâche propre à chaque responsable (jamais partagée).
    ParResponsable,
    /// Une tâche par foyer : le premier du foyer qui agit ferme pour le foyer.
    ParFoyer,
    /// Une réponse par élève : le premier qui agit ferme pour tous.
    ParEleve,
    /// Un seul titulaire suffit : indexée chez chacun, close pour tous sans dire qui (A11).
    ParAutorite,
    /// Tâche de file (collective).
    Collective,
}

impl Cardinalite {
    /// Une ligne `ent_afaire` par destinataire (sinon une ligne partagée).
    pub fn une_tache_par_destinataire(self) -> bool {
        matches!(self, Self::ParResponsable)
    }
}

/// Action de clôture : carte d'action (commande du propriétaire, aperçu, confirmation) ou lien profond.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActionDeclaree {
    Carte {
        libelle: &'static str,
    },
    Lien {
        libelle: &'static str,
        lien: &'static str,
    },
}

/// Escalade d'une tâche bloquante (§8.5) : au bout du délai, la file indiquée reçoit la tâche
/// (ajout d'un destinataire, jamais de retrait).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EscaladeDeclaree {
    pub delai_heures: i64,
    pub file: &'static str,
}

/// Une nature de tâche, déclarée par sa brique.
#[derive(Clone, Copy, Debug)]
pub struct NatureDeclaree {
    /// Code stable `brique.nature` (jamais renommé).
    pub code: &'static str,
    /// Socle propriétaire.
    pub socle: i16,
    pub objet_type: &'static str,
    /// Gabarit de titre neutre : `{variable}` typées uniquement.
    pub gabarit: &'static str,
    pub variables: &'static [Var],
    /// Règle de destinataires, en clair (la résolution est faite par le propriétaire).
    pub destinataires: &'static str,
    pub cardinalite: Cardinalite,
    /// Échéance par défaut en jours après le fait (indicative sauf `echeance_dure`).
    pub echeance_jours: Option<i64>,
    pub echeance_dure: bool,
    pub priorite: PrioriteAfaire,
    /// Montée en `due` quand l'échéance est à moins de N heures.
    pub montee_due_heures: Option<i64>,
    pub action: Option<ActionDeclaree>,
    /// Sensibilité de la tâche (C0 à C3).
    pub sensibilite: u8,
    /// Nature réservée à des profils habilités (C2 et plus admis).
    pub reservee: bool,
    /// File de repli : reçoit la tâche si aucun destinataire n'est actif (A13, R-PO9).
    pub file_repli: &'static str,
    pub escalade: Option<EscaladeDeclaree>,
    /// Audience famille ou élève : la plage calme s'applique (R-PO31).
    pub public_famille: bool,
    /// Urgence déclarée : exemptée de la plage calme.
    pub urgente: bool,
    /// Le propriétaire sait lister ses tâches ouvertes (`taches_ouvertes`) : rapprochement possible.
    pub derivee: bool,
    /// Conservation de l'index après clôture (§15.5).
    pub conservation_jours: i64,
    pub ouverte_ia: bool,
}

/// Valeurs par défaut pour écrire une déclaration lisible.
const BASE: NatureDeclaree = NatureDeclaree {
    code: "",
    socle: 0,
    objet_type: "",
    gabarit: "",
    variables: &[],
    destinataires: "",
    cardinalite: Cardinalite::ParResponsable,
    echeance_jours: None,
    echeance_dure: false,
    priorite: PrioriteAfaire::AFaire,
    montee_due_heures: None,
    action: None,
    sensibilite: 1,
    reservee: false,
    file_repli: "",
    escalade: None,
    public_famille: false,
    urgente: false,
    derivee: true,
    conservation_jours: 30,
    ouverte_ia: false,
};

/// Files déclarées (code → propriétaire, droit d'accès). Les membres sont **calculés**.
pub const FILE_JUSTIFICATIFS: &str = "vs.justificatifs";
pub const FILE_PROTECTIONS: &str = "dir.protections";
pub const FILE_DECISIONS: &str = "dir.decisions";
/// Escalade d'une décision restée sans réponse : file d'administration (droit `afaire.rapprocher`).
pub const FILE_ADMIN: &str = "admin.decisions";

/// Registre statique des natures (aucune table de définition, §22).
pub const NATURES: &[NatureDeclaree] = &[
    NatureDeclaree {
        code: "vs.justifier_absence",
        socle: 12,
        objet_type: "absence",
        gabarit: "Absence à justifier — {prenom}, {date}",
        variables: &[Var::Prenom, Var::Date],
        destinataires: "responsables du lien (autorité parentale, droit de justifier), ou l'élève majeur",
        cardinalite: Cardinalite::ParEleve,
        echeance_jours: Some(3),
        priorite: PrioriteAfaire::AFaire,
        montee_due_heures: Some(24),
        action: Some(ActionDeclaree::Carte { libelle: "Justifier l'absence" }),
        file_repli: FILE_JUSTIFICATIFS,
        public_famille: true,
        ..BASE
    },
    NatureDeclaree {
        code: "vs.signer_carnet",
        socle: 13,
        objet_type: "carnet_entree",
        gabarit: "Mot du carnet à signer — {prenom}, {date}",
        variables: &[Var::Prenom, Var::Date],
        destinataires: "titulaires de l'autorité parentale non visés par une interdiction de communication, ou l'élève majeur",
        cardinalite: Cardinalite::ParAutorite,
        echeance_jours: Some(7),
        priorite: PrioriteAfaire::AFaire,
        montee_due_heures: Some(24),
        action: Some(ActionDeclaree::Carte { libelle: "Signer le mot" }),
        file_repli: FILE_JUSTIFICATIFS,
        public_famille: true,
        ..BASE
    },
    NatureDeclaree {
        code: "doc.signer",
        socle: 7,
        objet_type: "doc_a_signer",
        gabarit: "Document à signer — {prenom}",
        variables: &[Var::Prenom],
        destinataires: "titulaires de l'autorité parentale, ou l'élève majeur",
        cardinalite: Cardinalite::ParAutorite,
        echeance_jours: None,
        priorite: PrioriteAfaire::AFaire,
        montee_due_heures: Some(48),
        action: Some(ActionDeclaree::Carte { libelle: "Signer le document" }),
        file_repli: FILE_JUSTIFICATIFS,
        public_famille: true,
        ..BASE
    },
    NatureDeclaree {
        code: "vs.trancher_justificatif",
        socle: 12,
        objet_type: "absence",
        gabarit: "Justificatif à trancher — {prenom}, {date}",
        variables: &[Var::Prenom, Var::Date],
        destinataires: "file de la vie scolaire (porteurs du droit de trancher)",
        cardinalite: Cardinalite::Collective,
        echeance_jours: Some(2),
        priorite: PrioriteAfaire::AFaire,
        montee_due_heures: Some(24),
        action: Some(ActionDeclaree::Lien { libelle: "Examiner", lien: "/web/absences" }),
        file_repli: FILE_JUSTIFICATIFS,
        ..BASE
    },
    NatureDeclaree {
        code: "vs.appel_a_faire",
        socle: 12,
        objet_type: "seance",
        gabarit: "Appel à faire — {classe}, {heure}",
        variables: &[Var::Classe, Var::Heure],
        destinataires: "le professeur de la séance",
        cardinalite: Cardinalite::ParResponsable,
        echeance_jours: None,
        priorite: PrioriteAfaire::Due,
        action: Some(ActionDeclaree::Lien { libelle: "Faire l'appel", lien: "/web/appel" }),
        file_repli: FILE_JUSTIFICATIFS,
        urgente: true,
        ..BASE
    },
    NatureDeclaree {
        code: "dos.protection_a_confirmer",
        socle: 4,
        objet_type: "protection",
        gabarit: "Protection à confirmer",
        variables: &[],
        destinataires: "direction (porteurs du droit de suspendre un lien)",
        cardinalite: Cardinalite::Collective,
        echeance_jours: None,
        priorite: PrioriteAfaire::Due,
        action: Some(ActionDeclaree::Lien {
            libelle: "Examiner",
            lien: "/web/direction/protections",
        }),
        // Une adresse protégée ou un « ne pas solliciter » ne s'écrit jamais en clair (R-PO5) : le titre
        // ne dit rien de la personne ; la nature n'est montrée qu'aux porteurs du droit.
        sensibilite: 3,
        reservee: true,
        file_repli: FILE_PROTECTIONS,
        ..BASE
    },
    NatureDeclaree {
        code: "afaire.decision_en_attente",
        socle: 29,
        objet_type: "afaire",
        gabarit: "Décision en attente depuis {nombre} jours",
        variables: &[Var::Nombre],
        destinataires: "direction (file de décision, escalade d'une tâche bloquante)",
        cardinalite: Cardinalite::Collective,
        echeance_jours: None,
        priorite: PrioriteAfaire::Bloquante,
        action: Some(ActionDeclaree::Lien { libelle: "Décider", lien: "/web/a-faire" }),
        file_repli: FILE_DECISIONS,
        escalade: Some(EscaladeDeclaree { delai_heures: 168, file: FILE_ADMIN }),
        // Ouverte par l'escalade du 29 lui-même : pas de source à rapprocher.
        derivee: false,
        ..BASE
    },
];

/// Nature déclarée par son code.
pub fn nature(code: &str) -> Option<&'static NatureDeclaree> {
    NATURES.iter().find(|n| n.code == code)
}

/// Contrôle des déclarations (POR-243, R-PO23) : retourne toutes les fautes, pas seulement la première.
pub fn verifier_declarations(natures: &[NatureDeclaree]) -> Result<(), Vec<String>> {
    let mut fautes = Vec::new();
    let mut vus = HashSet::new();
    for n in natures {
        let c = n.code;
        let forme = c.split('.').count() == 2
            && c.chars()
                .all(|ch| ch.is_ascii_lowercase() || ch == '.' || ch == '_')
            && !c.starts_with('.')
            && !c.ends_with('.');
        if !forme {
            fautes.push(format!("{c} : code invalide (attendu `brique.nature`)"));
        }
        if !vus.insert(c) {
            fautes.push(format!("{c} : code déclaré deux fois"));
        }
        if n.socle <= 0 {
            fautes.push(format!("{c} : socle propriétaire manquant"));
        }
        if n.objet_type.is_empty() {
            fautes.push(format!("{c} : type d'objet manquant"));
        }
        if n.gabarit.trim().is_empty() {
            fautes.push(format!("{c} : gabarit de titre neutre manquant"));
        }
        if n.destinataires.trim().is_empty() {
            fautes.push(format!("{c} : règle de destinataires manquante"));
        }
        if n.file_repli.trim().is_empty() {
            fautes.push(format!("{c} : file de repli manquante (A13)"));
        }
        if n.action.is_none() {
            fautes.push(format!("{c} : action de clôture manquante"));
        }
        if n.sensibilite > 3 {
            fautes.push(format!("{c} : sensibilité hors C0–C3"));
        }
        if n.priorite == PrioriteAfaire::Bloquante && n.escalade.is_none() {
            fautes.push(format!("{c} : tâche bloquante sans escalade (R-PO11)"));
        }
        if n.conservation_jours <= 0 {
            fautes.push(format!("{c} : catégorie de conservation manquante"));
        }
        // Variables : exactement celles du gabarit, typées, non sensibles.
        let mut dans_gabarit = Vec::new();
        let mut reste = n.gabarit;
        while let Some(i) = reste.find('{') {
            let Some(j) = reste[i..].find('}') else {
                fautes.push(format!("{c} : accolade non fermée dans le gabarit"));
                break;
            };
            let nom = &reste[i + 1..i + j];
            match Var::depuis_code(nom) {
                Some(v) => dans_gabarit.push(v),
                None => fautes.push(format!(
                    "{c} : variable libre ou inconnue `{{{nom}}}` (R-PO5)"
                )),
            }
            reste = &reste[i + j + 1..];
        }
        for v in &dans_gabarit {
            if !n.variables.contains(v) {
                fautes.push(format!(
                    "{c} : variable `{}` utilisée sans être déclarée",
                    v.code()
                ));
            }
        }
        for v in n.variables {
            if !dans_gabarit.contains(v) {
                fautes.push(format!(
                    "{c} : variable `{}` déclarée mais absente du gabarit",
                    v.code()
                ));
            }
            if v.sensibilite() >= 2 && !(n.reservee && n.sensibilite >= v.sensibilite()) {
                fautes.push(format!(
                    "{c} : la variable `{}` (C{}) est refusée hors nature réservée (R-PO5, §15.3)",
                    v.code(),
                    v.sensibilite()
                ));
            }
        }
        if n.gabarit.chars().count() + 40 * n.variables.len() > 220 {
            fautes.push(format!("{c} : gabarit trop long"));
        }
    }
    if fautes.is_empty() {
        Ok(())
    } else {
        Err(fautes)
    }
}

/// Titre neutre ≤ 120 caractères composé depuis le gabarit déclaré (R-PO5).
pub fn composer_titre(n: &NatureDeclaree, valeurs: &[(Var, String)]) -> Result<String, EntDbError> {
    let mut titre = n.gabarit.to_string();
    for v in n.variables {
        let Some((_, val)) = valeurs.iter().find(|(k, _)| k == v) else {
            return Err(EntDbError::SaisieInvalide("variable de gabarit manquante"));
        };
        let propre: String = val.chars().filter(|c| !c.is_control()).collect();
        titre = titre.replace(&format!("{{{}}}", v.code()), propre.trim());
    }
    Ok(titre.chars().take(120).collect())
}

// ---------------------------------------------------------------------------------------------
// Heures murales, plage calme (R-PO31)
// ---------------------------------------------------------------------------------------------

/// Fin de la plage calme pour un instant local, ou `None` hors plage. Arrêté du 7 septembre 2026 :
/// 20 h – 7 h en semaine, du vendredi 20 h au lundi 7 h.
pub fn fin_plage_calme(local: NaiveDateTime) -> Option<NaiveDateTime> {
    let h = local.hour();
    let sept_heures = |d: NaiveDateTime| d.date().and_hms_opt(7, 0, 0).unwrap_or(d);
    match local.weekday() {
        Weekday::Sat | Weekday::Sun => {
            let jours = i64::from(7 - local.weekday().num_days_from_monday());
            Some(sept_heures(local + Duration::days(jours)))
        }
        Weekday::Fri if h >= 20 => Some(sept_heures(local + Duration::days(3))),
        _ if h >= 20 => Some(sept_heures(local + Duration::days(1))),
        _ if h < 7 => Some(sept_heures(local)),
        _ => None,
    }
}

/// Prochain point de rafraîchissement neutre (S-2914) : 7 h locales strictement après l'instant
/// (au plus 24 h plus tard).
pub fn prochain_point_neutre(local: NaiveDateTime) -> NaiveDateTime {
    let sept = local.date().and_hms_opt(7, 0, 0).unwrap_or(local);
    if local < sept {
        sept
    } else {
        sept + Duration::days(1)
    }
}

/// Heure murale de l'établissement pour un instant (fuseau du paramètre d'établissement, Paris par défaut).
pub async fn heure_locale(
    conn: &mut PgConnection,
    etab: Uuid,
    instant: DateTime<Utc>,
) -> Result<NaiveDateTime, EntDbError> {
    let (l,): (NaiveDateTime,) = sqlx::query_as(
        "SELECT ($2::timestamptz AT TIME ZONE COALESCE(
             (SELECT p.fuseau FROM ent_etablissement_parametre p WHERE p.etablissement_id = $1),
             'Europe/Paris'))",
    )
    .bind(etab)
    .bind(instant)
    .fetch_one(&mut *conn)
    .await
    .map_err(db)?;
    Ok(l)
}

/// Instant UTC d'une heure murale de l'établissement.
pub async fn instant_depuis_local(
    conn: &mut PgConnection,
    etab: Uuid,
    local: NaiveDateTime,
) -> Result<DateTime<Utc>, EntDbError> {
    let (i,): (DateTime<Utc>,) = sqlx::query_as(
        "SELECT ($2::timestamp AT TIME ZONE COALESCE(
             (SELECT p.fuseau FROM ent_etablissement_parametre p WHERE p.etablissement_id = $1),
             'Europe/Paris'))",
    )
    .bind(etab)
    .bind(local)
    .fetch_one(&mut *conn)
    .await
    .map_err(db)?;
    Ok(i)
}

/// Échéance « jour J 23 h 59 » en heure murale (R-PO21).
pub async fn fin_de_journee(
    conn: &mut PgConnection,
    etab: Uuid,
    jour: chrono::NaiveDate,
) -> Result<DateTime<Utc>, EntDbError> {
    let local = jour.and_hms_opt(23, 59, 0).unwrap_or_default();
    instant_depuis_local(conn, etab, local).await
}

// ---------------------------------------------------------------------------------------------
// Destinataires
// ---------------------------------------------------------------------------------------------

/// Destinataire d'une tâche.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Destinataire {
    /// Un responsable (D-020), avec ou sans compte.
    Personne(Uuid),
    /// Un membre du personnel (ou un élève majeur) désigné par son compte.
    Compte(Uuid),
    Foyer(Uuid),
    File(String),
}

impl Destinataire {
    pub fn cle(&self) -> String {
        match self {
            Self::Personne(i) => format!("p:{i}"),
            Self::Compte(i) => format!("c:{i}"),
            Self::Foyer(i) => format!("f:{i}"),
            Self::File(c) => format!("q:{c}"),
        }
    }
    fn type_code(&self) -> &'static str {
        match self {
            Self::Personne(_) => "personne",
            Self::Compte(_) => "compte",
            Self::Foyer(_) => "foyer",
            Self::File(_) => "file",
        }
    }
}

/// Motif **neutre** de clôture (code fermé, jamais de texte libre).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MotifClotureAfaire {
    Faite,
    SansObjet,
    Expiree,
    Annulee,
    Restauration,
    Deces,
    Depart,
    DroitPerdu,
}

impl MotifClotureAfaire {
    pub fn code(self) -> &'static str {
        match self {
            Self::Faite => "faite",
            Self::SansObjet => "sans_objet",
            Self::Expiree => "expiree",
            Self::Annulee => "annulee",
            Self::Restauration => "restauration",
            Self::Deces => "deces",
            Self::Depart => "depart",
            Self::DroitPerdu => "droit_perdu",
        }
    }
    /// État final qui en découle.
    pub fn etat(self) -> EtatAfaire {
        match self {
            Self::Faite => EtatAfaire::Faite,
            Self::Expiree => EtatAfaire::Expiree,
            Self::Annulee => EtatAfaire::Annulee,
            _ => EtatAfaire::SansObjet,
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Ouverture
// ---------------------------------------------------------------------------------------------

/// Demande d'ouverture (port `ouvrir_tache`).
#[derive(Clone, Debug)]
pub struct OuvertureTache {
    pub etablissement_id: Uuid,
    pub nature: &'static str,
    pub objet_id: Uuid,
    pub eleve_id: Option<Uuid>,
    pub valeurs: Vec<(Var, String)>,
    /// Vide : la tâche va à la file de repli de la nature (R-PO9), jamais nulle part.
    pub destinataires: Vec<Destinataire>,
    pub echeance_le: Option<DateTime<Utc>>,
    /// Ordre des faits du propriétaire : un fait plus ancien n'écrase pas un plus récent (CPO-51).
    pub version_source: i64,
    pub par_compte_id: Option<Uuid>,
    pub maintenant: DateTime<Utc>,
}

/// Résultat d'une ouverture.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ResultatOuverture {
    pub taches_creees: usize,
    pub taches_rouvertes: usize,
    pub destinataires_ajoutes: usize,
    /// Fait ignoré parce qu'un fait plus récent est déjà connu.
    pub ignoree: bool,
}

/// Version source d'un instant : microsecondes depuis l'époque.
pub fn version_de(instant: DateTime<Utc>) -> i64 {
    instant.timestamp_micros()
}

async fn nature_active(
    conn: &mut PgConnection,
    etab: Uuid,
    code: &str,
) -> Result<bool, EntDbError> {
    let r: Option<(bool, String)> = sqlx::query_as(
        "SELECT active, pendant_doublon FROM ent_afaire_nature_reglage
          WHERE etablissement_id = $1 AND nature_code = $2",
    )
    .bind(etab)
    .bind(code)
    .fetch_optional(&mut *conn)
    .await
    .map_err(db)?;
    Ok(r.is_none_or(|(a, _)| a))
}

#[allow(clippy::too_many_arguments)]
async fn journaliser(
    conn: &mut PgConnection,
    etab: Uuid,
    afaire: Uuid,
    de: Option<EtatAfaire>,
    vers: EtatAfaire,
    par: Option<Uuid>,
    socle: i16,
    motif: Option<&str>,
    le: DateTime<Utc>,
) -> Result<(), EntDbError> {
    sqlx::query(
        "INSERT INTO ent_afaire_journal
            (id, etablissement_id, afaire_id, de_etat, vers_etat, par_compte_id, par_socle, motif_code, le)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
    )
    .bind(Uuid::new_v4())
    .bind(etab)
    .bind(afaire)
    .bind(de.map(EtatAfaire::code))
    .bind(vers.code())
    .bind(par)
    .bind(socle)
    .bind(motif)
    .bind(le)
    .execute(&mut *conn)
    .await
    .map_err(db)?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn ajouter_destinataire(
    conn: &mut PgConnection,
    etab: Uuid,
    afaire: Uuid,
    nature_code: &str,
    objet_id: Uuid,
    d: &Destinataire,
    role: &str,
    maintenant: DateTime<Utc>,
) -> Result<bool, EntDbError> {
    let cle = d.cle();
    let (personne, compte, foyer, file) = match d {
        Destinataire::Personne(i) => (Some(*i), None, None, None),
        Destinataire::Compte(i) => (None, Some(*i), None, None),
        Destinataire::Foyer(i) => (None, None, Some(*i), None),
        Destinataire::File(c) => (None, None, None, Some(c.clone())),
    };
    let n = sqlx::query(
        "INSERT INTO ent_afaire_destinataire
            (id, etablissement_id, afaire_id, type, personne_id, compte_id, foyer_id, file_code,
             role_dans_tache, destinataire_cle, ajoute_le)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
         ON CONFLICT (afaire_id, destinataire_cle) WHERE retire_le IS NULL DO NOTHING",
    )
    .bind(Uuid::new_v4())
    .bind(etab)
    .bind(afaire)
    .bind(d.type_code())
    .bind(personne)
    .bind(compte)
    .bind(foyer)
    .bind(file)
    .bind(role)
    .bind(&cle)
    .bind(maintenant)
    .execute(&mut *conn)
    .await
    .map_err(db)?
    .rows_affected();
    sqlx::query(
        "INSERT INTO ent_afaire_cle (etablissement_id, nature_code, objet_id, destinataire_cle, afaire_id)
         VALUES ($1, $2, $3, $4, $5)
         ON CONFLICT (etablissement_id, nature_code, objet_id, destinataire_cle)
         DO UPDATE SET afaire_id = EXCLUDED.afaire_id",
    )
    .bind(etab)
    .bind(nature_code)
    .bind(objet_id)
    .bind(&cle)
    .bind(afaire)
    .execute(&mut *conn)
    .await
    .map_err(db)?;
    Ok(n > 0)
}

/// Port `ouvrir_tache` : idempotent sur `(nature, objet, destinataire)`.
pub async fn ouvrir_tache(
    conn: &mut PgConnection,
    o: &OuvertureTache,
) -> Result<ResultatOuverture, EntDbError> {
    let n = nature(o.nature).ok_or(EntDbError::SaisieInvalide("nature de tâche inconnue"))?;
    let etab = o.etablissement_id;
    let mut res = ResultatOuverture::default();
    if !nature_active(conn, etab, n.code).await? {
        return Ok(res);
    }
    let titre = composer_titre(n, &o.valeurs)?;

    // Aucun destinataire actif : file de repli (A13), jamais nulle part.
    let (dests, role): (Vec<Destinataire>, &str) = if o.destinataires.is_empty() {
        (vec![Destinataire::File(n.file_repli.to_string())], "repli")
    } else {
        (o.destinataires.clone(), "titulaire")
    };

    // Plage calme : une famille ou un élève ne voit la tâche qu'à la fin de la plage (R-PO31).
    let visible = if n.public_famille && !n.urgente {
        let local = heure_locale(conn, etab, o.maintenant).await?;
        match fin_plage_calme(local) {
            Some(fin) => instant_depuis_local(conn, etab, fin).await?,
            None => o.maintenant,
        }
    } else {
        o.maintenant
    };
    let echeance = match (o.echeance_le, n.echeance_jours) {
        (Some(e), _) => Some(e),
        (None, Some(j)) => Some(o.maintenant + Duration::days(j)),
        _ => None,
    };

    // Groupes de destinataires : une tâche par destinataire (par responsable) ou une seule partagée.
    let groupes: Vec<Vec<Destinataire>> = if n.cardinalite.une_tache_par_destinataire() {
        dests.iter().map(|d| vec![d.clone()]).collect()
    } else {
        vec![dests.clone()]
    };

    for groupe in groupes {
        // Tâche déjà connue pour l'une des identités du groupe ?
        let cles: Vec<String> = groupe.iter().map(Destinataire::cle).collect();
        let existant: Option<(Uuid,)> = sqlx::query_as(
            "SELECT afaire_id FROM ent_afaire_cle
              WHERE etablissement_id = $1 AND nature_code = $2 AND objet_id = $3
                AND destinataire_cle = ANY($4)
              ORDER BY cree_le LIMIT 1",
        )
        .bind(etab)
        .bind(n.code)
        .bind(o.objet_id)
        .bind(&cles)
        .fetch_optional(&mut *conn)
        .await
        .map_err(db)?;

        let afaire_id = match existant {
            Some((id,)) => {
                let r = sqlx::query(
                    "SELECT etat, version_source FROM ent_afaire
                      WHERE etablissement_id = $1 AND id = $2 FOR UPDATE",
                )
                .bind(etab)
                .bind(id)
                .fetch_one(&mut *conn)
                .await
                .map_err(db)?;
                let etat = EtatAfaire::parse(&r.get::<String, _>(0)).unwrap_or(EtatAfaire::Ouverte);
                let version: i64 = r.get(1);
                if o.version_source < version {
                    // Fait plus ancien que celui déjà connu (CPO-51) : on n'écrase rien.
                    res.ignoree = true;
                    continue;
                }
                if etat.est_ouvert() {
                    sqlx::query(
                        "UPDATE ent_afaire SET titre_neutre = $3, echeance_le = $4, version_source = $5,
                                eleve_id = COALESCE($6, eleve_id), modifie_le = $7
                          WHERE etablissement_id = $1 AND id = $2",
                    )
                    .bind(etab)
                    .bind(id)
                    .bind(&titre)
                    .bind(echeance)
                    .bind(o.version_source)
                    .bind(o.eleve_id)
                    .bind(o.maintenant)
                    .execute(&mut *conn)
                    .await
                    .map_err(db)?;
                } else {
                    // Le propriétaire dit de nouveau « ouverte » (refus, restauration, nouvelle demande).
                    sqlx::query(
                        "UPDATE ent_afaire SET etat = 'ouverte', close_le = NULL, issue = NULL,
                                titre_neutre = $3, echeance_le = $4, version_source = $5,
                                visible_a_partir_de = $6, modifie_le = $7
                          WHERE etablissement_id = $1 AND id = $2",
                    )
                    .bind(etab)
                    .bind(id)
                    .bind(&titre)
                    .bind(echeance)
                    .bind(o.version_source)
                    .bind(visible)
                    .bind(o.maintenant)
                    .execute(&mut *conn)
                    .await
                    .map_err(db)?;
                    sqlx::query(
                        "UPDATE ent_afaire_destinataire SET masque_a_partir_de = NULL, modifie_le = $3
                          WHERE etablissement_id = $1 AND afaire_id = $2",
                    )
                    .bind(etab)
                    .bind(id)
                    .bind(o.maintenant)
                    .execute(&mut *conn)
                    .await
                    .map_err(db)?;
                    journaliser(
                        conn,
                        etab,
                        id,
                        Some(etat),
                        EtatAfaire::Ouverte,
                        o.par_compte_id,
                        n.socle,
                        Some("rouverte"),
                        o.maintenant,
                    )
                    .await?;
                    res.taches_rouvertes += 1;
                }
                id
            }
            None => {
                let id = Uuid::new_v4();
                let annee: Option<(Uuid,)> = sqlx::query_as(
                    "SELECT id FROM ent_annee_scolaire WHERE etablissement_id = $1 AND courante",
                )
                .bind(etab)
                .fetch_optional(&mut *conn)
                .await
                .map_err(db)?;
                sqlx::query(
                    "INSERT INTO ent_afaire
                        (id, etablissement_id, nature_code, socle_proprietaire, objet_type, objet_id,
                         eleve_id, titre_neutre, priorite, echeance_le, echeance_dure,
                         visible_a_partir_de, sensibilite, etat, annee_scolaire_id, version_source,
                         ouverte_le)
                     VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, 'ouverte', $14, $15, $16)",
                )
                .bind(id)
                .bind(etab)
                .bind(n.code)
                .bind(n.socle)
                .bind(n.objet_type)
                .bind(o.objet_id)
                .bind(o.eleve_id)
                .bind(&titre)
                .bind(n.priorite.code())
                .bind(echeance)
                .bind(n.echeance_dure)
                .bind(visible)
                .bind(i16::from(n.sensibilite))
                .bind(annee.map(|a| a.0))
                .bind(o.version_source)
                .bind(o.maintenant)
                .execute(&mut *conn)
                .await
                .map_err(db)?;
                journaliser(
                    conn,
                    etab,
                    id,
                    None,
                    EtatAfaire::Ouverte,
                    o.par_compte_id,
                    n.socle,
                    None,
                    o.maintenant,
                )
                .await?;
                res.taches_creees += 1;
                id
            }
        };
        for d in &groupe {
            if ajouter_destinataire(
                conn,
                etab,
                afaire_id,
                n.code,
                o.objet_id,
                d,
                role,
                o.maintenant,
            )
            .await?
            {
                res.destinataires_ajoutes += 1;
            }
        }
    }
    Ok(res)
}

// ---------------------------------------------------------------------------------------------
// Fermeture
// ---------------------------------------------------------------------------------------------

/// Demande de fermeture (port `fermer_tache`).
#[derive(Clone, Debug)]
pub struct FermetureTache {
    pub etablissement_id: Uuid,
    pub nature: &'static str,
    pub objet_id: Uuid,
    /// `None` : toutes les tâches de l'objet. Seule une tâche « par responsable » est restreinte.
    pub destinataire: Option<Destinataire>,
    pub motif: MotifClotureAfaire,
    /// Compte qui a fait l'acte (pour la neutralité entre foyers) ; `None` = système ou établissement.
    pub par_compte_id: Option<Uuid>,
    pub version_source: i64,
    pub maintenant: DateTime<Utc>,
}

/// Port `fermer_tache` : retourne le nombre de tâches fermées. Idempotent ; sans effet sur une
/// tâche déjà close ou sur un fait plus ancien que le dernier connu (CPO-51).
pub async fn fermer_tache(
    conn: &mut PgConnection,
    f: &FermetureTache,
) -> Result<usize, EntDbError> {
    let n = nature(f.nature).ok_or(EntDbError::SaisieInvalide("nature de tâche inconnue"))?;
    let etab = f.etablissement_id;
    let ids: Vec<(Uuid,)> = match (&f.destinataire, n.cardinalite.une_tache_par_destinataire()) {
        (Some(d), true) => sqlx::query_as(
            "SELECT DISTINCT afaire_id FROM ent_afaire_cle
              WHERE etablissement_id = $1 AND nature_code = $2 AND objet_id = $3 AND destinataire_cle = $4",
        )
        .bind(etab)
        .bind(n.code)
        .bind(f.objet_id)
        .bind(d.cle())
        .fetch_all(&mut *conn)
        .await
        .map_err(db)?,
        _ => sqlx::query_as(
            "SELECT DISTINCT afaire_id FROM ent_afaire_cle
              WHERE etablissement_id = $1 AND nature_code = $2 AND objet_id = $3",
        )
        .bind(etab)
        .bind(n.code)
        .bind(f.objet_id)
        .fetch_all(&mut *conn)
        .await
        .map_err(db)?,
    };
    let mut fermees = 0;
    for (id,) in ids {
        let r = sqlx::query(
            "SELECT etat, version_source, eleve_id FROM ent_afaire
              WHERE etablissement_id = $1 AND id = $2 FOR UPDATE",
        )
        .bind(etab)
        .bind(id)
        .fetch_optional(&mut *conn)
        .await
        .map_err(db)?;
        let Some(r) = r else { continue };
        let etat = EtatAfaire::parse(&r.get::<String, _>(0)).unwrap_or(EtatAfaire::Ouverte);
        let version: i64 = r.get(1);
        let eleve: Option<Uuid> = r.get(2);
        if !etat.est_ouvert() || f.version_source < version {
            continue;
        }
        let vers = f.motif.etat();
        sqlx::query(
            "UPDATE ent_afaire SET etat = $3, close_le = $4, issue = $5, version_source = $6, modifie_le = $4
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(id)
        .bind(vers.code())
        .bind(f.maintenant)
        .bind(f.motif.code())
        .bind(f.version_source)
        .execute(&mut *conn)
        .await
        .map_err(db)?;
        journaliser(
            conn,
            etab,
            id,
            Some(etat),
            vers,
            f.par_compte_id,
            n.socle,
            Some(f.motif.code()),
            f.maintenant,
        )
        .await?;
        // « Nouveau / vu » est effacé à la clôture (§29.4) : aucune trace de lecture ne survit.
        sqlx::query(
            "DELETE FROM ent_afaire_etat_personnel WHERE etablissement_id = $1 AND afaire_id = $2",
        )
        .bind(etab)
        .bind(id)
        .execute(&mut *conn)
        .await
        .map_err(db)?;
        // Neutralité temporelle (R-PO38) : fait par un responsable d'un AUTRE foyer, la tâche reste
        // affichée chez l'autre jusqu'au prochain point de rafraîchissement neutre ; fait par le même
        // foyer ou par l'établissement, elle disparaît tout de suite (CPO-75).
        if f.motif == MotifClotureAfaire::Faite {
            if let Some(compte) = f.par_compte_id {
                masquer_autres_foyers(conn, etab, id, compte, eleve, f.maintenant).await?;
            }
        }
        fermees += 1;
    }
    Ok(fermees)
}

async fn masquer_autres_foyers(
    conn: &mut PgConnection,
    etab: Uuid,
    afaire: Uuid,
    par_compte: Uuid,
    eleve: Option<Uuid>,
    maintenant: DateTime<Utc>,
) -> Result<(), EntDbError> {
    // L'auteur est-il un responsable de l'élève ? Sinon (personnel, établissement) : rien à masquer.
    let auteur: Option<(Uuid, Option<Uuid>)> = match eleve {
        Some(e) => sqlx::query_as(
            "SELECT p.id, l.foyer_id
               FROM ent_personne p
               JOIN ent_eleve_lien l ON l.personne_id = p.id AND l.etablissement_id = p.etablissement_id
              WHERE p.etablissement_id = $1 AND p.user_id = $2 AND l.eleve_id = $3
              LIMIT 1",
        )
        .bind(etab)
        .bind(par_compte)
        .bind(e)
        .fetch_optional(&mut *conn)
        .await
        .map_err(db)?,
        None => None,
    };
    let Some((auteur_personne, auteur_foyer)) = auteur else {
        return Ok(());
    };
    let local = heure_locale(conn, etab, maintenant).await?;
    let jusqu = instant_depuis_local(conn, etab, prochain_point_neutre(local)).await?;
    sqlx::query(
        "UPDATE ent_afaire_destinataire d SET masque_a_partir_de = $3, modifie_le = $4
          WHERE d.etablissement_id = $1 AND d.afaire_id = $2 AND d.retire_le IS NULL
            AND d.type = 'personne' AND d.personne_id <> $5
            AND NOT EXISTS (
                SELECT 1 FROM ent_eleve_lien l
                 WHERE l.etablissement_id = d.etablissement_id AND l.personne_id = d.personne_id
                   AND l.eleve_id = $6 AND l.foyer_id IS NOT NULL AND l.foyer_id = $7)",
    )
    .bind(etab)
    .bind(afaire)
    .bind(jusqu)
    .bind(maintenant)
    .bind(auteur_personne)
    .bind(eleve)
    .bind(auteur_foyer)
    .execute(&mut *conn)
    .await
    .map_err(db)?;
    Ok(())
}

/// Port `etat_taches(objet)` : y a-t-il encore une tâche ouverte sur cet objet ? Le socle 8 l'interroge
/// avant toute relance (R-PO6) : aucune relance pour une tâche close.
pub async fn tache_ouverte_sur(
    conn: &mut PgConnection,
    etab: Uuid,
    nature_code: &str,
    objet_id: Uuid,
) -> Result<bool, EntDbError> {
    let (n,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM ent_afaire
          WHERE etablissement_id = $1 AND nature_code = $2 AND objet_id = $3
            AND etat IN ('ouverte', 'prise', 'en_attente')",
    )
    .bind(etab)
    .bind(nature_code)
    .bind(objet_id)
    .fetch_one(&mut *conn)
    .await
    .map_err(db)?;
    Ok(n > 0)
}

/// Escalade des tâches bloquantes échues (§8.5) : la file de la nature **reçoit** la tâche (ajout d'un
/// destinataire de rôle `escalade`), sans rien retirer. Retourne le nombre de tâches escaladées.
pub async fn escalader_echues(
    conn: &mut PgConnection,
    etab: Uuid,
    maintenant: DateTime<Utc>,
) -> Result<usize, EntDbError> {
    let rows = sqlx::query(
        "SELECT id, nature_code, objet_id, ouverte_le FROM ent_afaire
          WHERE etablissement_id = $1 AND priorite = 'bloquante'
            AND etat IN ('ouverte', 'prise', 'en_attente')",
    )
    .bind(etab)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    let mut n_esc = 0;
    for r in rows {
        let (id, code, objet, ouverte): (Uuid, String, Uuid, DateTime<Utc>) =
            (r.get(0), r.get(1), r.get(2), r.get(3));
        let Some(n) = nature(&code) else { continue };
        let Some(esc) = n.escalade else { continue };
        if maintenant < ouverte + Duration::hours(esc.delai_heures) {
            continue;
        }
        if ajouter_destinataire(
            conn,
            etab,
            id,
            &code,
            objet,
            &Destinataire::File(esc.file.to_string()),
            "escalade",
            maintenant,
        )
        .await?
        {
            n_esc += 1;
        }
    }
    Ok(n_esc)
}

/// Priorité effective : une tâche passe `due` quand l'échéance approche ou est passée (§8.5).
pub fn priorite_effective(
    stockee: PrioriteAfaire,
    echeance: Option<DateTime<Utc>>,
    montee_heures: Option<i64>,
    maintenant: DateTime<Utc>,
) -> PrioriteAfaire {
    if stockee != PrioriteAfaire::AFaire {
        return stockee;
    }
    match (echeance, montee_heures) {
        (Some(e), _) if e <= maintenant => PrioriteAfaire::Due,
        (Some(e), Some(h)) if e - Duration::hours(h) <= maintenant => PrioriteAfaire::Due,
        _ => stockee,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn local(y: i32, m: u32, j: u32, h: u32, mi: u32) -> NaiveDateTime {
        NaiveDate::from_ymd_opt(y, m, j)
            .unwrap()
            .and_hms_opt(h, mi, 0)
            .unwrap()
    }

    #[test]
    fn les_natures_declarees_passent_le_controle_du_kit() {
        verifier_declarations(NATURES).expect("déclarations complètes");
    }

    #[test]
    fn une_declaration_incomplete_est_refusee() {
        let vide = NatureDeclaree {
            code: "x.y",
            socle: 1,
            objet_type: "o",
            ..BASE
        };
        let fautes = verifier_declarations(&[vide]).unwrap_err().join("|");
        for attendu in [
            "gabarit",
            "destinataires",
            "file de repli",
            "action de clôture",
        ] {
            assert!(
                fautes.contains(attendu),
                "{attendu} manquant dans : {fautes}"
            );
        }
    }

    #[test]
    fn un_gabarit_a_variable_sensible_ou_libre_est_refuse() {
        let base = NatureDeclaree {
            code: "t.nature",
            socle: 1,
            objet_type: "o",
            destinataires: "x",
            file_repli: "q.x",
            action: Some(ActionDeclaree::Lien {
                libelle: "a",
                lien: "/",
            }),
            ..BASE
        };
        // Variable sensible hors nature réservée (R-PO5, §15.3).
        let sensible = NatureDeclaree {
            gabarit: "Facture {montant}",
            variables: &[Var::Montant],
            ..base
        };
        assert!(verifier_declarations(&[sensible])
            .unwrap_err()
            .join("|")
            .contains("refusée hors nature réservée"));
        // Même variable dans une nature réservée de sensibilité suffisante : admise.
        let reservee = NatureDeclaree {
            gabarit: "Facture {montant}",
            variables: &[Var::Montant],
            reservee: true,
            sensibilite: 2,
            ..base
        };
        verifier_declarations(&[reservee]).expect("nature réservée");
        // Variable libre.
        let libre = NatureDeclaree {
            gabarit: "Absence {texte_libre}",
            ..base
        };
        assert!(verifier_declarations(&[libre])
            .unwrap_err()
            .join("|")
            .contains("variable libre"));
        // Un motif ne figure jamais dans l'index d'une nature ordinaire.
        let motif = NatureDeclaree {
            gabarit: "Absence {motif}",
            variables: &[Var::Motif],
            ..base
        };
        assert!(verifier_declarations(&[motif]).is_err());
    }

    #[test]
    fn une_tache_bloquante_sans_escalade_est_refusee() {
        let n = NatureDeclaree {
            code: "t.bloque",
            socle: 1,
            objet_type: "o",
            gabarit: "Décision",
            destinataires: "x",
            file_repli: "q.x",
            action: Some(ActionDeclaree::Lien {
                libelle: "a",
                lien: "/",
            }),
            priorite: PrioriteAfaire::Bloquante,
            ..BASE
        };
        assert!(verifier_declarations(&[n])
            .unwrap_err()
            .join("|")
            .contains("sans escalade"));
    }

    #[test]
    fn le_titre_est_compose_depuis_le_gabarit_et_borne() {
        let n = nature("vs.justifier_absence").unwrap();
        let t = composer_titre(
            n,
            &[(Var::Prenom, "Léa".into()), (Var::Date, "12/11".into())],
        )
        .unwrap();
        assert_eq!(t, "Absence à justifier — Léa, 12/11");
        assert!(
            composer_titre(n, &[(Var::Prenom, "Léa".into())]).is_err(),
            "variable manquante"
        );
        let long = composer_titre(
            n,
            &[(Var::Prenom, "x".repeat(500)), (Var::Date, "12/11".into())],
        )
        .unwrap();
        assert!(long.chars().count() <= 120);
    }

    #[test]
    fn plage_calme_semaine_vendredi_soir_et_week_end() {
        // Jeudi 12/11/2026, 21 h → vendredi 7 h.
        assert_eq!(
            fin_plage_calme(local(2026, 11, 12, 21, 0)),
            Some(local(2026, 11, 13, 7, 0))
        );
        // Mercredi 14 h : hors plage.
        assert_eq!(fin_plage_calme(local(2026, 11, 11, 14, 0)), None);
        // Mardi 6 h 30 → mardi 7 h.
        assert_eq!(
            fin_plage_calme(local(2026, 11, 10, 6, 30)),
            Some(local(2026, 11, 10, 7, 0))
        );
        // Vendredi 13/11 à 20 h → lundi 16/11 à 7 h.
        assert_eq!(
            fin_plage_calme(local(2026, 11, 13, 20, 0)),
            Some(local(2026, 11, 16, 7, 0))
        );
        // Vendredi 19 h 59 : encore hors plage.
        assert_eq!(fin_plage_calme(local(2026, 11, 13, 19, 59)), None);
        // Samedi et dimanche → lundi 7 h.
        assert_eq!(
            fin_plage_calme(local(2026, 11, 14, 10, 0)),
            Some(local(2026, 11, 16, 7, 0))
        );
        assert_eq!(
            fin_plage_calme(local(2026, 11, 15, 23, 0)),
            Some(local(2026, 11, 16, 7, 0))
        );
        // Lundi 6 h : fin du week-end à 7 h.
        assert_eq!(
            fin_plage_calme(local(2026, 11, 16, 6, 0)),
            Some(local(2026, 11, 16, 7, 0))
        );
        assert_eq!(fin_plage_calme(local(2026, 11, 16, 7, 0)), None);
    }

    #[test]
    fn point_de_rafraichissement_neutre_au_plus_24_heures() {
        assert_eq!(
            prochain_point_neutre(local(2026, 11, 12, 23, 2)),
            local(2026, 11, 13, 7, 0)
        );
        assert_eq!(
            prochain_point_neutre(local(2026, 11, 12, 3, 0)),
            local(2026, 11, 12, 7, 0)
        );
        assert_eq!(
            prochain_point_neutre(local(2026, 11, 12, 7, 0)),
            local(2026, 11, 13, 7, 0)
        );
    }

    #[test]
    fn la_priorite_monte_a_l_approche_de_l_echeance() {
        let t0 = DateTime::parse_from_rfc3339("2026-11-12T10:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let dans = |h| Some(t0 + Duration::hours(h));
        assert_eq!(
            priorite_effective(PrioriteAfaire::AFaire, dans(72), Some(24), t0),
            PrioriteAfaire::AFaire
        );
        assert_eq!(
            priorite_effective(PrioriteAfaire::AFaire, dans(20), Some(24), t0),
            PrioriteAfaire::Due
        );
        assert_eq!(
            priorite_effective(PrioriteAfaire::AFaire, dans(-1), None, t0),
            PrioriteAfaire::Due
        );
        assert_eq!(
            priorite_effective(PrioriteAfaire::Bloquante, dans(100), None, t0),
            PrioriteAfaire::Bloquante
        );
        assert_eq!(
            priorite_effective(PrioriteAfaire::Information, dans(-5), None, t0),
            PrioriteAfaire::Information
        );
    }
}
