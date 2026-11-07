//! Facturation foyer — P1 : facture + émission + encaissements + échéancier + quittances.
//! PDF / OGEC / SEPA = modules satellites (`facturation_pdf`, `facturation_ogec`, `facturation_sepa`).

use crate::DomainError;
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Nature facture : acompte réinscription N+1 (vanne paiement).
pub const NATURE_REINSCRIPTION_ACOMPTE: &str = "reinscription_acompte";
/// Nature facture : cantine facturée sur passages badge (pas sur grille seule).
pub const NATURE_CANTINE_BADGE: &str = "cantine_badge";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FactureStatut {
    Brouillon,
    Emise,
    Soldee,
}

impl FactureStatut {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Brouillon => "brouillon",
            Self::Emise => "emise",
            Self::Soldee => "soldee",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "brouillon" => Some(Self::Brouillon),
            "emise" => Some(Self::Emise),
            "soldee" => Some(Self::Soldee),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FactureLigne {
    pub id: Uuid,
    pub etablissement_id: Uuid,
    pub facture_id: Uuid,
    pub eleve_id: Option<Uuid>,
    pub libelle: String,
    pub quantite: i32,
    pub montant_unitaire_centimes: i64,
}

impl FactureLigne {
    pub fn total_centimes(&self) -> i64 {
        self.montant_unitaire_centimes.saturating_mul(i64::from(self.quantite))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Facture {
    pub id: Uuid,
    pub etablissement_id: Uuid,
    pub foyer_id: Uuid,
    pub numero: String,
    pub libelle: String,
    pub statut: FactureStatut,
    pub total_centimes: i64,
    pub nature: Option<String>,
    pub annee_scolaire: Option<String>,
    pub created_by_user_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub emise_at: Option<DateTime<Utc>>,
    pub soldee_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MoyenPaiement {
    Especes,
    Cheque,
    Virement,
    Cb,
    Prelevement,
    Autre,
}

impl MoyenPaiement {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Especes => "especes",
            Self::Cheque => "cheque",
            Self::Virement => "virement",
            Self::Cb => "cb",
            Self::Prelevement => "prelevement",
            Self::Autre => "autre",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "especes" | "espèces" | "cash" => Some(Self::Especes),
            "cheque" | "chèque" | "cheques" => Some(Self::Cheque),
            "virement" => Some(Self::Virement),
            "cb" | "carte" => Some(Self::Cb),
            "prelevement" | "prélèvement" | "sepa" => Some(Self::Prelevement),
            "autre" => Some(Self::Autre),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Encaissement {
    pub id: Uuid,
    pub etablissement_id: Uuid,
    pub facture_id: Uuid,
    pub montant_centimes: i64,
    pub moyen: MoyenPaiement,
    pub reference: Option<String>,
    pub encaisse_at: DateTime<Utc>,
    pub created_by_user_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct LigneFactureCmd {
    pub eleve_id: Option<Uuid>,
    pub libelle: String,
    pub quantite: i32,
    pub montant_unitaire_centimes: i64,
}

#[derive(Debug, Clone)]
pub struct CreerFacture {
    pub etablissement_id: Uuid,
    pub foyer_id: Uuid,
    pub numero: String,
    pub libelle: String,
    pub nature: Option<String>,
    pub annee_scolaire: Option<String>,
    pub lignes: Vec<LigneFactureCmd>,
    pub actor_user_id: Uuid,
}

/// Construit une ligne facture cantine à partir du nombre de passages badge.
pub fn plan_ligne_cantine_depuis_passages(
    eleve_id: Uuid,
    nb_passages: u32,
    prix_unitaire_centimes: i64,
    libelle: Option<String>,
) -> Result<LigneFactureCmd, DomainError> {
    if nb_passages == 0 {
        return Err(DomainError::FactureSansLigne);
    }
    if prix_unitaire_centimes <= 0 {
        return Err(DomainError::FactureInvalide);
    }
    let q = i32::try_from(nb_passages).map_err(|_| DomainError::FactureInvalide)?;
    let lib = libelle
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| format!("Cantine ({nb_passages} passage(s) badge)"));
    Ok(LigneFactureCmd {
        eleve_id: Some(eleve_id),
        libelle: lib,
        quantite: q,
        montant_unitaire_centimes: prix_unitaire_centimes,
    })
}

pub fn plan_creer_facture(cmd: CreerFacture) -> Result<(Facture, Vec<FactureLigne>), DomainError> {
    if cmd.etablissement_id.is_nil() || cmd.foyer_id.is_nil() {
        return Err(DomainError::Event(
            ent_events::EventBusError::MissingEtablissement,
        ));
    }
    let numero = cmd.numero.trim().to_string();
    let libelle = cmd.libelle.trim().to_string();
    if numero.is_empty() || libelle.is_empty() {
        return Err(DomainError::FactureInvalide);
    }
    if cmd.lignes.is_empty() {
        return Err(DomainError::FactureSansLigne);
    }
    let facture_id = Uuid::new_v4();
    let now = Utc::now();
    let mut lignes = Vec::with_capacity(cmd.lignes.len());
    let mut total: i64 = 0;
    for l in cmd.lignes {
        let lib = l.libelle.trim().to_string();
        if lib.is_empty() || l.quantite <= 0 {
            return Err(DomainError::FactureInvalide);
        }
        let ligne = FactureLigne {
            id: Uuid::new_v4(),
            etablissement_id: cmd.etablissement_id,
            facture_id,
            eleve_id: l.eleve_id,
            libelle: lib,
            quantite: l.quantite,
            montant_unitaire_centimes: l.montant_unitaire_centimes,
        };
        total = total.saturating_add(ligne.total_centimes());
        lignes.push(ligne);
    }
    if total < 0 {
        return Err(DomainError::FactureInvalide);
    }
    let nature = cmd
        .nature
        .map(|n| n.trim().to_ascii_lowercase())
        .filter(|n| !n.is_empty());
    let annee_scolaire = cmd
        .annee_scolaire
        .map(|a| a.trim().to_string())
        .filter(|a| !a.is_empty());
    if nature.as_deref() == Some(NATURE_REINSCRIPTION_ACOMPTE) && annee_scolaire.is_none() {
        return Err(DomainError::FactureInvalide);
    }
    let facture = Facture {
        id: facture_id,
        etablissement_id: cmd.etablissement_id,
        foyer_id: cmd.foyer_id,
        numero,
        libelle,
        statut: FactureStatut::Brouillon,
        total_centimes: total,
        nature,
        annee_scolaire,
        created_by_user_id: Some(cmd.actor_user_id),
        created_at: now,
        emise_at: None,
        soldee_at: None,
    };
    Ok((facture, lignes))
}

pub fn plan_emettre_facture(f: &Facture) -> Result<Facture, DomainError> {
    if f.statut != FactureStatut::Brouillon {
        return Err(DomainError::FactureDejaEmise);
    }
    if f.total_centimes < 0 {
        return Err(DomainError::FactureInvalide);
    }
    let mut out = f.clone();
    out.statut = FactureStatut::Emise;
    out.emise_at = Some(Utc::now());
    Ok(out)
}

#[derive(Debug, Clone)]
pub struct EncaisserFacture {
    pub etablissement_id: Uuid,
    pub facture_id: Uuid,
    pub montant_centimes: i64,
    pub moyen: MoyenPaiement,
    pub reference: Option<String>,
    pub actor_user_id: Uuid,
}

/// Enregistre un paiement sur facture émise. Passe en soldée si somme >= total.
pub fn plan_encaisser(
    facture: &Facture,
    deja_encaisse_centimes: i64,
    cmd: EncaisserFacture,
) -> Result<(Encaissement, Facture), DomainError> {
    if facture.etablissement_id != cmd.etablissement_id || facture.id != cmd.facture_id {
        return Err(DomainError::TenantMismatch);
    }
    match facture.statut {
        FactureStatut::Brouillon => return Err(DomainError::FactureNonEmise),
        FactureStatut::Soldee => return Err(DomainError::FactureDejaSoldee),
        FactureStatut::Emise => {}
    }
    if cmd.montant_centimes <= 0 {
        return Err(DomainError::EncaissementInvalide);
    }
    if deja_encaisse_centimes < 0 {
        return Err(DomainError::EncaissementInvalide);
    }
    let nouveau_total = deja_encaisse_centimes.saturating_add(cmd.montant_centimes);
    if nouveau_total > facture.total_centimes {
        return Err(DomainError::EncaissementDepasse);
    }
    let now = Utc::now();
    let reference = cmd
        .reference
        .map(|r| r.trim().to_string())
        .filter(|r| !r.is_empty());
    let enc = Encaissement {
        id: Uuid::new_v4(),
        etablissement_id: cmd.etablissement_id,
        facture_id: cmd.facture_id,
        montant_centimes: cmd.montant_centimes,
        moyen: cmd.moyen,
        reference,
        encaisse_at: now,
        created_by_user_id: Some(cmd.actor_user_id),
        created_at: now,
    };
    let mut out = facture.clone();
    if nouveau_total >= facture.total_centimes {
        out.statut = FactureStatut::Soldee;
        out.soldee_at = Some(now);
    }
    Ok((enc, out))
}

pub fn reste_a_payer(total_centimes: i64, encaisse_centimes: i64) -> i64 {
    total_centimes.saturating_sub(encaisse_centimes).max(0)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Quittance {
    pub id: Uuid,
    pub etablissement_id: Uuid,
    pub foyer_id: Uuid,
    pub facture_id: Uuid,
    pub encaissement_id: Uuid,
    pub numero: String,
    pub montant_centimes: i64,
    pub moyen: MoyenPaiement,
    pub emise_at: DateTime<Utc>,
    pub created_by_user_id: Option<Uuid>,
}

/// Numéro de quittance dérivé de la facture + rang (1-based).
pub fn numero_quittance(facture_numero: &str, rang: i32) -> String {
    format!("Q-{}-{:02}", facture_numero.trim(), rang.max(1))
}

/// Émet une quittance pour un encaissement (1:1).
pub fn plan_emettre_quittance(
    facture: &Facture,
    enc: &Encaissement,
    rang: i32,
) -> Result<Quittance, DomainError> {
    if facture.etablissement_id != enc.etablissement_id || facture.id != enc.facture_id {
        return Err(DomainError::TenantMismatch);
    }
    if enc.montant_centimes <= 0 {
        return Err(DomainError::EncaissementInvalide);
    }
    let numero = numero_quittance(&facture.numero, rang);
    if numero.len() < 4 {
        return Err(DomainError::QuittanceInvalide);
    }
    Ok(Quittance {
        id: Uuid::new_v4(),
        etablissement_id: facture.etablissement_id,
        foyer_id: facture.foyer_id,
        facture_id: facture.id,
        encaissement_id: enc.id,
        numero,
        montant_centimes: enc.montant_centimes,
        moyen: enc.moyen,
        emise_at: enc.encaisse_at,
        created_by_user_id: enc.created_by_user_id,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EcheanceCouverture {
    AVenir,
    Payee,
    EnRetard,
}

impl EcheanceCouverture {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::AVenir => "a_venir",
            Self::Payee => "payee",
            Self::EnRetard => "en_retard",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Echeance {
    pub id: Uuid,
    pub etablissement_id: Uuid,
    pub facture_id: Uuid,
    pub rang: i32,
    pub libelle: String,
    /// Montant dû famille (net) pour cette échéance.
    pub montant_centimes: i64,
    /// Part de bourse imputée à cette échéance (somme = déductions facture).
    pub bourse_centimes: i64,
    pub date_echeance: NaiveDate,
    pub created_at: DateTime<Utc>,
}

impl Echeance {
    /// Brut de période = net dû + bourse imputée.
    pub fn montant_brut_centimes(&self) -> i64 {
        self.montant_centimes.saturating_add(self.bourse_centimes)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EcheanceVue {
    pub echeance: Echeance,
    pub couverture: EcheanceCouverture,
    pub couvert_centimes: i64,
}

#[derive(Debug, Clone)]
pub struct EcheanceCmd {
    pub libelle: String,
    pub montant_centimes: i64,
    pub date_echeance: NaiveDate,
}

#[derive(Debug, Clone)]
pub struct PoserEcheancier {
    pub etablissement_id: Uuid,
    pub facture_id: Uuid,
    pub echeances: Vec<EcheanceCmd>,
}

/// Pose un échéancier sur facture émise/soldée. Somme des montants = total facture.
pub fn plan_poser_echeancier(
    facture: &Facture,
    cmd: PoserEcheancier,
) -> Result<Vec<Echeance>, DomainError> {
    if facture.etablissement_id != cmd.etablissement_id || facture.id != cmd.facture_id {
        return Err(DomainError::TenantMismatch);
    }
    if facture.statut == FactureStatut::Brouillon {
        return Err(DomainError::FactureNonEmise);
    }
    if cmd.echeances.is_empty() {
        return Err(DomainError::EcheancierInvalide);
    }
    let now = Utc::now();
    let mut out = Vec::with_capacity(cmd.echeances.len());
    let mut sum: i64 = 0;
    let mut prev_date: Option<NaiveDate> = None;
    for (i, e) in cmd.echeances.into_iter().enumerate() {
        let lib = e.libelle.trim().to_string();
        if lib.is_empty() || e.montant_centimes <= 0 {
            return Err(DomainError::EcheancierInvalide);
        }
        if let Some(prev) = prev_date {
            if e.date_echeance < prev {
                return Err(DomainError::EcheancierInvalide);
            }
        }
        prev_date = Some(e.date_echeance);
        sum = sum.saturating_add(e.montant_centimes);
        out.push(Echeance {
            id: Uuid::new_v4(),
            etablissement_id: cmd.etablissement_id,
            facture_id: cmd.facture_id,
            rang: (i as i32) + 1,
            libelle: lib,
            montant_centimes: e.montant_centimes,
            bourse_centimes: 0,
            date_echeance: e.date_echeance,
            created_at: now,
        });
    }
    if sum != facture.total_centimes {
        return Err(DomainError::EcheancierSommeIncorrecte);
    }
    Ok(out)
}

/// Somme absolue des lignes négatives (déductions : bourses, remises…).
pub fn total_deductions_lignes(lignes: &[FactureLigne]) -> i64 {
    lignes
        .iter()
        .map(|l| l.total_centimes())
        .filter(|&t| t < 0)
        .map(i64::abs)
        .sum()
}

/// Répartit `bourse_totale` proportionnellement aux montants d'échéances (reste sur la dernière).
/// Précondition : échéances déjà posées (somme = total net facture).
pub fn ventiler_bourse_sur_echeances(
    echeances: &mut [Echeance],
    bourse_totale: i64,
) -> Result<(), DomainError> {
    if bourse_totale < 0 {
        return Err(DomainError::EcheancierInvalide);
    }
    if echeances.is_empty() {
        return if bourse_totale == 0 {
            Ok(())
        } else {
            Err(DomainError::EcheancierInvalide)
        };
    }
    if bourse_totale == 0 {
        for e in echeances.iter_mut() {
            e.bourse_centimes = 0;
        }
        return Ok(());
    }
    let sum_montants: i64 = echeances.iter().map(|e| e.montant_centimes).sum();
    if sum_montants <= 0 {
        return Err(DomainError::EcheancierInvalide);
    }
    let mut alloue: i64 = 0;
    let last = echeances.len() - 1;
    for (i, e) in echeances.iter_mut().enumerate() {
        let part = if i == last {
            bourse_totale.saturating_sub(alloue)
        } else {
            // Arrondi inférieur ; le reste tombe sur la dernière.
            (bourse_totale.saturating_mul(e.montant_centimes)) / sum_montants
        };
        e.bourse_centimes = part.max(0);
        alloue = alloue.saturating_add(e.bourse_centimes);
    }
    let check: i64 = echeances.iter().map(|e| e.bourse_centimes).sum();
    if check != bourse_totale {
        return Err(DomainError::EcheancierInvalide);
    }
    Ok(())
}

/// Couverture des échéances (ordre rang) à partir du total encaissé.
pub fn couvrir_echeances(
    echeances: &[Echeance],
    encaisse_centimes: i64,
    today: NaiveDate,
) -> Vec<EcheanceVue> {
    let mut reste_pool = encaisse_centimes.max(0);
    let mut vues = Vec::with_capacity(echeances.len());
    for e in echeances {
        let couvert = reste_pool.min(e.montant_centimes);
        reste_pool = reste_pool.saturating_sub(couvert);
        let couverture = if couvert >= e.montant_centimes {
            EcheanceCouverture::Payee
        } else if e.date_echeance < today {
            EcheanceCouverture::EnRetard
        } else {
            EcheanceCouverture::AVenir
        };
        vues.push(EcheanceVue {
            echeance: e.clone(),
            couverture,
            couvert_centimes: couvert,
        });
    }
    vues
}

/// Découpe le total en N parts égales (reste sur la dernière) aux dates fournies.
pub fn plan_echeancier_parts_egales(
    facture: &Facture,
    dates: Vec<NaiveDate>,
) -> Result<Vec<Echeance>, DomainError> {
    if dates.is_empty() || facture.total_centimes <= 0 {
        return Err(DomainError::EcheancierInvalide);
    }
    let n = dates.len() as i64;
    let base = facture.total_centimes / n;
    let mut reste = facture.total_centimes - base * n;
    let mut cmds = Vec::with_capacity(dates.len());
    for (i, d) in dates.into_iter().enumerate() {
        let mut m = base;
        if i == 0 {
            m = m.saturating_add(reste);
            reste = 0;
        }
        cmds.push(EcheanceCmd {
            libelle: format!("Échéance {}", i + 1),
            montant_centimes: m,
            date_echeance: d,
        });
    }
    plan_poser_echeancier(
        facture,
        PoserEcheancier {
            etablissement_id: facture.etablissement_id,
            facture_id: facture.id,
            echeances: cmds,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creer_calcule_total() {
        let (f, lignes) = plan_creer_facture(CreerFacture {
            etablissement_id: Uuid::new_v4(),
            foyer_id: Uuid::new_v4(),
            numero: "F-001".into(),
            libelle: "Octobre".into(),
            nature: None,
            annee_scolaire: None,
            lignes: vec![
                LigneFactureCmd {
                    eleve_id: None,
                    libelle: "Cantine".into(),
                    quantite: 20,
                    montant_unitaire_centimes: 450,
                },
                LigneFactureCmd {
                    eleve_id: None,
                    libelle: "Bourse".into(),
                    quantite: 1,
                    montant_unitaire_centimes: -2000,
                },
            ],
            actor_user_id: Uuid::new_v4(),
        })
        .unwrap();
        assert_eq!(lignes.len(), 2);
        assert_eq!(f.total_centimes, 20 * 450 - 2000);
        assert_eq!(f.statut, FactureStatut::Brouillon);
    }

    #[test]
    fn emettre_ok() {
        let (f, _) = plan_creer_facture(CreerFacture {
            etablissement_id: Uuid::new_v4(),
            foyer_id: Uuid::new_v4(),
            numero: "F-002".into(),
            libelle: "Nov".into(),
            nature: None,
            annee_scolaire: None,
            lignes: vec![LigneFactureCmd {
                eleve_id: None,
                libelle: "Scolarité".into(),
                quantite: 1,
                montant_unitaire_centimes: 10000,
            }],
            actor_user_id: Uuid::new_v4(),
        })
        .unwrap();
        let e = plan_emettre_facture(&f).unwrap();
        assert_eq!(e.statut, FactureStatut::Emise);
        assert!(e.emise_at.is_some());
    }

    #[test]
    fn encaisser_partiel_puis_solde() {
        let (f, _) = plan_creer_facture(CreerFacture {
            etablissement_id: Uuid::new_v4(),
            foyer_id: Uuid::new_v4(),
            numero: "F-003".into(),
            libelle: "Déc".into(),
            nature: None,
            annee_scolaire: None,
            lignes: vec![LigneFactureCmd {
                eleve_id: None,
                libelle: "Scolarité".into(),
                quantite: 1,
                montant_unitaire_centimes: 10000,
            }],
            actor_user_id: Uuid::new_v4(),
        })
        .unwrap();
        let emise = plan_emettre_facture(&f).unwrap();
        let (e1, f1) = plan_encaisser(
            &emise,
            0,
            EncaisserFacture {
                etablissement_id: emise.etablissement_id,
                facture_id: emise.id,
                montant_centimes: 4000,
                moyen: MoyenPaiement::Cheque,
                reference: Some("CHQ-1".into()),
                actor_user_id: Uuid::new_v4(),
            },
        )
        .unwrap();
        assert_eq!(e1.montant_centimes, 4000);
        assert_eq!(f1.statut, FactureStatut::Emise);
        let (_e2, f2) = plan_encaisser(
            &f1,
            4000,
            EncaisserFacture {
                etablissement_id: emise.etablissement_id,
                facture_id: emise.id,
                montant_centimes: 6000,
                moyen: MoyenPaiement::Virement,
                reference: None,
                actor_user_id: Uuid::new_v4(),
            },
        )
        .unwrap();
        assert_eq!(f2.statut, FactureStatut::Soldee);
        assert!(f2.soldee_at.is_some());
        assert_eq!(reste_a_payer(10000, 10000), 0);
    }

    #[test]
    fn refuse_depassement() {
        let (f, _) = plan_creer_facture(CreerFacture {
            etablissement_id: Uuid::new_v4(),
            foyer_id: Uuid::new_v4(),
            numero: "F-004".into(),
            libelle: "Jan".into(),
            nature: None,
            annee_scolaire: None,
            lignes: vec![LigneFactureCmd {
                eleve_id: None,
                libelle: "X".into(),
                quantite: 1,
                montant_unitaire_centimes: 1000,
            }],
            actor_user_id: Uuid::new_v4(),
        })
        .unwrap();
        let emise = plan_emettre_facture(&f).unwrap();
        let err = plan_encaisser(
            &emise,
            0,
            EncaisserFacture {
                etablissement_id: emise.etablissement_id,
                facture_id: emise.id,
                montant_centimes: 1001,
                moyen: MoyenPaiement::Especes,
                reference: None,
                actor_user_id: Uuid::new_v4(),
            },
        )
        .unwrap_err();
        assert_eq!(err, DomainError::EncaissementDepasse);
    }

    #[test]
    fn echeancier_parts_et_couverture() {
        let (f, _) = plan_creer_facture(CreerFacture {
            etablissement_id: Uuid::new_v4(),
            foyer_id: Uuid::new_v4(),
            numero: "F-005".into(),
            libelle: "Trim".into(),
            nature: None,
            annee_scolaire: None,
            lignes: vec![LigneFactureCmd {
                eleve_id: None,
                libelle: "Scolarité".into(),
                quantite: 1,
                montant_unitaire_centimes: 10001,
            }],
            actor_user_id: Uuid::new_v4(),
        })
        .unwrap();
        let emise = plan_emettre_facture(&f).unwrap();
        let d1 = NaiveDate::from_ymd_opt(2026, 10, 1).unwrap();
        let d2 = NaiveDate::from_ymd_opt(2026, 11, 1).unwrap();
        let d3 = NaiveDate::from_ymd_opt(2026, 12, 1).unwrap();
        let ec = plan_echeancier_parts_egales(&emise, vec![d1, d2, d3]).unwrap();
        assert_eq!(ec.len(), 3);
        assert_eq!(
            ec.iter().map(|e| e.montant_centimes).sum::<i64>(),
            10001
        );
        let today = NaiveDate::from_ymd_opt(2026, 10, 15).unwrap();
        let vues = couvrir_echeances(&ec, 4000, today);
        assert_eq!(vues[0].couverture, EcheanceCouverture::Payee);
        assert_eq!(vues[1].couverture, EcheanceCouverture::AVenir);
        // 1re échéance passée et non couverte → retard
        let vues2 = couvrir_echeances(&ec, 0, today);
        assert_eq!(vues2[0].couverture, EcheanceCouverture::EnRetard);
    }

    #[test]
    fn ventilation_bourse_proportionnelle_reste_derniere() {
        let (f, lignes) = plan_creer_facture(CreerFacture {
            etablissement_id: Uuid::new_v4(),
            foyer_id: Uuid::new_v4(),
            numero: "F-BOURSE".into(),
            libelle: "Cantine".into(),
            nature: None,
            annee_scolaire: None,
            lignes: vec![
                LigneFactureCmd {
                    eleve_id: None,
                    libelle: "Cantine".into(),
                    quantite: 20,
                    montant_unitaire_centimes: 450,
                },
                LigneFactureCmd {
                    eleve_id: None,
                    libelle: "Bourse".into(),
                    quantite: 1,
                    montant_unitaire_centimes: -2000,
                },
            ],
            actor_user_id: Uuid::new_v4(),
        })
        .unwrap();
        assert_eq!(f.total_centimes, 7000);
        let bourse = total_deductions_lignes(&lignes);
        assert_eq!(bourse, 2000);
        let emise = plan_emettre_facture(&f).unwrap();
        let d1 = NaiveDate::from_ymd_opt(2026, 10, 1).unwrap();
        let d2 = NaiveDate::from_ymd_opt(2026, 11, 1).unwrap();
        let d3 = NaiveDate::from_ymd_opt(2026, 12, 1).unwrap();
        let mut ec = plan_echeancier_parts_egales(&emise, vec![d1, d2, d3]).unwrap();
        ventiler_bourse_sur_echeances(&mut ec, bourse).unwrap();
        assert_eq!(ec.iter().map(|e| e.bourse_centimes).sum::<i64>(), 2000);
        assert_eq!(
            ec.iter().map(|e| e.montant_brut_centimes()).sum::<i64>(),
            9000
        );
        // Parts égales : reste centimes sur la 1ʳᵉ → 2334/2333/2333 ; bourse floor + reste dernière
        assert_eq!(ec[0].montant_centimes, 2334);
        assert_eq!(ec[1].montant_centimes, 2333);
        assert_eq!(ec[2].montant_centimes, 2333);
        assert_eq!(ec[0].bourse_centimes, (2000 * 2334) / 7000);
        assert_eq!(ec[1].bourse_centimes, (2000 * 2333) / 7000);
        assert_eq!(
            ec[2].bourse_centimes,
            2000 - ec[0].bourse_centimes - ec[1].bourse_centimes
        );
        ventiler_bourse_sur_echeances(&mut ec, 0).unwrap();
        assert!(ec.iter().all(|e| e.bourse_centimes == 0));
    }

    #[test]
    fn ligne_cantine_depuis_passages_badge() {
        let eleve = Uuid::new_v4();
        let l = plan_ligne_cantine_depuis_passages(eleve, 4, 450, None).unwrap();
        assert_eq!(l.quantite, 4);
        assert_eq!(l.montant_unitaire_centimes, 450);
        assert_eq!(l.eleve_id, Some(eleve));
        assert!(l.libelle.contains("4"));
        assert!(plan_ligne_cantine_depuis_passages(eleve, 0, 450, None).is_err());
        assert!(plan_ligne_cantine_depuis_passages(eleve, 2, 0, None).is_err());
    }

    #[test]
    fn quittance_apres_encaissement() {
        let (f, _) = plan_creer_facture(CreerFacture {
            etablissement_id: Uuid::new_v4(),
            foyer_id: Uuid::new_v4(),
            numero: "F-100".into(),
            libelle: "Test".into(),
            nature: None,
            annee_scolaire: None,
            lignes: vec![LigneFactureCmd {
                eleve_id: None,
                libelle: "X".into(),
                quantite: 1,
                montant_unitaire_centimes: 5000,
            }],
            actor_user_id: Uuid::new_v4(),
        })
        .unwrap();
        let emise = plan_emettre_facture(&f).unwrap();
        let (enc, _) = plan_encaisser(
            &emise,
            0,
            EncaisserFacture {
                etablissement_id: emise.etablissement_id,
                facture_id: emise.id,
                montant_centimes: 2000,
                moyen: MoyenPaiement::Especes,
                reference: None,
                actor_user_id: Uuid::new_v4(),
            },
        )
        .unwrap();
        let q = plan_emettre_quittance(&emise, &enc, 1).unwrap();
        assert_eq!(q.numero, "Q-F-100-01");
        assert_eq!(q.montant_centimes, 2000);
        assert_eq!(q.encaissement_id, enc.id);
    }
}
