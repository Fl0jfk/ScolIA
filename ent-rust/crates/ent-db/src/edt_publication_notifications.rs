//! Socle 10, lot T5 — notifications de publication, **par le socle 8 seulement** (`notifier_personne`), un message par
//! personne (§14.2) :
//!
//! | Événement | Destinataires | Message |
//! |---|---|---|
//! | `edt.version_publiee` (temps 1) | intervenants des cours de la version | « nouvel emploi du temps à partir du … » + date de diffusion |
//! | `edt.version_diffusee` (temps 2) | élèves (comptes) et responsables des élèves **mineurs** (lien actif, sans interdiction de communication) des cours de la version, à la date d'effet | « votre emploi du temps change à partir du … » |
//! | `edt.version_individuelle` | l'élève (compte) et ses responsables si mineur | « votre emploi du temps est modifié à partir du … » |
//!
//! Jamais de motif (la version individuelle est un motif sensible : UPE2A, ULIS…), jamais de nom d'intervenant. L'état courant
//! prime : une version retirée avant la distribution n'est pas annoncée. Consommateur non rejouable (l'effet externe ne se
//! rattrape pas) ; idempotent par `(source_event_id, destinataire, kind)`.

use std::collections::BTreeSet;

use chrono::{DateTime, NaiveDate, Utc};
use sqlx::PgConnection;
use uuid::Uuid;

use crate::edt_publication::{EVT_VERSION_DIFFUSEE, EVT_VERSION_INDIVIDUELLE, EVT_VERSION_PUBLIEE};
use crate::notifications::{notifier_personne, EnvoiAPlanifier};
use crate::outbox::{Consommateur, EvenementDistribue, FutureTraitement};

pub const CONSOMMATEUR_EDT_T5: &str = "d4.notifications_edt_t5";

pub struct ConsommateurNotificationsEdtT5;

const TYPES_T5: &[&str] = &[
    EVT_VERSION_PUBLIEE,
    EVT_VERSION_DIFFUSEE,
    EVT_VERSION_INDIVIDUELLE,
];

fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}

fn uuid_payload(evt: &EvenementDistribue, cle: &str) -> Option<Uuid> {
    evt.event()
        .payload
        .get(cle)
        .and_then(|v| v.as_str())
        .and_then(|s| Uuid::parse_str(s).ok())
}

fn jour_fr(j: NaiveDate) -> String {
    j.format("%d/%m/%Y").to_string()
}

async fn envoyer(
    conn: &mut PgConnection,
    etab: Uuid,
    evt: &EvenementDistribue,
    user: Uuid,
    eleve: Option<Uuid>,
    kind: &str,
    titre: &str,
    corps: &str,
    famille: bool,
    maintenant: DateTime<Utc>,
) -> Result<(), String> {
    notifier_personne(
        conn,
        EnvoiAPlanifier {
            etablissement_id: etab,
            user_id: user,
            kind,
            titre,
            corps,
            eleve_id: eleve,
            notification_id: None,
            source_type: "evenement",
            source_id: evt.event().id,
            cle_dedup: format!("edt-t5:{}:{}", evt.event().id, user),
            droit_requis: famille.then_some("lien"),
            regroupable: true,
            canaux_voulus: &["email", "push"],
        },
        evt.event().id,
        maintenant,
    )
    .await
    .map(|_| ())
    .map_err(err)
}

/// Élèves (comptes) et responsables d'un ensemble d'élèves : `(user_id, eleve_id, famille)`.
async fn comptes_et_familles(
    conn: &mut PgConnection,
    etab: Uuid,
    eleves: &[Uuid],
    jour: NaiveDate,
) -> Result<Vec<(Uuid, Uuid, bool)>, String> {
    let mut out: Vec<(Uuid, Uuid, bool)> = Vec::new();
    let comptes: Vec<(Uuid, Uuid)> = sqlx::query_as(
        "SELECT c.user_id, c.eleve_id FROM ent_eleve_compte c
           JOIN ent_membership m ON m.user_id = c.user_id AND m.etablissement_id = c.etablissement_id
          WHERE c.etablissement_id = $1 AND c.eleve_id = ANY($2) AND m.role = 'eleve'",
    )
    .bind(etab)
    .bind(eleves)
    .fetch_all(&mut *conn)
    .await
    .map_err(err)?;
    out.extend(comptes.into_iter().map(|(u, e)| (u, e, false)));
    let parents: Vec<(Uuid, Uuid)> = sqlx::query_as(
        "SELECT DISTINCT l.user_id, l.eleve_id FROM ent_eleve_lien_actif l
           JOIN ent_membership m ON m.user_id = l.user_id AND m.etablissement_id = l.etablissement_id
           JOIN ent_eleve e ON e.id = l.eleve_id AND e.etablissement_id = l.etablissement_id
          WHERE l.etablissement_id = $1 AND l.eleve_id = ANY($2) AND m.role = 'parent' AND l.user_id IS NOT NULL
            AND (e.date_naissance IS NULL OR e.date_naissance > ($3::date - interval '18 years'))
            AND NOT EXISTS (SELECT 1 FROM ent_eleve_interdiction i
                             WHERE i.etablissement_id = l.etablissement_id AND i.eleve_id = l.eleve_id
                               AND i.personne_id = l.personne_id AND i.levee_le IS NULL
                               AND i.du <= $3 AND (i.au IS NULL OR i.au >= $3) AND 'communication' = ANY(i.objet))",
    )
    .bind(etab)
    .bind(eleves)
    .bind(jour)
    .fetch_all(&mut *conn)
    .await
    .map_err(err)?;
    out.extend(parents.into_iter().map(|(u, e)| (u, e, true)));
    Ok(out)
}

async fn traiter_t5(conn: &mut PgConnection, evt: &EvenementDistribue) -> Result<(), String> {
    let etab = evt.event().etablissement_id;
    let maintenant = evt.event().created_at;
    match evt.event().type_name.as_str() {
        EVT_VERSION_INDIVIDUELLE => {
            let Some(id) = uuid_payload(evt, "individuelle_id") else {
                return Ok(());
            };
            let r: Option<(Uuid, NaiveDate, bool)> = sqlx::query_as(
                "SELECT eleve_id, a_partir_du, (annulee_le IS NOT NULL) FROM ent_edt_individuelle
                  WHERE etablissement_id = $1 AND id = $2",
            )
            .bind(etab)
            .bind(id)
            .fetch_optional(&mut *conn)
            .await
            .map_err(err)?;
            let Some((eleve, du, annulee)) = r else {
                return Ok(());
            };
            if annulee {
                return Ok(());
            }
            let corps = format!("Votre emploi du temps est modifié à partir du {}. Consultez-le dans « Mon emploi du temps ».", jour_fr(du));
            for (user, e, famille) in comptes_et_familles(conn, etab, &[eleve], du).await? {
                envoyer(
                    conn,
                    etab,
                    evt,
                    user,
                    Some(e),
                    "edt_version_individuelle",
                    "Emploi du temps modifié",
                    &corps,
                    famille,
                    maintenant,
                )
                .await?;
            }
        }
        t @ (EVT_VERSION_PUBLIEE | EVT_VERSION_DIFFUSEE) => {
            let Some(version) = uuid_payload(evt, "version_id") else {
                return Ok(());
            };
            let v: Option<(String, Option<NaiveDate>, Option<DateTime<Utc>>, bool)> = sqlx::query_as(
                "SELECT etat, valable_du, diffusion_familles_le, provisoire FROM ent_edt_version
                  WHERE etablissement_id = $1 AND id = $2",
            )
            .bind(etab)
            .bind(version)
            .fetch_optional(&mut *conn)
            .await
            .map_err(err)?;
            let Some((etat, Some(effet), diffusion, provisoire)) = v else {
                return Ok(());
            };
            // L'état courant prime : annonce retirée, version archivée ou abandonnée : rien ne part.
            let annoncee: bool = sqlx::query_scalar(
                "SELECT annoncee_le IS NOT NULL FROM ent_edt_version WHERE etablissement_id = $1 AND id = $2",
            )
            .bind(etab)
            .bind(version)
            .fetch_one(&mut *conn)
            .await
            .map_err(err)?;
            if !annoncee || !matches!(etat.as_str(), "brouillon" | "publiee" | "remplacee") {
                return Ok(());
            }
            let mention = if provisoire { " (provisoire)" } else { "" };
            if t == EVT_VERSION_PUBLIEE {
                let users: Vec<Uuid> = sqlx::query_scalar(
                    "SELECT DISTINCT s.prof_user_id FROM ent_edt_placement p
                       JOIN ent_service_enseignement s ON s.cours_id = p.cours_id AND s.etablissement_id = p.etablissement_id
                      WHERE p.etablissement_id = $1 AND p.version_id = $2 AND s.etat = 'ouvert' AND s.prof_user_id IS NOT NULL",
                )
                .bind(etab)
                .bind(version)
                .fetch_all(&mut *conn)
                .await
                .map_err(err)?;
                let membres: Vec<Uuid> = sqlx::query_scalar(
                    "SELECT DISTINCT user_id FROM ent_membership WHERE etablissement_id = $1 AND user_id = ANY($2)",
                )
                .bind(etab)
                .bind(&users)
                .fetch_all(&mut *conn)
                .await
                .map_err(err)?;
                let suite = match diffusion {
                    Some(d) => format!(
                        " Il sera diffusé aux familles et aux élèves à partir du {}.",
                        jour_fr(d.date_naive())
                    ),
                    None => String::new(),
                };
                let corps = format!(
                    "Un nouvel emploi du temps{mention} s'applique à partir du {}. Vérifiez-le dans « Mon emploi du temps » et signalez toute erreur à la direction.{suite}",
                    jour_fr(effet)
                );
                for u in membres.into_iter().collect::<BTreeSet<_>>() {
                    envoyer(
                        conn,
                        etab,
                        evt,
                        u,
                        None,
                        "edt_version_publiee",
                        "Nouvel emploi du temps publié",
                        &corps,
                        false,
                        maintenant,
                    )
                    .await?;
                }
            } else {
                let eleves: Vec<Uuid> = sqlx::query_scalar(
                    "SELECT DISTINCT eleve_id FROM ent_public_cours_jour($1, $2)
                      WHERE cours_id IN (SELECT cours_id FROM ent_edt_placement WHERE etablissement_id = $1 AND version_id = $3)",
                )
                .bind(etab)
                .bind(effet)
                .bind(version)
                .fetch_all(&mut *conn)
                .await
                .map_err(err)?;
                let corps = format!(
                    "Votre emploi du temps{mention} change à partir du {}. Consultez-le dans « Mon emploi du temps ».",
                    jour_fr(effet)
                );
                for (user, e, famille) in comptes_et_familles(conn, etab, &eleves, effet).await? {
                    envoyer(
                        conn,
                        etab,
                        evt,
                        user,
                        Some(e),
                        "edt_version_diffusee",
                        "Nouvel emploi du temps",
                        &corps,
                        famille,
                        maintenant,
                    )
                    .await?;
                }
            }
        }
        _ => {}
    }
    Ok(())
}

impl Consommateur for ConsommateurNotificationsEdtT5 {
    fn nom(&self) -> &'static str {
        CONSOMMATEUR_EDT_T5
    }

    fn types(&self) -> &'static [&'static str] {
        TYPES_T5
    }

    /// Non rejouable : l'état courant de la version prime sur la charge utile et l'effet externe ne se rattrape pas.
    fn rejouable(&self) -> bool {
        false
    }

    fn traiter<'a>(
        &'a self,
        tx: &'a mut PgConnection,
        evt: &'a EvenementDistribue,
    ) -> FutureTraitement<'a> {
        Box::pin(traiter_t5(tx, evt))
    }
}
