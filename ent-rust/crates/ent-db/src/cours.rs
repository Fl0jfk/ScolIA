//! Public d'un cours et membres individuels (socle 5, lots S1 et S4).
//!
//! Depuis le lot S4 (migration 341) un **cours** est un objet à part, `ent_cours` : ce qui est
//! enseigné (matière) à qui (classe entière OU groupe), pour une année. Le **service**
//! (`ent_service_enseignement`, qui l'enseigne) pointe vers lui par `cours_id` ; un cours sans
//! service est un poste vacant. Les identifiants que l'API acceptait au lot S1 (ceux des lignes de
//! service) restent acceptés : [`resoudre_cours`] les ramène au cours. Son **public à la date d**
//! est calculé par **une seule** fonction SQL, `ent_public_cours_jour` :
//! `public = (classe ou groupe à d) ∪ inclusions actives − exclusions actives`, hors élèves
//! sortis. `perimetre.rs`, `eleves_autorises` et la liste « mes enseignements » s'y appuient :
//! aucune autre jointure service ↔ affectation ne doit exister ailleurs.
//!
//! Les **membres individuels** (R-S5) couvrent les cas où le public réel diffère de la classe
//! ou du groupe : élève d'ULIS inclus dans un cours d'une autre classe, décloisonnement, élève
//! accueilli (inclusion) ; échange, scolarité partagée, dispense (exclusion). Jamais supprimés :
//! annulés. Le motif (`ulis`, `upe2a`…) est une donnée sensible : réservé à la gestion de la
//! structure, absent des événements et de la liste du public (D-027).

use std::collections::HashSet;

use chrono::NaiveDate;
use ent_auth::Action;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::FromRow;
use uuid::Uuid;

use sqlx::PgConnection;

use crate::ref_commun::{db, introuvable, invalide, publier_ref, Acteur};
use crate::service::{EntDbError, PgEnt};
use crate::structure_etab::journaliser;
use crate::tenant_tx::TenantTx;

/// Ramène un identifiant reçu de l'extérieur au cours : soit déjà un cours de l'établissement,
/// soit une ligne de service (compatibilité du lot S1) → son cours. Inconnu, d'un autre
/// établissement, ou service sans cours → introuvable (aucun oracle).
pub(crate) async fn resoudre_cours(
    conn: &mut PgConnection,
    etab: Uuid,
    id: Uuid,
) -> Result<Uuid, EntDbError> {
    let r: Option<Uuid> = sqlx::query_scalar(
        "SELECT id FROM ent_cours WHERE etablissement_id = $1 AND id = $2
         UNION ALL
         SELECT cours_id FROM ent_service_enseignement
          WHERE etablissement_id = $1 AND id = $2 AND cours_id IS NOT NULL
         LIMIT 1",
    )
    .bind(etab)
    .bind(id)
    .fetch_optional(conn)
    .await
    .map_err(db)?;
    r.ok_or_else(introuvable)
}

/// Nature d'un membre individuel.
pub const NATURES_MEMBRE: [&str; 2] = ["inclusion", "exclusion"];

/// Motifs reconnus (liste fermée, sans donnée libre : minimisation).
pub const MOTIFS_MEMBRE: [&str; 10] = [
    "ulis",
    "upe2a",
    "decloisonnement",
    "avance",
    "parcours_amenage",
    "scolarite_partagee",
    "echange",
    "accueil_temporaire",
    "dispense_cours",
    "autre",
];

#[derive(Debug, Clone, PartialEq, Eq, FromRow, Serialize, Deserialize)]
pub struct CoursMembre {
    pub id: Uuid,
    pub cours_id: Uuid,
    pub eleve_id: Uuid,
    pub nature: String,
    pub motif_code: String,
    pub du: NaiveDate,
    pub au: Option<NaiveDate>,
    pub annulee: bool,
}

const COLONNES_MEMBRE: &str =
    "id, cours_id, eleve_id, nature, motif_code, du, au, (annulee_le IS NOT NULL) AS annulee";

impl PgEnt {
    /// Public du ou des cours à la date `jour` (paires cours × élève), par la règle unique.
    /// Filtres facultatifs : un cours, le professeur titulaire du cours, un élève.
    pub async fn public_cours(
        &self,
        etab: Uuid,
        jour: NaiveDate,
        cours: Option<Uuid>,
        prof: Option<Uuid>,
        eleve: Option<Uuid>,
    ) -> Result<Vec<(Uuid, Uuid)>, EntDbError> {
        // `cours` peut être un cours ou (compatibilité du lot S1) une ligne de service.
        sqlx::query_as(
            "SELECT cours_id, eleve_id FROM ent_public_cours_jour($1, $2,
                COALESCE((SELECT s.cours_id FROM ent_service_enseignement s
                           WHERE s.etablissement_id = $1 AND s.id = $3), $3), $4, $5)",
        )
        .bind(etab)
        .bind(jour)
        .bind(cours)
        .bind(prof)
        .bind(eleve)
        .fetch_all(&mut *TenantTx::begin(self.pool(), etab).await?)
        .await
        .map_err(db)
    }

    /// Public d'un cours **visible par l'acteur** : réservé à la gestion de la structure ou au
    /// professeur du cours, puis borné à ses élèves autorisés (`suivi_eleve`). Un cours d'un
    /// autre établissement ou inconnu → introuvable.
    pub async fn cours_public_pour_acteur(
        &self,
        a: &Acteur<'_>,
        cours_id: Uuid,
        jour: NaiveDate,
    ) -> Result<Vec<Uuid>, EntDbError> {
        let cours_id = {
            let mut c = TenantTx::begin(self.pool(), a.etablissement_id).await.map_err(db)?;
            resoudre_cours(&mut c, a.etablissement_id, cours_id).await?
        };
        // Professeur du cours = titulaire d'un service sur ce cours (co-interventions comprises).
        let est_prof: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM ent_service_enseignement
                             WHERE etablissement_id = $1 AND cours_id = $2 AND prof_user_id = $3)",
        )
        .bind(a.etablissement_id)
        .bind(cours_id)
        .bind(a.user_id)
        .fetch_one(&mut *TenantTx::begin(self.pool(), a.etablissement_id).await.map_err(db)?)
        .await
        .map_err(db)?;
        if !est_prof {
            self.exiger_referentiel(a, Action::ReferentielGerer).await?;
        } else if !ent_auth::Role::parse(a.role).is_some_and(|r| r.is_staff()) {
            return Err(EntDbError::Forbidden);
        }
        let autorises = self
            .eleves_autorises(a.etablissement_id, a.user_id, a.role, Action::SuiviEleve)
            .await?;
        let public: HashSet<Uuid> = self
            .public_cours(a.etablissement_id, jour, Some(cours_id), None, None)
            .await?
            .into_iter()
            .map(|(_, e)| e)
            .collect();
        let mut out: Vec<Uuid> = public
            .into_iter()
            .filter(|e| autorises.as_ref().is_none_or(|s| s.contains(e)))
            .collect();
        out.sort();
        Ok(out)
    }

    /// Membres individuels d'un cours (motifs inclus : gestion de la structure seulement).
    pub async fn cours_membres(
        &self,
        a: &Acteur<'_>,
        cours_id: Uuid,
    ) -> Result<Vec<CoursMembre>, EntDbError> {
        self.exiger_referentiel(a, Action::ReferentielGerer).await?;
        let cours_id = {
            let mut c = TenantTx::begin(self.pool(), a.etablissement_id).await.map_err(db)?;
            resoudre_cours(&mut c, a.etablissement_id, cours_id).await?
        };
        sqlx::query_as(&format!(
            "SELECT {COLONNES_MEMBRE} FROM ent_cours_membre
              WHERE etablissement_id = $1 AND cours_id = $2 ORDER BY du, created_at"
        ))
        .bind(a.etablissement_id)
        .bind(cours_id)
        .fetch_all(&mut *TenantTx::begin(self.pool(), a.etablissement_id).await.map_err(db)?)
        .await
        .map_err(db)
    }

    /// Ajoute un membre individuel (inclusion ou exclusion) daté. Refus : nature ou motif
    /// inconnu, élève ou cours d'un autre établissement, hors année du cours, chevauchement.
    #[allow(clippy::too_many_arguments)]
    pub async fn cours_membre_ajouter(
        &self,
        a: &Acteur<'_>,
        cours_id: Uuid,
        eleve_id: Uuid,
        nature: &str,
        motif_code: &str,
        du: NaiveDate,
        au: Option<NaiveDate>,
    ) -> Result<CoursMembre, EntDbError> {
        self.exiger_referentiel(a, Action::ReferentielGerer).await?;
        if !NATURES_MEMBRE.contains(&nature)
            || !MOTIFS_MEMBRE.contains(&motif_code)
            || au.is_some_and(|f| f < du)
        {
            return Err(invalide());
        }
        let mut tx = TenantTx::begin(self.pool(), a.etablissement_id).await.map_err(db)?;
        let cours_id = resoudre_cours(&mut tx, a.etablissement_id, cours_id).await?;
        let m: CoursMembre = sqlx::query_as(&format!(
            "INSERT INTO ent_cours_membre
                (id, etablissement_id, cours_id, eleve_id, nature, motif_code, du, au, cree_par)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)
             RETURNING {COLONNES_MEMBRE}"
        ))
        .bind(Uuid::new_v4())
        .bind(a.etablissement_id)
        .bind(cours_id)
        .bind(eleve_id)
        .bind(nature)
        .bind(motif_code)
        .bind(du)
        .bind(au)
        .bind(a.user_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        // Journal : qui, quoi, quand, sans le motif (donnée sensible : il reste dans la ligne).
        journaliser(
            &mut tx,
            a,
            "cours_membre",
            &m.id.to_string(),
            "cree",
            None,
            Some(
                json!({ "cours_id": m.cours_id, "eleve_id": m.eleve_id, "nature": m.nature,
                         "du": m.du, "au": m.au }),
            ),
        )
        .await?;
        // Ni motif ni identité dans l'événement : données sensibles (D-027).
        let ev = publier_ref(
            &mut tx,
            a,
            "referentiel.cours_membre_ajoute",
            m.cours_id,
            json!({ "membre_id": m.id, "nature": m.nature, "du": m.du, "au": m.au }),
        )
        .await?;
        tx.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&[ev]);
        Ok(m)
    }

    /// Annule un membre individuel (jamais supprimé). Idempotent ; inconnu → introuvable.
    pub async fn cours_membre_annuler(
        &self,
        a: &Acteur<'_>,
        membre_id: Uuid,
    ) -> Result<(), EntDbError> {
        self.exiger_referentiel(a, Action::ReferentielGerer).await?;
        let mut tx = TenantTx::begin(self.pool(), a.etablissement_id).await.map_err(db)?;
        let cours: Option<Uuid> = sqlx::query_scalar(
            "UPDATE ent_cours_membre SET annulee_le = coalesce(annulee_le, now()),
                    annulee_par = coalesce(annulee_par, $3)
              WHERE etablissement_id = $1 AND id = $2 RETURNING cours_id",
        )
        .bind(a.etablissement_id)
        .bind(membre_id)
        .bind(a.user_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let Some(cours) = cours else {
            return Err(introuvable());
        };
        journaliser(
            &mut tx,
            a,
            "cours_membre",
            &membre_id.to_string(),
            "annule",
            None,
            Some(json!({ "cours_id": cours })),
        )
        .await?;
        let ev = publier_ref(
            &mut tx,
            a,
            "referentiel.cours_membre_annule",
            cours,
            json!({ "membre_id": membre_id }),
        )
        .await?;
        tx.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&[ev]);
        Ok(())
    }
}
