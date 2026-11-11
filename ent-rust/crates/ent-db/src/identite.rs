//! Socle 2 (l'élève), lot E2 — identité versionnée : dépôt et commandes.
//!
//! Quatre commandes, chacune dans UNE transaction (`TenantTx`) : version + journal +
//! événement. L'identité de `ent_eleve` n'est jamais écrite ici : la base la projette
//! depuis la version insérée (trigger) et refuse toute autre écriture.
//!
//! * ELV-03 [`PgEnt::corriger_identite`] — nature `correction` (la valeur était fausse
//!   depuis le début) : remplace une version, garde sa date d'effet.
//! * ELV-04 [`PgEnt::enregistrer_changement_etat_civil`] — nature
//!   `changement_etat_civil` (la personne a changé à une date) : pièce obligatoire.
//! * ELV-05 [`PgEnt::definir_usage`] — nom ou prénom d'usage (sans toucher l'état civil).
//! * ELV-06 [`PgEnt::historique_identite_pour_acteur`] — lecture, journalisée, filtrée
//!   par les droits (G5 ; versions sensibles : G6).
//!
//! Aucun état civil dans les événements ni dans le journal : identifiant de version,
//! nature et NOMS des champs seulement.

use chrono::{DateTime, NaiveDate, Utc};
use domain_eleve::{
    filtrer_historique_identite, planifier_version_identite, DemandeVersion, DomainError,
    ErreurIdentite, IdentiteVersion, ModifIdentite, NatureVersion,
};
use ent_auth::Action;
use ent_events::{types, AggregateKind, MetierEventDraft};
use serde_json::json;
use sqlx::PgConnection;
use uuid::Uuid;

use crate::outbox::Publier;
use crate::service::{EntDbError, PgEnt};
use crate::tenant_tx::TenantTx;

fn aujourdhui() -> NaiveDate {
    domain_eleve::horloge::jour_paris()
}

#[derive(sqlx::FromRow)]
struct VersionRow {
    id: Uuid,
    etablissement_id: Uuid,
    eleve_id: Uuid,
    nom_naissance: String,
    nom_usage: Option<String>,
    prenoms: Vec<String>,
    rang_prenom_usuel: i16,
    prenom_usage: Option<String>,
    sexe: Option<String>,
    date_naissance: Option<NaiveDate>,
    naissance_commune_cog: Option<String>,
    naissance_commune: Option<String>,
    naissance_pays_cog: Option<String>,
    naissance_lieu_etranger: Option<String>,
    date_naissance_precision: String,
    date_naissance_statut: String,
    nom_ecriture_origine: Option<String>,
    prenoms_ecriture_origine: Option<String>,
    motif_code: Option<String>,
    nature: String,
    effet_du: NaiveDate,
    remplace_version_id: Option<Uuid>,
    remplacee_par: Option<Uuid>,
    motif: Option<String>,
    piece_document_id: Option<Uuid>,
    source: String,
    source_ref: Option<Uuid>,
    sensible: bool,
    enregistre_le: DateTime<Utc>,
    enregistre_par: Option<Uuid>,
}

impl VersionRow {
    fn en_version(self) -> IdentiteVersion {
        IdentiteVersion {
            id: self.id,
            etablissement_id: self.etablissement_id,
            eleve_id: self.eleve_id,
            nom_naissance: self.nom_naissance,
            nom_usage: self.nom_usage,
            prenoms: self.prenoms,
            rang_prenom_usuel: self.rang_prenom_usuel,
            prenom_usage: self.prenom_usage,
            sexe: self.sexe,
            date_naissance: self.date_naissance,
            naissance_commune_cog: self.naissance_commune_cog,
            naissance_commune: self.naissance_commune,
            naissance_pays_cog: self.naissance_pays_cog,
            naissance_lieu_etranger: self.naissance_lieu_etranger,
            date_naissance_precision: self.date_naissance_precision,
            date_naissance_statut: self.date_naissance_statut,
            nom_ecriture_origine: self.nom_ecriture_origine,
            prenoms_ecriture_origine: self.prenoms_ecriture_origine,
            motif_code: self.motif_code,
            nature: NatureVersion::parse(&self.nature).unwrap_or(NatureVersion::Reprise),
            effet_du: self.effet_du,
            remplace_version_id: self.remplace_version_id,
            remplacee_par: self.remplacee_par,
            motif: self.motif,
            piece_document_id: self.piece_document_id,
            source: self.source,
            source_ref: self.source_ref,
            sensible: self.sensible,
            enregistre_le: self.enregistre_le,
            enregistre_par: self.enregistre_par,
        }
    }
}

const COLONNES: &str = "id, etablissement_id, eleve_id, nom_naissance, nom_usage, prenoms,
    rang_prenom_usuel, prenom_usage, sexe, date_naissance, naissance_commune_cog,
    naissance_commune, naissance_pays_cog, naissance_lieu_etranger, date_naissance_precision, date_naissance_statut,
    nom_ecriture_origine, prenoms_ecriture_origine, motif_code, nature, effet_du,
    remplace_version_id, remplacee_par, motif, piece_document_id, source, source_ref,
    sensible, enregistre_le, enregistre_par";

/// Accès aux versions d'identité (toutes les requêtes portent `etablissement_id`).
pub struct PgIdentiteRepo;

impl PgIdentiteRepo {
    /// Verrouille la ligne d'élève (sérialise les commandes concurrentes) ; `false` si
    /// l'élève n'existe pas dans cet établissement.
    pub(crate) async fn verrouiller_eleve(
        conn: &mut PgConnection,
        etab: Uuid,
        eleve: Uuid,
    ) -> Result<bool, sqlx::Error> {
        let r: Option<(Uuid,)> = sqlx::query_as(
            "SELECT id FROM ent_eleve WHERE etablissement_id = $1 AND id = $2 FOR UPDATE",
        )
        .bind(etab)
        .bind(eleve)
        .fetch_optional(&mut *conn)
        .await?;
        Ok(r.is_some())
    }

    async fn version(
        conn: &mut PgConnection,
        etab: Uuid,
        eleve: Uuid,
        version: Uuid,
    ) -> Result<Option<IdentiteVersion>, sqlx::Error> {
        let sql = format!(
            "SELECT {COLONNES} FROM ent_eleve_identite
             WHERE etablissement_id = $1 AND eleve_id = $2 AND id = $3"
        );
        Ok(sqlx::query_as::<_, VersionRow>(&sql)
            .bind(etab)
            .bind(eleve)
            .bind(version)
            .fetch_optional(&mut *conn)
            .await?
            .map(VersionRow::en_version))
    }

    /// Version en vigueur (non remplacée, effective) à une date.
    pub(crate) async fn en_vigueur_a(
        conn: &mut PgConnection,
        etab: Uuid,
        eleve: Uuid,
        jour: NaiveDate,
    ) -> Result<Option<IdentiteVersion>, sqlx::Error> {
        let sql = format!(
            "SELECT {COLONNES} FROM ent_eleve_identite
             WHERE etablissement_id = $1 AND eleve_id = $2 AND remplacee_par IS NULL AND effet_du <= $3
             ORDER BY effet_du DESC, enregistre_le DESC, id LIMIT 1"
        );
        Ok(sqlx::query_as::<_, VersionRow>(&sql)
            .bind(etab)
            .bind(eleve)
            .bind(jour)
            .fetch_optional(&mut *conn)
            .await?
            .map(VersionRow::en_version))
    }

    /// Plus ancienne version en vigueur (base d'un changement antérieur à toute version).
    pub(crate) async fn plus_ancienne(
        conn: &mut PgConnection,
        etab: Uuid,
        eleve: Uuid,
    ) -> Result<Option<IdentiteVersion>, sqlx::Error> {
        let sql = format!(
            "SELECT {COLONNES} FROM ent_eleve_identite
             WHERE etablissement_id = $1 AND eleve_id = $2 AND remplacee_par IS NULL
             ORDER BY effet_du, enregistre_le, id LIMIT 1"
        );
        Ok(sqlx::query_as::<_, VersionRow>(&sql)
            .bind(etab)
            .bind(eleve)
            .fetch_optional(&mut *conn)
            .await?
            .map(VersionRow::en_version))
    }

    pub(crate) async fn dates_en_vigueur(
        conn: &mut PgConnection,
        etab: Uuid,
        eleve: Uuid,
    ) -> Result<Vec<NaiveDate>, sqlx::Error> {
        let l: Vec<(NaiveDate,)> = sqlx::query_as(
            "SELECT effet_du FROM ent_eleve_identite
             WHERE etablissement_id = $1 AND eleve_id = $2 AND remplacee_par IS NULL",
        )
        .bind(etab)
        .bind(eleve)
        .fetch_all(&mut *conn)
        .await?;
        Ok(l.into_iter().map(|(d,)| d).collect())
    }

    async fn marquer_remplacee(
        conn: &mut PgConnection,
        etab: Uuid,
        eleve: Uuid,
        ancienne: Uuid,
        nouvelle: Uuid,
    ) -> Result<(), sqlx::Error> {
        let r = sqlx::query(
            "UPDATE ent_eleve_identite SET remplacee_par = $4
             WHERE etablissement_id = $1 AND eleve_id = $2 AND id = $3 AND remplacee_par IS NULL",
        )
        .bind(etab)
        .bind(eleve)
        .bind(ancienne)
        .bind(nouvelle)
        .execute(&mut *conn)
        .await?;
        if r.rows_affected() != 1 {
            return Err(sqlx::Error::RowNotFound);
        }
        Ok(())
    }

    async fn inserer(conn: &mut PgConnection, v: &IdentiteVersion) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO ent_eleve_identite (
                id, etablissement_id, eleve_id, nom_naissance, nom_usage, prenoms,
                rang_prenom_usuel, prenom_usage, sexe, date_naissance, naissance_commune_cog,
                naissance_commune, naissance_pays_cog, naissance_lieu_etranger, date_naissance_precision, date_naissance_statut,
                nom_ecriture_origine, prenoms_ecriture_origine, motif_code, nature, effet_du,
                remplace_version_id, motif, piece_document_id, source, source_ref, sensible,
                enregistre_le, enregistre_par
             ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19,$20,$21,$22,$23,$24,$25,$26,$27,$28,$29)",
        )
        .bind(v.id)
        .bind(v.etablissement_id)
        .bind(v.eleve_id)
        .bind(&v.nom_naissance)
        .bind(&v.nom_usage)
        .bind(&v.prenoms)
        .bind(v.rang_prenom_usuel)
        .bind(&v.prenom_usage)
        .bind(&v.sexe)
        .bind(v.date_naissance)
        .bind(&v.naissance_commune_cog)
        .bind(&v.naissance_commune)
        .bind(&v.naissance_pays_cog)
        .bind(&v.naissance_lieu_etranger)
        .bind(&v.date_naissance_precision)
        .bind(&v.date_naissance_statut)
        .bind(&v.nom_ecriture_origine)
        .bind(&v.prenoms_ecriture_origine)
        .bind(&v.motif_code)
        .bind(v.nature.as_str())
        .bind(v.effet_du)
        .bind(v.remplace_version_id)
        .bind(&v.motif)
        .bind(v.piece_document_id)
        .bind(&v.source)
        .bind(v.source_ref)
        .bind(v.sensible)
        .bind(v.enregistre_le)
        .bind(v.enregistre_par)
        .execute(&mut *conn)
        .await?;
        Ok(())
    }

    /// Toutes les versions d'un élève (plus récentes d'abord).
    async fn historique(
        conn: &mut PgConnection,
        etab: Uuid,
        eleve: Uuid,
    ) -> Result<Vec<IdentiteVersion>, sqlx::Error> {
        let sql = format!(
            "SELECT {COLONNES} FROM ent_eleve_identite
             WHERE etablissement_id = $1 AND eleve_id = $2
             ORDER BY effet_du DESC, enregistre_le DESC, id"
        );
        Ok(sqlx::query_as::<_, VersionRow>(&sql)
            .bind(etab)
            .bind(eleve)
            .fetch_all(&mut *conn)
            .await?
            .into_iter()
            .map(VersionRow::en_version)
            .collect())
    }

    /// La pièce est-elle un document administratif ou familial de CET élève dans CET
    /// établissement ? (jamais un document de santé)
    pub(crate) async fn piece_de_l_eleve(
        conn: &mut PgConnection,
        etab: Uuid,
        eleve: Uuid,
        document: Uuid,
    ) -> Result<bool, sqlx::Error> {
        let r: Option<(Uuid,)> = sqlx::query_as(
            "SELECT id FROM ent_eleve_document
             WHERE etablissement_id = $1 AND eleve_id = $2 AND id = $3
               AND tiroir IN ('administratif', 'famille') AND confidentialite <> 'sante'",
        )
        .bind(etab)
        .bind(eleve)
        .bind(document)
        .fetch_optional(&mut *conn)
        .await?;
        Ok(r.is_some())
    }
}

/// Demande de correction (ELV-03).
#[derive(Debug, Clone)]
pub struct CorrectionIdentite {
    pub etablissement_id: Uuid,
    pub eleve_id: Uuid,
    /// Version corrigée ; à défaut, la version courante.
    pub version_id: Option<Uuid>,
    pub modif: ModifIdentite,
    /// Code du motif (obligatoire) ; texte libre seulement pour `autre`.
    pub motif_code: String,
    pub motif: Option<String>,
    pub piece_document_id: Option<Uuid>,
    pub source: String,
    pub acteur: Uuid,
}

/// Demande de changement d'état civil (ELV-04).
#[derive(Debug, Clone)]
pub struct ChangementEtatCivil {
    pub etablissement_id: Uuid,
    pub eleve_id: Uuid,
    pub modif: ModifIdentite,
    pub effet_du: NaiveDate,
    pub motif_code: String,
    pub motif: Option<String>,
    pub piece_document_id: Uuid,
    pub source: String,
    pub acteur: Uuid,
}

/// Nom ou prénom d'usage (ELV-05) : `Some(None)` le retire.
#[derive(Debug, Clone)]
pub struct UsageIdentite {
    pub etablissement_id: Uuid,
    pub eleve_id: Uuid,
    pub nom_usage: Option<Option<String>>,
    pub prenom_usage: Option<Option<String>>,
    pub effet_du: Option<NaiveDate>,
    pub motif_code: Option<String>,
    pub motif: Option<String>,
    pub piece_document_id: Option<Uuid>,
    pub source: String,
    pub acteur: Uuid,
}

/// Résultat d'une commande : la version créée et les NOMS des champs modifiés.
#[derive(Debug, Clone)]
pub struct ResultatIdentite {
    pub version: IdentiteVersion,
    pub champs: Vec<&'static str>,
}

fn erreur(e: ErreurIdentite) -> EntDbError {
    match e {
        ErreurIdentite::Invalide(m) => EntDbError::SaisieInvalide(m),
        ErreurIdentite::Conflit(m) => EntDbError::ConflitDossier(m),
    }
}

/// Une violation de la garde ou d'une contrainte devient un refus lisible.
fn traduire(e: sqlx::Error) -> EntDbError {
    if matches!(e, sqlx::Error::RowNotFound) {
        return EntDbError::ConflitDossier("la version à remplacer a changé, réessayer");
    }
    if let sqlx::Error::Database(d) = &e {
        match d.code().as_deref() {
            Some("23505") => {
                return EntDbError::ConflitDossier("une version existe déjà à cette date d'effet")
            }
            Some("23503") => return EntDbError::SaisieInvalide("référence inconnue"),
            Some("23514") => {
                return EntDbError::SaisieInvalide("valeur refusée par les règles d'identité")
            }
            _ => {}
        }
    }
    EntDbError::Sqlx(e)
}

impl PgEnt {
    /// Droit d'écriture d'identité (portée établissement) — le service refait le
    /// contrôle que la route n'a fait qu'en pré-filtre.
    async fn exiger_ecriture_identite(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
    ) -> Result<(), EntDbError> {
        if self
            .a_action_etablissement(etab, acteur, role, Action::ElevesIdentiteModifier)
            .await?
        {
            Ok(())
        } else {
            Err(EntDbError::Forbidden)
        }
    }

    /// ELV-03 : corrige l'identité (la valeur précédente était fausse depuis le début).
    pub async fn corriger_identite(
        &self,
        role: &str,
        cmd: CorrectionIdentite,
    ) -> Result<ResultatIdentite, EntDbError> {
        self.exiger_ecriture_identite(cmd.etablissement_id, cmd.acteur, role)
            .await?;
        let (etab, eleve) = (cmd.etablissement_id, cmd.eleve_id);
        let mut tx = self.ouvrir_commande(etab, eleve).await?;
        let base = match cmd.version_id {
            Some(id) => PgIdentiteRepo::version(&mut tx, etab, eleve, id).await?,
            None => PgIdentiteRepo::en_vigueur_a(&mut tx, etab, eleve, aujourdhui()).await?,
        }
        .ok_or(DomainError::NotFound)?;
        let dates = PgIdentiteRepo::dates_en_vigueur(&mut tx, etab, eleve).await?;
        let plan = planifier_version_identite(
            &base,
            &dates,
            DemandeVersion {
                nature: NatureVersion::Correction,
                modif: cmd.modif,
                effet_du: None,
                motif_code: Some(cmd.motif_code),
                motif: cmd.motif,
                piece_document_id: cmd.piece_document_id,
                source: cmd.source,
                source_ref: None,
                acteur: Some(cmd.acteur),
            },
            aujourdhui(),
        )
        .map_err(erreur)?;
        Self::exiger_libres(&mut tx, etab, &plan).await?;
        self.finaliser_identite(tx, plan, Some(cmd.acteur)).await
    }

    /// ELV-04 : enregistre un changement réel d'état civil à une date (pièce obligatoire).
    pub async fn enregistrer_changement_etat_civil(
        &self,
        role: &str,
        cmd: ChangementEtatCivil,
    ) -> Result<ResultatIdentite, EntDbError> {
        self.exiger_ecriture_identite(cmd.etablissement_id, cmd.acteur, role)
            .await?;
        let (etab, eleve) = (cmd.etablissement_id, cmd.eleve_id);
        let mut tx = self.ouvrir_commande(etab, eleve).await?;
        let base = base_a(&mut tx, etab, eleve, cmd.effet_du).await?;
        let dates = PgIdentiteRepo::dates_en_vigueur(&mut tx, etab, eleve).await?;
        let plan = planifier_version_identite(
            &base,
            &dates,
            DemandeVersion {
                nature: NatureVersion::ChangementEtatCivil,
                modif: cmd.modif,
                effet_du: Some(cmd.effet_du),
                motif_code: Some(cmd.motif_code),
                motif: cmd.motif,
                piece_document_id: Some(cmd.piece_document_id),
                source: cmd.source,
                source_ref: None,
                acteur: Some(cmd.acteur),
            },
            aujourdhui(),
        )
        .map_err(erreur)?;
        Self::exiger_libres(&mut tx, etab, &plan).await?;
        self.finaliser_identite(tx, plan, Some(cmd.acteur)).await
    }

    /// ELV-05 : définit ou retire un nom ou prénom d'usage (état civil inchangé).
    pub async fn definir_usage(
        &self,
        role: &str,
        cmd: UsageIdentite,
    ) -> Result<ResultatIdentite, EntDbError> {
        self.exiger_ecriture_identite(cmd.etablissement_id, cmd.acteur, role)
            .await?;
        let (etab, eleve) = (cmd.etablissement_id, cmd.eleve_id);
        let mut tx = self.ouvrir_commande(etab, eleve).await?;
        let effet = cmd.effet_du.unwrap_or_else(aujourdhui);
        let base = base_a(&mut tx, etab, eleve, effet).await?;
        let dates = PgIdentiteRepo::dates_en_vigueur(&mut tx, etab, eleve).await?;
        let plan = planifier_version_identite(
            &base,
            &dates,
            DemandeVersion {
                nature: NatureVersion::Usage,
                modif: ModifIdentite {
                    nom_usage: cmd.nom_usage,
                    prenom_usage: cmd.prenom_usage,
                    ..Default::default()
                },
                effet_du: Some(effet),
                motif_code: cmd.motif_code,
                motif: cmd.motif,
                piece_document_id: cmd.piece_document_id,
                source: cmd.source,
                source_ref: None,
                acteur: Some(cmd.acteur),
            },
            aujourdhui(),
        )
        .map_err(erreur)?;
        Self::exiger_libres(&mut tx, etab, &plan).await?;
        self.finaliser_identite(tx, plan, Some(cmd.acteur)).await
    }

    /// Lot E5 : une commande DIRECTE ne modifie pas un champ verrouillé ni un champ dont la
    /// modification locale est « par proposition » ou « interdite » (source externe qui fait
    /// foi). Les champs sans réglage restent libres : aucun changement de comportement tant que
    /// l'établissement n'a rien réglé.
    async fn exiger_libres(
        tx: &mut TenantTx<'static>,
        etab: Uuid,
        plan: &domain_eleve::VersionPlanifiee,
    ) -> Result<(), EntDbError> {
        let codes: Vec<&str> = plan
            .champs
            .iter()
            .filter_map(|c| domain_eleve::champ_catalogue_de(*c))
            .collect();
        Self::exiger_champs_modifiables(tx, etab, &codes).await
    }

    /// Ouvre la transaction d'une commande et verrouille l'élève (sérialise les commandes
    /// concurrentes sur un même élève).
    async fn ouvrir_commande(
        &self,
        etab: Uuid,
        eleve: Uuid,
    ) -> Result<TenantTx<'static>, EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        if !PgIdentiteRepo::verrouiller_eleve(&mut *tx, etab, eleve).await? {
            return Err(DomainError::NotFound.into());
        }
        Ok(tx)
    }

    /// Fin commune : pièce, remplacement, version, événement, journal, commit — dans la
    /// transaction ouverte. Toute erreur défait tout (aucun `let _ =`).
    async fn finaliser_identite(
        &self,
        mut tx: TenantTx<'static>,
        plan: domain_eleve::VersionPlanifiee,
        acteur: Option<Uuid>,
    ) -> Result<ResultatIdentite, EntDbError> {
        let r = Self::ecrire_version(&mut tx, plan, acteur).await?;
        tx.commit().await?;
        Ok(r)
    }

    /// Écrit version + événement + journal dans la transaction de l'appelant (sans commit).
    pub(crate) async fn ecrire_version(
        tx: &mut TenantTx<'static>,
        plan: domain_eleve::VersionPlanifiee,
        acteur: Option<Uuid>,
    ) -> Result<ResultatIdentite, EntDbError> {
        let v = plan.version;
        let remplacee = v.remplace_version_id;
        let (etab, eleve) = (v.etablissement_id, v.eleve_id);
        if let Some(p) = v.piece_document_id {
            if !PgIdentiteRepo::piece_de_l_eleve(&mut *tx, etab, eleve, p)
                .await
                .map_err(traduire)?
            {
                return Err(EntDbError::SaisieInvalide(
                    "pièce justificative inconnue ou non admise pour cet élève",
                ));
            }
        }
        // Ordre imposé par l'unicité (élève, date d'effet) parmi les versions en vigueur :
        // l'ancienne est marquée remplacée AVANT l'insertion de la nouvelle.
        if let Some(ancienne) = remplacee {
            PgIdentiteRepo::marquer_remplacee(&mut *tx, etab, eleve, ancienne, v.id)
                .await
                .map_err(traduire)?;
        }
        PgIdentiteRepo::inserer(&mut *tx, &v)
            .await
            .map_err(traduire)?;
        let champs: Vec<&'static str> = plan.champs.iter().map(|c| c.nom()).collect();
        tx.publier(MetierEventDraft {
            etablissement_id: etab,
            type_name: types::ELEVE_IDENTITY_CHANGED.to_string(),
            aggregate: AggregateKind::Eleve,
            aggregate_id: eleve,
            eleve_id: Some(eleve),
            // Jamais de valeur d'identité : version, nature, NOMS des champs.
            payload: json!({
                "version_id": v.id,
                "nature": v.nature.as_str(),
                "champs": champs,
                "sensible": v.sensible,
            }),
            actor_user_id: acteur.map(|a| a.to_string()),
        })
        .await?;
        sqlx::query(
            "INSERT INTO ent_journal_modif (id, etablissement_id, eleve_id, domaine, action, libelle,
                                             payload, visible_famille, actor_user_id, created_at)
             VALUES ($1,$2,$3,'administratif',$4,$5,$6,$7,$8, now())",
        )
        .bind(Uuid::new_v4())
        .bind(etab)
        .bind(eleve)
        .bind(format!("identite_{}", v.nature.as_str()))
        .bind("Identité de l'élève : nouvelle version")
        .bind(json!({ "version_id": v.id, "nature": v.nature.as_str(), "champs": champs }))
        // Une version sensible (prénom d'usage, sexe) n'est jamais montrée à la famille.
        .bind(!v.sensible)
        .bind(acteur)
        .execute(&mut **tx)
        .await?;
        Ok(ResultatIdentite { version: v, champs })
    }

    /// ELV-06 : historique d'identité. Il faut G5 (`eleves.historique.lire`) sur l'élève ;
    /// les versions sensibles n'apparaissent qu'avec G6 (`eleves.confidentiel.gerer`).
    /// La lecture est journalisée (acteur et objet, jamais le contenu).
    pub async fn historique_identite_pour_acteur(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        eleve: Uuid,
    ) -> Result<Vec<IdentiteVersion>, EntDbError> {
        if !self
            .autoriser_eleve(etab, acteur, role, Action::ElevesHistoriqueLire, eleve)
            .await?
        {
            return Err(EntDbError::Forbidden);
        }
        let voit_sensible = self
            .autoriser_eleve(etab, acteur, role, Action::ElevesConfidentielGerer, eleve)
            .await?;
        let mut conn = self.pool().acquire().await?;
        let versions = PgIdentiteRepo::historique(&mut conn, etab, eleve).await?;
        drop(conn);
        if versions.is_empty() {
            return Err(DomainError::NotFound.into());
        }
        self.tracer_acces_sensible(
            etab,
            acteur,
            role,
            Some(eleve),
            "identite",
            if voit_sensible {
                "historique_complet"
            } else {
                "historique"
            },
            "Consultation de l'historique d'identité",
        )
        .await;
        Ok(filtrer_historique_identite(versions, voit_sensible))
    }

    /// Tâche quotidienne : une version annoncée devenue effective est projetée sur la
    /// fiche. Idempotente ; renvoie le nombre d'élèves resynchronisés.
    pub async fn resynchroniser_identites(&self) -> Result<i64, EntDbError> {
        let (n,): (i64,) = sqlx::query_as("SELECT ent_eleve_identite_resynchroniser($1)")
            .bind(domain_eleve::horloge::jour_paris())
            .fetch_one(self.pool())
            .await?;
        Ok(n)
    }
}

impl PgEnt {
    /// Pont de compatibilité de l'ancienne saisie d'identité (`PUT /eleves/{id}/identite`) :
    /// un changement de nom, prénom, sexe ou date de naissance devient une **correction**
    /// versionnée dans la transaction de l'appelant. Refusé si un nom ou prénom d'usage
    /// masquerait la valeur affichée (utiliser la correction d'identité).
    pub(crate) async fn correction_heritee(
        tx: &mut TenantTx<'static>,
        avant: &domain_eleve::Eleve,
        apres: &domain_eleve::Eleve,
        acteur: Option<Uuid>,
    ) -> Result<Option<ResultatIdentite>, EntDbError> {
        let (etab, eleve) = (avant.etablissement_id, avant.id);
        let (nom, prenom) = (avant.nom != apres.nom, avant.prenom != apres.prenom);
        let (sexe, naissance) = (
            avant.sexe != apres.sexe,
            avant.date_naissance != apres.date_naissance,
        );
        if !(nom || prenom || sexe || naissance) {
            return Ok(None);
        }
        let base = PgIdentiteRepo::en_vigueur_a(&mut *tx, etab, eleve, aujourdhui())
            .await?
            .ok_or(DomainError::NotFound)?;
        if (nom && base.nom_usage.is_some()) || (prenom && base.prenom_usage.is_some()) {
            return Err(EntDbError::ConflitDossier(
                "un nom ou prénom d'usage est défini : utiliser la correction d'identité",
            ));
        }
        let mut modif = ModifIdentite::default();
        if nom {
            modif.nom_naissance = Some(apres.nom.clone());
        }
        if prenom {
            let mut ps = base.prenoms.clone();
            ps[(base.rang_prenom_usuel - 1) as usize] = apres.prenom.clone();
            modif.prenoms = Some(ps);
        }
        if sexe {
            modif.sexe = Some(apres.sexe.clone());
        }
        if naissance {
            modif.date_naissance = Some(apres.date_naissance);
        }
        let dates = PgIdentiteRepo::dates_en_vigueur(&mut *tx, etab, eleve).await?;
        let plan = planifier_version_identite(
            &base,
            &dates,
            DemandeVersion {
                nature: NatureVersion::Correction,
                modif,
                effet_du: None,
                motif_code: Some("autre".into()),
                motif: Some("Correction depuis la saisie d'identité".into()),
                piece_document_id: None,
                source: "saisie".into(),
                source_ref: None,
                acteur,
            },
            aujourdhui(),
        )
        .map_err(erreur)?;
        Self::exiger_libres(tx, etab, &plan).await?;
        Ok(Some(Self::ecrire_version(tx, plan, acteur).await?))
    }
}

/// Base d'un changement à la date `effet` : la version en vigueur à cette date, sinon
/// (date antérieure à toute version) la plus ancienne.
async fn base_a(
    conn: &mut PgConnection,
    etab: Uuid,
    eleve: Uuid,
    effet: NaiveDate,
) -> Result<IdentiteVersion, EntDbError> {
    let v = match PgIdentiteRepo::en_vigueur_a(conn, etab, eleve, effet).await? {
        Some(v) => Some(v),
        None => PgIdentiteRepo::plus_ancienne(conn, etab, eleve).await?,
    };
    v.ok_or_else(|| DomainError::NotFound.into())
}
