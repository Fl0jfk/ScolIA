//! Socle 12, lot AS6 : **vérification et file du matin** (`docs/socles/12_ASSIDUITE.md` §9.5, §9.7, §17.6, R-AS4,
//! R-AS25, R-AS35, R-AS37 ; actions AS-151 à AS-156, AS-209, AS-210).
//!
//! - **relance automatique** de l'appel non fait : **une seule fois**, à `début + délai` (le délai de vérification
//!   de la règle d'assiduité datée), à l'intervenant du jour ; **aucune relance** d'une séance annulée, déplacée,
//!   neutralisée, non assurée, d'un jour sans cours, ni d'un appel déjà validé ou repris par la vie scolaire.
//!   L'unicité est garantie par la base (index unique partiel). L'avis à l'intervenant suit le circuit existant
//!   (consommateur `d5.relance_appel`) : le moteur de notifications du socle 8 ne sait pas encore prévenir un
//!   personnel (N12), ce lot n'envoie donc rien lui-même ;
//! - **faire l'appel à la place** de l'intervenant (`appel.faire_tout`) : ouvre l'appel de la séance, **trace**
//!   la substitution (qui, pour quel intervenant, raison codée) et éteint la relance ; l'appel garde son auteur
//!   et son validateur ;
//! - **absence après présence** (CA-02, CA-61, R-N67) : un élève présent à une séance du jour puis absent à une
//!   séance suivante crée une **alerte interne immédiate** à la vie scolaire (jamais une modification
//!   automatique) ; la famille est prévenue par le circuit d'AS5, avec un délai d'au moins 15 minutes, et jamais
//!   si l'élève est localisé entre-temps ;
//! - **élève exclu de cours non arrivé** (R-AS37) : alerte à 10 minutes sans arrivée confirmée à la vie scolaire ;
//! - **« prévenir maintenant »** (CA-50) : décision tracée de la CPE ou de la direction ; l'avis **U3** part par le
//!   **moteur de notifications** (nature `vie_scolaire.absence_inquietante`), sans délai de confirmation, jamais
//!   par un envoi direct ; l'avis de demi-journée ordinaire qui n'était pas encore parti est retiré (la famille ne
//!   reçoit pas deux messages pour le même fait) ;
//! - **file du matin** (§17.6) : le tableau du jour **classé par urgence**, une ligne par élève (ses autres
//!   raisons sont listées), puis les appels non faits et les incohérences.
//!
//! Les événements ne portent que des codes et des identifiants (R-AS25) ; le jour est le jour métier de
//! l'établissement (`ent_aujourdhui_etab`), l'heure de séance est convertie dans le fuseau de l'établissement
//! (`ent_cdt_instant_local`), jamais l'heure UTC du serveur.

use crate::ntf::catalogue::ABSENCE_INQUIETANTE;
use crate::ntf::contrat::ValeurParam;
use crate::ntf::service::{annuler_avis, aviser, Annulation, DemandeAvis, RefusAvis};
use crate::outbox::{Consommateur, EvenementDistribue, FutureTraitement, Publier};
use crate::service::{EntDbError, PgEnt};
use crate::structure_calendrier::{calendrier, Cible};
use crate::tenant_tx::{etablissements_systeme, ContexteSysteme, TenantTx};
use crate::vie_scolaire::{db, introuvable, Seance};
use chrono::{DateTime, Duration, NaiveDate, NaiveTime, Utc};
use ent_auth::Action;
use ent_events::{AggregateKind, MetierEventDraft};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::PgConnection;
use std::collections::{BTreeMap, HashMap, HashSet};
use uuid::Uuid;

/// Événements du lot (codes et identifiants seulement, R-AS25).
pub mod types_as6 {
    pub const ABSENCE_APRES_PRESENCE: &str = "assiduite.absence_apres_presence";
    pub const NON_ARRIVEE: &str = "assiduite.non_arrivee";
    pub const ALERTE_URGENTE: &str = "assiduite.alerte_urgente";
    pub const APPEL_A_LA_PLACE: &str = "appel.a_la_place";
}

pub const CONSOMMATEUR_VERIFICATION: &str = "assiduite.verification";

/// Minutes sans arrivée à la vie scolaire avant l'alerte d'un élève exclu de cours (S-485).
const MINUTES_NON_ARRIVEE: i32 = 10;
/// Durée du verrou « je prends » (AS-210).
const VERROU_MINUTES: i64 = 15;
/// Délai minimal avant l'avis à la famille d'une absence après présence (R-N67).
const DELAI_APRES_PRESENCE_MIN: i64 = 15;

/// Raisons d'une substitution à l'appel (liste fermée, jamais de texte libre).
pub const RAISONS_A_LA_PLACE: [&str; 5] = ["oubli", "intervenant_injoignable", "intervenant_absent", "panne", "autre"];
/// Raisons de « prévenir maintenant » (liste fermée).
pub const RAISONS_PREVENIR: [&str; 5] = [
    "fugue_suspectee",
    "eleve_en_danger",
    "absence_inexpliquee_grave",
    "demande_de_la_famille",
    "autre_urgence",
];
/// Issues qu'une personne peut donner à une alerte (« corrigé » est réservé au système).
pub const CLOTURES_ALERTE: [&str; 6] = [
    "localise",
    "arrive",
    "famille_jointe",
    "famille_deja_informee",
    "sans_suite",
    "fausse_alerte",
];
/// États de séance qui ne sont pas assurés : ni appel attendu, ni relance (socle 10, T2).
const ETATS_NON_ASSURES: [&str; 5] = ["annulee", "deplacee", "neutralisee", "non_assuree", "retiree"];

fn invalide(m: &'static str) -> EntDbError {
    EntDbError::SaisieInvalide(m)
}

// ───────────────────────── DTO ─────────────────────────

/// Bilan d'un passage de la tâche de vérification (un établissement).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BilanVerification {
    pub relances: usize,
    pub non_assurees: usize,
    pub alertes_non_arrivee: usize,
    pub alertes_closes: usize,
}

/// Une ligne de la file du matin.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EntreeFile {
    /// Clé stable (catégorie et identifiant) pour l'écran.
    pub cle: String,
    /// `urgence` · `non_arrivee` · `apres_presence` · `non_jointe` · `sans_nouvelles` · `appel_non_fait` ·
    /// `incoherence`.
    pub categorie: String,
    /// Autres raisons de la même personne (une ligne par élève, jamais deux lignes pour le même élève).
    pub raisons: Vec<String>,
    /// 1 (le plus urgent) à 9 : voir [`niveau_de`].
    pub niveau: u8,
    /// Position dans la file (1, 2, 3…).
    pub rang: u32,
    pub eleve_id: Option<Uuid>,
    pub nom: Option<String>,
    pub prenom: Option<String>,
    pub classe: Option<String>,
    pub mineur: Option<bool>,
    /// Drapeau de vigilance, sans motif.
    pub vigilance: bool,
    /// Heure locale du premier constat.
    pub depuis: Option<NaiveTime>,
    pub minutes_depuis: Option<i64>,
    pub nb_seances: Option<i64>,
    pub absence_id: Option<Uuid>,
    pub alerte_id: Option<Uuid>,
    pub creneau_id: Option<Uuid>,
    pub classe_seance: Option<String>,
    pub intervenant_user_id: Option<Uuid>,
    /// Code court sans texte libre (`non_fait`, `non_clos`, `absent_puis_present`…).
    pub detail_code: Option<String>,
    pub contacts_joints: i64,
    pub contacts_sans_succes: i64,
    /// Verrou « je prends » encore valide.
    pub pris_par: Option<Uuid>,
    pub pris_jusqua: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FileDuMatin {
    pub jour: NaiveDate,
    pub heure_locale: NaiveTime,
    pub entrees: Vec<EntreeFile>,
    /// Nombre d'entrées par catégorie (onglets de l'écran).
    pub compteurs: BTreeMap<String, i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AlerteAsd {
    pub id: Uuid,
    pub eleve_id: Uuid,
    pub kind: String,
    pub jour: NaiveDate,
    pub statut: String,
    pub pris_par: Option<Uuid>,
    pub pris_jusqua: Option<DateTime<Utc>>,
    pub cloture_code: Option<String>,
    pub cree_le: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResultatALaPlace {
    pub appel_id: Uuid,
    /// Vrai si la personne est elle-même l'intervenant (aucune substitution à tracer).
    pub intervenant_lui_meme: bool,
    pub intervenant_user_id: Option<Uuid>,
    pub deja_ouvert: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrevenirMaintenant {
    pub eleve_id: Uuid,
    pub raison_code: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResultatPrevenir {
    pub id: Uuid,
    pub avis_id: Uuid,
    /// Déjà fait aujourd'hui pour cet élève : rien n'est envoyé une seconde fois.
    pub deja_fait: bool,
    pub destinataires: usize,
    /// Aucun responsable joignable par le moteur : la vie scolaire appelle (la ligne reste dans la file).
    pub sans_destinataire: bool,
    pub avis_retire: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VigilanceAsd {
    pub id: Uuid,
    pub eleve_id: Uuid,
    pub nom: String,
    pub prenom: String,
    pub pose_par: Uuid,
    pub pose_le: DateTime<Utc>,
}

// ───────────────────────── Classement ─────────────────────────

/// Niveau d'urgence d'une ligne de la file (1 = le plus urgent). **Hypothèse de lecture du §17.6** (« non jointes,
/// mineurs sans nouvelles, élèves signalés fragiles, absences après présence » ne fixe pas d'ordre) : ce qui met un
/// élève en danger passe avant ce qui demande un geste administratif.
///
/// 1 prévenir maintenant sans contact joint · 2 exclu de cours non arrivé · 3 absence après présence · 4 élève sous
/// vigilance sans nouvelles · 5 famille tentée sans succès · 6 mineur sans nouvelles · 7 majeur sans nouvelles ·
/// 8 appel non fait · 9 incohérence.
pub fn niveau_de(categorie: &str, vigilance: bool, mineur: bool) -> u8 {
    match categorie {
        "urgence" => 1,
        "non_arrivee" => 2,
        "apres_presence" => 3,
        "non_jointe" | "sans_nouvelles" if vigilance => 4,
        "non_jointe" => 5,
        "sans_nouvelles" if mineur => 6,
        "sans_nouvelles" => 7,
        "appel_non_fait" => 8,
        _ => 9,
    }
}

// ───────────────────────── Aides SQL ─────────────────────────

async fn emettre(
    conn: &mut PgConnection,
    etab: Uuid,
    type_name: &str,
    aggregate: AggregateKind,
    aggregate_id: Uuid,
    eleve_id: Option<Uuid>,
    payload: serde_json::Value,
    acteur: Option<Uuid>,
) -> Result<(), EntDbError> {
    conn.publier(MetierEventDraft {
        etablissement_id: etab,
        type_name: type_name.to_string(),
        aggregate,
        aggregate_id,
        eleve_id,
        payload,
        actor_user_id: acteur.map(|u| u.to_string()),
    })
    .await?;
    Ok(())
}

async fn jour_etab(conn: &mut PgConnection, etab: Uuid, instant: DateTime<Utc>) -> Result<NaiveDate, EntDbError> {
    sqlx::query_scalar("SELECT ent_aujourdhui_etab($1, $2)")
        .bind(etab)
        .bind(instant)
        .fetch_one(&mut *conn)
        .await
        .map_err(db)
}

async fn instant_local(
    conn: &mut PgConnection,
    etab: Uuid,
    jour: NaiveDate,
    heure: NaiveTime,
) -> Result<DateTime<Utc>, EntDbError> {
    sqlx::query_scalar("SELECT ent_cdt_instant_local($1, $2, $3)")
        .bind(etab)
        .bind(jour)
        .bind(heure)
        .fetch_one(&mut *conn)
        .await
        .map_err(db)
}

async fn heure_locale(conn: &mut PgConnection, etab: Uuid, instant: DateTime<Utc>) -> Result<NaiveTime, EntDbError> {
    sqlx::query_scalar("SELECT ent_cdt_heure_locale($1, $2)")
        .bind(etab)
        .bind(instant)
        .fetch_one(&mut *conn)
        .await
        .map_err(db)
}

/// Le jour est-il un jour de cours de l'établissement ? (Calendrier du socle 5 ; sans année scolaire définie, tout
/// jour est ouvert : le repli reste l'emploi du temps.)
async fn jour_de_cours(conn: &mut PgConnection, etab: Uuid, jour: NaiveDate) -> Result<bool, EntDbError> {
    let annees: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM ent_annee_scolaire WHERE etablissement_id = $1)")
        .bind(etab)
        .fetch_one(&mut *conn)
        .await
        .map_err(db)?;
    if !annees {
        return Ok(true);
    }
    let cible = Cible {
        site_id: None,
        niveau_id: None,
        classe_id: None,
        groupe_id: None,
    };
    Ok(!calendrier::jours_ouvres(conn, etab, jour, jour, &cible).await?.is_empty())
}

/// La séance est-elle assurée ? Un état de séance du socle 10 qui n'occupe pas (annulée, déplacée, neutralisée,
/// non assurée, retirée) n'attend ni appel ni relance.
async fn seance_assuree(conn: &mut PgConnection, etab: Uuid, creneau: Uuid, jour: NaiveDate) -> Result<bool, EntDbError> {
    let etat: Option<String> = sqlx::query_scalar(
        "SELECT etat FROM ent_edt_seance WHERE etablissement_id = $1 AND creneau_id = $2 AND jour = $3",
    )
    .bind(etab)
    .bind(creneau)
    .bind(jour)
    .fetch_optional(&mut *conn)
    .await
    .map_err(db)?;
    Ok(etat.is_none_or(|e| !ETATS_NON_ASSURES.contains(&e.as_str())))
}

/// Un élève présent (ou en retard) à une séance **antérieure** du même jour, dans un appel validé et non neutralisé ?
/// `sauf_appel` écarte l'appel qui vient de constater l'absence.
pub(crate) async fn etait_present_plus_tot(
    conn: &mut PgConnection,
    etab: Uuid,
    eleve: Uuid,
    jour: NaiveDate,
    avant: NaiveTime,
    sauf_appel: Option<Uuid>,
) -> Result<bool, EntDbError> {
    sqlx::query_scalar(
        "SELECT EXISTS (
            SELECT 1
              FROM ent_appel a
              JOIN ent_appel_ligne l ON l.etablissement_id = a.etablissement_id AND l.appel_id = a.id
              LEFT JOIN ent_edt_creneau c ON c.etablissement_id = a.etablissement_id AND c.id = a.creneau_id
             WHERE a.etablissement_id = $1 AND a.jour = $2 AND a.statut = 'clos' AND a.etat <> 'neutralise'
               AND l.eleve_id = $3 AND l.etat IN ('present', 'retard', 'present_dispense')
               AND COALESCE(a.heure_debut, c.heure_debut) < $4
               AND ($5::uuid IS NULL OR a.id <> $5))",
    )
    .bind(etab)
    .bind(jour)
    .bind(eleve)
    .bind(avant)
    .bind(sauf_appel)
    .fetch_one(&mut *conn)
    .await
    .map_err(db)
}

/// Délai de vérification (minutes) de la règle d'assiduité en vigueur ce jour-là (S-470, 15 par défaut).
async fn delai_verification(conn: &mut PgConnection, etab: Uuid, jour: NaiveDate) -> Result<i64, EntDbError> {
    let d: Option<i32> = sqlx::query_scalar(
        "SELECT delai_verification_min FROM ent_asd_regle
          WHERE etablissement_id = $1 AND valide_du <= $2 ORDER BY valide_du DESC LIMIT 1",
    )
    .bind(etab)
    .bind(jour)
    .fetch_optional(&mut *conn)
    .await
    .map_err(db)?;
    Ok(i64::from(d.unwrap_or(15).clamp(1, 240)))
}

/// Séance attendue dont l'appel n'est pas validé.
struct SeanceSansAppel {
    seance: Seance,
    appel_id: Option<Uuid>,
    /// Statut de l'appel ouvert (`ouvert`) ; `None` si aucun appel.
    substitution: bool,
    relancee: bool,
}

#[derive(sqlx::FromRow)]
struct AppelRow {
    id: Uuid,
    statut: String,
    etat: String,
}

impl PgEnt {
    /// Séances **assurées** du jour dont l'appel n'est pas validé (aucun appel, ou appel ouvert), avec l'état de leur
    /// substitution et de leur relance. Une séance annulée, déplacée, neutralisée ou non assurée n'en fait pas partie.
    async fn seances_sans_appel_valide(
        &self,
        etab: Uuid,
        jour: NaiveDate,
    ) -> Result<(Vec<SeanceSansAppel>, usize), EntDbError> {
        let seances = self.seances_du_jour(etab, jour).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let mut out = Vec::new();
        let mut non_assurees = 0;
        for s in seances {
            if !seance_assuree(&mut tx, etab, s.seance_ref, jour).await? {
                non_assurees += 1;
                continue;
            }
            let appel: Option<AppelRow> = sqlx::query_as(
                "SELECT id, statut, etat FROM ent_appel WHERE etablissement_id = $1 AND creneau_id = $2 AND jour = $3",
            )
            .bind(etab)
            .bind(s.seance_ref)
            .bind(jour)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db)?;
            if let Some(a) = &appel {
                // Appel validé ou neutralisé : rien n'est attendu.
                if a.statut != "ouvert" || a.etat == "neutralise" {
                    continue;
                }
            }
            let substitution: bool = match &appel {
                Some(a) => sqlx::query_scalar(
                    "SELECT EXISTS (SELECT 1 FROM ent_asd_appel_substitution WHERE etablissement_id = $1 AND appel_id = $2)",
                )
                .bind(etab)
                .bind(a.id)
                .fetch_one(&mut *tx)
                .await
                .map_err(db)?,
                None => false,
            };
            let relancee: bool = sqlx::query_scalar(
                "SELECT EXISTS (SELECT 1 FROM ent_appel_relance WHERE etablissement_id = $1 AND creneau_id = $2 AND jour = $3)",
            )
            .bind(etab)
            .bind(s.seance_ref)
            .bind(jour)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
            out.push(SeanceSansAppel {
                seance: s,
                appel_id: appel.map(|a| a.id),
                substitution,
                relancee,
            });
        }
        Ok((out, non_assurees))
    }

    // ───────────────────────── Relance automatique et alertes de temps ─────────────────────────

    /// Passage de vérification d'un établissement à l'instant `maintenant` : relance **automatique** des appels non
    /// faits (une seule fois par séance et par jour, à `début + délai`) puis alertes des élèves exclus de cours non
    /// arrivés. Idempotent : un second passage ne relance ni n'alerte de nouveau.
    pub async fn verifier_jour_systeme(
        &self,
        etab: Uuid,
        maintenant: DateTime<Utc>,
    ) -> Result<BilanVerification, EntDbError> {
        let mut bilan = BilanVerification::default();
        let jour = {
            let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
            jour_etab(&mut tx, etab, maintenant).await?
        };
        let ouvert = {
            let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
            jour_de_cours(&mut tx, etab, jour).await?
        };
        if ouvert {
            let (attendues, non_assurees) = self.seances_sans_appel_valide(etab, jour).await?;
            bilan.non_assurees = non_assurees;
            let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
            let delai = delai_verification(&mut tx, etab, jour).await?;
            for a in attendues {
                // Relance éteinte : appel repris par la vie scolaire, ou déjà relancé (une seule fois).
                if a.substitution || a.relancee {
                    continue;
                }
                let debut = instant_local(&mut tx, etab, jour, a.seance.heure_debut).await?;
                if maintenant < debut + Duration::minutes(delai) {
                    continue;
                }
                let id = Uuid::new_v4();
                let inseree = sqlx::query(
                    "INSERT INTO ent_appel_relance
                        (id, etablissement_id, creneau_id, jour, destinataire_user_id, envoyee_par_user_id, origine)
                     VALUES ($1, $2, $3, $4, $5, NULL, 'automatique')
                     ON CONFLICT (etablissement_id, creneau_id, jour) WHERE origine = 'automatique' DO NOTHING",
                )
                .bind(id)
                .bind(etab)
                .bind(a.seance.seance_ref)
                .bind(jour)
                .bind(a.seance.prof_user_id)
                .execute(&mut *tx)
                .await
                .map_err(db)?;
                if inseree.rows_affected() == 0 {
                    continue;
                }
                emettre(
                    &mut tx,
                    etab,
                    crate::vie_scolaire::types_vs::APPEL_RELANCE,
                    AggregateKind::Appel,
                    id,
                    None,
                    json!({
                        "relance_id": id,
                        "creneau_id": a.seance.seance_ref,
                        "jour": jour.to_string(),
                        "destinataire_user_id": a.seance.prof_user_id,
                        "origine": "automatique",
                    }),
                    None,
                )
                .await?;
                bilan.relances += 1;
            }
            tx.commit().await.map_err(db)?;
        }
        self.alertes_non_arrivee_systeme(etab, jour, maintenant, &mut bilan).await?;
        Ok(bilan)
    }

    /// Élèves exclus ponctuellement de cours sans arrivée confirmée à la vie scolaire (R-AS37) : alerte à
    /// [`MINUTES_NON_ARRIVEE`] minutes ; clôture de l'alerte quand l'arrivée est confirmée.
    async fn alertes_non_arrivee_systeme(
        &self,
        etab: Uuid,
        jour: NaiveDate,
        maintenant: DateTime<Utc>,
        bilan: &mut BilanVerification,
    ) -> Result<(), EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let lignes: Vec<(Uuid, Uuid, Uuid, Option<Uuid>)> = sqlx::query_as(
            "SELECT l.id, l.appel_id, l.eleve_id, a.creneau_id
               FROM ent_appel_ligne l
               JOIN ent_appel a ON a.etablissement_id = l.etablissement_id AND a.id = l.appel_id
              WHERE l.etablissement_id = $1 AND a.jour = $2 AND a.etat <> 'neutralise'
                AND l.exclusion_cours_le IS NOT NULL AND l.arrivee_vs_confirmee_le IS NULL
                AND l.exclusion_cours_le <= $3 - make_interval(mins => $4)
              ORDER BY l.exclusion_cours_le, l.id",
        )
        .bind(etab)
        .bind(jour)
        .bind(maintenant)
        .bind(MINUTES_NON_ARRIVEE)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        for (ligne, appel, eleve, creneau) in lignes {
            let id = Uuid::new_v4();
            let cree: Option<(Uuid,)> = sqlx::query_as(
                "INSERT INTO ent_asd_alerte (id, etablissement_id, eleve_id, kind, jour, seance_ref, appel_id, cle)
                 VALUES ($1, $2, $3, 'non_arrivee', $4, $5, $6, $7)
                 ON CONFLICT (etablissement_id, kind, cle) DO NOTHING
                 RETURNING id",
            )
            .bind(id)
            .bind(etab)
            .bind(eleve)
            .bind(jour)
            .bind(creneau)
            .bind(appel)
            .bind(ligne.to_string())
            .fetch_optional(&mut *tx)
            .await
            .map_err(db)?;
            if cree.is_some() {
                emettre(
                    &mut tx,
                    etab,
                    types_as6::NON_ARRIVEE,
                    AggregateKind::Appel,
                    appel,
                    Some(eleve),
                    json!({ "alerte_id": id, "appel_id": appel, "jour": jour.to_string() }),
                    None,
                )
                .await?;
                bilan.alertes_non_arrivee += 1;
            }
        }
        let closes = sqlx::query(
            "UPDATE ent_asd_alerte al
                SET statut = 'traitee', traite_le = now(), cloture_code = 'arrive', cloture_par_systeme = true,
                    modifie_le = now()
              WHERE al.etablissement_id = $1 AND al.kind = 'non_arrivee' AND al.statut <> 'traitee'
                AND EXISTS (SELECT 1 FROM ent_appel_ligne l
                             WHERE l.etablissement_id = al.etablissement_id AND l.id::text = al.cle
                               AND l.arrivee_vs_confirmee_le IS NOT NULL)",
        )
        .bind(etab)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        bilan.alertes_closes += closes.rows_affected() as usize;
        tx.commit().await.map_err(db)?;
        Ok(())
    }

    /// Tâche système (chaque minute) : vérification de chaque établissement à son propre jour métier. Un
    /// établissement en erreur ne bloque pas les suivants (R-09).
    pub async fn assiduite_verification_tache(&self) -> Result<usize, EntDbError> {
        let mut total = 0;
        for etab in etablissements_systeme(self.pool(), ContexteSysteme::JobPlanifie("assiduite_verification"))
            .await
            .map_err(db)?
        {
            match self.verifier_jour_systeme(etab, Utc::now()).await {
                Ok(b) => total += b.relances + b.alertes_non_arrivee,
                Err(e) => tracing::warn!(error = %e, "vérification d'assiduité non exécutée pour un établissement"),
            }
        }
        Ok(total)
    }

    // ───────────────────────── Faire l'appel à la place ─────────────────────────

    /// Ouvre l'appel d'une séance **à la place de l'intervenant** (AS-152, CA-41) : droit `appel.faire_tout`,
    /// séance assurée, jour courant ou veille. La substitution est **tracée** (auteur, intervenant prévu, raison
    /// codée) et éteint la relance ; l'appel garde son auteur et son validateur. L'appel se pointe et se valide
    /// ensuite par les routes de l'appel v2. Idempotent.
    pub async fn appel_a_la_place_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        creneau_id: Uuid,
        jour: NaiveDate,
        raison_code: &str,
    ) -> Result<ResultatALaPlace, EntDbError> {
        let droits = self.droits_appel(etab, acteur, role).await?;
        if !droits.tout {
            return Err(EntDbError::Forbidden);
        }
        let raison = RAISONS_A_LA_PLACE
            .iter()
            .find(|r| **r == raison_code)
            .copied()
            .ok_or_else(|| invalide("raison de substitution inconnue"))?;
        let aujourdhui = self.aujourdhui_etablissement(etab).await?;
        if jour > aujourdhui || jour < aujourdhui - Duration::days(1) {
            return Err(invalide("l'appel à la place se fait le jour même ou la veille"));
        }
        let seance = self
            .seances_du_jour(etab, jour)
            .await?
            .into_iter()
            .find(|s| s.seance_ref == creneau_id)
            .ok_or_else(introuvable)?;
        {
            let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
            if !seance_assuree(&mut tx, etab, creneau_id, jour).await? {
                return Err(EntDbError::ConflitDossier("séance non assurée : aucun appel n'est attendu"));
            }
            let existant: Option<AppelRow> = sqlx::query_as(
                "SELECT id, statut, etat FROM ent_appel WHERE etablissement_id = $1 AND creneau_id = $2 AND jour = $3",
            )
            .bind(etab)
            .bind(creneau_id)
            .bind(jour)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db)?;
            if let Some(a) = &existant {
                if a.etat == "neutralise" {
                    return Err(EntDbError::ConflitDossier("appel neutralisé : la séance n'a pas eu lieu"));
                }
                if a.statut != "ouvert" {
                    return Err(EntDbError::ConflitDossier("l'appel de cette séance est déjà validé"));
                }
            }
        }
        let (appel, _) = self.ouvrir_appel(etab, creneau_id, jour, acteur, role).await?;
        let lui_meme = seance.prof_user_id == acteur;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let mut deja_ouvert = true;
        if !lui_meme {
            let id = Uuid::new_v4();
            let inseree = sqlx::query(
                "INSERT INTO ent_asd_appel_substitution
                    (id, etablissement_id, appel_id, creneau_id, jour, auteur_user_id, intervenant_user_id, raison_code)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
                 ON CONFLICT (etablissement_id, appel_id) DO NOTHING",
            )
            .bind(id)
            .bind(etab)
            .bind(appel.id)
            .bind(creneau_id)
            .bind(jour)
            .bind(acteur)
            .bind(seance.prof_user_id)
            .bind(raison)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
            if inseree.rows_affected() == 1 {
                deja_ouvert = false;
                emettre(
                    &mut tx,
                    etab,
                    types_as6::APPEL_A_LA_PLACE,
                    AggregateKind::Appel,
                    appel.id,
                    None,
                    json!({
                        "appel_id": appel.id,
                        "creneau_id": creneau_id,
                        "jour": jour.to_string(),
                        "raison_code": raison,
                    }),
                    Some(acteur),
                )
                .await?;
            }
        }
        tx.commit().await.map_err(db)?;
        Ok(ResultatALaPlace {
            appel_id: appel.id,
            intervenant_lui_meme: lui_meme,
            intervenant_user_id: Some(seance.prof_user_id),
            deja_ouvert,
        })
    }

    // ───────────────────────── Prévenir maintenant ─────────────────────────

    /// « Prévenir maintenant » (AS-155, CA-50) : la CPE ou la direction (`assiduite.alerter`) alerte la famille d'une
    /// absence inquiétante. L'avis est émis par le **moteur de notifications** (U3, sans délai de confirmation) ; la
    /// décision est tracée (auteur, raison codée, avis). Une absence constatée aujourd'hui est exigée. L'avis de
    /// demi-journée ordinaire qui n'est pas encore parti est retiré. Idempotent pour un élève et un jour.
    pub async fn prevenir_maintenant_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        req: PrevenirMaintenant,
    ) -> Result<ResultatPrevenir, EntDbError> {
        self.exiger_action_etab(etab, acteur, role, Action::AssiduiteAlerter).await?;
        let raison = RAISONS_PREVENIR
            .iter()
            .find(|r| **r == req.raison_code)
            .copied()
            .ok_or_else(|| invalide("raison d'alerte inconnue"))?;
        let maintenant = Utc::now();
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        // Même verrou que le consommateur d'avis : jamais deux décisions concurrentes pour un élève.
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1, 0))")
            .bind(format!("asd-familles:{etab}:{}", req.eleve_id))
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        let existe: Option<i32> = sqlx::query_scalar("SELECT 1 FROM ent_eleve WHERE etablissement_id = $1 AND id = $2")
            .bind(etab)
            .bind(req.eleve_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db)?;
        if existe.is_none() {
            return Err(introuvable());
        }
        let jour = jour_etab(&mut tx, etab, maintenant).await?;
        let deja: Option<(Uuid, Uuid)> = sqlx::query_as(
            "SELECT id, avis_id FROM ent_asd_prevenir WHERE etablissement_id = $1 AND eleve_id = $2 AND jour = $3
              ORDER BY cree_le LIMIT 1",
        )
        .bind(etab)
        .bind(req.eleve_id)
        .bind(jour)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        if let Some((id, avis_id)) = deja {
            return Ok(ResultatPrevenir {
                id,
                avis_id,
                deja_fait: true,
                destinataires: 0,
                sans_destinataire: false,
                avis_retire: false,
            });
        }
        // Il faut un fait : une absence constatée aujourd'hui, ou un élève exclu de cours non arrivé.
        let fait: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM ent_asd_absence_seance s
                             WHERE s.etablissement_id = $1 AND s.eleve_id = $2 AND s.jour = $3 AND s.retiree_le IS NULL)
                 OR EXISTS (SELECT 1 FROM ent_asd_alerte al
                             WHERE al.etablissement_id = $1 AND al.eleve_id = $2 AND al.jour = $3
                               AND al.statut <> 'traitee')",
        )
        .bind(etab)
        .bind(req.eleve_id)
        .bind(jour)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        if !fait {
            return Err(EntDbError::ConflitDossier("aucune absence constatée aujourd'hui pour cet élève"));
        }
        // Un avis ordinaire encore planifié est retiré : la famille ne reçoit pas deux messages pour le même fait.
        let ordinaires: Vec<(Uuid, Uuid)> = sqlx::query_as(
            "SELECT id, avis_id FROM ent_asd_avis_demi_journee
              WHERE etablissement_id = $1 AND eleve_id = $2 AND jour = $3 AND statut = 'programme'
              ORDER BY periode, generation FOR UPDATE",
        )
        .bind(etab)
        .bind(req.eleve_id)
        .bind(jour)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let mut avis_retire = false;
        let mut avis_remplace: Option<Uuid> = None;
        for (ligne, avis) in ordinaires {
            if let Annulation::Annule = annuler_avis(&mut tx, etab, avis, "prevenir_maintenant", maintenant)
                .await
                .map_err(db)?
            {
                crate::assiduite_as5::marquer(&mut tx, etab, ligne, "annule", "urgence", None).await?;
                avis_retire = true;
                avis_remplace.get_or_insert(avis);
            }
        }
        let id = Uuid::new_v4();
        let cree = match aviser(
            &mut tx,
            DemandeAvis {
                etablissement_id: etab,
                nature: &ABSENCE_INQUIETANTE,
                eleve_id: Some(req.eleve_id),
                cle: format!("asd:urgent:{}:{jour}", req.eleve_id),
                parametres: vec![("jour", ValeurParam::Jour(jour))],
                emetteur_brique: Some("vie_scolaire"),
                source_type: "assiduite_prevenir".into(),
                source_id: Some(id),
                cause_event_id: None,
                effet_le: maintenant,
                urgence_plus: None,
                // Sans délai de confirmation : c'est le sens de « maintenant ».
                delai_min: Some(0),
            },
        )
        .await
        {
            Ok(c) => c,
            Err(RefusAvis::SujetGele) => {
                return Err(EntDbError::ConflitDossier("élève décédé : aucun avis n'est émis"))
            }
            Err(RefusAvis::NatureDesactivee) => {
                return Err(EntDbError::ConflitDossier("l'établissement a désactivé cette alerte"))
            }
            Err(RefusAvis::Base(e)) => return Err(db(e)),
            Err(_) => return Err(invalide("avis urgent refusé par le moteur de notifications")),
        };
        sqlx::query(
            "INSERT INTO ent_asd_prevenir
                (id, etablissement_id, eleve_id, jour, raison_code, auteur_user_id, avis_id, avis_remplace_id)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
        )
        .bind(id)
        .bind(etab)
        .bind(req.eleve_id)
        .bind(jour)
        .bind(raison)
        .bind(acteur)
        .bind(cree.avis_id)
        .bind(avis_remplace)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        emettre(
            &mut tx,
            etab,
            types_as6::ALERTE_URGENTE,
            AggregateKind::Absence,
            id,
            Some(req.eleve_id),
            json!({
                "prevenir_id": id,
                "avis_id": cree.avis_id,
                "jour": jour.to_string(),
                "raison_code": raison,
            }),
            Some(acteur),
        )
        .await?;
        tx.commit().await.map_err(db)?;
        Ok(ResultatPrevenir {
            id,
            avis_id: cree.avis_id,
            deja_fait: false,
            destinataires: cree.retenus.len(),
            sans_destinataire: cree.retenus.is_empty(),
            avis_retire,
        })
    }

    // ───────────────────────── Alertes : « je prends », traitement ─────────────────────────

    /// « Je prends » (AS-210) : verrou visible de 15 minutes sur une alerte. Refusé si un autre l'a prise et que son
    /// verrou court encore ; la même personne le prolonge. Droit `assiduite.suivre`.
    pub async fn alerte_prendre_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        alerte_id: Uuid,
    ) -> Result<AlerteAsd, EntDbError> {
        self.exiger_action_etab(etab, acteur, role, Action::AssiduiteSuivre).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let a: Option<(String, Option<Uuid>, Option<DateTime<Utc>>)> = sqlx::query_as(
            "SELECT statut, pris_par, pris_jusqua FROM ent_asd_alerte WHERE etablissement_id = $1 AND id = $2 FOR UPDATE",
        )
        .bind(etab)
        .bind(alerte_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let (statut, pris_par, pris_jusqua) = a.ok_or_else(introuvable)?;
        if statut == "traitee" {
            return Err(EntDbError::ConflitDossier("alerte déjà traitée"));
        }
        let maintenant = Utc::now();
        if let (Some(par), Some(jusqua)) = (pris_par, pris_jusqua) {
            if par != acteur && jusqua > maintenant {
                return Err(EntDbError::ConflitDossier("alerte déjà prise en charge par quelqu'un d'autre"));
            }
        }
        sqlx::query(
            "UPDATE ent_asd_alerte SET statut = 'prise', pris_par = $3, pris_le = $4,
                    pris_jusqua = $4 + make_interval(mins => $5), modifie_le = now()
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(alerte_id)
        .bind(acteur)
        .bind(maintenant)
        .bind(VERROU_MINUTES as i32)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        let r = lire_alerte(&mut tx, etab, alerte_id).await?;
        tx.commit().await.map_err(db)?;
        Ok(r)
    }

    /// Traite une alerte (issue de la liste fermée, sauf « corrigé » réservée au système). Droit `assiduite.suivre`.
    pub async fn alerte_traiter_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        alerte_id: Uuid,
        cloture_code: &str,
    ) -> Result<AlerteAsd, EntDbError> {
        self.exiger_action_etab(etab, acteur, role, Action::AssiduiteSuivre).await?;
        let code = CLOTURES_ALERTE
            .iter()
            .find(|c| **c == cloture_code)
            .copied()
            .ok_or_else(|| invalide("issue d'alerte inconnue"))?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let r = sqlx::query(
            "UPDATE ent_asd_alerte SET statut = 'traitee', traite_par = $3, traite_le = now(), cloture_code = $4,
                    modifie_le = now()
              WHERE etablissement_id = $1 AND id = $2 AND statut <> 'traitee'",
        )
        .bind(etab)
        .bind(alerte_id)
        .bind(acteur)
        .bind(code)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        if r.rows_affected() == 0 {
            // Inexistante, d'un autre établissement ou déjà traitée : même réponse (R-AS35).
            return Err(introuvable());
        }
        let r = lire_alerte(&mut tx, etab, alerte_id).await?;
        tx.commit().await.map_err(db)?;
        Ok(r)
    }

    /// Alertes à prendre en charge d'un jour (ouvertes ou prises). Droit `assiduite.suivre`.
    pub async fn alertes_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        jour: NaiveDate,
    ) -> Result<Vec<AlerteAsd>, EntDbError> {
        self.exiger_action_etab(etab, acteur, role, Action::AssiduiteSuivre).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        sqlx::query_as::<_, AlerteRow>(
            "SELECT id, eleve_id, kind, jour, statut, pris_par, pris_jusqua, cloture_code, cree_le
               FROM ent_asd_alerte WHERE etablissement_id = $1 AND jour = $2 AND statut <> 'traitee'
              ORDER BY cree_le, id",
        )
        .bind(etab)
        .bind(jour)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)
        .map(|v| v.into_iter().map(AlerteAsd::from).collect())
    }

    // ───────────────────────── Drapeau de vigilance ─────────────────────────

    /// Pose le drapeau « à surveiller » sur un élève : **sans motif** (aucune donnée de santé ni de situation
    /// familiale) ; il classe plus haut une absence non expliquée. Droit `absences.qualifier` (CPE, direction).
    /// Idempotent.
    pub async fn vigilance_poser_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        eleve_id: Uuid,
    ) -> Result<(), EntDbError> {
        self.exiger_action_etab(etab, acteur, role, Action::AbsencesQualifier).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let existe: Option<i32> = sqlx::query_scalar("SELECT 1 FROM ent_eleve WHERE etablissement_id = $1 AND id = $2")
            .bind(etab)
            .bind(eleve_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db)?;
        if existe.is_none() {
            return Err(introuvable());
        }
        sqlx::query(
            "INSERT INTO ent_asd_vigilance (id, etablissement_id, eleve_id, pose_par) VALUES ($1, $2, $3, $4)
             ON CONFLICT (etablissement_id, eleve_id) WHERE leve_le IS NULL DO NOTHING",
        )
        .bind(Uuid::new_v4())
        .bind(etab)
        .bind(eleve_id)
        .bind(acteur)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        Ok(())
    }

    /// Lève le drapeau de vigilance d'un élève (jamais supprimé : levé, historique conservé).
    pub async fn vigilance_lever_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        eleve_id: Uuid,
    ) -> Result<(), EntDbError> {
        self.exiger_action_etab(etab, acteur, role, Action::AbsencesQualifier).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let r = sqlx::query(
            "UPDATE ent_asd_vigilance SET leve_le = now(), leve_par = $3
              WHERE etablissement_id = $1 AND eleve_id = $2 AND leve_le IS NULL",
        )
        .bind(etab)
        .bind(eleve_id)
        .bind(acteur)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        if r.rows_affected() == 0 {
            return Err(introuvable());
        }
        tx.commit().await.map_err(db)?;
        Ok(())
    }

    /// Élèves sous vigilance (CPE, direction : `absences.qualifier`).
    pub async fn vigilances_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
    ) -> Result<Vec<VigilanceAsd>, EntDbError> {
        self.exiger_action_etab(etab, acteur, role, Action::AbsencesQualifier).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        sqlx::query_as::<_, VigilanceRow>(
            "SELECT v.id, v.eleve_id, e.nom, e.prenom, v.pose_par, v.pose_le
               FROM ent_asd_vigilance v
               JOIN ent_eleve e ON e.etablissement_id = v.etablissement_id AND e.id = v.eleve_id
              WHERE v.etablissement_id = $1 AND v.leve_le IS NULL
              ORDER BY e.nom, e.prenom, v.id",
        )
        .bind(etab)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)
        .map(|v| {
            v.into_iter()
                .map(|r| VigilanceAsd {
                    id: r.id,
                    eleve_id: r.eleve_id,
                    nom: r.nom,
                    prenom: r.prenom,
                    pose_par: r.pose_par,
                    pose_le: r.pose_le,
                })
                .collect()
        })
    }

    // ───────────────────────── File du matin ─────────────────────────

    /// **File du matin** (AS-156, §17.6) : ce qui reste à traiter, classé par urgence. Droits `appel.verifier` **et**
    /// `absences.lire` (établissement). Une ligne par élève (ses autres raisons sont listées) ; les appels non faits
    /// et les incohérences suivent. Aucun motif, aucune note interne, aucun texte libre.
    pub async fn file_du_matin_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        jour: Option<NaiveDate>,
        maintenant: DateTime<Utc>,
    ) -> Result<FileDuMatin, EntDbError> {
        self.exiger_action_etab(etab, acteur, role, Action::AbsencesLire).await?;
        self.exiger_action_etab(etab, acteur, role, Action::AppelVerifier).await?;
        let aujourdhui = self.aujourdhui_etablissement(etab).await?;
        let jour = jour.unwrap_or(aujourdhui);
        if jour > aujourdhui {
            return Err(invalide("la file d'un jour futur n'existe pas"));
        }
        let (attendues, _) = self.seances_sans_appel_valide(etab, jour).await?;
        let incoherences = self.incoherences_appels(etab, jour).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let heure = heure_locale(&mut tx, etab, maintenant).await?;
        let delai = delai_verification(&mut tx, etab, jour).await?;

        let vigilants: HashSet<Uuid> = sqlx::query_scalar(
            "SELECT eleve_id FROM ent_asd_vigilance WHERE etablissement_id = $1 AND leve_le IS NULL",
        )
        .bind(etab)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?
        .into_iter()
        .collect();

        let mut candidats: Vec<EntreeFile> = Vec::new();

        // 1. Prévenir maintenant sans contact joint depuis.
        let urgences: Vec<LigneEleve> = sqlx::query_as(
            "SELECT p.id, p.eleve_id, e.nom, e.prenom, e.classe,
                    ent_cdt_heure_locale(p.etablissement_id, p.cree_le) AS depuis,
                    COALESCE(e.date_naissance + INTERVAL '18 years' > $2, true) AS mineur,
                    1::bigint AS nb
               FROM ent_asd_prevenir p
               JOIN ent_eleve e ON e.etablissement_id = p.etablissement_id AND e.id = p.eleve_id
              WHERE p.etablissement_id = $1 AND p.jour = $2
                AND NOT EXISTS (SELECT 1 FROM ent_asd_contact c
                                 WHERE c.etablissement_id = p.etablissement_id AND c.eleve_id = p.eleve_id
                                   AND c.issue = 'joint' AND c.cree_le >= p.cree_le)
              ORDER BY p.cree_le, p.id",
        )
        .bind(etab)
        .bind(jour)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        for u in urgences {
            candidats.push(entree_eleve("urgence", &u, None, &vigilants, heure));
        }

        // 2. Alertes persistées : exclu non arrivé, absence après présence.
        let alertes: Vec<LigneAlerte> = sqlx::query_as(
            "SELECT al.id, al.eleve_id, al.kind, al.seance_ref, al.statut, al.pris_par, al.pris_jusqua,
                    e.nom, e.prenom, e.classe,
                    ent_cdt_heure_locale(al.etablissement_id, al.cree_le) AS depuis,
                    COALESCE(e.date_naissance + INTERVAL '18 years' > $2, true) AS mineur
               FROM ent_asd_alerte al
               JOIN ent_eleve e ON e.etablissement_id = al.etablissement_id AND e.id = al.eleve_id
              WHERE al.etablissement_id = $1 AND al.jour = $2 AND al.statut <> 'traitee'
              ORDER BY al.cree_le, al.id",
        )
        .bind(etab)
        .bind(jour)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        for a in alertes {
            let categorie = if a.kind == "non_arrivee" { "non_arrivee" } else { "apres_presence" };
            let base = LigneEleve {
                id: a.id,
                eleve_id: a.eleve_id,
                nom: a.nom.clone(),
                prenom: a.prenom.clone(),
                classe: a.classe.clone(),
                depuis: Some(a.depuis),
                mineur: a.mineur,
                nb: 1,
            };
            let mut e = entree_eleve(categorie, &base, None, &vigilants, heure);
            e.alerte_id = Some(a.id);
            e.creneau_id = a.seance_ref;
            // Un verrou « je prends » échu ne compte plus.
            if a.pris_jusqua.is_some_and(|j| j > maintenant) {
                e.pris_par = a.pris_par;
                e.pris_jusqua = a.pris_jusqua;
            }
            candidats.push(e);
        }

        // 3. Épisodes du jour sans explication : « famille tentée sans succès » ou « sans nouvelles ».
        let episodes: Vec<LigneEleve> = sqlx::query_as(
            "SELECT a.id, a.eleve_id, e.nom, e.prenom, e.classe,
                    MIN(s.heure_debut) AS depuis,
                    COALESCE(e.date_naissance + INTERVAL '18 years' > $2, true) AS mineur,
                    COUNT(*)::bigint AS nb
               FROM ent_asd_absence a
               JOIN ent_asd_absence_seance s
                 ON s.etablissement_id = a.etablissement_id AND s.absence_id = a.id AND s.jour = $2
                AND s.retiree_le IS NULL AND s.kind = 'seance'
               JOIN ent_eleve e ON e.etablissement_id = a.etablissement_id AND e.id = a.eleve_id
              WHERE a.etablissement_id = $1 AND a.etat IN ('ouvert', 'a_qualifier') AND a.qualification = 'en_attente'
                AND a.prevenue = false AND a.motif_id IS NULL AND a.nature_code IS NULL
                AND NOT EXISTS (SELECT 1 FROM ent_asd_absence_declaration d
                                 WHERE d.etablissement_id = a.etablissement_id AND d.absence_id = a.id)
                AND NOT EXISTS (SELECT 1 FROM ent_asd_justificatif j
                                 WHERE j.etablissement_id = a.etablissement_id AND j.absence_id = a.id
                                   AND j.presente <> 'retire')
                AND NOT EXISTS (SELECT 1 FROM ent_asd_situation sit
                                 WHERE sit.etablissement_id = a.etablissement_id AND sit.eleve_id = a.eleve_id
                                   AND sit.nature = 'localisation' AND sit.etat = 'active'
                                   AND sit.debut_jour <= $2 AND sit.fin_jour >= $2)
              GROUP BY a.id, a.eleve_id, e.nom, e.prenom, e.classe, e.date_naissance
              ORDER BY MIN(s.heure_debut), a.id",
        )
        .bind(etab)
        .bind(jour)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let ids: Vec<Uuid> = episodes.iter().map(|e| e.eleve_id).collect();
        let contacts: Vec<(Uuid, String, i64)> = sqlx::query_as(
            "SELECT eleve_id, issue, COUNT(*)::bigint FROM ent_asd_contact
              WHERE etablissement_id = $1 AND ent_aujourdhui_etab(etablissement_id, cree_le) = $2
                AND eleve_id = ANY($3)
              GROUP BY eleve_id, issue",
        )
        .bind(etab)
        .bind(jour)
        .bind(&ids)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let mut joints: HashMap<Uuid, i64> = HashMap::new();
        let mut sans_succes: HashMap<Uuid, i64> = HashMap::new();
        for (eleve, issue, n) in contacts {
            match issue.as_str() {
                "joint" | "rencontre_faite" => *joints.entry(eleve).or_default() += n,
                "ne_repond_pas" | "numero_faux" | "message_laisse" => *sans_succes.entry(eleve).or_default() += n,
                _ => {}
            }
        }
        for ep in episodes {
            let j = joints.get(&ep.eleve_id).copied().unwrap_or(0);
            // Famille jointe aujourd'hui : plus de raison de relancer.
            if j > 0 {
                continue;
            }
            let s = sans_succes.get(&ep.eleve_id).copied().unwrap_or(0);
            let categorie = if s > 0 { "non_jointe" } else { "sans_nouvelles" };
            let mut e = entree_eleve(categorie, &ep, Some(ep.id), &vigilants, heure);
            e.contacts_sans_succes = s;
            candidats.push(e);
        }

        // 4. Une ligne par élève : la plus urgente, les autres raisons listées.
        let mut par_eleve: BTreeMap<Uuid, EntreeFile> = BTreeMap::new();
        for c in candidats {
            let Some(eleve) = c.eleve_id else { continue };
            match par_eleve.remove(&eleve) {
                None => {
                    par_eleve.insert(eleve, c);
                }
                Some(existant) => {
                    let (mut garde, autre) = if (c.niveau, c.depuis) < (existant.niveau, existant.depuis) {
                        (c, existant)
                    } else {
                        (existant, c)
                    };
                    if !garde.raisons.contains(&autre.categorie) {
                        garde.raisons.push(autre.categorie.clone());
                    }
                    for r in autre.raisons {
                        if !garde.raisons.contains(&r) {
                            garde.raisons.push(r);
                        }
                    }
                    // Le verrou et l'alerte de l'autre ligne ne se perdent pas.
                    garde.alerte_id = garde.alerte_id.or(autre.alerte_id);
                    garde.absence_id = garde.absence_id.or(autre.absence_id);
                    garde.pris_par = garde.pris_par.or(autre.pris_par);
                    garde.pris_jusqua = garde.pris_jusqua.or(autre.pris_jusqua);
                    garde.contacts_sans_succes = garde.contacts_sans_succes.max(autre.contacts_sans_succes);
                    garde.nb_seances = garde.nb_seances.max(autre.nb_seances);
                    par_eleve.insert(eleve, garde);
                }
            }
        }
        let mut entrees: Vec<EntreeFile> = par_eleve.into_values().collect();

        // 5. Appels non faits (à `début + 2 × délai`, §9.5) ou non clos après la fin de la séance.
        for a in attendues {
            let debut = instant_local(&mut tx, etab, jour, a.seance.heure_debut).await?;
            let fin = instant_local(&mut tx, etab, jour, a.seance.heure_fin).await?;
            let detail = if a.appel_id.is_none() && maintenant >= debut + Duration::minutes(2 * delai) {
                "non_fait"
            } else if a.appel_id.is_some() && maintenant >= fin && !a.substitution {
                "non_clos"
            } else {
                continue;
            };
            entrees.push(EntreeFile {
                cle: format!("appel:{}", a.seance.seance_ref),
                categorie: "appel_non_fait".into(),
                raisons: Vec::new(),
                niveau: niveau_de("appel_non_fait", false, false),
                rang: 0,
                eleve_id: None,
                nom: None,
                prenom: None,
                classe: None,
                mineur: None,
                vigilance: false,
                depuis: Some(a.seance.heure_debut),
                minutes_depuis: Some((maintenant - debut).num_minutes()),
                nb_seances: None,
                absence_id: None,
                alerte_id: None,
                creneau_id: Some(a.seance.seance_ref),
                classe_seance: Some(a.seance.classe.clone()),
                intervenant_user_id: Some(a.seance.prof_user_id),
                detail_code: Some(detail.into()),
                contacts_joints: 0,
                contacts_sans_succes: 0,
                pris_par: None,
                pris_jusqua: None,
            });
        }

        // 6. Incohérences d'appel (§9.7) : jamais de modification automatique.
        for (i, inc) in incoherences.into_iter().enumerate() {
            entrees.push(EntreeFile {
                cle: format!("incoherence:{}:{}:{i}", inc.eleve_id, inc.kind),
                categorie: "incoherence".into(),
                raisons: Vec::new(),
                niveau: niveau_de("incoherence", false, false),
                rang: 0,
                eleve_id: Some(inc.eleve_id),
                nom: Some(inc.nom),
                prenom: Some(inc.prenom),
                classe: None,
                mineur: None,
                vigilance: vigilants.contains(&inc.eleve_id),
                depuis: None,
                minutes_depuis: None,
                nb_seances: None,
                absence_id: None,
                alerte_id: None,
                creneau_id: None,
                classe_seance: None,
                intervenant_user_id: None,
                detail_code: Some(inc.kind),
                contacts_joints: 0,
                contacts_sans_succes: 0,
                pris_par: None,
                pris_jusqua: None,
            });
        }

        entrees.sort_by(|a, b| {
            a.niveau
                .cmp(&b.niveau)
                // Le plus ancien d'abord : une heure inconnue passe après une heure connue.
                .then_with(|| match (a.depuis, b.depuis) {
                    (Some(x), Some(y)) => x.cmp(&y),
                    (Some(_), None) => std::cmp::Ordering::Less,
                    (None, Some(_)) => std::cmp::Ordering::Greater,
                    (None, None) => std::cmp::Ordering::Equal,
                })
                // À heure égale, celui qui a manqué le plus de séances.
                .then_with(|| b.nb_seances.cmp(&a.nb_seances))
                .then_with(|| a.nom.cmp(&b.nom))
                .then_with(|| a.prenom.cmp(&b.prenom))
                .then_with(|| a.cle.cmp(&b.cle))
        });
        let mut compteurs: BTreeMap<String, i64> = BTreeMap::new();
        for (i, e) in entrees.iter_mut().enumerate() {
            e.rang = (i + 1) as u32;
            *compteurs.entry(e.categorie.clone()).or_default() += 1;
        }
        tx.commit().await.map_err(db)?;
        Ok(FileDuMatin {
            jour,
            heure_locale: heure,
            entrees,
            compteurs,
        })
    }
}

// ───────────────────────── Lignes SQL ─────────────────────────

#[derive(sqlx::FromRow)]
struct LigneEleve {
    id: Uuid,
    eleve_id: Uuid,
    nom: String,
    prenom: String,
    classe: Option<String>,
    depuis: Option<NaiveTime>,
    mineur: bool,
    nb: i64,
}

#[derive(sqlx::FromRow)]
struct LigneAlerte {
    id: Uuid,
    eleve_id: Uuid,
    kind: String,
    seance_ref: Option<Uuid>,
    #[allow(dead_code)]
    statut: String,
    pris_par: Option<Uuid>,
    pris_jusqua: Option<DateTime<Utc>>,
    nom: String,
    prenom: String,
    classe: Option<String>,
    depuis: NaiveTime,
    mineur: bool,
}

#[derive(sqlx::FromRow)]
struct AlerteRow {
    id: Uuid,
    eleve_id: Uuid,
    kind: String,
    jour: NaiveDate,
    statut: String,
    pris_par: Option<Uuid>,
    pris_jusqua: Option<DateTime<Utc>>,
    cloture_code: Option<String>,
    cree_le: DateTime<Utc>,
}

impl From<AlerteRow> for AlerteAsd {
    fn from(r: AlerteRow) -> Self {
        Self {
            id: r.id,
            eleve_id: r.eleve_id,
            kind: r.kind,
            jour: r.jour,
            statut: r.statut,
            pris_par: r.pris_par,
            pris_jusqua: r.pris_jusqua,
            cloture_code: r.cloture_code,
            cree_le: r.cree_le,
        }
    }
}

#[derive(sqlx::FromRow)]
struct VigilanceRow {
    id: Uuid,
    eleve_id: Uuid,
    nom: String,
    prenom: String,
    pose_par: Uuid,
    pose_le: DateTime<Utc>,
}

async fn lire_alerte(conn: &mut PgConnection, etab: Uuid, id: Uuid) -> Result<AlerteAsd, EntDbError> {
    sqlx::query_as::<_, AlerteRow>(
        "SELECT id, eleve_id, kind, jour, statut, pris_par, pris_jusqua, cloture_code, cree_le
           FROM ent_asd_alerte WHERE etablissement_id = $1 AND id = $2",
    )
    .bind(etab)
    .bind(id)
    .fetch_optional(&mut *conn)
    .await
    .map_err(db)?
    .map(AlerteAsd::from)
    .ok_or_else(introuvable)
}

fn entree_eleve(
    categorie: &str,
    l: &LigneEleve,
    absence_id: Option<Uuid>,
    vigilants: &HashSet<Uuid>,
    maintenant_local: NaiveTime,
) -> EntreeFile {
    let vigilance = vigilants.contains(&l.eleve_id);
    EntreeFile {
        cle: format!("{categorie}:{}", l.id),
        categorie: categorie.into(),
        raisons: Vec::new(),
        niveau: niveau_de(categorie, vigilance, l.mineur),
        rang: 0,
        eleve_id: Some(l.eleve_id),
        nom: Some(l.nom.clone()),
        prenom: Some(l.prenom.clone()),
        classe: l.classe.clone(),
        mineur: Some(l.mineur),
        vigilance,
        depuis: l.depuis,
        minutes_depuis: l.depuis.map(|d| (maintenant_local - d).num_minutes()),
        nb_seances: Some(l.nb),
        absence_id,
        alerte_id: None,
        creneau_id: None,
        classe_seance: None,
        intervenant_user_id: None,
        detail_code: None,
        contacts_joints: 0,
        contacts_sans_succes: 0,
        pris_par: None,
        pris_jusqua: None,
    }
}

// ───────────────────────── Consommateur « assiduite.verification » ─────────────────────────

/// Alerte « absence après présence » (AS-154) et clôture des alertes quand le fait disparaît (localisation, correction).
pub struct ConsommateurVerification;

const TYPES_ECOUTES: &[&str] = &[
    crate::assiduite_as5::types_as5::ABSENCE_OUVERTE,
    crate::assiduite_as2::types_appel::LOCALISE,
    crate::assiduite_as2::types_appel::ABSENCE_DEMENTIE,
];

impl Consommateur for ConsommateurVerification {
    fn nom(&self) -> &'static str {
        CONSOMMATEUR_VERIFICATION
    }

    fn types(&self) -> &'static [&'static str] {
        TYPES_ECOUTES
    }

    fn traiter<'a>(&'a self, tx: &'a mut PgConnection, evt: &'a EvenementDistribue) -> FutureTraitement<'a> {
        Box::pin(async move { traiter(tx, evt).await.map_err(|e| e.to_string()) })
    }
}

async fn traiter(tx: &mut PgConnection, evt: &EvenementDistribue) -> Result<(), EntDbError> {
    let e = evt.event();
    let etab = evt.etablissement_id();
    let Some(eleve) = e.eleve_id else {
        return Ok(());
    };
    let texte = |cle: &str| e.payload.get(cle).and_then(|v| v.as_str());
    let Some(jour) = texte("jour").and_then(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d").ok()) else {
        return Ok(());
    };
    // Relecture dans l'établissement de l'événement : un élève d'un autre établissement est introuvable.
    let existe: Option<i32> = sqlx::query_scalar("SELECT 1 FROM ent_eleve WHERE etablissement_id = $1 AND id = $2")
        .bind(etab)
        .bind(eleve)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
    if existe.is_none() {
        return Ok(());
    }
    match e.type_name.as_str() {
        t if t == crate::assiduite_as5::types_as5::ABSENCE_OUVERTE => {
            let prevenue = e.payload.get("prevenue").and_then(|v| v.as_bool()).unwrap_or(false);
            let Some(appel) = texte("appel_id").and_then(|s| Uuid::parse_str(s).ok()) else {
                return Ok(());
            };
            // Une absence annoncée par la famille n'est pas « sans motif connu » : pas d'alerte.
            if prevenue {
                return Ok(());
            }
            let ligne: Option<(Option<NaiveTime>, Option<Uuid>)> = sqlx::query_as(
                "SELECT COALESCE(a.heure_debut, c.heure_debut), a.creneau_id
                   FROM ent_appel a
                   LEFT JOIN ent_edt_creneau c ON c.etablissement_id = a.etablissement_id AND c.id = a.creneau_id
                  WHERE a.etablissement_id = $1 AND a.id = $2",
            )
            .bind(etab)
            .bind(appel)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db)?;
            let Some((Some(heure), creneau)) = ligne else {
                return Ok(());
            };
            if !etait_present_plus_tot(tx, etab, eleve, jour, heure, Some(appel)).await? {
                return Ok(());
            }
            let id = Uuid::new_v4();
            let cree: Option<(Uuid,)> = sqlx::query_as(
                "INSERT INTO ent_asd_alerte (id, etablissement_id, eleve_id, kind, jour, seance_ref, appel_id, cle)
                 VALUES ($1, $2, $3, 'absence_apres_presence', $4, $5, $6, $7)
                 ON CONFLICT (etablissement_id, kind, cle) DO NOTHING
                 RETURNING id",
            )
            .bind(id)
            .bind(etab)
            .bind(eleve)
            .bind(jour)
            .bind(creneau)
            .bind(appel)
            .bind(format!("{appel}:{eleve}"))
            .fetch_optional(&mut *tx)
            .await
            .map_err(db)?;
            if cree.is_some() {
                emettre(
                    tx,
                    etab,
                    types_as6::ABSENCE_APRES_PRESENCE,
                    AggregateKind::Appel,
                    appel,
                    Some(eleve),
                    json!({ "alerte_id": id, "appel_id": appel, "jour": jour.to_string() }),
                    None,
                )
                .await?;
            }
            Ok(())
        }
        t if t == crate::assiduite_as2::types_appel::LOCALISE => {
            // Localisé (infirmerie, CDI…) : l'absence « après présence » est expliquée (R-N67).
            sqlx::query(
                "UPDATE ent_asd_alerte SET statut = 'traitee', traite_le = now(), cloture_code = 'localise',
                        cloture_par_systeme = true, modifie_le = now()
                  WHERE etablissement_id = $1 AND eleve_id = $2 AND jour = $3
                    AND kind = 'absence_apres_presence' AND statut <> 'traitee'",
            )
            .bind(etab)
            .bind(eleve)
            .bind(jour)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
            Ok(())
        }
        t if t == crate::assiduite_as2::types_appel::ABSENCE_DEMENTIE => {
            // Absence corrigée : plus d'alerte si plus aucune absence active ne vient de cet appel.
            sqlx::query(
                "UPDATE ent_asd_alerte al
                    SET statut = 'traitee', traite_le = now(), cloture_code = 'corrige', cloture_par_systeme = true,
                        modifie_le = now()
                  WHERE al.etablissement_id = $1 AND al.eleve_id = $2 AND al.jour = $3
                    AND al.kind = 'absence_apres_presence' AND al.statut <> 'traitee'
                    AND NOT EXISTS (SELECT 1 FROM ent_asd_absence_seance s
                                     WHERE s.etablissement_id = al.etablissement_id AND s.eleve_id = al.eleve_id
                                       AND s.jour = al.jour AND s.appel_id = al.appel_id AND s.retiree_le IS NULL)",
            )
            .bind(etab)
            .bind(eleve)
            .bind(jour)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
            Ok(())
        }
        _ => Ok(()),
    }
}

/// Délai minimal (minutes) avant l'avis d'une absence **après présence** (R-N67) : 15 minutes, plafonné à 30.
pub(crate) fn delai_apres_presence(delai_regle: i64) -> i64 {
    delai_regle.max(DELAI_APRES_PRESENCE_MIN).min(30)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn l_ordre_de_la_file_met_la_securite_avant_l_administratif() {
        let ordre = [
            niveau_de("urgence", false, true),
            niveau_de("non_arrivee", false, true),
            niveau_de("apres_presence", false, true),
            niveau_de("sans_nouvelles", true, true),
            niveau_de("non_jointe", false, true),
            niveau_de("sans_nouvelles", false, true),
            niveau_de("sans_nouvelles", false, false),
            niveau_de("appel_non_fait", false, false),
            niveau_de("incoherence", false, false),
        ];
        assert_eq!(ordre, [1, 2, 3, 4, 5, 6, 7, 8, 9]);
        // La vigilance fait passer une famille tentée sans succès avant les autres.
        assert_eq!(niveau_de("non_jointe", true, false), 4);
    }

    #[test]
    fn le_delai_apres_presence_est_d_au_moins_quinze_minutes_et_au_plus_trente() {
        assert_eq!(delai_apres_presence(0), 15);
        assert_eq!(delai_apres_presence(10), 15);
        assert_eq!(delai_apres_presence(20), 20);
        assert_eq!(delai_apres_presence(240), 30);
    }

    #[test]
    fn les_listes_fermees_sont_sans_doublon() {
        for l in [&RAISONS_A_LA_PLACE[..], &RAISONS_PREVENIR[..], &CLOTURES_ALERTE[..]] {
            let s: HashSet<_> = l.iter().collect();
            assert_eq!(s.len(), l.len());
        }
    }
}
