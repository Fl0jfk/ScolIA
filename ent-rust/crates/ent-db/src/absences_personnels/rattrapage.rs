//! Socle 15, lot A8 — rattrapages dus (`docs/socles/15_ABSENCES_PERSONNELS.md` §15 ; R-AP26, R-AP27, R-AP28 ;
//! S-1514, S-1515 ; ABP-190 à 199).
//!
//! Une absence validée avec le traitement `rattrapage` crée un **rattrapage dû** : des heures (tirées de l'impact),
//! une échéance (fin de l'année scolaire par défaut). L'agent **propose** des créneaux, la direction **confirme** (le
//! créneau est porté par une séance `ajoutee` du socle 10, **référence souple** : le 15 n'écrit jamais dans les tables
//! du 10, R-AP26), le créneau est **assuré** par un fait (appel fait au socle 12, ou attestation), et le rattrapage est
//! **soldé** quand les heures assurées couvrent les heures dues.
//!
//! | Commande                                       | Droit (D-017)                                              | ABP        |
//! |------------------------------------------------|------------------------------------------------------------|------------|
//! | `creer_rattrapage_du_tolerant` (à la décision) | système (traitement `rattrapage`, heures de l'impact)      | 190        |
//! | tâche `suivre_rattrapages`                     | système (relance J+10, alerte d'échéance, solde par appel) | 191, 194   |
//! | `proposer_creneaux_for_actor`                  | l'agent concerné, jamais un autre                           | 192        |
//! | `confirmer_creneau_for_actor`                  | `absences.valider` (jamais pour soi-même)                  | 193        |
//! | `attester_creneau_for_actor`                   | direction ou CPE (jamais pour soi-même)                    | 194        |
//! | `prolonger_rattrapage_for_actor`               | `absences.valider`, motif codé                              | 195        |
//! | `requalifier_en_retenue_for_actor` ★           | `absences.valider` ; l'agent est prévenu                    | 196        |
//! | `abandonner_rattrapage_for_actor`              | direction, motif codé                                      | 197        |
//! | `heures_eleves_perdues_for_actor`              | direction (adjoint, chargé des remplacements)              | 198        |
//! | `mes_rattrapages_for_actor` / `rattrapages_…`  | soi-même ; `absences.valider` ou `absences.lire_nature`    | 199        |
//!
//! **Garanties.**
//! * **Solder exige un créneau assuré, en base** (déclencheur) : jamais une déclaration seule (R-AP28). Un créneau
//!   n'est assuré que par un appel fait sur sa séance, ou par l'attestation de la direction ou du CPE — jamais de
//!   l'agent lui-même.
//! * Jamais deux créneaux confirmés ou assurés qui se chevauchent pour un même agent (exclusion en base) ; une
//!   séance du 10 ne sert qu'un créneau ; un créneau ne chevauche jamais un cours de l'agent.
//! * Personne ne confirme ni n'atteste son propre rattrapage ; un agent ne voit que les siens.
//! * Aucun événement, aucune ligne ni aucune route ne porte la nature de l'absence ni un texte libre (R-AP2) : les
//!   motifs de décision sont des codes.
//! * Le rattrapage d'**heures d'élèves perdues** (sans dette d'un agent) est une **proposition** au socle 10, jamais
//!   une dette ; la route ne nomme ni l'absent ni la nature.
//!
//! **Non fait** : la création de la séance `ajoutee` elle-même (le socle 10 n'expose pas encore de port d'écriture
//! `modifier_seance` : la direction la crée par l'emploi du temps puis la désigne ici), la plage de planning de
//! service du socle 6 pour un salarié (les heures de service se confirment sans séance et s'attestent), la
//! prévenance des familles (portée par la séance ajoutée, socles 8 et 10).

use super::rcd::{evenement_rcd, journal_objet};
use super::suppleance::jour_etab;
use super::*;
use crate::tenant_tx::{etablissements_systeme, ContexteSysteme};

pub const EVT_RATTRAPAGE_DU: &str = "absence_personnel.rattrapage_du";
pub const EVT_RATTRAPAGE_CRENEAUX: &str = "absence_personnel.rattrapage_creneaux_proposes";
pub const EVT_RATTRAPAGE_CONFIRME: &str = "absence_personnel.rattrapage_creneau_confirme";
pub const EVT_RATTRAPAGE_SOLDE: &str = "absence_personnel.rattrapage_solde";
pub const EVT_RATTRAPAGE_RELANCE: &str = "absence_personnel.rattrapage_relance";
pub const EVT_RATTRAPAGE_ECHU: &str = "absence_personnel.rattrapage_echu";
pub const EVT_RATTRAPAGE_PROLONGE: &str = "absence_personnel.rattrapage_prolonge";
pub const EVT_RATTRAPAGE_RETENUE: &str = "absence_personnel.rattrapage_retenue";
pub const EVT_RATTRAPAGE_ABANDONNE: &str = "absence_personnel.rattrapage_abandonne";

/// Délai de relance des créneaux (S-1515), en jours après la création du rattrapage.
pub const RELANCE_JOURS: i64 = 10;
/// Durée maximale d'un créneau proposé.
pub const CRENEAU_MAX_MINUTES: i64 = 12 * 60;
/// Créneaux proposés par appel, et créneaux ouverts par rattrapage.
const PROPOSITIONS_MAX: usize = 10;
const OUVERTS_MAX: i64 = 20;
const MOTIFS: &[&str] = &[
    "accord_agent",
    "absence_prolongee",
    "calendrier",
    "impossible",
    "decision_direction",
    "autre",
];

// ─────────────────────────────── logique pure (testée sans base) ───────────────────────────────

/// Heures dues depuis les minutes d'impact : arrondies **au-dessus** au dixième d'heure (on ne rend pas moins que ce qui
/// a été perdu).
pub fn heures_dues_depuis(minutes: i64) -> f64 {
    ((minutes.max(0) as f64) / 6.0).ceil() / 10.0
}

/// Un rattrapage est-il en retard : échéance passée et non clos.
pub fn en_retard(etat: &str, echeance: NaiveDate, aujourdhui: NaiveDate) -> bool {
    !etat_clos(etat) && echeance < aujourdhui
}

/// Un rattrapage clos ne change plus.
pub fn etat_clos(etat: &str) -> bool {
    matches!(etat, "solde" | "retenue" | "abandonne")
}

/// Les minutes assurées couvrent-elles les heures dues ? (tolérance d'une demi-minute : l'arrondi du dixième d'heure.)
pub fn couvre(heures_dues: f64, minutes_assurees: i64) -> bool {
    minutes_assurees as f64 >= heures_dues * 60.0 - 0.5
}

/// Heures (au dixième) d'un nombre de minutes.
pub fn heures_de(minutes: i64) -> f64 {
    ((minutes as f64) / 6.0).round() / 10.0
}

/// Relance due (R-AP27) : rattrapage ouvert sans créneau proposé, créé depuis `RELANCE_JOURS` jours, jamais relancé.
pub fn relance_due(
    etat: &str,
    cree_le: NaiveDate,
    aujourdhui: NaiveDate,
    relance_le: Option<NaiveDate>,
    creneaux: i64,
) -> bool {
    matches!(etat, "du" | "prolonge")
        && creneaux == 0
        && relance_le.is_none()
        && cree_le + Duration::days(RELANCE_JOURS) <= aujourdhui
}

/// Traitements qui font naître un rattrapage dû : `rattrapage_interne` (agent de l'État, rattrapé dans l'établissement,
/// §9.3) et `rattrapage` (salarié de l'OGEC).
pub fn est_traitement_rattrapage(t: &str) -> bool {
    matches!(t, "rattrapage" | "rattrapage_interne")
}

/// Traitement de l'absence quand la direction requalifie le rattrapage en retenue : pour un agent de l'État, « service
/// non fait » (la retenue est décidée et calculée hors ScolIA, R-11) ; pour un salarié de l'OGEC, « retenue de salaire ».
pub fn traitement_de_retenue(employeur: &str) -> &'static str {
    if employeur == "ogec" {
        "retenue_salaire"
    } else {
        "service_non_fait"
    }
}

fn motif_valide(m: &str) -> bool {
    MOTIFS.contains(&m)
}

// ─────────────────────────────── base ───────────────────────────────

#[derive(Debug, Clone, sqlx::FromRow)]
struct RattrapageLigne {
    id: Uuid,
    absence_id: Uuid,
    agent_id: Uuid,
    heures_dues: f64,
    echeance: NaiveDate,
    etat: String,
    forme: String,
    motif_code: Option<String>,
}

const RATTRAPAGE_COLONNES: &str =
    "r.id, r.absence_id, r.agent_id, r.heures_dues::float8 AS heures_dues, r.echeance, r.etat, r.forme, r.motif_code";

#[derive(Debug, Clone, sqlx::FromRow)]
struct CreneauLigne {
    id: Uuid,
    debut: DateTime<Utc>,
    fin: DateTime<Utc>,
    etat: String,
    seance_ref: Option<Uuid>,
    assure_preuve: Option<String>,
}

async fn ligne_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    id: Uuid,
    verrou: bool,
) -> Result<RattrapageLigne, EntDbError> {
    sqlx::query_as::<_, RattrapageLigne>(&format!(
        "SELECT {RATTRAPAGE_COLONNES} FROM ent_absp_rattrapage r WHERE r.etablissement_id = $1 AND r.id = $2 {}",
        if verrou { "FOR UPDATE OF r" } else { "" }
    ))
    .bind(etab)
    .bind(id)
    .fetch_optional(conn)
    .await
    .map_err(db)?
    .ok_or_else(introuvable)
}

async fn dto_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    id: Uuid,
) -> Result<RattrapageDto, EntDbError> {
    let jour = jour_etab(&mut *conn, etab).await?;
    let r = ligne_tx(&mut *conn, etab, id, false).await?;
    let cs: Vec<CreneauLigne> = sqlx::query_as(
        "SELECT id, debut, fin, etat, seance_ref, assure_preuve FROM ent_absp_rattrapage_creneau
          WHERE etablissement_id = $1 AND rattrapage_id = $2 ORDER BY debut, id",
    )
    .bind(etab)
    .bind(id)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    let creneaux: Vec<CreneauRattrapageDto> = cs
        .iter()
        .map(|c| CreneauRattrapageDto {
            id: c.id,
            debut: c.debut,
            fin: c.fin,
            minutes: (c.fin - c.debut).num_minutes() as i32,
            etat: c.etat.clone(),
            seance_id: c.seance_ref,
            preuve: c.assure_preuve.clone(),
        })
        .collect();
    let assurees: i64 = creneaux
        .iter()
        .filter(|c| c.etat == "assure")
        .map(|c| i64::from(c.minutes))
        .sum();
    let faites = heures_de(assurees);
    Ok(RattrapageDto {
        id: r.id,
        absence_id: r.absence_id,
        agent_id: r.agent_id,
        heures_dues: r.heures_dues,
        heures_assurees: faites,
        heures_restantes: (r.heures_dues - faites).max(0.0),
        echeance: r.echeance,
        en_retard: en_retard(&r.etat, r.echeance, jour),
        etat: r.etat,
        forme: r.forme,
        motif_code: r.motif_code,
        creneaux,
    })
}

/// Compte utilisateur de l'agent (peut ne pas en avoir).
async fn user_de_l_agent(
    conn: &mut PgConnection,
    etab: Uuid,
    agent: Uuid,
) -> Result<Option<Uuid>, EntDbError> {
    let u: Option<Option<Uuid>> = sqlx::query_scalar(
        "SELECT p.user_id FROM ent_agent a JOIN ent_personne p ON p.etablissement_id = a.etablissement_id AND p.id = a.personne_id
          WHERE a.etablissement_id = $1 AND a.id = $2",
    )
    .bind(etab)
    .bind(agent)
    .fetch_optional(conn)
    .await
    .map_err(db)?;
    Ok(u.flatten())
}

/// Le créneau chevauche-t-il un cours de l'agent (séance du plan ou modifiée, socle 10, lecture seule) ? Une séance
/// `ajoutee` n'en est pas un : c'est justement ce qui porte le rattrapage (leurs chevauchements entre eux sont tenus par
/// l'exclusion des créneaux retenus).
async fn chevauche_un_cours(
    conn: &mut PgConnection,
    etab: Uuid,
    agent_user: Option<Uuid>,
    debut: DateTime<Utc>,
    fin: DateTime<Utc>,
) -> Result<bool, EntDbError> {
    let Some(u) = agent_user else {
        return Ok(false);
    };
    sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM ent_edt_seance s
                         WHERE s.etablissement_id = $1 AND s.prof_user_id = $2
                           AND s.etat IN ('prevue', 'modifiee')
                           AND s.debut_utc < $4 AND s.fin_utc > $3)",
    )
    .bind(etab)
    .bind(u)
    .bind(debut)
    .bind(fin)
    .fetch_one(conn)
    .await
    .map_err(db)
}

/// Solde le rattrapage si les créneaux assurés couvrent les heures dues ; sinon, s'il ne reste aucun créneau confirmé,
/// il redevient « dû ». Renvoie `true` si le rattrapage vient d'être soldé.
async fn solder_si_complet_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    id: Uuid,
    actor: Option<Uuid>,
) -> Result<bool, EntDbError> {
    let r = ligne_tx(&mut *conn, etab, id, true).await?;
    if etat_clos(&r.etat) {
        return Ok(false);
    }
    let (assurees, confirmes): (i64, i64) = sqlx::query_as(
        "SELECT COALESCE(sum(EXTRACT(EPOCH FROM (fin - debut)) / 60) FILTER (WHERE etat = 'assure'), 0)::int8,
                count(*) FILTER (WHERE etat = 'confirme')
           FROM ent_absp_rattrapage_creneau WHERE etablissement_id = $1 AND rattrapage_id = $2",
    )
    .bind(etab)
    .bind(id)
    .fetch_one(&mut *conn)
    .await
    .map_err(db)?;
    if couvre(r.heures_dues, assurees) {
        sqlx::query("UPDATE ent_absp_rattrapage SET etat = 'solde', solde_le = now(), modifie_le = now() WHERE etablissement_id = $1 AND id = $2")
            .bind(etab)
            .bind(id)
            .execute(&mut *conn)
            .await
            .map_err(db)?;
        if let Some(a) = actor {
            journal_objet(
                &mut *conn,
                etab,
                "rattrapage",
                id,
                "solde",
                Some("etat"),
                Some(&r.etat),
                Some("solde"),
                a,
            )
            .await?;
        }
        evenement_rcd(
            &mut *conn,
            etab,
            EVT_RATTRAPAGE_SOLDE,
            id,
            json!({ "rattrapage_id": id, "agent_id": r.agent_id }),
            actor,
        )
        .await?;
        return Ok(true);
    }
    if r.etat == "planifie" && confirmes == 0 {
        sqlx::query("UPDATE ent_absp_rattrapage SET etat = 'du', modifie_le = now() WHERE etablissement_id = $1 AND id = $2")
            .bind(etab)
            .bind(id)
            .execute(&mut *conn)
            .await
            .map_err(db)?;
    }
    Ok(false)
}

/// Marque un créneau confirmé comme assuré (un fait) puis tente de solder. Le créneau doit être terminé.
async fn assurer_creneau_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    creneau: Uuid,
    preuve: &str,
    par: Option<Uuid>,
) -> Result<(), EntDbError> {
    sqlx::query(
        "UPDATE ent_absp_rattrapage_creneau
            SET etat = 'assure', assure_preuve = $3, assure_le = now(), assure_par_user_id = $4
          WHERE etablissement_id = $1 AND id = $2 AND etat = 'confirme'",
    )
    .bind(etab)
    .bind(creneau)
    .bind(preuve)
    .bind(par)
    .execute(conn)
    .await
    .map_err(ecriture)?;
    Ok(())
}

impl PgEnt {
    async fn exiger_valideur_rattrapage(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
    ) -> Result<Role, EntDbError> {
        let r = self.exiger_personnel_abs(role).await?;
        if !self
            .a_droit_abs(etab, actor, role, Action::PersonnelsAbsencesValider)
            .await?
        {
            return Err(EntDbError::Forbidden);
        }
        Ok(r)
    }

    // ───────────────────────────── Création (ABP-190) ─────────────────────────────

    /// Crée le rattrapage dû d'une absence validée avec le traitement `rattrapage` (idempotent). Les heures viennent de
    /// l'impact ; sans impact, rien n'est créé (la tâche planifiée reprend). Renvoie l'identifiant créé.
    pub(super) async fn creer_rattrapage_du_tx(
        &self,
        conn: &mut PgConnection,
        etab: Uuid,
        absence_id: Uuid,
        actor: Option<Uuid>,
    ) -> Result<Option<Uuid>, EntDbError> {
        let a: Option<(Uuid, String, Option<String>)> = sqlx::query_as(
            "SELECT agent_id, etat, traitement FROM ent_absp_absence WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(absence_id)
        .fetch_optional(&mut *conn)
        .await
        .map_err(db)?;
        let Some((agent, etat, traitement)) = a else {
            return Ok(None);
        };
        if !traitement.as_deref().is_some_and(est_traitement_rattrapage)
            || !matches!(etat.as_str(), "validee" | "traitee")
        {
            return Ok(None);
        }
        let existe: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM ent_absp_rattrapage WHERE etablissement_id = $1 AND absence_id = $2)")
            .bind(etab)
            .bind(absence_id)
            .fetch_one(&mut *conn)
            .await
            .map_err(db)?;
        if existe {
            return Ok(None);
        }
        let (minutes, seances): (i64, i64) = sqlx::query_as(
            "SELECT COALESCE(sum(EXTRACT(EPOCH FROM (fin - debut)) / 60), 0)::int8, count(*) FILTER (WHERE type = 'seance')
               FROM ent_absp_impact WHERE etablissement_id = $1 AND absence_id = $2 AND etat <> 'sans_objet'",
        )
        .bind(etab)
        .bind(absence_id)
        .fetch_one(&mut *conn)
        .await
        .map_err(db)?;
        if minutes <= 0 {
            return Ok(None);
        }
        let jour = jour_etab(&mut *conn, etab).await?;
        // S-1514 : fin de l'année scolaire qui porte le jour ; à défaut, un an.
        let fin_annee: Option<NaiveDate> = sqlx::query_scalar(
            "SELECT date_fin FROM ent_annee_scolaire WHERE etablissement_id = $1 AND date_debut <= $2 AND date_fin >= $2
              ORDER BY date_debut DESC LIMIT 1",
        )
        .bind(etab)
        .bind(jour)
        .fetch_optional(&mut *conn)
        .await
        .map_err(db)?;
        let echeance = fin_annee.unwrap_or(jour + Duration::days(365));
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_absp_rattrapage (id, etablissement_id, absence_id, agent_id, heures_dues, echeance, forme)
             VALUES ($1,$2,$3,$4,$5,$6,$7)
             ON CONFLICT (etablissement_id, absence_id) DO NOTHING",
        )
        .bind(id)
        .bind(etab)
        .bind(absence_id)
        .bind(agent)
        .bind(heures_dues_depuis(minutes))
        .bind(echeance)
        .bind(if seances > 0 { "cours" } else { "service" })
        .execute(&mut *conn)
        .await
        .map_err(db)?;
        if let Some(a) = actor {
            journal_objet(
                &mut *conn,
                etab,
                "rattrapage",
                id,
                "cree",
                Some("etat"),
                None,
                Some("du"),
                a,
            )
            .await?;
        }
        evenement_rcd(
            &mut *conn,
            etab,
            EVT_RATTRAPAGE_DU,
            id,
            json!({ "rattrapage_id": id, "agent_id": agent }),
            actor,
        )
        .await?;
        Ok(Some(id))
    }

    /// Même chose, sans jamais faire échouer la décision qui l'a déclenchée (SAVEPOINT, comme l'impact).
    pub(super) async fn creer_rattrapage_du_tolerant(
        &self,
        conn: &mut PgConnection,
        etab: Uuid,
        absence_id: Uuid,
        actor: Uuid,
    ) {
        if sqlx::query("SAVEPOINT rattrapage_du")
            .execute(&mut *conn)
            .await
            .is_err()
        {
            return;
        }
        match self
            .creer_rattrapage_du_tx(conn, etab, absence_id, Some(actor))
            .await
        {
            Ok(_) => {
                let _ = sqlx::query("RELEASE SAVEPOINT rattrapage_du")
                    .execute(&mut *conn)
                    .await;
            }
            Err(e) => {
                tracing::warn!(absence = %absence_id, "rattrapage dû non créé : {e}");
                let _ = sqlx::query("ROLLBACK TO SAVEPOINT rattrapage_du")
                    .execute(&mut *conn)
                    .await;
                let _ = sqlx::query("RELEASE SAVEPOINT rattrapage_du")
                    .execute(&mut *conn)
                    .await;
            }
        }
    }

    // ───────────────────────────── Lectures (ABP-199) ─────────────────────────────

    /// ABP-199 : mes rattrapages dus.
    pub async fn mes_rattrapages_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
    ) -> Result<Vec<RattrapageDto>, EntDbError> {
        self.exiger_personnel_abs(role).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let Some(moi) = agent_du_user_tx(&mut tx, etab, actor).await? else {
            return Ok(vec![]);
        };
        let ids: Vec<Uuid> = sqlx::query_scalar(
            "SELECT id FROM ent_absp_rattrapage WHERE etablissement_id = $1 AND agent_id = $2 ORDER BY echeance, id",
        )
        .bind(etab)
        .bind(moi)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let mut v = Vec::with_capacity(ids.len());
        for id in ids {
            v.push(dto_tx(&mut tx, etab, id).await?);
        }
        Ok(v)
    }

    /// Tous les rattrapages de l'établissement (direction ; secrétariat qui lit la nature). `ouverts` : sans les clos.
    pub async fn rattrapages_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        ouverts: bool,
    ) -> Result<Vec<RattrapageDto>, EntDbError> {
        self.exiger_personnel_abs(role).await?;
        if !(self
            .a_droit_abs(etab, actor, role, Action::PersonnelsAbsencesValider)
            .await?
            || self
                .a_droit_abs(etab, actor, role, Action::PersonnelsAbsencesLireNature)
                .await?)
        {
            return Err(EntDbError::Forbidden);
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let ids: Vec<Uuid> = sqlx::query_scalar(
            "SELECT id FROM ent_absp_rattrapage
              WHERE etablissement_id = $1 AND (NOT $2 OR etat IN ('du', 'planifie', 'prolonge'))
              ORDER BY echeance, id LIMIT 500",
        )
        .bind(etab)
        .bind(ouverts)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let mut v = Vec::with_capacity(ids.len());
        for id in ids {
            v.push(dto_tx(&mut tx, etab, id).await?);
        }
        Ok(v)
    }

    /// Un rattrapage : son agent, ou le cercle des valideurs et lecteurs.
    pub async fn rattrapage_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        id: Uuid,
    ) -> Result<RattrapageDto, EntDbError> {
        self.exiger_personnel_abs(role).await?;
        let gestion = self
            .a_droit_abs(etab, actor, role, Action::PersonnelsAbsencesValider)
            .await?
            || self
                .a_droit_abs(etab, actor, role, Action::PersonnelsAbsencesLireNature)
                .await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let r = ligne_tx(&mut tx, etab, id, false).await?;
        if !gestion && agent_du_user_tx(&mut tx, etab, actor).await? != Some(r.agent_id) {
            return Err(introuvable());
        }
        dto_tx(&mut tx, etab, id).await
    }

    // ───────────────────────────── Créneaux (ABP-192, 193, 194) ─────────────────────────────

    /// ABP-192 : l'agent propose des créneaux. Futurs, ≤ 12 h, avant l'échéance, hors de ses cours.
    pub async fn proposer_creneaux_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        id: Uuid,
        body: &ProposerCreneauxBody,
    ) -> Result<RattrapageDto, EntDbError> {
        self.proposer_creneaux_a(etab, actor, role, id, body, Utc::now())
            .await
    }

    pub(crate) async fn proposer_creneaux_a(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        id: Uuid,
        body: &ProposerCreneauxBody,
        maintenant: DateTime<Utc>,
    ) -> Result<RattrapageDto, EntDbError> {
        self.exiger_personnel_abs(role).await?;
        if body.creneaux.is_empty() || body.creneaux.len() > PROPOSITIONS_MAX {
            return Err(invalide("Proposez entre 1 et 10 créneaux."));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let r = ligne_tx(&mut tx, etab, id, true).await?;
        // Pas d'oracle : un autre agent ne sait même pas que ce rattrapage existe.
        if agent_du_user_tx(&mut tx, etab, actor).await? != Some(r.agent_id) {
            return Err(introuvable());
        }
        if etat_clos(&r.etat) {
            return Err(conflit("ce rattrapage est clos"));
        }
        let ouverts: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM ent_absp_rattrapage_creneau WHERE etablissement_id = $1 AND rattrapage_id = $2 AND etat = 'propose'",
        )
        .bind(etab)
        .bind(id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        if ouverts + body.creneaux.len() as i64 > OUVERTS_MAX {
            return Err(conflit("trop de créneaux en attente de confirmation"));
        }
        let user = user_de_l_agent(&mut tx, etab, r.agent_id).await?;
        for p in &body.creneaux {
            let minutes = (p.fin - p.debut).num_minutes();
            if minutes <= 0 || minutes > CRENEAU_MAX_MINUTES {
                return Err(invalide("Un créneau dure de 1 minute à 12 heures."));
            }
            if p.debut <= maintenant {
                return Err(invalide(
                    "Un créneau de rattrapage se propose dans le futur.",
                ));
            }
            let jour: NaiveDate = sqlx::query_scalar("SELECT ent_aujourdhui_etab($1, $2)")
                .bind(etab)
                .bind(p.debut)
                .fetch_one(&mut *tx)
                .await
                .map_err(db)?;
            if jour > r.echeance {
                return Err(conflit(
                    "ce créneau dépasse l'échéance du rattrapage : demandez une prolongation",
                ));
            }
            if chevauche_un_cours(&mut tx, etab, user, p.debut, p.fin).await? {
                return Err(conflit("ce créneau chevauche un de vos cours"));
            }
            sqlx::query(
                "INSERT INTO ent_absp_rattrapage_creneau (id, etablissement_id, rattrapage_id, agent_id, debut, fin, propose_par_user_id)
                 VALUES ($1,$2,$3,$4,$5,$6,$7)",
            )
            .bind(Uuid::new_v4())
            .bind(etab)
            .bind(id)
            .bind(r.agent_id)
            .bind(p.debut)
            .bind(p.fin)
            .bind(actor)
            .execute(&mut *tx)
            .await
            .map_err(ecriture)?;
        }
        journal_objet(
            &mut tx,
            etab,
            "rattrapage",
            id,
            "creneaux_proposes",
            Some("nombre"),
            None,
            Some(&body.creneaux.len().to_string()),
            actor,
        )
        .await?;
        evenement_rcd(
            &mut tx,
            etab,
            EVT_RATTRAPAGE_CRENEAUX,
            id,
            json!({ "rattrapage_id": id, "agent_id": r.agent_id }),
            Some(actor),
        )
        .await?;
        let d = dto_tx(&mut tx, etab, id).await?;
        tx.commit().await.map_err(db)?;
        Ok(d)
    }

    /// ABP-193 : la direction confirme un créneau. Pour un enseignant, une séance `ajoutee` du socle 10 doit le porter
    /// (même agent, mêmes instants) ; pour des heures de service, aucune séance. Jamais pour soi-même.
    pub async fn confirmer_creneau_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        id: Uuid,
        creneau_id: Uuid,
        body: &ConfirmerCreneauBody,
    ) -> Result<RattrapageDto, EntDbError> {
        self.exiger_valideur_rattrapage(etab, actor, role).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let r = ligne_tx(&mut tx, etab, id, true).await?;
        if agent_du_user_tx(&mut tx, etab, actor).await? == Some(r.agent_id) {
            return Err(EntDbError::Refus(
                crate::service::RefusMetier::Interdit,
                "Personne ne confirme son propre rattrapage.",
            ));
        }
        if etat_clos(&r.etat) {
            return Err(conflit("ce rattrapage est clos"));
        }
        let c: CreneauLigne = sqlx::query_as(
            "SELECT id, debut, fin, etat, seance_ref, assure_preuve FROM ent_absp_rattrapage_creneau
              WHERE etablissement_id = $1 AND rattrapage_id = $2 AND id = $3 FOR UPDATE",
        )
        .bind(etab)
        .bind(id)
        .bind(creneau_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?
        .ok_or_else(introuvable)?;
        if c.etat != "propose" {
            return Err(conflit("ce créneau n'est plus en attente de confirmation"));
        }
        let user = user_de_l_agent(&mut tx, etab, r.agent_id).await?;
        match body.seance_id {
            Some(s) => {
                let sc: Option<(String, Option<DateTime<Utc>>, Option<DateTime<Utc>>, Uuid)> = sqlx::query_as(
                    "SELECT etat, debut_utc, fin_utc, prof_user_id FROM ent_edt_seance WHERE etablissement_id = $1 AND id = $2",
                )
                .bind(etab)
                .bind(s)
                .fetch_optional(&mut *tx)
                .await
                .map_err(db)?;
                let Some((etat, d, f, prof)) = sc else {
                    return Err(conflit("séance inconnue de l'emploi du temps"));
                };
                if etat != "ajoutee" {
                    return Err(conflit(
                        "la séance désignée n'est pas une séance ajoutée à l'emploi du temps",
                    ));
                }
                if Some(prof) != user {
                    return Err(conflit("la séance désignée n'est pas celle de cet agent"));
                }
                if d != Some(c.debut) || f != Some(c.fin) {
                    return Err(conflit(
                        "la séance désignée ne correspond pas aux horaires du créneau",
                    ));
                }
            }
            None if r.forme == "cours" => {
                return Err(conflit("un cours rendu se confirme avec la séance ajoutée à l'emploi du temps qui le porte"));
            }
            None => {}
        }
        if chevauche_un_cours(&mut tx, etab, user, c.debut, c.fin).await? {
            return Err(conflit("ce créneau chevauche un cours de l'agent"));
        }
        sqlx::query(
            "UPDATE ent_absp_rattrapage_creneau SET etat = 'confirme', confirme_par_user_id = $4, confirme_le = now(), seance_ref = $5
              WHERE etablissement_id = $1 AND rattrapage_id = $2 AND id = $3",
        )
        .bind(etab)
        .bind(id)
        .bind(creneau_id)
        .bind(actor)
        .bind(body.seance_id)
        .execute(&mut *tx)
        .await
        .map_err(ecriture)?;
        if matches!(r.etat.as_str(), "du" | "prolonge") {
            sqlx::query("UPDATE ent_absp_rattrapage SET etat = 'planifie', modifie_le = now() WHERE etablissement_id = $1 AND id = $2")
                .bind(etab)
                .bind(id)
                .execute(&mut *tx)
                .await
                .map_err(db)?;
        }
        journal_objet(
            &mut tx,
            etab,
            "rattrapage",
            id,
            "creneau_confirme",
            Some("etat"),
            Some(&r.etat),
            Some("planifie"),
            actor,
        )
        .await?;
        evenement_rcd(
            &mut tx,
            etab,
            EVT_RATTRAPAGE_CONFIRME,
            id,
            json!({ "rattrapage_id": id, "agent_id": r.agent_id, "creneau_id": creneau_id }),
            Some(actor),
        )
        .await?;
        let d = dto_tx(&mut tx, etab, id).await?;
        tx.commit().await.map_err(db)?;
        Ok(d)
    }

    /// ABP-194 : la direction ou le CPE **atteste** qu'un créneau confirmé, terminé, a été assuré (l'appel ne l'a pas
    /// prouvé). Jamais l'agent lui-même. Si l'appel est fait sur la séance, c'est l'appel qui prouve.
    pub async fn attester_creneau_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        id: Uuid,
        creneau_id: Uuid,
    ) -> Result<RattrapageDto, EntDbError> {
        self.attester_creneau_a(etab, actor, role, id, creneau_id, Utc::now())
            .await
    }

    pub(crate) async fn attester_creneau_a(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        id: Uuid,
        creneau_id: Uuid,
        maintenant: DateTime<Utc>,
    ) -> Result<RattrapageDto, EntDbError> {
        let r = self.exiger_personnel_abs(role).await?;
        if !matches!(r, Role::Cpe | Role::Direction) {
            return Err(EntDbError::Forbidden);
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let rat = ligne_tx(&mut tx, etab, id, true).await?;
        if agent_du_user_tx(&mut tx, etab, actor).await? == Some(rat.agent_id) {
            return Err(EntDbError::Refus(
                crate::service::RefusMetier::Interdit,
                "Personne n'atteste son propre rattrapage.",
            ));
        }
        let c: CreneauLigne = sqlx::query_as(
            "SELECT id, debut, fin, etat, seance_ref, assure_preuve FROM ent_absp_rattrapage_creneau
              WHERE etablissement_id = $1 AND rattrapage_id = $2 AND id = $3 FOR UPDATE",
        )
        .bind(etab)
        .bind(id)
        .bind(creneau_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?
        .ok_or_else(introuvable)?;
        if c.etat != "confirme" {
            return Err(conflit("seul un créneau confirmé s'atteste"));
        }
        if c.fin > maintenant {
            return Err(conflit(
                "le créneau n'est pas terminé : on n'atteste pas un créneau à venir",
            ));
        }
        let appel = match c.seance_ref {
            Some(s) => sqlx::query_scalar(
                "SELECT EXISTS (SELECT 1 FROM ent_appel WHERE etablissement_id = $1 AND seance_id = $2 AND etat = 'valide')",
            )
            .bind(etab)
            .bind(s)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?,
            None => false,
        };
        assurer_creneau_tx(
            &mut tx,
            etab,
            creneau_id,
            if appel { "appel" } else { "attestation" },
            Some(actor),
        )
        .await?;
        solder_si_complet_tx(&mut tx, etab, id, Some(actor)).await?;
        journal_objet(
            &mut tx,
            etab,
            "rattrapage",
            id,
            "creneau_assure",
            Some("etat"),
            Some("confirme"),
            Some("assure"),
            actor,
        )
        .await?;
        let d = dto_tx(&mut tx, etab, id).await?;
        tx.commit().await.map_err(db)?;
        Ok(d)
    }

    // ───────────────────────────── Décisions (ABP-195, 196, 197) ─────────────────────────────

    /// ABP-195 : prolonge l'échéance (motif codé). Jamais vers une date antérieure ; l'alerte d'échéance peut se
    /// redéclencher.
    pub async fn prolonger_rattrapage_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        id: Uuid,
        body: &ProlongerRattrapageBody,
    ) -> Result<RattrapageDto, EntDbError> {
        self.exiger_valideur_rattrapage(etab, actor, role).await?;
        if !motif_valide(&body.motif_code) {
            return Err(invalide("Motif de prolongation inconnu."));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let r = ligne_tx(&mut tx, etab, id, true).await?;
        if etat_clos(&r.etat) {
            return Err(conflit("ce rattrapage est clos"));
        }
        if body.echeance <= r.echeance {
            return Err(conflit(
                "la nouvelle échéance doit être postérieure à l'échéance actuelle",
            ));
        }
        let nouvel_etat = if r.etat == "du" {
            "prolonge"
        } else {
            r.etat.as_str()
        };
        sqlx::query(
            "UPDATE ent_absp_rattrapage
                SET echeance = $3, etat = $4, motif_code = $5, decide_par_user_id = $6, decide_le = now(),
                    alerte_echeance_le = NULL, modifie_le = now()
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(id)
        .bind(body.echeance)
        .bind(nouvel_etat)
        .bind(&body.motif_code)
        .bind(actor)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        journal_objet(
            &mut tx,
            etab,
            "rattrapage",
            id,
            "prolonge",
            Some("echeance"),
            Some(&r.echeance.to_string()),
            Some(&body.echeance.to_string()),
            actor,
        )
        .await?;
        evenement_rcd(
            &mut tx,
            etab,
            EVT_RATTRAPAGE_PROLONGE,
            id,
            json!({ "rattrapage_id": id, "agent_id": r.agent_id }),
            Some(actor),
        )
        .await?;
        let d = dto_tx(&mut tx, etab, id).await?;
        tx.commit().await.map_err(db)?;
        Ok(d)
    }

    /// ABP-196 ★ : requalifie le rattrapage non soldé en **retenue** ; le traitement de l'absence devient
    /// `service_non_fait` (État) ou `retenue_salaire` (OGEC), décision tracée et l'agent est prévenu (événement). La décision de retenue reste humaine.
    pub async fn requalifier_en_retenue_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        id: Uuid,
        body: &DecisionRattrapageBody,
    ) -> Result<RattrapageDto, EntDbError> {
        self.exiger_valideur_rattrapage(etab, actor, role).await?;
        if body.motif_code.as_deref().is_some_and(|m| !motif_valide(m)) {
            return Err(invalide("Motif inconnu."));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let r = ligne_tx(&mut tx, etab, id, true).await?;
        if agent_du_user_tx(&mut tx, etab, actor).await? == Some(r.agent_id) {
            return Err(EntDbError::Refus(
                crate::service::RefusMetier::Interdit,
                "Personne ne requalifie son propre rattrapage.",
            ));
        }
        if etat_clos(&r.etat) {
            return Err(conflit("ce rattrapage est clos"));
        }
        sqlx::query(
            "UPDATE ent_absp_rattrapage SET etat = 'retenue', motif_code = $3, decide_par_user_id = $4, decide_le = now(), modifie_le = now()
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(id)
        .bind(body.motif_code.as_deref())
        .bind(actor)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        // Le traitement de l'absence suit la décision, avec sa trace.
        let a = charger(&mut tx, etab, r.absence_id, true).await?;
        let nouveau_traitement = traitement_de_retenue(&a.employeur);
        sqlx::query(
            "UPDATE ent_absp_absence SET traitement = $3 WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(r.absence_id)
        .bind(nouveau_traitement)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        decision(
            &mut tx,
            etab,
            r.absence_id,
            actor,
            NouvelleDecision {
                type_: "traitement",
                traitement_avant: a.traitement.as_deref(),
                traitement_apres: Some(nouveau_traitement),
                raison_code: body.motif_code.as_deref(),
                ..Default::default()
            },
        )
        .await?;
        journal_objet(
            &mut tx,
            etab,
            "rattrapage",
            id,
            "requalifie_retenue",
            Some("etat"),
            Some(&r.etat),
            Some("retenue"),
            actor,
        )
        .await?;
        evenement_rcd(
            &mut tx,
            etab,
            EVT_RATTRAPAGE_RETENUE,
            id,
            json!({ "rattrapage_id": id, "agent_id": r.agent_id }),
            Some(actor),
        )
        .await?;
        let d = dto_tx(&mut tx, etab, id).await?;
        tx.commit().await.map_err(db)?;
        Ok(d)
    }

    /// ABP-197 : abandonne le rattrapage, avec un motif codé obligatoire. La direction seule.
    pub async fn abandonner_rattrapage_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        id: Uuid,
        body: &DecisionRattrapageBody,
    ) -> Result<RattrapageDto, EntDbError> {
        let r = self.exiger_valideur_rattrapage(etab, actor, role).await?;
        if r != Role::Direction {
            return Err(EntDbError::Forbidden);
        }
        let Some(motif) = body.motif_code.as_deref().filter(|m| motif_valide(m)) else {
            return Err(invalide("Indiquez le motif de l'abandon (code)."));
        };
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let rat = ligne_tx(&mut tx, etab, id, true).await?;
        if agent_du_user_tx(&mut tx, etab, actor).await? == Some(rat.agent_id) {
            return Err(EntDbError::Refus(
                crate::service::RefusMetier::Interdit,
                "Personne n'abandonne son propre rattrapage.",
            ));
        }
        if etat_clos(&rat.etat) {
            return Err(conflit("ce rattrapage est clos"));
        }
        sqlx::query(
            "UPDATE ent_absp_rattrapage SET etat = 'abandonne', motif_code = $3, decide_par_user_id = $4, decide_le = now(), modifie_le = now()
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(id)
        .bind(motif)
        .bind(actor)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        journal_objet(
            &mut tx,
            etab,
            "rattrapage",
            id,
            "abandonne",
            Some("etat"),
            Some(&rat.etat),
            Some("abandonne"),
            actor,
        )
        .await?;
        evenement_rcd(
            &mut tx,
            etab,
            EVT_RATTRAPAGE_ABANDONNE,
            id,
            json!({ "rattrapage_id": id, "agent_id": rat.agent_id }),
            Some(actor),
        )
        .await?;
        let d = dto_tx(&mut tx, etab, id).await?;
        tx.commit().await.map_err(db)?;
        Ok(d)
    }

    // ───────────────────────────── Heures d'élèves perdues (ABP-198) ─────────────────────────────

    /// ABP-198 : les séances d'élèves **non assurées** (poste vacant, absence non remplacée) sur une fenêtre de jours,
    /// à proposer au socle 10 en séance ajoutée. Aucune dette d'un agent : la réponse ne nomme ni l'absent ni la nature.
    pub async fn heures_eleves_perdues_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        du: NaiveDate,
        au: NaiveDate,
    ) -> Result<Vec<HeuresElevesPerduesDto>, EntDbError> {
        self.exiger_personnel_abs(role).await?;
        if !self
            .a_droit_abs(etab, actor, role, Action::PersonnelsRcdGerer)
            .await?
        {
            return Err(EntDbError::Forbidden);
        }
        if au < du || (au - du).num_days() > 92 {
            return Err(invalide("Fenêtre de 92 jours au plus."));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let lignes: Vec<(Uuid, NaiveDate, NaiveTime, NaiveTime, String, String)> = sqlx::query_as(
            "SELECT DISTINCT s.id, s.jour, s.heure_debut, s.heure_fin, s.classe, s.matiere_libelle
               FROM ent_absp_impact i
               JOIN ent_edt_seance s ON s.etablissement_id = i.etablissement_id AND s.id = i.seance_id
              WHERE i.etablissement_id = $1 AND i.type = 'seance' AND i.etat = 'non_assure'
                AND s.jour BETWEEN $2 AND $3
              ORDER BY s.jour, s.heure_debut, s.id LIMIT 500",
        )
        .bind(etab)
        .bind(du)
        .bind(au)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        Ok(lignes
            .into_iter()
            .map(
                |(seance_id, jour, d, f, classe, matiere)| HeuresElevesPerduesDto {
                    seance_id,
                    jour,
                    heure_debut: d,
                    heure_fin: f,
                    minutes: ((f - d).num_minutes()).clamp(0, 1440) as i32,
                    classe,
                    matiere,
                },
            )
            .collect())
    }

    // ───────────────────────────── Tâche planifiée (ABP-190, 191, 194) ─────────────────────────────

    pub async fn suivre_rattrapages(&self) -> Result<BilanRattrapagesDto, EntDbError> {
        self.suivre_rattrapages_a(Utc::now()).await
    }

    /// Idempotente, `maintenant` injectable. Par établissement : (a) crée les rattrapages dus manquants (absence
    /// requalifiée, impact calculé après coup) ; (b) assure les créneaux dont la séance a eu son appel fait ; (c) relance
    /// à J+10 un rattrapage sans créneau (R-AP27) ; (d) alerte une fois la direction d'un rattrapage échu non soldé.
    pub(crate) async fn suivre_rattrapages_a(
        &self,
        maintenant: DateTime<Utc>,
    ) -> Result<BilanRattrapagesDto, EntDbError> {
        let mut bilan = BilanRattrapagesDto::default();
        for etab in etablissements_systeme(
            self.pool(),
            ContexteSysteme::JobPlanifie("absences_personnels_rattrapages"),
        )
        .await
        .map_err(db)?
        {
            match self.suivre_rattrapages_etab_a(etab, maintenant).await {
                Ok(b) => {
                    bilan.crees += b.crees;
                    bilan.relances += b.relances;
                    bilan.alertes_echeance += b.alertes_echeance;
                    bilan.soldes += b.soldes;
                }
                Err(e) => tracing::warn!(%etab, "suivi des rattrapages : {e}"),
            }
        }
        Ok(bilan)
    }

    /// Le suivi d'UN établissement, dans une transaction (les tests le restreignent à leurs propres lignes).
    pub(crate) async fn suivre_rattrapages_etab_a(
        &self,
        etab: Uuid,
        maintenant: DateTime<Utc>,
    ) -> Result<BilanRattrapagesDto, EntDbError> {
        let mut b = BilanRattrapagesDto::default();
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        // (a) rattrapages manquants.
        let manquantes: Vec<Uuid> = sqlx::query_scalar(
            "SELECT a.id FROM ent_absp_absence a
              WHERE a.etablissement_id = $1 AND a.traitement IN ('rattrapage', 'rattrapage_interne') AND a.etat IN ('validee', 'traitee')
                AND NOT EXISTS (SELECT 1 FROM ent_absp_rattrapage r WHERE r.etablissement_id = a.etablissement_id AND r.absence_id = a.id)
              ORDER BY a.cree_le LIMIT 200",
        )
        .bind(etab)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        for a in manquantes {
            if self
                .creer_rattrapage_du_tx(&mut tx, etab, a, None)
                .await?
                .is_some()
            {
                b.crees += 1;
            }
        }
        // (b) créneaux confirmés, terminés, dont l'appel est fait sur la séance ajoutée (R-AP28).
        let faits: Vec<(Uuid, Uuid)> = sqlx::query_as(
            "SELECT c.id, c.rattrapage_id FROM ent_absp_rattrapage_creneau c
               JOIN ent_edt_seance s ON s.etablissement_id = c.etablissement_id AND s.id = c.seance_ref AND s.etat = 'ajoutee'
              WHERE c.etablissement_id = $1 AND c.etat = 'confirme' AND c.fin <= $2
                AND EXISTS (SELECT 1 FROM ent_appel ap WHERE ap.etablissement_id = c.etablissement_id
                               AND ap.seance_id = c.seance_ref AND ap.etat = 'valide')
              ORDER BY c.fin LIMIT 500",
        )
        .bind(etab)
        .bind(maintenant)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        for (c, rat) in faits {
            assurer_creneau_tx(&mut tx, etab, c, "appel", None).await?;
            if solder_si_complet_tx(&mut tx, etab, rat, None).await? {
                b.soldes += 1;
            }
        }
        // (c) relance J+10 (R-AP27).
        let jour: NaiveDate = sqlx::query_scalar("SELECT ent_aujourdhui_etab($1, $2)")
            .bind(etab)
            .bind(maintenant)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
        let ouverts: Vec<(Uuid, Uuid, String, NaiveDate, Option<NaiveDate>, i64)> = sqlx::query_as(
            "SELECT r.id, r.agent_id, r.etat, ent_aujourdhui_etab(r.etablissement_id, r.cree_le), r.relance_le,
                    (SELECT count(*) FROM ent_absp_rattrapage_creneau c WHERE c.etablissement_id = r.etablissement_id
                        AND c.rattrapage_id = r.id AND c.etat IN ('propose', 'confirme', 'assure'))
               FROM ent_absp_rattrapage r
              WHERE r.etablissement_id = $1 AND r.etat IN ('du', 'prolonge') AND r.relance_le IS NULL
              ORDER BY r.cree_le LIMIT 500",
        )
        .bind(etab)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        for (id, agent, etat, cree, relance, nb) in ouverts {
            if relance_due(&etat, cree, jour, relance, nb) {
                sqlx::query("UPDATE ent_absp_rattrapage SET relance_le = $3, modifie_le = now() WHERE etablissement_id = $1 AND id = $2")
                    .bind(etab)
                    .bind(id)
                    .bind(jour)
                    .execute(&mut *tx)
                    .await
                    .map_err(db)?;
                evenement_rcd(
                    &mut tx,
                    etab,
                    EVT_RATTRAPAGE_RELANCE,
                    id,
                    json!({ "rattrapage_id": id, "agent_id": agent }),
                    None,
                )
                .await?;
                b.relances += 1;
            }
        }
        // (d) alerte d'échéance : une seule fois par échéance.
        let echus: Vec<(Uuid, Uuid)> = sqlx::query_as(
            "SELECT id, agent_id FROM ent_absp_rattrapage
              WHERE etablissement_id = $1 AND etat IN ('du', 'planifie', 'prolonge') AND echeance < $2 AND alerte_echeance_le IS NULL
              ORDER BY echeance LIMIT 500",
        )
        .bind(etab)
        .bind(jour)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        for (id, agent) in echus {
            sqlx::query("UPDATE ent_absp_rattrapage SET alerte_echeance_le = $3, modifie_le = now() WHERE etablissement_id = $1 AND id = $2")
                .bind(etab)
                .bind(id)
                .bind(jour)
                .execute(&mut *tx)
                .await
                .map_err(db)?;
            evenement_rcd(
                &mut tx,
                etab,
                EVT_RATTRAPAGE_ECHU,
                id,
                json!({ "rattrapage_id": id, "agent_id": agent }),
                None,
            )
            .await?;
            b.alertes_echeance += 1;
        }
        tx.commit().await.map_err(db)?;
        Ok(b)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(a: i32, m: u32, j: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(a, m, j).unwrap()
    }

    #[test]
    fn les_heures_dues_s_arrondissent_au_dessus() {
        assert_eq!(heures_dues_depuis(0), 0.0);
        assert_eq!(heures_dues_depuis(60), 1.0);
        assert_eq!(heures_dues_depuis(61), 1.1);
        assert_eq!(heures_dues_depuis(180), 3.0);
        assert_eq!(heures_dues_depuis(-5), 0.0);
    }

    #[test]
    fn couvre_tolere_l_arrondi_du_dixieme() {
        assert!(couvre(1.1, 66));
        assert!(couvre(1.0, 60));
        assert!(!couvre(1.0, 59));
        assert!(!couvre(3.0, 120));
    }

    #[test]
    fn un_rattrapage_clos_n_est_jamais_en_retard() {
        assert!(en_retard("du", d(2027, 6, 30), d(2027, 7, 1)));
        assert!(!en_retard("solde", d(2027, 6, 30), d(2027, 7, 1)));
        assert!(!en_retard("du", d(2027, 6, 30), d(2027, 6, 30)));
        assert!(etat_clos("abandonne") && etat_clos("retenue") && !etat_clos("prolonge"));
    }

    #[test]
    fn la_relance_a_j_plus_10_une_seule_fois_et_sans_creneau() {
        let cree = d(2027, 1, 4);
        assert!(!relance_due("du", cree, d(2027, 1, 13), None, 0));
        assert!(relance_due("du", cree, d(2027, 1, 14), None, 0));
        assert!(!relance_due(
            "du",
            cree,
            d(2027, 1, 20),
            Some(d(2027, 1, 14)),
            0
        ));
        assert!(!relance_due("du", cree, d(2027, 1, 20), None, 1));
        assert!(!relance_due("planifie", cree, d(2027, 1, 20), None, 0));
        assert!(relance_due("prolonge", cree, d(2027, 1, 20), None, 0));
    }

    #[test]
    fn les_deux_traitements_de_rattrapage_et_la_retenue_selon_l_employeur() {
        assert!(
            est_traitement_rattrapage("rattrapage")
                && est_traitement_rattrapage("rattrapage_interne")
        );
        assert!(
            !est_traitement_rattrapage("sans_incidence")
                && !est_traitement_rattrapage("retenue_salaire")
        );
        assert_eq!(traitement_de_retenue("etat"), "service_non_fait");
        assert_eq!(traitement_de_retenue("ogec"), "retenue_salaire");
    }

    #[test]
    fn les_motifs_sont_des_codes() {
        assert!(motif_valide("accord_agent"));
        assert!(!motif_valide("il avait la grippe"));
    }
}
