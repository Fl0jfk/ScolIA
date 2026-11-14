//! Socle 15, lot A6 — remplacement de courte durée (RCD)
//! (`docs/socles/15_ABSENCES_PERSONNELS.md` §12 ; R-01 à R-04 ; ABP-220 à 231).
//!
//! | Commande                                      | Droit (D-017)                                        | ABP        |
//! |-----------------------------------------------|------------------------------------------------------|------------|
//! | `creer_plan_rcd_for_actor`                    | `personnels.rcd.gerer`                               | 220        |
//! | `modifier_plan_rcd_for_actor` (brouillon)     | `rcd.gerer`                                          | 220        |
//! | `versionner_plan_rcd_for_actor`               | `rcd.gerer`                                          | 221        |
//! | `procedure_plan_rcd_for_actor`                | `rcd.gerer` (avis du conseil pédagogique, CA, recteur)| 222, 223   |
//! | `produire_compte_rendu_rcd_for_actor`         | `rcd.gerer`                                          | 224        |
//! | `lancer_campagne_rcd_for_actor`               | `rcd.gerer`                                          | 225        |
//! | `engager_rcd_for_actor` / `modifier_…`        | soi-même, ou `rcd.gerer` pour un autre               | 226, 227   |
//! | `vivier_rcd_for_actor`                        | `rcd.gerer`                                          | 228        |
//! | `mon_compteur_rcd_for_actor`                  | soi-même (position sans nommer les autres)           | 213        |
//! | `preparer_remontee_rcd_for_actor` + tâche     | `personnels.rcd.remontee` / système                  | 229        |
//! | `controler_` / `rejeter_` / `transmettre_remontee_rcd_for_actor` | `rcd.remontee`                    | 230        |
//! | `exporter_remontee_rcd_csv_for_actor`         | `rcd.remontee` (tableau équivalent, Q-AP3)           | 231        |
//!
//! **Garanties.**
//! * **Plan versionné** : une ligne par version, une seule version vivante par année ; le contenu d'une version
//!   présentée ou transmise ne change plus (déclencheur) : on en tire une nouvelle, l'ancienne est archivée et
//!   reste lisible. Une version ne se supprime pas.
//! * **Volontaires et compteurs** : un engagement actif par agent et par année ; sollicité, accepté, refusé (dont
//!   motif légitime, R-AP34), fait, restant, alerte au volume atteint (pas de blocage : l'accord vaut). Un agent ne
//!   voit que sa position par rapport à la médiane, jamais les autres (ABP-213).
//! * **Remontée sans nom** (R-02) : la table ne porte aucun identifiant de personne ; les lignes sont agrégées par
//!   discipline, niveau, catégorie nationale du motif et mode de continuité ; un administratif contrôle puis laisse
//!   partir ; une remontée transmise ne change plus. Tant que l'API du SI-RCD n'est pas connue (Q-AP3), le tableau
//!   équivalent s'exporte en CSV, tracé au journal.
//! * Le plan est lisible de tout le personnel (le délai de prévenance concerne chaque volontaire), les engagements
//!   et le vivier selon le droit ; aucune route ne renvoie la nature d'une absence.

use super::suppleance::jour_etab;
use super::*;
use crate::tenant_tx::{etablissements_systeme, ContexteSysteme};

const STATUTS_ENGAGEMENT: &[&str] = &["pacte", "hse", "autre"];
const INSTANCES: &[&str] = &["ca", "instance_etablissement"];

pub const EVT_RCD_CAMPAGNE: &str = "absence_personnel.rcd_campagne_lancee";
pub const EVT_RCD_REMONTEE_PRETE: &str = "absence_personnel.rcd_remontee_prete";
pub const EVT_RCD_REMONTEE_TRANSMISE: &str = "absence_personnel.rcd_remontee_transmise";

// ─────────────────────────────── logique pure (testée sans base) ───────────────────────────────

/// Lundi de la semaine d'un jour.
pub fn lundi_de(d: NaiveDate) -> NaiveDate {
    use chrono::Datelike;
    d - Duration::days(d.weekday().num_days_from_monday() as i64)
}

/// Semaine remontée par défaut : la semaine précédant celle du jour de l'établissement (§12.3).
pub fn semaine_par_defaut(aujourdhui: NaiveDate) -> NaiveDate {
    lundi_de(aujourdhui) - Duration::days(7)
}

/// Faits de procédure d'un plan (R-01, R-03).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Procedure {
    pub avis_cp_le: Option<NaiveDate>,
    pub presente_le: Option<NaiveDate>,
    pub instance: Option<String>,
    pub transmis_le: Option<NaiveDate>,
}

/// État d'une version déduit de ses faits : transmise dès qu'une date de transmission est posée, présentée dès
/// que l'avis, la présentation et l'instance sont connus, brouillon sinon.
pub fn etat_plan(p: &Procedure) -> &'static str {
    if p.transmis_le.is_some() {
        "transmis"
    } else if p.avis_cp_le.is_some() && p.presente_le.is_some() && p.instance.is_some() {
        "presente"
    } else {
        "brouillon"
    }
}

/// Cohérence des faits : aucun dans le futur ; l'avis précède la présentation, qui précède la transmission ;
/// pas de transmission sans présentation complète.
pub fn verifier_procedure(p: &Procedure, aujourdhui: NaiveDate) -> Result<(), &'static str> {
    if let Some(i) = p.instance.as_deref() {
        if !INSTANCES.contains(&i) {
            return Err("Instance inconnue : conseil d'administration ou instance de l'établissement.");
        }
    }
    for d in [p.avis_cp_le, p.presente_le, p.transmis_le].into_iter().flatten() {
        if d > aujourdhui {
            return Err("Une date de procédure ne peut pas être dans le futur.");
        }
    }
    if let (Some(a), Some(b)) = (p.avis_cp_le, p.presente_le) {
        if a > b {
            return Err("L'avis du conseil pédagogique précède la présentation.");
        }
    }
    if let (Some(b), Some(c)) = (p.presente_le, p.transmis_le) {
        if b > c {
            return Err("La transmission au recteur suit la présentation.");
        }
    }
    if p.transmis_le.is_some() && !(p.avis_cp_le.is_some() && p.presente_le.is_some() && p.instance.is_some()) {
        return Err("Le plan doit avoir reçu l'avis du conseil pédagogique et été présenté avant d'être transmis.");
    }
    Ok(())
}

/// Médiane d'une liste de valeurs (moyenne des deux valeurs centrales si le nombre est pair).
pub fn mediane(valeurs: &[f64]) -> f64 {
    let mut v: Vec<f64> = valeurs.to_vec();
    v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    match v.len() {
        0 => 0.0,
        n if n % 2 == 1 => v[n / 2],
        n => (v[n / 2 - 1] + v[n / 2]) / 2.0,
    }
}

/// Position d'un volontaire par rapport à la médiane, sans nommer personne (ABP-213). Écart de 0,05 h toléré.
pub fn position(valeur: f64, mediane: f64) -> &'static str {
    if valeur < mediane - 0.05 {
        "en_dessous"
    } else if valeur > mediane + 0.05 {
        "au_dessus"
    } else {
        "dans_la_mediane"
    }
}

/// Une ligne brute de remontée (avant ventilation des heures prévues).
#[derive(Clone, Debug, PartialEq)]
pub struct LigneBrute {
    pub discipline: String,
    pub niveau: String,
    pub categorie: String,
    pub mode: String,
    pub absence: f64,
    pub remplacees: f64,
}

/// Arrondi à un chiffre décimal (colonnes `NUMERIC(7,1)`).
pub fn arrondi1(x: f64) -> f64 {
    (x * 10.0).round() / 10.0
}

/// Construit les lignes définitives. Les heures prévues à l'EDT sont connues par (discipline, niveau) ; elles sont
/// portées **une seule fois** (sur la première ligne du couple, dans l'ordre catégorie puis mode) pour que la somme
/// des lignes reste exacte ; les autres lignes du couple portent 0. Les lignes sont triées et sans personne.
pub fn lignes_remontee(
    mut brutes: Vec<LigneBrute>,
    prevues: &std::collections::HashMap<(String, String), f64>,
) -> Vec<LigneRemonteeRcdDto> {
    brutes.sort_by(|a, b| {
        (&a.discipline, &a.niveau, &a.categorie, &a.mode).cmp(&(&b.discipline, &b.niveau, &b.categorie, &b.mode))
    });
    let mut fait: std::collections::HashSet<(String, String)> = Default::default();
    brutes
        .into_iter()
        .map(|b| {
            let cle = (b.discipline.clone(), b.niveau.clone());
            let p = if fait.insert(cle.clone()) { prevues.get(&cle).copied().unwrap_or(0.0) } else { 0.0 };
            LigneRemonteeRcdDto {
                discipline_code: b.discipline,
                niveau_code: b.niveau,
                categorie_nationale: b.categorie,
                mode_continuite: b.mode,
                heures_prevues: arrondi1(p),
                heures_absence: arrondi1(b.absence),
                heures_remplacees: arrondi1(b.remplacees),
            }
        })
        .collect()
}

/// Cellule CSV : guillemets doublés, et neutralisation d'une formule de tableur (`= + - @`).
pub fn cellule_csv(s: &str) -> String {
    let s = if s.starts_with(['=', '+', '-', '@', '\t', '\r']) { format!("'{s}") } else { s.to_string() };
    if s.contains([',', ';', '"', '\n', '\r']) {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s
    }
}

/// Tableau équivalent de la remontée (Q-AP3) : une ligne par (discipline, niveau, catégorie, mode), aucune personne.
pub fn csv_remontee(r: &RemonteeRcdDto) -> String {
    let mut s = String::from(
        "semaine,discipline,niveau,categorie_nationale,mode_continuite,heures_prevues,heures_absence,heures_remplacees\n",
    );
    for l in &r.lignes {
        s.push_str(&format!(
            "{},{},{},{},{},{:.1},{:.1},{:.1}\n",
            r.semaine,
            cellule_csv(&l.discipline_code),
            cellule_csv(&l.niveau_code),
            l.categorie_nationale,
            l.mode_continuite,
            l.heures_prevues,
            l.heures_absence,
            l.heures_remplacees
        ));
    }
    s
}

// ─────────────────────────────── base ───────────────────────────────

#[derive(Debug, Clone, sqlx::FromRow)]
struct PlanLigne {
    id: Uuid,
    annee_id: Uuid,
    version: i32,
    etat: String,
    objectifs: Option<String>,
    delai_prevenance_heures: i16,
    continuite_numerique: bool,
    referent_agent_id: Option<Uuid>,
    avis_cp_le: Option<NaiveDate>,
    presente_le: Option<NaiveDate>,
    instance: Option<String>,
    transmis_le: Option<NaiveDate>,
    campagne_lancee_le: Option<DateTime<Utc>>,
    comptes_rendus: i64,
    cree_le: DateTime<Utc>,
}

const PLAN_COLONNES: &str = "p.id, p.annee_id, p.version, p.etat, p.objectifs, p.delai_prevenance_heures,
    p.continuite_numerique, p.referent_agent_id, p.avis_cp_le, p.presente_le, p.instance, p.transmis_le,
    p.campagne_lancee_le, p.cree_le,
    (SELECT count(*) FROM ent_absp_rcd_compte_rendu c WHERE c.etablissement_id = p.etablissement_id AND c.plan_id = p.id) AS comptes_rendus";

impl From<PlanLigne> for PlanRcdDto {
    fn from(p: PlanLigne) -> Self {
        PlanRcdDto {
            id: p.id,
            annee_id: p.annee_id,
            version: p.version,
            etat: p.etat,
            objectifs: p.objectifs,
            delai_prevenance_heures: p.delai_prevenance_heures,
            continuite_numerique: p.continuite_numerique,
            referent_agent_id: p.referent_agent_id,
            avis_cp_le: p.avis_cp_le,
            presente_le: p.presente_le,
            instance: p.instance,
            transmis_le: p.transmis_le,
            campagne_lancee_le: p.campagne_lancee_le,
            comptes_rendus: p.comptes_rendus,
            cree_le: p.cree_le,
        }
    }
}

#[derive(Debug, Clone, sqlx::FromRow)]
struct EngagementLigne {
    id: Uuid,
    agent_id: Uuid,
    annee_id: Uuid,
    volume_heures: f64,
    statut: String,
    du: NaiveDate,
    au: NaiveDate,
    etat: String,
}

const ENGAGEMENT_COLONNES: &str = "id, agent_id, annee_id, volume_heures::float8 AS volume_heures, statut, du, au, etat";

impl From<EngagementLigne> for EngagementRcdDto {
    fn from(e: EngagementLigne) -> Self {
        EngagementRcdDto {
            id: e.id,
            agent_id: e.agent_id,
            annee_id: e.annee_id,
            volume_heures: e.volume_heures,
            statut: e.statut,
            du: e.du,
            au: e.au,
            etat: e.etat,
        }
    }
}

#[allow(clippy::too_many_arguments)]
async fn journal_objet(
    conn: &mut PgConnection,
    etab: Uuid,
    objet: &str,
    objet_id: Uuid,
    action: &str,
    colonne: Option<&str>,
    avant: Option<&str>,
    apres: Option<&str>,
    actor: Uuid,
) -> Result<(), EntDbError> {
    sqlx::query(
        "INSERT INTO ent_absp_journal
           (id, etablissement_id, objet, objet_id, action, auteur_user_id, colonne, valeur_avant, valeur_apres)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)",
    )
    .bind(Uuid::new_v4())
    .bind(etab)
    .bind(objet)
    .bind(objet_id)
    .bind(action)
    .bind(actor)
    .bind(colonne)
    .bind(avant)
    .bind(apres)
    .execute(conn)
    .await
    .map_err(db)?;
    Ok(())
}

async fn evenement_rcd(
    conn: &mut PgConnection,
    etab: Uuid,
    type_name: &str,
    aggregate_id: Uuid,
    payload: serde_json::Value,
    actor: Option<Uuid>,
) -> Result<(), EntDbError> {
    conn.publier(MetierEventDraft {
        etablissement_id: etab,
        type_name: type_name.to_string(),
        aggregate: AggregateKind::AbsenceRh,
        aggregate_id,
        eleve_id: None,
        payload,
        actor_user_id: actor.map(|u| u.to_string()),
    })
    .await
    .map_err(EntDbError::from)?;
    Ok(())
}

/// Année courante de l'établissement : (id, début, fin).
async fn annee_courante(conn: &mut PgConnection, etab: Uuid) -> Result<(Uuid, NaiveDate, NaiveDate), EntDbError> {
    sqlx::query_as("SELECT id, date_debut, date_fin FROM ent_annee_scolaire WHERE etablissement_id = $1 AND courante")
        .bind(etab)
        .fetch_optional(conn)
        .await
        .map_err(db)?
        .ok_or_else(|| conflit("aucune année scolaire courante pour cet établissement"))
}

async fn annee_de(conn: &mut PgConnection, etab: Uuid, id: Uuid) -> Result<(Uuid, NaiveDate, NaiveDate), EntDbError> {
    sqlx::query_as("SELECT id, date_debut, date_fin FROM ent_annee_scolaire WHERE etablissement_id = $1 AND id = $2")
        .bind(etab)
        .bind(id)
        .fetch_optional(conn)
        .await
        .map_err(db)?
        .ok_or_else(introuvable)
}

fn est_unique(e: &sqlx::Error) -> bool {
    matches!(e, sqlx::Error::Database(d) if d.code().as_deref() == Some("23505"))
}

fn est_garde(e: &sqlx::Error) -> bool {
    // Déclencheurs de garde (`RAISE EXCEPTION`, code P0001) : message métier, pas une panne.
    matches!(e, sqlx::Error::Database(d) if d.code().as_deref() == Some("P0001"))
}

impl PgEnt {
    async fn exiger_rcd_gerer(&self, etab: Uuid, actor: Uuid, role: &str) -> Result<(), EntDbError> {
        self.exiger_personnel_abs(role).await?;
        if !self.a_droit_abs(etab, actor, role, Action::PersonnelsRcdGerer).await? {
            return Err(EntDbError::Forbidden);
        }
        Ok(())
    }

    async fn exiger_rcd_remontee(&self, etab: Uuid, actor: Uuid, role: &str) -> Result<(), EntDbError> {
        self.exiger_personnel_abs(role).await?;
        if !self.a_droit_abs(etab, actor, role, Action::PersonnelsRcdRemontee).await? {
            return Err(EntDbError::Forbidden);
        }
        Ok(())
    }

    // ───────────────────────────── Plan annuel (ABP-220 à 225) ─────────────────────────────

    async fn plan_vivant(
        &self,
        conn: &mut PgConnection,
        etab: Uuid,
        plan_id: Uuid,
    ) -> Result<PlanLigne, EntDbError> {
        let p = sqlx::query_as::<_, PlanLigne>(&format!(
            "SELECT {PLAN_COLONNES} FROM ent_absp_rcd_plan p WHERE p.etablissement_id = $1 AND p.id = $2 FOR UPDATE OF p"
        ))
        .bind(etab)
        .bind(plan_id)
        .fetch_optional(&mut *conn)
        .await
        .map_err(db)?
        .ok_or_else(introuvable)?;
        Ok(p)
    }

    async fn plan_dto(&self, conn: &mut PgConnection, etab: Uuid, plan_id: Uuid) -> Result<PlanRcdDto, EntDbError> {
        let p = sqlx::query_as::<_, PlanLigne>(&format!(
            "SELECT {PLAN_COLONNES} FROM ent_absp_rcd_plan p WHERE p.etablissement_id = $1 AND p.id = $2"
        ))
        .bind(etab)
        .bind(plan_id)
        .fetch_one(conn)
        .await
        .map_err(db)?;
        Ok(p.into())
    }

    async fn verifier_contenu_plan(
        &self,
        conn: &mut PgConnection,
        etab: Uuid,
        body: &PlanRcdBody,
    ) -> Result<Option<String>, EntDbError> {
        if let Some(d) = body.delai_prevenance_heures {
            if !(0..=168).contains(&d) {
                return Err(invalide("Le délai de prévenance va de 0 à 168 heures."));
            }
        }
        if let Some(r) = body.referent_agent_id {
            let existe: bool =
                sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM ent_agent WHERE etablissement_id = $1 AND id = $2)")
                    .bind(etab)
                    .bind(r)
                    .fetch_one(conn)
                    .await
                    .map_err(db)?;
            if !existe {
                return Err(invalide("Le référent doit être un agent de l'établissement."));
            }
        }
        texte_borne(&body.objectifs, 1000, "Les objectifs sont limités à 1 000 caractères.")
    }

    /// ABP-220 : le plan de l'année, première version, en brouillon. Un seul plan vivant par année : pour en
    /// refaire un, on versionne (ABP-221).
    pub async fn creer_plan_rcd_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        body: &PlanRcdBody,
    ) -> Result<PlanRcdDto, EntDbError> {
        self.exiger_rcd_gerer(etab, actor, role).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let (annee, _, _) = match body.annee_id {
            Some(a) => annee_de(&mut tx, etab, a).await?,
            None => annee_courante(&mut tx, etab).await?,
        };
        let objectifs = self.verifier_contenu_plan(&mut tx, etab, body).await?;
        let version: i32 = sqlx::query_scalar(
            "SELECT COALESCE(max(version), 0) + 1 FROM ent_absp_rcd_plan WHERE etablissement_id = $1 AND annee_id = $2",
        )
        .bind(etab)
        .bind(annee)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        let id = Uuid::new_v4();
        let res = sqlx::query(
            "INSERT INTO ent_absp_rcd_plan
               (id, etablissement_id, annee_id, version, objectifs, delai_prevenance_heures, continuite_numerique,
                referent_agent_id, cree_par_user_id)
             VALUES ($1,$2,$3,$4,$5,COALESCE($6,24),COALESCE($7,false),$8,$9)",
        )
        .bind(id)
        .bind(etab)
        .bind(annee)
        .bind(version)
        .bind(&objectifs)
        .bind(body.delai_prevenance_heures)
        .bind(body.continuite_numerique)
        .bind(body.referent_agent_id)
        .bind(actor)
        .execute(&mut *tx)
        .await;
        if let Err(e) = res {
            return Err(if est_unique(&e) {
                conflit("un plan existe déjà pour cette année : créez-en une nouvelle version")
            } else {
                db(e)
            });
        }
        journal_objet(&mut tx, etab, "rcd_plan", id, "plan_cree", Some("version"), None, Some(&version.to_string()), actor)
            .await?;
        let d = self.plan_dto(&mut tx, etab, id).await?;
        tx.commit().await.map_err(db)?;
        Ok(d)
    }

    /// Modifie un **brouillon** (le contenu d'une version présentée ou transmise ne change plus : le déclencheur le
    /// refuse, le service le dit avant).
    pub async fn modifier_plan_rcd_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        plan_id: Uuid,
        body: &PlanRcdBody,
    ) -> Result<PlanRcdDto, EntDbError> {
        self.exiger_rcd_gerer(etab, actor, role).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let p = self.plan_vivant(&mut tx, etab, plan_id).await?;
        if p.etat != "brouillon" {
            return Err(conflit("ce plan a été présenté : créez une nouvelle version pour le modifier"));
        }
        let objectifs = self.verifier_contenu_plan(&mut tx, etab, body).await?;
        sqlx::query(
            "UPDATE ent_absp_rcd_plan SET
                objectifs = COALESCE($3, objectifs),
                delai_prevenance_heures = COALESCE($4, delai_prevenance_heures),
                continuite_numerique = COALESCE($5, continuite_numerique),
                referent_agent_id = COALESCE($6, referent_agent_id),
                modifie_le = now()
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(plan_id)
        .bind(&objectifs)
        .bind(body.delai_prevenance_heures)
        .bind(body.continuite_numerique)
        .bind(body.referent_agent_id)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        journal_objet(&mut tx, etab, "rcd_plan", plan_id, "plan_modifie", None, None, None, actor).await?;
        let d = self.plan_dto(&mut tx, etab, plan_id).await?;
        tx.commit().await.map_err(db)?;
        Ok(d)
    }

    /// ABP-221 : nouvelle version du plan. L'ancienne est archivée (et reste lisible) ; la nouvelle part en
    /// brouillon, avec le contenu de l'ancienne modifié par `body`, **sans** les faits de procédure : elle devra être
    /// de nouveau avisée, présentée et transmise.
    pub async fn versionner_plan_rcd_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        plan_id: Uuid,
        body: &PlanRcdBody,
    ) -> Result<PlanRcdDto, EntDbError> {
        self.exiger_rcd_gerer(etab, actor, role).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let p = self.plan_vivant(&mut tx, etab, plan_id).await?;
        if p.etat == "archive" {
            return Err(conflit("seule la version vivante du plan se versionne"));
        }
        if p.etat == "brouillon" {
            return Err(conflit("cette version est encore un brouillon : modifiez-la"));
        }
        let objectifs = self.verifier_contenu_plan(&mut tx, etab, body).await?;
        sqlx::query("UPDATE ent_absp_rcd_plan SET etat = 'archive', modifie_le = now() WHERE etablissement_id = $1 AND id = $2")
            .bind(etab)
            .bind(plan_id)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        let nouvelle = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_absp_rcd_plan
               (id, etablissement_id, annee_id, version, objectifs, delai_prevenance_heures, continuite_numerique,
                referent_agent_id, cree_par_user_id)
             VALUES ($1,$2,$3,$4,COALESCE($5,$6),COALESCE($7,$8),COALESCE($9,$10),COALESCE($11,$12),$13)",
        )
        .bind(nouvelle)
        .bind(etab)
        .bind(p.annee_id)
        .bind(p.version + 1)
        .bind(&objectifs)
        .bind(&p.objectifs)
        .bind(body.delai_prevenance_heures)
        .bind(p.delai_prevenance_heures)
        .bind(body.continuite_numerique)
        .bind(p.continuite_numerique)
        .bind(body.referent_agent_id)
        .bind(p.referent_agent_id)
        .bind(actor)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        journal_objet(
            &mut tx,
            etab,
            "rcd_plan",
            nouvelle,
            "plan_versionne",
            Some("version"),
            Some(&p.version.to_string()),
            Some(&(p.version + 1).to_string()),
            actor,
        )
        .await?;
        let d = self.plan_dto(&mut tx, etab, nouvelle).await?;
        tx.commit().await.map_err(db)?;
        Ok(d)
    }

    /// ABP-222 et ABP-223 : consigner l'avis du conseil pédagogique, la présentation (CA ou instance), la
    /// transmission au recteur. Les dates sont des faits : ni futures, ni dans le désordre ; une version transmise
    /// ne se corrige plus (nouvelle version).
    pub async fn procedure_plan_rcd_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        plan_id: Uuid,
        body: &ProcedurePlanRcdBody,
    ) -> Result<PlanRcdDto, EntDbError> {
        self.exiger_rcd_gerer(etab, actor, role).await?;
        if body.avis_cp_le.is_none() && body.presente_le.is_none() && body.instance.is_none() && body.transmis_le.is_none() {
            return Err(invalide("Indiquez au moins un fait de procédure."));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let p = self.plan_vivant(&mut tx, etab, plan_id).await?;
        if matches!(p.etat.as_str(), "archive" | "transmis") {
            return Err(conflit("cette version ne change plus : créez une nouvelle version du plan"));
        }
        let aujourdhui = jour_etab(&mut tx, etab).await?;
        let proc = Procedure {
            avis_cp_le: body.avis_cp_le.or(p.avis_cp_le),
            presente_le: body.presente_le.or(p.presente_le),
            instance: body.instance.clone().or(p.instance.clone()),
            transmis_le: body.transmis_le.or(p.transmis_le),
        };
        verifier_procedure(&proc, aujourdhui).map_err(invalide)?;
        let etat = etat_plan(&proc);
        sqlx::query(
            "UPDATE ent_absp_rcd_plan SET avis_cp_le = $3, presente_le = $4, instance = $5, transmis_le = $6,
                    etat = $7, modifie_le = now()
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(plan_id)
        .bind(proc.avis_cp_le)
        .bind(proc.presente_le)
        .bind(&proc.instance)
        .bind(proc.transmis_le)
        .bind(etat)
        .execute(&mut *tx)
        .await
        .map_err(|e| if est_garde(&e) { conflit("cette version ne change plus") } else { db(e) })?;
        if etat != p.etat {
            journal_objet(&mut tx, etab, "rcd_plan", plan_id, "plan_etat", Some("etat"), Some(&p.etat), Some(etat), actor)
                .await?;
        } else {
            journal_objet(&mut tx, etab, "rcd_plan", plan_id, "plan_procedure", None, None, None, actor).await?;
        }
        let d = self.plan_dto(&mut tx, etab, plan_id).await?;
        tx.commit().await.map_err(db)?;
        Ok(d)
    }

    /// ABP-225 : lancer la campagne de volontariat de l'année (une fois par version). L'événement porte seulement
    /// l'identifiant du plan et l'année ; la notification aux personnels est du socle 8.
    pub async fn lancer_campagne_rcd_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        plan_id: Uuid,
    ) -> Result<PlanRcdDto, EntDbError> {
        self.exiger_rcd_gerer(etab, actor, role).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let p = self.plan_vivant(&mut tx, etab, plan_id).await?;
        if p.etat == "archive" {
            return Err(conflit("la campagne se lance sur la version vivante du plan"));
        }
        if p.campagne_lancee_le.is_some() {
            return Err(conflit("la campagne de volontariat est déjà lancée pour cette version"));
        }
        sqlx::query("UPDATE ent_absp_rcd_plan SET campagne_lancee_le = now(), modifie_le = now() WHERE etablissement_id = $1 AND id = $2")
            .bind(etab)
            .bind(plan_id)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        journal_objet(&mut tx, etab, "rcd_plan", plan_id, "campagne_lancee", None, None, None, actor).await?;
        evenement_rcd(
            &mut tx,
            etab,
            EVT_RCD_CAMPAGNE,
            plan_id,
            json!({ "plan_id": plan_id, "annee_id": p.annee_id, "delai_prevenance_heures": p.delai_prevenance_heures }),
            Some(actor),
        )
        .await?;
        let d = self.plan_dto(&mut tx, etab, plan_id).await?;
        tx.commit().await.map_err(db)?;
        Ok(d)
    }

    /// ABP-224 : un compte rendu depuis les remontées **contrôlées ou transmises** de la période (agrégats, aucun
    /// nom). Au moins deux par an : la lecture du plan en donne le compte.
    pub async fn produire_compte_rendu_rcd_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        plan_id: Uuid,
        body: &CompteRenduRcdBody,
    ) -> Result<CompteRenduRcdDto, EntDbError> {
        self.exiger_rcd_gerer(etab, actor, role).await?;
        if body.periode_au < body.periode_du {
            return Err(invalide("La période du compte rendu est à l'envers."));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let p = self.plan_vivant(&mut tx, etab, plan_id).await?;
        if p.etat == "archive" {
            return Err(conflit("le compte rendu se rattache à la version vivante du plan"));
        }
        let (prevues, absence, remplacees, n): (f64, f64, f64, i64) = sqlx::query_as(
            "SELECT COALESCE(sum(l.heures_prevues), 0)::float8, COALESCE(sum(l.heures_absence), 0)::float8,
                    COALESCE(sum(l.heures_remplacees), 0)::float8, count(DISTINCT r.id)
               FROM ent_absp_rcd_remontee r
               LEFT JOIN ent_absp_rcd_remontee_ligne l ON l.etablissement_id = r.etablissement_id AND l.remontee_id = r.id
              WHERE r.etablissement_id = $1 AND r.semaine BETWEEN $2 AND $3 AND r.etat IN ('controlee', 'transmise')",
        )
        .bind(etab)
        .bind(body.periode_du)
        .bind(body.periode_au)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        if n == 0 {
            return Err(conflit("aucune remontée contrôlée sur cette période : rien à rendre compte"));
        }
        let aujourdhui = jour_etab(&mut tx, etab).await?;
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_absp_rcd_compte_rendu
               (id, etablissement_id, plan_id, le, periode_du, periode_au, heures_prevues, heures_absence,
                heures_remplacees, remontees, cree_par_user_id)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11)",
        )
        .bind(id)
        .bind(etab)
        .bind(plan_id)
        .bind(aujourdhui)
        .bind(body.periode_du)
        .bind(body.periode_au)
        .bind(arrondi1(prevues))
        .bind(arrondi1(absence))
        .bind(arrondi1(remplacees))
        .bind(n as i32)
        .bind(actor)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        journal_objet(&mut tx, etab, "rcd_plan", plan_id, "compte_rendu", None, None, None, actor).await?;
        tx.commit().await.map_err(db)?;
        Ok(CompteRenduRcdDto {
            id,
            plan_id,
            le: aujourdhui,
            periode_du: body.periode_du,
            periode_au: body.periode_au,
            heures_prevues: arrondi1(prevues),
            heures_absence: arrondi1(absence),
            heures_remplacees: arrondi1(remplacees),
            remontees: n as i32,
        })
    }

    /// Les versions du plan d'une année (la plus récente d'abord) : direction.
    pub async fn plans_rcd_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        annee_id: Option<Uuid>,
    ) -> Result<Vec<PlanRcdDto>, EntDbError> {
        self.exiger_rcd_gerer(etab, actor, role).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let annee = match annee_id {
            Some(a) => annee_de(&mut tx, etab, a).await?.0,
            None => annee_courante(&mut tx, etab).await?.0,
        };
        let v = sqlx::query_as::<_, PlanLigne>(&format!(
            "SELECT {PLAN_COLONNES} FROM ent_absp_rcd_plan p WHERE p.etablissement_id = $1 AND p.annee_id = $2
              ORDER BY p.version DESC"
        ))
        .bind(etab)
        .bind(annee)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        Ok(v.into_iter().map(Into::into).collect())
    }

    /// Le plan vivant de l'année courante, lisible de tout le personnel (le délai de prévenance et la continuité
    /// concernent chacun). Un brouillon n'est lisible que de la direction.
    pub async fn plan_courant_rcd_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
    ) -> Result<Option<PlanRcdDto>, EntDbError> {
        self.exiger_personnel_abs(role).await?;
        let gere = self.a_droit_abs(etab, actor, role, Action::PersonnelsRcdGerer).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let (annee, _, _) = annee_courante(&mut tx, etab).await?;
        let p = sqlx::query_as::<_, PlanLigne>(&format!(
            "SELECT {PLAN_COLONNES} FROM ent_absp_rcd_plan p
              WHERE p.etablissement_id = $1 AND p.annee_id = $2 AND p.etat <> 'archive'"
        ))
        .bind(etab)
        .bind(annee)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        Ok(p.filter(|p| gere || p.etat != "brouillon").map(Into::into))
    }

    // ───────────────────────────── Vivier (ABP-226 à 228, 213) ─────────────────────────────

    /// L'agent visé par une écriture d'engagement : soi-même, ou un autre avec `rcd.gerer`.
    async fn agent_engagement(
        &self,
        conn: &mut PgConnection,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        demande: Option<Uuid>,
    ) -> Result<Uuid, EntDbError> {
        let moi = agent_du_user_tx(conn, etab, actor).await?;
        match (demande, moi) {
            (Some(a), Some(m)) if a == m => Ok(a),
            (Some(a), _) => {
                if !self.a_droit_abs(etab, actor, role, Action::PersonnelsRcdGerer).await? {
                    return Err(EntDbError::Forbidden);
                }
                Ok(a)
            }
            (None, Some(m)) => Ok(m),
            (None, None) => Err(EntDbError::Forbidden),
        }
    }

    /// ABP-226 : s'engager comme volontaire (ou engager un collègue avec son accord, `rcd.gerer`).
    pub async fn engager_rcd_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        body: &EngagementRcdBody,
    ) -> Result<EngagementRcdDto, EntDbError> {
        self.exiger_personnel_abs(role).await?;
        if !STATUTS_ENGAGEMENT.contains(&body.statut.as_str()) {
            return Err(invalide("Statut inconnu : Pacte, HSE ou autre."));
        }
        if !(body.volume_heures > 0.0 && body.volume_heures <= 999.0) {
            return Err(invalide("Le volume annuel est de 1 à 999 heures."));
        }
        let volume = arrondi1(body.volume_heures);
        if body.au < body.du {
            return Err(invalide("La période de l'engagement est à l'envers."));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let agent = self.agent_engagement(&mut tx, etab, actor, role, body.agent_id).await?;
        let existe: bool =
            sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM ent_agent WHERE etablissement_id = $1 AND id = $2)")
                .bind(etab)
                .bind(agent)
                .fetch_one(&mut *tx)
                .await
                .map_err(db)?;
        if !existe {
            return Err(introuvable());
        }
        let (annee, debut, fin) = annee_courante(&mut tx, etab).await?;
        if body.du < debut || body.au > fin {
            return Err(invalide("L'engagement tient dans l'année scolaire courante."));
        }
        let id = Uuid::new_v4();
        let res = sqlx::query(
            "INSERT INTO ent_absp_rcd_engagement
               (id, etablissement_id, agent_id, annee_id, volume_heures, statut, du, au, cree_par_user_id)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)",
        )
        .bind(id)
        .bind(etab)
        .bind(agent)
        .bind(annee)
        .bind(volume)
        .bind(&body.statut)
        .bind(body.du)
        .bind(body.au)
        .bind(actor)
        .execute(&mut *tx)
        .await;
        if let Err(e) = res {
            return Err(if est_unique(&e) {
                conflit("cet agent est déjà engagé pour l'année : modifiez son engagement")
            } else {
                db(e)
            });
        }
        journal_objet(&mut tx, etab, "rcd_engagement", id, "engagement_cree", Some("statut"), None, Some(&body.statut), actor)
            .await?;
        let e = engagement_tx(&mut tx, etab, id).await?;
        tx.commit().await.map_err(db)?;
        Ok(e.into())
    }

    /// ABP-227 : modifier le volume ou le statut, ou mettre fin (à date, ou tout de suite). L'agent pour lui-même ;
    /// `rcd.gerer` pour tout autre. Une fin de période ne rouvre pas : un engagement terminé ne se modifie plus.
    pub async fn modifier_engagement_rcd_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        engagement_id: Uuid,
        body: &ModifierEngagementRcdBody,
    ) -> Result<EngagementRcdDto, EntDbError> {
        self.exiger_personnel_abs(role).await?;
        if let Some(s) = body.statut.as_deref() {
            if !STATUTS_ENGAGEMENT.contains(&s) {
                return Err(invalide("Statut inconnu : Pacte, HSE ou autre."));
            }
        }
        if let Some(v) = body.volume_heures {
            if !(v > 0.0 && v <= 999.0) {
                return Err(invalide("Le volume annuel est de 1 à 999 heures."));
            }
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let e = sqlx::query_as::<_, EngagementLigne>(&format!(
            "SELECT {ENGAGEMENT_COLONNES} FROM ent_absp_rcd_engagement WHERE etablissement_id = $1 AND id = $2 FOR UPDATE"
        ))
        .bind(etab)
        .bind(engagement_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?
        .ok_or_else(introuvable)?;
        // L'engagement d'un autre n'existe pas pour qui n'a pas `rcd.gerer` (pas d'oracle).
        if self.agent_engagement(&mut tx, etab, actor, role, Some(e.agent_id)).await.is_err() {
            return Err(introuvable());
        }
        if e.etat != "actif" {
            return Err(conflit("cet engagement est terminé"));
        }
        let aujourdhui = jour_etab(&mut tx, etab).await?;
        let mut au = body.au.unwrap_or(e.au);
        let mut etat = "actif";
        if body.terminer {
            etat = "termine";
            au = au.min(aujourdhui).max(e.du);
        }
        if au < e.du {
            return Err(invalide("La fin précède le début de l'engagement."));
        }
        let (_, _, fin_annee) = annee_de(&mut tx, etab, e.annee_id).await?;
        if au > fin_annee {
            return Err(invalide("L'engagement tient dans l'année scolaire."));
        }
        let volume = body.volume_heures.map(arrondi1).unwrap_or(e.volume_heures);
        let statut = body.statut.clone().unwrap_or_else(|| e.statut.clone());
        sqlx::query(
            "UPDATE ent_absp_rcd_engagement SET volume_heures = $3, statut = $4, au = $5, etat = $6, modifie_le = now()
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(engagement_id)
        .bind(volume)
        .bind(&statut)
        .bind(au)
        .bind(etat)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        if (volume - e.volume_heures).abs() > f64::EPSILON {
            journal_objet(
                &mut tx,
                etab,
                "rcd_engagement",
                engagement_id,
                "engagement_volume",
                Some("volume_heures"),
                Some(&format!("{:.1}", e.volume_heures)),
                Some(&format!("{volume:.1}")),
                actor,
            )
            .await?;
        }
        if statut != e.statut {
            journal_objet(&mut tx, etab, "rcd_engagement", engagement_id, "engagement_statut", Some("statut"), Some(&e.statut), Some(&statut), actor)
                .await?;
        }
        if au != e.au || etat != "actif" {
            journal_objet(
                &mut tx,
                etab,
                "rcd_engagement",
                engagement_id,
                if etat == "termine" { "engagement_termine" } else { "engagement_fin" },
                Some("au"),
                Some(&e.au.to_string()),
                Some(&au.to_string()),
                actor,
            )
            .await?;
        }
        let e = engagement_tx(&mut tx, etab, engagement_id).await?;
        tx.commit().await.map_err(db)?;
        Ok(e.into())
    }

    /// Compteurs bruts d'un engagement : (sollicitées, acceptées, refusées, refusées avec motif, heures acceptées,
    /// heures faites). « Faites » = séances acceptées dont l'heure de fin est passée.
    async fn compter_engagement(
        &self,
        conn: &mut PgConnection,
        etab: Uuid,
        e: &EngagementLigne,
    ) -> Result<(i64, i64, i64, i64, f64, f64), EntDbError> {
        let debut = instant(conn, etab, e.du, None).await?;
        let fin = instant(conn, etab, e.au + Duration::days(1), None).await?;
        sqlx::query_as(
            "SELECT count(*) FILTER (WHERE s.etat <> 'annulee'),
                    count(*) FILTER (WHERE s.etat IN ('acceptee', 'designee')),
                    count(*) FILTER (WHERE s.etat = 'refusee'),
                    count(*) FILTER (WHERE s.etat = 'refusee' AND s.refus_code IS NOT NULL),
                    COALESCE(sum(EXTRACT(EPOCH FROM (upper(s.plage) - lower(s.plage))) / 3600.0)
                             FILTER (WHERE s.etat IN ('acceptee', 'designee')), 0)::float8,
                    COALESCE(sum(EXTRACT(EPOCH FROM (upper(s.plage) - lower(s.plage))) / 3600.0)
                             FILTER (WHERE s.etat IN ('acceptee', 'designee') AND upper(s.plage) <= now()), 0)::float8
               FROM ent_absp_sollicitation s
              WHERE s.etablissement_id = $1 AND s.agent_id = $2 AND lower(s.plage) >= $3 AND lower(s.plage) < $4",
        )
        .bind(etab)
        .bind(e.agent_id)
        .bind(debut)
        .bind(fin)
        .fetch_one(conn)
        .await
        .map_err(db)
    }

    async fn compteurs_annee(
        &self,
        conn: &mut PgConnection,
        etab: Uuid,
        annee: Uuid,
    ) -> Result<Vec<CompteurRcdDto>, EntDbError> {
        let engagements = sqlx::query_as::<_, EngagementLigne>(&format!(
            "SELECT {ENGAGEMENT_COLONNES} FROM ent_absp_rcd_engagement
              WHERE etablissement_id = $1 AND annee_id = $2 AND etat = 'actif' ORDER BY cree_le"
        ))
        .bind(etab)
        .bind(annee)
        .fetch_all(&mut *conn)
        .await
        .map_err(db)?;
        let mut bruts = Vec::with_capacity(engagements.len());
        for e in &engagements {
            bruts.push(self.compter_engagement(&mut *conn, etab, e).await?);
        }
        let med = mediane(&bruts.iter().map(|b| b.4).collect::<Vec<_>>());
        Ok(engagements
            .into_iter()
            .zip(bruts)
            .map(|(e, (sol, acc, ref_, ref_m, h_acc, h_fait))| CompteurRcdDto {
                volume_atteint: h_acc >= e.volume_heures,
                heures_restantes: arrondi1((e.volume_heures - h_acc).max(0.0)),
                position: position(h_acc, med).to_string(),
                heures_acceptees: arrondi1(h_acc),
                heures_faites: arrondi1(h_fait),
                sollicitees: sol,
                acceptees: acc,
                refusees: ref_,
                refusees_motif_legitime: ref_m,
                engagement: e.into(),
            })
            .collect())
    }

    /// ABP-228 : l'état du vivier (volontaires, volume engagé, heures acceptées, et les compteurs de chacun).
    /// Direction seulement : les compteurs sont nominatifs.
    pub async fn vivier_rcd_for_actor(&self, etab: Uuid, actor: Uuid, role: &str) -> Result<VivierRcdDto, EntDbError> {
        self.exiger_rcd_gerer(etab, actor, role).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let (annee, _, _) = annee_courante(&mut tx, etab).await?;
        let compteurs = self.compteurs_annee(&mut tx, etab, annee).await?;
        Ok(VivierRcdDto {
            annee_id: annee,
            volontaires: compteurs.len() as i64,
            volume_engage: arrondi1(compteurs.iter().map(|c| c.engagement.volume_heures).sum()),
            heures_acceptees: arrondi1(compteurs.iter().map(|c| c.heures_acceptees).sum()),
            compteurs,
        })
    }

    /// ABP-213 : mon compteur de volontaire. Il ne contient que mes chiffres et ma **position** par rapport à la
    /// médiane (jamais les chiffres ni les noms des autres). Pas d'engagement : `None`.
    pub async fn mon_compteur_rcd_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
    ) -> Result<Option<CompteurRcdDto>, EntDbError> {
        self.exiger_personnel_abs(role).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let Some(moi) = agent_du_user_tx(&mut tx, etab, actor).await? else {
            return Ok(None);
        };
        let (annee, _, _) = annee_courante(&mut tx, etab).await?;
        Ok(self.compteurs_annee(&mut tx, etab, annee).await?.into_iter().find(|c| c.engagement.agent_id == moi))
    }

    // ───────────────────────────── Remontée hebdomadaire (ABP-229 à 231) ─────────────────────────────

    /// Lignes agrégées d'une semaine (sans aucune personne) depuis les impacts des absences de 14 jours au plus.
    async fn lignes_semaine(
        &self,
        conn: &mut PgConnection,
        etab: Uuid,
        semaine: NaiveDate,
    ) -> Result<Vec<LigneRemonteeRcdDto>, EntDbError> {
        let lignes: Vec<(String, String, String, String, f64, f64)> = sqlx::query_as(
            "SELECT lower(btrim(sc.matiere_libelle)), COALESCE(NULLIF(btrim(cl.niveau), ''), 'inconnu'),
                    COALESCE(n.categorie_nationale,
                             CASE WHEN a.cause = 'service' THEN 'institutionnel' ELSE 'en_attente_de_motif' END),
                    CASE r.type WHEN 'remplacement' THEN 'remplacement' WHEN 'echange' THEN 'remplacement'
                                WHEN 'autre_cours' THEN 'autre_cours' WHEN 'deplacement' THEN 'deplacement'
                                WHEN 'avance' THEN 'deplacement' WHEN 'continuite' THEN 'continuite'
                                WHEN 'annulation' THEN 'annulation' WHEN 'suppleance' THEN 'suppleance'
                                ELSE 'aucune' END,
                    sum(EXTRACT(EPOCH FROM (sc.heure_fin - sc.heure_debut)) / 3600.0)::float8,
                    COALESCE(sum(EXTRACT(EPOCH FROM (sc.heure_fin - sc.heure_debut)) / 3600.0)
                             FILTER (WHERE r.type IN ('remplacement', 'echange', 'suppleance')), 0)::float8
               FROM ent_absp_impact i
               JOIN ent_absp_absence a ON a.etablissement_id = i.etablissement_id AND a.id = i.absence_id
               JOIN ent_edt_seance sc ON sc.etablissement_id = i.etablissement_id AND sc.id = i.seance_id
               LEFT JOIN ent_classe cl ON cl.id = sc.classe_id
               LEFT JOIN ent_absp_nature n ON n.id = a.nature_id
               LEFT JOIN ent_absp_reponse r ON r.etablissement_id = i.etablissement_id AND r.id = i.reponse_active_id
              WHERE i.etablissement_id = $1 AND i.type = 'seance' AND sc.jour >= $2 AND sc.jour < $2 + 7
                AND a.etat NOT IN ('annulee', 'refusee', 'signalee') AND i.etat <> 'sans_objet'
                AND (a.fin - a.debut) <= interval '14 days'
              GROUP BY 1, 2, 3, 4",
        )
        .bind(etab)
        .bind(semaine)
        .fetch_all(&mut *conn)
        .await
        .map_err(db)?;
        let prevues: Vec<(String, String, f64)> = sqlx::query_as(
            "SELECT lower(btrim(sc.matiere_libelle)), COALESCE(NULLIF(btrim(cl.niveau), ''), 'inconnu'),
                    sum(EXTRACT(EPOCH FROM (sc.heure_fin - sc.heure_debut)) / 3600.0)::float8
               FROM ent_edt_seance sc LEFT JOIN ent_classe cl ON cl.id = sc.classe_id
              WHERE sc.etablissement_id = $1 AND sc.jour >= $2 AND sc.jour < $2 + 7
              GROUP BY 1, 2",
        )
        .bind(etab)
        .bind(semaine)
        .fetch_all(&mut *conn)
        .await
        .map_err(db)?;
        let borne = |s: &str, n: usize, defaut: &str| -> String {
            let t: String = s.chars().take(n).collect();
            if t.is_empty() { defaut.to_string() } else { t }
        };
        let prevues: std::collections::HashMap<(String, String), f64> =
            prevues.into_iter().map(|(d, n, h)| ((borne(&d, 40, "inconnue"), borne(&n, 20, "inconnu")), h)).collect();
        let brutes = lignes
            .into_iter()
            .map(|(d, n, c, m, h, r)| LigneBrute {
                discipline: borne(&d, 40, "inconnue"),
                niveau: borne(&n, 20, "inconnu"),
                categorie: c,
                mode: m,
                absence: h,
                remplacees: r,
            })
            .collect();
        Ok(lignes_remontee(brutes, &prevues))
    }

    /// Prépare (ou recalcule, tant qu'elle n'est pas transmise) la remontée d'une semaine. Écrit les lignes dans la
    /// même transaction que l'état et l'événement `rcd_remontee_prete`.
    pub(crate) async fn preparer_remontee_tx(
        &self,
        conn: &mut PgConnection,
        etab: Uuid,
        semaine: NaiveDate,
        actor: Option<Uuid>,
    ) -> Result<RemonteeRcdDto, EntDbError> {
        let existante: Option<(Uuid, String)> = sqlx::query_as(
            "SELECT id, etat FROM ent_absp_rcd_remontee WHERE etablissement_id = $1 AND semaine = $2 FOR UPDATE",
        )
        .bind(etab)
        .bind(semaine)
        .fetch_optional(&mut *conn)
        .await
        .map_err(db)?;
        let id = match existante {
            Some((_, etat)) if etat == "transmise" => {
                return Err(conflit("la remontée de cette semaine est déjà transmise : elle ne change plus"))
            }
            Some((id, _)) => {
                sqlx::query("DELETE FROM ent_absp_rcd_remontee_ligne WHERE etablissement_id = $1 AND remontee_id = $2")
                    .bind(etab)
                    .bind(id)
                    .execute(&mut *conn)
                    .await
                    .map_err(db)?;
                sqlx::query(
                    "UPDATE ent_absp_rcd_remontee SET etat = 'preparee', preparee_le = now(), controlee_par_user_id = NULL,
                            controlee_le = NULL, modifie_le = now()
                      WHERE etablissement_id = $1 AND id = $2",
                )
                .bind(etab)
                .bind(id)
                .execute(&mut *conn)
                .await
                .map_err(db)?;
                id
            }
            None => {
                let id = Uuid::new_v4();
                sqlx::query("INSERT INTO ent_absp_rcd_remontee (id, etablissement_id, semaine) VALUES ($1,$2,$3)")
                    .bind(id)
                    .bind(etab)
                    .bind(semaine)
                    .execute(&mut *conn)
                    .await
                    .map_err(db)?;
                id
            }
        };
        let lignes = self.lignes_semaine(&mut *conn, etab, semaine).await?;
        for l in &lignes {
            sqlx::query(
                "INSERT INTO ent_absp_rcd_remontee_ligne
                   (id, etablissement_id, remontee_id, discipline_code, niveau_code, categorie_nationale, mode_continuite,
                    heures_prevues, heures_absence, heures_remplacees)
                 VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)",
            )
            .bind(Uuid::new_v4())
            .bind(etab)
            .bind(id)
            .bind(&l.discipline_code)
            .bind(&l.niveau_code)
            .bind(&l.categorie_nationale)
            .bind(&l.mode_continuite)
            .bind(l.heures_prevues)
            .bind(l.heures_absence)
            .bind(l.heures_remplacees)
            .execute(&mut *conn)
            .await
            .map_err(db)?;
        }
        if let Some(a) = actor {
            journal_objet(conn, etab, "rcd_remontee", id, "remontee_preparee", Some("semaine"), None, Some(&semaine.to_string()), a)
                .await?;
        }
        evenement_rcd(
            conn,
            etab,
            EVT_RCD_REMONTEE_PRETE,
            id,
            json!({ "remontee_id": id, "semaine": semaine, "lignes": lignes.len() }),
            actor,
        )
        .await?;
        remontee_tx(conn, etab, id).await
    }

    /// ABP-229 : préparer la remontée d'une semaine (lundi ; absente : la semaine passée).
    pub async fn preparer_remontee_rcd_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        body: &PreparerRemonteeBody,
    ) -> Result<RemonteeRcdDto, EntDbError> {
        self.exiger_rcd_remontee(etab, actor, role).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let semaine = match body.semaine {
            Some(s) => s,
            None => semaine_par_defaut(jour_etab(&mut tx, etab).await?),
        };
        if lundi_de(semaine) != semaine {
            return Err(invalide("La semaine remontée est désignée par son lundi."));
        }
        let r = self.preparer_remontee_tx(&mut tx, etab, semaine, Some(actor)).await?;
        tx.commit().await.map_err(db)?;
        Ok(r)
    }

    async fn changer_etat_remontee(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        remontee_id: Uuid,
        depuis: &[&str],
        vers: &str,
    ) -> Result<RemonteeRcdDto, EntDbError> {
        self.exiger_rcd_remontee(etab, actor, role).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let (semaine, etat): (NaiveDate, String) = sqlx::query_as(
            "SELECT semaine, etat FROM ent_absp_rcd_remontee WHERE etablissement_id = $1 AND id = $2 FOR UPDATE",
        )
        .bind(etab)
        .bind(remontee_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?
        .ok_or_else(introuvable)?;
        if !depuis.contains(&etat.as_str()) {
            return Err(conflit("cette remontée n'est pas dans l'état attendu pour cette action"));
        }
        match vers {
            "controlee" => {
                sqlx::query(
                    "UPDATE ent_absp_rcd_remontee SET etat = 'controlee', controlee_par_user_id = $3, controlee_le = now(),
                            modifie_le = now() WHERE etablissement_id = $1 AND id = $2",
                )
                .bind(etab)
                .bind(remontee_id)
                .bind(actor)
                .execute(&mut *tx)
                .await
                .map_err(db)?;
            }
            "transmise" => {
                sqlx::query(
                    "UPDATE ent_absp_rcd_remontee SET etat = 'transmise', transmise_par_user_id = $3, transmise_le = now(),
                            modifie_le = now() WHERE etablissement_id = $1 AND id = $2",
                )
                .bind(etab)
                .bind(remontee_id)
                .bind(actor)
                .execute(&mut *tx)
                .await
                .map_err(db)?;
                // Le flux part vers le socle 9 (SI-RCD, Q-AP3) : l'événement ne porte que la semaine et l'identifiant.
                evenement_rcd(
                    &mut tx,
                    etab,
                    EVT_RCD_REMONTEE_TRANSMISE,
                    remontee_id,
                    json!({ "remontee_id": remontee_id, "semaine": semaine }),
                    Some(actor),
                )
                .await?;
            }
            _ => {
                sqlx::query("UPDATE ent_absp_rcd_remontee SET etat = $3, modifie_le = now() WHERE etablissement_id = $1 AND id = $2")
                    .bind(etab)
                    .bind(remontee_id)
                    .bind(vers)
                    .execute(&mut *tx)
                    .await
                    .map_err(db)?;
            }
        }
        journal_objet(&mut tx, etab, "rcd_remontee", remontee_id, "remontee_etat", Some("etat"), Some(&etat), Some(vers), actor)
            .await?;
        let r = remontee_tx(&mut tx, etab, remontee_id).await?;
        tx.commit().await.map_err(db)?;
        Ok(r)
    }

    /// ABP-230 : contrôler la remontée (elle a été relue ligne à ligne).
    pub async fn controler_remontee_rcd_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        id: Uuid,
    ) -> Result<RemonteeRcdDto, EntDbError> {
        self.changer_etat_remontee(etab, actor, role, id, &["preparee"], "controlee").await
    }

    /// ABP-230 : refuser de laisser partir une remontée (à recalculer).
    pub async fn rejeter_remontee_rcd_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        id: Uuid,
    ) -> Result<RemonteeRcdDto, EntDbError> {
        self.changer_etat_remontee(etab, actor, role, id, &["preparee", "controlee"], "rejetee").await
    }

    /// ABP-230 : laisser partir. **Seulement après contrôle** : une remontée jamais relue ne part pas.
    pub async fn transmettre_remontee_rcd_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        id: Uuid,
    ) -> Result<RemonteeRcdDto, EntDbError> {
        self.changer_etat_remontee(etab, actor, role, id, &["controlee"], "transmise").await
    }

    /// Les remontées des dernières semaines (la plus récente d'abord), lignes comprises.
    pub async fn remontees_rcd_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
    ) -> Result<Vec<RemonteeRcdDto>, EntDbError> {
        self.exiger_rcd_remontee(etab, actor, role).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let ids: Vec<Uuid> = sqlx::query_scalar(
            "SELECT id FROM ent_absp_rcd_remontee WHERE etablissement_id = $1 ORDER BY semaine DESC LIMIT 60",
        )
        .bind(etab)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let mut v = Vec::with_capacity(ids.len());
        for id in ids {
            v.push(remontee_tx(&mut tx, etab, id).await?);
        }
        Ok(v)
    }

    /// ABP-231 : le tableau équivalent en CSV (en l'absence d'API, Q-AP3), tracé au journal. Une remontée qui n'a pas
    /// été contrôlée ne s'exporte pas.
    pub async fn exporter_remontee_rcd_csv_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        id: Uuid,
    ) -> Result<String, EntDbError> {
        self.exiger_rcd_remontee(etab, actor, role).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let r = remontee_tx(&mut tx, etab, id).await?;
        if !matches!(r.etat.as_str(), "controlee" | "transmise") {
            return Err(conflit("la remontée doit être contrôlée avant d'être exportée"));
        }
        journal_objet(&mut tx, etab, "rcd_remontee", id, "remontee_exportee", Some("semaine"), None, Some(&r.semaine.to_string()), actor)
            .await?;
        tx.commit().await.map_err(db)?;
        Ok(csv_remontee(&r))
    }

    /// Tâche planifiée : chaque lundi (jour de l'établissement), prépare la remontée de la semaine passée pour les
    /// établissements qui ont un plan RCD vivant. Idempotente : une remontée existante n'est jamais recalculée ici.
    pub async fn preparer_remontees_hebdomadaires(&self) -> Result<BilanRemonteesDto, EntDbError> {
        let mut bilan = BilanRemonteesDto::default();
        for etab in etablissements_systeme(self.pool(), ContexteSysteme::JobPlanifie("absences_personnels_remontee_rcd"))
            .await
            .map_err(db)?
        {
            let r: Result<bool, EntDbError> = async {
                let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
                let a_un_plan: bool = sqlx::query_scalar(
                    "SELECT EXISTS (SELECT 1 FROM ent_absp_rcd_plan WHERE etablissement_id = $1 AND etat <> 'archive')",
                )
                .bind(etab)
                .fetch_one(&mut *tx)
                .await
                .map_err(db)?;
                if !a_un_plan {
                    return Ok(false);
                }
                let semaine = semaine_par_defaut(jour_etab(&mut tx, etab).await?);
                let deja: bool = sqlx::query_scalar(
                    "SELECT EXISTS (SELECT 1 FROM ent_absp_rcd_remontee WHERE etablissement_id = $1 AND semaine = $2)",
                )
                .bind(etab)
                .bind(semaine)
                .fetch_one(&mut *tx)
                .await
                .map_err(db)?;
                if deja {
                    return Ok(false);
                }
                self.preparer_remontee_tx(&mut tx, etab, semaine, None).await?;
                tx.commit().await.map_err(db)?;
                Ok(true)
            }
            .await;
            bilan.etablissements += 1;
            match r {
                Ok(true) => bilan.preparees += 1,
                Ok(false) => {}
                Err(e) => tracing::warn!(%etab, "remontée RCD : {e}"),
            }
        }
        Ok(bilan)
    }
}

async fn engagement_tx(conn: &mut PgConnection, etab: Uuid, id: Uuid) -> Result<EngagementLigne, EntDbError> {
    sqlx::query_as::<_, EngagementLigne>(&format!(
        "SELECT {ENGAGEMENT_COLONNES} FROM ent_absp_rcd_engagement WHERE etablissement_id = $1 AND id = $2"
    ))
    .bind(etab)
    .bind(id)
    .fetch_optional(conn)
    .await
    .map_err(db)?
    .ok_or_else(introuvable)
}

async fn remontee_tx(conn: &mut PgConnection, etab: Uuid, id: Uuid) -> Result<RemonteeRcdDto, EntDbError> {
    let (semaine, etat, preparee_le, controlee_le, transmise_le): (
        NaiveDate,
        String,
        DateTime<Utc>,
        Option<DateTime<Utc>>,
        Option<DateTime<Utc>>,
    ) = sqlx::query_as(
        "SELECT semaine, etat, preparee_le, controlee_le, transmise_le FROM ent_absp_rcd_remontee
          WHERE etablissement_id = $1 AND id = $2",
    )
    .bind(etab)
    .bind(id)
    .fetch_optional(&mut *conn)
    .await
    .map_err(db)?
    .ok_or_else(introuvable)?;
    let lignes: Vec<(String, String, String, String, f64, f64, f64)> = sqlx::query_as(
        "SELECT discipline_code, niveau_code, categorie_nationale, mode_continuite, heures_prevues::float8,
                heures_absence::float8, heures_remplacees::float8
           FROM ent_absp_rcd_remontee_ligne WHERE etablissement_id = $1 AND remontee_id = $2
          ORDER BY discipline_code, niveau_code, categorie_nationale, mode_continuite",
    )
    .bind(etab)
    .bind(id)
    .fetch_all(conn)
    .await
    .map_err(db)?;
    Ok(RemonteeRcdDto {
        id,
        semaine,
        etat,
        preparee_le,
        controlee_le,
        transmise_le,
        lignes: lignes
            .into_iter()
            .map(|(d, n, c, m, p, a, r)| LigneRemonteeRcdDto {
                discipline_code: d,
                niveau_code: n,
                categorie_nationale: c,
                mode_continuite: m,
                heures_prevues: p,
                heures_absence: a,
                heures_remplacees: r,
            })
            .collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn j(m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2027, m, d).unwrap()
    }

    #[test]
    fn semaine_par_defaut_est_le_lundi_de_la_semaine_passee() {
        // mardi 16 novembre 2027 → lundi 8 novembre ; un lundi vise la semaine d'avant, pas la sienne.
        assert_eq!(semaine_par_defaut(j(11, 16)), j(11, 8));
        assert_eq!(semaine_par_defaut(j(11, 15)), j(11, 8));
        assert_eq!(semaine_par_defaut(j(11, 21)), j(11, 8), "dimanche");
        assert_eq!(lundi_de(j(11, 15)), j(11, 15));
    }

    #[test]
    fn la_procedure_se_deduit_des_faits_et_reste_coherente() {
        let mut p = Procedure::default();
        assert_eq!(etat_plan(&p), "brouillon");
        p.avis_cp_le = Some(j(9, 10));
        assert_eq!(etat_plan(&p), "brouillon", "l'avis seul ne présente pas le plan");
        p.presente_le = Some(j(9, 20));
        p.instance = Some("ca".into());
        assert_eq!(etat_plan(&p), "presente");
        assert!(verifier_procedure(&p, j(10, 1)).is_ok());
        p.transmis_le = Some(j(9, 25));
        assert_eq!(etat_plan(&p), "transmis");
        assert!(verifier_procedure(&p, j(10, 1)).is_ok());
        // Désordre, futur, transmission sans présentation, instance inconnue.
        let mut q = p.clone();
        q.presente_le = Some(j(9, 5));
        assert!(verifier_procedure(&q, j(10, 1)).is_err(), "l'avis suit la présentation");
        q = p.clone();
        q.transmis_le = Some(j(9, 15));
        assert!(verifier_procedure(&q, j(10, 1)).is_err(), "transmission avant présentation");
        assert!(verifier_procedure(&p, j(9, 24)).is_err(), "transmission dans le futur");
        q = Procedure { transmis_le: Some(j(9, 1)), ..Default::default() };
        assert!(verifier_procedure(&q, j(10, 1)).is_err(), "pas de transmission sans présentation");
        q = p.clone();
        q.instance = Some("autre".into());
        assert!(verifier_procedure(&q, j(10, 1)).is_err());
    }

    #[test]
    fn mediane_et_position_ne_nomment_personne() {
        assert_eq!(mediane(&[]), 0.0);
        assert_eq!(mediane(&[3.0]), 3.0);
        assert_eq!(mediane(&[1.0, 9.0, 4.0]), 4.0);
        assert_eq!(mediane(&[1.0, 2.0, 4.0, 10.0]), 3.0);
        assert_eq!(position(1.0, 3.0), "en_dessous");
        assert_eq!(position(3.04, 3.0), "dans_la_mediane");
        assert_eq!(position(8.0, 3.0), "au_dessus");
    }

    #[test]
    fn les_heures_prevues_ne_sont_portees_qu_une_fois_par_discipline_et_niveau() {
        let mut prevues = HashMap::new();
        prevues.insert(("maths".to_string(), "4e".to_string()), 12.0);
        let b = |c: &str, m: &str, a: f64| LigneBrute {
            discipline: "maths".into(),
            niveau: "4e".into(),
            categorie: c.into(),
            mode: m.into(),
            absence: a,
            remplacees: 0.0,
        };
        let l = lignes_remontee(
            vec![b("raison_sante", "aucune", 2.0), b("formation", "remplacement", 3.0), b("formation", "aucune", 1.0)],
            &prevues,
        );
        assert_eq!(l.iter().map(|x| x.heures_prevues).sum::<f64>(), 12.0, "pas de double compte : {l:?}");
        assert_eq!(l[0].categorie_nationale, "formation", "tri stable par catégorie puis mode");
        assert_eq!(l[0].heures_prevues, 12.0);
        assert_eq!(l.iter().map(|x| x.heures_absence).sum::<f64>(), 6.0);
    }

    #[test]
    fn le_csv_neutralise_les_formules_et_n_a_aucune_personne() {
        assert_eq!(cellule_csv("maths"), "maths");
        assert_eq!(cellule_csv("=2+2"), "'=2+2");
        assert_eq!(cellule_csv("a,b"), "\"a,b\"");
        assert_eq!(cellule_csv("@x"), "'@x");
        let r = RemonteeRcdDto {
            id: Uuid::nil(),
            semaine: j(11, 8),
            etat: "controlee".into(),
            preparee_le: Utc::now(),
            controlee_le: None,
            transmise_le: None,
            lignes: vec![LigneRemonteeRcdDto {
                discipline_code: "=cmd".into(),
                niveau_code: "4e".into(),
                categorie_nationale: "formation".into(),
                mode_continuite: "aucune".into(),
                heures_prevues: 12.0,
                heures_absence: 2.0,
                heures_remplacees: 0.0,
            }],
        };
        let c = csv_remontee(&r);
        assert!(c.starts_with("semaine,discipline,niveau,categorie_nationale,mode_continuite,"));
        assert!(c.contains("2027-11-08,'=cmd,4e,formation,aucune,12.0,2.0,0.0"), "{c}");
        assert!(!c.contains(&Uuid::nil().to_string()), "aucun identifiant dans le tableau");
    }
}
