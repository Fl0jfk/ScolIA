//! Réactions de l'internat aux événements du cœur (socle 41, lots I0 et I1).
//!
//! | Déclencheur | Effet |
//! |---|---|
//! | `eleve.regime_changed` | séjour d'hébergement resynchronisé sur le régime daté (R-IT1) ; si l'élève n'est plus interne, son affectation ouverte prend fin la veille de la date d'effet ; `internat.regime_effet` publié (avis au responsable de l'internat) |
//!
//! Consommateur `i0.internat`, **rejouable** : la synchronisation des séjours est idempotente et
//! l'affectation n'est close qu'une fois (RI-05, INT-112).

use chrono::{Duration, NaiveDate};
use ent_events::{types, AggregateKind};
use serde_json::json;
use sqlx::PgConnection;
use std::sync::Arc;
use uuid::Uuid;

use crate::finances::publier;
use crate::internat_hebergement::synchroniser_sejours_tx;
use crate::internat_repo::{get_affectation_ouverte_tx, update_affectation_fin_tx};
use crate::outbox::{Consommateur, EvenementDistribue, FutureTraitement};
use domain_eleve::RegimeCode;

pub const CONSOMMATEUR_INTERNAT: &str = "i0.internat";

const TYPES: &[&str] = &[types::ELEVE_REGIME_CHANGED];

pub fn consommateurs_internat() -> Vec<Arc<dyn Consommateur>> {
    vec![Arc::new(Internat)]
}

struct Internat;

fn texte<'a>(evt: &'a EvenementDistribue, cle: &str) -> Option<&'a str> {
    evt.event().payload.get(cle).and_then(|v| v.as_str())
}

async fn traiter(tx: &mut PgConnection, evt: &EvenementDistribue) -> Result<(), String> {
    let etab = evt.etablissement_id();
    sqlx::query("SELECT set_config('app.etablissement_id', $1, true)")
        .bind(etab.to_string())
        .execute(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;
    let e = evt.event();
    if e.type_name != types::ELEVE_REGIME_CHANGED {
        return Ok(());
    }
    let (Some(eleve), Some(nouveau), Some(effet)) = (
        e.eleve_id,
        texte(evt, "nouveau"),
        texte(evt, "date_effet").and_then(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d").ok()),
    ) else {
        return Ok(());
    };
    let err = |e: crate::service::EntDbError| e.to_string();
    let acteur: Option<Uuid> = e.actor_user_id.as_deref().and_then(|a| a.parse().ok());
    synchroniser_sejours_tx(tx, etab, eleve, effet, acteur)
        .await
        .map_err(err)?;
    if RegimeCode::depuis_texte(nouveau).is_some_and(RegimeCode::est_heberge) {
        return Ok(());
    }
    // L'élève n'est plus interne : fin de l'affectation la veille de la date d'effet (au plus tôt
    // le jour de son début), puis avis au responsable.
    let Some(mut aff) = get_affectation_ouverte_tx(tx, etab, eleve)
        .await
        .map_err(|e| e.to_string())?
    else {
        return Ok(());
    };
    aff.fin = Some((effet - Duration::days(1)).max(aff.debut));
    update_affectation_fin_tx(tx, &aff)
        .await
        .map_err(|e| e.to_string())?;
    let causalite = evt.causalite_suivante();
    publier(
        tx,
        etab,
        types::INTERNAT_AFFECTATION,
        AggregateKind::Internat,
        aff.id,
        Some(eleve),
        json!({ "fin": aff.fin.map(|f| f.to_string()), "cloture": true, "motif": "changement_regime" }),
        None,
        causalite,
    )
    .await
    .map_err(err)?;
    publier(
        tx,
        etab,
        types::INTERNAT_REGIME_EFFET,
        AggregateKind::Internat,
        aff.id,
        Some(eleve),
        json!({
            "nouveau": nouveau,
            "date_effet": effet.to_string(),
            "affectation_close": true,
            "chambre_id": aff.chambre_id,
        }),
        None,
        causalite,
    )
    .await
    .map_err(err)?;
    Ok(())
}

impl Consommateur for Internat {
    fn nom(&self) -> &'static str {
        CONSOMMATEUR_INTERNAT
    }

    fn types(&self) -> &'static [&'static str] {
        TYPES
    }

    fn traiter<'a>(
        &'a self,
        tx: &'a mut PgConnection,
        evt: &'a EvenementDistribue,
    ) -> FutureTraitement<'a> {
        Box::pin(traiter(tx, evt))
    }
}
