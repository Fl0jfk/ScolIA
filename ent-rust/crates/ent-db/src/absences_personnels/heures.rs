//! Socle 15, lot A7 — heures constatées, états mensuels, éléments variables de paie, DSN
//! (`docs/socles/15_ABSENCES_PERSONNELS.md` §14 ; R-04, R-05, R-06, R-16, R-AP28, R-AP29, R-AP30, R-AP33 ; ABP-200 à 219).
//!
//! ScolIA **constate**, il ne calcule aucune rémunération (R-05). Une heure naît d'un **fait** (R-AP28) : une séance
//! remplacée **assurée** dont l'appel est fait au socle 12, ou que le remplaçant (ou la vie scolaire) atteste. Jamais
//! d'une déclaration seule ; en base, une heure sans séance ne peut pas être une heure de remplacement.
//!
//! | Commande                                      | Droit (D-017)                                          | ABP        |
//! |-----------------------------------------------|--------------------------------------------------------|------------|
//! | tâche `constater_heures_assurees`             | système (appel fait → heure `constatee`)               | 200, 218   |
//! | `attester_seance_for_actor`                   | le remplaçant, ou la vie scolaire (CPE, direction)     | 202        |
//! | `mes_heures_for_actor`                        | soi-même                                               | 200        |
//! | `heures_for_actor` / `etat_annuel_agent_…`    | `heures.lire` (périmètre d'employeur) ; soi-même       | 214, 219   |
//! | `contester_heure_for_actor`                   | l'agent, ou `heures.valider`                           | 201        |
//! | `corriger_heure_for_actor` / `saisir_heure_…` | `heures.valider` (jamais ses propres heures)           | 201, 203   |
//! | `valider_heures_for_actor`                    | `heures.valider`                                       | 203        |
//! | `figer_etat_for_actor`                        | `heures.valider`, ou `heures.exporter` (son employeur) | 204        |
//! | `signer_etat_for_actor`                       | `heures.signer` (direction)                            | 205        |
//! | `exporter_etat_csv_for_actor`                 | `heures.exporter` (son employeur)                      | 206        |
//! | `regulariser_heure_for_actor`                 | `heures.valider` ou `heures.exporter`                  | 208        |
//! | `preparer_elements_paie_…` / `exporter_…`     | `personnels.paie.absences` (employeur OGEC)            | 207, 209   |
//! | `preparer_dsn_…` / `dsn_file_…` / `marquer_…` | `personnels.paie.absences`                             | 210, 211   |
//!
//! **Garanties.**
//! * **R-AP28** : service (preuve vérifiée sur la séance) **et** base (`ent_absp_heure_fait_chk`, `_saisie_chk`).
//! * **Figée ou exportée, une heure ne change plus** (déclencheur) ; une correction est une **régularisation** : un
//!   fait en ajout seul, reporté une seule fois comme ligne signée sur l'état suivant, rattachée à la ligne d'origine
//!   (D-027 E). La durée de l'heure elle-même reste celle qui a été exportée.
//! * **Un état mensuel par mois et employeur**, figé après le mois, sans heure contestée ni à valider, signé par la
//!   direction avant d'être exporté ; chaque export est tracé au journal.
//! * **Éléments de paie** : un par (absence ou heure, mois, nature) — unicité **en base** ; préparation rejouable ;
//!   un mois déjà envoyé reçoit un élément de régularisation sur le mois ouvert, jamais une réécriture (R-AP29).
//! * Cloisonnement par employeur : le secrétariat ne lit que l'État, la comptabilité que l'OGEC (§23.3).
//! * Personne ne valide, ne corrige ni ne régularise ses propres heures ; aucun événement ne porte de nature
//!   d'absence ni de texte libre.

use super::rcd::{arrondi1, cellule_csv, evenement_rcd, journal_objet, lundi_de};
use super::suppleance::jour_etab;
use super::*;
use crate::tenant_tx::{etablissements_systeme, ContexteSysteme};

pub const NATURES_HEURE: &[&str] = &["rcd", "hse", "mission_pacte", "rattrapage_du", "service_non_fait", "vacation"];
/// Natures qu'une saisie sans séance peut porter (R-AP28) : jamais un remplacement.
const NATURES_SAISIE: &[&str] = &["hse", "mission_pacte", "vacation", "service_non_fait"];
const RAISONS: &[&str] = &["duree_erronee", "seance_non_assuree", "mauvais_agent", "doublon", "oubli", "autre"];

pub const EVT_HEURE_CONSTATEE: &str = "absence_personnel.heure_constatee";
pub const EVT_HEURES_VALIDEES: &str = "absence_personnel.heures_validees";
pub const EVT_HEURE_A_VERIFIER: &str = "absence_personnel.heure_a_verifier";
pub const EVT_ETAT_SIGNE: &str = "absence_personnel.etat_heures_signe";
pub const EVT_ETAT_EXPORTE: &str = "absence_personnel.etat_heures_exporte";

// ─────────────────────────────── logique pure (testée sans base) ───────────────────────────────

/// Nature d'une heure de remplacement selon l'engagement de volontariat du remplaçant (R-04) : Pacte → mission
/// Pacte, HSE → HSE, sinon (pas d'engagement, statut « autre ») heure de RCD.
pub fn nature_remplacement(statut_engagement: Option<&str>) -> &'static str {
    match statut_engagement {
        Some("pacte") => "mission_pacte",
        Some("hse") => "hse",
        _ => "rcd",
    }
}

/// Durée en minutes d'une séance.
pub fn minutes_entre(debut: NaiveTime, fin: NaiveTime) -> i32 {
    ((fin - debut).num_minutes()).clamp(0, 1440) as i32
}

/// Premier jour du mois d'une date.
pub fn premier_du_mois(d: NaiveDate) -> NaiveDate {
    use chrono::Datelike;
    NaiveDate::from_ymd_opt(d.year(), d.month(), 1).unwrap_or(d)
}

/// Dernier jour du mois d'un premier du mois.
pub fn dernier_du_mois(mois: NaiveDate) -> NaiveDate {
    use chrono::Datelike;
    let (a, m) = if mois.month() == 12 { (mois.year() + 1, 1) } else { (mois.year(), mois.month() + 1) };
    NaiveDate::from_ymd_opt(a, m, 1).map_or(mois, |p| p - Duration::days(1))
}

/// Mois suivant (premier jour).
pub fn mois_suivant(mois: NaiveDate) -> NaiveDate {
    dernier_du_mois(mois) + Duration::days(1)
}

/// Échéance du signalement DSN d'un arrêt : cinq jours calendaires après sa connaissance (R-16).
pub fn echeance_dsn(connaissance: NaiveDate) -> NaiveDate {
    connaissance + Duration::days(5)
}

/// Quantité en centièmes d'heure (colonne `NUMERIC(7,2)`), arrondie au plus proche.
pub fn centiemes_heure(minutes: i64) -> i64 {
    (minutes * 100 + if minutes >= 0 { 30 } else { -30 }) / 60
}

/// Employeurs qu'un rôle de base peut lire et exporter (§23.3) : la direction les deux, le secrétariat l'État,
/// la comptabilité l'OGEC ; aucun autre profil.
pub fn employeurs_permis(r: Role) -> &'static [&'static str] {
    match r {
        Role::Direction => &["etat", "ogec"],
        Role::Administratif => &["etat"],
        Role::Compta => &["ogec"],
        _ => &[],
    }
}

/// Une heure constatée est-elle comptée en minutes effectives : durée + régularisations.
pub fn duree_effective(duree: i32, regularisations: i64) -> i64 {
    i64::from(duree) + regularisations
}

/// Tableau d'un état mensuel (CSV, `;`-compatible tableur français non requis : virgules). Aucune formule : les
/// cellules libres sont neutralisées.
pub fn csv_etat(e: &EtatMensuelDto, noms: &std::collections::HashMap<Uuid, String>) -> String {
    let mut s = String::from("mois,employeur,agent_id,agent,nature,minutes,regularisation\n");
    for l in &e.lignes {
        s.push_str(&format!(
            "{},{},{},{},{},{},{}\n",
            e.mois,
            e.employeur,
            l.agent_id,
            cellule_csv(noms.get(&l.agent_id).map_or("", String::as_str)),
            l.nature,
            l.duree_minutes,
            if l.regularisation_de_id.is_some() { "oui" } else { "non" }
        ));
    }
    s
}

/// Éléments variables de paie d'un mois (CSV). Natures codées uniquement.
pub fn csv_paie(mois: NaiveDate, elements: &[ElementPaieDto], noms: &std::collections::HashMap<Uuid, String>) -> String {
    let mut s = String::from("mois,agent_id,agent,nature,quantite,unite,regularisation\n");
    for e in elements {
        s.push_str(&format!(
            "{},{},{},{},{:.2},{},{}\n",
            mois,
            e.agent_id,
            cellule_csv(noms.get(&e.agent_id).map_or("", String::as_str)),
            e.nature_code,
            e.quantite,
            e.unite,
            if e.regularise_de_id.is_some() { "oui" } else { "non" }
        ));
    }
    s
}

// ─────────────────────────────── base ───────────────────────────────

#[derive(Debug, Clone, sqlx::FromRow)]
struct HeureLigne {
    id: Uuid,
    agent_id: Uuid,
    seance_id: Option<Uuid>,
    nature: String,
    duree_minutes: i32,
    regul_minutes: i64,
    le: NaiveDate,
    mois: NaiveDate,
    employeur: String,
    etat: String,
    preuve: String,
    etat_id: Option<Uuid>,
}

const HEURE_COLONNES: &str = "h.id, h.agent_id, h.seance_id, h.nature, h.duree_minutes, h.le, h.mois, h.employeur, h.etat,
    h.preuve, h.etat_id,
    COALESCE((SELECT sum(f.duree_apres - f.duree_avant) FROM ent_absp_heure_fait f
               WHERE f.etablissement_id = h.etablissement_id AND f.heure_id = h.id AND f.type = 'regularisation'), 0)::int8
      AS regul_minutes";

impl From<HeureLigne> for HeureDto {
    fn from(h: HeureLigne) -> Self {
        HeureDto {
            id: h.id,
            agent_id: h.agent_id,
            seance_id: h.seance_id,
            nature: h.nature,
            duree_minutes: h.duree_minutes,
            regularisation_minutes: h.regul_minutes as i32,
            le: h.le,
            mois: h.mois,
            employeur: h.employeur,
            etat: h.etat,
            preuve: h.preuve,
            etat_mensuel_id: h.etat_id,
        }
    }
}

async fn heure_tx(conn: &mut PgConnection, etab: Uuid, id: Uuid, verrou: bool) -> Result<HeureLigne, EntDbError> {
    sqlx::query_as::<_, HeureLigne>(&format!(
        "SELECT {HEURE_COLONNES} FROM ent_absp_heure h WHERE h.etablissement_id = $1 AND h.id = $2 {}",
        if verrou { "FOR UPDATE OF h" } else { "" }
    ))
    .bind(etab)
    .bind(id)
    .fetch_optional(conn)
    .await
    .map_err(db)?
    .ok_or_else(introuvable)
}

/// Classe de l'employeur d'un agent à un jour : l'engagement qui couvre le jour (le plus récent), sinon l'État.
async fn employeur_agent(conn: &mut PgConnection, etab: Uuid, agent: Uuid, jour: NaiveDate) -> Result<&'static str, EntDbError> {
    let t: Option<String> = sqlx::query_scalar(
        "SELECT employeur_type FROM ent_agent_engagement
          WHERE etablissement_id = $1 AND agent_id = $2 AND du <= $3 AND (au IS NULL OR au >= $3)
          ORDER BY du DESC LIMIT 1",
    )
    .bind(etab)
    .bind(agent)
    .bind(jour)
    .fetch_optional(conn)
    .await
    .map_err(db)?;
    Ok(t.as_deref().map_or("etat", classe_employeur))
}

#[allow(clippy::too_many_arguments)]
async fn fait_heure(
    conn: &mut PgConnection,
    etab: Uuid,
    heure_id: Uuid,
    type_: &str,
    auteur: Option<Uuid>,
    raison: Option<&str>,
    note: Option<&str>,
    duree: (Option<i32>, Option<i32>),
    etat: (Option<&str>, Option<&str>),
) -> Result<Uuid, EntDbError> {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO ent_absp_heure_fait
           (id, etablissement_id, heure_id, type, auteur_user_id, raison_code, note, duree_avant, duree_apres,
            etat_avant, etat_apres)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11)",
    )
    .bind(id)
    .bind(etab)
    .bind(heure_id)
    .bind(type_)
    .bind(auteur)
    .bind(raison)
    .bind(note)
    .bind(duree.0)
    .bind(duree.1)
    .bind(etat.0)
    .bind(etat.1)
    .execute(conn)
    .await
    .map_err(db)?;
    Ok(id)
}

/// Heure à constater : le port d'écriture du socle 15 (`heures::constater`). Seul le 15 écrit dans ses tables ; un autre
/// socle (le N1 pour des HSE de projet, par exemple) appelle cette commande et ne lit que `heures_constatees`.
#[derive(Clone, Debug)]
pub struct ConstaterHeure {
    pub agent_id: Uuid,
    pub seance_id: Option<Uuid>,
    pub impact_id: Option<Uuid>,
    pub nature: &'static str,
    pub duree_minutes: i32,
    pub le: NaiveDate,
    /// `appel` · `attestation` (avec séance) ; `saisie` (sans séance).
    pub preuve: &'static str,
    pub auteur_user_id: Option<Uuid>,
}

/// Séance remplacée, telle que le 15 et le socle 10 la portent.
#[derive(Debug, Clone, sqlx::FromRow)]
struct SeanceAssuree {
    impact_id: Uuid,
    remplacant_agent_id: Uuid,
    jour: NaiveDate,
    minutes: i32,
    fin_instant: DateTime<Utc>,
    appel_fait: bool,
}

const SEANCE_ASSUREE_SQL: &str = "SELECT i.id AS impact_id, r.remplacant_agent_id, sc.jour,
        (EXTRACT(EPOCH FROM (sc.heure_fin - sc.heure_debut)) / 60)::int AS minutes,
        ((sc.jour + sc.heure_fin)::timestamp AT TIME ZONE COALESCE(
            (SELECT fuseau FROM ent_etablissement_parametre WHERE etablissement_id = i.etablissement_id), 'Europe/Paris'))
          AS fin_instant,
        EXISTS (SELECT 1 FROM ent_appel ap WHERE ap.etablissement_id = i.etablissement_id AND ap.seance_id = sc.id
                   AND ap.etat = 'valide') AS appel_fait
   FROM ent_absp_impact i
   JOIN ent_edt_seance sc ON sc.etablissement_id = i.etablissement_id AND sc.id = i.seance_id
   JOIN ent_absp_reponse r ON r.etablissement_id = i.etablissement_id AND r.id = i.reponse_active_id AND r.etat = 'active'
  WHERE i.etablissement_id = $1 AND i.type = 'seance' AND i.seance_id = $2 AND i.etat = 'remplace'
    AND r.type = 'remplacement' AND r.remplacant_agent_id IS NOT NULL";

impl PgEnt {
    async fn exiger_heures_valider(&self, etab: Uuid, actor: Uuid, role: &str) -> Result<Role, EntDbError> {
        let r = self.exiger_personnel_abs(role).await?;
        if !self.a_droit_abs(etab, actor, role, Action::PersonnelsHeuresValider).await? {
            return Err(EntDbError::Forbidden);
        }
        Ok(r)
    }

    /// Droit sur un employeur : l'action ET le périmètre du rôle de base (§23.3).
    async fn exiger_employeur(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        action: Action,
        employeur: &str,
    ) -> Result<(), EntDbError> {
        let r = self.exiger_personnel_abs(role).await?;
        if !self.a_droit_abs(etab, actor, role, action).await? || !employeurs_permis(r).contains(&employeur) {
            return Err(EntDbError::Forbidden);
        }
        Ok(())
    }

    // ───────────────────────────── Constat d'une heure (R-AP28) ─────────────────────────────

    /// Port `heures::constater` : écrit une heure constatée (ou ne fait rien si le fait existe déjà). Aucune heure sans
    /// fait : la preuve doit correspondre à la forme (séance avec appel ou attestation ; saisie sans séance, jamais
    /// un remplacement) — sinon refus, et la base refuse de toute façon.
    pub(crate) async fn constater_heure_tx(
        &self,
        conn: &mut PgConnection,
        etab: Uuid,
        c: &ConstaterHeure,
    ) -> Result<Option<HeureDto>, EntDbError> {
        if !NATURES_HEURE.contains(&c.nature) {
            return Err(invalide("Nature d'heure inconnue."));
        }
        if !(1..=1440).contains(&c.duree_minutes) {
            return Err(invalide("La durée d'une heure constatée va de 1 minute à 24 heures."));
        }
        match (c.preuve, c.seance_id) {
            ("appel" | "attestation", Some(_)) => {}
            ("saisie", None) if NATURES_SAISIE.contains(&c.nature) && c.auteur_user_id.is_some() => {}
            ("saisie", None) => {
                return Err(invalide("Une heure de remplacement ne se saisit pas : elle se constate sur une séance assurée."))
            }
            _ => return Err(invalide("La preuve ne correspond pas à l'heure (séance assurée, ou saisie sans séance).")),
        }
        let employeur = employeur_agent(&mut *conn, etab, c.agent_id, c.le).await?;
        let id = Uuid::new_v4();
        let sql = "INSERT INTO ent_absp_heure
              (id, etablissement_id, agent_id, seance_id, impact_id, nature, duree_minutes, le, employeur, preuve, saisie_par_user_id)
            VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11)";
        let sql = if c.seance_id.is_some() {
            format!("{sql} ON CONFLICT (etablissement_id, seance_id, agent_id, nature) WHERE seance_id IS NOT NULL DO NOTHING")
        } else {
            sql.to_string()
        };
        let inseree = sqlx::query(&sql)
            .bind(id)
            .bind(etab)
            .bind(c.agent_id)
            .bind(c.seance_id)
            .bind(c.impact_id)
            .bind(c.nature)
            .bind(c.duree_minutes)
            .bind(c.le)
            .bind(employeur)
            .bind(c.preuve)
            .bind(if c.preuve == "saisie" { c.auteur_user_id } else { None })
            .execute(&mut *conn)
            .await
            .map_err(db)?
            .rows_affected();
        if inseree == 0 {
            return Ok(None);
        }
        let type_fait = if c.preuve == "attestation" { "attestation" } else { "constat" };
        fait_heure(conn, etab, id, type_fait, c.auteur_user_id, None, None, (None, Some(c.duree_minutes)), (None, Some("constatee")))
            .await?;
        let h = heure_tx(conn, etab, id, false).await?;
        evenement_rcd(
            conn,
            etab,
            EVT_HEURE_CONSTATEE,
            id,
            json!({ "heure_id": id, "agent_id": c.agent_id, "mois": h.mois, "nature": c.nature }),
            c.auteur_user_id,
        )
        .await?;
        // R-AP33 : HSE ou RCD pour remplacer une absence de convenance → alerte, jamais un blocage. L'événement ne dit
        // pas pourquoi (R-AP2) : le gestionnaire ouvre l'heure et voit la règle.
        if matches!(c.nature, "hse" | "rcd") {
            if let Some(imp) = c.impact_id {
                let convenance: bool = sqlx::query_scalar(
                    "SELECT COALESCE(n.categorie_nationale = 'convenance_personnelle', false)
                       FROM ent_absp_impact i JOIN ent_absp_absence a ON a.etablissement_id = i.etablissement_id AND a.id = i.absence_id
                       LEFT JOIN ent_absp_nature n ON n.id = a.nature_id
                      WHERE i.etablissement_id = $1 AND i.id = $2",
                )
                .bind(etab)
                .bind(imp)
                .fetch_optional(&mut *conn)
                .await
                .map_err(db)?
                .unwrap_or(false);
                if convenance {
                    evenement_rcd(
                        conn,
                        etab,
                        EVT_HEURE_A_VERIFIER,
                        id,
                        json!({ "heure_id": id, "regle": "hse_remplacement" }),
                        c.auteur_user_id,
                    )
                    .await?;
                }
            }
        }
        Ok(Some(h.into()))
    }

    /// Nature de l'heure d'un remplaçant : selon son engagement de volontariat actif au jour (R-04).
    async fn nature_du_remplacant(&self, conn: &mut PgConnection, etab: Uuid, agent: Uuid, jour: NaiveDate) -> Result<&'static str, EntDbError> {
        let statut: Option<String> = sqlx::query_scalar(
            "SELECT statut FROM ent_absp_rcd_engagement
              WHERE etablissement_id = $1 AND agent_id = $2 AND etat = 'actif' AND du <= $3 AND au >= $3",
        )
        .bind(etab)
        .bind(agent)
        .bind(jour)
        .fetch_optional(conn)
        .await
        .map_err(db)?;
        Ok(nature_remplacement(statut.as_deref()))
    }

    /// Constate l'heure d'une séance remplacée, au vu des faits : séance **terminée** (à `maintenant`), remplacement
    /// actif, et — pour la preuve `appel` — appel validé. Renvoie `None` si le fait manque ou si l'heure existe.
    async fn constater_seance_tx(
        &self,
        conn: &mut PgConnection,
        etab: Uuid,
        seance_id: Uuid,
        maintenant: DateTime<Utc>,
        attestation_de: Option<Uuid>,
    ) -> Result<Option<HeureDto>, EntDbError> {
        let Some(s) = sqlx::query_as::<_, SeanceAssuree>(SEANCE_ASSUREE_SQL)
            .bind(etab)
            .bind(seance_id)
            .fetch_optional(&mut *conn)
            .await
            .map_err(db)?
        else {
            return Ok(None);
        };
        if s.fin_instant > maintenant || s.minutes <= 0 {
            return Ok(None);
        }
        let preuve = if s.appel_fait {
            "appel"
        } else if attestation_de.is_some() {
            "attestation"
        } else {
            return Ok(None);
        };
        let nature = self.nature_du_remplacant(&mut *conn, etab, s.remplacant_agent_id, s.jour).await?;
        self.constater_heure_tx(
            conn,
            etab,
            &ConstaterHeure {
                agent_id: s.remplacant_agent_id,
                seance_id: Some(seance_id),
                impact_id: Some(s.impact_id),
                nature,
                duree_minutes: s.minutes,
                le: s.jour,
                preuve,
                auteur_user_id: if preuve == "attestation" { attestation_de } else { None },
            },
        )
        .await
    }

    /// Tâche planifiée : constate les heures des séances remplacées, terminées, dont l'appel est fait (socle 12).
    /// Idempotente : une heure déjà constatée n'est jamais recréée. `maintenant` est injectable pour les tests.
    pub(crate) async fn constater_heures_assurees_a(&self, maintenant: DateTime<Utc>) -> Result<BilanHeuresDto, EntDbError> {
        let mut bilan = BilanHeuresDto::default();
        for etab in etablissements_systeme(self.pool(), ContexteSysteme::JobPlanifie("absences_personnels_heures"))
            .await
            .map_err(db)?
        {
            let seances: Vec<Uuid> = {
                let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
                sqlx::query_scalar(
                    "SELECT DISTINCT i.seance_id
                       FROM ent_absp_impact i
                       JOIN ent_edt_seance sc ON sc.etablissement_id = i.etablissement_id AND sc.id = i.seance_id
                       JOIN ent_absp_reponse r ON r.etablissement_id = i.etablissement_id AND r.id = i.reponse_active_id
                            AND r.etat = 'active' AND r.type = 'remplacement' AND r.remplacant_agent_id IS NOT NULL
                       JOIN ent_appel ap ON ap.etablissement_id = i.etablissement_id AND ap.seance_id = sc.id AND ap.etat = 'valide'
                      WHERE i.etablissement_id = $1 AND i.type = 'seance' AND i.etat = 'remplace'
                        AND sc.jour >= ent_aujourdhui_etab($1, $2) - 90
                        AND NOT EXISTS (SELECT 1 FROM ent_absp_heure h WHERE h.etablissement_id = i.etablissement_id
                                           AND h.seance_id = i.seance_id AND h.agent_id = r.remplacant_agent_id)
                      LIMIT 1000",
                )
                .bind(etab)
                .bind(maintenant)
                .fetch_all(&mut *tx)
                .await
                .map_err(db)?
            };
            bilan.etablissements += 1;
            for s in seances {
                let r: Result<Option<HeureDto>, EntDbError> = async {
                    let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
                    let h = self.constater_seance_tx(&mut tx, etab, s, maintenant, None).await?;
                    tx.commit().await.map_err(db)?;
                    Ok(h)
                }
                .await;
                match r {
                    Ok(Some(_)) => bilan.constatees += 1,
                    Ok(None) => {}
                    Err(e) => tracing::warn!(seance = %s, "heure constatée : {e}"),
                }
            }
        }
        Ok(bilan)
    }

    pub async fn constater_heures_assurees(&self) -> Result<BilanHeuresDto, EntDbError> {
        self.constater_heures_assurees_a(Utc::now()).await
    }

    /// ABP-202 : attester qu'une séance remplacée a bien été assurée, quand l'appel n'a pas été fait (CAP-56). Le
    /// remplaçant l'atteste pour lui-même ; la vie scolaire (CPE) ou la direction pour lui. La séance doit être
    /// **terminée** et son remplacement **actif** : on n'atteste ni une séance à venir, ni une séance non remplacée,
    /// ni celle d'un autre (R-AP28). Si l'appel est fait, c'est l'appel qui prouve.
    pub async fn attester_seance_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        body: &AttesterBody,
    ) -> Result<HeureDto, EntDbError> {
        self.attester_seance_a(etab, actor, role, body, Utc::now()).await
    }

    pub(crate) async fn attester_seance_a(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        body: &AttesterBody,
        maintenant: DateTime<Utc>,
    ) -> Result<HeureDto, EntDbError> {
        let r = self.exiger_personnel_abs(role).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let Some(s) = sqlx::query_as::<_, SeanceAssuree>(SEANCE_ASSUREE_SQL)
            .bind(etab)
            .bind(body.seance_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db)?
        else {
            return Err(conflit("cette séance n'a pas de remplacement actif : rien à attester"));
        };
        let moi = agent_du_user_tx(&mut tx, etab, actor).await?;
        let est_remplacant = moi == Some(s.remplacant_agent_id);
        if !(est_remplacant || matches!(r, Role::Cpe | Role::Direction)) {
            // Pas d'oracle sur la séance d'un autre.
            return Err(introuvable());
        }
        if s.fin_instant > maintenant {
            return Err(conflit("la séance n'est pas terminée : on n'atteste pas une séance à venir"));
        }
        let h = self.constater_seance_tx(&mut tx, etab, body.seance_id, maintenant, Some(actor)).await?;
        let h = match h {
            Some(h) => h,
            None => {
                // Déjà constatée (idempotence) : on renvoie l'heure existante.
                let existante: HeureLigne = sqlx::query_as::<_, HeureLigne>(&format!(
                    "SELECT {HEURE_COLONNES} FROM ent_absp_heure h WHERE h.etablissement_id = $1 AND h.seance_id = $2 AND h.agent_id = $3
                      ORDER BY h.cree_le LIMIT 1"
                ))
                .bind(etab)
                .bind(body.seance_id)
                .bind(s.remplacant_agent_id)
                .fetch_optional(&mut *tx)
                .await
                .map_err(db)?
                .ok_or_else(|| conflit("la séance ne peut pas être constatée"))?;
                existante.into()
            }
        };
        tx.commit().await.map_err(db)?;
        Ok(h)
    }

    // ───────────────────────────── Lectures (ABP-200, 214, 219) ─────────────────────────────

    /// ABP-200 et CAP-56 : mes heures (par mois), et **ce qui manque** : mes séances remplacées, terminées, sans heure
    /// (appel non fait) — à attester.
    pub async fn mes_heures_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        mois: Option<NaiveDate>,
    ) -> Result<MesHeuresDto, EntDbError> {
        self.mes_heures_a(etab, actor, role, mois, Utc::now()).await
    }

    pub(crate) async fn mes_heures_a(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        mois: Option<NaiveDate>,
        maintenant: DateTime<Utc>,
    ) -> Result<MesHeuresDto, EntDbError> {
        self.exiger_personnel_abs(role).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let Some(moi) = agent_du_user_tx(&mut tx, etab, actor).await? else {
            return Ok(MesHeuresDto { heures: vec![], a_attester: vec![] });
        };
        let heures: Vec<HeureLigne> = sqlx::query_as(&format!(
            "SELECT {HEURE_COLONNES} FROM ent_absp_heure h
              WHERE h.etablissement_id = $1 AND h.agent_id = $2 AND ($3::date IS NULL OR h.mois = $3)
              ORDER BY h.le DESC, h.cree_le DESC LIMIT 500"
        ))
        .bind(etab)
        .bind(moi)
        .bind(mois.map(premier_du_mois))
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let manque: Vec<(Uuid, NaiveDate, NaiveTime, NaiveTime, String)> = sqlx::query_as(
            "SELECT sc.id, sc.jour, sc.heure_debut, sc.heure_fin, sc.matiere_libelle
               FROM ent_absp_impact i
               JOIN ent_edt_seance sc ON sc.etablissement_id = i.etablissement_id AND sc.id = i.seance_id
               JOIN ent_absp_reponse r ON r.etablissement_id = i.etablissement_id AND r.id = i.reponse_active_id
                    AND r.etat = 'active' AND r.type = 'remplacement' AND r.remplacant_agent_id = $2
              WHERE i.etablissement_id = $1 AND i.type = 'seance' AND i.etat = 'remplace'
                AND ((sc.jour + sc.heure_fin)::timestamp AT TIME ZONE COALESCE(
                       (SELECT fuseau FROM ent_etablissement_parametre WHERE etablissement_id = $1), 'Europe/Paris')) <= $3
                AND NOT EXISTS (SELECT 1 FROM ent_absp_heure h WHERE h.etablissement_id = i.etablissement_id
                                   AND h.seance_id = i.seance_id AND h.agent_id = $2)
                AND NOT EXISTS (SELECT 1 FROM ent_appel ap WHERE ap.etablissement_id = i.etablissement_id
                                   AND ap.seance_id = sc.id AND ap.etat = 'valide')
              ORDER BY sc.jour DESC, sc.heure_debut LIMIT 200",
        )
        .bind(etab)
        .bind(moi)
        .bind(maintenant)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        Ok(MesHeuresDto {
            heures: heures.into_iter().map(Into::into).collect(),
            a_attester: manque
                .into_iter()
                .map(|(seance_id, le, d, f, m)| SeanceAAttesterDto {
                    seance_id,
                    le,
                    heure_debut: d,
                    heure_fin: f,
                    matiere: m,
                })
                .collect(),
        })
    }

    /// ABP-214 : les heures d'un agent (relevé de service fait d'un vacataire, heures d'un volontaire…), dans le
    /// périmètre d'employeur du lecteur.
    pub async fn heures_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        filtre: &FiltreHeures,
    ) -> Result<Vec<HeureDto>, EntDbError> {
        let r = self.exiger_personnel_abs(role).await?;
        if !self.a_droit_abs(etab, actor, role, Action::PersonnelsHeuresLire).await? {
            return Err(EntDbError::Forbidden);
        }
        let permis: Vec<String> = employeurs_permis(r).iter().map(|s| (*s).to_string()).collect();
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let v: Vec<HeureLigne> = sqlx::query_as(&format!(
            "SELECT {HEURE_COLONNES} FROM ent_absp_heure h
              WHERE h.etablissement_id = $1 AND h.employeur = ANY($2)
                AND ($3::uuid IS NULL OR h.agent_id = $3) AND ($4::date IS NULL OR h.mois = $4)
                AND ($5::text IS NULL OR h.nature = $5) AND ($6::text IS NULL OR h.etat = $6)
              ORDER BY h.le DESC, h.cree_le DESC LIMIT 1000"
        ))
        .bind(etab)
        .bind(&permis)
        .bind(filtre.agent_id)
        .bind(filtre.mois.map(premier_du_mois))
        .bind(&filtre.nature)
        .bind(&filtre.etat)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        Ok(v.into_iter().map(Into::into).collect())
    }

    /// ABP-219 : l'état annuel des heures d'un agent (par mois et par nature, minutes **effectives** :
    /// régularisations comprises). L'agent pour lui-même ; sinon `heures.lire` (périmètre d'employeur).
    pub async fn etat_annuel_agent_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        agent_id: Option<Uuid>,
        annee_id: Option<Uuid>,
    ) -> Result<EtatAnnuelDto, EntDbError> {
        let r = self.exiger_personnel_abs(role).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let moi = agent_du_user_tx(&mut tx, etab, actor).await?;
        let agent = agent_id.or(moi).ok_or(EntDbError::Forbidden)?;
        let permis: Vec<String> = if Some(agent) == moi {
            vec!["etat".into(), "ogec".into()]
        } else {
            if !self.a_droit_abs(etab, actor, role, Action::PersonnelsHeuresLire).await? {
                return Err(EntDbError::Forbidden);
            }
            employeurs_permis(r).iter().map(|s| (*s).to_string()).collect()
        };
        let (annee, debut, fin): (Uuid, NaiveDate, NaiveDate) = match annee_id {
            Some(a) => sqlx::query_as("SELECT id, date_debut, date_fin FROM ent_annee_scolaire WHERE etablissement_id = $1 AND id = $2")
                .bind(etab)
                .bind(a)
                .fetch_optional(&mut *tx)
                .await
                .map_err(db)?
                .ok_or_else(introuvable)?,
            None => sqlx::query_as("SELECT id, date_debut, date_fin FROM ent_annee_scolaire WHERE etablissement_id = $1 AND courante")
                .bind(etab)
                .fetch_optional(&mut *tx)
                .await
                .map_err(db)?
                .ok_or_else(|| conflit("aucune année scolaire courante"))?,
        };
        let lignes: Vec<(NaiveDate, String, i64)> = sqlx::query_as(
            "SELECT h.mois, h.nature,
                    sum(h.duree_minutes + COALESCE((SELECT sum(f.duree_apres - f.duree_avant) FROM ent_absp_heure_fait f
                         WHERE f.etablissement_id = h.etablissement_id AND f.heure_id = h.id AND f.type = 'regularisation'), 0))::int8
               FROM ent_absp_heure h
              WHERE h.etablissement_id = $1 AND h.agent_id = $2 AND h.employeur = ANY($3) AND h.le BETWEEN $4 AND $5
              GROUP BY h.mois, h.nature ORDER BY h.mois, h.nature",
        )
        .bind(etab)
        .bind(agent)
        .bind(&permis)
        .bind(debut)
        .bind(fin)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        Ok(EtatAnnuelDto {
            agent_id: agent,
            annee_id: annee,
            total_minutes: lignes.iter().map(|l| l.2).sum(),
            par_mois: lignes.into_iter().map(|(mois, nature, minutes)| LigneAnnuelleDto { mois, nature, minutes }).collect(),
        })
    }

    // ───────────────────────────── Cycle d'une heure (ABP-201, 203) ─────────────────────────────

    fn verifier_raison(raison: &str, note: &Option<String>) -> Result<Option<String>, EntDbError> {
        if !RAISONS.contains(&raison) {
            return Err(invalide("Motif inconnu."));
        }
        texte_borne(note, 500, "La note est limitée à 500 caractères.")
    }

    /// ABP-201 : contester une heure (motif codé). L'agent conteste la sienne ; `heures.valider` toute heure de son
    /// périmètre. Une heure figée ne se conteste plus : elle se régularise (ABP-208).
    pub async fn contester_heure_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        heure_id: Uuid,
        body: &ContesterHeureBody,
    ) -> Result<HeureDto, EntDbError> {
        let r = self.exiger_personnel_abs(role).await?;
        let note = Self::verifier_raison(&body.raison_code, &body.note)?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let h = heure_tx(&mut tx, etab, heure_id, true).await?;
        let moi = agent_du_user_tx(&mut tx, etab, actor).await?;
        let proprietaire = moi == Some(h.agent_id);
        let gestionnaire =
            self.a_droit_abs(etab, actor, role, Action::PersonnelsHeuresValider).await? && employeurs_permis(r).contains(&h.employeur.as_str());
        if !(proprietaire || gestionnaire) {
            return Err(introuvable());
        }
        if !matches!(h.etat.as_str(), "constatee" | "validee") {
            return Err(conflit("cette heure ne peut plus être contestée dans son état actuel (figée : régularisation)"));
        }
        sqlx::query("UPDATE ent_absp_heure SET etat = 'contestee', modifie_le = now() WHERE etablissement_id = $1 AND id = $2")
            .bind(etab)
            .bind(heure_id)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        fait_heure(
            &mut tx,
            etab,
            heure_id,
            "contestation",
            Some(actor),
            Some(&body.raison_code),
            note.as_deref(),
            (None, None),
            (Some(&h.etat), Some("contestee")),
        )
        .await?;
        let d = heure_tx(&mut tx, etab, heure_id, false).await?;
        tx.commit().await.map_err(db)?;
        Ok(d.into())
    }

    /// Corrige la durée d'une heure non figée (0 = annulée) ; le gestionnaire ne corrige pas ses propres heures. Une
    /// correction règle la contestation : l'heure est alors validée.
    pub async fn corriger_heure_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        heure_id: Uuid,
        body: &CorrigerHeureBody,
    ) -> Result<HeureDto, EntDbError> {
        let r = self.exiger_heures_valider(etab, actor, role).await?;
        let note = Self::verifier_raison(&body.raison_code, &body.note)?;
        if !(0..=1440).contains(&body.duree_minutes) {
            return Err(invalide("La durée va de 0 à 24 heures."));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let h = heure_tx(&mut tx, etab, heure_id, true).await?;
        if !employeurs_permis(r).contains(&h.employeur.as_str()) {
            return Err(introuvable());
        }
        if agent_du_user_tx(&mut tx, etab, actor).await? == Some(h.agent_id) {
            return Err(EntDbError::Refus(crate::service::RefusMetier::Interdit, "Personne ne corrige ses propres heures."));
        }
        if !matches!(h.etat.as_str(), "constatee" | "contestee" | "validee") {
            return Err(conflit("cette heure est figée : régularisez-la sur l'état suivant"));
        }
        if body.duree_minutes == h.duree_minutes && h.etat == "validee" {
            return Err(conflit("aucun changement"));
        }
        sqlx::query("UPDATE ent_absp_heure SET duree_minutes = $3, etat = 'validee', modifie_le = now() WHERE etablissement_id = $1 AND id = $2")
            .bind(etab)
            .bind(heure_id)
            .bind(body.duree_minutes)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        fait_heure(
            &mut tx,
            etab,
            heure_id,
            "correction",
            Some(actor),
            Some(&body.raison_code),
            note.as_deref(),
            (Some(h.duree_minutes), Some(body.duree_minutes)),
            (Some(&h.etat), Some("validee")),
        )
        .await?;
        let d = heure_tx(&mut tx, etab, heure_id, false).await?;
        tx.commit().await.map_err(db)?;
        Ok(d.into())
    }

    /// Saisie d'une heure **sans séance** (mission Pacte, HSE hors séance, vacation, service non fait) par la
    /// direction : preuve « saisie ». Jamais une heure de remplacement (R-AP28), jamais dans le futur, jamais pour
    /// soi-même.
    pub async fn saisir_heure_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        body: &SaisirHeureBody,
    ) -> Result<HeureDto, EntDbError> {
        self.exiger_heures_valider(etab, actor, role).await?;
        let nature = *NATURES_HEURE
            .iter()
            .find(|n| **n == body.nature)
            .ok_or_else(|| invalide("Nature d'heure inconnue."))?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        if body.le > jour_etab(&mut tx, etab).await? {
            return Err(invalide("Une heure ne se constate pas à l'avance."));
        }
        if agent_du_user_tx(&mut tx, etab, actor).await? == Some(body.agent_id) {
            return Err(EntDbError::Refus(crate::service::RefusMetier::Interdit, "Personne ne saisit ses propres heures."));
        }
        let existe: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM ent_agent WHERE etablissement_id = $1 AND id = $2)")
            .bind(etab)
            .bind(body.agent_id)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
        if !existe {
            return Err(introuvable());
        }
        let h = self
            .constater_heure_tx(
                &mut tx,
                etab,
                &ConstaterHeure {
                    agent_id: body.agent_id,
                    seance_id: None,
                    impact_id: None,
                    nature,
                    duree_minutes: body.duree_minutes,
                    le: body.le,
                    preuve: "saisie",
                    auteur_user_id: Some(actor),
                },
            )
            .await?
            .ok_or_else(|| conflit("heure déjà constatée"))?;
        tx.commit().await.map_err(db)?;
        Ok(h)
    }

    /// ABP-203 : valider les heures `constatee` d'un mois (ou d'un agent). Les heures contestées restent à régler ;
    /// celles du valideur lui-même sont laissées à un autre.
    pub async fn valider_heures_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        body: &ValiderHeuresBody,
    ) -> Result<BilanValidationDto, EntDbError> {
        let r = self.exiger_heures_valider(etab, actor, role).await?;
        let mois = premier_du_mois(body.mois);
        let permis: Vec<String> = employeurs_permis(r).iter().map(|s| (*s).to_string()).collect();
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let moi = agent_du_user_tx(&mut tx, etab, actor).await?;
        let ids: Vec<(Uuid, Uuid)> = sqlx::query_as(
            "SELECT id, agent_id FROM ent_absp_heure
              WHERE etablissement_id = $1 AND mois = $2 AND employeur = ANY($3) AND etat = 'constatee'
                AND ($4::uuid IS NULL OR agent_id = $4)
              ORDER BY le, cree_le FOR UPDATE",
        )
        .bind(etab)
        .bind(mois)
        .bind(&permis)
        .bind(body.agent_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let mut bilan = BilanValidationDto::default();
        for (id, agent) in ids {
            if moi == Some(agent) {
                bilan.laissees_a_un_autre += 1;
                continue;
            }
            sqlx::query("UPDATE ent_absp_heure SET etat = 'validee', modifie_le = now() WHERE etablissement_id = $1 AND id = $2")
                .bind(etab)
                .bind(id)
                .execute(&mut *tx)
                .await
                .map_err(db)?;
            fait_heure(&mut tx, etab, id, "validation", Some(actor), None, None, (None, None), (Some("constatee"), Some("validee")))
                .await?;
            bilan.validees += 1;
        }
        bilan.contestees_restantes = sqlx::query_scalar(
            "SELECT count(*) FROM ent_absp_heure WHERE etablissement_id = $1 AND mois = $2 AND employeur = ANY($3) AND etat = 'contestee'",
        )
        .bind(etab)
        .bind(mois)
        .bind(&permis)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        if bilan.validees > 0 {
            evenement_rcd(
                &mut tx,
                etab,
                EVT_HEURES_VALIDEES,
                Uuid::new_v4(),
                json!({ "mois": mois, "nombre": bilan.validees }),
                Some(actor),
            )
            .await?;
        }
        tx.commit().await.map_err(db)?;
        Ok(bilan)
    }

    // ───────────────────────────── États mensuels (ABP-204 à 206, 208) ─────────────────────────────

    async fn etat_dto(&self, conn: &mut PgConnection, etab: Uuid, id: Uuid) -> Result<EtatMensuelDto, EntDbError> {
        let (mois, employeur, etat, fige_le, signe_le, exporte_le): (
            NaiveDate,
            String,
            String,
            DateTime<Utc>,
            Option<DateTime<Utc>>,
            Option<DateTime<Utc>>,
        ) = sqlx::query_as(
            "SELECT mois, employeur, etat, fige_le, signe_le, exporte_le FROM ent_absp_etat WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(id)
        .fetch_optional(&mut *conn)
        .await
        .map_err(db)?
        .ok_or_else(introuvable)?;
        let lignes: Vec<(Uuid, Uuid, String, i32, Option<Uuid>, Option<Uuid>)> = sqlx::query_as(
            "SELECT id, agent_id, nature, duree_minutes, heure_id, regularisation_de_id FROM ent_absp_etat_ligne
              WHERE etablissement_id = $1 AND etat_id = $2 ORDER BY agent_id, nature, cree_le, id",
        )
        .bind(etab)
        .bind(id)
        .fetch_all(conn)
        .await
        .map_err(db)?;
        let total = lignes.iter().map(|l| i64::from(l.3)).sum();
        Ok(EtatMensuelDto {
            id,
            mois,
            employeur,
            etat,
            fige_le,
            signe_le,
            exporte_le,
            total_minutes: total,
            lignes: lignes
                .into_iter()
                .map(|(id, agent_id, nature, duree_minutes, heure_id, regularisation_de_id)| EtatLigneDto {
                    id,
                    agent_id,
                    nature,
                    duree_minutes,
                    heure_id,
                    regularisation_de_id,
                })
                .collect(),
        })
    }

    /// ABP-204 : figer l'état d'un mois et d'un employeur. Le mois doit être **terminé** ; il ne reste ni heure à
    /// valider ni heure contestée ; le mois n'a pas déjà son état. L'état emporte les heures validées (elles
    /// deviennent `figee`) et les régularisations en attente de l'employeur (lignes signées, rattachées à la ligne
    /// d'origine). Le valideur ou le gestionnaire de l'employeur.
    pub async fn figer_etat_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        body: &FigerEtatBody,
    ) -> Result<EtatMensuelDto, EntDbError> {
        let r = self.exiger_personnel_abs(role).await?;
        if !["etat", "ogec"].contains(&body.employeur.as_str()) {
            return Err(invalide("Employeur inconnu : l'État ou l'OGEC."));
        }
        let peut = employeurs_permis(r).contains(&body.employeur.as_str())
            && (self.a_droit_abs(etab, actor, role, Action::PersonnelsHeuresValider).await?
                || self.a_droit_abs(etab, actor, role, Action::PersonnelsHeuresExporter).await?);
        if !peut {
            return Err(EntDbError::Forbidden);
        }
        let mois = premier_du_mois(body.mois);
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        if jour_etab(&mut tx, etab).await? <= dernier_du_mois(mois) {
            return Err(conflit("le mois n'est pas terminé : l'état ne se fige qu'après son dernier jour"));
        }
        let (a_valider, contestees): (i64, i64) = sqlx::query_as(
            "SELECT count(*) FILTER (WHERE etat = 'constatee'), count(*) FILTER (WHERE etat = 'contestee')
               FROM ent_absp_heure WHERE etablissement_id = $1 AND mois = $2 AND employeur = $3",
        )
        .bind(etab)
        .bind(mois)
        .bind(&body.employeur)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        if a_valider + contestees > 0 {
            return Err(conflit("des heures du mois restent à valider ou contestées : réglez-les avant de figer l'état"));
        }
        let id = Uuid::new_v4();
        let res = sqlx::query(
            "INSERT INTO ent_absp_etat (id, etablissement_id, mois, employeur, fige_par_user_id) VALUES ($1,$2,$3,$4,$5)",
        )
        .bind(id)
        .bind(etab)
        .bind(mois)
        .bind(&body.employeur)
        .bind(actor)
        .execute(&mut *tx)
        .await;
        if let Err(e) = res {
            return Err(if matches!(&e, sqlx::Error::Database(d) if d.code().as_deref() == Some("23505")) {
                conflit("l'état de ce mois est déjà figé pour cet employeur")
            } else {
                db(e)
            });
        }
        // Heures du mois : lignes d'origine, puis figées.
        let heures: Vec<(Uuid, Uuid, String, i32)> = sqlx::query_as(
            "SELECT id, agent_id, nature, duree_minutes FROM ent_absp_heure
              WHERE etablissement_id = $1 AND mois = $2 AND employeur = $3 AND etat = 'validee' ORDER BY le, cree_le FOR UPDATE",
        )
        .bind(etab)
        .bind(mois)
        .bind(&body.employeur)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        for (hid, agent, nature, duree) in heures {
            if duree > 0 {
                sqlx::query(
                    "INSERT INTO ent_absp_etat_ligne (id, etablissement_id, etat_id, agent_id, nature, duree_minutes, heure_id)
                     VALUES ($1,$2,$3,$4,$5,$6,$7)",
                )
                .bind(Uuid::new_v4())
                .bind(etab)
                .bind(id)
                .bind(agent)
                .bind(&nature)
                .bind(duree)
                .bind(hid)
                .execute(&mut *tx)
                .await
                .map_err(db)?;
            }
            sqlx::query("UPDATE ent_absp_heure SET etat = 'figee', etat_id = $3, modifie_le = now() WHERE etablissement_id = $1 AND id = $2")
                .bind(etab)
                .bind(hid)
                .bind(id)
                .execute(&mut *tx)
                .await
                .map_err(db)?;
        }
        // Régularisations en attente (faits jamais reportés) de ce même employeur : lignes signées.
        let regul: Vec<(Uuid, Uuid, Uuid, String, i32, Uuid)> = sqlx::query_as(
            "SELECT f.id, h.id, h.agent_id, h.nature, (f.duree_apres - f.duree_avant)::int, l.id
               FROM ent_absp_heure_fait f
               JOIN ent_absp_heure h ON h.etablissement_id = f.etablissement_id AND h.id = f.heure_id
               JOIN ent_absp_etat_ligne l ON l.etablissement_id = h.etablissement_id AND l.heure_id = h.id
                    AND l.regularisation_de_id IS NULL
              WHERE f.etablissement_id = $1 AND f.type = 'regularisation' AND h.employeur = $2
                AND f.duree_apres <> f.duree_avant
                AND NOT EXISTS (SELECT 1 FROM ent_absp_etat_ligne x WHERE x.etablissement_id = f.etablissement_id AND x.heure_fait_id = f.id)
              ORDER BY f.le, f.id",
        )
        .bind(etab)
        .bind(&body.employeur)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        for (fait, hid, agent, nature, delta, origine) in regul {
            sqlx::query(
                "INSERT INTO ent_absp_etat_ligne
                   (id, etablissement_id, etat_id, agent_id, nature, duree_minutes, heure_id, regularisation_de_id, heure_fait_id)
                 VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)",
            )
            .bind(Uuid::new_v4())
            .bind(etab)
            .bind(id)
            .bind(agent)
            .bind(&nature)
            .bind(delta)
            .bind(hid)
            .bind(origine)
            .bind(fait)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        }
        journal_objet(&mut tx, etab, "etat", id, "etat_fige", Some("mois"), None, Some(&mois.to_string()), actor).await?;
        let d = self.etat_dto(&mut tx, etab, id).await?;
        tx.commit().await.map_err(db)?;
        Ok(d)
    }

    /// ABP-205 ★ : le chef d'établissement signe l'état. Seulement un état figé.
    pub async fn signer_etat_for_actor(&self, etab: Uuid, actor: Uuid, role: &str, etat_id: Uuid) -> Result<EtatMensuelDto, EntDbError> {
        let r = self.exiger_personnel_abs(role).await?;
        if !self.a_droit_abs(etab, actor, role, Action::PersonnelsHeuresSigner).await? {
            return Err(EntDbError::Forbidden);
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let (etat, employeur): (String, String) =
            sqlx::query_as("SELECT etat, employeur FROM ent_absp_etat WHERE etablissement_id = $1 AND id = $2 FOR UPDATE")
                .bind(etab)
                .bind(etat_id)
                .fetch_optional(&mut *tx)
                .await
                .map_err(db)?
                .ok_or_else(introuvable)?;
        if !employeurs_permis(r).contains(&employeur.as_str()) {
            return Err(introuvable());
        }
        if etat != "fige" {
            return Err(conflit("seul un état figé se signe"));
        }
        sqlx::query("UPDATE ent_absp_etat SET etat = 'signe', signe_par_user_id = $3, signe_le = now() WHERE etablissement_id = $1 AND id = $2")
            .bind(etab)
            .bind(etat_id)
            .bind(actor)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        journal_objet(&mut tx, etab, "etat", etat_id, "etat_signe", Some("etat"), Some("fige"), Some("signe"), actor).await?;
        evenement_rcd(&mut tx, etab, EVT_ETAT_SIGNE, etat_id, json!({ "etat_id": etat_id }), Some(actor)).await?;
        let d = self.etat_dto(&mut tx, etab, etat_id).await?;
        tx.commit().await.map_err(db)?;
        Ok(d)
    }

    /// Un état mensuel et ses lignes (lecture : `heures.lire` ou `heures.exporter`, périmètre d'employeur).
    pub async fn etat_mensuel_for_actor(&self, etab: Uuid, actor: Uuid, role: &str, etat_id: Uuid) -> Result<EtatMensuelDto, EntDbError> {
        let r = self.exiger_personnel_abs(role).await?;
        if !(self.a_droit_abs(etab, actor, role, Action::PersonnelsHeuresLire).await?
            || self.a_droit_abs(etab, actor, role, Action::PersonnelsHeuresExporter).await?)
        {
            return Err(EntDbError::Forbidden);
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let d = self.etat_dto(&mut tx, etab, etat_id).await?;
        if !employeurs_permis(r).contains(&d.employeur.as_str()) {
            return Err(introuvable());
        }
        Ok(d)
    }

    /// ABP-206 : exporter l'état signé (tableau CSV, format académique à obtenir, Q-AP5). Le premier export passe
    /// l'état et ses heures à « exporté » (ils ne bougent plus) ; chaque export est tracé au journal. Un état non
    /// signé ne s'exporte pas.
    pub async fn exporter_etat_csv_for_actor(&self, etab: Uuid, actor: Uuid, role: &str, etat_id: Uuid) -> Result<String, EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let (etat, employeur): (String, String) =
            sqlx::query_as("SELECT etat, employeur FROM ent_absp_etat WHERE etablissement_id = $1 AND id = $2 FOR UPDATE")
                .bind(etab)
                .bind(etat_id)
                .fetch_optional(&mut *tx)
                .await
                .map_err(db)?
                .ok_or_else(introuvable)?;
        self.exiger_employeur(etab, actor, role, Action::PersonnelsHeuresExporter, &employeur).await?;
        if etat == "fige" {
            return Err(conflit("l'état doit être signé par le chef d'établissement avant d'être exporté"));
        }
        if etat == "signe" {
            sqlx::query(
                "UPDATE ent_absp_etat SET etat = 'exporte', exporte_par_user_id = $3, exporte_le = now(), export_trace_id = $4
                  WHERE etablissement_id = $1 AND id = $2",
            )
            .bind(etab)
            .bind(etat_id)
            .bind(actor)
            .bind(Uuid::new_v4())
            .execute(&mut *tx)
            .await
            .map_err(db)?;
            sqlx::query("UPDATE ent_absp_heure SET etat = 'exportee', modifie_le = now() WHERE etablissement_id = $1 AND etat_id = $2 AND etat = 'figee'")
                .bind(etab)
                .bind(etat_id)
                .execute(&mut *tx)
                .await
                .map_err(db)?;
            evenement_rcd(&mut tx, etab, EVT_ETAT_EXPORTE, etat_id, json!({ "etat_id": etat_id }), Some(actor)).await?;
        }
        journal_objet(&mut tx, etab, "etat", etat_id, "etat_exporte", None, None, None, actor).await?;
        let d = self.etat_dto(&mut tx, etab, etat_id).await?;
        let noms = self.noms_agents(&mut tx, etab, &d.lignes.iter().map(|l| l.agent_id).collect::<Vec<_>>()).await?;
        tx.commit().await.map_err(db)?;
        Ok(csv_etat(&d, &noms))
    }

    async fn noms_agents(
        &self,
        conn: &mut PgConnection,
        etab: Uuid,
        agents: &[Uuid],
    ) -> Result<std::collections::HashMap<Uuid, String>, EntDbError> {
        let v: Vec<(Uuid, String)> = sqlx::query_as(
            "SELECT a.id, btrim(coalesce(p.prenom, '') || ' ' || p.nom) FROM ent_agent a
               JOIN ent_personne p ON p.etablissement_id = a.etablissement_id AND p.id = a.personne_id
              WHERE a.etablissement_id = $1 AND a.id = ANY($2)",
        )
        .bind(etab)
        .bind(agents)
        .fetch_all(conn)
        .await
        .map_err(db)?;
        Ok(v.into_iter().collect())
    }

    /// ABP-208 : régulariser une heure **figée ou exportée** (jamais une réécriture). Écrit un fait en ajout seul ;
    /// à la prochaine figeure de l'employeur, une ligne signée (nouvelle durée − durée effective) est reportée sur
    /// l'état suivant, rattachée à la ligne d'origine. L'heure garde sa durée exportée.
    pub async fn regulariser_heure_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        heure_id: Uuid,
        body: &CorrigerHeureBody,
    ) -> Result<RegularisationDto, EntDbError> {
        let r = self.exiger_personnel_abs(role).await?;
        let note = Self::verifier_raison(&body.raison_code, &body.note)?;
        if !(0..=1440).contains(&body.duree_minutes) {
            return Err(invalide("La durée va de 0 à 24 heures."));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let h = heure_tx(&mut tx, etab, heure_id, true).await?;
        let autorise = employeurs_permis(r).contains(&h.employeur.as_str())
            && (self.a_droit_abs(etab, actor, role, Action::PersonnelsHeuresValider).await?
                || self.a_droit_abs(etab, actor, role, Action::PersonnelsHeuresExporter).await?);
        if !autorise {
            return Err(introuvable());
        }
        if agent_du_user_tx(&mut tx, etab, actor).await? == Some(h.agent_id) {
            return Err(EntDbError::Refus(crate::service::RefusMetier::Interdit, "Personne ne régularise ses propres heures."));
        }
        if !matches!(h.etat.as_str(), "figee" | "exportee") {
            return Err(conflit("seule une heure figée ou exportée se régularise : les autres se corrigent"));
        }
        let effective = duree_effective(h.duree_minutes, h.regul_minutes);
        if i64::from(body.duree_minutes) == effective {
            return Err(conflit("la durée demandée est déjà la durée effective"));
        }
        let fait = fait_heure(
            &mut tx,
            etab,
            heure_id,
            "regularisation",
            Some(actor),
            Some(&body.raison_code),
            note.as_deref(),
            (Some(effective as i32), Some(body.duree_minutes)),
            (Some(&h.etat), Some(&h.etat)),
        )
        .await?;
        journal_objet(
            &mut tx,
            etab,
            "heure",
            heure_id,
            "heure_regularisee",
            Some("duree_minutes"),
            Some(&effective.to_string()),
            Some(&body.duree_minutes.to_string()),
            actor,
        )
        .await?;
        tx.commit().await.map_err(db)?;
        Ok(RegularisationDto {
            heure_id,
            fait_id: fait,
            duree_avant: effective as i32,
            duree_apres: body.duree_minutes,
            delta_minutes: body.duree_minutes - effective as i32,
        })
    }

    // ───────────────────────────── Paie de l'OGEC (ABP-207, 209) ─────────────────────────────

    async fn exiger_paie(&self, etab: Uuid, actor: Uuid, role: &str) -> Result<(), EntDbError> {
        self.exiger_employeur(etab, actor, role, Action::PersonnelsPaieAbsences, "ogec").await
    }

    /// Pose un élément de paie, **un seul** par (source, mois, nature) : l'unicité est tenue par la base, la pose est
    /// rejouable. Existant préparé : mis à jour ; déjà envoyé : une **régularisation** sur `mois_ouvert` si la
    /// quantité cible diffère (jamais une réécriture). Renvoie (créé, régularisé).
    #[allow(clippy::too_many_arguments)]
    async fn poser_element(
        &self,
        conn: &mut PgConnection,
        etab: Uuid,
        agent: Uuid,
        absence: Option<Uuid>,
        heure: Option<Uuid>,
        mois: NaiveDate,
        mois_ouvert: NaiveDate,
        nature: &str,
        cible_centi: i64,
        unite: &str,
    ) -> Result<(u32, u32), EntDbError> {
        let source = absence.or(heure);
        let existant: Option<(Uuid, i64, String)> = sqlx::query_as(
            "SELECT id, (quantite * 100)::int8, etat FROM ent_absp_element_paie
              WHERE etablissement_id = $1 AND COALESCE(absence_id, heure_id) = $2 AND mois = $3 AND nature_code = $4 FOR UPDATE",
        )
        .bind(etab)
        .bind(source)
        .bind(mois)
        .bind(nature)
        .fetch_optional(&mut *conn)
        .await
        .map_err(db)?;
        match existant {
            None => {
                if cible_centi == 0 {
                    return Ok((0, 0));
                }
                let n = sqlx::query(
                    "INSERT INTO ent_absp_element_paie (id, etablissement_id, agent_id, absence_id, heure_id, mois, nature_code, quantite, unite)
                     VALUES ($1,$2,$3,$4,$5,$6,$7,$8::numeric / 100,$9)
                     ON CONFLICT (etablissement_id, COALESCE(absence_id, heure_id), mois, nature_code) DO NOTHING",
                )
                .bind(Uuid::new_v4())
                .bind(etab)
                .bind(agent)
                .bind(absence)
                .bind(heure)
                .bind(mois)
                .bind(nature)
                .bind(cible_centi)
                .bind(unite)
                .execute(&mut *conn)
                .await
                .map_err(db)?
                .rows_affected();
                Ok((n as u32, 0))
            }
            Some((id, centi, etat)) if etat == "prepare" => {
                if centi != cible_centi {
                    if cible_centi == 0 {
                        sqlx::query("DELETE FROM ent_absp_element_paie WHERE etablissement_id = $1 AND id = $2")
                            .bind(etab)
                            .bind(id)
                            .execute(&mut *conn)
                            .await
                            .map_err(db)?;
                    } else {
                        sqlx::query("UPDATE ent_absp_element_paie SET quantite = $3::numeric / 100 WHERE etablissement_id = $1 AND id = $2")
                            .bind(etab)
                            .bind(id)
                            .bind(cible_centi)
                            .execute(&mut *conn)
                            .await
                            .map_err(db)?;
                    }
                }
                Ok((0, 0))
            }
            Some((id, centi, _)) => {
                // Déjà envoyé : régularisation sur le mois ouvert = cible − envoyé − régularisations déjà posées.
                let deja: i64 = sqlx::query_scalar(
                    "SELECT COALESCE(sum(quantite * 100), 0)::int8 FROM ent_absp_element_paie
                      WHERE etablissement_id = $1 AND regularise_de_id = $2",
                )
                .bind(etab)
                .bind(id)
                .fetch_one(&mut *conn)
                .await
                .map_err(db)?;
                let delta = cible_centi - centi - deja;
                if delta == 0 {
                    return Ok((0, 0));
                }
                let code = format!("regul_{nature}");
                let code: String = code.chars().take(40).collect();
                let existe_regul: Option<(Uuid, String)> = sqlx::query_as(
                    "SELECT id, etat FROM ent_absp_element_paie
                      WHERE etablissement_id = $1 AND COALESCE(absence_id, heure_id) = $2 AND mois = $3 AND nature_code = $4 FOR UPDATE",
                )
                .bind(etab)
                .bind(source)
                .bind(mois_ouvert)
                .bind(&code)
                .fetch_optional(&mut *conn)
                .await
                .map_err(db)?;
                match existe_regul {
                    Some((rid, etat)) if etat == "prepare" => {
                        sqlx::query("UPDATE ent_absp_element_paie SET quantite = quantite + $3::numeric / 100 WHERE etablissement_id = $1 AND id = $2")
                            .bind(etab)
                            .bind(rid)
                            .bind(delta)
                            .execute(&mut *conn)
                            .await
                            .map_err(db)?;
                    }
                    Some(_) => return Ok((0, 0)), // déjà envoyée sur le mois ouvert : à reprendre au mois suivant
                    None => {
                        sqlx::query(
                            "INSERT INTO ent_absp_element_paie
                               (id, etablissement_id, agent_id, absence_id, heure_id, mois, nature_code, quantite, unite, regularise_de_id)
                             VALUES ($1,$2,$3,$4,$5,$6,$7,$8::numeric / 100,$9,$10)",
                        )
                        .bind(Uuid::new_v4())
                        .bind(etab)
                        .bind(agent)
                        .bind(absence)
                        .bind(heure)
                        .bind(mois_ouvert)
                        .bind(&code)
                        .bind(delta)
                        .bind(unite)
                        .bind(id)
                        .execute(&mut *conn)
                        .await
                        .map_err(db)?;
                    }
                }
                sqlx::query("UPDATE ent_absp_element_paie SET etat = 'regularise' WHERE etablissement_id = $1 AND id = $2 AND etat = 'envoye'")
                    .bind(etab)
                    .bind(id)
                    .execute(&mut *conn)
                    .await
                    .map_err(db)?;
                Ok((0, 1))
            }
        }
    }

    /// ABP-207 et ABP-209 : prépare les éléments variables de paie d'un mois pour les salariés de l'OGEC. Deux
    /// sources, toutes deux **calculées depuis des faits** : les heures validées, figées ou exportées (en heures), et les
    /// absences décidées (en jours ouvrés du calendrier de l'établissement ; demi-journée si l'absence tient dans
    /// une demi-journée). Rejouable : rien n'est créé deux fois (unicité en base). Les régularisations des mois déjà
    /// envoyés sont posées sur ce mois (R-AP29).
    pub async fn preparer_elements_paie_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        body: &PreparerPaieBody,
    ) -> Result<BilanPaieDto, EntDbError> {
        self.exiger_paie(etab, actor, role).await?;
        let mois = premier_du_mois(body.mois);
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let mut bilan = BilanPaieDto::default();
        // 1. Heures de l'OGEC : le mois, puis les mois déjà envoyés dont une régularisation est en attente.
        let heures: Vec<(Uuid, Uuid, String, i32, NaiveDate, i64)> = sqlx::query_as(
            "SELECT h.id, h.agent_id, h.nature, h.duree_minutes, h.mois,
                    COALESCE((SELECT sum(f.duree_apres - f.duree_avant) FROM ent_absp_heure_fait f
                               WHERE f.etablissement_id = h.etablissement_id AND f.heure_id = h.id AND f.type = 'regularisation'), 0)::int8
               FROM ent_absp_heure h
              WHERE h.etablissement_id = $1 AND h.employeur = 'ogec' AND h.etat IN ('validee', 'figee', 'exportee')
                AND (h.mois = $2 OR (h.mois < $2 AND EXISTS (SELECT 1 FROM ent_absp_element_paie e
                        WHERE e.etablissement_id = h.etablissement_id AND e.heure_id = h.id AND e.mois = h.mois AND e.etat <> 'prepare')))
              ORDER BY h.le, h.id",
        )
        .bind(etab)
        .bind(mois)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        for (hid, agent, nature, duree, mois_h, regul) in heures {
            let cible = centiemes_heure(i64::from(duree) + regul);
            let (c, r) = self.poser_element(&mut tx, etab, agent, None, Some(hid), mois_h, mois, &nature, cible, "heure").await?;
            bilan.crees += c;
            bilan.regularisations += r;
        }
        // 2. Absences décidées des salariés de l'OGEC qui touchent le mois (nature codée, jamais un libellé).
        let absences: Vec<(Uuid, Uuid, String, NaiveDate, NaiveDate, String)> = sqlx::query_as(
            "SELECT a.id, a.agent_id, n.code, ent_aujourdhui_etab(a.etablissement_id, a.debut),
                    ent_aujourdhui_etab(a.etablissement_id, a.fin - interval '1 second'), a.etat
               FROM ent_absp_absence a JOIN ent_absp_nature n ON n.id = a.nature_id
              WHERE a.etablissement_id = $1 AND a.employeur = 'ogec' AND a.cause = 'rh'
                AND a.etat IN ('validee', 'traitee', 'close', 'annulee')
                AND ent_aujourdhui_etab(a.etablissement_id, a.debut) <= $3
                AND ent_aujourdhui_etab(a.etablissement_id, a.fin - interval '1 second') >= $2
              ORDER BY a.debut, a.id LIMIT 2000",
        )
        .bind(etab)
        .bind(mois)
        .bind(dernier_du_mois(mois))
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        for (aid, agent, code, d, f, etat) in absences {
            let du = d.max(mois);
            let au = f.min(dernier_du_mois(mois));
            let jours = if etat == "annulee" {
                0
            } else {
                crate::structure_calendrier::calendrier::jours_ouvres_administratifs(&mut tx, etab, du, au, "secretariat").await?.len() as i64
            };
            let (centi, unite) = if d == f && jours == 1 && self.absence_dans_une_demi_journee(&mut tx, etab, aid).await? {
                (100, "demi_jour")
            } else {
                (jours * 100, "jour")
            };
            let (c, r) = self.poser_element(&mut tx, etab, agent, Some(aid), None, mois, mois, &code, centi, unite).await?;
            bilan.crees += c;
            bilan.regularisations += r;
        }
        tx.commit().await.map_err(db)?;
        Ok(bilan)
    }

    /// L'absence tient-elle dans une demi-journée (une même matinée ou un même après-midi, heure de l'établissement) ?
    async fn absence_dans_une_demi_journee(&self, conn: &mut PgConnection, etab: Uuid, absence: Uuid) -> Result<bool, EntDbError> {
        sqlx::query_scalar(
            "SELECT (a.fin - a.debut) <= interval '5 hours'
                AND ((a.debut AT TIME ZONE tz.f)::time < time '12:30') = (((a.fin - interval '1 second') AT TIME ZONE tz.f)::time < time '12:30')
               FROM ent_absp_absence a,
                    LATERAL (SELECT COALESCE((SELECT fuseau FROM ent_etablissement_parametre WHERE etablissement_id = a.etablissement_id),
                                             'Europe/Paris') AS f) tz
              WHERE a.etablissement_id = $1 AND a.id = $2",
        )
        .bind(etab)
        .bind(absence)
        .fetch_one(conn)
        .await
        .map_err(db)
    }

    /// Les éléments d'un mois (ABP-209), pour un agent ou tous les salariés de l'OGEC.
    pub async fn elements_paie_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        mois: NaiveDate,
        agent_id: Option<Uuid>,
    ) -> Result<Vec<ElementPaieDto>, EntDbError> {
        self.exiger_paie(etab, actor, role).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        elements_tx(&mut tx, etab, premier_du_mois(mois), agent_id).await
    }

    /// ABP-207 : exporter les éléments d'un mois (CSV) ; ceux qui étaient « préparés » deviennent « envoyés » et ne
    /// se réécrivent plus. Rejouable : un second export renvoie les mêmes lignes, sans rien changer.
    pub async fn exporter_elements_paie_csv_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        mois: NaiveDate,
    ) -> Result<String, EntDbError> {
        self.exiger_paie(etab, actor, role).await?;
        let mois = premier_du_mois(mois);
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let n = sqlx::query(
            "UPDATE ent_absp_element_paie SET etat = 'envoye', envoye_le = now(), envoye_par_user_id = $3
              WHERE etablissement_id = $1 AND mois = $2 AND etat = 'prepare'",
        )
        .bind(etab)
        .bind(mois)
        .bind(actor)
        .execute(&mut *tx)
        .await
        .map_err(db)?
        .rows_affected();
        let elements = elements_tx(&mut tx, etab, mois, None).await?;
        let noms = self.noms_agents(&mut tx, etab, &elements.iter().map(|e| e.agent_id).collect::<Vec<_>>()).await?;
        journal_objet(&mut tx, etab, "element_paie", Uuid::new_v4(), "paie_exportee", Some("mois"), Some(&n.to_string()), Some(&mois.to_string()), actor)
            .await?;
        tx.commit().await.map_err(db)?;
        Ok(csv_paie(mois, &elements, &noms))
    }

    // ───────────────────────────── DSN J+5 (ABP-210, 211) ─────────────────────────────

    /// Crée, pour les arrêts de travail des salariés de l'OGEC (nature à arrêt attendu, absence décidée), le suivi
    /// « à signaler en DSN » : échéance J+5 depuis la connaissance de l'arrêt (sa réception, sinon la déclaration).
    /// Idempotent. ScolIA ne produit pas de DSN (socle 49).
    pub(crate) async fn preparer_dsn_tx(&self, conn: &mut PgConnection, etab: Uuid) -> Result<u32, EntDbError> {
        let lignes: Vec<(Uuid, NaiveDate)> = sqlx::query_as(
            "SELECT a.id, ent_aujourdhui_etab(a.etablissement_id,
                        COALESCE((SELECT min(r.recu_le) FROM ent_absp_arret r WHERE r.etablissement_id = a.etablissement_id AND r.absence_id = a.id),
                                 a.cree_le))
               FROM ent_absp_absence a JOIN ent_absp_nature n ON n.id = a.nature_id
              WHERE a.etablissement_id = $1 AND a.employeur = 'ogec' AND a.cause = 'rh' AND n.piece_attendue = 'arret'
                AND a.etat IN ('validee', 'traitee', 'close', 'declaree', 'en_instruction')
                AND NOT EXISTS (SELECT 1 FROM ent_absp_dsn_suivi s WHERE s.etablissement_id = a.etablissement_id AND s.absence_id = a.id)
              LIMIT 1000",
        )
        .bind(etab)
        .fetch_all(&mut *conn)
        .await
        .map_err(db)?;
        let mut n = 0;
        for (aid, connaissance) in lignes {
            n += sqlx::query(
                "INSERT INTO ent_absp_dsn_suivi (id, etablissement_id, absence_id, echeance_le) VALUES ($1,$2,$3,$4)
                 ON CONFLICT (etablissement_id, absence_id) DO NOTHING",
            )
            .bind(Uuid::new_v4())
            .bind(etab)
            .bind(aid)
            .bind(echeance_dsn(connaissance))
            .execute(&mut *conn)
            .await
            .map_err(db)?
            .rows_affected() as u32;
        }
        Ok(n)
    }

    /// ABP-210 : la file « arrêts à signaler en DSN » (échéance, retard), de la plus proche à la plus lointaine.
    /// Met d'abord la file à jour. Comptabilité de l'OGEC et direction.
    pub async fn dsn_file_for_actor(&self, etab: Uuid, actor: Uuid, role: &str) -> Result<Vec<DsnSuiviDto>, EntDbError> {
        self.exiger_paie(etab, actor, role).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        self.preparer_dsn_tx(&mut tx, etab).await?;
        let aujourdhui = jour_etab(&mut tx, etab).await?;
        let v: Vec<(Uuid, Uuid, NaiveDate, Option<NaiveDate>)> = sqlx::query_as(
            "SELECT s.absence_id, a.agent_id, s.echeance_le, s.signale_le FROM ent_absp_dsn_suivi s
               JOIN ent_absp_absence a ON a.etablissement_id = s.etablissement_id AND a.id = s.absence_id
              WHERE s.etablissement_id = $1 AND s.signale_le IS NULL AND a.etat <> 'annulee'
              ORDER BY s.echeance_le, s.absence_id LIMIT 500",
        )
        .bind(etab)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        Ok(v
            .into_iter()
            .map(|(absence_id, agent_id, echeance_le, signale_le)| DsnSuiviDto {
                absence_id,
                agent_id,
                echeance_le,
                signale_le,
                en_retard: signale_le.is_none() && echeance_le < aujourdhui,
            })
            .collect())
    }

    /// ABP-211 : marquer « signalé en DSN le … » (jamais dans le futur). Une seule fois.
    pub async fn marquer_dsn_signale_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        absence_id: Uuid,
        le: NaiveDate,
    ) -> Result<DsnSuiviDto, EntDbError> {
        self.exiger_paie(etab, actor, role).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let aujourdhui = jour_etab(&mut tx, etab).await?;
        if le > aujourdhui {
            return Err(invalide("Un signalement ne se date pas dans le futur."));
        }
        let (echeance, signale): (NaiveDate, Option<NaiveDate>) = sqlx::query_as(
            "SELECT echeance_le, signale_le FROM ent_absp_dsn_suivi WHERE etablissement_id = $1 AND absence_id = $2 FOR UPDATE",
        )
        .bind(etab)
        .bind(absence_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?
        .ok_or_else(introuvable)?;
        if signale.is_some() {
            return Err(conflit("ce signalement est déjà marqué"));
        }
        sqlx::query(
            "UPDATE ent_absp_dsn_suivi SET signale_le = $3, signale_par_user_id = $4 WHERE etablissement_id = $1 AND absence_id = $2",
        )
        .bind(etab)
        .bind(absence_id)
        .bind(le)
        .bind(actor)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        journal_objet(&mut tx, etab, "dsn", absence_id, "dsn_signale", Some("signale_le"), None, Some(&le.to_string()), actor).await?;
        let agent: Uuid = sqlx::query_scalar("SELECT agent_id FROM ent_absp_absence WHERE etablissement_id = $1 AND id = $2")
            .bind(etab)
            .bind(absence_id)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
        tx.commit().await.map_err(db)?;
        Ok(DsnSuiviDto { absence_id, agent_id: agent, echeance_le: echeance, signale_le: Some(le), en_retard: false })
    }

    /// Tâche planifiée : tient la file DSN à jour pour tous les établissements.
    pub async fn preparer_dsn_planifie(&self) -> Result<u32, EntDbError> {
        let mut n = 0;
        for etab in etablissements_systeme(self.pool(), ContexteSysteme::JobPlanifie("absences_personnels_dsn"))
            .await
            .map_err(db)?
        {
            let r: Result<u32, EntDbError> = async {
                let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
                let c = self.preparer_dsn_tx(&mut tx, etab).await?;
                tx.commit().await.map_err(db)?;
                Ok(c)
            }
            .await;
            match r {
                Ok(c) => n += c,
                Err(e) => tracing::warn!(%etab, "file DSN : {e}"),
            }
        }
        Ok(n)
    }
}

async fn elements_tx(conn: &mut PgConnection, etab: Uuid, mois: NaiveDate, agent: Option<Uuid>) -> Result<Vec<ElementPaieDto>, EntDbError> {
    let v: Vec<(Uuid, Uuid, Option<Uuid>, Option<Uuid>, NaiveDate, String, f64, String, String, Option<Uuid>)> = sqlx::query_as(
        "SELECT id, agent_id, absence_id, heure_id, mois, nature_code, quantite::float8, unite, etat, regularise_de_id
           FROM ent_absp_element_paie
          WHERE etablissement_id = $1 AND mois = $2 AND ($3::uuid IS NULL OR agent_id = $3)
          ORDER BY agent_id, nature_code, id",
    )
    .bind(etab)
    .bind(mois)
    .bind(agent)
    .fetch_all(conn)
    .await
    .map_err(db)?;
    Ok(v
        .into_iter()
        .map(|(id, agent_id, absence_id, heure_id, mois, nature_code, quantite, unite, etat, regularise_de_id)| ElementPaieDto {
            id,
            agent_id,
            absence_id,
            heure_id,
            mois,
            nature_code,
            quantite: arrondi1(quantite * 10.0) / 10.0,
            unite,
            etat,
            regularise_de_id,
        })
        .collect())
}

#[allow(dead_code)]
fn _lundi(d: NaiveDate) -> NaiveDate {
    lundi_de(d)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn j(m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2027, m, d).unwrap()
    }

    #[test]
    fn la_nature_d_une_heure_de_remplacement_suit_l_engagement() {
        assert_eq!(nature_remplacement(Some("pacte")), "mission_pacte");
        assert_eq!(nature_remplacement(Some("hse")), "hse");
        assert_eq!(nature_remplacement(Some("autre")), "rcd");
        assert_eq!(nature_remplacement(None), "rcd");
    }

    #[test]
    fn mois_et_echeances() {
        assert_eq!(premier_du_mois(j(11, 14)), j(11, 1));
        assert_eq!(dernier_du_mois(j(11, 1)), j(11, 30));
        assert_eq!(dernier_du_mois(j(12, 1)), j(12, 31));
        assert_eq!(mois_suivant(j(12, 1)), NaiveDate::from_ymd_opt(2028, 1, 1).unwrap());
        assert_eq!(dernier_du_mois(j(2, 1)), j(2, 28));
        assert_eq!(echeance_dsn(j(11, 14)), j(11, 19), "J+5");
        assert_eq!(echeance_dsn(j(11, 28)), j(12, 3));
    }

    #[test]
    fn minutes_et_centiemes() {
        let t = |h, m| NaiveTime::from_hms_opt(h, m, 0).unwrap();
        assert_eq!(minutes_entre(t(9, 0), t(10, 30)), 90);
        assert_eq!(minutes_entre(t(10, 0), t(9, 0)), 0);
        assert_eq!(centiemes_heure(90), 150);
        assert_eq!(centiemes_heure(20), 33);
        assert_eq!(centiemes_heure(-20), -33);
        assert_eq!(centiemes_heure(0), 0);
        assert_eq!(duree_effective(60, -15), 45);
    }

    #[test]
    fn chacun_son_perimetre_d_employeur() {
        assert_eq!(employeurs_permis(Role::Direction), &["etat", "ogec"]);
        assert_eq!(employeurs_permis(Role::Administratif), &["etat"]);
        assert_eq!(employeurs_permis(Role::Compta), &["ogec"]);
        assert!(employeurs_permis(Role::Prof).is_empty());
        assert!(employeurs_permis(Role::Admin).is_empty(), "l'administrateur technique ne lit aucune heure");
    }

    #[test]
    fn les_exports_neutralisent_les_formules_et_ne_portent_aucune_nature_d_absence() {
        let agent = Uuid::from_u128(5);
        let noms = std::collections::HashMap::from([(agent, "=cmd Dupont".to_string())]);
        let p = csv_paie(
            j(11, 1),
            &[ElementPaieDto {
                id: Uuid::nil(),
                agent_id: agent,
                absence_id: Some(Uuid::from_u128(9)),
                heure_id: None,
                mois: j(11, 1),
                nature_code: "convenance".into(),
                quantite: 2.0,
                unite: "jour".into(),
                etat: "prepare".into(),
                regularise_de_id: None,
            }],
            &noms,
        );
        assert!(p.contains("'=cmd Dupont"), "{p}");
        assert!(p.contains(",convenance,2.00,jour,non"), "{p}");
        assert!(!p.contains(&Uuid::from_u128(9).to_string()), "l'identifiant d'absence ne sort pas du tableau");
    }
}
