//! Socle 15, lot A5 — absence longue et suppléance
//! (`docs/socles/15_ABSENCES_PERSONNELS.md` §11.4, R-18, CAP-36, CAP-37, AR-23 ; ABP-180, 181, 239 ; EDT-228).
//!
//! Deux notions de remplacement existaient sans lien (AR-23) : la **réponse de séance** (RCD, collègue : lot A4,
//! socle 10) et la **suppléance de service** (intervention datée, socle 6). Ce module les relie sans les
//! confondre : la suppléance reste un fait du socle 6 (`ent_service_intervenant`, `origine = 'absence'`,
//! `origine_ref` = l'absence) ; le 15 en dérive l'état de ses impacts (`supplee`) et l'état du remplacement
//! publié aux familles.
//!
//! | Commande                                  | Droit (D-017)                                              | ABP |
//! |-------------------------------------------|------------------------------------------------------------|-----|
//! | `demander_suppleance_for_actor`           | `personnels.absences.suppleer` + lecture d'organisation    | 180 |
//! | `repondre_demande_suppleance_for_actor`   | idem (consigner la réponse de l'employeur)                 | 180 |
//! | `nommer_suppleant_absence_for_actor`      | `suppleer` + `valider` (direction) + `services.gerer` (6)  | 181 |
//! | `synchroniser_remplacement_tx` / tâche    | système (jamais une commande d'utilisateur)                | 239 |
//!
//! **Garanties.**
//! * **Le fait du socle 6 est la source** : l'état du remplacement et les impacts `supplee` sont *calculés* à partir
//!   des interventions `suppleant` actives de l'absence ; une suppléance nommée par la route du socle 6 avec
//!   `origine_ref` = l'absence produit les mêmes effets qu'une nomination faite ici (idempotent, rejouable).
//! * **Une notification par étape** (EDT-228, ABP-239) : le changement d'étape et l'événement sont écrits dans la
//!   même transaction, sous le verrou de l'absence ; rejouer la synchronisation ne publie rien de plus.
//! * **CAP-36** : seuls les impacts encore **sans réponse** à partir de la date de suppléance passent `supplee` ;
//!   les deux premières semaines (réponses de séance RCD) restent telles qu'elles sont ; une séance déjà couverte
//!   par une autre réponse après la date est **comptée** (à lever par la direction), jamais écrasée.
//! * **CAP-37** : un suppléant engagé « en remplacement » dont la prise de fonctions n'est pas constatée
//!   (`installe_le`, socle 6) n'a pas encore de séance : elles restent `a_traiter`, l'état dit « à partir du … ».
//! * **Jamais la nature** : événements, journal et DTO ne portent que des identifiants, des dates et des codes.
//! * Une suppléance terminée plus tôt (socle 6) rend à `a_traiter` les séances qu'elle ne couvre plus.

use super::*;
use crate::tenant_tx::{etablissements_systeme, ContexteSysteme};

pub const DESTINATAIRES_SUPPLEANCE: &[&str] = &["rectorat", "ia_dpe", "diocese", "autre"];
const ISSUES_REPONSE: &[&str] = &["proposee", "refusee", "sans_suite"];
/// États d'absence pour lesquels une suppléance se demande ou se nomme : jamais un simple signalement
/// (R-AP46), une annulation, un refus ou une absence close.
const ETATS_SUPPLEABLES: &[&str] = &[
    "declaree", "en_instruction", "validee", "prise_en_compte", "decision_employeur", "contestee", "traitee",
    "prevue",
];
/// Réponses d'impact qui couvrent déjà une séance (une autre réponse que la suppléance).
const ETATS_DEJA_REPONDUS: &[&str] = &["remplace", "autre_cours", "deplace", "continuite", "annule"];

pub const EVT_SUPPLEANCE_DEMANDEE: &str = "absence_personnel.suppleance_demandee";
pub const EVT_SUPPLEANCE_REPONDUE: &str = "absence_personnel.suppleance_repondue";
pub const EVT_ETAT_REMPLACEMENT: &str = "absence_personnel.etat_remplacement";

// ─────────────────────────────── logique pure (testée sans base) ───────────────────────────────

/// Une suppléance active de l'absence, telle que le socle 6 la porte.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SuppleanceFait {
    pub agent_id: Uuid,
    pub du: NaiveDate,
    pub au: Option<NaiveDate>,
    /// Premier jour où le suppléant tient réellement les séances : `du`, ou la prise de fonctions si le suppléant
    /// doit être installé (CAP-37) ; `None` tant que son installation n'est pas constatée.
    pub debut_effectif: Option<NaiveDate>,
}

/// Premier jour de présence réelle du suppléant (CAP-37).
pub fn debut_effectif(du: NaiveDate, installation_requise: bool, installe_le: Option<NaiveDate>) -> Option<NaiveDate> {
    if !installation_requise {
        return Some(du);
    }
    installe_le.map(|i| i.max(du))
}

/// Faits dont l'étape se déduit.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FaitsRemplacement {
    pub suppleances: Vec<SuppleanceFait>,
    /// État de la dernière demande non annulée (`demandee`, `proposee`, `acceptee`, `refusee`, `sans_suite`).
    pub demande: Option<(String, Option<Uuid>, Option<NaiveDate>)>,
}

/// Étape du remplacement, suppléant et date affichés (« remplacé par M. Y à partir du … »).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EtapeCalculee {
    pub etape: &'static str,
    pub suppleant: Option<Uuid>,
    pub a_partir_du: Option<NaiveDate>,
}

/// Étape déduite des faits au jour `jour` :
/// * une suppléance dont la fin est passée pour toutes → `termine` ;
/// * une suppléance effective (installée) commencée → `en_cours` ; sinon (à venir, ou suppléant pas encore
///   installé) → `nomme`, avec la date « à partir du » ;
/// * pas de suppléance : une proposition de l'employeur → `propose` ; une demande sans proposition (en attente,
///   refusée, sans suite) → `en_recherche` ; rien → pas d'étape.
pub fn etape_remplacement(f: &FaitsRemplacement, jour: NaiveDate) -> Option<EtapeCalculee> {
    let vivantes: Vec<&SuppleanceFait> = f.suppleances.iter().filter(|s| s.au.is_none_or(|a| a >= jour)).collect();
    if !f.suppleances.is_empty() && vivantes.is_empty() {
        let derniere = f.suppleances.iter().max_by_key(|s| s.au)?;
        return Some(EtapeCalculee { etape: "termine", suppleant: Some(derniere.agent_id), a_partir_du: Some(derniere.du) });
    }
    if let Some(en_cours) = vivantes
        .iter()
        .filter(|s| s.debut_effectif.is_some_and(|d| d <= jour))
        .min_by_key(|s| s.du)
    {
        return Some(EtapeCalculee { etape: "en_cours", suppleant: Some(en_cours.agent_id), a_partir_du: Some(en_cours.du) });
    }
    if let Some(prochaine) = vivantes.iter().min_by_key(|s| s.du) {
        let date = prochaine.debut_effectif.map_or(prochaine.du, |d| d.max(prochaine.du));
        return Some(EtapeCalculee { etape: "nomme", suppleant: Some(prochaine.agent_id), a_partir_du: Some(date) });
    }
    match f.demande.as_ref() {
        Some((etat, agent, du)) if etat == "proposee" || etat == "acceptee" => {
            Some(EtapeCalculee { etape: "propose", suppleant: *agent, a_partir_du: *du })
        }
        Some(_) => Some(EtapeCalculee { etape: "en_recherche", suppleant: None, a_partir_du: None }),
        None => None,
    }
}

/// La période d'une suppléance recoupe-t-elle le jour d'une séance ?
pub fn suppleance_couvre(du: NaiveDate, au: Option<NaiveDate>, jour: NaiveDate) -> bool {
    jour >= du && au.is_none_or(|a| jour <= a)
}

// ─────────────────────────────── base ───────────────────────────────

#[derive(Debug, Clone, sqlx::FromRow)]
struct DemandeLigne {
    id: Uuid,
    absence_id: Uuid,
    destinataire: String,
    reference: Option<String>,
    souhaitee_du: Option<NaiveDate>,
    etat: String,
    demandee_le: DateTime<Utc>,
    reponse_le: Option<DateTime<Utc>>,
    suppleant_agent_id: Option<Uuid>,
    suppleant_du: Option<NaiveDate>,
}

const DEMANDE_COLONNES: &str = "id, absence_id, destinataire, reference, souhaitee_du, etat, demandee_le, reponse_le,
    suppleant_agent_id, suppleant_du";

impl From<DemandeLigne> for DemandeSuppleanceDto {
    fn from(d: DemandeLigne) -> Self {
        DemandeSuppleanceDto {
            id: d.id,
            absence_id: d.absence_id,
            destinataire: d.destinataire,
            reference: d.reference,
            souhaitee_du: d.souhaitee_du,
            etat: d.etat,
            demandee_le: d.demandee_le,
            reponse_le: d.reponse_le,
            suppleant_agent_id: d.suppleant_agent_id,
            suppleant_du: d.suppleant_du,
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct BilanSynchro {
    pub etape_changee: bool,
    pub seances_supplees: u32,
    pub seances_deja_repondues: u32,
}

async fn jour_etab(conn: &mut PgConnection, etab: Uuid) -> Result<NaiveDate, EntDbError> {
    sqlx::query_scalar("SELECT ent_aujourdhui_etab($1, now())")
        .bind(etab)
        .fetch_one(conn)
        .await
        .map_err(db)
}

async fn publier_etat(
    conn: &mut PgConnection,
    etab: Uuid,
    absence_id: Uuid,
    agent_absent: Uuid,
    e: &EtapeCalculee,
    actor: Option<Uuid>,
) -> Result<(), EntDbError> {
    conn.publier(MetierEventDraft {
        etablissement_id: etab,
        type_name: EVT_ETAT_REMPLACEMENT.to_string(),
        aggregate: AggregateKind::AbsenceRh,
        aggregate_id: absence_id,
        eleve_id: None,
        payload: json!({
            "absence_id": absence_id, "agent_id": agent_absent, "cause": "rh", "etape": e.etape,
            "suppleant_agent_id": e.suppleant, "a_partir_du": e.a_partir_du,
        }),
        actor_user_id: actor.map(|u| u.to_string()),
    })
    .await
    .map_err(EntDbError::from)?;
    Ok(())
}

fn dto_etat(absence_id: Uuid, l: (String, Option<Uuid>, Option<NaiveDate>, DateTime<Utc>)) -> EtatRemplacementDto {
    EtatRemplacementDto { absence_id, etape: l.0, suppleant_agent_id: l.1, a_partir_du: l.2, etape_le: l.3 }
}

impl PgEnt {
    /// Droit de demander ou de consigner une suppléance : l'action dédiée ET la lecture d'organisation.
    async fn exiger_suppleer(&self, etab: Uuid, actor: Uuid, role: &str) -> Result<(), EntDbError> {
        self.exiger_personnel_abs(role).await?;
        if !(self.a_droit_abs(etab, actor, role, Action::PersonnelsAbsencesSuppleer).await?
            && self.peut_lire_organisation(etab, actor, role).await?)
        {
            return Err(EntDbError::Forbidden);
        }
        Ok(())
    }

    /// Une absence supplée-able, lisible de l'appelant (confidentialité renforcée comprise), verrouillée.
    async fn absence_suppleable(
        &self,
        conn: &mut PgConnection,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        absence_id: Uuid,
    ) -> Result<AbsenceLigne, EntDbError> {
        let a = charger(conn, etab, absence_id, true).await?;
        let (renforcee,): (bool,) = sqlx::query_as(
            "SELECT confidentialite_renforcee FROM ent_absp_absence WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(absence_id)
        .fetch_one(&mut *conn)
        .await
        .map_err(db)?;
        if renforcee && !self.voit_renforcees(etab, actor, role).await? {
            return Err(introuvable());
        }
        if !ETATS_SUPPLEABLES.contains(&a.etat.as_str()) {
            return Err(conflit("cette absence ne permet pas de demander ou de nommer un suppléant dans son état actuel"));
        }
        Ok(a)
    }

    // ───────────────────────────── ABP-180 : demander une suppléance ─────────────────────────────

    /// Demande de suppléance à l'employeur (privé sous contrat : rectorat, R-18 ; public : IA-IPR / DPE). Trace de
    /// la demande ; une seule demande en cours par absence. Publie l'état « en recherche » (une fois).
    pub async fn demander_suppleance_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        absence_id: Uuid,
        body: &DemanderSuppleanceBody,
    ) -> Result<DemandeSuppleanceDto, EntDbError> {
        self.exiger_suppleer(etab, actor, role).await?;
        if !DESTINATAIRES_SUPPLEANCE.contains(&body.destinataire.as_str()) {
            return Err(invalide("Destinataire de la demande inconnu."));
        }
        let reference = texte_borne(&body.reference, 60, "La référence est limitée à 60 caractères.")?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let a = self.absence_suppleable(&mut tx, etab, actor, role, absence_id).await?;
        let id = Uuid::new_v4();
        let res = sqlx::query(
            "INSERT INTO ent_absp_demande_suppleance
                (id, etablissement_id, absence_id, destinataire, reference, souhaitee_du, demandee_par_user_id)
             VALUES ($1,$2,$3,$4,$5,$6,$7)",
        )
        .bind(id)
        .bind(etab)
        .bind(absence_id)
        .bind(&body.destinataire)
        .bind(&reference)
        .bind(body.souhaitee_du)
        .bind(actor)
        .execute(&mut *tx)
        .await;
        if let Err(e) = res {
            return Err(if est_conflit_unique(&e) {
                conflit("une demande de suppléance est déjà en cours pour cette absence")
            } else {
                db(e)
            });
        }
        journal(&mut tx, etab, absence_id, "suppleance_demandee", Some("destinataire"), None, Some(&body.destinataire), actor)
            .await?;
        evenement(&mut tx, etab, EVT_SUPPLEANCE_DEMANDEE, absence_id, a.agent_id, json!({ "demande_id": id }), actor).await?;
        self.synchroniser_remplacement_tx(&mut tx, etab, absence_id, Some(actor)).await?;
        let d = demande_tx(&mut tx, etab, id).await?;
        tx.commit().await.map_err(db)?;
        Ok(d.into())
    }

    /// Consigne la réponse de l'employeur : suppléant proposé (un agent de l'établissement), refus, sans suite.
    /// La proposition passe l'état du remplacement à « proposé » ; un refus le laisse « en recherche ».
    pub async fn repondre_demande_suppleance_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        demande_id: Uuid,
        body: &ReponseSuppleanceBody,
    ) -> Result<DemandeSuppleanceDto, EntDbError> {
        self.exiger_suppleer(etab, actor, role).await?;
        if !ISSUES_REPONSE.contains(&body.issue.as_str()) {
            return Err(invalide("Issue de la demande inconnue."));
        }
        if body.issue == "proposee" && body.suppleant_agent_id.is_none() {
            return Err(invalide("Une proposition désigne un agent de l'établissement."));
        }
        if body.issue != "proposee" && (body.suppleant_agent_id.is_some() || body.suppleant_du.is_some()) {
            return Err(invalide("Seule une proposition porte un suppléant."));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let d = demande_tx(&mut tx, etab, demande_id).await?;
        let a = self.absence_suppleable(&mut tx, etab, actor, role, d.absence_id).await?;
        if !matches!(d.etat.as_str(), "demandee" | "proposee") {
            return Err(conflit("cette demande a déjà reçu sa réponse définitive"));
        }
        if let Some(agent) = body.suppleant_agent_id {
            if agent == a.agent_id {
                return Err(invalide("L'absent ne peut pas être son propre suppléant."));
            }
            let existe: bool = sqlx::query_scalar(
                "SELECT EXISTS (SELECT 1 FROM ent_agent WHERE etablissement_id = $1 AND id = $2)",
            )
            .bind(etab)
            .bind(agent)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
            if !existe {
                return Err(introuvable());
            }
        }
        sqlx::query(
            "UPDATE ent_absp_demande_suppleance
                SET etat = $3, reponse_le = now(), reponse_par_user_id = $4, suppleant_agent_id = $5,
                    suppleant_du = $6, modifie_le = now()
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(demande_id)
        .bind(&body.issue)
        .bind(actor)
        .bind(body.suppleant_agent_id)
        .bind(body.suppleant_du)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        journal(&mut tx, etab, d.absence_id, "suppleance_reponse", Some("etat"), Some(&d.etat), Some(&body.issue), actor).await?;
        evenement(
            &mut tx,
            etab,
            EVT_SUPPLEANCE_REPONDUE,
            d.absence_id,
            a.agent_id,
            json!({ "demande_id": demande_id, "issue": body.issue }),
            actor,
        )
        .await?;
        self.synchroniser_remplacement_tx(&mut tx, etab, d.absence_id, Some(actor)).await?;
        let d = demande_tx(&mut tx, etab, demande_id).await?;
        tx.commit().await.map_err(db)?;
        Ok(d.into())
    }

    /// Demandes de suppléance d'une absence, la plus récente d'abord (organisation : lecture d'organisation).
    pub async fn demandes_suppleance_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        absence_id: Uuid,
    ) -> Result<Vec<DemandeSuppleanceDto>, EntDbError> {
        self.exiger_suppleer(etab, actor, role).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let a = charger(&mut tx, etab, absence_id, false).await?;
        let _ = a;
        let (renforcee,): (bool,) = sqlx::query_as(
            "SELECT confidentialite_renforcee FROM ent_absp_absence WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(absence_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        if renforcee && !self.voit_renforcees(etab, actor, role).await? {
            return Err(introuvable());
        }
        let lignes: Vec<DemandeLigne> = sqlx::query_as(&format!(
            "SELECT {DEMANDE_COLONNES} FROM ent_absp_demande_suppleance
              WHERE etablissement_id = $1 AND absence_id = $2 ORDER BY demandee_le DESC"
        ))
        .bind(etab)
        .bind(absence_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        Ok(lignes.into_iter().map(Into::into).collect())
    }

    // ───────────────────────────── ABP-181 : nommer un suppléant ─────────────────────────────

    /// Nomme un suppléant sur un service du titulaire absent : port `nommer_suppleant` du socle 6 (PRS-46), origine
    /// `absence`, `origine_ref` = l'absence. Les séances de l'absence encore sans réponse à partir de la date
    /// passent `supplee` ; la demande en cours est acceptée si le suppléant est celui que l'employeur proposait ;
    /// l'état du remplacement avance (une notification par étape).
    pub async fn nommer_suppleant_absence_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        absence_id: Uuid,
        body: &NommerSuppleantAbsenceBody,
    ) -> Result<NominationSuppleantDto, EntDbError> {
        self.exiger_suppleer(etab, actor, role).await?;
        // La direction décide (ABP-181) : l'action `valider` est la sienne, le secrétariat n'a pas celle-ci.
        if !self.a_droit_abs(etab, actor, role, Action::PersonnelsAbsencesValider).await? {
            return Err(EntDbError::Forbidden);
        }
        if let Some(au) = body.au {
            if au < body.du {
                return Err(invalide("La fin de la suppléance précède son début."));
            }
        }
        {
            let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
            let a = self.absence_suppleable(&mut tx, etab, actor, role, absence_id).await?;
            if body.agent_id == a.agent_id {
                return Err(invalide("L'absent ne peut pas être son propre suppléant."));
            }
            // Le service doit être celui de l'absent, à la date de début.
            let titulaire: Option<Uuid> = sqlx::query_scalar(
                "SELECT agent_id FROM ent_service_intervenant
                  WHERE etablissement_id = $1 AND service_id = $2 AND nature = 'titulaire' AND annule_le IS NULL
                    AND du <= $3 AND (au IS NULL OR au >= $3)",
            )
            .bind(etab)
            .bind(body.service_id)
            .bind(body.du)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db)?;
            if titulaire != Some(a.agent_id) {
                return Err(invalide("Ce service n'est pas assuré par l'agent absent à cette date."));
            }
            // La suppléance doit commencer pendant l'absence (la nature de la suppléance est une absence).
            let jour_fin_absence: NaiveDate = sqlx::query_scalar("SELECT ent_aujourdhui_etab($1, $2::timestamptz - interval '1 microsecond')")
                .bind(etab)
                .bind(a.fin)
                .fetch_one(&mut *tx)
                .await
                .map_err(db)?;
            if body.du > jour_fin_absence {
                return Err(invalide("La suppléance doit commencer pendant l'absence."));
            }
        }
        // Le socle 6 contrôle ses règles (recouvrement, une saisie par jour, service modifiable) et ses droits.
        self.nommer_suppleant_for_actor(
            etab,
            actor,
            role,
            body.service_id,
            &ent_types::personnels_services::NommerSuppleantBody {
                agent_id: body.agent_id,
                du: body.du,
                au: body.au,
                origine: Some("absence".to_string()),
                origine_ref: Some(absence_id),
            },
        )
        .await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let a = charger(&mut tx, etab, absence_id, true).await?;
        journal(&mut tx, etab, absence_id, "suppleant_nomme", Some("service"), None, Some(&body.service_id.to_string()), actor)
            .await?;
        evenement(
            &mut tx,
            etab,
            "absence_personnel.suppleant_nomme",
            absence_id,
            a.agent_id,
            json!({ "service_id": body.service_id, "suppleant_agent_id": body.agent_id, "du": body.du, "au": body.au }),
            actor,
        )
        .await?;
        // Acceptation de la demande en cours si c'est le suppléant proposé.
        sqlx::query(
            "UPDATE ent_absp_demande_suppleance SET etat = 'acceptee', modifie_le = now()
              WHERE etablissement_id = $1 AND absence_id = $2 AND etat = 'proposee' AND suppleant_agent_id = $3",
        )
        .bind(etab)
        .bind(absence_id)
        .bind(body.agent_id)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        let bilan = self.synchroniser_remplacement_tx(&mut tx, etab, absence_id, Some(actor)).await?;
        let (seances_supplees, seances_deja_repondues) = self
            .compter_service_impacts(&mut tx, etab, absence_id, body.service_id, body.du, body.au)
            .await?;
        let _ = bilan;
        let rem = etat_remplacement_tx(&mut tx, etab, absence_id).await?.ok_or_else(introuvable)?;
        tx.commit().await.map_err(db)?;
        Ok(NominationSuppleantDto {
            absence_id,
            service_id: body.service_id,
            agent_id: body.agent_id,
            du: body.du,
            au: body.au,
            seances_supplees,
            seances_deja_repondues,
            remplacement: rem,
        })
    }

    /// (séances `supplee`, séances déjà couvertes par une autre réponse) d'un service sur une période.
    async fn compter_service_impacts(
        &self,
        conn: &mut PgConnection,
        etab: Uuid,
        absence_id: Uuid,
        service_id: Uuid,
        du: NaiveDate,
        au: Option<NaiveDate>,
    ) -> Result<(u32, u32), EntDbError> {
        let lignes: Vec<(String, i64)> = sqlx::query_as(
            "SELECT im.etat, count(*) FROM ent_absp_impact im
               JOIN ent_edt_seance s ON s.etablissement_id = im.etablissement_id AND s.id = im.seance_id
               JOIN ent_service_enseignement sv ON sv.etablissement_id = im.etablissement_id AND sv.id = $3
                                               AND sv.cours_id = s.cours_id
              WHERE im.etablissement_id = $1 AND im.absence_id = $2 AND im.type = 'seance'
                AND s.jour >= $4 AND ($5::date IS NULL OR s.jour <= $5)
              GROUP BY im.etat",
        )
        .bind(etab)
        .bind(absence_id)
        .bind(service_id)
        .bind(du)
        .bind(au)
        .fetch_all(conn)
        .await
        .map_err(db)?;
        let mut supplees = 0u32;
        let mut deja = 0u32;
        for (etat, n) in lignes {
            if etat == "supplee" {
                supplees += n as u32;
            } else if ETATS_DEJA_REPONDUS.contains(&etat.as_str()) {
                deja += n as u32;
            }
        }
        Ok((supplees, deja))
    }

    /// L'état du remplacement d'une absence (ABP-239 : ce que voient les familles et l'EDT ; jamais la nature).
    pub async fn etat_remplacement_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        absence_id: Uuid,
    ) -> Result<Option<EtatRemplacementDto>, EntDbError> {
        self.exiger_personnel_abs(role).await?;
        if !self.peut_lire_organisation(etab, actor, role).await? {
            return Err(EntDbError::Forbidden);
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let (renforcee,): (bool,) = sqlx::query_as(
            "SELECT confidentialite_renforcee FROM ent_absp_absence WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(absence_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?
        .ok_or_else(introuvable)?;
        if renforcee && !self.voit_renforcees(etab, actor, role).await? {
            return Err(introuvable());
        }
        etat_remplacement_tx(&mut tx, etab, absence_id).await
    }

    // ───────────────────────────── ABP-239 : synchronisation et publication ─────────────────────────────

    /// Recalcule, à partir des **faits du socle 6** (interventions `suppleant` de l'absence) et de la demande :
    /// (1) les impacts `supplee` — sans réponse, à partir de la date effective, dans la période — et le retour à
    /// `a_traiter` de ceux qu'une suppléance écourtée ne couvre plus ; (2) l'étape du remplacement, publiée **une
    /// seule fois** (même transaction que le changement). Idempotente. Verrouille l'absence.
    pub(crate) async fn synchroniser_remplacement_tx(
        &self,
        conn: &mut PgConnection,
        etab: Uuid,
        absence_id: Uuid,
        actor: Option<Uuid>,
    ) -> Result<BilanSynchro, EntDbError> {
        let a = charger(conn, etab, absence_id, true).await?;
        let jour = jour_etab(conn, etab).await?;
        let lignes: Vec<(Uuid, Uuid, NaiveDate, Option<NaiveDate>, bool, Option<NaiveDate>)> = sqlx::query_as(
            "SELECT i.service_id, i.agent_id, i.du, i.au,
                    EXISTS (SELECT 1 FROM ent_agent_engagement e
                             WHERE e.etablissement_id = i.etablissement_id AND e.agent_id = i.agent_id
                               AND e.annule_le IS NULL AND e.motif_arrivee = 'remplacement'
                               AND (e.au IS NULL OR e.au >= i.du)),
                    (SELECT max(e.installe_le) FROM ent_agent_engagement e
                      WHERE e.etablissement_id = i.etablissement_id AND e.agent_id = i.agent_id
                        AND e.annule_le IS NULL AND e.motif_arrivee = 'remplacement'
                        AND (e.au IS NULL OR e.au >= i.du))
               FROM ent_service_intervenant i
              WHERE i.etablissement_id = $1 AND i.nature = 'suppleant' AND i.origine = 'absence'
                AND i.origine_ref = $2 AND i.annule_le IS NULL
              ORDER BY i.du, i.id",
        )
        .bind(etab)
        .bind(absence_id)
        .fetch_all(&mut *conn)
        .await
        .map_err(db)?;
        let mut bilan = BilanSynchro::default();
        let mut faits = FaitsRemplacement::default();
        // (service, du effectif, au) des suppléances qui tiennent déjà les séances.
        let mut couvertures: Vec<(Uuid, NaiveDate, Option<NaiveDate>)> = Vec::new();
        for (service_id, agent_id, du, au, installation_requise, installe_le) in lignes {
            let eff = debut_effectif(du, installation_requise, installe_le);
            faits.suppleances.push(SuppleanceFait { agent_id, du, au, debut_effectif: eff });
            if let Some(d) = eff {
                couvertures.push((service_id, d, au));
            }
        }
        // (1) Impacts : passage à `supplee` (sans réponse, séance à venir), sollicitations ouvertes annulées.
        for (service_id, d, au) in &couvertures {
            let ids: Vec<Uuid> = sqlx::query_scalar(
                "WITH cible AS (
                    SELECT im.id FROM ent_absp_impact im
                      JOIN ent_edt_seance s ON s.etablissement_id = im.etablissement_id AND s.id = im.seance_id
                      JOIN ent_service_enseignement sv ON sv.etablissement_id = im.etablissement_id AND sv.id = $3
                                                      AND sv.cours_id = s.cours_id
                     WHERE im.etablissement_id = $1 AND im.absence_id = $2 AND im.type = 'seance'
                       AND im.etat IN ('a_traiter', 'propose', 'sollicite') AND im.reponse_active_id IS NULL
                       AND s.jour >= $4 AND ($5::date IS NULL OR s.jour <= $5)
                       AND im.fin > now()
                       FOR UPDATE OF im)
                 UPDATE ent_absp_impact im SET etat = 'supplee', a_revoir = false, calcule_le = now()
                   FROM cible WHERE im.id = cible.id RETURNING im.id",
            )
            .bind(etab)
            .bind(absence_id)
            .bind(service_id)
            .bind(d)
            .bind(au)
            .fetch_all(&mut *conn)
            .await
            .map_err(db)?;
            if !ids.is_empty() {
                sqlx::query(
                    "UPDATE ent_absp_sollicitation SET etat = 'annulee', annulee_cause = 'levee', repondue_le = now()
                      WHERE etablissement_id = $1 AND impact_id = ANY($2) AND etat = 'envoyee'",
                )
                .bind(etab)
                .bind(&ids)
                .execute(&mut *conn)
                .await
                .map_err(db)?;
            }
            bilan.seances_supplees += ids.len() as u32;
        }
        // Retour à `a_traiter` des séances `supplee` qu'aucune suppléance ne couvre plus (suppléance écourtée ou
        // annulée au socle 6) : jamais de séance déclarée couverte alors que personne ne la tient.
        let supplees: Vec<(Uuid, NaiveDate, Option<Uuid>)> = sqlx::query_as(
            "SELECT im.id, s.jour, sv.id
               FROM ent_absp_impact im
               JOIN ent_edt_seance s ON s.etablissement_id = im.etablissement_id AND s.id = im.seance_id
               LEFT JOIN ent_service_enseignement sv ON sv.etablissement_id = im.etablissement_id AND sv.cours_id = s.cours_id
              WHERE im.etablissement_id = $1 AND im.absence_id = $2 AND im.type = 'seance' AND im.etat = 'supplee'
                AND im.fin > now()",
        )
        .bind(etab)
        .bind(absence_id)
        .fetch_all(&mut *conn)
        .await
        .map_err(db)?;
        let mut a_rendre: Vec<Uuid> = Vec::new();
        for (impact_id, jour_seance, service) in &supplees {
            let couvert = service.is_some_and(|sid| {
                couvertures.iter().any(|(cs, d, au)| *cs == sid && suppleance_couvre(*d, *au, *jour_seance))
            });
            if !couvert {
                a_rendre.push(*impact_id);
            }
        }
        if !a_rendre.is_empty() {
            sqlx::query(
                "UPDATE ent_absp_impact SET etat = 'a_traiter', calcule_le = now()
                  WHERE etablissement_id = $1 AND id = ANY($2) AND etat = 'supplee'",
            )
            .bind(etab)
            .bind(&a_rendre)
            .execute(&mut *conn)
            .await
            .map_err(db)?;
        }
        // Séances de la période déjà couvertes par une autre réponse (à lever par la direction).
        for (service_id, d, au) in &couvertures {
            let (_, deja) = self.compter_service_impacts(conn, etab, absence_id, *service_id, *d, *au).await?;
            bilan.seances_deja_repondues += deja;
        }
        // (2) Étape du remplacement.
        faits.demande = sqlx::query_as::<_, (String, Option<Uuid>, Option<NaiveDate>)>(
            "SELECT etat, suppleant_agent_id, suppleant_du FROM ent_absp_demande_suppleance
              WHERE etablissement_id = $1 AND absence_id = $2 AND etat <> 'annulee'
              ORDER BY demandee_le DESC, id DESC LIMIT 1",
        )
        .bind(etab)
        .bind(absence_id)
        .fetch_optional(&mut *conn)
        .await
        .map_err(db)?;
        let Some(e) = etape_remplacement(&faits, jour) else {
            return Ok(bilan);
        };
        let actuel: Option<(Uuid, String)> = sqlx::query_as(
            "SELECT id, etape FROM ent_absp_remplacement WHERE etablissement_id = $1 AND absence_id = $2 FOR UPDATE",
        )
        .bind(etab)
        .bind(absence_id)
        .fetch_optional(&mut *conn)
        .await
        .map_err(db)?;
        match actuel {
            None => {
                sqlx::query(
                    "INSERT INTO ent_absp_remplacement (id, etablissement_id, absence_id, etape, suppleant_agent_id, a_partir_du)
                     VALUES ($1,$2,$3,$4,$5,$6)",
                )
                .bind(Uuid::new_v4())
                .bind(etab)
                .bind(absence_id)
                .bind(e.etape)
                .bind(e.suppleant)
                .bind(e.a_partir_du)
                .execute(&mut *conn)
                .await
                .map_err(db)?;
                publier_etat(conn, etab, absence_id, a.agent_id, &e, actor).await?;
                bilan.etape_changee = true;
            }
            Some((id, etape)) if etape != e.etape => {
                sqlx::query(
                    "UPDATE ent_absp_remplacement SET etape = $3, suppleant_agent_id = $4, a_partir_du = $5,
                            etape_le = now(), modifie_le = now()
                      WHERE etablissement_id = $1 AND id = $2",
                )
                .bind(etab)
                .bind(id)
                .bind(e.etape)
                .bind(e.suppleant)
                .bind(e.a_partir_du)
                .execute(&mut *conn)
                .await
                .map_err(db)?;
                publier_etat(conn, etab, absence_id, a.agent_id, &e, actor).await?;
                bilan.etape_changee = true;
            }
            Some((id, _)) => {
                // Même étape : le suppléant ou la date ont pu changer, sans nouvelle notification.
                sqlx::query(
                    "UPDATE ent_absp_remplacement SET suppleant_agent_id = $3, a_partir_du = $4, modifie_le = now()
                      WHERE etablissement_id = $1 AND id = $2
                        AND (suppleant_agent_id IS DISTINCT FROM $3 OR a_partir_du IS DISTINCT FROM $4)",
                )
                .bind(etab)
                .bind(id)
                .bind(e.suppleant)
                .bind(e.a_partir_du)
                .execute(&mut *conn)
                .await
                .map_err(db)?;
            }
        }
        Ok(bilan)
    }

    /// Tâche planifiée (chaque minute, idempotente) : fait avancer l'état du remplacement des absences qui ont une
    /// demande ou une suppléance (« nommé » → « en cours » à la date, « en cours » → « terminé » à la fin), et
    /// rattrape les séances nouvelles après un recalcul d'impact. Une absence en échec n'arrête pas les autres.
    pub async fn avancer_etats_remplacement(&self) -> Result<BilanRemplacementDto, EntDbError> {
        let mut bilan = BilanRemplacementDto::default();
        for etab in etablissements_systeme(self.pool(), ContexteSysteme::JobPlanifie("absences_personnels_remplacement"))
            .await
            .map_err(db)?
        {
            let ids: Vec<Uuid> = sqlx::query_scalar(
                "SELECT a.id FROM ent_absp_absence a
                  WHERE a.etablissement_id = $1 AND a.etat <> 'annulee'
                    AND (EXISTS (SELECT 1 FROM ent_absp_demande_suppleance d
                                  WHERE d.etablissement_id = a.etablissement_id AND d.absence_id = a.id)
                         OR EXISTS (SELECT 1 FROM ent_service_intervenant i
                                     WHERE i.etablissement_id = a.etablissement_id AND i.nature = 'suppleant'
                                       AND i.origine = 'absence' AND i.origine_ref = a.id AND i.annule_le IS NULL
                                       AND (i.au IS NULL OR i.au >= ent_aujourdhui_etab(a.etablissement_id, now()) - 1)))
                  LIMIT 2000",
            )
            .bind(etab)
            .fetch_all(&mut *TenantTx::begin(self.pool(), etab).await.map_err(db)?)
            .await
            .map_err(db)?;
            for id in ids {
                let r: Result<BilanSynchro, EntDbError> = async {
                    let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
                    let b = self.synchroniser_remplacement_tx(&mut tx, etab, id, None).await?;
                    tx.commit().await.map_err(db)?;
                    Ok(b)
                }
                .await;
                match r {
                    Ok(b) => {
                        bilan.absences += 1;
                        if b.etape_changee {
                            bilan.etapes_changees += 1;
                        }
                    }
                    Err(e) => tracing::warn!(absence = %id, "état du remplacement : {e}"),
                }
            }
        }
        Ok(bilan)
    }
}

async fn demande_tx(conn: &mut PgConnection, etab: Uuid, id: Uuid) -> Result<DemandeLigne, EntDbError> {
    sqlx::query_as::<_, DemandeLigne>(&format!(
        "SELECT {DEMANDE_COLONNES} FROM ent_absp_demande_suppleance WHERE etablissement_id = $1 AND id = $2 FOR UPDATE"
    ))
    .bind(etab)
    .bind(id)
    .fetch_optional(conn)
    .await
    .map_err(db)?
    .ok_or_else(introuvable)
}

async fn etat_remplacement_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    absence_id: Uuid,
) -> Result<Option<EtatRemplacementDto>, EntDbError> {
    Ok(sqlx::query_as::<_, (String, Option<Uuid>, Option<NaiveDate>, DateTime<Utc>)>(
        "SELECT etape, suppleant_agent_id, a_partir_du, etape_le FROM ent_absp_remplacement
          WHERE etablissement_id = $1 AND absence_id = $2",
    )
    .bind(etab)
    .bind(absence_id)
    .fetch_optional(conn)
    .await
    .map_err(db)?
    .map(|l| dto_etat(absence_id, l)))
}

/// Violation d'unicité (`23505`) : « déjà en cours » plutôt qu'une panne.
fn est_conflit_unique(e: &sqlx::Error) -> bool {
    matches!(e, sqlx::Error::Database(d) if d.code().as_deref() == Some("23505"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn j(m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2027, m, d).unwrap()
    }
    fn agent() -> Uuid {
        Uuid::from_u128(7)
    }
    fn supp(du: NaiveDate, au: Option<NaiveDate>, eff: Option<NaiveDate>) -> SuppleanceFait {
        SuppleanceFait { agent_id: agent(), du, au, debut_effectif: eff }
    }

    #[test]
    fn sans_fait_aucune_etape() {
        assert_eq!(etape_remplacement(&FaitsRemplacement::default(), j(11, 20)), None);
    }

    #[test]
    fn une_demande_donne_en_recherche_puis_propose() {
        let mut f = FaitsRemplacement { demande: Some(("demandee".into(), None, None)), ..Default::default() };
        assert_eq!(etape_remplacement(&f, j(11, 20)).unwrap().etape, "en_recherche");
        f.demande = Some(("refusee".into(), None, None));
        assert_eq!(etape_remplacement(&f, j(11, 20)).unwrap().etape, "en_recherche", "un refus laisse la recherche ouverte");
        f.demande = Some(("proposee".into(), Some(agent()), Some(j(12, 2))));
        let e = etape_remplacement(&f, j(11, 20)).unwrap();
        assert_eq!((e.etape, e.suppleant, e.a_partir_du), ("propose", Some(agent()), Some(j(12, 2))));
    }

    #[test]
    fn nomme_avant_la_date_en_cours_pendant_termine_apres() {
        let f = FaitsRemplacement {
            suppleances: vec![supp(j(12, 2), Some(j(12, 20)), Some(j(12, 2)))],
            demande: Some(("proposee".into(), Some(agent()), None)),
        };
        assert_eq!(etape_remplacement(&f, j(11, 25)).unwrap().etape, "nomme");
        assert_eq!(etape_remplacement(&f, j(12, 2)).unwrap().etape, "en_cours");
        assert_eq!(etape_remplacement(&f, j(12, 20)).unwrap().etape, "en_cours", "le dernier jour compte");
        assert_eq!(etape_remplacement(&f, j(12, 21)).unwrap().etape, "termine");
    }

    #[test]
    fn cap37_suppleant_non_installe_reste_nomme_avec_la_date() {
        assert_eq!(debut_effectif(j(12, 2), true, None), None);
        assert_eq!(debut_effectif(j(12, 2), true, Some(j(12, 5))), Some(j(12, 5)));
        assert_eq!(debut_effectif(j(12, 2), true, Some(j(11, 20))), Some(j(12, 2)));
        assert_eq!(debut_effectif(j(12, 2), false, None), Some(j(12, 2)));
        let f = FaitsRemplacement { suppleances: vec![supp(j(12, 2), None, None)], demande: None };
        let e = etape_remplacement(&f, j(12, 10)).unwrap();
        assert_eq!((e.etape, e.a_partir_du), ("nomme", Some(j(12, 2))), "« à partir du … » tant que non installé");
    }

    #[test]
    fn periode_de_suppleance() {
        assert!(suppleance_couvre(j(12, 2), Some(j(12, 10)), j(12, 2)));
        assert!(suppleance_couvre(j(12, 2), Some(j(12, 10)), j(12, 10)));
        assert!(!suppleance_couvre(j(12, 2), Some(j(12, 10)), j(12, 11)));
        assert!(!suppleance_couvre(j(12, 2), None, j(12, 1)));
        assert!(suppleance_couvre(j(12, 2), None, NaiveDate::from_ymd_opt(2028, 2, 1).unwrap()));
    }
}
