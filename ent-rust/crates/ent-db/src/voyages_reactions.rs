//! Socle 42, lot V0b : avis du socle 8 sur la vie d'une sortie (NX-03, NX-16, NX-64 ; §23).
//!
//! Consommateur `d42.voyages_notifications` : transforme des faits `voyages.*` en notifications
//! (dans l'ENT toujours, e-mail et push selon les préférences de la personne, via la chaîne du
//! socle 8). Destinataires **calculés par le serveur** et relus au traitement (R-SV12) ; texte fixe
//! par nature, **aucun texte libre recopié** (ni motif, ni note de décision) ; jamais de SMS ;
//! aucune identité d'élève. Rejouable : la clé unique (événement, destinataire, nature) évite tout
//! doublon.
//!
//! | Événement | Nature | Destinataires |
//! |---|---|---|
//! | `voyages.soumise`, `voyages.resoumise` (vers `soumise`) | `voyages_nouvelle_demande` | direction |
//! | `voyages.passee_aux_finances` (vers `budget_a_valider`) | `voyages_budget_a_valider` | comptabilité |
//! | `voyages.a_completer` | `voyages_a_completer` | porteur |
//! | `voyages.refusee` | `voyages_refusee` | porteur |
//! | `voyages.autorisee` | `voyages_validee` | porteur |
//! | `voyages.rappel_organisateur` | `voyages_rappel_organisateur` | porteur |

use chrono::Utc;
use ent_auth::{modele_par_defaut, Action, Portee, Role};
use sqlx::PgConnection;
use uuid::Uuid;

use crate::notifications::{notifier_personne, EnvoiAPlanifier};
use crate::outbox::{Consommateur, EvenementDistribue, FutureTraitement};
use crate::voyages_budget::message_rappel;

pub const CONSOMMATEUR_VOYAGES: &str = "d42.voyages_notifications";

pub struct ConsommateurVoyages;

const TYPES_VOYAGES: &[&str] = &[
    "voyages.soumise",
    "voyages.resoumise",
    "voyages.passee_aux_finances",
    "voyages.a_completer",
    "voyages.refusee",
    "voyages.autorisee",
    "voyages.rappel_organisateur",
];

enum Cible {
    /// Les profils qui détiennent l'action par défaut (direction, comptabilité…).
    Action(Action),
    Porteur,
}

struct Message {
    nature: &'static str,
    cible: Cible,
    titre: String,
    corps: String,
}

/// Profils de base dont le modèle par défaut contient l'action à l'échelle de l'établissement.
pub(crate) fn roles_avec_action(action: Action) -> Vec<&'static str> {
    [
        Role::Admin,
        Role::Direction,
        Role::Administratif,
        Role::Cpe,
        Role::Compta,
        Role::Prof,
        Role::Infirmerie,
        Role::Psy,
        Role::Accueil,
        Role::Personnel,
    ]
    .into_iter()
    .filter(|r| {
        modele_par_defaut(*r)
            .iter()
            .any(|(a, p)| *a == action && *p == Portee::Etablissement)
    })
    .map(|r| r.as_str())
    .collect()
}

/// Message d'un événement, ou `None` s'il n'appelle aucun avis. `titre_sortie` vient de la base.
fn message_pour(type_name: &str, payload: &serde_json::Value, titre_sortie: &str) -> Option<Message> {
    let apres = payload.get("apres").and_then(|v| v.as_str());
    Some(match type_name {
        "voyages.soumise" => Message {
            nature: "voyages_nouvelle_demande",
            cible: Cible::Action(Action::VoyagesValider),
            titre: "Nouvelle demande de sortie".into(),
            corps: format!("« {titre_sortie} » attend votre validation pédagogique."),
        },
        "voyages.resoumise" if apres == Some("soumise") => Message {
            nature: "voyages_nouvelle_demande",
            cible: Cible::Action(Action::VoyagesValider),
            titre: "Demande de sortie renvoyée".into(),
            corps: format!("« {titre_sortie} » a été renvoyée et attend votre validation pédagogique."),
        },
        "voyages.passee_aux_finances" if apres == Some("budget_a_valider") => Message {
            nature: "voyages_budget_a_valider",
            cible: Cible::Action(Action::VoyagesBudgetValider),
            titre: "Budget de sortie à valider".into(),
            corps: format!("Le budget de « {titre_sortie} » attend votre validation."),
        },
        "voyages.a_completer" => Message {
            nature: "voyages_a_completer",
            cible: Cible::Porteur,
            titre: "Modifications demandées".into(),
            corps: format!("Des modifications sont demandées sur « {titre_sortie} » : ouvrez le dossier pour lire la note."),
        },
        "voyages.refusee" => Message {
            nature: "voyages_refusee",
            cible: Cible::Porteur,
            titre: "Sortie refusée".into(),
            corps: format!("« {titre_sortie} » n'a pas été retenue : ouvrez le dossier pour en savoir plus."),
        },
        "voyages.autorisee" => Message {
            nature: "voyages_validee",
            cible: Cible::Porteur,
            titre: "Sortie validée".into(),
            corps: format!("« {titre_sortie} » est autorisée."),
        },
        "voyages.rappel_organisateur" => {
            let code = payload.get("code").and_then(|v| v.as_str()).unwrap_or("");
            Message {
                nature: "voyages_rappel_organisateur",
                cible: Cible::Porteur,
                titre: format!("Rappel : {titre_sortie}"),
                corps: message_rappel(code).to_string(),
            }
        }
        _ => return None,
    })
}

async fn traiter(tx: &mut PgConnection, evt: &EvenementDistribue) -> Result<(), String> {
    let etab = evt.etablissement_id();
    let e = evt.event();
    // Relu dans l'établissement de l'événement : titre et porteur de CETTE sortie.
    let sortie: Option<(String, Option<Uuid>)> = sqlx::query_as(
        "SELECT s.titre, p.user_id
           FROM ent_sortie s
           LEFT JOIN ent_personne p
             ON p.etablissement_id = s.etablissement_id AND p.id = s.porteur_personne_id
          WHERE s.etablissement_id = $1 AND s.id = $2",
    )
    .bind(etab)
    .bind(e.aggregate_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|x| x.to_string())?;
    let Some((titre_sortie, porteur_user)) = sortie else {
        return Ok(());
    };
    let Some(m) = message_pour(&e.type_name, &e.payload, &titre_sortie) else {
        return Ok(());
    };
    let destinataires: Vec<Uuid> = match m.cible {
        Cible::Porteur => porteur_user.into_iter().collect(),
        Cible::Action(action) => {
            let roles: Vec<String> = roles_avec_action(action).into_iter().map(String::from).collect();
            sqlx::query_scalar(
                "SELECT user_id FROM ent_membership
                  WHERE etablissement_id = $1 AND role = ANY($2) ORDER BY user_id",
            )
            .bind(etab)
            .bind(&roles)
            .fetch_all(&mut *tx)
            .await
            .map_err(|x| x.to_string())?
        }
    };
    for user_id in destinataires {
        // Le porteur doit toujours être membre de l'établissement : un compte parti ne reçoit rien.
        if matches!(m.cible, Cible::Porteur) {
            let membre: bool = sqlx::query_scalar(
                "SELECT EXISTS (SELECT 1 FROM ent_membership WHERE etablissement_id = $1 AND user_id = $2)",
            )
            .bind(etab)
            .bind(user_id)
            .fetch_one(&mut *tx)
            .await
            .map_err(|x| x.to_string())?;
            if !membre {
                continue;
            }
        }
        notifier_personne(
            &mut *tx,
            EnvoiAPlanifier {
                etablissement_id: etab,
                user_id,
                kind: m.nature,
                titre: &m.titre,
                corps: &m.corps,
                eleve_id: None,
                notification_id: None,
                source_type: "evenement",
                source_id: e.id,
                cle_dedup: format!("evt:{}:{}", e.id, m.nature),
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

impl Consommateur for ConsommateurVoyages {
    fn nom(&self) -> &'static str {
        CONSOMMATEUR_VOYAGES
    }

    fn types(&self) -> &'static [&'static str] {
        TYPES_VOYAGES
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
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn natures_destinataires_et_textes_fixes() {
        let m = message_pour("voyages.soumise", &json!({}), "Rome").unwrap();
        assert_eq!(m.nature, "voyages_nouvelle_demande");
        assert!(matches!(m.cible, Cible::Action(Action::VoyagesValider)));
        // Le motif d'une décision n'est jamais recopié dans l'avis.
        let m = message_pour("voyages.a_completer", &json!({ "motif": "SECRET" }), "Rome").unwrap();
        assert!(!m.corps.contains("SECRET") && matches!(m.cible, Cible::Porteur));
        // Passage aux finances : seulement vers l'étape du budget.
        assert!(message_pour("voyages.passee_aux_finances", &json!({ "apres": "a_autoriser" }), "x").is_none());
        let m = message_pour("voyages.passee_aux_finances", &json!({ "apres": "budget_a_valider" }), "x").unwrap();
        assert!(matches!(m.cible, Cible::Action(Action::VoyagesBudgetValider)));
        // Renvoi : avis à la direction seulement s'il revient à la validation pédagogique.
        assert!(message_pour("voyages.resoumise", &json!({ "apres": "a_signer" }), "x").is_none());
        assert!(message_pour("voyages.resoumise", &json!({ "apres": "soumise" }), "x").is_some());
        let m = message_pour("voyages.rappel_organisateur", &json!({ "code": "effectif" }), "Rome").unwrap();
        assert_eq!(m.corps, message_rappel("effectif"));
        assert!(message_pour("voyages.inconnue", &json!({}), "x").is_none());
    }

    #[test]
    fn profils_destinataires_calcules_depuis_le_modele_par_defaut() {
        let valider = roles_avec_action(Action::VoyagesValider);
        assert!(valider.contains(&"direction") && valider.contains(&"admin"));
        assert!(!valider.contains(&"prof") && !valider.contains(&"compta"));
        let budget = roles_avec_action(Action::VoyagesBudgetValider);
        assert!(budget.contains(&"compta") && !budget.contains(&"direction"));
    }
}
