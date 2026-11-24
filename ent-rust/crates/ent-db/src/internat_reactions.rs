//! Réactions de l'internat aux événements du cœur (socle 41, lots I0 et I1).
//!
//! | Déclencheur | Effet |
//! |---|---|
//! | `eleve.regime_changed` | séjour d'hébergement resynchronisé sur le régime daté (R-IT1) ; si l'élève n'est plus interne, son affectation ouverte prend fin la veille de la date d'effet ; `internat.regime_effet` publié (avis au responsable de l'internat) |
//!
//! Consommateur `i0.internat`, **rejouable** : la synchronisation des séjours est idempotente et
//! l'affectation n'est close qu'une fois (RI-05, INT-112).

use chrono::{DateTime, Duration, NaiveDate, Utc};
use ent_auth::Action;
use ent_events::{types, AggregateKind};
use serde_json::json;
use sqlx::PgConnection;
use std::sync::Arc;
use uuid::Uuid;

use crate::finances::publier;
use crate::internat_hebergement::synchroniser_sejours_tx;
use crate::internat_repo::{get_affectation_ouverte_tx, update_affectation_fin_tx};
use crate::notifications::{notifier_personne_selon, EnvoiAPlanifier};
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
    types::INTERNAT_RETOUR_DEMANDE,
];

/// Un événement plus ancien que cette limite n'est plus signalé : au premier passage du consommateur, l'historique encore
/// en attente dans l'outbox ne déclenche pas une rafale d'avis périmés.
const AGE_MAX_AVIS_HEURES: i64 = 24;

/// Identifiant déterministe d'une clé de regroupement (empreinte SHA-256 tronquée à 128 bits).
fn uuid_de_cle(cle: &str) -> Uuid {
    use sha2::{Digest, Sha256};
    let h = Sha256::digest(cle.as_bytes());
    let mut o = [0u8; 16];
    o.copy_from_slice(&h[..16]);
    Uuid::from_bytes(o)
}

/// Vrai si l'événement est trop ancien pour justifier un avis (pur : l'instant est fourni).
fn trop_ancien(cree_le: DateTime<Utc>, maintenant: DateTime<Utc>) -> bool {
    cree_le < maintenant - Duration::hours(AGE_MAX_AVIS_HEURES)
}

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
        t if t == types::INTERNAT_RETOUR_DEMANDE => Some((
            "internat_retour_demande",
            "Retour demandé : refus d'un parent",
            "Un parent a refusé une sortie de groupe après le départ de l'élève : le retour est demandé, ouvrez la sortie.",
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
    let maintenant = Utc::now();
    // Borne du rattrapage : un avis trop ancien n'est plus envoyé, sauf l'alerte critique (malaise…), toujours relayée ;
    // la perte est journalisée, jamais silencieuse.
    if e.type_name != types::INTERNAT_ALERTE && trop_ancien(e.created_at, maintenant) {
        tracing::warn!(evenement = %e.id, genre = %e.type_name, "avis d'internat trop ancien : non envoyé");
        return Ok(());
    }
    sqlx::query("SELECT set_config('app.etablissement_id', $1, true)")
        .bind(etab.to_string())
        .execute(&mut *tx)
        .await
        .map_err(|x| x.to_string())?;
    let roles: Vec<String> = crate::voyages_reactions::roles_avec_action(Action::InternatGerer)
        .into_iter()
        .map(String::from)
        .collect();
    let mut destinataires: Vec<Uuid> = sqlx::query_scalar(
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
    let mut corps_base = corps.to_string();
    if e.type_name == types::INTERNAT_RETOUR_DEMANDE {
        // La demande de retour désigne la SORTIE (titre, heure de fin dans le fuseau de l'établissement), jamais l'élève ;
        // elle part aussi aux accompagnateurs de la sortie, qui sont ceux qui peuvent ramener l'élève.
        let sortie: Option<(String, String)> = sqlx::query_as(
            "SELECT g.titre, to_char(g.fin AT TIME ZONE COALESCE(
                        (SELECT p.fuseau FROM ent_etablissement_parametre p WHERE p.etablissement_id = g.etablissement_id),
                        'Europe/Paris'), 'HH24\"h\"MI')
               FROM ent_internat_sortie_groupe g WHERE g.etablissement_id = $1 AND g.id = $2",
        )
        .bind(etab)
        .bind(e.aggregate_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|x| x.to_string())?;
        if let Some((titre_sortie, fin_locale)) = sortie {
            corps_base = format!(
                "Un parent a refusé la sortie « {titre_sortie} » (fin prévue à {fin_locale}) après le départ d'un élève : le retour est demandé, ouvrez la sortie."
            );
            let accompagnateurs: Vec<Uuid> = sqlx::query_scalar(
                "SELECT e.user_id FROM ent_internat_sortie_groupe_encadrant e
                   JOIN ent_membership m ON m.etablissement_id = e.etablissement_id AND m.user_id = e.user_id
                  WHERE e.etablissement_id = $1 AND e.sortie_groupe_id = $2",
            )
            .bind(etab)
            .bind(e.aggregate_id)
            .fetch_all(&mut *tx)
            .await
            .map_err(|x| x.to_string())?;
            for u in accompagnateurs {
                if !destinataires.contains(&u) {
                    destinataires.push(u);
                }
            }
        }
    }
    // Avis regroupés : un seul avis par établissement, par nature et par minute (une rafale de cas ou de signaux ne sature
    // pas les responsables), avec le nombre d'élèves concernés et jamais un nom. L'alerte critique et la demande de retour,
    // elles, ne sont jamais regroupées : un avis par événement.
    // Une demande de retour (refus d'un parent après le départ) n'est jamais regroupée non plus : chacune est un mineur
    // dehors contre l'avis d'un parent.
    let regroupe = e.type_name != types::INTERNAT_ALERTE && e.type_name != types::INTERNAT_RETOUR_DEMANDE;
    let minute = e.created_at.timestamp().div_euclid(60);
    let debut_minute = DateTime::<Utc>::from_timestamp(minute * 60, 0).unwrap_or(e.created_at);
    let mut corps_avis = corps_base.clone();
    if regroupe {
        let n: i64 = sqlx::query_scalar(
            "SELECT count(DISTINCT COALESCE(eleve_id, id)) FROM ent_metier_event \
              WHERE etablissement_id = $1 AND type = $2 AND created_at >= $3 AND created_at < $4",
        )
        .bind(etab)
        .bind(&e.type_name)
        .bind(debut_minute)
        .bind(debut_minute + Duration::minutes(1))
        .fetch_one(&mut *tx)
        .await
        .map_err(|x| x.to_string())?;
        if n > 1 {
            corps_avis = format!("{corps_base} ({n} élèves concernés cette minute.)");
        }
    }
    // Clé de regroupement : l'avis dans l'ENT est idempotent sur (événement source, destinataire, nature) ; l'événement
    // source d'un avis regroupé est donc un identifiant dérivé de (établissement, nature, minute), le même pour toute la
    // rafale.
    let (cle_dedup, source_avis) = if regroupe {
        let cle = format!("i4:{etab}:{nature}:{minute}");
        (cle.clone(), uuid_de_cle(&cle))
    } else {
        (format!("evt:{}:{}", e.id, nature), e.id)
    };
    for user_id in destinataires {
        // Avis regroupé : un second événement de la même minute met à jour le nombre de l'avis encore non lu.
        notifier_personne_selon(
            &mut *tx,
            EnvoiAPlanifier {
                etablissement_id: etab,
                user_id,
                kind: nature,
                titre,
                corps: &corps_avis,
                eleve_id: None,
                notification_id: None,
                source_type: "evenement",
                source_id: e.id,
                cle_dedup: cle_dedup.clone(),
                droit_requis: None,
                regroupable: false,
                canaux_voulus: &["email", "push"],
            },
            source_avis,
            maintenant,
            regroupe,
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

#[cfg(test)]
mod tests_unitaires {
    use super::*;

    #[test]
    fn un_evenement_de_plus_de_vingt_quatre_heures_ne_declenche_pas_d_avis() {
        let t = DateTime::parse_from_rfc3339("2026-04-10T10:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        assert!(!trop_ancien(t - Duration::hours(23), t));
        assert!(trop_ancien(t - Duration::hours(25), t));
        assert!(!trop_ancien(t, t));
    }

    #[test]
    fn la_cle_de_regroupement_est_stable_et_distincte_d_une_minute_a_l_autre() {
        assert_eq!(uuid_de_cle("i4:x:n:1"), uuid_de_cle("i4:x:n:1"));
        assert_ne!(uuid_de_cle("i4:x:n:1"), uuid_de_cle("i4:x:n:2"));
    }
}
