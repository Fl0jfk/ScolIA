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
    ("manifestation", "Manifestation", "manifestation"),
    ("journee_banalisee", "Journée banalisée", "journee"),
];

/// Sources de la vue agenda (R-AG1 : l'agenda lit les sources, il ne les recopie pas).
pub const SOURCES: &[&str] = &["evenements", "rendez_vous", "edt", "calendrier", "sorties"];

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
