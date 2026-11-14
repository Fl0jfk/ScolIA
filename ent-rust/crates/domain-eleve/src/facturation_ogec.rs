//! Export journal recettes format OGEC (CSV point-virgule).
//! Tranche P1 : encaissements émis → lignes importables en compta établissement.
//! Pas un connecteur Sage : fichier plat traçable, UTF-8 BOM pour Excel FR.

use crate::facturation::MoyenPaiement;
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Ligne plate d'un encaissement prête pour l'export.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OgecLigneExport {
    pub encaisse_at: DateTime<Utc>,
    pub facture_numero: String,
    pub facture_libelle: String,
    pub foyer_id: Uuid,
    pub quittance_numero: Option<String>,
    pub moyen: MoyenPaiement,
    pub reference: Option<String>,
    pub montant_centimes: i64,
    pub nature_facture: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ExporterOgec {
    pub etablissement_id: Uuid,
    pub debut: NaiveDate,
    pub fin: NaiveDate,
    pub actor_user_id: Option<String>,
}

/// Valide la fenêtre d'export (début ≤ fin, plage ≤ 366 jours).
pub fn plan_exporter_ogec(cmd: &ExporterOgec) -> Result<(), crate::DomainError> {
    if cmd.etablissement_id.is_nil() {
        return Err(crate::DomainError::Event(
            ent_events::EventBusError::MissingEtablissement,
        ));
    }
    if cmd.fin < cmd.debut {
        return Err(crate::DomainError::OgecPeriodeInvalide);
    }
    let days = (cmd.fin - cmd.debut).num_days();
    if days > 366 {
        return Err(crate::DomainError::OgecPeriodeInvalide);
    }
    Ok(())
}

fn csv_escape(s: &str) -> String {
    if s.contains(';') || s.contains('"') || s.contains('\n') || s.contains('\r') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

fn euros_fr(centimes: i64) -> String {
    let neg = centimes < 0;
    let abs = centimes.unsigned_abs();
    let euros = abs / 100;
    let cts = abs % 100;
    let s = format!("{euros},{cts:02}");
    if neg {
        format!("-{s}")
    } else {
        s
    }
}

/// CSV UTF-8 avec BOM + en-tête FR (séparateur `;`).
pub fn render_ogec_csv(lignes: &[OgecLigneExport]) -> Vec<u8> {
    let mut out = String::from("\u{FEFF}");
    out.push_str(
        "date;piece;libelle;foyer_id;quittance;moyen;reference;montant_eur;nature\n",
    );
    for l in lignes {
        let date = l.encaisse_at.format("%Y-%m-%d").to_string();
        let piece = csv_escape(&l.facture_numero);
        let lib = csv_escape(&l.facture_libelle);
        let quit = l
            .quittance_numero
            .as_deref()
            .map(csv_escape)
            .unwrap_or_default();
        let moyen = l.moyen.as_str();
        let reference = l
            .reference
            .as_deref()
            .map(csv_escape)
            .unwrap_or_default();
        let nature = l
            .nature_facture
            .as_deref()
            .map(csv_escape)
            .unwrap_or_default();
        out.push_str(&format!(
            "{date};{piece};{lib};{};{quit};{moyen};{reference};{};{nature}\n",
            l.foyer_id,
            euros_fr(l.montant_centimes),
        ));
    }
    out.into_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn csv_contient_entete_et_ligne() {
        let l = OgecLigneExport {
            encaisse_at: Utc.with_ymd_and_hms(2026, 10, 12, 10, 0, 0).unwrap(),
            facture_numero: "F-001".into(),
            facture_libelle: "Cantine; octobre".into(),
            foyer_id: Uuid::nil(),
            quittance_numero: Some("Q-F-001-01".into()),
            moyen: MoyenPaiement::Cheque,
            reference: Some("CHQ-1".into()),
            montant_centimes: 3500,
            nature_facture: None,
        };
        let csv = String::from_utf8(render_ogec_csv(&[l])).unwrap();
        assert!(csv.starts_with('\u{FEFF}') || csv.contains("date;piece"));
        assert!(csv.contains("F-001"));
        assert!(csv.contains("\"Cantine; octobre\""));
        assert!(csv.contains("35,00"));
        assert!(csv.contains("cheque"));
    }

    #[test]
    fn periode_invalide() {
        let cmd = ExporterOgec {
            etablissement_id: Uuid::new_v4(),
            debut: NaiveDate::from_ymd_opt(2026, 12, 1).unwrap(),
            fin: NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
            actor_user_id: None,
        };
        assert!(plan_exporter_ogec(&cmd).is_err());
    }
}
