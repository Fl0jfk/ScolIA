use chrono::{DateTime, Utc};
use chrono::NaiveDate;
use domain_eleve::{
    DomainError, Echeance, Encaissement, Facture, FactureLigne, FactureStatut, MandatSepa,
    MandatSepaStatut, MoyenPaiement, OgecLigneExport, PrelevementSepa, PrelevementStatut,
    Quittance, RappelCanal, RappelFacture, RappelKind, SepaLigneExport,
};
use crate::champs_chiffres::ChampsSensibles;
use crate::tenant_tx::TenantTx;
use sqlx::PgPool;
use uuid::Uuid;

pub struct PgFacRepo {
    pool: PgPool,
    champs: ChampsSensibles,
}

const T_MANDAT: &str = "ent_fac_sepa_mandat";

impl PgFacRepo {
    pub fn new(pool: PgPool) -> Self {
        Self {
            pool,
            champs: ChampsSensibles::depuis_env(),
        }
    }

    pub async fn foyer_exists(
        &self,
        etablissement_id: Uuid,
        foyer_id: Uuid,
    ) -> Result<bool, DomainError> {
        let mut tx = TenantTx::begin(&self.pool, etablissement_id).await.map_err(map_sqlx)?;
        let row: Option<(Uuid,)> = sqlx::query_as(
            r#"
            SELECT id FROM ent_foyer
            WHERE etablissement_id = $1 AND id = $2
            "#,
        )
        .bind(etablissement_id)
        .bind(foyer_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        tx.commit().await.map_err(map_sqlx)?;
        Ok(row.is_some())
    }

    pub async fn insert_facture(
        &self,
        facture: &Facture,
        lignes: &[FactureLigne],
    ) -> Result<(), DomainError> {
        let mut tx = TenantTx::begin(&self.pool, facture.etablissement_id).await.map_err(map_sqlx)?;
        sqlx::query(
            r#"
            INSERT INTO ent_fac_facture (
                id, etablissement_id, foyer_id, numero, libelle, statut,
                total_centimes, nature, annee_scolaire,
                created_by_user_id, created_at, emise_at, soldee_at
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13)
            "#,
        )
        .bind(facture.id)
        .bind(facture.etablissement_id)
        .bind(facture.foyer_id)
        .bind(&facture.numero)
        .bind(&facture.libelle)
        .bind(facture.statut.as_str())
        .bind(facture.total_centimes)
        .bind(&facture.nature)
        .bind(&facture.annee_scolaire)
        .bind(facture.created_by_user_id)
        .bind(facture.created_at)
        .bind(facture.emise_at)
        .bind(facture.soldee_at)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx)?;

        for l in lignes {
            sqlx::query(
                r#"
                INSERT INTO ent_fac_ligne (
                    id, etablissement_id, facture_id, eleve_id,
                    libelle, quantite, montant_unitaire_centimes
                ) VALUES ($1,$2,$3,$4,$5,$6,$7)
                "#,
            )
            .bind(l.id)
            .bind(l.etablissement_id)
            .bind(l.facture_id)
            .bind(l.eleve_id)
            .bind(&l.libelle)
            .bind(l.quantite)
            .bind(l.montant_unitaire_centimes)
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx)?;
        }
        tx.commit().await.map_err(map_sqlx)?;
        Ok(())
    }

    /// Émission d'un brouillon (F0, FD-05, R-F3) : le **serveur** attribue le numéro
    /// définitif dans la série continue de l'établissement (même compteur que le
    /// chemin `/finances` : `facture-AAAA`, `F2026-000042`), dans la transaction de
    /// l'émission. Renvoie le numéro attribué.
    pub async fn update_facture_emise(&self, facture: &Facture) -> Result<String, DomainError> {
        let mut tx = TenantTx::begin(&self.pool, facture.etablissement_id).await.map_err(map_sqlx)?;
        let annee = crate::finances::aujourd_hui().format("%Y").to_string();
        let (n,): (i64,) = sqlx::query_as(
            "INSERT INTO ent_fin_compteur (etablissement_id, cle, dernier) VALUES ($1, $2, 1)
             ON CONFLICT (etablissement_id, cle) DO UPDATE SET dernier = ent_fin_compteur.dernier + 1
             RETURNING dernier",
        )
        .bind(facture.etablissement_id)
        .bind(format!("facture-{annee}"))
        .fetch_one(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        let numero = format!("F{annee}-{n:06}");
        let res = sqlx::query(
            r#"
            UPDATE ent_fac_facture
            SET statut = $3, emise_at = $4, numero = $5
            WHERE etablissement_id = $1 AND id = $2 AND statut = 'brouillon'
            "#,
        )
        .bind(facture.etablissement_id)
        .bind(facture.id)
        .bind(facture.statut.as_str())
        .bind(facture.emise_at)
        .bind(&numero)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        if res.rows_affected() == 0 {
            // Rien n'est émis : la transaction (donc le numéro consommé) est annulée,
            // la série reste sans trou.
            return Err(DomainError::FactureDejaEmise);
        }
        tx.commit().await.map_err(map_sqlx)?;
        Ok(numero)
    }

    pub async fn sum_encaissements(
        &self,
        etablissement_id: Uuid,
        facture_id: Uuid,
    ) -> Result<i64, DomainError> {
        let mut tx = TenantTx::begin(&self.pool, etablissement_id).await.map_err(map_sqlx)?;
        let row: (i64,) = sqlx::query_as(
            r#"
            SELECT COALESCE(SUM(montant_centimes), 0)::bigint
            FROM ent_fac_encaissement
            WHERE etablissement_id = $1 AND facture_id = $2
            "#,
        )
        .bind(etablissement_id)
        .bind(facture_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        tx.commit().await.map_err(map_sqlx)?;
        Ok(row.0)
    }

    pub async fn count_echeances(
        &self,
        etablissement_id: Uuid,
        facture_id: Uuid,
    ) -> Result<i64, DomainError> {
        let mut tx = TenantTx::begin(&self.pool, etablissement_id).await.map_err(map_sqlx)?;
        let row: (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*)::bigint FROM ent_fac_echeance
            WHERE etablissement_id = $1 AND facture_id = $2
            "#,
        )
        .bind(etablissement_id)
        .bind(facture_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        tx.commit().await.map_err(map_sqlx)?;
        Ok(row.0)
    }

    pub async fn insert_echeances(&self, echeances: &[Echeance]) -> Result<(), DomainError> {
        let Some(premiere) = echeances.first() else {
            return Ok(());
        };
        let mut tx = TenantTx::begin(&self.pool, premiere.etablissement_id)
            .await
            .map_err(map_sqlx)?;
        for e in echeances {
            sqlx::query(
                r#"
                INSERT INTO ent_fac_echeance (
                    id, etablissement_id, facture_id, rang, libelle,
                    montant_centimes, bourse_centimes, date_echeance, created_at
                ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)
                "#,
            )
            .bind(e.id)
            .bind(e.etablissement_id)
            .bind(e.facture_id)
            .bind(e.rang)
            .bind(&e.libelle)
            .bind(e.montant_centimes)
            .bind(e.bourse_centimes)
            .bind(e.date_echeance)
            .bind(e.created_at)
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx)?;
        }
        tx.commit().await.map_err(map_sqlx)?;
        Ok(())
    }

    pub async fn list_echeances(
        &self,
        etablissement_id: Uuid,
        facture_id: Uuid,
    ) -> Result<Vec<Echeance>, DomainError> {
        let mut tx = TenantTx::begin(&self.pool, etablissement_id).await.map_err(map_sqlx)?;
        let rows = sqlx::query_as::<_, EchRow>(
            r#"
            SELECT id, etablissement_id, facture_id, rang, libelle,
                   montant_centimes, bourse_centimes, date_echeance, created_at
            FROM ent_fac_echeance
            WHERE etablissement_id = $1 AND facture_id = $2
            ORDER BY rang ASC
            "#,
        )
        .bind(etablissement_id)
        .bind(facture_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        tx.commit().await.map_err(map_sqlx)?;
        Ok(rows.into_iter().map(EchRow::into_ech).collect())
    }

    pub async fn list_quittances_facture(
        &self,
        etablissement_id: Uuid,
        facture_id: Uuid,
    ) -> Result<Vec<Quittance>, DomainError> {
        let mut tx = TenantTx::begin(&self.pool, etablissement_id).await.map_err(map_sqlx)?;
        let rows = sqlx::query_as::<_, QuitRow>(
            r#"
            SELECT id, etablissement_id, foyer_id, facture_id, encaissement_id,
                   numero, montant_centimes, moyen, emise_at, created_by_user_id
            FROM ent_fac_quittance
            WHERE etablissement_id = $1 AND facture_id = $2
            ORDER BY emise_at ASC
            "#,
        )
        .bind(etablissement_id)
        .bind(facture_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        tx.commit().await.map_err(map_sqlx)?;
        Ok(rows.into_iter().map(QuitRow::into_quit).collect())
    }

    pub async fn list_quittances_foyers(
        &self,
        etablissement_id: Uuid,
        foyer_ids: &[Uuid],
    ) -> Result<Vec<Quittance>, DomainError> {
        let mut tx = TenantTx::begin(&self.pool, etablissement_id).await.map_err(map_sqlx)?;
        if foyer_ids.is_empty() {
            return Ok(Vec::new());
        }
        let rows = sqlx::query_as::<_, QuitRow>(
            r#"
            SELECT id, etablissement_id, foyer_id, facture_id, encaissement_id,
                   numero, montant_centimes, moyen, emise_at, created_by_user_id
            FROM ent_fac_quittance
            WHERE etablissement_id = $1 AND foyer_id = ANY($2)
            ORDER BY emise_at DESC
            LIMIT 80
            "#,
        )
        .bind(etablissement_id)
        .bind(foyer_ids)
        .fetch_all(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        tx.commit().await.map_err(map_sqlx)?;
        Ok(rows.into_iter().map(QuitRow::into_quit).collect())
    }

    pub async fn get_quittance(
        &self,
        etablissement_id: Uuid,
        quittance_id: Uuid,
    ) -> Result<Option<Quittance>, DomainError> {
        let mut tx = TenantTx::begin(&self.pool, etablissement_id).await.map_err(map_sqlx)?;
        let row = sqlx::query_as::<_, QuitRow>(
            r#"
            SELECT id, etablissement_id, foyer_id, facture_id, encaissement_id,
                   numero, montant_centimes, moyen, emise_at, created_by_user_id
            FROM ent_fac_quittance
            WHERE etablissement_id = $1 AND id = $2
            "#,
        )
        .bind(etablissement_id)
        .bind(quittance_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        tx.commit().await.map_err(map_sqlx)?;
        Ok(row.map(QuitRow::into_quit))
    }

    pub async fn list_encaissements(
        &self,
        etablissement_id: Uuid,
        facture_id: Uuid,
    ) -> Result<Vec<Encaissement>, DomainError> {
        let mut tx = TenantTx::begin(&self.pool, etablissement_id).await.map_err(map_sqlx)?;
        let rows = sqlx::query_as::<_, EncRow>(
            r#"
            SELECT id, etablissement_id, facture_id, montant_centimes, moyen,
                   reference, encaisse_at, created_by_user_id, created_at
            FROM ent_fac_encaissement
            WHERE etablissement_id = $1 AND facture_id = $2
            ORDER BY encaisse_at ASC
            "#,
        )
        .bind(etablissement_id)
        .bind(facture_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        tx.commit().await.map_err(map_sqlx)?;
        Ok(rows.into_iter().map(EncRow::into_enc).collect())
    }

    pub async fn get_facture(
        &self,
        etablissement_id: Uuid,
        facture_id: Uuid,
    ) -> Result<Option<Facture>, DomainError> {
        let mut tx = TenantTx::begin(&self.pool, etablissement_id).await.map_err(map_sqlx)?;
        let row = sqlx::query_as::<_, FactureRow>(
            r#"
            SELECT id, etablissement_id, foyer_id, numero, libelle, statut,
                   total_centimes, nature, annee_scolaire,
                   created_by_user_id, created_at, emise_at, soldee_at
            FROM ent_fac_facture
            WHERE etablissement_id = $1 AND id = $2
            "#,
        )
        .bind(etablissement_id)
        .bind(facture_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        tx.commit().await.map_err(map_sqlx)?;
        Ok(row.map(FactureRow::into_facture))
    }

    pub async fn list_lignes(
        &self,
        etablissement_id: Uuid,
        facture_id: Uuid,
    ) -> Result<Vec<FactureLigne>, DomainError> {
        let mut tx = TenantTx::begin(&self.pool, etablissement_id).await.map_err(map_sqlx)?;
        let rows = sqlx::query_as::<_, LigneRow>(
            r#"
            SELECT id, etablissement_id, facture_id, eleve_id,
                   libelle, quantite, montant_unitaire_centimes
            FROM ent_fac_ligne
            WHERE etablissement_id = $1 AND facture_id = $2
            ORDER BY libelle
            "#,
        )
        .bind(etablissement_id)
        .bind(facture_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        tx.commit().await.map_err(map_sqlx)?;
        Ok(rows.into_iter().map(LigneRow::into_ligne).collect())
    }

    pub async fn list_factures_etab(
        &self,
        etablissement_id: Uuid,
        limit: i64,
    ) -> Result<Vec<Facture>, DomainError> {
        let mut tx = TenantTx::begin(&self.pool, etablissement_id).await.map_err(map_sqlx)?;
        let rows = sqlx::query_as::<_, FactureRow>(
            r#"
            SELECT id, etablissement_id, foyer_id, numero, libelle, statut,
                   total_centimes, nature, annee_scolaire,
                   created_by_user_id, created_at, emise_at, soldee_at
            FROM ent_fac_facture
            WHERE etablissement_id = $1
            ORDER BY created_at DESC
            LIMIT $2
            "#,
        )
        .bind(etablissement_id)
        .bind(limit)
        .fetch_all(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        tx.commit().await.map_err(map_sqlx)?;
        Ok(rows.into_iter().map(FactureRow::into_facture).collect())
    }

    pub async fn list_factures_foyers(
        &self,
        etablissement_id: Uuid,
        foyer_ids: &[Uuid],
        only_emises: bool,
    ) -> Result<Vec<Facture>, DomainError> {
        let mut tx = TenantTx::begin(&self.pool, etablissement_id).await.map_err(map_sqlx)?;
        if foyer_ids.is_empty() {
            return Ok(Vec::new());
        }
        let rows = if only_emises {
            sqlx::query_as::<_, FactureRow>(
                r#"
                SELECT id, etablissement_id, foyer_id, numero, libelle, statut,
                       total_centimes, nature, annee_scolaire,
                       created_by_user_id, created_at, emise_at, soldee_at
                FROM ent_fac_facture
                WHERE etablissement_id = $1
                  AND foyer_id = ANY($2)
                  AND statut IN ('emise', 'soldee')
                ORDER BY created_at DESC
                "#,
            )
            .bind(etablissement_id)
            .bind(foyer_ids)
            .fetch_all(&mut *tx)
            .await
            .map_err(map_sqlx)?
        } else {
            sqlx::query_as::<_, FactureRow>(
                r#"
                SELECT id, etablissement_id, foyer_id, numero, libelle, statut,
                       total_centimes, nature, annee_scolaire,
                       created_by_user_id, created_at, emise_at, soldee_at
                FROM ent_fac_facture
                WHERE etablissement_id = $1 AND foyer_id = ANY($2)
                ORDER BY created_at DESC
                "#,
            )
            .bind(etablissement_id)
            .bind(foyer_ids)
            .fetch_all(&mut *tx)
            .await
            .map_err(map_sqlx)?
        };
        tx.commit().await.map_err(map_sqlx)?;
        Ok(rows.into_iter().map(FactureRow::into_facture).collect())
    }

    /// Journal des encaissements sur une période (export OGEC).
    pub async fn list_encaissements_periode(
        &self,
        etablissement_id: Uuid,
        debut: chrono::NaiveDate,
        fin: chrono::NaiveDate,
    ) -> Result<Vec<OgecLigneExport>, DomainError> {
        let mut tx = TenantTx::begin(&self.pool, etablissement_id).await.map_err(map_sqlx)?;
        // F0 (FD-24) : la période se lit en heure de Paris (un encaissement de 0 h 30
        // appartient au jour même, pas à la veille).
        if fin < debut {
            return Err(DomainError::OgecPeriodeInvalide);
        }
        let rows = sqlx::query_as::<_, OgecRow>(
            r#"
            SELECT e.encaisse_at,
                   f.numero AS facture_numero,
                   f.libelle AS facture_libelle,
                   f.foyer_id,
                   q.numero AS quittance_numero,
                   e.moyen,
                   e.reference,
                   e.montant_centimes,
                   f.nature AS nature_facture
            FROM ent_fac_encaissement e
            INNER JOIN ent_fac_facture f
                ON f.id = e.facture_id AND f.etablissement_id = e.etablissement_id
            LEFT JOIN ent_fac_quittance q
                ON q.encaissement_id = e.id AND q.etablissement_id = e.etablissement_id
            WHERE e.etablissement_id = $1
              AND e.encaisse_at >= ($2::date)::timestamp AT TIME ZONE 'Europe/Paris'
              AND e.encaisse_at < (($3::date + 1)::timestamp AT TIME ZONE 'Europe/Paris')
            ORDER BY e.encaisse_at ASC, f.numero ASC
            "#,
        )
        .bind(etablissement_id)
        .bind(debut)
        .bind(fin)
        .fetch_all(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        tx.commit().await.map_err(map_sqlx)?;
        Ok(rows.into_iter().filter_map(OgecRow::into_ligne).collect())
    }

    /// Somme des encaissements sur factures acompte réinscription (émises/soldées).
    pub async fn sum_encaisse_acompte_reinscription(
        &self,
        etablissement_id: Uuid,
        foyer_id: Uuid,
        annee_scolaire: &str,
    ) -> Result<i64, DomainError> {
        let mut tx = TenantTx::begin(&self.pool, etablissement_id).await.map_err(map_sqlx)?;
        let row: (i64,) = sqlx::query_as(
            r#"
            SELECT COALESCE(SUM(e.montant_centimes), 0)::bigint
            FROM ent_fac_encaissement e
            INNER JOIN ent_fac_facture f
                ON f.id = e.facture_id
               AND f.etablissement_id = e.etablissement_id
            WHERE e.etablissement_id = $1
              AND f.foyer_id = $2
              AND f.nature = 'reinscription_acompte'
              AND f.annee_scolaire = $3
              AND f.statut IN ('emise', 'soldee')
            "#,
        )
        .bind(etablissement_id)
        .bind(foyer_id)
        .bind(annee_scolaire)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        tx.commit().await.map_err(map_sqlx)?;
        Ok(row.0)
    }

    pub async fn insert_sepa_mandat(&self, m: &MandatSepa) -> Result<(), DomainError> {
        // D3 : IBAN, BIC et titulaire chiffrés (AAD = table + colonne + id du mandat).
        let iban = self.champs.sceller(T_MANDAT, "iban", m.id, &m.iban)?;
        let bic = self.champs.sceller_opt(T_MANDAT, "bic", m.id, m.bic.as_deref())?;
        let titulaire = self.champs.sceller(T_MANDAT, "titulaire", m.id, &m.titulaire)?;
        let mut tx = TenantTx::begin(&self.pool, m.etablissement_id).await.map_err(map_sqlx)?;
        sqlx::query(
            r#"
            INSERT INTO ent_fac_sepa_mandat (
                id, etablissement_id, foyer_id, rum, iban, bic, titulaire,
                statut, signe_at, created_by_user_id, created_at
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11)
            "#,
        )
        .bind(m.id)
        .bind(m.etablissement_id)
        .bind(m.foyer_id)
        .bind(&m.rum)
        .bind(&iban)
        .bind(&bic)
        .bind(&titulaire)
        .bind(m.statut.as_str())
        .bind(m.signe_at)
        .bind(m.created_by_user_id)
        .bind(m.created_at)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        tx.commit().await.map_err(map_sqlx)?;
        Ok(())
    }

    pub async fn get_sepa_mandat(
        &self,
        etablissement_id: Uuid,
        mandat_id: Uuid,
    ) -> Result<Option<MandatSepa>, DomainError> {
        let mut tx = TenantTx::begin(&self.pool, etablissement_id).await.map_err(map_sqlx)?;
        let row = sqlx::query_as::<_, MandatRow>(
            r#"
            SELECT id, etablissement_id, foyer_id, rum, iban, bic, titulaire,
                   statut, signe_at, created_by_user_id, created_at
            FROM ent_fac_sepa_mandat
            WHERE etablissement_id = $1 AND id = $2
            "#,
        )
        .bind(etablissement_id)
        .bind(mandat_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        tx.commit().await.map_err(map_sqlx)?;
        row.and_then(|r| r.into_mandat(&self.champs).transpose()).transpose()
    }

    pub async fn get_sepa_mandat_actif_foyer(
        &self,
        etablissement_id: Uuid,
        foyer_id: Uuid,
    ) -> Result<Option<MandatSepa>, DomainError> {
        let mut tx = TenantTx::begin(&self.pool, etablissement_id).await.map_err(map_sqlx)?;
        let row = sqlx::query_as::<_, MandatRow>(
            r#"
            SELECT id, etablissement_id, foyer_id, rum, iban, bic, titulaire,
                   statut, signe_at, created_by_user_id, created_at
            FROM ent_fac_sepa_mandat
            WHERE etablissement_id = $1 AND foyer_id = $2 AND statut = 'actif'
            LIMIT 1
            "#,
        )
        .bind(etablissement_id)
        .bind(foyer_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        tx.commit().await.map_err(map_sqlx)?;
        row.and_then(|r| r.into_mandat(&self.champs).transpose()).transpose()
    }

    pub async fn list_sepa_mandats_etab(
        &self,
        etablissement_id: Uuid,
        limit: i64,
    ) -> Result<Vec<MandatSepa>, DomainError> {
        let mut tx = TenantTx::begin(&self.pool, etablissement_id).await.map_err(map_sqlx)?;
        let rows = sqlx::query_as::<_, MandatRow>(
            r#"
            SELECT id, etablissement_id, foyer_id, rum, iban, bic, titulaire,
                   statut, signe_at, created_by_user_id, created_at
            FROM ent_fac_sepa_mandat
            WHERE etablissement_id = $1
            ORDER BY created_at DESC
            LIMIT $2
            "#,
        )
        .bind(etablissement_id)
        .bind(limit)
        .fetch_all(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        tx.commit().await.map_err(map_sqlx)?;
        rows.into_iter()
            .filter_map(|r| r.into_mandat(&self.champs).transpose())
            .collect()
    }

    pub async fn update_sepa_mandat_statut(&self, m: &MandatSepa) -> Result<(), DomainError> {
        let mut tx = TenantTx::begin(&self.pool, m.etablissement_id).await.map_err(map_sqlx)?;
        let n = sqlx::query(
            r#"
            UPDATE ent_fac_sepa_mandat
            SET statut = $3
            WHERE etablissement_id = $1 AND id = $2
            "#,
        )
        .bind(m.etablissement_id)
        .bind(m.id)
        .bind(m.statut.as_str())
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx)?
        .rows_affected();
        if n == 0 {
            return Err(DomainError::NotFound);
        }
        tx.commit().await.map_err(map_sqlx)?;
        Ok(())
    }

    pub async fn insert_sepa_prelevement(&self, p: &PrelevementSepa) -> Result<(), DomainError> {
        let mut tx = TenantTx::begin(&self.pool, p.etablissement_id).await.map_err(map_sqlx)?;
        sqlx::query(
            r#"
            INSERT INTO ent_fac_sepa_prelevement (
                id, etablissement_id, mandat_id, facture_id, foyer_id,
                montant_centimes, date_execution, end_to_end_id, statut,
                encaissement_id, created_by_user_id, created_at
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12)
            "#,
        )
        .bind(p.id)
        .bind(p.etablissement_id)
        .bind(p.mandat_id)
        .bind(p.facture_id)
        .bind(p.foyer_id)
        .bind(p.montant_centimes)
        .bind(p.date_execution)
        .bind(&p.end_to_end_id)
        .bind(p.statut.as_str())
        .bind(p.encaissement_id)
        .bind(p.created_by_user_id)
        .bind(p.created_at)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        tx.commit().await.map_err(map_sqlx)?;
        Ok(())
    }

    pub async fn get_sepa_prelevement(
        &self,
        etablissement_id: Uuid,
        prelevement_id: Uuid,
    ) -> Result<Option<PrelevementSepa>, DomainError> {
        let mut tx = TenantTx::begin(&self.pool, etablissement_id).await.map_err(map_sqlx)?;
        let row = sqlx::query_as::<_, PrelevementRow>(
            r#"
            SELECT id, etablissement_id, mandat_id, facture_id, foyer_id,
                   montant_centimes, date_execution, end_to_end_id, statut,
                   encaissement_id, motif_rejet, rejete_at,
                   created_by_user_id, created_at
            FROM ent_fac_sepa_prelevement
            WHERE etablissement_id = $1 AND id = $2
            "#,
        )
        .bind(etablissement_id)
        .bind(prelevement_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        tx.commit().await.map_err(map_sqlx)?;
        Ok(row.and_then(PrelevementRow::into_prelevement))
    }

    /// Lookup prélèvement par end_to_end_id (rapprochement relevé bancaire).
    pub async fn get_sepa_prelevement_by_e2e(
        &self,
        etablissement_id: Uuid,
        end_to_end_id: &str,
    ) -> Result<Option<PrelevementSepa>, DomainError> {
        let mut tx = TenantTx::begin(&self.pool, etablissement_id).await.map_err(map_sqlx)?;
        let row = sqlx::query_as::<_, PrelevementRow>(
            r#"
            SELECT id, etablissement_id, mandat_id, facture_id, foyer_id,
                   montant_centimes, date_execution, end_to_end_id, statut,
                   encaissement_id, motif_rejet, rejete_at,
                   created_by_user_id, created_at
            FROM ent_fac_sepa_prelevement
            WHERE etablissement_id = $1 AND end_to_end_id = $2
            "#,
        )
        .bind(etablissement_id)
        .bind(end_to_end_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        tx.commit().await.map_err(map_sqlx)?;
        Ok(row.and_then(PrelevementRow::into_prelevement))
    }

    /// Historique exporté/exécuté sur un mandat (pour SeqTp FRST vs RCUR).
    pub async fn count_sepa_historique_mandat(
        &self,
        etablissement_id: Uuid,
        mandat_id: Uuid,
    ) -> Result<i64, DomainError> {
        let mut tx = TenantTx::begin(&self.pool, etablissement_id).await.map_err(map_sqlx)?;
        let row: (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*)::bigint
            FROM ent_fac_sepa_prelevement
            WHERE etablissement_id = $1
              AND mandat_id = $2
              AND statut IN ('exporte', 'execute')
            "#,
        )
        .bind(etablissement_id)
        .bind(mandat_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        tx.commit().await.map_err(map_sqlx)?;
        Ok(row.0)
    }

    /// Somme des prélèvements encore ouverts (prévu/exporté) sur une facture.
    pub async fn sum_prelevements_ouverts(
        &self,
        etablissement_id: Uuid,
        facture_id: Uuid,
    ) -> Result<i64, DomainError> {
        let mut tx = TenantTx::begin(&self.pool, etablissement_id).await.map_err(map_sqlx)?;
        let row: (i64,) = sqlx::query_as(
            r#"
            SELECT COALESCE(SUM(montant_centimes), 0)::bigint
            FROM ent_fac_sepa_prelevement
            WHERE etablissement_id = $1
              AND facture_id = $2
              AND statut IN ('prevu', 'exporte')
            "#,
        )
        .bind(etablissement_id)
        .bind(facture_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        tx.commit().await.map_err(map_sqlx)?;
        Ok(row.0)
    }

    pub async fn list_sepa_prelevements_facture(
        &self,
        etablissement_id: Uuid,
        facture_id: Uuid,
    ) -> Result<Vec<PrelevementSepa>, DomainError> {
        let mut tx = TenantTx::begin(&self.pool, etablissement_id).await.map_err(map_sqlx)?;
        let rows = sqlx::query_as::<_, PrelevementRow>(
            r#"
            SELECT id, etablissement_id, mandat_id, facture_id, foyer_id,
                   montant_centimes, date_execution, end_to_end_id, statut,
                   encaissement_id, motif_rejet, rejete_at,
                   created_by_user_id, created_at
            FROM ent_fac_sepa_prelevement
            WHERE etablissement_id = $1 AND facture_id = $2
            ORDER BY date_execution ASC, created_at ASC
            "#,
        )
        .bind(etablissement_id)
        .bind(facture_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        tx.commit().await.map_err(map_sqlx)?;
        Ok(rows
            .into_iter()
            .filter_map(PrelevementRow::into_prelevement)
            .collect())
    }

    /// Prélèvements exportables (prévu) sur une plage de dates d'exécution.
    pub async fn list_sepa_prelevements_a_exporter(
        &self,
        etablissement_id: Uuid,
        debut: NaiveDate,
        fin: NaiveDate,
    ) -> Result<Vec<SepaLigneExport>, DomainError> {
        let mut tx = TenantTx::begin(&self.pool, etablissement_id).await.map_err(map_sqlx)?;
        let rows = sqlx::query_as::<_, SepaExportRow>(
            r#"
            SELECT p.id, p.etablissement_id, p.mandat_id, p.facture_id, p.foyer_id,
                   p.montant_centimes, p.date_execution, p.end_to_end_id, p.statut,
                   p.encaissement_id, p.motif_rejet, p.rejete_at,
                   p.created_by_user_id, p.created_at,
                   m.rum, m.iban, m.bic, m.titulaire, m.signe_at AS mandat_signe_at,
                   f.numero AS facture_numero
            FROM ent_fac_sepa_prelevement p
            INNER JOIN ent_fac_sepa_mandat m
                ON m.id = p.mandat_id AND m.etablissement_id = p.etablissement_id
            INNER JOIN ent_fac_facture f
                ON f.id = p.facture_id AND f.etablissement_id = p.etablissement_id
            WHERE p.etablissement_id = $1
              AND p.statut = 'prevu'
              AND p.date_execution >= $2
              AND p.date_execution <= $3
              -- F0 (FD-04) : seulement les prélèvements du chemin historique (par facture) ;
              -- ceux du chemin par échéance s'exportent par /finances.
              AND p.echeance_id IS NULL
              -- F0 (FD-03, R-P6) : jamais sans pré-notification.
              AND p.prenotification_id IS NOT NULL
              -- F0 (FD-10, R-P9) : jamais sur un mandat révoqué.
              AND m.statut = 'actif'
            ORDER BY p.date_execution ASC, p.created_at ASC
            "#,
        )
        .bind(etablissement_id)
        .bind(debut)
        .bind(fin)
        .fetch_all(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        tx.commit().await.map_err(map_sqlx)?;
        rows.into_iter()
            .filter_map(|r| r.into_ligne(&self.champs).transpose())
            .collect()
    }

    /// RS3-03 : un foyer en paiement `manuel` ne se prélève pas (défaut : prélèvement).
    pub async fn foyer_en_paiement_manuel(
        &self,
        etablissement_id: Uuid,
        foyer_id: Uuid,
    ) -> Result<bool, DomainError> {
        let mut tx = TenantTx::begin(&self.pool, etablissement_id).await.map_err(map_sqlx)?;
        let r: Option<(String,)> = sqlx::query_as(
            "SELECT mode_paiement FROM ent_fin_compte_payeur WHERE etablissement_id = $1 AND foyer_id = $2",
        )
        .bind(etablissement_id)
        .bind(foyer_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        tx.commit().await.map_err(map_sqlx)?;
        Ok(r.map(|r| r.0 == "manuel").unwrap_or(false))
    }

    pub async fn update_sepa_prelevement(&self, p: &PrelevementSepa) -> Result<(), DomainError> {
        let mut tx = TenantTx::begin(&self.pool, p.etablissement_id).await.map_err(map_sqlx)?;
        let n = sqlx::query(
            r#"
            UPDATE ent_fac_sepa_prelevement
            SET statut = $3,
                encaissement_id = $4,
                motif_rejet = $5,
                rejete_at = $6
            WHERE etablissement_id = $1 AND id = $2
              AND statut IN ('prevu', 'exporte')
            "#,
        )
        .bind(p.etablissement_id)
        .bind(p.id)
        .bind(p.statut.as_str())
        .bind(p.encaissement_id)
        .bind(&p.motif_rejet)
        .bind(p.rejete_at)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx)?
        .rows_affected();
        if n == 0 {
            // RS3-02 : garde de statut dans le WHERE — un prélèvement déjà
            // exécuté ou rejeté ne se modifie plus (aucun double effet).
            let existe: Option<(Uuid,)> = sqlx::query_as(
                "SELECT id FROM ent_fac_sepa_prelevement WHERE etablissement_id = $1 AND id = $2",
            )
            .bind(p.etablissement_id)
            .bind(p.id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(map_sqlx)?;
            return Err(if existe.is_some() {
                DomainError::SepaPrelevementInvalide
            } else {
                DomainError::NotFound
            });
        }
        tx.commit().await.map_err(map_sqlx)?;
        Ok(())
    }

    /// RS3-02 : exécution d'un prélèvement SEPA **atomique et idempotente**.
    ///
    /// Une seule transaction : verrou de ligne (`FOR UPDATE`) sur le prélèvement
    /// puis sur la facture (sérialise les exécutions concurrentes d'une même
    /// facture), contrôle de statut **avant** toute écriture, puis encaissement,
    /// solde de la facture, quittance et passage du prélèvement à `execute`.
    /// Un prélèvement déjà `execute` renvoie `Ok((prelevement, None))` : aucun effet.
    /// Un prélèvement `rejete` renvoie `SepaPrelevementInvalide`.
    pub async fn executer_sepa_atomique(
        &self,
        etablissement_id: Uuid,
        prelevement_id: Uuid,
        actor_user_id: Uuid,
    ) -> Result<(PrelevementSepa, Option<(Encaissement, Facture, Quittance)>), DomainError> {
        let mut tx = TenantTx::begin(&self.pool, etablissement_id).await.map_err(map_sqlx)?;
        let row = sqlx::query_as::<_, PrelevementRow>(
            r#"
            SELECT id, etablissement_id, mandat_id, facture_id, foyer_id,
                   montant_centimes, date_execution, end_to_end_id, statut,
                   encaissement_id, motif_rejet, rejete_at,
                   created_by_user_id, created_at
            FROM ent_fac_sepa_prelevement
            WHERE etablissement_id = $1 AND id = $2
            FOR UPDATE
            "#,
        )
        .bind(etablissement_id)
        .bind(prelevement_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_sqlx)?
        .ok_or(DomainError::NotFound)?;
        let current = row.into_prelevement().ok_or(DomainError::SepaPrelevementInvalide)?;
        match current.statut {
            PrelevementStatut::Prevu | PrelevementStatut::Exporte => {}
            PrelevementStatut::Execute => {
                tx.commit().await.map_err(map_sqlx)?;
                return Ok((current, None));
            }
            _ => return Err(DomainError::SepaPrelevementInvalide),
        }
        let facture = sqlx::query_as::<_, FactureRow>(
            r#"
            SELECT id, etablissement_id, foyer_id, numero, libelle, statut,
                   total_centimes, nature, annee_scolaire,
                   created_by_user_id, created_at, emise_at, soldee_at
            FROM ent_fac_facture
            WHERE etablissement_id = $1 AND id = $2
            FOR UPDATE
            "#,
        )
        .bind(etablissement_id)
        .bind(current.facture_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_sqlx)?
        .map(FactureRow::into_facture)
        .ok_or(DomainError::NotFound)?;
        let (deja,): (i64,) = sqlx::query_as(
            r#"
            SELECT COALESCE(SUM(montant_centimes), 0)::bigint
            FROM ent_fac_encaissement
            WHERE etablissement_id = $1 AND facture_id = $2
            "#,
        )
        .bind(etablissement_id)
        .bind(current.facture_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        let (ouvert,): (i64,) = sqlx::query_as(
            r#"
            SELECT COALESCE(SUM(montant_centimes), 0)::bigint
            FROM ent_fac_sepa_prelevement
            WHERE etablissement_id = $1 AND facture_id = $2
              AND statut IN ('prevu', 'exporte')
            "#,
        )
        .bind(etablissement_id)
        .bind(current.facture_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        let (avoirs,): (i64,) = sqlx::query_as(
            r#"
            SELECT COALESCE(SUM(montant_centimes), 0)::bigint
            FROM ent_fin_lettrage
            WHERE etablissement_id = $1 AND facture_id = $2 AND piece = 'avoir'
            "#,
        )
        .bind(etablissement_id)
        .bind(current.facture_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        // F0 (FD-06) : le solde se décide sur l'encaissé réel (avoirs compris) ;
        // ce prélèvement quitte la réserve ouverte en devenant encaissement.
        let regle = deja.saturating_add(avoirs);
        let engage = regle.saturating_add(ouvert.saturating_sub(current.montant_centimes));
        let (enc, facture_maj) = domain_eleve::plan_encaisser(
            &facture,
            regle,
            engage,
            domain_eleve::EncaisserFacture {
                etablissement_id,
                facture_id: current.facture_id,
                montant_centimes: current.montant_centimes,
                moyen: MoyenPaiement::Prelevement,
                reference: Some(current.end_to_end_id.clone()),
                actor_user_id,
            },
        )?;
        sqlx::query(
            r#"
            INSERT INTO ent_fac_encaissement (
                id, etablissement_id, facture_id, montant_centimes, moyen,
                reference, encaisse_at, created_by_user_id, created_at
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)
            "#,
        )
        .bind(enc.id)
        .bind(enc.etablissement_id)
        .bind(enc.facture_id)
        .bind(enc.montant_centimes)
        .bind(enc.moyen.as_str())
        .bind(&enc.reference)
        .bind(enc.encaisse_at)
        .bind(enc.created_by_user_id)
        .bind(enc.created_at)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        if facture_maj.statut == FactureStatut::Soldee {
            let res = sqlx::query(
                r#"
                UPDATE ent_fac_facture
                SET statut = $3, soldee_at = $4
                WHERE etablissement_id = $1 AND id = $2 AND statut = 'emise'
                "#,
            )
            .bind(facture_maj.etablissement_id)
            .bind(facture_maj.id)
            .bind(facture_maj.statut.as_str())
            .bind(facture_maj.soldee_at)
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx)?;
            if res.rows_affected() == 0 {
                return Err(DomainError::FactureDejaSoldee);
            }
        }
        let (nb_q,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*)::bigint FROM ent_fac_quittance WHERE etablissement_id = $1 AND facture_id = $2",
        )
        .bind(etablissement_id)
        .bind(current.facture_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        let quittance =
            domain_eleve::plan_emettre_quittance(&facture_maj, &enc, (nb_q + 1) as i32)?;
        sqlx::query(
            r#"
            INSERT INTO ent_fac_quittance (
                id, etablissement_id, foyer_id, facture_id, encaissement_id,
                numero, montant_centimes, moyen, emise_at, created_by_user_id
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)
            "#,
        )
        .bind(quittance.id)
        .bind(quittance.etablissement_id)
        .bind(quittance.foyer_id)
        .bind(quittance.facture_id)
        .bind(quittance.encaissement_id)
        .bind(&quittance.numero)
        .bind(quittance.montant_centimes)
        .bind(quittance.moyen.as_str())
        .bind(quittance.emise_at)
        .bind(quittance.created_by_user_id)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        let executed = domain_eleve::plan_executer_prelevement(&current, enc.id)?;
        let n = sqlx::query(
            r#"
            UPDATE ent_fac_sepa_prelevement
            SET statut = $3, encaissement_id = $4
            WHERE etablissement_id = $1 AND id = $2 AND statut IN ('prevu', 'exporte')
            "#,
        )
        .bind(etablissement_id)
        .bind(executed.id)
        .bind(executed.statut.as_str())
        .bind(executed.encaissement_id)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx)?
        .rows_affected();
        if n == 0 {
            return Err(DomainError::SepaPrelevementInvalide);
        }
        tx.commit().await.map_err(map_sqlx)?;
        Ok((executed, Some((enc, facture_maj, quittance))))
    }

    /// F0 (FD-10) : à la révocation d'un mandat, les prélèvements encore « prévus »
    /// (jamais exportés) sont annulés. Renvoie le nombre de lignes annulées ; les
    /// prélèvements déjà exportés restent visibles pour traitement humain.
    pub async fn annuler_prelevements_prevus_mandat(
        &self,
        etablissement_id: Uuid,
        mandat_id: Uuid,
    ) -> Result<(u64, u64), DomainError> {
        let mut tx = TenantTx::begin(&self.pool, etablissement_id).await.map_err(map_sqlx)?;
        let annules = sqlx::query(
            "UPDATE ent_fac_sepa_prelevement SET statut = 'annule'
              WHERE etablissement_id = $1 AND mandat_id = $2 AND statut = 'prevu'",
        )
        .bind(etablissement_id)
        .bind(mandat_id)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx)?
        .rows_affected();
        let (deja_exportes,): (i64,) = sqlx::query_as(
            "SELECT count(*)::bigint FROM ent_fac_sepa_prelevement
              WHERE etablissement_id = $1 AND mandat_id = $2 AND statut = 'exporte'",
        )
        .bind(etablissement_id)
        .bind(mandat_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        tx.commit().await.map_err(map_sqlx)?;
        Ok((annules, deja_exportes as u64))
    }

    /// Avoirs imputés sur une facture (lettrage `avoir`), à ajouter à l'encaissé réel.
    pub async fn sum_avoirs(
        &self,
        etablissement_id: Uuid,
        facture_id: Uuid,
    ) -> Result<i64, DomainError> {
        let mut tx = TenantTx::begin(&self.pool, etablissement_id).await.map_err(map_sqlx)?;
        let (n,): (i64,) = sqlx::query_as(
            "SELECT COALESCE(SUM(montant_centimes), 0)::bigint FROM ent_fin_lettrage
              WHERE etablissement_id = $1 AND facture_id = $2 AND piece = 'avoir'",
        )
        .bind(etablissement_id)
        .bind(facture_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        tx.commit().await.map_err(map_sqlx)?;
        Ok(n)
    }

    /// Annule un prélèvement « prévu » (échec de pré-notification : il ne doit pas rester).
    pub async fn annuler_prelevement_prevu(
        &self,
        etablissement_id: Uuid,
        prelevement_id: Uuid,
    ) -> Result<(), DomainError> {
        let mut tx = TenantTx::begin(&self.pool, etablissement_id).await.map_err(map_sqlx)?;
        sqlx::query(
            "UPDATE ent_fac_sepa_prelevement SET statut = 'annule'
              WHERE etablissement_id = $1 AND id = $2 AND statut = 'prevu'",
        )
        .bind(etablissement_id)
        .bind(prelevement_id)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        tx.commit().await.map_err(map_sqlx)?;
        Ok(())
    }

    /// F0 (FD-23) : mandats des foyers donnés, filtrés **en SQL** (un parent voit
    /// toujours ses propres mandats, quelle que soit la taille de l'établissement).
    pub async fn list_sepa_mandats_foyers(
        &self,
        etablissement_id: Uuid,
        foyers: &[Uuid],
        limit: i64,
    ) -> Result<Vec<MandatSepa>, DomainError> {
        let mut tx = TenantTx::begin(&self.pool, etablissement_id).await.map_err(map_sqlx)?;
        let rows = sqlx::query_as::<_, MandatRow>(
            r#"
            SELECT id, etablissement_id, foyer_id, rum, iban, bic, titulaire,
                   statut, signe_at, created_by_user_id, created_at
            FROM ent_fac_sepa_mandat
            WHERE etablissement_id = $1 AND foyer_id = ANY($2)
            ORDER BY created_at DESC
            LIMIT $3
            "#,
        )
        .bind(etablissement_id)
        .bind(foyers)
        .bind(limit)
        .fetch_all(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        tx.commit().await.map_err(map_sqlx)?;
        rows.into_iter()
            .filter_map(|r| r.into_mandat(&self.champs).transpose())
            .collect()
    }

    /// F0 (FD-04/05/06/07) — **encaissement unique, atomique et verrouillé**.
    ///
    /// Dans **une seule transaction** : verrou du foyer (même clé que le chemin
    /// `/finances`) puis de la facture (`FOR UPDATE`), recalcul de l'encaissé réel
    /// (encaissements + avoirs) et des prélèvements ouverts, encaissement,
    /// imputation (lettrage), solde éventuel et quittance (rang par compteur sous
    /// verrou). Deux guichets simultanés ne peuvent plus dépasser le total ; un
    /// prélèvement ouvert ne solde plus la facture.
    pub async fn encaisser_atomique(
        &self,
        cmd: domain_eleve::EncaisserFacture,
    ) -> Result<(Encaissement, Facture, Quittance, i64), DomainError> {
        let etab = cmd.etablissement_id;
        let mut tx = TenantTx::begin(&self.pool, etab).await.map_err(map_sqlx)?;
        let (foyer,): (Uuid,) = sqlx::query_as(
            "SELECT foyer_id FROM ent_fac_facture WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(cmd.facture_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_sqlx)?
        .ok_or(DomainError::NotFound)?;
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1, 0))")
            .bind(format!("fin:{etab}:{foyer}"))
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx)?;
        let facture = sqlx::query_as::<_, FactureRow>(
            r#"
            SELECT id, etablissement_id, foyer_id, numero, libelle, statut,
                   total_centimes, nature, annee_scolaire,
                   created_by_user_id, created_at, emise_at, soldee_at
            FROM ent_fac_facture
            WHERE etablissement_id = $1 AND id = $2
            FOR UPDATE
            "#,
        )
        .bind(etab)
        .bind(cmd.facture_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_sqlx)?
        .map(FactureRow::into_facture)
        .ok_or(DomainError::NotFound)?;
        let (deja,): (i64,) = sqlx::query_as(
            "SELECT COALESCE(SUM(montant_centimes), 0)::bigint FROM ent_fac_encaissement
              WHERE etablissement_id = $1 AND facture_id = $2",
        )
        .bind(etab)
        .bind(cmd.facture_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        let (avoirs,): (i64,) = sqlx::query_as(
            "SELECT COALESCE(SUM(montant_centimes), 0)::bigint FROM ent_fin_lettrage
              WHERE etablissement_id = $1 AND facture_id = $2 AND piece = 'avoir'",
        )
        .bind(etab)
        .bind(cmd.facture_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        let (ouvert,): (i64,) = sqlx::query_as(
            "SELECT COALESCE(SUM(montant_centimes), 0)::bigint FROM ent_fac_sepa_prelevement
              WHERE etablissement_id = $1 AND facture_id = $2 AND statut IN ('prevu', 'exporte')",
        )
        .bind(etab)
        .bind(cmd.facture_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        let regle = deja.saturating_add(avoirs);
        let (enc, facture_maj) = domain_eleve::plan_encaisser(
            &facture,
            regle,
            regle.saturating_add(ouvert),
            cmd,
        )?;
        sqlx::query(
            r#"
            INSERT INTO ent_fac_encaissement (
                id, etablissement_id, facture_id, montant_centimes, moyen,
                reference, encaisse_at, created_by_user_id, created_at
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)
            "#,
        )
        .bind(enc.id)
        .bind(enc.etablissement_id)
        .bind(enc.facture_id)
        .bind(enc.montant_centimes)
        .bind(enc.moyen.as_str())
        .bind(&enc.reference)
        .bind(enc.encaisse_at)
        .bind(enc.created_by_user_id)
        .bind(enc.created_at)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        sqlx::query(
            "INSERT INTO ent_fin_lettrage (id, etablissement_id, facture_id, piece, piece_id, montant_centimes)
             VALUES ($1, $2, $3, 'encaissement', $4, $5)",
        )
        .bind(Uuid::new_v4())
        .bind(etab)
        .bind(enc.facture_id)
        .bind(enc.id)
        .bind(enc.montant_centimes)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        if facture_maj.statut == FactureStatut::Soldee {
            let res = sqlx::query(
                "UPDATE ent_fac_facture SET statut = 'soldee', soldee_at = $3
                  WHERE etablissement_id = $1 AND id = $2 AND statut = 'emise'",
            )
            .bind(etab)
            .bind(facture_maj.id)
            .bind(facture_maj.soldee_at)
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx)?;
            if res.rows_affected() == 0 {
                return Err(DomainError::FactureDejaSoldee);
            }
            let (n,): (i64,) = sqlx::query_as(
                "INSERT INTO ent_fin_compteur (etablissement_id, cle, dernier) VALUES ($1, 'lettrage', 1)
                 ON CONFLICT (etablissement_id, cle) DO UPDATE SET dernier = ent_fin_compteur.dernier + 1
                 RETURNING dernier",
            )
            .bind(etab)
            .fetch_one(&mut *tx)
            .await
            .map_err(map_sqlx)?;
            sqlx::query(
                "UPDATE ent_fin_lettrage SET code = $3
                  WHERE etablissement_id = $1 AND facture_id = $2 AND code IS NULL",
            )
            .bind(etab)
            .bind(facture_maj.id)
            .bind(format!("L{n:06}"))
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx)?;
        }
        // Rang de quittance sous verrou de la facture : jamais deux fois le même.
        let (nb_q,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*)::bigint FROM ent_fac_quittance WHERE etablissement_id = $1 AND facture_id = $2",
        )
        .bind(etab)
        .bind(facture_maj.id)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        let quittance = domain_eleve::plan_emettre_quittance(&facture_maj, &enc, (nb_q + 1) as i32)?;
        sqlx::query(
            r#"
            INSERT INTO ent_fac_quittance (
                id, etablissement_id, foyer_id, facture_id, encaissement_id,
                numero, montant_centimes, moyen, emise_at, created_by_user_id
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)
            "#,
        )
        .bind(quittance.id)
        .bind(quittance.etablissement_id)
        .bind(quittance.foyer_id)
        .bind(quittance.facture_id)
        .bind(quittance.encaissement_id)
        .bind(&quittance.numero)
        .bind(quittance.montant_centimes)
        .bind(quittance.moyen.as_str())
        .bind(quittance.emise_at)
        .bind(quittance.created_by_user_id)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        tx.commit().await.map_err(map_sqlx)?;
        Ok((enc, facture_maj, quittance, regle))
    }

    pub async fn list_factures_emises(
        &self,
        etablissement_id: Uuid,
        limit: i64,
    ) -> Result<Vec<Facture>, DomainError> {
        let mut tx = TenantTx::begin(&self.pool, etablissement_id).await.map_err(map_sqlx)?;
        let rows = sqlx::query_as::<_, FactureRow>(
            r#"
            SELECT id, etablissement_id, foyer_id, numero, libelle, statut,
                   total_centimes, nature, annee_scolaire,
                   created_by_user_id, created_at, emise_at, soldee_at
            FROM ent_fac_facture
            WHERE etablissement_id = $1 AND statut = 'emise'
              -- F0 (FD-04) : les rappels historiques ne portent que sur les pièces du chemin
              -- historique (origine NULL) ; le chemin par échéance a ses propres relances.
              AND origine IS NULL AND type_piece = 'facture'
            ORDER BY emise_at DESC NULLS LAST
            LIMIT $2
            "#,
        )
        .bind(etablissement_id)
        .bind(limit)
        .fetch_all(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        tx.commit().await.map_err(map_sqlx)?;
        Ok(rows.into_iter().map(FactureRow::into_facture).collect())
    }

    pub async fn first_eleve_id_foyer(
        &self,
        etablissement_id: Uuid,
        foyer_id: Uuid,
    ) -> Result<Option<Uuid>, DomainError> {
        let mut tx = TenantTx::begin(&self.pool, etablissement_id).await.map_err(map_sqlx)?;
        let row: Option<(Uuid,)> = sqlx::query_as(
            r#"
            SELECT eleve_id FROM ent_eleve_lien
            WHERE etablissement_id = $1 AND foyer_id = $2
            ORDER BY eleve_id ASC
            LIMIT 1
            "#,
        )
        .bind(etablissement_id)
        .bind(foyer_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        tx.commit().await.map_err(map_sqlx)?;
        Ok(row.map(|r| r.0))
    }

    /// `true` si insert effectué, `false` si déjà rappelé (conflit unique).
    pub async fn insert_rappel(&self, r: &RappelFacture) -> Result<bool, DomainError> {
        let mut tx = TenantTx::begin(&self.pool, r.etablissement_id).await.map_err(map_sqlx)?;
        let res = sqlx::query(
            r#"
            INSERT INTO ent_fac_rappel (
                id, etablissement_id, foyer_id, facture_id, echeance_id,
                kind, canal, corps, fil_id, envoye_at, created_by_user_id, created_at
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12)
            ON CONFLICT (etablissement_id, echeance_id, kind) DO NOTHING
            "#,
        )
        .bind(r.id)
        .bind(r.etablissement_id)
        .bind(r.foyer_id)
        .bind(r.facture_id)
        .bind(r.echeance_id)
        .bind(r.kind.as_str())
        .bind(r.canal.as_str())
        .bind(&r.corps)
        .bind(r.fil_id)
        .bind(r.envoye_at)
        .bind(r.created_by_user_id)
        .bind(r.created_at)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        tx.commit().await.map_err(map_sqlx)?;
        Ok(res.rows_affected() > 0)
    }

    pub async fn update_rappel_fil(
        &self,
        etablissement_id: Uuid,
        rappel_id: Uuid,
        fil_id: Uuid,
        canal: RappelCanal,
    ) -> Result<(), DomainError> {
        let mut tx = TenantTx::begin(&self.pool, etablissement_id).await.map_err(map_sqlx)?;
        let n = sqlx::query(
            r#"
            UPDATE ent_fac_rappel
            SET fil_id = $3, canal = $4
            WHERE etablissement_id = $1 AND id = $2
            "#,
        )
        .bind(etablissement_id)
        .bind(rappel_id)
        .bind(fil_id)
        .bind(canal.as_str())
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx)?
        .rows_affected();
        if n == 0 {
            return Err(DomainError::NotFound);
        }
        tx.commit().await.map_err(map_sqlx)?;
        Ok(())
    }

    pub async fn list_rappels_etab(
        &self,
        etablissement_id: Uuid,
        limit: i64,
    ) -> Result<Vec<RappelFacture>, DomainError> {
        let mut tx = TenantTx::begin(&self.pool, etablissement_id).await.map_err(map_sqlx)?;
        let rows = sqlx::query_as::<_, RappelRow>(
            r#"
            SELECT id, etablissement_id, foyer_id, facture_id, echeance_id,
                   kind, canal, corps, fil_id, envoye_at, created_by_user_id, created_at
            FROM ent_fac_rappel
            WHERE etablissement_id = $1
            ORDER BY envoye_at DESC
            LIMIT $2
            "#,
        )
        .bind(etablissement_id)
        .bind(limit)
        .fetch_all(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        tx.commit().await.map_err(map_sqlx)?;
        Ok(rows.into_iter().filter_map(RappelRow::into_rappel).collect())
    }
}

#[derive(Debug, sqlx::FromRow)]
struct RappelRow {
    id: Uuid,
    etablissement_id: Uuid,
    foyer_id: Uuid,
    facture_id: Uuid,
    echeance_id: Uuid,
    kind: String,
    canal: String,
    corps: String,
    fil_id: Option<Uuid>,
    envoye_at: DateTime<Utc>,
    created_by_user_id: Option<Uuid>,
    created_at: DateTime<Utc>,
}

impl RappelRow {
    fn into_rappel(self) -> Option<RappelFacture> {
        Some(RappelFacture {
            id: self.id,
            etablissement_id: self.etablissement_id,
            foyer_id: self.foyer_id,
            facture_id: self.facture_id,
            echeance_id: self.echeance_id,
            kind: RappelKind::parse(&self.kind)?,
            canal: RappelCanal::parse(&self.canal)?,
            corps: self.corps,
            fil_id: self.fil_id,
            envoye_at: self.envoye_at,
            created_by_user_id: self.created_by_user_id,
            created_at: self.created_at,
        })
    }
}

#[derive(Debug, sqlx::FromRow)]
struct MandatRow {
    id: Uuid,
    etablissement_id: Uuid,
    foyer_id: Uuid,
    rum: String,
    iban: String,
    bic: Option<String>,
    titulaire: String,
    statut: String,
    signe_at: DateTime<Utc>,
    created_by_user_id: Option<Uuid>,
    created_at: DateTime<Utc>,
}

impl MandatRow {
    fn into_mandat(self, champs: &ChampsSensibles) -> Result<Option<MandatSepa>, DomainError> {
        let iban = champs.ouvrir(T_MANDAT, "iban", self.id, &self.iban)?;
        let bic = champs.ouvrir_opt(T_MANDAT, "bic", self.id, self.bic)?;
        let titulaire = champs.ouvrir(T_MANDAT, "titulaire", self.id, &self.titulaire)?;
        let Some(statut) = MandatSepaStatut::parse(&self.statut) else {
            return Ok(None);
        };
        Ok(Some(MandatSepa {
            id: self.id,
            etablissement_id: self.etablissement_id,
            foyer_id: self.foyer_id,
            rum: self.rum,
            iban,
            bic,
            titulaire,
            statut,
            signe_at: self.signe_at,
            created_by_user_id: self.created_by_user_id,
            created_at: self.created_at,
        }))
    }
}

#[derive(Debug, sqlx::FromRow)]
struct PrelevementRow {
    id: Uuid,
    etablissement_id: Uuid,
    mandat_id: Uuid,
    facture_id: Uuid,
    foyer_id: Uuid,
    montant_centimes: i64,
    date_execution: NaiveDate,
    end_to_end_id: String,
    statut: String,
    encaissement_id: Option<Uuid>,
    motif_rejet: Option<String>,
    rejete_at: Option<DateTime<Utc>>,
    created_by_user_id: Option<Uuid>,
    created_at: DateTime<Utc>,
}

impl PrelevementRow {
    fn into_prelevement(self) -> Option<PrelevementSepa> {
        Some(PrelevementSepa {
            id: self.id,
            etablissement_id: self.etablissement_id,
            mandat_id: self.mandat_id,
            facture_id: self.facture_id,
            foyer_id: self.foyer_id,
            montant_centimes: self.montant_centimes,
            date_execution: self.date_execution,
            end_to_end_id: self.end_to_end_id,
            statut: PrelevementStatut::parse(&self.statut)?,
            encaissement_id: self.encaissement_id,
            motif_rejet: self.motif_rejet,
            rejete_at: self.rejete_at,
            created_by_user_id: self.created_by_user_id,
            created_at: self.created_at,
        })
    }
}

#[derive(Debug, sqlx::FromRow)]
struct SepaExportRow {
    id: Uuid,
    etablissement_id: Uuid,
    mandat_id: Uuid,
    facture_id: Uuid,
    foyer_id: Uuid,
    montant_centimes: i64,
    date_execution: NaiveDate,
    end_to_end_id: String,
    statut: String,
    encaissement_id: Option<Uuid>,
    motif_rejet: Option<String>,
    rejete_at: Option<DateTime<Utc>>,
    created_by_user_id: Option<Uuid>,
    created_at: DateTime<Utc>,
    rum: String,
    iban: String,
    bic: Option<String>,
    titulaire: String,
    mandat_signe_at: DateTime<Utc>,
    facture_numero: String,
}

impl SepaExportRow {
    fn into_ligne(self, champs: &ChampsSensibles) -> Result<Option<SepaLigneExport>, DomainError> {
        let iban = champs.ouvrir(T_MANDAT, "iban", self.mandat_id, &self.iban)?;
        let bic = champs.ouvrir_opt(T_MANDAT, "bic", self.mandat_id, self.bic.clone())?;
        let titulaire = champs.ouvrir(T_MANDAT, "titulaire", self.mandat_id, &self.titulaire)?;
        let Some(statut) = PrelevementStatut::parse(&self.statut) else {
            return Ok(None);
        };
        let prelevement = PrelevementSepa {
            id: self.id,
            etablissement_id: self.etablissement_id,
            mandat_id: self.mandat_id,
            facture_id: self.facture_id,
            foyer_id: self.foyer_id,
            montant_centimes: self.montant_centimes,
            date_execution: self.date_execution,
            end_to_end_id: self.end_to_end_id,
            statut,
            encaissement_id: self.encaissement_id,
            motif_rejet: self.motif_rejet,
            rejete_at: self.rejete_at,
            created_by_user_id: self.created_by_user_id,
            created_at: self.created_at,
        };
        Ok(Some(SepaLigneExport {
            prelevement,
            rum: self.rum,
            iban,
            bic,
            titulaire,
            mandat_signe_at: self.mandat_signe_at,
            facture_numero: self.facture_numero,
        }))
    }
}

#[derive(Debug, sqlx::FromRow)]
struct OgecRow {
    encaisse_at: DateTime<Utc>,
    facture_numero: String,
    facture_libelle: String,
    foyer_id: Uuid,
    quittance_numero: Option<String>,
    moyen: String,
    reference: Option<String>,
    montant_centimes: i64,
    nature_facture: Option<String>,
}

impl OgecRow {
    fn into_ligne(self) -> Option<OgecLigneExport> {
        Some(OgecLigneExport {
            encaisse_at: self.encaisse_at,
            facture_numero: self.facture_numero,
            facture_libelle: self.facture_libelle,
            foyer_id: self.foyer_id,
            quittance_numero: self.quittance_numero,
            moyen: MoyenPaiement::parse(&self.moyen)?,
            reference: self.reference,
            montant_centimes: self.montant_centimes,
            nature_facture: self.nature_facture,
        })
    }
}

#[derive(Debug, sqlx::FromRow)]
struct FactureRow {
    id: Uuid,
    etablissement_id: Uuid,
    foyer_id: Uuid,
    numero: String,
    libelle: String,
    statut: String,
    total_centimes: i64,
    nature: Option<String>,
    annee_scolaire: Option<String>,
    created_by_user_id: Option<Uuid>,
    created_at: DateTime<Utc>,
    emise_at: Option<DateTime<Utc>>,
    soldee_at: Option<DateTime<Utc>>,
}

impl FactureRow {
    fn into_facture(self) -> Facture {
        Facture {
            id: self.id,
            etablissement_id: self.etablissement_id,
            foyer_id: self.foyer_id,
            numero: self.numero,
            libelle: self.libelle,
            statut: FactureStatut::parse(&self.statut).unwrap_or(FactureStatut::Brouillon),
            total_centimes: self.total_centimes,
            nature: self.nature,
            annee_scolaire: self.annee_scolaire,
            created_by_user_id: self.created_by_user_id,
            created_at: self.created_at,
            emise_at: self.emise_at,
            soldee_at: self.soldee_at,
        }
    }
}

#[derive(Debug, sqlx::FromRow)]
struct LigneRow {
    id: Uuid,
    etablissement_id: Uuid,
    facture_id: Uuid,
    eleve_id: Option<Uuid>,
    libelle: String,
    quantite: i32,
    montant_unitaire_centimes: i64,
}

impl LigneRow {
    fn into_ligne(self) -> FactureLigne {
        FactureLigne {
            id: self.id,
            etablissement_id: self.etablissement_id,
            facture_id: self.facture_id,
            eleve_id: self.eleve_id,
            libelle: self.libelle,
            quantite: self.quantite,
            montant_unitaire_centimes: self.montant_unitaire_centimes,
        }
    }
}

#[derive(Debug, sqlx::FromRow)]
struct EncRow {
    id: Uuid,
    etablissement_id: Uuid,
    facture_id: Uuid,
    montant_centimes: i64,
    moyen: String,
    reference: Option<String>,
    encaisse_at: DateTime<Utc>,
    created_by_user_id: Option<Uuid>,
    created_at: DateTime<Utc>,
}

impl EncRow {
    fn into_enc(self) -> Encaissement {
        Encaissement {
            id: self.id,
            etablissement_id: self.etablissement_id,
            facture_id: self.facture_id,
            montant_centimes: self.montant_centimes,
            moyen: MoyenPaiement::parse(&self.moyen).unwrap_or(MoyenPaiement::Autre),
            reference: self.reference,
            encaisse_at: self.encaisse_at,
            created_by_user_id: self.created_by_user_id,
            created_at: self.created_at,
        }
    }
}

#[derive(Debug, sqlx::FromRow)]
struct EchRow {
    id: Uuid,
    etablissement_id: Uuid,
    facture_id: Uuid,
    rang: i32,
    libelle: String,
    montant_centimes: i64,
    bourse_centimes: i64,
    date_echeance: NaiveDate,
    created_at: DateTime<Utc>,
}

impl EchRow {
    fn into_ech(self) -> Echeance {
        Echeance {
            id: self.id,
            etablissement_id: self.etablissement_id,
            facture_id: self.facture_id,
            rang: self.rang,
            libelle: self.libelle,
            montant_centimes: self.montant_centimes,
            bourse_centimes: self.bourse_centimes,
            date_echeance: self.date_echeance,
            created_at: self.created_at,
        }
    }
}

#[derive(Debug, sqlx::FromRow)]
struct QuitRow {
    id: Uuid,
    etablissement_id: Uuid,
    foyer_id: Uuid,
    facture_id: Uuid,
    encaissement_id: Uuid,
    numero: String,
    montant_centimes: i64,
    moyen: String,
    emise_at: DateTime<Utc>,
    created_by_user_id: Option<Uuid>,
}

impl QuitRow {
    fn into_quit(self) -> Quittance {
        Quittance {
            id: self.id,
            etablissement_id: self.etablissement_id,
            foyer_id: self.foyer_id,
            facture_id: self.facture_id,
            encaissement_id: self.encaissement_id,
            numero: self.numero,
            montant_centimes: self.montant_centimes,
            moyen: MoyenPaiement::parse(&self.moyen).unwrap_or(MoyenPaiement::Autre),
            emise_at: self.emise_at,
            created_by_user_id: self.created_by_user_id,
        }
    }
}

fn map_sqlx(err: sqlx::Error) -> DomainError {
    crate::pg_err::map_sqlx("fac_repo", err)
}
