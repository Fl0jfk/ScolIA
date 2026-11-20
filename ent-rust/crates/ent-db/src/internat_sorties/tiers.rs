//! Socle 41, lot I3, suites (M3) — **tiers autorisé à récupérer** : un grand-parent, beau-parent ou proche sans autorité
//! parentale est remis à l'élève s'il porte un statut explicite, posé par le personnel (droit `internat.sorties.decider` :
//! CPE, vie scolaire, direction) à la demande ÉCRITE d'un titulaire, daté (du / au), motivé, historisé (faits `tiers_*`),
//! révocable. Il passe par l'unique fonction de remise `ent_internat_remise` (porte « tiers autorisé ») : interdiction
//! toujours prioritaire, acte qui tombe avec son auteur (le demandeur), refus par défaut, désaccord entre titulaires = le
//! plus restrictif jusqu'à la décision de la direction.

use super::*;

#[derive(sqlx::FromRow)]
struct TiersLigne {
    id: Uuid,
    eleve_id: Uuid,
    personne_id: Uuid,
    nom: String,
    prenom: String,
    du: NaiveDate,
    au: NaiveDate,
    statut: String,
    demande_par_personne_id: Uuid,
    motif: String,
}

const SELECT_TIERS: &str = "SELECT t.id, t.eleve_id, t.personne_id, p.nom, p.prenom, t.du, t.au, t.statut, \
     t.demande_par_personne_id, t.motif \
     FROM ent_internat_tiers_autorise t JOIN ent_personne p ON p.etablissement_id = t.etablissement_id AND p.id = t.personne_id";

/// Le titulaire (fiche) peut-il demander : lien actif avec autorité, ni visé par une interdiction ni privé de son droit ?
async fn demandeur_valide_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    eleve: Uuid,
    personne: Uuid,
    jour: NaiveDate,
) -> Result<bool, EntDbError> {
    sqlx::query_scalar(
        "SELECT GREATEST(ent_internat_personne_visee($1, $2, $3, $4), \
                         ent_internat_perte_droit($1, $2, ARRAY[$3::uuid], NULL, $4, true)) IS NULL",
    )
    .bind(etab)
    .bind(eleve)
    .bind(personne)
    .bind(jour)
    .fetch_one(&mut *conn)
    .await
    .map_err(db)
}

async fn ligne_tiers_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    id: Uuid,
    verrou: bool,
) -> Result<TiersLigne, EntDbError> {
    let l: Option<TiersLigne> = sqlx::query_as(&format!(
        "{SELECT_TIERS} WHERE t.etablissement_id = $1 AND t.id = $2{}",
        if verrou { " FOR UPDATE OF t" } else { "" }
    ))
    .bind(etab)
    .bind(id)
    .fetch_optional(&mut *conn)
    .await
    .map_err(db)?;
    Ok(l.ok_or(DomainError::NotFound)?)
}

async fn dto_tiers_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    l: TiersLigne,
    personnel: bool,
    jour: NaiveDate,
) -> Result<TiersDto, EntDbError> {
    let remettable = l.statut == "actif"
        && matches!(remise_possible_tx(conn, etab, l.eleve_id, l.personne_id, jour).await?,
                    Remise::Autorisee(ref r) if r == "tiers autorisé");
    Ok(TiersDto {
        id: l.id,
        eleve_id: l.eleve_id,
        personne_id: l.personne_id,
        nom: l.nom,
        prenom: l.prenom,
        du: l.du,
        au: l.au,
        statut: l.statut,
        remettable_aujourdhui: remettable,
        demande_par_personne_id: personnel.then_some(l.demande_par_personne_id),
        motif_texte: personnel.then_some(l.motif),
    })
}

/// Fiche de titulaire du compte pour l'élève (lien avec autorité, non visé) : la personne dont il agit.
async fn fiche_titulaire_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    user: Uuid,
    eleve: Uuid,
) -> Result<Vec<Uuid>, EntDbError> {
    sqlx::query_scalar(
        "SELECT personne_id FROM ent_eleve_lien WHERE etablissement_id = $1 AND user_id = $2 AND eleve_id = $3",
    )
    .bind(etab)
    .bind(user)
    .bind(eleve)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)
}

impl PgEnt {
    /// Pose le statut. Refus : personne visée par une interdiction (toujours prioritaire), dont un lien avec l'élève est
    /// suspendu, en garde « aucune », terminé ou pas commencé (une décision de justice ne se contourne pas par ce statut) ;
    /// demandeur qui n'est pas un titulaire actif non visé ; dates hors bornes ; motif absent. Un désaccord connu du socle 18
    /// pose le statut « conteste » (porte fermée jusqu'à la direction).
    pub async fn poser_tiers_a(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        b: &TiersPoseDto,
        maintenant: DateTime<Utc>,
    ) -> Result<TiersDto, EntDbError> {
        let motif = texte_court(Some(&b.motif_texte))?
            .ok_or_else(|| refus("motif obligatoire : le statut de tiers autorisé est une décision explicite, tracée"))?;
        if b.au < b.du || b.au > b.du + Duration::days(366) {
            return Err(refus(
                "période invalide : du / au obligatoires, un an au plus",
            ));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let jour = jour_local_tx(&mut tx, etab, maintenant).await?;
        self.personnel_exiger(
            etab,
            actor,
            role,
            &[Action::InternatSortiesDecider],
            b.eleve_id,
        )
        .await?;
        if b.demande_ecrite_le > jour {
            return Err(refus(
                "la date de la demande écrite ne peut pas être dans le futur",
            ));
        }
        if b.personne_id == b.demande_par_personne_id
            || meme_personne_tx(&mut tx, etab, b.personne_id, b.demande_par_personne_id).await?
        {
            return Err(refus(
                "le tiers ne peut pas être le titulaire qui le demande",
            ));
        }
        let personne_ok: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM ent_personne WHERE etablissement_id = $1 AND id = $2 AND fusionnee_dans IS NULL)",
        )
        .bind(etab)
        .bind(b.personne_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        if !personne_ok {
            return Err(DomainError::NotFound.into());
        }
        if interdiction_personne_depuis_tx(&mut tx, etab, b.eleve_id, b.personne_id, jour)
            .await?
            .is_some()
        {
            return Err(refus("cette personne fait l'objet d'une interdiction : elle ne peut pas être autorisée à récupérer l'élève"));
        }
        let ecarte: bool = sqlx::query_scalar("SELECT ent_internat_lien_ecarte($1, $2, $3, $4)")
            .bind(etab)
            .bind(b.eleve_id)
            .bind(b.personne_id)
            .bind(jour)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
        if ecarte {
            return Err(refus(
                "le lien de cette personne avec l'élève est suspendu, en garde « aucune » ou échu : le statut de tiers ne le remplace pas",
            ));
        }
        if !demandeur_valide_tx(&mut tx, etab, b.eleve_id, b.demande_par_personne_id, jour).await? {
            return Err(refus("le demandeur n'est pas un titulaire de l'autorité parentale en exercice, ou il est visé par une interdiction"));
        }
        let statut = if desaccord_connu_tx(&mut tx, etab, b.eleve_id).await? {
            "conteste"
        } else {
            "actif"
        };
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_internat_tiers_autorise \
             (id, etablissement_id, eleve_id, personne_id, demande_par_personne_id, demande_ecrite_le, du, au, motif, \
              statut, pose_par_user_id, cree_le, modifie_le) \
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$12)",
        )
        .bind(id)
        .bind(etab)
        .bind(b.eleve_id)
        .bind(b.personne_id)
        .bind(b.demande_par_personne_id)
        .bind(b.demande_ecrite_le)
        .bind(b.du)
        .bind(b.au)
        .bind(&motif)
        .bind(statut)
        .bind(actor)
        .bind(maintenant)
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            if matches!(&e, sqlx::Error::Database(d) if d.code().as_deref() == Some("23505")) {
                conflit("cette personne est déjà tiers autorisé de cet élève")
            } else {
                db(e)
            }
        })?;
        let motif_code = (statut == "conteste").then_some("desaccord_titulaires");
        fait_tx(
            &mut tx,
            etab,
            Cible::Tiers(id),
            "tiers_pose",
            "cpe",
            Some(actor),
            maintenant,
            motif_code,
            Some(&motif),
            Some(b.personne_id),
            None,
        )
        .await?;
        self.evenement_tiers_tx(&mut tx, etab, id, b.eleve_id, "tiers_pose", actor)
            .await?;
        let l = ligne_tiers_tx(&mut tx, etab, id, false).await?;
        let dto = dto_tiers_tx(&mut tx, etab, l, true, jour).await?;
        tx.commit().await.map_err(db)?;
        Ok(dto)
    }

    async fn evenement_tiers_tx(
        &self,
        conn: &mut TenantTx<'_>,
        etab: Uuid,
        id: Uuid,
        eleve: Uuid,
        action: &str,
        actor: Uuid,
    ) -> Result<(), EntDbError> {
        publier(
            conn,
            etab,
            types::INTERNAT_CORRESPONDANT,
            AggregateKind::Internat,
            id,
            Some(eleve),
            json!({ "action": action }),
            Some(actor),
            Causalite::default(),
        )
        .await?;
        Ok(())
    }

    /// Les tiers d'un élève (famille : titulaire, élève majeur ; personnel : portée de l'internat, lecture journalisée).
    pub async fn tiers_eleve_a(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        eleve: Uuid,
        maintenant: DateTime<Utc>,
    ) -> Result<Vec<TiersDto>, EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let jour = jour_local_tx(&mut tx, etab, maintenant).await?;
        let famille = self
            .famille_lecture_tx(&mut tx, etab, actor, role, eleve, jour)
            .await?;
        let personnel = famille.is_none();
        if personnel {
            self.personnel_exiger(
                etab,
                actor,
                role,
                &[
                    Action::InternatLire,
                    Action::InternatSortiesDecider,
                    Action::InternatAppel,
                ],
                eleve,
            )
            .await?;
            crate::internat_nuit::journaliser_lecture_eleves_tx(
                &mut tx,
                etab,
                actor,
                "tiers",
                Some(eleve),
                None,
                &[eleve],
                maintenant,
            )
            .await?;
        }
        let lignes: Vec<TiersLigne> = sqlx::query_as(&format!(
            "{SELECT_TIERS} WHERE t.etablissement_id = $1 AND t.eleve_id = $2 AND t.statut <> 'revoque' \
              ORDER BY p.nom, p.prenom, t.id"
        ))
        .bind(etab)
        .bind(eleve)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let mut v = Vec::with_capacity(lignes.len());
        for l in lignes {
            v.push(dto_tiers_tx(&mut tx, etab, l, personnel, jour).await?);
        }
        tx.commit().await.map_err(db)?;
        Ok(v)
    }

    /// Révocation : le personnel de l'internat, ou le titulaire qui a fait la demande (jamais un autre titulaire, qui
    /// s'oppose). Effet immédiat : les sorties à venir avec cette personne sont revues dans la même transaction.
    pub async fn revoquer_tiers_a(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        id: Uuid,
        b: &CorrespondantSuspensionDto,
        maintenant: DateTime<Utc>,
    ) -> Result<(), EntDbError> {
        let motif = texte_court(b.motif_texte.as_deref())?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let l = ligne_tiers_tx(&mut tx, etab, id, true).await?;
        let jour = jour_local_tx(&mut tx, etab, maintenant).await?;
        let qualite = match self
            .famille_tx(&mut tx, etab, actor, role, l.eleve_id, jour)
            .await?
        {
            Some(Famille {
                qualite: Qualite::Titulaire,
                ..
            }) => {
                let mut est_demandeur = false;
                for p in fiche_titulaire_tx(&mut tx, etab, actor, l.eleve_id).await? {
                    est_demandeur |=
                        meme_personne_tx(&mut tx, etab, p, l.demande_par_personne_id).await?;
                }
                if !est_demandeur {
                    return Err(EntDbError::Forbidden);
                }
                "titulaire"
            }
            Some(_) => return Err(EntDbError::Forbidden),
            None => {
                self.personnel_exiger(
                    etab,
                    actor,
                    role,
                    &[Action::InternatSortiesDecider],
                    l.eleve_id,
                )
                .await?;
                "cpe"
            }
        };
        let n = sqlx::query(
            "UPDATE ent_internat_tiers_autorise SET statut = 'revoque', modifie_le = $3 \
              WHERE etablissement_id = $1 AND id = $2 AND statut IN ('actif', 'conteste')",
        )
        .bind(etab)
        .bind(id)
        .bind(maintenant)
        .execute(&mut *tx)
        .await
        .map_err(db)?
        .rows_affected();
        if n == 0 {
            return Err(conflit("ce tiers est déjà révoqué"));
        }
        fait_tx(
            &mut tx,
            etab,
            Cible::Tiers(id),
            "tiers_revocation",
            qualite,
            Some(actor),
            maintenant,
            None,
            motif.as_deref(),
            Some(l.personne_id),
            None,
        )
        .await?;
        self.evenement_tiers_tx(&mut tx, etab, id, l.eleve_id, "tiers_revoque", actor)
            .await?;
        self.reviser_sorties_tx(&mut tx, etab, Some(l.eleve_id), jour, maintenant)
            .await?;
        tx.commit().await.map_err(db)?;
        Ok(())
    }

    /// Opposition d'un titulaire **autre que le demandeur** : la porte se ferme aussitôt (le plus restrictif l'emporte) ;
    /// la direction tranche. La famille ne voit jamais qui s'oppose.
    pub async fn opposer_tiers_a(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        id: Uuid,
        maintenant: DateTime<Utc>,
    ) -> Result<(), EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let l = ligne_tiers_tx(&mut tx, etab, id, true).await?;
        let jour = jour_local_tx(&mut tx, etab, maintenant).await?;
        match self
            .famille_tx(&mut tx, etab, actor, role, l.eleve_id, jour)
            .await?
        {
            Some(Famille {
                qualite: Qualite::Titulaire,
                ..
            }) => {}
            _ => return Err(EntDbError::Forbidden),
        }
        for p in fiche_titulaire_tx(&mut tx, etab, actor, l.eleve_id).await? {
            if meme_personne_tx(&mut tx, etab, p, l.demande_par_personne_id).await? {
                return Err(refus(
                    "le titulaire qui a fait la demande la révoque, il ne s'y oppose pas",
                ));
            }
        }
        let n = sqlx::query(
            "UPDATE ent_internat_tiers_autorise SET statut = 'conteste', modifie_le = $3 \
              WHERE etablissement_id = $1 AND id = $2 AND statut = 'actif'",
        )
        .bind(etab)
        .bind(id)
        .bind(maintenant)
        .execute(&mut *tx)
        .await
        .map_err(db)?
        .rows_affected();
        if n == 0 {
            return Err(conflit("l'état de ce tiers ne permet pas l'opposition"));
        }
        fait_tx(
            &mut tx,
            etab,
            Cible::Tiers(id),
            "tiers_opposition",
            "titulaire",
            Some(actor),
            maintenant,
            Some("desaccord_titulaires"),
            None,
            None,
            None,
        )
        .await?;
        self.evenement_tiers_tx(&mut tx, etab, id, l.eleve_id, "tiers_conteste", actor)
            .await?;
        self.reviser_sorties_tx(&mut tx, etab, Some(l.eleve_id), jour, maintenant)
            .await?;
        tx.commit().await.map_err(db)?;
        Ok(())
    }

    /// La direction (ou l'administration) seule tranche un tiers contesté : `lever` (motif obligatoire) ou `maintenir`
    /// (le statut est révoqué).
    pub async fn trancher_tiers_a(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        id: Uuid,
        b: &TrancheDto,
        maintenant: DateTime<Utc>,
    ) -> Result<(), EntDbError> {
        let motif = texte_court(b.motif_texte.as_deref())?;
        let (vers, acte) = match b.decision.as_str() {
            "lever" => ("actif", "tiers_levee"),
            "maintenir" => ("revoque", "tiers_maintenu"),
            _ => return Err(refus("décision attendue : lever ou maintenir")),
        };
        if vers == "actif" && motif.is_none() {
            return Err(refus(
                "motif obligatoire pour lever un désaccord sur un tiers autorisé",
            ));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let l = ligne_tiers_tx(&mut tx, etab, id, true).await?;
        self.direction_exiger(etab, actor, role, l.eleve_id).await?;
        let jour = jour_local_tx(&mut tx, etab, maintenant).await?;
        let n = sqlx::query(
            "UPDATE ent_internat_tiers_autorise SET statut = $3, modifie_le = $4 \
              WHERE etablissement_id = $1 AND id = $2 AND statut = 'conteste'",
        )
        .bind(etab)
        .bind(id)
        .bind(vers)
        .bind(maintenant)
        .execute(&mut *tx)
        .await
        .map_err(db)?
        .rows_affected();
        if n == 0 {
            return Err(conflit("seul un tiers contesté se tranche"));
        }
        fait_tx(
            &mut tx,
            etab,
            Cible::Tiers(id),
            acte,
            "direction",
            Some(actor),
            maintenant,
            Some("desaccord_titulaires"),
            motif.as_deref(),
            None,
            None,
        )
        .await?;
        self.evenement_tiers_tx(&mut tx, etab, id, l.eleve_id, acte, actor)
            .await?;
        self.reviser_sorties_tx(&mut tx, etab, Some(l.eleve_id), jour, maintenant)
            .await?;
        tx.commit().await.map_err(db)?;
        Ok(())
    }

    /// Un autre titulaire reprend la demande quand le demandeur est tombé (interdiction ou perte de droit) : le personnel
    /// consigne la demande écrite du nouveau titulaire. Sans cela, la porte reste fermée (l'acte est tombé avec son auteur).
    pub async fn reprendre_tiers_a(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        id: Uuid,
        b: &TiersRepriseDto,
        maintenant: DateTime<Utc>,
    ) -> Result<(), EntDbError> {
        let motif = texte_court(b.motif_texte.as_deref())?.ok_or_else(|| {
            refus("motif obligatoire : la reprise de la demande par un autre titulaire est tracée")
        })?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let l = ligne_tiers_tx(&mut tx, etab, id, true).await?;
        self.personnel_exiger(
            etab,
            actor,
            role,
            &[Action::InternatSortiesDecider],
            l.eleve_id,
        )
        .await?;
        let jour = jour_local_tx(&mut tx, etab, maintenant).await?;
        if b.demande_par_personne_id == l.personne_id
            || meme_personne_tx(&mut tx, etab, b.demande_par_personne_id, l.personne_id).await?
        {
            return Err(refus(
                "le tiers ne peut pas être le titulaire qui le demande",
            ));
        }
        if !demandeur_valide_tx(&mut tx, etab, l.eleve_id, b.demande_par_personne_id, jour).await? {
            return Err(refus("le nouveau demandeur n'est pas un titulaire de l'autorité parentale en exercice, ou il est visé"));
        }
        let n = sqlx::query(
            "UPDATE ent_internat_tiers_autorise SET demande_par_personne_id = $3, modifie_le = $4 \
              WHERE etablissement_id = $1 AND id = $2 AND statut <> 'revoque'",
        )
        .bind(etab)
        .bind(id)
        .bind(b.demande_par_personne_id)
        .bind(maintenant)
        .execute(&mut *tx)
        .await
        .map_err(db)?
        .rows_affected();
        if n == 0 {
            return Err(conflit("ce tiers est révoqué"));
        }
        fait_tx(
            &mut tx,
            etab,
            Cible::Tiers(id),
            "tiers_reprise",
            "cpe",
            Some(actor),
            maintenant,
            None,
            Some(&motif),
            Some(b.demande_par_personne_id),
            None,
        )
        .await?;
        self.evenement_tiers_tx(&mut tx, etab, id, l.eleve_id, "tiers_repris", actor)
            .await?;
        self.reviser_sorties_tx(&mut tx, etab, Some(l.eleve_id), jour, maintenant)
            .await?;
        tx.commit().await.map_err(db)?;
        Ok(())
    }
}
