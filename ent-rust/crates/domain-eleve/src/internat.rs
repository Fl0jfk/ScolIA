//! Internat — chambres, affectations, présence en faits, suivis (socle 41, lots I0 et I1).
//!
//! Règles portées ici (pures, sans base ni réseau, testées en natif) :
//! * **R-IT2** régime codé : [`RegimeCode`], seule interprétation d'un régime (table
//!   `ent_regime_correspondance`, migration 2821 — un test de base vérifie qu'elles coïncident) ;
//! * **R-IT5** la nuit est la date du SOIR, fuseau de l'établissement : [`nuit_paris`] ;
//! * **R-IT10 / R-IT11 / R-IT12** contrôles d'affectation : [`plan_affecter_eleve_internat`] ;
//! * **R-IT13** présence en faits : [`plan_pointer_presence_internat`] (jour futur refusé, nuit passée
//!   avec motif) ;
//! * **R-IT6 / RI-09** suivis courts, sans santé : [`mots_de_sante_detectes`].
//!
//! Geste VS : créer des chambres → affecter un interne → pointer le soir → noter un suivi.
//! Famille : lit la chambre + présence (pas les notes internes).

use crate::DomainError;
use chrono::{DateTime, Duration, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Régime d'un élève, vocabulaire unique (R-IT2). Jamais de texte libre interprété ailleurs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RegimeCode {
    Externe,
    DemiPensionnaire,
    Interne,
    InterneExterne,
}

/// Table de correspondance `(vocabulaire, valeur normalisée, code)` : miroir exact de
/// `ent_regime_correspondance` (migration 2821). Le vocabulaire « scolarite » prime sur « sigle »,
/// qui prime sur « historique ».
pub const CORRESPONDANCES_REGIME: &[(&str, &str, RegimeCode)] = &[
    ("sigle", "ext", RegimeCode::Externe),
    ("sigle", "dp", RegimeCode::DemiPensionnaire),
    ("sigle", "int", RegimeCode::Interne),
    ("scolarite", "externe", RegimeCode::Externe),
    ("scolarite", "demi_pension", RegimeCode::DemiPensionnaire),
    ("scolarite", "interne", RegimeCode::Interne),
    ("historique", "e", RegimeCode::Externe),
    ("historique", "demi-pension", RegimeCode::DemiPensionnaire),
    ("historique", "demi_pensionnaire", RegimeCode::DemiPensionnaire),
    ("historique", "1/2p", RegimeCode::DemiPensionnaire),
    ("historique", "1/2 p", RegimeCode::DemiPensionnaire),
    ("historique", "i", RegimeCode::Interne),
    ("historique", "internat", RegimeCode::Interne),
    ("historique", "interne_externe", RegimeCode::InterneExterne),
    ("historique", "interne externé", RegimeCode::InterneExterne),
    ("historique", "ie", RegimeCode::InterneExterne),
];

impl RegimeCode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Externe => "externe",
            Self::DemiPensionnaire => "demi_pensionnaire",
            Self::Interne => "interne",
            Self::InterneExterne => "interne_externe",
        }
    }

    /// Fonction serveur UNIQUE d'interprétation d'un régime (R-IT2). `None` si inconnu : l'appelant
    /// refuse, il ne devine pas.
    pub fn depuis_texte(texte: &str) -> Option<Self> {
        let v = texte.trim().to_lowercase();
        ["scolarite", "sigle", "historique"].iter().find_map(|voc| {
            CORRESPONDANCES_REGIME
                .iter()
                .find(|(vocab, valeur, _)| vocab == voc && *valeur == v)
                .map(|(_, _, code)| *code)
        })
    }

    /// Seul l'interne est hébergé (R-IT1) ; l'interne externé n'a pas de séjour d'hébergement.
    pub fn est_heberge(self) -> bool {
        matches!(self, Self::Interne)
    }
}

/// Nuit (date du SOIR) d'un instant, fuseau Europe/Paris, y compris aux changements d'heure (R-IT5).
/// Midi → midi : de 12 h le jour D à 11 h 59 le jour D+1 = nuit du jour D. Identique à la fonction SQL
/// `ent_nuit_etab` (un test de base le vérifie sur les nuits de changement d'heure).
pub fn nuit_paris(instant: DateTime<Utc>) -> NaiveDate {
    let local = instant + Duration::hours(crate::horloge::decalage_paris_heures(instant));
    (local - Duration::hours(12)).date_naive()
}

/// Public d'une chambre (R-IT11) : aucune chambre mixte.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PublicChambre {
    Filles,
    Garcons,
}

impl PublicChambre {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Filles => "filles",
            Self::Garcons => "garcons",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_lowercase().as_str() {
            "filles" | "fille" | "f" => Some(Self::Filles),
            "garcons" | "garçons" | "garcon" | "garçon" | "g" => Some(Self::Garcons),
            _ => None,
        }
    }

    /// Public correspondant au sexe d'état civil (`M` / `F`) ; `None` si inconnu (pas de devinette).
    pub fn pour_sexe(sexe: &str) -> Option<Self> {
        match sexe.trim() {
            "F" | "f" => Some(Self::Filles),
            "M" | "m" => Some(Self::Garcons),
            _ => None,
        }
    }
}

/// État d'une chambre.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EtatChambre {
    Active,
    Fermee,
    Retiree,
}

impl EtatChambre {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Fermee => "fermee",
            Self::Retiree => "retiree",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "active" => Some(Self::Active),
            "fermee" => Some(Self::Fermee),
            "retiree" => Some(Self::Retiree),
            _ => None,
        }
    }
}

/// Genre hérité de la migration 066 (lecture seule). Une chambre « mixte » ancienne reste lisible
/// mais ne peut plus être créée ni recevoir d'affectation (R-IT11) tant que son public n'est pas fixé.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChambreGenre {
    Mixte,
    Filles,
    Garcons,
}

impl ChambreGenre {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Mixte => "mixte",
            Self::Filles => "filles",
            Self::Garcons => "garcons",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "mixte" | "m" => Some(Self::Mixte),
            "filles" | "f" | "fille" => Some(Self::Filles),
            "garcons" | "garçons" | "g" | "garcon" | "garçon" => Some(Self::Garcons),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChambreInternat {
    pub id: Uuid,
    pub etablissement_id: Uuid,
    pub batiment: String,
    pub etage: i16,
    pub code: String,
    pub capacite: i16,
    pub genre: ChambreGenre,
    /// Public (R-IT11) ; `None` pour une ancienne chambre « mixte » non encore reclassée.
    pub public: Option<PublicChambre>,
    pub zone_id: Option<Uuid>,
    pub accessible_pmr: bool,
    pub douche: bool,
    pub lavabo: bool,
    pub bureau: bool,
    pub etat: EtatChambre,
    pub ferme_du: Option<NaiveDate>,
    pub ferme_au: Option<NaiveDate>,
    pub notes_entretien: Option<String>,
    pub actif: bool,
    pub created_by_user_id: Uuid,
    pub created_at: DateTime<Utc>,
}

impl ChambreInternat {
    /// La chambre peut-elle recevoir une affectation le `jour` ? (active, non fermée ce jour-là).
    pub fn ouverte_au(&self, jour: NaiveDate) -> bool {
        if self.etat == EtatChambre::Retiree || !self.actif {
            return false;
        }
        match (self.ferme_du, self.ferme_au) {
            (Some(du), Some(au)) => !(du <= jour && jour <= au),
            (Some(du), None) => jour < du,
            _ => self.etat != EtatChambre::Fermee,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CreerChambreInternat {
    pub etablissement_id: Uuid,
    pub batiment: String,
    pub etage: i16,
    pub code: String,
    pub capacite: i16,
    /// Public de la chambre : « filles » ou « garcons » (« mixte » refusé, R-IT11).
    pub genre: String,
    pub actor_user_id: Uuid,
}

/// Plan de création : borne de capacité 1 à 8 lits (conception §8.1), public obligatoire.
pub fn plan_creer_chambre_internat(
    cmd: CreerChambreInternat,
) -> Result<ChambreInternat, DomainError> {
    if cmd.etablissement_id.is_nil() {
        return Err(DomainError::Event(
            ent_events::EventBusError::MissingEtablissement,
        ));
    }
    let batiment = cmd.batiment.trim().to_string();
    let code = cmd.code.trim().to_string();
    if batiment.is_empty() || code.is_empty() {
        return Err(DomainError::InternatChambreInvalide);
    }
    if cmd.capacite < 1 || cmd.capacite > CAPACITE_MAX_CHAMBRE {
        return Err(DomainError::InternatChambreInvalide);
    }
    let public = PublicChambre::parse(&cmd.genre).ok_or(DomainError::InternatChambreInvalide)?;
    Ok(ChambreInternat {
        id: Uuid::new_v4(),
        etablissement_id: cmd.etablissement_id,
        batiment,
        etage: cmd.etage,
        code,
        capacite: cmd.capacite,
        genre: match public {
            PublicChambre::Filles => ChambreGenre::Filles,
            PublicChambre::Garcons => ChambreGenre::Garcons,
        },
        public: Some(public),
        zone_id: None,
        accessible_pmr: false,
        douche: false,
        lavabo: false,
        bureau: false,
        etat: EtatChambre::Active,
        ferme_du: None,
        ferme_au: None,
        notes_entretien: None,
        actif: true,
        created_by_user_id: cmd.actor_user_id,
        created_at: Utc::now(),
    })
}

/// Nombre maximal de lits d'une chambre (conception §8.1 : 1 à 8).
pub const CAPACITE_MAX_CHAMBRE: i16 = 8;

/// Modification d'une chambre (INT-104). Chaque champ absent reste inchangé.
#[derive(Debug, Clone, Default)]
pub struct ModifierChambreInternat {
    pub code: Option<String>,
    pub capacite: Option<i16>,
    pub public: Option<PublicChambre>,
    pub zone_id: Option<Option<Uuid>>,
    pub accessible_pmr: Option<bool>,
    pub douche: Option<bool>,
    pub lavabo: Option<bool>,
    pub bureau: Option<bool>,
    pub etat: Option<EtatChambre>,
    pub ferme_du: Option<Option<NaiveDate>>,
    pub ferme_au: Option<Option<NaiveDate>>,
    pub notes_entretien: Option<Option<String>>,
}

/// Plan de modification. `pic_occupation` = plus grand nombre d'occupants simultanés de la chambre
/// à partir de `aujourdhui` (affectations ouvertes ou futures) ; `occupee` = au moins une
/// affectation ouverte ou future. Refus : capacité sous l'occupation, retrait / fermeture d'une
/// chambre occupée (le relogement est proposé au lot I1, INT-118), changement de public d'une
/// chambre occupée (R-IT11).
pub fn plan_modifier_chambre_internat(
    actuelle: &ChambreInternat,
    cmd: &ModifierChambreInternat,
    pic_occupation: i16,
    occupee: bool,
) -> Result<ChambreInternat, DomainError> {
    let mut c = actuelle.clone();
    if let Some(code) = &cmd.code {
        let code = code.trim();
        if code.is_empty() || code.chars().count() > 40 {
            return Err(DomainError::InternatChambreInvalide);
        }
        c.code = code.to_string();
    }
    if let Some(cap) = cmd.capacite {
        if cap < 1 || cap > CAPACITE_MAX_CHAMBRE {
            return Err(DomainError::InternatChambreInvalide);
        }
        if cap < pic_occupation {
            return Err(DomainError::InternatCapaciteSousOccupation);
        }
        c.capacite = cap;
    }
    if let Some(p) = cmd.public {
        if occupee && c.public != Some(p) {
            return Err(DomainError::InternatChambreOccupee);
        }
        c.public = Some(p);
        c.genre = match p {
            PublicChambre::Filles => ChambreGenre::Filles,
            PublicChambre::Garcons => ChambreGenre::Garcons,
        };
    }
    if let Some(z) = cmd.zone_id {
        c.zone_id = z;
    }
    if let Some(v) = cmd.accessible_pmr {
        c.accessible_pmr = v;
    }
    if let Some(v) = cmd.douche {
        c.douche = v;
    }
    if let Some(v) = cmd.lavabo {
        c.lavabo = v;
    }
    if let Some(v) = cmd.bureau {
        c.bureau = v;
    }
    if let Some(n) = &cmd.notes_entretien {
        let n = n.as_ref().map(|t| t.trim().to_string()).filter(|t| !t.is_empty());
        if n.as_ref().is_some_and(|t| t.chars().count() > 300) {
            return Err(DomainError::InternatChambreInvalide);
        }
        c.notes_entretien = n;
    }
    if let Some(d) = cmd.ferme_du {
        c.ferme_du = d;
    }
    if let Some(d) = cmd.ferme_au {
        c.ferme_au = d;
    }
    if c.ferme_au.is_some() && c.ferme_du.is_none() {
        return Err(DomainError::InternatChambreInvalide);
    }
    if let (Some(du), Some(au)) = (c.ferme_du, c.ferme_au) {
        if au < du {
            return Err(DomainError::InternatChambreInvalide);
        }
    }
    if let Some(e) = cmd.etat {
        if e != EtatChambre::Active && occupee {
            return Err(DomainError::InternatChambreOccupee);
        }
        c.etat = e;
        c.actif = e != EtatChambre::Retiree;
    }
    Ok(c)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AffectationInternat {
    pub id: Uuid,
    pub etablissement_id: Uuid,
    pub eleve_id: Uuid,
    pub chambre_id: Uuid,
    pub lit: Option<String>,
    pub lit_id: Option<Uuid>,
    pub sejour_id: Option<Uuid>,
    pub motif_code: Option<String>,
    pub debut: NaiveDate,
    pub fin: Option<NaiveDate>,
    pub created_by_user_id: Uuid,
    pub created_at: DateTime<Utc>,
}

impl AffectationInternat {
    pub fn est_active_au(&self, jour: NaiveDate) -> bool {
        self.debut <= jour && self.fin.map(|f| jour <= f).unwrap_or(true)
    }
}

#[derive(Debug, Clone)]
pub struct AffecterEleveInternat {
    pub etablissement_id: Uuid,
    pub eleve_id: Uuid,
    pub chambre_id: Uuid,
    pub lit: Option<String>,
    pub debut: NaiveDate,
    pub actor_user_id: Uuid,
}

/// Motifs de changement de chambre ou d'affectation (codés, jamais de texte libre).
pub const MOTIFS_AFFECTATION: &[&str] = &[
    "rentree",
    "demande_eleve",
    "conflit",
    "travaux",
    "sante",
    "relogement",
    "autre",
];

/// Ce que le service a lu (base, séjour, dérogations) pour que les règles restent **pures**.
#[derive(Debug, Clone, Default)]
pub struct ContexteAffectation {
    /// Régime de l'élève **à la date de début** (périodes datées du socle 2), `None` si inconnu.
    pub regime: Option<RegimeCode>,
    /// Séjour d'hébergement couvrant la date (R-IT1) : `Some(id)` si le régime l'a ouvert.
    pub sejour_id: Option<Uuid>,
    /// Plus grand nombre d'occupants simultanés de la chambre sur la période (hors le nouvel arrivant).
    pub pic_occupants: i16,
    /// L'élève a déjà une affectation ouverte.
    pub deja_affecte: bool,
    /// Sexe d'état civil de l'élève (`M` / `F`).
    pub sexe: Option<String>,
    /// La direction a dérogé au public de cette chambre pour cet élève (R-IT11).
    pub derogation_public: bool,
    /// Élèves de la chambre sous interdiction de contact avec l'arrivant, SANS dérogation (R-IT12).
    pub interdits_sans_derogation: u16,
    /// L'élève a un besoin d'accessibilité (aménagement du socle 35, ou saisie de la direction).
    pub besoin_accessibilite: bool,
}

/// Contrôles d'affectation (R-IT1, R-IT10, R-IT11, R-IT12, §8.3) — **purs**.
pub fn plan_affecter_eleve_internat(
    cmd: AffecterEleveInternat,
    chambre: &ChambreInternat,
    ctx: &ContexteAffectation,
) -> Result<AffectationInternat, DomainError> {
    if cmd.etablissement_id.is_nil() || cmd.eleve_id.is_nil() || cmd.chambre_id.is_nil() {
        return Err(DomainError::Event(
            ent_events::EventBusError::MissingEtablissement,
        ));
    }
    if chambre.etablissement_id != cmd.etablissement_id || chambre.id != cmd.chambre_id {
        return Err(DomainError::InternatChambreInvalide);
    }
    if !chambre.ouverte_au(cmd.debut) {
        return Err(DomainError::InternatChambreFermee);
    }
    if ctx.deja_affecte {
        return Err(DomainError::InternatDejaAffecte);
    }
    // R-IT1 : l'hébergement naît du régime.
    if !ctx.regime.is_some_and(RegimeCode::est_heberge) {
        return Err(DomainError::InternatRegimeRequis);
    }
    // R-IT11 : public de la chambre contre le sexe d'état civil ; dérogation de la direction.
    let public = chambre.public.ok_or(DomainError::InternatPublicIncompatible)?;
    let attendu = ctx.sexe.as_deref().and_then(PublicChambre::pour_sexe);
    if attendu != Some(public) && !ctx.derogation_public {
        return Err(DomainError::InternatPublicIncompatible);
    }
    // R-IT12 : interdiction de contact, sauf dérogation de la direction.
    if ctx.interdits_sans_derogation > 0 {
        return Err(DomainError::InternatInterdictionContact);
    }
    if ctx.besoin_accessibilite && !chambre.accessible_pmr {
        return Err(DomainError::InternatAccessibiliteRequise);
    }
    // R-IT10 : capacité (le verrou et le contrôle définitifs sont en base, migration 2820).
    if ctx.pic_occupants >= chambre.capacite {
        return Err(DomainError::InternatChambrePleine);
    }
    let lit = cmd
        .lit
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    Ok(AffectationInternat {
        id: Uuid::new_v4(),
        etablissement_id: cmd.etablissement_id,
        eleve_id: cmd.eleve_id,
        chambre_id: cmd.chambre_id,
        lit,
        lit_id: None,
        sejour_id: ctx.sejour_id,
        motif_code: None,
        debut: cmd.debut,
        fin: None,
        created_by_user_id: cmd.actor_user_id,
        created_at: Utc::now(),
    })
}

pub fn plan_clore_affectation_internat(
    mut current: AffectationInternat,
    etablissement_id: Uuid,
    fin: NaiveDate,
) -> Result<AffectationInternat, DomainError> {
    if current.etablissement_id != etablissement_id {
        return Err(DomainError::TenantMismatch);
    }
    if current.fin.is_some() {
        return Err(DomainError::InternatAffectationDejaClose);
    }
    if fin < current.debut {
        return Err(DomainError::InternatAffectationInvalide);
    }
    current.fin = Some(fin);
    Ok(current)
}

/// Soirs attendus d'un séjour (rythme versionné, R-IT4) : une case par jour de semaine ISO
/// (lundi = 0 … dimanche = 6).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rythme {
    pub dimanche: bool,
    pub lundi: bool,
    pub mardi: bool,
    pub mercredi: bool,
    pub jeudi: bool,
    pub vendredi: bool,
    pub samedi: bool,
}

impl Rythme {
    /// Rythme par défaut (S-4100) : dimanche soir à jeudi soir.
    pub const DEFAUT: Self = Self {
        dimanche: true,
        lundi: true,
        mardi: true,
        mercredi: true,
        jeudi: true,
        vendredi: false,
        samedi: false,
    };

    /// Le soir de `nuit` est-il attendu ? La nuit est la date du soir (R-IT5).
    pub fn attend(&self, nuit: NaiveDate) -> bool {
        use chrono::{Datelike, Weekday};
        match nuit.weekday() {
            Weekday::Sun => self.dimanche,
            Weekday::Mon => self.lundi,
            Weekday::Tue => self.mardi,
            Weekday::Wed => self.mercredi,
            Weekday::Thu => self.jeudi,
            Weekday::Fri => self.vendredi,
            Weekday::Sat => self.samedi,
        }
    }

    pub fn au_moins_un_soir(&self) -> bool {
        self.dimanche
            || self.lundi
            || self.mardi
            || self.mercredi
            || self.jeudi
            || self.vendredi
            || self.samedi
    }
}

/// Version de rythme vaut à partir de `du` ; la version en vigueur est la plus récente dont `du` ne
/// dépasse pas la nuit (R-IT4).
pub fn rythme_en_vigueur(versions: &[(NaiveDate, Rythme)], nuit: NaiveDate) -> Option<Rythme> {
    versions
        .iter()
        .filter(|(du, _)| *du <= nuit)
        .max_by_key(|(du, _)| *du)
        .map(|(_, r)| *r)
}

/// Marque de présence d'un fait (R-IT13). `Retard` et `PermisSortie` sont les marques héritées de 066.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PresenceInternatStatut {
    Present,
    Absent,
    PermisSortie,
    Retard,
    Excuse,
    Activite,
    Infirmerie,
    Parti,
    NonAttenduPresent,
    Vu,
}

impl PresenceInternatStatut {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Present => "present",
            Self::Absent => "absent",
            Self::PermisSortie => "permis_sortie",
            Self::Retard => "retard",
            Self::Excuse => "excuse",
            Self::Activite => "activite",
            Self::Infirmerie => "infirmerie",
            Self::Parti => "parti",
            Self::NonAttenduPresent => "non_attendu_present",
            Self::Vu => "vu",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "present" | "présent" => Some(Self::Present),
            "absent" => Some(Self::Absent),
            "permis_sortie" | "permis" | "sortie" => Some(Self::PermisSortie),
            "retard" => Some(Self::Retard),
            "excuse" | "excusé" => Some(Self::Excuse),
            "activite" | "activité" => Some(Self::Activite),
            "infirmerie" => Some(Self::Infirmerie),
            "parti" => Some(Self::Parti),
            "non_attendu_present" => Some(Self::NonAttenduPresent),
            "vu" => Some(Self::Vu),
            _ => None,
        }
    }
}

/// Motifs de correction d'un pointage de nuit passée.
pub const MOTIFS_CORRECTION: &[&str] = &[
    "oubli",
    "erreur_saisie",
    "information_tardive",
    "rattrapage",
    "autre",
];

/// Un FAIT de présence (jamais écrasé). La valeur courante est le fait le plus récent.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PresenceInternat {
    pub id: Uuid,
    pub etablissement_id: Uuid,
    pub eleve_id: Uuid,
    pub affectation_id: Uuid,
    pub jour: NaiveDate,
    pub statut: PresenceInternatStatut,
    pub note: String,
    pub motif_correction_code: Option<String>,
    pub actor_user_id: Uuid,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct PointerPresenceInternat {
    pub etablissement_id: Uuid,
    pub eleve_id: Uuid,
    /// Nuit pointée (date du soir).
    pub jour: NaiveDate,
    pub statut: String,
    pub note: String,
    pub actor_user_id: Uuid,
    /// Obligatoire pour une nuit passée (R-IT13) ; codé, jamais de texte libre seul.
    pub motif_correction_code: Option<String>,
    pub motif_correction_texte: Option<String>,
}

/// Plan d'un pointage : nuit future refusée ; nuit passée avec motif codé ; seul l'élève hébergé
/// cette nuit-là est pointable (R-IT14 amorcé, corrige DI-19).
pub fn plan_pointer_presence_internat(
    cmd: PointerPresenceInternat,
    affectation: &AffectationInternat,
    nuit_courante: NaiveDate,
) -> Result<PresenceInternat, DomainError> {
    if cmd.etablissement_id != affectation.etablissement_id
        || cmd.eleve_id != affectation.eleve_id
    {
        return Err(DomainError::TenantMismatch);
    }
    if !affectation.est_active_au(cmd.jour) {
        return Err(DomainError::InternatAffectationInvalide);
    }
    if cmd.jour > nuit_courante {
        return Err(DomainError::InternatNuitFuture);
    }
    let motif = cmd
        .motif_correction_code
        .as_deref()
        .map(str::trim)
        .filter(|m| !m.is_empty());
    if let Some(m) = motif {
        if !MOTIFS_CORRECTION.contains(&m) {
            return Err(DomainError::InternatPresenceInvalide);
        }
    }
    if cmd.jour < nuit_courante && motif.is_none() {
        return Err(DomainError::InternatMotifRequis);
    }
    let statut =
        PresenceInternatStatut::parse(&cmd.statut).ok_or(DomainError::InternatPresenceInvalide)?;
    let note = cmd.note.trim().to_string();
    if note.chars().count() > 500 {
        return Err(DomainError::InternatPresenceInvalide);
    }
    if cmd
        .motif_correction_texte
        .as_ref()
        .is_some_and(|t| t.chars().count() > 300)
    {
        return Err(DomainError::InternatPresenceInvalide);
    }
    Ok(PresenceInternat {
        id: Uuid::new_v4(),
        etablissement_id: cmd.etablissement_id,
        eleve_id: cmd.eleve_id,
        affectation_id: affectation.id,
        jour: cmd.jour,
        statut,
        note,
        motif_correction_code: motif.map(str::to_string),
        actor_user_id: cmd.actor_user_id,
        created_at: Utc::now(),
    })
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SuiviInternat {
    pub id: Uuid,
    pub etablissement_id: Uuid,
    pub eleve_id: Uuid,
    pub jour: NaiveDate,
    pub texte: String,
    pub visible_famille: bool,
    pub created_by_user_id: Uuid,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct CreerSuiviInternat {
    pub etablissement_id: Uuid,
    pub eleve_id: Uuid,
    pub jour: NaiveDate,
    pub texte: String,
    /// Ignoré (RI-09) : plus de case « visible famille » à la saisie, la publication passe par le
    /// bilan de la semaine (lot I7). Conservé pour la compatibilité des appelants.
    pub visible_famille: bool,
    pub actor_user_id: Uuid,
}

/// Longueur maximale d'un suivi (RI-09, R-IT7).
pub const SUIVI_MAX_CARACTERES: usize = 300;

/// Mots de santé reconnus (R-IT6) : détection **indicative**, jamais une décision. Sert à avertir
/// l'auteur (« pas de santé dans l'internat ») ; la santé relève du module 34.
pub fn mots_de_sante_detectes(texte: &str) -> bool {
    let t: String = texte
        .to_lowercase()
        .chars()
        .map(|c| match c {
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'à' | 'â' | 'ä' => 'a',
            'î' | 'ï' => 'i',
            'ô' | 'ö' => 'o',
            'ù' | 'û' | 'ü' => 'u',
            'ç' => 'c',
            other => other,
        })
        .collect();
    const RACINES: &[&str] = &[
        "allerg",
        "medicament",
        "traitement",
        "diagnostic",
        "asthm",
        "diabet",
        "epilep",
        "insulin",
        "ordonnance",
        "ventoline",
        "anaphyla",
        "pathologie",
        "maladie",
        "prescri",
    ];
    RACINES.iter().any(|r| t.contains(r))
        || t.split(|c: char| !c.is_alphanumeric()).any(|m| m == "pai")
}

pub fn plan_creer_suivi_internat(cmd: CreerSuiviInternat) -> Result<SuiviInternat, DomainError> {
    if cmd.etablissement_id.is_nil() || cmd.eleve_id.is_nil() {
        return Err(DomainError::Event(
            ent_events::EventBusError::MissingEtablissement,
        ));
    }
    let texte = cmd.texte.trim().to_string();
    if texte.is_empty() || texte.chars().count() > SUIVI_MAX_CARACTERES {
        return Err(DomainError::InternatSuiviInvalide);
    }
    Ok(SuiviInternat {
        id: Uuid::new_v4(),
        etablissement_id: cmd.etablissement_id,
        eleve_id: cmd.eleve_id,
        jour: cmd.jour,
        texte,
        // RI-09 : jamais visible de la famille à la saisie.
        visible_famille: false,
        created_by_user_id: cmd.actor_user_id,
        created_at: Utc::now(),
    })
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RecapInternatEleve {
    pub affectation: Option<AffectationInternat>,
    pub chambre: Option<ChambreInternat>,
    pub presence_du_jour: Option<PresenceInternat>,
    pub suivis_famille: Vec<SuiviInternat>,
}

/// Plus grand nombre d'occupants simultanés d'une chambre à partir de `depuis`, sur des
/// affectations `(debut, fin)` (inclusives, `fin` absente = ouverte). Fonction pure (R-IT10).
pub fn pic_occupation(affectations: &[(NaiveDate, Option<NaiveDate>)], depuis: NaiveDate) -> i16 {
    let mut points: Vec<NaiveDate> = vec![depuis];
    points.extend(affectations.iter().map(|(d, _)| *d).filter(|d| *d > depuis));
    points
        .iter()
        .map(|p| {
            affectations
                .iter()
                .filter(|(d, f)| *d <= *p && f.map(|f| f >= *p).unwrap_or(true))
                .count() as i16
        })
        .max()
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn d(y: i32, m: u32, j: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, j).unwrap()
    }

    fn chambre(cap: i16, public: PublicChambre) -> ChambreInternat {
        ChambreInternat {
            id: Uuid::new_v4(),
            etablissement_id: Uuid::new_v4(),
            batiment: "A".into(),
            etage: 1,
            code: "A12".into(),
            capacite: cap,
            genre: ChambreGenre::Garcons,
            public: Some(public),
            zone_id: None,
            accessible_pmr: false,
            douche: false,
            lavabo: false,
            bureau: false,
            etat: EtatChambre::Active,
            ferme_du: None,
            ferme_au: None,
            notes_entretien: None,
            actif: true,
            created_by_user_id: Uuid::new_v4(),
            created_at: Utc::now(),
        }
    }

    fn cmd(ch: &ChambreInternat) -> AffecterEleveInternat {
        AffecterEleveInternat {
            etablissement_id: ch.etablissement_id,
            eleve_id: Uuid::new_v4(),
            chambre_id: ch.id,
            lit: None,
            debut: d(2026, 9, 1),
            actor_user_id: Uuid::new_v4(),
        }
    }

    fn ctx_ok() -> ContexteAffectation {
        ContexteAffectation {
            regime: Some(RegimeCode::Interne),
            sexe: Some("M".into()),
            ..Default::default()
        }
    }

    #[test]
    fn regime_unique_trois_vocabulaires() {
        for (texte, attendu) in [
            ("INT", RegimeCode::Interne),
            ("interne", RegimeCode::Interne),
            (" Internat ", RegimeCode::Interne),
            ("DP", RegimeCode::DemiPensionnaire),
            ("demi_pension", RegimeCode::DemiPensionnaire),
            ("1/2P", RegimeCode::DemiPensionnaire),
            ("EXT", RegimeCode::Externe),
            ("externe", RegimeCode::Externe),
            ("interne externé", RegimeCode::InterneExterne),
        ] {
            assert_eq!(RegimeCode::depuis_texte(texte), Some(attendu), "{texte}");
        }
        assert_eq!(RegimeCode::depuis_texte("???"), None);
        assert_eq!(RegimeCode::depuis_texte(""), None);
        assert!(RegimeCode::Interne.est_heberge());
        assert!(!RegimeCode::InterneExterne.est_heberge());
        assert!(!RegimeCode::DemiPensionnaire.est_heberge());
    }

    #[test]
    fn affectation_exige_regime_interne_r_it1() {
        let ch = chambre(2, PublicChambre::Garcons);
        for regime in [
            Some(RegimeCode::DemiPensionnaire),
            Some(RegimeCode::Externe),
            Some(RegimeCode::InterneExterne),
            None,
        ] {
            let ctx = ContexteAffectation { regime, ..ctx_ok() };
            assert_eq!(
                plan_affecter_eleve_internat(cmd(&ch), &ch, &ctx).unwrap_err(),
                DomainError::InternatRegimeRequis
            );
        }
    }

    #[test]
    fn affectation_public_de_la_chambre_r_it11() {
        let ch = chambre(2, PublicChambre::Filles);
        // Garçon dans une chambre de filles : refusé ; avec dérogation : accepté.
        assert_eq!(
            plan_affecter_eleve_internat(cmd(&ch), &ch, &ctx_ok()).unwrap_err(),
            DomainError::InternatPublicIncompatible
        );
        let derog = ContexteAffectation { derogation_public: true, ..ctx_ok() };
        assert!(plan_affecter_eleve_internat(cmd(&ch), &ch, &derog).is_ok());
        // Sexe inconnu : pas de devinette.
        let inconnu = ContexteAffectation { sexe: None, ..ctx_ok() };
        assert_eq!(
            plan_affecter_eleve_internat(cmd(&ch), &ch, &inconnu).unwrap_err(),
            DomainError::InternatPublicIncompatible
        );
        // Ancienne chambre « mixte » sans public : refus tant qu'elle n'est pas reclassée.
        let mut mixte = chambre(2, PublicChambre::Garcons);
        mixte.public = None;
        assert_eq!(
            plan_affecter_eleve_internat(cmd(&mixte), &mixte, &ctx_ok()).unwrap_err(),
            DomainError::InternatPublicIncompatible
        );
    }

    #[test]
    fn affectation_interdiction_contact_accessibilite_capacite() {
        let ch = chambre(1, PublicChambre::Garcons);
        let interdit = ContexteAffectation { interdits_sans_derogation: 1, ..ctx_ok() };
        assert_eq!(
            plan_affecter_eleve_internat(cmd(&ch), &ch, &interdit).unwrap_err(),
            DomainError::InternatInterdictionContact
        );
        let pmr = ContexteAffectation { besoin_accessibilite: true, ..ctx_ok() };
        assert_eq!(
            plan_affecter_eleve_internat(cmd(&ch), &ch, &pmr).unwrap_err(),
            DomainError::InternatAccessibiliteRequise
        );
        let plein = ContexteAffectation { pic_occupants: 1, ..ctx_ok() };
        assert_eq!(
            plan_affecter_eleve_internat(cmd(&ch), &ch, &plein).unwrap_err(),
            DomainError::InternatChambrePleine
        );
        let deja = ContexteAffectation { deja_affecte: true, ..ctx_ok() };
        assert_eq!(
            plan_affecter_eleve_internat(cmd(&ch), &ch, &deja).unwrap_err(),
            DomainError::InternatDejaAffecte
        );
        let ok = plan_affecter_eleve_internat(
            AffecterEleveInternat { lit: Some(" lit 2 ".into()), ..cmd(&ch) },
            &ch,
            &ctx_ok(),
        )
        .unwrap();
        assert_eq!(ok.lit.as_deref(), Some("lit 2"));
        assert!(ok.est_active_au(d(2026, 10, 1)));
    }

    #[test]
    fn chambre_fermee_refuse_les_affectations() {
        let mut ch = chambre(2, PublicChambre::Garcons);
        ch.etat = EtatChambre::Fermee;
        assert_eq!(
            plan_affecter_eleve_internat(cmd(&ch), &ch, &ctx_ok()).unwrap_err(),
            DomainError::InternatChambreFermee
        );
        // Fermeture datée : refus seulement dans la fenêtre.
        let mut ch2 = chambre(2, PublicChambre::Garcons);
        ch2.ferme_du = Some(d(2026, 10, 1));
        ch2.ferme_au = Some(d(2026, 10, 31));
        assert!(ch2.ouverte_au(d(2026, 9, 30)));
        assert!(!ch2.ouverte_au(d(2026, 10, 15)));
        assert!(ch2.ouverte_au(d(2026, 11, 1)));
    }

    #[test]
    fn chambre_mixte_refusee_a_la_creation() {
        let mk = |genre: &str, cap: i16| CreerChambreInternat {
            etablissement_id: Uuid::new_v4(),
            batiment: "A".into(),
            etage: 0,
            code: "1".into(),
            capacite: cap,
            genre: genre.into(),
            actor_user_id: Uuid::new_v4(),
        };
        assert_eq!(
            plan_creer_chambre_internat(mk("mixte", 2)).unwrap_err(),
            DomainError::InternatChambreInvalide
        );
        assert!(plan_creer_chambre_internat(mk("filles", 2)).is_ok());
        assert_eq!(
            plan_creer_chambre_internat(mk("garcons", 9)).unwrap_err(),
            DomainError::InternatChambreInvalide
        );
    }

    #[test]
    fn modifier_chambre_refuse_sous_l_occupation() {
        let ch = chambre(4, PublicChambre::Garcons);
        let reduire = |cap| ModifierChambreInternat { capacite: Some(cap), ..Default::default() };
        assert_eq!(
            plan_modifier_chambre_internat(&ch, &reduire(2), 3, true).unwrap_err(),
            DomainError::InternatCapaciteSousOccupation
        );
        assert_eq!(
            plan_modifier_chambre_internat(&ch, &reduire(3), 3, true).unwrap().capacite,
            3
        );
        let fermer = ModifierChambreInternat { etat: Some(EtatChambre::Fermee), ..Default::default() };
        assert_eq!(
            plan_modifier_chambre_internat(&ch, &fermer, 1, true).unwrap_err(),
            DomainError::InternatChambreOccupee
        );
        assert_eq!(
            plan_modifier_chambre_internat(&ch, &fermer, 0, false).unwrap().etat,
            EtatChambre::Fermee
        );
        let public = ModifierChambreInternat { public: Some(PublicChambre::Filles), ..Default::default() };
        assert_eq!(
            plan_modifier_chambre_internat(&ch, &public, 1, true).unwrap_err(),
            DomainError::InternatChambreOccupee
        );
    }

    fn affectation(etab: Uuid, eleve: Uuid) -> AffectationInternat {
        AffectationInternat {
            id: Uuid::new_v4(),
            etablissement_id: etab,
            eleve_id: eleve,
            chambre_id: Uuid::new_v4(),
            lit: None,
            lit_id: None,
            sejour_id: None,
            motif_code: None,
            debut: d(2026, 9, 1),
            fin: Some(d(2026, 9, 30)),
            created_by_user_id: Uuid::new_v4(),
            created_at: Utc::now(),
        }
    }

    fn pointage(etab: Uuid, eleve: Uuid, jour: NaiveDate, motif: Option<&str>) -> PointerPresenceInternat {
        PointerPresenceInternat {
            etablissement_id: etab,
            eleve_id: eleve,
            jour,
            statut: "present".into(),
            note: String::new(),
            actor_user_id: Uuid::new_v4(),
            motif_correction_code: motif.map(str::to_string),
            motif_correction_texte: None,
        }
    }

    #[test]
    fn presence_refuse_hors_affectation() {
        let (etab, eleve) = (Uuid::new_v4(), Uuid::new_v4());
        let aff = affectation(etab, eleve);
        let err = plan_pointer_presence_internat(
            pointage(etab, eleve, d(2026, 10, 5), None),
            &aff,
            d(2026, 10, 5),
        )
        .unwrap_err();
        assert_eq!(err, DomainError::InternatAffectationInvalide);
    }

    #[test]
    fn presence_nuit_future_refusee_nuit_passee_avec_motif() {
        let (etab, eleve) = (Uuid::new_v4(), Uuid::new_v4());
        let aff = affectation(etab, eleve);
        let courante = d(2026, 9, 10);
        assert!(plan_pointer_presence_internat(pointage(etab, eleve, courante, None), &aff, courante).is_ok());
        assert_eq!(
            plan_pointer_presence_internat(pointage(etab, eleve, d(2026, 9, 11), None), &aff, courante)
                .unwrap_err(),
            DomainError::InternatNuitFuture
        );
        assert_eq!(
            plan_pointer_presence_internat(pointage(etab, eleve, d(2026, 9, 9), None), &aff, courante)
                .unwrap_err(),
            DomainError::InternatMotifRequis
        );
        assert!(plan_pointer_presence_internat(
            pointage(etab, eleve, d(2026, 9, 9), Some("oubli")),
            &aff,
            courante
        )
        .is_ok());
        assert_eq!(
            plan_pointer_presence_internat(
                pointage(etab, eleve, d(2026, 9, 9), Some("n_importe_quoi")),
                &aff,
                courante
            )
            .unwrap_err(),
            DomainError::InternatPresenceInvalide
        );
    }

    #[test]
    fn nuit_paris_suit_le_fuseau_et_les_changements_d_heure() {
        let utc = |y, m, j, h, mi| Utc.with_ymd_and_hms(y, m, j, h, mi, 0).unwrap();
        // Hiver (UTC+1) : 23 h 30 UTC le 10 = 00 h 30 le 11 à Paris → nuit du 10.
        assert_eq!(nuit_paris(utc(2026, 12, 10, 23, 30)), d(2026, 12, 10));
        // 12 h 00 Paris le 11 (11 h UTC) ouvre la nuit du 11.
        assert_eq!(nuit_paris(utc(2026, 12, 11, 11, 0)), d(2026, 12, 11));
        assert_eq!(nuit_paris(utc(2026, 12, 11, 10, 59)), d(2026, 12, 10));
        // Passage à l'heure d'été 2027 (dimanche 28 mars, 01 h UTC) : 01 h 30 UTC = 03 h 30 CEST
        // → toujours la nuit du samedi 27 ; 13 h Paris (11 h UTC) → nuit du dimanche 28.
        assert_eq!(nuit_paris(utc(2027, 3, 28, 0, 30)), d(2027, 3, 27));
        assert_eq!(nuit_paris(utc(2027, 3, 28, 1, 30)), d(2027, 3, 27));
        assert_eq!(nuit_paris(utc(2027, 3, 28, 9, 59)), d(2027, 3, 27));
        assert_eq!(nuit_paris(utc(2027, 3, 28, 10, 0)), d(2027, 3, 28));
        // Retour à l'heure d'hiver 2027 (dimanche 31 octobre, 01 h UTC).
        assert_eq!(nuit_paris(utc(2027, 10, 31, 0, 30)), d(2027, 10, 30));
        assert_eq!(nuit_paris(utc(2027, 10, 31, 1, 30)), d(2027, 10, 30));
        assert_eq!(nuit_paris(utc(2027, 10, 31, 11, 0)), d(2027, 10, 31));
    }

    #[test]
    fn rythme_versionne_et_soirs_attendus() {
        let v1 = Rythme::DEFAUT;
        let v2 = Rythme { mercredi: false, ..Rythme::DEFAUT };
        let versions = [(d(2026, 9, 1), v1), (d(2027, 2, 3), v2)];
        // Mercredi 20 janvier 2027 : version 1 (attendu) ; mercredi 10 février : version 2 (rentre).
        assert!(rythme_en_vigueur(&versions, d(2027, 1, 20)).unwrap().attend(d(2027, 1, 20)));
        assert!(!rythme_en_vigueur(&versions, d(2027, 2, 10)).unwrap().attend(d(2027, 2, 10)));
        assert!(rythme_en_vigueur(&versions, d(2026, 8, 31)).is_none());
        // Dimanche soir attendu, vendredi non (S-4100).
        assert!(Rythme::DEFAUT.attend(d(2026, 11, 8)));
        assert!(!Rythme::DEFAUT.attend(d(2026, 11, 13)));
    }

    #[test]
    fn pic_occupation_sur_la_periode() {
        let a = vec![
            (d(2026, 9, 1), Some(d(2026, 9, 30))),
            (d(2026, 10, 5), None),
        ];
        assert_eq!(pic_occupation(&a, d(2026, 9, 10)), 1);
        // Une arrivée le 5 octobre alors que l'autre est partie le 30 septembre : jamais 2.
        assert_eq!(pic_occupation(&a, d(2026, 9, 1)), 1);
        let b = vec![
            (d(2026, 9, 1), None),
            (d(2026, 9, 15), None),
        ];
        assert_eq!(pic_occupation(&b, d(2026, 9, 1)), 2);
    }

    #[test]
    fn suivi_borne_300_pas_de_visibilite_famille_et_sante_signalee() {
        let mk = |texte: &str, visible| CreerSuiviInternat {
            etablissement_id: Uuid::new_v4(),
            eleve_id: Uuid::new_v4(),
            jour: d(2026, 9, 1),
            texte: texte.into(),
            visible_famille: visible,
            actor_user_id: Uuid::new_v4(),
        };
        assert!(!plan_creer_suivi_internat(mk("Calme ce soir", true)).unwrap().visible_famille);
        assert!(plan_creer_suivi_internat(mk(&"x".repeat(300), false)).is_ok());
        assert_eq!(
            plan_creer_suivi_internat(mk(&"x".repeat(301), false)).unwrap_err(),
            DomainError::InternatSuiviInvalide
        );
        assert!(mots_de_sante_detectes("Allergie aux arachides"));
        assert!(mots_de_sante_detectes("a pris son TRAITEMENT"));
        assert!(mots_de_sante_detectes("voir le PAI"));
        assert!(mots_de_sante_detectes("crise d'épilepsie"));
        assert!(!mots_de_sante_detectes("S'est bien intégré, rangé sa chambre"));
        assert!(!mots_de_sante_detectes("Il pleut, il pait calmement"));
    }
}
