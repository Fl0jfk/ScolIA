use chrono::NaiveDate;
use domain_eleve::{Absence, AttributKind, Eleve, ElevePeriode};
use ent_db::{AbsenceMotif, IbanHisto, MailOutboxRow, Matiere, Note, SmsOutboxRow};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize)]
pub struct EleveDto {
    pub id: Uuid,
    pub etablissement_id: Uuid,
    pub nom: String,
    pub prenom: String,
    pub ine: Option<String>,
    pub sexe: Option<String>,
    pub date_naissance: Option<NaiveDate>,
    pub numero_badge: Option<String>,
    pub redoublant: bool,
    pub status: String,
    pub classe: Option<String>,
    pub regime: Option<String>,
    pub mef: Option<String>,
    pub site: Option<String>,
    pub date_entree: Option<NaiveDate>,
    pub date_sortie: Option<NaiveDate>,
    pub motif_sortie: Option<String>,
}

impl From<&Eleve> for EleveDto {
    fn from(e: &Eleve) -> Self {
        Self {
            id: e.id,
            etablissement_id: e.etablissement_id,
            nom: e.nom.clone(),
            prenom: e.prenom.clone(),
            ine: e.ine.clone(),
            sexe: e.sexe.clone(),
            date_naissance: e.date_naissance,
            numero_badge: e.numero_badge.clone(),
            redoublant: e.redoublant,
            status: e.status.as_str().to_string(),
            classe: e.classe.clone(),
            regime: e.regime.clone(),
            mef: e.mef.clone(),
            site: e.site.clone(),
            date_entree: e.date_entree,
            date_sortie: e.date_sortie,
            motif_sortie: e.motif_sortie.clone(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct InviterCompteBody {
    pub email: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CompteProvisionneDto {
    pub invitation_id: Uuid,
    pub eleve_id: Uuid,
    pub email: String,
    pub mot_de_passe_temporaire: String,
    pub statut: String,
    pub user_id: Option<Uuid>,
}

impl CompteProvisionneDto {
    pub fn from_parts(
        inv: &domain_eleve::EleveInvitation,
        email: &str,
        mdp: &str,
    ) -> Self {
        Self {
            invitation_id: inv.id,
            eleve_id: inv.eleve_id,
            email: email.to_string(),
            mot_de_passe_temporaire: mdp.to_string(),
            statut: inv.statut.as_str().to_string(),
            user_id: inv.activated_user_id,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct InvitationCompteDto {
    pub id: Uuid,
    pub eleve_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eleve_label: Option<String>,
    pub email: String,
    pub statut: String,
    pub activated_user_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub activated_user_label: Option<String>,
    pub created_at: String,
}

impl From<&domain_eleve::EleveInvitation> for InvitationCompteDto {
    fn from(i: &domain_eleve::EleveInvitation) -> Self {
        Self {
            id: i.id,
            eleve_id: i.eleve_id,
            eleve_label: None,
            email: i.email.clone(),
            statut: i.statut.as_str().to_string(),
            activated_user_id: i.activated_user_id,
            activated_user_label: None,
            created_at: i.created_at.to_rfc3339(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreerPassageInfirmerieBody {
    pub eleve_id: Uuid,
    pub jour: NaiveDate,
    pub motif: String,
    pub note_soins: String,
    pub visible_famille: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct PassageInfirmerieDto {
    pub id: Uuid,
    pub eleve_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eleve_label: Option<String>,
    pub jour: NaiveDate,
    pub motif: String,
    pub motif_libelle: String,
    pub note_soins: String,
    pub visible_famille: bool,
    pub created_at: String,
}

impl From<&domain_eleve::PassageInfirmerie> for PassageInfirmerieDto {
    fn from(p: &domain_eleve::PassageInfirmerie) -> Self {
        Self {
            id: p.id,
            eleve_id: p.eleve_id,
            eleve_label: None,
            jour: p.jour,
            motif: p.motif.as_str().to_string(),
            motif_libelle: p.motif.libelle().to_string(),
            note_soins: p.note_soins.clone(),
            visible_famille: p.visible_famille,
            created_at: p.created_at.to_rfc3339(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct UpsertFicheSanteBody {
    pub allergies: String,
    pub traitements: String,
    pub contre_indications: String,
    pub pai_actif: bool,
    pub medecin_nom: Option<String>,
    pub urgence_tel: Option<String>,
    pub note_confidentielle: String,
}

#[derive(Debug, Serialize)]
pub struct FicheSanteDto {
    pub id: Uuid,
    pub eleve_id: Uuid,
    pub allergies: String,
    pub traitements: String,
    pub contre_indications: String,
    pub pai_actif: bool,
    pub medecin_nom: Option<String>,
    pub urgence_tel: Option<String>,
    pub note_confidentielle: String,
    pub updated_at: String,
}

impl From<&domain_eleve::FicheSante> for FicheSanteDto {
    fn from(f: &domain_eleve::FicheSante) -> Self {
        Self {
            id: f.id,
            eleve_id: f.eleve_id,
            allergies: f.allergies.clone(),
            traitements: f.traitements.clone(),
            contre_indications: f.contre_indications.clone(),
            pai_actif: f.pai_actif,
            medecin_nom: f.medecin_nom.clone(),
            urgence_tel: f.urgence_tel.clone(),
            note_confidentielle: f.note_confidentielle.clone(),
            updated_at: f.updated_at.to_rfc3339(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct UpsertPaiBody {
    pub motif: String,
    pub mesures_accueil: String,
    pub protocole_urgence: String,
    pub note_interne: Option<String>,
    pub date_debut: NaiveDate,
    pub date_fin: Option<NaiveDate>,
    pub actif: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct PaiDto {
    pub id: Uuid,
    pub eleve_id: Uuid,
    pub motif: String,
    pub mesures_accueil: String,
    pub protocole_urgence: String,
    pub note_interne: String,
    pub date_debut: NaiveDate,
    pub date_fin: Option<NaiveDate>,
    pub actif: bool,
    pub signe_famille: bool,
    pub signe_famille_at: Option<String>,
    pub updated_at: String,
}

impl From<&domain_eleve::Pai> for PaiDto {
    fn from(p: &domain_eleve::Pai) -> Self {
        Self {
            id: p.id,
            eleve_id: p.eleve_id,
            motif: p.motif.clone(),
            mesures_accueil: p.mesures_accueil.clone(),
            protocole_urgence: p.protocole_urgence.clone(),
            note_interne: p.note_interne.clone(),
            date_debut: p.date_debut,
            date_fin: p.date_fin,
            actif: p.actif,
            signe_famille: p.signe_famille_at.is_some(),
            signe_famille_at: p.signe_famille_at.map(|t| t.to_rfc3339()),
            updated_at: p.updated_at.to_rfc3339(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct PaiPieceDto {
    pub id: Uuid,
    pub pai_id: Uuid,
    pub nom: String,
    pub mime_type: String,
    pub taille_octets: i32,
    pub created_at: String,
}

impl From<&domain_eleve::PaiPieceMeta> for PaiPieceDto {
    fn from(p: &domain_eleve::PaiPieceMeta) -> Self {
        Self {
            id: p.id,
            pai_id: p.pai_id,
            nom: p.nom.clone(),
            mime_type: p.mime_type.clone(),
            taille_octets: p.taille_octets,
            created_at: p.created_at.to_rfc3339(),
        }
    }
}

impl From<&domain_eleve::PaiPiece> for PaiPieceDto {
    fn from(p: &domain_eleve::PaiPiece) -> Self {
        Self {
            id: p.id,
            pai_id: p.pai_id,
            nom: p.nom.clone(),
            mime_type: p.mime_type.clone(),
            taille_octets: p.taille_octets,
            created_at: p.created_at.to_rfc3339(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreerCommissionBody {
    pub jour: NaiveDate,
    /// Format `HH:MM`.
    pub heure: String,
    pub objet: String,
    pub lieu: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CommissionDto {
    pub id: Uuid,
    pub jour: NaiveDate,
    pub heure: String,
    pub objet: String,
    pub lieu: String,
    pub statut: String,
    pub statut_libelle: String,
}

impl From<&domain_eleve::CommissionVs> for CommissionDto {
    fn from(c: &domain_eleve::CommissionVs) -> Self {
        Self {
            id: c.id,
            jour: c.jour,
            heure: c.heure.format("%H:%M").to_string(),
            objet: c.objet.clone(),
            lieu: c.lieu.clone(),
            statut: c.statut.as_str().to_string(),
            statut_libelle: c.statut.libelle().to_string(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct ConvoquerCommissionBody {
    pub eleve_id: Uuid,
    pub motifs: String,
    pub note_interne: Option<String>,
    pub sanction_id: Option<Uuid>,
    pub incident_id: Option<Uuid>,
    pub signale_famille: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct DeciderCommissionBody {
    pub decision: String,
    pub decision_texte: Option<String>,
    pub points_retires: Option<i32>,
}

#[derive(Debug, Serialize)]
pub struct CommissionDossierDto {
    pub id: Uuid,
    pub commission_id: Uuid,
    pub eleve_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eleve_label: Option<String>,
    pub motifs: String,
    pub note_interne: String,
    pub decision: Option<String>,
    pub decision_libelle: Option<String>,
    pub decision_texte: String,
    pub points_retires: i32,
    pub signale_famille: bool,
    pub decided_at: Option<String>,
}

impl From<&domain_eleve::CommissionDossier> for CommissionDossierDto {
    fn from(d: &domain_eleve::CommissionDossier) -> Self {
        Self {
            id: d.id,
            commission_id: d.commission_id,
            eleve_id: d.eleve_id,
            eleve_label: None,
            motifs: d.motifs.clone(),
            note_interne: d.note_interne.clone(),
            decision: d.decision.map(|x| x.as_str().to_string()),
            decision_libelle: d.decision.map(|x| x.libelle().to_string()),
            decision_texte: d.decision_texte.clone(),
            points_retires: d.points_retires,
            signale_famille: d.signale_famille,
            decided_at: d.decided_at.map(|t| t.to_rfc3339()),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct SoldePointsDto {
    pub eleve_id: Uuid,
    pub solde: i32,
    pub depart: i32,
}

#[derive(Debug, Deserialize)]
pub struct CreerEntretienPsyBody {
    pub eleve_id: Uuid,
    pub jour: NaiveDate,
    pub kind: String,
    pub duree_minutes: i32,
    pub note_confidentielle: String,
    pub signale_famille: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct EntretienPsyDto {
    pub id: Uuid,
    pub eleve_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eleve_label: Option<String>,
    pub jour: NaiveDate,
    pub kind: String,
    pub kind_libelle: String,
    pub duree_minutes: i32,
    pub note_confidentielle: String,
    pub signale_famille: bool,
    pub created_at: String,
}

impl From<&domain_eleve::EntretienPsy> for EntretienPsyDto {
    fn from(e: &domain_eleve::EntretienPsy) -> Self {
        Self {
            id: e.id,
            eleve_id: e.eleve_id,
            eleve_label: None,
            jour: e.jour,
            kind: e.kind.as_str().to_string(),
            kind_libelle: e.kind.libelle().to_string(),
            duree_minutes: e.duree_minutes,
            note_confidentielle: e.note_confidentielle.clone(),
            signale_famille: e.signale_famille,
            created_at: e.created_at.to_rfc3339(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreerRdvPsyBody {
    pub eleve_id: Uuid,
    pub jour: NaiveDate,
    /// Format `HH:MM` ou `HH:MM:SS`.
    pub heure_debut: String,
    pub heure_fin: String,
    pub kind: String,
    pub motif: String,
    pub note_interne: Option<String>,
    pub signale_famille: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct ChangerStatutRdvPsyBody {
    pub statut: String,
}

#[derive(Debug, Serialize)]
pub struct RdvPsyDto {
    pub id: Uuid,
    pub eleve_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eleve_label: Option<String>,
    pub jour: NaiveDate,
    pub heure_debut: String,
    pub heure_fin: String,
    pub kind: String,
    pub kind_libelle: String,
    pub motif: String,
    pub note_interne: String,
    pub statut: String,
    pub signale_famille: bool,
    pub created_at: String,
}

impl From<&domain_eleve::RdvPsy> for RdvPsyDto {
    fn from(r: &domain_eleve::RdvPsy) -> Self {
        Self {
            id: r.id,
            eleve_id: r.eleve_id,
            eleve_label: None,
            jour: r.jour,
            heure_debut: r.heure_debut.format("%H:%M").to_string(),
            heure_fin: r.heure_fin.format("%H:%M").to_string(),
            kind: r.kind.as_str().to_string(),
            kind_libelle: r.kind.libelle().to_string(),
            motif: r.motif.clone(),
            note_interne: r.note_interne.clone(),
            statut: r.statut.as_str().to_string(),
            signale_famille: r.signale_famille,
            created_at: r.created_at.to_rfc3339(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct UpsertDossierPsyBody {
    pub axes_suivi: String,
    pub contacts_externes: String,
    pub note_confidentielle: String,
    pub suivi_actif: bool,
    pub signale_famille: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct DossierPsyDto {
    pub id: Uuid,
    pub eleve_id: Uuid,
    pub axes_suivi: String,
    pub contacts_externes: String,
    pub note_confidentielle: String,
    pub suivi_actif: bool,
    pub signale_famille: bool,
    pub updated_at: String,
}

impl From<&domain_eleve::DossierPsy> for DossierPsyDto {
    fn from(d: &domain_eleve::DossierPsy) -> Self {
        Self {
            id: d.id,
            eleve_id: d.eleve_id,
            axes_suivi: d.axes_suivi.clone(),
            contacts_externes: d.contacts_externes.clone(),
            note_confidentielle: d.note_confidentielle.clone(),
            suivi_actif: d.suivi_actif,
            signale_famille: d.signale_famille,
            updated_at: d.updated_at.to_rfc3339(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateEleveBody {
    pub nom: String,
    pub prenom: String,
    pub ine: Option<String>,
    pub sexe: Option<String>,
    pub date_naissance: Option<NaiveDate>,
    pub numero_badge: Option<String>,
    pub redoublant: Option<bool>,
    pub classe: Option<String>,
    pub regime: Option<String>,
    pub mef: Option<String>,
    pub site: Option<String>,
    pub date_entree: Option<NaiveDate>,
}

#[derive(Debug, Deserialize)]
pub struct ChangeClasseBody {
    pub nouvelle_classe: String,
    pub date_effet: NaiveDate,
}

#[derive(Debug, Deserialize)]
pub struct ChangeRegimeBody {
    pub nouveau_regime: String,
    pub date_effet: NaiveDate,
}

#[derive(Debug, Deserialize)]
pub struct SortirBody {
    pub date_sortie: NaiveDate,
    pub motif_sortie: Option<String>,
}

/// Édition identité élève — champs envoyés = appliqués (chaîne vide → effacement).
#[derive(Debug, Deserialize)]
pub struct UpdateIdentiteBody {
    pub nom: String,
    pub prenom: String,
    pub ine: Option<String>,
    pub sexe: Option<String>,
    pub date_naissance: Option<NaiveDate>,
    pub numero_badge: Option<String>,
    pub redoublant: bool,
    pub date_entree: Option<NaiveDate>,
}

#[derive(Debug, Deserialize)]
pub struct ChangeMefBody {
    pub nouveau_mef: String,
    pub date_effet: NaiveDate,
}

#[derive(Debug, Deserialize)]
pub struct ChangeSiteBody {
    pub nouveau_site: String,
    pub date_effet: NaiveDate,
}

#[derive(Debug, Serialize)]
pub struct PeriodeDto {
    pub id: Uuid,
    pub kind: String,
    pub valeur: String,
    pub date_debut: NaiveDate,
    pub date_fin: Option<NaiveDate>,
}

impl From<&ElevePeriode> for PeriodeDto {
    fn from(p: &ElevePeriode) -> Self {
        Self {
            id: p.id,
            kind: p.kind.as_str().to_string(),
            valeur: p.valeur.clone(),
            date_debut: p.date_debut,
            date_fin: p.date_fin,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct DeclareAbsenceBody {
    pub debut: NaiveDate,
    pub fin: NaiveDate,
    pub motif: String,
}

#[derive(Debug, Serialize)]
pub struct AbsenceDto {
    pub id: Uuid,
    pub etablissement_id: Uuid,
    pub eleve_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eleve_label: Option<String>,
    pub debut: NaiveDate,
    pub fin: NaiveDate,
    pub motif: String,
    pub statut: String,
    pub motif_justif: Option<String>,
    pub decision_note: Option<String>,
}

impl From<&Absence> for AbsenceDto {
    fn from(a: &Absence) -> Self {
        Self {
            id: a.id,
            etablissement_id: a.etablissement_id,
            eleve_id: a.eleve_id,
            eleve_label: None,
            debut: a.debut,
            fin: a.fin,
            motif: a.motif.clone(),
            statut: a.statut.as_str().to_string(),
            motif_justif: a.motif_justif.clone(),
            decision_note: a.decision_note.clone(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct JustifierAbsenceBody {
    pub motif_justif: String,
}

/// Note optionnelle CPE / prof sur accepter, refuser ou annuler→présent.
#[derive(Debug, Deserialize)]
pub struct DecisionAbsenceBody {
    pub note: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct AbsentDto {
    pub eleve_id: Uuid,
    pub jour: NaiveDate,
    pub absent: bool,
}

#[derive(Debug, Serialize)]
pub struct EffectifDto {
    pub classe: String,
    pub jour: NaiveDate,
    pub eleves: Vec<EleveDto>,
}

#[derive(Debug, Deserialize)]
pub struct CreateMatiereBody {
    pub code: String,
    pub libelle: String,
    pub source_nomenclature: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct MatiereDto {
    pub id: Uuid,
    pub code: String,
    pub libelle: String,
    pub source_nomenclature: Option<String>,
}

impl From<&Matiere> for MatiereDto {
    fn from(m: &Matiere) -> Self {
        Self {
            id: m.id,
            code: m.code.clone(),
            libelle: m.libelle.clone(),
            source_nomenclature: m.source_nomenclature.clone(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct SaisirNoteBody {
    pub matiere_id: Uuid,
    pub valeur: f64,
    pub sur: Option<f64>,
    pub date_note: NaiveDate,
    pub libelle: Option<String>,
    pub devoir_id: Option<Uuid>,
    pub periode_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
pub struct CreerNotePeriodeBody {
    pub code: String,
    pub libelle: String,
    pub date_debut: NaiveDate,
    pub date_fin: NaiveDate,
}

#[derive(Debug, Deserialize)]
pub struct PublierBulletinBody {
    pub periode_id: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct SetNotePeriodeStatutBody {
    pub statut: String,
}

#[derive(Debug, Deserialize)]
pub struct CreerDevoirBody {
    pub periode_id: Uuid,
    pub matiere_id: Uuid,
    pub classe: String,
    pub titre: String,
    pub date_devoir: NaiveDate,
    pub coef: Option<f64>,
    pub sur: Option<f64>,
}

#[derive(Debug, Serialize)]
pub struct NotePeriodeDto {
    pub id: Uuid,
    pub code: String,
    pub libelle: String,
    pub date_debut: NaiveDate,
    pub date_fin: NaiveDate,
    pub statut: String,
}

impl From<&domain_eleve::NotePeriode> for NotePeriodeDto {
    fn from(p: &domain_eleve::NotePeriode) -> Self {
        Self {
            id: p.id,
            code: p.code.clone(),
            libelle: p.libelle.clone(),
            date_debut: p.date_debut,
            date_fin: p.date_fin,
            statut: p.statut.as_str().to_string(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct DevoirDto {
    pub id: Uuid,
    pub periode_id: Uuid,
    pub matiere_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub matiere_libelle: Option<String>,
    pub classe: String,
    pub titre: String,
    pub date_devoir: NaiveDate,
    pub coef: f64,
    pub sur: f64,
}

impl From<&domain_eleve::Devoir> for DevoirDto {
    fn from(d: &domain_eleve::Devoir) -> Self {
        Self {
            id: d.id,
            periode_id: d.periode_id,
            matiere_id: d.matiere_id,
            matiere_libelle: None,
            classe: d.classe.clone(),
            titre: d.titre.clone(),
            date_devoir: d.date_devoir,
            coef: d.coef,
            sur: d.sur,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct MoyenneMatiereDto {
    pub matiere_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub matiere_libelle: Option<String>,
    pub periode_id: Uuid,
    pub moyenne_sur_20: f64,
}

impl From<&domain_eleve::MoyenneMatiere> for MoyenneMatiereDto {
    fn from(m: &domain_eleve::MoyenneMatiere) -> Self {
        Self {
            matiere_id: m.matiere_id,
            matiere_libelle: None,
            periode_id: m.periode_id,
            moyenne_sur_20: m.moyenne_sur_20,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct SaisirAppreciationBody {
    pub periode_id: Uuid,
    pub matiere_id: Option<Uuid>,
    pub texte: String,
}

#[derive(Debug, Serialize)]
pub struct AppreciationDto {
    pub id: Uuid,
    pub periode_id: Uuid,
    pub eleve_id: Uuid,
    pub matiere_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub matiere_libelle: Option<String>,
    pub texte: String,
    pub auteur_user_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auteur_label: Option<String>,
}

impl From<&domain_eleve::Appreciation> for AppreciationDto {
    fn from(a: &domain_eleve::Appreciation) -> Self {
        Self {
            id: a.id,
            periode_id: a.periode_id,
            eleve_id: a.eleve_id,
            matiere_id: a.matiere_id,
            matiere_libelle: None,
            texte: a.texte.clone(),
            auteur_user_id: a.auteur_user_id,
            auteur_label: None,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct OuvrirAppelBody {
    pub creneau_id: Uuid,
    pub jour: NaiveDate,
}

#[derive(Debug, Serialize)]
pub struct EdtCreneauDto {
    pub id: Uuid,
    pub classe: String,
    pub matiere_libelle: String,
    pub prof_user_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prof_label: Option<String>,
    pub jour_semaine: i16,
    pub heure_debut: String,
    pub heure_fin: String,
    pub salle: Option<String>,
    pub semaine_ab: String,
}

impl From<&ent_db::EdtCreneau> for EdtCreneauDto {
    fn from(c: &ent_db::EdtCreneau) -> Self {
        Self {
            id: c.id,
            classe: c.classe.clone(),
            matiere_libelle: c.matiere_libelle.clone(),
            prof_user_id: c.prof_user_id,
            prof_label: None,
            jour_semaine: c.jour_semaine,
            heure_debut: c.heure_debut.format("%H:%M").to_string(),
            heure_fin: c.heure_fin.format("%H:%M").to_string(),
            salle: c.salle.clone(),
            semaine_ab: c.semaine_ab.as_str().to_string(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct ConfigurerEdtCalendrierAbBody {
    pub reference_lundi: NaiveDate,
}

#[derive(Debug, Serialize)]
pub struct EdtCalendrierAbDto {
    pub reference_lundi: NaiveDate,
    pub updated_at: String,
}

impl From<&domain_eleve::EdtCalendrierAb> for EdtCalendrierAbDto {
    fn from(c: &domain_eleve::EdtCalendrierAb) -> Self {
        Self {
            reference_lundi: c.reference_lundi,
            updated_at: c.updated_at.to_rfc3339(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct PoserEdtExceptionBody {
    pub creneau_id: Uuid,
    pub jour: NaiveDate,
    pub kind: String,
    pub remplacant_user_id: Option<Uuid>,
    pub nouvelle_salle: Option<String>,
    pub motif: String,
}

#[derive(Debug, Serialize)]
pub struct EdtExceptionDto {
    pub id: Uuid,
    pub creneau_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub creneau_label: Option<String>,
    pub jour: NaiveDate,
    pub kind: String,
    pub remplacant_user_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remplacant_label: Option<String>,
    pub nouvelle_salle: Option<String>,
    pub motif: String,
    pub created_at: String,
}

impl From<&domain_eleve::EdtException> for EdtExceptionDto {
    fn from(e: &domain_eleve::EdtException) -> Self {
        Self {
            id: e.id,
            creneau_id: e.creneau_id,
            creneau_label: None,
            jour: e.jour,
            kind: e.kind.as_str().to_string(),
            remplacant_user_id: e.remplacant_user_id,
            remplacant_label: None,
            nouvelle_salle: e.nouvelle_salle.clone(),
            motif: e.motif.clone(),
            created_at: e.created_at.to_rfc3339(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreerCahierTexteBody {
    pub classe: String,
    pub matiere_libelle: String,
    pub titre: String,
    pub consigne: String,
    pub pour_le: NaiveDate,
    pub cree_le: Option<NaiveDate>,
}

#[derive(Debug, Serialize)]
pub struct CahierTexteDto {
    pub id: Uuid,
    pub classe: String,
    pub matiere_libelle: String,
    pub titre: String,
    pub consigne: String,
    pub pour_le: NaiveDate,
    pub cree_le: NaiveDate,
    pub created_at: String,
}

impl From<&domain_eleve::CahierTexteEntree> for CahierTexteDto {
    fn from(e: &domain_eleve::CahierTexteEntree) -> Self {
        Self {
            id: e.id,
            classe: e.classe.clone(),
            matiere_libelle: e.matiere_libelle.clone(),
            titre: e.titre.clone(),
            consigne: e.consigne.clone(),
            pour_le: e.pour_le,
            cree_le: e.cree_le,
            created_at: e.created_at.to_rfc3339(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreerCarnetBody {
    pub date_entree: NaiveDate,
    pub categorie: String,
    pub titre: String,
    pub corps: String,
    #[serde(default = "default_true")]
    pub visible_famille: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Serialize)]
pub struct CarnetEntreeDto {
    pub id: Uuid,
    pub eleve_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eleve_label: Option<String>,
    pub date_entree: NaiveDate,
    pub categorie: String,
    pub titre: String,
    pub corps: String,
    pub visible_famille: bool,
    pub signe_at: Option<String>,
    pub signe_par_user_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signe_par_label: Option<String>,
    pub fil_notif_id: Option<Uuid>,
}

impl From<&domain_eleve::CarnetEntree> for CarnetEntreeDto {
    fn from(e: &domain_eleve::CarnetEntree) -> Self {
        Self {
            id: e.id,
            eleve_id: e.eleve_id,
            eleve_label: None,
            date_entree: e.date_entree,
            categorie: e.categorie.as_str().to_string(),
            titre: e.titre.clone(),
            corps: e.corps.clone(),
            visible_famille: e.visible_famille,
            signe_at: e.signe_at.map(|t| t.to_rfc3339()),
            signe_par_user_id: e.signe_par_user_id,
            signe_par_label: None,
            fil_notif_id: e.fil_notif_id,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct MarquerAppelLigneBody {
    pub eleve_id: Uuid,
    pub etat: String,
}

#[derive(Debug, Serialize)]
pub struct AppelLigneDto {
    pub eleve_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eleve_label: Option<String>,
    pub etat: String,
}

#[derive(Debug, Serialize)]
pub struct AppelDto {
    pub id: Uuid,
    pub classe: String,
    pub jour: NaiveDate,
    pub statut: String,
    pub creneau_id: Option<Uuid>,
    pub matiere_libelle: Option<String>,
    pub lignes: Vec<AppelLigneDto>,
}

#[derive(Debug, Serialize)]
pub struct NoteDto {
    pub id: Uuid,
    pub eleve_id: Uuid,
    pub matiere_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub matiere_libelle: Option<String>,
    pub valeur: f64,
    pub sur: f64,
    pub date_note: NaiveDate,
    pub libelle: Option<String>,
    pub devoir_id: Option<Uuid>,
    pub periode_id: Option<Uuid>,
}

impl From<&Note> for NoteDto {
    fn from(n: &Note) -> Self {
        Self {
            id: n.id,
            eleve_id: n.eleve_id,
            matiere_id: n.matiere_id,
            matiere_libelle: None,
            valeur: n.valeur,
            sur: n.sur,
            date_note: n.date_note,
            libelle: n.libelle.clone(),
            devoir_id: n.devoir_id,
            periode_id: n.periode_id,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct EnfantFamilleDto {
    pub id: Uuid,
    pub nom: String,
    pub prenom: String,
    pub classe: Option<String>,
    pub status: String,
    pub lien: String,
    pub peut_voir_notes: bool,
    pub peut_voir_vs: bool,
    pub peut_justifier_absence: bool,
    pub peut_autoriser_sorties: bool,
}

impl From<&ent_db::EnfantFamille> for EnfantFamilleDto {
    fn from(e: &ent_db::EnfantFamille) -> Self {
        Self {
            id: e.eleve.id,
            nom: e.eleve.nom.clone(),
            prenom: e.eleve.prenom.clone(),
            classe: e.eleve.classe.clone(),
            status: e.eleve.status.as_str().to_string(),
            lien: e.lien.clone(),
            peut_voir_notes: e.peut_voir_notes,
            peut_voir_vs: e.peut_voir_vs,
            peut_justifier_absence: e.peut_justifier_absence,
            peut_autoriser_sorties: e.peut_autoriser_sorties,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct FamilleResumeDto {
    pub eleve_id: Uuid,
    pub absences_a_justifier: Vec<AbsenceDto>,
    pub carnet_a_signer: Vec<CarnetEntreeDto>,
    pub notes: Vec<NoteDto>,
}

#[derive(Debug, Deserialize)]
pub struct OuvrirFilBody {
    pub eleve_id: Uuid,
    pub sujet: String,
    pub message: String,
}

#[derive(Debug, Deserialize)]
pub struct OuvrirFilDepuisModeleBody {
    pub modele_id: Uuid,
    pub eleve_id: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct CreerMsgModeleBody {
    pub titre: String,
    pub sujet: String,
    pub corps: String,
}

#[derive(Debug, Deserialize)]
pub struct RepondreFilBody {
    pub corps: String,
}

#[derive(Debug, Serialize)]
pub struct MsgFilDto {
    pub id: Uuid,
    pub eleve_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eleve_label: Option<String>,
    pub sujet: String,
    pub created_by_user_id: Uuid,
    pub created_at: String,
    pub last_message_at: String,
    pub archive: bool,
    pub modele_id: Option<Uuid>,
    pub priorite: String,
    pub dossier: Option<String>,
    /// Messages des autres non encore accusés par l’acteur courant.
    pub nb_non_lus: i64,
}

impl From<&domain_eleve::MsgFil> for MsgFilDto {
    fn from(f: &domain_eleve::MsgFil) -> Self {
        Self {
            id: f.id,
            eleve_id: f.eleve_id,
            eleve_label: None,
            sujet: f.sujet.clone(),
            created_by_user_id: f.created_by_user_id,
            created_at: f.created_at.to_rfc3339(),
            last_message_at: f.last_message_at.to_rfc3339(),
            archive: f.archive,
            modele_id: f.modele_id,
            priorite: f.priorite.as_str().to_string(),
            dossier: f.dossier.clone(),
            nb_non_lus: 0,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct ClasserFilBody {
    pub priorite: String,
    pub dossier: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct MsgModeleDto {
    pub id: Uuid,
    pub titre: String,
    pub sujet: String,
    pub corps: String,
    pub actif: bool,
    pub created_at: String,
}

impl From<&domain_eleve::MsgModele> for MsgModeleDto {
    fn from(m: &domain_eleve::MsgModele) -> Self {
        Self {
            id: m.id,
            titre: m.titre.clone(),
            sujet: m.sujet.clone(),
            corps: m.corps.clone(),
            actif: m.actif,
            created_at: m.created_at.to_rfc3339(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct MsgPieceDto {
    pub id: Uuid,
    pub message_id: Uuid,
    pub nom: String,
    pub mime_type: String,
    pub taille_octets: i32,
    pub created_at: String,
}

impl From<&domain_eleve::MsgPieceMeta> for MsgPieceDto {
    fn from(p: &domain_eleve::MsgPieceMeta) -> Self {
        Self {
            id: p.id,
            message_id: p.message_id,
            nom: p.nom.clone(),
            mime_type: p.mime_type.clone(),
            taille_octets: p.taille_octets,
            created_at: p.created_at.to_rfc3339(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct AttacherPieceBody {
    pub nom: String,
    pub mime_type: String,
    /// Contenu encodé base64 (max 256 Ko décodé).
    pub contenu_base64: String,
}

#[derive(Debug, Serialize)]
pub struct MessageDto {
    pub id: Uuid,
    pub fil_id: Uuid,
    pub auteur_user_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auteur_label: Option<String>,
    pub corps: String,
    pub created_at: String,
    pub lu_par_moi: bool,
    pub nb_accuses: i64,
    pub pieces: Vec<MsgPieceDto>,
}

impl From<&domain_eleve::Message> for MessageDto {
    fn from(m: &domain_eleve::Message) -> Self {
        Self {
            id: m.id,
            fil_id: m.fil_id,
            auteur_user_id: m.auteur_user_id,
            auteur_label: None,
            corps: m.corps.clone(),
            created_at: m.created_at.to_rfc3339(),
            lu_par_moi: false,
            nb_accuses: 0,
            pieces: Vec::new(),
        }
    }
}

impl From<&domain_eleve::MessageLue> for MessageDto {
    fn from(m: &domain_eleve::MessageLue) -> Self {
        Self {
            id: m.message.id,
            fil_id: m.message.fil_id,
            auteur_user_id: m.message.auteur_user_id,
            auteur_label: None,
            corps: m.message.corps.clone(),
            created_at: m.message.created_at.to_rfc3339(),
            lu_par_moi: m.lu_par_moi,
            nb_accuses: m.nb_accuses,
            pieces: m.pieces.iter().map(MsgPieceDto::from).collect(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct FilDetailDto {
    pub fil: MsgFilDto,
    pub messages: Vec<MessageDto>,
}

#[derive(Debug, Serialize)]
pub struct CampagneDto {
    pub id: Uuid,
    pub annee_cible: String,
    pub ouverte: bool,
    pub acompte_centimes: i64,
}

impl From<&domain_eleve::ReinscriptionCampagne> for CampagneDto {
    fn from(c: &domain_eleve::ReinscriptionCampagne) -> Self {
        Self {
            id: c.id,
            annee_cible: c.annee_cible.clone(),
            ouverte: c.ouverte,
            acompte_centimes: c.acompte_centimes,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct OuvrirCampagneBody {
    pub annee_cible: String,
}

#[derive(Debug, Deserialize)]
pub struct ConfigurerAcompteBody {
    pub annee_cible: String,
    pub acompte_centimes: i64,
}

#[derive(Debug, Deserialize)]
pub struct MonteeRapideBody {
    pub eleve_id: Uuid,
    pub annee_courante: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct MonteeLotBody {
    pub niveau_source: String,
    pub annee_courante: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct LotMonteeIgnoreDto {
    pub eleve_id: Uuid,
    pub motif: String,
}

#[derive(Debug, Serialize)]
pub struct LotMonteeResultatDto {
    pub niveau_source: String,
    pub niveau_cible: String,
    pub annee_courante: String,
    pub annee_cible: String,
    pub montes: Vec<Uuid>,
    pub ignores: Vec<LotMonteeIgnoreDto>,
    pub montes_count: usize,
    pub ignores_count: usize,
}

impl From<&domain_eleve::LotMonteeResultat> for LotMonteeResultatDto {
    fn from(r: &domain_eleve::LotMonteeResultat) -> Self {
        Self {
            niveau_source: r.niveau_source.clone(),
            niveau_cible: r.niveau_cible.clone(),
            annee_courante: r.annee_courante.clone(),
            annee_cible: r.annee_cible.clone(),
            montes: r.montes.clone(),
            ignores: r
                .ignores
                .iter()
                .map(|i| LotMonteeIgnoreDto {
                    eleve_id: i.eleve_id,
                    motif: i.motif.clone(),
                })
                .collect(),
            montes_count: r.montes.len(),
            ignores_count: r.ignores.len(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct CandidatsMonteeLotDto {
    pub niveau_source: String,
    pub niveau_cible: String,
    pub annee_courante: String,
    pub candidats: Vec<ScolariteAnneeDto>,
    pub count: usize,
}

#[derive(Debug, Deserialize)]
pub struct ChoisirOptionBody {
    pub code: String,
    pub libelle: String,
    pub nature: Option<String>,
    #[serde(default = "default_true")]
    pub retenir: bool,
}

#[derive(Debug, Serialize)]
pub struct ScolariteAnneeDto {
    pub id: Uuid,
    pub eleve_id: Uuid,
    pub annee_scolaire: String,
    pub niveau: Option<String>,
    pub mef: Option<String>,
    pub classe: Option<String>,
    pub statut: String,
    pub source: String,
    pub passage_juridique: bool,
    pub redoublement_possible: bool,
}

impl From<&domain_eleve::ScolariteAnnee> for ScolariteAnneeDto {
    fn from(s: &domain_eleve::ScolariteAnnee) -> Self {
        Self {
            id: s.id,
            eleve_id: s.eleve_id,
            annee_scolaire: s.annee_scolaire.clone(),
            niveau: s.niveau.clone(),
            mef: s.mef.clone(),
            classe: s.classe.clone(),
            statut: s.statut.as_str().to_string(),
            source: s.source.as_str().to_string(),
            passage_juridique: s.passage_juridique,
            redoublement_possible: s.redoublement_possible,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct OptionAnneeDto {
    pub id: Uuid,
    pub eleve_id: Uuid,
    pub annee_scolaire: String,
    pub nature: String,
    pub code: String,
    pub libelle: String,
    pub statut: String,
}

impl From<&domain_eleve::OptionAnnee> for OptionAnneeDto {
    fn from(o: &domain_eleve::OptionAnnee) -> Self {
        Self {
            id: o.id,
            eleve_id: o.eleve_id,
            annee_scolaire: o.annee_scolaire.clone(),
            nature: o.nature.as_str().to_string(),
            code: o.code.clone(),
            libelle: o.libelle.clone(),
            statut: o.statut.as_str().to_string(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ExigenceDocDto {
    pub id: Uuid,
    pub annee_cible: String,
    pub code: String,
    pub libelle: String,
    pub tiroir: String,
    pub obligatoire: bool,
}

impl From<&domain_eleve::ExigenceDocReinscription> for ExigenceDocDto {
    fn from(e: &domain_eleve::ExigenceDocReinscription) -> Self {
        Self {
            id: e.id,
            annee_cible: e.annee_cible.clone(),
            code: e.code.clone(),
            libelle: e.libelle.clone(),
            tiroir: e.tiroir.as_str().to_string(),
            obligatoire: e.obligatoire,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct PoserExigenceDocBody {
    pub annee_cible: String,
    pub code: String,
    pub libelle: String,
    pub tiroir: Option<String>,
    #[serde(default = "default_true")]
    pub obligatoire: bool,
}

#[derive(Debug, Deserialize)]
pub struct DeposerDocReinscriptionBody {
    pub code: String,
    pub titre: Option<String>,
    #[serde(default)]
    pub mime_type: Option<String>,
    /// Contenu binaire (lab / dépôt famille) — écrit dans l’object store.
    #[serde(default)]
    pub contenu_base64: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ReinscriptionEnfantDto {
    pub enfant: EnfantFamilleDto,
    pub projection_n1: Option<ScolariteAnneeDto>,
    pub options: Vec<OptionAnneeDto>,
    pub confirmee: bool,
    pub documents_manquants: Vec<String>,
    pub exigences: Vec<ExigenceDocDto>,
    pub acompte_requis_centimes: i64,
    pub acompte_encaisse_centimes: i64,
    pub acompte_ok: bool,
}

#[derive(Debug, Serialize)]
pub struct ReinscriptionFamilleDto {
    pub campagne: Option<CampagneDto>,
    pub annee_n: String,
    pub annee_n1: String,
    pub exigences: Vec<ExigenceDocDto>,
    pub enfants: Vec<ReinscriptionEnfantDto>,
}

#[derive(Debug, Serialize)]
pub struct FactureLigneDto {
    pub id: Uuid,
    pub eleve_id: Option<Uuid>,
    pub libelle: String,
    pub quantite: i32,
    pub montant_unitaire_centimes: i64,
    pub total_centimes: i64,
}

impl From<&domain_eleve::FactureLigne> for FactureLigneDto {
    fn from(l: &domain_eleve::FactureLigne) -> Self {
        Self {
            id: l.id,
            eleve_id: l.eleve_id,
            libelle: l.libelle.clone(),
            quantite: l.quantite,
            montant_unitaire_centimes: l.montant_unitaire_centimes,
            total_centimes: l.total_centimes(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct FactureDto {
    pub id: Uuid,
    pub foyer_id: Uuid,
    pub numero: String,
    pub libelle: String,
    pub statut: String,
    pub total_centimes: i64,
    pub nature: Option<String>,
    pub annee_scolaire: Option<String>,
    pub created_at: String,
    pub emise_at: Option<String>,
    pub soldee_at: Option<String>,
}

impl From<&domain_eleve::Facture> for FactureDto {
    fn from(f: &domain_eleve::Facture) -> Self {
        Self {
            id: f.id,
            foyer_id: f.foyer_id,
            numero: f.numero.clone(),
            libelle: f.libelle.clone(),
            statut: f.statut.as_str().to_string(),
            total_centimes: f.total_centimes,
            nature: f.nature.clone(),
            annee_scolaire: f.annee_scolaire.clone(),
            created_at: f.created_at.to_rfc3339(),
            emise_at: f.emise_at.map(|t| t.to_rfc3339()),
            soldee_at: f.soldee_at.map(|t| t.to_rfc3339()),
        }
    }
}

/// Facture foyer parent — inclut soldes pour badge « reste à payer » sans N appels détail.
#[derive(Debug, Serialize)]
pub struct FactureFamilleListItemDto {
    #[serde(flatten)]
    pub facture: FactureDto,
    pub encaisse_centimes: i64,
    pub reste_centimes: i64,
}

#[derive(Debug, Serialize)]
pub struct EncaissementDto {
    pub id: Uuid,
    pub montant_centimes: i64,
    pub moyen: String,
    pub reference: Option<String>,
    pub encaisse_at: String,
}

impl From<&domain_eleve::Encaissement> for EncaissementDto {
    fn from(e: &domain_eleve::Encaissement) -> Self {
        Self {
            id: e.id,
            montant_centimes: e.montant_centimes,
            moyen: e.moyen.as_str().to_string(),
            reference: e.reference.clone(),
            encaisse_at: e.encaisse_at.to_rfc3339(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct EcheanceDto {
    pub id: Uuid,
    pub rang: i32,
    pub libelle: String,
    pub montant_centimes: i64,
    pub bourse_centimes: i64,
    pub montant_brut_centimes: i64,
    pub date_echeance: NaiveDate,
    pub couverture: String,
    pub couvert_centimes: i64,
}

impl From<&domain_eleve::EcheanceVue> for EcheanceDto {
    fn from(v: &domain_eleve::EcheanceVue) -> Self {
        Self {
            id: v.echeance.id,
            rang: v.echeance.rang,
            libelle: v.echeance.libelle.clone(),
            montant_centimes: v.echeance.montant_centimes,
            bourse_centimes: v.echeance.bourse_centimes,
            montant_brut_centimes: v.echeance.montant_brut_centimes(),
            date_echeance: v.echeance.date_echeance,
            couverture: v.couverture.as_str().to_string(),
            couvert_centimes: v.couvert_centimes,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct QuittanceDto {
    pub id: Uuid,
    pub facture_id: Uuid,
    pub encaissement_id: Uuid,
    pub numero: String,
    pub montant_centimes: i64,
    pub moyen: String,
    pub emise_at: String,
}

impl From<&domain_eleve::Quittance> for QuittanceDto {
    fn from(q: &domain_eleve::Quittance) -> Self {
        Self {
            id: q.id,
            facture_id: q.facture_id,
            encaissement_id: q.encaissement_id,
            numero: q.numero.clone(),
            montant_centimes: q.montant_centimes,
            moyen: q.moyen.as_str().to_string(),
            emise_at: q.emise_at.to_rfc3339(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct FactureDetailDto {
    pub facture: FactureDto,
    pub lignes: Vec<FactureLigneDto>,
    pub encaissements: Vec<EncaissementDto>,
    pub echeances: Vec<EcheanceDto>,
    pub quittances: Vec<QuittanceDto>,
    pub prelevements: Vec<PrelevementSepaDto>,
    pub encaisse_centimes: i64,
    pub reste_centimes: i64,
}

#[derive(Debug, Deserialize)]
pub struct CreerFactureLigneBody {
    pub eleve_id: Option<Uuid>,
    pub libelle: String,
    pub quantite: i32,
    pub montant_unitaire_centimes: i64,
}

#[derive(Debug, Deserialize)]
pub struct CreerFactureBody {
    pub foyer_id: Uuid,
    pub numero: String,
    pub libelle: String,
    pub nature: Option<String>,
    pub annee_scolaire: Option<String>,
    pub lignes: Vec<CreerFactureLigneBody>,
}

#[derive(Debug, Deserialize)]
pub struct EncaisserBody {
    pub montant_centimes: i64,
    pub moyen: String,
    pub reference: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct EcheanceBody {
    pub libelle: String,
    pub montant_centimes: i64,
    pub date_echeance: NaiveDate,
}

#[derive(Debug, Deserialize)]
pub struct PoserEcheancierBody {
    pub echeances: Vec<EcheanceBody>,
}

#[derive(Debug, Deserialize)]
pub struct EcheancierPartsBody {
    pub dates: Vec<NaiveDate>,
}

#[derive(Debug, Serialize)]
pub struct MandatSepaDto {
    pub id: Uuid,
    pub foyer_id: Uuid,
    pub rum: String,
    pub iban: String,
    pub bic: Option<String>,
    pub titulaire: String,
    pub statut: String,
    pub signe_at: String,
}

impl From<&domain_eleve::MandatSepa> for MandatSepaDto {
    fn from(m: &domain_eleve::MandatSepa) -> Self {
        Self {
            id: m.id,
            foyer_id: m.foyer_id,
            rum: m.rum.clone(),
            iban: m.iban.clone(),
            bic: m.bic.clone(),
            titulaire: m.titulaire.clone(),
            statut: m.statut.as_str().to_string(),
            signe_at: m.signe_at.to_rfc3339(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreerMandatSepaBody {
    pub foyer_id: Uuid,
    pub rum: String,
    pub iban: String,
    pub bic: Option<String>,
    pub titulaire: String,
}

#[derive(Debug, Serialize)]
pub struct PrelevementSepaDto {
    pub id: Uuid,
    pub mandat_id: Uuid,
    pub facture_id: Uuid,
    pub foyer_id: Uuid,
    pub montant_centimes: i64,
    pub date_execution: NaiveDate,
    pub end_to_end_id: String,
    pub statut: String,
    pub encaissement_id: Option<Uuid>,
    pub motif_rejet: Option<String>,
}

impl From<&domain_eleve::PrelevementSepa> for PrelevementSepaDto {
    fn from(p: &domain_eleve::PrelevementSepa) -> Self {
        Self {
            id: p.id,
            mandat_id: p.mandat_id,
            facture_id: p.facture_id,
            foyer_id: p.foyer_id,
            montant_centimes: p.montant_centimes,
            date_execution: p.date_execution,
            end_to_end_id: p.end_to_end_id.clone(),
            statut: p.statut.as_str().to_string(),
            encaissement_id: p.encaissement_id,
            motif_rejet: p.motif_rejet.clone(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct PlanifierPrelevementBody {
    pub mandat_id: Uuid,
    pub montant_centimes: i64,
    pub date_execution: NaiveDate,
}

#[derive(Debug, Deserialize)]
pub struct RejeterPrelevementBody {
    pub motif: String,
}

#[derive(Debug, Serialize)]
pub struct RappelFactureDto {
    pub id: Uuid,
    pub foyer_id: Uuid,
    pub facture_id: Uuid,
    pub echeance_id: Uuid,
    pub kind: String,
    pub canal: String,
    pub corps: String,
    pub fil_id: Option<Uuid>,
    pub envoye_at: String,
}

impl From<&domain_eleve::RappelFacture> for RappelFactureDto {
    fn from(r: &domain_eleve::RappelFacture) -> Self {
        Self {
            id: r.id,
            foyer_id: r.foyer_id,
            facture_id: r.facture_id,
            echeance_id: r.echeance_id,
            kind: r.kind.as_str().to_string(),
            canal: r.canal.as_str().to_string(),
            corps: r.corps.clone(),
            fil_id: r.fil_id,
            envoye_at: r.envoye_at.to_rfc3339(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct SignalAbsenceDto {
    pub id: Uuid,
    pub eleve_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eleve_label: Option<String>,
    pub annee_scolaire: String,
    pub seuil: i32,
    pub compteur: i32,
    pub statut: String,
    pub note_vs: Option<String>,
    pub traite_par_user_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub traite_par_label: Option<String>,
    pub fil_alerte_id: Option<Uuid>,
    pub commission_dossier_id: Option<Uuid>,
}

impl From<&domain_eleve::SignalAbsence> for SignalAbsenceDto {
    fn from(s: &domain_eleve::SignalAbsence) -> Self {
        Self {
            id: s.id,
            eleve_id: s.eleve_id,
            eleve_label: None,
            annee_scolaire: s.annee_scolaire.clone(),
            seuil: s.seuil,
            compteur: s.compteur,
            statut: s.statut.as_str().to_string(),
            note_vs: s.note_vs.clone(),
            traite_par_user_id: s.traite_par_user_id,
            traite_par_label: None,
            fil_alerte_id: s.fil_alerte_id,
            commission_dossier_id: s.commission_dossier_id,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CloreSignalBody {
    pub note: String,
}

#[derive(Debug, Deserialize)]
pub struct CantineBadgeFactureBody {
    pub eleve_id: Uuid,
    pub foyer_id: Uuid,
    pub debut: NaiveDate,
    pub fin: NaiveDate,
    pub prix_unitaire_centimes: i64,
    pub numero: String,
    #[serde(default = "default_midi_kind")]
    pub kind: String,
}

#[derive(Debug, Serialize)]
pub struct CantineBadgeApercuDto {
    pub eleve_id: Uuid,
    pub debut: NaiveDate,
    pub fin: NaiveDate,
    pub nb_passages: u32,
    pub prix_unitaire_centimes: i64,
    pub total_centimes: i64,
}

#[derive(Debug, Deserialize)]
pub struct ScanBadgeRepasBody {
    pub badge: String,
    #[serde(default = "default_midi_kind")]
    pub kind: String,
    pub jour: Option<NaiveDate>,
}

fn default_midi_kind() -> String {
    "midi".into()
}

#[derive(Debug, Serialize)]
pub struct PassageRepasDto {
    pub id: Uuid,
    pub eleve_id: Uuid,
    pub activite_id: Uuid,
    pub jour: NaiveDate,
    pub badge_lu: String,
    pub source: String,
    pub eleve_nom: Option<String>,
    pub eleve_prenom: Option<String>,
}

impl From<&domain_eleve::PassageRepas> for PassageRepasDto {
    fn from(p: &domain_eleve::PassageRepas) -> Self {
        Self {
            id: p.id,
            eleve_id: p.eleve_id,
            activite_id: p.activite_id,
            jour: p.jour,
            badge_lu: p.badge_lu.clone(),
            source: p.source.as_str().to_string(),
            eleve_nom: None,
            eleve_prenom: None,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JoursMasqueDto {
    pub lun: bool,
    pub mar: bool,
    pub mer: bool,
    pub jeu: bool,
    pub ven: bool,
    pub sam: bool,
    pub dim: bool,
}

impl From<&domain_eleve::JoursMasque> for JoursMasqueDto {
    fn from(j: &domain_eleve::JoursMasque) -> Self {
        Self {
            lun: j.lun,
            mar: j.mar,
            mer: j.mer,
            jeu: j.jeu,
            ven: j.ven,
            sam: j.sam,
            dim: j.dim,
        }
    }
}

impl From<&JoursMasqueDto> for domain_eleve::JoursMasque {
    fn from(j: &JoursMasqueDto) -> Self {
        Self {
            lun: j.lun,
            mar: j.mar,
            mer: j.mer,
            jeu: j.jeu,
            ven: j.ven,
            sam: j.sam,
            dim: j.dim,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct RepasActiviteDto {
    pub id: Uuid,
    pub code: String,
    pub libelle: String,
    pub kind: String,
    pub actif: bool,
}

impl From<&domain_eleve::RepasActivite> for RepasActiviteDto {
    fn from(a: &domain_eleve::RepasActivite) -> Self {
        Self {
            id: a.id,
            code: a.code.clone(),
            libelle: a.libelle.clone(),
            kind: a.kind.as_str().to_string(),
            actif: a.actif,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreerRepasActiviteBody {
    pub code: String,
    pub libelle: String,
    pub kind: String,
}

#[derive(Debug, Serialize)]
pub struct GrilleRepasDto {
    pub id: Uuid,
    pub eleve_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eleve_label: Option<String>,
    pub activite_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub activite_label: Option<String>,
    pub jours: JoursMasqueDto,
    pub date_debut: NaiveDate,
    pub date_fin: Option<NaiveDate>,
}

impl From<&domain_eleve::EleveGrilleRepas> for GrilleRepasDto {
    fn from(g: &domain_eleve::EleveGrilleRepas) -> Self {
        Self {
            id: g.id,
            eleve_id: g.eleve_id,
            eleve_label: None,
            activite_id: g.activite_id,
            activite_label: None,
            jours: JoursMasqueDto::from(&g.jours),
            date_debut: g.date_debut,
            date_fin: g.date_fin,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct PoserGrilleRepasBody {
    pub activite_id: Uuid,
    pub jours: JoursMasqueDto,
    pub date_debut: NaiveDate,
}

#[derive(Debug, Deserialize)]
pub struct PoserDispenseRepasBody {
    pub activite_id: Uuid,
    pub jour: NaiveDate,
    pub motif: String,
}

#[derive(Debug, Serialize)]
pub struct BourseTypeDto {
    pub id: Uuid,
    pub code: String,
    pub libelle: String,
    pub actif: bool,
}

impl From<&domain_eleve::BourseType> for BourseTypeDto {
    fn from(t: &domain_eleve::BourseType) -> Self {
        Self {
            id: t.id,
            code: t.code.clone(),
            libelle: t.libelle.clone(),
            actif: t.actif,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreerBourseTypeBody {
    pub code: String,
    pub libelle: String,
}

#[derive(Debug, Serialize)]
pub struct EleveBourseDto {
    pub id: Uuid,
    pub eleve_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eleve_label: Option<String>,
    pub bourse_type_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bourse_type_label: Option<String>,
    pub echelon: String,
    pub avec_procuration: bool,
    pub status: String,
    pub date_debut: NaiveDate,
    pub date_fin: Option<NaiveDate>,
}

impl From<&domain_eleve::EleveBourse> for EleveBourseDto {
    fn from(b: &domain_eleve::EleveBourse) -> Self {
        Self {
            id: b.id,
            eleve_id: b.eleve_id,
            eleve_label: None,
            bourse_type_id: b.bourse_type_id,
            bourse_type_label: None,
            echelon: b.echelon.clone(),
            avec_procuration: b.avec_procuration,
            status: b.status.as_str().to_string(),
            date_debut: b.date_debut,
            date_fin: b.date_fin,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct AffecterBourseBody {
    pub bourse_type_id: Uuid,
    pub echelon: String,
    pub avec_procuration: bool,
    pub date_debut: NaiveDate,
}

#[derive(Debug, Deserialize)]
pub struct DeciderBourseBody {
    pub accepter: bool,
}

#[derive(Debug, Serialize)]
pub struct DispenseRepasDto {
    pub id: Uuid,
    pub eleve_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eleve_label: Option<String>,
    pub activite_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub activite_label: Option<String>,
    pub jour: NaiveDate,
    pub motif: String,
    pub source: String,
    pub annulee: bool,
}

impl From<&domain_eleve::EleveDispenseRepas> for DispenseRepasDto {
    fn from(d: &domain_eleve::EleveDispenseRepas) -> Self {
        Self {
            id: d.id,
            eleve_id: d.eleve_id,
            eleve_label: None,
            activite_id: d.activite_id,
            activite_label: None,
            jour: d.jour,
            motif: d.motif.clone(),
            source: d.source.as_str().to_string(),
            annulee: d.annulee,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct VolumeRepasDto {
    pub eleve_id: Uuid,
    pub kind: String,
    pub debut: NaiveDate,
    pub fin: NaiveDate,
    pub jours_facturables: u32,
}

#[derive(Debug, Deserialize)]
pub struct UpsertAutorisationSortieBody {
    pub matin: bool,
    pub midi: bool,
    pub soir: bool,
    #[serde(default)]
    pub interdit_etablissement: bool,
}

#[derive(Debug, Serialize)]
pub struct AutorisationSortieDto {
    pub id: Uuid,
    pub eleve_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eleve_label: Option<String>,
    pub matin: bool,
    pub midi: bool,
    pub soir: bool,
    pub interdit_etablissement: bool,
}

impl From<&domain_eleve::AutorisationSortie> for AutorisationSortieDto {
    fn from(a: &domain_eleve::AutorisationSortie) -> Self {
        Self {
            id: a.id,
            eleve_id: a.eleve_id,
            eleve_label: None,
            matin: a.matin,
            midi: a.midi,
            soir: a.soir,
            interdit_etablissement: a.interdit_etablissement,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct DemanderSortieBody {
    pub jour: NaiveDate,
    pub creneau: String,
    pub motif: String,
}

#[derive(Debug, Deserialize)]
pub struct DeciderSortieBody {
    pub accepter: bool,
    pub decision_note: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct DemandeSortieDto {
    pub id: Uuid,
    pub eleve_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eleve_label: Option<String>,
    pub jour: NaiveDate,
    pub creneau: String,
    pub motif: String,
    pub statut: String,
    pub decision_note: String,
}

impl From<&domain_eleve::DemandeSortieExceptionnelle> for DemandeSortieDto {
    fn from(d: &domain_eleve::DemandeSortieExceptionnelle) -> Self {
        Self {
            id: d.id,
            eleve_id: d.eleve_id,
            eleve_label: None,
            jour: d.jour,
            creneau: d.creneau.as_str().to_string(),
            motif: d.motif.clone(),
            statut: d.statut.as_str().to_string(),
            decision_note: d.decision_note.clone(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct RecapSortieDto {
    pub eleve_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eleve_label: Option<String>,
    pub autorisation: Option<AutorisationSortieDto>,
    pub demandes_ouvertes: Vec<DemandeSortieDto>,
}

impl From<&domain_eleve::RecapSortieEleve> for RecapSortieDto {
    fn from(r: &domain_eleve::RecapSortieEleve) -> Self {
        Self {
            eleve_id: r.eleve_id,
            eleve_label: None,
            autorisation: r.autorisation.as_ref().map(AutorisationSortieDto::from),
            demandes_ouvertes: r
                .demandes_ouvertes
                .iter()
                .map(DemandeSortieDto::from)
                .collect(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct ControleSortieBody {
    pub eleve_id: Uuid,
    pub jour: Option<NaiveDate>,
    pub creneau: String,
    /// Site du portail de contrôle (ex. « SiteA ») — pour règle changement de site EDT.
    pub site_portail: Option<String>,
    /// Heure locale `HH:MM` (défaut = maintenant UTC).
    pub heure: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpsertSortieParamBody {
    pub autoriser_changement_site: bool,
}

#[derive(Debug, Serialize)]
pub struct SortieParamDto {
    pub autoriser_changement_site: bool,
}

#[derive(Debug, Deserialize)]
pub struct UpsertSortieRegleBody {
    pub niveau: String,
    pub matin_autorise: bool,
    pub midi_autorise: bool,
    pub soir_autorise: bool,
    #[serde(default)]
    pub choix_parent_interdit: bool,
}

#[derive(Debug, Serialize)]
pub struct SortieRegleNiveauDto {
    pub id: Uuid,
    pub niveau: String,
    pub matin_autorise: bool,
    pub midi_autorise: bool,
    pub soir_autorise: bool,
    pub choix_parent_interdit: bool,
}

impl From<&domain_eleve::SortieRegleNiveau> for SortieRegleNiveauDto {
    fn from(r: &domain_eleve::SortieRegleNiveau) -> Self {
        Self {
            id: r.id,
            niveau: r.niveau.clone(),
            matin_autorise: r.matin_autorise,
            midi_autorise: r.midi_autorise,
            soir_autorise: r.soir_autorise,
            choix_parent_interdit: r.choix_parent_interdit,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct UpsertSortieRegleRegimeBody {
    pub regime: String,
    pub matin_autorise: bool,
    pub midi_autorise: bool,
    pub soir_autorise: bool,
    #[serde(default)]
    pub choix_parent_interdit: bool,
}

#[derive(Debug, Serialize)]
pub struct SortieRegleRegimeDto {
    pub id: Uuid,
    pub regime: String,
    pub matin_autorise: bool,
    pub midi_autorise: bool,
    pub soir_autorise: bool,
    pub choix_parent_interdit: bool,
}

impl From<&domain_eleve::SortieRegleRegime> for SortieRegleRegimeDto {
    fn from(r: &domain_eleve::SortieRegleRegime) -> Self {
        Self {
            id: r.id,
            regime: r.regime.as_str().to_string(),
            matin_autorise: r.matin_autorise,
            midi_autorise: r.midi_autorise,
            soir_autorise: r.soir_autorise,
            choix_parent_interdit: r.choix_parent_interdit,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ControleSortieDto {
    pub id: Uuid,
    pub eleve_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eleve_label: Option<String>,
    pub jour: NaiveDate,
    pub creneau: String,
    pub verdict: String,
    pub source: String,
    pub motif: String,
}

impl From<&domain_eleve::ControleSortie> for ControleSortieDto {
    fn from(c: &domain_eleve::ControleSortie) -> Self {
        Self {
            id: c.id,
            eleve_id: c.eleve_id,
            eleve_label: None,
            jour: c.jour,
            creneau: c.creneau.as_str().to_string(),
            verdict: c.verdict.as_str().to_string(),
            source: c.source.as_str().to_string(),
            motif: c.motif.clone(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreerChambreInternatBody {
    pub batiment: String,
    #[serde(default)]
    pub etage: i16,
    pub code: String,
    pub capacite: i16,
    pub genre: String,
}

#[derive(Debug, Serialize)]
pub struct ChambreInternatDto {
    pub id: Uuid,
    pub batiment: String,
    pub etage: i16,
    pub code: String,
    pub capacite: i16,
    pub genre: String,
    pub actif: bool,
}

impl From<&domain_eleve::ChambreInternat> for ChambreInternatDto {
    fn from(c: &domain_eleve::ChambreInternat) -> Self {
        Self {
            id: c.id,
            batiment: c.batiment.clone(),
            etage: c.etage,
            code: c.code.clone(),
            capacite: c.capacite,
            genre: c.genre.as_str().to_string(),
            actif: c.actif,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct AffecterInternatBody {
    pub eleve_id: Uuid,
    pub chambre_id: Uuid,
    pub lit: Option<String>,
    pub debut: NaiveDate,
}

#[derive(Debug, Serialize)]
pub struct AffectationInternatDto {
    pub id: Uuid,
    pub eleve_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eleve_label: Option<String>,
    pub chambre_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chambre_label: Option<String>,
    pub lit: Option<String>,
    pub debut: NaiveDate,
    pub fin: Option<NaiveDate>,
}

impl From<&domain_eleve::AffectationInternat> for AffectationInternatDto {
    fn from(a: &domain_eleve::AffectationInternat) -> Self {
        Self {
            id: a.id,
            eleve_id: a.eleve_id,
            eleve_label: None,
            chambre_id: a.chambre_id,
            chambre_label: None,
            lit: a.lit.clone(),
            debut: a.debut,
            fin: a.fin,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CloreAffectationInternatBody {
    pub fin: NaiveDate,
}

#[derive(Debug, Deserialize)]
pub struct PointerPresenceInternatBody {
    pub eleve_id: Uuid,
    pub jour: Option<NaiveDate>,
    pub statut: String,
    #[serde(default)]
    pub note: String,
}

#[derive(Debug, Serialize)]
pub struct PresenceInternatDto {
    pub id: Uuid,
    pub eleve_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eleve_label: Option<String>,
    pub jour: NaiveDate,
    pub statut: String,
    pub note: String,
}

impl From<&domain_eleve::PresenceInternat> for PresenceInternatDto {
    fn from(p: &domain_eleve::PresenceInternat) -> Self {
        Self {
            id: p.id,
            eleve_id: p.eleve_id,
            eleve_label: None,
            jour: p.jour,
            statut: p.statut.as_str().to_string(),
            note: p.note.clone(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreerSuiviInternatBody {
    pub jour: Option<NaiveDate>,
    pub texte: String,
    #[serde(default)]
    pub visible_famille: bool,
}

#[derive(Debug, Serialize)]
pub struct SuiviInternatDto {
    pub id: Uuid,
    pub eleve_id: Uuid,
    pub jour: NaiveDate,
    pub texte: String,
    pub visible_famille: bool,
}

impl From<&domain_eleve::SuiviInternat> for SuiviInternatDto {
    fn from(s: &domain_eleve::SuiviInternat) -> Self {
        Self {
            id: s.id,
            eleve_id: s.eleve_id,
            jour: s.jour,
            texte: s.texte.clone(),
            visible_famille: s.visible_famille,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct RecapInternatDto {
    pub eleve_id: Uuid,
    pub affectation: Option<AffectationInternatDto>,
    pub chambre: Option<ChambreInternatDto>,
    pub presence_du_jour: Option<PresenceInternatDto>,
    pub suivis: Vec<SuiviInternatDto>,
}

#[derive(Debug, Deserialize)]
pub struct PoserDispenseVsBody {
    pub motif: String,
    pub debut: NaiveDate,
    pub fin: NaiveDate,
    #[serde(default)]
    pub commentaire: String,
}

#[derive(Debug, Serialize)]
pub struct DispenseVsDto {
    pub id: Uuid,
    pub eleve_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eleve_label: Option<String>,
    pub motif: String,
    pub debut: NaiveDate,
    pub fin: NaiveDate,
    pub commentaire: String,
    pub statut: String,
}

impl From<&domain_eleve::DispenseVs> for DispenseVsDto {
    fn from(d: &domain_eleve::DispenseVs) -> Self {
        Self {
            id: d.id,
            eleve_id: d.eleve_id,
            eleve_label: None,
            motif: d.motif.as_str().to_string(),
            debut: d.debut,
            fin: d.fin,
            commentaire: d.commentaire.clone(),
            statut: d.statut.as_str().to_string(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct JournalEntreeDto {
    pub id: Uuid,
    pub domaine: String,
    pub action: String,
    pub libelle: String,
    pub visible_famille: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl From<&domain_eleve::JournalEntree> for JournalEntreeDto {
    fn from(e: &domain_eleve::JournalEntree) -> Self {
        Self {
            id: e.id,
            domaine: e.domaine.clone(),
            action: e.action.clone(),
            libelle: e.libelle.clone(),
            visible_famille: e.visible_famille,
            created_at: e.created_at,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreerAlerteCpeBody {
    pub titre: String,
    #[serde(default)]
    pub detail: String,
    pub severite: String,
    #[serde(default)]
    pub visible_famille: bool,
    pub jour: Option<NaiveDate>,
}

#[derive(Debug, Deserialize)]
pub struct CloreAlerteCpeBody {
    #[serde(default)]
    pub note: String,
}

#[derive(Debug, Serialize)]
pub struct AlerteCpeDto {
    pub id: Uuid,
    pub eleve_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eleve_label: Option<String>,
    pub titre: String,
    pub detail: String,
    pub severite: String,
    pub statut: String,
    pub visible_famille: bool,
    pub jour: NaiveDate,
    pub cloture_note: String,
}

impl From<&domain_eleve::AlerteCpe> for AlerteCpeDto {
    fn from(a: &domain_eleve::AlerteCpe) -> Self {
        Self {
            id: a.id,
            eleve_id: a.eleve_id,
            eleve_label: None,
            titre: a.titre.clone(),
            detail: a.detail.clone(),
            severite: a.severite.as_str().to_string(),
            statut: a.statut.as_str().to_string(),
            visible_famille: a.visible_famille,
            jour: a.jour,
            cloture_note: a.cloture_note.clone(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct PublierDocASignerBody {
    pub code: String,
    pub titre: String,
    pub corps: String,
    pub annee_scolaire: String,
    pub echeance: Option<NaiveDate>,
}

#[derive(Debug, Serialize)]
pub struct DocASignerDto {
    pub id: Uuid,
    pub eleve_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eleve_label: Option<String>,
    pub code: String,
    pub titre: String,
    pub corps: String,
    pub annee_scolaire: String,
    pub statut: String,
    pub echeance: Option<NaiveDate>,
    pub signe_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl From<&domain_eleve::DocASigner> for DocASignerDto {
    fn from(d: &domain_eleve::DocASigner) -> Self {
        Self {
            id: d.id,
            eleve_id: d.eleve_id,
            eleve_label: None,
            code: d.code.clone(),
            titre: d.titre.clone(),
            corps: d.corps.clone(),
            annee_scolaire: d.annee_scolaire.clone(),
            statut: d.statut.as_str().to_string(),
            echeance: d.echeance,
            signe_at: d.signe_at,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreerConseilClasseBody {
    pub classe: String,
    pub periode_libelle: String,
    pub jour: NaiveDate,
    pub heure: String,
    #[serde(default)]
    pub lieu: String,
}

#[derive(Debug, Serialize)]
pub struct ConseilClasseDto {
    pub id: Uuid,
    pub classe: String,
    pub periode_libelle: String,
    pub jour: NaiveDate,
    pub heure: String,
    pub lieu: String,
    pub statut: String,
}

impl From<&domain_eleve::ConseilClasse> for ConseilClasseDto {
    fn from(c: &domain_eleve::ConseilClasse) -> Self {
        Self {
            id: c.id,
            classe: c.classe.clone(),
            periode_libelle: c.periode_libelle.clone(),
            jour: c.jour,
            heure: c.heure.format("%H:%M").to_string(),
            lieu: c.lieu.clone(),
            statut: c.statut.as_str().to_string(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct AjouterDossierConseilBody {
    pub eleve_id: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct DeciderDossierConseilBody {
    pub appreciation: String,
    pub mention: String,
    #[serde(default)]
    pub decision_texte: String,
    #[serde(default)]
    pub visible_famille: bool,
}

#[derive(Debug, Serialize)]
pub struct ConseilClasseDossierDto {
    pub id: Uuid,
    pub conseil_id: Uuid,
    pub eleve_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eleve_label: Option<String>,
    pub appreciation: String,
    pub mention: Option<String>,
    pub decision_texte: String,
    pub visible_famille: bool,
    pub decided_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl From<&domain_eleve::ConseilClasseDossier> for ConseilClasseDossierDto {
    fn from(d: &domain_eleve::ConseilClasseDossier) -> Self {
        Self {
            id: d.id,
            conseil_id: d.conseil_id,
            eleve_id: d.eleve_id,
            eleve_label: None,
            appreciation: d.appreciation.clone(),
            mention: d.mention.map(|m| m.as_str().to_string()),
            decision_texte: d.decision_texte.clone(),
            visible_famille: d.visible_famille,
            decided_at: d.decided_at,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreerInfoCompChampBody {
    pub code: String,
    pub libelle: String,
    pub type_valeur: String,
    #[serde(default)]
    pub obligatoire: bool,
    #[serde(default)]
    pub visible_famille: bool,
}

#[derive(Debug, Serialize)]
pub struct InfoCompChampDto {
    pub id: Uuid,
    pub code: String,
    pub libelle: String,
    pub type_valeur: String,
    pub obligatoire: bool,
    pub visible_famille: bool,
}

impl From<&domain_eleve::InfoCompChamp> for InfoCompChampDto {
    fn from(c: &domain_eleve::InfoCompChamp) -> Self {
        Self {
            id: c.id,
            code: c.code.clone(),
            libelle: c.libelle.clone(),
            type_valeur: c.type_valeur.as_str().to_string(),
            obligatoire: c.obligatoire,
            visible_famille: c.visible_famille,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct UpsertInfoCompValeurBody {
    pub champ_id: Uuid,
    pub valeur: String,
}

#[derive(Debug, Serialize)]
pub struct InfoCompValeurDto {
    pub champ_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub champ_label: Option<String>,
    pub valeur: String,
}

impl From<&domain_eleve::InfoCompValeur> for InfoCompValeurDto {
    fn from(v: &domain_eleve::InfoCompValeur) -> Self {
        Self {
            champ_id: v.champ_id,
            champ_label: None,
            valeur: v.valeur.clone(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct InfoCompEleveDto {
    pub champs: Vec<InfoCompChampDto>,
    pub valeurs: Vec<InfoCompValeurDto>,
}

#[derive(Debug, Deserialize)]
pub struct CreerPeriodeVacancesBody {
    pub libelle: String,
    pub kind: String,
    pub debut: NaiveDate,
    pub fin: NaiveDate,
}

#[derive(Debug, Serialize)]
pub struct PeriodeVacancesDto {
    pub id: Uuid,
    pub libelle: String,
    pub kind: String,
    pub debut: NaiveDate,
    pub fin: NaiveDate,
}

impl From<&domain_eleve::PeriodeVacances> for PeriodeVacancesDto {
    fn from(p: &domain_eleve::PeriodeVacances) -> Self {
        Self {
            id: p.id,
            libelle: p.libelle.clone(),
            kind: p.kind.as_str().to_string(),
            debut: p.debut,
            fin: p.fin,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct JourFermeDto {
    pub jour: NaiveDate,
    pub ferme: bool,
}

#[derive(Debug, Deserialize)]
pub struct RechercherHomonymesBody {
    pub nom: String,
    pub prenom: String,
    pub date_naissance: Option<NaiveDate>,
}

#[derive(Debug, Serialize)]
pub struct HomonymeCandidatDto {
    pub eleve_id: Uuid,
    pub nom: String,
    pub prenom: String,
    pub date_naissance: Option<NaiveDate>,
    pub classe: Option<String>,
    pub score: u8,
}

impl From<&domain_eleve::HomonymeCandidat> for HomonymeCandidatDto {
    fn from(c: &domain_eleve::HomonymeCandidat) -> Self {
        Self {
            eleve_id: c.eleve_id,
            nom: c.nom.clone(),
            prenom: c.prenom.clone(),
            date_naissance: c.date_naissance,
            classe: c.classe.clone(),
            score: c.score,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct UpsertMenuCantineBody {
    pub jour: NaiveDate,
    pub entree: String,
    pub plat: String,
    pub dessert: String,
    #[serde(default)]
    pub notes: String,
    #[serde(default)]
    pub publie: bool,
}

#[derive(Debug, Serialize)]
pub struct MenuCantineDto {
    pub id: Uuid,
    pub jour: NaiveDate,
    pub entree: String,
    pub plat: String,
    pub dessert: String,
    pub notes: String,
    pub publie: bool,
}

impl From<&domain_eleve::MenuCantineJour> for MenuCantineDto {
    fn from(m: &domain_eleve::MenuCantineJour) -> Self {
        Self {
            id: m.id,
            jour: m.jour,
            entree: m.entree.clone(),
            plat: m.plat.clone(),
            dessert: m.dessert.clone(),
            notes: m.notes.clone(),
            publie: m.publie,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct UpsertQuotientBody {
    pub foyer_id: Uuid,
    pub annee_scolaire: String,
    pub quotient_centimes: i64,
    pub parts_x100: i32,
}

#[derive(Debug, Serialize)]
pub struct QuotientFoyerDto {
    pub id: Uuid,
    pub foyer_id: Uuid,
    pub annee_scolaire: String,
    pub quotient_centimes: i64,
    pub parts_x100: i32,
}

impl From<&domain_eleve::QuotientFoyer> for QuotientFoyerDto {
    fn from(q: &domain_eleve::QuotientFoyer) -> Self {
        Self {
            id: q.id,
            foyer_id: q.foyer_id,
            annee_scolaire: q.annee_scolaire.clone(),
            quotient_centimes: q.quotient_centimes,
            parts_x100: q.parts_x100,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreerTauxTvaBody {
    pub code: String,
    pub libelle: String,
    pub taux_bps: i32,
}

#[derive(Debug, Serialize)]
pub struct TauxTvaDto {
    pub id: Uuid,
    pub code: String,
    pub libelle: String,
    pub taux_bps: i32,
}

impl From<&domain_eleve::TauxTva> for TauxTvaDto {
    fn from(t: &domain_eleve::TauxTva) -> Self {
        Self {
            id: t.id,
            code: t.code.clone(),
            libelle: t.libelle.clone(),
            taux_bps: t.taux_bps,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct CalculTtcDto {
    pub ht_centimes: i64,
    pub taux_bps: i32,
    pub ttc_centimes: i64,
}

#[derive(Debug, Serialize)]
pub struct TrombiEntreeDto {
    pub eleve_id: Uuid,
    pub nom: String,
    pub prenom: String,
    pub classe: Option<String>,
    pub photo_key: Option<String>,
}

impl From<&domain_eleve::TrombiEntree> for TrombiEntreeDto {
    fn from(t: &domain_eleve::TrombiEntree) -> Self {
        Self {
            eleve_id: t.eleve_id,
            nom: t.nom.clone(),
            prenom: t.prenom.clone(),
            classe: t.classe.clone(),
            photo_key: t.photo_key.clone(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct UpsertParamEtabBody {
    pub cle: String,
    pub valeur: String,
}

#[derive(Debug, Serialize)]
pub struct ParamEtabDto {
    pub cle: String,
    pub valeur: String,
}

impl From<&domain_eleve::ParamEtab> for ParamEtabDto {
    fn from(p: &domain_eleve::ParamEtab) -> Self {
        Self {
            cle: p.cle.clone(),
            valeur: p.valeur.clone(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreerSalleBody {
    pub code: String,
    pub libelle: String,
    pub capacite: Option<i32>,
    pub site: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct AbsenceMotifDto {
    pub code: String,
    pub libelle: String,
    pub etablissement_id: Option<Uuid>,
}

impl From<&AbsenceMotif> for AbsenceMotifDto {
    fn from(m: &AbsenceMotif) -> Self {
        Self {
            code: m.code.clone(),
            libelle: m.libelle.clone(),
            etablissement_id: m.etablissement_id,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreerAbsenceMotifBody {
    pub code: String,
    pub libelle: String,
}

#[derive(Debug, Serialize)]
pub struct DocumentMetaDto {
    pub id: Uuid,
    pub eleve_id: Uuid,
    pub titre: String,
    pub tiroir: String,
    pub confidentialite: String,
    pub source: String,
    pub code: Option<String>,
    pub accessible: bool,
    /// Dernière demande d'accès (grant) de l'utilisateur courant sur un doc verrouillé.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grant_etat: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct DeposerDocumentBody {
    pub titre: String,
    pub tiroir: String,
    pub confidentialite: String,
    pub code: Option<String>,
    pub mime_type: Option<String>,
    pub storage_key: Option<String>,
    /// Contenu binaire (lab / dépôt famille) — écrit dans l’object store.
    #[serde(default)]
    pub contenu_base64: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct DocumentGrantDto {
    pub id: Uuid,
    pub document_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub document_titre: Option<String>,
    pub grantee_user_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grantee_label: Option<String>,
    pub status: String,
    pub duration_days: i32,
    pub expires_at: Option<String>,
    pub note: Option<String>,
}

impl From<&domain_eleve::DocumentGrant> for DocumentGrantDto {
    fn from(g: &domain_eleve::DocumentGrant) -> Self {
        Self {
            id: g.id,
            document_id: g.document_id,
            document_titre: None,
            grantee_user_id: g.grantee_user_id,
            grantee_label: None,
            status: g.status.as_str().to_string(),
            duration_days: g.duration_days,
            expires_at: g.expires_at.map(|t| t.to_rfc3339()),
            note: g.note.clone(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct DemanderGrantBody {
    pub duration_days: i32,
    pub note: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct DeciderGrantBody {
    pub accepter: bool,
    pub note: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SalleDto {
    pub id: Uuid,
    pub code: String,
    pub libelle: String,
    pub capacite: Option<i32>,
    pub site: Option<String>,
}

impl From<&domain_eleve::Salle> for SalleDto {
    fn from(s: &domain_eleve::Salle) -> Self {
        Self {
            id: s.id,
            code: s.code.clone(),
            libelle: s.libelle.clone(),
            capacite: s.capacite,
            site: s.site.clone(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct MailOutboxDto {
    pub id: Uuid,
    pub eleve_id: Option<Uuid>,
    pub kind: String,
    pub to_email: String,
    pub subject: String,
    pub status: String,
    pub error: Option<String>,
}

impl From<&MailOutboxRow> for MailOutboxDto {
    fn from(m: &MailOutboxRow) -> Self {
        Self {
            id: m.id,
            eleve_id: m.eleve_id,
            kind: m.kind.clone(),
            to_email: m.to_email.clone(),
            subject: m.subject.clone(),
            status: m.status.clone(),
            error: m.error.clone(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct SmsOutboxDto {
    pub id: Uuid,
    pub eleve_id: Option<Uuid>,
    pub kind: String,
    pub to_phone: String,
    pub body: String,
    pub status: String,
    pub error: Option<String>,
}

impl From<&SmsOutboxRow> for SmsOutboxDto {
    fn from(s: &SmsOutboxRow) -> Self {
        Self {
            id: s.id,
            eleve_id: s.eleve_id,
            kind: s.kind.clone(),
            to_phone: s.to_phone.clone(),
            body: s.body.clone(),
            status: s.status.clone(),
            error: s.error.clone(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreerFiliereBody {
    pub code: String,
    pub libelle: String,
    pub mef: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct FiliereDto {
    pub id: Uuid,
    pub code: String,
    pub libelle: String,
    pub mef: Option<String>,
}

impl From<&domain_eleve::Filiere> for FiliereDto {
    fn from(f: &domain_eleve::Filiere) -> Self {
        Self {
            id: f.id,
            code: f.code.clone(),
            libelle: f.libelle.clone(),
            mef: f.mef.clone(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct PoserVoeuBody {
    pub filiere_id: Uuid,
    pub rang: i16,
    pub annee_scolaire: String,
}

#[derive(Debug, Serialize)]
pub struct VoeuDto {
    pub id: Uuid,
    pub eleve_id: Uuid,
    pub filiere_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filiere_label: Option<String>,
    pub rang: i16,
    pub annee_scolaire: String,
}

impl From<&domain_eleve::VoeuInscription> for VoeuDto {
    fn from(v: &domain_eleve::VoeuInscription) -> Self {
        Self {
            id: v.id,
            eleve_id: v.eleve_id,
            filiere_id: v.filiere_id,
            filiere_label: None,
            rang: v.rang,
            annee_scolaire: v.annee_scolaire.clone(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct UpsertInscriptionSuiviBody {
    pub annee_scolaire: String,
    pub etape: String,
    pub motif_code: Option<String>,
    #[serde(default)]
    pub commentaire: String,
}

#[derive(Debug, Serialize)]
pub struct InscriptionSuiviDto {
    pub id: Uuid,
    pub eleve_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eleve_label: Option<String>,
    pub annee_scolaire: String,
    pub etape: String,
    pub motif_code: Option<String>,
    pub commentaire: String,
}

impl From<&domain_eleve::InscriptionSuivi> for InscriptionSuiviDto {
    fn from(s: &domain_eleve::InscriptionSuivi) -> Self {
        Self {
            id: s.id,
            eleve_id: s.eleve_id,
            eleve_label: None,
            annee_scolaire: s.annee_scolaire.clone(),
            etape: s.etape.as_str().to_string(),
            motif_code: s.motif_code.clone(),
            commentaire: s.commentaire.clone(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct IbanHistoDto {
    pub id: Uuid,
    pub foyer_id: Uuid,
    pub iban: String,
    pub bic: Option<String>,
    pub titulaire: String,
    pub mandat_id: Option<Uuid>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl From<&IbanHisto> for IbanHistoDto {
    fn from(h: &IbanHisto) -> Self {
        Self {
            id: h.id,
            foyer_id: h.foyer_id,
            iban: h.iban.clone(),
            bic: h.bic.clone(),
            titulaire: h.titulaire.clone(),
            mandat_id: h.mandat_id,
            created_at: h.created_at,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreerTarifConsoBody {
    pub code: String,
    pub libelle: String,
    pub montant_centimes: i64,
    pub debut: NaiveDate,
    pub fin: Option<NaiveDate>,
}

#[derive(Debug, Serialize)]
pub struct TarifConsoDto {
    pub id: Uuid,
    pub code: String,
    pub libelle: String,
    pub montant_centimes: i64,
    pub debut: NaiveDate,
    pub fin: Option<NaiveDate>,
}

impl From<&domain_eleve::TarifConso> for TarifConsoDto {
    fn from(t: &domain_eleve::TarifConso) -> Self {
        Self {
            id: t.id,
            code: t.code.clone(),
            libelle: t.libelle.clone(),
            montant_centimes: t.montant_centimes,
            debut: t.debut,
            fin: t.fin,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct DeclarerAbsenceRhBody {
    pub user_id: Option<Uuid>,
    pub debut: NaiveDate,
    pub fin: NaiveDate,
    pub motif: String,
}

#[derive(Debug, Serialize)]
pub struct AbsenceRhDto {
    pub id: Uuid,
    pub user_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_label: Option<String>,
    pub debut: NaiveDate,
    pub fin: NaiveDate,
    pub motif: String,
    pub statut: String,
}

impl From<&domain_eleve::AbsenceRh> for AbsenceRhDto {
    fn from(a: &domain_eleve::AbsenceRh) -> Self {
        Self {
            id: a.id,
            user_id: a.user_id,
            user_label: None,
            debut: a.debut,
            fin: a.fin,
            motif: a.motif.clone(),
            statut: a.statut.clone(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct AffecterEdtCreneauBody {
    pub prof_user_id: Option<Uuid>,
    pub salle: Option<String>,
    pub salle_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
pub struct CreerEditionVsBody {
    pub code: String,
    pub titre: String,
    pub corps: String,
}

#[derive(Debug, Serialize)]
pub struct EditionVsDto {
    pub id: Uuid,
    pub code: String,
    pub titre: String,
    pub corps: String,
}

impl From<&domain_eleve::EditionVsTemplate> for EditionVsDto {
    fn from(t: &domain_eleve::EditionVsTemplate) -> Self {
        Self {
            id: t.id,
            code: t.code.clone(),
            titre: t.titre.clone(),
            corps: t.corps.clone(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct RendreEditionVsBody {
    pub eleve_id: Uuid,
    pub date: Option<NaiveDate>,
}

#[derive(Debug, Serialize)]
pub struct EditionVsRenduDto {
    pub texte: String,
}

#[derive(Debug, Deserialize)]
pub struct CreerEntrepriseBody {
    pub raison_sociale: String,
    pub siret: Option<String>,
    pub adresse: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct EntrepriseDto {
    pub id: Uuid,
    pub raison_sociale: String,
    pub siret: Option<String>,
    pub adresse: String,
}

impl From<&domain_eleve::Entreprise> for EntrepriseDto {
    fn from(e: &domain_eleve::Entreprise) -> Self {
        Self {
            id: e.id,
            raison_sociale: e.raison_sociale.clone(),
            siret: e.siret.clone(),
            adresse: e.adresse.clone(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreerConventionStageBody {
    pub eleve_id: Uuid,
    pub entreprise_id: Uuid,
    pub debut: NaiveDate,
    pub fin: NaiveDate,
    pub tuteur_nom: String,
}

#[derive(Debug, Serialize)]
pub struct ConventionStageDto {
    pub id: Uuid,
    pub eleve_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eleve_label: Option<String>,
    pub entreprise_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entreprise_label: Option<String>,
    pub debut: NaiveDate,
    pub fin: NaiveDate,
    pub tuteur_nom: String,
    pub statut: String,
}

impl From<&domain_eleve::ConventionStage> for ConventionStageDto {
    fn from(c: &domain_eleve::ConventionStage) -> Self {
        Self {
            id: c.id,
            eleve_id: c.eleve_id,
            eleve_label: None,
            entreprise_id: c.entreprise_id,
            entreprise_label: None,
            debut: c.debut,
            fin: c.fin,
            tuteur_nom: c.tuteur_nom.clone(),
            statut: c.statut.clone(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreerEdtContrainteBody {
    pub kind: String,
    pub prof_user_id: Option<Uuid>,
    pub salle_code: Option<String>,
    pub jour_semaine: Option<i16>,
    pub heure_debut: Option<String>,
    pub heure_fin: Option<String>,
    pub detail: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct EdtContrainteDto {
    pub id: Uuid,
    pub kind: String,
    pub prof_user_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prof_label: Option<String>,
    pub salle_code: Option<String>,
    pub jour_semaine: Option<i16>,
    pub heure_debut: Option<String>,
    pub heure_fin: Option<String>,
    pub detail: String,
}

impl From<&domain_eleve::EdtContrainte> for EdtContrainteDto {
    fn from(c: &domain_eleve::EdtContrainte) -> Self {
        Self {
            id: c.id,
            kind: c.kind.as_str().to_string(),
            prof_user_id: c.prof_user_id,
            prof_label: None,
            salle_code: c.salle_code.clone(),
            jour_semaine: c.jour_semaine,
            heure_debut: c.heure_debut.map(|t| t.format("%H:%M").to_string()),
            heure_fin: c.heure_fin.map(|t| t.format("%H:%M").to_string()),
            detail: c.detail.clone(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreerReferentielVsBody {
    pub code: String,
    pub libelle: String,
}

#[derive(Debug, Serialize)]
pub struct ReferentielVsDto {
    pub id: Uuid,
    pub code: String,
    pub libelle: String,
    pub actif: bool,
}

impl From<&domain_eleve::IncidentType> for ReferentielVsDto {
    fn from(t: &domain_eleve::IncidentType) -> Self {
        Self {
            id: t.id,
            code: t.code.clone(),
            libelle: t.libelle.clone(),
            actif: t.actif,
        }
    }
}

impl From<&domain_eleve::SanctionType> for ReferentielVsDto {
    fn from(t: &domain_eleve::SanctionType) -> Self {
        Self {
            id: t.id,
            code: t.code.clone(),
            libelle: t.libelle.clone(),
            actif: t.actif,
        }
    }
}

impl From<&domain_eleve::SanctionMotif> for ReferentielVsDto {
    fn from(t: &domain_eleve::SanctionMotif) -> Self {
        Self {
            id: t.id,
            code: t.code.clone(),
            libelle: t.libelle.clone(),
            actif: t.actif,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct DeclarerIncidentBody {
    pub type_id: Uuid,
    pub jour: NaiveDate,
    pub description: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct IncidentDto {
    pub id: Uuid,
    pub eleve_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eleve_label: Option<String>,
    pub type_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub type_label: Option<String>,
    pub jour: NaiveDate,
    pub description: Option<String>,
}

impl From<&domain_eleve::Incident> for IncidentDto {
    fn from(i: &domain_eleve::Incident) -> Self {
        Self {
            id: i.id,
            eleve_id: i.eleve_id,
            eleve_label: None,
            type_id: i.type_id,
            type_label: None,
            jour: i.jour,
            description: i.description.clone(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct PrononcerSanctionBody {
    pub type_id: Uuid,
    pub motif_id: Option<Uuid>,
    pub incident_id: Option<Uuid>,
    pub jour: NaiveDate,
    pub duree_minutes: Option<i32>,
    pub note: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SanctionDto {
    pub id: Uuid,
    pub eleve_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eleve_label: Option<String>,
    pub type_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub type_label: Option<String>,
    pub motif_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub motif_label: Option<String>,
    pub incident_id: Option<Uuid>,
    pub jour: NaiveDate,
    pub duree_minutes: Option<i32>,
    pub note: Option<String>,
    pub statut: String,
}

impl From<&domain_eleve::Sanction> for SanctionDto {
    fn from(s: &domain_eleve::Sanction) -> Self {
        Self {
            id: s.id,
            eleve_id: s.eleve_id,
            eleve_label: None,
            type_id: s.type_id,
            type_label: None,
            motif_id: s.motif_id,
            motif_label: None,
            incident_id: s.incident_id,
            jour: s.jour,
            duree_minutes: s.duree_minutes,
            note: s.note.clone(),
            statut: s.statut.as_str().to_string(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct FoyerListItemDto {
    pub id: Uuid,
    pub libelle: Option<String>,
    pub nb_eleves: i64,
}

#[derive(Debug, Deserialize)]
pub struct CreateFoyerBody {
    pub libelle: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RattacherResponsableFoyerBody {
    pub eleve_id: Uuid,
    pub email: String,
    pub display_name: Option<String>,
    pub telephone: Option<String>,
    /// Mot de passe initial si création du compte (défaut labo).
    pub password: Option<String>,
    pub lien: Option<String>,
    pub peut_voir_notes: Option<bool>,
    pub peut_voir_vs: Option<bool>,
    pub peut_justifier_absence: Option<bool>,
    pub peut_autoriser_sorties: Option<bool>,
    pub est_payeur: Option<bool>,
    pub part_paiement: Option<i16>,
}

#[derive(Debug, Deserialize)]
pub struct SetTelephoneBody {
    pub telephone: String,
}

#[derive(Debug, Deserialize)]
pub struct SetEmailBody {
    pub email: String,
}

#[derive(Debug, Serialize)]
pub struct FoyerLienDto {
    pub lien_id: Uuid,
    pub eleve_id: Uuid,
    pub eleve_label: String,
    pub eleve_classe: Option<String>,
    pub user_id: Uuid,
    pub display_name: Option<String>,
    pub user_email: Option<String>,
    pub telephone: Option<String>,
    pub lien: String,
    pub peut_voir_notes: bool,
    pub peut_voir_vs: bool,
    pub peut_justifier_absence: bool,
    pub peut_autoriser_sorties: bool,
    pub est_payeur: bool,
    pub part_paiement: Option<i16>,
}

#[derive(Debug, Serialize)]
pub struct FoyerDetailDto {
    pub id: Uuid,
    pub libelle: Option<String>,
    pub liens: Vec<FoyerLienDto>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateLienBody {
    pub lien: Option<String>,
    pub peut_voir_notes: bool,
    pub peut_voir_vs: bool,
    pub peut_justifier_absence: bool,
    pub peut_autoriser_sorties: bool,
    pub est_payeur: bool,
    pub part_paiement: Option<i16>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateLienBatchItem {
    pub lien_id: Uuid,
    pub lien: Option<String>,
    pub peut_voir_notes: bool,
    pub peut_voir_vs: bool,
    pub peut_justifier_absence: bool,
    pub peut_autoriser_sorties: bool,
    pub est_payeur: bool,
    pub part_paiement: Option<i16>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateLiensEleveBody {
    pub liens: Vec<UpdateLienBatchItem>,
}

#[derive(Debug, Serialize)]
pub struct EleveLienDto {
    pub id: Uuid,
    pub eleve_id: Uuid,
    pub user_id: Uuid,
    pub foyer_id: Option<Uuid>,
    pub lien: String,
    pub peut_voir_notes: bool,
    pub peut_voir_vs: bool,
    pub peut_justifier_absence: bool,
    pub peut_autoriser_sorties: bool,
    pub est_payeur: bool,
    pub part_paiement: Option<i16>,
}

#[allow(dead_code)]
pub fn kind_label(k: AttributKind) -> &'static str {
    k.as_str()
}
