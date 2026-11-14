//! Socle 12, lot AS2 : appel v2 et corrections (`docs/socles/12_ASSIDUITE.md` §8, §9, §17.1,
//! §17.4, §22.2, §22.3, §29).
//!
//! Ce que le lot ajoute à l'appel de l'AS0 :
//! - **états de ligne** étendus (`non_pointe`, `localise`, `attendu_ailleurs`, `present_dispense`) et
//!   précisions (minutes, lieu codé), **historique en ajout seul** de chaque ligne (R-AS8) ;
//! - **situations connues** (déclaration, localisation, sortie…) qui **pré-remplissent** l'appel sans
//!   jamais créer d'absence (R-AS4), actualisées jusqu'à la validation (une déclaration de 8 h 05
//!   apparaît dans l'appel de 8 h encore ouvert) ;
//! - **effectif actualisé** jusqu'à la validation (DA-19) ;
//! - appel d'une **activité** sans séance (permanence, étude…) par un rôle libre (DA-18) ;
//! - **validation** en lot avec récapitulatif, modes « tous présents » et « présents seulement » ;
//! - **corrections** après validation (intervenant le jour même, vie scolaire à tout moment avec
//!   motif), effets recalculés et **démenti** (AS-131, AS-136) ;
//! - **neutralisation** d'un appel dont la séance est annulée après coup (DA-09, AS-135) ;
//! - **délégation** datée (AS-105) ; **exclusion ponctuelle de cours** suivie jusqu'à l'arrivée à la
//!   vie scolaire (R-AS37) ; **incohérences** (§9.7), jamais de modification automatique.
//!
//! Minimisation (R-AS19, R-AS21, R-AS22) : les DTO de l'appel ne portent **jamais** un motif ; le
//! libellé court d'une situation est dérivé de sa nature (« prévenu », « en sortie », « infirmerie »).
//! Le texte libre d'une déclaration est dans une table à part, lue seulement par son auteur et la vie
//! scolaire. Les payloads d'événements ne portent que des codes et des identifiants (R-AS25).

use crate::appel_repo::{Appel, AppelStatut, EcritureLigne, LigneEtat, PgAppelRepo};
use crate::outbox::{Causalite, Publier};
use crate::service::{EntDbError, PgEnt};
use crate::tenant_tx::TenantTx;
use crate::vie_scolaire::{db, introuvable};
use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
use domain_eleve::DomainError;
use ent_auth::Action;
use ent_events::{AggregateKind, MetierEventDraft};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::PgConnection;
use std::collections::HashMap;
use uuid::Uuid;

/// Types d'événements de l'appel v2 (constantes locales : le catalogue central est partagé).
pub mod types_appel {
    pub const LIGNE_CORRIGEE: &str = "appel.ligne_corrigee";
    pub const NEUTRALISE: &str = "appel.neutralise";
    pub const DELEGUE: &str = "appel.delegue";
    pub const DELEGATION_REVOQUEE: &str = "appel.delegation_revoquee";
    pub const ABSENCE_DEMENTIE: &str = "assiduite.absence_dementie";
    pub const LOCALISE: &str = "assiduite.localise";
    pub const SITUATION_LEVEE: &str = "assiduite.situation_levee";
    pub const RETARD_ANNONCE: &str = "assiduite.retard_annonce";
    pub const EXCLUSION_COURS: &str = "appel.exclusion_cours";
    pub const ARRIVEE_VS_CONFIRMEE: &str = "appel.arrivee_vs_confirmee";
}

/// Lieux codés d'une localisation (jamais un motif, R-AS20).
pub const LIEUX_LOCALISATION: [&str; 5] = ["infirmerie", "vie_scolaire", "cdi", "entretien", "autre_activite"];

/// Types d'activités appelables hors cours.
const TYPES_ACTIVITE_APPEL: [&str; 5] = ["permanence", "etude", "cdi", "demi_pension", "accompagnement"];

// ── DTO ────────────────────────────────────────────────────────────────────────────────────

/// Ligne d'un appel pour l'écran de pointage : jamais de motif.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LigneAppelV2 {
    pub eleve_id: Uuid,
    pub etat: String,
    pub minutes_retard: Option<i32>,
    pub lieu_code: Option<String>,
    /// Pré-remplie par une situation connue (déclaration, sortie, localisation…).
    pub pre_remplie: bool,
    /// Libellé court dérivé de la nature de la situation (« prévenu », « en sortie »…), sans motif.
    pub libelle_court: Option<String>,
    pub version: i32,
    /// Nom affiché (rempli par l'API, jamais par le service).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub eleve_label: Option<String>,
    /// Élève exclu ponctuellement du cours et pas encore arrivé à la vie scolaire (R-AS37).
    pub arrivee_vs_attendue: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct RecapAppel {
    pub presents: i64,
    pub absents: i64,
    pub absents_prevenus: i64,
    pub retards: i64,
    pub localises: i64,
    pub attendus_ailleurs: i64,
    pub dispenses: i64,
    pub non_pointes: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppelV2 {
    pub id: Uuid,
    pub jour: NaiveDate,
    pub classe: String,
    pub type_activite: String,
    pub etat: String,
    pub mode_pointage: String,
    pub heure_debut: Option<NaiveTime>,
    pub heure_fin: Option<NaiveTime>,
    pub creneau_id: Option<Uuid>,
    pub matiere_libelle: Option<String>,
    pub delegue_a: Option<Uuid>,
    pub lignes: Vec<LigneAppelV2>,
    pub recapitulatif: RecapAppel,
}

/// Ouverture de l'appel d'une activité sans séance (permanence, étude, CDI…).
#[derive(Debug, Clone, Deserialize)]
pub struct OuvrirActivite {
    pub type_activite: String,
    pub jour: NaiveDate,
    pub heure_debut: NaiveTime,
    pub heure_fin: NaiveTime,
    /// Libellé court de l'activité (« Permanence salle 12 »), 1 à 120 caractères.
    pub libelle: String,
    /// Clé stable de l'activité (idempotence) ; par défaut dérivée du libellé et de l'horaire.
    pub cle: Option<String>,
    pub eleve_ids: Vec<Uuid>,
    pub mode_pointage: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SaisieLigne {
    pub etat: String,
    pub minutes: Option<i32>,
    pub lieu_code: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CorrectionLigne {
    pub etat: String,
    pub minutes: Option<i32>,
    pub lieu_code: Option<String>,
    /// `erreur_saisie`, `billet_entree`, `oubli`, `correction_vie_scolaire`.
    pub motif_code: Option<String>,
    /// Motif court facultatif (200 caractères) ; **obligatoire** pour la vie scolaire.
    pub motif_texte: Option<String>,
    /// Version de la ligne lue par l'appelant : conflit si elle a changé depuis.
    pub version_base: Option<i32>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ValidationAppel {
    /// `confirme_non_pointes` : en mode « présents seulement », confirme que les non pointés
    /// deviennent absents.
    #[serde(default)]
    pub confirme_non_pointes: bool,
    /// `web` (défaut), `mobile`, `hors_ligne`.
    pub canal: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct HistoLigne {
    pub version: i32,
    pub etat_avant: Option<String>,
    pub etat_apres: String,
    pub minutes: Option<i32>,
    pub lieu_code: Option<String>,
    pub auteur_id: Option<Uuid>,
    pub canal: String,
    pub motif_code: Option<String>,
    pub motif_texte: Option<String>,
    pub cree_le: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IncoherenceV2 {
    pub eleve_id: Uuid,
    /// `present_malgre_declaration`, `present_alors_quen_sortie`, `absent_puis_present`,
    /// `exclusion_sans_arrivee`.
    pub code: String,
}

/// Situation connue, sans le texte libre (qui reste à part, R-AS22).
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct SituationAsd {
    pub id: Uuid,
    pub eleve_id: Uuid,
    pub nature: String,
    pub debut_jour: NaiveDate,
    pub debut_heure: Option<NaiveTime>,
    pub fin_jour: NaiveDate,
    pub fin_heure: Option<NaiveTime>,
    pub heure_arrivee_prevue: Option<NaiveTime>,
    pub source: String,
    pub lieu_code: Option<String>,
    pub etat: String,
}

/// Localisation d'un élève (infirmerie, vie scolaire, CDI, entretien) : action `localisation.poser`.
#[derive(Debug, Clone, Deserialize)]
pub struct PoserLocalisation {
    pub eleve_id: Uuid,
    pub lieu_code: String,
    pub jour: Option<NaiveDate>,
    pub heure_debut: Option<NaiveTime>,
    pub heure_fin: Option<NaiveTime>,
}

/// « Il arrive à 10 h » (AS-218) : absence prévenue jusqu'à l'heure dite, présence attendue ensuite.
#[derive(Debug, Clone, Deserialize)]
pub struct AnnoncerRetard {
    pub eleve_id: Uuid,
    pub jour: Option<NaiveDate>,
    pub heure_arrivee_prevue: NaiveTime,
    pub declarant_nom: Option<String>,
}

const COLONNES_SITUATION: &str = "id, eleve_id, nature, debut_jour, debut_heure, fin_jour, fin_heure,
    heure_arrivee_prevue, source, lieu_code, etat";

// ── Calcul pur du pré-remplissage ───────────────────────────────────────────────────────────

/// Résultat du pré-remplissage d'une ligne.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Propose {
    pub etat: LigneEtat,
    pub lieu: Option<&'static str>,
    pub situation_id: Uuid,
}

/// Une situation couvre-t-elle la plage `[debut, fin)` de la séance du `jour` ?
/// Heure absente = début ou fin de journée ; `retard_annonce` couvre `[00:00, arrivée prévue)`.
pub fn situation_couvre(
    s: &SituationAsd,
    jour: NaiveDate,
    debut: NaiveTime,
    fin: NaiveTime,
) -> bool {
    if jour < s.debut_jour || jour > s.fin_jour {
        return false;
    }
    let minuit = NaiveTime::MIN;
    let fin_de_journee = NaiveTime::from_hms_opt(23, 59, 59).unwrap_or_default();
    let (mut d, mut f) = (
        if jour == s.debut_jour { s.debut_heure.unwrap_or(minuit) } else { minuit },
        if jour == s.fin_jour { s.fin_heure.unwrap_or(fin_de_journee) } else { fin_de_journee },
    );
    if s.nature == "retard_annonce" {
        let Some(arrivee) = s.heure_arrivee_prevue else { return false };
        d = minuit;
        f = arrivee;
    }
    d < fin && f > debut
}

/// Rang de priorité d'une nature : une localisation constatée l'emporte sur un « attendu ailleurs »,
/// qui l'emporte sur une simple déclaration (R-AS10).
fn rang(nature: &str) -> u8 {
    match nature {
        "localisation" | "exclusion_cours" => 1,
        "sortie" | "examen" | "mission" | "pfmp" | "exclusion_inclusion" | "mobilite"
        | "dispense_presence" | "prise_en_charge_exterieure" => 2,
        _ => 3,
    }
}

/// État proposé à l'appel par une situation (jamais une absence pour une situation « ailleurs »).
pub fn etat_propose(s: &SituationAsd) -> (LigneEtat, Option<&'static str>) {
    match s.nature.as_str() {
        "localisation" | "exclusion_cours" => (
            LigneEtat::Localise,
            Some(match s.lieu_code.as_deref() {
                Some("infirmerie") => "infirmerie",
                Some("cdi") => "cdi",
                Some("entretien") => "entretien",
                Some("autre_activite") => "autre_activite",
                _ => "vie_scolaire",
            }),
        ),
        "sortie" => (LigneEtat::AttenduAilleurs, Some("sortie")),
        "examen" => (LigneEtat::AttenduAilleurs, Some("examen")),
        "mission" => (LigneEtat::AttenduAilleurs, Some("instance")),
        "pfmp" => (LigneEtat::AttenduAilleurs, Some("stage")),
        "exclusion_inclusion" => (LigneEtat::AttenduAilleurs, Some("vie_scolaire")),
        "mobilite" => (LigneEtat::AttenduAilleurs, Some("autre_activite")),
        "dispense_presence" => (LigneEtat::AttenduAilleurs, Some("dispense")),
        "prise_en_charge_exterieure" => (LigneEtat::AttenduAilleurs, Some("prise_en_charge_exterieure")),
        // Déclaration, autorisation, sortie anticipée, retard annoncé : absent prévenu.
        _ => (LigneEtat::Absent, None),
    }
}

/// Meilleure situation pour un élève (priorité puis la plus récemment créée : l'ordre d'entrée).
pub fn meilleure_situation<'a>(
    situations: &'a [SituationAsd],
    jour: NaiveDate,
    debut: NaiveTime,
    fin: NaiveTime,
) -> Option<&'a SituationAsd> {
    situations
        .iter()
        .filter(|s| s.etat == "active" && situation_couvre(s, jour, debut, fin))
        .min_by_key(|s| rang(&s.nature))
}

/// Libellé court affiché au professeur : **jamais un motif**, seulement la nature ou le lieu.
pub fn libelle_court(nature: &str, lieu: Option<&str>) -> &'static str {
    match nature {
        "localisation" | "exclusion_cours" => match lieu {
            Some("infirmerie") => "à l'infirmerie",
            Some("cdi") => "au CDI",
            Some("entretien") => "en entretien",
            _ => "à la vie scolaire",
        },
        "sortie" | "sortie_anticipee" => "en sortie",
        "examen" => "examen",
        "mission" => "instance",
        "pfmp" => "en stage",
        "exclusion_inclusion" => "à la vie scolaire",
        "mobilite" => "mobilité",
        "dispense_presence" => "dispensé",
        "prise_en_charge_exterieure" => "prise en charge extérieure",
        _ => "prévenu",
    }
}

fn etat_saisissable(etat: &str) -> Result<LigneEtat, EntDbError> {
    match LigneEtat::parse(etat) {
        Some(LigneEtat::AttenduAilleurs) | Some(LigneEtat::NonPointe) | None => Err(
            EntDbError::SaisieInvalide("état de ligne non saisissable"),
        ),
        Some(e) => Ok(e),
    }
}

fn valider_precisions(etat: &LigneEtat, minutes: Option<i32>, lieu: Option<&str>) -> Result<(), EntDbError> {
    let m_ok = match (etat, minutes) {
        (LigneEtat::Retard, Some(m)) => (1..=600).contains(&m),
        (LigneEtat::Retard, None) => true,
        (_, None) => true,
        (_, Some(_)) => false,
    };
    let l_ok = match (etat, lieu) {
        (LigneEtat::Localise, Some(l)) => LIEUX_LOCALISATION.contains(&l),
        (LigneEtat::Localise, None) => false,
        (_, None) => true,
        (_, Some(_)) => false,
    };
    if m_ok && l_ok {
        Ok(())
    } else {
        Err(EntDbError::SaisieInvalide("précision de ligne invalide (minutes ou lieu)"))
    }
}

fn recapituler(lignes: &[LigneAppelV2]) -> RecapAppel {
    let mut r = RecapAppel::default();
    for l in lignes {
        match l.etat.as_str() {
            "present" => r.presents += 1,
            "absent" => {
                r.absents += 1;
                if l.pre_remplie {
                    r.absents_prevenus += 1;
                }
            }
            "retard" => r.retards += 1,
            "localise" => r.localises += 1,
            "attendu_ailleurs" => r.attendus_ailleurs += 1,
            "present_dispense" => r.dispenses += 1,
            _ => r.non_pointes += 1,
        }
    }
    r
}

#[derive(Debug, sqlx::FromRow)]
struct AppelMeta {
    jour: NaiveDate,
    classe: String,
    type_activite: String,
    etat: String,
    mode_pointage: String,
    heure_debut: Option<NaiveTime>,
    heure_fin: Option<NaiveTime>,
    creneau_id: Option<Uuid>,
    matiere_libelle: Option<String>,
    delegue_a: Option<Uuid>,
    effectif_complet: bool,
    auteur_user_id: Uuid,
}

const COLONNES_META: &str = "jour, classe, type_activite, etat, mode_pointage, heure_debut, heure_fin,
    creneau_id, matiere_libelle, delegue_a, effectif_complet, auteur_user_id";

impl PgEnt {
    /// Heure murale de l'établissement (fuseau de `ent_etablissement_parametre`, Paris par défaut).
    pub(crate) async fn maintenant_etablissement(
        &self,
        etablissement_id: Uuid,
    ) -> Result<NaiveDateTime, EntDbError> {
        let (t,): (NaiveDateTime,) = sqlx::query_as(
            "SELECT (now() AT TIME ZONE COALESCE(
                       (SELECT p.fuseau FROM ent_etablissement_parametre p WHERE p.etablissement_id = $1),
                       'Europe/Paris'))::timestamp",
        )
        .bind(etablissement_id)
        .fetch_one(&mut *TenantTx::begin(self.pool(), etablissement_id).await.map_err(db)?)
        .await
        .map_err(db)?;
        Ok(t)
    }

    async fn charger_appel(&self, etablissement_id: Uuid, appel_id: Uuid) -> Result<Appel, EntDbError> {
        self.repo_appels()
            .get(etablissement_id, appel_id)
            .await?
            .ok_or(EntDbError::AppelInvalide)
    }

    async fn meta_appel(&self, etablissement_id: Uuid, appel_id: Uuid) -> Result<AppelMeta, EntDbError> {
        let sql = format!("SELECT {COLONNES_META} FROM ent_appel WHERE etablissement_id = $1 AND id = $2");
        sqlx::query_as::<_, AppelMeta>(&sql)
            .bind(etablissement_id)
            .bind(appel_id)
            .fetch_optional(&mut *TenantTx::begin(self.pool(), etablissement_id).await.map_err(db)?)
            .await
            .map_err(db)?
            .ok_or(EntDbError::AppelInvalide)
    }

    // ── Ouverture d'une activité (permanence, étude…) ───────────────────────────────────────

    /// Ouvre l'appel d'une **activité** sans séance (permanence, étude, CDI, accompagnement,
    /// demi-pension), par toute personne qui a `appel.faire` — un rôle libre « AED » par exemple
    /// (DA-18). L'activité doit être appelée par l'établissement (`ent_asd_activite_appelee`).
    /// Idempotent par (type, clé, jour). L'effectif vient de l'appelant (le socle 10 fournira la
    /// permanence : EDT-219) ; chaque élève doit exister dans l'établissement.
    pub async fn ouvrir_appel_activite(
        &self,
        etablissement_id: Uuid,
        acteur: Uuid,
        role: &str,
        o: OuvrirActivite,
    ) -> Result<AppelV2, EntDbError> {
        let d = self.droits_appel(etablissement_id, acteur, role).await?;
        if !d.faire {
            return Err(EntDbError::Forbidden);
        }
        let libelle = o.libelle.trim();
        let mode = o.mode_pointage.as_deref().unwrap_or("tous_presents");
        if !TYPES_ACTIVITE_APPEL.contains(&o.type_activite.as_str())
            || o.heure_fin <= o.heure_debut
            || !(1..=120).contains(&libelle.chars().count())
            || !matches!(mode, "tous_presents" | "presents_seulement")
            || o.eleve_ids.is_empty()
            || o.eleve_ids.len() > 300
        {
            return Err(EntDbError::SaisieInvalide("activité d'appel invalide"));
        }
        if !self.activite_est_appelee(etablissement_id, &o.type_activite).await? {
            return Err(EntDbError::Refus(
                crate::service::RefusMetier::Conflit,
                "cette activité n'est pas appelée dans l'établissement",
            ));
        }
        let aujourdhui = self.aujourdhui_etablissement(etablissement_id).await?;
        if o.jour > aujourdhui || o.jour < aujourdhui - chrono::Duration::days(1) {
            return Err(EntDbError::SaisieInvalide("jour hors du jour même ou de la veille"));
        }
        let cle = o
            .cle
            .clone()
            .unwrap_or_else(|| format!("{}:{}", libelle.to_lowercase().replace(' ', "-"), o.heure_debut.format("%H%M")));
        if cle.is_empty() || cle.len() > 120 {
            return Err(EntDbError::SaisieInvalide("clé d'activité invalide"));
        }
        // Chaque élève existe dans CET établissement (jamais d'identifiant d'un autre).
        let mut ids = o.eleve_ids.clone();
        ids.sort();
        ids.dedup();
        let (connus,): (i64,) = sqlx::query_as(
            "SELECT count(*) FROM ent_eleve WHERE etablissement_id = $1 AND id = ANY($2)",
        )
        .bind(etablissement_id)
        .bind(&ids)
        .fetch_one(&mut *TenantTx::begin(self.pool(), etablissement_id).await.map_err(db)?)
        .await
        .map_err(db)?;
        if connus != ids.len() as i64 {
            return Err(introuvable());
        }
        if let Some(existant) = self
            .repo_appels()
            .get_by_activite(etablissement_id, &o.type_activite, &cle, o.jour)
            .await?
        {
            return self.appel_v2_lu(etablissement_id, existant.id, acteur, role).await;
        }
        let appel = Appel {
            id: Uuid::new_v4(),
            etablissement_id,
            classe: libelle.to_string(),
            jour: o.jour,
            auteur_user_id: acteur,
            statut: AppelStatut::Ouvert,
            creneau_id: None,
            matiere_libelle: Some(libelle.to_string()),
        };
        let etat_initial = if mode == "presents_seulement" { LigneEtat::NonPointe } else { LigneEtat::Present };
        let lignes: Vec<_> = ids
            .iter()
            .map(|e| crate::appel_repo::AppelLigne::nouvelle(etablissement_id, appel.id, *e, etat_initial.clone()))
            .collect();
        let ext = crate::appel_repo::AppelExt {
            type_activite: o.type_activite.clone(),
            activite_cle: Some(cle.clone()),
            heure_debut: Some(o.heure_debut),
            heure_fin: Some(o.heure_fin),
            effectif_complet: true,
            mode_pointage: mode.to_string(),
        };
        let mut tx = TenantTx::begin(self.pool(), etablissement_id).await?;
        let cree = self.repo_appels().inserer_atomique_tx(&mut tx, &appel, &ext, &lignes).await?;
        if !cree {
            drop(tx);
            let existant = self
                .repo_appels()
                .get_by_activite(etablissement_id, &o.type_activite, &cle, o.jour)
                .await?
                .ok_or(EntDbError::AppelInvalide)?;
            return self.appel_v2_lu(etablissement_id, existant.id, acteur, role).await;
        }
        let ev = tx
            .publier(MetierEventDraft {
                etablissement_id,
                type_name: ent_events::types::APPEL_OPENED.to_string(),
                aggregate: AggregateKind::Appel,
                aggregate_id: appel.id,
                eleve_id: None,
                payload: json!({
                    "type_activite": o.type_activite,
                    "jour": o.jour.to_string(),
                    "effectif": lignes.len(),
                }),
                actor_user_id: Some(acteur.to_string()),
            })
            .await?;
        self.preremplir_appel_tx(&mut tx, etablissement_id, appel.id).await?;
        tx.commit().await?;
        self.diffuser_apres_commit(&[ev]);
        self.appel_v2_lu(etablissement_id, appel.id, acteur, role).await
    }

    // ── Lecture, actualisation, pré-remplissage ─────────────────────────────────────────────

    /// Lit l'appel pour l'écran de pointage. Tant qu'il n'est pas validé, l'**effectif** et le
    /// **pré-remplissage** sont recalculés (DA-19, §9.3). Le contrôle d'accès est celui de l'appel.
    pub async fn appel_v2_lu(
        &self,
        etablissement_id: Uuid,
        appel_id: Uuid,
        acteur: Uuid,
        role: &str,
    ) -> Result<AppelV2, EntDbError> {
        let appel = self.charger_appel(etablissement_id, appel_id).await?;
        self.autoriser_acces_appel_pub(&appel, acteur, role).await?;
        let meta = self.meta_appel(etablissement_id, appel_id).await?;
        if meta.etat == "en_cours" {
            self.actualiser_effectif(etablissement_id, appel_id, &meta).await?;
            let mut tx = TenantTx::begin(self.pool(), etablissement_id).await?;
            self.preremplir_appel_tx(&mut tx, etablissement_id, appel_id).await?;
            tx.commit().await?;
        }
        self.assembler_appel(etablissement_id, appel_id).await
    }

    async fn assembler_appel(&self, etablissement_id: Uuid, appel_id: Uuid) -> Result<AppelV2, EntDbError> {
        let meta = self.meta_appel(etablissement_id, appel_id).await?;
        let rows: Vec<(Uuid, String, Option<i32>, Option<String>, Option<Uuid>, i32, Option<String>, bool)> =
            sqlx::query_as(
                "SELECT l.eleve_id, l.etat, l.minutes_retard, l.lieu_code, l.situation_id, l.version,
                        s.nature,
                        (l.exclusion_cours_le IS NOT NULL AND l.arrivee_vs_confirmee_le IS NULL)
                   FROM ent_appel_ligne l
                   LEFT JOIN ent_asd_situation s
                     ON s.etablissement_id = l.etablissement_id AND s.id = l.situation_id
                  WHERE l.etablissement_id = $1 AND l.appel_id = $2
                  ORDER BY l.eleve_id",
            )
            .bind(etablissement_id)
            .bind(appel_id)
            .fetch_all(&mut *TenantTx::begin(self.pool(), etablissement_id).await.map_err(db)?)
            .await
            .map_err(db)?;
        let lignes: Vec<LigneAppelV2> = rows
            .into_iter()
            .map(|(eleve_id, etat, minutes, lieu, sit, version, nature, attente)| {
                let libelle = nature
                    .as_deref()
                    .map(|n| libelle_court(n, lieu.as_deref()).to_string())
                    .or_else(|| {
                        (etat == "localise" || etat == "attendu_ailleurs")
                            .then(|| libelle_court("localisation", lieu.as_deref()).to_string())
                    });
                LigneAppelV2 {
                    eleve_id,
                    etat,
                    minutes_retard: minutes,
                    lieu_code: lieu,
                    pre_remplie: sit.is_some(),
                    libelle_court: libelle,
                    version,
                    eleve_label: None,
                    arrivee_vs_attendue: attente,
                }
            })
            .collect();
        let recapitulatif = recapituler(&lignes);
        Ok(AppelV2 {
            id: appel_id,
            jour: meta.jour,
            classe: meta.classe,
            type_activite: meta.type_activite,
            etat: meta.etat,
            mode_pointage: meta.mode_pointage,
            heure_debut: meta.heure_debut,
            heure_fin: meta.heure_fin,
            creneau_id: meta.creneau_id,
            matiere_libelle: meta.matiere_libelle,
            delegue_a: meta.delegue_a,
            lignes,
            recapitulatif,
        })
    }

    /// Accès à un appel existant (voir `autoriser_acces_appel`), exposé au reste du crate.
    pub(crate) async fn autoriser_acces_appel_pub(
        &self,
        appel: &Appel,
        acteur: Uuid,
        role: &str,
    ) -> Result<(), EntDbError> {
        self.autoriser_acces_appel(appel, acteur, role).await
    }

    /// Ajoute à l'appel les élèves arrivés dans l'effectif depuis l'ouverture (DA-19). Ne retire
    /// jamais une ligne : un élève parti est traité par la vie scolaire (correction).
    async fn actualiser_effectif(
        &self,
        etablissement_id: Uuid,
        appel_id: Uuid,
        meta: &AppelMeta,
    ) -> Result<(), EntDbError> {
        let Some(creneau_id) = meta.creneau_id else {
            return Ok(());
        };
        let Some(creneau) = self.repo_edt().get(etablissement_id, creneau_id).await? else {
            return Ok(());
        };
        let effectif = self
            .effectif_cours(&creneau, meta.jour, meta.auteur_user_id, meta.effectif_complet)
            .await?;
        if effectif.is_empty() {
            return Ok(());
        }
        let etat = if meta.mode_pointage == "presents_seulement" { "non_pointe" } else { "present" };
        let mut tx = TenantTx::begin(self.pool(), etablissement_id).await?;
        let statut: Option<(String,)> = sqlx::query_as(
            "SELECT etat FROM ent_appel WHERE etablissement_id = $1 AND id = $2 FOR SHARE",
        )
        .bind(etablissement_id)
        .bind(appel_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        if statut.as_ref().map(|s| s.0.as_str()) != Some("en_cours") {
            return Ok(());
        }
        for e in &effectif {
            let ligne_id = Uuid::new_v4();
            let ajoutee: Option<(Uuid,)> = sqlx::query_as(
                "INSERT INTO ent_appel_ligne (id, etablissement_id, appel_id, eleve_id, etat)
                 VALUES ($1,$2,$3,$4,$5) ON CONFLICT (appel_id, eleve_id) DO NOTHING RETURNING id",
            )
            .bind(ligne_id)
            .bind(etablissement_id)
            .bind(appel_id)
            .bind(e.id)
            .bind(etat)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db)?;
            if let Some((id,)) = ajoutee {
                sqlx::query(
                    "INSERT INTO ent_appel_ligne_histo
                        (id, etablissement_id, appel_id, ligne_id, eleve_id, version, etat_avant, etat_apres, canal, motif_code)
                     VALUES ($1,$2,$3,$4,$5,1,NULL,$6,'systeme','saisie')",
                )
                .bind(Uuid::new_v4())
                .bind(etablissement_id)
                .bind(appel_id)
                .bind(id)
                .bind(e.id)
                .bind(etat)
                .execute(&mut *tx)
                .await
                .map_err(db)?;
            }
        }
        tx.commit().await?;
        Ok(())
    }

    /// Pré-remplit (ou défait le pré-remplissage de) les lignes que **personne n'a encore touchées**
    /// à partir des situations actives (§9.3) : une ligne saisie par un humain n'est jamais modifiée.
    pub(crate) async fn preremplir_appel_tx(
        &self,
        tx: &mut PgConnection,
        etablissement_id: Uuid,
        appel_id: Uuid,
    ) -> Result<(), EntDbError> {
        let sql = format!("SELECT {COLONNES_META} FROM ent_appel WHERE etablissement_id = $1 AND id = $2");
        let meta: Option<AppelMeta> = sqlx::query_as(&sql)
            .bind(etablissement_id)
            .bind(appel_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db)?;
        let Some(meta) = meta else { return Ok(()) };
        if meta.etat != "en_cours" {
            return Ok(());
        }
        let (debut, fin) = match (meta.heure_debut, meta.heure_fin) {
            (Some(d), Some(f)) => (d, f),
            _ => {
                let h: Option<(NaiveTime, NaiveTime)> = match meta.creneau_id {
                    Some(c) => sqlx::query_as(
                        "SELECT heure_debut, heure_fin FROM ent_edt_creneau WHERE etablissement_id = $1 AND id = $2",
                    )
                    .bind(etablissement_id)
                    .bind(c)
                    .fetch_optional(&mut *tx)
                    .await
                    .map_err(db)?,
                    None => None,
                };
                h.unwrap_or((NaiveTime::MIN, NaiveTime::from_hms_opt(23, 59, 59).unwrap_or_default()))
            }
        };
        // Lignes sans intervention humaine (modifie_par NULL), verrouillées.
        let lignes: Vec<(Uuid, Uuid, String, Option<Uuid>, Option<String>)> = sqlx::query_as(
            "SELECT id, eleve_id, etat, situation_id, lieu_code FROM ent_appel_ligne
              WHERE etablissement_id = $1 AND appel_id = $2 AND modifie_par IS NULL
              ORDER BY eleve_id FOR UPDATE",
        )
        .bind(etablissement_id)
        .bind(appel_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        if lignes.is_empty() {
            return Ok(());
        }
        let eleves: Vec<Uuid> = lignes.iter().map(|l| l.1).collect();
        let sql = format!(
            "SELECT {COLONNES_SITUATION} FROM ent_asd_situation
              WHERE etablissement_id = $1 AND etat = 'active' AND debut_jour <= $2 AND fin_jour >= $2
                AND eleve_id = ANY($3)"
        );
        let situations: Vec<SituationAsd> = sqlx::query_as(&sql)
            .bind(etablissement_id)
            .bind(meta.jour)
            .bind(&eleves)
            .fetch_all(&mut *tx)
            .await
            .map_err(db)?;
        let mut par_eleve: HashMap<Uuid, Vec<SituationAsd>> = HashMap::new();
        for s in situations {
            par_eleve.entry(s.eleve_id).or_default().push(s);
        }
        let defaut = if meta.mode_pointage == "presents_seulement" { LigneEtat::NonPointe } else { LigneEtat::Present };
        for (_ligne_id, eleve_id, etat, sit_actuelle, _lieu) in lignes {
            let vide = Vec::new();
            let propose = meilleure_situation(par_eleve.get(&eleve_id).unwrap_or(&vide), meta.jour, debut, fin)
                .map(|s| {
                    let (e, l) = etat_propose(s);
                    Propose { etat: e, lieu: l, situation_id: s.id }
                });
            let (cible_etat, cible_lieu, cible_sit) = match &propose {
                Some(p) => (p.etat.clone(), p.lieu, Some(p.situation_id)),
                None if sit_actuelle.is_some() => (defaut.clone(), None, None),
                None => continue,
            };
            // Une ligne déjà passée à la main (retard, présent…) sans situation n'est pas concernée :
            // on ne touche que les lignes en état initial ou déjà pré-remplies.
            let initiale = etat == "present" || etat == "non_pointe" || sit_actuelle.is_some();
            if !initiale || (etat == cible_etat.as_str() && sit_actuelle == cible_sit) {
                continue;
            }
            let w = EcritureLigne {
                etat: cible_etat,
                minutes: None,
                lieu: cible_lieu.map(str::to_string),
                canal: "systeme",
                motif_code: Some("situation"),
                motif_texte: None,
            };
            PgAppelRepo::ecrire_ligne_tx(tx, etablissement_id, appel_id, eleve_id, &w, None)
                .await
                .map_err(db)?;
            sqlx::query(
                "UPDATE ent_appel_ligne SET situation_id = $4
                  WHERE etablissement_id = $1 AND appel_id = $2 AND eleve_id = $3",
            )
            .bind(etablissement_id)
            .bind(appel_id)
            .bind(eleve_id)
            .bind(cible_sit)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        }
        Ok(())
    }

    // ── Pointage ────────────────────────────────────────────────────────────────────────────

    /// Pointe un élève (état, minutes, lieu) tant que l'appel est ouvert ; historisé, canal tracé.
    /// Un retard sans minutes reçoit les minutes écoulées depuis le début de la séance (AS-107).
    pub async fn pointer_ligne_v2(
        &self,
        etablissement_id: Uuid,
        appel_id: Uuid,
        eleve_id: Uuid,
        acteur: Uuid,
        role: &str,
        saisie: SaisieLigne,
        canal: &str,
    ) -> Result<LigneAppelV2, EntDbError> {
        let appel = self.charger_appel(etablissement_id, appel_id).await?;
        self.autoriser_acces_appel_pub(&appel, acteur, role).await?;
        if appel.statut != AppelStatut::Ouvert {
            return Err(EntDbError::AppelInvalide);
        }
        let canal = canal_valide(canal)?;
        let etat = etat_saisissable(&saisie.etat)?;
        valider_precisions(&etat, saisie.minutes, saisie.lieu_code.as_deref())?;
        let mut minutes = saisie.minutes;
        if etat == LigneEtat::Retard && minutes.is_none() {
            let meta = self.meta_appel(etablissement_id, appel_id).await?;
            let debut = match meta.heure_debut {
                Some(d) => Some(d),
                None => match meta.creneau_id {
                    Some(c) => self.repo_edt().get(etablissement_id, c).await?.map(|c| c.heure_debut),
                    None => None,
                },
            };
            let now = self.maintenant_etablissement(etablissement_id).await?;
            if let Some(d) = debut.filter(|_| now.date() == meta.jour) {
                let ecoule = (now.time() - d).num_minutes();
                minutes = Some(ecoule.clamp(1, 600) as i32);
            }
        }
        let w = EcritureLigne {
            etat,
            minutes,
            lieu: saisie.lieu_code,
            canal,
            motif_code: Some("saisie"),
            motif_texte: None,
        };
        let ecrit = self
            .repo_appels()
            .ecrire_ligne_si_ouvert(etablissement_id, appel_id, eleve_id, &w, Some(acteur))
            .await?;
        if !ecrit {
            return Err(EntDbError::AppelInvalide);
        }
        let v = self.assembler_appel(etablissement_id, appel_id).await?;
        v.lignes.into_iter().find(|l| l.eleve_id == eleve_id).ok_or_else(introuvable)
    }

    // ── Validation ──────────────────────────────────────────────────────────────────────────

    /// Valide l'appel (geste « Appel fait », §9.2) : récapitulatif et confirmation humaine ;
    /// en mode « présents seulement », les non pointés deviennent absents **avec confirmation** ;
    /// sinon ils sont présents. Absences, retards et événements entrent dans **une** transaction.
    pub async fn valider_appel_v2(
        &self,
        etablissement_id: Uuid,
        appel_id: Uuid,
        acteur: Uuid,
        role: &str,
        v: ValidationAppel,
    ) -> Result<AppelV2, EntDbError> {
        let appel = self.charger_appel(etablissement_id, appel_id).await?;
        self.autoriser_acces_appel_pub(&appel, acteur, role).await?;
        if appel.statut != AppelStatut::Ouvert {
            return Err(EntDbError::AppelInvalide);
        }
        let canal = canal_valide(v.canal.as_deref().unwrap_or("web"))?;
        let meta = self.meta_appel(etablissement_id, appel_id).await?;
        // Dernière actualisation (arrivée tardive dans l'effectif, déclaration de dernière minute).
        self.actualiser_effectif(etablissement_id, appel_id, &meta).await?;
        let mut tx = TenantTx::begin(self.pool(), etablissement_id).await?;
        self.preremplir_appel_tx(&mut tx, etablissement_id, appel_id).await?;
        let verrou: Option<(String,)> = sqlx::query_as(
            "SELECT statut FROM ent_appel WHERE etablissement_id = $1 AND id = $2 FOR UPDATE",
        )
        .bind(etablissement_id)
        .bind(appel_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        if verrou.as_ref().map(|s| s.0.as_str()) != Some("ouvert") {
            return Err(EntDbError::AppelInvalide);
        }
        let non_pointes: Vec<Uuid> = sqlx::query_scalar(
            "SELECT eleve_id FROM ent_appel_ligne
              WHERE etablissement_id = $1 AND appel_id = $2 AND etat = 'non_pointe' ORDER BY eleve_id FOR UPDATE",
        )
        .bind(etablissement_id)
        .bind(appel_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        if !non_pointes.is_empty() {
            let devient = if meta.mode_pointage == "presents_seulement" {
                if !v.confirme_non_pointes {
                    return Err(EntDbError::SaisieInvalide(
                        "confirmation requise : les élèves non pointés deviendront absents",
                    ));
                }
                LigneEtat::Absent
            } else {
                LigneEtat::Present
            };
            for e in non_pointes {
                let w = EcritureLigne {
                    etat: devient.clone(),
                    minutes: None,
                    lieu: None,
                    canal,
                    motif_code: Some("validation"),
                    motif_texte: None,
                };
                PgAppelRepo::ecrire_ligne_tx(&mut tx, etablissement_id, appel_id, e, &w, Some(acteur))
                    .await
                    .map_err(db)?;
            }
        }
        tx.commit().await?;
        self.clore_appel_canal(etablissement_id, appel_id, acteur, role, Some(acteur.to_string()), canal)
            .await?;
        self.assembler_appel(etablissement_id, appel_id).await
    }

    // ── Historique et corrections ───────────────────────────────────────────────────────────

    /// Historique d'une ligne : l'intervenant (ses appels), la vie scolaire et la direction (AS-130).
    pub async fn historique_ligne_for_actor(
        &self,
        etablissement_id: Uuid,
        appel_id: Uuid,
        eleve_id: Uuid,
        acteur: Uuid,
        role: &str,
    ) -> Result<Vec<HistoLigne>, EntDbError> {
        let appel = self.charger_appel(etablissement_id, appel_id).await?;
        self.autoriser_acces_appel_pub(&appel, acteur, role).await?;
        sqlx::query_as::<_, HistoLigne>(
            "SELECT version, etat_avant, etat_apres, minutes, lieu_code, auteur_id, canal, motif_code,
                    motif_texte, cree_le
               FROM ent_appel_ligne_histo
              WHERE etablissement_id = $1 AND appel_id = $2 AND eleve_id = $3
              ORDER BY version, cree_le",
        )
        .bind(etablissement_id)
        .bind(appel_id)
        .bind(eleve_id)
        .fetch_all(&mut *TenantTx::begin(self.pool(), etablissement_id).await.map_err(db)?)
        .await
        .map_err(db)
    }

    /// **Corrige** une ligne d'un appel validé (AS-131) : l'intervenant le jour même (motif
    /// facultatif) ; la vie scolaire (`appel.corriger`) à tout moment, **motif obligatoire**. Les
    /// effets sont recalculés dans la même transaction (absence créée ou retirée, retard) et un
    /// **démenti** est émis si l'absence disparaît (AS-136). Rien n'est réécrit en silence.
    pub async fn corriger_ligne_for_actor(
        &self,
        etablissement_id: Uuid,
        appel_id: Uuid,
        eleve_id: Uuid,
        acteur: Uuid,
        role: &str,
        c: CorrectionLigne,
        canal: &str,
    ) -> Result<LigneAppelV2, EntDbError> {
        let appel = self.charger_appel(etablissement_id, appel_id).await?;
        let meta = self.meta_appel(etablissement_id, appel_id).await?;
        if meta.etat == "neutralise" {
            return Err(EntDbError::AppelInvalide);
        }
        let canal = canal_valide(canal)?;
        let etat = etat_saisissable(&c.etat)?;
        valider_precisions(&etat, c.minutes, c.lieu_code.as_deref())?;
        if appel.statut == AppelStatut::Ouvert {
            // Pas encore validé : c'est un simple pointage.
            return self
                .pointer_ligne_v2(
                    etablissement_id,
                    appel_id,
                    eleve_id,
                    acteur,
                    role,
                    SaisieLigne { etat: c.etat, minutes: c.minutes, lieu_code: c.lieu_code },
                    canal,
                )
                .await;
        }
        let corrige_tout = self
            .a_action_etablissement(etablissement_id, acteur, role, Action::AppelCorriger)
            .await?;
        let texte = c.motif_texte.as_deref().map(str::trim).filter(|t| !t.is_empty());
        if let Some(t) = texte {
            if t.chars().count() > 200 {
                return Err(EntDbError::SaisieInvalide("motif trop long (200 caractères)"));
            }
        }
        let motif_code: &'static str = match c.motif_code.as_deref() {
            None => if corrige_tout { "correction_vie_scolaire" } else { "erreur_saisie" },
            Some("erreur_saisie") => "erreur_saisie",
            Some("billet_entree") => "billet_entree",
            Some("oubli") => "oubli",
            Some("correction_vie_scolaire") => "correction_vie_scolaire",
            Some(_) => return Err(EntDbError::SaisieInvalide("code de motif de correction inconnu")),
        };
        if corrige_tout {
            // Vie scolaire : à tout moment, mais jamais sans motif (R-AS8).
            if texte.is_none() && c.motif_code.is_none() {
                return Err(EntDbError::SaisieInvalide("motif obligatoire pour corriger un appel validé"));
            }
        } else {
            // Intervenant : le jour même seulement ; au-delà, il demande à la vie scolaire.
            self.autoriser_acces_appel_pub(&appel, acteur, role).await?;
            let aujourdhui = self.aujourdhui_etablissement(etablissement_id).await?;
            if appel.jour != aujourdhui {
                return Err(EntDbError::Forbidden);
            }
        }
        let mut minutes = c.minutes;
        if etat == LigneEtat::Retard && minutes.is_none() {
            minutes = Some(1);
        }
        let mut tx = TenantTx::begin(self.pool(), etablissement_id).await?;
        let verrou: Option<(String, String)> = sqlx::query_as(
            "SELECT statut, etat FROM ent_appel WHERE etablissement_id = $1 AND id = $2 FOR UPDATE",
        )
        .bind(etablissement_id)
        .bind(appel_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        if verrou.as_ref().map(|s| (s.0.as_str(), s.1.as_str())) != Some(("clos", "valide")) {
            return Err(EntDbError::AppelInvalide);
        }
        let avant: Option<(String, i32)> = sqlx::query_as(
            "SELECT etat, version FROM ent_appel_ligne
              WHERE etablissement_id = $1 AND appel_id = $2 AND eleve_id = $3 FOR UPDATE",
        )
        .bind(etablissement_id)
        .bind(appel_id)
        .bind(eleve_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let Some((etat_avant, version)) = avant else {
            return Err(introuvable());
        };
        if c.version_base.is_some_and(|v| v != version) {
            return Err(EntDbError::ConflitDossier("la ligne a été modifiée depuis votre lecture"));
        }
        let avant_etat = LigneEtat::parse(&etat_avant).unwrap_or(LigneEtat::Present);
        let w = EcritureLigne {
            etat: etat.clone(),
            minutes,
            lieu: c.lieu_code.clone(),
            canal,
            motif_code: Some(motif_code),
            motif_texte: texte.map(str::to_string),
        };
        PgAppelRepo::ecrire_ligne_tx(&mut tx, etablissement_id, appel_id, eleve_id, &w, Some(acteur))
            .await
            .map_err(db)?;
        // Effets : retrait ou création de l'absence, du retard ; démenti si l'absence disparaît.
        let actor = Some(acteur.to_string());
        let mut publies: Vec<ent_events::MetierEvent> = Vec::new();
        let ev = tx
            .publier(MetierEventDraft {
                etablissement_id,
                type_name: types_appel::LIGNE_CORRIGEE.to_string(),
                aggregate: AggregateKind::Appel,
                aggregate_id: appel_id,
                eleve_id: Some(eleve_id),
                payload: json!({
                    "appel_id": appel_id,
                    "avant": etat_avant,
                    "apres": etat.as_str(),
                    "motif": motif_code,
                    "canal": canal,
                }),
                actor_user_id: actor.clone(),
            })
            .await?;
        let suite = Causalite::suite_de(&ev, Causalite::default());
        publies.push(ev);
        if avant_etat == LigneEtat::Retard && etat != LigneEtat::Retard {
            self.retirer_retard_tx(&mut tx, etablissement_id, appel_id, eleve_id).await?;
        }
        if avant_etat == LigneEtat::Absent && etat != LigneEtat::Absent {
            self.retirer_absence_appel_tx(&mut tx, &appel, eleve_id, &actor, suite, &mut publies, "correction").await?;
        }
        if etat == LigneEtat::Absent && avant_etat != LigneEtat::Absent {
            self.appliquer_absent_tx(&mut tx, &appel, eleve_id, &actor, suite, &mut publies).await?;
        }
        if etat == LigneEtat::Retard && avant_etat != LigneEtat::Retard {
            self.appliquer_retard_tx(&mut tx, &appel, eleve_id, minutes, &actor, suite, &mut publies).await?;
        }
        tx.commit().await?;
        self.diffuser_apres_commit(&publies);
        if etat == LigneEtat::Absent && avant_etat != LigneEtat::Absent {
            self.apres_absence_ecrite(etablissement_id, eleve_id, appel.jour).await?;
        }
        if (etat == LigneEtat::Absent) != (avant_etat == LigneEtat::Absent) {
            // AS4 : une correction qui touche un mois arrêté s'inscrit en rectification (R-AS16).
            self.rectifier_apres_changement(etablissement_id, eleve_id, appel.jour, appel.jour, "correction_appel", Some(appel_id)).await;
        }
        let v = self.assembler_appel(etablissement_id, appel_id).await?;
        v.lignes.into_iter().find(|l| l.eleve_id == eleve_id).ok_or_else(introuvable)
    }

    async fn retirer_retard_tx(
        &self,
        tx: &mut PgConnection,
        etablissement_id: Uuid,
        appel_id: Uuid,
        eleve_id: Uuid,
    ) -> Result<(), EntDbError> {
        sqlx::query(
            "DELETE FROM ent_retard WHERE etablissement_id = $1 AND source_appel_id = $2 AND eleve_id = $3",
        )
        .bind(etablissement_id)
        .bind(appel_id)
        .bind(eleve_id)
        .execute(tx)
        .await
        .map_err(db)?;
        Ok(())
    }

    /// Défait les effets d'un constat « absent » (correction, neutralisation) : retire l'absence à
    /// la séance ; si l'épisode du jour n'a plus aucune séance, l'absence est « annulée présent » ;
    /// un **démenti** est publié (AS-136). Les décisions déjà prises sur l'absence restent dans son
    /// historique : seul son statut final change.
    pub(crate) async fn retirer_absence_appel_tx(
        &self,
        tx: &mut PgConnection,
        appel: &Appel,
        eleve_id: Uuid,
        actor_user_id: &Option<String>,
        suite: Causalite,
        publies: &mut Vec<ent_events::MetierEvent>,
        cause: &'static str,
    ) -> Result<(), EntDbError> {
        let etablissement_id = appel.etablissement_id;
        // AS3 : les séances de l'épisode sont retirées (jamais supprimées) avant l'ancien circuit.
        self.episode_retirer_appel_tx(
            tx,
            etablissement_id,
            appel.id,
            eleve_id,
            cause,
            actor_user_id.as_deref().and_then(|s| Uuid::parse_str(s).ok()),
        )
        .await?;
        let touchees: Vec<(Uuid,)> = sqlx::query_as(
            "DELETE FROM ent_absence_seance
              WHERE etablissement_id = $1 AND appel_id = $2 AND eleve_id = $3 AND kind = 'seance'
              RETURNING absence_id",
        )
        .bind(etablissement_id)
        .bind(appel.id)
        .bind(eleve_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let mut vues: Vec<Uuid> = touchees.into_iter().map(|(a,)| a).collect();
        vues.sort();
        vues.dedup();
        for absence_id in vues {
            let (reste,): (bool,) = sqlx::query_as(
                "SELECT EXISTS (SELECT 1 FROM ent_absence_seance WHERE etablissement_id = $1 AND absence_id = $2)",
            )
            .bind(etablissement_id)
            .bind(absence_id)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
            if reste {
                continue;
            }
            let r = sqlx::query(
                "UPDATE ent_absence SET statut = 'annulee_present'
                  WHERE etablissement_id = $1 AND id = $2 AND statut <> 'annulee_present'",
            )
            .bind(etablissement_id)
            .bind(absence_id)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
            if r.rows_affected() > 0 {
                publies.push(
                    tx.publier_avec(
                        MetierEventDraft {
                            etablissement_id,
                            type_name: ent_events::types::ELEVE_ABSENCE_ANNULEE_PRESENT.to_string(),
                            aggregate: AggregateKind::Absence,
                            aggregate_id: absence_id,
                            eleve_id: Some(eleve_id),
                            payload: json!({ "statut": "annulee_present", "source": "correction_appel" }),
                            actor_user_id: actor_user_id.clone(),
                        },
                        suite,
                    )
                    .await?,
                );
            }
        }
        publies.push(
            tx.publier_avec(
                MetierEventDraft {
                    etablissement_id,
                    type_name: types_appel::ABSENCE_DEMENTIE.to_string(),
                    aggregate: AggregateKind::Absence,
                    aggregate_id: appel.id,
                    eleve_id: Some(eleve_id),
                    payload: json!({ "appel_id": appel.id, "jour": appel.jour.to_string() }),
                    actor_user_id: actor_user_id.clone(),
                },
                suite,
            )
            .await?,
        );
        Ok(())
    }

    // ── Neutralisation (séance annulée après l'appel) ───────────────────────────────────────

    /// Neutralise l'appel d'une séance **annulée après coup** (DA-09, AS-135, R-AS12) : les
    /// constats cessent de compter (absences retirées, retards supprimés), un démenti est publié par
    /// élève concerné, les lignes gardent leur historique. Sans appel, ne fait rien. Idempotent.
    pub async fn neutraliser_appel_seance(
        &self,
        etablissement_id: Uuid,
        creneau_id: Uuid,
        jour: NaiveDate,
        cause: &str,
        acteur: Option<Uuid>,
    ) -> Result<bool, EntDbError> {
        if !matches!(cause, "annulation_seance" | "fermeture" | "greve") {
            return Err(EntDbError::SaisieInvalide("cause de neutralisation inconnue"));
        }
        let mut tx = TenantTx::begin(self.pool(), etablissement_id).await?;
        let cible: Option<(Uuid, String, String)> = sqlx::query_as(
            "SELECT id, etat, classe FROM ent_appel
              WHERE etablissement_id = $1 AND creneau_id = $2 AND jour = $3 FOR UPDATE",
        )
        .bind(etablissement_id)
        .bind(creneau_id)
        .bind(jour)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let Some((appel_id, etat, classe)) = cible else {
            return Ok(false);
        };
        if etat == "neutralise" {
            return Ok(false);
        }
        let appel = self.charger_appel(etablissement_id, appel_id).await.or_else(|_| {
            Ok::<_, EntDbError>(Appel {
                id: appel_id,
                etablissement_id,
                classe,
                jour,
                auteur_user_id: acteur.unwrap_or_default(),
                statut: AppelStatut::Clos,
                creneau_id: Some(creneau_id),
                matiere_libelle: None,
            })
        })?;
        let actor = acteur.map(|a| a.to_string());
        let mut publies: Vec<ent_events::MetierEvent> = Vec::new();
        let ev = tx
            .publier(MetierEventDraft {
                etablissement_id,
                type_name: types_appel::NEUTRALISE.to_string(),
                aggregate: AggregateKind::Appel,
                aggregate_id: appel_id,
                eleve_id: None,
                payload: json!({ "appel_id": appel_id, "cause": cause, "jour": jour.to_string() }),
                actor_user_id: actor.clone(),
            })
            .await?;
        let suite = Causalite::suite_de(&ev, Causalite::default());
        publies.push(ev);
        let mut touches_decompte: Vec<Uuid> = Vec::new();
        if etat == "valide" {
            let lignes: Vec<(Uuid, Uuid, String)> = sqlx::query_as(
                "SELECT id, eleve_id, etat FROM ent_appel_ligne
                  WHERE etablissement_id = $1 AND appel_id = $2 AND etat IN ('absent', 'retard')
                  ORDER BY eleve_id FOR UPDATE",
            )
            .bind(etablissement_id)
            .bind(appel_id)
            .fetch_all(&mut *tx)
            .await
            .map_err(db)?;
            for (ligne_id, eleve_id, e) in lignes {
                if e == "absent" {
                    touches_decompte.push(eleve_id);
                    self.retirer_absence_appel_tx(&mut tx, &appel, eleve_id, &actor, suite, &mut publies, "neutralisation").await?;
                } else {
                    self.retirer_retard_tx(&mut tx, etablissement_id, appel_id, eleve_id).await?;
                }
                sqlx::query(
                    "INSERT INTO ent_appel_ligne_histo
                        (id, etablissement_id, appel_id, ligne_id, eleve_id, version, etat_avant, etat_apres,
                         auteur_id, canal, motif_code)
                     SELECT $1, $2, $3, l.id, l.eleve_id, l.version, l.etat, l.etat, $4, 'systeme', 'neutralisation'
                       FROM ent_appel_ligne l WHERE l.etablissement_id = $2 AND l.id = $5",
                )
                .bind(Uuid::new_v4())
                .bind(etablissement_id)
                .bind(appel_id)
                .bind(acteur)
                .bind(ligne_id)
                .execute(&mut *tx)
                .await
                .map_err(db)?;
            }
        }
        sqlx::query(
            "UPDATE ent_appel SET etat = 'neutralise', statut = 'clos', neutralise_le = now(),
                    neutralise_cause = $3, modifie_le = now()
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etablissement_id)
        .bind(appel_id)
        .bind(cause)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await?;
        self.diffuser_apres_commit(&publies);
        // AS4 : un mois déjà arrêté n'est pas réécrit, l'écart est inscrit en rectification.
        for e in touches_decompte {
            self.rectifier_apres_changement(etablissement_id, e, appel.jour, appel.jour, "neutralisation", Some(appel_id)).await;
        }
        Ok(true)
    }

    // ── Délégation (AS-105) ─────────────────────────────────────────────────────────────────

    /// Délègue l'appel d'une séance à un collègue, pour ce jour : l'intervenant (`appel.deleguer`).
    /// Une seule délégation active par appel ; visible de la vie scolaire.
    pub async fn deleguer_appel_for_actor(
        &self,
        etablissement_id: Uuid,
        appel_id: Uuid,
        acteur: Uuid,
        role: &str,
        a_user: Uuid,
        motif: Option<&str>,
    ) -> Result<(), EntDbError> {
        self.exiger_action_etab(etablissement_id, acteur, role, Action::AppelDeleguer).await?;
        let appel = self.charger_appel(etablissement_id, appel_id).await?;
        self.autoriser_acces_appel_pub(&appel, acteur, role).await?;
        if appel.statut != AppelStatut::Ouvert || a_user == acteur {
            return Err(EntDbError::AppelInvalide);
        }
        let motif = motif.map(str::trim).filter(|m| !m.is_empty());
        if motif.is_some_and(|m| m.chars().count() > 200) {
            return Err(EntDbError::SaisieInvalide("motif trop long (200 caractères)"));
        }
        // Le délégué est un membre de l'établissement (jamais une personne d'un autre).
        if !self.repo_auth().is_member(etablissement_id, a_user).await? {
            return Err(introuvable());
        }
        let mut tx = TenantTx::begin(self.pool(), etablissement_id).await?;
        sqlx::query(
            "UPDATE ent_appel_delegation SET revoquee_le = now()
              WHERE etablissement_id = $1 AND appel_id = $2 AND revoquee_le IS NULL",
        )
        .bind(etablissement_id)
        .bind(appel_id)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        sqlx::query(
            "INSERT INTO ent_appel_delegation (id, etablissement_id, appel_id, de_user, a_user, motif)
             VALUES ($1,$2,$3,$4,$5,$6)",
        )
        .bind(Uuid::new_v4())
        .bind(etablissement_id)
        .bind(appel_id)
        .bind(acteur)
        .bind(a_user)
        .bind(motif)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        sqlx::query("UPDATE ent_appel SET delegue_a = $3, modifie_le = now() WHERE etablissement_id = $1 AND id = $2")
            .bind(etablissement_id)
            .bind(appel_id)
            .bind(a_user)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        let ev = tx
            .publier(MetierEventDraft {
                etablissement_id,
                type_name: types_appel::DELEGUE.to_string(),
                aggregate: AggregateKind::Appel,
                aggregate_id: appel_id,
                eleve_id: None,
                payload: json!({ "appel_id": appel_id, "a_user": a_user }),
                actor_user_id: Some(acteur.to_string()),
            })
            .await?;
        tx.commit().await?;
        self.diffuser_apres_commit(&[ev]);
        Ok(())
    }

    /// Révoque la délégation active : l'intervenant, ou la supervision.
    pub async fn revoquer_delegation_for_actor(
        &self,
        etablissement_id: Uuid,
        appel_id: Uuid,
        acteur: Uuid,
        role: &str,
    ) -> Result<(), EntDbError> {
        let appel = self.charger_appel(etablissement_id, appel_id).await?;
        let d = self.droits_appel(etablissement_id, acteur, role).await?;
        if !d.tout {
            // L'intervenant d'origine seulement (le délégué ne révoque pas sa propre délégation).
            self.exiger_action_etab(etablissement_id, acteur, role, Action::AppelDeleguer).await?;
            let (est_delegant,): (bool,) = sqlx::query_as(
                "SELECT EXISTS (SELECT 1 FROM ent_appel_delegation
                                 WHERE etablissement_id = $1 AND appel_id = $2 AND de_user = $3 AND revoquee_le IS NULL)",
            )
            .bind(etablissement_id)
            .bind(appel_id)
            .bind(acteur)
            .fetch_one(&mut *TenantTx::begin(self.pool(), etablissement_id).await.map_err(db)?)
            .await
            .map_err(db)?;
            if !est_delegant {
                return Err(EntDbError::Forbidden);
            }
        }
        let mut tx = TenantTx::begin(self.pool(), etablissement_id).await?;
        sqlx::query(
            "UPDATE ent_appel_delegation SET revoquee_le = now()
              WHERE etablissement_id = $1 AND appel_id = $2 AND revoquee_le IS NULL",
        )
        .bind(etablissement_id)
        .bind(appel_id)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        sqlx::query("UPDATE ent_appel SET delegue_a = NULL, modifie_le = now() WHERE etablissement_id = $1 AND id = $2")
            .bind(etablissement_id)
            .bind(appel.id)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        let ev = tx
            .publier(MetierEventDraft {
                etablissement_id,
                type_name: types_appel::DELEGATION_REVOQUEE.to_string(),
                aggregate: AggregateKind::Appel,
                aggregate_id: appel_id,
                eleve_id: None,
                payload: json!({ "appel_id": appel_id }),
                actor_user_id: Some(acteur.to_string()),
            })
            .await?;
        tx.commit().await?;
        self.diffuser_apres_commit(&[ev]);
        Ok(())
    }

    // ── Exclusion ponctuelle de cours (R-AS37, AS-208) ──────────────────────────────────────

    /// Le professeur envoie un élève à la vie scolaire : la ligne devient « localisé, vie scolaire »
    /// à l'heure du geste ; l'élève n'est **jamais** « absent » tant qu'il n'est pas constaté hors de
    /// l'établissement ; son arrivée doit être confirmée (`confirmer_arrivee_vs`). La punition elle-même
    /// reste au socle 13.
    pub async fn exclure_du_cours_for_actor(
        &self,
        etablissement_id: Uuid,
        appel_id: Uuid,
        eleve_id: Uuid,
        acteur: Uuid,
        role: &str,
        canal: &str,
    ) -> Result<LigneAppelV2, EntDbError> {
        let canal = canal_valide(canal)?;
        let appel = self.charger_appel(etablissement_id, appel_id).await?;
        self.autoriser_acces_appel_pub(&appel, acteur, role).await?;
        let meta = self.meta_appel(etablissement_id, appel_id).await?;
        if meta.etat == "neutralise" {
            return Err(EntDbError::AppelInvalide);
        }
        let mut tx = TenantTx::begin(self.pool(), etablissement_id).await?;
        let verrou: Option<(String,)> = sqlx::query_as(
            "SELECT etat FROM ent_appel WHERE etablissement_id = $1 AND id = $2 FOR SHARE",
        )
        .bind(etablissement_id)
        .bind(appel_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        if verrou.is_none() {
            return Err(EntDbError::AppelInvalide);
        }
        let w = EcritureLigne {
            etat: LigneEtat::Localise,
            minutes: None,
            lieu: Some("vie_scolaire".into()),
            canal,
            motif_code: Some("exclusion_cours"),
            motif_texte: None,
        };
        let ok = PgAppelRepo::ecrire_ligne_tx(&mut tx, etablissement_id, appel_id, eleve_id, &w, Some(acteur))
            .await
            .map_err(db)?;
        if !ok {
            return Err(introuvable());
        }
        sqlx::query(
            "UPDATE ent_appel_ligne SET exclusion_cours_le = now(), arrivee_vs_confirmee_le = NULL, arrivee_vs_par = NULL
              WHERE etablissement_id = $1 AND appel_id = $2 AND eleve_id = $3",
        )
        .bind(etablissement_id)
        .bind(appel_id)
        .bind(eleve_id)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        let ev = tx
            .publier(MetierEventDraft {
                etablissement_id,
                type_name: types_appel::EXCLUSION_COURS.to_string(),
                aggregate: AggregateKind::Appel,
                aggregate_id: appel_id,
                eleve_id: Some(eleve_id),
                payload: json!({ "appel_id": appel_id }),
                actor_user_id: Some(acteur.to_string()),
            })
            .await?;
        // Si l'appel est déjà validé, l'ancien constat « absent » est défait comme une correction.
        tx.commit().await?;
        self.diffuser_apres_commit(&[ev]);
        let v = self.assembler_appel(etablissement_id, appel_id).await?;
        v.lignes.into_iter().find(|l| l.eleve_id == eleve_id).ok_or_else(introuvable)
    }

    /// La vie scolaire confirme l'**arrivée** de l'élève exclu (`localisation.poser`).
    pub async fn confirmer_arrivee_vs_for_actor(
        &self,
        etablissement_id: Uuid,
        appel_id: Uuid,
        eleve_id: Uuid,
        acteur: Uuid,
        role: &str,
    ) -> Result<(), EntDbError> {
        self.exiger_action_etab(etablissement_id, acteur, role, Action::LocalisationPoser).await?;
        let mut tx = TenantTx::begin(self.pool(), etablissement_id).await?;
        let r = sqlx::query(
            "UPDATE ent_appel_ligne SET arrivee_vs_confirmee_le = now(), arrivee_vs_par = $4
              WHERE etablissement_id = $1 AND appel_id = $2 AND eleve_id = $3
                AND exclusion_cours_le IS NOT NULL AND arrivee_vs_confirmee_le IS NULL",
        )
        .bind(etablissement_id)
        .bind(appel_id)
        .bind(eleve_id)
        .bind(acteur)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        if r.rows_affected() == 0 {
            return Err(introuvable());
        }
        let ev = tx
            .publier(MetierEventDraft {
                etablissement_id,
                type_name: types_appel::ARRIVEE_VS_CONFIRMEE.to_string(),
                aggregate: AggregateKind::Appel,
                aggregate_id: appel_id,
                eleve_id: Some(eleve_id),
                payload: json!({ "appel_id": appel_id }),
                actor_user_id: Some(acteur.to_string()),
            })
            .await?;
        tx.commit().await?;
        self.diffuser_apres_commit(&[ev]);
        Ok(())
    }

    // ── Incohérences (§9.7) : jamais de modification automatique ────────────────────────────

    /// Incohérences d'un appel : présent malgré une déclaration, présent alors qu'« en sortie »,
    /// absent à une séance puis présent à une séance suivante du jour, exclu sans arrivée
    /// confirmée. Lecture pour qui peut vérifier les appels ou en est l'intervenant.
    pub async fn incoherences_appel_for_actor(
        &self,
        etablissement_id: Uuid,
        appel_id: Uuid,
        acteur: Uuid,
        role: &str,
    ) -> Result<Vec<IncoherenceV2>, EntDbError> {
        let appel = self.charger_appel(etablissement_id, appel_id).await?;
        if !self.a_action_etablissement(etablissement_id, acteur, role, Action::AppelVerifier).await? {
            self.autoriser_acces_appel_pub(&appel, acteur, role).await?;
        }
        let mut out = Vec::new();
        let rows: Vec<(Uuid, String, Option<String>, Option<String>)> = sqlx::query_as(
            "SELECT l.eleve_id, l.etat, s.nature, s.lieu_code
               FROM ent_appel_ligne l
               JOIN ent_asd_situation s ON s.etablissement_id = l.etablissement_id AND s.id = l.situation_id
              WHERE l.etablissement_id = $1 AND l.appel_id = $2 AND l.modifie_par IS NOT NULL
              ORDER BY l.eleve_id",
        )
        .bind(etablissement_id)
        .bind(appel_id)
        .fetch_all(&mut *TenantTx::begin(self.pool(), etablissement_id).await.map_err(db)?)
        .await
        .map_err(db)?;
        for (eleve_id, etat, nature, _lieu) in rows {
            let presente = matches!(etat.as_str(), "present" | "retard" | "present_dispense");
            if !presente {
                continue;
            }
            match nature.as_deref() {
                Some("sortie") | Some("sortie_anticipee") => {
                    out.push(IncoherenceV2 { eleve_id, code: "present_alors_quen_sortie".into() })
                }
                Some("declaration") | Some("autorisation") | Some("retard_annonce") => {
                    out.push(IncoherenceV2 { eleve_id, code: "present_malgre_declaration".into() })
                }
                _ => {}
            }
        }
        let exclus: Vec<(Uuid,)> = sqlx::query_as(
            "SELECT eleve_id FROM ent_appel_ligne
              WHERE etablissement_id = $1 AND appel_id = $2
                AND exclusion_cours_le IS NOT NULL AND arrivee_vs_confirmee_le IS NULL
              ORDER BY eleve_id",
        )
        .bind(etablissement_id)
        .bind(appel_id)
        .fetch_all(&mut *TenantTx::begin(self.pool(), etablissement_id).await.map_err(db)?)
        .await
        .map_err(db)?;
        out.extend(exclus.into_iter().map(|(eleve_id,)| IncoherenceV2 {
            eleve_id,
            code: "exclusion_sans_arrivee".into(),
        }));
        // Absent à cette séance, présent à une séance qui commence plus tard le même jour.
        let suites: Vec<(Uuid,)> = sqlx::query_as(
            "SELECT DISTINCT a.eleve_id
               FROM ent_appel_ligne a
               JOIN ent_appel pa ON pa.etablissement_id = a.etablissement_id AND pa.id = a.appel_id
               JOIN ent_appel_ligne b ON b.etablissement_id = a.etablissement_id AND b.eleve_id = a.eleve_id
               JOIN ent_appel pb ON pb.etablissement_id = b.etablissement_id AND pb.id = b.appel_id
              WHERE a.etablissement_id = $1 AND a.appel_id = $2 AND a.etat = 'absent'
                AND pb.jour = pa.jour AND pb.id <> pa.id AND pb.etat <> 'neutralise'
                AND pb.heure_debut IS NOT NULL AND pa.heure_debut IS NOT NULL
                AND pb.heure_debut > pa.heure_debut
                AND b.etat IN ('present', 'retard', 'present_dispense')
              ORDER BY a.eleve_id",
        )
        .bind(etablissement_id)
        .bind(appel_id)
        .fetch_all(&mut *TenantTx::begin(self.pool(), etablissement_id).await.map_err(db)?)
        .await
        .map_err(db)?;
        out.extend(suites.into_iter().map(|(eleve_id,)| IncoherenceV2 {
            eleve_id,
            code: "absent_puis_present".into(),
        }));
        Ok(out)
    }

    // ── Situations connues ──────────────────────────────────────────────────────────────────

    /// Reflet d'une déclaration d'absence en situation connue (même transaction que la déclaration).
    /// Le texte libre est séparé (R-AS22) et borné à 500 caractères.
    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn miroir_declaration_situation_tx(
        tx: &mut PgConnection,
        etablissement_id: Uuid,
        declaration_id: Uuid,
        eleve_id: Uuid,
        source: &str,
        debut_jour: NaiveDate,
        debut_heure: Option<NaiveTime>,
        fin_jour: NaiveDate,
        fin_heure: Option<NaiveTime>,
        motif: &str,
        commentaire: Option<&str>,
        declarant_nom: Option<&str>,
        declarant_qualite: Option<&str>,
        acteur: Uuid,
    ) -> Result<(), EntDbError> {
        let situation_id = Uuid::new_v4();
        let source_asd = if source == "famille" { "famille" } else { "accueil" };
        let r = sqlx::query(
            "INSERT INTO ent_asd_situation
                (id, etablissement_id, eleve_id, nature, debut_jour, debut_heure, fin_jour, fin_heure, source,
                 origine_ref_type, origine_ref_id, motif_id, declarant_nom, declarant_qualite, declarant_verifie,
                 canal, cree_par)
             VALUES ($1,$2,$3,'declaration',$4,$5,$6,$7,$8,'ent_absence_declaration',$9,
                     (SELECT m.id FROM ent_asd_motif m WHERE m.etablissement_id = $2 AND m.code = $10),
                     $11,$12,$13,$14,$15)
             ON CONFLICT DO NOTHING",
        )
        .bind(situation_id)
        .bind(etablissement_id)
        .bind(eleve_id)
        .bind(debut_jour)
        .bind(debut_heure)
        .bind(fin_jour)
        .bind(fin_heure)
        .bind(source_asd)
        .bind(declaration_id)
        .bind(motif)
        .bind(declarant_nom)
        .bind(declarant_qualite)
        .bind(source == "famille")
        .bind(if source == "famille" { "web" } else { "telephone" })
        .bind(acteur)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        if r.rows_affected() == 0 {
            return Ok(());
        }
        if let Some(t) = commentaire.map(str::trim).filter(|t| !t.is_empty()) {
            let t: String = t.chars().take(500).collect();
            sqlx::query(
                "INSERT INTO ent_asd_situation_texte (situation_id, etablissement_id, auteur_user_id, texte)
                 VALUES ($1,$2,$3,$4)",
            )
            .bind(situation_id)
            .bind(etablissement_id)
            .bind(acteur)
            .bind(t)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        }
        Ok(())
    }

    /// Pose une **localisation** (infirmerie, vie scolaire, CDI, entretien) : action
    /// `localisation.poser`, lieu codé sans motif (R-AS20). Pré-remplit les appels ouverts du jour.
    pub async fn poser_localisation_for_actor(
        &self,
        etablissement_id: Uuid,
        acteur: Uuid,
        role: &str,
        p: PoserLocalisation,
    ) -> Result<SituationAsd, EntDbError> {
        self.exiger_action_etab(etablissement_id, acteur, role, Action::LocalisationPoser).await?;
        if !LIEUX_LOCALISATION.contains(&p.lieu_code.as_str()) {
            return Err(EntDbError::SaisieInvalide("lieu de localisation inconnu"));
        }
        self.exiger_eleve_existe(etablissement_id, p.eleve_id).await?;
        let jour = match p.jour {
            Some(j) => j,
            None => self.aujourdhui_etablissement(etablissement_id).await?,
        };
        if let (Some(d), Some(f)) = (p.heure_debut, p.heure_fin) {
            if f <= d {
                return Err(EntDbError::SaisieInvalide("heures de localisation incohérentes"));
            }
        }
        let s = self
            .creer_situation(
                etablissement_id,
                acteur,
                p.eleve_id,
                "localisation",
                jour,
                p.heure_debut,
                p.heure_fin,
                None,
                "vie_scolaire",
                Some(&p.lieu_code),
                None,
                types_appel::LOCALISE,
            )
            .await?;
        self.rafraichir_appels_ouverts_eleve(etablissement_id, p.eleve_id, jour).await?;
        Ok(s)
    }

    /// « Il arrive à 10 h » (AS-218) : action `absences.declarer` sur cet élève (accueil, vie scolaire).
    pub async fn annoncer_retard_for_actor(
        &self,
        etablissement_id: Uuid,
        acteur: Uuid,
        role: &str,
        a: AnnoncerRetard,
    ) -> Result<SituationAsd, EntDbError> {
        self.exiger_action_eleve(etablissement_id, acteur, role, Action::AbsencesDeclarer, a.eleve_id).await?;
        let jour = match a.jour {
            Some(j) => j,
            None => self.aujourdhui_etablissement(etablissement_id).await?,
        };
        let aujourdhui = self.aujourdhui_etablissement(etablissement_id).await?;
        if jour < aujourdhui || jour > aujourdhui + chrono::Duration::days(30) {
            return Err(EntDbError::SaisieInvalide("jour du retard annoncé hors fenêtre"));
        }
        if a.heure_arrivee_prevue == NaiveTime::MIN {
            return Err(EntDbError::SaisieInvalide("heure d'arrivée prévue invalide"));
        }
        let nom = a.declarant_nom.as_deref().map(str::trim).filter(|n| !n.is_empty());
        if nom.is_some_and(|n| n.chars().count() > 120) {
            return Err(EntDbError::SaisieInvalide("nom du déclarant trop long"));
        }
        let s = self
            .creer_situation(
                etablissement_id,
                acteur,
                a.eleve_id,
                "retard_annonce",
                jour,
                None,
                None,
                Some(a.heure_arrivee_prevue),
                "accueil",
                None,
                nom,
                types_appel::RETARD_ANNONCE,
            )
            .await?;
        self.rafraichir_appels_ouverts_eleve(etablissement_id, a.eleve_id, jour).await?;
        Ok(s)
    }

    #[allow(clippy::too_many_arguments)]
    async fn creer_situation(
        &self,
        etablissement_id: Uuid,
        acteur: Uuid,
        eleve_id: Uuid,
        nature: &str,
        jour: NaiveDate,
        debut_heure: Option<NaiveTime>,
        fin_heure: Option<NaiveTime>,
        arrivee: Option<NaiveTime>,
        source: &str,
        lieu: Option<&str>,
        declarant_nom: Option<&str>,
        type_event: &str,
    ) -> Result<SituationAsd, EntDbError> {
        // Les arguments `arrivee` et `lieu` sont distincts selon la nature ; le tri est fait ici.
        let (arrivee, lieu, declarant_nom) = if nature == "retard_annonce" {
            (arrivee, None, declarant_nom)
        } else {
            (None, lieu, declarant_nom)
        };
        let id = Uuid::new_v4();
        let mut tx = TenantTx::begin(self.pool(), etablissement_id).await?;
        sqlx::query(
            "INSERT INTO ent_asd_situation
                (id, etablissement_id, eleve_id, nature, debut_jour, debut_heure, fin_jour, fin_heure,
                 heure_arrivee_prevue, source, lieu_code, declarant_nom, canal, cree_par)
             VALUES ($1,$2,$3,$4,$5,$6,$5,$7,$8,$9,$10,$11,'web',$12)",
        )
        .bind(id)
        .bind(etablissement_id)
        .bind(eleve_id)
        .bind(nature)
        .bind(jour)
        .bind(debut_heure)
        .bind(fin_heure)
        .bind(arrivee)
        .bind(source)
        .bind(lieu)
        .bind(declarant_nom)
        .bind(acteur)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        sqlx::query(
            "INSERT INTO ent_asd_situation_version (id, etablissement_id, situation_id, etat_avant, etat_apres, auteur_user_id)
             VALUES ($1,$2,$3,NULL,'active',$4)",
        )
        .bind(Uuid::new_v4())
        .bind(etablissement_id)
        .bind(id)
        .bind(acteur)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        let ev = tx
            .publier(MetierEventDraft {
                etablissement_id,
                type_name: type_event.to_string(),
                aggregate: AggregateKind::Absence,
                aggregate_id: id,
                eleve_id: Some(eleve_id),
                payload: json!({ "situation_id": id, "nature": nature, "jour": jour.to_string(), "lieu": lieu }),
                actor_user_id: Some(acteur.to_string()),
            })
            .await?;
        let sql = format!("SELECT {COLONNES_SITUATION} FROM ent_asd_situation WHERE etablissement_id = $1 AND id = $2");
        let s: SituationAsd = sqlx::query_as(&sql)
            .bind(etablissement_id)
            .bind(id)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
        tx.commit().await?;
        self.diffuser_apres_commit(&[ev]);
        Ok(s)
    }

    /// Lève une situation (l'élève est revenu, la localisation est terminée) : `localisation.poser`
    /// pour une localisation ou un retard annoncé, `absences.declarer` sur l'élève sinon ; la
    /// situation passe à `close` (ou `venu` pour une déclaration), jamais supprimée.
    pub async fn lever_situation_for_actor(
        &self,
        etablissement_id: Uuid,
        acteur: Uuid,
        role: &str,
        situation_id: Uuid,
        venu: bool,
    ) -> Result<(), EntDbError> {
        let sql = format!("SELECT {COLONNES_SITUATION} FROM ent_asd_situation WHERE etablissement_id = $1 AND id = $2");
        let s: SituationAsd = sqlx::query_as(&sql)
            .bind(etablissement_id)
            .bind(situation_id)
            .fetch_optional(&mut *TenantTx::begin(self.pool(), etablissement_id).await.map_err(db)?)
            .await
            .map_err(db)?
            .ok_or_else(introuvable)?;
        if matches!(s.nature.as_str(), "localisation" | "exclusion_cours") {
            self.exiger_action_etab(etablissement_id, acteur, role, Action::LocalisationPoser).await?;
        } else {
            self.exiger_action_eleve(etablissement_id, acteur, role, Action::AbsencesDeclarer, s.eleve_id).await?;
        }
        if s.etat != "active" {
            return Err(EntDbError::Domain(DomainError::Conflit));
        }
        let apres = if venu { "venu" } else { "close" };
        let mut tx = TenantTx::begin(self.pool(), etablissement_id).await?;
        sqlx::query("UPDATE ent_asd_situation SET etat = $3, modifie_le = now() WHERE etablissement_id = $1 AND id = $2")
            .bind(etablissement_id)
            .bind(situation_id)
            .bind(apres)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        sqlx::query(
            "INSERT INTO ent_asd_situation_version (id, etablissement_id, situation_id, etat_avant, etat_apres, auteur_user_id)
             VALUES ($1,$2,$3,'active',$4,$5)",
        )
        .bind(Uuid::new_v4())
        .bind(etablissement_id)
        .bind(situation_id)
        .bind(apres)
        .bind(acteur)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        let ev = tx
            .publier(MetierEventDraft {
                etablissement_id,
                type_name: types_appel::SITUATION_LEVEE.to_string(),
                aggregate: AggregateKind::Absence,
                aggregate_id: situation_id,
                eleve_id: Some(s.eleve_id),
                payload: json!({ "situation_id": situation_id, "etat": apres }),
                actor_user_id: Some(acteur.to_string()),
            })
            .await?;
        tx.commit().await?;
        self.diffuser_apres_commit(&[ev]);
        self.rafraichir_appels_ouverts_eleve(etablissement_id, s.eleve_id, s.debut_jour).await?;
        Ok(())
    }

    /// Situations d'un jour : vie scolaire et direction (`absences.lire_motif`). Sans texte libre.
    pub async fn situations_du_jour_for_actor(
        &self,
        etablissement_id: Uuid,
        acteur: Uuid,
        role: &str,
        jour: NaiveDate,
    ) -> Result<Vec<SituationAsd>, EntDbError> {
        self.exiger_action_etab(etablissement_id, acteur, role, Action::AbsencesLireMotif).await?;
        let sql = format!(
            "SELECT {COLONNES_SITUATION} FROM ent_asd_situation
              WHERE etablissement_id = $1 AND debut_jour <= $2 AND fin_jour >= $2
              ORDER BY debut_jour, cree_le"
        );
        sqlx::query_as::<_, SituationAsd>(&sql)
            .bind(etablissement_id)
            .bind(jour)
            .fetch_all(&mut *TenantTx::begin(self.pool(), etablissement_id).await.map_err(db)?)
            .await
            .map_err(db)
    }

    /// Texte libre d'une situation (R-AS22, NJ-35) : lu seulement par son **auteur** et par la vie
    /// scolaire (`absences.lire_motif`) ; jamais par le professeur, l'accueil ni l'autre responsable.
    pub async fn texte_situation_for_actor(
        &self,
        etablissement_id: Uuid,
        acteur: Uuid,
        role: &str,
        situation_id: Uuid,
    ) -> Result<Vec<String>, EntDbError> {
        let peut = self
            .a_action_etablissement(etablissement_id, acteur, role, Action::AbsencesLireMotif)
            .await?;
        let rows: Vec<(String, Option<Uuid>)> = sqlx::query_as(
            "SELECT texte, auteur_user_id FROM ent_asd_situation_texte
              WHERE etablissement_id = $1 AND situation_id = $2 ORDER BY cree_le",
        )
        .bind(etablissement_id)
        .bind(situation_id)
        .fetch_all(&mut *TenantTx::begin(self.pool(), etablissement_id).await.map_err(db)?)
        .await
        .map_err(db)?;
        // Réponse identique pour « inexistant » et « interdit » (R-AS35).
        let visibles: Vec<String> = rows
            .into_iter()
            .filter(|(_, auteur)| peut || *auteur == Some(acteur))
            .map(|(t, _)| t)
            .collect();
        if visibles.is_empty() {
            return Err(introuvable());
        }
        Ok(visibles)
    }

    /// Actualise les appels encore ouverts d'un élève pour ce jour (une localisation posée à 8 h 05
    /// apparaît dans l'appel de 8 h encore ouvert).
    pub(crate) async fn rafraichir_appels_ouverts_eleve(
        &self,
        etablissement_id: Uuid,
        eleve_id: Uuid,
        jour: NaiveDate,
    ) -> Result<(), EntDbError> {
        let ids: Vec<Uuid> = sqlx::query_scalar(
            "SELECT a.id FROM ent_appel a
               JOIN ent_appel_ligne l ON l.etablissement_id = a.etablissement_id AND l.appel_id = a.id
              WHERE a.etablissement_id = $1 AND a.jour = $2 AND a.etat = 'en_cours' AND l.eleve_id = $3",
        )
        .bind(etablissement_id)
        .bind(jour)
        .bind(eleve_id)
        .fetch_all(&mut *TenantTx::begin(self.pool(), etablissement_id).await.map_err(db)?)
        .await
        .map_err(db)?;
        for id in ids {
            let mut tx = TenantTx::begin(self.pool(), etablissement_id).await?;
            self.preremplir_appel_tx(&mut tx, etablissement_id, id).await?;
            tx.commit().await?;
        }
        Ok(())
    }
}

fn canal_valide(canal: &str) -> Result<&'static str, EntDbError> {
    match canal {
        "web" => Ok("web"),
        "mobile" => Ok("mobile"),
        "hors_ligne" => Ok("hors_ligne"),
        _ => Err(EntDbError::SaisieInvalide("canal de saisie inconnu")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(h: u32, m: u32) -> NaiveTime {
        NaiveTime::from_hms_opt(h, m, 0).unwrap()
    }
    fn d(j: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2027, 3, j).unwrap()
    }
    fn sit(nature: &str, dj: u32, dh: Option<NaiveTime>, fj: u32, fh: Option<NaiveTime>) -> SituationAsd {
        SituationAsd {
            id: Uuid::new_v4(),
            eleve_id: Uuid::nil(),
            nature: nature.into(),
            debut_jour: d(dj),
            debut_heure: dh,
            fin_jour: d(fj),
            fin_heure: fh,
            heure_arrivee_prevue: None,
            source: "famille".into(),
            lieu_code: None,
            etat: "active".into(),
        }
    }

    #[test]
    fn une_declaration_a_la_journee_couvre_toutes_les_seances() {
        let s = sit("declaration", 8, None, 8, None);
        assert!(situation_couvre(&s, d(8), t(8, 0), t(9, 0)));
        assert!(situation_couvre(&s, d(8), t(16, 0), t(17, 0)));
        assert!(!situation_couvre(&s, d(9), t(8, 0), t(9, 0)));
    }

    #[test]
    fn une_declaration_a_heures_ne_couvre_que_le_chevauchement() {
        let s = sit("declaration", 8, Some(t(10, 0)), 8, Some(t(12, 0)));
        assert!(!situation_couvre(&s, d(8), t(8, 0), t(10, 0)), "la séance finit quand la déclaration commence");
        assert!(situation_couvre(&s, d(8), t(9, 0), t(11, 0)));
        assert!(!situation_couvre(&s, d(8), t(12, 0), t(13, 0)));
    }

    #[test]
    fn periode_de_plusieurs_jours_heures_aux_bornes() {
        let s = sit("declaration", 8, Some(t(14, 0)), 10, Some(t(10, 0)));
        assert!(!situation_couvre(&s, d(8), t(8, 0), t(9, 0)), "le premier jour, après 14 h seulement");
        assert!(situation_couvre(&s, d(8), t(15, 0), t(16, 0)));
        assert!(situation_couvre(&s, d(9), t(8, 0), t(9, 0)), "jour intermédiaire : entier");
        assert!(situation_couvre(&s, d(10), t(8, 0), t(9, 0)));
        assert!(!situation_couvre(&s, d(10), t(10, 0), t(11, 0)));
    }

    #[test]
    fn retard_annonce_couvre_jusqu_a_l_heure_dite() {
        let mut s = sit("retard_annonce", 8, None, 8, None);
        s.heure_arrivee_prevue = Some(t(10, 0));
        assert!(situation_couvre(&s, d(8), t(8, 0), t(9, 0)), "absent prévenu jusqu'à 10 h");
        assert!(situation_couvre(&s, d(8), t(9, 30), t(10, 30)), "séance à cheval : prévenu");
        assert!(!situation_couvre(&s, d(8), t(10, 0), t(11, 0)), "présence attendue ensuite");
        s.heure_arrivee_prevue = None;
        assert!(!situation_couvre(&s, d(8), t(8, 0), t(9, 0)));
    }

    #[test]
    fn une_situation_ailleurs_n_est_jamais_une_absence() {
        for n in ["sortie", "examen", "mission", "pfmp", "exclusion_inclusion", "mobilite", "dispense_presence",
                  "prise_en_charge_exterieure", "localisation", "exclusion_cours"] {
            let (e, lieu) = etat_propose(&sit(n, 8, None, 8, None));
            assert_ne!(e, LigneEtat::Absent, "{n} ne doit jamais proposer « absent » (R-AS4)");
            assert!(lieu.is_some(), "{n} : lieu codé");
        }
        for n in ["declaration", "autorisation", "sortie_anticipee", "retard_annonce"] {
            assert_eq!(etat_propose(&sit(n, 8, None, 8, None)).0, LigneEtat::Absent, "{n} : absent prévenu");
        }
    }

    #[test]
    fn la_localisation_l_emporte_sur_la_declaration() {
        let decl = sit("declaration", 8, None, 8, None);
        let loc = {
            let mut s = sit("localisation", 8, None, 8, None);
            s.lieu_code = Some("infirmerie".into());
            s
        };
        let v = vec![decl, loc.clone()];
        let m = meilleure_situation(&v, d(8), t(8, 0), t(9, 0)).expect("une situation");
        assert_eq!(m.id, loc.id, "constat d'un adulte > déclaration (R-AS10)");
    }

    #[test]
    fn les_libelles_courts_ne_portent_jamais_de_motif() {
        for n in ["declaration", "autorisation", "sortie", "examen", "mission", "pfmp", "localisation",
                  "dispense_presence", "retard_annonce"] {
            let l = libelle_court(n, Some("infirmerie"));
            assert!(!l.to_lowercase().contains("malad"), "{n} : {l}");
            assert!(!l.is_empty());
        }
        assert_eq!(libelle_court("declaration", None), "prévenu");
    }

    #[test]
    fn precisions_de_ligne() {
        assert!(valider_precisions(&LigneEtat::Retard, Some(5), None).is_ok());
        assert!(valider_precisions(&LigneEtat::Retard, Some(0), None).is_err());
        assert!(valider_precisions(&LigneEtat::Retard, Some(601), None).is_err());
        assert!(valider_precisions(&LigneEtat::Present, Some(5), None).is_err());
        assert!(valider_precisions(&LigneEtat::Localise, None, Some("infirmerie")).is_ok());
        assert!(valider_precisions(&LigneEtat::Localise, None, None).is_err());
        assert!(valider_precisions(&LigneEtat::Localise, None, Some("cantine")).is_err());
        assert!(valider_precisions(&LigneEtat::Absent, None, Some("cdi")).is_err());
        assert!(etat_saisissable("attendu_ailleurs").is_err(), "posé par le système seulement");
        assert!(etat_saisissable("non_pointe").is_err());
        assert!(etat_saisissable("inconnu").is_err());
    }

    #[test]
    fn recapitulatif() {
        let l = |etat: &str, pre: bool| LigneAppelV2 {
            eleve_id: Uuid::new_v4(),
            etat: etat.into(),
            minutes_retard: None,
            lieu_code: None,
            pre_remplie: pre,
            libelle_court: None,
            version: 1,
            eleve_label: None,
            arrivee_vs_attendue: false,
        };
        let r = recapituler(&[l("present", false), l("present", false), l("absent", true), l("absent", false),
                              l("retard", false), l("non_pointe", false)]);
        assert_eq!((r.presents, r.absents, r.absents_prevenus, r.retards, r.non_pointes), (2, 2, 1, 1, 1));
    }
}
