//! Grille repas / suivi + bourses — dossier élève (cantine & facturation).
//!
//! Régime (DP/EXT/INT) = projection haute ; la grille = cases jour × activité
//! (midi, soir, étude…). Bourse acceptée + procuration → déductible facture.

use crate::DomainError;
use chrono::{Datelike, NaiveDate, Weekday};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RepasActiviteKind {
    Midi,
    Soir,
    Etude,
    Garderie,
    Autre,
}

impl RepasActiviteKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Midi => "midi",
            Self::Soir => "soir",
            Self::Etude => "etude",
            Self::Garderie => "garderie",
            Self::Autre => "autre",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "midi" => Some(Self::Midi),
            "soir" => Some(Self::Soir),
            "etude" => Some(Self::Etude),
            "garderie" => Some(Self::Garderie),
            "autre" => Some(Self::Autre),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RepasActivite {
    pub id: Uuid,
    pub etablissement_id: Uuid,
    pub code: String,
    pub libelle: String,
    pub kind: RepasActiviteKind,
    pub actif: bool,
}

#[derive(Debug, Clone)]
pub struct CreerRepasActivite {
    pub etablissement_id: Uuid,
    pub code: String,
    pub libelle: String,
    pub kind: RepasActiviteKind,
    pub actor_user_id: Option<String>,
}

/// Masque lun→dim : qui mange / suit cette activité quels jours.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct JoursMasque {
    pub lun: bool,
    pub mar: bool,
    pub mer: bool,
    pub jeu: bool,
    pub ven: bool,
    pub sam: bool,
    pub dim: bool,
}

impl JoursMasque {
    pub fn aucun(self) -> bool {
        !(self.lun || self.mar || self.mer || self.jeu || self.ven || self.sam || self.dim)
    }

    pub fn lun_ven() -> Self {
        Self {
            lun: true,
            mar: true,
            mer: true,
            jeu: true,
            ven: true,
            sam: false,
            dim: false,
        }
    }

    pub fn actif_le(self, jour: NaiveDate) -> bool {
        match jour.weekday() {
            Weekday::Mon => self.lun,
            Weekday::Tue => self.mar,
            Weekday::Wed => self.mer,
            Weekday::Thu => self.jeu,
            Weekday::Fri => self.ven,
            Weekday::Sat => self.sam,
            Weekday::Sun => self.dim,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EleveGrilleRepas {
    pub id: Uuid,
    pub etablissement_id: Uuid,
    pub eleve_id: Uuid,
    pub activite_id: Uuid,
    pub jours: JoursMasque,
    pub date_debut: NaiveDate,
    pub date_fin: Option<NaiveDate>,
}

#[derive(Debug, Clone)]
pub struct PoserGrilleRepas {
    pub etablissement_id: Uuid,
    pub eleve_id: Uuid,
    pub activite_id: Uuid,
    pub jours: JoursMasque,
    pub date_debut: NaiveDate,
    pub actor_user_id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BourseStatus {
    Demandee,
    Acceptee,
    Refusee,
    Cloturee,
}

impl BourseStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Demandee => "demandee",
            Self::Acceptee => "acceptee",
            Self::Refusee => "refusee",
            Self::Cloturee => "cloturee",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "demandee" => Some(Self::Demandee),
            "acceptee" => Some(Self::Acceptee),
            "refusee" => Some(Self::Refusee),
            "cloturee" => Some(Self::Cloturee),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BourseType {
    pub id: Uuid,
    pub etablissement_id: Uuid,
    pub code: String,
    pub libelle: String,
    pub actif: bool,
}

#[derive(Debug, Clone)]
pub struct CreerBourseType {
    pub etablissement_id: Uuid,
    pub code: String,
    pub libelle: String,
    pub actor_user_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EleveBourse {
    pub id: Uuid,
    pub etablissement_id: Uuid,
    pub eleve_id: Uuid,
    pub bourse_type_id: Uuid,
    pub echelon: String,
    /// Avec procuration = l'établissement touche → déduction facture (privé).
    pub avec_procuration: bool,
    pub status: BourseStatus,
    pub date_debut: NaiveDate,
    pub date_fin: Option<NaiveDate>,
}

#[derive(Debug, Clone)]
pub struct AffecterBourse {
    pub etablissement_id: Uuid,
    pub eleve_id: Uuid,
    pub bourse_type_id: Uuid,
    pub echelon: String,
    pub avec_procuration: bool,
    pub date_debut: NaiveDate,
    pub actor_user_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct DeciderBourse {
    pub etablissement_id: Uuid,
    pub bourse_id: Uuid,
    pub accepter: bool,
    pub actor_user_id: Option<String>,
}

pub fn plan_creer_repas_activite(cmd: CreerRepasActivite) -> Result<RepasActivite, DomainError> {
    if cmd.etablissement_id.is_nil() {
        return Err(DomainError::Event(
            ent_events::EventBusError::MissingEtablissement,
        ));
    }
    let code = cmd.code.trim().to_string();
    let libelle = cmd.libelle.trim().to_string();
    if code.is_empty() || libelle.is_empty() {
        return Err(DomainError::EmptyAttributValeur);
    }
    Ok(RepasActivite {
        id: Uuid::new_v4(),
        etablissement_id: cmd.etablissement_id,
        code,
        libelle,
        kind: cmd.kind,
        actif: true,
    })
}

pub fn plan_poser_grille_repas(cmd: PoserGrilleRepas) -> Result<EleveGrilleRepas, DomainError> {
    if cmd.etablissement_id.is_nil() {
        return Err(DomainError::Event(
            ent_events::EventBusError::MissingEtablissement,
        ));
    }
    if cmd.jours.aucun() {
        return Err(DomainError::GrilleRepasSansJour);
    }
    Ok(EleveGrilleRepas {
        id: Uuid::new_v4(),
        etablissement_id: cmd.etablissement_id,
        eleve_id: cmd.eleve_id,
        activite_id: cmd.activite_id,
        jours: cmd.jours,
        date_debut: cmd.date_debut,
        date_fin: None,
    })
}

/// La grille couvre ce jour (dates d'effet + masque semaine).
pub fn grille_couvre_jour(g: &EleveGrilleRepas, jour: NaiveDate) -> bool {
    if jour < g.date_debut {
        return false;
    }
    if g.date_fin.map(|f| jour > f).unwrap_or(false) {
        return false;
    }
    g.jours.actif_le(jour)
}

pub fn plan_creer_bourse_type(cmd: CreerBourseType) -> Result<BourseType, DomainError> {
    if cmd.etablissement_id.is_nil() {
        return Err(DomainError::Event(
            ent_events::EventBusError::MissingEtablissement,
        ));
    }
    let code = cmd.code.trim().to_string();
    let libelle = cmd.libelle.trim().to_string();
    if code.is_empty() || libelle.is_empty() {
        return Err(DomainError::EmptyAttributValeur);
    }
    Ok(BourseType {
        id: Uuid::new_v4(),
        etablissement_id: cmd.etablissement_id,
        code,
        libelle,
        actif: true,
    })
}

pub fn plan_affecter_bourse(cmd: AffecterBourse) -> Result<EleveBourse, DomainError> {
    if cmd.etablissement_id.is_nil() {
        return Err(DomainError::Event(
            ent_events::EventBusError::MissingEtablissement,
        ));
    }
    let echelon = cmd.echelon.trim().to_string();
    if echelon.is_empty() {
        return Err(DomainError::EmptyAttributValeur);
    }
    Ok(EleveBourse {
        id: Uuid::new_v4(),
        etablissement_id: cmd.etablissement_id,
        eleve_id: cmd.eleve_id,
        bourse_type_id: cmd.bourse_type_id,
        echelon,
        avec_procuration: cmd.avec_procuration,
        status: BourseStatus::Demandee,
        date_debut: cmd.date_debut,
        date_fin: None,
    })
}

pub fn plan_decider_bourse(
    current: &EleveBourse,
    cmd: DeciderBourse,
) -> Result<EleveBourse, DomainError> {
    if current.etablissement_id != cmd.etablissement_id || current.id != cmd.bourse_id {
        return Err(DomainError::NotFound);
    }
    if current.status != BourseStatus::Demandee {
        return Err(DomainError::BourseDejaTranchee);
    }
    let mut b = current.clone();
    b.status = if cmd.accepter {
        BourseStatus::Acceptee
    } else {
        BourseStatus::Refusee
    };
    Ok(b)
}

/// Bourse qui doit être déduite sur facture : acceptée + procuration + en vigueur.
pub fn bourse_deductible_facture(b: &EleveBourse, jour: NaiveDate) -> bool {
    if b.status != BourseStatus::Acceptee || !b.avec_procuration {
        return false;
    }
    if jour < b.date_debut {
        return false;
    }
    !b.date_fin.map(|f| jour > f).unwrap_or(false)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DispenseSource {
    /// Prévenance famille (portail).
    Parent,
    /// Saisie staff (accueil / vie scolaire / admini).
    Staff,
}

impl DispenseSource {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Parent => "parent",
            Self::Staff => "staff",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "parent" => Some(Self::Parent),
            "staff" => Some(Self::Staff),
            _ => None,
        }
    }
}

/// Dispense d'un jour d'activité repas (ne pas facturer / ne pas compter présent cantine).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EleveDispenseRepas {
    pub id: Uuid,
    pub etablissement_id: Uuid,
    pub eleve_id: Uuid,
    pub activite_id: Uuid,
    pub jour: NaiveDate,
    pub motif: String,
    pub source: DispenseSource,
    pub annulee: bool,
}

#[derive(Debug, Clone)]
pub struct PoserDispenseRepas {
    pub etablissement_id: Uuid,
    pub eleve_id: Uuid,
    pub activite_id: Uuid,
    pub jour: NaiveDate,
    pub motif: String,
    pub source: DispenseSource,
    pub actor_user_id: Option<String>,
}

pub fn plan_poser_dispense_repas(cmd: PoserDispenseRepas) -> Result<EleveDispenseRepas, DomainError> {
    if cmd.etablissement_id.is_nil() {
        return Err(DomainError::Event(
            ent_events::EventBusError::MissingEtablissement,
        ));
    }
    let motif = cmd.motif.trim().to_string();
    if motif.is_empty() {
        return Err(DomainError::DispenseRepasInvalide);
    }
    Ok(EleveDispenseRepas {
        id: Uuid::new_v4(),
        etablissement_id: cmd.etablissement_id,
        eleve_id: cmd.eleve_id,
        activite_id: cmd.activite_id,
        jour: cmd.jour,
        motif,
        source: cmd.source,
        annulee: false,
    })
}

pub fn plan_annuler_dispense_repas(
    current: &EleveDispenseRepas,
    etablissement_id: Uuid,
    dispense_id: Uuid,
) -> Result<EleveDispenseRepas, DomainError> {
    if current.etablissement_id != etablissement_id || current.id != dispense_id {
        return Err(DomainError::NotFound);
    }
    if current.annulee {
        return Err(DomainError::DispenseRepasInvalide);
    }
    let mut d = current.clone();
    d.annulee = true;
    Ok(d)
}

pub fn dispense_couvre(d: &EleveDispenseRepas, activite_id: Uuid, jour: NaiveDate) -> bool {
    !d.annulee && d.activite_id == activite_id && d.jour == jour
}

/// Face cantine : inscrit à une activité `kind` ce jour (hors absence / dispense — à croiser).
pub fn inscrit_activite_kind_au_jour(
    grilles: &[EleveGrilleRepas],
    activites: &[RepasActivite],
    kind: RepasActiviteKind,
    jour: NaiveDate,
) -> bool {
    inscrit_activite_kind_au_jour_avec_dispenses(grilles, activites, &[], kind, jour)
}

/// Comme `inscrit_activite_kind_au_jour` mais une dispense active exclut le jour.
pub fn inscrit_activite_kind_au_jour_avec_dispenses(
    grilles: &[EleveGrilleRepas],
    activites: &[RepasActivite],
    dispenses: &[EleveDispenseRepas],
    kind: RepasActiviteKind,
    jour: NaiveDate,
) -> bool {
    for g in grilles {
        if !grille_couvre_jour(g, jour) {
            continue;
        }
        let Some(act) = activites
            .iter()
            .find(|a| a.id == g.activite_id && a.actif && a.kind == kind)
        else {
            continue;
        };
        if dispenses
            .iter()
            .any(|d| dispense_couvre(d, act.id, jour))
        {
            continue;
        }
        return true;
    }
    false
}

/// Jours facturables sur `[debut, fin]` pour un `kind` (grille − dispenses actives).
pub fn compter_jours_facturables(
    grilles: &[EleveGrilleRepas],
    activites: &[RepasActivite],
    dispenses: &[EleveDispenseRepas],
    kind: RepasActiviteKind,
    debut: NaiveDate,
    fin: NaiveDate,
) -> Result<u32, DomainError> {
    if fin < debut {
        return Err(DomainError::DispenseRepasInvalide);
    }
    let mut n: u32 = 0;
    let mut jour = debut;
    loop {
        if inscrit_activite_kind_au_jour_avec_dispenses(grilles, activites, dispenses, kind, jour) {
            n = n.saturating_add(1);
        }
        if jour == fin {
            break;
        }
        jour = jour
            .succ_opt()
            .ok_or(DomainError::DispenseRepasInvalide)?;
    }
    Ok(n)
}

/// Ligne d'export cuisine : effectif prévu (grille − prévenances) pour un jour × activité.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VolumeRepasJour {
    pub jour: NaiveDate,
    pub activite_id: Uuid,
    pub activite_code: String,
    pub kind: RepasActiviteKind,
    pub nb_prevus: u32,
}

#[derive(Debug, Clone)]
pub struct ExporterVolumesRepas {
    pub etablissement_id: Uuid,
    pub debut: NaiveDate,
    pub fin: NaiveDate,
    pub kind: Option<RepasActiviteKind>,
}

/// Valide la fenêtre d'export volumes (début ≤ fin, plage ≤ 92 jours).
pub fn plan_exporter_volumes_repas(cmd: &ExporterVolumesRepas) -> Result<(), DomainError> {
    if cmd.etablissement_id.is_nil() {
        return Err(DomainError::Event(
            ent_events::EventBusError::MissingEtablissement,
        ));
    }
    if cmd.fin < cmd.debut {
        return Err(DomainError::DispenseRepasInvalide);
    }
    let days = (cmd.fin - cmd.debut).num_days();
    if days > 92 {
        return Err(DomainError::DispenseRepasInvalide);
    }
    Ok(())
}

/// Agrège l'effectif prévu par jour × activité (lignes avec nb_prevus > 0 uniquement).
pub fn agreger_volumes_repas(
    grilles: &[EleveGrilleRepas],
    activites: &[RepasActivite],
    dispenses: &[EleveDispenseRepas],
    debut: NaiveDate,
    fin: NaiveDate,
    kind_filtre: Option<RepasActiviteKind>,
) -> Result<Vec<VolumeRepasJour>, DomainError> {
    if fin < debut {
        return Err(DomainError::DispenseRepasInvalide);
    }
    let acts: Vec<&RepasActivite> = activites
        .iter()
        .filter(|a| a.actif)
        .filter(|a| kind_filtre.map(|k| a.kind == k).unwrap_or(true))
        .collect();
    if acts.is_empty() {
        return Ok(Vec::new());
    }
    let mut eleve_ids: Vec<Uuid> = grilles.iter().map(|g| g.eleve_id).collect();
    eleve_ids.sort_unstable();
    eleve_ids.dedup();

    let mut out = Vec::new();
    let mut jour = debut;
    loop {
        for act in &acts {
            let mut nb: u32 = 0;
            for &eleve_id in &eleve_ids {
                let prevu = grilles.iter().any(|g| {
                    g.eleve_id == eleve_id
                        && g.activite_id == act.id
                        && grille_couvre_jour(g, jour)
                        && !dispenses.iter().any(|d| {
                            d.eleve_id == eleve_id && dispense_couvre(d, act.id, jour)
                        })
                });
                if prevu {
                    nb = nb.saturating_add(1);
                }
            }
            if nb > 0 {
                out.push(VolumeRepasJour {
                    jour,
                    activite_id: act.id,
                    activite_code: act.code.clone(),
                    kind: act.kind,
                    nb_prevus: nb,
                });
            }
        }
        if jour == fin {
            break;
        }
        jour = jour
            .succ_opt()
            .ok_or(DomainError::DispenseRepasInvalide)?;
    }
    Ok(out)
}

fn csv_escape_repas(s: &str) -> String {
    if s.contains(';') || s.contains('"') || s.contains('\n') || s.contains('\r') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

/// CSV UTF-8 BOM + `;` pour cuisine / economat.
pub fn render_volumes_cuisine_csv(lignes: &[VolumeRepasJour]) -> Vec<u8> {
    let mut out = String::from("\u{FEFF}");
    out.push_str("date;activite;kind;nb_prevus\n");
    for l in lignes {
        out.push_str(&format!(
            "{};{};{};{}\n",
            l.jour.format("%Y-%m-%d"),
            csv_escape_repas(&l.activite_code),
            l.kind.as_str(),
            l.nb_prevus
        ));
    }
    out.into_bytes()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PassageSource {
    Badge,
    Manuel,
}

impl PassageSource {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Badge => "badge",
            Self::Manuel => "manuel",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "badge" => Some(Self::Badge),
            "manuel" => Some(Self::Manuel),
            _ => None,
        }
    }
}

/// Pointage repas (badge ou saisie manuelle).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PassageRepas {
    pub id: Uuid,
    pub etablissement_id: Uuid,
    pub eleve_id: Uuid,
    pub activite_id: Uuid,
    pub jour: NaiveDate,
    pub badge_lu: String,
    pub source: PassageSource,
    pub annule: bool,
}

#[derive(Debug, Clone)]
pub struct EnregistrerPassageBadge {
    pub etablissement_id: Uuid,
    pub eleve_id: Uuid,
    pub activite_id: Uuid,
    pub jour: NaiveDate,
    pub badge_lu: String,
    pub source: PassageSource,
    pub actor_user_id: Option<String>,
}

/// Enregistre un passage si l'élève est prévu (grille − dispense) ce jour pour l'activité.
pub fn plan_enregistrer_passage_badge(
    cmd: EnregistrerPassageBadge,
    grilles: &[EleveGrilleRepas],
    dispenses: &[EleveDispenseRepas],
    deja_passe: bool,
) -> Result<PassageRepas, DomainError> {
    if cmd.etablissement_id.is_nil() {
        return Err(DomainError::Event(
            ent_events::EventBusError::MissingEtablissement,
        ));
    }
    let badge = cmd.badge_lu.trim().to_string();
    if badge.is_empty() {
        return Err(DomainError::BadgeRepasInconnu);
    }
    if deja_passe {
        return Err(DomainError::PassageBadgeDejaEnregistre);
    }
    let prevu = grilles.iter().any(|g| {
        g.eleve_id == cmd.eleve_id
            && g.activite_id == cmd.activite_id
            && grille_couvre_jour(g, cmd.jour)
            && !dispenses
                .iter()
                .any(|d| d.eleve_id == cmd.eleve_id && dispense_couvre(d, cmd.activite_id, cmd.jour))
    });
    if !prevu {
        return Err(DomainError::PassageBadgeRefuse);
    }
    Ok(PassageRepas {
        id: Uuid::new_v4(),
        etablissement_id: cmd.etablissement_id,
        eleve_id: cmd.eleve_id,
        activite_id: cmd.activite_id,
        jour: cmd.jour,
        badge_lu: badge,
        source: cmd.source,
        annule: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grille_sans_jour_refusee() {
        let err = plan_poser_grille_repas(PoserGrilleRepas {
            etablissement_id: Uuid::new_v4(),
            eleve_id: Uuid::new_v4(),
            activite_id: Uuid::new_v4(),
            jours: JoursMasque::default(),
            date_debut: NaiveDate::from_ymd_opt(2026, 9, 1).unwrap(),
            actor_user_id: None,
        })
        .unwrap_err();
        assert_eq!(err, DomainError::GrilleRepasSansJour);
    }

    #[test]
    fn lun_ven_couvre_mercredi() {
        let g = plan_poser_grille_repas(PoserGrilleRepas {
            etablissement_id: Uuid::new_v4(),
            eleve_id: Uuid::new_v4(),
            activite_id: Uuid::new_v4(),
            jours: JoursMasque::lun_ven(),
            date_debut: NaiveDate::from_ymd_opt(2026, 9, 1).unwrap(),
            actor_user_id: None,
        })
        .unwrap();
        let mer = NaiveDate::from_ymd_opt(2026, 10, 14).unwrap(); // mercredi
        assert_eq!(mer.weekday(), Weekday::Wed);
        assert!(grille_couvre_jour(&g, mer));
        let sam = NaiveDate::from_ymd_opt(2026, 10, 17).unwrap();
        assert!(!grille_couvre_jour(&g, sam));
    }

    #[test]
    fn bourse_procuration_deductible_apres_acceptation() {
        let mut b = plan_affecter_bourse(AffecterBourse {
            etablissement_id: Uuid::new_v4(),
            eleve_id: Uuid::new_v4(),
            bourse_type_id: Uuid::new_v4(),
            echelon: "2".into(),
            avec_procuration: true,
            date_debut: NaiveDate::from_ymd_opt(2026, 9, 1).unwrap(),
            actor_user_id: None,
        })
        .unwrap();
        let jour = NaiveDate::from_ymd_opt(2026, 10, 12).unwrap();
        assert!(!bourse_deductible_facture(&b, jour));
        b = plan_decider_bourse(
            &b,
            DeciderBourse {
                etablissement_id: b.etablissement_id,
                bourse_id: b.id,
                accepter: true,
                actor_user_id: None,
            },
        )
        .unwrap();
        assert!(bourse_deductible_facture(&b, jour));
    }

    #[test]
    fn dispense_exclut_jour_et_volume_facturable() {
        let act_id = Uuid::new_v4();
        let etab = Uuid::new_v4();
        let eleve = Uuid::new_v4();
        let g = plan_poser_grille_repas(PoserGrilleRepas {
            etablissement_id: etab,
            eleve_id: eleve,
            activite_id: act_id,
            jours: JoursMasque::lun_ven(),
            date_debut: NaiveDate::from_ymd_opt(2026, 10, 1).unwrap(),
            actor_user_id: None,
        })
        .unwrap();
        let act = RepasActivite {
            id: act_id,
            etablissement_id: etab,
            code: "MIDI".into(),
            libelle: "Cantine midi".into(),
            kind: RepasActiviteKind::Midi,
            actif: true,
        };
        let mer = NaiveDate::from_ymd_opt(2026, 10, 14).unwrap();
        assert!(inscrit_activite_kind_au_jour(&[g.clone()], &[act.clone()], RepasActiviteKind::Midi, mer));
        let d = plan_poser_dispense_repas(PoserDispenseRepas {
            etablissement_id: etab,
            eleve_id: eleve,
            activite_id: act_id,
            jour: mer,
            motif: "Sortie scolaire".into(),
            source: DispenseSource::Parent,
            actor_user_id: None,
        })
        .unwrap();
        assert!(!inscrit_activite_kind_au_jour_avec_dispenses(
            &[g.clone()],
            &[act.clone()],
            &[d.clone()],
            RepasActiviteKind::Midi,
            mer
        ));
        let debut = NaiveDate::from_ymd_opt(2026, 10, 12).unwrap(); // lun
        let fin = NaiveDate::from_ymd_opt(2026, 10, 16).unwrap(); // ven
        let sans = compter_jours_facturables(&[g.clone()], &[act.clone()], &[], RepasActiviteKind::Midi, debut, fin)
            .unwrap();
        let avec = compter_jours_facturables(
            &[g.clone()],
            &[act.clone()],
            &[d.clone()],
            RepasActiviteKind::Midi,
            debut,
            fin,
        )
        .unwrap();
        assert_eq!(sans, 5);
        assert_eq!(avec, 4);

        let vols = agreger_volumes_repas(
            &[g],
            &[act],
            &[d],
            debut,
            fin,
            Some(RepasActiviteKind::Midi),
        )
        .unwrap();
        assert_eq!(vols.len(), 4); // mercredi exclu
        assert!(vols.iter().all(|v| v.nb_prevus == 1));
        assert!(!vols.iter().any(|v| v.jour == mer));
        let csv = String::from_utf8(render_volumes_cuisine_csv(&vols)).unwrap();
        assert!(csv.contains("date;activite;kind;nb_prevus"));
        assert!(csv.contains("MIDI;midi;1"));
    }

    #[test]
    fn passage_badge_ok_puis_refuse_si_dispense_ou_doublon() {
        let act_id = Uuid::new_v4();
        let etab = Uuid::new_v4();
        let eleve = Uuid::new_v4();
        let g = plan_poser_grille_repas(PoserGrilleRepas {
            etablissement_id: etab,
            eleve_id: eleve,
            activite_id: act_id,
            jours: JoursMasque::lun_ven(),
            date_debut: NaiveDate::from_ymd_opt(2026, 10, 1).unwrap(),
            actor_user_id: None,
        })
        .unwrap();
        let mer = NaiveDate::from_ymd_opt(2026, 10, 14).unwrap();
        let p = plan_enregistrer_passage_badge(
            EnregistrerPassageBadge {
                etablissement_id: etab,
                eleve_id: eleve,
                activite_id: act_id,
                jour: mer,
                badge_lu: "B001".into(),
                source: PassageSource::Badge,
                actor_user_id: None,
            },
            &[g.clone()],
            &[],
            false,
        )
        .unwrap();
        assert_eq!(p.badge_lu, "B001");
        assert!(plan_enregistrer_passage_badge(
            EnregistrerPassageBadge {
                etablissement_id: etab,
                eleve_id: eleve,
                activite_id: act_id,
                jour: mer,
                badge_lu: "B001".into(),
                source: PassageSource::Badge,
                actor_user_id: None,
            },
            &[g.clone()],
            &[],
            true,
        )
        .is_err());
        let d = plan_poser_dispense_repas(PoserDispenseRepas {
            etablissement_id: etab,
            eleve_id: eleve,
            activite_id: act_id,
            jour: mer,
            motif: "sortie".into(),
            source: DispenseSource::Parent,
            actor_user_id: None,
        })
        .unwrap();
        assert_eq!(
            plan_enregistrer_passage_badge(
                EnregistrerPassageBadge {
                    etablissement_id: etab,
                    eleve_id: eleve,
                    activite_id: act_id,
                    jour: mer,
                    badge_lu: "B001".into(),
                    source: PassageSource::Badge,
                    actor_user_id: None,
                },
                &[g],
                &[d],
                false,
            )
            .unwrap_err(),
            DomainError::PassageBadgeRefuse
        );
    }

    #[test]
    fn bourse_sans_procuration_pas_deductible() {
        let mut b = plan_affecter_bourse(AffecterBourse {
            etablissement_id: Uuid::new_v4(),
            eleve_id: Uuid::new_v4(),
            bourse_type_id: Uuid::new_v4(),
            echelon: "1".into(),
            avec_procuration: false,
            date_debut: NaiveDate::from_ymd_opt(2026, 9, 1).unwrap(),
            actor_user_id: None,
        })
        .unwrap();
        b.status = BourseStatus::Acceptee;
        let jour = NaiveDate::from_ymd_opt(2026, 10, 12).unwrap();
        assert!(!bourse_deductible_facture(&b, jour));
    }
}
