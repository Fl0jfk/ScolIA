//! S2-5 — consommateur `d4.notifications_edt` : élèves, familles et remplaçant prévenus des
//! changements d'emploi du temps (RH-05, §4.3 `edt.exception` / `rh.absence_validee`).
//!
//! | Événement | Destinataires |
//! |---|---|
//! | `edt.exception` (annulation, changement de salle, déplacement) | élèves du cours (classe ou groupe), comptes élèves et responsables au lien actif |
//! | `edt.remplacement_organise` | idem (« assuré par un autre enseignant ») + le remplaçant |
//! | `edt.exception_levee` | idem (« cours rétabli ») |
//! | `edt.annulation_masse` | idem, **une** notification par destinataire et par lot |
//!
//! Règles de la toile : tout est **relu** dans l'établissement de l'événement (créneau,
//! exception, destinataires) ; l'état courant prime sur la charge utile (une annulation levée
//! entre-temps n'est pas notifiée) ; aucun motif libre n'est diffusé (seul le motif **public**
//! d'un lot l'est) ; idempotence par la marque du distributeur et la clé unique
//! (`source_event_id`, destinataire, kind). Consommateur **non rejouable** (l'état courant prime) ;
//! aucun envoi externe.

use std::collections::{BTreeMap, HashMap};

use chrono::{NaiveDate, NaiveTime};
use sqlx::PgConnection;
use uuid::Uuid;

use crate::outbox::{Consommateur, EvenementDistribue, FutureTraitement};

pub const CONSOMMATEUR_EDT: &str = "d4.notifications_edt";

pub struct ConsommateurNotificationsEdt;

const TYPES_EDT: &[&str] = &[
    "edt.exception",
    "edt.remplacement_organise",
    "edt.exception_levee",
    "edt.annulation_masse",
];

fn jour_fr(j: NaiveDate) -> String {
    j.format("%d/%m/%Y").to_string()
}

fn uuid_payload(evt: &EvenementDistribue, cle: &str) -> Option<Uuid> {
    evt.event()
        .payload
        .get(cle)
        .and_then(|v| v.as_str())
        .and_then(|s| Uuid::parse_str(s).ok())
}

fn jour_payload(evt: &EvenementDistribue, cle: &str) -> Option<NaiveDate> {
    evt.event()
        .payload
        .get(cle)
        .and_then(|v| v.as_str())
        .and_then(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d").ok())
}

/// (user, élève) des élèves d'un cours : comptes élèves et responsables au lien ACTIF.
/// Premier élève rencontré par destinataire (une notification par destinataire).
async fn destinataires(
    tx: &mut PgConnection,
    etab: Uuid,
    classe_code: &str,
    groupe_id: Option<Uuid>,
    jour: NaiveDate,
) -> Result<BTreeMap<Uuid, Uuid>, String> {
    let eleves: Vec<(Uuid,)> = sqlx::query_as(
        "SELECT DISTINCT e.id
           FROM ent_eleve e
           JOIN ent_classe cl ON cl.etablissement_id = e.etablissement_id AND cl.code = $2
           JOIN ent_annee_scolaire an ON an.id = cl.annee_id AND an.etablissement_id = cl.etablissement_id
                                     AND an.date_debut <= $4 AND an.date_fin >= $4
          WHERE e.etablissement_id = $1 AND (e.date_sortie IS NULL OR e.date_sortie > $4)
            AND (
              EXISTS (SELECT 1 FROM ent_affectation a
                       WHERE a.etablissement_id = e.etablissement_id AND a.eleve_id = e.id
                         AND a.date_debut <= $4 AND (a.date_fin IS NULL OR a.date_fin >= $4)
                         AND (($3::uuid IS NULL AND a.classe_id = cl.id) OR a.groupe_id = $3))
              OR ($3::uuid IS NULL AND e.classe_id = cl.id))",
    )
    .bind(etab)
    .bind(classe_code)
    .bind(groupe_id)
    .bind(jour)
    .fetch_all(&mut *tx)
    .await
    .map_err(|e| e.to_string())?;
    let ids: Vec<Uuid> = eleves.into_iter().map(|(i,)| i).collect();
    let mut out: BTreeMap<Uuid, Uuid> = BTreeMap::new();
    if ids.is_empty() {
        return Ok(out);
    }
    let comptes: Vec<(Uuid, Uuid)> = sqlx::query_as(
        "SELECT c.user_id, c.eleve_id FROM ent_eleve_compte c
           JOIN ent_membership m ON m.user_id = c.user_id AND m.etablissement_id = c.etablissement_id
          WHERE c.etablissement_id = $1 AND c.eleve_id = ANY($2) AND m.role = 'eleve'",
    )
    .bind(etab)
    .bind(&ids)
    .fetch_all(&mut *tx)
    .await
    .map_err(|e| e.to_string())?;
    let parents: Vec<(Uuid, Uuid)> = sqlx::query_as(
        "SELECT DISTINCT l.user_id, l.eleve_id FROM ent_eleve_lien_actif l
           JOIN ent_membership m ON m.user_id = l.user_id AND m.etablissement_id = l.etablissement_id
          WHERE l.etablissement_id = $1 AND l.eleve_id = ANY($2) AND m.role = 'parent'
            AND l.user_id IS NOT NULL",
    )
    .bind(etab)
    .bind(&ids)
    .fetch_all(&mut *tx)
    .await
    .map_err(|e| e.to_string())?;
    for (u, e) in comptes.into_iter().chain(parents) {
        out.entry(u).or_insert(e);
    }
    Ok(out)
}

#[allow(clippy::too_many_arguments)]
async fn inserer(
    tx: &mut PgConnection,
    etab: Uuid,
    user: Uuid,
    eleve: Option<Uuid>,
    kind: &str,
    titre: &str,
    corps: &str,
    source: Uuid,
) -> Result<(), String> {
    sqlx::query(
        "INSERT INTO ent_notification
           (id, etablissement_id, destinataire_user_id, eleve_id, kind, titre, corps, source_event_id)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8)
         ON CONFLICT (source_event_id, destinataire_user_id, kind) DO NOTHING",
    )
    .bind(Uuid::new_v4())
    .bind(etab)
    .bind(user)
    .bind(eleve)
    .bind(kind)
    .bind(titre)
    .bind(corps)
    .bind(source)
    .execute(tx)
    .await
    .map_err(|e| e.to_string())?;
    Ok(())
}

struct CoursLu {
    classe: String,
    matiere: String,
    debut: NaiveTime,
    groupe_id: Option<Uuid>,
}

async fn lire_cours(
    tx: &mut PgConnection,
    etab: Uuid,
    creneau: Uuid,
) -> Result<Option<CoursLu>, String> {
    let row: Option<(String, String, NaiveTime, Option<Uuid>)> = sqlx::query_as(
        "SELECT btrim(classe), matiere_libelle, heure_debut, groupe_id FROM ent_edt_creneau
          WHERE etablissement_id = $1 AND id = $2",
    )
    .bind(etab)
    .bind(creneau)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| e.to_string())?;
    Ok(row.map(|(classe, matiere, debut, groupe_id)| CoursLu {
        classe,
        matiere,
        debut,
        groupe_id,
    }))
}

#[allow(clippy::type_complexity)]
async fn traiter_edt(tx: &mut PgConnection, evt: &EvenementDistribue) -> Result<(), String> {
    let etab = evt.etablissement_id();
    let type_name = evt.event().type_name.clone();
    if type_name == "edt.annulation_masse" {
        return traiter_masse(tx, evt, etab).await;
    }
    let (Some(creneau), Some(jour)) = (uuid_payload(evt, "creneau_id"), jour_payload(evt, "jour"))
    else {
        return Ok(());
    };
    let Some(cours) = lire_cours(tx, etab, creneau).await? else {
        return Ok(()); // créneau d'un autre établissement ou supprimé : rien
    };
    // État courant de l'exception (source de vérité, jamais la charge utile).
    let exc: Option<(String, Option<String>, Option<Uuid>, Option<String>)> = sqlx::query_as(
        "SELECT kind, nouvelle_salle, remplacant_user_id, nature FROM ent_edt_exception
          WHERE etablissement_id = $1 AND creneau_id = $2 AND jour = $3",
    )
    .bind(etab)
    .bind(creneau)
    .bind(jour)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| e.to_string())?;
    let quand = format!("du {} à {}", jour_fr(jour), cours.debut.format("%H:%M"));
    let (kind, titre, corps): (&str, String, String) = match type_name.as_str() {
        "edt.remplacement_organise" => {
            let Some((k, _, remplacant, _)) = &exc else {
                return Ok(());
            };
            if k != "remplacement" {
                return Ok(());
            }
            if let Some(r) = remplacant {
                // Le remplaçant est relu de l'exception, membre de l'établissement.
                let membre: Option<(i32,)> = sqlx::query_as(
                    "SELECT 1 FROM ent_membership WHERE etablissement_id = $1 AND user_id = $2",
                )
                .bind(etab)
                .bind(r)
                .fetch_optional(&mut *tx)
                .await
                .map_err(|e| e.to_string())?;
                if membre.is_some() {
                    inserer(
                        tx,
                        etab,
                        *r,
                        None,
                        "edt_remplacement_assigne",
                        &format!("Remplacement : {}", cours.classe),
                        &format!(
                            "Vous assurez le cours de {} ({}) {quand}.",
                            cours.matiere, cours.classe
                        ),
                        evt.event().id,
                    )
                    .await?;
                }
            }
            (
                "edt_remplacement",
                format!("Cours remplacé : {}", cours.matiere),
                format!(
                    "Le cours de {} {quand} sera assuré par un autre enseignant.",
                    cours.matiere
                ),
            )
        }
        "edt.exception_levee" => (
            "edt_retabli",
            format!("Cours rétabli : {}", cours.matiere),
            format!(
                "Le cours de {} {quand} aura lieu normalement.",
                cours.matiere
            ),
        ),
        "edt.exception" => {
            let Some((k, salle, _, nature)) = &exc else {
                return Ok(());
            };
            let nature = nature.clone().unwrap_or_else(|| k.clone());
            match nature.as_str() {
                "annulation" => (
                    "edt_annulation",
                    format!("Cours annulé : {}", cours.matiere),
                    format!("Le cours de {} {quand} est annulé.", cours.matiere),
                ),
                "changement_salle" => {
                    let s = salle.clone().unwrap_or_default();
                    (
                        "edt_salle",
                        format!("Changement de salle : {}", cours.matiere),
                        format!(
                            "Le cours de {} {quand} aura lieu en salle {s}.",
                            cours.matiere
                        ),
                    )
                }
                "deplacement" => {
                    let d: Option<(NaiveDate, NaiveTime)> = sqlx::query_as(
                        "SELECT nouveau_jour, nouvelle_heure_debut FROM ent_edt_deplacement
                          WHERE etablissement_id = $1 AND creneau_id = $2 AND jour_origine = $3",
                    )
                    .bind(etab)
                    .bind(creneau)
                    .bind(jour)
                    .fetch_optional(&mut *tx)
                    .await
                    .map_err(|e| e.to_string())?;
                    let Some((nj, nh)) = d else { return Ok(()) };
                    (
                        "edt_deplacement",
                        format!("Cours déplacé : {}", cours.matiere),
                        format!(
                            "Le cours de {} {quand} est déplacé au {} à {}.",
                            cours.matiere,
                            jour_fr(nj),
                            nh.format("%H:%M")
                        ),
                    )
                }
                _ => return Ok(()),
            }
        }
        _ => return Ok(()),
    };
    let dest = destinataires(tx, etab, &cours.classe, cours.groupe_id, jour).await?;
    for (user, eleve) in dest {
        inserer(
            tx,
            etab,
            user,
            Some(eleve),
            kind,
            &titre,
            &corps,
            evt.event().id,
        )
        .await?;
    }
    Ok(())
}

async fn traiter_masse(
    tx: &mut PgConnection,
    evt: &EvenementDistribue,
    etab: Uuid,
) -> Result<(), String> {
    let Some(lot) = uuid_payload(evt, "lot_id") else {
        return Ok(());
    };
    let lot_row: Option<(NaiveDate, NaiveDate, String)> = sqlx::query_as(
        "SELECT du, au, motif_public FROM ent_edt_annulation_masse WHERE etablissement_id = $1 AND id = $2",
    )
    .bind(etab)
    .bind(lot)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| e.to_string())?;
    let Some((du, au, motif)) = lot_row else {
        return Ok(());
    };
    // Séances du lot encore annulées, regroupées par cible (classe, groupe).
    let groupes: Vec<(String, Option<Uuid>, NaiveDate, i64)> = sqlx::query_as(
        "SELECT btrim(c.classe), c.groupe_id, min(x.jour), count(*)
           FROM ent_edt_exception x
           JOIN ent_edt_creneau c ON c.id = x.creneau_id AND c.etablissement_id = x.etablissement_id
          WHERE x.etablissement_id = $1 AND x.lot_id = $2 AND x.kind = 'annulation'
          GROUP BY btrim(c.classe), c.groupe_id",
    )
    .bind(etab)
    .bind(lot)
    .fetch_all(&mut *tx)
    .await
    .map_err(|e| e.to_string())?;
    let mut par_user: HashMap<Uuid, (Uuid, i64)> = HashMap::new();
    for (classe, groupe, premier_jour, n) in groupes {
        for (user, eleve) in destinataires(tx, etab, &classe, groupe, premier_jour).await? {
            let e = par_user.entry(user).or_insert((eleve, 0));
            e.1 += n;
        }
    }
    let periode = if du == au {
        format!("le {}", jour_fr(du))
    } else {
        format!("du {} au {}", jour_fr(du), jour_fr(au))
    };
    for (user, (eleve, n)) in par_user {
        let pluriel = if n > 1 {
            "cours sont annulés"
        } else {
            "cours est annulé"
        };
        inserer(
            tx,
            etab,
            user,
            Some(eleve),
            "edt_annulation_masse",
            "Cours annulés",
            &format!("{n} {pluriel} {periode} ({}).", motif.trim()),
            evt.event().id,
        )
        .await?;
    }
    Ok(())
}

impl Consommateur for ConsommateurNotificationsEdt {
    fn nom(&self) -> &'static str {
        CONSOMMATEUR_EDT
    }

    fn types(&self) -> &'static [&'static str] {
        TYPES_EDT
    }

    /// Non rejouable : la notification dépend de l'état de l'exception **au moment du
    /// traitement** ; un rejeu tardif notifierait un état qui n'était pas celui de l'événement.
    fn rejouable(&self) -> bool {
        false
    }

    fn traiter<'a>(
        &'a self,
        tx: &'a mut PgConnection,
        evt: &'a EvenementDistribue,
    ) -> FutureTraitement<'a> {
        Box::pin(traiter_edt(tx, evt))
    }
}
