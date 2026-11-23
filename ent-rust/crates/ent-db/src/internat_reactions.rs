//! Réactions de l'internat aux événements du cœur (socle 41, lots I0 et I1).
//!
//! | Déclencheur | Effet |
//! |---|---|
//! | `eleve.regime_changed` | séjour d'hébergement resynchronisé sur le régime daté (R-IT1) ; si l'élève n'est plus interne, son affectation ouverte prend fin la veille de la date d'effet ; `internat.regime_effet` publié (avis au responsable de l'internat) |
//!
//! Consommateur `i0.internat`, **rejouable** : la synchronisation des séjours est idempotente et
//! l'affectation n'est close qu'une fois (RI-05, INT-112).

use chrono::{Duration, NaiveDate, Utc};
use ent_auth::Action;
use ent_events::{types, AggregateKind};
use serde_json::json;
use sqlx::PgConnection;
use std::sync::Arc;
use uuid::Uuid;

use crate::finances::publier;
use crate::internat_hebergement::synchroniser_sejours_tx;
use crate::internat_repo::{get_affectation_ouverte_tx, update_affectation_fin_tx};
use crate::notifications::{notifier_personne, EnvoiAPlanifier};
use crate::outbox::{Consommateur, EvenementDistribue, FutureTraitement};
use domain_eleve::RegimeCode;

pub const CONSOMMATEUR_INTERNAT: &str = "i0.internat";

const TYPES: &[&str] = &[types::ELEVE_REGIME_CHANGED];

pub fn consommateurs_internat() -> Vec<Arc<dyn Consommateur>> {
    vec![Arc::new(Internat), Arc::new(AlertesInternat)]
}

/// Passe 2 du lot I4 (M3) : les faits qui appellent un humain (cas de nuit ouvert, étude non pointée, malaise) deviennent un
/// avis aux responsables de l'internat (`internat.gerer` : direction, CPE et rôles attribués, relus au traitement).
/// Texte fixe, **aucun nom d'élève** (un élève protégé reste introuvable ailleurs que dans les écrans de l'internat) ;
/// jamais de SMS. Rejouable : la clé unique (événement, destinataire, nature) évite tout doublon.
pub const CONSOMMATEUR_ALERTES_INTERNAT: &str = "i4.internat_alertes";

const TYPES_ALERTES: &[&str] = &[
    types::INTERNAT_CAS_OUVERT,
    types::INTERNAT_ETUDE_NON_POINTEE,
    types::INTERNAT_ALERTE,
];

struct AlertesInternat;

/// Nature, titre et texte de l'avis (fixes) ; `None` : aucun avis.
fn avis_pour(type_name: &str) -> Option<(&'static str, &'static str, &'static str)> {
    match type_name {
        t if t == types::INTERNAT_CAS_OUVERT => Some((
            "internat_cas_ouvert",
            "Cas de nuit ouvert",
            "Un cas de nuit vient d'être ouvert à l'internat : ouvrez la liste des cas pour le suivre.",
        )),
        t if t == types::INTERNAT_ETUDE_NON_POINTEE => Some((
            "internat_etude_non_pointee",
            "Étude : un inscrit n'est pas à sa place",
            "Un inscrit à l'étude n'a pas été pointé ou est noté absent : ouvrez le temps du soir pour le suivre.",
        )),
        t if t == types::INTERNAT_ALERTE => Some((
            "internat_alerte",
            "Alerte à l'internat",
            "Une alerte a été déclarée à l'internat : ouvrez les événements de la nuit.",
        )),
        _ => None,
    }
}

async fn traiter_alertes(tx: &mut PgConnection, evt: &EvenementDistribue) -> Result<(), String> {
    let etab = evt.etablissement_id();
    let e = evt.event();
    let Some((nature, titre, corps)) = avis_pour(&e.type_name) else {
        return Ok(());
    };
    sqlx::query("SELECT set_config('app.etablissement_id', $1, true)")
        .bind(etab.to_string())
        .execute(&mut *tx)
        .await
        .map_err(|x| x.to_string())?;
    let roles: Vec<String> = crate::voyages_reactions::roles_avec_action(Action::InternatGerer)
        .into_iter()
        .map(String::from)
        .collect();
    let destinataires: Vec<Uuid> = sqlx::query_scalar(
        "SELECT DISTINCT m.user_id FROM ent_membership m
          WHERE m.etablissement_id = $1
            AND (m.role = ANY($2)
                 OR EXISTS (SELECT 1 FROM ent_role_attribution a
                              JOIN ent_role_droit d ON d.role_id = a.role_id AND d.etablissement_id = a.etablissement_id
                             WHERE a.etablissement_id = m.etablissement_id AND a.user_id = m.user_id
                               AND a.clos_le IS NULL AND d.action = $3 AND d.portee = 'etablissement'
                               AND a.debut <= ent_aujourdhui_etab($1) AND (a.fin IS NULL OR a.fin >= ent_aujourdhui_etab($1))))
          ORDER BY m.user_id",
    )
    .bind(etab)
    .bind(&roles)
    .bind(Action::InternatGerer.code())
    .fetch_all(&mut *tx)
    .await
    .map_err(|x| x.to_string())?;
    for user_id in destinataires {
        notifier_personne(
            &mut *tx,
            EnvoiAPlanifier {
                etablissement_id: etab,
                user_id,
                kind: nature,
                titre,
                corps,
                eleve_id: None,
                notification_id: None,
                source_type: "evenement",
                source_id: e.id,
                cle_dedup: format!("evt:{}:{}", e.id, nature),
                droit_requis: None,
                regroupable: false,
                canaux_voulus: &["email", "push"],
            },
            e.id,
            Utc::now(),
        )
        .await
        .map_err(|x| x.to_string())?;
    }
    Ok(())
}

impl Consommateur for AlertesInternat {
    fn nom(&self) -> &'static str {
        CONSOMMATEUR_ALERTES_INTERNAT
    }

    fn types(&self) -> &'static [&'static str] {
        TYPES_ALERTES
    }

    fn traiter<'a>(
        &'a self,
        tx: &'a mut PgConnection,
        evt: &'a EvenementDistribue,
    ) -> FutureTraitement<'a> {
        Box::pin(traiter_alertes(tx, evt))
    }
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
    // Deux émetteurs, deux formes de charge utile : la scolarité datée du socle 2
    // (`nouvelle_valeur`) et la facturation (`nouveau`).
    let (Some(eleve), Some(nouveau), Some(effet)) = (
        e.eleve_id,
        texte(evt, "nouvelle_valeur").or_else(|| texte(evt, "nouveau")),
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
