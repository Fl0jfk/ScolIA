//! Socle 2 (l'élève), lot E4 — scolarité : dépôt et commandes.
//!
//! * ELV-08 [`PgEnt::ouvrir_scolarisation`] (entrée) ; ELV-12 [`PgEnt::reintegrer_eleve`] ;
//! * ELV-11 [`PgEnt::cloturer_scolarisation`] (sortie, radiation) ; ELV-34 [`PgEnt::enregistrer_deces`] ;
//! * ELV-09 [`PgEnt::enregistrer_inscription_annuelle`], [`PgEnt::changer_classe`],
//!   [`PgEnt::changer_options`] ;
//! * ELV-10 [`PgEnt::changer_regime_ou_site`] (dates autorisées, dérogation motivée : R-E5) ;
//! * ELV-13 [`PgEnt::declarer_capacite`] / [`PgEnt::lever_capacite`] ;
//! * ELV-35 mobilité, ELV-36 modalités de scolarisation ;
//! * ELV-14 / ELV-39 et recalcul des statuts : [`PgEnt::tache_nocturne_eleves`].
//!
//! Chaque commande = UNE transaction (`TenantTx`) : fait + journal + événement. Le **statut
//! n'est jamais saisi** : la base le dérive des scolarisations (`ent_eleve_statut_a`) et le
//! met en cache ; les commandes en publient le changement. Aucune donnée d'identité dans un
//! événement ni dans le journal.
//!
//! Les commandes `*_systeme` (socle 3, imports) n'ont pas de contrôle de rôle : l'appelant
//! est un traitement du serveur, jamais une route.

use chrono::{DateTime, Duration, NaiveDate, Utc};
use domain_eleve::{
    date_changement_autorisee, planifier_cloture_scolarisation, planifier_ouverture_scolarisation,
    prochaine_date_autorisee, AttributKind, ChangeAttribut, CloturePlanifiee, CodeModalite, CutoverOp,
    DatesChangement, DemandeCloture, DemandeOuverture, DomainError, ElevePeriode, ErreurScolarite,
    EtablissementScolarite, EtatInscription, MotifDerogationRegime, MotifEntree, MotifSortie,
    NatureCapacite, PeriodeScolarisation, StatutApprenant, TypeMobilite,
};
use ent_auth::Action;
use ent_events::{types, AggregateKind, MetierEventDraft};
use serde_json::json;
use sqlx::PgConnection;
use uuid::Uuid;

use crate::eleve_repo::PgEleveRepo;
use crate::outbox::Publier;
use crate::service::{EntDbError, PgEnt};
use crate::tenant_tx::{lire_par_etablissement, ContexteSysteme, TenantTx};

fn erreur(e: ErreurScolarite) -> EntDbError {
    match e {
        ErreurScolarite::Invalide(m) => EntDbError::SaisieInvalide(m),
        ErreurScolarite::Conflit(m) => EntDbError::ConflitDossier(m),
    }
}

/// Une violation de contrainte devient un refus lisible.
fn traduire(e: sqlx::Error) -> EntDbError {
    if let sqlx::Error::Database(d) = &e {
        match d.code().as_deref() {
            // exclusion (chevauchement) et unicité
            Some("23P01") => {
                return EntDbError::ConflitDossier("la période chevauche une période existante de cet élève")
            }
            Some("23505") => return EntDbError::ConflitDossier("cet enregistrement existe déjà"),
            Some("23503") => return EntDbError::SaisieInvalide("référence inconnue"),
            Some("23514") => return EntDbError::SaisieInvalide("valeur refusée par les règles de scolarité"),
            Some("42501") => return EntDbError::Forbidden,
            _ => {}
        }
    }
    EntDbError::Sqlx(e)
}

// ───────────────────────────── Types ─────────────────────────────

/// Une scolarisation, telle que stockée.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ScolarisationEleve {
    pub id: Uuid,
    pub etablissement_id: Uuid,
    pub eleve_id: Uuid,
    pub date_entree: NaiveDate,
    pub motif_entree: String,
    pub origine_uai: Option<String>,
    pub origine_libelle: Option<String>,
    pub origine_type: Option<String>,
    pub date_sortie: Option<NaiveDate>,
    pub motif_sortie: Option<String>,
    pub motif_sortie_libre: Option<String>,
    pub destination_uai: Option<String>,
    pub destination_libelle: Option<String>,
    pub destination_type: Option<String>,
    pub exeat_document_id: Option<Uuid>,
    pub source: String,
    pub decide_par: Option<Uuid>,
    pub cree_le: DateTime<Utc>,
}

const COL_SCOL: &str = "id, etablissement_id, eleve_id, date_entree, motif_entree, origine_uai,
    origine_libelle, origine_type, date_sortie, motif_sortie, motif_sortie_libre, destination_uai,
    destination_libelle, destination_type, exeat_document_id, source, decide_par, cree_le";

/// Une inscription annuelle, telle que stockée.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct InscriptionEleve {
    pub id: Uuid,
    pub etablissement_id: Uuid,
    pub eleve_id: Uuid,
    pub scolarisation_id: Uuid,
    pub annee_id: Uuid,
    pub niveau_id: Option<Uuid>,
    pub mef_id: Option<Uuid>,
    pub statut_apprenant: String,
    pub etat: String,
    pub redoublement: Option<bool>,
    pub decision_passage: Option<String>,
    pub source: String,
}

const COL_INSC: &str = "id, etablissement_id, eleve_id, scolarisation_id, annee_id, niveau_id, mef_id,
    statut_apprenant, etat, redoublement, decision_passage, source";

/// Capacité juridique déclarée.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct CapaciteEleve {
    pub id: Uuid,
    pub etablissement_id: Uuid,
    pub eleve_id: Uuid,
    pub nature: String,
    pub du: NaiveDate,
    pub au: Option<NaiveDate>,
    pub piece_document_id: Uuid,
    pub motif: Option<String>,
    pub saisi_le: DateTime<Utc>,
    pub levee_le: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct MobiliteEleve {
    pub id: Uuid,
    pub eleve_id: Uuid,
    pub du: NaiveDate,
    pub au: Option<NaiveDate>,
    pub type_: String,
    pub retour_prevu: Option<NaiveDate>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ModaliteEleve {
    pub id: Uuid,
    pub eleve_id: Uuid,
    pub code: String,
    pub du: NaiveDate,
    pub au: Option<NaiveDate>,
    pub source: String,
}

/// Résultat d'une commande de scolarisation : la ligne et le statut dérivé.
#[derive(Debug, Clone)]
pub struct ResultatScolarisation {
    pub scolarisation: ScolarisationEleve,
    pub statut: String,
    pub statut_change: bool,
}

#[derive(Debug, Clone)]
pub struct OuvertureScolarisation {
    pub etablissement_id: Uuid,
    pub eleve_id: Uuid,
    pub date_entree: NaiveDate,
    pub motif: MotifEntree,
    pub origine: EtablissementScolarite,
    /// `saisie`, `admission`, `reinscription`, `montee`, `import`.
    pub source: String,
    pub source_ref: Option<Uuid>,
    pub acteur: Option<Uuid>,
}

#[derive(Debug, Clone)]
pub struct ClotureScolarisation {
    pub etablissement_id: Uuid,
    pub eleve_id: Uuid,
    pub date_sortie: NaiveDate,
    pub motif: MotifSortie,
    pub motif_libre: Option<String>,
    pub destination: EtablissementScolarite,
    pub exeat_document_id: Option<Uuid>,
    pub acteur: Option<Uuid>,
}

#[derive(Debug, Clone)]
pub struct InscriptionAnnuelle {
    pub etablissement_id: Uuid,
    pub eleve_id: Uuid,
    pub annee_id: Uuid,
    pub niveau_id: Option<Uuid>,
    pub mef_id: Option<Uuid>,
    pub statut_apprenant: StatutApprenant,
    pub etat: EtatInscription,
    pub redoublement: Option<bool>,
    pub source: String,
    pub acteur: Option<Uuid>,
}

#[derive(Debug, Clone)]
pub struct ChangementClasse {
    pub etablissement_id: Uuid,
    pub eleve_id: Uuid,
    pub classe_id: Uuid,
    pub date_effet: NaiveDate,
    pub acteur: Uuid,
}

#[derive(Debug, Clone)]
pub struct ChangementOptions {
    pub etablissement_id: Uuid,
    pub eleve_id: Uuid,
    pub annee_id: Uuid,
    pub ajouter: Vec<Uuid>,
    pub retirer: Vec<Uuid>,
    pub date_effet: NaiveDate,
    pub acteur: Uuid,
}

#[derive(Debug, Clone)]
pub struct ChangementRegimeSite {
    pub etablissement_id: Uuid,
    pub eleve_id: Uuid,
    /// `Regime` ou `Site` seulement.
    pub kind: AttributKind,
    pub valeur: String,
    pub date_effet: NaiveDate,
    /// Dérogation motivée si la date n'est pas une date autorisée (régime seulement).
    pub derogation: Option<MotifDerogationRegime>,
    pub acteur: Uuid,
}

#[derive(Debug, Clone)]
pub struct DeclarationCapacite {
    pub etablissement_id: Uuid,
    pub eleve_id: Uuid,
    pub nature: NatureCapacite,
    pub du: NaiveDate,
    pub piece_document_id: Uuid,
    pub motif: Option<String>,
    pub acteur: Uuid,
}

/// Bilan de la tâche quotidienne.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BilanTacheEleves {
    pub identites_resynchronisees: i64,
    pub statuts_changes: i64,
    pub repercussions_sortie: i64,
    pub majeurs: i64,
    pub majorites_proches: i64,
    pub marqueurs_silencieux: i64,
}

// ───────────────────────────── Dépôt ─────────────────────────────

struct Repo;

impl Repo {
    async fn jour(conn: &mut PgConnection, etab: Uuid) -> Result<NaiveDate, sqlx::Error> {
        let (j,): (NaiveDate,) = sqlx::query_as("SELECT ent_jour_etablissement($1)")
            .bind(etab)
            .fetch_one(&mut *conn)
            .await?;
        Ok(j)
    }

    async fn verrouiller_eleve(conn: &mut PgConnection, etab: Uuid, eleve: Uuid) -> Result<bool, sqlx::Error> {
        let r: Option<(Uuid,)> =
            sqlx::query_as("SELECT id FROM ent_eleve WHERE etablissement_id = $1 AND id = $2 FOR UPDATE")
                .bind(etab)
                .bind(eleve)
                .fetch_optional(&mut *conn)
                .await?;
        Ok(r.is_some())
    }

    /// (statut dérivé en cache, statut texte hérité) de l'élève.
    async fn statuts(conn: &mut PgConnection, etab: Uuid, eleve: Uuid) -> Result<(Option<String>, String), sqlx::Error> {
        sqlx::query_as("SELECT statut, status FROM ent_eleve WHERE etablissement_id = $1 AND id = $2")
            .bind(etab)
            .bind(eleve)
            .fetch_one(&mut *conn)
            .await
    }

    async fn periodes(conn: &mut PgConnection, etab: Uuid, eleve: Uuid) -> Result<Vec<PeriodeScolarisation>, sqlx::Error> {
        let l: Vec<(NaiveDate, Option<NaiveDate>, Option<String>)> = sqlx::query_as(
            "SELECT date_entree, date_sortie, motif_sortie FROM ent_eleve_scolarisation
             WHERE etablissement_id = $1 AND eleve_id = $2 ORDER BY date_entree",
        )
        .bind(etab)
        .bind(eleve)
        .fetch_all(&mut *conn)
        .await?;
        Ok(l.into_iter()
            .map(|(e, s, m)| PeriodeScolarisation {
                date_entree: e,
                date_sortie: s,
                motif_sortie: m.as_deref().and_then(MotifSortie::parse),
            })
            .collect())
    }

    async fn scolarisation(conn: &mut PgConnection, etab: Uuid, id: Uuid) -> Result<ScolarisationEleve, sqlx::Error> {
        let sql = format!("SELECT {COL_SCOL} FROM ent_eleve_scolarisation WHERE etablissement_id = $1 AND id = $2");
        sqlx::query_as(&sql).bind(etab).bind(id).fetch_one(&mut *conn).await
    }

    async fn ouverte(conn: &mut PgConnection, etab: Uuid, eleve: Uuid) -> Result<Option<ScolarisationEleve>, sqlx::Error> {
        let sql = format!(
            "SELECT {COL_SCOL} FROM ent_eleve_scolarisation
             WHERE etablissement_id = $1 AND eleve_id = $2 AND date_sortie IS NULL FOR UPDATE"
        );
        sqlx::query_as(&sql).bind(etab).bind(eleve).fetch_optional(&mut *conn).await
    }

    /// Document administratif ou familial de CET élève (jamais santé).
    async fn piece_de_l_eleve(conn: &mut PgConnection, etab: Uuid, eleve: Uuid, doc: Uuid) -> Result<bool, sqlx::Error> {
        let r: Option<(Uuid,)> = sqlx::query_as(
            "SELECT id FROM ent_eleve_document
             WHERE etablissement_id = $1 AND eleve_id = $2 AND id = $3
               AND tiroir IN ('administratif', 'famille') AND confidentialite <> 'sante'",
        )
        .bind(etab)
        .bind(eleve)
        .bind(doc)
        .fetch_optional(&mut *conn)
        .await?;
        Ok(r.is_some())
    }
}

/// Entrée de journal du domaine « scolarité », sans donnée d'identité.
async fn journal(
    tx: &mut TenantTx<'static>,
    etab: Uuid,
    eleve: Uuid,
    action: &str,
    libelle: &str,
    payload: serde_json::Value,
    visible_famille: bool,
    acteur: Option<Uuid>,
) -> Result<(), EntDbError> {
    // Les faits sensibles (décès, capacité, mobilité, modalité, dérogation de régime) sont
    // journalisés pour l'audit mais dans un domaine absent de la liste blanche de la
    // chronologie 360° : ils n'y apparaissent pas, même sous un titre neutre.
    let domaine = if matches!(
        action,
        "eleve_decede"
            | "capacite_declaree"
            | "capacite_levee"
            | "mobilite_ouverte"
            | "mobilite_fermee"
            | "modalite_enregistree"
            | "regime_derogation"
    ) {
        "eleve_sensible"
    } else {
        "scolarite"
    };
    sqlx::query(
        "INSERT INTO ent_journal_modif (id, etablissement_id, eleve_id, domaine, action, libelle,
                                         payload, visible_famille, actor_user_id, created_at)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9, now())",
    )
    .bind(Uuid::new_v4())
    .bind(etab)
    .bind(eleve)
    .bind(domaine)
    .bind(action)
    .bind(libelle)
    .bind(payload)
    .bind(visible_famille)
    .bind(acteur)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

async fn evenement(
    tx: &mut TenantTx<'static>,
    etab: Uuid,
    eleve: Uuid,
    type_name: &str,
    payload: serde_json::Value,
    acteur: Option<Uuid>,
) -> Result<(), EntDbError> {
    tx.publier(MetierEventDraft {
        etablissement_id: etab,
        type_name: type_name.to_string(),
        aggregate: AggregateKind::Eleve,
        aggregate_id: eleve,
        eleve_id: Some(eleve),
        payload,
        actor_user_id: acteur.map(|a| a.to_string()),
    })
    .await?;
    Ok(())
}

/// Publie `eleve.status_changed` si le statut dérivé a changé (clés héritées conservées).
async fn publier_changement_statut(
    tx: &mut TenantTx<'static>,
    etab: Uuid,
    eleve: Uuid,
    avant: &(Option<String>, String),
    acteur: Option<Uuid>,
) -> Result<(String, bool), EntDbError> {
    let apres = Repo::statuts(&mut *tx, etab, eleve).await?;
    let nouveau = apres.0.clone().unwrap_or_else(|| "a_venir".into());
    let change = avant.0 != apres.0;
    if change {
        evenement(
            tx,
            etab,
            eleve,
            types::ELEVE_STATUS_CHANGED,
            json!({
                "ancien_statut": avant.0,
                "nouveau_statut": nouveau,
                "ancien_status": avant.1,
                "nouveau_status": apres.1,
            }),
            acteur,
        )
        .await?;
    }
    Ok((nouveau, change))
}

fn etab_params(e: &EtablissementScolarite) -> (Option<String>, Option<String>, Option<&'static str>) {
    (e.uai.clone(), e.libelle.clone(), e.type_.map(|t| t.code()))
}

impl PgEnt {
    async fn exiger_scolarite(&self, etab: Uuid, acteur: Uuid, role: &str, action: Action) -> Result<(), EntDbError> {
        if self.a_action_etablissement(etab, acteur, role, action).await? {
            Ok(())
        } else {
            Err(EntDbError::Forbidden)
        }
    }

    /// Ouvre la transaction d'une commande et verrouille l'élève.
    async fn ouvrir_tx_eleve(&self, etab: Uuid, eleve: Uuid) -> Result<TenantTx<'static>, EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        if !Repo::verrouiller_eleve(&mut tx, etab, eleve).await? {
            return Err(DomainError::NotFound.into());
        }
        Ok(tx)
    }

    /// Jour courant dans le fuseau de l'établissement (R-E15).
    pub async fn jour_etablissement(&self, etab: Uuid) -> Result<NaiveDate, EntDbError> {
        let mut conn = TenantTx::begin(self.pool(), etab).await?;
        Ok(Repo::jour(&mut conn, etab).await?)
    }

    // ───────────────────── ELV-08 / ELV-12 : entrée et réintégration ─────────────────────

    async fn ecrire_ouverture(
        tx: &mut TenantTx<'static>,
        cmd: &OuvertureScolarisation,
        reintegration: bool,
    ) -> Result<ResultatScolarisation, EntDbError> {
        let (etab, eleve) = (cmd.etablissement_id, cmd.eleve_id);
        let jour = Repo::jour(&mut *tx, etab).await?;
        let existantes = Repo::periodes(&mut *tx, etab, eleve).await?;
        let plan = planifier_ouverture_scolarisation(
            &existantes,
            DemandeOuverture {
                date_entree: cmd.date_entree,
                motif: cmd.motif,
                origine: cmd.origine.clone(),
            },
            reintegration,
            jour,
        )
        .map_err(erreur)?;
        let avant = Repo::statuts(&mut *tx, etab, eleve).await?;
        let id = Uuid::new_v4();
        let (uai, libelle, type_) = etab_params(&plan.origine);
        sqlx::query(
            "INSERT INTO ent_eleve_scolarisation (id, etablissement_id, eleve_id, date_entree, motif_entree,
                origine_uai, origine_libelle, origine_type, source, source_ref, decide_par)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11)",
        )
        .bind(id)
        .bind(etab)
        .bind(eleve)
        .bind(plan.date_entree)
        .bind(plan.motif.code())
        .bind(uai)
        .bind(libelle)
        .bind(type_)
        .bind(&cmd.source)
        .bind(cmd.source_ref)
        .bind(cmd.acteur)
        .execute(&mut **tx)
        .await
        .map_err(traduire)?;
        let (statut, statut_change) = publier_changement_statut(tx, etab, eleve, &avant, cmd.acteur).await?;
        evenement(
            tx,
            etab,
            eleve,
            types::SCOLARITE_OPENED,
            json!({
                "scolarisation_id": id,
                "date_entree": plan.date_entree.to_string(),
                "motif": plan.motif.code(),
                "reintegration": reintegration,
            }),
            cmd.acteur,
        )
        .await?;
        if reintegration {
            evenement(
                tx,
                etab,
                eleve,
                types::ELEVE_REINTEGRE,
                json!({ "scolarisation_id": id }),
                cmd.acteur,
            )
            .await?;
        }
        journal(
            tx,
            etab,
            eleve,
            if reintegration { "eleve_reintegre" } else { "scolarisation_ouverte" },
            if reintegration { "Ancien élève réintégré" } else { "Scolarisation ouverte" },
            json!({ "scolarisation_id": id, "motif": plan.motif.code() }),
            true,
            cmd.acteur,
        )
        .await?;
        let scolarisation = Repo::scolarisation(&mut *tx, etab, id).await?;
        Ok(ResultatScolarisation {
            scolarisation,
            statut,
            statut_change,
        })
    }

    /// ELV-08 : ouvre une scolarisation (entrée). Pas de chevauchement.
    pub async fn ouvrir_scolarisation(
        &self,
        role: &str,
        cmd: OuvertureScolarisation,
    ) -> Result<ResultatScolarisation, EntDbError> {
        let acteur = cmd.acteur.ok_or(EntDbError::Forbidden)?;
        self.exiger_scolarite(cmd.etablissement_id, acteur, role, Action::ElevesScolariteGerer)
            .await?;
        self.ouverture_commune(cmd, false).await
    }

    /// ELV-08 pour le socle 3 et les imports (inscription validée, réinscription) : sans rôle.
    pub async fn ouvrir_scolarisation_systeme(
        &self,
        cmd: OuvertureScolarisation,
    ) -> Result<ResultatScolarisation, EntDbError> {
        self.ouverture_commune(cmd, false).await
    }

    /// ELV-12 : réintègre un ancien élève sur SA fiche (nouvelle scolarisation, motif « retour »).
    pub async fn reintegrer_eleve(
        &self,
        role: &str,
        mut cmd: OuvertureScolarisation,
    ) -> Result<ResultatScolarisation, EntDbError> {
        let acteur = cmd.acteur.ok_or(EntDbError::Forbidden)?;
        self.exiger_scolarite(cmd.etablissement_id, acteur, role, Action::ElevesScolariteGerer)
            .await?;
        cmd.motif = MotifEntree::Retour;
        self.ouverture_commune(cmd, true).await
    }

    /// ELV-12 pour le socle 3 : sans rôle.
    pub async fn reintegrer_eleve_systeme(
        &self,
        mut cmd: OuvertureScolarisation,
    ) -> Result<ResultatScolarisation, EntDbError> {
        cmd.motif = MotifEntree::Retour;
        self.ouverture_commune(cmd, true).await
    }

    async fn ouverture_commune(
        &self,
        cmd: OuvertureScolarisation,
        reintegration: bool,
    ) -> Result<ResultatScolarisation, EntDbError> {
        if !matches!(
            cmd.source.as_str(),
            "saisie" | "admission" | "reinscription" | "montee" | "import"
        ) {
            return Err(EntDbError::SaisieInvalide("source de scolarisation inconnue"));
        }
        let mut tx = self.ouvrir_tx_eleve(cmd.etablissement_id, cmd.eleve_id).await?;
        let r = Self::ecrire_ouverture(&mut tx, &cmd, reintegration).await?;
        tx.commit().await?;
        Ok(r)
    }

    // ───────────────────── ELV-11 / ELV-34 : sortie et décès ─────────────────────

    async fn ecrire_cloture(
        tx: &mut TenantTx<'static>,
        cmd: &ClotureScolarisation,
        deces: bool,
    ) -> Result<(ResultatScolarisation, NaiveDate), EntDbError> {
        let (etab, eleve) = (cmd.etablissement_id, cmd.eleve_id);
        let jour = Repo::jour(&mut *tx, etab).await?;
        let courante = Repo::ouverte(&mut *tx, etab, eleve)
            .await?
            .ok_or(EntDbError::ConflitDossier("aucune scolarisation ouverte à clôturer"))?;
        let periode = PeriodeScolarisation {
            date_entree: courante.date_entree,
            date_sortie: None,
            motif_sortie: None,
        };
        let plan: CloturePlanifiee = planifier_cloture_scolarisation(
            &periode,
            DemandeCloture {
                date_sortie: cmd.date_sortie,
                motif: if deces { MotifSortie::Deces } else { cmd.motif },
                motif_libre: cmd.motif_libre.clone(),
                destination: cmd.destination.clone(),
            },
            jour,
        )
        .map_err(erreur)?;
        if let Some(p) = cmd.exeat_document_id {
            if !Repo::piece_de_l_eleve(&mut *tx, etab, eleve, p).await? {
                return Err(EntDbError::SaisieInvalide("pièce inconnue ou non admise pour cet élève"));
            }
        }
        let avant = Repo::statuts(&mut *tx, etab, eleve).await?;
        let (uai, libelle, type_) = etab_params(&plan.destination);
        sqlx::query(
            "UPDATE ent_eleve_scolarisation SET date_sortie = $3, motif_sortie = $4, motif_sortie_libre = $5,
                destination_uai = $6, destination_libelle = $7, destination_type = $8, exeat_document_id = $9,
                cloture_le = now(), cloture_par = $10
             WHERE etablissement_id = $1 AND id = $2 AND date_sortie IS NULL",
        )
        .bind(etab)
        .bind(courante.id)
        .bind(plan.date_sortie)
        .bind(plan.motif.code())
        .bind(&plan.motif_libre)
        .bind(uai)
        .bind(libelle)
        .bind(type_)
        .bind(cmd.exeat_document_id)
        .bind(cmd.acteur)
        .execute(&mut **tx)
        .await
        .map_err(traduire)?;
        // Les affectations (classe, groupes) s'arrêtent la veille de la sortie.
        let veille = plan.date_sortie - Duration::days(1);
        sqlx::query(
            "UPDATE ent_affectation SET date_fin = greatest(date_debut, $3)
             WHERE etablissement_id = $1 AND eleve_id = $2 AND (date_fin IS NULL OR date_fin > $3)",
        )
        .bind(etab)
        .bind(eleve)
        .bind(veille)
        .execute(&mut **tx)
        .await?;
        // RP2-12 : l'inscription annuelle en cours prend fin à la date de départ (l'historique,
        // les années passées et les prévues ne sont pas touchées). Sortie future : la date de fin
        // est posée, l'état « terminée » vient avec la date.
        sqlx::query(
            "UPDATE ent_eleve_inscription
                SET date_fin = greatest($4, coalesce(date_debut, $4)),
                    etat = CASE WHEN $4 <= $5 THEN 'terminee' ELSE etat END,
                    modifie_le = now()
              WHERE etablissement_id = $1 AND eleve_id = $2 AND scolarisation_id = $3
                AND etat = 'en_cours'",
        )
        .bind(etab)
        .bind(eleve)
        .bind(courante.id)
        .bind(plan.date_sortie)
        .bind(jour)
        .execute(&mut **tx)
        .await?;
        let (statut, statut_change) = publier_changement_statut(tx, etab, eleve, &avant, cmd.acteur).await?;
        evenement(
            tx,
            etab,
            eleve,
            types::SCOLARITE_CLOSED,
            json!({
                "scolarisation_id": courante.id,
                "date_sortie": plan.date_sortie.to_string(),
                // Un décès n'apparaît que dans `eleve.decede` (consommateurs désignés).
                "motif": if deces { "autre" } else { plan.motif.code() },
            }),
            cmd.acteur,
        )
        .await?;
        // Événement hérité `eleve.sortie` (consommateurs existants) : mêmes clés, motif codé.
        evenement(
            tx,
            etab,
            eleve,
            types::ELEVE_SORTIE,
            json!({
                "date_sortie": plan.date_sortie.to_string(),
                "motif_sortie": if deces { "autre" } else { plan.motif.code() },
                "status": if plan.date_sortie <= jour { "sorti" } else { "actif" },
            }),
            cmd.acteur,
        )
        .await?;
        if deces {
            // Aucun détail : les consommateurs (envois, relances, appels…) s'arrêtent seuls (R-E12).
            evenement(tx, etab, eleve, types::ELEVE_DECEDE, json!({ "scolarisation_id": courante.id }), cmd.acteur)
                .await?;
        }
        journal(
            tx,
            etab,
            eleve,
            if deces { "eleve_decede" } else { "scolarisation_close" },
            if deces { "Décès enregistré" } else { "Scolarisation clôturée" },
            json!({ "scolarisation_id": courante.id, "motif": plan.motif.code() }),
            false,
            cmd.acteur,
        )
        .await?;
        let scolarisation = Repo::scolarisation(&mut *tx, etab, courante.id).await?;
        Ok((
            ResultatScolarisation {
                scolarisation,
                statut,
                statut_change,
            },
            jour,
        ))
    }

    async fn cloture_commune(
        &self,
        cmd: ClotureScolarisation,
        deces: bool,
    ) -> Result<ResultatScolarisation, EntDbError> {
        let mut tx = self.ouvrir_tx_eleve(cmd.etablissement_id, cmd.eleve_id).await?;
        let (r, jour) = Self::ecrire_cloture(&mut tx, &cmd, deces).await?;
        tx.commit().await?;
        // Répercussions transverses (cantine, internat, accès, documents) : idempotentes, une
        // fois le fait enregistré. Sortie future : la tâche quotidienne s'en charge à la date.
        if let Some(sortie) = r.scolarisation.date_sortie.filter(|s| *s <= jour) {
            self.appliquer_repercussions_sortie(
                cmd.etablissement_id,
                cmd.eleve_id,
                sortie,
                cmd.acteur.map(|a| a.to_string()),
            )
            .await?;
        }
        Ok(r)
    }

    /// ELV-11 ★ : clôture la scolarisation (sortie, radiation).
    pub async fn cloturer_scolarisation(
        &self,
        role: &str,
        cmd: ClotureScolarisation,
    ) -> Result<ResultatScolarisation, EntDbError> {
        let acteur = cmd.acteur.ok_or(EntDbError::Forbidden)?;
        self.exiger_scolarite(cmd.etablissement_id, acteur, role, Action::ElevesSortie).await?;
        if cmd.motif == MotifSortie::Deces {
            return Err(EntDbError::SaisieInvalide("un décès s'enregistre par sa commande dédiée"));
        }
        self.cloture_commune(cmd, false).await
    }

    /// ELV-11 pour le socle 3 (départ annoncé, exeat) : sans rôle.
    pub async fn cloturer_scolarisation_systeme(
        &self,
        cmd: ClotureScolarisation,
    ) -> Result<ResultatScolarisation, EntDbError> {
        if cmd.motif == MotifSortie::Deces {
            return Err(EntDbError::SaisieInvalide("un décès s'enregistre par sa commande dédiée"));
        }
        self.cloture_commune(cmd, false).await
    }

    /// ELV-34 ★ : enregistre le décès d'un élève (motif `deces`, `eleve.decede` sans détail).
    pub async fn enregistrer_deces(
        &self,
        role: &str,
        etab: Uuid,
        eleve: Uuid,
        date: NaiveDate,
        acteur: Uuid,
    ) -> Result<ResultatScolarisation, EntDbError> {
        self.exiger_scolarite(etab, acteur, role, Action::ElevesSortie).await?;
        self.cloture_commune(
            ClotureScolarisation {
                etablissement_id: etab,
                eleve_id: eleve,
                date_sortie: date,
                motif: MotifSortie::Deces,
                motif_libre: None,
                destination: EtablissementScolarite::default(),
                exeat_document_id: None,
                acteur: Some(acteur),
            },
            true,
        )
        .await
    }

    // ───────────────────── ELV-09 : inscription annuelle, classe, options ─────────────────────

    /// ELV-09 : enregistre (crée ou met à jour) l'inscription d'une année. Une par année ;
    /// rattachée à la scolarisation qui couvre l'année.
    pub async fn enregistrer_inscription_annuelle(
        &self,
        role: &str,
        cmd: InscriptionAnnuelle,
    ) -> Result<InscriptionEleve, EntDbError> {
        let acteur = cmd.acteur.ok_or(EntDbError::Forbidden)?;
        self.exiger_scolarite(cmd.etablissement_id, acteur, role, Action::ElevesScolariteGerer).await?;
        self.inscription_commune(cmd).await
    }

    /// ELV-09 pour le socle 3 (réinscription, montée) : sans rôle.
    pub async fn enregistrer_inscription_annuelle_systeme(
        &self,
        cmd: InscriptionAnnuelle,
    ) -> Result<InscriptionEleve, EntDbError> {
        self.inscription_commune(cmd).await
    }

    async fn inscription_commune(&self, cmd: InscriptionAnnuelle) -> Result<InscriptionEleve, EntDbError> {
        if !matches!(cmd.source.as_str(), "saisie" | "admission" | "reinscription" | "montee" | "import") {
            return Err(EntDbError::SaisieInvalide("source d'inscription inconnue"));
        }
        let (etab, eleve) = (cmd.etablissement_id, cmd.eleve_id);
        let mut tx = self.ouvrir_tx_eleve(etab, eleve).await?;
        let annee: Option<(NaiveDate, NaiveDate)> =
            sqlx::query_as("SELECT date_debut, date_fin FROM ent_annee_scolaire WHERE etablissement_id = $1 AND id = $2")
                .bind(etab)
                .bind(cmd.annee_id)
                .fetch_optional(&mut *tx)
                .await?;
        let (debut, fin) = annee.ok_or(EntDbError::SaisieInvalide("année scolaire inconnue"))?;
        // Niveau et MEF : ceux de CET établissement.
        if let Some(n) = cmd.niveau_id {
            let ok: Option<(Uuid,)> =
                sqlx::query_as("SELECT id FROM ent_niveau WHERE etablissement_id = $1 AND id = $2")
                    .bind(etab)
                    .bind(n)
                    .fetch_optional(&mut *tx)
                    .await?;
            ok.ok_or(EntDbError::SaisieInvalide("niveau inconnu"))?;
        }
        if let Some(m) = cmd.mef_id {
            let ok: Option<(Uuid,)> =
                sqlx::query_as("SELECT id FROM ent_mef WHERE etablissement_id = $1 AND id = $2")
                    .bind(etab)
                    .bind(m)
                    .fetch_optional(&mut *tx)
                    .await?;
            ok.ok_or(EntDbError::SaisieInvalide("MEF inconnu"))?;
        }
        // Scolarisation qui couvre l'année (à défaut, l'élève n'est pas scolarisé cette année-là).
        let scol: Option<(Uuid,)> = sqlx::query_as(
            "SELECT id FROM ent_eleve_scolarisation
             WHERE etablissement_id = $1 AND eleve_id = $2
               AND date_entree < $4 AND (date_sortie IS NULL OR date_sortie > $3)
             ORDER BY date_entree DESC LIMIT 1",
        )
        .bind(etab)
        .bind(eleve)
        .bind(debut)
        .bind(fin)
        .fetch_optional(&mut *tx)
        .await?;
        let (scolarisation_id,) =
            scol.ok_or(EntDbError::ConflitDossier("l'élève n'est pas scolarisé sur cette année"))?;
        let existante: Option<(Uuid,)> = sqlx::query_as(
            "SELECT id FROM ent_eleve_inscription WHERE etablissement_id = $1 AND eleve_id = $2
               AND annee_id = $3 AND etat <> 'annulee' FOR UPDATE",
        )
        .bind(etab)
        .bind(eleve)
        .bind(cmd.annee_id)
        .fetch_optional(&mut *tx)
        .await?;
        let id = match existante {
            Some((id,)) => {
                sqlx::query(
                    "UPDATE ent_eleve_inscription SET scolarisation_id = $3, niveau_id = $4, mef_id = $5,
                        statut_apprenant = $6, etat = $7, redoublement = $8, modifie_le = now()
                     WHERE etablissement_id = $1 AND id = $2",
                )
                .bind(etab)
                .bind(id)
                .bind(scolarisation_id)
                .bind(cmd.niveau_id)
                .bind(cmd.mef_id)
                .bind(cmd.statut_apprenant.code())
                .bind(cmd.etat.code())
                .bind(cmd.redoublement)
                .execute(&mut *tx)
                .await
                .map_err(traduire)?;
                id
            }
            None => {
                let id = Uuid::new_v4();
                sqlx::query(
                    "INSERT INTO ent_eleve_inscription (id, etablissement_id, eleve_id, scolarisation_id, annee_id,
                        niveau_id, mef_id, statut_apprenant, etat, redoublement, date_debut, date_fin, source, cree_par)
                     VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14)",
                )
                .bind(id)
                .bind(etab)
                .bind(eleve)
                .bind(scolarisation_id)
                .bind(cmd.annee_id)
                .bind(cmd.niveau_id)
                .bind(cmd.mef_id)
                .bind(cmd.statut_apprenant.code())
                .bind(cmd.etat.code())
                .bind(cmd.redoublement)
                .bind(debut)
                .bind(fin)
                .bind(&cmd.source)
                .bind(cmd.acteur)
                .execute(&mut *tx)
                .await
                .map_err(traduire)?;
                id
            }
        };
        evenement(
            &mut tx,
            etab,
            eleve,
            types::ELEVE_SCOLARITE_ANNEE_ENREGISTREE,
            json!({ "inscription_id": id, "annee_id": cmd.annee_id, "statut": cmd.etat.code() }),
            cmd.acteur,
        )
        .await?;
        journal(
            &mut tx,
            etab,
            eleve,
            "inscription_annuelle",
            "Inscription annuelle enregistrée",
            json!({ "inscription_id": id, "etat": cmd.etat.code() }),
            true,
            cmd.acteur,
        )
        .await?;
        let sql = format!("SELECT {COL_INSC} FROM ent_eleve_inscription WHERE etablissement_id = $1 AND id = $2");
        let r: InscriptionEleve = sqlx::query_as(&sql).bind(etab).bind(id).fetch_one(&mut *tx).await?;
        tx.commit().await?;
        Ok(r)
    }

    /// Changement daté d'un attribut (période + projection fiche + événement + journal) dans la
    /// transaction de l'appelant.
    async fn ecrire_changement_attribut(
        tx: &mut TenantTx<'static>,
        etab: Uuid,
        eleve: Uuid,
        kind: AttributKind,
        valeur: &str,
        date_effet: NaiveDate,
        acteur: Uuid,
    ) -> Result<(), EntDbError> {
        let courant = PgEleveRepo::get_pour_maj(&mut *tx, etab, eleve)
            .await?
            .ok_or(DomainError::NotFound)?;
        let ouverte: Option<(Uuid, String, NaiveDate)> = sqlx::query_as(
            "SELECT id, valeur, date_debut FROM ent_eleve_periode
             WHERE etablissement_id = $1 AND eleve_id = $2 AND kind = $3 AND date_fin IS NULL FOR UPDATE",
        )
        .bind(etab)
        .bind(eleve)
        .bind(kind.as_str())
        .fetch_optional(&mut **tx)
        .await?;
        let ouverte = ouverte.map(|(id, v, d)| ElevePeriode {
            id,
            etablissement_id: etab,
            eleve_id: eleve,
            kind,
            valeur: v,
            date_debut: d,
            date_fin: None,
        });
        let jour = Repo::jour(&mut *tx, etab).await?;
        let plan = domain_eleve::plan_cutover(
            &courant,
            ouverte.as_ref(),
            ChangeAttribut {
                etablissement_id: etab,
                eleve_id: eleve,
                kind,
                nouvelle_valeur: valeur.to_string(),
                date_effet,
                actor_user_id: Some(acteur.to_string()),
            },
            jour,
        )?;
        match &plan.op {
            CutoverOp::Noop => return Ok(()),
            CutoverOp::ReplaceOpen { period_id, nouvelle_valeur } => {
                sqlx::query("UPDATE ent_eleve_periode SET valeur = $3 WHERE etablissement_id = $1 AND id = $2 AND date_fin IS NULL")
                    .bind(etab)
                    .bind(period_id)
                    .bind(nouvelle_valeur)
                    .execute(&mut **tx)
                    .await?;
            }
            CutoverOp::CloseAndOpen { close_id, close_date_fin, open } => {
                sqlx::query("UPDATE ent_eleve_periode SET date_fin = $3 WHERE etablissement_id = $1 AND id = $2 AND date_fin IS NULL")
                    .bind(etab)
                    .bind(close_id)
                    .bind(close_date_fin)
                    .execute(&mut **tx)
                    .await?;
                crate::periode_repo::PgPeriodeRepo::ouvrir_premiere_sur(&mut *tx, open).await?;
            }
            CutoverOp::OpenFirst { period } => {
                crate::periode_repo::PgPeriodeRepo::ouvrir_premiere_sur(&mut *tx, period).await?;
            }
        }
        PgEleveRepo::upsert_sur(&mut *tx, &plan.eleve).await?;
        if let Some(e) = plan.event {
            tx.publier(e).await?;
        }
        journal(
            tx,
            etab,
            eleve,
            &format!("{}_change", kind.as_str()),
            "Changement daté de la scolarité",
            json!({ "kind": kind.as_str(), "date_effet": date_effet.to_string() }),
            true,
            Some(acteur),
        )
        .await?;
        Ok(())
    }

    /// ELV-09 : change l'élève de classe à une date (période + affectation au référentiel).
    pub async fn changer_classe(&self, role: &str, cmd: ChangementClasse) -> Result<(), EntDbError> {
        self.exiger_scolarite(cmd.etablissement_id, cmd.acteur, role, Action::ElevesScolariteGerer).await?;
        let (etab, eleve) = (cmd.etablissement_id, cmd.eleve_id);
        let mut tx = self.ouvrir_tx_eleve(etab, eleve).await?;
        let classe: Option<(String, Uuid, NaiveDate, NaiveDate)> = sqlx::query_as(
            "SELECT c.code, c.annee_id, a.date_debut, a.date_fin FROM ent_classe c
             JOIN ent_annee_scolaire a ON a.id = c.annee_id
             WHERE c.etablissement_id = $1 AND c.id = $2",
        )
        .bind(etab)
        .bind(cmd.classe_id)
        .fetch_optional(&mut *tx)
        .await?;
        let (code, annee, debut, fin) = classe.ok_or(EntDbError::SaisieInvalide("classe inconnue"))?;
        if cmd.date_effet < debut || cmd.date_effet > fin {
            return Err(EntDbError::SaisieInvalide("la date d'effet est hors de l'année de cette classe"));
        }
        let jour = Repo::jour(&mut tx, etab).await?;
        if Repo::statuts(&mut tx, etab, eleve).await?.0.as_deref() == Some("sorti")
            && Repo::periodes(&mut tx, etab, eleve).await?.iter().all(|p| !p.couvre(cmd.date_effet))
            && cmd.date_effet <= jour
        {
            return Err(EntDbError::ConflitDossier("l'élève n'est pas scolarisé à cette date"));
        }
        Self::ecrire_changement_attribut(&mut tx, etab, eleve, AttributKind::Classe, &code, cmd.date_effet, cmd.acteur)
            .await?;
        // Affectation de classe de l'année : l'ouverte est close la veille (ou remplacée si elle
        // démarre le jour même), puis la nouvelle est ouverte.
        let remplacee = sqlx::query(
            "UPDATE ent_affectation SET classe_id = $4, source = 'saisie'
             WHERE etablissement_id = $1 AND eleve_id = $2 AND annee_id = $3
               AND classe_id IS NOT NULL AND date_fin IS NULL AND date_debut >= $5",
        )
        .bind(etab)
        .bind(eleve)
        .bind(annee)
        .bind(cmd.classe_id)
        .bind(cmd.date_effet)
        .execute(&mut *tx)
        .await?;
        if remplacee.rows_affected() == 0 {
            sqlx::query(
                "UPDATE ent_affectation SET date_fin = $4
                 WHERE etablissement_id = $1 AND eleve_id = $2 AND annee_id = $3
                   AND classe_id IS NOT NULL AND date_fin IS NULL",
            )
            .bind(etab)
            .bind(eleve)
            .bind(annee)
            .bind(cmd.date_effet - Duration::days(1))
            .execute(&mut *tx)
            .await
            .map_err(traduire)?;
            sqlx::query(
                "INSERT INTO ent_affectation (id, etablissement_id, annee_id, eleve_id, classe_id, date_debut, source)
                 VALUES ($1,$2,$3,$4,$5,$6,'saisie')",
            )
            .bind(Uuid::new_v4())
            .bind(etab)
            .bind(annee)
            .bind(eleve)
            .bind(cmd.classe_id)
            .bind(cmd.date_effet)
            .execute(&mut *tx)
            .await
            .map_err(traduire)?;
        }
        tx.commit().await?;
        Ok(())
    }

    /// ELV-09 : ajoute et retire des options (= groupes du référentiel) à une date.
    pub async fn changer_options(&self, role: &str, cmd: ChangementOptions) -> Result<(), EntDbError> {
        self.exiger_scolarite(cmd.etablissement_id, cmd.acteur, role, Action::ElevesScolariteGerer).await?;
        if cmd.ajouter.is_empty() && cmd.retirer.is_empty() {
            return Err(EntDbError::SaisieInvalide("aucune option à ajouter ni à retirer"));
        }
        if cmd.ajouter.len() + cmd.retirer.len() > 30 {
            return Err(EntDbError::SaisieInvalide("trop d'options en une fois"));
        }
        let (etab, eleve) = (cmd.etablissement_id, cmd.eleve_id);
        let mut tx = self.ouvrir_tx_eleve(etab, eleve).await?;
        let an: Option<(NaiveDate, NaiveDate)> =
            sqlx::query_as("SELECT date_debut, date_fin FROM ent_annee_scolaire WHERE etablissement_id = $1 AND id = $2")
                .bind(etab)
                .bind(cmd.annee_id)
                .fetch_optional(&mut *tx)
                .await?;
        let (debut, fin) = an.ok_or(EntDbError::SaisieInvalide("année scolaire inconnue"))?;
        if cmd.date_effet < debut || cmd.date_effet > fin {
            return Err(EntDbError::SaisieInvalide("la date d'effet est hors de l'année"));
        }
        for groupe in cmd.ajouter.iter().chain(cmd.retirer.iter()) {
            let ok: Option<(Uuid,)> = sqlx::query_as(
                "SELECT id FROM ent_groupe WHERE etablissement_id = $1 AND annee_id = $2 AND id = $3",
            )
            .bind(etab)
            .bind(cmd.annee_id)
            .bind(groupe)
            .fetch_optional(&mut *tx)
            .await?;
            ok.ok_or(EntDbError::SaisieInvalide("groupe inconnu pour cette année"))?;
        }
        for groupe in &cmd.ajouter {
            let deja: Option<(Uuid,)> = sqlx::query_as(
                "SELECT id FROM ent_affectation WHERE etablissement_id = $1 AND eleve_id = $2 AND groupe_id = $3
                   AND date_debut <= $4 AND (date_fin IS NULL OR date_fin >= $4)",
            )
            .bind(etab)
            .bind(eleve)
            .bind(groupe)
            .bind(cmd.date_effet)
            .fetch_optional(&mut *tx)
            .await?;
            if deja.is_some() {
                continue;
            }
            sqlx::query(
                "INSERT INTO ent_affectation (id, etablissement_id, annee_id, eleve_id, groupe_id, date_debut, date_fin, source)
                 VALUES ($1,$2,$3,$4,$5,$6,$7,'saisie')",
            )
            .bind(Uuid::new_v4())
            .bind(etab)
            .bind(cmd.annee_id)
            .bind(eleve)
            .bind(groupe)
            .bind(cmd.date_effet)
            .bind(fin)
            .execute(&mut *tx)
            .await
            .map_err(traduire)?;
            evenement(
                &mut tx,
                etab,
                eleve,
                types::ELEVE_OPTION_ADDED,
                json!({ "groupe_id": groupe, "annee_id": cmd.annee_id, "date_effet": cmd.date_effet.to_string(), "action": "ajoutee" }),
                Some(cmd.acteur),
            )
            .await?;
        }
        for groupe in &cmd.retirer {
            let veille = cmd.date_effet - Duration::days(1);
            let r = sqlx::query(
                "UPDATE ent_affectation SET date_fin = greatest(date_debut, $4)
                 WHERE etablissement_id = $1 AND eleve_id = $2 AND groupe_id = $3
                   AND (date_fin IS NULL OR date_fin > $4)",
            )
            .bind(etab)
            .bind(eleve)
            .bind(groupe)
            .bind(veille)
            .execute(&mut *tx)
            .await?;
            if r.rows_affected() > 0 {
                evenement(
                    &mut tx,
                    etab,
                    eleve,
                    types::ELEVE_OPTION_ADDED,
                    json!({ "groupe_id": groupe, "annee_id": cmd.annee_id, "date_effet": cmd.date_effet.to_string(), "action": "retiree" }),
                    Some(cmd.acteur),
                )
                .await?;
            }
        }
        journal(
            &mut tx,
            etab,
            eleve,
            "options_changees",
            "Options modifiées",
            json!({ "ajoutees": cmd.ajouter.len(), "retirees": cmd.retirer.len(), "annee_id": cmd.annee_id }),
            true,
            Some(cmd.acteur),
        )
        .await?;
        tx.commit().await?;
        Ok(())
    }

    // ───────────────────── ELV-10 : régime et site, dates autorisées ─────────────────────

    /// Réglage des dates de changement de régime de l'établissement (R-E5).
    pub async fn regle_dates_changement_regime(&self, etab: Uuid, annee_jour: NaiveDate) -> Result<DatesChangement, EntDbError> {
        let p: Option<(String, Vec<NaiveDate>)> = sqlx::query_as(
            "SELECT dates_changement, dates FROM ent_param_eleve_regime WHERE etablissement_id = $1",
        )
        .bind(etab)
        .fetch_optional(&mut *TenantTx::begin(self.pool(), etab).await?)
        .await?;
        let Some((mode, dates)) = p else { return Ok(DatesChangement::Libre) };
        let fins = |kind: Option<&str>| {
            let kind = kind.map(String::from);
            let pool = self.pool().clone();
            async move {
                let mut tx = TenantTx::begin(&pool, etab).await?;
                let l: Vec<(NaiveDate,)> = sqlx::query_as(
                    "SELECT p.date_fin FROM ent_periode p
                     JOIN ent_annee_scolaire a ON a.id = p.annee_id
                     WHERE p.etablissement_id = $1 AND a.etablissement_id = $1
                       AND a.date_debut <= $2 AND a.date_fin >= $2 AND ($3::text IS NULL OR p.kind = $3)
                     ORDER BY p.date_fin",
                )
                .bind(etab)
                .bind(annee_jour)
                .bind(kind)
                .fetch_all(&mut *tx)
                .await?;
                Ok::<_, sqlx::Error>(l.into_iter().map(|(d,)| d).collect::<Vec<_>>())
            }
        };
        Ok(match mode.as_str() {
            "fin_de_periode" => DatesChangement::FinDePeriode(fins(None).await?),
            "fin_de_trimestre" => DatesChangement::FinDeTrimestre(fins(Some("trimestre")).await?),
            "liste" => DatesChangement::Liste(dates),
            _ => DatesChangement::Libre,
        })
    }

    /// Prochaine date de changement de régime autorisée à partir de `depuis` (proposée à l'écran).
    pub async fn prochaine_date_changement_regime(&self, etab: Uuid, depuis: NaiveDate) -> Result<Option<NaiveDate>, EntDbError> {
        let regle = self.regle_dates_changement_regime(etab, depuis).await?;
        Ok(prochaine_date_autorisee(&regle, depuis))
    }

    /// ELV-10 : change le régime ou le site à une date. Régime : la date doit être une date
    /// autorisée par l'établissement, sinon une dérogation motivée (motif codé, tracée).
    pub async fn changer_regime_ou_site(&self, role: &str, cmd: ChangementRegimeSite) -> Result<(), EntDbError> {
        self.exiger_scolarite(cmd.etablissement_id, cmd.acteur, role, Action::ElevesScolariteGerer).await?;
        if !matches!(cmd.kind, AttributKind::Regime | AttributKind::Site) {
            return Err(EntDbError::SaisieInvalide("seuls le régime et le site se changent ici"));
        }
        let valeur = cmd.valeur.trim();
        if valeur.is_empty() || valeur.chars().count() > 60 {
            return Err(EntDbError::SaisieInvalide("valeur invalide (1 à 60 caractères)"));
        }
        let (etab, eleve) = (cmd.etablissement_id, cmd.eleve_id);
        let derogation = if cmd.kind == AttributKind::Regime {
            let regle = self.regle_dates_changement_regime(etab, cmd.date_effet).await?;
            if date_changement_autorisee(&regle, cmd.date_effet) {
                None
            } else if let Some(m) = cmd.derogation {
                Some(m)
            } else {
                return Err(EntDbError::ConflitDossier(
                    "date de changement de régime non autorisée : choisir une date autorisée ou motiver une dérogation",
                ));
            }
        } else {
            None
        };
        let mut tx = self.ouvrir_tx_eleve(etab, eleve).await?;
        Self::ecrire_changement_attribut(&mut tx, etab, eleve, cmd.kind, valeur, cmd.date_effet, cmd.acteur).await?;
        if let Some(m) = derogation {
            // Dérogation tracée : motif codé, jamais de détail libre (raison médicale sans détail).
            journal(
                &mut tx,
                etab,
                eleve,
                "regime_derogation",
                "Changement de régime par dérogation",
                json!({ "motif": m.code(), "date_effet": cmd.date_effet.to_string() }),
                false,
                Some(cmd.acteur),
            )
            .await?;
        }
        tx.commit().await?;
        Ok(())
    }

    // ───────────────────── ELV-13 : capacité juridique ─────────────────────

    /// ELV-13 ★ : déclare une capacité (émancipation, mesure de protection) — pièce obligatoire.
    pub async fn declarer_capacite(&self, role: &str, cmd: DeclarationCapacite) -> Result<CapaciteEleve, EntDbError> {
        self.exiger_scolarite(cmd.etablissement_id, cmd.acteur, role, Action::ElevesIdentiteModifier).await?;
        if cmd.motif.as_deref().is_some_and(|m| m.chars().count() > 500) {
            return Err(EntDbError::SaisieInvalide("motif trop long (500 caractères)"));
        }
        let (etab, eleve) = (cmd.etablissement_id, cmd.eleve_id);
        let mut tx = self.ouvrir_tx_eleve(etab, eleve).await?;
        if !Repo::piece_de_l_eleve(&mut tx, etab, eleve, cmd.piece_document_id).await? {
            return Err(EntDbError::SaisieInvalide("pièce justificative inconnue ou non admise pour cet élève"));
        }
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_eleve_capacite (id, etablissement_id, eleve_id, nature, du, piece_document_id, motif, saisi_par)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8)",
        )
        .bind(id)
        .bind(etab)
        .bind(eleve)
        .bind(cmd.nature.code())
        .bind(cmd.du)
        .bind(cmd.piece_document_id)
        .bind(&cmd.motif)
        .bind(cmd.acteur)
        .execute(&mut *tx)
        .await
        .map_err(traduire)?;
        evenement(
            &mut tx,
            etab,
            eleve,
            types::ELEVE_CAPACITE_CHANGEE,
            json!({ "capacite_id": id, "nature": cmd.nature.code(), "action": "declaree" }),
            Some(cmd.acteur),
        )
        .await?;
        journal(&mut tx, etab, eleve, "capacite_declaree", "Capacité juridique déclarée",
            json!({ "capacite_id": id, "nature": cmd.nature.code() }), false, Some(cmd.acteur)).await?;
        let c = Self::capacite(&mut tx, etab, id).await?;
        tx.commit().await?;
        Ok(c)
    }

    async fn capacite(conn: &mut PgConnection, etab: Uuid, id: Uuid) -> Result<CapaciteEleve, sqlx::Error> {
        sqlx::query_as(
            "SELECT id, etablissement_id, eleve_id, nature, du, au, piece_document_id, motif, saisi_le, levee_le
             FROM ent_eleve_capacite WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(id)
        .fetch_one(&mut *conn)
        .await
    }

    /// ELV-13 ★ : lève une capacité déclarée (la ligne est conservée).
    pub async fn lever_capacite(
        &self,
        role: &str,
        etab: Uuid,
        eleve: Uuid,
        capacite: Uuid,
        au: Option<NaiveDate>,
        acteur: Uuid,
    ) -> Result<CapaciteEleve, EntDbError> {
        self.exiger_scolarite(etab, acteur, role, Action::ElevesIdentiteModifier).await?;
        let mut tx = self.ouvrir_tx_eleve(etab, eleve).await?;
        let c: CapaciteEleve = sqlx::query_as(
            "SELECT id, etablissement_id, eleve_id, nature, du, au, piece_document_id, motif, saisi_le, levee_le
             FROM ent_eleve_capacite WHERE etablissement_id = $1 AND eleve_id = $2 AND id = $3 FOR UPDATE",
        )
        .bind(etab)
        .bind(eleve)
        .bind(capacite)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(DomainError::NotFound)?;
        if c.levee_le.is_some() {
            return Err(EntDbError::ConflitDossier("cette mesure est déjà levée"));
        }
        let jour = Repo::jour(&mut tx, etab).await?;
        let fin = au.unwrap_or(jour).max(c.du);
        sqlx::query(
            "UPDATE ent_eleve_capacite SET levee_le = now(), levee_par = $3, au = $4
             WHERE etablissement_id = $1 AND id = $2 AND levee_le IS NULL",
        )
        .bind(etab)
        .bind(capacite)
        .bind(acteur)
        .bind(fin)
        .execute(&mut *tx)
        .await
        .map_err(traduire)?;
        evenement(
            &mut tx,
            etab,
            eleve,
            types::ELEVE_CAPACITE_CHANGEE,
            json!({ "capacite_id": capacite, "nature": c.nature, "action": "levee" }),
            Some(acteur),
        )
        .await?;
        journal(&mut tx, etab, eleve, "capacite_levee", "Capacité juridique levée",
            json!({ "capacite_id": capacite, "nature": c.nature }), false, Some(acteur)).await?;
        let c = Self::capacite(&mut tx, etab, capacite).await?;
        tx.commit().await?;
        Ok(c)
    }

    // ───────────────────── ELV-35 / ELV-36 : mobilité, modalités ─────────────────────

    /// ELV-35 : ouvre une période de mobilité (la scolarisation n'est PAS close).
    pub async fn ouvrir_mobilite(
        &self,
        role: &str,
        etab: Uuid,
        eleve: Uuid,
        type_: TypeMobilite,
        du: NaiveDate,
        retour_prevu: Option<NaiveDate>,
        acteur: Uuid,
    ) -> Result<MobiliteEleve, EntDbError> {
        self.exiger_scolarite(etab, acteur, role, Action::ElevesScolariteGerer).await?;
        let mut tx = self.ouvrir_tx_eleve(etab, eleve).await?;
        if Repo::periodes(&mut tx, etab, eleve).await?.iter().all(|p| !p.couvre(du)) {
            return Err(EntDbError::ConflitDossier("l'élève n'est pas scolarisé à cette date"));
        }
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_eleve_mobilite (id, etablissement_id, eleve_id, du, type, retour_prevu, cree_par)
             VALUES ($1,$2,$3,$4,$5,$6,$7)",
        )
        .bind(id)
        .bind(etab)
        .bind(eleve)
        .bind(du)
        .bind(type_.code())
        .bind(retour_prevu)
        .bind(acteur)
        .execute(&mut *tx)
        .await
        .map_err(traduire)?;
        evenement(&mut tx, etab, eleve, types::SCOLARITE_MOBILITE_OUVERTE,
            json!({ "mobilite_id": id, "type": type_.code() }), Some(acteur)).await?;
        journal(&mut tx, etab, eleve, "mobilite_ouverte", "Période de mobilité ouverte",
            json!({ "mobilite_id": id, "type": type_.code() }), false, Some(acteur)).await?;
        let m = sqlx::query_as::<_, MobiliteEleve>(
            "SELECT id, eleve_id, du, au, type AS type_, retour_prevu FROM ent_eleve_mobilite
             WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(id)
        .fetch_one(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(m)
    }

    /// ELV-35 : constate le retour (ferme la période de mobilité).
    pub async fn fermer_mobilite(
        &self,
        role: &str,
        etab: Uuid,
        eleve: Uuid,
        mobilite: Uuid,
        au: NaiveDate,
        acteur: Uuid,
    ) -> Result<(), EntDbError> {
        self.exiger_scolarite(etab, acteur, role, Action::ElevesScolariteGerer).await?;
        let mut tx = self.ouvrir_tx_eleve(etab, eleve).await?;
        let r = sqlx::query(
            "UPDATE ent_eleve_mobilite SET au = greatest(du, $4), ferme_le = now(), ferme_par = $5
             WHERE etablissement_id = $1 AND eleve_id = $2 AND id = $3 AND ferme_le IS NULL",
        )
        .bind(etab)
        .bind(eleve)
        .bind(mobilite)
        .bind(au)
        .bind(acteur)
        .execute(&mut *tx)
        .await
        .map_err(traduire)?;
        if r.rows_affected() != 1 {
            return Err(EntDbError::ConflitDossier("mobilité inconnue ou déjà fermée"));
        }
        evenement(&mut tx, etab, eleve, types::SCOLARITE_MOBILITE_FERMEE, json!({ "mobilite_id": mobilite }), Some(acteur)).await?;
        journal(&mut tx, etab, eleve, "mobilite_fermee", "Période de mobilité fermée",
            json!({ "mobilite_id": mobilite }), false, Some(acteur)).await?;
        tx.commit().await?;
        Ok(())
    }

    /// ELV-36 : enregistre une modalité de scolarisation datée.
    pub async fn enregistrer_modalite(
        &self,
        role: &str,
        etab: Uuid,
        eleve: Uuid,
        code: CodeModalite,
        du: NaiveDate,
        au: Option<NaiveDate>,
        piece_document_id: Option<Uuid>,
        acteur: Uuid,
    ) -> Result<ModaliteEleve, EntDbError> {
        self.exiger_scolarite(etab, acteur, role, Action::ElevesScolariteGerer).await?;
        let mut tx = self.ouvrir_tx_eleve(etab, eleve).await?;
        if let Some(p) = piece_document_id {
            if !Repo::piece_de_l_eleve(&mut tx, etab, eleve, p).await? {
                return Err(EntDbError::SaisieInvalide("pièce inconnue ou non admise pour cet élève"));
            }
        }
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_eleve_modalite (id, etablissement_id, eleve_id, code, du, au, piece_document_id, cree_par)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8)",
        )
        .bind(id)
        .bind(etab)
        .bind(eleve)
        .bind(code.code())
        .bind(du)
        .bind(au)
        .bind(piece_document_id)
        .bind(acteur)
        .execute(&mut *tx)
        .await
        .map_err(traduire)?;
        // « sans détail » : la modalité n'apparaît ni dans l'événement ni dans le journal.
        evenement(&mut tx, etab, eleve, types::SCOLARITE_MODALITE_CHANGEE,
            json!({ "modalite_id": id, "action": "enregistree" }), Some(acteur)).await?;
        journal(&mut tx, etab, eleve, "modalite_enregistree", "Modalité de scolarisation enregistrée",
            json!({ "modalite_id": id }), false, Some(acteur)).await?;
        let m = sqlx::query_as::<_, ModaliteEleve>(
            "SELECT id, eleve_id, code, du, au, source FROM ent_eleve_modalite WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(id)
        .fetch_one(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(m)
    }

    /// ELV-36 : met fin à une modalité.
    pub async fn terminer_modalite(
        &self,
        role: &str,
        etab: Uuid,
        eleve: Uuid,
        modalite: Uuid,
        au: NaiveDate,
        acteur: Uuid,
    ) -> Result<(), EntDbError> {
        self.exiger_scolarite(etab, acteur, role, Action::ElevesScolariteGerer).await?;
        let mut tx = self.ouvrir_tx_eleve(etab, eleve).await?;
        let r = sqlx::query(
            "UPDATE ent_eleve_modalite SET au = greatest(du, $4)
             WHERE etablissement_id = $1 AND eleve_id = $2 AND id = $3 AND au IS NULL",
        )
        .bind(etab)
        .bind(eleve)
        .bind(modalite)
        .bind(au)
        .execute(&mut *tx)
        .await
        .map_err(traduire)?;
        if r.rows_affected() != 1 {
            return Err(EntDbError::ConflitDossier("modalité inconnue ou déjà terminée"));
        }
        evenement(&mut tx, etab, eleve, types::SCOLARITE_MODALITE_CHANGEE,
            json!({ "modalite_id": modalite, "action": "terminee" }), Some(acteur)).await?;
        tx.commit().await?;
        Ok(())
    }

    // ───────────────────── Lectures (G1) ─────────────────────

    /// G1 : scolarisations et inscriptions d'un élève (`eleves.lire` sur l'élève), avec le
    /// statut dérivé au jour de l'établissement. Lecture journalisée (acteur et objet).
    pub async fn scolarite_pour_acteur(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        eleve: Uuid,
    ) -> Result<(String, Vec<ScolarisationEleve>, Vec<InscriptionEleve>), EntDbError> {
        if !self.autoriser_eleve(etab, acteur, role, Action::ElevesLire, eleve).await? {
            return Err(EntDbError::Forbidden);
        }
        let mut conn = TenantTx::begin(self.pool(), etab).await?;
        let existe: Option<(Uuid,)> =
            sqlx::query_as("SELECT id FROM ent_eleve WHERE etablissement_id = $1 AND id = $2")
                .bind(etab)
                .bind(eleve)
                .fetch_optional(&mut *conn)
                .await?;
        existe.ok_or(DomainError::NotFound)?;
        let jour = Repo::jour(&mut conn, etab).await?;
        let (statut,): (String,) = sqlx::query_as("SELECT ent_eleve_statut_a($1, $2, $3)")
            .bind(etab)
            .bind(eleve)
            .bind(jour)
            .fetch_one(&mut *conn)
            .await?;
        let sql = format!(
            "SELECT {COL_SCOL} FROM ent_eleve_scolarisation WHERE etablissement_id = $1 AND eleve_id = $2
             ORDER BY date_entree DESC"
        );
        let scol: Vec<ScolarisationEleve> = sqlx::query_as(&sql).bind(etab).bind(eleve).fetch_all(&mut *conn).await?;
        drop(conn);
        self.tracer_acces_sensible(etab, acteur, role, Some(eleve), "scolarite", "consultation",
            "Consultation de la scolarité de l'élève").await;
        let mut conn = TenantTx::begin(self.pool(), etab).await?;
        let sql = format!(
            "SELECT {COL_INSC} FROM ent_eleve_inscription WHERE etablissement_id = $1 AND eleve_id = $2
             ORDER BY date_debut DESC NULLS LAST"
        );
        let insc: Vec<InscriptionEleve> = sqlx::query_as(&sql).bind(etab).bind(eleve).fetch_all(&mut *conn).await?;
        Ok((statut, scol, insc))
    }

    /// Scolarité lue par un **responsable légal** (démo P1 n° 3). Droit : un lien ACTIF avec
    /// l'élève (même point que le reste du portail famille : `lien_parent_eleve`) et une
    /// personne qui n'est pas marquée « ne pas solliciter ». Sinon `Forbidden` (jamais
    /// `NotFound` : on ne révèle pas si l'élève existe). Minimisation A1 : la famille reçoit le
    /// statut, les dates et le motif d'entrée de chaque scolarisation et l'état de chaque
    /// inscription annuelle ; JAMAIS le motif de sortie (peut être disciplinaire), l'exeat,
    /// les établissements d'origine et de destination, ni la décision de passage (réservée au
    /// personnel tant qu'elle n'est pas communiquée par le conseil).
    pub async fn scolarite_pour_famille(
        &self,
        etab: Uuid,
        parent: Uuid,
        eleve: Uuid,
    ) -> Result<(String, Vec<ScolarisationEleve>, Vec<InscriptionEleve>, Vec<(Uuid, String)>), EntDbError> {
        let mut conn = TenantTx::begin(self.pool(), etab).await?;
        let lien: Option<(Uuid,)> = sqlx::query_as(
            "SELECT personne_id FROM ent_eleve_lien_actif
              WHERE etablissement_id = $1 AND user_id = $2 AND eleve_id = $3",
        )
        .bind(etab)
        .bind(parent)
        .bind(eleve)
        .fetch_optional(&mut *conn)
        .await?;
        let Some((personne_id,)) = lien else {
            return Err(EntDbError::Forbidden);
        };
        if crate::dossier_personnes::personne_ne_pas_solliciter_tx(&mut conn, etab, personne_id).await? {
            return Err(EntDbError::Forbidden);
        }
        let jour = Repo::jour(&mut conn, etab).await?;
        let (statut,): (String,) = sqlx::query_as("SELECT ent_eleve_statut_a($1, $2, $3)")
            .bind(etab)
            .bind(eleve)
            .bind(jour)
            .fetch_one(&mut *conn)
            .await?;
        let sql = format!(
            "SELECT {COL_SCOL} FROM ent_eleve_scolarisation WHERE etablissement_id = $1 AND eleve_id = $2
             ORDER BY date_entree DESC"
        );
        let mut scol: Vec<ScolarisationEleve> =
            sqlx::query_as(&sql).bind(etab).bind(eleve).fetch_all(&mut *conn).await?;
        for s in &mut scol {
            s.origine_uai = None;
            s.origine_libelle = None;
            s.origine_type = None;
            s.motif_sortie = None;
            s.motif_sortie_libre = None;
            s.destination_uai = None;
            s.destination_libelle = None;
            s.destination_type = None;
            s.exeat_document_id = None;
        }
        let sql = format!(
            "SELECT {COL_INSC} FROM ent_eleve_inscription WHERE etablissement_id = $1 AND eleve_id = $2
             ORDER BY date_debut DESC NULLS LAST"
        );
        let mut insc: Vec<InscriptionEleve> =
            sqlx::query_as(&sql).bind(etab).bind(eleve).fetch_all(&mut *conn).await?;
        for i in &mut insc {
            i.decision_passage = None;
        }
        let ids: Vec<Uuid> = insc.iter().map(|i| i.annee_id).collect();
        let annees: Vec<(Uuid, String)> = sqlx::query_as(
            "SELECT id, libelle FROM ent_annee_scolaire WHERE etablissement_id = $1 AND id = ANY($2)",
        )
        .bind(etab)
        .bind(&ids)
        .fetch_all(&mut *conn)
        .await?;
        Ok((statut, scol, insc, annees))
    }

    /// Statut dérivé d'un élève à un jour (usage interne : autres domaines, socle 3).
    pub async fn statut_eleve_a(&self, etab: Uuid, eleve: Uuid, jour: NaiveDate) -> Result<String, EntDbError> {
        let (s,): (String,) = sqlx::query_as("SELECT ent_eleve_statut_a($1, $2, $3)")
            .bind(etab)
            .bind(eleve)
            .bind(jour)
            .fetch_one(&mut *TenantTx::begin(self.pool(), etab).await?)
            .await?;
        Ok(s)
    }

    // ───────────────────── Tâche quotidienne ─────────────────────

    /// Tâche quotidienne (idempotente ; lancée toutes les heures, car « le jour » dépend du
    /// fuseau de chaque établissement) :
    /// 1. versions d'identité annoncées devenues effectives ;
    /// 2. statuts dérivés changés avec la date (entrée ou sortie atteinte) → `eleve.status_changed`,
    ///    répercussions de sortie ;
    /// 3. majorité (ELV-14) : `eleve.majeur` le jour des 18 ans, `eleve.majorite_proche` à J-30
    ///    (ELV-39) — une seule fois par élève (marqueur), sans valeur d'identité.
    pub async fn tache_nocturne_eleves(&self) -> Result<BilanTacheEleves, EntDbError> {
        let mut bilan = BilanTacheEleves {
            identites_resynchronisees: self.resynchroniser_identites().await?,
            ..Default::default()
        };
        // — Statuts dérivés —
        let changes: Vec<(Uuid, Uuid, Option<String>, String)> = crate::tenant_tx::appeler_par_etablissement(
            self.pool(),
            ContexteSysteme::TacheNocturneEleves,
            "SELECT etablissement_id, eleve_id, ancien, nouveau FROM ent_eleve_statut_recalculer()",
        )
        .await?;
        for (etab, eleve, ancien, nouveau) in changes {
            // Un échec sur un élève ne bloque ni les autres ni les autres établissements.
            let r: Result<(), EntDbError> = async {
                let mut tx = TenantTx::begin(self.pool(), etab).await?;
                let (_, legacy): (Option<String>, String) = Repo::statuts(&mut tx, etab, eleve).await?;
                evenement(
                    &mut tx,
                    etab,
                    eleve,
                    types::ELEVE_STATUS_CHANGED,
                    json!({
                        "ancien_statut": ancien,
                        "nouveau_statut": nouveau,
                        "nouveau_status": legacy,
                        "cause": "tache_quotidienne",
                    }),
                    None,
                )
                .await?;
                tx.commit().await?;
                Ok(())
            }
            .await;
            match r {
                Ok(()) => bilan.statuts_changes += 1,
                Err(e) => tracing::warn!(error = %e, "tâche quotidienne : événement de statut non publié"),
            }
        }
        // Répercussions de sortie (idempotentes) : on rejoue les sorties des 3 derniers jours, pour
        // qu'un échec ponctuel ou une sortie datée dans le futur ne soit jamais perdu.
        let sorties: Vec<(Uuid, Uuid, NaiveDate)> = lire_par_etablissement(
            self.pool(),
            ContexteSysteme::TacheNocturneEleves,
            "SELECT e.etablissement_id, e.id, max(s.date_sortie)
             FROM ent_eleve e JOIN ent_eleve_scolarisation s
               ON s.etablissement_id = e.etablissement_id AND s.eleve_id = e.id
             WHERE e.statut = 'sorti' AND s.date_sortie IS NOT NULL
               AND s.date_sortie >= ent_jour_etablissement(e.etablissement_id) - 3
               AND s.date_sortie <= ent_jour_etablissement(e.etablissement_id)
             GROUP BY e.etablissement_id, e.id",
        )
        .await?;
        for (etab, eleve, date) in sorties {
            match self.appliquer_repercussions_sortie(etab, eleve, date, None).await {
                Ok(()) => bilan.repercussions_sortie += 1,
                Err(e) => tracing::warn!(error = %e, "tâche quotidienne : répercussions de sortie à rejouer"),
            }
        }
        // — Majorité —
        let dues: Vec<(Uuid, Uuid, String, NaiveDate, NaiveDate, bool)> = lire_par_etablissement(
            self.pool(),
            ContexteSysteme::TacheNocturneEleves,
            "SELECT etablissement_id, eleve_id, sorte, majorite_le, jour, presumee FROM ent_eleve_majorites_a_traiter()",
        )
        .await?;
        for (etab, eleve, sorte, majorite_le, jour, presumee) in dues {
            // Fenêtre de rattrapage : au-delà de 14 jours après le jour des 18 ans (première mise
            // en service, panne longue), on pose le marqueur sans rien publier.
            // R-09 : une ligne en échec ne bloque ni les autres élèves ni les autres établissements.
            let r: Result<(), EntDbError> = async {
                let publier = sorte == "proche" || majorite_le > jour - Duration::days(14);
                let mut tx = TenantTx::begin(self.pool(), etab).await?;
                let pose = sqlx::query(
                    "INSERT INTO ent_eleve_majorite_evenement (etablissement_id, eleve_id, sorte, majorite_le, publie)
                     VALUES ($1,$2,$3,$4,$5) ON CONFLICT (eleve_id, sorte) DO NOTHING",
                )
                .bind(etab)
                .bind(eleve)
                .bind(&sorte)
                .bind(majorite_le)
                .bind(publier)
                .execute(&mut *tx)
                .await?;
                if pose.rows_affected() == 1 && publier {
                    // Jamais la date : elle révélerait la date de naissance.
                    let (type_name, compteur) = if sorte == "atteinte" {
                        (types::ELEVE_MAJEUR, &mut bilan.majeurs)
                    } else {
                        (types::ELEVE_MAJORITE_PROCHE, &mut bilan.majorites_proches)
                    };
                    evenement(&mut tx, etab, eleve, type_name, json!({ "date_presumee": presumee }), None).await?;
                    *compteur += 1;
                } else if pose.rows_affected() == 1 {
                    bilan.marqueurs_silencieux += 1;
                }
                tx.commit().await?;
                Ok(())
            }
            .await;
            if let Err(e) = r {
                crate::tenant_tx::signaler_echec_etablissement("tache_nocturne_majorites", &e);
            }
        }
        Ok(bilan)
    }
}
