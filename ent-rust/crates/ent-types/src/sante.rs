//! Santé et accompagnement (socle 2, lot S2-11) : passages à l'infirmerie, PAI complet et
//! signatures, contraintes publiées, fiche d'urgence, protocoles, dispositifs (PAP, PPS…),
//! aménagements. Miroir fidèle du JSON de `ent-api/src/sante.rs`, sans logique métier.
//!
//! **Minimisation.** Les types destinés aux professeurs, à la restauration et aux sorties
//! ([`AmenagementDto`], [`ContraintePublieeDto`], [`AmenagementsEleveDto`]) ne contiennent
//! que des codes du catalogue fermé et leur libellé neutre : jamais de diagnostic.

use chrono::{DateTime, NaiveDate, NaiveTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Entrée d'un catalogue fermé (`GET /sante/catalogue`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EntreeCatalogueDto {
    pub code: String,
    pub libelle: String,
    #[serde(default)]
    pub audiences: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CatalogueSanteDto {
    pub contraintes: Vec<EntreeCatalogueDto>,
    pub amenagements: Vec<EntreeCatalogueDto>,
    pub issues: Vec<String>,
    pub parties_pai: Vec<String>,
}

// ── Passages ────────────────────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PassageSanteDto {
    pub id: Uuid,
    pub eleve_id: Uuid,
    pub nom: String,
    pub prenom: String,
    pub classe: Option<String>,
    pub jour: NaiveDate,
    pub heure_arrivee: Option<NaiveTime>,
    pub heure_sortie: Option<NaiveTime>,
    pub motif: String,
    /// Vide pour la famille.
    pub note_soins: String,
    pub soins: String,
    pub issue: String,
    pub visible_famille: bool,
    /// `non` | `a_informer` | `informee`.
    pub famille_information: String,
    pub famille_informee_at: Option<DateTime<Utc>>,
    pub recupere_par_nom: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Corps de `POST /sante/passages`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NouveauPassageBody {
    pub eleve_id: Uuid,
    pub jour: NaiveDate,
    pub heure_arrivee: Option<NaiveTime>,
    pub motif: String,
    pub note_soins: String,
    pub soins: String,
    pub visible_famille: bool,
    pub issue: String,
    pub informer_famille: bool,
}

/// Corps de `POST /sante/passages/{id}/cloturer`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CloturePassageBody {
    pub heure_sortie: Option<NaiveTime>,
    pub issue: String,
    pub soins: Option<String>,
    pub recupere_par_personne_id: Option<Uuid>,
}

// ── PAI ─────────────────────────────────────────────────────────────────────

/// PAI historique (`ent-db` : `domain_eleve::Pai`). La famille ne reçoit jamais `note_interne`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaiBaseDto {
    pub id: Uuid,
    pub etablissement_id: Uuid,
    pub eleve_id: Uuid,
    pub motif: String,
    pub mesures_accueil: String,
    pub protocole_urgence: String,
    pub note_interne: String,
    pub date_debut: NaiveDate,
    pub date_fin: Option<NaiveDate>,
    pub actif: bool,
    pub signe_famille_at: Option<DateTime<Utc>>,
    pub signe_famille_user_id: Option<Uuid>,
    pub updated_by_user_id: Uuid,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignaturePaiDto {
    pub partie: String,
    pub signataire_nom: String,
    pub signe_at: DateTime<Utc>,
}

/// `GET /eleves/{id}/pai-complet`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaiCompletDto {
    pub pai: PaiBaseDto,
    pub version: i32,
    pub date_revision: Option<NaiveDate>,
    pub revision_due: bool,
    pub trousse_urgence: bool,
    pub trousse_lieu: String,
    pub trousse_peremption: Option<NaiveDate>,
    pub trousse_perimee: bool,
    pub signatures: Vec<SignaturePaiDto>,
    pub parties_manquantes: Vec<String>,
    pub complet: bool,
}

/// Corps de `PUT /eleves/{id}/pai-complet`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnregistrerPaiBody {
    pub motif: String,
    pub mesures_accueil: String,
    pub protocole_urgence: String,
    pub note_interne: String,
    pub date_debut: NaiveDate,
    pub date_fin: Option<NaiveDate>,
    pub actif: bool,
    pub date_revision: Option<NaiveDate>,
    pub trousse_urgence: bool,
    pub trousse_lieu: String,
    pub trousse_peremption: Option<NaiveDate>,
}

/// Corps de `POST /eleves/{id}/pai/signatures`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignerPaiBody {
    pub partie: String,
    pub signataire_nom: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaiARevoirDto {
    pub eleve_id: Uuid,
    pub nom: String,
    pub prenom: String,
    pub classe: Option<String>,
    pub date_revision: NaiveDate,
    /// Négatif : échue.
    pub jours_restants: i32,
}

// ── Contraintes publiées ────────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContraintePublieeDto {
    pub id: Uuid,
    pub eleve_id: Uuid,
    pub code: String,
    pub libelle: String,
    pub audiences: Vec<String>,
    pub date_debut: NaiveDate,
    pub date_fin: Option<NaiveDate>,
    /// Personnel de santé seulement.
    pub source: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContraintesEleveDto {
    pub eleve_id: Uuid,
    pub nom: String,
    pub prenom: String,
    pub classe: Option<String>,
    pub contraintes: Vec<ContraintePublieeDto>,
}

/// Corps de `POST /eleves/{id}/contraintes`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublierContrainteBody {
    pub code: String,
    pub audiences: Vec<String>,
    pub date_fin: Option<NaiveDate>,
}

// ── Fiche d'urgence ─────────────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersonneUrgenceDto {
    /// Sert au renvoi au domicile : la personne qui récupère l'élève.
    pub personne_id: Uuid,
    pub nom: String,
    pub relation: String,
    pub telephone: Option<String>,
    pub telephone_2: Option<String>,
    pub peut_recuperer: bool,
}

/// `GET /eleves/{id}/fiche-urgence`. `niveau` : `minimale` (encadrant) ou `complete` (santé).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FicheUrgenceDto {
    pub eleve_id: Uuid,
    pub nom: String,
    pub prenom: String,
    pub classe: Option<String>,
    pub date_naissance: Option<NaiveDate>,
    pub niveau: String,
    pub genere_le: DateTime<Utc>,
    pub responsables: Vec<PersonneUrgenceDto>,
    pub contacts: Vec<PersonneUrgenceDto>,
    pub contraintes: Vec<ContraintePublieeDto>,
    pub trousse_urgence: bool,
    pub trousse_lieu: String,
    pub consigne_generale: String,
    pub protocole_urgence: Option<String>,
    pub medecin_nom: Option<String>,
    pub urgence_tel: Option<String>,
}

// ── Protocoles ──────────────────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProtocoleSanteDto {
    pub id: Uuid,
    pub titre: String,
    pub categorie: String,
    pub contenu: String,
    pub version: i32,
    pub actif: bool,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnregistrerProtocoleBody {
    pub id: Option<Uuid>,
    pub titre: String,
    pub categorie: String,
    pub contenu: String,
    pub actif: bool,
}

// ── Accompagnement ──────────────────────────────────────────────────────────

/// Dispositif (PAP, PPS, PPRE, ULIS…) : personnel de suivi seulement.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DispositifDto {
    pub id: Uuid,
    pub eleve_id: Uuid,
    pub kind: String,
    pub libelle: Option<String>,
    pub date_debut: NaiveDate,
    pub date_fin: Option<NaiveDate>,
    pub date_revision: Option<NaiveDate>,
    /// `actif` | `clos`.
    pub statut: String,
    pub revision_due: bool,
}

/// Aménagement du catalogue fermé : visible des professeurs de l'élève.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AmenagementDto {
    pub id: Uuid,
    pub eleve_id: Uuid,
    pub code: String,
    pub libelle: String,
    /// `cours` | `examen` | `les_deux`.
    pub portee: String,
    pub date_debut: NaiveDate,
    pub date_fin: Option<NaiveDate>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AeshDto {
    pub id: Uuid,
    pub eleve_id: Uuid,
    pub aesh_nom: String,
    pub heures_hebdo: Option<i16>,
    pub date_debut: NaiveDate,
    pub date_fin: Option<NaiveDate>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReunionEquipeDto {
    pub id: Uuid,
    pub eleve_id: Uuid,
    pub kind: String,
    pub date_reunion: NaiveDate,
    pub statut: String,
    /// Vide hors gestion de l'accompagnement.
    pub compte_rendu: String,
}

/// `GET /eleves/{id}/accompagnement`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DossierAccompagnementDto {
    pub eleve_id: Uuid,
    pub dispositifs: Vec<DispositifDto>,
    pub amenagements: Vec<AmenagementDto>,
    pub aesh: Vec<AeshDto>,
    pub reunions: Vec<ReunionEquipeDto>,
}

/// `GET /accompagnement/mes-amenagements` : l'élève et ses aménagements, rien d'autre.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AmenagementsEleveDto {
    pub eleve_id: Uuid,
    pub nom: String,
    pub prenom: String,
    pub classe: Option<String>,
    pub amenagements: Vec<AmenagementDto>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DispositifARevoirDto {
    pub dispositif_id: Uuid,
    pub eleve_id: Uuid,
    pub nom: String,
    pub prenom: String,
    pub classe: Option<String>,
    pub kind: String,
    pub date_revision: NaiveDate,
    pub jours_restants: i32,
}

/// Corps de `POST /eleves/{id}/dispositifs`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OuvrirDispositifBody {
    pub kind: String,
    pub libelle: Option<String>,
    pub document_id: Option<Uuid>,
    pub date_debut: Option<NaiveDate>,
    pub date_revision: Option<NaiveDate>,
}

/// Corps de `POST /accompagnement/dispositifs/{id}/reviser`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviserDispositifBody {
    pub date_revision: Option<NaiveDate>,
    pub clore: bool,
}

/// Corps de `POST /eleves/{id}/amenagements`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DefinirAmenagementBody {
    pub code: String,
    pub portee: String,
    pub dispositif_id: Option<Uuid>,
    pub date_fin: Option<NaiveDate>,
}

/// Corps de `POST /eleves/{id}/aesh`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AffecterAeshBody {
    pub aesh_nom: String,
    pub heures_hebdo: Option<i16>,
    pub dispositif_id: Option<Uuid>,
    pub date_debut: NaiveDate,
    pub date_fin: Option<NaiveDate>,
}

/// Corps de `POST /eleves/{id}/reunions`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanifierReunionBody {
    pub kind: String,
    pub date_reunion: NaiveDate,
    pub dispositif_id: Option<Uuid>,
}

/// Corps de `POST /accompagnement/reunions/{id}/compte-rendu`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompteRenduBody {
    pub compte_rendu: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Les aménagements (vue professeur) ne portent ni type de dispositif ni libellé libre.
    #[test]
    fn la_vue_professeur_n_a_que_des_amenagements() {
        let v = AmenagementsEleveDto {
            eleve_id: Uuid::nil(),
            nom: "Potter".into(),
            prenom: "Harry".into(),
            classe: Some("4B".into()),
            amenagements: vec![AmenagementDto {
                id: Uuid::nil(),
                eleve_id: Uuid::nil(),
                code: "tiers_temps".into(),
                libelle: "Tiers-temps".into(),
                portee: "les_deux".into(),
                date_debut: NaiveDate::from_ymd_opt(2026, 9, 1).unwrap(),
                date_fin: None,
            }],
        };
        let json = serde_json::to_string(&v).unwrap();
        for interdit in ["kind", "dispositif", "pap", "pps", "diagnostic"] {
            assert!(!json.contains(interdit), "{interdit} dans {json}");
        }
        let relu: AmenagementsEleveDto = serde_json::from_str(&json).unwrap();
        assert_eq!(relu, v);
    }
}
