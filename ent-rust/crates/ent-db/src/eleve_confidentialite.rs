//! Socle 2 (l'élève), lot E7 — confidentialité renforcée (ELV-15) et journal de consultation
//! (§10.5).
//!
//! * **Poser / modifier / lever** (`eleves.confidentiel.gerer`, portée établissement : Admin,
//!   Direction) : une transaction écrit la mesure, le journal (ajout seul) et l'événement
//!   `eleve.confidentialite_changee` (sans motif ni mesures). Le **motif est scellé**
//!   (`champs_chiffres`) ; il n'apparaît ni dans un événement, ni dans un journal, ni dans une
//!   erreur. Aucune décision sur un élève avec lequel l'agent est en conflit d'intérêts (R-E10).
//! * **Effets minimaux non désactivables** (§9.2) : l'élève sort de l'annuaire, des recherches,
//!   des listes et de l'accès direct par identifiant pour **tout tiers** — qui n'a ni G6 ni lien
//!   direct avec lui (portée restreinte qui le contient : « mes classes », groupes, désignés) —
//!   et sa photo est masquée ; la réponse est celle d'un élève absent (`404`), jamais un `403`
//!   qui révélerait la mesure.
//! * **Journal de consultation** : toute lecture de l'état civil, des identifiants, de
//!   l'historique, du motif, toute ouverture de la fiche d'un élève protégé s'écrit dans
//!   `ent_journal_modif` (domaine `consultation`, ajout seul), lisible par `audit.lire`. Quand la
//!   mesure `alerte_consultation` est posée, la consultation d'un tiers publie
//!   `eleve.consultation_protegee` (l'acteur est l'auteur de l'événement).

use std::collections::HashSet;

use chrono::{DateTime, NaiveDate, Utc};
use domain_eleve::{
    normaliser_mesures, valider_motif_confidentialite, valider_motif_levee, valider_periode_mesure,
    DomainError, ErreurProtection, GroupeChamps, PorteeGroupe, ProfilChamps,
};
use ent_auth::{Action, Role};
use ent_events::{types, AggregateKind, MetierEventDraft};
use serde_json::json;
use sqlx::PgConnection;
use uuid::Uuid;

use crate::champs_chiffres::ChampsSensibles;
use crate::outbox::Publier;
use crate::service::{EntDbError, PgEnt};
use crate::tenant_tx::TenantTx;

fn coffre() -> ChampsSensibles {
    ChampsSensibles::depuis_env()
}

const TABLE: &str = "ent_eleve_confidentialite";
const COLONNE_MOTIF: &str = "motif_scelle";

/// Refus d'une règle pure → saisie invalide (messages fixes, sans donnée personnelle).
pub(crate) fn invalide(e: ErreurProtection) -> EntDbError {
    EntDbError::SaisieInvalide(match e {
        ErreurProtection::MesureInconnue => "mesure de confidentialité inconnue",
        ErreurProtection::MotifVide => "le motif est obligatoire",
        ErreurProtection::MotifTropLong => "le motif est trop long",
        ErreurProtection::MotifCaractere => "le motif contient un caractère interdit",
        ErreurProtection::MotifLeveeInconnu => "motif de levée inconnu",
        ErreurProtection::PeriodeInvalide => "période de la mesure invalide",
        ErreurProtection::MotifPhotoInconnu => "motif de photo inconnu",
        ErreurProtection::UsageInconnu => "usage de coordonnée inconnu",
        ErreurProtection::UsageIncompatible => "cet usage n'est pas possible pour ce type de coordonnée",
        ErreurProtection::TypeInconnu => "type de coordonnée inconnu",
        ErreurProtection::MotifCoordonneeInconnu => "motif de coordonnée inconnu",
    })
}

/// Demande de pose (ou de remplacement) d'une mesure.
#[derive(Debug, Clone)]
pub struct PoserConfidentialite {
    pub etablissement_id: Uuid,
    pub eleve_id: Uuid,
    /// Motif en clair : scellé avant toute écriture.
    pub motif: String,
    /// Mesures demandées en plus des effets minimaux (`masquer_photo`, `hors_annuaire`).
    pub mesures: Vec<String>,
    /// Début (défaut : aujourd'hui, jour de l'établissement).
    pub du: Option<NaiveDate>,
    /// Fin prévue (facultative).
    pub au: Option<NaiveDate>,
    pub acteur: Uuid,
}

/// Une mesure telle que la direction la voit. `motif` n'est renseigné que pour G6, descellé.
#[derive(Debug, Clone)]
pub struct MesureConfidentialiteVue {
    pub id: Uuid,
    pub eleve_id: Uuid,
    pub mesures: Vec<String>,
    pub du: NaiveDate,
    pub au: Option<NaiveDate>,
    pub decide_par: Option<Uuid>,
    pub decide_le: DateTime<Utc>,
    pub leve_par: Option<Uuid>,
    pub leve_le: Option<DateTime<Utc>>,
    pub leve_motif_code: Option<String>,
    pub motif: Option<String>,
}

/// Une ligne du journal de consultation.
#[derive(Debug, Clone)]
pub struct ConsultationVue {
    pub id: Uuid,
    pub eleve_id: Option<Uuid>,
    pub acteur: Option<Uuid>,
    pub action: String,
    pub libelle: String,
    pub quand: DateTime<Utc>,
}

#[derive(Debug, sqlx::FromRow)]
struct MesureRow {
    id: Uuid,
    eleve_id: Uuid,
    mesures: Vec<String>,
    du: NaiveDate,
    au: Option<NaiveDate>,
    decide_par: Option<Uuid>,
    decide_le: DateTime<Utc>,
    leve_par: Option<Uuid>,
    leve_le: Option<DateTime<Utc>>,
    leve_motif_code: Option<String>,
    motif_scelle: String,
}

impl MesureRow {
    fn en_vue(self, avec_motif: bool) -> Result<MesureConfidentialiteVue, EntDbError> {
        let motif = if avec_motif {
            Some(coffre().ouvrir(TABLE, COLONNE_MOTIF, self.id, &self.motif_scelle)?)
        } else {
            None
        };
        Ok(MesureConfidentialiteVue {
            id: self.id,
            eleve_id: self.eleve_id,
            mesures: self.mesures,
            du: self.du,
            au: self.au,
            decide_par: self.decide_par,
            decide_le: self.decide_le,
            leve_par: self.leve_par,
            leve_le: self.leve_le,
            leve_motif_code: self.leve_motif_code,
            motif,
        })
    }
}

const COLONNES: &str = "id, eleve_id, mesures, du, au, decide_par, decide_le, leve_par, leve_le, \
                        leve_motif_code, motif_scelle";

/// Écrit une ligne du journal d'audit (ajout seul) dans la transaction du fait.
pub(crate) async fn journaliser_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    eleve: Option<Uuid>,
    acteur: Uuid,
    domaine: &str,
    action: &str,
    libelle: &str,
    role: &str,
    maintenant: DateTime<Utc>,
) -> Result<(), EntDbError> {
    sqlx::query(
        "INSERT INTO ent_journal_modif (id, etablissement_id, eleve_id, domaine, action, libelle, payload,
                                         visible_famille, actor_user_id, created_at)
         VALUES ($1, $2, $3, $4, $5, $6, jsonb_build_object('role', $7::text), false, $8, $9)",
    )
    .bind(Uuid::new_v4())
    .bind(etab)
    .bind(eleve)
    .bind(domaine)
    .bind(action)
    .bind(libelle)
    .bind(role)
    .bind(acteur)
    .bind(maintenant)
    .execute(&mut *conn)
    .await?;
    Ok(())
}

impl PgEnt {
    async fn exiger_droit_confidentialite(&self, etab: Uuid, acteur: Uuid, role: &str) -> Result<(), EntDbError> {
        if self.a_action_etablissement(etab, acteur, role, Action::ElevesConfidentielGerer).await? {
            Ok(())
        } else {
            Err(EntDbError::Forbidden)
        }
    }

    /// Verrouille la fiche (sérialise les commandes sur un même élève). `NotFound` si elle est
    /// absente ou d'un autre établissement ; une fiche absorbée par une fusion est en lecture
    /// seule pour cette commande.
    pub(crate) async fn verrouiller_fiche(
        tx: &mut PgConnection,
        etab: Uuid,
        eleve: Uuid,
    ) -> Result<(), EntDbError> {
        let r: Option<(Option<Uuid>,)> = sqlx::query_as(
            "SELECT fusionne_dans FROM ent_eleve WHERE etablissement_id = $1 AND id = $2 FOR UPDATE",
        )
        .bind(etab)
        .bind(eleve)
        .fetch_optional(&mut *tx)
        .await?;
        match r {
            None => Err(DomainError::NotFound.into()),
            Some((Some(_),)) => Err(EntDbError::ConflitDossier("cette fiche a été fusionnée dans une autre")),
            Some((None,)) => Ok(()),
        }
    }

    async fn jour_a(tx: &mut PgConnection, etab: Uuid, instant: DateTime<Utc>) -> Result<NaiveDate, EntDbError> {
        Ok(sqlx::query_scalar("SELECT ent_aujourdhui_etab($1, $2)")
            .bind(etab)
            .bind(instant)
            .fetch_one(&mut *tx)
            .await?)
    }

    async fn inserer_mesure_tx(
        tx: &mut TenantTx<'static>,
        role: &str,
        cmd: &PoserConfidentialite,
        maintenant: DateTime<Utc>,
        action_journal: &str,
    ) -> Result<MesureConfidentialiteVue, EntDbError> {
        let etab = cmd.etablissement_id;
        let motif = valider_motif_confidentialite(&cmd.motif).map_err(invalide)?;
        let mesures = normaliser_mesures(&cmd.mesures).map_err(invalide)?;
        let jour = Self::jour_a(tx, etab, maintenant).await?;
        let du = cmd.du.unwrap_or(jour);
        valider_periode_mesure(du, cmd.au, jour).map_err(invalide)?;
        let id = Uuid::new_v4();
        let scelle = coffre().sceller(TABLE, COLONNE_MOTIF, id, &motif)?;
        let mesures_texte: Vec<String> = mesures.iter().map(|m| (*m).to_string()).collect();
        sqlx::query(
            "INSERT INTO ent_eleve_confidentialite
                (id, etablissement_id, eleve_id, niveau, motif_scelle, mesures, du, au, decide_par, decide_le)
             VALUES ($1, $2, $3, 'renforcee', $4, $5, $6, $7, $8, $9)",
        )
        .bind(id)
        .bind(etab)
        .bind(cmd.eleve_id)
        .bind(&scelle)
        .bind(&mesures_texte)
        .bind(du)
        .bind(cmd.au)
        .bind(cmd.acteur)
        .bind(maintenant)
        .execute(&mut **tx)
        .await?;
        journaliser_tx(
            tx,
            etab,
            Some(cmd.eleve_id),
            cmd.acteur,
            "eleve_sensible",
            action_journal,
            "Confidentialité renforcée",
            role,
            maintenant,
        )
        .await?;
        Ok(MesureConfidentialiteVue {
            id,
            eleve_id: cmd.eleve_id,
            mesures: mesures_texte,
            du,
            au: cmd.au,
            decide_par: Some(cmd.acteur),
            decide_le: maintenant,
            leve_par: None,
            leve_le: None,
            leve_motif_code: None,
            motif: None,
        })
    }

    /// Lève toutes les mesures ouvertes de l'élève ; renvoie leur nombre.
    async fn lever_mesures_tx(
        tx: &mut TenantTx<'static>,
        etab: Uuid,
        eleve: Uuid,
        acteur: Uuid,
        motif_code: &str,
        maintenant: DateTime<Utc>,
    ) -> Result<u64, EntDbError> {
        let jour = Self::jour_a(tx, etab, maintenant).await?;
        let r = sqlx::query(
            "UPDATE ent_eleve_confidentialite
                SET leve_le = $3, leve_par = $4, leve_motif_code = $5,
                    au = CASE WHEN au IS NULL OR au > $6 THEN GREATEST(du, $6) ELSE au END
              WHERE etablissement_id = $1 AND eleve_id = $2 AND leve_le IS NULL",
        )
        .bind(etab)
        .bind(eleve)
        .bind(maintenant)
        .bind(acteur)
        .bind(motif_code)
        .bind(jour)
        .execute(&mut **tx)
        .await?;
        Ok(r.rows_affected())
    }

    async fn publier_changement_tx(
        tx: &mut TenantTx<'static>,
        etab: Uuid,
        eleve: Uuid,
        acteur: Uuid,
        changement: &str,
        mesure: Option<Uuid>,
    ) -> Result<(), EntDbError> {
        tx.publier(MetierEventDraft {
            etablissement_id: etab,
            type_name: types::ELEVE_CONFIDENTIALITE_CHANGEE.to_string(),
            aggregate: AggregateKind::Eleve,
            aggregate_id: eleve,
            eleve_id: Some(eleve),
            payload: json!({ "changement": changement, "mesure_id": mesure }),
            actor_user_id: Some(acteur.to_string()),
        })
        .await?;
        Ok(())
    }

    /// ELV-15 ★ : pose la confidentialité renforcée.
    pub async fn poser_confidentialite(
        &self,
        role: &str,
        cmd: PoserConfidentialite,
    ) -> Result<MesureConfidentialiteVue, EntDbError> {
        self.poser_confidentialite_a(role, cmd, Utc::now()).await
    }

    /// Même commande à un instant donné (tests : aucune dépendance à l'horloge réelle).
    pub async fn poser_confidentialite_a(
        &self,
        role: &str,
        cmd: PoserConfidentialite,
        maintenant: DateTime<Utc>,
    ) -> Result<MesureConfidentialiteVue, EntDbError> {
        let (etab, eleve, acteur) = (cmd.etablissement_id, cmd.eleve_id, cmd.acteur);
        self.exiger_droit_confidentialite(etab, acteur, role).await?;
        // Validation avant toute écriture (aucune transaction ouverte pour rien).
        valider_motif_confidentialite(&cmd.motif).map_err(invalide)?;
        normaliser_mesures(&cmd.mesures).map_err(invalide)?;
        // R-E10 : pas de décision sensible sur un élève avec lequel l'agent est en conflit.
        self.exiger_pas_de_conflit(etab, acteur, role, eleve).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        Self::verrouiller_fiche(&mut tx, etab, eleve).await?;
        let jour = Self::jour_a(&mut tx, etab, maintenant).await?;
        let (ouvertes,): (i64,) = sqlx::query_as(
            "SELECT count(*) FROM ent_eleve_confidentialite
              WHERE etablissement_id = $1 AND eleve_id = $2 AND leve_le IS NULL
                AND (au IS NULL OR au > $3)",
        )
        .bind(etab)
        .bind(eleve)
        .bind(jour)
        .fetch_one(&mut *tx)
        .await?;
        if ouvertes > 0 {
            return Err(EntDbError::ConflitDossier(
                "une mesure est déjà en vigueur : la modifier ou la lever",
            ));
        }
        let vue = Self::inserer_mesure_tx(&mut tx, role, &cmd, maintenant, "confidentialite_posee").await?;
        Self::publier_changement_tx(&mut tx, etab, eleve, acteur, "pose", Some(vue.id)).await?;
        tx.commit().await?;
        Ok(vue)
    }

    /// ELV-15 ★ : remplace la mesure en vigueur (autres mesures, autre fin, autre motif) : la
    /// précédente est levée (`changement_de_mesure`) et la nouvelle posée dans la même
    /// transaction — l'élève n'est jamais exposé entre les deux.
    pub async fn modifier_confidentialite(
        &self,
        role: &str,
        cmd: PoserConfidentialite,
    ) -> Result<MesureConfidentialiteVue, EntDbError> {
        self.modifier_confidentialite_a(role, cmd, Utc::now()).await
    }

    pub async fn modifier_confidentialite_a(
        &self,
        role: &str,
        cmd: PoserConfidentialite,
        maintenant: DateTime<Utc>,
    ) -> Result<MesureConfidentialiteVue, EntDbError> {
        let (etab, eleve, acteur) = (cmd.etablissement_id, cmd.eleve_id, cmd.acteur);
        self.exiger_droit_confidentialite(etab, acteur, role).await?;
        valider_motif_confidentialite(&cmd.motif).map_err(invalide)?;
        normaliser_mesures(&cmd.mesures).map_err(invalide)?;
        self.exiger_pas_de_conflit(etab, acteur, role, eleve).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        Self::verrouiller_fiche(&mut tx, etab, eleve).await?;
        let levees =
            Self::lever_mesures_tx(&mut tx, etab, eleve, acteur, "changement_de_mesure", maintenant).await?;
        if levees == 0 {
            return Err(DomainError::NotFound.into());
        }
        let vue = Self::inserer_mesure_tx(&mut tx, role, &cmd, maintenant, "confidentialite_modifiee").await?;
        Self::publier_changement_tx(&mut tx, etab, eleve, acteur, "modification", Some(vue.id)).await?;
        tx.commit().await?;
        Ok(vue)
    }

    /// ELV-15 ★ : lève la confidentialité renforcée. `NotFound` si l'élève est absent, d'un autre
    /// établissement, ou n'a aucune mesure ouverte.
    pub async fn lever_confidentialite(
        &self,
        role: &str,
        etab: Uuid,
        acteur: Uuid,
        eleve: Uuid,
        motif_code: &str,
    ) -> Result<u64, EntDbError> {
        self.lever_confidentialite_a(role, etab, acteur, eleve, motif_code, Utc::now()).await
    }

    pub async fn lever_confidentialite_a(
        &self,
        role: &str,
        etab: Uuid,
        acteur: Uuid,
        eleve: Uuid,
        motif_code: &str,
        maintenant: DateTime<Utc>,
    ) -> Result<u64, EntDbError> {
        self.exiger_droit_confidentialite(etab, acteur, role).await?;
        let motif_code = valider_motif_levee(motif_code).map_err(invalide)?;
        self.exiger_pas_de_conflit(etab, acteur, role, eleve).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        Self::verrouiller_fiche(&mut tx, etab, eleve).await?;
        let n = Self::lever_mesures_tx(&mut tx, etab, eleve, acteur, motif_code, maintenant).await?;
        if n == 0 {
            return Err(DomainError::NotFound.into());
        }
        journaliser_tx(
            &mut tx,
            etab,
            Some(eleve),
            acteur,
            "eleve_sensible",
            "confidentialite_levee",
            "Confidentialité renforcée levée",
            role,
            maintenant,
        )
        .await?;
        Self::publier_changement_tx(&mut tx, etab, eleve, acteur, "levee", None).await?;
        tx.commit().await?;
        Ok(n)
    }

    /// Mesures de l'élève (la plus récente d'abord), motif descellé — **G6 seulement** pour cet
    /// élève. Quiconque n'a pas G6 reçoit `NotFound` (aucun oracle sur l'existence d'une mesure) ;
    /// un détenteur de G6 reçoit la liste, vide si l'élève n'a jamais eu de mesure.
    /// La lecture est journalisée.
    pub async fn mesures_confidentialite_pour_acteur(
        &self,
        role: &str,
        etab: Uuid,
        acteur: Uuid,
        eleve: Uuid,
    ) -> Result<Vec<MesureConfidentialiteVue>, EntDbError> {
        self.mesures_confidentialite_pour_acteur_a(role, etab, acteur, eleve, Utc::now()).await
    }

    pub async fn mesures_confidentialite_pour_acteur_a(
        &self,
        role: &str,
        etab: Uuid,
        acteur: Uuid,
        eleve: Uuid,
        maintenant: DateTime<Utc>,
    ) -> Result<Vec<MesureConfidentialiteVue>, EntDbError> {
        if !self.autoriser_eleve(etab, acteur, role, Action::ElevesConfidentielGerer, eleve).await? {
            return Err(DomainError::NotFound.into());
        }
        let sql = format!(
            "SELECT {COLONNES} FROM ent_eleve_confidentialite
              WHERE etablissement_id = $1 AND eleve_id = $2
              ORDER BY (leve_le IS NULL) DESC, decide_le DESC, id"
        );
        let lignes: Vec<MesureRow> = sqlx::query_as(&sql)
            .bind(etab)
            .bind(eleve)
            .fetch_all(&mut *TenantTx::begin(self.pool(), etab).await?)
            .await?;
        // Un élève absent ou d'un autre établissement : `NotFound` (G6 partout ne prouve pas
        // l'existence) ; un élève sans mesure : liste vide.
        let existe: Option<(Uuid,)> = sqlx::query_as("SELECT id FROM ent_eleve WHERE etablissement_id = $1 AND id = $2")
            .bind(etab)
            .bind(eleve)
            .fetch_optional(&mut *TenantTx::begin(self.pool(), etab).await?)
            .await?;
        if existe.is_none() {
            return Err(DomainError::NotFound.into());
        }
        self.tracer_consultation_a(etab, acteur, role, Some(eleve), "sensible", "Consultation de la mesure de confidentialité", false, maintenant)
            .await?;
        // R-E10 : un agent en conflit d'intérêts avec l'élève ne lit pas le motif descellé.
        let avec_motif = self.conflit_interet_eleve(etab, acteur, role, eleve).await?.is_none();
        lignes.into_iter().map(|l| l.en_vue(avec_motif)).collect()
    }

    /// Parmi `ids`, les élèves actuellement protégés (sans considération de lien : sert à filtrer
    /// les journaux et listes qui ne doivent jamais montrer un protégé hors G6).
    pub(crate) async fn eleves_proteges_parmi(&self, etab: Uuid, ids: &[Uuid]) -> Result<HashSet<Uuid>, EntDbError> {
        if ids.is_empty() {
            return Ok(HashSet::new());
        }
        let l: Vec<(Uuid,)> = sqlx::query_as(
            "SELECT e.id FROM ent_eleve e
              WHERE e.etablissement_id = $1 AND e.id = ANY($2) AND ent_eleve_confidentiel($1, e.id)",
        )
        .bind(etab)
        .bind(ids)
        .fetch_all(&mut *TenantTx::begin(self.pool(), etab).await?)
        .await?;
        Ok(l.into_iter().map(|(id,)| id).collect())
    }

    /// Élèves actuellement protégés (sans motif) — G6 partout : la direction y voit ses dossiers.
    pub async fn eleves_proteges_pour_acteur(
        &self,
        role: &str,
        etab: Uuid,
        acteur: Uuid,
        limite: i64,
    ) -> Result<Vec<MesureConfidentialiteVue>, EntDbError> {
        let profil = self.profil_champs(etab, acteur, role).await?;
        if !profil.ouvert_partout(GroupeChamps::Sensible) {
            return Err(EntDbError::Forbidden);
        }
        let sql = format!(
            "SELECT {COLONNES} FROM ent_eleve_confidentialite c
              WHERE etablissement_id = $1 AND leve_le IS NULL
                AND du <= ent_jour_etablissement($1) AND (au IS NULL OR au > ent_jour_etablissement($1))
                AND EXISTS (SELECT 1 FROM ent_eleve e WHERE e.etablissement_id = c.etablissement_id
                                AND e.id = c.eleve_id AND e.fusionne_dans IS NULL)
              ORDER BY decide_le DESC, id LIMIT $2"
        );
        let lignes: Vec<MesureRow> = sqlx::query_as(&sql)
            .bind(etab)
            .bind(limite.clamp(1, 500))
            .fetch_all(&mut *TenantTx::begin(self.pool(), etab).await?)
            .await?;
        // F4 : la liste de tous les élèves protégés est elle-même une lecture sensible, tracée.
        self.tracer_consultation(etab, acteur, role, None, "liste_proteges", "Consultation de la liste des élèves protégés", false)
            .await?;
        lignes.into_iter().map(|l| l.en_vue(false)).collect()
    }

    /// Journal de consultation d'un élève (`audit.lire`, portée établissement), le plus récent
    /// d'abord. Un élève absent ou d'un autre établissement : `NotFound`.
    pub async fn consultations_pour_acteur(
        &self,
        role: &str,
        etab: Uuid,
        acteur: Uuid,
        eleve: Uuid,
        limite: i64,
    ) -> Result<Vec<ConsultationVue>, EntDbError> {
        if !self.a_action_etablissement(etab, acteur, role, Action::AuditLire).await? {
            return Err(EntDbError::Forbidden);
        }
        // E7 : le journal d'un élève protégé n'existe pas pour un tiers (404, comme sa fiche).
        self.exiger_eleve_visible(etab, acteur, role, eleve).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let existe: Option<(Uuid,)> =
            sqlx::query_as("SELECT id FROM ent_eleve WHERE etablissement_id = $1 AND id = $2")
                .bind(etab)
                .bind(eleve)
                .fetch_optional(&mut *tx)
                .await?;
        if existe.is_none() {
            return Err(DomainError::NotFound.into());
        }
        let lignes: Vec<(Uuid, Option<Uuid>, Option<Uuid>, String, String, DateTime<Utc>)> = sqlx::query_as(
            "SELECT id, eleve_id, actor_user_id, action, libelle, created_at FROM ent_journal_modif
              WHERE etablissement_id = $1 AND eleve_id = $2 AND domaine = 'consultation'
              ORDER BY created_at DESC, id LIMIT $3",
        )
        .bind(etab)
        .bind(eleve)
        .bind(limite.clamp(1, 500))
        .fetch_all(&mut *tx)
        .await?;
        Ok(lignes
            .into_iter()
            .map(|(id, eleve_id, acteur, action, libelle, quand)| ConsultationVue {
                id,
                eleve_id,
                acteur,
                action,
                libelle,
                quand,
            })
            .collect())
    }

    // ───────────────────────── Effets sur les lectures ─────────────────────────

    /// Élèves de `ids` qu'un personnel **ne doit pas voir** : protégés, sans G6 ni lien direct.
    /// Pour un rôle hors personnel (famille, élève), rien n'est masqué ici : le lien actif
    /// (socle 4) décide, et la protection vise les tiers, pas les proches.
    pub(crate) async fn proteges_caches(
        &self,
        etab: Uuid,
        role: &str,
        profil: &ProfilChamps,
        ids: &[Uuid],
    ) -> Result<HashSet<Uuid>, EntDbError> {
        if ids.is_empty()
            || !Role::parse(role).is_some_and(Role::is_staff)
            || profil.ouvert_partout(GroupeChamps::Sensible)
        {
            return Ok(HashSet::new());
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        self.proteges_caches_conn(&mut tx, etab, role, profil, ids).await
    }

    /// Même calcul sur la connexion **déjà ouverte** de l'appelant (F7).
    pub(crate) async fn proteges_caches_conn(
        &self,
        conn: &mut PgConnection,
        etab: Uuid,
        role: &str,
        profil: &ProfilChamps,
        ids: &[Uuid],
    ) -> Result<HashSet<Uuid>, EntDbError> {
        if ids.is_empty()
            || !Role::parse(role).is_some_and(Role::is_staff)
            || profil.ouvert_partout(GroupeChamps::Sensible)
        {
            return Ok(HashSet::new());
        }
        let proteges: Vec<(Uuid,)> = sqlx::query_as(
            "SELECT e.id FROM ent_eleve e
              WHERE e.etablissement_id = $1 AND e.id = ANY($2) AND ent_eleve_confidentiel($1, e.id)",
        )
        .bind(etab)
        .bind(ids)
        .fetch_all(&mut *conn)
        .await?;
        let lien_direct = |id: Uuid| {
            [GroupeChamps::Designation, GroupeChamps::Sensible]
                .into_iter()
                .any(|g| matches!(profil.portee(g), PorteeGroupe::Ensemble(s) if s.contains(&id)))
        };
        Ok(proteges.into_iter().map(|(id,)| id).filter(|id| !lien_direct(*id)).collect())
    }

    /// Vrai si la fiche de cet élève est protégée **et** que l'acteur est un tiers (accès direct
    /// par identifiant : la réponse doit être celle d'un élève absent).
    pub async fn eleve_cache_au_tiers(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        eleve: Uuid,
    ) -> Result<bool, EntDbError> {
        if !Role::parse(role).is_some_and(Role::is_staff) {
            return Ok(false);
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        self.eleve_cache_au_tiers_conn(&mut tx, etab, acteur, role, eleve).await
    }

    /// Même réponse, sur la connexion de la transaction **déjà ouverte** de l'appelant : aucune connexion
    /// supplémentaire n'est prise au pool (F7 ; sinon un pool épuisé par des transactions qui s'attendent se bloque).
    /// La connexion doit venir de `TenantTx::begin(.., etab)` du même établissement (contre-revue du pilote, Faible 1).
    pub(crate) async fn eleve_cache_au_tiers_conn(
        &self,
        conn: &mut PgConnection,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        eleve: Uuid,
    ) -> Result<bool, EntDbError> {
        if !Role::parse(role).is_some_and(Role::is_staff) {
            return Ok(false);
        }
        let profil = self.profil_champs_conn(&mut *conn, etab, acteur, role).await?;
        Ok(!self.proteges_caches_conn(&mut *conn, etab, role, &profil, &[eleve]).await?.is_empty())
    }

    /// `NotFound` quand la fiche est cachée à ce tiers (à placer avant toute lecture par
    /// identifiant d'un élève).
    pub(crate) async fn exiger_eleve_visible(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        eleve: Uuid,
    ) -> Result<(), EntDbError> {
        if self.eleve_cache_au_tiers(etab, acteur, role, eleve).await? {
            return Err(DomainError::NotFound.into());
        }
        Ok(())
    }

    /// Comme [`Self::exiger_eleve_visible`], sur la connexion de la transaction **déjà ouverte** de l'appelant (aucune
    /// connexion supplémentaire au pool). Même contrat que `eleve_cache_au_tiers_conn` (connexion du même établissement).
    pub(crate) async fn exiger_eleve_visible_conn(
        &self,
        conn: &mut PgConnection,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        eleve: Uuid,
    ) -> Result<(), EntDbError> {
        if self.eleve_cache_au_tiers_conn(conn, etab, acteur, role, eleve).await? {
            return Err(DomainError::NotFound.into());
        }
        Ok(())
    }

    /// Journal de consultation (ajout seul) : écrit AVANT que la lecture soit servie (une trace
    /// sans lecture reste possible, jamais l'inverse). L'alerte est alignée sur `proteges_caches` :
    /// elle est décidée ICI, pour toute lecture d'un élève protégé par un membre du personnel sans
    /// G6 (tiers ou non-tiers à portée directe, quelle que soit la lecture) ; si la mesure
    /// `alerte_consultation` est en vigueur, l'événement est publié dans la même transaction.
    /// Le paramètre `_alerter` n'est conservé que pour les appelants existants : il est ignoré.
    pub(crate) async fn tracer_consultation(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        eleve: Option<Uuid>,
        action: &str,
        libelle: &str,
        _alerter: bool,
    ) -> Result<(), EntDbError> {
        self.tracer_consultation_a(etab, acteur, role, eleve, action, libelle, false, Utc::now()).await
    }

    pub(crate) async fn tracer_consultation_a(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        eleve: Option<Uuid>,
        action: &str,
        libelle: &str,
        _alerter: bool,
        maintenant: DateTime<Utc>,
    ) -> Result<(), EntDbError> {
        let sans_g6 = Role::parse(role).is_some_and(Role::is_staff)
            && !self
                .profil_champs(etab, acteur, role)
                .await?
                .ouvert_partout(GroupeChamps::Sensible);
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        // Une consultation d'un élève absent de cet établissement (identifiant d'un autre
        // établissement ou inexistant) ne laisse aucune trace : le journal de l'établissement ne
        // doit contenir aucune ligne qui désigne un élève d'ailleurs.
        if let Some(e) = eleve {
            let existe: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM ent_eleve WHERE etablissement_id = $1 AND id = $2)")
                .bind(etab)
                .bind(e)
                .fetch_one(&mut *tx)
                .await?;
            if !existe {
                return Ok(());
            }
        }
        journaliser_tx(&mut tx, etab, eleve, acteur, "consultation", action, libelle, role, maintenant)
            .await
            .inspect_err(|e| {
                tracing::error!(action, error = %e, "échec d'écriture de la trace de consultation : lecture refusée");
            })?;
        if let (true, Some(e)) = (sans_g6, eleve) {
            let jour = Self::jour_a(&mut tx, etab, maintenant).await?;
            let alerte: bool = sqlx::query_scalar(
                "SELECT EXISTS (SELECT 1 FROM ent_eleve_confidentialite c
                                 WHERE c.etablissement_id = $1 AND c.eleve_id = $2 AND c.leve_le IS NULL
                                   AND 'alerte_consultation' = ANY(c.mesures)
                                   AND c.du <= $3 AND (c.au IS NULL OR c.au > $3))",
            )
            .bind(etab)
            .bind(e)
            .bind(jour)
            .fetch_one(&mut *tx)
            .await?;
            if alerte {
                tx.publier(MetierEventDraft {
                    etablissement_id: etab,
                    type_name: types::ELEVE_CONSULTATION_PROTEGEE.to_string(),
                    aggregate: AggregateKind::Eleve,
                    aggregate_id: e,
                    eleve_id: Some(e),
                    payload: json!({ "eleve_id": e, "action": action }),
                    actor_user_id: Some(acteur.to_string()),
                })
                .await?;
            }
        }
        tx.commit().await?;
        Ok(())
    }
}
