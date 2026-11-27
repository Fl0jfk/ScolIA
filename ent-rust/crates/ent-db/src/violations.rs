//! **Registre des violations de données** (socle 27, palier G5) : déclaration en trente secondes, horloge des 72 h,
//! qualification, risque proposé par une grille déterministe puis décidé par la direction après avis du DPO,
//! notification à la CNIL (initiale et complémentaires), non-notification justifiée, information des personnes,
//! clôture et revue à 30 jours. Réf. : `27_RGPD_ARCHIVES.md` §8.7, §12.1, §13.3, §14.10 (RGP-240 à RGP-259), §17.6 ;
//! RGPD art. 33 et 34.
//!
//! Règles appliquées : **R-RG21** (l'horloge ne bloque jamais une notification tardive : elle exige un motif),
//! **R-RG32** (les écrans ne montrent que la violation qui justifie l'accès ; le déclarant ne voit que l'état de sa
//! déclaration), **R-RG34** (la grille propose, la direction décide), **R-RG40** (toute transition écrit la chronologie
//! dans la même transaction), **E14** (aucune donnée touchée dans une alerte ni dans la chronologie).
//!
//! Choix par défaut (D-069), notés dans la doc du palier :
//! * la décision du risque, la non-notification et l'information des personnes sont réservées à une action ★ distincte
//!   (`violations.decider`, direction) : le DPO et le référent sécurité qualifient et conseillent (`violations.gerer`),
//!   ils ne décident pas (RGP-245 : « décidé par la direction après avis du DPO ») ;
//! * l'avis du DPO est **exigé** avant la décision du risque : un établissement sans DPO nommé consigne l'avis de la
//!   personne qui en tient lieu (le texte le dit) ; la base le garantit (`rgp_violation_risque_decide`) ;
//! * la notification à la CNIL peut être saisie **avant** la décision du risque (art. 33.4 : notification par phases,
//!   dans l'urgence) ; la non-notification, elle, exige un risque « faible » décidé ;
//! * la prise de connaissance est l'instant de la déclaration ; seule une saisie par un gestionnaire peut la dater plus
//!   tôt (incident appris par téléphone la veille), jamais dans le futur ni plus d'un an en arrière.
//!
//! Cloisonnement : tout passe par [`TenantTx`] (RLS posée) et `exiger_action_etab` ; aucune fonction ne reçoit
//! d'établissement venant de la requête HTTP.

use crate::acces::EtatAcces;
use crate::notifications::{notifier_personne_cibles, EnvoiAPlanifier};
use crate::service::{EntDbError, PgEnt};
use crate::tenant_tx::{etablissements_systeme, ContexteSysteme, TenantTx};
use chrono::{DateTime, Datelike, Duration, NaiveDate, Utc};
use domain_eleve::DomainError;
use ent_auth::Action;
use serde::Serialize;
use sqlx::{PgConnection, Row};
use uuid::Uuid;

fn db(e: sqlx::Error) -> EntDbError {
    EntDbError::Sqlx(e)
}

fn invalide(m: &'static str) -> EntDbError {
    EntDbError::SaisieInvalide(m)
}

/// Délai de notification à la CNIL, en heures (art. 33.1 : « si possible, 72 heures au plus tard »).
pub const DELAI_CNIL_HEURES: i64 = 72;
/// Heures restantes auxquelles les gestionnaires sont alertés (§18.1 : H-24, H-6).
pub const ALERTE_ECHEANCE_HEURES: [i64; 2] = [24, 6];
/// Revue de retour d'expérience, en jours après la clôture (§8.7 point 8).
pub const REVUE_JOURS: i64 = 30;
/// Antériorité maximale d'une prise de connaissance saisie par un gestionnaire (au-delà, c'est une erreur de saisie).
pub const CONNUE_LE_MAX_JOURS: i64 = 366;

/// Natures de notification déclarées au socle 8 (§18.1).
pub const KIND_DECLAREE: &str = "rgpd.violation_declaree";
pub const KIND_ECHEANCE: &str = "rgpd.violation_echeance_72h";

/// Niveaux de risque (art. 33 et 34).
pub const RISQUES: [&str; 3] = ["faible", "avere", "eleve"];
pub const NATURES_INCIDENT: [&str; 2] = ["violation", "incident_securite"];

/// Catégories de données dont l'atteinte fait proposer un risque **élevé** (données de santé, décisions de justice,
/// discipline, coordonnées bancaires, aides sociales, secrets d'authentification) : art. 9 et 10 du RGPD, et
/// lignes directrices du CEPD 9/2022 sur les violations (nature sensible des données, mineurs).
pub const CATEGORIES_PARTICULIERES: [&str; 7] = [
    "sante.dossier",
    "adm.decision_justice",
    "disc.sanction",
    "disc.conseil",
    "fin.iban",
    "fin.bourse",
    "cpt.securite",
];
/// Catégories d'identification seule (nom, classe) : leur divulgation seule fait proposer un risque faible (CL27-41).
pub const CATEGORIES_IDENTIFICATION: [&str; 1] = ["scol.registre"];

/// Message invariable rendu au déclarant (RGP-240, CL27-45) : merci, rien d'autre.
pub const MERCI_DECLARANT: &str = "Merci : votre déclaration est enregistrée et la direction est prévenue. \
Déclarer vite est le bon réflexe ; aucune suite ne vise la personne qui déclare.";

fn texte(v: &str, min: usize, max: usize) -> Option<String> {
    let v = v.trim();
    let n = v.chars().count();
    (n >= min && n <= max && !v.chars().any(|c| c.is_control() && c != '\n')).then(|| v.to_string())
}

fn texte_exige(v: &str, min: usize, max: usize, m: &'static str) -> Result<String, EntDbError> {
    texte(v, min, max).ok_or(invalide(m))
}

/// Échéance CNIL : prise de connaissance + 72 h, exactement (instant UTC : le fuseau ne change pas une durée).
pub fn echeance_cnil(connue_le: DateTime<Utc>) -> DateTime<Utc> {
    connue_le + Duration::hours(DELAI_CNIL_HEURES)
}

/// Référence lisible : « V-2027-003 ».
pub fn reference(annee: i32, numero: i32) -> String {
    format!("V-{annee}-{numero:03}")
}

// ---------------------------------------------------------------------------------------------
// Grille déterministe (RGP-243) : propose, ne décide pas (R-RG34)
// ---------------------------------------------------------------------------------------------

/// Entrées de la grille, toutes tirées de la qualification.
#[derive(Clone, Debug, Default)]
pub struct EntreesGrille<'a> {
    pub confidentialite: bool,
    pub integrite: bool,
    pub disponibilite: bool,
    pub categories: &'a [&'a str],
    pub personnes_nombre: Option<i32>,
    pub personnes_mineurs: bool,
}

/// Risque proposé et raisons lisibles (affichées à côté de la proposition).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct RisquePropose {
    pub risque: &'static str,
    pub raisons: Vec<&'static str>,
}

/// Seuil de personnes au-delà duquel une atteinte à la confidentialité de données autres que l'identification seule
/// fait proposer un risque avéré même pour des catégories ordinaires (D-069, valeur prudente, à revoir avec le DPO).
pub const SEUIL_PERSONNES_AVERE: i32 = 50;

/// Grille de proposition du risque (§8.7 point 3). Déterministe, sans IA, sans base : la même qualification donne
/// toujours la même proposition. Lecture prudente des lignes directrices du CEPD 9/2022 :
/// * **élevé** : données de santé touchées (CL27-47, quelle que soit la nature de l'atteinte) ; ou catégorie
///   particulière touchée en confidentialité ou en intégrité ;
/// * **avéré** : confidentialité ou intégrité d'autres données que l'identification seule ; ou mineurs touchés en
///   confidentialité ; ou indisponibilité de données particulières ; ou plus de [`SEUIL_PERSONNES_AVERE`] personnes
///   touchées en intégrité ou en confidentialité hors identification seule ;
/// * **faible** : identification seule divulguée (nom et classe, CL27-41), indisponibilité de données ordinaires, ou
///   rien de qualifié.
pub fn proposer_risque(e: &EntreesGrille<'_>) -> RisquePropose {
    let mut raisons = Vec::new();
    let sante = e.categories.contains(&"sante.dossier");
    let particuliere = e.categories.iter().any(|c| CATEGORIES_PARTICULIERES.contains(c));
    let au_dela_identification = e
        .categories
        .iter()
        .any(|c| !CATEGORIES_IDENTIFICATION.contains(c));
    let atteinte = e.confidentialite || e.integrite;
    if sante {
        raisons.push("données de santé touchées : risque élevé par défaut (CL27-47), avis du DPO obligatoire");
    }
    if particuliere && atteinte {
        raisons.push("catégorie particulière divulguée ou altérée (santé, justice, discipline, banque, aides, secrets)");
    }
    if !raisons.is_empty() {
        return RisquePropose { risque: "eleve", raisons };
    }
    if atteinte && au_dela_identification {
        raisons.push("données au-delà de l'identification seule divulguées ou altérées");
    }
    if e.confidentialite && e.personnes_mineurs {
        raisons.push("des mineurs sont concernés par la divulgation");
    }
    if e.disponibilite && particuliere {
        raisons.push("données particulières indisponibles");
    }
    if atteinte && au_dela_identification && e.personnes_nombre.is_some_and(|n| n > SEUIL_PERSONNES_AVERE) {
        raisons.push("nombre important de personnes concernées");
    }
    if !raisons.is_empty() {
        return RisquePropose { risque: "avere", raisons };
    }
    if e.confidentialite {
        raisons.push("identification seule divulguée (nom, classe) : risque faible proposé (CL27-41)");
    } else if e.disponibilite {
        raisons.push("indisponibilité de données ordinaires, sans divulgation ni altération");
    } else {
        raisons.push("aucune atteinte qualifiée pour l'instant");
    }
    RisquePropose { risque: "faible", raisons }
}

// ---------------------------------------------------------------------------------------------
// Saisies et vues
// ---------------------------------------------------------------------------------------------

/// Déclaration par un personnel (RGP-240) : trois champs, rien d'obligatoire au-delà de « ce qui s'est passé ».
#[derive(Clone, Debug, Default)]
pub struct Declaration<'a> {
    /// Ce qui s'est passé, en une phrase.
    pub intitule: &'a str,
    /// Précisions libres (ce qui est touché, combien de personnes).
    pub description: Option<&'a str>,
    /// Depuis quand (quand la personne l'a constaté), si elle le sait.
    pub constatee_le: Option<DateTime<Utc>>,
    /// Incident de sécurité sans atteinte aux données (RGP-259) ; par défaut, violation.
    pub incident_securite: bool,
}

/// Qualification (RGP-243) : remplace la précédente (la chronologie garde la trace de chaque passage).
#[derive(Clone, Debug, Default)]
pub struct Qualification<'a> {
    pub nature_incident: &'a str,
    pub confidentialite: bool,
    pub integrite: bool,
    pub disponibilite: bool,
    pub categories: &'a [&'a str],
    pub personnes_nombre: Option<i32>,
    pub personnes_estime: bool,
    pub personnes_mineurs: bool,
    pub consequences: Option<&'a str>,
}

/// Ce que voit le déclarant de sa propre déclaration (R-RG32) : la référence et l'avancement, rien d'autre.
#[derive(Clone, Debug, Serialize)]
pub struct MaDeclaration {
    pub id: Uuid,
    pub reference: String,
    pub intitule: String,
    pub connue_le: DateTime<Utc>,
    /// `recue`, `en_cours` ou `close` : la qualification et la décision ne sont pas exposées au déclarant.
    pub avancement: &'static str,
    pub message: &'static str,
}

#[derive(Clone, Debug, Serialize)]
pub struct MesureVue {
    pub nature: String,
    pub texte: String,
    pub le: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize)]
pub struct NotificationVue {
    pub rang: i32,
    pub nature: String,
    pub notifiee_le: DateTime<Utc>,
    pub reference_cnil: String,
    pub resume: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct EtapeVue {
    pub etape: String,
    pub texte: Option<String>,
    pub par_role: Option<String>,
    pub le: DateTime<Utc>,
}

/// Fiche complète d'une violation (gestionnaires).
#[derive(Clone, Debug, Serialize)]
pub struct ViolationVue {
    pub id: Uuid,
    pub reference: String,
    pub nature_incident: String,
    pub origine: String,
    pub plateforme: bool,
    pub source_code: Option<String>,
    pub declaree_par_compte_id: Option<Uuid>,
    pub intitule: String,
    pub description: Option<String>,
    pub constatee_le: Option<DateTime<Utc>>,
    pub connue_le: DateTime<Utc>,
    pub echeance_cnil_le: DateTime<Utc>,
    /// Heures restantes avant l'échéance (négatif : dépassée). Calculé à l'instant de la lecture.
    pub heures_restantes: i64,
    /// L'échéance est dépassée sans notification ni non-notification justifiée.
    pub en_retard: bool,
    pub confidentialite: bool,
    pub integrite: bool,
    pub disponibilite: bool,
    pub categories: Vec<String>,
    pub personnes_nombre: Option<i32>,
    pub personnes_estime: bool,
    pub personnes_mineurs: bool,
    pub consequences: Option<String>,
    pub qualifiee_le: Option<DateTime<Utc>>,
    pub risque_propose: Option<String>,
    pub raisons_proposition: Vec<&'static str>,
    pub avis_dpo: Option<String>,
    pub avis_dpo_le: Option<DateTime<Utc>>,
    pub risque: Option<String>,
    pub risque_motif: Option<String>,
    pub risque_decide_le: Option<DateTime<Utc>>,
    pub notifiee_cnil_le: Option<DateTime<Utc>>,
    pub reference_cnil: Option<String>,
    pub retard_motif: Option<String>,
    pub non_notification_motif: Option<String>,
    pub personnes_informees_le: Option<DateTime<Utc>>,
    pub personnes_informees_nombre: Option<i32>,
    pub personnes_non_information_motif: Option<String>,
    pub etat: String,
    pub close_le: Option<DateTime<Utc>>,
    pub retour_experience: Option<String>,
    pub revue_le: Option<NaiveDate>,
    pub mesures: Vec<MesureVue>,
    pub notifications: Vec<NotificationVue>,
    pub chronologie: Vec<EtapeVue>,
    /// Ce qui manque pour clore (vide : la clôture est possible).
    pub manque_pour_clore: Vec<&'static str>,
    /// Violation de la plateforme propagée (RGP-255) : ce que l'éditeur en dit, pour CET établissement seulement.
    pub plateforme_element: Option<ElementPlateforme>,
}

/// Ce que l'établissement reçoit d'une violation de la plateforme (R-RG36) : la référence de l'éditeur, ce qu'il sait
/// de l'incident, et les éléments propres à l'établissement ; jamais ceux d'un autre.
#[derive(Clone, Debug, Serialize)]
pub struct ElementPlateforme {
    pub reference: String,
    pub intitule: String,
    pub description: String,
    /// Prise de connaissance par l'éditeur (le délai « dans les meilleurs délais » de l'art. 33.2 se lit d'ici).
    pub connue_editeur_le: DateTime<Utc>,
    pub informe_le: DateTime<Utc>,
    pub elements: String,
    pub personnes_nombre: Option<i32>,
    /// La violation touche la couche compte, dont ScolIA est responsable : ScolIA notifie pour sa part (RGP-311).
    pub couche_compte: bool,
    pub notifiee_cnil_par_editeur_le: Option<DateTime<Utc>>,
}

/// Une ligne du registre (RGP-258) : compteurs et dates, jamais les données touchées.
#[derive(Clone, Debug, Serialize)]
pub struct ViolationLigne {
    pub id: Uuid,
    pub reference: String,
    pub nature_incident: String,
    pub origine: String,
    pub intitule: String,
    pub connue_le: DateTime<Utc>,
    pub echeance_cnil_le: DateTime<Utc>,
    pub heures_restantes: i64,
    pub en_retard: bool,
    pub risque: Option<String>,
    pub notifiee_cnil_le: Option<DateTime<Utc>>,
    pub non_notifiee: bool,
    pub personnes_nombre: Option<i32>,
    pub etat: String,
    pub close_le: Option<DateTime<Utc>>,
}

/// Filtre du registre.
#[derive(Clone, Debug, Default)]
pub struct FiltreRegistre<'a> {
    pub annee: Option<i32>,
    pub etat: Option<&'a str>,
    pub nature_incident: Option<&'a str>,
    /// Seulement les violations ouvertes dont l'échéance n'est pas réglée (ni notifiée, ni justifiée).
    pub a_notifier: bool,
}

// ---------------------------------------------------------------------------------------------
// Lecture brute
// ---------------------------------------------------------------------------------------------

const COLONNES: &str = "id, annee, numero, nature_incident, origine, plateforme_violation_id, source_code, \
    declaree_par_compte_id, intitule, description, constatee_le, connue_le, echeance_cnil_le, confidentialite, \
    integrite, disponibilite, personnes_nombre, personnes_estime, personnes_mineurs, consequences, qualifiee_le, \
    risque_propose, avis_dpo, avis_dpo_le, risque, risque_motif, risque_decide_le, notifiee_cnil_le, reference_cnil, \
    retard_motif, non_notification_motif, personnes_informees_le, personnes_informees_nombre, \
    personnes_non_information_motif, etat, close_le, retour_experience, revue_le";

#[derive(Clone, Debug)]
struct Brut {
    id: Uuid,
    annee: i32,
    numero: i32,
    nature_incident: String,
    origine: String,
    plateforme_violation_id: Option<Uuid>,
    source_code: Option<String>,
    declaree_par_compte_id: Option<Uuid>,
    intitule: String,
    description: Option<String>,
    constatee_le: Option<DateTime<Utc>>,
    connue_le: DateTime<Utc>,
    echeance_cnil_le: DateTime<Utc>,
    confidentialite: bool,
    integrite: bool,
    disponibilite: bool,
    personnes_nombre: Option<i32>,
    personnes_estime: bool,
    personnes_mineurs: bool,
    consequences: Option<String>,
    qualifiee_le: Option<DateTime<Utc>>,
    risque_propose: Option<String>,
    avis_dpo: Option<String>,
    avis_dpo_le: Option<DateTime<Utc>>,
    risque: Option<String>,
    risque_motif: Option<String>,
    risque_decide_le: Option<DateTime<Utc>>,
    notifiee_cnil_le: Option<DateTime<Utc>>,
    reference_cnil: Option<String>,
    retard_motif: Option<String>,
    non_notification_motif: Option<String>,
    personnes_informees_le: Option<DateTime<Utc>>,
    personnes_informees_nombre: Option<i32>,
    personnes_non_information_motif: Option<String>,
    etat: String,
    close_le: Option<DateTime<Utc>>,
    retour_experience: Option<String>,
    revue_le: Option<NaiveDate>,
}

fn brut(r: &sqlx::postgres::PgRow) -> Brut {
    Brut {
        id: r.get("id"),
        annee: r.get("annee"),
        numero: r.get("numero"),
        nature_incident: r.get("nature_incident"),
        origine: r.get("origine"),
        plateforme_violation_id: r.get("plateforme_violation_id"),
        source_code: r.get("source_code"),
        declaree_par_compte_id: r.get("declaree_par_compte_id"),
        intitule: r.get("intitule"),
        description: r.get("description"),
        constatee_le: r.get("constatee_le"),
        connue_le: r.get("connue_le"),
        echeance_cnil_le: r.get("echeance_cnil_le"),
        confidentialite: r.get("confidentialite"),
        integrite: r.get("integrite"),
        disponibilite: r.get("disponibilite"),
        personnes_nombre: r.get("personnes_nombre"),
        personnes_estime: r.get("personnes_estime"),
        personnes_mineurs: r.get("personnes_mineurs"),
        consequences: r.get("consequences"),
        qualifiee_le: r.get("qualifiee_le"),
        risque_propose: r.get("risque_propose"),
        avis_dpo: r.get("avis_dpo"),
        avis_dpo_le: r.get("avis_dpo_le"),
        risque: r.get("risque"),
        risque_motif: r.get("risque_motif"),
        risque_decide_le: r.get("risque_decide_le"),
        notifiee_cnil_le: r.get("notifiee_cnil_le"),
        reference_cnil: r.get("reference_cnil"),
        retard_motif: r.get("retard_motif"),
        non_notification_motif: r.get("non_notification_motif"),
        personnes_informees_le: r.get("personnes_informees_le"),
        personnes_informees_nombre: r.get("personnes_informees_nombre"),
        personnes_non_information_motif: r.get("personnes_non_information_motif"),
        etat: r.get("etat"),
        close_le: r.get("close_le"),
        retour_experience: r.get("retour_experience"),
        revue_le: r.get("revue_le"),
    }
}

impl Brut {
    /// L'échéance CNIL est « réglée » : notifiée, ou non-notification justifiée, ou incident sans atteinte aux données.
    fn echeance_reglee(&self) -> bool {
        self.notifiee_cnil_le.is_some()
            || self.non_notification_motif.is_some()
            || self.nature_incident == "incident_securite"
            || self.etat == "close"
    }

    fn heures_restantes(&self, maintenant: DateTime<Utc>) -> i64 {
        // Arrondi vers le bas (une heure entamée n'est pas restante), y compris en négatif.
        (self.echeance_cnil_le - maintenant).num_minutes().div_euclid(60)
    }

    fn en_retard(&self, maintenant: DateTime<Utc>) -> bool {
        !self.echeance_reglee() && maintenant > self.echeance_cnil_le
    }

    /// Ce qui manque pour clore (même règle que `rgp_violation_cloture_complete`, dite en clair).
    fn manque_pour_clore(&self) -> Vec<&'static str> {
        let mut m = Vec::new();
        if self.etat == "close" {
            return m;
        }
        if self.nature_incident == "incident_securite" {
            return m;
        }
        if self.qualifiee_le.is_none() {
            m.push("qualifier la violation (nature, données, personnes, conséquences)");
        }
        if self.risque.is_none() {
            m.push("décider le niveau de risque, après l'avis du DPO");
        }
        if self.notifiee_cnil_le.is_none() && self.non_notification_motif.is_none() {
            m.push("notifier la CNIL, ou justifier la non-notification (risque faible seulement)");
        }
        if self.risque.as_deref() == Some("eleve")
            && self.personnes_informees_le.is_none()
            && self.personnes_non_information_motif.is_none()
        {
            m.push("informer les personnes concernées (risque élevé, art. 34), ou motiver l'exception");
        }
        m
    }
}

async fn charger_brut(conn: &mut PgConnection, etab: Uuid, id: Uuid) -> Result<Brut, EntDbError> {
    let sql = format!("SELECT {COLONNES} FROM rgp_violation WHERE etablissement_id = $1 AND id = $2");
    let r = sqlx::query(&sql)
        .bind(etab)
        .bind(id)
        .fetch_optional(&mut *conn)
        .await
        .map_err(db)?
        .ok_or(EntDbError::Domain(DomainError::NotFound))?;
    Ok(brut(&r))
}

/// Verrou de ligne : deux gestionnaires qui agissent en même temps sur la même violation se suivent.
async fn verrouiller(conn: &mut PgConnection, etab: Uuid, id: Uuid) -> Result<Brut, EntDbError> {
    let sql = format!("SELECT {COLONNES} FROM rgp_violation WHERE etablissement_id = $1 AND id = $2 FOR UPDATE");
    let r = sqlx::query(&sql)
        .bind(etab)
        .bind(id)
        .fetch_optional(&mut *conn)
        .await
        .map_err(db)?
        .ok_or(EntDbError::Domain(DomainError::NotFound))?;
    Ok(brut(&r))
}

fn exiger_non_close(b: &Brut) -> Result<(), EntDbError> {
    if b.etat == "close" {
        Err(EntDbError::ConflitDossier("la violation est close : le registre ne se modifie plus"))
    } else {
        Ok(())
    }
}

fn exiger_violation(b: &Brut) -> Result<(), EntDbError> {
    if b.nature_incident != "violation" {
        Err(EntDbError::ConflitDossier(
            "un incident de sécurité sans atteinte aux données n'a ni risque ni notification",
        ))
    } else {
        Ok(())
    }
}

async fn categories_de(conn: &mut PgConnection, etab: Uuid, id: Uuid) -> Result<Vec<String>, EntDbError> {
    sqlx::query_scalar(
        "SELECT categorie_code FROM rgp_violation_categorie
          WHERE etablissement_id = $1 AND violation_id = $2 ORDER BY categorie_code",
    )
    .bind(etab)
    .bind(id)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)
}

async fn charger(
    conn: &mut PgConnection,
    etab: Uuid,
    id: Uuid,
    maintenant: DateTime<Utc>,
) -> Result<ViolationVue, EntDbError> {
    let b = charger_brut(conn, etab, id).await?;
    let categories = categories_de(conn, etab, id).await?;
    let mesures = sqlx::query(
        "SELECT nature, texte, le FROM rgp_violation_mesure
          WHERE etablissement_id = $1 AND violation_id = $2 ORDER BY le, cree_le, id",
    )
    .bind(etab)
    .bind(id)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?
    .iter()
    .map(|r| MesureVue {
        nature: r.get("nature"),
        texte: r.get("texte"),
        le: r.get("le"),
    })
    .collect();
    let notifications = sqlx::query(
        "SELECT rang, nature, notifiee_le, reference_cnil, resume FROM rgp_violation_notification
          WHERE etablissement_id = $1 AND violation_id = $2 ORDER BY rang",
    )
    .bind(etab)
    .bind(id)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?
    .iter()
    .map(|r| NotificationVue {
        rang: r.get("rang"),
        nature: r.get("nature"),
        notifiee_le: r.get("notifiee_le"),
        reference_cnil: r.get("reference_cnil"),
        resume: r.get("resume"),
    })
    .collect();
    let chronologie = sqlx::query(
        "SELECT etape_code, texte, par_role, le FROM rgp_violation_etape
          WHERE etablissement_id = $1 AND violation_id = $2 ORDER BY le, ordre",
    )
    .bind(etab)
    .bind(id)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?
    .iter()
    .map(|r| EtapeVue {
        etape: r.get("etape_code"),
        texte: r.get("texte"),
        par_role: r.get("par_role"),
        le: r.get("le"),
    })
    .collect();
    let plateforme_element = match b.plateforme_violation_id {
        Some(pv) => sqlx::query(
            "SELECT p.reference, p.intitule, p.description, p.connue_le, p.couche_compte, p.notifiee_cnil_le,
                    e.elements, e.personnes_nombre, e.informe_le
               FROM rgp_violation_plateforme_etab e
               JOIN rgp_violation_plateforme p ON p.id = e.violation_plateforme_id
              WHERE e.etablissement_id = $1 AND e.violation_plateforme_id = $2 AND e.violation_id = $3",
        )
        .bind(etab)
        .bind(pv)
        .bind(id)
        .fetch_optional(&mut *conn)
        .await
        .map_err(db)?
        .map(|r| ElementPlateforme {
            reference: r.get("reference"),
            intitule: r.get("intitule"),
            description: r.get("description"),
            connue_editeur_le: r.get("connue_le"),
            informe_le: r.get("informe_le"),
            elements: r.get("elements"),
            personnes_nombre: r.get("personnes_nombre"),
            couche_compte: r.get("couche_compte"),
            notifiee_cnil_par_editeur_le: r.get("notifiee_cnil_le"),
        }),
        None => None,
    };
    let cats: Vec<&str> = categories.iter().map(String::as_str).collect();
    let raisons_proposition = if b.qualifiee_le.is_some() && b.nature_incident == "violation" {
        proposer_risque(&EntreesGrille {
            confidentialite: b.confidentialite,
            integrite: b.integrite,
            disponibilite: b.disponibilite,
            categories: &cats,
            personnes_nombre: b.personnes_nombre,
            personnes_mineurs: b.personnes_mineurs,
        })
        .raisons
    } else {
        Vec::new()
    };
    Ok(ViolationVue {
        id: b.id,
        reference: reference(b.annee, b.numero),
        heures_restantes: b.heures_restantes(maintenant),
        en_retard: b.en_retard(maintenant),
        manque_pour_clore: b.manque_pour_clore(),
        plateforme: b.plateforme_violation_id.is_some(),
        nature_incident: b.nature_incident,
        origine: b.origine,
        source_code: b.source_code,
        declaree_par_compte_id: b.declaree_par_compte_id,
        intitule: b.intitule,
        description: b.description,
        constatee_le: b.constatee_le,
        connue_le: b.connue_le,
        echeance_cnil_le: b.echeance_cnil_le,
        confidentialite: b.confidentialite,
        integrite: b.integrite,
        disponibilite: b.disponibilite,
        categories,
        personnes_nombre: b.personnes_nombre,
        personnes_estime: b.personnes_estime,
        personnes_mineurs: b.personnes_mineurs,
        consequences: b.consequences,
        qualifiee_le: b.qualifiee_le,
        risque_propose: b.risque_propose,
        raisons_proposition,
        avis_dpo: b.avis_dpo,
        avis_dpo_le: b.avis_dpo_le,
        risque: b.risque,
        risque_motif: b.risque_motif,
        risque_decide_le: b.risque_decide_le,
        notifiee_cnil_le: b.notifiee_cnil_le,
        reference_cnil: b.reference_cnil,
        retard_motif: b.retard_motif,
        non_notification_motif: b.non_notification_motif,
        personnes_informees_le: b.personnes_informees_le,
        personnes_informees_nombre: b.personnes_informees_nombre,
        personnes_non_information_motif: b.personnes_non_information_motif,
        etat: b.etat,
        close_le: b.close_le,
        retour_experience: b.retour_experience,
        revue_le: b.revue_le,
        mesures,
        notifications,
        chronologie,
        plateforme_element,
    })
}

// ---------------------------------------------------------------------------------------------
// Chronologie (R-RG40) et alertes (RGP-242)
// ---------------------------------------------------------------------------------------------

pub(crate) async fn etape(
    conn: &mut PgConnection,
    etab: Uuid,
    violation: Uuid,
    code: &str,
    texte: Option<&str>,
    par_compte: Option<Uuid>,
    par_role: Option<&str>,
) -> Result<(), EntDbError> {
    sqlx::query(
        "INSERT INTO rgp_violation_etape (id, etablissement_id, violation_id, etape_code, texte, par_compte_id, par_role)
         VALUES ($1, $2, $3, $4, $5, $6, $7)",
    )
    .bind(Uuid::new_v4())
    .bind(etab)
    .bind(violation)
    .bind(code)
    .bind(texte)
    .bind(par_compte)
    .bind(par_role)
    .execute(&mut *conn)
    .await
    .map_err(db)?;
    Ok(())
}

/// Comptes à prévenir : ceux qui gèrent ou décident (moteur de droits, jamais une liste figée de rôles), accès ouvert.
pub(crate) async fn gestionnaires(
    ent: &PgEnt,
    conn: &mut PgConnection,
    etab: Uuid,
    jour: NaiveDate,
) -> Result<Vec<Uuid>, EntDbError> {
    let membres: Vec<(Uuid, String)> = sqlx::query_as(
        "SELECT m.user_id, m.role FROM ent_membership m
          WHERE m.etablissement_id = $1
            AND m.role NOT IN ('parent', 'eleve')
            AND COALESCE(m.etat, 'ouvert') = 'ouvert'
            AND (m.date_fin IS NULL OR m.date_fin >= $2)
          ORDER BY m.user_id",
    )
    .bind(etab)
    .bind(jour)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    let mut out = Vec::new();
    for (u, role) in membres {
        let droits = ent.droits_effectifs_conn(&mut *conn, etab, u, &role).await?;
        if droits
            .iter()
            .any(|d| matches!(d.action, Action::ViolationsGerer | Action::ViolationsDecider))
            && !out.contains(&u)
        {
            out.push(u);
        }
    }
    Ok(out)
}

/// Instant lisible dans le fuseau de l'établissement (« 12/03/2027 à 14 h 05 »).
async fn instant_local(conn: &mut PgConnection, etab: Uuid, t: DateTime<Utc>) -> Result<String, EntDbError> {
    sqlx::query_scalar(
        "SELECT to_char($2::timestamptz AT TIME ZONE COALESCE(
                    (SELECT p.fuseau FROM ent_etablissement_parametre p WHERE p.etablissement_id = $1),
                    'Europe/Paris'), 'DD/MM/YYYY \"à\" HH24 \"h\" MI')",
    )
    .bind(etab)
    .bind(t)
    .fetch_one(&mut *conn)
    .await
    .map_err(db)
}

/// Alerte urgente (§18.1 : tous canaux, **hors plage calme**) : chaque canal est planifié pour maintenant. Le texte
/// ne nomme ni le déclarant ni une personne touchée (E14) : la référence et l'échéance suffisent.
#[allow(clippy::too_many_arguments)]
async fn alerter(
    conn: &mut PgConnection,
    etab: Uuid,
    violation: Uuid,
    destinataires: &[Uuid],
    kind: &str,
    titre: &str,
    corps: &str,
    cle: &str,
    maintenant: DateTime<Utc>,
) -> Result<usize, EntDbError> {
    let cibles: Vec<(String, DateTime<Utc>, bool)> = ["push", "email", "sms"]
        .iter()
        .map(|c| (c.to_string(), maintenant, false))
        .collect();
    let mut n = 0usize;
    for u in destinataires {
        n += notifier_personne_cibles(
            &mut *conn,
            EnvoiAPlanifier {
                etablissement_id: etab,
                user_id: *u,
                kind,
                titre,
                corps,
                eleve_id: None,
                notification_id: None,
                source_type: "systeme",
                source_id: violation,
                cle_dedup: format!("rgpd:violation:{violation}:{cle}"),
                droit_requis: None,
                regroupable: false,
                canaux_voulus: &["push", "email", "sms"],
            },
            // Une alerte par palier et par violation : la source de l'avis est dérivée de la violation et du palier.
            source_alerte(violation, cle),
            &cibles,
        )
        .await?;
    }
    Ok(n)
}

/// Identifiant d'avis stable par (violation, palier) : deux paliers ne se confondent jamais dans `ent_notification`
/// (unicité sur la source, le destinataire et la nature), et une même alerte rejouée reste une seule.
fn source_alerte(violation: Uuid, cle: &str) -> Uuid {
    let mut h: u128 = 0xcbf2_9ce4_8422_2325_cbf2_9ce4_8422_2325;
    for b in cle.bytes() {
        h = (h ^ u128::from(b)).wrapping_mul(0x0000_0000_0100_0000_0000_0000_0000_013b);
    }
    Uuid::from_u128(violation.as_u128() ^ h)
}

pub(crate) async fn alerter_declaration(
    ent: &PgEnt,
    conn: &mut PgConnection,
    etab: Uuid,
    violation: Uuid,
    declarant: Option<Uuid>,
    maintenant: DateTime<Utc>,
) -> Result<usize, EntDbError> {
    let b = charger_brut(conn, etab, violation).await?;
    let jour = jour_etablissement(conn, etab, maintenant).await?;
    let destinataires: Vec<Uuid> = gestionnaires(ent, conn, etab, jour)
        .await?
        .into_iter()
        .filter(|u| Some(*u) != declarant)
        .collect();
    let corps = if b.nature_incident == "violation" {
        format!(
            "L'incident {} a été déclaré. S'il s'agit d'une violation de données, la CNIL doit être notifiée au plus \
             tard le {} (72 h après la prise de connaissance).",
            reference(b.annee, b.numero),
            instant_local(conn, etab, b.echeance_cnil_le).await?
        )
    } else {
        format!(
            "L'incident de sécurité {} a été déclaré. Vérifiez qu'aucune donnée personnelle n'est touchée.",
            reference(b.annee, b.numero)
        )
    };
    let n = alerter(
        conn,
        etab,
        violation,
        &destinataires,
        KIND_DECLAREE,
        "Incident déclaré",
        &corps,
        "declaree",
        maintenant,
    )
    .await?;
    etape(
        conn,
        etab,
        violation,
        "alerte_envoyee",
        Some(&format!("{} gestionnaire(s) prévenu(s)", destinataires.len())),
        None,
        None,
    )
    .await?;
    Ok(n)
}

async fn jour_etablissement(
    conn: &mut PgConnection,
    etab: Uuid,
    maintenant: DateTime<Utc>,
) -> Result<NaiveDate, EntDbError> {
    sqlx::query_scalar(
        "SELECT ($2::timestamptz AT TIME ZONE COALESCE(
                    (SELECT p.fuseau FROM ent_etablissement_parametre p WHERE p.etablissement_id = $1),
                    'Europe/Paris'))::date",
    )
    .bind(etab)
    .bind(maintenant)
    .fetch_one(&mut *conn)
    .await
    .map_err(db)
}

// ---------------------------------------------------------------------------------------------
// Insertion
// ---------------------------------------------------------------------------------------------

pub(crate) struct Insertion<'a> {
    pub(crate) nature_incident: &'a str,
    pub(crate) origine: &'a str,
    pub(crate) plateforme_violation_id: Option<Uuid>,
    pub(crate) source_code: Option<&'a str>,
    pub(crate) source_ref: Option<Uuid>,
    pub(crate) declaree_par: Option<Uuid>,
    pub(crate) intitule: &'a str,
    pub(crate) description: Option<&'a str>,
    pub(crate) constatee_le: Option<DateTime<Utc>>,
    pub(crate) connue_le: DateTime<Utc>,
    pub(crate) personnes_nombre: Option<i32>,
}

/// Insère une violation avec son numéro (dans la transaction : un échec ne consomme pas de numéro). `cree_le` est
/// l'instant de la saisie (`maintenant`), la prise de connaissance le précède ou l'égale.
pub(crate) async fn inserer(
    conn: &mut PgConnection,
    etab: Uuid,
    i: Insertion<'_>,
    maintenant: DateTime<Utc>,
) -> Result<(Uuid, i32, i32), EntDbError> {
    let annee = jour_etablissement(conn, etab, i.connue_le).await?.year();
    let numero: i32 = sqlx::query_scalar("SELECT public.rgp_violation_numero_suivant($1, $2)")
        .bind(etab)
        .bind(annee)
        .fetch_one(&mut *conn)
        .await
        .map_err(db)?;
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO rgp_violation (id, etablissement_id, annee, numero, nature_incident, origine,
                plateforme_violation_id, source_code, source_ref, declaree_par_compte_id, intitule, description,
                constatee_le, connue_le, echeance_cnil_le, personnes_nombre, cree_le, modifie_le)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $17)",
    )
    .bind(id)
    .bind(etab)
    .bind(annee)
    .bind(numero)
    .bind(i.nature_incident)
    .bind(i.origine)
    .bind(i.plateforme_violation_id)
    .bind(i.source_code)
    .bind(i.source_ref)
    .bind(i.declaree_par)
    .bind(i.intitule)
    .bind(i.description)
    .bind(i.constatee_le)
    .bind(i.connue_le)
    .bind(echeance_cnil(i.connue_le))
    .bind(i.personnes_nombre)
    .bind(maintenant)
    .execute(&mut *conn)
    .await
    .map_err(db)?;
    Ok((id, annee, numero))
}

fn valider_declaration(
    d: &Declaration<'_>,
    connue_le: DateTime<Utc>,
) -> Result<(String, Option<String>), EntDbError> {
    let intitule = texte_exige(d.intitule, 3, 200, "dites en une phrase ce qui s'est passé (3 à 200 caractères)")?;
    let description = match d.description.map(str::trim).filter(|v| !v.is_empty()) {
        Some(v) => Some(texte_exige(v, 3, 4000, "précisions : 3 à 4 000 caractères")?),
        None => None,
    };
    if d.constatee_le.is_some_and(|c| c > connue_le) {
        return Err(invalide("la date de constat ne peut pas suivre la prise de connaissance"));
    }
    Ok((intitule, description))
}

fn ma_declaration(b: &Brut) -> MaDeclaration {
    MaDeclaration {
        id: b.id,
        reference: reference(b.annee, b.numero),
        intitule: b.intitule.clone(),
        connue_le: b.connue_le,
        avancement: match b.etat.as_str() {
            "ouverte" => "recue",
            "close" => "close",
            _ => "en_cours",
        },
        message: MERCI_DECLARANT,
    }
}

// ---------------------------------------------------------------------------------------------
// Service
// ---------------------------------------------------------------------------------------------

impl PgEnt {
    async fn violations_exiger_acces_ouvert(&self, etab: Uuid, acteur: Uuid) -> Result<(), EntDbError> {
        let jour = self.aujourdhui_etablissement(etab).await?;
        match self.acces_etat(etab, acteur).await? {
            Some(m) if m.etat == EtatAcces::Ouvert && m.date_fin.is_none_or(|f| f >= jour) => Ok(()),
            _ => Err(EntDbError::Forbidden),
        }
    }

    /// **RGP-240** — un personnel déclare un incident, en trente secondes. La prise de connaissance est l'instant de la
    /// déclaration (horodatage serveur) ; les gestionnaires sont prévenus tout de suite, tous canaux, hors plage calme
    /// (RGP-242). Le déclarant ne reçoit que la référence et un merci (CL27-45) : il ne voit ni la qualification ni la
    /// décision.
    pub async fn violations_declarer(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        d: Declaration<'_>,
    ) -> Result<MaDeclaration, EntDbError> {
        self.violations_declarer_a(etab, acteur, role, d, Utc::now()).await
    }

    pub(crate) async fn violations_declarer_a(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        d: Declaration<'_>,
        maintenant: DateTime<Utc>,
    ) -> Result<MaDeclaration, EntDbError> {
        self.exiger_action_etab(etab, acteur, role, Action::ViolationsDeclarer)
            .await?;
        self.violations_exiger_acces_ouvert(etab, acteur).await?;
        let (intitule, description) = valider_declaration(&d, maintenant)?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let (id, _, _) = inserer(
            &mut tx,
            etab,
            Insertion {
                nature_incident: if d.incident_securite { "incident_securite" } else { "violation" },
                origine: "declaration",
                plateforme_violation_id: None,
                source_code: None,
                source_ref: None,
                declaree_par: Some(acteur),
                intitule: &intitule,
                description: description.as_deref(),
                constatee_le: d.constatee_le,
                connue_le: maintenant,
                personnes_nombre: None,
            },
            maintenant,
        )
        .await?;
        etape(&mut tx, etab, id, "declaree", Some("déclaration par un personnel"), Some(acteur), Some(role)).await?;
        alerter_declaration(self, &mut tx, etab, id, Some(acteur), maintenant).await?;
        let b = charger_brut(&mut tx, etab, id).await?;
        tx.commit().await?;
        Ok(ma_declaration(&b))
    }

    /// Saisie par un gestionnaire (incident appris par téléphone, par courrier, par un sous-traitant local) : la prise
    /// de connaissance peut être datée plus tôt que la saisie, jamais après, jamais plus d'un an en arrière.
    pub async fn violations_saisir(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        d: Declaration<'_>,
        connue_le: Option<DateTime<Utc>>,
    ) -> Result<ViolationVue, EntDbError> {
        self.violations_saisir_a(etab, acteur, role, d, connue_le, Utc::now()).await
    }

    pub(crate) async fn violations_saisir_a(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        d: Declaration<'_>,
        connue_le: Option<DateTime<Utc>>,
        maintenant: DateTime<Utc>,
    ) -> Result<ViolationVue, EntDbError> {
        self.exiger_action_etab(etab, acteur, role, Action::ViolationsGerer)
            .await?;
        let connue_le = connue_le.unwrap_or(maintenant);
        if connue_le > maintenant {
            return Err(invalide("la prise de connaissance ne peut pas être dans le futur"));
        }
        if connue_le < maintenant - Duration::days(CONNUE_LE_MAX_JOURS) {
            return Err(invalide("prise de connaissance trop ancienne (plus d'un an) : vérifiez la date"));
        }
        let (intitule, description) = valider_declaration(&d, connue_le)?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let (id, _, _) = inserer(
            &mut tx,
            etab,
            Insertion {
                nature_incident: if d.incident_securite { "incident_securite" } else { "violation" },
                origine: "declaration",
                plateforme_violation_id: None,
                source_code: None,
                source_ref: None,
                declaree_par: Some(acteur),
                intitule: &intitule,
                description: description.as_deref(),
                constatee_le: d.constatee_le,
                connue_le,
                personnes_nombre: None,
            },
            maintenant,
        )
        .await?;
        let detail = if connue_le < maintenant {
            "saisie par un gestionnaire, prise de connaissance antérieure à la saisie"
        } else {
            "saisie par un gestionnaire"
        };
        etape(&mut tx, etab, id, "declaree", Some(detail), Some(acteur), Some(role)).await?;
        alerter_declaration(self, &mut tx, etab, id, Some(acteur), maintenant).await?;
        let v = charger(&mut tx, etab, id, maintenant).await?;
        tx.commit().await?;
        Ok(v)
    }

    /// **RGP-241** — port `violations::declarer` : déclaration automatique par la supervision (socle 28) ou par un
    /// socle qui détecte une anomalie (export anormal, fuite détectée, comptes compromis). Appel système, sans session.
    /// **Idempotent** sur `(source_code, source_ref)` : la même alerte ne crée jamais deux violations ; rend
    /// l'identifiant et `true` si la violation vient d'être créée.
    pub async fn violations_declarer_systeme(
        &self,
        etab: Uuid,
        source_code: &str,
        source_ref: Uuid,
        intitule: &str,
        description: Option<&str>,
        personnes_nombre: Option<i32>,
    ) -> Result<(Uuid, bool), EntDbError> {
        self.violations_declarer_systeme_a(
            etab,
            source_code,
            source_ref,
            intitule,
            description,
            personnes_nombre,
            Utc::now(),
        )
        .await
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn violations_declarer_systeme_a(
        &self,
        etab: Uuid,
        source_code: &str,
        source_ref: Uuid,
        intitule: &str,
        description: Option<&str>,
        personnes_nombre: Option<i32>,
        maintenant: DateTime<Utc>,
    ) -> Result<(Uuid, bool), EntDbError> {
        let code_valide = (3..=60).contains(&source_code.len())
            && source_code
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '.');
        if !code_valide {
            return Err(invalide("code de source invalide"));
        }
        if personnes_nombre.is_some_and(|n| n < 0) {
            return Err(invalide("nombre de personnes négatif"));
        }
        let (intitule, description) = valider_declaration(
            &Declaration {
                intitule,
                description,
                constatee_le: None,
                incident_securite: false,
            },
            maintenant,
        )?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        // Verrou transactionnel sur la source : deux alertes simultanées de la même source se suivent ; la seconde
        // trouve la première.
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1, 0))")
            .bind(format!("rgp_violation:{etab}:{source_code}:{source_ref}"))
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        let existe: Option<Uuid> = sqlx::query_scalar(
            "SELECT id FROM rgp_violation WHERE etablissement_id = $1 AND source_code = $2 AND source_ref = $3",
        )
        .bind(etab)
        .bind(source_code)
        .bind(source_ref)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        if let Some(id) = existe {
            tx.commit().await?;
            return Ok((id, false));
        }
        let (id, _, _) = inserer(
            &mut tx,
            etab,
            Insertion {
                nature_incident: "violation",
                origine: "supervision",
                plateforme_violation_id: None,
                source_code: Some(source_code),
                source_ref: Some(source_ref),
                declaree_par: None,
                intitule: &intitule,
                description: description.as_deref(),
                constatee_le: None,
                connue_le: maintenant,
                personnes_nombre,
            },
            maintenant,
        )
        .await?;
        etape(
            &mut tx,
            etab,
            id,
            "declaree",
            Some(&format!("déclaration automatique ({source_code})")),
            None,
            Some("systeme"),
        )
        .await?;
        alerter_declaration(self, &mut tx, etab, id, None, maintenant).await?;
        tx.commit().await?;
        Ok((id, true))
    }

    /// Mes déclarations (le déclarant suit l'avancement, sans la qualification ni la décision, R-RG32).
    pub async fn violations_mes_declarations(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
    ) -> Result<Vec<MaDeclaration>, EntDbError> {
        self.exiger_action_etab(etab, acteur, role, Action::ViolationsDeclarer)
            .await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let sql = format!(
            "SELECT {COLONNES} FROM rgp_violation
              WHERE etablissement_id = $1 AND declaree_par_compte_id = $2
              ORDER BY connue_le DESC, numero DESC LIMIT 200"
        );
        let rows = sqlx::query(&sql)
            .bind(etab)
            .bind(acteur)
            .fetch_all(&mut *tx)
            .await
            .map_err(db)?;
        tx.commit().await?;
        Ok(rows.iter().map(|r| ma_declaration(&brut(r))).collect())
    }

    /// **RGP-258** — registre des violations (toutes, notifiées ou non, art. 33.5), du plus récent au plus ancien.
    pub async fn violations_registre(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        f: FiltreRegistre<'_>,
    ) -> Result<Vec<ViolationLigne>, EntDbError> {
        self.violations_registre_a(etab, acteur, role, f, Utc::now()).await
    }

    pub(crate) async fn violations_registre_a(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        f: FiltreRegistre<'_>,
        maintenant: DateTime<Utc>,
    ) -> Result<Vec<ViolationLigne>, EntDbError> {
        self.exiger_action_etab(etab, acteur, role, Action::ViolationsGerer)
            .await?;
        if f.etat.is_some_and(|e| !["ouverte", "qualifiee", "notifiee", "close"].contains(&e)) {
            return Err(invalide("état inconnu"));
        }
        if f.nature_incident.is_some_and(|n| !NATURES_INCIDENT.contains(&n)) {
            return Err(invalide("nature d'incident inconnue"));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let sql = format!(
            "SELECT {COLONNES} FROM rgp_violation
              WHERE etablissement_id = $1
                AND ($2::int IS NULL OR annee = $2)
                AND ($3::text IS NULL OR etat = $3)
                AND ($4::text IS NULL OR nature_incident = $4)
                AND (NOT $5 OR (etat <> 'close' AND nature_incident = 'violation'
                                AND notifiee_cnil_le IS NULL AND non_notification_motif IS NULL))
              ORDER BY annee DESC, numero DESC LIMIT 1000"
        );
        let rows = sqlx::query(&sql)
            .bind(etab)
            .bind(f.annee)
            .bind(f.etat)
            .bind(f.nature_incident)
            .bind(f.a_notifier)
            .fetch_all(&mut *tx)
            .await
            .map_err(db)?;
        tx.commit().await?;
        Ok(rows
            .iter()
            .map(|r| {
                let b = brut(r);
                ViolationLigne {
                    id: b.id,
                    reference: reference(b.annee, b.numero),
                    heures_restantes: b.heures_restantes(maintenant),
                    en_retard: b.en_retard(maintenant),
                    non_notifiee: b.non_notification_motif.is_some(),
                    nature_incident: b.nature_incident,
                    origine: b.origine,
                    intitule: b.intitule,
                    connue_le: b.connue_le,
                    echeance_cnil_le: b.echeance_cnil_le,
                    risque: b.risque,
                    notifiee_cnil_le: b.notifiee_cnil_le,
                    personnes_nombre: b.personnes_nombre,
                    etat: b.etat,
                    close_le: b.close_le,
                }
            })
            .collect())
    }

    /// Fiche d'une violation (gestionnaires).
    pub async fn violations_fiche(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        id: Uuid,
    ) -> Result<ViolationVue, EntDbError> {
        self.violations_fiche_a(etab, acteur, role, id, Utc::now()).await
    }

    pub(crate) async fn violations_fiche_a(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        id: Uuid,
        maintenant: DateTime<Utc>,
    ) -> Result<ViolationVue, EntDbError> {
        self.exiger_action_etab(etab, acteur, role, Action::ViolationsGerer)
            .await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let v = charger(&mut tx, etab, id, maintenant).await?;
        tx.commit().await?;
        Ok(v)
    }

    /// **RGP-243** — qualification guidée : nature de l'atteinte, catégories de données (codes du catalogue de
    /// conservation), personnes (nombre, estimé ou calculé, mineurs), conséquences. Recalcule le risque proposé par la
    /// grille ; ne touche jamais au risque décidé. Une requalification en « incident de sécurité » n'est possible que
    /// tant que rien n'a été notifié ni décidé.
    pub async fn violations_qualifier(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        id: Uuid,
        q: Qualification<'_>,
    ) -> Result<ViolationVue, EntDbError> {
        self.exiger_action_etab(etab, acteur, role, Action::ViolationsGerer)
            .await?;
        if !NATURES_INCIDENT.contains(&q.nature_incident) {
            return Err(invalide("nature d'incident inconnue"));
        }
        if q.personnes_nombre.is_some_and(|n| n < 0) {
            return Err(invalide("nombre de personnes négatif"));
        }
        let mut cats: Vec<&str> = q.categories.iter().map(|c| c.trim()).collect();
        cats.sort_unstable();
        cats.dedup();
        if cats.iter().any(|c| crate::conservation::categorie(c).is_none()) {
            return Err(invalide("catégorie de données inconnue du catalogue"));
        }
        let consequences = match q.consequences.map(str::trim).filter(|v| !v.is_empty()) {
            Some(v) => Some(texte_exige(v, 3, 4000, "conséquences : 3 à 4 000 caractères")?),
            None => None,
        };
        let violation = q.nature_incident == "violation";
        if violation && !(q.confidentialite || q.integrite || q.disponibilite) {
            return Err(invalide(
                "une violation porte atteinte à la confidentialité, à l'intégrité ou à la disponibilité",
            ));
        }
        let maintenant = Utc::now();
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let b = verrouiller(&mut tx, etab, id).await?;
        exiger_non_close(&b)?;
        if !violation
            && (b.risque.is_some() || b.notifiee_cnil_le.is_some() || b.non_notification_motif.is_some())
        {
            return Err(EntDbError::ConflitDossier(
                "la violation est déjà décidée ou notifiée : elle ne redevient pas un simple incident",
            ));
        }
        let propose = violation.then(|| {
            proposer_risque(&EntreesGrille {
                confidentialite: q.confidentialite,
                integrite: q.integrite,
                disponibilite: q.disponibilite,
                categories: &cats,
                personnes_nombre: q.personnes_nombre,
                personnes_mineurs: q.personnes_mineurs,
            })
        });
        sqlx::query(
            "UPDATE rgp_violation
                SET nature_incident = $3, confidentialite = $4, integrite = $5, disponibilite = $6,
                    personnes_nombre = $7, personnes_estime = $8, personnes_mineurs = $9, consequences = $10,
                    qualifiee_le = $11, qualifiee_par_compte_id = $12, risque_propose = $13,
                    etat = CASE WHEN etat = 'ouverte' THEN 'qualifiee' ELSE etat END,
                    modifie_le = $11
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(id)
        .bind(q.nature_incident)
        .bind(q.confidentialite)
        .bind(q.integrite)
        .bind(q.disponibilite)
        .bind(q.personnes_nombre)
        .bind(q.personnes_estime)
        .bind(q.personnes_mineurs)
        .bind(consequences.as_deref())
        .bind(maintenant)
        .bind(acteur)
        .bind(propose.as_ref().map(|p| p.risque))
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        let avant = categories_de(&mut tx, etab, id).await?;
        let apres: Vec<String> = cats.iter().map(|c| c.to_string()).collect();
        if avant != apres {
            sqlx::query("DELETE FROM rgp_violation_categorie WHERE etablissement_id = $1 AND violation_id = $2")
                .bind(etab)
                .bind(id)
                .execute(&mut *tx)
                .await
                .map_err(db)?;
            for c in &cats {
                sqlx::query(
                    "INSERT INTO rgp_violation_categorie (id, etablissement_id, violation_id, categorie_code)
                     VALUES ($1, $2, $3, $4)",
                )
                .bind(Uuid::new_v4())
                .bind(etab)
                .bind(id)
                .bind(*c)
                .execute(&mut *tx)
                .await
                .map_err(db)?;
            }
            etape(
                &mut tx,
                etab,
                id,
                "categories",
                Some(&format!("{} catégorie(s) de données", cats.len())),
                Some(acteur),
                Some(role),
            )
            .await?;
        }
        etape(
            &mut tx,
            etab,
            id,
            "qualifiee",
            Some(if violation { "violation de données" } else { "incident de sécurité sans atteinte aux données" }),
            Some(acteur),
            Some(role),
        )
        .await?;
        if let Some(p) = &propose {
            if b.risque_propose.as_deref() != Some(p.risque) {
                etape(
                    &mut tx,
                    etab,
                    id,
                    "risque_propose",
                    Some(&format!("grille : {}", p.risque)),
                    None,
                    Some("systeme"),
                )
                .await?;
            }
        }
        let v = charger(&mut tx, etab, id, maintenant).await?;
        tx.commit().await?;
        Ok(v)
    }

    /// **RGP-246** — consigne une mesure prise ou envisagée (art. 33.3 d). Les mesures ne s'effacent pas.
    pub async fn violations_ajouter_mesure(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        id: Uuid,
        nature: &str,
        texte_mesure: &str,
    ) -> Result<ViolationVue, EntDbError> {
        self.exiger_action_etab(etab, acteur, role, Action::ViolationsGerer)
            .await?;
        if !["prise", "envisagee"].contains(&nature) {
            return Err(invalide("une mesure est « prise » ou « envisagée »"));
        }
        let t = texte_exige(texte_mesure, 3, 2000, "mesure : 3 à 2 000 caractères")?;
        let maintenant = Utc::now();
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let b = verrouiller(&mut tx, etab, id).await?;
        exiger_non_close(&b)?;
        sqlx::query(
            "INSERT INTO rgp_violation_mesure (id, etablissement_id, violation_id, nature, texte, le, par_compte_id)
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(Uuid::new_v4())
        .bind(etab)
        .bind(id)
        .bind(nature)
        .bind(&t)
        .bind(maintenant)
        .bind(acteur)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        etape(
            &mut tx,
            etab,
            id,
            "mesure",
            Some(if nature == "prise" { "mesure prise" } else { "mesure envisagée" }),
            Some(acteur),
            Some(role),
        )
        .await?;
        let v = charger(&mut tx, etab, id, maintenant).await?;
        tx.commit().await?;
        Ok(v)
    }

    /// Consigne l'avis du DPO (préalable à la décision du risque, RGP-245). Un nouvel avis remplace le précédent ; la
    /// chronologie garde chaque passage.
    pub async fn violations_avis_dpo(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        id: Uuid,
        avis: &str,
    ) -> Result<ViolationVue, EntDbError> {
        self.exiger_action_etab(etab, acteur, role, Action::ViolationsGerer)
            .await?;
        let avis = texte_exige(avis, 3, 2000, "avis du DPO : 3 à 2 000 caractères")?;
        let maintenant = Utc::now();
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let b = verrouiller(&mut tx, etab, id).await?;
        exiger_non_close(&b)?;
        exiger_violation(&b)?;
        sqlx::query(
            "UPDATE rgp_violation SET avis_dpo = $3, avis_dpo_le = $4, avis_dpo_par_compte_id = $5, modifie_le = $4
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(id)
        .bind(&avis)
        .bind(maintenant)
        .bind(acteur)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        etape(&mut tx, etab, id, "avis_dpo", Some("avis du DPO consigné"), Some(acteur), Some(role)).await?;
        let v = charger(&mut tx, etab, id, maintenant).await?;
        tx.commit().await?;
        Ok(v)
    }

    /// **RGP-245 ★** — la direction décide le niveau de risque, motivé, après l'avis du DPO consigné. La grille a
    /// proposé ; la décision peut s'en écarter, le motif le dit (R-RG34). Une décision peut être revue tant que la
    /// violation n'est pas close (faits nouveaux) ; revenir sous « élevé » après l'information des personnes ne défait
    /// rien de ce qui a été envoyé.
    pub async fn violations_decider_risque(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        id: Uuid,
        risque: &str,
        motif: &str,
    ) -> Result<ViolationVue, EntDbError> {
        self.exiger_action_etab(etab, acteur, role, Action::ViolationsDecider)
            .await?;
        if !RISQUES.contains(&risque) {
            return Err(invalide("risque inconnu : faible, avéré ou élevé"));
        }
        let motif = texte_exige(motif, 3, 2000, "motif de la décision : 3 à 2 000 caractères")?;
        let maintenant = Utc::now();
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let b = verrouiller(&mut tx, etab, id).await?;
        exiger_non_close(&b)?;
        exiger_violation(&b)?;
        if b.qualifiee_le.is_none() {
            return Err(EntDbError::ConflitDossier("qualifier la violation avant de décider le risque"));
        }
        if b.avis_dpo.is_none() {
            return Err(EntDbError::ConflitDossier("consigner l'avis du DPO avant de décider le risque"));
        }
        if risque != "faible" && b.non_notification_motif.is_some() {
            return Err(EntDbError::ConflitDossier(
                "la non-notification a été justifiée pour un risque faible : notifiez la CNIL",
            ));
        }
        sqlx::query(
            "UPDATE rgp_violation
                SET risque = $3, risque_motif = $4, risque_decide_le = $5, risque_decide_par_compte_id = $6,
                    modifie_le = $5
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(id)
        .bind(risque)
        .bind(&motif)
        .bind(maintenant)
        .bind(acteur)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        let ecart = b.risque_propose.as_deref().is_some_and(|p| p != risque);
        etape(
            &mut tx,
            etab,
            id,
            "risque_decide",
            Some(&format!(
                "risque {risque}{}",
                if ecart { " (écart à la proposition de la grille, motivé)" } else { "" }
            )),
            Some(acteur),
            Some(role),
        )
        .await?;
        let v = charger(&mut tx, etab, id, maintenant).await?;
        tx.commit().await?;
        Ok(v)
    }

    /// **RGP-248, RGP-250** — saisit la notification faite par l'établissement sur le téléservice de la CNIL (ScolIA
    /// ne transmet rien) : date, référence. Au-delà des 72 h, **acceptée avec le motif du retard** (R-RG21,
    /// CL27-46), jamais refusée. Possible avant la décision du risque (notification par phases, art. 33.4).
    #[allow(clippy::too_many_arguments)]
    pub async fn violations_notifier_cnil(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        id: Uuid,
        notifiee_le: DateTime<Utc>,
        reference_cnil: &str,
        retard_motif: Option<&str>,
    ) -> Result<ViolationVue, EntDbError> {
        self.exiger_action_etab(etab, acteur, role, Action::ViolationsGerer)
            .await?;
        let reference_cnil = texte_exige(reference_cnil, 2, 120, "référence de la notification CNIL : 2 à 120 caractères")?;
        let retard_motif = match retard_motif.map(str::trim).filter(|v| !v.is_empty()) {
            Some(v) => Some(texte_exige(v, 3, 2000, "motif du retard : 3 à 2 000 caractères")?),
            None => None,
        };
        let maintenant = Utc::now();
        if notifiee_le > maintenant {
            return Err(invalide("la notification ne peut pas être datée dans le futur"));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let b = verrouiller(&mut tx, etab, id).await?;
        exiger_non_close(&b)?;
        exiger_violation(&b)?;
        if b.notifiee_cnil_le.is_some() {
            return Err(EntDbError::ConflitDossier(
                "la CNIL est déjà notifiée : saisissez une notification complémentaire",
            ));
        }
        if b.non_notification_motif.is_some() {
            return Err(EntDbError::ConflitDossier(
                "la non-notification a été justifiée ; une notification la remplace seulement après une nouvelle décision du risque",
            ));
        }
        if notifiee_le < b.connue_le {
            return Err(invalide("la notification ne peut pas précéder la prise de connaissance"));
        }
        let tardive = notifiee_le > b.echeance_cnil_le;
        if tardive && retard_motif.is_none() {
            return Err(invalide(
                "notification au-delà de 72 h : indiquez le motif du retard (art. 33.1) ; elle est acceptée",
            ));
        }
        sqlx::query(
            "UPDATE rgp_violation
                SET notifiee_cnil_le = $3, reference_cnil = $4, retard_motif = $5, etat = 'notifiee', modifie_le = $6
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(id)
        .bind(notifiee_le)
        .bind(&reference_cnil)
        .bind(if tardive { retard_motif.as_deref() } else { None })
        .bind(maintenant)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        sqlx::query(
            "INSERT INTO rgp_violation_notification (id, etablissement_id, violation_id, rang, nature, notifiee_le,
                    reference_cnil, par_compte_id)
             VALUES ($1, $2, $3, 1, 'initiale', $4, $5, $6)",
        )
        .bind(Uuid::new_v4())
        .bind(etab)
        .bind(id)
        .bind(notifiee_le)
        .bind(&reference_cnil)
        .bind(acteur)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        etape(
            &mut tx,
            etab,
            id,
            "notifiee_cnil",
            Some(if tardive { "notification à la CNIL, au-delà de 72 h, retard motivé" } else { "notification à la CNIL dans les 72 h" }),
            Some(acteur),
            Some(role),
        )
        .await?;
        let v = charger(&mut tx, etab, id, maintenant).await?;
        tx.commit().await?;
        Ok(v)
    }

    /// **RGP-249** — notification complémentaire (art. 33.4 : informations communiquées par phases).
    #[allow(clippy::too_many_arguments)]
    pub async fn violations_notification_complementaire(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        id: Uuid,
        notifiee_le: DateTime<Utc>,
        reference_cnil: &str,
        resume: &str,
    ) -> Result<ViolationVue, EntDbError> {
        self.exiger_action_etab(etab, acteur, role, Action::ViolationsGerer)
            .await?;
        let reference_cnil = texte_exige(reference_cnil, 2, 120, "référence de la notification CNIL : 2 à 120 caractères")?;
        let resume = texte_exige(resume, 3, 2000, "résumé du complément : 3 à 2 000 caractères")?;
        let maintenant = Utc::now();
        if notifiee_le > maintenant {
            return Err(invalide("la notification ne peut pas être datée dans le futur"));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let b = verrouiller(&mut tx, etab, id).await?;
        exiger_non_close(&b)?;
        let Some(initiale) = b.notifiee_cnil_le else {
            return Err(EntDbError::ConflitDossier("saisissez d'abord la notification initiale"));
        };
        if notifiee_le < initiale {
            return Err(invalide("un complément ne précède pas la notification initiale"));
        }
        let rang: i32 = sqlx::query_scalar(
            "SELECT COALESCE(max(rang), 0) + 1 FROM rgp_violation_notification
              WHERE etablissement_id = $1 AND violation_id = $2",
        )
        .bind(etab)
        .bind(id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        sqlx::query(
            "INSERT INTO rgp_violation_notification (id, etablissement_id, violation_id, rang, nature, notifiee_le,
                    reference_cnil, resume, par_compte_id)
             VALUES ($1, $2, $3, $4, 'complementaire', $5, $6, $7, $8)",
        )
        .bind(Uuid::new_v4())
        .bind(etab)
        .bind(id)
        .bind(rang)
        .bind(notifiee_le)
        .bind(&reference_cnil)
        .bind(&resume)
        .bind(acteur)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        sqlx::query("UPDATE rgp_violation SET modifie_le = $3 WHERE etablissement_id = $1 AND id = $2")
            .bind(etab)
            .bind(id)
            .bind(maintenant)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        etape(
            &mut tx,
            etab,
            id,
            "notification_complementaire",
            Some(&format!("notification complémentaire n° {rang}")),
            Some(acteur),
            Some(role),
        )
        .await?;
        let v = charger(&mut tx, etab, id, maintenant).await?;
        tx.commit().await?;
        Ok(v)
    }

    /// **RGP-253 ★** — justifie la non-notification (art. 33.5) : seulement pour un risque décidé « faible ». La
    /// violation reste au registre, avec sa justification.
    pub async fn violations_justifier_non_notification(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        id: Uuid,
        motif: &str,
    ) -> Result<ViolationVue, EntDbError> {
        self.exiger_action_etab(etab, acteur, role, Action::ViolationsDecider)
            .await?;
        let motif = texte_exige(motif, 3, 2000, "justification : 3 à 2 000 caractères")?;
        let maintenant = Utc::now();
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let b = verrouiller(&mut tx, etab, id).await?;
        exiger_non_close(&b)?;
        exiger_violation(&b)?;
        if b.notifiee_cnil_le.is_some() {
            return Err(EntDbError::ConflitDossier("la CNIL est déjà notifiée"));
        }
        if b.risque.as_deref() != Some("faible") {
            return Err(EntDbError::ConflitDossier(
                "la non-notification n'est possible que pour un risque décidé « faible » (art. 33.1)",
            ));
        }
        sqlx::query(
            "UPDATE rgp_violation SET non_notification_motif = $3, modifie_le = $4
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(id)
        .bind(&motif)
        .bind(maintenant)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        etape(&mut tx, etab, id, "non_notification", Some("non-notification justifiée (risque faible)"), Some(acteur), Some(role))
            .await?;
        let v = charger(&mut tx, etab, id, maintenant).await?;
        tx.commit().await?;
        Ok(v)
    }

    /// **RGP-252 ★ (exception)** — motive l'absence d'information des personnes pour un risque élevé (art. 34.3 :
    /// mesures qui rendent les données incompréhensibles, mesures ultérieures qui écartent le risque, efforts
    /// disproportionnés remplacés par une communication publique). L'information elle-même part par le socle 8.
    pub async fn violations_motiver_non_information(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        id: Uuid,
        motif: &str,
    ) -> Result<ViolationVue, EntDbError> {
        self.exiger_action_etab(etab, acteur, role, Action::ViolationsDecider)
            .await?;
        let motif = texte_exige(motif, 3, 2000, "motif de l'exception (art. 34.3) : 3 à 2 000 caractères")?;
        let maintenant = Utc::now();
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let b = verrouiller(&mut tx, etab, id).await?;
        exiger_non_close(&b)?;
        exiger_violation(&b)?;
        if b.personnes_informees_le.is_some() {
            return Err(EntDbError::ConflitDossier("les personnes ont déjà été informées"));
        }
        if b.risque.as_deref() != Some("eleve") {
            return Err(EntDbError::ConflitDossier(
                "l'information des personnes n'est exigée que pour un risque élevé décidé",
            ));
        }
        sqlx::query(
            "UPDATE rgp_violation SET personnes_non_information_motif = $3, modifie_le = $4
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(id)
        .bind(&motif)
        .bind(maintenant)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        etape(
            &mut tx,
            etab,
            id,
            "personnes_non_informees",
            Some("exception à l'information des personnes motivée (art. 34.3)"),
            Some(acteur),
            Some(role),
        )
        .await?;
        let v = charger(&mut tx, etab, id, maintenant).await?;
        tx.commit().await?;
        Ok(v)
    }

    /// **RGP-256** — clôture : la décision est complète (risque, notification ou non-notification justifiée,
    /// information des personnes pour un risque élevé), le retour d'expérience est écrit, la revue est fixée à
    /// 30 jours. Après la clôture, plus rien ne change (la base le garantit).
    pub async fn violations_clore(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        id: Uuid,
        retour_experience: &str,
    ) -> Result<ViolationVue, EntDbError> {
        self.exiger_action_etab(etab, acteur, role, Action::ViolationsGerer)
            .await?;
        let rex = texte_exige(
            retour_experience,
            3,
            4000,
            "retour d'expérience : ce qui a été appris et corrigé (3 à 4 000 caractères)",
        )?;
        let maintenant = Utc::now();
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let b = verrouiller(&mut tx, etab, id).await?;
        exiger_non_close(&b)?;
        if b.nature_incident == "violation" && !b.manque_pour_clore().is_empty() {
            return Err(EntDbError::ConflitDossierDetail(format!(
                "clôture impossible : {}",
                b.manque_pour_clore().join(" ; ")
            )));
        }
        let jour = jour_etablissement(&mut tx, etab, maintenant).await?;
        sqlx::query(
            "UPDATE rgp_violation
                SET etat = 'close', close_le = $3, close_par_compte_id = $4, retour_experience = $5, revue_le = $6,
                    modifie_le = $3
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(id)
        .bind(maintenant)
        .bind(acteur)
        .bind(&rex)
        .bind(jour + Duration::days(REVUE_JOURS))
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        etape(&mut tx, etab, id, "close", Some("clôture, revue à 30 jours"), Some(acteur), Some(role)).await?;
        let v = charger(&mut tx, etab, id, maintenant).await?;
        tx.commit().await?;
        Ok(v)
    }

    /// Tâche : alerte les gestionnaires à H-24 puis H-6 de l'échéance CNIL d'une violation non réglée (ni notifiée,
    /// ni justifiée). Le palier **se franchit** (une tâche arrêtée une heure ne fait pas manquer l'alerte : le plus
    /// serré des paliers atteints est retenu) ; la clé de déduplication porte le palier. Une échéance dépassée n'alerte
    /// plus : le retard est affiché au registre. Rend le nombre d'avis créés.
    pub async fn violations_alerter_echeances(&self, etab: Uuid) -> Result<usize, EntDbError> {
        self.violations_alerter_echeances_a(etab, Utc::now()).await
    }

    /// Tâche planifiée : alertes d'échéance CNIL de tous les établissements (un échec n'arrête pas les autres).
    pub async fn violations_echeances_tache(&self) -> Result<usize, EntDbError> {
        let mut total = 0;
        for etab in etablissements_systeme(self.pool(), ContexteSysteme::JobPlanifie("violations_echeances"))
            .await
            .map_err(db)?
        {
            match self.violations_alerter_echeances(etab).await {
                Ok(n) => total += n,
                Err(e) => tracing::warn!(error = %e, "alertes d'échéance des violations non exécutées pour un établissement"),
            }
        }
        Ok(total)
    }

    pub(crate) async fn violations_alerter_echeances_a(
        &self,
        etab: Uuid,
        maintenant: DateTime<Utc>,
    ) -> Result<usize, EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let sql = format!(
            "SELECT {COLONNES} FROM rgp_violation
              WHERE etablissement_id = $1 AND etat <> 'close' AND nature_incident = 'violation'
                AND notifiee_cnil_le IS NULL AND non_notification_motif IS NULL
                AND echeance_cnil_le > $2"
        );
        let rows = sqlx::query(&sql)
            .bind(etab)
            .bind(maintenant)
            .fetch_all(&mut *tx)
            .await
            .map_err(db)?;
        if rows.is_empty() {
            tx.commit().await?;
            return Ok(0);
        }
        let jour = jour_etablissement(&mut tx, etab, maintenant).await?;
        let destinataires = gestionnaires(self, &mut tx, etab, jour).await?;
        let mut envoyes = 0usize;
        for r in &rows {
            let b = brut(r);
            let restant = b.echeance_cnil_le - maintenant;
            let Some(palier) = ALERTE_ECHEANCE_HEURES
                .iter()
                .copied()
                .filter(|h| restant <= Duration::hours(*h))
                .min()
            else {
                continue;
            };
            let deja: bool = sqlx::query_scalar(
                "SELECT EXISTS (SELECT 1 FROM rgp_violation_etape
                                 WHERE etablissement_id = $1 AND violation_id = $2 AND etape_code = 'echeance_alertee'
                                   AND texte = $3)",
            )
            .bind(etab)
            .bind(b.id)
            .bind(format!("H-{palier}"))
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
            if deja {
                continue;
            }
            let corps = format!(
                "Violation {} : la notification à la CNIL est attendue au plus tard le {}. Notifiez, ou faites décider \
                 et justifier la non-notification. Une notification tardive reste possible, avec son motif.",
                reference(b.annee, b.numero),
                instant_local(&mut tx, etab, b.echeance_cnil_le).await?
            );
            envoyes += alerter(
                &mut tx,
                etab,
                b.id,
                &destinataires,
                KIND_ECHEANCE,
                "Échéance CNIL proche",
                &corps,
                &format!("echeance_h{palier}"),
                maintenant,
            )
            .await?;
            etape(&mut tx, etab, b.id, "echeance_alertee", Some(&format!("H-{palier}")), None, Some("systeme")).await?;
        }
        tx.commit().await?;
        Ok(envoyes)
    }
}

/// Une rubrique du brouillon de notification (art. 33.3).
#[derive(Clone, Debug, Serialize)]
pub struct Rubrique {
    pub titre: &'static str,
    pub texte: String,
}

/// Brouillon de notification à la CNIL (RGP-247) : rubriques de l'art. 33.3, pré-remplies depuis la fiche, à recopier
/// sur le téléservice. Déterministe (aucune IA : le brouillon rédigé par l'IA est le lot G11). Une rubrique non
/// renseignée le dit (R-RG11) et figure dans `a_completer`.
#[derive(Clone, Debug, Serialize)]
pub struct BrouillonCnil {
    pub reference: String,
    pub rubriques: Vec<Rubrique>,
    pub a_completer: Vec<&'static str>,
}

const NON_RENSEIGNE: &str = "non renseigné";

/// Assemble le brouillon (fonction pure, testée sans base).
pub fn assembler_brouillon(v: &ViolationVue, dpo: Option<(&str, &str)>, connue_local: &str, echeance_local: &str) -> BrouillonCnil {
    let mut a_completer = Vec::new();
    let mut atteintes = Vec::new();
    if v.confidentialite {
        atteintes.push("confidentialité (divulgation ou accès non autorisé)");
    }
    if v.integrite {
        atteintes.push("intégrité (altération)");
    }
    if v.disponibilite {
        atteintes.push("disponibilité (perte ou indisponibilité)");
    }
    if atteintes.is_empty() {
        a_completer.push("nature de l'atteinte");
    }
    let categories: Vec<String> = v
        .categories
        .iter()
        .map(|c| {
            crate::conservation::categorie(c)
                .map(|k| k.libelle.to_string())
                .unwrap_or_else(|| c.clone())
        })
        .collect();
    if categories.is_empty() {
        a_completer.push("catégories de données");
    }
    let personnes = match v.personnes_nombre {
        Some(n) => format!(
            "{} {n} personne(s){}",
            if v.personnes_estime { "environ" } else { "exactement" },
            if v.personnes_mineurs { ", dont des mineurs" } else { "" }
        ),
        None => {
            a_completer.push("nombre de personnes concernées");
            NON_RENSEIGNE.to_string()
        }
    };
    let consequences = v.consequences.clone().unwrap_or_else(|| {
        a_completer.push("conséquences probables");
        NON_RENSEIGNE.to_string()
    });
    let mesures = |nature: &str| -> Vec<String> {
        v.mesures.iter().filter(|m| m.nature == nature).map(|m| format!("- {}", m.texte)).collect()
    };
    let (prises, envisagees) = (mesures("prise"), mesures("envisagee"));
    if prises.is_empty() && envisagees.is_empty() {
        a_completer.push("mesures prises ou envisagées");
    }
    let dpo_texte = match dpo {
        Some((nom, contact)) => format!("{nom} — {contact}"),
        None => {
            a_completer.push("coordonnées du délégué à la protection des données (ou d'un point de contact)");
            NON_RENSEIGNE.to_string()
        }
    };
    let mut rubriques = vec![
        Rubrique {
            titre: "Description de la violation",
            texte: format!(
                "{}{}\nPrise de connaissance : {connue_local}. Échéance de 72 h : {echeance_local}.",
                v.intitule,
                v.description.as_deref().map(|d| format!("\n{d}")).unwrap_or_default()
            ),
        },
        Rubrique {
            titre: "Nature de l'atteinte (art. 33.3 a)",
            texte: if atteintes.is_empty() { NON_RENSEIGNE.into() } else { atteintes.join(", ") },
        },
        Rubrique {
            titre: "Catégories de données concernées (art. 33.3 a)",
            texte: if categories.is_empty() { NON_RENSEIGNE.into() } else { categories.join(" ; ") },
        },
        Rubrique { titre: "Personnes concernées (art. 33.3 a)", texte: personnes },
        Rubrique { titre: "Délégué à la protection des données ou point de contact (art. 33.3 b)", texte: dpo_texte },
        Rubrique { titre: "Conséquences probables (art. 33.3 c)", texte: consequences },
        Rubrique {
            titre: "Mesures prises (art. 33.3 d)",
            texte: if prises.is_empty() { NON_RENSEIGNE.into() } else { prises.join("\n") },
        },
        Rubrique {
            titre: "Mesures envisagées (art. 33.3 d)",
            texte: if envisagees.is_empty() { NON_RENSEIGNE.into() } else { envisagees.join("\n") },
        },
    ];
    if let Some(r) = &v.risque {
        rubriques.push(Rubrique {
            titre: "Niveau de risque décidé",
            texte: format!(
                "{} — {}",
                match r.as_str() {
                    "eleve" => "élevé",
                    "avere" => "avéré",
                    _ => "faible",
                },
                v.risque_motif.clone().unwrap_or_default()
            ),
        });
    } else {
        a_completer.push("niveau de risque (décision de la direction après avis du DPO)");
    }
    if v.en_retard || v.heures_restantes < 0 {
        rubriques.push(Rubrique {
            titre: "Motif du retard (art. 33.1)",
            texte: "à indiquer : la notification intervient au-delà de 72 heures".into(),
        });
        a_completer.push("motif du retard");
    }
    BrouillonCnil { reference: v.reference.clone(), rubriques, a_completer }
}

/// Échappement d'une cellule CSV (séparateur `;`, guillemets doublés) ; une cellule qui commence par `=`, `+`, `-`
/// ou `@` est préfixée d'une apostrophe (injection de formule dans un tableur).
fn csv(v: &str) -> String {
    let v = if v.starts_with(['=', '+', '-', '@']) { format!("'{v}") } else { v.to_string() };
    if v.contains([';', '"', '\n', '\r']) {
        format!("\"{}\"", v.replace('"', "\"\""))
    } else {
        v
    }
}

impl PgEnt {
    /// **RGP-247** — brouillon de notification à la CNIL, rubrique par rubrique (art. 33.3).
    pub async fn violations_brouillon_cnil(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        id: Uuid,
    ) -> Result<BrouillonCnil, EntDbError> {
        self.exiger_action_etab(etab, acteur, role, Action::ViolationsGerer)
            .await?;
        let maintenant = Utc::now();
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let v = charger(&mut tx, etab, id, maintenant).await?;
        if v.nature_incident != "violation" {
            return Err(EntDbError::ConflitDossier(
                "un incident de sécurité sans atteinte aux données ne se notifie pas",
            ));
        }
        let dpo: Option<(String, String)> = sqlx::query_as(
            "SELECT dpo_nom, dpo_contact FROM rgp_gouvernance
              WHERE etablissement_id = $1 AND dpo_nom IS NOT NULL AND dpo_contact IS NOT NULL",
        )
        .bind(etab)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let connue = instant_local(&mut tx, etab, v.connue_le).await?;
        let echeance = instant_local(&mut tx, etab, v.echeance_cnil_le).await?;
        tx.commit().await?;
        Ok(assembler_brouillon(
            &v,
            dpo.as_ref().map(|(n, c)| (n.as_str(), c.as_str())),
            &connue,
            &echeance,
        ))
    }

    /// **RGP-258** — export du registre (CSV, `;`, UTF-8) : toutes les violations, notifiées ou non, avec la
    /// justification de la non-notification (art. 33.5). Sans description libre ni personne nommée : le registre
    /// prouve la tenue, la fiche porte le détail.
    pub async fn violations_registre_csv(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        annee: Option<i32>,
    ) -> Result<String, EntDbError> {
        self.exiger_action_etab(etab, acteur, role, Action::ViolationsGerer)
            .await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let rows = sqlx::query(
            "SELECT v.annee, v.numero, v.nature_incident, v.origine, v.intitule,
                    to_char(v.connue_le AT TIME ZONE f.fuseau, 'YYYY-MM-DD HH24:MI') AS connue,
                    to_char(v.notifiee_cnil_le AT TIME ZONE f.fuseau, 'YYYY-MM-DD HH24:MI') AS notifiee,
                    v.reference_cnil, v.retard_motif, v.non_notification_motif, v.risque, v.personnes_nombre,
                    v.personnes_estime, (v.personnes_informees_le IS NOT NULL) AS informees,
                    v.personnes_non_information_motif, v.etat,
                    to_char(v.close_le AT TIME ZONE f.fuseau, 'YYYY-MM-DD') AS close,
                    (SELECT string_agg(c.categorie_code, ',' ORDER BY c.categorie_code) FROM rgp_violation_categorie c
                      WHERE c.etablissement_id = v.etablissement_id AND c.violation_id = v.id) AS categories
               FROM rgp_violation v
              CROSS JOIN (SELECT COALESCE((SELECT p.fuseau FROM ent_etablissement_parametre p
                                            WHERE p.etablissement_id = $1), 'Europe/Paris') AS fuseau) f
              WHERE v.etablissement_id = $1 AND ($2::int IS NULL OR v.annee = $2)
              ORDER BY v.annee, v.numero",
        )
        .bind(etab)
        .bind(annee)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await?;
        let mut out = String::from(
            "reference;nature;origine;intitule;connue_le;categories;personnes;estime;risque;notifiee_cnil_le;\
             reference_cnil;motif_retard;motif_non_notification;personnes_informees;motif_non_information;etat;close_le\n",
        );
        for r in rows {
            let opt = |c: &str| -> String { r.get::<Option<String>, _>(c).map(|v| csv(&v)).unwrap_or_default() };
            let ligne = [
                reference(r.get("annee"), r.get("numero")),
                csv(r.get::<&str, _>("nature_incident")),
                csv(r.get::<&str, _>("origine")),
                csv(r.get::<&str, _>("intitule")),
                opt("connue"),
                opt("categories"),
                r.get::<Option<i32>, _>("personnes_nombre").map(|n| n.to_string()).unwrap_or_default(),
                if r.get::<bool, _>("personnes_estime") { "oui".into() } else { "non".into() },
                opt("risque"),
                opt("notifiee"),
                opt("reference_cnil"),
                opt("retard_motif"),
                opt("non_notification_motif"),
                if r.get::<bool, _>("informees") { "oui".into() } else { "non".into() },
                opt("personnes_non_information_motif"),
                csv(r.get::<&str, _>("etat")),
                opt("close"),
            ];
            out.push_str(&ligne.join(";"));
            out.push('\n');
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests_grille {
    use super::*;

    fn g(c: bool, i: bool, d: bool, cats: &[&str], n: Option<i32>, mineurs: bool) -> &'static str {
        proposer_risque(&EntreesGrille {
            confidentialite: c,
            integrite: i,
            disponibilite: d,
            categories: cats,
            personnes_nombre: n,
            personnes_mineurs: mineurs,
        })
        .risque
    }

    #[test]
    fn la_sante_est_toujours_elevee() {
        assert_eq!(g(false, false, true, &["sante.dossier"], Some(1), false), "eleve");
        assert_eq!(g(true, false, false, &["sante.dossier", "scol.registre"], None, false), "eleve");
    }

    #[test]
    fn categorie_particuliere_divulguee_ou_alteree_est_elevee() {
        for c in CATEGORIES_PARTICULIERES {
            assert_eq!(g(true, false, false, &[c], Some(1), false), "eleve", "{c} divulguée");
            assert_eq!(g(false, true, false, &[c], Some(1), false), "eleve", "{c} altérée");
        }
    }

    #[test]
    fn liste_de_classe_envoyee_par_erreur_est_faible_cl27_41() {
        // Nom + classe d'élèves envoyés à toutes les familles d'un niveau : l'identification seule.
        assert_eq!(g(true, false, false, &["scol.registre"], Some(120), false), "faible");
    }

    #[test]
    fn mineurs_divulgues_ou_donnees_ordinaires_sont_averes() {
        assert_eq!(g(true, false, false, &["scol.registre"], Some(3), true), "avere");
        assert_eq!(g(true, false, false, &["eva.note"], Some(3), false), "avere");
        assert_eq!(g(false, true, false, &["vs.appel"], Some(3), false), "avere");
        assert_eq!(g(false, false, true, &["fin.iban"], Some(3), false), "avere");
    }

    #[test]
    fn indisponibilite_ordinaire_ou_rien_de_qualifie_est_faible() {
        assert_eq!(g(false, false, true, &["edt.planning"], Some(500), false), "faible");
        assert_eq!(g(false, false, false, &[], None, false), "faible");
    }

    #[test]
    fn la_grille_donne_ses_raisons() {
        let p = proposer_risque(&EntreesGrille {
            confidentialite: true,
            categories: &["eva.note"],
            personnes_nombre: Some(80),
            ..Default::default()
        });
        assert_eq!(p.risque, "avere");
        assert!(p.raisons.len() >= 2, "{:?}", p.raisons);
    }

    #[test]
    fn echeance_exactement_72_h_et_reference() {
        let t = DateTime::parse_from_rfc3339("2027-03-28T00:30:00Z").unwrap().with_timezone(&Utc);
        // Passage à l'heure d'été la nuit du 28 mars 2027 à Paris : 72 h restent 72 h.
        assert_eq!((echeance_cnil(t) - t).num_hours(), 72);
        assert_eq!(reference(2027, 3), "V-2027-003");
        assert_eq!(reference(2027, 1234), "V-2027-1234");
    }

    #[test]
    fn csv_echappe_et_neutralise_les_formules() {
        assert_eq!(csv("simple"), "simple");
        assert_eq!(csv("a;b"), "\"a;b\"");
        assert_eq!(csv("dit \"x\""), "\"dit \"\"x\"\"\"");
        assert_eq!(csv("=SOMME(A1)"), "'=SOMME(A1)");
        assert_eq!(csv("-1"), "'-1");
    }

    #[test]
    fn categories_proposees_existent_au_catalogue() {
        for c in CATEGORIES_PARTICULIERES.iter().chain(CATEGORIES_IDENTIFICATION.iter()) {
            assert!(crate::conservation::categorie(c).is_some(), "{c} inconnue du catalogue");
        }
    }
}
