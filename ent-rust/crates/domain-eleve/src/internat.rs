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
        if !(1..=CAPACITE_MAX_CHAMBRE).contains(&cap) {
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

/// Lettre de base d'une lettre latine étendue **en minuscule** (Latin étendu B, U+0180–024F, et Latin étendu
/// additionnel, U+1E00–1EFF) : barrée, à crochet, à anneau, à point suscrit ou souscrit, à accent empilé…
/// Table écrite à la main par plages (aucune dépendance de plus, D-062) ; une lettre non listée reste
/// telle quelle. Les digrammes (« ǆ », « ǉ », « ǌ », « ǳ ») et « æ » se déploient.
fn base_etendue(c: char) -> Option<&'static str> {
    let u = c as u32;
    let plages: &[(u32, u32, &str)] = &[
        // Latin étendu B.
        (0x0180, 0x0183, "b"), (0x0187, 0x0188, "c"), (0x0189, 0x018C, "d"), (0x0191, 0x0192, "f"),
        (0x0193, 0x0193, "g"), (0x0197, 0x0197, "i"), (0x0198, 0x0199, "k"), (0x019A, 0x019A, "l"),
        (0x019D, 0x019E, "n"), (0x019F, 0x01A1, "o"), (0x01A4, 0x01A5, "p"), (0x01AB, 0x01AE, "t"),
        (0x01AF, 0x01B0, "u"), (0x01B2, 0x01B2, "v"), (0x01B3, 0x01B4, "y"), (0x01B5, 0x01B6, "z"),
        (0x01C4, 0x01C6, "dz"), (0x01C7, 0x01C9, "lj"), (0x01CA, 0x01CC, "nj"), (0x01CD, 0x01CE, "a"),
        (0x01CF, 0x01D0, "i"), (0x01D1, 0x01D2, "o"), (0x01D3, 0x01DC, "u"), (0x01DE, 0x01E1, "a"),
        (0x01E2, 0x01E3, "ae"), (0x01E4, 0x01E7, "g"), (0x01E8, 0x01E9, "k"), (0x01EA, 0x01ED, "o"),
        (0x01F0, 0x01F0, "j"), (0x01F1, 0x01F3, "dz"), (0x01F4, 0x01F5, "g"), (0x01F8, 0x01F9, "n"),
        (0x01FA, 0x01FB, "a"), (0x01FC, 0x01FD, "ae"), (0x01FE, 0x01FF, "o"), (0x0200, 0x0203, "a"),
        (0x0204, 0x0207, "e"), (0x0208, 0x020B, "i"), (0x020C, 0x020F, "o"), (0x0210, 0x0213, "r"),
        (0x0214, 0x0217, "u"), (0x0218, 0x0219, "s"), (0x021A, 0x021B, "t"), (0x021E, 0x021F, "h"),
        (0x0220, 0x0220, "n"), (0x0224, 0x0225, "z"), (0x0226, 0x0227, "a"), (0x0228, 0x0229, "e"),
        (0x022A, 0x0231, "o"), (0x0232, 0x0233, "y"), (0x0234, 0x0234, "l"), (0x0235, 0x0235, "n"),
        (0x0236, 0x0236, "t"), (0x0243, 0x0243, "b"), (0x0244, 0x0244, "u"), (0x0246, 0x0247, "e"),
        (0x0248, 0x0249, "j"), (0x024A, 0x024B, "q"), (0x024C, 0x024D, "r"), (0x024E, 0x024F, "y"),
        // Latin étendu additionnel.
        (0x1E00, 0x1E01, "a"), (0x1E02, 0x1E07, "b"), (0x1E08, 0x1E09, "c"), (0x1E0A, 0x1E13, "d"),
        (0x1E14, 0x1E1D, "e"), (0x1E1E, 0x1E1F, "f"), (0x1E20, 0x1E21, "g"), (0x1E22, 0x1E2B, "h"),
        (0x1E2C, 0x1E2F, "i"), (0x1E30, 0x1E35, "k"), (0x1E36, 0x1E3D, "l"), (0x1E3E, 0x1E43, "m"),
        (0x1E44, 0x1E4B, "n"), (0x1E4C, 0x1E53, "o"), (0x1E54, 0x1E57, "p"), (0x1E58, 0x1E5F, "r"),
        (0x1E60, 0x1E69, "s"), (0x1E6A, 0x1E71, "t"), (0x1E72, 0x1E7B, "u"), (0x1E7C, 0x1E7F, "v"),
        (0x1E80, 0x1E89, "w"), (0x1E8A, 0x1E8D, "x"), (0x1E8E, 0x1E8F, "y"), (0x1E90, 0x1E95, "z"),
        (0x1E96, 0x1E96, "h"), (0x1E97, 0x1E97, "t"), (0x1E98, 0x1E98, "w"), (0x1E99, 0x1E99, "y"),
        (0x1E9A, 0x1E9A, "a"), (0x1E9B, 0x1E9D, "s"), (0x1E9E, 0x1E9E, "ss"), (0x1EA0, 0x1EB7, "a"),
        (0x1EB8, 0x1EC7, "e"), (0x1EC8, 0x1ECB, "i"), (0x1ECC, 0x1EE3, "o"), (0x1EE4, 0x1EF1, "u"),
        (0x1EF2, 0x1EF9, "y"),
    ];
    plages.iter().find(|(d, f, _)| (*d..=*f).contains(&u)).map(|(_, _, b)| *b)
}

/// Marques combinantes et caractères invisibles qu'on retire du texte (avant **et** après la mise en minuscule).
fn a_ignorer(c: char) -> bool {
    matches!(
        c,
        '\u{0300}'..='\u{036F}'
            | '\u{1AB0}'..='\u{1AFF}'
            | '\u{1DC0}'..='\u{1DFF}'
            | '\u{20D0}'..='\u{20FF}'
            | '\u{FE20}'..='\u{FE2F}'
            | '\u{00AD}'
            | '\u{180E}'
            | '\u{200B}'..='\u{200D}'
            | '\u{2060}'..='\u{2064}'
            | '\u{FEFF}'
    )
}

/// Texte ramené en minuscules **sans diacritiques** (M1, F-4) : une **aide**, pas l'équivalent d'une
/// normalisation Unicode NFKD (aucune dépendance de plus, D-062 : la table est écrite à la main et ne
/// couvre donc pas tout Unicode). Couvre :
/// * les marques combinantes (texte décomposé « e » + U+0301) des blocs U+0300–036F, U+1AB0–1AFF,
///   U+1DC0–1DFF, U+20D0–20FF et U+FE20–FE2F ;
/// * les lettres latines accentuées du Latin-1, du Latin étendu A, du Latin étendu B (U+0180–024F) et du Latin
///   étendu additionnel (U+1E00–1EFF) ;
/// * les ligatures U+FB00–FB06 (« ﬁèvre ») et « œ », « æ », « ß » ;
/// * les formes pleine chasse (« ＦＩＥＶＲＥ ») ;
/// * les caractères invisibles qu'on glisserait au milieu d'un mot pour tromper la détection : trait d'union
///   conditionnel, espaces de largeur nulle, U+2060–2064 (opérateurs invisibles), U+180E, U+FEFF.
fn replier_en_ascii(texte: &str) -> String {
    let mut sortie = String::with_capacity(texte.len());
    for c in texte.chars() {
        let c = if ('\u{FF01}'..='\u{FF5E}').contains(&c) {
            char::from_u32(c as u32 - 0xFEE0).unwrap_or(c)
        } else {
            c
        };
        match c {
            c if a_ignorer(c) => {}
            '\u{FB00}' => sortie.push_str("ff"),
            '\u{FB01}' => sortie.push_str("fi"),
            '\u{FB02}' => sortie.push_str("fl"),
            '\u{FB03}' => sortie.push_str("ffi"),
            '\u{FB04}' => sortie.push_str("ffl"),
            '\u{FB05}' | '\u{FB06}' => sortie.push_str("st"),
            _ => {
                for l in c.to_lowercase() {
                    match l {
                        // La mise en minuscule peut elle-même produire une marque (« İ » → « i » + U+0307) : on
                        // refiltre APRÈS, avec le même ensemble que ci-dessus, pas seulement U+0300–036F.
                        l if a_ignorer(l) => {}
                        'œ' => sortie.push_str("oe"),
                        'æ' => sortie.push_str("ae"),
                        'ß' => sortie.push_str("ss"),
                        'à'..='å' | 'ā' | 'ă' | 'ą' => sortie.push('a'),
                        'ç' | 'ć' | 'ĉ' | 'ċ' | 'č' => sortie.push('c'),
                        'ð' | 'ď' | 'đ' => sortie.push('d'),
                        'è'..='ë' | 'ē' | 'ĕ' | 'ė' | 'ę' | 'ě' => sortie.push('e'),
                        'ĝ' | 'ğ' | 'ġ' | 'ģ' => sortie.push('g'),
                        'ĥ' | 'ħ' => sortie.push('h'),
                        'ì'..='ï' | 'ĩ' | 'ī' | 'ĭ' | 'į' | 'ı' => sortie.push('i'),
                        'ĵ' => sortie.push('j'),
                        'ķ' => sortie.push('k'),
                        'ĺ' | 'ļ' | 'ľ' | 'ŀ' | 'ł' => sortie.push('l'),
                        'ñ' | 'ń' | 'ņ' | 'ň' => sortie.push('n'),
                        'ò'..='ö' | 'ø' | 'ō' | 'ŏ' | 'ő' => sortie.push('o'),
                        'ŕ' | 'ŗ' | 'ř' => sortie.push('r'),
                        'ś' | 'ŝ' | 'ş' | 'š' => sortie.push('s'),
                        'ţ' | 'ť' | 'ŧ' => sortie.push('t'),
                        'ù'..='ü' | 'ũ' | 'ū' | 'ŭ' | 'ů' | 'ű' | 'ų' => sortie.push('u'),
                        'ŵ' => sortie.push('w'),
                        'ý' | 'ÿ' | 'ŷ' => sortie.push('y'),
                        'ź' | 'ż' | 'ž' => sortie.push('z'),
                        autre => match base_etendue(autre) {
                            Some(base) => sortie.push_str(base),
                            None => sortie.push(autre),
                        },
                    }
                }
            }
        }
    }
    sortie
}

/// Mots de santé reconnus (R-IT6) : détection **indicative**, jamais une décision. Sert à avertir
/// l'auteur (« pas de santé dans l'internat ») ; la santé relève du module 34.
///
/// C'est **une aide, pas une garantie** : une liste de racines (le texte est ramené en minuscules, sans
/// diacritiques ni « œ », par `replier_en_ascii` avant comparaison) ne reconnaît ni une périphrase, ni une faute de
/// frappe, ni une abréviation. La protection réelle reste le code de motif sans texte libre, la
/// confidentialité et la relecture humaine. Les faux positifs sont assumés (mieux vaut refuser un
/// texte inoffensif que laisser passer un diagnostic).
pub fn mots_de_sante_detectes(texte: &str) -> bool {
    let t = replier_en_ascii(texte);
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
        "malade",
        "prescri",
        "fievr",
        "crise",
        "vomi",
        "migrain",
        "depress",
        "suicid",
        "psy",
        "tdah",
        "angoiss",
        "anxie",
        "grippe",
        "gastro",
        "covid",
        "hospital",
        "fractur",
        "bless",
        "douleur",
        "nause",
        "plaie",
        "automutil",
        "anorex",
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
        assert!(mots_de_sante_detectes("FIÈVRE cette nuit"));
        assert!(mots_de_sante_detectes("fievre"));
        assert!(mots_de_sante_detectes("vomissements"));
        assert!(mots_de_sante_detectes("migraine"));
        assert!(mots_de_sante_detectes("signes de dépression"));
        assert!(mots_de_sante_detectes("idées suicidaires"));
        assert!(mots_de_sante_detectes("rdv psy"));
        assert!(mots_de_sante_detectes("TDAH"));
        assert!(mots_de_sante_detectes("diabète"));
        // M1 : racines ajoutées, textes décomposés (NFD), « œ », pleine chasse, caractères invisibles.
        for t in [
            "hospitalisé", "fracture du bras", "blessé au pied", "douleurs", "nausées", "une plaie",
            "automutilation", "anorexie", "fie\u{0300}vre", "e\u{0301}pilepsie", "ＦＩＥＶＲＥ",
            "fi\u{00AD}evre", "fi\u{200B}evre", "DÉPRESSION", "dépre\u{0301}ssion",
        ] {
            assert!(mots_de_sante_detectes(t), "{t}");
        }
        assert_eq!(replier_en_ascii("Œuvre ÆTHER façade ñ"), "oeuvre aether facade n");
        assert!(!mots_de_sante_detectes("S'est bien intégré, rangé sa chambre"));
        assert!(!mots_de_sante_detectes("maladroit mais gentil"));
        assert!(!mots_de_sante_detectes("Il pleut, il pait calmement"));
    }

    /// F-4 : ligatures, Latin étendu B et additionnel, marques combinantes hors U+0300–036F, invisibles
    /// U+2061–2064 et U+180E.
    #[test]
    fn repli_ligatures_latin_etendu_marques_et_invisibles() {
        // Ligatures U+FB00–FB06.
        assert_eq!(replier_en_ascii("\u{FB00}\u{FB01}\u{FB02}\u{FB03}\u{FB04}\u{FB05}\u{FB06}"), "fffiflffifflstst");
        assert!(mots_de_sante_detectes("\u{FB01}evre"), "ﬁevre");
        assert!(mots_de_sante_detectes("\u{FB01}\u{00E8}vre"), "ﬁèvre");
        // Latin étendu B (U+0180–024F) : « ǎ », « ȇ », « ơ », « ƫ », « ǆ ».
        assert_eq!(replier_en_ascii("ǍȆƠƫǅ"), "aeotdz");
        assert!(mots_de_sante_detectes("diab\u{0229}te"), "ȩ proche de e");
        assert!(mots_de_sante_detectes("m\u{0229}dicament"), "ȩ → e");
        assert!(mots_de_sante_detectes("fi\u{1EC7}vre"), "ệ du Latin étendu additionnel");
        assert!(mots_de_sante_detectes("\u{1E1E}IEVRE"), "Ḟ → f");
        assert_eq!(replier_en_ascii("ḂḃẠạỆệṦṧẞ"), "bbaaeessss");
        // Marques combinantes hors U+0300–036F.
        for marque in ['\u{1AB0}', '\u{1AC0}', '\u{1DC0}', '\u{1DFF}', '\u{20D0}', '\u{20FF}', '\u{FE20}', '\u{FE2F}'] {
            let t = format!("fi{marque}evre");
            assert!(mots_de_sante_detectes(&t), "marque {marque:?}");
        }
        // Invisibles U+2061–2064 et U+180E.
        for invisible in ['\u{2061}', '\u{2062}', '\u{2063}', '\u{2064}', '\u{180E}'] {
            let t = format!("fi{invisible}evre");
            assert!(mots_de_sante_detectes(&t), "invisible {invisible:?}");
        }
        // La mise en minuscule produit elle-même une marque (« İ » → « i » + U+0307) : refiltrée après.
        assert_eq!(replier_en_ascii("F\u{0130}EVRE"), "fievre");
        assert!(mots_de_sante_detectes("F\u{0130}EVRE"), "FİEVRE");
        assert!(mots_de_sante_detectes("M\u{00C9}D\u{0130}CAMENT"), "MÉDİCAMENT");
        // Rien de plus : un texte sans rapport reste libre, y compris avec des lettres étendues.
        assert!(!mots_de_sante_detectes("Ơn a rangé sa chambre ǎ ȇ ṧ ệ"));
    }
}
