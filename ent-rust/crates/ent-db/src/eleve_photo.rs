//! Socle 2 (l'élève), lot E7 — photos de l'élève (ELV-16) et refus de la photo administrative
//! (ELV-42).
//!
//! * **États** : proposée → validée | rejetée ; validée → retirée. Au plus une photo validée par
//!   élève ; `ent_eleve.photo_key` / `photo_id` en sont la projection, tenue par la base (les
//!   lecteurs hérités ne changent pas). Une photo est un **objet immuable** du stockage privé,
//!   jamais servi sans lien court émis après contrôle des droits.
//! * **Qui propose** : le personnel qui détient `eleves.photo.gerer` (sa photo peut être validée du
//!   même geste), la famille (lien actif, élève mineur et non émancipé) et l'élève lui-même
//!   (compte lié). La **source est déduite de l'acteur**, jamais choisie par le client. Une photo de
//!   la famille ou de l'élève attend toujours une validation humaine.
//! * **Qui décide** : `eleves.photo.gerer` (Admin, Direction, Administratif, CPE par défaut).
//! * **Confidentialité renforcée** : un élève protégé est invisible d'un tiers (`404`).
//! * **Refus (ELV-42)** : la famille (ou l'élève, ou le secrétariat) enregistre le refus de la
//!   photo administrative ; la photo validée est retirée, aucune proposition n'est acceptée tant
//!   que le refus est ouvert, l'élève sort de « photos manquantes », sans relance. Révocable.

use chrono::{DateTime, NaiveDate, Utc};
use domain_eleve::{
    est_majeur_le, famille_peut_proposer_coordonnee, plan_poser_photo_eleve, valider_motif_photo, DomainError,
    PoserPhotoEleve, SourcePhoto, StatutPhoto, PHOTOS_EN_ATTENTE_MAX,
};
use ent_auth::{Action, Role};
use ent_events::{types, AggregateKind, MetierEventDraft};
use serde_json::json;
use sqlx::PgConnection;
use uuid::Uuid;

use crate::eleve_confidentialite::{invalide, journaliser_tx};
use crate::outbox::Publier;
use crate::service::{EntDbError, PgEnt};
use crate::tenant_tx::TenantTx;
use crate::upload::UsageFichier;

/// Qui agit sur la fiche : détermine la source d'une proposition et ce qui lui est permis.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum QualiteActeur {
    Personnel,
    Famille,
    Eleve,
}

/// Une photo telle que l'écran la voit. La clé d'objet n'est jamais exposée.
#[derive(Debug, Clone)]
pub struct PhotoVue {
    pub id: Uuid,
    pub eleve_id: Uuid,
    pub source: String,
    pub statut: String,
    pub mime: String,
    pub taille_octets: i32,
    pub prise_le: Option<NaiveDate>,
    pub propose_par: Option<Uuid>,
    pub propose_le: DateTime<Utc>,
    pub decide_par: Option<Uuid>,
    pub decide_le: Option<DateTime<Utc>>,
    pub motif_code: Option<String>,
}

/// Dépôt d'une photo.
#[derive(Debug, Clone)]
pub struct ProposerPhoto {
    pub etablissement_id: Uuid,
    pub eleve_id: Uuid,
    pub acteur: Uuid,
    pub content_type: String,
    pub octets: Vec<u8>,
    pub prise_le: Option<NaiveDate>,
    /// Personnel seulement : valider du même geste (défaut des écrans du secrétariat).
    pub valider_aussitot: bool,
    /// Personnel seulement : `etablissement` (défaut) ou `photographe`.
    pub photographe: bool,
}

#[derive(Debug, sqlx::FromRow)]
struct PhotoRow {
    id: Uuid,
    eleve_id: Uuid,
    source: String,
    statut: String,
    mime: String,
    taille_octets: i32,
    prise_le: Option<NaiveDate>,
    propose_par: Option<Uuid>,
    propose_le: DateTime<Utc>,
    decide_par: Option<Uuid>,
    decide_le: Option<DateTime<Utc>>,
    motif_code: Option<String>,
}

impl From<PhotoRow> for PhotoVue {
    fn from(r: PhotoRow) -> Self {
        Self {
            id: r.id,
            eleve_id: r.eleve_id,
            source: r.source,
            statut: r.statut,
            mime: r.mime,
            taille_octets: r.taille_octets,
            prise_le: r.prise_le,
            propose_par: r.propose_par,
            propose_le: r.propose_le,
            decide_par: r.decide_par,
            decide_le: r.decide_le,
            motif_code: r.motif_code,
        }
    }
}

const COLONNES: &str = "id, eleve_id, source, statut, mime, taille_octets, prise_le, propose_par, propose_le, \
                        decide_par, decide_le, motif_code";

/// L'élève est-il majeur ou émancipé à ce jour ? (la famille ne gère alors plus ses données propres)
pub(crate) async fn majeur_ou_emancipe_tx(
    tx: &mut PgConnection,
    etab: Uuid,
    eleve: Uuid,
    jour: NaiveDate,
) -> Result<bool, EntDbError> {
    let r: Option<(Option<NaiveDate>, Option<String>, Option<String>)> = sqlx::query_as(
        "SELECT date_naissance, date_naissance_precision, date_naissance_statut
           FROM ent_eleve WHERE etablissement_id = $1 AND id = $2",
    )
    .bind(etab)
    .bind(eleve)
    .fetch_optional(&mut *tx)
    .await?;
    let Some((naissance, precision, statut)) = r else {
        return Err(DomainError::NotFound.into());
    };
    let majeur = est_majeur_le(
        naissance,
        precision.as_deref().unwrap_or("jour"),
        statut.as_deref().unwrap_or("declaree"),
        jour,
    )
    .unwrap_or(false);
    let emancipe: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM ent_eleve_capacite
                         WHERE etablissement_id = $1 AND eleve_id = $2 AND nature = 'emancipation'
                           AND levee_le IS NULL AND du <= $3 AND (au IS NULL OR au > $3))",
    )
    .bind(etab)
    .bind(eleve)
    .bind(jour)
    .fetch_one(&mut *tx)
    .await?;
    Ok(!famille_peut_proposer_coordonnee(majeur, emancipe))
}

impl PgEnt {
    /// Qui est l'acteur vis-à-vis de cette fiche ? `Forbidden` : rôle sans droit de gestion ;
    /// `NotFound` : la famille ou l'élève n'est pas lié à cette fiche (aucun oracle).
    pub(crate) async fn qualite_sur_la_fiche(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        eleve: Uuid,
        action_personnel: Action,
    ) -> Result<QualiteActeur, EntDbError> {
        let parsed = Role::parse(role).ok_or(EntDbError::Forbidden)?;
        if parsed.is_staff() {
            if !self.a_action_etablissement(etab, acteur, role, action_personnel).await? {
                return Err(EntDbError::Forbidden);
            }
            self.exiger_eleve_visible(etab, acteur, role, eleve).await?;
            return Ok(QualiteActeur::Personnel);
        }
        match parsed {
            Role::Parent => {
                if self.lien_parent_eleve(etab, acteur, eleve).await?.is_none() {
                    return Err(DomainError::NotFound.into());
                }
                Ok(QualiteActeur::Famille)
            }
            Role::Eleve => {
                let lie: Option<(Uuid,)> = sqlx::query_as(
                    "SELECT eleve_id FROM ent_eleve_compte
                      WHERE etablissement_id = $1 AND eleve_id = $2 AND user_id = $3",
                )
                .bind(etab)
                .bind(eleve)
                .bind(acteur)
                .fetch_optional(&mut *TenantTx::begin(self.pool(), etab).await?)
                .await?;
                if lie.is_none() {
                    return Err(DomainError::NotFound.into());
                }
                Ok(QualiteActeur::Eleve)
            }
            _ => Err(EntDbError::Forbidden),
        }
    }

    /// Qualité pour une **écriture** : le personnel en conflit d'intérêts (R-E10) est refusé, comme
    /// pour toute écriture professionnelle sur ce dossier.
    pub(crate) async fn qualite_ecriture_fiche(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        eleve: Uuid,
        action_personnel: Action,
    ) -> Result<QualiteActeur, EntDbError> {
        let q = self.qualite_sur_la_fiche(etab, acteur, role, eleve, action_personnel).await?;
        if q == QualiteActeur::Personnel {
            self.exiger_pas_de_conflit(etab, acteur, role, eleve).await?;
        }
        Ok(q)
    }

    async fn publier_photo_tx(
        tx: &mut TenantTx<'static>,
        etab: Uuid,
        eleve: Uuid,
        acteur: Uuid,
        photo: Uuid,
        statut: &str,
    ) -> Result<(), EntDbError> {
        tx.publier(MetierEventDraft {
            etablissement_id: etab,
            type_name: types::PHOTO_ELEVE.to_string(),
            aggregate: AggregateKind::Trombi,
            aggregate_id: eleve,
            eleve_id: Some(eleve),
            payload: json!({ "photo_id": photo, "statut": statut }),
            actor_user_id: Some(acteur.to_string()),
        })
        .await?;
        Ok(())
    }

    /// Passe la photo `photo` à `validee` ; l'éventuelle photo validée précédente est retirée
    /// (`remplacee`) dans le même mouvement.
    async fn valider_tx(
        tx: &mut TenantTx<'static>,
        etab: Uuid,
        eleve: Uuid,
        photo: Uuid,
        acteur: Uuid,
        maintenant: DateTime<Utc>,
    ) -> Result<(), EntDbError> {
        sqlx::query(
            "UPDATE ent_eleve_photo SET statut = 'retiree', decide_par = $3, decide_le = $4, motif_code = 'remplacee'
              WHERE etablissement_id = $1 AND eleve_id = $2 AND statut = 'validee'",
        )
        .bind(etab)
        .bind(eleve)
        .bind(acteur)
        .bind(maintenant)
        .execute(&mut **tx)
        .await?;
        let r = sqlx::query(
            "UPDATE ent_eleve_photo SET statut = 'validee', decide_par = $3, decide_le = $4
              WHERE etablissement_id = $1 AND id = $2 AND statut = 'proposee'",
        )
        .bind(etab)
        .bind(photo)
        .bind(acteur)
        .bind(maintenant)
        .execute(&mut **tx)
        .await?;
        if r.rows_affected() != 1 {
            return Err(EntDbError::ConflitDossier("cette photo n'est plus en attente de décision"));
        }
        Ok(())
    }

    async fn refus_ouvert_tx(tx: &mut PgConnection, etab: Uuid, eleve: Uuid) -> Result<bool, EntDbError> {
        Ok(sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM ent_eleve_photo_refus
                             WHERE etablissement_id = $1 AND eleve_id = $2 AND leve_le IS NULL)",
        )
        .bind(etab)
        .bind(eleve)
        .fetch_one(&mut *tx)
        .await?)
    }

    /// ELV-16 : propose (et, pour le personnel, valide aussitôt) une photo.
    pub async fn proposer_photo(&self, role: &str, cmd: ProposerPhoto) -> Result<PhotoVue, EntDbError> {
        self.proposer_photo_a(role, cmd, Utc::now()).await
    }

    pub async fn proposer_photo_a(
        &self,
        role: &str,
        cmd: ProposerPhoto,
        maintenant: DateTime<Utc>,
    ) -> Result<PhotoVue, EntDbError> {
        let (etab, eleve, acteur) = (cmd.etablissement_id, cmd.eleve_id, cmd.acteur);
        let qualite = self.qualite_ecriture_fiche(etab, acteur, role, eleve, Action::ElevesPhotoGerer).await?;
        // D2 : signature binaire, liste blanche « photo », taille maximale configurable.
        let valide = self
            .politique_fichiers()
            .valider(UsageFichier::Photo, "photo", Some(&cmd.content_type), &cmd.octets)
            .map_err(EntDbError::Fichier)?;
        let ext = plan_poser_photo_eleve(PoserPhotoEleve {
            etablissement_id: etab,
            eleve_id: eleve,
            content_type: valide.mime.to_string(),
            bytes_len: cmd.octets.len(),
            actor_user_id: acteur,
        })?;
        let source = match qualite {
            QualiteActeur::Personnel if cmd.photographe => SourcePhoto::Photographe,
            QualiteActeur::Personnel => SourcePhoto::Etablissement,
            QualiteActeur::Famille => SourcePhoto::Famille,
            QualiteActeur::Eleve => SourcePhoto::Eleve,
        };
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        Self::verrouiller_fiche(&mut tx, etab, eleve).await?;
        let jour: NaiveDate = sqlx::query_scalar("SELECT ent_aujourdhui_etab($1, $2)")
            .bind(etab)
            .bind(maintenant)
            .fetch_one(&mut *tx)
            .await?;
        if qualite == QualiteActeur::Famille && majeur_ou_emancipe_tx(&mut tx, etab, eleve, jour).await? {
            // Une famille ne gère plus la photo d'un majeur ou d'un émancipé : elle n'a plus rien à y voir.
            return Err(DomainError::NotFound.into());
        }
        if qualite == QualiteActeur::Famille {
            // Réglage « photo » du catalogue (E9) : l'établissement peut fermer la proposition par la famille.
            crate::param_eleve::exiger_saisie_famille_tx(&mut tx, etab, "photo").await?;
        }
        if Self::refus_ouvert_tx(&mut tx, etab, eleve).await? {
            return Err(EntDbError::ConflitDossier(
                "la photo administrative est refusée : lever le refus avant d'en proposer une",
            ));
        }
        let (en_attente,): (i64,) = sqlx::query_as(
            "SELECT count(*) FROM ent_eleve_photo
              WHERE etablissement_id = $1 AND eleve_id = $2 AND statut = 'proposee'",
        )
        .bind(etab)
        .bind(eleve)
        .fetch_one(&mut *tx)
        .await?;
        if en_attente >= PHOTOS_EN_ATTENTE_MAX {
            return Err(EntDbError::ConflitDossier("trop de photos en attente : en décider d'abord"));
        }
        // F2 : l'objet est réécrit SANS métadonnées (EXIF, GPS, commentaires), pixels intacts.
        let propre = crate::photo_nettoyage::sans_metadonnees(valide.mime, &cmd.octets)
            .map_err(EntDbError::SaisieInvalide)?;
        // Objet immuable : une nouvelle photo = un nouvel objet `{etab}/trombi/{eleve}/{uuid}.{ext}`.
        let id = Uuid::new_v4();
        let cle = format!("{etab}/trombi/{eleve}/{id}.{}", ent_object::sanitize_ext(&ext));
        self.stocker_objet(etab, &cle, Some(valide.mime), &propre).await?;
        // F2 : si la transaction échoue APRÈS l'écriture de l'objet, l'objet n'est référencé par rien :
        // il est retiré (sauf si la ligne existe, cas d'un commit dont la réponse s'est perdue).
        let suite = async {
            sqlx::query(
                "INSERT INTO ent_eleve_photo (id, etablissement_id, eleve_id, object_key, mime, taille_octets,
                                              prise_le, source, statut, propose_par, propose_le)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 'proposee', $9, $10)",
            )
            .bind(id)
            .bind(etab)
            .bind(eleve)
            .bind(&cle)
            .bind(valide.mime)
            .bind(i32::try_from(propre.len()).map_err(|_| EntDbError::SaisieInvalide("photo trop volumineuse"))?)
            .bind(cmd.prise_le)
            .bind(source.code())
            .bind(acteur)
            .bind(maintenant)
            .execute(&mut *tx)
            .await?;
            let mut statut = StatutPhoto::Proposee;
            if qualite == QualiteActeur::Personnel && cmd.valider_aussitot && !source.exige_validation() {
                Self::valider_tx(&mut tx, etab, eleve, id, acteur, maintenant).await?;
                statut = StatutPhoto::Validee;
            }
            journaliser_tx(&mut tx, etab, Some(eleve), acteur, "eleve_photo", "photo_proposee", "Photo proposée", role, maintenant)
                .await?;
            if statut == StatutPhoto::Validee {
                journaliser_tx(&mut tx, etab, Some(eleve), acteur, "eleve_photo", "photo_validee", "Photo validée", role, maintenant)
                    .await?;
            }
            Self::publier_photo_tx(&mut tx, etab, eleve, acteur, id, statut.code()).await?;
            let vue = Self::photo_tx(&mut tx, etab, id).await?;
            tx.commit().await?;
            Ok::<PhotoVue, EntDbError>(vue)
        }
        .await;
        match suite {
            Ok(vue) => Ok(vue),
            Err(e) => {
                self.retirer_objet_orphelin(etab, id, &cle).await;
                Err(e)
            }
        }
    }

    /// F2 : retire l'objet d'une photo dont l'enregistrement a échoué. Au mieux : une erreur ici est
    /// journalisée, jamais substituée à l'erreur d'origine. Si la ligne de la photo existe (commit dont
    /// la réponse s'est perdue), l'objet est conservé.
    async fn retirer_objet_orphelin(&self, etab: Uuid, photo: Uuid, cle: &str) {
        let existe = async {
            let mut tx = TenantTx::begin(self.pool(), etab).await?;
            let n: i64 = sqlx::query_scalar("SELECT count(*) FROM ent_eleve_photo WHERE etablissement_id = $1 AND id = $2")
                .bind(etab)
                .bind(photo)
                .fetch_one(&mut *tx)
                .await?;
            Ok::<bool, EntDbError>(n > 0)
        }
        .await;
        if existe.unwrap_or(true) {
            return;
        }
        if let Err(e) = self.objet_supprimer(cle).await {
            tracing::warn!(erreur = %e, "photo orpheline non retirée du stockage");
        }
        let registre = async {
            let mut tx = TenantTx::begin(self.pool(), etab).await?;
            // Le registre est immuable : la ligne est marquée supprimée, jamais effacée.
            sqlx::query("UPDATE ent_objet SET supprime_le = now() WHERE etablissement_id = $1 AND cle_objet = $2 AND supprime_le IS NULL")
                .bind(etab)
                .bind(cle)
                .execute(&mut *tx)
                .await?;
            tx.commit().await?;
            Ok::<(), EntDbError>(())
        }
        .await;
        if let Err(e) = registre {
            tracing::warn!(erreur = %e, "photo orpheline : ligne du registre des objets conservée");
        }
    }

    async fn photo_tx(tx: &mut PgConnection, etab: Uuid, id: Uuid) -> Result<PhotoVue, EntDbError> {
        let sql = format!("SELECT {COLONNES} FROM ent_eleve_photo WHERE etablissement_id = $1 AND id = $2");
        let r: Option<PhotoRow> = sqlx::query_as(&sql).bind(etab).bind(id).fetch_optional(&mut *tx).await?;
        r.map(PhotoVue::from).ok_or_else(|| DomainError::NotFound.into())
    }

    /// Verrouille la photo (et sa fiche) pour une décision ; `NotFound` si elle est absente ou
    /// d'un autre établissement, ou si l'élève est caché à ce tiers.
    async fn photo_a_decider(
        &self,
        tx: &mut TenantTx<'static>,
        role: &str,
        etab: Uuid,
        acteur: Uuid,
        photo: Uuid,
    ) -> Result<PhotoVue, EntDbError> {
        let sql = format!("SELECT {COLONNES} FROM ent_eleve_photo WHERE etablissement_id = $1 AND id = $2 FOR UPDATE");
        let r: Option<PhotoRow> = sqlx::query_as(&sql).bind(etab).bind(photo).fetch_optional(&mut **tx).await?;
        let vue: PhotoVue = r.ok_or(DomainError::NotFound)?.into();
        self.exiger_eleve_visible(etab, acteur, role, vue.eleve_id).await?;
        self.exiger_pas_de_conflit(etab, acteur, role, vue.eleve_id).await?;
        Ok(vue)
    }

    async fn exiger_gestion_photo(&self, etab: Uuid, acteur: Uuid, role: &str) -> Result<(), EntDbError> {
        if self.a_action_etablissement(etab, acteur, role, Action::ElevesPhotoGerer).await? {
            Ok(())
        } else {
            Err(EntDbError::Forbidden)
        }
    }

    /// ELV-16 : valide une photo proposée (la photo validée précédente est retirée).
    pub async fn valider_photo(&self, role: &str, etab: Uuid, acteur: Uuid, photo: Uuid) -> Result<PhotoVue, EntDbError> {
        self.valider_photo_a(role, etab, acteur, photo, Utc::now()).await
    }

    pub async fn valider_photo_a(
        &self,
        role: &str,
        etab: Uuid,
        acteur: Uuid,
        photo: Uuid,
        maintenant: DateTime<Utc>,
    ) -> Result<PhotoVue, EntDbError> {
        self.exiger_gestion_photo(etab, acteur, role).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let p = self.photo_a_decider(&mut tx, role, etab, acteur, photo).await?;
        if !StatutPhoto::parse(&p.statut).is_some_and(|s| s.peut_passer_a(StatutPhoto::Validee)) {
            return Err(EntDbError::ConflitDossier("cette photo n'est plus en attente de décision"));
        }
        if Self::refus_ouvert_tx(&mut tx, etab, p.eleve_id).await? {
            return Err(EntDbError::ConflitDossier("la photo administrative est refusée : lever le refus d'abord"));
        }
        Self::valider_tx(&mut tx, etab, p.eleve_id, photo, acteur, maintenant).await?;
        journaliser_tx(&mut tx, etab, Some(p.eleve_id), acteur, "eleve_photo", "photo_validee", "Photo validée", role, maintenant)
            .await?;
        Self::publier_photo_tx(&mut tx, etab, p.eleve_id, acteur, photo, "validee").await?;
        let vue = Self::photo_tx(&mut tx, etab, photo).await?;
        tx.commit().await?;
        Ok(vue)
    }

    /// ELV-16 : refuse une photo proposée (motif codé obligatoire).
    pub async fn rejeter_photo(
        &self,
        role: &str,
        etab: Uuid,
        acteur: Uuid,
        photo: Uuid,
        motif_code: &str,
    ) -> Result<PhotoVue, EntDbError> {
        self.decider_sans_validation(role, etab, acteur, photo, motif_code, StatutPhoto::Rejetee, Utc::now()).await
    }

    /// ELV-16 : retire la photo validée (motif codé obligatoire) ; la fiche n'a plus de photo.
    pub async fn retirer_photo(
        &self,
        role: &str,
        etab: Uuid,
        acteur: Uuid,
        photo: Uuid,
        motif_code: &str,
    ) -> Result<PhotoVue, EntDbError> {
        self.decider_sans_validation(role, etab, acteur, photo, motif_code, StatutPhoto::Retiree, Utc::now()).await
    }

    pub async fn decider_sans_validation(
        &self,
        role: &str,
        etab: Uuid,
        acteur: Uuid,
        photo: Uuid,
        motif_code: &str,
        cible: StatutPhoto,
        maintenant: DateTime<Utc>,
    ) -> Result<PhotoVue, EntDbError> {
        self.exiger_gestion_photo(etab, acteur, role).await?;
        let motif = valider_motif_photo(motif_code).map_err(invalide)?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let p = self.photo_a_decider(&mut tx, role, etab, acteur, photo).await?;
        if !StatutPhoto::parse(&p.statut).is_some_and(|s| s.peut_passer_a(cible)) {
            return Err(EntDbError::ConflitDossier("cette photo n'est pas dans l'état attendu"));
        }
        sqlx::query(
            "UPDATE ent_eleve_photo SET statut = $3, decide_par = $4, decide_le = $5, motif_code = $6
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(photo)
        .bind(cible.code())
        .bind(acteur)
        .bind(maintenant)
        .bind(motif)
        .execute(&mut *tx)
        .await?;
        let (action, libelle) = if cible == StatutPhoto::Rejetee {
            ("photo_rejetee", "Photo refusée")
        } else {
            ("photo_retiree", "Photo retirée")
        };
        journaliser_tx(&mut tx, etab, Some(p.eleve_id), acteur, "eleve_photo", action, libelle, role, maintenant).await?;
        Self::publier_photo_tx(&mut tx, etab, p.eleve_id, acteur, photo, cible.code()).await?;
        let vue = Self::photo_tx(&mut tx, etab, photo).await?;
        tx.commit().await?;
        Ok(vue)
    }

    /// Photos d'un élève (historique, la plus récente d'abord) : `eleves.photo.gerer`.
    pub async fn photos_eleve_pour_acteur(
        &self,
        role: &str,
        etab: Uuid,
        acteur: Uuid,
        eleve: Uuid,
    ) -> Result<Vec<PhotoVue>, EntDbError> {
        self.exiger_gestion_photo(etab, acteur, role).await?;
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
        let sql = format!(
            "SELECT {COLONNES} FROM ent_eleve_photo WHERE etablissement_id = $1 AND eleve_id = $2
              ORDER BY propose_le DESC, id"
        );
        let l: Vec<PhotoRow> = sqlx::query_as(&sql).bind(etab).bind(eleve).fetch_all(&mut *tx).await?;
        Ok(l.into_iter().map(PhotoVue::from).collect())
    }

    /// File des photos à valider (les plus anciennes d'abord), élèves protégés exclus pour qui
    /// n'y a pas droit.
    pub async fn photos_a_valider_pour_acteur(
        &self,
        role: &str,
        etab: Uuid,
        acteur: Uuid,
        limite: i64,
    ) -> Result<Vec<PhotoVue>, EntDbError> {
        self.exiger_gestion_photo(etab, acteur, role).await?;
        let profil = self.profil_champs(etab, acteur, role).await?;
        // Les protégés sont écartés en SQL AVANT la limite.
        let sql = format!(
            "SELECT {COLONNES} FROM ent_eleve_photo p WHERE etablissement_id = $1 AND statut = 'proposee'
                AND ($3 OR NOT ent_eleve_confidentiel($1, p.eleve_id))
              ORDER BY propose_le, id LIMIT $2"
        );
        let l: Vec<PhotoRow> = sqlx::query_as(&sql)
            .bind(etab)
            .bind(limite.clamp(1, 500))
            .bind(profil.ouvert_partout(domain_eleve::GroupeChamps::Sensible))
            .fetch_all(&mut *TenantTx::begin(self.pool(), etab).await?)
            .await?;
        Ok(l.into_iter().map(PhotoVue::from).collect())
    }

    /// ELV-42 : enregistre le refus de la photo administrative (la photo validée est retirée).
    pub async fn enregistrer_refus_photo(
        &self,
        role: &str,
        etab: Uuid,
        acteur: Uuid,
        eleve: Uuid,
    ) -> Result<(), EntDbError> {
        self.enregistrer_refus_photo_a(role, etab, acteur, eleve, Utc::now()).await
    }

    pub async fn enregistrer_refus_photo_a(
        &self,
        role: &str,
        etab: Uuid,
        acteur: Uuid,
        eleve: Uuid,
        maintenant: DateTime<Utc>,
    ) -> Result<(), EntDbError> {
        let qualite = self.qualite_ecriture_fiche(etab, acteur, role, eleve, Action::ElevesPhotoGerer).await?;
        let source = match qualite {
            QualiteActeur::Personnel => "etablissement",
            QualiteActeur::Famille => "famille",
            QualiteActeur::Eleve => "eleve",
        };
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        Self::verrouiller_fiche(&mut tx, etab, eleve).await?;
        if Self::refus_ouvert_tx(&mut tx, etab, eleve).await? {
            return Err(EntDbError::ConflitDossier("le refus de la photo est déjà enregistré"));
        }
        let jour: NaiveDate = sqlx::query_scalar("SELECT ent_aujourdhui_etab($1, $2)")
            .bind(etab)
            .bind(maintenant)
            .fetch_one(&mut *tx)
            .await?;
        // M1 : la famille n'a plus rien à dire sur la photo d'un majeur ou d'un émancipé (404).
        if qualite == QualiteActeur::Famille && majeur_ou_emancipe_tx(&mut tx, etab, eleve, jour).await? {
            return Err(DomainError::NotFound.into());
        }
        sqlx::query(
            "INSERT INTO ent_eleve_photo_refus (id, etablissement_id, eleve_id, source, du, enregistre_par, enregistre_le)
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(Uuid::new_v4())
        .bind(etab)
        .bind(eleve)
        .bind(source)
        .bind(jour)
        .bind(acteur)
        .bind(maintenant)
        .execute(&mut *tx)
        .await?;
        // La photo courante est retirée, les propositions en attente sont rejetées : le refus vaut
        // pour toutes les photos.
        sqlx::query(
            "UPDATE ent_eleve_photo SET statut = 'retiree', decide_par = $3, decide_le = $4, motif_code = 'refus_famille'
              WHERE etablissement_id = $1 AND eleve_id = $2 AND statut = 'validee'",
        )
        .bind(etab)
        .bind(eleve)
        .bind(acteur)
        .bind(maintenant)
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            "UPDATE ent_eleve_photo SET statut = 'rejetee', decide_par = $3, decide_le = $4, motif_code = 'refus_famille'
              WHERE etablissement_id = $1 AND eleve_id = $2 AND statut = 'proposee'",
        )
        .bind(etab)
        .bind(eleve)
        .bind(acteur)
        .bind(maintenant)
        .execute(&mut *tx)
        .await?;
        journaliser_tx(&mut tx, etab, Some(eleve), acteur, "eleve_photo", "photo_refus_enregistre", "Refus de la photo administrative", role, maintenant)
            .await?;
        Self::publier_photo_tx(&mut tx, etab, eleve, acteur, Uuid::nil(), "refusee").await?;
        tx.commit().await?;
        Ok(())
    }

    /// ELV-42 : lève le refus (révocable par la famille, l'élève ou le secrétariat).
    pub async fn lever_refus_photo(&self, role: &str, etab: Uuid, acteur: Uuid, eleve: Uuid) -> Result<(), EntDbError> {
        self.lever_refus_photo_a(role, etab, acteur, eleve, Utc::now()).await
    }

    pub async fn lever_refus_photo_a(
        &self,
        role: &str,
        etab: Uuid,
        acteur: Uuid,
        eleve: Uuid,
        maintenant: DateTime<Utc>,
    ) -> Result<(), EntDbError> {
        let qualite = self.qualite_ecriture_fiche(etab, acteur, role, eleve, Action::ElevesPhotoGerer).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        Self::verrouiller_fiche(&mut tx, etab, eleve).await?;
        let jour: NaiveDate = sqlx::query_scalar("SELECT ent_aujourdhui_etab($1, $2)")
            .bind(etab)
            .bind(maintenant)
            .fetch_one(&mut *tx)
            .await?;
        if qualite == QualiteActeur::Famille && majeur_ou_emancipe_tx(&mut tx, etab, eleve, jour).await? {
            return Err(DomainError::NotFound.into());
        }
        // M1 : seul le personnel, ou la même source que celle qui a posé le refus, peut le lever.
        let source_requise = match qualite {
            QualiteActeur::Personnel => None,
            QualiteActeur::Famille => Some("famille"),
            QualiteActeur::Eleve => Some("eleve"),
        };
        let r = sqlx::query(
            "UPDATE ent_eleve_photo_refus SET leve_le = $3, leve_par = $4
              WHERE etablissement_id = $1 AND eleve_id = $2 AND leve_le IS NULL
                AND ($5::text IS NULL OR source = $5)",
        )
        .bind(etab)
        .bind(eleve)
        .bind(maintenant)
        .bind(acteur)
        .bind(source_requise)
        .execute(&mut *tx)
        .await?;
        if r.rows_affected() == 0 {
            return Err(DomainError::NotFound.into());
        }
        journaliser_tx(&mut tx, etab, Some(eleve), acteur, "eleve_photo", "photo_refus_leve", "Refus de la photo administrative levé", role, maintenant)
            .await?;
        Self::publier_photo_tx(&mut tx, etab, eleve, acteur, Uuid::nil(), "refus_leve").await?;
        tx.commit().await?;
        Ok(())
    }

    /// Élèves de `ids` qui ont refusé la photo administrative (silhouette dans les listes).
    pub(crate) async fn eleves_refus_photo(&self, etab: Uuid, ids: &[Uuid]) -> Result<Vec<Uuid>, EntDbError> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let l: Vec<(Uuid,)> = sqlx::query_as(
            "SELECT eleve_id FROM ent_eleve_photo_refus
              WHERE etablissement_id = $1 AND eleve_id = ANY($2) AND leve_le IS NULL",
        )
        .bind(etab)
        .bind(ids)
        .fetch_all(&mut *TenantTx::begin(self.pool(), etab).await?)
        .await?;
        Ok(l.into_iter().map(|(id,)| id).collect())
    }

    /// « Photos manquantes » (qualité des données) : élèves scolarisés sans photo validée et sans
    /// refus ouvert. Réservé à `eleves.photo.gerer` ; les élèves protégés sont exclus pour qui
    /// n'y a pas droit.
    pub async fn eleves_sans_photo_pour_acteur(
        &self,
        role: &str,
        etab: Uuid,
        acteur: Uuid,
        limite: i64,
    ) -> Result<Vec<Uuid>, EntDbError> {
        self.exiger_gestion_photo(etab, acteur, role).await?;
        let voit_proteges = self
            .profil_champs(etab, acteur, role)
            .await?
            .ouvert_partout(domain_eleve::GroupeChamps::Sensible);
        let l: Vec<(Uuid,)> = sqlx::query_as(
            "SELECT e.id FROM ent_eleve e
              WHERE e.etablissement_id = $1 AND e.fusionne_dans IS NULL
                AND coalesce(e.statut, 'a_venir') IN ('scolarise', 'a_venir')
                AND e.photo_id IS NULL
                AND ($3 OR NOT ent_eleve_confidentiel(e.etablissement_id, e.id))
                AND NOT EXISTS (SELECT 1 FROM ent_eleve_photo_refus r
                                 WHERE r.etablissement_id = e.etablissement_id AND r.eleve_id = e.id AND r.leve_le IS NULL)
              ORDER BY e.id LIMIT $2",
        )
        .bind(etab)
        .bind(limite.clamp(1, 2000))
        .bind(voit_proteges)
        .fetch_all(&mut *TenantTx::begin(self.pool(), etab).await?)
        .await?;
        Ok(l.into_iter().map(|(id,)| id).collect())
    }

    /// Élèves de `ids` à **exclure d'un export ou d'un trombinoscope imprimé** : mesure
    /// `hors_trombinoscope_exporte` en vigueur. À appeler par tout export nominatif (E11), quel que
    /// soit le droit de l'exportateur.
    pub(crate) async fn eleves_hors_export(&self, etab: Uuid, ids: &[Uuid]) -> Result<Vec<Uuid>, EntDbError> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let l: Vec<(Uuid,)> = sqlx::query_as(
            "SELECT DISTINCT c.eleve_id FROM ent_eleve_confidentialite c
              WHERE c.etablissement_id = $1 AND c.eleve_id = ANY($2) AND c.leve_le IS NULL
                AND 'hors_trombinoscope_exporte' = ANY(c.mesures)
                AND c.du <= ent_jour_etablissement($1) AND (c.au IS NULL OR c.au > ent_jour_etablissement($1))",
        )
        .bind(etab)
        .bind(ids)
        .fetch_all(&mut *TenantTx::begin(self.pool(), etab).await?)
        .await?;
        Ok(l.into_iter().map(|(id,)| id).collect())
    }
}
