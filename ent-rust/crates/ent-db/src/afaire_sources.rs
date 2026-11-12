//! Socle 29, lot P2 : ce que chaque brique propriétaire sait dire de ses tâches (`taches_ouvertes`,
//! `docs/socles/29_PORTAILS.md` §6.3, §8.1).
//!
//! Chaque fonction **relit la source de vérité** (absence, carnet, document, protection) et résout
//! les destinataires **au moment de l'appel** : c'est ce qui sert au rapprochement (§8.10), à la
//! réaction aux événements et à la revérification à l'affichage (R-PO3). Le 29 ne copie rien : le
//! titre est composé depuis un gabarit déclaré.
//!
//! Destinataires (R-PO8, §8.8) :
//! * élève **majeur** : ses tâches de décision sont les siennes (son compte), plus celles des parents ;
//! * responsable : lien **actif** (non suspendu, dates valides), titulaire de l'**autorité parentale**
//!   et porteur du droit concerné ; un responsable sans exercice n'a **aucune** tâche de décision ;
//! * aucun destinataire actif : la file de repli de la nature (R-PO9).

use crate::afaire::{
    fin_de_journee, nature, ActionDeclaree, Destinataire, Var, FILE_JUSTIFICATIFS, FILE_PROTECTIONS,
};
use crate::service::EntDbError;
use chrono::{DateTime, NaiveDate, Utc};
use sqlx::{PgConnection, Row};
use std::collections::HashMap;
use uuid::Uuid;

fn db(e: sqlx::Error) -> EntDbError {
    EntDbError::Sqlx(e)
}

/// Une tâche telle que le propriétaire la dit **à cet instant**.
#[derive(Clone, Debug)]
pub struct TacheSource {
    pub objet_id: Uuid,
    pub eleve_id: Option<Uuid>,
    pub valeurs: Vec<(Var, String)>,
    /// Vide : file de repli de la nature.
    pub destinataires: Vec<Destinataire>,
    pub echeance_le: Option<DateTime<Utc>>,
}

/// Droit du lien exigé pour recevoir une tâche de décision.
#[derive(Clone, Copy, Debug)]
pub(crate) enum ExigenceLien {
    /// Autorité parentale + droit de justifier une absence.
    Justifier,
    /// Autorité parentale + droit de voir la vie scolaire et de justifier (signature du carnet, M0a).
    SignerCarnet,
    /// Autorité parentale seule (documents à signer).
    Autorite,
}

fn jour_fr(d: NaiveDate) -> String {
    d.format("%d/%m").to_string()
}

/// Destinataires de décision de chaque élève : élève majeur (son compte) ou responsables habilités.
pub(crate) async fn destinataires_decision(
    conn: &mut PgConnection,
    etab: Uuid,
    eleves: &[Uuid],
    exigence: ExigenceLien,
) -> Result<HashMap<Uuid, Vec<Destinataire>>, EntDbError> {
    let mut out: HashMap<Uuid, Vec<Destinataire>> = HashMap::new();
    if eleves.is_empty() {
        return Ok(out);
    }
    let filtre = match exigence {
        ExigenceLien::Justifier => "AND l.autorite_parentale AND l.peut_justifier_absence",
        ExigenceLien::SignerCarnet => {
            "AND l.autorite_parentale AND l.peut_voir_vs AND l.peut_justifier_absence"
        }
        ExigenceLien::Autorite => "AND l.autorite_parentale",
    };
    // Élèves majeurs : la décision bascule vers l'élève (§8.8).
    let majeurs = sqlx::query(
        "SELECT e.id, c.user_id
           FROM ent_eleve e
           LEFT JOIN ent_eleve_compte c ON c.etablissement_id = e.etablissement_id AND c.eleve_id = e.id
          WHERE e.etablissement_id = $1 AND e.id = ANY($2)
            AND e.date_naissance IS NOT NULL
            AND e.date_naissance + INTERVAL '18 years' <= ent_aujourdhui_etab($1)",
    )
    .bind(etab)
    .bind(eleves)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    let mut est_majeur: HashMap<Uuid, Option<Uuid>> = HashMap::new();
    for r in majeurs {
        est_majeur.insert(r.get(0), r.get(1));
    }
    let sql = format!(
        "SELECT l.eleve_id, l.personne_id
           FROM ent_eleve_lien_communicant l
          WHERE l.etablissement_id = $1 AND l.eleve_id = ANY($2) AND l.personne_id IS NOT NULL {filtre}
          ORDER BY l.eleve_id, l.personne_id"
    );
    let liens = sqlx::query(&sql)
        .bind(etab)
        .bind(eleves)
        .fetch_all(&mut *conn)
        .await
        .map_err(db)?;
    let mut resp: HashMap<Uuid, Vec<Uuid>> = HashMap::new();
    for r in liens {
        resp.entry(r.get(0)).or_default().push(r.get(1));
    }
    for e in eleves {
        let dests = match est_majeur.get(e) {
            Some(Some(user)) => vec![Destinataire::Compte(*user)],
            // Majeur sans compte : personne à solliciter, la file de repli s'en charge (R-PO9).
            Some(None) => vec![],
            None => resp
                .get(e)
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .map(Destinataire::Personne)
                .collect(),
        };
        out.insert(*e, dests);
    }
    Ok(out)
}

/// Absences d'élèves encore sans motif (à justifier) ou avec un justificatif soumis (à trancher).
async fn absences(
    conn: &mut PgConnection,
    etab: Uuid,
    soumis: bool,
    objets: Option<&[Uuid]>,
) -> Result<Vec<(Uuid, Uuid, String, NaiveDate, NaiveDate)>, EntDbError> {
    let motif = if soumis {
        "a.motif_justif IS NOT NULL"
    } else {
        "a.motif_justif IS NULL"
    };
    let sql = format!(
        "SELECT a.id, a.eleve_id, e.prenom, a.debut, a.fin
           FROM ent_absence a
           JOIN ent_eleve e ON e.etablissement_id = a.etablissement_id AND e.id = a.eleve_id
          WHERE a.etablissement_id = $1 AND a.statut = 'a_traiter' AND {motif}
            AND ($2::uuid[] IS NULL OR a.id = ANY($2))
          ORDER BY a.debut, a.id"
    );
    let rows = sqlx::query(&sql)
        .bind(etab)
        .bind(objets)
        .fetch_all(&mut *conn)
        .await
        .map_err(db)?;
    Ok(rows
        .into_iter()
        .map(|r| (r.get(0), r.get(1), r.get(2), r.get(3), r.get(4)))
        .collect())
}

/// Tâches ouvertes d'une nature **dérivée d'une lecture SQL** (toutes sauf l'appel, qui dépend du
/// service de vie scolaire). `objets` restreint la lecture (revérification, réaction à un événement).
pub async fn taches_ouvertes_sql(
    conn: &mut PgConnection,
    etab: Uuid,
    nature_code: &str,
    objets: Option<&[Uuid]>,
) -> Result<Vec<TacheSource>, EntDbError> {
    match nature_code {
        "vs.justifier_absence" => {
            let abs = absences(conn, etab, false, objets).await?;
            let eleves: Vec<Uuid> = abs.iter().map(|a| a.1).collect();
            let dest = destinataires_decision(conn, etab, &eleves, ExigenceLien::Justifier).await?;
            let mut out = Vec::new();
            for (id, eleve, prenom, debut, fin) in abs {
                let jours = if fin > debut {
                    format!("{} au {}", jour_fr(debut), jour_fr(fin))
                } else {
                    jour_fr(debut)
                };
                out.push(TacheSource {
                    objet_id: id,
                    eleve_id: Some(eleve),
                    valeurs: vec![(Var::Prenom, prenom), (Var::Date, jours)],
                    destinataires: dest.get(&eleve).cloned().unwrap_or_default(),
                    echeance_le: Some(
                        fin_de_journee(conn, etab, fin + chrono::Duration::days(3)).await?,
                    ),
                });
            }
            Ok(out)
        }
        "vs.trancher_justificatif" => {
            let abs = absences(conn, etab, true, objets).await?;
            Ok(abs
                .into_iter()
                .map(|(id, eleve, prenom, debut, _)| TacheSource {
                    objet_id: id,
                    eleve_id: Some(eleve),
                    valeurs: vec![(Var::Prenom, prenom), (Var::Date, jour_fr(debut))],
                    destinataires: vec![Destinataire::File(FILE_JUSTIFICATIFS.into())],
                    echeance_le: None,
                })
                .collect())
        }
        "vs.signer_carnet" => {
            let rows = sqlx::query(
                "SELECT c.id, c.eleve_id, e.prenom, c.date_entree
                   FROM ent_carnet_entree c
                   JOIN ent_eleve e ON e.etablissement_id = c.etablissement_id AND e.id = c.eleve_id
                  WHERE c.etablissement_id = $1 AND c.visible_famille AND c.signe_at IS NULL
                    AND ($2::uuid[] IS NULL OR c.id = ANY($2))
                  ORDER BY c.date_entree, c.id",
            )
            .bind(etab)
            .bind(objets)
            .fetch_all(&mut *conn)
            .await
            .map_err(db)?;
            let lignes: Vec<(Uuid, Uuid, String, NaiveDate)> = rows
                .into_iter()
                .map(|r| (r.get(0), r.get(1), r.get(2), r.get(3)))
                .collect();
            let eleves: Vec<Uuid> = lignes.iter().map(|l| l.1).collect();
            let dest =
                destinataires_decision(conn, etab, &eleves, ExigenceLien::SignerCarnet).await?;
            let mut out = Vec::new();
            for (id, eleve, prenom, date) in lignes {
                out.push(TacheSource {
                    objet_id: id,
                    eleve_id: Some(eleve),
                    valeurs: vec![(Var::Prenom, prenom), (Var::Date, jour_fr(date))],
                    destinataires: dest.get(&eleve).cloned().unwrap_or_default(),
                    echeance_le: Some(
                        fin_de_journee(conn, etab, date + chrono::Duration::days(7)).await?,
                    ),
                });
            }
            Ok(out)
        }
        "doc.signer" => {
            let rows = sqlx::query(
                "SELECT d.id, d.eleve_id, e.prenom, d.echeance
                   FROM ent_doc_a_signer d
                   JOIN ent_eleve e ON e.etablissement_id = d.etablissement_id AND e.id = d.eleve_id
                  WHERE d.etablissement_id = $1 AND d.statut = 'a_signer'
                    AND ($2::uuid[] IS NULL OR d.id = ANY($2))
                  ORDER BY d.publie_at, d.id",
            )
            .bind(etab)
            .bind(objets)
            .fetch_all(&mut *conn)
            .await
            .map_err(db)?;
            let lignes: Vec<(Uuid, Uuid, String, Option<NaiveDate>)> = rows
                .into_iter()
                .map(|r| (r.get(0), r.get(1), r.get(2), r.get(3)))
                .collect();
            let eleves: Vec<Uuid> = lignes.iter().map(|l| l.1).collect();
            let dest = destinataires_decision(conn, etab, &eleves, ExigenceLien::Autorite).await?;
            let mut out = Vec::new();
            for (id, eleve, prenom, echeance) in lignes {
                let echeance_le = match echeance {
                    Some(j) => Some(fin_de_journee(conn, etab, j).await?),
                    None => None,
                };
                out.push(TacheSource {
                    objet_id: id,
                    eleve_id: Some(eleve),
                    valeurs: vec![(Var::Prenom, prenom)],
                    destinataires: dest.get(&eleve).cloned().unwrap_or_default(),
                    echeance_le,
                });
            }
            Ok(out)
        }
        "dos.protection_a_confirmer" => {
            // Aucune identité dans l'index : seul l'identifiant de l'objet suit (R-PO5).
            let mut out = Vec::new();
            let adresses = sqlx::query(
                "SELECT a.id FROM ent_adresse_protegee a
                  WHERE a.etablissement_id = $1 AND a.statut = 'a_traiter'
                    AND ($2::uuid[] IS NULL OR a.id = ANY($2))
                  ORDER BY a.created_at, a.id",
            )
            .bind(etab)
            .bind(objets)
            .fetch_all(&mut *conn)
            .await
            .map_err(db)?;
            let nps = sqlx::query(
                "SELECT r.personne_id FROM ent_personne_reserve r
                  WHERE r.etablissement_id = $1 AND r.ne_pas_solliciter
                    AND r.ne_pas_solliciter_confirme_le IS NULL
                    AND ($2::uuid[] IS NULL OR r.personne_id = ANY($2))
                  ORDER BY r.ne_pas_solliciter_le, r.personne_id",
            )
            .bind(etab)
            .bind(objets)
            .fetch_all(&mut *conn)
            .await
            .map_err(db)?;
            for r in adresses.into_iter().chain(nps) {
                out.push(TacheSource {
                    objet_id: r.get(0),
                    eleve_id: None,
                    valeurs: vec![],
                    destinataires: vec![Destinataire::File(FILE_PROTECTIONS.into())],
                    echeance_le: None,
                });
            }
            Ok(out)
        }
        _ => Err(EntDbError::SaisieInvalide("nature sans lecture SQL")),
    }
}

/// Texte d'aperçu et champs d'une carte d'action (§8.7), rendus **par le propriétaire** au moment
/// de l'ouverture (le détail n'est jamais dans l'index).
pub(crate) async fn apercu_carte(
    conn: &mut PgConnection,
    etab: Uuid,
    nature_code: &str,
    objet_id: Uuid,
) -> Result<Option<(String, Vec<ent_types::afaire::ChampAction>)>, EntDbError> {
    use ent_types::afaire::ChampAction;
    match nature_code {
        "vs.justifier_absence" => {
            let Some((_, _, prenom, debut, fin)) = absences(conn, etab, false, Some(&[objet_id]))
                .await?
                .into_iter()
                .next()
            else {
                return Ok(None);
            };
            let periode = if fin > debut {
                format!("du {} au {}", jour_fr(debut), jour_fr(fin))
            } else {
                format!("du {}", jour_fr(debut))
            };
            Ok(Some((
                format!(
                    "Vous justifiez l'absence de {prenom} {periode}. Indiquez le motif ; la vie scolaire décidera."
                ),
                vec![ChampAction {
                    code: "motif".into(),
                    libelle: "Motif de l'absence".into(),
                    obligatoire: true,
                    choix: vec![],
                }],
            )))
        }
        "vs.signer_carnet" => {
            let Some(l) = taches_ouvertes_sql(conn, etab, nature_code, Some(&[objet_id]))
                .await?
                .into_iter()
                .next()
            else {
                return Ok(None);
            };
            let prenom = l
                .valeurs
                .iter()
                .find(|(k, _)| *k == Var::Prenom)
                .map(|v| v.1.clone());
            Ok(Some((
                format!(
                    "Vous signez le mot du carnet de {} : cela vaut prise de connaissance.",
                    prenom.unwrap_or_default()
                ),
                vec![],
            )))
        }
        "doc.signer" => {
            let r = sqlx::query(
                "SELECT d.titre, e.prenom FROM ent_doc_a_signer d
                   JOIN ent_eleve e ON e.etablissement_id = d.etablissement_id AND e.id = d.eleve_id
                  WHERE d.etablissement_id = $1 AND d.id = $2 AND d.statut = 'a_signer'",
            )
            .bind(etab)
            .bind(objet_id)
            .fetch_optional(&mut *conn)
            .await
            .map_err(db)?;
            Ok(r.map(|r| {
                let (titre, prenom): (String, String) = (r.get(0), r.get(1));
                (
                    format!("Vous signez « {titre} » pour {prenom}. Cette signature vous engage."),
                    vec![],
                )
            }))
        }
        _ => Ok(None),
    }
}

/// La nature propose-t-elle une carte d'action (et non un simple lien) ?
pub fn a_carte_d_action(nature_code: &str) -> bool {
    nature(nature_code).is_some_and(|n| matches!(n.action, Some(ActionDeclaree::Carte { .. })))
}
