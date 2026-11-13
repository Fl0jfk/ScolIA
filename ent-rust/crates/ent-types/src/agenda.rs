//! Types JSON des routes `/agenda/*` (socle 19, lot AG1 : événements, vue agenda calculée).
//!
//! Le JSON est celui qu'`ent-db` / `ent-api` produisent (mêmes types, partagés). États en chaînes
//! (`"brouillon"`, `"publie"`, `"annule"`, `"clos"`). Aucune règle ici : le serveur décide (droits,
//! audience sous les droits de l'auteur, minimisation R-AG5). Le frontend ne déduit rien.

use chrono::{DateTime, NaiveDate, NaiveTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Types d'événements intégrés : (code, libellé, famille).
pub const TYPES_INTEGRES: &[(&str, &str, &str)] = &[
    ("reunion", "Réunion", "reunion"),
    ("reunion_parents", "Réunion avec les familles", "reunion"),
    ("formation", "Formation", "reunion"),
    ("evenement", "Événement", "evenement"),
    (
        "feuille_semaine",
        "Feuille de semaine de l'établissement",
        "evenement",
    ),
    ("manifestation", "Manifestation", "manifestation"),
    ("journee_banalisee", "Journée banalisée", "journee"),
];

/// Sources de la vue agenda (R-AG1 : l'agenda lit les sources, il ne les recopie pas).
pub const SOURCES: &[&str] = &[
    "evenements",
    "rendez_vous",
    "edt",
    "calendrier",
    "sorties",
    "semaine_etablissement",
    "echeances",
    "partages",
];

/// Type d'événement de la feuille de semaine de l'établissement (calque `semaine_etablissement`).
pub const TYPE_FEUILLE_SEMAINE: &str = "feuille_semaine";

/// Saisie d'un créneau de la feuille de semaine (direction) : toujours publié, visible de tout
/// l'établissement ; les `notes` ne sont lisibles que de leur auteur.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CreneauSemaine {
    pub jour: NaiveDate,
    pub titre: String,
    #[serde(default)]
    pub heure_debut: Option<NaiveTime>,
    #[serde(default)]
    pub heure_fin: Option<NaiveTime>,
    #[serde(default)]
    pub lieu: Option<String>,
    #[serde(default)]
    pub notes: Option<String>,
}

/// Un élément de la vue agenda, **déjà minimisé pour son lecteur** (R-AG5).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ElementAgenda {
    /// Source (`evenements`, `rendez_vous`, `edt`, `calendrier`, `sorties`).
    pub source: String,
    /// Identifiant dans la source (`evenement`, créneau…) : sert à ouvrir le détail.
    pub id: Option<Uuid>,
    pub titre: String,
    pub debut: DateTime<Utc>,
    pub fin: DateTime<Utc>,
    /// Jour et heures **dans le fuseau de l'établissement** (calculés par le serveur : l'écran n'a
    /// aucun fuseau à deviner). `heure_*` absentes pour une journée entière.
    pub jour: NaiveDate,
    /// Dernier jour (inclus) d'un élément sur plusieurs jours ; égal à `jour` sinon.
    pub jour_fin: NaiveDate,
    #[serde(default)]
    pub heure_debut: Option<NaiveTime>,
    #[serde(default)]
    pub heure_fin: Option<NaiveTime>,
    pub journee_entiere: bool,
    pub lieu: Option<String>,
    /// `publie`, `brouillon`, `annule`, `clos` (événements) ; `prevue`, `annulee`… (séances).
    pub statut: String,
    /// Type d'événement ou nature de l'élément.
    pub nature: String,
    /// Rôle du lecteur : `organisateur`, `invite`, `famille`, `lecteur`.
    pub role: Option<String>,
    /// Réponse du lecteur à l'invitation (`en_attente`, `oui`, `non`, `peut_etre`).
    pub reponse: Option<String>,
    /// Enfant concerné (vue famille) : jamais pour les personnels.
    pub eleve_id: Option<Uuid>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SourceIndisponible {
    pub source: String,
    pub raison: String,
}

/// Vue agenda d'une plage (≤ 42 jours). Une source absente est **dite** (jamais un silence).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct VueAgenda {
    pub du: NaiveDate,
    pub au: NaiveDate,
    pub elements: Vec<ElementAgenda>,
    pub sources_indisponibles: Vec<SourceIndisponible>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NouveauCoupon {
    pub question: String,
    pub choix: Vec<String>,
}

/// Saisie d'un événement. Heure murale (jour + heures) : le serveur la convertit avec le fuseau de
/// l'établissement. Sans heures : journée entière (de `jour` à `jour_fin` inclus).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NouvelEvenement {
    pub type_code: String,
    pub titre: String,
    #[serde(default)]
    pub description: Option<String>,
    pub jour: NaiveDate,
    #[serde(default)]
    pub jour_fin: Option<NaiveDate>,
    #[serde(default)]
    pub heure_debut: Option<NaiveTime>,
    #[serde(default)]
    pub heure_fin: Option<NaiveTime>,
    #[serde(default)]
    pub salle_id: Option<Uuid>,
    #[serde(default)]
    pub lieu_texte: Option<String>,
    /// `etablissement`, `participants` (défaut) ou `restreint`.
    #[serde(default)]
    pub confidentialite: Option<String>,
    /// Comptes du personnel invités nominativement.
    #[serde(default)]
    pub invites: Vec<Uuid>,
    #[serde(default)]
    pub coupon: Option<NouveauCoupon>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NouvelleSerie {
    /// Le premier jour de la série est `modele.jour`.
    pub modele: NouvelEvenement,
    /// 1 = lundi … 7 = dimanche.
    pub jours_semaine: Vec<i16>,
    #[serde(default)]
    pub intervalle_semaines: Option<i16>,
    pub au: NaiveDate,
}

/// Modification d'un événement. Les champs absents ne changent pas. `portee` : `occurrence`
/// (défaut), `suivantes` ou `toutes` (séries). `version` : celle lue (conflit → 409).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ModifEvenement {
    pub version: i32,
    #[serde(default)]
    pub portee: Option<String>,
    #[serde(default)]
    pub titre: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub jour: Option<NaiveDate>,
    #[serde(default)]
    pub heure_debut: Option<NaiveTime>,
    #[serde(default)]
    pub heure_fin: Option<NaiveTime>,
    #[serde(default)]
    pub salle_id: Option<Uuid>,
    #[serde(default)]
    pub lieu_texte: Option<String>,
    #[serde(default)]
    pub confidentialite: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AudienceDto {
    /// `classe`, `niveau`, `groupe`, `etablissement` (familles) ou `tous_personnels`.
    pub kind: String,
    #[serde(default)]
    pub cible_id: Option<Uuid>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ApercuPublication {
    pub nb_familles: usize,
    pub nb_responsables: usize,
    pub nb_personnels: usize,
    /// Élèves de l'audience que l'auteur n'a pas le droit d'atteindre (R-AG8) : exclus.
    pub exclus_hors_droits: usize,
    /// Empreinte du calcul : à renvoyer pour confirmer (l'audience peut changer entre-temps).
    pub empreinte: String,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct DemandePublication {
    pub audiences: Vec<AudienceDto>,
    /// Série : `occurrence` (défaut), `suivantes` ou `toutes` (occurrences encore en brouillon).
    #[serde(default)]
    pub portee: Option<String>,
    #[serde(default)]
    pub empreinte: Option<String>,
    /// Vrai pour obtenir seulement l'aperçu (rien n'est écrit).
    #[serde(default)]
    pub apercu_seulement: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ParticipantDto {
    pub user_id: Uuid,
    pub nom: Option<String>,
    pub role: String,
    pub obligatoire: bool,
    pub reponse: String,
    pub motif_excuse: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CouponDto {
    pub question: String,
    pub choix: Vec<String>,
    /// Décompte (organisateur seulement) : une entrée par choix.
    #[serde(default)]
    pub decompte: Option<Vec<i64>>,
    /// Choix du lecteur (1..3), s'il a répondu.
    #[serde(default)]
    pub mon_choix: Option<i16>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EvenementDetail {
    pub id: Uuid,
    pub type_code: String,
    pub titre: String,
    pub description: Option<String>,
    pub debut: DateTime<Utc>,
    pub fin: DateTime<Utc>,
    /// Jour et heures dans le fuseau de l'établissement (calculés par le serveur).
    pub jour: NaiveDate,
    pub jour_fin: NaiveDate,
    #[serde(default)]
    pub heure_debut: Option<NaiveTime>,
    #[serde(default)]
    pub heure_fin: Option<NaiveTime>,
    pub journee_entiere: bool,
    pub lieu: Option<String>,
    pub confidentialite: String,
    pub statut: String,
    pub motif_annulation: Option<String>,
    pub serie_id: Option<Uuid>,
    pub occurrence_detachee: bool,
    pub version: i32,
    pub organisateur_user_id: Uuid,
    pub organisateur_nom: Option<String>,
    /// Le lecteur est l'organisateur (boutons de gestion).
    pub je_suis_organisateur: bool,
    pub ma_reponse: Option<String>,
    /// Liste complète : organisateur seulement. Les invités ne voient que les décomptes.
    #[serde(default)]
    pub participants: Option<Vec<ParticipantDto>>,
    pub nb_oui: i64,
    pub nb_non: i64,
    pub nb_peut_etre: i64,
    pub nb_en_attente: i64,
    #[serde(default)]
    pub coupon: Option<CouponDto>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ReponseInvitation {
    /// `oui`, `non` ou `peut_etre`.
    pub reponse: String,
    #[serde(default)]
    pub motif_excuse: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ReponseCoupon {
    /// 1, 2 ou 3.
    pub choix: i16,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AnnulationEvenement {
    pub motif: String,
    #[serde(default)]
    pub portee: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FaitJournal {
    pub action: String,
    pub par_user_id: Option<Uuid>,
    pub le: DateTime<Utc>,
    pub avant: Option<String>,
    pub apres: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EvenementCree {
    pub id: Uuid,
    pub serie_id: Option<Uuid>,
    pub nb_occurrences: usize,
}

// ───────────────────────────── Réservation de lieux (AG3) et disponibilités (AG2) ─────────────────────────────

/// Mode de réservation d'une salle (D-058) : réglage **par salle**, salles de classe comprises.
pub const MODES_LIEU: [&str; 3] = ["toujours", "sur_validation", "jamais"];

/// Recherche d'une salle libre sur un créneau (jour et heures murales de l'établissement).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RechercheSalle {
    pub jour: NaiveDate,
    pub heure_debut: NaiveTime,
    pub heure_fin: NaiveTime,
    #[serde(default)]
    pub capacite_min: Option<i32>,
    #[serde(default)]
    pub type_salle: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SalleDispo {
    pub salle_id: Uuid,
    pub code: String,
    pub libelle: String,
    pub capacite: Option<i32>,
    pub type_salle: String,
    pub site: Option<String>,
    /// `toujours` ou `sur_validation` (la demande est alors soumise au valideur).
    pub mode: String,
    /// Pourquoi une salle de classe est libre (« cours de 4B supprimé »), jamais le motif d'absence.
    pub raison_libre: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SerieReservation {
    /// 1 = lundi … 7 = dimanche.
    pub jours_semaine: Vec<i16>,
    #[serde(default)]
    pub intervalle_semaines: Option<i16>,
    pub au: NaiveDate,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NouvelleReservation {
    pub salle_id: Uuid,
    pub jour: NaiveDate,
    pub heure_debut: NaiveTime,
    pub heure_fin: NaiveTime,
    pub intitule: String,
    /// `enseignement`, `reunion`, `evenement` ou `autre` (défaut).
    #[serde(default)]
    pub motif_categorie: Option<String>,
    #[serde(default)]
    pub effectif: Option<i32>,
    #[serde(default)]
    pub mise_en_place: Option<String>,
    /// Lisible du demandeur, du bénéficiaire et du gestionnaire seulement.
    #[serde(default)]
    pub commentaire: Option<String>,
    /// Réservation pour un collègue (il est prévenu).
    #[serde(default)]
    pub pour_user_id: Option<Uuid>,
    #[serde(default)]
    pub serie: Option<SerieReservation>,
    /// Série : accepter d'écrire les occurrences libres quand d'autres sont en conflit.
    #[serde(default)]
    pub confirmer_partiel: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct OccurrenceReservation {
    pub jour: NaiveDate,
    pub reservation_id: Option<Uuid>,
    /// `confirmee` ou `demandee` quand écrite.
    pub statut: Option<String>,
    /// Pourquoi l'occurrence n'est pas possible (qui occupe, hors horizon…).
    pub conflit: Option<String>,
}

/// Rapport **par occurrence** (ND-02) : rien n'est perdu en silence.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RapportReservation {
    /// Faux quand une série en conflit attend la confirmation de l'auteur : rien n'a été écrit.
    pub ecrit: bool,
    pub occurrences: Vec<OccurrenceReservation>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ReservationVue {
    pub id: Uuid,
    pub salle_id: Uuid,
    pub salle_libelle: String,
    pub jour: NaiveDate,
    pub heure_debut: NaiveTime,
    pub heure_fin: NaiveTime,
    pub intitule: String,
    pub motif_categorie: String,
    pub demandeur_nom: Option<String>,
    pub pour_nom: Option<String>,
    pub statut: String,
    pub effectif: Option<i32>,
    pub mise_en_place: Option<String>,
    /// Absent pour qui n'est ni demandeur, ni bénéficiaire, ni gestionnaire.
    #[serde(default)]
    pub commentaire: Option<String>,
    pub je_suis_demandeur: bool,
    pub serie_id: Option<Uuid>,
    pub motif_decision: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CoursSalleVue {
    pub jour: NaiveDate,
    pub heure_debut: NaiveTime,
    pub heure_fin: NaiveTime,
    /// Classe seulement : ni intervenant, ni motif.
    pub classe: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LibereeVue {
    pub jour: NaiveDate,
    pub heure_debut: NaiveTime,
    pub heure_fin: NaiveTime,
    /// « cours de 4B supprimé » : jamais le motif d'absence du professeur.
    pub raison: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PlanningSalle {
    pub salle_id: Uuid,
    pub salle_libelle: String,
    pub du: NaiveDate,
    pub au: NaiveDate,
    pub reservations: Vec<ReservationVue>,
    pub cours: Vec<CoursSalleVue>,
    pub liberees: Vec<LibereeVue>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct AnnulationReservation {
    /// Obligatoire pour le gestionnaire qui annule la réservation d'un autre.
    #[serde(default)]
    pub motif: Option<String>,
    /// `occurrence` (défaut) ou `serie` (les occurrences à venir).
    #[serde(default)]
    pub portee: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DecisionReservation {
    pub accepter: bool,
    /// Obligatoire pour un refus.
    #[serde(default)]
    pub motif: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RegleLieu {
    pub mode: String,
    #[serde(default)]
    pub valideur_user_id: Option<Uuid>,
    pub delai_min_heures: i32,
    pub duree_max_minutes: i32,
    pub horizon_jours: i32,
}

impl Default for RegleLieu {
    fn default() -> Self {
        Self {
            mode: "toujours".into(),
            valideur_user_id: None,
            delai_min_heures: 0,
            duree_max_minutes: 600,
            horizon_jours: 56,
        }
    }
}

/// Disponibilité d'une personne sur un créneau (AG2) : libre / occupé, **jamais** le motif (R-AG6).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DisponibilitePersonne {
    pub user_id: Uuid,
    pub nom: Option<String>,
    /// `libre`, `occupe` ou `non_communique` (santé, social : occupation protégée, R-AG37).
    pub etat: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PlageCommune {
    pub jour: NaiveDate,
    pub debut: NaiveTime,
    pub fin: NaiveTime,
}
