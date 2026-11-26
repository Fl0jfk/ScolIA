//! Socle 27, **lot G4-2** : contrôles internes, alertes d'accès, purge par partition, traces dans l'extrait
//! (RGP-193 à RGP-199, R-RG27, R-RG28, §11.4, §11.5).
//!
//! * **Traces d'un sujet** (RGP-193) : `acces.consulter_traces`, **seulement** dans le cadre d'une demande de droit
//!   **ouverte** ou d'un contrôle **ouvert** ; la lecture est elle-même tracée (section non affichable aux familles).
//!   Aucune statistique individuelle, aucun classement « qui consulte le plus » (§11.4) : seules des lignes, jamais de
//!   total par agent.
//! * **Contrôle interne** (RGP-194, RGP-195, R-RG27) : `acces.controle_ouvrir`, **passkey récente** (défi lié au
//!   contrôle demandé), motif, durée **bornée** (31 jours), clôture avec une issue.
//! * **Différer** (RGP-197, R-RG28) : une consultation liée à un signalement (contrôle ouvert pour ce motif sur l'agent
//!   qui a consulté) n'est pas montrée à la famille pendant une durée **bornée** (90 jours), sur décision **motivée**
//!   de la direction (`droits.arbitrer`) ; rien n'est effacé.
//! * **Alertes** (RGP-196) : règles **déterministes**, une alerte par règle, agent et fenêtre ; notification
//!   `acces.alerte` aux comptes qui portent `acces.consulter_traces` (direction, DPO), **jamais** à l'agent concerné.
//! * **Purge** (RGP-198, S-541) : suppression d'une **partition** entière par une fonction de plateforme journalisée,
//!   jamais un `DELETE` ; une partition qui contient des traces d'un sujet visé par une demande ouverte (ou d'un agent
//!   sous contrôle ouvert) est **différée** et le DPO est alerté.
//! * **Extrait** (RGP-199) : section « consultations » du sujet, mêmes règles que la page du lot G4-1 ; le nom d'un agent
//!   n'y figure que sur arbitrage motivé de la direction (CL27-16).

use std::collections::BTreeMap;

use chrono::{DateTime, Datelike, Duration, Months, NaiveDate, TimeZone, Utc};
use domain_eleve::DomainError;
use ent_auth::{Action, PublicKeyCredential};
use sha2::{Digest, Sha256};
use sqlx::{PgConnection, Row};
use uuid::Uuid;

use crate::acces_traces::{
    ecrire_trace_tx, libelle_nature, libelle_section, ContexteTrace, Perimetre,
};
use crate::droits_remise::DefiRemise;
use crate::notifications::{notifier_personne, EnvoiAPlanifier};
use crate::service::{EntDbError, PgEnt, RefusMetier};
use crate::tenant_tx::{etablissements_systeme, ContexteSysteme, TenantTx};

/// Nature des notifications : **aucun nom d'agent ni d'élève** (§11.3, R-RG32).
pub const KIND_ALERTE: &str = "acces.alerte";
pub const KIND_PURGE_DIFFEREE: &str = "acces.purge_differee";

/// Durée maximale d'un contrôle (jours) et d'une consultation différée (jours).
pub const CONTROLE_JOURS_MAX: i64 = 31;
pub const DIFFERE_JOURS_MAX: i64 = 90;

fn non_trouve() -> EntDbError {
    EntDbError::Domain(DomainError::NotFound)
}

fn conflit(m: &'static str) -> EntDbError {
    EntDbError::Refus(RefusMetier::Conflit, m)
}

// ─────────────────────────────── Réglages (S-541, S-543, S-554) ───────────────────────────────

/// Réglages d'accès d'un établissement ; l'absence de ligne vaut valeurs par défaut (D-069).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReglageAcces {
    pub trace_mois: i32,
    pub rafale_seuil: i32,
    pub rafale_fenetre_min: i32,
    pub seuil_direction: i32,
    pub seuil_enseignant: i32,
    pub seuil_vie_scolaire: i32,
    pub seuil_administratif: i32,
}

impl Default for ReglageAcces {
    fn default() -> Self {
        Self {
            trace_mois: 12,
            rafale_seuil: 15,
            rafale_fenetre_min: 10,
            seuil_direction: 300,
            seuil_enseignant: 80,
            seuil_vie_scolaire: 200,
            seuil_administratif: 150,
        }
    }
}

/// Un réglage affiché : code, libellé, valeur, valeur par défaut, bornes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LigneReglage {
    pub code: &'static str,
    pub libelle: &'static str,
    pub valeur: i32,
    pub defaut: i32,
    pub min: i32,
    pub max: i32,
}

/// Réglages modifiables (liste **fermée** : jamais d'entrée libre dans le SQL) : code, libellé, bornes.
const REGLAGES: &[(&str, &str, i32, i32)] = &[
    (
        "acces_trace_mois",
        "Conservation des traces d'accès (mois)",
        12,
        60,
    ),
    (
        "acces_rafale_seuil",
        "Rafale : consultations hors périmètre dans la fenêtre",
        3,
        500,
    ),
    (
        "acces_rafale_fenetre_min",
        "Rafale : fenêtre (minutes)",
        1,
        240,
    ),
    (
        "acces_seuil_direction",
        "Seuil journalier : direction",
        10,
        5000,
    ),
    (
        "acces_seuil_enseignant",
        "Seuil journalier : enseignants",
        10,
        5000,
    ),
    (
        "acces_seuil_vie_scolaire",
        "Seuil journalier : vie scolaire",
        10,
        5000,
    ),
    (
        "acces_seuil_administratif",
        "Seuil journalier : administration",
        10,
        5000,
    ),
];

fn valeur_de(r: &ReglageAcces, code: &str) -> i32 {
    match code {
        "acces_trace_mois" => r.trace_mois,
        "acces_rafale_seuil" => r.rafale_seuil,
        "acces_rafale_fenetre_min" => r.rafale_fenetre_min,
        "acces_seuil_direction" => r.seuil_direction,
        "acces_seuil_enseignant" => r.seuil_enseignant,
        "acces_seuil_vie_scolaire" => r.seuil_vie_scolaire,
        _ => r.seuil_administratif,
    }
}

pub(crate) async fn lire_reglage_acces(
    conn: &mut PgConnection,
    etab: Uuid,
) -> Result<ReglageAcces, EntDbError> {
    let l: Option<(i32, i32, i32, i32, i32, i32, i32)> = sqlx::query_as(
        "SELECT acces_trace_mois, acces_rafale_seuil, acces_rafale_fenetre_min, acces_seuil_direction,
                acces_seuil_enseignant, acces_seuil_vie_scolaire, acces_seuil_administratif
           FROM rgp_reglage WHERE etablissement_id = $1",
    )
    .bind(etab)
    .fetch_optional(&mut *conn)
    .await?;
    Ok(l.map(|(a, b, c, d, e, f, g)| ReglageAcces {
        trace_mois: a,
        rafale_seuil: b,
        rafale_fenetre_min: c,
        seuil_direction: d,
        seuil_enseignant: e,
        seuil_vie_scolaire: f,
        seuil_administratif: g,
    })
    .unwrap_or_default())
}

/// Une modification passée d'un réglage.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChangementReglage {
    pub reglage: String,
    pub ancienne: Option<String>,
    pub nouvelle: String,
    pub le: DateTime<Utc>,
}

// ─────────────────────────────── Contrôles ───────────────────────────────

pub const MOTIFS_CONTROLE: &[&str] = &[
    "signalement",
    "alerte",
    "plainte",
    "verification_periodique",
];
pub const ISSUES_CONTROLE: &[&str] = &["rien", "violation", "transmis"];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ControleVue {
    pub id: Uuid,
    pub agent_compte_id: Uuid,
    /// Nom de l'agent contrôlé : connu de la direction qui ouvre le contrôle, jamais d'ailleurs.
    pub agent_nom: String,
    pub motif_code: String,
    pub motif_texte: String,
    pub ouvert_le: DateTime<Utc>,
    pub echeance_le: DateTime<Utc>,
    pub clos_le: Option<DateTime<Utc>>,
    pub issue: Option<String>,
    /// Ouvert : ni clos ni échu à l'instant de la lecture.
    pub ouvert: bool,
}

type LigneControle = (
    Uuid,
    Uuid,
    String,
    String,
    DateTime<Utc>,
    DateTime<Utc>,
    Option<DateTime<Utc>>,
    Option<String>,
);

fn controle_depuis(l: LigneControle, nom: String, maintenant: DateTime<Utc>) -> ControleVue {
    let (id, agent, code, texte, ouvert_le, echeance, clos, issue) = l;
    ControleVue {
        id,
        agent_compte_id: agent,
        agent_nom: nom,
        motif_code: code,
        motif_texte: texte,
        ouvert_le,
        echeance_le: echeance,
        ouvert: clos.is_none() && echeance > maintenant,
        clos_le: clos,
        issue,
    }
}

/// Nom lisible d'un compte (prénom nom de sa fiche personne, à défaut l'adresse de connexion).
async fn nom_du_compte(
    conn: &mut PgConnection,
    etab: Uuid,
    compte: Uuid,
) -> Result<String, EntDbError> {
    let n: Option<String> = sqlx::query_scalar(
        "SELECT btrim(COALESCE(prenom || ' ', '') || nom) FROM ent_personne
          WHERE etablissement_id = $1 AND user_id = $2 AND fusionnee_dans IS NULL LIMIT 1",
    )
    .bind(etab)
    .bind(compte)
    .fetch_optional(&mut *conn)
    .await?;
    Ok(n.unwrap_or_else(|| "Agent".to_string()))
}

/// Ce que le défi du contrôle demandé lie : la même chaîne à l'ouverture, sinon refus.
fn empreinte_controle(agent: Uuid, motif_code: &str, motif_texte: &str, jours: i64) -> String {
    let mut h = Sha256::new();
    h.update(format!(
        "controle|{agent}|{motif_code}|{}|{jours}",
        motif_texte.trim()
    ));
    h.finalize().iter().map(|b| format!("{b:02x}")).collect()
}

fn verifier_demande_controle(
    motif_code: &str,
    motif_texte: &str,
    jours: i64,
) -> Result<(), EntDbError> {
    if !MOTIFS_CONTROLE.contains(&motif_code) {
        return Err(EntDbError::SaisieInvalide("motif de contrôle inconnu"));
    }
    let n = motif_texte.trim().chars().count();
    if !(5..=1000).contains(&n) {
        return Err(EntDbError::SaisieInvalide(
            "précisez le motif du contrôle (5 à 1000 caractères)",
        ));
    }
    if !(1..=CONTROLE_JOURS_MAX).contains(&jours) {
        return Err(EntDbError::SaisieInvalide(
            "durée du contrôle : de 1 à 31 jours",
        ));
    }
    Ok(())
}

/// Le compte est un agent (ni parent ni élève) de l'établissement.
async fn exiger_agent(conn: &mut PgConnection, etab: Uuid, compte: Uuid) -> Result<(), EntDbError> {
    let ok: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM ent_membership
                         WHERE etablissement_id = $1 AND user_id = $2 AND role NOT IN ('parent', 'eleve'))",
    )
    .bind(etab)
    .bind(compte)
    .fetch_one(&mut *conn)
    .await?;
    if ok {
        Ok(())
    } else {
        Err(non_trouve())
    }
}

async fn controle_ouvert_de(
    conn: &mut PgConnection,
    etab: Uuid,
    agent: Uuid,
    maintenant: DateTime<Utc>,
) -> Result<Option<Uuid>, EntDbError> {
    Ok(sqlx::query_scalar(
        "SELECT id FROM acc_controle
          WHERE etablissement_id = $1 AND agent_compte_id = $2 AND clos_le IS NULL AND echeance_le > $3
          LIMIT 1",
    )
    .bind(etab)
    .bind(agent)
    .bind(maintenant)
    .fetch_optional(&mut *conn)
    .await?)
}

// ─────────────────────────────── Vues ───────────────────────────────

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TraceLigne {
    pub id: Uuid,
    pub le: DateTime<Utc>,
    pub fonction: String,
    pub service: String,
    pub nature: String,
    pub section: String,
    pub hors_perimetre: bool,
    pub perimetre_temporaire: Option<String>,
    pub motif: Option<String>,
    /// Élève consulté (contrôle d'un agent) ; vide pour les traces d'un sujet.
    pub sujet: String,
    /// Consultation différée pour la famille (R-RG28).
    pub differee: bool,
    /// Le nom de l'agent a été communiqué sur arbitrage pour cette demande.
    pub nom_communique: bool,
    pub agent_nom: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TracesSujet {
    pub sujet_type: String,
    pub sujet_id: Uuid,
    pub libelle: String,
    pub lignes: Vec<TraceLigne>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AlerteVue {
    pub id: Uuid,
    pub regle: String,
    pub libelle: String,
    pub agent_compte_id: Uuid,
    pub agent_nom: String,
    pub fenetre_debut: DateTime<Utc>,
    pub fenetre_fin: DateTime<Utc>,
    pub nombre: i32,
    pub cree_le: DateTime<Utc>,
    pub suite: String,
    pub suite_le: Option<DateTime<Utc>>,
    pub controle_id: Option<Uuid>,
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct RapportPurge {
    pub purgees: Vec<NaiveDate>,
    pub differees: Vec<NaiveDate>,
}

pub fn libelle_regle(code: &str) -> &'static str {
    match code {
        "rafale_hors_perimetre" => "Rafale de consultations hors périmètre",
        "seuil_jour" => "Nombre de consultations inhabituel sur une journée",
        "adresse_protegee" => "Élève à adresse protégée consulté sans lien",
        "conservation" => "Élève sorti ou archivé consulté",
        _ => "Alerte d'accès",
    }
}

fn debut_de_jour(jour: NaiveDate) -> DateTime<Utc> {
    Utc.from_utc_datetime(&jour.and_hms_opt(0, 0, 0).unwrap_or_default())
}

/// Les comptes de l'établissement qui portent une action (moteur de droits, rôles D-017 compris).
async fn comptes_avec_action(
    ent: &PgEnt,
    conn: &mut PgConnection,
    etab: Uuid,
    jour: NaiveDate,
    action: Action,
) -> Result<Vec<Uuid>, EntDbError> {
    let membres: Vec<(Uuid, String)> = sqlx::query_as(
        "SELECT m.user_id, m.role FROM ent_membership m
          WHERE m.etablissement_id = $1 AND m.role NOT IN ('parent', 'eleve')
            AND COALESCE(m.etat, 'ouvert') = 'ouvert' AND (m.date_fin IS NULL OR m.date_fin >= $2)",
    )
    .bind(etab)
    .bind(jour)
    .fetch_all(&mut *conn)
    .await?;
    let mut out = Vec::new();
    for (u, role) in membres {
        let droits = ent
            .droits_effectifs_conn(&mut *conn, etab, u, &role)
            .await?;
        if droits.iter().any(|d| d.action == action) {
            out.push(u);
        }
    }
    Ok(out)
}

fn limite_traces(maintenant: DateTime<Utc>, mois: i32) -> DateTime<Utc> {
    let m = u32::try_from(mois.max(12)).unwrap_or(12);
    maintenant
        .checked_sub_months(Months::new(m))
        .unwrap_or(maintenant - Duration::days(365))
}

// ─────────────────────────────── Services ───────────────────────────────

impl PgEnt {
    // ── Réglages ──

    /// Réglages d'accès (lecture : `conformite.gerer` ou `acces.consulter_traces`) avec valeur par défaut et bornes.
    pub async fn acces_reglages_lire(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
    ) -> Result<Vec<LigneReglage>, EntDbError> {
        if !self
            .a_action_etablissement(etab, acteur, role, Action::ConformiteGerer)
            .await?
            && !self
                .a_action_etablissement(etab, acteur, role, Action::AccesConsulterTraces)
                .await?
        {
            return Err(EntDbError::Forbidden);
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let r = lire_reglage_acces(&mut tx, etab).await?;
        let d = ReglageAcces::default();
        Ok(REGLAGES
            .iter()
            .map(|(code, libelle, min, max)| LigneReglage {
                code,
                libelle,
                valeur: valeur_de(&r, code),
                defaut: valeur_de(&d, code),
                min: *min,
                max: *max,
            })
            .collect())
    }

    /// Modifie un réglage d'accès (`conformite.gerer`) ; ancienne et nouvelle valeur journalisées. S-541 : jamais moins de
    /// 12 mois.
    pub async fn acces_reglage_modifier(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        reglage: &str,
        valeur: i32,
    ) -> Result<Vec<LigneReglage>, EntDbError> {
        self.exiger_action_etab(etab, acteur, role, Action::ConformiteGerer)
            .await?;
        let Some((code, _, min, max)) = REGLAGES.iter().find(|(c, _, _, _)| *c == reglage) else {
            return Err(EntDbError::SaisieInvalide("réglage inconnu"));
        };
        if !(*min..=*max).contains(&valeur) {
            return Err(EntDbError::SaisieInvalide(
                "valeur hors des bornes du réglage",
            ));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let avant = valeur_de(&lire_reglage_acces(&mut tx, etab).await?, code);
        // Colonne choisie dans la liste fermée ci-dessus : jamais d'entrée libre.
        let sql = format!(
            "INSERT INTO rgp_reglage (etablissement_id, {code}, modifie_par_compte_id) VALUES ($1, $2, $3)
             ON CONFLICT (etablissement_id) DO UPDATE
                SET {code} = EXCLUDED.{code}, modifie_par_compte_id = $3, modifie_le = now()"
        );
        sqlx::query(&sql)
            .bind(etab)
            .bind(valeur)
            .bind(acteur)
            .execute(&mut *tx)
            .await?;
        sqlx::query(
            "INSERT INTO rgp_reglage_journal (id, etablissement_id, reglage, ancienne_valeur, nouvelle_valeur, par_compte_id)
             VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(Uuid::new_v4())
        .bind(etab)
        .bind(*code)
        .bind(avant.to_string())
        .bind(valeur.to_string())
        .bind(acteur)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        self.acces_reglages_lire(etab, acteur, role).await
    }

    /// Historique des réglages RGPD (les 100 dernières modifications).
    pub async fn acces_reglages_historique(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
    ) -> Result<Vec<ChangementReglage>, EntDbError> {
        if !self
            .a_action_etablissement(etab, acteur, role, Action::ConformiteGerer)
            .await?
            && !self
                .a_action_etablissement(etab, acteur, role, Action::AccesConsulterTraces)
                .await?
        {
            return Err(EntDbError::Forbidden);
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let l: Vec<(String, Option<String>, String, DateTime<Utc>)> = sqlx::query_as(
            "SELECT reglage, ancienne_valeur, nouvelle_valeur, le FROM rgp_reglage_journal
              WHERE etablissement_id = $1 ORDER BY le DESC, id LIMIT 100",
        )
        .bind(etab)
        .fetch_all(&mut *tx)
        .await?;
        Ok(l.into_iter()
            .map(|(reglage, ancienne, nouvelle, le)| ChangementReglage {
                reglage,
                ancienne,
                nouvelle,
                le,
            })
            .collect())
    }

    // ── Contrôles internes ──

    /// Démarre l'ouverture d'un contrôle : **défi de passkey** du directeur, lié au contrôle demandé (agent, motif, durée).
    #[allow(clippy::too_many_arguments)]
    pub async fn acces_controle_defi_a(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        agent: Uuid,
        motif_code: &str,
        motif_texte: &str,
        jours: i64,
        maintenant: DateTime<Utc>,
    ) -> Result<DefiRemise, EntDbError> {
        self.exiger_action_etab(etab, acteur, role, Action::AccesControleOuvrir)
            .await?;
        verifier_demande_controle(motif_code, motif_texte, jours)?;
        if agent == acteur {
            return Err(conflit("un contrôle ne porte pas sur soi-même"));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        exiger_agent(&mut tx, etab, agent).await?;
        if controle_ouvert_de(&mut tx, etab, agent, maintenant)
            .await?
            .is_some()
        {
            return Err(conflit("un contrôle est déjà ouvert sur cet agent"));
        }
        let empreinte = empreinte_controle(agent, motif_code, motif_texte, jours);
        let defi = self
            .defi_creer(
                &mut tx,
                etab,
                acteur,
                None,
                "controle",
                &empreinte,
                "J'ouvre un contrôle interne sur les consultations d'un agent, pour le motif indiqué.",
                maintenant,
            )
            .await?;
        tx.commit().await?;
        Ok(defi)
    }

    /// Ouvre le contrôle (`acces.controle_ouvrir` ★) : le défi doit être celui **de cette demande**, brûlé d'abord.
    #[allow(clippy::too_many_arguments)]
    pub async fn acces_controle_ouvrir_a(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        agent: Uuid,
        motif_code: &str,
        motif_texte: &str,
        jours: i64,
        defi: Uuid,
        credential: &PublicKeyCredential,
        maintenant: DateTime<Utc>,
    ) -> Result<ControleVue, EntDbError> {
        self.exiger_action_etab(etab, acteur, role, Action::AccesControleOuvrir)
            .await?;
        verifier_demande_controle(motif_code, motif_texte, jours)?;
        {
            let mut tx = TenantTx::begin(self.pool(), etab).await?;
            let attendu: Option<String> = sqlx::query_scalar(
                "SELECT empreinte FROM rgp_defi WHERE etablissement_id = $1 AND id = $2 AND compte_id = $3",
            )
            .bind(etab)
            .bind(defi)
            .bind(acteur)
            .fetch_optional(&mut *tx)
            .await?;
            let attendue = empreinte_controle(agent, motif_code, motif_texte, jours);
            if attendu.as_deref() != Some(attendue.as_str()) {
                return Err(non_trouve());
            }
        }
        // Le défi est brûlé d'abord : sans passkey récente du directeur, rien ne s'ouvre.
        self.defi_consommer(etab, acteur, None, "controle", defi, credential, maintenant)
            .await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        exiger_agent(&mut tx, etab, agent).await?;
        if controle_ouvert_de(&mut tx, etab, agent, maintenant)
            .await?
            .is_some()
        {
            return Err(conflit("un contrôle est déjà ouvert sur cet agent"));
        }
        let id = Uuid::new_v4();
        let echeance = maintenant + Duration::days(jours);
        sqlx::query(
            "INSERT INTO acc_controle (id, etablissement_id, agent_compte_id, motif_code, motif_texte,
                                       ouvert_par_compte_id, ouvert_le, echeance_le)
             VALUES ($1, $2, $3, $4, btrim($5), $6, $7, $8)",
        )
        .bind(id)
        .bind(etab)
        .bind(agent)
        .bind(motif_code)
        .bind(motif_texte)
        .bind(acteur)
        .bind(maintenant)
        .bind(echeance)
        .execute(&mut *tx)
        .await?;
        let nom = nom_du_compte(&mut tx, etab, agent).await?;
        tx.commit().await?;
        Ok(ControleVue {
            id,
            agent_compte_id: agent,
            agent_nom: nom,
            motif_code: motif_code.to_string(),
            motif_texte: motif_texte.trim().to_string(),
            ouvert_le: maintenant,
            echeance_le: echeance,
            clos_le: None,
            issue: None,
            ouvert: true,
        })
    }

    /// Les contrôles de l'établissement (récents d'abord, 200 au plus). Aucun total par agent, aucun classement.
    pub async fn acces_controles_a(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        maintenant: DateTime<Utc>,
    ) -> Result<Vec<ControleVue>, EntDbError> {
        self.exiger_action_etab(etab, acteur, role, Action::AccesConsulterTraces)
            .await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let l: Vec<LigneControle> = sqlx::query_as(
            "SELECT id, agent_compte_id, motif_code, motif_texte, ouvert_le, echeance_le, clos_le, issue
               FROM acc_controle WHERE etablissement_id = $1 ORDER BY ouvert_le DESC, id LIMIT 200",
        )
        .bind(etab)
        .fetch_all(&mut *tx)
        .await?;
        let mut out = Vec::with_capacity(l.len());
        for c in l {
            let nom = nom_du_compte(&mut tx, etab, c.1).await?;
            out.push(controle_depuis(c, nom, maintenant));
        }
        Ok(out)
    }

    /// Clôt un contrôle ouvert avec une issue (`rien`, `violation`, `transmis`).
    pub async fn acces_controle_clore_a(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        controle: Uuid,
        issue: &str,
        maintenant: DateTime<Utc>,
    ) -> Result<ControleVue, EntDbError> {
        self.exiger_action_etab(etab, acteur, role, Action::AccesControleOuvrir)
            .await?;
        if !ISSUES_CONTROLE.contains(&issue) {
            return Err(EntDbError::SaisieInvalide("issue de contrôle inconnue"));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let n = sqlx::query(
            "UPDATE acc_controle SET clos_le = $3, clos_par_compte_id = $4, issue = $5
              WHERE etablissement_id = $1 AND id = $2 AND clos_le IS NULL",
        )
        .bind(etab)
        .bind(controle)
        .bind(maintenant)
        .bind(acteur)
        .bind(issue)
        .execute(&mut *tx)
        .await?
        .rows_affected();
        if n == 0 {
            let existe: bool = sqlx::query_scalar(
                "SELECT EXISTS (SELECT 1 FROM acc_controle WHERE etablissement_id = $1 AND id = $2)",
            )
            .bind(etab)
            .bind(controle)
            .fetch_one(&mut *tx)
            .await?;
            return Err(if existe {
                conflit("contrôle déjà clos")
            } else {
                non_trouve()
            });
        }
        let l: LigneControle = sqlx::query_as(
            "SELECT id, agent_compte_id, motif_code, motif_texte, ouvert_le, echeance_le, clos_le, issue
               FROM acc_controle WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(controle)
        .fetch_one(&mut *tx)
        .await?;
        let nom = nom_du_compte(&mut tx, etab, l.1).await?;
        tx.commit().await?;
        Ok(controle_depuis(l, nom, maintenant))
    }

    // ── Traces d'un sujet ou d'un agent ──

    /// Traces des **sujets d'une demande ouverte** (RGP-193) : fonction et service, jamais le nom. La lecture est tracée.
    pub async fn acces_traces_demande_a(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        demande: Uuid,
        maintenant: DateTime<Utc>,
    ) -> Result<Vec<TracesSujet>, EntDbError> {
        self.exiger_action_etab(etab, acteur, role, Action::AccesConsulterTraces)
            .await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let etat: Option<String> = sqlx::query_scalar(
            "SELECT etat FROM rgp_demande WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(demande)
        .fetch_optional(&mut *tx)
        .await?;
        match etat.as_deref() {
            None => return Err(non_trouve()),
            Some("close" | "refusee") => {
                return Err(conflit(
                    "les traces d'un sujet ne se consultent que pour une demande ouverte",
                ))
            }
            Some(_) => {}
        }
        let reglage = lire_reglage_acces(&mut tx, etab).await?;
        let depuis = limite_traces(maintenant, reglage.trace_mois);
        let sujets: Vec<(String, Uuid)> = sqlx::query_as(
            "SELECT sujet_type, sujet_id FROM (
                 SELECT sujet_type, sujet_id FROM rgp_demande
                  WHERE etablissement_id = $1 AND id = $2 AND sujet_id IS NOT NULL
                 UNION
                 SELECT sujet_type, sujet_id FROM rgp_demande_sujet WHERE etablissement_id = $1 AND demande_id = $2) x
              ORDER BY sujet_type, sujet_id",
        )
        .bind(etab)
        .bind(demande)
        .fetch_all(&mut *tx)
        .await?;
        let mut out = Vec::new();
        for (type_, id) in sujets {
            let (libelle, lignes) = match type_.as_str() {
                "eleve" => {
                    let l: Option<String> = sqlx::query_scalar(
                        "SELECT btrim(prenom || ' ' || nom) FROM ent_eleve WHERE etablissement_id = $1 AND id = $2",
                    )
                    .bind(etab)
                    .bind(id)
                    .fetch_optional(&mut *tx)
                    .await?;
                    let lignes = lire_traces(
                        &mut tx,
                        etab,
                        "eleve",
                        id,
                        None,
                        depuis,
                        maintenant,
                        Some(demande),
                    )
                    .await?;
                    (l.unwrap_or_default(), lignes)
                }
                _ => {
                    let l: Option<String> = sqlx::query_scalar(
                        "SELECT btrim(COALESCE(prenom || ' ', '') || nom) FROM ent_personne
                          WHERE etablissement_id = $1 AND id = $2",
                    )
                    .bind(etab)
                    .bind(id)
                    .fetch_optional(&mut *tx)
                    .await?;
                    let lignes = lire_traces(
                        &mut tx,
                        etab,
                        "personne",
                        id,
                        None,
                        depuis,
                        maintenant,
                        Some(demande),
                    )
                    .await?;
                    (l.unwrap_or_default(), lignes)
                }
            };
            // La lecture est tracée : une ligne non affichable aux familles, rattachée au sujet.
            let (eleve, personne) = match type_.as_str() {
                "eleve" => (Some(id), None),
                _ => (None, Some(id)),
            };
            ecrire_trace_tx(
                &mut tx,
                etab,
                Uuid::new_v4(),
                acteur,
                role,
                eleve,
                "acces_traces_lecture",
                &ContexteTrace {
                    perimetre: Perimetre::Dans,
                    motif: None,
                    personne,
                },
                maintenant,
            )
            .await?;
            out.push(TracesSujet {
                sujet_type: type_,
                sujet_id: id,
                libelle,
                lignes,
            });
        }
        tx.commit().await?;
        Ok(out)
    }

    /// Consultations faites par un **agent sous contrôle ouvert** (RGP-194) : lignes datées, jamais de total ni de rang.
    pub async fn acces_traces_controle_a(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        controle: Uuid,
        maintenant: DateTime<Utc>,
    ) -> Result<(ControleVue, Vec<TraceLigne>), EntDbError> {
        self.exiger_action_etab(etab, acteur, role, Action::AccesConsulterTraces)
            .await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let l: Option<LigneControle> = sqlx::query_as(
            "SELECT id, agent_compte_id, motif_code, motif_texte, ouvert_le, echeance_le, clos_le, issue
               FROM acc_controle WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(controle)
        .fetch_optional(&mut *tx)
        .await?;
        let l = l.ok_or_else(non_trouve)?;
        let nom = nom_du_compte(&mut tx, etab, l.1).await?;
        let vue = controle_depuis(l, nom, maintenant);
        if !vue.ouvert {
            return Err(conflit(
                "les traces d'un agent ne se consultent que pendant un contrôle ouvert",
            ));
        }
        let reglage = lire_reglage_acces(&mut tx, etab).await?;
        let depuis = limite_traces(maintenant, reglage.trace_mois);
        let lignes = lire_traces(
            &mut tx,
            etab,
            "agent",
            vue.agent_compte_id,
            Some(vue.ouvert_le),
            depuis,
            maintenant,
            None,
        )
        .await?;
        let personne: Option<Uuid> = sqlx::query_scalar(
            "SELECT id FROM ent_personne WHERE etablissement_id = $1 AND user_id = $2 AND fusionnee_dans IS NULL LIMIT 1",
        )
        .bind(etab)
        .bind(vue.agent_compte_id)
        .fetch_optional(&mut *tx)
        .await?;
        ecrire_trace_tx(
            &mut tx,
            etab,
            Uuid::new_v4(),
            acteur,
            role,
            None,
            "acces_traces_lecture",
            &ContexteTrace {
                perimetre: Perimetre::Dans,
                motif: None,
                personne,
            },
            maintenant,
        )
        .await?;
        tx.commit().await?;
        Ok((vue, lignes))
    }

    // ── Différer et communiquer le nom ──

    /// Diffère l'affichage d'une consultation liée à un signalement (RGP-197, R-RG28) : `droits.arbitrer`, motivé, borné
    /// à 90 jours. La consultation doit avoir été faite par un agent sous **contrôle ouvert pour signalement**.
    #[allow(clippy::too_many_arguments)]
    pub async fn acces_differer_a(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        trace: Uuid,
        motif: &str,
        jours: i64,
        maintenant: DateTime<Utc>,
    ) -> Result<DateTime<Utc>, EntDbError> {
        self.exiger_action_etab(etab, acteur, role, Action::DroitsArbitrer)
            .await?;
        if !(5..=500).contains(&motif.trim().chars().count()) {
            return Err(EntDbError::SaisieInvalide(
                "motivez la décision (5 à 500 caractères)",
            ));
        }
        if !(1..=DIFFERE_JOURS_MAX).contains(&jours) {
            return Err(EntDbError::SaisieInvalide(
                "durée du report : de 1 à 90 jours",
            ));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let t: Option<(DateTime<Utc>, Uuid)> = sqlx::query_as(
            "SELECT le, acteur_compte_id FROM acc_trace WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(trace)
        .fetch_optional(&mut *tx)
        .await?;
        let (le, agent) = t.ok_or_else(non_trouve)?;
        let controle: Option<Uuid> = sqlx::query_scalar(
            "SELECT id FROM acc_controle
              WHERE etablissement_id = $1 AND agent_compte_id = $2 AND motif_code = 'signalement'
                AND clos_le IS NULL AND echeance_le > $3 LIMIT 1",
        )
        .bind(etab)
        .bind(agent)
        .bind(maintenant)
        .fetch_optional(&mut *tx)
        .await?;
        let controle = controle.ok_or(conflit(
            "aucun contrôle ouvert pour signalement sur l'agent qui a consulté",
        ))?;
        let jusqu = maintenant + Duration::days(jours);
        sqlx::query(
            "INSERT INTO acc_differe (id, etablissement_id, trace_id, trace_le, controle_id, motif,
                                      decide_par_compte_id, decide_le, jusqu_au)
             VALUES ($1, $2, $3, $4, $5, btrim($6), $7, $8, $9)",
        )
        .bind(Uuid::new_v4())
        .bind(etab)
        .bind(trace)
        .bind(le)
        .bind(controle)
        .bind(motif)
        .bind(acteur)
        .bind(maintenant)
        .bind(jusqu)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(jusqu)
    }

    /// Communique le **nom d'un agent** pour une demande (§11.3, CL27-16) : `droits.arbitrer`, motivé ; sans cet arbitrage,
    /// aucun nom n'est jamais écrit dans l'extrait.
    pub async fn acces_nom_agent_arbitrer_a(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        demande: Uuid,
        trace: Uuid,
        motif: &str,
        maintenant: DateTime<Utc>,
    ) -> Result<(), EntDbError> {
        self.exiger_action_etab(etab, acteur, role, Action::DroitsArbitrer)
            .await?;
        if !(5..=500).contains(&motif.trim().chars().count()) {
            return Err(EntDbError::SaisieInvalide(
                "motivez la décision (5 à 500 caractères)",
            ));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let ouverte: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM rgp_demande WHERE etablissement_id = $1 AND id = $2
                             AND etat NOT IN ('close', 'refusee'))",
        )
        .bind(etab)
        .bind(demande)
        .fetch_one(&mut *tx)
        .await?;
        if !ouverte {
            return Err(conflit("la demande n'est pas ouverte"));
        }
        // La consultation doit concerner un sujet de la demande.
        let lie: bool = sqlx::query_scalar(
            "SELECT EXISTS (
                SELECT 1 FROM acc_trace t
                  JOIN (SELECT sujet_type, sujet_id FROM rgp_demande
                         WHERE etablissement_id = $1 AND id = $3 AND sujet_id IS NOT NULL
                        UNION
                        SELECT sujet_type, sujet_id FROM rgp_demande_sujet
                         WHERE etablissement_id = $1 AND demande_id = $3) s
                    ON ((s.sujet_type = 'eleve' AND t.sujet_eleve_id = s.sujet_id)
                     OR (s.sujet_type <> 'eleve' AND t.sujet_personne_id = s.sujet_id))
                WHERE t.etablissement_id = $1 AND t.id = $2)",
        )
        .bind(etab)
        .bind(trace)
        .bind(demande)
        .fetch_one(&mut *tx)
        .await?;
        if !lie {
            return Err(non_trouve());
        }
        sqlx::query(
            "INSERT INTO acc_nom_communique (id, etablissement_id, demande_id, trace_id, motif, decide_par_compte_id, decide_le)
             VALUES ($1, $2, $3, $4, btrim($5), $6, $7) ON CONFLICT (etablissement_id, demande_id, trace_id) DO NOTHING",
        )
        .bind(Uuid::new_v4())
        .bind(etab)
        .bind(demande)
        .bind(trace)
        .bind(motif)
        .bind(acteur)
        .bind(maintenant)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(())
    }

    // ── Alertes ──

    /// Évalue les règles déterministes d'un établissement sur les 26 dernières heures (RGP-196) ; idempotent : une alerte
    /// par règle, agent et fenêtre. Notifie la direction et le DPO, **jamais** l'agent. Renvoie le nombre d'alertes **nouvelles**.
    pub async fn acces_alertes_evaluer_a(
        &self,
        etab: Uuid,
        maintenant: DateTime<Utc>,
    ) -> Result<usize, EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let r = lire_reglage_acces(&mut tx, etab).await?;
        let depuis = maintenant - Duration::hours(26);
        // (règle, agent, début de fenêtre, fin, nombre)
        let mut trouvees: Vec<(&'static str, Uuid, DateTime<Utc>, DateTime<Utc>, i32)> = Vec::new();

        // 1. Rafale hors périmètre : fenêtres fixes de `rafale_fenetre_min` minutes.
        let fen = i64::from(r.rafale_fenetre_min) * 60;
        let l: Vec<(Uuid, i64, i32)> = sqlx::query_as(
            "SELECT acteur_compte_id, (extract(epoch FROM le)::bigint / $2), count(*)::int
               FROM acc_trace
              WHERE etablissement_id = $1 AND hors_perimetre AND le >= $3 AND le <= $4
              GROUP BY 1, 2 HAVING count(*) >= $5",
        )
        .bind(etab)
        .bind(fen)
        .bind(depuis)
        .bind(maintenant)
        .bind(r.rafale_seuil)
        .fetch_all(&mut *tx)
        .await?;
        for (agent, b, n) in l {
            let debut = Utc.timestamp_opt(b * fen, 0).single().unwrap_or(maintenant);
            trouvees.push((
                "rafale_hors_perimetre",
                agent,
                debut,
                debut + Duration::seconds(fen),
                n,
            ));
        }

        // 2. Seuil journalier par rôle (S-554) : jour de l'établissement.
        let l: Vec<(Uuid, String, NaiveDate, i32)> = sqlx::query_as(
            "SELECT acteur_compte_id, acteur_role, ent_aujourdhui_etab(etablissement_id, le) AS jour, count(*)::int
               FROM acc_trace
              WHERE etablissement_id = $1 AND le >= $2 AND le <= $3
              GROUP BY 1, 2, 3 HAVING count(*) >= 10",
        )
        .bind(etab)
        .bind(depuis)
        .bind(maintenant)
        .fetch_all(&mut *tx)
        .await?;
        let mut par_agent_jour: BTreeMap<(Uuid, NaiveDate), i32> = BTreeMap::new();
        for (agent, role, jour, n) in l {
            let seuil = match role.as_str() {
                "direction" | "admin" => r.seuil_direction,
                "prof" => r.seuil_enseignant,
                "cpe" => r.seuil_vie_scolaire,
                _ => r.seuil_administratif,
            };
            if n > seuil {
                *par_agent_jour.entry((agent, jour)).or_default() += n;
            }
        }
        for ((agent, jour), n) in par_agent_jour {
            let d = debut_de_jour(jour);
            trouvees.push(("seuil_jour", agent, d, d + Duration::days(1), n));
        }

        // 3. Élève à adresse protégée consulté sans lien (hors périmètre et protégé).
        let l: Vec<(Uuid, NaiveDate, i32)> = sqlx::query_as(
            "SELECT acteur_compte_id, ent_aujourdhui_etab(etablissement_id, le), count(*)::int
               FROM acc_trace
              WHERE etablissement_id = $1 AND hors_perimetre AND protegee AND le >= $2 AND le <= $3
              GROUP BY 1, 2",
        )
        .bind(etab)
        .bind(depuis)
        .bind(maintenant)
        .fetch_all(&mut *tx)
        .await?;
        for (agent, jour, n) in l {
            let d = debut_de_jour(jour);
            trouvees.push(("adresse_protegee", agent, d, d + Duration::days(1), n));
        }

        // 4. Élève sorti ou archivé (conservation intermédiaire ou d'archives) consulté.
        let l: Vec<(Uuid, NaiveDate, i32)> = sqlx::query_as(
            "SELECT t.acteur_compte_id, ent_aujourdhui_etab(t.etablissement_id, t.le), count(*)::int
               FROM acc_trace t JOIN ent_eleve e ON e.etablissement_id = t.etablissement_id AND e.id = t.sujet_eleve_id
              WHERE t.etablissement_id = $1 AND t.le >= $2 AND t.le <= $3
                AND (e.statut = 'archive'
                     OR ent_eleve_statut_a(t.etablissement_id, t.sujet_eleve_id, ent_aujourdhui_etab(t.etablissement_id, t.le)) = 'sorti')
              GROUP BY 1, 2",
        )
        .bind(etab)
        .bind(depuis)
        .bind(maintenant)
        .fetch_all(&mut *tx)
        .await?;
        for (agent, jour, n) in l {
            let d = debut_de_jour(jour);
            trouvees.push(("conservation", agent, d, d + Duration::days(1), n));
        }

        let jour = sqlx::query_scalar::<_, NaiveDate>("SELECT ent_aujourdhui_etab($1, $2)")
            .bind(etab)
            .bind(maintenant)
            .fetch_one(&mut *tx)
            .await?;
        let destinataires =
            comptes_avec_action(self, &mut tx, etab, jour, Action::AccesConsulterTraces).await?;
        let mut nouvelles = 0usize;
        for (regle, agent, debut, fin, n) in trouvees {
            let id = Uuid::new_v4();
            let ajoutee = sqlx::query(
                "INSERT INTO acc_alerte (id, etablissement_id, regle, agent_compte_id, fenetre_debut, fenetre_fin, nombre, cree_le)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
                 ON CONFLICT (etablissement_id, regle, agent_compte_id, fenetre_debut) DO NOTHING",
            )
            .bind(id)
            .bind(etab)
            .bind(regle)
            .bind(agent)
            .bind(debut)
            .bind(fin)
            .bind(n)
            .bind(maintenant)
            .execute(&mut *tx)
            .await?
            .rows_affected();
            if ajoutee == 0 {
                continue;
            }
            nouvelles += 1;
            let corps = format!(
                "{} : à examiner dans « Alertes d'accès » (fenêtre du {}).",
                libelle_regle(regle),
                debut.format("%d/%m/%Y %H:%M")
            );
            for u in destinataires.iter().copied().filter(|u| *u != agent) {
                notifier_personne(
                    &mut tx,
                    EnvoiAPlanifier {
                        etablissement_id: etab,
                        user_id: u,
                        kind: KIND_ALERTE,
                        titre: "Alerte d'accès aux dossiers",
                        corps: &corps,
                        eleve_id: None,
                        notification_id: None,
                        source_type: "systeme",
                        source_id: id,
                        cle_dedup: format!("acces:alerte:{id}"),
                        droit_requis: None,
                        regroupable: false,
                        canaux_voulus: &["email", "push"],
                    },
                    id,
                    maintenant,
                )
                .await?;
            }
        }
        tx.commit().await?;
        Ok(nouvelles)
    }

    /// Évalue les alertes de **tous** les établissements (tâche planifiée).
    pub async fn acces_alertes_evaluer_tous_a(
        &self,
        maintenant: DateTime<Utc>,
    ) -> Result<usize, EntDbError> {
        let etabs =
            etablissements_systeme(self.pool(), ContexteSysteme::JobPlanifie("acces_alertes"))
                .await?;
        let mut n = 0;
        for e in etabs {
            n += self.acces_alertes_evaluer_a(e, maintenant).await?;
        }
        Ok(n)
    }

    /// Les alertes (à traiter d'abord, 200 au plus). Aucun classement d'agents.
    pub async fn acces_alertes_lister(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
    ) -> Result<Vec<AlerteVue>, EntDbError> {
        self.exiger_action_etab(etab, acteur, role, Action::AccesConsulterTraces)
            .await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        #[allow(clippy::type_complexity)]
        let l: Vec<(Uuid, String, Uuid, DateTime<Utc>, DateTime<Utc>, i32, DateTime<Utc>, String, Option<DateTime<Utc>>, Option<Uuid>)> =
            sqlx::query_as(
                "SELECT id, regle, agent_compte_id, fenetre_debut, fenetre_fin, nombre, cree_le, suite, suite_le, controle_id
                   FROM acc_alerte WHERE etablissement_id = $1
                  ORDER BY (suite = 'a_traiter') DESC, cree_le DESC, id LIMIT 200",
            )
            .bind(etab)
            .fetch_all(&mut *tx)
            .await?;
        let mut out = Vec::with_capacity(l.len());
        for (id, regle, agent, d, f, n, cree, suite, suite_le, controle) in l {
            let nom = nom_du_compte(&mut tx, etab, agent).await?;
            out.push(AlerteVue {
                id,
                libelle: libelle_regle(&regle).to_string(),
                regle,
                agent_compte_id: agent,
                agent_nom: nom,
                fenetre_debut: d,
                fenetre_fin: f,
                nombre: n,
                cree_le: cree,
                suite,
                suite_le,
                controle_id: controle,
            });
        }
        Ok(out)
    }

    /// Suite donnée à une alerte (`rien`, `violation`, `controle`) : tracée (qui, quand). Pour `controle`, un contrôle
    /// ouvert sur l'agent doit exister (l'ouvrir demande la passkey récente).
    pub async fn acces_alerte_suite_a(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        alerte: Uuid,
        suite: &str,
        maintenant: DateTime<Utc>,
    ) -> Result<(), EntDbError> {
        self.exiger_action_etab(etab, acteur, role, Action::AccesConsulterTraces)
            .await?;
        if !["rien", "violation", "controle"].contains(&suite) {
            return Err(EntDbError::SaisieInvalide("suite d'alerte inconnue"));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let a: Option<(Uuid, String)> = sqlx::query_as(
            "SELECT agent_compte_id, suite FROM acc_alerte WHERE etablissement_id = $1 AND id = $2 FOR UPDATE",
        )
        .bind(etab)
        .bind(alerte)
        .fetch_optional(&mut *tx)
        .await?;
        let (agent, courante) = a.ok_or_else(non_trouve)?;
        if courante != "a_traiter" {
            return Err(conflit("alerte déjà traitée"));
        }
        let controle = if suite == "controle" {
            Some(
                controle_ouvert_de(&mut tx, etab, agent, maintenant)
                    .await?
                    .ok_or(conflit("ouvrez d'abord le contrôle sur cet agent"))?,
            )
        } else {
            None
        };
        sqlx::query(
            "UPDATE acc_alerte SET suite = $3, suite_par_compte_id = $4, suite_le = $5, controle_id = $6
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(alerte)
        .bind(suite)
        .bind(acteur)
        .bind(maintenant)
        .bind(controle)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(())
    }

    // ── Purge par partition ──

    /// Purge les partitions de traces échues (RGP-198, S-541). La durée retenue est la **plus longue** des réglages des
    /// établissements (une partition couvre tous les établissements) ; un plancher de 12 mois est tenu en base. Une
    /// partition qui contient des traces d'un sujet visé par une demande **ouverte**, ou d'un agent sous contrôle ouvert,
    /// est **différée** et les comptes qui portent `acces.consulter_traces` de l'établissement concerné sont alertés.
    pub async fn acces_purger_a(
        &self,
        maintenant: DateTime<Utc>,
    ) -> Result<RapportPurge, EntDbError> {
        self.acces_purger_filtre_a(maintenant, None).await
    }

    /// Comme [`Self::acces_purger_a`], restreinte à des mois donnés (tests : une purge n'emporte pas les partitions que
    /// d'autres tests utilisent en parallèle). `None` : toutes les partitions échues.
    pub(crate) async fn acces_purger_filtre_a(
        &self,
        maintenant: DateTime<Utc>,
        filtre: Option<&[NaiveDate]>,
    ) -> Result<RapportPurge, EntDbError> {
        let etabs =
            etablissements_systeme(self.pool(), ContexteSysteme::JobPlanifie("acces_purge"))
                .await?;
        let mut mois_max = 12;
        for e in &etabs {
            let mut tx = TenantTx::begin(self.pool(), *e).await?;
            mois_max = mois_max.max(lire_reglage_acces(&mut tx, *e).await?.trace_mois);
        }
        let premier = NaiveDate::from_ymd_opt(maintenant.year(), maintenant.month(), 1)
            .ok_or(EntDbError::Domain(DomainError::Infrastructure))?;
        let avant = premier
            .checked_sub_months(Months::new(u32::try_from(mois_max).unwrap_or(12)))
            .ok_or(EntDbError::Domain(DomainError::Infrastructure))?;
        let mut conn = self.pool().acquire().await?;
        let mois: Vec<NaiveDate> =
            sqlx::query_scalar("SELECT public.acc_trace_partitions_anciennes($1)")
                .bind(avant)
                .fetch_all(&mut *conn)
                .await?;
        let mut rapport = RapportPurge::default();
        for m in mois {
            if filtre.is_some_and(|f| !f.contains(&m)) {
                continue;
            }
            let fin = m
                .checked_add_months(Months::new(1))
                .ok_or(EntDbError::Domain(DomainError::Infrastructure))?;
            let (d, f) = (debut_de_jour(m), debut_de_jour(fin));
            let mut lignes: i64 = 0;
            let mut bloquants: Vec<Uuid> = Vec::new();
            for e in &etabs {
                let mut tx = TenantTx::begin(self.pool(), *e).await?;
                let n: i64 = sqlx::query_scalar(
                    "SELECT count(*) FROM acc_trace WHERE etablissement_id = $1 AND le >= $2 AND le < $3",
                )
                .bind(*e)
                .bind(d)
                .bind(f)
                .fetch_one(&mut *tx)
                .await?;
                lignes += n;
                if n == 0 {
                    continue;
                }
                let bloque: bool = sqlx::query_scalar(
                    "SELECT EXISTS (
                        SELECT 1 FROM rgp_demande dm
                          CROSS JOIN LATERAL (
                                SELECT dm.sujet_type, dm.sujet_id WHERE dm.sujet_id IS NOT NULL
                                UNION
                                SELECT s0.sujet_type, s0.sujet_id FROM rgp_demande_sujet s0
                                 WHERE s0.etablissement_id = dm.etablissement_id AND s0.demande_id = dm.id) s
                         WHERE dm.etablissement_id = $1 AND dm.etat NOT IN ('close', 'refusee')
                           AND EXISTS (SELECT 1 FROM acc_trace t
                                        WHERE t.etablissement_id = $1 AND t.le >= $2 AND t.le < $3
                                          AND ((s.sujet_type = 'eleve' AND t.sujet_eleve_id = s.sujet_id)
                                            OR (s.sujet_type <> 'eleve' AND t.sujet_personne_id = s.sujet_id))))
                     OR EXISTS (
                        SELECT 1 FROM acc_controle c
                         WHERE c.etablissement_id = $1 AND c.clos_le IS NULL AND c.echeance_le > $4
                           AND EXISTS (SELECT 1 FROM acc_trace t
                                        WHERE t.etablissement_id = $1 AND t.le >= $2 AND t.le < $3
                                          AND t.acteur_compte_id = c.agent_compte_id))",
                )
                .bind(*e)
                .bind(d)
                .bind(f)
                .bind(maintenant)
                .fetch_one(&mut *tx)
                .await?;
                if bloque {
                    bloquants.push(*e);
                }
            }
            if bloquants.is_empty() {
                let ok: bool =
                    sqlx::query_scalar("SELECT public.acc_trace_purger_partition($1, $2, $3, $4)")
                        .bind(m)
                        .bind(maintenant)
                        .bind(lignes)
                        .bind("échéance de conservation (S-541)")
                        .fetch_one(&mut *conn)
                        .await?;
                if ok {
                    rapport.purgees.push(m);
                }
                continue;
            }
            // Différée : une ligne de registre par semaine et par partition, et l'alerte du DPO de chaque établissement.
            let deja: bool = sqlx::query_scalar(
                "SELECT EXISTS (SELECT 1 FROM acc_purge WHERE partition_mois = $1 AND issue = 'differee' AND le > $2)",
            )
            .bind(m)
            .bind(maintenant - Duration::days(7))
            .fetch_one(&mut *conn)
            .await?;
            if !deja {
                sqlx::query("SELECT public.acc_trace_noter_purge_differee($1, $2, $3, $4)")
                    .bind(m)
                    .bind(maintenant)
                    .bind(lignes)
                    .bind("traces d'un sujet visé par une demande ouverte ou d'un agent sous contrôle")
                    .execute(&mut *conn)
                    .await?;
                for e in &bloquants {
                    let mut tx = TenantTx::begin(self.pool(), *e).await?;
                    let jour =
                        sqlx::query_scalar::<_, NaiveDate>("SELECT ent_aujourdhui_etab($1, $2)")
                            .bind(*e)
                            .bind(maintenant)
                            .fetch_one(&mut *tx)
                            .await?;
                    let dest =
                        comptes_avec_action(self, &mut tx, *e, jour, Action::AccesConsulterTraces)
                            .await?;
                    let corps = format!(
                        "La purge des traces d'accès de {} est reportée : une demande de droit ou un contrôle est ouvert.",
                        m.format("%m/%Y")
                    );
                    let source = Uuid::new_v4();
                    for u in dest {
                        notifier_personne(
                            &mut tx,
                            EnvoiAPlanifier {
                                etablissement_id: *e,
                                user_id: u,
                                kind: KIND_PURGE_DIFFEREE,
                                titre: "Purge des traces d'accès reportée",
                                corps: &corps,
                                eleve_id: None,
                                notification_id: None,
                                source_type: "systeme",
                                source_id: source,
                                cle_dedup: format!(
                                    "acces:purge:{m}:{}",
                                    maintenant.date_naive().iso_week().week()
                                ),
                                droit_requis: None,
                                regroupable: false,
                                canaux_voulus: &["email"],
                            },
                            source,
                            maintenant,
                        )
                        .await?;
                    }
                    tx.commit().await?;
                }
            }
            rapport.differees.push(m);
        }
        Ok(rapport)
    }
}

// ─────────────────────────────── Lecture des lignes ───────────────────────────────

/// Les traces d'un sujet (`eleve`, `personne`) ou d'un agent (`agent` : acteur). Avec `demande`, le nom de l'agent n'est
/// donné que s'il a été communiqué sur arbitrage ; sans, jamais (cadre d'un sujet).
#[allow(clippy::too_many_arguments)]
async fn lire_traces(
    conn: &mut PgConnection,
    etab: Uuid,
    type_: &str,
    sujet: Uuid,
    depuis_controle: Option<DateTime<Utc>>,
    depuis: DateTime<Utc>,
    maintenant: DateTime<Utc>,
    demande: Option<Uuid>,
) -> Result<Vec<TraceLigne>, EntDbError> {
    // Cadre d'un sujet : seules les consultations **affichables** (jamais santé ni psy, R-RG29). Cadre d'un contrôle :
    // toutes les consultations de l'agent, sous leurs seuls libellés génériques, depuis 12 mois au plus.
    let debut = depuis_controle.map_or(depuis, |c| (c - Duration::days(365)).max(depuis));
    let rows = sqlx::query(
        "SELECT t.id, t.le, t.fonction_libelle, t.service_libelle, t.nature, t.section, t.hors_perimetre,
                t.perimetre_temporaire, t.motif_code, t.support_editeur, t.acteur_compte_id,
                COALESCE(btrim(e.prenom || ' ' || e.nom), '') AS eleve_nom,
                EXISTS (SELECT 1 FROM acc_differe d WHERE d.etablissement_id = t.etablissement_id AND d.trace_id = t.id
                         AND d.jusqu_au > $5) AS differee,
                ($6::uuid IS NOT NULL AND EXISTS (SELECT 1 FROM acc_nom_communique n
                         WHERE n.etablissement_id = t.etablissement_id AND n.demande_id = $6 AND n.trace_id = t.id)) AS nom_communique
           FROM acc_trace t
           LEFT JOIN ent_eleve e ON e.etablissement_id = t.etablissement_id AND e.id = t.sujet_eleve_id
          WHERE t.etablissement_id = $1 AND t.le >= $4 AND t.le <= $5
            AND (($2 = 'eleve' AND t.sujet_eleve_id = $3 AND t.affichable)
              OR ($2 = 'personne' AND t.sujet_personne_id = $3 AND t.affichable)
              OR ($2 = 'agent' AND t.acteur_compte_id = $3))
          ORDER BY t.le DESC, t.id
          LIMIT 2000",
    )
    .bind(etab)
    .bind(type_)
    .bind(sujet)
    .bind(debut)
    .bind(maintenant)
    .bind(demande)
    .fetch_all(&mut *conn)
    .await?;
    let motifs: BTreeMap<String, String> = crate::acces_traces::motifs_actifs(&mut *conn, etab)
        .await?
        .into_iter()
        .map(|m| (m.code, m.libelle))
        .collect();
    let mut noms: BTreeMap<Uuid, String> = BTreeMap::new();
    let mut out = Vec::with_capacity(rows.len());
    for r in rows {
        let agent: Uuid = r.get("acteur_compte_id");
        let nom_communique: bool = r.get("nom_communique");
        let agent_nom = if nom_communique {
            if let std::collections::btree_map::Entry::Vacant(libre) = noms.entry(agent) {
                libre.insert(nom_du_compte(&mut *conn, etab, agent).await?);
            }
            noms.get(&agent).cloned()
        } else {
            None
        };
        let support: bool = r.get("support_editeur");
        out.push(TraceLigne {
            id: r.get("id"),
            le: r.get("le"),
            fonction: if support {
                "Support ScolIA".into()
            } else {
                r.get("fonction_libelle")
            },
            service: if support {
                String::new()
            } else {
                r.get("service_libelle")
            },
            nature: libelle_nature(&r.get::<String, _>("nature")).to_string(),
            section: libelle_section(&r.get::<String, _>("section")).to_string(),
            hors_perimetre: r.get("hors_perimetre"),
            perimetre_temporaire: r.get("perimetre_temporaire"),
            motif: r
                .get::<Option<String>, _>("motif_code")
                .map(|c| motifs.get(&c).cloned().unwrap_or(c)),
            sujet: if type_ == "agent" {
                r.get("eleve_nom")
            } else {
                String::new()
            },
            differee: r.get("differee"),
            nom_communique,
            agent_nom,
        });
    }
    Ok(out)
}

// ─────────────────────────────── Section « consultations » de l'extrait ───────────────────────────────

/// Lignes de la section « consultations » de l'extrait d'un sujet (RGP-199) : mêmes règles que la page du lot G4-1
/// (jamais santé ni psy, jamais une consultation **différée**, jamais une trace **protégée**), fonction et service au lieu
/// du nom ; le nom d'un agent n'y figure que sur arbitrage motivé pour **cette** demande. Valeurs triées, déterministes.
pub(crate) async fn lignes_extrait(
    conn: &mut PgConnection,
    etab: Uuid,
    demande: Uuid,
    sujet_type: &str,
    sujet: Uuid,
    maintenant: DateTime<Utc>,
) -> Result<Vec<Vec<(String, String)>>, EntDbError> {
    let reglage = lire_reglage_acces(&mut *conn, etab).await?;
    let depuis = limite_traces(maintenant, reglage.trace_mois);
    let type_ = if sujet_type == "eleve" {
        "eleve"
    } else {
        "personne"
    };
    let lignes = lire_traces(
        &mut *conn,
        etab,
        type_,
        sujet,
        None,
        depuis,
        maintenant,
        Some(demande),
    )
    .await?;
    // Les traces protégées sont écartées de l'extrait : il peut être remis à un parent visé par la protection (R-RG46).
    let protegees: Vec<Uuid> = sqlx::query_scalar(
        "SELECT id FROM acc_trace WHERE etablissement_id = $1 AND protegee
            AND ((($2 = 'eleve') AND sujet_eleve_id = $3) OR (($2 <> 'eleve') AND sujet_personne_id = $3))",
    )
    .bind(etab)
    .bind(type_)
    .bind(sujet)
    .fetch_all(&mut *conn)
    .await?;
    let mut out: Vec<Vec<(String, String)>> = Vec::new();
    for l in lignes {
        if l.differee || protegees.contains(&l.id) {
            continue;
        }
        let mut v = vec![
            (
                "date".to_string(),
                l.le.format("%Y-%m-%dT%H:%M:%SZ").to_string(),
            ),
            ("fonction".to_string(), l.fonction),
            ("nature".to_string(), l.nature),
            ("section".to_string(), l.section),
            ("service".to_string(), l.service),
        ];
        if let Some(m) = l.motif {
            v.push(("motif".to_string(), m));
        }
        if let Some(n) = l.agent_nom {
            v.push(("agent".to_string(), n));
        }
        v.sort_by(|a, b| a.0.cmp(&b.0));
        out.push(v);
    }
    Ok(out)
}
