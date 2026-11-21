//! Connecteur banque ENT — rapprochement des prélèvements SEPA.
//!
//! Format CSV (séparateur `;`, 1re ligne = en-tête optionnelle) :
//! `end_to_end_id;montant_centimes;result;motif`
//! - result = `settled` | `rejected`
//! - motif = code rejet (ex. AC04) si rejected
//!
//! Lab : dry-run (`ENT_BANK_DRY_RUN=1`) — parse + match, pas d'API bancaire.
//! Live : `ENT_BANK_LIVE=1` + secrets provider (branchement ultérieur).

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BankLineResult {
    Settled,
    Rejected,
}

impl BankLineResult {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Settled => "settled",
            Self::Rejected => "rejected",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "settled" | "ok" | "execute" | "executed" => Some(Self::Settled),
            "rejected" | "reject" | "rejet" => Some(Self::Rejected),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BankStatementLine {
    pub end_to_end_id: String,
    pub montant_centimes: i64,
    pub result: BankLineResult,
    pub motif: Option<String>,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum BankParseError {
    #[error("ligne CSV vide")]
    Empty,
    #[error("colonnes insuffisantes (attendu end_to_end_id;montant;result[;motif])")]
    Columns,
    #[error("montant invalide")]
    Amount,
    #[error("resultat inconnu (settled|rejected)")]
    Result,
    #[error("end_to_end_id vide")]
    EmptyE2e,
}

/// Parse un relevé CSV (ignore lignes vides et en-tête si 1re col = end_to_end_id).
pub fn parse_releve_csv(raw: &str) -> Result<Vec<BankStatementLine>, BankParseError> {
    let mut out = Vec::new();
    for (i, line) in raw.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let parts: Vec<&str> = line.split(';').map(str::trim).collect();
        if parts.len() < 3 {
            return Err(BankParseError::Columns);
        }
        // Skip header
        if i == 0
            && (parts[0].eq_ignore_ascii_case("end_to_end_id")
                || parts[0].eq_ignore_ascii_case("e2e"))
        {
            continue;
        }
        let e2e = parts[0].to_string();
        if e2e.is_empty() {
            return Err(BankParseError::EmptyE2e);
        }
        let montant: i64 = parts[1]
            .replace(',', ".")
            .parse()
            .map_err(|_| BankParseError::Amount)?;
        if montant <= 0 {
            return Err(BankParseError::Amount);
        }
        let result = BankLineResult::parse(parts[2]).ok_or(BankParseError::Result)?;
        let motif = parts.get(3).and_then(|s| {
            let t = s.trim();
            if t.is_empty() {
                None
            } else {
                Some(t.to_string())
            }
        });
        if result == BankLineResult::Rejected && motif.is_none() {
            // motif défaut générique
        }
        out.push(BankStatementLine {
            end_to_end_id: e2e,
            montant_centimes: montant,
            result,
            motif,
        });
    }
    if out.is_empty() {
        return Err(BankParseError::Empty);
    }
    Ok(out)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BankMatchAction {
    Execute,
    Reject,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BankMatch {
    pub line: BankStatementLine,
    pub action: BankMatchAction,
}

/// Décide l'action métier pour une ligne (hors lookup SQL).
pub fn plan_action_ligne(line: &BankStatementLine) -> BankMatch {
    let action = match line.result {
        BankLineResult::Settled => BankMatchAction::Execute,
        BankLineResult::Rejected => BankMatchAction::Reject,
    };
    BankMatch {
        line: line.clone(),
        action,
    }
}

#[derive(Clone, Default)]
pub struct BankConnector {
    dry_run: bool,
    live: bool,
    ingest_count: std::sync::Arc<std::sync::atomic::AtomicUsize>,
}

impl BankConnector {
    pub fn from_env() -> Self {
        Self {
            dry_run: env_flag("ENT_BANK_DRY_RUN"),
            live: env_flag("ENT_BANK_LIVE"),
            ingest_count: std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0)),
        }
    }

    pub fn dry_run_only() -> Self {
        Self {
            dry_run: true,
            live: false,
            ingest_count: std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0)),
        }
    }

    pub fn is_ready(&self) -> bool {
        self.dry_run || self.live
    }

    pub fn is_live(&self) -> bool {
        self.live && !self.dry_run
    }

    pub fn ingest_count(&self) -> usize {
        self.ingest_count
            .load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Ingest relevé — dry-run = parse only ; live = stub (pas encore d'API banque).
    pub fn ingest_csv(&self, raw: &str) -> Result<Vec<BankStatementLine>, BankParseError> {
        let lines = parse_releve_csv(raw)?;
        self.ingest_count
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        if self.is_live() {
            // Provider live (PSD2 / Open Banking) — à brancher.
            return Ok(lines);
        }
        Ok(lines)
    }
}

fn env_flag(name: &str) -> bool {
    match std::env::var(name) {
        Ok(v) => {
            let t = v.trim();
            t == "1" || t.eq_ignore_ascii_case("true") || t.eq_ignore_ascii_case("yes")
        }
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_settled_rejected() {
        let csv = "end_to_end_id;montant_centimes;result;motif\n\
SEPA-A;8000;settled;\n\
SEPA-B;2500;rejected;AC04\n";
        let lines = parse_releve_csv(csv).unwrap();
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].result, BankLineResult::Settled);
        assert_eq!(lines[1].motif.as_deref(), Some("AC04"));
        assert_eq!(plan_action_ligne(&lines[0]).action, BankMatchAction::Execute);
        assert_eq!(plan_action_ligne(&lines[1]).action, BankMatchAction::Reject);
    }
}
