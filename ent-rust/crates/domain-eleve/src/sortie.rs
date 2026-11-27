//! Autorisations de sortie — Vie scolaire (P1).
//!
//! Consentement permanent parent (créneaux midi / soir / matin) + forbid établissement,
//! demande exceptionnelle (RDV…) → accept/refus staff, contrôle badge portail.

use crate::DomainError;
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Créneau de sortie autorisé / demandé.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SortieCreneau {
    Matin,
    Midi,
    Soir,
}

impl SortieCreneau {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Matin => "matin",
            Self::Midi => "midi",
            Self::Soir => "soir",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "matin" => Some(Self::Matin),
            "midi" => Some(Self::Midi),
            "soir" => Some(Self::Soir),
            _ => None,
        }
    }
}

/// Consentement permanent (récap élève) — un enregistrement par élève.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AutorisationSortie {
    pub id: Uuid,
    pub etablissement_id: Uuid,
    pub eleve_id: Uuid,
    pub matin: bool,
    pub midi: bool,
    pub soir: bool,
    /// Interdiction totale posée par l'établissement (aucun choix parent).
    pub interdit_etablissement: bool,
    pub updated_by_user_id: Uuid,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct UpsertAutorisationSortie {
    pub etablissement_id: Uuid,
    pub eleve_id: Uuid,
    pub matin: bool,
    pub midi: bool,
    pub soir: bool,
    pub interdit_etablissement: bool,
    pub actor_user_id: Uuid,
}

pub fn plan_upsert_autorisation_sortie(
    cmd: UpsertAutorisationSortie,
) -> Result<AutorisationSortie, DomainError> {
    if cmd.etablissement_id.is_nil() || cmd.eleve_id.is_nil() {
        return Err(DomainError::Event(
            ent_events::EventBusError::MissingEtablissement,
        ));
    }
    // Forbid établissement : force tous les créneaux à false.
    let (matin, midi, soir) = if cmd.interdit_etablissement {
        (false, false, false)
    } else {
        (cmd.matin, cmd.midi, cmd.soir)
    };
    Ok(AutorisationSortie {
        id: Uuid::new_v4(),
        etablissement_id: cmd.etablissement_id,
        eleve_id: cmd.eleve_id,
        matin,
        midi,
        soir,
        interdit_etablissement: cmd.interdit_etablissement,
        updated_by_user_id: cmd.actor_user_id,
        updated_at: Utc::now(),
    })
}

impl AutorisationSortie {
    pub fn autorise_creneau(&self, creneau: SortieCreneau) -> bool {
        if self.interdit_etablissement {
            return false;
        }
        match creneau {
            SortieCreneau::Matin => self.matin,
            SortieCreneau::Midi => self.midi,
            SortieCreneau::Soir => self.soir,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DemandeSortieStatut {
    EnAttente,
    Acceptee,
    Refusee,
}

impl DemandeSortieStatut {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::EnAttente => "en_attente",
            Self::Acceptee => "acceptee",
            Self::Refusee => "refusee",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "en_attente" => Some(Self::EnAttente),
            "acceptee" => Some(Self::Acceptee),
            "refusee" => Some(Self::Refusee),
            _ => None,
        }
    }
}

/// Demande exceptionnelle hors cadre permanent (RDV médical, etc.).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DemandeSortieExceptionnelle {
    pub id: Uuid,
    pub etablissement_id: Uuid,
    pub eleve_id: Uuid,
    pub jour: NaiveDate,
    pub creneau: SortieCreneau,
    pub motif: String,
    pub statut: DemandeSortieStatut,
    pub demande_par_user_id: Uuid,
    pub decided_by_user_id: Option<Uuid>,
    pub decided_at: Option<DateTime<Utc>>,
    pub decision_note: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct DemanderSortieExceptionnelle {
    pub etablissement_id: Uuid,
    pub eleve_id: Uuid,
    pub jour: NaiveDate,
    pub creneau: SortieCreneau,
    pub motif: String,
    pub actor_user_id: Uuid,
}

pub fn plan_demander_sortie_exceptionnelle(
    cmd: DemanderSortieExceptionnelle,
) -> Result<DemandeSortieExceptionnelle, DomainError> {
    if cmd.etablissement_id.is_nil() || cmd.eleve_id.is_nil() {
        return Err(DomainError::Event(
            ent_events::EventBusError::MissingEtablissement,
        ));
    }
    let motif = cmd.motif.trim().to_string();
    if motif.is_empty() {
        return Err(DomainError::SortieDemandeInvalide);
    }
    Ok(DemandeSortieExceptionnelle {
        id: Uuid::new_v4(),
        etablissement_id: cmd.etablissement_id,
        eleve_id: cmd.eleve_id,
        jour: cmd.jour,
        creneau: cmd.creneau,
        motif,
        statut: DemandeSortieStatut::EnAttente,
        demande_par_user_id: cmd.actor_user_id,
        decided_by_user_id: None,
        decided_at: None,
        decision_note: String::new(),
        created_at: Utc::now(),
    })
}

#[derive(Debug, Clone)]
pub struct DeciderDemandeSortie {
    pub etablissement_id: Uuid,
    pub demande_id: Uuid,
    pub accepter: bool,
    pub decision_note: String,
    pub actor_user_id: Uuid,
}

pub fn plan_decider_demande_sortie(
    current: &DemandeSortieExceptionnelle,
    cmd: DeciderDemandeSortie,
) -> Result<DemandeSortieExceptionnelle, DomainError> {
    if current.etablissement_id != cmd.etablissement_id || current.id != cmd.demande_id {
        return Err(DomainError::TenantMismatch);
    }
    if current.statut != DemandeSortieStatut::EnAttente {
        return Err(DomainError::SortieDemandeDejaTranchee);
    }
    let mut next = current.clone();
    next.statut = if cmd.accepter {
        DemandeSortieStatut::Acceptee
    } else {
        DemandeSortieStatut::Refusee
    };
    next.decided_by_user_id = Some(cmd.actor_user_id);
    next.decided_at = Some(Utc::now());
    next.decision_note = cmd.decision_note.trim().to_string();
    Ok(next)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ControleSortieVerdict {
    Autorisee,
    Refusee,
}

impl ControleSortieVerdict {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Autorisee => "autorisee",
            Self::Refusee => "refusee",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "autorisee" => Some(Self::Autorisee),
            "refusee" => Some(Self::Refusee),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ControleSortieSource {
    Permanente,
    Exception,
    Interdit,
    Aucune,
    /// Prochain cours EDT sur un autre site (param établissement).
    AutreSite,
}

impl ControleSortieSource {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Permanente => "permanente",
            Self::Exception => "exception",
            Self::Interdit => "interdit",
            Self::Aucune => "aucune",
            Self::AutreSite => "autre_site",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "permanente" => Some(Self::Permanente),
            "exception" => Some(Self::Exception),
            "interdit" => Some(Self::Interdit),
            "aucune" => Some(Self::Aucune),
            "autre_site" => Some(Self::AutreSite),
            _ => None,
        }
    }
}

/// Paramètres établissement pour les sorties.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SortieParamEtab {
    pub etablissement_id: Uuid,
    pub autoriser_changement_site: bool,
    pub updated_by_user_id: Uuid,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct UpsertSortieParamEtab {
    pub etablissement_id: Uuid,
    pub autoriser_changement_site: bool,
    pub actor_user_id: Uuid,
}

pub fn plan_upsert_sortie_param(cmd: UpsertSortieParamEtab) -> Result<SortieParamEtab, DomainError> {
    if cmd.etablissement_id.is_nil() {
        return Err(DomainError::Event(
            ent_events::EventBusError::MissingEtablissement,
        ));
    }
    Ok(SortieParamEtab {
        etablissement_id: cmd.etablissement_id,
        autoriser_changement_site: cmd.autoriser_changement_site,
        updated_by_user_id: cmd.actor_user_id,
        updated_at: Utc::now(),
    })
}

/// Créneau EDT allégé pour décision « autre site ».
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SortieEdtApercu {
    pub heure_debut: chrono::NaiveTime,
    pub site: String,
}

/// True si un créneau ultérieur est sur un site ≠ portail.
pub fn a_cours_autre_site_apres(
    site_portail: &str,
    maintenant: chrono::NaiveTime,
    creneaux: &[SortieEdtApercu],
) -> Option<String> {
    let portail = site_portail.trim();
    if portail.is_empty() {
        return None;
    }
    creneaux.iter().find_map(|c| {
        let site = c.site.trim();
        if site.is_empty() || site.eq_ignore_ascii_case(portail) {
            return None;
        }
        if c.heure_debut > maintenant {
            Some(site.to_string())
        } else {
            None
        }
    })
}

/// Décision de contrôle (badge portail) — journalisée.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ControleSortie {
    pub id: Uuid,
    pub etablissement_id: Uuid,
    pub eleve_id: Uuid,
    pub jour: NaiveDate,
    pub creneau: SortieCreneau,
    pub verdict: ControleSortieVerdict,
    pub source: ControleSortieSource,
    pub motif: String,
    pub actor_user_id: Uuid,
    pub created_at: DateTime<Utc>,
}

/// Contexte optionnel pour la décision « autre site » (EDT).
#[derive(Debug, Clone, Default)]
pub struct ControleSortieContexteSite {
    pub autoriser_changement_site: bool,
    pub site_portail: Option<String>,
    pub maintenant: Option<chrono::NaiveTime>,
    pub creneaux_apres: Vec<SortieEdtApercu>,
}

/// Évalue si l'élève peut sortir à ce créneau (permanent + exceptions + autre site).
pub fn evaluer_controle_sortie(
    auth: Option<&AutorisationSortie>,
    demandes_jour: &[DemandeSortieExceptionnelle],
    creneau: SortieCreneau,
) -> (ControleSortieVerdict, ControleSortieSource, String) {
    evaluer_controle_sortie_avec_contexte(auth, demandes_jour, creneau, &ControleSortieContexteSite::default())
}

pub fn evaluer_controle_sortie_avec_contexte(
    auth: Option<&AutorisationSortie>,
    demandes_jour: &[DemandeSortieExceptionnelle],
    creneau: SortieCreneau,
    ctx: &ControleSortieContexteSite,
) -> (ControleSortieVerdict, ControleSortieSource, String) {
    if let Some(a) = auth {
        if a.interdit_etablissement {
            return (
                ControleSortieVerdict::Refusee,
                ControleSortieSource::Interdit,
                "interdiction établissement".into(),
            );
        }
    }
    let exception_acceptee = demandes_jour.iter().any(|d| {
        d.creneau == creneau && d.statut == DemandeSortieStatut::Acceptee
    });
    if exception_acceptee {
        return (
            ControleSortieVerdict::Autorisee,
            ControleSortieSource::Exception,
            "demande exceptionnelle acceptée".into(),
        );
    }
    let exception_en_attente = demandes_jour.iter().any(|d| {
        d.creneau == creneau && d.statut == DemandeSortieStatut::EnAttente
    });
    if let Some(a) = auth {
        if a.autorise_creneau(creneau) {
            return (
                ControleSortieVerdict::Autorisee,
                ControleSortieSource::Permanente,
                format!("consentement permanent {}", creneau.as_str()),
            );
        }
    }
    if ctx.autoriser_changement_site {
        if let (Some(portail), Some(now)) = (ctx.site_portail.as_deref(), ctx.maintenant) {
            if let Some(site_dest) =
                a_cours_autre_site_apres(portail, now, &ctx.creneaux_apres)
            {
                return (
                    ControleSortieVerdict::Autorisee,
                    ControleSortieSource::AutreSite,
                    format!("cours suivant sur site {site_dest}"),
                );
            }
        }
    }
    if exception_en_attente {
        return (
            ControleSortieVerdict::Refusee,
            ControleSortieSource::Aucune,
            "demande exceptionnelle en attente".into(),
        );
    }
    (
        ControleSortieVerdict::Refusee,
        ControleSortieSource::Aucune,
        format!("pas d'autorisation {}", creneau.as_str()),
    )
}

pub fn plan_controle_sortie(
    etablissement_id: Uuid,
    eleve_id: Uuid,
    jour: NaiveDate,
    creneau: SortieCreneau,
    auth: Option<&AutorisationSortie>,
    demandes_jour: &[DemandeSortieExceptionnelle],
    actor_user_id: Uuid,
) -> Result<ControleSortie, DomainError> {
    plan_controle_sortie_avec_contexte(
        etablissement_id,
        eleve_id,
        jour,
        creneau,
        auth,
        demandes_jour,
        actor_user_id,
        &ControleSortieContexteSite::default(),
    )
}

pub fn plan_controle_sortie_avec_contexte(
    etablissement_id: Uuid,
    eleve_id: Uuid,
    jour: NaiveDate,
    creneau: SortieCreneau,
    auth: Option<&AutorisationSortie>,
    demandes_jour: &[DemandeSortieExceptionnelle],
    actor_user_id: Uuid,
    ctx: &ControleSortieContexteSite,
) -> Result<ControleSortie, DomainError> {
    if etablissement_id.is_nil() || eleve_id.is_nil() {
        return Err(DomainError::Event(
            ent_events::EventBusError::MissingEtablissement,
        ));
    }
    let (verdict, source, motif) =
        evaluer_controle_sortie_avec_contexte(auth, demandes_jour, creneau, ctx);
    Ok(ControleSortie {
        id: Uuid::new_v4(),
        etablissement_id,
        eleve_id,
        jour,
        creneau,
        verdict,
        source,
        motif,
        actor_user_id,
        created_at: Utc::now(),
    })
}

/// Récapitulatif décision par élève (lecture staff / famille).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RecapSortieEleve {
    pub eleve_id: Uuid,
    pub autorisation: Option<AutorisationSortie>,
    pub demandes_ouvertes: Vec<DemandeSortieExceptionnelle>,
}

/// Règle établissement par niveau scolaire (CONFIGURABLE).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SortieRegleNiveau {
    pub id: Uuid,
    pub etablissement_id: Uuid,
    pub niveau: String,
    pub matin_autorise: bool,
    pub midi_autorise: bool,
    pub soir_autorise: bool,
    /// École / collège strict : aucun consentement parent.
    pub choix_parent_interdit: bool,
    pub updated_by_user_id: Uuid,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct UpsertSortieRegleNiveau {
    pub etablissement_id: Uuid,
    pub niveau: String,
    pub matin_autorise: bool,
    pub midi_autorise: bool,
    pub soir_autorise: bool,
    pub choix_parent_interdit: bool,
    pub actor_user_id: Uuid,
}

pub fn plan_upsert_sortie_regle_niveau(
    cmd: UpsertSortieRegleNiveau,
) -> Result<SortieRegleNiveau, DomainError> {
    if cmd.etablissement_id.is_nil() {
        return Err(DomainError::Event(
            ent_events::EventBusError::MissingEtablissement,
        ));
    }
    let niveau = cmd.niveau.trim().to_string();
    if niveau.is_empty() || niveau.len() > 32 {
        return Err(DomainError::SortieDemandeInvalide);
    }
    let (matin, midi, soir) = if cmd.choix_parent_interdit {
        (false, false, false)
    } else {
        (cmd.matin_autorise, cmd.midi_autorise, cmd.soir_autorise)
    };
    Ok(SortieRegleNiveau {
        id: Uuid::new_v4(),
        etablissement_id: cmd.etablissement_id,
        niveau,
        matin_autorise: matin,
        midi_autorise: midi,
        soir_autorise: soir,
        choix_parent_interdit: cmd.choix_parent_interdit,
        updated_by_user_id: cmd.actor_user_id,
        updated_at: Utc::now(),
    })
}

/// Applique la règle niveau sur un consentement parent (clamp des créneaux).
pub fn appliquer_regle_niveau_parent(
    mut auth: AutorisationSortie,
    regle: Option<&SortieRegleNiveau>,
) -> Result<AutorisationSortie, DomainError> {
    let Some(r) = regle else {
        return Ok(auth);
    };
    if r.choix_parent_interdit {
        return Err(DomainError::SortieRegleNiveauInterdit);
    }
    auth.matin = auth.matin && r.matin_autorise;
    auth.midi = auth.midi && r.midi_autorise;
    auth.soir = auth.soir && r.soir_autorise;
    Ok(auth)
}

/// Régime élève pour les règles de sortie (externe / DP / interne).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SortieRegime {
    Ext,
    Dp,
    Interne,
}

impl SortieRegime {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ext => "EXT",
            Self::Dp => "DP",
            Self::Interne => "INT",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match normalize_sortie_regime(s).as_str() {
            "EXT" => Some(Self::Ext),
            "DP" => Some(Self::Dp),
            "INT" => Some(Self::Interne),
            _ => None,
        }
    }
}

/// Normalise EXT / DP / INT (aliases FR courants).
pub fn normalize_sortie_regime(s: &str) -> String {
    let t = s.trim().to_ascii_uppercase().replace(' ', "");
    match t.as_str() {
        "EXT" | "EXTERNE" | "EXT." => "EXT".into(),
        "DP" | "DEMI-PENSION" | "DEMIPENSION" | "1/2P" | "1/2 PENSION" => "DP".into(),
        "INT" | "INTERNE" | "INTERNAT" => "INT".into(),
        other => other.to_string(),
    }
}

/// Règle établissement par régime (CONFIGURABLE).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SortieRegleRegime {
    pub id: Uuid,
    pub etablissement_id: Uuid,
    pub regime: SortieRegime,
    pub matin_autorise: bool,
    pub midi_autorise: bool,
    pub soir_autorise: bool,
    pub choix_parent_interdit: bool,
    pub updated_by_user_id: Uuid,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct UpsertSortieRegleRegime {
    pub etablissement_id: Uuid,
    pub regime: String,
    pub matin_autorise: bool,
    pub midi_autorise: bool,
    pub soir_autorise: bool,
    pub choix_parent_interdit: bool,
    pub actor_user_id: Uuid,
}

pub fn plan_upsert_sortie_regle_regime(
    cmd: UpsertSortieRegleRegime,
) -> Result<SortieRegleRegime, DomainError> {
    if cmd.etablissement_id.is_nil() {
        return Err(DomainError::Event(
            ent_events::EventBusError::MissingEtablissement,
        ));
    }
    let regime = SortieRegime::parse(&cmd.regime).ok_or(DomainError::SortieRegimeInvalide)?;
    let (matin, midi, soir) = if cmd.choix_parent_interdit {
        (false, false, false)
    } else {
        (cmd.matin_autorise, cmd.midi_autorise, cmd.soir_autorise)
    };
    Ok(SortieRegleRegime {
        id: Uuid::new_v4(),
        etablissement_id: cmd.etablissement_id,
        regime,
        matin_autorise: matin,
        midi_autorise: midi,
        soir_autorise: soir,
        choix_parent_interdit: cmd.choix_parent_interdit,
        updated_by_user_id: cmd.actor_user_id,
        updated_at: Utc::now(),
    })
}

/// Applique la règle régime sur un consentement parent.
pub fn appliquer_regle_regime_parent(
    mut auth: AutorisationSortie,
    regle: Option<&SortieRegleRegime>,
) -> Result<AutorisationSortie, DomainError> {
    let Some(r) = regle else {
        return Ok(auth);
    };
    if r.choix_parent_interdit {
        return Err(DomainError::SortieRegleRegimeInterdit);
    }
    auth.matin = auth.matin && r.matin_autorise;
    auth.midi = auth.midi && r.midi_autorise;
    auth.soir = auth.soir && r.soir_autorise;
    Ok(auth)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn forbid_annule_creneaux() {
        let a = plan_upsert_autorisation_sortie(UpsertAutorisationSortie {
            etablissement_id: Uuid::new_v4(),
            eleve_id: Uuid::new_v4(),
            matin: true,
            midi: true,
            soir: true,
            interdit_etablissement: true,
            actor_user_id: Uuid::new_v4(),
        })
        .unwrap();
        assert!(a.interdit_etablissement);
        assert!(!a.matin && !a.midi && !a.soir);
        assert!(!a.autorise_creneau(SortieCreneau::Midi));
    }

    #[test]
    fn exception_accepte_passe_avant_permanente() {
        let etab = Uuid::new_v4();
        let eleve = Uuid::new_v4();
        let auth = plan_upsert_autorisation_sortie(UpsertAutorisationSortie {
            etablissement_id: etab,
            eleve_id: eleve,
            matin: false,
            midi: false,
            soir: false,
            interdit_etablissement: false,
            actor_user_id: Uuid::new_v4(),
        })
        .unwrap();
        let dem = plan_demander_sortie_exceptionnelle(DemanderSortieExceptionnelle {
            etablissement_id: etab,
            eleve_id: eleve,
            jour: NaiveDate::from_ymd_opt(2026, 10, 13).unwrap(),
            creneau: SortieCreneau::Matin,
            motif: "RDV orthodontiste".into(),
            actor_user_id: Uuid::new_v4(),
        })
        .unwrap();
        let dem = plan_decider_demande_sortie(
            &dem,
            DeciderDemandeSortie {
                etablissement_id: etab,
                demande_id: dem.id,
                accepter: true,
                decision_note: "ok".into(),
                actor_user_id: Uuid::new_v4(),
            },
        )
        .unwrap();
        let (v, src, _) = evaluer_controle_sortie(Some(&auth), &[dem], SortieCreneau::Matin);
        assert_eq!(v, ControleSortieVerdict::Autorisee);
        assert_eq!(src, ControleSortieSource::Exception);
    }

    #[test]
    fn permanente_midi_sans_exception() {
        let auth = plan_upsert_autorisation_sortie(UpsertAutorisationSortie {
            etablissement_id: Uuid::new_v4(),
            eleve_id: Uuid::new_v4(),
            matin: false,
            midi: true,
            soir: true,
            interdit_etablissement: false,
            actor_user_id: Uuid::new_v4(),
        })
        .unwrap();
        let (v, src, _) = evaluer_controle_sortie(Some(&auth), &[], SortieCreneau::Midi);
        assert_eq!(v, ControleSortieVerdict::Autorisee);
        assert_eq!(src, ControleSortieSource::Permanente);
        let (v2, _, _) = evaluer_controle_sortie(Some(&auth), &[], SortieCreneau::Matin);
        assert_eq!(v2, ControleSortieVerdict::Refusee);
    }

    #[test]
    fn regle_niveau_clamp_parent() {
        let etab = Uuid::new_v4();
        let regle = plan_upsert_sortie_regle_niveau(UpsertSortieRegleNiveau {
            etablissement_id: etab,
            niveau: "3e".into(),
            matin_autorise: false,
            midi_autorise: false,
            soir_autorise: true,
            choix_parent_interdit: false,
            actor_user_id: Uuid::new_v4(),
        })
        .unwrap();
        let auth = plan_upsert_autorisation_sortie(UpsertAutorisationSortie {
            etablissement_id: etab,
            eleve_id: Uuid::new_v4(),
            matin: true,
            midi: true,
            soir: true,
            interdit_etablissement: false,
            actor_user_id: Uuid::new_v4(),
        })
        .unwrap();
        let clamped = appliquer_regle_niveau_parent(auth, Some(&regle)).unwrap();
        assert!(!clamped.matin && !clamped.midi && clamped.soir);
    }

    #[test]
    fn regle_regime_dp_bloque_midi() {
        let regle = plan_upsert_sortie_regle_regime(UpsertSortieRegleRegime {
            etablissement_id: Uuid::new_v4(),
            regime: "demi-pension".into(),
            matin_autorise: false,
            midi_autorise: false,
            soir_autorise: true,
            choix_parent_interdit: false,
            actor_user_id: Uuid::new_v4(),
        })
        .unwrap();
        assert_eq!(regle.regime, SortieRegime::Dp);
        let auth = plan_upsert_autorisation_sortie(UpsertAutorisationSortie {
            etablissement_id: regle.etablissement_id,
            eleve_id: Uuid::new_v4(),
            matin: true,
            midi: true,
            soir: true,
            interdit_etablissement: false,
            actor_user_id: Uuid::new_v4(),
        })
        .unwrap();
        let clamped = appliquer_regle_regime_parent(auth, Some(&regle)).unwrap();
        assert!(!clamped.matin && !clamped.midi && clamped.soir);
    }

    #[test]
    fn autre_site_apres_detecte() {
        use chrono::NaiveTime;
        let dest = a_cours_autre_site_apres(
            "SiteA",
            NaiveTime::from_hms_opt(10, 0, 0).unwrap(),
            &[
                SortieEdtApercu {
                    heure_debut: NaiveTime::from_hms_opt(9, 0, 0).unwrap(),
                    site: "SiteB".into(),
                },
                SortieEdtApercu {
                    heure_debut: NaiveTime::from_hms_opt(14, 0, 0).unwrap(),
                    site: "SiteB".into(),
                },
            ],
        );
        assert_eq!(dest.as_deref(), Some("SiteB"));
        let (v, src, _) = evaluer_controle_sortie_avec_contexte(
            None,
            &[],
            SortieCreneau::Midi,
            &ControleSortieContexteSite {
                autoriser_changement_site: true,
                site_portail: Some("SiteA".into()),
                maintenant: Some(NaiveTime::from_hms_opt(10, 0, 0).unwrap()),
                creneaux_apres: vec![SortieEdtApercu {
                    heure_debut: NaiveTime::from_hms_opt(14, 0, 0).unwrap(),
                    site: "SiteB".into(),
                }],
            },
        );
        assert_eq!(v, ControleSortieVerdict::Autorisee);
        assert_eq!(src, ControleSortieSource::AutreSite);
    }

    #[test]
    fn normalize_regime_aliases() {
        assert_eq!(normalize_sortie_regime("externe"), "EXT");
        assert_eq!(normalize_sortie_regime("1/2P"), "DP");
        assert_eq!(normalize_sortie_regime("internat"), "INT");
        assert_eq!(SortieRegime::parse("DP"), Some(SortieRegime::Dp));
    }

    #[test]
    fn regle_niveau_interdit_parent() {
        let regle = plan_upsert_sortie_regle_niveau(UpsertSortieRegleNiveau {
            etablissement_id: Uuid::new_v4(),
            niveau: "cm2".into(),
            matin_autorise: true,
            midi_autorise: true,
            soir_autorise: true,
            choix_parent_interdit: true,
            actor_user_id: Uuid::new_v4(),
        })
        .unwrap();
        assert!(regle.choix_parent_interdit);
        assert!(!regle.matin_autorise);
        let auth = plan_upsert_autorisation_sortie(UpsertAutorisationSortie {
            etablissement_id: regle.etablissement_id,
            eleve_id: Uuid::new_v4(),
            matin: true,
            midi: true,
            soir: true,
            interdit_etablissement: false,
            actor_user_id: Uuid::new_v4(),
        })
        .unwrap();
        let err = appliquer_regle_niveau_parent(auth, Some(&regle)).unwrap_err();
        assert_eq!(err, DomainError::SortieRegleNiveauInterdit);
    }

    #[test]
    fn re_decision_refusee() {
        let dem = plan_demander_sortie_exceptionnelle(DemanderSortieExceptionnelle {
            etablissement_id: Uuid::new_v4(),
            eleve_id: Uuid::new_v4(),
            jour: NaiveDate::from_ymd_opt(2026, 10, 13).unwrap(),
            creneau: SortieCreneau::Soir,
            motif: "dentiste".into(),
            actor_user_id: Uuid::new_v4(),
        })
        .unwrap();
        let decided = plan_decider_demande_sortie(
            &dem,
            DeciderDemandeSortie {
                etablissement_id: dem.etablissement_id,
                demande_id: dem.id,
                accepter: false,
                decision_note: "non".into(),
                actor_user_id: Uuid::new_v4(),
            },
        )
        .unwrap();
        let err = plan_decider_demande_sortie(
            &decided,
            DeciderDemandeSortie {
                etablissement_id: dem.etablissement_id,
                demande_id: dem.id,
                accepter: true,
                decision_note: "".into(),
                actor_user_id: Uuid::new_v4(),
            },
        )
        .unwrap_err();
        assert_eq!(err, DomainError::SortieDemandeDejaTranchee);
    }
}
