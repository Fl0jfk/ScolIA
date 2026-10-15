use chrono::NaiveDate;
use domain_eleve::{
    BourseStatus, BourseType, DispenseSource, DomainError, EleveBourse, EleveDispenseRepas,
    EleveGrilleRepas, JoursMasque, PassageRepas, PassageSource, RepasActivite, RepasActiviteKind,
};
use sqlx::PgPool;
use uuid::Uuid;

pub struct PgRepasRepo {
    pool: PgPool,
}

impl PgRepasRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn upsert_activite(&self, a: &RepasActivite) -> Result<(), DomainError> {
        sqlx::query(
            r#"
            INSERT INTO ent_repas_activite (
                id, etablissement_id, code, libelle, kind, actif, created_at
            ) VALUES ($1,$2,$3,$4,$5,$6, now())
            ON CONFLICT (id) DO UPDATE SET
                code = EXCLUDED.code,
                libelle = EXCLUDED.libelle,
                kind = EXCLUDED.kind,
                actif = EXCLUDED.actif
            WHERE ent_repas_activite.etablissement_id = EXCLUDED.etablissement_id
            "#,
        )
        .bind(a.id)
        .bind(a.etablissement_id)
        .bind(&a.code)
        .bind(&a.libelle)
        .bind(a.kind.as_str())
        .bind(a.actif)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(())
    }

    pub async fn list_activites(
        &self,
        etablissement_id: Uuid,
    ) -> Result<Vec<RepasActivite>, DomainError> {
        let rows = sqlx::query_as::<_, ActiviteRow>(
            r#"
            SELECT id, etablissement_id, code, libelle, kind, actif
            FROM ent_repas_activite
            WHERE etablissement_id = $1
            ORDER BY kind, code
            "#,
        )
        .bind(etablissement_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(rows.into_iter().filter_map(ActiviteRow::into_act).collect())
    }

    pub async fn get_activite(
        &self,
        etablissement_id: Uuid,
        activite_id: Uuid,
    ) -> Result<Option<RepasActivite>, DomainError> {
        let row = sqlx::query_as::<_, ActiviteRow>(
            r#"
            SELECT id, etablissement_id, code, libelle, kind, actif
            FROM ent_repas_activite
            WHERE etablissement_id = $1 AND id = $2
            "#,
        )
        .bind(etablissement_id)
        .bind(activite_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(row.and_then(ActiviteRow::into_act))
    }

    pub async fn upsert_grille(&self, g: &EleveGrilleRepas) -> Result<(), DomainError> {
        sqlx::query(
            r#"
            INSERT INTO ent_eleve_grille_repas (
                id, etablissement_id, eleve_id, activite_id,
                lun, mar, mer, jeu, ven, sam, dim,
                date_debut, date_fin, created_at
            ) VALUES (
                $1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13, now()
            )
            ON CONFLICT (id) DO UPDATE SET
                activite_id = EXCLUDED.activite_id,
                lun = EXCLUDED.lun, mar = EXCLUDED.mar, mer = EXCLUDED.mer,
                jeu = EXCLUDED.jeu, ven = EXCLUDED.ven, sam = EXCLUDED.sam,
                dim = EXCLUDED.dim,
                date_debut = EXCLUDED.date_debut,
                date_fin = EXCLUDED.date_fin
            WHERE ent_eleve_grille_repas.etablissement_id = EXCLUDED.etablissement_id
            "#,
        )
        .bind(g.id)
        .bind(g.etablissement_id)
        .bind(g.eleve_id)
        .bind(g.activite_id)
        .bind(g.jours.lun)
        .bind(g.jours.mar)
        .bind(g.jours.mer)
        .bind(g.jours.jeu)
        .bind(g.jours.ven)
        .bind(g.jours.sam)
        .bind(g.jours.dim)
        .bind(g.date_debut)
        .bind(g.date_fin)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(())
    }

    pub async fn list_grilles_eleve(
        &self,
        etablissement_id: Uuid,
        eleve_id: Uuid,
    ) -> Result<Vec<EleveGrilleRepas>, DomainError> {
        let rows = sqlx::query_as::<_, GrilleRow>(
            r#"
            SELECT id, etablissement_id, eleve_id, activite_id,
                   lun, mar, mer, jeu, ven, sam, dim, date_debut, date_fin
            FROM ent_eleve_grille_repas
            WHERE etablissement_id = $1 AND eleve_id = $2
            ORDER BY date_debut, activite_id
            "#,
        )
        .bind(etablissement_id)
        .bind(eleve_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(rows.into_iter().map(GrilleRow::into_grille).collect())
    }

    pub async fn list_grilles_etab(
        &self,
        etablissement_id: Uuid,
    ) -> Result<Vec<EleveGrilleRepas>, DomainError> {
        let rows = sqlx::query_as::<_, GrilleRow>(
            r#"
            SELECT id, etablissement_id, eleve_id, activite_id,
                   lun, mar, mer, jeu, ven, sam, dim, date_debut, date_fin
            FROM ent_eleve_grille_repas
            WHERE etablissement_id = $1
            ORDER BY eleve_id, date_debut, activite_id
            "#,
        )
        .bind(etablissement_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(rows.into_iter().map(GrilleRow::into_grille).collect())
    }

    pub async fn upsert_bourse_type(&self, t: &BourseType) -> Result<(), DomainError> {
        sqlx::query(
            r#"
            INSERT INTO ent_bourse_type (
                id, etablissement_id, code, libelle, actif, created_at
            ) VALUES ($1,$2,$3,$4,$5, now())
            ON CONFLICT (id) DO UPDATE SET
                code = EXCLUDED.code,
                libelle = EXCLUDED.libelle,
                actif = EXCLUDED.actif
            WHERE ent_bourse_type.etablissement_id = EXCLUDED.etablissement_id
            "#,
        )
        .bind(t.id)
        .bind(t.etablissement_id)
        .bind(&t.code)
        .bind(&t.libelle)
        .bind(t.actif)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(())
    }

    pub async fn list_bourse_types(
        &self,
        etablissement_id: Uuid,
    ) -> Result<Vec<BourseType>, DomainError> {
        let rows = sqlx::query_as::<_, BourseTypeRow>(
            r#"
            SELECT id, etablissement_id, code, libelle, actif
            FROM ent_bourse_type
            WHERE etablissement_id = $1
            ORDER BY code
            "#,
        )
        .bind(etablissement_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(rows.into_iter().map(BourseTypeRow::into_type).collect())
    }

    pub async fn get_bourse_type(
        &self,
        etablissement_id: Uuid,
        bourse_type_id: Uuid,
    ) -> Result<Option<BourseType>, DomainError> {
        let row = sqlx::query_as::<_, BourseTypeRow>(
            r#"
            SELECT id, etablissement_id, code, libelle, actif
            FROM ent_bourse_type
            WHERE etablissement_id = $1 AND id = $2
            "#,
        )
        .bind(etablissement_id)
        .bind(bourse_type_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(row.map(BourseTypeRow::into_type))
    }

    pub async fn upsert_bourse(&self, b: &EleveBourse) -> Result<(), DomainError> {
        sqlx::query(
            r#"
            INSERT INTO ent_eleve_bourse (
                id, etablissement_id, eleve_id, bourse_type_id, echelon,
                avec_procuration, status, date_debut, date_fin, created_at
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9, now())
            ON CONFLICT (id) DO UPDATE SET
                echelon = EXCLUDED.echelon,
                avec_procuration = EXCLUDED.avec_procuration,
                status = EXCLUDED.status,
                date_debut = EXCLUDED.date_debut,
                date_fin = EXCLUDED.date_fin
            WHERE ent_eleve_bourse.etablissement_id = EXCLUDED.etablissement_id
            "#,
        )
        .bind(b.id)
        .bind(b.etablissement_id)
        .bind(b.eleve_id)
        .bind(b.bourse_type_id)
        .bind(&b.echelon)
        .bind(b.avec_procuration)
        .bind(b.status.as_str())
        .bind(b.date_debut)
        .bind(b.date_fin)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(())
    }

    pub async fn get_bourse(
        &self,
        etablissement_id: Uuid,
        bourse_id: Uuid,
    ) -> Result<Option<EleveBourse>, DomainError> {
        let row = sqlx::query_as::<_, BourseRow>(
            r#"
            SELECT id, etablissement_id, eleve_id, bourse_type_id, echelon,
                   avec_procuration, status, date_debut, date_fin
            FROM ent_eleve_bourse
            WHERE etablissement_id = $1 AND id = $2
            "#,
        )
        .bind(etablissement_id)
        .bind(bourse_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(row.and_then(BourseRow::into_bourse))
    }

    pub async fn list_bourses_eleve(
        &self,
        etablissement_id: Uuid,
        eleve_id: Uuid,
    ) -> Result<Vec<EleveBourse>, DomainError> {
        let rows = sqlx::query_as::<_, BourseRow>(
            r#"
            SELECT id, etablissement_id, eleve_id, bourse_type_id, echelon,
                   avec_procuration, status, date_debut, date_fin
            FROM ent_eleve_bourse
            WHERE etablissement_id = $1 AND eleve_id = $2
            ORDER BY date_debut DESC, echelon
            "#,
        )
        .bind(etablissement_id)
        .bind(eleve_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(rows.into_iter().filter_map(BourseRow::into_bourse).collect())
    }

    pub async fn insert_dispense(&self, d: &EleveDispenseRepas) -> Result<(), DomainError> {
        let res = sqlx::query(
            r#"
            INSERT INTO ent_eleve_dispense_repas (
                id, etablissement_id, eleve_id, activite_id, jour, motif, source, annulee, created_at
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8, now())
            "#,
        )
        .bind(d.id)
        .bind(d.etablissement_id)
        .bind(d.eleve_id)
        .bind(d.activite_id)
        .bind(d.jour)
        .bind(&d.motif)
        .bind(d.source.as_str())
        .bind(d.annulee)
        .execute(&self.pool)
        .await;
        match res {
            Ok(_) => Ok(()),
            Err(sqlx::Error::Database(db)) if db.constraint().is_some() => {
                Err(DomainError::DispenseRepasDejaPosee)
            }
            Err(err) => Err(map_sqlx(err)),
        }
    }

    pub async fn update_dispense_annulee(&self, d: &EleveDispenseRepas) -> Result<(), DomainError> {
        let n = sqlx::query(
            r#"
            UPDATE ent_eleve_dispense_repas
            SET annulee = $3
            WHERE etablissement_id = $1 AND id = $2
            "#,
        )
        .bind(d.etablissement_id)
        .bind(d.id)
        .bind(d.annulee)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?
        .rows_affected();
        if n == 0 {
            return Err(DomainError::NotFound);
        }
        Ok(())
    }

    pub async fn get_dispense(
        &self,
        etablissement_id: Uuid,
        dispense_id: Uuid,
    ) -> Result<Option<EleveDispenseRepas>, DomainError> {
        let row = sqlx::query_as::<_, DispenseRow>(
            r#"
            SELECT id, etablissement_id, eleve_id, activite_id, jour, motif, source, annulee
            FROM ent_eleve_dispense_repas
            WHERE etablissement_id = $1 AND id = $2
            "#,
        )
        .bind(etablissement_id)
        .bind(dispense_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(row.and_then(DispenseRow::into_dispense))
    }

    pub async fn list_dispenses_eleve(
        &self,
        etablissement_id: Uuid,
        eleve_id: Uuid,
        actives_only: bool,
    ) -> Result<Vec<EleveDispenseRepas>, DomainError> {
        let rows = if actives_only {
            sqlx::query_as::<_, DispenseRow>(
                r#"
                SELECT id, etablissement_id, eleve_id, activite_id, jour, motif, source, annulee
                FROM ent_eleve_dispense_repas
                WHERE etablissement_id = $1 AND eleve_id = $2 AND annulee = false
                ORDER BY jour DESC, activite_id
                "#,
            )
            .bind(etablissement_id)
            .bind(eleve_id)
            .fetch_all(&self.pool)
            .await
            .map_err(map_sqlx)?
        } else {
            sqlx::query_as::<_, DispenseRow>(
                r#"
                SELECT id, etablissement_id, eleve_id, activite_id, jour, motif, source, annulee
                FROM ent_eleve_dispense_repas
                WHERE etablissement_id = $1 AND eleve_id = $2
                ORDER BY jour DESC, activite_id
                "#,
            )
            .bind(etablissement_id)
            .bind(eleve_id)
            .fetch_all(&self.pool)
            .await
            .map_err(map_sqlx)?
        };
        Ok(rows
            .into_iter()
            .filter_map(DispenseRow::into_dispense)
            .collect())
    }

    pub async fn list_dispenses_periode(
        &self,
        etablissement_id: Uuid,
        eleve_id: Uuid,
        debut: NaiveDate,
        fin: NaiveDate,
    ) -> Result<Vec<EleveDispenseRepas>, DomainError> {
        let rows = sqlx::query_as::<_, DispenseRow>(
            r#"
            SELECT id, etablissement_id, eleve_id, activite_id, jour, motif, source, annulee
            FROM ent_eleve_dispense_repas
            WHERE etablissement_id = $1 AND eleve_id = $2
              AND annulee = false
              AND jour >= $3 AND jour <= $4
            ORDER BY jour, activite_id
            "#,
        )
        .bind(etablissement_id)
        .bind(eleve_id)
        .bind(debut)
        .bind(fin)
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(rows
            .into_iter()
            .filter_map(DispenseRow::into_dispense)
            .collect())
    }

    pub async fn list_dispenses_etab_periode(
        &self,
        etablissement_id: Uuid,
        debut: NaiveDate,
        fin: NaiveDate,
    ) -> Result<Vec<EleveDispenseRepas>, DomainError> {
        let rows = sqlx::query_as::<_, DispenseRow>(
            r#"
            SELECT id, etablissement_id, eleve_id, activite_id, jour, motif, source, annulee
            FROM ent_eleve_dispense_repas
            WHERE etablissement_id = $1
              AND annulee = false
              AND jour >= $2 AND jour <= $3
            ORDER BY jour, eleve_id, activite_id
            "#,
        )
        .bind(etablissement_id)
        .bind(debut)
        .bind(fin)
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(rows
            .into_iter()
            .filter_map(DispenseRow::into_dispense)
            .collect())
    }

    pub async fn insert_passage(&self, p: &PassageRepas) -> Result<(), DomainError> {
        let res = sqlx::query(
            r#"
            INSERT INTO ent_repas_passage (
                id, etablissement_id, eleve_id, activite_id, jour, badge_lu, source, annule, created_at
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8, now())
            "#,
        )
        .bind(p.id)
        .bind(p.etablissement_id)
        .bind(p.eleve_id)
        .bind(p.activite_id)
        .bind(p.jour)
        .bind(&p.badge_lu)
        .bind(p.source.as_str())
        .bind(p.annule)
        .execute(&self.pool)
        .await;
        match res {
            Ok(_) => Ok(()),
            Err(sqlx::Error::Database(db)) if db.constraint().is_some() => {
                Err(DomainError::PassageBadgeDejaEnregistre)
            }
            Err(err) => Err(map_sqlx(err)),
        }
    }

    pub async fn has_passage_actif(
        &self,
        etablissement_id: Uuid,
        eleve_id: Uuid,
        activite_id: Uuid,
        jour: NaiveDate,
    ) -> Result<bool, DomainError> {
        let n: (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*)::bigint
            FROM ent_repas_passage
            WHERE etablissement_id = $1 AND eleve_id = $2
              AND activite_id = $3 AND jour = $4 AND annule = false
            "#,
        )
        .bind(etablissement_id)
        .bind(eleve_id)
        .bind(activite_id)
        .bind(jour)
        .fetch_one(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(n.0 > 0)
    }

    pub async fn list_passages_jour(
        &self,
        etablissement_id: Uuid,
        jour: NaiveDate,
    ) -> Result<Vec<PassageRepas>, DomainError> {
        let rows = sqlx::query_as::<_, PassageRow>(
            r#"
            SELECT id, etablissement_id, eleve_id, activite_id, jour, badge_lu, source, annule
            FROM ent_repas_passage
            WHERE etablissement_id = $1 AND jour = $2 AND annule = false
            ORDER BY created_at DESC
            "#,
        )
        .bind(etablissement_id)
        .bind(jour)
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(rows
            .into_iter()
            .filter_map(PassageRow::into_passage)
            .collect())
    }

    pub async fn list_passages_eleve_periode(
        &self,
        etablissement_id: Uuid,
        eleve_id: Uuid,
        debut: NaiveDate,
        fin: NaiveDate,
    ) -> Result<Vec<PassageRepas>, DomainError> {
        let rows = sqlx::query_as::<_, PassageRow>(
            r#"
            SELECT id, etablissement_id, eleve_id, activite_id, jour, badge_lu, source, annule
            FROM ent_repas_passage
            WHERE etablissement_id = $1 AND eleve_id = $2
              AND jour >= $3 AND jour <= $4 AND annule = false
            ORDER BY jour DESC, created_at DESC
            "#,
        )
        .bind(etablissement_id)
        .bind(eleve_id)
        .bind(debut)
        .bind(fin)
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(rows
            .into_iter()
            .filter_map(PassageRow::into_passage)
            .collect())
    }

    pub async fn count_passages_eleve_periode(
        &self,
        etablissement_id: Uuid,
        eleve_id: Uuid,
        debut: NaiveDate,
        fin: NaiveDate,
        activite_id: Option<Uuid>,
    ) -> Result<u32, DomainError> {
        let n: (i64,) = if let Some(aid) = activite_id {
            sqlx::query_as(
                r#"
                SELECT COUNT(*)::bigint
                FROM ent_repas_passage
                WHERE etablissement_id = $1 AND eleve_id = $2
                  AND annule = false AND jour >= $3 AND jour <= $4
                  AND activite_id = $5
                "#,
            )
            .bind(etablissement_id)
            .bind(eleve_id)
            .bind(debut)
            .bind(fin)
            .bind(aid)
            .fetch_one(&self.pool)
            .await
            .map_err(map_sqlx)?
        } else {
            sqlx::query_as(
                r#"
                SELECT COUNT(*)::bigint
                FROM ent_repas_passage
                WHERE etablissement_id = $1 AND eleve_id = $2
                  AND annule = false AND jour >= $3 AND jour <= $4
                "#,
            )
            .bind(etablissement_id)
            .bind(eleve_id)
            .bind(debut)
            .bind(fin)
            .fetch_one(&self.pool)
            .await
            .map_err(map_sqlx)?
        };
        Ok(u32::try_from(n.0.max(0)).unwrap_or(0))
    }
}

#[derive(Debug, sqlx::FromRow)]
struct PassageRow {
    id: Uuid,
    etablissement_id: Uuid,
    eleve_id: Uuid,
    activite_id: Uuid,
    jour: NaiveDate,
    badge_lu: String,
    source: String,
    annule: bool,
}

impl PassageRow {
    fn into_passage(self) -> Option<PassageRepas> {
        Some(PassageRepas {
            id: self.id,
            etablissement_id: self.etablissement_id,
            eleve_id: self.eleve_id,
            activite_id: self.activite_id,
            jour: self.jour,
            badge_lu: self.badge_lu,
            source: PassageSource::parse(&self.source)?,
            annule: self.annule,
        })
    }
}

#[derive(Debug, sqlx::FromRow)]
struct ActiviteRow {
    id: Uuid,
    etablissement_id: Uuid,
    code: String,
    libelle: String,
    kind: String,
    actif: bool,
}

impl ActiviteRow {
    fn into_act(self) -> Option<RepasActivite> {
        Some(RepasActivite {
            id: self.id,
            etablissement_id: self.etablissement_id,
            code: self.code,
            libelle: self.libelle,
            kind: RepasActiviteKind::parse(&self.kind)?,
            actif: self.actif,
        })
    }
}

#[derive(Debug, sqlx::FromRow)]
struct GrilleRow {
    id: Uuid,
    etablissement_id: Uuid,
    eleve_id: Uuid,
    activite_id: Uuid,
    lun: bool,
    mar: bool,
    mer: bool,
    jeu: bool,
    ven: bool,
    sam: bool,
    dim: bool,
    date_debut: chrono::NaiveDate,
    date_fin: Option<chrono::NaiveDate>,
}

impl GrilleRow {
    fn into_grille(self) -> EleveGrilleRepas {
        EleveGrilleRepas {
            id: self.id,
            etablissement_id: self.etablissement_id,
            eleve_id: self.eleve_id,
            activite_id: self.activite_id,
            jours: JoursMasque {
                lun: self.lun,
                mar: self.mar,
                mer: self.mer,
                jeu: self.jeu,
                ven: self.ven,
                sam: self.sam,
                dim: self.dim,
            },
            date_debut: self.date_debut,
            date_fin: self.date_fin,
        }
    }
}

#[derive(Debug, sqlx::FromRow)]
struct BourseTypeRow {
    id: Uuid,
    etablissement_id: Uuid,
    code: String,
    libelle: String,
    actif: bool,
}

impl BourseTypeRow {
    fn into_type(self) -> BourseType {
        BourseType {
            id: self.id,
            etablissement_id: self.etablissement_id,
            code: self.code,
            libelle: self.libelle,
            actif: self.actif,
        }
    }
}

#[derive(Debug, sqlx::FromRow)]
struct BourseRow {
    id: Uuid,
    etablissement_id: Uuid,
    eleve_id: Uuid,
    bourse_type_id: Uuid,
    echelon: String,
    avec_procuration: bool,
    status: String,
    date_debut: chrono::NaiveDate,
    date_fin: Option<chrono::NaiveDate>,
}

impl BourseRow {
    fn into_bourse(self) -> Option<EleveBourse> {
        Some(EleveBourse {
            id: self.id,
            etablissement_id: self.etablissement_id,
            eleve_id: self.eleve_id,
            bourse_type_id: self.bourse_type_id,
            echelon: self.echelon,
            avec_procuration: self.avec_procuration,
            status: BourseStatus::parse(&self.status)?,
            date_debut: self.date_debut,
            date_fin: self.date_fin,
        })
    }
}

#[derive(Debug, sqlx::FromRow)]
struct DispenseRow {
    id: Uuid,
    etablissement_id: Uuid,
    eleve_id: Uuid,
    activite_id: Uuid,
    jour: NaiveDate,
    motif: String,
    source: String,
    annulee: bool,
}

impl DispenseRow {
    fn into_dispense(self) -> Option<EleveDispenseRepas> {
        Some(EleveDispenseRepas {
            id: self.id,
            etablissement_id: self.etablissement_id,
            eleve_id: self.eleve_id,
            activite_id: self.activite_id,
            jour: self.jour,
            motif: self.motif,
            source: DispenseSource::parse(&self.source)?,
            annulee: self.annulee,
        })
    }
}

fn map_sqlx(err: sqlx::Error) -> DomainError {
    eprintln!("[ent-db] repas/bourse sqlx error: {err}");
    DomainError::NotFound
}
