//! Socle 42, lot V0a : sorties et voyages scolaires collectifs — **commandes et lectures**
//! (`docs/socles/42_SORTIES_VOYAGES.md` §19, §20, §21).
//!
//! Règles du lot, toutes appliquées ici (jamais dans le navigateur, R-SV1) :
//! * **commande typée** : le droit, l'état de départ, les préconditions et la version sont
//!   vérifiés côté serveur, puis le fait, le journal et l'outbox s'écrivent dans **une** transaction
//!   (`TenantTx`, RLS) ;
//! * **droits** : actions `voyages.*` du catalogue (D-017) à l'échelle de l'établissement, plus les
//!   droits propres au **porteur** de la sortie ; un encadrant lit la sienne ; un professeur non
//!   porteur ne voit ni le détail ni les brouillons des autres (§21.4) ;
//! * **participants = `eleve_id`** du cœur, scolarisé à la date de la sortie (R-SV2) ;
//! * **encadrants** = personnes du socle 6 (agents), jamais un nom libre (VD-19, VD-40) ;
//! * **journal** de la sortie en ajout seul, texte uniquement ; **trace d'accès** sur la liste
//!   nominative, l'export et le registre des élèves (R-SV27) ;
//! * événements `voyages.*` **sans identité d'élève** (identifiants de sortie et états seulement).

use std::collections::HashSet;

use chrono::{DateTime, NaiveDate, NaiveTime, Utc};
use ent_auth::{Action, Portee};
use ent_events::{AggregateKind, MetierEvent, MetierEventDraft};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::{FromRow, PgConnection};
use uuid::Uuid;

use crate::outbox::Publier;
use crate::ref_commun::{conflit, db, introuvable, invalide, Acteur};
use crate::service::{EntDbError, PgEnt, RefusMetier};
use crate::tenant_tx::TenantTx;
use crate::voyages::{
    audit, blocages_liste, blocages_soumission, champ_csv, dates_de_serie, est_modifiable,
    est_terminal, etats_de_depart, prochaine_etape, reference, Commande, Contexte, EtatDossier,
    PointAudit, ProchaineEtape, DESTINATIONS, ETAPES_AMONT, FORMES, MOTIFS_PARTICIPANT, NATURES,
    ROLES_ENCADRANT, TYPES_ETAPE,
};

fn refus(nature: RefusMetier, msg: &'static str) -> EntDbError {
    EntDbError::Refus(nature, msg)
}

fn interdit() -> EntDbError {
    EntDbError::Forbidden
}

async fn tx(e: &PgEnt, a: &Acteur<'_>) -> Result<TenantTx<'static>, EntDbError> {
    TenantTx::begin(e.pool(), a.etablissement_id)
        .await
        .map_err(db)
}

fn texte(v: &str, max: usize) -> Result<String, EntDbError> {
    let t = v.trim();
    if t.is_empty() || t.chars().count() > max {
        return Err(invalide());
    }
    Ok(t.to_string())
}

fn texte_opt(v: Option<&str>, max: usize) -> Result<Option<String>, EntDbError> {
    match v.map(str::trim).filter(|s| !s.is_empty()) {
        None => Ok(None),
        Some(s) if s.chars().count() > max => Err(invalide()),
        Some(s) => Ok(Some(s.to_string())),
    }
}

// ───────────────────────── Types ─────────────────────────

/// Saisie d'une sortie (création, et remplacement des champs du projet à la modification).
/// Les champs d'état, de version et de porteur ne se saisissent jamais ici (R-SV1).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SortieSaisie {
    pub titre: String,
    #[serde(default)]
    pub objectifs: Option<String>,
    pub nature: String,
    pub forme: String,
    pub destination: String,
    #[serde(default)]
    pub pays: Option<String>,
    #[serde(default)]
    pub destination_libelle: Option<String>,
    #[serde(default)]
    pub programme_libelle: Option<String>,
    pub debut: DateTime<Utc>,
    pub fin: DateTime<Utc>,
    /// Classes visées.
    #[serde(default)]
    pub classes: Vec<Uuid>,
    /// Groupes visés.
    #[serde(default)]
    pub groupes: Vec<Uuid>,
    /// `maximum` (défaut) ou `exact` (NX-27).
    #[serde(default)]
    pub effectif_mode: Option<String>,
    #[serde(default)]
    pub places: Option<i32>,
    #[serde(default)]
    pub effectif_eleves_declare: Option<i32>,
    #[serde(default)]
    pub effectif_adultes_declare: Option<i32>,
    #[serde(default)]
    pub besoin_car: bool,
    #[serde(default)]
    pub car_sur_place: bool,
    #[serde(default)]
    pub point_rendez_vous: Option<String>,
    #[serde(default)]
    pub consignes_transport: Option<String>,
    #[serde(default)]
    pub places_encadrant_a_pourvoir: i32,
}

/// Série hebdomadaire (NX-01) : un gabarit et une règle de jours ; chaque séance est une sortie liée.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SerieSaisie {
    pub libelle: String,
    /// 1 = lundi … 7 = dimanche.
    pub jour_semaine: u32,
    pub heure_debut: NaiveTime,
    pub heure_fin: NaiveTime,
    pub du: NaiveDate,
    pub au: NaiveDate,
    /// Gabarit des séances (`debut` et `fin` sont ignorés : ils viennent de la règle).
    pub modele: SortieSaisie,
}

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct SortieLigne {
    pub id: Uuid,
    pub annee_id: Uuid,
    pub serie_id: Option<Uuid>,
    pub reference: String,
    pub titre: String,
    pub objectifs: Option<String>,
    pub nature: String,
    pub forme: String,
    pub destination: String,
    pub pays: Option<String>,
    pub destination_libelle: Option<String>,
    pub programme_libelle: Option<String>,
    pub debut: DateTime<Utc>,
    pub fin: DateTime<Utc>,
    pub porteur_personne_id: Uuid,
    pub etat: String,
    pub etat_retour: Option<String>,
    pub motif_decision: Option<String>,
    pub effectif_mode: String,
    pub places: Option<i32>,
    pub effectif_eleves_declare: Option<i32>,
    pub effectif_adultes_declare: Option<i32>,
    pub besoin_car: bool,
    pub car_sur_place: bool,
    pub point_rendez_vous: Option<String>,
    pub consignes_transport: Option<String>,
    pub places_encadrant_a_pourvoir: i32,
    pub contournement_devis_motif: Option<String>,
    pub liste_confirmee_le: Option<DateTime<Utc>>,
    #[serde(skip)]
    pub liste_confirmee_par_user_id: Option<Uuid>,
    pub version: i32,
    pub cree_le: DateTime<Utc>,
    #[serde(skip)]
    pub cree_par_user_id: Option<Uuid>,
    pub modifie_le: DateTime<Utc>,
}

impl SortieLigne {
    fn contexte(&self) -> Contexte<'_> {
        Contexte {
            nature: &self.nature,
            forme: &self.forme,
            besoin_car: self.besoin_car,
        }
    }
}

/// Droits de l'appelant sur une sortie, calculés par le serveur (les boutons de l'interface les
/// reprennent, mais le serveur les revérifie à chaque commande).
#[derive(Debug, Clone, Default, Serialize, PartialEq, Eq)]
pub struct AccesVue {
    pub porteur: bool,
    pub encadrant: bool,
    pub detail: bool,
    pub modifier: bool,
    pub participants: bool,
    pub encadrement: bool,
    pub valider: bool,
    pub autoriser: bool,
    pub budget_valider: bool,
    pub prestataires: bool,
}

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct SortieResume {
    pub id: Uuid,
    pub reference: String,
    pub titre: String,
    pub debut: DateTime<Utc>,
    pub fin: DateTime<Utc>,
    pub destination: String,
    pub destination_libelle: Option<String>,
    pub nature: String,
    pub forme: String,
    pub etat: String,
    pub serie_id: Option<Uuid>,
    /// Porteur et effectif : seulement pour qui a le détail (sinon `None`).
    pub porteur_nom: Option<String>,
    pub nb_inscrits: Option<i64>,
    pub acces_detail: bool,
    pub est_porteur: bool,
}

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct PublicVue {
    pub id: Uuid,
    pub classe_id: Option<Uuid>,
    pub groupe_id: Option<Uuid>,
    pub libelle: String,
}

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct EtapeVue {
    pub id: Uuid,
    pub ordre: i16,
    #[sqlx(rename = "type")]
    #[serde(rename = "type")]
    pub type_etape: String,
    pub debut: DateTime<Utc>,
    pub fin: Option<DateTime<Utc>>,
    pub duree_minutes: Option<i32>,
    pub lieu: Option<String>,
    pub note: Option<String>,
    pub appel_requis: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EtapeSaisie {
    #[serde(rename = "type")]
    pub type_etape: String,
    pub debut: DateTime<Utc>,
    #[serde(default)]
    pub fin: Option<DateTime<Utc>>,
    #[serde(default)]
    pub duree_minutes: Option<i32>,
    #[serde(default)]
    pub lieu: Option<String>,
    #[serde(default)]
    pub note: Option<String>,
    #[serde(default)]
    pub appel_requis: bool,
}

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct EncadrantVue {
    pub id: Uuid,
    /// Agent du personnel (socle 6) : identifiant de l'annuaire du personnel.
    pub agent_id: Uuid,
    pub nom: String,
    pub prenom: Option<String>,
    pub role: String,
}

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct ParticipantVue {
    pub id: Uuid,
    pub eleve_id: Uuid,
    pub nom: String,
    pub prenom: String,
    pub classe: Option<String>,
    pub etat: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SortieDetail {
    pub sortie: SortieLigne,
    pub porteur_nom: String,
    pub publics: Vec<PublicVue>,
    pub etapes: Vec<EtapeVue>,
    pub encadrants: Vec<EncadrantVue>,
    pub nb_inscrits: i64,
    pub nb_desistes: i64,
    pub prochaine_etape: ProchaineEtape,
    pub audit: Vec<PointAudit>,
    pub acces: AccesVue,
    /// Commandes d'état proposées **à cet appelant** dans l'état actuel (calcul serveur : l'écran
    /// n'a aucune règle). Le serveur revérifie à l'exécution ; les préconditions (liste, encadrant…)
    /// sont vérifiées alors et expliquées par un refus.
    pub commandes: Vec<CommandeDispo>,
}

/// Une commande d'état proposée à l'appelant.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct CommandeDispo {
    pub code: &'static str,
    pub libelle: &'static str,
    pub motif_requis: bool,
    pub danger: bool,
    /// Étapes possibles pour une réouverture.
    pub vers: Vec<&'static str>,
}

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct JournalVue {
    pub a: DateTime<Utc>,
    pub action: String,
    pub etat_avant: Option<String>,
    pub etat_apres: Option<String>,
    pub champ: Option<String>,
    pub avant: Option<String>,
    pub apres: Option<String>,
    pub motif: Option<String>,
    pub acteur_nom: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct FiltreVoyages {
    #[serde(default)]
    pub q: Option<String>,
    #[serde(default)]
    pub etat: Option<String>,
    /// Seulement les sorties que je porte ou que j'encadre.
    #[serde(default)]
    pub mes: bool,
    /// Seulement celles qui ne sont pas passées.
    #[serde(default)]
    pub a_venir: bool,
}

/// Paramètres d'une commande d'état.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct ParamsCommande {
    /// Version lue par l'appelant ; refusée si la sortie a changé depuis (verrou optimiste).
    #[serde(default)]
    pub version: Option<i32>,
    #[serde(default)]
    pub motif: Option<String>,
    /// Étape visée (réouverture).
    #[serde(default)]
    pub vers: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ResultatSerie {
    pub serie_id: Uuid,
    pub sorties: Vec<SortieResume>,
    pub ecartees: Vec<DateEcartee>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DateEcartee {
    pub jour: NaiveDate,
    pub motif: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ResultatSeance {
    pub sortie_id: Uuid,
    pub titre: String,
    pub debut: DateTime<Utc>,
    pub ok: bool,
    pub raison: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ResultatAjout {
    pub ajoutes: Vec<Uuid>,
    pub deja: Vec<Uuid>,
    /// Élèves refusés : absents de l'établissement ou non scolarisés à la date de la sortie.
    pub refuses: Vec<Uuid>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CandidatsClasse {
    pub classe_id: Option<Uuid>,
    pub groupe_id: Option<Uuid>,
    pub libelle: String,
    pub eleves: Vec<Candidat>,
}

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct Candidat {
    pub eleve_id: Uuid,
    pub nom: String,
    pub prenom: String,
    pub deja_inscrit: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct TableauDirection {
    pub a_valider: i64,
    pub budget_a_valider: i64,
    pub a_autoriser: i64,
    pub en_preparation: i64,
    pub ouvertes: i64,
    pub pretes: i64,
    pub a_venir: i64,
    pub validees_de_l_annee: i64,
    pub a_completer: i64,
    pub bloquees_proches: Vec<SortieResume>,
}

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct VoyageFamille {
    pub sortie_id: Uuid,
    pub titre: String,
    pub debut: DateTime<Utc>,
    pub fin: DateTime<Utc>,
    pub destination: String,
    pub destination_libelle: Option<String>,
    pub point_rendez_vous: Option<String>,
    pub etat: String,
    pub enfant_prenom: String,
    pub etat_participant: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct Circulaire {
    pub titre: String,
    pub reference: String,
    pub corps: String,
}

/// Période pendant laquelle un élève est « en sortie » (port lu par l'appel, la vie scolaire,
/// l'internat et la cantine : NX-35, NX-82 ; R-SV24).
#[derive(Debug, Clone, FromRow, Serialize)]
pub struct EnSortie {
    pub eleve_id: Uuid,
    pub sortie_id: Uuid,
    pub debut: DateTime<Utc>,
    pub fin: DateTime<Utc>,
}

// ───────────────────────── Droits ─────────────────────────

struct Acces {
    porteur: bool,
    encadrant: bool,
    d: HashSet<Action>,
}

impl Acces {
    fn a(&self, x: Action) -> bool {
        self.d.contains(&x)
    }

    fn detail(&self) -> bool {
        self.porteur || self.encadrant || self.a(Action::VoyagesLire)
    }

    /// Droit « de gestion » : le porteur, ou le droit d'établissement donné.
    fn gere(&self, x: Action) -> bool {
        self.porteur || self.a(x)
    }

    fn vue(&self) -> AccesVue {
        AccesVue {
            porteur: self.porteur,
            encadrant: self.encadrant,
            detail: self.detail(),
            modifier: self.gere(Action::VoyagesModifier),
            participants: self.gere(Action::VoyagesParticipants),
            encadrement: self.gere(Action::VoyagesEncadrement),
            valider: self.a(Action::VoyagesValider),
            autoriser: self.a(Action::VoyagesAutoriser),
            budget_valider: self.a(Action::VoyagesBudgetValider),
            prestataires: self.gere(Action::VoyagesPrestataires),
        }
    }
}

/// Commandes proposées à l'appelant (état, droits du porteur et d'établissement). Miroir des gardes de
/// [`PgEnt::voyage_commande`] : un test vérifie qu'aucune commande proposée n'est refusée pour le
/// droit ou pour l'étape.
fn commandes_disponibles(s: &SortieLigne, acces: &Acces) -> Vec<CommandeDispo> {
    let etat = s.etat.as_str();
    let mut v = Vec::new();
    let mut ajoute =
        |code: &'static str, libelle: &'static str, motif_requis: bool, danger: bool| {
            v.push(CommandeDispo {
                code,
                libelle,
                motif_requis,
                danger,
                vers: Vec::new(),
            });
        };
    let gere = acces.gere(Action::VoyagesModifier);
    let valide = acces.a(Action::VoyagesValider);
    if etat == "brouillon" && gere {
        ajoute("soumettre", "Soumettre à la direction", false, false);
    }
    if etat == "soumise" && valide {
        ajoute("valider-pedagogie", "Valider la pédagogie", false, false);
    }
    if etat == "a_completer" && gere {
        ajoute("renvoyer", "Renvoyer après modification", false, false);
    }
    if etat == "a_completer" && (valide || acces.a(Action::VoyagesBudgetValider)) {
        ajoute(
            "annuler-demande",
            "Annuler la demande de modifications",
            false,
            false,
        );
    }
    if etat == "preparation" && s.besoin_car && acces.gere(Action::VoyagesPrestataires) {
        ajoute(
            "demander-signature",
            "Présenter le devis à la signature",
            false,
            false,
        );
    }
    if etat == "preparation" && !s.besoin_car && gere {
        ajoute("passer-finances", "Passer à l'étape suivante", false, false);
    }
    if etat == "a_signer" && acces.a(Action::VoyagesAutoriser) {
        ajoute(
            "passer-finances",
            "Passer aux finances sans devis signé",
            true,
            false,
        );
    }
    if etat == "budget_a_valider" && acces.a(Action::VoyagesBudgetValider) {
        ajoute("valider-budget", "Valider le budget", false, false);
    }
    if etat == "a_autoriser" && acces.a(Action::VoyagesAutoriser) {
        ajoute("autoriser", "Autoriser la sortie", false, false);
    }
    if matches!(etat, "a_autoriser" | "ouverte")
        && s.liste_confirmee_le.is_none()
        && acces.gere(Action::VoyagesParticipants)
    {
        ajoute(
            "confirmer-liste",
            "Confirmer la liste des élèves",
            false,
            false,
        );
    }
    let demande = if etat == "budget_a_valider" {
        valide || acces.a(Action::VoyagesBudgetValider)
    } else {
        valide
    };
    if etats_de_depart(Commande::DemanderModifications).contains(&etat) && demande {
        ajoute(
            "demander-modifications",
            "Demander des modifications",
            true,
            false,
        );
    }
    if etats_de_depart(Commande::Refuser).contains(&etat) && valide {
        ajoute("refuser", "Refuser le projet", true, true);
    }
    if matches!(etat, "ouverte" | "prete") && valide {
        v.push(CommandeDispo {
            code: "rouvrir",
            libelle: "Rouvrir vers une étape antérieure",
            motif_requis: true,
            danger: false,
            vers: ETAPES_AMONT.to_vec(),
        });
    }
    if etats_de_depart(Commande::Requalifier).contains(&etat) && valide && !s.besoin_car {
        v.push(CommandeDispo {
            code: "requalifier",
            libelle: "Requalifier en sortie avec car",
            motif_requis: false,
            danger: false,
            vers: Vec::new(),
        });
    }
    let avant_ouverture = !matches!(etat, "ouverte" | "prete" | "en_cours");
    if etats_de_depart(Commande::Annuler).contains(&etat) && (valide || (avant_ouverture && gere)) {
        v.push(CommandeDispo {
            code: "annuler",
            libelle: "Annuler la sortie",
            motif_requis: true,
            danger: true,
            vers: Vec::new(),
        });
    }
    v
}

impl PgEnt {
    async fn voyages_droits(&self, a: &Acteur<'_>) -> Result<HashSet<Action>, EntDbError> {
        let droits = self
            .droits_effectifs(a.etablissement_id, a.user_id, a.role)
            .await?;
        Ok(droits
            .into_iter()
            .filter(|d| d.portee == Portee::Etablissement)
            .map(|d| d.action)
            .collect())
    }

    async fn voyages_exiger(&self, a: &Acteur<'_>, x: Action) -> Result<(), EntDbError> {
        if self.voyages_droits(a).await?.contains(&x) {
            Ok(())
        } else {
            Err(interdit())
        }
    }
}

async fn personne_de(conn: &mut PgConnection, a: &Acteur<'_>) -> Result<Option<Uuid>, EntDbError> {
    sqlx::query_scalar(
        "SELECT id FROM ent_personne WHERE etablissement_id = $1 AND user_id = $2 AND fusionnee_dans IS NULL",
    )
    .bind(a.etablissement_id)
    .bind(a.user_id)
    .fetch_optional(conn)
    .await
    .map_err(db)
}

/// Fiche de personne d'un agent **actif** du personnel (socle 6) de l'établissement.
async fn personne_de_agent(
    conn: &mut PgConnection,
    a: &Acteur<'_>,
    agent_id: Uuid,
) -> Result<Option<Uuid>, EntDbError> {
    sqlx::query_scalar(
        "SELECT personne_id FROM ent_agent
          WHERE etablissement_id = $1 AND id = $2 AND fusionne_dans_agent_id IS NULL",
    )
    .bind(a.etablissement_id)
    .bind(agent_id)
    .fetch_optional(conn)
    .await
    .map_err(db)
}

async fn acces_sortie(
    conn: &mut PgConnection,
    a: &Acteur<'_>,
    d: HashSet<Action>,
    s: &SortieLigne,
) -> Result<Acces, EntDbError> {
    let moi = personne_de(conn, a).await?;
    let porteur = moi == Some(s.porteur_personne_id);
    let encadrant = match moi {
        Some(p) => sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS (SELECT 1 FROM ent_sortie_encadrant
                             WHERE etablissement_id = $1 AND sortie_id = $2 AND personne_id = $3)",
        )
        .bind(a.etablissement_id)
        .bind(s.id)
        .bind(p)
        .fetch_one(&mut *conn)
        .await
        .map_err(db)?,
        None => false,
    };
    Ok(Acces {
        porteur,
        encadrant,
        d,
    })
}

async fn charger(
    conn: &mut PgConnection,
    a: &Acteur<'_>,
    id: Uuid,
    verrou: bool,
) -> Result<SortieLigne, EntDbError> {
    let sql = if verrou {
        "SELECT * FROM ent_sortie WHERE id = $1 AND etablissement_id = $2 FOR UPDATE"
    } else {
        "SELECT * FROM ent_sortie WHERE id = $1 AND etablissement_id = $2"
    };
    sqlx::query_as::<_, SortieLigne>(sql)
        .bind(id)
        .bind(a.etablissement_id)
        .fetch_optional(conn)
        .await
        .map_err(db)?
        .ok_or_else(introuvable)
}

/// Un autre établissement, ou une sortie dont on n'a pas le détail : « introuvable » (jamais de
/// fuite d'existence, §21.4).
async fn charger_avec_acces(
    e: &PgEnt,
    conn: &mut PgConnection,
    a: &Acteur<'_>,
    id: Uuid,
    verrou: bool,
) -> Result<(SortieLigne, Acces), EntDbError> {
    let s = charger(conn, a, id, verrou).await?;
    let d = e.voyages_droits(a).await?;
    let acces = acces_sortie(conn, a, d, &s).await?;
    if !acces.detail() {
        return Err(introuvable());
    }
    Ok((s, acces))
}

// ───────────────────────── Journal et événements ─────────────────────────

struct Fait<'a> {
    action: &'a str,
    etat_avant: Option<&'a str>,
    etat_apres: Option<&'a str>,
    champ: Option<&'a str>,
    avant: Option<String>,
    apres: Option<String>,
    motif: Option<&'a str>,
}

impl<'a> Fait<'a> {
    fn simple(action: &'a str) -> Self {
        Self {
            action,
            etat_avant: None,
            etat_apres: None,
            champ: None,
            avant: None,
            apres: None,
            motif: None,
        }
    }
}

fn borne(s: Option<String>) -> Option<String> {
    s.map(|t| t.chars().take(2000).collect())
}

async fn journaliser(
    conn: &mut PgConnection,
    a: &Acteur<'_>,
    sortie: Uuid,
    f: Fait<'_>,
) -> Result<(), EntDbError> {
    sqlx::query(
        "INSERT INTO ent_sortie_journal
             (id, etablissement_id, sortie_id, acteur_user_id, action, etat_avant, etat_apres, champ, avant, apres, motif)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11)",
    )
    .bind(Uuid::new_v4())
    .bind(a.etablissement_id)
    .bind(sortie)
    .bind(a.user_id)
    .bind(f.action)
    .bind(f.etat_avant)
    .bind(f.etat_apres)
    .bind(f.champ)
    .bind(borne(f.avant))
    .bind(borne(f.apres))
    .bind(f.motif.map(|m| m.chars().take(2000).collect::<String>()))
    .execute(conn)
    .await
    .map_err(db)?;
    Ok(())
}

/// Publie un fait `voyages.*` dans la transaction du fait. Aucune identité d'élève.
async fn publier(
    conn: &mut PgConnection,
    a: &Acteur<'_>,
    type_name: &str,
    sortie: Uuid,
    payload: serde_json::Value,
) -> Result<MetierEvent, EntDbError> {
    Ok(conn
        .publier(MetierEventDraft {
            etablissement_id: a.etablissement_id,
            type_name: type_name.to_string(),
            aggregate: AggregateKind::Voyage,
            aggregate_id: sortie,
            eleve_id: None,
            payload,
            actor_user_id: Some(a.user_id.to_string()),
        })
        .await?)
}

// ───────────────────────── Dossier d'audit ─────────────────────────

struct DossierMesures {
    nb_publics: i64,
    nb_inscrits: i64,
    nb_enc: i64,
    nb_resp: i64,
    depot: i64,
    reprise: i64,
}

async fn dossier(conn: &mut PgConnection, s: &SortieLigne) -> Result<DossierMesures, EntDbError> {
    let (nb_publics, nb_inscrits, nb_enc, nb_resp, depot, reprise): (i64, i64, i64, i64, i64, i64) =
        sqlx::query_as(
            "SELECT (SELECT count(*) FROM ent_sortie_public WHERE sortie_id = $1),
                    (SELECT count(*) FROM ent_sortie_participant WHERE sortie_id = $1 AND etat IN ('inscrit','pret','bloque')),
                    (SELECT count(*) FROM ent_sortie_encadrant WHERE sortie_id = $1 AND role IN ('responsable','encadrant')),
                    (SELECT count(*) FROM ent_sortie_encadrant WHERE sortie_id = $1 AND role = 'responsable'),
                    (SELECT count(*) FROM ent_sortie_etape WHERE sortie_id = $1 AND type = 'depot'),
                    (SELECT count(*) FROM ent_sortie_etape WHERE sortie_id = $1 AND type = 'reprise')",
        )
        .bind(s.id)
        .fetch_one(conn)
        .await
        .map_err(db)?;
    Ok(DossierMesures {
        nb_publics,
        nb_inscrits,
        nb_enc,
        nb_resp,
        depot,
        reprise,
    })
}

impl DossierMesures {
    fn etat<'a>(&self, s: &'a SortieLigne) -> EtatDossier<'a> {
        EtatDossier {
            etat: &s.etat,
            nature: &s.nature,
            forme: &s.forme,
            destination: &s.destination,
            pays: s.pays.as_deref(),
            besoin_car: s.besoin_car,
            effectif_mode: &s.effectif_mode,
            effectif_eleves_declare: s.effectif_eleves_declare,
            effectif_adultes_declare: s.effectif_adultes_declare,
            places_encadrant_a_pourvoir: s.places_encadrant_a_pourvoir,
            nb_publics: self.nb_publics,
            nb_inscrits: self.nb_inscrits,
            nb_encadrants_comptes: self.nb_enc,
            nb_responsables: self.nb_resp,
            nb_etapes_depot: self.depot,
            nb_etapes_reprise: self.reprise,
            liste_confirmee: s.liste_confirmee_le.is_some(),
            contournement_devis: s.contournement_devis_motif.is_some(),
        }
    }
}

fn detail_blocages(b: &[PointAudit]) -> EntDbError {
    EntDbError::ConflitDossierDetail(
        b.iter()
            .map(|p| p.libelle.as_str())
            .collect::<Vec<_>>()
            .join(" ; "),
    )
}

// ───────────────────────── Validation de la saisie ─────────────────────────

struct SaisieValide {
    titre: String,
    objectifs: Option<String>,
    pays: Option<String>,
    destination_libelle: Option<String>,
    programme_libelle: Option<String>,
    effectif_mode: String,
    point_rendez_vous: Option<String>,
    consignes_transport: Option<String>,
}

fn valider_saisie(s: &SortieSaisie) -> Result<SaisieValide, EntDbError> {
    if !NATURES.contains(&s.nature.as_str())
        || !FORMES.contains(&s.forme.as_str())
        || !DESTINATIONS.contains(&s.destination.as_str())
    {
        return Err(invalide());
    }
    if s.fin <= s.debut {
        return Err(refus(RefusMetier::Invalide, "La fin doit suivre le début."));
    }
    // Un voyage avec nuitées dure plus d'un jour ; une sortie sans nuitée tient en 36 h (§8.2).
    if s.forme == "sans_nuitee" && (s.fin - s.debut) > chrono::Duration::hours(36) {
        return Err(refus(
            RefusMetier::Invalide,
            "Une sortie sans nuitée ne dépasse pas une journée : choisissez « voyage ».",
        ));
    }
    let effectif_mode = match s.effectif_mode.as_deref() {
        None | Some("maximum") => "maximum".to_string(),
        Some("exact") => "exact".to_string(),
        _ => return Err(invalide()),
    };
    for n in [
        s.places,
        s.effectif_eleves_declare,
        s.effectif_adultes_declare,
    ]
    .into_iter()
    .flatten()
    {
        if !(0..=2000).contains(&n) {
            return Err(invalide());
        }
    }
    if s.places == Some(0) || !(0..=200).contains(&s.places_encadrant_a_pourvoir) {
        return Err(invalide());
    }
    let pays = texte_opt(s.pays.as_deref(), 80)?;
    if s.destination == "etranger" && pays.as_deref().map(str::len).unwrap_or(0) < 2 {
        return Err(refus(
            RefusMetier::Invalide,
            "Le pays de destination est obligatoire à l'étranger.",
        ));
    }
    Ok(SaisieValide {
        titre: texte(&s.titre, 200)?,
        objectifs: texte_opt(s.objectifs.as_deref(), 4000)?,
        pays,
        destination_libelle: texte_opt(s.destination_libelle.as_deref(), 300)?,
        programme_libelle: texte_opt(s.programme_libelle.as_deref(), 4000)?,
        effectif_mode,
        point_rendez_vous: texte_opt(s.point_rendez_vous.as_deref(), 300)?,
        consignes_transport: texte_opt(s.consignes_transport.as_deref(), 2000)?,
    })
}

/// Année scolaire qui contient la date de début (R-SV29 : les dates sont dans l'année du socle 5).
async fn annee_de(
    conn: &mut PgConnection,
    a: &Acteur<'_>,
    debut: DateTime<Utc>,
) -> Result<Uuid, EntDbError> {
    sqlx::query_scalar(
        "SELECT id FROM ent_annee_scolaire
          WHERE etablissement_id = $1
            AND ($2::timestamptz AT TIME ZONE 'Europe/Paris')::date BETWEEN date_debut AND date_fin",
    )
    .bind(a.etablissement_id)
    .bind(debut)
    .fetch_optional(conn)
    .await
    .map_err(db)?
    .ok_or_else(|| {
        refus(
            RefusMetier::Invalide,
            "La date de début ne tombe dans aucune année scolaire de l'établissement.",
        )
    })
}

async fn verifier_publics(
    conn: &mut PgConnection,
    a: &Acteur<'_>,
    classes: &[Uuid],
    groupes: &[Uuid],
) -> Result<(), EntDbError> {
    let uniques: HashSet<&Uuid> = classes.iter().collect();
    let g: HashSet<&Uuid> = groupes.iter().collect();
    if uniques.len() != classes.len()
        || g.len() != groupes.len()
        || classes.len() + groupes.len() > 60
    {
        return Err(invalide());
    }
    let n: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM ent_classe WHERE etablissement_id = $1 AND id = ANY($2)",
    )
    .bind(a.etablissement_id)
    .bind(classes)
    .fetch_one(&mut *conn)
    .await
    .map_err(db)?;
    let m: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM ent_groupe WHERE etablissement_id = $1 AND id = ANY($2)",
    )
    .bind(a.etablissement_id)
    .bind(groupes)
    .fetch_one(&mut *conn)
    .await
    .map_err(db)?;
    if n as usize != classes.len() || m as usize != groupes.len() {
        return Err(introuvable());
    }
    Ok(())
}

async fn poser_publics(
    conn: &mut PgConnection,
    a: &Acteur<'_>,
    sortie: Uuid,
    classes: &[Uuid],
    groupes: &[Uuid],
) -> Result<(), EntDbError> {
    sqlx::query("DELETE FROM ent_sortie_public WHERE etablissement_id = $1 AND sortie_id = $2")
        .bind(a.etablissement_id)
        .bind(sortie)
        .execute(&mut *conn)
        .await
        .map_err(db)?;
    for c in classes {
        sqlx::query(
            "INSERT INTO ent_sortie_public (id, etablissement_id, sortie_id, classe_id) VALUES ($1,$2,$3,$4)",
        )
        .bind(Uuid::new_v4())
        .bind(a.etablissement_id)
        .bind(sortie)
        .bind(c)
        .execute(&mut *conn)
        .await
        .map_err(db)?;
    }
    for g in groupes {
        sqlx::query(
            "INSERT INTO ent_sortie_public (id, etablissement_id, sortie_id, groupe_id) VALUES ($1,$2,$3,$4)",
        )
        .bind(Uuid::new_v4())
        .bind(a.etablissement_id)
        .bind(sortie)
        .bind(g)
        .execute(&mut *conn)
        .await
        .map_err(db)?;
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn inserer_sortie(
    conn: &mut PgConnection,
    a: &Acteur<'_>,
    porteur: Uuid,
    annee: Uuid,
    serie: Option<Uuid>,
    debut: DateTime<Utc>,
    fin: DateTime<Utc>,
    s: &SortieSaisie,
    v: &SaisieValide,
) -> Result<SortieLigne, EntDbError> {
    let id = Uuid::new_v4();
    sqlx::query_as::<_, SortieLigne>(
        "INSERT INTO ent_sortie
            (id, etablissement_id, annee_id, serie_id, reference, titre, objectifs, nature, forme, destination,
             pays, destination_libelle, programme_libelle, debut, fin, porteur_personne_id, effectif_mode, places,
             effectif_eleves_declare, effectif_adultes_declare, besoin_car, car_sur_place, point_rendez_vous,
             consignes_transport, places_encadrant_a_pourvoir, cree_par_user_id)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19,$20,$21,$22,$23,$24,$25,$26)
         RETURNING *",
    )
    .bind(id)
    .bind(a.etablissement_id)
    .bind(annee)
    .bind(serie)
    .bind(reference(debut, id))
    .bind(&v.titre)
    .bind(&v.objectifs)
    .bind(&s.nature)
    .bind(&s.forme)
    .bind(&s.destination)
    .bind(&v.pays)
    .bind(&v.destination_libelle)
    .bind(&v.programme_libelle)
    .bind(debut)
    .bind(fin)
    .bind(porteur)
    .bind(&v.effectif_mode)
    .bind(s.places)
    .bind(s.effectif_eleves_declare)
    .bind(s.effectif_adultes_declare)
    .bind(s.besoin_car)
    .bind(s.car_sur_place)
    .bind(&v.point_rendez_vous)
    .bind(&v.consignes_transport)
    .bind(s.places_encadrant_a_pourvoir)
    .bind(a.user_id)
    .fetch_one(conn)
    .await
    .map_err(db)
}

fn o(v: &Option<String>) -> String {
    v.clone().unwrap_or_default()
}

fn n(v: Option<i32>) -> String {
    v.map(|x| x.to_string()).unwrap_or_default()
}

/// Jours à écarter d'une série (fériés, vacances, fermetures de l'établissement) : lus dans la
/// transaction du fait, avec le calendrier du socle 5 (`ent_jour_calendrier`, `ent_periode_vacances`).
async fn jours_neutralises(
    conn: &mut PgConnection,
    a: &Acteur<'_>,
    du: NaiveDate,
    au: NaiveDate,
) -> Result<std::collections::HashMap<NaiveDate, String>, EntDbError> {
    let mut out = std::collections::HashMap::new();
    let jours: Vec<(NaiveDate, String)> = sqlx::query_as(
        "SELECT jour, kind FROM ent_jour_calendrier
          WHERE etablissement_id = $1 AND jour BETWEEN $2 AND $3 AND kind IN ('ferme','ferie','vacances')",
    )
    .bind(a.etablissement_id)
    .bind(du)
    .bind(au)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    out.extend(jours);
    let periodes: Vec<(String, NaiveDate, NaiveDate)> = sqlx::query_as(
        "SELECT kind, debut, fin FROM ent_periode_vacances
          WHERE etablissement_id = $1 AND debut <= $3 AND fin >= $2",
    )
    .bind(a.etablissement_id)
    .bind(du)
    .bind(au)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    for (kind, debut, fin) in periodes {
        let code = match kind.as_str() {
            "ferie" => "ferie",
            "fermeture" => "ferme",
            _ => "vacances",
        };
        let mut j = debut.max(du);
        while j <= fin.min(au) {
            out.entry(j).or_insert_with(|| code.to_string());
            j += chrono::Duration::days(1);
        }
    }
    Ok(out)
}

// ───────────────────────── Commandes : projet ─────────────────────────

impl PgEnt {
    /// VOY-100, VOY-101 (NX-01, NX-02) : crée une sortie ou un voyage en `brouillon`. Le créateur
    /// en devient le porteur (VD-19 : une personne, pas un texte).
    pub async fn voyage_creer(
        &self,
        a: &Acteur<'_>,
        saisie: SortieSaisie,
    ) -> Result<SortieLigne, EntDbError> {
        self.voyages_exiger(a, Action::VoyagesCreer).await?;
        let v = valider_saisie(&saisie)?;
        let mut t = tx(self, a).await?;
        let annee = annee_de(&mut t, a, saisie.debut).await?;
        verifier_publics(&mut t, a, &saisie.classes, &saisie.groupes).await?;
        let porteur = personne_de(&mut t, a).await?.ok_or_else(|| {
            refus(
                RefusMetier::Interdit,
                "Ce compte n'est rattaché à aucune fiche de personnel : il ne peut pas porter une sortie.",
            )
        })?;
        let s = inserer_sortie(
            &mut t,
            a,
            porteur,
            annee,
            None,
            saisie.debut,
            saisie.fin,
            &saisie,
            &v,
        )
        .await?;
        poser_publics(&mut t, a, s.id, &saisie.classes, &saisie.groupes).await?;
        journaliser(
            &mut t,
            a,
            s.id,
            Fait {
                etat_apres: Some("brouillon"),
                ..Fait::simple("creee")
            },
        )
        .await?;
        let ev = publier(
            &mut t,
            a,
            "voyages.creee",
            s.id,
            json!({ "etat": "brouillon", "forme": s.forme }),
        )
        .await?;
        t.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&[ev]);
        Ok(s)
    }

    /// VOY-104 (NX-01) : série hebdomadaire. Les jours fériés, vacances et fermetures du calendrier
    /// de l'établissement (socle 5) sont **écartés et listés** ; chaque séance est une sortie liée.
    pub async fn voyage_serie_creer(
        &self,
        a: &Acteur<'_>,
        serie: SerieSaisie,
    ) -> Result<ResultatSerie, EntDbError> {
        self.voyages_exiger(a, Action::VoyagesCreer).await?;
        if !(1..=7).contains(&serie.jour_semaine)
            || serie.heure_fin <= serie.heure_debut
            || serie.au < serie.du
        {
            return Err(invalide());
        }
        if (serie.au - serie.du).num_days() > 400 {
            return Err(refus(
                RefusMetier::Invalide,
                "Une série ne dépasse pas une année.",
            ));
        }
        let libelle = texte(&serie.libelle, 200)?;
        // Contrôle du gabarit avec des dates fictives valides.
        let mut gabarit = serie.modele.clone();
        gabarit.debut = serie.du.and_time(serie.heure_debut).and_utc();
        gabarit.fin = serie.du.and_time(serie.heure_fin).and_utc();
        gabarit.forme = "sans_nuitee".into();
        let v = valider_saisie(&gabarit)?;

        let mut t = tx(self, a).await?;
        let exclus = jours_neutralises(&mut t, a, serie.du, serie.au).await?;
        let jours: HashSet<NaiveDate> = exclus.keys().copied().collect();
        let (retenues, ecartees) = dates_de_serie(serie.jour_semaine, serie.du, serie.au, &jours);
        if retenues.is_empty() {
            return Err(refus(
                RefusMetier::Invalide,
                "Aucune séance : toutes les dates tombent un jour férié, en vacances ou un jour fermé.",
            ));
        }

        let porteur = personne_de(&mut t, a).await?.ok_or_else(|| {
            refus(
                RefusMetier::Interdit,
                "Ce compte n'est rattaché à aucune fiche de personnel.",
            )
        })?;
        verifier_publics(&mut t, a, &serie.modele.classes, &serie.modele.groupes).await?;
        let serie_id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_sortie_serie (id, etablissement_id, libelle, jour_semaine, heure_debut, heure_fin, du, au, cree_par_user_id)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)",
        )
        .bind(serie_id)
        .bind(a.etablissement_id)
        .bind(&libelle)
        .bind(serie.jour_semaine as i16)
        .bind(serie.heure_debut)
        .bind(serie.heure_fin)
        .bind(serie.du)
        .bind(serie.au)
        .bind(a.user_id)
        .execute(&mut *t)
        .await
        .map_err(db)?;
        for j in &ecartees {
            sqlx::query(
                "INSERT INTO ent_sortie_serie_exclusion (etablissement_id, serie_id, jour, motif_code) VALUES ($1,$2,$3,$4)",
            )
            .bind(a.etablissement_id)
            .bind(serie_id)
            .bind(j)
            .bind(exclus.get(j).cloned().unwrap_or_else(|| "manuel".into()))
            .execute(&mut *t)
            .await
            .map_err(db)?;
        }
        let mut sorties = Vec::new();
        for jour in &retenues {
            let (debut, fin): (DateTime<Utc>, DateTime<Utc>) = sqlx::query_as(
                "SELECT (($1::date + $2::time) AT TIME ZONE 'Europe/Paris'),
                        (($1::date + $3::time) AT TIME ZONE 'Europe/Paris')",
            )
            .bind(jour)
            .bind(serie.heure_debut)
            .bind(serie.heure_fin)
            .fetch_one(&mut *t)
            .await
            .map_err(db)?;
            let annee = annee_de(&mut t, a, debut).await?;
            let mut m = serie.modele.clone();
            m.debut = debut;
            m.fin = fin;
            m.forme = "sans_nuitee".into();
            let s = inserer_sortie(
                &mut t,
                a,
                porteur,
                annee,
                Some(serie_id),
                debut,
                fin,
                &m,
                &v,
            )
            .await?;
            poser_publics(&mut t, a, s.id, &m.classes, &m.groupes).await?;
            journaliser(
                &mut t,
                a,
                s.id,
                Fait {
                    etat_apres: Some("brouillon"),
                    motif: Some("séance de série"),
                    ..Fait::simple("creee")
                },
            )
            .await?;
            sorties.push(s);
        }
        let ev = publier(
            &mut t,
            a,
            "voyages.serie_creee",
            serie_id,
            json!({ "seances": retenues.len() }),
        )
        .await?;
        t.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&[ev]);
        Ok(ResultatSerie {
            serie_id,
            sorties: sorties
                .into_iter()
                .map(|s| SortieResume {
                    id: s.id,
                    reference: s.reference,
                    titre: s.titre,
                    debut: s.debut,
                    fin: s.fin,
                    destination: s.destination,
                    destination_libelle: s.destination_libelle,
                    nature: s.nature,
                    forme: s.forme,
                    etat: s.etat,
                    serie_id: s.serie_id,
                    porteur_nom: None,
                    nb_inscrits: None,
                    acces_detail: true,
                    est_porteur: true,
                })
                .collect(),
            ecartees: ecartees
                .into_iter()
                .map(|jour| DateEcartee {
                    motif: exclus
                        .get(&jour)
                        .cloned()
                        .unwrap_or_else(|| "manuel".into()),
                    jour,
                })
                .collect(),
        })
    }

    /// VOY-106 (NX-12, NX-81) : modifie les champs du projet. Verrou optimiste : la `version` lue doit
    /// être la version courante. Une entrée de journal par champ changé (avant → après). Interdit
    /// après le départ et sur un dossier terminé. Modifier le public ou l'effectif d'une liste confirmée
    /// la renvoie en brouillon (NX-26).
    pub async fn voyage_modifier(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
        version: i32,
        saisie: SortieSaisie,
    ) -> Result<SortieLigne, EntDbError> {
        let v = valider_saisie(&saisie)?;
        let mut t = tx(self, a).await?;
        let (s, acces) = charger_avec_acces(self, &mut t, a, id, true).await?;
        if !acces.gere(Action::VoyagesModifier) {
            return Err(interdit());
        }
        if !est_modifiable(&s.etat) {
            return Err(refus(
                RefusMetier::Conflit,
                "Ce dossier n'est plus modifiable.",
            ));
        }
        if s.version != version {
            return Err(refus(
                RefusMetier::Conflit,
                "Le dossier a été modifié entre-temps : rechargez-le avant de recommencer.",
            ));
        }
        // Une fois le dossier présenté, validé ou autorisé, la nature, la forme, la destination et le
        // besoin de car engagent des décisions (budget, devis, autorisation) : seule la direction les
        // change ; le porteur demande une réouverture.
        let engage = matches!(
            s.etat.as_str(),
            "a_signer" | "budget_a_valider" | "a_autoriser" | "ouverte" | "prete"
        );
        if engage
            && !acces.a(Action::VoyagesValider)
            && (s.nature != saisie.nature
                || s.forme != saisie.forme
                || s.destination != saisie.destination
                || s.besoin_car != saisie.besoin_car)
        {
            return Err(refus(
                RefusMetier::Conflit,
                "Ce dossier est déjà validé : demandez à la direction de le rouvrir pour changer sa nature, sa forme, sa destination ou le car.",
            ));
        }
        verifier_publics(&mut t, a, &saisie.classes, &saisie.groupes).await?;
        let annee = annee_de(&mut t, a, saisie.debut).await?;
        let avant_publics: Vec<(Option<Uuid>, Option<Uuid>)> = sqlx::query_as(
            "SELECT classe_id, groupe_id FROM ent_sortie_public WHERE sortie_id = $1 ORDER BY id",
        )
        .bind(id)
        .fetch_all(&mut *t)
        .await
        .map_err(db)?;
        let mut anciens: HashSet<Uuid> =
            avant_publics.iter().filter_map(|(c, g)| c.or(*g)).collect();
        let n_publics = saisie.classes.len() + saisie.groupes.len();
        let publics_changent = anciens.len() != n_publics
            || !saisie
                .classes
                .iter()
                .chain(saisie.groupes.iter())
                .all(|x| anciens.remove(x));

        let changes: Vec<(&str, String, String)> = vec![
            ("titre", s.titre.clone(), v.titre.clone()),
            ("objectifs", o(&s.objectifs), o(&v.objectifs)),
            ("nature", s.nature.clone(), saisie.nature.clone()),
            ("forme", s.forme.clone(), saisie.forme.clone()),
            (
                "destination",
                s.destination.clone(),
                saisie.destination.clone(),
            ),
            ("pays", o(&s.pays), o(&v.pays)),
            (
                "destination_libelle",
                o(&s.destination_libelle),
                o(&v.destination_libelle),
            ),
            (
                "programme_libelle",
                o(&s.programme_libelle),
                o(&v.programme_libelle),
            ),
            ("debut", s.debut.to_rfc3339(), saisie.debut.to_rfc3339()),
            ("fin", s.fin.to_rfc3339(), saisie.fin.to_rfc3339()),
            (
                "effectif_mode",
                s.effectif_mode.clone(),
                v.effectif_mode.clone(),
            ),
            ("places", n(s.places), n(saisie.places)),
            (
                "effectif_eleves_declare",
                n(s.effectif_eleves_declare),
                n(saisie.effectif_eleves_declare),
            ),
            (
                "effectif_adultes_declare",
                n(s.effectif_adultes_declare),
                n(saisie.effectif_adultes_declare),
            ),
            (
                "besoin_car",
                s.besoin_car.to_string(),
                saisie.besoin_car.to_string(),
            ),
            (
                "car_sur_place",
                s.car_sur_place.to_string(),
                saisie.car_sur_place.to_string(),
            ),
            (
                "point_rendez_vous",
                o(&s.point_rendez_vous),
                o(&v.point_rendez_vous),
            ),
            (
                "consignes_transport",
                o(&s.consignes_transport),
                o(&v.consignes_transport),
            ),
            (
                "places_encadrant_a_pourvoir",
                s.places_encadrant_a_pourvoir.to_string(),
                saisie.places_encadrant_a_pourvoir.to_string(),
            ),
        ];
        let reset_liste = s.liste_confirmee_le.is_some()
            && (publics_changent
                || s.effectif_eleves_declare != saisie.effectif_eleves_declare
                || s.effectif_mode != v.effectif_mode
                || s.debut != saisie.debut
                || s.fin != saisie.fin);
        let etat_apres = if reset_liste && s.etat == "prete" {
            "ouverte".to_string()
        } else {
            s.etat.clone()
        };
        let maj = sqlx::query_as::<_, SortieLigne>(
            "UPDATE ent_sortie SET annee_id=$3, titre=$4, objectifs=$5, nature=$6, forme=$7, destination=$8, pays=$9,
                    destination_libelle=$10, programme_libelle=$11, debut=$12, fin=$13, effectif_mode=$14, places=$15,
                    effectif_eleves_declare=$16, effectif_adultes_declare=$17, besoin_car=$18, car_sur_place=$19,
                    point_rendez_vous=$20, consignes_transport=$21, places_encadrant_a_pourvoir=$22, etat=$23,
                    liste_confirmee_le = CASE WHEN $24 THEN NULL ELSE liste_confirmee_le END,
                    liste_confirmee_par_user_id = CASE WHEN $24 THEN NULL ELSE liste_confirmee_par_user_id END,
                    version = version + 1, modifie_le = now()
              WHERE id = $1 AND etablissement_id = $2
              RETURNING *",
        )
        .bind(id)
        .bind(a.etablissement_id)
        .bind(annee)
        .bind(&v.titre)
        .bind(&v.objectifs)
        .bind(&saisie.nature)
        .bind(&saisie.forme)
        .bind(&saisie.destination)
        .bind(&v.pays)
        .bind(&v.destination_libelle)
        .bind(&v.programme_libelle)
        .bind(saisie.debut)
        .bind(saisie.fin)
        .bind(&v.effectif_mode)
        .bind(saisie.places)
        .bind(saisie.effectif_eleves_declare)
        .bind(saisie.effectif_adultes_declare)
        .bind(saisie.besoin_car)
        .bind(saisie.car_sur_place)
        .bind(&v.point_rendez_vous)
        .bind(&v.consignes_transport)
        .bind(saisie.places_encadrant_a_pourvoir)
        .bind(&etat_apres)
        .bind(reset_liste)
        .fetch_one(&mut *t)
        .await
        .map_err(db)?;
        poser_publics(&mut t, a, id, &saisie.classes, &saisie.groupes).await?;
        let mut modifies = 0;
        for (champ, avant, apres) in changes {
            if avant != apres {
                modifies += 1;
                journaliser(
                    &mut t,
                    a,
                    id,
                    Fait {
                        champ: Some(champ),
                        avant: Some(avant),
                        apres: Some(apres),
                        ..Fait::simple("modifiee")
                    },
                )
                .await?;
            }
        }
        if publics_changent {
            modifies += 1;
            journaliser(
                &mut t,
                a,
                id,
                Fait {
                    champ: Some("public"),
                    apres: Some(format!(
                        "{} classe(s), {} groupe(s)",
                        saisie.classes.len(),
                        saisie.groupes.len()
                    )),
                    ..Fait::simple("modifiee")
                },
            )
            .await?;
        }
        if reset_liste {
            journaliser(
                &mut t,
                a,
                id,
                Fait {
                    etat_avant: Some(&s.etat),
                    etat_apres: Some(&etat_apres),
                    motif: Some("modification touchant la liste : à reconfirmer"),
                    ..Fait::simple("liste_rouverte")
                },
            )
            .await?;
        }
        // Au-delà de l'ouverture, un changement de dates ou de lieu appelle un avis aux familles
        // (proposé, jamais envoyé seul : R-SV3). Le journal en garde la trace.
        let apres_ouverture = matches!(s.etat.as_str(), "ouverte" | "prete");
        if apres_ouverture && modifies > 0 {
            journaliser(
                &mut t,
                a,
                id,
                Fait {
                    motif: Some("avis aux familles à proposer"),
                    ..Fait::simple("modifiee_apres_ouverture")
                },
            )
            .await?;
        }
        let ev = publier(
            &mut t,
            a,
            "voyages.modifiee",
            id,
            json!({ "champs": modifies, "apres_ouverture": apres_ouverture }),
        )
        .await?;
        t.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&[ev]);
        Ok(maj)
    }
}

// ───────────────────────── Commandes : automate ─────────────────────────

impl PgEnt {
    /// Commande d'état (VOY-110 à VOY-122, VOY-127, VOY-143) : vérifie droit, état de départ,
    /// préconditions et version, puis écrit le nouvel état, le journal et l'événement dans **une**
    /// transaction.
    pub async fn voyage_commande(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
        cmd: Commande,
        p: ParamsCommande,
    ) -> Result<SortieLigne, EntDbError> {
        let motif = texte_opt(p.motif.as_deref(), 2000)?;
        let mut t = tx(self, a).await?;
        let (s, acces) = charger_avec_acces(self, &mut t, a, id, true).await?;
        if let Some(v) = p.version {
            if v != s.version {
                return Err(refus(
                    RefusMetier::Conflit,
                    "Le dossier a été modifié entre-temps : rechargez-le avant de recommencer.",
                ));
            }
        }
        if matches!(cmd, Commande::Autoriser | Commande::ValiderBudget) && p.version.is_none() {
            return Err(refus(
                RefusMetier::Invalide,
                "Une décision porte sur la version du dossier que vous avez lue : rechargez-le.",
            ));
        }
        if !etats_de_depart(cmd).contains(&s.etat.as_str()) {
            return Err(refus(
                RefusMetier::Conflit,
                "Cette action n'est pas possible à l'étape actuelle du dossier.",
            ));
        }
        let ctx = s.contexte();
        let mesures = dossier(&mut t, &s).await?;
        let mut etat = s.etat.clone();
        let mut etat_retour: Option<String> = None;
        let mut motif_decision = s.motif_decision.clone();
        let mut liste = (s.liste_confirmee_le, s.liste_confirmee_par_user_id);
        let mut contournement = s.contournement_devis_motif.clone();
        let mut besoin_car = s.besoin_car;
        let evenement: &str;
        let motif_requis = |m: &Option<String>| -> Result<String, EntDbError> {
            m.clone()
                .ok_or_else(|| refus(RefusMetier::Invalide, "Un motif est obligatoire."))
        };
        match cmd {
            Commande::Soumettre => {
                if !acces.gere(Action::VoyagesModifier) {
                    return Err(interdit());
                }
                let b = blocages_soumission(&mesures.etat(&s));
                if !b.is_empty() {
                    return Err(detail_blocages(&b));
                }
                etat = "soumise".into();
                evenement = "voyages.soumise";
            }
            Commande::ValiderPedagogie => {
                if !acces.a(Action::VoyagesValider) {
                    return Err(interdit());
                }
                etat = crate::voyages::apres_validation_pedagogique(&ctx).into();
                evenement = "voyages.validee_pedagogie";
            }
            Commande::DemanderModifications => {
                let ok = if s.etat == "budget_a_valider" {
                    acces.a(Action::VoyagesBudgetValider) || acces.a(Action::VoyagesValider)
                } else {
                    acces.a(Action::VoyagesValider)
                };
                if !ok {
                    return Err(interdit());
                }
                motif_decision = Some(motif_requis(&motif)?);
                etat_retour = Some(s.etat.clone());
                etat = "a_completer".into();
                evenement = "voyages.a_completer";
            }
            Commande::AnnulerDemande => {
                if !(acces.a(Action::VoyagesValider) || acces.a(Action::VoyagesBudgetValider)) {
                    return Err(interdit());
                }
                etat = s.etat_retour.clone().ok_or_else(conflit)?;
                motif_decision = None;
                evenement = "voyages.demande_annulee";
            }
            Commande::Renvoyer => {
                if !acces.gere(Action::VoyagesModifier) {
                    return Err(interdit());
                }
                let retour = s.etat_retour.clone().ok_or_else(conflit)?;
                if retour == "soumise" {
                    let b = blocages_soumission(&mesures.etat(&s));
                    if !b.is_empty() {
                        return Err(detail_blocages(&b));
                    }
                }
                etat = retour;
                motif_decision = None;
                evenement = "voyages.resoumise";
            }
            Commande::Refuser => {
                if !acces.a(Action::VoyagesValider) {
                    return Err(interdit());
                }
                motif_decision = Some(motif_requis(&motif)?);
                etat = "refusee".into();
                evenement = "voyages.refusee";
            }
            Commande::DemanderSignature => {
                if !acces.gere(Action::VoyagesPrestataires) {
                    return Err(interdit());
                }
                if !s.besoin_car {
                    return Err(refus(
                        RefusMetier::Conflit,
                        "Cette sortie n'a pas besoin de car : pas de devis à signer.",
                    ));
                }
                etat = "a_signer".into();
                evenement = "voyages.signature_demandee";
            }
            Commande::PasserAuxFinances => {
                if s.etat == "a_signer" {
                    // NX-07 : passage sans devis signé, par la direction, avec motif tracé.
                    // Le devis signé lui-même arrive en V0c, qui rendra ce motif inutile.
                    if !acces.a(Action::VoyagesAutoriser) {
                        return Err(interdit());
                    }
                    contournement = Some(motif_requis(&motif)?);
                } else {
                    if !acces.gere(Action::VoyagesModifier) {
                        return Err(interdit());
                    }
                    if s.besoin_car {
                        return Err(refus(
                            RefusMetier::Conflit,
                            "Un car est nécessaire : présentez d'abord le devis à la signature.",
                        ));
                    }
                }
                etat = if ctx.a_budget() {
                    "budget_a_valider"
                } else {
                    "a_autoriser"
                }
                .into();
                evenement = "voyages.passee_aux_finances";
            }
            Commande::ValiderBudget => {
                // La version de budget validée (R-SV15, R-SV17) est le lot V0b.
                if !acces.a(Action::VoyagesBudgetValider) {
                    return Err(interdit());
                }
                etat = "a_autoriser".into();
                evenement = "voyages.budget_valide";
            }
            Commande::Autoriser => {
                if !acces.a(Action::VoyagesAutoriser) {
                    return Err(interdit());
                }
                etat = crate::voyages::apres_autorisation(s.liste_confirmee_le.is_some()).into();
                evenement = "voyages.autorisee";
            }
            Commande::ConfirmerListe => {
                if !acces.gere(Action::VoyagesParticipants) {
                    return Err(interdit());
                }
                if s.liste_confirmee_le.is_some() {
                    // Idempotent : rien à faire.
                    return Ok(s);
                }
                let b = blocages_liste(&mesures.etat(&s));
                if !b.is_empty() {
                    return Err(detail_blocages(&b));
                }
                liste = (Some(Utc::now()), Some(a.user_id));
                if s.etat == "ouverte" {
                    etat = "prete".into();
                }
                evenement = "voyages.liste_confirmee";
            }
            Commande::Rouvrir => {
                if !acces.a(Action::VoyagesValider) {
                    return Err(interdit());
                }
                let vers = p.vers.clone().ok_or_else(invalide)?;
                if !ETAPES_AMONT.contains(&vers.as_str()) {
                    return Err(invalide());
                }
                motif_requis(&motif)?;
                etat = vers;
                evenement = "voyages.rouverte";
            }
            Commande::Annuler => {
                let avant_ouverture = !matches!(s.etat.as_str(), "ouverte" | "prete" | "en_cours");
                let ok = acces.a(Action::VoyagesValider)
                    || (avant_ouverture && acces.gere(Action::VoyagesModifier));
                if !ok {
                    return Err(interdit());
                }
                motif_decision = Some(motif_requis(&motif)?);
                etat = "annulee".into();
                evenement = "voyages.annulee";
            }
            Commande::Requalifier => {
                if !acces.a(Action::VoyagesValider) {
                    return Err(interdit());
                }
                if est_terminal(&s.etat) {
                    return Err(conflit());
                }
                besoin_car = true;
                if !matches!(s.etat.as_str(), "brouillon" | "soumise" | "a_completer") {
                    etat = "preparation".into();
                }
                evenement = "voyages.requalifiee";
            }
        }
        let maj = sqlx::query_as::<_, SortieLigne>(
            "UPDATE ent_sortie SET etat=$3, etat_retour=$4, motif_decision=$5, liste_confirmee_le=$6,
                    liste_confirmee_par_user_id=$7, contournement_devis_motif=$8, besoin_car=$9,
                    version = version + 1, modifie_le = now()
              WHERE id = $1 AND etablissement_id = $2
              RETURNING *",
        )
        .bind(id)
        .bind(a.etablissement_id)
        .bind(&etat)
        .bind(&etat_retour)
        .bind(&motif_decision)
        .bind(liste.0)
        .bind(liste.1)
        .bind(&contournement)
        .bind(besoin_car)
        .fetch_one(&mut *t)
        .await
        .map_err(db)?;
        let action = evenement.trim_start_matches("voyages.");
        journaliser(
            &mut t,
            a,
            id,
            Fait {
                etat_avant: Some(&s.etat),
                etat_apres: Some(&etat),
                motif: motif.as_deref(),
                ..Fait::simple(action)
            },
        )
        .await?;
        let ev = publier(
            &mut t,
            a,
            evenement,
            id,
            json!({ "avant": s.etat, "apres": etat }),
        )
        .await?;
        t.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&[ev]);
        Ok(maj)
    }

    /// VOY-112 (NX-04) : valider la pédagogie de toute une série. Le droit est vérifié **séance par
    /// séance** (corrige VD-18) ; chaque séance est traitée dans sa propre transaction et le résultat
    /// dit celles qui n'ont pas pu l'être.
    pub async fn voyage_serie_valider(
        &self,
        a: &Acteur<'_>,
        serie_id: Uuid,
    ) -> Result<Vec<ResultatSeance>, EntDbError> {
        // Titres et dates des séances ne sortent que pour qui peut valider (jamais d'identifiant de série seul).
        self.voyages_exiger(a, Action::VoyagesValider).await?;
        let seances: Vec<(Uuid, String, DateTime<Utc>)> = {
            let mut t = tx(self, a).await?;
            let r = sqlx::query_as(
                "SELECT id, titre, debut FROM ent_sortie
                  WHERE etablissement_id = $1 AND serie_id = $2 ORDER BY debut",
            )
            .bind(a.etablissement_id)
            .bind(serie_id)
            .fetch_all(&mut *t)
            .await
            .map_err(db)?;
            t.commit().await.map_err(db)?;
            r
        };
        if seances.is_empty() {
            return Err(introuvable());
        }
        let mut sortie = Vec::new();
        for (id, titre, debut) in seances {
            let r = self
                .voyage_commande(a, id, Commande::ValiderPedagogie, ParamsCommande::default())
                .await;
            let (ok, raison) = match r {
                Ok(_) => (true, None),
                Err(EntDbError::Forbidden) => (false, Some("droit insuffisant".to_string())),
                Err(EntDbError::Refus(_, m)) => (false, Some(m.to_string())),
                Err(EntDbError::Domain(_)) => (false, Some("introuvable".to_string())),
                Err(e) => return Err(e),
            };
            sortie.push(ResultatSeance {
                sortie_id: id,
                titre,
                debut,
                ok,
                raison,
            });
        }
        Ok(sortie)
    }

    /// VOY-108 (NX-13) : attribue ou transfère le porteur (un agent du personnel, socle 6).
    pub async fn voyage_changer_porteur(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
        agent_id: Uuid,
    ) -> Result<SortieLigne, EntDbError> {
        self.voyages_exiger(a, Action::VoyagesPorteur).await?;
        let mut t = tx(self, a).await?;
        let s = charger(&mut t, a, id, true).await?;
        if est_terminal(&s.etat) {
            return Err(conflit());
        }
        let nouveau_porteur = personne_de_agent(&mut t, a, agent_id)
            .await?
            .ok_or_else(|| {
                refus(
                    RefusMetier::Invalide,
                    "Le porteur doit être un membre du personnel de l'établissement.",
                )
            })?;
        let maj = sqlx::query_as::<_, SortieLigne>(
            "UPDATE ent_sortie SET porteur_personne_id = $3, version = version + 1, modifie_le = now()
              WHERE id = $1 AND etablissement_id = $2 RETURNING *",
        )
        .bind(id)
        .bind(a.etablissement_id)
        .bind(nouveau_porteur)
        .fetch_one(&mut *t)
        .await
        .map_err(db)?;
        journaliser(
            &mut t,
            a,
            id,
            Fait {
                champ: Some("porteur"),
                ..Fait::simple("porteur_change")
            },
        )
        .await?;
        let ev = publier(&mut t, a, "voyages.porteur_change", id, json!({})).await?;
        t.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&[ev]);
        Ok(maj)
    }

    /// VOY-138 : remplace le programme (étapes lieu/heure). Les types « dépôt » et « reprise »
    /// alimentent la confirmation de la liste d'un voyage (NX-28).
    pub async fn voyage_etapes_remplacer(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
        etapes: Vec<EtapeSaisie>,
    ) -> Result<Vec<EtapeVue>, EntDbError> {
        if etapes.len() > 100 {
            return Err(invalide());
        }
        let mut etapes = etapes;
        etapes.sort_by_key(|e| e.debut);
        for e in &etapes {
            if !TYPES_ETAPE.contains(&e.type_etape.as_str()) {
                return Err(invalide());
            }
            if e.fin.is_some_and(|f| f < e.debut) || e.duree_minutes.is_some_and(|d| d <= 0) {
                return Err(invalide());
            }
        }
        let mut t = tx(self, a).await?;
        let (s, acces) = charger_avec_acces(self, &mut t, a, id, true).await?;
        if !acces.gere(Action::VoyagesModifier) {
            return Err(interdit());
        }
        if !est_modifiable(&s.etat) {
            return Err(refus(
                RefusMetier::Conflit,
                "Ce dossier n'est plus modifiable.",
            ));
        }
        sqlx::query("DELETE FROM ent_sortie_etape WHERE etablissement_id = $1 AND sortie_id = $2")
            .bind(a.etablissement_id)
            .bind(id)
            .execute(&mut *t)
            .await
            .map_err(db)?;
        for (i, e) in etapes.iter().enumerate() {
            sqlx::query(
                "INSERT INTO ent_sortie_etape (id, etablissement_id, sortie_id, ordre, type, debut, fin, duree_minutes, lieu, note, appel_requis)
                 VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11)",
            )
            .bind(Uuid::new_v4())
            .bind(a.etablissement_id)
            .bind(id)
            .bind((i + 1) as i16)
            .bind(&e.type_etape)
            .bind(e.debut)
            .bind(e.fin)
            .bind(e.duree_minutes)
            .bind(texte_opt(e.lieu.as_deref(), 300)?)
            .bind(texte_opt(e.note.as_deref(), 1000)?)
            .bind(e.appel_requis)
            .execute(&mut *t)
            .await
            .map_err(db)?;
        }
        sqlx::query(
            "UPDATE ent_sortie SET version = version + 1, modifie_le = now() WHERE id = $1 AND etablissement_id = $2",
        )
        .bind(id)
        .bind(a.etablissement_id)
        .execute(&mut *t)
        .await
        .map_err(db)?;
        journaliser(
            &mut t,
            a,
            id,
            Fait {
                apres: Some(format!("{} étape(s)", etapes.len())),
                ..Fait::simple("programme_modifie")
            },
        )
        .await?;
        let ev = publier(
            &mut t,
            a,
            "voyages.programme_modifie",
            id,
            json!({ "etapes": etapes.len() }),
        )
        .await?;
        let mut evs = vec![ev];
        // Les horaires de dépôt et de reprise conditionnent la liste d'un voyage : à reconfirmer.
        if s.forme == "voyage" && s.liste_confirmee_le.is_some() {
            evs.extend(rouvrir_liste(&mut t, a, &s, "programme_liste", 0).await?);
        }
        let rows = lire_etapes(&mut t, id).await?;
        t.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&evs);
        Ok(rows)
    }
}

async fn lire_etapes(conn: &mut PgConnection, id: Uuid) -> Result<Vec<EtapeVue>, EntDbError> {
    sqlx::query_as::<_, EtapeVue>(
        "SELECT id, ordre, type, debut, fin, duree_minutes, lieu, note, appel_requis
           FROM ent_sortie_etape WHERE sortie_id = $1 ORDER BY ordre",
    )
    .bind(id)
    .fetch_all(conn)
    .await
    .map_err(db)
}

// ───────────────────────── Participants ─────────────────────────

impl PgEnt {
    /// Registre des élèves proposés (NX-23, NX-24) : élèves **scolarisés à la date de la sortie** des
    /// classes et groupes visés, ou d'une autre classe au choix. Projection minimale (nom, prénom).
    /// La consultation est tracée au journal (NX-68, R-SV27).
    pub async fn voyage_candidats(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
        classe_id: Option<Uuid>,
    ) -> Result<Vec<CandidatsClasse>, EntDbError> {
        let mut t = tx(self, a).await?;
        let (s, acces) = charger_avec_acces(self, &mut t, a, id, false).await?;
        if !acces.gere(Action::VoyagesParticipants) {
            return Err(interdit());
        }
        let jour: NaiveDate =
            sqlx::query_scalar("SELECT ($1::timestamptz AT TIME ZONE 'Europe/Paris')::date")
                .bind(s.debut)
                .fetch_one(&mut *t)
                .await
                .map_err(db)?;
        let cibles: Vec<(Option<Uuid>, Option<Uuid>, String)> = if let Some(c) = classe_id {
            let lib: Option<String> = sqlx::query_scalar(
                "SELECT libelle FROM ent_classe WHERE etablissement_id = $1 AND id = $2",
            )
            .bind(a.etablissement_id)
            .bind(c)
            .fetch_optional(&mut *t)
            .await
            .map_err(db)?;
            vec![(Some(c), None, lib.ok_or_else(introuvable)?)]
        } else {
            sqlx::query_as(
                "SELECT p.classe_id, p.groupe_id, COALESCE(c.libelle, g.libelle)
                   FROM ent_sortie_public p
                   LEFT JOIN ent_classe c ON c.id = p.classe_id
                   LEFT JOIN ent_groupe g ON g.id = p.groupe_id
                  WHERE p.etablissement_id = $1 AND p.sortie_id = $2
                  ORDER BY 3",
            )
            .bind(a.etablissement_id)
            .bind(id)
            .fetch_all(&mut *t)
            .await
            .map_err(db)?
        };
        let mut out = Vec::new();
        for (c, g, libelle) in cibles {
            let eleves = sqlx::query_as::<_, Candidat>(
                "SELECT e.id AS eleve_id, e.nom, e.prenom,
                        EXISTS (SELECT 1 FROM ent_sortie_participant sp
                                 WHERE sp.sortie_id = $2 AND sp.eleve_id = e.id
                                   AND sp.etat NOT IN ('desiste','retire')) AS deja_inscrit
                   FROM ent_eleve e
                  WHERE e.etablissement_id = $1 AND e.status = 'actif'
                    AND EXISTS (SELECT 1 FROM ent_affectation af
                                 WHERE af.etablissement_id = e.etablissement_id AND af.eleve_id = e.id
                                   AND (($3::uuid IS NOT NULL AND af.classe_id = $3)
                                     OR ($4::uuid IS NOT NULL AND af.groupe_id = $4))
                                   AND af.date_debut <= $5 AND (af.date_fin IS NULL OR af.date_fin >= $5))
                  ORDER BY e.nom, e.prenom",
            )
            .bind(a.etablissement_id)
            .bind(id)
            .bind(c)
            .bind(g)
            .bind(jour)
            .fetch_all(&mut *t)
            .await
            .map_err(db)?;
            out.push(CandidatsClasse {
                classe_id: c,
                groupe_id: g,
                libelle,
                eleves,
            });
        }
        journaliser(
            &mut t,
            a,
            id,
            Fait {
                apres: Some(format!(
                    "{} élève(s) affichés",
                    out.iter().map(|c| c.eleves.len()).sum::<usize>()
                )),
                ..Fait::simple("registre_consulte")
            },
        )
        .await?;
        t.commit().await.map_err(db)?;
        Ok(out)
    }

    /// VOY-140 (NX-23) : inscrit des élèves choisis dans le registre. Refuse tout élève absent de
    /// l'établissement ou non scolarisé à la date de la sortie (R-SV2, corrige VD-44). Une liste déjà
    /// confirmée repasse en brouillon (NX-26) ; une sortie `prete` redevient `ouverte`.
    pub async fn voyage_participants_ajouter(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
        eleves: Vec<Uuid>,
    ) -> Result<ResultatAjout, EntDbError> {
        if eleves.is_empty() || eleves.len() > 400 {
            return Err(invalide());
        }
        let mut t = tx(self, a).await?;
        let (s, acces) = charger_avec_acces(self, &mut t, a, id, true).await?;
        if !acces.gere(Action::VoyagesParticipants) {
            return Err(interdit());
        }
        if !est_modifiable(&s.etat) {
            return Err(refus(
                RefusMetier::Conflit,
                "Ce dossier n'accepte plus de changement de liste.",
            ));
        }
        let jour: NaiveDate =
            sqlx::query_scalar("SELECT ($1::timestamptz AT TIME ZONE 'Europe/Paris')::date")
                .bind(s.debut)
                .fetch_one(&mut *t)
                .await
                .map_err(db)?;
        let valides: HashSet<Uuid> = sqlx::query_scalar::<_, Uuid>(
            "SELECT e.id FROM ent_eleve e
              WHERE e.etablissement_id = $1 AND e.id = ANY($2) AND e.status = 'actif'
                AND EXISTS (SELECT 1 FROM ent_affectation af
                             WHERE af.etablissement_id = e.etablissement_id AND af.eleve_id = e.id
                               AND af.classe_id IS NOT NULL
                               AND af.date_debut <= $3 AND (af.date_fin IS NULL OR af.date_fin >= $3))",
        )
        .bind(a.etablissement_id)
        .bind(&eleves)
        .bind(jour)
        .fetch_all(&mut *t)
        .await
        .map_err(db)?
        .into_iter()
        .collect();
        let mut res = ResultatAjout {
            ajoutes: vec![],
            deja: vec![],
            refuses: vec![],
        };
        let mut vus = HashSet::new();
        for e in eleves {
            if !vus.insert(e) {
                continue;
            }
            if !valides.contains(&e) {
                res.refuses.push(e);
                continue;
            }
            let existant: Option<(Uuid, String)> = sqlx::query_as(
                "SELECT id, etat FROM ent_sortie_participant WHERE sortie_id = $1 AND eleve_id = $2 FOR UPDATE",
            )
            .bind(id)
            .bind(e)
            .fetch_optional(&mut *t)
            .await
            .map_err(db)?;
            match existant {
                Some((_, etat)) if !matches!(etat.as_str(), "desiste" | "retire") => {
                    res.deja.push(e)
                }
                Some((pid, etat)) => {
                    sqlx::query(
                        "UPDATE ent_sortie_participant SET etat = 'inscrit', modifie_le = now() WHERE id = $1",
                    )
                    .bind(pid)
                    .execute(&mut *t)
                    .await
                    .map_err(db)?;
                    fait_participant(
                        &mut t,
                        a,
                        pid,
                        "reintegration",
                        Some(&etat),
                        "inscrit",
                        None,
                        None,
                    )
                    .await?;
                    res.ajoutes.push(e);
                }
                None => {
                    let pid = Uuid::new_v4();
                    sqlx::query(
                        "INSERT INTO ent_sortie_participant (id, etablissement_id, sortie_id, eleve_id, cree_par_user_id)
                         VALUES ($1,$2,$3,$4,$5)",
                    )
                    .bind(pid)
                    .bind(a.etablissement_id)
                    .bind(id)
                    .bind(e)
                    .bind(a.user_id)
                    .execute(&mut *t)
                    .await
                    .map_err(db)?;
                    fait_participant(&mut t, a, pid, "inscription", None, "inscrit", None, None)
                        .await?;
                    res.ajoutes.push(e);
                }
            }
        }
        if res.ajoutes.is_empty() {
            t.commit().await.map_err(db)?;
            return Ok(res);
        }
        let ev = rouvrir_liste(&mut t, a, &s, "participant_inscrit", res.ajoutes.len()).await?;
        t.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&ev);
        Ok(res)
    }

    /// VOY-145 : désistement à la demande de la famille, avec motif codé et date. La liste confirmée
    /// reste confirmée (le désistement est un fait daté, pas une modification de liste).
    pub async fn voyage_participant_desister(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
        participant_id: Uuid,
        motif_code: &str,
        motif_texte: Option<&str>,
    ) -> Result<(), EntDbError> {
        self.changer_etat_participant(
            a,
            id,
            participant_id,
            "desiste",
            "desistement",
            motif_code,
            motif_texte,
        )
        .await
    }

    /// VOY-146 : retire un participant, décision de l'établissement avec motif codé. **Jamais pour
    /// raison financière** sans décision motivée de la direction (R-SV19).
    pub async fn voyage_participant_retirer(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
        participant_id: Uuid,
        motif_code: &str,
        motif_texte: Option<&str>,
    ) -> Result<(), EntDbError> {
        self.changer_etat_participant(
            a,
            id,
            participant_id,
            "retire",
            "retrait",
            motif_code,
            motif_texte,
        )
        .await
    }

    #[allow(clippy::too_many_arguments)]
    async fn changer_etat_participant(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
        participant_id: Uuid,
        vers: &str,
        type_fait: &str,
        motif_code: &str,
        motif_texte: Option<&str>,
    ) -> Result<(), EntDbError> {
        if !MOTIFS_PARTICIPANT.contains(&motif_code) {
            return Err(invalide());
        }
        let motif_texte = texte_opt(motif_texte, 1000)?;
        let mut t = tx(self, a).await?;
        let (s, acces) = charger_avec_acces(self, &mut t, a, id, true).await?;
        if !acces.gere(Action::VoyagesParticipants) {
            return Err(interdit());
        }
        if !est_modifiable(&s.etat) {
            return Err(refus(
                RefusMetier::Conflit,
                "Ce dossier n'accepte plus de changement de liste.",
            ));
        }
        if motif_code == "financier"
            && !(acces.a(Action::VoyagesAutoriser) && motif_texte.is_some())
        {
            return Err(refus(
                RefusMetier::Interdit,
                "Aucun élève n'est écarté pour raison financière sans décision motivée de la direction.",
            ));
        }
        let actuel: Option<String> = sqlx::query_scalar(
            "SELECT etat FROM ent_sortie_participant
              WHERE id = $1 AND sortie_id = $2 AND etablissement_id = $3 FOR UPDATE",
        )
        .bind(participant_id)
        .bind(id)
        .bind(a.etablissement_id)
        .fetch_optional(&mut *t)
        .await
        .map_err(db)?;
        let actuel = actuel.ok_or_else(introuvable)?;
        if matches!(actuel.as_str(), "desiste" | "retire") {
            return Err(conflit());
        }
        sqlx::query(
            "UPDATE ent_sortie_participant SET etat = $2, rang_attente = NULL, modifie_le = now() WHERE id = $1",
        )
        .bind(participant_id)
        .bind(vers)
        .execute(&mut *t)
        .await
        .map_err(db)?;
        fait_participant(
            &mut t,
            a,
            participant_id,
            type_fait,
            Some(&actuel),
            vers,
            Some(motif_code),
            motif_texte.as_deref(),
        )
        .await?;
        let mut evs = Vec::new();
        if vers == "retire" {
            // Retirer un élève modifie la liste : à reconfirmer (NX-26).
            evs.extend(rouvrir_liste(&mut t, a, &s, "participant_retire", 1).await?);
        } else {
            journaliser(
                &mut t,
                a,
                id,
                Fait {
                    motif: Some(motif_code),
                    ..Fait::simple("participant_desiste")
                },
            )
            .await?;
            evs.push(
                publier(
                    &mut t,
                    a,
                    "voyages.participant_desiste",
                    id,
                    json!({ "motif": motif_code }),
                )
                .await?,
            );
        }
        t.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&evs);
        Ok(())
    }

    /// VOY-143 : liste nominative des participants (porteur, encadrants, direction, secrétariat). La
    /// consultation est tracée (R-SV27).
    pub async fn voyage_participants(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
    ) -> Result<Vec<ParticipantVue>, EntDbError> {
        let mut t = tx(self, a).await?;
        let (s, acces) = charger_avec_acces(self, &mut t, a, id, false).await?;
        if !(acces.detail() || acces.a(Action::VoyagesParticipants)) {
            return Err(introuvable());
        }
        let v = liste_participants(&mut t, a, &s, &[]).await?;
        journaliser(
            &mut t,
            a,
            id,
            Fait {
                apres: Some(format!("{} participant(s)", v.len())),
                ..Fait::simple("liste_consultee")
            },
        )
        .await?;
        t.commit().await.map_err(db)?;
        Ok(v)
    }

    /// VOY-148 (NX-31) : export tableur de la liste (nom, prénom, classe), colonnes minimales,
    /// formules neutralisées, **export journalisé**. Réservé au porteur et à ceux qui gèrent ou lisent
    /// toutes les sorties (jamais à un simple encadrant).
    pub async fn voyage_participants_csv(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
    ) -> Result<String, EntDbError> {
        let mut t = tx(self, a).await?;
        let (s, acces) = charger_avec_acces(self, &mut t, a, id, false).await?;
        if !(acces.porteur || acces.a(Action::VoyagesLire) || acces.a(Action::VoyagesParticipants))
        {
            return Err(interdit());
        }
        let v = liste_participants(
            &mut t,
            a,
            &s,
            &["inscrit", "pret", "parti", "rentre", "remis"],
        )
        .await?;
        let mut out = String::from("\u{feff}Nom;Prénom;Classe\r\n");
        for p in &v {
            out.push_str(&format!(
                "{};{};{}\r\n",
                champ_csv(&p.nom),
                champ_csv(&p.prenom),
                champ_csv(p.classe.as_deref().unwrap_or(""))
            ));
        }
        journaliser(
            &mut t,
            a,
            id,
            Fait {
                apres: Some(format!("{} ligne(s)", v.len())),
                ..Fait::simple("liste_exportee")
            },
        )
        .await?;
        let ev = publier(
            &mut t,
            a,
            "voyages.liste_exportee",
            id,
            json!({ "lignes": v.len() }),
        )
        .await?;
        t.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&[ev]);
        Ok(out)
    }
}

#[allow(clippy::too_many_arguments)]
async fn fait_participant(
    conn: &mut PgConnection,
    a: &Acteur<'_>,
    participant: Uuid,
    type_fait: &str,
    avant: Option<&str>,
    apres: &str,
    motif_code: Option<&str>,
    motif_texte: Option<&str>,
) -> Result<(), EntDbError> {
    sqlx::query(
        "INSERT INTO ent_sortie_participant_fait
             (id, etablissement_id, participant_id, acteur_user_id, type, etat_avant, etat_apres, motif_code, motif_texte)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)",
    )
    .bind(Uuid::new_v4())
    .bind(a.etablissement_id)
    .bind(participant)
    .bind(a.user_id)
    .bind(type_fait)
    .bind(avant)
    .bind(apres)
    .bind(motif_code)
    .bind(motif_texte)
    .execute(conn)
    .await
    .map_err(db)?;
    Ok(())
}

/// Une modification de la liste annule sa confirmation (NX-26) et rouvre une sortie `prete`.
async fn rouvrir_liste(
    conn: &mut PgConnection,
    a: &Acteur<'_>,
    s: &SortieLigne,
    action: &str,
    nombre: usize,
) -> Result<Vec<MetierEvent>, EntDbError> {
    let etat_apres = if s.etat == "prete" {
        "ouverte"
    } else {
        s.etat.as_str()
    };
    if s.liste_confirmee_le.is_some() {
        sqlx::query(
            "UPDATE ent_sortie SET liste_confirmee_le = NULL, liste_confirmee_par_user_id = NULL, etat = $3,
                    version = version + 1, modifie_le = now()
              WHERE id = $1 AND etablissement_id = $2",
        )
        .bind(s.id)
        .bind(a.etablissement_id)
        .bind(etat_apres)
        .execute(&mut *conn)
        .await
        .map_err(db)?;
        journaliser(
            conn,
            a,
            s.id,
            Fait {
                etat_avant: Some(&s.etat),
                etat_apres: Some(etat_apres),
                motif: Some("liste modifiée : à reconfirmer"),
                ..Fait::simple("liste_rouverte")
            },
        )
        .await?;
    }
    journaliser(
        conn,
        a,
        s.id,
        Fait {
            apres: Some(format!("{nombre} élève(s)")),
            ..Fait::simple(action)
        },
    )
    .await?;
    Ok(vec![
        publier(
            conn,
            a,
            &format!("voyages.{action}"),
            s.id,
            json!({ "nombre": nombre }),
        )
        .await?,
    ])
}

async fn liste_participants(
    conn: &mut PgConnection,
    a: &Acteur<'_>,
    s: &SortieLigne,
    etats: &[&str],
) -> Result<Vec<ParticipantVue>, EntDbError> {
    let etats: Vec<String> = etats.iter().map(|x| x.to_string()).collect();
    sqlx::query_as::<_, ParticipantVue>(
        "SELECT sp.id, sp.eleve_id, e.nom, e.prenom, sp.etat,
                (SELECT c.libelle FROM ent_affectation af JOIN ent_classe c ON c.id = af.classe_id
                  WHERE af.etablissement_id = e.etablissement_id AND af.eleve_id = e.id AND af.classe_id IS NOT NULL
                    AND af.date_debut <= ($3::timestamptz AT TIME ZONE 'Europe/Paris')::date
                    AND (af.date_fin IS NULL OR af.date_fin >= ($3::timestamptz AT TIME ZONE 'Europe/Paris')::date)
                  ORDER BY af.date_debut DESC LIMIT 1) AS classe
           FROM ent_sortie_participant sp
           JOIN ent_eleve e ON e.etablissement_id = sp.etablissement_id AND e.id = sp.eleve_id
          WHERE sp.etablissement_id = $1 AND sp.sortie_id = $2
            AND (cardinality($4::text[]) = 0 OR sp.etat = ANY($4))
          ORDER BY e.nom, e.prenom",
    )
    .bind(a.etablissement_id)
    .bind(s.id)
    .bind(s.debut)
    .bind(&etats)
    .fetch_all(conn)
    .await
    .map_err(db)
}

// ───────────────────────── Encadrants ─────────────────────────

impl PgEnt {
    /// VOY-150 (NX-32) : ajoute un encadrant **du personnel** (agent du socle 6, tel que l'annuaire du
    /// personnel le désigne) avec son rôle. Aucun nom libre (VD-19) ; un intervenant extérieur est
    /// d'abord créé au socle 6.
    pub async fn voyage_encadrant_ajouter(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
        agent_id: Uuid,
        role: &str,
    ) -> Result<EncadrantVue, EntDbError> {
        if !ROLES_ENCADRANT.contains(&role) {
            return Err(invalide());
        }
        let mut t = tx(self, a).await?;
        let (s, acces) = charger_avec_acces(self, &mut t, a, id, true).await?;
        if !acces.gere(Action::VoyagesEncadrement) {
            return Err(interdit());
        }
        if !est_modifiable(&s.etat) {
            return Err(refus(
                RefusMetier::Conflit,
                "Ce dossier n'est plus modifiable.",
            ));
        }
        let personne_id = personne_de_agent(&mut t, a, agent_id).await?.ok_or_else(|| {
            refus(
                RefusMetier::Invalide,
                "Cet adulte n'est pas dans le personnel de l'établissement : créez-le d'abord (personnels).",
            )
        })?;
        let deja: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM ent_sortie_encadrant WHERE sortie_id = $1 AND personne_id = $2)",
        )
        .bind(id)
        .bind(personne_id)
        .fetch_one(&mut *t)
        .await
        .map_err(db)?;
        if deja {
            return Err(conflit());
        }
        let eid = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_sortie_encadrant (id, etablissement_id, sortie_id, personne_id, role, cree_par_user_id)
             VALUES ($1,$2,$3,$4,$5,$6)",
        )
        .bind(eid)
        .bind(a.etablissement_id)
        .bind(id)
        .bind(personne_id)
        .bind(role)
        .bind(a.user_id)
        .execute(&mut *t)
        .await
        .map_err(db)?;
        journaliser(
            &mut t,
            a,
            id,
            Fait {
                champ: Some("encadrant"),
                apres: Some(role.to_string()),
                ..Fait::simple("encadrant_ajoute")
            },
        )
        .await?;
        let ev = publier(
            &mut t,
            a,
            "voyages.encadrant_ajoute",
            id,
            json!({ "role": role }),
        )
        .await?;
        let v = sqlx::query_as::<_, EncadrantVue>(
            "SELECT se.id, ag.id AS agent_id, p.nom, p.prenom, se.role
               FROM ent_sortie_encadrant se
               JOIN ent_personne p ON p.etablissement_id = se.etablissement_id AND p.id = se.personne_id
               JOIN ent_agent ag ON ag.etablissement_id = se.etablissement_id AND ag.personne_id = se.personne_id
              WHERE se.id = $1",
        )
        .bind(eid)
        .fetch_one(&mut *t)
        .await
        .map_err(db)?;
        t.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&[ev]);
        Ok(v)
    }

    pub async fn voyage_encadrant_retirer(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
        encadrant_id: Uuid,
    ) -> Result<(), EntDbError> {
        let mut t = tx(self, a).await?;
        let (s, acces) = charger_avec_acces(self, &mut t, a, id, true).await?;
        if !acces.gere(Action::VoyagesEncadrement) {
            return Err(interdit());
        }
        if !est_modifiable(&s.etat) {
            return Err(refus(
                RefusMetier::Conflit,
                "Ce dossier n'est plus modifiable.",
            ));
        }
        let lignes = sqlx::query(
            "DELETE FROM ent_sortie_encadrant WHERE id = $1 AND sortie_id = $2 AND etablissement_id = $3",
        )
        .bind(encadrant_id)
        .bind(id)
        .bind(a.etablissement_id)
        .execute(&mut *t)
        .await
        .map_err(db)?
        .rows_affected();
        if lignes == 0 {
            return Err(introuvable());
        }
        journaliser(
            &mut t,
            a,
            id,
            Fait {
                champ: Some("encadrant"),
                ..Fait::simple("encadrant_retire")
            },
        )
        .await?;
        let ev = publier(&mut t, a, "voyages.encadrant_retire", id, json!({})).await?;
        t.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&[ev]);
        Ok(())
    }
}

// ───────────────────────── Lectures et projections ─────────────────────────

impl PgEnt {
    /// VOY-134 (NX-17) : liste des sorties, **projection par rôle** (corrige VD-03). Tout le
    /// personnel voit la liste minimale des sorties ouvertes ; le porteur, les encadrants et les
    /// détenteurs de `voyages.lire` voient aussi les dossiers en cours de préparation. Les brouillons
    /// et refus des autres restent invisibles à un professeur non concerné.
    pub async fn voyages_liste(
        &self,
        a: &Acteur<'_>,
        f: FiltreVoyages,
    ) -> Result<Vec<SortieResume>, EntDbError> {
        let d = self.voyages_droits(a).await?;
        if !d.contains(&Action::VoyagesAnnuaire) {
            return Err(interdit());
        }
        let tout = d.contains(&Action::VoyagesLire);
        if let Some(e) = &f.etat {
            if !crate::voyages::ETATS.contains(&e.as_str()) {
                return Err(invalide());
            }
        }
        let q = texte_opt(f.q.as_deref(), 80)?.map(|s| format!("%{}%", s.replace(['%', '_'], "")));
        let mut t = tx(self, a).await?;
        let moi = personne_de(&mut t, a).await?;
        let rows = sqlx::query_as::<_, SortieResume>(
            "SELECT s.id, s.reference, s.titre, s.debut, s.fin, s.destination, s.destination_libelle, s.nature, s.forme,
                    s.etat, s.serie_id,
                    CASE WHEN x.detail THEN trim(coalesce(p.prenom || ' ', '') || p.nom) END AS porteur_nom,
                    CASE WHEN x.detail THEN (SELECT count(*) FROM ent_sortie_participant sp
                                              WHERE sp.sortie_id = s.id AND sp.etat IN ('inscrit','pret','bloque','parti')) END AS nb_inscrits,
                    x.detail AS acces_detail,
                    COALESCE(s.porteur_personne_id = $3, false) AS est_porteur
               FROM ent_sortie s
               JOIN ent_personne p ON p.etablissement_id = s.etablissement_id AND p.id = s.porteur_personne_id
               CROSS JOIN LATERAL (
                    SELECT COALESCE($2 OR s.porteur_personne_id = $3 OR EXISTS (
                               SELECT 1 FROM ent_sortie_encadrant se
                                WHERE se.sortie_id = s.id AND se.personne_id = $3), false) AS detail) x
              WHERE s.etablissement_id = $1
                AND (x.detail OR s.etat IN ('ouverte','prete','en_cours','rentree','close'))
                AND ($4::text IS NULL OR s.etat = $4)
                AND ($5::text IS NULL OR s.titre ILIKE $5 OR s.destination_libelle ILIKE $5
                     OR (x.detail AND (p.nom ILIKE $5 OR p.prenom ILIKE $5)))
                AND (NOT $6 OR s.porteur_personne_id = $3 OR EXISTS (
                        SELECT 1 FROM ent_sortie_encadrant se WHERE se.sortie_id = s.id AND se.personne_id = $3))
                AND (NOT $7 OR s.fin >= now())
              ORDER BY (s.fin < now()), CASE WHEN s.fin >= now() THEN s.debut END ASC, s.debut DESC
              LIMIT 500",
        )
        .bind(a.etablissement_id)
        .bind(tout)
        .bind(moi)
        .bind(&f.etat)
        .bind(q)
        .bind(f.mes)
        .bind(f.a_venir)
        .fetch_all(&mut *t)
        .await
        .map_err(db)?;
        t.commit().await.map_err(db)?;
        Ok(rows)
    }

    /// Détail d'une sortie (porteur, encadrants, détenteurs de `voyages.lire`) avec droits calculés,
    /// prochaine étape (NX-14) et audit (NX-15). Un autre profil reçoit « introuvable ».
    pub async fn voyage_detail(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
    ) -> Result<SortieDetail, EntDbError> {
        let mut t = tx(self, a).await?;
        let (s, acces) = charger_avec_acces(self, &mut t, a, id, false).await?;
        let mesures = dossier(&mut t, &s).await?;
        let porteur_nom: String = sqlx::query_scalar(
            "SELECT trim(coalesce(prenom || ' ', '') || nom) FROM ent_personne WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(a.etablissement_id)
        .bind(s.porteur_personne_id)
        .fetch_one(&mut *t)
        .await
        .map_err(db)?;
        let publics = sqlx::query_as::<_, PublicVue>(
            "SELECT p.id, p.classe_id, p.groupe_id, COALESCE(c.libelle, g.libelle) AS libelle
               FROM ent_sortie_public p
               LEFT JOIN ent_classe c ON c.id = p.classe_id
               LEFT JOIN ent_groupe g ON g.id = p.groupe_id
              WHERE p.etablissement_id = $1 AND p.sortie_id = $2 ORDER BY 4",
        )
        .bind(a.etablissement_id)
        .bind(id)
        .fetch_all(&mut *t)
        .await
        .map_err(db)?;
        let etapes = lire_etapes(&mut t, id).await?;
        let encadrants = sqlx::query_as::<_, EncadrantVue>(
            "SELECT se.id, ag.id AS agent_id, p.nom, p.prenom, se.role
               FROM ent_sortie_encadrant se
               JOIN ent_personne p ON p.etablissement_id = se.etablissement_id AND p.id = se.personne_id
               JOIN ent_agent ag ON ag.etablissement_id = se.etablissement_id AND ag.personne_id = se.personne_id
              WHERE se.etablissement_id = $1 AND se.sortie_id = $2 ORDER BY se.role, p.nom",
        )
        .bind(a.etablissement_id)
        .bind(id)
        .fetch_all(&mut *t)
        .await
        .map_err(db)?;
        let nb_desistes: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM ent_sortie_participant WHERE sortie_id = $1 AND etat IN ('desiste','retire')",
        )
        .bind(id)
        .fetch_one(&mut *t)
        .await
        .map_err(db)?;
        t.commit().await.map_err(db)?;
        Ok(SortieDetail {
            prochaine_etape: prochaine_etape(
                &s.etat,
                &s.contexte(),
                s.liste_confirmee_le.is_some(),
            ),
            audit: audit(&mesures.etat(&s)),
            nb_inscrits: mesures.nb_inscrits,
            nb_desistes,
            porteur_nom,
            publics,
            etapes,
            encadrants,
            acces: acces.vue(),
            commandes: commandes_disponibles(&s, &acces),
            sortie: s,
        })
    }

    /// VOY-131 (NX-15) : audit des manques et freins d'un dossier.
    pub async fn voyage_audit(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
    ) -> Result<Vec<PointAudit>, EntDbError> {
        Ok(self.voyage_detail(a, id).await?.audit)
    }

    /// VOY-136 (NX-19) : journal chronologique du séjour (porteur, direction, comptabilité,
    /// secrétariat ; pas un simple encadrant).
    pub async fn voyage_journal(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
    ) -> Result<Vec<JournalVue>, EntDbError> {
        let mut t = tx(self, a).await?;
        let (_, acces) = charger_avec_acces(self, &mut t, a, id, false).await?;
        if !(acces.porteur || acces.a(Action::VoyagesLire)) {
            return Err(introuvable());
        }
        let v = sqlx::query_as::<_, JournalVue>(
            "SELECT j.a, j.action, j.etat_avant, j.etat_apres, j.champ, j.avant, j.apres, j.motif,
                    (SELECT trim(coalesce(p.prenom || ' ', '') || p.nom) FROM ent_personne p
                      WHERE p.etablissement_id = j.etablissement_id AND p.user_id = j.acteur_user_id LIMIT 1) AS acteur_nom
               FROM ent_sortie_journal j
              WHERE j.etablissement_id = $1 AND j.sortie_id = $2 ORDER BY j.a, j.id",
        )
        .bind(a.etablissement_id)
        .bind(id)
        .fetch_all(&mut *t)
        .await
        .map_err(db)?;
        t.commit().await.map_err(db)?;
        Ok(v)
    }

    /// VOY-135 (NX-18) : tableau de bord de la direction et de la comptabilité.
    pub async fn voyages_tableau(&self, a: &Acteur<'_>) -> Result<TableauDirection, EntDbError> {
        let d = self.voyages_droits(a).await?;
        if !(d.contains(&Action::VoyagesValider)
            || d.contains(&Action::VoyagesAutoriser)
            || d.contains(&Action::VoyagesBudgetValider)
            || d.contains(&Action::VoyagesLire))
        {
            return Err(interdit());
        }
        let mut t = tx(self, a).await?;
        let (a_valider, budget, autoriser, prep, ouvertes, pretes, a_venir, validees, a_completer): (
            i64,
            i64,
            i64,
            i64,
            i64,
            i64,
            i64,
            i64,
            i64,
        ) = sqlx::query_as(
            "SELECT count(*) FILTER (WHERE etat = 'soumise'),
                    count(*) FILTER (WHERE etat = 'budget_a_valider'),
                    count(*) FILTER (WHERE etat = 'a_autoriser'),
                    count(*) FILTER (WHERE etat IN ('preparation','a_signer')),
                    count(*) FILTER (WHERE etat = 'ouverte'),
                    count(*) FILTER (WHERE etat = 'prete'),
                    count(*) FILTER (WHERE fin >= now() AND etat NOT IN ('brouillon','refusee','annulee','close')),
                    count(*) FILTER (WHERE etat IN ('prete','en_cours','rentree','close')
                                       AND annee_id = (SELECT id FROM ent_annee_scolaire WHERE etablissement_id = $1 AND courante)),
                    count(*) FILTER (WHERE etat = 'a_completer')
               FROM ent_sortie WHERE etablissement_id = $1",
        )
        .bind(a.etablissement_id)
        .fetch_one(&mut *t)
        .await
        .map_err(db)?;
        // Dossiers qui ne sont pas prêts alors que le départ approche (NX-16 : « bloqué à J-14 »).
        let bloquees = sqlx::query_as::<_, SortieResume>(
            "SELECT s.id, s.reference, s.titre, s.debut, s.fin, s.destination, s.destination_libelle, s.nature, s.forme,
                    s.etat, s.serie_id,
                    trim(coalesce(p.prenom || ' ', '') || p.nom) AS porteur_nom, NULL::bigint AS nb_inscrits,
                    true AS acces_detail, false AS est_porteur
               FROM ent_sortie s JOIN ent_personne p ON p.etablissement_id = s.etablissement_id AND p.id = s.porteur_personne_id
              WHERE s.etablissement_id = $1
                AND s.etat NOT IN ('prete','en_cours','rentree','close','refusee','annulee')
                AND s.debut BETWEEN now() AND now() + interval '14 days'
              ORDER BY s.debut LIMIT 20",
        )
        .bind(a.etablissement_id)
        .fetch_all(&mut *t)
        .await
        .map_err(db)?;
        t.commit().await.map_err(db)?;
        Ok(TableauDirection {
            a_valider,
            budget_a_valider: budget,
            a_autoriser: autoriser,
            en_preparation: prep,
            ouvertes,
            pretes,
            a_venir,
            validees_de_l_annee: validees,
            a_completer,
            bloquees_proches: bloquees,
        })
    }

    /// Projection famille : les sorties **ouvertes ou plus avancées** où mon enfant (parent) ou moi
    /// (élève) sommes inscrits. Rien d'autre : ni liste des autres élèves, ni budget, ni encadrants.
    pub async fn voyages_famille(&self, a: &Acteur<'_>) -> Result<Vec<VoyageFamille>, EntDbError> {
        let mut t = tx(self, a).await?;
        let v = sqlx::query_as::<_, VoyageFamille>(
            "SELECT s.id AS sortie_id, s.titre, s.debut, s.fin, s.destination, s.destination_libelle,
                    s.point_rendez_vous, s.etat, e.prenom AS enfant_prenom, sp.etat AS etat_participant
               FROM ent_sortie_participant sp
               JOIN ent_sortie s ON s.etablissement_id = sp.etablissement_id AND s.id = sp.sortie_id
               JOIN ent_eleve e ON e.etablissement_id = sp.etablissement_id AND e.id = sp.eleve_id
              WHERE sp.etablissement_id = $1
                AND sp.etat NOT IN ('propose','desiste','retire')
                AND s.etat IN ('ouverte','prete','en_cours','rentree')
                AND (EXISTS (SELECT 1 FROM ent_eleve_lien l
                              WHERE l.etablissement_id = $1 AND l.eleve_id = sp.eleve_id AND l.user_id = $2
                                AND l.suspendu_le IS NULL
                                AND (l.date_debut IS NULL OR l.date_debut <= current_date)
                                AND (l.date_fin IS NULL OR l.date_fin >= current_date))
                  OR EXISTS (SELECT 1 FROM ent_eleve_compte c
                              WHERE c.etablissement_id = $1 AND c.eleve_id = sp.eleve_id AND c.user_id = $2))
              ORDER BY s.debut",
        )
        .bind(a.etablissement_id)
        .bind(a.user_id)
        .fetch_all(&mut *t)
        .await
        .map_err(db)?;
        t.commit().await.map_err(db)?;
        Ok(v)
    }

    /// VOY-170 (NX-59), version V0a **sans fichier** : le texte de la circulaire aux familles, généré
    /// depuis les champs du séjour (jamais depuis un texte venu du navigateur). **Corrige le défaut de
    /// Next.js** (génération sans contrôle de droits) : réservé au porteur ou à ceux qui gèrent les
    /// sorties, et seulement une fois la sortie `ouverte` (R-SV5). Le PDF est le lot V0b.
    pub async fn voyage_circulaire(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
    ) -> Result<Circulaire, EntDbError> {
        let mut t = tx(self, a).await?;
        let (s, acces) = charger_avec_acces(self, &mut t, a, id, false).await?;
        if !acces.gere(Action::VoyagesModifier) {
            return Err(interdit());
        }
        if !matches!(s.etat.as_str(), "ouverte" | "prete") {
            return Err(refus(
                RefusMetier::Conflit,
                "La circulaire aux familles se prépare une fois la sortie autorisée.",
            ));
        }
        let (debut, fin): (String, String) = sqlx::query_as(
            "SELECT to_char($1::timestamptz AT TIME ZONE 'Europe/Paris', 'DD/MM/YYYY à HH24:MI'),
                    to_char($2::timestamptz AT TIME ZONE 'Europe/Paris', 'DD/MM/YYYY à HH24:MI')",
        )
        .bind(s.debut)
        .bind(s.fin)
        .fetch_one(&mut *t)
        .await
        .map_err(db)?;
        let mut corps = format!(
            "Madame, Monsieur,\n\nNous organisons la sortie « {} ».\n\n",
            s.titre
        );
        corps.push_str(&format!("Départ : {debut}\nRetour : {fin}\n"));
        if let Some(d) = &s.destination_libelle {
            corps.push_str(&format!("Destination : {d}\n"));
        }
        if let Some(r) = &s.point_rendez_vous {
            corps.push_str(&format!("Rendez-vous : {r}\n"));
        }
        if let Some(o) = &s.objectifs {
            corps.push_str(&format!("\nObjectifs : {o}\n"));
        }
        if let Some(p) = &s.programme_libelle {
            corps.push_str(&format!("\nProgramme : {p}\n"));
        }
        if let Some(c) = &s.consignes_transport {
            corps.push_str(&format!("\nConsignes : {c}\n"));
        }
        if s.nature == "facultative" {
            corps.push_str("\nCette sortie est facultative : l'autorisation d'un responsable légal est demandée.\n");
        }
        corps.push_str("\nNous restons à votre disposition.\n");
        journaliser(&mut t, a, id, Fait::simple("circulaire_generee")).await?;
        t.commit().await.map_err(db)?;
        Ok(Circulaire {
            titre: format!("Circulaire — {}", s.titre),
            reference: s.reference,
            corps,
        })
    }

    /// Port « en sortie » (NX-35, NX-82 ; R-SV24) pour l'appel, la vie scolaire, l'internat et la
    /// cantine : les élèves **inscrits à une sortie prête ou en cours** dont la plage chevauche la
    /// fenêtre demandée. Lecture seule, sans accès aux données de la sortie au-delà de la plage.
    pub async fn voyages_eleves_en_sortie(
        &self,
        etablissement_id: Uuid,
        du: DateTime<Utc>,
        au: DateTime<Utc>,
    ) -> Result<Vec<EnSortie>, EntDbError> {
        let mut t = TenantTx::begin(self.pool(), etablissement_id)
            .await
            .map_err(db)?;
        let v = sqlx::query_as::<_, EnSortie>(
            "SELECT sp.eleve_id, s.id AS sortie_id, s.debut, s.fin
               FROM ent_sortie s
               JOIN ent_sortie_participant sp ON sp.etablissement_id = s.etablissement_id AND sp.sortie_id = s.id
              WHERE s.etablissement_id = $1
                AND s.debut < $3 AND s.fin > $2
                AND ((s.etat = 'prete' AND sp.etat IN ('inscrit','pret'))
                  OR (s.etat = 'en_cours' AND sp.etat = 'parti'))
              ORDER BY s.debut, sp.eleve_id",
        )
        .bind(etablissement_id)
        .bind(du)
        .bind(au)
        .fetch_all(&mut *t)
        .await
        .map_err(db)?;
        t.commit().await.map_err(db)?;
        Ok(v)
    }
}
