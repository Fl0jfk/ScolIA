//! Facturation par événements (socle 2, lot S2-9 ; toile §4.3).
//!
//! | Déclencheur | Effet (pièces **brouillon**, émission humaine) |
//! |---|---|
//! | `inscription.validee` | lignes annuelles de l'année cible (prorata au mois d'entrée), réparties entre les foyers payeurs |
//! | `eleve.regime_changed` | avoir sur la restauration / l'internat qui ne sont plus dus, lignes pour ceux qui le deviennent |
//! | `eleve.sortie` | avoir des mois postérieurs à la sortie |
//! | `eleve.absence_justified` | remise d'ordre (avoir au prorata des jours) si l'absence atteint le seuil paramétré |
//!
//! Consommateur `d10.facturation`, **rejouable** : chaque ligne porte une clé de
//! source unique en base (une source = au plus une ligne par foyer payeur) ; un
//! rejeu, une double livraison ou la génération annuelle faite après coup ne
//! doublent rien. Données relues dans l'établissement de l'événement ; la
//! transaction pose aussi `app.etablissement_id` (RLS des tables de facturation).

use chrono::NaiveDate;
use ent_events::types;
use sqlx::PgConnection;
use std::sync::Arc;
use uuid::Uuid;

use crate::finances;
use crate::outbox::{Consommateur, EvenementDistribue, FutureTraitement};

pub const CONSOMMATEUR_FACTURATION: &str = "d10.facturation";

const TYPES: &[&str] = &[
    types::INSCRIPTION_VALIDEE,
    types::ELEVE_REGIME_CHANGED,
    types::ELEVE_SORTIE,
    types::ELEVE_ABSENCE_JUSTIFIED,
];

pub fn consommateurs_finances() -> Vec<Arc<dyn Consommateur>> {
    vec![Arc::new(Facturation)]
}

struct Facturation;

fn texte<'a>(evt: &'a EvenementDistribue, cle: &str) -> Option<&'a str> {
    evt.event().payload.get(cle).and_then(|v| v.as_str())
}

fn date(evt: &EvenementDistribue, cle: &str) -> Option<NaiveDate> {
    texte(evt, cle).and_then(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d").ok())
}

async fn traiter(tx: &mut PgConnection, evt: &EvenementDistribue) -> Result<(), String> {
    let etab = evt.etablissement_id();
    sqlx::query("SELECT set_config('app.etablissement_id', $1, true)")
        .bind(etab.to_string())
        .execute(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;
    let e = evt.event();
    let causalite = evt.causalite_suivante();
    let err = |e: crate::service::EntDbError| e.to_string();
    match e.type_name.as_str() {
        types::INSCRIPTION_VALIDEE => {
            let (Some(eleve), Some(annee)) = (e.eleve_id, texte(evt, "annee_cible")) else {
                return Ok(());
            };
            // Entrée = jour de la validation : avant la rentrée, année entière.
            let depuis = Some(e.created_at.date_naive());
            // Élève non facturable (pas de payeur, pas de tarif) : rien ; la
            // génération annuelle le signalera en anomalie.
            let _ = finances::facturer_annee_eleve(tx, etab, annee, eleve, depuis, None, causalite)
                .await
                .map_err(err)?;
        }
        types::ELEVE_REGIME_CHANGED => {
            let (Some(eleve), Some(ancien), Some(nouveau), Some(effet)) = (
                e.eleve_id,
                texte(evt, "ancien"),
                texte(evt, "nouveau"),
                date(evt, "date_effet"),
            ) else {
                return Ok(());
            };
            let p = finances::params_tx(tx, etab).await.map_err(err)?;
            let annee = finances::annee_de(&p, effet);
            finances::facturer_changement_regime(
                tx, etab, &annee, eleve, ancien, nouveau, effet, e.id, None, causalite,
            )
            .await
            .map_err(err)?;
        }
        types::ELEVE_SORTIE => {
            let (Some(eleve), Some(d)) = (e.eleve_id, date(evt, "date")) else {
                return Ok(());
            };
            finances::facturer_sortie(tx, etab, eleve, d, e.id, None, causalite)
                .await
                .map_err(err)?;
        }
        types::ELEVE_ABSENCE_JUSTIFIED => {
            let absence: Uuid = e.aggregate_id;
            finances::facturer_absence_prolongee(tx, etab, absence, None, causalite)
                .await
                .map_err(err)?;
        }
        _ => {}
    }
    Ok(())
}

impl Consommateur for Facturation {
    fn nom(&self) -> &'static str {
        CONSOMMATEUR_FACTURATION
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
