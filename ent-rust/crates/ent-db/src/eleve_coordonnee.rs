//! Socle 2 (l'élève), lot E7 — coordonnées **propres** de l'élève (ELV-17) : son e-mail et son
//! mobile, jamais ceux de ses responsables (socle 4).
//!
//! * **Qui** : le personnel (`eleves.identite.modifier`) ajoute, valide, retire ; l'élève gère les
//!   siennes (compte lié) ; la famille **propose** (lien actif, élève mineur et non émancipé) et
//!   le secrétariat décide. Une famille n'a plus aucune prise sur ces coordonnées dès la majorité
//!   ou l'émancipation (la réponse est celle d'une cible absente).
//! * **États** : proposée → active | refusée ; active → retirée. La valeur ne change jamais :
//!   remplacer = retirer puis ajouter ; chaque ligne reste (historique).
//! * **Vérification par code** (e-mail et SMS, comme le socle 4) : seule l'empreinte HMAC du code
//!   est stockée, cinq essais, quinze minutes, trois codes par heure. Elle se fait par l'élève (ou
//!   le personnel, qui lit le code à la personne) : la famille n'a pas la preuve de contrôle.
//!   Une coordonnée non vérifiée n'est utilisable pour aucun envoi sensible.
//! * **Aucune valeur** ni dans l'événement `eleve.coordonnees_modifiees` (type, état, identifiant)
//!   ni dans le journal.

use chrono::{DateTime, Duration, NaiveDate, Utc};
use domain_eleve::{
    normaliser_usages, valider_motif_coordonnee, DomainError, TypeCoordonneeEleve,
    COORDONNEES_VIVANTES_MAX_PAR_TYPE,
};
use ent_auth::Action;
use ent_events::{types, AggregateKind, MetierEventDraft};
use ent_mail::OutgoingMail;
use ent_sms::OutgoingSms;
use serde_json::json;
use sqlx::PgConnection;
use uuid::Uuid;

use crate::dossier_personnes::{
    classer_telephone, code_six_chiffres, empreinte_verification, normaliser_email, normaliser_telephone,
    VERIFICATION_DEMANDES_PAR_HEURE, VERIFICATION_ESSAIS_MAX, VERIFICATION_TTL_MINUTES,
};
use crate::eleve_confidentialite::{invalide, journaliser_tx};
use crate::eleve_photo::{majeur_ou_emancipe_tx, QualiteActeur};
use crate::outbox::Publier;
use crate::service::{EntDbError, PgEnt};
use crate::tenant_tx::TenantTx;

/// Une coordonnée de l'élève.
#[derive(Debug, Clone)]
pub struct CoordonneeEleveVue {
    pub id: Uuid,
    pub eleve_id: Uuid,
    pub type_: String,
    pub valeur: String,
    pub usages: Vec<String>,
    pub source: String,
    pub statut: String,
    pub verifiee_le: Option<DateTime<Utc>>,
    pub du: NaiveDate,
    pub au: Option<NaiveDate>,
    pub cree_par: Option<Uuid>,
    pub cree_le: DateTime<Utc>,
    pub decide_par: Option<Uuid>,
    pub decide_le: Option<DateTime<Utc>>,
    pub motif_code: Option<String>,
}

/// Ajout (ou proposition) d'une coordonnée.
#[derive(Debug, Clone)]
pub struct AjouterCoordonneeEleve {
    pub etablissement_id: Uuid,
    pub eleve_id: Uuid,
    pub acteur: Uuid,
    /// `email` ou `mobile`.
    pub type_: String,
    pub valeur: String,
    pub usages: Vec<String>,
}

/// Code envoyé : ce que l'écran peut dire (jamais le code).
#[derive(Debug, Clone)]
pub struct VerificationEleveEnvoyee {
    pub coordonnee_id: Uuid,
    pub canal: &'static str,
    pub expire_le: DateTime<Utc>,
}

#[derive(Debug, sqlx::FromRow)]
struct CoordRow {
    id: Uuid,
    eleve_id: Uuid,
    #[sqlx(rename = "type")]
    type_: String,
    valeur: String,
    usages: Vec<String>,
    source: String,
    statut: String,
    verifiee_le: Option<DateTime<Utc>>,
    du: NaiveDate,
    au: Option<NaiveDate>,
    cree_par: Option<Uuid>,
    cree_le: DateTime<Utc>,
    decide_par: Option<Uuid>,
    decide_le: Option<DateTime<Utc>>,
    motif_code: Option<String>,
}

impl From<CoordRow> for CoordonneeEleveVue {
    fn from(r: CoordRow) -> Self {
        Self {
            id: r.id,
            eleve_id: r.eleve_id,
            type_: r.type_,
            valeur: r.valeur,
            usages: r.usages,
            source: r.source,
            statut: r.statut,
            verifiee_le: r.verifiee_le,
            du: r.du,
            au: r.au,
            cree_par: r.cree_par,
            cree_le: r.cree_le,
            decide_par: r.decide_par,
            decide_le: r.decide_le,
            motif_code: r.motif_code,
        }
    }
}

const COLONNES: &str = "id, eleve_id, type, valeur, usages, source, statut, verifiee_le, du, au, cree_par, \
                        cree_le, decide_par, decide_le, motif_code";

fn refus_unicite(e: sqlx::Error) -> EntDbError {
    if let sqlx::Error::Database(d) = &e {
        if d.code().as_deref() == Some("23505") {
            return EntDbError::ConflitDossier("cette coordonnée est déjà enregistrée pour cet élève");
        }
    }
    EntDbError::Sqlx(e)
}

impl PgEnt {
    /// Qui agit : personnel (`eleves.identite.modifier`), élève (soi) ou famille (lien actif).
    async fn qualite_coordonnees(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        eleve: Uuid,
    ) -> Result<QualiteActeur, EntDbError> {
        self.qualite_sur_la_fiche(etab, acteur, role, eleve, Action::ElevesIdentiteModifier).await
    }

    async fn publier_coordonnee_tx(
        tx: &mut TenantTx<'static>,
        etab: Uuid,
        eleve: Uuid,
        acteur: Uuid,
        coordonnee: Uuid,
        type_: &str,
        etat: &str,
    ) -> Result<(), EntDbError> {
        tx.publier(MetierEventDraft {
            etablissement_id: etab,
            type_name: types::ELEVE_COORDONNEES_MODIFIEES.to_string(),
            aggregate: AggregateKind::Eleve,
            aggregate_id: eleve,
            eleve_id: Some(eleve),
            payload: json!({ "coordonnee_id": coordonnee, "type": type_, "etat": etat }),
            actor_user_id: Some(acteur.to_string()),
        })
        .await?;
        Ok(())
    }

    async fn coordonnee_tx(
        tx: &mut PgConnection,
        etab: Uuid,
        id: Uuid,
        verrou: bool,
    ) -> Result<CoordonneeEleveVue, EntDbError> {
        let sql = format!(
            "SELECT {COLONNES} FROM ent_eleve_coordonnee WHERE etablissement_id = $1 AND id = $2{}",
            if verrou { " FOR UPDATE" } else { "" }
        );
        let r: Option<CoordRow> = sqlx::query_as(&sql).bind(etab).bind(id).fetch_optional(&mut *tx).await?;
        r.map(CoordonneeEleveVue::from).ok_or_else(|| DomainError::NotFound.into())
    }

    /// Qualité de l'acteur sur la fiche **d'une coordonnée** (la fiche est celle de la ligne) :
    /// `NotFound` si la coordonnée est absente ou d'un autre établissement.
    async fn qualite_sur_coordonnee(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        coord: Uuid,
    ) -> Result<(QualiteActeur, Uuid), EntDbError> {
        let r: Option<(Uuid,)> = sqlx::query_as(
            "SELECT eleve_id FROM ent_eleve_coordonnee WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(coord)
        .fetch_optional(&mut *TenantTx::begin(self.pool(), etab).await?)
        .await?;
        let (eleve,) = r.ok_or(DomainError::NotFound)?;
        let q = self.qualite_coordonnees(etab, acteur, role, eleve).await?;
        // E5 : toute écriture/vérification passe par ici ; le personnel en conflit est refusé.
        if q == QualiteActeur::Personnel {
            self.exiger_pas_de_conflit(etab, acteur, role, eleve).await?;
        }
        Ok((q, eleve))
    }

    /// ELV-17 : ajoute une coordonnée (personnel et élève : active, à vérifier) ou la propose
    /// (famille : à décider par le secrétariat).
    pub async fn ajouter_coordonnee_eleve(
        &self,
        role: &str,
        cmd: AjouterCoordonneeEleve,
    ) -> Result<CoordonneeEleveVue, EntDbError> {
        self.ajouter_coordonnee_eleve_a(role, cmd, Utc::now()).await
    }

    pub async fn ajouter_coordonnee_eleve_a(
        &self,
        role: &str,
        cmd: AjouterCoordonneeEleve,
        maintenant: DateTime<Utc>,
    ) -> Result<CoordonneeEleveVue, EntDbError> {
        let (etab, eleve, acteur) = (cmd.etablissement_id, cmd.eleve_id, cmd.acteur);
        let qualite = self.qualite_coordonnees(etab, acteur, role, eleve).await?;
        if qualite == QualiteActeur::Personnel {
            self.exiger_pas_de_conflit(etab, acteur, role, eleve).await?;
        }
        let type_ = TypeCoordonneeEleve::parse(&cmd.type_).map_err(invalide)?;
        let usages = normaliser_usages(type_, &cmd.usages).map_err(invalide)?;
        let valeur = match type_ {
            TypeCoordonneeEleve::Email => normaliser_email(&cmd.valeur)?,
            TypeCoordonneeEleve::Mobile => {
                let e164 = normaliser_telephone(&cmd.valeur)
                    .ok_or(EntDbError::SaisieInvalide("numéro de téléphone invalide"))?;
                if classer_telephone(&e164) != "mobile" {
                    return Err(EntDbError::SaisieInvalide("un numéro de mobile est attendu"));
                }
                e164
            }
        };
        let (statut, source) = match qualite {
            QualiteActeur::Personnel => ("active", "etablissement"),
            QualiteActeur::Eleve => ("active", "eleve"),
            QualiteActeur::Famille => ("proposee", "famille"),
        };
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
        let (vivantes,): (i64,) = sqlx::query_as(
            "SELECT count(*) FROM ent_eleve_coordonnee
              WHERE etablissement_id = $1 AND eleve_id = $2 AND type = $3 AND statut IN ('proposee', 'active')",
        )
        .bind(etab)
        .bind(eleve)
        .bind(type_.code())
        .fetch_one(&mut *tx)
        .await?;
        if vivantes >= COORDONNEES_VIVANTES_MAX_PAR_TYPE {
            return Err(EntDbError::ConflitDossier("trop de coordonnées de ce type : en retirer une d'abord"));
        }
        let id = Uuid::new_v4();
        let usages_texte: Vec<String> = usages.iter().map(|u| (*u).to_string()).collect();
        sqlx::query(
            "INSERT INTO ent_eleve_coordonnee (id, etablissement_id, eleve_id, type, valeur, usages, source,
                                               statut, du, cree_par, cree_le)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)",
        )
        .bind(id)
        .bind(etab)
        .bind(eleve)
        .bind(type_.code())
        .bind(&valeur)
        .bind(&usages_texte)
        .bind(source)
        .bind(statut)
        .bind(jour)
        .bind(acteur)
        .bind(maintenant)
        .execute(&mut *tx)
        .await
        .map_err(refus_unicite)?;
        let (action, libelle) = if statut == "proposee" {
            ("coordonnee_proposee", "Coordonnée de l'élève proposée")
        } else {
            ("coordonnee_ajoutee", "Coordonnée de l'élève ajoutée")
        };
        journaliser_tx(&mut tx, etab, Some(eleve), acteur, "eleve_coordonnee", action, libelle, role, maintenant).await?;
        Self::publier_coordonnee_tx(&mut tx, etab, eleve, acteur, id, type_.code(), statut).await?;
        let vue = Self::coordonnee_tx(&mut tx, etab, id, false).await?;
        tx.commit().await?;
        Ok(vue)
    }

    /// ELV-17 : le secrétariat accepte ou refuse une proposition de la famille.
    pub async fn decider_coordonnee_eleve(
        &self,
        role: &str,
        etab: Uuid,
        acteur: Uuid,
        coord: Uuid,
        accepter: bool,
        motif_code: Option<&str>,
    ) -> Result<CoordonneeEleveVue, EntDbError> {
        self.decider_coordonnee_eleve_a(role, etab, acteur, coord, accepter, motif_code, Utc::now()).await
    }

    pub async fn decider_coordonnee_eleve_a(
        &self,
        role: &str,
        etab: Uuid,
        acteur: Uuid,
        coord: Uuid,
        accepter: bool,
        motif_code: Option<&str>,
        maintenant: DateTime<Utc>,
    ) -> Result<CoordonneeEleveVue, EntDbError> {
        let (qualite, eleve) = self.qualite_sur_coordonnee(etab, acteur, role, coord).await?;
        if qualite != QualiteActeur::Personnel {
            return Err(EntDbError::Forbidden);
        }
        let motif = match (accepter, motif_code) {
            (true, _) => None,
            (false, Some(m)) => Some(valider_motif_coordonnee(m).map_err(invalide)?),
            (false, None) => return Err(EntDbError::SaisieInvalide("le motif du refus est obligatoire")),
        };
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        Self::verrouiller_fiche(&mut tx, etab, eleve).await?;
        let c = Self::coordonnee_tx(&mut tx, etab, coord, true).await?;
        if c.statut != "proposee" {
            return Err(EntDbError::ConflitDossier("cette coordonnée n'est plus en attente de décision"));
        }
        sqlx::query(
            "UPDATE ent_eleve_coordonnee SET statut = $3, decide_par = $4, decide_le = $5, motif_code = $6
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(coord)
        .bind(if accepter { "active" } else { "refusee" })
        .bind(acteur)
        .bind(maintenant)
        .bind(motif)
        .execute(&mut *tx)
        .await?;
        let (action, libelle, etat) = if accepter {
            ("coordonnee_acceptee", "Coordonnée de l'élève acceptée", "active")
        } else {
            ("coordonnee_refusee", "Coordonnée de l'élève refusée", "refusee")
        };
        journaliser_tx(&mut tx, etab, Some(eleve), acteur, "eleve_coordonnee", action, libelle, role, maintenant).await?;
        Self::publier_coordonnee_tx(&mut tx, etab, eleve, acteur, coord, &c.type_, etat).await?;
        let vue = Self::coordonnee_tx(&mut tx, etab, coord, false).await?;
        tx.commit().await?;
        Ok(vue)
    }

    /// ELV-17 : retire une coordonnée (active → retirée ; une proposition retirée par la famille
    /// ou le personnel devient refusée). Le personnel et l'élève retirent la leur ; la famille ne
    /// retire que ses propositions et les coordonnées de son enfant mineur.
    pub async fn retirer_coordonnee_eleve(
        &self,
        role: &str,
        etab: Uuid,
        acteur: Uuid,
        coord: Uuid,
        motif_code: &str,
    ) -> Result<CoordonneeEleveVue, EntDbError> {
        self.retirer_coordonnee_eleve_a(role, etab, acteur, coord, motif_code, Utc::now()).await
    }

    pub async fn retirer_coordonnee_eleve_a(
        &self,
        role: &str,
        etab: Uuid,
        acteur: Uuid,
        coord: Uuid,
        motif_code: &str,
        maintenant: DateTime<Utc>,
    ) -> Result<CoordonneeEleveVue, EntDbError> {
        let (qualite, eleve) = self.qualite_sur_coordonnee(etab, acteur, role, coord).await?;
        let motif = valider_motif_coordonnee(motif_code).map_err(invalide)?;
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
        let c = Self::coordonnee_tx(&mut tx, etab, coord, true).await?;
        let (cible, etat) = match c.statut.as_str() {
            "proposee" => ("refusee", "refusee"),
            "active" => ("retiree", "retiree"),
            _ => return Err(EntDbError::ConflitDossier("cette coordonnée est déjà retirée ou refusée")),
        };
        sqlx::query(
            "UPDATE ent_eleve_coordonnee SET statut = $3, decide_par = $4, decide_le = $5, motif_code = $6,
                    au = GREATEST(du, $7)
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(coord)
        .bind(cible)
        .bind(acteur)
        .bind(maintenant)
        .bind(motif)
        .bind(jour)
        .execute(&mut *tx)
        .await?;
        // Un code en attente pour cette coordonnée n'a plus d'objet.
        sqlx::query(
            "UPDATE ent_eleve_coordonnee_verification SET statut = 'annulee'
              WHERE etablissement_id = $1 AND coordonnee_id = $2 AND statut = 'envoye'",
        )
        .bind(etab)
        .bind(coord)
        .execute(&mut *tx)
        .await?;
        journaliser_tx(&mut tx, etab, Some(eleve), acteur, "eleve_coordonnee", "coordonnee_retiree", "Coordonnée de l'élève retirée", role, maintenant)
            .await?;
        Self::publier_coordonnee_tx(&mut tx, etab, eleve, acteur, coord, &c.type_, etat).await?;
        let vue = Self::coordonnee_tx(&mut tx, etab, coord, false).await?;
        tx.commit().await?;
        Ok(vue)
    }

    /// ELV-17 : change les usages d'une coordonnée active (personnel ou élève).
    pub async fn modifier_usages_coordonnee_eleve(
        &self,
        role: &str,
        etab: Uuid,
        acteur: Uuid,
        coord: Uuid,
        usages: &[String],
    ) -> Result<CoordonneeEleveVue, EntDbError> {
        self.modifier_usages_coordonnee_eleve_a(role, etab, acteur, coord, usages, Utc::now()).await
    }

    pub async fn modifier_usages_coordonnee_eleve_a(
        &self,
        role: &str,
        etab: Uuid,
        acteur: Uuid,
        coord: Uuid,
        usages: &[String],
        maintenant: DateTime<Utc>,
    ) -> Result<CoordonneeEleveVue, EntDbError> {
        let (qualite, eleve) = self.qualite_sur_coordonnee(etab, acteur, role, coord).await?;
        if qualite == QualiteActeur::Famille {
            return Err(EntDbError::Forbidden);
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        Self::verrouiller_fiche(&mut tx, etab, eleve).await?;
        let c = Self::coordonnee_tx(&mut tx, etab, coord, true).await?;
        let type_ = TypeCoordonneeEleve::parse(&c.type_).map_err(invalide)?;
        let usages = normaliser_usages(type_, usages).map_err(invalide)?;
        // M2 : l'usage « compte » (identifiant de connexion de l'élève) est réservé à l'élève
        // lui-même ; le personnel ne peut ni l'ajouter ni le retirer à sa place.
        let avait_compte = c.usages.iter().any(|u| u == "compte");
        let veut_compte = usages.contains(&"compte");
        if qualite != QualiteActeur::Eleve && avait_compte != veut_compte {
            return Err(EntDbError::Forbidden);
        }
        if c.statut != "active" {
            return Err(EntDbError::ConflitDossier("seule une coordonnée active a des usages"));
        }
        // Un usage « compte » ou « urgence » exige une coordonnée vérifiée.
        if c.verifiee_le.is_none() && usages.iter().any(|u| *u == "compte" || *u == "urgence") {
            return Err(EntDbError::ConflitDossier("vérifier la coordonnée avant de l'utiliser pour le compte ou l'urgence"));
        }
        let usages_texte: Vec<String> = usages.iter().map(|u| (*u).to_string()).collect();
        sqlx::query("UPDATE ent_eleve_coordonnee SET usages = $3 WHERE etablissement_id = $1 AND id = $2")
            .bind(etab)
            .bind(coord)
            .bind(&usages_texte)
            .execute(&mut *tx)
            .await?;
        journaliser_tx(
            &mut tx,
            etab,
            Some(eleve),
            acteur,
            "eleve_coordonnee",
            "usages_modifies",
            "Usages d'une coordonnée de l'élève modifiés",
            role,
            maintenant,
        )
        .await?;
        Self::publier_coordonnee_tx(&mut tx, etab, eleve, acteur, coord, &c.type_, "usages").await?;
        let vue = Self::coordonnee_tx(&mut tx, etab, coord, false).await?;
        tx.commit().await?;
        Ok(vue)
    }

    /// Envoie un code à la coordonnée (e-mail ou SMS). Personnel ou élève ; trois codes par heure.
    pub async fn demander_verification_coordonnee_eleve(
        &self,
        role: &str,
        etab: Uuid,
        acteur: Uuid,
        coord: Uuid,
    ) -> Result<VerificationEleveEnvoyee, EntDbError> {
        self.demander_verification_coordonnee_eleve_a(role, etab, acteur, coord, Utc::now()).await
    }

    pub async fn demander_verification_coordonnee_eleve_a(
        &self,
        role: &str,
        etab: Uuid,
        acteur: Uuid,
        coord: Uuid,
        maintenant: DateTime<Utc>,
    ) -> Result<VerificationEleveEnvoyee, EntDbError> {
        let (qualite, eleve) = self.qualite_sur_coordonnee(etab, acteur, role, coord).await?;
        if qualite == QualiteActeur::Famille {
            return Err(EntDbError::Forbidden);
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let c = Self::coordonnee_tx(&mut tx, etab, coord, true).await?;
        if c.statut != "active" {
            return Err(EntDbError::ConflitDossier("seule une coordonnée active se vérifie"));
        }
        if c.verifiee_le.is_some() {
            return Err(EntDbError::ConflitDossier("cette coordonnée est déjà vérifiée"));
        }
        let type_ = TypeCoordonneeEleve::parse(&c.type_).map_err(invalide)?;
        let depuis = maintenant - Duration::hours(1);
        let (recentes,): (i64,) = sqlx::query_as(
            "SELECT count(*) FROM ent_eleve_coordonnee_verification WHERE etablissement_id = $1 AND coordonnee_id = $2 AND envoye_le > $3",
        )
        .bind(etab)
        .bind(coord)
        .bind(depuis)
        .fetch_one(&mut *tx)
        .await?;
        // Plafond par élève : retirer puis rajouter la coordonnée ne remet pas le compteur à zéro.
        let (par_eleve,): (i64,) = sqlx::query_as(
            "SELECT count(*) FROM ent_eleve_coordonnee_verification
              WHERE etablissement_id = $1 AND eleve_id = $2 AND envoye_le > $3",
        )
        .bind(etab)
        .bind(eleve)
        .bind(depuis)
        .fetch_one(&mut *tx)
        .await?;
        if recentes >= VERIFICATION_DEMANDES_PAR_HEURE || par_eleve >= 4 * VERIFICATION_DEMANDES_PAR_HEURE {
            return Err(EntDbError::ConflitDossier("trop de codes demandés, réessayer plus tard"));
        }
        sqlx::query(
            "UPDATE ent_eleve_coordonnee_verification SET statut = 'annulee'
              WHERE etablissement_id = $1 AND coordonnee_id = $2 AND statut = 'envoye'",
        )
        .bind(etab)
        .bind(coord)
        .execute(&mut *tx)
        .await?;
        let id = Uuid::new_v4();
        let code = code_six_chiffres();
        let expire = maintenant + Duration::minutes(VERIFICATION_TTL_MINUTES);
        sqlx::query(
            "INSERT INTO ent_eleve_coordonnee_verification
                (id, etablissement_id, eleve_id, coordonnee_id, canal, code_hash, expire_le, demande_par, envoye_le)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
        )
        .bind(id)
        .bind(etab)
        .bind(eleve)
        .bind(coord)
        .bind(type_.canal())
        .bind(empreinte_verification(id, &code)?)
        .bind(expire)
        .bind(acteur)
        .bind(maintenant)
        .execute(&mut *tx)
        .await?;
        // La demande est validée AVANT l'envoi (aucune connexion ni verrou retenu pendant un envoi
        // lent) ; si l'envoi échoue, elle est annulée dans une transaction à part.
        tx.commit().await?;
        let envoi_ok = match type_ {
            TypeCoordonneeEleve::Email => {
                let r = self
                    .mailer()
                    .send(OutgoingMail {
                        to: c.valeur.clone(),
                        subject: "Votre code de vérification".into(),
                        body_text: format!(
                            "Bonjour,\n\nVotre code de vérification est : {code}\n\
Il est valable {VERIFICATION_TTL_MINUTES} minutes.\n\n\
Si vous n'êtes pas à l'origine de cette demande, ignorez ce message.\n"
                        ),
                        attachments: vec![],
                    })
                    .await;
                matches!(r, Ok(ref r) if self.envoi_reussi(r))
            }
            TypeCoordonneeEleve::Mobile => {
                let r = self
                    .sms()
                    .send(OutgoingSms {
                        to: c.valeur.clone(),
                        body: format!("Code de vérification : {code} (valable {VERIFICATION_TTL_MINUTES} min)."),
                    })
                    .await;
                matches!(r, Ok(ref r) if self.sms_envoi_reussi(r))
            }
        };
        if !envoi_ok {
            tracing::warn!(canal = type_.canal(), "vérification de coordonnée d'élève : code non envoyé");
            let mut tx = TenantTx::begin(self.pool(), etab).await?;
            sqlx::query(
                "UPDATE ent_eleve_coordonnee_verification SET statut = 'annulee'
                  WHERE id = $1 AND etablissement_id = $2 AND statut = 'envoye'",
            )
            .bind(id)
            .bind(etab)
            .execute(&mut *tx)
            .await?;
            tx.commit().await?;
            return Err(EntDbError::EnvoiImpossible);
        }
        Ok(VerificationEleveEnvoyee { coordonnee_id: coord, canal: type_.canal(), expire_le: expire })
    }

    /// Confirme le code. Erreur unique quelle que soit la cause ; cinq essais au plus.
    pub async fn verifier_coordonnee_eleve(
        &self,
        role: &str,
        etab: Uuid,
        acteur: Uuid,
        coord: Uuid,
        code: &str,
    ) -> Result<CoordonneeEleveVue, EntDbError> {
        self.verifier_coordonnee_eleve_a(role, etab, acteur, coord, code, Utc::now()).await
    }

    pub async fn verifier_coordonnee_eleve_a(
        &self,
        role: &str,
        etab: Uuid,
        acteur: Uuid,
        coord: Uuid,
        code: &str,
        maintenant: DateTime<Utc>,
    ) -> Result<CoordonneeEleveVue, EntDbError> {
        let generique = || EntDbError::SaisieInvalide("code invalide ou expiré");
        let code = code.trim();
        if code.len() != 6 || !code.chars().all(|c| c.is_ascii_digit()) {
            return Err(generique());
        }
        let (qualite, eleve) = self.qualite_sur_coordonnee(etab, acteur, role, coord).await?;
        if qualite == QualiteActeur::Famille {
            return Err(EntDbError::Forbidden);
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let c = Self::coordonnee_tx(&mut tx, etab, coord, true).await?;
        if c.statut != "active" || c.verifiee_le.is_some() {
            return Err(generique());
        }
        let row: Option<(Uuid, String, i16, DateTime<Utc>)> = sqlx::query_as(
            "SELECT id, code_hash, tentatives, expire_le FROM ent_eleve_coordonnee_verification
              WHERE etablissement_id = $1 AND coordonnee_id = $2 AND statut = 'envoye' FOR UPDATE",
        )
        .bind(etab)
        .bind(coord)
        .fetch_optional(&mut *tx)
        .await?;
        let Some((vid, hash, tentatives, expire)) = row else {
            return Err(generique());
        };
        if expire < maintenant {
            sqlx::query("UPDATE ent_eleve_coordonnee_verification SET statut = 'expiree' WHERE etablissement_id = $1 AND id = $2")
                .bind(etab)
                .bind(vid)
                .execute(&mut *tx)
                .await?;
            tx.commit().await?;
            return Err(generique());
        }
        if !ent_auth::ct_eq(&hash, &empreinte_verification(vid, code)?) {
            let epuise = tentatives + 1 >= VERIFICATION_ESSAIS_MAX;
            sqlx::query(
                "UPDATE ent_eleve_coordonnee_verification
                    SET tentatives = tentatives + 1, statut = CASE WHEN $2 THEN 'epuisee' ELSE statut END
                  WHERE etablissement_id = $3 AND id = $1",
            )
            .bind(vid)
            .bind(epuise)
            .bind(etab)
            .execute(&mut *tx)
            .await?;
            tx.commit().await?;
            return Err(generique());
        }
        sqlx::query("UPDATE ent_eleve_coordonnee_verification SET statut = 'verifiee', verifiee_le = $2 WHERE etablissement_id = $3 AND id = $1")
            .bind(vid)
            .bind(maintenant)
            .bind(etab)
            .execute(&mut *tx)
            .await?;
        sqlx::query("UPDATE ent_eleve_coordonnee SET verifiee_le = $3 WHERE etablissement_id = $1 AND id = $2")
            .bind(etab)
            .bind(coord)
            .bind(maintenant)
            .execute(&mut *tx)
            .await?;
        journaliser_tx(&mut tx, etab, Some(eleve), acteur, "eleve_coordonnee", "coordonnee_verifiee", "Coordonnée de l'élève vérifiée", role, maintenant)
            .await?;
        Self::publier_coordonnee_tx(&mut tx, etab, eleve, acteur, coord, &c.type_, "verifiee").await?;
        let vue = Self::coordonnee_tx(&mut tx, etab, coord, false).await?;
        tx.commit().await?;
        Ok(vue)
    }

    /// Coordonnées de l'élève. Personnel : `eleves.identite.modifier` sur l'élève (qui les gère),
    /// historique complet, lecture journalisée ; élève : les siennes ; famille : celles de son
    /// enfant **mineur et non émancipé**, proposées ou actives seulement.
    pub async fn coordonnees_eleve_pour_acteur(
        &self,
        role: &str,
        etab: Uuid,
        acteur: Uuid,
        eleve: Uuid,
    ) -> Result<Vec<CoordonneeEleveVue>, EntDbError> {
        self.coordonnees_eleve_pour_acteur_a(role, etab, acteur, eleve, Utc::now()).await
    }

    pub async fn coordonnees_eleve_pour_acteur_a(
        &self,
        role: &str,
        etab: Uuid,
        acteur: Uuid,
        eleve: Uuid,
        maintenant: DateTime<Utc>,
    ) -> Result<Vec<CoordonneeEleveVue>, EntDbError> {
        let parsed = ent_auth::Role::parse(role).ok_or(EntDbError::Forbidden)?;
        let personnel = parsed.is_staff();
        if personnel {
            if !self.autoriser_eleve(etab, acteur, role, Action::ElevesIdentiteModifier, eleve).await? {
                return Err(EntDbError::Forbidden);
            }
            self.exiger_eleve_visible(etab, acteur, role, eleve).await?;
        } else {
            self.qualite_coordonnees(etab, acteur, role, eleve).await?;
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let jour: NaiveDate = sqlx::query_scalar("SELECT ent_aujourdhui_etab($1, $2)")
            .bind(etab)
            .bind(maintenant)
            .fetch_one(&mut *tx)
            .await?;
        let existe: Option<(Uuid,)> =
            sqlx::query_as("SELECT id FROM ent_eleve WHERE etablissement_id = $1 AND id = $2")
                .bind(etab)
                .bind(eleve)
                .fetch_optional(&mut *tx)
                .await?;
        if existe.is_none() {
            return Err(DomainError::NotFound.into());
        }
        if parsed == ent_auth::Role::Parent && majeur_ou_emancipe_tx(&mut tx, etab, eleve, jour).await? {
            return Err(DomainError::NotFound.into());
        }
        let filtre = if personnel { "TRUE" } else { "statut IN ('proposee', 'active')" };
        let sql = format!(
            "SELECT {COLONNES} FROM ent_eleve_coordonnee WHERE etablissement_id = $1 AND eleve_id = $2 AND {filtre}
              ORDER BY cree_le DESC, id"
        );
        let l: Vec<CoordRow> = sqlx::query_as(&sql).bind(etab).bind(eleve).fetch_all(&mut *tx).await?;
        tx.commit().await?;
        if personnel {
            self.tracer_consultation_a(etab, acteur, role, Some(eleve), "coordonnees", "Consultation des coordonnées de l'élève", false, maintenant)
                .await?;
        }
        Ok(l.into_iter().map(CoordonneeEleveVue::from).collect())
    }

    /// Coordonnées **utilisables pour un envoi** : actives et vérifiées, pour l'usage demandé
    /// (`notifications`, `compte`, `urgence`). Point d'entrée de la messagerie et des notifications
    /// (jamais une coordonnée non vérifiée ni retirée).
    pub async fn coordonnees_eleve_pour_usage(
        &self,
        etab: Uuid,
        eleve: Uuid,
        usage: &str,
    ) -> Result<Vec<(String, String)>, EntDbError> {
        let l: Vec<(String, String)> = sqlx::query_as(
            "SELECT type, valeur FROM ent_eleve_coordonnee
              WHERE etablissement_id = $1 AND eleve_id = $2 AND statut = 'active'
                AND verifiee_le IS NOT NULL AND $3 = ANY(usages)
              ORDER BY type, cree_le, id",
        )
        .bind(etab)
        .bind(eleve)
        .bind(usage)
        .fetch_all(&mut *TenantTx::begin(self.pool(), etab).await?)
        .await?;
        Ok(l)
    }
}
