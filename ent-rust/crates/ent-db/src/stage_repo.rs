//! Stages : organismes (`ent_entreprise`) et stages déclarés (`ent_convention_stage`), lot ST0.
//!
//! Toute écriture passe par une [`TenantTx`] (RLS) et **le fait et son événement s'écrivent dans
//! la même transaction** (outbox, RS-04) : si l'écriture échoue, aucun événement ne reste. Les
//! charges utiles ne portent que des identifiants, des dates et des états (RS-05) : jamais le
//! nom d'une personne extérieure (tuteur) ni une identité.
//!
//! Les refus sont des [`StageErreur`] typées (RS-10) : l'interface sait dire ce qui ne va pas.

use chrono::NaiveDate;
use domain_eleve::{ConventionStage, DomainError, Entreprise, StageErreur};
use ent_events::{AggregateKind, MetierEventDraft};
use serde_json::json;
use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

use crate::outbox::Publier;
use crate::service::{EntDbError, RefusMetier};
use crate::tenant_tx::TenantTx;

pub struct PgStageRepo {
    pool: PgPool,
}

/// Convertit un motif typé en refus métier lisible (422 saisie invalide, 409 conflit d'état).
pub(crate) fn refus_stage(e: StageErreur) -> EntDbError {
    let nature = if e.est_conflit() {
        RefusMetier::Conflit
    } else {
        RefusMetier::Invalide
    };
    EntDbError::Refus(nature, e.message())
}

fn map_sqlx(e: sqlx::Error) -> DomainError {
    crate::pg_err::map_sqlx("stage", e)
}

/// Violation de contrainte → motif typé quand la contrainte est connue ; sinon classement commun.
pub(crate) fn ecriture(e: sqlx::Error) -> EntDbError {
    if let sqlx::Error::Database(d) = &e {
        match d.constraint().unwrap_or_default() {
            "ent_entreprise_siret_uq" => return refus_stage(StageErreur::SiretDoublon),
            "ent_convention_stage_sans_chevauchement" => {
                return refus_stage(StageErreur::Chevauchement)
            }
            "ent_entreprise_siret_cle_chk" => return refus_stage(StageErreur::SiretInvalide),
            "ent_convention_dates_chk" => return refus_stage(StageErreur::DatesInvalides),
            _ => {}
        }
    }
    EntDbError::Domain(map_sqlx(e))
}

impl PgStageRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Crée un organisme et publie `stage.entreprise` dans la même transaction.
    pub async fn insert_entreprise(&self, e: &Entreprise) -> Result<(), EntDbError> {
        let mut tx = TenantTx::begin(&self.pool, e.etablissement_id)
            .await
            .map_err(|x| EntDbError::Domain(map_sqlx(x)))?;
        if let Some(siret) = e.siret.as_deref() {
            // RS-08 : refus explicite avant l'écriture (l'index unique couvre la course).
            let deja: Option<(Uuid,)> = sqlx::query_as(
                "SELECT id FROM ent_entreprise WHERE etablissement_id = $1 AND siret = $2 LIMIT 1",
            )
            .bind(e.etablissement_id)
            .bind(siret)
            .fetch_optional(&mut *tx)
            .await
            .map_err(ecriture)?;
            if deja.is_some() {
                return Err(refus_stage(StageErreur::SiretDoublon));
            }
        }
        sqlx::query(
            r#"
            INSERT INTO ent_entreprise (
                id, etablissement_id, raison_sociale, siret, adresse,
                created_by_user_id, cree_par_personne_id, created_at, modifie_le, version
            ) VALUES ($1,$2,$3,$4,$5,$6,
                (SELECT p.id FROM ent_personne p
                  WHERE p.etablissement_id = $2 AND p.user_id = $6 LIMIT 1),
                $7,$8,$9)
            "#,
        )
        .bind(e.id)
        .bind(e.etablissement_id)
        .bind(&e.raison_sociale)
        .bind(e.siret.as_deref())
        .bind(&e.adresse)
        .bind(e.created_by_user_id)
        .bind(e.created_at)
        .bind(e.modifie_le)
        .bind(e.version)
        .execute(&mut *tx)
        .await
        .map_err(ecriture)?;
        publier(
            &mut tx,
            e.etablissement_id,
            ent_events::types::ENTREPRISE,
            AggregateKind::Entreprise,
            e.id,
            None,
            // Identifiants et état seulement : la raison sociale reste dans la table.
            json!({ "siret_renseigne": e.siret.is_some() }),
            e.created_by_user_id,
        )
        .await?;
        tx.commit()
            .await
            .map_err(|x| EntDbError::Domain(map_sqlx(x)))?;
        Ok(())
    }

    pub async fn list_entreprises(
        &self,
        etablissement_id: Uuid,
    ) -> Result<Vec<Entreprise>, DomainError> {
        let rows = sqlx::query_as::<_, EntRow>(
            r#"
            SELECT id, etablissement_id, raison_sociale, siret, adresse,
                   created_by_user_id, created_at, modifie_le, version
            FROM ent_entreprise
            WHERE etablissement_id = $1
            ORDER BY raison_sociale
            "#,
        )
        .bind(etablissement_id)
        .fetch_all(&mut *TenantTx::begin(&self.pool, etablissement_id).await.map_err(map_sqlx)?)
        .await
        .map_err(map_sqlx)?;
        Ok(rows.into_iter().map(EntRow::into_entreprise).collect())
    }

    pub async fn entreprise_labels(
        &self,
        etablissement_id: Uuid,
        entreprise_ids: &[Uuid],
    ) -> Result<std::collections::HashMap<Uuid, String>, DomainError> {
        if entreprise_ids.is_empty() {
            return Ok(std::collections::HashMap::new());
        }
        let rows: Vec<(Uuid, String)> = sqlx::query_as(
            r#"
            SELECT id, raison_sociale
            FROM ent_entreprise
            WHERE etablissement_id = $1 AND id = ANY($2)
            "#,
        )
        .bind(etablissement_id)
        .bind(entreprise_ids)
        .fetch_all(&mut *TenantTx::begin(&self.pool, etablissement_id).await.map_err(map_sqlx)?)
        .await
        .map_err(map_sqlx)?;
        Ok(rows.into_iter().collect())
    }

    pub async fn get_entreprise(
        &self,
        etablissement_id: Uuid,
        id: Uuid,
    ) -> Result<Option<Entreprise>, DomainError> {
        let row = sqlx::query_as::<_, EntRow>(
            r#"
            SELECT id, etablissement_id, raison_sociale, siret, adresse,
                   created_by_user_id, created_at, modifie_le, version
            FROM ent_entreprise
            WHERE etablissement_id = $1 AND id = $2
            "#,
        )
        .bind(etablissement_id)
        .bind(id)
        .fetch_optional(&mut *TenantTx::begin(&self.pool, etablissement_id).await.map_err(map_sqlx)?)
        .await
        .map_err(map_sqlx)?;
        Ok(row.map(EntRow::into_entreprise))
    }

    /// Crée un stage déclaré. Dans la même transaction : dates dans l'année scolaire (RS-09),
    /// pas de chevauchement avec un autre stage non annulé du même élève (RS-09), insertion,
    /// puis événement `stage.convention` (RS-04).
    pub async fn insert_convention(&self, c: &ConventionStage) -> Result<(), EntDbError> {
        let mut tx = TenantTx::begin(&self.pool, c.etablissement_id)
            .await
            .map_err(|x| EntDbError::Domain(map_sqlx(x)))?;
        verifier_annee(&mut tx, c.etablissement_id, c.debut, c.fin).await?;
        let chevauche: Option<(Uuid,)> = sqlx::query_as(
            r#"
            SELECT id FROM ent_convention_stage
             WHERE etablissement_id = $1 AND eleve_id = $2 AND statut <> 'annulee'
               AND debut <= $4 AND fin >= $3
             LIMIT 1
            "#,
        )
        .bind(c.etablissement_id)
        .bind(c.eleve_id)
        .bind(c.debut)
        .bind(c.fin)
        .fetch_optional(&mut *tx)
        .await
        .map_err(ecriture)?;
        if chevauche.is_some() {
            return Err(refus_stage(StageErreur::Chevauchement));
        }
        sqlx::query(
            r#"
            INSERT INTO ent_convention_stage (
                id, etablissement_id, eleve_id, entreprise_id, debut, fin,
                tuteur_nom, statut, created_by_user_id, cree_par_personne_id,
                created_at, modifie_le, version
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,
                (SELECT p.id FROM ent_personne p
                  WHERE p.etablissement_id = $2 AND p.user_id = $9 LIMIT 1),
                $10,$11,$12)
            "#,
        )
        .bind(c.id)
        .bind(c.etablissement_id)
        .bind(c.eleve_id)
        .bind(c.entreprise_id)
        .bind(c.debut)
        .bind(c.fin)
        .bind(&c.tuteur_nom)
        .bind(&c.statut)
        .bind(c.created_by_user_id)
        .bind(c.created_at)
        .bind(c.modifie_le)
        .bind(c.version)
        .execute(&mut *tx)
        .await
        .map_err(ecriture)?;
        publier(
            &mut tx,
            c.etablissement_id,
            ent_events::types::CONVENTION_STAGE,
            AggregateKind::ConventionStage,
            c.id,
            Some(c.eleve_id),
            // RS-05 : identifiants, dates et état. Ni nom de tuteur, ni libellé.
            json!({
                "entreprise_id": c.entreprise_id,
                "debut": c.debut.to_string(),
                "fin": c.fin.to_string(),
                "statut": c.statut,
            }),
            c.created_by_user_id,
        )
        .await?;
        tx.commit()
            .await
            .map_err(|x| EntDbError::Domain(map_sqlx(x)))?;
        Ok(())
    }

    /// Change le statut d'un stage déclaré (clôture, annulation) avec verrou optimiste (RS-07) :
    /// la version lue doit être la version courante. Événement dans la même transaction.
    pub async fn changer_statut(
        &self,
        etablissement_id: Uuid,
        id: Uuid,
        version_lue: i32,
        nouveau: &str,
        acteur: Uuid,
    ) -> Result<ConventionStage, EntDbError> {
        let mut tx = TenantTx::begin(&self.pool, etablissement_id)
            .await
            .map_err(|x| EntDbError::Domain(map_sqlx(x)))?;
        let actuel: Option<ConvRow> = sqlx::query_as(&format!(
            "SELECT {COLS_CONV} FROM ent_convention_stage
              WHERE etablissement_id = $1 AND id = $2 FOR UPDATE"
        ))
        .bind(etablissement_id)
        .bind(id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(ecriture)?;
        let Some(actuel) = actuel else {
            return Err(EntDbError::Domain(DomainError::NotFound));
        };
        if actuel.version != version_lue {
            return Err(refus_stage(StageErreur::ConflitDeVersion));
        }
        if !domain_eleve::statut_stage::transition_permise(&actuel.statut, nouveau) {
            return Err(refus_stage(StageErreur::StatutInvalide));
        }
        let maj: ConvRow = sqlx::query_as(&format!(
            "UPDATE ent_convention_stage
                SET statut = $3, version = version + 1, modifie_le = now()
              WHERE etablissement_id = $1 AND id = $2 AND version = $4
          RETURNING {COLS_CONV}"
        ))
        .bind(etablissement_id)
        .bind(id)
        .bind(nouveau)
        .bind(version_lue)
        .fetch_one(&mut *tx)
        .await
        .map_err(ecriture)?;
        publier(
            &mut tx,
            etablissement_id,
            ent_events::types::CONVENTION_STAGE_STATUT,
            AggregateKind::ConventionStage,
            id,
            Some(actuel.eleve_id),
            json!({ "de": actuel.statut, "vers": nouveau, "version": maj.version }),
            acteur,
        )
        .await?;
        tx.commit()
            .await
            .map_err(|x| EntDbError::Domain(map_sqlx(x)))?;
        Ok(maj.into_convention())
    }

    pub async fn list_conventions_eleve(
        &self,
        etablissement_id: Uuid,
        eleve_id: Uuid,
    ) -> Result<Vec<ConventionStage>, DomainError> {
        let rows = sqlx::query_as::<_, ConvRow>(&format!(
            "SELECT {COLS_CONV} FROM ent_convention_stage
              WHERE etablissement_id = $1 AND eleve_id = $2
              ORDER BY debut DESC"
        ))
        .bind(etablissement_id)
        .bind(eleve_id)
        .fetch_all(&mut *TenantTx::begin(&self.pool, etablissement_id).await.map_err(map_sqlx)?)
        .await
        .map_err(map_sqlx)?;
        Ok(rows.into_iter().map(ConvRow::into_convention).collect())
    }

    pub async fn get_convention(
        &self,
        etablissement_id: Uuid,
        id: Uuid,
    ) -> Result<Option<ConventionStage>, DomainError> {
        let row = sqlx::query_as::<_, ConvRow>(&format!(
            "SELECT {COLS_CONV} FROM ent_convention_stage
              WHERE etablissement_id = $1 AND id = $2"
        ))
        .bind(etablissement_id)
        .bind(id)
        .fetch_optional(&mut *TenantTx::begin(&self.pool, etablissement_id).await.map_err(map_sqlx)?)
        .await
        .map_err(map_sqlx)?;
        Ok(row.map(ConvRow::into_convention))
    }
}

/// Le stage doit tenir dans une année scolaire de l'établissement (RS-09). Un établissement sans
/// année paramétrée retombe sur l'année scolaire civile (1er septembre → 31 août).
async fn verifier_annee(
    conn: &mut PgConnection,
    etablissement_id: Uuid,
    debut: NaiveDate,
    fin: NaiveDate,
) -> Result<(), EntDbError> {
    let (a_des_annees, dans_une_annee): (bool, bool) = sqlx::query_as(
        r#"
        SELECT EXISTS (SELECT 1 FROM ent_annee_scolaire WHERE etablissement_id = $1),
               EXISTS (SELECT 1 FROM ent_annee_scolaire
                        WHERE etablissement_id = $1 AND date_debut <= $2 AND date_fin >= $3)
        "#,
    )
    .bind(etablissement_id)
    .bind(debut)
    .bind(fin)
    .fetch_one(conn)
    .await
    .map_err(ecriture)?;
    let ok = if a_des_annees {
        dans_une_annee
    } else {
        domain_eleve::annee_scolaire_de(debut) == domain_eleve::annee_scolaire_de(fin)
    };
    if ok {
        Ok(())
    } else {
        Err(refus_stage(StageErreur::DatesHorsAnnee))
    }
}

#[allow(clippy::too_many_arguments)]
async fn publier(
    conn: &mut PgConnection,
    etablissement_id: Uuid,
    type_name: &str,
    aggregate: AggregateKind,
    aggregate_id: Uuid,
    eleve_id: Option<Uuid>,
    payload: serde_json::Value,
    acteur: Uuid,
) -> Result<(), EntDbError> {
    conn.publier(MetierEventDraft {
        etablissement_id,
        type_name: type_name.to_string(),
        aggregate,
        aggregate_id,
        eleve_id,
        payload,
        actor_user_id: Some(acteur.to_string()),
    })
    .await?;
    Ok(())
}

#[derive(sqlx::FromRow)]
struct EntRow {
    id: Uuid,
    etablissement_id: Uuid,
    raison_sociale: String,
    siret: Option<String>,
    adresse: String,
    created_by_user_id: Uuid,
    created_at: chrono::DateTime<chrono::Utc>,
    modifie_le: chrono::DateTime<chrono::Utc>,
    version: i32,
}

impl EntRow {
    fn into_entreprise(self) -> Entreprise {
        Entreprise {
            id: self.id,
            etablissement_id: self.etablissement_id,
            raison_sociale: self.raison_sociale,
            siret: self.siret,
            adresse: self.adresse,
            created_by_user_id: self.created_by_user_id,
            created_at: self.created_at,
            modifie_le: self.modifie_le,
            version: self.version,
        }
    }
}

const COLS_CONV: &str = "id, etablissement_id, eleve_id, entreprise_id, debut, fin, tuteur_nom, \
                         statut, created_by_user_id, created_at, modifie_le, version";

#[derive(sqlx::FromRow)]
struct ConvRow {
    id: Uuid,
    etablissement_id: Uuid,
    eleve_id: Uuid,
    entreprise_id: Uuid,
    debut: chrono::NaiveDate,
    fin: chrono::NaiveDate,
    tuteur_nom: String,
    statut: String,
    created_by_user_id: Uuid,
    created_at: chrono::DateTime<chrono::Utc>,
    modifie_le: chrono::DateTime<chrono::Utc>,
    version: i32,
}

impl ConvRow {
    fn into_convention(self) -> ConventionStage {
        ConventionStage {
            id: self.id,
            etablissement_id: self.etablissement_id,
            eleve_id: self.eleve_id,
            entreprise_id: self.entreprise_id,
            debut: self.debut,
            fin: self.fin,
            tuteur_nom: self.tuteur_nom,
            statut: self.statut,
            created_by_user_id: self.created_by_user_id,
            created_at: self.created_at,
            modifie_le: self.modifie_le,
            version: self.version,
        }
    }
}
