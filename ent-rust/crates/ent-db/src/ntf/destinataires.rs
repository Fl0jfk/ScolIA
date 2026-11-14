//! Socle 8, lot N1 — destinataires d'un avis : adaptateur provisoire sur les liens actifs.
//!
//! Le port `destinataires` du socle 4 (A11) n'existe pas encore : cet adaptateur reprend la règle de l'ancien
//! chemin (lien actif et sollicitable, compte « parent » membre de l'établissement, droit du lien exigé par la
//! nature). Le lot N2 le remplace par les **personnes** (sans compte, organisations, exclusions et raisons).

use super::contrat::DroitRequis;
use sqlx::PgConnection;
use uuid::Uuid;

/// Un destinataire candidat (compte de responsable).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Candidat {
    pub compte_id: Uuid,
    pub raison_code: &'static str,
}

/// Responsables à prévenir pour un élève, avec le droit du lien exigé par la nature.
pub async fn responsables_de_l_eleve(
    conn: &mut PgConnection,
    etablissement_id: Uuid,
    eleve_id: Uuid,
    droit: DroitRequis,
) -> Result<Vec<Candidat>, sqlx::Error> {
    // Colonne choisie dans une liste fermée (jamais d'entrée libre dans le SQL).
    let clause = match droit.colonne() {
        Some("peut_voir_vs") => "AND l.peut_voir_vs",
        Some("peut_voir_notes") => "AND l.peut_voir_notes",
        _ => "",
    };
    let lignes: Vec<(Uuid,)> = sqlx::query_as(&format!(
        "SELECT DISTINCT l.user_id
           FROM ent_eleve_lien_sollicitable l
           JOIN ent_membership m ON m.user_id = l.user_id AND m.etablissement_id = l.etablissement_id
          WHERE l.etablissement_id = $1 AND l.eleve_id = $2 AND m.role = 'parent' {clause}
          ORDER BY l.user_id"
    ))
    .bind(etablissement_id)
    .bind(eleve_id)
    .fetch_all(&mut *conn)
    .await?;
    Ok(lignes
        .into_iter()
        .map(|(compte_id,)| Candidat { compte_id, raison_code: "responsable_lien_actif" })
        .collect())
}

/// Le lien est-il toujours valable pour cette nature ? (relu à la remise, R-N1.)
pub async fn lien_toujours_valable(
    conn: &mut PgConnection,
    etablissement_id: Uuid,
    eleve_id: Uuid,
    compte_id: Uuid,
    droit: DroitRequis,
) -> Result<bool, sqlx::Error> {
    let clause = match droit.colonne() {
        Some("peut_voir_vs") => "AND l.peut_voir_vs",
        Some("peut_voir_notes") => "AND l.peut_voir_notes",
        _ => "",
    };
    let ok: Option<(i32,)> = sqlx::query_as(&format!(
        "SELECT 1 FROM ent_eleve_lien_sollicitable l
           JOIN ent_membership m ON m.user_id = l.user_id AND m.etablissement_id = l.etablissement_id
          WHERE l.etablissement_id = $1 AND l.eleve_id = $2 AND l.user_id = $3 AND m.role = 'parent' {clause}
          LIMIT 1"
    ))
    .bind(etablissement_id)
    .bind(eleve_id)
    .bind(compte_id)
    .fetch_optional(&mut *conn)
    .await?;
    Ok(ok.is_some())
}
