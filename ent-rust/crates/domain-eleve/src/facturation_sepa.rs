//! Prélèvement SEPA foyer — mandat + planification + export CSV / pain.008 + exécution / rejet.
//! Pas de connecteur banque live : fichiers traçables pour remise PSP.

use crate::facturation::{Facture, FactureStatut, MoyenPaiement};
use crate::DomainError;
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MandatSepaStatut {
    Actif,
    Revoke,
}

impl MandatSepaStatut {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Actif => "actif",
            Self::Revoke => "revoke",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "actif" => Some(Self::Actif),
            "revoke" => Some(Self::Revoke),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PrelevementStatut {
    Prevu,
    Exporte,
    Execute,
    Annule,
    Rejete,
}

impl PrelevementStatut {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Prevu => "prevu",
            Self::Exporte => "exporte",
            Self::Execute => "execute",
            Self::Annule => "annule",
            Self::Rejete => "rejete",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "prevu" => Some(Self::Prevu),
            "exporte" => Some(Self::Exporte),
            "execute" => Some(Self::Execute),
            "annule" => Some(Self::Annule),
            "rejete" => Some(Self::Rejete),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MandatSepa {
    pub id: Uuid,
    pub etablissement_id: Uuid,
    pub foyer_id: Uuid,
    pub rum: String,
    pub iban: String,
    pub bic: Option<String>,
    pub titulaire: String,
    pub statut: MandatSepaStatut,
    pub signe_at: DateTime<Utc>,
    pub created_by_user_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PrelevementSepa {
    pub id: Uuid,
    pub etablissement_id: Uuid,
    pub mandat_id: Uuid,
    pub facture_id: Uuid,
    pub foyer_id: Uuid,
    pub montant_centimes: i64,
    pub date_execution: NaiveDate,
    pub end_to_end_id: String,
    pub statut: PrelevementStatut,
    pub encaissement_id: Option<Uuid>,
    pub motif_rejet: Option<String>,
    pub rejete_at: Option<DateTime<Utc>>,
    pub created_by_user_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct CreerMandatSepa {
    pub etablissement_id: Uuid,
    pub foyer_id: Uuid,
    pub rum: String,
    pub iban: String,
    pub bic: Option<String>,
    pub titulaire: String,
    pub signe_at: DateTime<Utc>,
    pub actor_user_id: Uuid,
}

#[derive(Debug, Clone)]
pub struct PlanifierPrelevement {
    pub etablissement_id: Uuid,
    pub facture_id: Uuid,
    pub mandat_id: Uuid,
    pub montant_centimes: i64,
    pub date_execution: NaiveDate,
    pub actor_user_id: Uuid,
}

/// Normalise IBAN (espaces retirés, majuscules) et valide format basique FR/EU.
pub fn normalize_iban(raw: &str) -> Result<String, DomainError> {
    let iban: String = raw
        .chars()
        .filter(|c| !c.is_whitespace())
        .map(|c| c.to_ascii_uppercase())
        .collect();
    if iban.len() < 15 || iban.len() > 34 {
        return Err(DomainError::SepaIbanInvalide);
    }
    if !iban.chars().all(|c| c.is_ascii_alphanumeric()) {
        return Err(DomainError::SepaIbanInvalide);
    }
    let (cc, rest) = iban.split_at(2);
    if !cc.chars().all(|c| c.is_ascii_alphabetic()) {
        return Err(DomainError::SepaIbanInvalide);
    }
    let (check, body) = rest.split_at(2);
    if !check.chars().all(|c| c.is_ascii_digit()) || body.is_empty() {
        return Err(DomainError::SepaIbanInvalide);
    }
    Ok(iban)
}

fn normalize_rum(raw: &str) -> Result<String, DomainError> {
    let rum = raw.trim().to_string();
    if rum.is_empty() || rum.len() > 35 {
        return Err(DomainError::SepaMandatInvalide);
    }
    Ok(rum)
}

fn normalize_bic(raw: Option<&str>) -> Result<Option<String>, DomainError> {
    let Some(s) = raw.map(str::trim).filter(|s| !s.is_empty()) else {
        return Ok(None);
    };
    let bic: String = s.chars().map(|c| c.to_ascii_uppercase()).collect();
    if !(bic.len() == 8 || bic.len() == 11) || !bic.chars().all(|c| c.is_ascii_alphanumeric()) {
        return Err(DomainError::SepaMandatInvalide);
    }
    Ok(Some(bic))
}

pub fn plan_creer_mandat_sepa(cmd: CreerMandatSepa) -> Result<MandatSepa, DomainError> {
    if cmd.etablissement_id.is_nil() || cmd.foyer_id.is_nil() {
        return Err(DomainError::Event(
            ent_events::EventBusError::MissingEtablissement,
        ));
    }
    let rum = normalize_rum(&cmd.rum)?;
    let iban = normalize_iban(&cmd.iban)?;
    let bic = normalize_bic(cmd.bic.as_deref())?;
    let titulaire = cmd.titulaire.trim().to_string();
    if titulaire.is_empty() || titulaire.len() > 70 {
        return Err(DomainError::SepaMandatInvalide);
    }
    Ok(MandatSepa {
        id: Uuid::new_v4(),
        etablissement_id: cmd.etablissement_id,
        foyer_id: cmd.foyer_id,
        rum,
        iban,
        bic,
        titulaire,
        statut: MandatSepaStatut::Actif,
        signe_at: cmd.signe_at,
        created_by_user_id: Some(cmd.actor_user_id),
        created_at: Utc::now(),
    })
}

pub fn plan_revoquer_mandat(mandat: &MandatSepa) -> Result<MandatSepa, DomainError> {
    if mandat.statut != MandatSepaStatut::Actif {
        return Err(DomainError::SepaMandatInactif);
    }
    let mut m = mandat.clone();
    m.statut = MandatSepaStatut::Revoke;
    Ok(m)
}

pub fn plan_planifier_prelevement(
    facture: &Facture,
    mandat: &MandatSepa,
    deja_encaisse: i64,
    deja_preleve_ouvert: i64,
    cmd: PlanifierPrelevement,
) -> Result<PrelevementSepa, DomainError> {
    if facture.etablissement_id != cmd.etablissement_id
        || mandat.etablissement_id != cmd.etablissement_id
    {
        return Err(DomainError::TenantMismatch);
    }
    if facture.id != cmd.facture_id || mandat.id != cmd.mandat_id {
        return Err(DomainError::NotFound);
    }
    if facture.foyer_id != mandat.foyer_id {
        return Err(DomainError::SepaMandatInvalide);
    }
    if mandat.statut != MandatSepaStatut::Actif {
        return Err(DomainError::SepaMandatInactif);
    }
    match facture.statut {
        FactureStatut::Brouillon => return Err(DomainError::FactureNonEmise),
        FactureStatut::Soldee => return Err(DomainError::FactureDejaSoldee),
        FactureStatut::Emise => {}
    }
    if cmd.montant_centimes <= 0 {
        return Err(DomainError::SepaPrelevementInvalide);
    }
    let engage = deja_encaisse.saturating_add(deja_preleve_ouvert);
    let reste = facture.total_centimes.saturating_sub(engage);
    if cmd.montant_centimes > reste {
        return Err(DomainError::EncaissementDepasse);
    }
    let short = facture.numero.chars().take(12).collect::<String>();
    let end_to_end_id = format!(
        "SEPA-{}-{}",
        short,
        &Uuid::new_v4().to_string()[..8]
    );
    Ok(PrelevementSepa {
        id: Uuid::new_v4(),
        etablissement_id: cmd.etablissement_id,
        mandat_id: mandat.id,
        facture_id: facture.id,
        foyer_id: facture.foyer_id,
        montant_centimes: cmd.montant_centimes,
        date_execution: cmd.date_execution,
        end_to_end_id,
        statut: PrelevementStatut::Prevu,
        encaissement_id: None,
        motif_rejet: None,
        rejete_at: None,
        created_by_user_id: Some(cmd.actor_user_id),
        created_at: Utc::now(),
    })
}

pub fn plan_marquer_exporte(p: &PrelevementSepa) -> Result<PrelevementSepa, DomainError> {
    if p.statut != PrelevementStatut::Prevu {
        return Err(DomainError::SepaPrelevementInvalide);
    }
    let mut out = p.clone();
    out.statut = PrelevementStatut::Exporte;
    Ok(out)
}

/// Prépare l'exécution : statut → execute + référence pour encaissement Prélevement.
pub fn plan_executer_prelevement(
    p: &PrelevementSepa,
    encaissement_id: Uuid,
) -> Result<PrelevementSepa, DomainError> {
    if p.statut != PrelevementStatut::Prevu && p.statut != PrelevementStatut::Exporte {
        return Err(DomainError::SepaPrelevementInvalide);
    }
    if encaissement_id.is_nil() {
        return Err(DomainError::SepaPrelevementInvalide);
    }
    let mut out = p.clone();
    out.statut = PrelevementStatut::Execute;
    out.encaissement_id = Some(encaissement_id);
    Ok(out)
}

/// Rejet banque (ou retour PSP) : libère la réserve sans encaisser.
pub fn plan_rejeter_prelevement(
    p: &PrelevementSepa,
    motif: &str,
) -> Result<PrelevementSepa, DomainError> {
    if p.statut != PrelevementStatut::Prevu && p.statut != PrelevementStatut::Exporte {
        return Err(DomainError::SepaPrelevementInvalide);
    }
    let motif = motif.trim();
    if motif.is_empty() || motif.len() > 140 {
        return Err(DomainError::SepaPrelevementInvalide);
    }
    let mut out = p.clone();
    out.statut = PrelevementStatut::Rejete;
    out.motif_rejet = Some(motif.to_string());
    out.rejete_at = Some(Utc::now());
    Ok(out)
}

/// Identifiant créancier SEPA (ICS) + compte établissement pour pain.008.
#[derive(Debug, Clone)]
pub struct SepaCrediteur {
    pub nom: String,
    pub iban: String,
    pub bic: String,
    pub ics: String,
}

pub fn plan_sepa_crediteur(
    nom: &str,
    iban: &str,
    bic: &str,
    ics: &str,
) -> Result<SepaCrediteur, DomainError> {
    let nom = nom.trim().to_string();
    if nom.is_empty() || nom.len() > 70 {
        return Err(DomainError::SepaMandatInvalide);
    }
    let iban = normalize_iban(iban)?;
    let bic = normalize_bic(Some(bic))?.ok_or(DomainError::SepaMandatInvalide)?;
    let ics = ics.trim().to_ascii_uppercase();
    if ics.len() < 8 || ics.len() > 35 || !ics.chars().all(|c| c.is_ascii_alphanumeric()) {
        return Err(DomainError::SepaMandatInvalide);
    }
    Ok(SepaCrediteur {
        nom,
        iban,
        bic,
        ics,
    })
}

fn xml_escape(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            '&' => "&amp;".to_string(),
            '<' => "&lt;".to_string(),
            '>' => "&gt;".to_string(),
            '"' => "&quot;".to_string(),
            '\'' => "&apos;".to_string(),
            c => c.to_string(),
        })
        .collect()
}

fn euros_xml(centimes: i64) -> String {
    let abs = centimes.unsigned_abs();
    format!("{}.{:02}", abs / 100, abs % 100)
}

/// pain.008.001.02 minimal CORE — un `PmtInf` par (date_exécution, SeqTp).
/// `seq_tp_par_mandat` : `FRST` ou `RCUR` pour chaque mandat_id du lot.
pub fn render_sepa_pain008(
    crediteur: &SepaCrediteur,
    lignes: &[SepaLigneExport],
    msg_id: &str,
    created_at: DateTime<Utc>,
    seq_tp_par_mandat: &std::collections::HashMap<Uuid, &'static str>,
) -> Result<Vec<u8>, DomainError> {
    if lignes.is_empty() {
        return Err(DomainError::SepaPrelevementInvalide);
    }
    let msg_id = msg_id.trim();
    if msg_id.is_empty() || msg_id.len() > 35 {
        return Err(DomainError::SepaPrelevementInvalide);
    }
    let nb = lignes.len();
    let ctrl: i64 = lignes.iter().map(|l| l.prelevement.montant_centimes).sum();

    // Groupes (date, seq) → lignes
    let mut groups: Vec<(NaiveDate, &'static str, Vec<&SepaLigneExport>)> = Vec::new();
    for l in lignes {
        let seq = seq_tp_par_mandat
            .get(&l.prelevement.mandat_id)
            .copied()
            .unwrap_or("RCUR");
        let date = l.prelevement.date_execution;
        if let Some(g) = groups
            .iter_mut()
            .find(|(d, s, _)| *d == date && *s == seq)
        {
            g.2.push(l);
        } else {
            groups.push((date, seq, vec![l]));
        }
    }

    let mut xml = String::from(
        r#"<?xml version="1.0" encoding="UTF-8"?>"#,
    );
    xml.push('\n');
    xml.push_str(
        r#"<Document xmlns="urn:iso:std:iso:20022:tech:xsd:pain.008.001.02">"#,
    );
    xml.push_str("<CstmrDrctDbtInitn>");
    xml.push_str("<GrpHdr>");
    xml.push_str(&format!("<MsgId>{}</MsgId>", xml_escape(msg_id)));
    xml.push_str(&format!(
        "<CreDtTm>{}</CreDtTm>",
        created_at.format("%Y-%m-%dT%H:%M:%S")
    ));
    xml.push_str(&format!("<NbOfTxs>{nb}</NbOfTxs>"));
    xml.push_str(&format!("<CtrlSum>{}</CtrlSum>", euros_xml(ctrl)));
    xml.push_str(&format!(
        "<InitgPty><Nm>{}</Nm></InitgPty>",
        xml_escape(&crediteur.nom)
    ));
    xml.push_str("</GrpHdr>");

    for (i, (date, seq, group)) in groups.iter().enumerate() {
        let g_nb = group.len();
        let g_sum: i64 = group.iter().map(|l| l.prelevement.montant_centimes).sum();
        let pmt_id = format!("{msg_id}-P{i}");
        xml.push_str("<PmtInf>");
        xml.push_str(&format!("<PmtInfId>{}</PmtInfId>", xml_escape(&pmt_id)));
        xml.push_str("<PmtMtd>DD</PmtMtd>");
        xml.push_str(&format!("<NbOfTxs>{g_nb}</NbOfTxs>"));
        xml.push_str(&format!("<CtrlSum>{}</CtrlSum>", euros_xml(g_sum)));
        xml.push_str("<PmtTpInf><SvcLvl><Cd>SEPA</Cd></SvcLvl>");
        xml.push_str("<LclInstrm><Cd>CORE</Cd></LclInstrm>");
        xml.push_str(&format!("<SeqTp>{seq}</SeqTp></PmtTpInf>"));
        xml.push_str(&format!("<ReqdColltnDt>{date}</ReqdColltnDt>"));
        xml.push_str(&format!(
            "<Cdtr><Nm>{}</Nm></Cdtr>",
            xml_escape(&crediteur.nom)
        ));
        xml.push_str(&format!(
            "<CdtrAcct><Id><IBAN>{}</IBAN></Id><Ccy>EUR</Ccy></CdtrAcct>",
            xml_escape(&crediteur.iban)
        ));
        xml.push_str(&format!(
            "<CdtrAgt><FinInstnId><BIC>{}</BIC></FinInstnId></CdtrAgt>",
            xml_escape(&crediteur.bic)
        ));
        xml.push_str("<ChrgBr>SLEV</ChrgBr>");
        xml.push_str(&format!(
            "<CdtrSchmeId><Id><PrvtId><Othr><Id>{}</Id><SchmeNm><Prtry>SEPA</Prtry></SchmeNm></Othr></PrvtId></Id></CdtrSchmeId>",
            xml_escape(&crediteur.ics)
        ));
        for l in group {
            let p = &l.prelevement;
            let dbtr_bic = l.bic.as_deref().unwrap_or("NOTPROVIDED");
            xml.push_str("<DrctDbtTxInf>");
            xml.push_str(&format!(
                "<PmtId><EndToEndId>{}</EndToEndId></PmtId>",
                xml_escape(&p.end_to_end_id)
            ));
            xml.push_str(&format!(
                "<InstdAmt Ccy=\"EUR\">{}</InstdAmt>",
                euros_xml(p.montant_centimes)
            ));
            xml.push_str("<DrctDbtTx><MndtRltdInf>");
            xml.push_str(&format!("<MndtId>{}</MndtId>", xml_escape(&l.rum)));
            xml.push_str(&format!(
                "<DtOfSgntr>{}</DtOfSgntr>",
                // signe_at not on export line — use created_at of prelevement as fallback; better pass signe
                // We'll extend SepaLigneExport with mandat_signe_at
                l.mandat_signe_at.format("%Y-%m-%d")
            ));
            xml.push_str("<AmdmntInd>false</AmdmntInd>");
            xml.push_str("</MndtRltdInf></DrctDbtTx>");
            xml.push_str(&format!(
                "<DbtrAgt><FinInstnId><BIC>{}</BIC></FinInstnId></DbtrAgt>",
                xml_escape(dbtr_bic)
            ));
            xml.push_str(&format!(
                "<Dbtr><Nm>{}</Nm></Dbtr>",
                xml_escape(&l.titulaire)
            ));
            xml.push_str(&format!(
                "<DbtrAcct><Id><IBAN>{}</IBAN></Id></DbtrAcct>",
                xml_escape(&l.iban)
            ));
            xml.push_str(&format!(
                "<RmtInf><Ustrd>{}</Ustrd></RmtInf>",
                xml_escape(&l.facture_numero)
            ));
            xml.push_str("</DrctDbtTxInf>");
        }
        xml.push_str("</PmtInf>");
    }

    xml.push_str("</CstmrDrctDbtInitn></Document>\n");
    Ok(xml.into_bytes())
}

fn csv_escape(s: &str) -> String {
    if s.contains(';') || s.contains('"') || s.contains('\n') || s.contains('\r') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

fn euros_fr(centimes: i64) -> String {
    let abs = centimes.unsigned_abs();
    format!("{},{:02}", abs / 100, abs % 100)
}

/// Ligne plate pour export banque (jointure mandat + facture côté appelant).
#[derive(Debug, Clone)]
pub struct SepaLigneExport {
    pub prelevement: PrelevementSepa,
    pub rum: String,
    pub iban: String,
    pub bic: Option<String>,
    pub titulaire: String,
    pub mandat_signe_at: DateTime<Utc>,
    pub facture_numero: String,
}

pub fn render_sepa_csv(lignes: &[SepaLigneExport]) -> Vec<u8> {
    let mut out = String::from("\u{FEFF}");
    out.push_str(
        "date_execution;rum;iban;bic;titulaire;end_to_end_id;montant_eur;facture;foyer_id\n",
    );
    for l in lignes {
        let p = &l.prelevement;
        out.push_str(&format!(
            "{};{};{};{};{};{};{};{};{}\n",
            p.date_execution,
            csv_escape(&l.rum),
            csv_escape(&l.iban),
            csv_escape(l.bic.as_deref().unwrap_or("")),
            csv_escape(&l.titulaire),
            csv_escape(&p.end_to_end_id),
            euros_fr(p.montant_centimes),
            csv_escape(&l.facture_numero),
            p.foyer_id,
        ));
    }
    out.into_bytes()
}

/// Moyen paiement pour l'encaissement issu d'un prélèvement.
pub fn moyen_prelevement() -> MoyenPaiement {
    MoyenPaiement::Prelevement
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::facturation::FactureStatut;
    use chrono::TimeZone;

    fn facture_emise(foyer: Uuid, total: i64) -> Facture {
        Facture {
            id: Uuid::new_v4(),
            etablissement_id: Uuid::new_v4(),
            foyer_id: foyer,
            numero: "F-SEPA-1".into(),
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

    #[test]
    fn iban_fr_ok() {
        let iban = normalize_iban("FR76 3000 6000 0112 3456 7890 189").unwrap();
        assert!(iban.starts_with("FR76"));
        assert!(!iban.contains(' '));
    }

    #[test]
    fn iban_trop_court() {
        assert!(normalize_iban("FR7612").is_err());
    }

    #[test]
    fn planifier_refuse_si_mandat_autre_foyer() {
        let foyer = Uuid::new_v4();
        let f = facture_emise(foyer, 5000);
        let mandat = MandatSepa {
            id: Uuid::new_v4(),
            etablissement_id: f.etablissement_id,
            foyer_id: Uuid::new_v4(),
            rum: "RUM-1".into(),
            iban: "FR7630006000011234567890189".into(),
            bic: None,
            titulaire: "Dupont".into(),
            statut: MandatSepaStatut::Actif,
            signe_at: Utc::now(),
            created_by_user_id: None,
            created_at: Utc::now(),
        };
        let err = plan_planifier_prelevement(
            &f,
            &mandat,
            0,
            0,
            PlanifierPrelevement {
                etablissement_id: f.etablissement_id,
                facture_id: f.id,
                mandat_id: mandat.id,
                montant_centimes: 1000,
                date_execution: NaiveDate::from_ymd_opt(2026, 10, 20).unwrap(),
                actor_user_id: Uuid::new_v4(),
            },
        )
        .unwrap_err();
        assert_eq!(err, DomainError::SepaMandatInvalide);
    }

    #[test]
    fn csv_export_ligne() {
        let p = PrelevementSepa {
            id: Uuid::new_v4(),
            etablissement_id: Uuid::new_v4(),
            mandat_id: Uuid::new_v4(),
            facture_id: Uuid::new_v4(),
            foyer_id: Uuid::nil(),
            montant_centimes: 4250,
            date_execution: NaiveDate::from_ymd_opt(2026, 10, 15).unwrap(),
            end_to_end_id: "SEPA-F1-abcd".into(),
            statut: PrelevementStatut::Prevu,
            encaissement_id: None,
            motif_rejet: None,
            rejete_at: None,
            created_by_user_id: None,
            created_at: Utc.with_ymd_and_hms(2026, 10, 1, 0, 0, 0).unwrap(),
        };
        let ligne = SepaLigneExport {
            prelevement: p.clone(),
            rum: "RUM-X".into(),
            iban: "FR7630006000011234567890189".into(),
            bic: Some("AGRIFRPP".into()),
            titulaire: "Lily".into(),
            mandat_signe_at: Utc.with_ymd_and_hms(2026, 1, 15, 0, 0, 0).unwrap(),
            facture_numero: "F-1".into(),
        };
        let csv = String::from_utf8(render_sepa_csv(&[ligne.clone()])).unwrap();
        assert!(csv.contains("date_execution;rum"));
        assert!(csv.contains("42,50"));
        assert!(csv.contains("SEPA-F1-abcd"));

        let cdtr = plan_sepa_crediteur(
            "College Hogwarts",
            "FR76 1010 7001 0112 3456 7890 129",
            "CCBPFRPP",
            "FR72ZZZ123456",
        )
        .unwrap();
        let mut seq = std::collections::HashMap::new();
        seq.insert(p.mandat_id, "FRST");
        let xml = String::from_utf8(
            render_sepa_pain008(
                &cdtr,
                &[ligne],
                "MSG-LAB-001",
                Utc.with_ymd_and_hms(2026, 10, 13, 9, 0, 0).unwrap(),
                &seq,
            )
            .unwrap(),
        )
        .unwrap();
        assert!(xml.contains("pain.008.001.02"));
        assert!(xml.contains("<SeqTp>FRST</SeqTp>"));
        assert!(xml.contains("SEPA-F1-abcd"));
        assert!(xml.contains("42.50"));
        assert!(xml.contains("FR72ZZZ123456"));

        let rej = plan_rejeter_prelevement(&p, "AC04").unwrap();
        assert_eq!(rej.statut, PrelevementStatut::Rejete);
        assert_eq!(rej.motif_rejet.as_deref(), Some("AC04"));
    }
}
