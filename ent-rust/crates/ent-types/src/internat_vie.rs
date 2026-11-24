//! Types JSON des routes `/internat/*` du socle 41, lot I4 « Vie quotidienne » : sorties de groupe, liaison codée,
//! événements de nuit, temps du soir (études, activités), préoccupation.
//!
//! Aucune règle ici : le serveur décide (droits, périmètre, qualité de titulaire, interdictions, désaccords).
//! Les états et les codes restent des chaînes. **Aucune donnée de santé** (R-IT6) ; un élève protégé (socle 2, E7)
//! n'est jamais nommé à un tiers ; une famille ne voit jamais un autre élève (R-IT32).

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ───────────────────────── Sorties de groupe ─────────────────────────

/// Création d'une sortie de groupe (brouillon). Les accompagnateurs sont des comptes du personnel (R-IT25).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SortieGroupeSaisieDto {
    pub titre: String,
    pub lieu: String,
    pub debut: DateTime<Utc>,
    pub fin: DateTime<Utc>,
    #[serde(default)]
    pub encadrants: Vec<Uuid>,
    #[serde(default)]
    pub eleves: Vec<Uuid>,
}

/// Élève écarté d'une invitation (jamais en silence), avec le motif codé : `plus_interne`, `inconnu`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EcarteDto {
    pub eleve_id: Uuid,
    pub motif: String,
}

/// Un participant tel que le personnel le voit. `nom` et `prenom` sont absents pour un élève protégé.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ParticipantGroupeDto {
    pub eleve_id: Uuid,
    #[serde(default)]
    pub nom: Option<String>,
    #[serde(default)]
    pub prenom: Option<String>,
    pub protege: bool,
    /// `invite`, `en_attente`, `autorise`, `refuse`, `non_autorise`, `parti`, `rentre`, `retire`.
    pub etat: String,
    /// `titulaire`, `eleve_majeur`, `annuelle`, `hors_ligne`.
    #[serde(default)]
    pub reponse_qualite: Option<String>,
    #[serde(default)]
    pub reponse_le: Option<DateTime<Utc>>,
    /// Code du motif d'un refus ou d'un blocage ; jamais « interdiction » détaillée à la famille (voir le contrat).
    #[serde(default)]
    pub motif: Option<String>,
    #[serde(default)]
    pub parti_le: Option<DateTime<Utc>>,
    #[serde(default)]
    pub rentre_le: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SortieGroupeDto {
    pub id: Uuid,
    pub titre: String,
    pub lieu: String,
    pub debut: DateTime<Utc>,
    pub fin: DateTime<Utc>,
    /// `brouillon`, `soumise`, `acceptee`, `refusee`, `annulee`, `close`.
    pub statut: String,
    pub cree_par_user_id: Uuid,
    #[serde(default)]
    pub decide_par_user_id: Option<Uuid>,
    #[serde(default)]
    pub decide_le: Option<DateTime<Utc>>,
    #[serde(default)]
    pub decision_motif_code: Option<String>,
    #[serde(default)]
    pub annule_motif_code: Option<String>,
    #[serde(default)]
    pub bilan_code: Option<String>,
    #[serde(default)]
    pub modele_id: Option<Uuid>,
    pub encadrants: Vec<Uuid>,
    pub participants: Vec<ParticipantGroupeDto>,
    /// Élèves écartés à la création ou à l'ajout (seulement dans la réponse à cet appel).
    #[serde(default)]
    pub ecartes: Vec<EcarteDto>,
}

/// Ligne de liste (sans participants nominatifs).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SortieGroupeResumeDto {
    pub id: Uuid,
    pub titre: String,
    pub lieu: String,
    pub debut: DateTime<Utc>,
    pub fin: DateTime<Utc>,
    pub statut: String,
    pub nb_participants: i64,
    pub nb_autorises: i64,
    pub nb_en_attente: i64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ElevesAjoutDto {
    pub eleves: Vec<Uuid>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EncadrantAjoutDto {
    pub user_id: Uuid,
}

/// Décision de la direction : `accepte` ; un refus porte un motif codé (`securite`, `encadrement`, `calendrier`,
/// `reglement`, `autre`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DecisionGroupeDto {
    pub accepte: bool,
    #[serde(default)]
    pub motif_code: Option<String>,
}

/// Annulation : motif codé (`meteo`, `encadrement`, `organisateur`, `securite`, `autre`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AnnulationGroupeDto {
    pub motif_code: String,
}

/// Réponse d'un titulaire (ou de l'élève majeur) pour UN élève.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ReponseGroupeDto {
    pub accord: bool,
}

/// Accord reçu hors ligne (R-IT24) : la personne qui l'a donné (un titulaire), la date de réception, le canal
/// (`telephone`, `papier`, `en_personne`) ; la personne qui l'a reçu est le compte appelant.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AccordHorsLigneDto {
    pub donne_par_personne_id: Uuid,
    /// Vrai (par défaut) : accord reçu hors ligne. Faux : **refus** reçu hors ligne (téléphone, papier) ; il se saisit
    /// comme un accord, avec la personne, la date et le canal (règle générale 5 : tout fait se corrige par un fait).
    #[serde(default = "vrai")]
    pub accord: bool,
    pub recu_le: DateTime<Utc>,
    pub canal: String,
    #[serde(default)]
    pub piece_id: Option<Uuid>,
}

/// Appel de départ ou de retour : les élèves effectivement pointés.
fn vrai() -> bool {
    true
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct AppelGroupeDto {
    pub eleves: Vec<Uuid>,
    /// Départ seulement : accompagnateurs pointés présents (l'acteur, s'il est accompagnateur, l'est d'office). Au
    /// moins un accompagnateur présent est exigé ; l'encadrement se compte sur les présents.
    #[serde(default)]
    pub encadrants_presents: Vec<Uuid>,
}

/// Correction d'un retour pointé par erreur : un fait motivé remet l'élève « parti » (règle générale 5).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AnnulationRetourDto {
    pub eleve_id: Uuid,
    /// `erreur_pointage`, `retour_non_effectif`, `autre`.
    pub motif_code: String,
}

/// Remplacement d'un accompagnateur pendant la sortie (fait tracé).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RemplacementEncadrantDto {
    pub ancien_user_id: Uuid,
    pub nouveau_user_id: Uuid,
}

/// Résultat par élève d'un appel de départ : `ok` ou le motif codé du refus (`non_autorise`, `accord_tombe`,
/// `pas_invite`, `deja_parti`, `pas_parti`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ResultatAppelDto {
    pub eleve_id: Uuid,
    pub resultat: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ClotureGroupeDto {
    /// `sans_incident`, `retard`, `incident`.
    pub bilan_code: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DuplicationGroupeDto {
    pub debut: DateTime<Utc>,
    pub fin: DateTime<Utc>,
}

/// Ce que voit une famille (ou l'élève lui-même) d'une sortie de groupe : seulement SON enfant, jamais un autre
/// participant, jamais le nombre d'inscrits.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SortieGroupeFamilleDto {
    pub sortie_id: Uuid,
    pub eleve_id: Uuid,
    pub titre: String,
    pub lieu: String,
    pub debut: DateTime<Utc>,
    pub fin: DateTime<Utc>,
    pub statut: String,
    /// `en_attente` : une réponse est attendue ; `autorise` ; `refuse` ; `non_autorise` (sans cause) ; `parti` ; `rentre`.
    pub etat: String,
    pub peut_repondre: bool,
}

// ───────────────────────── Liaison codée ─────────────────────────

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LiaisonCodeDto {
    pub code: String,
    pub libelle: String,
    pub pour_le_jour: bool,
    pub valorisation: bool,
    pub ordre: i16,
    pub actif: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct LiaisonCodeSaisieDto {
    #[serde(default)]
    pub libelle: Option<String>,
    #[serde(default)]
    pub pour_le_jour: Option<bool>,
    #[serde(default)]
    pub actif: Option<bool>,
    #[serde(default)]
    pub ordre: Option<i16>,
}

/// Création d'un code propre à l'établissement (INT-247) ; jamais une catégorie de santé.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LiaisonCodeCreationDto {
    pub code: String,
    pub libelle: String,
    #[serde(default)]
    pub pour_le_jour: bool,
}

/// Écriture d'un fait de liaison. `nuit` : date du soir (aujourd'hui par défaut, jamais future).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LiaisonSaisieDto {
    pub eleve_id: Uuid,
    pub code: String,
    #[serde(default)]
    pub texte: Option<String>,
    #[serde(default)]
    pub nuit: Option<NaiveDate>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LiaisonDto {
    pub id: Uuid,
    pub eleve_id: Uuid,
    #[serde(default)]
    pub nom: Option<String>,
    #[serde(default)]
    pub prenom: Option<String>,
    pub protege: bool,
    pub code: String,
    pub libelle: String,
    /// Absent pour la vie scolaire de jour (qui ne voit jamais le texte libre).
    #[serde(default)]
    pub texte: Option<String>,
    pub nuit: NaiveDate,
    pub par_user_id: Uuid,
    pub pour_le_jour: bool,
    pub masque: bool,
    pub a_revoir: bool,
    pub signale_pp: bool,
    pub cree_le: DateTime<Utc>,
    /// Posé à l'écriture seulement : rappel de ne saisir aucune information de santé (F12).
    #[serde(default)]
    pub avertissement: Option<String>,
}

/// Geste du CPE sur un fait : `masquer` (motif obligatoire), `a_revoir`, `lever_a_revoir`, `signaler_pp`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LiaisonGesteDto {
    pub geste: String,
    #[serde(default)]
    pub motif_code: Option<String>,
}

// ───────────────────────── Événements de nuit ─────────────────────────

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ParticipantEvenementSaisieDto {
    #[serde(default)]
    pub eleve_id: Option<Uuid>,
    #[serde(default)]
    pub user_id: Option<Uuid>,
    /// `auteur_presume`, `victime`, `temoin`, `present` (un adulte n'est que `temoin` ou `present`).
    pub role: String,
}

/// Déclaration d'un événement. `nature` : `incident`, `comportement`, `intrusion`, `alarme`, `malaise`,
/// `degradation`, `objet_dangereux`. `niveau` : `information`, `important`, `critique`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EvenementSaisieDto {
    pub nature: String,
    #[serde(default)]
    pub niveau: Option<String>,
    #[serde(default)]
    pub a: Option<DateTime<Utc>>,
    #[serde(default)]
    pub zone_id: Option<Uuid>,
    #[serde(default)]
    pub chambre_id: Option<Uuid>,
    #[serde(default)]
    pub mesures_code: Option<String>,
    #[serde(default)]
    pub participants: Vec<ParticipantEvenementSaisieDto>,
    #[serde(default)]
    pub complement: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ParticipantEvenementDto {
    #[serde(default)]
    pub eleve_id: Option<Uuid>,
    #[serde(default)]
    pub nom: Option<String>,
    #[serde(default)]
    pub prenom: Option<String>,
    pub protege: bool,
    #[serde(default)]
    pub user_id: Option<Uuid>,
    pub role: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ComplementDto {
    pub id: Uuid,
    pub texte: String,
    pub par_user_id: Uuid,
    pub cree_le: DateTime<Utc>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EvenementDto {
    pub id: Uuid,
    pub nature: String,
    pub niveau: String,
    pub a: DateTime<Utc>,
    pub nuit: NaiveDate,
    #[serde(default)]
    pub zone_id: Option<Uuid>,
    #[serde(default)]
    pub chambre_id: Option<Uuid>,
    #[serde(default)]
    pub mesures_code: Option<String>,
    pub declare_par_user_id: Uuid,
    #[serde(default)]
    pub transmis_13_le: Option<DateTime<Utc>>,
    #[serde(default)]
    pub intervention_demandee_le: Option<DateTime<Utc>>,
    pub participants: Vec<ParticipantEvenementDto>,
    pub complements: Vec<ComplementDto>,
    /// Signaux codés émis vers le mouvement du socle santé (malaise : un par élève concerné) ; accusé de l'émission.
    #[serde(default)]
    pub signaux_sante: i64,
    /// Accusé minimal : l'acteur n'a pas le droit de lire cet événement (ou il sort de sa portée) ; ni participants ni
    /// compléments ne sont rendus (M4).
    #[serde(default)]
    pub accuse_minimal: bool,
    #[serde(default)]
    pub avertissement: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ComplementSaisieDto {
    pub texte: String,
}

// ───────────────────────── Temps du soir ─────────────────────────

/// Création d'une étude ou d'une activité du soir. `nature` : `etude`, `activite`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TempsSaisieDto {
    pub nature: String,
    pub libelle: String,
    #[serde(default)]
    pub lieu: Option<String>,
    pub debut: DateTime<Utc>,
    pub fin: DateTime<Utc>,
    #[serde(default)]
    pub encadrant_user_id: Option<Uuid>,
    #[serde(default)]
    pub places: Option<i16>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TempsDto {
    pub id: Uuid,
    pub nature: String,
    pub libelle: String,
    #[serde(default)]
    pub lieu: Option<String>,
    pub debut: DateTime<Utc>,
    pub fin: DateTime<Utc>,
    #[serde(default)]
    pub encadrant_user_id: Option<Uuid>,
    #[serde(default)]
    pub places: Option<i16>,
    pub statut: String,
    /// Jamais donné à un élève (F7) : ni le nombre d'inscrits ni les places.
    #[serde(default)]
    pub nb_inscrits: Option<i64>,
    /// Seulement pour le personnel : les inscrits (jamais montrés à une famille).
    #[serde(default)]
    pub inscrits: Vec<Uuid>,
    /// Pointage des inscrits (personnel seulement).
    #[serde(default)]
    pub pointages: Vec<PointageInscritDto>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PointageInscritDto {
    pub eleve_id: Uuid,
    /// `present`, `absent` ou absent du JSON (pas encore pointé).
    #[serde(default)]
    pub pointage: Option<String>,
    #[serde(default)]
    pub rentre_le: Option<DateTime<Utc>>,
    /// Non pointé au-delà du délai : signalé à l'équipe.
    pub signale: bool,
    /// Élève protégé : il est compté et suivi (état, retour) mais présenté comme « élève protégé », sans nom (règle générale
    /// 6, M7 : l'accompagnateur compte ses élèves).
    #[serde(default)]
    pub protege: bool,
}

/// Pointage d'un inscrit à une étude ou une activité du soir.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PointageTempsDto {
    pub eleve_id: Uuid,
    /// `present`, `absent` ou `rentre` (activité seulement : retour à l'internat).
    pub etat: String,
}

/// Réglages de vie quotidienne de l'établissement.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ReglagesVieDto {
    pub groupe_eleves_par_encadrant: i16,
    pub groupe_preavis_heures: i16,
    /// Tolérance après l'heure de fin prévue avant l'ouverture d'un cas de nuit (défaut prudent : 15 min).
    pub groupe_tolerance_retour_minutes: i16,
    /// Délai après le début d'une étude au-delà duquel un inscrit non pointé est signalé (défaut : 15 min).
    pub etude_delai_absence_minutes: i16,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ReglagesVieSaisieDto {
    #[serde(default)]
    pub groupe_eleves_par_encadrant: Option<i16>,
    #[serde(default)]
    pub groupe_preavis_heures: Option<i16>,
    #[serde(default)]
    pub groupe_tolerance_retour_minutes: Option<i16>,
    #[serde(default)]
    pub etude_delai_absence_minutes: Option<i16>,
}

/// Ce qu'un cas de nuit doit à une sortie ou à une activité non rentrée (règle générale 1).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CasOrigineDto {
    #[serde(default)]
    pub sortie_groupe_id: Option<Uuid>,
    #[serde(default)]
    pub temps_id: Option<Uuid>,
    /// Heure de retour prévue (fin de la sortie ou du temps).
    pub heure_prevue: DateTime<Utc>,
    /// Comptes à joindre : accompagnateurs de la sortie, ou encadrant du temps.
    pub a_joindre: Vec<Uuid>,
}

/// Avertissement affiché à chaque saisie de texte libre (F12) : la liste de mots n'est qu'une aide, pas une garantie.
pub const AVERTISSEMENT_TEXTE_LIBRE: &str =
    "Ne saisissez aucune information de santé ni aucun nom d'autre élève dans ce texte : la détection automatique n'est qu'une aide. Pour la santé, utilisez le mouvement du socle santé.";

/// Planning d'un élève (lu par lui, ses titulaires ou le personnel) : jamais le nom d'un autre élève.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PlanningEleveDto {
    pub eleve_id: Uuid,
    pub etudes_et_activites: Vec<PlanningTempsDto>,
    pub sorties_groupe: Vec<SortieGroupeFamilleDto>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PlanningTempsDto {
    pub temps_id: Uuid,
    pub nature: String,
    pub libelle: String,
    #[serde(default)]
    pub lieu: Option<String>,
    pub debut: DateTime<Utc>,
    pub fin: DateTime<Utc>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct InscriptionTempsDto {
    pub eleve_id: Uuid,
}

// ───────────────────────── Préoccupation ─────────────────────────

/// Ouverture du circuit du socle 36 : aucun contenu (R-IT29).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PreoccupationDto {
    pub eleve_id: Uuid,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PreoccupationAccuseDto {
    pub id: Uuid,
    pub a: DateTime<Utc>,
}

// ───────────────────────── Vigilance et référent (INT-257) ─────────────────────────

/// Pose d'une vigilance codée (`suivi_rapproche`, `premier_mois`, `accompagner_coucher`), sans texte. `du` : aujourd'hui
/// par défaut ; `au` : sans fin tant qu'elle n'est pas levée. Le référent est un compte du personnel.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct VigilanceSaisieDto {
    pub eleve_id: Uuid,
    pub code: String,
    #[serde(default)]
    pub referent_user_id: Option<Uuid>,
    #[serde(default)]
    pub du: Option<NaiveDate>,
    #[serde(default)]
    pub au: Option<NaiveDate>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct VigilanceDto {
    pub id: Uuid,
    pub eleve_id: Uuid,
    pub code: String,
    #[serde(default)]
    pub referent_user_id: Option<Uuid>,
    pub du: NaiveDate,
    #[serde(default)]
    pub au: Option<NaiveDate>,
    pub revue_le: DateTime<Utc>,
    /// Vrai quand la dernière revue date de plus de 7 jours : la vigilance attend sa revue hebdomadaire.
    pub a_reviser: bool,
    /// Vrai tant que la vigilance est en vigueur au jour de la lecture.
    pub active: bool,
}

// ───────────────────────── Libellés destinés aux familles ─────────────────────────

/// Texte **destiné aux familles** décrivant un fait du journal d'une sortie de groupe (suites 3, F6, D-069). Langage
/// courant, sans code interne ni jargon (« interdiction », « fait », « acte »), sans nom d'un autre élève. `None` : ce fait
/// ne s'affiche pas à une famille.
///
/// `refus_apres_depart` : le parent a refusé la sortie alors que l'élève était déjà parti avec le groupe. Le refus est
/// enregistré, mais il ne peut pas annuler un départ déjà fait ; l'établissement est prévenu et organise le retour. Le
/// texte ne promet pas d'heure de retour (l'établissement ne la connaît pas encore à cet instant).
pub fn libelle_fait_groupe_famille(acte: &str) -> Option<&'static str> {
    match acte {
        "refus_apres_depart" => Some(
            "Votre refus a bien été enregistré. Votre enfant était déjà parti avec le groupe : l'établissement a été prévenu \
             et demande son retour. En cas d'urgence, appelez directement l'établissement.",
        ),
        _ => None,
    }
}

#[cfg(test)]
mod tests_libelles_famille {
    use super::libelle_fait_groupe_famille;

    #[test]
    fn le_libelle_du_refus_apres_depart_est_clair_et_sans_jargon() {
        let t = libelle_fait_groupe_famille("refus_apres_depart").expect("libellé défini");
        assert!(t.contains("refus a bien été enregistré"), "{t}");
        assert!(t.contains("déjà parti"), "{t}");
        assert!(t.contains("retour"), "{t}");
        for jargon in ["refus_apres_depart", "interdiction", "acte", "fait_groupe", "_"] {
            assert!(!t.contains(jargon), "jargon « {jargon} » dans : {t}");
        }
        assert!(!t.contains("heure"), "aucune promesse d'heure : {t}");
    }

    #[test]
    fn un_acte_inconnu_ou_interne_n_a_pas_de_libelle_famille() {
        assert_eq!(libelle_fait_groupe_famille("refus_hors_ligne"), None);
        assert_eq!(libelle_fait_groupe_famille("encadrant_remplace"), None);
        assert_eq!(libelle_fait_groupe_famille(""), None);
    }
}
