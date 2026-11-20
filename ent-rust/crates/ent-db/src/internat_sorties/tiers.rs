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
    cree_le: DateTime<Utc>,
}

/// Ce que le lecteur a le droit de voir d'un tiers (M4, M5) : la **famille** (statut neutre, ni motif ni demandeur), le
/// **personnel de l'internat** sans décision (état réel, ni motif ni demandeur) ou le **décideur** (`internat.sorties.decider`
/// : CPE, vie scolaire, direction, administration), seul à lire le motif et le demandeur.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Vue {
    Famille,
    Personnel,
    Decideur,
}

/// Alerte avant échéance : un statut qui s'achève dans ce délai (jours) est signalé aux titulaires et au personnel.
const ECHEANCE_PROCHE_JOURS: i64 = 14;
/// Durée maximale d'un statut : un an (365 jours entre « du » et « au »).
const DUREE_MAX_JOURS: i64 = 365;

const SELECT_TIERS: &str = "SELECT t.id, t.eleve_id, t.personne_id, p.nom, p.prenom, t.du, t.au, t.statut, \
     t.demande_par_personne_id, t.motif, t.cree_le \
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
    vue: Vue,
    jour: NaiveDate,
) -> Result<TiersDto, EntDbError> {
    let remettable = l.statut == "actif"
        && matches!(remise_possible_tx(conn, etab, l.eleve_id, l.personne_id, jour).await?,
                    Remise::Autorisee(ref r) if r == "tiers autorisé");
    let en_periode = l.du <= jour && jour <= l.au;
    // M5 : la famille ne voit qu'un libellé neutre quand la porte est fermée en pleine période, quelle qu'en soit la cause
    // (opposition, décision en cours, demandeur ou lien écarté, interdiction) : elle n'en déduit ni l'auteur ni le motif.
    let statut = if vue == Vue::Famille && en_periode && !remettable {
        "en_cours_d_examen".to_string()
    } else {
        l.statut
    };
    Ok(TiersDto {
        id: l.id,
        eleve_id: l.eleve_id,
        personne_id: l.personne_id,
        nom: l.nom,
        prenom: l.prenom,
        du: l.du,
        au: l.au,
        statut,
        remettable_aujourdhui: remettable,
        echeance_proche: jour <= l.au && (l.au - jour).num_days() <= ECHEANCE_PROCHE_JOURS,
        demande_par_personne_id: (vue == Vue::Decideur).then_some(l.demande_par_personne_id),
        motif_texte: (vue == Vue::Decideur).then_some(l.motif),
    })
}

/// Conflit d'intérêt (même règle que la requalification d'un contact) : un agent ne pose ni ne reprend un statut de tiers
/// dont il est lui-même la personne, le demandeur ou un des deux (chaîne de fusions comprise), ni pour un élève dont il est
/// un proche (lien avec l'élève, ou compte de l'élève). Un autre agent doit le faire.
async fn conflit_interet_tiers_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    agent: Uuid,
    eleve: Uuid,
    personnes: &[Uuid],
) -> Result<bool, EntDbError> {
    sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM ent_personne me WHERE me.etablissement_id = $1 AND me.user_id = $3 \
                          AND me.id IN (SELECT ent_internat_personne_chaine($1, x) FROM unnest($4::uuid[]) x)) \
             OR EXISTS (SELECT 1 FROM ent_eleve_lien l WHERE l.etablissement_id = $1 AND l.eleve_id = $2 \
                          AND (l.user_id = $3 OR l.personne_id IN (SELECT id FROM ent_personne \
                                                                    WHERE etablissement_id = $1 AND user_id = $3))) \
             OR EXISTS (SELECT 1 FROM ent_eleve_compte k \
                         WHERE k.etablissement_id = $1 AND k.eleve_id = $2 AND k.user_id = $3)",
    )
    .bind(etab)
    .bind(eleve)
    .bind(agent)
    .bind(personnes)
    .fetch_one(&mut *conn)
    .await
    .map_err(db)
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
        if b.au < b.du || b.au > b.du + Duration::days(DUREE_MAX_JOURS) {
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
        if conflit_interet_tiers_tx(
            &mut tx,
            etab,
            actor,
            b.eleve_id,
            &[b.personne_id, b.demande_par_personne_id],
        )
        .await?
        {
            return Err(refus(
                "conflit d'intérêt : vous ne pouvez pas poser vous-même, ni pour un proche, le statut de tiers autorisé d'un élève dont vous êtes un proche ; un autre agent doit le faire",
            ));
        }
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
                "le lien de cette personne avec l'élève est suspendu, en garde « aucune », échu ou pas commencé, ou c'est celui d'un parent ou tuteur sans autorité parentale : le statut de tiers ne le remplace pas",
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
        self.evenement_tiers_tx(&mut tx, etab, id, b.eleve_id, "tiers_pose", Some(actor))
            .await?;
        let l = ligne_tiers_tx(&mut tx, etab, id, false).await?;
        let dto = dto_tiers_tx(&mut tx, etab, l, Vue::Decideur, jour).await?;
        tx.commit().await.map_err(db)?;
        Ok(dto)
    }

    /// Événement dédié aux tiers (F3) : identifiants et action seulement — **jamais le motif, le demandeur ni l'opposant**.
    /// Destinataires : tous les titulaires de l'élève (M3, résolus par le socle 8, qui écarte les personnes visées).
    pub(super) async fn evenement_tiers_tx(
        &self,
        conn: &mut PgConnection,
        etab: Uuid,
        id: Uuid,
        eleve: Uuid,
        action: &str,
        actor: Option<Uuid>,
    ) -> Result<(), EntDbError> {
        publier(
            conn,
            etab,
            types::INTERNAT_TIERS,
            AggregateKind::Internat,
            id,
            Some(eleve),
            json!({ "action": action, "destinataires": "titulaires" }),
            actor,
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
        let vue = if famille.is_some() {
            Vue::Famille
        } else {
            // M4 : motif et demandeur ne se lisent qu'avec `internat.sorties.decider` ; la lecture de l'internat ou de
            // l'appel ne donne que la liste (personne, dates, état).
            let vue = match self
                .personnel_exiger(etab, actor, role, &[Action::InternatSortiesDecider], eleve)
                .await
            {
                Ok(()) => Vue::Decideur,
                Err(EntDbError::Forbidden) => {
                    self.personnel_exiger(
                        etab,
                        actor,
                        role,
                        &[Action::InternatLire, Action::InternatAppel],
                        eleve,
                    )
                    .await?;
                    Vue::Personnel
                }
                Err(e) => return Err(e),
            };
            crate::internat_nuit::journaliser_lecture_eleves_tx(
                &mut tx,
                etab,
                actor,
                "correspondants",
                Some(eleve),
                None,
                &[eleve],
                maintenant,
            )
            .await?;
            vue
        };
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
            v.push(dto_tiers_tx(&mut tx, etab, l, vue, jour).await?);
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
        self.evenement_tiers_tx(&mut tx, etab, id, l.eleve_id, "tiers_revoque", Some(actor))
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
        // F1 : une fois le désaccord levé par la direction, une nouvelle opposition ne le rouvre pas (sinon le cycle
        // opposer / lever n'aurait pas de fin et la direction serait saisie sans limite) : la décision vaut ; le
        // personnel ou le demandeur reste libre de révoquer.
        if decision_apres_tx(&mut tx, etab, Cible::Tiers(id), l.cree_le).await? {
            return Err(conflit(
                "la direction a déjà tranché ce désaccord : une nouvelle opposition ne le rouvre pas ; l'établissement peut révoquer ce statut",
            ));
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
        self.evenement_tiers_tx(&mut tx, etab, id, l.eleve_id, "tiers_conteste", Some(actor))
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
        self.evenement_tiers_tx(&mut tx, etab, id, l.eleve_id, acte, Some(actor))
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
        if conflit_interet_tiers_tx(
            &mut tx,
            etab,
            actor,
            l.eleve_id,
            &[l.personne_id, b.demande_par_personne_id],
        )
        .await?
        {
            return Err(refus(
                "conflit d'intérêt : vous ne pouvez pas reprendre vous-même, ni pour un proche, la demande d'un élève dont vous êtes un proche ; un autre agent doit le faire",
            ));
        }
        // M3 : la reprise n'est permise que si le demandeur actuel est réellement tombé (interdiction ou perte de droit) ;
        // sinon elle servirait à changer de demandeur à volonté et à contourner la révocation.
        if demandeur_valide_tx(&mut tx, etab, l.eleve_id, l.demande_par_personne_id, jour).await? {
            return Err(conflit(
                "le demandeur actuel est toujours un titulaire en exercice : la demande ne se reprend pas (il peut la révoquer)",
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
        self.evenement_tiers_tx(&mut tx, etab, id, l.eleve_id, "tiers_repris", Some(actor))
            .await?;
        self.reviser_sorties_tx(&mut tx, etab, Some(l.eleve_id), jour, maintenant)
            .await?;
        tx.commit().await.map_err(db)?;
        Ok(())
    }
}

impl PgEnt {
    /// F4 — alerte avant échéance : les tiers dont le statut s'achève dans [`ECHEANCE_PROCHE_JOURS`] jours (non révoqués,
    /// pas encore échus) sont signalés **une seule fois** (fait `tiers_echeance`, idempotent) par un événement sans motif ni
    /// demandeur ; le renouvellement est une nouvelle pose (un an au plus). Renvoie le nombre d'alertes émises.
    pub(super) async fn alerter_tiers_echeance_tx(
        &self,
        conn: &mut PgConnection,
        etab: Uuid,
        jour: NaiveDate,
        maintenant: DateTime<Utc>,
    ) -> Result<i32, EntDbError> {
        let a_alerter: Vec<(Uuid, Uuid)> = sqlx::query_as(
            "SELECT t.id, t.eleve_id FROM ent_internat_tiers_autorise t \
              WHERE t.etablissement_id = $1 AND t.statut <> 'revoque' AND t.au >= $2 AND t.au - $2 <= $3 \
                AND NOT EXISTS (SELECT 1 FROM ent_internat_i3_fait f \
                                 WHERE f.etablissement_id = t.etablissement_id AND f.tiers_id = t.id \
                                   AND f.acte = 'tiers_echeance') \
              ORDER BY t.au, t.id",
        )
        .bind(etab)
        .bind(jour)
        .bind(i32::try_from(ECHEANCE_PROCHE_JOURS).unwrap_or(14))
        .fetch_all(&mut *conn)
        .await
        .map_err(db)?;
        for (id, eleve) in &a_alerter {
            fait_tx(
                conn,
                etab,
                Cible::Tiers(*id),
                "tiers_echeance",
                "systeme",
                None,
                maintenant,
                None,
                None,
                None,
                None,
            )
            .await?;
            self.evenement_tiers_tx(conn, etab, *id, *eleve, "tiers_echeance", None)
                .await?;
        }
        Ok(i32::try_from(a_alerter.len()).unwrap_or(i32::MAX))
    }
}
