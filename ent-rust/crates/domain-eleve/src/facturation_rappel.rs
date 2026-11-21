//! Rappels automatiques d'échéances facture — J-3 et en retard.
//! Canal P1 : enregistrement traçable + message famille (fil messagerie).

use crate::facturation::{
    couvrir_echeances, Echeance, EcheanceCouverture, Facture, FactureStatut,
};
use crate::DomainError;
use chrono::{DateTime, Duration, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RappelKind {
    JMoins3,
    EnRetard,
}

impl RappelKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::JMoins3 => "j_moins_3",
            Self::EnRetard => "en_retard",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "j_moins_3" => Some(Self::JMoins3),
            "en_retard" => Some(Self::EnRetard),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RappelCanal {
    Interne,
    Message,
}

impl RappelCanal {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Interne => "interne",
            Self::Message => "message",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "interne" => Some(Self::Interne),
            "message" => Some(Self::Message),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RappelFacture {
    pub id: Uuid,
    pub etablissement_id: Uuid,
    pub foyer_id: Uuid,
    pub facture_id: Uuid,
    pub echeance_id: Uuid,
    pub kind: RappelKind,
    pub canal: RappelCanal,
    pub corps: String,
    pub fil_id: Option<Uuid>,
    pub envoye_at: DateTime<Utc>,
    pub created_by_user_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RappelCandidat {
    pub etablissement_id: Uuid,
    pub foyer_id: Uuid,
    pub facture_id: Uuid,
    pub facture_numero: String,
    pub echeance_id: Uuid,
    pub echeance_libelle: String,
    pub echeance_date: NaiveDate,
    pub montant_centimes: i64,
    pub reste_echeance_centimes: i64,
    pub kind: RappelKind,
}

/// Délai avant échéance pour le rappel anticipé (jours calendaires).
pub const RAPPEL_J_MOINS: i64 = 3;

/// Sélectionne les échéances à rappeler aujourd'hui (non soldées, facture émise).
pub fn plan_candidats_rappel(
    facture: &Facture,
    echeances: &[Echeance],
    encaisse_centimes: i64,
    today: NaiveDate,
) -> Vec<RappelCandidat> {
    if facture.statut != FactureStatut::Emise {
        return Vec::new();
    }
    let vues = couvrir_echeances(echeances, encaisse_centimes, today);
    let j3 = today + Duration::days(RAPPEL_J_MOINS);
    let mut out = Vec::new();
    for v in vues {
        if v.couverture == EcheanceCouverture::Payee {
            continue;
        }
        let reste = v
            .echeance
            .montant_centimes
            .saturating_sub(v.couvert_centimes);
        if reste <= 0 {
            continue;
        }
        let kind = if v.echeance.date_echeance < today {
            Some(RappelKind::EnRetard)
        } else if v.echeance.date_echeance == j3 {
            Some(RappelKind::JMoins3)
        } else {
            None
        };
        let Some(kind) = kind else {
            continue;
        };
        out.push(RappelCandidat {
            etablissement_id: facture.etablissement_id,
            foyer_id: facture.foyer_id,
            facture_id: facture.id,
            facture_numero: facture.numero.clone(),
            echeance_id: v.echeance.id,
            echeance_libelle: v.echeance.libelle.clone(),
            echeance_date: v.echeance.date_echeance,
            montant_centimes: v.echeance.montant_centimes,
            reste_echeance_centimes: reste,
            kind,
        });
    }
    out
}

pub fn corps_rappel(c: &RappelCandidat) -> String {
    let euros = format!(
        "{},{:02} €",
        c.reste_echeance_centimes / 100,
        (c.reste_echeance_centimes % 100).unsigned_abs()
    );
    match c.kind {
        RappelKind::JMoins3 => format!(
            "Rappel : l'échéance « {} » de la facture {} arrive le {} (reste {} à régler).",
            c.echeance_libelle, c.facture_numero, c.echeance_date, euros
        ),
        RappelKind::EnRetard => format!(
            "Relance : l'échéance « {} » de la facture {} était due le {} (reste {} à régler).",
            c.echeance_libelle, c.facture_numero, c.echeance_date, euros
        ),
    }
}

pub fn sujet_rappel(c: &RappelCandidat) -> String {
    match c.kind {
        RappelKind::JMoins3 => format!("Rappel échéance facture {}", c.facture_numero),
        RappelKind::EnRetard => format!("Relance facture {}", c.facture_numero),
    }
}

pub fn plan_creer_rappel(
    c: &RappelCandidat,
    canal: RappelCanal,
    actor_user_id: Uuid,
    fil_id: Option<Uuid>,
) -> Result<RappelFacture, DomainError> {
    if c.etablissement_id.is_nil() || c.echeance_id.is_nil() {
        return Err(DomainError::Event(
            ent_events::EventBusError::MissingEtablissement,
        ));
    }
    let corps = corps_rappel(c);
    if corps.trim().is_empty() {
        return Err(DomainError::RappelInvalide);
    }
    let now = Utc::now();
    Ok(RappelFacture {
        id: Uuid::new_v4(),
        etablissement_id: c.etablissement_id,
        foyer_id: c.foyer_id,
        facture_id: c.facture_id,
        echeance_id: c.echeance_id,
        kind: c.kind,
        canal,
        corps,
        fil_id,
        envoye_at: now,
        created_by_user_id: Some(actor_user_id),
        created_at: now,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::facturation::FactureStatut;

    fn facture(total: i64) -> Facture {
        Facture {
            id: Uuid::new_v4(),
            etablissement_id: Uuid::new_v4(),
            foyer_id: Uuid::new_v4(),
            numero: "F-R1".into(),
            libelle: "Test".into(),
            statut: FactureStatut::Emise,
            total_centimes: total,
            nature: None,
            annee_scolaire: None,
            created_by_user_id: None,
            created_at: Utc::now(),
            emise_at: Some(Utc::now()),
            soldee_at: None,
        }
    }

    fn ech(facture_id: Uuid, etab: Uuid, date: NaiveDate, montant: i64) -> Echeance {
        Echeance {
            id: Uuid::new_v4(),
            etablissement_id: etab,
            facture_id,
            rang: 1,
            libelle: "Part 1".into(),
            montant_centimes: montant,
            bourse_centimes: 0,
            date_echeance: date,
            created_at: Utc::now(),
        }
    }

    #[test]
    fn candidat_j_moins_3_et_retard() {
        let f = facture(10000);
        let today = NaiveDate::from_ymd_opt(2026, 10, 13).unwrap();
        let e1 = ech(
            f.id,
            f.etablissement_id,
            today + Duration::days(3),
            5000,
        );
        let e2 = ech(
            f.id,
            f.etablissement_id,
            today - Duration::days(2),
            5000,
        );
        // Rang order matters for couverture — rebuild with rang
        let mut e1 = e1;
        e1.rang = 1;
        let mut e2 = e2;
        e2.rang = 2;
        let cands = plan_candidats_rappel(&f, &[e1.clone(), e2.clone()], 0, today);
        assert_eq!(cands.len(), 2);
        assert!(cands.iter().any(|c| c.kind == RappelKind::JMoins3));
        assert!(cands.iter().any(|c| c.kind == RappelKind::EnRetard));
    }

    #[test]
    fn ignore_si_payee() {
        let f = facture(5000);
        let today = NaiveDate::from_ymd_opt(2026, 10, 13).unwrap();
        let e = ech(f.id, f.etablissement_id, today - Duration::days(1), 5000);
        let cands = plan_candidats_rappel(&f, &[e], 5000, today);
        assert!(cands.is_empty());
    }
}
