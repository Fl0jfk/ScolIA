//! **Port de reprise du module RGPD de Next.js** (socle 27, §19, D-035) — **posé, non branché**.
//!
//! La reprise (incidents, documents validés, identité du DPO) dépend de la mise en service du socle 28 (lot
//! MS2 : portes et reprise réelle) ; elle n'est pas prête. Ce fichier ne fait que **fixer le contrat** pour que
//! G5 (violations) et G9 (questionnaire) puissent s'y brancher :
//!
//! * la source est lue par un **port** ([`SourceRepriseRgpd`]), jamais par une connexion directe à la production
//!   Next.js (interdit : aucun accès à la base de production, même en lecture) ; la mise en service fournira une
//!   implémentation sur un export anonymisé ou autorisé ;
//! * l'historique de conversation IA **n'est pas repris** (données personnelles possibles), le score non plus ;
//! * les sous-traitants déclarés dans l'ancien outil sont **listés au rapport**, pas repris comme sous-traitants ScolIA.
//!
//! Tant qu'aucune implémentation n'est fournie, [`PgEnt::reprendre_rgpd`] répond `PortNonBranche` : **rien n'est
//! lu, rien n'est écrit**.

use crate::service::{EntDbError, PgEnt};
use chrono::NaiveDate;
use ent_auth::Action;
use serde::Serialize;
use uuid::Uuid;

/// Un incident de l'ancien outil (champs repris : faits, dates ; pas de conversation IA).
#[derive(Clone, Debug)]
pub struct IncidentRepris {
    pub reference_source: String,
    pub detecte_le: NaiveDate,
    pub resume: String,
    pub notifie_cnil: bool,
}

/// Un document validé de l'ancien outil (référence et empreinte, jamais le contenu dans le port).
#[derive(Clone, Debug)]
pub struct DocumentRepris {
    pub code: String,
    pub empreinte_sha256: String,
}

/// Contrat de la source de reprise. Sans implémentation fournie, rien ne se passe.
pub trait SourceRepriseRgpd {
    fn incidents(&self) -> Vec<IncidentRepris>;
    fn documents(&self) -> Vec<DocumentRepris>;
    /// Sous-traitants déclarés dans l'ancien outil : portés au rapport, jamais importés.
    fn soustraitants_declares(&self) -> Vec<String>;
}

/// Rapport de reprise (en comptes ; aucun contenu).
#[derive(Clone, Debug, Default, Serialize)]
pub struct RapportRepriseRgpd {
    pub incidents: usize,
    pub documents: usize,
    pub soustraitants_non_repris: usize,
}

impl PgEnt {
    /// Reprise du module RGPD de l'ancien outil : **port non branché** (socle 28, MS2). Droit `conformite.gerer`.
    /// Refuse sans rien lire ni écrire.
    pub async fn reprendre_rgpd(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        _source: Option<&dyn SourceRepriseRgpd>,
    ) -> Result<RapportRepriseRgpd, EntDbError> {
        self.exiger_action_etab(etab, acteur, role, Action::ConformiteGerer)
            .await?;
        Err(EntDbError::ConflitDossier(
            "reprise du module RGPD non disponible : le port attend la mise en service (socle 28, MS2)",
        ))
    }
}
