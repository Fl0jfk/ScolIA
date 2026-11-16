//! Cahier de textes, lot C3 : agenda, cahier de l'élève et de la famille, « ce que j'ai manqué », coche, résumé
//! hebdomadaire, abonnement au calendrier.
//!
//! Contrat JSON des routes `/cahier/c3/*` d'`ent-api`. Le serveur donne des **jours et heures de l'établissement** :
//! le client ne convertit aucun instant UTC. Rien ici ne porte un identifiant de compte, une copie, un dépôt, ni le motif
//! d'un aménagement ; la coche « fait » n'est jamais servie à un responsable (R-C14).

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Genre d'une ligne de l'agenda.
pub const GENRE_TRAVAIL: &str = "travail";
pub const GENRE_EVALUATION: &str = "evaluation";
pub const GENRE_EPREUVE: &str = "epreuve";
pub const GENRE_SORTIE: &str = "sortie";

/// Statuts **constatés** d'un travail à rendre (R-C12 : un fait, jamais un zéro).
pub const STATUT_A_FAIRE: &str = "a_faire";
pub const STATUT_RENDU: &str = "rendu";
pub const STATUT_EN_RETARD: &str = "en_retard";
pub const STATUT_NON_RENDU: &str = "non_rendu";
pub const STATUT_DISPENSE: &str = "dispense";
pub const STATUT_EXCUSE: &str = "excuse";
pub const STATUT_RETOUR: &str = "retour_disponible";

/// Une ligne de l'agenda : un travail, une évaluation annoncée, une épreuve ou une sortie.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgendaLigneDto {
    pub genre: String,
    pub id: Uuid,
    pub cours_id: Option<Uuid>,
    pub matiere: String,
    pub titre: String,
    /// Libellé de la nature (« Exercices », « Leçon »…), pour un travail.
    pub nature: Option<String>,
    /// Consigne ; vide pour les évaluations, épreuves et sorties.
    pub consigne: Option<String>,
    /// Jour et heure **de l'établissement** de l'échéance effective (la prolongation individuelle fait foi).
    pub jour: NaiveDate,
    pub heure: Option<String>,
    pub duree_estimee_min: Option<i16>,
    /// `aucun`, `en_ligne` ou `en_classe`.
    pub rendu: Option<String>,
    /// Nom d'affichage de l'intervenant (jamais un identifiant de compte).
    pub auteur: Option<String>,
    /// Statut constaté ; absent quand aucune remise n'est attendue.
    pub statut: Option<String>,
    /// Coche « fait » : **élève seulement**, jamais pour un responsable (R-C14).
    pub fait: Option<bool>,
    /// L'échéance de cet élève a été prolongée (sans en dire le motif).
    pub echeance_prolongee: bool,
    /// Le travail a été modifié depuis sa publication : « modifié le … » (R-C29, jamais caché).
    pub modifie_le: Option<DateTime<Utc>>,
    pub version: i32,
}

/// Lecteur de l'agenda : l'élève lui-même ou un responsable habilité.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgendaDto {
    pub eleve_id: Uuid,
    pub du: NaiveDate,
    pub au: NaiveDate,
    /// `eleve` ou `famille`.
    pub vue: String,
    /// Trié par jour, puis heure, puis genre.
    pub lignes: Vec<AgendaLigneDto>,
}

/// Un travail donné pendant une séance manquée.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManqueTravailDto {
    pub id: Uuid,
    pub titre: String,
    pub consigne: String,
    pub echeance_jour: NaiveDate,
    pub echeance_heure: Option<String>,
}

/// « Ce que j'ai manqué » : le contenu d'une séance où l'élève était absent (CDT-162).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManqueSeanceDto {
    pub seance_id: Uuid,
    pub jour: NaiveDate,
    pub heure_debut: String,
    pub heure_fin: String,
    pub matiere: String,
    pub auteur: Option<String>,
    pub titre: Option<String>,
    pub contenu: Option<String>,
    pub notions: Vec<String>,
    pub travaux: Vec<ManqueTravailDto>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManqueDto {
    pub eleve_id: Uuid,
    pub du: NaiveDate,
    pub au: NaiveDate,
    pub seances: Vec<ManqueSeanceDto>,
}

/// Corps de `PUT /cahier/c3/travaux/{id}/fait`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CocheBody {
    pub fait: bool,
}

/// Résumé de la semaine (CDT-180) : décomptes et lignes de la semaine, sans coche pour une famille.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResumeSemaineDto {
    pub eleve_id: Uuid,
    /// Lundi de la semaine.
    pub semaine_debut: NaiveDate,
    pub semaine_fin: NaiveDate,
    pub nb_travaux: i32,
    pub nb_evaluations: i32,
    pub nb_autres: i32,
    pub duree_estimee_min: i32,
    pub lignes: Vec<AgendaLigneDto>,
}

/// Abonnement au calendrier de l'élève (CDT-176).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AbonnementDto {
    pub id: Uuid,
    pub eleve_id: Uuid,
    pub cree_le: DateTime<Utc>,
    /// Jour de l'établissement de la création, lisible tel quel.
    pub cree_jour: NaiveDate,
}

/// Création : l'adresse complète n'est donnée **qu'une fois** (le jeton n'est pas relisible).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AbonnementCreeDto {
    pub abonnement: AbonnementDto,
    /// Chemin relatif à servir au calendrier (`/cahier/c3/calendrier/{jeton}`).
    pub chemin: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AbonnementBody {
    /// Élève concerné ; omis pour un élève (le sien).
    pub eleve_id: Option<Uuid>,
}

/// Un avis déclaré par le cahier de textes (contrat d'avis du socle 8, déclaratif tant que le consommateur n'existe pas).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct AvisDeclare {
    /// Événement de l'outbox qui déclenche l'avis.
    pub evenement: &'static str,
    /// Catégorie de préférence de l'utilisateur (« Notes et devoirs »).
    pub categorie: &'static str,
    /// Qui reçoit : `eleves_du_cours`, `responsables_habilites`.
    pub destinataires: &'static str,
    /// Gabarit : aucun contenu de consigne, jamais de note, jamais de motif.
    pub gabarit: &'static str,
    /// Peut être désactivé par l'utilisateur.
    pub desactivable: bool,
}
