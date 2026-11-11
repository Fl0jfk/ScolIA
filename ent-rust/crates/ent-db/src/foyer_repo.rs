use domain_eleve::{assert_parts_paiement, DomainError, LienDossier};
use sqlx::PgPool;
use uuid::Uuid;
use crate::tenant_tx::TenantTx;

#[derive(Debug, Clone)]
pub struct EleveLien {
    pub id: Uuid,
    pub etablissement_id: Uuid,
    pub eleve_id: Uuid,
    pub user_id: Uuid,
    pub foyer_id: Option<Uuid>,
    pub lien: String,
    pub peut_voir_notes: bool,
    pub peut_voir_vs: bool,
    pub peut_justifier_absence: bool,
    pub peut_autoriser_sorties: bool,
    pub est_payeur: bool,
    pub part_paiement: Option<i16>,
}

#[derive(Debug, Clone)]
pub struct CreateLien {
    pub etablissement_id: Uuid,
    pub eleve_id: Uuid,
    pub user_id: Uuid,
    pub foyer_id: Option<Uuid>,
    pub lien: String,
    pub peut_voir_notes: bool,
    pub peut_voir_vs: bool,
    pub peut_justifier_absence: bool,
    pub peut_autoriser_sorties: bool,
    pub est_payeur: bool,
    pub part_paiement: Option<i16>,
}

#[derive(Debug, Clone)]
pub struct UpdateLien {
    pub lien: String,
    pub peut_voir_notes: bool,
    pub peut_voir_vs: bool,
    pub peut_justifier_absence: bool,
    pub peut_autoriser_sorties: bool,
    pub est_payeur: bool,
    pub part_paiement: Option<i16>,
}

/// Lien enrichi pour la fiche foyer (enfant + responsable).
#[derive(Debug, Clone)]
pub struct LienDossierFoyer {
    pub lien_id: Uuid,
    pub eleve_id: Uuid,
    pub eleve_nom: String,
    pub eleve_prenom: String,
    pub eleve_classe: Option<String>,
    pub user_id: Uuid,
    pub display_name: Option<String>,
    pub user_email: Option<String>,
    pub telephone: Option<String>,
    pub lien: String,
    pub peut_voir_notes: bool,
    pub peut_voir_vs: bool,
    pub peut_justifier_absence: bool,
    pub peut_autoriser_sorties: bool,
    pub est_payeur: bool,
    pub part_paiement: Option<i16>,
}

pub struct PgFoyerRepo {
    pool: PgPool,
}

impl PgFoyerRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create_foyer(
        &self,
        etablissement_id: Uuid,
        libelle: Option<&str>,
    ) -> Result<Uuid, DomainError> {
        let id = Uuid::new_v4();
        let mut tx = TenantTx::begin(&self.pool, etablissement_id).await.map_err(map_sqlx)?;
        sqlx::query(
            r#"
            INSERT INTO ent_foyer (id, etablissement_id, libelle, created_at)
            VALUES ($1, $2, $3, now())
            "#,
        )
        .bind(id)
        .bind(etablissement_id)
        .bind(libelle)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        tx.commit().await.map_err(map_sqlx)?;
        Ok(id)
    }

    pub async fn get_foyer(
        &self,
        etablissement_id: Uuid,
        foyer_id: Uuid,
    ) -> Result<Option<(Uuid, Option<String>)>, DomainError> {
        let row: Option<(Uuid, Option<String>)> = sqlx::query_as(
            r#"
            SELECT id, libelle
            FROM ent_foyer
            WHERE etablissement_id = $1 AND id = $2
            "#,
        )
        .bind(etablissement_id)
        .bind(foyer_id)
        .fetch_optional(&mut *TenantTx::begin(&self.pool, etablissement_id).await.map_err(map_sqlx)?)
        .await
        .map_err(map_sqlx)?;
        Ok(row)
    }

    /// Tous les liens rattachés à un foyer (tous les enfants du foyer).
    pub async fn list_liens_foyer(
        &self,
        etablissement_id: Uuid,
        foyer_id: Uuid,
    ) -> Result<Vec<LienDossierFoyer>, DomainError> {
        let rows = sqlx::query_as::<_, LienFoyerRow>(
            r#"
            SELECT l.id AS lien_id, l.eleve_id, e.nom AS eleve_nom, e.prenom AS eleve_prenom,
                   e.classe AS eleve_classe, l.user_id, u.display_name, u.email AS user_email,
                   u.telephone, l.lien, l.peut_voir_notes, l.peut_voir_vs, l.peut_justifier_absence,
                   l.peut_autoriser_sorties, l.est_payeur, l.part_paiement
            FROM ent_eleve_lien l
            JOIN ent_eleve e ON e.id = l.eleve_id AND e.etablissement_id = l.etablissement_id
            LEFT JOIN ent_user u ON u.id = l.user_id
            WHERE l.etablissement_id = $1 AND l.foyer_id = $2 AND l.user_id IS NOT NULL
            ORDER BY e.nom, e.prenom, l.created_at
            "#,
        )
        .bind(etablissement_id)
        .bind(foyer_id)
        .fetch_all(&mut *TenantTx::begin(&self.pool, etablissement_id).await.map_err(map_sqlx)?)
        .await
        .map_err(map_sqlx)?;
        Ok(rows
            .into_iter()
            .map(|r| LienDossierFoyer {
                lien_id: r.lien_id,
                eleve_id: r.eleve_id,
                eleve_nom: r.eleve_nom,
                eleve_prenom: r.eleve_prenom,
                eleve_classe: r.eleve_classe,
                user_id: r.user_id,
                display_name: r.display_name,
                user_email: r.user_email,
                telephone: r.telephone,
                lien: r.lien,
                peut_voir_notes: r.peut_voir_notes,
                peut_voir_vs: r.peut_voir_vs,
                peut_justifier_absence: r.peut_justifier_absence,
                peut_autoriser_sorties: r.peut_autoriser_sorties,
                est_payeur: r.est_payeur,
                part_paiement: r.part_paiement,
            })
            .collect())
    }

    pub async fn list_foyers(
        &self,
        etablissement_id: Uuid,
        q: Option<&str>,
        limit: i64,
    ) -> Result<Vec<(Uuid, Option<String>, i64)>, DomainError> {
        let pattern = q
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .map(|s| format!("%{s}%"));
        let mut tx = TenantTx::begin(&self.pool, etablissement_id).await.map_err(map_sqlx)?;
        let rows: Vec<(Uuid, Option<String>, i64)> = if let Some(p) = pattern {
            sqlx::query_as(
                r#"
                SELECT f.id, f.libelle, COUNT(DISTINCT l.eleve_id)::bigint AS nb_eleves
                FROM ent_foyer f
                LEFT JOIN ent_eleve_lien l
                  ON l.foyer_id = f.id AND l.etablissement_id = f.etablissement_id
                LEFT JOIN ent_user u ON u.id = l.user_id
                WHERE f.etablissement_id = $1
                  AND (
                    f.libelle ILIKE $2
                    OR u.display_name ILIKE $2
                    OR u.email ILIKE $2
                  )
                GROUP BY f.id, f.libelle
                ORDER BY f.libelle NULLS LAST, f.id
                LIMIT $3
                "#,
            )
            .bind(etablissement_id)
            .bind(p)
            .bind(limit)
            .fetch_all(&mut *tx)
            .await
            .map_err(map_sqlx)?
        } else {
            sqlx::query_as(
                r#"
                SELECT f.id, f.libelle, COUNT(DISTINCT l.eleve_id)::bigint AS nb_eleves
                FROM ent_foyer f
                LEFT JOIN ent_eleve_lien l
                  ON l.foyer_id = f.id AND l.etablissement_id = f.etablissement_id
                WHERE f.etablissement_id = $1
                GROUP BY f.id, f.libelle
                ORDER BY f.libelle NULLS LAST, f.id
                LIMIT $2
                "#,
            )
            .bind(etablissement_id)
            .bind(limit)
            .fetch_all(&mut *tx)
            .await
            .map_err(map_sqlx)?
        };
        Ok(rows)
    }

    pub async fn update_lien(
        &self,
        etablissement_id: Uuid,
        lien_id: Uuid,
        cmd: UpdateLien,
    ) -> Result<EleveLien, DomainError> {
        let current = self
            .get_lien(etablissement_id, lien_id)
            .await?
            .ok_or(DomainError::NotFound)?;
        // Parts : TOUS les liens de l'élève (avec ou sans compte, S2-1).
        let mut parts: Vec<(bool, Option<i16>)> = self
            .parts_eleve(etablissement_id, current.eleve_id)
            .await?
            .into_iter()
            .filter(|(id, _, _, _)| *id != lien_id)
            .map(|(_, _, p, part)| (p, part))
            .collect();
        parts.push((cmd.est_payeur, cmd.part_paiement));
        assert_parts_paiement(&parts)?;

        let lien = cmd.lien.trim();
        let lien = if lien.is_empty() {
            current.lien.as_str()
        } else {
            lien
        };
        let mut tx = TenantTx::begin(&self.pool, etablissement_id).await.map_err(map_sqlx)?;

        sqlx::query(
            r#"
            UPDATE ent_eleve_lien SET
                lien = $3,
                peut_voir_notes = $4,
                peut_voir_vs = $5,
                peut_justifier_absence = $6,
                peut_autoriser_sorties = $7,
                est_payeur = $8,
                part_paiement = $9
            WHERE etablissement_id = $1 AND id = $2
            "#,
        )
        .bind(etablissement_id)
        .bind(lien_id)
        .bind(lien)
        .bind(cmd.peut_voir_notes)
        .bind(cmd.peut_voir_vs)
        .bind(cmd.peut_justifier_absence)
        .bind(cmd.peut_autoriser_sorties)
        .bind(cmd.est_payeur)
        .bind(cmd.part_paiement)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        tx.commit().await.map_err(map_sqlx)?;

        self.get_lien(etablissement_id, lien_id)
            .await?
            .ok_or(DomainError::NotFound)
    }

    /// Mise à jour atomique de plusieurs liens d'un même élève (parts 100 % en une fois).
    pub async fn update_liens_eleve(
        &self,
        etablissement_id: Uuid,
        eleve_id: Uuid,
        cmds: Vec<(Uuid, UpdateLien)>,
    ) -> Result<Vec<EleveLien>, DomainError> {
        if cmds.is_empty() {
            return self.list_liens_eleve(etablissement_id, eleve_id).await;
        }
        let existing = self.list_liens_eleve(etablissement_id, eleve_id).await?;
        let by_id: std::collections::HashMap<Uuid, &EleveLien> =
            existing.iter().map(|l| (l.id, l)).collect();
        for (lien_id, _) in &cmds {
            let Some(l) = by_id.get(lien_id) else {
                return Err(DomainError::NotFound);
            };
            if l.eleve_id != eleve_id {
                return Err(DomainError::NotFound);
            }
        }
        let cmd_map: std::collections::HashMap<Uuid, &UpdateLien> =
            cmds.iter().map(|(id, c)| (*id, c)).collect();
        let parts: Vec<(bool, Option<i16>)> = self
            .parts_eleve(etablissement_id, eleve_id)
            .await?
            .into_iter()
            .map(|(id, _, p, part)| {
                if let Some(c) = cmd_map.get(&id) {
                    (c.est_payeur, c.part_paiement)
                } else {
                    (p, part)
                }
            })
            .collect();
        assert_parts_paiement(&parts)?;

        let mut tx = TenantTx::begin(&self.pool, etablissement_id).await.map_err(map_sqlx)?;
        for (lien_id, cmd) in &cmds {
            let current = by_id.get(lien_id).ok_or(DomainError::NotFound)?;
            let lien = cmd.lien.trim();
            let lien = if lien.is_empty() {
                current.lien.as_str()
            } else {
                lien
            };
            sqlx::query(
                r#"
                UPDATE ent_eleve_lien SET
                    lien = $3,
                    peut_voir_notes = $4,
                    peut_voir_vs = $5,
                    peut_justifier_absence = $6,
                    peut_autoriser_sorties = $7,
                    est_payeur = $8,
                    part_paiement = $9
                WHERE etablissement_id = $1 AND id = $2 AND eleve_id = $10
                "#,
            )
            .bind(etablissement_id)
            .bind(lien_id)
            .bind(lien)
            .bind(cmd.peut_voir_notes)
            .bind(cmd.peut_voir_vs)
            .bind(cmd.peut_justifier_absence)
            .bind(cmd.peut_autoriser_sorties)
            .bind(cmd.est_payeur)
            .bind(cmd.part_paiement)
            .bind(eleve_id)
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx)?;
        }
        tx.commit().await.map_err(map_sqlx)?;
        self.list_liens_eleve(etablissement_id, eleve_id).await
    }

    pub async fn delete_lien(
        &self,
        etablissement_id: Uuid,
        lien_id: Uuid,
    ) -> Result<(), DomainError> {
        let current = self
            .get_lien(etablissement_id, lien_id)
            .await?
            .ok_or(DomainError::NotFound)?;
        let parts: Vec<(bool, Option<i16>)> = self
            .parts_eleve(etablissement_id, current.eleve_id)
            .await?
            .into_iter()
            .filter(|(id, _, _, _)| *id != lien_id)
            .map(|(_, _, p, part)| (p, part))
            .collect();
        assert_parts_paiement(&parts)?;
        let mut tx = TenantTx::begin(&self.pool, etablissement_id).await.map_err(map_sqlx)?;
        let res = sqlx::query(
            r#"
            DELETE FROM ent_eleve_lien
            WHERE etablissement_id = $1 AND id = $2
            "#,
        )
        .bind(etablissement_id)
        .bind(lien_id)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        if res.rows_affected() == 0 {
            return Err(DomainError::NotFound);
        }
        tx.commit().await.map_err(map_sqlx)?;
        Ok(())
    }

    pub async fn get_lien(
        &self,
        etablissement_id: Uuid,
        lien_id: Uuid,
    ) -> Result<Option<EleveLien>, DomainError> {
        let row = sqlx::query_as::<_, LienRow>(
            r#"
            SELECT id, etablissement_id, eleve_id, user_id, foyer_id, lien,
                   peut_voir_notes, peut_voir_vs, peut_justifier_absence,
                   peut_autoriser_sorties, est_payeur, part_paiement
            FROM ent_eleve_lien
            WHERE etablissement_id = $1 AND id = $2 AND user_id IS NOT NULL
            "#,
        )
        .bind(etablissement_id)
        .bind(lien_id)
        .fetch_optional(&mut *TenantTx::begin(&self.pool, etablissement_id).await.map_err(map_sqlx)?)
        .await
        .map_err(map_sqlx)?;
        Ok(row.map(LienRow::into_lien))
    }

    pub async fn link_parent(&self, cmd: CreateLien) -> Result<EleveLien, DomainError> {
        let id = Uuid::new_v4();
        let lien = cmd.lien.trim();
        let lien = if lien.is_empty() { "autre" } else { lien };

        // Invariant parts 100 % après upsert de ce lien.
        let mut parts: Vec<(bool, Option<i16>)> = self
            .parts_eleve(cmd.etablissement_id, cmd.eleve_id)
            .await?
            .into_iter()
            .filter(|(_, user, _, _)| *user != Some(cmd.user_id))
            .map(|(_, _, p, part)| (p, part))
            .collect();
        parts.push((cmd.est_payeur, cmd.part_paiement));
        assert_parts_paiement(&parts)?;

        let mut tx = TenantTx::begin(&self.pool, cmd.etablissement_id).await.map_err(map_sqlx)?;
        sqlx::query(
            r#"
            INSERT INTO ent_eleve_lien (
                id, etablissement_id, eleve_id, user_id, foyer_id, lien,
                peut_voir_notes, peut_voir_vs, peut_justifier_absence,
                peut_autoriser_sorties, est_payeur, part_paiement, created_at
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12, now())
            ON CONFLICT (etablissement_id, eleve_id, user_id) DO UPDATE SET
                foyer_id = EXCLUDED.foyer_id,
                lien = EXCLUDED.lien,
                peut_voir_notes = EXCLUDED.peut_voir_notes,
                peut_voir_vs = EXCLUDED.peut_voir_vs,
                peut_justifier_absence = EXCLUDED.peut_justifier_absence,
                peut_autoriser_sorties = EXCLUDED.peut_autoriser_sorties,
                est_payeur = EXCLUDED.est_payeur,
                part_paiement = EXCLUDED.part_paiement
            "#,
        )
        .bind(id)
        .bind(cmd.etablissement_id)
        .bind(cmd.eleve_id)
        .bind(cmd.user_id)
        .bind(cmd.foyer_id)
        .bind(lien)
        .bind(cmd.peut_voir_notes)
        .bind(cmd.peut_voir_vs)
        .bind(cmd.peut_justifier_absence)
        .bind(cmd.peut_autoriser_sorties)
        .bind(cmd.est_payeur)
        .bind(cmd.part_paiement)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        tx.commit().await.map_err(map_sqlx)?;

        // Lecture brute (le lien peut être suspendu : la mise à jour reste visible).
        self.lien_compte_brut(cmd.etablissement_id, cmd.user_id, cmd.eleve_id)
            .await?
            .ok_or(DomainError::NotFound)
    }

    /// Parts de paiement de tous les liens de l'élève : (lien, compte, payeur, part).
    async fn parts_eleve(
        &self,
        etablissement_id: Uuid,
        eleve_id: Uuid,
    ) -> Result<Vec<(Uuid, Option<Uuid>, bool, Option<i16>)>, DomainError> {
        sqlx::query_as(
            r#"
            SELECT id, user_id, est_payeur, part_paiement
            FROM ent_eleve_lien
            WHERE etablissement_id = $1 AND eleve_id = $2
            "#,
        )
        .bind(etablissement_id)
        .bind(eleve_id)
        .fetch_all(&mut *TenantTx::begin(&self.pool, etablissement_id).await.map_err(map_sqlx)?)
        .await
        .map_err(map_sqlx)
    }

    async fn lien_compte_brut(
        &self,
        etablissement_id: Uuid,
        user_id: Uuid,
        eleve_id: Uuid,
    ) -> Result<Option<EleveLien>, DomainError> {
        let row = sqlx::query_as::<_, LienRow>(
            r#"
            SELECT id, etablissement_id, eleve_id, user_id, foyer_id, lien,
                   peut_voir_notes, peut_voir_vs, peut_justifier_absence,
                   peut_autoriser_sorties, est_payeur, part_paiement
            FROM ent_eleve_lien
            WHERE etablissement_id = $1 AND user_id = $2 AND eleve_id = $3
            "#,
        )
        .bind(etablissement_id)
        .bind(user_id)
        .bind(eleve_id)
        .fetch_optional(&mut *TenantTx::begin(&self.pool, etablissement_id).await.map_err(map_sqlx)?)
        .await
        .map_err(map_sqlx)?;
        Ok(row.map(LienRow::into_lien))
    }

    /// Liens ACTIFS d'un élève portant un compte (envois aux familles).
    /// Liens à qui une communication à propos de l'élève peut être adressée (M0a, D1 et D6) :
    /// jamais une personne visée par une interdiction de communication ; avec
    /// `sollicitables_seulement`, jamais non plus une personne « ne pas solliciter »
    /// (contact ouvert par l'établissement : push, annonce…).
    pub async fn list_liens_eleve_actifs(
        &self,
        etablissement_id: Uuid,
        eleve_id: Uuid,
        sollicitables_seulement: bool,
    ) -> Result<Vec<EleveLien>, DomainError> {
        let vue = if sollicitables_seulement {
            "ent_eleve_lien_sollicitable"
        } else {
            "ent_eleve_lien_communicant"
        };
        let rows = sqlx::query_as::<_, LienRow>(&format!(
            r#"
            SELECT id, etablissement_id, eleve_id, user_id, foyer_id, lien,
                   peut_voir_notes, peut_voir_vs, peut_justifier_absence,
                   peut_autoriser_sorties, est_payeur, part_paiement
            FROM {vue}
            WHERE etablissement_id = $1 AND eleve_id = $2 AND user_id IS NOT NULL
            ORDER BY created_at
            "#
        ))
        .bind(etablissement_id)
        .bind(eleve_id)
        .fetch_all(&mut *TenantTx::begin(&self.pool, etablissement_id).await.map_err(map_sqlx)?)
        .await
        .map_err(map_sqlx)?;
        Ok(rows.into_iter().map(LienRow::into_lien).collect())
    }

    /// Lien ACTIF du compte sur l'élève **et** non visé par une interdiction de communication
    /// (M0a, D1) : le contrôle de la messagerie et du carnet côté parent. Renvoie aussi
    /// `autorite_parentale`.
    pub async fn lien_communicant(
        &self,
        etablissement_id: Uuid,
        user_id: Uuid,
        eleve_id: Uuid,
    ) -> Result<Option<(EleveLien, bool)>, DomainError> {
        #[derive(sqlx::FromRow)]
        struct Row {
            #[sqlx(flatten)]
            lien: LienRow,
            autorite_parentale: bool,
        }
        let row = sqlx::query_as::<_, Row>(
            r#"
            SELECT id, etablissement_id, eleve_id, user_id, foyer_id, lien,
                   peut_voir_notes, peut_voir_vs, peut_justifier_absence,
                   peut_autoriser_sorties, est_payeur, part_paiement, autorite_parentale
            FROM ent_eleve_lien_communicant
            WHERE etablissement_id = $1 AND user_id = $2 AND eleve_id = $3
            "#,
        )
        .bind(etablissement_id)
        .bind(user_id)
        .bind(eleve_id)
        .fetch_optional(&mut *TenantTx::begin(&self.pool, etablissement_id).await.map_err(map_sqlx)?)
        .await
        .map_err(map_sqlx)?;
        Ok(row.map(|r| (LienRow::into_lien(r.lien), r.autorite_parentale)))
    }

    /// **Point unique des contrôles famille** (S2-1, B7) : lien ACTIF du compte
    /// sur l'élève (vue `ent_eleve_lien_actif` : non suspendu, dans ses dates,
    /// personne non fusionnée). `None` = aucun accès famille.
    pub async fn lien_parent_eleve(
        &self,
        etablissement_id: Uuid,
        user_id: Uuid,
        eleve_id: Uuid,
    ) -> Result<Option<EleveLien>, DomainError> {
        let row = sqlx::query_as::<_, LienRow>(
            r#"
            SELECT id, etablissement_id, eleve_id, user_id, foyer_id, lien,
                   peut_voir_notes, peut_voir_vs, peut_justifier_absence,
                   peut_autoriser_sorties, est_payeur, part_paiement
            FROM ent_eleve_lien_actif
            WHERE etablissement_id = $1 AND user_id = $2 AND eleve_id = $3
            "#,
        )
        .bind(etablissement_id)
        .bind(user_id)
        .bind(eleve_id)
        .fetch_optional(&mut *TenantTx::begin(&self.pool, etablissement_id).await.map_err(map_sqlx)?)
        .await
        .map_err(map_sqlx)?;
        Ok(row.map(LienRow::into_lien))
    }

    /// Un document déposé par `createur` est-il lisible du responsable `lecteur` ? (D-014,
    /// RP1-11) : oui si le déposant n'est pas un responsable (établissement, élève), si c'est
    /// le lecteur, ou si les deux responsables sont dans le **même foyer** pour cet élève.
    pub async fn document_famille_lisible(
        &self,
        etablissement_id: Uuid,
        eleve_id: Uuid,
        createur: Option<Uuid>,
        lecteur: Uuid,
    ) -> Result<bool, DomainError> {
        let Some(createur) = createur else {
            return Ok(true);
        };
        if createur == lecteur {
            return Ok(true);
        }
        let (ok,): (bool,) = sqlx::query_as(
            r#"
            SELECT NOT EXISTS (SELECT 1 FROM ent_membership m
                                WHERE m.user_id = $3 AND m.etablissement_id = $1 AND m.role = 'parent')
                OR EXISTS (SELECT 1 FROM ent_eleve_lien a
                             JOIN ent_eleve_lien b
                               ON b.etablissement_id = a.etablissement_id
                              AND b.eleve_id = a.eleve_id AND b.foyer_id = a.foyer_id
                            WHERE a.etablissement_id = $1 AND a.eleve_id = $2
                              AND a.user_id = $3 AND b.user_id = $4)
            "#,
        )
        .bind(etablissement_id)
        .bind(eleve_id)
        .bind(createur)
        .bind(lecteur)
        .fetch_one(&mut *TenantTx::begin(&self.pool, etablissement_id).await.map_err(map_sqlx)?)
        .await
        .map_err(map_sqlx)?;
        Ok(ok)
    }

    /// E-mails des **payeurs actifs** d'un foyer (facture, quittance : D-014).
    pub async fn emails_payeurs_foyer(
        &self,
        etablissement_id: Uuid,
        foyer_id: Uuid,
    ) -> Result<Vec<String>, DomainError> {
        let rows: Vec<(String,)> = sqlx::query_as(
            r#"
            SELECT DISTINCT u.email
            FROM ent_eleve_lien_communicant l
            JOIN ent_user u ON u.id = l.user_id
            WHERE l.etablissement_id = $1 AND l.foyer_id = $2 AND l.est_payeur
              AND u.email IS NOT NULL AND trim(u.email) <> ''
            ORDER BY u.email
            "#,
        )
        .bind(etablissement_id)
        .bind(foyer_id)
        .fetch_all(&mut *TenantTx::begin(&self.pool, etablissement_id).await.map_err(map_sqlx)?)
        .await
        .map_err(map_sqlx)?;
        Ok(rows.into_iter().map(|(e,)| e).collect())
    }

    /// E-mails des responsables liés à l'élève (portail famille).
    ///
    /// Lit les coordonnées de la **personne** (D-020), qu'elle ait un compte ou non ; l'e-mail
    /// du compte n'est qu'un repli quand la fiche n'en a pas. Respecte ses préférences
    /// (`ent_personne_preference.canal_email`). Un parent sans compte reçoit donc ses messages.
    /// Une personne marquée « ne pas solliciter » (A0-c) ne reçoit aucun envoi automatique.
    pub async fn emails_parents_eleve(
        &self,
        etablissement_id: Uuid,
        eleve_id: Uuid,
    ) -> Result<Vec<String>, DomainError> {
        let rows: Vec<(String,)> = sqlx::query_as(
            r#"
            SELECT DISTINCT lower(btrim(COALESCE(NULLIF(btrim(p.email), ''), NULLIF(btrim(u.email), '')))) AS adresse
            FROM ent_eleve_lien_communicant l
            JOIN ent_personne p ON p.id = l.personne_id AND p.etablissement_id = l.etablissement_id
            LEFT JOIN ent_user u ON u.id = p.user_id
            LEFT JOIN ent_personne_preference pr ON pr.personne_id = p.id
            WHERE l.etablissement_id = $1 AND l.eleve_id = $2
              AND COALESCE(pr.canal_email, true)
              AND NOT EXISTS (SELECT 1 FROM ent_personne_reserve nps
                               WHERE nps.etablissement_id = p.etablissement_id
                                 AND nps.personne_id = p.id AND nps.ne_pas_solliciter)
              AND COALESCE(NULLIF(btrim(p.email), ''), NULLIF(btrim(u.email), '')) IS NOT NULL
            ORDER BY adresse
            "#,
        )
        .bind(etablissement_id)
        .bind(eleve_id)
        .fetch_all(&mut *TenantTx::begin(&self.pool, etablissement_id).await.map_err(map_sqlx)?)
        .await
        .map_err(map_sqlx)?;
        Ok(rows.into_iter().map(|(e,)| e).collect())
    }

    /// Téléphones des responsables liés (SMS) : fiche de la personne d'abord, compte en repli,
    /// préférences respectées (`canal_sms`).
    pub async fn telephones_parents_eleve(
        &self,
        etablissement_id: Uuid,
        eleve_id: Uuid,
    ) -> Result<Vec<String>, DomainError> {
        let rows: Vec<(String,)> = sqlx::query_as(
            r#"
            SELECT DISTINCT COALESCE(NULLIF(btrim(p.telephone), ''), NULLIF(btrim(u.telephone), '')) AS numero
            FROM ent_eleve_lien_communicant l
            JOIN ent_personne p ON p.id = l.personne_id AND p.etablissement_id = l.etablissement_id
            LEFT JOIN ent_user u ON u.id = p.user_id
            LEFT JOIN ent_personne_preference pr ON pr.personne_id = p.id
            WHERE l.etablissement_id = $1 AND l.eleve_id = $2
              AND COALESCE(pr.canal_sms, true)
              AND NOT EXISTS (SELECT 1 FROM ent_personne_reserve nps
                               WHERE nps.etablissement_id = p.etablissement_id
                                 AND nps.personne_id = p.id AND nps.ne_pas_solliciter)
              AND COALESCE(NULLIF(btrim(p.telephone), ''), NULLIF(btrim(u.telephone), '')) IS NOT NULL
            ORDER BY numero
            "#,
        )
        .bind(etablissement_id)
        .bind(eleve_id)
        .fetch_all(&mut *TenantTx::begin(&self.pool, etablissement_id).await.map_err(map_sqlx)?)
        .await
        .map_err(map_sqlx)?;
        Ok(rows.into_iter().map(|(e,)| e).collect())
    }

    pub async fn list_liens_eleve(
        &self,
        etablissement_id: Uuid,
        eleve_id: Uuid,
    ) -> Result<Vec<EleveLien>, DomainError> {
        let rows = sqlx::query_as::<_, LienRow>(
            r#"
            SELECT id, etablissement_id, eleve_id, user_id, foyer_id, lien,
                   peut_voir_notes, peut_voir_vs, peut_justifier_absence,
                   peut_autoriser_sorties, est_payeur, part_paiement
            FROM ent_eleve_lien
            WHERE etablissement_id = $1 AND eleve_id = $2 AND user_id IS NOT NULL
            ORDER BY created_at
            "#,
        )
        .bind(etablissement_id)
        .bind(eleve_id)
        .fetch_all(&mut *TenantTx::begin(&self.pool, etablissement_id).await.map_err(map_sqlx)?)
        .await
        .map_err(map_sqlx)?;
        Ok(rows.into_iter().map(LienRow::into_lien).collect())
    }

    pub async fn list_liens_dossier(
        &self,
        etablissement_id: Uuid,
        eleve_id: Uuid,
    ) -> Result<Vec<LienDossier>, DomainError> {
        let rows = sqlx::query_as::<_, LienDossierRow>(
            r#"
            SELECT l.id AS lien_id, l.user_id, l.foyer_id, f.libelle AS foyer_libelle,
                   l.lien, u.display_name, u.telephone, u.email,
                   l.peut_voir_notes, l.peut_voir_vs, l.peut_justifier_absence,
                   l.peut_autoriser_sorties, l.est_payeur, l.part_paiement
            FROM ent_eleve_lien l
            LEFT JOIN ent_foyer f ON f.id = l.foyer_id
            LEFT JOIN ent_user u ON u.id = l.user_id
            WHERE l.etablissement_id = $1 AND l.eleve_id = $2 AND l.user_id IS NOT NULL
            ORDER BY l.created_at
            "#,
        )
        .bind(etablissement_id)
        .bind(eleve_id)
        .fetch_all(&mut *TenantTx::begin(&self.pool, etablissement_id).await.map_err(map_sqlx)?)
        .await
        .map_err(map_sqlx)?;
        Ok(rows
            .into_iter()
            .map(|r| LienDossier {
                lien_id: r.lien_id,
                user_id: Some(r.user_id),
                foyer_id: r.foyer_id,
                foyer_libelle: r.foyer_libelle,
                lien: r.lien,
                display_name: r.display_name,
                telephone: r.telephone,
                email: r.email,
                peut_voir_notes: r.peut_voir_notes,
                peut_voir_vs: r.peut_voir_vs,
                peut_justifier_absence: r.peut_justifier_absence,
                peut_autoriser_sorties: r.peut_autoriser_sorties,
                est_payeur: r.est_payeur,
                part_paiement: r.part_paiement,
            })
            .collect())
    }

    pub async fn eleves_lies(
        &self,
        etablissement_id: Uuid,
        user_id: Uuid,
    ) -> Result<Vec<Uuid>, DomainError> {
        let liens = self.liens_for_parent(etablissement_id, user_id).await?;
        Ok(liens.into_iter().map(|l| l.eleve_id).collect())
    }

    /// Liens ACTIFS d'un responsable (parent) dans l'établissement (point unique).
    pub async fn liens_for_parent(
        &self,
        etablissement_id: Uuid,
        user_id: Uuid,
    ) -> Result<Vec<EleveLien>, DomainError> {
        let rows = sqlx::query_as::<_, LienRow>(
            r#"
            SELECT id, etablissement_id, eleve_id, user_id, foyer_id, lien,
                   peut_voir_notes, peut_voir_vs, peut_justifier_absence,
                   peut_autoriser_sorties, est_payeur, part_paiement
            FROM ent_eleve_lien_actif
            WHERE etablissement_id = $1 AND user_id = $2
            ORDER BY eleve_id
            "#,
        )
        .bind(etablissement_id)
        .bind(user_id)
        .fetch_all(&mut *TenantTx::begin(&self.pool, etablissement_id).await.map_err(map_sqlx)?)
        .await
        .map_err(map_sqlx)?;
        Ok(rows.into_iter().map(LienRow::into_lien).collect())
    }
}

#[derive(Debug, sqlx::FromRow)]
struct LienRow {
    id: Uuid,
    etablissement_id: Uuid,
    eleve_id: Uuid,
    user_id: Uuid,
    foyer_id: Option<Uuid>,
    lien: String,
    peut_voir_notes: bool,
    peut_voir_vs: bool,
    peut_justifier_absence: bool,
    peut_autoriser_sorties: bool,
    est_payeur: bool,
    part_paiement: Option<i16>,
}

impl LienRow {
    fn into_lien(self) -> EleveLien {
        EleveLien {
            id: self.id,
            etablissement_id: self.etablissement_id,
            eleve_id: self.eleve_id,
            user_id: self.user_id,
            foyer_id: self.foyer_id,
            lien: self.lien,
            peut_voir_notes: self.peut_voir_notes,
            peut_voir_vs: self.peut_voir_vs,
            peut_justifier_absence: self.peut_justifier_absence,
            peut_autoriser_sorties: self.peut_autoriser_sorties,
            est_payeur: self.est_payeur,
            part_paiement: self.part_paiement,
        }
    }
}

#[derive(Debug, sqlx::FromRow)]
struct LienDossierRow {
    lien_id: Uuid,
    user_id: Uuid,
    foyer_id: Option<Uuid>,
    foyer_libelle: Option<String>,
    lien: String,
    display_name: Option<String>,
    telephone: Option<String>,
    email: Option<String>,
    peut_voir_notes: bool,
    peut_voir_vs: bool,
    peut_justifier_absence: bool,
    peut_autoriser_sorties: bool,
    est_payeur: bool,
    part_paiement: Option<i16>,
}

#[derive(Debug, sqlx::FromRow)]
struct LienFoyerRow {
    lien_id: Uuid,
    eleve_id: Uuid,
    eleve_nom: String,
    eleve_prenom: String,
    eleve_classe: Option<String>,
    user_id: Uuid,
    display_name: Option<String>,
    user_email: Option<String>,
    telephone: Option<String>,
    lien: String,
    peut_voir_notes: bool,
    peut_voir_vs: bool,
    peut_justifier_absence: bool,
    peut_autoriser_sorties: bool,
    est_payeur: bool,
    part_paiement: Option<i16>,
}

fn map_sqlx(err: sqlx::Error) -> DomainError {
    crate::pg_err::map_sqlx("foyer_repo", err)
}
